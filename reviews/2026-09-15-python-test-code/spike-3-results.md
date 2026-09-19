---
title: "Spike 3 results — what test_build_key.py costs on a cold runner"
created: 2026-09-15
status: complete
source: reviews/2026-09-15-python-test-code/review.md §1.1, §5.6
decides: how test_build_key.py is wired into CI and just ci-local
decision: "Cold is 26.4s at the worst proxy this host can build — the under-30s branch. Wired into `ci-tooling` unconditionally, positioned after the step that already produces `scripts/target/debug/ci-build` so it compiles nothing; the `just ci-local` half is blocked on a four-site suite-list coupling."
---

# Spike 3 results — What does `test_build_key.py` cost on a cold runner?

## Results to record

| Measurement | Value |
|---|---|
| Cold wall time | **14.3s** (16 cores, warm registry, compiler cache bypassed). **24.6s** at four Cargo jobs. **26.4s** at four jobs with a cold registry — the worst-case cold proxy this host can build. **4.3s** with the host's `kache` wrapper left active, which is not cold at all. |
| Cold: did `cargo` compile? | Yes. 207 crates, a 1.0G `scripts/target`, 171M of registry downloaded when `CARGO_HOME` was fresh, and `scripts/target/debug/ci-build` left behind by the run. |
| Warm wall time | **0.10s–0.12s** measured (macOS), consistent with the review's 0.13s. |
| Does `ci-tooling` already build `scripts/`? | Yes — three `cargo nextest` steps at the end of the job. `cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-build` **does** materialize `scripts/target/debug/ci-build` (verified by deleting the artifact and re-running the step). All the Python suites currently precede those steps, so at the position §1.1 proposed the suite is maximally cold. |
| Tests that pass with no binary and no `cargo` | **4 of 12**, in 0.063s. The other 8 raise `RuntimeError` and are reported as errors. Nothing hangs. |

## Measurement method, and what it is worth

The plan's step 1 says `rm -rf scripts/target`. That directory is a 7.7G live build
cache on the development host, so it was **moved aside**, not deleted:
`mv scripts/target /Volumes/coding/wt/rusty-biscuit/.spike3-target-aside` (same APFS
volume, so an instantaneous rename), measured against, then moved back. The restore
was confirmed: 7.7G, `debug/ci-build` at its original 05:56 mtime, and the warm suite
green again at 0.10s.

A redirected `CARGO_TARGET_DIR` was rejected as the cold method, and this is worth
recording because it is the obvious first choice and it does not work:
`build_key._candidates()` (`scripts/ci/build_key.py:59-62`) composes the probe path
from `ROOT / "scripts" / "target"` and never consults `CARGO_TARGET_DIR`. Verified
directly — with `CARGO_TARGET_DIR` pointed into the scratchpad, `helper_command()`
still resolved `scripts/target/debug/ci-build`, and the suite still ran in 0.12s. A
redirected target directory measures nothing.

Three fidelity corrections were applied on top of the plain cold run:

1. **The host's compiler cache had to be neutralized.** `~/.cargo/config.toml:7` sets
   `rustc-wrapper = "kache"` for every Cargo invocation on this machine, and a `kache`
   daemon is resident. With it active, an empty `scripts/target` rebuilt 1.0G in
   **4.3s**. Bypassing it (`RUSTC_WRAPPER=` and, as a cross-check,
   `RUSTC_WRAPPER=/usr/bin/env` as a passthrough) gave 14.37s and 14.27s — two methods
   agreeing to within 0.1s, against 207 compiled crates both times. Env precedence over
   `~/.cargo/config.toml` was confirmed by pointing the wrapper at a nonexistent binary
   and watching Cargo fail.
2. **Core count.** The host is a 16-core `Mac16,5`; `ubuntu-latest` is 4 cores.
   Re-measured at `CARGO_BUILD_JOBS=4`: 24.6s.
3. **Registry warmth.** `ci-tooling` has **no** `Swatinem/rust-cache` step — the only
   `rust-cache` in `ci.yml` is in the `build` job (`ci.yml:592`) — so every `ci-tooling`
   run starts with an empty registry as well as an empty target directory.
   Re-measured with a fresh `CARGO_HOME` in the scratchpad: 26.4s, 171M downloaded.

**Fidelity verdict.** 26.4s is a faithful *lower bound* on a hosted cold runner and
nothing more. It holds the target directory, the compiler cache, the registry and the
job count at CI's values, but it cannot correct for per-core speed: these are M4 cores,
and an `ubuntu-latest` vCore is materially slower on `rustc` work. Scaling only that
remaining factor puts a real `ubuntu-latest` somewhere around **40–70s** — that number
is an **estimate**, by single-factor extrapolation from the 4-job measurement, not a
measurement. A hosted `ubuntu-latest` cold runner cannot be measured from this host at
all.

Per the repo's standing rule, the warm `kache` figure (4.3s) is recorded as an
optimization and is not treated as cold. The same rule is why the `fetch-depth` and
`BISCUIT_CI_BUILD_BIN` choices below fail loudly rather than depending on a cache being
present.

## The ruling

The decision rule's branches are **under ~30s / 30s–3min / over ~3min**. The measured
cold wall time, at the most pessimistic proxy available, is **26.4s** — the **first
branch**: *add to `ci-tooling` and to the `just ci-local` self-test loop
unconditionally.*

The estimated hosted figure (40–70s) straddles into the second branch, so the position
inside `ci-tooling` was chosen to satisfy the *stricter* branch too, which the first
branch leaves open. The second branch's own words are "reorder after a step that
already builds `scripts/`", and that reorder turns out to be free: the job's last step,
`cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-build`, provably leaves
`scripts/target/debug/ci-build` in place. Run after it, the suite costs 0.1s and
compiles nothing, so the ruling is robust whichever side of 30s the real runner lands
on. No estimate is load-bearing.

The third branch is not reached, and the plan's risk about it does not materialize: the
pinned XXH64 vectors (`test_build_key.py:94-97`) are checked on every `ci-tooling` run,
with the binary built rather than the tests gated — which is what the plan asked for.

### The `just ci-local` half is not shipped, and why

The first branch also calls for the suite in the `just ci-local` self-test loop. It is
not there, and this is a file-ownership block rather than a judgement:

The loop's suite list is hardcoded in **four** places, and all four must move together —
`just/ci-local.just:449`, `.githooks/tests/test-pre-push.sh:2099`, and
`scripts/ci/test_ci_local.py:129` and `:883`. The latter two are fixtures that write a
no-op stub for each named suite into a temp root; a suite in the recipe but not in the
stub list is run by the recipe against a file that does not exist, fails, and
`ci-local.just:557` exits 1 on any failed entry. Measured, not assumed: adding
`test_build_key.py` to `ci-local.just:449` alone took `test_ci_local.py` from 66 passing
to **10 failures**. `scripts/ci/test_ci_local.py` belongs to a completed spike and was
out of scope here, so the probe was reverted.

The one-line follow-up is: add `test_build_key.py` to all four lists in one change. It
is safe on the local path — `ci-local.just` runs `affected_scope.py` (lines 212/217/232)
long before the self-test loop at 449, and `affected_scope.py:2291` computes build keys
through `build_key`, so the helper is already resolved by then.

## What shipped

- **`scripts/ci/test_build_key.py:139`** — `timeout=300` on the `subprocess.run`, the
  last unbounded subprocess in the test suites. 300s matches the ceiling the other
  build-capable suites in the directory already use (`test_affected_scope.py:2658`,
  `:3569`) and leaves a wide margin over the 26.4s worst case, so a genuinely cold
  developer host cannot be turned into a flake by the bound.
- **`.github/workflows/ci.yml`**, `ci-tooling` only:
  - `Test the build-key hashing boundary` added as the job's **last** step, after
    `Test the compiler-work counter`, with
    `BISCUIT_CI_BUILD_BIN: ${{ github.workspace }}/scripts/target/debug/ci-build`.
    Naming the helper rather than letting `helper_command()` probe for it is the part
    that matters: if a future reorder removes the producing step, the suite fails
    loudly (`build_key.py:87` — "names '…', which is not a file") instead of quietly
    growing a minute-long Rust build back inside a unit suite. The step carries a
    comment saying the position is load-bearing.
  - `Test the measurement baseline constructor` added with the other pure-Python
    suites, ahead of the Cargo steps. The plan's claim was re-verified after spike 1's
    refactor and holds: 11 tests, 5.84s, no `build_key` and no `cargo` — its
    subprocesses are `git`, and it sets `GIT_AUTHOR_*`/`GIT_COMMITTER_*` and
    `commit.gpgsign false` on its temp repositories, so it needs no runner Git
    identity. It now imports spike 1's `workflow_reading`, which reads only checked-out
    workflow files.
  - `fetch-depth: 0` on the job's checkout — see below.

## What the plan did not anticipate

**1. Wiring `test_build_baseline_revision.py` into `ci-tooling` silently disables most
of it.** The suite gates two classes on `BASE_AVAILABLE`
(`test_build_baseline_revision.py:121`, `:294`), which tests whether
`build_baseline_revision.BASE_REVISION` (`8aa105e7c…`) is in the checkout's history.
`ci-tooling`'s checkout took the default depth of 1, at which that commit is absent —
verified against a real `git clone --depth 1`, where `git cat-file -e` on it fails. So
the suite would have reported 11 tests green having actually run **3**; `ConstructedRevision`
(7 tests) and `PublishedRevision` (1) would have skipped. The plan treats this suite as
needing "no spike, only the same PR", and on that basis it would have shipped a fresh
instance of the review's own §1.2 defect. Fixed with `fetch-depth: 0`, consistent with
`scope` (`ci.yml:133`) and `preflight` (`ci.yml:459`), which already pay it. Cost of the
deeper fetch measured locally: a `--depth 1` clone of this repository is 531M against a
645.6 MiB full pack, so roughly 100–115 MiB more on this leg.

**2. The cold number could not have been taken naively on this host.** Any timing spike
run here without neutralizing `rustc-wrapper = "kache"` reports a number 3–6× optimistic
(4.3s vs 14.3s on the identical work). This is not specific to spike 3 and belongs
wherever the repo records how to time a build.

**3. The split tally in §5.6 and in step 4 is wrong, and in the direction that matters.**
§5.6 says six tests reach the helper plus `test_the_cli_and_the_module_agree_on_one_input`.
Measured, **8 of 12** fail without it. `DigestTests` is 7 tests, of which 6 need the
helper (`test_an_empty_batch_needs_no_helper_at_all` returns before resolving it), plus
`CanonicalizationTests`' 1, plus `HelperResolutionTests:76`. More importantly, the plan
records `:76` as a shape-only test that "need[s] no real binary" — that holds only while
`cargo` is on `PATH`. With neither a binary nor `cargo`, `helper_command()` raises and the
test errors. Had the third branch been taken, the "runs everywhere" set would have been 4
tests, not 5, and one of the four would have been conditional on a Cargo install.

**4. `scripts/target` is not where `just ci-local` builds.** Noted because it bears on the
local wiring and on the probe's reliability generally: `ci-local.just:160` exports
`CARGO_TARGET_DIR="${BISCUIT_CI_TARGET_DIR:-${ci_repo_root}/target/ci-local}"`, while both
`build_key._candidates()` (`build_key.py:59-62`) and `just/devops.just:59` compose their
paths from a literal `scripts/target`. Under `ci-local`, Cargo writes `ci-build` to
`target/ci-local/debug/` and neither the probe nor `devops.just:60`'s `[[ -x … ]]` guard
looks there; on a developer host they succeed only because some earlier non-`ci-local`
build happened to leave an artifact at the literal path. Not verified end-to-end — no
`target/ci-local` exists in this worktree and `devops.just` was out of scope — but the
path composition is plainly independent of `CARGO_TARGET_DIR`, and it is worth a look
before the local half of the wiring is added.

## Verification

| Check | Result |
|---|---|
| `actionlint .github/workflows/ci.yml` | clean |
| `python3 -m py_compile scripts/ci/*.py` | clean |
| All 13 `scripts/ci/test_*.py` suites | **611 tests, 0 failures, 0 skips** — the post-spike-1-and-2 baseline held exactly |
| `python3 scripts/ci/test_build_key.py` with `BISCUIT_CI_BUILD_BIN` set, as `ci-tooling` will run it | 12 tests, OK, 0.084s |
| `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | 119 passed, 0 skipped |
| `scripts/target` moved aside and restored | restored: 7.7G, `debug/ci-build` at its original mtime, warm suite green |

`.githooks/tests/test-pre-push.sh` was not run: the suite list was not changed, and its
only mention of `ci.yml` is a comment (`:1581`), so nothing in it reads the edited
workflow.
