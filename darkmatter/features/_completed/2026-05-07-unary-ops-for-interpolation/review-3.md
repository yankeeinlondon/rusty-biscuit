---
ready: false
agent: codex
model: ""
---

# Review: Unary Ops for Interpolation

## Findings

### High: frontmatter ternaries can resolve templated-key dependencies too early

Frontmatter interpolation still uses dependency ordering, but the dependency scanner only recognizes expressions that are a single bare identifier. In [frontmatter_interpolation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:220), each templated key calls `extract_simple_key_refs`, and [frontmatter_interpolation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:283) explicitly excludes expressions with operators. That means `{{ use ? spec : 'none' }}` does not declare a dependency on the templated `spec` key.

This breaks a natural extension of the new feature: selecting a frontmatter-derived branch where the selected value is another templated frontmatter property. I reproduced it with:

```md
---
use: true
message: "{{ use ? spec : 'none' }}"
base: "{{ ctx.today }}"
spec: "{{ base }}/spec.md"
---
{{message}}
```

Running `cargo run -q -p darkmatter-cli -- compose <file> --output markdown` produced an empty body instead of `2024-06-15/spec.md`. Because `message` appears before `base`/`spec`, it is evaluated with `spec` absent from the seed map. The current tests in [ternary_integration.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/ternary_integration.rs:25) cover frontmatter ternaries only when all referenced values are already seed values, so this ordering failure is not covered.

Suggested fix: make frontmatter dependency extraction parse interpolation expressions and collect unprefixed `Expr::Variable` roots from the AST, including variables inside ternary conditions and both branches. Treat `ctx.*` and `env.*` as non-frontmatter dependencies, but include roots like `spec` even when they appear inside `Fallback`, `Ternary`, `Comparison`, `Paren`, or function args. Then add a compose-level regression where a frontmatter ternary references a templated key declared later in the YAML.

Verification level: Level 1 is the appropriate level for this requirement because this is deterministic compose-pipeline text transformation. Current strongest verification for this dependency case is none; existing Level 1 tests miss it. No Level 2 or Level 3 terminal verification is required for this feature as specified.

## Verification Rigor

| Requirement | Strongest present verification | Match? |
| --- | --- | --- |
| Simple interpolation ternary chooses truthy/falsy branch | Level 1 `Markdown::compose` integration | Yes |
| Recursive ternary in true branch | Level 1 parser, evaluator, and compose integration | Yes |
| Recursive ternary in false branch | Level 1 parser/evaluator and false-branch nested interpolation integration | Yes |
| Parentheses preserve grouping and reject invalid groupings | Level 1 parser tests | Yes |
| Selected branch strings are rescanned for nested `{{ }}` placeholders | Level 1 rewrite and compose integration | Yes |
| Frontmatter interpolation supports ternary expressions over seed values | Level 1 compose integration | Yes |
| Frontmatter ternaries participate correctly in templated-key dependency ordering | No direct test; reproduced broken behavior | No |
| Page-block `when` supports ternary expressions | Level 1 compose integration | Yes |

## Notes

The previous review's blocking nested-interpolation issue appears addressed: selected ternary branch text is rescanned with depth protection, and tests now cover both true and false branches. `cargo test -p darkmatter --test ternary_integration` passed: 9 tests passed.

There is also a small documentation drift in [interpolation/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/interpolation/mod.rs:40): the module docs still say code spans are not processed, while the current scanner intentionally processes inline code spans and skips fenced/indented blocks only. This is not a production blocker for unary interpolation, but it should be cleaned up while touching the docs.

## Recommendation

Do not mark this production-ready until frontmatter dependency ordering understands variables inside ternary expressions and the later-declared templated-key case is covered by a Level 1 compose integration test.
