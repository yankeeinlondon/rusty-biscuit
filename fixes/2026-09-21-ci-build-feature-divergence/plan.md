---
title: Stop divergent base features from recompiling the workspace per archive
status: ready
created: 2026-09-21
phase: 3
total_phases: 4
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
spec: fixes/2026-09-21-ci-build-feature-divergence/spec.md
related:
    - 2026-09-12-single-os-compile
packages:
    - repo-deps
source_files_during_phase_1:
    - scripts/feature-attribution.rs
    - scripts/feature-attribution-tests.rs
    - scripts/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/Cargo.lock
    - scripts/ci/fixtures/feature-attribution/core/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/core/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/util/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/util/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-own/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-own/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-plain/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-plain/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-third/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-third/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/owner-wsdep/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/owner-wsdep/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/vendor/base/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/vendor/base/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/vendor/twin-1/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/vendor/twin-1/src/lib.rs
    - scripts/ci/fixtures/feature-attribution/vendor/twin-2/Cargo.toml
    - scripts/ci/fixtures/feature-attribution/vendor/twin-2/src/lib.rs
    - fixes/2026-09-21-ci-build-feature-divergence/timed-pass.sh
    - fixes/2026-09-21-ci-build-feature-divergence/attribution-data/render-attribution.py
docs_updated_during_phase_1:
    - fixes/2026-09-21-ci-build-feature-divergence/spec.md
    - fixes/2026-09-21-ci-build-feature-divergence/plan.md
docs_created_during_phase_1:
    - fixes/2026-09-21-ci-build-feature-divergence/attribution-2026-09-21.md
    - fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
skills_files_updated_during_phase_1:
    - .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_2: []
docs_updated_during_phase_2:
    - fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
    - fixes/2026-09-21-ci-build-feature-divergence/plan.md
    - fixes/2026-09-21-ci-build-feature-divergence/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
    - fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
    - fixes/2026-09-21-ci-build-feature-divergence/plan.md
    - fixes/2026-09-21-ci-build-feature-divergence/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
---

# Plan: Stop divergent base features from recompiling the workspace per archive

## Work Summary and Success Criteria

The `2026-09-12-single-os-compile` design compiles each selected package's
Nextest archive inside one owner target tree, preserving separate artifacts for
incompatible feature sets. The analyzed `build (ubuntu-latest)` job shows the
cost of what that design faithfully preserves: 89 workspace-crate compiles for
28 distinct crates, because a handful of third-party feature flags near the
base of the graph (`proc-macro2/span-locations`, `libc/extra_traits`,
`serde_core/default`, and others) re-identify nearly every workspace crate
built above them, per owner.

The work splits into four phases:

1. **Rulings, spikes, and attribution (spec part 1).** Resolve the spec's Open
   Question and the other decision points as recorded rulings, run four cheap
   local spikes, then build an attribution script that enumerates every
   `(workspace crate, configuration)` for the ten-package selection and —
   joined with one local timed pass — charges each configuration the compile
   seconds it causes. The ranked table decides how far part 2 goes.
2. **Per-flag remedy decisions.** From the ranking, produce a decision table
   assigning one of the spec's remedies (isolate the requester, remove the
   source, align for every owner, leave divergent) to every divergent
   configuration, and execute whichever non-alignment removals are viable.
3. **Alignment crate and reviewed entries.** If (and only if) the ranking
   warrants it, create the hand-written `tools/` alignment crate carried by
   every member as a dev-dependency, and add eligible third-party feature
   entries one reviewed change at a time.
4. **Verification and follow-through.** Re-run the attribution script, prove
   isolated-package fidelity, confirm the archive contract tests are
   unchanged, finish the documentation obligations, and record — without
   gating and without triggering — the next ordinary pull request's owner-job
   build-time sum.

**Successful completion looks like:**

- The attribution table exists for the ten-package selection
  (`biscuit-file`, `claudine`, `claudine-cli`, `claudine-gen`, `darkmatter`,
  `darkmatter-cli`, `dmls`, `repo-deps`, `sniff`, `sniff-cli`), with every
  configuration beyond a crate's first explained, and the implementation log
  states the split between third-party-inherited and workspace-own divergence
  in **compile seconds**, not only counts.
- The spec's Open Question has a recorded ruling before any part 2 work, and
   no isolation change lands before that ruling exists.
- Every flag handled under part 2 is a separate, reviewable change naming the
  crate, flag, remedy, recompiles removed, and — where alignment was chosen —
  why isolation and source removal did not apply, plus the written reason the
  feature cannot change a test outcome.
- No workspace crate's own feature is aligned (`biscuit-file/fetch`,
  `biscuit-hash/blake3`, `darkmatter/effects-instrumentation` stay exactly as
  declared).
- `scripts/ci-build-archive-tests.rs` — including
  `one_owner_tree_shares_a_dependency_compile_without_unifying_features` —
  passes unchanged.
- Re-running the attribution script after alignment shows fewer configurations
  for the affected workspace crates; the next ordinary pull request with a
  comparable package set shows a lower sum of Cargo build time in its owner
  job. Both are recorded, neither gates anything, and no run was triggered to
  obtain them.
- For each package whose closure changed, `cargo test -p <package>` alone
  still builds.
- `docs/dependencies.md` and the `rust-devops` skill are updated in the same
  changes that move dependency edges, and the skill's CI notes record the
  finding for the future.

## Execution Constraints

- **No new CI runs, matrix cells, fixtures, or gates.** Never trigger a CI run
  to learn anything this plan can derive locally; the attribution and timing
  work is local by design. The `ci:all-os` label is not used for this fix.
- **`2026-09-12-single-os-compile` stays in force.** Packages remain separate
  Cargo invocations in one owner target tree; nothing unifies features across
  packages except the narrow, reviewed, third-party-only alignment mechanism
  this spec defines.
- **Each part 2 flag is a separate, reviewable change.** Prepare each as its
  own diff/commit; commit only when instructed (never proactively); commits
  carry no agent attribution and are signed per the repository's git
  configuration.
- **Never run `cargo fmt`.** Use `just lint` for touched areas and
  `just test` / `just test-l2` recipes for tests (nextest, not `cargo test`,
  for suites).
- **Load the `rust-devops` skill before Phase 2 work** — the alignment crate
  and any remedy that touches manifests sit inside the CI conventions that
  skill records.
- **GitNexus discipline:** run `just gitnexus` for a fresh index before
  editing, impact analysis before modifying any existing symbol, and
  `detect_changes` (scope `all`) before handing any change over for commit.
- **Storage:** the timed pass is a multi-archive local build; confirm disk
  headroom first (storage-strategy skill) and let it share the one root
  `target/` tree like the owner does.
- **No test execution during the timed pass.** Only archive-style builds run;
  no terminal or browser window may gain or lose focus at any point.
- **US English** throughout symbols, docs, and comments.

## Phase 1 — Rulings, spikes, and the attribution table

Phase 1 delivers spec part 1 in full: the rulings recorded, the four spikes
run, the attribution script built with tests, the one timed local pass, and
the ranked table plus one-page reading that decides how far part 2 goes.
Nothing in Phase 2 or 3 may start before this phase's rulings and ranking
exist.

### Necessary Rules

The following rulings resolve the spec's open decision points. Task 1.5
records them in `spec.md` (the Open Question ruling goes into that spec's Open
Questions section) and in `implementation-notes.md` before any part 2 work.

1. **Isolation ruling (resolves the spec's Open Question): adopt Option 1.**
   `proc-macro2/span-locations` is aligned through the alignment crate
   whenever that crate is created; the source-scan guard test stays in
   `claudine-cli`; no new crate, no planner change. Option 2 (a
   `[package.metadata.ci]` watch declaration in `affected_scope.py`) is
   chosen **only** if the Phase 1 ranking shows the alignment crate is not
   warranted at all. Option 3 (a real dependency edge from the guard to
   `claudine`) is rejected outright: it re-imports the divergent
   configuration for `claudine`'s whole closure and guts the remedy.
2. **Alignment crate identity.** The crate, if created, is named
   `feature-alignment` at `tools/feature-alignment` — a root workspace member
   with no bins and no tests, only an empty `src/lib.rs` and reviewed
   `[dependencies]` entries, `publish = false`. Renaming during Phase 3 is
   allowed only with a recorded reason (collision or clearer convention).
3. **`expectrl` source-removal fallback.** If the Phase 1 spike confirms the
   newest `expectrl` still rides `ptyprocess 0.5` → `nix 0.26` (blocked
   upstream), `libc/extra_traits` is handled by alignment and the block is
   recorded with the check's evidence. Wholesale replacement of `expectrl`
   across its five dev-dependency holders is out of scope for this fix.
4. **The "ruled per crate, default no" set stays divergent.**
   `time/local-offset`, `hyper/server`, `hyper-util/*`, `tower/*`,
   `crossterm/*`, `mio/*`, `tokio-stream/net`, `clap/*` remain divergent in
   this fix. One of them may be aligned only if the ranking shows it causes
   material seconds **and** a per-flag ruling is recorded in `spec.md` first
   (not in the implementation log alone).
5. **`rust-cache` `cache-workspace-crates` is dismissed unless falsified.**
   The expectation — a fresh checkout resets mtimes, so restored workspace
   artifacts are judged stale — is verified cheaply (spike below) and
   recorded. The option is adopted only if that verification falsifies the
   expectation; never on hope.
6. **Attribution tooling disposition.** The first-pass tool is a standalone
   `repo-deps` binary (`feature-attribution`, behind the `local-tools`
   feature), not a `ci-build` subcommand. Whether it becomes a reusable
   report beside `ci-build`, emitted from the plan the owner already reads,
   is decided once in Phase 4 from how useful the first table was; the
   decision and its reason are recorded. No CI emission is added by this fix.
7. **Publish interaction is verified before the fan-out.** A path
   **dev-dependency** on a `publish = false` crate must be proven not to
   block `cargo publish`/`cargo package` for publishable members (Cargo's
   packaging of dev-dependencies is the trap). The check runs in Phase 3
   before any member manifest is edited; if it blocks, the remedy (e.g.,
   version-annotated path dev-dependency, or a ruling recorded in the spec)
   is decided before proceeding.

### Spikes

Four cheap, local, time-boxed investigations. All four are Wave 1 tasks;
none compiles the workspace beyond what `cargo tree`/`cargo metadata`
require, and none triggers CI.

- **S1 — Re-derive the divergence table at the implementing branch's base.**
  The spec's thirteen-row table was derived at `feat/dark-fixes`, not `main`.
  Re-derive it at the implementing branch's base with the two documented
  traps avoided: key rows on `(crate, version)` and ignore the root
  package's own feature display. Discrepancies against the spec's table are
  recorded and, if a base flag row appears or disappears, escalated as a
  ruling amendment before Phase 2.
- **S2 — Trace which owners demand `libc/extra_traits`.** Determine which
  `nix` versions in the lock (0.26.4, 0.29.0, 0.31.3) enable
  `libc/extra_traits` and therefore exactly which of the ten owners pull the
  variant (expected: `claudine-cli` and `sniff-cli` via `expectrl`).
- **S3 — Re-check `expectrl` upstream.** Confirm whether any release newer
  than 0.8.0 still pins `ptyprocess 0.5` → `nix 0.26` (time-boxed; crates.io
  and docs.rs only). The outcome feeds ruling 3.
- **S4 — Cheaply verify the workspace-crates cache expectation.** Simulate
  what a fresh checkout does to workspace-crate mtimes in a scratch clone
  with a warmed target tree and observe whether Cargo recompiles them.
  Record confirmed/refuted; time-boxed. This only informs ruling 5 — no CI
  or workflow change follows from it in this fix.

#### Wave 1 (all four spikes in parallel)

- [x] **Task 1.1 — Re-derive divergence** (Spike S1; no prerequisites).
  - Run `cargo tree -e features,normal,build,dev` for each of the ten owner
    packages at the implementing branch's base; diff resolved feature sets
    pairwise against the shared closure, keyed on `(crate, version)`.
  - Record the reproduced table (expected: the spec's thirteen
    `claudine`/`claudine-cli` rows and no others) plus any cross-owner rows
    the ten-package set adds, in `implementation-notes.md`.
- [x] **Task 1.2 — Trace nix owners** (Spike S2; parallel with 1.1, 1.3, 1.4).
  - From the lock and `cargo tree -i libc` per owner, list which owners
    demand `libc/extra_traits` and through which `nix` version/chain.
  - Record the owner list; this seeds the Phase 2 decision table's
    `libc/extra_traits` row.
- [x] **Task 1.3 — Re-check expectrl** (Spike S3; parallel; time-boxed).
  - Record the newest `expectrl` version and its `ptyprocess`/`nix` chain,
    with a link to the evidence. Feeds ruling 3; no code change.
- [x] **Task 1.4 — Verify cache expectation** (Spike S4; parallel;
  time-boxed).
  - In a scratch clone under the temp workspace directory, warm a target tree,
    reset workspace-source mtimes the way a fresh checkout does, and observe
    whether Cargo recompiles workspace crates.
  - Record confirmed/refuted with the exact commands; feeds ruling 5.

#### Wave 2 (after Wave 1 outcomes)

- [x] **Task 1.5 — Record rulings** (prerequisite: Wave 1 results; edits
  `spec.md`, so surgical changes only).
  - Write the seven rulings above into the spec: the Open Question ruling
    (Option 1, with the Option 2 fallback condition and Option 3 rejection)
    into the spec's Open Questions section, and the remainder as a dated
    rulings note in the spec's Scope and design decisions area or the
    implementation log referenced from it.
  - Capture Wave 1's spike outcomes as the evidence attached to rulings 3
    and 5.
  - Flip the spec's `status` to `finalized-spec` once the rulings are
    recorded (the author owns any further lifecycle moves).

#### Wave 3 (two tasks in parallel; both depend only on the selection, not on Wave 2)

- [x] **Task 1.6 — Build attribution script** (complexity: the highest in
  this plan — reverse-dependency attribution; budget tests accordingly).
  - Add a `feature-attribution` binary to `scripts/` (package `repo-deps`,
    `[[bin]]` behind the existing `local-tools` feature, alongside `drift`
    and `repo-deps`). Inputs: the owner package list; per-owner
    `cargo tree -e features,normal,build,dev` output (dev edges included —
    archives always resolve dev-dependencies); `Cargo.lock`; the manifests.
  - Output: for every workspace crate, each distinct configuration the
    selection builds, the owner that first builds it, and for each later
    configuration the cause chain classified as the spec requires — the
    crate's own features, a workspace dependency's features, or a
    third-party dependency's features (naming the crate and flag).
    Attribution walks the reverse-dependency closure to the first crate
    whose resolved features differ and is not itself explained by a deeper
    difference (the root of the divergence).
  - Bake the two methodology traps into the implementation and its tests:
    `(crate, version)` keying and ignoring the root package's own feature
    display.
  - L1 unit tests against a fixture workspace (precedent:
    `scripts/ci/fixtures/archive-portability`) covering: own-feature cause,
    workspace-dep cause, third-party cause, and a deliberately divergent
    fixture mirroring
    `one_owner_tree_shares_a_dependency_compile_without_unifying_features`.
  - Human output uses `TerminalRenderable` components where it renders to a
    terminal; the machine output is JSON beside the rendered table. Update
    `scripts/Cargo.toml` features comment block and `docs/dependencies.md`
    in the same change if any dependency is added (none expected).
- [x] **Task 1.7 — Run timed pass** (prerequisite: storage headroom check;
    parallel with 1.6; no prerequisites on it).
  - Reproduce the owner's archive builds locally, once per owner package, in
    the producer's deterministic package order (read it from the resolved
    plan the owner already produces — `just ci-local --plan` — so the
    incremental recompile sequence matches the analyzed job), in the one
    root `target/` tree, with each package's CI features from its
    `[package.metadata.ci.tests]` (for `claudine-cli`: `daemon-tests`,
    `terminal-tests`).
  - Build only; never execute tests. Capture per-crate compile seconds
    (for example `cargo build --timings` for the same package and feature
    graph, or timing the archive command per owner — record which was used
    and its fidelity caveat).
  - Persist the raw timings (JSON/HTML artifacts plus a summarized table)
    under this fix's directory for Phase 1's join and Phase 4's before/after
    comparison.

#### Wave 4 (after Waves 3)

- [x] **Task 1.8 — Publish attribution table** (prerequisite: Tasks 1.6 and
  1.7; this task decides how far Phases 2 and 3 go).
  - Join script output with timed-pass seconds: charge each configuration
    beyond a crate's first the seconds of the crates it re-identifies.
    Produce the ranked output — configurations by compile seconds caused.
  - Write `fixes/2026-09-21-ci-build-feature-divergence/attribution-2026-09-21.md`
    (the table, the selection, the tree/revision, the method) and the
    one-page reading in `implementation-notes.md` (the same document is this
    fix's implementation log), stating explicitly:
    - the split between third-party-inherited and workspace-own divergence
      in compile seconds (acceptance criterion 1), and
    - the recommended extent of part 2: which flags justify action, which
      remedy per flag, and whether the alignment crate is warranted at all
      (the trigger for ruling 1's Option 2 fallback).
  - Validation: the table explains every configuration beyond a crate's
    first for all ten owners; the thirteen-row third-party divergence from
    S1 appears in it or is accounted for.

**Phase 1 validation checkpoint.** `just test repo-deps` passes with the new
binary's L1 tests; `just lint` is clean for `scripts/`; the spec contains the
recorded rulings; `attribution-2026-09-21.md` and the log's one-page reading
exist with the seconds split; no CI run was triggered by any Phase 1 task.

## Phase 2 — Per-flag remedy decisions and non-alignment removals

Phase 2 converts the ranking into committed decisions and executes every
remedy that is **not** alignment. Nothing here edits feature declarations
yet; alignment (if warranted) is Phase 3. The `rust-devops` skill is loaded
before this phase's work.

#### Wave 5 (single task; depends on Phase 1's ranking and rulings)

- [x] **Task 2.1 — Write decision table** (prerequisite: Task 1.8 and
  recorded rulings; no source changes).
  - For **every** divergent configuration the attribution table found (the
    three base flags, the remaining ten third-party rows, and the
    workspace-own rows), record one decision-table row in
    `implementation-notes.md`: crate, flag, shape (member-declared /
    transitive-some-owners / workspace-own), remedy chosen (isolate, remove
    the source, align, leave divergent), the recompiles it removes (in
    seconds, from the ranking), and the rationale.
  - Apply the spec's ordering rules: isolation and source removal are tried
    before alignment, and where alignment is later used the log must already
    say why the other two did not apply. Under ruling 1, isolation's
    inapplicability for `proc-macro2/span-locations` is the recorded Option 1
    choice; under ruling 3, source removal for `libc/extra_traits` is the
    recorded upstream block (unless Task 2.2 executed); `serde_core/default`
    has no source-removal path (axum is in `claudine-cli`'s closure by
    design) and says so.
  - Workspace-own divergence (`biscuit-file/fetch`, `biscuit-hash`
    features, `darkmatter/effects-instrumentation`) is rows with remedy
    "leave divergent by design". If the attribution data surfaces a package
    whose tests pass only because another owner enabled a workspace feature
    (the missing-declaration hazard), record it as a report-only finding for
    the author — do not fix declarations in this fix.
  - If the ranking shows the alignment crate is not warranted at all, this
    task records that conclusion plus the Option 2 fallback decision from
    ruling 1, and Phases 3 and 4 collapse to their no-op rationale.

#### Wave 6 (conditional; tasks exist only if the decision table assigns them)

- [x] **Task 2.2 — Execute source removals** (only for flags whose decision
  row says "remove the source"; expected: none, per ruling 3 — then this
  task is closed with the recorded block evidence).
  - Each removal is a separate, reviewable change: the manifest/lock edit,
    the recompile seconds it removes (re-derive the affected owners'
    `cargo tree` diff), and `docs/dependencies.md` updated in the same
    change.
  - Verify no behavior change: the affected packages' L1 suites pass
    locally (`just test <pkg>`).
  - **Closed (Phase 2):** no flag row assigns source removal; every
    third-party removal is blocked or not applicable (see the decision
    table). The `schematic-define` → `biscuit-file` edge (option B, measured
    at ≈77 s) is a source-removal candidate waiting on the author's ruling;
    if approved, it runs here as a separate change before Phase 3.

**Phase 2 validation checkpoint.** Every divergent configuration has a
decision row; no row aligns a workspace crate's own feature; every executed
change is separable and carries its docs update; `detect_changes` was run
over each diff handed off; the archive contract suite still passes
(`just test repo-deps`).

## Phase 3 — Alignment crate and reviewed entries

Phase 3 exists only if Task 2.1 concluded the alignment crate is warranted.
Each task below is a separate reviewable change, ordered so the crate exists
and is wired before any entry is added. Entries are add-only, each with a
comment naming the recompiles it removes, and are added in the ranking's
seconds order. No workspace crate's own feature is ever an entry.

#### Wave 7 (two tasks in parallel)

- [x] **Task 3.1 — Verify publish interaction** (ruling 7; must complete
  before Task 3.2's fan-out — a blocking prerequisite, scheduled early so
  the result is known before any member manifest is touched).
  - On a scratch branch of the manifests, add the prospective path
    dev-dependency to one **publishable** member and run
    `cargo package -p <member> --list` / `cargo publish --dry-run` (or the
    repository's release-plz dry run) to prove packaging is unaffected.
  - Record the outcome in `implementation-notes.md`. If it blocks, record
    the chosen remedy as a ruling amendment before proceeding.
  - **Done (Phase 3): does not block.** A temporary `publish = false`
    crate, wired as a versionless path dev-dependency of the publishable
    `biscuit-hash`: `cargo package --list` and `cargo publish --dry-run`
    both succeed, and the normalized `Cargo.toml` inside the `.crate` has no
    `feature-alignment` entry (Cargo strips it). The edit was reverted. No
    ruling amendment is needed.
- [x] **Task 3.2 — Scaffold alignment crate** (parallel with 3.1; no member
  edges yet).
  - Create `tools/feature-alignment` per ruling 2: root workspace member
    (`members` list entry beside `tools/test-toolkit`), empty `src/lib.rs`,
    no bins, no tests, `publish = false`, and an empty `[dependencies]`
    table with the crate's contract documented in its manifest comment and
    lib doc: third-party base-graph features only, add-only, one reviewed
    entry at a time, never a workspace crate's own features.
  - Update `docs/dependencies.md` (new workspace member) in the same change.
  - **Closed (Phase 3): not built.** Task 2.1 found the alignment crate not
    warranted: the spec's eligible entries remove 0.6 s, and every option
    that removes more (C/D) needs a ruling-4 spec entry that does not exist.
    This phase exists only if the crate is warranted, so it collapses to
    that no-op rationale. If the author later rules C or D, Tasks 3.2–3.4
    run as written. Task 3.1's result already clears their prerequisite.

#### Wave 8 (after 3.1 and 3.2)

- [x] **Task 3.3 — Fan out dev-dependency edges** (prerequisite: Tasks 3.1
  and 3.2; one atomic change — the crate is inert until every member
  carries it).
  - Add `feature-alignment` as a path **dev-dependency** (dev-only: normal
    and release graphs, including published manifests, stay untouched) to
    every workspace member from the root `members` list except
    `feature-alignment` itself. Update `Cargo.lock` in the same change.
  - Update `docs/dependencies.md` and the `rust-devops` skill's CI notes in
    this same change: the crate's contract, why dev-only reaches exactly the
    archive invocations where divergence costs anything, and the known
    fan-out consequence.
  - Record the first real fan-out cost (the next planner/`check`-cell
    behavior after this change) in `implementation-notes.md` when observed —
    per the spec, this cost stays visible, bounded by the established
    compile-coverage rules.
  - Validation in-change: `cargo check -p feature-alignment` and
    `cargo check -p repo-deps --all-targets` pass; `just test repo-deps`
    (including `scripts/ci-build-archive-tests.rs`) passes unchanged.
  - **Closed (Phase 3): not built.** See Task 3.2. No member manifest or
    `Cargo.lock` was edited, so no fan-out cost exists to record.

#### Wave 9 (sequential entries; same file, ranked order)

- [x] **Task 3.4 — Add ranked entries** (prerequisite: Task 3.3; one
  separate change per entry, highest-seconds first; expected order from the
  spec's evidence: `libc/extra_traits`, then `proc-macro2/span-locations`
  per ruling 1, then `serde_core/default` if the ranking justifies it).
  - Each entry is one `[dependencies]` line in
    `tools/feature-alignment/Cargo.toml` with a comment naming the
    recompiles it removes, plus — in the same change — the written reason
    the feature cannot change a test outcome:
    - `libc/extra_traits` — adds trait impls only;
    - `proc-macro2/span-locations` — adds diagnostics data only (this entry
      is the ruling 1 decision, not a new judgment);
    - `serde_core/default` — the crate's ordinary default features.
  - After each entry, re-derive the affected owners' `cargo tree` diff to
    confirm exactly the intended configurations collapsed (no workspace
    crate's own feature set changed for any owner), and run
    `just test repo-deps`.
  - Any "default no" flag promoted per ruling 4 requires its spec ruling
    recorded **before** its entry is written — not after.
  - **Closed (Phase 3): no entries.** See Task 3.2. Under ruling 1,
    `proc-macro2/span-locations` is aligned only if the crate exists, so it
    stays divergent (the pending Open Questions amendment in the spec).

**Phase 3 validation checkpoint.** `just test repo-deps` green including the
unchanged archive contract fixture; the attribution script run over the
ten-package selection shows fewer configurations for every crate the entries
target (full before/after recording happens in Phase 4, a quick count check
happens here per entry); every entry's written reason exists in the manifest
comment and the log; `docs/dependencies.md` and the `rust-devops` skill
carry the crate.

## Phase 4 — Verification, documentation, and follow-through

Phase 4 proves the acceptance criteria that span the whole fix and closes
the documentation obligations. Nothing here gates on CI, and nothing here
triggers a run.

#### Wave 10 (three verification tasks in parallel)

- [ ] **Task 4.1 — Re-run attribution** (prerequisite: Phase 3 complete, or
  its no-op rationale recorded).
  - Re-run `feature-attribution` over the same ten-package selection at the
    final tree; record the before/after configuration counts per workspace
    crate beside the Phase 1 table in `attribution-2026-09-21.md`.
  - Expected: fewer configurations for the crates the entries targeted; the
    workspace-own divergence rows unchanged. Record the result either way —
    acceptance criterion 5 asks for the recording, not a gate.
- [ ] **Task 4.2 — Prove isolated-package fidelity** (acceptance criterion
  6; may reuse qualifying local evidence).
  - For each package whose closure changed (at minimum the ten-owner
    selection once any Phase 3 entry landed), prove that
    `cargo test -p <package>` alone still builds — `cargo test -p <pkg>
    --no-run` (compiles all test targets, runs nothing) is the acceptable
    build proof; full runs are not required.
  - This is the check that no package came to depend on a feature only the
    workspace-level declaration provides. Record the package list and
    results in `implementation-notes.md`.
- [ ] **Task 4.3 — Confirm contracts unchanged** (acceptance criterion 4).
  - Run `just test repo-deps` (L1 plus the Python companion suites,
    including `scripts/ci-build-archive-tests.rs` and
    `one_owner_tree_shares_a_dependency_compile_without_unifying_features`)
    and `just lint` for every touched package area. The archive mechanism
    is not what changed; any fixture failure is a defect in this fix, not a
    fixture update.

#### Wave 11 (closure; after Wave 10)

- [ ] **Task 4.4 — Decide tooling disposition** (ruling 6; single decision,
  recorded).
  - Decide from how useful the first table was whether `feature-attribution`
    becomes a reusable report beside `ci-build` (emitted from the plan the
    owner already reads), stays a standalone `local-tools` diagnostic, or is
    retired to the fix's history. Record the decision and reason in
    `implementation-notes.md`. If "reusable report" is chosen, implement it
    as a bounded follow-on beside `ci-build` **without adding any CI
    emission in this fix**.
- [ ] **Task 4.5 — Record follow-through observation** (acceptance criterion
  5, second half; explicitly not gating, explicitly not triggered).
  - When the next ordinary pull request that selects a comparable package
    set completes anyway, record its owner job's sum of Cargo build time
    beside the analyzed job's 27.9 min in `implementation-notes.md`, with
    the run link. This observation may postdate the plan's terminal state —
    note it as pending if it has not happened by then.
- [ ] **Task 4.6 — Finalize log and spec state**.
  - Complete `implementation-notes.md`: the seconds split (from Task 1.8),
    the decision table (Task 2.1), each remedy change with its rationale,
    the fan-out record, the before/after attribution counts, and the
    follow-through observation status. Apply the comment-quality pass to
    every symbol whose behavior this fix touched.
  - Set the spec's `implemented: true` (and `implemented_by`); the author
    alone moves the fix to `_completed` after the review cycle closes.
  - Run `detect_changes` (scope `all`) over the final working tree and
    record the result with the handoff summary.

**Phase 4 validation checkpoint (plan terminal state).** All seven
acceptance criteria of the spec are demonstrably met and recorded in the
implementation log; `just test repo-deps` and `just lint` for touched areas
are green; the spec frontmatter reflects the implemented state; the working
tree is ready for the author's review, with each part 2 change separable and
signed commits pending the author's instruction.
