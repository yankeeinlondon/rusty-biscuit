---
$schema: feature-review.yaml
ready: false
findings:
  - priority: high
    title: Encoded Azure SSH path segments are double-encoded in commit URLs
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T18:35:06-07:00
spec: 2026-09-15-recent-commits/spec.md
implemented: true
implemented_by: claude/default
log: sniff/features/2026-09-15-recent-commits/implementation-log.md
description: "A **feature** review of `2026-09-15-recent-commits/spec.md`"
feature: 2026-09-15-recent-commits/review-2.md
previous: 2026-09-15-recent-commits/review-1.md
next: 2026-09-15-recent-commits/review-3.md
---

# Review 2 — Recent Commits

## Verdict

The feature is **not ready for production**. Four of review 1's five findings
are fully resolved, and the new Level 1 and Level 2 regressions exercise the
right boundaries. The provider-link finding is only partially resolved:
SourceHut and ordinary Azure SSH URLs now link correctly, but an Azure SSH URL
whose project or repository name is already percent-encoded produces a
double-encoded, invalid browser URL.

No review 1 finding was blocked, so there was no blocked item to reclassify.
The remaining defect has a bounded code and Level 1 test fix and does not
require human review.

## Review 1 Finding Closure

| Review 1 finding | Status | Evidence |
| --- | --- | --- |
| Hash-bounded walks can omit descendants in a skewed merge history | Closed | Hash selection now hides the boundary's parents and walks graph membership rather than stopping at commit-time position. The original skewed merge topology is pinned in `selection::hash_bounds_the_range_by_graph_membership_not_by_commit_time`. |
| Valid breaking-change Conventional Commits are treated as non-conventional | Closed | Both `feat!:` and `feat(api)!:` retain operation/scope through parser, public collection, filtering, payload, and shipped-CLI tests. |
| Large duration scopes panic instead of returning `InvalidPeriod` | Closed | Checked multiplication and `Duration::try_seconds` reject each oversized unit; window subtraction saturates at `DateTime::MIN_UTC`. Unit boundary tests and all three shipped commit-family commands prove non-panicking typed errors with empty stdout. |
| SourceHut and Azure SSH remotes receive invalid commit URLs | Partially closed | ASCII SourceHut/Azure transport forms and containment are fixed, but encoded Azure SSH path segments remain broken (finding below). |
| The Level 2 test does not prove the normative style-to-span mapping | Closed | The tmux test decodes captured cells and associates bold, blue, dim, italic, the commit URL, and the file URL with their exact text spans. It ran against real tmux with backend proof: 2 passed, 0 skipped. |

## Findings

### High — Encoded Azure SSH path segments are double-encoded in commit URLs

`parse_remote_identity` obtains the path of a URL-form remote through
`url::Url::path()`, which retains percent escapes
([`commit_links.rs:57`](../../lib/src/filesystem/git/commit_links.rs#L57)).
The Azure normalization then passes each already-encoded segment through
`urlencoding::encode`
([`commit_links.rs:146`](../../lib/src/filesystem/git/commit_links.rs#L146)).
Consequently `%20` becomes `%2520`.

This is reachable with valid Azure names: Azure permits spaces in Git
repository names, and URL-form SSH remotes must represent such spaces with
percent escapes. A shipped-CLI reproduction used:

```text
ssh://git@ssh.dev.azure.com/v3/acme/My%20Project/My%20Repo
```

with a containing `refs/remotes/origin/main`. The JSON payload reported
`remote: true` but emitted:

```text
https://dev.azure.com/acme/My%2520Project/_git/My%2520Repo/commit/<sha>
```

instead of preserving one encoding layer. This means review 1's provider URL
finding is not fully implemented. The table-driven provider test uses only
ASCII path segments
([`commit_links.rs:439`](../../lib/src/filesystem/git/commit_links.rs#L439)),
and the public collection regression does the same
([`recent_commits.rs:906`](../../lib/tests/recent_commits.rs#L906)), so both
stay green.

Normalize from decoded path segments and encode each segment exactly once, or
preserve valid existing escapes while encoding only raw segment bytes. Add
table-driven unit cases for spaces and non-ASCII names in both SCP-style and
URL-form Azure remotes, plus one `RecentCommits::collect` regression asserting
the exact `commit_url`. The regression must distinguish `%20` from `%2520`.

## Requirement Verification Levels

| User-observable requirement | Required level | Strongest evidence present | Assessment |
| --- | ---: | --- | --- |
| Bare-array JSON schema, author, moved files, line counts, UTC datetimes, attribution, and three-state containment | L1 | Library unit/integration tests plus shipped-CLI process tests | Correct level; passes. |
| Count/date/duration/hash/branch selection and operation/scope/author/package/file-type filters | L1 | Real temporary-repository integration tests plus CLI process tests | Correct level; review 1's selection/parser/input gaps are closed. |
| Provider-specific commit URL normalization and containment | L1 | URL tables plus public collection tests | Correct level, but incomplete encoded-path coverage exposes the finding. |
| Empty-result exit status and stdout/stderr routing, including valid JSON stdout | L1 | CLI subprocess tests across all three commands | Correct level; passes. |
| Plain and Markdown degradation, projections, escaping, and library-authored report bytes | L1 | Library render tests and CLI subprocess tests | Correct level; passes. |
| Word wrapping, exact SGR style-to-text mapping, file-link fallback, and linked-hash rendering | L2 | Real tmux cell/style/link capture with required-backend proof | Correct level; 2 passed, 0 skipped. |
| Keyboard, hotkey, paste, IME, or mouse behavior | L3 | No such feature requirement | Not applicable. |

## Validation Performed

- `cargo nextest run -p sniff --features remote --test recent_commits`: 33
  passed, 0 failed, 0 skipped.
- `just test` in `sniff/`: 2,748 passed, 0 failed, 24 tests excluded by the
  canonical tier filter.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 --test
  level2_recent_commits_rendering` in `sniff/`: 2 passed, 0 failed, 0 skipped;
  backend proof recorded two tmux executions.
- `just lint -D warnings` in `sniff/`: passed.
- Strict all-target clippy passed for `sniff`, `sniff-cli`, and `sniff-cli`
  with `test-fixtures` enabled.
- A disposable real Git repository reproduced the Azure `%20` → `%2520`
  defect through the built `sniff` CLI; the fixture was removed afterward.
- GitNexus index `sniff` was refreshed at commit `96b63d7a0`. Its untruncated
  change analysis reports medium risk, 42 changed symbols, and one affected
  process (`branch_exists_on_remote_at` through `provider_https_git_url`). The
  full L1 run includes the shared canonical-endpoint tests for that process.

## Production Readiness

Not ready. Preserve exactly one percent-encoding layer when normalizing Azure
SSH repository paths and add the missing Level 1 regressions; no other blocker
was found in this iteration.
