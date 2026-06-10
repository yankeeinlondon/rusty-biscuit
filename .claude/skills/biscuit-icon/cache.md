# Local SQLite Cache

`Icon::iconify` and the `icon sets` / `icon icons` CLI commands are
backed by a single on-disk SQLite database. Cache hits never touch the
network; cache misses fetch from the Iconify API and persist the body
for next time.

## Location

Default path (resolved via `dirs`):

```
~/.cache/biscuit-icon/icons.db
```

`IconCache::open_default()` creates the parent directory if needed.
Tests use `IconCache::open_at(path)` to point at a `tempfile` directory.

There is no TTL — entries persist until `icon cache clear` or until the
file is removed by hand. This is a deliberate reproducibility stance
(see the kickoff spec, "Cache expiry: none").

## Schema

```sql
CREATE TABLE IF NOT EXISTS icons (
    prefix     TEXT NOT NULL,
    name       TEXT NOT NULL,
    body       TEXT NOT NULL,   -- raw Iconify <path>… body
    left       INTEGER NOT NULL DEFAULT 0,
    top        INTEGER NOT NULL DEFAULT 0,
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    fetched_at TEXT NOT NULL,   -- RFC3339
    PRIMARY KEY (prefix, name)
);
CREATE INDEX IF NOT EXISTS idx_icons_name ON icons(name);

CREATE TABLE IF NOT EXISTS sets (
    prefix          TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    license         TEXT,
    license_title   TEXT,
    license_url     TEXT,
    total           INTEGER,
    fetched_at      TEXT NOT NULL
);
```

The cache stores **the body and viewBox only** — no styling, no
glyphs. Styling is applied locally by `Style::assemble`. Glyphs are
only attached to embedded domain icons.

## Schema Versioning

The store uses `PRAGMA user_version` to drive migration. Each step
runs in a single transaction so a failure mid-migration rolls back
rather than advertising a partially-migrated schema.

- **v0 → v1** — table creation, the original `view_box TEXT` →
  `left INTEGER` / `top INTEGER` origin conversion, column drops,
  and the version bump.
- **v1 → v2** — `ALTER TABLE sets ADD COLUMN total INTEGER` (nullable)
  + `PRAGMA user_version = 2`. Existing `sets` rows read as
  `total = None`; no backfill, no network request on open.

The migration is idempotent: a database that already contains `total`
but reports an older `user_version` is detected by a
`PRAGMA table_info(sets)` check and the `ALTER` is skipped.

## API Surface

`biscuit_icon::cache::IconCache` (re-exported at the crate root via
`SetInfo`):

```rust
impl IconCache {
    pub fn open_default() -> Result<Self>;            // ~/.cache/biscuit-icon/icons.db
    pub fn open_at(path: impl AsRef<Path>) -> Result<Self>;
    pub fn clear(&self) -> Result<()>;
    pub fn put(&self, prefix: &str, name: &str, body: &IconBody) -> Result<()>;
    pub fn get(&self, prefix: &str, name: &str) -> Result<Option<IconBody>>;
    pub fn search_names(&self, needle: &str) -> Result<Vec<String>>;
    pub fn cached_prefixes(&self) -> Result<Vec<String>>;
    pub fn cached_icon_counts(&self, prefixes: &[String]) -> Result<BTreeMap<String, usize>>;
    pub fn put_set(&self, set: &SetInfo) -> Result<()>;
    pub fn search_sets(&self, needle: &str) -> Result<Vec<SetInfo>>;
    pub fn all_sets(&self) -> Result<Vec<SetInfo>>;
    pub fn path(&self) -> &Path;
}
```

`SetInfo` is the cache-resident shape for a collection:

```rust
pub struct SetInfo {
    pub prefix: String,
    pub title: String,
    pub license: Option<String>,         // SPDX id
    pub license_title: Option<String>,
    pub license_url: Option<String>,
    pub total: Option<usize>,            // None when unknown (not zero)
}
```

## Concurrency

Cache reads and writes are synchronous SQLite I/O. The library moves
them off the async runtime thread with `tokio::task::spawn_blocking`,
so a Tokio runtime (or the CLI's `#[tokio::main]`) does not block on
disk. The `IconCache` value is cheaply `Clone` (it carries only a
`PathBuf`; each call re-opens the connection).

## What Is Not Cached

- Embedded domain icons. The compiled-in curated subset is always
  available offline and is never written to the cache. The
  `Cached` column in `icon sets` counts SQLite rows only.
- Styling. Every render is reassembled from the body + a fresh
  `Style`.
- Glyph data. Only embedded domain icons carry Unicode / Nerd Font
  codepoints, and those are compiled in.

## Maintenance

| Action | Effect |
|--------|--------|
| `icon cache clear` | Drops every row in `icons` and `sets`. The schema is preserved. |
| `rm ~/.cache/biscuit-icon/icons.db` | Same effect, plus the file is removed. Re-opened on next use. |
| `RUST_LOG=icon=debug` or `--debug` | Logs cache operations to stderr. |

The cache is intentionally inspectable: any SQLite client (the
`sqlite3` CLI, `DB Browser for SQLite`, ...) can read it.

## Worked Example

```rust
use biscuit_icon::Icon;
use biscuit_icon::cache::IconCache;

let cache = IconCache::open_default()?;

// First call: cache miss → fetch from api.iconify.design → persist.
let home = Icon::iconify_with("mdi:home", &cache, &IconifyClient::new()).await?;

// Second call: cache hit → never touches the network.
let home_again = Icon::iconify_with("mdi:home", &cache, &IconifyClient::new()).await?;
```

A custom base URL (for self-hosted Iconify or a test server) is wired
through the `IconifyClient`:

```rust
use biscuit_icon::iconify::IconifyClient;
let client = IconifyClient::with_base("http://localhost:8080");
```
