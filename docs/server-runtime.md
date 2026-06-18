---
title: Server Runtime
layout: default
nav_order: 3
permalink: /docs/server-runtime/
---

# Server Runtime

This chapter follows the executable entrypoint, the shared `Server` object, and the per-connection loop that every socket eventually enters.

## File boundaries

- `src/main.rs`
- `src/server.rs`
- `src/options.rs`

## Runtime map

The runtime is intentionally compact. There is no separate acceptor service, scheduler, or replication supervisor.

The top-level flow is:

```text
CLI args
-> DBOption / ReplicationOption
-> Server::new(...)
-> Server::init(...)
-> optional follower replication bootstrap
-> TcpListener accept loop
-> one Tokio task per socket
-> Server::handle(...)
```

That shape matters because almost every subsystem in the repo is reached from this one path.

## Executable entrypoint

`src/main.rs` owns the only binary entrypoint.

It parses four CLI arguments:

- `--dir`
- `--dbfilename`
- `--port`
- `--replicaof`

Those values are assembled into `DBOption` and `ReplicationOption` before any server state exists.

The process role is therefore fixed at startup:

- no `--replicaof` -> master
- with `--replicaof` -> slave

There is no later promotion or role negotiation.

## Configuration objects

`src/options.rs` keeps configuration deliberately small.

`DBOption` carries:

- `dir`
- `db_file_name`
- `replication`
- `port`

`ReplicationOption` carries:

- `role`
- `master_replid`
- `master_repl_offset`
- `replica_of`

Two details are worth keeping in mind:

1. `master_replid` is hard-coded in `main.rs` instead of being generated dynamically.
2. `master_repl_offset` is part of configuration, not the live shared counter used by command execution.

That second split explains why `INFO replication` and `REPLCONF GETACK` do not report the same source of truth later in the code.

## What a `Server` clone actually shares

`Server` derives `Clone`, but it is not a deep independent copy.

The fields break down into two groups.

Shared across tasks through `Arc`:

- `storage: Arc<Mutex<Storage>>`
- `streams: Arc<Mutex<HashMap<String, Stream>>>`
- `offset: Arc<AtomicU64>`
- `master_repl_clients: Arc<Mutex<Option<MasterReplicationClient>>>`
- `stream_reader_blocker: Arc<Mutex<Vec<Sender<()>>>>`

Copied by value into each task-local `Server` clone:

- `option: DBOption`
- `master_addr: Option<String>`

So every accepted connection gets its own lightweight orchestration shell, but all meaningful mutable runtime state is shared.

## `Server::new(...)`

`Server::new(...)` in `src/server.rs` does three things:

1. derive `master_addr` from `replica_of` when the role is slave
2. allocate the shared containers
3. call `init().await`

The master-only fan-out client list is created only when the role is master:

- master -> `Some(MasterReplicationClient::new())`
- slave -> `None`

This is one of the clearer ownership decisions in the repo. Follower nodes never carry unused downstream-replica machinery.

## `Server::init(...)`

`init(...)` is the runtime bootstrap hook, but today it only performs master-side local persistence restore.

The control flow is:

1. check `self.is_master()`
2. build `dir/dbfilename`
3. open the file with create-if-missing semantics
4. inspect file length
5. if non-zero, call `rdb::parse_rdb_file(...)`

Slave nodes skip this whole branch. Their initial state is expected to come from the upstream master during replication bootstrap instead of local file restore.

That asymmetry is intentional and visible in code, not hidden behind one generic storage-init path.

## Follower bootstrap stays in `main.rs`

Slave startup is not buried inside `Server::new(...)`.

`main.rs` keeps the sequence explicit:

1. build `Server`
2. clone it into `sc`
3. call `get_follower_repl_client(...)`
4. `ping_master()`
5. `report_port(server.option.port)`
6. `report_sync_protocol()`
7. `start_psync(&mut sc)`
8. spawn `sc.handle(follower_repl_client.stream, true)`

This is good for code reading because the role-dependent startup path is still visible at the top level.

## `get_follower_repl_client(...)`

`Server::get_follower_repl_client(...)` is a small role gate.

- slave -> create `FollowerReplicationClient`
- master -> return `None`

It does not cache the upstream connection. It creates it on demand during startup and hands the socket back to `main.rs`.

## Listener and concurrency model

After startup, `main.rs` binds one Tokio `TcpListener` on `127.0.0.1:<port>`.

For every accepted socket it:

1. clones the shared `Server`
2. spawns one Tokio task
3. calls `Server::handle(stream, false)`

The concurrency model is therefore:

- one listener
- one async task per socket
- no central request queue
- shared mutable state guarded by `Mutex`

That keeps the runtime readable, but it also means lock boundaries directly shape behavior.

## The per-connection loop in `Server::handle(...)`

`Server::handle(...)` is where the ordinary client path and replication-socket path finally converge.

The loop currently works like this:

1. read into a fixed `512` byte buffer
2. stop if `len == 0`
3. interpret the bytes as UTF-8 with `str::from_utf8(...)`
4. parse one command with `Cmd::from(...)`
5. run it with `cmd.run(...)`
6. if this is not a replication connection, write the encoded response back
7. if the server is master and the command was `PSYNC`, switch this socket into downstream-replica mode

There is also a task-local transaction queue:

```text
queued_cmd: Option<Vec<(Cmd, Protocol)>>
```

That queue is created and consumed entirely inside the connection loop, which is why transactions are per-client-session rather than globally visible.

## Normal request data flow

For an ordinary client connection, the data path is:

```text
socket bytes
-> Server::handle
-> Cmd::from
-> Cmd::run
-> Protocol response
-> stream.write(...)
```

`Server` mostly orchestrates. It does not implement command semantics itself.

## Replication-socket mode switch

`PSYNC` is special because it is both:

- a command with a logical response
- a connection-state transition

The split is visible in `handle(...)`:

1. `cmd.run(...)` returns `FULLRESYNC ...`
2. because `is_rep_conn == false`, that response is written to the socket
3. `handle(...)` sees `Cmd::Psync`
4. `MasterReplicationClient::send_rdb_file(...)` writes the snapshot payload
5. `MasterReplicationClient::add_stream(...)` stores the socket for later fan-out
6. the normal request loop breaks

After that point, the socket stops being a normal request/response connection and becomes a registered replica downstream.

## Locking and ownership boundaries

The runtime uses a few coarse-grained locks:

- all string-key storage behind one `Mutex<Storage>`
- all streams behind one `Mutex<HashMap<...>>`
- all downstream replica sockets behind one `Mutex<Vec<TcpStream>>`
- all blocked stream readers behind one `Mutex<Vec<Sender<()>>>`

This keeps the code straightforward, but the granularity is intentionally broad. The design optimizes for readability over parallel fine-grained coordination.

## Current implementation limits

- `handle(...)` assumes a full command fits in one read and one `512` byte buffer
- there is no incremental framing across partial socket reads
- input is treated as UTF-8 text early, not as raw bytes all the way through
- there is no structured shutdown or task supervision tree
- follower bootstrap has no reconnect loop if the master later disappears
- connection mode is inferred from ad hoc branches rather than a dedicated state enum

The runtime is small enough to trace in one sitting, but the docs should be read as documentation of the current implementation, not of a fuller production Redis architecture.
