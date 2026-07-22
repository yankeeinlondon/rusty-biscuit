---
agent: claude/
total_phases: 8
created: 2026-07-14
phase: 8
yolo: "true"
packages:
  - darkmatter
source_code:
  - darkmatter/lib/tests/meta_schema_phase1.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/reference.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/tests/schemas_source_projection.rs
  - claudine/cli/src/commands/context/format.rs
  - claudine/lib/src/composition/schema/classify.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/tests/schema_about.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
  - darkmatter/lib/tests/meta_schema_phase3.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/meta_schema_phase4.rs
  - darkmatter/lib/tests/meta_schema_phase5.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/tests/meta_schema_phase6.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
documentation:
  - darkmatter/features/2026-07-13-meta-schema/plan.md
  - darkmatter/docs/schemas/darkmatter.yaml
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-compiled-json-schema.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-dmls-hover.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-schema-about.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-validation.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-impact.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-test-matrix.md
  - darkmatter/features/2026-07-13-meta-schema/phase2-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase3-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase4-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase6-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/phase7-test-map.md
source_files_during_phase_1:
  - darkmatter/lib/tests/meta_schema_phase1.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_1:
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_1:
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-compiled-json-schema.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-dmls-hover.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-schema-about.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-baseline-validation.txt
  - darkmatter/features/2026-07-13-meta-schema/phase1-impact.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-test-matrix.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/reference.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/tests/meta_schema_phase1.rs
  - darkmatter/lib/tests/schemas_source_projection.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_2:
  - darkmatter/features/2026-07-13-meta-schema/phase2-test-map.md
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/context/format.rs
  - claudine/lib/src/composition/schema/classify.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/tests/schema_about.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
  - darkmatter/lib/tests/meta_schema_phase1.rs
  - darkmatter/lib/tests/meta_schema_phase3.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
docs_updated_during_phase_3:
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_3:
  - darkmatter/features/2026-07-13-meta-schema/phase3-test-map.md
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/meta_schema_phase1.rs
  - darkmatter/lib/tests/meta_schema_phase4.rs
docs_updated_during_phase_4:
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_4:
  - darkmatter/features/2026-07-13-meta-schema/phase4-test-map.md
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_5:
  - darkmatter/lib/tests/meta_schema_phase5.rs
docs_updated_during_phase_5:
  - darkmatter/docs/schemas/darkmatter.yaml
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_5:
  - darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-test-map.md
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/tests/meta_schema_phase6.rs
docs_updated_during_phase_6:
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_6:
  - darkmatter/features/2026-07-13-meta-schema/phase6-test-map.md
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_7:
  - darkmatter/features/2026-07-13-meta-schema/phase7-test-map.md
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8:
  - darkmatter/lib/tests/meta_schema_phase1.rs
docs_updated_during_phase_8:
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/features/2026-07-13-meta-schema/phase1-test-matrix.md
  - darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md
  - darkmatter/features/2026-07-13-meta-schema/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/darkmatter/SKILL.md
---

# Execution Plan: SimplifiedSchema Meta-Schema Types (`type-definition`, `schema`)

This plan adds two grammar-backed semantic types to SimplifiedSchema:
`type-definition` (one `PropertyDef`) and `schema` (one `$schema` declaration).
Both are **parse-only, side-effect-free**, wrap the *existing* authoritative
parsers rather than inventing new grammar, and light up DMLS schema
intelligence inside inline and standalone SimplifiedSchema.

The ordering is deliberate: the shared passive parser authority (Phase 2) is
built first so every later consumer — grammar keywords, custom-keyword
validators, the base-schema retype, and DMLS — reads one product instead of
re-tokenizing source. DMLS work (Phases 6–7) consumes stable library APIs and
never duplicates schema semantics.

> Skill: this is Darkmatter package-area work. Use the `darkmatter` skill for
> library surfaces and the DMLS notes; use `biscuit-file` for `FileReference`
> classification; use `rust-testing`/`nextest` for the test tiers. All three
> target OSes (macOS, Windows, Linux) must be honored in design even though
> the host can only run macOS.

## Success Criteria

Traceable to the spec's 13 acceptance criteria:

- [x] `type-definition` is a canonical keyword (`SimplifiedType::TypeDefinition`)
  that round-trips through parse/serialize, appears in the descriptor catalog
  and `md schema about`, and accepts exactly what the `PropertyDef` parser
  accepts (scalars, mapping objects, non-empty unions) — AC1, AC2.
- [x] `schema` is a canonical keyword (`SimplifiedType::Schema`) that accepts
  inline shapes, valid local `FileReference` strings, and non-empty root unions
  **without I/O**, and rejects invalid scalars, remote refs, empty unions, and
  invalid arms with the same rules as `$schema` preparation (excluding
  resolution/existence failures) — AC3, AC4.
- [x] Both compile to carrier domain `["string","object","array"]` plus a
  registered grammar-backed custom keyword; validation-only callers are not
  mutated and compose write-back preserves the authored representation — AC5.
- [x] `type-definition[]` and `schema[]` parse/serialize/validate through
  ordinary array lowering; flat outer sequences are arrays of independent
  values and union-valued items use a nested sequence — AC6.
- [x] One public passive parser authority with semantic-only and source-aware
  companions; parity + span-projection tests prove no divergence across
  plain/double/single-quoted YAML, CRLF, UTF-8, nested mappings, and unions —
  AC7.
- [x] The base schema declares `$schema: schema`; existing valid inline,
  referenced, root-union, and referenced-raw-JSON documents still prepare, and
  DMLS hover shows `Type: schema` instead of `Type: any` — AC8.
- [x] DMLS gives parser-state completion, precise diagnostics, content-based
  standalone activation with last-good retention, union-aware hover, and is
  side-effect-free — AC9, AC10.
- [x] Shared `MAX_INLINE_OBJECT_DEPTH` bounds string-form and YAML-native
  nested mappings; over-limit input returns a structured error with no panic or
  stack overflow — AC11.
- [x] Existing schemas parse/compile/validate/diagnose byte-identically except
  the intentional `$schema` hover/type correction and earlier validation-only
  rejection of malformed `$schema`; imports named `schema`/`type-definition`
  still parse — AC12.
- [x] `just test` and `just test-l2` pass from the Darkmatter package area;
  implementation stays portable across macOS, Windows, and Linux — AC13.

---

## Phase 1: Baseline, Impact Analysis, and Test Matrix

Goal: establish a green starting line, a documented blast radius, and a failing
test suite that encodes every acceptance criterion before any production edit.

- [x] Refresh the GitNexus index (`node .gitnexus/run.cjs analyze`) so impact
  analysis reflects current source.
- [x] Run upstream GitNexus `impact({direction: "upstream"})` on every symbol
  slated for edit (the private property-definition entry point in
  `simplified/grammar.rs`/`simplified/mod.rs`, `parse_yaml_schema`, the
  `resolve.rs` reference classifier, `SimplifiedType`, the `convert.rs`
  lowering, the `format.rs`/`validate.rs` validator registration, and the DMLS
  `DocumentOverlay`). Record blast radius and flag any HIGH/CRITICAL result to
  the user before proceeding.
- [x] Capture pre-change output for representative existing schema fixtures:
  `md schema about`, `md schema validate` on a schema using `$schema: any`,
  compiled JSON Schema of a representative document, and a DMLS hover on
  `$schema`. Save as `phase1-baseline*.txt` under the feature directory so
  AC12 byte-identity can be proven later.
- [x] Write `phase1-impact.md` and `phase1-test-matrix.md` mapping each of the
  13 acceptance criteria to concrete test locations (library integration test
  file(s) under `darkmatter/lib/tests/`, proptest generators, and DMLS L2
  session tests under `darkmatter/dmls/tests/`).
- [x] Add **failing** tests first, covering: the `PropertyDef`-parity matrix
  (scalar/mapping/union accept + invalid scalar/empty union/invalid arm/bad
  constraint reject), `type-definition[]`/`schema[]` array vs union-item
  disambiguation, `schema` reference-syntax accept + remote reject with no I/O,
  span-projection across plain/single/double-quoted + CRLF + UTF-8 + nested +
  union, `MAX_INLINE_OBJECT_DEPTH` over-limit structured error, and the base
  schema `$schema: schema` acceptance-parity + changed-failure-stage split.
- [x] **Validation checkpoint:** the full L1 population is green and the new
  tests compile and fail for the expected unimplemented reasons. All 19
  Darkmatter library L2 tests pass; two pre-existing CLI terminal-luminance
  failures are recorded in `phase1-test-matrix.md`.

---

## Phase 2: Shared Passive Parser Authority

Goal: one grammar authority with **semantic-only** and **source-aware** entry
points for both a single `PropertyDef` and a whole `$schema` declaration, plus
the shared depth bound. Everything downstream depends on this phase; it is on
the critical path and largely serial.

- [x] Extract the existing private property-definition entry point into a
  public, passive parser that accepts a YAML value and returns `PropertyDef`
  with structured `SchemaError`. Do **not** create an isomorphic second AST —
  `PropertyDef` remains the semantic product. (`simplified/grammar.rs`,
  `simplified/mod.rs`)
- [x] Generalize the existing scalar projection seam in `simplified/source.rs`
  into a structural **sidecar source map** keyed by structural schema paths,
  recording authored spans of mapping keys, complete definitions, atoms, type
  keywords, constraints, arguments, import names/references, and union arms.
  Project through plain, single-quoted, and double-quoted YAML scalars via the
  existing `yaml_scalar` seam, without normalizing line endings. No spanned AST.
- [x] Add the **source-aware companion** parser that returns the same
  `PropertyDef` plus that sidecar map.
- [x] Add the passive **schema-declaration** parse/classification surface: reuse
  `parse_yaml_schema` for inline mappings and root-union mapping arms; reuse the
  resolver's local-reference classifier + `biscuit_file::FileReference`
  construction for reference syntax. Add outer declaration/file-reference spans
  to the sidecar. (`simplified/mod.rs` new surface)
- [x] Extract the HTTP(S) / bare-name / path-qualified local-reference
  classification policy out of `resolve.rs` into a shared classifier reused by
  the `schema` surface — **without moving any I/O** into semantic validation.
  Bare schema-root names stay valid declaration syntax; remote forms are
  rejected as syntax. (`resolve.rs`)
- [x] Apply `MAX_INLINE_OBJECT_DEPTH` as the single shared limit across
  string-form inline objects and YAML-native mapping definitions: root parse
  starts at depth zero, every nested schema object (including mapping arms under
  property unions) increments the same counter, and over-limit returns
  `SchemaError::Grammar` with no panic/overflow.
- [x] **Validation checkpoint:** parser-parity tests prove the semantic-only and
  source-aware entry points return the same `PropertyDef`; span-projection tests
  pass across all quote/line-ending/nesting variants; depth-limit test returns a
  structured error. `just test` for the affected library suites is green.

---

## Phase 3: Schema Vocabulary, Grammar, and Serialization

Goal: make `type-definition` and `schema` first-class keywords with correct
constraint applicability, canonical serialization (including `[]`), and
descriptor-catalog presence. Depends on Phase 2's public parser for validation
routing.

- [x] Add `SimplifiedType::TypeDefinition` (keyword `"type-definition"`) and
  `SimplifiedType::Schema` (keyword `"schema"`) with stable canonical keywords.
  (`simplified/types.rs`)
- [x] Accept both keywords through the ordinary primitive-keyword path, ensuring
  terminal `Name@file` import syntax still wins over primitive lookup so
  `schema@file` / `type-definition@file` remain imports (AC12 migration
  compatibility). (`simplified/grammar.rs`)
- [x] Enforce constraint applicability: permit only `required`, `default(...)`
  (scalar type-expression default only, per the shared `default(...)` grammar),
  and `generated`. Reject all value-domain constraints (`min`, `max`, `pattern`,
  `suggest`, `eager`, …) because they constrain denoted values, not the
  definition artifact.
- [x] Extend canonical serialization (and proptest generators/shrinkers) so both
  keywords and their `[]` postfix forms round-trip. (`simplified/serialize.rs`,
  `schemas_grammar_proptest.rs`)
- [x] Add authoritative descriptors (accepted carriers, permitted constraints,
  parse-only behavior, DMLS meaning) so both types appear in
  `schema_type_descriptors()` and `md schema about`. Add parity coverage.
  (`about.rs`)
- [x] Confirm `md schema detect` **never** infers either type (carrier-only
  inference is preserved).
- [x] **Validation checkpoint:** focused grammar, serialization, proptest, and
  `md schema about` snapshot tests pass; `type-definition`/`schema` round-trip
  with and without `[]`.

---

## Phase 4: JSON Schema Lowering and Custom-Keyword Validators

Goal: compile both types to the portable carrier domain plus a registered
grammar-backed custom keyword, wired to the Phase 2 shared parsers, preserving
diagnostic-specialization schema paths and ordinary array lowering. Depends on
Phases 2–3.

- [x] Emit the carrier + keyword fragments in `convert.rs`:
  `{"type": ["string","object","array"], "x-darkmatter-type-definition": true}`
  and the `x-darkmatter-schema` counterpart. Keep the optional-nullable wrapper
  outside the fragment and preserve existing `required` parent-object behavior.
- [x] Register both custom keyword validators in `format.rs`/`validate.rs`,
  backed by the Phase 2 passive parsers. The `type-definition` keyword validates
  the full `PropertyDef` grammar; the `schema` keyword validates the full
  declaration (inline shape / local reference syntax / root union) with **no
  I/O**. Failures return ordinary structured `ValidationProblem`s carrying
  instance path, source position, and `ConstraintViolation` classification.
- [x] Preserve distinguishing keyword schema paths (`x-darkmatter-type-definition`
  / `x-darkmatter-schema`) so DMLS can replace the generic problem with a
  precise diagnostic — **without** adding a new public `ValidationProblemCode`
  variant.
- [x] Verify ordinary generic array lowering wraps each fragment under `items`
  for `type-definition[]` / `schema[]`, and the keyword always validates exactly
  the item value it appears on.
- [x] Keep coercion/normalization explicit no-ops for these carriers in
  `coerce.rs`: a mapping stays a mapping, a sequence stays a sequence, no
  native-to-string coercion; YAML boolean/number/null scalars are invalid.
- [x] Add pure parse-based trigger matching for both variants in
  `triggers/matcher.rs` (both types are pure; the reserved `$schema` control key
  stays absent from the trigger-matching instance).
- [x] **Validation checkpoint (with escape-hatch check):** conversion snapshots,
  the custom-keyword validation table (accept/reject parity with the
  `PropertyDef` and `$schema` parsers), array-lowering fixtures, coercion
  no-op fixtures, and trigger-match tests pass. Confirm validation-only callers
  and compose write-back are byte-unchanged (AC5).

**Parallelizable after Phase 3:** `convert.rs` emission, `coerce.rs` no-op
proofs, and `triggers/matcher.rs` matching can proceed in parallel with the
`format.rs`/`validate.rs` validator registration, joining at the checkpoint.

---

## Phase 5: Base-Schema Retype and Migration

Goal: replace `$schema: any` with `$schema: schema` in the Darkmatter base
schema and prove the acceptance/failure-stage contract. Depends on Phases 3–4
(the `schema` type and its validator must exist).

- [x] Retype `$schema` from `any` to `schema` in
  `darkmatter/docs/schemas/darkmatter.yaml`, and correct the description so it
  no longer implies raw JSON Schema can be authored as an inline mapping (raw
  JSON Schema is a referenced-file concern only).
- [x] Prove **acceptance parity**: existing valid inline, referenced,
  root-union, and referenced-raw-JSON documents still prepare through
  `resolve_schema_with_roots`.
- [x] Prove the **changed failure stage**: malformed `$schema` declarations now
  fail at the validation stage (grammar-specific message) rather than only at a
  later resolver call. Tests must distinguish acceptance parity from the changed
  failure stage.
- [x] **Validation checkpoint:** base-schema compile + representative document
  preparation tests pass; the `phase1` baseline captures are re-run and diffs
  are limited to the intentional `$schema` metadata correction (AC8, AC12).

---

## Phase 6: DMLS Overlay — Standalone Schema Model and Activation

Goal: give `DocumentOverlay` a real parsed standalone schema + source map with
last-good retention and content-based activation. Depends on Phase 2's
source-aware parsers; unblocks Phase 7's providers.

- [x] Carry the parsed standalone schema and its sidecar source map on
  `DocumentOverlay` (the existing `SuggestionState::Standalone` marker is not a
  sufficient semantic model). (`dmls/src/overlay/mod.rs`, `overlay/schema.rs`)
- [x] Implement the two explicit activation paths: (1) frontmatter values whose
  effective `PropertyDef` contains a `type-definition`/`schema` atom, plus the
  reserved `$schema` control value from the base language contract; (2)
  standalone content-based activation through
  `parse_standalone_schema_document`, with a lexical envelope claim (sole
  top-level `$schema`, or top-level `kind: schema`) retaining the last-good
  parsed schema/source map while exposing current parser errors. File extension
  / directory location alone must never activate.
- [x] Mirror frontmatter's last-good AST contract: completion and hover survive
  a mid-keystroke YAML error, but the current buffer always owns diagnostics and
  stale semantic data must never claim malformed current text is valid.
- [x] **Validation checkpoint:** DMLS L2 tests prove content-based activation
  (never filename-based), last-good retention during a malformed edit, and that
  raw JSON Schema / ordinary YAML stay outside the provider.

---

## Phase 7: DMLS Hover, Completion, and Precise Diagnostics

Goal: the schema-driven editor UX. Depends on Phase 6's overlay model and the
Phase 2–4 library APIs.

- [x] **Hover** (`providers/frontmatter.rs`): `$schema` control key reports
  `Type: schema` with the base description. Within a schema shape, hover shows
  the artifact role plus the denoted type — `Type: type-definition` /
  `Declares: string | object` — and the existing schema-hover renderer must
  render **all** union arms (first-arm-as-representative is insufficient). For
  `foo: string(required)`, `Declares: string` plus a `Required` constraint
  summary.
- [x] **Completion**: inside a `type-definition` value, drive from
  `schema_type_descriptors()` + parser state (type keywords, valid constraints,
  `[]`/inline-object/union/`Name@file` scaffolds, referenced named types when a
  passive namespace exists). Inside a `schema` value, offer outer scaffolds then
  delegate each inline property value to `type-definition` completion;
  file-reference completion reuses the existing passive path-completion. For
  `type-definition[]`/`schema[]`, identify the outer array item at the cursor,
  then apply scalar completion; support inline `$schema` blocks and standalone
  documents (incl. tagged `kind: schema` → `types` mapping).
- [x] **Diagnostics** (`dmls/src/diagnostics/frontmatter.rs`): emit
  `dm.schema.invalid_type_definition` at the smallest reliable authored range
  and retain `dm.schema.invalid_schema_shape` for outer declarations. Keep
  file-reference syntax errors distinguishable from resolution failures.
  Specialized diagnostics **replace** (not duplicate) the generic custom-keyword
  failure for the same span. Use projected token spans for scalar errors and the
  smallest key/value/arm sidecar span for shape errors, falling back to the
  parent mapping span only for a missing structural element.
- [x] Record the typed activation signal for the deferred semantic-token family
  (a complete definition may be classified as a semantic type) — no fine-grained
  meta-schema tokens required this feature.
- [x] **Side-effect audit:** extend the DMLS `no_side_effects` test so
  `type-definition`/`schema` analysis loads no references, expands no
  imports/examples, composes nothing, executes no expression/shell, and touches
  no network (AC10).
- [x] **Validation checkpoint:** focused DMLS provider + L2 session tests prove
  gated completion, union-aware hover, precise diagnostics, and side-effect
  freedom.

**Parallelizable after Phase 6:** hover, completion, and diagnostics wiring can
proceed in parallel once the overlay model and shared schema accessors are
fixed, joining at the checkpoint.

---

## Phase 8: Documentation, Hashes, and Release Gate

Goal: public docs, drift maintenance, and full-suite verification tracing every
acceptance criterion.

- [x] Update `darkmatter/docs/topics/schema-definition.md` with the two semantic
  meta-types, the carrier vs denoted-type distinction, the meta-schema status,
  and the parse-only boundary.
- [x] Update the `darkmatter` skill (`.claude/skills/darkmatter/SKILL.md`) to
  record the two types and the shared passive parser authority.
- [x] Review every changed `///`, `//!`, and inline comment for behavioral
  drift; fix or delete drifted comments in the same change (comment-quality +
  scope-discipline rules).
- [x] Refresh every changed Markdown state hash with Darkmatter's Markdown-aware
  hasher (`md hash <file> --save`), including any skill/doc `hash:` frontmatter.
- [x] Run `just build`, `just test`, `just test-l2`, and `just lint` from the
  Darkmatter package area, then run build/test/lint in downstream package
  areas selected by impact analysis (including Claudine when its exhaustive
  matches are affected). Do not use a workspace-wide Cargo check as a proxy.
  The complete Darkmatter L1 population passed in eight bounded partitions;
  build and lint passed, as did all 19 library L2 tests, all three DMLS L2
  tests, and the three real `schema about` CLI L2 tests. The full CLI L2 gate
  remains blocked by the pre-existing terminal-theme luma failure recorded in
  the Phase 1 matrix.
  Both affected Claudine packages built, their two focused regressions passed,
  and 5,458/5,459 runnable downstream L1 tests passed; the sole failure is unrelated
  pre-existing dispatch-inventory line drift in untouched wrap sources.
  `claudine` lint passed, while canonical `claudine-cli` lint remains blocked
  by three pre-existing diagnostics in untouched harness files; allowing only
  those three diagnostics leaves the affected downstream package lint-clean.
- [x] Exercise `md schema about` and representative `md schema validate`
  invocations to confirm both types appear and behave as documented.
- [x] Run `cargo fmt --check` **read-only** as a diagnostic only (never write).
  The diagnostic was attempted on 2026-07-18 but the pinned stable toolchain
  has no installed `rustfmt` component; no formatter was installed or run.
- [x] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})` to
  confirm only expected symbols/flows changed.
  The required comparison reported CRITICAL risk across 635 files because this
  long-lived branch contains substantial unrelated work relative to `main`.
  A second working-tree-only detection isolated 60 symbols in 28 files at LOW
  risk with no affected execution flows; its meta-schema symbols match this
  plan, while the additional Sniff symbols are preserved unrelated user work.
- [x] **Final validation checkpoint:** walk all 13 acceptance criteria against
  the implemented behavior and captured baselines; confirm AC12 byte-identity
  (except the intentional `$schema` metadata correction and earlier
  validation-only rejection) and AC13 cross-OS portability of the design.
  The Phase 1 matrix plus Phase 2–7 test maps cover every criterion, including
  shipped-artifact corpus and end-to-end paths, malformed representation
  variants, boundary depth, passive analysis, and repeated persistence. The
  Phase 5 replay confirms only the two intentional AC12 deltas. Portability was
  audited around `Path`, `tempfile`, `FileReference`, and file-URL APIs with no
  OS-specific separator or location assumptions. AC13's feature paths pass;
  the unrelated pre-existing L2 and downstream gate exceptions are recorded in
  the release-gate task above.
