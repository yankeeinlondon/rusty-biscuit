---
title: Single-OS compile — implementation notes
kind: implementation-notes
created: 2026-09-14
spec: fixes/2026-09-12-single-os-compile/spec.md
plan: fixes/2026-09-12-single-os-compile/plan.md
audited_revision: 8aa105e7c5a180b2d2a10ffbed2332b88671c996
---

# Implementation notes

Running record for the execution plan. Each section is written by the task that
produced it and is not rewritten by a later phase; a correction is appended with
its own date rather than replacing the original observation.

## Task 1.1 — CI-cleanup prerequisite audit

**Audited revision:** `8aa105e7c5a180b2d2a10ffbed2332b88671c996` (branch
`feat/single-os`, clean working tree at audit time).
**Dependency specification:** `fixes/_completed/2026-09-11-cicd-cleanup/spec.md`
(moved to `_completed/` in `5185f1fc2`; this plan unblocked in `463b8a3a2`).

The plan names five prerequisites. Each is audited against the shipped
implementation at the revision above, not against the dependency's own
narrative.

| # | Prerequisite | Result | Evidence at the audited revision |
|---|---|---|---|
| 1 | The resolved plan is the only scheduler | **Satisfied** | `scripts/ci/affected_scope.py` is invoked for scheduling in exactly one place: `ci.yml`'s `scope` job (`ci.yml:165` fresh selection, `ci.yml:209` `--apply-to` overlay). The only other occurrences (`ci.yml:422,424,522,524`) run `test_affected_scope.py`/`test_resolved_plan.py`. `_area-ci.yml`, `_package-ci.yml`, and `_wsl-ci.yml` contain no invocation — they download the `ci-resolved-plan` artifact (`_area-ci.yml:140,209`) and read `resolved-plan.json` (`_area-ci.yml:151,274`). |
| 2 | Evidence is overlaid per cell | **Satisfied** | `ci.yml:182-216`: `local_evidence.py verify --cells` produces an accepted-cell list, which `affected_scope.py --apply-to --accepted-cells --evidence-rejections` overlays onto the plan already in hand. The overlay writes `applied-plan.json` and replaces the plan only on success (`ci.yml:216`), so a crash cannot leave a half-written plan. `apply_accepted_cells` (`affected_scope.py:2042`) performs no selection. |
| 3 | Every store remains package-keyed | **Satisfied** | Producer artifacts are `junit-<package>-<tier>-<environment>` and `status-<package>-<tier>-<environment>` in both `_package-ci.yml` (lines 275, 463, 524, 684, 849, 877, 934) and `_wsl-ci.yml` (lines 673, 806). The only area-bearing name is the per-area result *slice* `ci-results-${{ inputs.slug }}` (`_area-ci.yml:287`), which the dependency spec explicitly permits. No area-keyed store exists. |
| 4 | Every selected area owns its rollup | **Satisfied** | `ci_workflow_contracts::every_selected_area_owns_an_always_coverage_audit`, `::the_area_is_the_top_level_identity_of_the_package_fan_out`, `::a_coverage_blocked_area_still_publishes_its_result_slice`, `::runner_loss_attribution_is_narrowed_to_the_owning_area`, and `::the_area_coverage_audit_reads_plan_policy_and_baseline` all pass (93/93 in the suite run below). |
| 5 | `ci-gate` is the policy-free required-context fold | **Satisfied** | `ci.yml:613-659`: the job `needs` the six blocking top-level jobs, runs `if: always()`, and folds `needs.*.result` with `success|skipped` passing. It reads no plan, baseline, or artifact. Pinned by `ci_workflow_contracts::ci_gate_is_the_single_required_check`, `::every_top_level_job_is_either_folded_by_ci_gate_or_the_advisory_summary`, and `::only_ci_gate_makes_a_run_level_claim`. The **branch-protection migration is live**: `gh api repos/yankeeinlondon/rusty-biscuit/rulesets/19747338` reports ruleset `protect-your-bacon` with `required_status_checks = [{"context": "ci-gate"}]`. This closes closure item C10, which was open when the dependency cycle's `closure.md` was last written. |

Supporting suite results at the audited revision:

| Check | Result |
|---|---|
| `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` | 470 passed |
| `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | 93 passed, 0 skipped |

**Outstanding dependency-cycle items, and why they do not block this plan.**
`fixes/_completed/2026-09-11-cicd-cleanup/closure.md` leaves C8 (hosted
four-case fixture regression) and C9 (combined readiness review) open, and its
spec still carries `implemented: false`. Neither is one of the five
prerequisites this plan names. C8 is hosted *fixture* proof of the dependency's
own scheduling behavior; the schemas, stores, rollups, and gate this plan
extends are live, tested, and — for the gate — already the required merge
context. C10 is now closed by the ruleset read above. Recorded as a known open
item rather than a blocker.

**Ruling:** every prerequisite is satisfied. The plan's `status` is set to
`ready`. This plan does not absorb, re-run, or duplicate the dependency's
remaining hosted fixture work.

## Task 1.2 — Code-intelligence evidence

**Index:** refreshed with `just gitnexus` at the audited revision; exit 0,
"Repository indexed successfully (81.8s), 144,676 nodes | 283,820 edges | 3620
clusters | 600 flows". The failed/stale index observed during planning is
superseded and is not used as change-safety evidence.

Upstream impact for every symbol this plan edits:

| Symbol | Impacted | Risk | Epistemic | Direct callers in the graph |
|---|---:|---|---|---|
| `calculate_scope` (`scripts/ci/affected_scope.py:1805`) | 4 | LOW | exact | `main` |
| `apply_accepted_cells` (`scripts/ci/affected_scope.py:2042`) | 4 | LOW | exact | `main` |
| `validate_resolved_plan` (`scripts/ci/schema.py:431`) | 11 | MEDIUM | exact | `affected_scope.main`, `local_evidence.verify_cells`, `local_evidence.record_cells`, `local_evidence.record_cross_check`, `schema.validate_scope_receipt` |
| `matrix_record` (`scripts/ci/affected_scope.py:1659`) — the workflow-facing matrix projection | 5 | LOW | exact | `calculate_scope` |
| `plan_expected_cells` (`scripts/ci-rollup.rs:1255`) — the rollup plan reader | 3 | LOW | exact | `render_rollup` caller at `ci-rollup.rs:3675` |
| `classify` (`scripts/ci/runner_loss.py:187`) — runner-loss classification | 2 | LOW | exact | `runner_loss.main` |

**No HIGH or CRITICAL risk, and no `UNKNOWN` verdict.** `classify` is ambiguous
by bare name (20 candidates across the repo); it was resolved by uid
(`Function:scripts/ci/runner_loss.py:classify`) rather than accepting the
`UNKNOWN` aggregate.

**Lower-bound and incomplete-edge boundaries, confirmed by direct search.**
The graph walks Python-to-Python and Rust-to-Rust call edges only. Every one of
these symbols has its *real* consumers in YAML and shell, which the graph cannot
see, and the indexer additionally reported two truncations at this revision:
`[processes] 600 flows reported ... 26386 of 26586 candidate entry point(s)
never ranked in`, and `[scope-resolution] 709 property read/write site(s) name a
field defined only in another language` (the listed fields include `base`,
`head`, `head_tree`, `environments`, `expiry` — i.e. exactly the plan/receipt
document fields). Both make every count above a **lower bound**. Direct search
closes the gap:

| Symbol | Non-graph consumers found by text search |
|---|---|
| `affected_scope.py` | `.githooks/pre-push`, `.githooks/tests/test-pre-push.sh` (+ `fixtures/affected_scope_stub.py`), `.github/workflows/ci.yml`, `.github/workflows/_area-ci.yml`, `.github/workflows/_package-ci.yml`, `just/ci-local.just`, `scripts/cross-check.sh`, `scripts/ci-rollup.rs`, `.github/ci/README.md`, `.github/ci/schemas/README.md` |
| `validate_resolved_plan` | `scripts/ci/constraints.py`, `scripts/ci/local_evidence.py`, `.githooks/tests/test-pre-push.sh`, `.github/ci/schemas/README.md`, and five `test_*.py` suites |
| `matrix_record` | `scripts/ci/test_affected_scope.py` only — no non-graph consumer; its output reaches workflows through `calculate_scope`'s projection |
| `runner_loss.py` | `.github/workflows/ci.yml`, `.github/workflows/_area-ci.yml`, `.github/workflows/ci-infra-retry.yml`, `just/ci-local.just`, `.githooks/tests/test-pre-push.sh`, `.github/ci/README.md` |
| `plan_expected_cells` | `scripts/ci-rollup-tests.rs` (12 sites) — no non-graph consumer |

Treat the YAML/shell column as the authority for blast radius on any later
phase that changes these signatures or their document shapes.

## Task 1.3 — Frozen compile-path inventory

This is the pre-change state at the audited revision. Every row is a place
where a compiler runs today, traced from the resolved package record through
its workflow to the canonical recipe.

### Compile paths

| # | Path | Driver | Compiler invocation | Target selection | Cache |
|---|---|---|---|---|---|
| 1 | L1, native | `_package-ci.yml` `test` job → `just _test <pkg>` (`devops.just:845`) | `cargo nextest run -p <pkg> -E <L1 filterset>` | host default triple, `lib`/`bin`/`test` kinds | `rust-cache` `package-ci-<pkg>-test-<environment>` |
| 2 | L2, native | `_package-ci.yml` `l2` job → `just _test_l2 <pkg>` (`devops.just:1527`) | `cargo nextest run -p <pkg> -E <L2 filterset>` | same as #1 | **same** `shared-key` as #1 |
| 3 | browser, native | `_package-ci.yml` `browser` job → `just _test_browser <pkg>` (`devops.just:1708`) | `cargo nextest run -p <pkg> -E <browser filterset>` | same as #1 | **same** `shared-key` as #1 |
| 4 | WSL2 L1 archive | `_wsl-ci.yml` `archive` job on `ubuntu-latest` | `cargo nextest archive --archive-file … --target x86_64-unknown-linux-gnu <archive-args>` (`_wsl-ci.yml:203`) | explicit `x86_64-unknown-linux-gnu`; `archive-args` is package + features only, never a target-kind selector | `rust-cache` `package-ci-<pkg>-test-ubuntu-latest` — the **same key as #1–#3** |
| 5 | check | `_package-ci.yml` `check` job | `cargo check <check-args>` with explicit `--examples`/`--benches` | example/bench kinds only | `package-ci-<pkg>-check-<os>` |
| 6 | dependent seam | `_package-ci.yml` `check` job, `ubuntu-latest` only | `cargo check <dependents-check-args>` | one `-p` per unchanged direct dependent, `--lib`/`--bins`/`--tests` | shares #5's cache |
| 7 | lint | `_package-ci.yml` `lint` job → `just _lint <pkg>` (`devops.just:115`) | `cargo clippy` | package default | `package-ci-<pkg>-lint-ubuntu-latest` |

**Compatible today, and therefore Phase 2's first consolidation target:** rows
1, 2, 3 for one package and environment. They differ only in the runtime
Nextest filterset and already share one `rust-cache` key; nothing about their
Cargo invocation differs. On `ubuntu-latest`, row 4 joins them — it pins
`x86_64-unknown-linux-gnu` explicitly, which is what the hosted x86_64 runner
resolves to anyway, so this is one key serving both the Linux and WSL2 result
cells.

**Must remain separate, by design (spec §6):** row 5 (check-only target kinds
that may emit no executable), row 6 (a different package set entirely), and
row 7 (Clippy is another compiler driver with other flags). None of them
consumes a Nextest archive.

**Must remain separate, by ABI:** macOS (`macos-latest`, arm64 Darwin),
native Windows (`windows-latest`, MSVC), and the Linux/WSL2 pair are three
distinct producer environments. Native Windows and WSL2 are never compatible.

### Package-specific files needing archive includes or sidecars

| Class | Concrete instance | Produced today by | Phase 3 disposition |
|---|---|---|---|
| Non-test binary (sidecar) | six `messenger` desktop stubs (`stub_dunstify`, `stub_notify_send`, `stub_snoretoast`, `stub_burnttoast`, `stub_terminal_notifier`, `stub_alerter`) | `_package-ci.yml:351` and `_wsl-ci.yml:207-224` (`cargo build`), locally `just _build_messenger_desktop_stubs` (`devops.just:826`) | producer-owned declarative **sidecar**; already uploaded as a separate artifact on the WSL path |
| Cross-package binary fixture | `darkmatter-cli`'s `md`, needed by `claudine-cli`'s `inline_compose_hash` | `_package-ci.yml:398` `cargo build -q -p darkmatter-cli --bin md` | producer-owned **sidecar** (it is a compile-time tool, not a runtime service) |
| Harness tooling binary | `biscuit-harness-broker` | `just _test_l2` builds it at recipe time (`devops.just:1597`) — a **runtime Cargo build inside a test recipe** | must become a producer-owned sidecar or archive include; a consumer cannot build it |
| Runtime-only stub (stays on the consumer) | `ai-provider-stubs` — nine shell/`.cmd` stubs written to `$RUNNER_TEMP` | `_package-ci.yml:372-386`, no compiler | unchanged; consumer-side provisioning |
| Runtime-only tool (stays on the consumer) | `neovim`, `zed-extension`, `node-22`, `pnpm-10` | consumer provisioning steps | unchanged |
| Companion suite (not Cargo) | `homelab-frontend` → `cd homelab && just test-frontend` | `_package-ci.yml:448` | unchanged; runs beside the archive consumer |

The closed `runner-tools` vocabulary is `{ai-provider-stubs,
darkmatter-md-fixture, messenger-desktop-stubs, node-22, pnpm-10,
l2-parallel-self-spawn, neovim, zed-extension}`
(`affected_scope.py:136-144`). Phase 3 extends the *metadata* vocabulary with
archive includes and named sidecars rather than adding an arbitrary shell field;
the three compile-time entries above are the ones that move.

### Runtime Cargo builds (every one is a Phase 3/6 blocker)

| Site | Invocation | Notes |
|---|---|---|
| `just/devops.just:1597` | `cargo build -p biscuit-test-harness --bin biscuit-harness-broker` | inside `_test_l2`, on every L2 run |
| `just/devops.just:1601` | `cargo metadata --no-deps` | resolves the broker's absolute path; archive mode must avoid `cargo metadata` entirely (Task 3.4) |
| `just/devops.just:837` | `cargo build --features desktop -p messenger --bin stub_*` | skipped when `MESSENGER_STUB_BIN_DIR` is set, which is how CI already bypasses it |
| `.github/workflows/_package-ci.yml:398` | `cargo build -q -p darkmatter-cli --bin md` | guarded by a `command -v md` / `target/debug/md` probe |
| `schematic/gen/tests/e2e_generation.rs:99,136,814` | `Command::new("cargo")` at test runtime | three sites; schematic is not currently an archive consumer, but this is the pattern Task 6.5's source-string test must catch |
| `tools/test-toolkit/tests/ci_workflow_contracts.rs:80` | `Command::new("cargo")` (`cargo metadata`) | test-toolkit reads package policy at runtime |
| `tools/test-toolkit/tests/nextest_config_verification.rs:99,133` | `Command::new("cargo")` | same class |
| `unchained-ai/model_id/tests/ui.rs:3` | `Command::new("cargo")` | trybuild-style probe |

### Compile-time path coupling

- **WSL2 absolute checkout-path coupling (the one this plan removes).**
  `_wsl-ci.yml:134-152` asserts that the archive builder's workspace path
  equals the path the guest will clone to, because "every test binary in the
  archive carries this job's workspace path baked in". The guest then clones to
  `/home/runner/work/<repo>/<repo>` to match, and `BISCUIT_JUNIT_STAGE_DIR`
  (`_wsl-ci.yml:276`) is spelled as that same literal path. Task 4.4 removes
  the clone workaround; the assertion at `:134` is what must be replaced by
  manifest compatibility verification rather than deleted.
- **`env!("CARGO_BIN_EXE_…")` compile-time lookups**, to be audited in
  Task 3.6: `darkmatter/cli/tests/common/fixture.rs` (2),
  `darkmatter/cli/tests/level2_schema_about.rs`,
  `darkmatter/dmls/tests/stdio_subprocess.rs`,
  `sniff/cli/tests/level2_cicd_styling.rs`,
  `claudine/gen/tests/steering_check.rs`,
  `biscuit-terminal/lib/tests/level2_terminal_osc_wezterm.rs`,
  `biscuit-terminal/lib/tests/common/pty.rs`. The repository already ships the
  relocatable replacement, `biscuit_test_harness::bin_exe!`
  (`biscuit-test-harness/src/bin_exe.rs`).
- **`scripts/cross-check.sh`** also references `CARGO_BIN_EXE_`; Task 6.3
  revisits it.

## Task 1.4 — Compiler-work counter

See `scripts/ci-build.rs` and `scripts/ci-build-tests.rs`. Design points worth
recording:

- **Re-entry, not a persistent wrapper.** `ci-build wrap` is the value written
  into `RUSTC_WRAPPER` for exactly one measured command; it appends one event
  file, then `exec`s the real rustc. `RUSTC_WRAPPER` stays empty by default and
  no `.cargo/config.toml` or host-global wrapper is introduced, per the plan's
  execution constraints.
- **Collision-free event files without a lock.** Each invocation writes
  `<counter dir>/<pid>-<nanos>-<digest>.json`. The digest is
  `biscuit_hash::xx_hash` over the full argv plus the package and configuration
  labels, so two rustc processes that start in the same nanosecond on the same
  pid (impossible) would still need identical argv to collide. Writes are
  create-new; a collision is retried rather than overwriting.
- **`biscuit-hash`, not a new implementation.** `ci-build` is in the
  `local-tools` feature precisely so `biscuit-hash` can be a path dependency
  without entering the no-default-features `ci-rollup` build.
- **Human diagnostics are `TerminalRenderable`.** `ci-build report` renders a
  `Table` of per-package/configuration counts and `Prose` for the narrative;
  `--json` emits the versioned machine document instead.

## Task 1.5 — Instrumenting the old schedule

### The switch

`measure-compiler-work` is a boolean threaded
`ci.yml -> _area-ci.yml -> _package-ci.yml -> _wsl-ci.yml`, defaulting to
`false` at every level and exposed on `ci.yml`'s **`workflow_dispatch` only**.
`ci.yml` passes it as `${{ inputs.measure-compiler-work == true }}`: `inputs` is
null on every other event and `null == true` is `false`, so a pull request or a
push cannot turn it on without an event-name guard.

**Why opt-in rather than always-on.** The counter binary links
`biscuit-terminal` and `biscuit-hash` through the `local-tools` feature, and
`scripts/` is a separate workspace whose `target/` no CI cache key covers — so
building it costs real minutes in every measured job. Paying that on every
ordinary run would distort the very timings the baseline is supposed to capture,
and this repository's standing policy is that `RUSTC_WRAPPER` is empty by
default. A controlled cold/warm experiment is also exactly what Task 1.6 and
Task 7.4 ask for. The cost is recorded as its own `counter_setup_seconds` stage
rather than folded into setup, and it is constant across the Phase 1 and Phase 7
measurements, so it cancels in the comparison.

### The wrapper is command-scoped, not global

Each measured gate step sets `RUSTC_WRAPPER`, `BISCUIT_CI_BUILD_WRAP`,
`BISCUIT_CI_BUILD_COUNTER_DIR`, `BISCUIT_CI_BUILD_PACKAGE`, and
`BISCUIT_CI_BUILD_CONFIGURATION` in its **own** `env:` block, from the
preceding `Prepare the compiler-work counter` step's outputs. Nothing reaches
`$GITHUB_ENV`. Every other build in the same job — native prerequisites,
runner-tool stubs, the counter's own build — still sees the workflow-level
`RUSTC_WRAPPER: ""`, so wrapped and unwrapped units never mix in one `target/`.
`ci_workflow_contracts::the_compiler_work_wrapper_is_never_global` pins this:
the only non-empty `RUSTC_WRAPPER` anywhere in the four workflows must be
`${{ steps.counter.outputs.wrapper }}` at step-`env:` indentation.

### The measurement is its own artifact, not a status field

Originally drafted as extra fields inside `status.json`, then changed: the
rollup reads that document to decide a cell's outcome, and a baseline
measurement must not be able to change — or appear to change — what a cell
concluded. The measurement is published as
`measurement-<package>-<gate>-<environment>`, keyed exactly like every other
store, and `ci-rollup` is untouched.
`::the_compiler_work_measurement_defaults_off_and_produces_its_own_artifact`
pins the default-off inputs, the strict-boolean pass-through, the artifact name,
and that no producer-status step reads a measurement output.

### Advisory by construction

`Report compiler work` and `Upload the compiler-work measurement` carry
`continue-on-error: true`, which is what keeps a broken measurement out of
`ci-gate`'s fold of the job result. `only_recovery_and_diagnostic_steps_ignore_errors`
was extended to allow exactly those two steps in `test`, `test-l2`,
`test-browser`, and `_wsl-ci.yml`'s `archive`, and nothing else — `check` and
`lint` still allow none.

`Prepare the compiler-work counter` is deliberately **not** advisory. It runs
only under an explicit measurement dispatch, and a counter that failed to build
should fail loudly there rather than silently produce an unmeasured run that
looks like a valid observation.

### Shared logic lives in two `just` recipes

`just/devops.just` gains `_ci_build_counter` and `_ci_build_report`. Three
cross-platform traps are held in one place rather than duplicated across five
YAML steps: Windows needs the `.exe` suffix and a native path (`pwd -W`),
because Cargo hands `RUSTC_WRAPPER` to the OS loader and not to git-bash; the
counter directory must be emptied before the measured command so a retried step
does not accumulate two runs' events; and the bindings must be step outputs.

**Planner note (a known, benign model gap).** `affected_scope.py` decides a
just-file change gate by gate, from the recipes reachable from
`CI_RECIPES_BY_GATE` (`_test`, `_test_l2`, `_test_browser`, `_lint`) plus
`_ensure-native-libs`. The two new recipes are reached from workflow steps
directly, not through any of those entry points, so the planner classifies this
change set as `change_class: documentation` with zero package cells. That is the
correct outcome — a CI-infrastructure edit must not manufacture package gates —
but the model does not know these recipes are CI-invoked. It is safe here
because they run only under the opt-in switch and cannot alter a cell's outcome.
Revisit if a later phase makes a workflow-invoked recipe able to affect a gate.

### What each path measures

| Path | Wrapper bound to | Stages timed |
|---|---|---|
| `_package-ci.yml` `test` | `just _test` | `setup_seconds`, `gate_seconds`, `counter_setup_seconds` |
| `_package-ci.yml` `test-l2` | `just _test_l2` | same |
| `_package-ci.yml` `test-browser` | `just _test_browser` | same |
| `_wsl-ci.yml` `archive` | `cargo nextest archive` | `setup_seconds`, `archive_seconds`, `counter_setup_seconds` |

The guest is never instrumented: it has no toolchain, which is the point. Its
producer's measurement is published under
`measurement-<package>-L1-wsl2-ubuntu`, because the compile belongs to that cell
even though another machine performed it. This is the producer/consumer split
the whole plan generalizes, observed here for the first time.

`queued_seconds` is deliberately absent from the document. Parsing an ISO
run-start needs GNU `date -d` on Linux and BSD `date -jf` on macOS;
`github.run_started_at` is also missing from actionlint's context model, which
is how the trap surfaced. The document carries `job_started_epoch`, `run_id`,
and `run_attempt` instead, and the baseline reader joins them against the jobs
API once, in Python, on one host.

### Cross-OS evidence for the instrumentation

| Environment | What was proven | How |
|---|---|---|
| macOS (this host) | The whole chain: `just _ci_build_counter` → wrapped `just _test biscuit-hash` → `just _ci_build_report` → a valid `measurement.json`. `ci-build`'s 41 tests, including the cargo-driven end-to-end fixtures, pass. | local |
| Native Windows (`$BUILD_WIN`) | `_ci_build_counter`'s Windows branch: `MSYSTEM=MINGW64`, `OS=Windows_NT`, `suffix=.exe`, and `pwd -W` yielding `C:/Users/ken` — a drive-qualified path with forward slashes, which is what Cargo needs for `RUSTC_WRAPPER` and what git-bash's `$PWD` (`/c/Users/ken`) is not. `jq` 1.8.1 and `just` both present. | `ssh -o BatchMode=yes` with a transferred probe script |
| Linux (`$BUILD_LINUX`) | **Not obtained.** The host timed out during SSH banner exchange. | — |
| WSL2 (`$BUILD_WSL`) | **Not obtained.** Connection timed out. | — |

Two further Windows notes. The standing clone at `W:\ci-verification\rusty-biscuit`
was **locked** by another `cross-check` run and its `scripts/target` was cold, so
no build was attempted there: the skill's rule is that only a lock's owner
removes it, and `repo-deps` is not a workspace package `just cross-check` can
target anyway. The residual gap is therefore `ci-build` *compiled and executed*
on Windows and Linux. It is low risk — the binary carries no `#[cfg]`, no path
separator assumption, and uses `env::args_os` precisely so a non-Unicode Windows
rustc argument cannot panic the wrapper — and CI's `ci-tooling` leg runs its
suite on `ubuntu-latest` on every CI-tooling change, which closes the Linux half
on the first hosted run.

The two OS-specific idioms in `_ci_build_counter` are not new inventions: the
`pwd -W` spelling is copied from `_package-ci.yml`'s shipped messenger-stub step
and the `MSYSTEM`/`OS` detection from `_test_l2` (`just/devops.just`), both
already proven on `windows-latest` in CI.

### Local validation of the whole chain

`just _test biscuit-hash` under the wrapper, into an isolated
`CARGO_TARGET_DIR`: 26 tests passed, 70 rustc invocations recorded (2 for the
package's own crates), 18.7s of compiler time inside a 6.0s window, 5 version
probes excluded, one non-zero exit from a build-script feature probe. That last
number is why the report's prose names `autocfg`/`version_check` explicitly — a
green build normally reports a few, and a reader who did not know that would
read a healthy build as broken.

## Task 1.6 — Baseline

See `baseline-2026-09-12.md`. The instrumentation, its schema, and the exact
collection procedure are complete and locally validated; **the hosted cold and
warm observations are not collected**, so the task stays unchecked in the plan.
Three consecutive green hosted runs per environment, in controlled cold and warm
conditions, require pushing this branch and dispatching `ci.yml` six times per
environment, and this phase was instructed not to commit or push. Phases 2 and 3
read nothing from this document; Task 7.5's measured architecture gate does.

## Phase 2 — build ownership in the canonical plan

### The shape that landed

Resolved plan **version 3** adds two fields: the plan-level `builds[]` list and
the per-cell `build` reference. A build record is
`{key, package, producer, artifact, compatible_environments,
compatibility_reason, consumers[], identity}`. Result identity is untouched —
still `{package, environment, gate}` — and nothing about a build record is
area-keyed or baseline-eligible.

One shipped example, from the real planner on a `biscuit-terminal` source
change: **one** `ubuntu-latest` record whose consumers are
`ubuntu-latest/L1`, `ubuntu-latest/browser`, and `wsl2-ubuntu/L1`, plus one
macOS and one Windows record. That is the specification's central claim made
into a document: three tiers whose only compile-time difference is a runtime
Nextest filter, and a guest that consumes its host's archive, now name one key.

### Where derivation happens, and why the overlay does not

`derive_build_records` runs at the **end** of `calculate_scope`, after every
cell's execution is decided. That is what makes "an all-reused plan schedules
no owner" a property of the document rather than of a workflow condition.

`apply_accepted_cells` does **not** derive. It prunes: a cell resolved to reuse
stops referencing its build, and a record whose last consumer is satisfied is
removed. This was a deliberate constraint, not a simplification — `ci.yml`'s
scope job reaches `--apply-to` on the valid-receipt path, where R9 of the
dependency cycle says the run "pays for no Rust it does not use". Deriving
there would put a Cargo build on the fast path. Every key the overlay needs is
already in the carried plan, so it never has to.

### The hashing boundary

`ci-build` gained a `key` subcommand: it reads
`{"schema_version":1,"material":[<canonical string>, …]}` and answers
`{"schema_version":1,"keys":[…]}`, sixteen hex digits of XXH64 through
`biscuit-hash` per input. `scripts/ci/build_key.py` is the only Python caller
and has **no fallback** — an unavailable helper raises.

**Canonicalization stays on the Python side.** The helper hashes bytes it is
given, because Python's `json.dumps(ensure_ascii=True)` and `serde_json`
disagree about non-ASCII escaping, and asking two languages to independently
agree on a canonical form is how two "identical" digests drift. `schema.canonical`
is already the serializer every CI document round-trips through.

One subprocess per plan, not per record: the keys are computed as one batch,
and the `Cargo.lock` digest is cached in-process by the lockfile's own bytes.

### The feature split this forced

`ci-build` was behind `local-tools`, which pulls `sniff` — and therefore duckdb
and gix. The planner now calls it on **every** scope calculation, including the
pre-push hook's, so that gating would have made resolving a plan compile a
database engine to answer a hash. `scripts/Cargo.toml` now has three levels:

| feature | binaries | cost |
|---|---|---|
| (none) | `ci-rollup` | no monorepo crates at all — unchanged |
| `build-tools` | `ci-plan`, `ci-build` | `biscuit-hash` + `biscuit-terminal`; 201 crates, 13s cold on this host |
| `local-tools` | `repo-deps`, `drift` | adds `cargo_metadata`, `ctrlc`, `sniff` |

`just/devops.just::_ci_build_counter` and `build_key.py`'s Cargo fallback both
build with `--no-default-features --features build-tools`.
`ci_workflow_contracts::the_planner_facing_binaries_are_gated_below_local_tools`
replaces `the_plan_renderer_is_local_tools_gated` and pins all three levels.

### Toolchain and lockfile as keyed inputs

`rust-toolchain.toml` is the one place the pinned channel lives, so the key
reads it there rather than letting `environments.json` carry a second copy that
could disagree with what CI installs. The lockfile enters the key as its own
xxHash digest through the same boundary.

Consequence for fixtures: a synthetic workspace root now needs both files.
`test_affected_scope.seed_build_inputs` writes them; the alternative — letting
the planner tolerate their absence — would have made the key silently weaker in
exactly the case where it matters.

### `nextest: "latest"` is honest, and deliberate

Task 2.2 asks for a "pinned Nextest version" in each build contract. The
shipped workflows install nextest through `taiki-e/install-action@nextest` and
`https://get.nexte.st/latest/linux` — i.e. unpinned. Writing an exact release
into `environments.json` today would put a pin in data that five shipped
workflow steps contradict, which is the drift this repository's comment policy
exists to prevent.

So the field carries the version **specifier** both sides of a compatibility
edge must share, and validation enforces that the producer and its archive-only
consumer declare the same one. Pinning an exact release is a workflow change
that belongs with Task 4.4, where the archive first crosses a machine boundary
under the new contract and a producer/consumer skew becomes a real failure mode
rather than a latent one. **Recorded as deliberately deferred, not as done.**

### Structural impossibility, not a predicate check

Windows-to-WSL2 is refused twice over, on purpose. A producer's `executes` may
name itself and, beyond that, only an archive-only environment it is already the
`native_key` of — and `wsl2-ubuntu`'s `native_key` is `ubuntu-latest`, so a
Windows contract cannot reach it at all. The `arch`/`abi`/`libc` comparison
would refuse it too (`msvc` against `gnu`), but the hosting rule means a future
edit cannot get there by "fixing" a predicate.

### Shipped artifacts that had to move with the schema

Every hand-written plan in the repository is a shipped artifact of this
contract, and `test_the_plan_fixtures_are_valid_resolved_plans` in the pre-push
suite is the passive corpus test over them:

- `.githooks/tests/fixtures/plan-*.json` (5 documents) — version 3 with derived
  build records. `plan-wsl-reused.json` is the interesting one: its reused WSL2
  cell correctly leaves the Linux record with one consumer.
- `.githooks/tests/fixtures/affected_scope_stub.py` — the stub planner now
  emits `builds`, projects `build_owners`, and prunes on `--apply-to`, so the
  hook exercises the same shape the real planner produces.
- `scripts/ci/plan_fixtures.py` — new shared helper deriving build records from
  cells, used by `test_evidence_reuse`, `test_ci_local`, and the fixture
  migration. Written once because three hand-built plans would otherwise drift
  from the cells they claim to serve.

### Known, deliberate gap at the end of Phase 2

Nothing in `ci.yml` builds `ci-build` before the scope job runs, so on a plan
**miss** the planner falls back to `cargo run --no-default-features --features
build-tools`. That is correct and cheap, but it is implicit. Wiring an explicit
producer-owned build step is Phase 4's work (Task 4.1 adds the owner job);
until then the fallback is the documented path and
`BISCUIT_CI_BUILD_BIN` short-circuits it wherever a binary already exists.

### Pre-existing breakage observed, not introduced

`scripts/drift.rs` does not compile: six `no method named 'fallback_render'`
errors against `biscuit-terminal`'s current API. The file is byte-identical to
`origin/main`, so this is inherited, and CI never hits it — the `ci-tooling`
leg runs `cargo nextest run --manifest-path scripts/Cargo.toml --bin <name>`
per binary and never builds the package's full target set. Left alone: fixing
it is not this phase's scope, and a `--bin`-filtered build is how every
consumer reaches this workspace.

## Phase 3 — immutable, relocatable archives

### What the producer/consumer boundary turned out to be

`ci-build` gained two subcommands in a new module,
`scripts/ci-build-archive.rs`:

- `produce --plan --producer --out-dir` reads only its own owner slice, sorts
  it by `{package, key}`, and for each record runs **one** `cargo nextest
  archive` in one target tree, builds the declared sidecars, digests everything,
  and writes `<artifact>.manifest.json`.
- `verify --manifest --environment` is what a consumer runs before it extracts
  anything. It answers a versioned verdict document and exits `3` on a
  rejection — distinct from `2`, which still means the tool itself failed.

A rejection is a **verdict**, not an error: `run_verify` returns `Ok(false)`.
That distinction is what lets a workflow report "this cell is blocked by build
`<key>`" rather than "a tool crashed".

### The realized digest excludes the observational fields

`digest` covers the identity, the discovered tool versions, the archive and
sidecar checksums, and the binary inventory — but **not** `timings` or
`compiler_work`. Two runs that produced byte-identical artifacts have to agree
on the digest, or a consumer could never compare one build to another. It also
means editing any *claim* in the manifest breaks the digest, so tamper detection
covers the description as well as the bytes.

### Nextest 0.9.136 forced the archive-include mechanism

Three empirical findings, each of which changed the design:

1. **`relative-to = "workspace-root"` does not exist** in this version; only
   `"target"` does. A repository file therefore cannot enter the archive that
   way — which is fine, because every consumer already has the source checkout.
   `archive-includes` are consequently **build outputs**, declared relative to
   the profile output directory.
2. **A tool config's `profile.default` loses to the repository's.** The
   repository already sets `profile.default.archive.include` (the
   `discovery_probe` entries), so a generated `profile.default` in a
   `--tool-config-file` was silently ignored — proven by archiving with both and
   finding "1 extra path".
3. **A *named* profile in the tool config wins.** `produce` therefore writes
   `[profile.ci-build-archive]` and passes `--profile ci-build-archive`, and
   **merges** the repository's own `profile.default.archive.include` entries
   into it. Dropping them would remove `discovery_probe` from the archive and
   break biscuit-terminal's PTY tests in exactly the leg that cannot debug it.

The generated config is TOML built through the `toml` crate, not hand-rolled
JSON: `{"path":"…"}` is valid JSON and invalid TOML, which the first version got
wrong.

4. **Nextest does not archive a workspace `dylib`.** The fixture's
   `libarchive_portability_dylib.dylib` sits in `target/debug/` and was absent
   from the archive until declared as an include. Build-script **output
   directories** and **linked paths** *are* archived automatically, and
   `nextest list --message-format json` reports all three classes, which is
   where the manifest's `runtime_assets` come from — discovered, not guessed.

### The include vocabulary is profile-relative with platform placeholders

A package declares `examples/discovery_probe`, not
`target/x86_64-unknown-linux-gnu/debug/examples/discovery_probe`: the producer
supplies the `<triple>/<profile>` prefix its own invocation created, and
`{DLL_PREFIX}`, `{DLL_SUFFIX}`, `{EXE_SUFFIX}` cover the three producers'
spellings of one file. Validation refuses absolute paths, `..`, backslashes,
unknown placeholders, and an entry that names its own profile directory —
at *scheduling* time, because a malformed include otherwise surfaces as a failed
producer minutes into a fan-out.

### Sidecars are a data table, not a shell field

`.github/ci/sidecars.json` maps each name to `{package, features, bins,
reason}`. Both the planner and `ci-build produce` read it, so they cannot
disagree about which names exist. `darkmatter-md-fixture` and
`messenger-desktop-stubs` moved out of `KNOWN_RUNNER_TOOLS` into it;
`harness-broker` is new.

**One declaration, two readers.** The shipped `_package-ci.yml` still
provisions the first two under their old `runner-tools` name, so
`package_ci_policy` projects them back into the plan's `runner_tools` list
(`LEGACY_RUNNER_TOOL_SIDECARS`). A package therefore names the tool once. Phase
4 deletes the workflow steps and the projection together.

**`harness-broker` is deliberately not declared by any package yet.** The
vocabulary, the producer's emission, and `_test_l2`'s
`BISCUIT_HARNESS_BROKER_BIN` consumption all exist; wiring the declaration onto
the thirteen L2 packages belongs with Phase 4's consumer cutover, when there is
something to consume it. Declaring it now would rebuild the broker thirteen
times for no reader and change thirteen build keys.

### Archive mode across the three canonical recipes

`_test` already had it (the WSL leg). `_test_l2` and `_test_browser` now take
the same inputs: `-p <pkg>` moves into the filterset as `package(<pkg>) & …`,
`_archive_drop_build_flags` removes the Cargo build flags nextest refuses
alongside `--archive-file`, and a missing `cargo-nextest` is a hard error rather
than a `cargo test` fallback that would recompile.

`_test_l2`'s two runtime Cargo calls (`cargo build -p biscuit-test-harness` and
`cargo metadata`) are now skipped entirely in archive mode; the broker comes
from `BISCUIT_HARNESS_BROKER_BIN` or the tier degrades to per-test spawning, the
path it already took on a host with no broker.

**Known remaining runtime Cargo call in the L2 path:** `_backend_proof` runs
`cargo run -p test-toolkit`. It is inert unless `BISCUIT_TEST_REQUIRED_BACKENDS`
is set, and no archive consumer sets it today. Making the proof tool a sidecar
is Task 6.4 work; recorded here so it is not rediscovered as a surprise.

### The fixture, and what it actually proved

`scripts/ci/fixtures/archive-portability/` is its own three-member workspace —
never a monorepo member, never sharing its lockfile. Its archive carries an L1
binary, an L2 marker, a browser marker, a non-test executable, a Rust `dylib`, a
build-script runtime asset reached through the archived linked path, a
repository fixture read through the *runtime* `CARGO_MANIFEST_DIR`, a declared
archive include, and one sidecar class.

`the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach`
builds it in one checkout, renames the producer's target directory away, copies
the source to a second checkout, extracts to a third location, and runs all
three filtersets on a `PATH` containing only `cargo-nextest`, the verified
sidecar, and system directories with no Cargo in them. The filterset strings
come from `just _tier_filter`, not from a second copy.
`the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` does
the same through `just _test`, `_test_l2`, and `_test_browser`.

This also **empirically confirmed** the fact the whole of Task 3.6 rests on:
`--workspace-remap` rewrites the run-time `CARGO_MANIFEST_DIR`, so a fixture
read through `std::env::var` is found in the consumer's checkout.

### Task 3.6 — the audit, and what is left

| Class | Finding |
|---|---|
| `env!("CARGO_BIN_EXE_…")` in test code | **None remain.** Every site in Task 1.3's list had already moved to `biscuit_test_harness::bin_exe!` or a runtime `std::env::var` lookup. The only remaining occurrence is the macro's own fallback. |
| Producer target paths | `messenger/lib/src/tests/desktop_helpers.rs` and `biscuit-terminal/lib/tests/common/pty.rs` derive their paths from `current_exe()`, which is relocation-correct: nextest extracts test binaries to `<extract>/target/<profile>/deps`, so popping `deps` reaches the archived non-test binaries and the `discovery_probe` include. Both also honor an explicit environment override. Nothing to change. |
| Compile-time `env!("CARGO_MANIFEST_DIR")` | **131 files, 160 sites**, concentrated in `darkmatter` (45 files) and `claudine` (44). Every one is a relocation hazard for a package that becomes an archive consumer. |

The 160 sites are **not** converted here. Phase 3's mandate is the assumptions
*the fixture exposes*; the per-package sweep is Task 6.4 ("close the
archive/runtime inventory"), which wants a negative test per discovered class
rather than a bulk rewrite. What Phase 3 adds is the one-line mechanical
conversion Phase 6 needs:
`biscuit_test_harness::manifest_dir!()`, the exact analogue of `bin_exe!` —
run-time variable first, compile-time constant as the fallback.

Intentional compile-time paths, to be left alone by that sweep:

- `scripts/ci-build-tests.rs` and `scripts/ci-build-archive-tests.rs` use
  `env!("CARGO_MANIFEST_DIR")` to find the repository. They are CI tooling; they
  never run from an archive.
- `playa/lib/build.rs` reads `CARGO_MANIFEST_DIR` at build-script run time,
  which is the correct variable for a build script.
- `biscuit-test-harness/src/bin_exe.rs`'s own macro fallbacks.

### `biscuit-hash` gained a streaming BLAKE3

An archive is checksummed with BLAKE3 (a cryptographic digest for an artifact
crossing a machine boundary), while `xx_hash` stays the fast identity hash for
plan keys and counter events. `blake3_hash_bytes` would have held a
multi-hundred-megabyte archive in memory, so `blake3_hash_reader` was added
beside it rather than reaching for `blake3::Hasher` directly and creating a
second hashing boundary inside one contract.

### Validation checkpoint 3 — what was run, and what each injected fault proved

| Suite | Result |
|---|---|
| `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --features build-tools --bin ci-build` | 92 passed |
| `… --bin ci-rollup` (no default features) | 178 passed |
| `… --features build-tools --bin ci-plan` | 13 passed |
| `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | 95 passed |
| `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` | 542 passed |
| `bash .githooks/tests/test-pre-push.sh` | 66 passed, 0 failed |
| `cargo nextest run -p biscuit-hash --all-features` (L1) | 50 passed |
| `cargo nextest run -p biscuit-test-harness` (L1) | 114 passed, 1 skipped |
| `cargo clippy` for `ci-build`, `biscuit-hash`, `biscuit-test-harness`, all targets, `-D warnings` | clean |
| `python3 -m py_compile scripts/ci/*.py` | clean |

No workflow file changed in this phase, so `actionlint` had nothing new to read.

Every injected fault the checkpoint names, and the test that proves it fails
**before** any test binary starts:

| Injected fault | Test | Code |
|---|---|---|
| missing inventory | `an_archive_missing_a_declared_binary_is_refused_before_any_test_starts` | `build-inventory-incomplete` |
| extra inventory | `an_archive_of_more_programs_than_the_plan_resolved_is_refused` | `build-inventory-unexpected` |
| altered archive (same length) | `an_archive_altered_without_changing_its_length_is_still_refused`, `a_tampered_fixture_archive_is_refused_with_a_stable_code_and_exit_status` | `build-archive-corrupt` |
| altered sidecar | `an_altered_sidecar_is_refused` | `build-sidecar-corrupt` |
| missing sidecar | `a_missing_sidecar_is_refused`, `a_missing_sidecar_is_refused_end_to_end` | `build-sidecar-missing` |
| wrong source tree | `an_archive_of_another_revision_is_refused_even_when_intact` | `build-source-mismatch` |
| wrong target / feature graph | `a_build_produced_from_another_identity_is_refused` | `build-key-mismatch` |
| wrong environment / ABI | `a_windows_consumer_cannot_execute_the_linux_archive`, `a_host_of_another_architecture_refuses_the_archive` | `build-environment-incompatible`, `build-runtime-incompatible` |
| edited manifest | `an_edited_manifest_field_breaks_its_own_digest` | `build-digest-mismatch` |
| unavailable toolchain | `every_archive_mode_recipe_refuses_to_run_without_a_nextest_driver` | recipe exit 1, no `cargo test` fallback |

**No test opens or focuses a terminal or browser window.** The fixture's `level2_`
and `browser_` binaries are plain assertions that drive no backend, and every
run asserts an exact test count, so nothing else executed. The three non-archive
recipe branches were exercised against `biscuit-hash` (which owns no `level2_`
or `browser_` tests) with `BISCUIT_HARNESS_BROKER_BIN` pointed at a nonexistent
path, so the broker never spawned a pane.

### Cross-OS evidence for Phase 3

| Environment | What was proven | How |
|---|---|---|
| macOS (this host) | Everything above, including the full produce → verify → relocate → run cycle with no compiler on `PATH`. | local |
| Linux | `ci-build` and its whole dependency closure compile for `x86_64-unknown-linux-gnu`. Behavioral proof comes from CI's `ci-tooling` leg, which runs this suite on `ubuntu-latest` for every `scripts/` change. | `cargo check --target x86_64-unknown-linux-gnu` |
| Native Windows | The `#[cfg(windows)]` verbatim-prefix branch of `canonical_path` compiles for `x86_64-pc-windows-msvc`. A full cross-compile is blocked on this host by `blake3`'s assembly (`cc-rs: failed to find tool "ml64.exe"`), so the branch was checked with `rustc --emit=metadata` over an isolated copy. | `rustc --target x86_64-pc-windows-msvc --emit=metadata` |
| WSL2 | **Not obtained.** `$BUILD_WSL` timed out at the TCP level, as it did in Phase 1. | — |

`just cross-check` cannot carry this phase's main subject: it runs one
*workspace package's* L1 suite, and `repo-deps` (which owns `ci-build`) is a
separate workspace. The Linux and Windows rigs were reachable
(`ssh -o BatchMode=yes`); the WSL2 rig was not.

**The Windows trap this phase found and fixed.** `fs::canonicalize` answers
`\\?\C:\…` on Windows. Two of the arguments `produce` builds are *parsed*
rather than merely opened — nextest splits `--tool-config-file` on the colon
after the tool name, and compares `--workspace-remap` against ordinary paths —
and neither is written for a verbatim spelling. `canonical_path` strips the
prefix for a drive-qualified path under the 260-character limit and leaves a
long or UNC path alone, where an unopenable path is a louder failure than a
silently mismatched one.

**Observed, non-failing:** nextest intermittently reports one `LEAK` in the
`ci-build` suite (one run in three), and the test it names varies and is
sometimes a pure-computation one that spawns nothing. It is the leak detector
under the load of the suite's `cargo`-spawning fixtures, not a held resource.
The `scripts/` workspace has no `.config/nextest.toml`, so nextest's default
`result = "pass"` applies and it cannot turn CI red.

## Phase 4 — one Linux owner, two environments

### The shape that landed

`ci.yml` gains one top-level job, `build`. It is the only place in the run that
compiles a package's test binaries for a cut-over producer, and it schedules
nothing: its matrix is `build_artifacts`, a flattening of `build_owners`, itself
derived from the plan's `builds[]`. An all-reused plan emits `[]` and the job is
skipped whole.

Native Linux L1, L2, and browser, plus the WSL2 guest, now download **one**
artifact per package, verify it, and run it. `_wsl-ci.yml` lost its
`ubuntu-latest` archive job entirely — the second compile this whole plan exists
to remove.

### One leg per RECORD, not one per producer

Task 4.1 reads naturally as one job per producer ("install the union of
build-time native prerequisites, run `ci-build produce` once"). It is
implemented as one leg per *record* instead, for a reason that is not a
preference:

- every build artifact must keep its `{package, producer, key}` name, and
- GitHub cannot upload a dynamic number of artifacts from one job.
  `actions/upload-artifact` is a single named upload, and a `uses:` step cannot
  be looped. One job per producer would have to collapse N archives into one
  artifact — which is both an unkeyed store and a consumer downloading every
  other package's archive.

Expanding the record also gives "continue after an individual build failure so
unrelated keys can complete" an exact meaning: `fail-fast: false`, rather than a
producer job deciding how far to carry on after one key died mid-upload.

`ci-build produce` was still given the producer-scoped semantics the task
describes — it attempts every record it is handed, writes a status document for
each, and exits 1 if any failed — so a future producer-scoped invocation needs
no further change. `--key` is what narrows one leg to its own record.

### `archive_cutover`: the migration switch, in data

Phase 4 cuts Linux over while macOS and native Windows still compile in place,
so something has to say which producers are converted. That is
`environments.json`'s `build.archive_cutover`, a per-producer boolean.

The plan still derives a build record for **every** executing L1/L2/browser cell
on every producer — the record is the planner's statement of what one compile
is, and that does not change when its consumers are converted. Only the two
*workflow-facing projections* narrow: `build_owner_matrix` and a package's
`matrix.builds`. An unconverted consumer therefore sees no record for its
environment, takes the old compile-in-place path, and never looks for an
artifact no owner uploaded.

The alternative — filtering in `ci.yml` — would have put migration state in a
workflow condition where no test can see it. Phase 5 flips two booleans; Task
6.5 removes the field with the paths it guarded.

### A build is not a result cell

`build-status-<package>-<producer>-<key>/build-status.json` is published under
`always()` for success, compile failure, upload failure, and cancellation. It
carries the realized digest and stage timings where the producer got far enough
to have them, and the consumers the record was resolved for. It is **not** a
cell: no `{package, environment, gate}` identity, no JUnit, no `status-…`
artifact, never baselined. A workflow-contract test pins that the owner job
publishes none of those names.

`ci-rollup` reads the documents by key and renders a dependent cell as
`MISSING — blocked by build <key> (<package> on <producer>) concluded `failure`
at the compile stage: …`. MISSING blocks, and the skip baseline judges skipped
*test identities* only, so a failed build can never be excused into a merge.
Three deliberate asymmetries:

- **A real test result outranks the plumbing diagnostic.** A cell that produced
  failing or passing tests reports those; the build note is added as context.
  Only a cell with nothing to show is *blocked*.
- **An absent status is not a block.** Producers before their cutover reference
  records no owner job was scheduled for; inferring a block from that absence
  would fail every one of them.
- **On a rerun that publishes both attempts, the failure wins.** An owner leg
  that failed on either attempt did not deliver the archive this attempt's
  consumers were told to download.

A runner-lost owner writes no status at all, so `runner_loss.py attribute`
gained `--plan` and synthesizes one from the plan's own record. A build job
owns no cell, so `parse_job_name` still answers `None` for it; a *real* build
failure is not a lost runner and still vetoes the one-shot retry.

### The verifier travels inside the artifact

A consumer cannot build `ci-build`: the WSL2 guest has no Cargo at all, and a
native consumer that installed a toolchain to verify an archive would defeat the
point. The owner copies the binary it already built into
`<artifact>/tools/ci-build` before uploading.

Shipping it as a separate artifact was rejected: two artifacts can drift apart,
and the guest would need a second download for one file. The integrity concern
is the honest counter-argument — a tampered artifact could ship a tampered
verifier — but this boundary detects accidental corruption and producer/consumer
skew, not an adversary with write access to the run's artifacts.

**Verification runs in the guest, not on its Windows host.** `host_runtime()`
reads `cfg!`, so a verifier executed on the `windows-latest` host would report
`msvc` and prove nothing about the machine that runs the tests.

### `wsl-bash` cannot write `$GITHUB_OUTPUT`

The first draft had the guest's verify step emit step outputs. It cannot:
`wsl-bash` forwards only WSLENV-listed variables, and `$GITHUB_OUTPUT` is a
Windows path the guest could not write even if it were forwarded. Every value a
later guest step needs — the realized digest, the archive file name, the
producer's workspace — is therefore read on the **host** by a `Read the
producer's manifest` step and reaches the guest through GitHub expression
interpolation, which is the same discipline the existing
`BISCUIT_CI_ENVIRONMENT` re-export already follows.

### The guest path, and the coupling that is still there

`_wsl-ci.yml`'s old `Verify the guest path assumption holds` step asserted that
the archive builder's `GITHUB_WORKSPACE` equalled the literal the guest cloned
to. That step is gone. What replaced it is *not* an arbitrary guest path:

`--workspace-remap` rewrites the **run-time** `CARGO_MANIFEST_DIR`, but **163
sites across 131 files** still read the **compile-time** `env!` (Task 3.6's
audit; `darkmatter` and `claudine` hold two thirds of them). Those open the
producer's path whatever the guest does. Relocating the guest checkout in this
phase would have turned those packages' WSL2 cells red for a reason that has
nothing to do with this phase's subject.

So the manifest gained `producer_workspace` — the absolute workspace root the
producer compiled at, forward-slashed, inside the realized digest — and the
guest clones to whatever it says. The coupling is now **data-driven and
self-correcting** rather than a literal that drifts the day a runner image moves
its workspace, and it is recorded here as the one remaining compile-time path
assumption. Task 6.4 moves those sites to `biscuit_test_harness::manifest_dir!()`
and frees the guest to clone anywhere. The archive **and** the verifier go to
`/home/biscuit/build`, independent of the checkout, so nothing about the
extraction can be mistaken for source.

**Manifest schema version 2** carries the new field; a version-1 manifest misses
cleanly as `build-manifest-schema`.

### Everything the consumer must not ask Cargo for

Task 4.3 says a consumer contains no Cargo invocation. Three paths would have
reached one on a hosted Linux runner — where Cargo exists, so they would have
succeeded silently and failed only in the toolchain-free guest:

| Path | Fix |
|---|---|
| `_stage_junit`'s `cargo metadata` fallback for the workspace root and target dir | archive mode exports `BISCUIT_JUNIT_WORKSPACE_ROOT`/`BISCUIT_JUNIT_TARGET_DIR` |
| `insta`'s own `cargo metadata` workspace-root probe | archive mode exports `INSTA_WORKSPACE_ROOT` |
| `_backend_proof`'s `cargo run -p test-toolkit` | new **`backend-proof` build sidecar**; the recipe prefers `BISCUIT_BACKEND_PROOF_BIN` |

`BISCUIT_NEXTEST_BIN='cargo-nextest nextest'` is set for the same reason: the
standalone driver, never `cargo nextest`. All four are the exact knobs the WSL2
guest already used — one spelling for both consumers rather than two.

**One deliberate exception, recorded rather than fixed.** `Prepare the
compiler-work counter` builds `ci-build` with Cargo, and it is not conditioned
on archive mode. It runs only under the opt-in `measure-compiler-work` dispatch,
and measuring a consumer is the point: a consumer's compiler-work count should
be **zero**, which is the single cleanest proof the cutover worked. The
contracts therefore assert the *gate* step reaches no compiler, not the whole
job. Task 7.4 owns what that measurement says.

### The sidecars Phase 3 deferred are now declared

`harness-broker` and `backend-proof` are declared by the eleven packages whose
L2 tier is CI-hostable (their `l2-backends` include `tmux`; a package whose L2
declares only GUI emulators renders a POLICY GAP and never executes the tier, so
declaring the sidecars there would rebuild them for no reader and change the
build key for nothing). `darkmatter-lib` and `biscuit-terminal-lib` are the two
deliberate omissions.

The legacy `runner-tools` steps for `messenger-desktop-stubs` and
`darkmatter-md-fixture` are **retained**, guarded by
`!steps.build.outputs.archive`: macOS and native Windows still need them.
`LEGACY_RUNNER_TOOL_SIDECARS` goes with those steps when the last producer is
cut over, not here.

### `has_builds`, and the trap it avoids

The owner job's `if:` reads `needs.scope.outputs.has_builds == 'true'`, not
`build_artifacts != '[]'`. On the reuse path the `scope` job is **skipped**, so
every one of its outputs is the empty string — and `'' != '[]'` is true, which
would have fanned the owner matrix out over `fromJSON('')`. `area-ci`'s
`has_packages` already had the right shape; this follows it.

### Validation checkpoint 4 — what was run

| Check | Result |
|---|---|
| `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` | 574 passed |
| `bash .githooks/tests/test-pre-push.sh` | 66 passed, 0 failed |
| `cargo nextest run … --features build-tools --bin ci-build` | 96 passed |
| `cargo nextest run … --no-default-features --bin ci-rollup` | 188 passed |
| `cargo nextest run … --features build-tools --bin ci-plan` | 13 passed |
| `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | 103 passed |
| `actionlint .github/workflows/*.yml` | clean |
| `cargo clippy` for `ci-build`, `ci-rollup`, `test-toolkit`, all targets, `-D warnings` | clean |
| `python3 -m py_compile scripts/ci/*.py` + the hook's stub | clean |

**One Linux build, two environments** is proven as a property of the documents
and of the shipped workflows rather than of a hosted run:
`test_one_linux_build_feeds_native_linux_and_the_wsl2_guest` (the real planner,
real workspace: one `ubuntu-latest` key whose consumers are the Linux L1, a
Linux higher tier, and `wsl2-ubuntu` L1 — one artifact name through both
projections, two distinct cells),
`the_wsl2_guest_consumes_the_same_build_as_native_linux` (both legs download
`${{ steps.build.outputs.artifact }}`, resolved from the same
`inputs.builds`), and `the_linux_archive_is_accepted_by_its_wsl2_guest_as_well`
(the verifier accepts one manifest for both environments). Relocation itself —
producer target hidden, second checkout, third extraction path, no compiler on
`PATH` — is Phase 3's `the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach`,
re-run green here.

Injected faults, and the test that proves each:

| Fault | Test |
|---|---|
| producer compile failure | `a_failed_key_still_reports_itself_and_lets_an_unrelated_key_finish` (ci-build), `a_cell_whose_build_failed_is_missing_and_names_the_build` (rollup) |
| cancellation, upload failure | `a_cancelled_or_unuploaded_build_blocks_its_cells_just_as_a_failed_one_does` |
| one unrelated successful key | `a_failed_build_leaves_an_unrelated_cell_untouched`, `a_key_narrowed_leg_produces_only_its_own_record` |
| missing artifact / lost runner | `a_lost_build_owner_is_attributed_to_its_build_record` (runner-loss), `an_archive_miss_never_enters_a_fallback_build` (workflow) |
| corrupt transfer, consumer setup failure | `a_consumer_that_refused_its_build_reports_the_refusal` (rollup); the verifier's own codes are Phase 3's |
| a real result outranking the diagnostic | `a_real_test_result_outranks_the_build_diagnostic` |
| a reused cell that must not be blocked | `a_reused_cell_consumes_no_build_and_cannot_be_blocked_by_one` |

**No test opens or focuses a terminal or browser window.** Nothing in this phase
runs an L2 or browser tier: the workflow contracts read YAML, the rollup and
runner-loss tests are pure, and the `ci-build` fixtures drive the
archive-portability workspace whose `level2_`/`browser_` binaries are plain
assertions with no backend.

### Cross-OS evidence for Phase 4

| Environment | What was proven | How |
|---|---|---|
| macOS (this host) | Every suite above, plus an end-to-end `just _ci_build_verify` over a produced fixture: a matching workspace verifies, a byte-appended archive is refused with exit 3. | local |
| Linux | `ci-build` and its closure compile for `x86_64-unknown-linux-gnu`. Behavioral proof comes from CI's `ci-tooling` leg, which runs these suites on `ubuntu-latest` for every `scripts/` change — and from the cutover itself, which is a Linux-only change. | `cargo check --target x86_64-unknown-linux-gnu` |
| Native Windows | **Not exercised, and deliberately out of reach this phase.** `archive_cutover` is `false` for `windows-latest`, so no Windows consumer takes the archive path; the only Windows code this phase touches is the owner job's `.exe` suffix on the staged verifier, which Phase 5 is the first run to execute. | — |
| WSL2 | **Not obtained.** The guest leg's behavior is hosted-only: it needs a `windows-latest` runner with nested virtualization. The rig timed out at the TCP level in Phases 1 and 3 and was not retried. | — |

The residual hosted risk is concentrated in one place: the guest step order
(download → manifest read on the host → clone to `producer_workspace` → verify
in the guest → run). `actionlint` covers the expression syntax; nothing local
can cover a WSL2 guest.

### Pre-existing lint fixed in passing, and named

`cargo clippy -p test-toolkit --all-targets -- -D warnings` was already failing
at `HEAD` on `clippy::if_same_then_else` in `ci_workflow_contracts.rs` (the
`inputs.companion-suites` and `inputs.dependents` branches both answer `"[]"`).
`test-toolkit` is `gates = false`, so no CI leg lints it and the breakage was
invisible. The two branches were merged so this phase could report a clean lint;
it is a one-line change with no behavior in it.

## Phase 5 — macOS and native Windows owners

### The cutover itself was two booleans; the work was everything Windows

`archive_cutover` is `true` for all three native producers now. Flipping the two
remaining ones changed no scheduling logic at all — the plan already derived a
record for every executing L1/L2/browser cell on every producer, and only the
two workflow-facing projections (`build_owner_matrix` and a package's
`matrix.builds`) were narrowed. The owner job's matrix, `runs-on`, native
prerequisites, cache key, and every consumer step were already written against
those projections in Phase 4, so no branch was added anywhere for macOS or
Windows.

What the flip *did* do is send an archive down a path nothing in this plan had
ever executed. Two of the three defects below are Windows-only, and one of them
was silent everywhere else.

### The runner label is not the compatibility authority

Task 5.1 asks for the compiler host and target to be validated "rather than
inferring compatibility from `runner.os`". This is `preflight_toolchain` in
`ci-build-archive.rs`, called per record in `run_produce` *before*
`produce_one`:

- `rustc -vV`'s `host:` must equal the record's `identity.host`;
- a `target` that differs from the host must have a standard library the
  toolchain can actually name (`rustc --print target-libdir --target …`, then
  the directory must exist — `target-libdir` answers for any *recognized*
  triple, installed or not).

A refusal writes a status with a new `stage`, `preflight`. It is per record
rather than per leg because two records owned by one producer can name different
targets, and because `fail-fast: false` only means something if a sibling key
still finishes.

**Why this is not belt-and-braces.** Compiler host and target are *keyed*
inputs. An archive produced by the wrong toolchain therefore **verifies**: the
consumer compares the manifest against the key the planner computed, not against
what actually ran. The failure would surface as every test in the cell dying at
exec time, on an environment whose plan said it was compatible. The one witness
that cannot be fooled is the toolchain about to run.

This also turns a future GitHub runner-image change into a named failure. When
`macos-latest` moves off `aarch64-apple-darwin`, the owner leg refuses at
preflight with both triples in the message, instead of shipping x86 binaries to
an arm consumer.

### `$PWD` is an MSYS path, and it reached four native programs

Under Git Bash — what `shell: bash` gets on `windows-latest` — `$PWD` is
`/d/a/repo/repo`. The L1, L2, and browser gate steps handed that value to
`--workspace-remap`, `INSTA_WORKSPACE_ROOT`, `BISCUIT_JUNIT_WORKSPACE_ROOT`, and
`BISCUIT_JUNIT_TARGET_DIR`; `_ci_build_verify` handed it to the verifier as
`--workspace`. None of those is an MSYS program. In the same shell
`$RUNNER_TEMP` is the *opposite* problem — a Windows path (`D:\a\_temp`) being
joined and tested by MSYS tools.

`just _native_path` answers the one spelling both layers accept:
`cygpath -m`, drive-qualified with forward slashes and no `\\?\` verbatim prefix
(the prefix that broke nextest's own argument grammar in Phase 3). Measured on
`build-win-native`, 2026-09-14, in the exact call shapes the workflow uses:

| input | answer |
|---|---|
| `$PWD` (`/w/ci-verification/rusty-biscuit`) | `W:/ci-verification/rusty-biscuit` |
| `$RUNNER_TEMP`-shaped `D:\a\_temp/build` | `D:/a/_temp/build` |

`_ci_build_verify` computes it **once** and publishes it as a `workspace` step
output; the three gate steps bind it as `ARCHIVE_WORKSPACE`. One computation
rather than three that must agree. Off Windows — and in the WSL2 guest, which
runs the same recipe — it is the identity function.

### The defect the fixture found: backslashes do not survive a recipe's `*args`

`just` pastes `{{ args }}` raw into `forwarded=({{ args }})`, so **bash both
word-splits the list and processes backslash escapes.** A native Windows
`--archive-file=C:\Users\ken\AppData\Local\Temp\…\x.tar.zst` reached nextest as
`C:UserskenAppDataLocalTempx.tar.zst`, and nextest said only

    error: error extracting archive `C:Usersken…`
    Caused by: The system cannot find the file specified. (os error 2)

— a missing-file error for a path nobody typed. Found by
`the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` on
`build-win-native`, 2026-09-14. It is invisible on macOS and Linux, and it would
have been invisible in CI too, because `_native_path` had already removed every
backslash from the values the workflow passes.

Two changes, because the mangled value and the confusing diagnostic are separate
faults:

- the fixture now hands `just` what CI hands it, through the shipped recipe
  (`recipe_path` shells out to `just _native_path`) rather than a second copy of
  its rule — which makes every archive-mode fixture an end-to-end test of the
  helper;
- `_test`, `_test_l2`, and `_test_browser` capture the `--archive-file` value in
  the loop that already detects archive mode and call `_archive_file_check`,
  which refuses a path it cannot read, prints it, and on Windows names the
  backslash hazard. It runs **after** the missing-driver check, so
  `every_archive_mode_recipe_refuses_to_run_without_a_nextest_driver` keeps its
  exact meaning: no driver at all outranks one unreadable archive.

`_expected_manifest` deliberately does **not** call it — that runs at staging
time, after the tier has already read the archive, and only needs to know that
it did.

`_native_path` is itself backslash-safe: its argument is interpolated inside
single quotes, so a caller may hand it a native spelling and ask it to fix one.
`the_native_path_helper_answers_a_spelling_a_recipe_can_carry` pins that on
every OS.

### Check and lint are counted, not just excused

Specification section 6 asks for check and lint work to be "counted as a
separate configuration reason, not as an unexplained duplicate". Both jobs keep
their toolchain, cache, feature selection, and result identity exactly as they
were — the change is that each now carries the same command-scoped compiler-work
counter the test tiers carry, labelled `check <environment>` and
`lint ubuntu-latest`, and publishes a `{package, environment, gate}` measurement
document of its own.

Without that, Task 7.4's post-cutover measurement would show one archive plus
two unattributed compiles per package. With it, every compile in the run has a
name. `check`'s two `cargo check` invocations (the package's own example/bench
kinds and the unchanged-dependent seam) share one configuration, because they
are two halves of one cell.

Both measurement steps are `continue-on-error: true`, which is what keeps a
broken measurement out of `ci-gate`'s fold — the same structural rule the test
tiers follow, and `only_recovery_and_diagnostic_steps_ignore_errors` now expects
the pair in all five jobs.

**One Windows compile is deliberately left outside this accounting.**
`biscuit-tui-windows-captured-stdout.yml` is a specialized runtime contract with
its own cache and a `cargo test` invocation. It owns no build record, is not a
`_package-ci.yml` cell, and is separately scoped work.

### The owner in `ci-gate`, and why it is not double-counting

Task 5.5 asks for the owner job to be in the gate "only for unrepresented
infrastructure failure". It is, and the `needs:` list now says so: a build that
failed to *compile* is already a cell — the rollup renders every dependent cell
`MISSING — blocked by build <key> …` and the skip baseline cannot excuse it.
What no cell can show is an owner that never reached a compile: a lost runner, a
cancellation, an archive that could not be uploaded.
`the_owner_job_reaches_the_gate_as_infrastructure_and_owns_no_cell` pins the
three halves of that — the owner publishes no `status-`/JUnit artifact and holds
no `checks:` scope, it may not carry `continue-on-error` (which would fold as
success and hide a lost archive), and the area fan-out waits for it under
`!cancelled()`.

Reusable-workflow depth and the accepted-gap publisher's `checks: write` were
already pinned by `the_reusable_workflow_chain_stays_within_githubs_four_levels`
and `the_gap_publisher_is_the_only_job_holding_checks_write`; both still pass
unchanged, which is the whole of the "preserve" half of the task.

### Nothing else needed an OS branch

Everything below was checked and left alone, because the Phase 3 and 4 contracts
already covered it and Rule 3 says so:

| Concern | Why it needed nothing |
|---|---|
| `.exe` sidecars | `_ci_build_verify` already derives the suffix from `MSYSTEM`/`OS`, and the consumer steps already probe both spellings for every *bound* sidecar. A sidecar resolved from `PATH` needs nothing — `PATHEXT` finds the `.exe`. |
| DLL discovery | `archive_includes` spell `{DLL_PREFIX}…{DLL_SUFFIX}`, expanded per producer; the fixture's `dylib` member proves the whole path, and it passed on Windows. |
| `USERPROFILE`-based home | No consumer step and no archive-mode recipe reads a home directory. The one repository component that resolves a Windows home is `scripts/ci/constraints.py`, which CI deliberately never reads. |
| Target directories | The producer uses the host's default; nothing overrides it. |
| Handle cleanup | The fixtures' scratch directories already tolerate a failed removal, and the Windows suite left none behind. |
| tmux on macOS L2 | Runtime provisioning, deliberately NOT conditioned on the archive — an archive consumer needs it exactly as much as a cell that compiled in place. |

### Validation checkpoint 5 — what was run, and where

| Check | macOS (this host) | native Windows (`build-win-native`) | Linux (`build-linux`) |
|---|---|---|---|
| `ci-build` suite, incl. the archive fixture end to end | 101 passed | **101 passed** | 101 passed |
| `ci_workflow_contracts` | 111 passed | 110 passed (one `#[cfg(unix)]`) | 111 passed |
| `python3 -m unittest discover -s scripts/ci` | 576 passed | — | 576 passed |
| `actionlint .github/workflows/*.yml` | clean | — | — |
| `bash .githooks/tests/test-pre-push.sh` | 66 passed, 0 failed | — | — |
| `cargo nextest … --bin ci-rollup` / `--bin ci-plan` | 188 / 13 passed | — | — |
| clippy, `ci-build` + `test-toolkit`, all targets, `-D warnings` | clean | — | — |
| `python3 -m py_compile scripts/ci/*.py` | clean | — | — |

The Windows column is the point of this checkpoint. `ci-build`'s fixture suite
*is* the producer contract: it produces an archive for the record, verifies it,
hides the producer's target directory, copies the source to a second checkout,
extracts to a third location, and runs all three tier filtersets through the
canonical `just` recipes with no Cargo, rustc, or linker on `PATH` — on native
Windows, against `x86_64-pc-windows-msvc`, for the first time in this plan. Its
injected faults (tamper, wrong environment, missing sidecar, short and long
inventory, foreign compiler host, unbuildable target) all refuse there too.

**No test opened or focused a terminal or browser window.** The fixture's
`level2_` and `browser_` binaries are plain assertions that drive no backend,
every run asserts an exact test count, and the Windows run was an
`ssh -o BatchMode=yes` session with no interactive console.

### Two environment facts recorded rather than worked around

- **`build-win-native` is short on disk.** `_storage_preflight` refused the
  recipe fixtures at 37.4 GiB free against a 50 GiB floor, and its automatic
  Cargo reclaim could not recover the headroom: an unrelated 104 GiB
  `Ubuntu-26.04 ext4.vhdx` holds 37% of that volume. The verification run set
  `BISCUIT_BUILD_MIN_FREE_GIB=0` — a bounded, intentional override for fixtures
  that compile a three-crate workspace into a temp target directory. Reclaiming
  the vhdx needs `just wsl-compact`, which is elevated and ends any running WSL
  session on a shared rig; that is a host maintenance decision, not this phase's.
- **`build-linux`'s shared `ci-verification` clone was locked** by an unrelated
  `nightly-reward-spike` run for the whole of this session. The Linux evidence
  was taken in a separate `~/p5-verify` clone rather than by waiting on, or
  removing, someone else's lock.

### A footgun worth knowing before trusting a green `ci-build` run

`shipped_wrapper()` rebuilds `scripts/target/<profile>/ci-build` only when the
file is **absent**. `cargo nextest run --bin ci-build` builds the test harness,
not necessarily the bin, so a stale binary from an earlier build can serve every
end-to-end fixture and make a suite pass against code that is no longer there.
It cost a confusing failure in this phase. Run
`cargo build --manifest-path scripts/Cargo.toml --no-default-features --features build-tools --bin ci-build`
first, as CI's `ci-tooling` leg effectively does.

## Phase 6 — dependency work, local and cross-host alignment

### The defect that made every earlier phase unprovable

`ci-build produce --plan <resolved-plan.json>` — the only way CI ever calls it —
could not read a single record the planner writes:

    ci-build: reading the plan's build records: invalid type: string "", expected a sequence

`schema.py::BUILD_IDENTITY_FIELDS` validates `identity.features` as a **string**
(one command-line fragment: `--features a,b`, `--all-features`, or empty) and
the planner writes one. `Identity` in `ci-build-archive.rs` declared
`Vec<String>`. Every Rust fixture in Phases 2–5 hand-wrote `"features": []`, so
all 101 of them passed against a shape the planner never produces.

Fixed by making the Rust field a `String`, split on whitespace where it is
handed to Cargo and digested as written everywhere else. The guard is
`every_build_record_the_shipped_planner_writes_round_trips_through_this_reader`:
it runs the real planner with `--all`, reads the document through `read_plan`,
and compares each re-serialized record to the planner's own object. A field
whose *type* differs between the two languages is exactly what no hand-written
fixture can catch, and `deny_unknown_fields` already covered the other
direction.

### `examples/` is the one include class the archive build does not produce

An `archive-includes` entry **copies** what the build produced, and `cargo
nextest archive` builds lib, bin, and test targets — never an example. The
repository carried `biscuit-terminal`'s `discovery_probe` as a repository-wide
`profile.default.archive.include` with two hard-coded layouts
(`debug/examples/…` and `x86_64-unknown-linux-gnu/debug/examples/…`) and
`on-missing = "ignore"`. `ci-build produce` always passes `--target`, so:

- the macOS and Windows producers matched **neither** spelling;
- nothing built the example in any producer at all;
- `on-missing = "ignore"` made all of that silent.

`biscuit-terminal`'s PTY tests `panic!` on a missing probe, and they are L1 —
so every archive consumer of that package was going to fail with
`discovery_probe example not found`, on every environment, for a reason no test
named.

Two changes. `ci-build produce` recognizes a declared include naming a direct
child of `examples/` and builds that target before archiving (only a direct
child: Cargo has no nested example targets, so a deeper path is some other
build output). And `discovery_probe` moved out of `.config/nextest.toml` into
`biscuit-terminal/lib`'s own `archive-includes`, where `include_path` spells it
under whatever triple the producer used. Measured on this host, 2026-09-14:

| | before | after |
|---|---|---|
| macOS `biscuit-terminal` archive | no probe, silently | `aarch64-apple-darwin/debug/examples/discovery_probe`, 49 970 752 bytes, digested |

The fixture workspace gained an `examples/probe.rs` so this is covered
end-to-end rather than only by the real package, and
`an_example_include_naming_no_target_fails_the_record_at_compile` pins that a
broken declaration fails the record instead of shipping without it.

### Task 6.1 — the fixture is a discriminator, not a demonstration

`scripts/ci/fixtures/shared-deps` is four crates: `alpha` and `beta` over
`common` (configured identically) and `divergent` (`alpha` asks for its `extra`
feature, `beta` does not). Producing both records into one owner target tree,
measured through the compiler-work counter:

| crate | rustc invocations |
|---|---|
| `shared_deps_common` | **1** |
| `shared_deps_divergent` | **2** |

That is the whole claim: the identical dependency compiles once across two
separate archive invocations, the divergent one is two units and compiles twice.

The fixture is a *discriminator* because the combined invocation is red.
`cargo nextest run --workspace` over it fails
`beta_observes_no_feature_it_did_not_ask_for`: Cargo unifies features across the
selected packages and `beta` links a `divergent` it never asked for.
`a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart`
pins that, which is what Task 6.1's "permit a combined invocation only if this
fixture remains equivalent" reduces to in practice: it is not equivalent, and a
later "one invocation is faster" change cannot be made quietly.

### The producer had no way to measure itself

`--counter-dir` existed on `produce` and only ever *read* a directory nothing
wrote: nothing set `RUSTC_WRAPPER` for the Cargo and Nextest children, so a
measured producer recorded zero and a manifest's `compiler_work` could only
reflect some other command's events.

`measurement_env` now sets the command-scoped wrapper on those children when
`--counter-dir` is given, and on nothing else — the producer's own process
environment is untouched, which matters because `ci-build` re-enters as the
wrapper when `BISCUIT_CI_BUILD_WRAP` is set and would otherwise never reach its
own subcommand. Each record counts into `<counter-dir>/<artifact>`, because a
flat directory folds two records compiled in one leg into each other's
manifests and "this dependency compiled once across both records" needs them
separable.

**Not wired into `ci.yml`'s owner job.** That is Task 7.1/7.4's measurement
surface; this phase made the producer capable of being measured and left the
dispatch to the phase that reads it.

### Task 6.2 — the local run reports a key it never serializes

`just ci-local` already compiled once for every tier it runs: one process tree,
one `CARGO_TARGET_DIR`. What it could not do was say *which* build that was. It
now prints the planned key of every cell resolved for this host's environment,
grouped by `[package, key]` so a package that somehow resolved two keys shows as
the two lines it is, and it serializes no archive — there is no second machine
to hand one to. Host detection became best-effort (it was required only for
`--l2`/`--plan-in`), so a host without `sniff` keeps working and loses only the
report.

### Task 6.3 — cross-check is a producer/consumer pair on every host

`scripts/cross-check.sh` ran `cargo nextest run` natively everywhere except
`--os wsl`, which built its own archive on the guest. Both are gone. Every host
now gets the same seven steps: build `ci-build`, `produce` the record the
shipped plan names, stage the verifier beside the archive, copy the whole
producer directory into one the consumer owns, materialize a second checkout as
a `git worktree` at another path, hide the producer's target tree, verify
through `just _ci_build_verify`, and run the canonical tier recipe in archive
mode.

Three decisions worth recording:

- **The plan is resolved locally, once, and shipped.** Recomputing it on each
  host would make "the same build key" a coincidence; `--all` is used so a
  smoke run is never limited to what happens to be in scope, and it costs 0.6 s.
- **A `git worktree`, not a copy.** `tar`-ing the tree minus `target` is 2.2 GB
  on this repository (nested `scripts/target`, `node_modules`); a worktree is
  the same tree at another path for free, and the patch the host already has is
  re-applied into it.
- **A build flag selects the native path.** In archive mode the plan's declared
  feature arguments *are* the archive's, so honoring `--features terminal-tests`
  would change the key the run reports. Passing one now falls back to the old
  `cargo nextest run` for every host, announced, and publishes nothing — which
  keeps the documented `BISCUIT_TEST_REQUIRED_BACKENDS=tmux … --features
  terminal-tests level2_` invocation working verbatim.

A qualifying WSL receipt now carries `cells[].build = {key, digest}`, read from
the manifest the remote **verified** rather than from the local plan's
expectation. The field is optional in `RECEIPT_CELL_FIELDS`, so every receipt
written before archives existed still validates and the native path records no
half-named build.

**What cross-check's WSL leg does and does not prove.** It produces the
`ubuntu-latest` record *on the guest* — a real `x86_64-unknown-linux-gnu` host,
so `preflight_toolchain` accepts it and the key is the one CI's Linux producer
computes — and consumes it as `wsl2-ubuntu`. That proves the archive contract
and relocation on a real WSL2 guest. It does not prove the machine-to-machine
transfer; that edge is CI's, and `_wsl-ci.yml` is where Phase 4 proved it.

`scripts/ci/test_cross_check.py` is the automated half: `ssh` and `scp` are
stubs that record their arguments and stage the shipped files, and every
assertion is about the bytes a build host would have run — the producer command
with the plan's own key, verification before the tier, the hidden target, the
`git worktree`, the `cross-check-*` markers, `BatchMode=yes` on every
invocation, and the native escape hatch shipping no plan.

#### Proven end to end on native Windows

`just cross-check --os windows biscuit-hash`, `build-win-native`, 2026-09-14 —
the whole sequence, on a real host:

| Step | Observed |
|---|---|
| producer tool | `cargo build --release … --bin ci-build` |
| `ci-build produce` | `build-biscuit-hash-windows-latest-43fc98e60a661219.tar.zst`, 15 files |
| key / realized digest | `43fc98e60a661219` / `17745a8f0a11fc3f` |
| second checkout | `git worktree` at `W:\ci-verification\…-consume\src` |
| producer target | renamed to `target.hold`, restored in `finally` |
| `just _ci_build_verify` | accepted |
| `just _test` in archive mode | **50 tests run: 50 passed**, `cross-check-exit: 0`, 19 s |

`_native_path`, `git worktree`, the `.exe` producer tool, and the backslash-free
`--archive-file` spelling all held. Two runs over two different working trees
answered the same **key** and different **realized digests**, which is the
contract working: the key is the plan's identity (`source_commit` is the local
HEAD, unchanged), the digest covers what was actually compiled.

The host is left clean — no lock, patch, plan, script, `consume`, `out`, or
report survives. The staged JUnit report needed its own disposal: it
deliberately outlives the remote run because the WSL leg's receipt fetches one
afterwards, so every other host is told to drop it and the WSL path drops it
right after the local copy is made rather than on each of the four paths that
decline to publish. Left alone, a shared rig accumulates one per run forever —
the old script already did, for the WSL leg. The first attempt refused at
`_storage_preflight` (35.9 GiB free against a 50 GiB Windows floor, the same
unrelated `Ubuntu-26.04 ext4.vhdx` the Phase 5 notes named); the run above set
`BISCUIT_BUILD_MIN_FREE_GIB=0`, which `cross-check` now validates and forwards
to the remote exactly as it forwards `BISCUIT_TEST_REQUIRED_BACKENDS`. The floor
itself is left alone: it is Windows-only, hosted runners clear it, and the rig's
free space is a host maintenance decision.

#### The defect that first attempt found: a failing run reported `pass`

The tier exited 1 at the storage floor and the summary said `windows  pass`.
`$code = Invoke-CrossCheck` binds a PowerShell **function's output stream**, and
every native command inside it — `git clone`, `git worktree add`, `just` —
writes its stdout into that stream. `$code` was an array whose first element was
a git message, and `exit $code` reported that.

The exit code is now `$script:code`, assigned at every refusal point, and the
call is `Invoke-CrossCheck | Out-Host` so the log still reaches the console
without being bound to anything. `assertPowerShellExitIsNotTheFunctionsOutput`
pins all three halves for both the archive and the native body. The old native
body had the same shape and the same latent bug; it is fixed too.

### Task 6.5 — what came out, and the one thing that went in

Deleted from all three test tiers in `_package-ci.yml`: `rustup show`,
`Swatinem/rust-cache@v2` (the `package-ci-<package>-test-<environment>` key
entirely), the `Build messenger desktop stubs` prebuild, the `Build the
darkmatter md fixture` prebuild, and twelve now-always-true
`if: ${{ steps.build.outputs.archive }}` guards. Deleted from the planner:
`LEGACY_RUNNER_TOOL_SIDECARS` and its projection back into `runner_tools`.
Deleted from the environment table: `archive_cutover`, which the closed
contract vocabulary now refuses.

The one addition: `just _ci_build_consumer` **refuses** a cell the plan names no
build for. It used to answer `archive=` and let the tier compile in place. There
is no such cell any more — every executing L1/L2/browser cell references exactly
one record — so an empty answer is a disagreement between the plan and the
workflow, and on a toolchain-free consumer the fallback would not even fail
honestly. `a_cell_with_no_build_record_refuses_instead_of_compiling_in_place`
drives the shipped recipe rather than matching its text, because a recipe that
printed the message and carried on would pass a string match.

`no_test_tier_carries_a_compile_in_place_path` is the standing guard: no tier
may contain either prebuild, the `!steps.build.outputs.archive` condition,
`rust-cache`, or `rustup show`, and the only `cargo ` line a tier may execute is
the opt-in compiler-work counter's own build — because a consumer's count must
be **zero**, and the tool that says so has to exist.

### Task 6.4 — the inventory, and what it closed

| Class | Real example exercised | Outcome |
|---|---|---|
| Example target | `biscuit-terminal`'s `discovery_probe` | **was missing on every producer**; now a declared include the producer builds |
| Dynamic library | fixture `dylib` member, real `{DLL_PREFIX}…{DLL_SUFFIX}` includes | already carried |
| Non-test binary | fixture `archive-portability-tool` | already carried by nextest |
| Build-script output | fixture `build.rs` `OUT_DIR` | already carried, discovered from `rust-build-meta` |
| Repository fixture | fixture `crate/fixtures/data.txt` | reached through `--workspace-remap` |
| messenger stubs | `messenger-desktop-stubs` sidecar | declared; legacy prebuild removed |
| darkmatter CLI fixture | `darkmatter-md-fixture` sidecar | declared; legacy prebuild removed |
| Harness broker / backend proof | `harness-broker`, `backend-proof` sidecars | declared by the eleven CI-hostable-L2 packages, now **enforced** |
| Companion suite | `homelab-frontend` | runtime, unchanged |

Two audits enforce the closure rather than a list someone maintains:

- `missing_tier_sidecars` answers which sidecars a package's CI-hostable L2 tier
  needs and does not declare, and `ArchiveInventoryClosureTests` asks it of the
  whole workspace. It is a **query, not a raise**: a synthetic L2 fixture models
  scheduling, has no recipe to run, and must not be made to carry a declaration
  it has no use for.
- `every_shipped_sidecar_is_declared_by_a_package_and_names_real_binaries`
  closes the other direction — no orphan entries, and every binary a sidecar
  names is a real Cargo target. Reading the manifests as TOML rather than
  grepping them matters: `messenger/cli` declares a *bin* named `messenger`, and
  a line-wise `name = "messenger"` match finds the wrong file and reports the
  right package's binaries missing.

`an_environment_missing_a_linked_native_library_refuses_the_archive` closes
checkpoint 6's "missing native library" fault at the boundary that actually
runs: the planner checks the same containment when it validates a compatibility
*edge*, against the table as it was when the key was resolved.

### What Task 6.4 deliberately did NOT do

Phase 3's notes said Task 6.4 "moves those sites to
`biscuit_test_harness::manifest_dir!()` and frees the guest to clone anywhere".
**It did not.** The sweep is ~120 files across 15 package areas (45 in
`darkmatter`, 44 in `claudine`), each needing `biscuit-test-harness` as a
dev-dependency, and its validation surface is every one of those packages' L1
suites — far outside this plan's declared `packages` blast radius, and not
something this session could show green. The guest therefore still clones to the
manifest's `producer_workspace`, which remains data-driven and self-correcting.
It is named here as remaining work rather than left implied.

### Validation checkpoint 6 — what was run

| Check | Result |
|---|---|
| `python3 -m py_compile scripts/ci/*.py` | clean |
| `python3 -m unittest discover -s scripts/ci` | **588 passed** (was 576) |
| `cargo nextest run --bin ci-build` | **109 passed** (was 101) |
| `cargo nextest run --no-default-features --bin ci-rollup` | 188 passed |
| `cargo nextest run --bin ci-plan` | 13 passed |
| `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | **112 passed** (was 111) |
| `bash .githooks/tests/test-pre-push.sh` | 66 passed, 0 failed |
| `actionlint .github/workflows/*.yml` | clean |
| clippy, `ci-build` + `test-toolkit`, all targets, `-D warnings` | clean |
| `just ci-local --plan` | resolved, no prohibited cell |
| real `biscuit-terminal` macOS archive through the shipped planner's plan | probe carried, digest recorded |
| `just cross-check --os windows biscuit-hash` on `build-win-native` | produce → verify → archive-mode tier, 50/50 passed |
| `just cross-check --os linux biscuit-hash` on `build-linux` | queued 30 min on an unrelated lock, exit 75 (remote half unexecuted) |

Injected faults already covered and still refusing: tamper, wrong environment,
wrong architecture, missing/short/long inventory, missing and altered sidecar,
foreign compiler host, unbuildable target, unreadable `--archive-file`, missing
nextest driver — plus this phase's two new ones, a missing linked native library
and an example include naming no target.

**No test opened or focused a terminal or browser window.** The fixture's
`level2_` and `browser_` binaries drive no backend, every archive-mode run
asserts an exact test count, and no tmux server, pane, or fixture process
survived the session.

### Two environment facts, recorded rather than worked around

- **`build-linux`'s shared `ci-verification` clone is locked**, by the same
  unrelated `nightly-reward-spike` run the Phase 5 notes named — held since
  2026-09-14T18:25Z. A real `just cross-check --os linux biscuit-hash` was
  dispatched to validate the rewritten Unix path end to end. Its **local half
  ran against the real host**: the plan resolved, the record was found and
  announced (`linux  d7cb6cbab4cea238 produced on ubuntu-latest, consumed as
  ubuntu-latest`), and the script, the 21 860-line patch, and the plan were
  shipped. Then it queued its whole 30 minutes behind that lock and gave up with
  exit 75, which is the designed behavior. The **remote** half of the Unix path
  is therefore unexecuted; `test_cross_check.py` stands in for it, proving the
  shipped remote script rather than its execution. The path's new prerequisites
  were confirmed present on that host (`just`, `cargo-nextest`, `cargo` all on
  the login PATH).
- **`build-win` (the WSL2 guest) was unreachable** for the whole session —
  `ssh` to `192.168.100.64:22` times out — so the WSL2 leg could not be
  exercised on a real guest. Native Windows, on the same physical machine,
  was reachable and ran the whole path (above), so what is unproven is the
  guest, not the Unix script it shares with Linux and macOS.

### A pre-existing scoping gap, observed and NOT touched

`gate_triggers`/`global_trigger` — the per-gate global-input machinery that is
supposed to widen scope when `.config/nextest.toml`, `ci.yml`,
`_package-ci.yml`, or `affected_scope.py` itself changes — is **never called by
`calculate_scope`**. `full_gates` is `set(GATES) if force_all else set()`, and
`force_all` is only `--all`. Both functions are unit-tested in isolation and
have no production caller.

Verified identical on `origin/main` and on this branch's base, so it predates
this plan. It is left alone deliberately: this plan's execution constraints say
build derivation may not reopen affected scope, and wiring it would change the
breadth of every CI run — a scope-policy decision with its own cost argument.
It is recorded here so the next reader of `just ci-local --plan` is not
surprised to see a 119-file diff select three packages.

## Phase 7 — Document, measure, and complete rollout

### Task 7.1 — where each of the seven stages is actually measured

The gap Phase 7 found was not "the numbers are wrong" but "there are no
numbers": a consumer's `status.json` carried no timings at all, and a build
status carried only `ci-build produce`'s internal milliseconds. Three of the
seven windows are invisible from inside any tool, so the workflow observes them
between steps.

| Stage | Observed by | Field |
|---|---|---|
| queue | owner job (`scope.outputs.plan_epoch` → `steps.job_start`) | `stage_seconds.queue_seconds` |
| compile + archive | `ci-build produce` | `timings.compile_archive_ms` |
| upload | owner job (`steps.staged` → status step) | `stage_seconds.upload_seconds` |
| download | `transfer` marker → `_ci_build_verify` | `timings.download_seconds` |
| verify | `_ci_build_verify`'s own window | `timings.verify_seconds` |
| extract | `ci-build verify`'s `--extract-to` | `timings.extract_ms` |
| execute | the gate step, already timed since Phase 1 | `timings.execute_seconds` |

**Queue is measured against the plan, not the run.** GitHub's `github` context
carries no run-start time, and the existing design (the comment on
`_ci_build_report`) deliberately refuses to parse an ISO timestamp in shell
because `date -d` is GNU and `date -jf` is BSD. `scope` therefore stamps
`plan_epoch` the moment the plan artifact is published — which is the moment an
owner became *schedulable* — and the owner subtracts its own start. That is the
wait this design can change; the run's total queue is an API join, unchanged.

#### Extraction: the one stage that is measured somewhere else on purpose

`cargo nextest run --archive-file` extracts inside the run. Separating that
extraction from test time means running from an already-extracted tree with
`--binaries-metadata`/`--cargo-metadata`/`--target-dir-remap` instead of
`--archive-file`. That was verified to work (a two-phase `nextest list
--extract-to` then `nextest run --binaries-metadata …` ran the probe crate's
tests from the extracted directory), and it was **not** adopted: it changes the
execution path of `_test`, `_test_l2`, `_test_browser`, the WSL leg,
`cross-check`, and `ci-local` — the path Phases 3–6 proved on four OSes — for
one number, and it would leave hosted CI and `cross-check` running the archive
two different ways.

What is reported instead is the verifier's own extraction. `ci-build verify`
*already* extracts the whole archive to list its inventory; that scratch
extraction was previously thrown away untimed. It is the same archive, the same
host, and the same operation the run repeats, so the stage now has a measured
cost rather than an invisible one. `--verdict-out` writes the document the
consumer's status reads, so nothing is parsed back out of a log.

One consequence worth recording for whoever picks up the measurement: **the
archive is extracted twice per consumer today**, once by the verifier and once
by the run. Making `verify` extract to a persistent directory the run then uses
would remove one of them *and* separate the stage properly. It is a real
optimization, it needs the recipe change above, and it belongs to its own
change with its own evidence.

#### Absent is not zero

Both status scripts refuse to fabricate. A tier whose `TIMINGS` is unset — or
malformed — writes no `timings` object at all, and `ci-rollup` renders `—`
rather than `0s`; a stage that genuinely finished inside one clock tick renders
`0s`. This is load-bearing for the WSL2 cell, whose guest cannot write
`$GITHUB_OUTPUT` and leaves `wsl-timing/*.seconds` plus a copy of the verdict in
the 9p workspace: a guest that died leaves none of them, and that must read as
"unmeasured", not "instantaneous".

The hardening is not decorative. `the_status_fold_preserves_every_failure_shape`
executes each shipped status script with a simulated environment, and it caught
the first version immediately: the harness resolves any unrecognized `${{ … }}`
env value to `"success"`, `jq --argjson timings success` is invalid JSON, and
the step exited non-zero — under `always()`, losing the cell's only evidence. A
status script must survive its own instrumentation being wrong.

#### `ci-rollup` stays GFM, and that is the standing answer

Task 7.1 asks for `TerminalRenderable` "rather than ad hoc ANSI output".
`ci-rollup` emits no ANSI at all and never has: it writes GitHub-flavored
Markdown to `$GITHUB_STEP_SUMMARY`, and it is the one binary that must link none
of the monorepo's crates (it is the always-runs merge-gate tool, built
`--no-default-features`). The reasoning is already on `render_grid`; Phase 7
added the two new sections in the same plain GFM. `ci-plan` and `ci-build`
render through `TerminalRenderable` and were already doing so.

### Task 7.2 — three pieces of drift the documentation pass found

Two were dead code and stale prose left by Phase 6's cache removal, and one was
a promise no phase kept:

- **`_package-ci.yml`'s "Prepare nested test caches for rust-cache cleanup"**
  survived in the L1 job after the job's `rust-cache` step was deleted. Its
  entire purpose is to create two directories the action's *post-job* cleanup
  opens; with no action in the job there is no post step, so it did nothing.
  Removed, with the `.github/ci/README.md` paragraph that described it rewritten
  to say the workaround went with the cache and comes back only if a test tier
  ever restores one again.
- **`docs/kache-strategy.md`** still claimed `Swatinem/rust-cache@v2` "remains
  on every native leg". It remains on the legs that still compile — the owner,
  `check`, and `lint`. Same correction in `docs/topics/ci-cd.md`, which also
  still listed the retired `package-ci-<package>-test-<environment>` key.
- **"Task 6.4 moves those sites to runtime lookup"** appears in
  `.github/ci/README.md` and the `rust-devops` skill as though it were
  scheduled. Task 6.4's own notes say it did not, and why. Both now say the
  sweep spans 15 package areas and remains undone, so nobody plans against it.

`.claude/skills/rust-testing/SKILL.md` gained the section that was missing
entirely: a test author's view of archive-mode execution — no compile-time
paths, declare what the archive would not carry, provision runtime facilities
only, and reproduce with `cross-check`. `nextest.md`'s archive section said this
was the WSL2 leg's contract; it is now every hosted cell's.

### Tasks 7.4 and 7.5 — not done, and not workable from here

Both need hosted runs this session may not produce: three consecutive green runs
per environment, cold and warm, which means pushing the branch and dispatching
`ci.yml` with `measure-compiler-work: true` six times per environment. Task 1.6
left the *pre*-cutover half empty for the same reason, so there is also nothing
to compare against.

Task 7.5 is therefore recorded as **not applied**, not as "single-owner
accepted". Writing down an acceptance would be a ruling with no measurement
behind it. Compatibility cohorts stay out, which is the specification's own
default — adopt them only if the measurements reject single-owner.

What *is* measured and does support the design: the compiler-work counter
proves the shared dependency compiles once and the divergent one twice
(`one_owner_tree_shares_a_dependency_compile_without_unifying_features`), and
every consumer's compiler-work count is structurally zero. That is the
compile-once claim. It is not the critical-path claim, and the ledger does not
let the two be confused.

### Validation checkpoint 7

`detect-changes --scope all`: 359 changed symbols, 64 files, 5 affected
processes, `risk_level: medium`, neither `partial` nor `truncated` set. No
affected process is HIGH or CRITICAL, so no re-analysis was owed. The symbols
Phase 7 edited (`run_verify`, `render_grid`, `build_reports`, `VerifyOptions`)
have no caller outside `scripts/`; the two hits elsewhere are a `sniff` test
name and a comment in `ci_workflow_contracts.rs`.

Diff review found no area-keyed store (`runner_loss.py`'s three "area-owned"
mentions are the narrowing *argument*, not a key), no baseline or `ci-gate`
policy change (the gate gained one `needs.build.result` entry and no logic), and
unchanged reusable-workflow depth: `ci.yml → _area-ci → _package-ci → _wsl-ci`.
Duplicate owners, unconsumed archives, and consumer compilers are each held by a
test that runs in this suite rather than by this review.

### A host quirk, recorded so the next session does not chase it

`.githooks/tests/test-pre-push.sh` resolves its root with `cd … && pwd`, and
`cd` prints the target directory when the invoking shell has `CDPATH` set. This
session's did, so `REPO_ROOT` came back as two lines and the harness looked for
the hook in an unrelated worktree (`feat-unifi`) — a confusing failure that says
nothing about the hook. `CDPATH= bash .githooks/tests/test-pre-push.sh` is the
correct invocation and produced 66/0. CI sets no `CDPATH`, so the harness was
left alone rather than changed from inside this plan.

### The Phase 7 recipe change, proven on the OS most likely to break it

`_ci_build_verify` grew a `--verdict-out` write and two `jq` reads, and Git Bash
on Windows is where a path or quoting mistake in a `just` recipe actually
surfaces. `BISCUIT_BUILD_MIN_FREE_GIB=0 just cross-check biscuit-hash --os
windows` on `build-win-native` ran the whole producer → verify → archive-mode
tier path afterwards: **50/50 passed, `cross-check-exit: 0`, 19 s**. The
`BISCUIT_BUILD_MIN_FREE_GIB=0` is the same Windows-only storage floor Phase 6
recorded, not a new requirement.

## Review 1 corrections (2026-09-15)

The Phase 4 artifact-upload rationale changed the specified ownership model and
was incorrect as an architecture constraint. A programmatic artifact client can
publish multiple named artifacts from one job. The workflow now selects native
producer environments, invokes `produce-owner.sh` once, and publishes individual
package bundles and statuses through `scripts/ci/artifacts/publish.cjs`. The
Phase 7 statement that no per-package owner was introduced was also incorrect.

Source manifests now bind clean tracked source to the plan's actual commit and
Git tree; producer checks run before and after compilation, and consumers check
the same identity. The old `--source-tree` bypass is an assertion only. Archive
fixtures initialize real temporary Git repositories. Dirty local archives fail;
explicit native diagnostic runs remain available without reusable evidence.

Manifest generation 3 observes native dependency identities from payloads and
sidecars. Linux resolves libraries with `ldd -v`; macOS inspects Mach-O load
commands/UUIDs with `dyld_info` and pins the OS build for sealed system libraries;
Windows reads PE imports and pins the API-set schema. External library content
identities must match exactly on the consumer. This conservative rule rejects
some compatible upgrades; a broader ABI-version rule needs its own evidence.
The linker is discovered, probed, and explicitly selected for compilation.

The Bash corrections cover associative arrays, empty arrays under `set -u`, and
argument quoting for both Unix and PowerShell. Python fixtures use 3.9-compatible
cleanup. A Linux run exposed Cargo's target-information query being counted as
compiler work; it is now excluded alongside version probes, while real
build-script feature-probe compilations still count.

The shared fixture now exercises the shipped workflow script and its measurement
switch. Per-package compiler reports carry owner aggregates and measured record
counts. No hosted baseline or post-cutover observations have been collected;
Tasks 1.6, 7.4, and 7.5 remain open. The original review is retained unchanged.
