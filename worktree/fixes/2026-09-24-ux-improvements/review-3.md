---
$schema: feature-review.yaml
ready: false
human_review: true
human_review_items:
    - |-
        Confirm the fourteen proposed implementation choices in the specification's “Proposed rulings from Phase 1 (awaiting the author's confirmation)” section, numbered 20–33. They cover removal safety, network time limits, shell behavior, stored records, and graph sizing. The implementation currently follows these proposals.

        - Accept the proposals as written.
        - Identify the choices to change and describe the replacement behavior before release.
    - |-
        Confirm whether the branch graph should remain visible in terminals narrower than 80 columns. The implementation shrinks it to fit, following the specification's sizing rule; the specification's human-review notes still leave this choice open.

        - Keep the smaller graph, as implemented.
        - Restore the previous behavior that hides the graph below 80 columns, and update the implementation and specification together.
reviewed_by: codex/default
created: 2026-09-25T10:28:52-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-3.md
previous: 2026-09-24-ux-improvements/review-2.md
next: 2026-09-24-ux-improvements/review-4.md
findings:
    - title: Remote deletion approval does not identify the remote repository
      priority: high
    - title: Handoff fingerprint misses changes to staged content
      priority: high
---

# Review 3: Worktree UX improvements

## Verdict

**Not ready for production.** The previous review's requested changes are implemented, including the author-approved PR badge fallback. Two additional removal-safety defects remain: the handoff can delete a branch in a different remote repository, and it can discard staged content changed after approval. Both were reproduced with the current binary and disposable repositories.

Pending author decisions and collection of cross-platform evidence are separate from this readiness judgment. No implementation code was changed for this review.

## Previous findings

The previous review contains three unblocked findings and one blocked finding. Its blocked PR-link finding now contains an explicit author decision: show the number without a visible URL when clickable links are unavailable. The implementation log records that decision before its corresponding specification edit.

| Previous finding | Review 3 assessment |
| --- | --- |
| Remote deletion handoff can delete an unapproved branch | The requested branch-name comparison is implemented before removal, and its same-commit/different-branch CLI regression passes in Level 1. Repository identity is a separate remaining hole described below. |
| Real-terminal tests depend on clearing stale pane output | Resolved. Table background variants and individual shell-removal scenes now receive fresh detached tmux sessions. The styling test and all five removal tests passed here, including fork-parent landing and failed handoffs. |
| Held-directory refusal lacks the specified Level 2 regression | Implemented. The Windows console test holds the target with a separate windowless process, invokes removal from the base checkout, checks exit 4 and the rendered refusal, and checks tracked, untracked, and ignored files, registration, and branch preservation. The implementation log records two passing native Windows runs and a negative control. I inspected the test; I did not rerun Windows. |
| Pull-request URLs disappear on terminals without clickable links | Resolved by the author's recorded decision. The specification now explicitly accepts number-only badges, matching the CLI, user documentation, and passing Level 1 fallback test. The suggested shared hyperlink abstraction was explicitly excluded from closure. |

## Unblocked Findings

### High: Remote deletion approval does not identify the remote repository

In the `worktree` package, [RemoteApproval](../../lib/src/remove/handoff.rs:55) carries a branch name and observed commit, but no remote repository identity. In `worktree-cli`, [run_handoff](../../cli/src/commands/remove/mod.rs:555), which resumes removal after the shell moves, compares those two values. The `worktree` package's [delete_remote_branch](../../lib/src/remove/remote.rs:108) subsequently pushes to the current `origin` configuration. A different repository can contain the same branch at the same commit, so these checks and the commit lease do not establish that deletion still targets the approved repository.

**Reproduction, using only local bare repositories:**

1. Create a base checkout on `main`, a linked worktree on `feat/x`, and two bare repositories named `approved.git` and `other.git`. Put `main` and `feat/x` at the same commit in both bare repositories. Set `origin` to `approved.git`.
2. Inside the linked worktree, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-remote` and retain the emitted landing directory and token.
3. From the base checkout, run `git remote set-url origin <other.git>`.
4. From the emitted landing directory, run `wt remove --handoff <token>`.

**Observed:** exit 0; worktree and local branch deleted; `feat/x` deleted from `other.git` and preserved in `approved.git`. The output says `Deleted origin/feat/x`.

I also reproduced the same result by changing only the push URL with `git remote set-url --push origin <other.git>`. This exposes an additional mismatch within the same defect: [LsRemote::live_head](../../lib/src/remove/live_remote.rs:43), the `worktree` library's live check, reads the fetch destination through `git ls-remote origin`, whereas deletion follows the push destination. The report and lease must describe the repository actually being modified.

**Required change:** resolve and record the effective deletion endpoint as well as the branch and commit; verify that endpoint before local removal and use it consistently for remote observation and deletion. Account for separate or multiple push URLs, either verifying each intended destination or refusing unsupported configurations before removing anything. A mutable remote nickname alone is insufficient.

**Verification:** add Level 1 CLI regressions with two local bare repositories for both URL changes above, asserting exit 3 and preservation of both remote branches, the worktree, and the local branch. Also cover a separate push URL configured before the first invocation, proving that the report and deletion consult the same endpoint. No real-terminal or physical-key test is needed for this Git-state requirement.

### High: Handoff fingerprint misses changes to staged content

In the `worktree` package, [Inventory::fingerprint](../../lib/src/remove/inventory.rs:102) is the content check used to refuse changed work after the shell moves. It hashes status letters, paths, and files read from the working directory. It never includes the staged file version stored in Git's index. Staged content can change while both the status letters and working-directory bytes remain identical.

**Reproduction:**

1. Create a linked worktree on `feat/x` at `main`, containing a tracked file.
2. Write `staged before` to that file and stage it. Then overwrite the working file with `working copy`, leaving status `MM` (both staged and unstaged changes).
3. Inside the worktree, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-worktree` and retain the landing directory and token.
4. Write `new staged work`, stage it, then restore the working file to `working copy`. Status is still `MM`; the staged version has changed.
5. From the landing directory, invoke `wt remove --handoff <token>`.

**Observed:** exit 0; the worktree and branch are removed. The new staged version loses its index reference rather than causing the required changed-state refusal. Git may retain its blob temporarily, but it is no longer present as staged work. This violates the specification's requirement to recheck modified, staged, and untracked content before honoring the recorded approval, even when force choices were recorded.

**Required change:** include a deterministic representation of staged state in the fingerprint, including staged object identities and modes and any conflict stages. Preserve the existing working-file and ignored-entry checks. Hashing Git's raw index file would include incidental cache metadata; use its logical entries instead. Update the fingerprint documentation to describe the actual coverage.

**Verification:** add a Level 1 fingerprint test and a CLI handoff regression for the unchanged-`MM` sequence above. Assert that the second invocation exits 3 and leaves the working file, staged contents, registration, and branch intact. The existing staging test changes status letters, so it does not detect this omission.

## Blocked Findings

None. Both findings can be implemented without an author decision. The two design confirmations in frontmatter remain external human-review items.

## Requirement verification

“Present” below describes retained tests; execution results are distinguished explicitly. These requirements need Level 1 logic tests or Level 2 terminal captures. None introduces a physical modifier, hotkey, mouse, or input-encoding contract requiring Level 3.

| User-facing requirement | Strongest relevant verification present | Assessment |
| --- | --- | --- |
| Branch/directory completion candidates, `base`, detached names, ambiguity errors | Level 1 resolver and temporary-repository tests | Appropriate; included in the passing area suite. |
| Safety tiers, force flags, ignored entries, PR source/head identity, lost-commit reporting, branch retention and exit codes | Level 1 policy matrix, stub providers, repository and CLI tests | Appropriate level. Additional remote-endpoint and staged-content cases are missing, as detailed above. |
| Removal report precedes prompts; one blank line; default is to retain work | Level 2 tmux captures and Level 1 policy tests | Passed here. These check displayed questions and answers rather than a physical keyboard encoder. |
| Dirty-file tree colors, connectors, and large-count replacement | Level 1 rendering assertions plus Level 2 styled tree capture | Appropriate; dirty-tree Level 2 passed here. |
| Bash, zsh, and fish move-first removal, base/fork-parent landing, subdirectory preservation, changed/expired handoff and failed `cd` | Level 1 protocol tests plus Level 2 wrapper scenes | All retained tmux removal tests passed here after capture isolation. The two newly reproduced state gaps still require Level 1 regressions. |
| PowerShell launched inside target; both working directories move; prompt and deletion succeed | Windows Level 2 ConPTY screen-buffer tests plus Level 1 wrapper tests | Correct level present; inspected, with execution recorded by the implementer rather than rerun here. |
| Windows held-directory refusal, exit 4, nothing removed | Windows Level 2 ConPTY refusal test | Previous test-level gap is closed in code; same execution qualification as above. |
| `wt go`/`wt create` wrapper detection; explicit local `--from`; detached/error cases; fork-origin persistence | Level 1 CLI, wrapper, and repository tests | Appropriate; area suite passed. |
| Table caption, comparison vocabulary, fork-parent tree, deleted parents, PR placement and cache-age text | Level 1 snapshots/data tests plus Level 2 table captures | Appropriate division of logic and rendering; area suite and tmux captures passed. |
| Table dots, badge backgrounds, conflict connectors, current-row emphasis on dark/light backgrounds | Level 2 styled-cell assertions, supported by Level 1 style tests | Passed here with fresh sessions for both backgrounds. |
| Clickable PR badge where supported; number-only fallback elsewhere | Level 1 OSC 8/fallback assertions plus Level 2 badge display | Matches the author's revised contract. |
| Graph lanes, tags, origin divergence, parent/PR labels, commit/lane trimming and scale arithmetic | Level 1 graph facts, generated Mermaid, and sizing tests | Correct level for structure and arithmetic; worktree-side tests passed. Cross-package component tests were inspected but not rerun. |
| Actual graph pixels, narrow-window width, short-window height cap and notice, table surviving image output | Level 2 private Kitty screenshots, text capture, and recorded image transmission | Correct level exists. Review 2 and implementation log record passes. This review's narrow-window test failed on a blank content capture; current screenshot evidence is unavailable, as explained below. |
| Warm/cold gathering and full-command time; slow/offline PR request; fresh cache suppresses request | Level 1 timed CLI and request-count performance tests | All 17 passed through the serial performance recipe. |
| Four-provider PR normalization, unavailable authentication, source identity and merged-head handling | Level 1 loopback-provider tests in sniff's declared test suite | Appropriate; inspected target/module wiring, not rerun in this iteration. |
| Renderer upgrade and pie-color contrast | Level 1 SVG/color assertions in biscuit-visualized | Appropriate; retained HSL color parsing and updated pie assertions inspected, not rerun in this iteration. |

No additional wrong-level test gap was found. Missing cases in the two findings concern state coverage within Level 1, not a need to escalate them to a terminal test.

## Validation and limitations

- `worktree/just test`: **289 passed**, 17 excluded by the area's filter.
- `worktree/just test-perf`: **17 passed**.
- `worktree/just lint`: passed.
- `just check-tier-coverage worktree`: **zero stranded tests**.
- `BISCUIT_TEST_REQUIRED_BACKENDS=kitty,tmux just test-l2`: dirty-tree test passed; narrow-window Kitty test failed after 13.35 seconds; fail-fast left nine tests unrun.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_list level2_move level2_remove`: **8 passed**, covering all remaining tmux cases. Together these runs prove all nine tmux tests passed.
- Three disposable-repository probes reproduced the two findings: changed origin URL, changed push URL, and changed staged content with unchanged status/working bytes. All unexpectedly returned 0 and removed the worktree. No external remote was modified.

The Kitty failure occurred in the screenshot assertion after text and transmission checks. I inspected the saved screenshot: the title bar is visible, but the entire content area is black, including the table text that Kitty's text capture had already returned. That is consistent with the test's documented Screen Recording restriction and does not isolate a graph renderer defect. I did not weaken assertions, change permissions, or repeat the same screenshot test. Re-run both Kitty cases from a capture-capable session to obtain current screenshot evidence; the short-window case was not reached in this run.

The Windows test is an explicit Cargo target requiring `terminal-tests`; the live `test-l2` recipe and package CI feature metadata enable that feature. It is Windows-gated and therefore did not execute on this Mac. Windows CI still lacks the declared backend provisioning for these console tests, as recorded in the implementation log. That unmet cross-OS scheduling criterion is retained here, but is not itself a production-readiness finding under this review's instructions.

The requested previous-review path under `prompts/_reviews/worktree/…` does not exist. The existing review beside the specification is the file updated with `implemented: true` and `next: 2026-09-24-ux-improvements/review-3.md`. The specification's `review_iterations` is set to 3; it is not marked completed.

## Author decisions

DECISION (2026-09-25): Decisions 20–33 are confirmed as written. Decision 21 is amended by this review's first finding: the approval records the resolved deletion endpoint, and the live check and the deletion both use it.

DECISION (2026-09-25): keep the smaller graph in terminals narrower than 80 columns (spec Decision 34).
