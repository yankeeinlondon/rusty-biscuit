---
$schema: feature-review.yaml
ready: false
human_review: true
human_review_items:
    - |-
        Confirm or revise Decisions 20–33 in the specification. These fourteen proposed choices cover removal safety, timing, shell behavior, stored records, and graph sizing. Choose either to accept all fourteen or identify the numbered decisions and replacement behavior you want.
    - |-
        Decide how pull requests should appear in terminals that cannot make text clickable. Choose (1) show only the PR number and update the specification, or (2) preserve the visible URL required by the specification and make it fit, for example on a separate line below the table.
reviewed_by: codex/default
created: 2026-09-25T09:48:03-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-2.md
previous: 2026-09-24-ux-improvements/review-1.md
next: 2026-09-24-ux-improvements/review-3.md
findings:
    - title: Remote deletion handoff can delete an unapproved branch
      priority: high
    - title: Real-terminal tests depend on clearing stale pane output
      priority: high
    - title: Held-directory refusal lacks the specified Level 2 regression
      priority: high
    - title: Pull-request URLs disappear on terminals without clickable links
      priority: medium
---

# Review 2: Worktree UX improvements

## Verdict

**Not ready for production.** A reproducible handoff defect deletes a different remote branch from the one approved. Real-terminal tests fail on stale pane output, the specified Windows held-directory L2 regression is missing, and the previously reported PR URL contract mismatch remains. Human approval and collection of cross-OS results are separate from this readiness judgment.

## Previous findings

Review 1 has a single `Findings` section rather than separate `Unblocked Findings` and `Blocked Findings` sections. Its human-review items and the implementation log identify the URL decision as blocked. No answer unblocking it is recorded; Decisions 20–33 also remain explicitly proposed.

| Previous finding | Review 2 assessment |
| --- | --- |
| Real-terminal colors and emphasis | Partially addressed. `styled_capture` now checks specific cells, glyphs, colors, and row emphasis. The dirty-tree L2 test passed, and the table test reached the end of its dark-background assertions, but its second capture stalls; see the high finding below. |
| Graph visibility and sizing | Addressed. Both new Kitty L2 tests passed here. They check actual screenshot pixels against the transmitted image's bounding box and placement, width, reserved rows, table borders, and short-window lane elision. The exact-integer row calculation also has an L1 regression test. |
| PowerShell move-first L2 | Addressed in the implementation. Two declared, feature-enabled Windows tests launch PowerShell in ConPTY inside the target, answer the prompt, inspect the console screen buffer, verify both working directories, and verify deletion. The implementation log records Windows execution and a negative control removing the process-directory update. I inspected these tests but did not rerun Windows. |
| PR URLs on terminals without clickable links | Still blocked on the same author decision; no implementation or contract change resolves it. |

## Unblocked Findings

### High: Remote deletion handoff can delete an unapproved branch

**Location:** `worktree/cli/src/commands/remove/mod.rs:555–565`, with destination selection in `worktree/lib/src/remove/remote.rs:15–27`.

`RemoteApproval` stores both `destination` and `observed_sha`, but `run_handoff` checks only the SHA. The fresh gather recomputes the destination from branch configuration, and `execute` deletes that newly selected destination. Two different branches can point to the same commit, so the lease does not catch a change of branch identity.

Reproduced against the built `wt` using only a temporary repository and local bare origin:

1. Put `main`, remote `approved`, remote `unapproved`, and local `feat/x` at the same commit. Configure `feat/x` to track `origin/approved` and check it out in `feat-x`.
2. From inside that worktree, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-remote` and retain the returned handoff token. The first invocation exits 0 and approves the `approved` destination.
3. Before the second invocation, run `git branch --set-upstream-to=origin/unapproved feat/x` from the base checkout.
4. From the emitted landing directory, run `wt remove --handoff <token>`.

Actual result: exit 0, `Deleted origin/unapproved`, with `approved` still present on origin. The worktree and local branch are also removed. The spec requires the second invocation to honor the recorded approval and refuse changed state before mutation.

Compare the fresh destination with the stored destination before any removal, and reject a mismatch even when the heads are identical. Add an L1 CLI regression using this same-SHA/different-destination fixture, asserting exit 3, both remote branches preserved, and the local worktree and branch intact. This is Git state/authorization logic, so L1 is the appropriate verification level.

### High: Real-terminal tests depend on clearing stale pane output

**Location:** `worktree/cli/tests/level2_list_verbose.rs:378–382`, called again for the light-background assertions at the end of `level2_list_styles_follow_the_design_in_tmux`.

The test sends `cd … && clear` and then waits for all occurrences of `parent deleted` to disappear. On this host, the captured pane retains the first table and legend after that command has returned to the shell prompt. The wait times out after 15 seconds, before the second `wt list` invocation and its light-background assertion.

This failed twice: once in the full L2 suite and once through `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_list_styles_follow_the_design_in_tmux`. Both failed at line 410 after approximately 16.27 seconds with the old legend and the subsequent shell prompt visible. The stronger dark-background assertions are useful, but the new test is not a passing verification of the whole requirement.

Make the second run independent of the old frame, for example with a fresh detached pane for each background, or use an explicit terminal reset and completion marker whose behavior is controlled by the fixture. Do not solve this by increasing the timeout or dropping the light-background assertion. Verify both variants through the normal L2 recipe. This is a defective L2 test, not evidence that the renderer's light style itself is wrong; the requirement still needs reliable Level 2 verification.

The subsequent filtered L2 run also failed `level2_move_first_through_each_wrapper_lands_in_the_fork_parent` at `level2_remove.rs:313`. Its capture contained earlier bash and zsh prompts alongside the current fish prompt; `assert_one_blank_line_before` selected the first matching prompt, now at the top of the pane, and failed with “too close to the top.” Apply the same per-run capture isolation to this existing test. This second failure concerns a test introduced for this specification, although it was not newly added by the review-1 implementation.

### High: Held-directory refusal lacks the specified Level 2 regression

**Location:** `worktree/cli/tests/level2_powershell_remove.rs`, and specification acceptance criterion 3's Windows bullet.

The new PowerShell tests both exercise successful move-first removal. The held-directory refusal is covered by a Windows L1 subprocess test, while the specification explicitly requires Level 2 verification of exit 4 with nothing removed. The implementation log's negative control—temporarily removing the wrapper's process-directory update and observing the new success tests fail—is useful evidence that the fixture holds a directory, but it is not a retained regression for the refusal requirement.

Add a ConPTY L2 case with a separate process retaining the target directory. Run removal from outside it, assert the console shows the in-use refusal and exit 4, and assert tracked, dirty, and ignored files, registration, and branch survive. Release the lock in cleanup without opening or focusing a window. This is a missing test at the specified level, independent of whether Windows CI is provisioned or its results have been collected.

## Blocked Findings

### Medium: Pull-request URLs disappear on terminals without clickable links

**Location:** `worktree/cli/src/commands/list_table.rs:374–382`; specification item 5, `Dropped by design`.

This is unchanged from review 1. `pr_badge` drops the URL when `osc_link_support` is false, while the specification still requires a visible `[text](url)` fallback. The L1 no-link test deliberately expects the URL to be absent. The implementation log explicitly defers the choice, and the current human-review metadata does not record approval of that departure.

DECISION: the visible `[text](url)` is NOT desired as it would be awkward in this kind of layout. 

SIDE NOTE: we need to make sure that all terminal links go through a common struct that can both enforce standards but also provide useful options like declaring what the lack of support's fallback should be. This does not need to be addressed to close this spec.

After the author chooses, either update the specification to accept badge-only output or preserve the URL in a layout that fits and verify it with L1 assertions and a real-terminal width/capture test. The unresolved contract mismatch contributes to readiness; needing a human decision by itself does not.

## Requirement verification

| Requirement | Strongest relevant verification present | Assessment |
| --- | --- | --- |
| Branch/directory completion candidates, `base`, ambiguity, detached names | L1 resolver and temporary-repository tests | Appropriate for candidate generation and name resolution. |
| Removal safety tiers, force flags, ignored entries, PR identity/head, live remote checks, lease failures, exit codes | L1 policy matrix, stub providers, temporary repositories, CLI tests | Appropriate for state semantics. The remote destination handoff regression above is missing. |
| Report ordering, exactly one blank line before questions, default prompt choices | L2 tmux prompt captures plus L1 policy tests | Appropriate for display and flow; no physical-key encoder contract. |
| Dirty-file tree colors and connector glyphs | L2 styled cell and complete tree-row assertions | Appropriate; passed here. |
| Table dots, local/remote/PR badge backgrounds, conflict connectors, current-row emphasis | L1 style/snapshot tests and new L2 styled cell assertions | Correct intended level, but the L2 test fails before the light-background case. High finding. |
| Bash/zsh/fish move-first, fork-parent/base landing, preserved subdirectory, failed handoff | L2 tmux wrapper tests plus L1 protocol/state tests | Appropriate level; the multi-shell fork-parent test fails on stale prompt selection, included in the test-isolation finding. Base landing and failed-handoff tests passed. |
| PowerShell launched inside target, prompt, both working directories, successful removal | Windows L2 ConPTY screen-buffer tests plus L1 wrapper tests | Appropriate implementation of the required test. Windows execution is recorded in the implementation log, not rerun for this review. |
| Windows held-directory refusal | L1 Windows subprocess test; logged negative control of the ConPTY L2 test | **Wrong retained test level:** acceptance criterion 3 explicitly requires L2. High finding above. |
| `wt go`/`wt create` wrapper detection and `--from`, fork-origin persistence | L1 CLI, wrapper, and repository tests | Appropriate for protocol and Git semantics. |
| Table captions, comparison vocabulary, branch tree, PR placement, stale-cache age, verbose output | L1 snapshots/pure tests and L2 table/verbose captures | Appropriate division between data semantics and real-terminal layout. URL fallback remains a contract mismatch. |
| Graph lanes/tags, origin divergence, parent and PR tags, trimming and scale calculation | L1 graph facts, generated Mermaid, and sizing tests | Appropriate for graph structure and sizing arithmetic. |
| Actual graph display, narrow width, base height cap, elision notice, table surviving the image | L2 private Kitty screenshots, screen text, and recorded transmission | Appropriate; both new tests passed here. |
| Warm/cold gathering, full command time, network failure/deadline, fresh PR-cache request suppression | L1 timed CLI and request-count performance tests | Appropriate; all 17 performance tests passed here. |

No requirement here depends on a terminal encoding a physical modifier or hotkey press, so Level 3 is not required.

## Validation and scope

- `just check-tier-coverage worktree`: zero stranded tests.
- `worktree/just test`: 288 passed; 17 excluded by the area's filter.
- `worktree/just test-perf`: 17 passed.
- `BISCUIT_TEST_REQUIRED_BACKENDS=kitty,tmux just test-l2`: dirty-tree and both Kitty tests passed; table styling failed; fail-fast left seven tests unrun.
- Isolated table-styling L2 run: same failure, approximately 16.27 seconds.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_remove level2_move level2_list_verbose`: both verbose tests, failed-handoff test, and base-landing test passed; multi-shell fork-parent test failed on stale prompt selection; two prompt tests remained unrun after fail-fast. An earlier attempt to filter with `--skip` was rejected by nextest and executed no tests.
- The handoff reproduction used disposable repositories and a local bare remote; no external remote was modified.

The new L2 files are explicit Cargo test targets with `required-features = ["terminal-tests"]`; the live `test-l2` recipe and CI test metadata enable that feature. `styled_capture_parse.rs` is an automatically discovered L1 target and ran in the 288-test suite. The PowerShell file is Windows-gated, so it contributes no tests to this macOS run. Its CI execution remains a documented provisioning gap because the package advertises tmux/Kitty backends; that cross-OS scheduling gap alone is not a readiness finding here.

The requested `prompts/_reviews/worktree/fixes/…/review-1.md` path does not exist. The existing review beside the spec is the previous review updated with `implemented: true` and `next`. That lifecycle flag records completion of the prior implementation attempt, not a claim that every finding passed this review.
