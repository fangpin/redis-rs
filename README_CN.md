# 使用 Rust 构建你自己的 Redis

该项目旨在构建一个玩具 Redis-Server，它能够解析 Redis 协议并处理基本的 Redis 命令; 将 Redis 持久化化到 RDB 文件中；从 RDB 文件解析和初始化 Redis，
支持主从复制、Redis 流。

您可以在 [我的 github repo](https://github.com/fangpin/redis-rs) 中找到所有源代码和提交历史记录。

## 主要功能
+ 解析 Redis 协议
+ 处理基本的 Redis 命令
+ 从 RDB 文件解析和初始化 Redis
+ 主从复制
+ Redis stream

## 先决条件
首先安装 `redis-cli`（用于测试目的的 redis 客户端实现）
```sh
cargo install mini-redis
```

学习:
- [Redis protocoal](https://redis.io/docs/latest/develop/reference/protocol-spec)
- [RDB file format](https://rdb.fnordig.de/file_format.html)
- [Redis replication](https://redis.io/docs/management/replication/)

## 启动 Redis-rs 服务器
```sh
# 以主机启动
cargo run -- --dir /some/db/path --dbfilename dump.rdb
# 以从机启动
cargo run -- --dir /some/db/path --dbfilename dump.rdb --port 6380 --replicaof "localhost 6379"
```


## 支持的命令
```sh
redis-cli PING
redis-cli ECHO hey
redis-cli SET foo bar
redis-cli SET foo bar px/ex 100
redis-cli GET foo

redis-cli CONFIG GET dbfilename
redis-cli KEYS "*"

redis-cli INFO replication

redis-cli TYPE some_key

# streams
redis-cli XADD stream_key 1526919030474-0 temperature 36 humidity 95
redis-cli XADD stream_key 1526919030474-* temperature 37 humidity 94
redis-cli XADD stream_key "*" foo bar
## read strea
redis-cli XRANGE stream_key 0-2 0-3
```

## RDB 持久性
获取 Redis-rs 服务器配置
```sh
redis-cli CONFIG GET dbfilename
```

### RDB 文件格式概述
以下是 [RDB 文件](https://rdb.fnordig.de/file_format.html) 的各个部分，顺序如下：
+ 标头部分
+ 元数据部分
+ 数据库部分
+ 文件结尾部分

#### 标头部分
以某个魔法数字开头
```sh
52 45 44 49 53 30 30 31 31 // 魔法字符串 + 版本号 (ASCII)：“REDIS0011”。
```

#### 元数据部分
包含零个或多个“元数据子部分”，每个子部分指定一个元数据属性
例如
```sh
FA // 表示元数据子部分的开始。
09 72 65 64 69 73 2D 76 65 72 // 元数据属性的名称（字符串编码）：“redis-ver”。
06 36 2E 30 2E 31 36 // 元数据属性的值（字符串编码）：“6.0.16”。
```

#### 数据库部分
包含零个或多个“数据库子部分”，每个子部分描述一个数据库。
例如
```sh
FE // 表示数据库子部分的开始。
00 /* 数据库的索引（大小编码）。此处，索引为 0。 */

FB // 表示后面是哈希表大小信息。
03 /* 存储键和值的哈希表的大小（大小编码）。此处，总键值哈希表大小为 3。 */
02 /* 存储键的到期时间的哈希表的大小（大小编码）。此处，具有到期时间的键数为 2。 */
```

```sh
00 /* 指定值的类型和编码的 1 字节标志。此处，标志为 0，表示“字符串”。 */
06 66 6F 6F 62 61 72 // 键的名称（字符串编码）。此处为“foobar”。
06 62 61 7A 71 75 78 // 值（字符串编码）。此处为“bazqux”。
```

```sh
FC /* 表示此键（“foo”）具有到期时间，并且到期时间戳以毫秒为单位。 */
15 72 E7 07 8F 01 00 00 /* 过期时间戳，以 Unix 时间表示，存储为 8 字节无符号长整型，采用小端序（从右到左读取）。此处，过期时间戳为 1713824559637。 */
00 // 值类型为字符串。
03 66 6F 6F // 键名为“foo”。
03 62 61 72 // 值为“bar”。
```

```sh
FD /* 表示此键（“baz”）有过期时间，过期时间戳以秒为单位。 */
52 ED 2A 66 /* 过期时间戳，以 Unix 时间表示，存储为 4 字节无符号整数，采用小端序（从右到左读取）。此处，过期时间戳为 1714089298。 */
00 // 值类型为字符串。
03 62 61 7A // 键名为“baz”。
03 71 75 78 // 值为“qux”。
```

总之，
- 可选过期信息（下列之一）：
- 以秒为单位的时间戳：
- FD
- 以秒为单位的过期时间戳（4 字节无符号整数）
- 以毫秒为单位的时间戳：
- FC
- 以毫秒为单位的过期时间戳（8 字节无符号长整型）
- 值类型（1 字节标志）
- 键（字符串编码）
- 值（编码取决于值类型）

#### 文件结束部分
```sh
FF /* 表示文件即将结束，随后是校验和。 */
89 3b b7 4e f8 0f 77 19 // 整个文件的 8 字节 CRC64 校验和。
```

#### 大小编码
```sh
/* 如果前两位为 0b00：
大小为字节的剩余 6 位。
在此示例中，大小为 10：*/
0A
00001010

/* 如果前两位为 0b01：
大小为接下来的 14 位
（第一个字节的剩余 6 位，与下一个字节合并），
大端字节序（从左到右读取）。
在此示例中，大小为 700：*/
42 BC
01000010 10111100

/* 如果前两位为 0b10：
忽略第一个字节的剩余 6 位。
大小是接下来的 4 个字节，采用大端字节序（从左到右读取）。
在此示例中，大小为 17000：*/
80 00 00 42 68
10000000 00000000 00000000 01000010 01101000

/* 如果前两位为 0b11：
其余 6 位指定字符串编码类型。
请参阅字符串编码部分。*/
```

#### 字符串编码
+ 字符串的大小（编码的大小）。
+ 字符串。
```sh
/* 0x0D 大小指定字符串长度为 13 个字符。其余字符拼出“Hello, World!”。 */
0D 48 65 6C 6C 6F 2C 20 57 6F 72 6C 64 21
```

对于以 0b11 开头的大小，其余 6 位表示字符串格式的类型：
```sh
/* 0xC0 大小表示字符串是 8 位整数。在此示例中，字符串为“123”。 */
C0 7B

/* 0xC1 大小表示字符串是 16 位整数。其余字节采用小端序（从右到左读取）。在此示例中，字符串为“12345”。 */
C1 39 30

/* 0xC2 大小表示字符串是 32 位整数。其余字节采用小端序（从右到左读取），在此示例中，字符串为“1234567”。 */
C2 87 D6 12 00

/* 0xC3 尺寸指示
C3 ...
```

## 复制
Redis 服务器 [leader-follower 复制](https://redis.io/docs/management/replication/)。
运行多个 Redis 服务器，其中一个充当“主服务器”，其他充当“副本服务器”。对主服务器所做的更改将自动复制到副本服务器。

### 发送握手（follower -> master）
1. 当跟随者启动时，它将以 RESP 数组的形式向主服务器发送 PING 命令。
2. 然后从跟随者向主服务器发送 2 个 REPLCONF（复制配置）命令，以传达端口和同步协议。一个是 *REPLCONF listening-port <PORT>*，另一个是 *REPLCONF capa psync2*。psnync2 是本项目支持的一个示例同步协议。
3. 跟随者向主服务器发送 *PSYNC* 命令，其中包含复制 ID 和偏移量，以启动复制过程。

### 接收握手（主服务器 -> 从服务器）
1. 向从服务器响应 PONG 消息。
2. 向从服务器响应两个 REPLCONF 命令的 OK 消息。
3. 向从服务器响应 *+FULLRESYNC <REPL_ID> 0\r\n*，其中包含复制 ID 和偏移量。

### RDB 文件传输
从服务器启动时，它会发送 *PSYNC ? -1* 命令来告诉主服务器它还没有任何数据，需要完全重新同步。

然后，主服务器向从服务器发送 *FULLRESYNC* 响应作为确认。

最后，主服务器向从服务器发送 RDB 文件以表示其当前状态。从服务器应将收到的 RDB 文件加载到内存中，替换其当前状态。

### 接收写入命令（主服务器 -> 从服务器）
主服务器向从服务器发送以下写入命令，其中包含偏移量信息。
发送是为了重用握手和 RDB 文件传输的相同 TCP 连接。
由于所有命令都像普通客户端命令一样被编码为 RESP 数组，因此跟随者可以重用相同的逻辑来处理来自主服务器的复制命令。唯一的区别是命令来自主服务器，不需要回复。

## 流
流由一个键标识，它包含多个条目。
每个条目由一个或多个键值对组成，并分配一个唯一 ID。
[有关 redis 流的更多信息](https://redis.io/docs/latest/develop/data-types/streams/)
[Radix 树](https://en.wikipedia.org/wiki/Radix_tree)

它看起来像一个键值对列表。
```sh
entries:
- id: 1526985054069-0 #（第一个条目的 ID）
temperature: 36 #（第一个条目中的键值对）
humness: 95 #（第一个条目中的另一个键值对）

- id: 1526985054079-0 #（第二个条目的 ID）
temperature: 37 #（第一个条目中的键值对）
humness: 94 #（第一个条目中的另一个键值对）
# ...（等等）
```

Redis 流用例的示例包括：
- 事件源（例如，跟踪用户操作、点击等）
- 传感器监控（例如，从现场设备读取）
- 通知（例如，将每个用户的通知记录存储在单独的流中）


-- 未完待续