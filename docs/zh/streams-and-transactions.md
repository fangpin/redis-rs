---
title: Streams 与事务
layout: default
nav_exclude: true
permalink: /zh/docs/streams-and-transactions/
---

# Streams 与事务

[返回首页]({{ '/' | relative_url }}) | [文档总览]({{ '/zh/docs/overview/' | relative_url }}) | [命令执行]({{ '/zh/docs/command-execution/' | relative_url }}) | [存储模型]({{ '/zh/docs/storage-model/' | relative_url }})

这一章专门看两个比基础字符串命令更高一层的能力：Streams 和事务。

## 文件边界

- `src/cmd.rs`
- `src/server.rs`

## Stream 的存储结构

Stream 不放在 `Storage` 里，而是放在 `Server::streams` 中。

核心类型是：

```text
BTreeMap<String, Vec<(String, String)>>
```

整个容器则是：

```text
HashMap<String, Stream>
```

也就是：

- 顶层 key -> stream 名
- 有序 map key -> entry id
- value -> field/value 对列表

这里选择 `BTreeMap` 的核心原因，就是为了天然支持按 entry id 做有序范围查询。

## Stream ID 解析

`split_offset(...)` 是 stream offset / id 的底层 helper。

它会拆出三部分：

- 毫秒时间戳部分
- sequence 部分
- 是否使用了 wildcard sequence

这个 helper 同时被读写路径复用，因此 stream 排序规则集中在一个地方。

## `XADD`

`xadd_cmd(...)` 负责 stream 写入，它的流程是：

1. 把 `*` 归一化成 `now_in_millis()-*`
2. 拆分 incoming id
3. 拒绝非法 `0-0`
4. 如果 stream 已存在，就和当前尾部 entry 比较
5. 如果用了 wildcard，就推导最终 sequence
6. 把 field/value 写进 stream entry
7. 唤醒阻塞读者
8. 如有需要，把原始命令复制到 replicas

这是 stream 逻辑里最复杂的一条路径，因为它同时负责 ID 验证和 append 语义。

## `XRANGE`

`xrange_cmd(...)` 先把两个 Redis 特殊边界值转掉：

- `-` -> 从最小开始
- `+` -> 到最大结束

然后使用 `BTreeMap::range(...)` 做范围查询，并把结果重新编码成 `Protocol::Array`。

因此它的核心数据流是：

```text
stream key
-> BTreeMap range
-> 顺序遍历 entry
-> 组装 Protocol::Array
```

## `XREAD`

`xread_cmd(...)` 当前支持：

- 多个 stream
- 每个 stream 一个起始 offset
- 可选阻塞模式

阻塞逻辑有两种：

- `BLOCK <millis>` 且大于 0 -> 直接 sleep
- `BLOCK 0` -> 注册唤醒 sender，然后等通知

第二种模式会借助 `server.stream_reader_blocker` 作为一个轻量等待队列。

## 读者唤醒模型

`XADD` 成功后，会拿到 `stream_reader_blocker`，向每个等待者发送一个空信号，然后清空列表。

这个模型非常简单：

- 不是按 stream 分组等待
- 没有公平性调度
- 只有一份全局等待者列表

但它足够演示阻塞读取的基本思路。

## 事务队列模型

事务状态不是放在全局 `Server` 里，而是连接本地的：

```text
Option<Vec<(Cmd, Protocol)>>
```

这意味着事务只属于当前 client session，不会变成共享全局状态。

## `MULTI`、`EXEC`、`DISCARD`

事务控制流如下：

- `MULTI` -> 创建空队列
- 队列存在时，普通命令入队并返回 `QUEUED`
- `EXEC` -> 通过 `cmd.run(...)` 依次重放队列
- `DISCARD` -> 丢弃整个队列

`exec_cmd(...)` 很紧凑，因为它没有造第二套“事务专用执行器”，而是直接复用现有命令执行路径。

## 为什么事务队列里同时保存 `Cmd` 和 `Protocol`

两者各自有作用：

- `Cmd`：供 replay 时直接分发
- `Protocol`：供复制和 offset 统计继续使用

这样就不需要在事务执行阶段重新解析一次命令。

## 当前实现限制

- stream waiters 不是按 stream key 区分
- `BLOCK <millis>` 用的是 sleep，而不是事件+超时组合
- 事务队列是连接本地、非持久化的
- stream 和事务边缘语义远少于真实 Redis

尽管如此，这部分代码已经足够清楚地展示 higher-level Redis 功能是如何挂到统一命令执行链上的。
