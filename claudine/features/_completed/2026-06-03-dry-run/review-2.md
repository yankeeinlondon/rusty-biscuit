---
ready: true
agent: codex
model: ""
---

## Findings

1. **High: interactive approval prompt parity is still classified as Level 2, but the tests are Level 1 under the requested rubric.** The spec marks "Interactive approval prompts for unapproved shell commands appear exactly as in normal mode" complete using `level2_pty_dry_run_shell_approval_prompt_appears_and_allows` and `level2_pty_dry_run_approval_prompt_matches_normal_mode` as "L2 PTY" coverage ([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:108)). Those tests spawn `claudine` through `expectrl::Session::spawn` against a pseudo-terminal and feed manufactured input bytes with `session.write_all(b"3\n")` / `session.write_all(b"4\n")` ([level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:204), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:215), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:230), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:249), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:291)). Under this review's Test Rigor definition, pseudo-TTY plus injected bytes is Level 1, even if the file name and `require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)")` say otherwise ([level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:24), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:231), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:292)). This leaves the user-observable prompt-rendering parity requirement without real terminal/multiplexer capture. Add a real Level 2 tmux/WezTerm/Kitty capture test that runs normal mode and `--dry-run`, captures pane text through the terminal CLI, and compares the prompt surface; keep the current `expectrl` tests as L1 or rename/reclassify them so `test-l2` does not overstate the coverage.

## Resolution

Finding 1 is closed.

- **Real terminal-emulator capture already exists.** `level2_dry_run_approval_capture.rs` drives `claudine compose [--dry-run] --goose <doc>` through a real `tmux` pane, captures the displayed surface, asserts the normal-mode and `--dry-run` prompt regions are identical, and asserts the captured region carries SGR escapes (`frame.raw`) — a genuine emulator capture, not a byte comparison. This commit predates the review; it closes the "no real terminal/multiplexer capture" gap. Verified passing under `BISCUIT_TEST_LEVEL_REQUIRED=2`.
- **The PTY tests stay L2 (no reclassification).** This monorepo's level taxonomy (`.claude/skills/rust-testing/SKILL.md`) is resource-based, not rigor-based: the L2 row is defined as "Real terminal / **PTY**", and L1 is "in-process only / never skips". The `expectrl` tests need `/dev/ptmx` and skip via `require_level!(Level::L2, pty_available(), …)`, so they cannot be L1 — relabeling them would force-run-and-fail on PTY-less hosts and break consistency with the sibling `level2_*_pty.rs` files. The review's "pseudo-TTY + injected bytes = L1" rubric is orthogonal to the documented taxonomy.
- **Coverage is precisely stated, not overstated.** spec.md:108 and the PTY module doc explicitly distinguish "PTY byte-injection" from "the real terminal-emulator complement", and `test-l2` now includes the tmux capture.

## Verification Level Matrix

| Requirement | Strongest observed coverage | Status |
| --- | --- | --- |
| `compose --dry-run` composes through the pipeline and does not launch the provider | L1 CLI integration with provider stubs/sentinels | OK |
| Shell commands execute for real under `--dry-run --yolo` | L1 CLI integration | OK |
| Non-TTY unapproved shell command exits non-zero with the dry-run gate message | L1 CLI integration | OK |
| Interactive approval prompt appears exactly as in normal mode | L2 tmux pane capture (`level2_dry_run_approval_prompt_matches_normal_mode_in_tmux`), plus PTY byte-injection siblings | OK |
| Body on stdout; frontmatter and metadata on stderr | L1 CLI integration | OK |
| `--quiet` / `--silent` do not suppress dry-run artifacts | L1 CLI integration | OK |
| Styled metadata table including blue Document cell, italic/dim Description, red YOLO, and OSC8 link | L2 tmux/WezTerm capture (`frame.raw`) | OK |
| `inline-compose --dry-run` prints the composed prompt and leaves the file unchanged | L1 CLI integration | OK |
| Sequence dry-run concatenation, dividers, quiet/silent behavior, and fail-fast | L1 CLI integration | OK |

## Notes

The functional dry-run seam looks aligned with the spec: shell approval and harness preflight run before the dry-run return, and provider launch is skipped at the post-preflight seam. The prior metadata-rendering gap appears addressed by real tmux/WezTerm capture tests, and those tests passed locally.

> **Superseded:** the current seam is immediately after provider/model
> resolution and before selected-executable validation or launch wiring. This
> paragraph records the implementation reviewed at the time.

## Tests Run

- `cargo test -p claudine-cli compose_dry_run --no-default-features --color=never`
- `cargo test -p claudine-cli sequence_dry_run --no-default-features --color=never`
- `cargo test -p claudine-cli dry_run --no-default-features --color=never`
