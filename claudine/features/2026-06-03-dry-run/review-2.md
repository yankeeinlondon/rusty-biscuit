---
ready: false
agent: codex
model: ""
---

## Findings

1. **High: interactive approval prompt parity is still classified as Level 2, but the tests are Level 1 under the requested rubric.** The spec marks "Interactive approval prompts for unapproved shell commands appear exactly as in normal mode" complete using `level2_pty_dry_run_shell_approval_prompt_appears_and_allows` and `level2_pty_dry_run_approval_prompt_matches_normal_mode` as "L2 PTY" coverage ([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:108)). Those tests spawn `claudine` through `expectrl::Session::spawn` against a pseudo-terminal and feed manufactured input bytes with `session.write_all(b"3\n")` / `session.write_all(b"4\n")` ([level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:204), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:215), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:230), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:249), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:291)). Under this review's Test Rigor definition, pseudo-TTY plus injected bytes is Level 1, even if the file name and `require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)")` say otherwise ([level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:24), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:231), [level2_dry_run_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:292)). This leaves the user-observable prompt-rendering parity requirement without real terminal/multiplexer capture. Add a real Level 2 tmux/WezTerm/Kitty capture test that runs normal mode and `--dry-run`, captures pane text through the terminal CLI, and compares the prompt surface; keep the current `expectrl` tests as L1 or rename/reclassify them so `test-l2` does not overstate the coverage.

## Verification Level Matrix

| Requirement | Strongest observed coverage | Status |
| --- | --- | --- |
| `compose --dry-run` composes through the pipeline and does not launch the provider | L1 CLI integration with provider stubs/sentinels | OK |
| Shell commands execute for real under `--dry-run --yolo` | L1 CLI integration | OK |
| Non-TTY unapproved shell command exits non-zero with the dry-run gate message | L1 CLI integration | OK |
| Interactive approval prompt appears exactly as in normal mode | L1 pseudo-TTY byte-injection comparison, mislabeled as L2 | Gap |
| Body on stdout; frontmatter and metadata on stderr | L1 CLI integration | OK |
| `--quiet` / `--silent` do not suppress dry-run artifacts | L1 CLI integration | OK |
| Styled metadata table including blue Document cell, italic/dim Description, red YOLO, and OSC8 link | L2 tmux/WezTerm capture (`frame.raw`) | OK |
| `inline-compose --dry-run` prints the composed prompt and leaves the file unchanged | L1 CLI integration | OK |
| Sequence dry-run concatenation, dividers, quiet/silent behavior, and fail-fast | L1 CLI integration | OK |

## Notes

The functional dry-run seam looks aligned with the spec: shell approval and harness preflight run before the dry-run return, and provider launch is skipped at the post-preflight seam. The prior metadata-rendering gap appears addressed by real tmux/WezTerm capture tests, and those tests passed locally.

## Tests Run

- `cargo test -p claudine-cli compose_dry_run --no-default-features --color=never`
- `cargo test -p claudine-cli sequence_dry_run --no-default-features --color=never`
- `cargo test -p claudine-cli dry_run --no-default-features --color=never`
