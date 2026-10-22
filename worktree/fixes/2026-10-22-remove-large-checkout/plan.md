---
total_phases: 5
created: 2026-10-22
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Plan: `wt remove` of a built checkout finishes in one run

## Summary and definition of done

Four changes, spread over three package areas (`sniff`, `worktree` lib,
`worktree-cli`):

1. **Computed journal limit.** Replace the fixed 64 MiB `DEFAULT_JOURNAL_LIMIT`
   with a bound computed from the attempt's inventory by a batch planner shared
   with the deletion engine.
2. **Free-space refusal before the move.** A new focused, path-based
   available-space query in Sniff feeds a refusal (exit 4) that fires before
   publishing a first attempt or before archiving or replacing on a retry.
3. **Caller-supplied tree index.** Sniff exposes `TreeIndex` publicly, and
   `PathUsageOptions` accepts it so `query_path_usage` skips the walk. `wt`
   converts its captured `DeletionInventory` into the index. Reports record
   provenance (`walked`, `supplied`, or `unknown` for old reports); report
   versions are bumped with backward-compatible loading.
4. **One discovery summary.** The stop message prints identical current-attempt
   coverage notes once.

Done means every item below holds:

- [ ] A synthetic inventory whose batch bound exceeds 64 MiB streams through a
  counting sink with no `LimitReached`, and emitted bytes stay within the bound.
- [ ] Replaying all batches with worst-case `ExternallyAbsent` results fits the
  bound exactly, and a one-byte-smaller limit rejects the final reservation.
- [ ] An insufficient or unavailable space reading refuses with exit 4 before
  the move (first attempt) or before archive and replacement (retry), leaving
  checkout, registration, branch, and earlier evidence intact.
- [ ] `query_path_usage` with a supplied index performs zero tree walks and
  reaches process enumeration; the walked and supplied indexes of the same tree
  find the same holders and hard-link aliases.
- [ ] Version 4 and 5 pending removals and old Sniff reports still load, with
  provenance `unknown` for old Sniff reports.
- [ ] The stop message prints identical current-attempt notes once.
- [ ] `just test` and `just lint` are green in `sniff/`, `worktree/`; docs and
  skills listed in the spec are updated; CI evidence for all four OS
  environments follows the repo's existing rules.
- [ ] The 500,000-entry acceptance case is run on a disposable built checkout
  if one is available (not a mandatory L1 fixture).

Work stays at L1 (≤5 s per test, no terminal or browser windows). Per repo rules:
no `cargo fmt`, no commits unless told, load the `rust-testing`, `sniff`,
`os` and `worktree` skills before touching the relevant code.

---

## Phase 1: Rulings and survey

### Necessary Rules

The spec leaves these points to the implementer. Each has a proposed ruling
that the author can overrule before Phase 2 starts.

1. **Report version numbers.** Sniff's usage-report schema version goes up by
   one from its current value. The `worktree` recovery report goes to version 6
   (current `main` writes 5). Version 4 and 5 stay loadable. *Proposed: adopt as
   stated.*
2. **Old Sniff report provenance.** An absent provenance field in a report whose
   schema version predates provenance deserializes to `Unknown`. An absent field
   in a current-version report is an error, never `Unknown` or `Walked`.
   Explicit `null` follows the existing optional-field convention. *Proposed:
   adopt.*
3. **Batch-bound arithmetic contract.** The planner produces `(batch_number,
   entries)` in the deletion engine's order. The bound is
   `Σ (len(intent_line(n)) + len(worst_results_line(n)))` with the `\n`
   included, using `ExternallyAbsent`. The checkout root is not an entry. Overflow
   (`checked_add`/`checked_mul`) is a pre-move refusal. This is the same
   function the engine uses to plan, so a grouping difference cannot exist.
   *Proposed: adopt.*
4. **Journal size accounting for the space check.** Needed bytes = header +
   inventory line + batch bound + initial recovery record (first attempt), or
   header + fresh inventory line + batch bound + the previous journal's full
   length when a new archive copy is needed (retry). The "already identical
   archive" case adds zero. *Proposed: adopt.*
5. **Available-space API home and shape.** A new function in Sniff's filesystem
   area, `available_space(path) -> Result<AvailableSpace, SpaceError>`. It
   resolves the path's real location (alias-aware) and uses the user-available
   figure (`statvfs` `f_bavail`, `GetDiskFreeSpaceExW` caller quota). It spawns no
   subprocess and calls no device inventory. Injected through the existing
   platform seams so `worktree` tests can inject readings through its own seam
   (a small trait or closure on the store). *Proposed: adopt.*
6. **Supplied-index constructor errors.** A `TreeIndexError` maps onto
   `PathUsageError` invalid-option at query time for root identity mismatch;
   constructor-time validation errors (absolute path, `..`, duplicates,
   descendants under a link) are returned from the builder. *Proposed: adopt.*
7. **Stop message wording for an internal limit.** `LimitReached` is rendered as
   "`wt` reached an internal limit; a retry continues" with no byte count
   presented as the user's problem. *Proposed: adopt.*
8. **Wider measurements** (not scheduled; for the author to decide): measuring
   the space query and the batch-bound pass on Windows/WSL2 hosts, and running
   the 500,000-entry acceptance case on every OS. The spec asks for none of
   these, so this plan schedules none.

### Spikes

The spec's own figures already answer the performance questions (494 B/entry,
136,000 entries per 64 MiB, 218 MB bound), so **no performance spike** is
scheduled. One read-only survey spike remains because it decides how much
refactoring Phases 2 and 3 need:

- [x] **Code survey** (single pass, no code changes; write findings as a
  short note in the implementation log):
    - Sniff: the current shape of `TreeIndex` (fields, visibility, how
      identities, links and Windows probe entries are represented), where the
      walk is invoked in `query_path_usage`, the report schema version and
      `PathUsageOptions` fields, and the existing storage seams.
    - Worktree: how `deletion.rs` plans batches (so the planner extraction is
      mechanical), how `begin_batch` reserves the worst-case results line, and
      where `RecoveryStore::begin`/`reopen` pick the limit.
    - Report the gaps against Rulings 3 and 6 and flag any ruling that the
      code contradicts.

### Validation checkpoint

- [x] Rulings 1–7 confirmed (or amended) and the survey note written before
  Phase 2 begins.

---

## Phase 2: Sniff capabilities

Two independent waves can run concurrently; both are in `sniff/`.

### Wave 1: Focused available-space query (parallel with Wave 2)

- [ ] **Space query API**
    - Add the path-based query described in Ruling 5 under the filesystem
      area, with a platform seam for tests.
    - Resolve the actual path including ancestor aliases (symlinks,
      firmlinks, junctions, mount points); no string-prefix volume matching.
    - Report space available to the calling user.
- [ ] **Space query tests** (L1, through the seams)
    - Path to filesystem resolution, alias ancestors, a store on a different
      filesystem than the checkout, nonexistent leaf resolved via nearest
      existing ancestor, and error mapping.
    - Prove neither the storage device inventory nor any subprocess is invoked.
- [ ] **Docs**: storage API docs entry: the query, errors, "a reading is not a
  reservation".

### Wave 2: Supplied tree index and provenance (parallel with Wave 1)

- [ ] **Public TreeIndex builder** (depends on the Phase 1 survey)
    - Expose `TreeIndex` plus a constructor/builder taking the root identity
      and entries of `(relative path, kind, identity)`, with no dependency on
      caller types.
    - Constructor validation: reject absolute paths, parent traversal,
      duplicate relative paths, descendants below a supplied link. Keep native
      names (no lossy conversion), all hard-link aliases, and link identities
      without following targets.
    - Sniff's own walk must produce the same type; one matcher only.
    - Keep the Windows regular-file/directory probe entries; never probe links
      as targets.
- [ ] **Option on `PathUsageOptions`**
    - Takes the prebuilt index (shared reference/`Arc`, validated once before the
      query; no per-query conversion or clone of the whole tree).
    - Reject: root identity differs (invalid-option `PathUsageError`);
      `target_only`; file root.
    - When set, skip the walk. Root verification and final consistency checks
      stay in the budget; the saved time goes to process enumeration.
- [ ] **Provenance and coverage in the report**
    - Add `walked` | `supplied` | `unknown` provenance; a caller may declare the
      index partial and Sniff reports partial coverage. Sniff never claims to
      have verified a supplied descendant. Process-mechanism coverage and
      outcome stay library-owned.
    - Bump schema version; old reports load with `unknown` (Ruling 2). Update
      the CLI JSON contract, schema fixtures, round-trip tests.
- [ ] **Tests** (existing fake usage backend + performance counters)
    - Zero tree walks and zero descendant identity reads during a supplied
      query; root verification still happens.
    - Walked vs. supplied indexes of the same tree: same holders and
      hard-link aliases.
    - Invalid paths, wrong root, partial coverage, invalid target scope, links,
      Windows probe-entry behavior; unsupported mechanisms stay unsupported;
      budget exhaustion in process inspection stays visible.
- [ ] **Docs**: `sniff/docs/topics/filesystem-query.md` (lead with "you already
  walked the tree"; builder, root requirement, point-in-time limit, provenance,
  old-report compatibility, unchanged Windows limits) and the `sniff` skill.

### Input Robustness Matrix (report loader and index builder)

**Default outcome:** every invalid shape produces a located error (field path
for reports; the offending relative path for the index builder), runs nothing,
and publishes nothing. Exceptions only:

| Field / shape | Outcome |
| --- | --- |
| Report `provenance` absent, schema version older than the bump | `Unknown` (explicit, never `Walked`) |
| Report `provenance` absent, current schema version | error |
| Report `provenance` explicit `null` | error (the field is not optional; null is not absent) |
| Report `provenance` wrong type / unrecognized string | error |
| Optional report fields already using the explicit-null convention | unchanged |
| Index entries `[]` (empty) | valid: a root with no descendants |

Tests: **one** field-shape walk through the shared validator, plus a control
case (an unedited current fixture loads) and an old-version fixture. The
loader is shared by all report consumers: test document shapes (duplicate key,
trailing content) once for that loader, and spawn the Sniff CLI only for one
control and one invalid case proving wiring. Grep before finishing:
`#[serde(default)]` on `provenance`, `Option<T>` where absent and null must
differ, `unwrap_or_default()`, `.ok()` on parses.

### Validation checkpoint

- [ ] `cd sniff && just test && just lint` pass; no `walked` default appears
  anywhere for old reports.

---

## Phase 3: Worktree library

### Wave 1: Shared planner and computed bound (Rust, `worktree/lib`)

- [ ] **Extract batch planner**
    - Move the deletion engine's batching (content first, `.git` last; deepest
      first, then directory order; chunks of `MAX_BATCH_ENTRIES`; entry order
      inside a directory preserved; no grouping by parent alone) into a shared
      function used by both the engine and the sizer. State and test the "each
      entry planned at most once per attempt" invariant.
- [ ] **Bound computation**
    - Serialize each planned batch with its actual number (from 1), newline
      included, intent plus worst-case (`ExternallyAbsent`) results, via a
      counting writer or one reusable buffer; checked arithmetic; inventory data
      only (no filesystem access).
- [ ] **Tests**: all cases in the spec's "Computed bound" and "Former size
  ceiling" bullets (short/long paths, JSON escaping, non-UTF-8 names on
  applicable platforms, hard links, empty inventory, many small directories,
  >`MAX_BATCH_ENTRIES` siblings, `.git` separation, digit transitions, exact fit,
  one-byte-smaller rejection, overflow without writes, synthetic >64 MiB bound
  through a counting sink).

### Wave 2: Wiring and refusal (depends on Wave 1 of this phase and Phase 2 Wave 1)

- [ ] **Store limits**
    - `RecoveryStore::begin`/`reopen` compute the limit from the attempt's
      inventory and pass it to `JournalWriter::start`; remove
      `DEFAULT_JOURNAL_LIMIT`. Keep `LimitReached` and the `begin_batch` reserve;
      render as an internal limit (Ruling 7). Tests that exercise
      `LimitReached` still pass an explicit small limit.
- [ ] **Free-space check**
    - Before publishing the first attempt, and before archiving or creating
      the replacement on retry, compare the needed bytes (Ruling 4) with the
      reading. Failures use the existing storage-refusal path (exit 4), naming
      the store path, needed and available bytes, or why the reading failed.
    - First attempt: no operation directory published; checkout, registration and
      branch intact. Retry: no further deletion, archive, or replacement; prior
      evidence still reads. Do not credit space deletion may free.
    - Injection seam for readings.
- [ ] **Tests**: wiring (initial and resumed writers get inventory-derived
  limits); resume after a confirmed tracked-file deletion needs no renewed
  consent and no `discard_files`, `.git` last, only approved branch actions;
  refusal cases from the spec's "Free-space refusal" bullet, including the
  archive-copy cost, the already-identical archive, and a sufficient reading
  followed by an injected write failure under the existing crash-recovery rules.
  This also adds the large-checkout regression for the own-deletions consent
  case.

### Wave 3: Usage handoff and report version (depends on Phase 2 Wave 2)

- [ ] **Index conversion**
    - Convert `DeletionInventory` to Sniff's `TreeIndex` once per attempt
      (step 2 → step 3, and the fresh surviving inventory on quarantined retry).
      The Windows Search recheck reuses that attempt's index and still verifies
      the root. Keep the observer-handle exclusions and removal policy as they
      are; keep the index construction *before* event drain/confirmation and
      journal sizing ordering as the spec states.
- [ ] **UsageSource**: extend it so preflight receives the index and diagnostics
  after a stop do not (they query current leftovers with a fresh walk).
- [ ] **Report version 6**: embed the new Sniff reports, retain loading of
  versions 4 and 5, document that older binaries need not read new reports. Old
  journals keep their format; the limit is attempt-local, not stored.
- [ ] **Tests**: preflight, resumed preflight and Windows recheck get the index;
  diagnostics don't; with deterministic budget and backend seams the process
  enumeration is reached without a walk (no assertion of full coverage from the
  live host); load v4/v5 reports and old journals then resume with the newly
  sized journal without losing earlier reports or confirmed results.

### Validation checkpoint

- [ ] `cd worktree && just test && just lint` pass; grep for
  `DEFAULT_JOURNAL_LIMIT` returns nothing but historical docs.

---

## Phase 4: CLI stop message

- [ ] **Merge identical notes** in `worktree/cli/src/commands/remove/recovery.rs`
    - Compare the rendered coverage notes (not whole reports) of the current
      attempt's preflight and moved-path diagnostic; if equal, print once and
      say it applies to both paths, with before/after timing explicit.
    - Never merge earlier-attempt notes, failed-query messages, or holder
      evidence; all runs stay in the saved report.
    - Use `biscuit-terminal` components (`Prose`, lists).
- [ ] **Render the internal-limit wording** (Ruling 7).
- [ ] **Tests**: integration test (snapshots belong in an integration test, not
  shared unit tests): identical notes once with both contexts; different notes
  separately; earlier attempts/failed queries/holders uncollapsed.
- [ ] Checkpoint: `cd worktree && just test && just lint`; one spawned-CLI
  wiring case for the space refusal (exit 4, message names path, needed and
  available), no focus-stealing windows.

---

## Phase 5: Docs, skills and final verification

Doc tasks may run concurrently (no code dependencies); write each from the
perspective of a developer new to the repo, per CLAUDE.md. Docs state behavior,
never name this fix.

- [ ] `worktree/docs/cli/remove.md`: no journal ceiling; free-space refusal and
  message; usage-check coverage on large checkouts; one summary when matching.
- [ ] `worktree/README.md`: brief note on built checkouts, link to remove topic.
- [ ] `.claude/skills/worktree/remove.md` and `lifecycle.md`: computed journal
  limit and the inventory passed to the usage check.
- [ ] `.claude/skills/sniff/`: when to supply your own tree index.
- [ ] `docs/dependencies.md`: only if crates changed (not expected).
- [ ] Drift pass: `///` and `//!` docs and comments on every symbol whose
  behavior changed (journal, store, usage, deletion engine, TreeIndex).
- [ ] **Final verification**
    - `just test` and `just lint` in `sniff/`, `worktree/`; `just test-l2` where
      the area has relevant L2 tests; review `just ci-local --plan` before any
      push; cross-platform evidence follows existing CI and the `os` skill.
    - If a disposable built checkout is available, run the 500,000-entry
      acceptance case once and record the result.
    - Update `status` in the spec frontmatter only per the author's process; the
      agent's terminal state is "implementation complete, ready for review" and
      the agent never moves the spec to `_completed`.
