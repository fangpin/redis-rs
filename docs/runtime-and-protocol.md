---
title: Runtime and Protocol
layout: default
nav_exclude: true
permalink: /docs/runtime-and-protocol/
---

# Runtime and Protocol

This chapter covers the runtime entrypoint, the TCP server loop, and the RESP parser that turns raw client bytes into executable commands.

## File boundaries

- `src/main.rs`
- `src/server.rs`
- `src/protocol.rs`
- `src/options.rs`

## Entry flow

`src/main.rs` is the only executable entrypoint. It parses four CLI inputs:

- `--dir`
- `--dbfilename`
- `--port`
- `--replicaof`

Those values are assembled into `DBOption` and `ReplicationOption` from `src/options.rs`, then passed into `Server::new(...)`.

The important consequence is that role selection is entirely CLI-driven. If `--replicaof` is present, the server starts in slave mode; otherwise it starts as master.

## Listener model

The server binds one Tokio `TcpListener` to `127.0.0.1:<port>`.

For each accepted socket:

1. clone the `Server`
2. spawn a Tokio task
3. hand the socket to `Server::handle(...)`

This means the implementation uses a shared logical server with per-connection async tasks rather than a single-threaded command loop.

## Shared server state

`src/server.rs` stores the shared runtime in `Server`:

- `storage`: key-value state protected by `Arc<Mutex<Storage>>`
- `streams`: stream data protected by `Arc<Mutex<HashMap<...>>>`
- `option`: runtime configuration
- `offset`: replication offset via `AtomicU64`
- `master_repl_clients`: replica downstream connections when running as master
- `stream_reader_blocker`: async blockers used by blocking stream reads

The storage and stream maps are protected independently, which keeps ordinary string operations and stream operations from sharing the same mutex.

## Connection handling

`Server::handle(...)` performs a read-decode-execute loop:

1. read bytes into a fixed buffer
2. decode the buffer as UTF-8
3. call `Cmd::from(...)`
4. execute `cmd.run(...)`
5. write a response unless the connection is a replication channel

The current implementation assumes a single decoded command per read buffer and uses a fixed `512` byte buffer. That keeps the code short, but it also means protocol framing is simpler than a production Redis implementation.

## RESP implementation

`src/protocol.rs` defines a small `Protocol` enum:

- `SimpleString`
- `BulkString`
- `Null`
- `Array`

The parser supports:

- simple strings starting with `+`
- bulk strings starting with `$`
- arrays starting with `*`

`Protocol::from(...)` returns both the parsed value and consumed byte length, which is then reused by array parsing.

## Important behavior in this parser

### Lowercasing bulk strings

`parse_bulk_string_sfx(...)` lowercases the decoded bulk string before storing it in `Protocol::BulkString`.

That means command tokens and bulk string payloads are normalized the same way. It simplifies command dispatch, but it also means this parser does not preserve original case for bulk string values.

### Encoding helpers

The module provides helpers used broadly across the server:

- `Protocol::ok()`
- `Protocol::err(...)`
- `Protocol::none()`
- `encode()`
- `decode()`

These helpers are the common boundary between command execution and socket I/O.

## Role-specific startup

If the server starts as slave:

1. it creates a `FollowerReplicationClient`
2. it performs the replication handshake
3. it loads the initial RDB state from the master
4. it spawns another handler task for the replication connection itself

That startup sequence is still driven from `main.rs`, not hidden inside `Server::new(...)`, so the high-level control flow is easy to trace from the executable entrypoint.

## Current implementation limits

- fixed socket read buffer size
- UTF-8 assumption over the full incoming chunk
- no incremental request framing across partial reads
- protocol parser is intentionally small and command-oriented rather than fully general

These limits fit the repo's educational purpose: the code stays readable while still exposing the essential Redis request lifecycle.
