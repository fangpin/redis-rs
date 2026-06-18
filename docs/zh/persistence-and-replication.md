---
title: 持久化与复制
layout: default
nav_exclude: true
permalink: /zh/docs/persistence-and-replication/
---

# 持久化与复制

[返回首页]({{ '/' | relative_url }}) | [总览]({{ '/overview/' | relative_url }}) | [运行时与协议]({{ '/zh/docs/runtime-and-protocol/' | relative_url }}) | [命令、流与事务]({{ '/zh/docs/commands-streams-and-transactions/' | relative_url }})

这一章覆盖项目如何从 RDB 文件恢复状态，以及 master/slave 之间的同步链路是怎么接起来的。

## 文件边界

- `src/rdb.rs`
- `src/replication_client.rs`
- `src/server.rs`

## RDB 加载入口

master 启动时会在 `Server::init(...)` 中打开配置里的 DB 文件。只有文件非空时，才会进入 RDB 解析流程。

调用链如下：

1. `Server::init(...)`
2. `rdb::parse_rdb_file(...)`
3. `rdb::parse_rdb(...)`

解析出来的数据会直接写入 `server.storage`。

## 当前支持的 RDB 内容

`src/rdb.rs` 实现的是一个聚焦版 RDB 解析器，当前覆盖：

- magic header 校验
- 版本读取
- metadata 辅助字段
- 数据库选择标记
- table size 信息
- 不带过期时间的字符串键值
- 带过期时间的字符串键值
- EOF 与 CRC 读取

其中过期时间支持两种编码：

- `0xFC`：毫秒时间戳
- `0xFD`：秒时间戳

如果 entry 自带过期时间，解析器会走 `Storage::setx(...)`。

## 字符串解码模型

当前支持的 string encoding 有：

- 原始字符串
- 8 位整数
- 16 位整数
- 32 位整数

LZF 压缩字符串明确返回“不支持”，不会被悄悄跳过。

## 持久化与存储层的配合

`src/storage.rs` 很小，只做三件关键事：

- `set(...)` 写普通值
- `setx(...)` 写带绝对过期时间的值
- `get(...)` 在读取时顺便清理过期键

这让 RDB 层只需要判断“有没有过期信息”，然后调用正确的存储接口即可。

## 从节点握手流程

`src/replication_client.rs` 里的 `FollowerReplicationClient` 按顺序完成：

1. `PING`
2. `REPLCONF listening-port <port>`
3. `REPLCONF capa psync2`
4. `PSYNC ? -1`

每一步都会通过 `check_resp(...)` 校验返回值是否符合预期。

## 全量同步

`PSYNC` 之后，从节点会继续读取：

1. `FULLRESYNC` 响应行
2. `$<len>` 形式的 RDB 长度头
3. 真正的 RDB 二进制快照

拿到快照后，不会再走另一套专门逻辑，而是直接复用 `rdb::parse_rdb(...)`。

这点很重要：本地文件恢复和主从快照恢复，共用的是同一条 RDB 解码路径。

## Master 侧复制行为

当 master 在 `Server::handle(...)` 中收到 `PSYNC`：

1. 通过 `MasterReplicationClient::send_rdb_file(...)` 发出 RDB 数据
2. 用 `add_stream(...)` 把 replica 的 socket 记下来
3. 不再把这个连接当成普通请求-响应客户端

后续复制写命令的 fan-out，则由 `MasterReplicationClient::send_command(...)` 负责。

## 当前实现限制

- master 发送的是常量的空 RDB 种子内容
- CRC 被读取但不校验
- metadata 会被解析但暂时忽略
- 复制状态跟踪远小于生产级 Redis

这些取舍让重点落在“如何恢复状态、如何建立角色关系、如何开始回放写流量”上。
