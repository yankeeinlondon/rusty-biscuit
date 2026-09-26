---
$schema: feature-review.yaml
ready: true
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T12:23:25-07:00
spec: 2026-09-25-worktree-file/spec.md
implemented: false
description: "A **fix** review of `2026-09-25-worktree-file/spec.md`"
fix: 2026-09-25-worktree-file/review-2.md
previous: 2026-09-25-worktree-file/review-1.md
---

# Review 2: `.worktreeinclude` support

## Verdict

**Production ready.** The three unblocked findings from the first review are addressed. That review listed no blocked findings, so there was nothing to reassess for an earlier unblock. This review found no remaining implementation or test gap that blocks the specification.

## Prior findings

### Copying can follow a source link created after path validation — resolved

The `worktree` library's [copy_include_set](../../lib/src/include/copy.rs) now keeps open handles for the source file and destination directory while it copies and publishes a file. A replacement of the source name therefore cannot redirect the copy to an outside file; a replacement of a destination parent cannot redirect the write. Level 1 tests replace each path during an injected copy operation and check both the copied bytes and the outside location. The source-swap test also checks that the copy receives no trusted baseline after its source name changes.

### The new create report lacks real-terminal verification — resolved

The `worktree-cli` package declares [level2_create](../../cli/tests/level2_create.rs) as a test target with its terminal-test feature. The area's live `test-l2` recipe selects it. In a headless tmux pane narrowed to 48 columns, the test captures several copied names and a partial-copy warning, checks their wrapping and visibility, verifies the copied files, and checks that stdout contains only the shell movement protocol. The focused Level 2 run passed.

### The ignored-file summary can omit disposable files in a protected directory — resolved

The `worktree` library's [expand_mixed_ignored](../../lib/src/remove/inventory.rs) expands a Git-collapsed ignored directory when it also contains a protected included file. Its Level 1 repository test checks that the summary names the disposable descendants while leaving the protected file out. A `worktree-cli` Level 1 test checks the displayed summary and confirms that refusal leaves both files in place. Ordinary ignored directories retain their single-name summary.

## Requirement verification

| User-visible requirement | Strongest verification | Assessment |
| --- | --- | --- |
| Git ignore and include-pattern intersection, rule precedence, unusual names, and repository boundaries | Level 1 tests against temporary Git repositories | Appropriate for filesystem selection. |
| Create source choice, copied contents, permissions, links, records, partial failures, and stdout protocol | Level 1 library and CLI tests | Appropriate for content, persistence, and exit behavior. The source and destination path-swap cases now have deterministic Level 1 tests. |
| Copied-files report and warning layout in a narrow terminal | Level 2 tmux pane capture, with Level 1 output and escaping checks | Appropriate for rendered wrapping and visibility. No physical key behavior is introduced, so Level 3 is not required. |
| New, changed, unknown, unchanged, and ordinary ignored-file removal decisions | Level 1 policy and CLI tests; Level 2 tmux prompt scenes | Appropriate for consent and prompts. The mixed-directory summary now has a Level 1 display test. |
| Removal handoff after included-content, rules, baseline, source, dirty-file, and index changes | Level 1 CLI tests and a Level 2 tmux handoff scene | Appropriate for state verification and the visible prompt flow. |

The new Level 1 tests are compiled by the library unit target or the automatically discovered CLI integration target. The new Level 2 test is a declared target, has the required name marker and feature, and is selected by a live recipe. `just check-tier-coverage worktree` found no stranded tests.

## Validation

- `worktree/just test`: 392 passed, 17 excluded by the Level 1 tier filter.
- `worktree/just test-l2 level2_create`: 1 passed.
- `worktree/just lint`: passed for the library and CLI.
- `just check-tier-coverage worktree`: zero stranded tests.
- `git diff --check`: passed before writing this review.

These local runs verify the revised behavior and test placement. Cross-platform execution evidence belongs to the separate CI process and is not a readiness condition for this review.
