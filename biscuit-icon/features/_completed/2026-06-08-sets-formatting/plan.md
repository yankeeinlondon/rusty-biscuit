---
title: Icon Sets Formatting and Count Tracking — Execution Plan
phases: 5
created: 2026-06-08
start_phase: 1
spec: ./spec.md
source_files_during_phase_1:
  - biscuit-icon/lib/src/iconify/client.rs
  - biscuit-icon/lib/src/iconify/mod.rs
  - biscuit-icon/cli/src/commands.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-icon/lib/src/cache/store.rs
  - biscuit-icon/lib/src/cache/mod.rs
  - biscuit-icon/lib/src/catalog.rs
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/cli/tests/cli.rs
  - biscuit-icon/lib/tests/catalog.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-icon/cli/src/commands.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/cli/src/main.rs
  - biscuit-icon/cli/src/sets_table.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/cli/tests/cli.rs
docs_updated_during_phase_5:
  - biscuit-icon/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
  - biscuit-icon/lib/src/iconify/client.rs
  - biscuit-icon/lib/src/iconify/mod.rs
  - biscuit-icon/lib/src/cache/store.rs
  - biscuit-icon/lib/src/cache/mod.rs
  - biscuit-icon/lib/src/catalog.rs
  - biscuit-icon/cli/src/commands.rs
  - biscuit-icon/cli/src/main.rs
  - biscuit-icon/cli/src/sets_table.rs
  - biscuit-icon/cli/tests/cli.rs
  - biscuit-icon/lib/tests/catalog.rs
documentation:
  - biscuit-icon/README.md
packages:
  - biscuit-icon
  - biscuit-icon-cli
---

# Icon Sets Formatting and Count Tracking — Execution Plan

Derived from [`spec.md`](./spec.md). The work splits into two independent library
seams (Iconify collection metadata, SQLite cache schema/queries), a command-flow
integration that consumes both, a terminal rendering/layout layer, and a final
validation pass.

## Conventions for the Implementation Team

- Follow TDD: write the failing test described in each task, then implement.
- Tests must **never** call the live Iconify service. Use `wiremock` for HTTP and
  `ICONIFY_BASE_URL` / `tempfile` HOME isolation for the CLI (see existing tests
  in `biscuit-icon/cli/tests/cli.rs`).
- Use US English (en-US) for all symbol names and docs.
- Do not run `cargo fmt` unless explicitly told.
- Build/test with targeted package flags, never a bare repo-root build:
  - `cargo test -p biscuit-icon` (library)
  - `cargo test -p biscuit-icon-cli` (CLI)
  - or the area recipes: `just -f biscuit-icon/justfile test` / `lint`.
- Rustdoc: no `# H1` inside `///`; use `## Examples`, `## Errors`, etc.

## Key Files

| Concern | File |
|---------|------|
| Iconify client + `fetch_collections` | `biscuit-icon/lib/src/iconify/client.rs` |
| Iconify module re-exports | `biscuit-icon/lib/src/iconify/mod.rs` |
| SQLite cache, `SetInfo`, migration | `biscuit-icon/lib/src/cache/store.rs` |
| Cache module re-exports | `biscuit-icon/lib/src/cache/mod.rs` |
| Offline set assembly | `biscuit-icon/lib/src/catalog.rs` |
| `sets` command flow | `biscuit-icon/cli/src/commands.rs` |
| Library tests | `biscuit-icon/lib/src/**` (`#[cfg(test)]`), `lib/tests/*` |
| CLI tests | `biscuit-icon/cli/tests/cli.rs`, `cli/tests/level2_terminal.rs` |

## Verified API Anchors (already present, do not re-derive)

- `biscuit_terminal::components::table::{Table, TableColumn, TableCellContent, ColumnType}`
  - `Table::new().with_columns(Vec<TableColumn>).with_data(Vec<Vec<TableCellContent>>).alternate_background_color()`
  - `TableColumn::new(header).with_type(ColumnType::Integer).with_max_width(usize).with_word_wrap(..)`
  - `TableCellContent::Integer(i64)` (thousands separators via `ColumnType::Integer`);
    `TableCellContent::Text(String)` for the `Unknown` total cell. `From<i64>` / `From<&str>` exist.
- `biscuit_terminal::components::two_column::TwoColumn::new(left, right).with_left_percent(f32).with_gap(u32)`
- Render via `TerminalRenderable` + a single `biscuit_terminal::terminal::Terminal`.

---

## Phase 1 — Iconify Collection Metadata API (`CollectionInfo` + total parsing)

Goal: `fetch_collections` returns a named `CollectionInfo` carrying an
`Option<usize>` upstream total. *Independent of Phase 2 — may run in parallel.*

- [x] In `iconify/client.rs`, add `total` to the private `CollectionMeta`
      response type as `#[serde(default)] total: Option<usize>` so a **missing**
      value deserializes to `None`, a present `0` to `Some(0)`, and a present `N`
      to `Some(N)`. Do **not** use `usize` + default (that would mask missing as 0).
- [x] Add the public type in `iconify/client.rs`:
      ```rust
      #[derive(Debug, Clone, PartialEq)]
      pub struct CollectionInfo {
          pub prefix: String,
          pub title: String,
          pub license: Option<License>,
          pub total: Option<usize>,
      }
      ```
- [x] Change `fetch_collections` signature to
      `pub async fn fetch_collections(&self) -> Result<Vec<CollectionInfo>>`,
      mapping each `BTreeMap` entry into `CollectionInfo`. Preserve the existing
      `BTreeMap` decode so prefix ordering is retained. Update the rustdoc to
      mention the total field.
- [x] Re-export `CollectionInfo` from `iconify/mod.rs`
      (`pub use client::{CollectionInfo, IconifyClient, License, parse_id};`).
- [x] **Test** (`client.rs` `#[cfg(test)]`): replace/extend the
      `fetch_collections_*` test so the `/collections` mock returns three shapes —
      a present total, a present `0`, and a missing total — and assert the parsed
      `CollectionInfo.total` is `Some(n)`, `Some(0)`, and `None` respectively.
- [x] **Test**: assert `fetch_collections` returns `CollectionInfo` values in
      prefix order.
- [x] **Checkpoint:** `cargo test -p biscuit-icon iconify` is green; the old
      tuple-based assertions are gone.

---

## Phase 2 — Cache Schema v2 + Batch Cached Counts (`store.rs`)

Goal: `SetInfo` carries `total`, the DB migrates to `user_version = 2` with a
nullable `sets.total` column, and a single grouped query reports cached icon
counts. *Independent of Phase 1 — may run in parallel.*

### 2a — `SetInfo.total` and persistence

- [x] Add `pub total: Option<usize>` to `SetInfo` in `cache/store.rs`.
- [x] Update `put_set` to write `total` (bind `info.total` — `Option<usize>`
      maps to a nullable INTEGER; cast to `i64`/`u64` as required by `rusqlite`).
- [x] Update `search_sets` and `all_sets` SELECTs + row mappers to read `total`.
- [x] Update the in-module test `SetInfo { .. }` literals to include `total: None`
      (and existing `set_metadata_*` tests still pass).

### 2b — `user_version`-driven migration to schema 2

- [x] In `open_at`, after the existing `version < 1` block, add a `version < 2`
      block that:
      1. Runs in a transaction.
      2. Inspects `PRAGMA table_info(sets)` and adds `total INTEGER` **only when
         absent** (idempotent for DBs that already have the column but a stale
         `user_version`).
      3. Sets `PRAGMA user_version = 2` after success, inside/with the same
         transactional step so a failed migration never advertises v2.
- [x] Ensure a fresh database (created by the `version < 1` branch) ends at
      `user_version = 2` with the `total` column present — either by adding
      `total INTEGER` to the v1 `CREATE TABLE sets` and letting the v2 step bump
      the version, or by falling through the v2 block. Pick whichever keeps a
      single source of truth; document the choice in a code comment.

### 2c — Batch cached icon counts

- [x] Add:
      ```rust
      pub fn cached_icon_counts(
          &self,
          prefixes: &[String],
      ) -> Result<BTreeMap<String, usize>>;
      ```
      - Empty input returns an empty map **without** opening a query.
      - Build a `prefix IN (?, ?, …)` placeholder list with **bound parameters**
        (one per prefix); run
        `SELECT prefix, COUNT(*) FROM icons WHERE prefix IN (...) GROUP BY prefix`.
      - Omit prefixes with zero cached rows (no map entry); presentation treats a
        missing entry as zero.
      - Count only `icons` rows (embedded domain icons are not counted).

### 2d — Phase 2 tests (in `store.rs` `#[cfg(test)]`)

- [x] Fresh DB is created at `user_version = 2` and `PRAGMA table_info(sets)`
      contains a nullable `total`.
- [x] A v0 DB (reuse the existing `migration_from_old_schema_*` fixture shape)
      migrates through v2 without losing icon origins, license metadata, or set
      rows; existing set totals read as `None`.
- [x] A v1 DB migrates to v2 and existing set totals read as `None`.
- [x] A DB that already has a `total` column but reports an older/missing
      `user_version` migrates idempotently (no duplicate-column error).
- [x] `SetInfo.total` round-trips through `put_set` → `search_sets` / `all_sets`.
- [x] `cached_icon_counts` returns grouped counts, omits zero-count prefixes, and
      returns an empty map for an empty prefix list.
- [x] Cached counts exclude embedded domain icons (count reflects only inserted
      `icons` rows).
- [x] **Checkpoint:** `cargo test -p biscuit-icon cache` is green.

---

## Phase 3 — Command Flow Integration (`catalog.rs` + `commands.rs`)

Goal: assemble the final, prefix-sorted set rows with merged totals and a single
cached-count lookup. **Depends on Phases 1 and 2.**

- [x] `catalog.rs::offline_sets`: add `total: None` to the built-in-only
      `SetInfo` constructor at line ~47. (Cached rows from `search_sets`/`all_sets`
      already carry their persisted `total`.)
- [x] `commands.rs::sets`: replace tuple destructuring `(prefix, title, license)`
      with `CollectionInfo` field access for both the cache-all loop and the
      filtered-merge loop. Populate `SetInfo.total` from `info.total` when building
      cached rows, so the upstream total persists.
- [x] After the online/offline merge and final `sort_by(prefix)`, collect the
      sorted prefixes and call `cache.cached_icon_counts(&prefixes)?` **once**.
- [x] Define an internal presentation row model (e.g. a small struct or tuple)
      capturing `{ title, prefix, total: Option<usize>, cached: usize }`, where
      `cached` is `counts.get(prefix).copied().unwrap_or(0)`.
- [x] Preserve the existing error contract: when `fetch_collections` fails **and**
      no offline rows exist, return the current "no offline set listings…" error;
      when the final row set is empty, keep the existing empty/error behavior
      rather than rendering an empty table.
- [x] Replace the `println!("{}\t{}", prefix, title)` loop with a call into the
      Phase 4 rendering entry point (added next). Until Phase 4 lands, the row
      model is the integration boundary.
- [x] **Checkpoint:** `cargo build -p biscuit-icon-cli` compiles; the existing
      `sets_merges_online_and_caches` and `sets_lists_builtin_prefixes_offline`
      tests still pass (output now flows through the table, so update their
      substring assertions to match table output — titles/prefixes still appear).

---

## Phase 4 — Table Rendering + Adaptive Layout

Goal: render the rows as a striped four-column table, choosing a single table or a
balanced `TwoColumn` split based on tested height/width budgets. **Depends on
Phase 3's row model.** Build these as **pure, unit-testable helpers** that take a
fixed-size `Terminal` and the row slice (no reliance on the live terminal).

- [x] Add layout constants (module-level in `commands.rs` or a new
      `cli/src/sets_table.rs`):
      ```rust
      const SPLIT_GAP: u32 = 3;
      const MIN_TABLE_WIDTH: u32 = 48;
      const MIN_SPLIT_WIDTH: u32 = MIN_TABLE_WIDTH * 2 + SPLIT_GAP;
      ```
- [x] `fn build_table(rows: &[Row]) -> Table`:
      - Four columns: `Set` (Left, String, max-width + word wrap), `Prefix`
        (Left, String, max-width + word wrap), `Total` (`ColumnType::Integer`),
        `Cached` (`ColumnType::Integer`).
      - `Total` cell = `TableCellContent::Integer(n as i64)` when `Some(n)`, else
        `TableCellContent::Text("Unknown".into())`.
      - `Cached` cell = `TableCellContent::Integer(cached as i64)`.
      - `.alternate_background_color()`. No ANSI/Prose markup in cells.
      - Constrain `Set`/`Prefix` with `with_max_width` + wrapping so a long title
        cannot push the count columns off-screen; never truncate count values.
- [x] `fn rows_per_table(term_height: u32) -> usize`: data rows that fit in one
      table for a given height, accounting for header + border overhead, clamped
      to **at least 1**.
- [x] `fn choose_layout(rows, term) -> Layout`: single table when
      `rows.len() <= rows_per_table(term.height())`; otherwise a split **only if**
      `term.width() >= MIN_SPLIT_WIDTH`; otherwise fall back to a single table.
- [x] Split distribution: `let midpoint = rows.len().div_ceil(2); rows.split_at(midpoint)`
      (left gets the extra odd row). Both tables repeat headers and preserve
      column-major ordering (top→bottom, then left→right). Build with
      `TwoColumn::new(left, right).with_left_percent(0.5).with_gap(SPLIT_GAP)`.
- [x] `fn render_sets(rows: &[Row], term: &Terminal) -> String`: the pure entry
      point used by `sets`; renders the chosen layout through `TerminalRenderable`
      using the single passed-in `Terminal`. The `sets` command obtains one
      `Terminal`, calls this, and prints the result.
- [x] **Test** (pure helper tests with a fixed-size `Terminal`):
      - `rows_per_table` clamps to ≥1 and accounts for header/border overhead.
      - Narrow width → single table even when rows exceed the height budget.
      - Wide + short → two tables, balanced counts (left ≥ right, differ by ≤1),
        stable column-major ordering.
      - Wide + tall (all rows fit) → single table.
      - Rendered output contains `Set`/`Prefix`/`Total`/`Cached` headers,
        `Unknown` for missing totals, thousands separators for large known values,
        and **no** raw Prose markup / ANSI escapes authored by hand.
- [x] **Checkpoint:** `cargo test -p biscuit-icon-cli sets` green for the layout
      helpers.

---

## Phase 5 — Integration Validation, CLI Tests & Drift

Goal: end-to-end CLI behavior verified, persistence across offline runs proven,
docs/skills updated. **Depends on Phases 1–4.**

- [x] CLI test: online collection totals are displayed and **persisted** —
      first run (wiremock `/collections` with totals) shows the total; second run
      against a dead endpoint with a different filter still shows the cached total.
- [x] CLI test: a collection with a missing total displays `Unknown`, not `0`.
- [x] CLI test: cached counts display correct zero and nonzero values (pre-seed
      the cache with `put` rows for a prefix, leave another prefix empty).
- [x] CLI test: a large known total renders with thousands separators.
- [x] CLI test: fixed narrow dimensions render one table; fixed wide+short render
      two balanced tables with stable column-major ordering; fixed wide+tall
      render one table. Drive deterministic dimensions (env/flag or the pure
      helper) — do **not** depend on the developer's live terminal size.
- [x] CLI test: output contains no raw Prose markup.
- [x] Confirm existing online-failure and offline-fallback tests still pass
      (update only the assertions that legitimately changed from tab-separated to
      table output).
- [x] Use `biscuit-test-harness` levels only for any real-terminal (L2) coverage
      in `cli/tests/level2_terminal.rs`; keep pure-render assertions at L1.
- [x] Drift: update `biscuit-icon/README.md` and any `biscuit-icon/docs/*`
      describing `icon sets` output (tabbed → table; new `Total`/`Cached`
      columns). Update `biscuit-icon/docs/dependencies.md` only if a new crate was
      added (none expected — `biscuit-terminal` is already a CLI dependency).
- [x] **Final checkpoint (all on macOS):**
      - `just -f biscuit-icon/justfile lint` clean (clippy).
      - `cargo test -p biscuit-icon` green.
      - `cargo test -p biscuit-icon-cli` green.
      - Manual smoke: `cargo run -p biscuit-icon-cli -- sets` renders a striped,
        aligned, four-column table; `sets <filter>` narrows correctly.

---

## Dependency Graph

```
Phase 1 (Iconify CollectionInfo) ─┐
                                  ├─► Phase 3 (command flow) ─► Phase 4 (table/layout) ─► Phase 5 (validation)
Phase 2 (cache v2 + counts) ──────┘
```

- **Parallelizable:** Phase 1 and Phase 2 are fully independent (different files,
  different seams) and can be implemented concurrently.
- **Serial after merge:** Phase 3 requires both; Phase 4 requires Phase 3's row
  model; Phase 5 requires the full stack.

## Acceptance Mapping

| Spec acceptance criterion | Covered by |
|---|---|
| 1. Striped four-column table | Phase 4 (`build_table`), Phase 5 smoke |
| 2. Right-aligned counts, thousands sep, `Unknown` | Phase 4 cells, Phase 5 tests |
| 3. Split only when rows exceed height budget AND width ≥ `MIN_SPLIT_WIDTH` | Phase 4 `choose_layout` |
| 4. Balanced, header-repeating, deterministic split | Phase 4 distribution + tests |
| 5. Totals persist across offline runs | Phase 2 persistence, Phase 3 wiring, Phase 5 test |
| 6. v0/v1 → v2 migration without data loss | Phase 2b/2d |
| 7. One grouped query for cached counts | Phase 2c |
| 8. Renderable components, no hand-authored escapes | Phase 4 |
| 9. Library/CLI/migration/layout tests pass on macOS | Phase 5 final checkpoint |
