---
title: Icon Sets Formatting and Count Tracking Specification
status: ready for planning and implementation
reviewed: true
last_updated: 2026-06-09
---

# Icon Sets Formatting and Count Tracking

This feature replaces the tab-separated `icon sets` output with a
`biscuit-terminal` table, records Iconify's collection totals in the SQLite
cache, and reports how many icons from each collection are cached locally.

> **Reader's note:** The draft proposed a combined count string, dimmed prefix
> cells, a split breakpoint of 60 columns, one cached-count query per set, and
> an optional API total defaulting to zero. This revision uses separate typed
> count columns, removes unsupported per-cell styling, requires enough width for
> both tables, batches cached-count queries, and preserves an unknown upstream
> total as `None`. These choices match the current `biscuit-terminal` and cache
> contracts and avoid misleading output or unnecessary database work.

## Goals

1. Render `icon sets` with the `biscuit-terminal` `Table` component.
2. Use a side-by-side `TwoColumn` layout when it reduces vertical scrollback
   and both tables fit at readable widths.
3. Persist each collection's upstream icon total from Iconify's `/collections`
   response.
4. Report the upstream total and the number of SQLite-cached icons for every
   displayed collection.
5. Preserve useful output when the network is unavailable or cached metadata
   predates this feature.

## Non-Goals

- Changing when `icon sets` attempts to refresh collection metadata.
- Counting embedded domain icons as cached icons. `Cached` means rows in the
  SQLite `icons` table.
- Downloading icons to make cached and total counts match.
- Adding raw ANSI escape sequences or extending `Table` with styled cell
  content.
- Adding JSON, CSV, or other machine-readable output modes.

## Design Decisions

### Collection metadata API

Replace the public tuple returned by `fetch_collections` with a named type:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionInfo {
    pub prefix: String,
    pub title: String,
    pub license: Option<License>,
    pub total: Option<usize>,
}

pub async fn fetch_collections(&self) -> Result<Vec<CollectionInfo>>;
```

This is an intentional breaking change to the pre-1.0 API. A named type avoids
adding another positional tuple field and gives future collection metadata a
stable home.

The private Iconify response type must deserialize `total` as `Option<usize>`,
not `usize` with `#[serde(default)]`. A missing value is unknown and must not be
reported as zero. A present `0` remains a valid known value.

`fetch_collections` continues returning collections in prefix order because the
response is decoded through a `BTreeMap`.

### Cached set metadata

Extend `SetInfo` with the same optional total:

```rust
pub struct SetInfo {
    pub prefix: String,
    pub title: String,
    pub license: Option<String>,
    pub license_title: Option<String>,
    pub license_url: Option<String>,
    pub total: Option<usize>,
}
```

All constructors, queries, fixtures, and call sites must set or read this
field. Built-in-only set entries use `None` until matching Iconify collection
metadata has been cached.

### SQLite migration

Schema version 2 adds a nullable `total INTEGER` column to `sets`.

Migration must be driven by `PRAGMA user_version`, not only by a column check
inside the existing version-0 migration:

1. Run the existing version-0 to version-1 migration when required.
2. When `user_version < 2`, inspect `PRAGMA table_info(sets)`.
3. Add `total INTEGER` only when the column is absent.
4. Set `PRAGMA user_version = 2` after the migration succeeds.

Run each version step in a transaction so a failed migration does not advertise
the new version. The column check keeps migration idempotent for databases that
already contain `total` but have an older or missing `user_version`.

`put_set`, `search_sets`, and `all_sets` must include `total`. Existing rows
naturally read as `None`; no backfill or network request occurs during database
open.

### Cached icon counts

Do not call `COUNT(*)` once per displayed set. Add a batch cache API:

```rust
pub fn cached_icon_counts(
    &self,
    prefixes: &[String],
) -> Result<BTreeMap<String, usize>>;
```

The implementation performs one grouped query equivalent to:

```sql
SELECT prefix, COUNT(*)
FROM icons
WHERE prefix IN (...)
GROUP BY prefix;
```

Use bound parameters for every prefix. Return no entry for a prefix with zero
cached rows; presentation treats a missing map entry as zero. An empty input
returns an empty map without opening a query.

The query counts only SQLite rows. Embedded domain icons are always available
offline but are not cache entries and therefore do not increase `Cached`.

## Command Flow

The existing online/offline merge behavior remains:

1. Load matching built-in and cached set metadata.
2. Attempt `fetch_collections`.
3. On success, cache all returned collection metadata, including `total`, then
   merge matching online rows by prefix.
4. On failure, use matching offline rows. Fail only when no offline rows are
   available, preserving the current error contract.
5. Sort the final rows by prefix.
6. Fetch cached icon counts once for all final prefixes.
7. Build and render the selected layout.

An online collection replaces an offline row with the same prefix. A cached
row's known total survives an offline run. A built-in-only row displays an
unknown total.

## Table Output

Every table has four columns:

| Column | Content | Alignment |
|--------|---------|-----------|
| `Set` | Human-readable collection title | Left |
| `Prefix` | Iconify collection prefix | Left |
| `Total` | Upstream icon count, or `Unknown` | Right |
| `Cached` | Number of matching rows in SQLite | Right |

Use `ColumnType::Integer` for the count columns so known values receive the
table component's locale-independent thousands separators. `Unknown` is a text
cell in the otherwise right-aligned `Total` column.

Enable `.alternate_background_color()` on every table. Do not embed ANSI
escapes in cells. The current `TableCellContent` contract has no `Prose` or
per-cell style variant, so the prefix remains plain text.

Constrain `Set` and `Prefix` to reasonable maximum widths and allow text
wrapping rather than allowing a long collection title to force the count
columns off screen. The implementation should use the existing `TableColumn`
width and wrapping APIs; it must not truncate count values.

Render through `TerminalRenderable` using one `Terminal` instance so width,
height, color capability, and table rendering share the same terminal context.

## Adaptive Layout

Layout selection depends on both height and width.

### Height decision

Define a tested helper that returns the number of data rows that fit in one
table for a terminal height. It must account for the table's header and border
overhead and clamp to at least one data row.

Use a single table when all rows fit within that budget. Consider a split only
when the result set exceeds the budget.

### Width decision

Use these explicit layout constants:

```rust
const SPLIT_GAP: u32 = 3;
const MIN_TABLE_WIDTH: u32 = 48;
const MIN_SPLIT_WIDTH: u32 = MIN_TABLE_WIDTH * 2 + SPLIT_GAP;
```

Use two tables only when `term.width() >= MIN_SPLIT_WIDTH`. Otherwise use one
table, even when it exceeds the height budget. This prevents the draft's
61-column case from squeezing two four-column tables into unreadable panes.

Construct the split with:

```rust
TwoColumn::new(left_table, right_table)
    .with_left_percent(0.5)
    .with_gap(SPLIT_GAP)
```

### Row distribution

Split rows in column-major reading order:

```rust
let midpoint = rows.len().div_ceil(2);
let (left, right) = rows.split_at(midpoint);
```

The left table receives the extra row for odd result counts. Both tables repeat
the same headers and preserve prefix ordering from top to bottom, then left to
right.

If there are no rows, preserve the existing command error behavior rather than
rendering an empty table.

## Testing

### Library tests

- `/collections` parses a present total, a present zero, and a missing total.
- `fetch_collections` returns named `CollectionInfo` values in prefix order.
- A fresh database is created at schema version 2 with nullable `sets.total`.
- A version-0 database migrates through version 2 without losing icon origins,
  license metadata, or set rows.
- A version-1 database migrates to version 2 and existing set totals read as
  `None`.
- A database that already has `total` but reports an older version migrates
  idempotently.
- `SetInfo.total` round-trips through `put_set`, `search_sets`, and `all_sets`.
- `cached_icon_counts` returns grouped counts, omits zero-count prefixes, and
  handles an empty prefix list.
- Cached counts do not include embedded domain icons.

### CLI tests

- Online collection totals are displayed and persisted for a later offline run.
- Missing totals display `Unknown`, not `0`.
- Cached counts display zero and nonzero values correctly.
- Known counts use thousands separators.
- Fixed narrow dimensions render one table.
- Fixed wide, short dimensions render two tables with balanced row counts and
  stable column-major ordering.
- Fixed wide dimensions still render one table when all rows fit vertically.
- Output contains no raw Prose markup.
- Existing online-failure and offline-fallback tests continue to pass.

Layout tests should exercise a pure rendering helper with a fixed-size
`Terminal`; they must not depend on the developer's live terminal dimensions.
Use the existing `biscuit-test-harness` levels for any real-terminal coverage.
Tests must not call the live Iconify service.

## Acceptance Criteria

1. `icon sets` renders a striped, aligned table with `Set`, `Prefix`, `Total`,
   and `Cached` columns.
2. Known totals and cached counts are right-aligned and formatted with thousands
   separators; unavailable totals display `Unknown`.
3. A side-by-side layout is used only when rows exceed the tested height budget
   and the terminal is at least `MIN_SPLIT_WIDTH` columns wide.
4. Split output is balanced, repeats headers, and preserves deterministic prefix
   ordering.
5. Iconify collection totals persist across offline runs.
6. Existing version-0 and version-1 cache databases migrate transparently to
   schema version 2 without data loss.
7. Cached counts are obtained with one grouped query rather than an N+1 query
   loop.
8. The implementation uses `biscuit-terminal` renderable components and emits
   no hand-authored terminal escape sequences.
9. Relevant library, CLI, migration, and terminal-layout tests pass on macOS.
