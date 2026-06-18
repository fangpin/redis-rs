---
title: 运行时与服务端
layout: default
nav_exclude: true
permalink: /zh/docs/server-runtime/
---

# 运行时与服务端

[返回首页]({{ '/' | relative_url }}) | [文档总览]({{ '/zh/docs/overview/' | relative_url }}) | [RESP 协议]({{ '/zh/docs/resp-protocol/' | relative_url }}) | [复制链路]({{ '/zh/docs/replication-flow/' | relative_url }})

这一章从可执行入口开始，跟踪整个服务端运行时是怎么被搭起来的。

## 文件边界

- `src/main.rs`
- `src/server.rs`
- `src/options.rs`

## 进程是如何启动的

`src/main.rs` 是唯一的二进制入口，它解析四个 CLI 参数：

- `--dir`
- `--dbfilename`
- `--port`
- `--replicaof`

然后把这些参数组装成 `DBOption` 和 `ReplicationOption`，再传入 `Server::new(...)`。

这意味着节点角色在服务创建之前就已经确定：

- 没有 `--replicaof` -> master
- 有 `--replicaof` -> slave

## 配置对象

`src/options.rs` 只保留了一组很小的运行配置：

`DBOption` 包含：

- DB 目录
- DB 文件名
- 监听端口
- 复制配置

`ReplicationOption` 包含：

- `role`
- `master_replid`
- `master_repl_offset`
- `replica_of`

这个项目没有动态角色发现逻辑，角色完全来自命令行配置。

## 监听与任务模型

`main.rs` 使用 Tokio 的 `TcpListener` 监听 `127.0.0.1:<port>`。

每接到一个新连接，就执行：

1. 克隆 `Server`
2. 启动 Tokio 任务
3. 调用 `Server::handle(...)`

因此运行时模型可以概括成：

- 一份共享逻辑服务端状态
- 每个 socket 一个异步任务
- 没有单独的中央调度线程

## `Server` 里到底存了什么

`src/server.rs` 里的 `Server` 是整个共享运行时外壳，关键字段有：

- `storage: Arc<Mutex<Storage>>`
- `streams: Arc<Mutex<HashMap<String, Stream>>>`
- `option: DBOption`
- `offset: Arc<AtomicU64>`
- `master_repl_clients: Arc<Mutex<Option<MasterReplicationClient>>>`
- `stream_reader_blocker: Arc<Mutex<Vec<Sender<()>>>>`
- `master_addr: Option<String>`

这里把普通字符串存储和 stream 存储拆成了两套容器，因此两类操作不会争用同一把锁。

## `Server::new(...)` 做了什么

`Server::new(...)` 的高层流程是：

1. 如果自己是 slave，就从配置里推导 `master_addr`
2. 分配共享状态容器
3. 调用 `init().await`

master 才会初始化下游 replica fan-out 客户端；slave 则没有这个对象。

## Master 侧初始化

`Server::init(...)` 当前只负责 master 节点的本地持久化恢复。

控制流如下：

1. 用 `dir + db_file_name` 拼出 DB 文件路径
2. 如果文件不存在则创建
3. 检查文件长度
4. 如果非空，则进入 `rdb::parse_rdb_file(...)`

slave 不走这里，因为它的初始状态来自主节点发来的快照。

## Slave 侧启动链路

slave 的特殊启动逻辑仍然显式写在 `main.rs`，没有被藏进构造函数里。

顺序如下：

1. 创建 `FollowerReplicationClient`
2. `PING` master
3. 上报监听端口
4. 上报同步能力
5. 发起 `PSYNC`
6. 再为复制 socket 启动一个 `Server::handle(...)`

这样从入口文件就能直接看到 slave 的完整启动过程。

## 连接处理循环

`Server::handle(...)` 是每个 socket 的主循环：

1. 从 socket 读到固定 `512` 字节缓冲区
2. 按 UTF-8 转成字符串
3. 用 `Cmd::from(...)` 解析命令
4. 通过 `cmd.run(...)` 执行
5. 如果不是复制连接，就把结果写回客户端

如果当前节点是 master，并且命令是 `PSYNC`，这个循环会切换模式：

1. 发送 RDB 快照
2. 把当前 socket 注册到 replica 列表
3. 不再把它当普通请求-响应连接

这就是普通客户端连接和复制连接在运行时分叉的地方。

## 普通请求的数据流

一个普通客户端请求的路径可以概括成：

```text
socket bytes
-> Server::handle
-> Cmd::from
-> Cmd::run
-> Protocol response
-> socket write
```

`Server` 自己更像是一个编排层，而不是命令语义实现层。

## 当前实现限制

- 固定大小读缓冲区
- 一次 read 默认只形成一条命令
- 没有跨多次读取的增量协议拼接
- 没有显式的 shutdown / supervisor 模型

这些取舍让运行时入口保持得很短，也方便顺着源码追控制流。
