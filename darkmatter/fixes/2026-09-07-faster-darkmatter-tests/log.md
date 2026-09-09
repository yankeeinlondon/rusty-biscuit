---
fix: 2026-09-07-faster-darkmatter-tests
implementation_1: "2026-09-09T10:50:12-07:00"
deferred_perf_measurement: true
---

# Validation ledger — faster-darkmatter-tests

One entry per run or gate: command, selection, source state, result, and the
three costs kept apart. `inventory.md` owns dispositions and budgets;
`results.md` links here. Times are UTC.

## Phase 1 — shared tooling, baseline, evidence discipline (2026-09-08)

### 1A — shared TypeScript analysis tool

**Delivered:** `tools/test-audit` (`@rusty-biscuit/test-audit@0.1.0`), a root
pnpm-workspace member (`pnpm-workspace.yaml`, root `pnpm-lock.yaml`), Node 22,
TypeScript strict, vitest. Commands: `config`, `capture`, `fetch`, `junit`,
`sources`, `reconcile`, `attribute`, `measure` (`report|parse|run`),
`counters` (`validate|compare`). Parsers: `fast-xml-parser` for JUnit,
`web-tree-sitter` + `tree-sitter-rust` 0.24 for the source scan, `zod` for
configuration. Own checks: `just check` there → `tsc --noEmit` clean, vitest
**160 passed / 13 files** (includes replays of the preserved Claudine
inputs). CI route: `ci.yml` `ci-tooling` leg runs `pnpm --dir tools/test-audit
check`; `scripts/ci/affected_scope.py` `CI_TOOLING_PREFIXES` gained
`tools/test-audit/`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`
(`python3 scripts/ci/test_affected_scope.py`: 67 OK). Both edits touch
GLOBAL_PATHS, so the first CI run of this branch is full-scope.
Documentation: `.claude/skills/rust-testing/test-audit-tooling.md` (new),
`SKILL.md` + `test-suite-audits.md` entries (hash updated), `sniff/SKILL.md`
note, `docs/dependencies.md` note, `tools/test-audit/README.md`.

**Requirement → command/configuration mapping**

| Requirement | Darkmatter | Sniff |
|---|---|---|
| Distinct local vs CI L1 populations | cohorts `l1-local` (`just test`, excludes `slow_`) and `l1-ci` (`BISCUIT_L1_INCLUDE_SLOW=1`, CI features) in `audit.config.json` | cohorts `l1-local` (`sniff --features remote; sniff-cli`) and `l1-ci` (`sniff-cli --features test-fixtures`) |
| Feature/route representation | 11 selections, 15 routes incl. `check-zed`, `zed-verify` (`_package-ci.yml:519`), `vscode-dmls` (absent), `effects-instrumentation` (manual) | 6 selections incl. `lib-bare`/`lib-network` feature-gap probes; routes incl. `work-counts` (sniff-performance.yml), `test-real` |
| Sanity cohort timed separately | cohort `sanity` (tiers `["sanity"]`) | cohort `sanity` with the 15 s budget noted |
| CI legs, not rectangular | 4 legs; L2 cells only on ubuntu/macos for cli+dmls, browser only on ubuntu, wsl2 L1-only | 4 legs; L2 cell only for sniff-cli on ubuntu/macos |
| Work-count evidence as data | `counters.signals`: `effects` (instrumented, `effects-instrumentation`), `http-requests` (fixture), `discovery`/`composition` (pending) | 8 instrumented signals from `counters.rs`; keys incl. `requestShape`, `counterVersion`, `phase`, `platformKind`; boundary `production-caching-2026-07-22` |
| Listing captures | `capture` → `enumeration/` (11) | `capture` → `sniff/.../enumeration/` (6) |
| CI artifact reuse | `fetch --run 34008778001` → `baseline/34008778001/` (21 cells) | same run → `sniff/.../baseline/34008778001/` (10 cells) |
| Report validation | `junit` gate: 21/21 cells clean | `junit` gate: 10/10 cells clean |
| Source population | `sources`: 7,862 attributes, 598 files | `sources`: 2,662 attributes, 216 files |
| Family reconciliation / attribution | `reconcile`/`attribute` exercised on the Claudine replay (7,400 identities; 6,861 results); run for Darkmatter in Phases 2–3 once `families.json` exists | same; Sniff Phase 2 |

No unresolved tooling gap for either area. Claudine compatibility:
`junit-metrics.ts`, `inventory-reconciler.ts`, `attribution.ts`,
`measurement.ts`, `measurement-runner.ts` are wrappers forwarding to the
shared commands with `claudine/.../audit.config.json`; the old tests moved to
`tools/test-audit/tests/`. Old vs new on preserved inputs: JUnit gate output,
attribution (6,861 / 561.00 s / 36.05 s), measurement report and gate exit,
runner universe (7,400) and source column (4,167/21/2,761/52/158/24/88/184
at `/tmp/rb-baseline-9fc5151a0`) all reproduced. Intentional corrections:
retries are now disclosed per suite (nextest `flakyFailure`/`rerunFailure`
were unread before); the source scan resolves `proptest!` bodies and
`macro_rules!` templates by grammar rather than brace counting; grammar gaps
(`&raw`/`raw` identifiers, `unsafe extern`) are reported as `parse-error`
diagnostics instead of silently absorbed — 142 such diagnostics in 44
Claudine files with zero lost tests. Not verified on other hosts: the tool's
own suite ran on macOS only; Linux/Windows behavior is pending the
`ci-tooling` leg (Linux) — Windows evidence is **pending** (no Windows Node leg
exists).

### 1B — baseline

**Identity.** Revision `973d1d9184601ab3409654955ffacc249f4870a5`
(`fix/cli-slow-tests`); merge-base with `main` `a9e88c069`; `main` is 67
commits ahead. Dirty tree at the start: 15 modified + 5 untracked entries, all
under `claudine/` and `.claude/skills/{claudine,rust-testing}` (the sibling
faster-claudine-tests fix, assumption 4) — **sibling changes present, disjoint
packages; no darkmatter path was dirty.** Toolchain: rustc 1.97.1
(8bab26f4f 2026-07-14), cargo 1.97.1, cargo-nextest 0.9.136. Profile
`default` (local). Host: Darwin 27.0.0 arm64, Apple M4 Max, 16 cores, 128 GiB.
Host load before the first timed run: 10.7 (1-min), rising to 50–70 during
the runs from macOS `identityservicesd`/`syspolicyd` (code-signing scans of
the freshly linked test binaries) — the diagnostic timings below are
**contended** and serve attribution only, not budgets.

**wasm32-wasip2:** already installed (`rustup target list --installed`), so
`just lint` → `check-zed` is not a false red.

**nextest.toml invariant.** `git diff a9e88c069 -- .config/nextest.toml`
(branch-side) is **empty**, and the working-tree diff is empty. `git diff
main -- .config/nextest.toml` is 97 lines, all from `main` moving ahead;
none of them names darkmatter, dmls, or zed. Starting invariant: darkmatter
owns zero nextest overrides; the branch must add none.

**Baseline source state.** Worktree `/tmp/rb-dm-baseline-973d1d9` (detached
at 973d1d9). Darkmatter sources there are byte-identical to this checkout, so
the warm artifacts in this checkout's `target/` served the diagnostic runs;
the worktree gets its own build directory in Phase 9 for alternation.

**Listing captures** (`test-audit capture`, 11 selections, revision 973d1d9,
26 dirty entries recorded in `captures.json`):

| label | packages | features | identities |
|---|---|---|---:|
| `cli-bare` | darkmatter-cli | — | 661 |
| `cli-terminal` | darkmatter-cli | `--features terminal-tests` | 734 |
| `dmls-bare` | dmls | — | 643 |
| `dmls-terminal` | dmls | `--features terminal-tests` | 646 |
| `l1-local-all` | darkmatter, darkmatter-cli, dmls, zed-dmls-cli | — | 7711 |
| `lib-bare` | darkmatter | — | 6380 |
| `lib-browser` | darkmatter | `--features browser-tests` | 6426 |
| `lib-effects` | darkmatter | `--features effects-instrumentation` | 6380 |
| `lib-terminal` | darkmatter | `--features terminal-tests` | 6400 |
| `lib-terminal-browser` | darkmatter | `--features terminal-tests,browser-tests` | 6446 |
| `zed-cli-bare` | zed-dmls-cli | — | 27 |

`l1-local-all` (7,711) = the local `just test` invocation: 7,661 run + 50
skipped by the L1 filter. Feature unification differs per selection
(`lib-bare` 6,380 vs `lib-terminal-browser` 6,446).

**Local diagnostic runs** (`baseline/local/run.sh`; console logs and
`runs.jsonl` beside it; parsed with `test-audit measure parse`):

| run | command | wall | runner elapsed | summed duration | tests | skipped | slow marks | exit |
|---|---|---:|---:|---:|---:|---:|---:|---|
| sanity | `just sanity` | 53.5 s | lib 44.33 s · cli 0.28 s · dmls 1.95 s · zed 0.05 s | 707.7 · 4.2 · 25.7 · 0.6 s | 5,630 · 137 · 517 · 21 | 48 · 0 · 0 · 0 | 8 · 0 · 0 · 0 | 0 |
| test-l1-local | `just test` | 70.3 s | 68.07 s | 1,081.1 s | 7,661 | 50 | 7 | 0 |
| test-l1-include-slow | `BISCUIT_L1_INCLUDE_SLOW=1 just test` | 59.0 s | 57.50 s | 907.1 s | 7,662 | 49 | 2 | 0 |
| doctest | `just doctest` | 5.6 s | lib doctests 3.24 s (merged compile 1.64 s); cli 0 doctests | — | — | — | — | 0 |
| test-l2 | `just test-l2` | 147.7 s | lib 16.23 s · cli 98.36 s · dmls 2.52 s | 16.2 · 98.4 · 2.5 s | 18 · 69 · 3 | 6,382 · 665 · 643 | 0 | 0 |
| test-browser | `just test-browser` | 32.6 s | 31.26 s | 31.3 s | 86 | 6,340 | 0 | 0 |
| test-l3-optout | `just test-l3` | 0.1 s | — | — | — | — | — | **1: pending** |

Build/setup for the timed runs is wall − runner elapsed (≈2 s each; artifacts
were warm from the captures). Findings: `just sanity` runs 53 s against the
15 s fast-confidence budget — the lib's 5,630-test `--lib` binary alone is
44 s; the slowest L1 identities are `markdown::compose::tests::rendering::
test_compose_cleanup_preserves_*` (5.5–9.0 s each), the preflight acceptance
tests (4.5–6.9 s), and `darkmatter-cli::compose_remote_caching` (6.4 s each).
L2 ran with both WezTerm (this session runs inside WezTerm, socket present)
and tmux 3.7b; browser ran with Google Chrome. **L3 is pending — harness
unavailable unattended:** `_test_l3` refuses to run without a human or
`BISCUIT_L3_TAKE_FOCUS=1` (it raises a GUI terminal and injects keystrokes);
this session is unattended, so it was not forced. A clean refusal is not
evidence of passing.

**CI-selected cohort delta.** Local (7,661 run / 50 skipped) vs
`BISCUIT_L1_INCLUDE_SLOW=1` (7,662 / 49): exactly one identity,
`darkmatter markdown::compose::tests::rendering::slow_compose_cleanup_preserves_quoted_marker_looking_indented_code`
— confirmed, not assumed. (The area has 11 `#[ignore]` sites and this single
`slow_` test; Phase 2 enumerates the ignores.)

**CI legs declared for the four packages** (read from
`.github/ci/environments.json` + `scripts/ci/affected_scope.py`
`matrix_record` + the four `[package.metadata.ci.tests]` blocks): natives
`ubuntu-latest`, `macos-latest`, `windows-latest` (L1 for all four packages);
L2 on the tmux-hostable natives (ubuntu, macos) for `darkmatter-cli` and
`dmls` only — the lib's L2 backend is wezterm, which no runner hosts (POLICY
GAP, covered locally); browser on `ubuntu-latest` for the lib; `wsl2-ubuntu`
(hosted by windows-latest, `_wsl-ci.yml`) L1 for all four. That is four legs
and 21 cells, encoded in `audit.config.json`.

**Compatible CI baseline.** Run **34008778001** (`main` @ `03ce3f8c1`,
PR #68, 2026-09-06, conclusion success, full-scope). `03ce3f8c1` is an
ancestor of HEAD; `git diff 03ce3f8c1 973d1d9 -- darkmatter/` is only this
fix's `plan.md`/`spec.md`; shared inputs differ only in `.config/nextest.toml`
(16+/41−, none naming darkmatter). Fetched with `test-audit fetch` into
`baseline/34008778001/` (`fetch.json` records the run and artifact set);
gated with `test-audit junit` (`junit-gate.md`): 21/21 cells present, 0
failures, 0 skips, 0 retries. Headline per-leg numbers (runner elapsed /
summed / tests):

| leg | darkmatter L1 | darkmatter-cli L1 | dmls L1 | zed-dmls-cli L1 | other cells |
|---|---|---|---|---|---|
| ubuntu-latest | 167.4 / 668.2 s / 6,332 | 47.1 / 184.9 / 665 | 7.8 / 31.1 / 643 | 0.1 / 0.4 / 27 | cli L2 7.1 s/69 · dmls L2 1.7 s/3 · browser 40.6 s/86 |
| macos-latest | 174.6 / 523.0 / 6,333 | 47.7 / 143.1 / 665 | 10.6 / 31.7 / 643 | 0.6 / 1.7 / 27 | cli L2 13.2 s/69 · dmls L2 4.1 s/3 |
| windows-latest | 238.0 / 950.1 / 6,319 | 80.6 / 315.4 / 667 | 10.8 / 42.9 / 645 | 2.2 / 7.1 / 26 | — |
| wsl2-ubuntu | 229.1 / 895.4 / 6,332 | 66.8 / 253.5 / 665 | 8.1 / 32.1 / 643 | 0.1 / 0.5 / 27 | — |

The later full-scope run 34232285291 (`main` @ `6504747e2`, PR #70) is
**not** compatible: 38 darkmatter files differ from this baseline. One
baseline sample per leg supports attribution; budgets stay **pending** until
Phase 9 decides whether more samples are needed (`gh run rerun 34008778001`
is the only way to resample that source state).

**Work counters available for later proof:**

| signal | coverage | source |
|---|---|---|
| effects | instrumented | `darkmatter` feature `effects-instrumentation`: `effects::instrumentation::{engine_build_count, network_attempt_count}` (process-wide atomics) |
| http-requests | fixture | `cli/tests/common/mod.rs::MockHttpServer::request_count()`; `lib/tests/effects_integration.rs` request counter |
| discovery | pending | no stable counter; Phase 7/8 candidate: lldb entry-hit counts on `capture_file_resolution_context` / `GitRepo::discover` |
| composition | pending | no stable counter; Phase 7/8 candidate |

Gaps are Phase 7/8 work, not a reason to fall back on timing.

**Shared-tool validation of the captured reports:** `junit` over
`baseline/34008778001` — exit 0 (above); `measure parse` over the five local
console logs — all parsed, `non_passing_lines=0`; `sources` over the four
packages — 7,862 attributes / 598 files, 66 cfg-gated, 5 ignored, 9
`parse-error` diagnostics (`raw` identifiers; no test lost); `config
validate` — OK. Tool version for every report: `@rusty-biscuit/test-audit@0.1.0`;
its own suite result is recorded under 1A. No metrics or reconciler
implementation exists in this directory.

**Gates run for this phase (Sniff is the session's package area):**
`cd sniff && just test` → 2,599 passed / 23 skipped (42.06 s runner elapsed),
exit 0 (`sniff/fixes/2026-09-07-faster-sniff-tests/phase1a-gate-just-test.log`);
`cd sniff && just lint` → exit 0. No Rust source changed in this phase, so no
darkmatter Rust gate was rerun beyond the baseline runs above (which all
passed).

**Checkpoint 1 status:** 1A satisfied (both consumers configured and
exercised; Claudine compatibility proved; reproducible typecheck/tests; CI
route; Sniff handoff written into its plan; skill updated). Baseline source
and build state recorded; three costs separated; shared validation passes;
CI baseline linked for all four legs; **pending:** L3 (unattended host),
Windows/Linux runs of the tool's own suite, CI budget ratification (Phase 9).
Darkmatter fixture implementation may open.

## Phase 2 — Complete inventory (2026-09-08)

**Delivered:** [`inventory.md`](inventory.md) (63 families, 25 declared
exclusions, override census, recipe reconciliation, ignored/slow enumeration,
non-nextest entry points, shared-fixture rows, per-family dispositions) and
[`families.json`](families.json). Document-only: no Rust source or test in
the four packages changed. Source state: revision `973d1d9` unchanged; the
Phase 1 dirty entries plus this phase's new files. Analysis tool unchanged
(`@rusty-biscuit/test-audit@0.1.0`), so its own suite was not rerun.

**Commands and results** (from `tools/test-audit`, `CFG` = this directory's
`audit.config.json`):

| Command | Result |
|---|---|
| `reconcile --config $CFG` | **GATE EXIT=0** — 7,853 runner identities, each in exactly one of 63 families; 25 exclusions all live; inventory counts match; every row dispositioned. 54 `parse-error` diagnostics (all `raw`/`&raw` + `dmls/src/bench.rs`), no test lost. |
| `sources --config $CFG --json` / `--markdown` | regenerated into `sources.json` / `sources.md` at the fix root (moved out of `enumeration/`, where an undeclared JSON fails the gate): 7,874 attributes / 598 files (Phase 1: 7,862 — the tool now resolves 12 `proptest!` bodies). |
| `attribute baseline/local/test-l1-local.log --config $CFG` | exit 0; 7,661 results, summed 1,081.06 s, runner elapsed 68.07 s — per-family table folded into `inventory.md`. |
| `attribute …/sanity.log` · `…/test-l1-include-slow.log` · `…/test-l2.log` · `…/test-browser.log` | exit 0 each; 6,305 / 7,662 / 90 / 86 results attributed. |
| `cd sniff && just lint` | exit 0 (session package area; no Rust changed). |
| `cd sniff && just test` | exit 0 — 2,599 passed / 23 skipped, 44.52 s runner elapsed. |

**Findings that supersede the plan's grounding facts** (details in
`inventory.md` § Population and § Runner override census):

- `#[ignore]` sites are **5**, not 11; three `schema_plus_phase1` doc comments
  are stale (say ignore-gated, none is). Comment-only cleanup owed.
- **Darkmatter owns nextest overrides**: four targets / seven blocks
  (`execution_subset_of_approval_across_randomized_conditions`,
  `every_catalog_variable_survives_ambient_options`,
  `slow_compose_cleanup_preserves_quoted_marker_looking_indented_code`,
  `level2_render_tree_style_in_wezterm` — the last binds **nothing**). Present
  at merge-base and on `main`; assumption 6's freeze stands, re-scoped to
  "no change", not "none exist".
- **Six identities have no local route**: 4 `level2_harness_integrity`, 1
  `pixel_classification_…`, 1 `sanitized_real_mermaid_…` sit in feature-gated
  binaries without a tier prefix and run only on CI L1 (feature-unified
  builds). The last also skips-as-pass without a Mermaid toolchain.
- **44 in-process tests are `browser_`-prefixed** and therefore run only in the
  browser tier (locally with Chrome; `ubuntu-latest` only on CI). None touches
  a browser.
- The source scan misses four `schemas::coerce` tests because a `//` comment
  sits between `#[test]` and `#[allow]`; runner-only, coverage unaffected;
  noted in `test-audit-tooling.md`.

**Checkpoint 2 status:** reconciler exits 0; all four packages, every tier,
doctests, all 16 benches, the fuzz target, gated and ignored tests, the WASM
extension route (`check-zed` / `zed-verify` at `_package-ci.yml:519`), the
`vscode-dmls` absence row and every shared helper are covered; every row
carries a disposition and an execution route. **Pending:** L3 evidence
(unattended host), budgets (Phase 3), CI samples (Phase 10).

## Phase 3 — Attribution and ratified budgets (2026-09-08)

Analysis only: no Rust source, test, or tool code changed in this phase.
Identity: revision `973d1d9` unchanged; dirty state = the Phase 1 entries
plus this fix's documents (no darkmatter Rust path dirty). Tool:
`@rusty-biscuit/test-audit@0.1.0` (unchanged; own suite not rerun).

**Evidence produced** (all under `attribution/`; every command run from
`tools/test-audit` with `CFG` = this directory's `audit.config.json`):

| Evidence | Command | Result |
|---|---|---|
| Launch-cost probe | `attribution/launch-probe.sh` (hyperfine 1.20.0; 15 serial runs warmup 3 `--shell=none`, 5 burst runs; identity in `launch-probe-identity.txt`) | serial + burst tables written; load 10.0 → 22.0 during; key rows: launch floor 6.9 ms, compose at `darkmatter/cli` CWD 294.8 ms vs 66.6 ms outside, compose doc-in-checkout 758.1 ms, 16-wide compose burst 6.0× worse in-repo |
| Local by-binary attribution | `tsx src/cli.ts attribute ../../darkmatter/…/baseline/local/test-l1-local.log --config $CFG --by-binary --json` (likewise `test-l2.log`) | exit 0; 7,661 results, elapsed 68.07 s, summed 1,081.06 s, 0 violations; reproduced 2026-09-08 identical (families + binaries) |
| CI per-leg attribution | `tsx src/cli.ts attribute ../../darkmatter/…/baseline/34008778001/<leg>/<tier>/*.xml --config $CFG --json` per leg × tier | exit 0 each; ubuntu L1 884.6 s / macos 699.5 / windows 1,315.4 / wsl2 1,181.6 summed; L2 + browser cells attributed; every JSON re-derived 2026-09-08 and matched to float summation order |
| Budget gate | `tsx src/cli.ts attribute budgets --config $CFG --runs attribution/budgets-pending.json` | **exit 1 by design** — 4 × `insufficient-runs` (1 green run per leg; 3 consecutive required); no budget derived |

`budgets-pending.json` carries the per-leg family sums (L1 for all four legs;
L2/browser for the legs that have the cells) with provenance
`kind: ci, runs: [34008778001]`, headroom 0.25. The stored attribution JSONs
keep the tool's trailing `GATE EXIT=0` line on stdout; parse by stripping the
last line (the manifest generator does).

**Findings** (written into `inventory.md` § Attribution / § Budgets):

- Decision 1: launch floor ~7 ms/spawn (≈3.5 s CPU over 504 sites,
  parallelizes — not a driver); ambient discovery +228 ms per compose at
  monorepo CWD, +461 ms for docs inside the checkout, 6× contention under
  16-way concurrency; command work dominates (compose families 422.7–626.1 s
  of the 699.5–1,315.4 s CI L1 sums).
- Decision 2: host-tool paths enumerated and marked — `compose_shell` (shell),
  git-dependent context tests (fixture-owned topology), tmux/WezTerm/nvim/
  Chrome tiers (genuine tool tests; WezTerm = documented POLICY GAP), the
  ignored host-shell alias test, the Mermaid skip-as-pass row; the 509 `md`
  spawns need no host tool.
- Decision 3: every dmls/zed surface except `vscode-dmls` is reachable from
  an existing recipe; the WASM extension's `check-zed`/`zed-verify` ride the
  dmls **ubuntu lint job** (single leg), outside `just test` — corrected
  route+leg table in the inventory.
- Decision 4: repeated corpus/setup operations measured at ≈12.5 s of the
  1,081 s local L1 (1.2%); merging fails the cost test; justified
  consolidations are the `meta_schema_*` passive corpus (Phase 7) and the
  dmls `ClientFixture` copies (Phase 8).
- Attribution discipline: the redundant-walk results are cited as the
  standing reason no percentage target is set; no speedup extrapolated.
- Production findings: none new; the in-repo compose cost corroborates the
  already-documented capture walk (ambient_ctx_capture remediation landed;
  redundant-walk fix landed). Nothing filed; no production code touched.

**Gates run for this phase (session area; nothing in the Rust area changed):**
`cd sniff && just test` → exit 0 (2,599 passed / 23 skipped, 41.20 s runner
elapsed); `cd sniff && just lint` → exit 0. Skill drift fixed:
`.claude/skills/rust-testing/test-audit-tooling.md` now documents the
`attribute budgets` subcommand and JUnit-XML input (used here for the first
time on this area).

**Checkpoint 3 status:** the spec's four baseline-review decisions each have
a written, measurement-backed answer in `inventory.md`; budgets sit beside
the baseline in `inventory.md`; no budget was derived from a local run (the
gate refuses, recorded above); no production defect was found in place of a
filing. **Pending:** budget *ratification* — needs two more green CI runs of
the `03ce3f8c1` source state (operator-gated `gh run rerun 34008778001`;
Phase 9 decides whether to request them) and, inherently, L3 (no CI route).
Implementation continues per the plan's pending clause.

## Phase 4 — the deterministic CLI fixture (2026-09-08)

First Rust-code phase of this fix. `common/mod.rs`'s existing helpers
(`md_cmd`, `md_file`, `mock_http_server`, `baseline::*`, `layout::*`) are
byte-identical to their Phase 1 state; the phase only adds.

### What landed

- `darkmatter/cli/tests/common/fixture.rs` (new, 860 lines): the L1 spawn
  contract — `CliProcessFixture` (disposable workspace: `cwd`/`home`/`bin`/
  `config`/`cache`/`tmp`, alive through process completion and cleanup,
  containment-checked at construction against the canonicalized checkout
  root), `MdCommandBuilder` (named escapes `fake_only_path`, `host_path`,
  `ambient_context`, `inherit_no_env`; two surfaces — `build()` for
  `assert_cmd`, `build_std()` for a held child — driven by one computed
  `ChildEnvironment`), the env policy (scrub `DARKMATTER_*`/`DM_*`/`MD_*` by
  prefix, Git plumbing + `GIT_CONFIG_*`, rendering inputs, named darkmatter
  inputs; pin `HOME`/`USERPROFILE`/`APPDATA`/`LOCALAPPDATA`/`XDG_CONFIG_HOME`/
  `XDG_CACHE_HOME`/`TMPDIR`-family, `GIT_CONFIG_NOSYSTEM=1`, `NO_COLOR=1`,
  minimal `PATH`), Windows console restore (`SystemRoot`/`COMSPEC`/`PATHEXT`
  after `env_clear`, `cfg!`-gated so both arms compile everywhere), and
  topology builders (`initialize_repository[_at]` with repository-local
  identity and host-gitconfig bypass, `nested_schema_layout`,
  `relative_reference_layout`, `relative_reference`, byte-for-byte
  `copy_tree`).
- `darkmatter/cli/tests/md_process_fixture.rs` (new, 943 lines, 17 tests):
  proves the contract **from outside** — a recording stub
  (`dm-fixture-probe`) is executed by `md compose -`'s shell expansion
  (whitelisted at the fixture home and cwd) and asserts are made against
  what the stub recorded. Covers: pinned CWD/HOME/PATH and host-prefix
  isolation; every scrub family; post-build overrides outranking the scrub;
  `inherit_no_env` + Windows console restore; both-surface drift (×2,
  default and cleared); the three escapes plus two `#[should_panic]`
  rejections; the named hostile-environment test (throwaway-repo `GIT_DIR`,
  relocated `HOME`, `COLUMNS=44`, `FORCE_COLOR=1`, poisoned `PATH` with a
  hostile same-named stub, checkout-ancestor `TMPDIR` — all disposable,
  ancestor removed on drop); containment including the symlink-spelled
  variant; repository-local git init against a hostile `GIT_CONFIG_GLOBAL`/
  `GIT_CONFIG_SYSTEM`; relative-reference topology; byte-for-byte
  `copy_tree` of shipped `example-docs` content.
- `darkmatter/cli/tests/common/mod.rs`: `pub mod fixture;` + re-exports
  (323 → 335 lines; no existing symbol touched).
- `darkmatter/justfile`: the two new files registered in `lint-files`'s
  `ACCEPTED_OVERCAP` (reviewed single-responsibility exceptions; the recipe
  is advisory and not part of `just lint` — pre-existing unaccepted over-cap
  files were already listed before this phase and are unchanged).

### Design notes recorded for Phases 5–6

- md has **no env form of `--cache-root`**; persistent compose caching is
  strictly opt-in via the compose-only flag. The builder therefore pins the
  `dirs::cache_dir()` fallback (`XDG_CACHE_HOME`, `LOCALAPPDATA`) at the
  fixture, and cache tests pass `--cache-root <fixture dir>` explicitly.
- md **normalizes a missing `TERM` to `dumb`** for its shell children (an
  inherited TERM passes through verbatim) — probe-based TERM assertions
  therefore check for `[dumb]`, which still distinguishes scrubbed from
  inherited.
- The stdin-compose shell whitelist **anchors at the launch directory**
  (base-dir fallback after repository/home resolution), so probe fixtures
  seed `.darkmatter-shell-whitelist` at both cwd and home; the
  `ambient_context` escape needs it at the pinned directory.
- Shell-executed children observe md's pinned CWD only for stdin compose
  (a file source anchors the child at the document's parent); the self-tests
  compose from stdin for that reason.

### Validation ledger

| Command | Scope | Result |
|---|---|---|
| `cargo nextest run -p darkmatter-cli --test md_process_fixture` | fixture self-tests, after each iteration and after fmt | 17/17 PASS (0.26 s) |
| `cargo clippy -p darkmatter-cli --tests` | new code under `-D warnings` bar locally | clean |
| `RUSTFLAGS='-Wa,-mbig-obj' cargo check -p darkmatter-cli --tests --target x86_64-pc-windows-gnu` | Windows arms (mingw present) | PASS (2m08s cold); caught one real issue — `write_executable` import unused on Windows — fixed with `#[cfg(unix)]` gate. Runtime authority remains `windows-latest` (compile ≠ run) |
| `cd darkmatter && just test` | area L1 (lib + cli + dmls + zed-dmls), post-batch | 7,678 passed / 0 failed / 50 skipped, 42.0 s runner elapsed |
| `cd darkmatter && just lint` | area lint (clippy `-D warnings` ×4 + `check-zed`) | exit 0 |
| `cd darkmatter && just lint-files` | soft-cap report | both new files listed as **accepted**; no new unaccepted entries (pre-existing ones unchanged) |
| `rustfmt --edition 2024 --check` (new files only) | formatting | clean after one `rustfmt` pass on the three touched files |
| `git diff main -- .config/nextest.toml` | override invariant | file untouched by this fix; the pre-existing branch diff is the sibling claudine fix's override **removals** (zero darkmatter/dmls additions — verified no `+` line matches darkmatter/dmls) |

Source state for the gates above: this branch's working tree
(`fix-cli-slow-tests`, revision 973d1d918 + uncommitted Phase 4 files);
sibling claudine test-only changes present in the tree (recorded, per
assumption 4). GitNexus impact before editing: `md_cmd` (shared) upstream
risk LOW / 0 graph dependents; the phase adds without modifying existing
symbols.

**Checkpoint 4 status:** met. Focused fixture and affected-consumer tests
pass (all 30 `mod common` binaries recompiled and green in the area run);
hostile-environment and Windows-compile proof recorded; `common/mod.rs` at
335 lines (near its 323 baseline); override invariant holds. Windows
**runtime** behavior remains with `windows-latest` (compile-verified only
locally). `md_file()` and `baseline::*` unchanged and working; their caller
migration is Phase 6.

### Ledger addendum (post-format-revert)

The `rustfmt` pass used to format the new files walked `mod common;`'s child
modules and reformatted unrelated pre-existing code (`common/mod.rs`'s
`layout` module, `common/level2.rs`). The repo is not rustfmt-clean at large
(`args/cli.rs` and others pre-date this phase in `cargo fmt --check` drift,
and the area's `just lint` gate is clippy-only), so that was unrelated churn:
reverted. The final `common/mod.rs` diff is the 13-line module addition only;
`level2.rs` is untouched. Revalidation after the revert (whitespace-only
semantic no-op, rerun anyway): `just _test_local_all "darkmatter-cli"` →
678/678 PASS (8.85 s); `cargo clippy -p darkmatter-cli --tests` → clean. The
whole-area L1/lint rows above covered the other three packages, which do not
compile `cli/tests/common`, so their evidence is unaffected by the revert.

## Phase 5 — structural protection against new bypasses (2026-09-08)

Guard-only phase: no migrated test, no production code, no existing test
edited. `common/mod.rs`, `md_process_fixture.rs`, `common/fixture.rs`, and all
38 population files are byte-identical to their Phase 4 state.

### What landed

- `darkmatter/cli/tests/spawn_site_guard.rs` (new, 1,421 lines, 18 tests),
  modeled on claudine's `spawn_site_guard.rs` — same mechanics, no new
  mechanism:
  - **Spawn gate** (`l1_tests_spawn_md_through_the_fixture_builder`): three
    detected forms — `cargo_bin` naming `"md"` (catches both inline spawns
    *and* the bodies of private `fn md_cmd()` redefinitions, plan bullet a),
    any `md_cmd(…)` call (the shared helper at a call site, a private
    redefinition's own `fn` line, plan bullet b), and `bin_exe!("md")`
    (preventive; zero live L1 sites — its only user is tier-excluded
    `common/level2.rs::md_bin`).
  - **Isolation gate** (`migrated_l1_tests_keep_the_isolation_the_builder_gave_them`):
    `.current_dir(…)`, `.env("PATH", …)`, `.env_remove("PATH")`,
    `.env_clear()` in method position on a built command. Claudine's fifth
    form (`augmented_path`) has no darkmatter equivalent — PATH composition is
    internal to `MdCommandBuilder` — so four forms carry the contract.
  - `SPAWN_ALLOWLIST` seeded with **all 38 files**, each entry carrying its
    burn-down batch as the reason (`PENDING_6A/6B/6C`); the **stale-entry
    arm** fails on an entry with no live site, and reasons are mandatory.
  - `ISOLATION_ALLOWLIST` with exactly one entry — `md_process_fixture.rs`,
    whose two `.current_dir` sites (`:669`, `:869`) pin parent-side `git()`
    helper commands (hostile-state construction, config readback), the
    reference model's sanctioned resolution for receiver-unresolvable
    textual hits.
  - Census artifacts in the staging directory (`test_toolkit::stage_dir()`),
    `md-`prefixed to not collide with claudine's identically-shaped files:
    spawn `{"kind":"total",...,"files":38,"sites":518,"scanned_sites":518,"governed_files":41}`
    (reason roll-up: 6A 8 files/133 sites, 6B 11/210, 6C 19/175);
    isolation `{"kind":"total",...,"files":1,"sites":2,"governed_files":1}`.
  - Detector negatives (prose, string literals, comments,
    `cargo_bin("claudine")`/`cargo_bin("zed-dmls")`/`bin_exe!("dmls")` — the
    other-package case — identifier boundaries, path-qualified lookalikes,
    `std::env::set_var("PATH", …)` parent-process hostile-state seeding,
    `.env("COLUMNS", …)` post-build rendering pins which stay supported).
  - Population non-vacuity: the spawn gate asserts the seed census exactly
    (518 sites / 38 files / every entry live) and still detects a planted raw
    spawn appended to real site-free governed source; the isolation gate's
    population is asserted to be exactly the fixture self-test binary today,
    and `deleting_a_spawn_entry_is_what_widens_the_isolation_population`
    proves the single-edit widening rule Phase 6 relies on.
- `darkmatter/cli/tests/common/source_scan.rs` (new, 144 lines): the
  byte-preserving comment/literal sanitizer (claudine's, verbatim mechanics),
  included via `#[path]` so the guard binary compiles none of `mod common`.
- `darkmatter/justfile`: guard registered in `lint-files`'
  `ACCEPTED_OVERCAP`.
- `inventory.md`: new **§ Guard scope** recording the dmls / zed-dmls-cli
  decision (below).

### Census reconciliation

Detector census = 502 `md_cmd` call sites + 7 `fn md_cmd(` definition lines +
7 private-definition `cargo_bin("md")` bodies + 2 inline spawns = **518 sites
across 38 files**, matching the plan's grounding facts (its "504 sites" =
502 calls + 2 inline; the 7 definition lines and bodies are the guard's own
per-site decomposition of the same 7 private helpers). Every one of the 38
files carries an entry; every entry has ≥1 live site; `governed_files` = 41
(51 top-level test files − 11 `level2_*` − 4 `common/` + the guard itself).

### Neuter/restore transcription (non-vacuity of both gates)

Applied to the guard file only, each followed by restore and
`diff` back to identical (final state: `diff /tmp/spawn_site_guard.rs.bak
darkmatter/cli/tests/spawn_site_guard.rs` → empty):

1. **Raw-spawn violation arm** — deleted `toc.rs`'s `SPAWN_ALLOWLIST` entry:
   `l1_tests_spawn_md_through_the_fixture_builder` FAILS with
   `Raw md spawns outside the fixture builder. … Unlisted sites:
   toc.rs:8 md_cmd / toc.rs:19 md_cmd / toc.rs:31 md_cmd`.
2. **Stale-entry arm** — added a bogus entry for site-free
   `layout_alignment.rs`: the same test FAILS with
   `Stale SPAWN_ALLOWLIST entries name files with no raw spawn site left. …
   Stale entries: layout_alignment.rs`.
3. **Isolation-bypass arm** — emptied `ISOLATION_ALLOWLIST`:
   `migrated_l1_tests_keep_the_isolation_the_builder_gave_them` FAILS with
   `Post-build() escapes from the L1 isolation contract. … Escapes:
   md_process_fixture.rs:669 .current_dir(…) /
   md_process_fixture.rs:869 .current_dir(…)`.

### Guard scope decision (dmls / zed-dmls-cli)

Recorded in `inventory.md` § Guard scope: the guard governs
`darkmatter/cli/tests` only. `dmls`'s single `bin_exe!("dmls")` site is a
live-child LSP stdio session under Phase 8's remediation (an `md`-naming
detector would guard nothing 6D/8 do not already own); `zed-dmls-cli`'s six
`cargo_bin("zed-dmls")` sites are Phase 6D's named disposition and **cannot**
burn down under this guard until the Phase 11 `test_toolkit` promotion makes
a fixture importable from that package — seeding unburnable entries would
violate the burn-down-to-zero contract the guard enforces.

### Validation ledger

| Command | Scope | Result |
|---|---|---|
| `cargo nextest run -p darkmatter-cli --test spawn_site_guard` | guard tests, after each iteration | 18/18 PASS (0.04 s) |
| Neuter/restore cycles ×3 (above) | both gates' named failures | each FAIL→restore→PASS; file `diff`-identical |
| `rustfmt --edition 2024 --check` (new files only) | formatting | clean after one `rustfmt` pass on the two files |
| `cargo clippy -p darkmatter-cli --tests` | new code | clean |
| `RUSTFLAGS='-Wa,-mbig-obj' cargo check -p darkmatter-cli --tests --target x86_64-pc-windows-gnu` | Windows arms (mingw present) | PASS; only the known `-Wa,-mbig-obj` E0602 flag artifact (affects every crate in the chain, not this code) |
| `cd darkmatter && just test` | area L1 (guard included via `_tier_filter L1`; name carries no excluded prefix) | 7,696 passed / 0 failed / 50 skipped, 40.5 s runner elapsed — exactly Phase 4's 7,678 + the guard's 18; no test lost |
| `cd darkmatter && just lint` | area lint (clippy ×4 + check-zed) | exit 0 |
| `cd darkmatter && just lint-files` | soft-cap report | guard listed as **accepted**; no new unaccepted entries |
| `git diff main -- .config/nextest.toml` (darkmatter/dmls/zed `+` lines) | override invariant | 0 — unchanged from Phase 4's record |

Source state: this branch's working tree (`fix-cli-slow-tests`), Phase 4
files + Phase 5 additions uncommitted; sibling claudine test-only changes
present (recorded, per assumption 4). Guard-only changes: Phase 4 fixture
evidence reused; no full-area re-run required by the checkpoint, though one
was taken anyway (above).

**Checkpoint 5 status:** met. The guard runs in L1; its census is reconciled
(518/38/41 recorded in the artifact and asserted in the non-vacuity test);
focused tests reject raw spawns, isolation bypasses, and stale entries, with
all three named failures demonstrated live by the neuter/restore cycles. The
temporary migration exemption list is seeded, reasoned, and now owned by
Phase 6's burn-down.

## Phase 6 — Migration burn-down (2026-09-08)

All 518 guarded raw-spawn sites were migrated without changing the test
population. The seven private `md_cmd()` helpers and the shared compatibility
helper were deleted; the two baseline orphan spawns moved to the same fixture
contract. The clean/hash/frontmatter and compose/layout/render/graph families
now construct one `CliProcessFixture` per test and pass it through helpers.
Repository and relative-reference cases construct disposable fixture topology
without replacing authored relative references with absolute paths. Genuine
shell tests use documented `host_path()` escapes; command-absence coverage uses
`fake_only_path()`. Rendering-policy tests pin terminal inputs explicitly. The
HTTP server implementation was deliberately left for Phase 8.

The six `zed-dmls-cli` integration spawns use a package-local
`ZedDmlsFixture`: fixture-owned CWD/home/config/cache/temp, empty `PATH`,
scrubbed Git/rendering/Darkmatter inputs, and explicit fixture-owned
staging/data/log paths. Their original exit, stdout/stderr, staged-artifact,
and registration-link assertions remain unchanged.

### Requirement-to-test mapping

| Requirement | Observable proof |
|---|---|
| Every deterministic `md` spawn uses the fixture | `spawn_site_guard::{l1_tests_spawn_md_through_the_fixture_builder,the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site}` reports 0 raw sites across 41 scanned L1 files and still rejects a planted site. |
| Migrated commands cannot undo isolation | `spawn_site_guard::migrated_l1_tests_keep_the_isolation_the_builder_gave_them` governs 39 fixture-using files; its only allow-listed sites are two parent-side Git setup commands in `md_process_fixture.rs`. |
| Hostile inherited inputs do not affect results | `md_process_fixture::{inherited_families_do_not_reach_the_child_and_fixture_defaults_win,hostile_inherited_environment_leaves_the_fixture_defaults_unchanged}` covers hostile CWD/home/config/cache, throwaway `GIT_DIR`/`GIT_WORK_TREE`, poisoned `PATH`, rendering/application variables, and checkout-ancestor `TMPDIR`. |
| Relative and shipped topology survive relocation | `md_process_fixture::{topology_builders_keep_references_relative,copy_tree_relocates_shipped_content_byte_for_byte}` plus the preserved graph/validate/schema baseline E2E tests. |
| Persisted values still round-trip | Preserved frontmatter set/rm and hash save/diff/idempotence tests in the Phase 6B focused cohort; no assertion or identity was removed. |
| `zed-dmls` public behavior survives isolation | All six tests in `zed-dmls-cli/tests/cli.rs`, including the exact original command arguments and dependent filesystem/link assertions. |

No parser, schema, template, prompt, or configuration artifact changed in this
phase, so no new passive corpus was required. Existing shipped-schema and
baseline corpus/E2E cases remained selected. Test attributes changed by this
phase: **0 added, 0 removed**; the full L1 population remained 7,696.

### Validation ledger

| Command | Result |
|---|---|
| Phase 6A focused nextest | 118/118 passed, 0 skipped |
| Phase 6B focused nextest (including guard) | 220/220 passed, 0 skipped |
| Phase 6C focused nextest | 194/194 passed, 0 skipped; post-tightening rerun 159/159 |
| `cargo test -p zed-dmls-cli --test cli` | 6/6 passed |
| `cargo nextest run -p darkmatter-cli --test md_process_fixture` | 17/17 passed |
| `cargo test -p darkmatter-cli --test spawn_site_guard` | 18/18 passed; census 0 sites / 0 exempt files / 41 scanned files |
| `cd darkmatter && just test` | 7,696/7,696 passed (1 slow), 50 tier-filtered skips, 37.339 s runner |
| `cd darkmatter && just lint` | passed for all four packages plus `wasm32-wasip2` Zed extension check |
| `cd darkmatter && just test-l2` | darkmatter 18/18, darkmatter-cli 69/69, dmls 3/3; all selected tests passed |
| `RUSTFLAGS='-Wa,-mbig-obj' cargo check -p darkmatter-cli -p zed-dmls-cli --tests --target x86_64-pc-windows-gnu` | passed; only the known command-line `-Wa,-mbig-obj` E0602 warning repeated across dependencies/targets |
| `git diff --check` | passed for the Phase 6 paths |

A diagnostic plain-`cargo test` run of `md_process_fixture` passed 16/17 and
exposed the known shared-process environment race between tests that mutate
`PATH`; the canonical nextest rerun passed 17/17 because nextest gives each
test its own process. No retry, timeout increase, or exclusion was added.

**Checkpoint 6 status:** met locally. Zero generic spawn exemptions remain;
all migrated families, contamination probes, full L1, lint, and affected L2
coverage are green. Browser/L3 were not run because Phase 6 changed neither
browser nor OS-input behavior.

## Phase 7 — test design map (2026-09-08)

This map was recorded before Phase 7 source/test edits.

| Changed behavior | Public observable proof and required boundary |
|---|---|
| Incidental composition tests must not walk the checkout | Existing compose integration assertions remain byte-for-byte behavioral proof while their options use the request-scoped, demand-driven context API. `regression_ctx_agent_uses_compose_env_override` retains the original `AGENT="  codex  "` / `MODEL="  gpt-5  "` inputs and rendered `Agent: codex` / `Model: gpt-5` outputs. `capture_shape_matches_projected_type` continues to assert every catalog value's JSON shape, but against a disposable repository rather than the checkout. |
| Full ambient capture remains covered deliberately | Retain `ambient_ctx_capture::{every_catalog_variable_survives_ambient_options,explicit_full_capture_matches_ambient_options}` unchanged: both exercise the public compose path over the purpose-built two-package repository and assert every catalog projection, capture requirements, and diagnostics. |
| Demand-driven capture observes only referenced groups | Retain `context::runtime::evidence_tests::{unrequested_host_groups_need_no_host_evidence,missing_supplied_evidence_is_partial_and_never_falls_back}` and `normal_compose_path_renders_all_git_context_values_from_one_snapshot`; together they assert requested/unrequested groups, fail-closed missing evidence, and dependent rendered Git output. |
| Source provenance and CLI-to-library wiring remain covered | Retain `reference_integration::explicit_context_is_shared_by_enumeration_graph_and_validation` (one disposable repository, relative source winner, graph and validation agreement) plus Phase 6's real `md compose` fixture tests. |
| Schema validation and trigger matching are passive | Add a feature-gated library integration test that records `engine_build_count` and `network_attempt_count`, validates native/quoted/missing/null/malformed/boundary values, matches and rejects triggers, and asserts unchanged zero deltas plus the validation/match outputs. This is L1 because it is in-process and uses no resource. |
| DMLS diagnostics, completion, and hover are passive | Extend `dmls/tests/no_side_effects.rs` to record the same counter deltas around its real in-memory LSP session. Preserve the exact hostile document (`$(echo pwned)`, dangerous/sentinel shell directives, missing local references, and `https://example.com` values), assert diagnostics and successful read-side responses, and additionally assert no engine build/network attempt and no sentinel file. |
| Shipped schema artifacts remain passively covered without another scan | Reuse `meta_schema_phase4::repo_schemas_validate_their_own_examples`, `meta_schema_repo_schemas::repo_root_schemas_all_classify_as_standalone_schemas`, and `meta_schema_phase7_shipped_schema_provider_path`; no shipped parser/schema/configuration artifact changes in this phase, so another corpus walk would add cost without new failure detection. |
| Pure browser-target unit tests run in ordinary L1 | Rename only the 44 `browser_*` leaf names identified by the inventory to non-tier-prefixed names. Their unchanged bodies and exact HTML/CSS assertions are the replacement proof; Nextest listing plus targeted L1 execution proves the route. Real `browser_render::browser_*` tests retain the browser route. |
| Feature-gated helper tests have honest routes | Move/duplicate the pure pixel-classifier proof to an L1-reachable unit location, and give the real Mermaid integration a `browser_` route with an explicit harness/tool availability decision rather than silent skip-as-pass. Targeted listing/execution proves each route. |
| Host-shell-only ignored test is explicit | Preserve the original `ll` input but add an ignore reason naming the user's shell configuration dependency; stub-shell siblings remain the deterministic negative/success proof. |

No implementation semantics, parser, schema, template, prompt, persistence, or
configuration artifact is planned to change. Consequently the representation
matrix lives at the cheapest schema API boundary, while the existing real
shipped-artifact and LSP/CLI paths supply the integration coverage.

### Phase 7 implementation and validation

The capture audit found two incidental ambient captures. The agent/model
regression now asks `ComposeContext::capture_for_content` for exactly its
`ctx.agent.*` references, retaining the original whitespace-padded environment
inputs and normalized rendered output. The catalog shape matrix now captures
inside a disposable initialized repository rather than whichever checkout ran
the test. `ambient_ctx_capture.rs` was inspected and deliberately left
unchanged: both tests already use their own two-package repository and are the
named full-capture contract.

The passive schema and trigger tests now compare both effects counters before
and after their existing native, quoted, missing, null, malformed, and boundary
cases. The DMLS proof makes the same comparison around a complete in-memory LSP
session over its original hostile document, while retaining diagnostics,
completion, hover, definition, reference, shutdown, and sentinel assertions.
The local and CI L1 recipes enable the instrumentation feature for both
packages, so these assertions execute in the canonical gate rather than only
in an opt-in test.

The route audit made three population-preserving repairs:

- 44 pure HTML/CSS assertion tests use `render_browser_*` leaf names and now
  execute in L1 on every platform instead of only in the Chrome cohort.
- `pixel_classification_distinguishes_magenta_from_black` moved from the
  `terminal-tests`-gated binary into an always-built integration binary. Its
  exact 64×64 solid-magenta and solid-black inputs still check total,
  near-target, and non-black counts. The classifier remains shared with the
  L3 paint proof.
- `browser_sanitized_real_mermaid_retains_diagram_geometry` now has the
  canonical browser route. Missing `mmdc` is recorded through `require_level!`;
  when the CLI is installed, failure to produce SVG is an assertion failure
  instead of a successful early return.

No parser, schema, template, prompt, persistence format, or shipped artifact
changed. The existing passive corpus tests
`repo_schemas_validate_their_own_examples`,
`repo_root_schemas_all_classify_as_standalone_schemas`, and
`meta_schema_phase7_shipped_schema_provider_path` therefore remain the shared
corpus and real normal-invocation proofs; adding another scan would not detect
a new failure. Real shell expansion, transclusion, interpolation, hashing,
persisted-state round trips, source provenance, demand-driven context capture,
and CLI-to-library composition tests were retained.

#### Requirement-to-test mapping

| Requirement | Observable proof |
|---|---|
| Incidental capture is request-scoped | `expression_regression::regression_ctx_agent_uses_compose_env_override` retains the exact `AGENT="  codex  "` and `MODEL="  gpt-5  "` inputs and `Agent: codex` / `Model: gpt-5` outputs; `context::catalog::tests::capture_shape_matches_projected_type` retains every projected JSON shape in a disposable Git repository. |
| Full and lazy capture remain intentional | Unchanged `ambient_ctx_capture::{every_catalog_variable_survives_ambient_options,explicit_full_capture_matches_ambient_options}`, `context::runtime::evidence_tests::{unrequested_host_groups_need_no_host_evidence,missing_supplied_evidence_is_partial_and_never_falls_back}`, and `normal_compose_path_renders_all_git_context_values_from_one_snapshot`. |
| Provenance and CLI wiring survive | Unchanged `reference_integration::explicit_context_is_shared_by_enumeration_graph_and_validation` plus the Phase 6 `md compose` fixture cohort in the full L1 run. |
| Validation and trigger matching perform no effects | `schemas_literal_expression::expression_validation_never_evaluates` and `meta_schema_phase4::semantic_types_match_triggers_by_passive_parse` assert their original outputs and zero effect-engine/network counter deltas. |
| DMLS read-side requests perform no effects | `no_side_effects::dsl_requests_spawn_no_processes_and_open_no_sockets` retains the exact hostile shell/remote/missing-reference document, security diagnostic and response assertions, sentinel absence, and now zero counter deltas. |
| Representation variants stay at the cheapest boundary | Existing `schemas_literal_expression` and `meta_schema_phase4` cases cover native/quoted, present/missing/null, malformed, collection, and integer-boundary values; the shipped corpus and DMLS session cover the real artifact/integration paths. |
| Renderer unit tests have an L1 route | All 44 renamed tests ran under the ordinary `just test` selector with unchanged HTML/CSS assertions. |
| Feature-gated proofs have honest routes | `image_pixel_classification::pixel_classification_distinguishes_magenta_from_black` ran in L1; `browser_sanitized_real_mermaid_retains_diagram_geometry` ran in the 43-test browser cohort with `BISCUIT_TEST_LEVEL_REQUIRED=2` also passing on this host. |
| Host-shell smoke test explains its gate | `resolve_alias_ll` retains the exact `ll` input and now carries an explicit host login-shell ignore reason; deterministic stub-shell success and negative siblings remain in L1. |

#### Validation ledger

| Command | Result |
|---|---|
| Focused library integrations (`schemas_literal_expression`, `meta_schema_phase4`, `expression_regression`, `image_pixel_classification`, `disclosure_render_targets`) with effects instrumentation | 110/110 passed |
| Focused library selector (`capture_shape_matches_projected_type` and `render_browser_*`) | 44/44 passed |
| `cargo nextest run -p dmls --features effects-instrumentation --test no_side_effects` | 1/1 passed |
| Required Mermaid test with `BISCUIT_TEST_LEVEL_REQUIRED=2` | 1/1 passed; installed toolchain produced sanitized SVG geometry |
| `cd darkmatter && just test` | 7,741/7,741 passed (1 slow), 6 intentionally skipped |
| `cd darkmatter && just test-l2` | darkmatter 18/18, darkmatter-cli 69/69, dmls 3/3; all passed |
| `cd darkmatter && just test-browser` | 43/43 passed, including the newly routed Mermaid identity |
| `cd darkmatter && just lint` | passed for all four packages and the `wasm32-wasip2` Zed extension check |
| Terminal-feature target warning check | `level2_render_tree_terminal` compiled warning-free after removing an unnecessary shared-helper import |
| L3 image target static gate | `cargo clippy -p darkmatter --features terminal-tests,browser-tests --test level3_image_painting --no-deps -- -D warnings` passed; no foreground input was invoked |
| `test-audit config validate` | `config OK: darkmatter` (4 packages, 11 selections, 4 environments) |
| `git diff --check` for Phase 7 paths | passed |
| GitNexus pre-edit impacts | all edited existing test symbols reported LOW risk; no production function was changed |
| GitNexus `detect_changes(compare main)` | completed; repository-wide result is CRITICAL because the shared worktree already contains 1,599 changed files from all active phases/packages, not because of the Phase 7 test-only slice |

L3 was not run: it requires foreground OS input and screen capture, which is
outside an unattended session. Its shared classifier compiled through the
terminal-feature graph, and the classifier's exact positive/negative inputs ran
in L1. No doctest inputs changed, so no affected doctest run was required.
The six L1 skips are the four opt-in performance harnesses, the explicitly
ignored host-`ll` smoke test, and the locally excluded `slow_` cleanup case.
There were no test failures, retries, or pre-existing failures in the gates
above. The first L2 compile emitted warnings for an unnecessary helper import;
that import was removed and the affected target recompiled warning-free.

**Checkpoint 7 status:** met. All Phase 7 todos are checked, passive paths are
counter-proven, full/shipped/real-composition coverage is named, route repairs
are exercised at their proper tiers, and the required broad L1 and lint gates
are green.

## Phase 8 — Resource ownership (2026-09-08)

Phase 8 changed test infrastructure only. The loopback HTTP fixture now owns a
nonblocking accept worker, bounds in-flight reads, captures request content,
and joins on explicit shutdown or drop. Its 10 ms shutdown poll plus 1 s read
timeout has a documented 2 s termination bound. The original no-request
case is now a direct teardown regression instead of relying on process exit.

The real DMLS stdio test launches from per-test CWD/home/XDG directories and
owns its child through an RAII guard. `wait-timeout` replaces the 20 ms
`try_wait` loop, successful process exit is asserted, and timeout/unwind paths
kill and reap. `lsp_session` retains the exact protocol-response and
notification conditions used by its 88 tests while now owning and joining the
server thread on both normal shutdown and unwind. Those sessions already use
per-test temporary workspaces; DMLS has no persistent cache layer on this
passive server path.

Neovim tests now isolate both the editor and DMLS child under per-test
HOME/XDG roots. Their real pane checks poll the final SGR clear/repaint
condition every 50 ms to a deadline. The CLI's three post-sentinel 250 ms
sleeps use `biscuit_test_harness::capture_settled`, whose observable condition
is a visible prompt plus two byte-identical frames. Existing 50 ms sentinel
and render-tree polls were retained because they already observe the final
asserted condition with a deadline.

The L3 popover and image-painting waits now poll final DOM/accessibility/pixel
state with 2–5 s deadlines. L3 remains opt-in: unattended `just test-l3`
refused before opening or focusing a window because
`BISCUIT_L3_TAKE_FOCUS=1` was not set, so runtime L3 evidence is pending. L2
used background panes and browser coverage stayed headless with protocol input
only; neither tier invoked focus or host-input APIs.

### Requirement-to-test mapping

| Changed behavior | Observable proof |
|---|---|
| Expected HTTP request never arrives | `compose_remote_caching::mock_http_server_without_expected_request_shuts_down_within_bound` uses the original zero-request condition and asserts teardown inside the documented bound. |
| Consent, request content, refresh, TTL, fallback, and errors remain correct | The 11 real `md compose` tests retain success/error and rendered-output assertions; fetched cases add exact GET path and loopback Host checks. `test_compose_remote_ttl_serves_cached_url_without_second_request` performs two normal CLI reads and proves one HTTP request. |
| Native DMLS child exits and cannot survive failure/cancellation | `native_binary_speaks_lsp_over_stdio` asserts initialize capabilities, shutdown response, and successful process status. `child_guard_reaps_process_during_unwind` starts the exact test-binary probe, unwinds a panicking assertion, and proves the detached-completion marker is never written. |
| In-memory DMLS sessions finish on protocol state | All 88 `lsp_session` tests passed; request, diagnostics, startup progress, refresh, shutdown, and exit paths use bounded channel receives and the server thread is joined. |
| Real editor and terminal assertions wait for final state | Canonical L2 passed 18/18 library, 69/69 CLI, and 3/3 DMLS tests; Neovim clear/repaint and terminal frames are condition-driven. |
| Browser remains headless; L3 owns focus | Canonical browser passed 43/43 with no survivor and no headful/input API. L3 feature targets compile lint-free; runtime is pending because the unattended focus gate refused. |
| Workers and children leave no survivors | Targeted nextest run passed 103/103 under the configured leak timeout and external survivor sweep; browser passed 43/43 under the sweep. Both reported no survivors. |

No production parser, schema, prompt, persistence format, configuration, or
shipped application artifact changed. Therefore the passive corpus and
read/write/read requirements are unchanged; the Phase 7 shipped-schema corpus
and existing persistence matrices remained in the 7,745-test L1 gate. The
Neovim Lua file is test harness input and is exercised end-to-end by all three
real Neovim tests.

### Validation ledger

| Command | Result |
|---|---|
| `cargo test -p darkmatter-cli --test compose_remote_caching` | 12/12 passed |
| `cargo test -p dmls --test stdio_subprocess` | 3/3 passed, including unwind cleanup |
| `cargo test -p dmls --test lsp_session` | 88/88 passed |
| Feature-target compile checks for Neovim, CLI L2, and L3 tests | passed |
| `cd darkmatter && just test-l2` | darkmatter 18/18, darkmatter-cli 69/69, dmls 3/3; no nextest LEAK result |
| `cd darkmatter && just test-browser` | 43/43 passed |
| Targeted L1 nextest under `leak-sweep` | 103/103 passed; no surviving process |
| Browser tier under `leak-sweep` | 43/43 passed; no surviving process |
| L2 tier under `leak-sweep` | external sweep found no survivors, but its wrapper caused the shared WezTerm broker panes to disappear after 5 library and 10 CLI passes (`no such pane`); the direct canonical run above is the passing L2 authority |
| `cd darkmatter && just test` | 7,745/7,745 passed; 6 intentional skips |
| `cd darkmatter && just lint` | passed for all four packages and `wasm32-wasip2` Zed extension |
| L3 clippy with terminal/browser features and `-D warnings` | passed |
| Windows GNU test compile for `darkmatter-cli` and `dmls` | passed; only the known command-line `-Wa,-mbig-obj` E0602 warning repeated |
| `cd darkmatter && just test-l3` | intentionally refused unattended before focus; runtime evidence pending |
| GitNexus `detect_changes(compare main)` | completed; repository-wide result is CRITICAL because the shared worktree contains 1,601 changed files from concurrent phases/packages, while Phase 8 itself is confined to the test-only symbols whose pre-edit impacts were LOW/MEDIUM |

One post-gate targeted HTTP rerun exposed an intermediate empty-request
capture when a connected client did not send headers inside the initial 250 ms
fixture read bound. The bound was changed to 1 s (with a 2 s total documented
shutdown bound), and the exact 12-test target passed on rerun. This was an
implementation-iteration failure, not a pre-existing suite failure or retry
policy change.

**Checkpoint 8 status:** met locally. Every Phase 8 todo is checked; required
L1/lint and available L2/browser gates pass, cleanup is empirically clean, and
the only unavailable evidence is the deliberately human-authorized L3 tier.

## Phase 9 — Local measurement (2026-09-08)

The detached baseline at `973d1d9184601ab3409654955ffacc249f4870a5`
was compiled and warmed in its own `/tmp/rb-dm-baseline-973d1d9/target`
directory. The candidate retained this checkout's separate warm `target/`.
No cold-build performance claim is made; the baseline's initial 106-second
compile is setup evidence only and is excluded from every alternating sample.

The accepted series is defined by `measurement/plan.json` and stamped in
`measurement/provenance.json`: macOS 27 arm64, Apple M4 Max (16 cores), rustc
and Cargo 1.97.1, cargo-nextest 0.9.136, just 1.56.0, Node 22.20.0, default
nextest profile, `TERM=dumb`, and `NO_COLOR=1`. Baseline was detached and
clean; candidate was the recorded dirty `fix/cli-slow-tests` source state.
Filesystem modification-time checks found no source edit beneath the four
Darkmatter package source/test roots during the accepted window. No competing
Cargo/nextest/test job was present. macOS background services remained
variable, so all numbers below are attribution only.

### Alternating L1 evidence

`test-audit measure report` passed with five alternating warm runs per revision
for each separately labeled population. Every run passed with zero failures,
timeouts, leaks, or retries and stable identities within its revision.

| Population | Revision | Identities | Runner elapsed min / median / max | Summed duration min / median / max | Skips |
|---|---|---:|---:|---:|---:|
| local-default | baseline | 7,661 | 43.17 / 56.54 / 58.32 s | 684.79 / 899.03 / 925.70 s | 50 |
| local-default | candidate | 7,745 | 50.02 / 62.36 / 63.70 s | 787.43 / 988.08 / 1,010.29 s | 6 |
| CI-selected (`BISCUIT_L1_INCLUDE_SLOW=1`) | baseline | 7,662 | 51.60 / 57.28 / 60.57 s | 817.09 / 909.24 / 959.65 s | 49 |
| CI-selected (`BISCUIT_L1_INCLUDE_SLOW=1`) | candidate | 7,746 | 60.87 / 62.50 / 75.52 s | 963.46 / 990.97 / 1,195.39 s | 5 |

Candidate adds 84 asserted identities and removes none in either population;
the local/CI delta remains exactly the one `slow_` cleanup identity. Aggregate
candidate medians were about 9–10% higher but remained inside the broad local
drift brackets, so no aggregate change is established. The changed
`darkmatter-cli` integration cohort fell from a 194.03-second to a
100.81-second median summed duration (−48.0%, outside both drift brackets), and
the loopback HTTP cohort fell from 25.27 to 3.13 seconds (−87.6%). These are
local attribution findings, not CI targets; Phase 3's CI budgets remain the
authority and pending CI evidence remains pending.

### Timing, concurrency, and eliminated work

Every Phase 8 L1 HTTP/process/protocol target ran twelve candidate times (two
warm-ups plus ten alternating population runs). Ten additional canonical
`just test-l2` runs exercised all 90 selected real-terminal tests; all passed,
with runner elapsed 88.93–90.53 seconds, no retry/leak result, and fresh broker
setup/teardown per run. The report includes per-identity spreads. L3 runtime
remains pending because this non-interactive session cannot set the explicit
focus/OS-input authorization; it was not misreported as a pass.

`measurement/work-evidence.md` maps discovery, composition, effects, and HTTP
claims to hostile/executable sentinels, effect/network counters, and exact
loopback request counts. Discovery/composition lack general production
counters, so those claims are scoped to the eliminated paths rather than
inferred from timing.

### Failures retained during measurement

- A first attempt was rejected before evidence collection because its initial
  load was unsuitable; its partial record is under `rejected-high-load/`.
- The next candidate warm-up exposed
  `test_compose_remote_prologue_allowed_host_fetches_url`: the nonblocking
  listener's accepted macOS socket could return `WouldBlock` before reqwest
  wrote the request, consuming the only response. GitNexus reported MEDIUM
  test-only impact (8 direct callers, no production flow). Explicitly restoring
  blocking mode fixed the fixture; the exact failing test then passed directly
  and in all twelve accepted loaded executions. The failed run is retained
  under `rejected-http-regression/`.

The final `measurement/report.gate.txt` is `GATE EXIT=0`. Post-series process
inspection found no surviving `md`, `dmls`, broker, test, or Chrome-debug
process. Build/setup, runner elapsed, and summed duration remain separate in
`measurement/report.md`; raw compressed logs and `runs.jsonl` retain every
identity, failure, skip, timeout, retry, slow mark, command, and load sample.

Final gates: six accepted post-fix local-default runs each executed
`cd darkmatter && just test` successfully (7,745/7,745 passed, 6 intentional
skips); six CI-selected variants passed 7,746/7,746 with 5 skips. A final
`cd darkmatter && just lint` passed all four packages and the
`wasm32-wasip2` Zed extension check. `git diff --check` passed for the Phase 9
paths. GitNexus `detect_changes(compare main)` reports CRITICAL only for the
shared worktree's existing 1,601-file, 9,700-symbol aggregate; Phase 9's
pre-edit analysis remains the scoped authority for its one changed symbol
(MEDIUM, 8 direct test callers, no production execution flow).

**Checkpoint 9 status:** met locally. Both full L1 populations have five
alternating samples per revision, changed cohorts come from those reports,
every available changed timing/concurrency contract has at least ten candidate
executions, and all eliminated-work claims have counter or sentinel proof.

## Phase 10 — CI evidence and operator handoff (2026-09-08)

Phase 10 changes no Darkmatter production behavior. Its observable contracts
are evidence integrity, cross-platform compilation, unchanged tier/feature
selection, and a reproducible operator handoff. The test map was fixed before
the one source edit:

| Requirement | Observable proof |
|---|---|
| Complete reports route through the console adapter while malformed inputs fail | `attribute.test.ts` feeds one real-shaped nextest result plus its closing summary through `parseAndCheckLogs`; the existing malformed XML case still asserts `malformed-report`. |
| Current test identities remain completely inventoried | A fresh 11-selection `capture` followed by `reconcile` assigns all 7,892 feature-unioned identities exactly once across 65 families and exits 0. |
| CI-feature L1 behavior remains green | Root `just ci-local darkmatter` expands to the four declared packages and their exact CI feature sets, then runs the preflight plus lint and L1 tests once. |
| Windows-only arms compile | One GNU-target test check covers all four packages with the same terminal/browser/effects feature selections; native runtime evidence remains owned by `windows-latest`. |
| Resource tiers remain reachable without focus theft | Current-state Phase 9 canonical L2 runs (ten consecutive, 90 tests each) and the Phase 8 headless browser run (43 tests) remain reusable because Phase 10 changed only audit evidence/test input. L3 remains pending behind its unattended focus gate. |

The first `tools/test-audit && just check` exposed a fixture regression in
`attribute.test.ts`: the exact input
`PASS [   0.400s] demo::beta test_two` lacked nextest's `(current/total)` field,
so the strict adapter correctly ignored it and reported `truncated-log`. The
fixture now uses `PASS [   0.400s] (1/1) demo::beta test_two`. No parser
acceptance was widened. The targeted test passed, then the complete tool gate
passed **164/164**. No parser, schema, prompt, template, persistence format, or
shipped configuration changed, so the existing passive shipped-artifact corpus
and read/write/read matrices remain the appropriate coverage; no duplicate
corpus scan was added.

### Local validation ledger

| Command/evidence | Result |
|---|---|
| `just ci-local darkmatter --dry-run` | four packages; `darkmatter` with terminal/browser/effects, CLI with terminal, DMLS with terminal/effects, and `zed-dmls-cli`; lint + test are both included |
| `just ci-local darkmatter` | exit 0; CI-infrastructure preflight plus lint and L1 for all four packages, **9/9 gates** |
| Darkmatter package results inside that gate | `darkmatter` 6,375/6,375; `darkmatter-cli` 702/702; `dmls` 645/645; `zed-dmls-cli` 27/27; no failure or retry |
| Windows GNU feature/test compile | exit 0 for all four packages; the known command-line `-Wa,-mbig-obj` E0602 warning repeated, with no source warning or compile error |
| `cd sniff && just lint && just test` | exit 0; lint clean, then 2,599/2,599 passed with 23 intentional skips |
| `tools/test-audit && just check` | TypeScript clean; 13 files, 164/164 tests passed after the targeted fixture correction |
| Fresh `test-audit capture` | 11/11 selections captured; local aggregate 7,751 identities; feature union 7,892 |
| Fresh `test-audit reconcile` | **GATE EXIT=0**; 7,892 identities assigned exactly once across 65 families |
| Baseline `test-audit junit` | 21/21 configured cells present for run 34008778001; zero failures, skips, or retries |
| `attribute budgets` over `budgets-pending.json` | expected **GATE EXIT=1**: one baseline run on each of ubuntu, macOS, Windows, and WSL2; three are required |

CI selection is not reduced. `BISCUIT_L1_INCLUDE_SLOW=1` remains in both
native and WSL workflows and still adds exactly the one documented `slow_`
identity. The dry run retains every declared `terminal-tests` and
`browser-tests` feature (and adds only the Phase 7 effects instrumentation),
and `_package-ci.yml:519` still runs `cd darkmatter && just zed-verify`.

### PR and CI handoff

This agent did not commit or push, as the phase and user both prohibit it.
There is no candidate CI source to fetch: local `HEAD` is `973d1d918`, the
tracking branch remains at merge-base `a9e88c069`, and the implementation is
in the dirty worktree. `gh run list` could not read GitHub because this
non-interactive host has no `GH_TOKEN`/GitHub CLI authentication. The operator
handoff is therefore:

1. Integrate current `main` without changing Darkmatter's nextest overrides,
   commit the complete working-tree implementation, and push/open the PR.
2. Review the first run on every configured cell for Linux, macOS, native
   Windows, and WSL2 correctness; retain every intervening failure.
3. Accumulate three consecutive green candidate runs per configured leg at
   one source/workflow/runner/feature state.
4. Fetch each qualifying run with `test-audit fetch`, gate each staging tree
   with `junit --baseline baseline/34008778001`, and compare matched identities
   within—not across—environments. Report added, removed, and gated identities
   separately.
5. Obtain the two missing compatible baseline samples, run `attribute budgets`
   to ratify the per-leg family ceilings, then compare all three candidate
   samples without changing the ceilings.

The nextest freeze also needs operator resolution before push. The working-tree
diff for `.config/nextest.toml` is empty, so Phase 10 added no override, retry,
or timeout change. The literal required command
`git diff main -- .config/nextest.toml` is **not** empty: local `main` is 67
commits ahead, while this shared branch is 36 commits ahead and carries sibling
Claudine commit `0af6cbb50` (`perf(claudine): declare CI tier metadata and
tighten lib test fixtures`). Its 16 additions / 41 removals are Claudine
comments and timeout-override removals; it does not add or alter a Darkmatter
override. The operator must reconcile that sibling branch history before this
plan's exact whole-file diff can close.

**Checkpoint 10 status:** local readiness and handoff are complete, but final
performance evidence is **pending**, not passing. Missing external evidence is
three candidate CI runs per leg, two additional compatible baseline runs for
budget ratification, matched per-environment comparison, and native CI runtime
for Windows/WSL2. L3 runtime remains deliberately pending in this unattended
session. No timeout, retry, tier, assertion, or nextest override was changed to
produce a number.

## Phase 11 — closure and acceptance ledger (2026-09-08)

Phase 11 changes documentation only. It introduces no production or test
behavior, so there is no new failing implementation input and no new targeted
test added in this phase. The behavior-to-test map for the implementation being
closed is:

| Requirement | Public observable test evidence |
|---|---|
| Deterministic CLI launch contract, including hostile inherited inputs and native path variants | `md_process_fixture` real-child contract tests plus `spawn_site_guard` planted raw-spawn, post-build mutation, stale-exemption, and sanitizer negatives; the complete exact identity list is in `inventory.md`. |
| Persisted values and representation variants | Existing `clean_frontmatter`, `hash_kind_save_diff`, `get_set_rm`, schema, and composition matrices retain native/quoted, missing/present, malformed, boundary, and repeated read/write/read assertions through the real `md` invocation path. |
| Passive shipped artifacts and dependent DMLS output | `schemas_literal_expression`, `meta_schema_phase4`, and `dmls::no_side_effects` cover the shipped corpus, real normal invocation path, diagnostics, completion, hover, definitions, references, and unchanged effect/network counters. |
| Bounded HTTP/process/protocol cleanup | `compose_remote_caching`, `stdio_subprocess`, `lsp_session`, and the DMLS cancellation/unwind tests assert request/result state and termination when expected interaction never occurs. |
| Terminal/browser boundary and unchanged assertions | Canonical `just test-l2` and `just test-browser` evidence verifies the helpers and real browser route; the 44 renderer identities moved to L1 retain their exact HTML/CSS assertions. |

The closure deliverables are `results.md`, an owned fixture-promotion feature,
an owned residual-findings fix, updated Darkmatter area READMEs, and updated
Darkmatter/rust-testing skills. The plan frontmatter now records Phase 11's
source, documentation, skill, package, and aggregate eleven-phase file lists.
No contradiction between an edited comment/document and current code was found.

### Reconciled validation ledger

| Command/evidence | Result |
|---|---|
| `cd darkmatter && just lint` | exit 0 for all four packages, including the `wasm32-wasip2` Zed check |
| `cd darkmatter && just doctest` | exit 0; 180 library doctests passed, 10 intentionally ignored; CLI doctests passed with zero selected |
| Phase 9 accepted `cd darkmatter && just test` samples | six current-implementation local-default executions passed 7,745/7,745 with 6 intentional skips; six CI-selected executions passed 7,746/7,746 with 5 skips |
| Phase 10 `just ci-local darkmatter` | exit 0; all nine gates, including lint and L1 for all four packages |
| Phase 9 canonical `just test-l2` | ten consecutive executions passed all 90 selected tests; no survivors or retries |
| Phase 8 canonical `just test-browser` | exit 0; 43 real browser tests passed |
| Phase 10 `cd sniff && just lint && just test` | exit 0; lint clean and 2,599/2,599 tests passed with 23 intentional skips; Phase 11 changes no Sniff code |
| Fresh `test-audit reconcile` | **GATE EXIT=0**; 7,892 feature-unioned identities assigned once across 65 families |
| `md hash --diff` for the two hash-managed edited documents | exit 0 for both after saving their current hashes |
| Phase 11 plan frontmatter parse | exit 0; phase 11, 137 aggregate source entries, 63 aggregate documentation entries, and all required list properties present |
| Local-link check for the three created closure/follow-up documents | exit 0; every relative target resolves |
| Phase 11 path-scoped `git diff --check`, plus `--no-index --check` for four new files | no whitespace diagnostics |
| Working-tree `git diff -- .config/nextest.toml` | empty; Phase 11 changed no override, retry, timeout, or tier |
| Required `git diff main -- .config/nextest.toml` | still nonempty: 16 additions and 41 removals from sibling Claudine branch history; operator reconciliation remains pending |

Two redundant fresh `just test` attempts were rejected under shared-host build
contention. They timed out different identities at the unchanged 30-second
limit after 2,177 and 5,837 passing tests respectively. The exact identities
then passed directly with the same features and timeout policy:

- `markdown::compose::frontmatter_interpolation::tests::seed_state_tests::env_resolves`:
  6.649 seconds;
- `markdown::compose::type_tests::test_compose_context_capture`: 6.871 seconds.

No retry or timeout adjustment was made, and the full suite was not restarted
again without a relevant source/environment change. The earlier accepted full
gates remain applicable because Phase 11 touched documentation only.

### Remaining external evidence

L3 runtime remains pending because this unattended session cannot grant its
foreground/focus prerequisite. Candidate `just zed-verify`, native Windows/WSL
runtime, three green candidate samples per configured leg, two additional
compatible baseline samples, budget ratification, matched-identity comparison,
and baseline/candidate report gating all remain operator-owned CI work. These
items prevent AC7 and final performance closure; they do not invalidate the
completed implementation or available-resource local verification.

**Checkpoint 11 status:** implementation, closure documents, owned follow-ups,
drift updates, local evidence reconciliation, and the acceptance table are
complete. Final CI performance verification remains pending, so the fix is not
reported as archived or fully verified on CI.

## Implementation of Review Findings #1

> **started at:** 2026-09-09T10:50:12-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests/darkmatter/fixes/2026-09-07-faster-darkmatter-tests/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review carries five findings — four **High**, one **Medium** — and two of
  the High findings carry operator annotations that change how they are treated:
        - finding 1 (CI comparison and ratified budgets) is annotated
          `**DECISION:** this bar was set to high and should not be considered a
          blocker being production ready`
        - finding 2 (Level-3 behavior) is annotated `**CRITICAL:** There needs
          to be a CLEAR explanation why the synchronization was changed`
- orchestration plan — findings are worked serially, each by a subagent using
  the `rust`, `rust-testing`, and `darkmatter` skills, with `just test` and
  `just lint` run in the `darkmatter/` package area only (the four packages the
  spec names: `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli`)
- starting state — the worktree is clean apart from `spec.md`; the fix's
  implementation is already committed as `8540d36df`, `a1746f217`, `a05e3b747`,
  which materially changes finding 4's premise (recorded under that finding)
- starting the work on 'F1 — required CI comparison and ratified budgets do not exist' at 10:53:41
        - handled by the orchestrator rather than a subagent: the finding has no
          code surface, and its resolution is an evidence/ruling record. Running
          the package-area `just test` + `just lint` gates for a two-paragraph
          documentation edit would burn several minutes of CPU for no signal;
          the gates still run for every finding that does touch code.
        - recorded the operator ruling in `results.md` § Budgets as a new
          subsection *Operator ruling — hosted budget evidence is not a
          readiness blocker*, stating plainly that the ruling changes the
          finding's **severity**, not the evidence: no budget was invented, no
          local timing was promoted to a target, no leg was marked green
        - the substantive reason the requirement cannot be met from this side:
          a *candidate* sample is by definition a hosted CI run of the candidate
          revision, and hosted CI does not run an unpushed revision. Holding
          readiness on it makes the fix permanently un-mergeable by its author.
        - created `deferred-performance-measurement.md` with the exact missing
          artifact table (1 of 3 baseline samples, 0 of 3 candidate samples, 0
          ratified budgets, 0 matched-identity comparisons, per leg) and the
          six-step operator-owned closing sequence, mapped back to `review-1.md`
        - noted explicitly in that file that this is **not** the usual
          "host CPU load spoiled the measurement" deferral — the two genuine
          host-load rejections this cycle are already retained under
          `measurement/rejected-high-load/` and
          `measurement/rejected-http-regression/`. This deferral is structural
          runner inaccessibility.
        - `results.md` status cells updated: *Verified on CI* → **Deferred**,
          AC7 → **Deferred (operator-ruled non-blocking)**
- work completed for 'F1 — required CI comparison and ratified budgets do not exist' at 10:58:07
- starting the work on 'F2 — Level-3 keyboard, pointer, and image-paint behavior was not executed' at 10:59:14
        - dispatched to a `rust-developer` subagent using the `rust`,
          `rust-testing`, and `darkmatter` skills, briefed that the whole
          session tree is non-interactive and that `just test-l3` must **not**
          be run — it injects OS input and seizes desktop focus on a host a
          person is currently using
        - the finding's `**CRITICAL:**` annotation (explain the synchronization
          change) was set as the primary deliverable; the reviewer's other ask,
          executing L3, is not reachable from an unattended session
- what the synchronization actually changed to, established from
  `git diff main...HEAD` plus a full read of both L3 files:
        - `level3_popover.rs` — five `sleep(150ms)`-then-`evaluate` sites became
          `wait_for_evaluation(script, what, condition)`, polling at 25 ms
          against a 3 s deadline. The polled predicate was verified byte-identical
          to the assertion it feeds in all five cases, so the wait cannot
          terminate on a weaker signal.
        - `level3_popover.rs` — `resolve_window_process_id` went from 20 × 100 ms
          fixed attempts to a 2 s deadline at 25 ms; same success condition, only
          the bound's units changed.
        - `level3_image_painting.rs` — the fixed `sleep(400ms)` plus a single
          `capture_window_png` became a 5 s / 50 ms poll whose exit test
          `magenta > 1000 && non_black * 100 >= total` is exactly the conjunction
          of the `assert!` below it and the non-black guard above it. Side
          effect: one transient `None` capture used to abandon the pixel claim as
          a skip; only an all-`None` five seconds does now.
- why the change is strictly stronger rather than merely faster — the answer to
  the `**CRITICAL:**` annotation:
        - a fixed sleep is simultaneously a flake source and a hidden-pass
          source. Too short under load gives a spurious failure; and it asserts
          nothing about *when* the value became true, so a condition already true
          before the OS input arrived passed identically — which is precisely the
          thing L3 exists to rule out.
        - the polls assert the same final condition the test claims, and report
          the last observed value plus what they were waiting for on timeout
        - candid about masking, and recorded as such: both bounds are real
          ceilings. A regression delaying focus/navigation/hover to 2.9 s, or
          paint to 4 s, now passes where the old sleeps failed. Accepted because
          neither file asserts a latency contract at any tier — a sleep failing at
          151 ms was enforcing host load, not a budget.
        - the change is mandated by `spec.md` §4 ("Poll the final asserted
          condition"), not incidental
- **defect found and fixed** while answering task 3 (how the harness records a
  skip):
        - `require_level!` only enforces the gate at the *top* of a test.
          `level3_rich_image_node_paints_distinctive_pixels` had two further
          exits decided mid-body — every capture `None`, or an essentially-black
          capture from missing Screen Recording permission — that were bare
          `return`s and therefore reported **green** even under
          `BISCUIT_TEST_LEVEL_REQUIRED=3`.
        - consequence had it shipped: an operator's authorized L3 run on a
          mis-permissioned macOS host would have filed a pass for a pixel claim
          that was never evaluated
        - both exits now route through a new `skip_pixel_assertion(reason)`
          helper applying the same `BISCUIT_TEST_LEVEL_REQUIRED=3` rule and
          panicking with the reason; inert unless the variable is set
        - this is the one place in F2 where behavior changed rather than
          comments. Kept deliberately: the reviewer's "record skips as
          unavailable rather than passes" is unenforceable at that test without
          it.
- how a skip is recorded, verified in source rather than assumed:
        - libtest has no runtime "skipped" outcome — `require_level!` resolves to
          `LevelDecision::Skip`, prints `skipping: …` to stderr and returns
          (`tools/test-toolkit/src/lib.rs:258-272`), so the identity reports green
        - three mechanisms keep that green out of this fix's record, each checked:
          the `_tier_filter L1` excludes `test(/(^|::)level3_/)` so `just test`
          never selects them; `_test_l3` (`just/devops.just:1609-1634`) hard-refuses
          without a TTY unless `BISCUIT_L3_TAKE_FOCUS=1`; and
          `BISCUIT_TEST_LEVEL_REQUIRED=3` promotes an unavailable harness from
          Skip to Panic
- documentation written in two places, per the brief:
        - `//!` module docs in both L3 test files (`## Why the fixed sleeps became
          polls`, `## What the 3-second bound gives up`, and the image-paint
          equivalents) plus per-function docs on `wait_for_evaluation`
        - a new `## Level-3 synchronization and availability` section in
          `results.md`, anchor-able as
          `#level-3-synchronization-and-availability` (already linked from
          `deferred-performance-measurement.md`), carrying the rationale, a
          polled-predicate/assertion correspondence table, the masking
          trade-off, and the four UNAVAILABLE identities
- the four identities recorded as UNAVAILABLE, never as passes:
  `level3_popover_tab_focuses_anchor_and_reveals_prompt`,
  `level3_popover_enter_activates_link`,
  `level3_popover_pointer_hover_reveals_prompt`,
  `level3_rich_image_node_paints_distinctive_pixels`. These are all four L3
  tests in the area. `results.md` prescribes the closing invocation
  `BISCUIT_L3_TAKE_FOCUS=1 BISCUIT_TEST_LEVEL_REQUIRED=3 just test-l3` with
  stderr retained to prove no `skipping` line was emitted.
- gates:
        - compile-only verification, no tests executed:
          `cargo nextest list -p darkmatter --features terminal-tests,browser-tests
          -E 'test(/(^|::)level3_/)'` lists all 4 identities, exit 0, before and
          after the edits
        - `just lint` (darkmatter area): **PASS**, exit 0, zero warnings across
          all four packages plus the `wasm32-wasip2` Zed check
        - gap noticed and covered: `_lint` runs `cargo clippy --all-targets`
          without features, and both L3 targets carry `required-features`, so
          `just lint` does not actually reach them. Ran
          `cargo clippy -p darkmatter --all-targets --features
          terminal-tests,browser-tests -- -D warnings` explicitly — exit 0, clean.
        - `just test` (darkmatter L1): **PASS**, exit 0 —
          `7745 tests run: 7745 passed, 6 skipped` in 31.5 s across 121 binaries.
          The 6 skips match the population `results.md` already records. Nothing
          was failing before the change.
- blocked: L3 execution itself, for the reason above. Compile reachability is
  recorded as the only evidence and explicitly labelled non-behavioral.
- environment friction, not caused by this work: a concurrent agent editing the
  claudine fix in this shared worktree staged renames of four
  `claudine/cli/tests/level2_*_pty.rs` files without updating
  `claudine/cli/Cargo.toml`'s `[[test]] name = "level2_dry_run_pty"`, breaking
  workspace manifest parsing for every cargo invocation for ~3 minutes. All
  results above are from after it resolved.
- follow-up candidate noted, deliberately not fixed (outside this finding):
  `verify_keyboard_canary`'s `if !observed_canary` and `verify_pointer_canary`'s
  zero-check branches are now unreachable, since `wait_for_evaluation` returns
  `Ok` only when the identical predicate already holds. Their richer
  `PROVISIONING_FAILURE` diagnostics (pid, window title) are consequently dead
  and would be worth folding into the timeout path.
- work completed for 'F2 — Level-3 keyboard, pointer, and image-paint behavior was not executed' at 11:17:26
- starting the work on 'F3 — DMLS server-thread cleanup is neither complete nor bounded' at 11:18:02
        - dispatched to a `rust-developer` subagent with the `rust`,
          `rust-testing`, and `darkmatter` skills; the four acceptance criteria
          taken directly from the review (one shared owned fixture,
          non-panicking teardown during unwind, a documented completion bound,
          and a failure-path regression proving the worker finished before the
          workspace is released)
- verified all three of the review's claims against source before changing
  anything — the finding was accurate on every point:
        - `lsp_session.rs`'s `ClientFixture::drop` joined with no deadline and
          `.expect("server thread panicked")`
        - `no_side_effects.rs` (`struct Fixture`) and `suggest_constraint_phase1.rs`
          (its own `struct ClientFixture`) discarded the `thread::spawn` handle
          and had **no** `Drop` at all
        - `stdio_subprocess.rs`'s existing unwind regression covers only the
          `ChildGuard` subprocess path
- one shared owned fixture now lives at `darkmatter/dmls/tests/common/mod.rs`,
  consumed via `mod common;` from all three targets. `#![allow(dead_code)]` is
  load-bearing there (removing it produces 5 dead-code warnings, since each
  binary uses a different subset).
        - **deliberate design deviation from the brief, and a better one:**
          `LspFixture<'workspace>` *borrows* an `LspWorkspace` rather than owning
          the `TempDir`. Because `LspFixture` has a `Drop` impl, dropck makes
          that shared borrow strict — releasing the workspace while a session is
          live is a **compile error**, and a fixture declared after its workspace
          always drops first.
        - this converts the ordering contract from a drop-order convention
          documented in a comment into something the compiler enforces, which is
          what the review was reaching for
        - it also avoids rewriting 84 workspace call sites across 88 tests, and
          keeps working for the two tests that run two sequential fixtures
          against one workspace — fixture-owned `TempDir` would have broken those
        - per-test churn collapsed to two mechanical lines
          (`tempfile::tempdir()` → `LspWorkspace::new()`,
          `ClientFixture::start()` → `LspFixture::start(&workspace)`); every
          `workspace.path()` call site is untouched. Three filesystem-free
          sessions gained a workspace purely to have something to borrow.
- **completion bound: `SERVER_EXIT_BOUND = 10 s`**, placed on the worker's
  `mpsc` outcome channel rather than on `join()`, because `JoinHandle` has no
  timed join:
        - the worker records completion and sends its outcome as its last two
          acts, so observing the outcome *proves* the body finished and the
          subsequent `join()` cannot block meaningfully
        - on expiry the worker is deliberately **detached and reported**, never
          joined — joining a stuck worker would hang the runner forever. The
          reasoning is documented at the constant.
        - 10 s chosen because it is the same budget `MESSAGE_BOUND` already gives
          every single protocol message in this suite (the slowest legitimate
          reason a worker has not returned is an in-flight request or the startup
          disk walk), and because it must stay well under nextest's
          `terminate-after` ceiling (`slow-timeout = 5s × 6` = 30 s) or the
          teardown is killed before printing the diagnostic that makes a timeout
          actionable
        - a `Disconnected` outcome (worker unwound without sending) is
          distinguished from `Timeout`: the former is still safe to join, the
          latter is not
- **non-panicking teardown:** `release_server()` returns a `Vec<String>` of
  problems instead of asserting. `shutdown()` asserts on them; `Drop` gates on
  `std::thread::panicking()` and prints to stderr during unwind, because a
  second panic aborts the process and destroys the real assertion diagnostic.
        - confirmed live rather than by inspection — the regression run shows the
          original panic message intact with the teardown diagnostic printed
          *beside* it, not replacing it
        - added a `clean_shutdown` flag so an aborted session's expected
          `client sent 'exit' before 'shutdown'` server complaint is not
          misreported as a teardown failure; a session that did handshake still
          owes a clean exit, preserving the original `assert_eq!(outcome, Ok(()))`
- **new regression:** `server_worker_finishes_before_workspace_release_during_unwind`
  (`dmls/tests/lsp_session.rs:5711`). Drives the shared fixture to a failing
  assertion inside `catch_unwind`, then asserts the recorded teardown sequence is
  `[WorkerFinished { workspace_present: true }, WorkspaceReleased]` — the worker
  itself records, from its own thread, whether the workspace root still existed
  at the instant its body returned.
        - a 300 ms worker epilogue delay makes the distinction deterministic;
          without it an undelayed worker finishes so soon after the connection
          closes that a detaching teardown would win the race by accident
        - **non-vacuity proven in both directions**, as required before a new test
          is accepted:
                - break A (`SERVER_EXIT_BOUND` → `Duration::ZERO`) → FAIL:
                  `assertion 'left == right' failed: the unwinding fixture must
                  reap its server worker, and must observe the workspace still
                  present when it does / left: [WorkerAbandoned] / right:
                  [WorkerFinished { workspace_present: true }]`
                - break B (silent `drop(server_thread)` before the wait) → FAIL
                  with the same assertion and `left: []`
                - both breaks reverted; the test passes in 0.32 s
- gates, all run in the `darkmatter/` package area:
        - `cargo nextest run -p dmls --features effects-instrumentation --test
          stdio_subprocess --test no_side_effects --test suggest_constraint_phase1`
          — **23 passed, 0 skipped**, identical to the review's recorded baseline
        - `cargo nextest run -p dmls --test lsp_session` — **89 passed, 0 skipped**
          (88 pre-existing + the new regression)
        - `just lint` — **pass**; additionally
          `cargo clippy -p dmls --all-targets --features effects-instrumentation
          -- -D warnings` clean, confirming the test targets are actually covered
        - `just test` — **7746 passed, 6 skipped, exit 0**
        - nothing was failing before the change; no pre-existing failure is being
          claimed as fixed or introduced
- assertion accounting, so the coverage delta is not silent: `lsp_session`
  310 → 309, `no_side_effects` 12 → 9, `suggest_constraint_phase1` 49 → 46. The
  deltas are exactly the fixture helpers' own assertions (`initialize`,
  `flush_server`, `shutdown`) relocating into `common/mod.rs`, offset by the new
  regression's 3. No test assertion was removed or weakened.
        - one behavior change worth recording: `no_side_effects`'s diagnostics
          wait moved from a 64-iteration cap panicking `no diagnostics for {uri}`
          to the shared `recv_timeout(10 s)` loop panicking
          `diagnostics before timeout`. Same effective bound, different message.
- `results.md` corrected so it no longer overstates the evidence:
        - the "Resource cleanup" row now names the two proofs separately —
          `child_guard_reaps_process_during_unwind` for the subprocess path and
          `server_worker_finishes_before_workspace_release_during_unwind` for the
          in-memory path — and states that the latter asserts recorded ordering
          rather than absence of a hang
        - the coverage-changes table splits the old "DMLS unwind/cancellation
          ownership" row into a subprocess row and a new in-memory row
        - the AC4 row now states which session shapes are proven, names the
          shared fixture file, the 10 s bound, the detach-on-expiry behavior, and
          the dropck-enforced workspace ordering
- work completed for 'F3 — DMLS server-thread cleanup is neither complete nor bounded' at 11:34:22
- starting the work on 'F4 — the reviewed tree is not an isolated candidate' at 11:35:10
        - handled by the orchestrator: the finding has no code surface, and its
          premise changed between the review and this implementation
        - the finding's premise has changed since the review ran. At review time
          the work was uncommitted; it is now committed as three named commits,
          so a candidate that identifies the exact bytes this fix owns does
          exist.
        - root cause of the reviewer's measurement: the branch
          `fix/cli-slow-tests` carries **three sibling fixes** — the Darkmatter,
          Claudine and Sniff test-performance fixes — plus shared tooling, and
          `detect_changes(scope=all)` covered the whole branch and every
          concurrent agent's uncommitted working tree at once. The 1,306 files
          and CRITICAL risk were correct for what they measured, and they did not
          measure this fix.
        - identified and recorded the owning commit set: `8540d36df` (47 files,
          +6,493/−1,443), `a1746f217` (8 files, +890/−242), `a05e3b747`
          (21 files, +6,199/−5,556), plus the shared `d35b4c23b`/`b0a5142aa`
          tooling commits and six planning/evidence commits. The remaining ~45
          branch commits belong to the sibling fixes.
        - re-ran change detection scoped to the candidate range
          (`detect_changes(scope=compare, base_ref=f0aaa4832)` — the commit
          immediately preceding this fix's first implementation commit):
                - **0 affected execution flows** (review measured 119)
                - risk `low` (review measured CRITICAL)
                - 1,444 changed symbols (review: 8,067), 99 files (review: 1,306)
                - 1,396 of the 1,444 symbols are in `darkmatter/` — 47 files
                  under `cli/`, 20 under `lib/`, 6 under `dmls/`. The residual 35
                  Claudine and 13 Sniff symbols are the concurrent agents'
                  uncommitted work plus `1e7f2fc30`, which the range unavoidably
                  spans; none belong to this fix.
        - zero affected execution flows is the substantive result, and it is
          exactly what the undifferentiated CRITICAL aggregate obscured
- audited the `results.md:22-24` scope claim rather than restating it, and found
  it **accurate in substance but imprecise as written** — corrected it:
        - ten of the 76 candidate files are not under a `tests/` directory
        - `lib/src/layout/page/tests.rs` plus the `#[cfg(test)]` modules inside
          `code_block.rs`, `catalog.rs`, `alias.rs`, `code_renderer.rs` and
          `entrypoints.rs` are test code that happens to live in `src/`;
          `a05e3b747` unindents those inner modules, which is why the diffs look
          large
        - within those same files, every non-test hunk inspected is a
          **line-wrapping reflow with no semantic change**
        - one substantive change does live in `src/`:
          `catalog.rs`'s `capture_shape_matches_projected_type` no longer walks
          the real monorepo for a Git root, it initializes a disposable
          repository with `gix::init` in a temp dir — a hermeticity fix, not a
          production change
        - `Cargo.lock`, two `Cargo.toml` files (test targets/dev-deps) and
          `cli/README.md` round out the set
        - the corrected claim now reads: no production behavior, signature or API
          changed; test code inside `src/` did
        - written up as a new `## Candidate identity` section in `results.md`,
          which `deferred-performance-measurement.md` already links to
- note on method: did not run `just gitnexus` (a full re-index) — it would
  rewrite the shared index while sibling agents are mid-run, and the range-scoped
  query answered the finding without it
- work completed for 'F4 — the reviewed tree is not an isolated candidate' at 11:44:55
- starting the work on 'F5 — the isolation guard permits most protected environment defaults to be undone' at 11:45:38
        - dispatched to a `rust-developer` subagent with the `rust`,
          `rust-testing` and `darkmatter` skills; the three acceptance criteria
          taken from the review (retain intentional claims, route them through a
          named builder override or a narrow reasoned allowlist, add negative
          tests for the home/cache, Git plumbing, rendering and Darkmatter
          application namespaces)
- verified the gap before changing anything, and it was worse than the review
  stated:
        - `isolation_sites()` recognized only four post-`build()` forms
          (`.current_dir(…)`, `.env("PATH", …)`, `.env_remove("PATH")`,
          `.env_clear()`), and its negative test asserted `HOME`, `COLUMNS` and
          `HOMEDRIVE` overrides were ignored
        - a survey found **60 post-`build()` `.env`/`.env_remove` sites across 13
          files**, every one naming a variable the fixture pins — none flagged by
          either gate
- two-class design, in a new shared table `cli/tests/common/protected_env.rs`
  (`ProtectedClass` + `protected_class(key)`). The line is drawn on *what undoing
  it costs*, not on which namespace it belongs to:
        - **Containment** — `HOME`, `HOMEDRIVE`, `HOMEPATH`, `USERPROFILE`,
          `APPDATA`, `LOCALAPPDATA`, `XDG_*`, `TMPDIR`/`TEMP`/`TMP`, and
          `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`/`GIT_COMMON_DIR`/
          `GIT_OBJECT_DIRECTORY`/`GIT_CONFIG_*`. Handing one back re-contaminates
          the child with host state, so the class carries **no** builder method at
          any spelling and the guard always flags it.
        - **Behavior input** — rendering (`COLUMNS`, `LINES`, `TERM`, `COLORTERM`,
          `COLORFGBG`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `NO_COLOR`, `DARK_MODE`,
          `THEME`, `CODE_THEME`, `PREFER_ITALICS`, `TERMINAL_IMAGES`) and
          darkmatter application (`DARKMATTER_*`/`DM_*`/`MD_*`, `AGENT`, `MODEL`,
          `RUST_LOG`, `HASH_PROPERTY`, `HASH_IGNORE_PROPERTIES`,
          `BASELINE_SCHEMA`). A different value here is a real claim about
          behavior, so it must be **declared at construction**, not silently
          re-set after `build()`.
        - `PATH` deliberately stays unclassified so the table cannot open a
          second, weaker route to an escape that already has dedicated vocabulary
          (`host_path`/`fake_only_path`) and two existing guard forms
        - the table is single-sourced: `common/fixture.rs` validates declarations
          against it and `spawn_site_guard.rs` includes the same file via
          `#[path]`, so the guard binary stays light
- named builder methods added to `MdCommandBuilder`, alongside the existing
  `host_path`/`fake_only_path`/`ambient_context`/`inherit_no_env`:
        - `rendering_input` / `rendering_input_removed`
        - `application_input` / `application_input_removed`
        - `plain_terminal(columns, lines)` — the whole
          `COLUMNS`/`LINES`/`TERM=dumb`/`-COLORTERM`/`NO_COLOR=1`/`-FORCE_COLOR`
          frame as one claim; it collapses four byte-identical six-line per-file
          helpers
        - declarations are stored as `EnvironmentOp`s and appended *after* the
          fixture defaults, so a declared value out-ranks both the inherited value
          and the default; both command surfaces get them through the existing
          `ChildEnvironment::apply`
        - each accessor panics on a wrong-class key with a message naming the
          correct method, or stating that containment has no override
- **all 60 call sites migrated with no allowlist entry needed** — every
  intentional claim became a declaration or moved out of a protected namespace:
        - `layout_fill.rs`, `layout_flags.rs`, `compose_layout.rs`,
          `layout_style_frontmatter.rs` — four identical `rendering_command()`
          helpers collapse to `plain_terminal(80, 24)`; 39 call sites untouched
        - `compose_terminal_detection.rs`, `code_block.rs`, `schema_about.rs`,
          `hash_kind_save_diff.rs` (7), `compose_base_schema.rs` (8),
          `hash_directory.rs` (2), `schema_validate.rs`,
          `schema_validate_baseline.rs`, `render_basic.rs`
        - `md_process_fixture.rs`, the fixture's own self-test, handled with
          care: `MD_PROBE_CAPTURE` renamed to `FIXTURE_PROBE_CAPTURE` (43
          occurrences) because it is harness plumbing rather than a darkmatter
          input — that takes it out of the `MD_*` namespace entirely instead of
          allow-listing the file
        - three post-`build()` `.env` sites remain suite-wide, all correct:
          `compose_transclusion.rs`'s `PROJECT_ROOT` (the contract does not pin
          it, so there is nothing to undo) and the two `FIXTURE_PROBE_CAPTURE`
          sites
- guard changes: four new forms, a `first_string_argument()` reader over the
  *original* source (an escaped literal is rejected rather than half-decoded),
  and one new match arm classifying any `.env`/`.env_remove` in method position
  after the two `PATH` arms. The existing four detections are unchanged.
        - no generic exemption added; `ISOLATION_ALLOWLIST` still holds exactly
          its one `md_process_fixture.rs` entry and the census is unchanged at
          `{"files":1,"sites":2,"scanned_sites":2,"governed_files":39}`
- anti-drift test added
  (`every_variable_the_spawn_contract_owns_is_classified_as_protected`): collects
  `inherited_scrub_keys()` plus `ChildEnvironment::set_keys()` and asserts every
  one except `PATH` is classified. This keeps the guard's table honest against
  the fixture's own policy without rewriting the scrub.
- **non-vacuity proven three separate ways**, as required:
        - four namespace negatives, with the new match arm neutered — exactly
          four tests failed, all `assertion left == right failed, left: []`,
          against expected forms
          `".env/.env_remove of a home/config/cache/temp anchor"`,
          `"… of a Git plumbing variable"`,
          `"… of an undeclared rendering input"`,
          `"… of an undeclared application input"`. The control
          `a_declared_input_is_not_an_isolation_escape` correctly stayed green
          under the break.
        - the two accidents the review names specifically, against the **live**
          gate: planting `.env("HOME", host_home)`, `.env("GIT_DIR", checkout)`,
          `.env("COLUMNS", "44")` and
          `.env_remove("DARKMATTER_NO_BASELINE_SCHEMA")` into the real governed
          file `clean.rs` failed
          `migrated_l1_tests_keep_the_isolation_the_builder_gave_them` with all
          four listed by file and line. `clean.rs` restored, gate green.
        - the anti-drift test: deleting `"HOME"` and `"RUST_LOG"` from
          `protected_env.rs` failed with `the spawn contract pins or scrubs
          these, but protected_env.rs does not classify them, so a post-build()
          override of one would pass the isolation gate: ["HOME", "RUST_LOG"]`
- gates, all in the `darkmatter/` package area:
        - `just lint` — **PASS** across all four packages plus the
          `wasm32-wasip2` extension check
        - `just test` — **PASS**, `7755 tests run: 7755 passed, 6 skipped` in 31.9 s
        - `cargo nextest run -p darkmatter-cli --test md_process_fixture --test
          spawn_site_guard` — **PASS**, `44 passed, 0 skipped`, up from the
          review's baseline of 35. The +9 are 5 guard cases (4 namespace
          negatives + the declaration control) and 4 fixture cases (the
          classification anti-drift test and three `#[should_panic]` validator
          tests for containment, wrong-class, and `PATH`).
- pre-existing condition disclosed rather than silently fixed:
  `cargo fmt -p darkmatter-cli -- --check` reports drift in 17 `cli/src/*` and 7
  `cli/tests/*` files, none of them modified in the working tree — the drift is
  committed on this branch and predates this change. `just lint` runs no fmt
  check, so it does not gate. The two spots in `md_process_fixture.rs` that did
  carry drift were hand-corrected; `cargo fmt` was **not** run.
- docs moved with the behavior, per CLAUDE.md's drift rule: `common/fixture.rs`
  module docs gained a *Declared inputs* section, `spawn_site_guard.rs` module
  docs describe the widened gate and the two classes,
  `darkmatter/cli/README.md` § Integration tests documents the declaration
  vocabulary, and `.claude/skills/darkmatter/SKILL.md` § Testing and verification
  does the same
- `results.md` updated: the spawn/isolation guard coverage row 18 → 23 with the
  four-namespace contract described, the fixture-contract row 17 → 21, the
  population total 84 → 93 added identities, and the AC2/AC3 rows restated
- work completed for 'F5 — the isolation guard permits most protected environment defaults to be undone' at 12:03:14
- final consolidated gate over all three code findings together, run by the
  orchestrator rather than trusting the per-subagent runs:
        - `just test` (darkmatter area) — **PASS**,
          `7755 tests run: 7755 passed, 6 skipped` in 32.3 s. Both new
          regressions visible in the tail:
          `dmls::lsp_session server_worker_finishes_before_workspace_release_during_unwind`
          and `dmls::stdio_subprocess child_guard_reaps_process_during_unwind`.
        - `just lint` (darkmatter area) — **PASS**, exit 0, including the
          `wasm32-wasip2` Zed extension check
        - identity accounting reconciles: 7,745 before this cycle, +1 from F3's
          unwind regression, +9 from F5's guard/fixture cases = 7,755

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1 hour and
2 minutes. During this implementation all 5 review findings were evaluated to
see if they could be fixed as a part of this implementation cycle: 3 were fixed
in full, 1 was fixed in part with one component deferred, and 1 was deferred
(see reasons below):

- **F1 — Required CI comparison and ratified budgets do not exist — DEFERRED.**
  The requirement cannot be satisfied from the implementing side at all. A
  *candidate* sample is by definition a hosted CI run of the candidate revision,
  and hosted CI does not run an unpushed revision; pushing is the operator's
  action. Holding readiness on it would make the fix permanently un-mergeable by
  its own author. The operator had already ruled the bar too high
  (`**DECISION:**` in `review-1.md`), and that ruling is now recorded in
  `results.md` as a change of **severity, not of evidence** — no budget was
  invented, no local timing promoted to a target, no leg marked green. The exact
  missing artifacts (1 of 3 baseline samples, 0 of 3 candidate samples, 0
  ratified budgets, 0 matched-identity comparisons, per leg) and the six-step
  closing sequence are in `deferred-performance-measurement.md`.
        - note this is **not** the usual host-CPU-load deferral. The two genuine
          load rejections from earlier in this fix are already retained under
          `measurement/rejected-high-load/` and
          `measurement/rejected-http-regression/`. This one is structural runner
          inaccessibility, and the log records it as such.
- **F2 — Level-3 keyboard, pointer and image-paint behavior was not executed —
  PARTIALLY FIXED; the runtime half DEFERRED.**
        - **Fixed:** the finding's `**CRITICAL:**` annotation, which was the
          operator's actual demand. The synchronization change is now explained
          in both L3 test modules and in a new `results.md` section: fixed
          sleeps became polls of the *same final asserted condition*, which is
          strictly stronger than a sleep (a sleep is simultaneously a flake
          source and a hidden-pass source), and it is mandated by `spec.md` §4
          rather than incidental. The masking trade-off the new bounds introduce
          is stated candidly rather than glossed.
        - **Fixed, and unplanned:** a real defect surfaced while answering the
          "record skips as unavailable, not passes" ask.
          `level3_rich_image_node_paints_distinctive_pixels` had two mid-body
          bare `return`s that `require_level!` cannot reach, so an operator's
          authorized L3 run on a mis-permissioned macOS host would have filed a
          **green pass for a pixel claim it never evaluated**. Both exits now
          route through `skip_pixel_assertion`, which honors
          `BISCUIT_TEST_LEVEL_REQUIRED=3`.
        - **Deferred:** executing the four L3 identities. They require an
          attended macOS host with foreground focus and OS input injection. This
          session is non-interactive and the host is in use by a person; running
          them would seize the desktop, which repo policy forbids. Compile
          reachability was verified with `cargo nextest list` (which does not
          execute) and is explicitly labelled non-behavioral evidence. The four
          identities are recorded as UNAVAILABLE, never as passes, with the exact
          closing invocation prescribed in `results.md`.
- **F3 — DMLS server-thread cleanup is neither complete nor bounded — FIXED.**
  All four of the review's criteria are met, and the ordering criterion is met
  more strongly than asked: rather than documenting a drop-order convention,
  `LspFixture<'workspace>` borrows its `LspWorkspace`, so dropck makes releasing
  a workspace while a session is live a **compile error**.
- **F4 — The reviewed tree is not an isolated candidate — FIXED.** The premise
  had already changed (the work is now three named commits), and the reviewer's
  CRITICAL aggregate turned out to be measuring three sibling fixes plus other
  agents' uncommitted trees at once. Change detection re-run on the isolated
  candidate range reports **0 affected execution flows and `low` risk**, against
  119 and CRITICAL for the undifferentiated tree. The `results.md` scope claim
  was also audited and found imprecise; it is now corrected.
- **F5 — The isolation guard permits most protected environment defaults to be
  undone — FIXED.** The gap was wider than reported: 60 unguarded post-`build()`
  sites across 13 files. All 60 are migrated, no allowlist entry was needed, and
  the guard now enforces a two-class contract with non-vacuity proven three
  separate ways.

The files carrying this cycle's evidence are `results.md` (candidate identity,
the operator ruling, the L3 synchronization section, and corrected AC2/AC3/AC4
rows), `deferred-performance-measurement.md` (new), and this log.

Both package-area gates pass over the combined result: `just test` 7,755 passed
/ 6 skipped, `just lint` exit 0.

One pre-existing condition is disclosed rather than silently repaired:
`cargo fmt -p darkmatter-cli -- --check` reports drift in 24 files that are
unmodified in the working tree, so the drift is committed on this branch and
predates this cycle. `just lint` runs no fmt check, so it does not gate, and
`cargo fmt` was not run.
