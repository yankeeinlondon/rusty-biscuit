---
total_phases: 5
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine
---

# Recursive Lifecycle Schema Implementation Plan

## Phase 1 — Establish Scope and Contracts

### Necessary Rules

1. **Prerequisites are implementation gates.** Verify `2026-10-21-claudine-schemas`, `2026-10-27-darkmatter-lifecycle-ownership`, `2026-10-28-schema-enhancements`, and `2026-10-29-recursive-schema-types` by their delivered APIs and tests, not directory location or status alone. The planning checkout had placeholder lifecycle schemas and no production `action.yaml`; the typed functions partial (`darkmatter/schemas/partials/functions.yaml`) and the schema-enhancement engine, binder, and catalog API have since landed, and so has `2026-10-29-recursive-schema-types` (2026-10-23; see Rule 11). Implement against the prerequisites after they land; this plan does not absorb their implementation.
2. **Ownership stays with Darkmatter.** Shared action shapes, typed effect metadata, the lifecycle parser, conversions, and deferred-value recognition remain Darkmatter-owned. Claudine selects its closed host profile. Preserve Darkmatter's tolerant baseline and the dependency direction; no Darkmatter crate depends on Claudine.
3. **Schema and parser verdicts are separate.** Schemas recurse without a conditional-depth limit. The parser counts root-list conditionals as level one, examines both branches, and rejects level six with `LifecycleStackInvalidShape` and its path. Neither surface must prioritize one error when a document contains both a type defect and a depth defect.
4. **Preserve the existing grammar.** Keep event-specific action availability, bare-action enums, short-only bundles, closed action/parameter maps, initialization shell restrictions, trigger selection, declaration precedence, and validation phases. Empty event lists remain legal. Determine empty branch, root-bundle, zero-argument, scalar, alias, and optional-event/null verdicts from the landed parser and prerequisite tests, recording schema and parser outcomes separately before changing fixtures. Do not guess missing contracts or make optional events required.
5. **Required and nullable are independent for arguments.** Parameter omission follows signature optionality; supplied null follows the parameter type. The schema system's general optional-property null allowance must not widen a nonnullable argument. Lifecycle `set` remains a destination/value mapping without long form or action-level `no_error`; it wins over catalog `set(key, value)`. Record all other existing collisions and winners; generation fails on a newly ambiguous collision.
6. **No performance spike.** The recursive resolver prerequisite already measures duplication reduction. Its results do not establish a speedup for this catalog. Keep the final resolved embedded payload limit of 2 MB; The convention exists in the recursive-types prerequisite's frozen-fixture size test: `serde_json::to_string` of the resolved JSON Schema (compact UTF-8, `$defs` included) against 2,000,000 bytes; Phase 1 records it and the measured root so the assertion is reproducible. A failure requires investigation, not a relaxed limit or new timing threshold. Wider measurements or a threshold change require an explicit author ruling and are outside this plan.
7. **Use one authoritative catalog grammar.** The historical generator in `2026-10-20-lifecycle-grammar` reads Rust with regular expressions and writes its spike `schemas/`; it now derives its paths from its own location, and Darkmatter L1 tests pin that behavior and the byte equality of frozen copies of its `action.yaml` and `lifecycle.yaml`, so leave the spike files and generator untouched. Reuse its useful projection/emission work in a maintained authoring entry point with its own inputs and outputs. Prefer Darkmatter's library APIs for expression signatures, effect descriptors, and shared parameter metadata. If authoritative YAML is read directly, use its full existing grammar. Never create a reduced signature parser or local effect-type table.
8. **Keep reads passive and tests hermetic.** Validation, launch preflight, completion, and hover must not evaluate expressions, probe the host for operand values, execute effects, or launch real providers. Tests use recording fakes and Test Toolkit fixtures. Preserve initialization's parser prohibition and runtime shell backstop, including unselected branches and `set` shell expansion.
9. **Use the delivered schema-enhancement seams (2026-10-23).** Read signatures through `expression_function_descriptors()` and each descriptor's `function_type`; judge operands with `FunctionRegistration::bind_call` (passive) as the overload authority beside the generated value schema; express deferral through the engine's `BindOptions.defer` path predicate; inline catalog helper types into generated YAML rather than importing `functions.yaml`; give callback-typed parameters no projection. The spec's "Reconciliation with the implemented schema enhancements" section records why.
10. **Do not create new CI scope.** Add ordinary L1 coverage through existing recipes and explicit repository-input declarations. Internal prompt reads use frozen copies under `tests/fixtures/frozen_prompts/`; live prompt checks remain opt-in through `just test-prompts`. No new CI cells, gates, timing infrastructure, or DMLS caching work.
11. **Use the delivered recursive-schema seams (2026-10-23).** `2026-10-29-recursive-schema-types` is implemented; the spec's "Reconciliation with the implemented recursive schema types" section records why. Write every family so each cycle crosses a property, dictionary value, list item, or tuple slot (otherwise `SchemaError::NonDescendingCycle`). Assert the absence of the unroll on authored type names and `DefKey` names, never on `$defs` keys (`item-2` is a legal collision key). Resolve a reference across layers with `EffectiveSchema::definition(&DefKey)`; `definitions()` is the document's table only. Generic recursive traversal, deep conversion, completion, and hover are delivered; Phase 4 integration should need none of its own. Criterion 11's sample uses `md schema validate -vv`. This plan previously listed nine criteria; criteria 10 (recursive containers) and 11 (validation time goal), amended into the spec on 2026-10-16, are now scheduled in Waves 5, 6, and 9.

### Work and Successful Completion

Replace the three-level schema unroll with separate ordinary and initialization recursive families, and replace untyped generated operands with catalog-derived value schemas. Integrate those projections with the existing conversion and deferred-value boundaries, then prove agreement across library validation, `md schema validate`, DMLS, and Claudine launch preflight.

Completion means all eleven specification acceptance criteria have observable evidence: recursive embedded schemas work without disk files and stay within 2 MB; generated definitions preserve full signatures and parser collision policy; event restrictions hold at every tested depth; depth-six schema/parser verdicts intentionally differ; literal mistakes block launch; deferred arguments remain supported and are checked at runtime; editor requests remain passive; regeneration is deterministic; current docs and skills describe the landed behavior; containers recurse with every event typed; the validation time sample is recorded. Finish as **implementation complete, ready for review**. Do not commit, move the feature, or run `just complete`.

The specification estimates 2–3 engineer-days, likeliest 2.5, after prerequisites land. Shared metadata gaps and additional deferred-validation integration are excluded from that estimate; record their added cost in the implementation log when discovered.

### Scheduling Conventions

Waves are globally numbered and execute in order. Tasks within a wave may run concurrently only where indicated; tasks in a serial wave execute in listed order. A later wave starts after the preceding checkpoint passes. Parallel subagents receive disjoint file ownership, the shared fixture/contract manifest, and the instruction that the entire session tree is non-interactive. The coordinating implementer owns shared test registrations and manifests so simultaneous edits cannot overwrite each other. These are implementation instructions; planning itself does not require subagents.

### Wave 1 — Readiness (serial)

- [x] **Verify Prerequisites**
  - Locate delivered schema imports, public exports, embedded source manifest, typed signature API, shared action metadata, descriptor API, template/deferred checks, conversion boundary, parser host profiles, and recursive editor support.
  - Map those actual paths into an implementation log beside this plan. Current entry points include `darkmatter/lib/src/markdown/schemas/`, `darkmatter/lib/src/effects/catalog.rs`, `darkmatter/dmls/src/`, `claudine/schemas/`, and `claudine/docs/topics/schemas.md`; prerequisite changes may relocate shared lifecycle definitions.
  - Verify the prerequisite tests demonstrate tuples, function-catalog parsing, recursive `$ref`/`$defs`, positioned errors, separate event profiles, strict launch validation, and embedded/disk origin separation. Report an unmet prerequisite as a dependency blocker rather than implementing a competing seam.

### Wave 2 — Contracts and Integration Spike (parallel)

- [x] **Record Fixture Contracts**
  - Inventory generated canonical verbs, accepted aliases, host filtering, name collisions and winners, effect overloads, exported schema types, and embedding imports. Specify the maintained generator location and its exact checkout-relative invocation after inspecting the landed authoring infrastructure.
  - Review existing schema/parser parity fixtures, including copies of the lifecycle-grammar spike examples, and write expected schema and parser verdicts before running replacements. Record every planned verdict change and its reason; leave historical spike files unchanged.
  - Complete the input robustness tables below against actual public fields and supported configuration formats. Assign shared fixture names and source positions for CLI/DMLS/launch assertions; publish that manifest before downstream test agents start.
  - Ownership: fixture inventory and matrix documentation only; coordinate new shared fixture files centrally.

- [x] **Probe Deferred Integration**
  - Run one bounded correctness spike, once, on the current host. Using one numeric catalog parameter and one supported structured operand, confirm the delivered pieces: conversion through `markdown/coercion/` (frontmatter validation of a converted copy, `schemas/coerce.rs`), deferral through `BindOptions.defer`, passive overload binding through `bind_call`, and the template parser for recognizing `{{ … }}`; then identify the event-time argument check.
  - Demonstrate `3`, `"3"`, `"three"`, a deferred scalar, and a structured deferred operand with recording evaluator/host/effect fakes. Establish how optional-but-nonnullable parameters are represented without inheriting automatic null acceptance.
  - Deliver a short API/implementation decision with a minimal reproducer and any proven missing integration. Do not benchmark, expand the spike to an OS matrix, or rerun it in later phases. Keep disposable spike code separate from production schema and shared fixture edits.
  - If the landed APIs cannot express the required projection, identify the smallest Darkmatter-owned extension; record added work and estimate before Phase 2. Do not introduce a second evaluator or conversion policy.

**Checkpoint:** Prerequisite contracts are available, current collisions and fixture verdicts are documented, robustness columns are complete, the generator invocation is chosen, and the one-time spike identifies a passive shared integration path. Unresolved prerequisite failures stop dependent implementation. Specification decisions need no new approval; departures requiring an author ruling remain explicit blockers.

### Input Robustness Matrix

This feature changes validation of YAML lifecycle configuration and generated argument values. It does not add a general Markdown parser, serializer input contract, lockfile, or manifest format. Catalogs consumed through existing Darkmatter APIs keep their existing reader contract; do not add a parallel loader. If implementation changes a catalog/configuration reader, add its own field-by-field matrix before that edit. YAML is the specified configuration format; Wave 2 verifies whether the landed invocation accepts additional configuration formats and adds separate tables/tests for any it does.

The tables below define outcome classes. **S** is schema validation, **P** is parser validation, and **R** is event-time argument binding. A rejection must appear through a public verdict/diagnostic, CLI outcome, or launch/runtime result, with the field path and source position where applicable. A parser's internal return value alone is insufficient evidence. An absent field and a supplied null are separate edits even where the contract permits both.

The ordinary-event column expands into one column for every exposed event key; likewise each `then` and `else` family gets its own column. Wave 2 replaces these family headings with the actual exported keys in the fixture manifest. Loop controls retain their prerequisite types and placement rules; list and matrix any control whose reader is changed instead of expanding scope to untouched controls.

| Shape | Each ordinary event | `initialize` | `loop.gate` | Conditional `when` | Ordinary / initialization `then` | Ordinary / initialization `else` |
|---|---|---|---|---|---|---|
| Positive control | Valid ordinary stack passes S/P | Valid shell-free stack passes S/P | Valid ordinary gate passes S/P | Valid built-in expression passes S/P | Valid corresponding stack passes S/P | Valid corresponding stack passes S/P |
| Absent | Optional event stays absent; no stack runs | Same optional-event contract | Preserve optional gate contract; no implicit stack | Reject missing required guard | Reject missing required branch | Optional branch stays absent |
| Explicit null | Preserve prerequisite explicit-null event behavior, distinct from omission | Same, with initialization profile | Record prerequisite null verdict separately | Reject null guard | Reject null required branch | Preserve prerequisite optional-branch null verdict separately |
| Wrong whole type | Reject a value outside stack/bundle union | Reject under initialization union | Reject outside ordinary stack union | Reject non-expression value | Reject outside matching stack union | Reject outside matching stack union |
| One invalid element | Reject entire stack; identify element | Same, including deep shell action | Reject invalid gate element | Reject invalid expression; no filtering | Reject entire branch | Reject entire branch |
| All invalid elements | Reject; never reduce to empty | Same | Same | Reject invalid expression | Reject; never reduce to empty | Reject; never reduce to empty |
| Empty | `[]` legal; test `{}` separately under root-bundle rules | Same with initialization restrictions | Record inherited `[]` / `{}` parser verdicts separately | Use existing expression grammar verdict | S accepts typed empty list; P retains documented empty-body rule | Same; explicit empty differs from absence |
| Duplicate key | Reject duplicate YAML key before validation; no last-wins | Same | Same | Same | Same | Same |
| Trailing / invalid YAML | Reject malformed frontmatter or extra YAML document inside its envelope | Same | Same | Same | Same | Same |

For generated operands, create **one concrete column for every parameter of every overload**, on each supported positional/long-form/bundle projection. The following are the column rules, not permission to test one representative parameter and omit the rest. Expand nullable and constrained types directly from the catalog; do not invent coercions. Positional omissions are tested by removing the slot and checking the resulting arity/binding, never by silently shifting expected argument identities.

| Shape | Required nonnullable parameter | Optional nonnullable parameter | Null-admitting parameter | Rest parameter / typed collection | `no_error` | Lifecycle `set` mapping |
|---|---|---|---|---|---|---|
| Positive control | Valid value passes S/R | Supplied valid value passes S/R | Valid nonnull value passes S/R | Valid members satisfy declared element type/arity | Supported boolean passes S/P | Valid destination/value mapping passes S/P |
| Absent | Reject missing argument | Accept omission; preserve runtime default contract | Requiredness independently determines omission | Respect required fixed prefix and declared rest arity | Preserve default error behavior | Missing `set` means no write; retain action placement rules |
| Explicit null | Reject, unless another documented overload accepts it | Reject; never substitute omission | Accept supplied null | Accept only if declared type permits null | Preserve prerequisite boolean-field null rule; record separately | Reject null action value; destination null follows open value contract |
| Wrong whole type | Reject if shared conversion cannot bind it | Same | Same | Reject wrong container; no implicit empty collection | Reject outside prerequisite boolean contract | Reject nonmapping action value |
| One invalid element | If structured, reject whole value; scalar cell uses incompatible value | Same | Same | Reject entire value; name index | Reject invalid structured value | Arbitrary destination data allowed; reject malformed YAML structure |
| All invalid elements | Reject incompatible value; no empty fallback | Same | Same | Reject entire value; never filter to empty | Reject invalid structured value | Preserve open destination-value semantics |
| Empty | Shared type/constraint verdict; absence remains distinct | Same | Same | Enforce min length / arity; accept only when allowed | Reject empty collection | Preserve prerequisite empty-map verdict separately from missing action |
| Duplicate key | Reject duplicate YAML action/parameter keys | Same | Same | Reject duplicate nested mapping keys | Reject duplicate key | Reject duplicate destination key |
| Trailing / invalid YAML | Reject invalid envelope content | Same | Same | Same | Same | Same |

A duplicate catalog signature/parameter that the grammar forbids also fails public catalog loading/generation. A valid Markdown body after closed frontmatter is not trailing YAML garbage. Deferred operands are an additional case: passive S accepts the recognized template structure, and R rejects an evaluated incompatible value. Numeric text accepted by shared conversion is not a wrong-type rejection case.

Implement one parameterized matrix test per supported format, walking **all** populated table cells from genuine, valid shipped/tool-generated fixtures with one edit per cell and a positive control for each fixture. Use representative real actions to construct controls, then derive the complete parameter inventory from authoritative metadata. Assert CLI/DMLS/launch public behavior where applicable and validate runtime binding with recording fakes. Link cells to tests so a review finding reports every other cell in the same row and column. Before closing, search touched readers for `#[serde(default)]`, conflating `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` that could hide malformed load-bearing input.

## Phase 2 — Project Typed Action Arguments

### Wave 3 — Shared Metadata and Validation (parallel)

- [ ] **Complete Effect Metadata**
  - Compare every public side-effect descriptor with Darkmatter-owned action parameter definitions. Add a coverage test that fails for a missing typed definition; use a removed-entry test fixture to prove the failure is detected.
  - Extend only proven metadata gaps, preserving overloads, parameter names, requiredness, nullability, constraints, and rest structure. Check against established runtime behavior, not guessed types from names.
  - Expose the existing shared metadata to the authoring generator as needed. Keep the generator free of local effect-type tables. Record excluded estimate cost for gaps.
  - Ownership: shared action metadata and its coverage tests; prerequisite: Wave 2 inventory.

- [ ] **Integrate Operand Validation**
  - Implement the spike's minimal shared projection hook using Darkmatter's conversion engine, a `BindOptions.defer` predicate driven by the template parser, and passive `bind_call` for overload selection. Passivity includes structured operands containing deferred values, not only whole scalar strings.
  - Add a parity test that binds every fixture call through `bind_call` and through the generated value schema and requires the same verdict and overload, including an overloaded action where both rules could disagree (two overloads that both need conversion).
  - Preserve operand constraints, source spans, and optional-versus-null distinctions. Do not add an unrestricted string alternative to typed parameters or widen `unknown` beyond genuine catalog acceptance.
  - Assert numeric `3` and `"3"` pass, `"three"` fails by path/position, and deferred scalar/structured values pass S but incompatible evaluated values fail R at the existing boundary. Recording fakes prove no evaluation, probe, or effect during S.
  - Ownership: schema projection/deferred integration and focused tests; coordinate shared metadata API changes with the other task.

### Wave 4 — Generator Projection (serial)

- [ ] **Adapt Schema Generator**
  - Implement the Phase 1 authoring entry point using authoritative signature APIs/grammar and descriptor metadata. Use checkout-relative inputs/outputs; do not run or overwrite the historical generator/spike fixtures.
  - Emit ordinary value schemas, never `function` value imports or return-type projections. Inline catalog helper types (`agentic-cli`, `ip-address`, `recent-commits`) instead of importing `functions.yaml`, with a drift test against the catalog body. Skip any function with a `function`-typed (callback) parameter and record the exclusion in the parity test. Preserve tuple positions/arity, fixed prefixes/rest tails, scalar and zero-argument forms admitted by the parser, and separate overload arms; deduplicate identical shapes only.
  - Emit nonvariadic long forms with catalog parameter names and supported `no_error`; variadic actions remain positional-only. All categories, standalone actions, and short-only bundles reference the same projected definitions.
  - Apply documented alias/collision policy, retaining lifecycle `set`; reject a missing effect type or newly ambiguous collision. Canonical completion names come from the same inventory.
  - Add structural drift comparisons covering full generated shapes, not only name lists. Demonstrate overload parity for `ensure_file`, variadic `and`/`or`, optional nonnullable arguments, legitimate `unknown`, and invalid/missing/extra/unknown parameters.
  - Generate checked-in YAML as an authoring operation. Ordinary builds embed it and never regenerate it.

**Checkpoint:** All descriptors have shared typed definitions; valid/invalid operands have correct public results; generated names and structures match full catalog signatures, aliases, collision winners, and host filtering. Missing metadata and ambiguous collisions fail explicitly. Regeneration no longer reads Rust source or uses absolute/historical output paths.

## Phase 3 — Replace the Unroll and Embed Recursion

### Wave 5 — Recursive Schemas (serial)

- [ ] **Replace Recursive Families**
  - Replace numbered stack/item/conditional types and loose deep bodies with ordinary and initialization recursive families under their proper owners. Preserve public Claudine imports/exports unless a prerequisite already moved them.
  - Replace the numbered container types (`step-2`, `task-2`, `group-2`) and the hand-unrolled `cancelled-*` family with one recursive `task`, one recursive `group`, and a recursive cancellation family (literal `then`/`else` keys, `stop` the only flow directive), also used by document-level `cancelled`. Type every container event with its family; keep scalar steps and string file-reference alternatives for `sequence:` and `group:` (spec criterion 10).
  - Keep conditionals separate from single-verb actions and bundles. Require `when`/`then`, allow `else`, and reject sibling actions or sibling `no_error`; reject `else` without `then`.
  - Reference supported bare-action enums and generated short/long forms. Initialization excludes direct shells through both branches; `loop.gate` references the ordinary family. Preserve loop placement and open document/user metadata, `with`, and `set` destination data.
  - Update the existing embedded import/source manifest for every transitive reference. Preserve disk origins and errors without embedded basename fallback.

### Wave 6 — Embedding and Grammar Evidence (parallel)

- [ ] **Verify Embedded Payload**
  - Resolve every exported type, ordinary and initialization imports, and loop gate using only embedded sources in an isolated fixture without checkout schemas.
  - Assert local `$ref`/`$defs`, absence of numbered unroll/loose deep-body definitions (by authored type and `DefKey` names, Rule 11), correct reference resolution, nested task/group references in the size-checked payload, and the final resolved payload size under the Phase 1 byte convention.
  - Add a disk-origin negative fixture with a same-basename embedded file to prove missing/broken disk imports never silently substitute it.
  - Ownership: embedding tests and import evidence; manifest edits stay coordinator-owned.

- [ ] **Verify Grammar Parity**
  - Populate production fixtures and matrix tests from the approved inventory. At depths 1, 3, and 5 cover both branches, bundles, bare actions, loop gates, malformed long forms, typo verbs, conditional/action mixtures, and initialization shells in unselected branches.
  - Preserve initialization `set` shell-expansion rejection and runtime backstop through fakes; preserve valid open application mappings and event-specific availability.
  - Assert depth 5 passes S/P; `depth6.md` passes S and fails P at the sixth conditional path. Assert `depth6-wrong-verb.md` fails S with offending key/path/span without demanding parser error ordering.
  - For criterion 10, walk container paths sequence → sequence, sequence → group, group → sequence, and group → group to container depth 4: an unknown action in each of the seven container events, each forbidden `cancelled` action (also inside `then`/`else` at conditional level 4), valid `[stop]` and `[]`, an initialization shell in each branch, and a malformed generated argument in an ordinary event; prove each container event starts its own conditional-depth count.
  - Explain each changed fixture verdict in the log. Register new tests in consolidated L1 binaries centrally; do not silently leave test files undiscovered.
  - Ownership: library schema/parser parity and shared fixture content; no CLI/DMLS production edits.

**Checkpoint:** All embedded exports resolve without disk files and fit the size budget; both recursive families preserve permissions and openness; matrix public outcomes match recorded contracts; parser-only depth behavior is proven independently of schema acceptance.

## Phase 4 — Prove CLI, Editor, and Launch Integration

### Wave 7 — Consumer Parity (parallel)

- [ ] **Verify Schema CLI**
  - Use Darkmatter's isolated `CliProcessFixture` to run `md schema validate` against the shared corpus, checking exit status, offending paths/keys, and source positions.
  - Prove the intended trigger is discovered with `md schema triggers`, independently of validation results. Preserve declaration precedence and validation phases, including disk versus embedded import behavior.
  - Assert valid depth 6 succeeds and the deep misspelled verb fails; all generated literal/deferred/matrix cases use the agreed verdicts.
  - Ownership: `darkmatter/cli` integration tests; declared repository reads only.

- [ ] **Verify Editor Parity**
  - Exercise DMLS using its existing in-process/LSP session harness, without opening an editor window. Match CLI diagnostic verdicts and source ranges for the shared corpus, including both depth-six fixtures.
  - At depth 4 in `then` and `else`, assert completion/hover for generated canonical verbs and long-form parameters. Preserve alias acceptance, initialization shell filtering, and ordinary loop-gate availability.
  - Recording fake evaluator/host/effect services prove passive requests. If integration changes are necessary, use the existing recursive traversal and shared projection; generic recursive traversal remains a prerequisite responsibility.
  - Ownership: `darkmatter/dmls` integration and tests; fixture manifest is read-only.

- [ ] **Verify Launch Boundary**
  - Use Claudine's isolated command fixture and a portable recording fake provider. A malformed generated literal must fail preflight with its control key retained and no provider invocation; absence of invocation is observable evidence.
  - Supported deferred scalar and structured arguments must survive preflight unchanged, then bind through the established event-time check. Assert valid evaluated values succeed and wrong types fail using fake action/effect services.
  - Preserve optional events, initialization prohibition/backstop, precedence, and current failure mapping; no scrubbing invalid Claudine controls and no real shell/audio/provider execution.
  - Ownership: `claudine/lib` / `claudine/cli` launch integration tests and necessary narrow adapters.

**Checkpoint:** CLI, DMLS, and preflight agree on schema verdicts and positioned errors. Parser depth failure is intentionally independent. Completion and hover traverse both branches with event filtering, and recording services show zero passive-stage evaluation, probes, effects, or invalid-provider launches.

## Phase 5 — Document and Validate the Deliverable

### Wave 8 — Current Docs and Corpus (parallel)

- [ ] **Update Current Guidance**
  - Update `claudine/docs/topics/schemas.md`, affected Darkmatter schema topic pages, and relevant `.claude/skills/claudine/` and `.claude/skills/darkmatter/` guidance. Lead with what authors can do and compact valid/invalid examples.
  - Explain recursive families, positional/long/bundle arguments, optional/null distinctions, numeric text conversion, deferred values, initialization restrictions, parser-only five-level limit, shared metadata ownership, and the exact deterministic authoring command.
  - Include a compact Mermaid flow for passive schema validation followed by parser/event-time checks. Remove current-doc claims of unrolled/untyped behavior; do not name feature directories in docs or alter historical specs/spikes.
  - Review behavior-changing symbol docs/comments for drift in touched production code. Update dependency docs only if dependencies actually change.

- [ ] **Validate Shipped Corpus**
  - Validate ordinary shipped examples and lifecycle documents plus frozen prompt copies against explicit schema/parser expectations. Declare repository-input reads using recognized patterns and source-input metadata when another package's source is read.
  - Keep live internal prompts out of ordinary L1/CI; retain opt-in `just test-prompts`. Ensure existing DMLS parity tests remain in the test graph.
  - Audit the complete per-field/per-format robustness matrix and suspicious parse patterns. Record the state of the full affected row and column for any discovered defect and fix it before closure.
  - Ownership: corpus tests/input declarations; documentation agent owns docs/skills only.

### Wave 9 — Final Verification (serial)

- [ ] **Prove Deterministic Regeneration**
  - Run the documented generator with isolated temporary outputs; compare parsed structures with checked-in YAML for all projected definitions, short forms, bundles, and host filters. Repeat to prove stable output; confirm historical/frozen fixtures are untouched.
  - Resolve the final embedded payload and report byte size using the established convention. This is the final artifact regression assertion, not a repeated spike.

- [ ] **Run Scoped Gates**
  - Run nextest through `just test` in `claudine/` and `darkmatter/`, then `just lint` in both areas. Confirm the Darkmatter area recipe includes DMLS and the chosen authoring generator's tests are included; run its existing targeted recipe if necessary.
  - All new tests are hermetic L1: no terminal/browser focus, live network, real providers/effects/shells/audio, interactive prompts, or ambient checkout/home dependence. Do not run `cargo test` or `cargo fmt`.
  - Require portable fixture paths, public process fixtures, and registrations for macOS, Linux, native Windows, and WSL2. Use existing CI scheduling for environment evidence; do not add matrix cells or claim unexecuted OS results. Load the `os` skill if an OS-specific implementation or failure requires it.
  - Fix failures and rerun affected checks; broaden only for an actual changed dependency surface or unresolved failure. Record commands/results and any evidence limitations.

- [ ] **Prepare Review Evidence**
  - Map specification AC1–AC11 to passing tests and artifact/doc evidence: AC1 embedding; AC2 catalog projection; AC3 restrictions; AC4 depth fixtures; AC5 editor/passivity; AC6 launch/corpus; AC7 docs/regeneration; AC8 conversion/deferred checks; AC9 metadata coverage; AC10 recursive containers (container paths to depth 4, every container event, forbidden `cancelled` actions, independent conditional-depth counts, DMLS `then`/`else` completion in `cancelled`); AC11 one `md schema validate -vv` sample of a nested-container Claudine document with command, document, result, host load, and stage breakdown recorded (a goal, not a gate).
  - Check the final diff for unintended grammar, permission, ownership, trigger, validation-phase, or CI-scope changes. Record departures, added metadata/integration cost, final payload size, and each deliberately changed fixture verdict in the implementation log.
  - Mark completed tasks and report implementation complete, ready for review. Preserve feature lifecycle location and specification snapshot; do not commit or move the feature.

**Checkpoint:** All eleven acceptance criteria have linked evidence; both area's required checks pass; generated definitions reproduce; current docs/skills match behavior; the final payload meets its limit; no undisclosed fixture changes or expanded CI scope remain.
