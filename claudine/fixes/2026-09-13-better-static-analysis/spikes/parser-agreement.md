---
created: 2026-09-13
kind: spike
question: Can DMLS report `EXPRESSION_MALFORMED` on an expression that Darkmatter's composer accepts, and if so, what must be true before raising that diagnostic from WARNING to ERROR?
---

# Parser agreement: DMLS vs. Darkmatter on `EXPRESSION_MALFORMED`

## The question

D8 of `claudine/fixes/2026-09-13-better-static-analysis/spec.md` raises
`dm.expression.malformed` from `WARNING` to `ERROR` on the rule "an error means
it will never work". That is only safe if DMLS's parse verdict is not stricter
than Darkmatter's. This spike asked whether DMLS can call an expression
malformed that Darkmatter accepts, and under what conditions.

## What I did

1. Read both producers (`diagnostics/frontmatter.rs`, `providers/dsl.rs`), the
   DMLS parse overlay, Darkmatter's expression lexer/parser, its `expression`
   schema-format validator, and `interpolate_text`.
2. Built a throwaway binary outside the repo that constructs a real
   `DocumentContext` exactly the way `diagnostics::frontmatter`'s own test
   helper does (`OverlayState::for_document` + `SourceMap` + `WorkspaceGraph`)
   and runs both diagnostic producers over ~50 crafted documents.
3. Ran a 59-item expression corpus through **both** parse dialects and compared
   accept/reject.
4. Probed `decode_scalar` directly on every YAML scalar style.

Scratch crate (not in the repo):
`/private/tmp/claude-501/-Volumes-coding-wt-rusty-biscuit-fix-cli-slow-tests/6a3e76e1-d646-4bc3-b900-9e820e518340/scratchpad/spike`.
No repository file was modified.

## Findings

### 1. DMLS has no parser of its own — confirmed

`darkmatter/dmls/src/overlay/expressions.rs` is a thin wrapper: `parse` →
`darkmatter::…::parse_spanned`, `parse_condition` →
`darkmatter::…::parse_condition_spanned`, interpolation regions from
Darkmatter's `ExpressionFinder`. `dmls/Cargo.toml` carries no parser
dependency of its own; `rlsp-yaml-parser` is YAML-only and is confined to the
frontmatter overlay. **There is no independent expression grammar in DMLS.**
Dialect routing and YAML scalar decoding are therefore the only mismatch
vectors, and the risk is bounded.

### 2. The two dialects accept exactly the same language

`ParseMode` is branched on in exactly one place in the lexer
(`darkmatter/lib/src/markdown/compose/expression/lexer.rs:584-593`: `||`
becomes `Token::Pipe` in interpolation mode and `Token::OrOr` in condition
mode; bare `|` is an error in both) and one place in the parser
(`parser.rs:275-279`: `parse_ternary_branch` dispatches to `parse_logical_or`
vs `parse_fallback`). Those two productions are structurally identical
(`parser.rs:288` and `parser.rs:334`) — each consumes its own `||` token at the
same precedence, differing only in the AST node produced (`or(a, b)` vs
`Fallback`).

Empirically: **59 inputs covering every grammar production (member access,
index, unary, all arithmetic and comparison operators, `&&`, `||`, nested
ternaries, function calls, both quote styles, and eleven deliberately
malformed inputs), zero divergences** between `parse` and `parse_condition`.

Darkmatter's own comment at `schemas/format.rs:107-110` calls condition mode "a
parse superset"; the measurement says it is not a superset but an *equality* of
accepted strings. Either way, **routing an interpolation-dialect value to
`parse_condition` cannot produce a malformed false positive.** Line of
investigation 1 is closed: dialect mismatch is a semantics risk (`||` meaning
fallback vs OR), not a malformed-diagnostic risk.

### 3. The frontmatter producer is subordinate to Darkmatter's own verdict

The guard at `darkmatter/dmls/src/diagnostics/frontmatter.rs:622` is much
broader than its comment suggests. `union_rejected_paths` (`frontmatter.rs:307`)
is `report.problems.iter().map(|p| p.path)` — **every** path carrying **any**
validation problem, not only union arms. So the producer emits
`EXPRESSION_MALFORMED` only when Darkmatter's schema validation *also* rejected
that value.

That matters because Darkmatter's `expression` format validator
(`darkmatter/lib/src/markdown/schemas/format.rs:243-254`) has a pending-value
deferral DMLS does not replicate:

```rust
if value.contains("$(") || value.contains("{{") {
    return true;
}
```

A value like `when: '{{ ctx.area }}'` or `when: "$(git branch) == 'main'"` is
accepted by Darkmatter and would be rejected by a bare `parse_condition`. The
`union_rejected` guard catches it — *because* the deferral means no problem is
recorded — and the empirical run confirms **0 malformed diagnostics** for both.
This is a **coincidental** coupling, not a designed one; see precondition C4.

Empirical result for the frontmatter producer (schema `when: expression`), all
`malformed=0` unless noted:

| Value | Result |
| --- | --- |
| `a \|\| b`, `a && b`, `a \|\| 'x'`, ternary, nested ternary, `as_csv(items)`, `'x' + a + 'y'`, `ctx.repo == 'main'` | 0 |
| single-quoted `''` escape, double-quoted `\"` escape, plain unquoted, multiline plain, trailing `# comment` | 0 |
| `\|-`, `\|`, `\|+`, `>-` block scalars holding a valid expression | 0 |
| `!!str 'a == 1'`, `*anchor` alias | 0 |
| `'{{ ctx.area }}'`, `"$(git branch) == 'main'"`, `'{{ ctx.area }} 1 +'` | 0 |
| `'1 +'` (control), `'a \| b'` | 1 — correct |

**I could not construct a case where the frontmatter producer flags a value
Darkmatter accepts.** The guard held against every vector I tried.

### 4. `decode_scalar` mis-decodes more styles than the spec records

The spec's D3 names block scalars. Probing `decode_scalar`
(`darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs:71`, called
unguarded at `darkmatter/dmls/src/providers/frontmatter.rs:916`) shows the
defect is wider — every style whose first byte is neither `'` nor `"` falls
through to `plain_scalar(raw.trim_end())`:

| Authored | `decode_scalar` yields | parses? |
| --- | --- | --- |
| `\|-\n  a == 1` | `"\|-\n  a == 1"` | no |
| `\|\n  a == 1` | `"\|\n  a == 1"` | no |
| `\|+\n  a == 1` | `"\|+\n  a == 1"` | no |
| `>-\n  a == 1` | `">-\n  a == 1"` | no |
| `\|2-\n  a == 1` | `"\|2-\n  a == 1"` | no |
| `!!str 'a == 1'` | `"!!str 'a == 1'"` | no |
| `*anchor` | `"*anchor"` | no |
| plain / single / double quoted, multiline plain | correct | yes |

So a fix that guards only `|`-family block scalars leaves `>`-family folded
scalars, explicit indentation indicators (`|2-`), YAML tags, and aliases with
the same defect. **The spec's statement that the D3 guard is sufficient is not
correct as written** (line of investigation 5).

What saves this today is again the `union_rejected` guard: for all of these,
Darkmatter validated the *decoded YAML* value, found it fine, recorded no
problem, and DMLS skipped. The observable damage is therefore **not a false
positive — it is a wrong message and a wrong range on a true positive**:

```
when: |-
  1 +
```

emits `malformed expression: Unexpected '|'. Use '||' for logical OR.` ranged
at the `|-` indicator, not `Expected expression, found '<EOF>'` ranged at the
`1 +`. Same for `>-\n  1 +`, which reports `Expected expression, found '>'`.
At `WARNING` that is a cosmetic wrong-squiggle. At `ERROR` it is a blocking red
squiggle on the block-scalar indicator with an explanation that describes a
character the author did not write.

### 5. The body producer has no guard at all — and that is defensible

`darkmatter/dmls/src/providers/dsl.rs:670-694` parses every body interpolation
with `expressions::parse` and pushes `EXPRESSION_MALFORMED` unconditionally.
That is safe *with respect to compose*, because it is the same code path:
`interpolate_text` (`darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:97`)
uses the same `ExpressionFinder::new(...).find_all()` and the same `parse`, and
on a parse error either returns `MarkdownError` (`fail_fast`) or emits a
`ComposeWarning` and leaves the `{{ … }}` verbatim in the output
(`rewrite.rs:175-186`). A malformed body interpolation is **never** evaluated,
so "will never work" is literally true and `ERROR` is the right severity.

Empirically consistent: `{{ a || b }}`, `{{ a && b }}`, ternaries, function
calls, `{{{ … }}}` literals (excluded from `find_all`), `{{ err.kind }}`,
`{{ timing.total }}`, `{{ current.ctx.x }}`, and `{{ }}` inside fenced code
all produce 0. Bare `|`, `$(…)`, and truncated expressions produce 1 — all
genuinely unevaluatable.

### 6. Claudine extends the namespace, not the grammar

`claudine/lib/src/dispatch/matcher.rs:67`, `dispatch/runner/mod.rs:103`, and
`dispatch/template.rs:477` all construct Darkmatter's own
`Parser::with_mode(...)` / `parse_condition`. Claudine adds identifiers
(`err`, `timing`, `current`) and functions (`file_exists`, …) resolved at
evaluation time. Functions are `IDENT "(" args ")"` in the shared grammar, so
a new function is not a grammar extension. **No new operators, no new syntax.**
Claudine's extras can therefore only produce
`EXPRESSION_UNKNOWN_IDENTIFIER`, never `EXPRESSION_MALFORMED` — confirmed
empirically. Line of investigation 4 is closed in the benign direction.

### 7. The real blast radius is applicability, not parser disagreement

The body producer fires on **any** markdown buffer in the workspace, with or
without frontmatter, and DMLS has no per-code severity or enablement toggle
(`DiagnosticsConfig` at `darkmatter/dmls/src/config/mod.rs:220` holds only
`debounce_ms`). Foreign mustache-family templating in a non-Darkmatter document
becomes a hard error:

| Body | malformed |
| --- | --- |
| `{{#each items}}x{{/each}}` (Handlebars) | **2** |
| `{{ x \| default: 1 }}` (Liquid) | **1** |
| `{{ a\|upper }}` (Jinja) | **1** |
| `{{> partial}}` (Mustache) | **1** |
| `{{ $json.id }}` (n8n) | **1** |
| `{{ site.title }}`, `{{ item.name }}`, `{{ matrix.os }}` | 0 (valid Darkmatter syntax) |

Same counts with no frontmatter block at all. This is the largest practical
risk of the raise and it is **not** a DMLS-vs-Darkmatter disagreement — it is
DMLS asserting a hard verdict on documents Darkmatter will never compose.
It is out of scope for the spec as written, but it is the change users will
notice, and D7 (wider firing) compounds it.

## Verdict

**Raising `EXPRESSION_MALFORMED` to `ERROR` is safe with respect to the
original risk.** DMLS reuses Darkmatter's parser verbatim; the two dialects
accept identical strings; Claudine adds no grammar; and the frontmatter
producer only speaks when Darkmatter's own validation already rejected the
value. I found no expression that DMLS calls malformed and Darkmatter accepts.

The raise is **conditional** on the following, in priority order.

**C1 (blocking) — widen the D3 decode guard beyond block scalars.**
`decode_scalar` at `providers/frontmatter.rs:916` mis-decodes `|`, `|-`, `|+`,
`>`, `>-`, `|2-`, `!!tag`-prefixed scalars, and `*alias` references. Today each
produces a wrong message and a wrong range on an otherwise-correct diagnostic.
The spec's "block-scalar guard" is necessary but not sufficient. Either decode
all of them, or make the producer skip any value whose authored first byte is
not `'`, `"`, or a plain-scalar start — skipping is the safe direction because
the `union_rejected` guard means the schema layer still reports the failure
generically.

**C2 (blocking) — regression tests for the wrong-range case at ERROR.**
A `|-` block holding `1 +` must range and describe the `1 +`, not the `|-`.
Currently `frontmatter.rs:1445`-era tests assert only diagnostic counts, so the
range corruption is untested.

**C3 (blocking) — preserve the `union_rejected` guard, explicitly.**
`frontmatter.rs:622` is the single reason the frontmatter producer cannot
outrank Darkmatter. It currently reads as a union-arm special case; its comment
should be corrected to state the real invariant ("never report malformed for a
value the schema validation accepted") and it should carry a test that pins the
invariant, because D7's descent into sequences and nested shapes will add new
callers into this producer.

**C4 (blocking if C3 is ever relaxed) — replicate the pending-value deferral.**
Darkmatter accepts any `expression` value containing `$(` or `{{`
(`format.rs:249`). DMLS does not. Today C3 hides the difference. If any future
change reports malformed without consulting the validation report, DMLS must
apply the same deferral first, or `when: '{{ ctx.area }}'` and
`when: "$(git rev-parse --abbrev-ref HEAD) == 'main'"` become blocking errors
on valid documents.

**C5 (should, not must) — decide the foreign-template question before shipping.**
The body producer will put hard errors on Handlebars, Liquid, Jinja, Mustache,
and n8n snippets in any `.md` file in the workspace. Options, cheapest first:
(a) ship as-is and say so in the PR description; (b) add a `diagnostics`
severity override to `DmlsConfig` so a workspace can dial the code back;
(c) restrict the body producer's `ERROR` severity to documents DMLS can tell
are Darkmatter documents (frontmatter present, or a `$schema` / known
Darkmatter key). This is a ruling for Ken, not a technical blocker.

**Not conditional on anything:** the dialect-routing concern in the spec's
framing. It is a real semantics difference (`||` as fallback vs logical OR) but
it cannot produce a malformed diagnostic, and no task is needed for it.

## Not settled within the budget

- I could not *prove* the `union_rejected` guard is unbypassable — only that
  every vector I constructed (pending markers, all scalar styles, tags,
  aliases, mixed unions, both arm orders) was suppressed. A value that both
  fails schema validation at its own path *and* is a valid expression after
  correct YAML decoding would defeat it; I could not construct one, because
  the `expression` format is the only constraint the type carries. If D7 adds
  constraint-bearing expression properties, re-check.
- I did not run `md compose` end-to-end against the false-positive candidates;
  the compose-side evidence here is read from `interpolate_text` and from the
  shared-parser identity, not from a composed document.
- I did not audit the `nested_span_in_literal` diagnostic D8 also introduces at
  `ERROR`; it did not exist to test.
