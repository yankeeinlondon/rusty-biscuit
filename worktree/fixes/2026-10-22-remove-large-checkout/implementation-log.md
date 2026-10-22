---
spec: /Volumes/coding/wt/rusty-biscuit/fix-remove-large-checkout/worktree/fixes/2026-10-22-remove-large-checkout/spec.md
plan: worktree/fixes/2026-10-22-remove-large-checkout/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 was survey-only; read "## Phase 1" of this log before starting. Three corrections to the plan's rulings matter for Phase 2:
    (1) Sniff's PathUsageReport has no version-dispatching loader (plain derive, deny_unknown_fields). Ruling 2 needs a version-aware Deserialize: absent provenance + schema 1 -> Unknown; absent + schema 2 -> error; explicit null -> error (the plan's robustness matrix overrides the ruling's "optional-field convention" wording); no #[serde(default)].
    (2) Sniff has no storage seam. hardware/storage.rs spawns diskutil on macOS and is a device inventory, so do not reuse it. Add a new path-based module under sniff/lib/src/filesystem/ with its own platform trait: libc statvfs f_bavail, and GetDiskFreeSpaceExW (Win32_Storage_FileSystem is already enabled).
    (3) TreeIndex stores absolute paths today, but matching is identity-keyed. A supplied index can store relative paths and join onto root.resolved only for matched paths and Windows probe entries, which avoids a per-query whole-tree conversion. QUERY_TREE_WALKS / QUERY_TREE_IDENTITY_READS counters already exist for the zero-walk test.
    For Phase 3: reopen() has three callers (resume.rs:247 recovered inventory, :403 fresh inventory, :429 empty inventory); archive_journal() decides "identical archive" while writing, so split out a non-writing predicate for the space check.
---

# Implementation Log for 2026-10-22-remove-large-checkout (5 phases)

## Phase 1

Phase 1 is read-only: a code survey and a ruling check. No source, doc, or
skill file changed.

### Survey: Sniff

- **`TreeIndex`** (`sniff/lib/src/filesystem/query/tree.rs`) is `pub(crate)`
  and `Default`. It holds `paths: HashMap<FileIdentity, Vec<PathBuf>>`, which
  keeps every hard-link alias, plus `entries: Vec<ProbeEntry>` under
  `#[cfg(any(windows, test))]`. A `ProbeEntry` is `{path, identity, directory}`
  and is pushed only for regular files and directories, so links are never
  probed. The paths are **absolute**: the root's `resolved` path and the
  walker's joined paths. The root itself is inserted as an index entry and a
  probe entry.
- **The walk.** `tree::collect(&root, recursive, budget)` is called once, from
  `query::run` (`mod.rs:141`), straight after `root::resolve`. It increments
  `QUERY_TREE_WALKS` once and `QUERY_TREE_IDENTITY_READS` once per entry
  (`performance/counters.rs:64,68`), so the "zero walks" test can use the
  counters that already exist. It returns `TreeCollection { index, coverage }`,
  where `coverage` is the `TreeIdentity` record. That record is the natural
  place for provenance and for a partial supplied index. On Windows the walk
  skips non-surrogate reparse directories, such as cloud placeholders.
- **The matcher** (`matching.rs`) looks up `index.paths(&identity)` and copies
  the paths it finds into `matched_paths`. Path text is used only as a
  pre-filter (`root.spellings()`/`may_contain`) before an identity lookup. So a
  supplied index can keep **relative** paths and join them onto
  `root.resolved` only for objects that match. No whole-tree conversion is
  needed per query, which keeps Ruling 6's "no per-query conversion" true.
  Other consumers of the index are `linux.rs:356` (`identities()` for the
  inotify inode set), `win32.rs:290`, and `win32_paths.rs:87` (iterate
  `entries`, opening each path). The Windows probe opens an absolute path, so
  it has to join per entry, and it does that work for every entry anyway.
- **`PathUsageOptions`** (`options.rs`) has only `recursive: bool` and
  `deadline: Duration`. `PathUsageError::InvalidOption { message }` already
  exists for rejected options.
- **Report schema.** `report::SCHEMA_VERSION = 1`. `PathUsageReport` is a plain
  `#[derive(Deserialize)] #[serde(deny_unknown_fields)]` struct.
  `schema_version` is an ordinary `u32` field, and **no version dispatch exists
  on load**. Optional fields use `deserialize_with = "nullable"`, which allows
  explicit null and rejects absence.
- **Storage seams: none.** `hardware/storage.rs` is a device inventory:
  `getmntinfo`+`statvfs` plus a `diskutil` subprocess on macOS, `/proc/mounts`
  plus `statvfs` on Linux, and `sysinfo::Disks` on Windows. It has no
  path-based query and no injection seam, and Ruling 5 forbids both a
  subprocess and the device inventory. `libc` is already a dependency, and the
  `windows` crate already enables `Win32_Storage_FileSystem`, which provides
  `GetDiskFreeSpaceExW`. No new crate is needed.

### Survey: Worktree

- **Batch planning** (`remove/observation/deletion.rs`, `Engine::run` plus
  `levels`). The engine partitions inventory entries by
  `path.starts_with(".git")`, a component-wise test, so content comes first and
  `.git` last. For each partition, `levels()` builds a
  `BTreeMap<depth, BTreeMap<parent PathBuf, Vec<&InventoryEntry>>>`, which is
  iterated deepest first (`.rev()`) and then in parent order. Each parent's
  children keep inventory order (byte order) and are split by
  `children.chunks(MAX_BATCH_ENTRIES)` (256). Extracting a planner means
  moving the partition, `levels`, and `chunks` into one function that yields
  `(batch_number, &[&InventoryEntry])`. Drains, interrupts, and stops stay in
  the engine. The engine plans every entry exactly once and stops early on any
  stop, so the bound is an upper bound.
- **Batch numbering.** The engine's `self.batches` and the writer's
  `last_batch + 1` both start at 1 per attempt. `reopen` builds a fresh writer
  through `after_header`, so `last_batch` is 0 again. The planner's numbering
  from 1 therefore matches the journal on every attempt.
- **Reservation** (`journal.rs:386`). `begin_batch` refuses when
  `written + len(intent) + len(results_line(batch, entries, [ExternallyAbsent; n])) > limit`.
  `externally_absent` is the longest state name (`deleted`, `kept`), so the
  reserve is the worst case. `line()` appends `\n`. `start` resets `written`
  to 0 after the header and inventory, so the limit counts batch lines only.
  That is exactly Ruling 3's Σ. The sizer should reuse `line`/`results_line`,
  or the same `Line` serialization into a counting writer, so the two
  computations cannot drift.
- **Limit selection.** `DEFAULT_JOURNAL_LIMIT` (64 MiB, `journal.rs:50`) is
  passed in four places in `recovery/store.rs`: `begin` (`start` at 156,
  `after_header` at 172) and `reopen` (`start` at 251, `after_header` at 262).
  Its other uses are the journal unit tests (`journal.rs:531–653`).
- **`reopen` call sites.** `lifecycle/resume.rs:403` passes the fresh surviving
  inventory captured at `resume.rs:459`. **`resume.rs:247` passes
  `recovered.inventory`**, the replayed earlier inventory, and **`resume.rs:429`
  passes `DeletionInventory::empty(..)`**. All three get an inventory-derived
  limit; for the empty inventory the bound is 0.
- **Archive on retry.** `archive_journal` (`store.rs:427`) writes
  `journal-<n>.jsonl` unless the newest archive is byte-identical to the
  current journal. The new journal is staged with `temp_beside` and
  `publish`ed (a rename) over `journal.jsonl`, so the old journal's bytes stay
  allocated until the rename. Ruling 4's "previous journal's full length when
  an archive copy is needed, zero when identical" matches this. **The decision
  happens inside a function that also writes**, so the pre-move space check
  needs a non-writing "would archive" predicate split out of `archive_journal`.
- **Exit 4.** `Refusal::Store(StoreError::Storage{..} | Io | JournalWrite)` maps
  to `ExitClass::Blocked` (exit 4) in `lifecycle.rs:284`. The space refusal can
  be a new `StoreError` variant mapped to `Blocked` there.
- **Usage handoff order.** `lifecycle.rs:413` captures the inventory before
  `check_usage` at 422, and `resume.rs:459` before 466. Both attempts therefore
  already hold the inventory when the usage query runs. `UsageSource::query(&self, path)`
  (`remove/usage.rs:37`) takes only a path, and `SniffUsage` calls
  `query_path_usage(path, &PathUsageOptions::default())`. The Windows Search
  recheck calls `source.query` a second time from `check_usage`.
  `lifecycle.rs:1101` captures an inventory with `&[]` for post-stop
  diagnostics; that path must keep a fresh walk.
- **Inventory shape.** `InventoryEntry { path: NativeString, identity: FileIdentity, shape }`
  with `Shape::{File, Directory, Link}`, and the root identity held
  separately (`DeletionInventory::root()`). That maps directly onto
  `(relative path, kind, identity)` with no caller types. The root is not an
  entry. Paths are `NativeString`, which is lossless.
- **Report version.** `outcome::REPORT_VERSION = 5`. Loading does not check the
  version, and a v4 fixture test already exists (`outcome.rs:456`). The report
  embeds `sniff::PathUsageReport` (`outcome.rs:154`), so a v4 or v5 worktree
  report carries Sniff schema-1 reports. That makes Ruling 2's old-version
  loading a requirement of worktree report compatibility as well.
- **`LimitReached` text.** It is the `thiserror` text "the deletion journal
  reached its {limit}-byte limit" (`journal.rs`). No CLI code matches it
  specifically, so Ruling 7's wording goes in the stop-message rendering.

### Rulings

All seven are adopted, with these amendments:

- **Ruling 1**: adopted. Sniff `SCHEMA_VERSION` 1 → 2; worktree
  `REPORT_VERSION` 5 → 6.
- **Ruling 2**: adopted, with one correction. The ruling says explicit null
  "follows the existing optional-field convention". The plan's own robustness
  matrix says explicit `null` is an **error**, because provenance is not
  optional. The matrix wins: null → error. Old-version loading needs new code,
  because the loader does no version dispatch. Phase 2 needs a
  version-aware `Deserialize` for `PathUsageReport`, for example through a
  private raw struct, so that absent+v1 → `Unknown`, absent+v2 → error,
  null → error, and unknown string → error. `#[serde(default)]` on
  provenance does not meet this and is not allowed.
- **Ruling 3**: adopted with no gap. The reservation, the `\n`, the per-attempt
  numbering, and the root-is-not-an-entry rule all match the code.
- **Ruling 4**: adopted. The archive decision has to be split out of
  `archive_journal` so it can be evaluated without writing. All three `reopen`
  call sites are in scope.
- **Ruling 5**: adopted, with a correction. Sniff has **no** existing storage
  seam to inject through, and `hardware::storage` must not be reused because it
  spawns a subprocess and inventories devices. Phase 2 adds a new module under
  `sniff/lib/src/filesystem/` with its own platform trait. `statvfs` comes from
  `libc` and `GetDiskFreeSpaceExW` from the already-enabled
  `Win32_Storage_FileSystem` feature.
- **Ruling 6**: adopted. Implementation note for Phase 2: store supplied
  entries relative and join them onto `root.resolved` only on output (matched
  paths) and per Windows probe. Insert the root identity, or handle it, at
  query time. `TreeCollection.coverage` carries provenance and partial status.
- **Ruling 7**: adopted.
- **Ruling 8**: not scheduled, as the plan states.

### Gates

No source file changed in this phase. Scope check run from the repository root
after all edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `"packages": []`, `"change_class": "documentation"`. Under the
skip rule, `just test` and `just lint` were **skipped** for this phase: no
package reads any file this phase changed (plan, spec, and this log), and
`source_files_during_phase_1` is empty. No cross-OS check was needed, since
nothing was compiled. The worktree skill was not updated, because nothing
about the area's architecture or workflow changed.

### Requirement-to-test mapping

This phase changed no behavior, so it has no tests. The survey's test hooks
for later phases are the counters `QUERY_TREE_WALKS` and
`QUERY_TREE_IDENTITY_READS` (supplied index, zero walks), the existing
`VERSION_4_REPORT` fixture in `outcome.rs` (report compatibility), and the
existing small-limit `LimitReached` tests in `journal.rs`, which keep passing
an explicit limit.
