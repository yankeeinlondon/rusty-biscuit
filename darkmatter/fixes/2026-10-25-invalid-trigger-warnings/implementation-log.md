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
    Phase 1 landed the library recovery core. Facts the next phases need:
    (1) Findings: `TriggerRegistry::findings()`, `DarkmatterSchemas::discovery_findings()`, `ValidationReport::trigger_findings`, `CleanSchemaContext::discovery_findings()`, `SchemaCleanAnalysis::trigger_findings()`. Each `InvalidTriggerFinding` has `source()`, `identity()` (normalized path or bundle/name; dedup by this), `cause()`, `cause_path()`, `position()`, `code()`, `message()`.
    (2) `md compose` already shows the warning on stderr through `warning_rows` and in report-json through `warning_diagnostic` (unlocated), because `prepare_document` adds `ComposeWarning::from_invalid_trigger` before any effect. `md schema validate`, `md schema triggers`, and `md clean` show nothing yet (Phase 2).
    (3) Phase 2: two CLI tests were weakened because their surfaces print nothing yet; restore warning assertions on them. `cli/tests/l1/schema_triggers.rs::schema_validate_skips_a_trigger_with_a_forbidden_path_prefix` should also assert stderr names `bad.trigger.yaml` and the pattern `@prompts/**`. `an_unparseable_claimed_schema_root_file_is_skipped_with_its_yaml_location` previously also covered `md schema triggers`; add that command back once the trace lists skipped triggers. The old YAML hint text ("must be quoted") is not in the finding message.
    (4) Phase 3: `dmls/src/overlay/registry.rs::fail_closed_on_findings` is a TEMPORARY bridge that turns a scan's first finding back into `SchemaError::TriggerLoad`, so DMLS still fails the scope closed with last-good. Delete it when DMLS installs valid subsets; the skill note in `.claude/skills/darkmatter/dmls.md` goes with it.
    (5) Phase 4: `effective_schema_for` (compose) still returns only `Option<EffectiveSchema>`. Claudine's two schema matrices classify "discovery reported a finding" as their `Error` outcome (`claudine_schemas_matrix.rs::discover_from`).
    (6) Two DMLS tests fail under full-package load on the unchanged base as well: `registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running` and `startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration`. They cross the 500 ms slow-request log threshold. Do not chase them as regressions.
    (7) User docs under `darkmatter/docs/` are untouched (Phase 5).
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
