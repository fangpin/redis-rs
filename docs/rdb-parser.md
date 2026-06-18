---
title: RDB Parser
layout: default
nav_order: 6
permalink: /docs/rdb-parser/
---

# RDB Parser

This chapter isolates the local snapshot parser from the rest of the persistence and replication story.

## File boundary

- `src/rdb.rs`

## Role of the module

`src/rdb.rs` does one job: turn an RDB byte stream into writes against `server.storage`.

It is reused in two different contexts:

- local startup restore from a DB file
- replica bootstrap after downloading a snapshot from the master

That reuse is one of the cleaner decisions in the repo. There is only one RDB decoding path.

## Entry functions

There are two entrypoints:

- `parse_rdb_file(...)`
- `parse_rdb(...)`

`parse_rdb_file(...)` is a convenience wrapper that creates a `BufReader` over a file.

`parse_rdb(...)` is the actual streaming parser and accepts any `AsyncRead + Unpin`, which is why replication can reuse it directly.

## Overall control flow

The parser starts by locking `server.storage`, then processes the stream in this order:

1. `parse_magic(...)`
2. `parse_version(...)`
3. repeated opcode dispatch loop
4. stop on `EOF`

Inside the opcode loop, it recognizes:

- `META`
- `DB_SELECT`
- `TABLE_SIZE_INFO`
- `EOF`

Anything else is treated as an error.

## Header validation

`parse_magic(...)` reads exactly five bytes and checks for `REDIS`.

`parse_version(...)` then reads the next four bytes as the RDB version payload.

The parser validates structure here, but does not interpret version-specific feature differences.

## Metadata sections

When it sees `META`, the parser calls `parse_aux(...)` twice and then discards the results.

So metadata is syntactically parsed but semantically ignored.

That is a deliberate simplification:

- keep the parser aligned with real RDB structure
- avoid building a metadata model the server does not yet use

## Database selection and table sizes

`DB_SELECT` is parsed and ignored because this project effectively uses one logical DB.

`TABLE_SIZE_INFO` is more operationally important. It provides:

- number of non-expiring entries
- number of expiring entries

The parser uses these counts to decide how many entries to read in each category.

## Entry parsing

There are two entry readers:

- `parse_no_expire_entry(...)`
- `parse_expire_entry(...)`

Both currently assume string value type `0`.

That means this parser is focused on string keys, not the full Redis type matrix.

## Expiration decoding

`parse_expire_entry(...)` supports two encodings:

- `0xFC` -> 8-byte little-endian milliseconds
- `0xFD` -> 4-byte little-endian seconds

Second-based expiration is converted to milliseconds before being written into storage.

This keeps the rest of the runtime on one time unit.

## Length and string decoding

`parse_len(...)` interprets Redis RDB length prefixes and returns:

- decoded length
- `StringEncoding`

Supported string encodings are:

- `Raw`
- `I8`
- `I16`
- `I32`
- `LZF`

`parse_string(...)` can decode every variant above except `LZF`, which returns an explicit error.

## Data flow into storage

Once entries are parsed, they are written immediately:

- non-expiring entry -> `storage.set(...)`
- expiring entry -> `storage.setx(...)`

The parser does not build an intermediate object graph. It streams values straight into runtime state.

## CRC handling

When `EOF` is reached, the parser reads an 8-byte CRC field and ignores it.

So the file structure is consumed correctly, but checksum validation is not implemented.

## Current implementation limits

- only a narrow subset of the RDB format is supported
- metadata is ignored
- DB index is ignored
- only string-type entries are handled
- LZF strings are rejected
- CRC is not validated

Even with those limits, the parser is detailed enough to explain the real shape of Redis persistence and to power both local restore and replica bootstrap.
