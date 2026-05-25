# Testing in Rusty Biscuit

## Using the `just` Infrastructure

We use [just](https://just.systems/) in all of the package areas (and root) of Rusty Biscuit to automate all of the common operations and that includes testing (as well as lint testing).

> TODO: fill in and describe: how user runs full tests, partial tests, benchmark tests, etc.

### Leveraging Shared Just Recipes

The `.just/*` folder contains several *.just files which contain reusable recipes that each area of the monorepo will leverage in their `justfile`. This allows high reuse across each area of the monorepo while 

> TODO: don't list all the reusable recipes but choose a few that have high usage and demonstrate. it's worth demonstrating recipes that just "pass through" when imported versus the `_{name}` recipes which expect the jusfile to call into the shared resource.

## Test Nomenclature

> TODO: introduce all the key nomenclature that we use for testing ... level 1,2,3, test harness, performance testing, fuzz testing. The goal is not to go into depth but instead provide a set of testing vocabulary that a user is likely to encounter when 

## The Rusty Biscuit Test Harness

> TODO: fill in all the details here


## Linting

> TODO: fill in all the details here

## Performance Testing

> TODO: fill in all the details here

## Test Coverage

> TODO: fill in all the details here

### Using `cargo-crap`

`cargo-crap` produces a per-function **Change Risk Anti-Patterns (CRAP)** score by combining cyclomatic complexity with LCOV-derived test coverage. Methodology, formula, and known blind spots are documented in [the research note](../research/cargo-crap.md); this section is concerned only with how we integrate it into Rusty Biscuit's testing workflow.

#### Cost shape drives where we run it

The `cargo crap` analysis itself takes seconds. The expensive prerequisite is producing `lcov.info`, which requires a coverage-instrumented rebuild plus a full test execution — for a 48-crate workspace that is minutes, not seconds, even with `cargo nextest` parallelism. This single fact determines our integration strategy: CRAP runs where slow feedback is acceptable, never on the interactive loop.

#### When to run it

- **Not on pre-commit, not on PR.** Lint and the L1/L2 tiers stay on the hot path. CRAP would make PR runs cost-prohibitive without changing the signal a reviewer needs.
- **Nightly per package area, in CI.** Each area in the root `justfile`'s curated list is one shard. Shards run in parallel and produce a per-area report.
- **Release-candidate gate.** A workspace-wide pass on RC tags. Output is attached to the release draft as an advisory artifact — it informs the release notes, it does not block the merge train.
- **On demand from each area's `justfile`.** A shared `crap` recipe in `.just/*` lets a developer run `just crap` locally when refactoring an area, mirroring the pattern already used for `lint` and `test`.

#### Automating it responsibly

1. A scheduled GitHub Actions workflow (`0 7 * * *` plus `workflow_dispatch`) drives the nightly pass.
2. The matrix shards by package area, reusing the same curated list the root `justfile` iterates. Each shard runs `cargo llvm-cov --workspace --lcov --output-path lcov.info` scoped to its area, then `cargo crap --workspace --lcov lcov.info --json`.
3. Results are diffed against the previous successful run. The digest — functions newly crossing the threshold, functions that improved — is posted as a workflow summary and stored as a build artifact. Nothing is auto-committed.
4. A separate workflow keys on release-candidate tags, runs the full-workspace pass, and attaches the CRAP report to the release draft for the release manager to review.
5. Persistent regressions (a function above threshold for N consecutive runs) open or update a tracking issue; they do not fail the build.

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