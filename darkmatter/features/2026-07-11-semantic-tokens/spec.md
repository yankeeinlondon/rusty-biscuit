---
status: ready for planning and implementation
reviewed: true
review_iterations: 4
inputs:
  - ../../dmls/src/capabilities.rs
  - ../../dmls/src/overlay/expressions.rs
  - ../../dmls/src/overlay/directives.rs
  - ../../dmls/src/providers/dsl.rs
  - ../../dmls/design/markdown-lsp.md
  - ../../dmls/design/zed-lsp.md
related:
  - ../_completed/2026-07-04-dmls/spec.md
---

# DMLS Semantic Tokens

**Status:** Ready for planning and implementation. Design section 5.3 of
[markdown-lsp.md](../../dmls/design/markdown-lsp.md) sketched semantic tokens
as a v2 candidate; this spec scopes a concrete v1. The five open questions
were ruled by Ken on 2026-07-11; the rulings are recorded at the end and
folded into the body.

> **Reader's note (spec review):** The review retained the ruled token model
> but tightened four implementation contracts that otherwise left observable
> behavior to accident: family precedence for overlapping constructs, range
> clipping, configuration refresh, and the interaction between wiki tokens and
> `wiki.enable`. It also excludes shell command payloads from F2 so the v1
> scope remains consistent with the explicit F6 deferral.

> **Revision note (2026-07-12, pending reapproval):** The Zed extension API
> cannot inject semantic-token colors — this is a platform constraint, not an
> implementation choice. The spec originally stated that styling defaults would
> ship inside the `zed-dmls` extension; the actual implementation delivers Zed
> styling as a documented copyable `experimental.theme_overrides` recipe
> (in `zed-dmls/README.md` and `dmls/docs/editors/zed.md`), matching the
> documentation-based approach already used for VS Code
> (`editor.semanticTokenColorCustomizations`) and Neovim (`@lsp.*` highlight
> links). This revision changes the accepted product behavior for Zed and is
> **pending Ken's reapproval**.

## Goal

Give editors enough classification to *de-emphasize Darkmatter machinery* so
the prose stands out:

1. **Interpolations** — the full `{{ … }}` span in the body renders dim/muted,
   present but not attention-grabbing.
2. **Directives** — the twelve `::` directives (`::file`, `::code`, `::url`,
   `::file-links`, `::toc-linking`, `::shell`, `::block`, `::shell-block`,
   `::end-block`, `::disclosure`, `::details`, `::end-disclosure`) get subtle,
   consistent highlighting instead of reading as stray prose.
3. **Wiki links** (pulled into v1 by ruling #4) — `[[wiki]]` links read as
   *links* rather than bracketed prose. Inverse emphasis to the first two:
   not de-emphasis but distinction.

Standard Markdown grammars (TextMate and Tree-sitter alike) know nothing about
Darkmatter's DSL, so today these constructs render as plain paragraph text.
Semantic tokens are the only LSP mechanism that lets the *server* influence
styling, and they work across every semantic-token-capable client without
per-editor grammar forks.

## The one constraint that shapes everything

**LSP semantic tokens classify; they never style.** The server says "this span
is a `macro` with modifier `interpolation`"; the *theme* decides what that
looks like. "Dim" is therefore not something DMLS can emit — it is something
DMLS makes *targetable*, and then we provide documented per-editor styling
recipes: VS Code `editor.semanticTokenColorCustomizations` snippets, Neovim
`@lsp.*` highlight links, and Zed `experimental.theme_overrides` recipes.
None of these editors allow DMLS to inject styling defaults via extension or
LSP; all three rely on user-copyable configuration documented in
`dmls/docs/editors/`.

Consequences:

- Token types/modifiers must be chosen so that (a) unaware clients still do
  something sensible, and (b) aware clients/themes can target Darkmatter
  constructs specifically.
- Helix's current LSP client does not advertise semantic-token support; it is
  naturally excluded by capability gating and loses nothing it has today.
- Zed ships semantic tokens **disabled by default**; users must opt in
  (`"semantic_tokens": "combined"` or `"full"`). The
  [zed-lsp design notes](../../dmls/design/zed-lsp.md) anticipated the
  extension carrying semantic token styling defaults, but the Zed extension API
  does not permit injecting semantic-token colors. The implementation delivers
  a documented `experimental.theme_overrides` recipe instead (see the styling
  table below).

## Legend

### Strategy: standard base types + custom modifiers

Custom token *types* are legal in LSP but render as nothing in clients without
an explicit mapping. Custom *modifiers* on standard types degrade gracefully:
an unaware client styles the token by its base type; an aware theme targets
the modifier. This is the recommended shape.

### Token types (all standard)

| Type | Used for |
|------|----------|
| `macro` | interpolation spans (including `{{{ }}}` literals), all directive keywords |
| `function` | expression function names inside `{{ }}` (fine-grained phase) |
| `variable` | expression identifiers: frontmatter keys, `ctx.*`, `env.*` (fine-grained phase) |
| `property` | directive option keys (`title=`, `disclosure=`, `when=`) |
| `string` | directive option values, expression string literals, wiki-link inner path |
| `number` | expression number literals |
| `operator` | expression operators (`??`, `?:`, comparison, arithmetic) |

Ruling #2 removed `comment` from the legend: `{{{ }}}` literals stay in the
interpolation family (`macro`) with the `inert` modifier, so one theme rule
dims the whole family and comment-aware tooling (spell-check, TODO
highlighters) never sees a false comment. Ruling #3 removed `keyword`:
closers are uniform `macro` with the `closer` modifier.

### Token modifiers (custom, the targeting surface)

| Modifier | Used on |
|----------|---------|
| `interpolation` | every token inside (and including) a `{{ }}` or `{{{ }}}` span |
| `inert` | `{{{ … }}}` literal spans (ruling #2 — same family, distinguishable) |
| `directive` | every token on a directive line |
| `closer` | `::end-block`, `::details`, `::end-disclosure` (ruling #3 — uniform color by default; themes opt into fading closers harder) |
| `wiki` | every token in a `[[wiki]]` link (the F4 family targeting surface) |
| `injected` | reserved: transcluded-target paths (future) |
| `defaultLibrary` (standard) | `ctx.*` / `env.*` roots and expression functions — "Darkmatter-owned, not yours" |
| `readonly` (standard) | `ctx.*` (mirrors the hover's read-only framing) |

So the whole-span interpolation token is `macro.interpolation`, an inert
literal is `macro.interpolation.inert`, a directive keyword is
`macro.directive`, a closer is `macro.directive.closer`, and a `ctx.today`
inside an interpolation is (fine-grained phase)
`variable.interpolation.defaultLibrary.readonly`.

## Token families

Each family is independently emittable. V1 ships F1 + F2 + F4 (ruling #4
pulled wiki links forward); the rest are enumerated so the legend is designed
once and never breaks compatibility (legend changes invalidate client caches
and are effectively a protocol migration).

### F1 — Body interpolations (v1)

- One token per `{{ … }}` occurrence covering the **entire outer span**
  (delimiters included, ruling #1): `macro` + `interpolation`. Whole-span
  matches the goal — the unit reads as "a placeholder lives here" — and the
  fine-grained phase (F3) refines rather than reverses it.
- Source: `overlay::expressions::interpolations(text, body_base)` — the same
  scanner the diagnostics pipeline already runs on every keystroke; spans are
  byte-accurate and code-fence-aware.
- `{{{ … }}}` literals: one token per occurrence over the outer span,
  `macro` + `interpolation` + `inert` (ruling #2; source:
  `overlay::expressions::literals`).
- Unclosed or otherwise malformed interpolations emit no token. Diagnostics
  own error reporting; guessing an end span would make token output flicker
  across unrelated text while the user is typing.

### F2 — Directive lines (v1)

- Directive keyword (`::file` … including the leading `::`): `macro` +
  `directive`. The three structural closers (`::end-block`, `::details`,
  `::end-disclosure`) additionally carry `closer` (ruling #3): identical
  color out of the box, and a theme can opt into fading closers harder.
- Structured target spans (the `./intro.md` in `::file ./intro.md`): `string`
  + `directive`. The free-form payloads of `::shell` and `::disclosure` remain
  untokenized: shell text is deferred to F6, while disclosure summary text is
  prose.
- Option keys / values (`disclosure="Summary"`, `when="…"`): `property` /
  `string`, both + `directive`.
- Source: `compose::directives_api::scan_darkmatter_directives` — the
  `ParsedDirective` product already carries keyword/target/per-option
  key/value spans. Unknown directives get **no** token (the
  `dm.directive.unknown` diagnostic already flags them; styling them as valid
  would contradict it).
- Disclosure openers (`::disclosure max-width=60ch License`): keyword plus
  recognized inline style tokens from `scan_disclosures`; each style token is
  split at its source `=` into `property.directive` and `string.directive`.
  This requires either exposing those subspans from the library scanner or a
  span-preserving lexical split in the provider. Summary text stays
  untokenized (it is prose).

### F3 — Fine-grained expression tokens (v2)

Replaces F1's whole-span token with a token stream *inside* the braces:
delimiters `macro.interpolation`, identifiers `variable.interpolation`
(+ `defaultLibrary`/`readonly` for `ctx.*`/`env.*`), function names
`function.interpolation.defaultLibrary`, literals `string`/`number`,
operators `operator`. Source: `parse_spanned` / `lex_spanned` — the spanned
AST exists precisely for this. **Overlap rule:** semantic tokens must not
overlap (most clients ignore overlapping tokens), so F3 *replaces* F1 for a
given interpolation rather than layering on it. The "everything inside is
dim-targetable" property survives because every F3 token still carries the
`interpolation` modifier.

### F4 — Wiki links (v1, ruling #4)

`[[wiki]]` is invisible to standard Markdown grammars — tied with F1 for the
most-invisible construct, and the scanner already exists (it powers wiki
navigation/diagnostics), so the marginal cost of shipping it first is one
family function. Tokens: brackets (and the `|` / `#` separators) `macro` +
`wiki`; the path/heading/alias segments `string` + `wiki`. Unlike F1/F2 the
goal is *distinction*, not dimming — the styling defaults render the inner
text link-like and only the brackets muted. Source: the `dmls::wiki` lexical
scanner. Resolved vs. unresolved must **not** be encoded as different tokens
(diagnostics own correctness signaling; tokens own structure).

The existing scanner does not expose alias or separator spans. P1 therefore
extends `ScannedWikiLink` with source spans for the alias and present `#` / `|`
separators (or an equivalent structured segment list); the semantic-token
provider must not reconstruct spans from unescaped values. Empty segments
produce no zero-length token. Supported and currently unsupported wiki forms
use the same lexical token structure when the scanner recognizes them; the
existing diagnostic remains the only validity signal.

### F5 — Frontmatter (deliberately minimal, v2+)

Editors already YAML-highlight frontmatter via language injection, and
semantic tokens *merge over* grammar tokens in VS Code — easy to fight the
theme here. Candidates worth having, nothing more: `$schema` key and `ctx.*`
keys as `property.defaultLibrary`, whole-value `$(…)` shell strings as
`macro`. Everything else stays with the YAML grammar.

### F6 — `::shell` / `::shell-block` command text (deferred)

Could token as `string.directive`; embedded-shell injection is a client
grammar concern, not ours. Deferred until someone misses it.

## Protocol surface

- **Endpoints:** `textDocument/semanticTokens/full` and
  `…/range` in v1. **Delta (`…/full/delta`) is deferred** — documents are
  Markdown-sized, the scanners are already per-keystroke cheap, and delta adds
  a result-id cache with real invalidation complexity for negligible win at
  this scale (the R-6 bench found full-repo indexing at ~1.9 s; a single
  document's token scan is microseconds).
- **Capability gating:** advertise `semantic_tokens_provider` (legend + full +
  range) only when the client sent `text_document.semantic_tokens`. New
  `ClientProfile.supports_semantic_tokens` following the existing gate
  pattern (`supports_folding` et al.).
- **Positions:** all spans flow through the existing `SourceMap`
  (UTF-8/UTF-16 negotiation, CRLF/lone-CR). **Multi-line spans must be split
  into non-empty per-line tokens for every client**, even when the client
  advertises `multiline_token_support`. A single representation avoids
  client-specific output and makes CRLF handling unambiguous. F1 whole-span
  tokens can legitimately cross source lines, so the splitter is v1 scope.
- **Family precedence:** collect raw spans with an explicit priority of F1
  interpolation > F4 wiki > F2 directive. A higher-priority construct owns
  every byte it covers; lower-priority tokens are clipped around it, with
  empty fragments discarded. This makes `when="{{ enabled }}"` an
  interpolation token inside a directive rather than depending on sort
  stability. Within one family, a structural subtoken wins over a broader
  token (for example, wiki separators win over inner text).
- **Ordering:** after precedence resolution and line/range clipping, the
  encoder sorts by `(line, character, length, token type, modifiers)`, removes
  exact duplicates, and asserts that output is strictly ordered and
  non-overlapping. One owner enforces this invariant (see Architecture).
- **Range semantics:** generate the same canonical raw stream as `full`,
  intersect each token with the requested half-open range, then run the normal
  line splitter and encoder. Tokens crossing a range boundary are clipped;
  no token outside the requested range is returned.

## Styling: how "dim" actually lands per editor

| Editor | Mechanism | Ships where |
|--------|-----------|-------------|
| Zed | theme targets token types/modifiers via `experimental.theme_overrides`; semantic tokens require user opt-in (`"semantic_tokens": "combined"`) | documented `experimental.theme_overrides` recipe in `zed-dmls/README.md` + `docs/editors/zed.md` |
| VS Code | `editor.semanticTokenColorCustomizations` rules like `"*.interpolation": { "foreground": "#7d8590" }`; token colors cannot carry alpha, so "dim" = muted foreground per theme | documented snippet in `docs/editors/vscode.md` |
| Neovim (0.9+) | built-in maps to `@lsp.type.macro`, `@lsp.mod.interpolation`, … — e.g. `vim.api.nvim_set_hl(0, '@lsp.mod.interpolation.markdown', { link = 'Comment' })` | documented snippet in `docs/editors/neovim.md` |
| Helix | no semantic-token support; capability-gated off | note in `docs/editors/helix.md` |

The per-editor docs additions are part of this feature's acceptance, not an
afterthought — without them the feature is invisible everywhere except themes
that happen to style `macro` distinctly.

## Configuration

New `[semantic_tokens]` section in `.dmls.toml` / `workspace/configuration`,
following the `SymbolsConfig` precedent. **Master switch only** in v1
(ruling #5):

```toml
[semantic_tokens]
enable = true                 # master switch (default true; gating still wins)
```

Per-family toggles are deliberately deferred: config keys — unlike the token
legend — can be added later without breaking existing setups, and the first
family that genuinely needs its own switch (F5 frontmatter, default-off) ships
in a later phase and brings its toggle with it. Until then, a user who
dislikes one family's styling can neutralize it with a per-editor theme rule.

Emission requires both `semantic_tokens.enable` and the relevant domain
feature switch. In v1 this means F4 emits only when `wiki.enable` is also
true; disabling wiki behavior must not leave wiki-specific presentation
active.

`didChangeConfiguration` already re-publishes diagnostics. P1 adds
`ClientProfile.supports_semantic_tokens_refresh`, derived from
`workspace.semantic_tokens.refresh_support`, and sends
`workspace/semanticTokens/refresh` when token-affecting configuration changes.
When refresh is unsupported, the new configuration still applies to every
subsequent full/range request; no stronger immediate repaint guarantee is
possible in the protocol. Refresh failure is logged and must not fail the
configuration notification.

## Architecture

A **standalone provider module** (`providers::semantic_tokens`), not a
registry capability. Precedent: Phase 10's rename/code-action/formatting
providers stayed outside the registry because they have a single correct
answer; semantic tokens are the same — the sorted, non-overlapping, legend-
encoded token stream needs exactly one owner, and per-provider union merging
would push the overlap invariant into the merge policy. Internally the module
is one function per family (each unit-testable as `(text, …) → Vec<RawToken>`
with byte spans), a combiner that applies precedence, clips, sorts,
deduplicates, and splits multi-line spans, and one encoder to the LSP
relative-delta wire format.

Inputs are the existing passive surfaces only — `overlay::expressions`,
`scan_darkmatter_directives`, `scan_disclosures`, the wiki scanner,
`FrontmatterAst` — so the no-side-effects acceptance criterion
(`tests/no_side_effects.rs`) extends to the new request handler with zero new
analysis machinery.

## Phasing

| Phase | Scope |
|-------|-------|
| P1 | Legend, capability + `ClientProfile` gates (including refresh support), encoder (precedence/range clipping/sort/multi-line split, both position encodings), `full` + `range` handlers, F1 + F2 + F4, master-switch config, refresh wiring, L2 session tests |
| P2 | Per-editor styling recipes: all four editor docs (`docs/editors/{zed,vscode,neovim,helix}.md`) documenting copyable configuration snippets; Zed recipe in `zed-dmls/README.md`; manual smoke checklist entries |
| P3 | F3 fine-grained expressions (replaces F1 spans) |
| P4 | F5 frontmatter (default-off, brings the first per-family config toggle), `full/delta` if measurements show material cache or transfer pressure |

## Acceptance criteria (v1 = P1+P2)

1. A `{{ title }}` in the body yields exactly one `macro.interpolation` token
   spanning `{{` through `}}`; a `{{{ literal }}}` yields one
   `macro.interpolation.inert` token.
2. Every recognized directive line yields keyword/structured-target/option tokens per
   F2, with `closer` on exactly the three structural closers; an unknown
   `::frobnicate` yields none. Shell payloads and disclosure summary prose are
   not tokenized.
3. Every wiki link yields bracket/segment tokens per F4, identically for
   resolved and unresolved targets.
4. Tokens never overlap and are strictly (line, char)-ordered in both UTF-8
   and UTF-16 encodings; a multi-line interpolation is split per line for
   every client.
5. Interpolations, directives, and wiki links inside fenced code blocks yield
   no tokens (scanner parity with diagnostics).
6. A client that does not advertise `textDocument.semanticTokens` sees no
   `semantic_tokens_provider` capability.
7. `semantic_tokens.enable = false` suppresses emission without a restart and
   requests a refresh when supported; `wiki.enable = false` suppresses F4.
8. The no-side-effects harness passes with semantic-token requests included.
9. Range requests return exactly the full-document tokens intersected and
   clipped to the requested half-open range.
10. Overlapping constructs follow the documented family precedence in both
    full and range requests, including an interpolation inside a directive
    option value.

## Rulings (Ken, 2026-07-11)

All five open questions were ruled in a walkthrough on 2026-07-11 and are
folded into the body above.

1. **F1 dims the whole `{{ … }}` span** (braces and inner text as one unit).
   Delimiters-only would leave the inner text looking like prose — most of
   the problem unsolved. F3 later refines the inside without reversing this.
2. **`{{{ }}}` literals stay in the interpolation family** — `macro` +
   `interpolation` + `inert`, **not** `comment`. One theme rule dims the
   whole family the same shade; comment-aware tooling (spell-check, TODO
   highlighters) never sees a false comment. The instant-styling advantage of
   `comment` was moot because P2 ships documented per-editor styling recipes
   (copyable configuration snippets for VS Code, Neovim, and Zed) regardless.
3. **All twelve directives share one base type (`macro`); the three
   structural closers carry the `closer` modifier.** Identical color out of
   the box (no accidental two-tone from theme defaults), with a one-word
   frozen-legend insurance policy for themes that want to fade closers.
4. **Wiki links (F4) ship in v1.** The scanner exists; the marginal cost is
   one family function, and F4 is tied with F1 for the most-invisible
   construct. Deferring risked the follow-up phase drifting.
5. **Config is a master switch only.** Unlike the legend, config keys are
   backwards-compatibly extensible — granularity gets added when the first
   family that needs it (F5 frontmatter, default-off) ships, or when a user
   asks.
