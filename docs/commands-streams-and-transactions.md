---
title: Commands, Streams, and Transactions
layout: default
nav_exclude: true
permalink: /docs/commands-streams-and-transactions/
---

# Commands, Streams, and Transactions

This chapter covers command decoding, the command execution surface, and the parts of the runtime that go beyond basic key-value storage.

## File boundaries

- `src/cmd.rs`
- `src/storage.rs`
- `src/server.rs`

## Command decoding

`Cmd::from(...)` converts a parsed RESP array into a typed command enum.

The command surface currently includes:

- `PING`
- `ECHO`
- `GET`
- `SET`
- `SET PX`
- `SET EX`
- `DEL`
- `KEYS *`
- `CONFIG GET`
- `INFO`
- `TYPE`
- `INCR`
- `REPLCONF`
- `PSYNC`
- `XADD`
- `XRANGE`
- `XREAD`
- `MULTI`
- `EXEC`
- `DISCARD`

This enum is the main command boundary for the whole server.

## Execution path

`Cmd::run(...)` is the dispatch hub. It receives:

- the mutable `Server`
- the original `Protocol`
- a flag telling whether the connection is a replication connection
- the current queued transaction state

The function routes each command into a smaller helper such as:

- `get_cmd(...)`
- `set_cmd(...)`
- `xadd_cmd(...)`
- `xread_cmd(...)`
- `exec_cmd(...)`

That keeps parsing and execution separate: `Cmd::from(...)` answers "what command is this?" while `run(...)` answers "what should it do?"

## Basic storage commands

For string keys, `cmd.rs` ultimately delegates to `Storage`:

- `GET` calls into `storage.get(...)`
- `SET` and timed `SET` paths call `set(...)` or `setx(...)`
- `DEL` removes keys
- `INCR` builds on the stored string representation and updates the same key

The key-value layer is intentionally string-based, which keeps both RDB loading and RESP round-tripping simple.

## Stream model

`Server` stores streams separately from ordinary keys:

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

Conceptually:

- top-level key -> stream name
- inner `BTreeMap` -> ordered stream entry ids
- entry payload -> field/value pairs

This design makes ordered range reads straightforward because `BTreeMap` preserves key ordering.

## Stream commands

The stream surface in `cmd.rs` includes:

- `XADD`
- `XRANGE`
- `XREAD`

Important responsibilities handled in the command layer:

- validating stream IDs
- generating IDs for `*`
- reading by range
- reading from one or many streams
- optional blocking behavior for `XREAD`

Blocking reads coordinate through `server.stream_reader_blocker`, which gives the implementation a simple wake-up path when new entries arrive.

## Transactions

The transaction implementation is centered on `queued_cmd: Option<Vec<(Cmd, Protocol)>>`.

Behavior:

- `MULTI` creates an empty queue
- ordinary commands after `MULTI` are appended and return `QUEUED`
- `EXEC` replays the queued commands in order
- `DISCARD` clears the queue

This is a compact design because it reuses:

- the parsed `Cmd`
- the original `Protocol`
- the existing `run(...)` path during replay

In other words, transaction execution does not need a second execution engine. It re-enters the same command dispatch path with queueing turned off.

## Replication interaction

After successful command execution, `Cmd::run(...)` updates `server.offset` using the encoded protocol length. That gives the server a simple running notion of how much write traffic has flowed through the replication stream.

## Current implementation limits

- command parsing assumes well-formed array shapes for each supported command
- unsupported commands collapse into `Unknow`
- transaction queueing is connection-local and intentionally simple
- stream and transaction behavior is implemented for learning clarity, not wire-compatibility depth

That tradeoff is consistent across the repo: code paths are small and readable, while still exposing enough of Redis to make persistence, replication, streams, and transactions feel connected.
