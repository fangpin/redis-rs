---
title: 复制链路
layout: default
nav_exclude: true
permalink: /zh/docs/replication-flow/
---

# 复制链路

[返回首页]({{ '/' | relative_url }}) | [文档总览]({{ '/zh/docs/overview/' | relative_url }}) | [运行时与服务端]({{ '/zh/docs/server-runtime/' | relative_url }}) | [RDB 解析器]({{ '/zh/docs/rdb-parser/' | relative_url }})

这一章专门讲 master/slave 之间如何建立关系、传输快照，以及后续如何广播写命令。

## 文件边界

- `src/main.rs`
- `src/replication_client.rs`
- `src/server.rs`
- `src/cmd.rs`

## Slave 启动链路

slave 的复制启动过程是显式写在 `main.rs` 里的。

顺序如下：

1. 构造 `Server`
2. 创建 `FollowerReplicationClient`
3. `ping_master()`
4. `report_port(...)`
5. `report_sync_protocol()`
6. `start_psync(...)`
7. 再为复制 socket 启动一个 `Server::handle(...)`

所以复制控制流并没有被藏起来，入口处就能看见。

## Follower 侧握手

`FollowerReplicationClient` 会在一条 `TcpStream` 上按顺序发送：

1. `PING`
2. `REPLCONF listening-port <port>`
3. `REPLCONF capa psync2`
4. `PSYNC ? -1`

每一步都会通过 `check_resp(...)` 校验返回值是否匹配预期。

## `PSYNC` 之后会读什么

`start_psync(...)` 发完命令后，立即进入 `recv_rdb_file(...)`。

这里期待的输入是：

1. 一行类似 `FULLRESYNC <id> <offset>` 的信息
2. 一个 `$<len>` 形式的快照长度头
3. 真正的 RDB 二进制快照内容

快照内容不会走另一套解析逻辑，而是直接复用 `rdb::parse_rdb(...)`。

## Master 侧如何处理 `PSYNC`

master 对 `PSYNC` 的处理跨了两个层次：

- `Cmd::from(...)` 把它识别成 `Cmd::Psync`
- `Server::handle(...)` 在命令执行后对这个 socket 做特殊处理

`Server::handle(...)` 中的分支是复制模式切换的关键：

1. 调 `send_rdb_file(...)`
2. 调 `add_stream(...)`
3. 跳出普通请求循环

从这一刻起，这个 socket 就是一个已注册的 replica 下游。

## 为什么 Master 行为不全写在 `cmd.rs`

`PSYNC` 既是：

- 一条逻辑命令
- 一次连接模式切换

后者必须在 `server.rs` 里做，因为服务端需要接管并保存这个 replica socket 以便后续 fan-out。

所以当前实现故意把控制流拆成两层：

- 命令语义 -> `psync_cmd(...)`
- socket 生命周期切换 -> `Server::handle(...)`

## 快照数据从哪里来

`MasterReplicationClient::send_rdb_file(...)` 当前发送的是一段常量十六进制编码的空 RDB 文件。

这并不是对当前内存状态的真实序列化，只是为了让协议流程能继续跑通。

这也是当前实现和真实 Redis 差距最大的点之一。

## 写命令如何广播

一旦 replica socket 被注册，后续写命令就通过 `MasterReplicationClient::send_command(...)` 广播。

它的逻辑很简单：

1. 锁住 replica socket 列表
2. 遍历每个连接
3. 把编码后的命令写出去

没有做每个 replica 的背压或独立状态管理。

## 命令执行如何接上复制

`cmd.rs` 中的写命令最终会调用 `resp_and_replicate(...)`。

它根据节点角色做分支：

- master -> 先广播给 replicas，再返回本地响应
- slave 的普通客户端连接 -> 拒绝写入
- slave 的复制连接 -> 接受来自 master 的回放命令

这就是命令语义和复制规则真正交汇的地方。

## Offset 跟踪

`server.offset` 保存了一份粗粒度复制偏移量。

命令执行成功后，会按协议编码长度去递增它；`REPLCONF GETACK` 则从这里读进度。

这不是完整 Redis 的复制状态机，但足够表达“当前节点已经处理了多少复制流量”。

## 当前实现限制

- 快照来源是常量空 RDB
- 不支持 partial resync
- 没有 backlog 窗口
- 没有 replica 存活状态管理
- 除启动过程外，没有重连策略

即便如此，它已经清楚展示了 leader-follower 复制的核心骨架：握手、全量同步、注册 socket、再广播写流量。
