---
total_phases: 3
created: 2026-10-23
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
    - worktree/lib/src/remove/recovery/store/retention.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/Cargo.toml
    - Cargo.lock
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/remove_recovery.rs
docs_updated_during_phase_1:
    - worktree/docs/cli/remove.md
    - worktree/README.md
    - docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
packages:
    - worktree
    - worktree-cli
source_files_during_phase_2:
    - worktree/lib/Cargo.toml
    - Cargo.lock
    - worktree/lib/src/remove/outcome.rs
    - worktree/lib/src/remove/lifecycle.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/entries.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/store/tests/saved_report.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/observation/deletion/tests.rs
    - worktree/cli/src/commands/remove/recovery.rs
    - worktree/cli/tests/remove_recovery.rs
    - worktree/cli/tests/snapshots/remove_recovery_report__a_git_command_in_the_moved_checkout.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_after_partial_discovery.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__long_lists_are_counted_and_uncertain_entries_named.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__the_specs_example.snap
docs_updated_during_phase_2:
    - worktree/docs/cli/remove.md
    - worktree/README.md
    - docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
start_phase: 4
source_files_during_phase_3:
    - Cargo.toml
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/observation/inventory.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/store/tests/journal_size.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
    - worktree/cli/tests/remove_recovery.rs
docs_updated_during_phase_3:
    - worktree/docs/cli/remove.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
source_code:
    - Cargo.lock
    - Cargo.toml
    - worktree/lib/Cargo.toml
    - worktree/lib/src/remove/outcome.rs
    - worktree/lib/src/remove/lifecycle.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/entries.rs
    - worktree/lib/src/remove/recovery/store/retention.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/store/tests/saved_report.rs
    - worktree/lib/src/remove/recovery/store/tests/journal_size.rs
    - worktree/lib/src/remove/observation/inventory.rs
    - worktree/lib/src/remove/observation/deletion/tests.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/src/commands/remove/recovery.rs
    - worktree/cli/tests/remove_recovery.rs
    - worktree/cli/tests/snapshots/remove_recovery_report__a_git_command_in_the_moved_checkout.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_after_partial_discovery.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__long_lists_are_counted_and_uncertain_entries_named.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__the_specs_example.snap
documentation:
    - worktree/docs/cli/remove.md
    - worktree/README.md
    - docs/dependencies.md
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
completed_phase: 3
implemented: true
---

# Removal store size implementation plan

## Summary and Definition of Done

Keep recovery state in `<git-common-dir>/wt-removal/`, expire completed operations after 14 days, move the full saved entry list into `entries.jsonl.gz`, and replace repeated batch paths with inventory positions. Deliver these changes in the specification's order: retention, report version 6, then journal version 3. Each phase includes its tests and current documentation and ends in a reviewable state. Do not commit or move the fix into `_completed`.

Completion means:

- Removal startup silently removes only eligible completed operations and old staging directories, respects admin-name locks, and never expires pending or unreadable records.
- Every saved version 6 report omits `journal_entries`, names a complete atomic gzip sidecar or explicitly uses `entries_file: null`, and preserves existing summaries, truncation, activity, diagnostics, and earlier attempts.
- Journal writers emit version 3; recovery reads version 2 and version 3 independently for each generation and preserves deletion, uncertainty, and retry semantics.
- Legacy report versions 4 and 5 remain readable. A pending operation with version 5 metadata and version 2 journals resumes to new version 3 attempts.
- A deterministic 100,000-entry, 150-byte-path storage fixture proves that a completed operation occupies strictly less than 5% of the corresponding version 2/report version 5 completed folder's bytes.
- `just test` and `just lint` pass from `worktree/`. Tests remain L1, use isolated local fixtures and no network, and open no terminal or browser windows. No `cargo fmt` runs.

Wave numbers are unique across the plan. Tasks within a wave may run concurrently only where explicitly stated; the next wave starts after all tasks and the checkpoint of the preceding wave pass. Any delegated session must receive the non-interactive constraint, its file ownership, and the settled rules below.

## Phase 1 — Retain only recent completed operations

### Necessary Rules

1. **Scope and measurement.** Implement all three requirements without moving the store, changing inventory encoding, reclaiming stranded pending operations, or adding notifications. No spike or wider performance survey is required: the specification already supplies the observed sizes and identifies repeated paths as the cost. The 100,000-entry comparison is a deterministic regression test, not a runtime discovery exercise.

2. **Cleanup API and invocation.** Add `RecoveryStore::prune_completed(&self, now: SystemTime) -> ()`, with implementation in a new `recovery/store/retention.rs` child module. Add a best-effort CLI helper `prune_recovery_store(base: &Path)` in `cli/src/commands/remove/mod.rs`. Invoke it immediately after resolving the base checkout in both `run` and `run_handoff`, before normal name resolution or handoff consumption. The helper checks for an existing store before calling `RecoveryStore::open`, suppresses every cleanup/open error, and uses `SystemTime::now()`. Do not put cleanup in `open`, `resolve_target_in`, or `pending_records`: `PendingRemovals::read` uses those readers from `wt list`, which must not clean the store. Evidence: `remove/mod.rs::{run,run_handoff}`, `lifecycle/resolve.rs::{pending_records,PendingRemovals::read}`.

3. **Expiration authority.** A valid `RecoveryRecord` with `completed_phase == Phase::Completed` is the sole authority to expire an operation. Read it, acquire `self.lock(&record.repository)` without waiting, then reread the record and its eligibility under that lock. Keep the lock alive until deletion finishes. Skip every failed read, invalid binding, busy lock, linked directory, or storage validation failure. Never infer completion from report outcome: a branch failure can leave `outcome: removed` with a pending record. Evidence: `store.rs::{read_record,lock}`, `lock.rs::acquire`, `lifecycle.rs::Runner::run` completion path.

4. **Retention clock.** Use the last element of the saved report's top-level `timeline`, whose `at_ms` is Unix milliseconds; do not use its maximum or an earlier attempt's timeline. A missing, unreadable, malformed report, empty timeline, or unrepresentable timestamp falls back to the operation directory's modification time. Delete only when `now.duration_since(completion)` succeeds and is strictly greater than `Duration::from_secs(14 * 24 * 60 * 60)`. Keep exact-boundary and future timestamps. An unreadable fallback mtime also keeps the folder. Read report metadata only, never the compressed entries, to determine age. Evidence: specification requirement 1 and `outcome.rs::TimelineEvent`; current `RecoveryStore::read_report` returns `Option<OperationReport>`.

5. **Directory boundaries.** Enumerate immediate children only; recognize operations with `OperationId::parse`, staging folders by the exact `.<valid-operation-id>.tmp` form, and leave `locks/`, `.discarded` folders, arbitrary names, files, and links/reparse points alone. Validate candidates with `storage::validate_dir` before deleting. Old staging folders use their own mtime and require no admin lock because they have no published operation record. Attempt each eligible recursive deletion once, synchronously; suppress failures and continue. No waiting for locks, journal scans, retries, output, background threads, or time-based test sleeps. The specification's no-delay requirement means no waits or retry loops; a directory scan and filesystem deletion necessarily consume time. Evidence: `RecoveryStore::begin` staging format, `Operation::discard`'s distinct hidden format, and `storage.rs` trust rules.

6. **Report memory and wire contract.** Keep `OperationReport::journal_entries: Vec<JournalEntry>` as the complete in-memory list so `Runner::summarize_journal`, renderers, and existing lifecycle assertions retain their semantics. Exclude it from version 6 serialization. Add `entries_file: Option<String>`, serialized even when null. Set it to the literal `entries.jsonl.gz` for a nonempty successfully summarized list and to null for no entries or failed summarization; never construct a path from a deserialized string. `OperationReport::new` initializes null, and `Runner::summarize_journal` sets the field alongside the list. Evidence: `outcome.rs::OperationReport`, `lifecycle.rs::summarize_journal`, and the complete entry assertions in `lifecycle/tests.rs` and `lifecycle/tests/resume_consent.rs`.

7. **Report compatibility.** Set `REPORT_VERSION = 6`. Implement version-aware `Deserialize` for `OperationReport` using private wire structs in `outcome.rs`: versions 4/5 require the existing inline `journal_entries` and have no `entries_file`; version 6 requires `entries_file` (null or exactly `entries.jsonl.gz`), forbids inline `journal_entries`, and produces an empty in-memory list when read. Preserve version 4's absent-observation compatibility and existing observation-null semantics. Keep `Serialize` on the public struct with the in-memory list skipped. Reject unsupported versions and duplicate/unknown members; reuse `strict_json::members(...).into_value()` semantics through a deserialize visitor or equivalent strict intermediate rather than permissive `Value` version dispatch. No gzip hydration is added to `read_report`: `Resumed::attempt` needs earlier metadata, and `recover` reconstructs entries from journals. Evidence: `outcome.rs`'s version 4 fixture, `store.rs::read_report`, `lifecycle/resume.rs::Resumed::attempt`, and `OperationReport::continue_from`. Update saved-report equality assertions to compare metadata and separately decompress the sidecar, rather than equating a disk metadata report with its full in-memory counterpart.

8. **Sidecar writer and dependencies.** Add `ENTRIES_FILE` in `store.rs` and a focused `recovery/store/entries.rs` child module. Add `flate2 = { version = "1", default-features = false, features = ["rust_backend"] }` to `worktree/lib/Cargo.toml`; use `GzBuilder` with mtime 0, no filename, and default compression. Stream each `JournalEntry` directly with `serde_json::to_writer` plus a newline into the encoder; never allocate a second complete JSONL byte buffer. Finish gzip, flush the returned file with `storage::flush`, close it, then `storage::publish` the temporary sibling. Remove the temporary on ordinary failures. Preserve owner-only storage and Windows close-before-rename rules. `flate2` 1.1.9 already exists in `Cargo.lock`; update only dependency-resolution changes, not unrelated versions. Add `filetime = "0.2"` as a library dev-dependency for portable staging/fallback-mtime fixtures; document both additions in `docs/dependencies.md`. Evidence: `storage.rs::{temp_beside,create_private_file,flush,publish}` and the existing lockfile entry.

9. **Publication order and failure.** `Operation::save_report` publishes the gzip sidecar before `report.json`; `complete` then persists `Phase::Completed` and only then removes journals, preserving its current `journal_reported` gate. An entries write/finish/flush/publish failure returns `StoreError`, leaves journals and completion phase untouched, and follows current lifecycle warning handling. A report write failure also prevents completion/journal deletion. An empty list creates no gzip file; remove an obsolete sidecar only after publishing the null-reference report, with deletion failure treated as best-effort housekeeping. An interrupted two-file publication may leave a newer complete sidecar beside older report metadata; journals remain authoritative while pending. No two-file transactional guarantee is added. Failed summarization keeps journals and publishes null rather than claiming that an old sidecar is complete. Evidence: `Operation::{save_report,complete}`, `Runner::{save_report,summarize_journal}`, and `recovery/mod.rs` crash guarantees.

10. **Entry meaning and CLI text.** Preserve the exact `attempt`, `batch`, lossless `NativeString` path, kind, state, generation order, and uncertainty semantics returned by `replay_generations`. Do not add observed post-approval activity to the entries list or change consent/deletion policy. Keep all non-entry report fields and caps. Replace the `ConcurrentWriteLimit` caveat in `cli/src/commands/remove/recovery.rs` with the specified saved-entries wording, and change the deleted-count location hint from “listed in the saved report” to “listed in entries.jsonl.gz”. Update associated snapshots explicitly; no blanket snapshot acceptance. Evidence: `store.rs::replay_generations`, `outcome.rs::{set_deleted,continue_from}`, `recovery.rs` caveat and `deleted` formatter. Policy changes described by other removal fixes are outside this change.

11. **Journal representation.** Keep public `PlannedEntry`, `Batch`, `JournalLog`, `JournalWriter::{begin_batch,finish_batch}`, and replay results unchanged. Replace private wire `Line` with version-specific types `LineV2` and `LineV3` plus a shared header decoder. Version 3 intents contain zero-based inventory positions as `u64`; results contain `states: Vec<ResultState>`. Resolve indices with checked conversion to `usize` and a bounds check, then derive path and kind from that journal's inventory. The existing `DeletionInventory::{position,entries}` already supplies both directions. No sorting or inventory renumbering is allowed. Evidence: `journal.rs::{Line,Batch,JournalWriter}`, `observation/inventory.rs::{position,entries}`, and the deletion engine's unchanged writer calls.

12. **Journal validation and limits.** `JOURNAL_VERSION = 3` is the written version; parse accepts exactly 2 and 3 by each file's own header, including archived files. Normalize version-specific records into existing batches and retain ordering, open-batch, alignment, same-directory, and 256-entry checks. Enforce cross-batch uniqueness in both writer and reader for both versions; valid historical writers do not plan an entry twice. Replace the writer's `filter_map(position)` with checked collection that errors on an unknown entry. Compute `limit_for` from actual version 3 intent/results encodings for every inventory position in a singleton batch, using `u64::MAX` as the batch number and `ExternallyAbsent` as the longest state. Sum those exact worst-case lengths with saturating arithmetic. Header/inventory remain outside the batch budget, as today. Evidence: `check_batch`, `begin_batch`, `results_line`, the existing singleton worst-case test, and the specification's index requirements.

13. **No extra gzip reader.** Production writes the sidecar but does not parse it for recovery. Test-only decoding verifies its contents, gzip trailer, and line termination. Accordingly, the robustness matrix covers the report and journal readers changed here, not a new sidecar loader. Keep the current public recovered-entry path through `RecoveryStore::recover`. Evidence: recovery already reads all journal generations; completed operations cannot resume and their entries file is an audit artifact.

### Wave 1

- [x] **Implement retention**
  - Implement rules 2–5 in the library and wire both CLI entry points. Preserve all normal errors and output of the requested removal.
  - Extend `recovery/store/tests.rs` with a fixed-clock store containing a 15-day completed record, 13-day completed record, 30-day pending record, old staging directory, and locked completed record. Assert only the expired unlocked completed operation and staging folder disappear.
  - Cover the strict 14-day boundary, future times, last-event precedence over directory mtime, unreadable/missing report fallback, empty timeline fallback, malformed record preservation, young staging, unrelated names, locks preservation, links, and one deletion failure without aborting cleanup of another eligible folder. Use existing platform fixture helpers; use `filetime` for mtimes, not sleeps.
  - Add CLI wiring cases to `cli/tests/remove_recovery.rs`: run `wt remove` against a nonexistent name after planting expired records and compare stdout/stderr/exit status with the clean-store control. This proves cleanup precedes target resolution and emits no extra text. Add a handoff-start case proving the second entry point invokes cleanup. Add a `wt list` control proving expired folders remain untouched there.
- [x] **Document retention**
  - May run concurrently with implementation; owns only `worktree/docs/cli/remove.md`, `worktree/README.md`, `.claude/skills/worktree/remove.md`, `.claude/skills/worktree/lifecycle.md`, and dependency notes for `filetime` in `docs/dependencies.md`.
  - Replace indefinite retention statements with completion-based 14-day expiration on removal startup, lock skipping, silent best-effort behavior, and permanent preservation of pending records. Keep README to a compact overview/link; put details in the CLI topic page. Do not pre-document unimplemented formats in this phase.

Validation checkpoint: inspect the targeted retention and CLI cases through `just test`, then run `just test` and `just lint` from `worktree/`. Retention must work with existing version 5 reports before Phase 2 begins. Add no CI cells or gates.

## Phase 2 — Save compact reports and compressed entries

### Wave 2

- [x] **Implement sidecar storage**
  - Implement rules 6–9, including version-aware report deserialization, `entries_file`, streaming gzip, durable publication, and lifecycle integration. Own `outcome.rs`, `lifecycle.rs`, `recovery/{mod.rs,store.rs}`, the new entries module, library manifest, lockfile, and persistence/lifecycle tests.
  - Extend existing completion and resume tests to assert actual saved JSON contains no inline list; decompressed lines equal the full expected in-memory entries across attempts, including deleted, externally absent, uncertain, kept, and planned states. Assert counts, first 200 deleted paths, omitted count, activity, observation, timeline, and earlier attempts remain unchanged.
  - Add empty-list and failed-summary cases, repeated stop/report replacement, and deterministic failure injection at gzip finish/flush, entries publication, report publication, and completion persistence. Use a scoped test-only fault seam modeled on `store::fault`/`ReopenStep`. At each stop, verify complete old-or-new files, retained journals, and no premature completed record. Verify that journal deletion occurs only after successful sidecar and report publication and durable completion.
  - Add real version 5 saved metadata fixtures with inline entries and retain the existing version 4 fixture. Read through `RecoveryStore::read_report` and resume through lifecycle APIs, rather than proving only a private deserializer works. Freeze legacy wire bytes in test fixture code; do not generate “legacy” reports using the new serializer.
  - Add the report-loader robustness cases below, sharing validators instead of multiplying the field matrix by report version.
- [x] **Update entry presentation**
  - May run concurrently with storage work because it consumes only the existing `ReportNote` and count fields. Own `cli/src/commands/remove/recovery.rs`, `cli/tests/remove_recovery_report.rs`, and only affected snapshots.
  - Apply rule 10 to caveat and deleted-count location text. Cover both successful completion and a stopped removal containing deletions. Preserve activity's reference to `report.json`, because that data remains there.

### Input Robustness Matrix

Default outcome for invalid persisted input is a located returned error, no resume mutation, no deletion, and no newly published report or entries file. Journal errors identify operation and line through existing `StoreError::Journal`; report errors must add the `report.json` path in `RecoveryStore::read_report` while preserving JSON line/column details where available. Retention deliberately suppresses report errors and uses its specified mtime fallback; it never treats an invalid record as completed. A failed optional previous-report read retains the existing resume warning/metadata behavior; valid journals, binding, and renewed consent remain required before resume mutation. Evidence: `Resumed::attempt` already treats `read_report` separately from recovery authority.

- **Report loader:** start from valid version 4, 5, and 6 reports; require controls through `read_report`. Test the shared version validator for absent, explicit null, wrong scalar type, empty string/object, negative/fractional value, and unsupported integer. Test `entries_file` with absent, null, wrong whole-field type, empty string, exact valid literal, other filename, absolute path, and traversal. Null is valid only as an explicit version 6 field; absence is invalid for version 6 and expected for legacy versions. Legacy versions reject the new field. Version 6 rejects inline `journal_entries`.
- **Legacy inline list:** one shared collection/entry validator covers absent, null, wrong whole-field type, one invalid element among valid entries, every element invalid, and empty collection. Empty is a valid explicit no-entry list; absence and null are invalid. Exercise required entry fields `attempt`, `batch`, `path`, `kind`, and `state` with absent/null/wrong-type/empty/invalid-content cases; reuse the journal's native path and enum validation tests and add one valid and one invalid wiring case per reused validator. Preserve the explicit observation compatibility exceptions in rule 7.
- **Report document shapes:** reject duplicate top-level and nested keys, trailing data, malformed JSON, and a second JSON document. Empty files are errors, not absent files; a missing report returns `Ok(None)`. Add one CLI control and one invalid-report retry case proving the path appears in its warning and the established recovery behavior is preserved; do not launch a binary for every shape.
- **Journal loader:** implement its matrix in Phase 3's existing `journal_robustness_matrix`, not a duplicate Phase 2 suite. Its field and document outcomes are fixed below.

### Wave 3

- [x] **Document report format**
  - After Wave 2, update `docs/cli/remove.md`'s recovery layout, version 6 JSON example, separate decompressed JSONL example, stop/completion writes, null behavior, failure guarantees, and the distinction between bounded activity/deleted previews and full journaled entries. Keep the retention behavior from Phase 1.
  - Update `.claude/skills/worktree/{remove.md,lifecycle.md}`, `recovery/mod.rs`, and affected symbol comments to remove the obsolete claim that `report.json` lists every entry. Record `flate2` and its Rust backend in `docs/dependencies.md`. README receives only the brief overview adjustment needed to mention compressed audit entries.
  - Extend the retention fixture with both legacy and new reports to prove the timestamp rule does not depend on entry representation and never decompresses the sidecar.

Validation checkpoint: `just test` and `just lint` from `worktree/`; inspect changed snapshots, gzip contents, no-entry representation, all publication failure cases, and legacy reads. Journals still write version 2 at this checkpoint. Run the smell search from Phase 3 now for report-loading changes as well.

## Phase 3 — Encode journal batches with inventory positions

### Wave 4

- [x] **Implement indexed batches**
  - Implement rules 11–12 in `recovery/journal.rs`, retaining the deletion engine's public call contract. Keep version-specific wire representations private and normalize before replay.
  - Extend `recovery/store/tests.rs::journal_robustness_matrix` with frozen valid version 2 bytes and version 3 writer output. Retain common inventory checks once through their shared validator, with a control and invalid wiring case in each version branch. Keep all reopen crash-point and generation-deduplication tests.
  - For required version 3 `intent.entries` and `results.states`, cover absent, null, wrong whole-field type, one wrong element, every element wrong, and empty. Both arrays must be nonempty for an actual batch. For indices additionally cover zero, last valid index, first out-of-range, negative, fractional, string, and integer conversion overflow. For states cover all valid enum strings plus unknown, numeric, null, and empty values. Test both too few and too many states, changed batch number, results with no open intent, repeated results, nonconsecutive intents, more than 256 entries, mixed parents, and duplicates within/across batches.
  - Test required header `journal_version`/`operation_id` and intent/results `batch` scalar fields for absent, null, wrong type, empty, and invalid content. Reject duplicate keys, extra members, wrong-version line shapes, trailing garbage, blank JSONL lines, repeated header/inventory, and missing inventory. Preserve the deliberate exception: an unterminated final line is ignored as torn, while missing/torn opening inventory remains an error. Test malformed terminated lines separately from torn lines.
  - Assert invalid journal cells through `RecoveryStore::recover`, including error line and unchanged operation/quarantine. Use one CLI valid control and two invalid-journal cases to prove rendered location and refusal before deletion. Extend the existing matrix infrastructure rather than spawning per-cell binaries.
  - Update `the_limit_fits_every_entry_planned_alone_with_the_longest_result`, exact-budget/one-byte-short tests, and failed append/sync poisoning tests to version 3 bytes. Include inventories that cross index digit boundaries (9/10, 99/100, 999/1000), empty inventory, full 256-entry batches, and synthetic longest batch number encoding. Prove version 3 batches contain positions/states and no path/kind copies.

### Wave 5

- [x] **Verify migration size**
  - Add an L1 persisted-store fixture in `recovery/store/tests.rs` with exactly 100,000 file paths, each exactly 150 ASCII bytes, using a shared `target/` prefix, padding, and a six-digit unique suffix. Use synthetic `DeletionInventory` and `JournalEntry` data with real temporary store files; do not create or unlink 100,000 real files. Include identical record metadata, bounded deleted preview, and complete entry states in both comparisons.
  - Build the old completed-folder baseline with a test-only frozen version 5 pretty-JSON representation containing the full inline list and the same `record.json`. Old completed journals were already deleted; exclude them from the baseline. Measure actual logical file lengths of the completed new folder, including record, report, and gzip and any accidentally retained journals/temporaries. Exclude the sibling `locks/` directory from both. Assert `new_bytes * 100 < old_bytes * 5`, gzip entry equality, and absence of journals after completion. Report sizes in assertion diagnostics; impose no timing SLA.
  - Add a separate stopped-store comparison for the same synthetic inventory using bounded full batches: produce frozen version 2 and actual version 3 journals, verify identical normalized replay, and assert version 3 batch-line bytes are smaller. Count header/inventory separately to demonstrate that this change preserves the inventory cost. No extra cross-host timing experiment is scheduled.
  - Extend `lifecycle/tests.rs` migration coverage with a pending version 5 report, version 2 archived/current journals, and a fresh version 3 attempt. Stop, resume, and complete through lifecycle APIs; assert generation order, no duplicate accounting, uncertainty preservation, metadata carry-forward, complete gzip entries, and final journal removal.
- [x] **Document journal format**
  - May run concurrently with migration/size verification; owns documentation and skill topic files only. Update `docs/cli/remove.md` with version 3 index/state examples, unchanged inventory approval, mixed-generation reading, and continued version 2 support. Preserve the accessible recovery flow and add a compact Mermaid diagram showing journals → atomic entries/report save → completion → journal removal → retention cleanup.
  - Update `.claude/skills/worktree/remove.md` journal facts and `.claude/skills/worktree/lifecycle.md` completion facts; audit `recovery/mod.rs`, `journal.rs`, `store.rs`, and `outcome.rs` comments against the final implementation. Remove drift introduced by the old “only version” wording and report-inline entry references. Do not link current docs to a feature/fix snapshot.

Final validation checkpoint:

- Run `just test` and `just lint` from `worktree/`; use nextest through these recipes, no `cargo test` and no formatter invocation. Confirm both `worktree` and `worktree-cli` ran. Keep platform-neutral persistence fixtures eligible on macOS, Linux, native Windows, and WSL2, with no focus-taking resources.
- Search changed loading paths for `#[serde(default)]`, `Option<T>`, `filter_map`, `unwrap_or_default()`, and `.ok()`. Verify each remaining use is one of the explicit compatibility or torn-tail exceptions, rather than swallowing invalid load-bearing input. Require explicit-present deserialization for nullable version 6 `entries_file`.
- Reconcile every robustness finding against all sibling field shapes and document-shape cases in the affected validator/loader, and record the state of those cases in the implementation log. No narrow one-cell fix without its row/column audit.
- Inspect the final diff for unrelated changes, leaked temporary files, stale inline-entry docs, blanket snapshot changes, and unintended dependency upgrades. Record actual byte measurements and passing validation in an implementation log beside this plan. End at “implementation complete, ready for review”; leave the specification's lifecycle location unchanged.
