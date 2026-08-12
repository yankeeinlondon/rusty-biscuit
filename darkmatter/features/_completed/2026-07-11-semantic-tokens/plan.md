---
agent: codex/
total_phases: 6
created: 2026-07-11
phase: 1
yolo: true
source_files_during_phase_1:
  - darkmatter/dmls/src/wiki/scanner.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/dmls/src/providers/semantic_tokens.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/source_map/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/dmls/src/providers/semantic_tokens.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - dmls
source_files_during_phase_4:
  - darkmatter/dmls/src/config/mod.rs
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/src/providers/semantic_tokens.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/providers/dsl.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - dmls
source_files_during_phase_5:
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/dmls/tests/no_side_effects.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - dmls
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - darkmatter/dmls/zed-dmls/README.md
  - darkmatter/dmls/docs/editors/zed.md
  - darkmatter/dmls/docs/editors/vscode.md
  - darkmatter/dmls/docs/editors/neovim.md
  - darkmatter/dmls/docs/editors/helix.md
  - darkmatter/dmls/docs/editors/smoke-checklist.md
  - darkmatter/dmls/docs/features.md
  - darkmatter/dmls/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/SKILL.md
packages_during_phase_6:
  - dmls
source_code:
  - darkmatter/dmls/src/wiki/scanner.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/dmls/src/providers/semantic_tokens.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/source_map/mod.rs
  - darkmatter/dmls/src/config/mod.rs
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/dmls/tests/no_side_effects.rs
documentation:
  - darkmatter/dmls/zed-dmls/README.md
  - darkmatter/dmls/docs/editors/zed.md
  - darkmatter/dmls/docs/editors/vscode.md
  - darkmatter/dmls/docs/editors/neovim.md
  - darkmatter/dmls/docs/editors/helix.md
  - darkmatter/dmls/docs/editors/smoke-checklist.md
  - darkmatter/dmls/docs/features.md
  - darkmatter/dmls/README.md
  - .claude/skills/darkmatter/SKILL.md
packages:
  - darkmatter
  - dmls
---

# Execution Plan: DMLS Semantic Tokens

## References and success criteria

- Functional specification: `darkmatter/features/2026-07-11-semantic-tokens/spec.md`
- Primary package: `darkmatter/dmls` (`dmls`)
- Supporting scanner package: `darkmatter/lib` (`darkmatter`)
- Editor integration: `darkmatter/dmls/zed-dmls` and `darkmatter/dmls/docs/editors`
- V1 is complete when F1 interpolations, F2 directives, and F4 wiki links are
  available through full and range semantic-token requests, obey capability and
  configuration gates, produce canonical non-overlapping tokens in UTF-8 and
  UTF-16, and have usable setup guidance for Zed, VS Code, Neovim, and Helix.
- F3 fine-grained expressions, F5 frontmatter tokens, F6 shell payloads, and
  full/delta are explicitly outside this plan.

## Phase 1 — Baseline, contracts, and scanner span fidelity

**Goal:** Establish a passing baseline and expose every source span required by
the three V1 token families without duplicating parsing in the provider.

**Validation checkpoint:** Existing suites remain green, and scanner unit tests
prove all semantic-token inputs have byte-accurate, code-fence-aware spans.

- [x] Record the pre-change baseline by running `just test` and `just lint` from
  `darkmatter/`, including the `darkmatter`, `darkmatter-cli`, and `dmls`
  results; preserve any pre-existing failures separately from feature work.
  <!-- Baseline: `just test` exit 0 (415 dmls + darkmatter/darkmatter-cli, all
  passing) and `just lint` exit 0. No pre-existing failures. -->
- [x] Inventory the current public fields and byte-offset conventions of
  `overlay::expressions::{interpolations,literals}`,
  `compose::directives_api::scan_darkmatter_directives`,
  `render_tree::disclosure_scan::scan_disclosures`, and
  `dmls::wiki::scan_wiki_links`; document any missing spans in test names or
  task notes before changing types.
  <!-- Inventory: `Interpolation{outer,inner}` / `Literal{outer}` are byte
  spans, code-fence-aware via `ExpressionFinder` (sufficient for F1). `ParsedDirective`
  carries `keyword_span`, `target: Option<Spanned<String>>`, and per-option
  `key`/`value` Spanned (sufficient for F2 keyword/target/option tokens);
  unknown directives are never surfaced. `DisclosureParse` exposes whole
  `key=value` `style_token_spans` (sufficient for a provider-local split at `=`).
  MISSING: `ScannedWikiLink` had no alias span and no `#`/`|` separator spans —
  added here as source-derived spans. -->
- [x] Extend `darkmatter/dmls/src/wiki/scanner.rs::ScannedWikiLink` with
  source-derived spans for the optional alias and present `#`/`|` separators
  (or an equivalent structured segment representation), update all consumers,
  and test escaped values, empty segments, embeds, headings, aliases, and
  recognized-but-invalid forms without reconstructing offsets from decoded
  strings.
  <!-- Added `hash_span`, `pipe_span`, `alias_span` (all `Option<SourceSpan>`,
  single-byte for separators). Consumers only read fields, so additive; only
  `parse_link` + `shift_spans` updated. -->
- [x] Extend the Darkmatter disclosure scanner product only if necessary so a
  `::disclosure` opener exposes exact recognized style key/value subspans;
  otherwise add a provider-local, span-preserving lexical split over the
  scanner's raw source span. Test that summary prose is excluded.
  <!-- NOT necessary: `DisclosureParse.style_token_spans` already gives each
  recognized `key=value` token's exact source span; a provider split at the
  byte offset of `=` within is lossless (Phase 3). Summary-exclusion already
  proven by `captures_opener_style_token_spans`. No scanner change. -->
- [x] Add or strengthen scanner parity tests proving interpolations, literals,
  directives, disclosures, and wiki links inside fenced code blocks are not
  surfaced, malformed/unclosed interpolations are absent, and unknown
  directives remain distinguishable from recognized directives.
  <!-- Added `interpolations_inside_fenced_code_are_not_surfaced`,
  `literals_inside_fenced_code_are_not_surfaced`,
  `unclosed_interpolation_yields_no_token` (overlay::expressions) and
  `unknown_directive_is_not_surfaced_but_recognized_one_is` (directives_api).
  Disclosure/wiki fence exclusion already covered by existing module tests. -->
- [x] Review rustdoc and inline comments on every changed scanner type and
  remove or correct any behavior drift discovered during the span changes.
  <!-- Updated `ScannedWikiLink` rustdoc to document the source-derivation
  invariant and the single-byte separator spans; no drift found elsewhere. -->
- [x] Run focused scanner tests with `cargo nextest run -p darkmatter` and
  `cargo nextest run -p dmls`, then rerun `just test` from `darkmatter/`.

## Phase 2 — Canonical semantic-token model and encoder

**Goal:** Build the single-owner token pipeline independently of LSP routing so
precedence, clipping, splitting, ordering, and encoding are deterministic.

**Depends on:** Phase 1.

**Validation checkpoint:** Table-driven unit tests demonstrate the same logical
stream for UTF-8 and UTF-16 mappings, with valid relative LSP encoding and no
overlap.

- [x] Create `darkmatter/dmls/src/providers/semantic_tokens.rs` and define the
  frozen V1 legend in protocol order: standard token types `macro`, `function`,
  `variable`, `property`, `string`, `number`, `operator`; custom modifiers
  `interpolation`, `inert`, `directive`, `closer`, `wiki`, `injected`, followed
  by standard `defaultLibrary` and `readonly` entries required by future F3.
  <!-- `TokenType` (repr(u32) discriminant == wire legend index) + `modifier`
  bit-mask module (bit n == legend modifier index n) + `legend()` emit the
  exact frozen order; `legend_is_frozen_in_protocol_order` /
  `modifier_bits_match_legend_indices` pin it. -->
- [x] Define a private `RawToken` carrying a byte span, token type, modifier
  bitset, family/structural priority, and enough provenance for useful test
  failures; keep LSP-relative encoding out of family scanners.
  <!-- `RawToken { span, token_type, modifiers, family, structural, provenance }`;
  `Family` (Ord: Interpolation > Wiki > Directive) + `structural: u8` compose
  `priority()`. Encoding lives only in `encode`. -->
- [x] Implement family precedence as F1 interpolation > F4 wiki > F2 directive,
  clipping lower-priority spans around higher-priority ownership and discarding
  empty fragments; within a family, make structural subtokens win over broader
  text tokens.
  <!-- `resolve_precedence`: highest `(family, structural)` first; `subtract`
  clips each lower token around already-owned bytes, empty fragments dropped.
  `wiki_beats_directive_but_loses_to_interpolation`,
  `structural_subtoken_wins_within_family`,
  `containment_higher_priority_fully_absorbs_lower`,
  `zero_length_fragment_produces_no_token`. -->
- [x] Implement half-open requested-range intersection on the canonical raw
  stream before line splitting, including tokens crossing either range edge
  and ranges beginning/ending within non-ASCII text.
  <!-- `intersect_range` (half-open `[start,end)`, empties dropped) runs after
  precedence, before splitting. `range_clips_token_crossing_start_edge`,
  `range_clips_token_crossing_end_edge_and_drops_outside`,
  `range_boundary_inside_non_ascii_text`, `range_is_exactly_full_intersected`. -->
- [x] Split every surviving multi-line byte span into non-empty per-line spans
  regardless of the client's multiline-token capability, correctly excluding
  CRLF and lone-CR line terminators through `SourceMap`.
  <!-- Added `SourceMap::split_span_by_line` (line_bounds-based, terminator-
  excluding, empties omitted) + 7 source_map tests; `to_absolute` calls it
  unconditionally. `multiline_span_is_split_per_line_lf`,
  `multiline_span_excludes_crlf_terminator`,
  `multiline_span_excludes_lone_cr_terminator`. -->
- [x] Convert byte spans through `SourceMap`, sort by
  `(line, character, length, token type, modifiers)`, remove exact duplicates,
  reject/assert overlap or non-increasing output, and encode the final stream as
  LSP relative deltas.
  <!-- `to_absolute` uses `byte_to_lsp` (length = encoding-unit char delta);
  `encode` sorts by `sort_key`, `dedup`s, `debug_assert`s non-overlap (drops
  residual overlap defensively), emits relative deltas. -->
- [x] Add table-driven encoder tests for UTF-8 and UTF-16, astral Unicode, CRLF,
  lone CR, multiline tokens, exact duplicates, partial overlap, containment,
  adjacency, zero-length fragments, deterministic tie-breaking, and range
  clipping; decode test output back to absolute positions for readable
  assertions.
  <!-- 21 `semantic_tokens::tests` cases with a `decode` helper reversing the
  relative encoding to `(line, char, len, type, mods)` tuples; UTF-8/UTF-16
  parity + astral + CRLF/lone-CR + adjacency + inert-literal covered. -->

**Phase 2 status: COMPLETE.** `providers::semantic_tokens` model + encoder and
`SourceMap::split_span_by_line` implemented; `just test` (453 dmls tests) and
`just lint` green from `darkmatter/`. Family emitters and handlers remain
Phases 3–4.

## Phase 3 — F1, F2, and F4 family emitters

**Goal:** Populate the canonical pipeline exclusively from existing passive
scanner surfaces.

**Depends on:** Phases 1–2.

**Parallelizable:** The F1, F2, and F4 emitter tasks can be implemented in
parallel after `RawToken`, legend indices, and family priority contracts are
fixed; integration/precedence tests follow all three.

**Validation checkpoint:** Family tests cover every acceptance example and the
combined raw stream is strictly ordered and non-overlapping after resolution.

- [x] Implement F1 emission from
  `overlay::expressions::{interpolations,literals}`: emit one whole outer span
  as `macro.interpolation`, add `inert` for triple-brace literals, and emit
  nothing for malformed constructs or fenced-code content.
  <!-- `interpolation_tokens`: whole `outer` spans, INTERPOLATION on `{{ }}`,
  INTERPOLATION|INERT on `{{{ }}}`; fence/malformed suppression is already the
  scanner's (no re-filter). -->
- [x] Implement F2 emission from `scan_darkmatter_directives` and
  `scan_disclosures`: emit recognized keywords including leading `::` as
  `macro.directive`; add `closer` only to `::end-block`, `::details`, and
  `::end-disclosure`; emit structured targets as `string.directive` and option
  keys/values as `property.directive`/`string.directive`.
  <!-- `directive_tokens`: keyword (macro.directive, +closer via `is_closer`),
  target only for `has_structured_target` families (Shell/Disclosure excluded),
  option key/value; disclosure style tokens split at source `=` into
  property/string. Scanned over the body slice (frontmatter `::` lines never
  tokenized), spans shifted by `body_base`. -->
- [x] Test all twelve directive keywords and explicitly prove unknown
  directives, `::shell`/`::shell-block` command payloads, disclosure summaries,
  and other free-form prose receive no token.
  <!-- `f2_emits_all_twelve_directive_keywords`,
  `f2_unknown_directive_yields_no_token`,
  `f2_shell_payload_is_not_tokenized`,
  `f2_disclosure_summary_prose_is_not_tokenized`,
  `f2_disclosure_style_tokens_split_at_equals`. -->
- [x] Implement F4 emission from structured `ScannedWikiLink` spans: brackets
  and present `#`/`|` separators as `macro.wiki`, non-empty path/heading/alias
  segments as `string.wiki`, with identical token shapes for resolved,
  unresolved, and scanner-recognized unsupported forms.
  <!-- `wiki_tokens`: one broad `macro.wiki` frame (structural 0) per link plus
  non-empty segment `string.wiki` subtokens (structural 1); precedence clipping
  leaves brackets + `#`/`|` separators as `macro.wiki`. Resolution-agnostic
  (spans only), empty segments dropped, embed/block-ref share the shape. -->
- [x] Add combined-family tests for interpolation text inside directive option
  values, wiki-looking text inside interpolation spans, structural subtoken
  precedence, duplicate suppression, and both full-stream and clipped-range
  results.
  <!-- `combined_interpolation_inside_directive_option_value`,
  `combined_wiki_looking_text_inside_interpolation`,
  `combined_wiki_structural_subtoken_precedence`,
  `combined_duplicate_family_output_is_deduplicated`,
  `combined_full_and_range_stream_stays_ordered_and_nonoverlapping`. -->
- [x] Confirm family functions remain pure `(text, scanner context) ->
  Vec<RawToken>` operations with no filesystem, graph, network, shell, or
  workspace mutation.
  <!-- All three emitters take only `(&str, body_base)` and read the passive
  scanner surfaces (`expressions`, `scan_darkmatter_directives`,
  `scan_disclosures`, `scan_wiki_links`); no `ctx`, path, or I/O parameter is
  in scope. -->

**Phase 3 status: COMPLETE.** `family_tokens` (F1 interpolations, F2 directives
+ disclosure style tokens, F4 wiki links) feeds the Phase-2 pipeline; 21 new
`semantic_tokens::tests` cases (42 total). `just test` (474 dmls tests) and
`just lint` green from `darkmatter/`. The `full`/`range` handlers, config, and
capability gating remain Phase 4.

## Phase 4 — Capability, configuration, handlers, and refresh wiring

**Goal:** Expose full/range requests only to capable clients and apply live
configuration changes with protocol-correct refresh behavior.

**Depends on:** Phases 2–3.

**Parallelizable:** Configuration parsing tests and client-capability profile
tests can proceed in parallel before handler/router integration.

**Validation checkpoint:** In-process handler tests prove gating and live config
behavior; server initialization advertises the exact frozen legend and only the
V1 full/range endpoints.

- [x] Add `SemanticTokensConfig { enable: bool }` to
  `darkmatter/dmls/src/config/mod.rs`, defaulting `enable` to `true`, and wire
  `[semantic_tokens]` through file and `workspace/configuration` merging using
  the `SymbolsConfig` precedent; add default, explicit false, partial update,
  and malformed-value tests.
  <!-- `SemanticTokensConfig { enable: bool }` (manual Default = true, like
  WikiConfig) added as `DmlsConfig.semantic_tokens`; snake_case `[semantic_tokens]`
  section flows through the JSON overlay merge like `code_actions`. Tests:
  `test_semantic_tokens_{defaults_enabled,explicit_false,partial_overlay_update,
  malformed_value_keeps_previous}` plus the full-toml/false coverage. -->
- [x] Add `ClientProfile.supports_semantic_tokens` from
  `text_document.semantic_tokens` and
  `supports_semantic_tokens_refresh` from
  `workspace.semantic_tokens.refresh_support`, including conservative fallback
  profiles and capability unit tests.
  <!-- Both gates default off (capability-driven, `unwrap_or(false)`); no keyed
  override needed. Tests: `test_profile_from_capabilities` extended,
  `test_profile_semantic_tokens_without_refresh_support`,
  `test_profile_unknown_client_capability_driven_only` extended. -->
- [x] Advertise `semantic_tokens_provider` only when semantic tokens are
  supported, publishing the frozen legend with `full: true`, `range: true`, and
  no delta support; test both capable and incapable initialization responses.
  <!-- `semantic_tokens_capabilities()` → `SemanticTokensOptions` with the frozen
  `legend()`, `range: Some(true)`, `full: Bool(true)` (no Delta), gated by
  `profile.supports_semantic_tokens.then(...)`.
  `test_semantic_tokens_advertised_only_when_supported`. -->
- [x] Implement `textDocument/semanticTokens/full` and `/range` handlers in the
  standalone provider, obtain the existing document snapshot/`SourceMap`, and
  return empty emission when `semantic_tokens.enable` is false; additionally
  suppress only F4 when `wiki.enable` is false.
  <!-- `semantic_tokens_full`/`semantic_tokens_range` + private `encode_document`
  in `providers::semantic_tokens`: master-switch → empty stream, `wiki.enable`
  gates only the `wiki_tokens` extend. Range maps LSP→byte via `SourceMap`,
  clamping unmappable endpoints to `[0, text.len()]`. Handler tests:
  `handler_master_switch_toggles_emission`, `handler_wiki_enable_suppresses_only_f4`,
  `handler_range_is_full_intersected`. -->
- [x] Register full/range requests in the existing router/server dispatch using
  the same error and missing-document conventions as neighboring providers;
  do not add the provider to the multi-provider registry.
  <!-- Router `dispatch_request` arms + `semantic_tokens_full`/`_range` methods
  route through `state.with_document` (missing doc → `null` result); standalone,
  never added to `ProviderRegistry`. `invalid_params` on malformed payloads. -->
- [x] Detect token-affecting changes during `didChangeConfiguration` and send
  `workspace/semanticTokens/refresh` when the client supports it; ensure config
  applies to every later request even without refresh support and log refresh
  failures without failing the notification.
  <!-- `reload_config` now returns `bool`; `should_request_semantic_tokens_refresh`
  = capability AND `semantic_tokens_refresh_needed` (semantic_tokens OR wiki.enable
  changed). Router sends via fire-and-forget `send_semantic_tokens_refresh`
  (logs send errors). Config always applied regardless of the signal. -->
- [x] Add handler tests proving a runtime `true -> false -> true` master-switch
  transition, independent `wiki.enable` suppression, refresh sent/not sent by
  capability, and refresh failure isolation.
  <!-- Master-switch/wiki/range in `semantic_tokens::tests`; refresh decision +
  isolation in `router::tests`
  (`test_semantic_tokens_refresh_needed_detects_token_knobs`,
  `test_should_request_refresh_gated_on_capability`,
  `test_send_semantic_tokens_refresh_isolates_failure`). -->

**Phase 4 status: COMPLETE.** `SemanticTokensConfig` + the two `ClientProfile`
gates + capability advertisement + the standalone `full`/`range` handlers +
router dispatch + `didChangeConfiguration` refresh wiring are implemented and
tested. `just test` (486 dmls tests) and `just lint` green from `darkmatter/`.
The L1 in-process session tests, no-side-effects, and editor/docs work remain
Phases 5–6 (real-editor verification is manual via the smoke checklist).

## Phase 5 — Protocol and side-effect acceptance coverage

**Goal:** Verify observable behavior across real JSON-RPC sessions and protect
the provider's passive-analysis contract.

**Depends on:** Phase 4.

**Validation checkpoint:** L1 session tests and no-side-effects suites pass with
all ten V1 acceptance criteria represented by named tests (real-editor L2
verification is manual, per the smoke checklist).

- [x] Extend `darkmatter/dmls/tests/lsp_session.rs` with Level 1 (in-process)
  sessions for semantic-token-capable and incapable clients, full and range
  requests, UTF-8/UTF-16 negotiation, live configuration changes, and refresh
  support.
  <!-- `semantic_capable_params(encoding, refresh)` + a `ClientFixture`
  server-request buffer (`wait_for_server_request`); tests span capable/incapable
  advertisement, full+range, utf-8/utf-16 negotiation, master-switch toggle with
  refresh, config-applies-without-refresh, and wiki.enable suppression. -->
- [x] In session tests, decode relative semantic-token responses and assert
  exact absolute `(line, character, length, type, modifiers)` tuples for
  ordinary/multiline/inert interpolations, all directive token classes, wiki
  segments, non-ASCII content, and CRLF documents.
  <!-- `decode_tokens` reverses the relative deltas to `Tok` tuples;
  `semantic_tokens_full_{interpolations_ordinary_inert_and_multiline,
  directive_token_classes,wiki_segments_identical_for_resolved_and_unresolved}`,
  `_encoding_utf8_vs_utf16_on_non_ascii`, `_crlf_multiline_splits_per_line`. -->
- [x] Add full-versus-range property-style cases proving a range response is
  exactly the intersection and clipping of the canonical full response,
  including boundaries inside a token and inside a line ending.
  <!-- `semantic_tokens_range_is_full_intersected`: a single-line range starting
  inside a token and a range crossing the LF, each asserted to the exact clipped
  tuples. -->
- [x] Add overlap acceptance cases proving F1 > F4 > F2 and structural-subtoken
  precedence for both full and range requests; assert strict order and no
  overlapping decoded spans in every response.
  <!-- `semantic_tokens_family_precedence_full_and_range`: interpolation inside a
  directive option value (F1 clips F2) plus wiki `#` separator over the frame
  (structural), full and range; `assert_ordered_nonoverlapping` on every response.
  The synthetic full F1>F4>F2 three-way stack stays a `semantic_tokens::tests`
  unit case (real docs cannot nest all three on the same bytes). -->
- [x] Extend `darkmatter/dmls/tests/no_side_effects.rs` to issue full and range
  requests and prove no files, directories, processes, or other analysis
  side effects are introduced.
  <!-- Full + range requests added to the passive-analysis suite; the sentinel
  assertion already proves no directive executed. -->
- [x] Run `just test` and `just test-l2` from `darkmatter/`; if the host cannot
  provide an L2 harness, record the explicit skip and run the directly
  addressable `dmls` session tests with nextest.
- [x] Run `just lint` and `cargo fmt --check` from `darkmatter/`; use format
  check diagnostically only and correct touched-file style by hand rather than
  running write-mode formatting.

**Phase 5 status: COMPLETE.** Ten in-memory L2 semantic-token sessions
(`tests/lsp_session.rs`) cover capable/incapable advertisement with the frozen
legend, full+range with exact decoded tuples (ordinary/inert/multiline
interpolations, all directive classes, resolved/unresolved wiki segments),
UTF-8-vs-UTF-16 non-ASCII columns, CRLF per-line splitting, range=full-intersected
(inside a token and across a line ending), F1-clips-F2 precedence with
structural wiki subtokens for full and range, master-switch toggle with
`workspace/semanticTokens/refresh`, config-applies-without-refresh, and
`wiki.enable` F4-only suppression. `tests/no_side_effects.rs` now drives full +
range and still proves the passive contract (sentinel absent). `just test`
(497 dmls / 5481 area), `just test-l2` (69 L2), and `just lint` green from
`darkmatter/`; `cargo fmt --check` shows only pre-existing whole-file
local-rustfmt-vs-`main` drift (touched code matches surrounding style by hand).
Editor defaults + docs remain Phase 6.

## Phase 6 — Editor defaults, documentation, and closeout

**Goal:** Make the classification visible and configurable in supported editor
surfaces, then verify the complete V1 deliverable.

**Depends on:** The legend and modifier order from Phase 2; final smoke testing
depends on Phases 4–5.

**Parallelizable:** The Zed integration and four editor documentation updates
can proceed in parallel once the legend is frozen.

**Validation checkpoint:** Each editor has accurate enablement/styling guidance,
Zed ships defaults where its extension API permits, and the full package test
and lint gates pass.

- [x] Add `zed-dmls` semantic-token styling defaults supported by the extension
  surface for `*.interpolation`, `*.directive`, `*.closer`, `*.wiki`, and wiki
  inner text; preserve the distinction between muted machinery and link-like
  wiki segments.
  <!-- Zed's `zed_extension_api` LSP-launcher surface (registered for the
  built-in `Markdown` language) has NO API to inject semantic-token colors —
  those are theme-owned, and shipping color defaults would require a distinct
  `Darkmatter` language (deferred by design.md:234). "Where the extension API
  permits" therefore = the extension's shipped guidance: `zed-dmls/README.md`
  gained a "Semantic token styling" section documenting the per-language opt-in
  and the recommended `experimental.theme_overrides` recipe (muted machinery vs.
  link-like `string.wiki`). -->
- [x] Update `darkmatter/dmls/docs/editors/zed.md` with the required
  `"semantic_tokens": "combined"` (or `"full"`) opt-in and a smoke example.
  <!-- "Semantic tokens" section: per-language `"semantic_tokens": "combined"`
  opt-in, combined-vs-full note, theme-override cross-link, `.dmls.toml` master
  switch, and a `{{ }}` / `::file` / `[[wiki]]` smoke doc with expected muting. -->
- [x] Update `darkmatter/dmls/docs/editors/vscode.md` with copyable
  `editor.semanticTokenColorCustomizations` examples, noting that muted
  foreground colors—not alpha—implement de-emphasis.
  <!-- Copyable `rules` block (`*.interpolation`/`*.directive`/`*.closer`/
  `macro.wiki`/`string.wiki`), explicit "colors cannot carry alpha — dim = muted
  foreground", a theme-scoped variant, and the `enable`/`wiki.enable` note. -->
- [x] Update `darkmatter/dmls/docs/editors/neovim.md` with Neovim 0.9+ LSP
  highlight-group examples for token types/modifiers and Markdown-scoped
  overrides.
  <!-- `@lsp.mod.*.markdown` + `@lsp.typemod.<type>.<modifier>.markdown` links
  (interpolation/directive/closer → Comment/NonText; string.wiki → @markup.link),
  typemod-wins-over-mod note, and ColorScheme/LspAttach persistence guidance. -->
- [x] Update `darkmatter/dmls/docs/editors/helix.md` to state that current
  semantic-token capability absence is safely capability-gated and leaves
  existing behavior unchanged.
  <!-- "Semantic tokens" section: no client support → provider not advertised
  (capability-gated off), tree-sitter highlighting unchanged, auto-enables if
  Helix adds support later, no server change. -->
- [x] Extend `darkmatter/dmls/docs/editors/smoke-checklist.md` with full/range
  coverage and visible checks for ordinary/inert interpolation, directives and
  closers, wiki links, fenced-code exclusion, Unicode, multiline spans,
  `semantic_tokens.enable`, and `wiki.enable`.
  <!-- Added a scratch-doc prep paragraph and six semantic-token checks:
  capability (per-editor opt-in), families-full, range, fenced-code exclusion,
  Unicode+multiline, and config (`enable` master switch + `wiki.enable`). -->
- [x] Update `darkmatter/dmls/docs/features.md`, relevant README/setup text,
  and `.claude/skills/darkmatter/SKILL.md` so the public feature matrix,
  configuration surface, protocol endpoints, and standalone-provider
  architecture do not drift from the implementation.
  <!-- features.md: new "Semantic tokens" subsection (legend/families/precedence/
  endpoints/gating), a matrix row (✅ VS Code/Neovim, ⚠️ Zed opt-in, ❌ Helix),
  Zed/Helix caveat notes, and the config-keys line. README.md: feature-layers
  "Semantic tokens" row + config-keys line. SKILL.md: standalone-provider
  paragraph after Phase 11 (legend, emitters, pipeline, config, editor recipes);
  hash regenerated. -->
- [x] Perform the manual editor smoke checks available on the macOS host and
  record unavailable editor/platform checks explicitly for Windows/Linux
  follow-up; avoid encoding macOS-only paths or behavior in implementation or
  examples.
  <!-- Non-interactive session: no editor GUI / manual smoke run is possible on
  this host. Automated equivalence stands in — the ten L2 in-memory
  semantic-token sessions in tests/lsp_session.rs exercise capable/incapable
  advertisement, full+range decoded tuples, UTF-8/UTF-16, CRLF multiline,
  fenced-code exclusion, precedence, master-switch+refresh, and wiki.enable
  suppression (all passing). Manual VS Code/Zed/Neovim/Helix visual checks on
  macOS + the Windows/Linux passes are recorded as TODO in smoke-checklist.md and
  deferred to a host with editor GUIs; no macOS-only path is baked into code or
  doc examples. -->
- [x] Run final `just test`, `just test-l2`, `just lint`, and
  `cargo fmt --check` from `darkmatter/`, and verify each of the specification's
  ten V1 acceptance criteria maps to at least one passing automated test.
  <!-- `just test` (darkmatter area): 497 dmls + lib/cli, 497/497 passed.
  `just lint`: clean (all three crates). `just test-l2`: real-terminal
  (WezTerm/tmux) render-tree tier — no harness on this non-interactive host, so
  it skipped cleanly (0 run / 497 skipped; BISCUIT_TEST_LEVEL_REQUIRED unset).
  The DMLS in-memory session tests (all 12 `semantic_tokens_*`) are L1 and ran
  under `just test`. `cargo fmt --check` shows only the pre-existing whole-file
  local-rustfmt-vs-`main` drift in `dmls/src/bench.rs` (untouched this phase).
  Acceptance-criteria → test map (all 65 green via targeted nextest):
  (1) f1_interpolation_is_whole_span_macro / f1_literal_carries_inert_modifier /
  semantic_tokens_full_interpolations_ordinary_inert_and_multiline;
  (2) f2_emits_all_twelve_directive_keywords / f2_structured_target_and_options /
  f2_unknown_directive_yields_no_token / f2_shell_payload_is_not_tokenized /
  f2_disclosure_summary_prose_is_not_tokenized /
  semantic_tokens_full_directive_token_classes;
  (3) f4_resolved_and_unresolved_have_identical_shapes /
  semantic_tokens_full_wiki_segments_identical_for_resolved_and_unresolved;
  (4) utf8_and_utf16_agree_on_logical_stream_but_differ_in_units /
  multiline_span_is_split_per_line_lf /
  semantic_tokens_encoding_utf8_vs_utf16_on_non_ascii /
  semantic_tokens_crlf_multiline_splits_per_line;
  (5) f1_malformed_and_fenced_constructs_emit_nothing /
  f4_fenced_wiki_link_is_not_surfaced;
  (6) test_semantic_tokens_advertised_only_when_supported /
  semantic_tokens_capability_is_gated_on_client_support;
  (7) handler_master_switch_toggles_emission / handler_wiki_enable_suppresses_only_f4 /
  test_semantic_tokens_refresh_needed_detects_token_knobs /
  test_should_request_refresh_gated_on_capability /
  semantic_tokens_master_switch_toggles_and_requests_refresh /
  semantic_tokens_wiki_enable_suppresses_only_f4 /
  semantic_tokens_config_applies_without_refresh_capability;
  (8) no_side_effects::dsl_requests_spawn_no_processes_and_open_no_sockets;
  (9) handler_range_is_full_intersected / range_is_exactly_full_intersected /
  semantic_tokens_range_is_full_intersected;
  (10) wiki_beats_directive_but_loses_to_interpolation /
  structural_subtoken_wins_within_family /
  combined_interpolation_inside_directive_option_value /
  semantic_tokens_family_precedence_full_and_range. -->
- [x] Review the final diff for scope: confirm no F3/F5/F6/delta implementation,
  no token legend reordering, no duplicated scanner/parser logic, no stale
  comments/docs, and no unrelated formatting changes.
  <!-- Phase-6 diff is 9 documentation files (zed-dmls/README.md, docs/editors/
  {zed,vscode,neovim,helix,smoke-checklist}.md, docs/features.md, README.md,
  .claude/skills/darkmatter/SKILL.md) + this plan — zero `.rs`, zero justfile,
  zero legend/scanner/parser edits. No F3/F5/F6/delta code, no legend reorder, no
  duplicated parser logic; docs were brought into sync with the shipped
  implementation (not left stale); no unrelated formatting changes. -->
