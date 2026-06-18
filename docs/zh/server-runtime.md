---
title: 运行时与服务端
layout: default
nav_order: 3
permalink: /zh/docs/server-runtime/
---

# 运行时与服务端

这一章沿着可执行入口、共享 `Server` 对象，以及每个 socket 最终进入的连接循环来展开。

## 文件边界

- `src/main.rs`
- `src/server.rs`
- `src/options.rs`

## 运行时总图

这个仓库的运行时非常直接，没有额外的 acceptor service、调度器或复制 supervisor。

顶层流程是：

```text
CLI 参数
-> DBOption / ReplicationOption
-> Server::new(...)
-> Server::init(...)
-> 可选的 follower 复制握手
-> TcpListener accept loop
-> 每个 socket 一个 Tokio task
-> Server::handle(...)
```

这个结构很重要，因为仓库里的几乎所有子系统都是从这条主路径接进去的。

## 可执行入口

`src/main.rs` 是唯一的二进制入口。

它解析四个命令行参数：

- `--dir`
- `--dbfilename`
- `--port`
- `--replicaof`

这些值会先被拼成 `DBOption` 和 `ReplicationOption`，然后才进入 `Server::new(...)`。

因此进程角色是在启动期一次性决定的：

- 没有 `--replicaof` -> master
- 带 `--replicaof` -> slave

后面没有再做角色切换或协商。

## 配置对象

`src/options.rs` 里的配置模型刻意保持得很小。

`DBOption` 包含：

- `dir`
- `db_file_name`
- `replication`
- `port`

`ReplicationOption` 包含：

- `role`
- `master_replid`
- `master_repl_offset`
- `replica_of`

这里有两个需要注意的点：

1. `master_replid` 在 `main.rs` 里是硬编码的，不是运行时生成的。
2. `master_repl_offset` 是配置字段，不是后面命令执行时共享更新的实时 offset。

这也解释了为什么后面的 `INFO replication` 和 `REPLCONF GETACK` 用的并不是同一个实时来源。

## `Server` clone 到底共享什么

`Server` 虽然实现了 `Clone`，但它并不是深拷贝。

通过 `Arc` 在各个 task 间共享的字段有：

- `storage: Arc<Mutex<Storage>>`
- `streams: Arc<Mutex<HashMap<String, Stream>>>`
- `offset: Arc<AtomicU64>`
- `master_repl_clients: Arc<Mutex<Option<MasterReplicationClient>>>`
- `stream_reader_blocker: Arc<Mutex<Vec<Sender<()>>>>`

每个 task 自己持有值拷贝的字段有：

- `option: DBOption`
- `master_addr: Option<String>`

所以每个连接 task 都拿到一个轻量级的 `Server` 外壳，但真正的运行时可变状态仍然是共享的。

## `Server::new(...)`

`src/server.rs` 里的 `Server::new(...)` 主要做三件事：

1. 如果角色是 slave，就从 `replica_of` 推导出 `master_addr`
2. 分配共享状态容器
3. 调用 `init().await`

只有 master 才会初始化下游副本 fan-out 所需的对象：

- master -> `Some(MasterReplicationClient::new())`
- slave -> `None`

这个边界比较干净：follower 不会带着一套自己用不到的下游复制结构。

## `Server::init(...)`

`init(...)` 是启动钩子，但当前实现里它只做 master 侧的本地持久化恢复。

控制流是：

1. 判断 `self.is_master()`
2. 组出 `dir/dbfilename`
3. 以 create-if-missing 方式打开文件
4. 检查文件长度
5. 如果非空，调用 `rdb::parse_rdb_file(...)`

slave 会跳过这一整段。它的初始状态依赖后续复制链路从上游 master 拉下来，而不是本地文件恢复。

这类主从不对称逻辑是直接写在代码路径里的，没有被抽成一个统一但看不清分支的 init 过程。

## follower 启动路径保留在 `main.rs`

slave 启动时的复制握手没有被藏进 `Server::new(...)`。

`main.rs` 里显式写着：

1. 创建 `Server`
2. clone 出 `sc`
3. 调用 `get_follower_repl_client(...)`
4. `ping_master()`
5. `report_port(server.option.port)`
6. `report_sync_protocol()`
7. `start_psync(&mut sc)`
8. `tokio::spawn(sc.handle(follower_repl_client.stream, true))`

这样做的好处是：入口文件本身就能看清主从角色差异。

## `get_follower_repl_client(...)`

`Server::get_follower_repl_client(...)` 本质上只是一个角色门禁：

- slave -> 创建 `FollowerReplicationClient`
- master -> 返回 `None`

它不会缓存上游连接，而是在启动时创建一次，把 socket 交还给 `main.rs`。

## listener 与并发模型

启动完成后，`main.rs` 会在 `127.0.0.1:<port>` 上绑定一个 Tokio `TcpListener`。

每当 accept 到一个新连接，它都会：

1. clone 共享的 `Server`
2. 新起一个 Tokio task
3. 调用 `Server::handle(stream, false)`

所以这里的并发模型是：

- 一个 listener
- 每个 socket 一个异步 task
- 没有中央请求队列
- 共享状态靠 `Mutex` 保护

这让运行时很好读，但也意味着锁边界会直接影响行为。

## `Server::handle(...)` 里的连接循环

`Server::handle(...)` 是普通客户端连接和复制连接最终汇合的地方。

当前循环逻辑是：

1. 读入一个固定大小的 `512` 字节缓冲区
2. 如果 `len == 0` 就结束
3. 用 `str::from_utf8(...)` 把字节当成 UTF-8 文本
4. 用 `Cmd::from(...)` 解析一条命令
5. 用 `cmd.run(...)` 执行
6. 如果这不是复制连接，就把响应写回去
7. 如果当前节点是 master 且命令是 `PSYNC`，把这个 socket 切到下游副本模式

这里还有一个 task 局部的事务队列：

```text
queued_cmd: Option<Vec<(Cmd, Protocol)>>
```

它完全活在连接循环里，所以事务天然就是“每个连接各自维护”的语义。

## 普通请求的数据流

普通客户端请求的数据路径是：

```text
socket bytes
-> Server::handle
-> Cmd::from
-> Cmd::run
-> Protocol response
-> stream.write(...)
```

`Server` 自己主要负责编排，并不直接实现命令语义。

## 复制连接的模式切换

`PSYNC` 很特殊，因为它同时是：

- 一个有逻辑返回值的命令
- 一个连接状态切换点

在 `handle(...)` 里，这个分叉很清楚：

1. `cmd.run(...)` 先生成 `FULLRESYNC ...`
2. 因为 `is_rep_conn == false`，这个响应会先按普通响应写回 socket
3. `handle(...)` 识别到 `Cmd::Psync`
4. `MasterReplicationClient::send_rdb_file(...)` 继续写 RDB snapshot
5. `MasterReplicationClient::add_stream(...)` 把这个 socket 注册成下游副本
6. 跳出普通请求循环

从这一步开始，这条连接就不再是普通 request/response socket，而是复制下游通道。

## 锁与所有权边界

运行时当前用了几把比较粗粒度的锁：

- 所有字符串键值都在一个 `Mutex<Storage>` 后面
- 所有 streams 都在一个 `Mutex<HashMap<...>>` 后面
- 所有下游副本 socket 都在一个 `Mutex<Vec<TcpStream>>` 后面
- 所有阻塞的 stream reader 都在一个 `Mutex<Vec<Sender<()>>>` 后面

这明显是在用“可读性”优先于“细粒度并发控制”的方式写实现。

## 当前实现限制

- `handle(...)` 假设一条完整命令能在一次 `read` 和一个 `512` 字节缓冲里拿全
- 没有做 partial read 的增量 framing
- 输入很早就被当成 UTF-8 文本，而不是全程按原始字节处理
- 没有结构化的 shutdown 或 task supervision
- follower 启动完成后，如果 master 断开，没有重连循环
- 连接模式切换是靠分支语句完成的，没有单独的状态枚举

这个运行时足够小，可以一口气读完，但文档描述的是“当前源码行为”，不是一个更完整的生产级 Redis 架构。
