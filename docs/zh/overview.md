---
title: 文档总览
layout: default
nav_exclude: true
permalink: /zh/docs/overview/
---

# 文档总览

[返回首页]({{ '/' | relative_url }}) | [项目概览]({{ '/overview/' | relative_url }}) | [运行时与服务端]({{ '/zh/docs/server-runtime/' | relative_url }}) | [RESP 协议]({{ '/zh/docs/resp-protocol/' | relative_url }})

这里的文档区现在按实现边界来组织 `redis-rs`，而不是把源码压成几篇泛概述。

每个主章节都会回答同一组具体问题：

- 这段行为归哪个文件负责
- 入口函数是谁
- 控制流先发生什么、后发生什么
- 关键数据结构怎么流动
- 如果继续扩展，这里的 shortcut 和限制是什么

## 推荐阅读路径

如果你想按控制流理解整个实现，可以按这个顺序：

1. 运行时入口与共享服务端状态
2. RESP 解析和编码
3. 字符串存储与 stream 容器
4. RDB snapshot 解析
5. 复制握手与 fan-out
6. 命令分发
7. Stream 与事务重放

## 章节地图

- [运行时与服务端]({{ '/zh/docs/server-runtime/' | relative_url }})
  - `src/main.rs`
  - `src/server.rs`
  - `src/options.rs`
- [RESP 协议]({{ '/zh/docs/resp-protocol/' | relative_url }})
  - `src/protocol.rs`
- [存储模型]({{ '/zh/docs/storage-model/' | relative_url }})
  - `src/storage.rs`
  - `src/server.rs`
  - `src/cmd.rs`
- [RDB 解析器]({{ '/zh/docs/rdb-parser/' | relative_url }})
  - `src/rdb.rs`
- [主从复制链路]({{ '/zh/docs/replication-flow/' | relative_url }})
  - `src/main.rs`
  - `src/replication_client.rs`
  - `src/server.rs`
  - `src/cmd.rs`
- [命令执行]({{ '/zh/docs/command-execution/' | relative_url }})
  - `src/cmd.rs`
  - `src/server.rs`
  - `src/protocol.rs`
- [Streams 与事务]({{ '/zh/docs/streams-and-transactions/' | relative_url }})
  - `src/cmd.rs`
  - `src/server.rs`
  - `src/storage.rs`

## 和旧版章节拆分相比，这次补充了什么

这个仓库虽然不大，但实际实现边界并不少：

- 入口 / 运行时编排
- RESP 解析
- 存储层
- RDB 解码
- 复制握手与 fan-out
- 命令语义
- stream / 事务扩展

现在的拆分会把这些边界保持可见，同时也会把“当前实现和真实 Redis 的差距”如实写出来，而不是把文档写成理想化设计图。
