---
title: "Spike 4 results — can the sniff area-drift contract run on a hosted runner?"
created: 2026-09-15
status: complete
source: reviews/2026-09-15-python-test-code/review.md §1.2
decides: where AC15 is enforced, and whether the cheap sniff query is acceptable
decision: "Not in `ci-tooling`: a release `sniff-cli` is 4m25s cold against a 2-minute budget, so the contracts move to a new `area-drift` workflow (daily, on demand, and on any `sniff/**` pull request) that builds the binary once and sets `BISCUIT_REQUIRE_SNIFF` so absence fails instead of skipping; query semantics stay **A**, because `SNIFF_SELF_INCONSISTENT` re-measures non-empty."
---

# Spike 4 results — Can the sniff area-drift contract run on a hosted runner?

## Results to record

| Measurement | Value |
|---|---|
| `cargo build -p sniff-cli --release`, warm | **99.5s** (1m39s). Warm = `Swatinem/rust-cache`'s real shape: third-party deps and registry cached, the **11 workspace crates** in the closure rebuilt (`biscuit-file`, `biscuit-hash`, `biscuit-terminal`, `biscuit-visualized`, `darkmatter`, `renderable`, `schematic-define`, `schematic-definitions`, `schematic-schema`, `sniff`, `sniff-cli`). A no-op rebuild with those artifacts kept is 0.7s, but `rust-cache` never restores them, so 0.7s is not a CI number. |
| …cold | **264.6s** (4m25s), 556 crates, 1.7G target directory, 493M downloaded into a fresh `CARGO_HOME`, 54M binary. 16-core M4, `CARGO_BUILD_JOBS=4`, host compiler cache bypassed. |
| Option A wall/sys at `max_workers` 1 / 4 / 8 | **10.9s / 7.1s / 6.2s wall**; **69s / 74s / 70s system**; 16-way adds nothing (6.1s). Serial is 4.7s slower and saves no system time — the pool is **not** the cost driver. |
| `ci-tooling` wall before | **5m54s – 6m43s** on `ubuntu-latest`, six real runs where the job actually executed (GitHub run ids 34718327823, 34721727500, 34737848192, 34745972791, 34763514018, 34765185022; median ≈ 6m10s). |
| `ci-tooling` wall after | Not shipped, so not measured. **Estimate: 12.5–18 min** cold (6m10s + a 6.5–11.5 min hosted build, extrapolated below) — against the job's own `timeout-minutes: 20`. With a `rust-cache` step added: **estimate 9–11 min**. |
| Query semantics ruling | **A** (keep per-directory detection). |

Supporting measurements, same host and session, `sniff` 0.1.0 (both the installed
binary and a release build from this worktree — identical numbers and identical
answers):

| Query | Calls | Wall | User | Sys | Answers `biscuit-test-harness`? |
|---|---:|---:|---:|---:|---|
| A. `repo package-area` per directory, 8-way pool | 73 | 6.2s | 18.2s | 70.4s | yes — `biscuit-test-harness` |
| B. `repo package-areas` + `packages --package-area` per area | 33 | 2.7s | 1.1s | 1.6s | **no — absent** |
| C′. `repo package-areas --package PKG` per package | 73 | 6.3s | 2.3s | 3.8s | **no — empty result** |
| D. `repo packages --json`, one call | 1 | 0.08s | — | — | n/a: a flat list of names, **no area field** |

The three contracts, timed individually inside a suite run (`setUpClass`
amortized): 6.8s for `test_every_workspace_members_area_matches_sniff`, 1.5s for
the five-layout fixture, 0.7s for the universe-subset check — **≈ 8s of the
suite's 13.7s**. The whole `AreaGroupingTests` class, run exactly as
`area-drift.yml` runs it (fresh release binary on `PATH`,
`BISCUIT_REQUIRE_SNIFF=1`): 7.8s wall, 20s user, 78s system.

## Measurement method, and what it is worth

**The build was never measured against the repository's `target/`.** A redirected
`CARGO_TARGET_DIR` into the session scratchpad was used instead, and it does
produce a genuinely cold build (556 `Compiling` lines, a registry download).
Spike 3's objection to a redirected target directory was specific to
`build_key._candidates()`, which composes a literal `scripts/target` path; nothing
in this spike reads a target path. **Nothing was moved aside**, so there is nothing
to restore: the repository's `target/` and `scripts/target` were not written to,
and `scripts/target/debug/ci-build` was reused read-only for the regression run.

Three fidelity corrections, deliberately the same three as spike 3 so the numbers
are comparable:

1. **The host compiler cache was neutralized.** `~/.cargo/config.toml:7` sets
   `rustc-wrapper = "kache"` for every Cargo invocation here, which makes an
   un-neutralized build 3–6× optimistic. Two independent neutralizations were
   applied — `RUSTC_WRAPPER=` in the environment **and** a fresh `CARGO_HOME`,
   which bypasses `~/.cargo/config.toml` altogether (the repository itself tracks
   no `.cargo/config.toml`). Verified rather than assumed: the build ran under
   `cargo build -v`, and the resulting log contains **zero** occurrences of
   `kache` across 556 crates — every `Running` line invokes
   `~/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin/rustc` directly.
2. **Job count.** `CARGO_BUILD_JOBS=4`, matching `ubuntu-latest`'s 4 vCPUs, on a
   16-core `Mac16,5`.
3. **Registry warmth.** `ci-tooling` has no `rust-cache` step, so a fresh
   `CARGO_HOME` in the scratchpad modelled the cold registry: 493M downloaded.

**Fidelity verdict.** 264.6s is a faithful *lower bound* for a hosted cold runner
and nothing more: it holds target directory, compiler cache, registry and job
count at CI's values but cannot correct per-core speed. Applying the same
single-factor scaling spike 3 used (M4 core → `ubuntu-latest` vCore, 1.5–2.6×)
puts the hosted cold build at **6.5–11.5 min** and the hosted warm build at
**2.5–4.5 min**. Those two are **estimates by extrapolation, not measurements**.
The `ci-tooling` "before" figures are neither — they are real hosted wall times
read from the GitHub API.

The fan-out numbers need no such correction to be decisive in shape, only in
magnitude: 73 calls cost **88s of CPU** (18s user + 70s system), i.e. ~1.2s of CPU
each. Four vCPUs cannot beat ~22s of wall for that work even packed perfectly, and
slower cores put the honest estimate at **30–60s** for the contract on
`ubuntu-latest` — against 8s here.

`RAYON_NUM_THREADS` was also pinned to 1 and to 4 as a cross-check: it changes
nothing (10.8/7.1/6.0/6.2s at pools 1/4/8/16 either way), so the ~750% CPU a
single `sniff repo package-area` draws is not Rayon-configurable from the
environment.

## Ruling 1 — where AC15 gets enforced

The decision rule is **total job delta under ~2 minutes → install in
`ci-tooling`**. The measured delta is a **4m25s cold build plus ~8s of contract**,
and on the runner that matters the estimate is 7–12 min. That is not close: even
the most favourable variant — adding a `rust-cache` step to `ci-tooling` and
paying only the warm build — is 99.5s + 8s ≈ **1m48s on this host**, which squeaks
under the rule here and lands at 3–5.5 min hosted, and it would introduce a
1.7G-target / 493M-registry cache entry into a job that deliberately has none.

So the second branch applies: **a scheduled job that builds sniff once, plus an
honest skip message naming it.** Two facts found during the spike make that the
right answer for reasons beyond cost:

- **The suite runs in two jobs, not one.** `test_resolved_plan.py` is a step in
  `ci-tooling` (`ci.yml`) **and** in `preflight`, whose matrix is up to
  `ubuntu-latest` + `macos-latest` + `windows-latest` and which runs on every
  push, not only on CI-tooling changes. "Install sniff wherever the suite runs"
  means paying that build up to **four** times per run, once of them on Windows.
  The review's §1.2 and the spike plan both describe the guards as a `ci-tooling`
  problem; they are a two-job problem.
- **`ci_tooling` cannot see the drift that matters most.** The flag is set by
  path only: `scripts/`, `.github/ci/`, `.github/workflows/`, `tools/test-audit/`,
  the two pnpm manifests, and one Rust contract file
  (`affected_scope.py:120-128`). A pull request that changes
  `sniff/lib/src/filesystem/repo/detection.rs` — sniff changing the very rule the
  planner replicates — sets nothing, and would schedule these contracts **nowhere
  even if `ci-tooling` provisioned sniff**. AC15 names sniff as the authority, so
  that is the direction the gate exists for.

The plan's stated risk is also real and is avoided rather than accepted: building
a workspace package inside the CI-infrastructure gate makes `ci-tooling` red for a
sniff compile error, which is not what that gate reports, and `ci-tooling` is a
`ci-gate` dependency, so it would block merges.

**Shipped:** `.github/workflows/area-drift.yml`, a job outside the merge path
(nothing in `ci-gate` folds it) that

- builds `cargo build -p sniff-cli --release` behind a `rust-cache` entry of its
  own (`shared-key: area-drift`), with `RUSTC_WRAPPER: ""` per repository policy;
- runs `python3 scripts/ci/test_resolved_plan.py AreaGroupingTests` with the built
  binary on `PATH` and **`BISCUIT_REQUIRE_SNIFF=1`**, which turns the guard's skip
  into a failure — the provisioning regression spike 3 guarded against with
  `BISCUIT_CI_BUILD_BIN` is the same failure mode;
- triggers on `schedule` (05:40, clear of `sniff-performance`'s 04:00),
  `workflow_dispatch`, **and `pull_request` limited to `sniff/**`,
  `affected_scope.py`, `test_resolved_plan.py`, and the workflow itself** — the
  paths `ci_tooling` structurally cannot cover. The PR leg is what turns this from
  after-the-fact reporting into pre-merge signal for the sniff-side direction; it
  is not a required context, so it informs rather than blocks.

Estimated cost: 7–12 min on its first run, **3–5.5 min steady-state** with the
cache warm, once a day plus once per sniff pull request.

**The tension, named rather than resolved silently.** A nightly that nobody reads
is only marginally better than a skip. This workflow is not a required check and
`ci-gate` cannot see it, which is exactly what keeps a sniff compile break out of
the merge gate — and also what lets a red run sit unnoticed. The `pull_request`
trigger is the mitigation, because that leg appears in the checks list of the pull
request that causes the drift. If the repository later wants AC15 *blocking*, the
cheaper home is not `ci-tooling` but the sniff area's own CI: `sniff-performance.yml`
already builds `-p sniff --release` under a per-OS `rust-cache` on every
`sniff/**` pull request, so the marginal cost there is the CLI's own crates plus
~30–60s of contract. That file belongs to the sniff area and was left alone.

## Ruling 2 — query semantics: A, B, or C

**A.** The rule says adopt B only if detection and the universe agree for every
member including `biscuit-test-harness`. Re-measured this session, they do not.

- B loses `biscuit-test-harness` entirely (73 members in, 72 out), exactly as the
  plan predicted, and it is 2.3× faster in wall time and ~44× cheaper in system
  time. That is a real cost difference and it buys a strictly weaker contract:
  "which packages does sniff *list* under this area" instead of "what area does
  sniff *detect* for this directory". AC15 makes the second authoritative.
- C′ (`repo package-areas --package PKG`, the plan's third option) is worth
  recording as **measured and disqualified on semantics, not cost**: 6.3s wall but
  only 3.8s system, a ~20× system-time saving over A at the same call count. It
  returns an **empty result** for `biscuit-test-harness`, so it shares B's
  semantics — it reads the same universe, one package at a time.
- D, a single `sniff repo packages --json`, would have been the cheap win if it
  carried an area per package. It does not: the payload is a flat list of names.
- C (both) is defensible but does not reduce the cost that motivated the spike, and
  the universe assertion it would add already exists as
  `test_the_area_universe_is_a_subset_of_sniffs`.

So the per-directory fan-out stays, and with it the contract AC15 actually names.

### The `max_workers` sub-question, answered against the plan's hypothesis

The plan expected the `ThreadPoolExecutor` to be the cost driver ("the harness
fighting it") and pre-authorized a one-line fix. **Measured, it is not, and the
fix is not taken.** System time is flat at ~70s across pool widths 1, 4, 8 and 16;
the ~70s lives inside sniff's own per-invocation walk, 73 times over. The pool
only overlaps that work: it buys 4.7s of wall (10.9s → 6.2s) and costs nothing.
Narrowing to `max_workers=1` would have made the contract slower and saved no CPU.

What shipped instead is the measurement, as a comment at the `max_workers=8` line,
so the next reader does not re-derive it — and does not "fix" it the wrong way.

The plan's baseline (14.5s wall / 85s system) is not reproduced as a wall figure:
the same work measures 6.2s / 70s here. The system time matches; the wall gap is
the planner setup the earlier figure included, plus host load. The *conclusion*
the plan drew from that baseline — that the pool multiplies an already-parallel
process — is what the re-measurement contradicts.

## `SNIFF_SELF_INCONSISTENT`, re-measured

**Still exactly one entry. It is not deleted.**

```
{'biscuit-test-harness': ('root', 'biscuit-test-harness')}
```

Measured twice over all 73 workspace members: once with the installed `sniff`
0.1.0, once with a release `sniff-cli` built from this worktree in this session —
byte-identical results, so the entry is not an artifact of a stale developer
binary. `biscuit-test-harness` is also still the only detection answer absent from
`sniff repo package-areas`, still the only member the inverted query loses, and
still the only member `--package` mode answers nothing for. Its comment's
instruction ("when sniff is fixed, this entry fails and gets deleted") does not
fire; the entry's date line now records both measurements.

## What shipped

- **`scripts/ci/test_resolved_plan.py`**
  - `require_sniff()` replaces the three
    `@unittest.skipUnless(shutil.which("sniff"), "requires sniff")` decorators
    (`:272`, `:299`, `:334` before the edit). It skips where sniff is genuinely
    absent, with a message naming `area-drift` and `just ci-local` as the surfaces
    that do enforce AC15 and stating plainly that `ci-tooling` and `preflight` do
    not provision it — and it **raises `AssertionError` instead of skipping when
    `BISCUIT_REQUIRE_SNIFF` is set**, which is the same "a skip here would be a
    green cell that verified nothing" principle `test_ci_local.py:1267` already
    applies, wired to the job that provisions the tool rather than to `CI`.
    Wiring it to `CI` would have turned `preflight` red on three OSes.
  - `SNIFF_ENFORCEMENT`, the written answer to "where is AC15 enforced?", which
    §1.2 correctly says did not exist anywhere.
  - The `max_workers=8` measurement, as a comment.
  - `SNIFF_SELF_INCONSISTENT`'s date line records the 2026-09-15 re-measurement.
- **`.github/workflows/area-drift.yml`** (new) — described under ruling 1.
- **`.github/workflows/ci.yml`**, `ci-tooling` only — a four-line comment above
  `Test the resolved execution plan` saying three of that suite's tests skip there
  deliberately, with the reason and the job that enforces them. Spike 3's step
  ordering is untouched: no sniff build was added to this job, so the
  `cargo nextest … --bin ci-build` → `test_build_key.py` adjacency that makes that
  suite cost 0.1s is preserved exactly. Had a sniff build gone in, it would have
  belonged *after* those steps for the same reason — it shares no crates with
  `scripts/`' `--no-default-features` builds and would have pushed a 4-minute
  compile ahead of the cheap Python suites — but the ruling makes the question
  moot.

## Step 4 — the other tool-gated contracts

Nothing else in `test_resolved_plan.py` depended on sniff's absence. The file has
exactly three tool guards (now zero decorators and three `require_sniff()` calls)
and one other `subprocess.run`, the `package-areas` call inside the universe test,
which is already behind the same guard. With sniff present the suite is 64 tests,
0 skips; with sniff removed from `PATH` it is 61 + 3 skips; with sniff removed and
`BISCUIT_REQUIRE_SNIFF=1` it is 61 + 3 failures. All three states were run.

## What the plan did not anticipate

1. **`preflight` runs this suite too, on up to three OSes, on every push.** The
   plan and §1.2 both frame the guards as a `ci-tooling` question. This is what
   ruled out the review's own recommendation of a `require_tool()` that fails
   under `CI`: as written it would fail `preflight` on macOS and Windows, where
   provisioning sniff is three more builds, not one. Keying the hard failure to
   the provisioning job's own env var is what makes the honest-skip principle
   applicable at all here.
2. **The `ci_tooling` flag cannot see sniff.** `affected_scope.py:120-128` follows
   CI-tooling paths only, so the sniff-side drift direction is unscheduled even
   with sniff installed in `ci-tooling`. Adding `sniff/**` to `CI_TOOLING_PREFIXES`
   would be the one-line alternative to this spike's `pull_request` trigger, but
   `affected_scope.py` belongs to another spike and was not touched; it is worth
   considering, because it would put the contracts on the merge path.
3. **The pool hypothesis is wrong** (above). Worth recording because it is the one
   item the plan pre-authorized shipping unconditionally.
4. **`sniff repo packages --json` carries no area**, so there is no one-call
   detection map to be had; and `--package` mode is cheap in *system* time (3.8s
   vs 70.4s) while being semantically the universe, not detection. If sniff ever
   grows a `repo package-areas --detect --json` that answers per directory in one
   process, option A's 88s of CPU collapses and this ruling should be revisited.
5. **`sniff-performance.yml` already exists** and already builds sniff nightly on
   three OSes and on every `sniff/**` pull request. The spike priced a build from
   scratch as if none existed; the cheapest *blocking* enforcement available is
   that workflow's Linux leg, not `ci-tooling`. Left alone as another area's file.
6. **Warm does not mean what the plan's step 1 implies.** `Swatinem/rust-cache`
   cleans workspace crates before saving, and sniff-cli's closure contains 11 of
   them (including `darkmatter`, which the sniff library alone does not pull), so
   the warm number is 99.5s, not the 0.7s a "warm cache" intuitively suggests.
7. **Cold `ci-tooling` with a sniff build would flirt with its own timeout.** The
   job is `timeout-minutes: 20` and already runs 6m10s; the estimated cold
   addition is 6.5–11.5 min. A cache miss on a slow runner could turn AC15
   enforcement into a 20-minute cancellation of the CI-infrastructure gate.

## Verification

| Check | Result |
|---|---|
| `actionlint .github/workflows/ci.yml` | exit 0, no findings |
| `actionlint .github/workflows/area-drift.yml` | exit 0, no findings |
| `python3 -m py_compile scripts/ci/*.py` | clean |
| All 13 `scripts/ci/test_*.py` suites | **611 tests, 0 failures, 0 skips, 144s** — the post-spike-1–3 baseline held exactly |
| `python3 scripts/ci/test_resolved_plan.py` | 64 tests, OK, 13.7s |
| …with `sniff` off `PATH` | 61 OK + 3 skipped, each naming `area-drift` |
| …with `sniff` off `PATH` and `BISCUIT_REQUIRE_SNIFF=1` | 3 failures — the provisioning regression is loud |
| `python3 scripts/ci/test_resolved_plan.py AreaGroupingTests`, release binary on `PATH`, `BISCUIT_REQUIRE_SNIFF=1` (as `area-drift.yml` runs it) | 7 tests, OK, 7.8s |
| `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | **119 passed, 0 skipped** — the new workflow satisfies the three `read_dir(.github/workflows)` contracts, including `every_cargo_workflow_neutralizes_a_stray_rustc_wrapper` |
| Repository `target/` and `scripts/target` | never written to; no directory was moved aside, so nothing needs restoring |

`.githooks/tests/test-pre-push.sh` was not run: the `just ci-local` suite list is
unchanged, and nothing in the hook suite reads `ci.yml` or the new workflow.

## What could not be measured here

- **A hosted `ubuntu-latest` build.** Every build figure is from a 16-core M4 with
  the job count pinned to 4. The two hosted numbers that are real — `ci-tooling`'s
  5m54s–6m43s — come from the GitHub API, not from this host. The ruling does not
  depend on the extrapolation: it is over the 2-minute budget by a factor of two
  on the *measured* proxy alone.
- **The new workflow has never run.** Its shape is lint-clean and its test step
  was executed locally in the exact configuration it will use, but the Linux build
  is unproven. sniff-cli's closure declares no `[package.metadata.ci.native]`, and
  `ci.yml`'s build owner compiles every package on `ubuntu-latest` from that same
  union, which is the evidence that no `apt-get` step is needed —
  `sniff-performance.yml` nevertheless installs `pkg-config libssl-dev libgit2-dev`
  on its Linux legs. If the first `area-drift` run fails in `Build the area
  authority`, that apt step is the first thing to try.
- **A debug `sniff-cli`** was not priced. A cheaper compile would be paid back
  with a materially slower 73-call fan-out, and the release build's 99.5s warm
  figure already exceeds the budget, so it cannot change the branch.
