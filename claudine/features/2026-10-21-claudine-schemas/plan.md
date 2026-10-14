---
total_phases: 5
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: 
    - darkmatter/lib/src/markdown/schemas/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: 
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
source_files_during_phase_2: 
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - claudine/schemas/linking.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/inline-compose.yaml
    - claudine/schemas/partials/lifecycle.yaml
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/fixtures/claudine-schemas/
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/meta_schema_reference_graph.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
docs_updated_during_phase_2: 
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_2: 
    - claudine/schemas/README.md
skills_files_updated_during_phase_2: 
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/claudine/cli-commands.md
source_files_during_phase_3: 
    - Cargo.toml
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/error/render/schema.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: 
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/darkmatter/schema.md
---
# Claudine Schemas Implementation Plan

## Work summary and success criteria

Deliver five authoritative YAML files under `claudine/schemas/`, use them for disk-discovered editor validation and embedded fallback launch validation, and retire Darkmatter's duplicate Claudine schemas. The largest code change is connecting trigger discovery to Claudine's existing preparation seam while retaining winning-declaration provenance, initialization ordering, optional-value cleanup, and the frozen inline completion schema.

Success means all 14 specification acceptance criteria have observable evidence: the five files resolve; action inventories and bundle short forms agree with the parser/catalogs; three typed lifecycle levels reject malformed controls; deeper bodies retain parser enforcement; installed Claudine resolves its embedded imports without a checkout; disk shadowing and authored overrides keep their precedence; invalid Claudine controls prevent `start` and provider launch; application properties retain their existing behavior; DMLS reads the disk bundle without a Claudine dependency; old copies and live references are removed; current documentation and affected L1/lint suites agree with the implementation. Record the single required size/time measurement without a timing gate.

This plan implements `2026-10-21-claudine-schemas`. Its prerequisites are `2026-10-16-core-schema-types` and `2026-10-20-lifecycle-grammar`, including their named-type and union dependencies. No new action execution, recursive schemas, sequence task typing, schema-kind rename, trigger combinators, or CI cells belong to this work.

Each numbered wave is a dependency barrier. Tasks marked **parallel** may be assigned to separate subagents with the stated file ownership; tasks in an unmarked wave run in listed order. All downstream waves require the preceding checkpoint. A shared-file edit or test registration belongs to the integration owner. Execution is non-interactive: use existing hermetic fixtures, one-shot commands, and fake providers; never wait for feedback, launch live agents/audio, or focus windows.

## Phase 1 — Establish contracts and reduce design risk

### Necessary Rules

The specification reports no open questions. The following rules make implementation boundaries explicit; they do not reopen decisions already made.

1. **Prerequisite gate.** Confirm prerequisite APIs and lifecycle grammar in the actual checkout before implementing this feature. A dated directory or finalized spec is not implementation evidence. Record a missing prerequisite as a blocker rather than implementing its grammar or engine changes here.
2. **Schema spelling.** Use `kind: schema` for libraries and `kind: trigger-schema` for triggers, matching this spec and the landing prerequisite APIs. Do not adopt the later `schema-trigger` rename from newer drafts or the kind catalog.
3. **Authoritative definitions.** Hand-maintained YAML is the only schema definition. Embed it with `include_str!`; no build-time generator, runtime YAML copy, temporary schema extraction, or DMLS-to-Claudine crate dependency. Retain the existing action generator for its later workstream.
4. **Origin and ownership.** Reuse Darkmatter's existing `EffectiveSchema.origins` and per-property replacement rules. Extend source identity only where embedded imports and host-declaration ownership require it. Embedded identity must be distinct from a real path. Claudine owns the bundle and strict cleanup policy; Darkmatter owns generic resolution, merging, and validation. A filename/property-name blacklist alone cannot identify a winning Claudine declaration: an authored override wins and changes ownership.
5. **Resolution precedence.** The five document-context disk locations retain their order and filename shadowing before matching. Append embedded Claudine triggers only for names with no winning disk file. A disk file that does not match still shadows that name; a malformed discovered trigger produces an error. Neither case permits embedded rescue. Disk imports stay on disk; embedded relative imports stay within the bundle. Authored `$schema` remains last; do not add a baseline or a second validation pass.
6. **Validation snapshot.** Use the same effective-value snapshot, caller overlays, captured file-resolution context, literal-token decoding, and phase rules the existing preparation pipeline uses. Trigger discovery must not become a premature verdict on raw authored input. Keep shell-free `initialize` before the stabilized verdict and preserve pending/deferred lifecycle values. Reuse existing fresh-read boundaries for retry, resume, proxy, and sequence; freeze the launch schema for inline completion. No resolution per action or new per-iteration pass.
7. **Strict cleanup boundary.** Invalid values governed by winning Claudine declarations are hard failures, including optional controls. Authored application declarations retain their cleanup behavior, and authored overrides may weaken Claudine declarations as specified. Root-union authored schemas already have special merge/provenance behavior: characterize their existing result before editing and preserve it; do not fabricate property ownership for unsupported merges.
8. **Schema versus execution.** Three typed levels, loose bodies below, parser nesting limit five. Generated verbs have literal keys and untyped values; hand-action objects and parameter maps are closed. Document roots, metadata, destination maps, and `with` overlays stay open. Schema acceptance of planned verbs grants no execution support. The hand-inventory difference is exactly `break` and `prep` until implemented; `defer` is already a parser-recognized planned directive.
9. **Measurement scope.** Prior spike figures justify three levels and eliminate any new recursion/unroll comparison. They do not measure the final closed-key payload. Perform exactly one quick one-host measurement after that payload exists, before launch integration, recording resolved bytes and resolution time. No OS matrix, additional workload, statistical threshold, or test timing assertion. Any broader measurement requires an author ruling and is outside this plan.
10. **Input matrix boundary.** Matrix coverage below applies to schema-file loading and any newly changed loader fields, not Markdown/lifecycle text grammar, renderers, or schema writers. Frontmatter behavior is covered by the specification's lifecycle/launch fixtures. Preserve documented existing format semantics rather than imposing a new schema grammar.
11. **Retirement scope.** The spec's old-file inventory is illustrative. Search the entire repository for both old paths and basenames, including test corpora, prompt-test sources, comments, skill guidance, and editor examples. Preserve spec snapshots. If another historical record still names a deleted path, record and resolve its status explicitly against AC14 rather than silently exempting all history. Never edit this finalized spec to hide a departure.
12. **Review boundary.** Maintain `implementation-log.md` beside this plan with rulings, departures, measurement, baseline changes, and evidence. Leave the feature active and finish ready for review; only the author moves it to `_completed`. Do not commit, run `cargo fmt`, or change CI scheduling.

### Wave 1 — Contract inventory

- [x] **Confirm prerequisites**
  - Verify built-in `shell`/`expression`, non-null presence union semantics, required constraints through named references, `@this` precedence, union-list lowering, and the new lifecycle parser using focused existing tests.
  - Inspect the actual launch integration points: `claudine/lib/src/composition/schema/{mod,translate,classify,supplied}.rs`, `composition/prepare.rs`, and completion handling; Darkmatter's `markdown/schemas/{mod,resolve,roots}.rs` and `triggers/{discovery,assemble,envelope}.rs`; compose-stage schema validation; DMLS frontmatter and nested diagnostic paths.
  - Note both authored-schema-only gates and the optional-drop/retry paths. Establish how a composed snapshot reaches trigger matching and how a retained effective schema reaches completion. Keep original successful-launch assertions unchanged.

- [x] **Freeze inventories**
  - Inventory every current property in `claudine/docs/topics/frontmatter-properties.md`, sequence overlays, all eleven linking keys, linking reader input forms, loop controls, parser hand/bare verb inventories, and both Darkmatter generated-verb catalogs.
  - Record the specific source/test owners, payload property types, optionality, and planned descriptions; identify declarations duplicated by the inline payload and require equal types.
  - Inventory every consumer of the retiring schemas. Current additional consumers include Darkmatter CLI shipped-artifact/schema-validation tests; library `base_schema_end_to_end`, `shipped_schema_corpus`, `meta_schema_phase1`, and `schemas_constraints_block_matrix`; prompt-only identifier corpus; DMLS nested-span parity and mapping corpus. Recheck rather than trusting this list.
  - Define an AC1–AC14 ledger with test locations and owners. Register new tests in existing consolidated targets, not new CI gates.

- [x] **Agree loader seam**
  - Write the concrete minimal source/origin and fallback API contract into the implementation log: bundle name/text inputs, import anchoring, disk-name suppression, malformed-file errors, property ownership, and dependency reporting.
  - Characterize existing root-union authored-schema behavior, eager/non-eager collection, literal tokens, deferred verdicts, and optional cleanup through public results. Preserve those contracts when passing the effective schema through composition and retained completion state.
  - Finalize the input matrix below against actual loader semantics and enumerate any additional load-bearing field introduced by the seam. No implementation proceeds with an unnamed cell outcome or a silently assumed format behavior.

**Checkpoint:** prerequisites pass; authoritative inventories and source identities are recorded; loader/cleanup contracts and every applicable matrix cell are concrete. Missing prerequisite behavior is reported before dependent work begins.

### Input robustness matrix

There is no new manifest or lockfile. The read boundary is standalone schema configuration, on both disk and embedded inputs. The shipped YAML files are the real-tool fixtures; serialize those same fixtures as JSON syntax accepted by the YAML schema reader for the second format test. A `.json` raw JSON Schema is a different existing format, not a renamed trigger envelope. If the seam changes that reader, extend its existing matrix separately before claiming coverage.

Use the following outcome tables for both block YAML and JSON syntax in `.yaml` files. `Error` means a visible public discovery/resolution error with no fallback or provider launch. `Unclassified` means no tagged library/trigger classification, not an empty schema or missing disk file; disk-name shadowing still applies. For an empty object definition, assert that an object value is accepted and a scalar is rejected, distinguishing the supplied definition from a missing export. An invalid definition is always an error.

Library envelope fields:

| Shape | `kind` | `types` | Each `types.<export>` read by an import |
| --- | --- | --- | --- |
| Unedited control | Imported probe has expected valid/invalid verdicts | Every export resolves | Representative violating probe is rejected |
| Absent | Unclassified; importing the expected type fails visibly | Error for claimed library | Import of removed export fails visibly |
| Explicit null | Unclassified null discriminator; never a library | Error, distinct from absence | Error for null definition, not missing export |
| Wrong type, whole field | Unclassified invalid discriminator; never a library | Error | Error using a value outside accepted definition forms |
| Wrong type, one element | Not an element collection; wrong-type field outcome | Error for one invalid definition among valid exports | Error for one invalid union arm |
| Wrong type, every element | Not an element collection; wrong-type field outcome | Error; no filtered empty map | Error; no filtered empty union |
| Empty | Empty string is an unrecognized kind | Explicit empty export map; requested export fails | `{}` defines an object: accepts `{}`, rejects `42`; `[]` is an invalid empty union and errors |
| Duplicate key | Error before map decoding loses the duplicate | Error | Error for repeated export or nested declaration key |
| Trailing/invalid content | Error for whole document | Error for whole document | Error for whole document |

Trigger envelope fields:

| Shape | `kind` | `match` | `$schema` |
| --- | --- | --- | --- |
| Unedited control | Trigger contributes expected constraints | Matching and nonmatching documents differ | Invalid governed value is rejected |
| Absent | Unclassified; no trigger contribution | Error for claimed trigger | Error at public discovery/payload resolution |
| Explicit null | Unclassified null discriminator; never a trigger | Error, not absent/no-match | Error for supplied null payload, not missing payload |
| Wrong type, whole field | Unclassified invalid discriminator; never a trigger | Error | Error using a value outside accepted payload forms |
| Wrong type, one element | Not an element collection; wrong-type field outcome | Error for one malformed arm/condition | Error for one invalid definition among valid payload properties |
| Wrong type, every element | Not an element collection; wrong-type field outcome | Error; never empty/no-match | Error; never empty/no-schema |
| Empty | Empty string is an unrecognized kind | Error for empty/vacuous matcher | Explicit `{}` is an empty mergeable payload; no constraints, distinct from absence; unsupported empty forms error |
| Duplicate key | Error before duplicate loss | Error, including duplicated condition/combinator keys | Error, including repeated payload property keys |
| Trailing/invalid content | Error for whole document | Error for whole document | Error for whole document |

These envelope tables do not replace nested-field coverage. Wave 1 expands one column per concrete load-bearing field traversed by changed code, using the frozen inventory: every imported export, trigger condition/combinator operand, payload property definition, and any constraint block whose reader changes. For each column, select a positive and violating probe that makes the field load-bearing and write the exact outcome for all shapes. Reuse unchanged engine matrices rather than duplicating the entire SimplifiedSchema grammar. No unresolved outcome placeholders may remain in the executable/finalized table.

One matrix-walking test per supported syntax traverses the entire applicable table from the unedited shipped bundle, making one source edit per cell and asserting public discovery, effective validation, or launch results. Run it for both disk and embedded origins; syntax-invalid embedded fixtures use test-provided text, never a second production bundle. Construct duplicates/trailing garbage in source text. Check `#[serde(default)]`, `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and load-bearing `.ok()` calls; explain legitimate uses and fix silent data loss. A finding on one cell requires reporting the other cells in its row and column. Loader behavior conflicting with these outcomes is a Phase 1 ruling to resolve and log, not permission to bless an accidental default.

## Phase 2 — Author and validate the disk bundle

### Wave 2 — Independent definitions (parallel)

- [x] **Define action types** — **parallel**, action/schema-test owner
  - Author `claudine/schemas/action.yaml`: `communicate`, `flow`, `shell`, `expression`, `bundle`, and `initialize-bundle`.
  - Encode hand verbs' inventory short/long forms, required long-form payload fields, `no_error` except for `set`, zero-argument spellings, and planned descriptions for `defer`, `break`, and `prep`. Use `shell@this` and `expression@this` for local names colliding with built-ins.
  - List side-effect and expression-function catalog names as untyped literal keys. Keep category/parameter/bundle objects closed without wildcard/pattern escape keys. Bundle values copy short forms only; destination and `with` maps remain open.
  - Add inventory, bundle-key-union, short-form-equality, closure, required-through-reference, and parser-bare-name parity assertions (AC2/AC4). Keep schema support distinct from runtime implementation.

- [x] **Define linking types** — **parallel**, linking-schema owner
  - Author `claudine/schemas/linking.yaml` for all eleven optional linking keys. Match reader-supported forms, including scalar/list tool lists and scalar/list argument hints; preserve opaque metadata.
  - Test representative accepted reader forms through resolved validation. Do not normalize the public schema to a single provider's output shape.

- [x] **Prepare origin support** — **parallel**, Darkmatter resolver owner
  - Implement the agreed generic in-memory source/import seam and winning-declaration identity in Darkmatter, without importing Claudine or defining its schema text there.
  - Reuse the ordinary resolver and validator. Prove synthetic embedded relative imports, disk-origin isolation, load errors, and preservation of existing authored/baseline/trigger merge contracts.
  - Keep the seam narrowly scoped to bundle loading and provenance; do not create a new schema engine or broad virtual filesystem. Complete relevant matrix reader coverage for changed fields. This owner alone edits shared resolver/origin files.

**Checkpoint:** action/linking files load and parity tests pass; generic origin support preserves existing disk behavior. Freeze exported names before lifecycle authoring.

### Wave 3 — Lifecycle and trigger authoring

- [x] **Define lifecycle types**
  - Author `claudine/schemas/lifecycle.yaml`: list-or-bundle `stack` and `initialize-stack` families, exactly three typed conditional levels with required `when`/`then`, optional `else`, matching bundle/category/bare-name arms, and loose list-or-map branch bodies below level three.
  - Exclude shell from every typed initialize branch/root bundle. Define `loop` controls from the frozen inventory, `gate: stack`, and all four categories' loose verb short forms. Preserve parser-only placement, cardinality, `skip`, gate mixing, shell expansion, and depth rules.
  - Test depth boundaries and every exported type before building the trigger payloads (AC3/AC5).

- [x] **Define trigger payloads**
  - Replace `claudine/schemas/claudine.yaml` and add `inline-compose.yaml`. Use non-null presence unions so malformed identifying values still trigger validation. Inline selection requires non-null `prompt` and no non-null `sequence`.
  - Type the complete current frontmatter catalog, timeouts, run/loop controls, sequence overlays, and imported linking properties. Keep root payloads mergeable/open and all execution inputs optional. `prompt` is optional string; `sequence` is an optional untyped list or file-reference string. Shared declarations have identical types.
  - Delete `partials/lifecycle.yaml` and remove its directory if empty; leave `review.yaml` untouched. Add file-kind classification to `schemas/README.md`.

- [x] **Validate disk fixtures**
  - Build the complete AC5 corpus under lib/CLI fixture directories, copying applicable lifecycle spike content into stable fixtures rather than reading a moving feature directory from production tests.
  - Assert trigger selection separately from final verdict: malformed `start`/`agent`/`loop` values activate; missing/null identifiers and unrelated metadata do not; malformed non-null `sequence` defeats inline selection but is rejected by the Claudine payload. Test simultaneous triggers and open application/linking metadata.
  - Cover depth 1/3/5/6; root and conditional-body bundles; loop/gate; bare stop; valid long forms; shell in initialize at all typed sites; shell/depth defects delegated to the parser below typed levels; guarded multi-verb bundles; mixed conditional/sibling keys; misspelled verb/parameter; long forms forbidden in bundles; arbitrary generated-verb values.
  - Run fixtures without `$schema` through `md schema validate` and Darkmatter's current `with_trigger_discovery` → `effective_for` → `validate` path. Assert event/offending-key diagnostic paths, not just error counts; use isolated schema roots and established CLI fixtures (AC1–AC5).

**Checkpoint:** all five files and their exports resolve from disk, every AC5 verdict is represented, and the three-level shape has no catch-all action keys.

### Wave 4 — Required one-time measurement

- [x] **Measure final payload**
  - Adapt the lifecycle spike harness in test/spike support to the current context-taking API and resolve the real `claudine.yaml` trigger payload, including action/linking imports. Its existing `size` mode builds synthetic event properties, so it cannot be used unchanged as AC9 evidence.
  - On one host, take one quick sample of serialized resolved JSON bytes and resolution elapsed time. Log command, host, payload/source revision, measured scope, and numbers in `implementation-log.md`.
  - Report an unexpectedly large result as a bounded finding; keep the decided schema shape unless an author ruling changes it. No repeated measurement in later phases and no benchmark infrastructure or timing tests (AC9).

**Checkpoint:** the final closed-key payload has the single requested measurement; launch integration can proceed with its cost visible.

## Phase 3 — Embed the bundle and integrate launch validation

### Wave 5 — Embedded fallback integration

- [x] **Embed schema bundle**
  - Expose library name/text pairs using `include_str!` of the five authoritative paths. Resolve every exported type and both payloads using the generic source seam from Wave 2.
  - Prove byte identity, declared kinds, full import resolution with disk schemas absent and CWD unrelated to the checkout, and no temporary schema writes or repository runtime lookup (AC6).

- [x] **Apply fallback ordering**
  - Wire ordinary document-context trigger discovery plus last-resort embedded names into the one effective-schema assembly path. Preserve nearest-name shadowing, the five location roots, ignored file symlinks, relative-import anchors, authored-last replacement, and visible invalid-trigger failures.
  - Add AC13 fixtures for complete bundle locations, nearer shadowing, ignored symlinks, rejected trigger-envelope `$schema` references, same-name disk override, absent-disk embedded behavior, authored overrides, malformed disk triggers, and disk-import failure despite a same-basename embedded file.
  - Verify classification/no-match shadowing and simultaneous disk/embedded winning triggers. Disk ownership and embedded ownership must yield equivalent strict-policy classification.

- [x] **Integrate launch verdict**
  - Replace authored-`$schema`-only gates in `composition/schema/mod.rs` and `prepare.rs::resolve_launch_schema` and connect composition's validation stage to the same effective schema/snapshot. Do not merely attach a trigger schema after composition has already cleaned invalid controls.
  - Carry winning-declaration ownership through initial failure classification, drop/retry, post-composition validation, reports, retained launch schema, and completion. Hard-fail invalid Claudine-owned optional properties; keep cleanup for unrelated authored optional application values.
  - Preserve unphased `compose`, phased `inline-compose`, required-eager collection/non-interactive failure, deferred required non-eager values, pending expressions, literal data, and authored override precedence. Adjust touched public docs/comments and misleading all-optionals-drop status wording; render any changed terminal diagnostic through existing biscuit-terminal components.
  - Preserve shell-free initialize before stabilized validation and shell prohibition in early failure/finalize chains. `start` and provider launch require a passing verdict. Retain existing retry/resume/proxy/sequence fresh-read and frozen-inline-completion boundaries without adding a new resolution loop (AC10/AC11).

**Checkpoint:** a no-`$schema` typo produces a typed launch failure; valid documents retain their existing launch behavior; optional application cleanup and authored overrides still work; embedded fallback imports are checkout-independent.

### Wave 6 — Runtime and editor proofs (parallel)

- [x] **Prove launch behavior** — **parallel**, Claudine CLI-test owner
  - Extend existing `compose_schema_cli`, `sequence_schema`, and completion/preparation tests through `CliProcessFixture`, private home/CWD/schema roots, dry audio, and recording fake providers. Prove invalid controls are retained for diagnostics and the provider never ran.
  - Cover both compose commands, non-interactive required absence, required-eager absence, deferred required outputs, original valid-launch assertions, application cleanup, and authored overrides. Use existing collection tests for interactive behavior without creating windows or waiting for input.
  - Demonstrate observable initialization before a stabilized schema rejection and no `start`/provider execution. Exercise shared preparation at retry, resume, proxy-target, and sequence boundaries; demonstrate inline completion uses the launch schema even after `$schema` edits and the next run reloads it.
  - Compare CLI/editor passive fixture verdicts using identical effective values, sources, and phase; explicitly account for existing composition coercion rather than asserting parity between raw and composed snapshots (AC10).

- [x] **Prove editor behavior** — **parallel**, DMLS-test owner
  - Move `lifecycle_inventory_matches_the_authored_claudine_schema` to the new bundle paths: event keys from the trigger's payload and communication verbs from `action.yaml::communicate`. Preserve the guard during the later deletion.
  - Test DMLS trigger discovery for a document under a fixture-owned Claudine area with a complete copied bundle. Confirm lifecycle diagnostic paths and linking properties are recognized; exercise completion/hover using existing headless provider tests.
  - DMLS must read disk schema sources through existing contexts, with no embedded bundle dependency. Spell cross-package repository reads so the CI index resolves them, using `include_str!` or runtime harness roots; use existing source-input metadata rules where needed, without adding a CI cell (AC7).

**Checkpoint:** AC6/AC7/AC10/AC13 pass through public entry points; launch and editor agree for equal passive inputs; parser-only execution checks remain independently covered.

## Phase 4 — Retire duplicate schemas and publish current guidance

### Wave 7 — Consumer migration and documentation (parallel)

- [ ] **Migrate schema consumers** — **parallel**, Darkmatter test/config owner
  - Migrate every consumer found in Wave 1, including old schema corpora and prompt-only test source references omitted by the spec's initial list. Tests specifically about Claudine use the new complete bundle; generic schema/config tests use focused inline or stable test-owned schemas.
  - Preserve the constraint-block input matrix's load-bearing positive/violating controls when replacing its deleted fixture; do not simply remove those tests. Keep generic filename-completion tests independent of Claudine's catalog.
  - Update config examples, sequence fixtures, shipped-artifact lists, and nested-span module comments. Run `mapping_only_corpus`; re-bless only a reviewed, intentional removal/change with `DMLS_BLESS_MAPPING_CORPUS=1`, log the diagnostic delta/reason, and prove it passes without bless mode.
  - After replacement inventory coverage and all migrated consumers pass, delete `darkmatter/docs/schemas/claudine.yaml` and `claudine-types.yaml`. Search repository-wide for remaining old references and document any historical-reference ruling (AC14).

- [ ] **Document schema behavior** — **parallel**, documentation/skill owner
  - Update Claudine's schema topic and composition launch-validation section, relevant README guidance, `schemas/README.md`, Darkmatter's DMLS schema-support page, and the DMLS editor README.
  - Explain the five files, category/event permissions, bundles versus long forms, three typed levels and parser-only rules, generated-value limits, planned verbs, non-null trigger selection, optional inputs, and open application metadata. Give compact valid/invalid examples for readers new to the repo.
  - Explain complete regular-file bundles in repository `schemas/`, an absolute `SCHEMAS_DIR`, or captured-home `~/schemas` for documents outside the area. Individual file symlinks and a trigger-envelope `$schema` pointer are unsupported; `.dmls.toml` extension settings do not configure the CLI.
  - Include a Mermaid flow showing disk discovery/shadowing → missing-name embedded fallback in Claudine → matching payload merge → authored override → phase verdict → permitted launch. State DMLS's disk-only source boundary, strict winning-Claudine cleanup policy, and frozen completion behavior.
  - Update `.claude/skills/claudine/` architecture/command/schema guidance and Darkmatter skill guidance if its resolver workflow changes. Fix touched behavior comments; log detected drift. Update dependency docs only for actual crate changes. Topic docs must not name feature/fix snapshots (AC8/AC11).

The migration owner owns Rust/config test fixtures; the documentation owner owns Markdown guidance/skills. Coordinate editor README/config examples explicitly to avoid overlapping edits. Deletion is the final operation of the migration task, never a prerequisite for replacing coverage.

**Checkpoint:** replacement schema inventory tests pass, migrated corpora preserve their subjects, old files are gone, and current guidance describes only the supported bundle/source behavior.

## Phase 5 — Verify acceptance and hand off for review

### Wave 8 — Scoped validation (parallel)

- [ ] **Verify Claudine suites** — **parallel**, Claudine validation owner
  - Run `just test` and `just lint` in `claudine/`, using nextest-backed recipes. Include the new schema/launch matrix and existing initialization, cleanup, retry/resume/proxy, sequence, and completion regressions through normal L1 registration.
  - Verify embedded tests run with fixture-owned contexts and no checkout dependency; binary tests use fixture spawn policy and no real audio/provider. Run no L2/L3 suite merely to test YAML/launch behavior.

- [ ] **Verify Darkmatter suites** — **parallel**, Darkmatter validation owner
  - Run `just test` and `just lint` in `darkmatter/`; existing recipes include library, CLI, and DMLS. Confirm loader/merge tests, schema CLI corpus, nested-span parity, input matrices, mapping corpus without blessing, and no incidental schema regression.
  - If migrated prompt-only test sources need verification, use their existing explicit opt-in recipe/filter and inspect actual reported outcomes; keep them outside default L1/CI. Do not rerecord unrelated snapshots or change working assertions to manufacture parity.

Parallel validation processes may contend on shared Cargo locks; report that wait as build contention, not a test failure. Serialize if necessary without changing the logical independence of the tasks.

### Wave 9 — Final acceptance audit

- [ ] **Audit acceptance evidence**
  - Fill AC1–AC14 with exact test/fixture names, commands, outcomes, and the AC9 measurement. Verify matrix rows/columns are complete for all changed load-bearing reader fields and both supported syntaxes; record outcomes for disk and embedded origins.
  - Search for stale old-schema references, shell-capable initialize schema arms, catch-all action keys, required execution inputs in trigger payloads, duplicate runtime schema definitions, temporary extraction, and introduced per-action resolution. Review diagnostics for event/parameter names.
  - Review the diff for surgical scope, source-origin integrity, public docs/comment accuracy, unchanged existing launch assertions, and corpus re-bless justification. No new CI cell, broad benchmark, live-provider test, or action feature may have slipped in.

- [ ] **Prepare review handoff**
  - Record portability requirements and actual environment evidence for macOS, Linux, native Windows, and WSL2 without calling unexecuted environments passed. Reuse qualifying evidence; use existing required scheduling for missing cells, not speculative new runs. Load the OS skill before touching Windows/path-comparison code or declaring an environment unavailable.
  - Summarize changes, passing checks, unresolved blockers, departures, and the one measurement in `implementation-log.md`. Mark tasks and phase metadata according to achieved results.
  - Report implementation complete, ready for review only when all required acceptance evidence is satisfied. Do not move the feature, commit, push, or change its finalized specification as part of this plan.

**Final checkpoint:** all 14 acceptance criteria are accounted for with passing required evidence; the five-file bundle is authoritative for editor and launch validation; retired copies have no unexplained live consumers; the author has a concrete reviewable implementation and evidence log.
