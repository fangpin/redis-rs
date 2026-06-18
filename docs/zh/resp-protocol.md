---
title: RESP 协议
layout: default
nav_order: 4
permalink: /zh/docs/resp-protocol/
---

# RESP 协议

这一章单独看负责 socket、命令解析和响应编码之间转换的 wire-format 适配层。

## 文件边界

- `src/protocol.rs`

## 为什么这个模块是中枢

后面的其他模块基本都默认“已经有一个解析好的 `Protocol` 对象了”。

最重要的使用方包括：

- `Cmd::from(...)`，把顶层 RESP array 变成 typed command
- `Protocol::encode(...)`，把命令执行结果重新编码成返回字节
- 复制辅助函数，它们会用 `from_vec(...)` 和 `ok()` 直接构造小的 RESP 消息

所以 `src/protocol.rs` 既是入站解析器，也是出站序列化器。

## 内部协议模型

当前枚举非常小：

- `SimpleString(String)`
- `BulkString(String)`
- `Null`
- `Array(Vec<Protocol>)`

没有单独的 `Error`、`Integer`，也没有二进制安全的 blob 变体。

这个设计让实现很短，但也会带来几个直接后果：

- 错误回复是按 simple string 编码的
- 看起来像整数的值一般也是按字符串处理
- 由于内部模型完全是字符串，payload 保真度有限

## 解析契约

`Protocol::from(protocol: &str)` 是顶层入口。

它的契约是：

- 看首字节
- 分发到不同 suffix parser
- 返回 `(Protocol, consumed_len)`

这里的 consumed length 不是附带信息，而是数组递归解析成立的关键。

当前分发到：

- `parse_simple_string_sfx(...)`
- `parse_bulk_string_sfx(...)`
- `parse_array_sfx(...)`

遇到不支持的前缀就立刻失败。

## `parse_simple_string_sfx(...)`

这是文件里最小的 parser。

它会：

1. 找第一个 `\r\n`
2. 取前面的内容
3. 返回 `SimpleString(...)`
4. 把分隔符也计入 consumed length

除了分隔符查找之外，没有做额外结构校验。

## `parse_bulk_string_sfx(...)`

bulk string 是两段式解析。

第一段：

- 先读出第一个 `\r\n` 之前的十进制长度

第二段：

- 读取后面的 payload，直到下一个 `\r\n`
- 校验实际字符串长度是否和声明长度一致

如果长度不匹配，会直接返回错误，不尝试恢复。

## 一个需要明确写出来的当前行为：payload 会被转成小写

bulk string 解析成功后，当前实现会存成：

```rust
Protocol::BulkString(s.to_lowercase())
```

这样做的好处是命令匹配很简单，因为 `Cmd::from(...)` 可以默认所有 token 都是小写。

但代价也很明显：

- 命令名会变成大小写不敏感
- key 会被转小写
- value 也会被转小写
- 从外部客户端读进来的命令，在复制链路里也不再是字节级原样转发

比如 `SET Foo Bar` 进入命令层时，已经是一组小写 token 了。

这不是文档措辞问题，而是当前实现里的真实 shortcut。

## `parse_array_sfx(...)`

数组解析是 `(Protocol, consumed_len)` 这个设计真正发挥作用的地方。

控制流是：

1. 解析数组长度前缀
2. 用 `offset` 游标跳过 header
3. 对剩余字符串调用 `Protocol::from(&s[offset..])`
4. 用子解析器返回的 consumed length 推进 `offset`
5. 收集成 `Protocol::Array`

它在结构上支持递归，但前提仍然是整个输入已经以一个完整 `&str` 的形式拿到了。

## 其他模块会用到的构造辅助函数

这个文件还暴露了一些 helper：

- `from_vec(...)`
- `ok()`
- `err(...)`
- `write_on_slave_err()`
- `psync_on_slave_err()`
- `none()`

这些 helper 很重要，因为上层经常需要构造 RESP 返回值，而不想每次都手写数组和字符串拼接。

其中两个尤其能看出语义：

- `from_vec(...)` 用 bulk string 构造数组，但不会把输入统一转小写
- `err(...)` 返回的是 `SimpleString`，不是单独的 RESP error 类型

所以“内部直接构造出来的消息”和“外部请求解析出来的消息”语义并不完全一致。

## `decode()`

`decode()` 会把 `Protocol` 压平成普通字符串。

规则是：

- `SimpleString` -> 内部字符串
- `BulkString` -> 内部字符串
- `Null` -> `""`
- `Array` -> 子元素字符串用空格拼接

`Cmd::from(...)` 很依赖这个方法，因为它要把 RESP array 变成命令 token 列表。

这很实用，但也意味着它不是一个无损的结构化视图。嵌套数组会被压平成空格连接的文本。

## `encode()`

`encode()` 做反方向的序列化。

当前映射规则：

- `SimpleString` -> `+...\r\n`
- `BulkString` -> `$len\r\npayload\r\n`
- `Array` -> `*len\r\n` 加上所有子元素编码
- `Null` -> `$-1\r\n`

同一个编码器会被复用在：

- 普通客户端响应
- 复制握手响应
- 向副本广播命令

## 端到端数据流

普通命令的 wire-format 路径是：

```text
socket text
-> Protocol::from
-> Cmd::from
-> command handler result as Protocol
-> Protocol::encode
-> socket write
```

而内部构造的复制消息通常是：

```text
helper constructor such as Protocol::from_vec
-> Protocol::encode
-> socket write
```

这个差异很重要，因为后者绕过了“解析时统一小写”这个行为。

## 错误面

解析错误会以 `DBError` 的形式向上返回。

而命令语义层的很多错误，后面会被表达成 `Protocol::err("...")`，它最终仍然编码成 simple string。

所以这里实际存在两层失败：

- parser / builder 失败 -> Rust error
- 命令 / 运行时失败 -> string reply

当前模块并没有把完整 RESP 错误语义单独建模出来。

## 当前实现限制

- parser 输入是 `&str`，不是原始字节流
- parser 假设完整 frame 已经一次性在内存里
- bulk string 在解析时会被转成小写
- `decode()` 会把数组压平成命令友好的字符串
- 错误回复用的是 `SimpleString`
- 枚举没有单独建模 integer 或 binary-safe blob

协议层虽然小，但如果理解了这里的取舍，后面命令执行和复制链路里很多看起来奇怪的行为就会顺很多。
