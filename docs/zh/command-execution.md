---
title: 命令执行
layout: default
nav_exclude: true
permalink: /zh/docs/command-execution/
---

# 命令执行

[返回首页]({{ '/' | relative_url }}) | [文档总览]({{ '/zh/docs/overview/' | relative_url }}) | [RESP 协议]({{ '/zh/docs/resp-protocol/' | relative_url }}) | [Streams 与事务]({{ '/zh/docs/streams-and-transactions/' | relative_url }})

这一章聚焦 `cmd.rs`，也就是整个仓库里命令语义最密集的地方。

## 文件边界

- `src/cmd.rs`
- `src/server.rs`
- `src/protocol.rs`

## 为什么 `cmd.rs` 是语义中心

`src/cmd.rs` 是仓库里体量最大的行为模块之一，这里同时承担了：

- 把协议数组变成命令枚举
- 把命令分发到具体 helper
- 把写命令和复制规则接起来
- 把 stream 和事务挂进同一条执行链

如果说 `server.rs` 是运行时外壳，那 `cmd.rs` 就是语义核心。

## 命令枚举

`Cmd` 总结了当前支持的命令面，包括：

- 字符串命令：`GET`、`SET`、`DEL`、`INCR`
- 信息查看命令：`CONFIG GET`、`INFO`、`TYPE`
- 复制相关命令：`REPLCONF`、`PSYNC`
- Stream 命令：`XADD`、`XRANGE`、`XREAD`
- 事务命令：`MULTI`、`EXEC`、`DISCARD`

无法识别的输入会落到 `Unknow`。

## 解析路径

`Cmd::from(...)` 从一个已解析的 `Protocol` 开始，要求顶层必须是数组。

控制流是：

1. 先调 `Protocol::from(...)`
2. 确认顶层是 `Protocol::Array`
3. 把每个子元素 `decode()` 成 token
4. 按第一个 token 匹配命令类型
5. 对每条命令检查参数个数和形状
6. 返回 `(Cmd, Protocol)`

这里同时返回原始 `Protocol` 很重要，因为后面复制广播还需要原始命令载荷。

## 分发路径

`Cmd::run(...)` 是中央调度器。

它接收：

- `&mut Server`
- 原始 `Protocol`
- `is_rep_con`
- `queued_cmd`

再把命令路由到各个 helper，例如：

- `get_cmd(...)`
- `set_cmd(...)`
- `keys_cmd(...)`
- `info_cmd(...)`
- `xadd_cmd(...)`
- `exec_cmd(...)`

这样解析、调度和具体语义就被分层了。

## 统一的写命令 helper

很多写命令最终都会走 `resp_and_replicate(...)`。

它负责统一决定：

- 本地应该返回什么响应
- 是否要向 replicas 广播原始命令
- slave 是否要拒绝普通客户端写入

这避免了每个写命令都重复实现一遍复制规则。

## 信息查看类命令

`config_get_cmd(...)` 和 `info_cmd(...)` 基本不碰存储层，它们只是把 `server.option` 里的配置和复制状态序列化成 `Protocol` 响应。

这给项目提供了一条轻量的自描述接口。

## `KEYS` 和 `TYPE`

`keys_cmd(...)` 直接从字符串存储里返回当前键集合。

`type_cmd(...)` 则会先查普通存储，再查 stream 容器，然后返回：

- `string`
- `stream`
- `none`

它是“字符串容器”和“stream 容器”两条数据线在客户端层面汇合的一个例子。

## `INCR`

`incr_cmd(...)` 的流程是：

1. 读出当前值
2. 如果 key 不存在，默认当作 `1`
3. 尝试把字符串解析成 `u64`
4. 自增并写回为字符串
5. 如果解析失败，返回显式错误

这也说明当前存储层始终是“字符串优先”，数值语义是在命令层附加出来的。

## Offset 统计

命令成功执行后，`Cmd::run(...)` 会用原始协议编码后的长度递增 `server.offset`。

所以 offset 统计附着在“命令完成”这个节点上，而不是附着在 socket read 位置上。

这个模型很粗，但在整个仓库里是一致的。

## 当前实现限制

- 命令解析默认输入已经是完整数组
- 很多分支仍然假设参数形状良好
- 不支持的命令会收敛到统一 unknown 分支
- 有些 helper 内部还保留了 unwrap 风格

尽管如此，`cmd.rs` 已经足够清楚地展示一个 Redis-like 服务器的命令驱动结构。
