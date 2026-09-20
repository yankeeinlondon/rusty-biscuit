---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T20:32:14-07:00
spec: 2026-09-15-recent-commits/spec.md
implemented: false
description: "A **feature** review of `2026-09-15-recent-commits/spec.md`"
feature: 2026-09-15-recent-commits/review-3.md
previous: 2026-09-15-recent-commits/review-2.md
---

# Review 3 — Recent Commits

## Verdict

The feature is **ready for production**. Review 2's sole unblocked finding is
closed, no blocked finding existed to reclassify, and this iteration found no
new specification, correctness, performance, ergonomics, or feature-specific
test-rigor gap.

No human review is required. The remaining cross-platform execution belongs to
CI/CD evidence collection and does not change feature readiness.

## Review 2 Finding Closure

| Review 2 finding | Status | Evidence |
| --- | --- | --- |
| Encoded Azure SSH path segments are double-encoded in commit URLs | Closed | URL-form segments are decoded and then encoded exactly once in `canonical_repository_url`. The provider table covers spaces and non-ASCII names in URL and SCP forms; the public `RecentCommits::collect` regression asserts the exact `%20` URL and rejects `%25`; live branch observation consumes the same canonical URL function and pins the encoded endpoint. |

The fix preserves the library as the authority. Both commit linking and the
SSH-provider live-observation fallback consume
[`canonical_repository_url`](../../lib/src/filesystem/git/commit_links.rs), so
the CLI performs no URL normalization of its own. GitNexus reports two direct
callers, seven upstream impacts, one affected Git process, and low risk after a
fresh index rebuild.

## Findings

None.

## Requirement Verification Levels

| User-observable requirement | Required level | Strongest evidence present | Assessment |
| --- | ---: | --- | --- |
| Bare-array JSON schema, author, moved files, line counts, UTC datetimes, attribution, and three-state containment | L1 | Library integration tests and shipped-CLI subprocess tests | Correct level; passes. |
| Count/date/duration/hash/branch selection and operation/scope/author/package/file-type filters | L1 | Manufactured Git repositories through the public library and shipped CLI | Correct level; passes. |
| Provider URL normalization, including encoded Azure SSH names, and containment | L1 | Provider tables, shared live-endpoint test, and public collection regression | Correct level; passes. |
| Empty-result exit status and stdout/stderr routing, including valid JSON stdout | L1 | Shipped-CLI subprocess tests for all three commit-family commands | Correct level; passes. |
| Plain and Markdown degradation, projections, escaping, and library-authored report bytes | L1 | Library render tests and CLI subprocess tests | Correct level; passes. |
| Word wrapping, exact SGR style-to-text mapping, file-link fallback, and linked-hash rendering | L2 | Real tmux cell/style/link capture with required-backend proof | Correct level; 2 passed, 0 skipped. |
| Keyboard, hotkey, paste, IME, or mouse behavior | L3 | No such feature requirement | Not applicable. |

## Validation Performed

- `cargo nextest run -p sniff --features remote --test recent_commits`: 34
  passed, 0 failed, 0 skipped.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 --test
  level2_recent_commits_rendering` in `sniff/`: 2 passed, 0 failed, 0 skipped;
  backend proof recorded two tmux executions.
- `just lint -D warnings` in `sniff/`: passed.
- Strict all-target Clippy for `sniff-cli` with `test-fixtures`: passed.
- Strict all-target Clippy for `sniff` with `remote` reached three pre-existing
  `redundant_closure` warnings in `remote_observation.rs` and
  `focused_provider.rs`; none is in this feature's diff or execution path.
- The full `just test` run passed 2,236 tests, including the recent-commits and
  Azure regressions, before 13 host/hardware discovery tests timed out and
  fail-fast cancelled the remaining 500. A serial rerun isolated the same
  timeout to the pre-existing macOS audio detector. This is not feature
  evidence and is not counted as a passing full-suite gate.
- GitNexus was rebuilt at `96b63d7a0`. Untruncated worktree change analysis
  reports medium risk, 47 changed symbols, and one affected process
  (`branch_exists_on_remote_at` through `provider_https_git_url`); its shared
  endpoint regression passed in the L1 run.
- `git diff --check`: passed.

## Production Readiness

Ready. The encoded Azure SSH path defect is fixed without duplicating business
logic in the CLI, its dependent projections are covered at the appropriate
levels, and no unresolved review finding remains.
