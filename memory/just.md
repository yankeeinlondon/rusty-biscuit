# Just Monorepo Learnings

## Using `mod` instead of `import` to avoid variable conflicts

When shared `.just` files (like `util.just`) define their own variables (e.g., `bold`, `reset`), importing them into another shared file with `import` causes duplicate-variable errors. Using `mod` creates a proper submodule namespace, avoiding the conflict entirely.

Example:

```just
# devops.just
mod util

# Recipes in devops.just can call util::_fzf_select
```

This is especially useful in a monorepo where multiple shared recipe files in `just/*.just` may define the same ANSI color variables or other helpers.

Note: `mod` was stabilized in just 1.31.0.

## Unscheduled Feature Priority Prefixes

Unscheduled features and fixes in `features/_unscheduled/` and `fixes/_unscheduled/` may be prefixed with a single digit priority: `{#}-{name}` where `1` is highest priority. The `just schedule` recipe will:
- Sort the fzf picker by priority (lowest number first)
- Strip the numeric prefix when moving to a scheduled `YYYY-MM-DD-{name}` directory

The `just status` recipe lists unscheduled features sorted by name, which respects the priority prefix order.

## `just` aborts a recipe at its first failing line — loops are not optional

`test *args:` written as a stack of `@just _test <pkg>` lines silently stops
testing at the first red package. Nothing in the output says the later packages
were skipped, so a failing package reads identically to a missing one. Any
recipe that must cover N packages needs a driver that loops, accumulates
failures, prints a summary, and exits non-zero at the end — see `_run_all` in
`just/devops.just`. Keep an `INT` trap that re-raises exit `130`, or
`just check-test-interrupts` breaks.

## A `just` parameter cannot hold a list, so encode one in a string

`just` has no array type. To pass "these packages, and give this one an extra
flag", pass a single string and split it in bash: whitespace for the plain case,
`;` when an entry needs its own arguments
(`_test_all "sniff --features remote; sniff-cli"`). Deciding the separator from
whether the string contains a `;` keeps the common call sites unadorned.

## An absolute `import` lets a throwaway justfile reuse the shared recipes

`import '/abs/path/to/just/devops.just'` works from anywhere, and the imported
file's own relative `import`/`mod` statements resolve against *its* directory,
not the importer's. That makes it possible to build a temp-dir cargo workspace
whose `justfile` drives the real shared recipes — the basis of
`tools/test-toolkit/tests/junit_staging_contracts.rs`.

Gotcha when generating such a justfile from a shell heredoc: `{{{{ args }}}}` is
`just`'s escape for a *literal* `{{`, so an unquoted heredoc that tries to emit
`{{ args }}` by doubling the braces produces a recipe that passes the literal
text `{{ args }}` to the shell. Write `{{ args }}` verbatim in a quoted heredoc
(`<<'EOF'`) instead.

## Recipe-line exit codes vanish once you add trailing output

`cargo nextest run … {{ args }}` as the last line of a `set -e` recipe
propagates its status for free, but the moment a trailing `echo` is appended the
status becomes the `echo`'s. Capture it (`code=0; cmd || code=$?`) and `exit
"${code}"` explicitly — and do the same when a post-step (report staging) has to
run after a *failing* command.

## A shared recipe reached by two paths needs an ownership flag to bracket once

`_test_l2` is invoked directly by most areas *and* indirectly by `_test_l2_all`
→ `_run_all` → `just _test_l2 <pkg>` for multi-package areas. Anything that must
happen once per *tier* (a `reset`/`verify` bracket, a lock, a report rollup)
cannot simply live in the per-package recipe: the loop would re-run it per
package, and in the `reset` case erase the earlier packages' evidence so the
verdict depended on whichever package went last.

The fix is an exported env var as an ownership claim. The driver sets
`export BISCUIT_BACKEND_PROOF_OWNER=1` before calling `_run_all`; the leaf
recipe brackets itself only when that variable is unset. `just` recipes are
separate processes, so an ordinary shell export is all the signalling needed,
and neither path has to know whether the other exists.

## `set -e` kills `(( cond )) && assignment`

A guarded assignment written as a compound command returns the status of the
`((…))` when the condition is false, which under `set -euo pipefail` aborts the
recipe:

```bash
(( code == 0 )) && code="${proof_code}"   # exits the recipe when code != 0
if (( code == 0 )); then code="${proof_code}"; fi   # correct
```

`(( … ))` inside an `if`/`while` *condition* is fine — `set -e` exempts those.
The trap only bites in statement position.

## A `finish()` function beats sprinkling `exit` when adding a post-step

`_test_l2` has three exit points (parallel mode, broker-missing fallback, shared
pane). Adding a post-run step to all of them means either duplicating it three
times or adding an `EXIT` trap — and an `EXIT` trap is already taken there by
the harness pane teardown, so a second one would silently replace it. Funnelling
every branch through one `finish "${code}"` helper keeps a single definition and
leaves the existing trap intact (it still fires, since `finish` ends in `exit`).

## A recipe guard can only see what its area list contains

`check-canonical` (root `justfile`) validated the canonical 12-recipe set for
whatever `areas := "…"` listed — and that string was a verbatim copy of the
`ci: true` records in `.github/ci/areas.json` (the policy store at the time;
CI policy now lives in each package's own `[package.metadata.ci]`). So the
guard whose whole job is
"catch a package area missing canonical recipes" was structurally incapable of
seeing any *excluded* area, which is exactly where missing recipes accumulate:
six areas sat at `ci: false` with "blocked on the canonical recipe set" as the
recorded reason, and nothing ever reported it. When a lint's scope is a
hand-maintained list, check that the list is not derived from the same fact the
lint is supposed to police.

`areas` is also consumed by `_orchestrate` (root `lint`/`sanity`/`build`/
`bench`/`coverage`/`fuzz`/`all`), `changed-areas`, and `install` — widening it
widens those too. `install` degrades safely (falls back to `build --release`,
then skips), so a library-only area needs no `install` recipe.

## `just --summary` enrolment is by file, not by list

`_check_test_interrupts` finds areas with `rg --files -g justfile`, so a NEW
package-area justfile is auto-enrolled in that gate the moment it exists — and
a nested one (`tools/test-toolkit/justfile` under `tools/justfile`) is scanned
too, but skipped because it defines no `test` recipe.

## Shared recipes with fixed parameters silently break on extra args

`_lint pkg:` takes exactly one parameter — no `*args=""`. Area justfiles that
call `just _lint {{ LIBRARY }} {{ args }}` work only while `args` is empty. If
an area needs feature flags at lint time, spell the `cargo clippy` line out
locally (messenger and biscuit-visualized both do) rather than passing them
through `_lint`. `_sanity`, `_test`, `_coverage`, and `_doctest` *do* take
`*args=""`, so `--features …` / `--all-features` forwards cleanly through those.

## `-p` and `--archive-file` cannot both reach `cargo nextest run`

A `*args` passthrough that forwards `--archive-file` verbatim still fails, because
the recipe's own `-p <pkg>` is one of the ~30 Cargo *build* flags nextest refuses
to accept alongside an archive (`--features`, `--release`, `--lib`, `--target`,
`--all-features`, … all of them). The binaries already exist; there is nothing
left to build or select at build time.

The fix is to move the package selection into the filterset, which the archive
*can* evaluate:

```bash
# normal
cargo nextest run -p "$pkg" -E "$filter"
# archive
cargo nextest run --archive-file … --workspace-remap … -E "package($pkg) & ($filter)"
```

and to strip build-only flags from the forwarded args in archive mode. Both
selections choose the same tests (verified: 92 either way). So "confirm the
passthrough empirically" is not paranoia — the flags arriving verbatim is
necessary but nowhere near sufficient.

## nextest's store dir follows `--workspace-remap`, not `--extract-to`

The JUnit report from an archive run lands under
`<remapped-workspace>/target/nextest/<profile>/`, **not** in the temporary
extraction directory. Anything that copies the report after the run can keep
using the workspace-derived target dir unchanged.

## An overridable tool invocation must be an *array*, and cover the probe too

`BISCUIT_NEXTEST_BIN` has to be two words (`cargo-nextest nextest`) to name the
standalone binary, so it cannot be a scalar:

```bash
read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"
"${nextest_bin[@]}" --version   # probe
"${nextest_bin[@]}" run …       # invocation
```

Overriding only the invocation and leaving `cargo nextest --version` as the
availability probe is worse than not overriding at all: the probe fails on a host
without cargo, the recipe takes its `cargo test` fallback, and the run dies with
`cargo: command not found` instead of the actionable error.

## `set -u` + empty array: use `${arr[@]+"${arr[@]}"}`

`"${arr[@]}"` on an empty array is an unbound-variable error under `set -u` in
bash 3.2 (still what `/usr/bin/env bash` finds on stock macOS). Every array built
from `{{ args }}` — which is routinely empty — needs the `+` guard, both when
expanding it and when passing it on.

## `just` accepts recipe arguments that start with `-`

`just _recipe --no-fail-fast --archive-file /x` passes all three through to a
`*args` parameter; the leading hyphens are not parsed as `just`'s own options.
That makes a small filter/transform recipe (`_archive_drop_build_flags`) a viable
way to share argument-munging logic between two recipes instead of duplicating a
bash function into both.

## Silent `exit 0` in a "never fail the build" helper is its own bug

`_stage_junit` must not change a test invocation's outcome, so it exits 0 when it
cannot find its paths. Exiting 0 *quietly* meant a passing run staged nothing and
the CI rollup scored the cell MISSING — indistinguishable from a job that never
ran. Keep the exit code, add an unmissable stderr block that names the
environment variables that would have prevented it. "Cannot fail the build" is
not a reason to be silent.
