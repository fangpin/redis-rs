---
title: RDB 解析器
layout: default
nav_order: 6
permalink: /zh/docs/rdb-parser/
---

# RDB 解析器

这一章单独看 snapshot 解析器。它既服务本地恢复，也服务主从复制初始化。

## 文件边界

- `src/rdb.rs`

## 模块职责

`src/rdb.rs` 的职责很单一：把一个 RDB 字节流解析成内存写入。

它会在两个地方被复用：

- master 启动时从本地 DB 文件恢复
- follower 复制握手后消费 master 发送过来的 snapshot

这个复用很重要，因为仓库里只有一条 RDB 解码路径，没有“文件恢复版”和“网络复制版”两套实现。

## 入口函数

当前有两个公开入口：

- `parse_rdb_file(...)`
- `parse_rdb(...)`

`parse_rdb_file(...)` 只是面向文件的包装器。

它做的事只有：

1. 把 `tokio::fs::File` 包成 `BufReader`
2. 转调 `parse_rdb(...)`

真正的解析逻辑都在 `parse_rdb(...)`。因为它接收的是 `AsyncRead + Unpin`，所以既能吃文件，也能吃网络流。

## 高层解析顺序

`parse_rdb(...)` 一开始就锁住 `server.storage`，然后按这个顺序走：

1. `parse_magic(...)`
2. `parse_version(...)`
3. 进入 opcode dispatch loop
4. 遇到 `EOF` 结束

当前支持的 opcode 有：

- `META` (`0xFA`)
- `DB_SELECT` (`0xFE`)
- `TABLE_SIZE_INFO` (`0xFB`)
- `EOF` (`0xFF`)

遇到其他值就会报错。

## 头部校验

`parse_magic(...)` 会精确读取 5 个字节，并要求它等于 `REDIS`。

`parse_version(...)` 再读取 4 个字节版本号并原样返回。

这里做的是文件结构级别的校验，但后面没有按不同版本分支处理。

## metadata 处理

循环读到 `META` 时，会连续调用两次 `parse_aux(...)`，然后直接丢掉结果。

也就是说，这个解析器“结构上知道 metadata 的存在”，但“语义上不使用它”。

这很符合这个仓库的风格：

- 尽量保持对真实格式的结构对齐
- 但不为当前运行时用不到的状态额外建模

## DB 选择和 table size

`DB_SELECT` 会被解析，但结果被忽略，因为这个服务基本只当成一个逻辑 DB 来用。

`TABLE_SIZE_INFO` 则不只是装饰字段，它会驱动后续两段 entry 解析：

1. 读取 `size_no_expire`
2. 读取 `size_expire`
3. 解析对应数量的非过期 entry
4. 解析对应数量的过期 entry

也就是说，当前实现默认 snapshot 会按预期的分组顺序组织。

## entry 解析函数

每条记录会走两个不同 helper：

- `parse_no_expire_entry(...)`
- `parse_expire_entry(...)`

`parse_no_expire_entry(...)` 会要求下一个字节必须是类型 `0`，然后通过 `parse_aux(...)` 读取 key 和 value。

这说明当前解析器只支持字符串 value。

`parse_expire_entry(...)` 则会先读过期前缀，再回到 `parse_no_expire_entry(...)` 解析正文。

当前支持两种过期编码：

- `0xFC` -> 8 字节 little-endian 毫秒时间戳
- `0xFD` -> 4 字节 little-endian 秒级时间戳

秒级时间戳会立刻转成毫秒。

## 长度解析

`parse_len(...)` 是底层长度辅助函数。

它返回：

- 解码后的长度
- `StringEncoding`

当前支持的编码类型有：

- `Raw`
- `I8`
- `I16`
- `I32`
- `LZF`

`parse_string(...)` 除了 `LZF` 之外都能解码；遇到 `LZF` 会显式报错。

## 一个需要明确指出的实现细节：长度前缀支持仍然不完整

当前实现显然是在尝试映射 Redis RDB 的长度前缀规则，但分支还没有完全对齐。

例如，本来用于处理 14-bit length 的分支，当前匹配的是 `0x04`，不是 `0x40`。

所以这个模块已经把“格式长什么样”表达出来了，但并没有完整覆盖所有前缀。

文档里应该把它当作当前实现限制，而不是写得像已经完整支持。

## 数据如何流入 storage

这个解析器不会先构建一个中间 snapshot 对象图，而是边读边写。

普通 entry：

```text
parse_no_expire_entry
-> storage.set(key, value)
```

过期 entry：

```text
parse_expire_entry
-> storage.setx(key, value, expire_timestamp)
```

## 当前恢复路径里和过期时间有关的注意点

这里必须按源码真实行为来写。

`parse_expire_entry(...)` 解析出来的是“绝对过期时间戳”。

但 `storage.setx(...)` 期待的是“相对 TTL 毫秒数”，并且会再加一遍 `now_in_millis()`。

因此当前实现下，RDB 恢复出来的带过期 key 并不会精确保留原始绝对截止时间，而是会被再次当成相对 TTL 处理。

## follower 初始化时的复用

在复制握手阶段，`FollowerReplicationClient::recv_rdb_file(...)` 最终会调用 `rdb::parse_rdb(&mut reader, server)`。

也就是说，follower 应用 master snapshot 时，用的还是同一套解析器、同一套写入逻辑，也包括上面提到的过期时间行为。

## EOF 和 CRC

读到 `EOF` 后，解析器会继续读取一个尾部 `u64` CRC 字段，但不会做校验。

所以字节流对齐是对的，只是 checksum 还没有真正验证。

## 当前实现限制

- 只支持字符串 value
- metadata 会解析但会被忽略
- DB index 会解析但会被忽略
- `LZF` 编码不支持
- checksum 不校验
- 长度前缀支持仍然不完整
- 带过期时间的 snapshot entry 复用的是相对 TTL 写入接口

即使有限制，`src/rdb.rs` 仍然是仓库里很值得细读的一块，因为它把“本地恢复”和“复制初始化”串进了同一条解码路径里。
