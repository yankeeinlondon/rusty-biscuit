---
total_phases: 6
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_validator_probe.rs
    - darkmatter/lib/tests/l1/recursive_schema_golden.rs
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/manifest.yaml
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/inputs/
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/consumers.md
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/README.md
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-gen
source_files_during_phase_2:
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/schema/classify.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/graph.rs
    - darkmatter/lib/src/markdown/schemas/resolve/recursive.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/scope.rs
    - darkmatter/lib/src/markdown/schemas/simplified/definitions.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
    - claudine/features/2026-10-29-recursive-schema-types/inventory/contracts.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
---

# Recursive Schema Types Implementation Plan

## Phase 1: Establish Contracts and Evidence

### Necessary Rules

- **Prerequisite gate:** begin implementation only after `2026-10-14-union-type-constraints` and `2026-10-28-schema-enhancements` are implemented, including the earlier shared-table, YAML-loader, named-presence/local-name, and diagnostic-position contracts. As of 2026-10-23 the schema-enhancement work is implemented through its Phase 5 (its implementation log records the delivered conversion engine, call binder, catalog, and design-time call check); verify its final phase and the other prerequisites by behavior and implementation logs, not directory placement alone. Do not implement missing prerequisites inside this feature or capture their unfinished behavior as the golden baseline.
- **Ownership and scope:** implement in `darkmatter`, `darkmatter-cli`, and `dmls`; inspect actual downstream users of public types. This plan's location does not authorize migration of Claudine's production lifecycle schemas. That belongs to `2026-10-30-claudine-recursive-schemas`. Preserve existing unrelated working-tree edits.
- **Cycle admission:** object properties, dictionary values, array items, and tuple slots are the only descending edges. Remove all descending edges and require the remaining graph to be acyclic. A callable parameter, alias, or union arm is not descent. Reject the actual non-descending cycle with every reference's source span, including across files; an SCC containing some descending edge is insufficient proof. Recognize back-edges before checking import-chain depth; retain existing limits for genuinely acyclic chains and existing errors for root-file and trigger-payload cycles.
- **Representation:** use `TypeExpr::Ref(DefKey)` for every use of a cyclic named definition, including its first use. `DefKey` identifies namespace plus name; separate inline roots, root-union arms, embedded schemas, and files. Share finite tables through `Arc`, without owning reference cycles. Allocate deterministic opaque JSON keys in stable traversal order, escape JSON Pointer tokens, and carry an explicit identity/key map. No new hashing dependency is needed.
- **Use-site semantics:** shared definitions describe reusable values. Derive inherited/use-site required, generated, eager, nullable-union, constraints, and annotations without modifying a shared body. Optional property nullability does not make array items nullable. Keep array-container constraints separate from item constraints. Reject conflicts with both origins; retain descriptions, defaults, and suggestions. No authored depth syntax is added.
- **Raw schema scope:** retain Draft 2020-12 document semantics. Newly admitted raw definitions at baseline/trigger merge boundaries support checked `#/$defs/…` references without scope-changing `$id` or external targets. Retain simple object roots and reject root references, unions, and applicators there. Existing accepted acyclic property schemas must not become more restrictive. Re-key only with original scope accounted for; do not blindly rewrite raw documents.
- **Walker safety:** preserve `$ref` siblings, missing-target errors, and each consumer's existing union policy. Instance traversal guards schema-location/instance-location pairs; whole-schema traversal visits definitions once; editor lookup follows the cursor path plus the next editable shape. Closest-arm reporting never authorizes cleanup, conversion, or file rewriting. Validation and editor inspection remain passive; composition uses the captured request context and existing authorized transformations.
- **Measurement ruling:** the specification's growth figures already establish the need for recursion. Schedule no feasibility or performance spike and no OS timing matrix. Run the pinned-validator correctness probe once before resolver work; run the specified release timing comparison once after integration. A dependency upgrade, larger benchmark campaign, or broader raw-schema admission requires an author ruling and is outside this plan.
- **Input policy:** the expression grammar and generated JSON are excluded from the robustness matrix; authored YAML schemas and raw JSON/YAML reference-bearing schemas read at the changed boundaries are included. Distinguish absence from null even where both are valid. Native YAML null in a named definition or union arm means the null type, not an omitted definition. Raw JSON object duplicate-member policy is an explicit Phase 1 ruling: reject ambiguous duplicate members at the new reference-bearing merge ingress; if this would tighten an already accepted acyclic schema, record the conflict for the author before changing that existing path. Do not silently choose last-wins.
- **Prerequisite reconciliation (2026-10-23):** `2026-10-28-schema-enhancements` is implemented, so Waves 1, 4, and 5 name its delivered files and cutoffs instead of asking an implementer to locate them, and Wave 5 now owns the type-level consumers that prerequisite added (`is_subtype`, the variable-type projection, the design-time call check). The spec's Decided list records why. No outcome changed.
- **Unresolved decisions:** the specification has no open product questions. Record the finite table APIs, phase/use-site context representation, error variant, reference-scope API, and robustness outcomes against the completed prerequisites before dependent waves. Report a prerequisite or contract conflict as a blocker; do not guess a new product rule in this non-interactive session.

### Work and Successful Completion

Replace cyclic named-type expansion with a finite definition graph and local JSON Schema references, while retaining byte-identical acyclic lowering. Carry those definitions through merges, validation phases, transformations, CLI inspection, and DMLS assistance. Fix the historical generator's paths and freeze its committed measurement inputs independently of generation.

Success means all eleven specification acceptance criteria have concrete evidence: nested valid and invalid documents work beyond depth 5 with positioned errors; all walkers and synthetic validators follow references; editor results work at levels 1–3 without unbounded expansion; shared definitions preserve use-site semantics and scope; conversion works past its former cutoff and fails atomically; the frozen recursive schema meets the size and recorded proportional timing targets; compatibility, L1, lint, documentation, and consumer audits pass. The estimate remains **12–18 engineer-days, likeliest 15**, subject to prerequisite availability; waves describe dependency order, not a promised elapsed duration.

Wave numbers are unique across the plan. Each wave is a dependency step; finish its checkpoint before starting the next. Only waves marked **parallel** permit concurrent subagent tasks, with disjoint file ownership and stable shared APIs. Inform every implementation subagent that the entire session tree is non-interactive. Shared test registration and common fixtures have one coordinating owner; tasks use separate test modules and do not edit another task's files.

### Wave 1 — Starting Evidence (Sequential)

- [x] **Verify Prerequisites**
  - Check implementation evidence for all prerequisite contracts and run focused existing public-behavior checks through the area's nextest-backed recipes. The delivered translators and cutoffs are named in the spec (Design 9): `schemas/json_shape.rs` (`CYCLE_EXPANSIONS` 4, `MAX_REFERENCE_DEPTH` 32, `REFERENCE_BUDGET` 4,096) and `compose/expression/call_binding/translate.rs` (`TooDeep` at 32); confirm those values in code and record them as the old cutoffs.
  - Inventory `TypeExpr` matches, definition storage, lowering calls, merges, synthetic validators, typed/JSON walkers, DMLS diagnostics/navigation, and public downstream consumers. Record file ownership and test targets in an implementation log alongside this plan.
  - Confirm the completed tree's supported formats and loader outcomes against the matrix below. Resolve the raw duplicate-member compatibility ruling before raw merge work. If a prerequisite is missing, stop dependent implementation with the exact missing contract recorded.
- [x] **Probe Validator Correctness**
  - With pinned `jsonschema` 0.55 and Draft 2020-12, compile a self-recursive schema and accept a finite valid depth-8 instance while rejecting a bad value at depth 8. Include mutual recursion and a `$ref` sibling constraint that visibly rejects a value.
  - Keep this as a small L1 correctness regression. Run it once as the prerequisite probe; later ordinary suite execution is regression coverage, not another spike. Do not upgrade dependencies or benchmark the validator.
- [x] **Freeze Compatibility Corpus**
  - Capture byte-for-byte serialized JSON, including key order, after prerequisites and before recursion edits. Use an explicit manifest covering shipped schema/trigger artifacts, non-kinded pure `$schema` documents, the embedded baseline, resolver fixtures, inline roots, and root-union references.
  - Register fixture reads in the repository-read forms in the `rust-testing` skill; avoid indiscriminate discovery under package areas. Commit-worthy golden artifacts are implementation deliverables; this task does not authorize a Git commit.
- [x] **Specify Reference APIs**
  - Record `DefKey`, definition bodies/origins, table ownership, namespace allocation, identity/key mapping, effective-use metadata, phase context, and scope-aware lookup contracts. Identify every synthetic validator that must receive its original root context.
  - Agree on typed cycle/missing-target/conflict errors and diagnostic projection. Publish these interfaces before parallel consumer tasks; exhaustive matches must have deliberate handling, not wildcard suppression.

Checkpoint: prerequisite behavior is evidenced, the validator probe passes, golden bytes are frozen, and the consumer inventory and shared contracts are reviewable. Missing prerequisite evidence blocks Wave 2.

### Input Robustness Matrix

Use real schema files produced by the existing generator/tooling as starting fixtures, copied into isolated test workspaces; retain their unedited controls. Add the smallest recursive/raw-reference edits needed for these boundaries. One table-driven public-invocation test per supported format walks every applicable cell with exactly one mutation from its control, including format-level failures before conversion to a map. Assert public schema diagnostics, validation reports, and prepared behavior, not just deserializer return values. YAML tests cover tagged and pure envelopes plus raw-schema ingress; JSON tests cover raw-schema ingress only. No new TOML or JSON SimplifiedSchema loader is introduced.

`Error` means a structured schema/load diagnostic and no successful preparation or mutation. `Unresolved` means a positioned missing-target diagnostic. Scalar fields have no element shape: substitute a collection containing one/all invalid members and assert the whole-field error. Grammar-string constraints retain prerequisite coverage rather than receiving configuration-field cells.

| Shape (YAML declarations) | `kind` (tagged control) | `types` | Referenced `types.item` | Definition union arms | Pure/inline `$schema` |
|---|---|---|---|---|---|
| Control | `schema` selects tagged loading | Nonempty map resolves | Recursive body validates finite children | Valid string/map/null arms all retained | Map, file string, or nonempty root union selects authored schema |
| Absent | No tagged recognition; remaining content uses existing raw fallback, never empty tagged types | Error under `kind: schema` | Unresolved at each surviving use | Remove the union declaration: Unresolved at use | Pure file without the key is not a pure envelope; document without the key retains ordinary baseline/trigger selection |
| Explicit null | Not tagged; existing raw interpretation is observable | Error | Declares null type; object instance fails, null succeeds according to presence rules | Null arm retained and changes admitted values | Error, distinct from document absence |
| Wrong whole type | Not tagged; malformed raw keyword remains subject to raw validation | Scalar/list error | Number/boolean error | Number/boolean instead of union: error | Number/boolean error |
| One wrong element | Collection kind: not tagged; no silent filtering | Map with one invalid definition: error | Body union with one numeric arm: error | Valid arms plus numeric arm: error | Root union with one numeric arm: error |
| Every wrong element | Same whole-field outcome | Map whose definitions are all invalid: error | Numeric-only union: error | Numeric-only union: error | Numeric-only root union: error |
| Empty | Empty string is not tagged | `{}` is a valid empty table, not missing; a surviving named use is Unresolved | `{}` is empty object; `[]` is invalid union; empty expression string is error | `[]` error | `{}` valid empty schema; `[]` and empty reference string error |
| Duplicate key | YAML load error | YAML load error | Repeated named key: load error | Duplicate containing key: load error | YAML load error |
| Trailing/invalid content | YAML load error | YAML load error | YAML load error | YAML load error | YAML load error, including an extra YAML document |

Raw tables apply **separately to JSON and YAML**. They describe the new simple-object merge ingress; unrestricted raw document validation remains Draft 2020-12. A control schema has a local recursive object definition, a property referencing it, nested required children, and an array; use positive and negative instances so removing a field changes the public decision. To exercise `$id`, use a second control documenting the unrestricted document path and prove its rejection at the restricted merge path.

| Shape (raw reference scope) | `$defs` | `$defs.item` | Property `$ref` | `$id` |
|---|---|---|---|---|
| Control | Local table resolves | Object body rejects bad nested value | `#/$defs/item` follows target and siblings | Valid URI retains document scope; scope-changing use at new merge ingress errors |
| Absent | Existing references: Unresolved, not empty success | Unresolved | No reference assertion; retained siblings still constrain | Inherit current scope, not an empty URI |
| Explicit null | Error | Invalid schema: error | Error | Error |
| Wrong whole type | Scalar/list error | Numeric/string body error; boolean schemas remain valid JSON Schema | Non-string error | Non-string error |
| One wrong element | One invalid definition body: error | Object keyword with one invalid member: error | Collection instead of string: error | Collection instead of string: error |
| Every wrong element | All invalid bodies: error | Object keyword with all invalid members: error | Same whole-field error | Same whole-field error |
| Empty | `{}` is empty table; surviving reference is Unresolved | `{}` is unconstrained schema, not absent | Empty string is root reference on document path; unsupported at restricted ingress | Empty URI is an authored scope value, not absent; preserve document semantics, reject at ingress when scope rule disallows it |
| Duplicate key | YAML error; JSON rejection per Phase 1 ruling | Same; include duplicate definition names | Same | Same |
| Trailing/invalid content | Format load error | Format load error | Format load error | Format load error |

| Shape (raw structural fields, JSON and YAML) | `type` | `properties` | `required` | `items` | `anyOf` / `oneOf` (each) |
|---|---|---|---|---|---|
| Control | Object/array assertions enforced | Child assertions enforced | Named child required | Referenced item body enforced | Valid alternatives enforced; root remains forbidden at restricted ingress |
| Absent | No type assertion; other keywords apply | No declared child schemas | No required-property assertion | No item assertion | No union assertion |
| Explicit null | Error | Error | Error | Error | Error |
| Wrong whole type | Invalid scalar/object error | Scalar/list error | Scalar/object error | Numeric/string/list error; boolean schemas remain valid | Scalar/object error |
| One wrong element | Valid type plus invalid member: error | One invalid property schema: error | String plus numeric member: error | One invalid keyword inside object schema: error | One invalid schema arm: error |
| Every wrong element | Invalid-only type list: error | All invalid property schemas: error | Numeric-only list: error | All invalid keyword values inside object schema: error | Invalid-only arm list: error |
| Empty | Empty type list error | `{}` valid empty declarations | `[]` valid, no required properties | `{}` unconstrained item schema | `[]` error |
| Duplicate key | Format policy above; repeated type-list entries error | Format policy above; repeated child name covered | Format policy above; repeated required-list entry error | Format policy above | Format policy above |
| Trailing/invalid content | Format load error | Format load error | Format load error | Format load error | Format load error |

Extend these tables in Wave 1 if completed prerequisites expose additional load-bearing configuration fields in code this feature changes, such as tuple keyword readers; give each its own column before implementing its reader. Do not expand this into a rewrite of unrelated JSON Schema vocabulary. Public validation of the finite-leaf instance separately asserts required `children` absent/null failures, empty-list success, mixed/all invalid list elements failing without filtering, optional-reference missing/null success, and null list-item failure at depth 4 and beyond.

Before matrix sign-off, inspect touched reads for `#[serde(default)]`, absence/null-collapsing `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and parse `.ok()`. A review finding records the state of every other cell in its row and column. Preserve valid empty and null-type outcomes rather than hardening them into errors.

## Phase 2: Build the Finite Reference Core

### Wave 2 — Reference Graph (Sequential)

- [x] **Discover Definition Graph**
  - Own `schemas/resolve.rs`, `simplified/types.rs`, source/provenance helpers, errors, and shared schema structures. Discover reachable definitions/dependencies once and compute recursive groups, including direct, mutual, cross-file, alias-into-cycle, and repeated-import cases.
  - Classify child-value edges using the completed core-type representation. Reject cycles in the graph after descending edges are removed; report only the offending non-descending cycle, with spans from every file. Preserve non-type cycle and chain-limit errors.
- [x] **Store Typed References**
  - Add `Ref(DefKey)` and finite shared tables to `ResolvedSchema`, `EffectiveSchema`, and bundle-facing APIs. Preserve defining namespaces, origins, imports reached only on closing edges, and deterministic traversal order.
  - Preserve authored serialization and unresolved-import rejection. Update exhaustive matches to keep packages compiling while later tasks add consumer semantics. Resolve outer shape before rejecting an array postfix on an already-array target, without recursive expansion.
- [x] **Lower Recursive Definitions**
  - Own `simplified/convert.rs` and lowering context. Emit one body per reachable cyclic definition and local references for all uses; acyclic bodies still inline identically. Keep reusable shapes separate from property presence/nullability and effective use-site constraints.
  - Test stable keys, JSON Pointer escaping, namespace isolation, descriptions/defaults/suggestions, nullable unions, inherited required/generated/eager, conflicting constraints, array bounds, and two differently constrained uses of one shared definition.
  - Implement declaration-matrix coverage through public preparation and validation. Add graph boundary tests for all four allowed descent categories, alias/self-union/callable rejection, mixed descending/non-descending SCCs, and two-file diagnostic spans.

Checkpoint: a recursive schema compiles and validates a finite depth-8 instance; every unsupported cycle fails with typed provenance; repeated resolution is deterministic and golden acyclic bytes remain identical. Publish stable definition/lowering APIs for Wave 3.

## Phase 3: Preserve Scope Across Assembly and Phases

### Wave 3 — Shared Runtime Context (Sequential)

- [x] **Introduce Reference Lookup**
  - Add one scope-aware JSON lookup context for the original root, definitions, siblings, and schema location. Missing targets fail structurally; traversal exhaustion fails rather than accepting an unconstrained value.
  - Inventory and adapt context construction for every independently compiled synthetic schema in validation, union selection, coercion, discriminants, and rewriting. Publish stable lookup APIs before parallel walker work; consumers retain ownership of union decisions.
- [x] **Isolate Merge Definitions**
  - Own `resolve.rs` merge checks, `schemas/mod.rs` effective assembly, and `triggers/assemble.rs`. Admit supported property references/definitions while retaining simple object roots, document-over-baseline behavior, and trigger precedence.
  - Re-key colliding generated and admitted raw definition names with every affected reference updated in its original scope. Cover independent lowered roots, same-named types in different files, root-union arms, mixed raw/simplified arms, per-arm baseline merge, and embedded namespaces.
  - Implement raw-ingress matrix checks and scope tests: missing targets, local escaped pointers, `$ref` siblings, `$id`, external targets, and unsupported roots. Existing acyclic acceptance and raw document semantics stay unchanged.
- [x] **Project Validation Phases**
  - Own `schemas/phase.rs`. Project each cyclic definition finitely per phase with effective presence/timing derived at each use; avoid eager/deferred contamination of shared definitions.
  - Test Launch/Completion below recursion, inherited and use-site markers, nullable unions, and a definition shared between differently timed properties. Ensure phase re-lowering carries definitions and every phase validator retains root scope.

Checkpoint: merged and phase-projected schemas validate recursively without key collision, altered scope, dropped siblings, or widened baseline roots. Shared lookup/context and phase APIs are stable; all synthetic validator construction sites are accounted for.

## Phase 4: Adapt Every Consumer

### Wave 4 — JSON Walkers (Parallel)

- [ ] **Report Recursive Failures**
  - Own `schemas/validate.rs` and `discriminant.rs`. Follow references for explanations, node lookup, literal-discriminant selection, and synthetic arm validation; retain sibling constraints and existing matching policies.
  - Assert depth ≥ 6 failures have the offending JSON path and source line/column. Add reference-member discriminant tests and synthetic-validator tests that fail if root context is lost.
- [ ] **Preserve Cleanup Policy**
  - Own both walks in `schemas/clean.rs`. Follow recursive values at new instance paths without skipping deeper children, while retaining existing genuine-union cleanup restrictions.
  - Give each clean walk a public-result test through a reference; show permitted cleanup and forbidden ambiguous-union repair independently.
- [ ] **Convert Recursive Values**
  - Own `schemas/coerce.rs`, `schemas/json_shape.rs`, and `compose/expression/call_binding/translate.rs`. Replace bounded inline target expansion in both translators with reusable handles or instance-directed traversal; add no second conversion engine (`markdown/coercion/` stays the only one).
  - Assert declared conversions continue beyond the recorded old cutoff, final validation still runs, union candidate safety is unchanged, failure leaves the original input unchanged, and the engine's `max_depth` (64) limit fails explicitly as `ResourceLimit`. Extend the translator conformance test with recursive cases so both translators give the same outcome.
- [ ] **Rewrite Recursive Files**
  - Own `schemas/rewrite.rs`. Follow references using the captured request context and original eager-file/union eligibility policy; do not let explanatory arm choice grant mutation.
  - Test a successful deep rewrite, lazy/ambiguous alternatives unchanged, passive validation doing no rewrite or execution, and successful repeated normalization idempotence.

Checkpoint: every listed JSON walker, both cleanup walks, the translator, and every synthetic validator have targeted public-result tests that fail when dereference is skipped. All transformations retain existing authorization and failure semantics.

### Wave 5 — Typed Library Consumers (Parallel)

- [ ] **Resolve Completion Shapes**
  - Own `schemas/completion.rs` and reference-aware typed query helpers. Follow the finite instance path and expose the next editable properties in incomplete recursive items; retain union narrowing and annotation behavior.
  - Test repeated recursive levels and incomplete objects/list items without eagerly expanding unused descendants. Publish this stable editor-facing interface before Wave 6.
- [ ] **Describe Recursive Types**
  - Own `frontmatter_shape.rs`, `compose/context/catalog.rs`, `schemas/simplified/subtype.rs`, `compose/expression/scope/projection.rs`, `compose/expression/call_check.rs`, and audits of `about.rs`, `detect.rs`, and `triggers/matcher.rs`. `is_subtype`, `DeclaredTypes::top_level_types`, and `check_call` compare recursive references by definition identity and terminate (spec, Decided); test a category 1 and a category 5 call whose argument is a recursive-typed property, through composition's call precheck and DMLS `dm.expression.incompatible_call`. Whole-type operations visit definitions once and render named back-references; no inference/unresolved-trigger path gains file resolution merely to handle the enum.
  - Add focused public tests where resolved references are consumed; record an evidenced non-consumer conclusion for paths needing no change. Review docs/comments affected by every behavior change.

Checkpoint: all inventoried typed consumers have deliberate reference handling or a recorded reason they never receive it. Catalog and whole-type descriptions terminate; completion helpers provide stable lazy lookup to editors and CLI.

## Phase 5: Prove User Paths and Maintain Documentation

### Wave 6 — Editor and CLI (Parallel)

- [ ] **Support Recursive Editing**
  - Own DMLS `overlay/schema.rs`, bundle propagation in `overlay/mod.rs`, `providers/frontmatter.rs`, named-type navigation, and diagnostic projection. Carry library definition identities, imported dependencies, and defining file/spans; preserve the existing document-text cache design.
  - Test completion and hover at recursion levels 1, 2, and 3, including incomplete items. Instrument lookup to prove only the cursor path and next editable shape are explored. Whole-type hover uses named back-references; definition navigation reaches the correct file/span.
  - Test positioned deep validation failures and non-descending cycle diagnostics with related spans, including two files, through existing L1 editor/protocol APIs. Do not launch an editor or focus a window.
- [ ] **Exercise Normal Invocation**
  - Own CLI `commands/schema/assignment.rs` and hermetic `md schema validate`, `md schema about`, and normal composition tests. Use `CliProcessFixture`, private home/CWD/PATH, runtime binary paths, and the captured request context.
  - Cover direct and two-file recursion at depth ≥ 6; required `children` with a valid `[]` leaf; missing/null children at depth 4; optional-reference missing/null versus null list items; passive expression text; conversions beyond the old cutoff; and repeated successful normalization.
  - Assert source bytes and original input are preserved on passive validation and failed conversion. Expose the same positions/errors as DMLS and complete public matrix assertions without parser-only substitutes.

Checkpoint: CLI, composition, and DMLS demonstrate consistent recursive semantics, positioned diagnostics, finite inspection, and existing mutation policy.

### Wave 7 — Fixtures and Current Docs (Parallel)

- [ ] **Freeze Measurement Fixtures**
  - Copy committed `action.yaml` with its 143-verb table and depth-5 `lifecycle.yaml` from `2026-10-20-lifecycle-grammar` byte-for-byte into a Darkmatter fixture directory. Add a hand-written `lifecycle-recursive.yaml` using one recursive item type over that same action fixture; retain historical grammar approximations.
  - Own the historical `spike/gen-action-schema.py` path fix: derive repository root from its own location and output beside itself under `spike/schemas/`. Update usage and path-related comments; leave other inputs unchanged. Never regenerate the frozen measurement baseline from current catalogs.
  - In an isolated fresh-checkout-shaped workspace, run the generator from an unrelated CWD and assert adjacent output and no hardcoded absolute path. Separately assert frozen copies equal the committed historical inputs; use CI-indexable literal repository reads.
  - Add L1 checks for recursive serialized size ≤ 2 MB, unchanged definitions/schema as instance depth increases, and equivalent valid/invalid behavior within the unrolled fixture's supported levels; only the recursive fixture exercises deeper levels.
- [ ] **Update Current Documentation**
  - Own `darkmatter/docs/topics/schemas/definition.md`, `authoring-schemas.md`, `local-type-references.md`, `schema-triggers.md`, `dmls-schema-support.md`, and `.claude/skills/darkmatter/schema.md`. Update affected README claims and the skill index only where needed.
  - Replace the recursion ban with compact examples of valid finite leaves, supported cycles, prohibited non-descending cycles, namespace/definition versus use-site scope, nullability, phases, raw/baseline restrictions, diagnostics, and lazy editor behavior. Use a Mermaid graph/flow where it explains cycle admission or lookup clearly.
  - Verify examples against the implementation. Current docs never name or link feature/fix directories, and do not claim Claudine production-schema migration or a new schema cache. Update dependency docs only if an intentional dependency change was approved and made.

Checkpoint: fixture controls are frozen independently of generation, size/depth invariance passes, the generator works outside its launch directory, and docs describe delivered behavior rather than delegating to the spec.

## Phase 6: Validate Compatibility and Prepare Review

### Wave 8 — Performance Evidence (Sequential)

- [ ] **Measure Resolve Cost**
  - On one host in one session, using the release profile, take the median of five warm resolves for the frozen recursive and unrolled depth-5 fixtures. Exclude and report I/O and validator compilation separately; use the same resolve boundary for both measurements.
  - Record host description, build/profile, both medians, and the threshold `150 ms × (host unrolled median / 2,683 ms)` in the implementation log. Meet the proportional target; investigate a miss before declaring completion. This is recorded acceptance evidence, never an L1 timing assertion, CI gate, repeat spike, or OS matrix.

### Wave 9 — Compatibility and Gates (Sequential)

- [ ] **Compare Golden Corpus**
  - Compare all Phase 1 acyclic serialized JSON byte-for-byte, including key order and passive shipped artifacts. Investigate every mismatch rather than refreshing snapshots to accept unexplained churn.
  - Confirm graph cases, repeated resolution stability, absence of absolute paths in generated keys, scope/collision safety, and finite size. Complete the public-type consumer audit and run targeted downstream compile/tests where the changed API reaches consumers such as Claudine.
- [ ] **Run Scoped Validation**
  - Run `just test` and `just lint` in `darkmatter/`; use nextest-backed focused recipes during implementation. Account for the area's existing Zed/WASM lint check without launching/installing editors. Do not run `cargo fmt`, consolidated `cargo test`, or workspace-wide gates.
  - New behavior coverage is portable L1 for macOS, Linux, native Windows, and WSL2; no terminal/browser focus, live provider dependency, or CI cell is added. Reuse qualifying existing required-environment evidence and otherwise follow existing required checks. Load the `os` skill before Windows/path-comparison changes, single-environment failures, or a claim that an OS cannot be tested.
  - Record observed environments and results separately from portability review. Report any environment blocker with the exact failed check; a check that did not run is not a pass. Keep timing measurements separate from correctness portability evidence.

### Wave 10 — Review Handoff (Sequential)

- [ ] **Audit Acceptance Evidence**
  - Map specification criteria 1–11 to concrete test names/results, golden artifacts, documentation, and measurement entries. Include every walker/synthetic validator, matrix row/column, graph descent family, merge namespace, phase case, editor level, normal invocation, and generator check.
  - Review the final diff for silent missing-reference fallbacks, global visited-definition sets in instance walks, dropped siblings, owning `Arc` cycles, use-site contamination, depth-limited conversion, reporting-driven mutations, scope-changing rewrites, invalid-element filtering, and documentation/comment drift.
  - Record deviations in the implementation log without rewriting the finalized spec. Mark tasks only when their public checkpoint is demonstrated. Confirm no unrelated edits, accidental dependency upgrades, CI scheduling changes, or production Claudine schema migration entered this feature.
  - Final checkpoint: **implementation complete, ready for review**, with all acceptance evidence and any explicit blocker accurately reported. Leave the feature in its active directory; the author owns review closure, lifecycle moves, and Git commits.
