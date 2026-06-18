---
title: 存储模型
layout: default
nav_order: 5
permalink: /zh/docs/storage-model/
---

# 存储模型

这一章看字符串命令和 stream 命令背后的内存状态容器。

## 文件边界

- `src/storage.rs`
- `src/server.rs`
- `src/cmd.rs`

## 两类存储容器

这个仓库没有把所有 Redis 类型都放进一个统一的 value enum。

而是拆成了两个独立容器：

- 普通字符串键值存在 `Storage`
- stream 存在 `Server::streams`

这就是仓库一个很核心的结构取舍：不做通用对象系统，而是按命令族拆开，让代码更容易读。

## 字符串键值的内部结构

`src/storage.rs` 定义的核心结构是：

```text
HashMap<String, (String, Option<u128>)>
```

这个 tuple 表示：

- 存储的字符串值
- 可选的过期时间戳

代码里用 `ValueType` 别名把这个语义固定了下来。

## `Storage` 的 API 面

它公开的方法很少：

- `new()`
- `get(...)`
- `set(...)`
- `setx(...)`
- `del(...)`
- `keys(...)`

这比真实 Redis 窄很多。复杂行为仍然留在 `cmd.rs`，`Storage` 基本只负责点查和写入。

## 时间源

`now_in_millis()` 会把 `SystemTime` 转成 Unix 毫秒时间戳。

这个 helper 在多个子系统里复用：

- `Storage` 里的过期判断
- `cmd.rs` 里 stream 自动生成 ID

所以“毫秒时间”已经是这个仓库多个模块共享的基本原语了。

## 过期时间模型

`Storage` 为每个 key 维护一个 `Option<u128>` 过期字段。

重点不是“存了过期时间”，而是 `setx(...)` 期待的输入语义：

```text
相对 TTL，单位是毫秒
```

`setx(...)` 总是按下面的规则写入：

```text
now_in_millis() + expire_ms
```

这个语义和下面几类写入是匹配的：

- `SET PX <millis>`
- 命令层先把秒转成毫秒后的 `SET EX <seconds>`

## `get(...)` 与惰性过期

`Storage::get(...)` 不只是简单的 map lookup。

它的控制流是：

1. 在 `HashMap` 里查 key
2. 看是否存在过期时间
3. 用 `now_in_millis()` 比较当前时间和过期时间
4. 如果已过期，删除 key 并返回 `None`
5. 否则 clone 出值

这里没有后台清理线程。过期 key 是在被访问时才被清掉的。

## 写路径

`set(...)` 存无过期的字符串。

`setx(...)` 接收相对 TTL，写成绝对截止时间。

`del(...)` 直接删 key。

`keys(...)` 只是返回当前 map 里的 key，不会先做一轮过期清扫。

这也意味着：如果某个 key 已经过期，但一直没人 `GET` 它，它仍然可能出现在 `KEYS *` 的结果里。

## stream 容器结构

stream 不在 `Storage` 里。

`src/server.rs` 里的展开类型实际上是：

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

可以读成：

- stream 名 -> stream 本体
- entry ID -> 有序记录
- 记录值 -> 一组 field/value 对

这里最关键的设计选择是 `BTreeMap`。这样 `XRANGE` 和 `XREAD` 都能直接做有序范围查询，不需要额外排序索引。

## 命令处理器如何访问存储

`src/cmd.rs` 里的大部分 helper 都遵循同一模式：

```text
lock 共享容器
-> 调一个很小的存储操作
-> 组装 Protocol 响应
```

比如：

- `GET` -> `storage.get(...)`
- `SET` -> `storage.set(...)`
- `SET PX/EX` -> `storage.setx(...)`
- `DEL` -> `storage.del(...)`
- `TYPE` -> 先查 `storage`，再查 `streams`

所以存储层本身才会这么简单：参数解释、角色判断和返回格式都在上层。

## 需要按源码真实行为写出来的地方

这部分文档不能只写“理想语义”，要写当前实现。

两个点尤其重要：

1. `Storage` 只存字符串，`INCR` 这种命令依然是先把字符串 parse 成数字，再算完再存回字符串。
2. `setx(...)` 永远把第三个参数当成“相对 TTL”。

第二点会直接影响 RDB 恢复行为：

- `parse_expire_entry(...)` 从 snapshot 里解析出来的是绝对过期时间
- `parse_rdb(...)` 当前会把这个值直接传给 `storage.setx(...)`

因此，RDB 恢复出来的过期时间并不会严格保留原始绝对截止时间，而是会被再次当成相对 TTL，加上一遍当前时间。

这不是文档吹毛求疵，而是当前源码的真实表现。

## 字符串命令的数据流

普通字符串命令的路径是：

```text
Cmd::run
-> command helper in cmd.rs
-> lock server.storage
-> Storage method
-> Protocol response
```

stream 命令走的也是类似模式，只是锁的是 `server.streams`。

## 扩展边界

如果后面要支持更多 Redis 类型，当前结构上的阻力点很明确：

- `Storage` 只知道字符串
- stream 被单独建模在 `Storage` 之外
- `TYPE` 只会探测这两个容器
- 命令 helper 直接知道要锁哪个容器

所以如果要加 list、set、hash，大概率不是改一个局部 helper 就够，而是要重新整理顶层状态结构。

## 当前实现限制

- 普通 value 全都是字符串
- 过期清理是惰性的
- `keys()` 不会顺手清理过期数据
- stream 和字符串存储是完全分开的
- `setx(...)` 只天然适配相对 TTL 语义
- 这一层没有内存统计、淘汰策略或持久化 hook

存储模型本身很克制，但正因为它足够明确，命令层和 RDB 层的行为才会比较容易顺着看下去。
