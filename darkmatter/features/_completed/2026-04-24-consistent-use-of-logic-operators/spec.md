# Consistent Use of Logic Operators

> **IMPORTANT:** use the 'darkmatter' skill.

Darkmatter's expression language currently overloads the single pipe (`|`) as a fallback operator in both interpolation (`{{ ... }}`) and conditions (`when="..."`). That overload competes with the more familiar `||` / `&&` boolean operators, splits the operator surface between the two parse modes, and makes frontmatter shell expressions (`$(...)`) particularly confusing because darkmatter's shell tokenizer rejects every `|` up-front.

This feature removes the single `|` spelling entirely. Going forward, `||` is the only fallback/OR operator in darkmatter expressions, and `&&` is the only logical-AND operator.

## Motivation

A quick tour of the three places pipes show up today:

1. **Interpolation** — `{{ name | "default" }}` and `{{ name || "default" }}` both mean fallback. `&&` is a parse error.
2. **Conditions** — `when="a | b"` means fallback, `when="a || b"` means logical OR, `when="a && b"` means logical AND.
3. **Frontmatter shell** — `$(...)` tokenizer rejects any `|` (including `||`) as "shell pipes are not allowed", forcing authors to push OR-style choices into interpolation.

This produces three concrete problems:

- Authors coming from JavaScript, Rust, Bash, or almost any mainstream language reach for `||` expecting logical OR and silently get fallback in interpolation mode.
- The same glyph `|` carries two subtly different meanings (fallback vs. nothing-allowed) depending on whether it sits inside `{{ ... }}` or `$(...)`.
- Condition expressions are the only place where `|` and `||` diverge, but that divergence leaks into every doc and test that demonstrates fallback.

A single spelling per operator — `||` for fallback/OR, `&&` for AND — removes every one of these friction points.

## Goals

- Single canonical operator for fallback: `||`.
- Single canonical operator for logical OR in conditions: `||` (unchanged from today).
- Single canonical operator for logical AND in conditions: `&&` (unchanged from today).
- Eliminate the single pipe (`|`) as a valid operator in all darkmatter expression modes.

## Non-Goals

- Changing the meaning of `&&` or introducing a new AND spelling.
- Changing shell tokenizer behavior — it continues to reject pipe characters entirely.
- Adding new operators such as `??` for nullish coalescing (see **Open Questions** for why this is called out but not part of the goal set).
- Changing truthiness rules, comparison operators, or function-call syntax.

## Current vs. Proposed Operator Surface

| Context                     | Operator | Today                   | After this change       |
|-----------------------------|----------|-------------------------|-------------------------|
| Interpolation `{{ ... }}`   | `\|`     | fallback                | **removed (parse error)** |
| Interpolation `{{ ... }}`   | `\|\|`   | fallback (alias)        | fallback                |
| Interpolation `{{ ... }}`   | `&&`     | parse error             | parse error (unchanged) |
| Condition `when="..."`      | `\|`     | fallback                | **removed (parse error)** |
| Condition `when="..."`      | `\|\|`   | logical OR              | logical OR              |
| Condition `when="..."`      | `&&`     | logical AND             | logical AND             |
| Frontmatter shell `$(...)`  | `\|` / `\|\|` | parse error ("pipes not allowed") | parse error (unchanged) |

The only behavior change is that bare `|` becomes a parse error in both interpolation and condition expressions. Every `||` keeps its current meaning in the context it appears.

## Example Migrations

Interpolation fallback:

```md
<!-- before -->
{{ name | "friend" }}
{{ spec | design }}
{{ env.EDITOR | env.VISUAL | "vi" }}

<!-- after -->
{{ name || "friend" }}
{{ spec || design }}
{{ env.EDITOR || env.VISUAL || "vi" }}
```

Condition fallback:

```md
<!-- before -->
::block when="env.AGENT | env.DEFAULT_AGENT"
::file ./notes.md when="(env.AGENT | env.DEFAULT_AGENT) == 'claude'"

<!-- after -->
::block when="env.AGENT || env.DEFAULT_AGENT"
::file ./notes.md when="(env.AGENT || env.DEFAULT_AGENT) == 'claude'"
```

Mixed fallback and boolean logic in a condition becomes simpler because readers no longer need to memorize that `|` binds tighter than `&&`:

```md
<!-- before -->
::file ./notes.md when="(env.AGENT | env.DEFAULT_AGENT) == 'claude' && !draft"

<!-- after -->
::file ./notes.md when="(env.AGENT || env.DEFAULT_AGENT) == 'claude' && !draft"
```

## Affected Code and Docs

Rough blast radius (counts from the current tree):

- Lexer: `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs` — remove the single-`|` path that produces `Token::Pipe`; `||` continues to lex as `Token::Pipe` in interpolation mode and `Token::OrOr` in condition mode, or is renamed as part of the refactor.
- Parser: `darkmatter/lib/src/markdown/compose/interpolation/parser.rs` — `parse_fallback` keeps its shape; the token it consumes is whatever `||` maps to.
- AST / evaluator: no semantic change to `Expr::Fallback`; naming may be refreshed for clarity.
- Tests: all `fallback` / `pipe` tests in the lexer and parser modules need to switch their inputs from `|` to `||`, and new tests should assert that bare `|` produces a parse error.
- Docs:
    - `darkmatter/docs/inline/interpolation.md`
    - `darkmatter/docs/inline/fm-interpolation.md`
    - `darkmatter/docs/topics/boolean-conditional-logic.md` (including the mode-comparison table at lines 91–94)
    - Any rustdoc examples that still use `|` for fallback.
- Skill content: `.claude/skills/darkmatter/SKILL.md` fallback example, and any referenced topic docs.
- Historical feature docs under `darkmatter/features/_completed/` should be left as-is — they describe the state of the world at the time.

A repo-wide audit should grep for:

- `\|` not preceded or followed by another `|` inside `{{ ... }}` and `when="..."` bodies
- rustdoc and markdown code blocks mentioning fallback

## Error Messaging

When the parser encounters a bare `|` it should produce an error that names the replacement explicitly, so users are not left guessing. Rough shape:

```
Unexpected '|' at position <N>. Use '||' for fallback.
```

If the surrounding context is a condition expression, the hint can lean further into logical-OR language:

```
Unexpected '|' at position <N>. Use '||' for logical OR / fallback.
```

Error messages should flow through the `BlockError` work already in progress so the rendered terminal output stays consistent.

## Backward Compatibility

This is a breaking change to document authors. There is no silent rewrite step — existing docs that use `|` will fail composition with the new parser error after the cut-over.

Mitigations:

- Provide a clear, actionable error message (see above).
- Land the doc and in-tree example migrations in the same change so the reference material never lags the parser.
- Call the change out in the darkmatter CHANGELOG / release notes once the feature is scheduled.

No migration tool is planned for this feature. If adoption friction becomes a real problem later, a `md migrate` subcommand could be considered, but it is out of scope here.

## Open Questions

1. **Does `||` keep distinct meanings per mode, or should it unify?**
    - Today (and under this spec as written) `||` means fallback in interpolation and logical OR in conditions. The spec preserves that split because interpolation's job is to produce values, not booleans.
    - A stricter unification would have `||` always mean logical OR and introduce a second operator (for example `??`) for fallback. That is a larger change with its own migration cost and is **not** part of this feature unless the reviewer decides otherwise.
    - **DECISION:** `||` should always mean logical OR, `|` should never act as a logical OR
2. **Should the removal be a hard error or a deprecation warning in the first release?**
    - Default assumption: hard error at parse time.
    - Alternative: emit a compose warning for one release cycle and flip to a hard error after.
    - **DECISION:** hard error
3. **Does any consumer rely on `|` appearing as a literal pipe inside an expression?**
    - The existing lexer already treats `|` as a structural token, so literal pipes would have to be wrapped in string literals today. This should be verified but is unlikely to be a blocker.
    - NOTE: literal pipes are allowed and used on shell commands
