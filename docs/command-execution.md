---
title: Command Execution
layout: default
nav_order: 8
permalink: /docs/command-execution/
---

# Command Execution

This chapter covers how parsed RESP values become typed commands and how those commands dispatch into concrete behavior.

## File boundaries

- `src/cmd.rs`
- `src/server.rs`
- `src/protocol.rs`

## Why `cmd.rs` is the semantic center

`src/cmd.rs` is where several ownership boundaries meet:

- parsed protocol arrays become typed commands
- command handlers are dispatched
- write commands intersect with replication
- stream commands share the normal command path
- transactions queue and replay commands

If `server.rs` is the runtime shell, `cmd.rs` is the module that gives the server meaning.

## `Cmd` enum

The `Cmd` enum is the typed summary of the supported command surface.

It includes:

- basic commands: `Ping`, `Echo`
- string commands: `Get`, `Set`, `SetPx`, `SetEx`, `Del`, `Incr`
- introspection commands: `Keys`, `ConfigGet`, `Info`, `Type`
- replication commands: `Replconf`, `Psync`
- stream commands: `Xadd`, `Xrange`, `Xread`
- transaction commands: `Multi`, `Exec`, `Discard`
- fallback: `Unknow`

The misspelling in `Unknow` is part of the current source and is reflected in later fallback behavior.

## `Cmd::from(...)`

`Cmd::from(...)` receives raw RESP text, not a pre-parsed protocol object.

Its control flow is:

1. call `Protocol::from(s)`
2. require the top-level value to be `Protocol::Array`
3. call `decode()` on every child to build a flat token vector
4. match on `cmd[0]`
5. validate argument count and shape for each supported command
6. return `(Cmd, original_protocol)`

Returning both values is an important design choice:

- `Cmd` drives semantic dispatch
- `Protocol` is reused later for replication fan-out and offset accounting

## Why lowercasing in the protocol layer matters here

Because `Protocol::parse_bulk_string_sfx(...)` lowercases bulk-string payloads, `Cmd::from(...)` effectively operates on lowercase tokens.

That simplifies pattern matching:

- `"set"` instead of handling mixed case
- `"px"` and `"ex"` are easy to compare

But it also means data values reach command handlers lowercased too.

That is a protocol-layer shortcut with command-layer consequences.

## Parsing by command family

`Cmd::from(...)` uses a narrow, explicit parser for each supported command family.

Examples:

- `set` distinguishes plain, `px`, and `ex` forms by argument count and token position
- `config` only supports `config get <name>`
- `keys` only supports `keys *`
- `xadd` collects field/value pairs starting at index `3`
- `xread` optionally parses `block <millis>` before splitting stream keys and offsets

Unsupported argument shapes return `DBError` immediately instead of being deferred to the handler.

## `Cmd::run(...)`

`Cmd::run(...)` is the central dispatcher.

Its inputs are:

- `&mut Server`
- original `Protocol`
- `is_rep_con`
- `queued_cmd`

Its output is `Result<Protocol, DBError>`.

The dispatch table sends each variant to a focused helper such as:

- `get_cmd(...)`
- `set_cmd(...)`
- `config_get_cmd(...)`
- `info_cmd(...)`
- `replconf_cmd(...)`
- `xadd_cmd(...)`
- `xread_cmd(...)`
- `exec_cmd(...)`

That is the module's main structural win. Parsing and per-command behavior stay separated.

## Transaction-aware queueing in `run(...)`

Before dispatching, `run(...)` checks whether the current connection already has a transaction queue.

If `queued_cmd` is present and the incoming command is not `EXEC`, `MULTI`, or `DISCARD`, the function:

1. pushes `(self.clone(), protocol.clone())` into the queue
2. returns `QUEUED`

So transaction queueing happens before normal handler execution.

This is why queued commands do not mutate storage immediately.

## String command helpers

The basic string helpers are thin wrappers over `Storage`.

Examples:

- `get_cmd(...)` -> `storage.get(...)`
- `set_cmd(...)` -> `storage.set(...)`
- `set_px_cmd(...)` -> `storage.setx(..., px_millis)`
- `set_ex_cmd(...)` -> `storage.setx(..., seconds * 1000)`
- `del_cmd(...)` -> `storage.del(...)`

The command layer owns argument interpretation and reply shape. The storage layer only owns point operations.

## Introspection helpers

`config_get_cmd(...)` reads directly from `server.option`.

Only two names are supported:

- `dir`
- `dbfilename`

`info_cmd(...)` only supports the `replication` section and renders a small text blob out of `server.option.replication`.

That means `INFO replication` is configuration-oriented, not a fully live runtime report.

## `type_cmd(...)`

`type_cmd(...)` makes the split state model visible to clients.

Its control flow is:

1. lock string storage and try `get(...)`
2. if found, return `string`
3. otherwise lock streams and probe `server.streams`
4. if found, return `stream`
5. otherwise return `none`

This command is a small but useful map of the repo's two-container design.

## `replconf_cmd(...)` and `psync_cmd(...)`

`replconf_cmd(...)` is intentionally shallow.

- `getack` -> build `REPLCONF ACK <offset>` from `server.offset`
- everything else -> `OK`

So `REPLCONF listening-port ...` and `REPLCONF capa psync2` are accepted without argument-level validation in the command layer.

`psync_cmd(...)` is also narrow:

- master -> `FULLRESYNC <master_replid> 0`
- slave -> `PSYNC ON SLAVE IS NOT ALLOWED`

The socket-mode transition is handled later in `Server::handle(...)`, not here.

## Shared write policy: `resp_and_replicate(...)`

Several write helpers end by calling `resp_and_replicate(...)`.

That helper centralizes role policy:

- on a master, send the original command to downstream replicas and return the local response
- on a slave receiving a normal client request, reject the write
- on a slave replaying data from the master, accept the write

This removes duplicated role checks from `SET`, `DEL`, and `XADD`.

## A notable exception: `INCR`

`incr_cmd(...)` reads, parses, increments, and writes a string value locally.

Unlike `SET`, `DEL`, or `XADD`, it does **not** end in `resp_and_replicate(...)`.

So in the current implementation:

- `INCR` mutates local storage
- `INCR` is not propagated to downstream replicas
- `INCR` is not rejected on a slave normal-client connection through the shared write guard

That is a current behavior gap worth documenting directly.

## Offset accounting

After any successful command, `Cmd::run(...)` increments `server.offset` by `p.encode().len()`.

But some write helpers also manually increment the same counter by `1` inside the helper body.

That means offset tracking is not a single consistent accounting rule today. Some writes effectively advance the counter twice for one logical command.

## Unknown commands

Unsupported top-level command names become `Cmd::Unknow`.

Later, `run(...)` maps that branch to `Protocol::err("unknow cmd")`.

So the system distinguishes:

- malformed supported commands -> `DBError`
- unknown command names -> fallback reply

That split is slightly uneven, but it matches the current code.

## Current implementation limits

- command parsing assumes one fully decoded top-level RESP array
- payload lowercasing happens before command parsing
- some helpers rely on narrow argument-shape assumptions
- `INCR` bypasses the shared replication/write-guard helper
- offset accounting is inconsistent across helpers
- unknown commands collapse into one generic reply shape

`cmd.rs` is still the best single file to read after `server.rs`, because it exposes where the repo chooses directness over abstraction.
