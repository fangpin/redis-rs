---
title: 运行时与协议
layout: default
nav_exclude: true
permalink: /zh/docs/runtime-and-protocol/
---

# 运行时与协议

[返回首页]({{ '/' | relative_url }}) | [总览]({{ '/overview/' | relative_url }}) | [持久化与复制]({{ '/zh/docs/persistence-and-replication/' | relative_url }}) | [命令、流与事务]({{ '/zh/docs/commands-streams-and-transactions/' | relative_url }})

这一章覆盖程序入口、TCP 服务循环，以及把原始 RESP 请求变成可执行命令的协议解析层。

## 文件边界

- `src/main.rs`
- `src/server.rs`
- `src/protocol.rs`
- `src/options.rs`

## 启动入口

`src/main.rs` 是唯一的可执行入口。它解析四个 CLI 参数：

- `--dir`
- `--dbfilename`
- `--port`
- `--replicaof`

这些参数会被组装成 `DBOption` 和 `ReplicationOption`，再传给 `Server::new(...)`。

最关键的效果是：节点角色完全由命令行决定。只要传入 `--replicaof`，服务就会以 slave 模式启动；否则就是 master。

## 监听模型

服务通过 Tokio 的 `TcpListener` 绑定到 `127.0.0.1:<port>`。

每收到一个新连接时，会执行这条链路：

1. 克隆一份 `Server`
2. 启动一个 Tokio 任务
3. 把 socket 交给 `Server::handle(...)`

因此，这个实现是“共享逻辑服务 + 每连接异步任务”，而不是单线程命令循环。

## 共享状态

`src/server.rs` 里的 `Server` 保存了几类共享状态：

- `storage`：字符串键值存储
- `streams`：Stream 数据
- `option`：运行配置
- `offset`：复制偏移量
- `master_repl_clients`：master 下游的 replica 连接
- `stream_reader_blocker`：阻塞式 `XREAD` 的唤醒通道

其中普通 key-value 和 stream 数据走的是两把不同的 mutex，这让两类操作不需要共用同一把锁。

## 连接处理循环

`Server::handle(...)` 的主循环是：

1. 读 socket 到固定缓冲区
2. 按 UTF-8 解释
3. 调用 `Cmd::from(...)`
4. 执行 `cmd.run(...)`
5. 如果不是复制连接，就把结果写回客户端

这里使用的是固定 `512` 字节缓冲区，并且默认一次读取就能形成一条可解析命令。它比生产级 Redis 简化很多，但控制流非常直观。

## RESP 解析器

`src/protocol.rs` 定义了一个很小的 `Protocol` 枚举：

- `SimpleString`
- `BulkString`
- `Null`
- `Array`

当前支持的 RESP 形态包括：

- `+` 开头的简单字符串
- `$` 开头的 bulk string
- `*` 开头的数组

`Protocol::from(...)` 会返回两个值：

- 解析后的协议对象
- 消耗的字节数

这个“值 + 消耗长度”的返回方式会继续被数组解析复用。

## 当前解析器的重要行为

### Bulk string 会被转成小写

`parse_bulk_string_sfx(...)` 在得到字符串后会调用 `to_lowercase()`。

这意味着：

- 命令关键字大小写不敏感
- bulk string 负载内容也不会保留原始大小写

这对教学实现很方便，但和完整 Redis 的“按字节保真”并不完全一致。

### 编码辅助函数

这个模块还提供了很多执行层依赖的辅助方法：

- `Protocol::ok()`
- `Protocol::err(...)`
- `Protocol::none()`
- `encode()`
- `decode()`

这些方法就是“命令执行结果”和“socket 写回格式”之间的公共边界。

## 从节点启动的特殊流程

如果节点以 slave 身份启动：

1. 创建 `FollowerReplicationClient`
2. 执行复制握手
3. 接收并加载 master 发来的 RDB 快照
4. 再启动一个 handler 任务来消费复制连接上的后续命令

这一套高层流程仍然写在 `main.rs` 里，而不是被隐藏进 `Server::new(...)`，所以入口控制流很好追。

## 当前实现限制

- 固定大小 socket 读缓冲区
- 默认读到的是完整 UTF-8 命令片段
- 没有做跨多次 read 的增量协议拼接
- 解析器更偏“能驱动当前命令集”，而不是完整 RESP 实现

这些限制和仓库定位是一致的：重点是把 Redis 的请求生命周期讲清楚，而不是把协议边角做全。
