---
ready: true
agent: codex
model: ""
created: 2026-06-18T20:51:15
---

# Review: Composition Shell-Error Diagnostics

## Findings

No production-blocking findings.

The gaps from review 1 appear to be closed:

- `::shell-block` wrapper diagnostics now add the frontmatter line offset to `block_start_line`, `command_line`, and the inner `ShellCommandOrigin::ShellBlock`.
- `ShellBlockError::Command` now renders the rich execution-failure diagnostic for `ExecutionFailed`: linked source path, wrapper lines, source excerpt, frontmatter block, captured stderr/stdout, and partial output from earlier commands.
- Claudine now has both direct `CompositionError::ShellExpansionFailed` rendering tests and a full Markdown-to-`CompositionError` boundary test for a real failing shell command. The CLI test suite also includes a `claudine compose` execution-failure case that checks stderr, file-relative line, source excerpt, frontmatter, and ANSI-free output under `NO_COLOR=1`.

## Verification Level Matrix

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Body `::shell` failures use file-relative coordinates, including CRLF source | Level 1: `darkmatter/lib/tests/shell_expansion_coordinates.rs` composes real Markdown fixtures and asserts line 7 excerpts | Appropriate. This is deterministic rendering/state behavior; no real terminal input encoder is involved. |
| `::shell-block` wrapper and inner error use the same file-relative coordinate space | Level 1: exact assertions for wrapper `Display`, wrapper status block lines, inner origin, and CRLF fixture | Appropriate. The previous weak assertion was replaced by wrapper-specific checks. |
| Frontmatter `$(...)` failures preserve frontmatter-origin coordinates | Level 1: composed fixture asserts `frontmatter.cmd` and excerpt at file line 3 | Appropriate. |
| Captured stderr is surfaced, stdout is conditionally surfaced, and output truncation is tail-biased | Level 1: `ShellExpansionError` unit tests cover stderr, stdout rules, trimming, UTF-8-safe truncation, and no ANSI in markdown/plain render | Appropriate. |
| SourceContext rendering includes linked path, excerpt, and composed frontmatter | Level 1: direct shell-error tests, shell-block tests, and composed Markdown tests assert these sections | Appropriate. |
| Claudine boundary preserves rich shell execution diagnostics instead of flattening them | Level 1: library boundary test exercises real Markdown -> `prepare_direct` -> `report_block_error`; CLI test source exercises `claudine compose` with `NO_COLOR=1` | Appropriate minimum for this non-interactive diagnostic path. L2/L3 are not required because the feature does not depend on terminal emulator input encoding, scrolling, or OS keyboard events. |

## Notes

One implementation smell remains but is not a blocker: `ShellBlockError::Command` duplicates the `ExecutionFailed` section assembly and tail-truncation helper from `ShellExpansionError`. It is currently covered by focused tests, but extracting a shared formatter would reduce drift risk if this diagnostic evolves again.

Targeted checks run:

- `cargo test --color=never -p darkmatter --test shell_expansion_coordinates` passed.
- `cargo test --color=never -p claudine shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic` passed.
- `cargo test --color=never -p claudine contextual_errors::compose_shell_execution_failure_renders_rich_block` completed but selected 0 tests because `contextual_errors` belongs to the CLI crate.
- `cargo test --color=never -p claudine-cli --test contextual_errors compose_shell_execution_failure_renders_rich_block` was stopped with exit 130 after exceeding the non-interactive wait budget during compilation/build-lock contention.
