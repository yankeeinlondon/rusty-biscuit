---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-15T06:41:51-07:00"
spec: 2026-09-12-single-os-compile/spec.md
implemented: false
description: "A **fix** review of `2026-09-12-single-os-compile/spec.md`"
fix: 2026-09-12-single-os-compile/review-3.md
next: 2026-09-12-single-os-compile/review-4.md
previous: 2026-09-12-single-os-compile/review-2.md
---

# Review 3

**Production-ready.** The three implementation defects from Review 2 are
closed at the appropriate executable boundaries. Review 2's performance item
remains explicitly deferred because it requires hosted observations; under
this review's closure rules, cross-OS CI evidence does not determine
implementation readiness.

No human review is required before the implementation enters CI. A later
human design decision is necessary only if the deferred measurements show a
critical-path regression greater than the specification's 15% threshold.

## Review 2 closure

| Previous finding | Status |
|---|---|
| Source identity rejects pull-request and pre-push archive orchestration | Implemented. The workflow resolves one tested revision and pins every relevant checkout to it. The archive suite executes the PR-head-versus-merge-revision boundary, and cross-check now transfers and checks out a clean synthetic revision rather than applying an uncommitted patch. |
| Real package archives still depend on the producer's absolute checkout path | Implemented. Archive-executed code uses runtime-remapped manifest and binary lookup helpers; a repository-wide guard rejects new compile-time paths. The real-package relocation fixture builds `test-toolkit` and `biscuit-file`, deletes the producer checkout and target tree, and runs each L1 archive from another checkout without compilation. |
| The required performance acceptance gate has no usable baseline | Externally blocked, not an implementation finding. The implementation now constructs and verifies an instrumentation-only pre-cutover revision and documents a matched collection procedure. The actual three-run hosted observations remain absent, as the rollout ledger correctly states. This is cross-OS CI evidence and is excluded from readiness by the review instructions. |
| Owner measurement publication is not schema-tested | Implemented. Rust generates the compiler-work fixture from its serialized `Report`; JavaScript consumes that fixture, asserts record and owner totals, and rejects empty, schema-drifted, duration-less, and timing-less measurements. |

Review 2 contained no separate `## Blocked Findings` section. None of its
implementation findings became blocked during the correction cycle.

## Unblocked Findings

None.

## Blocked Findings

### Deferred — AC8 hosted performance observations

The pre-cutover and post-cutover cold/warm tables still need three consecutive
green hosted runs per producer environment. The repository now contains a
deterministic baseline revision constructor, validation tests, collection
instructions, and the 15% decision rule. Producing the observations requires
publishing the constructed revision and dispatching hosted workflows, which
was outside this non-interactive review's authorization.

This is recorded for specification closure, but it is not a readiness finding:
the review instructions explicitly exclude cross-OS CI evidence from the
`ready` decision. It also does not require human review unless a measured
regression exceeds 15% and one of the specification's alternative owner
topologies must be selected.

## Verification-level assessment

This fix has no keyboard, mouse, paste, IME, terminal rendering, or browser UI
interaction requirement. Level 2 terminal capture and Level 3 OS keyboard
injection are therefore not applicable. Its observable behavior is CI
planning, immutable artifact production and refusal, archive-only execution,
and result reporting; executable Level 1 and workflow-contract tests are the
appropriate local boundary. Cross-OS hosted outcomes remain CI/CD evidence.

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — evidence-aware plan ownership | Level 1 Python planner/schema fixtures plus the real `just ci-local --plan` projection | Appropriate and passing. The plan retains package/environment/gate cells and gives every executing test cell one compatible owner. |
| AC2 — one archive serves L1/L2/browser | Level 1 real-Cargo archive fixture executes all three canonical tier recipes from a relocated checkout without a compiler | Appropriate and passing. Workflow contracts also reject any compile-in-place consumer path. |
| AC3 — shared dependency reuse without feature unification | Level 1 real-Cargo two-package owner fixture | Appropriate and passing for compiler behavior. |
| AC4 — native/WSL compatibility and distinct results | Level 1 runtime compatibility/refusal fixtures, workflow contracts, repository path guard, and real-package relocation execution | Appropriate local proof. Actual macOS/Linux/Windows/WSL2 outcomes are external CI evidence and are not used for readiness. |
| AC5 — payload completeness and refusal before execution | Level 1 real archive tests for checksums, inventory, dynamic libraries, non-test binaries, build-script output, fixtures, includes, and sidecars | Appropriate and passing. |
| AC6 — failures cannot pass or silently rebuild | Level 1 Rust folds, executable refusal tests, and workflow step simulations | Appropriate and passing. |
| AC7 — established CI semantics remain | Level 1 planner, rollup, workflow, tier, and `actionlint` contracts | Appropriate and passing. |
| AC8 — matched performance comparison | Instrumentation, generated cross-language fixture, deterministic baseline revision, and documented hosted procedure; hosted observations deferred | Implementation support is present. The remaining evidence is explicitly excluded from readiness. |
| AC9 — relevant contract suites pass | Level 1 Rust, Python, JavaScript, workflow validation, and real local plan resolution | Appropriate and passing. |

## Validation performed

- `actionlint .github/workflows/*.yml`: passed.
- `git diff --check`: passed before and after the review metadata edits.
- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 607 passed.
- `node --test 'scripts/ci/artifacts/*.test.cjs'`: 3 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
  --features build-tools --bin ci-build --no-fail-fast`: 117 passed, including
  real-package archive relocation. Nextest labeled two pure, process-free
  refusal tests as non-failing stock-window pipe leaks; both assertions passed,
  and neither test spawns a child.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
  --features build-tools --bin ci-plan --no-fail-fast`: 13 passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
  --bin ci-rollup --no-fail-fast`: 195 passed.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts
  --no-fail-fast`: 119 passed.
- `python3 scripts/ci/build_baseline_revision.py --explain`: passed and resolved
  the documented pre-cutover revision.
- `just ci-local --plan`: passed and resolved package-keyed macOS, Linux,
  native Windows, and WSL2 cells with explicit archive owners.

The first scripts-workspace Rust invocation encountered the pre-existing
read-only `.rmeta` host-cache condition already documented in Review 2. The
same suite passed from a fresh isolated target directory, so that machine-local
cache condition is not an implementation finding.

## Recommendation

Approve the implementation for production CI. Retain the deferred AC8 record
until the hosted measurement procedure has run; if the result exceeds the 15%
band, reopen the architecture decision before declaring the specification
fully complete.
