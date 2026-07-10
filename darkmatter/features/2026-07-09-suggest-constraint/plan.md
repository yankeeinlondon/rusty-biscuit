---
agent: codex/
total_phases: 8
created: 2026-07-09
phase: 8
yolo: true
source_files_during_phase_1:
  - darkmatter/lib/tests/suggest_constraint_phase1.rs
  - darkmatter/dmls/tests/suggest_constraint_phase1.rs
  - darkmatter/dmls/tests/fixtures/suggest_constraint/inline.md
  - darkmatter/dmls/tests/fixtures/suggest_constraint/pure.yaml
  - darkmatter/dmls/tests/fixtures/suggest_constraint/tagged.yaml
  - darkmatter/dmls/tests/fixtures/suggest_constraint/completion.md
  - darkmatter/dmls/tests/fixtures/suggest_constraint/unions.md
  - darkmatter/dmls/tests/fixtures/suggest_constraint/raw-schema.json
  - darkmatter/dmls/tests/fixtures/suggest_constraint/raw-consumer.md
docs_updated_during_phase_1:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
  - darkmatter/docs/topics/context-variables.md
docs_created_during_phase_1:
  - darkmatter/features/2026-07-09-suggest-constraint/phase1-baseline.md
  - darkmatter/features/2026-07-09-suggest-constraint/test-matrix.md
skills_files_updated_during_phase_1: []
packages:
  - darkmatter
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/tests/suggest_constraint_phase1.rs
  - darkmatter/lib/tests/suggest_constraint_phase2.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/tests/snapshots/suggest_constraint_phase3__conversion_snapshot_covers_valid_and_invalid_metadata.snap
  - darkmatter/lib/tests/suggest_constraint_phase1.rs
  - darkmatter/lib/tests/suggest_constraint_phase2.rs
  - darkmatter/lib/tests/suggest_constraint_phase3.rs
docs_updated_during_phase_3:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/tests/suggest_constraint_phase4.rs
docs_updated_during_phase_4:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_5:
  - darkmatter/dmls/Cargo.toml
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/suggestions.rs
  - darkmatter/dmls/tests/suggest_constraint_phase1.rs
docs_updated_during_phase_5:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/schemas/simplified/query.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/suggest_constraint_phase1.rs
docs_updated_during_phase_6:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
  - darkmatter/lib/src/markdown/schemas/simplified/query.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
docs_updated_during_phase_7:
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
  - darkmatter/features/2026-07-09-suggest-constraint/test-matrix.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
source_code:
  - darkmatter/lib/tests/suggest_constraint_phase1.rs
  - darkmatter/lib/tests/suggest_constraint_phase2.rs
  - darkmatter/lib/tests/suggest_constraint_phase3.rs
  - darkmatter/lib/tests/suggest_constraint_phase4.rs
  - darkmatter/lib/tests/snapshots/suggest_constraint_phase3__conversion_snapshot_covers_valid_and_invalid_metadata.snap
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/query.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/suggestions.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/suggest_constraint_phase1.rs
  - darkmatter/dmls/tests/fixtures/suggest_constraint/inline.md
  - darkmatter/dmls/tests/fixtures/suggest_constraint/pure.yaml
  - darkmatter/dmls/tests/fixtures/suggest_constraint/tagged.yaml
  - darkmatter/dmls/tests/fixtures/suggest_constraint/completion.md
  - darkmatter/dmls/tests/fixtures/suggest_constraint/unions.md
  - darkmatter/dmls/tests/fixtures/suggest_constraint/raw-schema.json
  - darkmatter/dmls/tests/fixtures/suggest_constraint/raw-consumer.md
documentation:
  - darkmatter/features/2026-07-09-suggest-constraint/plan.md
  - darkmatter/features/2026-07-09-suggest-constraint/test-matrix.md
  - darkmatter/features/2026-07-09-suggest-constraint/phase1-baseline.md
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/docs/topics/context-variables.md
  - .claude/skills/darkmatter/SKILL.md
---

# Suggested Values for SimplifiedSchema Execution Plan

Success means exact `string` and `number` schemas can author one non-empty `suggest(...)` list; Darkmatter preserves ordered, interpreted suggestions as non-validating `x-darkmatter-suggest` metadata and exposes span-bearing lint results; standalone SimplifiedSchema envelopes resolve consistently; DMLS warns at the authoring argument and completes only valid suggestions at every specified YAML position; raw JSON Schema remains behaviorally distinct; and the Darkmatter/DMLS test and lint recipes pass.

Assumptions:

- The closure request lists `agent` twice with conflicting values. This plan uses the last requested value, `codex/`, so the frontmatter remains valid YAML with one authoritative key.
- Candidate spans are authoring-source byte spans. The string grammar may initially produce expression-relative spans, but a library-owned source-aware parse product must project them through YAML quoting/escaping and frontmatter offsets before lint results reach DMLS.
- The supported JSON numeric model is the one used by the existing `serde_json::Number`/JSON Schema pipeline. Tests will discover its exact lossless boundary by canonical round trip instead of encoding a presumed integer width or floating-point limit.
- Existing JSON Schema validator construction remains the authority for candidate constraint checks. The implementation will remove `x-darkmatter-suggest` from a candidate's target fragment and validate the interpreted candidate rather than duplicate `min`, `max`, `integer`, length, and pattern semantics.
- No filename pattern or configured glob will classify standalone SimplifiedSchema documents; only the pure and tagged content envelopes in the functional specification may claim a YAML document.

## Phase 1 - Baseline Mapping and Acceptance Test Scaffolding

- [x] Confirm the active package names and paths with `sniff repo` and `cargo metadata --no-deps --format-version 1`, then record the applicable `darkmatter` library and `dmls` test recipes without inferring workspace membership from directories.
- [x] Trace the current schema flow through `darkmatter/lib/src/markdown/schemas/simplified/{types.rs,grammar.rs,mod.rs,serialize.rs,convert.rs}`, `schemas/{resolve.rs,validate.rs,completion.rs,about.rs}`, and the compose schema-validation call sites; identify every exhaustive `Constraint` match and cache/hash input that a new constraint must update.
- [x] Trace DMLS schema assembly, open-document parsing, diagnostics, and value completion through `darkmatter/dmls/src/overlay/{frontmatter.rs,schema.rs}`, `diagnostics/{frontmatter.rs,codes.rs}`, `providers/frontmatter.rs`, the document store/corpus, and existing LSP session fixtures.
- [x] Add a requirement-to-test matrix beside the feature tests that maps all 24 acceptance criteria to a unit, snapshot, or DMLS L2 test and identifies the source file responsible for each behavior.
- [x] Add focused failing grammar tests for eligible scalar/array forms, empty `suggest()`, unsupported types, duplicate interpreted candidates, and a second `suggest(...)` within a single atom or across property-union atoms.
- [x] Add focused failing numeric tests for simple-decimal syntax, canonicalization, exact JSON round trips, positive/negative lossless boundaries, long fractions, leading/trailing zeros, negative zero, and normalization-created duplicates; derive boundary fixtures by observable round trip rather than hard-coded platform assumptions.
- [x] Add failing conversion/lint tests for ordered `x-darkmatter-suggest` output, string versus number interpretation, invalid-metadata fallback strings, candidate constraint failures, array item targets, and non-validating document behavior.
- [x] Add failing DMLS fixtures for exact inline and standalone warning ranges plus scalar, nested-object, block-array, and flow-array completion, including property/root union selection and raw JSON Schema exclusion.
- [x] Validation checkpoint: run the narrow new test filters with `cargo nextest run --color=never` or the package `just` recipes and confirm failures are attributable to missing `suggest(...)` behavior, not fixture paths, YAML parsing, line endings, or harness setup.

Parallelizable after the baseline is captured: numeric interpretation tests, standalone-envelope fixtures, and DMLS completion-position fixtures can be prepared independently. AST changes must land before converter, lint, or DMLS consumers compile.

## Phase 2 - Span-Bearing Suggestion AST and Grammar

- [x] Add a dedicated suggestion candidate type in `simplified/types.rs` that retains decoded text, interpreted `serde_json::Value`, canonical decimal text when applicable, and the exact authored argument `SourceSpan`; add `Constraint::Suggest` using this type without conflating it with `Members` or `Example`.
- [x] Extend the existing argument lexer/parser in `simplified/grammar.rs` so `suggest(...)` reuses current comma, quote, and escape rules, rejects an empty list structurally, and captures each argument's expression-relative source span before interpretation.
- [x] Implement target-directed string interpretation so every decoded token becomes a JSON string and quoted/bare equivalent spellings compare as the same candidate.
- [x] Implement a platform-independent simple-decimal normalizer using string operations: validate the exact grammar, remove redundant integer zeros and trailing fractional zeros, remove an empty fractional component, and normalize every signed zero to `0` without first converting through a machine number.
- [x] Implement lossless numeric interpretation by parsing canonical text through the supported JSON number model, canonically serializing it, expanding serializer exponent notation to exact decimal text, renormalizing, and accepting a JSON number only when the canonical strings are byte-identical; retain syntax-invalid decoded strings and representability-invalid canonical strings as metadata.
- [x] Reject duplicate candidates after target-directed interpretation at the later argument's span, covering quoted/bare strings and every specified decimal normalization equivalence.
- [x] Enforce eligibility on exact `string` and `number` primitives (including item constraints on their array forms) and reject `suggest(...)` on array-level constraints, imports that do not resolve to an eligible exact type, and every unsupported primitive.
- [x] Enforce at most one `suggest(...)` across the complete `PropertyDef`, including all atoms of a property union, while keeping declarations of the same property in separate root-union arms independent.
- [x] Introduce or extend the source-aware SimplifiedSchema parse product so expression-relative candidate spans can be projected through plain, single-quoted, and double-quoted YAML scalar decoding into document-relative byte spans for inline frontmatter and standalone YAML.
- [x] Update `simplified/serialize.rs`, compose cache hashing, equality/clone behavior, schema detection fallbacks, and all exhaustive `Constraint` matches so suggestion metadata round-trips deterministically without changing legacy schema output.
- [x] Validation checkpoint: grammar, span-projection, canonical-decimal, duplicate, serializer, and legacy round-trip tests pass; tests include LF, CRLF, escaped quoted scalars, and multibyte UTF-8 before candidate arguments.

Parallelizable in this phase: decimal normalization/round-trip logic and YAML source-span projection can be implemented independently, then joined in the candidate parser. Cardinality enforcement can proceed once `Constraint::Suggest` exists.

## Phase 3 - Generated Annotation and Library-Owned Candidate Linting

- [x] Extend `simplified/convert.rs` to emit `x-darkmatter-suggest` on the annotated scalar schema or array `items` schema, preserving interpreted declaration order and JSON scalar types, and never emitting standard `examples` or `x-darkmatter-example` for this feature.
- [x] Keep invalid number metadata in the generated annotation as its specified string fallback while allowing SimplifiedSchema conversion, validator construction, frontmatter validation, and composition to continue.
- [x] Define a public structured lint API with a stable problem type containing decoded text, interpreted value, reason/category, property/root-arm provenance, and exact authoring span; expose one problem per invalid candidate through `markdown::schemas`.
- [x] Build each candidate's target schema from the exact non-null scalar fragment or array item fragment, remove `x-darkmatter-suggest`, and validate the candidate with the existing JSON Schema compiler so numeric/string constraints remain single-sourced.
- [x] Add explicit lint reasons for invalid decimal syntax and unsupported lossless numeric representation before target validation, and map validator failures to stable range/integer/length/not-empty/pattern/type reason categories suitable for DMLS messages.
- [x] Exclude `required`, `default`, `generated`, and `example(...)` from the candidate target while retaining applicable `min`, `max`, `integer`, `minLength`, `maxLength`, `not-empty`, and `pattern` behavior.
- [x] Ensure lint walks every property-union atom and every root-union arm, even when a later arm will not supply completion, and produces deterministic declaration-order results.
- [x] Add public API tests proving invalid suggestions are inspectable without turning into `SchemaError`, and regression tests proving arbitrary document values still validate when they satisfy the underlying schema but do not appear in the suggestion list.
- [x] Add conversion snapshots for strings, exact numbers, arrays, constraint-invalid values, syntax-invalid numeric strings, and lossless-boundary fallback strings; assert annotation placement and the absence of `examples`.
- [x] Add compose/validation regression tests proving invalid metadata does not block schema resolution, validator construction, frontmatter validation, composition, or valid sibling completion infrastructure.
- [x] Validation checkpoint: targeted Darkmatter unit and snapshot tests pass, generated schemas validate independently, and lint output contains exact authored spans and stable reasons for every invalid candidate.

Parallelizable in this phase: converter annotation work and lint API design can proceed independently after the AST lands. Constraint-target integration depends on converter output being available.

## Phase 4 - Content-Based Standalone SimplifiedSchema Envelopes

- [x] Add one library-owned content classifier/parser for standalone YAML schema documents that recognizes a pure envelope only when `$schema` is the sole top-level key and recognizes a tagged envelope when `kind: schema` claims the document.
- [x] Parse pure mapping and tagged `types` mapping payloads into the same source-aware SimplifiedSchema representation, and preserve pure sequence payloads as whole-file root unions without exposing a named-import namespace.
- [x] Treat missing/malformed tagged `types`, non-mapping `types`, and unsupported tagged-envelope top-level keys as claimed schema-document errors rather than falling back to ordinary YAML or raw JSON Schema.
- [x] Update `schemas/resolve.rs` so whole-file references to pure and tagged mapping envelopes produce identical complete object schemas, preserve dependency/origin metadata, and resolve nested imports/examples relative to the authoring file.
- [x] Update named-import resolution so `Name@fileref` reads the shared mapping payload from either envelope, rejects pure sequence payloads for named imports, and retains existing eager depth bounds, cycle checks, and `FileReference` path semantics.
- [x] Keep raw JSON Schema resolution distinct: existing YAML/JSON raw schemas continue validating, cannot supply named imports, do not produce suggestion lint data, and do not expose hand-authored `x-darkmatter-suggest` to SimplifiedSchema consumers.
- [x] Expose standalone parse/lint products that retain the authoring document path and candidate source spans, allowing an open schema buffer to be diagnosed directly without transferring ranges to consumers.
- [x] Add resolver tests for both mapping envelopes as whole files and named-import namespaces, pure sequence whole-file use, malformed claimed envelopes, import cycles, relative paths, and raw JSON Schema regressions.
- [x] Validation checkpoint: the same mapping payload yields equivalent resolved SimplifiedSchema/JSON Schema from pure and tagged envelopes; malformed claimed documents fail with schema-document errors; raw JSON Schema behavior remains unchanged.

Parallelizable in this phase: envelope classification tests and resolver integration can proceed alongside DMLS fixture preparation. Named-import work depends on the common envelope parser.

## Phase 5 - DMLS Authoring Documents and Suggestion Diagnostics

- [x] Extend the DMLS document model/router so an open YAML buffer can be classified by the library's content envelopes and analyzed as a standalone SimplifiedSchema authoring document without relying on filename, glob, or consumer discovery.
- [x] Reuse the library's source-aware parse and suggestion-lint products for inline Markdown frontmatter and standalone envelopes; do not reimplement candidate interpretation, decimal handling, or target validation in DMLS.
- [x] Add stable diagnostic catalog entries for source `darkmatter.schema` and code `dm.schema.invalid_suggestion`, mapping every lint problem to a `WARNING` on the exact original argument range with a reason-specific message.
- [x] Publish malformed recognized-envelope diagnostics against the open schema document, including missing/malformed `types` and unsupported tagged-envelope keys, while retaining last-good state only where current DMLS policy permits it.
- [x] Ensure warnings for a referenced standalone schema are published on that authoring document when it is open and are not duplicated on each consuming Markdown document's `$schema` reference.
- [x] Keep the effective schema available after suggestion warnings so key completion, hover, validation, and valid sibling value completion continue operating.
- [x] Verify candidate linting and standalone analysis are passive: no filesystem discovery beyond explicit schema references, composition directive evaluation, shell execution, or network access occurs on open/change/diagnostics.
- [x] Add unit tests for UTF-8/UTF-16 LSP range conversion, YAML escapes, LF/CRLF source maps, and inline versus standalone diagnostic ownership.
- [x] Add DMLS L2 session tests that open/change/close both envelope forms and assert severity, source, code, message category, exact range, warning removal after correction, and no warning on consumers.
- [x] Validation checkpoint: targeted DMLS diagnostic unit and L2 tests pass under both negotiated position encodings, while valid schema completion and validation remain active beside invalid candidates.

Parallelizable in this phase: diagnostic-code/message lowering and standalone document routing can proceed independently after the library lint/envelope APIs stabilize.

## Phase 6 - Suggestion-Aware DMLS Completion

- [x] Add a library-facing query over resolved SimplifiedSchema that returns lint-valid suggestions for a property path, preserving declaration order and retaining scalar/array-item type information without consulting raw generated annotations.
- [x] Implement property-union selection using its sole eligible suggestion-bearing arm and root-union selection using the first declaration-order arm with an eligible suggestion-bearing definition, while continuing to lint all arms.
- [x] Extend DMLS frontmatter completion context detection for scalar property values and eligible properties nested in inline-object schemas.
- [x] Add block-sequence and flow-sequence item contexts so array suggestions insert one candidate element at the cursor instead of replacing or inserting an entire array.
- [x] Prefix-filter against decoded candidate values, preserve declaration order after filtering, and omit every candidate represented by a library lint problem while keeping valid siblings.
- [x] Lower string suggestions to YAML-safe double-quoted insertion text with required escaping and lower numeric suggestions to canonical decimal insertion text; provide precise text edits compatible with existing client capability gates.
- [x] Ensure existing enum, file, and format-hint completion behavior remains unchanged and raw JSON Schema `x-darkmatter-suggest` fields never enter this completion path.
- [x] Add provider unit tests for empty/partial prefixes, escaped strings, negative/canonical numbers, nested objects, block/flow arrays, property unions, root unions, invalid filtering, and raw-schema exclusion.
- [x] Add end-to-end DMLS L2 completion tests for inline schemas and both referenced standalone envelopes, including changes to an open schema document invalidating/recomputing suggestions for consumers through existing dependency edges.
- [x] Validation checkpoint: completion works at all four required position classes, inserts the exact specified YAML text, honors union ordering, excludes invalid candidates, and leaves unrelated completion providers unchanged.

Parallelizable in this phase: YAML insertion escaping and completion-context detection can proceed independently once the library query shape is fixed. L2 sessions should follow provider unit coverage.

## Phase 7 - Public Contract, Catalog, and Documentation Alignment

- [x] Update the public constraint/shape descriptor catalog in `darkmatter/lib/src/markdown/schemas/about.rs` so `suggest(...)` eligibility, cardinality, advisory semantics, and annotation name are generated from the same contract exposed to consumers.
- [x] Update SimplifiedSchema serialization and schema documentation examples to show canonical `suggest(...)` output without rewriting authored values as `enum(...)`, `example(...)`, or JSON Schema `examples`.
- [x] Document `suggest(...)`, number syntax/canonicalization, invalid metadata behavior, standalone pure/tagged envelopes, whole-file and named-import behavior, DMLS warnings/completion, and raw JSON Schema boundaries in `darkmatter/docs/topics/schema-definition.md` and any DMLS user-facing schema documentation.
- [x] Add rustdoc for the new public candidate, lint, envelope, and completion-query types, emphasizing span units, ordering guarantees, side-effect freedom, and error versus lint behavior; remove or correct any nearby comments made stale by the behavior change.
- [x] Update `.claude/skills/darkmatter/SKILL.md` if the implementation adds new public schema/lint entry points or changes the documented standalone-schema architecture.
- [x] Verify no dependency additions are needed; if implementation requires one, use an existing workspace dependency where possible and update `darkmatter/docs/dependencies.md` plus the repository dependency documentation in the same change.
- [x] Add catalog/docs consistency tests or assertions that every advertised constraint parses on its documented eligible types and rejects unsupported types.
- [x] Validation checkpoint: public descriptors, serializer output, rustdoc, topic documentation, and executable grammar/converter tests describe the same behavior and terminology.

Documentation work is parallelizable with late DMLS tests once public API names and insertion semantics are stable.

## Phase 8 - Cross-Platform Regression and Release Validation

- [x] Run the targeted grammar, conversion snapshot, lint, resolver/envelope, validation/composition, DMLS diagnostics, and DMLS completion tests; resolve every failure without weakening an acceptance assertion.
- [x] Run `just test` from the Darkmatter package area and confirm all library, CLI, and DMLS unit tests pass under nextest.
- [x] Run `just test-l2` from the package area and confirm DMLS integration sessions and existing schema/render integration coverage pass without interactive input.
- [x] Run `just lint` from the package area and address all lint findings; run `cargo fmt --check` only as a read-only diagnostic and do not run write-mode formatting.
- [x] Review snapshots and generated JSON Schema fixtures to confirm `x-darkmatter-suggest` ordering/types are stable, invalid fallback metadata is retained, and no unrelated schema output changed.
- [x] Audit tests and implementation for platform-specific path separators, newline assumptions, filesystem ordering, locale-sensitive number handling, and machine-word boundaries; ensure Windows, Linux, and macOS behavior is defined by byte/string operations and `FileReference` semantics.
- [x] Run the no-side-effects DMLS coverage and verify suggestion lint/completion cannot execute shell expressions, follow composition directives, scan unrelated files, or access the network.
- [x] Check every acceptance criterion and Definition of Done item against the Phase 1 matrix, recording the passing test or documentation location and leaving no criterion justified only by manual inspection.
- [x] Validation checkpoint: `just test`, `just test-l2`, and `just lint` all pass; the acceptance matrix is complete; documentation and skill drift are resolved; and the working tree contains only intentional feature changes with no write-mode formatting or git commit performed.
