---
$schema: feature-review.yaml
ready: false
findings:
  - title: Source identity rejects pull-request and pre-push archive orchestration
    priority: high
  - title: Real package archives still depend on the producer's absolute checkout path
    priority: high
  - title: The required performance acceptance gate has no usable baseline
    priority: high
  - title: Owner measurement publication is not schema-tested
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-15T03:44:16-07:00"
spec: 2026-09-12-single-os-compile/spec.md
implemented: true
next: 2026-09-12-single-os-compile/review-3.md
implemented_by: claude/opus
log: fixes/2026-09-12-single-os-compile/log.md
description: "A **fix** review of `2026-09-12-single-os-compile/spec.md`"
fix: 2026-09-12-single-os-compile/review-2.md
previous: 2026-09-12-single-os-compile/review-1.md
---

# Review 2

**Not production-ready.** All six findings from Review 1 were implemented, and
Review 1 contained no `## Blocked Findings` section to re-evaluate. The revised
owner topology, source-tree validation, runtime inspection, Bash 3 support,
owner instrumentation, and Python 3.9 compatibility all have passing local
tests. Four readiness gaps remain at integration and acceptance boundaries.

Cross-OS execution results were not used to decide readiness; those results are
CI/CD evidence, as directed by this review. The findings below instead concern
contracts that the implementation or its tests currently violate.

## Review 1 closure

| Previous finding | Implementation status |
|---|---|
| Hosted build fan-out defeats cross-package compilation reuse | Implemented. The workflow now fans out once per producer and `produce-owner.sh` builds that producer's records in one target tree. |
| Producer provenance accepts a dirty or mismatched source tree | Implemented in the archive tool. Production and verification now require the planned `HEAD`, a clean tracked tree, and its resolved tree identity. The orchestration mismatch created by that stricter contract is a new finding below. |
| Runtime compatibility is copied from policy instead of discovered | Implemented. Produced binaries are inspected with platform-native mechanisms and a real external dynamic-dependency fixture exercises rejection. |
| Cross-host execution fails under the macOS system Bash | Implemented. Indexed arrays replace associative arrays, and the cross-check suite passes under macOS Bash 3. |
| The required performance comparison cannot measure owner builds | Implemented as instrumentation. The counter now reaches producer owners and publication aggregates owner measurements; collection and acceptance remain outstanding. |
| A build-key contract test is incompatible with the host Python | Implemented. The canonical Python 3.9 suite passes. |

## Findings

### High — Source identity rejects pull-request and pre-push archive orchestration

The archive tool now correctly rejects any checkout whose `HEAD` differs from
`plan.head`, or whose tracked tree is dirty
(`scripts/ci-build-archive.rs:762-773`). The shipped orchestration does not
consistently satisfy that contract.

For pull requests, the scope job names `github.event.pull_request.head.sha` as
the plan head (`.github/workflows/ci.yml:117-152`), while the scope, owner, and
package jobs use `actions/checkout@v5` without a `ref`
(`.github/workflows/ci.yml:104-106`, `:502`, and
`.github/workflows/_package-ci.yml:207`). [GitHub documents that a pull-request
workflow checks out the merge branch by default](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request).
The owner and every archive consumer will therefore fail
`build-source-mismatch` before producing or running an archive. The scope
calculation also reads Cargo metadata from one revision while labeling the plan
with another.

The local cross-host path has the same contradiction. It records the local
`HEAD` in the plan, but checks the remote repository out at `base_sha` and
applies the local diff as uncommitted changes (`scripts/cross-check.sh:241-279`
and `:354-364`). Archive production at `:416` must then reject either the
different `HEAD`, the dirty tracked tree, or both. The current cross-check tests
validate generated command text; they do not execute this identity boundary.

Choose one immutable tested revision and use it end to end. For hosted PRs,
either pin all relevant checkouts to the PR head or consistently make the merge
revision the plan identity. For pre-push cross-checks, create and name a clean
synthetic revision containing the shipped patch, or explicitly route dirty and
ahead trees through the non-archive diagnostic mode. Add executable fixtures
for a PR checkout and a patched/ahead cross-check, not only source assertions.

### High — Real package archives still depend on the producer's absolute checkout path

The specification explicitly supersedes the WSL workaround that recreates the
producer's literal checkout path and requires validation from a different path
without the producer checkout (`spec.md:183-202`, `:322-325`). The workflow
still reads `producer_workspace` from the archive manifest, creates that exact
path inside WSL2, and clones the repository there
(`.github/workflows/_wsl-ci.yml:275-303`). Its own comment explains that roughly
150 test sites embed compile-time `env!("CARGO_MANIFEST_DIR")` paths.

The relocation fixture proves that its synthetic package is relocatable, but
it does not establish that the real package corpus is. Native hosted consumers
also commonly receive the same checkout layout as their producer, which masks
this coupling. The result does not meet Decision 4 or the AC4 requirement that
WSL2 execute without the producer checkout path.

Migrate archive-consuming tests and fixtures to runtime-remapped lookup (using
the repository test-harness helpers where appropriate), prevent new
compile-time workspace-path dependencies, and exercise representative real
package archives from a genuinely different checkout path with the producer
workspace and target absent. Remove `producer_workspace` from the execution
workaround once the corpus passes that test.

### High — The required performance acceptance gate has no usable baseline

AC8 requires matched cold and warm observations, total runner compute and
critical path, and three consecutive green runs per environment before the
single-owner architecture can be accepted (`spec.md:335-340`). Plan Tasks 1.6,
7.4, and 7.5 remain unchecked, the baseline observation rows are empty, and the
rollout ledger correctly marks AC8 `NOT MET`.

This is more than missing cross-OS proof. The instrumentation and ownership
cutover are in the same working-tree change, so dispatching the current
workflow can measure only the post-cutover schedule. It cannot retroactively
produce the required pre-cutover baseline. The rollout's claim that the first
measured hosted run after merge closes both tasks also conflicts with the plan,
which correctly says neither a first run nor merging closes Task 7.5.

Construct an instrumentation-only pre-cutover revision or record an explicit
approved replacement comparison, then collect the specified matched runs and
apply the 15% decision rule. Until that acceptance gate has an actual baseline
and ruling, the performance-sensitive owner architecture is not ready.

### Medium — Owner measurement publication is not schema-tested

The publisher aggregates `manifest.compiler_work.compiler_invocations` and
`compiler_ms`, then embeds the owner aggregate in each record's measurement
artifact (`scripts/ci/artifacts/publish.cjs:7-18`, `:39-45`). Its only test uses
the incompatible fixture shape `compiler_work: {invocations: 1}` and merely
checks that a measurement filename is uploaded
(`scripts/ci/artifacts/publish.test.cjs:16-33`). It never parses the artifact or
asserts the owner totals. Zero invocations and missing duration data therefore
pass this test, even though those values underpin AC8.

Use the Rust manifest's real schema in the fixture and assert the published
record and owner values, including measured records, compiler invocations,
compiler time, and owner timing. Prefer generating the fixture through the
Rust producer when practical so the JavaScript/Rust boundary cannot drift.
Rename the workflow contract that still says “one leg per planned record” now
that it asserts one leg per producer; its current name obscures the topology it
is intended to protect.

## Verification-level assessment

This fix has no keyboard, mouse, paste, IME, terminal-rendering, or browser-UI
interaction requirement. Level 2 terminal capture and Level 3 OS keyboard
injection are therefore not applicable. The appropriate boundary is executable
CI orchestration and real archive execution.

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 plan ownership and validation | Level 1 Python planner/schema fixtures | Appropriate and passing. |
| AC2 one archive serves L1/L2/browser without compilation | Level 1 real-Cargo archive fixtures and workflow contracts | The archive path is tested, but the hosted PR source identity prevents it from running (finding 1). |
| AC3 shared dependencies compile once without feature unification | Level 1 real-Cargo two-package owner fixture | Appropriate for compilation behavior; the producer-scoped hosted topology is statically contracted. |
| AC4 native/WSL compatibility and distinct results | Level 1 compatibility and workflow fixtures | Wrong boundary for checkout relocation: real packages are still run at the producer path (finding 2). |
| AC5 payloads and pre-test refusal | Level 1 real archive, checksum, inventory, provenance, and dynamic-library fixtures | Appropriate for archive rejection; orchestration cannot satisfy source identity (finding 1). |
| AC6 failure cannot pass or rebuild | Level 1 Rust folds and executable step simulations | Appropriate and passing. |
| AC7 existing CI semantics remain | Level 1 planner/workflow contracts plus `actionlint` | Static contracts pass; PR and cross-check revision identity are not exercised (finding 1). |
| AC8 measured architecture decision | No completed hosted comparison | Missing required acceptance verification (findings 3 and 4). |
| AC9 relevant contract suites pass | Level 1 Rust, Python, JavaScript, and workflow validation | Passing locally. The publisher assertion is too weak to validate its measurement payload (finding 4). |

## Validation performed

- `actionlint .github/workflows/*.yml`: passed.
- `git diff --check`: passed after the review metadata updates.
- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 590 passed.
- `node --test scripts/ci/artifacts/publish.test.cjs`: 1 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
  --features build-tools --bin ci-build --no-fail-fast`: 114 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
  --features rollup --bin ci-rollup --no-fail-fast`: 195 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
  --features plan --bin ci-plan --no-fail-fast`: 13 passed.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts
  --no-fail-fast`: 116 passed.
- `just ci-local --plan`: passed and resolved the expected macOS, Linux, native
  Windows, and WSL2 cells without violating a local execution constraint.

Initial Rust invocations against the shared target cache encountered existing
read-only `.rmeta` files. The same suites passed from isolated target
directories, so that host cache condition is not an implementation finding.

## Human review

No human review is required for this iteration. The source/revision contract,
relocation work, and publisher test can be corrected mechanically. Human input
is needed only if the completed AC8 measurements exceed the 15% band or if the
team chooses to revise that acceptance criterion instead of producing a valid
comparison.

The supplied previous-review reference under `prompts/_reviews/fixes/` does not
resolve through `bf reference`. The existing previous review resolves at
`fixes/2026-09-12-single-os-compile/review-1.md`; that file now has the requested
`next` link and `implemented: true`. The specification records
`review_iterations: 2`.
