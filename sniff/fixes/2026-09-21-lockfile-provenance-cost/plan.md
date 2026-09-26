---
total_phases: 5
created: 2026-09-26
phase: 1
agent: "claude/default"
yolo: "true"
fix: 2026-09-21-lockfile-provenance-cost
spec: sniff/fixes/2026-09-21-lockfile-provenance-cost/spec.md
related:
    - 2026-09-20-repo-perf
packages:
    - sniff
---

# Stop paying for lockfile provenance on every structure detection — implementation plan

Implements `2026-09-21-lockfile-provenance-cost`: make lockfile corroboration a
thing a `RepoRequest` asks for (change 1), and make the `Cargo.lock` parse keep
only package names and versions (change 2). `review-1.md` in this directory lists
the four open findings this plan closes.

## Work Summary and Success Definition

### Summary of required work

The production changes are confined to the Sniff library, plus the docs and
tests that describe it:

- **Request setting.** `sniff/lib/src/request.rs`: add a lockfile-corroboration
  setting to `RepoRequest`. `structure()` sets it to false, `full()` to true, and
  `focused(..)` to false unless the caller opts in. When the setting is absent
  on deserialization, corroboration is on. Add an accessor and a builder.
- **Gate.** `sniff/lib/src/filesystem/repo/detection.rs`: the
  `upgrade_provenance_with_lockfile` loop (currently `detection.rs:747-750`,
  unconditional) runs only when the request asks for corroboration. The
  dependency-version load sites (`synthesize_root_package_repo_with_store`,
  `lock_versions_for_seed`) stay gated on `wants_dependencies()` and keep
  working when corroboration is declined.
- **Read-attempt counter.** `sniff/lib/src/performance/counters.rs`: add one
  stable lockfile read-attempt counter. Increment it before the read in both
  lockfile read paths: `read_counted_lockfile` (pnpm and uv) and
  `CargoLockVersions::parse` (Cargo).
- **Typed Cargo parse.** `sniff/lib/src/filesystem/repo/manifest_index.rs`:
  replace `toml::Value` deserialization in `CargoLockVersions::parse` with a
  typed struct that keeps only `package[].name` and `package[].version`. Keep
  the current behavior for malformed entries and for version order. Keep the
  current implementation as a `#[cfg(test)]` reference for parity tests.
- **Conditional pnpm parse.** Do this only if the measurement in Phase 1 calls
  for it: a typed `importers:`-keys-only parse of `pnpm-lock.yaml`.
- **Evidence and docs.** Record a consumer audit, write Level 1 fixtures and
  counter tests, update CLI JSON tests, and update public docs, the README, and
  `.claude/skills/sniff/performance.md`. Take before/after timings under the
  `2026-09-20-repo-perf` protocol.

### Success looks like

Each item maps to one of the spec's acceptance criteria (AC).

1. `implementation-log.md` records the consumer audit (AC1), and the chosen
   default cites it.
2. Under a fresh work collector, a structure detection that declines
   corroboration on a fixture with `Cargo.lock`, `pnpm-lock.yaml`, and
   `uv.lock` reports **0** read attempts and **0** parses. A request that asks
   for corroboration performs the same checks as before. A focused
   dependencies request may still read `Cargo.lock` (AC2).
3. A controlled fixture matrix produces complete `RepoInfo` values equal to
   independently written expectations when provenance is requested: Cargo, pnpm,
   and uv authorities; matching, extra-member, and missing-member lockfiles;
   absent and unparseable lockfiles. The matrix explicitly shows that Cargo
   accepts extra lockfile entries while pnpm and uv require exact member sets
   (AC3).
4. The typed and reference `CargoLockVersions` parsers agree on name → ordered
   versions for this checkout's `Cargo.lock` and for a duplicate-names or
   malformed-entry fixture (AC4).
5. `just test` passes in `darkmatter/`, including the three
   observation-boundary tests named in `2026-09-20-repo-perf`. The
   `REPOSITORY_DISCOVERY_COUNT` expectations are unchanged (AC5).
6. Tests show that legacy serialized requests keep corroboration on and that
   newly constructed requests use the documented defaults. Docs, the skill, and
   the README are updated, and CLI `--json` stays valid and reflects the request
   tier (AC6).
7. `just test` and `just lint` pass in `sniff/`, and the affected Darkmatter and
   Claudine tests pass. No new CI cell or timing gate is added. Cross-OS
   evidence comes from the existing workflows (AC7).
8. Results are written into `results.md`, with parser, detection, and compose
   effects reported separately, together with load average and median/range.

The agent's terminal state is **"implementation complete, ready for review"**.
The agent never moves this fix to `_completed` and never runs `just complete`.

## Phase 1 — Rulings, audit, spikes, and baseline

This phase makes no production code changes. It settles the open questions and
collects the evidence that Phase 3 depends on.

### Necessary Rules

These rulings bind implementation. If a ruling is reversed, record the reversal
in `implementation-log.md`.

- [ ] **R1 — Default policy: Option A, provisionally, gated on the audit.**
    - The spec recommends A and `review-1.md` asks the author to confirm it. This
      plan runs under `yolo: true`, so it adopts **A** as long as the Phase 1
      audit finds no structure-tier consumer that depends on `Lockfile`
      provenance or `lockfile_match`, or finds that every such consumer can move
      to an explicit request.
    - If the audit finds a consumer whose stable public output cannot be kept
      that way, **switch to B**, record why, and move Darkmatter's capture to an
      explicit declining request (spec AC5, B branch).
    - The default flip is a single task (Phase 3, task "Structure default"). The
      mechanism (setting, gate, counter) does not depend on it and ships either
      way. This lets the author reverse the policy without undoing other work.
- [ ] **R2 — Setting shape.**
    - Add a public field named `lockfile_provenance: bool` to `RepoRequest`,
      matching its existing all-public fields.
    - Put `#[serde(default = "…")]` on it with a default of `true`, so a legacy
      plan keeps corroboration.
    - Always serialize it. Do not use `skip_serializing_if`: the spec says a
      newly constructed request sets the value explicitly.
    - Add the accessor `wants_lockfile_provenance()` and the builder
      `with_lockfile_provenance(self, bool) -> Self`. These follow the existing
      `full_worktree_details(mut self, full: bool)` idiom in `request.rs`.
    - Adding a public field breaks external struct literals. No `RepoRequest { .. }`
      literal exists in the workspace outside `request.rs` (checked 2026-09-26),
      but record the break in the commit body.
- [ ] **R3 — Counter semantics.**
    - Add a new counter,
      `REPO_LOCKFILE_READS = "filesystem.repo.lockfile_reads"`. It counts **read
      attempts**: increment it before `read_to_string`, so an absent lockfile
      still counts as one attempt.
    - `REPO_LOCKFILE_PARSES` keeps its current meaning: incremented after a
      successful read and before parsing.
    - Do not infer lockfile reads from `FS_FILE_OPENS`.
- [ ] **R4 — `lockfile_match: None` wire shape is unchanged.**
    - Do not add a "not requested" state. Document the new meaning on the field
      and on the request API.
- [ ] **R5 — Typed Cargo parser leniency.**
    - Malformed entries must be skipped, not rejected. This covers an absent
      `name` or `version` and a non-string `name` or `version`.
    - A non-array `package` produces an empty index, not `None`, because the
      generic parser did that.
    - Only a document that is not valid TOML yields `None`.
    - Use lenient field deserializers (for example `Option<String>` via a
      `deserialize_with` that maps non-strings to `None`, and a lenient
      `package` field). Do not use `toml::Value`, and do not hand-roll a line
      scanner.
    - Spike S2 confirms that this parity is achievable.
- [ ] **R6 — The pnpm typed parse is conditional.**
    - Implement it only if spike S3 shows that `pnpm_lockfile_matches` costs at
      least 10% of `upgrade_provenance_with_lockfile` time on a
      pnpm-authoritative fixture in a release build.
    - Otherwise, record the measurement and leave it out.
- [ ] **R7 — Worktree hygiene.**
    - The worktree already contains uncommitted changes from other streams, such
      as `nested.rs` worker-cap work and the `2026-09-20-repo-perf` docs. Never
      stage those.
    - Stage explicit paths only.
    - Commits use Ken Snyder's identity, are signed, carry **no** agent
      attribution trailers (repo `CLAUDE.md` overrides the harness default), and
      are checked with `git verify-commit HEAD`.
- [ ] **R8 — Measurement baseline tree.**
    - Take the baseline on `HEAD` with `2026-09-20-repo-perf` already applied,
      as the spec requires ("measure after that fix").
    - Record in `evidence/environment.md` whether the uncommitted `nested.rs`
      worker-cap change was present in the measured binaries. Use the same tree
      state for the baseline and changed runs.

### Wave 1 — independent investigations (parallel)

- [ ] **Consumer audit (AC1)**
    - Using text search (and optionally GitNexus `context` or `impact`), list
      every reader of `MonorepoLayer::provenance`, `PackageSeed::provenance`,
      `Package::provenance`, and `lockfile_match` across the workspace.
    - List every caller that serializes a complete `RepoInfo`.
    - Include the Sniff CLI (`sniff/cli/src/commands/repo.rs`,
      `output/repo_json.rs`, `output/filesystem/mod.rs`), Claudine
      (`claudine/lib/src/composition/resolve.rs`, `invocation_context.rs`,
      `system_prompt/context.rs`, `events/environment.rs`,
      `claudine/cli/src/completion/scopes.rs`,
      `commands/wrap/composition/prep_context.rs`), Darkmatter
      (`darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs`,
      `darkmatter/cli/src/commands/compose.rs`), `sniff/lib/src/filesystem/docs.rs`,
      `git/recent_commits/collect.rs`, `identity.rs`, and any
      `scripts/ci/*.py` or justfile that consumes `sniff repo --json` (confirm
      that "provenance" hits there are unrelated CI-evidence provenance).
    - For each caller, record its request tier (direct `detect_repo_structure`,
      `detect_repo_structure_or_root_package`, `RepoRequest::structure()`, a
      focused request, or full) and whether it depends on the old value.
    - Write this as a table in `implementation-log.md`, then resolve R1 and
      record the outcome.
- [ ] **S1 — Re-verify the ungated load sites**
    - Enumerate every call to `ManifestStore::cargo_lock`, `pnpm_lock`,
      `uv_lock`, `read_counted_lockfile`, and `CargoLockVersions::parse`.
    - Confirm that in the structure tier only `upgrade_provenance_with_lockfile`
      reaches them. Today the others are `detection.rs:174-178` and
      `lock_versions_for_seed`, both gated on `wants_dependencies()`.
    - Also check the nested and polyglot paths and the
      `detect_repo_structure_or_root_package` synthesis path.
    - Record the list in the log. Any newly found ungated site becomes a Phase 3
      task.
- [ ] **S2 — Typed-parser feasibility spike**
    - In a scratch test (not committed), deserialize this checkout's
      `Cargo.lock` and a malformed fixture into the R5 typed shape through
      `biscuit_file::toml_crate`.
    - Confirm three things: lenient fields skip bad entries without failing the
      document; a non-array `package` yields an empty index; unknown keys
      (`source`, `checksum`, `dependencies`) are ignored.
    - Take a quick release timing against the generic parse.
    - Record the result, or the reason R5 must change, in the log.
- [ ] **S3 — pnpm cost spike (feeds R6)**
    - Build or choose a pnpm-authoritative fixture with a realistically sized
      `pnpm-lock.yaml`. Generate it in a temp dir; do not commit a large
      lockfile.
    - Measure the share of `pnpm_lockfile_matches` versus the whole provenance
      step in a release build, and record the R6 decision.
- [ ] **Measurement seam and harness**
    - Reuse `2026-09-20-repo-perf`'s harness: `sniff/lib/examples/work_counts.rs`,
      `sniff/lib/benches/cases/repo.rs`, and its `evidence/after/bracket.sh.txt`
      and `summarize.py.txt`.
    - Identify how to time `upgrade_provenance_with_lockfile` in isolation. For
      example, add a crate-private or `#[doc(hidden)]` measurement seam in the
      style of that fix's "measurement-only exposure seam", or a bench row that
      times `CargoLockVersions::parse` plus the three match functions.
    - Any seam must be a no-behavior-change commit.

### Wave 2 — baseline measurement (depends on Wave 1 harness task)

- [ ] **Baseline timings**
    - Build and hash the baseline binaries (debug and release) from the R8 tree.
    - Using the spec's protocol (≥3 warmups, ≥20 samples, median and range,
      warmed-cache labels, and the 1-minute load average with every sample set),
      record the following:
        - provenance step in isolation;
        - public `detect_repo_structure(<repo root>)` (today this always
          corroborates);
        - one ambient `md compose` of a trivial document.
    - Store the results under `evidence/baseline/`.
    - Keep the baseline binaries so that Phase 5 can alternate them with the
      changed binaries.

### Phase 1 checkpoint

- [ ] R1–R8 resolved and logged. The audit table exists, and S1–S3 are
      recorded. Baseline evidence is committed as a docs-only commit (fix
      directory, plus the seam if one was added, in its own commit).

## Phase 2 — Mechanism foundations

These changes do not alter behavior: every existing request keeps corroborating,
because nothing reads the new setting yet.

### Wave 3 — parallel, disjoint files

- [ ] **Request setting** (`sniff/lib/src/request.rs`)
    - Add the R2 field, its serde default, the accessor, and the builder.
      `structure()` sets it to false, `full()` to true, and `focused(..)` to
      false. Update the constructor docs.
    - Document on the field that the setting controls corroboration only, and
      that a dependency request may still read `Cargo.lock`.
    - Unit tests in `request.rs`:
        - legacy JSON without the field → `wants_lockfile_provenance()` is true;
        - `structure()`, `focused(..)`, and `full()` defaults;
        - builder opt-in on `focused` and `structure`;
        - the field is always present in serialized output;
        - extend the existing round-trip tests near `request.rs:1244-1285`.
- [ ] **Read-attempt counter** (`performance/counters.rs`,
      `filesystem/repo/detection.rs` `read_counted_lockfile`, and
      `filesystem/repo/manifest_index.rs` `CargoLockVersions::parse`)
    - Add `REPO_LOCKFILE_READS` with a doc comment stating the attempt semantics
      from R3.
    - Increment it before the read in both sites.
    - Register it wherever counters are listed or rendered: check
      `sniff/cli/src/output/perf_tree.rs` fixtures and any counter catalog or
      docs.
    - Extend `cargo_lock_is_shared_by_provenance_and_dependency_enrichment`
      (`detection.rs:3108`) to assert exactly 1 read.
    - Add a test for an absent lockfile: 1 read attempt and 0 parses.

### Phase 2 checkpoint

- [ ] `cargo nextest` for the `sniff` lib passes with no changes to existing
      assertions apart from the additions above. `just lint` in `sniff/` is
      clean. Commit both changes, as one commit or two, with signed commits.

## Phase 3 — Behavior changes

### Wave 4 — parallel, disjoint files

- [ ] **Gate corroboration** (`filesystem/repo/detection.rs`)
    - Wrap the `upgrade_provenance_with_lockfile` loop in
      `if request.wants_lockfile_provenance()`.
    - Update that function's docs and the inline comment above the loop to say
      that the step is request-driven.
    - Fix any ungated site found in S1.
    - Update `structure_mode_performs_zero_enrichment_work`
      (`detection.rs:3226`), or add a sibling test, that uses a fixture carrying
      all three lockfiles and asserts `REPO_LOCKFILE_READS == 0` and
      `REPO_LOCKFILE_PARSES == 0` for a declining structure request.
    - Add the counter-part test: an opt-in structure request performs the same
      reads and parses as the old unconditional path.
    - Add a test that a focused dependencies request that declines
      corroboration still reads `Cargo.lock` once and resolves versions.
- [ ] **Typed Cargo parser** (`filesystem/repo/manifest_index.rs`)
    - Implement the R5 typed deserialization in `CargoLockVersions::parse`,
      keeping `HashMap<String, Vec<String>>` in lockfile order and the counter
      placement from R3.
    - Move the current body into a `#[cfg(test)] fn parse_reference(..)`.
    - Add parity tests (AC4):
        - a duplicate-names-at-multiple-versions fixture (order preserved;
          `resolve` returns the first version);
        - a malformed-entries fixture (missing name, integer version, non-table
          entry, non-array `package`);
        - invalid TOML → `None`;
        - this checkout's `Cargo.lock`, with a full map equality between the
          typed and reference results.
    - For the repository-file read, spell the path in a form the
      `rust-testing` skill lists, so CI's test-input scoping sees it (see
      `docs/cicd/test-inputs.md`).
    - Update the `parse` docs to state the retained-fields contract.

### Wave 5 — depends on Wave 4 gate task

- [ ] **Structure default** (the R1 outcome; public API docs in
      `filesystem/repo/types.rs`)
    - **Option A:** no code change beyond Phase 2's `structure()` default.
      Update the `detect_repo_structure`, `detect_repo_structure_or_root_package`,
      and `detect_repo` docs: structure omits corroboration, and
      `detect_repo_with_request` plus `with_lockfile_provenance(true)` opts in.
    - **Option B:** make `structure()` default to true instead, and migrate
      Darkmatter's `snapshot.rs` capture to `detect_repo_with_request` with a
      declining structure request.
    - Move each consumer flagged by the audit to an explicit request.
- [ ] **Result docs** (`filesystem/repo/types.rs` and `standard.rs`)
    - Update the `MonorepoLayer::lockfile_match` and `provenance` field docs to
      describe the three meanings of `None` (R4) and that provenance stays
      manifest-derived when corroboration is not requested.
- [ ] **Existing tests follow the policy** (`sniff/lib/tests/l1/integration.rs`
      around line 728, plus any detection unit test that expects `Lockfile`
      provenance from a structure call)
    - Under A, move these tests to explicit opt-in requests, and add assertions
      that a plain structure call now reports manifest-derived provenance and
      `lockfile_match: None`.

### Wave 6 — fixture matrix and CLI (parallel; depends on Wave 5)

- [ ] **Provenance fixture matrix (AC3)** — this belongs in the existing
      consolidated `l1` `integration` module, not a new test target.
    - Build temp-dir fixtures for each authority (Cargo, pnpm, uv) × lockfile
      state (matching, extra lockfile member, missing lockfile member, absent,
      unparseable).
    - For each, write the complete expected `RepoInfo` by hand (layer
      `provenance`, `lockfile_match`, and per-package provenance) and assert
      equality under a corroborating request. Do not compare only before and
      after.
    - Add an explicit assertion pair showing that Cargo with an extra lockfile
      entry gives `Some(true)`, while pnpm and uv with an extra member give
      `Some(false)`.
    - Run the same fixtures under a declining request and assert
      manifest-derived provenance with `lockfile_match: None`.
- [ ] **CLI JSON tests** (`sniff/cli`)
    - Add or update process tests to show that `sniff repo … --json` output
      parses as JSON and reflects the chosen tier: `lockfile_match` is absent or
      null and provenance is manifest-derived for structure commands under A.
    - Refresh snapshots only where the policy changed them. Nothing may go to
      stdout except JSON.

### Phase 3 checkpoint

- [ ] `just test` and `just lint` in `sniff/` are green, with `remote` feature
      coverage per the recipe. Commits are signed and verified. The behavior
      change is in a separate commit from docs-only changes.

## Phase 4 — Conditional pnpm parse, docs, and downstream packages

### Wave 7 — parallel

- [ ] **Typed pnpm parse (only if R6 says so)**
    - In `ManifestStore::pnpm_lock`, deserialize only the `importers:` key set
      (for example, a struct with `importers: IndexMap<String, IgnoredAny>`) and
      keep the current root-key normalization.
    - Add parity tests against the generic `serde_yaml_ng::Value` path, kept as
      a test-only reference, covering the matching, extra, missing, and
      unparseable cases.
    - If R6 says no, check this item off with "skipped per R6" and a link to the
      log entry.
- [ ] **Docs and skill (AC6)**
    - Update `sniff/lib/README.md` (structure output and the `monorepo_layers`
      description near line 684) and `sniff/cli/README.md` where structure
      commands are described.
    - Update `.claude/skills/sniff/performance.md` with the request-cost text:
      lockfile corroboration is a request option, and the new counter's name and
      meaning.
    - Update `.claude/skills/sniff/SKILL.md` or
      `remote-and-repository.md` if they describe structure-tier provenance.
- [ ] **Downstream verification (AC5)**
    - Run `just test` in `darkmatter/`, including the three
      observation-boundary tests named in `2026-09-20-repo-perf`. Confirm that
      the `REPOSITORY_DISCOVERY_COUNT` expectations are untouched.
    - Run the affected Claudine tests (completion scopes, composition resolve,
      invocation context, and environment events).
    - Fix only breakages caused by this change, and log the results.

### Phase 4 checkpoint

- [ ] Sniff, Darkmatter, and Claudine tests are green. Docs match the behavior.
      Signed commits exist.

## Phase 5 — Measurement, cross-OS evidence, and close-out

### Wave 8 — after-measurements (sequential; one host, alternated runs)

- [ ] **After timings**
    - Alternate the kept baseline binaries with the changed binaries using the
      Phase 1 harness and the same protocol and tree state (R8).
    - Measure, in both debug and release:
        - the provenance step in isolation, before and after change 2;
        - `detect_repo_structure` with provenance requested and with it
          declined;
        - one ambient `md compose` before and after.
    - Record load average with every set, and store the results under
      `evidence/after/`.
- [ ] **Results write-up**
    - Write `results.md`, reporting the parser, detection, and compose effects
      separately. Do not present the parse speedup as the detection speedup, or
      the detection speedup as the suite speedup.

### Wave 9 — cross-OS and close-out (parallel)

- [ ] **Cross-OS evidence (AC7)**
    - Load the `os` skill.
    - Reuse qualifying evidence for macOS and Linux from the pull-request
      workflow.
    - Windows evidence comes from the post-merge `main` push and WSL2 evidence
      from the nightly run, unless `ci:all-os` is applied. Alternatively, run
      the Sniff L1 suite on the build hosts that the `os` skill lists
      (`BatchMode=yes` SSH aliases only).
    - Pay particular attention to Windows path handling in the fixture matrix,
      because the lockfile paths are joined from the layer root.
    - Add no new CI cell, fixture gate, or timing gate.
- [ ] **Implementation log and spec frontmatter**
    - Complete `implementation-log.md` with the audit, rulings, spike results,
      commands run, and commit SHAs.
    - Set the spec's `status: implemented`, `implemented: true`, and
      `implemented_by`.
    - Record any comment/code drift that was found and how it was resolved.
    - Stop at "implementation complete, ready for review".

### Phase 5 checkpoint

- [ ] Every acceptance criterion (AC1–AC7) is linked to evidence in
      `implementation-log.md`. `results.md` exists, all commits pass
      `git verify-commit`, and no unrelated worktree changes are staged.

## Dependency and Parallelism Summary

| Wave | Phase | Tasks | Depends on |
|---|---|---|---|
| 1 | 1 | Audit, S1, S2, S3, harness | — |
| 2 | 1 | Baseline timings | Wave 1 harness |
| 3 | 2 | Request setting ‖ Read-attempt counter | Phase 1 rulings |
| 4 | 3 | Gate corroboration ‖ Typed Cargo parser | Wave 3 (gate needs setting and counter; parser needs counter placement) |
| 5 | 3 | Structure default, result docs, existing tests | Wave 4 gate; R1 outcome |
| 6 | 3 | Fixture matrix ‖ CLI JSON tests | Wave 5 |
| 7 | 4 | Typed pnpm (conditional) ‖ Docs and skill ‖ Downstream tests | Phase 3 |
| 8 | 5 | After timings, then results | Phases 3–4; Wave 2 binaries |
| 9 | 5 | Cross-OS ‖ Close-out | Wave 8 |

File-conflict notes for parallel waves:

- In Wave 3, the counter task edits `detection.rs` only in
  `read_counted_lockfile` and `manifest_index.rs` only in `parse`. The request
  task edits only `request.rs`.
- In Wave 4, the gate task owns `detection.rs` and the parser task owns
  `manifest_index.rs`.
- Change 2 (the typed parser in Wave 4) and the Phase 2 mechanism do not depend
  on the R1 policy decision. They may proceed even if R1 escalates to the
  author; only Wave 5's "Structure default" task waits on R1.
