---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-clean-fidelity/darkmatter/fixes/2026-10-11-clean-fidelity/spec.md"
plan: "darkmatter/fixes/2026-10-11-clean-fidelity/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
---

# Implementation Log for 2026-10-11-clean-fidelity (6 phases)

## Phase 1

Phase 1 is a design-risk phase. It changed no production source. A throwaway
spike was compiled into the `darkmatter` lib test target, run once, and then
removed from the crate. Its source and output are archived, not compiled, at
`fixes/2026-10-11-clean-fidelity/spike/` (`fidelity_spike.rs.txt`,
`report.txt`).

### 1. Root-cause confirmation (current `md clean`, 2026-10-11)

Every reproduction in the spec reproduces byte-for-byte with the current
`target/debug/md clean`: E1, D1, C1, A1, B1 and F1. Additional defaults
observed:

- `one\ntwo` becomes `one two` (Strip is the default).
- `1. a\n1. a\n1. a` becomes `1. 2. 3.` (`increment_ordered_list_bullets: true`).
- CRLF input comes back as LF.

The hypotheses are confirmed:

- **Directive-blind parse.** Confirmed by A1 and B1. Only `::shell-block`
  bodies are masked (`protect_opaque_directive_bodies`). Directive *lines*
  reach pulldown-cmark, become lazy continuations, and are re-prefixed by
  `pulldown-cmark-to-cmark`.
- **Container prefixes lost.** Confirmed by E1, C1 and D1. The serializer, then
  `fix_list_indentation`, `restore_list_markers` and
  `extract_additional_paragraph_contexts`, rebuild prefixes line by line. They
  have no model of a non-paragraph child of an item, so those lines lose the
  item's continuation indent. The indent intended for them is left behind as a
  whitespace-only line, which also explains the introduced whitespace in F1.
- **No output check.** Confirmed: nothing compares the output with the input.

### 2. Entrypoint map

All paths are relative to `darkmatter/lib/src/markdown/` unless stated.

**Counted parses** today go through `cleanup_parser` (`cleanup/mod.rs:51`).
**Real parses** additionally include the uninstrumented `find_code_regions`
(`compose/parse_utils.rs:416`, `Options::all()`), reached through
`OpaqueBodyScan` and `scan_block_pairs`.

| Entrypoint | Location | Default newlines | Counted / real parses today | Phase 4 disposition | Ceiling |
| --- | --- | --- | --- | --- | --- |
| `cleanup_content`, `_compact`, `_loose` | `cleanup/mod.rs:139/162/185` | Strip | 1 / 3–4 | Guarded string wrapper over the body result API; returns the original on refusal; Preserve | 2 counted |
| `cleanup_content_with_indent{,_compact,_loose}` | `:211/234/257` | Strip | 1 / 3–4 | Guarded wrapper; indentation override plus padding override (compact → `none`, loose → `one`); Preserve | 2 |
| `cleanup_content_with_indent*_preserving_incidental` (3 variants) | `:221/244/267` | Preserve | 1 / 2–3 | Collapse into the same guarded wrapper; names may go once nothing calls them (no external users) | 2 |
| `cleanup_to_fixed_width` | `:284` | strip, then reflow | 2 / 4 | Guarded operation: body transform with `fixed-width: N`; verification after reflow | 3 |
| `strip_incidental_newlines` | `cleanup/reflow.rs:58` | n/a | 1 / 2 | **Standalone helper, unguarded**, existing count kept; it already computes source-span edits, which the `unwrap` rule reuses | 1 |
| `reflow_to_width` | `cleanup/reflow.rs:205` | n/a | 1 / 2 | Standalone helper, unguarded, existing count kept. A cleanup that requests a width runs it inside the transaction, followed by the final verification parse | 1 alone; 3 within cleanup |
| `Markdown::cleanup`, `cleanup_compact`, `cleanup_loose`, `cleanup_with_indent{,_compact,_loose}` | `markdown/mod.rs:527-582` | Strip | as above | Guarded; body only (the frontmatter field is untouched); defaults Preserve | 2 |
| `Markdown::cleanup_with_fixed_width` | `markdown/mod.rs:539` | strip + reflow | 2 | Guarded, like `cleanup_to_fixed_width` | 3 |
| `Markdown::strip_incidental_newlines` | `markdown/mod.rs:533` | n/a | 1 | Standalone helper | 1 |
| Compose `ComposeOperation::Cleanup` | `compose/pipeline/phases.rs:107-160` | Strip, forced by `fixed_width` | 1–2 | Calls the guarded wrapper. Default `incidental_newline_mode` stays an explicit compose option; compose is not in scope for `style.markdown` | 2/3 |
| `md clean`, `md FILE --save` | `cli/src/commands/clean.rs:73`, `cli/src/main.rs:165-173` | Strip | 1–2 + YAML | Raw-source transaction (§5) | 2/3 + repair |
| DMLS `textDocument/formatting` | `dmls/src/providers/formatting.rs:26/46`, `router.rs:994` | Strip | 1–2 | Raw-source transaction with the workspace level | 2/3 + repair |

**External callers outside the area** (the Wave 11 consumer check):

- `claudine/lib/src/composition/closure.rs:134,268` and `closure/tests.rs:858`
  call `cleanup_content`. These see behavior change: wraps are kept, `1,1,1`
  is kept, and the output is guarded.
- `claudine/lib/src/composition/prepare.rs:366` sets
  `IncidentalNewlineMode::Preserve` on compose.

**Other in-area callers:**

- `cli/src/commands/render.rs:34-35`: `cleanup_with_indent`.
- `lib/benches/clean_hot_paths.rs`.
- `lib/fuzz/fuzz_targets/markdown_parser.rs`.
- Compose tests: `compose/tests/rendering.rs`.

### 3. Pass dispositions

**Disposition legend:**

- **A** = artifact repair: kept only while the serializer is used.
- **R(rule)** = maps to a rule.
- **X** = removed.

The chosen design (§4) edits authored spans instead of re-serializing. Every
serializer-artifact repair therefore becomes unnecessary and is deleted with
the serializer path in Phase 3. None remains as a separate pass.

| Pass | Location | Disposition |
| --- | --- | --- |
| `protect_opaque_directive_bodies` / `restore_opaque_directive_bodies` | `cleanup/lists.rs:360/410`, `opaque.rs` | Replaced by source protection (§4). Its malformed-scan → source fallback becomes a `declined` result with reason `malformed-region` |
| `extract_list_markers` / `restore_list_markers` | `lists.rs:8/495` | A, so X (markers are never re-serialized) |
| `extract_list_item_contexts`, `extract_additional_paragraph_contexts`, `extract_indented_code_markers`, `fix_list_indentation` | `lists.rs:59/103/214/775` | R(indentation). Rewritten as line-prefix edits; this is the root cause of E1, C1 and D1. Marker-width and marker-padding logic (`list_marker_byte_width`, `commonmark_marker_padding`, `task_marker_byte_width`) is reused |
| `detect_list_indentation` (only reachable from the `None` branch, which no public API takes) | `lists.rs:640`, `mod.rs:407` | X (dead). `indentation: preserve` means "no edit", not "detect" |
| `get_preferred_emphasis_style` (`PREFER_ITALICS`) | `emphasis.rs:11` | X (spec: ambient restyling removed) |
| `preserve_original_emphasis`, `restore_emphasis_placeholders`, `unescape_emphasis_chars`, `restore_backslash_placeholders` | `emphasis.rs` | A, so X (authored bytes are never re-serialized). The `2026-08-27-preserve-backslash-escapes` regression tests remain as fidelity tests |
| `unescape_brackets` | `brackets.rs` | A, so X |
| `fix_blockquote_formatting` (`" > x"` → `"> x"`, `">  > x"` → `"> > x"`) | `blockquote.rs:3` | Partly A, partly unrequested restyling, so X. F1 is caused by the serializer, not by this pass. No rule owns quote-marker spacing |
| `collapse_incidental_soft_break_events` / `SoftBreakModel` | `cleanup/reflow.rs:10`, `reflow/semantic.rs` | R(unwrap). `SoftBreakModel` already yields source-span `StripEdit`s with join-separator logic; it is reused |
| `reflow_to_width` (line-based) | `reflow.rs:205` | R(fixed-width). It normalizes CRLF→LF today (`reflow.rs:208`), so a terminator-preserving path is required. Its protected-line logic (`is_structural_line`, `directive_trimmed`) stays a backstop behind the shared protection |
| `add_text_language_to_empty_code_blocks` | `mod.rs:444` | R(default-language). Becomes an insertion after the fence run |
| `align_tables_in_stream` | `tables.rs:4` | R(table-layout). Becomes a rewrite of the table's own lines, each line keeping its container prefix |
| serializer `increment_ordered_list_bullets: true` | `mod.rs:354` | R(ordered-lists). Becomes number-only edits, applied only when the list is neither uniform nor sequential |
| `normalize_list_spacing` (Normal/Compact/Loose) | `lists.rs:1041` | Compact/Loose become R(list-item-padding: none/one). Normal's "blank at level transitions" behavior is X: the default is `preserve`, and `list-interior-padding` owns item interiors |
| serializer `newlines_after_*` (for example, a blank line inserted between a heading and a paragraph) | cmark defaults | X (no global blank-line normalization). Tests that assert it (for example, `test_cleanup_adds_blank_line_between_header_and_paragraph`) are rewritten in Phase 3 |
| trim leading blank lines; force exactly one final newline | `mod.rs:427-435` | X |
| `list-padding` (new) | none | R(list-padding). New blank-line insertion at the list boundary, using the local terminator; never next to a directive line |
| `list-interior-padding` (new) | none | R(list-interior-padding). New |
| `perf_profile.rs` replica | test-only | Drifted from the real pipeline (it pre-strips). Delete or rebuild in Phase 3 |

### 4. Fidelity spike: chosen representation

**What was prototyped** (archived in `spike/`):

1. **Protection scan.**
   - Whole protected lines are the union of two sets:
     - every `scan_darkmatter_directives` line, with its terminator;
     - every `scan_block_pairs` `Shell` pair's full span. This covers quoted
       `> ::shell-block` regions and their opaque bodies.
   - Inline protected spans are the `ExpressionFinder::find_all` tokens.
2. **Parse view.**
   - The source minus whole protected lines, plus a segment map from view
     offsets to source offsets.
   - This is how directive-aware parsing avoids lazy continuation. A removed
     line cannot lazily continue anything, and a list that continues across
     `::block`/`::end-block` stays one list. That matches compose semantics,
     because compose removes directive lines. Directive-like text in code is
     not reported by the scanners (they skip code regions), so it stays code.
3. **Edits.** One parse of the view yields an edit list in source
   coordinates. A run applies edits, never re-serializes, and applies no edit
   to a protected byte. Prototyped rules:
   - default-language: insertion after the fence run;
   - unwrap: the soft-break span through the next line's container prefix,
     replaced with a space. The edit is refused when the span crosses a removed
     line or an expression;
   - indentation: a per-line whitespace delta applied to every line of a
     nested list's subtree. The target is clamped to the CommonMark-valid band
     `[parent content column, parent content column + 3]`.
4. **Comparison** of view(input) against view(output):
   - normalized events, with text coalesced;
   - soft break becomes a space only under unwrap;
   - empty fence info becomes `text` only under default-language;
   - reference definitions from `Parser::reference_definitions()`, including
     unused ones;
   - each protected line's exact bytes, whether its neighbors are blank, and
     its semantic anchor (the event index it precedes);
   - expression spellings.

**Results** (`spike/report.txt`):

- **All rules off.** Byte-identical, and idempotent, for E1, D1, C1, A1, B1,
  F1, a quoted shell block, a multi-line expression, an unused reference
  definition, mixed CRLF/LF, directive-looking code, and a directive between
  two paragraphs.
- **D1 + default-language.** Only `text` is added, at column 8. This matches
  acceptance criterion 1 exactly.
- **D1 and B1 + indentation 2.**
  - The nested list moves to column 3, the nearest valid continuation indent,
    because the `1.` marker width sets the minimum. Its fence, its paragraph
    and its child lines move together.
  - B1's `::block`/`::end-block` lines are untouched while the list spans
    them.
  - Structure is equal and the result is idempotent.
- **Unwrap.**
  - Joins are made inside paragraphs. A soft break inside `{{ name\n }}` is
    left alone.
  - MIXED keeps the untouched `\r\n` terminators, giving
    `one two three\r\n\r\n- a\n- b\r\n`.
  - The soft break next to a `::file` line between two paragraphs is refused.
- **Guard catches.** The comparator rejected every corrupting candidate the
  prototype produced:
  - the soft-break join into a directive line, before the boundary fix;
  - E1's re-indent of the list inside a quote inside the list, which split the
    quote;
  - a fence-info normalization slip.

  This is direct evidence for the fail-closed backstop.

**Lessons that Phase 3 must design for:**

1. **Container prefixes as segments.** A line's prefix must be modeled as a
   stack of container segments (list indent, `>` marker plus its optional
   space, and so on). An indentation delta applies to the segment owned by the
   target list, not to leading whitespace. E1 under indentation 2 or 8 failed
   (safely) without this.
2. **Columns from source bytes.** pulldown-cmark ranges for a nested
   `List`/`Item` start at the parent's content offset, not at the marker.
   Marker columns must be read from source bytes.
3. **Segment boundaries.** View→source mapping at a segment boundary must
   count as crossing protection when the next segment starts later; line
   starts map to the later segment. Otherwise an edit lands on the terminator
   just before a directive line.
4. **Marker padding.** Indentation 8 under a narrow marker needs
   parent-marker padding (`-    item`), as in today's `fix_list_indentation`.
   This is the case "a requested indent cannot be used literally"; it is
   reused, not reinvented.
5. **Parse counts.**
   - Counted: the spike used 3 per run (planning, input comparison, output
     comparison). Reusing the planning parse's events for the input comparison
     gives the specified **2**.
   - Uncounted: each protection scan reaches `find_code_regions` three times
     (directives, block pairs, `ExpressionFinder`'s private copy).
   - Phase 3 should compute code regions once per text and pass them to the
     scanners through `_in`-style variants. That gives one scanner parse per
     text, counted by a separate `scanner_parse_count`.
   - `find_code_regions` uses `Options::all()`, which differs from
     `cleanup_parser_options()`. Phase 3 either keeps the scanners' own options
     (they define what compose recognizes) or proves equivalence. Directive
     recognition must match compose, so **keep the compose options for
     scanning** and record it.
6. **CRLF.** Existing `strip_incidental_newlines` and `reflow_to_width` start
   with `replace("\r\n", "\n")`. Inside the transaction they must run on
   authored terminators. `SoftBreakModel` edits already carry source spans.

**Discarded alternative: serializer plus repair.** This means keeping
pulldown-cmark-to-cmark and repairing prefixes afterward. The spec's
"all-body-rules-off ⇒ exact body bytes" invariant, CRLF and BOM preservation,
and "disabled regions must not undergo incidental serializer normalization"
would each need a reverse-mapping pass. Every pass in §3 marked A exists only
because of the serializer. The spike shows bounded edits meet every invariant
with no artifact repair. **Decision: bounded source edits plus final semantic
verification. Serializer replacement is not needed, because the serializer is
removed from cleanup.**

### 5. Frozen contracts (Phase 2 to 4 module ownership)

There is **one module tree, `markdown::cleanup`**. No second `clean` module
and no generic pipeline framework are added.

| Module (new unless noted) | Owner phase | Contents |
| --- | --- | --- |
| `cleanup/rules.rs` | P2 rule model | `Leaf<T>`, the rule value types, `MarkdownRuleOverrides`, `ResolvedMarkdownRules`, `RuleSource`, leaf validation, `resolve` |
| `cleanup/diagnostic.rs` | P2 repair pipeline | `CleanStage`, `CleanCode`, `CleanDiagnostic`, `FallbackReason` (moved from the CLI and extended) |
| `cleanup/frontmatter.rs` | P2 repair pipeline | `repair_frontmatter` orchestration, `AuthoredCoordinateMap`, and edit projection, moved from `cli/src/commands/clean/frontmatter_repair.rs` |
| `cleanup/protect.rs` | P3 protect | Protection scan, parse view, segment map, container-prefix segments |
| `cleanup/compare.rs` | P3 compare | Semantic comparison and first-divergence location; test-only broken-pass seam |
| `cleanup/edits.rs` | P3 transforms | `SourceEdit` set; apply with overlap and protection checks; the local terminator for inserted lines |
| `cleanup/mod.rs` (existing) | P3/P4 | Body result API and string wrappers |
| `cleanup/document.rs` | P4 transaction | `clean_document` (the raw-source transaction) |
| `style/schema/markdown.rs` + `descriptor.rs` rows + `docs/schemas/darkmatter.yaml` | P2 rule model | `style.markdown` declarations. Validation delegates to `cleanup::rules`, the single validator |
| `dmls/src/config/mod.rs` (existing) | P2 workspace | `formatting.markdown` and legacy migration |
| `cli/src/commands/clean.rs` and `clean/json.rs` | P4 CLI | Flags to overrides, JSON v1 adaptation, terminal rendering |
| `dmls/src/providers/formatting.rs` and `router.rs` | P4 DMLS | Edits; `window/showMessage` |

**Contract sketch.** Names are frozen. Field-level additions are allowed if
they do not change these semantics.

```rust
// cleanup/rules.rs
pub enum Leaf<T> { Absent, Null, Value(T) }          // absent and null recorded separately; both fall through
pub enum FixedWidth { Off, Columns(u16) }            // 1..=1000; YAML/JSON `true` => Columns(120)
pub enum Indentation { Preserve, Two, Four, Eight }  // integers only; "4" (string) invalid
pub enum ListItemPadding { Preserve, None, One }
pub enum ListInteriorPadding { Preserve, None, ExceptNestedLists, All }
pub struct DefaultLanguage(String);                  // validated: no line breaks, control chars or backticks; "" = add none
pub struct MarkdownRuleOverrides {                   // one per precedence level
    pub frontmatter: Leaf<bool>, pub table_layout: Leaf<bool>, pub fixed_width: Leaf<FixedWidth>,
    pub unwrap: Leaf<bool>, pub indentation: Leaf<Indentation>, pub ordered_lists: Leaf<bool>,
    pub list_padding: Leaf<bool>, pub list_item_padding: Leaf<ListItemPadding>,
    pub list_interior_padding: Leaf<ListInteriorPadding>, pub default_language: Leaf<DefaultLanguage>,
}
pub enum RuleSource { Cli, Document, Workspace, Default }
pub struct ResolvedMarkdownRules { /* each value plus its RuleSource */ }
impl ResolvedMarkdownRules {
    pub fn defaults() -> Self;                       // runtime defaults; independent of the baseline schema
    pub fn all_body_rules_off() -> Self;
    pub fn joins_soft_breaks(&self) -> bool;         // unwrap || a positive width, after precedence
    pub fn permits_tightness_change(&self) -> bool;  // only when a padding rule's source != Default and its value != Preserve
}
pub fn resolve(cli: &MarkdownRuleOverrides, document: Option<&MarkdownRuleOverrides>,
               workspace: Option<&MarkdownRuleOverrides>) -> ResolvedMarkdownRules;
/// Passive. Pending executable values and invalid leaves produce diagnostics and
/// invalidate the whole `style.markdown` object (returns Err).
pub fn document_rules(frontmatter: &serde_yaml_ng::Value, yaml_source: &str)
    -> Result<Leaf<MarkdownRuleOverrides>, Vec<CleanDiagnostic>>;
/// Shared by the TOML and JSON readers in DMLS.
pub fn rules_from_json(value: &serde_json::Value) -> Result<MarkdownRuleOverrides, Vec<RuleError>>;

// cleanup/diagnostic.rs
pub enum CleanStage { Syntax, Schema, Rules, Fidelity }
pub enum CleanCode { Yaml(YamlDiagnosticCode), FidelityFallback /* "clean.fidelity-fallback" */,
                     InvalidRules /* "clean.invalid-rules" */ }
pub enum FallbackReason { MalformedRegion, SerializationFailure, SemanticMismatch }
pub struct CleanDiagnostic { pub stage: CleanStage, pub code: CleanCode, pub span: SourceSpan,
    pub classification: YamlCertainty, pub message: String, pub repairs: Vec<YamlRepair>,
    pub reason: Option<FallbackReason> }
// The fidelity fallback is NonDeterministicFind, repairs: [], span 0..len. Its
// message names the first affected input block as 1-based line and byte column;
// an output-only block uses the nearest input boundary, or EOF.

// cleanup/mod.rs: body level (no frontmatter)
pub enum CleanOutcome { Unchanged, Changed, Declined, Invalid }
pub struct BodyCleanup { pub text: String, pub outcome: CleanOutcome, pub diagnostics: Vec<CleanDiagnostic> }
pub fn clean_body(body: &str, rules: &ResolvedMarkdownRules) -> BodyCleanup; // never returns Invalid
// The existing cleanup_content* wrappers are rule presets that return clean_body(..).text.

// cleanup/document.rs: raw source
pub struct CleanRequest<'a> {
    pub source: &'a str,
    pub cli: MarkdownRuleOverrides,                  // explicit flags only; a CLI default is not an override
    pub workspace: Option<&'a MarkdownRuleOverrides>,// DMLS only
    pub schema: CleanSchemaConfig,                   // existing; the caller builds it from its own context
    pub document_path: Option<&'a Path>,             // None for stdin/untitled: no trigger discovery
}
pub struct CleanResult {
    pub text: String,                                // == source unless outcome == Changed
    pub outcome: CleanOutcome,
    pub frontmatter_span: Option<SourceSpan>,
    pub diagnostics: Vec<CleanDiagnostic>,           // authored coordinates, stable source order
    pub applied: Vec<AppliedRepair>,                 // only repairs actually emitted; [] unless Changed
    pub rules: ResolvedMarkdownRules,
}
pub fn clean_document(request: &CleanRequest<'_>) -> Result<CleanResult, SchemaError>;
// SchemaError covers a missing or unreadable referenced schema (existing typed errors).
// Outcome mapping for the CLI: Invalid => exit 1, no save; Declined => original, exit 0, applied [].
```

**Dependencies.**

- The P2 rule model has no dependencies. The P2 repair pipeline depends on the
  diagnostic types, which the repair task owns. P2 workspace depends only on
  the `rules_from_json` signature above.
- P3 protect and compare depend on nothing from P2 except `ResolvedMarkdownRules`.
- P4 depends on all of them.

### 6. Robustness tables compared with existing loaders (departures recorded before editing)

**YAML document (`style.markdown`).**

- A duplicate key is already invalid YAML: verified with
  `md clean --json dup.md`, which reports `yaml.parse` and
  `yaml.duplicate-key` and exits 1. Matches **I**.
- Today the style parser (`style/parse.rs`) has no `markdown` key. It would
  report `style.markdown` as `UnknownKey` (and reject it under
  `--strict-style`), so registration is required (P2).
- The `StyleFrontmatter` fields are `#[serde(default)]` Options. That is
  acceptable for render styles, but the cleanup reader must not reuse it for
  rules, because `Option` conflates absent with null. `cleanup::rules`
  reads the raw `Value` into `Leaf<T>`.

**TOML workspace (`.dmls.toml`).** Departures from today's loader, all
required by the plan:

1. `FormattingConfig` is `#[serde(default)]` with no `deny_unknown_fields`.
   Unknown keys are silently ignored today, and `formatting.*` unknown keys
   must become **W**. This is scoped to `formatting` only, not to unrelated
   sections (ruling 6).
2. `fixed_width: Option<u16>` has no range check today; it becomes 1–1000.
3. `discover` skips a malformed `.dmls.toml` with only `tracing::warn!`. The
   **W** cells require a client-visible diagnostic, and **no client
   notification channel exists in DMLS today**: there is no `showMessage` or
   `logMessage` anywhere in `dmls/src`. P2 workspace adds the warning record;
   the P4 DMLS task routes it through `router.rs`.
4. `toml` already rejects duplicate keys and trailing garbage as parse errors,
   so those become **W** naturally.

**JSON editor settings.**

1. `recompute` (`config/mod.rs:392`) serializes the *file* config, including
   serde defaults, to JSON and deep-merges the overlay. This is exactly the
   "serialized defaults manufacture legacy keys" hazard in ruling 6. A file
   with the new `formatting.markdown` would serialize as
   `cleanup: "default"`, which makes it look mixed. The fix is to normalize
   each *authored* source to `MarkdownRuleOverrides` before merging, and to
   merge rules leaf by leaf.
2. `merge_json` replaces a value with `null`, so an editor `null` leaf would
   erase a valid file leaf. Under the plan, **N** must release the leaf to the
   lower level instead. This is handled by the leaf-level merge in item 1.
3. On a deserialization failure, `recompute` falls back to the *file* config.
   That already matches "invalid overlay retains valid file settings", but it
   must now also surface a diagnostic.
4. Duplicate names: the settings arrive as `serde_json::Value`, which keeps the
   last duplicate member. The loader cannot observe duplicates (ruling 7, cell
   **X: Value**). This boundary is documented and tested, not changed.
5. `initializationOptions` and `workspace/configuration` are not used today.
   Settings arrive only through `didChangeConfiguration`. The matrix's "normal
   editor configuration request" fixture is therefore a
   `workspace/didChangeConfiguration` payload.

### 7. Acceptance criteria: owners and planned tests

| AC | Owner (phase/task) | Planned tests (L1 unless noted) |
| --- | --- | --- |
| 1 Reproductions (exact, all-off, indentation/unwrap/width/padding variants, multi-digit markers, code children) | P3 verify rule fidelity | `lib/src/markdown/cleanup/tests/fidelity_repro.rs` |
| 2 A1/B1 compose equivalence, `when` true/false | P4 invocation parity | `lib/tests/l1/clean_compose_equivalence.rs` (fixture-owned data, no shell or network) |
| 3 Directive fixed points per rule (after a quote, nested lists, a table) | P3 protect | `cleanup/tests/directive_fixed_points.rs` |
| 4 One test per rule value, list-spacing examples, required separators, ordered sequences | P3 verify rule fidelity | `cleanup/tests/rules.rs` |
| 5 DMLS migration, mixed-shape rejection, compact/loose equivalence | P2 workspace; P4 parity | `dmls/src/config` unit tests plus `dmls/tests/l1/` protocol test |
| 6 Precedence (CLI > document > workspace > default) and invalid-object fallback | P2 rule model; P4 parity | `cleanup/tests/rules_resolution.rs`; CLI and DMLS protocol tests |
| 7 Fidelity guard via the test-only broken-pass seam; mutation list; tight/loose; reflow-introduced; refusal after a YAML repair | P3 compare; P4 transaction, CLI and DMLS | `cleanup/tests/guard.rs`; `cli/tests/l1/clean_fallback.rs`; DMLS protocol test |
| 8 Frontmatter parity (CLI vs DMLS), `frontmatter: false` | P2 repair; P4 parity | `lib/tests/l1/clean_document.rs`; CLI and DMLS fixtures sharing one input |
| 9 Idempotence for AC 1–4 fixtures | P3 verify | helper asserted in each fixture test |
| 10 `prompts/commit.md` @ `464e2d90e` fixture | P5 corpus | `lib/tests/l1/clean_real_world.rs` plus a checked-in fixture |
| 11 Corpus scan counts | P5 corpus | log evidence (not a CI gate) |
| 12 Parse ceilings plus separate scanner counts | P4 transaction | `cleanup/tests/parse_count.rs` (extended) |
| 13 CRLF, mixed endings, BOM, empty frontmatter, missing final newline, all-off, unused reference definitions, malformed regions; CLI and DMLS routes | P3 protect; P4 CLI and DMLS | `cleanup/tests/source_fidelity.rs`; `cli/tests/l1/clean*.rs` (`CliProcessFixture`); `dmls/tests/l1/` |
| 14 Docs | P5 docs | n/a |

### 8. Author-only decisions found

None block Phase 2. Ruling 7's raw-JSON duplicate-name widening stays out of
scope, as the plan says. Two implementation choices are recorded rather than
escalated, because both follow from the spec:

- Scanners keep the compose `Options::all()` for code-region detection (§4,
  lesson 5).
- Inside `markdown::cleanup`, `fix_blockquote_formatting`'s quote-marker
  normalization is removed outright, with no rule replacing it (§3). The spec
  lists "blockquote spacing fixes" among the passes to classify; it is neither
  an artifact repair under the edit model nor mapped to a rule.

### Verification

**Requirement-to-test mapping for this phase.** Phase 1 changed no
behavior, so it added no tests. The spike's assertions (`spike/report.txt`)
are design evidence, not regression tests. Each acceptance criterion's
planned test home and owner are in §7.

**Gates run on macOS.** The tree after the spike was removed has no source
changes.

- `just lint`: **passed** (exit 0, including the zed-dmls wasm32 check).
- `just test`: 8980 tests run, 8974 passed, 5 failed, 1 timed out, 12 skipped.
  None of these failures involves cleanup. All are pre-existing:
  - **5 failures** come from one committed, unmodified file,
    `prompts/_reviews/feature-review.md` (last touched in `c5ad95142`). Its
    frontmatter does not parse: "could not find expected ':' at line 112
    column 9". The failing tests are:
    - `dasherized_identifier_corpus::shipped_corpus_expressions_all_parse`
    - `dasherized_identifier_corpus::corpus_walk_reaches_every_present_surface`
    - `feature_review_incident::the_original_typo_would_have_failed_to_compose`
    - `feature_review_incident::the_typo_is_one_identifier_under_the_current_grammar`
    - `feature_review_incident::the_shipped_feature_review_composes_and_names_its_spec`

    That prompt is outside this fix's scope and was not edited.
  - **1 timeout**, `entry_point_parity::darkmatter_entry_points_agree_on_every_reference`,
    hit the 30s limit under full-suite `--no-fail-fast` load. It took 7.5s in
    the first run and **passes alone in 5.2s**.

**Cross-OS.** No production code changed, so no cross-OS run was needed.
Line-ending and BOM risks are recorded for Phase 3 (§4, lesson 6).
