---
$schema: feature-review.yaml
ready: true
findings:
    - title: No remedy has reduced the workspace rebuilds
      priority: high
    - title: The guard-test fallback ruling still contradicts the implementation
      priority: medium
human_review: true
human_review_items:
    - |-
        Decide whether this fix must reduce build time before it closes. The measured Linux selection still builds 90 workspace crate configurations, including 65 beyond each crate's first. A local model estimates that changing `schematic-define` to use the YAML library directly would reduce the count to 84 and save about 77 seconds for this selection. Choose one:

        1. Approve that dependency change. Expect the implementation to run the affected package tests, measure the configuration count again, and record build time from the next comparable routine CI run.
        2. Change the specification's goal to a measurement-only investigation and explicitly accept that it did not reduce build time.
    - |-
        Decide where to keep the Claudine command-line package's source-checking test. The specification says to move it and update CI scheduling if no shared feature-alignment package is built. None was built, and the measured saving from moving this test is zero. Choose one:

        1. Keep the test where it is and amend the specification's fallback rule to say so.
        2. Keep the fallback rule and require the test move plus CI scheduling coverage.
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T21:50:54-07:00"
spec: 2026-09-21-ci-build-feature-divergence/spec.md
implemented: false
description: "A **fix** review of `2026-09-21-ci-build-feature-divergence/spec.md`"
fix: 2026-09-21-ci-build-feature-divergence/review-3.md
previous: 2026-09-21-ci-build-feature-divergence/review-2.md
---

# Review 3

**Not production-ready.** The last implementation corrected the two diagnostic findings from review 2, but it did not deliver the spec's build-reduction outcome or resolve the guard-test ruling. Review 2 has no separate `## Unblocked Findings` or `## Blocked Findings` sections; its four `## Findings` and two human-review items form the closure set. Neither human decision was recorded before this review.

## Review 2 closure

| Previous finding                                                                  | Status                                                                                                                                                                                                                                                       |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| No remedy has reduced the workspace rebuilds                                      | Open, decision dependent. `schematic-define` still reaches `serde_yaml_ng` through `biscuit-file`, and the Phase 4 report remains 90 → 90 Linux configurations.                                                                                              |
| The guard-test fallback ruling still contradicts the implementation               | Open, decision dependent. No alignment crate was built, but the guard remains in `claudine-cli` and the fallback ruling remains unchanged.                                                                                                                   |
| The wrapped-build reconciliation conflates configurations with build-script units | Addressed. The fixture now has a real `fa-util` build script. The wrapped-build test compares modeled configurations only with library compiles, checks build-script events separately, and checks that their seconds join the corresponding configurations. |
| Decision records still quote superseded attribution figures                       | Addressed in the decision-facing text. The spec now uses 90 / 65, 90 → 84, and 0.9 seconds; the Phase 4 report uses 0.9 seconds for the eligible trio. Historical measurements in the implementation log remain labeled as history.                          |

## Unblocked Findings

None from review 2. The two actionable test and documentation corrections were implemented, and I found no new implementation defect in the reviewed changes.

## Blocked Findings

### High — No remedy has reduced the workspace rebuilds

The spec's Outcome and acceptance criterion 5 require fewer repeated workspace builds and a shorter owner-job critical path. The Phase 4 report still shows **90 → 90** Linux configurations and **65 → 65** configurations beyond a crate's first. `schematic/define/Cargo.toml:10-14` still enables `biscuit-file` for `openapi`; its three YAML imports still use the re-export through `biscuit_file`. The estimated **90 → 84** configurations and roughly **77 seconds** from removing that edge are a local what-if, not an implemented or verified reduction.

The author's decision is still needed: approve that direct dependency change and verify it, or amend the spec's Outcome and acceptance criterion 5 to accept a measurement-only result. The next ordinary pull request's build-time observation is a follow-through measure, not a reason to block this review on cross-OS evidence.

APPROVED

### Medium — The guard-test fallback ruling still contradicts the implementation

The spec's Open Questions ruling says to use isolation with a planner watch when the alignment crate is not warranted. The implementation decided against that crate, but left the source-scan guard in `claudine-cli` and made no planner change. The measured saving from moving the guard is zero, so retaining it is reasonable; the written fallback must still be reconciled with that choice. Record the author's ruling to keep the guard in place and amend the spec, or implement the specified move with a test that proves CI schedules the guard for relevant source changes.

APPROVED

## Verification level and test reachability

| Requirement                                                                   | Strongest verification present                                                                     | Assessment                                                                                                |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Attribute configurations and explain divergent flags                          | Level 1 Cargo-tree fixtures and model assertions                                                   | Appropriate for this diagnostic; the fixture exercises own, workspace-dependency, and third-party causes. |
| Match reported configurations to Cargo builds and assign build-script seconds | Level 1 wrapped Cargo build with a real fixture build script, plus a focused join test             | The review 2 gap is closed. Library counts and build-script events are checked separately.                |
| Estimate compile seconds removed by alignment                                 | Level 1 saved-event and forwarded-feature fixture tests, including a build with alignment declared | Appropriate for the what-if model; the estimate does not prove a remedy was applied to this workspace.    |
| Reduce repeated workspace builds without hiding package feature needs         | No remedy or resulting regression test                                                             | Missing behavior; high finding above.                                                                     |
| Keep affected packages buildable independently                                | No package closure changed                                                                         | Applies when a remedy changes a closure; no new fidelity claim is made here.                              |
| Preserve the existing archive-reuse mechanism                                 | Level 1 `one_owner_tree_shares_a_dependency_compile_without_unifying_features`                     | Appropriate and passing.                                                                                  |

This spec has no keyboard, mouse, paste, styling, scrolling, or other terminal-emulator behavior. Level 2 and Level 3 tests are not required for its user-observable requirements. The `feature-attribution` binary is declared in `scripts/Cargo.toml`, its required `local-tools` feature is enabled by the package default used for L1, and its test names carry no higher-tier marker. Repository fixture paths are literal joins onto `repo_root()`, a form recognized by `docs/cicd/test-inputs.md`.

I ran the 22 `feature-attribution` tests and the named archive-reuse test with Nextest; all passed. `cargo nextest list` selected all 22 attribution tests, and `just check-tier-coverage scripts` reported no stranded tests. These checks establish the stated Level 1 coverage, not a build-time reduction.
