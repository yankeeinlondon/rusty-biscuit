---
$schema: feature-review.yaml
ready: false
findings:
    - title: Copying can follow a source link created after path validation
      priority: high
    - title: The new create report lacks real-terminal verification
      priority: high
    - title: The ignored-file summary can omit disposable files in a protected directory
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-25T20:27:48-07:00
spec: 2026-09-25-worktree-file/spec.md
implemented: true
next: 2026-09-25-worktree-file/review-2.md
log: worktree/fixes/2026-09-25-worktree-file/implementation-log.md
implemented_by: codex/default
description: "A **fix** review of `2026-09-25-worktree-file/spec.md`"
fix: 2026-09-25-worktree-file/review-1.md
---

# Review 1: `.worktreeinclude` support

## Verdict

**Not production ready.** The normal create, removal-consent, and handoff paths have useful tests, but one copy boundary can be crossed if the source changes during copying. The new create report also lacks the real-terminal test required for its wrapping behavior.

## Findings

### High: Copying can follow a source link created after path validation

In the `worktree` library, [copy_include_set](../../lib/src/include/copy.rs:103) checks that a source path is a regular file, then [copy_file](../../lib/src/include/copy.rs:169) opens the path later through the clone or byte-copy operation. If another process replaces that file with a symbolic link between those steps, the copy operation can follow the link and copy bytes from outside the source checkout. A similar replacement of a checked destination parent can redirect the temporary file outside the destination checkout. This breaks the spec's explicit source and destination boundaries, even though the checks work for paths that remain stable.

Hold validated file or directory handles through the copy, or use another platform-specific method that prevents a later path substitution from redirecting the operation. Add a deterministic Level 1 test using the existing injected copy operations to swap the source after validation, plus a destination-parent swap test. Assert that no outside bytes are copied and no outside path is created. The existing ancestor-link tests cover links present *before* validation, so they do not exercise this case.

### High: The new create report lacks real-terminal verification

The spec requires the copied-files line and warnings to wrap long names and render safely in the terminal. In `worktree-cli`, [render_include_report](../../cli/src/commands/create.rs:88) uses terminal components, but [create_include.rs](../../cli/tests/create_include.rs:50) only captures a process's stderr at Level 1. No Level 2 test captures this report from a real terminal at a narrow width. Level 1 confirms the text and output channel; it cannot confirm the terminal's wrapping and rendered layout. The strongest test is therefore at the wrong level for this user-visible requirement.

Add a focus-preserving Level 2 scene through the existing terminal harness. Capture a narrow pane containing several long copied paths and a partial-copy warning, then assert that names remain visible, wrap within the pane, and leave the shell protocol on stdout unchanged. Keep the Level 1 byte and escaping assertions.

### Medium: The ignored-file summary can omit disposable files in a protected directory

In the `worktree` library, [disposable_ignored_names](../../lib/src/remove/inventory.rs:45) removes a top-level ignored directory from the summary whenever any protected included file lies beneath it. A temporary Git repository with `.gitignore` containing `config/` and `.worktreeinclude` containing `config/.env` confirms the reachable case: Git status reports one ignored `config/` entry, while include matching selects `config/.env`. If `config/cache.bin` also exists, the code drops `config/` from the summary and never names that disposable content. This conflicts with the spec's one-line summary of ignored entries that need no consent.

Build the summary from the actual ignored paths and protected paths without discarding a whole directory merely because one child is protected. Add a Level 1 report test for a mixed directory and a Level 2 capture if the display shape changes materially.

## Requirement verification

| User-observable requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Git ignore and include-pattern intersection, rule precedence, unusual names, and repository boundaries | Level 1 tests against temporary Git repositories | Appropriate for file selection; source-path substitution remains untested. |
| Create source choice, copied contents, copy records, permissions, links, partial failure, and stdout protocol | Level 1 library and binary tests | Appropriate for content and exit behavior; the copy boundary finding remains. |
| Copied-files line, warnings, escaping, and wrapping in a terminal | Level 1 captured stderr | **Level mismatch:** wrapping needs Level 2 terminal capture. |
| New, changed, unknown, unchanged, and ordinary ignored-file removal decisions | Level 1 policy and binary tests; Level 2 tmux prompt scenes | Appropriate for consent and default answer; mixed-directory summary is untested and incorrect. |
| Changed included content, rules, baseline, source, dirty files, and index across the removal handoff | Level 1 binary tests; a Level 2 tmux handoff scene | Appropriate. These commands have no new physical-key encoding requirement, so Level 3 is not required. |

The new Level 1 integration files are compiled through `worktree-cli`'s automatic test discovery. The Level 2 removal tests are declared targets, have the `level2_` name marker, and use the enabled `terminal-tests` feature. The package has a live `test-l2` recipe. The tier audit reports zero stranded tests.

## Validation

- `worktree/just test`: 388 passed, 17 excluded by the Level 1 tier filter.
- `just check-tier-coverage worktree`: zero stranded tests.

The implementation log records passing targeted Level 2 removal tests and cross-platform runs. I inspected their assertions but did not rerun Level 2 in this review. This review changes only the review document and the spec's `review_iterations` value. No human design decision is needed to address the findings.
