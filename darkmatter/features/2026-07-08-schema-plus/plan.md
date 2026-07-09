---
agent: codex/
total_phases: 7
created: 2026-07-09
phase: 1
yolo: true
---

# SimplifiedSchema Composition Primitives Execution Plan

Success means SimplifiedSchema supports `example(...)`, `Name@file` / `Name@this` imports, pattern keys with object arity constraints, and `yaml` / `json` content-format types; the schema-plus fixtures validate; existing schemas remain compatible; and `just test darkmatter` plus targeted L2 validation pass.

Assumptions:

- The closure request listed `agent` twice with conflicting values (`codex` and `codex/`). This plan uses the last requested value, `codex/`, to keep valid YAML frontmatter.
- The requested save path contained `darkmatter//darkmatter`, but the actual functional spec is at `darkmatter/features/2026-07-08-schema-plus/spec.md` in this package area.
- DMLS cache behavior is out of implementation scope here, but dependency edges must be represented so downstream DMLS invalidation can consume them later.

## Phase 1 - Baseline Discovery and Test Scaffolding

- [ ] Confirm the active workspace package list with `sniff repo` and note the `darkmatter`, `darkmatter-cli`, and `dmls` crate names and paths.
- [ ] Read the current simplified-schema modules: `darkmatter/lib/src/markdown/schemas/simplified/{types.rs,grammar.rs,convert.rs,mod.rs,serialize.rs}`, `darkmatter/lib/src/markdown/schemas/{resolve.rs,format.rs,validate.rs,coerce.rs}`, and existing schema tests.
- [ ] Record the existing parser/converter behavior for representative schemas before edits by running targeted unit tests or snapshot tests that cover primitive types, inline objects, arrays, `file`, root unions, and validation coercion.
- [ ] Add failing parser tests for `example(./a.yaml, ./b.yaml)`, `type@./types.yaml`, `parameter[]@./types.yaml`, `type(required)@this`, `<string>`, `<starting::x->`, `<ending::.md>`, `<pattern::[0-9_]$>`, `$constraints`, `yaml`, and `json`.
- [ ] Add failing converter/resolver tests for unresolved imports, named import expansion, import cycles, pattern-key JSON Schema output, literal-key precedence, min/max property counts, and content-format output.
- [ ] Add fixture-validation tests for `darkmatter/features/2026-07-08-schema-plus/{example.yaml,types.yaml,today-example.yaml,as_unordered_list-example.yaml,as_unordered_list-example-2.yaml,as_unordered_list-example-fm.yaml}`.
- [ ] Validation checkpoint: confirm the new tests fail for the expected missing features, not because of unrelated fixture path or test harness issues.

Parallelizable after this phase:

- Parser/AST changes, fixture migration, and content-format validation can proceed independently once the baseline failing tests exist.
- Import resolution should wait until the AST can represent imported type expressions.

## Phase 2 - Extend the SimplifiedSchema AST and Grammar

- [ ] Add `Constraint::Example(Vec<String>)` or a dedicated file-reference wrapper to `types.rs`, keeping raw authored strings in the parser layer and deferring resolution to `resolve.rs`.
- [ ] Add `Constraint::MinKeys(usize)` and `Constraint::MaxKeys(usize)` to `types.rs`, with canonical keywords `min-keys` and `max-keys`.
- [ ] Add `SimplifiedType::Yaml` and `SimplifiedType::Json`, with keywords `yaml` and `json`, and update keyword round-trip tests.
- [ ] Add an explicit imported-type AST variant, such as `TypeExpr::Imported { name, reference, is_array, constraints }`, or an equivalent structure that preserves `Name@file`, `Name[]@file`, and `Name(constraints)@file` until resolution.
- [ ] Introduce a schema-object key model that can distinguish literal keys from pattern keys, such as `SchemaKey::Literal(String)` and `SchemaKey::Pattern(PatternKey)`, while preserving deterministic declaration order.
- [ ] Update the string lexer/parser to recognize terminal `@fileref` after the base identifier plus optional `[]` and optional constraints.
- [ ] Update inline-object key parsing to accept quoted or bracketed pattern forms exactly as authored: `<string>`, `<starting::PREFIX>`, `<ending::SUFFIX>`, and `<pattern::RE>`.
- [ ] Update constraint parsing so `example(...)` accepts comma-separated file references, `min-keys(n)` and `max-keys(n)` accept non-negative integer arguments, and invalid arity/type combinations produce `SchemaError::Grammar`.
- [ ] Update YAML-shape parsing in `simplified::mod` so `$constraints` inside schema block objects is stripped from property matching and converted into the same canonical `Constraint` variants as postfix constraints.
- [ ] Ensure `$constraints` remains reserved only in schema-authoring objects; do not add any global validation rule that rejects user frontmatter containing `$constraints`.
- [ ] Keep existing schemas byte-equivalent where they do not use the new syntax by avoiding changes to legacy parser output and converter ordering.
- [ ] Validation checkpoint: parser and AST tests pass, and legacy parser/converter snapshot output is unchanged except where test names intentionally cover new syntax.

Parallelizable in this phase:

- `Constraint` and primitive keyword additions can be done alongside pattern-key AST work.
- `$constraints` block-form parsing can be implemented alongside string-parser work once the canonical constraint variants exist.

## Phase 3 - Implement Pattern-Key Objects and Object Arity Conversion

- [ ] Update object conversion so literal schema keys continue to lower into JSON Schema `properties`.
- [ ] Lower `<string>` catch-all keys into `additionalProperties: <value_schema>`.
- [ ] Lower `<starting::PREFIX>`, `<ending::SUFFIX>`, and `<pattern::RE>` keys into `patternProperties`, using ECMA-262-compatible regex strings.
- [ ] Preserve closed-object semantics for pattern-keyed objects by emitting `additionalProperties: false` unless a `<string>` catch-all is present.
- [ ] Implement literal-key precedence by wrapping each emitted non-catch-all pattern with a negative lookahead that excludes literal property names that could otherwise double-validate.
- [ ] Add conversion errors when a user-supplied `<pattern::RE>` cannot be wrapped into a valid emitted pattern.
- [ ] Emit `minProperties` and `maxProperties` for `min-keys` and `max-keys`, whether authored via postfix constraints or `$constraints`.
- [ ] Reject `min-keys` / `max-keys` when applied to non-object atoms or array-level constraint positions where they would be ambiguous.
- [ ] Add tests for mixed literal and pattern keys, multiple pattern keys, catch-all plus specific pattern keys, exact one-key dictionary objects, and invalid pattern wrapping.
- [ ] Validation checkpoint: generated JSON Schema validates representative literal, pattern, catch-all, and exact-one-key documents exactly as specified.

Parallelizable in this phase:

- Pattern conversion and object arity constraints can be implemented independently once the object-key AST is available.

## Phase 4 - Implement Cross-File Named-Type Imports

- [ ] Add a resolver pass that expands `TypeExpr::Imported` before `to_json_schema` runs, while keeping `to_json_schema` able to fail loudly if an unresolved import reaches conversion.
- [ ] Resolve import file references with `biscuit_file::FileReference::resolve_from(base_dir)`, matching existing `$schema` file-reference behavior in `resolve.rs`.
- [ ] Implement `@this` as the current schema file target, including inline top-level schema documents loaded from disk.
- [ ] Define named types as the target file's top-level `$schema:` entries and reject imports from raw JSON Schema files or files without a matching named type.
- [ ] Apply postfix composition outward: `Name[]@file` becomes array-of-expanded-`Name`; `Name(constraints)@file` applies constraints to the expanded value type.
- [ ] Cycle-check named-type imports with a stack of `(resolved_path, type_name)` entries and return a structural recursion error for direct or transitive cycles.
- [ ] Enforce bounded eager expansion so import chains cannot bypass the existing inline-object depth protections indefinitely.
- [ ] Track import dependency edges in the resolved schema product or a sibling metadata structure so the schema cache and DMLS index can invalidate on imported-file changes later.
- [ ] Ensure root-union file refs and named-type imports share file resolution semantics without conflating their error messages.
- [ ] Add tests for cross-file import, `@this`, array import, constrained import, missing file, missing type name, import from non-SimplifiedSchema file, unresolved conversion, and cycle detection.
- [ ] Validation checkpoint: `example.yaml` can resolve `type@./types.yaml` and `parameter[]@./types.yaml` into the ratified structural shapes.

Parallelizable in this phase:

- Dependency-edge recording can be implemented alongside import expansion after the resolver has a central import context.
- Error-message tests can be added while expansion is being implemented, then tightened once exact wording settles.

## Phase 5 - Implement Content-Format Types and Coercion

- [ ] Add format constants `darkmatter-yaml` and `darkmatter-json` in `schemas/format.rs`.
- [ ] Lower `SimplifiedType::Yaml` and `SimplifiedType::Json` to `{ "type": "string", "format": "darkmatter-yaml" }` and `{ "type": "string", "format": "darkmatter-json" }`.
- [ ] Register custom format validators that parse string values via biscuit-file YAML and JSON parsing facilities; `yaml` must accept JSON strings and YAML strings, while `json` must reject YAML-only syntax.
- [ ] Extend the schema coercion/normalization path so native mappings, sequences, and scalars can be serialized to YAML or JSON strings for validation against these content-format schemas.
- [ ] Keep validation-only APIs non-mutating by validating against a transient coerced copy and leaving the caller's original frontmatter value untouched.
- [ ] Expose serialized write-back values only through the existing composing/write-back normalization path.
- [ ] Add tests for valid YAML strings, valid JSON strings, YAML-only strings rejected by `json`, invalid YAML/JSON strings, native mapping coercion, native sequence coercion, scalar coercion, and non-mutating validation-only behavior.
- [ ] Validation checkpoint: the `{ frontmatter: yaml }` union arm in `example.yaml` accepts both authored string YAML and the native mapping shape in `as_unordered_list-example-fm.yaml`.

Parallelizable in this phase:

- Format registration and converter output can be implemented independently from native-value coercion.

## Phase 6 - Implement `example(...)` Resolution and Fixture Migration

- [ ] Update `darkmatter/features/2026-07-08-schema-plus/example.yaml` to remove inherited `type` declarations from the example envelope schema and keep `invocation` as `string | { frontmatter: yaml }`.
- [ ] Update `today-example.yaml` and all `as_unordered_list-example*.yaml` fixtures to remove redundant `type` fields.
- [ ] Replace `parameters: null` with omission or a nullable schema decision that is explicitly represented and tested.
- [ ] Fix stale `as_unordered_list` fixture prose so native arrays are described as native arrays and list strings are described as list strings.
- [ ] Resolve `Constraint::Example` file references relative to the referencing schema file, including magic paths and `this`.
- [ ] Validate each referenced example at schema-load time against the corrected `example.yaml` envelope.
- [ ] Validate example `parameters` against the inherited target shape for frontmatter context-variable examples.
- [ ] Add the JSON Schema extension `x-darkmatter-example` containing resolved example objects so downstream consumers do not re-read example files.
- [ ] Add content-hash caching or reuse the existing schema cache shape so unchanged example artifacts are not repeatedly revalidated on warm loads.
- [ ] Return schema-load errors for missing, malformed, or invalid example files; do not downgrade these to warnings.
- [ ] Add tests for multiple examples, relative paths, `@this`, missing example file, malformed example envelope, invalid inherited parameters, and emitted `x-darkmatter-example`.
- [ ] Validation checkpoint: all schema-plus YAML fixtures validate through the same public schema-load path expected by CLI and library callers.

Parallelizable in this phase:

- Fixture migration can happen while `Constraint::Example` resolution is being built.
- JSON Schema extension emission can proceed once example validation produces resolved objects.

## Phase 7 - Integration, Compatibility, and Release Readiness

- [ ] Run targeted library tests for simplified-schema parser, converter, resolver, format validators, coercion, and fixture validation.
- [ ] Run `just test darkmatter` from the repo root, using nextest through the repo recipe.
- [ ] Run `just test-l2` inside the `darkmatter` package area for CLI-level schema validation if the targeted unit suite passes.
- [ ] Run `just lint` inside the `darkmatter` package area after tests are green.
- [ ] Add or update public docs for SimplifiedSchema syntax covering `example(...)`, imports, pattern keys, `$constraints`, `yaml`, and `json`.
- [ ] Update `darkmatter/.claude/skills/darkmatter/SKILL.md` if the public schema architecture, workflows, or authority boundaries changed.
- [ ] Update any relevant `docs/dependencies.md` files only if new crates are added; otherwise confirm no dependency-doc change is needed.
- [ ] Review changed rustdoc and inline comments for drift, especially around `SimplifiedType`, `Constraint`, `resolve.rs`, format validators, and coercion.
- [ ] Confirm no `cargo fmt` or write-mode `rustfmt` was run; match local style manually.
- [ ] Validation checkpoint: existing SimplifiedSchema documents without new syntax parse and compile equivalently, schema-plus fixtures pass, and build/test/lint status is recorded with exact commands and outcomes.

Final handoff checklist:

- [ ] Include implementation notes describing any deviations from this plan and why they were necessary.
- [ ] Include the exact validation commands run and their pass/fail results.
- [ ] Call out any deferred DMLS cache invalidation wiring as downstream work, with the dependency-edge data shape documented.
