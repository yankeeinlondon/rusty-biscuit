---
title: Finalizing the single-os-compile branch after the main merge
status: ready
created: 2026-09-16
agent: claude/opus-5
spec: fixes/2026-09-12-single-os-compile/spec.md
previous: fixes/2026-09-12-single-os-compile/review-4.md
packages:
    - repo-deps
    - test-toolkit
---

# Finalizing Plan

Five steps between the committed merge (`27230c97f`) and a pushed branch. Four
are mechanical and verifiable locally; step 4 is a design decision that should
be taken before the branch is presented as meeting its objective.

The ordering matters: step 2 plausibly reduces step 1's cost, so step 1's final
measurement should be taken after step 2 lands. Step 3 is independent. Step 4
gates nothing mechanically but decides what the branch claims.

## Context the steps rest on

Measured on this host, 2026-09-15/16, after the merge:

| Fact | Value |
|---|---|
| `slow_real_package_archives…` cold / warm / in-suite | 107.5s / 73.1s / 66.0s |
| Full-scope tier cells (L1/L2/browser) | 294 (270 L1, 22 L2, 2 browser) |
| Full-scope compiles before this work / after | 228 / 204 (**−24, 10.5%**) |
| Available saving captured | 24 of 24 |
| `claudine/lib` change: new compiles saved | **0** (its only sharing predates this work) |
| `biscuit-terminal/lib` change: new compiles saved | 1 of 4 |
| Python suites / workflow contracts / rollup | 742 · 136 · 237, all green |

The headline: the compile-once mechanism captures **every** saving available to
it, but the repository's tier mix (270 L1 against 24 L2+browser) caps that
saving at ~10% of full-scope compiles, and at **zero** for a single-package
L1-only change — which is the common pull request. Against that, each executing
tier cell now pays archive upload, download, extraction, and manifest
verification that it did not pay before. Net wall-clock is unmeasured.

---

## Step 1 — Make the relocation test cheap, then restore the timeout ceiling

**Why.** A 480s ceiling was added to `.config/nextest.toml` (both `default` and
`ci` profiles) so this test could pass. That tolerates the cost instead of
removing it, and makes a slow test permanently invisible. The cost is
duplication, not necessity.

**What is actually expensive.** `slow_real_package_archives_read_their_fixtures_from_the_consumers_checkout`
(`scripts/ci-build-archive-tests.rs:2988`) loops over `["test-toolkit", "biscuit-file"]`
and for **each** package calls `relocate_real_package` (`:2863`), which:

1. `copy_repository(&repo_root(), &producer)` (`:2870`) — a recursive
   file-by-file copy of the whole working tree, minus
   `UNCOPIED = ["target", ".git", "assets", "node_modules"]` (`:2777`). Done
   **twice**.
2. `git init` + `add -A` + `commit` over that copy (`:2871-2873`) — `source_tree`
   refuses a dirty or non-git workspace, so the producer must be a real
   repository.
3. `produce` with `--target-dir` pointing at a **fresh empty** `dir.join("target")`
   (`:2877`, `:2888`) — so each package cold-builds its entire dependency graph,
   sharing nothing with the other.
4. `run_relocated` (`:2941`) then actually executes the archived L1 suite.

The fixture crate itself compiles in 0.15s and archives in 0.03s. Steps 1 and 3
are the cost, and both are duplicated across the two packages.

**Change.** Hoist the producer checkout and the producer target directory out of
the per-package loop:

- Build **one** producer checkout (copy + git init/commit) and reuse it for both
  packages.
- Pass **one** shared `--target-dir` to both `produce` invocations, so
  `test-toolkit` and `biscuit-file` share their common dependency compilation.
- Keep one `Scratch` per package for `out`, `extracted`, and `consumer`, so each
  package's archive and consumer checkout remain isolated.

**Why this does not weaken the contract.** What the test proves is that a real
package archive runs from a *different checkout* with the producer's target
absent. `run_relocated` extracts to its own directory (`:2942`) and passes
`--workspace-remap <consumer>` (`:2950-2951`); it never reads the producer
target. A warm producer target is irrelevant to that claim. Confirm this by
re-reading `run_relocated`'s assertions before changing anything, and keep the
consumer checkout per-package.

**Follow-on, not required now.** A stronger version puts both packages in one
plan with one `produce` invocation, which is what the specification's
"deterministic order in one target tree" describes and would test more than the
current shape. It needs distinct build keys — `real_plan` (`:2810`) hardcodes
`"1122334455667788"` for every record, so two records in one plan would collide
on the plan validator's duplicate-key rule. Note it and leave it.

**Then revert the ceiling.** Remove both overrides added in
`.config/nextest.toml` (the `[[profile.default.overrides]]` and
`[[profile.ci.overrides]]` blocks filtered on `package(repo-deps) & test(/slow_/)`).
If the test still exceeds 30s after step 2, do **not** reinstate a large
ceiling: report the remaining cost and decide whether the test belongs in L1 at
all, or should be scoped to changes that touch the archive producer or consumer.

**Done when**

- One repository copy and one producer target per test run, verified by reading
  the diff rather than by timing alone.
- `cargo nextest run -p repo-deps --bin ci-build` passes with **no** `slow_`
  override present.
- The measured duration is recorded here, alongside the 107.5s/73.1s baseline.

**Risks**

- Sharing a target directory between two packages could mask a per-package
  build defect that a cold target would surface. Mitigation: the archive
  manifest still records each package's own inventory, and `run_relocated`
  still executes each archive independently.
- If `source_tree` hashes the producer checkout, one shared checkout changes
  what `real_plan`'s `source_commit` resolves to for the second package. Check
  `real_plan` (`:2810-2811`) before assuming the commit can be shared.

---

## Step 2 — Move the test scratch onto the repository's volume

**Why.** Two problems with one cause. `scratch()`
(`scripts/ci-build-archive-tests.rs:19-30`) builds every fixture under
`std::env::temp_dir()` — `/private/var/folders/...` on this host, the **system
volume**. The kache store lives on `/Volumes/coding`. Every fixture build is
therefore cross-volume, so kache can never hardlink into its store, always falls
back to copying, and prints an advisory whose text begins with `error: Cross-device
link` into the stream the producer's output is captured from. The result is
non-deterministic failures in the `ci-build` archive tests: a different pair
fails on each run.

**Evidence.** `kache doctor` reports **all checks passed**, including "hardlink
build-tree → `<store>/staging` works" — because it probes the repository build
tree (`/Volumes/coding/wt/rusty-biscuit/feat-single-os`), not the fixture build
tree. Setting `TMPDIR`, `RUSTC_WRAPPER=""`, and `CARGO_BUILD_RUSTC_WRAPPER` all
failed to move or silence it: Cargo treats an empty wrapper as unset and falls
back to `~/.cargo/config.toml`'s `[build] rustc-wrapper = "kache"`, and the
staged `.dsym.tar` path ignores `TMPDIR`.

**Change.** Default the scratch root to a directory on the repository's own
volume — `<repo>/target/ci-build-scratch/` is the obvious candidate, since
`target` is already gitignored and already excluded from `copy_repository` by
`UNCOPIED`. Keep an environment override (`BISCUIT_CI_BUILD_SCRATCH` or
similar) so a runner with a different layout can redirect it, and keep
`Scratch`'s `Drop` cleanup exactly as it is, including the read-only retry
(`:45-55`) that stops a fixture repository from being left behind.

**Why this is the right fix rather than a workaround.** It puts the build tree,
the kache store, and the staging area on one volume, which is the layout kache
asks for. That restores the zero-copy sharing kache exists to provide, on
precisely the builds that dominate step 1's cost — so this step may reduce step
1's remaining time as a side effect. It uses the tool as intended instead of
disabling it.

**Done when**

- `cargo nextest run -p repo-deps --bin ci-build` passes **repeatedly** — run it
  at least three times; the failure this fixes is non-deterministic, so a single
  green run proves nothing.
- No `kache: operation fell back to COPY … EXDEV` line appears in the captured
  output of a fixture build.
- The scratch root is honoured on a host whose repository is not on
  `/Volumes/coding` (check the override path, do not assume).

**Risks**

- `target/` is swept by the weekly `cargo-sweep` launchd job noted in
  `~/.cargo/config.toml`. Scratch directories are removed by `Drop` anyway, but
  confirm a sweep mid-run cannot delete a live fixture.
- Windows and Linux hosts have different temp semantics; the override must work
  there. This is a cross-platform repository — do not hardcode a POSIX path.

**Worth reporting upstream:** `kache doctor` reports a green link layout while a
real EXDEV fallback is occurring on the dsym staging path. That is a blind spot
in `doctor`, not a misconfiguration.

---

## Step 3 — Bump the resolved-plan schema to version 4

**Why.** Both sides of the merge independently bumped the plan schema 2→3 for
different additions: this branch added `builds[]`, main added
`change_inventory`. "Version 3" now names two incompatible shapes, and the
merged validator requires both fields.

**It is already safe, and that is not the point.** A stale v3 document fails the
required-field check *before* the version check — `validate_resolved_plan`
returns accumulated field problems at `scripts/ci/schema.py:712-713`, ahead of
the version comparison at `:715`. So a pre-merge document is rejected as
`malformed-receipt`, and a scope receipt carrying one as `scope-malformed`. CI's
documented response to either is to recalculate scope. Nothing is misread; the
cost is lost evidence reuse on the first runs after merge.

The defect is diagnostic: a version skew is reported as a corrupt document,
which sends the reader looking for the wrong thing. Version 4 makes the
rejection state the true reason, and is honest — the merged shape genuinely is a
fourth revision.

**Sites to change**

| File | Anchor |
|---|---|
| `scripts/ci/schema.py` | `RESOLVED_PLAN_SCHEMA_VERSION = 3` (`:77`) |
| `scripts/ci-rollup.rs` | `const PLAN_SCHEMA_VERSION: u32 = 3` (`:78`), with the comment at `:77` pointing at the Python constant |
| `.github/ci/schemas/contract.json` | **regenerate** with `python3 scripts/ci/schema.py`; never hand-edit |
| `.githooks/tests/fixtures/plan-macos-executing.json`, `plan-macos-two-packages.json`, `plan-wsl-absent.json`, `plan-wsl-executing.json`, `plan-wsl-reused.json` | `"schema_version": 3` |
| `.githooks/tests/fixtures/affected_scope_stub.py` | same |
| `scripts/ci-plan-tests.rs`, `scripts/ci-rollup-tests.rs` | fixture literals |

Search for remaining occurrences rather than trusting this table; it was built
by grep and a fixture may carry the number in an unexpected shape.

**Done when**

- All 13 Python suites, `ci_workflow_contracts`, `ci-rollup`, and `ci-plan` pass.
- `test_shipped_contract_matches_this_module` and the byte-stability test pass,
  and `contract.json` differs **only** in the version.
- A deliberately-stale v3 document is rejected with a message naming the
  version, not with a field-shape complaint. Add or adjust a test for this —
  the whole point of the bump is the diagnostic, so leaving it unpinned would
  waste the change.

**Risk.** Any scope receipt already published on `refs/notes/ci-local/scope`
stops qualifying. That is the intended outcome and it is already true for
pre-merge receipts; it costs one recalculation per branch.

---

## Step 4 — Decide what the performance comparison now measures

**This is a decision, not a task, and it should be made before the branch claims
to have met its objective.**

**Why the original comparison is gone.** `build_baseline_revision.py` constructs
an instrumentation-only pre-cutover revision by copying `CARRIER_PATHS`
(`:94-103`) onto base `8aa105e7`. That list includes `scripts/Cargo.lock`, which
main deleted when it made `scripts` a member of the root workspace. All 8
failures in `scripts/ci/test_build_baseline_revision.py` are this one cause:
`build_files` refuses at `:528` with "carrier path is missing from the source
tree". It is not a list edit — the constructed tree would copy a post-merge
`scripts/Cargo.toml`, now a workspace *member*, onto a base whose root manifest
does not list `scripts` and whose `scripts/` carried its own lock. The
instrument and its base are from different worlds.

Nothing measured is lost: the observation tables in `baseline-2026-09-12.md`
(lines 184-191 and the warm table below them) are empty.

**The more important reason to revisit it.** The measurements above suggest the
original comparison would answer a question that no longer matters much. The
architecture captures 24 of 24 available compile savings, but that is 10.5% at
full scope and **zero** for a single-package L1-only change, while every
executing tier cell now pays archive transfer it did not pay before. A cold/warm
comparison of the full-scope schedule would mostly measure the 270 L1 cells
where nothing changed.

**Options**

1. **Re-base the baseline onto post-#79 `main`.** Reconstruct the
   instrumentation-only revision there and run the original comparison.
   - *For*: keeps the specification's acceptance criterion intact and
     measurable.
   - *Against*: costs a reconstruction and three green runs per environment to
     answer a question whose expected effect is ~10%, inside a 15% noise band.
     The criterion could pass while the common case regressed.
2. **Replace it with the comparison the measurements point at** (recommended):
   on a representative **single-package** pull request, measure archive upload,
   download, extraction, and verification against compiles avoided. That is
   where the architecture looks break-even or negative, and it is the shape of
   the majority of runs.
   - *For*: answers the question that decides whether this work pays.
   - *Against*: it is not the criterion the specification wrote down, so
     adopting it is an explicit amendment and must be recorded as one.
3. **Record the baseline as invalidated and defer**, as
   `deferred-performance-measurements.md` already does for the hosted runs.
   - *For*: honest, cheap, unblocks the push.
   - *Against*: the branch then merges without evidence for its central claim.

**Recommendation: 2, with 3 as the fallback if hosted runs cannot be scheduled
soon.** Whichever is chosen, `CARRIER_PATHS` must stop naming a file that does
not exist, and `test_build_baseline_revision.py` must either pass or be
explicitly retired — a suite that cannot run is the failure mode this branch has
spent its time eliminating.

**Done when** the choice is recorded in `rollout-2026-09-12.md` and
`deferred-performance-measurements.md`, and the baseline suite is green or
deliberately removed.

---

## Step 5 — Push

The merge is committed (`27230c97f`). Only the push remains.

**Gates, all run from a clean tree before pushing**

```
for f in scripts/ci/test_*.py; do python3 "$f"; done      # expect 0 failures, 0 skips
cargo nextest run -p test-toolkit --test ci_workflow_contracts
cargo nextest run -p repo-deps                            # includes ci-build, ci-rollup, ci-plan
actionlint .github/workflows/*.yml
just ci-local --plan
```

Current status of those gates: Python 742 tests with `test_build_baseline_revision.py`
failing (step 4), workflow contracts 136/136, rollup 237/237, `ci-build` 117/117
under the default profile, `actionlint` clean.

**Expect the pre-push hook to run `repo-deps` L1**, which includes the `ci-build`
archive tests. Until step 2 lands, that can fail non-deterministically on this
host for reasons unrelated to the change. Do not bypass the hook to get past it
— fix step 2 first, or the same failure reaches CI as a mystery.

**What the pull request will exercise on its own:** the `area-drift` job, since
the branch changes `scripts/ci/affected_scope.py` and many `Cargo.toml` files,
both of which set the `area_drift` flag. That is the first real run of a gate
that has never executed, so read its result rather than assuming a green fold.

**Do not** dispatch `area-drift.yml` manually to test it first: `workflow_dispatch`
only surfaces once a workflow is on the default branch, so it would not run from
this branch anyway.

---

## Sequencing

```
step 2 ──► step 1 (measure after 2; 2 may reduce 1's cost)
step 3 ──► independent, any time
step 4 ──► decision; blocks the branch's claim, not its push
                        └──► step 5
```

Steps 1, 2, and 3 are mechanical and can be verified locally in full. Step 4
needs a human ruling. Step 5 should follow all four.
