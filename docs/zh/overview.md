---
title: 文档总览
layout: default
nav_exclude: true
permalink: /zh/docs/overview/
---

# 文档总览

[返回首页]({{ '/' | relative_url }}) | [项目概览]({{ '/overview/' | relative_url }}) | [运行时与服务端]({{ '/zh/docs/server-runtime/' | relative_url }}) | [RESP 协议]({{ '/zh/docs/resp-protocol/' | relative_url }})

这里的文档区不再把实现细节压成少数几篇概览，而是按源码职责拆开，便于顺着代码边界逐步阅读。

## 推荐阅读路径

如果你想按控制流理解整个实现，可以按这个顺序：

1. 运行时入口与共享服务端状态
2. RESP 协议解析和编码
3. 字符串键值存储模型
4. RDB 文件解析
5. 主从复制链路
6. 命令分发
7. Stream 与事务

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
- [RDB 解析器]({{ '/zh/docs/rdb-parser/' | relative_url }})
  - `src/rdb.rs`
- [复制链路]({{ '/zh/docs/replication-flow/' | relative_url }})
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

## 为什么改成这个结构

这个仓库虽然不大，但实现边界是清楚的：

- 入口和运行时
- 协议层
- 存储层
- RDB 持久化
- 复制链路
- 命令语义
- Stream / 事务扩展

如果只保留 3 个大章节，很多控制流和数据流会被压平，看不出模块职责。
