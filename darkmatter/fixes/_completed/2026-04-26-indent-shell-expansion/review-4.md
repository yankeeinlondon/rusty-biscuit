---
agent: codex
model: ""
ready: true
---

# Review 4

## Findings

No blocking findings.

## Verification Level Assessment

The specification is a Markdown byte-rewrite feature: user-observable behavior is the composed Markdown shape and the downstream CommonMark structure after shell output is spliced. It does not assert terminal rendering, terminal input encoding, keyboard behavior, paste/IME behavior, mouse behavior, or SGR styling. Level 1 verification is therefore the appropriate level for every requirement in this spec.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| `::shell` output inside a 4-space list continuation is prefixed on every emitted line | Level 1: raw composed Markdown assertion at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:748` plus CommonMark HTML structure check at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:823` | Sufficient |
| Interior blank lines are prefixed, while trailing final newline does not create an indent-only line | Level 1: `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:765` and `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:782`; helper behavior at `darkmatter/lib/src/markdown/compose/indent.rs:27` | Sufficient |
| Column-1 `::shell` output remains unchanged | Level 1: raw output check at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:773` and CommonMark sibling baseline at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:833` | Sufficient |
| Block quote and nested block quote `::shell` output stays quoted | Level 1: raw checks at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:794` and `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:805`; CommonMark blockquote check at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:842` | Sufficient |
| Tabs are preserved byte-for-byte as the directive prefix | Level 1: `::shell` tab case at `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:757`; shared helper tab case at `darkmatter/lib/src/markdown/compose/indent.rs:87` | Sufficient |
| `::shell-block` shares the same indentation fix | Level 1: splice applies the opener prefix once to rendered block output at `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:179`; list, tab, root, trailing-newline, and CommonMark tests at `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:565`, `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:578`, `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:587`, `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:610`, and `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:643` | Sufficient |
| Block quoted `::shell-block` output stays quoted and nested | Level 1: raw and CommonMark checks at `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:665`, `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:678`, `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:689`, and `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:704` | Sufficient |
| Shell output is otherwise preserved | Level 1: shared helper preserves trailing whitespace and newline shape at `darkmatter/lib/src/markdown/compose/indent.rs:41`; shell-block rendering concatenates command outputs verbatim before indentation at `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:151` and `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:183`; integration expectation updated in `darkmatter/lib/tests/shell_block_integration.rs:132` | Sufficient |

## Implementation Notes

The implementation captures the directive prefix before parsing `::shell` (`darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs:56`) and applies the shared indentation helper immediately before span replacement (`darkmatter/lib/src/markdown/compose/mod.rs:1241`). `::shell-block` follows the same strategy at block granularity: individual commands execute with empty per-command indentation, then the combined rendered output is indented once at the splice boundary (`darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:107` and `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs:183`). That avoids compounding indentation across multiple commands and matches requirement 5's "do not modify command output otherwise" constraint.

The one spec-language wrinkle is that the implementation captures block-quote markers as part of the effective prefix, not only whitespace. That is necessary for the block quote acceptance criterion (`> > ::shell ...`) to remain structurally nested in CommonMark, and the tests now pin that behavior for both directive forms.

## Tests Run

- `cargo test --color=never -p darkmatter --test shell_block_integration compose_shell_block_with_multiple_commands`
- `cargo test --color=never -p darkmatter indented`
- `cargo test --color=never -p darkmatter blockquote_shell`
