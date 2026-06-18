---
title: Storage Model
layout: default
nav_order: 5
permalink: /docs/storage-model/
---

# Storage Model

This chapter covers the in-memory key-value storage layer and the stream container shape that sits beside it.

## File boundaries

- `src/storage.rs`
- `src/server.rs`

## String key-value storage

`src/storage.rs` defines the core storage type used for normal Redis string commands.

The internal representation is:

```text
HashMap<String, (String, Option<u128>)>
```

Each entry stores:

- the string value
- an optional absolute expiration timestamp in milliseconds

The type alias `ValueType` makes that tuple explicit in the code.

## Why expiration is stored as an absolute timestamp

The storage layer does not keep:

- insertion time
- relative TTL duration

Instead, it stores an absolute expiration timestamp. That decision keeps reads simple:

- compare `now_in_millis()` with stored timestamp
- remove expired key if needed
- otherwise return the value

This matches the needs of both direct `SET PX/EX` writes and RDB restore.

## `now_in_millis()`

`now_in_millis()` is the time source for expiry behavior. It converts `SystemTime` into Unix milliseconds.

That helper is reused in two places:

- storage expiry checks
- auto-generated stream IDs in `cmd.rs`

So the project already treats millisecond time as a shared primitive across multiple subsystems.

## Read path

`Storage::get(...)` is more than a plain lookup.

The control flow is:

1. look up key in `HashMap`
2. if key has an expiration timestamp, compare it with current time
3. if expired, delete the key and return `None`
4. otherwise clone and return the stored string

This is lazy expiration. Expired keys disappear when accessed, not through a background eviction loop.

## Write paths

The write surface is intentionally narrow:

- `set(...)` -> store value without expiry
- `setx(...)` -> store value with expiry
- `del(...)` -> remove key
- `keys(...)` -> list current keys

`setx(...)` converts the provided relative TTL into an absolute timestamp by adding `now_in_millis()`.

That means callers do not need to share a common time conversion policy. They only need to provide a duration in milliseconds.

## What is not stored here

Streams are not part of `Storage`.

Instead, `Server` keeps a second container:

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

That separation matters because:

- string keys have optional expiry and simple point lookups
- stream keys need ordered range queries by entry ID

Using `BTreeMap` for streams gives ordered traversal without complicating the ordinary string-key storage model.

## How command handlers interact with storage

Command helpers in `cmd.rs` typically access storage like this:

1. lock `server.storage`
2. call one storage method
3. release lock

Because `Storage` itself is synchronous and small, the async boundary lives outside it in the server's `Mutex`.

## Data flow for string commands

For commands such as `GET`, `SET`, `DEL`, and `INCR`, the data path is:

```text
command helper in cmd.rs
-> lock server.storage
-> call Storage method
-> build Protocol response
```

This is one reason the repo stays readable: command helpers remain thin adapters over a tiny storage core.

## Current implementation limits

- all values are stored as strings
- expiration cleanup is lazy, not proactive
- there is no size accounting or eviction policy
- `keys()` does not filter expired entries unless they were previously touched by `get()`

Those tradeoffs are reasonable for a teaching implementation and make the storage layer easy to inspect.
