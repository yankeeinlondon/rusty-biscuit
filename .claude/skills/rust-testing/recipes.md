# Running Recipes

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## Fail-fast follows the cost of the next run

Fail fast locally (nextest's default): the first failure is usually enough to
act on. Run to completion where a rerun is expensive, because a truncated
report there costs another full round-trip to learn the second failure.

- **CI already passes `--no-fail-fast`** (`.github/workflows/_package-ci.yml`,
  `.github/workflows/_wsl-ci.yml`, `just/ci-local.just`). Do not add it.
- **Root `just test` passes it too**, as the broadest local scope. Root
  arguments are all consumed as selectors, so `just test claudine` inherits
  it and cannot override it. Inside a package area the arguments reach
  nextest, so an area `just test` fails fast and you can override per run.
- **Non-vacuous proofs at multi-package scope need the full list.** When you
  neuter a guard to confirm its tests go red, a truncated failure list looks
  like a narrow blast radius. Against a single package, fail-fast is fine.

## Restore corrupted sources with a fresh mtime

Cargo rebuilds only when a source is newer than its build output. A restore
that keeps the backup's older mtime (`cp -p`, Python's `shutil.copy2`) leaves
the corrupted build in place: `cmp` passes and every later run silently tests
broken code. Restore with a plain write, or `touch` the files afterward.

## Leaked process detection

Two layers catch tests that fail to reap child processes:

1. **nextest `LEAK`, per test.** `.config/nextest.toml` owns the
   `leak-timeout` values, and both profiles fail on a leak. Before changing a
   timeout, diagnose ownership, reaping, and inherited handles; any change must
   keep leak failure.
2. **`just test-leaks`, after the run.** Wraps `just test` in the
   `leak-sweep` binary (`tools/test-toolkit`, feature `leak-sweep`), diffs the
   process list before and after, and exits `99` on survivors whose executable
   or command line is under the repository. It catches detached orphans that
   closed the test's pipes, which `LEAK` cannot see.
