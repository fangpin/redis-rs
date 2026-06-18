---
title: RESP Protocol
layout: default
nav_order: 4
permalink: /docs/resp-protocol/
---

# RESP Protocol

This chapter isolates the wire-format adapter that sits between sockets, command parsing, and response encoding.

## File boundary

- `src/protocol.rs`

## Why this module is central

The rest of the server assumes that a parsed `Protocol` value exists.

Important consumers are:

- `Cmd::from(...)`, which turns a top-level RESP array into a typed command
- `Protocol::encode(...)`, which turns command results back into bytes
- replication helpers, which construct small RESP messages through `from_vec(...)` and `ok()`

So `src/protocol.rs` is both the inbound parser and the outbound serializer.

## Internal protocol model

The enum is intentionally small:

- `SimpleString(String)`
- `BulkString(String)`
- `Null`
- `Array(Vec<Protocol>)`

There is no dedicated `Error`, `Integer`, or binary-safe blob variant.

That design choice keeps the implementation short, but it has visible consequences later:

- error replies are encoded as simple strings
- integer-like values are usually rendered as strings
- payload fidelity is limited by the string-centric representation

## Parsing contract

`Protocol::from(protocol: &str)` is the top-level parser.

Its contract is:

- inspect the first byte
- dispatch to a suffix parser
- return `(Protocol, consumed_len)`

The consumed length is not incidental. It is what lets array parsing recurse through a single input buffer without reparsing from the beginning.

Dispatch currently goes to:

- `parse_simple_string_sfx(...)`
- `parse_bulk_string_sfx(...)`
- `parse_array_sfx(...)`

Unsupported prefixes fail immediately.

## `parse_simple_string_sfx(...)`

This parser is the smallest in the file.

It:

1. searches for the first `\r\n`
2. takes the bytes before it
3. returns `SimpleString(...)`
4. reports the consumed length including the delimiter

There is no extra validation beyond delimiter discovery.

## `parse_bulk_string_sfx(...)`

Bulk strings use a two-stage parse.

Stage 1:

- read the decimal length prefix before the first `\r\n`

Stage 2:

- read the following payload up to the next `\r\n`
- verify that the actual string length matches the declared length

If the lengths do not match, parsing fails instead of attempting recovery.

## Important current behavior: payload lowercasing

When a bulk string is accepted, the parser stores it as:

```rust
Protocol::BulkString(s.to_lowercase())
```

That is convenient for command matching because `Cmd::from(...)` can assume lowercase command tokens.

But it also changes semantics:

- command names become case-insensitive
- keys are lowercased
- values are lowercased
- replicated external command payloads are not byte-exact copies of the original wire input

For example, `SET Foo Bar` enters the command layer as lowercase tokens.

This is a real implementation shortcut, not just a presentation detail.

## `parse_array_sfx(...)`

Array parsing is where the `(Protocol, consumed_len)` contract pays off.

The control flow is:

1. parse the array length prefix
2. move an `offset` cursor past the header
3. call `Protocol::from(&s[offset..])` for each child
4. advance `offset` by the child parser's consumed length
5. collect all children into `Protocol::Array`

The function is structurally recursive, but it still relies on the outer input being available as one contiguous `&str`.

## Construction helpers used by other modules

The file also exposes a few helper constructors:

- `from_vec(...)`
- `ok()`
- `err(...)`
- `write_on_slave_err()`
- `psync_on_slave_err()`
- `none()`

These helpers are important because higher layers often need to build RESP replies without repeating manual array or string assembly.

Two of them are especially revealing:

- `from_vec(...)` constructs RESP arrays out of bulk strings and does not lowercase inputs
- `err(...)` returns `SimpleString`, not a dedicated RESP error type

So internal helper-built messages and externally parsed messages do not have exactly the same semantics.

## `decode()`

`decode()` flattens a `Protocol` value into a plain string.

Mapping:

- `SimpleString` -> inner string
- `BulkString` -> inner string
- `Null` -> `""`
- `Array` -> child strings joined by spaces

This is heavily used by `Cmd::from(...)` to turn a parsed array into command tokens.

That is practical, but it is not a lossless structural view. Nested arrays become flattened space-joined text.

## `encode()`

`encode()` performs the reverse mapping back to RESP text.

Current rules are:

- `SimpleString` -> `+...\r\n`
- `BulkString` -> `$len\r\npayload\r\n`
- `Array` -> `*len\r\n` plus encoded children
- `Null` -> `$-1\r\n`

This single serializer is reused for:

- ordinary client replies
- replication handshake replies
- command propagation to replicas

## End-to-end data flow

For a normal command, the wire-format path is:

```text
socket text
-> Protocol::from
-> Cmd::from
-> command handler result as Protocol
-> Protocol::encode
-> socket write
```

For internally generated replication messages, the path is usually:

```text
helper constructor such as Protocol::from_vec
-> Protocol::encode
-> socket write
```

That difference matters because helper-generated data bypasses the lowercase-on-parse behavior.

## Error surface

Parsing errors return `DBError`.

Semantic command errors are usually represented later as `Protocol::err("...")`, which still encodes to a RESP simple string.

So there are really two layers of failure:

- parser/build failures as Rust errors
- command/runtime failures as string replies

The module does not model the full RESP error vocabulary separately.

## Current implementation limits

- parser input is `&str`, not raw bytes
- the parser assumes a full frame is already available in memory
- bulk strings are lowercased on parse
- `decode()` flattens arrays into command-friendly text instead of preserving structure
- error replies use `SimpleString`
- the enum does not model integers or binary-safe blobs separately

The protocol layer is deliberately small, but it is still the hinge that explains several later behaviors that would otherwise look surprising in command execution and replication.
