---
title: Storage Model
layout: default
nav_order: 5
permalink: /docs/storage-model/
---

# Storage Model

This chapter covers the in-memory state containers behind string commands and stream commands.

## File boundaries

- `src/storage.rs`
- `src/server.rs`
- `src/cmd.rs`

## Two storage families

The runtime does not keep every Redis data type in one universal value enum.

Instead it splits state into two separate containers:

- string keys in `Storage`
- streams in `Server::streams`

That split is one of the main structural shortcuts in the repo. It avoids a generic object system and keeps the code readable by command family.

## String-key storage shape

`src/storage.rs` defines:

```text
HashMap<String, (String, Option<u128>)>
```

The tuple means:

- stored string value
- optional expiration timestamp

The alias `ValueType` makes that explicit in code.

## `Storage` API surface

The public methods are small:

- `new()`
- `get(...)`
- `set(...)`
- `setx(...)`
- `del(...)`
- `keys(...)`

This API is intentionally narrower than Redis itself. Complex behavior stays in `cmd.rs`, while `Storage` mostly performs point lookups and writes.

## Time source

`now_in_millis()` converts `SystemTime` into Unix milliseconds.

That helper is reused across subsystems:

- expiry handling in `Storage`
- auto-generated stream IDs in `cmd.rs`

So millisecond time is already a shared runtime primitive.

## Expiration model

`Storage` stores expiration as one `Option<u128>` per key.

The important detail is what `setx(...)` expects:

```text
relative duration in milliseconds
```

`setx(...)` always computes:

```text
now_in_millis() + expire_ms
```

This is the right fit for:

- `SET PX <millis>`
- `SET EX <seconds>` after command-layer conversion to milliseconds

## `get(...)` and lazy expiry

`Storage::get(...)` is the main read path.

The control flow is:

1. look up the key in the `HashMap`
2. check whether an expiration exists
3. compare the stored timestamp with `now_in_millis()`
4. if expired, remove the key and return `None`
5. otherwise clone and return the stored value

There is no background cleanup task. Expired keys disappear when they are touched.

## Write paths

`set(...)` stores a string with no expiry.

`setx(...)` stores a string with an absolute deadline derived from a relative TTL.

`del(...)` removes a key directly.

`keys(...)` returns the current map keys without performing an expiry sweep first.

That last point matters: untouched expired keys can still appear in `KEYS *` until some later `GET` removes them.

## Stream container shape

Streams do not live inside `Storage`.

The type aliases in `src/server.rs` expand to:

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

Read it as:

- stream name -> ordered entry map
- entry ID -> field/value pairs

`BTreeMap` is the key design choice here. It gives ordered range traversal for `XRANGE` and `XREAD` without needing a separate secondary index.

## How command handlers reach storage

Most command helpers in `src/cmd.rs` follow the same pattern:

```text
lock shared container
-> call one small storage operation
-> build Protocol response
```

Examples:

- `GET` -> `storage.get(...)`
- `SET` -> `storage.set(...)`
- `SET PX/EX` -> `storage.setx(...)`
- `DEL` -> `storage.del(...)`
- `TYPE` -> check `storage`, then check `streams`

This is why the storage layer stays simple: orchestration and role checks live above it.

## Current behavior worth documenting exactly

The docs need to reflect the current implementation, including shortcuts.

Two details matter:

1. `Storage` only stores strings. Numeric commands such as `INCR` still round-trip through string parsing.
2. `setx(...)` always treats its third argument as a relative TTL.

That second point creates a visible interaction with the RDB parser:

- `parse_expire_entry(...)` decodes an absolute expiration timestamp from the snapshot
- `parse_rdb(...)` currently forwards that value into `storage.setx(...)`

So snapshot restore does not preserve absolute expiry exactly. The parsed timestamp is treated as a relative duration and shifted by the current wall clock again.

That is not a docs nit. It is how the code behaves today.

## Data flow for string commands

The normal string-command path is:

```text
Cmd::run
-> command helper in cmd.rs
-> lock server.storage
-> Storage method
-> Protocol response
```

The stream-command path is similar but targets `server.streams` instead.

## Extension boundaries

If someone wanted to add more Redis types, the main pressure points are obvious:

- `Storage` only knows string payloads
- streams are modeled outside `Storage`
- `TYPE` is hard-coded to probe exactly these two containers
- command helpers directly know which container to lock

So adding lists, sets, or hashes would likely require a new top-level state structure rather than a small local patch.

## Current implementation limits

- all normal values are strings
- expiry cleanup is lazy
- `keys()` does not sweep expired entries
- stream data is completely separate from string storage
- `setx(...)` only supports relative TTL semantics cleanly
- there is no memory accounting, eviction policy, or persistence hook at this layer

The storage model is intentionally modest, but understanding these exact choices makes the command layer much easier to read.
