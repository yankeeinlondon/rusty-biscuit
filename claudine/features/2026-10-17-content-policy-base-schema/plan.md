---
total_phases: 5
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Content Policy Base Schema Implementation Plan

## Work and Success Criteria

Adopt Content Policy's exported editor schema in Darkmatter's runtime baseline, resolve its imports entirely from embedded build inputs, and make that baseline the default for `md schema validate`. Preserve the existing library configuration boundary, compose behavior, DMLS extension layering, schema precedence, diagnostic provenance, and passive validation contract.

Successful completion means schema-free documents receive policy-list validation in the library when its baseline is attached, `md schema validate`, `md compose`, and DMLS. Invalid entries report their instance path and source position; DMLS offers rule/action completion and hover within list entries. Built `md` and DMLS binaries work outside the checkout without schema files or host configuration. The shipped baseline is prepared and lowered once per process, disk and embedded resolution agree, and a custom document declaration still overrides the baseline. Missing `content_policy` remains missing; explicit null retains Darkmatter's optional-property semantics.

The implementation must also prove the CLI's five-level baseline selection order and existing output/exit contracts, distinguish parser/schema agreement from documented limitations, update current documentation and skills, and pass the package-area L1 and lint gates. No policy evaluation, renewal, evidence access, new public resource registry, new CI cells, or new terminal/browser tiers are part of this work.

Waves are globally numbered. Finish each wave and its checkpoint before starting the next. Tasks within an explicitly parallel wave may be delegated to separate subagents with the listed ownership boundaries. The implementation coordinator owns shared test helpers, dependency changes, and integration; subagents receive narrow tasks and the session's non-interactive constraints. Dependencies below refer to task names. File paths are repository-relative.

## Phase 1 — Establish Contracts and Prerequisites

### Necessary Rules

- The specification reports no open questions. Treat its decisions as binding; do not seek new rulings for choices it already resolves. Work is owned by Darkmatter and Content Policy even though this plan is stored under Claudine.
- Prerequisite behavior must be present before implementation: `2026-10-13-named-type-presence-and-local-names`, `2026-10-14-union-type-constraints`, and `2026-10-15-diagnostic-node-positions`. Verify behavior rather than relying on lifecycle labels. If a prerequisite fails, record the failing example and defer dependent work to that prerequisite; do not rebuild its capability here.
- Keep `DarkmatterSchemas::new` unconfigured. Change default callers and the actual runtime baseline, `darkmatter/docs/schemas/darkmatter.yaml`; the authored global catalog is a separate artifact.
- Embedded imports resolve under the repository's virtual root, using portable `/` segments. The baseline's three `..` segments are valid. Traversal above that root, host absolute/drive/UNC paths, URLs, and unregistered targets fail without fallback. An embedded namespace must have an identity distinct from every disk namespace, and must never enter filesystem watcher dependencies.
- Share parsing, named-type expansion, constraint handling, cycle detection, and lowering between embedded and filesystem sources. Keep the source-loading seam internal. Preserve public baseline accessor signatures and panic contracts. If existing public source-identity types force an API change, record the exact change and affected callers in the implementation log before continuing; prefer an internal representation that avoids it.
- Validation is read-only and syntax-only. Optional whole-list null is accepted by Darkmatter and rejected by `Policy::from_declaration`; null items and required long-form fields are rejected by both. Missing policy means no baseline materialization; parser comparison must not substitute its caller's default policy for an absent declaration.
- Adopt existing compose disable spellings through a pure predicate using captured values. Only `md schema validate` gains the specified selection rules; compose and DMLS retain their own configuration interfaces.
- Input robustness applies to YAML schema configuration and YAML policy frontmatter, not Markdown body text, generated JSON output, or the declaration-string grammar. Reject duplicate mapping keys and invalid YAML instead of silently retaining a value. Establish the existing reader's enforcement in this phase. If it lacks that contract, identify the smallest reader-boundary correction and its caller impact; do not disguise it as an embedded-only grammar or silently expand into a general parser rewrite.
- No performance or installed-operation spike is scheduled. The specification's existing embedded-reference findings answer feasibility and cost; their placeholder does not prove real union semantics. Real-artifact integration and within-process cache assertions supply the missing evidence. No wider measurement is authorized by this plan.

### Wave 1 — Contract Inventory (Sequential)

- [ ] **Verify Prerequisites**
  - Inspect and run the existing focused nextest-backed checks for constrained lists of named unions, bare local references with `(required)`, definition survival through merging, DMLS's typed view, and node-level diagnostic positions.
  - Exercise the actual target declaration `policy[](min(1))@…` with `rule: short_form(required)` in a temporary disk fixture. Assert empty-list and missing/null-rule rejection, item diagnostics, and baseline/trigger/document precedence.
  - Record existing evidence and blockers in an implementation log alongside this plan. A blocked prerequisite stops dependent waves, not independent inventory work. This is a prerequisite checkpoint, not a repeated spike.

- [ ] **Map Integration**
  - Inventory `schemas/resolve.rs` (`ImportEngine`, `NamespaceKey`, namespace cache, dependency collection), `schemas/mod.rs` (baseline `OnceLock`s and accessors), source-identity types, baseline merging, and `darkmatter/dmls/src/overlay/schema.rs`'s typed and JSON consumers.
  - Identify CLI argument/dispatch sites under `darkmatter/cli/src/args/`, `commands/schema/validate.rs::load_api`, request snapshot access, and compose's current disable predicate. Inventory tests that assume no CLI baseline or parse the authored baseline without import expansion.
  - Confirm dependency direction and feature boundaries in the manifests. Identify existing `CliProcessFixture`, DMLS stdio fixtures, Test Toolkit helpers, and all environment inputs requiring isolation. Load the OS skill before modifying Windows handling or path comparison.

- [ ] **Define Fixtures**
  - Establish one complete valid mixed-policy fixture from the specification's authoring example, with a valid long-form `rule` and `action`. Establish a second control containing every supported rule form, with `Evergreen` alone in its own declaration.
  - Define a shared declaration-case table with separately named parser and baseline-validator expectations, preserved source strings, and expected diagnostic paths/positions. Reuse it across consumers rather than constructing unrelated `p0`/`p1` properties.
  - Pin the robustness tables below to concrete mutations. For each YAML reading surface, one matrix test walks its positive fixture and changes one cell at a time; assert the public validation report, compose result, or document/schema-load failure rather than only a YAML parser return. Internal embedded-source failures separately assert typed schema errors.

**Checkpoint:** prerequisite examples pass; integration owners, matrix fixtures, and reader-boundary behavior are explicit. No unresolved contract is hidden in a later implementation task.

### Input Robustness Matrix

Only YAML is read as the changed schema/declaration format. JSON output is not a second configuration reader. Run declaration mutations in block and quoted flow-list presentations; commas inside rule strings remain quoted. Each field column uses the same positive declaration with all other fields unchanged. `Reject` means an observable failed report or load/parse diagnostic, never filtering, coercion, partial validation success, or fallback.

| YAML declaration shape | `content_policy` | Long-form `rule` | Long-form `action` |
| --- | --- | --- | --- |
| Positive control | Mixed valid list accepted by both | `ValidFor(3mo)` accepted by both | `archive` accepted by both |
| Absent | Baseline validation accepts; compose leaves absent; no parser declaration call | Both reject; position is containing mapping | Both reject; position is containing mapping |
| Explicit null | Schema accepts; parser rejects; present null remains distinct from absence | Both reject at value | Both reject at value |
| Wrong type, whole field | Number/string/object rejected by both | Number/list/object rejected by both | Number/list/object rejected by both |
| Wrong type, one element | Valid entry plus number/null rejected, with item path | Scalar field: replacing with a mixed list rejects the whole field | Scalar field: replacing with a mixed list rejects the whole field |
| Wrong type, every element | `[123]` and `[null]` rejected, never empty | Scalar field: replacing with numeric list rejects | Scalar field: replacing with numeric list rejects |
| Empty | `[]` and `{}` rejected; neither means absent | Empty string/map/list rejected | Empty string/map/list rejected |
| Duplicate key | Duplicate frontmatter key fails document loading | Duplicate entry key fails document loading | Duplicate entry key fails document loading |
| Trailing/invalid content | Invalid content inside frontmatter fails document loading | Malformed YAML containing the field fails document loading | Malformed YAML containing the field fails document loading |

Valid Markdown after the closing frontmatter delimiter is document body, not trailing YAML garbage. Direct `Policy::from_declaration` receives an already parsed value and cannot observe duplicate source keys; prove those rows at the document reader boundary, not by constructing a deduplicated map.

The embedded loader reads the existing baseline `$schema` envelope and Content Policy's `kind: schema`/`types` envelope. Test-owned tables retain the real artifact's structure and introduce one mutation; disk resolution must give the same failure or defined changed result. These columns cover the envelope fields that select how schema text is interpreted.

| YAML schema shape | Baseline `$schema` | Policy `kind` | Policy `types` |
| --- | --- | --- | --- |
| Positive control | Real baseline prepares/compiles and rejects an invalid policy | `schema` exposes named definitions | Real types expand and validate the control |
| Absent | Shipped-baseline preparation fails; never empty fallback | This artifact's types-only shape fails standalone classification | Requested policy type cannot load; preparation fails |
| Explicit null | Preparation fails; distinct from missing envelope | Invalid standalone envelope fails | Preparation fails; never empty namespace |
| Wrong type, whole field | Numeric schema declaration fails | Numeric/list/map kind fails | Numeric/string/list types container fails |
| Wrong type, one element | Mixed invalid root-union branch fails; no skipped arm | Scalar kind replaced by mixed list fails | Mapping with one malformed type definition fails |
| Wrong type, every element | All-invalid root-union branches fail | Scalar kind replaced by all-invalid list fails | Mapping of all malformed definitions fails |
| Empty | Empty mapping is an explicitly empty schema; prove it no longer enforces the policy; empty union is invalid | Empty kind string/map/list fails | Empty namespace fails the baseline's requested import |
| Duplicate key | Duplicate envelope key fails loading | Duplicate kind fails loading | Duplicate types key or duplicate type name fails loading |
| Trailing/invalid content | Malformed or additional unconsumed YAML document fails loading | Same document-load failure | Same document-load failure |

The next table covers the type definitions and reference/constraint strings whose values change policy decisions. `Long-form field definitions` means **two separately mutated columns in the executable case table**, one for `types.long_form.rule` and one for `types.long_form.action`; they are grouped here because their structural outcomes match.

| YAML schema shape | Baseline `$schema.content_policy` | `types.short_form` | `types.long_form` | `types.policy` | Long-form field definitions |
| --- | --- | --- | --- | --- | --- |
| Positive control | Real non-empty list import rejects bad list | Real rule union recognizes valid forms | Closed object requires both fields | Compact/object union survives merging | Required rule reference and action enum reject missing/null values |
| Absent | Policy becomes untyped; invalid policy is accepted, proving this field is load-bearing | Missing requested local type fails preparation | Missing requested local type fails preparation | Missing imported type fails preparation | Removed declaration rejects that key as undeclared in the closed object; prove changed diagnostics |
| Explicit null | Legal `null` type; non-null policy list is rejected, unlike absent declaration | Legal `null` type; compact-rule control is rejected | Legal `null` type; long-form control is rejected | Legal `null` type; non-null entry control is rejected | Legal `null` type; corresponding non-null scalar control is rejected |
| Wrong type, whole field | Number/bool fails preparation | Number/bool fails preparation | Number/bool fails preparation | Number/bool fails preparation | Number/bool fails preparation |
| Wrong type, one element | Mixed malformed definition list fails | One invalid union arm fails | One malformed mapping field fails | One invalid union arm fails | Mixed malformed definition list fails |
| Wrong type, every element | All-invalid definition list fails | All-invalid union fails | All-invalid field definitions fail | All-invalid union fails | All-invalid definition list fails |
| Empty | Empty string/list invalid; empty object is object schema and rejects policy list | Empty union invalid; empty object changes rule typing and rejects compact control | Empty object accepts an empty entry and rejects declared keys: assert both | Empty union invalid; empty object changes typing and rejects mixed control | Empty string/list invalid; empty object changes field to object and rejects scalar control |
| Duplicate key | Duplicate property fails loading | Duplicate named definition fails loading | Duplicate type/field key fails loading | Duplicate named definition fails loading | Duplicate rule/action definition key fails loading |
| Trailing/invalid content | Invalid YAML fails loading; malformed import or constraint fails preparation | Malformed YAML/type definition fails | Malformed YAML/field definition fails | Malformed YAML/reference fails | Malformed YAML/reference/constraint fails |

Keep source-loading failures distinct from schema-value validation failures. For intentionally well-formed mutations that weaken a schema, assert the changed public decision rather than demanding rejection of a legal schema. Include missing/minimum constraints and misspelled local/imported type names in the semantic mutation cases. Existing untouched baseline fields retain their corpus coverage; this work does not redefine them.

Before declaring these tables complete, search affected load-bearing reads for `#[serde(default)]`, null-collapsing `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()`. Explain legitimate uses and remove silent loss. Any finding records every other cell in its row and field column, not only the failing case.

## Phase 2 — Prepare the Real Embedded Baseline

### Wave 2 — Independent Foundations (Parallel)

- [ ] **Update Artifact**
  - Owner: Content Policy schema and its own tests. Change `long_form.rule` to `short_form(required)`, use bare local names in `policy`, and remove the obsolete missing-rule header note. Preserve suggestions, descriptions, regex escaping, and remaining limitations.
  - Update `content-policy/lib/tests/editor_schema.rs` as needed to prove the exported text equals the shipped artifact. Correct patterns only where the shared case table proves drift, keeping schema ownership in Content Policy.
  - Validate the artifact through the existing disk resolver. Do not add a Darkmatter dependency to Content Policy, including all-features builds.

- [ ] **Separate Sources**
  - Owner: Darkmatter internal resolver. Introduce a small internal source-loading seam around namespace acquisition in `schemas/resolve.rs`; both sources feed existing parsing and expansion.
  - Add a test-owned embedded source table and distinct namespace identities. Resolve virtual paths lexically; cover legal parent traversal, above-root traversal, absolute/drive/UNC forms, URLs, missing documents/types, malformed imports, bare local names, and existing cycle/depth/definition rules.
  - Preserve filesystem request-scoped `FileReference` resolution, dependency tracking, namespace caching, provenance, and definition isolation. Embedded entries contribute no disk watcher paths and perform no disk/network fallback.
  - Return existing typed schema errors identifying the importing virtual document and requested type. Tests exercise the internal preparation operation; no public resource registry is introduced.

**Checkpoint:** both tasks pass focused checks independently; shared expansion still passes filesystem regressions. The artifact and embedded loader are ready for integration.

### Wave 3 — Baseline Integration (Sequential)

- [ ] **Wire Baseline**
  - Depends on **Update Artifact** and **Separate Sources**. Add `content-policy` to the Darkmatter library with `default-features = false` and no `file-adapter`; update the lockfile if dependency resolution requires it.
  - Register exactly the baseline's embedded text and `content_policy::EDITOR_SCHEMA` under the two virtual paths in the spec. Add the exact `content_policy: "policy[](min(1))@../../../content-policy/schemas/content-policy.yaml"` declaration to the runtime baseline, without a default.
  - Replace bare YAML baseline loading with internal preparation, then use the existing `OnceLock` caches for resolved typed and lowered JSON representations. Keep accessor signatures and documented invalid-artifact panic behavior; no partial or empty fallback.
  - Preserve referenced definitions and their identities through baseline merging and typed DMLS views, including schema unions. Update behavior-linked accessor/resolver comments to reflect imports and virtual identity.

- [ ] **Prove Resolution**
  - Depends on **Wire Baseline**. Add a passive test that prepares, lowers, and compiles the actual shipped baseline. Update `shipped_schema_corpus.rs`, `base_schema_end_to_end.rs`, and other located corpus checks that currently bypass import expansion.
  - Compare disk and embedded real-artifact validation, typed rule alternatives, constraints, descriptions, suggestions, and required fields. Test documents and triggers using the same local type names, including union declarations, without namespace collisions.
  - Assert baseline origin, typed problem information, instance paths, and node positions for bad entries, null fields, wrong list shape, and empty list; missing-field diagnostics use the containing mapping.
  - Prove cache reuse in one process using the existing cached-reference identity or test-only preparation/lowering counters. Do not compare subprocesses or add a public instrumentation API. Assert embedded names are absent from filesystem dependencies.

**Checkpoint:** the real shipped baseline resolves and compiles; both representations agree and remain cached; disk resolver regressions remain green. Default-entry-point tasks may now proceed.

## Phase 3 — Adopt Defaults Across Entry Points

### Wave 4 — Consumer Implementation (Parallel)

- [ ] **Select CLI Baseline**
  - Depends on **Prove Resolution**. Owner: Darkmatter CLI argument definitions, dispatch, validation command, and baseline process tests.
  - Add `--no-baseline-schema`, conflict it with explicit `--schema`, and select: disable flag → explicit custom schema → truthy captured disable environment → captured `BASELINE_SCHEMA` → embedded default. Explicit custom schema wins over environment opt-out; failed custom loading never falls back.
  - Extract/reuse a pure disable-value predicate for `1`, `true`, `TRUE`, `yes`, `YES`, using the request snapshot. Preserve compose's flags and environment behavior; do not reuse its live-environment-reading helper.
  - Keep launch-context custom-path resolution and document-context schema/trigger resolution. Opt-out leaves document and trigger schemas active; `--no-trigger-schemas` remains independent.
  - Preserve JSON fields (`schema` may be null with an effective baseline), quiet behavior, and exit codes 0/1/2/3. Pretty output bases its no-schema claim on the effective schema. Use existing `TerminalRenderable` components for changed terminal messages.
  - Add table-driven process checks for the entire selection order, combined inputs, every truthy spelling, representative non-disabling values, broken custom files, the usage conflict, trigger/document validation after opt-out, and custom replacement of `content_policy`. Cover `last_updated: not-a-date` default rejection and explicit opt-out compatibility.

- [ ] **Preserve Editor Typing**
  - Depends on **Prove Resolution**. Owner: DMLS schema overlay/typed consumers and editor protocol tests.
  - Carry the resolved baseline into both typed completion/hover and compiled validation views. Make only necessary consumer changes; matching extension baselines still overlay built-in → triggers → document schema, including document replacement of `content_policy`.
  - Add DMLS tests for compact-list completion, `rule:` values, `action:` values, long-form keys, and hover on compact and long-form rules. Assert shipped suggestion text and branch descriptions survive resolution.
  - Assert item/field diagnostics and missing-field mapping positions through DMLS's protocol result, plus extension override and same-name definition isolation. Embedded entries must not become watcher dependencies.

- [ ] **Verify Compose Contract**
  - Depends on **Prove Resolution**. Owner: library declaration comparisons and compose tests; no CLI argument or DMLS source edits.
  - Replace unrelated-property editor-schema fixtures with actual `content_policy` lists in `content_policy_editor_schema.rs`. Replace `long_form_without_rule_is_not_flagged_by_the_schema` with both-reject coverage; remove mixed-`Evergreen` positive fixtures.
  - Run the shared declarations against `Policy::from_declaration` and baseline validation. Cover every supported rule/action, mixed forms, malformed durations/units/capitalization, leading zero/zero, nested properties, missing/null fields, null items, extra fields, and all documented schema limitations with separate expected results.
  - Preserve legal `FileChanged` opening-parenthesis, `&`, `^`, and `../` cases. Syntax checks for a nonexistent watched file pass without evidence or a clock; no trimming/coercion repairs invalid declarations.
  - Assert compose pending values are checked after resolution, resolved invalid values fail, the baseline never inserts absent policy/null, and an explicit document declaration retains its existing optional-null materialization behavior.
  - Assert validation preserves input bytes/frontmatter and produces no policy evidence reads, URL fetches, expression evaluation, renewal, or date stamping. Use existing observation/side-effect fixtures where available rather than introducing runtime evaluation to test passivity.

**Checkpoint:** CLI selection/output tests, DMLS protocol checks, and parser/compose comparisons pass. The coordinator resolves shared-file edits such as a pure disable predicate before merging task results.

## Phase 4 — Prove Portability and Update Behavior Records

### Wave 5 — Independent Closure Evidence (Parallel)

- [ ] **Verify Standalone Use**
  - Depends on all Wave 4 tasks. Owner: library out-of-checkout tests and built `md`/DMLS subprocess fixtures.
  - Run outside the repository with only test documents present and fixture-owned home/config/cache/temp. Scrub `BASELINE_SCHEMA`, `DARKMATTER_NO_BASELINE_SCHEMA`, `SCHEMAS_DIR`, and other schema/editor overrides through established fixture builders.
  - Prove built `md schema validate` and `md compose` accept the valid policy and reject the acceptance-case invalid policies; prove built DMLS validates and completes over stdio. Use existing protocol fixtures with bounded waits and child teardown, never an editor UI.
  - Prove a library baseline load outside the checkout and a second load's same-process cache reuse. Subprocess success must not depend on installed host binaries, schema paths, inherited configuration, or launch CWD.
  - Execute every robustness-table mutation and positive control, asserting observable outcomes. Complete the load-bearing read audit and fix or explicitly report any reader-boundary gap identified in Phase 1.
  - Keep fixtures portable across macOS, Linux, native Windows, and WSL2. Use Test Toolkit/runtime manifest and binary lookup conventions; declare repository reads in CI-indexable forms. Use existing OS coverage without adding cells or workloads.

- [ ] **Update Behavior Records**
  - Depends on all Wave 4 tasks. Owner: documentation and skills; coordinate any artifact header correction with its owner.
  - Update Darkmatter's schema definition, runtime baseline contract, CLI schema-validation documentation, and `docs/topics/schemas/dmls-schema-support.md`. Explain effective schema precedence, captured CLI selection, compatibility change, opt-out examples, node diagnostics, and passive validation.
  - Update affected Darkmatter/Content Policy READMEs, `content-policy/docs/topics/policy-lifecycle.md`, `darkmatter/docs/dependencies.md`, and repository `docs/dependencies.md`. Record the dependency direction and disabled `file-adapter` feature.
  - Update Darkmatter and Content Policy skills for resolved embedding, real list support, required `rule`, and remaining parser/schema differences. Remove obsolete planned wording and missing-rule limitations; retain calendar/overflow/path, whole-null, and cross-entry limitations with examples.
  - Current docs describe behavior directly without naming or linking this feature. Add a compact example for each selection rule and a Mermaid precedence flow where helpful. Update the implementation log for departures; leave the spec as its original decision snapshot.

**Checkpoint:** out-of-checkout operation works without schema artifacts, input tables have complete evidence, and documentation matches the implemented behavior.

## Phase 5 — Validate and Prepare for Review

### Wave 6 — Final Gates (Sequential)

- [ ] **Run Package Gates**
  - Depends on **Verify Standalone Use** and **Update Behavior Records**. Run `just test` then `just lint` in `darkmatter/`, and `just test` then `just lint` in `content-policy/`. Darkmatter's L1 recipe includes DMLS; Content Policy lint includes its dependency-direction check.
  - Use nextest-based canonical recipes, not `cargo test`. Do not run lint concurrently with tests, invoke `cargo fmt`, add CI scope, or run real-terminal/browser tiers for this feature.
  - Confirm default/all-feature dependency direction and that Darkmatter does not enable `file-adapter`. Check current OS evidence through existing coverage; report actual executed environments and remaining coverage accurately. Consult the OS skill for environment-specific failures or additional existing-host validation.
  - Fix scoped regressions, rerun the affected focused checks, and rerun final gates only when those changes invalidate their passing evidence.

- [ ] **Audit Acceptance**
  - Map specification criteria 1–7 to evidence: default acceptance/no insertion; invalid-entry diagnostics and editor features; standalone operation/cache reuse; disk/embedded equivalence and source isolation; CLI precedence/compatibility; shared declarations/passivity/compose semantics; canonical gates and portable fixtures.
  - Review every behavior-changing symbol's comments for drift. Inspect the final diff for copied schema text, runtime evidence evaluation, silent fallback, disk watcher pollution, accidental API/default-constructor changes, and unnecessary adjacent edits.
  - Check each robustness row and column is complete, including positive controls and legal schema mutations that intentionally change decisions. Record remaining risks, exact blockers, and validation results in the implementation log.
  - Mark only completed tasks and report **implementation complete, ready for review** when all checkpoints pass. Leave the feature directory in place; do not run `just complete`, move it into `_completed`, or commit without explicit authorization.

**Final checkpoint:** all acceptance criteria have observable passing evidence and current behavior records. Any unresolved prerequisite, input contract, or failed gate is reported as incomplete rather than converted into success.
