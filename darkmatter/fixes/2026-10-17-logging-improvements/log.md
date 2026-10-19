---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/fixes/2026-10-17-logging-improvements/spec.md"
review: "/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/fixes/2026-10-17-logging-improvements/review-1.md"
implementation_1: "2026-10-17T18:43:53-07:00"
implementation_2: "2026-10-17T20:31:37-07:00"
---

# Logging Improvements: Review-to-Implement Log

## Implementation of Review Findings #1

> **started at:** 2026-10-17T18:43:53-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/fixes/2026-10-17-logging-improvements/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- findings are worked serially, one subagent per finding, in the order R1-F5, R1-F1, R1-F3, R1-F2, R1-F4
        - smaller, self-contained registry fixes go first so the larger registry-lifecycle rework (R1-F4) lands on top of corrected classification and scope preparation

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1 | Startup omits package and package-area schema roots | fixed | all children verified incl. follow-up rows F1.16–F1.18; startup and every refresh prepare catalog packages/areas, fallback areas, and nested repositories through `contexts.for_document` |
| R1-F1.1 | Startup package-root preparation: nested package with broken trigger is prepared and classified at startup | fixed | `workspace_schema_scopes::startup_classifies_a_broken_trigger_in_an_unopened_package` passes; 0 Markdown parses |
| R1-F1.2 | Startup package-area preparation: area root and its named schema included in startup totals | fixed | `workspace_schema_scopes::startup_counts_the_named_schema_of_an_unopened_package_area` passes (`roots=2 scoped_schemas=1`) |
| R1-F1.3 | Startup readiness/count projection: scoped failure and counts reported with no open documents | fixed | `workspace_schema_scopes::startup_reports_an_unopened_package_failure_with_no_open_documents` passes (`initialized with failures … failed_triggers=1 failed_scopes=1`) |
| R1-F1.4 | Tree/home root preparation (control): known-count startup fixture keeps correct counts | clean | control: `startup_log_lines::*` incl. `startup_and_full_index_lines_carry_the_fixture_counts` pass |
| R1-F1.5 | Environment roots and per-document shadowing (control): cached registry equals direct discovery | clean | control: `schema_roots_parity::*` pass in the 338-test scoped run |
| R1-F1.6 | Sibling: workspace folder that is a repository subfolder | fixed | `a_workspace_subfolder_keeps_the_repository_roots_above_it` passes (roots=3, tree_schemas=1) |
| R1-F1.7 | Sibling: multiple workspace folders | fixed | `each_workspace_folder_prepares_only_the_packages_inside_it` passes |
| R1-F1.8 | Sibling: nested package areas | fixed | `nested_package_areas_each_prepare_their_roots` passes (scoped_schemas=3) |
| R1-F1.9 | Sibling: watched manifest change re-prepares workspace scopes | fixed | `a_watched_manifest_change_prepares_a_new_package` passes |
| R1-F1.10 | Sibling: `rescan_workspace` re-prepares workspace scopes | fixed | `a_server_rescan_prepares_a_new_package` passes |
| R1-F1.11 | Sibling: configuration reload re-prepares workspace scopes | fixed | `a_configuration_change_prepares_a_new_package` passes |
| R1-F1.12 | Sibling: watched YAML fix clears diagnostic in unopened scope | fixed | `a_watched_fix_clears_a_trigger_failure_in_an_unopened_package` passes |
| R1-F1.13 | Sibling: didSave / save-fallback fix in unopened scope | fixed | `a_saved_fix_clears_a_trigger_failure_in_an_unopened_package` passes |
| R1-F1.14 | Sibling: schema-directory watch registration | fixed | no code change needed: schema-location watches derive from prepared scopes, which now include unopened package/area roots |
| R1-F1.15 | Sibling: per-document lookup and other count lines | clean | per-document lookup, untitled buffers, file-list and full-index lines unchanged; they count listed files, not prepared scopes |
| R1-F1.16 | Sibling: package area from the monorepo first-directory fallback (no catalog entry) not prepared at startup | fixed | `workspace_schema_scopes::startup_prepares_a_fallback_package_area_with_no_catalog_entry` (failed_triggers=1 failed_scopes=1, 0 parses), `a_watched_schema_creation_prepares_a_fallback_package_area`, `first_level_directories_without_a_schema_root_add_no_scopes` pass |
| R1-F1.17 | Sibling: repositories nested under a non-repository workspace folder not prepared at startup | fixed | `workspace_schema_scopes::startup_prepares_repositories_nested_in_a_non_repository_folder` (roots=2 tree_schemas=2, 0 parses), `a_server_rescan_prepares_a_new_nested_repository`, unit `workspace::discover::tests::the_walk_reports_each_directory_holding_a_git_entry` pass; repositories recorded by the existing walk, no second walk |
| R1-F1.18 | Sibling: repository created mid-session found only at next relist | fixed | `workspace_schema_scopes::a_watched_git_config_creation_prepares_a_new_nested_repository` passes |
| R1-F2 | Unlisted Markdown targets never receive on-demand analysis | fixed | all children verified; unlisted resolved Markdown targets become graph dependencies (`DocumentRecord::dependency`) carrying headings/transclusions only, kept out of full-index answers; `referenced_targets::*` (33) pass |
| R1-F2.1 | Lazy parse eligibility / graph-target extraction admits resolved Markdown dependencies independent of discovery | fixed | `needs_parse` no longer requires listing; `target_paths` takes Markdown file targets; unit `graph::invalidate::tests::test_unlisted_markdown_targets_become_dependencies_outside_the_full_index` passes |
| R1-F2.2 | Fragment-link diagnostics report `#missing` for excluded and outside-folder targets | fixed | `referenced_targets::{excluded,outside}_target_missing_anchor` pass |
| R1-F2.3 | Heading completion after `#` offers `alpha`, `beta` for both targets | fixed | `referenced_targets::{excluded,outside}_target_heading_completion` pass (alpha, beta) |
| R1-F2.4 | Link hover on `#beta` shows title and Beta heading | fixed | `referenced_targets::{excluded,outside}_target_hover` pass (Alpha, Beta) |
| R1-F2.5 | Heading definition on `#beta` navigates to line two | fixed | `referenced_targets::{excluded,outside}_target_definition` pass (0-based line 2, `## Beta`, matching control) |
| R1-F2.6 | Transclusion traversal follows the dependency and reports the cycle | fixed | `referenced_targets::{excluded,outside}_target_transclusion_cycle` pass |
| R1-F2.7 | Target invalidation via watched changes and save-fallback rescans tracks/refreshes unlisted targets (change + deletion) | fixed | `*_watched_refresh`, `*_save_rescan_refresh`, `{excluded,outside}_target_folderless_save_refresh`, `an_outside_target_folder_is_watched_once_parsed` pass (change and deletion) |
| R1-F2.8 | Listed Markdown control across diagnostics, completion, hover, definition, cycles | clean | control: `referenced_targets::listed_target_*` cells pass |
| R1-F2.9 | Wiki heading targets (control): listed-candidate behavior preserved | clean | control: `referenced_targets::*_wiki_heading` pass; listed gives Alpha/Beta, unlisted gives nothing (no external wiki search) |
| R1-F2.10 | Sibling: `needs_parse` vs full-build/`is_full` coupling | fixed | full-build paths and `is_full` still look only at listed files; R1-F3 tests still pass |
| R1-F2.11 | Sibling: heading wiki-link targets | clean | wiki targets are list members by contract (R1-F2.9) |
| R1-F2.12 | Sibling: `reference_target_at` | fixed | covered by hover, definition, parsed-once cells |
| R1-F2.13 | Sibling: `completion_heading_target` | fixed | new `completion::heading_target_file`; heading-completion cells pass |
| R1-F2.14 | Sibling: watched-change filter rejected unlisted paths | fixed | new `unlisted_changed` filter; refresh cells pass |
| R1-F2.15 | Sibling: watch registration for dependency locations | fixed | dependency folders join schema-location watch registration; `an_outside_target_folder_is_watched_once_parsed` passes |
| R1-F2.16 | Sibling: save fallback without folder watches | fixed | `recheck_dependencies`; folderless save-refresh cells pass |
| R1-F2.17 | Sibling: graph membership used as file-list membership | fixed | path/`file(...)`/wiki-file completions, workspace symbols, wiki candidates, key index, portability, `transcluded_by`, full-index log counts exclude dependencies; `*_workspace_symbols` pass |
| R1-F2.18 | Sibling: references and rename | clean | a dependency adds no links, so no backlinks |
| R1-F2.19 | Sibling: document links, `diagnose_unresolved`, `listing_gates` | clean | read the same graph edges; covered by cells |
| R1-F2.20 | Sibling: `close_buffer`, `install_listing`, `open_buffer` | clean | unit test covers a dependency joining the list |
| R1-F3 | A ready full index stays ready after losing a readable document | fixed | all children verified; `WorkspaceIndex::is_full` = built && every listed path has a current parse (dmls/src/graph/invalidate.rs); request runs a build round that fails with the initial-build `RequestFailed` and retries later |
| R1-F3.1 | Watched modification to unreadable withdraws completeness and exposes failure | fixed | `full_index::a_ready_index_after_a_watched_modification_fails_{references,heading_rename,file_rename,workspace_symbols}` pass |
| R1-F3.2 | Watched creation of unreadable Markdown fails complete-index operations | fixed | `full_index::a_ready_index_after_a_watched_creation_fails_*` (4) pass |
| R1-F3.3 | Save-fallback rescan: same failure policy | fixed | `full_index::a_ready_index_after_a_save_rescan_fails_*` (4) pass |
| R1-F3.4 | Buffer close over unreadable disk bytes: same failure policy | fixed | `full_index::a_ready_index_after_a_buffer_close_fails_*` (4) pass |
| R1-F3.5 | File-list replacement (ignore-file relist) with unreadable new file: same failure policy | fixed | `full_index::a_ready_index_after_a_relist_fails_*` (4) pass (relist via `.ignore` creation) |
| R1-F3.6 | References: error naming the unreadable input after each transition | fixed | references column of the `readiness_withdrawn!` table: no result, `RequestFailed`, message names the file |
| R1-F3.7 | Heading rename: error, no partial edit | fixed | heading-rename column of the table: error, no edit |
| R1-F3.8 | File rename: error, no partial edit | fixed | file-rename column of the table: error, no edit |
| R1-F3.9 | Workspace symbols: error naming the unreadable input | fixed | workspace-symbols column of the table: error naming the file |
| R1-F3.10 | Recovery after watched modification restores complete answers | fixed | `full_index::a_repaired_file_after_readiness_is_read_again_and_complete_answers_return` passes; references and rename include the referrer again |
| R1-F3.11 | Initial full build unreadable (control): error and retry preserved | clean | control: `full_index::an_unreadable_listed_file_fails_the_build_and_a_later_request_retries` passes |
| R1-F3.12 | Deletion during build (control): stale work discarded, absence not treated as unreadable | clean | control: `full_index::edits_and_deletes_during_a_build_never_reach_the_ready_index` and unit `test_a_build_round_fails_only_for_current_unreadable_files` pass |
| R1-F3.13 | Sibling: watched change on a listed path with no parse was never retried | fixed | `disk_changed` now reads any listed path that needs a parse; covered by F3.10 test and unit test |
| R1-F3.14 | Sibling: `recheck_parsed` (rescan) skipped unparsed listed paths | fixed | `full_index::a_save_rescan_retries_a_file_that_failed_after_readiness` passes |
| R1-F3.15 | Sibling: `install_listing` read only newly joined paths after build | fixed | unit `test_a_read_failure_after_the_build_withdraws_completeness_until_a_reread_succeeds` passes |
| R1-F3.16 | Sibling: watched delete of a failed file | clean | `full_index::deleting_a_file_that_failed_after_readiness_completes_the_index` passes; leaving the list is absence, not unreadability |
| R1-F3.17 | Sibling: buffer open/change | clean | an open buffer is a current parse; asserted in the unit test |
| R1-F3.18 | Sibling: workspace file rename | clean | `willRenameFiles` is gated; its disk side arrives as delete + create (F3.2, F3.16) |
| R1-F3.19 | Sibling: configuration reload | clean | configuration relist goes through `install_listing` (F3.15) |
| R1-F3.20 | Sibling: other completeness consumers | clean | the only completeness gate is `ServerState::index_is_full` in `Router::handle_request`, used by all four triggers |
| R1-F3.21 | Sibling: lazy-target cache reusing old headings after a failed read | clean | `ensure_parsed` drops the old parse when a read fails (existing unit test) |
| R1-F4 | Trigger preparation still blocks message dispatch | fixed | all required children verified; `TriggerRegistries::lookup` is `&self` (lookup only); preparation is begin (short lock) → `Preparation::run` (no lock) → `install` (generation check) on a background registry worker (`dmls/src/registry_jobs.rs`); requests park on the shared `Waiter`; full dmls suite 1089 passed |
| R1-F4.1 | First document registry prepared off dispatch; only dependent work waits | fixed | `registry_preparation::a_first_document_registry_is_prepared_off_the_dispatch_thread`, `a_held_diagnostics_refresh_is_published_when_its_registry_installs` pass |
| R1-F4.2 | Watched schema reload: background preparation, atomic install | fixed | `registry_preparation::a_watched_schema_reload_prepares_off_the_dispatch_thread` passes |
| R1-F4.3 | YAML save reload: same asynchronous prerequisite | fixed | `registry_preparation::a_saved_yaml_reload_prepares_off_the_dispatch_thread` passes |
| R1-F4.4 | Configuration/context reload: same asynchronous prerequisite | fixed | `registry_preparation::a_configuration_reload_prepares_off_the_dispatch_thread` passes |
| R1-F4.5 | Save-fallback rescan: background preparation before dependent refresh | fixed | `registry_preparation::a_save_fallback_rescan_prepares_off_the_dispatch_thread` passes |
| R1-F4.6 | Schema-dependent request builder and trigger-authoring overlay only look up prepared state | fixed | `registry_preparation::waits_for_its_registry!` table (one test per schema-dependent method + authoring overlay) and unit `overlay::tests::the_server_overlay_build_only_looks_registries_up` pass |
| R1-F4.7 | Startup preparation does not hold the overlay mutex across preparation I/O | fixed | `registry_preparation::startup_preparation_runs_off_the_dispatch_thread`; units `a_preparation_runs_apart_from_the_cache_and_installs_in_one_step`, `an_older_preparation_never_replaces_a_newer_one`, `a_change_during_a_preparation_leaves_its_result_stale` pass |
| R1-F4.8 | Ready unchanged lookup (control): zero enumeration/read/hash/scan work | clean | control: `repeated_ready_lookups_do_no_io`, `repeated_overlay_builds_skip_the_registry_but_keep_schema_freshness`, `requests_on_a_ready_document_do_no_registry_work` pass |
| R1-F4.9 | Orchestrator-added: R1-F1 post-refresh workspace preparation ran on dispatch | fixed | `registry_preparation::the_post_refresh_workspace_preparation_runs_off_the_dispatch_thread` passes; `workspace_schema_scopes::*` green |
| R1-F4.10 | Sibling: serverless overlay entry point | clean | `OverlayState::for_document` prepares on the caller's thread by contract; the server never calls it |
| R1-F4.11 | Sibling: named-schema assembly and standalone named types | clean | out of scope per spec Review Decisions "Reader's note — cache scope" |
| R1-F4.12 | Sibling: `invalidate_paths` re-hashing changed paths | clean | specified R3 invalidation; reads only the event's paths |
| R1-F4.13 | Sibling: watch folders, summary, transitions, `load_error_for` | clean | lookups only |
| R1-F4.14 | Sibling: `relist`, `recheck_parsed`, `scan_manifests` during rescans/config changes run on dispatch | deferred | nonblocking follow-up: same defect class but not trigger-registry preparation; the spec's off-dispatch sentence covers startup preparation/discovery, and no spec sentence requires rescan-time relisting off dispatch; propose as a separate finding |
| R1-F4.15 | Sibling: referenced-target parsing on dispatch | clean | documented design: referenced-target parsing runs on dispatch under the index lock (R6 lazy parse) |
| R1-F4.16 | Required responsiveness proof: paused preparation permits edit, cancellation, shutdown | fixed | `registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`, `shutdown_while_a_preparation_is_held_answers_its_waiter` pass |
| R1-F5 | Trigger payload failures are missing from failed-trigger counts | fixed | all children verified; `Outcome::TriggerFailed { trigger_file }` records the failing trigger and `TriggerRegistries::summary` counts it under `failed_triggers` (dmls/src/overlay/registry.rs) |
| R1-F5.1 | Valid envelope + valid payload (control): 1 / 0; ready | clean | control: `startup_log_lines::a_loading_payload_counts_a_ready_trigger` passes (1/0/0, ready) |
| R1-F5.2 | `match: {}` envelope failure (control): 0 / 1; failed scope | clean | control: `startup_log_lines::a_malformed_envelope_counts_a_failed_trigger` passes (0/1/1) |
| R1-F5.3 | Missing payload file: 0 / 1; failed scope | fixed | `startup_log_lines::a_missing_payload_counts_a_failed_trigger` passes (0/1/1); fails with fix disabled showing reviewed 1/0/1 |
| R1-F5.4 | Payload parser failure: 0 / 1; failed scope | fixed | `startup_log_lines::an_unparseable_payload_counts_a_failed_trigger` passes (0/1/1) |
| R1-F5.5 | Payload import resolution failure: 0 / 1; failed scope | fixed | `startup_log_lines::an_unresolved_payload_import_counts_a_failed_trigger` passes (0/1/1) |
| R1-F5.6 | Payload example resolution failure: 0 / 1; failed scope | fixed | `startup_log_lines::an_unresolved_payload_example_counts_a_failed_trigger` passes (0/1/1) |
| R1-F5.7 | Startup failure-line and count projection keep failure-line selection and correct categories | fixed | each case above asserts line selection plus `trigger_schemas`/`failed_triggers`/`failed_scopes`; selection still keys on `failed_scopes` |
| R1-F5.8 | Sibling: trigger loads in one context and fails in another counts once, as failed | fixed | `overlay::registry::tests::a_payload_failure_counts_its_trigger_as_failed_once_across_contexts` passes |
| R1-F5.9 | Sibling: shadowed trigger never reached keeps its envelope class | fixed | `overlay::registry::tests::a_shadowed_trigger_counts_as_failed_only_once_a_preparation_reaches_its_payload` passes |
| R1-F5.10 | Sibling sweep: other class-to-count projections | clean | rescan/file-list lines, trigger-authoring diagnostics, `RootFileClass` re-export inspected: none project file classes into counts |

### R1-F5: Trigger payload failures in failed-trigger counts

- starting the work on 'R1-F5 trigger payload failures in failed-trigger counts' at 18:44:43
- defect class: a file's outcome category is decided before the operation that determines its success, and the count projection reads only that early category
- sibling sites checked
        - `TriggerRegistries::summary` (the only projection of root-file classes into counts): **fixed**
        - startup registry line `log_registry_ready` in `router.rs` (the only consumer of the summary; line selection keys on `failed_scopes`): **fixed** through the summary; failure-line selection unchanged
        - `workspace rescan complete` / `workspace files listed` lines: **clean**; they count listed Markdown/YAML files and never read `RootFileClass`
        - trigger-authoring overlay diagnostics (`load_error_for`, `take_transitions`): **clean**; they read the per-scope failure records, which already carry payload `TriggerLoad` errors, not the file class
        - `RootFileClass` re-export in `overlay/mod.rs`: **clean**; no other consumer in `dmls`, `lib`, or `cli`
- change (`dmls/src/overlay/registry.rs`)
        - root-file classification stays passive; `Outcome::TriggerFailed` now records the normalized trigger file a `TriggerLoad` error names
        - `summary` counts a `TriggerSchema` file as a failed trigger when any prepared scope failed it; the file still counts once
        - rule: failed in any prepared scope counts as failed (payloads resolve per context; counting it as loaded would hide a warned failure); a trigger no preparation reached (shadowed everywhere, or after an earlier failing trigger) keeps its envelope's class
        - doc comments on the module, `RootFileClass`, `RegistrySummary::{trigger_schemas, failed_triggers}`, and `summary` updated
- docs: `docs/lsp/indexing.md` classification table, `failed_triggers` field, and a new counting rule for per-context payload failures
- tests added
        - L1 `startup_log_lines`: `a_loading_payload_counts_a_ready_trigger` (F5.1), `a_malformed_envelope_counts_a_failed_trigger` (F5.2), `a_missing_payload_counts_a_failed_trigger` (F5.3), `an_unparseable_payload_counts_a_failed_trigger` (F5.4), `an_unresolved_payload_import_counts_a_failed_trigger` (F5.5), `an_unresolved_payload_example_counts_a_failed_trigger` (F5.6); each asserts the line variant plus trigger/failed/scope counts (F5.7)
        - unit `overlay::registry`: `a_payload_failure_counts_its_trigger_as_failed_once_across_contexts`, `a_shadowed_trigger_counts_as_failed_only_once_a_preparation_reaches_its_payload`
        - mutation check: disabling the new summary arm fails both unit tests and `a_missing_payload_counts_a_failed_trigger` (observed `trigger_schemas=1 failed_triggers=0 failed_scopes=1`); reverted
- commands
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/startup_log|overlay::/)'`: 174 passed
        - `just lint` (darkmatter): clean
- work completed for 'R1-F5 trigger payload failures in failed-trigger counts' at 18:49:21
        - orchestrator reviewed the `registry.rs` diff: classification stays passive; the context-bound load outcome is combined only in `summary`

### R1-F1: Startup package and package-area schema roots

- starting the work on 'R1-F1 startup omits package and package-area schema roots' at 18:49:21
- defect class: a parent scope's preparation is treated as covering the distinct scopes of its descendants, and a one-time preparation is treated as current after a refresh marks it stale
- sibling sites checked
        - startup `OverlayState::prepare_workspace_registries` (via `spawn_startup`): **fixed**; prepares each folder plus every catalog package-area and package root inside it (`workspace_probes`), each through `contexts.for_document` and the same `SchemaRoots::for_document` lookup a document there gets
        - startup registry line (`log_registry_ready` fed by `TriggerRegistries::summary`): **fixed** through the wider prepared set; counting rules unchanged
        - server rescan `ServerState::rescan_workspace`: **fixed**; invalidated every registry but never re-prepared unopened scopes, so a newly declared package and a fixed trigger in an unopened scope stayed stale; now prepares the workspace scopes after invalidating
        - configuration / context-generation reload `ServerState::update_config` (client settings and watched `.dmls.toml`): **fixed**; same pattern
        - watched files `apply_watched_changes` (package manifest / `.git` context inputs and watched YAML): **fixed**; prepares after either invalidation branch
        - `didSave` of a YAML file and the save-driven fallback when no schema watcher exists: **fixed**; both now prepare after invalidating
        - schema-location watches `sync_schema_watches` / `trigger_watch_folders`: **fixed** through the wider prepared set (package and area roots of unopened packages are now watched); no code change
        - per-document lookup (`OverlayCache::trigger_registry`, untitled buffers): **clean**; still the document's own context and five roots, unchanged
        - full-index and file-list lines: **clean**; they count listed Markdown/YAML, not prepared scopes
- changes
        - `biscuit-file/lib/src/file_reference/context.rs`: additive getter `FileResolutionContext::repository_scope_catalog()` (the catalog the context was built with had no accessor); `biscuit-file/docs/topics/file-references.md` inspector row updated
        - `dmls/src/overlay/mod.rs`: `prepare_workspace_registries` prepares the catalog-derived union; new private `workspace_probes`; `WORKSPACE_PROBE` doc
        - `dmls/src/router.rs`: new `ServerState::prepare_workspace_registries` called after every refresh that invalidates registries; doc comments on `rescan_workspace`, `reload_config`, `apply_watched_changes`, `spawn_startup`, `StartupEvent::RegistryReady`, `Internal::RegistryReady`
        - docs: `docs/lsp/indexing.md` (startup order, flowchart, scope table, lock granularity, refresh behavior, `roots`/`elapsed_ms` fields, counting rule replacing the "startup prepares only the folders" bullet); `docs/topics/schemas/dmls-schema-support.md` registry-preparation bullet
        - not prepared at startup (documented in `indexing.md`): a folder whose package area comes only from the monorepo first-directory fallback, and repositories nested below a non-repository workspace folder; both still prepare on first document analysis
- tests added (`dmls/tests/l1/workspace_schema_scopes.rs`, declared in `tests/l1/main.rs`)
        - F1.1 `startup_classifies_a_broken_trigger_in_an_unopened_package`: `dm.schema.prepare` published on the unopened `area/pkg/schemas/broken.yaml`; zero Markdown parses
        - F1.2 `startup_counts_the_named_schema_of_an_unopened_package_area`: `ready roots=2 tree_schemas=0 scoped_schemas=1 trigger_schemas=0 failed_triggers=0 failed_scopes=0`
        - F1.3 `startup_reports_an_unopened_package_failure_with_no_open_documents`: `initialized with failures roots=2 tree_schemas=0 scoped_schemas=1 trigger_schemas=0 unclassified=0 failed_triggers=1 failed_scopes=1`; zero parses
        - F1.6 `a_workspace_subfolder_keeps_the_repository_roots_above_it`, F1.7 `each_workspace_folder_prepares_only_the_packages_inside_it`, F1.8 `nested_package_areas_each_prepare_their_roots`
        - F1.9 `a_watched_manifest_change_prepares_a_new_package`, F1.10 `a_server_rescan_prepares_a_new_package`, F1.11 `a_configuration_change_prepares_a_new_package`, F1.12 `a_watched_fix_clears_a_trigger_failure_in_an_unopened_package`, F1.13 `a_saved_fix_clears_a_trigger_failure_in_an_unopened_package`
        - mutation checks: disabling the catalog loop fails all 11; disabling the refresh-time preparation fails exactly F1.9–F1.13; both reverted
        - updated expectations in `repository_contexts`: `a_watched_manifest_change_rebuilds_the_repository_context` (1 → 2) and `a_configuration_change_drops_every_context` (2 → 3); the workspace folder sits above the repositories and its own context is now rebuilt eagerly to prepare its registry
- commands
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/workspace_schema_scopes/)'`: 11 passed
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/startup|registry|schema_roots|overlay::|rescan|workspace_schema_scopes|repository_contexts|workspace_file_list|trigger|lsp_session/)'`: 338 passed (first run: 2 failed, the context-build counts above, before updating them)
        - `just lint` (darkmatter): clean; `just lint` (biscuit-file): clean
- work completed for 'R1-F1 startup omits package and package-area schema roots' at 19:04:22
- work paused for 'R1-F1 startup omits package and package-area schema roots' at 19:05:50
        - orchestrator review: catalog-derived probes go through `contexts.for_document`, so `SchemaRoots::for_document` stays the five-root authority
        - new cross-area change: `biscuit-file` gains getter `FileResolutionContext::repository_scope_catalog()`; `biscuit-file` added to the phase gates
        - the post-refresh `ServerState::prepare_workspace_registries` runs on dispatch; carried into R1-F4 as a required site
        - open sibling rows R1-F1.16 (first-directory fallback area) and R1-F1.17 (nested repositories) go to a follow-up after R1-F4

### R1-F3: Full-index readiness after a document becomes unreadable

- starting the work on 'R1-F3 ready full index stays ready after losing a readable document' at 19:05:50
- defect class: a completeness assertion is not withdrawn when an input it covers stops being current
        - `WorkspaceIndex::is_full` returned the build-completed flag alone; a read failure after the build dropped the document's parse without affecting it, and a listed path without a parse was never retried by later events
- sibling sites checked (dispositions)
        - mutators after readiness: watched change/create (`disk_changed`): **fixed** (reads whenever the built index needs a parse, so a failed path is retried); save rescan (`recheck_parsed`): **fixed** (also reads unparsed listed paths once built); buffer close (`close_buffer`): **fixed by readiness** (failed re-read now withdraws completeness); file-list replacement (`install_listing`, ignore-file relist and config relist): **fixed** (reads every unparsed listed path once built, not only the joined ones); watched delete (`disk_deleted`): **clean**, an unlisted path is absent, not unreadable; buffer open/change (`open_buffer`): **clean**, a buffer is a current parse; workspace file rename (`willRenameFiles` is a request; the disk side arrives as delete+create): **clean**, covered by the delete/create paths; configuration reload (`update_config`): relist is `install_listing` (fixed), `relink`/`set_wiki_roots` do not change parses: **clean**
        - completeness consumers: the only full-index prerequisite is `ServerState::index_is_full` in `Router::handle_request` (`Trigger::for_method`: references, rename, willRenameFiles, workspace symbols); no other `is_full` caller: **fixed** through `is_full`
        - lazy-target cache (R6): `ensure_parsed` already drops the old parse on a failed read, so old headings are never served: **clean** (already covered by `test_ensure_parsed_reuses_an_unchanged_parse_and_never_serves_a_failed_read`)
        - router state: `FullIndexState::Ready` is overwritten by `Building` when a request finds the index incomplete; the failed round reuses the first build's `RequestFailed` message and `Failed` state, and the next request retries: **fixed** (no new error type)
- changes
        - `dmls/src/graph/invalidate.rs`: field `full` renamed `built`; `is_full` = built and every listed document has a current parse; `disk_changed`, `recheck_parsed`, `install_listing` read unparsed listed paths once built; module and method docs updated
        - `dmls/src/full_index.rs`: state diagram gains Ready -> Building; `FullIndexState::Ready` doc
        - `dmls/src/router.rs`: `index_is_full` and `rescan_workspace` doc comments
        - `docs/lsp/indexing.md`: build-state diagram, "Complete, or not ready" rule with example, kept-current list, failure paragraph, referenced-target failed-read bullet
- tests
        - unit: `graph::invalidate::tests::test_a_read_failure_after_the_build_withdraws_completeness_until_a_reread_succeeds` (watched change, rescan, relist, buffer close, deletion)
        - L1 `tests/l1/full_index.rs`: `readiness_withdrawn!` table, 20 tests `a_ready_index_after_a_{watched_modification,watched_creation,save_rescan,buffer_close,relist}_fails_{references,heading_rename,file_rename,workspace_symbols}`; each asserts no result, `RequestFailed`, message names the unreadable file
        - recovery: `a_repaired_file_after_readiness_is_read_again_and_complete_answers_return`, `a_save_rescan_retries_a_file_that_failed_after_readiness`
        - control: `deleting_a_file_that_failed_after_readiness_completes_the_index`; existing `an_unreadable_listed_file_fails_the_build_and_a_later_request_retries` (R1-F3.11) and `edits_and_deletes_during_a_build_never_reach_the_ready_index` / `test_a_build_round_fails_only_for_current_unreadable_files` (R1-F3.12) pass unchanged
        - mutation check: reverting `is_full` to the flag alone fails 22 of the 23 new L1 tests (the deletion control passes, as expected); reverted
- commands
        - `cargo nextest run -p dmls --lib -E 'test(/invalidate/)'`: 25 passed
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/full_index|invalidate/)'`: 63 passed, 1 failed (recovery test counted the eager parse of the change notification; fixed by a round trip before counting), then 143 passed with the wider filter `test(/full_index|graph::|invalidate|rename|references|workspace_symbol/)`
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/full_index|graph::|invalidate|rename|references|workspace_symbol|workspace_file_list|lsp_session/)'`: 262 passed
        - `just lint` (darkmatter): exit 0
- work completed for 'R1-F3 ready full index stays ready after losing a readable document' at 19:11:43
- work completed for 'R1-F3 ready full index stays ready after losing a readable document' at 19:12:26
        - orchestrator review: `is_full` is one pass over the listed set per gated request; acceptable cost; R1-F2 must keep its meaning when it changes `needs_parse`

### R1-F2: On-demand analysis of unlisted Markdown targets

- starting the work on 'R1-F2 unlisted Markdown targets never receive on-demand analysis' at 19:12:26
- defect class: membership in the discovery list is used as eligibility for analysis of a target that resolution already found
        - `WorkspaceIndex::needs_parse` admitted only listed paths, and every target extractor (`target_paths`, `reference_target_at`, `completion_heading_target`) followed node edges only, so a resolved `EdgeTarget::File` Markdown target was never offered to the parser; watched events for unlisted paths were dropped by `listed_kind`
        - the Phase 4 departure "unchecked targets outside the workspace" is rejected; `entry_point_parity::markdown_links_resolve_existing_targets_above_a_narrower_workspace_folder` encoded it (narrower folder expected `#anywhere` unchecked) and now expects the same anchor checking as the whole-repository control
- design
        - the file list stays separate: a Markdown target read while unlisted is recorded in `WorkspaceIndex::unlisted_reads`; it is a dependency while parsed, unlisted, and closed (`is_dependency`, `dependencies()`)
        - `DocumentRecord::dependency`: the graph mints only headings and transclusions for it (no links, wiki links, frontmatter keys, schema/file/variable uses), so it adds no backlinks; it is not a wiki candidate (`DocumentRecord::wiki_doc` gives an empty logical path), not in the key index or portability collisions, and `transcluded_by` skips it
        - base-branch check (`git show main:darkmatter/dmls/src/graph/invalidate.rs`): on `main` the graph held discovered files and open buffers only, so an unlisted target never contributed to references, rename, workspace symbols, or wiki resolution; dependencies are kept out of all of those, and do not contribute backlinks
        - eligibility is `graph::is_markdown_path` (`.md`/`.markdown`, any case), so a `::code ./mod.rs` target is never parsed as Markdown
        - `is_full` unchanged in meaning: it iterates the listed set only; a dependency never withdraws or grants completeness
- sibling sites checked (dispositions)
        - R1-F2.10 `needs_parse`: **fixed** (no listed requirement; callers pass only listed paths or resolved Markdown targets); `unparsed_paths`, `begin_full_build`, `finish_full_build` (`current` requires listed), `is_full`: **clean**, still listed-only
        - R1-F2.11 `prepare_targets`/`target_paths` (fragment links, transclusion closure): **fixed** (Markdown `File` edges admitted); heading wiki links: **clean by design** (wiki candidates are list members, R1-F2.9)
        - R1-F2.12 `providers::reference_target_at` (hover, definition, completion request preparation): **fixed** (Markdown `File` edges)
        - R1-F2.13 `providers::completion_heading_target` / `completion::anchor_target`: **fixed** (`completion::heading_target_file` resolves the anchor path with `ctx.resolve_reference` when no graph document matches)
        - R1-F2.14 router `apply_disk_change` filtering: **fixed** (`WorkspaceIndex::unlisted_changed`: a dependency is marked stale; a created unlisted Markdown file rebuilds the graph so a reference to it resolves again); deletion (`disk_deleted`): **clean**, already unconditional, now also forgets the unlisted read
        - R1-F2.15 watch registration: **fixed** (`sync_schema_watches` adds each dependency's folder to the schema-location registration, `<folder>/*` rooted at its parent; reused, no new registration id)
        - R1-F2.16 save fallback for a client without `relativePatternSupport`: **fixed** (`ServerState::recheck_dependencies`: re-reads every dependency, else re-resolves the graph); server-rescan mode (`rescan_workspace`/`recheck_parsed`): **clean**, already re-reads every parsed closed document, and `install_listing` always rebuilds
        - R1-F2.17 graph-membership consumers that stand for the file list: `completion::path_completions`, `frontmatter::file_path_completions`, `wiki::file_completions`, `symbols::workspace_symbols`, `WorkspaceGraph::wiki_docs` (wiki resolution, `heading_target_doc`, rename `post_docs`), key index, portability collisions, `dsl::references` via `transcluded_by`, `log_full_index_built` heading/link counts: **fixed** (dependencies excluded)
        - R1-F2.18 references and heading/file rename (`references.rs`, `rename.rs`, `wiki.rs` incoming `References`): **clean**, a dependency mints no links
        - R1-F2.19 document links (`definition::document_links`), `diagnose_unresolved`, `listing_gates`: **clean**, they read the same edges and now see the parsed dependency exactly as a listed target
        - R1-F2.20 `close_buffer` of an unlisted buffer, `install_listing` join/leave, `open_buffer`: **clean** (a dependency that joins the list becomes a listed document; one that leaves loses its parse and is re-read as a dependency on next need)
- changes
        - `dmls/src/graph/arena.rs`: `DocumentRecord::dependency`, `DocumentRecord::wiki_doc`, `is_markdown_path`; `build_listed` takes the dependency set; dependency documents mint headings and transclusions only; key index, portability, `transcluded_by` skip them
        - `dmls/src/graph/invalidate.rs`: `unlisted_reads`; `needs_parse`, `is_dependency`, `dependencies`, `unlisted_changed`, `recheck_dependencies`, `reresolve`; `target_paths` admits Markdown `File` edges; module and method docs
        - `dmls/src/graph/mod.rs`: export `is_markdown_path`
        - `dmls/src/providers/{mod,completion,frontmatter,symbols,wiki}.rs`: target extraction and dependency exclusion as above
        - `dmls/src/router.rs`: `apply_disk_change`, `sync_schema_watches`, `dependency_folders`, `recheck_dependencies`, didSave fallback, `log_full_index_built`, `rescan_workspace` doc
        - docs: `docs/lsp/indexing.md` (new "Targets outside the file list" with examples, lifecycle diagram, watch table; failed-read, Markdown-only, watching, list-current rows), `docs/lsp/architecture.md` ("Two ways to resolve" table and prose), `dmls/docs/file-references.md`, `.claude/skills/darkmatter/dmls.md`
- tests
        - unit: `graph::invalidate::tests::test_unlisted_markdown_targets_become_dependencies_outside_the_full_index` (one parse and one read, never `mod.rs`; `is_full` stays true; no links/wiki candidacy/`transcluded_by`; stale mark, recheck, failed read, re-creation, joining the list)
        - L1 `tests/l1/referenced_targets.rs` (declared in `tests/l1/main.rs`), `cells!` macro, one test per (case × consumer), cases `listed` (control), `excluded` (`workspace.exclude`), `outside` (folder `docs/`, `../target.md`, minimal repository): `*_missing_anchor` (R1-F2.2), `*_heading_completion` (R1-F2.3), `*_hover` (R1-F2.4), `*_definition` (R1-F2.5, line 2 zero-based, the `## Beta` line, same as the control), `*_transclusion_cycle` (R1-F2.6), `*_watched_refresh`, `*_save_rescan_refresh`, `{excluded,outside}_target_folderless_save_refresh` (R1-F2.7: change, deletion, re-creation), `*_parsed_once` (2 parses, 1 read on open; 0/0 for later hover/definition; `::code` `mod.rs` never parsed), `*_wiki_heading` (R1-F2.9), `*_workspace_symbols`, and `an_outside_target_folder_is_watched_once_parsed`; 33 tests, each under 0.1 s
        - mutation checks (run with `--no-fail-fast` for the diagnostic only): `is_markdown_path` forced false fails exactly the 19 unlisted analysis cells and the watch-registration test, the 14 controls pass; `DocumentRecord::dependency` forced false fails exactly the 4 unlisted `wiki_heading`/`workspace_symbols` cells; both reverted
        - updated: `entry_point_parity::markdown_links_resolve_existing_targets_above_a_narrower_workspace_folder` (see the rejected departure above)
- commands
        - `cargo nextest run -p dmls --lib -E 'test(/invalidate|graph::/)'`: 79 passed (first run: 3 failed, because inferring dependencies from "parsed, unlisted, closed" misclassified `set_document` documents; fixed by recording unlisted reads explicitly)
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/referenced_targets/)'`: 33 passed
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/lazy|target|invalidate|graph::|full_index|workspace_file_list|anchor|completion|hover|definition|cycle|wiki|referenced_targets|symbol|references|rename|lsp_session|level1_graph_index|startup|rescan|watch|transclu/)'`: 272 passed, 1 failed (`entry_point_parity` above, rerun alone, updated, then passed)
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/lazy|target|invalidate|graph::|full_index|workspace_file_list|anchor|completion|hover|definition|cycle|wiki|referenced_targets|entry_point_parity/)'`: 395 passed
        - `cargo nextest run -p dmls --features effects-instrumentation` (dmls only): 1060 passed, 6 skipped
        - `just lint` (darkmatter): clean
- work completed for 'R1-F2 unlisted Markdown targets never receive on-demand analysis' at 19:30:31
- work completed for 'R1-F2 unlisted Markdown targets never receive on-demand analysis' at 19:31:43
        - orchestrator review: `entry_point_parity::markdown_links_resolve_existing_targets_above_a_narrower_workspace_folder` was updated because it pinned the departure this finding rejects; accepted

### R1-F4: Trigger preparation off the dispatch thread

- starting the work on 'R1-F4 trigger preparation still blocks message dispatch' at 19:31:43
- defect class: prerequisite I/O for a message (trigger-registry preparation: root enumeration, root reads, payload resolution, dependency hashing) ran synchronously on the message-dispatch thread, or under the overlay lock the dispatch thread's lookups need
- design
        - `TriggerRegistries::lookup(&self, path)` and `is_fresh` are lookups only (`&self`, so the overlay path cannot prepare by type); preparation is three steps: `begin` (under the lock: copies the cached root contents, takes a sequence number and the change epoch), `Preparation::run` (no lock: all I/O), `install` (under the lock: drops a result older than the installed scope's `seq`; adopts the copy's root reads only when no change arrived meanwhile; replays changes recorded since `begin` against the new scope, leaving it stale when one touches what it read; `record_failure`/`record_recovery` stay the one source of truth for "this failure is new")
        - `OverlayState::registry_demand` (lookup) says whether a document needs a registry (frontmatter block, or a claimed envelope in `schemas/`) and returns a `RegistryNeed`; `OverlayState::prepare_registry` runs the three steps; `prepare_workspace_registries` uses it, so the startup thread no longer holds the lock across preparation
        - new registry worker thread (`router::spawn_registry_worker`) runs `RegistryJob::Workspace` / `RegistryJob::Document` in queue order and reports `Internal::RegistryPrepared`; `registry_jobs::PendingRegistries` (in `ServerState`) dedupes one job per document and holds its waiting requests and held diagnostics refresh
        - requests: `Router::proceed` checks the full index, then the document's registry (`SCHEMA_DEPENDENT` methods), parking a `Waiter` (shared with the full index, a generalization of its pattern rather than a duplicate state machine: the full index is round-based over the index, registries are per-document jobs); `$/cancelRequest` answers a registry waiter `RequestCanceled` and keeps the job; `shutdown` answers registry waiters `ServerCancelled`; the full-index completion now resumes waiters through `proceed`
        - notifications: `refresh_diagnostics` holds an open document's publish while its registry is not fresh and queues its job; the job's completion publishes it, then the trigger-file transitions (`publish_trigger_transitions`, split out of `refresh_all_diagnostics`), then answers waiters (diagnostics before answers, as for the full index); every post-refresh `prepare_workspace_registries` site now queues `RegistryJob::Workspace`
        - background threads log through the server thread's subscriber (`tracing::dispatcher` handed to the startup thread and the worker)
        - completion events gain `waited_for_trigger_registry`
- sites and dispositions
        - R1-F4.1 first document registry: **fixed** (`Document` job; requests and diagnostics wait)
        - R1-F4.2 watched schema reload: **fixed** (`Workspace` job plus held document refreshes)
        - R1-F4.3 YAML didSave reload (watched and fallback branches): **fixed**
        - R1-F4.4 configuration/context reload (`update_config`): **fixed**
        - R1-F4.5 save-fallback rescan (`rescan_workspace`): **fixed**
        - R1-F4.6 schema-dependent request builder (`schema_for`) and trigger-authoring overlay: **fixed** (lookup only; `for_document_timed` documented as never preparing)
        - R1-F4.7 startup preparation: **fixed** (lock not held across I/O; still off dispatch)
        - R1-F4.8 ready unchanged lookup: **clean**, control kept green
        - R1-F4.9 R1-F1's post-refresh `ServerState::prepare_workspace_registries`: **fixed** (queued on the worker; transitions published on completion)
        - R1-F4.10 `OverlayState::for_document` (no-server entry used by unit and corpus tests): **clean**, prepares inline on the caller's thread by contract; the router never calls it
        - R1-F4.11 named-schema assembly, `standalone_named_types`, `deps_unchanged` re-hashing on dispatch: **clean / out of scope**, the spec's "Reader's note — cache scope" keeps these freshness reads
        - R1-F4.12 `invalidate_paths` re-hashing the event's changed paths on dispatch: **clean**, specified R3 invalidation (event-bounded reads, no enumeration or payload resolution); the in-flight replay compares nothing, so `install` reads nothing under the lock
        - R1-F4.13 `trigger_watch_folders`, `trigger_registry_summary`, `take_trigger_diagnostic_transitions`, `load_error_for`: **clean**, lookups
        - R1-F4.14 `ServerState::relist` and `rescan_workspace`'s `recheck_parsed`/`scan_manifests` (file-list walk and re-reads on dispatch for rescans and configuration changes): **deferred**, same defect class but not registry preparation; reported to the orchestrator as a candidate finding
        - R1-F4.15 referenced-target parsing on dispatch (`prepare_document`, `prepare_target`): **clean**, documented design ("All parsing runs on the dispatch thread under the index lock"), bounded per need
- changes
        - `dmls/src/overlay/registry.rs`: `Preparation`, `Prepared`, `begin`/`run`/`install`/`prepare`, `lookup(&self)`, `is_fresh`, change log (`Change`, `epoch`, `in_flight`, `changes`), `scope_affected`, `roots_read` as a free function, `RootContents::adopt`, `Scope::seq`; module docs
        - `dmls/src/overlay/mod.rs`: `RegistryNeed`, `registry_demand`, `prepare_registry`, `prepare_for_document`, lookup-only `trigger_registry`, `is_trigger_envelope`; `for_document`/`for_document_timed`/`prepare_workspace_registries` docs
        - `dmls/src/registry_jobs.rs` (new): `RegistryJobKind` (re-exported), `RegistryJob`, `Pending`, `PendingRegistries`
        - `dmls/src/router.rs`: worker, `SCHEMA_DEPENDENT`, `StartupHooks::registry_gate`, `StartupEvent::{RegistryPreparing, RegistryPrepared, RegistryJobsOutstanding}`, `Internal::RegistryPrepared`, `proceed`, `request_registry_need`, `finish_registry_job`, `cancel_registry_waiter`, `abandon_registry_waiters`, `registry_need`, `publish_trigger_transitions`, queued `prepare_workspace_registries`; docs
        - `dmls/src/request_timing.rs`: `Wait::TriggerRegistry`, `waited_for_trigger_registry`; `dmls/src/full_index.rs`: `Waiter` doc; `dmls/src/work_counters.rs`: `registry_scan_threads`; `dmls/src/lib.rs`
        - tests: `tests/common/mod.rs` (`start_with_registry_gate`, `release_registry_job`, `open_registry_gate`, `wait_for_registry_prepared`, `next_registry_prepared`, `server_thread_id`, `round_trip`; `flush_server` now also waits for queued registry jobs and buffers what they sent); `tests/l1/stdio_subprocess.rs` (a pipelined hover before `shutdown` may be answered `ServerCancelled`; the assertion accepts either answer frame)
        - docs: `docs/lsp/indexing.md` (new "Preparing registries off the dispatch thread" with a sequence diagram; the "Requests wait" row; the startup lock paragraph removed; refresh paragraph; timing samples and the new field), `docs/topics/schemas/dmls-schema-support.md`, `.claude/skills/darkmatter/dmls.md`
- tests
        - unit (`overlay::registry::tests`): `a_lookup_never_prepares`, `a_preparation_runs_apart_from_the_cache_and_installs_in_one_step`, `an_older_preparation_never_replaces_a_newer_one`, `a_change_during_a_preparation_leaves_its_result_stale`; (`overlay::tests`) `the_server_overlay_build_only_looks_registries_up`; R1-F4.8 controls `repeated_ready_lookups_do_no_io`, `repeated_overlay_builds_skip_the_registry_but_keep_schema_freshness` unchanged and green
        - L1 `tests/l1/registry_preparation.rs` (24 tests, each about 0.1 s): `a_held_preparation_leaves_edits_cancellation_and_other_requests_running` (didChange, another document's request, `$/cancelRequest` answered `RequestCanceled`, the other waiter resumes with the edited text, `waited_for_trigger_registry=true`), `shutdown_while_a_preparation_is_held_answers_its_waiter`, `a_held_diagnostics_refresh_is_published_when_its_registry_installs`; `waits_for_its_registry!` table, one test per schema-dependent method plus the trigger-authoring overlay (R1-F4.6); thread placement `startup_preparation_runs_off_the_dispatch_thread` (F4.7), `a_first_document_registry_is_prepared_off_the_dispatch_thread` (F4.1), `a_watched_schema_reload_…` (F4.2), `a_saved_yaml_reload_…` (F4.3), `a_configuration_reload_…` (F4.4), `a_save_fallback_rescan_…` (F4.5), `the_post_refresh_workspace_preparation_…` (F4.9); control `requests_on_a_ready_document_do_no_registry_work` (F4.8)
        - mutation checks (diagnostic runs, reverted): preparing inline in `ServerState::registry_need` fails 22 of 24 new tests (all but startup placement and the ready control); requests never waiting fails 14 (every wait case; references made discriminating by building the full index first)
        - existing tests that used `flush_server` as a barrier (`entry_point_parity`, `repository_contexts`, `lsp_session` trigger repairs) assumed synchronous preparation; fixed in the fixture barrier, not in the tests; a flush race (a job finishing after the response) was found by repetition and fixed (`drain_registry_job_counts` forces another round trip whenever a count moved)
- commands
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/registry_preparation/)'`: 24 passed
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/overlay::registry/)'`: 20 passed
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/registry_preparation|lsp_session|workspace_schema_scopes|entry_point_parity|repository_contexts/)'` repeated 15 times: 168 passed each
        - `cargo nextest run -p dmls --features effects-instrumentation` (dmls only): 1089 passed, 6 skipped (first diagnostic run before the fixture barrier: 12 failed, listed above)
        - `cargo clippy -p dmls --features terminal-tests,effects-instrumentation --all-targets -- -D warnings`: clean; `cargo check -p dmls --tests`: clean
        - `just lint` (darkmatter): clean (first run flagged `large_enum_variant` on `RegistryJob`; boxed)
- work completed for 'R1-F4 trigger preparation still blocks message dispatch' at 20:07:43
        - orchestrator review: behavior changes accepted. A pipelined request still waiting at `shutdown` may be answered `ServerCancelled`, and the stdio routing test now accepts either frame. `flush_server` also drains registry jobs
        - R1-F4.14 recorded as a nonblocking follow-up (no spec authority for rescan relisting off dispatch); it is not counted as a required child

### R1-F1 follow-up: scopes outside the package catalog

- starting the work on 'R1-F1 follow-up (R1-F1.16, R1-F1.17)' at 20:07:43
- investigation
        - R1-F1.16, fallback area: `RepositoryScopeCatalog::scope_for` (biscuit-file `file_reference/context.rs`) gives a base no listed package contains, and no listed area contains, the area `{repo}/{first component of base relative to repo}`, only when the catalog was built with `PackageAreaFallback::FirstComponent`; darkmatter's `repository_scope_catalog` sets that exactly when Sniff reports `is_monorepo`. The fallback is computed per document and never appears in `package_area_roots()`, so `workspace_probes` could not name it. A document at the repository root has no area (empty relative path); a document in `{repo}/docs/x` has `{repo}/docs`. So a probe at `{child}/.dmls-workspace` for a first-level child is representative of every document in that child that no package claims
        - R1-F1.17, nested repositories: `RepositoryContexts::for_document` keys a document's folder by `biscuit_file::find_git_root(folder)`, the nearest enclosing repository, and builds one context at that root. A document in `W/one/` (W not a repository) therefore gets `one`'s context, catalog, and tree root `W/one/schemas`, while `W`'s own probe gets a repository-less context at `W`. The roots of `one` are reachable only through a probe inside `one`; finding `one` needs a walk, and the startup file-list walk already visits every directory and drops `.git` in `filter_entry`
        - probe cost for a directory without a `schemas` folder: `begin`/`run` creates and scans a scope (one `registry_scans` count, context derivation, and a `find_git_root` per folder) even when every root dedups, so such probes are not free; the derivation filters on a `schemas` metadata read before any context is built
- decisions
        - fallback areas (`overlay::fallback_areas`): one `read_dir` of the catalog's repository root (only when that root lies inside the scope folder, the same rule as catalog roots), first-level directories other than `.git` and listed catalog roots, kept only with a `schemas` directory (`symlink_metadata`, not following a symlink, as the package-area root does). Each survivor's probe goes through `contexts.for_document`; it is kept only when its context's `package_area()` is that directory and `SchemaRoots::for_document` reports the `PackageArea` root `Searched` (an `Err` keeps the probe, so preparation reports the unreadable root). No ancestor walk, no environment read
        - nested repositories: `discover_workspace`'s `filter_entry` records each skipped `.git` entry's parent (`DiscoveryReport::repositories`, dir or file, so linked worktrees and submodules count); no second walk. The set is held in `OverlayState::workspace_repositories` (its own lock), and `workspace_probes` treats each listed repository inside a workspace folder as one more scope folder (probe, catalog roots, fallback areas). Repositories nested in a repository folder (submodules) are included too: same defect class
        - registry line: one line with complete totals. `spawn_startup` now prepares the known scopes, walks, records the repositories, prepares again (fresh scopes do no work), and only then sends `Internal::RegistryReady` followed by `Internal::FileList`; `elapsed_ms` is the sum of the two preparation steps and excludes the walk. The line still precedes `workspace files listed`. No request waits for the walk: document registries are prepared on the registry worker independently of startup. `StartupEvent::RegistryReady` now follows the walk, so `LspFixture::initialize_unlisted` no longer waits for it (a gated walk would hold it); `initialize` still sees it before `FileListInstalled`. Documented in `docs/lsp/indexing.md`
- class sweep: every site that re-prepares workspace scopes queues `RegistryJob::Workspace`, which runs `prepare_workspace_registries` and so `workspace_probes` with both extensions
        - startup (`spawn_startup`): **fixed**, both steps
        - server rescan (`rescan_workspace`): **fixed**; `relist` → `install_listing` records repositories and queues another workspace job when the set changed
        - configuration reload (`update_config`, client settings and watched `.dmls.toml`): **fixed**; the job queued before `relist` could read the old set, so `install_listing` queues again on a change
        - watched manifest / `.git` input and watched YAML (`apply_watched_changes`): **fixed** for fallback areas; for repositories, R1-F1.18 below
        - ignore-file change (`apply_watched_changes` → `relist`): **fixed** through `install_listing`
        - `didSave` of YAML and the schema-watch fallback: **fixed**, same job
        - R1-F1.18 (new sibling): a repository created during a session under a watched (non-rescan) client was found only by a later relist. `ServerState::note_repository_change` records the repository of any watched path inside a `.git` entry below a workspace folder (added when `.git` exists, removed when not) before the job is queued; `**/.git/config` is already a registered watch glob. **fixed**
- changes
        - `dmls/src/workspace/discover.rs`: `DiscoveryReport::repositories`, `walker` recorder, module doc
        - `dmls/src/overlay/mod.rs`: `OverlayState::workspace_repositories`, `set_workspace_repositories`, `note_workspace_repository`, `workspace_probes(roots, repositories, contexts)`, `fallback_areas`; `prepare_workspace_registries` docs
        - `dmls/src/router.rs`: `spawn_startup` order and docs, `StartupEvent::RegistryReady` doc, `install_listing`, `note_repository_change`, `apply_watched_changes`
        - tests: `tests/common/mod.rs` (`initialize_unlisted`, `initialize` docs); `tests/l1/repository_contexts.rs` `documents_in_one_repository_share_one_context_built_at_its_root` now counts from before startup (2 builds: the folder's and the nested repository's, which startup now prepares; the documents add none)
        - docs: `docs/lsp/indexing.md` (three-step startup, diagram, new "Which scopes are prepared", refresh paragraph, `elapsed_ms`, counting rule; the fallback-area gap removed), `.claude/skills/darkmatter/dmls.md`
- tests (`tests/l1/workspace_schema_scopes.rs`, each under 0.1 s)
        - R1-F1.16: `startup_prepares_a_fallback_package_area_with_no_catalog_entry` (monorepo `libs/one`, `libs/two`; `docs/schemas/broken.yaml`; `failed_triggers=1 failed_scopes=1`, 0 parses); refresh `a_watched_schema_creation_prepares_a_fallback_package_area`; control `first_level_directories_without_a_schema_root_add_no_scopes` (three extra `schemas`-less directories, including hidden `.claude`, add 0 `registry_scans`; `docs/schemas` adds exactly 1)
        - R1-F1.17: `startup_prepares_repositories_nested_in_a_non_repository_folder` (`roots=2 tree_schemas=2`, 0 parses); refresh `a_server_rescan_prepares_a_new_nested_repository`
        - R1-F1.18: `a_watched_git_config_creation_prepares_a_new_nested_repository`
        - unit: `workspace::discover::tests::the_walk_reports_each_directory_holding_a_git_entry` (`.git` dir and `.git` file; nothing under `.git` listed)
        - mutation check (reverted): disabling `fallback_areas` and the repository scopes fails all six new L1 tests
- commands
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/workspace_schema_scopes|startup|registry|repository_contexts|workspace_file_list|schema_roots/)'`: 106 passed (first run: 1 failed, `documents_in_one_repository_share_one_context_built_at_its_root`, rerun alone, fixed as above)
        - `cargo nextest run -p dmls --features effects-instrumentation` (dmls only, blast-radius check): 1096 passed, 6 skipped
        - `just lint` (darkmatter): clean
- implementation of 'R1-F1 follow-up (R1-F1.16, R1-F1.17, R1-F1.18)' complete, ready for review, at 20:18:15
- work completed for 'R1-F1 follow-up (R1-F1.16, R1-F1.17)' at 20:18:59
        - orchestrator review: one startup registry line with complete totals, emitted after the listing-discovered repositories are prepared and still before `workspace files listed`. Requests never wait for the walk

### Phase Gates (orchestrator)

- starting the phase gates at 20:18:59, with no subagent running
        - `just test` (darkmatter): 10,348 passed, 10 skipped, exit 0. Verifies every R1-F1 to R1-F5 test row and that nothing regressed across the library, CLI, dmls, and zed-dmls-cli
        - `just lint` (darkmatter): exit 0, including the zed-dmls wasm32-wasip2 check
        - `just check-tier-coverage darkmatter`: 0 stranded tests, exit 0. The new files `referenced_targets.rs`, `registry_preparation.rs`, and `workspace_schema_scopes.rs` are declared in `tests/l1/main.rs` and selected by L1
        - `just lint` and `just test` (biscuit-file): exit 0; 1,127 passed. Verifies the new `FileResolutionContext::repository_scope_catalog()` getter (R1-F1)
        - `just cross-check dmls --os windows`: 1,097 passed, 9 skipped, exit 0. Covers the path-sensitive work in R1-F1.16/17 and R1-F2.7/15 on native Windows
                - the rig published no CI observation because it tested an uncommitted tree; this is expected, since nothing is committed
        - no Linux, WSL2, L2, or L3 runs: no changed requirement needs a terminal or browser, and CI proves the other OS legs
- phase gates completed at 20:24:42

### Completion Report

- **outcome: complete.** Elapsed time was 1 h 41 min (18:43:53 to 20:24:42 local)
- parent findings: total 5 = fixed 5 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - R1-F1 fixed: 18 child rows, including sibling rows F1.6 to F1.18
        - R1-F2 fixed: 20 child rows, including sibling rows F2.10 to F2.20
        - R1-F3 fixed: 21 child rows, including sibling rows F3.13 to F3.21
        - R1-F4 fixed: 16 child rows, including sibling rows F4.9 to F4.16
        - R1-F5 fixed: 10 child rows, including sibling rows F5.8 to F5.10
- verified repairs
        - R1-F5: a trigger whose payload fails to load in any prepared scope now counts under `failed_triggers`, not `trigger_schemas`. Root-file classification stays passive
        - R1-F1: startup and every refresh prepare each workspace folder, every catalog package and package area inside it, monorepo fallback areas that have a `schemas` directory, and repositories nested in a non-repository folder. Every probe goes through `contexts.for_document`, so `SchemaRoots::for_document` is still the only source of the five roots. No unopened Markdown is parsed
        - R1-F3: the full index counts as complete only while every listed Markdown file has a current parse. After any of the five transitions, references, rename, file rename, and workspace symbols return the initial-build `RequestFailed` error naming the file. Reading the file again restores complete answers
        - R1-F2: a resolved Markdown target outside the file list becomes a graph dependency. It carries headings and transclusions, is invalidated on watched-file and save-fallback paths, and stays out of full-index answers. `::code` bytes are never parsed as Markdown
        - R1-F4: a registry lookup only reads installed state. Preparation runs on a background registry worker in three steps: copy the cache under a short lock, run without the lock, then install with a generation check. Requests wait on the shared `Waiter`. A held preparation still lets the server handle an edit, a cancellation, and shutdown
- open items: none of the review's required rows remain open
        - R1-F4.14 is a **deferred, nonblocking** sibling: `relist`, `recheck_parsed`, and `scan_manifests` still run on dispatch during rescans and configuration changes
                - it is not counted as a required child: the spec requires off-dispatch work only for startup registry preparation and file discovery, and no sentence requires rescan-time relisting off dispatch
                - next action: the follow-up pass can carry it to the backlog as a separate finding
- behavior changes to note for the next review
        - a request still waiting for its registry when `shutdown` arrives is answered `ServerCancelled`, and `stdio_subprocess::request_logs_go_to_stderr_and_never_into_the_stdout_protocol_stream` now accepts either frame
        - the startup `schema registry …` line is now emitted after the nested repositories found by the walk are prepared, and still before `workspace files listed`
        - `entry_point_parity::markdown_links_resolve_existing_targets_above_a_narrower_workspace_folder` now expects anchors to be checked above a narrower folder; it previously pinned the departure R1-F2 rejects
        - `repository_contexts` context-build counts went up, because workspace and nested-repository contexts are now built at startup
- no performance measurement was required or deferred
- changed files
        - biscuit-file: `lib/src/file_reference/context.rs` (R1-F1) and `docs/topics/file-references.md` (R1-F1)
        - dmls source
                - `src/overlay/registry.rs` (R1-F5, R1-F4)
                - `src/overlay/mod.rs` (R1-F1, R1-F4)
                - `src/registry_jobs.rs` (new; R1-F4)
                - `src/router.rs` (R1-F1, R1-F2, R1-F3, R1-F4)
                - `src/request_timing.rs`, `src/work_counters.rs`, and `src/lib.rs` (R1-F4)
                - `src/full_index.rs` (R1-F3, R1-F4)
                - `src/graph/{arena,invalidate,mod}.rs` (R1-F2, R1-F3)
                - `src/providers/{mod,completion,frontmatter,symbols,wiki}.rs` (R1-F2)
                - `src/workspace/discover.rs` (R1-F1)
        - dmls tests
                - new: `tests/l1/{referenced_targets,registry_preparation,workspace_schema_scopes}.rs`
                - changed: `tests/l1/{main,full_index,startup_log_lines,entry_point_parity,repository_contexts,stdio_subprocess}.rs` and `tests/common/mod.rs`
        - docs
                - `darkmatter/docs/lsp/indexing.md` and `darkmatter/docs/lsp/architecture.md`
                - `darkmatter/docs/topics/schemas/dmls-schema-support.md` and `darkmatter/dmls/docs/file-references.md`
                - `.claude/skills/darkmatter/dmls.md`

## Implementation of Review Findings #2

> **started at:** 2026-10-17T20:31:37-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/fixes/2026-10-17-logging-improvements/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the review has one finding (R2-F1) and two nonblocking observations
        - observations are not repaired here; the follow-up pass carries them to the backlog

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1   | Diagnostics tests use a response as a background-work barrier | fixed | all children fixed or clean; area `just test` 10349/10349 passed; 20× repeat of the racy tests: 0 failures |
| R2-F1.1 | File-list helper, external payload, no folder watches: observe the completed refresh before reading diagnostics (open and save) | fixed | `workspace_file_list::latest` now calls `flush_server_with("textDocument/documentSymbol", …)` (registry jobs drained plus a receiving round trip); `without_folder_watches_a_save_rechecks_external_schema_inputs` passed 20/20 |
| R2-F1.2 | Same helper, external payload with folder watches: synchronize the refresh before checking diagnostics and watch registration | fixed | same helper; `schema_inputs_outside_the_workspace_are_watched_by_folder` passed 20/20 |
| R2-F1.3 | Other file-list helper consumers (plain Markdown, no registry job): preserve synchronous path/parse-counter assertions | fixed | they share `latest`, so they get the same barrier; the round trip is still `documentSymbol` on an open document (no new parse, no full index); every parse-counter test in `workspace_file_list::` passed |
| R2-F1.4 | Referenced-target diagnostics helper: preserve lazy parse counts, no forced full index | fixed | the review called it clean, but it had the same response-only barrier, and the save-rescan path can queue preparation, so it was fixed: `referenced_targets::latest` now uses `flush_server_with(documentSymbol)`; all `referenced_targets::` sessions passed with unchanged parse counts |
| R2-F1.5 | Repository-context and entry-point parity readers: preserve background-aware barrier | clean | `repository_contexts::current_diagnostics` and `entry_point_parity` (line 324) call `flush_server()` before reading buffered diagnostics; `flush_server` now delegates to `flush_server_with("workspace/symbol")`, with the same loop as before; their tests passed |
| R2-F1.6 | LSP session diagnostic readers and strict-recovery drains: preserve explicit publication/idle synchronization | clean | `lsp_session` (lines 230, 815) and `strict_mode_recovery_spike` (lines 104, 138) call `flush_server()` before reading or draining; 164 `wait_for_diagnostics*` sites wait for the publication event itself; tests passed |
| R2-F1.7 | Held-registry negative assertion: keep proving absence while the worker is held | clean | the only `round_trip()` users (`registry_preparation.rs:220,280`) assert absence or wait while the gate is held; `round_trip` is documented as not waiting for registry jobs; tests passed |
| R2-F1.8 | Common fixture publication wait and registry-aware flush: reuse an existing primitive | fixed | `flush_server`'s loop was factored into `LspFixture::flush_server_with(method, params)` (tests/common/mod.rs), so callers can choose a round trip that does not build the full index; new ordering test `registry_preparation::a_flush_through_any_request_buffers_a_held_refresh` holds the job, shows a `documentSymbol` response arrives with nothing published, then shows the flush buffers the refresh |
| R2-F1.9 | Sibling (found by the late audit): `schema_roots_parity::a_forbidden_path_prefix_is_the_same_definition_error_in_md_and_dmls` read startup's unversioned publication for the envelope rather than the open buffer's | fixed | now loops until a versioned publication arrives, as `lsp_session`'s position-encoding test already did; test passes |
| R2-F1.10 | Sibling (found by the late audit): `lsp_session::trigger_payload_failure_retains_effective_schema_and_diagnoses_envelope` waits on an open envelope after save without first dropping a possible leftover publication | fixed | `flush_server()` plus a drain of the envelope's buffered publications before the save; test passes |
| R2-F1.11 | Sibling: the remaining `wait_for_diagnostics` readers in the other dmls test modules | clean | audited by a background subagent: each is the first wait for a newly opened uri, one wait per event, or flush and drain first. `suggest_constraint_phase1` double opens publish identical content, or publish once while the consumer's refresh is held |
| OBS-1   | Observation: rescans still perform synchronous disk work | nonblocking | not repaired in this cycle (observation) |
| OBS-2   | Observation: existing tests exceed the L1 time budget | nonblocking | not repaired in this cycle (observation) |

### Progress

- starting the work on 'R2-F1: Diagnostics tests use a response as a background-work barrier' at 20:32:10
        - defect class: a test treats a request's response as proof that background work queued earlier (registry jobs and the diagnostic refresh they hold) has finished and been published
        - the first subagent ended early, after adding `flush_server_with`, switching `workspace_file_list::latest` to it, and adding the gated ordering test; the orchestrator reviewed those changes and finished the sweep itself
        - sweep: every `take_buffered_diagnostics`, `round_trip`, and `wait_for_diagnostics*` site under `darkmatter/dmls/tests/` was checked (seven files)
        - found another instance: `referenced_targets::latest` used the same response-only barrier, so it was fixed in this cycle (R2-F1.4)
        - no sleeps, retries, or full-index builds were added, and server response ordering is unchanged
- a background audit of all `wait_for_diagnostics` sites, started by the first subagent, reported after the first closing pass. It found the stale-publication half of the class at two more sites (R2-F1.9, R2-F1.10); both were fixed in this cycle and the area gates were rerun
- work completed for 'R2-F1: Diagnostics tests use a response as a background-work barrier' at 20:41:10

### Completion Report

- outcome: **complete**; elapsed about 10 minutes (20:31:37 to 20:41)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - R2-F1 child rows: 7 fixed (F1.1–F1.4, F1.8, F1.9, F1.10) and 4 clean (F1.5–F1.7, F1.11)
- verified repair: both file-list and referenced-target diagnostic readers wait for outstanding trigger-registry jobs to finish, then make a receiving `documentSymbol` round trip, before reading buffered diagnostics
        - lazy parse counts are preserved
        - a gated L1 test shows that a response alone is not a barrier and that the flush is
- open items: none
        - OBS-1 (synchronous rescans) and OBS-2 (Level 1 tests over budget) are nonblocking observations, left for the follow-up pass
- no performance measurement was required or deferred
- no cross-OS check was run: the change is test-only synchronization, with no path or OS-specific code
- changed files
        - `darkmatter/dmls/tests/common/mod.rs`: adds `flush_server_with`, and `flush_server` delegates to it (R2-F1.8)
        - `darkmatter/dmls/tests/l1/workspace_file_list.rs`: `latest` barrier and its doc comment (R2-F1.1–F1.3)
        - `darkmatter/dmls/tests/l1/referenced_targets.rs`: `latest` barrier and its doc comment (R2-F1.4)
        - `darkmatter/dmls/tests/l1/registry_preparation.rs`: new test `a_flush_through_any_request_buffers_a_held_refresh` (R2-F1.8)
        - `darkmatter/dmls/tests/l1/schema_roots_parity.rs`: waits for the versioned publication (R2-F1.9)
        - `darkmatter/dmls/tests/l1/lsp_session.rs`: flush and drain before the save (R2-F1.10)
- verification
        - `cargo nextest run -p dmls --features effects-instrumentation --test l1 -E 'test(workspace_file_list::) | test(referenced_targets::) | test(registry_preparation::) | test(lsp_session::) | test(repository_contexts::) | test(entry_point_parity::) | test(strict_mode_recovery_spike::)'`: 206 passed (R2-F1.1–F1.8)
        - the two external-schema file-list tests plus the ordering test, run 20 times in a row: 0 failures (R2-F1.1, F1.2, F1.8)
        - the two R2-F1.9 and R2-F1.10 tests run on their own: 2 passed
        - `just test` in `darkmatter/`, rerun after R2-F1.9 and R2-F1.10: 10349 passed, 10 skipped, 0 failed (all)
        - `just lint` in `darkmatter/`, rerun after R2-F1.9 and R2-F1.10: exit 0
        - `just check-tier-coverage darkmatter`: 0 stranded tests
