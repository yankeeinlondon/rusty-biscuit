---
total_phases: 10
created: 2026-09-07
phase: 1
agent: "opencode/zai-coding-plan/glm-5.3"
yolo: "true"
fix: 2026-09-07-faster-sniff-tests
spec: sniff/fixes/2026-09-07-faster-sniff-tests/spec.md
packages:
    - sniff
    - sniff-cli
---

# Execution plan — Faster Sniff tests with explicit discovery contracts

Converts [spec.md](spec.md) into ten ordered phases. Required behaviors are
referenced as **RB1**–**RB5** (spec §1–§5) and acceptance criteria as
**AC1**–**AC9** (spec §Verification and acceptance, in order).

## Grounding facts (verified on this branch, 2026-09-07)

Checked against the working tree, not assumed. A phase that contradicts one of
them should stop and re-derive rather than proceed.

- **The coordinated production-caching fix has not landed.**
  `sniff/fixes/2026-07-22-inefficient-calling/` contains only its `spec.md`,
  and `sniff/lib/src/hardware/mod.rs` / `os/mod.rs` carry no
  `OnceLock`/`OnceCell`/`Lazy` memoization. Every baseline taken now
  **predates** that fix. The spec's coordination rule is therefore a standing
  protocol (Phase 1), not a phase-1 merge gate as in the claudine sibling plan.
- **Raw `cargo_bin` spawn census** (grep; the guard built in Phase 4 becomes
  the authoritative count):

  | file | sites | note |
  |---|---|---|
  | `cli/tests/cli.rs` | 357 | ~6,700 lines; families listed in Phase 5 |
  | `cli/tests/install_plan.rs` | 9 | |
  | `cli/tests/snapshots.rs` | 4 | owns the `run_stdout` helper |
  | `cli/tests/level2_cicd_styling.rs` | 2 | L2; spawns raw on purpose |
  | `cli/tests/level2_git_status_styling.rs` | 2 | L2; spawns raw on purpose |
  | `cli/tests/tty.rs` | 1 | `expectrl` PTY; needs a raw `Command` |
  | `cli/tests/install_interactive_pty.rs` | 1 | `SNIFF_INTERACTIVE_PTY=1` gate |
  | `cli/tests/install_interview_cli.rs` | 1 | |

  Total ≈ 377. Many `cli.rs` sites already pin `.current_dir(tempdir)` (a grep
  finds 10+ in the repo/JSON families) but every one inherits the full host
  environment; tests without `.current_dir` launch from the package directory
  inside the checkout.
- **`run_isolated_software`** (`cli/tests/cli.rs:6`) pins only `PATH`, `HOME`,
  `ProgramFiles`, `ProgramFiles(x86)`, `LocalAppData` — no launch CWD, no
  `GIT_DIR`/`GIT_WORK_TREE` scrub, no cache/XDG/Windows-home variables, no
  rendering inputs. Its seven consumers are the `software` contract tests.
- **No shared test `common/` module exists** in `sniff/cli/tests/`; helpers are
  inline per file (`run_isolated_software`, snapshots' `run_stdout`). The
  claudine reference shape (`claudine/cli/tests/common/mod.rs` +
  `spawn_site_guard.rs`) is the model to port, not a dependency.
- **Test population is ≈2,280 identities**: ~1,476 embedded in
  `sniff/lib/src`, ~416 across 21 `lib/tests/*.rs` targets (integration 121,
  git_parity 86, remote_providers 71, focused_provider 51, …), ~391 across 8
  `cli/tests/*.rs` files (cli.rs 363, snapshots.rs 14, install_plan.rs 9, …).
  A one-row-per-test hand inventory is not credible; RB1's "one reviewed row
  **or an explicitly enumerated family**" needs mechanical reconciliation.
- **Known dead and bespoke items** (spec-named, verified): `lib/tests/foo.rs`
  is `#[test] fn bar() {}`; `install_interactive_pty.rs` skips unless
  `SNIFF_INTERACTIVE_PTY=1` (no canonical recipe selects it); the eight
  `real_*` tests in `lib/src/package/network.rs` are the only `fn real_`
  definitions in `lib/src` and run only under `just test-real`'s `network`
  feature selection.
- **Runner overrides touching sniff** (`.config/nextest.toml`): the
  `sniff-windows-l1` test-group (CI profile, `cfg(windows)`, `max-threads = 1`)
  covering `package(sniff) + package(sniff-cli)`;
  `test_detect_completes_in_reasonable_time` with `threads-required =
  "num-cpus"`; global `slow-timeout` 5s/terminate-after 6 and the 30s leak
  window. Phase 2 must census any per-test overrides additionally in force.
- **Serialization in tests**: `serial_test` appears in `integration.rs` (18),
  `remote_providers.rs` (16), `program_serialization.rs` (6),
  `remote_observation.rs` (4), `focused_provider.rs` (4), `cli.rs` (8), and
  the two L2 files (2 each). `serial_test` is a no-op across nextest's
  per-test processes, so each use needs a real-shared-resource justification.
- **Readiness sleeps**: 200 ms fixed sleeps at
  `cli/tests/level2_cicd_styling.rs:39` and
  `cli/tests/level2_git_status_styling.rs:44` — the spec's
  replace-with-bounded-observation targets.
- **Existing seams to extend, not invent**: `PerformanceCollector` /
  `with_current_collector` / `counters` (demonstrated by
  `lib/tests/benchmark_workloads.rs` and `lib/tests/integration.rs`); the
  deterministic repository builder at `lib/benches/support/builder.rs` already
  shared via `#[path]` by `git_parity.rs`, `bench_fixtures.rs`, and
  `benchmark_workloads.rs`; wiremock already backs the remote suites
  (`remote_providers.rs`, `remote_observation.rs`).
- **Feature/recipe contract** (`sniff/justfile`): `test` runs
  `sniff --features remote` + `sniff-cli` locally (no `test-fixtures`),
  CI-mode adds `test-fixtures`; `sanity` runs
  `sniff --features remote; sniff-cli --features test-fixtures`;
  `test-real` selects `network` for the lib and bare `sniff-cli`;
  `test-l2` selects `sniff-cli --features test-fixtures`. `lint`, `check`,
  `doctest` all require `remote`.
- **CI metadata**: `sniff/lib` declares `[package.metadata.ci.tests]
  features = ["remote"]`; `sniff-cli` declares tiers `L1`+`L2`, backends
  `tmux`, `features = ["test-fixtures"]`, `local-features = []`.
- **Windows compile authority**: this area has no local mingw check recipe —
  the `windows-latest` CI leg is the compile authority for Windows-only test
  targets (spec AC, verified: no `check-windows` usage in `sniff/justfile`).
- **Ambient-checkout precedent inside `cli.rs`**: the comment above
  `repo_json_succeeds_in_a_shallow_clone` documents that
  `repo_json_stdout_is_exactly_one_json_document` deliberately runs in the
  ambient checkout and failed on shallow-clone runners — evidence that CWD
  pinning is a correctness issue here, not only a cost issue. Intentional
  ambient use must become a *named* escape, not an accident.
- **Branch state**: this fix directory is untracked on `fix/cli-slow-tests`;
  sibling faster-tests plans exist for claudine and darkmatter. Work in this
  area must not assume their phases have landed.

## Assumptions and stated decisions

1. **Baseline before code.** No code change from Phases 4–7 may be committed
   until Phase 1's checkpoint passes; the same warm-run protocol is this fix's
   evidence and its baseline. Document-only Phases 2–3 may proceed during the
   measurement window.
2. **The inventory is family-based with a mechanical completeness proof.** A
   small reconciler script in this fix directory (house convention — the
   claudine predecessor shipped `junit-metrics.ts`) joins
   `cargo nextest list --message-format json` output against the inventory's
   declared families and fails on any unassigned or doubly-assigned identity.
   Hand enumeration of ~2,280 identities is not a credible deliverable.
3. **One fixture, two command surfaces, one policy.** The builder computes an
   environment description (clear flag, ordered removes, ordered sets,
   `current_dir`, `PATH` value) applied by thin adapters to
   `assert_cmd::Command` and `std::process::Command` — the latter because
   `tty.rs` and the interactive-PTY flow drive `expectrl`, which needs a raw
   `Command`. A drift test proves both surfaces produce the same effective
   environment.
4. **`run_isolated_software` is grown into the builder or becomes one named
   escape** — it must not survive as a parallel convention (spec §2).
5. **Scope discipline on commits.** Assertion repair, fixture migration, and
   guard changes land in separately reviewable commits; a reviewer can read
   any one without the others.
6. **`inventory.md` carries the baseline table, budgets beside it, and
   dispositions; `results.md` carries measurements, coverage deltas, and the
   three separate completion claims** (implemented / verified-locally /
   verified-on-CI). Both live in this fix directory.
7. **No new override, retry, tier change, or disabled assertion may be
   introduced at any point.** `git diff main -- .config/nextest.toml` is
   inspected at every checkpoint and must show removals/justifications only.
8. **Coordination protocol with 2026-07-22-inefficient-calling**: every
   measurement records whether its revision predates or includes that fix; if
   it lands during this work, affected cohorts are re-baselined at the new
   revision and **no timing comparison spans the landing**.

---

## Phase 1 — Baseline, metrics tooling, and the coordination window (RB5)

Opens the attribution window. Everything downstream measures against this.

- [ ] Record the baseline state: branch, revision SHA, dirty files,
      toolchain (`rustc -vV`), host platform, and the explicit statement that
      the revision **predates** 2026-07-22-inefficient-calling (verified: no
      memoization in `sniff/lib/src/hardware/` or `os/`).
- [ ] Confirm `git diff main -- .config/nextest.toml` and record the
      overrides in force for sniff cohorts (the `sniff-windows-l1` group, the
      `test_detect_completes_in_reasonable_time` threads-required override,
      global slow-timeout/leak windows, plus any per-test overrides the
      census finds).
- [ ] Build the metrics tool in this fix directory (fork the claudine
      predecessor's `junit-metrics.ts` convention): reads nextest JUnit
      artifacts and emits the baseline table with **build/setup, runner
      elapsed, and summed test duration as three separate columns**. It must
      reject malformed reports, missing expected artifacts or tests, duplicate
      identities, invalid durations, and failed runs — a script that prints a
      miss and exits 0 is not a gate.
- [ ] Warm the revision's build artifacts, then collect **five alternating
      warm local runs** (spec §5) of, at minimum: the full local L1 suite
      (`just test` from `sniff/`), the `just sanity` cohort (its duration is
      reported against the 15-second fast-confidence budget), and the CLI
      integration cohort (`sniff-cli` package tests). Alternate rather than
      batch so page-cache warming cannot masquerade as a trend.
- [ ] Capture per-test durations via nextest's JUnit output for the same runs
      and store everything under
      `sniff/fixes/2026-09-07-faster-sniff-tests/baseline/<run-id>/`.
- [ ] Record test identities/counts, failures, skips, timeouts, retries, and
      slow cases (`slow-timeout` 5 s marks) next to the timings so lost
      coverage can never read as an optimization.
- [ ] Pair one representative cohort with work-counter readings
      (`PerformanceCollector` snapshot via the `work_counts` example or an
      equivalent bench entry point) so timing claims have a counter baseline.
- [ ] Write the standing coordination note into this fix's `log`/inventory:
      the rule that no timing comparison may span a landing of
      2026-07-22-inefficient-calling, and the re-baseline obligation if it
      lands mid-work.
- [ ] Run `just test-l2` from `sniff/` once and record the L2 pair's baseline
      (tmux backend) so Phase 7's sleep replacement has a before measurement.

**Validation checkpoint 1** — five alternating warm runs exist per cohort with
artifacts stored; the metrics tool reproduces the baseline table and fails on
a deliberately malformed report (proved once, transcript recorded); the
coordination note exists; `just sanity` baseline duration is recorded against
the 15-second budget.

---

## Phase 2 — Reconciled inventory (RB1, AC1) ‖ runs during Phase 1's window

Document-only. No source file changes. Produces `inventory.md`.

- [ ] Build the enumeration substrate: capture
      `cargo nextest list --message-format json` for both packages under
      every feature selection a canonical recipe uses — bare, `remote`,
      `network`, `test-fixtures`, `remote,test-fixtures` — recording command,
      revision, toolchain, and features beside each capture.
- [ ] Capture the source-side population separately (attribute scan for
      `#[test]`, `#[tokio::test]`, `#[rstest]`, plus `#[ignore]` and `#[cfg]`
      gates) and diff against runner discovery. Every source test the runner
      never lists under any recipe is a **cfg/feature exclusion row** with its
      reason and actual execution route. This is where `network`-vs-`remote`
      reachability (spec §1) is proven, not assumed.
- [ ] Inventory the embedded unit tests explicitly, including the eight
      `real_*` tests in `lib/src/package/network.rs` and any other embedded
      gated tests the diff surfaces — `tests/` targets are not the whole
      population.
- [ ] Inventory doctests (`just doctest`) and the `lib/benches` entry points
      (the `perf` bench plus its `support/` modules); confirm the area has no
      fuzz targets and record that as a finding rather than an omission.
- [ ] Inventory shared fixture machinery as first-class rows:
      `run_isolated_software` (cli.rs), `run_stdout` (snapshots.rs),
      `lib/tests/fixtures.rs`, `lib/tests/fixtures/remote/`,
      `lib/benches/support/{builder,fixtures,plans,network_fixture,
      remote_report_fixture,bench_ids,util}.rs`, and the
      `render_cicd_fixture` test-fixtures helper binary.
- [ ] Write the per-family rows. Each records: the behavior proved and whether
      assertions distinguish a plausible failure; the three-purpose split
      (deterministic / native-detector / external-resource); required
      boundary and inputs (CWD, home/config/cache, environment, repository,
      installed tools, network, terminal); setup cost, waits, ownership and
      cleanup; timing floor; runner overrides in force; tier, features,
      platforms; canonical recipe; observed cost with provenance; disposition
      (satisfactory / remediation in this fix / linked follow-up).
- [ ] Enumerate family membership explicitly. A family row is valid only when
      its members are listed and share setup and proof; anything that differs
      gets its own row. `cli.rs`'s 363 tests split along its section headers
      (repo-JSON aggregates, repo leaf e2e, terminal-subset, section
      subcommands, hardware/filesystem detail, software, test-runner, repo
      version, negative, services, scoped enrichment, verbose, remote,
      install, plain, blast-radius, recent-commits ×3, repo packages,
      package-areas, stable-JSON shape, help/version/completions/output-mode/
      flag-position).
- [ ] Classify OS specificity per spec §1: basic native OS API coverage stays
      ordinary cfg-gated L1; document every proposed tier change with the
      actual resource that motivates it.
- [ ] Write the runner-override census as its own table: every per-test and
      per-group override touching sniff packages, each marked *justified*
      (naming the contract its floor expresses) or *remove with the cost it
      hides*. Include the `sniff-windows-l1` CI serialization group — its
      comment claims Windows host-network API fail-fast; that claim gets
      evidence or a follow-up, never a silent change.
- [ ] Record recipe/feature reconciliation findings: `test-real` selecting
      `network` (does any real-gated test require `remote`?); `sanity`
      enabling `test-fixtures` for `sniff-cli` while local `test` leaves it
      off; the `SNIFF_INTERACTIVE_PTY=1` bespoke gate resolving to a route or
      an unreachable-with-reason row; `foo.rs` as a dead test binary.
- [ ] Implement the completeness reconciler in this fix directory. It reads
      the nextest listings plus the inventory's declared families and **fails**
      on any identity assigned to zero or more than one row. Run it and paste
      its output into `inventory.md`.
- [ ] State the disposition of every row. No row may be dispositioned by
      timing threshold; no exclusions based on historical speed (AC1).

**Validation checkpoint 2** — the reconciler exits 0; the inventory covers
both packages, all tiers, embedded unit tests, doctests, benches, excluded and
ignored tests, and shared fixtures; every row has a three-purpose split and a
disposition; the override census and recipe findings are complete.

---

## Phase 3 — Attribution and ratified budgets (RB5 first half; resolves the spec's five draft decisions)

Document-only. Depends on Phases 1 and 2.

- [ ] Attribute cost by family against the Phase 1 baseline, keeping build,
      elapsed, and summed-duration columns separate, and pairing each suspect
      family with work-counter readings. Answer draft decision 1 — *which host
      observations are intentional coverage and which are accidental* — with
      per-family counter deltas (e.g. `GIT_DISCOVERIES`, `FS_WALK_*` fires
      when a CLI test launches from the package directory).
- [ ] Answer draft decision 2 — *which existing observation/tool seams are
      sufficient for deterministic tests* — by mapping each deterministic
      family to the seam that already proves it (captured observations,
      focused requests, wiremock fixtures, builder repositories) and naming
      the gaps that justify new seams in Phase 4.
- [ ] Answer draft decision 3 — *which feature/tier combinations currently
      leave tests unreachable* — from Phase 2's exclusion rows, verifying the
      `test-real`/`network`/`remote` selection explicitly (spec §1: `network`
      alone must not be mistaken for `remote` coverage).
- [ ] Answer draft decision 4 — *which bespoke gates become tiered or
      recipe-routed and which are removed* — with a per-item decision:
      `SNIFF_INTERACTIVE_PTY` (PTY resource ⇒ canonical L2 route or removal
      with reason), `foo.rs` (dead ⇒ removal), and any others surfaced.
- [ ] Measure the native-detector cohort separately (real OS/hardware/GPU
      detector tests) and keep it separate throughout (spec §5: never compare
      a real detector before with a fake projection afterward). Answer draft
      decision 5 — *what cost and concurrency budgets are justified per
      native platform* — including the Windows L1 group: gather evidence for
      the `sniff-windows-l1` serialization's stated host-network fail-fast
      rationale before proposing any concurrency change (spec §4).
- [ ] Ratify per-family work and timing budgets **after** attribution, written
      into `inventory.md` beside the baseline table so budget review and
      evidence review are one act. No universal percentage target (spec §5).
      Local timing attributes cost; it does not set a CI target.
- [ ] Record the acquisition-vs-execution accounting plan for families whose
      collection boundaries differ, and the collector-propagation checks
      (threads, Rayon, walker workers) required before any lower counter is
      read as less work (spec §3; the performance skill's interpretation trap).
- [ ] For any production defect surfaced during attribution, open a linked
      issue/spec with the evidence rather than fixing it here.

**Validation checkpoint 3** — all five draft decisions have written answers
backed by measurements or reconciler output; budgets live in `inventory.md`
next to the baseline; the native-detector cohort is separately measured; the
Windows concurrency question has evidence or an explicit no-change decision.

---

## Phase 4 — One fixture, one spawn: the CLI command builder and guard (RB2 infrastructure)

First code phase. Requires checkpoint 1. Everything in Phases 5–7 that spawns
the `sniff` binary depends on this, so it lands alone.

- [ ] Create `sniff/cli/tests/common/mod.rs` with a `SniffCliFixture`
      (naming to taste, modeled on claudine's `CliProcessFixture`): temp
      `cwd`/`home`/`bin` outside the checkout, rejection of fixture roots
      inside the checkout **including after canonicalization**, and a builder
      returning the one supported `assert_cmd::Command` with `current_dir`
      pinned to the fixture cwd.
- [ ] Implement the environment policy as a computed description — clear/scrub
      flag, ordered removes (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` and
      related plumbing; `HOMEDRIVE`/`HOMEPATH`/`XDG_CONFIG_HOME` and friends;
      rendering inputs such as width/color forcing; application variables),
      ordered sets (home/config/cache variables directed into the fixture,
      `PATH`) — applied by thin adapters to both `assert_cmd::Command` and
      `std::process::Command`. Scrub inherited values **before** applying
      intentional overrides so a per-key `.env` after build still wins.
- [ ] Default `PATH` is the fixture `bin` plus a minimal system set
      (`/usr/bin:/bin` on Unix; `%SystemRoot%\System32` with `PATHEXT`
      untouched on Windows, restoring `SystemRoot`/`COMSPEC` where stubs need
      them) — bounded lookup, not fake-only, per the rust-testing contract.
- [ ] Provide the named escapes, each requiring a call-site comment naming
      the tool or the proof it needs: `host_path()` (real tool is the
      subject), `fake_only_path()` (absence is the assertion),
      `ambient_context(dir)` (launch CWD pinned to a repository the test
      built, panicking outside it — the route for the ambient-checkout
      aggregate tests `cli.rs` documents).
- [ ] Grow `run_isolated_software` into the builder (a named software-contract
      preset) or convert it to one named escape. It must not survive as a
      parallel convention (spec §2).
- [ ] Add the raw-command surface (`command_std()` or builder equivalent)
      carrying the identical policy, for `tty.rs`'s `expectrl::spawn` and the
      interactive-PTY flow which need `std::process::Command`.
- [ ] Add a drift test proving the two surfaces produce the same effective
      environment (recording stub through both paths, comparing captured
      key/value sets; Windows arm compiled everywhere via `cfg!(windows)`).
- [ ] Create `sniff/cli/tests/spawn_site_guard.rs` on claudine's mechanics:
      source scan for raw `Command::cargo_bin("sniff")` /
      `cargo::cargo_bin("sniff")` / `bin_exe!` spawns outside the builder,
      with an **explicit `(file, one-line reason)` allowlist whose stale
      entries fail**, comment/string sanitization before searching, and
      negative detector tests (prose mention, string literal, a neutered
      detector shown to fail then restored — transcript recorded).
      `level2_*` and `real_*` files are out of the scan: they drive real
      terminals and host tooling on purpose.
- [ ] Seed the allowlist with every current raw-spawn file (Phase 5 burns it
      down); the guard census prints and writes a JSONL artifact to the
      staging directory on the claudine pattern.
- [ ] Harden parent-side helper commands too: audit `Command::new("git")` and
      any other parent-side spawns in the test helpers for inherited Git
      plumbing — isolating only the `sniff` child does not protect those
      (spec §2; see `cli.rs`'s `git()` helper inside the shallow-clone test).
- [ ] Windows compile authority: no local mingw recipe exists in this area,
      so record `windows-latest` CI as the authority for Windows-only arms
      and mark local Windows verification pending where the host cannot run
      it (spec AC).

**Validation checkpoint 4** — `just test` and `just lint` green from `sniff/`;
the drift test passes; the guard's census matches the Phase 1 spawn count
(± sites the guard legitimately excludes); negative guard tests prove both
the violation arm and the stale-entry arm; `git diff main --
.config/nextest.toml` empty.

---

## Phase 5 — CLI spawn burn-down (RB2, AC2, AC5, AC7)

Three migration batches plus closure. **5A, 5B, and 5C touch disjoint file
sets and are mutually parallelizable**; each ends by deleting its own
allowlist entries, and the guard's stale-entry arm catches a mis-merge.

### Phase 5A — `cli.rs` repository-bearing families (‖ with 5B, 5C)

- [ ] Migrate the repo/git-bearing families in `cli/tests/cli.rs` — the
      `repo --json` aggregate tests, repo leaf end-to-end, terminal-subset,
      repo language/version, blast-radius temp-repo, recent-commits (all
      three sub-families), repo packages, package-areas, and stable-JSON
      shape sections — to the builder, each test's disposable repository
      living inside its own fixture root outside the checkout.
- [ ] Decide per ambient-checkout test: the deliberately-ambient aggregate
      documented at `repo_json_stdout_is_exactly_one_json_document` moves to
      `ambient_context()` on a test-built repository (its shallow-clone
      sibling already proves why ambience is a correctness hazard), or gets a
      named-escape entry with the reason. No silent ambience remains.
- [ ] Delete these files' allowlist entries; confirm the stale-entry arm
      would fire if one were left behind.

### Phase 5B — `cli.rs` contract families + satellite files (‖ with 5A, 5C)

- [ ] Migrate the remaining `cli.rs` families — help/version, completions,
      output mode, global flag position, section subcommands, hardware/filesystem
      detail, software (the `run_isolated_software` consumers), test-runner,
      negative, services, scoped enrichment, verbose, invalid subcommand,
      remote, install, plain — plus `snapshots.rs` (`run_stdout`), and
      `install_interview_cli.rs` to the builder.
- [ ] Migrate `install_plan.rs`'s nine sites; its stub-tool and install-root
      fixtures move into fixture-owned `bin`/roots so platform-specific
      software search roots cannot expose the host roster (spec §2).
- [ ] Delete this batch's allowlist entries.

### Phase 5C — raw-command cohort + bespoke gates (‖ with 5A, 5B)

- [ ] Route `tty.rs` through the builder's raw-command surface so the
      `expectrl` session inherits the policy; keep the `#[cfg(unix)]` gate.
- [ ] Resolve `install_interactive_pty.rs` per Phase 3's decision: route it
      through a canonical recipe (an L2 tier with `require_level!` + a real
      backend, named for the PTY resource it needs) or remove it with the
      recorded reason. The `SNIFF_INTERACTIVE_PTY=1` bespoke gate must not
      survive silently unreachable (AC7).
- [ ] Delete `lib/tests/foo.rs` (dead placeholder binary) and remove any
      recipe/reference drift it leaves (AC7).
- [ ] Delete this batch's allowlist entries.

### Phase 5D — burn-down closure

- [ ] Assert the allowlist contains **zero generic fixture-migration
      exemptions** (AC5). Any survivor carries a specific technical
      necessity — e.g. the L2 pair, which the guard excludes by tier — plus
      equivalent isolation/ownership evidence written at the entry.
- [ ] Confirm the two L2 files' raw spawns are covered by their tier
      exclusion (not an exemption), and that `install_interactive_pty`'s
      resolution is recorded either way.
- [ ] Add the contamination probes AC-adjacent checks require, using
      **disposable state only** — never an edit to the real checkout or the
      user's configuration: Git plumbing (`GIT_DIR`/`GIT_WORK_TREE` pointed
      at a throwaway repo), home/cache relocation, `PATH` injection of a
      shadowing fake tool, inherited width/color (`COLUMNS=44`,
      `FORCE_COLOR=1`), and a checkout-ancestor `TMPDIR`. Each probe must
      leave unrelated test results unchanged.
- [ ] Re-run `just test` from `sniff/` with the probes exported and record
      that no deterministic test changed its result.

**Validation checkpoint 5** — `just test`, `just lint`, `just doctest` green
from `sniff/`; the guard census shows zero generic exemptions with every
exclusion tier-named; contamination probes pass; test count reconciles
(additions and removals reported separately, never netted); `just sanity`
still green.

---

## Phase 6 — Bound and prove requested work in the library (RB3, AC3)

Touches `lib/tests/` and embedded unit tests; largely disjoint from Phase 5's
CLI files, so **partially parallelizable with Phase 5** after Phase 4 lands
(serialize only where a commit would mix both).

- [ ] Walk the deterministic lib families (`integration.rs`,
      `focused_provider.rs`, `remote_providers.rs`, `remote_observation.rs`,
      `remote_resolution.rs`, `host_capability_cache.rs`,
      `merge_conflict_prediction.rs`, `uv_with_install_plan.rs`,
      `program_*`, `windows_*`) substituting focused requests and captured
      observations where full detection is incidental. Tests of aggregate
      behavior keep aggregate execution; do not narrow a request that is
      itself under test (spec §3).
- [ ] Extend the counter-contract pattern from
      `benchmark_workloads.rs`/`integration.rs`: absent work is zero; seeded
      execution does not rediscover Git (`GIT_DISCOVERIES` stays at its
      seeded count); projection does not reacquire observations; acquisition
      and execution accounted separately where collection boundaries differ
      (the seeded-observation accounting in the performance skill is the
      reference).
- [ ] Verify collector propagation across threads, Rayon, and walker workers
      before interpreting any lower count as less work — the skill's stated
      interpretation trap. Record the verification method per family.
- [ ] Audit repeated repository construction: identify tests that rebuild
      equivalent repositories per case, then share immutable data or shrink to
      representative fixtures via the `lib/benches/support/builder.rs` seam
      (the `#[path]` pattern `git_parity.rs`/`bench_fixtures.rs` already
      use). Remember process-local caches share nothing across nextest test
      processes (spec §3).
- [ ] Audit the benchmark-fixture tests (`bench_fixtures.rs`,
      `benchmark_workloads.rs`, `bench_ids_sync.rs`, `bench_plans.rs`) for
      fixture sizes larger than their proof needs; keep separate scaling
      coverage where large topologies or real process behavior are the
      contract (spec §3).
- [ ] Review snapshot normalization: volatile values may be removed, but the
      identity, selection, ordering, and error behavior under assertion may
      not be erased (spec §3). Record every change.
- [ ] Repair weak assertions (exit-success-only checks, value-compared-with-
      itself) in separately reviewable commits; for each, record the original
      failing input where one exists, the defect the old assertion could not
      distinguish, and the additional failure the replacement detects.
      Record every replaced or removed test's coverage mapping (AC-adjacent).
- [ ] Apply Phase 3's dispositions for any lib families marked
      remediation-in-scope; linked follow-ups get their owner documents
      updated instead.

**Validation checkpoint 6** — `just test`, `just doctest` green from `sniff/`
with `remote` coverage preserved (spec AC); every eliminated-work claim has a
counter assertion behind it, not a timing note; collector-propagation
verification recorded; test-population changes each carry a coverage mapping;
`git diff main -- .config/nextest.toml` unchanged.

---

## Phase 7 — Bounded external effects, cleanup, and waits (RB4, AC4) ‖ with Phase 6

- [ ] Verify the remote-provider deterministic tests run against local
      controlled servers only (wiremock fixtures in `remote_providers.rs`,
      `remote_observation.rs`, `focused_provider.rs`): assert request counts,
      pagination bounds, and typed failures; retain exact-host
      consent/credential-scope behavior; confirm no deterministic test makes
      a hidden live API request (spec §4). Genuine probes stay `real_` under
      `just test-real`.
- [ ] Audit every subprocess-owning test for drain-while-waiting, deadlines,
      and reaping on error and cancellation — including the PTY sessions
      (`tty.rs`, the routed interactive-PTY test) and any `Command::new` in
      fixtures. Fixtures own and clean up children, threads, sockets, and
      directories on failure too.
- [ ] Replace the two 200 ms readiness sleeps
      (`level2_cicd_styling.rs:39`, `level2_git_status_styling.rs:44`) with
      bounded condition/protocol observation polling on the **final asserted
      content**, with deadlines (spec §4; the rust-testing two-phase
      capture-race note governs the polling shape).
- [ ] Preserve real termination paths and semantic timing floors for
      timeout-behavior tests; justify each retained floor's budget, polling
      cadence, and shutdown margin in the inventory row.
- [ ] Ensure unique sockets/ports/directories for server-owning tests and
      shutdown/join of local server workers (wiremock guards are scoped).
- [ ] Audit serialization: keep runner-visible serialization only for
      actually-shared resources; remove `#[serial]` where the resource is
      per-test, remembering `serial_test` is a no-op across nextest processes
      — genuinely shared state needs runner-visible coordination or a
      test-group. Apply Phase 3's Windows decision for `sniff-windows-l1`
      (evidence-based change, explicit no-change, or follow-up — never a
      silent change).
- [ ] Prove cleanup for process-owning cohorts with nextest's per-test leak
      policy plus the root `just test-leaks` post-run sweep — inspection
      alone is not proof (spec §4).
- [ ] Run L2 coverage only through `just test-l2` (canonical recipe, shared
      pane broker, no focus changes); verify the L2 pair still passes after
      the sleep replacement.

**Validation checkpoint 7** — `just test` green from `sniff/`; `just test-l2`
green; root `just test-leaks` reports no survivors attributable to sniff
cohorts; no readiness sleep remains that is not itself a timeout contract
with a justified floor; the serialization audit is written with per-use
justifications.

---

## Phase 8 — Local candidate measurement (RB5, first evidence tranche)

Requires Phases 5–7 complete and green.

- [ ] Warm the candidate revision's artifacts, then collect **five
      alternating warm local runs** of every changed cohort and the full
      local L1 suite, alternating baseline-revision and candidate-revision
      runs on the same host with matching toolchain, features, profile,
      concurrency, and fixture inputs. Record revision, dirty state,
      platform, cache state, and the exact commands.
- [ ] Re-run every changed timeout or concurrency case (the L2 pair, any
      test whose wait was replaced) **ten times under representative suite
      load**, not in isolation.
- [ ] Measure any cold-build claim in an isolated build directory — never by
      clearing the developer's working cache.
- [ ] Re-collect the work-counter readings paired in Phase 1 and show the
      counter deltas independently of timing: a timing improvement is not
      evidence that a walk was removed, and a lower counter without
      propagation verification is not less work.
- [ ] Keep the three costs separate in every table (build/setup, runner
      elapsed, summed test duration) and track identities, counts, failures,
      skips, timeouts, and slow cases alongside speed.
- [ ] Measure and report `just sanity` duration against the 15-second
      fast-confidence budget (spec AC) — measured, not assumed.
- [ ] Record local numbers as **attribution only**; they establish no CI
      target (spec §5).

**Validation checkpoint 8** — five alternating runs exist per changed cohort
on both revisions; ten reruns exist per changed wait/concurrency case; every
eliminated-work claim has a counter or sentinel behind it; the sanity budget
comparison is recorded; no comparison spans a landing of
2026-07-22-inefficient-calling (re-baseline if it landed — see Phase 1's
protocol).

---

## Phase 9 — CI evidence (RB5 second tranche; AC6, AC8)

Human-gated: push and read.

- [ ] Run `just ci-local --lint-only` then `just ci-local` at the repo root
      for the branch's affected scope before pushing (repo pre-push
      discipline); record the selected scope and commands.
- [ ] Push and collect **three consecutive green candidate CI runs for each
      configured sniff package/environment leg** (the dependency-aware
      workflow fans from `[package.metadata.ci.tests]`: lib `remote` L1; cli
      L1 + L2/tmux). Record every intervening failed attempt and its cause —
      selecting only successful attempts is disallowed.
- [ ] Treat the `windows-latest` leg as the compile authority for
      Windows-only test targets (spec AC; this area has no local mingw
      recipe) and record its results explicitly; native Windows and WSL are
      distinct environments and are never compared to each other.
- [ ] Compare **matched tests within each environment** against that
      environment's own Phase 1-compatible baseline via the metrics tool as
      gate; additions, removals, and platform exclusions are reported
      separately, never netted; missing execution evidence stays **pending**
      and cannot be satisfied by a feature-disabled run.
- [ ] Compare against the Phase 3 budgets: report misses and their causes;
      do not invent a universal speedup percentage and do not close a miss by
      adjusting the budget after the fact.
- [ ] Run the affected L2 tests via `just test-l2` where the tmux resource
      exists; run `just test-real` only for relevant available resources
      **after verifying the required features** (spec AC: confirm the
      `network`-selection reachability question Phase 2/3 answered). Record
      unavailable runtime evidence as pending, not passing.

**Validation checkpoint 9** — three consecutive green runs exist per
configured leg with failures disclosed; every budget is met or its miss is
explained with cause; no override, retry, tier change, or disabled assertion
was used to reach a number; `git diff main -- .config/nextest.toml` shows
removals/justifications only.

---

## Phase 10 — Closure: `results.md`, drift, acceptance sweep

- [ ] Write `results.md` in this fix directory with: the measurements
      (baseline and candidate, three costs separate, per leg); coverage
      changes (tests added, removed, moved boundary, replacement coverage for
      each changed assertion); work-count evidence; ratified budget
      comparison; failures/skips; local and CI results; linked deferred
      findings; and **separate** implemented / verified-locally /
      verified-on-CI completion claims.
- [ ] Give every deferred finding evidence, a reason, and a linked owner
      document. Generic fixture-migration exemptions may **not** be deferred
      (AC5).
- [ ] Update the Sniff skill and area docs **only where fixture or workflow
      contracts changed** — candidates: `.claude/skills/sniff/`
      (performance.md's collector guidance if propagation contracts moved),
      `sniff/docs/`, `sniff/just.md` if recipes changed. Shared production
      changes require a separate scope and downstream impact review (spec AC;
      `CLAUDE.md` § Drift Maintenance governs).
- [ ] Sweep the acceptance criteria explicitly, one subsection each:
  - [ ] **AC1** — every test/family has an evaluated purpose, disposition,
        and real execution route; no exclusions based on historical speed
        (reconciler output attached).
  - [ ] **AC2** — deterministic CLI/repository tests inherit no accidental
        checkout, user configuration, software roster, or Git plumbing;
        named host-discovery tests retain native behavior and
        platform-appropriate assertions (guard census + probes attached).
  - [ ] **AC3** — request/work-count contracts prove eliminated incidental
        work, with collection boundaries and worker propagation verified.
  - [ ] **AC4** — remote and subprocess fixtures are bounded and cleaned up
        on missing interaction, failure, and cancellation; no focus changes
        or hidden live API requests in deterministic tests (`test-leaks`
        evidence attached).
  - [ ] **AC5** — generic fixture-migration exemptions eliminated; technical
        exceptions carry specific reasons and equivalent isolation/ownership
        evidence.
  - [ ] **AC6** — `just test`, `just check`, `just lint`, `just doctest`
        preserve `remote` coverage; affected L2 ran via `just test-l2`;
        `just sanity` green with measured duration against the 15-second
        budget; Windows evidence via `windows-latest` with the same feature
        contract.
  - [ ] **AC7** — bespoke environment gates and placeholder test binaries
        removed or canonically routed; none silently unreachable.
  - [ ] **AC8** — `results.md` complete: coverage changes, work counts,
        ratified budgets, failures/skips, local/CI results, linked deferrals;
        no reduced coverage, new retries, or timeout-limit increases
        substituted for optimization; process-owning cohorts show clean
        `just test-leaks` sweeps.
  - [ ] **AC9** — Sniff skill and area docs updated where contracts changed;
        shared production changes routed to a separate scope.
- [ ] Final gate run from the `sniff/` package area: `just sanity`,
      `just lint`, `just check`, `just doctest`, `just test`, `just test-l2`;
      plus root `just test-leaks` and `just check-tier-coverage` if any tier
      marker or recipe changed.
- [ ] Confirm `git diff main -- .config/nextest.toml` contains
      removals/justifications only, and that every surviving override's
      justification is written in the inventory.
- [ ] Move this fix directory to `_completed/` per the area's lifecycle
      convention once review signs off.

**Validation checkpoint 10** — all nine acceptance criteria are answered with
evidence or an explicitly linked deferral; `results.md` keeps the three
completion claims separate; no gate was weakened to close a criterion; the
coordination note records whether 2026-07-22-inefficient-calling landed and
that no comparison spans it.

---

## Parallelism map

| Can run concurrently | Why it is safe |
|---|---|
| Phase 2 ‖ Phase 1's measurement window | Phase 2 is document-only; it lands no code, so it cannot contaminate the baseline. |
| Phase 3 ‖ late Phase 1 | Document-only; consumes Phase 1's artifacts as they land, needs both complete at its checkpoint. |
| Phase 5A ‖ 5B ‖ 5C | Disjoint file sets, each deleting only its own allowlist entries; the guard's stale-entry arm catches a mis-merge. |
| Phase 6 ‖ Phase 7 | Different concerns (request bounding vs. effects/cleanup) and largely different files; serialize only where both touch one binary. |
| Phase 6/7 ‖ Phase 5 (post-Phase-4) | Lib-side vs. CLI-side files; serialize only where a commit would mix scopes. |

**Strictly serial**: Phase 1 → Phase 4 (no code lands before the baseline
window closes); Phase 4 → Phase 5 (the builder must exist); Phases 5–7 →
Phase 8 → Phase 9 → Phase 10 (evidence follows the change it measures).

## Dependency order (summary)

```text
1 ──┬─→ 4 ─→ 5A ─┐
    │        5B ─┼→ 5D ─┬→ 8 ─→ 9 ─→ 10
    │        5C ─┘      │
    └─→ 2 ─→ 3 ─────────┴→ 6 ‖ 7 ──┘
```

## Out of scope (guard rails)

Named so a phase does not quietly widen. Anything here that turns out to be
necessary becomes a linked follow-up, not an in-flight expansion:

- production detector redesign, process-wide caching (that is
  2026-07-22-inefficient-calling's scope — coordinate, don't duplicate), new
  remote integrations, changed request semantics, new platform support, or
  global CI matrix changes;
- weakening the spawn guard or dropping stale-entry failure to resolve a
  false positive — the sanctioned resolution is a tier exclusion or an
  allow-list entry naming the specific necessity;
- any new `slow-timeout` override, retry, tier change, or disabled assertion
  as a substitute for optimization;
- raising the leak window or any timeout limit to make an ownership defect
  disappear;
- moving OS-specific tests to L2/L3 because of their OS alone, or dropping
  Windows coverage to close an exemption;
- fixing a production defect found during attribution in place of filing it
  with linked evidence;
- a universal percentage speedup target — budgets are per-family and ratified
  only after attribution.
