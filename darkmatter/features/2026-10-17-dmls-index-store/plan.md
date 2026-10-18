---
total_phases: 7
created: 2026-10-17
phase: 1
agent: claude/sonnet
yolo: true
packages:
    - biscuit-file
    - dmls
source_files_during_phase_1:
    - darkmatter/dmls/Cargo.toml
    - Cargo.lock
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/common/oracle.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/index_store_oracle.rs
    - darkmatter/dmls/tests/fixtures/index_store_corpus/
docs_updated_during_phase_1:
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
    - biscuit-file/docs/topics/file-references.md
    - biscuit-file/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/api.md
    - .claude/skills/darkmatter/testing.md
---

# Plan: DMLS Index Store

## Summary and Definition of Done

The spec replaces the in-memory full index that `2026-10-17-logging-improvements`
R7 builds on first need (about 4 s, 550-680 MiB peak RSS) with a persistent
SQLite store of per-file facts. The store **narrows** (proposes candidate
referrer files); the existing `WorkspaceGraph` **decides**, built over a small
temporary file set per request.

The work falls into six pieces:

- **biscuit-file:** `user_cache_dir()` (R7).
- **Graph API:** `build_with_roots` takes an externally supplied wiki-name
  universe, so a small graph judges wiki ambiguity and spellings like the full
  graph (R3).
- **Store core:** facts model, SQLite layout, candidate keys, location,
  permissions, sweep, multi-process writes, `FactsSource` with `StoreFacts` /
  `MemoryFacts` and the failure ladder (R1, R2, R7-R12).
- **Fill and freshness:** first-need revalidation, "ready", open-buffer
  override, live updates, fallback-mode rename freshness (R4-R6).
- **Request paths:** references, DSL transcluded-by, wiki references, heading
  rename, file rename, workspace symbols over the store, multi-root (R3, R8).
- **Removal, observability, docs:** delete the production full build, R13 log
  lines, `--bench-index` (R14), docs and skills.

**Done** means all twelve acceptance criteria in the spec hold, each with a
test or a recorded measurement, and:

- `just test`, `just test-l2` (if any L2 test was touched) and `just lint`
  pass in `darkmatter/`; biscuit-file passes its own `just test` and
  `just lint`.
- No production code path parses every listed file into a `DocumentIndex` or
  keeps a workspace-wide `WorkspaceGraph` for these four requests; the
  full-graph oracle exists only under test.
- The docs and skills named in the spec's Documentation section are updated,
  and none of them names the spec.
- The implementation log records every measurement, ruling departure, and
  the smell-grep results. The author (not an agent) moves the spec to
  `_completed`; this plan's terminal state is "implementation complete, ready
  for review".

## Phase 1: Gate, Rulings, Foundations

Confirm the dependency has landed, settle open points, and build the pieces
every later phase leans on: the test oracle, `user_cache_dir`, the `rusqlite`
dependency.

### Dependency gate

The spec's `message_to_agent` forbids implementing before
`2026-10-17-logging-improvements` lands. At planning time that plan has 9 of 63
tasks checked (Phase 1 only). It supplies the R5 file list, R6 lazy target
cache, R7 state machine, R8 log style, and the `configuration.md` /
`indexing.md` pages that this feature extends.

- [x] **Gate check.** Do not start Wave 2 until `plan.md` of the dependency is
  fully checked and its final validation checkpoint is recorded in its
  implementation log.
  - If the gate is not met, stop and report. Do not stub the dependency's
    interfaces inside this feature.
  - If it is met but an interface differs from this plan (names, the R7 state
    type, the R8 `trigger` values), adapt the plan's wording in the
    implementation log; do not edit the spec.

### Necessary Rules

The spec leaves these points open or S1 contradicted them. They are
**recommendations**; the author confirms or overrides before Wave 2.

1. **S1 has already run; no spike is scheduled.** `spike-s1.md` and
   `spike-s1-windows.md` answer every S1 row except cold-disk revalidation (see
   ruling 14). The spec's "S1 blocks planning" is satisfied. Per the spike
   policy, a spike runs once and is not re-run as a re-check in later phases.
2. **R2 changes (Q3).** A joined-path key is kept, but:
   - explicitly relative forms (`./`, `../`) yield the joined key only;
   - **bare** relative forms (`a/b.md`, `spec.md`) yield the joined key **and**
     the file-name key;
   - wiki links and root-prefixed forms (`~`, `/`, `^`, `@`, `$`, scheme,
     directory) yield the file-name key;
   - no suffix key (S1: barely narrower than the file-name key).

   R2 as written missed 18 of 3,017 true referrers (0.6%), all bare
   repository-root-relative paths; this breaks the spec's own "never miss"
   rule. With the change S1 measured 0 misses, p95 139 candidates.
3. **Batch size and busy timeout (Q2).** Keep 200 files per batch and
   `busy_timeout = 5000` ms. Batch size is not a performance lever (write phase
   0.2-0.6 s against seconds of parsing; worst lock wait 218 ms).
4. **WAL switch retry (new, from S1).** `PRAGMA journal_mode=WAL` on a new file
   returned `SQLITE_BUSY` immediately, bypassing the busy timeout, in 2 of 3
   concurrent first creations. Rule: retry the pragma with 5 ms sleeps for up to
   about 2 s; a still-busy switch is "busy" (R12: use memory), never
   corruption.
5. **Heading scan (Q4).** A full scan is enough (75k headings: 28 ms load,
   11-13 ms per query). No narrowing step. The loaded rows may be kept for the
   session only if a test shows invalidation by every row write path; default
   is to query SQLite per request.
6. **Numeric targets (Q1).** Warm restart, first references request:
   at most **500 ms** from request to answer, not counting the dependency's
   file-list walk (at most **1 s** including it); at most **64 MiB** peak RSS
   added by the request. Acceptance 11 applies to **warm restarts only**. A
   cold fill has no bound tighter than today's full build. S1 numbers came from
   a heavily loaded host (load average 30-190); see ruling 14.
7. **Spec corrections the implementation must carry.**
   - R4 "about 360 ms" for read + hash is about 50 ms parallel (110-150 ms
     sequential) warm; the walk (190-300 ms) belongs to the file list.
   - The Decisions table reason for C over A ("keeps the 680 MB resident") is
     wrong: only about 97 MiB is retained data (25 MiB `DocumentIndex` + 72 MiB
     graph); the rest is parse high-water memory that a cold fill pays under
     either option. C still wins (about 5 MiB and 0.1 s per request versus a
     0.5-1.9 s full build). The docs state the corrected reason. **The spec text
     itself returns to the author**; this plan does not edit it.
8. **Wiki universe shape (R3).** Introduce a `WikiUniverse` value (entries keyed
   by canonical logical path, built from the file list and wiki roots).
   `build_with_roots` takes it as a required parameter. The graph maps each
   `DocumentId` to its universe slot; resolution and rename index the
   universe, never graph documents. Tests and the oracle construct it with a
   helper from a document set. No `Option` default, so no caller silently
   judges ambiguity against a partial set.
9. **Facts load into `DocumentIndex`.** `build_with_roots` still takes
   `BTreeMap<PathBuf, DocumentIndex>`. Stored facts load into a `DocumentIndex`
   whose unstored fields (`schema_uses`, `file_uses`, `variable_uses`,
   `frontmatter_keys`) are empty and whose `content_hash` is the row hash
   (S1's approach; `LinkTarget` is recomputed with `classify_link_target` on
   load). S1 found 18 `uses_file` / `uses_schema` edges to indexed `.md` files
   and no provider that queries those edge kinds; the oracle test (acceptance
   4) is what keeps that true.
10. **Candidate paths are stored as given by the file list** (absolute,
    canonical-simplified). The joined-path key is built with `/` separators,
    lexically normalized (`.` and `..` folded), case-folded, NFC. A root move
    changes the store key (R8), so absolute paths are safe. On Windows the key
    must not depend on verbatim (`\\?\`) prefixes or separators.
11. **`DMLS_CACHE_DIR` value shapes.** Unset or empty = unset (default
    location). `off` (ASCII case-insensitive) = kill switch. A relative path or
    one that exists as a file = memory with one `warn` (an explicit override
    that cannot be honored is not silently ignored).
12. **Unix sweep liveness (from the Windows spike).** On Unix `unlink` of an
    open store succeeds, so the age check is the only protection. Rule:
    touch the store file's mtime and `meta.last_opened` on open and at most
    once per hour on any use or batch thereafter. On Windows open the file for
    write before `set_modified` (a read handle fails with os error 5). The
    sharing violation check is `raw_os_error() == Some(32)`
    (`cfg(windows)`); `io::ErrorKind` cannot be matched.
13. **Corruption recovery strands other processes on Unix.** Delete-and-recreate
    succeeds while another process holds the old inode; that process keeps
    writing to an orphan. Accepted: the store is only a cache and the other
    process's next open rebuilds. Documented in `indexing.md`, not engineered
    around.
14. **Wider measurement (author decision, not scheduled).** Cold-disk
    revalidation (`sudo purge` between runs, not runnable by an agent) and a
    quiet-host re-measure of every S1 absolute time. Phase 7 takes one quick
    warm sample on this host to check the ruling 6 numbers only. A cold-disk
    figure and a multi-OS performance sample are the author's call.
15. **Nested roots (Q6).** Yes: candidates and workspace-symbol results are
    de-duplicated by canonical path before the graph is built and before
    results are returned, matching today's single graph.
16. **`storage-strategy` skill (Q5).** It lives only in
    `~/.claude/skills/`. Not copied into the repo. The implementation log
    records the exact line to add so the author applies it outside this change.
17. **Strict tables.** Create store tables as `STRICT` (bundled SQLite 3.45
    supports it) so a wrongly typed value is a write error, and a wrongly typed
    value on read is corruption (R12), never coerced.
18. **Concurrency model.** One `rusqlite::Connection` per root behind a mutex,
    used from the existing R7 shared-build worker. Revalidation reads and
    hashes in parallel with the same bounded worker pool the dependency uses
    for parsing; SQLite access is serialized.

### Spikes

None. S1 reported (ruling 1). Nothing else in the spec needs measuring before
work starts.

### Input Robustness Matrix

The store adds readers of a persisted format (the SQLite file and its `meta`
table) and of two environment inputs (`DMLS_CACHE_DIR`, `XDG_CACHE_HOME`).
Text grammars are not in scope.

**Default outcome.** Every unusable store or override produces a located
`warn` (once per process), switches the process to `MemoryFacts`, and all four
requests still answer correctly. A cache problem never fails a request and
never publishes a wrong answer.

**Loaders (test once each):**

| Loader | Shapes tested | Outcome |
| --- | --- | --- |
| `IndexStore::open` (SQLite file) | control: valid store; zero-byte file; non-database garbage; truncated file; valid DB missing a table; `meta` format stamp differing from the file name's `FACTS_VERSION`; `meta.root` mismatch | control opens; garbage, truncated, missing table, stamp mismatch = corruption, recreate once; root mismatch = delete and refill (not a warn-level failure) |
| `user_cache_dir()` | control per OS; `$XDG_CACHE_HOME` unset, empty, relative, absolute; home unavailable | relative or empty = unset; home unavailable = `None` = memory |
| `DMLS_CACHE_DIR` | unset, empty, `off`, `OFF`, absolute missing, absolute existing, relative, path that is a file | per ruling 11; missing directory created `0700` |

**Field shapes (store rows, one shared decode path):** a row decode has one
validator. Walk absent (NULL in a `NOT NULL` column cannot be written),
wrong type (`STRICT` rejects on write; on read a type mismatch = corruption),
and empty (`""` title, zero-length file: stored as a valid empty row, never
skipped). Each table (`files`, `headings`, `links`, `wiki`, `transclusions`,
`cand`) needs the control read plus one corrupted-cell read proving it goes
through the shared decode and yields corruption, not an empty result.

**Exceptions listed:** an empty heading title and a zero-byte Markdown file
are valid facts. A row whose hash differs from the process's own read is
re-parsed, not an error.

**Tests:** in-process at `IndexStore::open` / `FactsSource` and
`user_cache_dir`; spawn the `dmls` binary only for one wiring test (an
`DMLS_CACHE_DIR=off` session and a corrupt-store session answering a
references request over stdio).

**Smell grep before closing the matrix** (record results in the log):
`rg 'unwrap_or_default\(\)|\.ok\(\)|filter_map|serde\(default' darkmatter/dmls/src/index_store`
and the same for the new `user_cache_dir`. A row decode that turns an error into
"no rows" is a defect. `query_map(..).filter_map(Result::ok)` is the typical
form.

### Wave 1 (parallel)

- [x] **Oracle harness.** Move the dependency's full-index build into a
  test-only module and the shared fixture corpus.
  - The oracle builds `WorkspaceGraph` over every file of a corpus and runs
    the four providers; the store path must later equal it (acceptance 4, 5).
  - Corpus covers: relative and bare links, root-relative bare paths (the 18
    S1 misses), case and NFC differences, extension-elided wiki links,
    ambiguous wiki names, percent-encoded and `<...>` destinations, `file://`
    and absolute forms, directory links, `::file` / `::code` transclusions, a
    symlinked directory, two roots, a nested root, a duplicate file name, and
    a referrer with a heading anchor.
  - Prerequisite: the dependency gate.
  - Do this **before** deleting the production path (Phase 6).
- [x] **`user_cache_dir`.** In `biscuit-file`, add
  `pub fn user_cache_dir() -> Option<PathBuf>` beside `home_dir()`
  (`file_reference/context.rs`).
  - Linux `$XDG_CACHE_HOME` (relative or empty = unset) else `~/.cache`; macOS
    `~/Library/Caches`; Windows `%LOCALAPPDATA%`.
  - Take injected environment and home for tests; the public function wraps
    the process values.
  - Load the `biscuit-file` skill first. No `dirs::cache_dir` in dmls.
  - Prerequisite: none.
- [x] **Dependency add.** Add `rusqlite = { version = "0.31", features =
  ["bundled"] }` to `darkmatter/dmls/Cargo.toml`, matching `claudine/lib`.
  - Update `darkmatter/docs/dependencies.md` and `docs/dependencies.md`.
  - Prerequisite: none.
- [x] **Removal inventory.** Use GitNexus / `rg` to list every production
  consumer of the dependency's full-index build (`collect_indices*`,
  `index_workspace`, `WorkspaceIndex::from_indices`, `bench.rs`), and of
  `WorkspaceGraph::wiki_docs()`. Write the list in the implementation log.
  - `WorkspaceIndex` (`graph/invalidate.rs`) also serves open-document
    diagnostics; record which of its uses survive.

### Wave 2 (after Wave 1; needs the gate)

- [x] **`user_cache_dir` tests.** Per OS with injected environment and home; the
  relative and empty `$XDG_CACHE_HOME` rows; `None` when no home.
  - Prerequisite: Wave 1 `user_cache_dir`.
- [x] **Oracle self-check.** Assert the oracle's answers on the corpus against
  hand-written expectations for a handful of cases, so a bug in the oracle
  cannot make the store tests vacuous.
  - Prerequisite: Wave 1 oracle.
- [x] **Skill update (biscuit-file).** Add `user_cache_dir` to
  `.claude/skills/biscuit-file/` and `biscuit-file/docs/topics/` beside
  `home_dir()`.

### Validation checkpoint 1

- [x] Dependency gate recorded as met in the implementation log.
- [x] Rulings 2-18 confirmed or overridden by the author, recorded in the log.
  (Adopted as written under `yolo: true` in a headless run; the author was not
  consulted. See the Phase 1 log.)
- [x] `just test` and `just lint` pass in `biscuit-file/` and `darkmatter/`.
- [x] Oracle exists under test only, with its self-check.

## Phase 2: Wiki Universe API (R3)

Independent of the store. **Phases 2 and 3 share no files** (graph and
providers vs. a new `index_store` module) and may run concurrently.

### Wave 3 (sequential: one design, one agent)

- [ ] **`WikiUniverse` type.** Build the universe from a list of paths plus wiki
  roots (`wiki_doc_for`), separate from graph documents.
  - Prerequisite: Phase 1.
- [ ] **`build_with_roots` change.** Add the universe parameter; `assemble`
  (`graph/arena.rs`, the `wiki_docs` build near line 334) uses it instead of
  deriving docs from parsed documents. The graph records each document's
  universe slot.
  - Update every caller found by the removal inventory (`graph/invalidate.rs`,
    `providers/code_actions.rs` test, bench, tests).
  - `WorkspaceIndex` (open documents) builds its universe from its own
    documents plus, once the file list exists, the file list. Decide and record
    which one in the log; the choice must not change diagnostics for open
    documents.
- [ ] **Resolver wiring.** `resolve_file` and the ambiguity check
  (`arena.rs` near 937-981) index the universe by slot, not by
  `DocumentId`.
- [ ] **Rename wiring.** `apply_file_rename` and `safe_new_spelling`
  (`providers/rename.rs`, near 261-330) and `providers/wiki.rs` (near 290,
  333) read the universe from the graph instead of `graph.wiki_docs()`.
  - The post-rename universe replaces the old path's slot with the new path
    (as `apply_file_rename` does today).
  - Remove or retarget `WorkspaceGraph::wiki_docs()`.

### Wave 4 (after Wave 3)

- [ ] **Equivalence tests.** On the Phase 1 corpus, a graph over a **small**
  document subset with the full universe gives the same ambiguity diagnostics
  and rename spellings as the full-graph oracle.
  - Include the spec's case: an ambiguous `[[name]]` whose rival is not a
    candidate stays ambiguous.
- [ ] **No-regression run.** Existing wiki, rename, and graph L1 tests
  (`level1_wiki.rs`, `level1_graph_index.rs`, rename tests) pass unchanged in
  behavior.

### Validation checkpoint 2

- [ ] `just test` and `just lint` pass in `darkmatter/`.
- [ ] No caller passes an empty or document-derived universe except the
  oracle helper and `WorkspaceIndex`, as recorded in Wave 3.

## Phase 3: Store Core (R1, R2, R7-R12)

A new `darkmatter/dmls/src/index_store/` module. No wiring into the server yet.

### Wave 5 (parallel; three agents, disjoint files)

- [ ] **Facts model (R1, R9).** `FileFacts` (path, hash, headings, links, wiki
  links, transclusions, candidate keys), `STORE_FORMAT`, `FACTS_VERSION`.
  - `FileFacts::from_index(&DocumentIndex, hash, candidate_keys)` and
    `FileFacts::into_index(...)` per ruling 9.
  - The row invariant: facts depend only on `(path, bytes, FACTS_VERSION)`.
    No `FileResolutionContext`, configuration, or resolved path is stored.
  - The hash is raw-bytes xxHash via `biscuit-hash` (load the `biscuit-hash`
    skill), the same value as `DocumentIndex::content_hash`.
- [ ] **Candidate keys (R2, ruling 2).** Pure function from a link / wiki /
  transclusion plus the source directory to a set of keys.
  - Decode first: percent-encoding, `<...>`, `file://`, absolute, directory,
    magic-root forms; a form with no probable path falls back to the file-name
    key.
  - File-name key: case-folded, NFC, Markdown extension stripped.
  - Property test over the corpus: keys of every oracle referrer include a key
    of the target (no misses). Add the 18 S1 root-relative cases explicitly.
- [ ] **Location, sweep, permissions (R7, R8, R10).**
  - `store_path(root)`: `<user_cache_dir>/darkmatter/dmls/<hex16>-f<FACTS_VERSION>.sqlite`
    with the root key `xxhash(canonicalize_simplified(root))` as 16 hex digits.
  - `DMLS_CACHE_DIR` per ruling 11; directory `0700`, files `0600` on Unix.
  - `sweep_stale_stores(dir, max_age, now)`: 30 days by newest mtime across
    `.sqlite`, `-wal`, `-shm`; skip in use (`cfg(windows)` os error 32);
    never delete for a missing root. Injected clock from the dependency's
    `clock.rs` seam.
  - Mtime touch helper per ruling 12 (write handle on Windows).

### Wave 6 (after Wave 5)

- [ ] **`IndexStore` (R1, R11).** `open`, `upsert_batch`, `remove_missing`,
  `candidates_for(target)`, `scan_headings`, `touch`.
  - `STRICT` tables from ruling 17; `meta` table (format, `FACTS_VERSION`,
    root, `last_opened`); indexes on each key column and on `file_id`
    (S1 layout, without the suffix column).
  - WAL with the retry loop from ruling 4; `synchronous = NORMAL`;
    `busy_timeout = 5000`.
  - Batches of about 200 files in one `BEGIN IMMEDIATE`; upsert by path, skip
    when the stored hash matches; delete only paths missing on disk.
  - Open checks: root mismatch = delete and refill; stamp mismatch = corruption.
  - Every row decode failure is `StoreFailure::Corrupt`, never an empty result.
- [ ] **`FactsSource` (R12).** Trait with `StoreFacts` and
  `MemoryFacts(HashMap<PathBuf, FileFacts>)`, and `StoreFailure`.
  - Failure ladder from the R12 table: corruption = recreate once, a second
    failure or a failed delete = memory; read-only, full, busy beyond timeout,
    write error mid-fill = memory keeping computed facts.
  - One `warn` per process; memory persists for the process lifetime.
  - Provide the fault-injection seam the R12 test needs (a wrapper that fails
    the Nth operation); no permission tricks in tests.

### Wave 7 (after Wave 6; parallel tests)

- [ ] **Round trip and golden (R1, R9).** Extract, write, read back equal;
  write/read/write stability; golden snapshot of the corpus facts that fails
  on change without a `FACTS_VERSION` bump.
- [ ] **Open and fallback matrix.** Every Input Robustness Matrix row for
  `open`, the env vars, and row decode; R12 ladder with fault injection.
  - Unix `0700`/`0600` under `#[cfg(unix)]`; key stability and pinned macOS
    case behavior of `canonicalize_simplified`.
  - Root mismatch refills.
- [ ] **Sweep tests (R10).** Stale deleted, fresh kept, missing root kept, a
  store touched by a recent batch kept, Unix and Windows variants.
  - The Windows sharing-violation test runs on the native Windows leg; it
    cannot be proven on macOS. Load the `os` skill for how to reach a Windows
    host. Nothing here gates a merge (repo CI policy); failures found after
    merge are fixed forward.
- [ ] **Concurrency (R11).** Two connections writing interleaved batches
  converge; a foreign row with a mismatching hash is re-parsed. Two processes
  cold-filling one store both reach identical rows (acceptance 9). S1 did not
  check row equality.

### Validation checkpoint 3

- [ ] `just test` and `just lint` pass in `darkmatter/`.
- [ ] Smell grep over `index_store/` recorded in the log.
- [ ] Candidate no-miss property holds on the whole corpus.

## Phase 4: Fill, Ready, and Freshness (R4-R6)

Wire the store into the dependency's file list and R7 state machine.
Needs Phase 3; Phase 2 is not required yet.

### Wave 8 (parallel)

- [ ] **Revalidation pass (R4).** On first need: open every root's store, sweep,
  read and hash every Markdown file in the file list in parallel (ruling 18),
  re-parse only files whose hash differs or has no row, upsert in batches,
  delete rows for missing paths.
  - No size/mtime fast path.
  - Unreadable listed file = state failed (the dependency's definition).
  - Cancellation between batches and between parses; reconcile changes that
    arrive during the pass (the dependency's R7 behaviors carry over).
  - Work-done progress and the shared single build carry over.
- [ ] **Parse counter seam.** Use the dependency's `work_counters.rs`; the
  tests for acceptance 1 and 2 count parses, not time.
- [ ] **Open-buffer override (R5).** A function returning the facts overlay for
  every open buffer (saved or not); a buffer never writes a row.

### Wave 9 (after Wave 8)

- [ ] **Ready state.** "Ready" = every Markdown file in the list has a row whose
  hash matches bytes read in this process, and no row exists for a path missing
  on disk. Duplicate paths and symlinks follow the dependency's R5 policy.
- [ ] **Live updates (R6).**
  - Before ready: events touch only the file list.
  - After ready, client watching: re-read, re-hash, upsert or delete per event.
  - After ready, fallback watch mode: a save re-hashes only the saved file
    plus files with pending events (amends the dependency's R5).
  - Before each heading or file rename in fallback mode: a full re-hash pass.
  - Every rename re-reads and re-hashes each referrer it will edit; a mismatch
    re-parses that file first.
- [ ] **Failure to memory mid-process.** A store failure after ready switches
  `FactsSource` to `MemoryFacts` without losing computed facts and without
  failing the request in flight.

### Wave 10 (after Wave 9; parallel tests)

- [ ] **R4 tests.** Restart with unchanged files parses zero (acceptance 1);
  one change parses one (2); deleted file's row removed; unreadable file fails;
  empty and zero-byte files are valid rows.
- [ ] **R5 tests.** Unsaved and saved-but-differing buffers override rows; no
  buffer writes a row.
- [ ] **R6 tests.** A watched event updates one row; before ready only the
  list changes; a fallback save re-hashes only the saved file plus events;
  an unwatched disk change that adds a link then rename edits the new
  referrer (acceptance 7); rename edits the new bytes of a referrer changed
  after ready.

### Validation checkpoint 4

- [ ] `just test` and `just lint` pass in `darkmatter/`.
- [ ] Acceptance 1, 2, 7, and 9 have passing tests.

## Phase 5: Requests Over the Store (R3, R8)

Needs Phases 2 and 4.

### Wave 11 (sequential: one shared helper)

- [ ] **Temporary-graph builder.** Given a target, collect `candidates_for` over
  every root, union and de-duplicate by canonical path (ruling 15), load facts
  from the `FactsSource`, add every open buffer, add the target, and build a
  `WorkspaceGraph` with the wiki universe from the file list and wiki roots.
  - Drop the graph after the request; keep only the dependency's R6 lazy
    target cache across requests.
  - Reuse `RepositoryContexts` across requests, since S1 showed warm contexts
    cut a rebuild from about 100 ms to 5-50 ms.
  - Run store and parse work off the request thread per the existing
    router pattern, under the 2-second-class timeouts the related request
    timeout work defines (read `2026-10-11-dmls-request-timeouts` for the
    interaction; do not change it here).

### Wave 12 (after Wave 11; parallel, one agent per provider, disjoint files)

- [ ] **References and DSL transcluded-by.** `providers/references.rs`,
  `providers/dsl.rs`.
- [ ] **Wiki references.** `providers/wiki.rs`.
- [ ] **Heading rename and file rename.** `providers/rename.rs`; file rename
  uses the post-rename universe; a directory rename still makes no edits.
- [ ] **Workspace symbols.** `providers/symbols.rs`: scan stored headings of
  every root, document title rule unchanged (first level-1 heading, else first
  heading, else file name), fuzzy match, de-duplicate nested roots. No graph.

### Wave 13 (after Wave 12; parallel tests)

- [ ] **Oracle equivalence (acceptance 4).** For each of the six request kinds
  on the corpus, store-backed result equals the oracle's, including wiki
  ambiguity diagnostics and file-rename spellings.
- [ ] **No-miss (acceptance 5).** For every target in the corpus, candidates
  contain every oracle referrer.
- [ ] **After restart (acceptance 3).** Heading and file rename edit a referrer
  that was never open and never referenced by an open document.
- [ ] **Two roots (acceptance 6).** A file rename in one root edits a referrer
  in the other, after a restart; a nested root yields no duplicates.
- [ ] **Fallback to memory (acceptance 8).** With the directory read-only,
  corrupt, locked, or `off`, all four requests answer correctly from memory
  with one `warn`; `off` creates no store file. One stdio wiring test spawns
  `dmls` once with `off` and once with a corrupt store.
  - Tests that spawn `dmls` must not gain window focus (none open windows).

### Validation checkpoint 5

- [ ] `just test` and `just lint` pass in `darkmatter/`.
- [ ] Acceptance 3-8 have passing tests.
- [ ] Peak memory and latency sample taken on this repository (full numbers in
  Phase 7).

## Phase 6: Removal, Logs, and Bench (R13, R14, acceptance 12)

### Wave 14 (parallel)

- [ ] **R13 log lines.** Emit the six lines of the spec table with the
  dependency's R8 style (`workspace` field); one `warn` per process for the
  fallback; never log file contents or heading text.
  - Prerequisite: Phase 4. `trigger` takes the dependency's R8 values.
- [ ] **`--bench-index` (R14).** Rewrite `dmls/src/bench.rs` to report store
  fill and revalidation, each cold (no store) and warm (store present,
  unchanged), in the summary and `--json` forms. Update the CLI help text in
  `main.rs`.
- [ ] **Delete the production full build.** Remove the code the Wave 1 inventory
  lists (full parse-everything index, workspace-wide resident graph for these
  requests, the R8 `full index built ...` line). Keep `WorkspaceIndex` only for
  the open-document use recorded in the inventory.
  - Prerequisite: Phase 5 equivalence tests are green (the oracle is now the
    only full-graph build, and lives in tests).

### Wave 15 (after Wave 14; parallel tests)

- [ ] **R13 test.** A captured `tracing` subscriber shows each line and field
  once, and the fallback `warn` once.
- [ ] **R14 test.** `bench_index` on a small fixture reports cold and warm fill
  and revalidation sections (acceptance 10).
- [ ] **Removal guard.** A source-scan test or `rg` check in the log that no
  production module references the removed functions (acceptance 12).

### Validation checkpoint 6

- [ ] `just test` and `just lint` pass in `darkmatter/`.
- [ ] Downstream consumers: find users of any removed or changed public item
  with Sniff / GitNexus (the `dmls` library surface, `bench`). Do not run
  workspace-wide Cargo gates.

## Phase 7: Documentation, Skills, and Final Measurement

### Wave 16 (parallel; documentation agents on disjoint files)

- [ ] **`darkmatter/docs/lsp/indexing.md`.** Extend with the store: what it
  holds and why it is only a cache; the sequence diagram; ready and
  revalidation; open-buffer override; rename freshness per watch mode;
  multiple roots; location; `DMLS_CACHE_DIR` and `off`; sweep; multiple
  processes; fallback ladder; network filesystem limit; "always safe to
  delete"; the R13 lines; `--bench-index`. Remove the in-memory full-build
  description and state the corrected C-over-A reason (ruling 7).
  - Audience: a developer new to this repository; an example per rule.
- [ ] **`configuration.md` and `architecture.md`.** `DMLS_CACHE_DIR` and why it
  is not a `.dmls.toml` key; the external wiki universe of `build_with_roots`.
- [ ] **Dependencies and skills.** Confirm `docs/dependencies.md` entries
  (Wave 1); update `.claude/skills/darkmatter/dmls.md` (store, `FactsSource`,
  removed full build) and `remote-cache.md` (its no-fallback rule does not
  apply to the store); `biscuit-file` skill done in Wave 2.
  - No `docs/` page or skill names this spec or the dependency fix.
- [ ] **Docs drift pass.** Re-read every comment and doc touched by a changed
  symbol (`build_with_roots`, `wiki_docs`, the removed functions) and fix
  drift; assume the code is correct.

### Wave 17 (after Wave 16)

- [ ] **Acceptance 11 sample.** One warm restart, first references request on
  this repository: record latency (request to answer, and including the file
  list walk) and added peak RSS, against ruling 6's 500 ms / 1 s / 64 MiB.
  A single quick sample on this host, not a spike. If the host is loaded,
  note the load average in the log.
- [ ] **Implementation log.** Record: gate result, rulings as confirmed,
  every departure from the spec, smell-grep output, the exact
  `storage-strategy` line for the author (ruling 16), and the acceptance
  checklist with the test or measurement behind each of the 12 items.

### Validation checkpoint 7 (final)

- [ ] `just test` and `just lint` pass in `darkmatter/` and `biscuit-file/`.
- [ ] `just test-l2` run in `darkmatter/` if any L2 test was touched.
- [ ] All twelve acceptance criteria have a recorded test or measurement.
- [ ] Windows evidence: R10/R12 sharing-violation test runs on the native
  Windows leg (nightly or `just ci-seed`); result recorded or marked pending
  with the reason.
- [ ] Implementation complete, ready for review. The author moves the spec to
  `_completed`.

## Requirement-to-Test Map

| Requirement | Phase / Wave | Test |
| --- | --- | --- |
| R1 | 3 / 7 | round trip; repeated write-read-write |
| R1, R9 | 3 / 7 | golden snapshot vs `FACTS_VERSION` |
| R2 | 3 / 5, 5 / 13 | key no-miss property; candidate superset of oracle referrers |
| R3 | 2 / 4, 5 / 13 | wiki universe equivalence; six request kinds equal oracle |
| R4 | 4 / 10 | zero re-parse on restart; one change one parse |
| R5 | 4 / 10 | buffers override rows; no buffer row |
| R6 | 4 / 10 | live-update modes; fallback rename freshness |
| R7 | 1 / 2, 3 / 7 | `user_cache_dir`; `DMLS_CACHE_DIR`; modes |
| R8 | 3 / 7, 5 / 13 | key stability, root mismatch, two roots |
| R10 | 3 / 7 | sweep (Unix and Windows legs) |
| R11 | 3 / 7 | interleaved writers; two processes |
| R12 | 3 / 7, 5 / 13 | fault-injected ladder; four requests from memory |
| R13 | 6 / 15 | captured subscriber |
| R14 | 6 / 15 | `bench_index` fixture |
