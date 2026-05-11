---
ready: true
agent: codex
model: ""
---

# Review: Unary Ops for Interpolation

## Findings

### Low: interpolation docs still describe the old code-span exclusion behavior

[docs/inline/interpolation.md](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/inline/interpolation.md:91) says the body interpolation scanner skips inline code and fenced code blocks. The implementation and tests now intentionally process inline code spans and skip only fenced/indented code blocks, as described in [interpolation/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/interpolation/mod.rs:42) and verified by the rewrite tests.

This is not a production blocker for unary interpolation, but it is user-facing documentation drift. Update the implementation section to say inline code spans are interpolated by default while fenced and indented code blocks are skipped.

Verification level: documentation-only issue. No Level 2 or Level 3 terminal verification is applicable.

## Verification Rigor

| Requirement | Strongest present verification | Match? |
| --- | --- | --- |
| Content interpolation supports `cond ? truthy : falsy` over truthy/falsy values | Level 1 `Markdown::compose` integration and evaluator unit tests | Yes |
| Frontmatter interpolation supports ternary expressions | Level 1 `Markdown::compose` integration | Yes |
| Recursive ternaries work in the true branch | Level 1 parser, evaluator, rewrite, and compose integration tests | Yes |
| Recursive ternaries work in the false branch | Level 1 parser, evaluator, rewrite, and compose integration tests | Yes |
| Parentheses can group complete expressions and reject malformed groupings | Level 1 parser unit tests, including the invalid groupings from the spec | Yes |
| Ternary branches may contain nested `{{ }}` placeholders that are rescanned | Level 1 rewrite tests and compose integration tests for true and false branches | Yes |
| Frontmatter dependency ordering sees variables inside ternary/fallback ASTs | Level 1 compose integration tests for later-declared templated dependencies | Yes |
| Page-block `when` expressions can use ternary results | Level 1 compose integration | Yes |

The feature has no terminal rendering, key input, paste, mouse, IME, scrolling, or emulator-encoded behavior. Level 1 is therefore the appropriate verification level for the user-observable behavior in this spec; Level 2 and Level 3 tests are not required here.

## Notes

The previous review's frontmatter dependency-ordering issue appears addressed. [frontmatter_interpolation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:287) now parses interpolation expressions and walks the AST to collect frontmatter roots through `Ternary`, `Fallback`, `Comparison`, `Paren`, unary operators, and function args. [ternary_integration.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/ternary_integration.rs:89) covers the later-declared templated `spec` dependency that failed in review 3.

I did not find remaining functional gaps against the spec. The implementation is ready for production after accepting the low documentation cleanup as non-blocking.

## Verification Run

- `cargo test -p darkmatter --test ternary_integration` — passed, 11 tests.
- `cargo test -p darkmatter markdown::compose::expression::parser::tests::ternary_expressions` — passed, 8 tests.
- `cargo test -p darkmatter markdown::compose::expression::parser::tests::error_cases` — passed, 17 tests.
- `cargo test -p darkmatter markdown::compose::frontmatter_interpolation::tests::interpolate_frontmatter_tests` — passed, 9 tests.
- `cargo test -p darkmatter markdown::compose::interpolation::rewrite::tests` — passed, 14 tests.
