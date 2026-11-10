# SP1: Quote-aware `{{ }}` scanner corpus diff

Spike for R11 of `2026-11-04-loading-data`, run 2026-11-05.

## Verdict

**R11 as written is not safe. A tightened version is.**

- **R11 as written** (variant A below): skip quoted strings, and fall back to
  today's scan only when no `}}` is found. This **regresses real prose**.
  An apostrophe or a quoted `{{` in inline code opens a "string" that can run
  for hundreds of lines and swallow valid expressions. One corpus file loses
  3 valid expressions into a 411-line span. A one-line adversarial case
  (`{{ don't }} it's fine {{ x }} and more }}`) regresses the same way. The
  no-closer fallback does not catch either case, because a `}}` is eventually
  found.
- **Recommended** (variant G below): quote-aware **and** single-brace balanced,
  with a **parse gate**. Use the new span only when it differs from today's
  span **and** the new span's text parses as an expression. Otherwise use
  today's span. Over the whole repository corpus this variant changes only
  spans that fail to parse today, and every changed span now parses. It also
  fixes `{{ {a: {b: 1}} }}`, which quote-awareness alone cannot fix because
  that `}}` is not inside a string.

## Method

The spike was a throwaway standalone binary, now deleted. It depended on
`simplified-schema` by path with default features off, and held a verbatim copy
of `scan` plus `scan_legacy_expression` ("old"). For every file it checked that
old matched the library's `scan_plain`. There were 0 mismatches, so the copy is
faithful.

- **Corpus:** every `.md`, `.markdown`, `.mdx`, `.yaml`, and `.yml` file from
  `git ls-files -co --exclude-standard`. That is 6,909 files: all of `prompts/`,
  `darkmatter/prompts/`, `claudine/prompts/`, every `tests/fixtures/`, every
  `docs/`, every spec, and agent and skill files. This is a superset of what
  the passive shipped-artifact corpus test reads.
- **What was compared:** each whole file was scanned with `scan_plain`, with
  no code-block exclusion. Hosts exclude fenced and indented code, so a
  diff inside a fence has lower impact. Each diff line was tagged `fence=`, and
  each span was tagged with whether `expression::parse` accepts it.
- **Baseline:** 1,064 files contain at least one expression, with 10,399
  expressions in total.

### Variants

All variants keep today's handling of `\{{` escapes, `{{{ … }}}` literals, and
`{{!data:…}}` tokens. Only `scan_legacy_expression` changes.

| Variant | Rule | Files that differ | Regressions |
|---|---|---|---|
| A (R11 as written) | Skip `'…'` and `"…"` strings (with `\` escapes); fall back to old when there is no closing `}}` or a string reaches EOF | 8 | **yes** (2 files) |
| B | A, and a newline inside a string counts as unterminated (falls back) | 6 | **yes** (adversarial one-line case; corpus diff swallows text in the spec) |
| C | A, and use the new span only if it differs from old **and** `parse` accepts it | 6 | none |
| D | B with C's parse gate | 6 | none |
| E | B, and `}}` closes only when every single `{` opened in the expression is closed | 11 | none lost, but garbage spans grow |
| F | E with the parse gate | 6 | none |
| **G (recommended)** | E without B's newline rule (multi-line strings allowed, as the lexer allows), with the parse gate | 6 | **none** |

## Results

### Corpus diffs

Variants C, D, F, and G produce the same file-level diff: only the six files
below, and only improvements.

| File:line | Old span | New span (G) | Judgment |
|---|---|---|---|
| `.claude/agents/just-scripter.md:1081` (prose) | `"` (does not parse) | `"{{"` (parses) | fix |
| `.claude/agents/just-scripter.md:1085` (fenced) | `"{{" }}LOVE` (does not parse) | `"{{"` (parses) | fix |
| same two lines in `.codex/`, `.gemini/`, `.opencode/`, `.qwen/agents/just-scripter.md` (mirrors) | as above | as above | fix |
| `darkmatter/features/2026-11-04-loading-data/spec.md:701` (fenced; the R11 example) | `load_data(f, "{ a: { b: string` | `load_data(f, "{ a: { b: string }}")` | fix |
| `spec.md:727` (prose; the R11 acceptance examples in inline code) | `"` and a garbage span `'{{' }}`… | `"}}"`, `'{{'` (both parse), then garbage spans at 727–728 and 729 | fix; garbage stays garbage and gets shorter |

Diffs that only variant A, B, or E produce (the reason not to use them):

| File:line | Variant | What happens | Judgment |
|---|---|---|---|
| `claudine/fixes/_completed/2026-09-27-agent-text-is-data/implementation-log.md:273` | A | `` `contains("{{")` `` in prose: the `"` after `{{` opens a string that ends many lines later. The new span runs L273–L684 and swallows valid `title` (L297), `b * 2`, and `c + 1` (L323) | **regression** |
| `claudine/fixes/_completed/2026-09-27-agent-text-is-data/spike-s3-token-lexer.md:34` | A | span L34–L69 swallows `title` (L53) and `"x {{!data:v1:aGk}}"` (L59) | **regression** |
| `just-scripter.md:1064` (all five mirrors) | A, B, E | garbage span moves from inside the fence (L1068) to prose (L1064–1071) | neutral (garbage to garbage), but now outside the fence |
| `spec.md:727` | A, B, E | garbage span grows to L727–L755 | neutral (garbage to garbage), but larger |
| `darkmatter/docs/inline/interpolation.md:251` | E | garbage span grows by one line | neutral |
| `docs/shell-completions.md:45,54,63`; `schematic/docs/research/postman.md` (4 sites); `schematic/features/_completed/ergonomics-and-postman-projects/review-plan-3.md:78,142`; `.claude/skills/resvg/modification.md:49` | E | `${tag` becomes `${tag}`, and `{{{}}` becomes `{{{}}}` | neutral (garbage to garbage; mostly fenced) |

No file under `prompts/`, `darkmatter/prompts/`, `claudine/prompts/`, any
`tests/fixtures/`, or any `docs/` changes under C, D, F, or G.

### Targeted cases

Each list shows the expressions found.

| Input | Old | A / B | G |
|---|---|---|---|
| `{{ load_data(f, "{a: {b: string}}") }}` | `load_data(f, "{a: {b: string` | whole call | whole call |
| `{{ load_data(f, "{ a: { b: string }}") }}` | truncated | whole call | whole call |
| `{{ {a: {b: 1}} }}` | `{a: {b: 1` | `{a: {b: 1` (**not fixed**) | `{a: {b: 1}}` |
| `{{ "}}" }}` | `"` | `"}}"` | `"}}"` |
| `{{ '{{' }}` | `'` | `'{{'` | `'{{'` |
| `{{ "a\"}}" }}` | `"a\"` | `"a\"}}"` | `"a\"}}"` |
| `{{ 'it\'s }}' }}` | `'it\'s` | `'it\'s }}'` | `'it\'s }}'` |
| `{{ "a\\" }} b` (escaped backslash ends the string) | `"a\\"` | same | same |
| `{{ f("multi⏎line }}") }}` | truncated | A: whole; B: truncated | whole |
| `{{ "open and no close` | none | none | none |
| `{{ "open }} rest` | `"open` | `"open` (fallback) | `"open` |
| `{{ "open }} then {{ y }} and " more }}` | `"open`, `y` | **one span to the last `}}`** | `"open`, `y` |
| `{{ don't }} and later 'quoted' text {{ x }}` | `don't`, `x` | same (fallback, by luck of quote parity) | same |
| `{{ don't }} it's fine {{ x }} and more }}` | `don't`, `x` | **`don't }} it's fine {{ x }} and more`** | `don't`, `x` |
| `Ken's notes: {{ name }} isn't here`, `{{ x }}'s value` | `name` / `x` | same | same |
| `{{ a {{ b }} c }}` | `a {{ b }} c` | same | same |
| `{{{ x }}` (unclosed triple) | `{ x` | same | same |
| `{{ a { }} b` (stray `{`) | `a {` | same | same (balanced scan finds no close, so it falls back) |
| `{{{VAR}}}`, `{{{ "}}}" }}}` | literal | literal | literal (the literal scanner is unchanged) |

Today `{{ "open }} then {{ y }} and " more }}` and the `don't` case each hold
one malformed and one valid expression. A and B merge them into a single
malformed one. The parse gate prevents this.

## Escape rules (lexer `read_string`)

`simplified-schema/lib/src/expression/lexer.rs:444` (`read_string`, called
at `:339` for `'"' | '\''`):

- `"` and `'` are the only string delimiters. There are no backtick or
  triple-quoted strings.
- A string closes at the next unescaped matching quote. The other quote kind is
  ordinary text inside it.
- `\` always consumes the next character. `\n`, `\t`, `\r`, `\\`, and
  `\<quote>` are decoded, and any other `\x` is kept as both characters. So
  for scanning, "backslash skips the next character" is exact.
- A backslash at EOF, or a string with no closing quote, is a lexer error.
- **Newlines are allowed inside a string.**
- Any quote outside a string starts a string. Identifiers
  (`is_identifier_start`: alphabetic or `_`) never contain quotes, so `don't`
  lexes as `don`, then a string starting at `'`.

The spike's scanner skips 2 bytes on `\`. A UTF-8 continuation byte is never
`"`, `'`, `\`, `{`, or `}`, so a multi-byte character after the backslash is
safe. The scanner therefore agrees exactly with the lexer on where every
string starts and ends. The only gap is by design: an unterminated string
makes the scan fall back.

## Other `{{` / `}}` detectors

Each detector is classified against a G-style scanner. A presence check such
as `contains("{{")` decides only whether text may hold an expression, not
where one ends, so it is unaffected (this covers about 40 call sites across
darkmatter, claudine, content-policy, biscuit-file, and simplified-schema
`pending.rs:168`).

| Location | What it does | Must change? |
|---|---|---|
| `darkmatter/dmls/src/overlay/expressions.rs:568-576` `completion_partial` | `rfind("{{")`, then "any `}}` between it and the cursor closes the region" | **Yes.** With the cursor after `load_data(f, "{a: {b: string}}") `, the `}}` inside the string makes it return `None`, so there is no completion. A `'{{'` string earlier in the expression also moves `open`. Derive the open region from the shared scanner (for example, a quote- and brace-aware "is offset inside an unclosed opener" helper exported from `scan.rs`) |
| `darkmatter/dmls/src/providers/dsl.rs:226` | `prefix.rfind("{{")` picks the start of `cursor_scope` text | **Should.** A quoted `{{` earlier in the same expression makes the scope start inside the string. Use the scanner's `ExpressionLocation.start` |
| `darkmatter/dmls/src/providers/frontmatter.rs:1068` | the same `rfind("{{")` for frontmatter completion (same line only) | **Should**, same fix as `dsl.rs:226` |
| `darkmatter/lib/src/markdown/frontmatter.rs:589-625` `protect_interpolation_expressions` | YAML-fallback placeholder masking. It is a second copy of the legacy depth scan, and runs only when the primary YAML parse fails | **Should.** It restores bytes exactly, so the result is still correct today (the leftover `") }}` stays in a plain scalar). But it is a second implementation of the closing rule; replace it with `scan_plain` spans so one rule governs |
| `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:1068-1088` `mask_interpolations` | masks `{{…}}` with `__DM_INTERP_n__` before shell tokenizing. It closes at the **first** `}}`, with no nesting | **Yes.** With `$(cmd {{ f("}}") }})`, the leftover `") }}` puts a `"` into the shell quote tracker and misreports the command shape. Use `scan_plain` spans |
| `claudine/lib/src/composition/sequence/grammar.rs:119,146-153` `whole_value_span` | `sequence: "{{ … }}"` is an expression only if the inner text contains no `}}` | **Yes.** `sequence: "{{ load_data(f, \"{a: {b: string}}\") }}"` is misread as a file reference. Use: exactly one scanner expression spanning the whole trimmed value |
| `claudine/lib/src/composition/sequence/grammar.rs:346-385` `SpanScanner` | finds `::` and `->` outside spans. It is already quote-aware, but has no backslash escapes, and quotes toggle outside `{{` as well | **Optional.** It diverges only on `\"` inside an interpolation; adopt the same escape rule if touched |
| `darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs:296-314` `hole` | approval-rule template holes; already quote-aware without escapes | No. Holes hold type names, not expressions |
| `darkmatter/lib/src/markdown/compose/link_normalization.rs:93-96` `compose_spelling` | rewrites a leading `{{VAR}}` env anchor to `{{{VAR}}}` | No. `VAR` matches `[A-Z0-9_]+` and holds no quotes |
| `biscuit-file/lib/src/file_reference/parse.rs:313-330` `parse_template` | `{{VAR}}` env segments in file references | No. Same `[A-Z0-9_]+` names |
| `simplified-schema/lib/src/expression/scan.rs:121-131` (`{{{` literal) and `:166` (`{{!data:` token) | literal and token closers | No. A literal is verbatim by definition (`docs/inline/interpolation.md` already says content cannot hold `}}}`), and the token alphabet holds no quotes or braces |
| `darkmatter/lib/src/markdown/compose/expression/lexer.rs:60` `ExpressionFinder` and DMLS `overlay/expressions.rs:70,352` | wrap `scan` with code-region exclusion | No; they inherit the change |
| `claudine/lib/src/dispatch/template.rs:570-590` | legacy `{key}` placeholder rewrite | No; this is unrelated syntax |

## Recommendations for the spec

1. **Rewrite R11's rule.** Proposed wording:

   > While looking for the closing `}}`, the scanner skips single- and
   > double-quoted strings using the expression lexer's escape rule (`\`
   > consumes the next character; strings may span lines), and a `}}` closes
   > the expression only when every single `{` opened inside it is closed.
   > The scanner uses this span only when it differs from today's span **and**
   > the expression parses; otherwise (no closing `}}`, an unterminated string,
   > an unbalanced `{`, or a parse failure) it uses today's span unchanged.

   Then drop the claim that "text that scans today never stops scanning", which
   is not true of the no-closer fallback alone. Replace it with: *no span that
   parses today changes, and a changed span always parses.*
2. **Add acceptance criteria** for the cases this spike found:
   - `{{ {a: {b: 1}} }}` yields the object `{a: {b: 1}}`.
   - `{{ don't }} it's fine {{ x }} and more }}` finds `don't` and `x` exactly
     as today, and so does `{{ "open }} then {{ y }} and " more }}`.
   - `` `contains("{{")` `` in prose, followed by valid expressions further
     down, leaves those expressions found.
   - `{{ f("multi⏎line }}") }}` yields the whole call.
3. **Bring the detectors into scope.** Fix `completion_partial`,
   `mask_interpolations`, and claudine's `whole_value_span` (must), and the two
   DMLS `rfind("{{")` sites and `protect_interpolation_expressions` (should).
   Do this by routing them through the shared scanner, not by adding a second
   quote rule. The R11 line "applies to every surface that finds expressions"
   is not true today, because these sites do not call the scanner. Add a
   claudine criterion: `sequence: "{{ load_data(f, \"{a: {b: string}}\") }}"`
   is an expression source.
4. **Corpus test.** Keep SP1's old-vs-new comparison as an L1 test over the
   shipped corpus. It asserts that every changed span parses and that the old
   span did not.
5. **Document.** The parse gate makes the scanner depend on
   `expression::parse`. The parser is in the same module, so no layering
   changes, but the module doc's "purely lexical" line in `scan.rs` must be
   updated. The parse runs only when spans differ (6 of 6,909 files), so the
   cost is negligible.
6. **Trade-off to record.** With the gate, an expression that has a real
   syntax error **and** a `}}` inside a string still truncates at that `}}`, so
   its error message describes the truncated text. That is today's behavior,
   so it is not a regression.
7. **Unchanged.** `{{{ … }}}` literals still close at the first `}}}` even
   inside quotes, and `{{{a: 1}}}` with no space is a literal, not an object
   expression. The R12 docs should say so next to the object-literal example.
