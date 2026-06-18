---
title: RDB 解析器
layout: default
nav_exclude: true
permalink: /zh/docs/rdb-parser/
---

# RDB 解析器

[返回首页]({{ '/' | relative_url }}) | [文档总览]({{ '/zh/docs/overview/' | relative_url }}) | [存储模型]({{ '/zh/docs/storage-model/' | relative_url }}) | [复制链路]({{ '/zh/docs/replication-flow/' | relative_url }})

这一章把 RDB 快照解析单独抽出来讲，不和复制链路混在一起。

## 文件边界

- `src/rdb.rs`

## 这个模块的职责

`src/rdb.rs` 只做一件事：把 RDB 字节流解析成对 `server.storage` 的写入。

它被复用在两个场景：

- master 本地启动时从 DB 文件恢复
- slave 从 master 下载快照后恢复

这点很重要，因为仓库里只有一条 RDB 解码路径。

## 入口函数

模块对外有两个入口：

- `parse_rdb_file(...)`
- `parse_rdb(...)`

`parse_rdb_file(...)` 只是包装了一层 `BufReader`。

真正的流式解析逻辑都在 `parse_rdb(...)` 里，它接受任何 `AsyncRead + Unpin`，所以网络流和文件流都能复用。

## 总体控制流

`parse_rdb(...)` 的流程是：

1. 锁住 `server.storage`
2. `parse_magic(...)`
3. `parse_version(...)`
4. 进入 opcode 循环
5. 遇到 `EOF` 结束

循环里当前识别这些 opcode：

- `META`
- `DB_SELECT`
- `TABLE_SIZE_INFO`
- `EOF`

其他 opcode 都直接报错。

## Header 校验

`parse_magic(...)` 会读取 5 个字节并校验是否为 `REDIS`。

`parse_version(...)` 再读取 4 个字节版本号。

当前实现只验证结构，不做更细的版本分支逻辑。

## Metadata 段

当遇到 `META` 时，代码会调用两次 `parse_aux(...)`，然后丢弃结果。

也就是说 metadata 是“语法上解析了，但语义上忽略了”。

这个取舍很合理：

- 结构上贴近真实 RDB
- 又不需要额外引入一套 metadata 模型

## DB 选择和表大小信息

`DB_SELECT` 会被读出来，但因为这个项目实际上只处理单个逻辑 DB，所以结果被忽略。

`TABLE_SIZE_INFO` 则更关键，它提供：

- 不带过期时间的 entry 数量
- 带过期时间的 entry 数量

随后解析器就按这两个计数分别读取对应数量的 entry。

## Entry 解析

目前有两条 entry 解析路径：

- `parse_no_expire_entry(...)`
- `parse_expire_entry(...)`

两者都默认 value type 为字符串类型 `0`。

所以这个实现聚焦的是字符串键，而不是完整 Redis 类型矩阵。

## 过期时间解码

`parse_expire_entry(...)` 支持两种编码：

- `0xFC`：8 字节 little-endian 毫秒时间戳
- `0xFD`：4 字节 little-endian 秒级时间戳

秒级时间戳会先转成毫秒，再写入存储层。

这样整个仓库内部就统一成毫秒时间单位。

## 长度与字符串解码

`parse_len(...)` 会解释 RDB 的长度前缀，并返回：

- 解码后的长度
- `StringEncoding`

目前支持的字符串编码有：

- `Raw`
- `I8`
- `I16`
- `I32`
- `LZF`

`parse_string(...)` 可以处理前四种，`LZF` 则明确返回“不支持”。

## 如何写入存储层

entry 一旦解析出来，就会立刻写入 `storage`：

- 不带过期 -> `storage.set(...)`
- 带过期 -> `storage.setx(...)`

这里不会先构造一棵中间对象树，而是边读边落到运行时状态里。

## CRC 处理

读到 `EOF` 后，代码会再读 8 字节 CRC，但不会验证它。

也就是说文件尾结构被消费了，但校验逻辑还没实现。

## 当前实现限制

- 只支持 RDB 的一部分结构
- metadata 被忽略
- DB index 被忽略
- 只处理字符串类型 entry
- LZF 不支持
- CRC 不校验

尽管如此，这个解析器已经足够展示 Redis 持久化格式的关键骨架，并且能支撑本地恢复和复制快照恢复。
