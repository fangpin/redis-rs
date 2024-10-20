# Build Your Own Redis in Rust

This project is to build a toy Redis-Server clone that's capable of handling
basic commands like `PING`, `SET` and `GET`. Also implement the event loops, the Redis protocol and more.

## Prerequisites
install `redis-cli` first (an implementation of redis client for test purpose)
```sh
cargo install mini-redis
```

Know about [Redis protocoal](https://redis.io/docs/latest/develop/reference/protocol-spec)

## Start the Redis-rs server
```sh
cargo run -- --dir /tmp/redis-files --dbfilename dump.rdb
```


## Supported Commands
```sh
redis-cli PING
redis-cli ECHO hey
redis-cli SET foo bar
redis-cli SET foo bar px 100
redis-cli GET foo
redis-cli CONFIG GET dbfilename
redis-cli KEYS "*"
```

## RDB Persistence
Get Redis-rs server config
```sh
redis-cli CONFIG GET dbfilename
```
### RDB file format overview
Here are the different sections of the [RDB file](https://rdb.fnordig.de/file_format.html), in order:
+ Header section
+ Metadata section
+ Database section
+ End of file section
#### Header section
start with some magic number
```sh
52 45 44 49 53 30 30 31 31  // Magic string + version number (ASCII): "REDIS0011".
```
#### Metadata section
contains zero or more "metadata subsections", which each specify a single metadata attribute
e.g.
```sh
FA                             // Indicates the start of a metadata subsection.
09 72 65 64 69 73 2D 76 65 72  // The name of the metadata attribute (string encoded): "redis-ver".
06 36 2E 30 2E 31 36           // The value of the metadata attribute (string encoded): "6.0.16".
```
#### Database section
contains zero or more "database subsections," which each describe a single database.
e.g.
```sh
FE                       // Indicates the start of a database subsection.
00                       /* The index of the database (size encoded). Here, the index is 0. */

FB                       // Indicates that hash table size information follows.
03                       /* The size of the hash table that stores the keys and values (size encoded).  Here, the total key-value hash table size is 3. */
02                       /* The size of the hash table that stores the expires of the keys (size encoded).  Here, the number of keys with an expiry is 2. */
```

```sh
00                       /* The 1-byte flag that specifies the value’s type and encoding.  Here, the flag is 0, which means "string." */
06 66 6F 6F 62 61 72     // The name of the key (string encoded). Here, it's "foobar".
06 62 61 7A 71 75 78     // The value (string encoded). Here, it's "bazqux".
```

```sh
FC                       /* Indicates that this key ("foo") has an expire, and that the expire timestamp is expressed in milliseconds. */
15 72 E7 07 8F 01 00 00  /* The expire timestamp, expressed in Unix time, stored as an 8-byte unsigned long, in little-endian (read right-to-left).  Here, the expire timestamp is 1713824559637. */
00                       // Value type is string.
03 66 6F 6F              // Key name is "foo".
03 62 61 72              // Value is "bar".
```

```sh
FD                       /* Indicates that this key ("baz") has an expire, and that the expire timestamp is expressed in seconds. */
52 ED 2A 66              /* The expire timestamp, expressed in Unix time, stored as an 4-byte unsigned integer, in little-endian (read right-to-left).  Here, the expire timestamp is 1714089298. */
00                       // Value type is string.
03 62 61 7A              // Key name is "baz".
03 71 75 78              // Value is "qux".
```

In summary, 
- Optional expire information (one of the following):
  - Timestamp in seconds:
    - FD
    - Expire timestamp in seconds (4-byte unsigned integer)
  - Timestamp in milliseconds:
    - FC
    - Expire timestamp in milliseconds (8-byte unsigned long)
- Value type (1-byte flag)
- Key (string encoded)
- Value (encoding depends on value type)

#### End of file section
```sh
FF                       /* Indicates that the file is ending, and that the checksum follows. */
89 3b b7 4e f8 0f 77 19  // An 8-byte CRC64 checksum of the entire file.
```

#### Size encoding
```sh
/* If the first two bits are 0b00:
   The size is the remaining 6 bits of the byte.
   In this example, the size is 10: */
0A
00001010

/* If the first two bits are 0b01:
   The size is the next 14 bits
   (remaining 6 bits in the first byte, combined with the next byte),
   in big-endian (read left-to-right).
   In this example, the size is 700: */
42 BC
01000010 10111100

/* If the first two bits are 0b10:
   Ignore the remaining 6 bits of the first byte.
   The size is the next 4 bytes, in big-endian (read left-to-right).
   In this example, the size is 17000: */
80 00 00 42 68
10000000 00000000 00000000 01000010 01101000

/* If the first two bits are 0b11:
   The remaining 6 bits specify a type of string encoding.
   See string encoding section. */
```

#### String encoding
+ The size of the string (size encoded).
+ The string.
```sh
/* The 0x0D size specifies that the string is 13 characters long.  The remaining characters spell out "Hello, World!". */
0D 48 65 6C 6C 6F 2C 20 57 6F 72 6C 64 21
```
For sizes that begin with 0b11, the remaining 6 bits indicate a type of string format:
```sh
/* The 0xC0 size indicates the string is an 8-bit integer. In this example, the string is "123". */
C0 7B

/* The 0xC1 size indicates the string is a 16-bit integer. The remaining bytes are in little-endian (read right-to-left).  In this example, the string is "12345". */
C1 39 30

/* The 0xC2 size indicates the string is a 32-bit integer.  The remaining bytes are in little-endian (read right-to-left), In this example, the string is "1234567". */
C2 87 D6 12 00

/* The 0xC3 size indicates that the string is compressed with the LZF algorithm.  You will not encounter LZF-compressed strings in this challenge. */
C3 ...
```