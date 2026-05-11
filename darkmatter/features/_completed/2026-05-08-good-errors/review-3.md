---
ready: true
agent: codex
model: ""
---

# Review: Good Errors (2026-05-08) - Review 3

The implementation has moved substantially since the previous reviews: the code now has the `StatusBlock::body`/`body_line` API, `SourceContext`, docs, the darkmatter error skill, snapshot files, ANSI-preserving assertions, and Level 2 WezTerm tests for the canonical page-block error. I still do not consider the feature production-ready because there are remaining correctness and coverage blockers.

## Findings

### High: `SourceContext::frontmatter_prose()` can panic on valid frontmatter

`biscuit-terminal/lib/src/errors/source_context.rs:129` computes byte offsets by iterating `content.lines()` and then unconditionally sets `end_byte = line_end + 1` at `source_context.rs:138`. For a valid file that ends immediately after the closing frontmatter delimiter, e.g. `---\ntitle: Test\n---`, that range extends one byte past `content.len()`. `frontmatter_prose()` then slices `&self.content[range.clone()]` at `source_context.rs:71`, which panics while rendering the error block.

The same offset code assumes `\n` is always one byte at `source_context.rs:142`, so CRLF input undercounts each line separator and can produce a malformed frontmatter slice. This breaks spec §3.1 requirement 3, "Frontmatter snapshot when the error originated in a document with frontmatter."

Verification level present: Level 1 unit tests cover only `---\ntitle: Test\n---\n# Body\n` and missing-frontmatter cases. There is no Level 1 test for no trailing newline or CRLF frontmatter.

Recommended fix: derive byte ranges with `char_indices`/`split_inclusive` or an existing frontmatter detector that preserves exact byte spans. Add unit tests for frontmatter ending at EOF and CRLF input.

### High: snapshot coverage still misses `FileReference` variants

The spec §3.6 says each `BlockError` variant gets a checked-in snapshot. Two variants in the swept error files are not exercised:

- `TransclusionError::FileReference` is implemented at `darkmatter/lib/src/markdown/compose/transclusion/types.rs:641`, but `darkmatter/lib/tests/error_snapshots/transclusion.rs` has no `file_reference` test.
- `ReferenceError::FileReference` is implemented at `darkmatter/lib/src/markdown/reference/errors.rs:113`, but `darkmatter/lib/tests/error_snapshots/reference.rs` has no `file_reference` test.

These are user-facing status blocks, so their strongest verification level is currently none. Minimum required is Level 1 snapshot coverage.

Recommended fix: construct representative `biscuit_file::FileReferenceError` values or trigger them through the parser/resolver and add snapshots beside the existing I/O/URL cases.

### Medium: path and diagnostic values are interpolated into Prose without escaping

Several user-controlled strings are placed inside Prose tags as raw text, for example `SourceContext::linked_path_prose()` writes `href="{}"` and `{}` directly at `biscuit-terminal/lib/src/errors/source_context.rs:62`, and transclusion/reference bodies wrap raw `reference`, `message`, `raw`, `name`, and path values in Prose markup. A path containing `"`, `<`, `>`, or a diagnostic value containing markup-looking text can change the generated Prose structure or styling rather than being displayed literally.

This is adjacent to the original bare-markup leak: the body now parses Prose correctly, but user content is not consistently escaped before entering that grammar.

Verification level present: Level 1 snapshots use benign values only. There are no tests with paths/messages containing quotes, angle brackets, or closing tags.

Recommended fix: provide a small `Prose::escape_text`/`escape_attr` helper and use it for all user content inserted into Prose markup. Add targeted Level 1 tests with adversarial path and message strings.

## Verification Notes

I ran:

- `cargo test -p biscuit-terminal frontmatter_detection --lib` - passed, but it only exercises the currently covered happy paths.
- `cargo test -p darkmatter --test error_snapshots page_block::unterminated_block_shows_opening_line -- --exact` - did not complete; the build was terminated by SIGTERM while compiling dependencies after about a minute. I did not get a meaningful test failure from darkmatter itself.

## Verification Level Summary

| Requirement | Strongest present | Gap |
|---|---:|---|
| No bare markup from `StatusBlock::body` | Level 1 snapshots plus ANSI-preserving unit assertions | Acceptable for the API contract |
| Canonical page-block OSC 8 link and inverse hint | Level 2 WezTerm tests plus Level 1 ANSI assertions | Acceptable |
| Canonical page-block gutter/dim code excerpt | Level 2 WezTerm test | Acceptable |
| Frontmatter snapshot rendering | Level 1 happy-path only | Not enough: missing EOF/CRLF cases, current code can panic |
| Every swept `BlockError` variant has snapshot coverage | Level 1 for most variants | Missing `FileReference` variants |
| User-controlled content inside Prose grammar is displayed literally | Not covered | Add escaping plus adversarial tests |

## Readiness

Not ready for production. The remaining issues are smaller than the prior reviews, but a panic in the shared source-context renderer and missing required variant snapshots are release blockers for this feature.
