# Running Recipes

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## Fail-fast is an environment policy, not a flag preference

Locally, fail fast — nextest's default, and the right one. The first failure is
usually enough to act on, and if more than one test is broken, fixing the first
surfaces the next. In CI, run to completion instead. The rule keys on **how
expensive the next run is**, which is what makes it a property of the
environment rather than a preference between flags: a truncated Windows or WSL2
report costs a full round-trip measured in hours to learn what the second
failure was.

**CI already passes `--no-fail-fast`** — in `.github/workflows/_package-ci.yml`,
`.github/workflows/_wsl-ci.yml`, and `just/ci-local.just`. Do not add it.

Root `just test` keeps the flag by explicit decision (2026-09-15,
`fixes/2026-09-14-cicd-improvements`). It is the repository's broadest local
scope and re-running it is expensive enough to sit on the CI side of the
cost-of-next-run test. **At the repository root** the choice is also not
reversible from the command line — every argument there is consumed as a
selector — so selector-narrowed invocations such as `just test claudine` inherit
the flag by design, not by oversight. Inside a package area the arguments reach
nextest, so an area `just test` keeps nextest's fail-fast default and you can
override it per run.

**Consequence for non-vacuous proofs.** Proving a guard fix non-vacuous — neuter
the guard, confirm the new tests go red, restore — needs the complete failure
list when the pass runs at CI-shaped or multi-package scope, because a truncated
list looks exactly like a narrow blast radius, which is the opposite of what the
proof is for. Against a single package locally, fail-fast is correct and faster.

**Restore corrupted sources with a fresh mtime.** Cargo rebuilds only when a
source is newer than its build output. A restore that keeps the backup's older
mtime (`cp -p`, Python's `shutil.copy2`) leaves the *last corruption's* binary
in place. A byte-exact `cmp` then passes, and every later run silently tests
broken code. Restore with a plain write or `touch` the files afterward. In
`fixes/2026-09-12-shadow-home` Phase 11 this turned a passing L2 test red 40
times out of 40 and made two full `just test-l2` runs unusable as evidence.

## Leaked Process Detection

Two complementary layers catch tests that spawn child processes and fail to
reap them:

1. **nextest `LEAK` (per test, all platforms).** `.config/nextest.toml` is the
   authority for the profile and per-test `leak-timeout` values. Both profiles
   keep `result = "fail"`: a pipe still held after the effective observation
   window fails the run. The window is not a fixed delay paid by every test.
   Diagnose resource ownership, child reaping, and inherited handles before
   changing it; any timing adjustment needs scoped evidence under load and
   must retain leak failure. Browser and parallel L2 processes may have
   different teardown costs. `#[serial(browser)]` cannot coordinate separate
   nextest processes; the browser recipe's runner-level serialization does.
2. **`just test-leaks` (post-run sweep, all platforms).** Wraps `just test` in
   `leak-sweep` (`tools/test-toolkit`, `--features leak-sweep`). It diffs the
   process list before/after the whole run and reports survivors whose
   executable or command line is under the repo (exit code `99`). Catches
   detached orphans that closed the test's pipes — which `LEAK` cannot see.
   Attribution is by workspace path, not parent PID (orphan reparenting is
   OS-specific).
