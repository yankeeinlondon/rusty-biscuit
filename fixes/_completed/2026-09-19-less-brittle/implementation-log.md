---
kind: implementation-log
fix: 2026-09-19-less-brittle
deferred_perf_measurement: false
implementation_1: "2026-09-20T00:09:56-07:00"
implementation_2: "2026-09-20T04:56:08-07:00"
implementation_3: "2026-09-20T09:32:13-07:00"
implementation_4: "2026-09-20T11:06:02-07:00"
implementation_5: "2026-09-20T11:53:00-07:00"
---

# Implementation Log — `2026-09-19-less-brittle`

## Implementation of Review Findings #1

> **started at:** 2026-09-20T00:09:56-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-unifi/fixes/2026-09-19-less-brittle/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review metadata
        - reviewer: `codex/gpt-5.6-luna`
        - findings: 5, all at `high` priority
        - spec under review: `2026-09-19-less-brittle/spec.md`
- package areas in scope (derived from the spec's **Decisions** and **Acceptance** sections)
        - `tools/test-toolkit` — the guard implementation, matcher, scanner, and fixtures
        - `scripts/ci` (`repo-deps`) — the canonical planner `affected_scope.py` and its tests
        - `.github/workflows` + `just/` — guard selection, execution, and reporting
        - `claudine`, `darkmatter/dmls`, `messenger` — the twelve migrated fixture sites

### Finding 1 — The twelve known archive-path violations remain unfixed

- starting the work on 'twelve known archive-path violations' at 00:12:04-0700
- **already satisfied on this branch.** The review was taken against a checkout that
  predates commit `9b797b61a` (`test(packages): route twelve fixture paths through
  manifest_dir!`), which is merged into `fix/ci-worker-budget` through `cd8076515`
        - verified: `cargo nextest run -p test-toolkit --test archive_path_guard` →
          **2 passed, 0 failed** (previously 1 passed / 1 failed with twelve sites named)
        - verified: no `env!("CARGO_MANIFEST_DIR")` remains in any of the twelve files;
          all now call `biscuit_test_harness::manifest_dir!()`
        - verified: `messenger/lib/Cargo.toml:51` declares the direct
          `biscuit-test-harness` dev-dependency the review asked for
        - verified: the guard passes with **no new allowlist entries** — `ALLOWED` is
          unchanged at five entries, all still live
- the one sub-part of this finding still outstanding is the *relocation coverage*, which
  is carried into the Finding 5 work rather than duplicated here
- work completed for 'twelve known archive-path violations' at 00:17:36-0700

### Finding 2 — The guard still performs unsafe text matching with file-wide exemptions

- starting the work on 'token-aware guard matcher' at 00:22:37-0700
- moved the guard's logic out of the test binary into a library module so it can be
  unit-tested without scanning the repository
        - `tools/test-toolkit/src/archive_guard.rs` (new, declared from `lib.rs`) — lexer,
          matcher, scanner, plan reader, exemption rules
        - `tools/test-toolkit/src/archive_guard/matcher_tests.rs` (new) — the fixture corpus,
          kept in its own file because every fixture necessarily *spells* a forbidden form
        - `tools/test-toolkit/tests/archive_path_guard.rs` rewritten as a 86-line driver:
          resolve the plan, scan, print the mode, assert
- replaced the "blank `//` comments, then `str::contains`" scan with a one-pass lexer
        - drops line comments, block comments (nested), and treats string literals,
          raw strings (any hash count), byte/C strings, and char literals as opaque tokens
        - lifetimes are distinguished from char literals, so `<'a>` does not desynchronize
          the scan; numbers are consumed whole
        - a forbidden form is now the *token sequence* `env` `!` `(` string `)`, which makes
          the multiline and extra-whitespace spellings match and makes prose and fixture
          corpora stop matching
        - line numbers come from a precomputed line-start table, so every reported line is
          the line the invocation starts on
- expression-level safe-fallback recognition, documented as a closed set in the module doc
        - two tails (`T1` `.map(PathBuf::from).unwrap_or_else(…)`, `T2` `.map_or_else(…)`),
          two manifest shapes (`M1`/`M2`), two binary shapes (`B1`/`B2`)
        - the empty-value rejection `.filter(|v| !v.is_empty())` is required by every shape,
          with the closure binding checked as a backreference — a shape that omits it
          disagrees with `manifest_dir!`/`bin_exe!` and stays a violation
        - the binary shapes require nextest's variable first, the `-`→`_` mangling to agree,
          and all three spellings to name the same target
        - exemption attaches to the single `env!` token the shape consumed, so a second
          unguarded occurrence in the same file is still reported, at its own line
- the hosted-root check is its own pass over string-literal tokens
        - it now also catches a raw-string spelling, which the old `"\"/home/runner/work/"`
          needle missed, and no fallback recognition can suppress it
- scanner hardening
        - `collect_rust_files` returns `Result` and fails with the path when an existing
          directory cannot be listed or a file cannot be read; the old code returned early
          on both and reported a pass it had not verified
        - directory descent compares **canonicalized** paths against a canonicalized
          checkout boundary and keeps a visited set, so a symlink out of the tree is not
          followed and a cycle terminates — macOS's symlinked `/tmp` and Windows's `\\?\`
          prefixes both make raw string comparison wrong here
        - `SKIPPED_DIRS` and the `build.rs` exclusion are unchanged, reasons included
- self-exclusion is now by exact repository-relative path (`GUARD_OWN_SOURCES`), not by
  basename suffix, with a per-entry reason
        - recorded honestly as defense in depth: the token-aware matcher already ignores
          the forms these files hold in comments and string literals
- two scan modes driven by the planner's resolved plan
        - **decision:** the plan reader lives in `src/archive_guard.rs` and `serde_json`
          was promoted from `[dev-dependencies]` to `[dependencies]`. It adds nothing to
          the graph — `biscuit-test-harness`, already a regular dependency, builds it — and
          keeping the reader in the test binary would have put the contract out of reach of
          unit tests and of any future consumer
        - `BISCUIT_ARCHIVE_GUARD_PLAN` unset or empty → full tree; unreadable or malformed →
          hard error; `mode: "changed"` with an explicitly empty `paths` is a real state and
          is never a full scan
        - the driver prints `mode=… files-checked=… missing=… ineligible=… violations=…`
          before asserting
- exemption maintenance validates `ALLOWED` against a **raw** full-tree form scan in both
  modes, and also rejects duplicates, empty reasons, and entries naming a file that no
  longer exists (with a diagnostic that names the rename case)
- verification
        - `cargo nextest run -p test-toolkit --lib` → **95 passed** (61 new: 32 matcher
          fixtures, 29 scanner/plan/exemption fixtures)
        - `cargo nextest run -p test-toolkit --test archive_path_guard` → **2 passed**;
          full-tree run reports `files-checked=3474 … violations=5`, all five inside the
          unchanged five-entry `ALLOWED`
        - `just _test test-toolkit` → **277 passed, 2 skipped**; `just test test-toolkit`
          (root selector) → same
        - `just _lint test-toolkit` → clean, zero warnings
        - non-vacuity proved out of band: a temporary `tests/tmp_probe/probe.rs` holding
          `PathBuf::from(env!("CARGO_MANIFEST_DIR"))` failed the full-tree run at the right
          line, passed a changed-mode plan that omitted it, failed one that listed it, and
          a malformed plan errored rather than scanning nothing; the probe was removed
- **no new `ALLOWED` entries and no new skipped directories** — the new matcher flags
  exactly the five files the old one did
- blockers / not done
        - `just lint test-toolkit` does not exist at the repository root (there is no root
          `lint` recipe and `tools/test-toolkit/justfile` carries only
          `verify-nextest-config`), so `just _lint test-toolkit` from `just/devops.just`
          was used instead
        - the planner side of the contract (Finding 3) is out of scope here; the reader is
          implemented and unit-tested against the JSON shape recorded above, so the planner
          only has to emit it
- work completed for 'token-aware guard matcher' at 00:31:49-0700

### Finding 3 — The planner does not select or provide the guard's scan scope

- starting the work on 'planner guard selection' at 00:35:12-0700
- design: the spec's Open Question is resolved to **Option 1** — explicit suite-only
  selection on the existing lint-cell path, owned by `test-toolkit`. No new top-level CI
  job, no new gate kind, no new package, no area-keyed store; cell identity stays
  `{package, environment, gate}`
- registry
        - `archive-path-guard` is the first **lint-only** companion: it declares
          `lint_recipe` and deliberately no `recipe`, so `companion_records(…, "L1")`
          never attaches it and the same scan cannot run twice on Linux under two
          different sets of evidence rules
        - `validate_suite_registry` was relaxed to accept either half, with the comment
          recording that the shape is deliberate
        - audited the other readers: `matrix_record.companion_environments` already
          filters on `entry.get("recipe")`, so the guard stays out of it; `companion_suites.py`
          consumes `companion_records` output, which resolves `lint_recipe` for the lint
          gate, so it cannot crash on the missing key
- trigger policy — two narrow rules and nothing else
        - **scanned source**: ends `.rs`, basename is not `build.rs`, and no path component
          is in `ARCHIVE_GUARD_SKIPPED_DIRS` (mirrors the Rust `SKIPPED_DIRS`). Rust files
          outside workspace members trigger; removed `.rs` files trigger, because exemption
          maintenance must still run
        - **owned inputs**: five exact paths, including `scripts/ci/affected_scope.py`
          (which `scripts/` would otherwise skip) and `tools/test-toolkit/justfile`
        - documentation, manifests, lockfiles, and workflow YAML select no guard
- deletions: `git diff --name-only` cannot tell a deletion from a missing file, so the
  planner's input contract was extended rather than the distinction guessed — a repeatable
  `--deleted PATH`, threaded into `change_inventory` (new `deleted` key, present exactly
  when `diff_available`) and into the guard's scope
- guard-only cell: when `test-toolkit` is not otherwise selected, `package_cells` is handed
  a policy with no tier and no target kind, which makes exactly one cell exist —
  `{test-toolkit, ubuntu-latest, lint}` — carrying `companions == ["archive-path-guard"]`
  and the new optional cell field `companions_only: true`. Lint cells are already
  `reusable=False`, which is what the spec's "initially prefer non-reusable guard
  execution" asks for
- schema bumped 4 → 5 in lockstep: `RESOLVED_PLAN_SCHEMA_VERSION`, `ci-rollup.rs`'s
  `PLAN_SCHEMA_VERSION`, `.github/ci/schemas/contract.json` (regenerated through
  `python3 scripts/ci/schema.py`, which is the writer), and the schema README's table and
  prose. `validate_scope_receipt` reads the same constant, so a version-4 scope receipt
  misses once as `scope-schema`, which is the intended behavior
- cross-language policy agreement: `ci_workflow_contracts::the_guards_eligibility_policy_agrees_across_the_language_boundary`
  text-slices `ARCHIVE_GUARD_SKIPPED_DIRS` and `ARCHIVE_GUARD_OWN_INPUTS` out of the `.py`
  source and compares them to `SKIPPED_DIRS` and `GUARD_OWN_SOURCES`, including a count
  check so a directory only one side skips fails too
- tests updated rather than papered over — each is a real, intended behavior change
        - every Rust change now also selects `test-toolkit` (one Linux lint cell), so
          `RealWorkspaceAreaFanOutTests`, `RealWorkspaceRetirementScopeTests`,
          `AreaGroupingTests`, `SelectionTests`, `ResultCompletenessTests`,
          `WorkflowScopeStepTests`, and `ci-rollup-tests::the_real_planners_plan_rolls_up`
          gained `tools` / `test-toolkit` to their expected lists
        - `scripts/ci/affected_scope.py` is a guard-owned input, so
          `test_ci_tooling_change_schedules_its_owner_and_nothing_else` now expects
          `["repo-deps", "test-toolkit"]`
        - `test_a_recipe_less_suite_fails_validation` had to blank BOTH halves, because
          `homelab-frontend` still lints
        - five hand-written plan fixtures gained `archive_guard` through the new
          `plan_fixtures.archive_guard()` helper
- new tests: 24 planner cases (`ArchiveGuardScopeTests`, `ArchiveGuardOnlyCellTests`),
  17 schema cases (`ArchiveGuardValidationTests`, plus `deleted` and `companions_only`
  placement), 4 real-workspace cases (`ArchiveGuardCorpusTests`), 1 renderer case, and
  1 cross-language contract
- verification
        - `test_affected_scope` 275, `test_schema` 129, `test_resolved_plan` 79,
          `test_ci_local` 87, `test_evidence_reuse` 76, `test_reuse_validation` 21 — all OK
        - `just test repo-deps` 417 passed; `just test test-toolkit` 278 passed
        - `just _lint repo-deps` and `just _lint test-toolkit` — zero warnings
        - round trip: planner → `/tmp/plan.json` → `BISCUIT_ARCHIVE_GUARD_PLAN` → guard
          printed `mode=changed(2 listed) files-checked=2 missing=0 ineligible=0
          violations=0` and echoed the planner's reason verbatim
        - standalone `just archive-path-guard` printed `mode=full-tree
          files-checked=3474 … violations=5` (the five unchanged `ALLOWED` entries), and
          the same recipe with the variable exported printed the changed-file scope, so
          the pass-through is environment inheritance and not a second diff
- blockers / not done, all belonging to Finding 4
        - **no workflow was touched.** `matrix_record` now emits `lint_companions_only`
          and keeps `companion_suites` on a companions-only lint selection, but
          `_package-ci.yml` does not yet read either: its lint job still runs Clippy
          unconditionally. Without that wiring a guard-only cell would lint a package no
          change selected — the extra Clippy is wasted work, not a false pass
        - the coverage audit, receipt identity, and `ci-gate` folding for the guard cell
          are likewise Finding 4's
        - `legacy_scope_document` does not project `archive_guard`;
          `SCOPE_PROJECTION_FIELDS` is a closed list the scope receipt validates against,
          and the guard reads the PLAN, which the receipt already carries
- work completed for 'planner guard selection' at 01:16:59-0700

### Finding 4 — Guard execution is not integrated into CI ownership, reporting, or local validation

- starting the work on 'CI ownership, reporting, and local validation' at 01:17:20-0700
  (reconstructed: this agent was launched immediately after the Finding 3 section closed
  at 01:16:59-0700; every other timestamp here is a real `date +%H:%M:%S%z` reading)
- verified each of Finding 3's hand-offs before acting on it
        - `_package-ci.yml` read neither `lint_companions_only` nor `companion_suites`
          in its lint job; the Clippy step carried no `if:` at all
        - no coverage-audit, receipt-identity, or gate wiring existed for the guard cell
- **workflow: threading `lint_companions_only`**
        - `ci.yml` needed no change — it hands `_area-ci.yml` the whole matrix document
          (`toJSON(fromJSON(needs.scope.outputs.area_matrix)[matrix.area])`), so a new
          matrix field travels without a second projection
        - `_area-ci.yml` gained `lint-companions-only: ${{ matrix.lint_companions_only }}`
          beside `companion-suites`, the route the task named
        - `_package-ci.yml` gained a `boolean` input defaulting `false`, which is every
          ordinary lint cell; `matrix_record` always emits the key, so the expression is
          never empty
- **the Clippy skip**
        - `- name: Lint / id: clippy` now carries `if: ${{ !inputs.lint-companions-only }}`
        - gated, never deleted: an ordinary lint cell is byte-identical to before
- **the fold — the part that had to be exactly right.** Implemented in
  `Record producer status`, with a new `COMPANIONS_ONLY: ${{ inputs.lint-companions-only }}`
  env entry and the invariant written above it:

        ```bash
        companion_uncovered=""
        result="$JOB_STATUS"
        if [ "$COMPANIONS_ONLY" = "true" ]; then
          if [ "$result" = "success" ] && [ "$COMPANION" != "success" ]; then
            result=failure
            companion_uncovered=1
            echo "::error …::companion lint: ${COMPANION}. This cell runs no clippy of its
        own, so a companion that did not succeed leaves it with no evidence."
          fi
        elif [ "$result" = "success" ] && { [ "$GATE" = "failure" ] || [ "$ZED" = "failure" ] || [ "$COMPANION" = "failure" ]; }; then
          result=failure
          echo "::error …::clippy: ${GATE}; Zed extension: ${ZED}; companion lint: ${COMPANION}."
        fi
        ```

        - on a companions-only cell the companion's outcome **is** the cell's and only
          `success` passes — `!= failure` would pass a companion that never ran
        - the skipped Clippy is read as neither: as a failure it would block every pull
          request touching a Rust file, as a pass it would greenlight a cell that ran
          nothing
        - the ordinary branch is untouched, so no existing cell changes meaning
        - `detail` gained a companions-only wording; the old string claimed "clippy's
          result does not cover them", and on this cell there is no clippy result
- **duration/telemetry**: nothing divides by the measurement. The step writes
  `duration_s=<elapsed>` from a `python3` child; a skipped step writes nothing, and the
  status step's `jq` already drops an empty `$duration` rather than coercing it. Only the
  stale comment needed repair — it said the absent case was "a cancelled job", which is
  now also a companions-only cell
- **the plan artifact, and the decision the task asked me to state**
        - new `Download the resolved execution plan` step in the lint job, gated on
          `!cancelled() && inputs.companion-suites != '[]'` — the same condition as the
          companion step that reads it, so a lint cell with no companion pays no transfer
        - **chosen: a missing plan is an ERROR**, not an `if`-gated variable.
          `download-artifact` fails the step when `ci-resolved-plan` is absent, and
          `BISCUIT_ARCHIVE_GUARD_PLAN` is exported unconditionally so a file that arrived
          unreadable makes the guard hard-error (`GuardError::PlanUnreadable`). The
          rejected alternative — export only when the file exists — turns a lost artifact
          into a full-tree scan **recorded as this cell's evidence** for a run that never
          had a plan, which is precisely the false evidence the cell exists to prevent
        - the path is **absolute** (`${{ github.workspace }}/…`). The guard opens it
          verbatim and neither the recipe (`cd tools/test-toolkit && …`) nor nextest's test
          binary runs in the workspace directory the artifact lands in — a relative path
          would have silently resolved nowhere
        - **leakage, checked rather than assumed**: `companion_suites.py`'s `run_one` does
          `subprocess.run(command, shell=True, cwd=root)`, so the variable reaches every
          lint companion in that one process. Inert today — the only other lint-half suite
          is `homelab-frontend`, whose `lint-frontend` recipe never reads it — and the
          reliance is recorded in a comment beside the `env:` entry
- **rollup (`scripts/ci-rollup.rs`)**
        - `PlanCell` gained `#[serde(default)] companions_only: bool`; `plan_expected_cells`
          carries it onto a new `ExpectedCell::companions_only`
        - `companion_lint_downgrade` gained the flag. The downgrade itself was already
          correct — R12's `companion_problems` fails a green lint whose declared companion
          produced no success evidence — so the flag sharpens the REASON rather than the
          verdict: such a cell "evidenced nothing at all", which is a different finding
          from an uncovered companion
        - `status_cells`'s `Missing` path needed no change and was verified rather than
          assumed: a planned guard-only cell with no producer status reaches
          `CellState::Missing` / `Origin::Unproduced` and blocks with rule `cell-missing`
        - the second caller (a status with no plan cell behind it) passes `false`, with the
          reason recorded inline
- **local validation (`just/ci-local.just`)**
        - the per-package loop reads `lint_companions_only` from the legacy matrix and
          SKIPS `just _lint` for a companions-only cell, matching CI's skipped Clippy
        - a new block runs the guard whenever the resolved plan attached
          `archive-path-guard` to that package's lint cell — read from `.cells[].companions`,
          so it covers both the guard-only cell and an ordinary `test-toolkit` selection
        - it shells `scripts/ci/companion_suites.py`, the SAME runner CI's lint job uses,
          so the recipe string stays the registry's single canonical spelling
        - `BISCUIT_ARCHIVE_GUARD_PLAN` names `${plan_file}` — the plan this run just
          resolved (an absolute `mktemp` path), never a re-derived diff
        - on a host that is not `ubuntu-latest` it prints, before running, that the planned
          `test-toolkit/ubuntu-latest/lint` execution **remains OUTSTANDING**, and the
          summary line carries the same words
        - **no receipt machinery was added, and none is needed**: a lint cell is already
          `reusable: false` in the plan (verified against the real planner in
          `test_a_guard_only_lint_cell_is_not_reusable`), which is the spec's "initially
          prefer non-reusable guard execution". Recorded as a comment rather than code
        - `--plan`'s jq fallback gained an `Archive-path guard: …` line at parity with
          `ci-plan`'s `ArchiveGuard::summary`, so both surfaces distinguish a changed-file
          scan from a full-tree one
- **new tests**
        - `scripts/ci-rollup-tests.rs` (3): `a_planned_guard_only_lint_cell_that_never_ran_is_missing_and_blocks`,
          `a_guard_only_lint_cell_with_no_companion_result_fails_rather_than_passes`,
          `a_guard_only_lint_cell_passes_on_its_companion_alone`, with a shared
          `companions_only_lint_cell_json` fixture
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` (3):
          `a_failing_archive_path_guard_blocks_the_merge_through_the_existing_fold`
          (asserts `TARGET_CI_JOBS.len() == 8` and `GATED_JOBS.len() == 6` — the arrays that
          would catch a seventh top-level job — plus each link of the fold chain and the
          exact companions-only comparison),
          `a_guard_only_selection_runs_the_guard_and_nothing_else` (registry has
          `lint_recipe` and NO `recipe`; the Clippy step's `if:` anchored to its own three
          lines; the download step gated, unignored, and carrying no `continue-on-error`),
          `the_guards_lint_recipe_and_its_just_tuple_name_the_same_recipe`
        - `scripts/ci/test_ci_local.py` (5, class `ArchiveGuardLocalExecutionTests`):
          the guard runs once against the plan the run resolved, the registry's recipe is
          the canonical one, a companions-only cell does not lint its package, a non-Linux
          host reports the outstanding Linux execution, and the guard-only cell is not
          reusable. `CiLocalTests.run_recipe` gained three optional parameters
          (`extra_matrix`, `plan`, `capture`) and two stubs; every existing caller is
          unchanged
- **`validate_package_ci` coverage, checked before duplicating it**: it already proves the
  `just` tuple names a recipe `tools/test-toolkit/justfile` DEFINES (anchored regex, so
  `archive-path-guard-watch:` would not satisfy it). What it cannot see is `lint_recipe`
  and the tuple drifting apart, so only that narrow gap was added
- **documentation**
        - `.github/ci/README.md` — extended the existing `### The archive-path guard`
          section with `#### How the cell executes` (the three reads of the input, the
          missing-plan decision, the leakage note, the fold chain, and MISSING/Fail
          enforcement), `#### What the scan does not cover` (every skipped directory named,
          with `scripts` explicitly NOT described as unexecuted), and `#### Evidence`
          (non-reusability and the `ci-local` behavior)
        - `tools/test-toolkit/justfile` — the canonical-recipe comment now records that this
          is the one spelling, who reaches it, and the same skipped-directory limitation
        - `.claude/skills/rust-devops/ci-cd.md` — extended Finding 3's paragraph with the
          three workflow reads, the missing-plan rule, the gate path, and local validation
- **drift found and repaired**: `.github/ci/README.md` said "`ci.yml` defines exactly six
  top-level jobs" and listed six, while `TARGET_CI_JOBS` has held eight since the
  single-OS-compile fix added `build` and `area-drift`. The code is correct and the prose
  was stale; the table now lists all eight and names the contract that pins them. Called
  out here because it predates this fix and now directly backs an assertion added by it
- verification (all on this checkout, macOS host)
        - `actionlint .github/workflows/*.yml` → rc 0, no output
        - `just test test-toolkit` → **281 passed, 2 skipped**
        - `just test repo-deps` → **420 passed, 1 skipped**
        - `cargo nextest run -p repo-deps --bin ci-rollup` → **243 passed** (the rollup suite
          is `#[path]`-included into the binary, so `--bin ci-rollup` is the selector;
          `--test ci-rollup-tests` does not exist)
        - `test_ci_local` 92, `test_affected_scope` 275, `test_schema` 129,
          `test_resolved_plan` 79, `test_evidence_reuse` 76, `test_local_evidence` 20,
          `test_reuse_validation` 21 — all OK
        - the repository's compact CI contract recipes, per `.github/ci/README.md`:
          `python3 scripts/ci/test_reuse_validation.py` (21 OK), `actionlint`, and the
          `ci_workflow_contracts` nextest suite (inside `just test test-toolkit`)
        - `just _lint test-toolkit` and `just _lint repo-deps` — zero warnings
        - `just ci-local --plan` rendered
          `Archive-path guard: changed-file scan of 5 file(s) — …`
        - `just ci-local --lint-only test-toolkit` ran the guard end to end through
          `cd tools/test-toolkit && just archive-path-guard`, printed
          `mode=full-tree files-checked=3474 … violations=5` and the outstanding-Linux
          notice, and summarized
          `✅ archive-path-guard test-toolkit — the planned ubuntu-latest execution remains OUTSTANDING`
        - a plan round trip through the real runner:
          `BISCUIT_ARCHIVE_GUARD_PLAN=… python3 scripts/ci/companion_suites.py --suites
          '["archive-path-guard"]' --environment ubuntu-latest --gate lint` printed
          `mode=changed(1 listed) files-checked=1 … violations=0`
- blockers / not done
        - **no CI run was triggered and nothing was committed or pushed**, per the session
          rules; every claim above is a local result
        - the guard's behavior on a real `ubuntu-latest` runner — the artifact download and
          `${{ github.workspace }}` expansion in particular — is proven by contract tests
          and by the local round trip, not by a hosted execution
        - `legacy_scope_document` still does not project `archive_guard` (Finding 3's note);
          unchanged here, because the lint job reads the PLAN artifact directly
- work completed for 'CI ownership, reporting, and local validation' at 01:41:03-0700

### Finding 5 — Required regression coverage and verification boundaries are absent

- starting the work on 'regression coverage and verification boundaries' at 01:43:28-0700
- audited the spec's **Acceptance** list bullet by bullet against what Findings 1–4
  already built, then added only what was genuinely missing — three tests and one
  relocation fixture, not a second matrix
        - **planner corpus, deletions/renames, skipped directories, owned inputs,
          documentation, explicit full scope** — COVERED by
          `test_affected_scope.py::ArchiveGuardScopeTests` (16 cases)
        - **guard-only selection adds exactly one Linux execution and no unrelated
          toolkit suite or OS cell** — COVERED by `ArchiveGuardOnlyCellTests`. Checked
          that the negatives are real assertions rather than the absence of a
          positive: `assertEqual(1, len(cells))`, `assertEqual([], …)` for every
          non-lint gate AND for `plan["builds"]`, and
          `assertEqual({"ubuntu-latest"}, …)` over the cell environments
        - **events: pull request, push, manual dispatch, WSL2-only schedule, absent
          diff, explicit empty inventory** — COVERED by `ArchiveGuardScopeTests`
        - **proven-environment reuse** — GAP. `test_the_guard_cell_is_never_satisfied_by_reuse`
          covers CELL reuse; nothing covered the event-level rule. Added
          `ArchiveGuardScopeTests::test_a_proven_linux_environment_gains_no_guard_execution`:
          a push whose pull request already proved `ubuntu-latest` narrows to
          `windows-latest`, and the guard accepts that the scan already ran rather
          than inventing the extra Linux run the spec forbids
        - **no consumer independently derives source scope** — GAP. Every existing
          assertion was positive (the variable names the plan); nothing would have
          caught a second `git diff`. Added
          `ci_workflow_contracts::no_guard_consumer_derives_its_own_source_scope`,
          which pins that exactly ONE job in `.github/workflows/*.yml` takes a
          source diff (`ci.yml:scope`, the job that resolves the plan) and that the
          canonical recipe, `_package-ci.yml`'s lint job, and `ci-local.just`'s
          guard block each take a diff of none. A second diff is the silently-wrong
          failure: it produces a plausible list against a base the planner never
          used, the scan passes, and the cell records that as its evidence
        - **no Linux added to the nightly** — COVERED by
          `test_the_nightly_adds_no_linux_for_the_guard`
        - **matcher fixtures** (valid fallback, empty values, runtime read elsewhere,
          different variable/binary, binary precedence, second unsafe occurrence,
          multiline, comments, strings, independent hosted-root; diagnostics naming
          file, line, and the shared macro) — COVERED by
          `archive_guard::matcher_tests` (32 fixtures)
        - **scan fixtures** (listed/unlisted, full-tree default, malformed plan,
          read failures, deletions, rename destinations, duplicate paths,
          directory symlink boundaries) — COVERED by `archive_guard::tests`
                - the two symlink fixtures are `#[cfg(unix)]` because
                  `symlink_dir` needs Developer Mode or elevation on Windows; the
                  gap and the reason are recorded in a comment immediately above
                  them, and the boundary decision itself is platform-independent
                  (both sides compare canonicalized paths)
        - **exemption entries in BOTH scan modes** — partial GAP. `live`, `stale`,
          `renamed`, and `duplicate` each had one full-tree case, and
          `exemption_maintenance_uses_the_full_tree_during_a_changed_scan` proved
          the raw form scan ignores the mode; nothing proved the four VERDICTS are
          mode-invariant. Added
          `archive_guard::tests::every_exemption_verdict_is_the_same_in_both_scan_modes`:
          a changed-file scan listing none of the exempted files still reaches all
          four verdicts, byte-identical to the full-tree run. Threading the scan's
          own file list into exemption validation is the obvious optimization, and
          it would turn every entry outside the changed set stale on every pull
          request while the three existing tests stayed green
        - **no new exemptions or skipped directories** — verified by diff against
          `HEAD:tools/test-toolkit/tests/archive_path_guard.rs`: `ALLOWED` is the
          same five files, `SKIPPED_DIRS` the same seven entries
          (`target`, `.git`, `node_modules`, `.gitnexus`, `scripts`, `examples`,
          `fuzz`)
        - **workflow and reporting contracts** (guard failure blocks the merge,
          missing execution fails the owning audit, guard-only selection runs
          nothing else and manufactures no full-tree evidence) — COVERED by the
          three `ci_workflow_contracts` cases and the three `ci-rollup-tests` cases
          Finding 4 added
        - **documentation** — COVERED by Finding 4; this finding changed no
          behavior, so nothing drifted
- **relocation coverage — measured, then traded deliberately**
        - the natural candidate was adding `messenger` to
          `ci-build-archive-tests::slow_real_package_archives_read_their_fixtures_from_the_consumers_checkout`
          (seven of the twelve migrated sites, and the package that gained the
          `biscuit-test-harness` dev-dependency)
        - measured first, because that fixture's own doc comment records two
          concurrent relocations exhausting the temp filesystem
                - the existing two-package test: **88.7 s** end to end
                - `messenger` at its CI feature set resolves **777 crates** against
                  `test-toolkit` + `biscuit-file`'s **317 combined**
                - `cargo build -p messenger --all-features --tests` into an empty
                  target directory: **2 m 32 s and 2.9 GB** on a 16-core macOS host
        - **decision: not added.** One package would have cost more than the whole
          fixture and put 2.9 GB into a shared temp target directory on a runner
          with roughly 14 GB free — the exact failure the existing comment warns
          about. The measurement and the decision are recorded in that test's doc
          comment so the next author does not re-derive them
        - the cheaper equivalent was written regardless:
          **`messenger/lib/tests/research_relocation.rs`**, one test alone in its
          binary
                - it copies a real research fixture into a scratch checkout with a
                  marker the producer's copy does not carry, remaps
                  `CARGO_MANIFEST_DIR` onto it through `EnvGuard::set_safe`, and
                  asserts both that `manifest_dir!()` EXPANDED IN THIS CRATE
                  resolves there and that the fixture read lands on the marked copy
                - then repeats the read through
                  `biscuit_test_harness::bin_exe::manifest_dir(<absent path>)`, which
                  is the consumer's real condition: the producer's checkout is gone,
                  not merely different
                - the marker is load-bearing. Without it a resolver that ignored the
                  run-time value would read an identical file and the assertion would
                  prove nothing
                - deliberately NOT re-proving empty values or binary-name mangling:
                  `biscuit-test-harness`'s own `manifest_dir_with`/`resolve_with`
                  unit tests already own those
                - **note on the env mutation**: `manifest_dir(compiled)` takes the
                  COMPILE-TIME fallback as its argument and reads the process
                  environment for the run-time value, so the run-time half cannot be
                  passed explicitly. `EnvGuard::set_safe` plus a binary with no
                  siblings is the isolation instead of `#[serial]`, which would
                  enforce nothing under nextest's process-per-test model. Recorded
                  in the file's `//!`
        - it is gated `#![cfg(feature = "research")]` like its five siblings, so it
          also counts toward the research-feature evidence below
- verification, all on this checkout
        - `cd tools/test-toolkit && just archive-path-guard` → **2 passed**;
          `mode=full-tree files-checked=3475 missing=0 ineligible=0 violations=5`
          (3474 → 3475 is the new messenger test file; the five violations are the
          unchanged `ALLOWED`)
        - `just test test-toolkit` → **283 passed, 2 skipped** (281 → 283 is the two
          new tests)
        - focused package recipes for the migrated sites
                - `just _test dmls --features effects-instrumentation` → **715 passed**
                - `just _test claudine` → **4325 passed**
                - `just _test claudine-cli` → **2759 passed, 9 skipped**
                - `just _test_local_all "messenger --features desktop,research; messenger-cli"`
                  → **688 passed, 2 skipped**
        - **the research feature actually executed its modules**, counted rather
          than assumed: 126 tests in research test binaries ran —
          `research_corpus` 34, `research_refresh` 23, `research_validation` 21,
          `research_publication` 19, `research_lifecycle` 9,
          `research_relocation` 1, `research_cli` 9, `research_lifecycle_cli` 10.
          The counterfactual: `cargo nextest list -p messenger` WITHOUT the feature
          lists **15**, all of them the ungated half of `research_corpus` — so 92 of
          the library's 107 research tests exist only with it
        - compact CI contract recipes: `python3 scripts/ci/test_reuse_validation.py`
          → 21 OK; `actionlint .github/workflows/*.yml` → rc 0, no output; the
          `ci_workflow_contracts` suite inside `just test test-toolkit`
        - `test_affected_scope` 276, `test_schema` 129, `test_resolved_plan` 79,
          `test_ci_local` 92, `test_evidence_reuse` 76, `test_local_evidence` 20 —
          all OK
        - `just _lint test-toolkit`, `just _lint repo-deps`, `just _lint messenger-cli`,
          and messenger's own `cargo clippy -p messenger --features desktop,research
          --all-targets -- -D warnings` — zero warnings
- **cross-OS**
        - the guard walks the filesystem, resolves symlinks, and normalizes paths,
          and its CI cell is `ubuntu-latest`, so Linux was worth real evidence
        - `just cross-check test-toolkit --os linux` **could not run**: the rig's
          `ci-verification/.cross-check.lock` has been held since
          `2026-09-14T18:25:30Z` by a dead `nightly-reward-spike` run
          (`reward-20260914-c3e60d0`). The recipe waited its full 1800 s and gave up,
          correctly refusing to remove a lock it did not create
        - Linux evidence was obtained instead through the route the `os` skill
          documents for exactly this case — a private `--shared --no-checkout` clone
          that only READS the standing one, the missing base shipped as a
          `git bundle`, and the working tree applied as a
          `git diff --cached --binary` built under a temporary `GIT_INDEX_FILE`
                - `cd tools/test-toolkit && just archive-path-guard` on `build-linux`
                  → **2 passed**, `mode=full-tree files-checked=3475 missing=0
                  ineligible=0 violations=5` — identical to macOS, which is the
                  claim: the scan's verdict does not depend on the host
                - `just _test test-toolkit` → **282 passed, 1 failed, 2 skipped**
                - the one failure is
                  `ci_workflow_contracts::the_lint_step_measures_a_sub_second_command_instead_of_recording_zero`,
                  and it is **pre-existing and unrelated**: reproduced on a clean
                  worktree of the unpatched merge base `cd8076515` on the same host.
                  On a fast Linux host the stubbed `just` finishes inside the step's
                  rounding precision and it publishes `duration_s=0.0`, which the
                  test's `seconds > 0.0` rejects. Left alone under Rule 3 and
                  reported rather than repaired here
                - the scratch clone and its worktree were removed afterwards
        - `just cross-check test-toolkit --os windows` reported `windows pass`
          **while every upload failed**. `W:` on `build-win-native` has **4096 bytes
          free** (300 GB used), so `scp` failed for the script, the bundle, and the
          plan, and nothing ran. Freeing `W:` is the host owner's call and
          `CARGO_TARGET_DIR` must not be pointed at `C:` there, so **native Windows
          behavioral evidence for the guard remains OUTSTANDING**
                - the false green is a defect in `scripts/cross-check.sh` worth its
                  own fix: a leg whose transfer failed must not summarize as `pass`
                - the one Windows signal available was taken instead:
                  `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu`
                  → clean. That is **compile evidence only** and says nothing about
                  `canonicalize`'s `\\?\` spellings or component matching at run time
        - `$BUILD_WSL` (`build-win`) refused the connection
          (`Connection reset by 192.168.100.64 port 22`); WSL2 is not a required
          environment for this Linux-only guard and was not pursued further
- blockers / not done
        - **nothing was committed, pushed, or triggered on CI**, per the session
          rules; every result above is local or from a build host
        - native Windows behavioral coverage is OUTSTANDING (full `W:`), and the
          `ubuntu-latest` CI execution of the planned guard cell remains
          OUTSTANDING in the sense the recipe itself prints — a build-host run is
          not the hosted cell
        - pre-existing and left alone, both surfaced while running the required
          verification
                - root `just test <package>` mis-parses a single-package selector
                  that carries per-package features: `_test_local_all` word-splits
                  `dmls --features effects-instrumentation` into three ENTRIES and
                  builds `-p dmls -p --features -p effects-instrumentation`, so
                  `just test dmls` dies with "a value is required for
                  '--package <PACKAGES>'". `just/devops.just` is untouched on this
                  branch. `just _test <pkg> [flags]` was used instead
                - `the_lint_step_measures_a_sub_second_command_instead_of_recording_zero`
                  on a fast Linux host, above
- work completed for 'regression coverage and verification boundaries' at 02:28:13-0700

### Successful Completion

The implementation of review cycle 1 has completed successfully in 2 hours and 26
minutes. During this implementation all 5 review findings were evaluated to see if
they could be fixed as a part of this implementation cycle: 5 were fixed, 0 were
deferred (see reasons below):

- no finding was deferred; every finding in
  '/Volumes/coding/wt/rusty-biscuit/feat-unifi/fixes/2026-09-19-less-brittle/review-1.md'
  was implemented and verified in this cycle
        - **Finding 1** was already satisfied on this branch by commit `9b797b61a`,
          which the review's checkout predated; it was verified rather than
          re-implemented, and the relocation coverage it asked for was delivered
          under Finding 5
        - no performance measurement was required by this review, so
          `deferred_perf_measurement` remains `false`
- two items are **outstanding rather than deferred** — they are environmental, not
  findings, and neither blocks the review cycle
        - native Windows *behavioral* coverage of the scanner: `build-win-native`'s
          `W:` volume has 4096 bytes free, so every `cross-check` upload failed.
          `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` is
          clean, which is compile evidence only
        - the `ubuntu-latest` CI execution of the newly planned guard cell, which
          by design only a hosted run can produce; the local recipe prints that
          fact rather than recording a pass
- two **pre-existing defects** were found while running the required verification
  and deliberately left alone under Rule 3 (surgical changes)
        - `scripts/cross-check.sh` reports `pass` for a leg whose `scp` transfers
          all failed — this is how a full disk becomes a green summary
        - root `just test <package>` mis-parses a single-package selector carrying
          per-package features

The files changed by this implementation cycle are:

- guard implementation and fixtures
        - `tools/test-toolkit/src/archive_guard.rs` (new)
        - `tools/test-toolkit/src/archive_guard/matcher_tests.rs` (new)
        - `tools/test-toolkit/tests/archive_path_guard.rs`
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs`
        - `tools/test-toolkit/src/lib.rs`, `tools/test-toolkit/Cargo.toml`,
          `tools/test-toolkit/justfile`
- planner, schema, and plan readers
        - `scripts/ci/affected_scope.py`, `scripts/ci/schema.py`,
          `scripts/ci/plan_fixtures.py`
        - `scripts/ci-rollup.rs`, `scripts/ci-plan.rs`,
          `scripts/ci-build-archive-tests.rs`
        - `.github/ci/schemas/contract.json`
- CI execution, reporting, and local validation
        - `.github/workflows/_package-ci.yml`, `.github/workflows/_area-ci.yml`
        - `just/ci-local.just`
- tests
        - `scripts/ci/test_affected_scope.py`, `scripts/ci/test_schema.py`,
          `scripts/ci/test_resolved_plan.py`, `scripts/ci/test_ci_local.py`,
          `scripts/ci/test_evidence_reuse.py`, `scripts/ci/test_local_evidence.py`
        - `scripts/ci-rollup-tests.rs`, `scripts/ci-plan-tests.rs`
        - `messenger/lib/tests/research_relocation.rs` (new)
- documentation
        - `.github/ci/README.md`, `.github/ci/schemas/README.md`,
          `.claude/skills/rust-devops/ci-cd.md`

Final verification on the macOS host, all green:

| Suite | Result |
| --- | --- |
| `cd tools/test-toolkit && just archive-path-guard` | 2 passed; `mode=full-tree files-checked=3475 violations=5` |
| `just test test-toolkit` | 283 passed, 2 skipped |
| `just test repo-deps` | 420 passed, 1 skipped |
| `just _test messenger --features desktop,research` | 536 passed, 2 skipped |
| `python3 scripts/ci/test_affected_scope.py` | 276 OK |
| `python3 scripts/ci/test_schema.py` | 129 OK |
| `python3 scripts/ci/test_resolved_plan.py` | 79 OK |
| `python3 scripts/ci/test_ci_local.py` | 92 OK |
| `python3 scripts/ci/test_evidence_reuse.py` | 76 OK |
| `python3 scripts/ci/test_local_evidence.py` | 20 OK |
| `python3 scripts/ci/test_reuse_validation.py` | 21 OK |
| `actionlint .github/workflows/*.yml` | rc 0, no output |
| `just _lint` for `test-toolkit`, `repo-deps`, `messenger` | zero warnings |

## Implementation of Review Findings #2

> **started at:** 2026-09-20T04:56:08-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-unifi/fixes/2026-09-19-less-brittle/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review metadata
        - reviewer: `codex/gpt-5.6-sol`
        - findings: 4 — three `high`, one `low`
        - spec under review: `2026-09-19-less-brittle/spec.md`
- package areas in scope (derived from the review findings)
        - `repo-deps` (`scripts/ci/`) — planner deletion input contract and owned-input policy
        - `tools/test-toolkit` — `archive_guard` fail-closed filesystem and containment behavior
        - `.github/workflows/ci.yml`, `.githooks/pre-push`, `just/ci-local.just` — the three production selection boundaries
        - `messenger` — dependency documentation only

### Finding 1 (high) — Deleted-path identity never reaches the production planner


- starting the work on 'deleted-path identity at the production boundaries' at 04:56:56-0700
- discovered
        - `affected_scope.py` already accepts the repeatable `--deleted`, and `change_inventory` / `archive_guard_scope` already consume it; only the three callers were missing
        - `git diff --name-only` prints a rename's DESTINATION and omits its source, so the changed list needed no change in content — only the second list was missing
        - `git diff --name-status -z` frames the status and each path as SEPARATE NUL-terminated records, and `R<score>`/`C<score>` carry TWO paths (source then destination); mis-stepping that puts a rename's source in the changed list, which is precisely the "unexpectedly missing path" the deletion identity exists to rule out
        - `.githooks/pre-push` is POSIX `sh` and has no `read -d ''`, so it cannot parse NUL records at all; `just/ci-local.just` runs `#!/usr/bin/env bash`, which may be macOS's stock 3.2 (no `mapfile`); only `ci.yml`'s runner Bash is guaranteed >= 4.4
        - the hook fixture repositories already copy `scripts/ci/*.py` wholesale, so a new shared module lands there for free
- decided
        - ONE parser, three callers: new `scripts/ci/diff_scope.py` reads the `--name-status -z` stream on stdin and writes the planner's argument tail NUL-delimited — `--deleted <path>... -- <changed path>...`. Three hand-rolled shell parsers in three dialects (one of which cannot express it) would have drifted, and the review explicitly flagged the framing as fiddly
        - rename treatment, consistent with the spec's "scan the current destination of a rename; skip deleted files": the DESTINATION is the changed path, and the SOURCE is reported nowhere. It is not a diff-reported removal, and `deleted` is documented as a subset of `files`; exemption maintenance already reads the full tree in either scan mode, so a renamed-away exemption is still caught
        - the tail always ends with `--`, so an empty diff still produces a non-empty argument list — BSD `xargs` runs nothing at all on empty input, which is why the hook's `git diff --quiet` branch existed and why it could now be removed
        - a malformed/truncated stream is an error (exit 2), not an empty scan; each boundary routes the two steps so a failure is visible without relying on `pipefail` in a process substitution
        - NOT changed: `GLOBAL_PATHS_ALL_GATES` / `ORCHESTRATION_PATHS` were left alone. `scripts/ci/diff_scope.py` sits under `scripts/`, so editing it already selects `repo-deps`, whose cell runs `test_affected_scope.py` — the suite that now covers the parser. Adding a fourth global path would widen every gate for no question it answers (CI/CD Test-scope Discipline)
- completed
        - `scripts/ci/diff_scope.py` — the shared parser (`parse_name_status`, `arguments`), documented framing and rename rule
        - `.github/workflows/ci.yml` — the scope step now runs `git diff --name-status -z "$base" "$head" | python3 scripts/ci/diff_scope.py` into a file and `mapfile -d ''`s the whole argument tail into `scope_args`; the file (rather than a process substitution) is what lets the step's own `set -o pipefail` see a failing diff or parse
        - `just/ci-local.just` — same parser, with the untracked union (`git ls-files --others --exclude-standard -z`) passed as positional extra changed paths; the old unquoted `${changed}` word-split is gone, so the whole path set is now NUL-safe; the displayed file count is derived by stepping the `--deleted` pairs to the separator
        - `.githooks/pre-push` — `--name-status -z` into `$SCOPE_DIFF_FILE`, parsed into `$SCOPE_ARGS_FILE`, fed to the planner through `xargs -0`; both files registered in `TEMP_FILES`; the now-redundant `git diff --quiet` branch removed
        - `.github/ci/README.md` and `.claude/skills/rust-devops/ci-cd.md` — the `change_inventory` / version-5 sections now record how a caller obtains the deletion identity and how a rename is treated
- regression coverage added
        - `scripts/ci/test_affected_scope.py::DiffScopeParserTests` — 12 cases over the framing: deletion, plain edit, rename, copy, rename beside a deletion, a path containing a newline, empty stream, truncated status, rename without a destination, the argument tail's shape, the always-emitted separator, and one end-to-end run of the real tool over a real `git diff`
        - `scripts/ci/test_ci_local.py::WorkflowScopeStepTests` — `test_a_pull_request_declares_every_deletion_to_the_planner` and `test_a_renames_source_is_neither_changed_nor_deleted`, over a new `seed_deletion_and_rename` fixture repository; the step runs the SHIPPED `diff_scope.py` (copied, not trampolined) and the assertions read the resolved plan's `change_inventory.deleted` and `archive_guard.paths`
        - `scripts/ci/test_ci_local.py::CiLocalDiffScopeTests` — `just ci-local --dry-run` over a real repository that deletes, renames, and carries an untracked file; asserts the exact `--deleted` list and that the untracked file is changed but never deleted
        - `.githooks/tests/test-pre-push.sh` — `test_the_committed_scope_declares_deletions_and_omits_rename_sources`: a scope-only push whose committed range deletes `pkg/alpha/src/lib.rs` and renames `README.md` to `READING.md`; asserts `--deleted pkg/alpha/src/lib.rs` in the planner log, the destination present, and the source absent
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` — `every_selection_boundary_declares_its_deletions_to_the_planner` pins all three boundary sources; `no_guard_consumer_derives_its_own_source_scope` updated from `git diff --name-only` to `git diff --name-` so it keeps catching a second diff under either spelling
- non-vacuity proved (each mutation reverted immediately)
        - reverting `ci.yml` to the `--name-only` form: `test_a_pull_request_declares_every_deletion_to_the_planner` FAILS with the planner call line showing no `--deleted`
        - reverting `ci-local.just` to the `--name-only` + `sort -u` form: `CiLocalDiffScopeTests` FAILS (`['gone.rs'] != []`)
        - neutering the parser's two-path branch: the rename fixture FAILS loudly (`diff_scope: status '...' has no path; the diff stream is truncated`)
        - running the current hook suite against the PRE-CHANGE hook: the new hook test FAILS (23 failures instead of 22)
- verification results
        - `just test repo-deps` — 420 tests run, 420 passed (17 slow), 1 skipped
        - `just lint repo-deps` — exit 0 (only the pre-existing `claudine-cli` linker `__eh_frame` note)
        - `just test test-toolkit` — 284 tests run, 284 passed, 2 skipped
        - `just _lint test-toolkit` — exit 0, zero warnings
        - `python3 scripts/ci/test_affected_scope.py` — 288 tests, OK
        - `python3 scripts/ci/test_ci_local.py` — 95 tests, OK
        - `python3 scripts/ci/test_schema.py` — 129 tests, OK
        - `python3 scripts/ci/test_resolved_plan.py` — 79 tests, OK
        - `python3 scripts/ci/test_evidence_reuse.py` — 76 tests, OK
        - `python3 scripts/ci/test_local_evidence.py` — 20 tests, OK
        - `actionlint -no-color .github/workflows/ci.yml` — clean
        - `shellcheck -s sh .githooks/pre-push` — clean; `shellcheck -s bash .githooks/tests/test-pre-push.sh` — only pre-existing SC2329 info notes
        - `./.githooks/tests/test-pre-push.sh` — 45 passed, 22 failed; the new test PASSES. **All 22 failures pre-date this work** and are unrelated to it: the branch bumped the resolved-plan schema to version 5 while `.githooks/tests/fixtures/plan-*.json` are still version 4, so every fixture-consuming test reports `unknown-schema-version: resolved plan is version 4, this tool writes 5`. Proof: the same 22 fail with the pre-change hook (`PRE_PUSH_HOOK_UNDER_TEST`), where the count is 23 because the new test fails too
        - `./.githooks/tests/test-pre-push-dispatcher.sh` — PASS
- outstanding, for the author
        - the five version-4 plan fixtures under `.githooks/tests/fixtures/` need bumping to version 5 (with an `archive_guard` scope) before `just test-pre-push-hook` is green again; that is the schema work of this fix, not this finding, so it was left untouched here
- work completed for 'deleted-path identity at the production boundaries' at 06:11:44-0700
- **orchestrator note.** The subagent flagged a red that pre-dates this finding: iteration 1 bumped
  the resolved-plan schema to version 5, but `.githooks/tests/fixtures/plan-*.json` still declare
  `"schema_version": 4`, so 22 fixture-consuming cases in `.githooks/tests/test-pre-push.sh` fail
  with `unknown-schema-version`. Confirmed independently by the orchestrator. Tracked below as an
  additional work item for this iteration, because the drift was introduced by this fix's own
  schema bump and leaves `just test-pre-push-hook` red.

### Finding 2 (high) — The scanner can silently leave the repository or skip unreadable inputs


- starting the work on 'fail-closed scanner and repository containment' at 06:09:49-0700
- what the scan actually did wrong, confirmed by reading
        - `walk`'s `let Ok(canonical) = fs::canonicalize(dir) else { return Ok(()) }` and
          `let Ok(kind) = fs::metadata(&path) else { continue }` both spelled *every* failure as
          "it wasn't there", so `EACCES`, `ELOOP`, and `ENAMETOOLONG` on a path that exists were
          omissions inside a scan that then reported a pass
        - `scan`'s changed arm used `Path::is_file`, which is `fs::metadata(...).map(...).unwrap_or(false)`
          — the same collapse, plus it filed the result under `report.missing`, i.e. as a deletion
        - `collect_rust_files`'s `fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf())`
          degraded the containment boundary to the *spelling* on failure. On macOS that alone
          defeats the boundary: the canonical form of a `/tmp` tree is `/private/tmp/...`, which
          `starts_with` a `/tmp/...` boundary never matches
        - `root.join(path)` in changed mode applied no containment at all, and `Path::join` with an
          absolute argument **discards the root entirely**
- decisions
        - two new `GuardError` variants rather than overloading the existing pair: `Inspect { path, source }`
          for "exists but could not be stat'd/resolved", and `Unscannable { path, detail }` for a listed
          path that is not a scannable file inside the checkout. `Unscannable` carries the plan's own
          spelling, not a `PathBuf`, because that is the string a maintainer has to go fix
        - `NotFound` keeps the silent skip in exactly two places, both documented at the branch: a
          directory that vanished between listing and descent, and a broken symlink. Nowhere else
        - a listed path that exists but is **not a regular file** is a hard error, not `missing`. A
          changed-path list names files; a directory there means the planner emitted something the
          guard has no rule for, and silently skipping it is how a real Rust file goes unscanned
        - a root that cannot be canonicalized — including one that does not exist — is now an error.
          A boundary that fell back to the spelling is not a weaker check, it is a *wrong* one
        - path spelling is rejected **before** the filesystem is consulted, in both the plan reader
          (`GuardError::MalformedPlan`) and `scan` (`GuardError::Unscannable`). Containment alone is
          not enough: an escaping path that happens not to exist would still be filed as a deletion,
          which is precisely the defect the review named
        - `scan` keeps its own check rather than trusting the plan reader, because `ScanMode::Changed`
          is public and constructible by hand — the unit tests do exactly that
        - spelling rules are ordered containment-first, so a native Windows absolute path
          (`C:\a\b.rs`, which is both drive-prefixed and backslash-spelled) reports that it leaves
          the checkout rather than reporting a slash-direction nit
- Rust changes — `tools/test-toolkit/src/archive_guard.rs`
        - `GuardError::Inspect` and `GuardError::Unscannable`, with `Display` in the existing voice
          and `source()` wired for the former
        - `unscannable_spelling(&str) -> Option<&'static str>`: empty, whitespace-padded, absolute,
          Windows drive prefix, backslashes, `..` component, leading `./`. Phrased as a predicate so
          a caller splices it after the offending path
        - `canonical_root(&Path)`: the one place the boundary is computed, failing loudly, with the
          macOS-`/tmp` and Windows-`\\?\` reasons recorded at the function
        - `walk`: `canonicalize` and `metadata` now match on `io::ErrorKind::NotFound` and propagate
          everything else as `Inspect` naming the path
        - `scan` changed mode: explicit `fs::metadata` match (`Ok(file)` → canonicalize, assert
          `starts_with(boundary)`, scan; `Ok(_)` → `Unscannable`; `Err(NotFound)` → `missing`;
          `Err(_)` → `Inspect`), and the scanned target is the *canonical* path, so the path that
          was verified is the path that is read
        - `GuardPlan::from_plan_json` rejects an unusable path spelling before any filesystem access
- Python changes — `scripts/ci/schema.py`
        - `_is_normalized_relative_path` is the single predicate; `_unnormalized` renders the problem
        - the `change_inventory` bucket loop had an **inline copy** of the old rule, so `deleted`
          and `archive_guard.paths` would have been tightened while the buckets were not. Both now
          call the one helper
        - added to the existing backslash / `./` / whitespace rules: empty, absolute (`/…`, which
          covers UNC-ish `//host/share`), Windows drive prefix (`_DRIVE = [A-Za-z]:`, because
          `C:rel.rs` is drive-*relative* and the leading-slash test misses it), and any `..` component
        - `.github/ci/schemas/contract.json` needed **no** edit: it carries field sets and
          vocabularies, not path rules, and `schema.contract()` still round-trips byte-identical to
          the shipped file (verified)
        - `.github/ci/schemas/README.md` gained one bullet stating the rule once for all three path
          lists (`change_inventory.paths.*`, `change_inventory.deleted`, `archive_guard.paths`)
        - `scripts/ci-rollup.rs` and `scripts/ci-plan.rs` carry **no** duplicate path validation —
          `ci-plan` only renders `ArchiveGuard::summary()`. Nothing to tighten there
- tests added — 10 Rust, 3 Python
        - `an_entry_that_cannot_be_inspected_is_an_error_naming_its_path` — `#[cfg(unix)]`, a
          `0o444` parent: readable so `read_dir` yields the name, not searchable so the child's
          `metadata` is denied. That is the only directory mode that reaches the scanner's stat with
          a non-`NotFound` failure; Windows has no bit separating list from traverse, which is why
          the gate is `#[cfg(unix)]` by construction and a comment says so. Probes `fs::metadata`
          first and returns without asserting if the denial did not happen (root, or a mode-ignoring
          filesystem) — verified live on this host: `read_dir` OK, `stat` denied
        - `a_broken_symlink_is_the_one_stat_failure_the_full_tree_scan_skips` — the allowed
          `NotFound` case, still silent
        - `a_root_that_cannot_be_resolved_is_an_error_rather_than_an_unbounded_scan`
        - `a_listed_path_that_is_a_directory_is_an_error_rather_than_a_deletion`
        - `a_listed_path_climbing_out_of_the_checkout_is_an_error_even_when_it_exists`
        - `a_listed_path_that_escapes_and_does_not_exist_is_not_filed_as_a_deletion` — the exact
          shape the review called out
        - `an_absolute_listed_path_is_an_error` (literal POSIX spelling) and
          `a_listed_path_spelled_absolutely_is_rejected_rather_than_read` (the host's own spelling of
          a file that *is* inside the tree, asserted on the variant only, because which rule fires
          is platform-dependent)
        - `a_symlink_inside_the_checkout_pointing_out_of_it_is_never_read` — `#[cfg(unix)]`, matching
          the two symlink fixtures already in the file
        - `a_plan_path_leaving_the_checkout_is_malformed_before_any_filesystem_access` — five
          spellings through the plan reader
        - `test_a_path_that_leaves_the_checkout_is_rejected` (archive_guard.paths, six spellings),
          `test_a_deletion_that_leaves_the_checkout_is_rejected`, and
          `test_a_bucket_path_that_leaves_the_checkout_is_rejected`
        - a changed-mode path that does not exist landing in `report.missing` was already pinned by
          `a_deleted_listed_path_is_skipped_for_violations_and_counted_in_the_summary`; left alone
          rather than duplicated
- non-vacuity proof — neutered both guards at once (restored `metadata`'s catch-all `continue`,
  deleted the spelling check from `scan`), re-ran, and **4 of the new tests went red**:
  `a_listed_path_that_escapes_and_does_not_exist_is_not_filed_as_a_deletion`,
  `a_listed_path_climbing_out_of_the_checkout_is_an_error_even_when_it_exists`,
  `an_absolute_listed_path_is_an_error`, `an_entry_that_cannot_be_inspected_is_an_error_naming_its_path`
  (101 passed, 4 failed). Restored with a fresh mtime (`cp` then `touch`) so Cargo could not serve
  the corrupted build, and re-confirmed 105/105 on the lib target
- verification results
        - `just test test-toolkit` — 294 tests run, 294 passed, 2 skipped
        - `just _lint test-toolkit` — exit 0, zero warnings
        - `tools/test-toolkit` `just archive-path-guard` — 2 tests run, 2 passed, 0 skipped
          (`mode=full-tree files-checked=3475 missing=0 ineligible=0 violations=5`, all five
          allow-listed)
        - `just test repo-deps` — 420 tests run, 420 passed (16 slow), 1 skipped
        - `just lint repo-deps` — exit 0. Note: the **root** `lint` recipe ignores its selector and
          linted every area; the narrow `just _lint repo-deps` was run too, also exit 0
        - `python3 scripts/ci/test_schema.py` — 132 tests, OK (129 before; +3)
        - `python3 scripts/ci/test_affected_scope.py` — 288 tests, OK
        - `python3 scripts/ci/test_resolved_plan.py` — 79 tests, OK
        - `python3 scripts/ci/test_ci_local.py` — 95 tests, OK
        - `python3 scripts/ci/test_evidence_reuse.py` — 76 tests, OK
        - `python3 scripts/ci/test_local_evidence.py` — 20 tests, OK
- cross-OS
        - `just cross-check` was **not** run: `BUILD_WINDOWS` is unset in this non-interactive
          environment, so the native Windows host is unreachable from here
        - instead `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` — clean. It
          is the local proof the `rust-testing` skill prescribes, and it compiles the Windows arms
        - the containment logic is Windows-*relevant* but not Windows-*conditional*: there is no new
          `#[cfg(windows)]` code. Containment reuses the canonicalized-boundary technique the file
          already documents for `\\?\`, and the drive-prefix rule is pure string logic exercised on
          every platform by the plan-reader and schema tests. The one assertion whose *message*
          would differ on Windows was rewritten to assert the variant instead, so no test is
          green-here / red-there by construction
        - the permission fixture is Unix-only by construction, as the finding anticipated
- work completed for 'fail-closed scanner and repository containment' at 06:27:11-0700

### Finding 3 (high) — Relevant guard configuration does not select a guard scan


- starting the work on 'guard-owned configuration selection' at 06:26:44-0700
- reproduced the finding against the real planner before changing anything
        - `python3 scripts/ci/affected_scope.py --resolved-plan --event pull_request tools/test-toolkit/Cargo.toml`
          emitted `archive_guard.selected: false` while still scheduling
          `{test-toolkit, ubuntu-latest, lint}` with `archive-path-guard` attached — exactly the
          false account the finding describes: the companion reads an unselected scope and performs
          an empty changed-file scan
        - the same held for `.github/workflows/_package-ci.yml` and `.github/workflows/_area-ci.yml`
- adopted an explicit admission rule so the list stays narrow: a file is an owned input when it
  carries configuration whose **only** consumer is the guard, so a change to it can alter whether
  the scan runs or what it covers while every other check stays green
- added three configuration surfaces to `ARCHIVE_GUARD_OWN_INPUTS`
        - `tools/test-toolkit/Cargo.toml` — the registry binding. Its
          `[package.metadata.ci.tests] companion-suites` is the only declaration attaching
          `archive-path-guard` to an owner; delete the entry and every gate stays green while the
          scan never runs again
        - `.github/workflows/_package-ci.yml` — the guard's two execution controls: the
          `BISCUIT_ARCHIVE_GUARD_PLAN` export (one reader, the guard) and the companions-only status
          fold that makes a guard-only cell's verdict its companion's
        - `.github/workflows/_area-ci.yml` — the sole conduit for `lint-companions-only`
- verified the marginal cost is **zero cells**: all three already select `test-toolkit` through
  `SUITE_OWNER_PREFIXES` / `SUITE_OWNER_PATHS`, so the lint cell and its companion existed either
  way. After the change each emits `selected: true, mode: "changed", paths: []` and the cell set is
  unchanged at `{macos L1, ubuntu L1, ubuntu lint}` — no check cell, no second lint cell, no extra OS
- `mode: "changed"` with an explicitly empty path list is the spec-conformant answer for these
  inputs: the decision table gives a pull request the eligible Rust files in its inventory, and none
  of the three carries a `.rs` file. The scan such a change actually needs — is the exemption list
  still live — is the driver's full-tree half, which runs in either mode
- deliberately excluded, each with the question it fails to answer
        - `.github/workflows/ci.yml` — carries no guard-specific configuration. The resolved-plan
          artifact it uploads is read by the coverage audits, the gap publisher, and every area, so
          losing it is loud and general rather than a guard-shaped silence. Its existing negative
          test is preserved
        - `.github/ci/environments.json` — decides whether Linux is scheduled at all; when it is
          not, the guard can run nowhere, so selecting it from that change would schedule nothing
        - `scripts/ci/schema.py` and its generated `.github/ci/schemas/contract.json` — they
          *validate* the `archive_guard` block; the emitter is `archive_guard_scope` in
          `affected_scope.py`, already an owned input. A validator-only change the emitter does not
          follow fails the planner and `test_schema.py` / `test_resolved_plan.py`, which `scripts/`
          ownership already selects. The guard's Rust reader never reads `contract.json`
        - `just/ci-local.just` — the local driver. No CI guard execution runs it, so scheduling one
          answers nothing about it; `ci_workflow_contracts` and `test_ci_local.py` cover that path
- adjacent gap noticed and **not** fixed (out of scope for this finding): a change to
  `just/ci-local.just` alone selects **no package at all** today, so its guard-invocation contracts
  in `ci_workflow_contracts` and `test_ci_local.py` are scheduled by nothing. That is a
  suite-ownership question (`SUITE_OWNER_PATHS`), not a guard-selection one
- cross-language contract extended in `tools/test-toolkit/tests/ci_workflow_contracts.rs`
  (`the_guards_eligibility_policy_agrees_across_the_language_boundary`)
        - the three new entries joined the explicitly-named `extra` list
        - **new**: every entry parsed out of the Python frozenset must exist on disk, so a rename
          cannot turn the policy into a rule that matches no change and silently stops scheduling
          the guard. The entry count is asserted too (8), so a future addition cannot slip past this
          contract or `.github/ci/README.md` unexamined
- planner fixtures added in `scripts/ci/test_affected_scope.py`, new class
  `ArchiveGuardConfigurationInputTests`, run against the **shipped** tree because the claim is about
  real files
        - the registry binding's premise is checked, not assumed — the manifest really declares the
          suite under `[package.metadata.ci.tests]`
        - a real pull-request invocation naming `tools/test-toolkit/Cargo.toml` asserts
          `selected: true`, `mode: "changed"`, `paths: []`
        - one case per newly added configuration input, same assertions
        - containment: each adds no cell at all — exactly
          `{macos-latest L1, ubuntu-latest L1, ubuntu-latest lint}` for `test-toolkit`
        - the guard rides the owner's ordinary lint cell and `companions_only` stays **absent**, so
          Clippy is not stood down for a package that was genuinely selected
        - negatives against real files: `messenger/lib/Cargo.toml`, `.github/workflows/release-plz.yml`,
          and `.github/workflows/ci.yml` all stay `selected: false` with no `mode`
        - every owned input still exists on disk (the Python-side twin of the new Rust assertion)
        - guard-**only** containment (one Linux execution, no other suite, no dependent compile
          check, no extra OS) is already proved by the existing `ArchiveGuardOnlyCellTests`; it was
          reused rather than duplicated and still passes unchanged
- non-vacuity proved: with the configuration half of `ARCHIVE_GUARD_OWN_INPUTS` monkeypatched back
  to the pre-fix set, 4 of the 7 new tests fail with
  `no scanned source and no owned guard input changed`; restoring the policy makes them pass
- documentation updated
        - `.github/ci/README.md` — the trigger table gained the configuration half, plus the
          admission rule, a per-surface "what it decides" table, the zero-marginal-cost note, and
          the excluded-with-reasons paragraph. The catch-all row now reads "every **other** manifest
          or workflow file"
        - `.claude/skills/rust-devops/ci-cd.md` — five-entry list corrected to eight, with the
          admission rule and the exclusions named
- verification, all green
        - `just test repo-deps` — 420 tests run, 420 passed (16 slow), 1 skipped
        - `just _lint repo-deps` — clean
        - `just test test-toolkit` — 294 tests run, 294 passed, 2 skipped
        - `just _lint test-toolkit` — clean
        - `cd tools/test-toolkit && just archive-path-guard` — 2 tests run, 2 passed;
          `mode=full-tree files-checked=3475 missing=0 ineligible=0 violations=5` (all five allowed)
        - `python3 scripts/ci/test_affected_scope.py` — 295 tests, OK
        - `python3 scripts/ci/test_schema.py` — 132 tests, OK
        - `python3 scripts/ci/test_resolved_plan.py` — 79 tests, OK
        - `python3 scripts/ci/test_ci_local.py` — 95 tests, OK
        - `python3 scripts/ci/test_evidence_reuse.py` — 76 tests, OK
        - `python3 scripts/ci/test_local_evidence.py` — 20 tests, OK
        - `actionlint -no-color .github/workflows/*.yml` — exit 0
        - `.githooks/tests/test-pre-push.sh` left alone: its 22 failures are the tracked
          `schema_version: 4` fixture drift, not this change
- work completed for 'guard-owned configuration selection' at 06:38:04-0700

### Finding 4 (low) — The new Messenger development dependency is absent from dependency documentation


- starting the work on 'Messenger dev-dependency documentation' at 06:38:28-0700
- discovery
        - `messenger/lib/Cargo.toml` line 51 declares
          `biscuit-test-harness = { path = "../../biscuit-test-harness" }` under
          `[dev-dependencies]`; six `tests/research_*.rs` files call
          `biscuit_test_harness::manifest_dir!()`
        - no `messenger/docs/dependencies.md` exists — Messenger has no per-area
          dependency record, so the root `docs/dependencies.md` is the only place to fix
        - no test validates `docs/dependencies.md` against the manifests; the only
          reference to the file in test code is a required-phrase check
          (`"root Cargo workspace member"`) in
          `tools/test-toolkit/tests/ci_workflow_contracts.rs`, and `repo-deps` is a
          reporting binary over `cargo metadata`, not a doc checker. Nothing would have
          caught this drift automatically
- decisions
        - mirrored the adjacent `messenger/cli` harness bullet's wording and its
          "No new external crate was added." closer, placed directly after the
          `messenger/lib` research bullet
        - `git diff --stat main -- '**/Cargo.toml'` showed one other manifest change on
          this branch: `tools/test-toolkit` promoted `serde_json` from
          `[dev-dependencies]` to `[dependencies]` for `archive_guard`. Recorded it on
          the existing `tools/test-toolkit` bullet, which already explains that crate's
          unconditional-vs-gated dependency policy
- completed
        - `docs/dependencies.md` — two edits, documentation only; no Rust or manifest
          changes
- verification, all green
        - `just test repo-deps` — 420 tests run, 420 passed (17 slow), 1 skipped
        - `cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(the_ci_documentation_states_the_implemented_behavior)'`
          — 1 test run, 1 passed, 142 skipped
- work completed for 'Messenger dev-dependency documentation' at 06:40:06-0700

### Additional work item — the pre-push hook's plan fixtures are stranded on schema version 4

- starting the work on 'pre-push hook plan fixtures at schema version 5' at 06:40:23-0700
- reproduced
        - `./.githooks/tests/test-pre-push.sh` — 45 passed, 22 failed. Every failure
          was the same cause: `unknown-schema-version: resolved plan is version 4,
          this tool writes 5`. No failure had a different cause
- discovered
        - the five `.githooks/tests/fixtures/plan-*.json` documents were not the whole
          of the drift. `.githooks/tests/fixtures/affected_scope_stub.py` — the stub
          planner the hook tests run in place of `scripts/ci/affected_scope.py` — also
          emitted `"schema_version": 4`. Fixing only the JSON took the suite to 65
          passed / 2 failed, and the two survivors (`two open pull requests from one
          head are each reviewed`, `a simultaneous push with nothing prohibited passes
          in either order`) failed on `scope-schema: scope receipt carries a version 4
          plan, this tool reads 5`, which is the stub's plan reaching
          `local_evidence.record_scope`
        - version 5 requires two additions, not one: the `archive_guard` block, and
          `change_inventory.deleted`, which is required exactly when `diff_available`
          is true. All six documents carried `diff_available: true` and no deletions
        - the top-level `archive_guard` block is inert for hook behavior. `just
          ci-local` runs the guard off the *cell's* `companions` entry
          (`just/ci-local.just`, the `guard_planned` jq), not off this block; the block
          is read only by the `--plan` render. No hook test asserts on that line. So
          the block's value could not change a test outcome, which is what let me
          choose it for fidelity rather than for expedience
- decisions
        - did not hand-write the `archive_guard` blocks. Derived each one by calling
          the real `affected_scope.archive_guard_scope` with that fixture's own
          `change_inventory`, `full_scope`, and the environment set its cells name, so
          every block is one the shipped planner would actually emit for that document
          and cannot drift from the selection rule
        - read "scheduled" as the environments the fixture's own cells name, the only
          scheduling signal these documents carry. This is what makes the guard scope
          differ per fixture instead of being a constant: `plan-macos-executing.json`
          schedules macOS alone and so records `selected: false` for the environment
          reason (the guard is hosted on `ubuntu-latest` alone), while the three WSL
          fixtures and `plan-macos-two-packages.json` have `ubuntu-latest` cells and
          record a `changed` scan naming their own source paths. Cross-checked against
          each fixture's `preflight_os`, which agrees
        - the stub planner cannot import the real `affected_scope` — it *replaces* it
          inside the fixture repositories — so its two blocks are named constants
          transcribing the same two refusals the real function returns.
          `NO_GUARD_ENVIRONMENT` covers `fixed_plan` (and the `--all` branch, because
          the real function makes the environment check before any mode decision, so a
          full-scope request does not change the answer); `NO_GUARD_TRIGGER` covers
          `empty_plan`, whose diff selects nothing to scan. The stub schedules
          `macos-latest` alone, so it is never a selected guard whatever changed
        - rewrote the JSON with `json.dumps(indent=2, sort_keys=True)`, verified
          beforehand to round-trip the existing files byte-identically, so the diffs
          are the three added fields and nothing else. No cell, environment, build,
          evidence, or execution state was touched in any fixture — each scenario is
          the one its name describes
- guard against recurrence — added
        - the hook suite already had `the plan fixtures are valid resolved plans`, and
          it *did* catch this. The gap was placement, not absence: it lives in a suite
          the author of a `scripts/ci/schema.py` bump has no reason to run, and it did
          not cover the stub planner at all — the stub's staleness surfaced only
          indirectly, as two failures that named a scope receipt rather than the stub
        - added `StoredPlanDocumentTests` to `scripts/ci/test_resolved_plan.py`, which
          `repo-deps` owns and which a change under `scripts/ci/` schedules.
          `test_every_stored_hook_plan_validates` validates every
          `.githooks/tests/fixtures/plan-*.json` against
          `schema.validate_resolved_plan` and fails if the glob ever matches nothing;
          `test_the_hook_stub_planner_emits_valid_plans` loads the stub by path and
          validates `fixed_plan` and `empty_plan`. The next schema bump now fails in
          the schema's own suite instead of silently stranding the hook suite
        - proved non-vacuous: reverting `plan-wsl-absent.json` to
          `"schema_version": 4` fails the new subtest with `AssertionError: [] !=
          ['unknown-schema-version: resolved plan is version 4, this tool writes 5']`;
          restoring it passes
- completed
        - `.githooks/tests/fixtures/plan-macos-executing.json`,
          `plan-macos-two-packages.json`, `plan-wsl-absent.json`,
          `plan-wsl-executing.json`, `plan-wsl-reused.json` — version 5,
          `change_inventory.deleted`, derived `archive_guard`
        - `.githooks/tests/fixtures/affected_scope_stub.py` — version 5,
          `change_inventory.deleted` in both plan shapes, two `archive_guard` constants
        - `scripts/ci/test_resolved_plan.py` — `StoredPlanDocumentTests`, plus the
          `importlib.util` import it needs
- verification, all green
        - `./.githooks/tests/test-pre-push.sh` — 67 passed, 0 failed (was 45/22)
        - `./.githooks/tests/test-pre-push-dispatcher.sh` — 1 passed, 0 failed
        - `just test-githooks` (the recipe wrapping both suites) — dispatcher 1 passed,
          pre-push 67 passed, 0 failed
        - `just test repo-deps` — 420 tests run, 420 passed (17 slow), 1 skipped
        - `just _lint repo-deps` — clean
        - `just test test-toolkit` — 294 tests run, 294 passed, 2 skipped
        - `just _lint test-toolkit` — clean
        - `python3 scripts/ci/test_affected_scope.py` — 295 tests, OK
        - `python3 scripts/ci/test_schema.py` — 132 tests, OK
        - `python3 scripts/ci/test_resolved_plan.py` — 81 tests, OK (79 before the two
          added here)
        - `python3 scripts/ci/test_ci_local.py` — 95 tests, OK
        - `python3 scripts/ci/test_evidence_reuse.py` — 76 tests, OK
        - `python3 scripts/ci/test_local_evidence.py` — 20 tests, OK
- work completed for 'pre-push hook plan fixtures at schema version 5' at 06:53:42-0700


### Cross-OS verification — both remote legs blocked by host environment

- starting the work on 'cross-OS verification' at 06:58:00-0700
- **macOS (this host): complete and green.** Every suite in the impacted areas was
  re-run by the orchestrator after all five work items had landed, so the numbers
  below reflect the combined diff rather than any single subagent's checkpoint.
- **Linux (`build-linux`): blocked — could not produce evidence.**
        - `just cross-check test-toolkit --os linux` waited the full
          `LOCK_WAIT_SECS=1800` and exited 75 without compiling anything
        - `~/ci-verification/.cross-check.lock` on that host is **six days stale**,
          created `Sep 14 18:25` and still owned by
          `{"purpose": "nightly-reward-spike", "owner": "reward-20260914-c3e60d0",
          "branch": "feat-nightly-perf", "started": "2026-09-14T18:25:30Z"}`
        - no process on the host holds it; the owning run is long dead
        - the lock was **deliberately not removed**: `scripts/cross-check.sh` prints
          `if that run is dead its owner removes the lock by hand; never remove
          someone else's`, and clearing another run's lock is the author's call
- **Native Windows (`build-win-native`): blocked — and the tooling reported a false pass.**
        - every `scp` in the leg failed with `write remote "W:/ci-verification/…": Failure`
        - cause: the `W:` volume is **completely full** — `Get-PSDrive` reports
          `Free 0 GB / Used 280 GB` (`C:` still has 25.8 GB)
        - nothing was uploaded and nothing ran, yet the run printed
          `windows  pass` and exited 0
        - this is a **fail-open defect in `scripts/cross-check.sh`**, not in this fix:
          `run_windows` is invoked as `if run_windows; then`, which suspends `set -e`
          inside the function, so the failed `"${SCP[@]}"` does not abort; the
          subsequent `powershell -File` on a script that was never uploaded returns 0
          and is recorded as a passing tier
        - the file already carries a comment at `scripts/cross-check.sh:717` about an
          earlier instance of this same class of bug, so the guard there is incomplete
        - **left unfixed on purpose**: `cross-check.sh` is not a file this fix touches,
          and widening the diff into unrelated verification tooling during a review
          cycle is the author's call. Recommended as immediate follow-up — a
          verification harness that reports `pass` after uploading nothing is the same
          fail-open failure mode this very fix exists to remove.
- **WSL2 (`build-win`): not attempted.** It is a guest of the same Windows host whose
  disk is full, so it shares the blocker.
- **OS-risk assessment for the code that did change.** Judged low, on these grounds
  rather than on remote execution:
        - no new `#[cfg(windows)]` or otherwise platform-conditional code was added
        - containment reuses the canonicalized-boundary technique `collect_rust_files`
          already documents for macOS's symlinked `/tmp` and Windows's `\\?\` prefixes,
          and both sides of every comparison are canonicalized
        - the rejected-spelling rules (absolute, drive-prefixed, `..`, backslash) are
          pure string logic and run on every platform in the local suite
        - `cargo check -p test-toolkit --tests --target x86_64-pc-windows-gnu` is clean
        - the Unix-only permission fixtures are `#[cfg(unix)]` by construction and probe
          the denial live rather than assuming it
        - `.githooks/pre-push` stayed POSIX `sh` and is `shellcheck -s sh` clean;
          the BSD-vs-GNU `xargs -0` empty-argument difference was handled explicitly
- work completed for 'cross-OS verification' at 07:30:41-0700

### Consolidated verification — orchestrator, macOS, after every work item landed

| Gate | Result |
| --- | --- |
| `just test test-toolkit` | 294 run, **294 passed**, 2 skipped |
| `just _lint test-toolkit` | exit 0, zero warnings |
| `just test repo-deps` | 420 run, **420 passed** (17 slow), 1 skipped |
| `just _lint repo-deps` | exit 0, zero warnings |
| `cd tools/test-toolkit && just archive-path-guard` | 2 run, **2 passed** |
| `./.githooks/tests/test-pre-push.sh` | **67 passed, 0 failed** (was 45/22) |
| `scripts/ci/test_affected_scope.py` | 295 OK |
| `scripts/ci/test_schema.py` | 132 OK |
| `scripts/ci/test_resolved_plan.py` | 81 OK |
| `scripts/ci/test_ci_local.py` | 95 OK |
| `scripts/ci/test_evidence_reuse.py` | 76 OK |
| `scripts/ci/test_local_evidence.py` | 20 OK |
| `actionlint -no-color .github/workflows/*.yml` | exit 0 |
| `git diff --check` | exit 0 |
| `just cross-check test-toolkit --os linux` | **blocked** — stale lock, no evidence |
| `just cross-check test-toolkit --os windows` | **blocked** — `W:` full, no evidence |

- no Claudine, DMLS, or Messenger source was touched this iteration — `git status`
  confirms the only Messenger-side change is `docs/dependencies.md` — so those
  packages' suites were not re-run; review 2 already records them green

### Successful Completion

The implementation of review cycle 2 has completed successfully in 2h 34m. During this
implementation all 4 review findings were evaluated to see if they could be fixed as a
part of this implementation cycle: 4 were fixed, 0 were deferred.

No finding was deferred and no performance measurement was required, so
`deferred_perf_measurement` remains `false`.

One item outside the review's four findings was also fixed, because it left the
repository red and was introduced by this fix's own earlier iteration: iteration 1
bumped the resolved-plan schema to version 5 without bringing
`.githooks/tests/fixtures/plan-*.json` and `.githooks/tests/fixtures/affected_scope_stub.py`
with it, which failed 22 cases in `.githooks/tests/test-pre-push.sh`. Those documents are
now version 5, derived from the real planner rather than hand-edited, and a new
`StoredPlanDocumentTests` in `scripts/ci/test_resolved_plan.py` — owned by `repo-deps`,
so it is scheduled by any change under `scripts/ci/` — validates them against the current
schema so the next bump fails at the source instead of silently stranding the hook suite.

Two matters remain open for the author and are **not** implementation gaps in these
findings:

- **Cross-OS execution evidence could not be produced.** Linux is blocked by a six-day-old
  `.cross-check.lock` on `build-linux` owned by a dead `nightly-reward-spike` run, which
  only its owner may clear; native Windows and WSL2 are blocked by a completely full `W:`
  volume on `build-win-native`. The OS-risk assessment above explains why the changed code
  is judged low risk regardless, but that is reasoning, not execution.
- **`scripts/cross-check.sh` reports a false pass.** A leg whose uploads all failed was
  summarized `windows  pass` with exit 0. Recommended as immediate follow-up; left unfixed
  here to keep this fix's diff inside its own scope.

The files changed in this iteration:

- production selection boundaries
        - `.github/workflows/ci.yml`, `.githooks/pre-push`, `just/ci-local.just`
        - `scripts/ci/diff_scope.py` (new — the one `--name-status -z` parser)
- planner, schema, and guard policy
        - `scripts/ci/affected_scope.py`, `scripts/ci/schema.py`, `scripts/ci/plan_fixtures.py`
        - `.github/ci/schemas/contract.json`
- guard implementation
        - `tools/test-toolkit/src/archive_guard.rs`, `tools/test-toolkit/src/lib.rs`
        - `tools/test-toolkit/Cargo.toml`, `tools/test-toolkit/justfile`
- CI execution and reporting
        - `.github/workflows/_package-ci.yml`, `.github/workflows/_area-ci.yml`
        - `scripts/ci-plan.rs`, `scripts/ci-rollup.rs`, `scripts/ci-build-archive-tests.rs`
- hook fixtures
        - `.githooks/tests/fixtures/plan-*.json` (five),
          `.githooks/tests/fixtures/affected_scope_stub.py`, `.githooks/tests/test-pre-push.sh`
- tests
        - `scripts/ci/test_affected_scope.py`, `scripts/ci/test_schema.py`,
          `scripts/ci/test_resolved_plan.py`, `scripts/ci/test_ci_local.py`,
          `scripts/ci/test_evidence_reuse.py`, `scripts/ci/test_local_evidence.py`
        - `scripts/ci-plan-tests.rs`, `scripts/ci-rollup-tests.rs`
        - `tools/test-toolkit/tests/archive_path_guard.rs`,
          `tools/test-toolkit/tests/ci_workflow_contracts.rs`
- documentation
        - `.github/ci/README.md`, `.github/ci/schemas/README.md`, `docs/dependencies.md`,
          `.claude/skills/rust-devops/ci-cd.md`

## Implementation of Review Findings #3

> **started at:** 2026-09-20T09:32:13-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-unifi/fixes/2026-09-19-less-brittle/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- review metadata
        - reviewer: `codex/gpt-5.6-sol`
        - findings: 2 — one `high`, one `medium`
        - spec under review: `2026-09-19-less-brittle/spec.md`
- package areas in scope (both findings land on the same two)
        - `tools/test-toolkit` — the guard's scanner and its resolved-plan reader
        - `repo-deps` (`scripts/ci/`) — the canonical planner and shared plan contract the reader must agree with
- orchestration plan
        - the two findings both edit `tools/test-toolkit/src/archive_guard.rs`, so they run
          serially rather than in parallel

### Finding 1 (high) — Changed-file scans still accept unexpectedly missing inputs

- starting the work on 'fail-closed listed-path scan' at 09:33:02-0700
- what the fail-open hole actually was, confirmed by reading both sides
        - `archive_guard_scope` in `scripts/ci/affected_scope.py` builds `paths` from the changed
          list with `removed` subtracted, so by construction no path the diff called a deletion
          ever reaches the scanner. The planner half is correct and was left untouched
        - `scan`'s changed arm nevertheless still answered `ErrorKind::NotFound` with
          `report.missing.push(...)` and a success, and the driver printed "listed path no longer
          exists, skipped". With deletions already filtered out upstream, that branch could only
          ever fire for a file that vanished *after* the scope was resolved — a rename destination
          or an ordinary modified `.rs` — which is exactly the false pass the deletion identity was
          introduced to remove
- decisions
        - `NotFound` on a listed path is now a hard error, `GuardError::MissingListedPath { path }`.
          It carries the plan's own spelling rather than a `PathBuf`, matching `Unscannable`: that
          string is what a maintainer has to go look up in the plan
        - the `missing` concept is deleted rather than kept and promoted, because it is now
          unreachable: `ScanReport::missing` and the `missing=` field of `ScanReport::summary()` are
          gone, and a summary that still counted a number that can only be zero would be noise
        - ordering is unchanged and still load-bearing. `unscannable_spelling` and canonical
          containment are judged before `fs::metadata`, so an escaping path that happens not to
          exist is still reported for leaving the checkout rather than for disappearing
        - `NotFound` keeps its silent skip in the two full-tree places it was already documented at
          (a directory that vanished between listing and descent, a broken symlink). Full-tree
          discovery reads what it found; it is given no list to be accountable to
- Rust changes — `tools/test-toolkit/src/archive_guard.rs`
        - `GuardError::MissingListedPath` at `:234`, `Display` at `:285`, `source()` arm at `:321`.
          The message states the contract — the planner omits known deletions, so an absent listed
          path disappeared after planning — rather than narrating the branch
        - `ScanReport::missing` removed (`:570` is now `ineligible`); `summary()` at `:583` prints
          `mode=… files-checked=… ineligible=… violations=…`
        - the changed-arm `NotFound` branch returns the new error at `:1395`
        - documentation that encoded the stale contract: the `scan()` `## Errors` block at `:1324`,
          `ScanMode::Changed`'s doc, and `GuardError::Inspect`'s "distinct from absent" note, which
          now names which absence goes where. The module `//!` header described neither deletions
          nor `missing`, so it needed no edit
        - two message strings that still said "rather than a deletion" — `Unscannable`'s `Display`
          tail and the not-a-regular-file `detail` — were trimmed: with deletions unlistable, the
          contrast they drew no longer exists. Comment-quality drift, resolved in favor of the code
- Rust changes — `tools/test-toolkit/tests/archive_path_guard.rs`
        - the driver's `for missing in &report.missing` print loop is gone; the summary line and the
          scope reason remain the whole announcement
- fixtures replaced, per the review
        - `a_deleted_listed_path_is_skipped_for_violations_and_counted_in_the_summary` →
          `an_absent_listed_path_fails_rather_than_being_skipped` (`:1565`). It now proves the
          absent listed path produces `MissingListedPath` naming that path
        - `a_rename_destination_is_scanned_like_any_other_listed_path` (`:1580`) keeps the
          destination-is-scanned assertion and drops the missing-source acceptance, which is a shape
          the corrected diff parser deliberately no longer emits
- new L1 planner-to-reader regression — `tools/test-toolkit/tests/ci_workflow_contracts.rs`
        - `the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`
          (`:3777`) runs the shipped `scripts/ci/affected_scope.py --resolved-plan --deleted <gone>
          -- <modified> <gone>`, feeds the planner's own document to `GuardPlan::from_plan_json`,
          asserts the deletion is absent from `archive_guard.paths` and the modified file present,
          then scans an empty temp tree through that same `ScanMode` and requires
          `MissingListedPath`. This is the one test that can fail when either half regresses
        - placed in `ci_workflow_contracts` because `test-toolkit` owns that suite and the guard's
          other cross-language contracts already live there; it follows the existing shipped-planner
          pattern (`scripts/ci-rollup-tests.rs::the_real_planners_plan_rolls_up`), including a
          `python_interpreter()` probe at `:3757` that skips where no interpreter exists rather than
          failing a host without one
        - cross-OS: the scope paths are repository-relative with `/` separators, and the "missing"
          path is produced by scanning an empty `TempDir` rather than by spelling a POSIX-only
          absent path, so the fixture is identical on macOS, Linux, native Windows, and WSL2
- non-vacuity proof
        - with the new branch neutered back to a `continue`, both
          `an_absent_listed_path_fails_rather_than_being_skipped` and the planner-to-reader
          regression go red (`2 tests run: 0 passed, 2 failed`); the source was restored with a
          plain write plus `touch`, so Cargo could not hand back the neutered build
- verification
        - `just test test-toolkit` — **295 tests run, 295 passed, 2 skipped** (294 before; one net
          test added, two rewritten)
        - `just _lint test-toolkit` — clean. `cd tools/test-toolkit && just lint` does not exist:
          this area's justfile carries only `archive-path-guard` and `verify-nextest-config`, and
          the shared `_lint` recipe (`cargo clippy -p <pkg> --all-targets -- -D warnings`) is the
          canonical lint for it
        - `cd tools/test-toolkit && just archive-path-guard` — **2 passed**, printing
          `mode=full-tree files-checked=3475 ineligible=0 violations=5` (the five existing
          allowlisted forms)
        - `just test repo-deps` — **420 tests run, 420 passed, 1 skipped**. Nothing under
          `scripts/ci/` was changed; run as confirmation that the planner's output still satisfies
          its own suite while a Rust consumer reads it
- deliberately left out
        - the planner's behavior. `archive_guard_scope` already omits known deletions, and the
          review says so
        - review 3's medium finding (the plan reader's contract validation), which is a separate
          work item
- work completed for 'fail-closed listed-path scan' at 09:39:29-0700

### Finding 2 (medium) — The Rust plan reader accepts contract-invalid plans

- starting the work on 'rust plan-contract validation' at 09:45:43-0700
- the gap, read against the Python authority
        - `scripts/ci/schema.py::_archive_guard` (`:737`) and the document-level version check at
          `:889` are the contract. `GuardPlan::from_plan_json` enforced only a subset of the first
          and none of the second, so a document supplied through `BISCUIT_ARCHIVE_GUARD_PLAN` — which
          never passes the Python validator — could be an unsupported generation, or
          `selected: false` carrying stale `mode`/`paths`, and still run as an empty changed scan
          reporting success
- schema-version constant — `tools/test-toolkit/src/archive_guard.rs:206`
        - `pub const PLAN_SCHEMA_VERSION: u64 = 5`, new. No reuse was available: the only other
          Rust resolved-plan constant is `scripts/ci-rollup.rs:78`, a private `const` inside the
          `repo-deps` package, and `test-toolkit` neither depends on `repo-deps` nor should (that
          crate's graph is gix/duckdb/sniff-shaped, and `test-toolkit` is deliberately a three-crate
          leaf). The frozen contract document is what keeps the copies honest, so the constant is
          paired with a contract test rather than shared through code
        - `from_plan_json` checks the version **before** the shape, mirroring
          `validate_resolved_plan`'s recorded reasoning: a document from another generation usually
          differs in both, and reporting a missing field sends the reader after a corrupt plan when
          the answer is that the two sides moved apart. Missing, non-numeric, and wrong-version are
          three distinct diagnostics
- `reason` type decision — `GuardPlan::reason` is now `String`, not `Option<String>`
        - the contract requires a non-blank `reason` in all three `archive_guard` shapes, so the
          `None` case modelled a state the planner cannot emit. Removing it makes the invariant
          unrepresentable rather than merely checked
        - `GuardPlan::full_tree()` — the standalone default that never came from a plan — keeps
          coherence by stating its own reason,
          `"BISCUIT_ARCHIVE_GUARD_PLAN is unset; the standalone default checks the full tree"`. A
          distinct representation was the alternative, but it buys nothing: the field's only
          consumer is the driver's printed summary, and that sentence is *more* informative than
          the `Option` branch it replaces, which printed no reason line at all for a standalone run
        - the driver (`tools/test-toolkit/tests/archive_path_guard.rs:22`) drops both
          `as_deref()` branches and prints the reason unconditionally; the guard recipe's output now
          carries `archive-path guard: scope reason — …` on every invocation
- the rest of the parity work — all in `from_plan_json`, all `GuardError::MalformedPlan`
        - `reason` (`:518`): required, must be a string, must not be blank or whitespace-only
        - `selected: false` (`:541`): rejects a companion `mode` or `paths` instead of returning
          early and ignoring them, naming which of the two is present
        - `paths` (`:588`): must be sorted and duplicate-free, matching Python's
          `_normalized_path_list`. Sortedness is checked first because it makes any repeat adjacent;
          both diagnostics name the offending path. This rule lives at the plan-reading boundary
          only — `scan()`'s defensive de-duplication and the hand-built scan fixtures that exercise
          duplicate listed paths are untouched and still pass
        - every pre-existing rejection (unknown `mode`, `changed` without `paths`, `full` with
          `paths`, non-array/non-string entries, escaping spellings) is preserved, and the
          per-entry spelling checks still run before any filesystem access
- fixtures — `tools/test-toolkit/src/archive_guard.rs` unit tests
        - two new helpers, `plan_document` (`:1628`) and `read_scope` (`:1633`), wrap a scope object
          in a versioned document so a fixture testing something else cannot spell the version by
          hand — the drift this finding is about
        - existing reader fixtures were minimal documents without `schema_version` or `reason`;
          each was made valid under the new contract while keeping the assertion it was written for
        - ten new rejection fixtures, one per Python schema case: unsupported version, missing
          version, non-numeric version, missing/blank/non-string `reason`, `selected: false` with
          `mode` and with `paths` (one table-driven test), unsorted `paths`, duplicate `paths`. Each
          asserts the diagnostic text, not just the variant
        - `the_standalone_default_states_its_own_reason` pins the `full_tree()` half of the type
          decision
- cross-language fixtures — `tools/test-toolkit/tests/ci_workflow_contracts.rs`
        - `the_plan_schema_version_matches_the_frozen_contract` (`:3847`) asserts the Rust constant
          equals `resolved_plan.schema_version` in `.github/ci/schemas/contract.json`. This is the
          mechanism iteration 1 lacked when it bumped the version in one place and stranded the
          pre-push fixtures; it follows the existing
          `scripts/ci-rollup-tests.rs::plan_fields_match_the_frozen_contract` pattern
        - `the_shipped_planner_emits_plans_the_guards_reader_accepts` (`:3868`) runs the real
          `scripts/ci/affected_scope.py --resolved-plan` three times — a documentation-only change,
          `--all`, and a source change — and requires `from_plan_json` to accept each, covering all
          three `archive_guard` shapes against the real emitter. It reuses the sibling agent's
          `python_interpreter()` probe and its skip-when-absent behavior rather than inventing a
          second one. Hand-written fixtures agree with whichever side wrote them; only this one
          fails when the planner and the reader drift apart
- documentation
        - `from_plan_json`'s `## Errors` now enumerates every rejection and says why the rules are
          enforced twice; `from_env` defers to it instead of restating a stale subset; the
          `GuardPlan::reason` field doc records the non-empty invariant
        - `.github/ci/schemas/README.md` — the `archive_guard` bullet now states that
          `BISCUIT_ARCHIVE_GUARD_PLAN` bypasses the validator and that the Rust reader re-enforces
          the whole shape including `schema_version`. `.github/ci/README.md` describes scheduling
          and transport, not what the reader accepts, and needed no change
- cross-OS: pure JSON and string validation. The path-spelling checks are untouched and still
  reject native-Windows spellings — drive prefixes and backslashes — in a supplied plan regardless
  of host, proven by the existing
  `a_plan_path_leaving_the_checkout_is_malformed_before_any_filesystem_access` table
- verification
        - `just test test-toolkit` — **307 tests run, 307 passed, 2 skipped** (295 after finding 1;
          twelve fixtures added)
        - `just _lint test-toolkit` — clean
        - `cd tools/test-toolkit && just archive-path-guard` — **2 passed**, printing
          `mode=full-tree files-checked=3475 ineligible=0 violations=5` and the new standalone
          scope-reason line
        - `just test repo-deps` — **420 tests run, 420 passed, 1 skipped**. Nothing under
          `scripts/ci/` was changed; run as confirmation that the planner still satisfies its own
          suite while the stricter Rust consumer reads its output
- deliberately left out
        - `scripts/ci/schema.py` and the planner. The Python side is the authority this change was
          brought to parity with, not a thing to change
        - the other resolved-plan fields (`base`, `head`, `areas`, `cells`, …). The guard reads
          `schema_version` and `archive_guard` and nothing else; validating fields it never consumes
          would duplicate `ci-rollup`'s job and make the guard fail on plans that are fine for it
- work completed for 'rust plan-contract validation' at 09:47:44-0700

### Consolidated verification — orchestrator, macOS, after both findings landed

- re-ran every suite the two findings touch, from the repo root on the macOS host, with both
  changes in the tree together
        - `just test test-toolkit` — **307 tests run, 307 passed, 2 skipped**
        - `just test repo-deps` — **420 tests run, 420 passed, 1 skipped** (14 slow)
        - `just test-githooks` — **67 passed, 0 failed**; the pre-push plan fixtures still satisfy
          the stricter reader, which matters because they are stored resolved-plan documents
        - `cd tools/test-toolkit && just archive-path-guard` — **2 passed**, full-tree mode checked
          3,475 files and reported only the five existing allowlisted forms
        - `just _lint test-toolkit` — clean (`cargo clippy -p test-toolkit --all-targets -D warnings`)
        - `git diff --check` — clean
- `actionlint` was not run: no workflow file changed in this iteration. The only non-Rust file
  touched is `.github/ci/schemas/README.md`

### Cross-OS verification — Linux leg still blocked by the same stale remote lock

- `just cross-check test-toolkit --os linux` planned correctly (archive key `9ac02fa38bbaef9e`,
  produced on and consumed as `ubuntu-latest`) but could not execute
        - `build-linux` is still held by the six-day-old `nightly-reward-spike` lock recorded in
          iteration 2 — owner `reward-20260914-c3e60d0`, started `2026-09-14T18:25:30Z`. Only its
          owner may clear it, so this is unchanged host state rather than a new obstacle
        - native Windows and WSL2 were blocked in iteration 2 by a full `W:` volume on
          `build-win-native`; neither was retried here because the guard is Linux-hosted and no
          Windows-specific code path changed
- OS-risk assessment for what actually changed in this iteration
        - finding 1 replaces an `ErrorKind::NotFound` arm with an error return. `NotFound` is the
          mapping every supported platform produces for an absent path, and the new test reaches it
          through an empty `TempDir` rather than a POSIX-only spelling
        - finding 2 is JSON and string validation. The one OS-sensitive rule it sits beside —
          rejecting drive prefixes and backslashes in a supplied plan — is unchanged and is judged
          from the plan text, not from the host's path semantics, so it behaves identically on all
          four environments
        - both new planner-invoking tests skip when no `python3` interpreter is on `PATH`, which is
          the existing repo pattern and is what native Windows hosts hit

### Successful Completion

The implementation of review cycle 3 has completed successfully in 19m. During this implementation
all 2 review findings were evaluated to see if they could be fixed as a part of this implementation
cycle: 2 were fixed, 0 were deferred.

No finding was deferred and no performance measurement was required, so
`deferred_perf_measurement` remains `false`.

The fail-open branch the review named is gone: a listed changed path that does not exist is now
`GuardError::MissingListedPath` rather than a skipped entry in a successful report, and
`ScanReport::missing` — the field that made the false pass expressible — no longer exists. The
planner and the reader are now held together by tests rather than by convention: one L1 regression
resolves a real plan through the shipped `affected_scope.py` and proves a known deletion is omitted
while an unexpectedly absent listed path fails the guard, another proves the reader accepts all
three shapes the shipped planner emits, and a third pins the Rust `PLAN_SCHEMA_VERSION` constant to
`resolved_plan.schema_version` in the frozen `.github/ci/schemas/contract.json` — the mechanism
whose absence stranded the hook fixtures in iteration 1.

One matter remains open for the author and is **not** an implementation gap in these findings:

- **Linux execution evidence still cannot be produced locally.** The `build-linux` rig has been
  held since `2026-09-14T18:25:30Z` by another run's lock, which only that run's owner may clear.
  The OS-risk reasoning above explains why the changed code is judged low risk on every supported
  environment, but that is reasoning, not execution; the Linux CI leg remains the proof.

The files changed in this iteration:

- guard implementation
        - `tools/test-toolkit/src/archive_guard.rs` — `GuardError::MissingListedPath`, the removal
          of `ScanReport::missing`, `PLAN_SCHEMA_VERSION`, and the full `archive_guard` shape check
          in `GuardPlan::from_plan_json`
- tests
        - `tools/test-toolkit/tests/archive_path_guard.rs` — the driver, which no longer prints a
          skipped-path list and no longer branches on an optional reason
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` — the `python_interpreter()` probe and
          three cross-language regressions
- documentation
        - `.github/ci/schemas/README.md`

## Implementation of Review Findings #4

> **started at:** 2026-09-20T11:06:02-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-unifi/fixes/2026-09-19-less-brittle/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- review metadata
        - reviewer: `codex/gpt-5.6-sol`
        - findings: 1, at `medium` priority
        - spec under review: `2026-09-19-less-brittle/spec.md`
        - review 4 records no blocked-findings section, so nothing from an earlier
          iteration became newly actionable here
- package areas in scope (the single finding spans the plan contract's two consumers)
        - `tools/test-toolkit` — `GuardPlan::from_plan_json`, the Rust half of the
          resolved-plan `archive_guard` contract
        - `scripts/ci` (`repo-deps`) — `schema.py`'s canonical validator and its
          `test_schema.py` fixtures, the Python half

### Finding 1 — The Rust plan reader still accepts unknown guard fields

- starting the work on 'unknown guard fields' at 11:07:34-0700
- what the two sides do today
        - `scripts/ci/schema.py::_archive_guard` calls `_keys` first, so a non-object
          `archive_guard` is `must be an object` and any field outside
          `ARCHIVE_GUARD_FIELDS` is `has unknown field '<name>'`, both before any scope
          is interpreted
        - `GuardPlan::from_plan_json` reads `reason`, `selected`, `mode`, and `paths`
          individually and never looks at the object's key set; a non-object
          `archive_guard` falls through `Value::get` and is reported as a missing
          `reason`, which names the wrong defect
- the mechanism chosen for the cross-language table, and why
        - a new hand-authored fixture, `.github/ci/schemas/archive_guard_cases.json`,
          read by `scripts/ci/test_schema.py::ArchiveGuardSharedCorpusTests` through
          `validate_resolved_plan` and by
          `ci_workflow_contracts.rs::the_guards_reader_agrees_with_the_shared_scope_corpus`
          through `GuardPlan::from_plan_json`
        - `contract.json` was considered first and rejected for the corpus: it is
          *generated* by `python3 scripts/ci/schema.py` and asserted byte-stable, so a
          hand-written accept/reject table there would have to be authored inside the
          validator module it is meant to test
        - `contract.json` **is** used for the other half: `PLAN_SCOPE_FIELDS` is asserted
          against `resolved_plan.archive_guard`'s key set by
          `the_guard_scope_field_set_matches_the_frozen_contract`, beside the existing
          `the_plan_schema_version_matches_the_frozen_contract`. So the field *names* ride
          the generated contract and the *behavior* rides the written corpus
        - the corpus needs no Python subprocess on the Rust side — it is a JSON document,
          not an invocation — so it adds no new `python_interpreter()` skip and no new
          Windows App-Execution-Alias exposure
        - the table was proved to be a real synchronization point rather than a passive
          fixture: with the new key check disabled, the Rust half fails on the
          `unknown-field` case with `the reader accepted GuardPlan { ... }`
- what changed in the reader
        - the key-set check runs after the schema version and before any scope is
          interpreted, matching `_archive_guard`'s `_keys`-then-interpret order
        - a non-object `archive_guard` is now its own diagnostic; previously it fell
          through `Value::get` and was reported as a missing `reason`, which named the
          wrong defect. Python already answered `must be an object` here
        - unknown keys are sorted before they are named, so the message is deterministic
          regardless of any serde_json map ordering
- `PLAN_SCHEMA_VERSION` was **not** bumped, and should not be: the closed key set is
  unchanged, no plan field was added or removed, and every shipped plan fixture still
  validates on both sides. The Rust reader is only being brought up to a contract the
  Python validator already enforced
- one CI-scope decision was needed for the table to be genuinely two-sided
        - `.github/ci/schemas/**` previously selected `repo-deps` alone, so a
          corpus-only edit would have run the Python half in CI and not the Rust half —
          exactly the silent divergence this finding is about
        - added `(".github/ci/schemas/", ("repo-deps", "test-toolkit"))` ahead of the
          broader `.github/ci/` entry in `SUITE_OWNER_PREFIXES`, which is the mechanism
          already in place for `.github/workflows/**` selecting `test-toolkit`, and whose
          comment already contemplates a path naming two owners
        - the gap was pre-existing for `contract.json` too; the new entry closes both
- work completed for 'unknown guard fields' at 11:25:02-0700
        - files changed
                - `tools/test-toolkit/src/archive_guard.rs` — the public
                  `PLAN_SCOPE_FIELDS` constant, the `backticked` diagnostic helper, the
                  non-object and unknown-key checks in `from_plan_json` ahead of any
                  scope interpretation, the updated `## Errors` doc enumerating both new
                  rules, and three wording fixtures
                  (`a_plan_carrying_an_unknown_guard_field_is_an_error`,
                  `unknown_guard_fields_are_named_in_a_stable_order`,
                  `a_guard_scope_that_is_not_an_object_is_an_error`)
                - `tools/test-toolkit/tests/ci_workflow_contracts.rs` — the two new
                  cross-language tests, `the_guard_scope_field_set_matches_the_frozen_contract`
                  and `the_guards_reader_agrees_with_the_shared_scope_corpus`
                - `.github/ci/schemas/archive_guard_cases.json` — new; the shared
                  accept/reject corpus, 26 cases (4 valid, 22 invalid)
                - `scripts/ci/test_schema.py` — `ArchiveGuardSharedCorpusTests`, the
                  Python half of the corpus; no existing guard test was removed
                - `scripts/ci/affected_scope.py` — the narrower
                  `.github/ci/schemas/` entry in `SUITE_OWNER_PREFIXES`
                - `scripts/ci/test_affected_scope.py` —
                  `test_a_cross_language_schema_document_selects_both_readers`
                - `.github/ci/schemas/README.md` — the corpus documented beside
                  `contract.json`, and the guard bullet now names the closed field set
                  among what the Rust reader re-enforces
                - `.github/ci/README.md` — the suite-owner table's new `schemas/` row
        - verification, all green on macOS 27.0.0
                - `just test test-toolkit` — 312 tests run, 312 passed, 2 skipped
                - `just test repo-deps` — 420 tests run, 420 passed (19 slow), 1 skipped
                - the CI Python suites `just ci-local` runs, plus the two evidence
                  suites: `test_schema.py` 134 OK, `test_affected_scope.py` 296 OK,
                  `test_resolved_plan.py` 81 OK, `test_ci_local.py` 95 OK,
                  `test_constraints.py` 39 OK, `test_publish_gaps.py` 19 OK,
                  `test_runner_loss.py` 44 OK, `test_build_key.py` 12 OK,
                  `test_evidence_reuse.py` 76 OK, `test_local_evidence.py` 20 OK
                - `cd tools/test-toolkit && just archive-path-guard` — 2 tests run,
                  2 passed, 0 skipped; full-tree scan of 3475 files
                - `just _lint test-toolkit` — clean, zero warnings
                - `just test-githooks` — 67 passed, 0 failed; all five
                  `.githooks/tests/fixtures/plan-*.json` re-validated against
                  `validate_resolved_plan` and carry only known guard fields
                - `actionlint` not run: no `.github/workflows/*.yml` file was touched
        - OS risk judged low. The change is JSON key-set validation over an in-memory
          `serde_json::Value`; it spawns no process, touches no path spelling, and the
          new Rust test reads a repository file through the existing `read()` helper,
          which already normalizes CRLF for Windows checkouts. The corpus's
          `windows-spelled-path`, `absolute-path`, and `drive-prefixed-path` cases are
          data, not filesystem access, and assert identically on every host

### Orchestrator verification

- the orchestrator re-ran the finding's verification independently rather than
  accepting the subagent's report at face value
        - `just test test-toolkit` — **312 tests run, 312 passed, 2 skipped**
          (review 4 measured 307; the five new tests are the shared-corpus reader,
          the frozen-contract field-set check, and three diagnostic-wording fixtures)
        - `just test repo-deps` — **420 tests run, 420 passed (14 slow), 1 skipped**
        - `cd tools/test-toolkit && just archive-path-guard` — **2 passed, 0 skipped**
        - `just _lint test-toolkit` — zero warnings
        - `just test-githooks` — **67 passed, 0 failed**
- the shared corpus was **mutation-tested** to prove it is a synchronization point
  rather than a passive fixture that would pass whatever either side did
        - the reader's closed-set filter was temporarily neutered to accept every key
        - `the_guards_reader_agrees_with_the_shared_scope_corpus` then failed on the
          `unknown-field` case with `the reader accepted GuardPlan { ... }`, naming the
          rule the case exists to hold
        - the filter was restored and the three contract tests re-confirmed green
        - this is the property review 4 asked for: a future shape addition made in one
          language fails the other language's suite instead of drifting silently
- six commits (`306cd71aa`..`5efb9a417`, authored 11:09 by Ken Snyder) landed on the
  branch while this iteration ran; they carry the iteration 1–3 work. This iteration's
  changes sit uncommitted on top of them and were verified against that tree

### Successful Completion

The implementation of review cycle 4 has completed successfully in 22 minutes. During
this implementation all 1 review findings were evaluated to see if they could be fixed
as a part of this implementation cycle: 1 were fixed, 0 were deferred.

No finding was deferred. The single medium-priority finding was fully implemented,
including the cross-language table-driven fixture the review recommended rather than
the two manually synchronized test lists it warned against.

The files changed in this iteration:

- guard implementation
        - `tools/test-toolkit/src/archive_guard.rs` — `PLAN_SCOPE_FIELDS`, the closed
          key-set check in `GuardPlan::from_plan_json` placed after the schema-version
          check and before scope interpretation (Python's `_keys`-then-interpret
          order), a distinct diagnostic for a non-object `archive_guard`, and the
          `## Errors` doc pass both rules required
- cross-language contract
        - `.github/ci/schemas/archive_guard_cases.json` *(new)* — 26 `archive_guard`
          cases (4 valid, 22 invalid), each carrying the scope fragment, the verdict,
          and the rule it exercises; read by both languages
- tests
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` — the shared-corpus
          reader, `the_guard_scope_field_set_matches_the_frozen_contract`, and the
          Rust half of the agreement assertion
        - `scripts/ci/test_schema.py` — `ArchiveGuardSharedCorpusTests`, the Python
          half, reading the same corpus file
        - `scripts/ci/test_affected_scope.py` — coverage for the new schema-directory
          selection rule
- selection
        - `scripts/ci/affected_scope.py` — `.github/ci/schemas/` now selects both
          `repo-deps` and `test-toolkit`, so a corpus-only edit cannot run one half of
          the contract in CI and not the other; this is the same two-owner mechanism
          `.github/workflows/**` already uses, and it closed a pre-existing gap for
          `contract.json` as well
- documentation
        - `.github/ci/schemas/README.md` — the corpus document and its role
        - `.github/ci/README.md` — the schema-directory selection rule

`PLAN_SCHEMA_VERSION` was deliberately **not** bumped: the closed key set is unchanged
and no plan field moved. The Rust reader is catching up to a contract the Python
validator already enforced, which is a consumer fix, not a schema generation.

## Implementation of Review Findings #5

> **started at:** 2026-09-20T11:53:00-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-unifi/fixes/2026-09-19-less-brittle/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- review metadata
        - reviewer: `codex/gpt-5.6-sol`
        - findings: 1, at `medium` priority
        - spec under review: `2026-09-19-less-brittle/spec.md`
        - review 4's single finding is recorded as implemented; no blocked finding
          needed reevaluation in this iteration
- package areas in scope
        - `scripts/ci` (`repo-deps`) — the canonical planner `affected_scope.py` and
          its selection tests
        - `.github/ci` — the selection-rule documentation that must match the planner

### Finding 1 — The schema-directory ownership rule schedules unrelated files

- starting the work on 'schema-directory ownership' at 11:54:00-0700
        - confirmed the report: `SUITE_OWNER_PREFIXES` carried
          `(".github/ci/schemas/", ("repo-deps", "test-toolkit"))`, so the directory's
          own `README.md` and any schema added later selected `test-toolkit`, whose
          Rust suite reads neither
        - moved the two genuine cross-language documents —
          `.github/ci/schemas/contract.json` and
          `.github/ci/schemas/archive_guard_cases.json` — into `SUITE_OWNER_PATHS`
          with owners `("repo-deps", "test-toolkit")`, and removed the prefix entry.
          `suite_owner_paths()` consults the exact-path table first, so the two
          documents keep both owners while everything else under `.github/ci/schemas/`
          falls through to the surviving `.github/ci/` prefix and selects `repo-deps`
          alone
        - rewrote the comment that justified the prefix: the WHY (both readers consume
          the same bytes) now sits on the path entries, together with the reason the
          set is named one by one rather than taken as a directory
        - verified rather than assumed the claim at `affected_scope.py:382` that
          `_package-ci.yml` and `_area-ci.yml` "already select `test-toolkit` through
          `SUITE_OWNER_PREFIXES`": the `.github/workflows/` prefix is untouched, so the
          statement still holds and needed no edit
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs::tooling_inputs_select_their_registered_suite_owner`
          parses the `SUITE_OWNER_PREFIXES` literal for the three remaining prefixes;
          the block's delimiters are unchanged, so it still reads and passes
        - tests: kept `test_a_cross_language_schema_document_selects_both_readers` and
          added two negative fixtures pinning the boundary —
          `test_schemas_documentation_selects_repo_deps_alone` for
          `.github/ci/schemas/README.md`, and
          `test_an_unrelated_future_schema_selects_repo_deps_alone` for
          `.github/ci/schemas/unrelated-future-schema.json`; both assert `["repo-deps"]`
        - documentation drift fixed in the same change: the selection table in
          `.github/ci/README.md` now names the two documents instead of
          `.github/ci/schemas/**`, and the paragraph beneath it states that the rest of
          the directory keeps `repo-deps` only; `.claude/skills/rust-devops/ci-cd.md`
          gained the same two-path exception, which its owner list had not recorded at
          all. `.github/ci/schemas/README.md` makes no directory-wide selection claim,
          so it needed no edit
        - verification: `python3 scripts/ci/test_affected_scope.py` **298 passed**
          (296 before the two new fixtures); `python3 scripts/ci/test_schema.py`
          **134 passed**; `just test test-toolkit` **312 run, 312 passed, 2 skipped**;
          `just test repo-deps` **420 run, 420 passed (25 slow), 1 skipped** on a warm
          tree in a single run — no archive-fixture timeout occurred, so no rerun was
          needed; `just _lint repo-deps` and `just _lint test-toolkit` both clean;
          `git diff --check` clean
- work completed for 'schema-directory ownership' at 11:56:08-0700

### Successful Completion

The implementation of review cycle 5 has completed successfully in 5 minutes
(2026-09-20T11:53:00-07:00 to 2026-09-20T11:58:00-07:00). During this
implementation all 1 review findings were evaluated to see if they could be
fixed as a part of this implementation cycle: 1 was fixed, 0 were deferred.

The files changed in this cycle were:

- `scripts/ci/affected_scope.py` — the `.github/ci/schemas/` entry removed from
  `SUITE_OWNER_PREFIXES`; `contract.json` and `archive_guard_cases.json` added
  to `SUITE_OWNER_PATHS` with owners `("repo-deps", "test-toolkit")`
- `scripts/ci/test_affected_scope.py` — two negative fixtures added pinning the
  boundary, with the existing positive case retained
- `.github/ci/README.md` — the selection table and its explanatory paragraph
- `.claude/skills/rust-devops/ci-cd.md` — the owner list's two-path exception
- `fixes/2026-09-19-less-brittle/implementation-log.md` — this log

No performance measurement was required by this review, so
`deferred_perf_measurement` remains `false`.

Cross-OS risk was considered and judged absent: the change replaces a prefix
comparison with exact-key lookups in the same already-normalized,
repository-relative, forward-slash path space produced by `normalized_path`,
so no new platform-dependent path behavior enters the planner. No
`just cross-check` run was warranted.
