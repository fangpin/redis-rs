---
title: Streams and Transactions
layout: default
nav_order: 9
permalink: /docs/streams-and-transactions/
---

# Streams and Transactions

This chapter focuses on the two higher-level extensions layered on top of the basic string-command path: stream data and transaction queueing.

## File boundaries

- `src/cmd.rs`
- `src/server.rs`
- `src/storage.rs`

## Stream state shape

Streams live in `Server::streams`, not in `Storage`.

The full shape is:

```text
HashMap<String, BTreeMap<String, Vec<(String, String)>>>
```

Read it as:

- stream name -> stream object
- entry ID string -> ordered record
- record -> list of field/value pairs

The `BTreeMap` is the core design choice. It gives the implementation sorted entry IDs and efficient range traversal without another index.

## Stream IDs and `split_offset(...)`

`split_offset(...)` is the low-level helper behind stream ordering.

Given an ID like:

- `1526985054069-0`
- `1526985054069-*`
- `0-*`

it returns:

- timestamp part
- sequence part
- whether the original ID used a wildcard sequence

This helper is reused by both write and read logic, which keeps stream-ID interpretation in one place.

## `XADD`

`xadd_cmd(...)` is the densest stream helper in the file.

Its control flow is:

1. if the incoming ID is `*`, rewrite it to `now_in_millis()-*`
2. parse the ID with `split_offset(...)`
3. reject explicit `0-0`
4. lock `server.streams`
5. create the target stream if missing
6. compare the proposed ID against the stream tail
7. if the ID used a wildcard sequence and shares the same timestamp as the tail, bump the sequence
8. insert field/value pairs into the target entry
9. wake blocked readers
10. call `resp_and_replicate(...)`

The return value is the final inserted entry ID as a bulk string.

## Ordering rule enforcement

The stream tail comparison is one of the key behaviors:

- lower timestamp than the tail -> reject
- same timestamp and explicit sequence not greater than the tail -> reject
- same timestamp and wildcard sequence -> auto-increment sequence

That is how the implementation preserves monotonic stream IDs without a separate sequence allocator object.

## `XRANGE`

`xrange_cmd(...)` is structurally much simpler.

It:

1. locks `server.streams`
2. resolves special sentinels
3. performs `BTreeMap::range(...)`
4. serializes the matching entries into `Protocol::Array`

Special bound handling:

- `-` -> `"0"`
- `+` -> `u64::MAX.to_string()`

The response shape is flattened as alternating:

```text
entry-id, field-value-array, entry-id, field-value-array, ...
```

## `XREAD`

`xread_cmd(...)` supports:

- multiple stream keys
- multiple starting offsets
- optional `BLOCK`

The control flow is:

1. parse optional block duration earlier in `Cmd::from(...)`
2. if `BLOCK <millis>` and `millis > 0`, sleep for that duration
3. if `BLOCK 0`, register a sender in `server.stream_reader_blocker` and wait on the receiver
4. lock `server.streams`
5. for each stream, compute the exclusive-next starting ID
6. perform `BTreeMap::range(...)`
7. serialize results into one flat RESP array

The starting ID is made exclusive by incrementing the parsed sequence before building the range lower bound.

## Current blocking model

The waiter registry is:

```text
Arc<Mutex<Vec<Sender<()>>>>
```

After `XADD`, the code:

1. locks that vector
2. sends one empty signal to every sender
3. clears the vector

This is a deliberately small coordination mechanism:

- one global waiter list
- no per-stream partitioning
- no fairness policy
- no explicit timeout cancellation path beyond normal control flow

## Important current behavior: `BLOCK 0`

The current `BLOCK 0` branch is intended to wait until some later `XADD` wakes the reader.

But the receive loop is:

```rust
while let Some(_) = receiver.recv().await {
    println!("get new xadd cmd, release block");
}
```

and it does not break after the first wake-up.

So the current implementation waits for channel closure rather than returning immediately after the first notification. That makes the infinite-block branch narrower in practice than the intended Redis behavior.

## Transaction queue shape

Transactions are not stored globally in `Server`.

Instead, each connection loop in `Server::handle(...)` owns:

```text
Option<Vec<(Cmd, Protocol)>>
```

This makes transaction state:

- connection-local
- in-memory only
- invisible to other clients

That is the right shape for this repo, but it is worth stating explicitly.

## `MULTI`, `EXEC`, and `DISCARD`

Transaction control is implemented directly in `Cmd::run(...)` plus `exec_cmd(...)`.

`MULTI`:

- replace `queued_cmd` with `Some(Vec::new())`
- return `ok`

Ordinary commands while a queue exists:

- push `(Cmd, Protocol)` into the queue
- return `QUEUED`

`EXEC`:

1. iterate over queued commands
2. call `cmd.run(server, protocol.clone(), is_rep_con, &mut None)` for each one
3. collect every response into `Protocol::Array`
4. clear the queue

`DISCARD`:

- if a queue exists, drop it and return `ok`
- otherwise return `ERR Discard without MULTI`

## Why the queue stores both `Cmd` and `Protocol`

The pair is intentional.

- `Cmd` is the already-parsed semantic form used for replay
- `Protocol` is still needed by handlers that replicate or account based on the original message

So `EXEC` can reuse the normal execution path without reparsing the raw command text.

## Interactions with replication

Queued commands are replayed through normal `cmd.run(...)`.

That means transaction replay inherits the same downstream behavior as ordinary execution:

- commands using `resp_and_replicate(...)` still replicate or reject by role
- commands with their own local-only behavior, such as the current `INCR`, keep that behavior inside `EXEC` too

This is a good example of how reusing one execution path keeps the implementation compact while also preserving current quirks.

## Current implementation limits

- stream waiters are global rather than per stream
- `BLOCK <millis>` sleeps first, then reads, instead of event-driven wait-with-timeout
- `BLOCK 0` does not break after the first wake-up
- stream replies are encoded in a flattened custom shape
- transaction state is connection-local and non-persistent
- transaction replay does not add separate atomic rollback semantics

Even with those limits, streams and transactions are implemented in a way that is easy to trace from one file, which is exactly why they make good chapter boundaries in the docs.
