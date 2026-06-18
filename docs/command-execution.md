---
title: Command Execution
layout: default
nav_order: 8
permalink: /docs/command-execution/
---

# Command Execution

This chapter focuses on how parsed protocol values become typed commands and how those commands dispatch into concrete behaviors.

## File boundaries

- `src/cmd.rs`
- `src/server.rs`
- `src/protocol.rs`

## Why `cmd.rs` is the center of behavior

`src/cmd.rs` is the largest behavior module in the repo. It is where:

- protocol arrays become command enums
- command handlers are dispatched
- write commands intersect with replication
- streams and transactions attach themselves to the normal command path

If `server.rs` is the runtime shell, `cmd.rs` is the semantic heart of the server.

## Command enum

`Cmd` is a typed summary of the supported command surface.

It currently includes:

- string commands such as `GET`, `SET`, `DEL`, `INCR`
- inspection commands such as `CONFIG GET`, `INFO`, `TYPE`
- replication commands such as `REPLCONF`, `PSYNC`
- stream commands such as `XADD`, `XRANGE`, `XREAD`
- transaction commands such as `MULTI`, `EXEC`, `DISCARD`

Unknown inputs fall into `Unknow`.

## Parsing path

`Cmd::from(...)` starts from a parsed `Protocol` and expects the top-level shape to be an array.

The control flow is:

1. call `Protocol::from(...)`
2. ensure the top-level value is `Protocol::Array`
3. decode each child into a flat token vector
4. match on the first token
5. validate argument count and shape for each supported command
6. return `(Cmd, Protocol)`

Returning the original `Protocol` together with the typed command is important because replication later needs the exact command payload again.

## Dispatch path

`Cmd::run(...)` is the central dispatcher.

Its inputs are:

- `&mut Server`
- original `Protocol`
- `is_rep_con`
- `queued_cmd`

Its output is always `Result<Protocol, DBError>`.

The dispatch table sends each command to a focused helper such as:

- `get_cmd(...)`
- `set_cmd(...)`
- `keys_cmd(...)`
- `info_cmd(...)`
- `xadd_cmd(...)`
- `exec_cmd(...)`

That keeps parsing, orchestration, and individual command semantics separate.

## Shared write helper

Several write commands end in `resp_and_replicate(...)`.

That helper decides:

- what local response should be returned
- whether the command should be forwarded to registered replicas
- whether a slave should reject the write

This is a useful design choice because replication rules are not duplicated across every write handler.

## Introspection commands

`config_get_cmd(...)` and `info_cmd(...)` expose parts of the runtime configuration and replication state.

These commands do not touch storage directly. They serialize fields from `server.option` into `Protocol` values.

That gives the repo a lightweight self-description surface without adding separate metadata layers.

## Keys and type inspection

`keys_cmd(...)` simply returns the current keys from string storage.

`type_cmd(...)` checks both containers:

1. string storage
2. stream map

and returns `string`, `stream`, or `none`.

That small helper is one of the places where the repo's split between normal storage and streams becomes visible to clients.

## Incr path

`incr_cmd(...)` reads the current string value, defaults missing keys to `1`, parses the value as `u64`, increments it, then stores the result back as a string.

If parsing fails, it returns an explicit Redis-style error message.

This is a compact example of how command helpers sit above the string-only storage model rather than replacing it.

## Offset accounting

After a successful command, `Cmd::run(...)` increments `server.offset` by the encoded length of the original protocol payload.

That means offset accounting is attached to command completion, not socket read position.

It is a crude model, but it is consistent across the command layer.

## Current implementation limits

- command parsing assumes fully decoded arrays
- unsupported shapes error out early
- many handlers still unwrap or assume well-formed values
- unknown commands collapse into one generic branch

Even with those limits, `cmd.rs` does a good job showing the command-oriented structure of a Redis-like server.
