---
title: Streams 与事务
layout: default
nav_order: 9
permalink: /zh/docs/streams-and-transactions/
---

# Streams 与事务

这一章看在基础字符串命令路径之上叠加出来的两类更高层行为：stream 数据结构和事务队列。

## 文件边界

- `src/cmd.rs`
- `src/server.rs`
- `src/storage.rs`

## stream 状态结构

stream 不存在 `Storage` 里，而是挂在 `Server::streams` 上。

完整结构是：

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

可以拆开理解成：

- stream 名 -> 一个 stream
- entry ID 字符串 -> 一个有序记录
- 记录值 -> 一组 field/value 对

这里最核心的设计就是 `BTreeMap`。因为它天然有序，所以 `XRANGE` 和 `XREAD` 可以直接做范围查询。

## stream ID 与 `split_offset(...)`

`split_offset(...)` 是 stream 排序规则的底层 helper。

像下面这些 ID：

- `1526985054069-0`
- `1526985054069-*`
- `0-*`

都会被拆成：

- 时间戳部分
- sequence 部分
- 是否带 wildcard sequence

这个 helper 会同时被写路径和读路径复用，所以 stream ID 的解释逻辑集中在一处。

## `XADD`

`xadd_cmd(...)` 是整个文件里最密集的 stream helper。

它的控制流是：

1. 如果传入 ID 是 `*`，先改写成 `now_in_millis()-*`
2. 用 `split_offset(...)` 解析 ID
3. 显式拒绝 `0-0`
4. 锁住 `server.streams`
5. 如果 stream 不存在就创建
6. 拿新 ID 和 stream 尾部最后一个 ID 比较
7. 如果是同一毫秒且用了 wildcard sequence，就自动递增 sequence
8. 把 field/value 对插入目标 entry
9. 唤醒阻塞 reader
10. 调 `resp_and_replicate(...)`

最终返回值是实际插入成功的 entry ID。

## 有序性校验

和 stream 尾部比较的逻辑是这里最关键的约束之一：

- 时间戳比尾部小 -> 拒绝
- 时间戳相同且显式 sequence 不大于尾部 -> 拒绝
- 时间戳相同且 sequence 是 wildcard -> 自动补成下一个 sequence

这就是当前实现保持 stream ID 单调递增的方式，并没有单独抽象出一个 sequence allocator。

## `XRANGE`

`xrange_cmd(...)` 结构上简单很多。

它会：

1. 锁住 `server.streams`
2. 处理特殊边界值
3. 调 `BTreeMap::range(...)`
4. 把命中的 entry 序列化成 `Protocol::Array`

特殊边界值规则：

- `-` -> `"0"`
- `+` -> `u64::MAX.to_string()`

返回结构是扁平的，交替出现：

```text
entry-id, field-value-array, entry-id, field-value-array, ...
```

## `XREAD`

`xread_cmd(...)` 支持：

- 多个 stream key
- 多个起始 offset
- 可选的 `BLOCK`

控制流是：

1. `Cmd::from(...)` 里先可选解析 `block <millis>`
2. 如果是 `BLOCK <millis>` 且 `millis > 0`，先 sleep 指定时长
3. 如果是 `BLOCK 0`，就在 `server.stream_reader_blocker` 里注册一个 sender，然后阻塞等待 receiver
4. 锁住 `server.streams`
5. 对每个 stream 计算“起始 offset 的下一个 ID”
6. 调 `BTreeMap::range(...)`
7. 把结果拼成一个扁平 RESP array

这里通过把 sequence 加一来实现“从给定 offset 之后开始读”的排他下界。

## 当前 blocking 模型

reader 等待队列是：

```text
Arc<Mutex<Vec<Sender<()>>>>
```

`XADD` 成功后会：

1. 锁住这个 sender 向量
2. 给每个 sender 发一个空信号
3. 清空向量

这是一个刻意保持很小的协调机制：

- 全局 waiter 列表
- 不按 stream key 分桶
- 没有公平性策略
- 也没有单独的超时取消结构

## 一个必须写清楚的当前行为：`BLOCK 0`

`BLOCK 0` 这条分支本来意图是“等到后面有 `XADD` 把 reader 唤醒”。

但当前接收循环是：

```rust
while let Some(_) = receiver.recv().await {
    println!("get new xadd cmd, release block");
}
```

它在第一次收到通知后并不会 `break`。

因此当前实现实际上不是“收到一次唤醒就返回”，而是“继续等到 channel 关闭”。这和预期的 Redis 语义之间还有明显差距。

## 事务队列结构

事务状态没有挂在全局 `Server` 上。

每个连接循环在 `Server::handle(...)` 里维护自己的：

```text
Option<Vec<(Cmd, Protocol)>>
```

这意味着事务状态天然是：

- connection-local
- 纯内存
- 对其他客户端不可见

对于这个仓库来说，这样的结构很合理，但文档里需要明确讲出来。

## `MULTI`、`EXEC`、`DISCARD`

事务控制主要分布在 `Cmd::run(...)` 和 `exec_cmd(...)`。

`MULTI`：

- 把 `queued_cmd` 设成 `Some(Vec::new())`
- 返回 `ok`

事务开启后的普通命令：

- 把 `(Cmd, Protocol)` push 进队列
- 返回 `QUEUED`

`EXEC`：

1. 遍历队列里的每一条命令
2. 对每条命令调用 `cmd.run(server, protocol.clone(), is_rep_con, &mut None)`
3. 收集每条返回值，拼成 `Protocol::Array`
4. 清空队列

`DISCARD`：

- 如果队列存在，就丢弃并返回 `ok`
- 否则返回 `ERR Discard without MULTI`

## 为什么队列里同时存 `Cmd` 和 `Protocol`

这个 pair 是有意义的。

- `Cmd` 是已经解析好的语义形式，重放时可直接 dispatch
- `Protocol` 仍然要给那些依赖原始消息做复制或 offset 计数的 helper 使用

所以 `EXEC` 可以重用正常执行路径，而不必再重新解析一遍原始命令文本。

## 和复制链路的交互

队列里的命令在 `EXEC` 时仍然是通过普通 `cmd.run(...)` 重放。

这意味着事务重放会继承普通执行路径的现有行为：

- 用 `resp_and_replicate(...)` 的命令，仍然会按角色去复制或拒绝
- 像当前 `INCR` 这种本地更新型 helper，在事务里也会保持同样的局部行为

这也是“复用一条执行路径”的典型副作用：代码更短，但现在有哪些 quirks，事务里也会原样保留。

## 当前实现限制

- stream waiter 是全局的，不是按 stream 分开的
- `BLOCK <millis>` 是先 sleep 再读，不是事件驱动的 wait-with-timeout
- `BLOCK 0` 收到第一次唤醒后不会立刻 break
- stream 返回结构是当前实现自定义的扁平编码
- 事务状态是连接局部、非持久化的
- `EXEC` 重放没有额外的原子回滚语义

即使有限制，stream 和事务仍然很适合作为单独章节，因为它们在源码里确实形成了独立的实现边界，而且能一口气顺着读下来。
