---
prompt: |-
    The 'duckdb' crate will be used in the "rendezvous" daemon.
    
    Your task is to do a deep dive on the 'duckdb' crate and answer the following questions through thorough research:
    
    - What is the functional footprint of the 'duckdb' crate?
    - What features does duckdb expose and what functionality do these features map to? When should you use each feature? When should you avoid?
    - What are the key URLs for this crate? Repo? Website? Docs?
    - What are 2-3 common use cases that this crate would be used for? For each use case, describe the use case and provide Rust code examples of how this use case might be implemented.
    - What do developers say about using this crate? What "gotchas" are there and how can they be worked around?
    - What is it's major version history? What is the latest version as of today?

    If you want to use diagrams for visual information then it is preferred if you use Mermaid as the diagram DSL.

    When completed with the research, set the `latest_version` Frontmatter property on this page to the latest version of this crate.
latest_version: 1.10503.1
last_updated: 2026-05-24
---
## `duckdb` crate

The **`duckdb`** crate (officially published as `duckdb-rs`) provides ergonomic, high-performance Rust bindings for **DuckDB**, an embeddable analytical (OLAP) database engine. It is designed to feel familiar to users of `rusqlite` while exposing DuckDB's vectorized execution engine and rich analytical feature set.

### Functional Footprint

The crate enables Rust applications to execute complex SQL queries against large datasets with minimal overhead. Its footprint covers:

- **Vectorized Query Execution:** Directly leverages DuckDB's columnar storage and SIMD-optimized execution.
- **Embedded OLAP:** Operates as a library within the process (no separate server required) or connects via the "Quack" protocol.
- **Interoperability:** Native integration with Apache Arrow, Polars, and various data formats (Parquet, CSV, JSON).
- **Extension Development:** Utilities to write DuckDB extensions (UDFs, Virtual Tables) entirely in Rust.
- **Bulk Data Ingestion:** Highly optimized `Appender` API for zero-copy data loading from Rust memory to DuckDB.

```mermaid
graph TD
    subgraph "Rust Application"
        App[Application Logic] --> Conn[Connection / Pool]
        Conn --> SQL[SQL Query Engine]
        Conn --> Appender[Bulk Appender API]
    end
    subgraph "DuckDB Engine (C++)"
        SQL --> Vector[Vectorized Execution]
        Appender --> Vector
        Vector --> Storage[Hybrid Storage / Parquet / Memory]
    end
    subgraph "Data Sources"
        Storage --> FS[File System]
        Storage --> S3[Cloud Storage]
        Storage --> Arrow[Arrow / Polars Data]
    end
```

---

### Features & Functionality

The crate uses feature flags to manage its large dependency footprint.

| Feature Flag       | Maps To...                      | When to Use                                                                           | When to Avoid                                                     |
|:-------------------|:--------------------------------|:--------------------------------------------------------------------------------------|:------------------------------------------------------------------|
| `bundled`          | Statically links DuckDB source. | **Always** for standalone apps to ensure portability and simple builds.               | When building a **loadable extension** (causes symbol conflicts). |
| `parquet` / `json` | Support for these formats.      | When processing logs, data lake files, or document stores.                            | If your app only uses in-memory SQL tables.                       |
| `vtab-arrow`       | Arrow-based Virtual Tables.     | High-performance data sharing with `arrow` or `polars` crates.                        | If you don't need zero-copy data exchange.                        |
| `chrono` / `uuid`  | Type support for common crates. | When your schema uses standard Rust types for time or IDs.                            | If using built-in DuckDB types or raw strings is sufficient.      |
| `appender-arrow`   | Arrow data bulk loading.        | When ingesting millions of rows from an Arrow source quickly.                         | For small, infrequent inserts (use standard SQL).                 |
| `polars`           | `query_polars` integration.     | When you need to jump between SQL (DuckDB) and DataFrame (Polars) logic.              | If you prefer standard SQL for everything.                        |
| `bundled-cmake`    | Uses CMake to build core.       | On systems where the default build script fails or when building specific extensions. | Standard Unix/macOS builds (slower than default).                 |

---

### Key URLs

- **Crates.io:** [crates.io/crates/duckdb](https://crates.io/crates/duckdb)
- **GitHub Repository:** [github.com/duckdb/duckdb-rs](https://github.com/duckdb/duckdb-rs)
- **Official Documentation:** [docs.rs/duckdb](https://docs.rs/duckdb)
- **DuckDB Project Site:** [duckdb.org](https://duckdb.org)

---

### Common Use Cases

#### 1. High-Performance Analytical Querying (Parquet)

Using DuckDB to query millions of rows from a Parquet file and returning the results into Rust structs.

```rust
use duckdb::{Connection, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Metric {
    timestamp: i64,
    value: f64,
}

fn query_parquet(file_path: &str) -> Result<Vec<Metric>> {
    let conn = Connection::open_in_memory()?;
    
    // Query directly from a file without loading into memory first
    let mut stmt = conn.prepare(&format!(
        "SELECT timestamp, value FROM read_parquet('{}') WHERE value > 0.5", 
        file_path
    ))?;

    let metric_iter = stmt.query_map([], |row| {
        Ok(Metric {
            timestamp: row.get(0)?,
            value: row.get(1)?,
        })
    })?;

    metric_iter.collect()
}
```

#### 2. Bulk Data Ingestion (Appender)

Loading a stream of telemetry data into the database at maximum speed.

```rust
use duckdb::{Connection, Result};

fn bulk_load_telemetry(conn: &Connection, batch: Vec<(String, f64)>) -> Result<()> {
    // Create table if not exists
    conn.execute("CREATE TABLE IF NOT EXISTS telemetry (tag TEXT, val DOUBLE)", [])?;

    // Use Appender for high-performance bulk insert
    let mut appender = conn.appender("telemetry")?;
    for (tag, val) in batch {
        appender.append_row([tag, val])?;
    }
    
    // Flush to storage
    appender.flush()?;
    Ok(())
}
```

#### 3. Client-Server Communication (Quack Protocol)

In a daemon like `rendezvous`, connecting to a remote DuckDB instance or exposing a local one.

```rust
use duckdb::{Connection, Result};

fn connect_remote() -> Result<Connection> {
    // As of 2026, DuckDB supports the Quack protocol for client-server
    // This allows the daemon to interact with a centralized signal store.
    Connection::open("quack://admin:secret@127.0.0.1:5555/signals_db")
}
```

---

### Developer Feedback & Gotchas

#### Developer Sentiment

Developers generally praise the crate for its **speed** and **ease of use**, particularly the ability to query CSV/Parquet files as if they were tables. The alignment with `rusqlite` makes the learning curve very low for Rust backend engineers.

#### Key Gotchas & Workarounds

1. **Arrow Version Hell:** The `vtab-arrow` feature depends on a specific version of the `arrow` crate.

    - **Workaround:** Always check `duckdb::arrow` and use the re-exported types from the `duckdb` crate rather than importing your own `arrow` dependency to avoid type mismatches.

2. **`bundled` vs Loadable Extensions:** If you are building a `.duckdb_extension` in Rust, enabling the `bundled` feature will cause a linker error or a crash on load because the extension will try to link its own copy of DuckDB.

    - **Workaround:** Use the `loadable-extension` feature and ensure `bundled` is disabled in the extension crate.

3. **Relational API Performance:** Using `conn.table(...)` (the Relational API) with many filters is sometimes slower than raw SQL because it may bypass certain query optimizer steps.

    - **Workaround:** Use standard SQL `conn.prepare(...)` for performance-critical path queries.

4. **Transaction Management:** While DuckDB is ACID, its locking model is "Single Writer, Multiple Reader" (SWMR). Attempting to write from multiple connections simultaneously will result in a "Database is locked" error.

    - **Workaround:** Use a single write connection with a queue or `r2d2` for managing access, or use the `Appender` which is optimized for sequential batching.

---

### Version History

DuckDB and its Rust crate transitioned to a "Engine-Aligned" versioning scheme in early 2026.

- **1.0.0 (June 2024):** First stable production release.
- **1.2.0 (April 2025):** Full support for DuckDB's internal `ENUM` types and upgraded Arrow 50 support.
- **1.4.0 (Sept 2025):** LTS release for DuckDB 1.4; introduced zero-copy string inserts.
- **1.10500.0 (March 2026):** **The Great Version Shift.** Transitioned to `1.1[MAJOR][MINOR][PATCH].x` format. Upgraded to Rust 2024 Edition.
- **1.10503.0 (Latest):** Released May 20, 2026. Added native support for the Quack remote protocol and Arrow 58 integration.

**Latest Version:** `1.10503.0` (Aligned with DuckDB v1.5.3)
