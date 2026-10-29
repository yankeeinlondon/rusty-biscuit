---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - Windows timing evidence covers a different workload
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-06T18:06:39-07:00"
spec: 2026-10-06-archive-key-helper/spec.md
implemented: false
description: "A **fix** review of `2026-10-06-archive-key-helper/spec.md`"
fix: 2026-10-06-archive-key-helper/review-1.md
---

# Review 1

**Production ready.** No blocking implementation defect was found. The
archive consumers use the transferred verifier as the planner's key helper,
and key requests clear compiler-wrapper mode in the child without changing
the parent's measurement environment. One discrepancy in the recorded
Windows validation is noted below. Cross-OS evidence is excluded from
readiness under this review's instructions.

## Findings

None.

## Verification

The review covered the verifier-output recipe, the CI archive gate, both
cross-check script generators, the `repo-deps` helper subprocess boundary,
their changed contracts, and the matching documentation. Native branches
retain their existing helper resolution. No file-format or configuration
reader changed, so the input robustness matrix does not apply.

| Spec requirement | Verification level and result |
| --- | --- |
| Verification reports the exact helper only after success | Level 1: recipe contract passed; executing the extracted recipe with a temporary verifier confirmed success emits the exact spaced path and failure emits no output. |
| Archive gate rejects missing or empty output and overrides inherited helper selection | Level 1: workflow contract passed; executing the extracted gate shell with a recording replacement for `just` confirmed both refusals, a spaced helper path, and unchanged native override behavior. |
| Unix and Windows scripts bind the transferred helper between verification and testing | Level 1: generated-script tests passed for Linux, macOS, WSL, and Windows, including quoting, `.exe` spelling, and native-mode exclusion. |
| Contracts detect removal of the binding, export, or recipe output | Level 1: independently removed each line, ran its owning contract, confirmed the intended assertion failed, and restored the original file bytes. |
| Resolver boundaries, unchanged keys, and child environment isolation | Level 1: all resolver, pinned-digest, canonicalization, and subprocess-environment tests passed. |
| Focused Windows archive execution and workload durations | Existing implementation log records the alternate `repo-deps` planner test using the consumer's helper with working Python in 4.755 seconds. The original workload remains present; its Windows timings are not established by that record. |

Checks run during this review:

- `just test test-toolkit`: 461 passed, 4 skipped; both changed workflow
  contracts executed. The original planner-to-guard tests also executed.
- `python3 -m unittest scripts/ci/test_build_key.py scripts/ci/test_cross_check.py`:
  41 passed.
- `just check-tier-coverage tools/test-toolkit`: no stranded tests. The Rust
  contracts are compiled integration targets selected by L1; both Python
  suites are registered `repo-deps` companion suites.
- `bash -n scripts/cross-check.sh` and `git diff --check`: passed.

These subprocess and wiring requirements need Level 1 verification; none
requires terminal rendering or keyboard injection. The Windows record was
reviewed, not rerun. No CI scheduling, key protocol, or digest implementation
changed, and no additional abstraction or performance change is warranted.

## Observations

### Windows timing evidence covers a different workload

The [implementation log](implementation-log.md:114) says the two
`the_shipped_planner_*` tests no longer exist and therefore the 30-second
timing check does not apply. Both remain in the `test-toolkit` package's
[workflow contract suite](../../tools/test-toolkit/tests/ci_workflow_contracts.rs:4024)
and passed locally in 1.728 and 16.972 seconds. The recorded Windows run of
the `repo-deps` package's
[alternate planner test](../../scripts/ci-rollup-tests.rs:4881) supports helper
selection, but the spec permits that substitution only after the original
tests are retired. Its 4.755-second duration does not establish that the
original Windows slowdown is resolved. The record should be read as evidence
for the alternate workload only; original Windows timings remain follow-up
validation, not a blocking implementation finding under this review's
cross-OS evidence exclusion.
