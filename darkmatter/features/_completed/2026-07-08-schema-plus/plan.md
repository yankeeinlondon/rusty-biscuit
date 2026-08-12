---
agent: codex/
total_phases: 7
created: 2026-07-09
phase: 7
yolo: true
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/completion.rs
  - darkmatter/lib/src/markdown/schemas/detect.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/cli/src/commands/schema/assignment.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/Cargo.toml
docs_updated_during_phase_3:
  - darkmatter/docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/schemas/example.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/features/2026-07-08-schema-plus/as_unordered_list-example-fm.yaml
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
docs_updated_during_phase_7:
  - darkmatter/docs/topics/schema-definition.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/SKILL.md
source_code:
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/completion.rs
  - darkmatter/lib/src/markdown/schemas/detect.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/example.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/lib/Cargo.toml
  - darkmatter/cli/src/commands/schema/assignment.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/features/2026-07-08-schema-plus/as_unordered_list-example-fm.yaml
documentation:
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/docs/dependencies.md
  - docs/dependencies.md
packages:
  - darkmatter
---

# SimplifiedSchema Composition Primitives Execution Plan

Success means SimplifiedSchema supports `example(...)`, `Name@file` / `Name@this` imports, pattern keys with object arity constraints, and `yaml` / `json` content-format types; the schema-plus fixtures validate; existing schemas remain compatible; and `just test darkmatter` plus targeted L2 validation pass.

Assumptions:

- The closure request listed `agent` twice with conflicting values (`codex` and `codex/`). This plan uses the last requested value, `codex/`, to keep valid YAML frontmatter.
- The requested save path contained `darkmatter//darkmatter`, but the actual functional spec is at `darkmatter/features/2026-07-08-schema-plus/spec.md` in this package area.
- DMLS cache behavior is out of implementation scope here, but dependency edges must be represented so downstream DMLS invalidation can consume them later.

## Phase 1 - Baseline Discovery and Test Scaffolding

- [x] Confirm the active workspace package list with `sniff repo` and note the `darkmatter`, `darkmatter-cli`, and `dmls` crate names and paths.
- [x] Read the current simplified-schema modules: `darkmatter/lib/src/markdown/schemas/simplified/{types.rs,grammar.rs,convert.rs,mod.rs,serialize.rs}`, `darkmatter/lib/src/markdown/schemas/{resolve.rs,format.rs,validate.rs,coerce.rs}`, and existing schema tests.
- [x] Record the existing parser/converter behavior for representative schemas before edits by running targeted unit tests or snapshot tests that cover primitive types, inline objects, arrays, `file`, root unions, and validation coercion.
- [x] Add failing parser tests for `example(./a.yaml, ./b.yaml)`, `type@./types.yaml`, `parameter[]@./types.yaml`, `type(required)@this`, `<string>`, `<starting::x->`, `<ending::.md>`, `<pattern::[0-9_]$>`, `$constraints`, `yaml`, and `json`.
- [x] Add failing converter/resolver tests for unresolved imports, named import expansion, import cycles, pattern-key JSON Schema output, literal-key precedence, min/max property counts, and content-format output.
- [x] Add fixture-validation tests for `darkmatter/features/2026-07-08-schema-plus/{example.yaml,types.yaml,today-example.yaml,as_unordered_list-example.yaml,as_unordered_list-example-2.yaml,as_unordered_list-example-fm.yaml}`.
- [x] Validation checkpoint: confirm the new tests fail for the expected missing features, not because of unrelated fixture path or test harness issues.

Parallelizable after this phase:

- Parser/AST changes, fixture migration, and content-format validation can proceed independently once the baseline failing tests exist.
- Import resolution should wait until the AST can represent imported type expressions.

## Phase 2 - Extend the SimplifiedSchema AST and Grammar

- [x] Add `Constraint::Example(Vec<String>)` or a dedicated file-reference wrapper to `types.rs`, keeping raw authored strings in the parser layer and deferring resolution to `resolve.rs`.
- [x] Add `Constraint::MinKeys(usize)` and `Constraint::MaxKeys(usize)` to `types.rs`, with canonical keywords `min-keys` and `max-keys`.
- [x] Add `SimplifiedType::Yaml` and `SimplifiedType::Json`, with keywords `yaml` and `json`, and update keyword round-trip tests.
- [x] Add an explicit imported-type AST variant, such as `TypeExpr::Imported { name, reference, is_array, constraints }`, or an equivalent structure that preserves `Name@file`, `Name[]@file`, and `Name(constraints)@file` until resolution. (Used `TypeExpr::Imported { name, reference }`; `is_array`/`constraints` ride on the enclosing `PropertyAtom`, the existing carriers for postfix `[]`/`()`.)
- [x] Introduce a schema-object key model that can distinguish literal keys from pattern keys, such as `SchemaKey::Literal(String)` and `SchemaKey::Pattern(PatternKey)`, while preserving deterministic declaration order. (Used the equivalent structure: literal keys stay in `SchemaShape.properties`; pattern keys collect in `SchemaShape.pattern_keys: Vec<PatternKeyDef>`, each preserving declaration order.)
- [x] Update the string lexer/parser to recognize terminal `@fileref` after the base identifier plus optional `[]` and optional constraints.
- [x] Update inline-object key parsing to accept quoted or bracketed pattern forms exactly as authored: `<string>`, `<starting::PREFIX>`, `<ending::SUFFIX>`, and `<pattern::RE>`.
- [x] Update constraint parsing so `example(...)` accepts comma-separated file references, `min-keys(n)` and `max-keys(n)` accept non-negative integer arguments, and invalid arity/type combinations produce `SchemaError::Grammar`.
- [x] Update YAML-shape parsing in `simplified::mod` so `$constraints` inside schema block objects is stripped from property matching and converted into the same canonical `Constraint` variants as postfix constraints.
- [x] Ensure `$constraints` remains reserved only in schema-authoring objects; do not add any global validation rule that rejects user frontmatter containing `$constraints`.
- [x] Keep existing schemas byte-equivalent where they do not use the new syntax by avoiding changes to legacy parser output and converter ordering.
- [x] Validation checkpoint: parser and AST tests pass, and legacy parser/converter snapshot output is unchanged except where test names intentionally cover new syntax.

Parallelizable in this phase:

- `Constraint` and primitive keyword additions can be done alongside pattern-key AST work.
- `$constraints` block-form parsing can be implemented alongside string-parser work once the canonical constraint variants exist.

## Phase 3 - Implement Pattern-Key Objects and Object Arity Conversion

- [x] Update object conversion so literal schema keys continue to lower into JSON Schema `properties`.
- [x] Lower `<string>` catch-all keys into `additionalProperties: <value_schema>`.
- [x] Lower `<starting::PREFIX>`, `<ending::SUFFIX>`, and `<pattern::RE>` keys into `patternProperties`, using ECMA-262-compatible regex strings.
- [x] Preserve closed-object semantics for pattern-keyed objects by emitting `additionalProperties: false` unless a `<string>` catch-all is present.
- [x] Implement literal-key precedence by wrapping each emitted non-catch-all pattern with a negative lookahead that excludes literal property names that could otherwise double-validate. (The linear `regex` engine rejects lookaround, so `build_validator` selects `jsonschema`'s `fancy-regex` engine per-schema — only when a `patternProperties` key carries a lookaround — keeping every other schema on the ReDoS-safe linear engine.)
- [x] Add conversion errors when a user-supplied `<pattern::RE>` cannot be wrapped into a valid emitted pattern. (`validate_emitted_pattern` compiles the emitted regex under the same engine schema validation uses and returns `SchemaError::Convert` on failure.)
- [x] Emit `minProperties` and `maxProperties` for `min-keys` and `max-keys`, whether authored via postfix constraints or `$constraints`.
- [x] Reject `min-keys` / `max-keys` when applied to non-object atoms or array-level constraint positions where they would be ambiguous. (Non-object atoms hit the per-type `invalid_constraint` guard; array-level positions hit `apply_array_constraints`' rejection.)
- [x] Add tests for mixed literal and pattern keys, multiple pattern keys, catch-all plus specific pattern keys, exact one-key dictionary objects, and invalid pattern wrapping.
- [x] Validation checkpoint: generated JSON Schema validates representative literal, pattern, catch-all, and exact-one-key documents exactly as specified.

Parallelizable in this phase:

- Pattern conversion and object arity constraints can be implemented independently once the object-key AST is available.

## Phase 4 - Implement Cross-File Named-Type Imports

- [x] Add a resolver pass that expands `TypeExpr::Imported` before `to_json_schema` runs, while keeping `to_json_schema` able to fail loudly if an unresolved import reaches conversion. (`expand_document_imports` in `resolve.rs`, wired into all three post-parse `to_json_schema` call sites; `to_json_schema`'s existing `Imported` rejection is retained and re-tested by `unresolved_import_reaching_conversion_is_a_convert_error`.)
- [x] Resolve import file references with `biscuit_file::FileReference::resolve_from(base_dir)`, matching existing `$schema` file-reference behavior in `resolve.rs`. (`ImportEngine::resolve_namespace`.)
- [x] Implement `@this` as the current schema file target, including inline top-level schema documents loaded from disk. (`@this` targets the current `Namespace`; the inline document root uses `NamespaceKey::Root`.)
- [x] Define named types as the target file's top-level `$schema:` entries and reject imports from raw JSON Schema files or files without a matching named type. (`load_named_types` returns `AmbiguousReferenced` for non-SimplifiedSchema targets; a missing named type is a `Convert` error.)
- [x] Apply postfix composition outward: `Name[]@file` becomes array-of-expanded-`Name`; `Name(constraints)@file` applies constraints to the expanded value type. (`apply_import_postfix`; verbatim substitution when no postfix is present.)
- [x] Cycle-check named-type imports with a stack of `(resolved_path, type_name)` entries and return a structural recursion error for direct or transitive cycles. (`ImportEngine::stack` keyed by `(NamespaceKey, name)`; new `SchemaError::ImportCycle`.)
- [x] Enforce bounded eager expansion so import chains cannot bypass the existing inline-object depth protections indefinitely. (`MAX_IMPORT_DEPTH` cap alongside the cycle stack.)
- [x] Track import dependency edges in the resolved schema product or a sibling metadata structure so the schema cache and DMLS index can invalidate on imported-file changes later. (`ResolvedSchema.imports: Vec<PathBuf>`, deduped + sorted; downstream invalidation wiring deferred.)
- [x] Ensure root-union file refs and named-type imports share file resolution semantics without conflating their error messages. (Both route through `FileReference::resolve_from`; import failures surface as `ImportCycle` / `Convert` / `AmbiguousReferenced`, distinct from root-union `FrontmatterShape` wording.)
- [x] Add tests for cross-file import, `@this`, array import, constrained import, missing file, missing type name, import from non-SimplifiedSchema file, unresolved conversion, and cycle detection.
- [x] Validation checkpoint: `example.yaml` can resolve `type@./types.yaml` and `parameter[]@./types.yaml` into the ratified structural shapes. (Proven by `expands_named_type_import` / `expands_array_named_type_import`, which resolve the exact `types.yaml` shapes; `example.yaml`'s own use of these imports is the Phase 6 fixture migration.)

Parallelizable in this phase:

- Dependency-edge recording can be implemented alongside import expansion after the resolver has a central import context.
- Error-message tests can be added while expansion is being implemented, then tightened once exact wording settles.

## Phase 5 - Implement Content-Format Types and Coercion

- [x] Add format constants `darkmatter-yaml` and `darkmatter-json` in `schemas/format.rs`.
- [x] Lower `SimplifiedType::Yaml` and `SimplifiedType::Json` to `{ "type": "string", "format": "darkmatter-yaml" }` and `{ "type": "string", "format": "darkmatter-json" }`.
- [x] Register custom format validators that parse string values via biscuit-file YAML and JSON parsing facilities; `yaml` must accept JSON strings and YAML strings, while `json` must reject YAML-only syntax. (`yaml` reuses `biscuit_file::Yaml::from_str`; `json` uses strict `serde_json::from_str` since biscuit-file exposes no strict-JSON type.)
- [x] Extend the schema coercion/normalization path so native mappings, sequences, and scalars can be serialized to YAML or JSON strings for validation against these content-format schemas. (New `CoercionTarget::{ToYamlString,ToJsonString}`, recognized from the `format` seam ahead of the plain `string` → `ToString` match.)
- [x] Keep validation-only APIs non-mutating by validating against a transient coerced copy and leaving the caller's original frontmatter value untouched. (`coerce_frontmatter` is pure; `EffectiveSchema::validate*` coerce a working copy only.)
- [x] Expose serialized write-back values only through the existing composing/write-back normalization path. (No new mutation surface; compose's `coerce_frontmatter_with_pending` write-back picks up the new targets automatically.)
- [x] Add tests for valid YAML strings, valid JSON strings, YAML-only strings rejected by `json`, invalid YAML/JSON strings, native mapping coercion, native sequence coercion, scalar coercion, and non-mutating validation-only behavior.
- [x] Validation checkpoint: the `{ frontmatter: yaml }` union arm in `example.yaml` accepts both authored string YAML and the native mapping shape in `as_unordered_list-example-fm.yaml`. (`frontmatter_yaml_union_arm_accepts_string_and_native_mapping` proves the union arm; the full fixture wiring is the Phase 6 migration.)

Parallelizable in this phase:

- Format registration and converter output can be implemented independently from native-value coercion.

## Phase 6 - Implement `example(...)` Resolution and Fixture Migration

- [x] Update `darkmatter/features/2026-07-08-schema-plus/example.yaml` to remove inherited `type` declarations from the example envelope schema and keep `invocation` as `string | { frontmatter: yaml }`. (Already in the corrected O-A1 shape from the prior fixture-alignment commit; verified byte-equivalent to the new built-in `EXAMPLE_ENVELOPE_YAML` via `example_envelope_matches_fixture`.)
- [x] Update `today-example.yaml` and all `as_unordered_list-example*.yaml` fixtures to remove redundant `type` fields. (Already absent post-alignment; verified.)
- [x] Replace `parameters: null` with omission or a nullable schema decision that is explicitly represented and tested. (No fixture carries `parameters: null`; `today-example.yaml` omits it and the `as_unordered_list` fixtures use real single-key `parameters`, validated by the `parameter[]` shape check.)
- [x] Fix stale `as_unordered_list` fixture prose so native arrays are described as native arrays and list strings are described as list strings. (Fixed the `-fm` fixture: "native numeric array" + removed the stray "in an configs:" fragment.)
- [x] Resolve `Constraint::Example` file references relative to the referencing schema file, including magic paths and `this`. (`resolve_one_example` in `resolve.rs`: `this` → the current schema file, everything else via `FileReference::resolve_from(base_dir)`.)
- [x] Validate each referenced example at schema-load time against the corrected `example.yaml` envelope. (`example::validate_example_bytes` → `validate_example_object`, run during `resolve_document_examples`, fail-loud.)
- [x] Validate example `parameters` against the inherited target shape for frontmatter context-variable examples. (Layer 2 in `validate_example_object`: `parameters` validates against the built-in `parameter[]` = single-key-map shape, O-A4.)
- [x] Add the JSON Schema extension `x-darkmatter-example` containing resolved example objects so downstream consumers do not re-read example files. (`convert.rs` emits the raw refs onto `x-darkmatter-example`; `resolve.rs` replaces them with resolved objects.)
- [x] Add content-hash caching or reuse the existing schema cache shape so unchanged example artifacts are not repeatedly revalidated on warm loads. (Process-wide `example_cache` keyed by `biscuit_hash::xx_hash_bytes` of the file bytes.)
- [x] Return schema-load errors for missing, malformed, or invalid example files; do not downgrade these to warnings. (Missing → `Unresolved`/`Io`; malformed/invalid → new `SchemaError::InvalidExample`.)
- [x] Add tests for multiple examples, relative paths, `@this`, missing example file, malformed example envelope, invalid inherited parameters, and emitted `x-darkmatter-example`. (`schema_plus_phase1` Feature A section + `example::tests`.)
- [x] Validation checkpoint: all schema-plus YAML fixtures validate through the same public schema-load path expected by CLI and library callers. (The four corpus fixtures now validate through `validate_example_object` — the same path `resolve_document_examples` uses — and are no longer `#[ignore]`-gated.)

Parallelizable in this phase:

- Fixture migration can happen while `Constraint::Example` resolution is being built.
- JSON Schema extension emission can proceed once example validation produces resolved objects.

## Phase 7 - Integration, Compatibility, and Release Readiness

- [x] Run targeted library tests for simplified-schema parser, converter, resolver, format validators, coercion, and fixture validation.
- [x] Run `just test darkmatter` from the repo root, using nextest through the repo recipe.
- [x] Run `just test-l2` inside the `darkmatter` package area for CLI-level schema validation if the targeted unit suite passes.
- [x] Run `just lint` inside the `darkmatter` package area after tests are green.
- [x] Add or update public docs for SimplifiedSchema syntax covering `example(...)`, imports, pattern keys, `$constraints`, `yaml`, and `json`.
- [x] Update `darkmatter/.claude/skills/darkmatter/SKILL.md` if the public schema architecture, workflows, or authority boundaries changed.
- [x] Update any relevant `docs/dependencies.md` files only if new crates are added; otherwise confirm no dependency-doc change is needed.
- [x] Review changed rustdoc and inline comments for drift, especially around `SimplifiedType`, `Constraint`, `resolve.rs`, format validators, and coercion.
- [x] Confirm no `cargo fmt` or write-mode `rustfmt` was run; match local style manually.
- [x] Validation checkpoint: existing SimplifiedSchema documents without new syntax parse and compile equivalently, schema-plus fixtures pass, and build/test/lint status is recorded with exact commands and outcomes.

Final handoff checklist:

- [x] Include implementation notes describing any deviations from this plan and why they were necessary.
- [x] Include the exact validation commands run and their pass/fail results.
- [x] Call out any deferred DMLS cache invalidation wiring as downstream work, with the dependency-edge data shape documented.

## Phase 7 Implementation Notes

### Validation commands and outcomes

| Command (run in `darkmatter/` area unless noted)      | Result |
|-------------------------------------------------------|--------|
| `just test` (repo `just test darkmatter`, nextest)    | 5142 passed, 111 skipped, **2 pre-existing host-specific failures** (`layout::page::tests::render_code_block_with_pad_fill`, `render_code_block_center_aligned_with_max_fill`) — both unrelated to schema work (`layout/page.rs` unmodified vs `main`; documented flakes on this host) |
| `cargo nextest run -p darkmatter --lib 'markdown::schemas'` | 509 passed, 0 failed |
| `just test-l2`                                        | darkmatter (19) + dmls (30) L2 tests all passed |
| `just lint` (clippy `--all-targets -- -D warnings`)   | clean for `darkmatter`, `darkmatter-cli`, `dmls` |
| `cargo check -p darkmatter --lib`                     | clean |

### Deviations

- **No behavior changes in Phase 7.** All Phase-7 source edits were comment/rustdoc-only drift fixes surfaced by a review pass, not logic changes: `format.rs` module doc (added the Feature-D `darkmatter-yaml`/`darkmatter-json` validators to the header and corrected the "Two validators" count), `grammar.rs` EBNF (added `yaml`/`json` type names, the `import` and `pattern_key` productions, and `fileref`), `convert.rs` (`inline_object_fragment` no longer "always" emits `additionalProperties: false` — a `<string>` catch-all overrides it; `reject_unsupported` doc now lists the full `required`/`default`/`generated`/`example` universal set), and `types.rs` (`TypeExpr` doc now enumerates the third `Imported` arm).
- **`docs/dependencies.md` (root and area) needed no Phase-7 change** — the new `fancy-regex` lib dependency (Phase 3) was already documented in the root doc; the area doc does not enumerate lib crates.
- **Editor rust-analyzer flagged `compose/schema_validation.rs` matches on `TypeExpr` as non-exhaustive (E0004).** These are stale IDE diagnostics — `cargo check`, `just test`, and `just lint` all compile clean, so the `Imported` arm is handled by the real compiler.

### Deferred downstream work — DMLS cache invalidation

Named-type import dependency edges are already materialized but **DMLS live-index invalidation on imported-file changes is deferred** (out of this plan's scope per the plan assumptions). The dependency-edge data shape downstream wiring must consume:

- `ResolvedSchema.imports: Vec<PathBuf>` — deduped, sorted absolute paths of every `Name@file` import resolved for a schema (Phase 4). Each entry is an edge "this schema depends on that file"; when any listed path changes, the base-schema LRU cache entry and the DMLS `uses_schema`/graph snapshot for the depending document should be invalidated (DMLS already content-hashes files for this purpose).
- `Constraint::Example(...)` referenced files are validated at schema-load time and cached by `biscuit_hash::xx_hash_bytes` of the file bytes; the same content-hash key is the invalidation signal for example artifacts.
