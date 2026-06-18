---
title: Server Runtime
layout: default
nav_order: 3
permalink: /docs/server-runtime/
---

# Server Runtime

This chapter follows the executable entrypoint and the shared server object that every connection task clones.

## File boundaries

- `src/main.rs`
- `src/server.rs`
- `src/options.rs`

## What starts the process

`src/main.rs` is the only binary entrypoint. It parses four command-line arguments:

- `--dir`
- `--dbfilename`
- `--port`
- `--replicaof`

Those inputs are assembled into `DBOption` and `ReplicationOption`, then passed into `Server::new(...)`.

This means process role is selected before the server exists:

- no `--replicaof` -> master
- with `--replicaof` -> slave

## Configuration model

`src/options.rs` keeps runtime configuration deliberately small.

`DBOption` carries:

- DB directory
- DB filename
- TCP port
- replication settings

`ReplicationOption` carries:

- `role`
- `master_replid`
- `master_repl_offset`
- `replica_of`

The server never discovers its role dynamically. It trusts the CLI-derived option object.

## Listener and task model

After parsing arguments, `main.rs` binds one Tokio `TcpListener` on `127.0.0.1:<port>`.

For each accepted connection, it:

1. clones `Server`
2. spawns a Tokio task
3. calls `Server::handle(...)`

This gives the project a simple concurrency model:

- one logical shared server state
- one async task per socket
- no central command scheduler

## What `Server` stores

`Server` in `src/server.rs` is the shared runtime shell. The important fields are:

- `storage: Arc<Mutex<Storage>>`
- `streams: Arc<Mutex<HashMap<String, Stream>>>`
- `option: DBOption`
- `offset: Arc<AtomicU64>`
- `master_repl_clients: Arc<Mutex<Option<MasterReplicationClient>>>`
- `stream_reader_blocker: Arc<Mutex<Vec<Sender<()>>>>`
- `master_addr: Option<String>`

The separation between `storage` and `streams` is meaningful: string keys and stream keys do not contend on the same mutex.

## Startup path in `Server::new(...)`

`Server::new(...)` does three high-level things:

1. derives `master_addr` if the node is configured as a slave
2. allocates shared state containers
3. calls `init().await`

Master-only replication fan-out state is also allocated here. If the node is a slave, `master_repl_clients` is `None`.

## Master-side initialization

`Server::init(...)` currently performs only master-side local persistence bootstrap.

The control flow is:

1. construct DB file path from `dir` and `db_file_name`
2. create the file if it does not exist
3. check file length
4. if non-empty, parse it through `rdb::parse_rdb_file(...)`

Slave nodes skip this branch because their initial state is expected to come from the upstream master during replication bootstrap.

## Slave-side startup path

Slave-specific startup is not hidden inside `Server::new(...)`; it remains visible in `main.rs`.

The sequence is:

1. create a follower replication client with `get_follower_repl_client(...)`
2. `PING` master
3. report listening port
4. report sync capability
5. start `PSYNC`
6. spawn a dedicated handler task for the replication socket

This keeps the top-level runtime easy to trace from the binary entrypoint.

## The connection loop

`Server::handle(...)` is the per-socket runtime loop.

For each iteration it:

1. reads into a fixed `512` byte buffer
2. converts bytes into UTF-8 text
3. parses one command with `Cmd::from(...)`
4. executes it through `cmd.run(...)`
5. writes a response unless the socket is a replication connection

If the node is master and the command is `PSYNC`, it switches behavior:

1. send RDB payload
2. register the socket as a replica downstream
3. stop normal request/response handling for that connection

That branch is the runtime hinge between ordinary clients and replication clients.

## Data flow through a normal request

For a normal client connection the data path is:

```text
socket bytes
-> Server::handle
-> Cmd::from
-> Cmd::run
-> Protocol response
-> socket write
```

The server object itself stays mostly orchestration-oriented. It does not implement command semantics directly.

## Current implementation limits

- one fixed-size read buffer per connection
- one parsed command per successful read
- no incremental framing across partial reads
- no explicit shutdown or task supervision structure

Those tradeoffs keep the runtime surface compact enough to read in one pass.
