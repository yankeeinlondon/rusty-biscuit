---
spec: claudine/features/2026-10-21-claudine-schemas/spec.md
review: claudine/features/2026-10-21-claudine-schemas/review-2.md
implementation_2: "2026-10-14T00:35:15-07:00"
---

## Implementation of Review Findings #2

> **started at:** 2026-10-14T00:35:15-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/features/2026-10-21-claudine-schemas/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the earlier phase-by-phase record is in `implementation-log.md` (its open item F2 is the same defect as R2-F1)
- review-2's `implemented` flag was confirmed `false` before repairs started

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1   | [high] Trigger schemas do not supply the promised editor completion and hover | fixed | all 15 child rows fixed or clean; darkmatter `just test` 9997/9997, claudine `just test` 9063/9063, both lints exit 0 |
| R2-F1.1 | Top-level key completion: `mod` offers `mode` from trigger payload | fixed | `claudine_schema_discovery::trigger_payload_keys_complete_without_a_schema` — `mod` gives exactly `mode`, `model`; none outside the Claudine area |
| R2-F1.2 | Nested key completion: `start: [{say: {te}}]` offers `text` | fixed | same test — nested `te` under `say:` gives exactly `text` |
| R2-F1.3 | Lifecycle item key completion: `- sa` offers `say` | fixed | same test — `- sa` gives `say` and only `sa*` verbs |
| R2-F1.4 | Union item completion: empty `- ` item offers action keys and bare verbs incl. `stop` | fixed | `trigger_payload_items_and_values_complete_without_a_schema` — empty item offers `say`, `notify`, `shell`, `set`, `when`, `stop`, `skip`, `retry`; no top-level keys |
| R2-F1.5 | Loop key completion: `loop: {ma}` offers `max` | fixed | `trigger_payload_keys_complete_without_a_schema` — `loop:`/`ma` gives `max`, no top-level keys |
| R2-F1.6 | Scalar enum value completion: empty `mode:` offers `append`/`replace` | fixed | `trigger_payload_items_and_values_complete_without_a_schema` — exactly `append`, `replace` |
| R2-F1.7 | Boolean value completion: empty `interactive:` offers `true`/`false` | fixed | same test — exactly `true`, `false` |
| R2-F1.8 | Top-level hover: `mode: append` describes enum and purpose | fixed | `trigger_payload_properties_hover_without_a_schema` — enum type, values, description |
| R2-F1.9 | Lifecycle parameter hover: `text` in `say` describes type and action category | fixed | same test — `string`, `Required`, "matches alternative" naming the communication verbs; same rendering as the authored control review-2 judged clean (literal name `communicate` not printed; nonblocking note below) |
| R2-F1.10 | Loop property hover: `max` describes integer constraint | fixed | same test — `max` shows `number` and its description |
| R2-F1.11 | Linking property hover: `max_turns` describes integer type | fixed | same test — `max_turns` shows `number` and its description |
| R2-F1.12 | Authored `$schema` override still wins over trigger declaration (per-property precedence) in completion/hover | fixed | `an_authored_redeclaration_wins_over_the_trigger_per_property` (DMLS) and three lib `property_shape_*` tests (registry order, non-matching trigger excluded, authored wins, base/extension under triggers) |
| R2-F1.13 | Trigger-based diagnostics unchanged (control) | clean | diagnostics read the merged `json_schema`, untouched; existing diagnostics/discovery tests pass unedited |
| R2-F1.14 | Sibling sweep: every other DMLS consumer of the effective simplified shape (e.g. code actions, inlay hints, expression completion, semantic tokens) | fixed | code-action scaffold, interpolation hover, `undeclared_property` (new test `a_trigger_declared_property_is_not_an_undeclared_expression_root`), meta-schema activation, `md schema validate key=value` coercion (2 CLI unit tests) fixed; authored-parameter consumers clean by contract (see Work Log) |
| R2-F1.15 | Docs: `claudine/docs/topics/schemas.md`, DMLS docs, claudine skill stop describing the editor as diagnostics-only | fixed | schemas.md, dmls-schema-support.md, definition.md, editors README, dmls README, claudine and darkmatter skills updated |
| R2-O1   | [observation] previous review (`review-1.md`) is unavailable | deferred | nonblocking observation; `review-1.md` is absent from checkout and history, nothing to restore in this cycle |

### Work Log

- starting the work on 'R2-F1 trigger schemas missing from editor completion and hover' at 00:35:45
        - defect class: schema-derived editor projections (completion, hover, and any other shape-driven editor feature) read only the authored declaration, so properties contributed by matching trigger payloads reach validation but no editor projection
        - root cause located: `DarkmatterSchemas::effective_for_with_override` (`darkmatter/lib/src/markdown/schemas/mod.rs`) sets `simplified` from the authored `$schema` only, while `json_schema` merges trigger layers; DMLS `known_shape` builds its shape from baseline + extensions + that authored `simplified`
        - delegated to one foreground rust-developer subagent (single finding, serial)
        - discovery: trigger payloads kept no SimplifiedSchema after resolution — `triggers::assemble::ResolvedPayload` held only `json_schema`, dependencies, and advisories, although both resolution paths (disk `resolve_yaml_schema_with_roots`, embedded `prepare_embedded_*`) produce a `ResolvedSchema` with `simplified`
        - discovery: the library already had the one editor-shape authority, `schemas::effective_property_shape` (base → extensions → authored), but DMLS duplicated it twice (`providers/frontmatter.rs::known_shape`, `overlay/schema.rs::effective_shape`); neither copy could see triggers
        - design: retain each payload's object shape at registry-scan time (`ResolvedPayload::shape: Option<Arc<SchemaShape>>`, carried on `TriggerLayer`), store the matching ones on `EffectiveSchema` (`trigger_shapes`, private, accessor `trigger_shapes()`), and add `EffectiveSchema::property_shape(extension_shapes, frontmatter)` = base → extensions → matching triggers in registry order → authored `$schema`, whole top-level property replacement (the `merge_baseline` precedence validation uses); `effective_property_shape` keeps its signature and delegates to the shared `layered_property_shape` with no triggers
        - design: `EffectiveSchema::simplified` keeps its meaning (authored `$schema` alone) so every "authored parameters" consumer is unchanged; its `///` now says so and points to `property_shape`
        - design: both DMLS copies now call the library authority (`known_shape` with the document's frontmatter for discriminant narrowing, `effective_shape` with `Null`, which is the merged-arms behavior it had); the duplicated DMLS union helpers were removed; DMLS gains no Claudine dependency
        - sibling sweep, DMLS (all shape-driven features go through `known_shape` or `effective_shape`):
                - key, nested key, list item, union item, enum/boolean value completion and frontmatter hover (`providers/frontmatter.rs`, via `known_shape`) — fixed
                - code action "add missing key" literal scaffold (`code_actions.rs`, via `known_shape`) — fixed (now scaffolds a trigger-declared `literal(...)`)
                - interpolation hover for a declared-but-unset property (`dsl.rs::schema_property_hover`) — fixed
                - `dm.expression.undeclared_property` (`dsl.rs::KnownRoots`) — fixed: was a pre-existing false warning for an unset trigger-declared property, disagreeing with compose (`compose/unknown_identifiers.rs` uses `declares_top_level_property` on the merged JSON); new test covers it
                - meta-schema value activation (`overlay/schema.rs::frontmatter_authoring`, via `effective_shape`) — fixed (trigger-declared `type-definition`/`schema` values now activate)
                - expression property-name completion (`providers/frontmatter.rs::expression_property_keys`, reads `effective.simplified`) — clean by policy: authored parameters only, as the review instructs; documented in `dmls-schema-support.md`
                - diagnostics (`diagnostics/frontmatter.rs`) — clean: validate against `json_schema`, which already merged triggers; related-information uses the origin map, not the shape
                - semantic tokens, document symbols, folding, rename/references — clean: read no schema shape (grep of `SchemaShape`/`PropertyDef` across `providers/` and `diagnostics/` finds only `frontmatter.rs`, `dsl.rs`, `code_actions.rs`); DMLS has no inlay-hint or signature-help provider that reads frontmatter schema
        - sibling sweep, darkmatter lib and CLI:
                - `md schema validate key=value` coercion (`cli/src/commands/schema/assignment.rs::force_string_for_path`) — fixed: the caller builds the effective schema with triggers "to coerce to declared property types" but only the authored shape was consulted; now authored first, else the last matching trigger declaring the property
                - `schemas::completion::{for_property, completable_properties}` (used by Claudine's shell completion of prompt parameters) — clean by policy: authored parameters, not host controls; documented in `definition.md` "Shell-Completion Integration"
                - `compose/schema_validation.rs::materialize_optional_document_bindings` and `compose/directive_targets.rs` — clean: their contract is document-owned properties only (origin-gated, comment names trigger payloads as excluded)
                - `EffectiveSchema::validate_for_phase_with_positions` phase projection of `simplified` — not an editor projection; trigger shapes are carried into the projected schema unchanged; out of this class
                - Claudine consumers of `simplified` (`composition/schema/{classify,translate,supplied,mod}.rs`, `inline_prompt.rs`, `cli/src/completion/schema_completion/keys.rs`) — clean: authored-schema semantics by contract (`frontmatter-properties.md`: "Argument-token ownership and caller eager-file completion read this authored schema alone"); field meaning unchanged
        - tests added: lib `schemas::phase4_trigger_assembly::{property_shape_layers_matching_triggers_in_registry_order, property_shape_lets_an_authored_redeclaration_win_per_property, property_shape_puts_triggers_over_the_base_and_extension_layers}` (two matching triggers + one non-matching, each asserting expected and absent properties and that validation applies the same precedence)
        - tests added: DMLS `claudine_schema_discovery::{trigger_payload_keys_complete_without_a_schema, trigger_payload_items_and_values_complete_without_a_schema, trigger_payload_properties_hover_without_a_schema, an_authored_redeclaration_wins_over_the_trigger_per_property, a_trigger_declared_property_is_not_an_undeclared_expression_root}` (public LSP requests, shared fixture, no `$schema`, one session per test, documents closed after each request; 2.7–3.5 s each under full-suite load)
        - tests added: CLI `commands::schema::assignment::tests::{coerces_to_string_when_a_matching_trigger_says_string, an_authored_declaration_outranks_the_trigger_for_coercion}`
        - mutation check: with `property_shape` fed no trigger shapes, all five new DMLS tests fail and the four other discovery tests still pass
        - existing test updated: `strict_mode_recovery_spike::spike_recovery_protocol_shared_import_failure_repair_and_directory_recreation` pinned the old gap ("current trigger-only completion gap"); it now asserts the trigger-only document completes and hovers from the payload, and in the failure route that its completion and hover agree with its (last-good) diagnostics
        - hover note: `text` in a `say` item reports its string type, `Required`, and "matches alternative 1 of 7" listing the communication verbs; the literal type name `communicate` is not shown because import names are not kept on resolved inline objects — identical to the authored-`$schema` control the review judged clean
        - docs: `darkmatter/docs/topics/schemas/dmls-schema-support.md` (diagram, details, Claudine section, coverage note 16, extensions paragraph), `claudine/docs/topics/schemas.md` ("In the editor"), `darkmatter/dmls/docs/editors/README.md`, `darkmatter/docs/topics/schemas/definition.md` (completion module reads the authored schema), `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/dmls.md`; drift fixed in `darkmatter/dmls/README.md` ("pure config (globs → baseline schema)" → trigger schemas on disk)
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `cd darkmatter && just test`: 9997 passed, 10 skipped, exit 0
        - `cd darkmatter && just lint`: exit 0
        - `cd claudine && just test`: 9063 passed, 9 skipped, exit 0
        - `cd claudine && just lint`: exit 0 (only the known macOS `__eh_frame` linker note)
- work completed for 'R2-F1 trigger schemas missing from editor completion and hover' at 01:01:47
        - orchestrator reconciliation: inspected the combined diff (16 files), confirmed `EffectiveSchema::simplified` keeps its authored-only meaning and the new `property_shape` layers base → extensions → matching triggers → authored per top-level property, matching `merge_baseline`; re-ran both packages' `just test` and `just lint` independently (results below)
        - the `spec.md` diff (`review_iterations: 2`) predates this cycle and was not made by it

### Completion Report

- **status:** complete
- **elapsed:** 2026-10-14T00:35:15-07:00 → 01:03 (about 28 minutes)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
- **child rows:** 15 (14 fixed, 1 clean); no child row open
- **observations:** R2-O1 deferred (nonblocking): `review-1.md` does not exist in the checkout or Git history, so it cannot be restored here
- **verified repair (R2-F1):** matching trigger payloads now reach every shape-driven editor projection in DMLS through one library authority, `EffectiveSchema::property_shape`, so a Claudine prompt with no `$schema` gets key, nested-key, item, union-item, enum, and boolean completion plus hover from the disk bundle, with authored `$schema` declarations still winning per property; DMLS gains no Claudine dependency and its two duplicate shape assemblies were removed
- **nonblocking note (R2-F1.9):** hover identifies a single-verb item's category by its matching alternative and the category's verb list, not the literal type name `communicate`; import type names are not kept on resolved inline objects, and authored `$schema` hover renders it the same way, which review-2 judged clean. Printing the literal name would be a separate hover-rendering change for all imported types
- **changed files:**
        - library: `darkmatter/lib/src/markdown/schemas/mod.rs`, `darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs`, `darkmatter/lib/src/markdown/schemas/triggers/assemble.rs`
        - DMLS: `darkmatter/dmls/src/providers/frontmatter.rs`, `darkmatter/dmls/src/overlay/schema.rs`
        - CLI: `darkmatter/cli/src/commands/schema/assignment.rs`
        - tests: `darkmatter/dmls/tests/l1/claudine_schema_discovery.rs`, `darkmatter/dmls/tests/l1/strict_mode_recovery_spike.rs` (plus inline tests in `mod.rs` and `assignment.rs`)
        - docs and skills: `claudine/docs/topics/schemas.md`, `darkmatter/docs/topics/schemas/dmls-schema-support.md`, `darkmatter/docs/topics/schemas/definition.md`, `darkmatter/dmls/docs/editors/README.md`, `darkmatter/dmls/README.md`, `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/dmls.md`
- **final verification (macOS, run by the orchestrator after the last change):**
        - `cd darkmatter && just test`: 9997 passed, 10 skipped, exit 0 (R2-F1.1–F1.14)
        - `cd darkmatter && just lint`: exit 0, including the zed-dmls wasm32-wasip2 check
        - `cd claudine && just test`: 9063 passed, 9 skipped, exit 0 (launch validation's use of `EffectiveSchema` unchanged)
        - `cd claudine && just lint`: exit 0
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `git diff --check`: clean
- **cross-OS:** the change is platform-neutral schema/shape logic with no path, process, or `cfg` code; pull-request CI proves Linux and macOS, so no cross-check rig run was made
- **performance:** no measurement required; `deferred_perf_measurement` not set
