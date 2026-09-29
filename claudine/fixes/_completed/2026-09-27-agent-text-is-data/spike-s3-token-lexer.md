---
created: 2026-09-28
spike: S3
---

# Spike S3: token lexer surface

Which scanners meet `{{!data:v1:<base64url>}}` today, what each one does with
it, and whether N5's prefix check keeps every one of them from treating a token
as an expression or a shell candidate. Line numbers are as of `ff7219ffc`.

## Findings

Almost every `{{` scanner goes through `ExpressionFinder` (`expression/lexer.rs:133`).
So the prefix check has one natural home: **the scanner's classification of an
opener**, not the parser and not a raw substring search. The table's last
column says whether a check placed there covers the row.

| # | Scanner | file:line | Today on the token | Required change | Covered by the prefix check? |
|---|---|---|---|---|---|
| 1 | `ExpressionFinder::scan` / `scan_legacy_expression` | `darkmatter/lib/src/markdown/compose/expression/lexer.rs:133`, `:226` | Returns it as an expression `!data:v1:aGk`. `!` lexes as `Bang` (`:705`), and `:` then fails the parse: "Expected end of expression, found ':' at position 5". | When a two-brace opener is followed by `!data:`, emit a separate `tokens` list (span, payload) instead of an expression. Keep literal (`{{{`), escaped (`\{{`), and code-region rules ahead of it. | **y** (this is the check) |
| 2 | `interpolate_text_located` parse | `.../interpolation/rewrite.rs:205` (fast path `:163`) | Parse error, which is fatal under `Strict`. Under `Lenient` it becomes a warning and the text is kept. | Any token from row 1 raises `MalformedLiteralToken` at its authored span. It must be **authoring-fatal** (`ExpressionError::is_authoring_fatal`, `expression/error.rs:483`) so `Lenient` (preflight, `compose_subtree`) cannot swallow it. | y, if the error is authoring-fatal |
| 3 | `interpolate_value_located` whole-value branch | `rewrite.rs:376-381`, `lint.rs:100` `whole_value_span` | `whole_value_span` trims, so `" {{!data:…}}"` also counts as a whole value, which is then parsed and fails. | Decode a whole leaf **before** this function runs, in pass 1, on an exact match with no trim. After that, any token that reaches here is malformed (row 2). | y |
| 4 | `is_whole_value_span` (public) | `lint.rs:92` | `true` for a token. | None for the lexer. Its callers are rows 13, 15, and 18. | y (through row 1) |
| 5 | Nested-span lint inside string literals | `lint.rs:227`, `:348` | Flags `{{!data:…}}` inside `"…"` as a nested span. At runtime, the fixed-point rescan evaluates the string, finds the token, and hits the same parse error. | N5 says "inside an expression" is malformed. Report row 1's `tokens` from here as `MalformedLiteralToken`, not as a nested span. | y, with a small consumer change |
| 6 | `convert_literals` / `{{{ }}}` | `rewrite.rs:89`, `:305`; `frontmatter_interpolation.rs:469` | `{{{!data:v1:aGk}}}` is a literal and becomes the text `{{!data:v1:aGk}}`. In frontmatter, pass 2 then rescans it and fails (see E6). In the body it is left untouched (E7). | Nothing for S3. Triple braces are the author's escape and must win over the token. Phase 2's single pass makes the converted text data, so it is never rescanned. | y (literal branch runs first) |
| 7 | Frontmatter pass 1 shell deferral | `frontmatter_interpolation.rs:714` | Not a `$(` value, so it is not deferred. | A whole-leaf token is decoded here. The decoded value must be a **Data** leaf, and the `$(` predicate must read the authored source (N4). | **n**: needs N2/N4 |
| 8 | Preflight best-effort interpolation | `preflight/collect.rs:693` → `frontmatter_interpolation.rs:663` | Lenient, so the parse error is dropped and the token stays raw. | Same decode as pass 1, and row 2's fatal error. | y (rows 2 and 3) |
| 9 | `detect_dynamic_frontmatter_command_shape` | `collect.rs:841` (root heuristic `:858`) | `d: "$(echo {{!data:v1:aGk}})"` gives the misleading "depends on frontmatter key 'data'" (E8). | Row 1's `tokens` are not expressions. Row 2 fails first with a located error. | y |
| 10 | `pending_shell_literals`, `scan_frontmatter`, `parse_shell_value`, `extract_original_inner` | `collect.rs:883`; `frontmatter_shell_expansion.rs:1245`, `:523`, `:673` | Not reached for a raw token (no `$(`). **After decoding**, a `$(echo X)` payload **is** a shell candidate. `scan_frontmatter` tests the resolved `s`, and `extract_original_inner` ignores a snapshot (`{{!data:…}}`) that does not start with `$(`. | Skip Data leaves (N2), or test the authored snapshot (N4). | **n** |
| 11 | `mask_interpolations` | `frontmatter_shell_expansion.rs:1048` | Masks `{{!data:…}}` inside `$(…)` to `__DM_INTERP_n__`. This is shape-only, so harmless. | None. Row 2 fails pass 1 before shell expansion. | y (not reached) |
| 12 | Executable-interpolation guards | `frontmatter_shell_expansion.rs:877`, `:992`, `:1174` | `contains("{{")` makes a token in executable position a "command". | None. It is unreachable after row 2. | y |
| 13 | YAML fallback placeholder protection | `markdown/frontmatter.rs:634` (`$(`), `:695` (`{{`), called at `:572-575` | Runs only if the primary YAML parse fails. For an unquoted `t: {{!data:v1:aGk}}` it swaps in `__DM_EXPR_0__` and restores the **exact bytes** (E4). Double-quoted tokens (N5) never get here. | None. The round trip is byte-transparent, and the restored leaf is decoded like any other. | y (not a classifier) |
| 14 | Schema "pending" classifiers | `schemas/format.rs:265`, `schemas/mod.rs:1565`, `compose/schema_validation.rs:1253`, `schemas/rewrite.rs:374`; DMLS `diagnostics/frontmatter.rs:655` | `contains("{{")`, so a **raw** token is "pending" and validation is deferred. On passive paths (`md schema validate`, DMLS) an agent-owned value is never validated. | Passive paths call `decode_literal_tokens` (N6) first. A decoded Data value containing `{{`/`$(` must not count as pending (N2). | **n** |
| 15 | DMLS `interpolations` / `frontmatter_interpolations` | `dmls/src/overlay/expressions.rs:70`, `:352` (`:362`, `:365`, `:385`) | Through `ExpressionFinder`. The body gives the `EXPRESSION_MALFORMED` warning (`providers/dsl.rs:716-733`). Semantic tokens (`semantic_tokens.rs:321`), hover (`dsl.rs:397`), and the graph (`graph/substrate.rs:400`) treat it as an expression. | Consume row 1's `tokens`: no diagnostic for a whole-leaf frontmatter token, `MalformedLiteralToken` elsewhere. Decoded display is out of scope (N13). | y (source code read only; DMLS not run) |
| 16 | Claudine `reject_surviving_spans(_deep)` | `claudine/lib/src/composition/lifecycle/executor.rs:2050`, `:2067` | A raw token read from another document through `frontmatter(…)` is a "SurvivingSpan" error. | **Hazard:** with row 1 alone, a raw token is no longer a span and **base64 passes through silently**. Readers must decode (N6/I2), and N10 moves this guard to authored text. | **n** |
| 17 | Claudine lifecycle re-resolve gates | `executor.rs:1034` (`resolve_emit`), `:1314` (`render_message`), `:1875` | `contains("{{")`, then Darkmatter interpolation, then a parse error. | N10 deletes `:1314`/`:1875`. `:1034` reaches row 2. | y for `:1034`. The rest need N10 |
| 18 | Claudine `sequence/grammar.rs` `whole_value_span` | `claudine/lib/src/composition/sequence/grammar.rs:108`, `:122` | Its **own** strip-prefix scanner. `sequence: "{{!data:…}}"` becomes `Expression("!data:v1:…")` and later fails to parse. | Decode through N6 before classifying, or reject the `{{!data:` prefix here. | **n** (separate scanner) |
| 19 | Claudine `dispatch/template.rs::interpolate` | `claudine/lib/src/dispatch/template.rs:443` (`:447`), `:549` | Parse fails, so the original text is kept (lenient by design; hook/speak templates, not frontmatter). | None. Row 1 keeps the pass-through. | y |
| 20 | Other Claudine `contains("{{")` gates | `composition/preflight.rs:391`, `sequence/preflight/mod.rs:899`, `sequence/task/mod.rs:1073`, `schema/mod.rs:926`, `looping/actions.rs:306`, `lifecycle/validate.rs:87` | Either send the value to Darkmatter interpolation (row 2) or treat it as "needs composition" / "unresolved template". | Interpolating callers are covered. `schema/mod.rs:926` and `task/mod.rs:1073` need decoded input (N6), like row 14. | Partly |

## Empirical evidence

Run with `/Users/ken/.cargo/bin/md compose <file>`, stdin `/dev/null`, fixtures in `/tmp/s3`.

| # | Input | Result |
|---|---|---|
| E1 | frontmatter `title: "{{!data:v1:aGk}}"` | exit 1. "The `title` frontmatter property failed to evaluate `!data:v1:aGk`: parse error: Expected end of expression, found ':' at position 5", line 2, column 9 |
| E2 | `title: "{{!data:v1:}}"` (empty) | the same error on `!data:v1:` |
| E3 | body `hello {{!data:v1:aGk}} there` | exit 1. "A document expression failed to evaluate", line 5, column 7 |
| E4 | unquoted `title: {{!data:v1:aGk}}` | the same parse error on `!data:v1:aGk`. The YAML fallback restored the bytes exactly, and the leaf still reached interpolation |
| E5 | `"{{ !data:v1:aGk }}"` (spaced) | the same parse error (the scanner trims) |
| E6 | frontmatter `t: "{{{!data:v1:aGk}}}"` | exit 1 with the same error: `convert_literals` produced `{{!data:…}}` and pass 2 rescanned it. The control `"{{{ nope_x }}}"` + body `{{ title }}` rendered as `body` plus an unknown-identifier warning for `nope_x`, which shows the same rescan today |
| E7 | body `{{{!data:v1:aGk}}}` and `{{{ x }}}` | exit 0, output keeps the triple braces verbatim |
| E8 | `d: "$(echo {{!data:v1:aGk}})"` (quoted or not) | preflight: "dynamic command shape … depends on frontmatter key 'data'". This is misattributed: the heuristic at `collect.rs:858` reads `data` as a root |
| E9 | body `::shell echo {{!data:v1:aGk}}` | no token error. Approval requested for the raw `echo {{!data:v1:aGk}}` (lenient preflight kept the text) |
| E10 | token in a fenced block, and `\{{!data:…}}` | exit 0, both verbatim. An inline code span `` `{{!data:…}}` `` fails (inline code is scanned by design) |
| E11 | `X{{!data:v1:aGk}}}Y` (adjacent `}`) | parse error on `!data:v1:aGk`: the scanner closed at the first `}}` and left the third `}` as text |
| E12 | `{{ "x {{!data:v1:aGk}}" }}` | string evaluated, then the rescan hit the token: the same parse error |
| E13 | `--set '{"t":"{{!data:v1:aGk}}"}'` | the same frontmatter parse error. Authored setters are templates (N1) |

## Alphabet and brace boundaries

- base64url (`A-Z a-z 0-9 - _`, unpadded) contains no `{`, `}`, `$`, `(`, `)`,
  `"`, `\`, `:`, `#`, or whitespace. The payload cannot raise the legacy
  scanner's depth, cannot close the span early, cannot open `$(`, and needs no
  YAML escaping inside the double-quoted scalar. `-` and `_` only matter to the
  expression lexer, which never sees the payload once row 1 classifies it.
- The token always ends at the **first** `}}` after the prefix. A following `}`
  (E11) is plain text, so `{{!data:…}}}` is not ambiguous.
- `{{{!data:…}}}` is ambiguous only to a raw substring check: `contains("{{!data:")`
  matches at offset 1. The scanner reads it as a literal first, and that must
  stay the rule so authors and docs can write the token spelling literally. A
  `{{{{!data:…}}}}` opener is a normal nested `{{` span and fails to parse. That
  stays an error.
- The whole-leaf test is `s.starts_with("{{!data:") && s.ends_with("}}")` with
  **no trim**, and nothing else in between. Do not reuse `whole_value_span`,
  because it trims. `" {{!data:…}}"` is mixed text and malformed (N5).
- A token in a fenced or indented code block, or behind `\`, is not scanned, so
  it is not an error (E10). N5's "anywhere else" should read "anywhere the
  scanner would otherwise find an expression".

## Conclusion

N5's prefix check is **sufficient for the expression side**, as long as it
lives in these two places:

1. **`ExpressionFinder::scan`** (`lexer.rs:133`): classify an unescaped,
   non-code, two-brace opener followed by `!data:` as a token (a new `tokens`
   list), never an expression. Every row marked y reads this scanner.
2. **`interpolate_text_located` / `interpolate_value_located`** (`rewrite.rs`),
   plus the nested-span lint (`lint.rs:227`): turn any token they are handed
   into a located `MalformedLiteralToken` that is authoring-fatal under both
   policies. Whole-leaf decoding happens before them, in pass 1, and in the
   preflight best-effort pass that shares its code.

Checking in `parse()` instead would be weaker. It loses the span, it cannot
tell `{{{…}}}` apart, and `Lenient` callers (rows 8 and 19, E8, E9) swallow it.

**Not covered by a single prefix check.** These need N2/N4/N6/N10 instead:

- **Decoded values as shell candidates** (rows 7 and 10). `scan_frontmatter`
  and `pending_shell_literals` test the *resolved* string for `$(`, so a decoded
  `$(echo X)` would execute. This is the Phase 3 acceptance test and depends on
  the Data set or N4's authored-source predicate.
- **"Pending" classifiers** (rows 14 and 20: `format.rs:265`,
  `schemas/mod.rs:1565`, `schema_validation.rs:1253`, `schemas/rewrite.rs:374`,
  Claudine `schema/mod.rs:926`). A raw token skips validation, and a decoded
  value containing `{{` would too. Passive readers must decode, and Data leaves
  must not count as pending.
- **Claudine `reject_surviving_spans`** (row 16). Once row 1 exists, it stops
  catching raw tokens, and undecoded base64 would reach the agent. Every I2
  reader must use `decode_literal_tokens`.
- **Claudine `sequence/grammar.rs:122`** (row 18). It is its own scanner, so it
  needs decoded input or its own prefix rejection.
