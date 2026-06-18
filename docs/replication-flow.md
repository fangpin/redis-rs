---
title: Replication Flow
layout: default
nav_order: 7
permalink: /docs/replication-flow/
---

# Replication Flow

This chapter follows how a follower connects to a master, receives the initial snapshot, and then starts consuming replicated commands.

## File boundaries

- `src/main.rs`
- `src/replication_client.rs`
- `src/server.rs`
- `src/cmd.rs`

## Replication is startup-driven

There is no separate replication service object coordinating everything in the background.

Follower replication begins explicitly in `main.rs` right after the `Server` is created.

The startup sequence is:

```text
Server::new(...)
-> get_follower_repl_client(...)
-> ping_master()
-> report_port(...)
-> report_sync_protocol()
-> start_psync(...)
-> spawn Server::handle(replication_stream, true)
```

This makes the full bootstrap visible at the binary entrypoint.

## Follower-side client object

`src/replication_client.rs` defines `FollowerReplicationClient`.

It owns one field:

- `stream: TcpStream`

That single socket is reused for:

- handshake commands
- snapshot transfer
- later command replay from the master

So bootstrap and steady-state replication intentionally share one TCP connection.

## Handshake steps

The follower issues four messages in order:

1. `PING`
2. `REPLCONF listening-port <port>`
3. `REPLCONF capa psync2`
4. `PSYNC ? -1`

`ping_master(...)`, `report_port(...)`, and `report_sync_protocol(...)` each:

- build a RESP array
- write it to the socket
- call `check_resp(...)`

`check_resp(...)` performs an exact byte comparison against the expected simple-string reply.

That is simple and readable, but it assumes the full expected reply arrives in one read.

## `PSYNC` on the follower

`start_psync(...)` writes `PSYNC ? -1` and then immediately calls `recv_rdb_file(...)`.

So from the follower's point of view, `PSYNC` means:

1. ask for full synchronization
2. parse the master's `FULLRESYNC` line
3. consume the following RDB payload
4. leave the socket positioned for later live command replay

## `recv_rdb_file(...)`

`recv_rdb_file(...)` wraps the socket in `BufReader` and reads:

1. one line ending in `\r\n` for replication metadata
2. one `$<len>\r\n` header for the RDB payload length
3. the RDB bytes themselves by delegating to `rdb::parse_rdb(...)`

The first line is expected to contain three tokens, such as:

```text
FULLRESYNC <replid> <offset>
```

One detail is worth documenting exactly: the parsed `rdb_file_len` is logged, but not used to bound the snapshot read. The code relies on `rdb::parse_rdb(...)` reaching `EOF` inside the stream.

## Master-side `PSYNC`

On the master, `PSYNC` is handled across two modules.

`cmd.rs` handles the command semantics:

- `Cmd::from(...)` recognizes `psync`
- `psync_cmd(...)` returns `FULLRESYNC <master_replid> 0` on a master

`server.rs` handles the socket-state transition:

1. `cmd.run(...)` produces the `FULLRESYNC` reply
2. `Server::handle(...)` writes that reply as part of normal request/response flow
3. `handle(...)` sees that the command was `Cmd::Psync`
4. `MasterReplicationClient::send_rdb_file(...)` writes the snapshot payload
5. `MasterReplicationClient::add_stream(...)` registers the socket for future fan-out
6. `handle(...)` breaks out of the normal loop

That split is important. `PSYNC` is not just another command handler because the socket has to change role afterward.

## Snapshot payload source

`MasterReplicationClient::send_rdb_file(...)` does not serialize the live in-memory state.

Instead it decodes a hard-coded hex string named `EMPTY_RDB_FILE_HEX_STRING` and writes that payload to the follower.

So the current full-sync behavior is:

- send a valid empty snapshot shell
- rely on later command replay for subsequent writes

This is one of the clearest fidelity gaps relative to production Redis.

## Downstream replica registration

`MasterReplicationClient` stores replica sockets in:

```text
Arc<Mutex<Vec<TcpStream>>>
```

`add_stream(...)` simply pushes the socket into that vector.

There is no separate replica metadata record for:

- replica ID
- health
- last acknowledged offset
- backpressure state

The master only remembers writable sockets.

## Command fan-out

Once a replica is registered, write propagation happens through `send_command(...)`.

The control flow is:

1. lock the vector of replica streams
2. iterate over each socket
3. write `protocol.encode()` to every socket

There is no per-replica retry or drop-on-failure policy beyond surfacing a write error.

## Where replication intersects command execution

Most write-like commands call `resp_and_replicate(...)` in `cmd.rs`.

That helper applies the role rules:

- master -> broadcast to downstream replicas, then return the local response
- slave on ordinary client connection -> reject the write
- slave on replication connection -> allow the replayed write

This is the semantic hinge where replication policy meets command semantics.

## Offset tracking

The repo currently has two different offset notions.

Live shared counter:

- `server.offset`
- updated inside `Cmd::run(...)`
- also incremented manually inside some write helpers
- used by `REPLCONF GETACK`

Static configuration field:

- `server.option.replication.master_repl_offset`
- initialized in `main.rs`
- returned by `INFO replication`

So `GETACK` and `INFO replication` do not actually report the same live source of truth.

## Current implementation limits

- full sync always sends a constant empty RDB payload
- the follower does not enforce the advertised RDB payload length
- there is no partial resync
- there is no backlog window
- there is no reconnect loop after startup
- replica sockets are stored without health or metadata
- offset reporting is internally inconsistent

Even so, the replication path still shows the essential educational shape: explicit handshake, full snapshot bootstrap, socket registration, then downstream command replay.
