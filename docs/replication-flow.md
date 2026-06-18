---
title: Replication Flow
layout: default
nav_order: 7
permalink: /docs/replication-flow/
---

# Replication Flow

This chapter focuses on how master and slave nodes establish a relationship and exchange state.

## File boundaries

- `src/main.rs`
- `src/replication_client.rs`
- `src/server.rs`
- `src/cmd.rs`

## Replica startup path

Slave startup begins in `main.rs`, not inside a hidden background service.

The explicit sequence is:

1. build `Server`
2. create `FollowerReplicationClient`
3. `ping_master()`
4. `report_port(...)`
5. `report_sync_protocol()`
6. `start_psync(...)`
7. spawn `Server::handle(...)` on the replication socket

That makes replication easy to trace because the orchestration is all near the binary entrypoint.

## Follower-side handshake

`FollowerReplicationClient` drives the upstream handshake over a single `TcpStream`.

The messages are:

1. `PING`
2. `REPLCONF listening-port <port>`
3. `REPLCONF capa psync2`
4. `PSYNC ? -1`

Each step writes a RESP array and then validates the master's response through `check_resp(...)`.

## `PSYNC` response handling

`start_psync(...)` sends the command and immediately delegates to `recv_rdb_file(...)`.

That method expects:

1. one line containing replication metadata such as `FULLRESYNC <id> <offset>`
2. a `$<len>` header for the snapshot payload
3. the raw RDB payload

The snapshot bytes are then parsed by reusing `rdb::parse_rdb(...)`.

## Master-side `PSYNC` handling

On the master, `PSYNC` is recognized in two places:

- `Cmd::from(...)` maps the command into `Cmd::Psync`
- `Server::handle(...)` special-cases that command after execution

The runtime branch in `Server::handle(...)` does the critical work:

1. call `send_rdb_file(...)`
2. call `add_stream(...)`
3. break out of the normal request loop

At that point the socket becomes a registered replica downstream.

## Why master behavior lives in `Server::handle(...)`

The code treats `PSYNC` as both:

- a logical command that returns `FULLRESYNC`
- a connection-mode transition

That second part cannot live entirely inside `cmd.rs`, because the server must keep the replica socket for future fan-out.

So the control flow is intentionally split:

- command semantics -> `psync_cmd(...)`
- socket ownership transition -> `Server::handle(...)`

## Snapshot payload source

`MasterReplicationClient::send_rdb_file(...)` currently sends a constant hex-encoded empty RDB image.

This is not a real serialization of live server state. It is a bootstrap seed that lets the protocol flow continue.

That is one of the biggest fidelity gaps relative to production Redis.

## Write propagation

Once replicas are registered, write propagation goes through `MasterReplicationClient::send_command(...)`.

The method:

1. locks the list of replica streams
2. iterates over each socket
3. writes the encoded protocol to every replica

This is a simple broadcast fan-out model with no per-replica backpressure logic.

## How command execution triggers replication

Write-like commands in `cmd.rs` eventually call `resp_and_replicate(...)`.

That function behaves differently by role:

- master -> send command to replicas, then return local response
- slave on normal client connection -> reject writes
- slave on replication connection -> accept replayed writes

This is the module boundary where replication rules intersect with command semantics.

## Offset tracking

The server tracks a coarse replication offset in `server.offset`.

Successful command execution increments it using the encoded protocol length. `REPLCONF GETACK` reads from that value to report progress.

This is not a complete replication state machine, but it is enough to model the basic flow of acknowledged progress.

## Current implementation limits

- snapshot source is a constant empty RDB payload
- no partial resync
- no backlog window
- no replica liveness tracking
- no retry or reconnect loop beyond startup path

Even so, the code shows the essential educational shape of leader-follower replication: handshake, full sync, socket registration, then downstream command replay.
