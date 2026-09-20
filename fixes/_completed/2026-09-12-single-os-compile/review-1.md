---
$schema: feature-review.yaml
ready: false
findings:
  - title: Hosted build fan-out defeats cross-package compilation reuse
    priority: high
  - title: Producer provenance accepts a dirty or mismatched source tree
    priority: high
  - title: Runtime compatibility is copied from policy instead of discovered
    priority: high
  - title: Cross-host execution fails under the macOS system Bash
    priority: high
  - title: The required performance comparison cannot measure owner builds
    priority: high
  - title: A build-key contract test is incompatible with the host Python
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-14T17:59:28-07:00"
spec: 2026-09-12-single-os-compile/spec.md
implemented: true
description: "A **fix** review of `2026-09-12-single-os-compile/spec.md`"
fix: 2026-09-12-single-os-compile/review-1.md
next: 2026-09-12-single-os-compile/review-2.md
---

# Review 1

**Not production-ready.** The archive producer and verifier have substantial
L1 coverage, but the shipped workflow does not preserve the shared target tree
that the compile-once objective depends on. The provenance manifest also does
not observe two classes of inputs it claims to prove, the canonical cross-host
entry point fails on macOS's system Bash, and the performance acceptance path
cannot currently measure the new owner build.

This review covers HEAD plus the complete working-tree implementation. Missing
Linux/WSL2 execution evidence is recorded below but does not determine
readiness, in accordance with the review instructions. No implementation files
were changed.

## Findings

### High — Hosted build fan-out defeats cross-package compilation reuse

The specification requires one native build owner per producer environment,
building selected packages deterministically in one target tree so compatible
dependencies compile once. The shipped workflow instead flattens owners into
one matrix leg per build record (`scripts/ci/affected_scope.py:2544-2576`) and
runs each record on a fresh runner (`.github/workflows/ci.yml:470-510`). Each
leg then narrows `ci-build produce` with `--key` and uses a package-specific
cache key (`.github/workflows/ci.yml:583-611`). Two packages selected for the
same environment therefore do not share a live target tree, and cache hits are
only an optimization, not the producer contract the specification requires.

`one_owner_tree_shares_a_dependency_compile_without_unifying_features` is a
valid L1 test of `ci-build produce` when two records are deliberately run in
one process and target directory. It does not exercise the workflow topology;
the workflow contract suite instead explicitly requires one leg per record.
Thus AC3's fixture proves a path hosted CI never takes, while AC2/AC3 and design
decision 2 remain unimplemented.

Make the matrix producer-scoped (or use planner-declared cohorts) and invoke
the producer once for all records assigned to that owner/cohort. Preserve the
separate per-key archive identities through a transport that can publish the
resulting set. Add an executable workflow-boundary fixture that proves two
package records are built in the same target tree and observes one compile of
their identical dependency.

### High — Producer provenance accepts a dirty or mismatched source tree

`source_tree()` is documented as the producer's working-tree identity but runs
`git rev-parse HEAD^{tree}` (`scripts/ci-build-archive.rs:762-769`). That value
is the committed tree and does not change when tracked source files are
modified. This review confirmed the current dirty implementation tree reports
the exact same value from `source_tree()`'s command and `git show
-s --format=%T HEAD`. The producer does not require a clean tracked tree.

Verification compares only `plan.head` with `manifest.source_commit`
(`scripts/ci-build-archive.rs:1639-1647`); it never compares
`manifest.source_tree` with a plan or consumer tree. A producer can therefore
compile modified source at the planned commit, emit the planned commit's tree
identifier, and have the archive accepted. That violates the Objective and
AC4/AC5 requirement that consumers verify the exact source tree.

Require a clean producer checkout and bind the manifest to the actual content
compiled. Carry the expected tree in the plan (or derive it from the resolved
commit at both boundaries), compare it during verification, and add an
end-to-end test in which a tracked source modification makes production or
verification fail before tests start.

### High — Runtime compatibility is copied from policy instead of discovered

The realized manifest is required to add the actual linker and relevant native
dependency versions. In `produce_one`, however, `realized.linker` is copied
from `identity.linker` and `realized.runtime` is cloned from the plan
(`scripts/ci-build-archive.rs:1166-1188`). The environment table currently
declares an empty `native_libraries` list for every producer. No emitted binary
is inspected to discover its linked libraries or versions, and no linker
version is probed.

The verifier consequently compares policy-derived values with other
policy-derived values. The passing
`an_environment_missing_a_linked_native_library_refuses_the_archive` test
manually inserts `libdbus-1.so.3` into a synthetic manifest
(`scripts/ci-build-archive-tests.rs:390-425`); it does not demonstrate that a
real producer would ever discover that dependency. Linux-to-WSL2 compatibility
therefore is not proved by the manifest as design decision 5 and AC4 require.

Inspect the produced executables and dynamic libraries with platform-native,
non-interactive mechanisms, record the actual linker identity/version and
runtime library requirements, and verify them against the consumer. Add a
producer-to-verifier fixture containing a real dynamic dependency so removing
the consumer declaration fails without hand-editing the manifest.

### High — Cross-host execution fails under the macOS system Bash

`scripts/cross-check.sh:100-106` uses associative arrays. The supported macOS
host's `/bin/bash` is 3.2.57, where `declare -A` is unavailable; the script is
invoked with `bash`, so it aborts at line 100 with `linux: unbound variable`.
The independent `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`
run reproduced this across every cross-check contract: 11 failures, including
Linux, WSL2, macOS, and native Windows shipping paths, all before any host was
contacted.

This breaks the specification's canonical local/cross-host orchestration on a
required OS. Replace the associative maps with Bash-3-compatible case
functions or indexed data, unless the repository explicitly provisions and
invokes a newer Bash on every supported host. Keep the existing subprocess
tests running under `/bin/bash` on macOS.

### High — The required performance comparison cannot measure owner builds

AC8 requires matched cold/warm observations, compiler-work counts, total
runner compute, critical path, and three consecutive green runs per
environment before accepting the architecture. The rollout ledger correctly
records that neither the pre-cutover nor post-cutover observations exist.

There is also an implementation blocker: `measure-compiler-work` is passed to
the package workflow and instruments check/lint/tier steps, but the top-level
`build` job never reads it and its `ci-build produce` command supplies no
`--counter-dir` (`.github/workflows/ci.yml:498-611`). Now that test consumers
do not compile, a dispatch cannot collect compiler-work evidence for the owner
whose reuse is the subject of the comparison. Merely running the documented
dispatches would still leave the central measurement absent.

Wire the opt-in counter into each owner invocation and publish its per-record
and per-owner results without making instrumentation part of ordinary CI.
Then collect the matched pre/post cold and warm observations and apply the 15%
decision rule. If the result rejects a single owner, record the architecture
ruling before implementing cohorts. This is a required performance acceptance
gate, not a request for additional cross-OS correctness proof.

### Medium — A build-key contract test is incompatible with the host Python

`test_a_helper_answering_another_generation_is_refused` calls
`unittest.TestCase.enterContext` (`scripts/ci/test_build_key.py:57`), which is
not available in the host's Python 3.9.6. The full canonical Python suite
therefore has one error in addition to the cross-check failures, and the
wrong-helper-generation behavior is not verified on this supported macOS
development environment.

Use a `TemporaryDirectory` managed by `addCleanup`, `ExitStack`, or an explicit
context manager compatible with the repository's supported Python floor. Pin
that floor in CI/tooling documentation or tests so future helpers do not
silently raise it.

## Verification-level assessment

This feature has no keyboard, mouse, paste, IME, terminal-rendering, or browser
interaction requirement, so Level 2 terminal capture and Level 3 OS input are
not applicable. Its real boundary is the CI planner/process boundary and, for
workflow semantics, GitHub Actions itself.

| Requirement | Strongest evidence present | Assessment |
|---|---|---|
| AC1 plan ownership and validation | L1 Python planner/schema fixtures | Appropriate and passing. |
| AC2 one archive serves L1/L2/browser without compilers | L1 end-to-end archive and canonical-recipe tests on macOS; workflow source contracts | Archive behavior is strong, but the shipped owner topology violates the compile-once half (finding 1). |
| AC3 shared dependencies compile once without feature unification | L1 real-Cargo fixture using a manually shared owner tree | Wrong integration path: hosted CI isolates the records (finding 1). |
| AC4 native/WSL compatibility and distinct results | Behavioral macOS/native-Windows notes; L1 contracts for Linux/WSL2 | Missing Linux/WSL2 runs are evidence gaps only. Runtime provenance itself is incomplete (findings 2 and 3). |
| AC5 payloads and pre-test refusal | L1 real archive fixtures plus synthetic rejection tests | Appropriate for payloads/checksums; native-library discovery is synthetic only (finding 3). |
| AC6 failures cannot pass or rebuild | L1 Rust folds and executable workflow-step simulations | Good lower-bound coverage; no compiler fallback was found. |
| AC7 established CI semantics remain | L1 planner/workflow contracts and actionlint | Appropriate for static contracts; the macOS cross-host entry point is red (finding 4). |
| AC8 performance and three-run decision gate | No hosted measurements; owner compiler counter not wired | Required boundary absent and currently unreachable (finding 5). |
| AC9 contract suites and actionlint | Rust suites and actionlint pass; Python suite fails | Not met because the canonical Python suite is red (findings 4 and 6). |

## Validation performed

- `actionlint .github/workflows/*.yml`: passed.
- `git diff --check`: passed.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features
  --features build-tools --bin ci-build`, with a fresh target directory:
  111 passed, 0 skipped.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`, with a
  fresh target directory: 116 passed, 0 skipped.
- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: 589 run,
  11 failures and 1 error. All 11 failures stem from the Bash-3-incompatible
  cross-check initialization; the error is the Python-version issue above.

The first Rust attempts against the shared target directories failed because
cached `.rmeta` files were read-only. Fresh isolated target directories passed,
so that host cache state is not reported as an implementation finding.

## Human review

No human review is required at this iteration. The implementation and test
defects can be resolved mechanically. A human architecture decision becomes
necessary only if the completed AC8 measurements exceed the specification's
15% regression band and the team chooses among the documented fallback
designs.
