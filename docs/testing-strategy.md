---
title: Testing Strategy
status: living
audience: technical person but with no knowledge of this monorepo
created: 2026-05-24
updated: 2026-09-10
---

# Rusty Biscuit Testing Strategy

This document walks through how testing works in the Rusty Biscuit monorepo.
It assumes you know Rust and Cargo but nothing about this repository. It is
the single authoritative description of the approach; if another document or
a recipe disagrees with it, treat that as drift and fix one of them.

Two companion documents cover neighboring ground:

- `.claude/skills/rust-testing/SKILL.md` is the compact, agent-facing version
  of this material, with test-design rules and fixture patterns.
- `docs/topics/ci-cd.md` explains the GitHub Actions pipeline and the release
  process. This document says *what* runs and *why*; that one says *how* CI
  wires it up.

## The short version

Every package area in the repository answers to the same handful of commands.
From inside an area such as `darkmatter/`:

```bash
just sanity        # fast confidence check, seconds not minutes
just test          # the full ordinary test suite for the area
just lint          # clippy with warnings treated as errors
just all           # everything that gates a pull request, cheapest first
```

The intended loop while working on a change is:

1. Run `just sanity` often. It should finish in well under a minute.
2. Run `just test` and `just lint` before you consider the change done.
3. Run `just test-l2`, `just test-browser`, or the other opt-in suites only
   when your change touches behavior those suites cover.
4. Run `just all` before handing off anything non-trivial.
5. Run `just cross-check <package>` when the change touches path handling,
   process spawning, or terminal behavior, to exercise the operating systems
   your own machine is not.
6. Push. A pre-push hook validates the packages your change affects and hands
   the result to CI so CI does not repeat it.

Two principles shape everything below.

**All four platforms count equally.** macOS, Linux, native Windows, and WSL2
are supported to the same standard. None is the reference platform and a
failure on any one of them is a real failure.

**Evidence is produced once, in the cheapest place that can produce it.**
Hosted CI is the slowest and most expensive environment available, so it runs
only what a change actually affects and only what has not already been proven
somewhere cheaper. Coverage is not the thing being economized; duplicated work
is.

The rest of this document explains what each of those commands does, why the
suite is split into levels, how the less frequent activities such as
benchmarking, coverage, and fuzzing fit in, and how the same coverage is
reached across four operating systems without running everything everywhere.

## A few words you will see everywhere

The repository has its own small vocabulary. These terms are used precisely
throughout the rest of the document.

- **Package**: a Cargo package, the thing `cargo metadata` lists. Examples are
  `darkmatter` and `darkmatter-cli`.
- **Package area**: a top-level directory that groups related packages.
  Almost every area is a library plus a command-line tool, named `{name}` and
  `{name}-cli`, living in `{area}/lib` and `{area}/cli`. A few areas own more
  packages than that.
- **Curated area list**: the `areas` variable in the root `justfile`. Root
  commands that iterate "every area" use this list. It is a local
  orchestration convenience, not a complete manifest of the workspace, and CI
  does not read it.
- **Level** or **tier**: a category of test defined by what it needs from the
  outside world. Levels are explained in the next section.
- **Harness**: reusable infrastructure that makes an expensive resource
  testable, such as a real terminal window or a headless browser.
- **`just`**: the command runner used throughout the repository. Each area has
  a `justfile`, and shared recipes live under `just/`. You do not need to know
  `just` deeply to use the recipes in this document.
- **nextest**: the test runner. We use
  [cargo-nextest](https://nexte.st) rather than `cargo test` because it runs
  each test in its own process, supports name-based filtering, and produces
  machine-readable reports. Prefer the `just` recipes over invoking nextest
  directly; the recipes carry the filter expressions and environment setup.

## What kinds of testing we do

We use several distinct activities, each answering a different question.

| Activity | Question it answers | Runs where |
| --- | --- | --- |
| Lint | Does the code meet compiler and clippy hygiene expectations? | Every change, locally and in CI |
| Unit and integration tests | Does the code do the right thing? | Every change, split into levels |
| Doctests | Do the examples in rustdoc still compile and pass? | `just doctest`, and inside `just all` |
| Coverage | Which code did the tests actually exercise? | Locally, on demand, report only |
| Benchmarks | Did this change make a hot path faster or slower? | Locally, on demand, comparative |
| Fuzzing | Does a parser survive hostile input? | Nightly and on demand, never blocking |

Lint and the ordinary test suite gate pull requests. Coverage, benchmarks, and
fuzzing are signals that inform review; they never block a merge. There is
currently no load testing anywhere in the repository. That will be added if a
package ever needs it.

## Test levels

Rusty Biscuit is unusually heavy on command-line and terminal-rendering code.
Much of what matters cannot be verified from inside a plain Rust test: whether
a terminal emulator renders a glyph at the right width, whether a browser
computes the expected style, whether the operating system delivers a key press
the way the program expects. The suite is therefore split into levels by what
each test needs from the environment.

| Level | Name prefix | What it covers | Default behavior |
| --- | --- | --- | --- |
| Level 1 (L1) | none | Ordinary in-process tests, including tests that spawn the package's own binary or drive it through a pseudo-terminal. No real terminal, browser, device, or network. | Always runs. This is what `just test` means. |
| Level 2 (L2) | `level2_` | Tests that run inside a real terminal emulator or multiplexer (tmux, WezTerm, Kitty, Apple Terminal) and read back what it rendered. | Skips cleanly when no terminal harness is available. CI can require specific backends. |
| Level 3 (L3) | `level3_` | Tests that inject real keyboard or mouse events at the operating-system level, so the terminal's own input encoder fires. | Always skipped unless `RUN_LEVEL3=1`. |
| Browser | `browser_` | Headless Chrome tests through the browser harness. | Skips cleanly when Chrome is absent; CI can require it. |
| Real | `real_` | Tests against real devices, networks, or provider APIs. | Skipped unless the relevant per-package environment variables opt in. |
| Slow | `slow_` | Otherwise ordinary tests that exceed the time budget for fast runs. | Left out of `sanity` and of local `just test`; CI includes them for packages that ask. |

Why three numbered levels for terminal code? Level 1 can spawn a program in a
pseudo-terminal and feed it bytes, but *we* manufacture those bytes, so Level 1
cannot notice that a real terminal never sent them. Level 2 puts the program
inside a real terminal and captures the rendered pane, which proves that
glyphs, widths, styling, and scrolling survive a real emulator. Input is still
injected as bytes through the terminal's command-line interface, so the
terminal's own key encoding is not exercised. Only Level 3 presses real keys,
which is the only way to answer "what does the terminal emit when the user
holds Ctrl?" Level 3 needs window focus, which is fragile and
platform-specific, so it is strictly opt-in.

Level 2, Level 3, and browser tests must never steal foreground focus or close
a window they did not open. A test that grabs focus makes the developer's
machine unusable during a run and is not portable to CI.

### Naming is the mechanism

Nextest selects tests by name, so the prefixes in the table are not a
convention, they are the switch. A test function named `level2_renders_table`
is a Level 2 test; the same body named `renders_table` would run as Level 1
and fail on machines without a terminal harness. The prefix may appear on the
function name or on an enclosing module, so a whole `mod level2_tests` is
selected as one unit.

The canonical filter expressions are:

```text
level2  = test(/(^|::)level2_/)
level3  = test(/(^|::)level3_/)
browser = test(/(^|::)browser_/)
real    = test(/(^|::)real_/)
slow    = level2 + level3 + browser + real + test(/(^|::)slow_/)
```

Nextest does not yet support user-named filterset aliases as a stable feature,
so these expressions are passed to `cargo nextest run -E '...'` directly by the
shared recipes in `just/devops.just`. The header of `.config/nextest.toml`
repeats them as documentation; that file itself holds only retry, timeout, and
leak-detection settings.

### Sanity: the fast subset

`just sanity` runs the Level 1 tests of the area's library and binary crates
with every other tier excluded, and it skips doctests because compiling them
is noisy and slow. The budget is about fifteen seconds per package. It exists
so a developer can get a broad "did I break anything obvious?" answer many
times an hour.

Two consequences follow. Every new test should be considered for whether it
belongs in sanity, and a test that turns out to be too slow should be renamed
with `slow_` so it drops out, ideally with a faster test covering the same
ground. A green sanity run is a preliminary signal, never a substitute for
`just test`.

### Where slow tests run

The Level 1 filter used by `sanity` and by local `just test` excludes `slow_`
tests. That keeps the local loop fast, but it means a slow regression test is
not exercised locally unless you run it by name. In CI, a package can declare
that its Level 1 leg includes slow tests, which sets `BISCUIT_L1_INCLUDE_SLOW=1`
for that leg; Darkmatter does this. When you write a `slow_` test, make sure
the package's CI policy actually runs it.

## How a test decides to skip or fail

Tests above Level 1 need a resource that may not exist on the current machine.
The rule is: skip cleanly by default, fail hard when the environment says the
resource is required. This is implemented by the `require_level!` macro in
the `tools/test-toolkit` crate, called at the top of the test body:

```rust
use test_toolkit::{Level, require_level};

#[test]
fn level2_renders_in_terminal() {
    require_level!(Level::L2, harness_is_available(), "terminal harness");
    // test body
}
```

The third argument names what is required. It is usually a backend identity
such as `Backend::Tmux`, and may instead be a plain label such as
`"WezTerm + cliclick"` for composite requirements no single backend describes.

The environment variables that drive the decision:

| Variable | Purpose |
| --- | --- |
| `BISCUIT_TEST_LEVEL=1\|2\|3` | Ceiling. Tests above this level skip. Defaults to 3, meaning run everything a harness is available for. |
| `BISCUIT_TEST_LEVEL_REQUIRED=2\|3` | A missing harness for that level panics instead of skipping. All-or-nothing, so prefer the next variable. |
| `BISCUIT_TEST_REQUIRED_BACKENDS` | Comma-separated list such as `tmux,wezterm`. The named backends must be present and must run at least one test; the rest still skip. |
| `BISCUIT_BROWSER_REQUIRED=1` | Missing Chrome panics instead of skipping. |
| `RUN_LEVEL3=1` | Explicit opt-in for Level 3. |
| `BISCUIT_JUNIT_STAGE_DIR` | Where JUnit reports and execution evidence are staged. Defaults to `target/nextest/ci-reports`. |

Older per-package variables such as `DARKMATTER_LEVEL2_REQUIRED` were removed.
Use the `BISCUIT_*` set exclusively.

### Availability is not execution

An installed `tmux` plus zero tmux tests proves nothing. When
`BISCUIT_TEST_REQUIRED_BACKENDS` is set, every gate decision is appended to a
JSON Lines evidence file, and the `test-l2` recipe brackets the run with a
`backend-proof` reset before and verify after. Verification fails the tier if
a required backend produced no executed test. Nothing is recorded when the
variable is unset, so local runs pay no cost.

### Serialization and fixtures

Mark a test with `#[serial_test::serial]` when it mutates process-global
state, shares a harness, sets environment variables, or needs exclusive access
to a real terminal. Use `test_toolkit::EnvGuard` for environment setup so
cleanup happens even when the test panics. Browser tests should assert
computed results such as computed CSS values or DOM state, not substrings of
the page source.

## Running tests with `just`

### The canonical recipe set

Every area in the curated list exposes the same twelve recipes. When a recipe
does not apply to an area it is still present, as a one-line no-op that
explains why, so root-level orchestration can iterate every area without
special cases.

| Recipe | What it does |
| --- | --- |
| `sanity` | Fast Level 1 subset, library and binary crates only. About fifteen seconds. |
| `test` | Full Level 1 suite for the area. Slow-tagged tests excluded locally. |
| `test-l2` | Real-terminal tests. |
| `test-l3` | OS keyboard and mouse tests. Needs `RUN_LEVEL3=1`. |
| `test-browser` | Headless browser tests. |
| `test-real` | Tests against real devices or services. |
| `lint` | `cargo clippy` with warnings as errors. |
| `bench` | Criterion benchmarks, one run, no comparison. |
| `coverage` | Per-package LCOV report via `cargo llvm-cov`. |
| `doctest` | `cargo test --doc`. |
| `fuzz` | `cargo +nightly fuzz run` targets. |
| `all` | `sanity`, then `lint`, `doctest`, `test`, `test-l2`, `test-browser`, in that order. |

`all` is ordered so the cheapest signal fails first. Tiers that skip cleanly
when their resource is absent do not fail `all`. Benchmarks, fuzzing, Level 3,
and real-resource tests are deliberately left out of `all` because each needs
an explicit opt-in: a quiet CPU, a nightly toolchain, window focus, or a
device.

`just check-canonical` from the root verifies that every curated area exposes
all twelve recipes and names any that are missing. CI runs the same check. A
new area must pass it before it joins the curated list.

### Root-level commands

From the repository root the same names fan out across areas:

```bash
just sanity                     # every curated area
just sanity darkmatter claudine # named areas only
just test                       # every workspace package
just test biscuit-file          # one package, or every package under an area path
just lint
just coverage biscuit-terminal
just bench sniff
just all
```

Root `just test` is the one exception to area iteration. It discovers every
workspace package from `cargo metadata`, resolves each package's declared
features, and hands all of them to a single nextest invocation with
`--no-fail-fast`, so one scheduler sees every test binary and one failure
cannot hide the rest. Ctrl+C stops it immediately with exit code 130, and
`just check-test-interrupts` verifies that contract for every area.

### How an area's justfile is built

Areas do not reimplement test mechanics. The shared file `just/devops.just`
holds private template recipes, prefixed with an underscore, that own the
nextest invocation, the filter expressions, timing output, and skip behavior.
An area's public recipe only decides scope: which packages it owns.

```just
sanity:
    @just _sanity darkmatter
    @just _sanity darkmatter-cli

test *args="":
    @just _test darkmatter {{ args }}
    @just _test darkmatter-cli {{ args }}
```

The high-traffic templates are `_sanity`, `_test`, `_test_l2`, `_test_l3`,
`_test_browser`, `_test_real`, `_lint`, `_doctest`, `_bench`, `_coverage`,
`_fuzz`, and `_all`. Areas that own more than the usual pair simply call the
template once per package. Schematic owns four, Unchained AI owns five, and
Homelab owns seven: a library, a CLI, a server, and four device-integration
crates, plus a Vue front end driven by its own `test-frontend` recipe.

Some workspace members are not in the curated list on purpose. Generated code
under `schematic/schema` is excluded from the workspace entirely and rebuilt by
`just generate`. The `so-you-say` binary is not an area; it ships from
`biscuit-speaks/cli`, which owns its lifecycle.

### What `test-l2` does behind the scenes

Spawning a terminal costs two to three seconds. Rather than pay that per test,
the `test-l2` recipe starts one shared pane per available backend before
nextest runs, exports the pane identifiers through `BISCUIT_SHARED_*`
variables, and runs nextest single-threaded. Tests attach to the shared pane
when the variable is set and otherwise spawn their own background pane. The
recipe tears the panes down when nextest exits. A backend whose tooling is
missing on the host is silently skipped at spawn time, and its tests then skip
through `require_level!`.

## Linting and formatting

`just lint` runs `cargo clippy -p <package> -- -D warnings` for each package
the area owns. No work is complete while lint warnings remain. Lint and sanity
answer different questions and are kept separate, which is why `all` runs
sanity first: a cheap behavioral failure should surface before clippy output.

Formatting is a different matter. There is a preferred style, enforced by the
root `rustfmt.toml`, but developers should never run `cargo fmt` as part of
ordinary work. Reformatting produces commits with no semantic content and
buries the real change. Instead the repository is reformatted periodically in a
dedicated commit. Do not use lint fixes as an excuse for formatting churn, and
keep each fix scoped to the warning at hand.

## Coverage

`just coverage` runs `cargo llvm-cov -p <package> --lcov` for each package the
area owns and writes `lcov-<package>.info` next to the area. The root recipe
iterates the curated areas rather than running one workspace-wide command.

Coverage is report-only. Pull requests are not failed on a coverage
percentage, because percentage gates are easy to game and produce false alarms
during legitimate refactors. Coverage is still valuable for finding untested
public paths, checking that a bug fix came with a regression test, spotting
accidental coverage loss on risky code, and giving a reviewer context when a
change claims to be test-backed.

Doctests are not included unless you add them to the invocation, and Level 2,
Level 3, browser, and real-resource tests will be absent from a local report
whenever the host cannot run them. CI stopped producing coverage on
2026-08-12: the nightly job ran an un-tiered workspace-wide `cargo test` that
aborted on any known-red test before writing a report, so it burned a slot and
produced nothing.

### CRAP scores

A CRAP score (Change Risk Anti-Patterns) combines a function's cyclomatic
complexity with its test coverage to flag code that is both complicated and
poorly tested. The `cargo-crap` tool computes it from an LCOV file. The
analysis itself takes seconds; the expensive part is producing the LCOV file,
which needs an instrumented rebuild and a full test run.

Because of that cost shape, CRAP belongs where slow feedback is acceptable,
never in the interactive loop or on the pull-request path. It is not yet wired
into any `just` recipe. When it is, the coverage and analysis scopes must
match: run `cargo llvm-cov -p <package>` and then `cargo crap --path
<package-dir> --lcov <that-file>` per package, and reserve `cargo crap
--workspace` for LCOV produced by a workspace-wide run. The planned guardrails
are a cyclomatic-complexity ceiling of 10, which coverage cannot dilute, and an
advisory CRAP threshold of 30 that prompts review rather than blocking. CRAP is
noisy on generated code, proc-macro crates, examples, and test harnesses, so
those are excluded through the tool's own filters rather than by adjusting the
formula.

The methodology and known blind spots are written up in
`docs/research/cargo-crap.md`.

## Performance testing

Benchmarks answer a different question from tests. A test asks "is this
right?" and gives a pass or fail. A benchmark asks "is this change faster or
slower?" and is only meaningful as a comparison: the same machine, the same
tool, before and after.

We use [Criterion](https://github.com/bheisler/criterion.rs). It runs a
small piece of code thousands of times, computes statistics, and writes the
result under `target/criterion/`. On the next run it compares against saved
data and reports improved, regressed, or no change, with a confidence
interval and a p-value. Benchmark sources live in each package's `benches/`
directory; you do not need to author one to run one.

### The workflow

Suppose you are about to change Darkmatter's Markdown parser.

```bash
cd darkmatter
just bench-save        # 1. on the merge base: save today's numbers as the baseline

# 2. make your change

just bench-compare     # 3. Criterion prints improved / regressed / no change per bench

# 4. paste the comparison block into the pull request
```

You never choose a baseline name; both recipes derive one from the machine.
Two more recipes cover specific needs. `just bench` runs once with no baseline
and no comparison, useful as a smoke test or for a quick absolute number.
`just bench-id` prints the derived baseline name so you can cite it in a
pull request.

Rollout status: the shared templates for all four recipes exist in
`just/devops.just`, but only Worktree has wired the public `bench-save` and
`bench-compare` wrappers so far, and the root `justfile` orchestrates only
`bench`. If an area reports "unknown recipe", it still needs its wrappers
added.

### Baselines are per host

The baseline name has the shape
`{host8}--{arch}--{os}--{cores}c--{mem}g`, for example
`4ef2e814--arm64--macos--16c--128g`. The first field is a short hash of a
stable machine identifier and the rest come from the `sniff` host-discovery
tool. The effect is that a baseline saved on your laptop can never be
compared against numbers from a Linux VM, because the names differ. Each
machine keeps its own independent history. There is no cross-host
aggregation; if you need to know whether a change regresses on all three
operating systems, run the comparison on each and paste all three results.

Adding memory or changing hardware changes the name, which orphans the old
baseline. That is correct: the old numbers came from a materially different
machine.

### Preflight checks

Before running, `bench-save` and `bench-compare` inspect the host and warn if
the machine is on battery, has less than thirty percent memory available, or
has a one-minute load average above half the core count. In an interactive
terminal you are prompted to continue or quit. Under `CI=true` or
`BENCH_NONINTERACTIVE=1` the recipe refuses. `BENCH_YES=1` overrides in either
case and logs that it did; if you use it, say so in the pull request.

### Reading and reporting results

A comparison line looks like this:

```text
parse/markdown/large
                        time:   [11.234 ms 11.456 ms 11.689 ms]
                        change: [-8.21% -6.74% -5.31%] (p = 0.00 < 0.05)
                        Performance has improved.
```

The `time` line is the current measurement with its confidence interval. The
`change` line is the delta against the baseline; negative is faster. If the
p-value is above 0.05 Criterion reports no change regardless of how dramatic
the percentage looks. Trust the verdict, not the raw percentage.

A good pull-request note has four parts: one sentence on what changed and why
it should affect performance, the host name from `just bench-id` for each
machine used, the Criterion block pasted verbatim, and any preflight warning
that fired with the reason you proceeded. Do not paste a single absolute
number, a screenshot of the HTML report, or numbers from two hosts as if they
were comparable.

### When to bench

Run benchmarks when a change plausibly touches a measured path: parsers,
serializers, render pipelines, filesystem walks, allocation-heavy code, or
async dispatch. Skip them for renames, documentation, comment changes, and
trivial fixes that do not alter an algorithm. A full run for a large area can
take several minutes.

### Housekeeping

Baselines live under `target/criterion/`, so `cargo clean` deletes them and
two checkouts on one machine have independent stores. Saving again under the
same derived name overwrites the previous baseline. To keep a historical
baseline, such as one for a release tag, call `cargo bench` directly with an
explicit `--save-baseline` name. To reset one bench on the current host:

```bash
rm -rf target/criterion/<bench-name>/$(just bench-id)
```

### Declaring what an area benchmarks

Each area that participates in performance testing describes its measured
surfaces in `{area}/docs/performance-testing.md`. The body names the core
blocks of functionality worth benchmarking, one H2 heading per block, with the
assumptions each measurement relies on and what is intentionally excluded.
Worktree's document is the reference example.

An area may opt out, temporarily or indefinitely. Brand-new areas usually
should, because benchmarking code that is still changing shape wastes effort.
A package that has no measurable hot path declares
`[package.metadata.benchmarks] required = false` in its `Cargo.toml`, and its
`bench` recipe becomes a documented no-op. The opt-out is enforced by reviewer
judgment, not by a checker.

## Fuzz testing

Fuzzing pushes a large volume of random, often malformed, input through a
parser or decoder and keeps any input that reaches new code or causes a
failure. It is the right tool for anything that accepts data from outside the
process boundary: file-format readers, serializers, protocol handlers, and
security-sensitive code.

We use `cargo-fuzz`, which needs nightly Rust. Fuzz targets live in
`<package>/fuzz/`, currently under `biscuit-file/lib` and `darkmatter/lib`.
Each directory contains its own `Cargo.toml`, a `rust-toolchain.toml` pinning
nightly, one binary per target under `fuzz_targets/`, a small hand-curated
`corpus-seed/` committed to the repository, and `crashes/<target>/` holding
minimized crash inputs committed as regression fixtures. Only seed and crash
inputs are committed, which avoids a Git LFS dependency.

```bash
cd biscuit-file/lib/fuzz
cargo +nightly fuzz run pdf_extract -- -runs=1000

cd darkmatter/lib/fuzz
cargo +nightly fuzz run markdown_parser -- -runs=1000
```

Add a fuzz target when all three hold: the code accepts external data, a
crash or hang in it is a real defect, and its surface is stable. The remaining
priority candidates are Claudine's hook JSON, Tree Hugger's queries, and
Schematic's schema definitions.

Fuzzing runs nightly in CI through its own advisory workflow. It is never part
of `sanity`, `test`, or any pull-request gate, because it needs nightly Rust
and long wall-clock time. Areas without fuzz targets have a no-op `fuzz`
recipe.

## Where tests run: operating systems and CI

macOS, Linux, native Windows, and WSL2 are equally supported. No one of them
is the reference platform and none is a second-class target. Every package is
expected to compile and behave correctly on all four, and a failure on any one
of them is a real failure.

That principle says nothing about *where* the evidence has to be produced.
Running everything everywhere on every push is the expensive way to honor it,
and it is not what we do. The rest of this section explains how the same
coverage is reached for much less compute.

### The economics problem

Hosted CI is the slowest and most expensive place to learn anything. A runner
must be allocated, the repository checked out, a toolchain installed, and the
dependency graph compiled before a single assertion runs. Compiling the test
binaries dominates: it is most of the cost of any leg, and it is paid again in
every parallel job that cannot share a build cache. A full-workspace run across
four environments takes hours, and because a push to a branch cancels the
previous run, a long feedback loop also means fewer complete answers per day.

So the strategy is not "run less" but "run each thing exactly once, in the
cheapest place that can produce real evidence." Three levers do the work:
narrow the selection, move what can be moved to the developer's machine, and
never repeat what has already been proven.

### Lever one: select narrowly, per gate

`scripts/ci/affected_scope.py` is the single deterministic calculator used by
both CI and the local hook, so both agree on what a change affects. Its policy
is deliberately tight:

- A package that owns a changed source file gets its configured validation:
  lint, compile check, Level 1, and any higher tiers it declares.
- An unchanged direct reverse dependency gets a compile check and nothing
  else. It is not linted and not tested.
- Ordinary dependencies and transitive reverse dependencies are not selected
  at all.
- Documentation, manifests, lockfiles, `just` recipes, and CI configuration
  schedule no package jobs. The CI tooling has its own compact contract tests.
- `workflow_dispatch` is the one explicit full-workspace path. Uncertainty is
  not a reason to trigger one.

Selection is also **per gate**, which matters more than it sounds. The three
gates are lint, check, and test, and a shared input widens only the gates it
can actually change. Editing `clippy.toml` widens lint alone. Editing
`.config/nextest.toml` widens test alone. Editing a `just` file widens a gate
only when the recipe that changed is reachable from that gate's entry recipe,
so touching the `pre-push` or planning recipes gates nothing. A package
selected for lint alone schedules exactly one job: no test tiers, no
environments, no compile-check leg.

### Lever two: prove the local platform locally

Your development machine is already warm, already checked out, and already has
a populated target directory. It is the cheapest environment in the system, and
it is one of the four supported platforms. Using a hosted runner to rediscover
what your own machine could have told you is pure waste.

The versioned `.githooks/pre-push` hook therefore runs `just ci-local --l2`
before a push completes. That is the same scope calculation and the same gate
recipes CI would use, applied to the source-changed packages: clippy, Level 1,
a compile check for direct reverse dependencies, and every Level 2 suite the
host can run without taking window focus. Apple Terminal and Level 3 are
excluded for that reason. A documentation-only push gates nothing, exactly as
in CI.

`RUSTY_BISCUIT_PRE_PUSH` controls the hook: `strict` blocks the push on
failure and is the default, `warn` runs and reports without blocking, and
`off` skips it. `RUSTY_BISCUIT_PRE_PUSH_AREAS` replaces the computed scope
with a fixed selection.

For a clean outgoing tree the hook publishes a Git-note receipt naming the
environment it validated, which `sniff` detects as macOS, Linux, native
Windows, or WSL2. CI verifies that receipt against the scope it expects for
the exact commit and tree, and then drops that environment's duplicate work.
The other three environments still run. If the receipt is missing, stale,
dirty, bypassed, or does not match, CI simply runs that environment as usual:
every failure mode adds work rather than removing it.

Run `just ci-local` yourself before pushing anything substantial. When the
scope is large, run `just ci-local --lint-only` first: it takes minutes rather
than tens of minutes, and on one recent occasion it would have caught four of
seven consecutive CI failures on its own.

### Lever three: reach the other platforms without a runner

A developer on macOS can still produce genuine Linux, Windows, and WSL2
evidence without waiting on CI. `just cross-check <package>` runs a package's
Level 1 suite against your local working tree on standing clones on real build
hosts, reached over SSH and declared by the `BUILD_LINUX`, `BUILD_WIN`,
`BUILD_WSL`, and `BUILD_MACOS` environment variables. An unset variable means
that host is unavailable from this machine. `--os` picks one platform and
defaults to every declared platform except your own. The WSL2 path deliberately
uses the same prebuilt nextest archive mode CI uses, so a failure reproduces
faithfully.

No commit and no push is needed. This is the expected step for a change that
touches path handling, process spawning, or terminal behavior, because those
are exactly the areas your local Level 1 run cannot exercise. A cross-compile
check is compile evidence only; say which kind you have.

CI is the final proof, not the discovery loop. Surface an operating system's
exact failure on the matching host first, then push once.

### What is in flight

The receipt mechanism is being extended as this is written. The agreed model
separates two claims: **scope evidence**, which records what the calculator
selected for an exact base, head, and tree, and **validation evidence**, which
records the outcome of each completed package, environment, and tier cell. The
target behavior lets CI adopt a matching local scope without recalculating it,
reuse a completed local run whether it passed or failed, exclude individual
proven cells rather than a whole environment, and replaces `off` with a
`scope-only` mode that publishes scope while leaving every test cell enabled.
Until the hook, schema, workflow, rollup, tests, and documentation land
together, the behavior described in the previous section is what actually runs.
The specification is `fixes/2026-09-10-local-affected-scope/spec.md`.

### The platform matrix

With selection settled, this is what a *selected* package runs where. Read
"full" as the complete suite for that one package, never as a workspace-wide
run.

| Tier | Linux | Windows | macOS | WSL2 |
| --- | --- | --- | --- | --- |
| Compile check, all targets | via test job | dedicated job | via test job | no |
| Level 1 | full | full | full | full, from a prebuilt archive |
| Level 2 | yes, tmux | policy gap | yes, tmux | policy gap |
| Browser | yes | policy gap | policy gap | policy gap |
| Level 3 | opt-in | opt-in | opt-in | no |

A **policy gap** is a cell that is deliberately, visibly, and temporarily not
covered by hosted CI, rather than a green cell that ran zero tests. Each one is
registered in `.github/ci/environments.json` with a reason, a named owner, and
an expiry date. The reasons are concrete: tmux has no Windows port, and
WezTerm, Kitty, and Apple Terminal need a live desktop session no runner
provides. Those packages' Level 2 suites are covered by local `test-l2` runs
instead, which is why the gap is a scheduling decision rather than a coverage
hole. Do not close a gap by amending an acceptance criterion; record the
criterion as unmet and provisioning as the required change.

Other things worth knowing about the matrix:

- **Windows runs the full Level 1 suite for a selected package** because it is
  the platform most prone to silent API and type drift that only appears at run
  time. Windows-only tests stay behind `#[ignore]` or a `level3_` prefix.
- **WSL2 follows Linux code paths** and is never evidence for native Windows.
  It runs from an archive built on Linux, which is why anything resolved at
  compile time to a builder path breaks there and nowhere else.
- **The all-targets compile check runs on Windows** and nowhere else. It is the
  only leg that compiles benches and examples. It does not deny warnings; the
  lint job does.
- **Only the lint job treats warnings as errors.** Setting `-D warnings` on
  test legs once made a plain rustc warning fail the build before any test ran,
  and on the check job it blamed a dependency's warning on whichever package
  built it. The lint recipe passes `-D warnings` to clippy directly, so the
  same bar applies locally.
- **Lint does not gate Level 1.** Compile, lint, and test are independent gates
  per package. One clippy hint used to erase every Level 1 result for a whole
  directory of packages, which is how Claudine's Windows tests went unrun for a
  long time. Only the expensive Level 2 and browser tiers wait for Level 1.
- **Level 2 in CI requires backends by name.** CI never sets
  `BISCUIT_TEST_LEVEL_REQUIRED=2`, which would panic on GUI backends a headless
  runner cannot host. It provisions tmux, verifies it, and sets
  `BISCUIT_TEST_REQUIRED_BACKENDS` to the package's declared backends
  intersected with what was provisioned, so an installed-but-unused backend
  fails the execution-proof check.
- **A test that passes only on retry is a failed test.** Both the CI and local
  profiles set `retries = 0`, so a deterministic failure runs exactly once. CI
  marks a test slow at thirty seconds and kills it at ninety, which is the line
  between "slow under contention" and "hung".
- **Every configured Level 1 leg gates.** There is no `continue-on-error` on
  any package gate. A previous "soft" mechanism did not merely make a leg
  non-blocking, it removed the leg from the verdict, so fourteen permanently
  red Windows directories read as a normal run. A known failure is recorded in
  the results baseline instead, which keeps it counted and visible.
- **Sharding was removed.** Compiling the test binaries is most of a shard's
  cost and every shard pays it in full, so four shards cost roughly three times
  the compute to save a couple of minutes. Level 1 runs with `--no-fail-fast`
  so one failure cannot suppress the rest of the evidence.
- **Lost runners are attributed, not read as regressions.** When a hosted
  runner dies mid-step, GitHub kills the job after about forty-five minutes
  with no log. The verdict step synthesizes a status naming the interrupted
  step so the grid reads as infrastructure loss, and a follower workflow reruns
  the failed jobs once, only when every failure was a lost runner. A real test
  failure is never retried.
- **Compare timings within one environment only.** macOS has the fewest cores,
  Windows the slowest build, and WSL2 the slowest execution. Run-to-run noise
  is five to fifteen percent per leg, so a delta under fifteen percent is not a
  regression, and three consecutive green runs is the evidence standard.

### Feature-gated code is invisible to the matrix

`cargo check --all-targets` resolves only a package's default features. Code
behind an off-by-default feature compiles on no platform unless a step names
the feature, so the matrix can stay green precisely because it never builds the
code in question. When a feature gates real code, add it to that area's compile
check. Sniff's `remote` feature is the live example: its CI policy adds
`remote` to the compile check, and its `just test` recipe runs the provider
suites with the feature enabled rather than merely compiling them.

### Toolchain

`rust-toolchain.toml` pins an exact Rust version with the clippy and rustfmt
components, so local and CI builds are identical. It is the only place the
version is written; `just toolchain` shows the pin beside the current stable
release and `just toolchain-upgrade` advances it. CI materializes the pin with
`rustup show` and never overrides it. A separate scheduled, advisory workflow
tests the latest stable toolchain and runs `cargo fmt --check` so drift is
visible without blocking anyone.

## Decision log

| Decision | Rationale |
| --- | --- |
| Runtime `require_level!` macro rather than a proc-macro attribute | Avoids a proc-macro crate and compile-time cost. Skip-versus-fail behavior is visible in the test body. |
| `sanity` excludes doctests | Doctest compile cost would blow the fifteen-second budget. |
| `all` runs sanity, lint, doctest, test, test-l2, test-browser in that order | Cheapest signal fails first. |
| Level 3, real-resource, fuzz, and bench are outside `all` | Each needs an explicit opt-in: focus, devices, nightly Rust, or a quiet CPU. |
| Coverage is report-only, and local-only since 2026-08-12 | Percentage gates create perverse incentives; the CI job produced nothing. |
| Fuzz corpus committed as seed plus minimized crashes only | Avoids Git LFS. |
| tmux is the default Level 2 backend | Headless and available on every CI runner. |
| `[package.metadata.benchmarks] required = false` is the bench opt-out | Grep-able and enforced by reviewer discretion. |
| Windows runs full Level 1 for a selected package | Highest-risk platform for silent runtime drift; compile-only would miss it. |
| One deterministic scope calculator shared by CI and the pre-push hook | Two calculators would disagree, and disagreement always resolves toward running more. |
| Direct reverse dependencies get a compile check, never tests | Their behavior did not change. Testing them re-runs unaffected suites for no signal. |
| Scope is resolved per gate, not per package | A `clippy.toml` edit cannot change test results, so it must not schedule tests. |
| Local pre-push evidence removes that environment's CI work | The developer's machine is one of the four supported platforms and is already warm. Rediscovering its result on a hosted runner buys nothing. |
| Every evidence failure adds CI work rather than removing it | A missing, stale, or unverifiable receipt must never be read as a pass. |
| Local evidence never substitutes for another operating system | macOS cannot stand in for Linux, and WSL2 cannot stand in for native Windows. |
| Platform gaps are registered with an owner and expiry, not hidden | A green cell that ran zero tests is worse than an honest gap, because nobody schedules work against it. |
| `workflow_dispatch` is the only full-workspace run | Uncertainty is not a reason to spend hours of compute. |
| Retries are zero in CI and locally | A pass-on-retry hides timing and contention defects. |
| Cargo artifacts cached with `Swatinem/rust-cache`, no rustc wrapper in CI | The `kache` wrapper measured 0 to 6 percent hit rates in CI and was removed on 2026-07-30; it remains a per-host developer opt-in governed by `docs/kache-strategy.md`. |

## Where to look next

- `.claude/skills/rust-testing/SKILL.md` for test-design rules, the tier
  decision tree, and fixture patterns.
- `docs/topics/ci-cd.md` for the pipeline, release process, and how to add a
  workflow.
- `tools/test-toolkit/src/lib.rs` for `Level`, `require_level!`, and the
  environment contract.
- `biscuit-test-harness/README.md` for the terminal harness backends and
  shared panes.
- `biscuit-browser-harness/README.md` for the browser harness.
- `just/devops.just` for the shared recipe templates and the canonical-recipe
  check.
- `.config/nextest.toml` for retry, timeout, and leak-detection settings.
- `docs/research/cargo-crap.md` for the CRAP methodology.
- `worktree/docs/performance-testing.md` as the model performance contract.
- `.claude/skills/os/SKILL.md` for which host can produce which operating
  system's evidence, and the traps specific to each.
- `scripts/ci/affected_scope.py` for the scope calculator itself, and
  `just/ci-local.just` for the local stand-in for the CI gates.
- `.github/ci/environments.json` for the registry of platform capabilities and
  policy gaps, each with an owner and an expiry.
