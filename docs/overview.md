---
title: Docs Overview
layout: default
nav_exclude: true
permalink: /docs/overview/
---

# Docs Overview

[Homepage]({{ '/' | relative_url }}) | [Server Runtime]({{ '/docs/server-runtime/' | relative_url }}) | [RESP Protocol]({{ '/docs/resp-protocol/' | relative_url }}) | [Storage Model]({{ '/docs/storage-model/' | relative_url }})

This docs area now maps `redis-rs` at module level instead of collapsing several ownership boundaries into three broad chapters.

## Reading path

Use this sequence if you want to understand the implementation from the outside in:

1. runtime entry and shared server state
2. RESP parsing and encoding
3. string storage and stream containers
4. RDB snapshot parsing
5. replication handshake and fan-out
6. command dispatch
7. streams and transaction replay

## Chapters

- [Server Runtime]({{ '/docs/server-runtime/' | relative_url }})
  - `src/main.rs`
  - `src/server.rs`
  - `src/options.rs`
- [RESP Protocol]({{ '/docs/resp-protocol/' | relative_url }})
  - `src/protocol.rs`
- [Storage Model]({{ '/docs/storage-model/' | relative_url }})
  - `src/storage.rs`
  - `src/server.rs`
- [RDB Parser]({{ '/docs/rdb-parser/' | relative_url }})
  - `src/rdb.rs`
- [Replication Flow]({{ '/docs/replication-flow/' | relative_url }})
  - `src/main.rs`
  - `src/replication_client.rs`
  - `src/server.rs`
  - `src/cmd.rs`
- [Command Execution]({{ '/docs/command-execution/' | relative_url }})
  - `src/cmd.rs`
  - `src/server.rs`
  - `src/protocol.rs`
- [Streams and Transactions]({{ '/docs/streams-and-transactions/' | relative_url }})
  - `src/cmd.rs`
  - `src/server.rs`

## Why this split

The repository is compact, but the code still has more than three real implementation boundaries:

- entry/runtime orchestration
- RESP parsing
- storage
- RDB decoding
- replication handshake and fan-out
- command dispatch
- stream and transaction extensions

Keeping those boundaries visible makes the docs much closer to the source tree and much better for code reading.
