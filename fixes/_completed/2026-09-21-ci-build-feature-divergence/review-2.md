---
$schema: feature-review.yaml
ready: false
findings:
    - title: No remedy has reduced the workspace rebuilds
      priority: high
    - title: The guard-test fallback ruling still contradicts the implementation
      priority: medium
    - title: The wrapped-build reconciliation conflates configurations with build-script units
      priority: medium
    - title: Decision records still quote superseded attribution figures
      priority: low
human_review: true
human_review_items:
    - |-
      Decide what this fix must deliver. Its current report measures repeated builds, but no change has reduced them. Letting the API schema package use the YAML library directly, instead of importing it through the general file library, is predicted to remove six of the 90 measured Linux builds and save about 77 seconds for the measured package selection. Choose one:

      1. Approve that package change, then require its tests and a fresh measurement before closing this fix.
      2. Change the specification's goal to a measurement-only investigation and explicitly accept that build time has not improved.
    - |-
      Decide what to do with a syntax-checking test in the Claudine command-line package. The specification currently says to move it and change CI scheduling if no shared feature-alignment package is built. No such package was built, and the measured benefit of moving this test is zero. Choose one:

      1. Keep the test where it is and amend the specification's fallback rule to match.
      2. Keep the fallback rule and require the test move plus CI scheduling coverage.
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T21:38:01-07:00"
spec: 2026-09-21-ci-build-feature-divergence/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/2026-09-21-ci-build-feature-divergence/implementation-log.md
next: 2026-09-21-ci-build-feature-divergence/review-3.md
description: "A **fix** review of `2026-09-21-ci-build-feature-divergence/spec.md`"
fix: 2026-09-21-ci-build-feature-divergence/review-2.md
previous: 2026-09-21-ci-build-feature-divergence/review-1.md
---

# Review 2

**Not production-ready.** Review 1's two diagnostic findings have been addressed, but its build-reduction finding and guard-test ruling remain open. The prior review has no separate `## Unblocked Findings` or `## Blocked Findings` sections; its four `## Findings` and two human-review items are the closure set. Neither human decision was recorded before the last implementation.

## Review 1 closure

| Previous finding | Status |
|---|---|
| No change reduces the workspace rebuilds the fix targets | Open. The dependency edge is unchanged. The corrected before/after report remains 90 Linux configurations, 65 beyond a crate's first. |
| The attribution table counts packages with no library compile | Addressed. The model excludes bin- and test-only roots; the refreshed table drops from 27 to 25 workspace crates and from 92 to 90 Linux configurations. A wrapped Cargo fixture checks the bin-only case. The reconciliation test's build-script limitation is a new finding below. |
| The alignment what-if misses configurations collapsed within one owner | Addressed for the reported case. Per-compile timing credits a configuration removed within one owner, and a fixture compares the forwarded-feature what-if with a wrapped build after declaring the alignment. Newly enabled optional dependencies remain explicitly outside the what-if model. |
| The guard-test fallback ruling remains unresolved | Open. The spec still selects isolation and a planner watch when the alignment crate is not built; neither exists. |

## Findings

### High — No remedy has reduced the workspace rebuilds

The spec's Outcome requires fewer workspace crate configurations and a shorter owner-job critical path. No source dependency was removed, no feature was isolated, and no alignment crate was created. The corrected Phase 4 table still has **90 → 90** Linux configurations and **65 → 65** divergent configurations (`attribution-2026-09-21.md`, “Phase 4 re-run”). This is a failure of the requested outcome, independent of later CI or cross-OS evidence.

The implementation log's temporary `schematic-define` dependency edit predicts **90 → 84** Linux configurations and about **77 seconds** saved on the measured selection, but the edit was reverted. Record a decision to include that remedy and verify it, or revise the spec's Outcome and acceptance criterion 5 to a measurement-only result. A future ordinary pull request's timing observation is explicitly non-gating and is not the reason for this finding.

### Medium — The guard-test fallback ruling still contradicts the implementation

The Open Questions ruling says to isolate the `claudine-cli` source-scan guard and add a planner watch if the alignment crate is not warranted. The implementation decided against that crate, yet left the guard and planner unchanged (`spec.md`, “Open Questions”; `implementation-log.md`, “Implementation of Review Findings #1”). Its measured saving is zero, so leaving it in place is plausible, but the written rule and implementation disagree. Record the author's ruling: either amend the fallback to retain the guard or implement the isolation and prove its scheduling.

### Medium — The wrapped-build reconciliation conflates configurations with build-script units

`a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations` compares one reported configuration per `(first owner, package)` with a count of **both** library and build-script rustc events (`scripts/feature-attribution-tests.rs:625-655`, `:658-677`). Its fixture has no build-script package, so the equality passes. The selected macOS timed pass does have them: for example, `claudine` first builds one `renderable` configuration but records both its library and build-script compilations. Repeating the test's count against the saved events gives seven owner/package mismatches, including `playa`, `renderable`, and `darkmatter-cli`. The model intentionally charges build-script time to a configuration (`scripts/feature-attribution.rs:1050-1097`); that does not make a build-script unit a second configuration.

Use a fixture with a real build script and reconcile library configurations and build-script units separately. Assert both that every modeled configuration has the expected library compile and that build-script seconds attach to the correct configuration. This is a **Level 1 test-quality gap**, not a request for Level 2 or Level 3 terminal testing.

### Low — Decision records still quote superseded attribution figures

The spec's `human_review_items` and `message_to_agent` still say 92 Linux configurations, a 92 → 86 remedy, and 0.6 seconds for the eligible trio (`spec.md:13-42`). The corrected report says **90**, **90 → 84**, and **0.9 seconds** (`attribution-2026-09-21.md`, “Phase 4 re-run”). That report also retains one 0.6-second explanation in the same section. Update these decision-facing figures so the author evaluates the current measurement.

## Verification level and reachability

| User-observable requirement | Strongest present verification | Assessment |
|---|---|---|
| Attribute feature divergence and name its cause | Level 1 Cargo-tree fixtures and model assertions | Appropriate boundary; 22 attribution tests are selected by L1. |
| Report configurations Cargo builds | Level 1 wrapped Cargo fixture plus saved timed pass | Bin-only correction is proven; build-script reconciliation is incomplete as described above. |
| Estimate seconds removed by an alignment | Level 1 per-compile and forwarded-feature tests, including a real aligned fixture build | Covers the previous same-owner and forwarding gaps; optional dependencies remain a documented model limit. |
| Reduce repeated workspace builds without hiding package feature needs | No remedy and no remedy regression test | Missing behavior; high finding above. |
| Keep each affected package buildable on its own | No package closure changed, so no new isolated-package verification applies | Recheck with Level 1 Cargo builds if a remedy changes a closure. |
| Preserve the existing archive reuse mechanism | Level 1 `one_owner_tree_shares_a_dependency_compile_without_unifying_features` | Appropriate; unchanged and passing. |

There are no keyboard, paste, mouse, color, scrolling, or terminal-emulator requirements in this spec. Level 2 and Level 3 are therefore not required. The `feature-attribution` binary is a declared target with `local-tools` enabled by `repo-deps`'s default and CI test configuration; its test names carry no higher-tier marker. `cargo nextest list -p repo-deps --bin feature-attribution` lists all 22 tests, and `just check-tier-coverage scripts` reports no stranded tests. Fixture reads use `repo_root().join(...)` with literal paths, which the test-input policy recognizes.

I ran the 22 attribution tests and the named archive reuse test with Nextest; all passed. `git diff --check` passed. These results verify test reachability and the behavior those tests assert; they do not establish a build-time reduction.
