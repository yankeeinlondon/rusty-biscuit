---
prompt: |-
    The 'redb' crate in Rust provides a modern and highly performant transactional database which we plan to use in the "rendezvous" daemon.
    
    Your task is to do a deep dive on the 'redb' crate and answer the following questions through thorough research:
    
    - What is the functional footprint of the 'redb' crate?
    - What features does redb expose and what functionality do these features map to? When should you use each feature? When should you avoid?
    - What are the key URLs for this crate? Repo? Website? Docs?
    - What are 2-3 common use cases that this crate would be used for? For each use case, describe the use case and provide Rust code examples of how this use case might be implemented.
    - What do developers say about using this crate? What "gotchas" are there and how can they be worked around?
last_updated: 2026-05-24
---
# `redb` crate

## Functional Footprint

`redb` is a pure-Rust, embedded, transactional key-value store inspired by LMDB. It persists data to a single file using a collection of copy-on-write B+trees. The crate provides a type-safe, `BTreeMap`-like API where tables are defined at compile time with explicit key and value types. It is designed for single-process use with one concurrent writer and multiple non-blocking readers.

Core capabilities include:

- **ACID transactions** with configurable per-transaction durability
- **MVCC (Multi-Version Concurrency Control)** allowing readers to operate without blocking the writer, and vice versa
- **Zero-copy reads** where values can be accessed directly from memory-mapped pages without deserialization overhead
- **Savepoints and rollbacks** for sub-transactions or complex recovery logic
- **Multimap tables** for one-to-many key-to-value mappings
- **Online compaction** to reclaim space from deleted or updated entries
- **Crash safety** via checksums and a two-phase commit option

## Features

The `redb` crate exposes minimal Cargo features, keeping the surface area small:

| Feature         | Description                                        | When to Use                                                                            | When to Avoid                                                                                             |
|-----------------|----------------------------------------------------|----------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------|
| `logging`       | Enables internal log messages via the `log` crate. | Use when debugging redb internals or tracing commit/compaction behavior in production. | Avoid in performance-sensitive paths where log verbosity is unwanted; the feature is opt-in for a reason. |
| `cache_metrics` | Enables cache hit/miss statistics collection.      | Use when tuning page cache size or investigating read performance anomalies.           | Avoid when you do not need the overhead of metric collection.                                             |

Note: `redb` has no `std`-disabling feature; it requires the standard library. Several downstream crates (e.g., `shodh-redb`) extend redb with `no_std` support, but upstream `redb` itself does not.

## Key URLs

| Resource          | URL                                                                |
|-------------------|--------------------------------------------------------------------|
| **Repository**    | <https://github.com/cberner/redb> |
| **Website**       | <https://www.redb.org>                       |
| **Documentation** | <https://docs.rs/redb>                       |

## Common Use Cases

### 1. Local Application State Persistence

A desktop or CLI tool that needs durable, structured storage without running a separate database process. `redb` fits because it is a single file, requires no external runtime, and the typed API eliminates serialization boilerplate.

```rust
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings");

fn save_theme(path: &Path, theme: &str) -> anyhow::Result<()> {
    let db = Database::create(path)?;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(SETTINGS)?;
        table.insert("ui_theme", theme)?;
    }
    txn.commit()?;
    Ok(())
}

fn load_theme(path: &Path) -> anyhow::Result<Option<String>> {
    let db = Database::create(path)?; // opens existing if already present
    let txn = db.begin_read()?;
    let table = txn.open_table(SETTINGS)?;
    Ok(table.get("ui_theme")?.map(|g| g.value().to_string()))
}
```

### 2. High-Throughput Write Cache or Event Log

A service that batches events or acts as a fast ingestion buffer before uploading to a remote store. `redb` excels at single-writer, high-frequency inserts with its `Durability::None` option, which batches fsyncs for better throughput while still preserving atomicity and isolation.

```rust
use redb::{Database, Durability, ReadableTable, TableDefinition};
use std::path::Path;

const EVENTS: TableDefinition<u64, &str> = TableDefinition::new("events");

fn append_events(path: &Path, events: &[String]) -> anyhow::Result<u64> {
    let db = Database::create(path)?;
    let txn = db.begin_write()?;
    txn.set_durability(Durability::None); // fast, non-durable commit
    let mut next_id = 0u64;

    {
        let mut table = txn.open_table(EVENTS)?;
        // find the next id
        if let Some(last) = table.last()? {
            next_id = last.value().0 + 1;
        }
        for (i, ev) in events.iter().enumerate() {
            table.insert(&next_id, ev.as_str())?;
            next_id += 1;
        }
    }
    txn.commit()?;
    Ok(next_id)
}

fn read_events(path: &Path, start_id: u64) -> anyhow::Result<Vec<String>> {
    let db = Database::create(path)?;
    let txn = db.begin_read()?;
    let table = txn.open_table(EVENTS)?;
    let mut out = Vec::new();
    for result in table.range(start_id..)? {
        let (_, value) = result?;
        out.push(value.value().to_string());
    }
    Ok(out)
}
```

### 3. Multimap Index (One-to-Many Lookup)

Building indexes where a single key maps to multiple values—such as tagging systems or inverted indexes. `redb` provides a first-class `MultimapTable` that handles duplicate keys without manual serialization of collections.

```rust
use redb::{Database, ReadableMultimapTable, MultimapTableDefinition};
use std::path::Path;

const TAGS: MultimapTableDefinition<&str, &str> = MultimapTableDefinition::new("tags");

fn index_document(path: &Path, doc_id: &str, tags: &[&str]) -> anyhow::Result<()> {
    let db = Database::create(path)?;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_multimap_table(TAGS)?;
        for tag in tags {
            table.insert(*tag, doc_id)?;
        }
    }
    txn.commit()?;
    Ok(())
}

fn find_by_tag(path: &Path, tag: &str) -> anyhow::Result<Vec<String>> {
    let db = Database::create(path)?;
    let txn = db.begin_read()?;
    let table = txn.open_multimap_table(TAGS)?;
    let mut docs = Vec::new();
    for result in table.get(tag)? {
        let guard = result?;
        docs.push(guard.value().to_string());
    }
    Ok(docs)
}
```

## Developer Feedback and Gotchas

### What developers say

- **"LMDB-like but in safe Rust"** is the most common sentiment. Developers appreciate not needing unsafe FFI bindings to a C library while getting comparable semantics.
- **Strongly typed tables** are praised for eliminating an entire class of serialization bugs; the compile-time schema acts as documentation.
- **Single-writer performance** is a highlight; benchmarks often show `redb` beating LMDB and SQLite on individual writes.

### Known gotchas and workarounds

| Gotcha                                    | Explanation                                                                                                                                                                                                             | Workaround                                                                                                                                                                                                                   |
|-------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Single writer only**                    | `begin_write()` blocks if another write transaction is active. There is no concurrent multi-writer support.                                                                                                             | Architect your application around a single writer thread or a writer task queue. Batch writes to minimize lock contention.                                                                                                   |
| **File size growth**                      | Like other copy-on-write stores, the file can grow significantly under write-heavy workloads because old pages are not immediately reclaimed.                                                                           | Call `Database::compact()` periodically during maintenance windows. The database remains online during compaction.                                                                                                           |
| **Zero-copy lifetime complexity**         | The `Value` and `Key` traits use GATs (generic associated types) and lifetime parameters. It is easy to run into borrow-checker issues when wrapping `redb` in generic helper functions or when serializing on the fly. | Keep transaction scopes tight. If you need generic wrappers, prefer owned types or use the `redb-derive` crate for custom types. Avoid trying to pass serialized buffers across function boundaries with borrowed lifetimes. |
| **No native async API**                   | `redb` is synchronous. Holding a write transaction across an `.await` point will block the async runtime.                                                                                                               | Perform `redb` operations in `spawn_blocking` or a dedicated thread. Do not hold transactions across async boundaries.                                                                                                       |
| **No built-in encryption or compression** | The core crate does not encrypt or compress data at rest.                                                                                                                                                               | Layer encryption/compression in your value serialization, or use extensions like `redb-turbo` which provide page-level encryption and compression plug-ins.                                                                  |
| **Removals can be slower than inserts**   | Benchmarks show that bulk removals lag behind LMDB and RocksDB.                                                                                                                                                         | If your workload involves heavy deletion, test compaction behavior and consider periodic database rebuilds if latency matters.                                                                                               |

### Security note

`redb` uses xxhash (a fast, non-cryptographic hash) for page checksums. This is sufficient for crash detection but **not** for malicious tampering. If you need protection against adversarial disk modification, layer a cryptographic integrity scheme above `redb`, or use the 2-phase commit option which hardens against a narrower class of attack scenarios.
<choice>STOP</choice>
