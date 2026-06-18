---
title: RESP 协议
layout: default
nav_exclude: true
permalink: /zh/docs/resp-protocol/
---

# RESP 协议

[返回首页]({{ '/' | relative_url }}) | [文档总览]({{ '/zh/docs/overview/' | relative_url }}) | [运行时与服务端]({{ '/zh/docs/server-runtime/' | relative_url }}) | [命令执行]({{ '/zh/docs/command-execution/' | relative_url }})

这一章单独讨论项目里的 RESP 模型，以及它如何把 socket 输入和命令执行连接起来。

## 文件边界

- `src/protocol.rs`

## 协议枚举

项目内部使用 `Protocol` 作为统一协议表示，当前支持：

- `SimpleString(String)`
- `BulkString(String)`
- `Null`
- `Array(Vec<Protocol>)`

这已经足够覆盖当前仓库的命令输入和响应输出。

## 为什么这一层重要

项目里几乎所有模块都建立在 `Protocol` 之上：

- `Cmd::from(...)` 依赖它来判断收到的是哪条命令
- `Protocol::encode(...)` 负责把执行结果重新变成 RESP 文本

所以它既是入站适配层，也是出站适配层。

## 解析入口

`Protocol::from(protocol: &str)` 是总入口。

它会检查首字符，并分发到三个后缀解析器：

- `parse_simple_string_sfx(...)`
- `parse_bulk_string_sfx(...)`
- `parse_array_sfx(...)`

返回值是一个二元组：

- 解析后的 `Protocol`
- 消耗掉的字节长度

第二个值对于数组递归解析尤其重要。

## Simple String

`parse_simple_string_sfx(...)` 会找到第一个 `\\r\\n`，并把之前的内容作为 `SimpleString`。

这一支最简单：

- 没有长度前缀
- 没有嵌套结构
- 直接切片取值

## Bulk String

`parse_bulk_string_sfx(...)` 分两步：

1. 先解析前面的长度声明
2. 再取出对应长度的 payload，并校验实际长度是否匹配

如果声明长度和实际内容长度不匹配，就直接报错，不做容错恢复。

## 一个很重要的行为：会强制转小写

bulk string 被接受后，当前实现会保存成：

```rust
Protocol::BulkString(s.to_lowercase())
```

这有利有弊。

好处：

- 命令关键字天然大小写不敏感

代价：

- bulk string 的原始大小写不会被保留
- payload 不再是完全按字节保真的

对于教学项目这是可接受的，但它确实和完整 Redis 行为不完全一致。

## 数组解析

`parse_array_sfx(...)` 会先读数组长度，再循环调用 `Protocol::from(...)` 去解析后续子元素。

每个子元素都会返回“自己消耗了多少字节”，父解析器就用 `offset` 持续向前推进。

数据流可以概括成：

```text
array header
-> child 1 parse + len
-> child 2 parse + len
-> ...
-> Protocol::Array(vec)
```

这也是为什么顶层解析函数必须返回“值 + 消耗长度”。

## 构造辅助函数

`protocol.rs` 还提供了一些命令层常用的 helper：

- `from_vec(...)`
- `ok()`
- `err(...)`
- `write_on_slave_err()`
- `psync_on_slave_err()`
- `none()`

这样 `cmd.rs` 不需要在每个分支里手写小段 RESP 结构。

## 编码路径

`encode()` 负责把 `Protocol` 转回 RESP 文本：

- `SimpleString` -> `+...\\r\\n`
- `BulkString` -> `$len\\r\\npayload\\r\\n`
- `Array` -> `*len\\r\\n` + 子元素编码结果
- `Null` -> `$-1\\r\\n`

这套编码路径被多个地方复用：

- 普通客户端响应
- 复制握手消息
- 向 replica 广播写命令

## 人类可读的 `decode()`

`decode()` 会把 `Protocol` 展平成普通字符串。

命令解析阶段大量依赖它，尤其是把数组转成 token 列表时。

对数组来说，它会把子元素用空格拼接起来。这非常适合当前仓库的命令式解析需求，但并不是完全保结构的展示形式。

## 当前实现限制

- 解析输入是 `&str`，不是原始字节流
- 没有更细的 RESP 类型区分
- 结构上支持嵌套数组，但命令层只消费很窄的一部分
- bulk string 强制小写会改变 payload 语义

尽管如此，这一层依然是整个项目最关键的 wire-format 边界。
