---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-13-named-type-presence-and-local-names/spec.md
plan: claudine/features/2026-10-13-named-type-presence-and-local-names/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Implementation Log for 2026-10-13-named-type-presence-and-local-names (6 phases)

## Phase 1

- Started 2026-10-11. Phase 1 is investigation only (Wave 1: Surface Inventory, Provenance Spike); no production code changes are planned.
- Investigation fanned out to three read-only agents (schema internals; DMLS + Claudine consumers; shipped artifacts) plus one disposable probe run by the orchestrator. Line numbers below are as of this phase's start tree (`f1d031f7a`) and will drift.

### Surface Inventory

Paths are relative to `darkmatter/lib/src/markdown/schemas/` unless prefixed.

#### Parser and representation

- Entry: `simplified/grammar.rs::parse_type_expr` (:69) → `Parser::parse` (:494) → `Parser::parse_type_expr` (:831). YAML layer: `simplified/mod.rs::parse_property_definition_at_depth` (:306; union arms :331).
- Import detection is a lookahead, `looks_like_import_suffix` (grammar.rs:996), run before the keyword check (:848); then `parse_import` (:1051) / `read_fileref` (:1087). Only `Name[](c)@ref` and `Name(c)@ref` are accepted (`Name(c)[]@ref` is not).
- Representation: `TypeExpr::Imported { name, reference }` (types.rs:195), `"this"` = current file; no span on the AST. Postfix rides on `PropertyAtom` (types.rs:205); `array_constraints` always empty for imports (grammar.rs:1078).
- Spans exist only in `SchemaSourceMap` (source.rs:99, keyed `(SchemaSourcePath, SchemaSpanKind)`): `ImportName`/`ImportReference` (source.rs:1541-1574); with no `@`, the leading identifier is recorded as `TypeKeyword` (:1566) — so a bare reference already gets a span under that kind.
- Bare unknown identifier today: `SchemaError::Grammar { "unknown type `{name}`", span }` (grammar.rs:852-856), pinned by `rejects_unknown_type` (grammar.rs:2027) and loosely by `inline_object_description_with_unescaped_comma_is_not_supported` (:2665). Keyword table `SimplifiedType::from_keyword` (types.rs:368): `string date datetime time number numberlike boolean boolish object file enum url email yaml json literal expression type-definition schema any` (`integer`, `array`, `null` are not keywords).
- Serialization: `serialize_property_atom` (serialize.rs:15), `Imported` arm :56-67 always emits `name[](c)@reference`.
- Cursor: `SchemaCursorRole::ImportReference` (cursor.rs:56) triggered by `@` (:423-495); a bare name yields `Type`. `query.rs:236`, `lint.rs:174-191` read `Suggest` from the atom (first match only).

#### Resolver (`resolve.rs`)

- Fast path: `expand_document_imports` (:942) returns early unless `schema_has_imports` (:1454-1482), which only looks for `TypeExpr::Imported` — a bare-only schema would skip expansion. Callers: :168 (inline mapping), :220 (root-union arm), :727 (standalone file), :779 (standalone root-union arm).
- `@this` namespace: `Single` shape properties; root `Union` → empty map (:954-957).
- Engine: `NamespaceKey` (:889), `Namespace` (:901), `ImportEngine { stack, dependencies, schema_roots, file_resolution_context, namespace_cache }` (:914), `CachedNamespace` (:929). Walk: `expand_schema` :975, `expand_shape` :1002 (passes `shape.constraints` through), `expand_property_def` :1025 (flattens union targets), `expand_atom` :1046. No property path is threaded today.
- `expand_import` (:1089): depth guard `MAX_IMPORT_DEPTH = 32` → `ImportCycle`; missing name → `SchemaError::Convert { property: <type name>, … }` with **no file and no span** (:1109-1119); cycle frame check → `ImportCycle { chain }` (:1121); `strip_top_level_presence` call at :1135 with the stale comment :1131-1134.
- `resolve_namespace` (:1152): `this` returns without a dependency edge (:1158); file targets insert the edge (:1215) and use the cache (:1219-1237). `load_named_types` (:1269) parses through `parse_standalone_schema_document` — which **already builds a `SchemaSourceMap`** — and discards it; rejects root unions and whole-file references.
- `strip_top_level_presence` (:1297-1313) strips `Required`/`Generated` from `constraints` and `array_constraints` of every arm.
- `apply_import_postfix` (:1319): verbatim when no postfix; union target + any postfix (incl. description) → `Convert` (:1333); array-level `suggest` → Grammar; suggest needs exact string/number target and is reinterpreted (`grammar.rs::reinterpret_suggestion_candidates` :1752); >1 suggest → Grammar (:1397); already-array + `[]` → `Convert` (:1418); array case appends use-site constraints to `array_constraints` (:1429); non-array case plain `extend` (:1440) — no dedup, no bound comparison. Description: `description.or(base)` (use site wins).
- Typed wrapper: `SchemaError::ReferencedSchema { path, source }` (errors.rs:46) is applied only to Grammar/Convert/Aggregate in `load_guarded_in_context` (resolve.rs:524-530). Only `Grammar` carries a span; `Convert`, `ImportCycle`, `SchemaDocument` carry none. No contradiction or unknown-named-type variant exists.

#### Constraints and bounds

- `Constraint` (types.rs:401-501): `Required Default Generated Min Max Integer MinLen MaxLen NotEmpty Pattern Suggest Members LiteralValue Match Eager Scheme Unique MinItems MaxItems MinKeys MaxKeys Example`. No span except `SuggestionCandidate.span`.
- `min`/`max` dispatch (grammar.rs:1444-1493): array level → `MinItems`; `String` → `MinLen`; else `Min(f64)`. Imports parse with `SimplifiedType::Any` (:1065), so **bound interpretation is not deferred for imports** — only `suggest` is.
- `$constraints` (simplified/mod.rs:197-199, `parse_dollar_constraints` :229): keys `min-keys max-keys min-items max-items required not-empty unique generated integer`; integers via `constraints_usize` (:283, errors on non-integers), flags via `constraints_flag` (:294, must be `true`); unknown key → Grammar; spans `0..0`. No `serde(default)`/`unwrap_or_default`/`filter_map(as_str)` there. Lowering `apply_object_arity` (convert.rs:727) emits only `MinKeys`/`MaxKeys` and **silently ignores every other `$constraints` key**; inline objects insert shape constraints then atom constraints, last insert wins (convert.rs:521-522).

#### Lowering (`simplified/convert.rs`)

- `string_fragment` (:847): `NotEmpty` writes `pattern: "\S"` (:880) and `Pattern` writes `pattern` (:883) — single key, last wins. Bounds: `minLength`/`maxLength` :868/:871, `minimum`/`maximum` :921/:924, `minItems`/`maxItems`/`uniqueItems` :756-762, `minProperties`/`maxProperties` :731/:734, `x-darkmatter-url-scheme` :1037.
- `file_fragment` (:970) ignores `Match` (:994). `attach_root_union_match` (:127) / `attach_match_keyword` (:138) emit flat-array `x-darkmatter-match`; called from convert.rs:106 and resolve.rs:237, :265, :798, :816.
- Other single-`pattern` readers: `coerce.rs:252` (`target_from_any_of` compares `pattern` with `NUMBERLIKE_PATTERN`), resolve.rs:663 keyword allow-list; tests asserting `v["pattern"]`: convert.rs:1293, :1299, :1328, :1443, :1513; coerce.rs:734.

#### File-match model and every consumer

- `file_match.rs` public: `DARKMATTER_MATCH_KEYWORD` (:31), `FileMatchGlobs { globs: GlobReference }` (:41; `new`, `matches`, `prepare`), `root_union_match_patterns` (:92), `file_match_admits` (:120). Crate-private: `definition_error` :79, `file_match_patterns` :99 (**first** `Match` via `find_map`), `admits` :127, `match_keyword_factory` :158 (expects a flat string array), `MatchKeyword` :235.
- Darkmatter consumers: `completion.rs:158-165` (`suggestion_from_union` **concatenates** arms' patterns) and :239-248 (`kind_for_atom`, first `Match` → `CompletionKind::File { patterns }`); `validate.rs:517` (keyword registration), `is_match_rule_out` :609 and `is_match_keyword_error` :705 (assume the schema path **ends with** the keyword — breaks if wrapped in `allOf`); `compose/schema_validation.rs:840` `without_match_keyword`; `about.rs:82,258,813,976`; `format.rs:35-37`; grammar.rs:1554; serialize.rs:190; `darkmatter/cli/src/commands/schema/about.rs:496` (display only).
- Claudine consumers: `claudine/cli/src/completion/schema_completion/candidates.rs` (`MatchGlobs` alias :406; `property_value` :40/:53; `file_candidates` :91/:136; `file_candidate_paths` :177/:183; `match_glob_files` :211 walks one prepared glob); `claudine/cli/src/commands/schema_interactive/mod.rs` (:40, :225, :369, :495, :657) and `schema_interactive/supplied.rs:89`; `claudine/lib/src/composition/schema/supplied.rs` (imports :5, `FileTarget.patterns` :99, `eager_file_target` :122 and `file_reference_target` :211 **concatenate arms**, `file_match_target` :163 first `Match`, `admits_arm_files` :322-335 root-union matching); `classify.rs:239-243, :351` (first `Match`); public fields `UnresolvedFileReference.patterns` and `InteractiveShape::File.patterns` (`claudine/lib/src/composition/error/mod.rs:2450, :2878`).
- Shared test matrix: `darkmatter/lib/tests/common/entry_point_parity/mod.rs` (`patterns()` :660/:703) is included by lib, cli, dmls, claudine-cli L1 and claudine parity tests.
- Other `TypeExpr::Imported` readers to update for bare names: `claudine/gen/src/schema_compat.rs:81, :307`, `darkmatter/cli/src/commands/schema/assignment.rs:191`, `compose/context/catalog.rs:296`, `schemas/detect.rs:433`, `simplified/query.rs:236`, `simplified/source.rs:2118`.

#### DMLS (`darkmatter/dmls/`)

- `overlay/schema.rs`: `assemble` :934, `combined_baseline` :1006 (extension dependencies :1027), `semantic_type_regions` :74, `frontmatter_authoring` :212. No named-type logic.
- `providers/frontmatter.rs`: `type_definition_completions` :378 offers `"{name}@this"` :452-466 from `passive_namespace_names` :543; `Name@./types.yaml` scaffold :437; `ImportReference` role returns nothing :403; `constraint_completions` :479 only knows built-ins. Hover: `meta_schema_hover` :1198, `meta_schema_definition_hover_body` :1307, `schema_hover_details` :1364 — none shows the *target* definition. Go-to-definition (`definition` :1431, `nav_targets` :1466) has **no** named-type support. Test `meta_schema_completion_catalog_is_descriptor_driven` (:3886) asserts `"Base@this"` (:3972).
- Diagnostics: `diagnostics/frontmatter.rs::schema_prepare_diagnostic_with_origin` (:156) unwraps `Aggregate`/`ReferencedSchema`; property-bearing Grammar/Convert are ranged at `$schema.<property>` (so today's missing-name error, whose `property` is the *type* name, lands on the wrong key or the fallback); everything else → `SCHEMA_PREPARE` at the `$schema` value. `relatedInformation` only for referenced origins (zero range).
- Dependencies: edge recorded at resolve.rs:1215, exposed by `EffectiveSchema::dependencies` (schemas/mod.rs:877); DMLS cache `overlay/mod.rs` (lookup :430-436, `schema_cache_key` :513, `collect_dep_hashes` :527, `deps_unchanged` :548) with unit tests :611-885; watched files `router.rs::apply_watched_changes` (~:1195).
- L1 LSP harness: `darkmatter/dmls/tests/common/mod.rs` (`LspWorkspace` :102, `LspFixture` :144, in-memory connection); `tests/l1/lsp_session.rs` helpers `hover_markup` :318, `hover_value` :327, `hover_surfaces` :4446; related tests :3807, :3944, :4668. Suggestion lints: `overlay/suggestions.rs` → lib `lint_suggestions` (lint.rs:115; `lint_atom` :174 reads `atom.ty` directly).

#### Shipped artifacts (pre-audit; the disposition is Phase 5's)

| Location | Named types / references | Effect of this feature |
|---|---|---|
| `darkmatter/docs/schemas/claudine-types.yaml`, `claudine.yaml` | 7 `@this` (lines 14, 15, 31, 43, 46, 61, 72), 9 `@./claudine-types.yaml` | referenced types carry no top-level presence; only use-site `lifecycle-action(required)@this` / `lifecycle-action[](required)@this` — no repeat |
| `darkmatter/docs/schemas/err.yaml`, `env.yaml` | all types `required;generated` | not referenced anywhere |
| `darkmatter/docs/schemas/darkmatter.yaml` (embedded baseline, `schemas/mod.rs:149`, `simplified/mod.rs:801/852/910`) | no references | none |
| `content-policy/schemas/content-policy.yaml` (embedded `content-policy/lib/src/lib.rs:67`) | `short_form` (union), `long_form`, 3 self-refs `@./content-policy.yaml` | no top-level presence on referenced types |
| `darkmatter/schemas/` (+`partials/`) | 6 `@./partials/…` | `err`/`context` have only inner required (not inherited); no test loads this tree |
| repo-root `schemas/`, `claudine/schemas/`, `.claudine/schemas/`, `darkmatter/example-docs/schemas/` | none | none |
| `claudine/docs/schemas/` (+`partials/`) — drafts | bare `stack_item`, `terminal_node`, `stack`, `stop`, `skip`; `shell@./shell.yaml` (missing file); `number`/`date` local types in `partials/functions.yaml` (built-in collisions) | draft-only; the corpus test walks it non-recursively and skips non-loadable files |
| Inline `$schema` in shipped Markdown (922 files) | references only in `messenger/features/…/spikes/` probes; `darkmatter/docs/research/parsing/_fleet_future.md:5` bare `crate[]` (`crate` lives under `$types:`, stays an unknown-name error) | none |

- No tested schema has a local type colliding with a built-in, and no shipped use site repeats a bound/pattern/match/scheme/default. Expected outcome: **no shipped schema edit**; Phase 5 must still confirm and record per file.
- Existing corpus/readers: `darkmatter/lib/tests/l1/schema_phase_validation.rs:549` (`shipped_schema_and_trigger_corpus_parses_passively`, non-recursive, skips `darkmatter/schemas/`, `content-policy/schemas/`, `partials/`, `err.yaml`), `meta_schema_phase4.rs:263`, `meta_schema_repo_schemas.rs:50/:83`, `content_policy_editor_schema.rs:52-172`, `base_schema_end_to_end.rs:63-351`, `claudine/cli/tests/l1/shipped_prompts.rs:8`, `darkmatter/dmls/tests/l1/mapping_only_corpus.rs:64`. `md schema validate` L1: `darkmatter/cli/tests/l1/schema_validate.rs` (`CliProcessFixture`, inline constants, **no import coverage yet**).

#### Tests and comments that assert stripping (Phase 3 must update)

- Comments: resolve.rs:1131-1134 and the `strip_top_level_presence` doc :1297-1300.
- resolve.rs tests: `expands_named_type_import` :2462 (nullable at :2478), `resolves_this_self_import` :2486 (:2494), `constrained_import_applies_use_site_constraint` :2575 (comment :2578), `multiple_imports_from_same_file_resolve_and_dedup` :3591 (comment :3613-3615), `cross_file_type_imported_twice_keeps_each_use_independent` :3787 (:3800, :3804), `root_this_type_imported_twice_keeps_each_use_independent` :3810 (:3823, :3826), `this_inside_imported_file_selects_that_files_type` :3831 (:3855, :3861). No `darkmatter/lib/tests/**` file asserts stripping.

#### Package areas requiring checks

- Code changes: `darkmatter` (crates `darkmatter`, `darkmatter-cli`, `dmls`), `claudine` (crates `claudine`, `claudine-cli`, `claudine-gen` — `schema_compat.rs` reads `TypeExpr::Imported`).
- Reader whose results may move: `messenger` (`messenger/lib/src/research/load.rs`, `tests/research_corpus.rs`, `tests/research_validation.rs` use the schemas API) — run its `just test` in Phase 6.
- `content-policy` only if its schema changes (not expected).

### Provenance Spike

One disposable probe (`darkmatter/lib/tests/l1/zz_provenance_spike_probe.rs`, plus a temporary `mod` line) was run twice and then deleted; `git status` is clean apart from this log. Fixture: `types.yaml` = `kind: schema` with `label: "number(min(10))"` (quoted scalar; first run used `string(min(10))`), `title_base: label@this` (multi-hop), `req: expression(required)`, `card: { $constraints: { min-keys: 3 }, a: string }`; inline `$schema` = `title: title_base(max(5))@./types.yaml`, `when: req@./types.yaml`, `box: card(max-keys(2))@./types.yaml`.

Observed:

| Question | Result |
|---|---|
| Defining-file constraint spans | `parse_standalone_schema_document` source map: `[label]` Constraint → `min(10)`; `[title_base]` ImportName → `label`; `[req]` Constraint → `required`; `[card, $constraints, min-keys]` Constraint → `min-keys: 3` |
| Use-site spans (inline `$schema`) | `parse_schema_declaration_with_source`: `[title]` Constraint → `max(5)`, ImportName → `title_base` (the consuming reference); `[box]` Constraint → `max-keys(2)` |
| Today, numeric target | resolves **silently** to `minimum: 10, maximum: 5`; object to `minProperties: 3, maxProperties: 2`; `when` is nullable (stripped) |
| Today, string target | `Convert { property: "title", "constraint `max` is not valid on `string`" }` — bounds are not reinterpreted for the resolved target (pre-existing defect the spec's `label(max(80))` example depends on) |
| Block scalar in a named-type file | `parse_standalone_schema_document` **fails** (`could not project SimplifiedSchema expression spans through YAML source`): importable files are already limited to the v1 source presentations, so every importable declaration has spans |
| Nested typed error | `$schema: ./wrapper.yaml` whose import names a missing type → `ReferencedSchema { path: wrapper.yaml, source: Convert { property: "missing", … } }` — wrapper survives, but the inner error has no file/span and names the type, not the property |
| Bare name today | `Grammar { property: "when", "unknown type `req`", span: 0..3 }` |

#### Chosen representation

- **No span on `Constraint` or `PropertyAtom`.** Their constructors are used across hundreds of tests and `Constraint` equality is asserted widely; spans are already recorded by `SchemaSourceMap` for every authored constraint (including `$constraints` entries). Provenance is an *address* into a source map, carried only by the resolver and by errors.
- New public types (Phase 2 owner: `errors.rs` / `simplified/source.rs`):
  - `SchemaSourceId` — `InlineDocument` | `File(PathBuf)` (canonical, via `biscuit_file::canonicalize_simplified`).
  - `SchemaConstraintOrigin { source: SchemaSourceId, path: SchemaSourcePath, keyword: String, occurrence: usize, span: Option<Range<usize>> }` — `path` is the structural path of the authoring atom in that source (`[type-name, …]` in a named-type file, `[property, …]` under an inline `$schema`; `$constraints` entries use `path.property("$constraints").property(key)`); `occurrence` is the index among `SchemaSpanKind::Constraint` spans at `path` whose text begins with `keyword` (value-level groups precede array-level groups in source, matching `T(c)[](c)`).
  - `SchemaReferenceSite { source: SchemaSourceId, path: SchemaSourcePath }` — the consuming reference; its span is the `ImportName` span (explicit) or `TypeKeyword` span (bare) at `path`.
  - `SchemaError::ConstraintConflict { site: SchemaReferenceSite, name: String, kind: (bounds | scheme | default), constraints: Vec<SchemaConstraintOrigin> }` and `SchemaError::UnknownNamedType { name, namespace: SchemaSourceId, site: SchemaReferenceSite }` (replacing today's `Convert` for missing names). Both join the `ReferencedSchema` wrapping set in `load_guarded_in_context`.
  - One shared lookup helper on `SchemaSourceMap` (e.g. `constraint_span(source_text, path, keyword, occurrence)`) used by both the resolver and DMLS.
- **Resolver-internal tracking:** thread a `SchemaSourcePath` through `expand_shape`/`expand_property_def`/`expand_atom`, and have `expand_import` return the expanded definition plus origins for each surviving top-level constraint (value, array-container, and shape `$constraints` for object targets). Only top-level constraints can conflict at a merge, so nested origins need not be stored. Stricter-bound selection keeps the winning constraint's origin; a conflict carries both.
- **Span filling:** `load_named_types` keeps the `source_map` (already computed) and the file text in `CachedNamespace`; on the **error path only** the resolver fills `span` for `File` origins. `InlineDocument` origins leave `span: None` — inline resolution receives a YAML value, not text — and DMLS fills them from the `$schema` source map it projects with the same helper. If DMLS cannot project (e.g. a block scalar in frontmatter), it falls back to the property's entry span, as it does today.
- **Use-site paths:** the reference site uses the consuming property path (inline `$schema` or the importing file's type path), so DMLS anchors the primary range on the reference, never on the definition.

Diagnostic example for the fixture (planned shape, DMLS):

```
document.md 2:8-2:18  error dm.schema.invalid-type-definition
  property `title`: contradictory bounds on named type `title_base`: `min(10)` and `max(5)`
  related: types.yaml 3:18-3:25  `min(10)` — declared by `label` (via `title_base` → `label`)
  related: document.md 2:20-2:26 `max(5)` — use site
```

(primary range = `title_base` ImportName span in the frontmatter; types.yaml range from the cached source map; the second related location is the use-site constraint span.)

Affected public constructors: `SchemaError` (two new variants; existing variants unchanged), new public `SchemaSourceId`/`SchemaConstraintOrigin`/`SchemaReferenceSite`. `Constraint`, `PropertyAtom`, `PropertyDef`, `TypeExpr` constructors unchanged by provenance (bare-name representation is a separate Phase 2 decision; recommended: reuse `TypeExpr::Imported { name, reference: "this" }` with a marker for bare spelling so serialization can stay bare, rather than a second resolver path).

### Engineering choices settled at the checkpoint

1. **Bound reinterpretation is in scope** (Phase 3, resolver): after resolving the target, reinterpret import-site `Min/Max(f64)` as `MinLen/MaxLen` for string targets, mirroring `reinterpret_suggestion_candidates`. Without it the spec's `label(max(80))` example and AC 4/9 ("target-directed bound interpretation") cannot pass.
2. **Postfix on an already-array target** (`tags(min(1))` where `tags: string[]`): today the constraints are appended to the *item* list (resolve.rs:1440). Choice: interpret against the resolved target, i.e. the array container (item count), consistent with "applicability and interpretation depend on the resolved target"; `required`/`generated` go to the property either way. No shipped schema uses this form. Recorded here as a departure risk for review.
3. **Single-pattern output unchanged:** emit `allOf` only when an atom has more than one pattern-producing constraint (`pattern`, `not-empty`); a lone pattern keeps today's `pattern` keyword so `coerce.rs:252` and unaffected schemas keep equivalent output (AC 10).
4. **Grouped match encoding** (settle exactly in Wave 2): keep one `Constraint::Match(Vec<String>)` per authored group (already representable); `x-darkmatter-match` stays a flat string array for one group and becomes an array of string arrays only for more than one group. Union-arm aggregation in `completion.rs:158` and Claudine `supplied.rs` must become a union of per-arm group lists, not one concatenated list.
5. **`$constraints` silent loss:** `apply_object_arity` ignores `required`, `generated`, `not-empty`, `unique`, `integer`, `min-items`, `max-items`. The plan's robustness matrix covers `required`/`generated` in `$constraints`; Phase 3/5 must define and test their effect at the public seam (or reject them) rather than continue dropping them.
6. **`match` validation path:** `validate.rs::is_match_rule_out`/`is_match_keyword_error` assume the error path ends in `x-darkmatter-match`; grouped validation must keep that suffix (e.g. one keyword holding all groups) rather than wrapping groups in `allOf`.

### Acceptance-to-test map (planned public assertions)

| AC | Planned evidence | Phase |
|---|---|---|
| 1 | resolve.rs/lib L1: `a: code`, `b: code@this`, `c: code@./types.yaml` → equal fragments and `required`/`x-darkmatter-generated`; authored vs effective validation (absent permitted authored, rejected effective; invalid supplied value rejected in both) | 3 |
| 2 | lib L1: `req: expression(required)` as `when: req` at top level and inside an inline mapping type → `required` contains `when` in both | 3 |
| 3 | lib L1 table: required-only / generated-only / both / unconstrained; multi-hop local and cross-file; nested object; `person[]` required; union arm hoist | 3 |
| 4 | lib L1 merge table per bound kind (string length, number, item count, keys incl. `$constraints` + postfix); `min(10)+max(5)` → `ConstraintConflict` with two origins (file + span); patterns/`not-empty` conjunctive via `allOf` and actual validation; scheme intersect + empty → error; defaults `3`/`3.0` equal, `"3"`/`3` conflict; examples ordered with origins; duplicate suggest across union arms | 3 |
| 5 | grammar/resolve L1: bare vs `@this` identical for each Decided-3 form; `string` vs `string@this`; `UnknownNamedType` names type + file; bare inside imported file resolves in its defining file | 2–3 |
| 6 | dmls L1 (`LspFixture`): bare completion, `string@this` for collisions, hover/definition on bare refs, conflict diagnostic with related locations, missing-name range on the reference, mixed-cycle chain, dependency refresh after editing an imported file | 4 |
| 7 | Phase 5 audit table in this log; no remaining stripping assertion (grep) | 5 |
| 8 | `just test`/`just lint` in `darkmatter/`, `claudine/`, `messenger/` (test) | 6 |
| 9 | serialize.rs round-trip tests; passive corpus test over all schema folders + baseline + inline blocks; `darkmatter/cli/tests/l1/schema_validate.rs` import fixtures | 2, 5 |
| 10 | union-postfix and already-array rejections retained; unaffected-schema lowered-output equivalence; `file_match.rs` group tests; claudine `schema_completion/tests.rs` `guide-intro.md` vs `notes.md`/`guide.txt`; empty intersection with no walk; root-union arm selection; single-schema passivity | 2–4 |

### Verification and close

- No production source, docs, or skill files changed in Phase 1; the only repository change besides this log is the plan's checkboxes/frontmatter and the spec's `human_review`/`message_to_agent` frontmatter. The probe file and its `mod` line were removed (verified with `git status --short`).
- `just test` in `claudine/`: 8673 passed, 9 skipped (exit 0). `just lint` in `claudine/`: exit 0. Darkmatter gates were not run: no Darkmatter code changed; the probe compiled and ran under `cargo nextest run -p darkmatter --test l1` before removal.
- OS evidence: macOS only; nothing OS-specific was changed or concluded. Path-identity work (`SchemaSourceId::File` canonicalization, related-location URIs) is where Phase 2/4 must load the `os` skill.
- No requirement-to-test mapping applies to Phase 1 (no behavior changed); the planned mapping for criteria 1–10 is the table above.
- Claudine skill: no update needed for Phase 1 (no behavior change). The grouped-match change to Claudine's completion description is scheduled for Phase 5.
- Phase 1 complete: inventory and provenance design delivered; checkpoint choices recorded under "Engineering choices settled at the checkpoint".
