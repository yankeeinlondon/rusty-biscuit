---
ready: false
agent: codex
model: ""
---

# Review: `::shell` / `::shell-block` Indentation Preservation

## Findings

### High: Block-quoted directives from the acceptance criteria are explicitly not implemented

The spec requires a fixture where `::shell` appears inside a nested block quote (`> > ::shell ...`) and expects every emitted line to be prefixed with the block-quote markers plus the directive indentation (`spec.md:59-63`). The implementation only captures leading whitespace before the first non-whitespace byte: `shell_expansion/parser.rs:50-54` uses `line.trim()` for detection and stores `line[..line.len() - line.trim_start().len()]` as the indent. A line beginning with `> > ::shell` is therefore not detected as a directive at all.

The shell-block path has the same gap. `shell_blocks/parser.rs:14-21` captures only leading whitespace, and the new test at `shell_blocks/mod.rs:637-646` asserts that a `>`-led `::shell-block` is not a directive. The plan also documents block-quote markers as "unreachable" rather than implemented. That is a direct mismatch with the acceptance criteria, not just a missing test.

Verification level: none for the required block-quote behavior. This is user-observable Markdown composition behavior, so Level 1 in-process compose plus CommonMark structural tests would be sufficient, but those tests need to cover the `> > ::shell ...` and `> > ::shell-block ...` cases the spec names.

Recommendation: either update the parser to recognize directive lines after block-quote markers and capture an effective prefix such as `> > ` plus any whitespace after the final marker, or revise the spec before merging. As written, the feature is not production-ready.

### Medium: Final blank lines from trailing-newline output are not indented as specified

The spec says every captured output line, including blanks, must receive the prefix, and the notes call out trailing-newline output specifically: a trailing blank line should become an indentation-only line rather than `""` (`spec.md:49-53`, `spec.md:78`). The shared helper intentionally does the opposite. `indent.rs:14-17` documents that a trailing newline does not gain a prefix, and `indent.rs:44-47` only inserts the indent when another character follows the newline. The test at `indent.rs:76-79` locks this behavior in.

The new shell tests cover interior blank lines (`shell_expansion/mod.rs:761-768`, `shell_blocks/mod.rs:564-574`), but not the trailing blank-line case produced by common commands that end stdout with `\n`. This leaves a spec requirement unverified and likely unmet at both splice sites (`mod.rs:1241-1247`, `shell_blocks/mod.rs:183-188`).

Verification level: Level 1 exists for interior blank lines, but no Level 1 test covers trailing blank lines. Level 1 is the appropriate level here because the behavior is pure Markdown composition, not terminal encoder/rendering behavior.

Recommendation: add explicit `printf 'one\n'` style fixtures for `::shell` and `::shell-block` under a list/container and assert the resulting replacement contains an indented whitespace-only final blank line if the spec remains unchanged. If the intended behavior is to preserve trailing newlines without materializing a whitespace-only line, update the spec and comments to remove the conflicting requirement.

## Test Rigor

For the implemented whitespace-indented list/root cases, the strongest tests are Level 1 in-process compose tests plus CommonMark HTML structural assertions. That level is appropriate for this feature because it does not depend on terminal glyph widths, terminal encoders, hotkeys, PTY input, or OS keyboard injection.

The missing block-quote acceptance criteria have no valid verification level because the behavior is not implemented. The trailing-final-blank requirement also lacks a targeted Level 1 fixture.

## Readiness

Not ready for production. The implementation covers the simple leading-whitespace list/root cases, but it does not satisfy the spec's block-quote-marker requirement and appears to contradict the spec's trailing blank-line note.
