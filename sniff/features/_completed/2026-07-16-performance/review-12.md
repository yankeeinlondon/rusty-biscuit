---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T11:18:25-07:00
spec: 2026-07-16-performance/spec.md
implemented: false
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-12.md
---

# Review 12

## Findings

### High: native Linux and Windows execution and matched work-count artifacts remain absent

The feature's production completion boundary still requires the cross-platform tests to pass
natively on macOS, Linux, and Windows and the scheduled matrix to retain comparable work-count
artifacts ([spec.md](spec.md#ci), [spec.md](spec.md#acceptance-criteria)). The implementation record
continues to state that native Linux and Windows Level-1 execution and a matched three-OS artifact
set have not occurred
([deferred-perf-tests.md](deferred-perf-tests.md#review-11-deferred-items),
[Phase 8](phases/_completed/08-cross-platform-validation/spec.md#completion-boundary)).

The current `HEAD` is `2eed94a51f9943596d9de549130cc3bfac0c4dc7`, and
`git branch -r --contains` finds no fetched remote branch containing it. The current workspace also
contains the final phase-reference repairs outside that commit. Consequently, neither the native
`sniff-cross-platform` job nor the three-OS `sniff-work-counts` matrix can have produced evidence
for the exact final reviewed tree. The workflow definitions are appropriate future coverage, but a
definition is not an execution record. Windows GNU cross-compilation is useful supplemental compile
evidence; it does not exercise Windows Job Objects, registry/path behavior, or service detection,
and it supplies no native Linux `/proc` descendant-cleanup evidence.

Publish one immutable final implementation identifier, run the canonical Level-1 matrix natively
on all three operating systems, run the work-count matrix for that same identifier, and retain the
three per-OS artifacts. Record the identifier and workflow/artifact references in the completion
evidence before marking the umbrella feature complete.

## Resolved Since Review 11

- All eight phase specifications and the Phase 1 log are restored under `phases/_completed/`, which
  follows the feature lifecycle convention while preserving the historical evidence.
- Production rustdoc now targets the archived paths. The Sniff skill and scheduled performance
  workflow have corresponding path repairs in the current workspace. A repository-wide reference
  audit found only the deliberately preserved historical references to the Phase 1 review file that
  never existed; they describe why that review cycle was blocked rather than acting as live links.
- No new correctness, ergonomics, or performance defect was found in R1-R14. The implementation
  keeps the request-scoped observation, manifest, ownership, Git, remote, subprocess, and executable
  indexes at the intended library boundaries, and the profile-gated R14 deferrals remain supported
  by the archived measurements.

## Verification Levels

| User-observable requirement | Strongest present verification | Review result |
|---|---|---|
| R2 aggregate JSON schema, ordering, valid-JSON-only stdout, exit behavior, one status walk/discovery, and observation-free projection | Level 1 spawned-CLI, snapshot, and work-counter tests | Appropriate tier; previously green on macOS and unchanged by the documentation-only review-11 implementation. |
| R5/R7 shallow structure values and serialized inventory `truncated`/`limit` behavior | Level 1 unit, fixture, and serialization tests | Appropriate tier; these are data-contract requirements, not terminal-emulator behavior. |
| R11/R12 WAN fallback, default NTP gating, bounded service/subprocess behavior, partial timeout results, and installation timeout warning/non-zero verdict | Level 1 wiremock, subprocess, PTY/interview, and composed command-boundary tests | Appropriate tier; no exact terminal styling is specified for the timeout warning. |
| Changed styling-sensitive Git-status and CI/CD terminal output | Level 2 tmux pane capture | Appropriate tier; real-terminal glyph/SGR rendering is covered. |
| macOS, Linux, and Windows runtime behavior, including Linux `/proc` cleanup and Windows Job Object, registry/path, and service paths | Native macOS Level 1 plus Windows GNU cross-compilation | Insufficient. Native Linux and Windows Level-1 execution is required and absent. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No feature requirement | Level 3 is not applicable. |

Internal performance requirements R1-R14 are additionally pinned by Level-1 work-counter tests for
walker scope, one enumeration, one enrichment/parse, saturation, one dirty-side load/diff, focused
metadata, bounded history, remote request counts, subprocess batching/timeouts, and eager index
reuse. Those tests are the appropriate verification level because the requirements concern work
performed and serialized results rather than terminal encoding.

## Checks Run

```text
sniff repo packages
  succeeded; confirmed sniff and sniff-cli workspace packages

implementation and test audit
  mapped R1-R14 to direct work-counter/unit/integration coverage and inspected the L2 suite

git branch -r --contains 2eed94a51f9943596d9de549130cc3bfac0c4dc7
  no fetched remote branch contains the reviewed HEAD

phase-reference audit
  all live phase evidence links resolve to phases/_completed; only intentional historical
  references to the never-created Phase 1 review file retain the old spelling

just test  # sniff/
  stopped with exit 130 after the non-interactive 60-second boundary while compiling dependencies;
  no test failure occurred, but this attempt is not reported as a pass

review-11 canonical evidence
  sniff: 1,677 passed, 19 skipped
  sniff-cli: 782 passed, 3 skipped
  just lint passed on macOS
```

Review 11's documentation-evidence finding is resolved, and no additional implementation defect was
found. The specification's required native Linux/Windows execution and matched three-OS work-count
artifacts remain absent. Production readiness: **not ready**.
