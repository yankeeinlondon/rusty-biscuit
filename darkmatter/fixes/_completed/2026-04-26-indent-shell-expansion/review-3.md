---
ready: false
agent: codex
model: ""
---

# Review: `::shell` / `::shell-block` Indentation Preservation, Iteration 3

## Findings

### High: focused `shell_expansion` verification fails deterministically

`cargo test -p darkmatter --lib shell_expansion --color=never` fails in
`markdown::compose::shell_expansion::integration_tests::allow_once_persists_across_sibling_transclusions`
(`darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:1062`). Rerunning the single test also
fails: `composed.content().matches("hello").count()` is `1`, but the test expects `2`
(`darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:1091`).

This test is not one of the new indentation assertions, but it is in the body shell-expansion module
touched by this change and it exercises the same compose path after transclusion. A feature that
changes shell directive parsing/splicing should not ship with a deterministic failure in the focused
shell-expansion suite unless the expected behavior changed and the test is intentionally updated.

Verification level: Level 1 is appropriate here because the requirement is in-process Markdown
composition and transclusion, not terminal rendering or keyboard input. Current Level 1 verification
does not pass.

Recommendation: determine whether duplicate sibling transclusions should still both materialize shell
output. If yes, fix the compose/cache interaction that now drops one occurrence. If no, update the
test and document the behavior change because it affects user-visible composed Markdown.

### Medium: `::shell-block` still modifies command output beyond indentation

The spec says the fix must only add leading whitespace and must otherwise preserve command output,
including trailing whitespace, line endings, embedded code fences, and Unicode content
(`spec.md`, requirement 5). That is true for `::shell`, which re-indents `execution.combined_output()`
at the splice boundary. It is not true for `::shell-block`: `render_block_output` calls
`result.output.trim()` and then emits normalized `\n` separators (`darkmatter/lib/src/markdown/compose/shell_blocks/render.rs:14-37`).
The existing test suite even locks this in with `trimmed_output`
(`darkmatter/lib/src/markdown/compose/shell_blocks/render.rs:80-82`).

That means an indented shell block whose command emits leading spaces, trailing spaces, or a specific
final newline shape will not preserve captured stdout/stderr byte-for-byte apart from the added
container prefix. This is a designed requirement that remains either unimplemented for shell blocks
or inconsistent with the pre-existing shell-block render contract.

Verification level: Level 1 is appropriate. The current Level 1 tests cover shell-block indentation,
blank separator indentation, tabs, root-level output, and CommonMark structure, but they do not verify
preservation of trailing whitespace or line endings for shell-block output. The implementation
currently contradicts that requirement.

Recommendation: either change `::shell-block` rendering so indentation is applied to the captured
combined output without trimming/normalizing it, or explicitly narrow the spec to preserve the
existing shell-block rendering contract and add tests documenting that exception.

## Test Rigor

Appropriately covered at Level 1:

- `::shell` list indentation, tab indentation, interior blank-line indentation, root-level no-indent,
  trailing-newline behavior, block-quote marker replay, nested block-quote replay, and CommonMark
  structural assertions.
- `::shell-block` list indentation, tab indentation, root-level no-indent, trailing-newline behavior,
  empty-output behavior, single and nested block-quote marker replay, and CommonMark structural
  assertions.

Gaps or failures:

- Focused `shell_expansion` Level 1 suite fails deterministically.
- `::shell-block` output-preservation behavior is not verified and currently fails the spec as written.

No Level 2 or Level 3 tests are required for this feature. The user-observable behavior is Markdown
composition and CommonMark structure; it does not depend on terminal emulator rendering, terminal input
encoding, or OS keyboard injection.

## Verification

- `cargo test -p darkmatter --lib shell_blocks --color=never`: passed, 88 tests.
- `cargo test -p darkmatter --lib parse_utils --color=never`: passed, 9 tests.
- `cargo test -p darkmatter --lib shell_expansion --color=never`: failed, 368 passed, 1 failed, 1 ignored.
- `cargo test -p darkmatter --lib allow_once_persists_across_sibling_transclusions --color=never`: failed.

I also observed an unrelated untracked empty file named `::end-block` in the repository root and did
not modify it.

## Readiness

Not ready for production under the review criteria. The main indentation mechanics now look well
covered, including nested block quotes, but the focused shell-expansion suite fails and the
`::shell-block` output-preservation requirement is not satisfied as written.
