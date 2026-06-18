---
title: 主从复制链路
layout: default
nav_order: 7
permalink: /zh/docs/replication-flow/
---

# 主从复制链路

这一章沿着 follower 如何连上 master、如何接收初始化 snapshot，以及后续如何继续消费复制命令来展开。

## 文件边界

- `src/main.rs`
- `src/replication_client.rs`
- `src/server.rs`
- `src/cmd.rs`

## 复制是由启动流程驱动的

当前没有一个单独的 replication service 在后台统一协调所有事。

follower 复制是在 `main.rs` 里，`Server` 创建完成后立刻显式启动的。

启动顺序是：

```text
Server::new(...)
-> get_follower_repl_client(...)
-> ping_master()
-> report_port(...)
-> report_sync_protocol()
-> start_psync(...)
-> spawn Server::handle(replication_stream, true)
```

好处是：整个 bootstrap 在二进制入口附近就能一眼看到。

## follower 侧客户端对象

`src/replication_client.rs` 里定义了 `FollowerReplicationClient`。

它只有一个核心字段：

- `stream: TcpStream`

这一个 socket 会被复用于：

- 握手命令
- snapshot 传输
- 后续 master 的实时命令回放

所以“初始化复制”和“稳定态复制”当前是共用一个 TCP 连接的。

## 握手步骤

follower 会按固定顺序发四条消息：

1. `PING`
2. `REPLCONF listening-port <port>`
3. `REPLCONF capa psync2`
4. `PSYNC ? -1`

`ping_master(...)`、`report_port(...)`、`report_sync_protocol(...)` 都会：

- 构造 RESP array
- 写到 socket
- 调 `check_resp(...)`

`check_resp(...)` 当前做的是“按字节精确比较预期 simple string 响应”。

这很直观，但也意味着它假设整个响应能在一次读取里完整拿到。

## follower 侧的 `PSYNC`

`start_psync(...)` 会先写出 `PSYNC ? -1`，然后立刻进入 `recv_rdb_file(...)`。

所以从 follower 视角看，`PSYNC` 的含义是：

1. 请求全量同步
2. 解析 master 返回的 `FULLRESYNC` 行
3. 消费后续的 RDB payload
4. 让 socket 继续留在可接收实时复制命令的位置上

## `recv_rdb_file(...)`

`recv_rdb_file(...)` 会先把 socket 包成 `BufReader`，然后依次读取：

1. 一行以 `\r\n` 结尾的复制元信息
2. 一行 `$<len>\r\n` 形式的 RDB 长度头
3. RDB 正文，并把解析工作交给 `rdb::parse_rdb(...)`

第一行当前要求恰好能拆成三个 token，比如：

```text
FULLRESYNC <replid> <offset>
```

这里有个值得写清楚的点：虽然代码读出了 `rdb_file_len` 并打印日志，但后面并没有用这个长度去限制 snapshot 的读取边界，而是依赖 `rdb::parse_rdb(...)` 自己在字节流中遇到 `EOF`。

## master 侧的 `PSYNC`

在 master 上，`PSYNC` 是跨两个模块完成的。

`cmd.rs` 负责命令语义：

- `Cmd::from(...)` 识别 `psync`
- `psync_cmd(...)` 在 master 上返回 `FULLRESYNC <master_replid> 0`

`server.rs` 负责 socket 状态切换：

1. `cmd.run(...)` 先生成 `FULLRESYNC` 响应
2. `Server::handle(...)` 按普通请求先把这个响应写回去
3. `handle(...)` 识别到命令是 `Cmd::Psync`
4. `MasterReplicationClient::send_rdb_file(...)` 继续把 snapshot 写到 socket
5. `MasterReplicationClient::add_stream(...)` 把这个 socket 注册进下游副本列表
6. `handle(...)` 跳出普通连接循环

这个拆分很关键，因为 `PSYNC` 不只是一个命令，它还会让连接的角色发生切换。

## snapshot payload 的来源

`MasterReplicationClient::send_rdb_file(...)` 当前不会把内存里的实时状态序列化出来。

它做的是把硬编码的十六进制常量 `EMPTY_RDB_FILE_HEX_STRING` 还原成字节，再发给 follower。

所以当前全量同步的实际行为是：

- 发送一个有效但固定的空 snapshot 壳子
- 后续写入再靠命令回放补齐

这是和生产级 Redis 差距最明显的地方之一。

## 下游副本注册

`MasterReplicationClient` 会把副本 socket 保存在：

```text
Arc<Mutex<Vec<TcpStream>>>
```

`add_stream(...)` 只是简单地把 socket push 进去。

这里没有独立的副本元数据结构去记录：

- 副本 ID
- 健康状态
- 最后 ACK offset
- 背压状态

master 当前只记“后面可以写哪些 socket”。

## 命令 fan-out

副本注册之后，写命令传播走的是 `send_command(...)`。

控制流是：

1. 锁住副本 socket 列表
2. 遍历每个 socket
3. 把 `protocol.encode()` 的结果写出去

没有针对单个副本的重试、摘除或限流策略。

## 复制策略与命令执行交汇的地方

大部分写命令最终都会走到 `cmd.rs` 里的 `resp_and_replicate(...)`。

这个 helper 统一处理角色规则：

- master -> 广播给下游副本，再返回本地响应
- slave 收到普通客户端写请求 -> 拒绝
- slave 从复制连接收到回放命令 -> 接受

这就是“复制语义”和“命令语义”真正交汇的地方。

## offset 追踪

当前仓库里其实有两套 offset 概念。

实时共享计数器：

- `server.offset`
- 在 `Cmd::run(...)` 里更新
- 某些写 helper 里也会手动额外加一次
- `REPLCONF GETACK` 读取它

静态配置字段：

- `server.option.replication.master_repl_offset`
- 在 `main.rs` 里初始化
- `INFO replication` 返回它

所以 `GETACK` 和 `INFO replication` 实际上并没有共享同一个实时来源。

## 当前实现限制

- full sync 永远发固定的空 RDB payload
- follower 不会按声明长度强约束 snapshot 读取边界
- 不支持 partial resync
- 没有 backlog window
- 启动后如果链路断开，没有自动重连循环
- 副本 socket 只存在线，不带额外元数据
- offset 汇报来源不一致

即使如此，这条复制链路仍然把最核心的教学形状展示出来了：显式握手、全量 snapshot 初始化、注册副本 socket，然后再做命令回放。
