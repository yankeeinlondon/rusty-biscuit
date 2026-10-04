---
$schema: feature-review.yaml
ready: false
findings:
    - title: Replacement directory links bypass removal guards and allow file deletion
      priority: critical
    - title: Missing-record removal discards index changes made after inspection
      priority: high
    - title: New listing markers and recovery notes lack real-terminal verification
      priority: high
human_review: true
human_review_items:
    - |-
        Resolve the specification's still-open choice about repair affecting other worktrees. Git's repair command can restore missing links and overwrite unusable `.git` files in other registered worktrees, even when only one path is named. The implementation and documentation currently accept and disclose this behavior. Choose whether that remains the intended contract:

        - Accept Git's repair behavior and disclose its effects on other worktrees (current implementation).
        - Refuse automatic repair when another worktree has a broken link; this adds checks and still leaves a race.
        - Write and verify only the target's link, which requires supporting Git's link-file format and relative paths.

        This is an existing unresolved design choice, not a request to reaffirm a previously accepted decision. It does not block fixing the code and test findings below.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-04T00:46:35-07:00
spec: 2026-10-03-broken-worktree-entries/spec.md
implemented: true
next: 2026-10-03-broken-worktree-entries/review-2.md
description: "A **fix** review of `2026-10-03-broken-worktree-entries/spec.md`"
fix: 2026-10-03-broken-worktree-entries/review-1.md
---

# Review 1: Broken worktree entries

**Not production ready.** Two reproduced cases delete work that the new safeguards should protect. A separate terminal-verification gap remains. None of these findings requires human input to fix. The specification's existing unresolved repair-side-effect choice is carried above for the later human review.

The review covered the specification, plan, implementation log, changed library and CLI paths, current documentation, worktree skill, and test placement. Related changes to remote refresh were excluded except where they intersect this fix. No production source code was changed during review.

## Findings

### Critical: Replacement directory links bypass removal guards and allow file deletion

**Defect class:** Treating the absence of Git's `prunable` marker as proof of a safe filesystem target allows replacement directory links through preparation, repair verification, and handoff validation.

In the `worktree` package, [classify_with](../../lib/src/availability.rs:71), which determines whether a checkout is available, returns `Healthy` without inspecting any entry that lacks `prunable`. Git does not mark a directory symlink as prunable when the symlink still leads to a readable checkout. Consequently, the `worktree` package's [prepare](../../lib/src/remove/mod.rs:70), which prepares removal before inventory, does not enforce the specification's unconditional refusal of a target that is itself a link.

**Reproduction through the shipped CLI:** Create a local repository on `main` with a committed file `a`; add a linked worktree `target` on branch `topic`. Move `target` to `saved`, then create a directory symlink `target -> saved`. Run `wt remove topic --force-worktree --force-branch` from the base checkout. Git's listing has no `prunable` marker. The command reports no uncommitted files, then exits 1 with `failed to delete .../target: Not a directory`. **The committed file `saved/a` has already been deleted.** The symlink remains. These were disposable, isolated repositories; no user checkout was modified.

The same loss occurs without a force flag when the checkout is clean. A failing Git exit is too late to serve as the link guard.

| Site / sibling path | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Ordinary preparation and execution in `worktree-cli` | Readable directory symlink replacing the listed target | Followed link; deleted `saved/a`; exit 1 | Refuse with exit 3 before inventory/removal; preserve all files |
| Repair verification in `worktree` | A Git shim delegates repair, then moves the repaired directory to `saved` and puts a symlink at the original path | Reports verified restoration; deletes `saved/a`; exit 1 | Reject replacement link after repair; preserve all files |
| First handoff run in `worktree-cli` | Replacement link already present before `wt remove`, invoked from the checkout with `WT_SHELL_WRAPPER=1` | Produces approval token; second run deletes `saved/a`; exit 1 | Refuse before producing a token |
| Second handoff run in `worktree-cli` | Healthy checkout approved first; directory moved and replaced with a symlink before `--handoff` | Accepts old approval; deletes `saved/a`; exit 1 | Refuse with exit 3; invalidate approval |
| Listing availability projection in `worktree` / `worktree-cli` | Same readable replacement symlink | `○ target`, no link warning | Removal must independently enforce the link prohibition; a list warning would also make the unsafe state visible |
| Preparation: dangling target symlink | Symlink points to absent directory; Git marks it prunable | Exit 3; target link and saved files preserved | Clean |
| Preparation: target replaced by a regular file | Git marks it prunable | Exit 3; replacement file preserved | Clean |
| Missing-record execution | A link appears after initial absence check | Existing public-API test refuses with `Reappeared(Link)` | Clean |
| Ordinary directory control | Original real directory and unchanged checkout | Removes normally | Clean |

The repair variant exposes a second entry point into the same class: the `worktree` package's [postcondition_failures](../../lib/src/remove/repair.rs:158), which verifies repository and checkout identity, compares paths after following links but never reinspects the target's kind. Both the administrative back-reference and top-level checkout checks therefore accept this replacement.

The `worktree-cli` package's [run_handoff](../../cli/src/commands/remove/mod.rs:937), which finishes approved removal from outside the checkout, uses the same marker-only guard. Its path checks also resolve the stored path again against the current filesystem. A newly introduced symlink changes both sides of that comparison together; it is not detected as a changed checkout. The administrative Git directory and content fingerprint remain identical.

**Fix:** Inspect the target without following links independently of `prunable`, before preparation, after repair, in both handoff runs, and immediately before an existing-directory removal. Refuse symlinks and Windows reparse points with exit 3. Preserve the established handling of path aliases in ancestors; the prohibited object is the target directory itself. Add regression cases for every failing row above, with assertions on retained files, record, and branch, including a clean checkout without force flags. Existing link tests manufacture a prunable entry or a dangling link and miss the readable replacement.

### High: Missing-record removal discards index changes made after inspection

**Defect class:** Destructive record removal trusts a staged-work snapshot that is not checked again before deleting the administrative index.

In the `worktree` package, [inspect_missing](../../lib/src/remove/missing.rs:85), which reads the surviving index of a deleted checkout, correctly determines whether staged changes need consent. But [remove_missing_record](../../lib/src/remove/missing.rs:152), which later deletes that record, receives only the worktree entry. It checks the listing and path absence, then removes the record without checking whether the index still matches the inspected state. Git itself does not protect that index when the checkout directory is gone.

**Reproduction through the shipped CLI:** In a local repository, create `target` on `topic`. Save its clean index, stage a modification to `a`, save that index separately, then restore the clean index and delete the checkout directory. Place a one-shot Git shim first on `PATH`. For `diff-index`, it delegates to real Git, captures the clean result, replaces the administrative index with the saved staged index, and returns the captured result. Delegate all other Git calls unchanged. Run `wt remove topic` without force flags or an interactive terminal. It exits 0, says the index matches the last commit, removes the administrative index, and deletes `topic`. No discard consent was obtained for the new staged version.

This deterministic edit models an index writer between inspection and deletion; it needs neither timing sleeps nor network access. It can be addressed by a final validation and does not depend on closing every possible operating-system race.

| Site / sibling path | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Missing directory: record removal | Index gains staged content immediately after the inspection command | Exit 0; index and branch removed without consent | Detect changed index; exit 3 and require a fresh invocation |
| Healthy checkout: ordinary execution | Same staged-index replacement immediately after `git status` returns clean | Git refuses removal; exit 1; index, checkout, branch retained | Clean for preservation of newly staged work |
| Repaired checkout: ordinary execution | Same staged-index replacement after repair and status | Git refuses removal; exit 1; repaired link, index, files, branch retained | Clean for preservation of newly staged work |
| Existing-directory handoff | Stage a new index version between approval and `--handoff`; restore working bytes | Exit 3, reporting changed uncommitted/included files; retained | Clean |
| Missing directory: initial staged changes | Existing binary test has staged changes before inspection | Exit 3 without force; removal allowed with `--force-worktree` | Clean |
| Missing directory: initial absent/corrupt index | Existing public-API and binary tests | Refuses before record removal | Clean |
| Missing directory: target reappears | Existing public-API directory/link cases | Refuses before record removal | Clean |

**Fix:** Carry the inspected administrative identity and an index binding into execution. Re-read the index and compare it with the state that informed the report and consent; refusal is appropriate if it changed or cannot be inspected. Validate the resolved tip too, since it is the basis of the index comparison and branch-safety decision. Keep explicit force consent separate from approval of a previously observed state. Add deterministic regressions for changed, disappeared, and corrupted indexes after inspection, plus unchanged controls. The existing handoff index binding is a useful precedent; a missing directory currently bypasses that protection because it never hands off.

### High: New listing markers and recovery notes lack real-terminal verification

**Defect class:** New terminal-visible states are verified only through manufactured rendering output, leaving their actual glyphs, styling, width, and placement untested in a terminal.

In the `worktree-cli` package, [the unavailable-row tests](../../cli/tests/list_table.rs:1305) assert rendered strings and ANSI sequences for the `✕` and `?` markers, conditional legend, and recovery notes. These are Level 1 tests. The [existing real-terminal style scene](../../cli/tests/level2_list_verbose.rs:662) covers healthy/dirty rows and the original legend, but its fixture contains no missing, unlinked, or unknown-status checkout. Passing that scene cannot prove the new visible states.

| Site / sibling presentation | Shape checked | Strongest relevant verification present | Expected verification |
| --- | --- | --- | --- |
| Worktree marker | Missing/unlinked checkout renders `✕` | Level 1 snapshot / string checks | Level 2 pane capture verifies glyph and placement |
| Unknown-status marker | Non-prunable checkout whose status fails renders dim `?` | Level 1 string and ANSI checks | Level 2 capture verifies glyph and dim style |
| Conditional legend and table width | Cross-only, question-only, both, neither | Level 1 conditional-legend and width assertions | Level 2 captures with both markers and an ordinary control; verify borders and wrapping |
| Recovery notes | One dim note per unavailable row, in row order, after existing closing notes | Level 1 snapshots and ANSI checks | Level 2 text/style capture verifies order, dim style, and wrapping |
| Existing healthy/dirty rows and original legend | Ordinary design fixture | Level 2 cell/style assertions | Clean |
| Repair cancellation and wrapper handoff | Actual repair followed by decline/removal | Level 2 scenes in `level2_remove.rs` | Clean |
| Refusal when `.git` changes between handoff runs | Filesystem changes and binary exit/output assertions | Level 1 binary tests | Appropriate: this is filesystem/protocol behavior, not keyboard encoding |

**Fix:** Extend the existing focus-free tmux style fixture, or add a related scene, with a real missing checkout, a real unlinked checkout, and a deterministic status failure. Capture text and styles from the terminal. Assert the new glyphs, legend conditions, table boundaries, and dim ordered notes at normal and narrow widths. No Level 3 test is needed: this fix adds no keyboard-event behavior.

## Input robustness sweep

The changed readers consume Git's text/byte formats, not JSON/YAML/TOML configuration. Structured nulls, collection element types, and duplicate object keys do not exist in these formats. The plan identifies the load-bearing `prunable` marker and administrative `gitdir` back-reference; the new index reader also consumes Git's `--name-status -z` output. The existing handoff record gains a Git-directory binding but retains its JSON loader and version-rejection behavior.

For the back-reference, I created a separate real Git fixture per cell, changed only that file, and ran the built `wt remove topic --force-worktree --force-branch`. Both missing and unlinked consumers were swept. Controls removed normally; every invalid row retained the administrative record and branch, and unlinked working files stayed intact.

| Shape | `gitdir`: unlinked removal | `gitdir`: missing removal | `prunable` reader / projection |
| --- | --- | --- | --- |
| Unedited control | Exit 0, verified repair then removal | Exit 0, inspected record then removal | Real Git fixtures classify missing/unlinked; no checkout status calls |
| Absent | Exit 1; Git cannot read the broken record; retained | Same | `None`; marker absence does not prove filesystem safety (first finding) |
| Null / whole-field wrong type | Literal `null` and `123`: exit 3; retained | Same | No typed null; reason is display text, never a safety decision |
| Wrong-type element / all elements | Not applicable: one path, no collection | Same | Not applicable: marker plus optional text |
| Empty | Exit 1; retained | Same | Bare/trailing-space marker is present with empty reason; tests preserve it |
| Duplicate | Two path lines: exit 3; retained; association test also refuses two matching records | Same | Duplicate reasons are kept together; no last-wins coercion |
| Trailing / invalid | Trailing garbage and NUL: exit 3; retained; path replaced by directory: exit 1; retained | Same | Parser tests cover CRLF, final entry, boundaries, and embedded marker text |

The different exit codes above arise because some damage makes Git's initial worktree listing fail before the association reader runs. None was accepted as an empty or safe record. The `gitdir` matrix test also covers relative paths and platform-specific raw-byte handling. `parse_name_status_z` tests exercise empty output, valid records, missing paths, and multi-letter statuses; the staged-index binary tests assert consent through the public CLI. No additional permissive-default defect was found in those readers. The unsafe snapshot consumption after a successful parse is the second finding.

## Requirement coverage and validation

| Requirement group | Verification present | Review result |
| --- | --- | --- |
| Preserve porcelain markers and distinguish inspected filesystem states | Level 1 parser, filesystem, and injected-error tests | Marker parsing works; readable target links evade removal checks |
| Failed status stays unknown; skip prunable status; graph does not claim source dirtiness for unknown | Level 1 library, refresh, rendering, and graph tests | Matches contract |
| Ref comparisons retain meaning; fast-forward refuses prunable default-branch holders | Level 1 real-Git and injected-runner tests | Matches contract for the specified prunable holders |
| Missing directory: staged consent, detached HEAD, branch policy, failure cleanup, reappearance | Level 1 API and binary tests | Initial checks work; index changes before execution are unprotected |
| Repair: exact admin/common directory, back-reference, top level, refreshed identity, exit-status independence | Level 1 real-Git and injected-repair tests | Checks work for ordinary directories; replacement links bypass them |
| Repair refusal/cancellation keeps files and repaired metadata; protected ignored content | Level 1 policy/binary plus Level 2 cancellation and wrapper scenes | Matches contract in covered states |
| Handoff refuses a broken/redirected `.git` without another repair | Level 1 binary and binding tests | Covered for `.git` changes; target-directory replacement remains unsafe |
| Markers, legend, dim notes, row order, safe text and commands | Level 1 snapshots / ANSI / quoting tests | Missing Level 2 verification of new visible states |
| Contextual errors preserve categories and describe partial success | Level 1 context and binary tests; documentation review | Covered in ordinary error paths; link-loss repro demonstrates why preflight refusal is essential |

- Fresh macOS `just test`: **1,033 passed, 32 skipped**, 26.632 seconds. Also ran the narrower `just test remove`: 152 passed.
- Fresh `just check-tier-coverage worktree`: **no stranded tests**. The CLI uses automatic integration targets; terminal targets require `terminal-tests`, enabled by the declared CI features and L2 recipe.
- Fresh focus-free `just test-l2 remove` with `BISCUIT_L2_THREADS=1` and required tmux: **3 passed**, tmux proof recorded. This substring does not select the two repair scenes, so it is not evidence for them; the full L2 result is recorded below.
- Fresh full `just test-l2` with the same settings: **34 passed, 703 skipped**, 88.063 seconds. Both new repair scenes passed; tmux proof records 28 executions. The six Kitty graph scenes also passed. The skipped tests belong to other tiers. Self-spawn mode avoided broker pre-spawning unrelated terminal windows.
- Review reproductions used the built `target/debug/wt`, real Git, disposable repositories, disabled credential prompts, closed stdin, and no remotes. Deterministic Git shims delegated unchanged commands and altered one filesystem state at a named boundary.
- No lint run was needed for a review-only Markdown change; the implementation log records the prior clean lint result. No formatter, commit, or lifecycle move was run. Cross-OS evidence is left to CI and does not affect this readiness decision.

The README, current list/remove documentation, and skill describe repair before consent, the kept-link side effect, unknown status, and force limits. The known repository-wide repair side effects are explicitly disclosed. The prose is consistent with the intended safety contract; code must meet that contract in the failing states above.
