---
fix: 2026-10-25-invalid-trigger-warnings
review: 2026-10-25-invalid-trigger-warnings/review-1.md
implementation_1: "2026-10-25T18:10:15-07:00"
implementation_2: "2026-10-25T19:37:38-07:00"
---

# Review Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-10-25T18:10:15-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-25-invalid-trigger-warnings/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs
- the review lists three high findings and no observations, blockers, or human-review items

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1    | Early composition exits discard discovered trigger warnings | fixed | every required child row verified; siblings R1-F1.10–F1.14 fixed |
| R1-F1.1  | `compose --shell` success: one stderr warning; shell report and exit 0 preserved | fixed | `compose_shell_report_keeps_the_warning`: exit 0, shell report kept, one stderr warning |
| R1-F1.2  | `compose --shell` with missing authored schema: earlier warning plus existing failure | fixed | `compose_shell_report_failure_keeps_the_warning`: exit 1, warning plus `missing.yaml` error |
| R1-F1.3  | lifecycle bootstrap `initialize: [skip]`, ordinary output: one stderr warning | fixed | `compose_lifecycle_skipped_by_initialize_keeps_the_bootstrap_warning` (ordinary leg) |
| R1-F1.4  | lifecycle bootstrap skip, `--output report-json`: one warning diagnostic, no stderr row | fixed | same test, report-json leg: one row, empty stderr, `document: null`, outcome success |
| R1-F1.5  | ordinary composition and approval failure delivery: preserve | clean | existing `compose_keeps_the_warning_*` / `compose_report_json_*` tests pass |
| R1-F1.6  | lifecycle continuing past initialize (with/without missing schema, both modes): warning once, no duplicate | clean | `compose_lifecycle_past_initialize_warns_once`: with/without missing schema, both modes, exactly one |
| R1-F1.7  | library report/failure carriers: preserve | clean | library carrier tests pass; new `a_preflight_walk_reports_the_skipped_trigger_once`, `the_effective_schema_reports_the_skipped_trigger` |
| R1-F1.8  | Claudine preliminary checks and invocation drain: preserve | clean | no Claudine code changed; claudine `just test invalid_trigger` 13/13, full claudine `just test` 11648/11648 |
| R1-F1.9  | Claudine selected composition/shell preparation: preserve existing discovery boundaries | clean | Claudine `compose_preflight` callers read only `entries`/`candidates` and never call `effective_schema_for`; discovery boundaries unchanged |
| R1-F2    | Inspection and failed clean reports route warnings to the wrong stream | fixed | every required child row verified; sweep R1-F2.10 clean |
| R1-F2.1  | `schema triggers`: warning on stderr; ordinary inspection data on stdout | fixed | `schema_triggers_warns_on_stderr_and_lists_the_skipped_trigger_on_stdout`; `schema_triggers::an_unparseable_claimed_schema_root_file_is_skipped_with_its_yaml_location` updated |
| R1-F2.2  | failed `clean --json`: retained warning in version-1 `warnings`; no stderr warning row; exit 1 | fixed | `clean_json_schema_failure_carries_the_warning_in_the_envelope`: exit 1, version-1 envelope, row in `warnings`, stderr has only the schema error |
| R1-F2.3  | ordinary clean (control and missing schema): preserve | clean | `clean_keeps_the_warning_when_the_authored_schema_fails_to_load`: one stderr warning, exit 1 |
| R1-F2.4  | successful `clean --json`: preserve | clean | `clean_json_carries_the_warning_in_the_top_level_warnings_array` passes |
| R1-F2.5  | schema validation JSON reports: preserve | clean | `schema_validate_json_*` and `schema_validate_assignments_use_the_valid_trigger_beside_an_invalid_one` pass |
| R1-F2.6  | ordinary schema validation: preserve | clean | `schema_validate_warns_once_on_stderr_for_several_files` passes |
| R1-F2.7  | ordinary/JSON composition: preserve | clean | all `compose_*` tests in `invalid_trigger_warnings.rs` pass |
| R1-F2.8  | Claudine CLI: preserve | clean | claudine `just test invalid_trigger` 13/13 |
| R1-F2.9  | DMLS protocol stdout: preserve | clean | DMLS `invalid_trigger_warnings` session tests read framed protocol messages; DMLS prints to stdout only for non-protocol subcommands |
| R1-F3    | Warning projections discard available source information | fixed | every required child row verified; siblings R1-F3.16–F3.18 fixed |
| R1-F3.1  | compose warning/shared diagnostic keeps trigger file and known `(3, 1)` position (JSON `source` not null) | fixed | `a_malformed_envelope_reports_the_trigger_file_at_its_yaml_error`; CLI `compose_report_json_places_a_malformed_envelope_at_its_yaml_error` (source = trigger at 3:1) |
| R1-F3.2  | `AmbiguousReferenced` payload path preserved in finding and DMLS related location | fixed | `a_payload_that_is_neither_schema_form_names_the_payload`; DMLS `payload_failures_relate_the_file_that_authored_the_rejected_schema` |
| R1-F3.3  | root-union merge rejection keeps resolved payload identity (finding, explanation, DMLS) | fixed | `a_referenced_root_union_payload_names_the_payload_file`, `a_non_object_payload_reached_through_an_alias_names_the_schema_file`; DMLS test above |
| R1-F3.4  | compile rejection through `$schema` alias keeps the actual schema source | fixed | `a_payload_that_fails_to_compile_through_an_alias_names_the_schema_file` (names `raw.json`); DMLS test above |
| R1-F3.5  | discovery position and DMLS primary location: preserve | clean | `malformed_claimed_yaml_is_a_positioned_finding` asserts `Some((3, 1))`; DMLS `a_malformed_envelope_is_placed_on_its_error_line_with_no_related_file` |
| R1-F3.6  | malformed YAML payload: preserve | clean | DMLS startup test; `a_payload_failure_names_the_payload_but_never_places_it_on_the_trigger` |
| R1-F3.7  | invalid SimplifiedSchema constraint: preserve | clean | `SchemaDocument` arm keeps the related path; constraint tests pass |
| R1-F3.8  | direct raw JSON compile failure: preserve (also via alias) | clean | direct `bad.json` test passes; alias form covered by R1-F3.4 |
| R1-F3.9  | malformed imported type file: preserve | clean | existing broken-import test passes |
| R1-F3.10 | unknown named type import: preserve | clean | `an_unknown_type_the_trigger_imports_names_the_searched_import` |
| R1-F3.11 | missing payload/import: preserve, no invented location | clean | `a_missing_import_names_no_file` and missing-payload tests |
| R1-F3.12 | unsupported envelope keys: file-level fallback preserved | clean | `a_trigger_failure_without_a_position_reports_no_source`; CLI incident test asserts `source: null` |
| R1-F3.13 | other path-bearing causes keep cause path | clean | `related_path` classifies every variant with no wildcard arm; payload-cycle and symlink tests |
| R1-F3.14 | CLI validation and clean warning rows: preserve | clean | `FindingRow` shape unchanged; CLI validate/clean tests pass |
| R1-F3.15 | embedded findings: virtual identity, no host path | clean | `an_embedded_trigger_record_has_no_source_even_with_a_position`; embedded `cause_path` stays `None` |
| R1-F1.10 | sibling: lifecycle call rejection dropped the check's discovery result | fixed | `compose_lifecycle_call_rejection_keeps_the_warning` |
| R1-F1.11 | sibling: lifecycle call warning blocked by `--deny` stopped before bootstrap without warning | fixed | `compose_lifecycle_blocked_call_warning_keeps_the_warning` |
| R1-F1.12 | sibling: reference refusal (exit 2) dropped warnings, including those discoverable only from a transcluded child | fixed | `compose_reference_refusal_keeps_the_warning`, `compose_reference_refusal_keeps_a_child_only_warning`, `compose_reference_report_and_composition_share_a_child_only_warning`, lib `reference_validation_reports_root_and_child_only_triggers_once` |
| R1-F1.13 | sibling: ordinary output lost the warning when a composed document failed to render | fixed | `compose_render_failure_keeps_the_warning` |
| R1-F1.14 | sibling: lifecycle failing in `initialize` or at a lifecycle shell during preparation | fixed | `compose_lifecycle_failing_in_initialize_keeps_the_bootstrap_warning`, `compose_lifecycle_shell_failing_in_preparation_keeps_the_bootstrap_warning` |
| R1-F2.10 | sweep: every other `md` command that could deliver trigger findings | clean | discovery runs only in `compose`, `clean`, `schema validate`, `schema triggers`; `validate refs`/`graph` run with trigger schemas off; `schema detect`/`about`/`read`/frontmatter never discover |
| R1-F3.16 | sibling: unknown type referenced inside an import file had no related path | fixed | `an_unknown_type_referenced_inside_an_import_names_the_import`; DMLS `types.yaml` case |
| R1-F3.17 | sibling: trigger/cause comparison on raw spellings named the trigger as its own related file through a symlinked root | fixed | `a_self_referencing_trigger_found_through_a_symlink_names_no_other_file` (unix) |
| R1-F3.18 | sibling: DMLS related-information text said "payload" for an import | fixed | `assert_related_at_file_start` asserts "the trigger's schema failed to load here"; `invalid_trigger` 57/57 |

### Work Log

- starting the work on 'R1-F1 early composition exits discard discovered trigger warnings' at 18:10:46
        - defect class: collected trigger-discovery findings are lost when a preparation path returns before the ordinary composition reporter runs
        - subagent 1 (rust-developer) repaired the named sites and swept siblings:
                - `ComposePreflightReport.warnings` now carries the pre-flight walk's skipped-trigger warnings (was always empty)
                - `effective_schema_for` returns skipped-trigger warnings with the schema (only caller: darkmatter-cli; claudine compiles)
                - new `PendingTriggerWarnings` in `cli/src/commands/compose.rs` holds early-pass findings (lifecycle call check, bootstrap, `--shell` walk) and merges them into the first report/failure, deduplicated by trigger source identity; exits that never compose deliver them directly
                - bootstrap now uses `compose_with_warnings`; `--shell` failure drains `take_preflight_failure_warnings`
                - siblings found and fixed: lifecycle call rejection (R1-F1.10), lifecycle call warning blocked by `--deny` (R1-F1.11), reference refusal (R1-F1.12), render failure in ordinary output (R1-F1.13), lifecycle failure in `initialize` / lifecycle shell during preparation (R1-F1.14)
        - subagent 2 (rust-developer) closed two gaps left by subagent 1:
                - R1-F1.12: the first repair re-ran root discovery on refusal, which missed triggers discoverable only from a transcluded child's context; `ReferenceValidationReport` now has an unserialized `trigger_warnings` field collected per graph node, and `refused_document_triggers` was removed
                - R1-F1.14: added tests for `initialize: [- error: ...]` and lifecycle shell failures during preparation (placeholder failure, unapproved shell)
        - clean, with reasons: approval pre-flight failure already drained warnings; `--shell` with `--output report-json` is a usage error; `--shell` is a passive inspection with no effects, so `--deny dm.schema.invalid_trigger` has no effect to stop before (spec: "stops composition before its next effect"); it now warns, where before it was silent
        - noted, not changed: if a node's preparation fails, the reference graph build fails and `check_references` treats it as clean; composition then reports that node's warnings itself
        - docs updated: `docs/topics/schemas/schema-triggers.md`, `docs/cli/compose.md`, `docs/inline/link-validation.md`, `docs/structs/Markdown.md`; skill `.claude/skills/darkmatter/SKILL.md`
        - scoped checks: `just test invalid_trigger` 49/49, `references` 98/98, `lifecycle` 454/454, `report_json` 17/17, `reference` 718/718, `graph` 220/220, `preflight` 156/156, `compose_shell` 38/38, `policy_verbs` 25/25, `just lint` clean, tier coverage 0 stranded, `cargo check -p claudine -p dmls` clean, claudine `just test invalid_trigger` 13/13
- work completed for 'R1-F1 early composition exits discard discovered trigger warnings' at 18:41:22
- starting the work on 'R1-F2 inspection and failed clean reports route warnings to the wrong stream' at 18:41:22
        - defect class: warning routing that bypasses the output mode's delivery boundary (ordinary → stderr once; structured report → in the report, never on stderr)
        - subagent 3 (rust-developer):
                - `schema triggers` (no JSON mode) now writes the warning row to stderr through the shared `emit_warnings`; its stdout "Skipped invalid triggers" trace section keeps only source and code, not a second copy of the warning
                - `clean()` now returns `CleanFailure { error, trigger_findings }` and `run_clean` routes by mode; failed `clean --json` prints the existing version-1 envelope (`json::schema_failure_report`: empty `diagnostics`/`applied`, `changed: false`, rows in `warnings`) and then returns the original error (exit 1 preserved)
                - an envelope is printed only on the schema-failure branch, the only failure where discovery ran; other early failures still print none (documented in `docs/cli/clean.md`)
                - updated tests that asserted the old stream: `schema_triggers_warns_on_stderr_and_lists_the_skipped_trigger_on_stdout`, `schema_triggers::an_unparseable_claimed_schema_root_file_is_skipped_with_its_yaml_location`
                - sweep (R1-F2.10): trigger discovery runs only in `compose`, `clean`, `schema validate`, `schema triggers`; `schema detect`/`about`/`read`/frontmatter never discover; `validate refs`/`graph` run with trigger schemas off
        - docs: `docs/topics/schemas/schema-triggers.md`, `docs/cli/clean.md`, `docs/cli/index.md`; skill `.claude/skills/darkmatter/schema.md`
        - scoped checks: `just test invalid_trigger` 50/50, `clean` 630/630, `triggers` 212/212 (after updating one stale stdout assertion), `schema_triggers` 35/35, `trigger_` 342/342, `just lint` clean, tier coverage 0 stranded, claudine `invalid_trigger` 13/13
- work completed for 'R1-F2 inspection and failed clean reports route warnings to the wrong stream' at 18:47:50
- starting the work on 'R1-F3 warning projections discard available source information' at 18:47:50
        - defect class: source provenance (position, payload/import identity) is lost while turning a discovery failure into a finding or projecting that finding into a host report
        - subagent 4 (rust-developer):
                - `ComposeWarning::from_invalid_trigger` now fills the shared diagnostic record: report-json `source` is the trigger identity at the known line/column; with no position, or for an embedded trigger, `source` stays `null` (the format's source-free representation, because report-json `source` requires a line); `line_number` stays `None` because it counts lines of the consuming document
                - `SchemaError::TriggerPayloadNotMergeable` gained public `payload_path: Option<PathBuf>`
                - `resolve_file_payload` takes the payload path from `resolved.origin.file()` (the last file of a reference chain) for both the merge and compile gates, replacing a single-file `referenced_files == [file]` check that gave up on chains
                - `related_path` now matches every variant with no wildcard arm: added `AmbiguousReferenced`, `payload_path`, `TriggerLoad` (recursed), and the named-type failures
                - siblings fixed: R1-F3.16 (unknown type referenced inside an import file had no related path) and R1-F3.17 (trigger/cause comparison used raw spellings, so a trigger reached through the macOS `/var` symlink named itself as its related file; now compared canonically, with each path kept in its cause's spelling)
                - downstream: DMLS and Claudine project the finding's own path/position/message, so they needed no change
        - orchestrator sibling fix (R1-F3.18): the DMLS related-information text said "the trigger's payload failed to load here" even when the related file is an import; it now says "the trigger's schema failed to load here", asserted in `assert_related_at_file_start` (`dmls/src/diagnostics/frontmatter.rs`, `dmls/tests/l1/invalid_trigger_warnings.rs`)
        - decision for the author to confirm: a cause without a position keeps report-json `source: null` rather than inventing a line 1 file-level location
        - docs: `docs/cli/report-json.md`, `docs/topics/schemas/schema-triggers.md`, `dmls/docs/diagnostics.md`; skill `.claude/skills/darkmatter/schema.md`; the `/var` symlink fact is already recorded in `.claude/skills/os/macos.md`
        - scoped checks: `invalid_trigger` 57/57 (rerun after the orchestrator fix), `trigger_` 357/357, `report_json` 18/18, `finding` 43/43, `payload` 125/125, `schemas::` 1149/1149, `just lint` clean, tier coverage 0 stranded, claudine `invalid_trigger` 13/13
- work completed for 'R1-F3 warning projections discard available source information' at 19:05:02
- starting phase gates at 19:05:02: full `just test` and `just lint` in darkmatter and claudine
        - darkmatter `just test`: 12506/12506 passed (5 slow under full-suite load; rerun alone, each passed in 2.9–3.5 s; none touched by this cycle, so recorded as load-sensitive)
        - darkmatter `just lint`: exit 0 (includes the zed-dmls wasm32-wasip2 check)
        - claudine `just test`: 11648/11648 passed (4 slow under load; rerun alone, each passed in 3.0–3.5 s; untouched by this cycle, load-sensitive)
        - claudine `just lint`: exit 0
        - `just check-tier-coverage darkmatter` and `claudine`: 0 stranded
        - cross-OS: `just cross-check darkmatter --os windows` (build-win-native): 9633/9633 passed, exit 0; run because R1-F3.17 changed path comparison
- phase gates completed at 19:21:26

### Completion Report

- **outcome:** complete
- **elapsed:** about 1 h 11 min (18:10:15 → 19:21:26 local)
- **parent findings:** total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 42 child rows (R1-F1: 14, R1-F2: 10, R1-F3: 18): 18 fixed (R1-F1.1–.4 and .10–.14; R1-F2.1–.2; R1-F3.1–.4 and .16–.18), 24 clean with evidence, 0 open
- verified repairs:
        - R1-F1: every early `md compose` exit (the `--shell` walk on success and failure, lifecycle bootstrap and `initialize` skip/failure, the lifecycle call check and its `--deny` block, reference refusal including child-only triggers, render failure) now delivers each skipped trigger once, in the stream or report its output mode requires
        - R1-F2: `md schema triggers` writes the warning to stderr; failed `md clean --json` puts it in the version-1 envelope's `warnings` and keeps stderr free of it
        - R1-F3: report-json `source` carries the trigger file and position; payload and import provenance survives `AmbiguousReferenced`, merge and compile rejections, alias chains, imports, and symlinked roots, and DMLS publishes it as related information
- decisions recorded for the author to confirm:
        - report-json keeps `source: null` for a cause with no known position, because that format's `source` requires a line (the spec's "source-free representation")
        - `schema triggers` stdout keeps a "Skipped invalid triggers" trace section listing source and code only; the warning text is on stderr
        - failed `clean --json` prints an envelope only on the schema-failure branch, the one failure where discovery ran
        - `--shell` is passive inspection, so `--deny dm.schema.invalid_trigger` has no effect to stop before; it now warns, and exit status is unchanged
- open findings: none
- public API changes: `ComposePreflightReport.warnings` is populated; `effective_schema_for` returns `(schema, warnings)`; `ReferenceValidationReport::trigger_warnings` (not serialized); `SchemaError::TriggerPayloadNotMergeable { payload_path }`
- changed files:
        - library: `darkmatter/lib/src/markdown/compose/{context/report.rs,preflight/collect.rs,preflight/mod.rs,report_diagnostics.rs,schema_validation.rs,tests/invalid_triggers.rs}`, `darkmatter/lib/src/markdown/reference/{graph.rs,types.rs,validate.rs}`, `darkmatter/lib/src/markdown/schemas/{errors.rs,triggers/assemble.rs,triggers/finding.rs}`, `darkmatter/lib/tests/l1/trigger_discovery_recovery.rs`
        - CLI: `darkmatter/cli/src/commands/{compose.rs,compose/lifecycle.rs,clean.rs,clean/json.rs,schema/triggers.rs}`, `darkmatter/cli/tests/l1/{invalid_trigger_warnings.rs,schema_triggers.rs}`
        - DMLS: `darkmatter/dmls/src/diagnostics/frontmatter.rs`, `darkmatter/dmls/tests/l1/invalid_trigger_warnings.rs`, `darkmatter/dmls/docs/diagnostics.md`
        - docs: `darkmatter/docs/cli/{clean.md,compose.md,index.md,report-json.md}`, `darkmatter/docs/inline/link-validation.md`, `darkmatter/docs/structs/Markdown.md`, `darkmatter/docs/topics/schemas/schema-triggers.md`
        - skills: `.claude/skills/darkmatter/{SKILL.md,schema.md}`
- final verification:
        - R1-F1, R1-F2, R1-F3: darkmatter `just test` 12506/12506, `just lint` clean; claudine `just test` 11648/11648, `just lint` clean; tier coverage 0 stranded for both areas; Windows cross-check of darkmatter 9633/9633
        - focused: `just test invalid_trigger` 57/57 after the last change

## Implementation of Review Findings #2

> **started at:** 2026-10-25T19:37:38-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-25-invalid-trigger-warnings/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs
- the review lists two high findings (R1-F1 recurring, R2-F1 new), no observations, no blockers, no human-review items
- recurrence list (minimum scope): R1-F1.5 (unchanged instance), R1-F1.15–R1-F1.20 (missed siblings), R2-F1 (decision-consumer sibling)

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1    | Early composition exits discard discovered trigger warnings | fixed | every required child row below fixed or verified clean; sweep-found R1-F1.21 fixed |
| R1-F1.5  | darkmatter-cli ordinary approval failure: root trigger + unapproved shell → approval error plus one trigger warning | fixed | CLI `compose_approval_failure_keeps_the_root_warning`: exit 1, `Approval required`, one warning, ordinary + JSON (failed with zero warnings before the fix) |
| R1-F1.15 | recursive collector: earlier child warns, later child missing schema → full warning prefix, once, in order (inspection + both compose consumers) | fixed | library `a_failed_walk_keeps_an_earlier_siblings_trigger`; CLI `a_later_siblings_failure_keeps_an_earlier_child_only_warning` (`--shell` + ordinary) and `..._in_a_lifecycle`, both output modes |
| R1-F1.16 | approval-wrapper failure projections (approval required, blacklist, policy-file error, handler rejection) carry warnings | fixed | `approval_required_keeps_the_root_trigger` / `_a_child_only_trigger`, `a_user_blacklist_keeps_the_root_trigger` / `_a_child_only_trigger`, `the_builtin_blacklist_keeps_a_child_only_trigger`, `a_handler_denial_keeps_a_child_only_trigger`, `an_unreadable_whitelist_keeps_a_child_only_trigger` (policy-file `PolicyIo`) |
| R1-F1.17 | approval-wrapper success result carries preflight warnings | fixed | `ComposePreflightApprovals::warnings`; `preflight_approvals_carry_a_skipped_childs_trigger` |
| R1-F1.18 | darkmatter-cli ordinary approval consumer merges success/failure preparation warnings (ordinary + JSON) | fixed | `compose_skipping_a_child_keeps_its_child_only_warning`, `compose_approval_failure_keeps_a_child_only_warning`, `compose_blacklist_refusal_keeps_a_child_only_warning` (ordinary + JSON) |
| R1-F1.19 | darkmatter-cli lifecycle approval consumer retains child-only findings independently of bootstrap | fixed | same three CLI tests repeated with `initialize: [{info: start}]` on the child-only fixture (bootstrap finds nothing, so it cannot mask the loss) |
| R1-F1.20 | in-pipeline membership collector retains warnings in composition report | fixed | `a_pre_approved_composition_reports_a_skipped_childs_trigger`; dedupe control `a_pre_approved_composition_reports_an_active_childs_trigger_once` |
| R1-F1.21 | `as_markdown` pre-approval gate failure: gate walk's trigger warnings delivered with the failure (found by sweep) | fixed | `as_markdown` gate keeps its report on failure; `NestedCompose::finish` hands nested triggers to `record_failed_triggers`; `a_failed_nested_gate_keeps_a_child_only_trigger` |
| R1-F1.1/F1.7 | public preflight / CLI inspection controls with false-condition child: one warning | clean | `a_preflight_walk_reports_a_skipped_childs_trigger_once` (new) and `compose_shell_report_keeps_the_warning` pass, one warning each |
| R1-F1.2  | failed-collector control: root warning before root missing schema retained | clean | `compose_shell_report_failure_keeps_the_warning`, `the_warning_is_kept_when_the_authored_schema_fails_to_load` pass |
| R1-F1.12 | reference-check/invocation controls: final warning once | clean | reference refusal/report tests incl. child-only and lifecycle variants pass (`just test reference` 720/720) |
| R1-F1.8/F1.9 | Claudine preliminary/composition consumers: preserve | clean | Claudine calls only `compose_preflight` with trigger discovery off (invariant); claudine full `just test` 11648/11648 |
| R2-F1    | Preflight-only trigger warnings bypass explicit denial | fixed | every child row fixed or verified clean; sweep-found R2-F1.10–F1.12 fixed |
| R2-F1.1  | approval wrapper applies request deny policy to collected warnings before approval | fixed | `stop_if_warnings_blocked` runs before policy files, prompts, approvals; `a_denied_child_only_trigger_stops_the_approval_walk_before_any_prompt` (handler never asked, `BlockedDiagnostics`, warning kept in sink) |
| R2-F1.2  | darkmatter-cli ordinary compose: whitelisted shell + skipped child, code/all denied → policy stop, shell not run | fixed | `compose_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell`: code + `all`, ordinary + JSON, exit 1, warning severity/trigger source, marker absent; default-policy leg runs the shell and warns |
| R2-F1.3  | darkmatter-cli lifecycle compose: same, initialize continuing → stop before shell | fixed | `compose_lifecycle_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell` (needed R2-F1.12) |
| R2-F1.4  | library full-document approval-membership gate (`validate_pre_approved`) checks/retains warning before execution | fixed | `validate_pre_approved` stops after adding walk triggers, before membership; `a_denied_child_only_trigger_stops_a_pre_approved_body_shell` / `_frontmatter_shell`, marker absent; default control runs one shell and warns |
| R2-F1.5  | root preparation/bootstrap policy controls: preserve | clean | existing `denying_the_code/all_stops_before_the_first_effect`, `compose_deny_of_the_code_or_all_stops_before_writing_the_document` pass |
| R2-F1.6  | active-child policy control: preserve | clean | new control `compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell` (ordinary + lifecycle, marker absent) |
| R2-F1.7  | frontmatter-only membership branch: contract clean | clean | contract: frontmatter-only branch of `validate_pre_approved` walks no body child; root triggers already stop in preparation's `stop_if_blocked` |
| R2-F1.8  | discovery opt-out and passive inspection: preserve | clean | `compose_with_schema_triggers_disabled_reports_nothing`; new `compose_shell_inspection_under_deny_warns_without_running_anything` (exit 0, warns, runs nothing; inspection unchanged) |
| R2-F1.9  | Claudine composition/approval consumers: invariant clean | clean | invariant: Claudine never sets `with_trigger_schemas(true)` on composition and never calls `compose_preflight_approvals*`; preliminary pass owns findings |
| R2-F1.10 | `as_markdown` gate: passing gate parked its walk warnings until `finish`, so nested content and root shells ran unjudged (found by sweep) | fixed | gate calls `stop_if_blocked` before membership; `a_denied_child_only_trigger_stops_the_nested_gate` |
| R2-F1.11 | `NestedCompose::finish` dropped every nested report on any failure (found by sweep) | fixed | `PipelineRuntime::record_failed_triggers` / `take_failed_warnings` (once per trigger); covered by `a_failed_nested_gate_keeps_a_child_only_trigger` |
| R2-F1.12 | CLI lifecycle reference check expanded whitelisted body `::shell` before the approval walk (effect ahead of policy, and ran twice) (found by sweep) | fixed | `check_references` disables `ShellExpansion`; reference `prepare_content` no longer re-enables a disabled operation; lifecycle marker test passes; full darkmatter suite 12533/12533 |

### Progress

- starting the work on 'R1-F1: early composition exits discard discovered trigger warnings' at 19:38:15
- R1-F1 class sweep (19:55): carriers between discovery and delivery checked: `collect_effects`/`collect_recursive` (lost successful-sibling triggers on later failure), `compose_preflight` (keeps `warnings`; failures go to sink), `compose_preflight_approvals_with` (dropped warnings on success and every post-collection error), `ComposePreflightApprovals` (no field), `validate_pre_approved` (pipeline and `as_markdown` gate; discarded walk warnings), CLI ordinary/lifecycle approval consumers (never held success warnings), CLI `--shell` (holds report/sink; clean), Claudine `compose_preflight` callers (trigger discovery disabled; invariant clean)
- R1-F1 repair, library (20:05):
        - `collect_recursive` is now a wrapper over `collect_document`; on failure it prepends the document's own kept warnings plus its successful descendants' trigger warnings (taken from the walk's trigger list) to the request's failure sink, replacing the `WarningsOnFailure` drop guard
        - `PreflightFailureWarnings::take` drains each skipped trigger once, first-kept order
        - `ComposePreflightApprovals::warnings` added; `compose_preflight_approvals_with` records the walk's warnings into the failure sink on every post-collection error (policy paths/load, blacklist, approval required, handler denial)
        - `validate_pre_approved` adds the walk's trigger warnings to the composition report (deduped by trigger via new `ComposeReport::add_trigger_warnings`) before judging membership, and drains the sink into the report on walk failure; the `as_markdown` gate folds them through a gate report
- R1-F1 repair, CLI (20:10): ordinary and lifecycle approval consumers `hold` `approvals.warnings` in `PendingTriggerWarnings`
- R1-F1 regressions (20:30):
        - library (`markdown::compose::tests::invalid_triggers`): `a_failed_walk_keeps_an_earlier_siblings_trigger` (F1.15), `preflight_approvals_carry_a_skipped_childs_trigger` (F1.17), `approval_required_keeps_the_root_trigger` / `_a_child_only_trigger`, `a_user_blacklist_keeps_the_root_trigger` / `_a_child_only_trigger`, `the_builtin_blacklist_keeps_a_child_only_trigger` (F1.16), `a_pre_approved_composition_reports_a_skipped_childs_trigger` and `..._an_active_childs_trigger_once` (F1.20), `a_preflight_walk_reports_a_skipped_childs_trigger_once` (F1.1/F1.7 control)
        - CLI (`invalid_trigger_warnings`): `compose_approval_failure_keeps_the_root_warning` (F1.5), `compose_skipping_a_child_keeps_its_child_only_warning`, `compose_approval_failure_keeps_a_child_only_warning`, `compose_blacklist_refusal_keeps_a_child_only_warning` (F1.18 + F1.19, ordinary/JSON, with and without `initialize`), `a_later_siblings_failure_keeps_an_earlier_child_only_warning` (+ `_in_a_lifecycle`) (F1.15 inspection + both consumers)
        - mutation check: disabling the sibling drain, the approval-error sink record, or the gate's report add fails the matching library and CLI tests (source restored, verified by grep)
- R1-F1 docs (20:35): `docs/topics/schemas/schema-triggers.md` (preparation-walk delivery: skipped child, approval/blacklist refusal, later-sibling failure); darkmatter skill carrier summary; rustdoc on `collect_recursive`, `ComposePreflightApprovals`, `compose_preflight_approvals`, `take_preflight_failure_warnings`, `validate_pre_approved`, `PendingTriggerWarnings`
- R1-F1 verification (20:40): darkmatter `just test invalid_trigger` 73/73, `trigger_` 365/365, `preflight` 159/159, `lifecycle` 455/455, `approval` 75/75, `nested` 355/355, `shell_template` 104/104, `report` 360/360, `transclu` 332/332, `shell` 1048/1048; `just lint` clean; `cargo check -p claudine -p claudine-cli --tests` clean; claudine `just test invalid_trigger` 13/13
- orchestrator review of the R1-F1 hand-back (19:56:05):
        - subagent reported a residual gap: when the `as_markdown` gate's `validate_pre_approved` errors, `gate_report` (with the walk's trigger warnings) is dropped, and `NestedCompose::finish` returns the restored error before any merge; confirmed by reading `nested.rs` `prepare_child`/`finish`. Added ledger row R1-F1.21
        - R1-F1.16 handler-denial and policy-file-error branches share the recorded error path but lack a dedicated regression; these remain open under R1-F1.16
        - both handed to the R2-F1 subagent, since it edits the same gate and approval wrapper
- work paused for 'R1-F1' at 19:56:05 (R1-F1.16 tests and R1-F1.21 carried into the next subagent)
- starting the work on 'R2-F1: preflight-only trigger warnings bypass explicit denial' at 19:56:05
        - note: the R1-F1 subagent's bullet times (20:05–20:40) are later than the real clock; its work ran 19:38–19:55 local time
- R2-F1 class sweep (19:57:01): sites deciding whether an effect may run once discovery warnings exist
        - `pipeline::stop_if_blocked` callers (preparation after trigger discovery, before frontmatter `$(…)`, before each body stage, between `::shell` commands, after `when=` preparation, nested child stop): clean
        - `compose_preflight_approvals_with`: defective, the walk's warnings never judged before policy files, prompts, or approvals
        - `validate_pre_approved` at the root pipeline gate: the R1-F1 report add plus the following `stop_if_blocked` already stopped before the first frontmatter shell; hardened to judge before membership
        - `validate_pre_approved` at the `as_markdown` gate (frontmatter pass 1): defective, a passing gate parked its warnings until `NestedCompose::finish`, so nested content and the root's shells ran unjudged; a failing gate dropped them (R1-F1.21)
        - `NestedCompose::finish`: defective sibling, any failure dropped every nested report (the gate's included)
        - CLI lifecycle `try_prepare` order (bootstrap, call check, `initialize`, reference check, approvals, compose): defective, the reference check's graph preparation ran whitelisted body `::shell` commands before the approval walk (found by the lifecycle marker test); ordinary mode's reference check ran with no shell policy yet
        - other `compose_preflight_approvals*` callers: none (Claudine never calls it)
        - frontmatter-only membership branch (R2-F1.7): walks no body child; root triggers stop in preparation; clean by reading
        - Claudine (R2-F1.9): composition requests never set `with_trigger_schemas(true)`; its discovery is the preliminary `DarkmatterSchemas::with_trigger_discovery` pass, which owns its findings; no approval-wrapper call; clean by reading
- R2-F1 repair, library (20:04:50):
        - `pipeline::stop_if_warnings_blocked` (shares `policy_stop` with `stop_if_blocked_with`); `compose_preflight_approvals_with` judges the walk's warnings first, returning `ShellExpansionError::Preflight(BlockedDiagnostics)` and recording the warnings in the failure sink
        - `validate_pre_approved` calls `stop_if_blocked` after adding the walk's triggers, before membership (both gates)
        - `as_markdown` gate keeps its report on success and failure; `NestedCompose::finish` hands nested triggers to `PipelineRuntime::record_failed_triggers` on any failure; `take_failed_warnings` keeps each trigger once, first report
        - library regressions in `compose::tests::invalid_triggers`: `a_denied_child_only_trigger_stops_the_approval_walk_before_any_prompt` (R2-F1.1), `a_denied_child_only_trigger_stops_a_pre_approved_body_shell` / `_frontmatter_shell` (R2-F1.4, `git init` marker absent; default-policy control runs it once and warns), `a_denied_child_only_trigger_stops_the_nested_gate`, `a_failed_nested_gate_keeps_a_child_only_trigger` (R1-F1.21), `a_handler_denial_keeps_a_child_only_trigger`, `an_unreadable_whitelist_keeps_a_child_only_trigger` (R1-F1.16)
        - mutation check: removing the wrapper check, the gate's stop, or the finish hand-off fails the matching tests; removing the R1-F1 gate add fails the body/frontmatter tests (sources restored by copy, verified by grep)
- R2-F1 repair, CLI (20:10): `check_references` disables `ShellExpansion` (passive check); reference `prepare_content` no longer re-enables an operation the request disabled
        - CLI regressions in `invalid_trigger_warnings`: `compose_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell` (R2-F1.2), `compose_lifecycle_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell` (R2-F1.3), `compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell` (R2-F1.6), `compose_shell_inspection_under_deny_warns_without_running_anything` (R2-F1.8)
- R2-F1 docs (20:15): `docs/cli/policy.md` (shell-preparation step), `docs/cli/compose.md` (passive reference check, preparation denial), `docs/topics/schemas/schema-triggers.md`, `docs/inline/preflight-checks.md`; darkmatter skill; rustdoc on `compose_preflight_approvals`, `validate_pre_approved`, `NestedCompose::finish`, `check_references`, `prepare_content`
- R2-F1 verification (20:11:18–20:17:22): darkmatter `just test invalid_trigger` 84/84, `trigger_` 373/373, `preflight` 159/159, `lifecycle` 456/456, `approval` 76/76, `nested` 357/357, `deny` 25/25, `policy` 244/244, `shell` 1054/1054, `reference` 720/720, `validate` 425/425, `transclu` 332/332, `report` 360/360, `compose_` 541/541, `graph` 220/220, `blocking` 36/36, `link` 436/436; `just lint` clean; `cargo check -p claudine -p claudine-cli --tests` clean; claudine `just test invalid_trigger` 13/13
- work finished for 'R2-F1' at 20:17:22 (R1-F1.16 and R1-F1.21 closed with it)
- orchestrator reconciliation (20:32:05):
        - read the combined diff of `preflight/lifecycle.rs` (the walk's warnings are checked by `stop_if_warnings_blocked` before `approve_collected`, and on any error they are recorded in the failure sink), `nested.rs` and `reference/graph.rs`; they match the subagents' reports
        - the R2-F1.12 change alters behavior for every reference-check caller, so the full suites of both affected areas were run after the final change (results below)
        - R2-F1 subagent's bullet times are consistent with the real clock
- work completed for 'R1-F1: early composition exits discard discovered trigger warnings' at 20:32:05
- work completed for 'R2-F1: preflight-only trigger warnings bypass explicit denial' at 20:32:05

### Completion Report

- **outcome: complete**: implementation complete, ready for review
- elapsed: 2026-10-25T19:37:38-07:00 → 20:32:05 local (about 56 min)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1: fixed. Required rows R1-F1.5 and R1-F1.15–F1.20 plus the sweep-found R1-F1.21 are fixed; controls R1-F1.1/F1.7, F1.2, F1.12 and F1.8/F1.9 are clean
        - R2-F1: fixed. R2-F1.1–F1.4 plus the sweep-found R2-F1.10–F1.12 are fixed; controls R2-F1.5–F1.9 are clean
- open finding or child IDs: none
- verified repairs:
        - the recursive preflight collector keeps every discovered trigger warning, including those from earlier successful siblings, when a later child fails, once each and in discovery order
        - the shell-approval wrapper returns the walk's warnings on success (`ComposePreflightApprovals::warnings`). On every later error (policy-file, blacklist, approval required, handler denial, deny stop) it leaves them in the request's failure sink. Both CLI consumers deliver them, in ordinary and report-json output
        - the in-pipeline pre-approval gate (root and `as_markdown`) adds the walk's trigger warnings to the composition report, deduplicated by trigger. On gate or nested failure the warnings are carried through the nested-failure hand-off
        - an explicitly denied invalid-trigger warning that only preparation discovers (`--deny dm.schema.invalid_trigger` or `--deny all`) stops with the existing `BlockedDiagnostics` stop. It stops before any policy file is read, any prompt is shown, or any approved command runs, keeps warning severity and the trigger source, and applies in ordinary, lifecycle, library pre-approved and nested-gate composition
        - the CLI reference check before composition is passive: it no longer expands body `::shell`, so lifecycle mode can no longer run an approved command before the approval walk (or run it twice)
- decisions for the author to confirm:
        - D1 (R2-F1.12): `md compose`'s up-front reference check no longer runs `::shell` commands. A broken link that only a command's output would produce is now reported during composition rather than up front. This matches the docs' description of that check as passive. The alternative, moving the approval walk ahead of the reference check in lifecycle mode, would change which error wins when a document has both a broken reference and an unapproved shell
        - D2 (R2-F1.3): `initialize` steps still run before the preparation walk, so a denied trigger held only by a child cannot stop an `initialize` shell. The warning is not known yet at that point ("when the warning becomes known"), and `initialize` comes first on purpose so it can create files the body includes. The docs now state this
        - D3 (R1-F1.15): a failed walk reports the parent's warnings, then the trigger warnings of earlier successful siblings, then the failed document's own warnings. Only trigger warnings are carried from successful siblings
- changed files in this iteration:
        - library: `darkmatter/lib/src/markdown/compose/preflight/{collect,mod,lifecycle}.rs`, `compose/context/{report,options}.rs`, `compose/pipeline/mod.rs`, `compose/nested.rs`, `compose/shell_expansion/types.rs`, `reference/graph.rs` (R1-F1, R2-F1)
        - CLI: `darkmatter/cli/src/commands/compose.rs`, `compose/lifecycle.rs` (R1-F1.18/F1.19, R2-F1.2/F1.3/F1.12)
        - tests: `darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs` (17 new), `darkmatter/cli/tests/l1/invalid_trigger_warnings.rs` (10 new); both are declared L1 targets
        - docs and skill: `darkmatter/docs/cli/{policy,compose}.md`, `darkmatter/docs/topics/schemas/schema-triggers.md`, `darkmatter/docs/inline/preflight-checks.md`, `.claude/skills/darkmatter/SKILL.md`
- final verification, run by the orchestrator after the last change with no subagent running:
        - darkmatter `just test`: 12533 run, 12533 passed, 10 skipped (all findings)
        - darkmatter `just lint`: clean (all findings)
        - claudine `just test`: 11648 run, 11648 passed, 9 skipped (R1-F1.8/F1.9, R2-F1.9; claudine compiles against the changed compose/reference code)
        - claudine `just lint`: exit 0. Its only output is the macOS linker's `__eh_frame` size note, which is not a lint finding
        - `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine`: 0 stranded
        - darkmatter `just test invalid_trigger`: 84/84, slowest under 2 s (5 s L1 budget)
- cross-OS: not run. The changes are compose-policy and warning-carrier logic with no path, process or OS-specific code, and the new tests use temp-dir fixtures and a non-UTF-8 whitelist file that fails the same way on every OS. The spec's existing `os_failing: windows` from an earlier phase is left unchanged
- no performance measurement was required; `deferred_perf_measurement` not set
