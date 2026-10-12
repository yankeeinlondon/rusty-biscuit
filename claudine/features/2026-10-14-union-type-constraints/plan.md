---
total_phases: 5
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
    - darkmatter/lib/tests/union_fixtures/mod.rs
    - darkmatter/lib/tests/l1/schemas_union_fixture_controls.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
---

# Union-Typed Named-Type Constraints Implementation Plan

## Phase 1: Establish Scope and Contracts

### Necessary Rules

- **Prerequisite gate:** implement against the completed `2026-10-13-named-type-presence-and-local-names` tree. The inspected checkout still contains `strip_top_level_presence` and the blanket union-postfix rejection in `resolve.rs`; it is not the compatibility baseline. Do not implement the prerequisite opportunistically in this change. If it is unavailable, complete the planning and spike analysis, then report implementation blocked on that dependency.
- **Ownership:** the feature lives in `darkmatter`, `darkmatter-cli`, and `dmls`, despite its specification and plan living under claudine. Claudine is a downstream consumer to inspect for public-type impact. Do not change Content Policy's shipped schema or baseline wiring; use a modified fixture copy for the required-policy examples.
- **Representation:** select one canonical array-of-union representation in Wave 2. Preserve authored references, defining namespaces, arm order, and provenance through passive parsing and resolution. An expanded anonymous union may have a display description, but must not masquerade as supported parseable `(A | B)[]` syntax.
- **Scope of constraints:** property presence, generation, and timing hoist from union arms; value constraints remain on their item arms; post-array constraints belong to the container. Universal `eager` before or after `[]` never changes a lazy file arm into an eager file arm. Nested fields keep their own metadata. `generated` suppresses static requiredness as today and inserts no value.
- **Defaults:** lift only an array-compatible inherited default, using the prerequisite's equal-value merge rule. Reject scalar defaults and conflicting defaults with structured provenance. Do not silently discard, wrap, or insert defaults. `U(default(x))` without `[]` remains unsupported.
- **Consumer safety:** validation accepts any matching arm; closest-arm scoring is explanatory. Coercion requires exactly one independently validating candidate, rewriting requires an unambiguous eligible eager-file arm, and cleanup never repairs inside a genuine union. Literal discriminants narrow reporting and editor choices only under the existing unambiguous-selection contract.
- **Parser scope:** the string grammar is excluded from the Input Robustness Matrix, but the YAML schema envelopes, named-type definitions, and document inputs exercised by the changed resolution path are included below. Do not add JSON/TOML named-definition loaders or refactor unrelated raw JSON Schema readers. Existing raw JSON Schema behavior remains in the compatibility corpus.
- **Spike scope:** one bounded representation/serialization spike is justified because the specification deliberately leaves the Rust spelling open. No performance spike is scheduled: the specification identifies no new performance target or OS-specific cost. Any proposed wider measurement needs an author ruling before being added.
- **Unresolved decisions:** the functional spec has no open product questions. The Rust representation and exact structured-error spelling are implementation decisions constrained by the spec. Record their resolution before dependent waves; do not invent new syntax, recursive references, item source-position support, or CI cells.

### Work and Successful Completion

Extend named-union reference resolution to accept property markers, descriptions,
and array postfixes; lower arrays to a separate container with `items.anyOf`;
then teach the library walkers, typed-tree consumers, CLI, and DMLS to traverse
those item alternatives under their existing safety rules. Update current docs
and the Darkmatter skill alongside the implementation.

Completion means all nine specification acceptance criteria are demonstrated:
the modified Content Policy fixture validates correctly, invalid operations
have structured errors, every consumer handles union items, existing schemas
lower to byte-identical output relative to the post-prerequisite tree, and
hermetic CLI/DMLS tests plus Darkmatter's L1 and lint recipes pass. The terminal
state is **implementation complete, ready for review**; leave the feature in
its active directory and do not run `just complete` or commit.

### Wave Scheduling

Wave numbers are unique across this plan. Each wave is a dependency step: finish
its checkpoint before entering the next wave. Tasks within waves explicitly
marked parallel may be assigned to subagents with disjoint file ownership.
All implementation agents must be told the entire session tree is non-interactive.
No task depends on another task in the same parallel wave. Shared AST, resolver,
lowering, and fixture changes belong to their designated owner; coordinate
consolidated test-module registration rather than editing it concurrently.

### Wave 1 — Baseline and Inventory (Sequential)

- [x] **Verify Prerequisite**
  - Confirm the prerequisite's local-reference, inherited-presence, merge, and provenance behavior through its public tests and implementation log. Record the exact baseline revision/tree before changing union behavior.
  - Capture deterministic serialized JSON Schema for existing shipped schemas and resolver fixtures on that tree. Preserve serializer ordering; comparing parsed JSON values alone does not prove byte identity.
  - Checkpoint: the baseline passes its existing named-type tests and does not incorporate this feature's new syntax or lowering.
- [x] **Inventory Consumers**
  - Inspect `simplified/{types,grammar,serialize,convert,source,query,cursor,lint}.rs`, `resolve.rs`, `errors.rs`, and all `TypeExpr` matches after the prerequisite lands. Record the actual affected paths rather than relying on this checkout's pre-prerequisite enum.
  - Audit the JSON Schema walkers `validate`, `clean`, `rewrite`, `coerce`, and `discriminant`; typed consumers `completion`, `frontmatter_shape`, `about`, `phase`, `detect`, `triggers/matcher`, and `compose/context/catalog`; CLI `commands/schema/assignment`; DMLS `overlay/schema` and `providers/frontmatter` and their delegates.
  - Use text search and, where callers are unclear, the repo's GitNexus workflow to identify downstream public-type consumers, especially Claudine. Use Sniff with its skill for package/dependency discovery. Record scoped downstream compile/test checks for Phase 5.
  - Establish existing fixture helpers, consolidated L1 test registration, nullable-wrapper handling, literal-tag selection, and typed-error projection before assigning file ownership.

### Wave 2 — Representation Spike (Sequential, Once)

- [x] **Prove Representation**
  - Evaluate the smallest extension of the prerequisite's typed model that stores an array container separately from ordered union items. Prefer reuse of existing property-union semantics over a second array encoding or new general-purpose abstraction.
  - Prototype only the uncertain seam: passive parse of authored `U[]`, resolve with its defining table/origin, lower to `items.anyOf`, and retain enough authored state to serialize/reparse the named reference. Inspect description, marker, and default ownership in that shape.
  - Deliver a short decision record in the implementation log with the chosen fields/variant, provenance flow, serializer/display contract, and shared API/file ownership for later waves. Retain useful proof tests or remove throwaway code before production work.
  - Checkpoint: demonstrate one mixed scalar/object item union and an inherited marker without distributing array constraints into arms. If the existing authored/resolved split cannot preserve references, resolve that design here before proceeding. Do not repeat the spike later.
- [x] **Prepare Fixtures**
  - After the representation decision, establish fixtures reused by later tasks: modified Content Policy schema with bare same-file references and `long_form.rule: short_form(required)`, tagged/untagged unions, overlapping arms, eager/lazy files, multi-hop imports, and a repairable non-union sibling.
  - Seed from the shipped schema via an explicit repository read recognized by the test-input index; copy it into a hermetic temporary workspace before modifications. Freeze a valid document/control for each matrix below. Keep schema-artifact tests separate from prompt opt-in tests.
  - Confirm fixture paths and process setup use existing request-context and `CliProcessFixture` helpers, `test_toolkit`, and runtime manifest/binary lookup. Avoid ambient HOME, CWD, providers, shell execution, network access, terminal windows, and browser windows.

### Input Robustness Matrix

These are YAML inputs, not proposed constraints on type-expression strings.
Use one table-driven test per YAML input surface, each starting from a real
shipped-schema-derived positive fixture and making exactly one edit per cell.
Assert the public preparation/validation result, problem paths, and unchanged
input; parser success/failure alone is insufficient. Schema-envelope edits are
checked by attempting the document's named import. Thus an absent export table
must not become a successful empty schema through raw-schema fallback.

For definition rows, remove a field that the control document references.
`types.policy` is a YAML union sequence; `$schema.content_policy` is a reference
string. Wrong-element edits of strings/mappings replace the whole field with
the shown malformed sequence rather than pretending scalar elements exist.
Empty mappings are legal object definitions where the existing format allows
them; distinguish that explicit changed schema from an absent definition.

| Shape / YAML schema surface | Imported `kind` | Imported `types` | Imported `types.policy` | Document `$schema` | Document `$schema.content_policy` |
|---|---|---|---|---|---|
| Control | `schema`; named import resolves | Complete exports; import resolves | Ordered scalar/object arms validate the mixed list | Mapping loads the declared list schema | `policy[](min(1))@./policy.yaml`; mixed list validates |
| Absent | Named import fails: no kinded exports | Named import fails: missing table | Referenced name fails resolution | No explicit schema; baseline-only behavior, never a fabricated empty explicit schema | No list declaration; no union-specific validation, never an empty union |
| Explicit null | Named import fails | Schema preparation fails | Definition preparation fails | Public schema-shape error | Property-definition error |
| Wrong whole type | Replace with number; named import fails | Number; preparation fails | Number; definition error | Number; schema-shape error | Number; property-definition error |
| One wrong element | Mixed sequence in scalar slot; fails | Mixed sequence in mapping slot; fails | One numeric arm; whole definition fails | Root sequence containing one number; preparation fails | Mixed sequence with one numeric arm; definition fails |
| Every element wrong | Numeric sequence; fails | Numeric sequence; fails | Numeric arms; fails, never empty | Numeric root arms; fails | Numeric arms; fails, never empty |
| Empty | Empty string; named import fails | `{}`; target name unresolved | `[]` fails; `{}` explicitly changes target to an object and the control list's scalar item fails validation | `{}` is an explicit empty shape; `[]` is an invalid empty root union | Empty string/`[]` fails; `{}` is an explicit object definition and the control list fails validation |
| Duplicate key | Duplicate envelope key fails loading | Duplicate table key fails loading | Duplicate exported name fails loading | Duplicate root key fails loading | Duplicate property declaration fails loading |
| Trailing / invalid content | Invalid YAML or extra document fails loading | Same | Same | Invalid frontmatter YAML fails loading; normal Markdown body remains legal | Same |

Document-data rows use the prepared Content Policy fixture and public validation.
`content_policy` is optional and has `min(1)`; object-arm `rule` and `action`
are required. Absence and null deliberately share validity only where the
existing schema contract defines them as absence; retain their distinct input
states and assert validation does not modify either.

| Shape / YAML document surface | `content_policy` | Object-item `rule` | Object-item `action` |
|---|---|---|---|
| Control | Mixed scalar/object list validates | `ValidUntil(2027-01-01)` validates | `archive` validates |
| Absent | Valid optional property; key stays absent | Required-field problem | Required-field problem |
| Explicit null | Valid optional property; null stays null | Required/type problem | Required/type problem |
| Wrong whole type | Scalar rule rejected as non-array | Number rejected | Number rejected |
| One wrong element | Valid item plus number fails at that index; no filtering | Mixed array replaces scalar; fails | Mixed array replaces scalar; fails |
| Every element wrong | Numeric list fails; no empty-list substitution | Numeric array replaces scalar; fails | Numeric array replaces scalar; fails |
| Empty | `[]` fails `minItems` | Empty string/list/map fails | Empty string/list/map fails |
| Duplicate key | Duplicate property fails YAML loading | Duplicate nested key fails loading | Duplicate nested key fails loading |
| Trailing / invalid content | Malformed frontmatter fails; body remains legal | Same | Same |

Duplicate-key and trailing-document failures must be exercised through the
actual loader before conversion to an in-memory map. If that loader currently
accepts them, record the gap and implement the smallest fix required for these
surfaces in Phase 2; do not silently inherit last-wins or broaden the change to
unrelated formats. Before matrix sign-off, inspect load-bearing reads for
`#[serde(default)]`, absent/null-collapsing `Option<T>`, `filter_map` with
`as_str`, `unwrap_or_default`, and parse `.ok()`. A finding about one cell
requires recording every other cell in that row and column.

## Phase 2: Resolve and Lower Named Unions

### Wave 3 — Core Implementation (Sequential)

- [ ] **Extend Typed Model**
  - Implement the Wave 2 representation and its passive parsing, serialization, namespace, and origin retention. Update required/default/generated/eager queries and all exhaustive matches sufficiently to keep affected packages compiling; consumer semantics are completed in later waves.
  - Preserve existing property unions and ordinary arrays. Keep anonymous union array syntax rejected; display/catalog descriptions must explain alternatives without claiming to be round-trippable expressions.
- [ ] **Apply Union Postfixes**
  - Extend `resolve.rs::apply_import_postfix` and its callers for bare, `@this`, and cross-file unions; retain multi-hop definition scope, dependency tracking, cycle errors, and expansion limits.
  - Accept `required`, `generated`, `eager`, descriptions, and `[]`. Reject non-array value constraints by name. Reject any resolved/flattened arm whose outer value is already an array; permit arrays inside an object arm.
  - Keep inherited arm constraints and descriptions; hoist property markers without moving string bounds to array bounds. Apply existing container applicability/merge rules, including contradictory bounds, `unique`, and equal/conflicting defaults.
  - Extend structured errors to carry the property, named definition, rejected constraint/array arm/default, and available definition/use-site provenance. Do not rely on message parsing downstream. Review changed symbols' docs/comments, including the blanket-rejection comment.
- [ ] **Lower Item Unions**
  - Update `simplified/convert.rs` to emit the array separately from its flattened ordered `items.anyOf`; keep arm annotations/descriptions, enclosing description, marker semantics, and array constraints at the proper levels.
  - Apply optional-property null wrapping to the enclosing property only; do not wrap items. Confirm inherited scalar defaults fail and array-compatible defaults preserve existing validation and merge semantics without runtime insertion.
- [ ] **Verify Core Boundaries**
  - Add resolver/lowering tests covering all three reference forms, forward/local/multi-hop references, hidden array arms, nested object arrays, cycles and depth limits, marker combinations, descriptions, pre/post-array constraints, bounds, uniqueness, defaults, and structured errors.
  - Implement the YAML schema-surface matrix through public preparation/validation. Register new L1 modules in the existing consolidated targets.
  - Checkpoint: the mixed Content Policy fixture prepares; lowering contains the expected `items.anyOf`; `[null]` fails; required union properties behave correctly; unchanged baseline corpus bytes still match. Core API/file ownership is stable before parallel consumers begin.

## Phase 3: Adapt Library Consumers

### Wave 4 — Library Walkers (Parallel)

- [ ] **Report Union Items**
  - Own `schemas/validate.rs` and `discriminant.rs`. Traverse nullable container wrappers, array indices, item alternatives, and nested object fields for problem descriptions and schema-node lookup, including `/content_policy/1/rule`.
  - Reuse the shared literal-discriminant selector. Missing, unknown, duplicate, conflicting, and overlapping tags retain ordinary union reporting; any matching arm still validates. Closest-arm explanations must not authorize effects.
  - Test independent items selecting different arms and item/property parity for the same union; verify failure paths and descriptions through public reports.
- [ ] **Coerce Union Items**
  - Own `schemas/coerce.rs`. Preserve specialized boolish/numberlike recognition before generic `anyOf` handling and apply the property-union rule separately to each item.
  - Evaluate candidates from the original item independently; commit only one validating candidate. Test zero/multiple candidates remaining unchanged, nested object candidates, mixed-item choices, and item/property parity.
- [ ] **Rewrite Union Items**
  - Own `schemas/rewrite.rs`. Reuse the unambiguous eligible eager-file rule for scalar items and nested object-arm file fields, with the existing captured request context.
  - Test successful normalization using fixture files, lazy-file preservation under union/container `eager`, ambiguous alternatives without rewriting or new probes, and repeated normalization idempotence. Validation-only entry points remain read-only.
- [ ] **Preserve Cleanup Safety**
  - Own `schemas/clean.rs`. Descend through arrays but leave genuine union items unrepaired, including literal-tagged items that fail every closed object arm because of an extra key.
  - Assert tagged and untagged invalid items remain unchanged, no key disappears, their problems are reported, and a repairable non-union sibling still changes as before. Preserve specialized non-genuine-union handling.

Checkpoint: each walker has a public-result regression that would fail if it
read the `items.anyOf` wrapper as a plain object or primitive. Parallel tasks
use separate tests/modules and do not edit shared discriminant APIs until the
reporting owner publishes any necessary interface changes.

### Wave 5 — Typed Consumers (Parallel)

- [ ] **Complete Item Alternatives**
  - Own `schemas/completion.rs` and its typed query helpers. Offer object-arm keys and scalar-arm values for incomplete list items, combining applicable alternatives and narrowing only with an unambiguous literal tag.
  - Test missing/invalid/partial items, overlapping arms, different choices at neighboring indices, advisory suggestions, and no effectful evaluation.
- [ ] **Describe Typed Arrays**
  - Own `schemas/about.rs`, `frontmatter_shape.rs`, `phase.rs`, `detect.rs`, `triggers/matcher.rs`, and `compose/context/catalog.rs`. Audit each new representation branch for item traversal versus container metadata rather than adding blind wildcard fallbacks.
  - Verify descriptors expose all alternatives and descriptions, nested presence/timing remains scoped, launch/completion behavior retains the existing marker contract, and passive detection/matching/catalog inspection performs no effects.
  - Test `about`'s model and descriptor catalog output without unsupported anonymous syntax; use existing terminal components for any rendered output changes.
- [ ] **Adapt Editor Surfaces**
  - Own DMLS `overlay/schema.rs`, `providers/frontmatter.rs`, and delegated parser/hover/diagnostic walkers. Consume the canonical library representation and error tree, not an editor-specific union interpretation.
  - Hover shows an arm only for a unique valid match; missing, invalid, or multiply matching values show alternatives. Completion retains combined choices unless a literal tag selects one arm.
  - Add L1 parser, hover, completion, and schema-error tests through existing editor APIs/protocol fixtures. Verify use-site and definition related information where available; do not add item line/column support.

Checkpoint: every inventoried typed-tree consumer has a deliberate new-shape
branch or an evidenced reason its existing traversal already works. DMLS uses
the library's selection/error contracts and requires no anonymous-union parser.

## Phase 4: Prove End-to-End Behavior and Document It

### Wave 6 — Integration and Documentation (Parallel)

- [ ] **Verify CLI Pipeline**
  - Own CLI `commands/schema/assignment.rs` changes and hermetic `md schema validate`, `md schema about`, and normal compose tests. Match the existing CLI assignment rules without introducing effects into validation.
  - Exercise the acceptance fixture: valid mixed list, missing/null optional property, `[null]`, scalar input, empty list, `Duration(3mo)`, missing/null object `rule`, and invalid `action: delete`. Implement the document-data matrix through public reports.
  - Assert validation leaves source bytes and in-memory input unchanged; composition preserves current stage order, candidate coercion rules, successful eager rewriting, and repeated normalization idempotence. No default insertion is added by this feature.
  - Use `CliProcessFixture`, private fixture HOME/CWD/PATH, runtime binary paths, and explicit captured contexts; no terminal/browser/provider launch. Extend existing public tests where possible rather than creating redundant implementation-mirroring tests.
- [ ] **Update Current Docs**
  - Update `darkmatter/docs/topics/schemas/definition.md` under arrays, unions, and cross-file named imports with named `U[]`, all allowed/rejected operations, item versus container scope, presence/timing, default restrictions, and a compact Content Policy example.
  - Keep anonymous union array syntax, recursion, baseline migration, and item source positions outside the documented implemented surface. Explain reporting versus transformation safety, using a small Mermaid flow where it clarifies selection.
  - Update `darkmatter/docs/topics/schemas/dmls-schema-support.md` and `.claude/skills/darkmatter/schema.md` (and the skill index only if needed). Review relevant READMEs for public-behavior drift and update only affected claims.
  - Review comments in changed consumers and the existing `content_policy_editor_schema.rs` limitation note. Preserve the shipped schema's still-optional rule and its associated explanation; that migration belongs to the later feature. Docs must not name or link feature/fix snapshots.

Checkpoint: CLI and DMLS demonstrate the same schema semantics. The documented
example runs against the fixture, every matrix cell is asserted, and docs
describe the final implementation rather than delegating to the specification.

## Phase 5: Validate Compatibility and Prepare Review

### Wave 7 — Final Gates (Sequential)

- [ ] **Compare Compatibility Corpus**
  - Compare passive shipped-schema corpus and existing resolver-fixture lowering byte-for-byte with the Phase 1 post-prerequisite baseline, excluding only fixtures deliberately authored to use the new forms. Investigate every other difference; do not bless snapshot churn.
  - Confirm the shipped Content Policy schema is unchanged and still passes its existing tests. Run scoped downstream compile/test checks identified by the consumer inventory, including Claudine where its use of public Darkmatter types or semantics is affected.
- [ ] **Run Scoped Gates**
  - Run `just test` and `just lint` from `darkmatter/`, which cover the library, CLI, and DMLS through the area's recipes. Use nextest-backed recipes for targeted runs during implementation; do not use consolidated `cargo test`, workspace-wide gates, or `cargo fmt`.
  - All new tests are L1 and portable to macOS, Linux, native Windows, and WSL2. Avoid platform-specific path literals and ambient filesystem state. Load the `os` skill before changing Windows/path-comparison code, investigating a single-OS failure, or claiming another OS is unavailable.
  - Record the host/environment of evidence and distinguish observed passes from portability review. Use existing required OS evidence/scheduling; add no CI cells, fixtures solely for CI expansion, or L2/L3/browser checks for schema semantics.
  - Record any environment blocker with the exact failed check and supported alternative; do not claim a gate passed when it did not run. The area's lint includes the existing Zed WASM check, so account for its provisioned target without installing or launching editors.
- [ ] **Audit Acceptance Coverage**
  - Attach a concise implementation-log mapping from spec criteria 1–9 to concrete test names/results and docs. Include every walker, all boundary families, matrix row/column coverage, baseline identity, and authored-reference round trips.
  - Review the final diff for duplicate representations, accidental constraint redistribution, reporting-driven mutations, missing provenance, dropped invalid elements, new parser effects, unsupported syntax, and comment/document drift.
  - Confirm phase/wave checkpoints are complete, shared test registrations compile, the implementation log records any departures, and no unrelated files, dependency changes, or CI scheduling changes slipped in. If dependencies changed intentionally, update dependency docs in the same change.
  - Final checkpoint: report **implementation complete, ready for review** with passing evidence and any remaining limitation. Leave lifecycle placement and commits to the author.
