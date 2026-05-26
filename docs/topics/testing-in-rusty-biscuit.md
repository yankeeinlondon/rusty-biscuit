# Testing in Rusty Biscuit

## Using the `just` Infrastructure

We use [just](https://just.systems/) in all of the package areas (and root) of Rusty Biscuit to automate all of the common operations and that includes testing (as well as lint testing).

The root `justfile` is the normal entry point when you want to run a lifecycle command across the curated package-area list:

```bash
# Fast L1 subset — excludes slow, L2, L3, browser, and real-resource tests
just sanity
# Full Level-1 test suite via cargo nextest
just test
# Clippy and formatting checks
just lint
# LCOV coverage report via cargo llvm-cov
just coverage
# Canonical PR-gate sequence: sanity → lint → doctest → test → test-l2 → test-browser
just all
```

Those root recipes iterate the curated `areas` list in the root `justfile`; they do not imply every Cargo workspace member is covered. The authoritative package list is still `cargo metadata --no-deps --format-version 1`, and the authoritative package-area inventory is `sniff repo`.

Most root lifecycle recipes also accept area names when you want a narrower run:

```bash
# Run sanity for specific package areas
just sanity darkmatter claudine
# Run L1 tests for a specific package area
just test biscuit-file
# Generate coverage for a specific package area
just coverage biscuit-terminal
# Run Criterion benchmarks for a specific package area
just bench sniff
```

Inside a package area, use the same recipe names for local work:

```bash
cd darkmatter
# Fast L1 subset for quick feedback
just sanity
# Full Level-1 test suite
just test
# Level-2 terminal/PTY tests via biscuit-test-harness
just test-l2
# Headless browser tests via biscuit-browser-harness
just test-browser
# Criterion performance benchmarks
just bench
# LCOV coverage report
just coverage
```

The intended loop is:

- `just sanity` for frequent fast feedback.
- `just test` for the full Level-1 package test suite.
- `just lint` for Clippy and formatting checks where the area wires them in.
- `just doctest` when rustdoc examples changed.
- `just test-l2`, `just test-l3`, `just test-browser`, or `just test-real` only when the change touches behavior covered by those tiers.
- `just all` before handing off a non-trivial change; it runs the canonical PR-gate sequence: `sanity -> lint -> doctest -> test -> test-l2 -> test-browser`.
- `just bench` for Criterion benchmarks. Benchmarks are performance signals, not pass/fail unit tests, and are kept out of `all`.
- `just coverage` for LCOV output. Coverage is report-only; it is not a percentage gate.

### Leveraging Shared Just Recipes

The `just/` folder contains shared `*.just` files that package areas import from their own `justfile`. This gives every area the same lifecycle vocabulary while still letting each area decide which concrete crates it owns.

There are two common patterns.

Some imported recipes are direct pass-through utilities. For example, package-area justfiles import `just/devops.just`, and that makes shared helper recipes like `deps` available directly. The area does not reimplement them; it just imports the file.

Other shared recipes are intentionally private templates with a leading underscore. The package-area recipe provides the public name and calls the shared implementation once per crate it owns:

```just
sanity:
    @just _sanity darkmatter
    @just _sanity darkmatter-cli

test *args="":
    @just _test darkmatter {{ args }}
    @just _test darkmatter-cli {{ args }}

coverage *args="":
    @just _coverage darkmatter {{ args }}
    @just _coverage darkmatter-cli {{ args }}
```

The underscore recipe owns the shared mechanics: `cargo nextest` selection, tier filter expressions, LCOV generation, timing output, and skip behavior. The public area recipe owns scope: which crates are included and which tiers are meaningful for that area.

The high-usage shared recipes are `_sanity`, `_test`, `_test_l2`, `_test_browser`, `_lint`, `_doctest`, `_bench`, `_coverage`, `_fuzz`, and `_all`. Area recipes that do not apply should be explicit no-ops rather than silently missing, so `just check-canonical` can verify the expected lifecycle surface.

## Test Nomenclature

Rusty Biscuit uses a small set of testing terms consistently across package areas:

- **Package**: a Cargo package from `cargo metadata`, such as `darkmatter` or `darkmatter-cli`.
- **Package area**: a repo area, usually with a `lib` and `cli` split, such as `darkmatter/` or `biscuit-file/`.
- **Curated area list**: the root `justfile`'s lifecycle list. It is practical orchestration scope, not a complete workspace manifest.
- **Level 1 / L1**: ordinary in-process tests. These should be deterministic, fast enough for regular local use, and should not require a real terminal, browser, external device, or external API.
- **Sanity**: the fast L1 subset used for frequent confidence checks. It runs library and binary tests while excluding test names or module paths that contain `level2_`, `level3_`, `browser_`, `real_`, or `slow_`.
- **Slow L1**: an otherwise ordinary test whose runtime makes it inappropriate for `sanity`; name it with `slow_`.
- **Level 2 / L2**: tests that need a real terminal, PTY, or terminal harness. Name them with `level2_`.
- **Level 3 / L3**: tests that need OS keyboard or mouse injection. Name them with `level3_`; they require explicit opt-in through `RUN_LEVEL3=1`.
- **Browser tests**: headless Chrome/Chromium tests. Name them with `browser_`.
- **Real-resource tests**: tests that need a real service, network API, device, or other external dependency. Name them with `real_`.
- **Harness**: reusable infrastructure that makes a real resource testable, such as `biscuit-test-harness` for terminal panes or `biscuit-browser-harness` for Chrome.
- **Doctest**: a rustdoc example compiled and run by `cargo test --doc`; kept out of `sanity` because compile cost is noisy.
- **Benchmark**: a Criterion performance measurement run through `just bench`. It tracks throughput or latency, not correctness.
- **Fuzz test**: an adversarial input generator run with `cargo fuzz` and nightly Rust. Fuzzing is a nightly or explicit local activity, not part of the normal PR loop.
- **Coverage**: LCOV output from `cargo llvm-cov`. Coverage is a diagnostic artifact and an input to CRAP analysis; it is not a standalone quality gate.
- **CRAP score**: Change Risk Anti-Patterns score, combining cyclomatic complexity with coverage to identify complex, poorly tested functions.

## The Rusty Biscuit Test Harness

The test harness crates keep resource-heavy tests behind explicit gates so the normal test suite remains predictable.

`test_toolkit` provides the common gating vocabulary. Tests that need a resource call `require_level!` at the top of the test body. If the resource is unavailable, the test skips cleanly by default. If the corresponding "required" environment variable is set, the same missing resource becomes a hard failure, which is useful in CI jobs that are expected to provide the resource.

```rust
use test_toolkit::{Level, require_level};

#[test]
fn level2_renders_in_terminal() {
    require_level!(Level::L2, harness_is_available(), "terminal harness");
    // test body
}
```

`biscuit-test-harness` owns real-terminal harnesses for WezTerm, Kitty, tmux, and Apple Terminal. The shared `_test_l2` recipe builds `biscuit-harness-broker`, spawns one shared pane or session per available backend, exports `BISCUIT_SHARED_*` environment variables, and then runs nextest with the `level2_` filter at `-j 1`. Tests attach to the shared resource when possible and fall back to local spawning or skip behavior when not.

`biscuit-browser-harness` owns the headless browser path. Browser tests should assert computed behavior, such as computed CSS values or DOM state, rather than checking source substrings. Missing Chrome skips by default; `BISCUIT_BROWSER_REQUIRED=1` converts that into a hard failure.

The naming convention matters because nextest selection is name-based:

```text
level2  = test(/level2_/)
level3  = test(/level3_/)
browser = test(/browser_/)
real    = test(/real_/)
slow    = level2 + level3 + browser + real + test(/slow_/)
```

Use `#[serial_test::serial]` when a test mutates process-global state, shares a harness, uses environment variables, or relies on exclusive access to a real terminal. Use `test_toolkit::EnvGuard` for environment setup so cleanup happens even on failure.


## Linting

`just lint` is the standard lint entry point. At the package level it delegates to `_lint <pkg>`, which runs:

```bash
cargo clippy -p <pkg> -- -D warnings
```

Package areas that own multiple crates call `_lint` once per crate. The root `just lint` iterates the curated area list and invokes each area's public `lint` recipe.

Linting is deliberately separate from `sanity`. `sanity` answers "do the fast tests pass?" while `lint` answers "does the code meet compiler and Clippy hygiene expectations?" `just all` runs both, with `sanity` first so cheap behavioral failures surface before lint output.

Do not use lint recipes as a place for broad formatting churn. Keep lint fixes scoped to the warning or error at hand, and update docs only when public behavior or workflow changes.

## Performance Testing

Performance testing uses Criterion benchmarks exposed through `just bench`. The shared `_bench <pkg>` recipe runs:

```bash
cargo bench -p <pkg>
```

Some areas add narrower benchmark recipes when the package has distinct performance surfaces. For example, `darkmatter` has separate schema, compose, and render benchmark recipes, plus baseline comparison helpers.

Benchmarks are not part of `sanity`, `test`, or `all` because they need a quiet machine and their output is comparative. Use them when a change touches parsing, rendering, filesystem walks, terminal layout, API dispatch, or any other path where latency or throughput is part of the user-visible contract.

When reporting benchmark results, prefer before/after comparisons from the same host and similar power state. A single absolute number from a laptop under load is not a regression proof.

## Test Coverage

Coverage is generated with `cargo-llvm-cov` and exported as LCOV. The shared `_coverage <pkg>` recipe runs:

```bash
cargo llvm-cov -p <pkg> --lcov --output-path lcov-<pkg>.info
```

Package areas call `_coverage` once for each crate they own. The root `just coverage` recipe iterates curated areas through `_orchestrate coverage`, so it is an area-level aggregation workflow rather than a single Cargo workspace command.

Coverage is report-only in Rusty Biscuit. We do not fail PRs on a package-wide coverage percentage because percentage gates are easy to game and noisy during legitimate refactors. Coverage is still useful for:

- finding untested public paths,
- checking that a bug fix has a regression test,
- feeding `cargo-crap`,
- spotting accidental loss of coverage on risky code,
- giving reviewers context when a change claims to be test-backed.

Doctests are separate from normal coverage recipes unless explicitly added to a `cargo llvm-cov` invocation. L2, L3, browser, and real-resource tests may also be absent from ordinary local coverage depending on host capabilities and environment variables.

### Using `cargo-crap`

`cargo-crap` produces a per-function **Change Risk Anti-Patterns (CRAP)** score by combining cyclomatic complexity with LCOV-derived test coverage. Methodology, formula, and known blind spots are documented in [the research note](../research/cargo-crap.md); this section is concerned only with how we integrate it into Rusty Biscuit's testing workflow.

#### Cost shape drives where we run it

The `cargo crap` analysis itself takes seconds. The expensive prerequisite is producing `lcov.info`, which requires a coverage-instrumented rebuild plus a full test execution. In this workspace that is minutes, not seconds, even with `cargo nextest` parallelism. This single fact determines our integration strategy: CRAP runs where slow feedback is acceptable, never on the interactive loop.

#### Isolated package and package-area runs

Yes, an isolated CRAP analysis is possible without rerunning coverage for every package in the monorepo, but the coverage and analysis scopes must match.

For a single package:

```bash
cargo llvm-cov -p darkmatter --lcov --output-path lcov-darkmatter.info
cargo crap --path darkmatter/lib --lcov lcov-darkmatter.info
```

For an area with a library and CLI, run one package-scoped coverage pass per crate and one CRAP pass per crate path:

```bash
cargo llvm-cov -p darkmatter --lcov --output-path lcov-darkmatter.info
cargo crap --path darkmatter/lib --lcov lcov-darkmatter.info --format json --output crap-darkmatter.json

cargo llvm-cov -p darkmatter-cli --lcov --output-path lcov-darkmatter-cli.info
cargo crap --path darkmatter/cli --lcov lcov-darkmatter-cli.info --format json --output crap-darkmatter-cli.json
```

Use `cargo crap --workspace --lcov lcov.info` only when the LCOV file was generated for the workspace. In workspace mode, `cargo-crap` ignores `--path`, discovers members with `cargo metadata`, and walks every member crate. That is correct for release-candidate or nightly aggregate reports, but it is the wrong shape for a local "tell me about this package" command.

This maps well to the repo's existing coverage recipe: `_coverage <pkg>` already uses `cargo llvm-cov -p <pkg>`. A future shared `_crap <pkg> <path>` recipe can build directly on that instead of forcing a workspace coverage run.

#### When to run it

- **Not on pre-commit, not on PR.** Lint and the L1/L2 tiers stay on the hot path. CRAP would make PR runs cost-prohibitive without changing the signal a reviewer needs.
- **Nightly per package area, in CI.** Each area in the root `justfile`'s curated list is one shard. Shards run in parallel and produce a per-area report.
- **Release-candidate gate.** A workspace-wide pass on RC tags. Output is attached to the release draft as an advisory artifact — it informs the release notes, it does not block the merge train.
- **On demand from each area's `justfile`.** A shared `crap` recipe in `just/` lets a developer run `just crap` locally when refactoring an area, mirroring the pattern already used for `lint` and `test`.

#### Automating it responsibly

1. A scheduled GitHub Actions workflow (`0 7 * * *` plus `workflow_dispatch`) drives the nightly pass.
2. The matrix shards by package area, reusing the same curated list the root `justfile` iterates. Each shard runs package-scoped coverage for the crates owned by that area, then runs package-scoped CRAP analysis with `--path <crate-dir>` and the matching LCOV file. A workspace-wide shard should use `cargo llvm-cov --workspace` plus `cargo crap --workspace`.
3. Results are diffed against the previous successful run with `cargo-crap`'s JSON baseline / delta mode. The digest — functions newly crossing the threshold, functions that improved — is posted as a workflow summary and stored as a build artifact. Nothing is auto-committed.
4. A separate workflow keys on release-candidate tags, runs the full-workspace pass, and attaches the CRAP report to the release draft for the release manager to review.
5. Persistent regressions (a function above threshold for N consecutive runs) open or update a tracking issue; they do not fail the build.

#### Making CRAP cheaper with caching and change detection

There are two layers to optimize.

First, keep Cargo's normal caches warm. Coverage builds use different instrumentation flags, so they cannot fully reuse ordinary debug artifacts, but they can reuse dependency downloads, the Cargo registry, the git checkout, and previous coverage target artifacts when the CI cache key is stable. Locally, `cargo llvm-cov --no-clean` can avoid deleting old coverage build artifacts, but it should be used with care in CI because stale profile data can make reports misleading if the workspace shape changes.

Second, avoid running CRAP where neither the source nor the relevant tests changed. A practical package-area cache key should include:

- the package's `src/`, `tests/`, `benches/`, `examples/`, and `build.rs`,
- the package `Cargo.toml`,
- the workspace `Cargo.lock` and root `Cargo.toml`,
- local path dependencies from `cargo metadata`,
- shared test infrastructure such as `tools/test-toolkit`, `biscuit-test-harness`, and `biscuit-browser-harness` when the package uses them,
- the commands/config that affect coverage or CRAP output, including `.config/nextest.toml`, `just/devops.just`, and any `.cargo-crap.toml`.

If the key matches the previous successful run, reuse the prior JSON report and LCOV artifact instead of rerunning coverage. If only report rendering changed, reuse the LCOV and rerun `cargo crap`, because the analysis phase is cheap. If source or tests changed, regenerate LCOV for the affected package only.

For local developer ergonomics, the most useful addition would be a shared recipe pair:

```just
_crap pkg path *args="":
    cargo llvm-cov -p {{ pkg }} --lcov --output-path lcov-{{ pkg }}.info
    cargo crap --path {{ path }} --lcov lcov-{{ pkg }}.info {{ args }}

_crap_cached pkg path keyfile *args="":
    # compute a package fingerprint; reuse lcov/report artifacts when unchanged
```

The cached variant should be conservative. It is better to rerun coverage unnecessarily than to publish a stale risk report.

#### Thresholds and the "preservation by accumulation" trap

The research flags an AI-coding failure mode worth taking seriously: agents lower a high CRAP score by adding trivial assertions rather than simplifying the function. Two guardrails together neutralize this:

- **Cyclomatic complexity ceiling** — flag any function with `CC > 10`. Coverage cannot dilute this signal; the function must be refactored.
- **CRAP advisory threshold** — flag any function with `CRAP >= 30`. This is a review prompt, not a gate. The reviewer chooses refactor, add meaningful tests, or accept with justification recorded in the PR.

#### Exclusions

CRAP is noisy on code that we either do not author by hand or cannot meaningfully unit-test in place:

- `schematic/schema` — already excluded from the workspace; exclude from coverage runs as well.
- Proc-macro crates (e.g. `unchained-ai/model_id`). Static cyclomatic complexity undercounts macro-expanded branches; the consumer's tests are the real signal.
- Generated bindings, `examples/`, and test harnesses.

Use `cargo-llvm-cov`'s ignore filters and `cargo crap`'s `--exclude` flags rather than tuning the formula.

#### What CRAP does not replace

CRAP is one signal alongside others, not a substitute for them:

- **Mutation testing** (`cargo-mutants`) — verifies that the tests behind a coverage number actually assert behavior.
- **Cognitive complexity review** — a wide `match` has high CC but low cognitive load; a short generic function with nested `Result` handling can be the opposite. Human review still owns this axis.
- **The L1/L2/L3 testing tier discipline** — see [`rust-testing`](../../.claude/skills/rust-testing/SKILL.md) for the canonical taxonomy and the `require_level!` gating mechanism.
