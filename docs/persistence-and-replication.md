---
title: Persistence and Replication
layout: default
nav_exclude: true
permalink: /docs/persistence-and-replication/
---

# Persistence and Replication

This chapter covers how the project restores state from RDB files and how master/slave synchronization is wired.

## File boundaries

- `src/rdb.rs`
- `src/replication_client.rs`
- `src/server.rs`

## RDB loading path

The master startup path in `Server::init(...)` opens the configured DB file and only parses it when the file is non-empty.

The load chain is:

1. `Server::init(...)`
2. `rdb::parse_rdb_file(...)`
3. `rdb::parse_rdb(...)`

The parser writes decoded values directly into `server.storage`.

## What the RDB parser supports

`src/rdb.rs` implements a focused subset of the Redis RDB format:

- magic header validation
- version read
- auxiliary metadata sections
- database selection markers
- table size info
- string keys with and without expiration
- EOF marker and CRC read

The code recognizes two expiration encodings:

- `0xFC` for millisecond timestamps
- `0xFD` for second timestamps

When expiration exists, the parser converts the loaded timestamp into the `Storage::setx(...)` path.

## String decoding model

The parser supports these string encodings:

- raw string
- 8-bit integer
- 16-bit integer
- 32-bit integer

LZF-compressed strings are explicitly not supported yet. That is surfaced as an error rather than silently ignored.

## Storage interaction

`src/storage.rs` is intentionally small:

- `set(...)` stores plain values
- `setx(...)` stores values with an absolute expiration timestamp
- `get(...)` lazily deletes expired keys on read

This is a simple model, but it keeps the persistence path easy to understand: the RDB loader only has to decide whether an entry has an expiry and then call the corresponding storage method.

## Slave handshake flow

`src/replication_client.rs` implements the follower-side startup sequence:

1. `PING`
2. `REPLCONF listening-port <port>`
3. `REPLCONF capa psync2`
4. `PSYNC ? -1`

The implementation checks each expected response explicitly through `check_resp(...)`.

## Full sync behavior

After `PSYNC`, the follower reads:

1. the `FULLRESYNC` line
2. the `$<len>` RDB payload header
3. the RDB payload itself

The received snapshot is then parsed by reusing `rdb::parse_rdb(...)`.

That reuse is important: there is one RDB decoding path for local file restore and replica bootstrap.

## Master-side replication behavior

When the master receives `PSYNC` inside `Server::handle(...)`:

1. it sends the RDB payload through `MasterReplicationClient::send_rdb_file(...)`
2. it stores the replica socket through `add_stream(...)`
3. it stops treating that connection as a normal request/response client

`MasterReplicationClient` then becomes the fan-out point for replicated write commands using `send_command(...)`.

## Current implementation limits

- the master sends a constant empty RDB seed payload before downstream command replay
- RDB CRC is read but not validated
- auxiliary metadata is parsed and ignored
- replication state tracking is intentionally narrow compared with production Redis

Those choices keep the project focused on the essential control flow: load state, establish role relationship, and start replaying write traffic.
