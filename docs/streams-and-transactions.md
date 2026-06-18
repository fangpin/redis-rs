---
title: Streams and Transactions
layout: default
nav_order: 9
permalink: /docs/streams-and-transactions/
---

# Streams and Transactions

This chapter zooms in on the two higher-level behaviors that extend the basic string command set: streams and transactional queueing.

## File boundaries

- `src/cmd.rs`
- `src/server.rs`

## Stream storage shape

Streams live in `Server::streams`, not in `Storage`.

The type alias is:

```text
BTreeMap<String, Vec<(String, String)>>
```

and the full container is:

```text
HashMap<String, Stream>
```

That means:

- top-level key -> stream name
- ordered map key -> entry id
- value -> list of field/value pairs

The use of `BTreeMap` is what makes range queries possible without an extra ordering index.

## Stream ID parsing

`split_offset(...)` is the low-level helper behind stream IDs.

It extracts three pieces of information:

- millisecond timestamp part
- sequence part
- whether the ID used a wildcard sequence

This helper is reused by both writes and reads, so stream ordering logic stays in one place.

## `XADD`

`xadd_cmd(...)` handles stream writes.

Its control flow is:

1. normalize `*` into `now_in_millis()-*`
2. split the incoming ID
3. reject invalid `0-0`
4. compare against current stream tail if one exists
5. resolve wildcard sequence when needed
6. insert field/value pairs into the target stream entry
7. wake any blocked readers
8. replicate the original command if needed

This is the densest stream command because it owns both ID validation and append semantics.

## `XRANGE`

`xrange_cmd(...)` converts the special Redis range sentinels:

- `-` -> start from zero
- `+` -> end at max

It then performs a `BTreeMap::range(...)` query and serializes the results back into a flat RESP array.

So the read path is effectively:

```text
stream key
-> BTreeMap range
-> iterate ordered entries
-> encode as Protocol::Array
```

## `XREAD`

`xread_cmd(...)` supports:

- multiple streams
- per-stream starting offsets
- optional blocking mode

For blocking behavior there are two branches:

- positive `BLOCK <millis>` -> sleep for that duration
- `BLOCK 0` -> register a wake-up sender and wait for notification

The second branch uses `server.stream_reader_blocker` as a lightweight waiter registry.

## Reader wake-up model

After `XADD`, the code acquires `stream_reader_blocker`, sends one empty signal to each waiting reader, then clears the list.

This is intentionally simple:

- no per-stream wait queues
- no fairness logic
- one global wake-up list

It is good enough to demonstrate blocking reads without introducing a larger async coordination subsystem.

## Transaction queue model

Transactions are represented by:

```text
Option<Vec<(Cmd, Protocol)>>
```

This queue is local to one connection inside `Server::handle(...)`.

The important consequence is that transaction state does not live in the global server object. It belongs to the current client session.

## `MULTI`, `EXEC`, `DISCARD`

The transaction control flow is:

- `MULTI` -> initialize empty queue
- ordinary commands while queue exists -> push `(Cmd, Protocol)` and return `QUEUED`
- `EXEC` -> replay queued commands through `cmd.run(...)`
- `DISCARD` -> drop queue

`exec_cmd(...)` is compact because it does not implement special transaction-only semantics. It just reuses the existing execution path with queueing disabled.

## Why transaction replay stores both `Cmd` and `Protocol`

The queue keeps both forms because they serve different purposes:

- `Cmd` is the parsed semantic form used for dispatch
- `Protocol` is still useful for replication and offset accounting

That pairing avoids reparsing commands during replay.

## Current implementation limits

- stream waiters are global rather than per stream key
- `BLOCK <millis>` uses sleep instead of wake-on-event-with-timeout
- transaction queueing is connection-local and non-persistent
- stream and transaction semantics are narrower than real Redis edge cases

Even so, these features are implemented in a way that remains easy to follow from source.
