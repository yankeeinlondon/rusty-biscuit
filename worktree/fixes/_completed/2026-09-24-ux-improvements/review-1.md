---
$schema: feature-review.yaml
ready: false
human_review: true
human_review_items:
    - |-
        Review Decisions 20–33 in the specification. These are fourteen proposed choices about removal safety, timing, shell behavior, records, and graph sizing that the implementation and user documentation already use. Confirm all fourteen, or name each decision you want changed and the behavior you prefer.
    - |-
        Decide what `wt list` should show for a pull request when the terminal cannot make its badge clickable. The specification says to show the full URL, but the implementation shows only the PR number because the full URL can make the table fail to fit. Choose either (1) show the PR number only and update the specification, or (2) show the URL and change table rendering so long links fit.
reviewed_by: codex/gpt-6-sol
agent: codex/gpt-6-sol
created: 2026-09-25T01:50:52-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
next: 2026-09-24-ux-improvements/review-2.md
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-1.md
findings:
    - title: Real-terminal tests do not verify the specified colors and emphasis
      priority: high
    - title: Graph visibility and sizing lack real-terminal verification
      priority: high
    - title: PowerShell move-first behavior lacks its required Level 2 test
      priority: high
    - title: Pull-request URLs disappear on terminals without clickable links
      priority: medium
---

# Review 1: Worktree UX improvements

## Verdict

**Not ready for production.** The main logic has broad Level 1 coverage and the worktree area has live Level 2 test recipes, but three user-visible terminal requirements are verified at the wrong level. One documented behavior also differs from the current specification. Cross-OS result collection and the later human review are separate from this readiness judgment.

## Findings

### High: Real-terminal tests do not verify the specified colors and emphasis

The spec requires distinct dirty-file colors, dirty-state dots, badge backgrounds, red conflict connectors, and a highlighted current row (item 3's report and item 5's table). The Level 1 `styles_follow_the_design` test checks exact SGR sequences on a manufactured `Terminal`. The Level 2 tests in `cli/tests/level2_dirty_tree.rs` and `cli/tests/level2_list_verbose.rs` check only that the pane contains *some* `\x1b[` sequence; a shell prompt or another part of the output can satisfy that assertion even when the relevant glyph or row loses its style. The dirty-tree Level 2 test also does not assert the tree connector glyphs it describes.

This is a Level 1 → Level 2 verification mismatch for real-terminal styling. Capture the styled tmux pane and assert the expected SGR state around the specific file, dot, badge, connector, and highlighted row, with fixtures that exercise both source and non-source files and a conflict. Assert the visible glyphs and row layout in the same capture.

### High: Graph visibility and sizing lack real-terminal verification

The spec requires the graph to render at a consistent text scale, fit the available width, and limit the base view to about half the terminal's height while showing an elision notice (item 5, Graph sizing). Level 1 tests cover the generated Mermaid and computed sizes. The Kitty Level 2 test in `cli/tests/level2_list_verbose.rs` checks for a Kitty graphics protocol introducer and a table header. Those bytes prove that an image command was sent, but they do not prove that the emulator displayed the image, accepted its dimensions, preserved the table layout, or showed the elision notice. The tmux graph-path test deliberately falls back without displaying an image.

This is a Level 1 → Level 2 verification mismatch for the visible graph. Add real Kitty capture or screenshot assertions at representative narrow and short pane sizes, including the base-view lane cap. Keep the Level 1 structural graph tests for lane and tag semantics.

### High: PowerShell move-first behavior lacks its required Level 2 test

The spec explicitly requires a PowerShell window launched inside a worktree to move out and then remove that worktree, with the prompts and landing location verified at Level 2 on Windows (acceptance criteria 3 and 7). `cli/tests/powershell_wrapper_exec.rs` launches `powershell.exe` with captured pipes, so it verifies wrapper execution at Level 1. `cli/tests/level2_remove.rs` uses tmux for bash, zsh, and fish and contains no PowerShell case. The area has no Windows real-console Level 2 test for this path.

This is a Level 1 → Level 2 verification mismatch, independent of whether a Windows CI result has been collected. Provision the Windows terminal harness and run a PowerShell move-first test through it, checking the prompt, successful directory change, removal, and final location. Keep the existing Level 1 wrapper test for protocol details.

### Medium: Pull-request URLs disappear on terminals without clickable links

The spec says Prose's no-OSC8 fallback shows a visible `[text](url)` link (item 5, Dropped by design). `cli/src/commands/list_table.rs` passes `terminal.osc_link_support` to `RowCells` and `pr_badge`; `cli/tests/list_table.rs` explicitly asserts that the no-link rendering contains `PR #99` but no `https://`. The implementation log explains that a long URL can make the Table component fail to render at an ordinary width. The narrower fallback is understandable, but it changes the user-visible contract and leaves users without a PR destination.

Choose the fallback behavior with the author. If the spec's visible URL is retained, make long links wrap or place the URL outside the table and add a real-terminal width test. If badge-only output is accepted, update the spec and user documentation to state it.

## Requirement verification

| User-observable requirement | Strongest test present | Assessment |
| --- | --- | --- |
| Worktree and branch name completions, ambiguity, and `base` resolution | Level 1 temporary-repository and pure resolver tests in `lib/src/worktree.rs` | Appropriate for completion candidates and Git resolution. No terminal input encoder behavior is involved. |
| Remove safety tiers, ignored files, exit codes, remote lease, and branch removal | Level 1 temporary Git repositories, policy matrix, CLI subprocess tests, and stubbed PR answers | Appropriate for state and policy semantics. |
| Report before prompts and exactly one blank line before each question | Level 2 tmux captures in `cli/tests/level2_remove.rs` | Appropriate for terminal layout; both question types are exercised. |
| Dirty tree glyphs and exact colors; list dots, badge colors, conflict styling, row emphasis | Level 1 exact SGR assertions; Level 2 checks only generic escapes and partial text | **Wrong level for specified terminal styling; high finding.** |
| Move-first removal, prompt choices, landing location, and failed handoff for bash, zsh, fish | Level 2 tmux shell runs with injected input, plus Level 1 protocol tests | Appropriate for shell and prompt behavior. These tests do not assert OS-keyboard encoder behavior, and the spec defines no bare-modifier or key-chord requirement. |
| PowerShell prompt and move-first removal from a window launched inside the worktree | Level 1 captured-pipe PowerShell execution on Windows | **Wrong level; high finding.** |
| `wt go` and `wt create` wrapper protocol and `--from` semantics | Level 1 subprocess and temporary-repository tests; remove wrapper path has Level 2 coverage | Appropriate for protocol and Git semantics. |
| List table text, caption, PR placement, link fallback, and verbose text | Level 1 snapshots and specific assertions; Level 2 tmux pane text for table and verbose path | Text layout has Level 2 coverage. The URL fallback contradicts the spec; medium finding. |
| Visible graph, scaled width, height cap, and elision notice | Level 1 graph and size tests; Level 2 Kitty test checks only protocol bytes | **Wrong level for visible image and fit; high finding.** |

No Level 3 test is called for by this specification: it defines no requirement whose result depends on a terminal emulator encoding a physical key press. Existing Level 2 prompt tests inject bytes and therefore verify display and prompt flow, not physical-key translation.

## Validation

- `just check-tier-coverage worktree`: no stranded tests. The worktree CLI's three `level2_*` integration files are declared test targets, the `terminal-tests` feature is enabled by the L2 recipe and CI metadata, and the L2 recipe is live.
- `worktree/just test`: 286 passed, 17 excluded by tier filtering.
- Inspected the Level 2 tests and their assertions. I did not rerun L2 as part of this review; the findings concern assertions and a missing test, which a green run of the current suite would not resolve.
