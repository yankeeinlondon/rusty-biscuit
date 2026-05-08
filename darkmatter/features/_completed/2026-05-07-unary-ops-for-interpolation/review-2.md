---
ready: false
agent: codex
model: ""
---

# Review: Unary Ops for Interpolation

## Findings

### High: the spec's canonical Frontmatter example is still not implemented or tested

The parser now supports recursive ternaries, but the user-facing example in the spec is stronger than "choose one static branch". It shows a selected branch that itself contains `{{ctx.current_package}}`:

```yaml
in_pkg_dir: "{{ctx.current_package}} ? 'in a package directory: {{ctx.current_package}}' : 'not in a package directory'"
```

There are two gaps here:

- Written exactly as specified, only the first `{{ctx.current_package}}` is an interpolation expression; the remaining `? ... : ...` text is left as literal text after frontmatter interpolation.
- Written in the expression grammar's valid shape, for example `{{ ctx.current_package ? 'in a package directory: {{ctx.current_package}}' : 'not in a package directory' }}`, the selected branch string is returned as a replacement but is not rescanned, so the inner `{{ctx.current_package}}` remains unresolved.

The reason is in [rewrite.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:54): interpolation locations are collected once from the original input, then replacements are applied in reverse order at [rewrite.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:71). No pass evaluates newly introduced `{{ }}` text from a selected ternary branch. I confirmed this with `md compose` on `{{ pkg ? 'in package: {{ pkg }}' : 'not in package' }}`, which produced `in package: {{ pkg }}`. The existing integration tests in [ternary_integration.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/ternary_integration.rs:5) cover static branch strings, nested branch selection, frontmatter ternaries, page-block conditions, parentheses, and comparisons, but not the spec's templated branch output.

Requirement verification level: Level 1 is the appropriate level for this compose-pipeline behavior because it is deterministic text transformation and does not depend on terminal encoder/renderer behavior. Current strongest verification is none for this specific requirement. This is a blocker unless the spec is corrected to say branch strings are literal and must not contain nested interpolation.

## Verification Rigor

| Requirement | Strongest present verification | Match? |
| --- | --- | --- |
| Simple interpolation ternary chooses truthy/falsy branch | Level 1 integration via `Markdown::compose` | Yes |
| Recursive ternary in true branch | Level 1 parser and compose integration | Yes |
| Recursive ternary in false branch | Level 1 parser only | Acceptable for parser behavior, but a compose-level false-branch integration test would be better |
| Parentheses preserve grouping and reject invalid groupings | Level 1 parser tests | Yes |
| Frontmatter interpolation supports ternary expressions | Level 1 compose integration | Yes |
| Page-block `when` supports ternary expressions | Level 1 compose integration | Yes |
| Spec example with templated text inside a selected branch | No direct test; observed implementation path does not support it | No |

No Level 2 or Level 3 terminal verification is required for this feature as specified; it has no terminal rendering, keyboard, paste, mouse, scroll, or emulator-encoded input behavior.

## Notes

The implemented recursive parser shape is otherwise sound: `ternary = fallback ("?" ternary ":" ternary)?`, with `Expr::Paren` preserving user grouping for display/debugging and evaluator support for `BoolLiteral`, `Paren`, and recursive `Ternary`. Boolean literals also address the ergonomics issue from the previous review.

`cargo test -p darkmatter ternary --lib --tests` passed, but emitted unused-import warnings in [ternary_integration.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/ternary_integration.rs:2). Those imports should be removed while touching the test file.

## Recommendation

Do not mark this production-ready until either:

- interpolation recursively resolves selected branch strings in frontmatter/body interpolation, with loop-depth protection and tests for the exact spec pattern; or
- the spec and docs are changed to explicitly require branch text to be literal and to show the equivalent supported pattern without nested `{{ }}` inside string literals.
