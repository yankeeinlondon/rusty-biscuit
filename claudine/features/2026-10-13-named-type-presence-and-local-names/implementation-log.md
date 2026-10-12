---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-13-named-type-presence-and-local-names/spec.md
plan: claudine/features/2026-10-13-named-type-presence-and-local-names/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/tests/l1/glob_consumers.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/supplied/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/parity_tests.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/src/commands/schema_interactive/supplied.rs
    - claudine/cli/src/commands/schema_interactive/tests.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/completions/shell-completions.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/schema_validate_phase.rs
docs_updated_during_phase_5:
    - darkmatter/docs/topics/schemas/local-type-references.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/inline/schema-validation.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/claudine/cli-commands.md
packages:
    - darkmatter
    - dmls
    - claudine
    - claudine-cli
    - darkmatter-cli
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

## Phase 2

- Started 2026-10-11 by claude/opus. Wave 2 (Model Contracts) was done by the orchestrator in one pass. Wave 3 (Local Grammar, Grouped Match Model) went to two parallel foreground sub-agents with disjoint file ownership.

### Wave 2 — Model Contracts (API record for Phases 3–4)

**File ownership.** `simplified/types.rs`, `simplified/source.rs` (provenance types only), `errors.rs`, and the re-exports in `simplified/mod.rs` and `schemas/mod.rs` belong to this wave. In later waves, a change to a shared type returns to this contract and is recorded here.

**Local-reference representation (no new AST variant).** A bare non-keyword `Name` parses to `TypeExpr::Imported { name, reference: "this" }`, the same value `Name@this` produces. New helpers in `types.rs`: the `LOCAL_REFERENCE` (`"this"`) constant, `TypeExpr::local(name)`, `TypeExpr::is_local_reference()`, and `SimplifiedType::is_keyword(name)`, which is the built-in collision predicate. The spelling is not stored, for three reasons.
  - Both spellings must resolve identically (AC 5). One value makes that true by construction, and the existing import path does all of the resolving: the `schema_has_imports` fast path, `this` namespace selection, cycle stack, depth limit, and no dependency edge for `this`. No second resolver exists.
  - Canonical serialization is bare unless `SimplifiedType::is_keyword(name)` requires `@this`. With no stored spelling, serialize → parse round-trips to an *equal* AST.
  - Source spans distinguish the spellings where an editor needs it. The span of the reference name is `SchemaSpanKind::ImportName` in both spellings. Only the explicit spelling also records `ImportReference`.
  - Consequence: every existing `TypeExpr::Imported` consumer (`claudine/gen/src/schema_compat.rs`, `claudine/lib/src/composition/schema/classify.rs`, DMLS hover) already handles bare references, and still renders them as `name@this` until Phase 4/5 adopts `serialize_property_atom` there.

**Provenance (in `simplified/source.rs`, re-exported from `schemas`).**
  - `SchemaSourceId { InlineDocument, File(PathBuf) }`. A `File` path is the canonical path the resolver loaded from.
  - `SchemaConstraintOrigin { source, path, keyword, occurrence, span: Option<Range<usize>> }`.
  - `SchemaReferenceSite { source, path }`.
  - `SchemaConstraintConflictKind { Bounds, Scheme, Default }`.
  - `impl Display for SchemaSourcePath` (`a[1].b`, `<root>`).
  - `SchemaSourceMap::constraint_span(source_text, path, keyword, occurrence)` turns an origin address into a span. It matches whole words, so `min` never selects `min-keys(…)`. A `$constraints` entry is addressed at `path/$constraints/<key>`.
  - `Constraint`/`PropertyAtom` carry no spans, as decided in Phase 1.

**Typed errors (`errors.rs`).**
  - `SchemaError::UnknownNamedType { name, namespace: SchemaSourceId, site }`, whose message is `"<site.path>: unknown named type `<name>` in <namespace>"`.
  - `SchemaError::ConstraintConflict { name, kind, site, constraints: Vec<SchemaConstraintOrigin> }`. It lists every origin in merge order.
  - Both variants have status blocks and join the `ReferencedSchema` wrapping set in `resolve.rs::load_guarded_in_context`.
  - **Neither is produced yet.** Phase 3 produces them once it threads `SchemaSourcePath` through `expand_*`. Until then, a missing named type is still `SchemaError::Convert { property: <type name>, "named type `x` is not defined in `this` …" }`.

**Grouped file-match encoding (settled; Wave 3 implements the model, Phase 3 the emission).**
  - Each authored `match(...)` stays one `Constraint::Match(Vec<String>)` group.
  - `x-darkmatter-match` is internal generated-schema metadata, not user configuration. One group keeps today's flat string array, so unchanged schemas lower identically. More than one group is an array of string arrays, `[["*.md"], ["guide*"]]`. The keyword decoder accepts both, and the keyword name stays the last path segment, so `validate.rs::is_match_rule_out`/`is_match_keyword_error` keep working.

### Wave 3 — Local Grammar

- `grammar.rs`: `parse_import` became `parse_named_reference(name, explicit)`, one path for both spellings. Bare and `@this` produce equal ASTs for `Name`, `Name[]`, `Name(c)`, `Name[](c)`, `-> description`, inline objects, and union arms. Constraints are parsed with `Any` plus `defer_suggestion_target`, so bound interpretation is deferred as for explicit imports. Keywords win, and `keyword@ref` is still an explicit reference. `Name(c)[]` and `Name(c)[]@ref` now share one error, "`[]` must come before the constraint list in a named type reference"; the explicit form used to fall through to "unknown type". One unchanged asymmetry remains: a keyword followed by `(c)[]@ref` takes the primitive path and fails with "expected end of expression". The module EBNF was updated.
- `source.rs`: a bare non-keyword name records its span as `ImportName` with no `ImportReference`, while keywords stay `TypeKeyword`. `cursor.rs`/`query.rs` needed no logic change because the AST is identical; their docs were updated and tests added. `serialize.rs` writes a local reference bare unless its name is a keyword, in which case it writes `name@this`. Cross-file references are unchanged.
- Source-aware edits: no production edit path re-serializes atoms. `serialize_property_atom` is used only by `detect.rs`, which infers a whole schema, and by Claudine `inline_prompt.rs`, which renders a display table. The leaf writer `locate_frontmatter_leaves` is covered by a test showing an untouched `x@this` is not respelled.
- Passivity: `type_definition_values_accept_reference_syntax_without_lookup` validates the values `person[]` and `person(required)@./missing.yaml` in an empty tempdir context.
- Fallout: every test that relied on "unknown type" now uses genuinely malformed input such as `string(min(nope))` or `str(`, so it survives Phase 3. Two tests whose negative `!contains("unknown type")` check had become vacuous now make the positive check `contains("unknown constraint")`. The changed tests are `meta_schema_phase3.rs::semantic_defaults_use_the_passive_parser_authority`, `simplified/mod.rs::property_grammar_errors_propagate_with_property_name`, `cursor.rs::a_hard_parse_failure_still_yields_a_usable_state`, `resolve.rs::bare_map_advisory_excludes_json_schema_envelopes_and_invalid_maps`, seven `dmls lsp_session.rs` meta-schema and standalone tests, `grammar.rs::parameterized_expression_is_rejected`, and `schemas_literal_expression.rs::expression_parameterized_rejected_in_v1`.
- **Known gap, for Phase 3/4:** in a DMLS standalone schema, a half-typed keyword (`title: str`) or a missing bare name (`title: nope`) publishes no diagnostic, because the standalone authoring path never resolves named types. A missing `x@this` already behaved this way; bare names make the case common. Phase 4's missing-name diagnostic must cover the standalone path.

### Wave 3 — Grouped Match Model

- `file_match.rs` adds:
  - `FileMatchGroups { groups: Vec<FileMatchGlobs> }`, which is never empty. `new(&[G: AsRef<[String]>])`, where an empty list gives `NoPositivePattern`. It also has `groups()`, `matches()` (every group), and `prepare()`.
  - `PreparedFileMatch`, with `roots()` (the first group's roots), `lists_file()` (every group), and `may_list_below()` (every group). The first group's roots suffice, because every admitted file must be listed by the first group.
  - `root_union_match_groups(arm, property) -> Option<Vec<&[String]>>`, the crate function `file_match_groups(def)`, and `file_match_admits(value, groups: &[G], origin)`.
- `root_union_match_patterns` and `file_match_patterns` were removed. `FileMatchGlobs` is unchanged as the one-group judgment.
- Keyword decoder:
  - a flat array is one group, with byte-identical messages, so `[]` errors exactly as before;
  - an array of arrays is several groups;
  - a mixed shape, an empty group, or a non-string member is a schema error;
  - the message for several groups names each group;
  - the rejection still comes from the keyword itself, never through `allOf`.
- Compile-only consumer edits, with behavior unchanged:
  - `convert.rs` still emits the first group, as `find_map` did before.
  - Claudine `supplied.rs::admits_arm_files` passes the first group as a one-group list.
  - Test call sites in `claudine/cli/src/completion/schema_completion/tests.rs` and `darkmatter/lib/tests/l1/glob_consumers.rs` wrap their one list in a one-group list.
  - `completion.rs`, `classify.rs`, `candidates.rs`, and `schema_interactive/*` read `Constraint::Match` directly or use the unchanged `FileMatchGlobs`; Phase 3/4 migrates them.
- No-walk proof: construction takes no context. Preparing against a directory that does not exist yet still gives a judgment that lists a file created afterwards. So the prepared value holds resolved roots and patterns, not a snapshot of a walk.

### Requirement-to-test map (Phase 2 behaviors)

| Behavior | Tests |
|---|---|
| Provenance address → span (plain, quoted, `$constraints`, occurrence, whole-word keyword) | `simplified::source::tests::constraint_span_resolves_an_origin_address_in_each_presentation` |
| Path/source rendering for messages | `simplified::source::tests::schema_source_path_and_id_render_for_messages` |
| Typed error contracts (display + status block) | `errors::tests::unknown_named_type_names_the_type_the_reference_and_the_namespace`, `errors::tests::constraint_conflict_lists_every_origin_in_merge_order` |
| Bare reference parses as local; misspelled keyword is a reference; keywords win; `string@this` is a reference | `grammar.rs`: `unknown_identifier_parses_as_local_reference`, `misspelled_keyword_parses_as_local_reference`, `bare_keyword_stays_primitive_and_explicit_keyword_is_a_reference` |
| Bare and `@this` agree for every postfix form; deferred interpretation; inline objects; identifier chars; `Name(c)[]` rejected in both spellings | `bare_reference_postfixes_match_explicit_reference`, `bare_reference_records_constraints_and_array_marker`, `bare_reference_defers_suggestion_target`, `bare_reference_works_in_inline_objects`, `bare_reference_identifier_characters_match_explicit_reference`, `named_reference_rejects_array_marker_after_constraints_in_both_spellings` |
| Source projection of bare names, including documents with no `@` (plain, single/double-quoted, flow, inline object, block/flow union arms, `$schema` with offset) | `schemas_source_projection.rs`: `bare_local_reference_projects_its_name_as_an_import_name_span`, `bare_keyword_stays_a_type_keyword_span_and_explicit_this_records_both_spans`, `bare_local_reference_in_a_schema_declaration_records_import_name_spans` |
| Localized edit does not respell `@this` | `a_leaf_edit_does_not_respell_an_untouched_explicit_local_reference` |
| Cursor roles for bare names | `cursor.rs`: `a_bare_reference_name_is_a_type_position`, `bare_reference_constraint_positions_match_explicit_reference_positions`, `a_bare_reference_in_an_inline_object_keeps_its_constraint_role` |
| Suggestion query parity | `query.rs::bare_reference_suggestions_match_explicit_local_reference` |
| Serialization canonical form + round trip | `serialize.rs`: `local_reference_serializes_bare_and_round_trips`, `local_reference_with_a_keyword_name_keeps_the_explicit_spelling`, `cross_file_reference_serialization_is_unchanged`, `local_reference_inside_an_inline_object_round_trips` |
| Passive meta-type acceptance | `meta_schema_phase4.rs::type_definition_values_accept_reference_syntax_without_lookup` |
| One-group parity; several positives/exclusions; cross-group conjunction (`guide-intro.md` vs `notes.md`/`guide.txt`); group-local exclusion; empty intersection without walk; invalid group lists | `file_match.rs`: `one_group_judges_exactly_as_its_globs`, `one_group_takes_several_positives_and_exclusions`, `groups_intersect_instead_of_widening`, `an_exclusion_stays_inside_its_own_group`, `an_empty_intersection_constructs_and_prepares_without_a_walk`, `an_empty_or_invalid_group_list_is_an_error` |
| Grouped query + keyword encoding/decoding | `root_union_match_groups_keeps_every_group_in_authored_order`, `a_flat_keyword_is_one_group_with_its_message_unchanged`, `a_nested_keyword_requires_every_group`, `a_malformed_keyword_is_a_schema_error` |

- Input Robustness Matrix: not applicable to this phase. Phase 2 changed a text grammar, a renderer, and an in-process API, and read no new configuration fields. The matrix's `$constraints` table is scheduled for Phase 5. Malformed `x-darkmatter-match` shapes, the one generated-metadata decoder touched here, are covered by `a_malformed_keyword_is_a_schema_error`.

### Verification and close

- `darkmatter`: `just test` passed 9195 tests, with 11 skipped. `just lint` exited 0.
- `claudine`: `just test` passed 8673 tests, with 9 skipped. `just lint` exited 0; its only warning is the existing linker `__eh_frame` notice.
- `messenger` (it reads the schemas API): `just test` passed 710 tests, with 2 skipped.
- OS evidence: everything ran on macOS. `just cross-check darkmatter --os windows --lib file_match::tests` passed 15 of 15 on native Windows, since the grouped-glob tests compare paths. Linux and WSL2 were not run locally; CI covers them.
- Not done in this phase, by plan:
  - producing `UnknownNamedType`/`ConstraintConflict` (Phase 3);
  - emitting `x-darkmatter-match` for several groups and reading every group in completion and root-union selection (Phases 3 and 4);
  - DMLS bare-name assistance (Phase 4);
  - docs (Phase 5).
- Skill: `.claude/skills/darkmatter/schema.md` gained the grouped-match note. The `claudine` skill needs no change, because Claudine behavior is unchanged in this phase.
- Phase 2 complete: shared model, bare-reference grammar, and grouped-match model landed.

## Phase 3

- Started 2026-10-11 by claude/opus. Wave 4 ran as two parallel foreground sub-agents with disjoint ownership:
  - **Constraint Resolution** owned `resolve.rs`, `simplified/lint.rs`, and `tests/l1/schemas_named_type_merge.rs`.
  - **Conjunctive Lowering** owned `simplified/convert.rs`, `about.rs`, and `tests/l1/schemas_conjunctive_lowering.rs`.
  - The orchestrator registered both test modules in `tests/l1/main.rs` before dispatch. It then closed the `$constraints` presence gap and added the cross-pipeline checkpoint test.

### Wave 4 — Constraint Resolution (`resolve.rs`)

- **Presence:** `strip_top_level_presence` and its call are deleted, and the stale comment now states the contract. Inherited `required`/`generated` survive every reference. Presence hoisting from item and union-arm constraints was already in the converter, so `person[]` and union arms inherit `required` with no further change.
- **Provenance:** a `SchemaSourcePath` is threaded through `expand_shape`/`expand_property_def`/`expand_atom`/`expand_import`. It uses the source map's own segments: union arms are `union_arm(i)`, pattern keys use their authored key, and inline root-union arms start at `[N]`.
  - Each expanded definition carries one `SchemaConstraintOrigin` per top-level constraint. That covers value, array container, and `$constraints` entries at `path/$constraints/<key>`.
  - A named type is expanded at `[type-name]` in its own namespace. An inherited constraint keeps the origin of the declaration that introduced it; in a multi-hop chain, `min` points at `[label]` in `types.yaml`.
  - Spans are filled for `File` origins on the error path only, by re-reading and re-projecting that file. `InlineDocument` origins carry `span: None`, as designed in Phase 1.
- **Typed errors:**
  - A missing name is now `SchemaError::UnknownNamedType { name, namespace, site }`. `namespace` is the searched schema and `site` is where the reference was written.
  - Contradictions are `SchemaError::ConstraintConflict` with both winning origins, inherited first.
  - When either error comes from a referenced schema, `ReferencedSchema` wraps it (tested). Cycles keep `ImportCycle`, and a mixed bare/explicit chain renders `b -> a -> b`.
- **Merge (`apply_import_postfix` rewritten):** this follows the spec's Decided 4 table.
  - **Bounds:** stricter wins per kind: number, string length, item count, key count. Lower > upper is a `ConstraintConflict` of kind `Bounds`. Equal bounds are valid.
  - **Flags:** deduplicated.
  - **Patterns and `match`:** every pattern is kept, and every `match` group stays a separate group, inherited first.
  - **Schemes:** they intersect; an empty intersection is a conflict.
  - **Defaults:** compared with a local copy of the converter's number normalization, so `3` equals `3.0` and `"3"` differs from `3`. Different defaults are a conflict.
  - **Suggestions:** at-most-one `suggest` is checked again after union flattening, so duplicates inherited through different arms fail. The error is `Grammar` with span `0..0`, because an inherited candidate's span belongs to another file.
  - **Description:** a use-site description replaces the inherited one.
  - **Existing rejections:** union-target postfix, already-array `[]`, and array-level `suggest` are retained.
- **Target-directed bounds** (checkpoint choices 1 and 2):
  - On a `string` target, a use-site `min`/`max` becomes `MinLen`/`MaxLen`. A non-integer is a `Convert` error naming the property.
  - On an already-array target, the value-level postfix applies to the container: `min`/`max` become `MinItems`/`MaxItems`. `suggest` is kept on the items, because candidates describe items and array-level `suggest` is rejected.
- **Object key bounds:** `$constraints` key bounds, the definition's postfix key bounds, and use-site key bounds merge as one scope. The single winners are placed in `shape.constraints` and removed from the atom.
- **Examples keep their defining-file origin:** when a definition is inlined from another file, its `example(...)` references are rewritten to the resolved absolute native path, with `this` meaning the defining file. Path resolution was factored out of `resolve_one_example` into `resolve_example_path`. `FileReference` accepts the rewritten path as absolute on every OS; this passed on native Windows.
- **Lint defect found and fixed (`simplified/lint.rs`):** any schema file that put `suggest(...)` on a named reference failed to load, because the passive suggestion lint tried to convert the unresolved reference. The passive lint now skips references, and the resolved schema's lint still judges candidates against the resolved target (tested).
- **Existing tests updated, stripping assertions removed:** the resolver unit tests `expands_named_type_import`, `resolves_this_self_import`, `constrained_import_applies_use_site_constraint` (comment only), `multiple_imports_from_same_file_resolve_and_dedup`, the three `shared_type_tables` independence tests, and `import_of_missing_type_name_errors` (now `UnknownNamedType`). A search of `darkmatter/` and `claudine/` found no other test asserting stripping or the old "is not defined in" message.

### Wave 4 — Conjunctive Lowering (`convert.rs`)

- **Patterns:** an atom with exactly one pattern-producing constraint keeps today's single `pattern` keyword, so its output is byte-identical (checkpoint choice 3). With several, the base fragment gains `allOf: [{"pattern": …}, …]`, deduplicated, with no combined regex. `type`, `format`, bounds, `description`, `default`, and every `x-darkmatter-*` annotation stay on the base fragment. `coerce.rs` reads only the numberlike arm, which is unchanged. The triggers matcher already evaluates every constraint.
- **No keyword overwrites on one atom:**
  - repeated bounds keep the stricter value per kind;
  - lower > upper on one atom is a `SchemaError::Convert` backstop, since the resolver catches reuse conflicts first with origins;
  - schemes intersect, and an empty intersection is a `Convert` error;
  - defaults are compared after `normalize_json_number`;
  - examples keep authored order.
- **Object bounds:** the `$constraints` block and the postfix are one scope, for inline objects and root shapes alike.
- **`$constraints` silent loss closed** (checkpoint choice 5):
  - On a property's object, `required`/`generated` mean the same as the postfix (see the orchestrator fix below).
  - `not-empty`, `unique`, `integer`, `min-items`, and `max-items` are a `Convert` error ("`$constraints` key `…` is not valid on an inline object").
  - On the root shape or a root-union arm, every key except `min-keys`/`max-keys` is an error ("… on the root object").
  - The only shipped use is `min-keys: 1` in `darkmatter/docs/schemas/claudine-types.yaml`, which is unaffected.
- **Grouped match emission:** `attach_root_union_match` keeps its signature and emits every group in one `x-darkmatter-match` keyword. One group is a flat array, as before; several are `[["*.md"],["guide*"]]`.
  - Groups are not wrapped in `allOf`, so `validate.rs::is_match_rule_out` and `is_match_keyword_error` still work (checkpoint choice 6).
  - The keyword validator already requires every group, and `without_match_keyword`, `format.rs`, and `about.rs` handle the nested form unchanged.
  - Single-schema validation stays passive.
- `about.rs` descriptions of `min`, `max`, `not-empty`, `pattern`, `match`, and `scheme` now describe what repeats do.

### Orchestrator fixes and checkpoint

- **`$constraints` presence moved to the parser (`simplified/mod.rs::parse_mapping_atom`).** The lowering agent first hoisted block `required`/`generated` only in `convert.rs`. Phase projection (`phase.rs::project_atom`), `PropertyAtom::is_required`, the triggers matcher, and `compose` consumers still read the atom, so the block and postfix spellings disagreed:
  - at the Completion phase, a block `required` + `generated` property passed when absent;
  - at Launch, a block `required` without `eager` stayed required.
  
  The parser now moves those two flags off a property's object block onto the property's atom, so every consumer sees one representation. The converter's private hoist was removed, and lowering now rejects a block presence flag wherever one remains, which is only possible at the root or in a programmatic shape. The tests `block_required_makes_the_object_property_required_like_the_postfix` (now also asserting Launch defers) and `block_generated_defers_presence_in_authored_validation_like_the_postfix` (now also asserting Completion enforces) pin the parity.
- **Checkpoint test** `schemas_named_type_merge::reused_patterns_and_match_groups_stay_conjunctive_in_validation` runs the whole resolver → converter → validator path:
  - a reused `code(pattern(z$))@./types.yaml` over `string(pattern(^a))` accepts `az` and rejects `ab` and `bz`;
  - a root-union arm `doc(match('guide*'))@./types.yaml` over `file(match('*.md'))`, checked against real files, accepts `guide-intro.md` and rejects `notes.md` and `guide.txt`.
- **Checkpoint comparison:** the following tests show that bare, `@this`, and cross-file references agree after property names and origins are accounted for:
  - `bare_and_explicit_local_references_resolve_identically`
  - `required_and_generated_survive_every_reference_spelling`
  - `a_reference_without_presence_or_repeats_lowers_like_its_inlined_definition`
  
  Conflicts expose both declarations (`a_multi_hop_bound_conflict_names_both_origins_with_the_defining_span` and `a_constraints_block_conflict_points_at_the_block_entry`), and conjunctions survive actual validation.

### Requirement-to-test map (Phase 3 behaviors)

Unless prefixed, test names are in `darkmatter/lib/tests/l1/schemas_named_type_merge.rs` (`M`) or `schemas_conjunctive_lowering.rs` (`L`); `convert::repeated_constraints` tests are unit tests.

| AC / behavior | Tests |
|---|---|
| AC1 presence through bare/`@this`/cross-file; authored vs runtime; invalid supplied values | M `required_and_generated_survive_every_reference_spelling` |
| AC2 spike case at top level and inside a mapping type | M `required_expression_type_is_required_at_top_level_and_inside_a_mapping_type` |
| AC3 combinations, multi-hop, nested object, `person[]`, union arms | M `each_presence_combination_is_inherited_as_declared`, `multi_hop_chains_inherit_presence_locally_and_across_files`, `a_required_inner_field_does_not_require_the_enclosing_property`, `an_array_of_a_required_type_is_a_required_property`, `union_arms_hoist_inherited_required` |
| AC4 bounds per kind, target-directed interpretation, item vs container | M `string_length_bounds_are_read_against_the_resolved_target_and_keep_the_stricter_value`, `a_fractional_length_bound_on_a_string_target_is_rejected`, `number_bounds_keep_the_stricter_value`, `item_count_bounds_merge_on_an_already_array_target_and_never_with_item_bounds`, `key_bounds_from_constraints_blocks_and_postfixes_are_one_scope` |
| AC4 conflict with both origins and spans; wrapper | M `a_multi_hop_bound_conflict_names_both_origins_with_the_defining_span`, `a_constraints_block_conflict_points_at_the_block_entry`, `a_conflict_inside_a_referenced_schema_keeps_the_referenced_wrapper` |
| AC4 schemes, defaults, patterns/match groups, examples, suggestions | M `schemes_intersect_and_an_empty_intersection_is_a_conflict`, `defaults_must_be_equal_after_number_normalization`, `repeated_patterns_and_match_groups_are_all_kept`, `inherited_examples_resolve_from_their_defining_file_and_come_first`, `duplicate_suggestions_through_different_union_arms_fail`, `suggestions_on_a_reference_are_linted_against_the_resolved_target` |
| AC4 conjunctive lowering + actual validation | L `a_value_must_satisfy_every_pattern`, `not_empty_and_a_pattern_both_apply`, `several_patterns_lower_to_all_of_beside_the_base_keywords`, `repeated_bounds_keep_the_stricter_value_in_validation`, `block_and_postfix_key_bounds_merge_as_one_scope`, `equal_bounds_are_valid_and_contradictory_bounds_are_errors`, `repeated_schemes_intersect`, `repeated_defaults_compare_after_number_normalization`; M `reused_patterns_and_match_groups_stay_conjunctive_in_validation`; unit `convert::repeated_constraints::*` (11 tests) |
| AC5 bare vs explicit, built-in collision, unknown names, defining-file resolution, cycles | M `bare_and_explicit_local_references_resolve_identically`, `a_bare_keyword_is_the_built_in_and_keyword_at_this_is_the_local_type`, `an_unknown_bare_name_is_a_typed_error_naming_the_type_and_schema`, `an_unknown_name_in_a_root_union_arm_is_addressed_through_the_arm`, `an_unknown_name_inside_an_imported_definition_names_its_defining_file`, `a_bare_name_inside_an_imported_definition_resolves_in_its_defining_file`, `a_schema_file_with_only_bare_references_expands`, `a_cycle_through_bare_and_explicit_references_reports_the_chain` |
| AC10 restrictions retained; unaffected output equivalent; grouped root union; passivity | M `union_postfix_and_already_array_suffix_remain_rejected`, `a_reference_without_presence_or_repeats_lowers_like_its_inlined_definition`; L `schemas_without_repeats_keep_their_lowered_output`, `root_union_selection_requires_every_match_group`, `an_exclusion_applies_only_inside_its_own_group`, `match_groups_stay_passive_in_a_single_schema`, `several_groups_emit_nested_arrays_and_one_group_a_flat_array` |
| `$constraints` presence parity and invalid keys | L `block_required_makes_the_object_property_required_like_the_postfix`, `block_generated_defers_presence_in_authored_validation_like_the_postfix`, `block_keys_with_no_object_meaning_are_errors` |

- **Input Robustness Matrix:** the full `$constraints` matrix is scheduled for Phase 5 (Wave 7). In this phase, the `$constraints` reader's previously silent drops became defined outcomes: presence keys move to the atom, and keys with no meaning are errors. That code path was checked for `serde(default)`, `unwrap_or_default`, and `.ok()`, and none was found. The other matrix cells (null, wrong type, duplicate key, trailing content) are unchanged parser behavior and remain Phase 5's evidence.

### Verification and close

- `darkmatter`: `just test` passed 9254 tests, with 11 skipped. Nine tests were slow but passed; all are existing DMLS parity and corpus tests this phase didn't touch. `just lint` exited 0 after a `matches!` clippy fix in `convert.rs`. The `convert` lib tests were rerun after that fix: 122 of 122 passed.
- `claudine`: `just test` passed 8673 tests, with 9 skipped. `just lint` exited 0.
- `messenger` (reads the schemas API): `just test` passed 710 tests, with 2 skipped.
- OS evidence:
  - macOS: everything above.
  - Native Windows: `just cross-check darkmatter --os windows schemas_named_type_merge schemas_conjunctive_lowering` passed 48 of 48. This covers the absolute-path example rewrite and the real-file grouped root union.
  - Linux and WSL2 were not run locally; CI covers them.
- Docs: no topic docs or skills were changed in this phase. Current docs (`local-type-references.md`, `authoring-schemas.md`, `definition.md`, the `darkmatter` skill) still describe stripping and single-group behavior until Phase 5 (Wave 6) updates them, as planned. New `$constraints` key rules and same-atom contradiction errors also belong in Phase 5's docs.
- The Claudine skill needs no change, because Claudine behavior is unchanged in this phase.
- Phase 3 complete: resolution, merging, and lowering are implemented. The plan's Wave 4 tasks are checked off.

## Phase 4

- Started 2026-10-11 by claude/opus. Wave 5 ran as two parallel foreground sub-agents with disjoint ownership: **Editor Assistance** owned `darkmatter/dmls/` (plus DMLS-facing test files); **Claudine Consumers** owned `claudine/cli/src/completion/schema_completion/`, `claudine/cli/src/commands/schema_interactive/`, `claudine/lib/src/composition/schema/` and `claudine/lib/src/composition/error/` and their tests.

### Shared contract set by the orchestrator before dispatch

- `darkmatter/lib/src/markdown/schemas/completion.rs`: `CompletionKind::File { patterns: Vec<String> }` became `CompletionKind::File { alternatives: Vec<Vec<Vec<String>>> }` — an OR of AND-ed `match(...)` groups. A single property gives one alternative holding every group (a reused type adds the use site's groups to the inherited ones); a root union gives one alternative per contributing `file` arm, deduplicated. Before, every arm's patterns were concatenated into one list, which let one arm's exclusions leak into another and, with several groups, widened the conjunction into an OR. Empty means no `match` at all (bare `file`).
  - Tests updated in the same module, plus `repeated_match_groups_stay_separate_groups_of_one_alternative` and `root_union_arm_keeps_its_groups_together`; `root_union_combines_file_patterns_for_shared_property` was renamed `root_union_offers_each_file_arm_as_its_own_alternative`. 22/22 pass.
  - The only consumer outside Darkmatter's lib is Claudine's `candidates.rs`.
- `file_match::file_match_groups` became `pub` so Claudine can read every group of a property without touching `Constraint::Match` itself.

### Wave 5 — Editor Assistance (`darkmatter/dmls/`)

- **Library seam (`resolve.rs`):** two new public functions, both reusing the resolver's own path; neither adds a second resolver.
  - `named_type_reference_path(reference, base_dir, roots, ctx)` returns the canonical file an explicit `Name@target` selects (`None` for `this`). The lookup moved out of `resolve_namespace` into a private `named_type_file_path`, with no behavior change.
  - `expand_named_types(schema, base_dir, defining_file, roots, ctx)` expands a schema the editor holds in memory. It uses that schema, not the file on disk, as `this`, so an unsaved buffer resolves. Root-union arms expand one at a time with each arm's properties as `this`, as the runtime does.
  - Unit tests: `expand_named_types_reads_this_from_the_supplied_schema_not_the_file`, `named_type_reference_path_selects_the_resolver_target`.
- **Completion:**
  - Local names are offered bare. A local name that is also a built-in is offered as `name@this`, and an imported name with its target (`Name@./types.yaml`).
  - Forward references are offered.
  - Root-union, raw JSON Schema, and whole-file-reference files export nothing.
  - `meta_schema_completion_catalog_is_descriptor_driven` now expects `Base`, and asserts `Base@this` is absent.
- **Hover and go-to-definition:** both work on bare, `@this`, and cross-file references (new `providers/frontmatter/named_references.rs`, with the catalog of reachable definitions in `overlay/named_types.rs`). Hover shows the selected definition with local references spelled bare. Definition lands on the defining key in the correct file.
- **Diagnostics (`diagnostics/named_types.rs`):** `UnknownNamedType`, `ConstraintConflict`, and `ImportCycle` are schema-definition diagnostics, `dm.schema.invalid_type_definition`.
  - The primary range is the consuming reference's `ImportName` span.
  - Conflicts carry related locations for each origin: the `File` span, or for an inline origin the span from `SchemaSourceMap::constraint_span` over the projected `$schema`.
  - A block scalar falls back to the property entry's range. An import cycle carries no location, so it is ranged on the whole declaration.
  - An error inside a schema the document references by file points at the reference inside that file.
- **Standalone gap from Phase 2 closed:** a standalone schema file now resolves its references, so a missing bare name (`title: nope`) is reported at the reference. As a new consequence, an explicit target in a standalone file that cannot be found or read is reported as `dm.schema.prepare` on the target text; it was silent before.
- **Suggestion checks:** `suggest(...)` on a reference is checked against the resolved type, and an inherited value is reported in its own file (`overlay/suggestions.rs`).
- **Dependency refresh:** imported files remain dependency edges. Local references record none (`overlay::tests::local_references_record_no_external_dependency`).
  - **Pre-existing defect fixed:** a failed schema assembly was cached with no dependencies, so fixing an imported file never cleared the error. A failure now records the files its error names.
  - **Pre-existing defect fixed:** hover ranges on `$schema` definitions after the first line drifted by the indentation. That path now re-parses the value in place.
- **Known limitations, left for review:**
  - In a root-union file, completion offers every inline arm's local names, although each arm resolves `this` against its own properties. A name from another arm can therefore be offered and then reported unknown.
  - The existing "at most one `suggest`" error, raised when a type and its use site both have one, is still ranged on the whole `$schema`, because an inherited candidate's span belongs to another file.

### Wave 5 — Claudine Consumers

- **Public API:**
  - `CompositionError::UnresolvedFileReference.patterns` became `alternatives: Vec<Vec<Vec<String>>>`, with the same OR-of-AND meaning as Darkmatter's `CompletionKind::File`. It is never empty for this variant.
  - `InteractiveShape::File.patterns` became `alternatives` too. It holds one alternative with every group, or is empty for a bare `file`.
  - Crate-internal: `FileTarget.alternatives` and `FileTarget::merge` (one alternative per arm, deduplicated, arms must agree on array shape). `file_candidate_paths` takes alternatives.
- **Consumers migrated:**
  - `candidates.rs`: TAB completion and the ENTER chooser share `match_alternative_files`. It walks each alternative with `FileMatchGroups`/`PreparedFileMatch` and unions the results in authored order, deduplicated by canonical path. An alternative with invalid globs contributes nothing.
  - `supplied.rs`: `file_match_target` returns every group (it used the first `Match`). `eager_file_target` and `file_reference_target` keep each root-union arm as its own alternative instead of concatenating. `admits_arm_files` passes every group to `file_match_admits` (it passed the first).
  - `classify.rs`: collects every group (it used the first `Match`).
  - `schema_interactive/*`: passes the alternatives through.
- **Remaining single-list consumers:** none. Searched `claudine/` (lib, cli, gen, contract) for `Constraint::Match`, `x-darkmatter-match`, `DARKMATTER_MATCH_KEYWORD`, `FileMatchGlobs`, and `file_match`; the other `pattern(s)` hits (protect, runaway, OpenCode permissions, wrap guards) are unrelated.
- **Deviation from the spec example's spelling:** a `kind: schema` file cannot carry both `types` and `$schema`, which Darkmatter rejects. The local-spelling fixtures therefore define the type as a sibling property in the prompt's own `$schema`, and the cross-file fixture uses a types-only file. `darkmatter/docs/topics/schemas/authoring-schemas.md` (around line 157) shows the rejected combination: Phase 5 doc drift.

### Requirement-to-test map (Phase 4 behaviors)

DMLS tests are in `darkmatter/dmls/tests/l1/named_type_references.rs` (`N`); Claudine tests are named with their module.

| AC / behavior | Tests |
|---|---|
| AC6 completion: bare local, `@this` collision, imported target, forward reference, no exports from root unions/raw/whole-file | N `completion_offers_local_names_bare_collisions_with_this_and_imports_with_their_target`; dmls unit `meta_schema_completion_catalog_is_descriptor_driven` |
| AC6 hover on bare/`@this`/cross-file references | N `hover_on_a_reference_shows_the_definition_it_selects` |
| AC6 go-to-definition selects the defining file and key | N `definition_selects_the_defining_file_and_key_for_every_reference_spelling` |
| AC6 missing name ranged on the reference (standalone, inline plain/quoted/flow/explicit, block-scalar fallback, referenced file) | N `a_missing_local_name_in_a_standalone_schema_is_reported_at_the_reference`, `a_missing_name_in_an_inline_schema_is_a_schema_definition_error_at_the_reference`, `a_block_scalar_reference_falls_back_to_the_property_entry`, `an_error_inside_a_referenced_schema_points_into_that_file` |
| AC6 contradiction with related locations; mixed bare/explicit cycle chain | N `a_multi_hop_contradiction_names_the_reference_and_both_declarations`, `a_standalone_contradiction_and_a_mixed_cycle_are_schema_definition_errors` |
| AC6 imported-definition edit refreshes completion/hover/definition/diagnostics; local refs add no edge | N `editing_an_imported_definition_refreshes_every_assistance_without_reopening`; dmls unit `overlay::tests::local_references_record_no_external_dependency` |
| Suggestion checks on references | N `suggestions_on_a_reference_are_checked_against_the_resolved_target` |
| Library seam | `resolve::tests::expand_named_types_reads_this_from_the_supplied_schema_not_the_file`, `resolve::tests::named_type_reference_path_selects_the_resolver_target` |
| Grouped completion metadata (one alternative of every group; one alternative per root-union arm) | darkmatter `completion::tests::repeated_match_groups_stay_separate_groups_of_one_alternative`, `root_union_arm_keeps_its_groups_together`, `root_union_offers_each_file_arm_as_its_own_alternative` |
| AC10 `guide-intro.md` offered, `notes.md`/`guide.txt` not (cross-file, bare, `@this`; TAB, chooser, typed partial) | `completion::schema_completion::tests::reused_match_groups_offer_only_files_every_group_admits`; CLI L1 `compose_schema_cli::completion_file_match_intersects_a_reused_types_groups` |
| AC10 exclusion stays in its group and arm | `completion::schema_completion::tests::a_match_exclusion_stays_with_its_own_group_and_arm` |
| AC10 empty intersection offers nothing, no error | `completion::schema_completion::tests::an_empty_match_intersection_offers_nothing_without_an_error`; `composition::schema::tests::an_empty_match_intersection_parses_and_reports_the_partial` |
| One file admitted by two arms offered once | `completion::schema_completion::tests::a_file_two_arms_admit_through_different_roots_is_offered_once` |
| AC3 inherited `required` + every group at the Claudine boundary | `composition::schema::tests::a_reused_file_type_keeps_inherited_presence_and_every_match_group` |
| AC10 single-schema passivity | `composition::schema::tests::match_groups_stay_passive_for_an_existing_file_in_a_single_schema` |
| AC10 root-union arm selection needs every group, judged from the caller's origin | `composition::schema::supplied::tests::an_existing_caller_file_selects_an_arm_only_when_every_match_group_admits_it`. The sub-agent confirmed it fails when only the first group is passed. |

- Input Robustness Matrix: not applicable to this phase. It read no new configuration fields; the changes are an editor projection and in-process consumers of already-parsed schemas. The `$constraints` matrix remains Phase 5 (Wave 7).

### Verification and close

- `darkmatter`: `just test` passed 9272 tests with 11 skipped; `just lint` exited 0, including the `zed-dmls` wasm check.
- `claudine`: `just test` passed 8682 tests with 9 skipped; `just lint` exited 0.
- `messenger` (reads the schemas API): `just test` passed 710 tests with 2 skipped.
- All four gates above were rerun by the orchestrator on the combined tree after both sub-agents finished, sequentially with lint after tests.
- OS evidence: macOS for everything above. Native Windows: `just cross-check dmls --os windows named_type_references` passed 11/11 (canonical-path URIs), and `just cross-check claudine-cli --os windows schema_completion` passed 51/51 (grouped walk, canonical-path dedup). Linux and WSL2 were not run locally; CI covers them.
- Docs: `claudine/docs/topics/completions/shell-completions.md` described the old behavior, concatenating root-union arms' globs. It now describes grouped intersection, group-local exclusions, empty intersections, and per-arm alternatives. All other docs remain Phase 5 (Wave 6) work. Phase 5 must cover, for `dmls-schema-support.md`:
  - completion spellings and non-exporting files;
  - hover and definition on references;
  - the three diagnostics with their ranges and related locations;
  - standalone resolution and the new `dm.schema.prepare` for unreadable targets;
  - suggestion checks on use sites;
  - refresh after imported-file edits, including clearing cached failures;
  - the two known limitations.

  For Claudine docs, it must cover the renamed `alternatives` fields and their OR-of-AND meaning.
- Skill: the `claudine` skill's own files do not describe `match()` completion; its `topics/` link is a symlink to the doc updated above, so no skill file changed.
- Phase 4 complete: both Wave 5 tasks are checked off in the plan.

## Phase 5

- Started 2026-10-11 by claude/opus. Wave 6 ran as two parallel sub-agents: **Artifact Audit** (read-only probe; no artifact edits) and **Current Documentation** (docs and skills only). Wave 7 ran as three parallel foreground sub-agents with disjoint files: the corpus check (`darkmatter/lib/tests/l1/shipped_schema_corpus.rs`), the `$constraints` robustness matrix (`darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs`), and the invocation fixtures (`darkmatter/cli/tests/l1/schema_validate.rs` plus a new `schema_validate_phase.rs`). The orchestrator declared both new lib modules in `tests/l1/main.rs` before dispatch.
- Deviation from the plan's wave order: Wave 7 started while the audit was still running. Its tests do not depend on the docs, and the audit changed no artifact, so no Wave 7 result needed to be redone.

### Wave 6 — Artifact Audit

**Method.** A temporary probe linked against `darkmatter` was built on this tree and on a read-only worktree at the pre-feature commit `f1d031f7a`, and both were run on the same input files; the probe and worktree were then removed. For each artifact the probe compared:
- the whole-file resolution through `DarkmatterSchemas::effective_for` (`$schema: ./<file>`): full JSON Schema, empty-frontmatter verdict, and `completion::for_property` for every completable property;
- every `types`/`$schema` entry referenced on its own as `name@./<file>` ("synthetic": many are never referenced in practice);
- 929 Markdown files with a frontmatter `$schema`, by JSON hash and empty-frontmatter verdict.

| Path | References | Presence on referenced types | Duplicates / collisions / `$constraints` | Before vs after | Disposition |
|---|---|---|---|---|---|
| `darkmatter/docs/schemas/claudine-types.yaml` | 7 `@this` | none at top level; use sites `lifecycle-action(required)@this`, `lifecycle-action[](required)@this` | `lifecycle-action` has `$constraints: min-keys: 1`; use site adds only `required` | identical (file and all 7 types) | unchanged |
| `darkmatter/docs/schemas/claudine.yaml` | 9 `@./claudine-types.yaml` | none on referenced types; own `state`, `is_first`, `step`, `_loop_*` are `generated; required` but unreferenced | none | whole-file JSON identical; completion `sequence` is `File { alternatives: [] }` (the Phase 4 rename; still a bare file) | unchanged |
| `darkmatter/docs/schemas/darkmatter.yaml` (embedded baseline) | none | none | none | identical | unchanged |
| `darkmatter/docs/schemas/env.yaml` | none; unreferenced anywhere | all `required; generated` or `generated` | none | synthetic references now inherit presence; no real consumer | unchanged (inheritance is the intended meaning) |
| `darkmatter/docs/schemas/err.yaml` | none | all `required; generated` | none | not loadable on either tree (envelope keys `$schema`, `name`, `description`) | unchanged |
| `darkmatter/docs/schemas/expression-functions.yaml`, `schema-definition.yaml` (empty) | none | n/a | none | `expression-functions.yaml` fails "could not build JSON Schema validator" on both trees | unchanged |
| `darkmatter/schemas/darkmatter.yaml` + `partials/*` | 6 `@./partials/…` | `err`/`context` inner-field presence only | none | not loadable on either tree (tagged files with `description`/`$schema`) | unchanged |
| `content-policy/schemas/content-policy.yaml` | 3 `@./content-policy.yaml` | none on `short_form`/`long_form`; `long_form.action` inner only | none | identical | unchanged |
| `messenger/docs/platforms.schema.yaml`, `messenger/docs/research/platforms/{_schema,_types,_overrides.schema}.yaml`, `messenger/docs/research/implementation/_schema.yaml` (**missed by Phase 1**) | ~87 `@this`/`@./` | none on any referenced type; presence always at the use site (`Id(required)@this` ×59) | none; no keyword-named types | whole-file JSON identical | unchanged |
| `claudine/docs/research/_types.yaml`, 20 `*/_schema.yaml`, `agent-cli/_types.yaml`, `reasoning-level/_types.yaml` (**missed by Phase 1**) | ~49, incl. `@../_types.yaml` | none at top level; `evidence` inner only | none | whole-file JSON identical; the 20 research documents using `./_schema.yaml` unchanged | unchanged |
| `claudine/schemas/{claudine,review}.yaml`, `claudine/schemas/partials/lifecycle.yaml`, `.claudine/schemas/review.yaml`, root `schemas/*.yaml`, `darkmatter/example-docs/schemas/*` | none | `external.yaml` `name: string(required)` unreferenced | none | identical; `partials/lifecycle.yaml` not loadable on either tree | unchanged |
| `claudine/docs/schemas/**` (drafts) | bare `stack_item`, `terminal_node`, `stack`; `shell@./shell.yaml` (missing); `shell_command @ this` | inner only | `partials/functions.yaml` local `number`/`date` collide with built-ins (built-in wins) | no draft loadable on either tree; 4 empty | recorded only |
| Feature snapshots (lifecycle-grammar spike, steering-pipeline research-schema, schema-enhancements drafts, `_completed` schema-plus) | many | none at top level | `$constraints` only `min-keys`/`max-keys` | identical | unchanged (snapshots) |
| `messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/**` | many | `p3_types` `State: enum(…; required)`; `p5_types` `Knowledge` required union arm | none | **differs**: `p3` `items[].state` and `p5` `facts[].knowledge` now required; `p5_missing.md` now fails validation (passed before) | unchanged (spike snapshot; the intended inheritance) |
| Inline `$schema` in 929 shipped Markdown files | only the messenger probes above and `darkmatter/docs/research/parsing/_fleet_future.md` | — | no `$constraints` key besides `min-keys`/`max-keys` | 925 identical; `_fleet_future.md` error went from Grammar "unknown type `crate`" to `UnknownNamedType` (expected); 3 messenger probes differ as above | unchanged |

- **Changes made: none.** No shipped schema relied on stripping, so no optional variant was needed. No source comment or test needed editing.
- **Stale stripping search:** no `*.rs`, current doc, skill, or README still asserts or describes stripping. The 7 Phase 1 resolver tests now assert inheritance. The nullable assertions in `schemas_named_type_merge.rs` cover generated-only, unconstrained, and inner-required cases, which are correct. Snapshots that mention stripping were left alone under the snapshot rule: lifecycle-grammar `spike-results.md` Finding 10, messenger schema-pilots `findings.md` L4, and the union-type-constraints spec/plan.
- **Readers run as evidence (nothing changed):** darkmatter L1 `schema_phase_validation`, `meta_schema_repo_schemas`, `content_policy_editor_schema`, `base_schema_end_to_end`, `meta_schema_phase4` (51 passed); dmls `mapping_only_corpus` (2); claudine-cli `shipped_prompts` (2); messenger `research_corpus` + `research_validation` (15); content-policy (117).
- **Changed artifact owner areas:** none. Audited areas: darkmatter, claudine, content-policy, messenger, repo-root `schemas/` and `.claudine/schemas/`.

### Wave 6 — Current Documentation

- `darkmatter/docs/topics/schemas/local-type-references.md` (rewritten): bare names ≡ `@this`; where local names live; built-ins win; case sensitivity; forward references; `UnknownNamedType`; cycles and the depth limit of 32; a Mermaid diagram of defining-file resolution; inheritance; canonical serialization.
- `authoring-schemas.md`: removed both status notes; `min(n)`/`max(n)` grammar; "Presence travels with the type" (optional outer object vs required inner fields, no opt-out, declare the optional variant directly, generated phases); nullable unions marked **planned**.
- `definition.md`: named-type imports with bare names; the full merge table and rules (target-directed bounds, item vs container, description precedence, conflict provenance, one-atom contradictions); a `$constraints` block table; several patterns lowering to `allOf`; grouped `match`; `CompletionKind::File { alternatives }`; the two new error variants; nullable unions **planned**.
- `dmls-schema-support.md`: completion spellings, hover/definition on every spelling, named-type diagnostics with ranges and related locations, `dm.schema.prepare` for unreadable targets, `suggest` checks, refresh after imported-file edits, the two known limitations, and the two new `resolve::` functions.
- `darkmatter/docs/inline/schema-validation.md`: a root-union `match` requires every group.
- `claudine/docs/topics/composition.md`: `UnresolvedFileReference.alternatives` and `InteractiveShape::File.alternatives` (OR over root-union arms, AND over one property's groups).
- Skills: `.claude/skills/darkmatter/schema.md`, `.claude/skills/darkmatter/dmls.md`, and `.claude/skills/claudine/cli-commands.md` (it still described merged globs, `MatchGlobs`, and `match_glob_files`).
- `darkmatter/README.md` was not changed: it does not describe the affected behavior. No dependency docs changed.
- **Doc drift found and resolved (code wins):**
  1. `authoring-schemas.md` showed `kind: schema` with `$schema` + `types` and a helpers/exports split. `standalone.rs` accepts exactly `kind` and `types`, and there is no "no exported schema" error. The examples were corrected.
  2. **Root-union namespace:** the spec says an inline root union keeps "today's empty `@this` namespace". The code, both now and at `f1d031f7a`, expands each inline arm as its own single schema, so `this` is that arm's own entries and nothing is shared across arms. This is not a departure introduced by the feature, since today's behavior is retained. The docs describe the per-arm behavior, and a root-union *file* still exports no names.
  3. `number(min(1),max(150))` (comma) is a grammar error; the examples now use `;`.
  4. `definition.md` said root unions return no completion data; `completion.rs` handles them.
  5. `related_specs` frontmatter and spec links were removed from `definition.md`, `schema-validation.md`, and `composition.md` (current docs must not name specs).

### Wave 7 — Corpus Validation

`darkmatter/lib/tests/l1/shipped_schema_corpus.rs`:

| Test | Asserts | macOS / Windows time |
|---|---|---|
| `shipped_schema_artifacts_parse_passively` | 35 YAML files in 8 recursive directories (`darkmatter/docs/schemas`, `darkmatter/schemas`, `content-policy/schemas`, `schemas`, `claudine/schemas`, `.claudine/schemas`, `darkmatter/example-docs/schemas`, `claudine/docs/schemas`) parse passively, or fail exactly as their `EXCEPTIONS` entry says. Floors: 30 files, 10 loadable, 120 definitions | 0.14 s / — |
| `loadable_shipped_schemas_resolve_and_lower` | each loadable file resolves through `$schema: ./file` from its own directory, anchored in an empty temp context, and lowers; rejected envelopes still fail with `SchemaDocument`, and their bodies resolve inline with no `UnknownNamedType`/`ConstraintConflict` | 0.1 s / — |
| `shipped_research_contracts_resolve_and_lower` (added by the orchestrator after the audit found the gap) | 30 `_schema.yaml`/`_types.yaml`/`*.schema.yaml` research contracts in `claudine/docs/research` and `messenger/docs` parse passively, resolve with no named-type error, and lower. Floors: 30 files, 550 definitions (623 today), 330 named references (375 today). No effect counter moves and no input changes | 0.21 s / 0.33 s |
| `shipped_markdown_inline_schemas_parse_passively` | 648 frontmatter `$schema` values from 16 shipped roots parse passively (only files containing `$schema` are decoded; `fixtures`, `target`, `node_modules`, `.git` and the internal `prompts/` folders are skipped). The 4 known named-reference documents are found. Floor: 500 | 1.0 s / 3.0 s |
| `shipped_markdown_named_type_references_resolve` | those 4 resolve from their own directories; `_fleet_future.md` fails with `UnknownNamedType { name: "crate" }`, and none of the others fails on a named type | 1.0 s / 2.9 s |

- **Passivity is observed:** the effect-engine and network-attempt counters must not move. They count only under `effects-instrumentation`, which CI enables. A standalone parse is repeated with the file's path moved into an empty directory and must give the same result, and every input's bytes and mtime are unchanged afterwards. **Not provable with existing instrumentation:** no counter covers expression evaluation, and no recorder covers filesystem lookups, so "no file search" is shown by result independence, not by an absence of reads.
- `EXCEPTIONS` is an explicit, reasoned list (placeholders, not-a-schema, envelope-rejected, drafts with their expected error). An entry that no longer matches its file fails the test.
- `schema_phase_validation.rs::shipped_schema_and_trigger_corpus_parses_passively` is now redundant for schemas. Its trigger half (`darkmatter/tests/fixtures/schema-triggers/schemas/*.trigger.yaml`) is not covered elsewhere, so the test was left in place.

### Wave 7 — Input Robustness Matrix (`$constraints`)

`darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs`: `constraints_block_matrix_yaml` and `constraints_block_matrix_json_form`, one per accepted source format.
- **Fixture:** both tests read the shipped `darkmatter/docs/schemas/claudine-types.yaml` with `include_str!` inside the test, check that `lifecycle-action` still has `$constraints: {min-keys: 1}`, and make one source-text edit per cell into a fresh `types-N.yaml`.
- **Sites:** each cell is judged reused (`lifecycle-action(...)@./types-N.yaml` with a use-site postfix) and inline (the same object as a plain property).
- **Validation path:** documents load through `Markdown::try_from(&Path)` and are judged with `DarkmatterSchemas::validate` and `validate_for_phase(Completion)`. Assertion labels are `{format} / {column} / {row} / {site}`.
- **Load-bearing fields:**
  - `min-keys`/`$constraints`: an empty object fails only with the bound present.
  - `max-keys`: the block becomes `max-keys: 2`, so a 3-key object fails only with the bound present, and `max-keys: 0` is a real zero.
  - `required`: a missing `action` fails only with the flag present.
  - `generated`: authored validation allows a missing `action` only with the flag, and the Completion phase rejects it either way.

Observed (YAML and JSON-form are the same except where noted):

| Row | `$constraints` | `min-keys` / `max-keys` | `required` / `generated` |
|---|---|---|---|
| Positive control | positive | positive | positive |
| Absent | B; use-site bound applies | B; use-site bound applies | B; use-site flag applies |
| Explicit null | E "`$constraints` must be a mapping" | E "requires a non-negative integer" | E "is a flag constraint and must be `true`" |
| Wrong whole type | E (number, string, list) | E (`"1"`, `1.5`, `-1`, `true`, object) | E (`"true"`, `1`, object) |
| One invalid element | E (unknown key `bogus`; invalid `max-keys`) | E (`[1,"x"]`) | E (`[true,"x"]`) |
| All invalid elements | E (never an empty block) | E (`["x","y"]`, `[1]`) | E (`["x"]`, `[true]`) |
| Empty | `{}` = B; `[]` E | E (`""`, `[]`, `{}`); `0` valid | E (`""`, `[]`, `{}`, `false`) |
| Duplicate key (source edit) | E | E | E |
| Trailing / invalid content | E | E | E |

- Every E names `types-N.yaml` (reused site) or property `action` (inline site). Duplicate keys and trailing content in a schema file surface as "referenced schema file `types-N.yaml` is neither a valid standalone SimplifiedSchema nor a valid JSON Schema". In frontmatter they surface as "duplicate entry with key …".
- No cell silently accepted bad input, so there was **no production change**.
- **Smell grep:** `simplified/mod.rs` has no `serde(default)`, `unwrap_or_default`, or `filter_map(as_str)`. Its one `.ok()` is the integer-size conversion in `constraints_usize`, and that failure becomes an error.
- **Out-of-scope findings, not changed:**
  - `standalone.rs::parse_standalone_schema_document` discards the YAML parse error, so a malformed schema file gets the generic "neither …" message with no line or column.
  - In the same function, `filter_map(as_str)` drops non-string keys from the "unsupported keys" message, though the file is still rejected.
  - `Markdown::from(String)` treats frontmatter that fails to parse as *no frontmatter*. A document whose inline `$schema` has a duplicate key therefore validates with no schema; `Markdown::try_from(&Path)` errors correctly. This is a pre-existing loader behavior outside this feature.

### Wave 7 — Invocation Fixtures (`md schema validate`)

The shared fixture is a types-only `types.yaml` (`code: string(required; generated)`, `title: string(required)`, `label: string(min(3))`, `title_base: label`) plus inline `$schema` reuse in all three spellings. All tests run through `--no-trigger-schemas --format json`.

| Test (`darkmatter-cli` L1 `schema_validate::`) | Asserts |
|---|---|
| `schema_validate_inherited_required_is_reported_for_every_reference_spelling` | exit 1; exactly `/a /b /c /title` missing; no generated property reported |
| `schema_validate_supplied_required_and_absent_generated_pass_authored_validation` | exit 0 with generated properties absent |
| `schema_validate_rejects_a_wrong_typed_value_through_every_generated_spelling` | exit 1; `code ga gb gc` each `[1] is not of type "string"` |
| `schema_validate_enforces_inherited_and_use_site_length_bounds_across_hops` | exit 1; `/heading` longer than 5; `/hop` and `/title_base` shorter than 3 |
| `schema_validate_conjunctive_patterns_{accept_a_value_matching_both, reject_a_value_missing_the_use_site_pattern, reject_a_value_missing_the_inherited_pattern}` | `az` exit 0; `ab` fails `z$`; `bz` fails `^a` |
| `schema_validate_multi_hop_bound_contradiction_names_both_origins` | exit 2, `error:"schema"`; message names `heading`, `title_base`, `min` in the types path, `max` in the inline `$schema` |
| `schema_validate_unknown_bare_name_in_an_inline_schema_is_a_schema_error`, `…_in_a_referenced_schema_names_that_file` | exit 2; ``unknown named type `nosuchtype` in …`` |
| `schema_validate_grouped_match_arm_accepts_a_path_every_group_admits`, `…_rejects_a_path_outside_the_use_site_group`, `…_rejects_a_path_outside_the_inherited_group`, `…_rules_out_the_arm_for_an_undecided_document`, `schema_validate_grouped_match_is_passive_in_a_single_schema` | real files `guide-intro.md` accepted; `notes.md`/`guide.txt` rejected with `match(*.md)` and `match(guide*)`; an undecided document falls to the other arm; a plain schema accepts `guide.txt` |
| shipped `darkmatter/docs/schemas/claudine.yaml`: `…_accepts_reused_lifecycle_and_loop_types`, `…_rejects_an_empty_lifecycle_action`, `…_rejects_values_inside_a_reused_loop_event` | positive control exit 0; `{}` action fails (`min-keys: 1` applies through reuse); bad `op` enum and `max: 0` fail |

- `md schema validate` runs authored validation only and has no `--phase` flag. The new `darkmatter/cli/tests/l1/schema_validate_phase.rs` therefore checks the same document in the library: `schema_validate_phase_absent_generated_passes_authored_and_fails_completion` and `schema_validate_phase_wrong_typed_generated_value_fails_both_phases`.
- The shipped path is built with `biscuit_test_harness::manifest_dir!()` plus `.parent()` steps rather than `env!("CARGO_MANIFEST_DIR")`, per the `rust-testing` rule against compile-time paths.
- **Production observations, not changed (for review):**
  1. The `ConstraintConflict` message names the bound kinds (`min`, `max`) and their origins, but not the values (`min(10)`, `max(5)`); DMLS related locations show the authored text.
  2. The pretty CLI output word-wraps the types-file path with an inserted hyphen.
  3. Problems in nested values report the line of the top-level key.

### Pre-existing performance observation

The corpus agent found that `claudine/features/2026-10-20-lifecycle-grammar/spike/fixtures/variants/lifecycle-d{1..4}.md` resolve in time that doubles per nesting depth. With the debug `md`, d1/d2/d3 took 1.65/3.71/7.07 s. The Oct 3 pre-feature `md` from the main checkout took 1.36/3.00/8.96 s, so this is **not a regression from this feature**. The fixtures are excluded from the corpus (a `fixtures` directory). Per the plan's spike-scope rule, no measurement or fix was attempted; it is recorded for a separate decision.

### Requirement-to-test map (Phase 5)

| Requirement | Evidence |
|---|---|
| AC7 shipped-schema and test-suite audits recorded; no stripping assertion | Wave 6 audit table and stale-stripping search above |
| AC9 shipped schemas parse passively | `shipped_schema_corpus::*` (5 tests) |
| AC9 `md schema validate` proves inherited presence and conjunction through normal resolution | `schema_validate::*` tests above |
| AC1 authored vs effective phases | `schema_validate_phase::*` (library, since the CLI has no phase flag) |
| AC4 multi-hop contradiction names both origins (CLI) | `schema_validate_multi_hop_bound_contradiction_names_both_origins` |
| AC5 unknown bare name (CLI) | `schema_validate_unknown_bare_name_in_an_inline_schema_is_a_schema_error`, `…_in_a_referenced_schema_names_that_file` |
| AC10 grouped root union and single-schema passivity (CLI) | `schema_validate_grouped_match_*` |
| Input Robustness Matrix (`$constraints`, YAML and JSON-form) | `schemas_constraints_block_matrix::constraints_block_matrix_yaml`, `…_json_form` |
| Shipped-schema positive control with invalid document values | `schema_validate_…claudine…` three tests |

### Verification and close

- `darkmatter`: `just test` passed 9299 tests with 11 skipped; `just lint` exited 0.
- `claudine`: `just test` passed 8682 tests with 9 skipped; `just lint` exited 0. This phase changed only Claudine docs and a skill.
- Feature-flag run: `cargo nextest run -p darkmatter --test l1 --features effects-instrumentation shipped_research_contracts` passed, and the corpus agent ran its tests with and without the feature.
- Native Windows: `just cross-check darkmatter --os windows shipped_schema_corpus schemas_constraints_block_matrix` passed 8/8, and `just cross-check darkmatter-cli --os windows schema_validate` passed 71/71. **Timing note:** on Windows, the matrix tests take about 3.6 s and the Markdown corpus scans about 3 s, under the 5 s L1 budget but closer to it than on macOS (0.35 s and 1 s). Linux and WSL2 were not run locally; CI covers them.
- `messenger` `just test` was not run as a gate in this phase because no schema it reads changed. The audit ran its `research_corpus` and `research_validation` (15 passed). Phase 6 runs it as planned.
- Phase 5 complete: all four Wave 6/7 tasks are checked off in the plan.
