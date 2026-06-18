---
title: 存储模型
layout: default
nav_exclude: true
permalink: /zh/docs/storage-model/
---

# 存储模型

[返回首页]({{ '/' | relative_url }}) | [文档总览]({{ '/zh/docs/overview/' | relative_url }}) | [RDB 解析器]({{ '/zh/docs/rdb-parser/' | relative_url }}) | [Streams 与事务]({{ '/zh/docs/streams-and-transactions/' | relative_url }})

这一章聚焦项目里的内存存储层，以及 stream 为什么没有放进同一个容器。

## 文件边界

- `src/storage.rs`
- `src/server.rs`

## 字符串 key-value 存储

`src/storage.rs` 定义了普通字符串键值存储，内部结构是：

```text
HashMap<String, (String, Option<u128>)>
```

每条记录包含：

- 字符串值
- 可选的绝对过期时间戳（毫秒）

`ValueType` 这个 type alias 就是在表达这个结构。

## 为什么过期时间存绝对时间戳

这里不会保存：

- 插入时间
- 相对 TTL

而是直接保存“绝对过期时间”。这样读路径很简单：

- 取当前时间
- 和记录中的过期时间比较
- 过期就删掉
- 没过期就返回值

这同时适配：

- `SET PX/EX`
- RDB 恢复

## `now_in_millis()`

`now_in_millis()` 是整个项目的毫秒级时间源。

它不仅用于字符串过期判断，也被 stream 的自动 ID 生成复用。

所以“毫秒时间”其实是这个仓库多个子系统共享的基础原语。

## 读路径

`Storage::get(...)` 不只是查表，它还会顺带做惰性过期处理：

1. 先查 `HashMap`
2. 如果有过期时间，就和当前时间比较
3. 如果过期，删除该键并返回 `None`
4. 否则返回值的克隆

这属于 lazy expiration，没有后台清理线程。

## 写路径

写接口保持得很小：

- `set(...)`：写普通值
- `setx(...)`：写带过期时间的值
- `del(...)`：删键
- `keys(...)`：列出当前 key

其中 `setx(...)` 会把“相对 TTL”加上 `now_in_millis()`，转换成绝对时间戳后再保存。

## 哪些数据不在这里

Stream 不在 `Storage` 里，而是单独放在 `Server::streams`：

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

这样拆开的原因很直接：

- 普通字符串键需要的是点查和可选过期
- stream 需要按 entry id 有序范围查询

内部使用 `BTreeMap`，可以天然支持有序遍历。

## 命令层如何调用存储

`cmd.rs` 里的大多数字符串命令流程都差不多：

1. 锁住 `server.storage`
2. 调用一个同步存储方法
3. 解锁
4. 构造 `Protocol` 响应

因此 async 边界是在 `Server` 的 `Mutex` 外层，而不是在 `Storage` 内部。

## 字符串命令的数据流

像 `GET`、`SET`、`DEL`、`INCR` 的数据流可以概括成：

```text
cmd.rs helper
-> lock server.storage
-> 调用 Storage 方法
-> 构造 Protocol 响应
```

这也是这个仓库可读性较高的原因之一：命令层没有把存储语义重新实现一遍。

## 当前实现限制

- 所有值都按字符串保存
- 过期清理是惰性的
- 没有容量统计和淘汰策略
- `keys()` 不会主动过滤那些尚未被读取过的过期键

这些限制是合理的，因为当前目标是把存储模型讲清楚，而不是实现完整 Redis 内存管理。
