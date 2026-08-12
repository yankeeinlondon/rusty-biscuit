---
ready: false
agent: codex
model: ""
---

## Findings

1. **High: interactive approval prompt coverage is misclassified as Level 2, so the user-observable TTY requirement is not verified at the required level.** The spec marks "Interactive approval prompts for unapproved shell commands appear exactly as in normal mode" complete using `level2_pty_dry_run_shell_approval_prompt_appears_and_allows` as "L2 PTY" coverage ([spec.md:108](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:108)). That test is a direct `expectrl::Session::spawn` pseudo-terminal test and submits manufactured bytes with `session.write_all(b"3\n")` ([level2_dry_run_pty.rs:147](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:147), [level2_dry_run_pty.rs:155](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:155)). Under the review rubric, PTY tests with injected bytes are Level 1, not Level 2. This is good L1 coverage that the dry-run TTY branch can show a prompt and continue, but it does not verify rendering through a real terminal emulator or multiplexer capture. It also does not compare the dry-run prompt transcript to a normal-mode prompt transcript, even though the requirement says "exactly as in normal mode" ([level2_dry_run_pty.rs:149](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_dry_run_pty.rs:149)). Add a real Level 2 test using the repo's terminal harness/tmux/WezTerm capture and assert the same approval prompt surface appears in both normal and dry-run mode; keep the existing PTY test as L1 or rename/reclassify it.

2. **High: the styled metadata rendering contract lacks Level 2 verification.** The spec requires the metadata table's Document cell to render blue and as an OSC8 link, Description to render italic/dim, Agent and Model placeholders to render styled, and YOLO to render green/red ([spec.md:77](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:77), [spec.md:82](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:82), [spec.md:84](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:84), [spec.md:87](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:87), [spec.md:92](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-03-dry-run/spec.md:92)). The implementation builds these cells with `Prose` markup ([dry_run.rs:147](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/dry_run.rs:147), [dry_run.rs:153](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/dry_run.rs:153), [dry_run.rs:160](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/dry_run.rs:160), [dry_run.rs:167](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/dry_run.rs:167), [dry_run.rs:172](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/dry_run.rs:172)), but the tests strip escape codes before asserting semantics ([dry_run.rs:215](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/composition/dry_run.rs:215)) and the CLI test only checks row labels after stripping ANSI. That misses the user-observable styling/link requirement and cannot catch broken OSC8/SGR rendering in a real terminal. Add Level 2 terminal-capture coverage for `compose --dry-run` that verifies the metadata table renders through a real backend with the expected visible cells and stable styled/link output, or narrow the spec if the exact styling is not a production contract.

## Verification Level Matrix

| Requirement | Strongest observed coverage | Status |
| --- | --- | --- |
| `compose --dry-run` composes and does not launch provider | L1 CLI integration with provider stubs/sentinels | OK |
| Shell commands execute for real under `--dry-run --yolo` | L1 CLI integration | OK |
| Non-TTY unapproved shell command fails with the dry-run gate message | L1 CLI integration | OK |
| Interactive approval prompt appears exactly as normal mode | L1 PTY byte-injection, mislabeled L2; no normal-mode comparison | Gap |
| Body on stdout; frontmatter and metadata on stderr | L1 CLI integration | OK |
| `--quiet`/`--silent` do not suppress dry-run output | L1 CLI integration | OK |
| Styled metadata table including blue OSC8 Document link | Unit/L1 semantic checks with escape codes stripped | Gap |
| Sequence dry-run concatenation, dividers, fail-fast | L1 CLI integration | OK |

## Notes

The core implementation shape looks aligned with the dry-run behavior: composition and shell/harness preflight run, then `execute_composition_request_inner` returns before provider launch at the post-preflight seam. I did not find a functional blocker in the compose/inline/sequence L1 paths I inspected.

> **Superseded:** the current seam is immediately after provider/model
> resolution and before selected-executable validation or launch wiring. This
> paragraph records the implementation reviewed at the time.

## Tests Run

- `cargo test -p claudine-cli compose_dry_run --no-default-features --color=never`
- `cargo test -p claudine-cli sequence_dry_run --no-default-features --color=never`
