---
title: RDB Parser
layout: default
nav_order: 6
permalink: /docs/rdb-parser/
---

# RDB Parser

This chapter isolates the snapshot parser used for local restore and replica bootstrap.

## File boundary

- `src/rdb.rs`

## Role of the module

`src/rdb.rs` turns an RDB byte stream into writes against in-memory state.

It is reused in two places:

- master startup restore from a local DB file
- follower bootstrap after receiving the master's snapshot

That reuse is important. The repo has one RDB decode path, not separate file and replication implementations.

## Entry functions

There are two public entrypoints:

- `parse_rdb_file(...)`
- `parse_rdb(...)`

`parse_rdb_file(...)` is the file-oriented wrapper.

Its job is only:

1. wrap a `tokio::fs::File` in `BufReader`
2. delegate to `parse_rdb(...)`

`parse_rdb(...)` is the real parser. Because it accepts `AsyncRead + Unpin`, the same function can consume a file or a network stream.

## High-level parse sequence

`parse_rdb(...)` locks `server.storage` once at the beginning, then walks the stream in this order:

1. `parse_magic(...)`
2. `parse_version(...)`
3. repeated opcode dispatch loop
4. stop on `EOF`

The opcode loop handles:

- `META` (`0xFA`)
- `DB_SELECT` (`0xFE`)
- `TABLE_SIZE_INFO` (`0xFB`)
- `EOF` (`0xFF`)

Anything else is treated as an error.

## Header validation

`parse_magic(...)` reads exactly five bytes and expects `REDIS`.

`parse_version(...)` reads the next four bytes and returns them unchanged.

The parser validates the outer file structure here, but it does not branch on version-specific behavior later.

## Metadata handling

When the loop sees `META`, it calls `parse_aux(...)` twice and discards both values.

That means the parser is structurally aware of auxiliary sections, but semantically ignores them.

This is a good example of the repo's style:

- parse enough to stay aligned with the real file format
- skip building state the rest of the server does not use

## Database selection and table sizing

`DB_SELECT` is parsed but ignored because the server effectively behaves like a single logical DB.

`TABLE_SIZE_INFO` is more than decoration. It drives the next two loops:

1. read `size_no_expire`
2. read `size_expire`
3. parse that many non-expiring entries
4. parse that many expiring entries

So this implementation assumes the snapshot is laid out in the expected grouped order.

## Entry readers

The per-entry helpers are:

- `parse_no_expire_entry(...)`
- `parse_expire_entry(...)`

`parse_no_expire_entry(...)` expects the next byte to be type `0`, then reads key and value through `parse_aux(...)`.

That means the parser currently supports only string values.

`parse_expire_entry(...)` first reads an expiration opcode, then delegates back to `parse_no_expire_entry(...)`.

Supported expiration encodings are:

- `0xFC` -> 8-byte little-endian milliseconds
- `0xFD` -> 4-byte little-endian seconds

Second-based values are converted to milliseconds immediately.

## Length decoding

`parse_len(...)` is the low-level helper behind strings and table sizes.

It returns:

- decoded length
- `StringEncoding`

Supported encodings are:

- `Raw`
- `I8`
- `I16`
- `I32`
- `LZF`

`parse_string(...)` can decode every variant except `LZF`, which returns an explicit error.

## A detail worth calling out: current prefix handling

The implementation is clearly trying to model Redis RDB length prefixes, but the current branch structure is narrower than the ideal format support.

In particular, the branch intended for 14-bit lengths matches on `0x04` rather than `0x40`.

So the parser documents the shape of the format, but it does not yet implement every prefix correctly.

That is best understood as a current limitation of this teaching implementation, not hidden as if full support existed.

## Data flow into storage

The parser streams entries directly into runtime state.

For ordinary entries:

```text
parse_no_expire_entry
-> storage.set(key, value)
```

For expiring entries:

```text
parse_expire_entry
-> storage.setx(key, value, expire_timestamp)
```

There is no intermediate snapshot object graph.

## Current restore caveat for expirations

This is one of the places where the docs need to describe current behavior exactly.

`parse_expire_entry(...)` returns an absolute expiration timestamp from the RDB payload.

But `storage.setx(...)` expects a relative TTL in milliseconds and adds `now_in_millis()` again.

So expiring keys restored from an RDB file do not preserve the original absolute deadline exactly in the current implementation. Their deadline is effectively shifted forward by the current time once more.

## Follower bootstrap reuse

During replication bootstrap, `FollowerReplicationClient::recv_rdb_file(...)` eventually calls `rdb::parse_rdb(&mut reader, server)`.

That means a follower applies the master's snapshot by reusing the same parser and the same storage-write behavior, including the expiration caveat above.

## EOF and CRC

When `EOF` is reached, the parser reads one trailing `u64` CRC field and ignores it.

So the byte stream stays aligned, but checksum validation is not implemented.

## Current implementation limits

- only string values are supported
- metadata is parsed then ignored
- DB selection is parsed then ignored
- LZF strings are rejected
- checksum is not validated
- the length-prefix implementation is partial
- expiring snapshot entries are restored through a relative-TTL helper

Even with those limits, `src/rdb.rs` is one of the most educational modules in the repo because it shows how local restore and replication bootstrap can share the same decoding pipeline.
