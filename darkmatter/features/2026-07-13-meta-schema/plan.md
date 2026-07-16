---
agent: claude/
total_phases: 8
created: 2026-07-14
phase: 1
yolo: "true"
packages:
  - darkmatter
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

- [ ] `type-definition` is a canonical keyword (`SimplifiedType::TypeDefinition`)
  that round-trips through parse/serialize, appears in the descriptor catalog
  and `md schema about`, and accepts exactly what the `PropertyDef` parser
  accepts (scalars, mapping objects, non-empty unions) — AC1, AC2.
- [ ] `schema` is a canonical keyword (`SimplifiedType::Schema`) that accepts
  inline shapes, valid local `FileReference` strings, and non-empty root unions
  **without I/O**, and rejects invalid scalars, remote refs, empty unions, and
  invalid arms with the same rules as `$schema` preparation (excluding
  resolution/existence failures) — AC3, AC4.
- [ ] Both compile to carrier domain `["string","object","array"]` plus a
  registered grammar-backed custom keyword; validation-only callers are not
  mutated and compose write-back preserves the authored representation — AC5.
- [ ] `type-definition[]` and `schema[]` parse/serialize/validate through
  ordinary array lowering; flat outer sequences are arrays of independent
  values and union-valued items use a nested sequence — AC6.
- [ ] One public passive parser authority with semantic-only and source-aware
  companions; parity + span-projection tests prove no divergence across
  plain/double/single-quoted YAML, CRLF, UTF-8, nested mappings, and unions —
  AC7.
- [ ] The base schema declares `$schema: schema`; existing valid inline,
  referenced, root-union, and referenced-raw-JSON documents still prepare, and
  DMLS hover shows `Type: schema` instead of `Type: any` — AC8.
- [ ] DMLS gives parser-state completion, precise diagnostics, content-based
  standalone activation with last-good retention, union-aware hover, and is
  side-effect-free — AC9, AC10.
- [ ] Shared `MAX_INLINE_OBJECT_DEPTH` bounds string-form and YAML-native
  nested mappings; over-limit input returns a structured error with no panic or
  stack overflow — AC11.
- [ ] Existing schemas parse/compile/validate/diagnose byte-identically except
  the intentional `$schema` hover/type correction and earlier validation-only
  rejection of malformed `$schema`; imports named `schema`/`type-definition`
  still parse — AC12.
- [ ] `just test` and `just test-l2` pass from the Darkmatter package area;
  implementation stays portable across macOS, Windows, and Linux — AC13.

---

## Phase 1: Baseline, Impact Analysis, and Test Matrix

Goal: establish a green starting line, a documented blast radius, and a failing
test suite that encodes every acceptance criterion before any production edit.

- [ ] Refresh the GitNexus index (`node .gitnexus/run.cjs analyze`) so impact
  analysis reflects current source.
- [ ] Run upstream GitNexus `impact({direction: "upstream"})` on every symbol
  slated for edit (the private property-definition entry point in
  `simplified/grammar.rs`/`simplified/mod.rs`, `parse_yaml_schema`, the
  `resolve.rs` reference classifier, `SimplifiedType`, the `convert.rs`
  lowering, the `format.rs`/`validate.rs` validator registration, and the DMLS
  `DocumentOverlay`). Record blast radius and flag any HIGH/CRITICAL result to
  the user before proceeding.
- [ ] Capture pre-change output for representative existing schema fixtures:
  `md schema about`, `md schema validate` on a schema using `$schema: any`,
  compiled JSON Schema of a representative document, and a DMLS hover on
  `$schema`. Save as `phase1-baseline*.txt` under the feature directory so
  AC12 byte-identity can be proven later.
- [ ] Write `phase1-impact.md` and `phase1-test-matrix.md` mapping each of the
  13 acceptance criteria to concrete test locations (library integration test
  file(s) under `darkmatter/lib/tests/`, proptest generators, and DMLS L2
  session tests under `darkmatter/dmls/tests/`).
- [ ] Add **failing** tests first, covering: the `PropertyDef`-parity matrix
  (scalar/mapping/union accept + invalid scalar/empty union/invalid arm/bad
  constraint reject), `type-definition[]`/`schema[]` array vs union-item
  disambiguation, `schema` reference-syntax accept + remote reject with no I/O,
  span-projection across plain/single/double-quoted + CRLF + UTF-8 + nested +
  union, `MAX_INLINE_OBJECT_DEPTH` over-limit structured error, and the base
  schema `$schema: schema` acceptance-parity + changed-failure-stage split.
- [ ] **Validation checkpoint:** `just test` / `just test-l2` are green on the
  pre-existing suites; the new tests compile and fail for the expected
  (unimplemented) reasons — not for setup errors.

---

## Phase 2: Shared Passive Parser Authority

Goal: one grammar authority with **semantic-only** and **source-aware** entry
points for both a single `PropertyDef` and a whole `$schema` declaration, plus
the shared depth bound. Everything downstream depends on this phase; it is on
the critical path and largely serial.

- [ ] Extract the existing private property-definition entry point into a
  public, passive parser that accepts a YAML value and returns `PropertyDef`
  with structured `SchemaError`. Do **not** create an isomorphic second AST —
  `PropertyDef` remains the semantic product. (`simplified/grammar.rs`,
  `simplified/mod.rs`)
- [ ] Generalize the existing scalar projection seam in `simplified/source.rs`
  into a structural **sidecar source map** keyed by structural schema paths,
  recording authored spans of mapping keys, complete definitions, atoms, type
  keywords, constraints, arguments, import names/references, and union arms.
  Project through plain, single-quoted, and double-quoted YAML scalars via the
  existing `yaml_scalar` seam, without normalizing line endings. No spanned AST.
- [ ] Add the **source-aware companion** parser that returns the same
  `PropertyDef` plus that sidecar map.
- [ ] Add the passive **schema-declaration** parse/classification surface: reuse
  `parse_yaml_schema` for inline mappings and root-union mapping arms; reuse the
  resolver's local-reference classifier + `biscuit_file::FileReference`
  construction for reference syntax. Add outer declaration/file-reference spans
  to the sidecar. (`simplified/mod.rs` new surface)
- [ ] Extract the HTTP(S) / bare-name / path-qualified local-reference
  classification policy out of `resolve.rs` into a shared classifier reused by
  the `schema` surface — **without moving any I/O** into semantic validation.
  Bare schema-root names stay valid declaration syntax; remote forms are
  rejected as syntax. (`resolve.rs`)
- [ ] Apply `MAX_INLINE_OBJECT_DEPTH` as the single shared limit across
  string-form inline objects and YAML-native mapping definitions: root parse
  starts at depth zero, every nested schema object (including mapping arms under
  property unions) increments the same counter, and over-limit returns
  `SchemaError::Grammar` with no panic/overflow.
- [ ] **Validation checkpoint:** parser-parity tests prove the semantic-only and
  source-aware entry points return the same `PropertyDef`; span-projection tests
  pass across all quote/line-ending/nesting variants; depth-limit test returns a
  structured error. `just test` for the affected library suites is green.

---

## Phase 3: Schema Vocabulary, Grammar, and Serialization

Goal: make `type-definition` and `schema` first-class keywords with correct
constraint applicability, canonical serialization (including `[]`), and
descriptor-catalog presence. Depends on Phase 2's public parser for validation
routing.

- [ ] Add `SimplifiedType::TypeDefinition` (keyword `"type-definition"`) and
  `SimplifiedType::Schema` (keyword `"schema"`) with stable canonical keywords.
  (`simplified/types.rs`)
- [ ] Accept both keywords through the ordinary primitive-keyword path, ensuring
  terminal `Name@file` import syntax still wins over primitive lookup so
  `schema@file` / `type-definition@file` remain imports (AC12 migration
  compatibility). (`simplified/grammar.rs`)
- [ ] Enforce constraint applicability: permit only `required`, `default(...)`
  (scalar type-expression default only, per the shared `default(...)` grammar),
  and `generated`. Reject all value-domain constraints (`min`, `max`, `pattern`,
  `suggest`, `eager`, …) because they constrain denoted values, not the
  definition artifact.
- [ ] Extend canonical serialization (and proptest generators/shrinkers) so both
  keywords and their `[]` postfix forms round-trip. (`simplified/serialize.rs`,
  `schemas_grammar_proptest.rs`)
- [ ] Add authoritative descriptors (accepted carriers, permitted constraints,
  parse-only behavior, DMLS meaning) so both types appear in
  `schema_type_descriptors()` and `md schema about`. Add parity coverage.
  (`about.rs`)
- [ ] Confirm `md schema detect` **never** infers either type (carrier-only
  inference is preserved).
- [ ] **Validation checkpoint:** focused grammar, serialization, proptest, and
  `md schema about` snapshot tests pass; `type-definition`/`schema` round-trip
  with and without `[]`.

---

## Phase 4: JSON Schema Lowering and Custom-Keyword Validators

Goal: compile both types to the portable carrier domain plus a registered
grammar-backed custom keyword, wired to the Phase 2 shared parsers, preserving
diagnostic-specialization schema paths and ordinary array lowering. Depends on
Phases 2–3.

- [ ] Emit the carrier + keyword fragments in `convert.rs`:
  `{"type": ["string","object","array"], "x-darkmatter-type-definition": true}`
  and the `x-darkmatter-schema` counterpart. Keep the optional-nullable wrapper
  outside the fragment and preserve existing `required` parent-object behavior.
- [ ] Register both custom keyword validators in `format.rs`/`validate.rs`,
  backed by the Phase 2 passive parsers. The `type-definition` keyword validates
  the full `PropertyDef` grammar; the `schema` keyword validates the full
  declaration (inline shape / local reference syntax / root union) with **no
  I/O**. Failures return ordinary structured `ValidationProblem`s carrying
  instance path, source position, and `ConstraintViolation` classification.
- [ ] Preserve distinguishing keyword schema paths (`x-darkmatter-type-definition`
  / `x-darkmatter-schema`) so DMLS can replace the generic problem with a
  precise diagnostic — **without** adding a new public `ValidationProblemCode`
  variant.
- [ ] Verify ordinary generic array lowering wraps each fragment under `items`
  for `type-definition[]` / `schema[]`, and the keyword always validates exactly
  the item value it appears on.
- [ ] Keep coercion/normalization explicit no-ops for these carriers in
  `coerce.rs`: a mapping stays a mapping, a sequence stays a sequence, no
  native-to-string coercion; YAML boolean/number/null scalars are invalid.
- [ ] Add pure parse-based trigger matching for both variants in
  `triggers/matcher.rs` (both types are pure; the reserved `$schema` control key
  stays absent from the trigger-matching instance).
- [ ] **Validation checkpoint (with escape-hatch check):** conversion snapshots,
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

- [ ] Retype `$schema` from `any` to `schema` in
  `darkmatter/docs/schemas/darkmatter.yaml`, and correct the description so it
  no longer implies raw JSON Schema can be authored as an inline mapping (raw
  JSON Schema is a referenced-file concern only).
- [ ] Prove **acceptance parity**: existing valid inline, referenced,
  root-union, and referenced-raw-JSON documents still prepare through
  `resolve_schema_with_roots`.
- [ ] Prove the **changed failure stage**: malformed `$schema` declarations now
  fail at the validation stage (grammar-specific message) rather than only at a
  later resolver call. Tests must distinguish acceptance parity from the changed
  failure stage.
- [ ] **Validation checkpoint:** base-schema compile + representative document
  preparation tests pass; the `phase1` baseline captures are re-run and diffs
  are limited to the intentional `$schema` metadata correction (AC8, AC12).

---

## Phase 6: DMLS Overlay — Standalone Schema Model and Activation

Goal: give `DocumentOverlay` a real parsed standalone schema + source map with
last-good retention and content-based activation. Depends on Phase 2's
source-aware parsers; unblocks Phase 7's providers.

- [ ] Carry the parsed standalone schema and its sidecar source map on
  `DocumentOverlay` (the existing `SuggestionState::Standalone` marker is not a
  sufficient semantic model). (`dmls/src/overlay/mod.rs`, `overlay/schema.rs`)
- [ ] Implement the two explicit activation paths: (1) frontmatter values whose
  effective `PropertyDef` contains a `type-definition`/`schema` atom, plus the
  reserved `$schema` control value from the base language contract; (2)
  standalone content-based activation through
  `parse_standalone_schema_document`, with a lexical envelope claim (sole
  top-level `$schema`, or top-level `kind: schema`) retaining the last-good
  parsed schema/source map while exposing current parser errors. File extension
  / directory location alone must never activate.
- [ ] Mirror frontmatter's last-good AST contract: completion and hover survive
  a mid-keystroke YAML error, but the current buffer always owns diagnostics and
  stale semantic data must never claim malformed current text is valid.
- [ ] **Validation checkpoint:** DMLS L2 tests prove content-based activation
  (never filename-based), last-good retention during a malformed edit, and that
  raw JSON Schema / ordinary YAML stay outside the provider.

---

## Phase 7: DMLS Hover, Completion, and Precise Diagnostics

Goal: the schema-driven editor UX. Depends on Phase 6's overlay model and the
Phase 2–4 library APIs.

- [ ] **Hover** (`providers/frontmatter.rs`): `$schema` control key reports
  `Type: schema` with the base description. Within a schema shape, hover shows
  the artifact role plus the denoted type — `Type: type-definition` /
  `Declares: string | object` — and the existing schema-hover renderer must
  render **all** union arms (first-arm-as-representative is insufficient). For
  `foo: string(required)`, `Declares: string` plus a `Required` constraint
  summary.
- [ ] **Completion**: inside a `type-definition` value, drive from
  `schema_type_descriptors()` + parser state (type keywords, valid constraints,
  `[]`/inline-object/union/`Name@file` scaffolds, referenced named types when a
  passive namespace exists). Inside a `schema` value, offer outer scaffolds then
  delegate each inline property value to `type-definition` completion;
  file-reference completion reuses the existing passive path-completion. For
  `type-definition[]`/`schema[]`, identify the outer array item at the cursor,
  then apply scalar completion; support inline `$schema` blocks and standalone
  documents (incl. tagged `kind: schema` → `types` mapping).
- [ ] **Diagnostics** (`dmls/src/diagnostics/frontmatter.rs`): emit
  `dm.schema.invalid_type_definition` at the smallest reliable authored range
  and retain `dm.schema.invalid_schema_shape` for outer declarations. Keep
  file-reference syntax errors distinguishable from resolution failures.
  Specialized diagnostics **replace** (not duplicate) the generic custom-keyword
  failure for the same span. Use projected token spans for scalar errors and the
  smallest key/value/arm sidecar span for shape errors, falling back to the
  parent mapping span only for a missing structural element.
- [ ] Record the typed activation signal for the deferred semantic-token family
  (a complete definition may be classified as a semantic type) — no fine-grained
  meta-schema tokens required this feature.
- [ ] **Side-effect audit:** extend the DMLS `no_side_effects` test so
  `type-definition`/`schema` analysis loads no references, expands no
  imports/examples, composes nothing, executes no expression/shell, and touches
  no network (AC10).
- [ ] **Validation checkpoint:** focused DMLS provider + L2 session tests prove
  gated completion, union-aware hover, precise diagnostics, and side-effect
  freedom.

**Parallelizable after Phase 6:** hover, completion, and diagnostics wiring can
proceed in parallel once the overlay model and shared schema accessors are
fixed, joining at the checkpoint.

---

## Phase 8: Documentation, Hashes, and Release Gate

Goal: public docs, drift maintenance, and full-suite verification tracing every
acceptance criterion.

- [ ] Update `darkmatter/docs/topics/schema-definition.md` with the two semantic
  meta-types, the carrier vs denoted-type distinction, the meta-schema status,
  and the parse-only boundary.
- [ ] Update the `darkmatter` skill (`.claude/skills/darkmatter/SKILL.md`) to
  record the two types and the shared passive parser authority.
- [ ] Review every changed `///`, `//!`, and inline comment for behavioral
  drift; fix or delete drifted comments in the same change (comment-quality +
  scope-discipline rules).
- [ ] Refresh every changed Markdown state hash with Darkmatter's Markdown-aware
  hasher (`md hash <file> --save`), including any skill/doc `hash:` frontmatter.
- [ ] Run `just build`, `just test`, `just test-l2`, and `just lint` from the
  Darkmatter package area, then run build/test/lint in downstream package
  areas selected by impact analysis (including Claudine when its exhaustive
  matches are affected). Do not use a workspace-wide Cargo check as a proxy.
- [ ] Exercise `md schema about` and representative `md schema validate`
  invocations to confirm both types appear and behave as documented.
- [ ] Run `cargo fmt --check` **read-only** as a diagnostic only (never write).
- [ ] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})` to
  confirm only expected symbols/flows changed.
- [ ] **Final validation checkpoint:** walk all 13 acceptance criteria against
  the implemented behavior and captured baselines; confirm AC12 byte-identity
  (except the intentional `$schema` metadata correction and earlier
  validation-only rejection) and AC13 cross-OS portability of the design.
