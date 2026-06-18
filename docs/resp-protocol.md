---
title: RESP Protocol
layout: default
nav_order: 4
permalink: /docs/resp-protocol/
---

# RESP Protocol

This chapter isolates the project's RESP model from the rest of the server logic.

## File boundary

- `src/protocol.rs`

## Protocol enum

`Protocol` is the internal representation used between parsing, command execution, and socket output.

The enum currently supports:

- `SimpleString(String)`
- `BulkString(String)`
- `Null`
- `Array(Vec<Protocol>)`

That is enough for the current command set and response surface.

## Why this module matters

Everything else in the project assumes a parsed `Protocol` value exists.

Two especially important boundaries depend on it:

- `Cmd::from(...)` consumes parsed arrays to decide which command was requested
- `Protocol::encode(...)` turns command results back into bytes for clients and replicas

So this module is both the inbound and outbound wire-format adapter.

## Parsing entrypoint

`Protocol::from(protocol: &str)` is the top-level parser.

It inspects the first byte and dispatches to one of three suffix parsers:

- `parse_simple_string_sfx(...)`
- `parse_bulk_string_sfx(...)`
- `parse_array_sfx(...)`

It returns a pair:

- parsed `Protocol`
- consumed byte length

That consumed length is what makes recursive array parsing possible.

## Simple strings

`parse_simple_string_sfx(...)` looks for the first `\\r\\n` pair and returns the bytes before it as a `SimpleString`.

This is the simplest parser in the file:

- no nested structure
- no declared payload length
- direct substring extraction

## Bulk strings

`parse_bulk_string_sfx(...)` has two stages:

1. parse the declared string length before the first `\\r\\n`
2. parse the following payload and verify its actual length matches the declared length

If lengths disagree, the parser returns an error instead of trying to recover.

## Important behavior: lowercasing payloads

When a bulk string is accepted, the implementation stores it as:

```rust
Protocol::BulkString(s.to_lowercase())
```

This has a practical benefit and a semantic cost.

Benefit:

- command matching becomes case-insensitive with no extra logic in `Cmd::from(...)`

Cost:

- bulk string payloads do not preserve original casing
- values are no longer byte-exact relative to the wire input

This is acceptable for the repo's learning focus, but it is a real difference from production Redis behavior.

## Arrays

`parse_array_sfx(...)` first reads the array length prefix, then repeatedly calls `Protocol::from(...)` on the remaining suffix.

Each child parse returns its own consumed length, and the parser advances an `offset` cursor until all array elements have been parsed.

The data flow is:

```text
array header
-> child 1 parse + length
-> child 2 parse + length
-> ...
-> Protocol::Array(vec)
```

This is the only place where the length-returning parse API really pays off.

## Construction helpers

The file also exposes helpers used by higher layers:

- `from_vec(...)`
- `ok()`
- `err(...)`
- `write_on_slave_err()`
- `psync_on_slave_err()`
- `none()`

These helpers keep command handlers from repeatedly rebuilding small RESP fragments by hand.

## Encoding path

`encode()` converts `Protocol` back into RESP text.

The current mapping is:

- `SimpleString` -> `+...\\r\\n`
- `BulkString` -> `$len\\r\\npayload\\r\\n`
- `Array` -> `*len\\r\\n` followed by encoded children
- `Null` -> `$-1\\r\\n`

This makes the same structure reusable for:

- client responses
- replication handshake messages
- write propagation to replicas

## Human-readable decoding

`decode()` flattens a `Protocol` value into a plain string.

This is used heavily by command parsing, especially when turning a parsed array into a vector of command tokens.

For arrays, `decode()` joins child values with spaces. That is a convenient command-oriented representation, even though it is not a lossless structural rendering.

## Current implementation limits

- parser input is a Rust `&str`, not raw bytes
- null bulk strings beyond the explicit `Null` encoding are not modeled separately
- nested arrays are supported structurally, but the command layer only expects a narrow subset
- bulk string lowercasing changes payload semantics

The module is intentionally small, but it is still the core wire-format hinge of the repo.
