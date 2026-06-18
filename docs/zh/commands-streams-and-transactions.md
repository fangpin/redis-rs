---
title: 命令、流与事务
layout: default
nav_exclude: true
permalink: /zh/docs/commands-streams-and-transactions/
---

# 命令、流与事务

[返回首页]({{ '/' | relative_url }}) | [总览]({{ '/overview/' | relative_url }}) | [运行时与协议]({{ '/zh/docs/runtime-and-protocol/' | relative_url }}) | [持久化与复制]({{ '/zh/docs/persistence-and-replication/' | relative_url }})

这一章覆盖命令解析、执行分发，以及超出基本 key-value 读写之外的 Stream 与事务能力。

## 文件边界

- `src/cmd.rs`
- `src/storage.rs`
- `src/server.rs`

## 命令解析

`Cmd::from(...)` 会把 RESP 数组转换成一个强类型命令枚举。

当前支持的命令包括：

- `PING`
- `ECHO`
- `GET`
- `SET`
- `SET PX`
- `SET EX`
- `DEL`
- `KEYS *`
- `CONFIG GET`
- `INFO`
- `TYPE`
- `INCR`
- `REPLCONF`
- `PSYNC`
- `XADD`
- `XRANGE`
- `XREAD`
- `MULTI`
- `EXEC`
- `DISCARD`

这里的 `Cmd` 枚举就是整个服务端的命令边界。

## 执行入口

`Cmd::run(...)` 是调度中心。它接收：

- 可变的 `Server`
- 原始 `Protocol`
- 是否为复制连接
- 当前事务排队状态

然后再把具体命令路由到更小的 helper，例如：

- `get_cmd(...)`
- `set_cmd(...)`
- `xadd_cmd(...)`
- `xread_cmd(...)`
- `exec_cmd(...)`

这样解析和执行是分开的：`Cmd::from(...)` 负责“这是什么命令”，`run(...)` 负责“它要做什么”。

## 基础存储命令

对字符串键来说，`cmd.rs` 最终还是会落到 `Storage`：

- `GET` -> `storage.get(...)`
- `SET` / 定时 `SET` -> `set(...)` 或 `setx(...)`
- `DEL` -> 删除键
- `INCR` -> 取出字符串数值、自增后写回

这层保持了“所有值先当字符串处理”的思路，也让 RDB 恢复和 RESP 输出更直白。

## Stream 数据模型

`Server` 对 stream 单独维护了一套结构：

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

可以理解成：

- 顶层 `HashMap` 的 key 是 stream 名
- 内层 `BTreeMap` 的 key 是 entry id
- value 是 field/value 对列表

因为内部用的是 `BTreeMap`，范围查询天然更顺手。

## Stream 命令

`cmd.rs` 当前支持的 stream 命令有：

- `XADD`
- `XRANGE`
- `XREAD`

它们负责的关键行为包括：

- 校验 stream id
- 支持 `*` 自动生成 id
- 单 stream 范围查询
- 多 stream 读取
- `XREAD` 的可选阻塞模式

阻塞读取会借助 `server.stream_reader_blocker` 来协调唤醒逻辑。

## 事务实现

事务的核心状态是：

```text
queued_cmd: Option<Vec<(Cmd, Protocol)>>
```

行为如下：

- `MULTI` 创建空队列
- 后续普通命令入队，并返回 `QUEUED`
- `EXEC` 依次执行队列中的命令
- `DISCARD` 清空队列

这个实现的一个优点是没有再造第二套执行器，而是复用：

- 已解析好的 `Cmd`
- 原始 `Protocol`
- 现有的 `run(...)` 分发逻辑

因此，事务执行本质上是“关闭排队模式后，再重放一次相同命令”。

## 与复制偏移量的关系

命令执行成功后，`Cmd::run(...)` 会用协议编码后的长度去递增 `server.offset`。

这个 offset 不是完整 Redis 的复制状态机，但足够表达“有多少写流量从当前节点流过”。

## 当前实现限制

- 每条命令都假定 RESP 数组形状基本正确
- 不支持的命令会落到 `Unknow`
- 事务队列是连接本地状态
- Stream 和事务行为强调学习可读性，而不是完全对齐 Redis 的边缘语义

这和整个项目的目标一致：让命令执行、Stream 数据结构、事务排队这几块能被直接读懂。
