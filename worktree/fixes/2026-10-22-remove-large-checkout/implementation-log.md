---
spec: /Volumes/coding/wt/rusty-biscuit/fix-remove-large-checkout/worktree/fixes/2026-10-22-remove-large-checkout/spec.md
plan: worktree/fixes/2026-10-22-remove-large-checkout/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - sniff
    - sniff-cli
    - worktree
    - worktree-cli
source_files_during_phase_2:
    - sniff/lib/src/filesystem/space.rs
    - sniff/lib/src/filesystem/mod.rs
    - sniff/lib/src/performance/counters.rs
    - sniff/lib/src/filesystem/query/tree.rs
    - sniff/lib/src/filesystem/query/report.rs
    - sniff/lib/src/filesystem/query/options.rs
    - sniff/lib/src/filesystem/query/mod.rs
    - sniff/lib/src/filesystem/query/matching.rs
    - sniff/lib/src/filesystem/query/win32.rs
    - sniff/lib/src/filesystem/query/win32_paths.rs
    - sniff/lib/src/filesystem/query/tests.rs
    - sniff/lib/src/filesystem/git/changed_paths_tests.rs
    - sniff/cli/src/output/filesystem/query.rs
    - sniff/cli/tests/l1/filesystem_query.rs
docs_updated_during_phase_2:
    - sniff/docs/topics/filesystem-query.md
    - sniff/docs/cli/filesystem_query.md
docs_created_during_phase_2:
    - sniff/docs/topics/available-space.md
skills_files_updated_during_phase_2:
    - .claude/skills/sniff/SKILL.md
human_review: false
message_to_agent: |-
    Phase 3 is in "## Phase 3" of this log. Facts Phase 4 (CLI stop message) depends on:
    (1) Ruling 7 is already applied in the library: JournalWriteError::LimitReached now displays "wt reached an internal limit of its deletion journal; a retry continues" (no byte count). The stop message prints that Display through Stop::Journal; Phase 4 only needs a test that the rendered stop message shows it.
    (2) The space refusal is Refusal::Store(StoreError::InsufficientSpace { path, needed, available }) or StoreError::SpaceUnknown { path, needed, reason }, both exit 4 (ExitClass::Blocked). Their Display already names the store path, the bytes needed, and the bytes available (or the reason). The CLI's Environment passes space: &NativeSpace. For the spawned-CLI wiring case there is no CLI seam to fake a low reading; either add a test-only environment hook to the CLI or prove the exit-4 rendering in-process with refusal_markup-style rendering of a Refusal built from those variants.
    (3) UsageSource::query and check_usage take an Option<&Arc<TreeIndex>>. Preflight runs carry index_provenance "supplied"; diagnostic runs after a stop are walked ("walked"). Phase 4's "identical notes" comparison should compare rendered coverage notes, which may now differ only by the tree_identity coverage reason ("supplied" says the query did not read the descendants), so a preflight note and a diagnostic note can legitimately differ on that line.
    (4) The 64 MiB regression test uses control-character names (six-byte JSON escapes) to stay near 1.4 s in debug builds; debug serde_json escapes at about 50 MB/s, so do not grow it.
source_files_during_phase_3:
    - worktree/lib/src/remove/observation/deletion.rs
    - worktree/lib/src/remove/observation/deletion/plan.rs
    - worktree/lib/src/remove/observation/deletion/tests.rs
    - worktree/lib/src/remove/observation/inventory.rs
    - worktree/lib/src/remove/observation/fsevents/tests.rs
    - worktree/lib/src/remove/observation/inotify/tests.rs
    - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/recovery/space.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/usage.rs
    - worktree/lib/src/remove/usage/tests.rs
    - worktree/lib/src/remove/lifecycle.rs
    - worktree/lib/src/remove/lifecycle/resume.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/lifecycle/tests/large_checkout.rs
    - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
    - worktree/lib/src/remove/lifecycle/tests/startup_warning.rs
    - worktree/lib/src/remove/outcome.rs
    - worktree/lib/src/remove/quarantine/unregister/tests.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/remove_usage_refusal.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/lifecycle.md
    - .claude/skills/worktree/remove.md
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

## Phase 2

All work is in `sniff/` (library `sniff`, CLI `sniff-cli`). Both waves were
done in this session without subagents: Wave 1 was small, and a foreground
subagent would only have blocked the Wave 2 work.

### Wave 1: available-space query

- New module `sniff/lib/src/filesystem/space.rs`:
  `available_space(path) -> Result<AvailableSpace, SpaceError>`.
  `AvailableSpace { requested, measured, available_bytes }`;
  `SpaceError::{Resolve, Reading}`, each naming the requested path (and
  `measured` for `Reading`).
- Resolution: canonicalize the path (`biscuit_file::canonicalize_simplified`,
  so Windows never reports `\\?\`), climbing to the parent **only on
  `NotFound`**. Any other failure (permission denied, symlink loop) is a
  `Resolve` error, because climbing further could measure a different
  filesystem. Aliases are followed by canonicalization, never by
  string-prefix matching against a mount list.
- Native readings: Unix `statvfs` `f_bavail × f_frsize` (falling back to
  `f_bsize` when `f_frsize` is 0); Windows `GetDiskFreeSpaceExW`
  caller-available bytes on the directory (a file's parent), with a trailing
  separator for UNC roots. No new crate or feature was needed.
- Seam: the crate-private `SpaceSource` trait (`canonicalize`,
  `available_bytes`). Worktree's Phase 3 seam goes on its own store, as
  planned. New counter `filesystem.space.readings` (`FS_SPACE_READINGS`).
- **Departure from the Phase 1 note:** the module is
  `sniff::filesystem::space`, not `query`, because it is not a usage query.

### Wave 2: supplied tree index and provenance

- **One representation.** `TreeIndex` (`query/tree.rs`) is now public and
  stores **root-relative** paths, with the root as the empty path, for both
  the walk and a supplied index. `paths(identity, root)` joins only the matched
  paths onto `root.resolved`. Windows probe entries are stored relative and
  joined once per scan (`TreeIndex::probe_entries`, called in
  `win32.rs::inspect_paths`, then passed to `win32_paths::scan` and the
  lifetime recheck). That costs the same as the per-entry open the scan
  already does. Linux's `identities()` is unchanged.
- **Builder.** `TreeIndex::builder(root_identity)` →
  `.entry(relative, TreeEntryKind, FileIdentity)` / `.partial(reason)` →
  `.build() -> Result<TreeIndex, TreeIndexError>`. `TreeEntryKind` is
  `File | Directory | Link | Other`; `Other` exists because Sniff's own walk
  indexes sockets and FIFOs. Validation names the offending path:
  `AbsolutePath`, `ParentTraversal`, `NotNormalized` (empty path or a `.`
  component), `Duplicate` (component-equal spellings such as `a/b` and
  `a//b/`), and `BelowLink`. Duplicate and below-link checks sort the
  indices once: component order puts a path's descendants right after it.
  Stored paths are normalized by `components()`, which keeps native names.
  Only `File` and `Directory` entries become probes.
- **Options.** `PathUsageOptions::with_tree_index(Arc<TreeIndex>)` and
  `tree_index()`. `PartialEq` is now manual and compares the index with
  `Arc::ptr_eq`, never entry by entry. `run()` rejects `target_only` before
  resolving the root. After resolving, `tree::supplied` rejects a file root and
  a root identity mismatch as `InvalidOption`, and nothing is enumerated.
  Root resolution and the final recheck still run.
- **Coverage.** A supplied index yields a `tree_identity` coverage record with
  status `complete`, or `partial` with a new `LimitationKind::IncompleteTreeIndex`
  carrying the caller's reason. `attempted`/`succeeded` are `null`, and `reason`
  says the query did not read the descendants. The existing prerequisite-gap
  logic then marks dependent mechanisms `partial`.
- **Report.** `SCHEMA_VERSION` 1 → 2, with a new required field
  `index_provenance: walked | supplied | unknown`. `PathUsageReport`
  deserializes through a private `RawReport` (`#[serde(try_from)]`, still
  `deny_unknown_fields`). The field uses `#[serde(default, deserialize_with =
  "present")]` **only to tell absent from null**: `present` makes explicit
  `null` an error. Version 1 + absent → `Unknown`. Version 1 + `"unknown"` →
  `Unknown`. Version 1 + `walked`/`supplied` → error. Version ≥2 + absent →
  error naming `index_provenance`.
- **Decision, not in the plan:** a schema-1 report may carry
  `"unknown"`. Without that, an old report that `worktree` loads and writes
  back into a recovery report would fail to load again: it keeps
  `schema_version: 1` and now serializes `unknown`. A test covers the
  read/write/read round trip.
- **CLI.** Text output adds a `Tree index:` line only for `supplied`/`unknown`,
  so existing live and walked output is unchanged. JSON carries the new field.

### Input robustness matrix (report loader, index builder)

Default outcome: an error naming the field or entry. The exceptions, and the
test that covers each:

| Field / shape | Outcome | Test |
| --- | --- | --- |
| `index_provenance` absent, schema 2 | error naming the field | `old_reports_load_with_unknown_provenance_and_round_trip`, and the existing matrix row `/index_provenance` |
| `index_provenance` null / `123` / `""` / duplicate key | error | existing matrix `the_report_reader_rejects_every_malformed_shape_of_its_load_bearing_fields` (row added) |
| `index_provenance` absent, schema 1 | `Unknown` | `old_reports_load_with_unknown_provenance_and_round_trip` |
| `index_provenance` `"unknown"`, schema 1 | `Unknown` (round trip) | same |
| `index_provenance` `walked`/`supplied`/null, schema 1 | error | same |
| trailing content, unknown field | error (shared loader) | existing matrix |
| index entries `[]` | valid: the root alone | `an_empty_index_matches_only_the_root` |
| entry absolute / `..` / `./` / empty / duplicate / below a link | error naming the entry | `the_index_builder_rejects_each_invalid_entry_by_name` |

CLI wiring (spawned, one control plus one invalid case):
`a_version_1_replay_loads_with_unknown_provenance_and_says_so` and
`a_current_replay_without_provenance_is_an_error_naming_the_field`. The live
test `a_live_json_perf_query_is_one_object_and_does_no_inventory` now
asserts `schema_version` 2 and `walked`.

Smell grep (`serde(default)`, `unwrap_or_default`, `.ok()`) over `report.rs`,
`tree.rs`, `options.rs`, and `space.rs`: the only hit is the intentional
`default` + `present` pair described above.

### Requirement-to-test mapping

All tests are L1 and in-process, except the three spawned CLI tests.
Fidelity is F2 at the crate boundary: a real temporary filesystem with the
fake usage backend for queries, and a scripted `SpaceSource` plus real native
readings for space.

| Requirement | Test(s) |
| --- | --- |
| Space: path → its own filesystem; nested mount | `an_existing_path_is_measured_on_its_own_filesystem`, `a_path_under_a_nested_mount_reads_that_mount_not_its_parent_volume` |
| Space: store on a different filesystem from the checkout | `a_store_on_another_filesystem_from_the_checkout_reads_its_own_filesystem` |
| Space: alias ancestors (scripted, plus a real Unix symlink) | `an_aliased_ancestor_reads_the_filesystem_it_points_to_not_the_one_its_text_names`, `a_native_reading_through_a_symlinked_ancestor_measures_the_link_target` |
| Space: nonexistent leaf → nearest ancestor (absolute and relative) | `a_missing_leaf_is_measured_at_its_nearest_existing_ancestor`, `a_relative_path_with_no_existing_component_is_measured_at_the_current_directory` |
| Space: error mapping | `a_resolution_failure_other_than_not_found_does_not_climb_to_a_higher_ancestor`, `a_path_with_no_existing_ancestor_is_a_resolve_error`, `a_failed_filesystem_query_is_a_reading_error_naming_both_paths`, `a_zero_byte_reading_is_reported_as_zero_not_an_error` |
| Space: no subprocess, no device inventory, only its own work | `a_native_reading_spawns_nothing_and_does_only_its_own_work` (exact counter-set assertion; `PROC_SPAWNS` = 0), `a_native_reading_of_a_file_measures_the_file_location` |
| Index: zero walks, zero identity reads, root still verified, processes reached | `a_supplied_index_replaces_the_walk_but_the_root_is_still_verified` |
| Index: walked vs supplied give the same holders and hard-link aliases | `walked_and_supplied_indexes_find_the_same_holders_and_hard_link_aliases` |
| Index: wrong root, `target_only`, file root | `an_index_for_a_different_root_is_an_invalid_option_and_nothing_runs`, `an_index_with_target_only_or_for_a_file_is_an_invalid_option` |
| Index: partial coverage | `a_partial_index_is_reported_as_the_callers_partial_coverage` |
| Index: links matched as themselves, never probed | `a_supplied_link_is_matched_as_itself_and_never_probed_as_its_target` (Unix) |
| Index: Windows probe entries match the walk's; no links or special files | `walked_and_supplied_indexes_hand_the_holder_scan_the_same_probe_entries` (all hosts), plus the existing `win32_paths` ordering test, adapted |
| Index: unsupported mechanisms stay unsupported | asserted in the first index test (`fsevents` unsupported) |
| Index: budget exhaustion during inspection stays visible | `a_budget_expiring_during_inspection_stays_visible_with_a_supplied_index` |
| Index: native names, hard-link aliases, normalization | `the_index_keeps_native_names_without_lossy_conversion` (Unix), `the_index_keeps_every_hard_link_alias_and_sorts_them_under_the_root`, controls in the builder test |
| Provenance and compatibility | the matrix table above |

### Gates and other runs

- Toolchain trap: the first build failed to link `-lgit2`, because Homebrew
  had moved libgit2 to `1.9.7_2`. **kache replayed the stale `libgit2-sys`
  build-script output** that still pointed at `Cellar/libgit2/1.9.7`. Neither
  `cargo clean -p libgit2-sys` nor deleting the build directory fixed it.
  One `KACHE_DISABLED=1 cargo nextest run …` re-ran the build script, and
  normal builds have worked since.
- `cd sniff && just test`: 3491 passed, 33 skipped.
- `cd sniff && just lint`: clean. `just check-tier-coverage sniff`: 0
  stranded.
- Dependent packages: only `worktree` uses `filesystem::query`. `cd worktree
  && just test`: 1594 passed, 15 skipped. `just lint`: clean. Worktree test
  JSON fixtures use `schema_version: 1` without provenance and load as
  `unknown`, so no worktree change was needed. No other package
  constructs `PathUsageReport` or `PathUsageOptions` (checked with grep), so
  no other dependent was run.
- `just cross-check sniff --os windows`, **attempt 1**: 2445/2446 passed. Every
  new query and space test passed on Windows. The one failure was the
  existing `filesystem::git::changed_paths_tests::staged_unstaged_both_and_untracked_paths_union_once`,
  which reported `unstaged.txt` as missing. That file was rewritten with
  content of the same size (`u1\n` → `u2\n`) right after the index was
  written. The likely cause is a stat check that cannot see an edit inside
  the index's timestamp granularity. The cause is **not proven**; it may also
  be a plain flake. Test-only fix: the unstaged edit now changes the file size.
  **Attempt 2**: 2446/2446 passed. This phase's code did not cause the
  failure. No production Git code changed, and the `os` skill was not updated
  because the cause is unconfirmed.
- Linux, WSL2: not cross-checked locally. The Unix `statvfs` path compiles and
  runs on macOS. CI covers Linux, and the nightly covers WSL2.

### Validation checkpoint

`just test` and `just lint` pass in `sniff/`. No old-report path reads as
`walked`: an absent field at schema 1 reads as `unknown`, and at schema 2 it is
an error.

## Phase 3

All work is in the `worktree` library, plus two mechanical compile fixes in
`worktree-cli` (the `Environment` gained a field, and a test stub's
`UsageSource::query` gained a parameter). It was done in this session without
subagents: the three waves edit the same few files (`lifecycle.rs`,
`resume.rs`, `store.rs`, and the lifecycle tests), so splitting them would
have produced edit conflicts and half-compiling trees.

### Wave 1: shared planner and computed bound

- New `remove/observation/deletion/plan.rs`: `BatchPlan::of(inventory)`. It
  does the partition the engine did (content first, then `.git`, compared
  component-wise), then deepest level first, then directory order, keeping
  inventory order inside a directory, in chunks of `MAX_BATCH_ENTRIES`.
  `levels()` keeps the level boundaries, because the engine drains after each
  level. `numbered()` yields `(batch number from 1, entries)`. The engine now
  iterates this plan, and its private `levels()` helper is gone.
- `recovery/journal.rs`: `journal_size(id, inventory) -> JournalSize {
  opening, batches }`, plus `JournalSizeOverflow`. `batches` is
  `batch_bound`. For each planned batch it adds the intent line and the
  all-`externally_absent` results line, both serialized by the writer's own
  `Line` into a `Counter` writer, with the newline and checked arithmetic.
  `begin_batch` now computes its reservation with the same
  `worst_results_len`, so the bound and the reservation share one function.
  `DEFAULT_JOURNAL_LIMIT` is removed. `JournalWriter::limit()` was added for
  the wiring tests.
- **Departure (no behavior change):** `DeletionInventory` now serializes
  through a borrowed record (`InventoryRecordRef`) instead of `into =
  "InventoryRecord"`, and the journal writes its inventory line through a
  borrowed `InventoryLine`. Before, every `start` cloned an inventory as
  large as the checkout (167 MB for `feat/osc-9-4-protocol`), and the sizing
  pass would have cloned it again. The bytes are identical:
  `the_opening_size_is_what_start_writes` parses them back.
- **Departure (Ruling 7, done early):** `JournalWriteError::LimitReached`'s
  `Display` is now "wt reached an internal limit of its deletion journal; a
  retry continues", with no byte count. This is the text the CLI's stop
  message prints. Phase 4 only needs to verify that rendering.

### Wave 2: store limits and the space refusal

- New `remove/recovery/space.rs`: the `SpaceSource` trait
  (`available_bytes(path) -> Result<u64, String>`) and `NativeSpace`, which
  calls `sniff::filesystem::space::available_space`.
- `RecoveryStore::begin(.., space)` and `reopen(.., space)` take the reading
  seam. `begin` computes `journal_size` and the record's JSON length, then
  checks the space at the store root **before** creating its staging
  directory. It writes the same `record_json` it counted. `reopen` reads the
  previous journal, decides with the new non-writing `archive_target` (split
  out of `archive_journal`) whether a new archive copy is needed, and adds
  `previous.len()` only in that case. It checks the space at the operation
  directory before archiving or creating anything. The writer limit is
  `size.batches` in both `start` and `after_header`.
- New `StoreError` variants: `JournalSize`, `InsufficientSpace { path,
  needed, available }`, and `SpaceUnknown { path, needed, reason }`. All
  three map to `ExitClass::Blocked` (exit 4) in `Refusal::exit_class`.
- `Environment` gained `space: &dyn SpaceSource`. The first attempt and a
  `Quarantined` retry pass `env.space`. `release_unstarted` has no
  environment, so it passes `NativeSpace` to the `reopen` it does just before
  `discard`. The CLI passes `NativeSpace`.

### Wave 3: usage handoff and report version

- `DeletionInventory::tree_index() -> TreeIndex` maps `Shape` to
  `TreeEntryKind` one to one, with the root identity as the index root. It
  `expect`s success, because `try_from` already guarantees everything the
  builder refuses: plain relative paths, each listed once, each in a listed
  *directory*, so nothing is ever below a link.
- `UsageSource::query(path, index: Option<&Arc<TreeIndex>>)`. `SniffUsage`
  adds `with_tree_index` when it is given an index.
  `check_usage(source, checkout, own, index)` passes the same `Arc` to the
  Windows Search recheck. The first attempt builds the index right after
  `capture` (step 2 → 3), and so does a resumed `Quarantined` attempt. Both
  happen before the drain, the confirmation, and `begin`'s sizing, as the
  spec orders. Diagnostics after a stop call `query(path, None)`.
- `REPORT_VERSION` 5 → 6. The rustdoc says what version 6 adds, that
  versions 4 and 5 still read, and that an older `wt` need not read a newer
  report. The journal format and version are unchanged; the limit is
  attempt-local.

### Requirement-to-test mapping

Every test is L1. They are F2 at the lifecycle boundary (real temporary
repositories, the injected observer, and injected usage and space sources),
and F2 for the real-Sniff usage test (a real process and a real temporary
tree).

| Requirement | Test(s) |
| --- | --- |
| Planner: each entry once; content before `.git`; deepest first; directory order; inventory order kept; chunks; root content and `.git` in separate batches; empty | `deletion::plan::tests::every_entry_is_planned_once_in_the_engines_order`, `an_empty_inventory_plans_no_batch` |
| Bound covers short and long paths, JSON escapes, a non-UTF-8 name (cfg Unix bytes / Windows surrogate), hard links, many small directories, more than 256 siblings, a link, `.git`, more than 100 batches (digit transitions) | the `varied()` inventory in `journal::tests` |
| Exact fit under worst-case results; one byte less refuses only the final reservation and writes nothing of it | `worst_case_results_fill_the_bound_exactly_and_one_byte_less_refuses_the_last_batch` |
| Every result state fits | `every_result_state_fits_the_bound` |
| Opening bytes = what `start` writes; the borrowed inventory line reads back | `the_opening_size_is_what_start_writes` |
| Empty inventory → bound 0 | `an_empty_inventory_needs_no_batch_bytes` |
| Overflow refused, never wrapped | `a_size_past_u64_is_refused_not_wrapped`. The store computes the size before any write (`begin` before staging, `reopen` before archiving), so an overflow writes nothing |
| Former 64 MiB ceiling: a bound > 64 MiB streams through a counting sink with no `LimitReached`, and emitted bytes ≤ bound | `a_journal_past_the_former_64_mib_ceiling_streams_without_reaching_its_limit` (about 1.4 s) |
| Initial and resumed writers get limits from their own inventories | `store::tests::begin_and_reopen_limit_their_journals_to_their_own_inventories_bound`. The lifecycle passes `env.space` and the inventories to them; `Space::asked` shows which path each attempt read |
| First-attempt refusal: insufficient and unavailable readings; boundary (exactly enough begins); no operation or staging directory | `store::tests::begin_refuses_before_writing_when_the_journal_cannot_fit`; lifecycle `large_checkout::a_first_attempt_whose_journal_cannot_fit_refuses_before_publishing` (exit 4, message names the store path, needed and available bytes or the reason, checkout, registration, and branch intact, nothing pending, then a control run removes) |
| Retry refusal: no deletion, archive, or replacement; earlier evidence reads; archive-copy cost; already-identical archive costs nothing | `store::tests::reopen_refuses_before_archiving_or_replacing_when_the_journal_cannot_fit`, `an_already_identical_archive_adds_nothing_to_the_space_a_retry_needs`; lifecycle `a_retry_whose_journal_cannot_fit_refuses_before_archiving_or_deleting` |
| A sufficient reading, then a write failure, follows the crash rules | `store::tests::a_sufficient_reading_then_a_failed_write_leaves_the_previous_attempt_recoverable` (the existing reopen fault matrix also runs with a sufficient reading) |
| Resume after confirmed tracked-file deletions without `discard_files`; `.git` last; only the approved branch action (large-checkout regression) | `resume_consent::a_retry_asks_nothing_for_tracked_files_deleted_over_several_batches` (300 tracked files, at least 2 batches) |
| Preflight and resumed preflight get the attempt's index (root = checkout identity, len = inventory); diagnostics get none | `large_checkout::preflight_queries_get_their_attempts_index_and_diagnostics_get_none` |
| The Windows Search recheck reuses the same index (`Arc::ptr_eq`) | `large_checkout::the_windows_search_recheck_reuses_the_attempts_index` |
| A real Sniff query with the supplied index: provenance `supplied`, the same holder as the walk, process enumeration reached; an index of another root → `Unverified(invalid_option)` | `usage::tests::a_real_query_names_a_process_working_in_the_checkout` (extended; it does not assert full coverage) |
| Pending removals saved by report versions 4 and 5 (Sniff reports with no provenance, and no `observation` in version 4) resume with a newly sized journal; provenance `unknown`; earlier confirmed deletions and the attempt are kept; saved as version 6 | `large_checkout::a_pending_removal_from_report_versions_4_and_5_resumes_without_losing_its_evidence`, plus the existing `outcome::tests::a_version_4_report_reads_…` |

Input robustness: this phase adds no new reader. Report loading is Sniff's
version-aware reader (Phase 2's matrix) embedded in `OperationReport`. The
lifecycle test above is its end-to-end round trip (load old, resume, write
version 6, read again).

### Test-speed note

The first version of the 64 MiB test took 5.7 s. Debug-build serde_json
escapes strings at about 50 MB/s, and the test serializes the bound once
for sizing and about 1.5 times more in the writer. Measured and rejected:
`opt-level = 1` for `worktree` in the test profile (25% faster, at the cost
of compile time), and serializing each `NativeString` with one UTF-8 check
instead of two (no gain; reverted, so Sniff is unchanged in this phase). The
fix: the test's names are control characters, each written as a six-byte
`\u00XX` escape. That passes 64 MiB with 7,400 entries instead of 31,000 and
covers JSON escaping at scale. The test also skips the inventory line, which
never counts against the limit. It now takes about 1.4 s.

### Gates and other runs

- `cd worktree && just test`: 1613 passed, 15 skipped, 0 failed. `just lint`:
  clean. It first failed on four clippy findings in the new code
  (`fold` on `Try`, `never_loop`, and two `cloned_ref_to_slice_refs`), all
  fixed. `just check-tier-coverage worktree`: 0 stranded.
- Dependents: no package outside the `worktree` area depends on `worktree`,
  and this phase changed nothing in Sniff, so no other package was run.
- `just cross-check worktree --os windows`, **attempt 1**: 889/892. The three
  failures all came from the `varied()` journal fixture: a name containing a
  backslash is a path separator on Windows, so the inventory refused it
  ("directory not listed"). Fix: test-only, on Windows that name drops the
  backslash. **Attempt 2**: 892/892. `just cross-check worktree-cli --os
  windows`: 654/654. These include the real `wt remove` runs, whose usage
  check now gets the inventory's index from Sniff, so the checkout identity
  `wt` captures matches the one Sniff resolves on native Windows.
- After that fixture fix, macOS was re-run: the journal, store, and
  `large_checkout` tests (43 passed), and `just lint` (clean).
- Linux and WSL2 were not cross-checked locally. Nothing in this phase is
  specific to them; CI covers Linux, and the nightly covers WSL2.
- Scope check, run from the repository root after every edit:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gives `packages: [sniff, sniff-cli, worktree, worktree-cli, test-toolkit,
  darkmatter]` and `change_class: package`. Sniff is listed because Phase 2's
  changes are still uncommitted in this working tree. The gates were
  therefore required, and they were run.
- `DEFAULT_JOURNAL_LIMIT` appears nowhere outside `fixes/`.

### Skipped or pre-existing

- Nothing was skipped. No pre-existing failure was seen on macOS or Windows.
- `worktree/docs/cli/remove.md` and `worktree/README.md` are Phase 5's
  scheduled doc tasks and were not edited here. The worktree skill pages
  `remove.md` and `lifecycle.md` were updated now, because the code they
  describe changed (journal limit, space check, `Environment.space`, and the
  `UsageSource` and `check_usage` signatures).
- The 500,000-entry acceptance case is Phase 5's task.
