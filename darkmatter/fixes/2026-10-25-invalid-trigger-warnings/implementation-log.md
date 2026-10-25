---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-25-invalid-trigger-warnings/spec.md"
plan: "darkmatter/fixes/2026-10-25-invalid-trigger-warnings/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/policy.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/trigger_input_matrix.rs
    - darkmatter/lib/tests/l1/trigger_payload_load_errors.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
    - darkmatter/cli/tests/l1/schema_trigger_rename.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/l1/schema_validate_timings.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
human_review: false
message_to_agent: |-
    Phase 3 landed DMLS. Facts the next phases need:
    (1) The temporary bridge `fail_closed_on_findings` is gone. A scan with findings is `Outcome::Ready` and installs its valid subset (even empty); `failed_scopes` never counts a skipped trigger, `failed_triggers` does, so the startup line stays `schema registry ready`. `TriggerRegistries::reconcile_warnings` publishes one warning per skipped disk trigger across all scopes (`TriggerDiagnosticTransition { path, finding }`), clears it when no scope reports it, and logs WARN `ignoring invalid schema trigger …` (fields `trigger=`, `code=`) / INFO `invalid schema trigger no longer reported`. Embedded findings are logged only.
    (2) Diagnostic: severity Warning, code `dm.schema.invalid_trigger`, source `darkmatter.schema`, message = `InvalidTriggerFinding::message()`, `relatedInformation` = `cause_path()` when present (an unresolved import inside a payload carries no path in its error, so it has none). Open trigger files: buffer envelope parses → the warning (`SuggestionState::TriggerWarning`); malformed buffer → the strict `dm.schema.prepare` ERROR wins. `didClose` republishes the warning.
    (3) Phase 5 docs to update in `darkmatter/docs/lsp/indexing.md` (log table still lists the old trigger-failure rows and says `initialized with failures` covers failed triggers) and any DMLS schema-support page describing last-good retention for trigger load errors: last-good now applies only to whole-scan failures (case-fold collision); root I/O fails closed; skipped triggers install the valid subset and warn on the trigger file.
    (4) DMLS only reports triggered schemas' document diagnostics under `[schema] strict = true`; session tests of trigger properties must set it.
    (5) The load-sensitive DMLS test from Phases 1-2 (`registry_preparation::a_held_preparation_…`) did not fail this phase.
source_files_during_phase_2:
    - darkmatter/cli/src/commands/trigger_findings.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/clean/json.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - darkmatter/lib/src/markdown/cleanup/frontmatter.rs
    - darkmatter/lib/src/markdown/cleanup/document.rs
    - darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/l1/clean_json.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/clean_frontmatter_repair.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/router/logging_tests.rs
    - darkmatter/dmls/tests/l1/invalid_trigger_warnings.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/formatting_transaction.rs
    - darkmatter/dmls/tests/l1/workspace_schema_scopes.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/startup_log_lines.rs
    - darkmatter/dmls/tests/l1/searched_candidates.rs
    - darkmatter/dmls/tests/l1/registry_observability.rs
    - darkmatter/dmls/tests/l1/strict_mode_recovery_spike.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
---

# Implementation Log for 2026-10-25-invalid-trigger-warnings (6 phases)

## Phase 1

### Progress

- started Phase 1 (darkmatter library recovery core)

### Design Decisions

1. **Decision: findings carry an `Arc<SchemaError>` cause, compared by text**
    - **decision:** `InvalidTriggerFinding` (`lib/src/markdown/schemas/triggers/finding.rs`) stores the cause as `Arc<SchemaError>` and implements `PartialEq` by source, identity, locations, and the cause's text. `identity()` is computed once at construction (`canonicalize_simplified` for a file, `(bundle, name)` for embedded).
    - **alternatives considered:** make `SchemaError` `Clone` (it holds `std::io::Error`); store only the cause's text (loses the typed cause the spec requires); compute identity on every call (repeats filesystem work in passive consumers).
    - **why:** `TriggerRegistry` derives `Clone`/`PartialEq` and DMLS clones registries; the spec requires the typed cause.
    - **downsides:** two findings whose causes differ only in non-displayed fields compare equal.
2. **Decision: failure observations are collected out-of-band**
    - **decision:** `resolve::observed::collect` keeps a thread-local stack of collectors; the two probe sites (`resolve_file_reference_in_context`, `try_bare_name_in_roots`) and the successful reads (`load_schema_from_path_in_context`, `load_named_types`, `resolve_one_example`) record into it. Discovery attaches the observations to the finding, and `TriggerRegistry::dependencies()` / `searched_locations()` report them (the skipped envelope is also a dependency).
    - **alternatives considered:** thread observations through every error variant in `resolve.rs` (a 6000-line module with dozens of error paths); return partial results from every resolver.
    - **why:** the plan allowed "an observation bag out-of-band"; it touches five call sites and nothing in the error taxonomy. Resolution is single-threaded per call, so a thread-local is sound.
    - **downsides:** a resolver that later moves work onto another thread would stop recording; the module doc says so.
3. **Decision: the validator gate compiles the structural validator**
    - **decision:** `enforce_compiles` builds `SHARED_VALIDATOR_CACHE`'s structural (context-free) validator for each payload, keyed by the shared-`Arc` digest, and maps a failure to the new `SchemaError::TriggerPayloadCompile { path, payload, payload_path, message }`.
    - **alternatives considered:** compile with the document's context (one compile per context for `file`-judging payloads); reuse `TriggerPayloadNotMergeable`.
    - **why:** compile errors do not depend on the judging context; the structural slot is cached once per payload content, satisfying "do not compile every accepted payload again"; the payload memo also returns the same `Arc`, so a memo hit never re-runs the gate.
    - **downsides:** a payload is compiled once structurally even if every document later compiles it in a context.
4. **Decision: findings are added to the compose report before any effect**
    - **decision:** `prepare_document` (`compose/pipeline/mod.rs`) adds `ComposeWarning::from_invalid_trigger` for every finding right after `prepare_schemas`, then calls `stop_if_blocked`. A disk finding's subject is `WarningSubject::Path(identity)`; an embedded one uses a new `WarningSubject::Embedded { bundle, name }`, never attributed to a document, so the warning collapses across passes and transclusion children. `line_number` stays `None`; the message carries the YAML position.
    - **alternatives considered:** add them in the validation stage (after frontmatter effects could run); attach the trigger's YAML line to `line_number` (consumers pair it with the composed document).
    - **why:** spec: deny stops "before its next effect", including preflight and children; warnings precede `passive_warnings`, so preflight truncation keeps them and `compose_with_warnings` returns them on a later failure.
    - **downsides:** report-json rows are unlocated in Phase 1; Phase 2 may add the trigger location.
5. **Decision: an authored `$schema` naming a malformed claimed envelope is a trigger reference**
    - **decision:** `file_claims_trigger_envelope` (`assemble.rs`) now treats `parse_trigger_envelope_from_str` returning `Err` as a claimed trigger, so `$schema: ./schemas/memory.trigger.yaml` fails with `TriggerSchemaReferenced` even though discovery skipped that file. Payload-cycle detection uses the same rule.
    - **alternatives considered:** leave it (the file was silently read as a raw JSON Schema constraining nothing — found by the new test `document_schema_referencing_an_ignored_trigger_file_still_fails`).
    - **why:** spec acceptance 9: "An authored `$schema` referencing the same ignored file still fails."
    - **downsides:** none found; unclaimed YAML is still `Ok(None)`.
6. **Decision: a temporary DMLS bridge keeps the scope failing closed until Phase 3**
    - **decision:** `dmls/src/overlay/registry.rs::fail_closed_on_findings` turns a scan's first finding back into `SchemaError::TriggerLoad { path, source }`, so DMLS's last-good registry, failure logging, and trigger-file diagnostics behave exactly as before.
    - **alternatives considered:** implement Phase 3's valid-subset install now (Phase 3 scope); rewrite ~45 DMLS tests to an interim contract Phase 3 would rewrite again.
    - **why:** keeps `just test` green with a four-line adapter Phase 3 deletes, without changing editor behavior twice.
    - **downsides:** between Phase 1 and Phase 3, DMLS still fails a scope on one invalid trigger (the old behavior).
7. **Decision: CLI and Claudine tests that asserted the old hard failure assert the recovered contract**
    - **decision:** `md compose` tests now assert success with the warning on stderr (compose's existing `warning_rows` already renders it). `md schema validate` tests that relied on a hard failure assert success (the forbidden-prefix case) or use a real infrastructure failure, a non-UTF-8 root file (the partial-timings case). Claudine's two matrices classify "discovery reported a finding" as their `Error` outcome; three Claudine verdict tests assert the skipped disk trigger still shadows the embedded copy.
    - **alternatives considered:** leave validate/clean/triggers/Claudine delivery in place now (Phases 2 and 4 own it).
    - **why:** each test keeps its original intent under the new contract; delivery surfaces stay with their phases.
    - **downsides:** `md schema validate`, `md schema triggers`, and `md clean` do not yet show the warning (Phase 2); Claudine does not yet deliver it (Phase 4).

### Work Completed

- **Wave 1:** `InvalidTriggerFinding` (`triggers/finding.rs`), `SCHEMA_INVALID_TRIGGER_CODE` (`schemas/mod.rs`), registered in `COMPOSE_DIAGNOSTIC_CODES` (`compose/policy.rs`); catalog uniqueness test and the CLI `--deny` cross-check stay green.
- **Wave 2:** `scan_with_source` classifies and resolves each unshadowed file in one loop, recording a finding per invalid envelope or payload; `append_embedded_bundle` records embedded findings and claims their names; `load_embedded_trigger` returns the unwrapped cause (memo still keeps successes only). Module and function docs rewritten from "transactional" to per-trigger recovery with the hard-error taxonomy.
- **Wave 3:** `enforce_compiles` gate for disk and embedded payloads; `resolve::observed` failure observations wired into `dependencies()`/`searched_locations()`.
- **Wave 4:** `TriggerRegistry::findings`/`findings()`, `DarkmatterSchemas::discovery_findings()`, `ValidationReport::trigger_findings` (filled by `validate`/`validate_for_phase` on both branches), `PreparedSchemas::discovery_findings()`, `CleanSchemaContext::discovery_findings()`, `SchemaCleanAnalysis::trigger_findings()`.
- **Wave 5:** compose projection (`ComposeWarning::from_invalid_trigger`, `WarningSubject::Embedded`), added before any effect with a `stop_if_blocked` check.
- Skills: `.claude/skills/darkmatter/schema.md` (recovery contract), `.claude/skills/darkmatter/dmls.md` (the temporary bridge).
- User docs (`darkmatter/docs/…`) are left to Phase 5 as planned.

### Requirement-to-Test Mapping

| Requirement | Test(s) |
| --- | --- |
| Incident `memory.yaml` skipped, warning names source + reason, no schema, vacuous validation carries findings | `trigger_discovery_recovery::an_envelope_with_unknown_keys_is_a_finding_and_validation_stays_vacuous`; compose: `compose::tests::invalid_triggers::a_prompt_without_schema_composes_beside_the_incident_trigger` |
| Valid trigger beside invalid, both filename orders; validation still enforced | `trigger_discovery_recovery::a_valid_trigger_{after,before}_an_invalid_one_still_applies`; `invalid_triggers::a_valid_trigger_beside_the_invalid_one_still_enforces_its_schema`; `discovery::tests::malformed_trigger_beside_a_valid_one_installs_the_valid_one`, `payload_resolution_failure_skips_only_its_trigger` |
| Every envelope classification error (shape matrix) | `trigger_input_matrix::*` (111 cells), `schema_roots::path_field_input_matrix`, `schema_embedded_trigger_bundles::the_trigger_envelope_matrix_holds_for_{block_yaml,json_syntax}` |
| Malformed claimed YAML (positioned) vs unclaimed (silent) | `trigger_discovery_recovery::malformed_claimed_yaml_is_a_positioned_finding`, `malformed_unclaimed_yaml_is_silently_ignored`; `schema_roots::an_unparseable_claimed_schema_root_file_is_reported_with_its_yaml_location` |
| Missing payload, broken import, cycle, non-mergeable, non-compiling raw JSON (even when a later layer redeclares it, even unmatched) | `trigger_discovery_recovery::{a_missing_payload_is_searched_not_a_dependency_and_creating_it_installs_the_trigger, a_broken_import_depends_on_the_files_read_and_repairing_it_installs_the_trigger, a_payload_cycle_is_a_finding_and_a_valid_neighbor_installs, a_root_union_payload_is_not_mergeable_and_a_valid_neighbor_installs, a_raw_payload_that_fails_to_compile_is_skipped_even_when_a_later_layer_redeclares_its_property}`; `trigger_payload_load_errors::*` (matched and unmatched documents) |
| Embedded findings by bundle/document; name stays claimed; bundle structure errors stay errors | `trigger_discovery_recovery::{an_invalid_embedded_trigger_is_a_finding_that_keeps_its_name_claimed, an_invalid_embedded_bundle_structure_still_fails_discovery}`; `schema_embedded_trigger_bundles::*` |
| Shadowing unchanged after a skip; shadowed files unread and warning-free; deleting restores normal shadowing | `trigger_discovery_recovery::a_nearer_invalid_trigger_shadows_a_same_named_valid_one_until_deleted`; `schema_embedded_trigger_bundles::a_same_name_malformed_disk_trigger_is_skipped_with_no_embedded_fallback`; Claudine `claudine_schemas_embedded::malformed_disk_trigger_is_skipped_without_embedded_fallback` |
| Distinct identities: shared broken payload → two findings | `trigger_discovery_recovery::two_triggers_sharing_a_broken_payload_are_two_findings` |
| Discovery order (disk before embedded) | `trigger_discovery_recovery::findings_are_in_filename_order_with_disk_before_embedded`; `trigger_payload_load_errors::a_payload_cycle_skips_each_trigger_for_matched_and_unmatched_documents` |
| Infrastructure errors stay errors | `trigger_discovery_recovery::an_unreadable_schema_root_still_fails_discovery`; `discovery::tests::an_unreadable_schema_root_fails_the_scan`; CLI `schema_validate_timings::a_discovery_failure_keeps_its_error_and_reports_partial_timings` (non-UTF-8 root file) |
| Repair / creation refreshes (incl. missing parent folder, first load all-invalid) | `trigger_discovery_recovery::creating_a_missing_payload_and_its_parent_folder_installs_the_trigger` and the two repair tests above; `discovery::tests::malformed_trigger_is_skipped_with_a_finding` (envelope is a dependency) |
| Failed payloads are uncached; accepted payloads not recompiled | `payload_memo_tests::a_failed_resolution_is_a_miss_and_is_not_kept`; `trigger_discovery_recovery::an_accepted_payload_is_resolved_once_across_scans` |
| Passive parsers stay strict | `schema_roots_parity` (direct `parse_trigger_envelope_from_str`), `trigger_envelope_claim::*` unchanged and green |
| Authored `$schema` naming the ignored trigger still fails | `phase4_trigger_assembly::document_schema_referencing_an_ignored_trigger_file_still_fails` |
| Warnings retained before an authored-schema failure | `invalid_triggers::the_warning_is_kept_when_the_authored_schema_fails_to_load` |
| Dedup across transclusion | `invalid_triggers::a_transcluded_child_reports_the_shared_trigger_once` |
| `--deny dm.schema.invalid_trigger` / `--deny all` stop before the first effect, warning severity and trigger source kept | `invalid_triggers::denying_the_code_stops_before_the_first_effect`, `denying_all_stops_before_the_first_effect` |
| Disabled discovery emits nothing | `invalid_triggers::disabled_trigger_discovery_reports_nothing` |
| Normal CLI path (F3): compose succeeds, stderr warns | `cli::schema_trigger_rename::compose_warns_about_an_invalid_trigger_field_by_location_and_composes_without_it`, `cli::schema_triggers::an_unparseable_claimed_schema_root_file_is_skipped_with_its_yaml_location` |

Boundary and fidelity: discovery and compose are covered in-process at the crate's public API (F2, L1, temp repositories with controlled roots and no home); the `md compose` path is covered by spawned-CLI L1 tests (F3). No test needs a terminal or browser window.

### Gates

- `just test` (darkmatter): 12,240 of 12,241 passed on the last full run; the only failure is `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running` (load-sensitive, below). Fail-fast stopped the remaining 197 tests, so DMLS was also run whole with `--no-fail-fast`: 1,429 of 1,431 pass; the two failures are the load-sensitive tests below.
- `just lint` (darkmatter): pass. `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents: `claudine` targeted (`schema|trigger|composition|prepare`, 3,169 tests) pass; `claudine-cli` targeted (`schema|trigger|composition|compose`, 1,012 tests) pass; `cargo clippy -p claudine --tests -D warnings` clean; `messenger` `research_corpus` (its only schema test) passes. No other dependent uses a changed symbol.

### Load-Sensitive and Pre-Existing Failures

- `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running` and `dmls::l1 startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration` fail only under full-package load: the held request crosses DMLS's 500 ms slow-request threshold, so it logs `WARN slow request` instead of the `request complete` line the test filters for. Each passes alone (3/3). **Pre-existing:** the same `cargo nextest run -p dmls --features effects-instrumentation --no-fail-fast` on the unchanged base (Phase 1 changes set aside in a tagged stash and restored by SHA) fails the same two tests; disabling the new validator gate also leaves them failing.
- `claudine composition::lifecycle::prewarm::worker::tests::a_successful_or_no_op_preparation_counts_as_done` failed once during a loaded run and passed on the rerun.

### Cross-OS Check

- `just cross-check darkmatter --os windows`, attempt 1: 9,612 of 9,613 passed. `trigger_discovery_recovery::an_envelope_with_unknown_keys_is_a_finding_and_validation_stays_vacuous` failed because the test expected the message to contain `root.join("schemas/memory.yaml").display()`, which keeps the authored `/` on Windows. `Path` equality is component-wise, so the source assertion passed. The test now checks that the message names `finding.source()`.
- Attempt 2: 9,613 of 9,613 passed. `os_failing` is not set.

## Phase 2

### Progress

- started Phase 2 (`md` CLI warning delivery): compose, schema validate, schema triggers, clean
- all four Wave 6 tasks implemented and checked off in the plan; the work was done inline (no subagents), since the four tasks share one new delivery module

### Design Decisions

1. **Decision: one shared CLI delivery module for the non-compose commands**
    - **decision:** `cli/src/commands/trigger_findings.rs` holds `emit_warnings` (stderr `Status` warning rows, the same presentation `md compose` already gives the warning), `FindingRow` (`{source, code, path, message}`; `path` is the trigger file, `null` for an embedded trigger), and `DeliveredFindings` (per-invocation dedup by `InvalidTriggerFinding::identity()`).
    - **alternatives considered:** a renderer per command; route validate/clean through compose's `warning_rows`/`ComposeWarning`.
    - **why:** the plan asks for one row shape for validate and clean and one dedup rule; compose's types are composition-specific.
    - **downsides:** compose keeps its own `warning_rows`; both render through `Status` with warning state, so the two must stay visually aligned by hand.
2. **Decision: `md schema validate` reports a finding on any outcome of the file that first discovers it**
    - **decision:** `validate_one` returns a `CheckedFile { outcome, timings, findings }`; findings are kept even when the later effective-schema or validation step fails. Pretty mode prints them on stderr before the file's outcome (also under `--quiet`, which hides successes, not warnings). JSON mode puts the rows first in the record's `warnings` (discovery runs before validation), followed by schema advisories, on `Validated` and `SchemaError` records alike. Exit statuses are untouched.
    - **alternatives considered:** only attach to `Validated` records (loses the warning on exit 2); suppress under `--quiet`.
    - **why:** spec: "extend failure outcomes to retain earlier discovery warnings without changing existing outcome fields or exit statuses", and the warning "must be visible during ordinary CLI use".
    - **downsides:** a `SchemaError` record can now carry `warnings`, a key consumers previously saw only on validated records.
3. **Decision: the trigger trace gains a "Skipped invalid triggers" section between "Shadowed envelopes" and "Triggers"**
    - **decision:** always printed (with `none` when empty, as the neighbouring sections do), each row the finding's message plus its code; nothing on stderr.
    - **alternatives considered:** append at the end; print only when non-empty.
    - **why:** it describes envelope status, like shadowing, and the existing sections' content is unchanged.
    - **downsides:** none found; existing trace tests match by substring.
4. **Decision: `md clean` gets findings from the library result, and a schema failure carries them on its error**
    - **decision:** `FrontmatterRepair` and `CleanResult` gain `trigger_findings` (any outcome); `FrontmatterRepairError::Schema` becomes `Schema { error: Box<SchemaError>, trigger_findings }`, filled from `CleanSchemaContext::discovery_findings()` when `analyze` fails after discovery. The CLI writes them to stderr in ordinary mode and into a new always-present top-level `warnings` array in `--json` (version stays 1). On a schema failure there is no envelope, so the CLI writes them to stderr ahead of the error in both modes. `SchemaError` is boxed to satisfy `clippy::result_large_err`.
    - **alternatives considered:** let the CLI rebuild a `CleanSchemaContext` to read findings (a second discovery); drop findings on the failure path.
    - **why:** spec: retain warnings collected before a later authored-schema failure and deliver them with it; `md clean --json` "adds a top-level `warnings` array, separate from `diagnostics`".
    - **downsides:** `FrontmatterRepairError::Schema` changed shape (tuple → struct); its only matchers were the CLI and one lib test.
5. **Decision: ordinary `md compose` prints a failed run's trigger warnings before the error**
    - **decision:** found while writing the tests: compose's ordinary mode dropped `ComposeFailure::warnings`, so a document that failed validation under a valid trigger, or whose authored `$schema` was missing, never showed the invalid-trigger warning. `report_failure_trigger_warnings` (in `commands/compose.rs`, used by the plain and lifecycle paths, for both the preflight and compose failures) now prints the `dm.schema.invalid_trigger` warnings a failure carries, skipping any the `BlockedDiagnostics` error already shows as blocking.
    - **alternatives considered:** print every warning family on failure (changes output for warnings the spec does not cover); leave compose alone (violates "retain ... and deliver them with that failure").
    - **why:** spec requirement for this warning; Rule 3 keeps other families unchanged.
    - **downsides:** other compose warning families still disappear on failure in ordinary mode, as before.
6. **Decision: embedded `path: null` is covered by construction, not a CLI test**
    - **decision:** `FindingRow::path` is `finding.source().path()`, which is `None` for `TriggerSource::Embedded`; `md` has no embedded bundle to discover, so no `md` invocation can produce an embedded finding.
    - **alternatives considered:** a unit test constructing an embedded finding (the constructor is `pub(crate)` in the library).
    - **why:** the criterion cannot be exercised through `md`; Phase 4's Claudine delivery covers embedded sources.
    - **downsides:** the null branch is proven by the type, not observed.

### Work Completed

- `cli/src/commands/trigger_findings.rs` (new): shared delivery (stderr rows, JSON row, per-invocation dedup).
- `md schema validate` (`cli/src/commands/schema/validate.rs`): findings ride `CheckedFile`, dedup across files, stderr in pretty mode, `warnings` rows in JSON mode on any outcome.
- `md schema triggers` (`cli/src/commands/schema/triggers.rs`): "Skipped invalid triggers" trace section.
- `md clean` (`cli/src/commands/clean.rs`, `clean/json.rs`; lib `cleanup/frontmatter.rs`, `cleanup/document.rs`): `trigger_findings` on the results and the schema error; stderr warnings; `--json` top-level `warnings`.
- `md compose` (`cli/src/commands/compose.rs`, `compose/lifecycle.rs`): trigger warnings printed on failure in ordinary mode.
- Restored the two tests Phase 1 weakened: `schema_triggers::schema_validate_skips_a_trigger_with_a_forbidden_path_prefix` asserts stderr names `bad.trigger.yaml` and `@prompts/**`; `an_unparseable_claimed_schema_root_file_is_skipped_with_its_yaml_location` again covers `md schema triggers`.
- `clean_json::test_json_envelope_has_exactly_the_documented_top_level_fields` pins the new `warnings` key.
- Skill: `.claude/skills/darkmatter/schema.md` documents the CLI delivery. User docs (`darkmatter/docs/…`, including the clean `--json` and validate JSON references) remain Phase 5's.

### Requirement-to-Test Mapping

All CLI tests are spawned-`md` L1 tests (F3) in isolated temp repositories (`CliProcessFixture`, hand-written `.git`), no terminal or browser window. Library tests are in-process at the public API (F2, L1).

| Requirement | Test(s) |
| --- | --- |
| Compose of a `$schema`-less prompt beside the incident trigger succeeds; stdout carries the document; exactly one stderr warning naming the trigger and reason (incl. a transcluded child) | `invalid_trigger_warnings::compose_beside_the_incident_trigger_succeeds_with_one_warning` |
| `--output report-json` carries `dm.schema.invalid_trigger` (severity warning), no stderr row | `invalid_trigger_warnings::compose_report_json_carries_the_warning_as_a_diagnostic_and_not_on_stderr` |
| Warnings retained on an independent validation / authored-schema failure | `compose_keeps_the_warning_when_a_valid_trigger_rejects_the_document`, `compose_keeps_the_warning_when_the_authored_schema_fails_to_load` |
| `--deny dm.schema.invalid_trigger` / `--deny all` stop before the document is written, warning severity kept, no schema-load error | `compose_deny_of_the_code_or_all_stops_before_writing_the_document` (ordinary and report-json) |
| `--disable schema-triggers` emits nothing | `compose_with_schema_triggers_disabled_reports_nothing`, `schema_validate_with_schema_triggers_disabled_reports_nothing`, `clean_with_schema_triggers_disabled_reports_nothing`, lib `trigger_discovery_recovery::clean_with_trigger_schemas_disabled_has_no_findings` |
| Authored `$schema` naming the ignored trigger file still fails | `compose_of_a_document_referencing_the_ignored_trigger_still_fails` |
| Validate pretty: stderr warning once per invocation for several files, exit 0 | `schema_validate_warns_once_on_stderr_for_several_files` |
| Validate JSON: row shape, first-file attachment, none on the second | `schema_validate_json_attaches_a_shared_warning_to_the_first_file` |
| Validate JSON failure outcomes retain the warning; exit 1 and exit 2 unchanged | `schema_validate_json_failure_keeps_the_discovery_warning`, `schema_validate_json_schema_error_keeps_the_discovery_warning` |
| Assignments against the recovered registry, invalid trigger before/after the valid one | `schema_validate_assignments_use_the_valid_trigger_beside_an_invalid_one` (both filename orders, with and without the required assignment) |
| Forbidden path prefix names trigger and pattern | `schema_triggers::schema_validate_skips_a_trigger_with_a_forbidden_path_prefix` |
| Trigger trace lists skipped triggers inline, valid neighbor still matched; `none` when clean; YAML-located claimed file | `schema_triggers_lists_the_skipped_trigger_and_still_matches_the_valid_one`, `schema_triggers_reports_none_skipped_for_a_valid_root`, `schema_triggers::an_unparseable_claimed_schema_root_file_is_skipped_with_its_yaml_location` |
| Clean ordinary: succeeds, stdout unchanged document, one stderr warning | `clean_beside_the_incident_trigger_succeeds_with_one_stderr_warning` |
| Clean `--json`: version 1, `diagnostics`/`applied`/`changed` unchanged, top-level `warnings` row, clean stderr | `clean_json_carries_the_warning_in_the_top_level_warnings_array`, `clean_json::test_json_envelope_has_exactly_the_documented_top_level_fields` |
| Clean schema failure still delivers the warning (both modes) | `clean_keeps_the_warning_when_the_authored_schema_fails_to_load`; lib `a_clean_schema_failure_carries_the_findings_discovered_before_it` |
| `CleanResult::trigger_findings` is outside `diagnostics` and a valid neighbor still constrains | lib `clean_carries_the_skipped_trigger_outside_its_diagnostics` |

The Input Robustness Matrix does not apply: Phase 2 adds no reader of a file format (it delivers findings the Phase 1 loaders already classify; the envelope matrix is Phase 1's `trigger_input_matrix`).

### Gates

- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages: [darkmatter, darkmatter-cli, test-toolkit]`, `change_class: package`, so both gates ran.
- `just lint` (darkmatter): pass. Fixes it forced: `SchemaError` boxed inside `FrontmatterRepairError::Schema` (`clippy::result_large_err`), `expect_err` in the new lib test.
- `just test` (darkmatter): 12,264 of 12,265 passed; the one failure was `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running` (the load-sensitive test from Phase 1). It passes alone. Fail-fast skipped 196 tests, so `cargo nextest run -p dmls --features effects-instrumentation --no-fail-fast` ran next: 1,431 of 1,431 passed.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents: no package outside `darkmatter/` uses `CleanResult`, `FrontmatterRepair`, or `FrontmatterRepairError` (the ripgrep matches are unrelated names). `cargo check -p claudine -p claudine-cli -p messenger --tests` is clean.

### Load-Sensitive and Pre-Existing Failures

- `dmls::l1 registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`: load-sensitive, as Phase 1 recorded (fails only under full-package load, passes alone and in the whole DMLS package run).

### Cross-OS Check

- `just cross-check darkmatter --os windows`, attempt 1: 9,616 of 9,616 passed (65 skipped). `os_failing` is not set.

## Phase 3

### Progress

- started Phase 3 (DMLS valid-subset install, warning diagnostics, refresh)
- removed the Phase 1 bridge `fail_closed_on_findings`; installed the valid subset; added per-source warning reconciliation, publication, and authoring-surface handling
- converted 28 L1 tests and 13 unit tests (9 registry, 3 overlay, 1 router logging) that encoded the last-good/error contract (two parallel subagents took the L1 files: `workspace_schema_scopes.rs`; and `lsp_session.rs`, `startup_log_lines.rs`, `searched_candidates.rs`, `registry_observability.rs`, `strict_mode_recovery_spike.rs`)
- added `dmls/tests/l1/invalid_trigger_warnings.rs`, a formatting passthrough test, and five registry unit tests
- gates green; Windows cross-check green on attempt 2

### Design Decisions

1. **Decision: a scope's findings are its served registry's findings**
    - **decision:** `Scope::findings()` returns `installed.registry.findings()` unless the scope is `RootFailed`; there is no separate per-scope findings field.
    - **alternatives considered:** a dedicated `findings` field set only by the latest scan; dropping findings while a whole-scan failure (`TriggerFailed`) serves last-good.
    - **why:** what DMLS reports must match what it serves: a `TriggerFailed` scope serves its last-good registry, so its warnings are that registry's; a `RootFailed` scope serves nothing, so it reports none. One source of truth, no extra state.
    - **downsides:** while a case-fold collision holds, a warning reflects the last completed scan, not the current disk.
2. **Decision: one published warning per trigger identity, reconciled after every install**
    - **decision:** `TriggerRegistries::reconcile_warnings` folds every scope's findings (stale scopes included) in sorted scope-path order, keyed by `InvalidTriggerFinding::identity()`, and diffs the result with the published map. A new or changed finding publishes and logs at `warn`; a vanished one clears and logs at `info`; an unchanged one does nothing.
    - **alternatives considered:** keep the per-failure-scope `FailureRecord` transition model (one record per scope, which republishes a source shared by scopes and lets a healthy scope clear another's warning); exclude stale scopes (a warning would flicker off at invalidation and republish after rescan).
    - **why:** spec: "publish each source warning once; one healthy scope must not clear a warning still reported by another"; "repeated unchanged refreshes must not repeat warning logs or diagnostic publications". Identity is computed when the finding is built, so reconciliation does no I/O under the lock.
    - **downsides:** when two contexts report different causes for one file, the message shown is the first scope path's; a stale scope keeps a repaired file's warning until it re-prepares (workspace passes re-prepare stale scopes).
3. **Decision: the old trigger-load error diagnostic path is removed, not kept beside the warning**
    - **decision:** `FailureRecord::owns_diagnostic`, `load_error_for`, and `trigger_load_diagnostic` are deleted; `TriggerDiagnosticTransition` now carries `finding: Option<InvalidTriggerFinding>`. Whole-scan failures still log through `record_failure` but publish no trigger-file diagnostic.
    - **alternatives considered:** keep the error path for whole-scan failures.
    - **why:** since Phase 1, discovery never returns `SchemaError::TriggerLoad`, so that path was reachable only through the bridge. Keeping dead code is debt.
    - **downsides:** a case-fold collision gets no file diagnostic (as before: it never owned one).
4. **Decision: on an open trigger file, the strict authoring error takes precedence over the warning**
    - **decision:** a buffer whose envelope does not parse shows only `SuggestionState::TriggerError` (ERROR `dm.schema.prepare`). A buffer whose envelope parses, but that discovery skipped on disk, shows `SuggestionState::TriggerWarning` (the registry's warning, ranged at the YAML error span or the whole file).
    - **alternatives considered:** show both (two diagnostics with near-identical text on the same file); show only the warning (loses the spec's strict authoring check).
    - **why:** spec: "Direct authoring checks of an open trigger remain strict … Those errors do not turn its recoverable discovery warning into a document preparation failure." The error already describes the defect the warning would repeat.
    - **downsides:** while a malformed buffer is open, the disk-state warning is hidden; it comes back on close.
5. **Decision: `didClose` republishes a closed trigger's warning**
    - **decision:** `didClose` publishes `closed_file_diagnostics(uri)` (the warning, or empty) instead of always clearing.
    - **alternatives considered:** clear on close as before (the warning would vanish until the next transition, even though the trigger is still skipped).
    - **why:** spec: publish findings on disk trigger files, including ones no document has open.
    - **downsides:** none found.
6. **Decision: related information only where the cause names a file**
    - **decision:** `relatedInformation` is `InvalidTriggerFinding::cause_path()` when present. An unresolved import inside a payload (`NoMatch`/`Unresolved` causes) names no file, so its warning has none.
    - **alternatives considered:** derive the payload path from the finding's read observations.
    - **why:** spec: "Preserve **available** cause locations as related information." Guessing a location from observations could point at the wrong file.
    - **downsides:** for that cause, the message names the import but not which payload holds it.
7. **Decision: the startup registry line keeps its rule**
    - **decision:** `schema registry initialized with failures` still means `failed_scopes > 0`. A skipped trigger leaves the line `schema registry ready` with `failed_triggers` counting it.
    - **alternatives considered:** switch the line whenever `failed_triggers > 0`.
    - **why:** a skipped trigger is a warning, not a failed preparation; the count already surfaces it, and the per-trigger WARN log names it.
    - **downsides:** a reader scanning only the message misses skipped triggers; `docs/lsp/indexing.md` must say so (Phase 5).

### Work Completed

- `dmls/src/overlay/registry.rs`: bridge removed; `Outcome::TriggerFailed` is now a unit variant for whole-scan failures; `Scope::findings`, `reconcile_warnings`, and `finding_for` added; summary counts skipped triggers through findings; failure records lost their diagnostic role. Module and item docs rewritten for the new contract.
- `dmls/src/overlay/mod.rs`: `TriggerDiagnosticTransition { path, finding }`; `SuggestionState::TriggerWarning`; the open-trigger path reads `finding_for`; `OverlayState::trigger_finding`.
- `dmls/src/diagnostics/frontmatter.rs`: `invalid_trigger_diagnostic` (closed files) and `invalid_trigger_diagnostic_at` (open buffers), Warning + related information; `trigger_load_diagnostic` removed.
- `dmls/src/router.rs`: `publish_trigger_transitions` publishes findings; `closed_file_diagnostics` used on `didClose`.
- Skills: `.claude/skills/darkmatter/dmls.md` (the bridge paragraph replaced by the skipped-trigger contract, failure-logging scope, the strict-mode testing fact), `.claude/skills/darkmatter/schema.md` (where last-good still applies).
- Docs: none this phase; Phase 5 owns `docs/lsp/indexing.md` and the DMLS schema-support pages (see `message_to_agent`).

### Requirement-to-Test Mapping

Boundary: the DMLS registry cache (in-process unit, F1) and the LSP protocol over an in-memory connection (L1, F2: real server, real files in a temp workspace, no window or focus).

| Requirement (spec acceptance 8 / plan Phase 3) | Test |
| --- | --- |
| Valid-subset install; a formerly valid trigger saved invalid loses its properties while a valid neighbor changes | `overlay::registry::tests::a_trigger_saved_invalid_is_dropped_while_a_valid_neighbor_changes`; L1 `invalid_trigger_warnings::a_trigger_saved_invalid_loses_its_properties_while_a_valid_neighbor_changes` (diagnostics, completion, hover) |
| First-load all-invalid installs an empty ready registry with findings | `registry::tests::a_first_load_with_only_invalid_triggers_installs_an_empty_ready_registry`; L1 `invalid_trigger_warnings::a_first_load_with_only_invalid_triggers_warns_on_each_and_an_unchanged_refresh_publishes_nothing` |
| Multiple simultaneous warnings | the two first-load tests above (3 and 2 triggers) |
| One source shared by scopes is published once and cleared only when no scope reports it | `registry::tests::a_shared_source_is_published_once_and_cleared_only_when_no_scope_reports_it` |
| Prepared scopes with no open document | L1 `workspace_schema_scopes::*` (14 converted), `invalid_trigger_warnings::a_first_load_…` |
| Repair and deletion clear the warning | `registry::tests::deleting_a_trigger_removes_it_and_creating_a_missing_payload_recovers`, `external_home_and_environment_roots_recover`; L1 `lsp_session::closed_trigger_envelope_diagnostic_round_trip` (both watch modes), `repairing_trigger_payload_clears_open_envelope_immediately`, `invalid_trigger_warnings::a_trigger_saved_invalid_…` (deletion) |
| Unchanged refresh repeats neither logs nor publications | `registry::tests::repeated_identical_failures_warn_once_and_recovery_logs_info`; `router::logging_tests::a_mid_edit_unrelated_schema_file_logs_nothing_and_a_broken_trigger_warns_once`; L1 `invalid_trigger_warnings::a_first_load_…` (no buffered publish after an unchanged event) |
| Creation of a missing payload or its folder refreshes | `registry::tests::deleting_a_trigger_…` (absent `payloads/` folder); L1 `searched_candidates::a_candidate_created_during_an_in_flight_preparation_…`; `strict_mode_recovery_spike::…directory_recreation` |
| Watches survive a failed resolution | `registry::tests::a_malformed_nearer_candidate_skips_its_trigger_and_keeps_its_watch_until_repaired` |
| Related information = cause location | L1 `invalid_trigger_warnings::a_first_load_…` (unparseable payload), `searched_candidates::…` (nearer import) |
| Strict authoring diagnostics while editing; closed file keeps the warning | L1 `invalid_trigger_warnings::an_open_trigger_stays_strict_while_edited_and_keeps_its_warning_once_closed` |
| Skipped trigger never fails the document | L1 `invalid_trigger_warnings::a_trigger_saved_invalid_…` (no `dm.schema.prepare` on the doc), `overlay::tests::trigger_payload_edit_skips_the_trigger_without_failing_the_document` |
| Formatting passes through with no added message | L1 `formatting_transaction::an_invalid_schema_trigger_beside_the_document_neither_blocks_formatting_nor_adds_a_message` (incident-shaped `memory.yaml`) |
| Whole-scan failure keeps last-good and warns once; root failure fails closed | `registry::tests::a_whole_scan_failure_keeps_last_good_and_warns_once`, `a_cold_failure_logs_unavailable_once`, `a_root_io_error_fails_closed_and_recovers`; L1 `startup_log_lines::a_failed_scope_switches_the_registry_line_to_its_failure_variant` |
| Startup counts | L1 `startup_log_lines::*` (6), `registry_observability::*` |

The regression shape: every converted test asserted the old contract (last-good retention / `dm.schema.prepare` ERROR / `failed_scopes=1`), failed after the bridge was removed (28 L1, 13 unit), and passes against the new one. The new L1 tests' neighbor-change and closed-file assertions cannot pass under last-good retention.

Input robustness: no parser or field validation was added or changed (the plan's Decisions → Input robustness), so no matrix applies. Smell grep over the changed DMLS source: no new `#[serde(default)]`, `unwrap_or_default()`, or `.ok()` on a load-bearing parse; `finding_for` uses `canonicalize_simplified(..).unwrap_or_else` only to fall back to the raw path for lookup.

### Gates

- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages: [darkmatter, darkmatter-cli, dmls, test-toolkit]`, `change_class: package` (the tree still holds the uncommitted Phase 2 changes too), so both gates ran.
- `just lint` (darkmatter): pass, also re-run after the final test edit.
- `just test` (darkmatter): 12,471 of 12,471 passed (10 skipped).
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependents: no package depends on `dmls`, and the `darkmatter` library is unchanged this phase, so no dependent tests apply.

### Load-Sensitive and Pre-Existing Failures

- None this phase. `registry_preparation::a_held_preparation_…` passed in the full run.

### Cross-OS Check

- `just cross-check dmls --os windows`, attempt 1: 1,437 of 1,438. `router::logging_tests::a_mid_edit_unrelated_schema_file_logs_nothing_and_a_broken_trigger_warns_once` failed because my widened log filter (`"trigger"`) also matched Windows's `slow schema validation … matched_triggers=` WARN line. Fixed by filtering on `trigger registry` / `invalid schema trigger`.
- Attempt 2: 1,438 of 1,438 passed. `os_failing` is not set.
