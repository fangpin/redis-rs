---
title: 命令执行
layout: default
nav_order: 8
permalink: /zh/docs/command-execution/
---

# 命令执行

这一章看 RESP 解析结果是怎样变成 typed command，并最终落到具体行为上的。

## 文件边界

- `src/cmd.rs`
- `src/server.rs`
- `src/protocol.rs`

## 为什么 `cmd.rs` 是语义中心

`src/cmd.rs` 是这个仓库里几个边界真正汇合的地方：

- RESP array 变成 typed command
- 命令处理器被分发
- 写命令和复制逻辑交叉
- stream 命令挂到普通命令路径上
- 事务队列和重放也在这里实现

如果说 `server.rs` 是运行时外壳，那 `cmd.rs` 就是这个服务真正有“Redis 行为感”的地方。

## `Cmd` 枚举

`Cmd` 枚举覆盖了当前支持的命令面：

- 基础命令：`Ping`、`Echo`
- 字符串命令：`Get`、`Set`、`SetPx`、`SetEx`、`Del`、`Incr`
- 观察类命令：`Keys`、`ConfigGet`、`Info`、`Type`
- 复制命令：`Replconf`、`Psync`
- stream 命令：`Xadd`、`Xrange`、`Xread`
- 事务命令：`Multi`、`Exec`、`Discard`
- 兜底：`Unknow`

`Unknow` 这个拼写错误就是当前源码的一部分，后面的 fallback 行为也沿用了它。

## `Cmd::from(...)`

`Cmd::from(...)` 接收的是原始 RESP 文本，不是已经解析好的 protocol 对象。

它的控制流是：

1. 先调 `Protocol::from(s)`
2. 要求顶层一定是 `Protocol::Array`
3. 对每个子元素调 `decode()`，压平成 token 向量
4. 匹配 `cmd[0]`
5. 对每个支持的命令校验参数个数和结构
6. 返回 `(Cmd, original_protocol)`

这里同时返回两个值是个很好的设计点：

- `Cmd` 用来做后续语义分发
- `Protocol` 原件后面还要拿去做复制广播和 offset 计数

## 为什么协议层的小写化会影响这里

由于 `Protocol::parse_bulk_string_sfx(...)` 在解析 bulk string 时会统一 `to_lowercase()`，所以 `Cmd::from(...)` 看到的 token 基本都是小写。

这让命令匹配非常简单：

- `"set"` 不用再考虑大小写
- `"px"`、`"ex"` 都能直接比较

但代价同样会一路传下来：

- key 被转成小写
- value 也被转成小写

这是协议层 shortcut 对命令层造成的直接影响。

## 按命令族解析参数

`Cmd::from(...)` 当前对每类命令都写了非常明确、也比较窄的参数解析逻辑。

例如：

- `set` 通过参数长度和位置区分普通版、`px` 版、`ex` 版
- `config` 只支持 `config get <name>`
- `keys` 只支持 `keys *`
- `xadd` 从索引 `3` 开始按 pair 收集 field/value
- `xread` 会先可选地解析 `block <millis>`，再把 stream keys 和 offsets 平分

不支持的参数形状会直接返回 `DBError`，不会拖到命令 helper 里再判断。

## `Cmd::run(...)`

`Cmd::run(...)` 是中央分发器。

它的输入包括：

- `&mut Server`
- 原始 `Protocol`
- `is_rep_con`
- `queued_cmd`

输出统一是 `Result<Protocol, DBError>`。

分发表会把不同命令送到对应 helper，比如：

- `get_cmd(...)`
- `set_cmd(...)`
- `config_get_cmd(...)`
- `info_cmd(...)`
- `replconf_cmd(...)`
- `xadd_cmd(...)`
- `xread_cmd(...)`
- `exec_cmd(...)`

这个结构最大的好处就是：解析逻辑和命令实现没有混在一起。

## `run(...)` 里先处理事务队列

正式分发之前，`run(...)` 先看当前连接是不是已经处于事务队列模式。

如果 `queued_cmd` 已经存在，而且当前命令不是 `EXEC`、`MULTI`、`DISCARD`，它会：

1. 把 `(self.clone(), protocol.clone())` push 进队列
2. 直接返回 `QUEUED`

这就是为什么事务里的普通命令不会立即修改存储。

## 字符串命令 helper

字符串命令基本都是对 `Storage` 的薄封装。

比如：

- `get_cmd(...)` -> `storage.get(...)`
- `set_cmd(...)` -> `storage.set(...)`
- `set_px_cmd(...)` -> `storage.setx(..., px_millis)`
- `set_ex_cmd(...)` -> `storage.setx(..., seconds * 1000)`
- `del_cmd(...)` -> `storage.del(...)`

命令层负责解释参数和定义响应形状，存储层只负责点操作。

## 观察类 helper

`config_get_cmd(...)` 直接从 `server.option` 里读配置。

当前只支持两个名字：

- `dir`
- `dbfilename`

`info_cmd(...)` 也很窄，目前只支持 `replication` section，并且返回的是根据 `server.option.replication` 拼出来的一小段文本。

这意味着 `INFO replication` 更像“配置快照”，而不是完整的运行时实时状态。

## `type_cmd(...)`

`type_cmd(...)` 把仓库的“双容器模型”直接暴露给了客户端。

控制流是：

1. 先锁字符串存储并调用 `get(...)`
2. 如果存在，返回 `string`
3. 否则锁 `streams`
4. 如果存在，返回 `stream`
5. 否则返回 `none`

这是个很小但很有说明力的 helper，因为它把状态布局直接映射成了客户端可见行为。

## `replconf_cmd(...)` 和 `psync_cmd(...)`

`replconf_cmd(...)` 当前实现非常浅。

- `getack` -> 根据 `server.offset` 构造 `REPLCONF ACK <offset>`
- 其他子命令 -> 一律返回 `OK`

所以像 `REPLCONF listening-port ...` 和 `REPLCONF capa psync2` 在命令层并没有更细的参数校验。

`psync_cmd(...)` 也很窄：

- master -> 返回 `FULLRESYNC <master_replid> 0`
- slave -> 返回 `PSYNC ON SLAVE IS NOT ALLOWED`

真正的 socket 模式切换是在后面的 `Server::handle(...)` 里做的，不在这里。

## 共享写策略：`resp_and_replicate(...)`

多个写命令最后都会进入 `resp_and_replicate(...)`。

它把角色规则集中到了一处：

- master -> 向下游副本广播原始命令，再返回本地响应
- slave 收到普通客户端写请求 -> 拒绝
- slave 从复制连接收到回放命令 -> 接受

这样 `SET`、`DEL`、`XADD` 等命令就不需要各自重复实现一套角色判断。

## 一个很值得写出来的例外：`INCR`

`incr_cmd(...)` 会先读字符串，再 parse 成数字，加一后写回字符串。

但它和 `SET`、`DEL`、`XADD` 不一样，没有走 `resp_and_replicate(...)`。

所以当前实现下：

- `INCR` 会修改本地存储
- `INCR` 不会被传播给下游副本
- `INCR` 也不会通过共享写保护逻辑去拒绝 slave 上的普通客户端写请求

这是源码层面当前存在的行为差异，文档里需要明确写出来。

## offset 计数

每次命令成功后，`Cmd::run(...)` 都会用 `p.encode().len()` 去增加一次 `server.offset`。

但某些写 helper 内部又会手动把这个计数器额外加 `1`。

因此当前 offset 不是一套统一、严格一致的计数规则，有些写命令实际上会推进两次。

## unknown command 的处理

不认识的顶层命令名会先变成 `Cmd::Unknow`。

然后在 `run(...)` 里统一映射成 `Protocol::err("unknow cmd")`。

所以系统现在区分的是：

- 已知命令但参数形状不合法 -> `DBError`
- 未知命令名 -> fallback reply

这种分法不算特别对称，但和源码一致。

## 当前实现限制

- 命令解析假设输入一定是一条完整的顶层 RESP array
- payload 在进入命令层之前就已经统一小写了
- 一些 helper 对参数结构的假设比较窄
- `INCR` 绕过了共享复制/写保护 helper
- offset 计数在不同 helper 间并不一致
- unknown command 会落进统一的兜底响应

`cmd.rs` 仍然是读完整个项目时最值得紧跟 `server.rs` 看的文件，因为它把这个服务真正的语义骨架都集中起来了。
