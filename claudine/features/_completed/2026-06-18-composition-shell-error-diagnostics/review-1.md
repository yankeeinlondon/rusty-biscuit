---
ready: false
agent: codex
model: ""
created: 2026-06-18T18:50:49
---

# Review: Composition Shell-Error Diagnostics

## Findings

### High: `::shell-block` wrapper diagnostics still report body-relative lines

The implementation normalizes the inner `ShellCommandOrigin::ShellBlock` to file-relative lines, but the surrounding `ShellBlockError::Command` still stores `block_start_line: pair.start_line` and `command_line: command.start_line` without adding the frontmatter offset in both error paths.

- `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:130`
- `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:164`

That means the outer error can still say `Shell block command failed at line 4` / `Command at line: 4` for a command that is actually on file line 7, while the nested source says line 7. This violates Section 1's "one coordinate space everywhere" requirement for shell-block commands and keeps a misleading line number in the user-facing diagnostic.

The current regression test misses this because it accepts `msg.contains("line 7") || msg.contains("ShellExpansionError")`, so any nested line-7 text can pass while the wrapper remains wrong.

Verification level present: Level 1, but with insufficient assertion strength. The appropriate minimum is Level 1 with exact assertions for the outer `Display` and `StatusBlock` lines, plus the existing CRLF/body coverage.

### High: `ShellBlockError::status_block` drops the rich inner execution failure

`ShellBlockError::Command` builds a shell-block-specific status block and only uses the inner `ShellExpansionError` to pick a hint. It never renders the inner `ExecutionFailed` status block, never includes the captured stderr/stdout, and `block_source()` always returns `None`.

- `darkmatter/lib/src/markdown/compose/shell_blocks/types.rs:245`
- `darkmatter/lib/src/markdown/compose/shell_blocks/types.rs:279`
- `darkmatter/lib/src/markdown/compose/shell_blocks/types.rs:307`

This leaves direct Darkmatter rendering of `::shell-block` command failures without the required linked path, composed frontmatter, exact source excerpt, and captured stderr/stdout from Sections 2 and 3. Claudine's top-level walker may find the deepest source in some paths, but the Darkmatter `MarkdownError::ShellBlock(inner) => inner.status_block(term)` path still renders the lossy wrapper.

Verification level present: Level 1 coverage only checks that a shell-block failure errors and that some line-7 text appears. There is no assertion that the rendered shell-block diagnostic contains stderr, frontmatter, or the file-relative excerpt. This is a functional gap, not just a test gap.

### High: Claudine boundary coverage does not exercise an actual execution failure through the CLI/writer path

Section 4 requires a claudine-side Writer-seam or L2 capture test proving that a rendered shell execution failure contains the file-relative line, stderr text, and source excerpt, and that piped/JSON-like output carries no ANSI. The new claudine tests construct `CompositionError::ShellExpansionFailed` directly, which is useful Level 1 unit coverage, but they do not exercise the real boundary from `MarkdownError` through `map_compose_error`, `color_eyre`, the output walker, and the `claudine compose` command.

- `claudine/lib/src/composition/error.rs:1890`
- `claudine/lib/src/composition/error.rs:1905`
- `claudine/lib/src/composition/error.rs:1921`
- Existing CLI coverage in `claudine/cli/tests/contextual_errors.rs:21` is Unix-only and uses `Blacklisted`, not `ExecutionFailed`, so it does not verify captured stderr/stdout.

Verification level present: Level 1 unit rendering for a hand-built error, plus older Level 1 Unix-only CLI coverage for a different variant. Required minimum: Level 1 CLI/writer-seam coverage using a portable failing command/shim that produces known stderr and verifies stderr, excerpt, file-relative line, and no ANSI under `NO_COLOR`/plain output.

## Other Notes

The direct `::shell` `ExecutionFailed` renderer is substantially improved: it delegates through `ShellExpansionError::status_block`, trims and tail-truncates captured output, includes the source context, and has focused Level 1 tests for stderr/stdout/truncation/frontmatter rendering.

Targeted checks run:

- `cargo test --color=never -p claudine shell_expansion_failed` passed.
- `cargo test --color=never -p darkmatter --test shell_expansion_coordinates` passed.

Those passing tests do not close the findings above because the shell-block assertions are too weak and the claudine command path is not exercised for `ExecutionFailed`.
