---
ready: false
agent: codex
model: ""
---

# Review: `::shell` / `::shell-block` Indentation Preservation, Iteration 2

## Findings

### High: `::shell-block` does not have the required nested block-quote fixture

The spec requires the `::shell-block` fixtures to mirror the `::shell` acceptance cases, including the nested block-quote form (`> > ::shell ...`) and CommonMark structural round-trip (`spec.md:59-63`). The `::shell` side now has raw and structural block-quote coverage, including `> > ::shell` (`shell_expansion/mod.rs:792-848`). The `::shell-block` side only covers a single block-quote marker (`> ::shell-block`) for raw output and HTML structure (`shell_blocks/mod.rs:651-670`); there is no `> > ::shell-block` fixture and no structural assertion for the nested block-quote case.

This is a verification-level gap, not necessarily an implementation failure. The implementation appears to share the same prefix capture path, so it may already work, but the strongest verification for that user-observable requirement is absent. Level 1 is the appropriate level because this is pure Markdown composition and CommonMark parsing, not terminal rendering or keyboard input.

Recommendation: add a Level 1 `::shell-block` fixture with `> > ::shell-block`, block-quoted body and closer lines, and output containing at least two lines plus an interior blank. Assert the raw output is prefixed with `> > ` on every emitted line, then render through CommonMark and assert it remains inside the nested block quote.

### Medium: Block-quoted page blocks became recognized as directives as a side effect

The shared block scanner now strips block-quote markers before matching both `::shell-block` and `::block` (`block_pairs.rs:98-116`). That was needed for shell-block acceptance, but it also changes page-block behavior: a quoted `> ::block ...` / `> ::end-block` region is now treated as an active conditional page block instead of literal quoted Markdown. The spec only covers body `::shell` and `::shell-block`; it does not call for changing `::block` semantics.

That can remove or rewrite user-visible quoted content during the page-block stage, especially for documentation that quotes directive examples in block quotes rather than fenced code. I did not find a page-block test that locks this behavior in either direction.

Recommendation: either constrain block-quote marker stripping to shell-block matching only, or document and test the broader page-block behavior change explicitly with true and false conditions.

## Test Rigor

Implemented and appropriately Level 1:

- `::shell` 4-space list indentation, tab indentation, interior blank-line indentation, root-level no-indent behavior, trailing-newline behavior, block-quote marker replay, nested block-quote marker replay, and CommonMark list / block-quote structural assertions.
- `::shell-block` 4-space list indentation, tab indentation, root-level no-indent behavior, trailing-newline behavior, empty-output behavior, single block-quote marker replay, and CommonMark list / single-blockquote structural assertions.

Missing or insufficient:

- `::shell-block` nested block-quote acceptance (`> > ::shell-block ...`) has no Level 1 raw-output or CommonMark structural test.
- The side-effect behavior for block-quoted `::block` page blocks has no targeted test coverage.

No Level 2 or Level 3 tests are required for this feature. The user-observable behavior is Markdown composition and parser structure, not terminal emulator rendering, terminal input encoding, or OS keyboard injection.

## Verification

I attempted the focused test filters:

- `cargo test -p darkmatter --lib shell_expansion --color=never`
- `cargo test -p darkmatter --lib shell_blocks --color=never`

Both processes exited with code `-1` during dependency compilation before running tests. No Rust test failure output was produced. The review above is therefore based on source and test inspection, not a completed local test run.

## Readiness

Not ready for production under the review criteria. The main implementation path looks substantially improved from iteration 1, but one explicit acceptance fixture for `::shell-block` is still missing at the required verification level, and the block scanner appears to broaden page-block behavior outside the requested feature.
