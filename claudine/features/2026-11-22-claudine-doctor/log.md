---
implementation_1: "2026-11-24T02:16:10+00:00"
implementation_2: "2026-11-24T05:27:48+00:00"
implementation_3: "2026-11-24T07:26:50+00:00"
implementation_4: "2026-11-24T08:43:46+00:00"
---

# Claudine Doctor log

## Implementation of Review Findings #1

> **started at:** 2026-11-24T02:16:10+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- findings are handled serially, one subagent per finding; each subagent reports log items back to the orchestrator, which records them below

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1 | Permissions repair follows linked configuration directories | fixed | all four children verified; walk below the scope root at eligibility and again before the writer opens anything |
| R1-F1.1 | User scope: linked `.claudine` ancestor is a manual finding; config and sidecar untouched; ordinary directory still repairs | fixed | `permissions_tests::a_linked_claudine_folder_is_manual_and_neither_file_nor_sidecar_is_written` (user case): ordinary folder repairs; linked is `error`, not fixable, names `<home>/.claudine`; tree bytes and mtimes unchanged, no sidecar created; fails with the old final-component check |
| R1-F1.2 | Repository scope: linked parent refused before either write; inline policy kept; reference scope checked against target | fixed | same test (repository case) plus `a_folder_linked_after_planning_is_refused_before_the_writer_opens_anything` and `a_reference_that_would_not_name_the_planned_file_is_refused_before_any_write` |
| R1-F1.c1 | Direct link at `config.yaml` / sidecar in each scope stays a manual finding | clean | `a_linked_destination_or_lock_sidecar_is_manual_and_nothing_is_written` now covers both scopes and all four targets |
| R1-F1.c2 | Registration tree and legacy tree refuse linked `.claudine` | clean | `registration_tests::obstructions::a_linked_claudine_folder_holding_the_tree_and_the_retired_layout`: manual, link named, nothing written |
| R1-F2 | Doctor accepts changed registrations that Darkmatter refuses | fixed | all original child cases verified; shared Darkmatter verifier (`bundle::verify_read`, `registration::load_verified`) over doctor's no-follow reads; judged through `Installation::judge_manifest` with a required context |
| R1-F2.1 | Digest-correct invalid schema library member → `error`, exit 1 | fixed | `an_invalid_member_with_an_updated_digest_is_an_error::case_1_schema_library` → `error`; CLI exit 1 in `a_changed_bundle_darkmatter_refuses_is_an_error_in_both_formats_and_is_left_without_fix` |
| R1-F2.2 | Digest-correct invalid schema trigger member → `error`, exit 1 | fixed | `…::case_2_schema_trigger` → `error` |
| R1-F2.3 | Digest-correct invalid standalone YAML schema → `error`, exit 1 | fixed | `…::case_3_standalone_yaml_schema` → `error` |
| R1-F2.4 | Namespace schema change that breaks dependent lifecycle artifact → `error` | fixed | `…::case_4_namespace_schema` (invalid YAML) and `a_namespace_comment_with_an_updated_digest_is_an_error_since_the_lifecycle_artifact_disagrees` → `error` |
| R1-F2.5 | Lifecycle artifact replaced with `{}` and digest updated → `error` | fixed | `an_emptied_lifecycle_artifact_with_an_updated_digest_is_an_error` → `error`; artifacts are checked by the shared `profile_of` preparation |
| R1-F2.6 | Loadable member edit warns without alleging a digest mismatch | fixed | `a_loadable_byte_edit_with_an_updated_digest_warns_without_alleging_a_digest_mismatch` (4 cases) → `warn`, no digest/refusal text; `ExpectedFile::digest_mismatch` gates the note |
| R1-F2.c1 | Untouched / comment / host change → `ok` / `warn` / `error` controls kept | clean | `the_state_setup_leaves_passes` (ok), `manifest_edits_are_judged_by_whether_the_bundle_loads` (comment warn, `host: other` error) |
| R1-F2.c2 | Unchanged manifest with stale / missing member keeps warn / error | clean | `a_stale_member_is_a_fixable_warning_that_explains_the_digest`, `a_missing_member_is_a_fixable_error` |
| R1-F3 | Flat configuration entries hide genuine unknown keys | fixed | all children verified; flat entries validated in their keyed view with authored positions; migration finding kept separately |
| R1-F3.1 | User `actions` flat entry with typo: migration finding plus located `timout` | fixed | `doctor::config_tests::a_flat_user_action_keeps_its_migration_finding_and_locates_a_typo`: four forms; flat typo at line 5, column 7 |
| R1-F3.2 | Repository `actions` flat entry with typo: same, repository location | fixed | `a_flat_repository_action_keeps_its_migration_finding_and_locates_a_typo` with the repository file location |
| R1-F3.3 | User `messenger.configurations` flat route with typo: migration plus located typo | fixed | `a_flat_messenger_route_keeps_its_migration_finding_and_locates_a_typo`: migration at line 4, typo at line 6, column 7 |
| R1-F3.c1 | Repository `messenger` stays forbidden top-level key | clean | forbidden repository `messenger` check untouched; existing tests pass |
| R1-F3.c2 | TTS `pitch` and permissions `sockets` stay located | clean | `nested_unknown_keys_are_found_in_the_tts_mapping_the_imported_permissions_and_a_keyed_action` still locates `pitch` and `sockets` |
| R1-F4 | PowerShell startup writes during inspection | fixed | all children verified on build-win |
| R1-F4.1 | PowerShell 7 and Windows PowerShell probes leave startup/cache dirs unchanged; Windows proof without exclusions | fixed | Sniff `a_real_powershell_probe_answers_and_leaves_the_profile_unchanged` on build-win against PowerShell 7.6.10 and Windows PowerShell 5.1, whole-profile comparison with no exclusions, 1.4–1.6 s; red with the redirection disabled; CLI healthy-home `AppData` exclusion removed (2.56 s on Windows) |
| R1-F4.c1 | Bash-family probes keep startup files disabled | clean | Sniff argument test asserts `--noprofile --norc -p` and removed `BASH_ENV`/`ENV`; Git Bash and Cygwin snapshots unchanged on build-win |
| R1-F4.c2 | Process PATH lookup / absent shell start no child | clean | PATH check uses the executable index only; absent shell starts no child |
| R1-F5 | Failed cleanup can hide files already removed | fixed | all children verified; every removal goes through `remove_entry` and is recorded as it happens |
| R1-F5.1 | Legacy tree and manifest cleanup report partial removals; retry handles only remaining | fixed | `repair_tests::a_failed_retired_tree_root_removal_reports_what_went_and_a_retry_removes_only_the_rest`, `a_failed_retired_manifest_removal_still_reports_the_removed_tree`, doctor `a_read_only_claudine_folder_keeps_the_retired_root_and_the_report_lists_what_went` (review's shape); setup sibling `setup_command::setup_reports_a_partly_removed_retired_registration` |
| R1-F5.2 | Publisher temp cleanup records each deletion as it happens | fixed | `a_failed_temporary_file_removal_keeps_the_earlier_ones_in_the_journal` |
| R1-F5.3 | Publisher retirement records removed parent directories | fixed | `folders_emptied_by_retirement_are_reported_when_a_later_step_fails` |
| R1-F5.c1 | Publisher member replacement journal kept on later failure | clean | `a_failed_publication_withholds_the_register_edit_and_keeps_the_retired_layout` passes |
| R1-F5.c2 | Doctor extras and permissions transaction keep earlier entries | clean | `remove_extras` appends after each removal; permissions interruption tests pass |
| R1-F6 | CLI validation and repair-format tests miss required boundaries | fixed | all children verified; every spec acceptance state asserted in both formats with exit status through the CLI |
| R1-F6.1 | CLI duplicate-key and nested stray-key cases with file/line/column, files preserved | fixed | `a_duplicate_key_is_located_in_both_formats_and_changes_nothing`, `a_nested_stray_key_is_located_in_both_formats_and_changes_nothing` |
| R1-F6.2 | CLI changed-bundle refusal: unchanged files, exit 1 | fixed | `a_changed_bundle_darkmatter_refuses_is_an_error_in_both_formats_and_is_left_without_fix` (1.69 s) |
| R1-F6.3 | CLI human report for unresolved, repaired, failed repair | fixed | held-lock test asserts `Repair failed:` human and JSON (one home per format, 2.14 s) and a human `Repaired:` retry; unresolved-error human text in `a_repaired_home_exits_zero_and_a_second_run_changes_nothing` |
| R1-F6.4 | All-invalid permission rule array yields located error (in-process) | fixed | Darkmatter `config_fields.rs::an_all_invalid_shell_rule_array_locates_every_element_instead_of_granting_nothing` |
| R1-F6.c1 | Warning routing / plain / stdout failure evidence kept | clean | warning, forced-color, plain, `/dev/full` tests unchanged and passing |
| R1-F7 | Case-only aliases can be deleted as registration extras | fixed | all children verified on Linux (sensitive) and Windows (insensitive); macOS not run (rig lacks `just` on `PATH`), covered by the PR's macOS CI leg |
| R1-F7.1 | Case-only file rename kept on case-insensitive volume; Linux control preserved | fixed | `a_case_only_renamed_member_is_the_expected_file_…`: Linux rewrites/removes alias; Windows no operations; red on Windows with `one_entry` forced false |
| R1-F7.2 | Case-only folder rename treated as expected subtree | fixed | `a_case_only_renamed_folder_is_the_expected_subtree_…` on both OSes |
| R1-F7.3 | Publisher retirement uses on-disk membership | fixed | `a_previous_manifest_spelling_…`, `a_previous_manifest_folder_in_another_case_…` on both OSes |
| R1-F7.c1 | Publisher replacement / final verification use filesystem lookup | clean | existing repair and doctor tests pass on Linux and Windows; replacement and verification use filesystem lookup |
| R1-F8 | Configuration remedies promise behavior from commands that bypass configuration | fixed | both children verified; the original syntax and migration remedies name commands verified for those cases |
| R1-F8.1 | Doctor config remedy names a command confirmed to load config; docs/guidance updated | fixed | lib `every_config_remedy_names_a_command_that_reads_that_file`, `a_semantic_remedy_names_a_command_only_where_one_reports_the_rule`; CLI `the_legacy_remedy_is_claudine_setup_which_converts_the_file`, `the_user_file_remedy_names_claudine_actions_which_prints_the_located_error`, `the_repository_file_remedy_names_claudine_claude_which_stops_before_the_provider` |
| R1-F8.2 | (discovered) repository legacy `config.json` without YAML is reported with a confirmed remedy and left in place | fixed | lib `a_repository_legacy_file_is_reported_and_left_in_place_by_both_runs`, `a_legacy_file_at_a_repository_at_the_home_is_reported_once_as_the_users`; CLI `a_repository_legacy_file_is_left_in_place_and_claudine_claude_converts_it` |

### Progress
- starting the work on 'R1-F1 Permissions repair follows linked configuration directories' at 02:16:38
        - defect class: a write-safety check examines only a destination's final path component, so a linked ancestor directory redirects the write
        - `config_target_blockers(root, config)` now walks every component below the trusted scope root (home or repository root) for the config and its lock sidecar, reusing the publisher's no-follow `Layout::route`/`entry_at` (symlinks, junctions, reparse points)
        - `HandoffPlan` records its scope root; `HandoffPlan::obstruction()` decides fixability in `doctor/permissions.rs::assess` and is re-checked at the top of `commit_permissions_handoff` (doctor mode) before any lock or file is opened (`HandoffCommitError::Obstructed`)
        - doctor-mode commit refuses a reference that does not resolve within the scope root to the planned Claudine file (`HandoffCommitError::ReferenceOutOfScope`)
        - sibling sites: doctor/permissions.rs fixed; config/setup.rs commit fixed (doctor mode); registration inspect/repair callers fixed by signature change; publisher route, legacy.rs, read-only config readers clean
        - decision: setup mode keeps its existing behavior (writes through a linked `~/.claudine`) so dotfiles users are not broken; only doctor's automatic repair refuses
        - tests (lib, `#[cfg(unix)]`): `permissions_tests::a_linked_claudine_folder_is_manual_and_neither_file_nor_sidecar_is_written`, `a_folder_linked_after_planning_is_refused_before_the_writer_opens_anything`, `a_reference_that_would_not_name_the_planned_file_is_refused_before_any_write`, `a_linked_destination_or_lock_sidecar_is_manual_and_nothing_is_written` (both scopes, four targets), `registration_tests::obstructions::a_linked_claudine_folder_holding_the_tree_and_the_retired_layout`
        - regression proof: reverting the walk to final-component-only failed both new permissions tests
        - focused runs: lib doctor/registration/setup 151 passed; lib integration 95 passed; CLI doctor/setup 20 passed; `just lint` clean
        - docs: `claudine/docs/cli/doctor.md` permissions repair section covers linked folders and the lock file
        - limitation: no Windows junction end-to-end test (no cross-platform junction helper); relies on the shared reparse-point check's unit test
- work completed for 'R1-F1 Permissions repair follows linked configuration directories' at 02:28:36
- starting the work on 'R1-F2 Doctor accepts changed registrations that Darkmatter refuses' at 02:28:36
        - defect class: byte integrity (digest agreement) is used as proof that content can be loaded, so an internally consistent but unloadable bundle passes
        - new shared Darkmatter verifier over already-read bytes: `bundle::verify_read` (admission, member resolution, digests; shared with `load_snapshot`) and `registration::load_verified` (the same `profile_of` preparation as `load_entries`: member kinds, payloads, catalog, lifecycle/function artifacts)
        - Doctor's `manifest_load` keeps its no-follow regular-file reads, then calls the verifier; the source guard is unchanged because the new functions read no files
        - `ExpectedFile::digest_mismatch` gates the digest-refusal note in `disk_findings`, so a loadable byte-only edit warns without alleging a mismatch
        - sibling sites: inspect.rs fixed; doctor/registration.rs note fixed; repair's post-publish verification clean (byte-equality with the inventory the process already prepares; passes `None` with a reasoned comment); publisher retire decision clean (admission only decides membership); `entry_registered` clean (identity)
        - verified first that Darkmatter refuses the namespace-comment case (lifecycle artifact disagrees); the doctor note names `globals.lifecycle.json`
        - decision: with no launch context the manifest cannot be judged, so the row is `warn` with a note (was `error`); doctor may not build its own context under `context_construction_guard`
        - tests (lib `doctor/registration_tests.rs`): `an_invalid_member_with_an_updated_digest_is_an_error` (library, trigger, standalone, namespace cases), `a_namespace_comment_with_an_updated_digest_is_an_error_since_the_lifecycle_artifact_disagrees`, `an_emptied_lifecycle_artifact_with_an_updated_digest_is_an_error`, `a_loadable_byte_edit_with_an_updated_digest_warns_without_alleging_a_digest_mismatch` (4 cases), `manifest_edits_are_judged_by_whether_the_bundle_loads`; Darkmatter `host_registration_snapshot.rs` parity tests for `verify_read`
        - fixture helper `publish_with_edited_member` (public API only) for R1-F6.2
        - focused runs: claudine lib 192 passed; CLI doctor/registrations 28 passed (source guard included); darkmatter registration/bundle/snapshot 473 passed; freshness/embedded filters passed; `just lint` clean in claudine and darkmatter
        - docs: `claudine/docs/cli/doctor.md` (severity table, digest-mismatch note), `darkmatter/docs/topics/host-registration.md` ("Judging a bundle you have already read"), claudine and darkmatter repo skills
- work completed for 'R1-F2 Doctor accepts changed registrations that Darkmatter refuses' at 02:44:58
- starting the work on 'R1-F3 Flat configuration entries hide genuine unknown keys' at 02:44:58
        - defect class: a migration (legacy-form) diagnostic suppresses unrelated unknown-property evidence at the same location
        - `load.rs::project` validates a document with flat entries in its keyed form (`wire::upgrade_flat`) with the same scope validator, maps each problem inside a flat entry back to the authored key (drops the added kind segment), and adds the migration finding separately; the old below-entry `retain` filter is gone
        - `wire::flat_entries` returns `FlatEntry { segments, kind, message }` with `authored()` remapping
        - ordinary loads share the problem list, so `ConfigInvalid` now carries the typo beside the flat-form refusal
        - sibling sites: `project` filter fixed; `failure_problems` exact-duplicate dedup, agent-name conflict and exit-expression checks, `doctor/config.rs` kind split and stray-key dedup, `schema.rs`, `json_conversion`, JSON `is_old_format` all clean (they drop nothing but exact duplicates or run after schema success)
        - decision: validate the whole keyed view in one call (rejected: re-adding only unknown keys, which would still drop wrong-type problems)
        - tests: `doctor::config_tests::a_flat_user_action_keeps_its_migration_finding_and_locates_a_typo`, `a_flat_repository_action_keeps_its_migration_finding_and_locates_a_typo`, `a_flat_messenger_route_keeps_its_migration_finding_and_locates_a_typo`, `a_flat_entry_of_an_unknown_kind_or_with_a_wrong_type_is_still_judged`; `wire_tests::a_flat_entry_keeps_its_other_problems_beside_the_migration_message`; existing `pitch`/`sockets` and single-problem flat controls still pass
        - focused runs: claudine lib 742 passed; CLI doctor/config 238 passed; `just lint` clean
        - docs: `claudine/docs/topics/configuration.md`, `claudine/docs/cli/doctor.md` (stray-config-keys example), claudine repo skill
- work completed for 'R1-F3 Flat configuration entries hide genuine unknown keys' at 02:51:28
- starting the work on 'R1-F4 PowerShell startup writes during inspection' at 02:51:28
        - defect class: a read-only command starts a child process whose default startup writes to the user's profile or cache
        - Sniff's PowerShell probe now runs with `USERPROFILE`, `LOCALAPPDATA`, `APPDATA` pointed at a private temp folder (and `PSModuleAnalysisCachePath` inside it), deleted when the probe returns; `PATH` untouched; a folder that cannot be created is `SpawnFailed` with no shell started
        - build-win experiments: redirecting `LOCALAPPDATA` alone is not enough (the cache follows `USERPROFILE` through known folders); `DOTNET_MultiCoreJit=0` does not stop it; 5.1 writes only when its cache folder exists (it does in a real profile)
        - decision: "no side effects" is met as "nothing changes under the profile and nothing is left behind"; the probe's own writes land in OS temp and are removed (documented); spec left as the snapshot
        - decision: also set `POWERSHELL_TELEMETRY_OPTOUT=1` so a read-only probe makes no telemetry call
        - `tempfile` moved from a Sniff dev-dependency to a dependency (already in Sniff's graph through `gix-odb`)
        - sibling sites: only PowerShell needed the change; Bash family clean (`--noprofile --norc -p`, `BASH_ENV`/`ENV` removed; Git Bash and Cygwin snapshots unchanged on build-win; MSYS2 not installed); no other child process in doctor (TTS detection only looks up executables)
        - Windows proof `sniff windows_shells_tests::a_real_powershell_probe_answers_and_leaves_the_profile_unchanged` probes PowerShell 7.6.10 and Windows PowerShell 5.1 against a fresh profile holding both cache folders, checks a hit and a miss, compares the whole profile with no exclusions; 1.39–1.55 s; fails with the redirection disabled
        - CLI healthy-home test no longer excludes `AppData`; passed on Windows in 2.56 s
        - runs: sniff windows_shells 25 passed; claudine doctor 120 passed; CLI doctor 15 passed; lint clean; `just cross-check sniff --os windows windows_shells` 24 passed; `just cross-check claudine-cli --os windows doctor` 14 passed; `just cross-check claudine --os windows doctor` 110 passed
        - orchestrator corrected the CLI test module comment that still said each probe "costs seconds"
        - docs: `sniff/docs/topics/windows-shells.md`, `claudine/docs/cli/doctor.md`, dependency docs, `os` and `sniff` skills
- work completed for 'R1-F4 PowerShell startup writes during inspection' at 03:20:01
- starting the work on 'R1-F5 Failed cleanup can hide files already removed' at 03:20:01
        - defect class: a fallible batch of mutations reports its completed operations only on full success, so a caller loses evidence of partial mutation
        - every registration-module removal goes through one `remove_entry` with a test seam `fail_removals([paths])` that works on every OS
        - `remove_legacy_registration` returns `Result<LegacyCleanup, CleanupFailure { removed, error }>`; doctor's repair adds those removals to its journal before reporting failure; "No change was made before the failure" appears only when the journal is empty
        - `remove_temps` appends each deletion directly to `changes.removed_temps`; new `PublishChanges.removed_folders` records parents emptied by retirement
        - siblings fixed: setup's legacy cleanup (`ClaudineSetup.cleanup`, CLI `init_wizard.rs` reports removed entries and "only partly removed"); setup's publish (`SetupError::Publish { error, changes }`); `claudine registrations publish` (new `publish_recording`); misleading "so nothing was published" texts now say "the publish stopped before it"
        - siblings clean: repair verify and register edit are single operations; permissions two-target transaction already keeps earlier entries; `publish_with` test hook only
        - decision: failures and successes list the outermost removed entries (a removed folder stands for its contents) rather than every file, so success still names exactly the two legacy roots and large trees do not flood the report
        - tests: `repair_tests::a_failed_retired_tree_root_removal_reports_what_went_and_a_retry_removes_only_the_rest`, `a_failed_retired_manifest_removal_still_reports_the_removed_tree`, `a_failed_temporary_file_removal_keeps_the_earlier_ones_in_the_journal`, `folders_emptied_by_retirement_are_reported_when_a_later_step_fails`; `doctor::registration::tests::a_failed_retired_tree_removal_reports_the_removed_entries_and_a_retry_only_the_rest`, `a_read_only_claudine_folder_keeps_the_retired_root_and_the_report_lists_what_went` (Unix, the review's shape); `config::setup::tests::a_retired_layout_cleanup_that_fails_partway_reports_what_it_already_removed`, `a_publish_that_fails_partway_reports_what_it_already_changed`; CLI `setup_command::setup_reports_a_partly_removed_retired_registration`
        - runs: claudine lib 291 passed; CLI doctor/setup/init/registrations 239 passed (source guard included); `just lint` clean; slowest about 1.2 s
        - docs: `claudine/docs/cli/doctor.md`, `setup.md`, `registrations.md`, `topics/schemas.md`; claudine repo skill
- work completed for 'R1-F5 Failed cleanup can hide files already removed' at 03:37:59
- starting the work on 'R1-F6 CLI validation and repair-format tests miss required boundaries' at 03:37:59
        - defect class: acceptance cases stop testing just before the process boundary, or test only one of the two output formats
        - sweep of spec acceptance states through the CLI: warnings alone kept; unresolved errors, repaired errors, failed repairs, missing config, invalid config gained human-output and exit assertions; duplicate key, nested stray key, changed-bundle refusal gained new two-format CLI tests
        - new CLI tests (`cli/tests/l1/doctor.rs`): `a_duplicate_key_is_located_in_both_formats_and_changes_nothing` (0.08 s), `a_nested_stray_key_is_located_in_both_formats_and_changes_nothing` (0.13 s), `a_changed_bundle_darkmatter_refuses_is_an_error_in_both_formats_and_is_left_without_fix` (1.69 s, uses a CLI copy of `publish_with_edited_member` over the public `registrations_dir` locator)
        - held-lock failed-repair test now asserts both formats with one home per format, so both runs wait out the 2 s lock deadline in parallel (2.14 s); a human `--fix` retry after release shows `Repaired:` and exits 0; the `fail_removals` seam is `#[cfg(test)]` only, so no new production seam was added
        - Darkmatter `config_fields.rs::an_all_invalid_shell_rule_array_locates_every_element_instead_of_granting_nothing`: `[3, true]` locates `/permissions/shell-commands/0` and `/1` with line and column (0.02 s)
        - no product bug found; CLI and library agree
        - runs: CLI doctor 18 passed; darkmatter new test passed; `just lint` clean in claudine and darkmatter
- work completed for 'R1-F6 CLI validation and repair-format tests miss required boundaries' at 03:47:14
- starting the work on 'R1-F7 Case-only aliases can be deleted as registration extras' at 03:47:14
        - defect class: one decision identifies a disk entry through filesystem lookup while a following decision identifies it by case-sensitive string comparison, so they disagree on case-insensitive volumes
        - new `registration.rs::one_entry(a, b)`: two spellings are one entry when neither is a link (no-follow `entry_at`), both are regular files or folders, they share file identity (new public `sniff::filesystem::query::FileIdentity::of_path(path, false)`, no-follow), share a parent folder, and that folder does not list both names (keeps hard links distinct on Linux); an identity failure during the extras scan is a blocker, never a deletion
        - rejected: case-folding after a volume probe (misses APFS/NTFS Unicode folding), the `same-file` crate (follows reparse points; new dependency), identity alone (merges hard links)
        - sites fixed: `inspect.rs` `scan` (file and folder aliases; a folder alias is scanned under the inventory spelling), `repair.rs` `remove_extras` (through `scan`), publisher retirement in `Layout::preflight`, `remove_empty_parents` (safeguard), `mark_digest_mismatches` (finds a manifest entry listed in another case)
        - sites clean: publisher replacement, `holds`/`writable`, final verification, `manifest_load` (filesystem lookup); `remove_temps` (own prefix); `legacy.rs` (find and remove both by lookup; pinned by a test); doctor report, `config_target_blockers`, `entry_registered`
        - tests (`repair_tests.rs`, each probes the temp volume and asserts that branch's outcome): `a_case_only_renamed_member_is_the_expected_file_…`, `a_case_only_renamed_folder_is_the_expected_subtree_…`, `a_previous_manifest_spelling_…`, `a_previous_manifest_folder_in_another_case_…`, `a_digest_listed_under_another_case_…`, `the_retired_layout_is_found_and_removed_by_the_same_lookup_in_any_case`
        - Linux (ext4, sensitive control): 208 registration/doctor tests passed; Windows (NTFS, insensitive): `cross-check` 44 passed and the 6 new tests passed on the case-insensitive branch; red before the fix on Windows (5 of 6 failed with `one_entry` forced false)
        - macOS not run: bolt's `bash -lc` lacks `just` on `PATH` (known rig issue in the `os` skill); the PR's macOS CI leg supplies that evidence
        - orchestrator gated the Unix-only `permissions_tests::sidecar` helper with `#[cfg(unix)]` after the Windows run reported it dead (from R1-F1)
        - unrelated: `cargo clippy -p claudine --all-targets --features work-counters` reports `result_large_err` in untouched `lib/tests/l1/invalid_trigger_findings.rs`; `just lint` does not run that combination
        - docs: `claudine/docs/cli/doctor.md`, `claudine/docs/topics/schemas.md`, `sniff/docs/topics/filesystem-query.md`; `os` and `claudine` skills
- work completed for 'R1-F7 Case-only aliases can be deleted as registration extras' at 04:14:39
- starting the work on 'R1-F8 Configuration remedies promise behavior from commands that bypass configuration' at 04:14:39
        - defect class: a remedy promises behavior from a class of commands, some of which bypass the code path that provides it
        - observed in temp homes: user legacy JSON is converted by `claudine actions`, `claudine hooks`, and `claudine setup`; user invalid YAML / bad values are printed located (exit 1) by `actions`, `hooks`, `setup`; a repository file is read only by provider wrappers such as `claudine claude` (not `actions`, `setup`, `skills`, `claude --dry-run`, or `goose`); a repository-only merged rule break is reported by no command
        - remedies now: legacy user JSON → `claudine setup`; user file error → `claudine actions` prints the full message; repository file error → `claudine claude` run inside the repository stops with the full message; repository-scope rule → "correct the setting" only
        - sibling sweep: all four `md` remedies (`config.rs`, three in `permissions.rs`) now say "`md compose` of a document that runs a shell command prints the full message" (true in every case observed); Claude's-permissions-unusable remedy no longer promises `md` prints it; missing-file `claudine setup` remedies, `path` and registration remedies clean
        - tests: lib `config_tests::every_config_remedy_names_a_command_that_reads_that_file` (also forbids "any other"/"any ordinary"/"`md` prints"), `a_semantic_remedy_names_a_command_only_where_one_reports_the_rule`; CLI `the_legacy_remedy_is_claudine_setup_which_converts_the_file`, `the_user_file_remedy_names_claudine_actions_which_prints_the_located_error`, `the_repository_file_remedy_names_claudine_claude_which_stops_before_the_provider` (fake provider; asserts no launch)
        - runs: lib doctor 124 passed; CLI doctor/remedy 21 passed (new ones 0.06–0.15 s); `just lint` clean
        - docs: `claudine/docs/cli/doctor.md` (new "Where the full message is printed" subsection), `claudine/docs/topics/configuration.md`; caller and repo skills had no such promise, so `just publish-skills` was not needed
        - discovered sibling: doctor never reports a repository-level legacy `config.json`; repaired in this cycle (see R1-F8.2)
        - discovered, outside this feature's spec (runtime behavior of other commands, recorded for the author): a repository-only rule break is ignored at runtime; `claudine goose` does not read the repository config; `claudine opencode` failed in the probe without showing the YAML error
- starting the work on 'R1-F8.2 repository legacy config.json is not reported' at 04:30:28
        - observed in a temp repo: `claudine setup`, `actions`, `skills`, `agents`, `claude --dry-run`, and an interactive `claudine claude` do not convert a repository legacy file; `claudine claude hi` converts it (or stops with the reason, exit 1, before the provider); `skills --fix` converts only as a side effect of link fixes and is not named
        - `doctor/config.rs` now detects a repository legacy `config.json`; `config` reports it as `warn` with the remedy "`claudine claude` with a prompt, run inside the repository, converts it before the provider starts" and the note that doctor leaves it as it is; `repo-config` stays `info` and says a legacy file awaits conversion; a repository at the home is reported once as the user's
        - decision: `warn` (user legacy stays `error` because the user file is required; a missing repository file is `ok`/`info`, so a legacy one is a step up); alternatives `info` (hides an ignored config) and `error` (fails runs over an optional file)
        - tests: lib `a_repository_legacy_file_is_reported_and_left_in_place_by_both_runs`, `a_legacy_file_at_a_repository_at_the_home_is_reported_once_as_the_users`, extended `every_config_remedy_names_a_command_that_reads_that_file`; CLI `a_repository_legacy_file_is_left_in_place_and_claudine_claude_converts_it` (0.29 s)
        - runs: lib doctor 126 passed; CLI doctor 22 passed; `just lint` clean
        - docs: `claudine/docs/cli/doctor.md` (config table, legacy table, repo-config)
- work completed for 'R1-F8 Configuration remedies promise behavior from commands that bypass configuration' at 04:36:32
- starting final verification gates at 04:36:32
        - first claudine `just test` failed `context_construction_guard::production_source_builds_contexts_only_through_the_builder`: R1-F2 had added an `Option<&FileResolutionContext>` parameter to `inspect_installation`, and that gate takes no allowlist
                - fix: `inspect_installation(home, config, inventory)` keeps a changed manifest's bytes; the new `Installation::judge_manifest(home, &FileResolutionContext)` runs the shared verifier, and doctor calls it only when its context resolves; repair never judges (byte equality with the inventory)
        - second run failed `test_placement::repository_test_placement`: `composition/registration.rs` had grown to 966 production lines (limit 800)
                - fix: moved the no-follow entry helpers (`Entry`, `obstruction_of`, `attributes`, `entry_at`, `one_entry`) to `registration/entries.rs`, the publisher lock to `registration/lock.rs`, and the removals (`remove_entry`, the `fail_removals` seam, `remove_temps`, `remove_empty_parents`) to `registration/removal.rs`; no behavior change
        - Windows cross-check reported unused `CleanupError`/`Obstruction` imports in `lib/tests/l1/installed_registration.rs` (only a `#[cfg(unix)]` test uses them; the import line was already there); gated the import with `#[cfg(unix)]`
        - Windows cross-check also reports unused `Duration`/`Instant` imports in `cli/tests/l1/lifecycle_edit_action.rs`, a file this work did not touch; recorded as unrelated and left as it was
- final verification completed at 05:15:15

### Completion Report

**Outcome: complete.** Elapsed: about 2 h 59 min (02:16:10 to 05:15:15 UTC).

Parent findings: **8 total = 8 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed.** Every child row is fixed or clean with evidence. One discovered sibling row, R1-F8.2, was added and fixed. No row is open.

#### Verified repairs

- **R1-F1:** the permissions repair walks every path component below the home or repository root, for both config files and their lock sidecars. It does this when deciding fixability and again before the writer opens anything. A linked `.claudine` is a manual finding, and nothing is written.
- **R1-F2:** after Doctor admits the member paths, a changed manifest goes through Darkmatter’s shared verification and preparation. The tested loadable byte-only edits warn without claiming a digest mismatch. The preceding path admission is stricter than Darkmatter’s installed reader, as recorded below.
- **R1-F3:** a flat (legacy) entry is validated in its keyed form. Its other problems are reported at the authored key positions, beside the migration finding.
- **R1-F4:** PowerShell probes run with their profile and application-data folders pointed at a private temp folder, which is removed afterwards. Verified on build-win against PowerShell 7.6.10 and Windows PowerShell 5.1, with no exclusions in the comparison.
- **R1-F5:** every registration removal is recorded as it happens. Failed cleanup, setup, and publish now report the changes already made.
- **R1-F6:** CLI tests cover duplicate and nested keys, a changed bundle Darkmatter refuses, and human output for unresolved, repaired, and failed repairs. An in-process Darkmatter test covers the all-invalid shell-rule array.
- **R1-F7:** membership is decided by directory-entry identity, without following links. A case-only alias is never deleted on a case-insensitive volume; Linux behavior is unchanged.
- **R1-F8:** the verified syntax and legacy migration remedies name commands that handle those cases. Doctor also reports a repository’s legacy `config.json`. The additional command categories and scopes exercised by review 2 exposed gaps, recorded below.

#### Decisions recorded (details in the progress entries)

- `claudine setup` keeps writing through a linked `~/.claudine`; only doctor's automatic repair refuses (R1-F1).
- With no launch context, a changed manifest is a `warn` because it cannot be judged (R1-F2).
- The PowerShell probe writes its cache to an OS-temp folder that it then removes. "No side effects" therefore means nothing changes under the profile and nothing is left behind (R1-F4).
- Cleanup reports list the outermost removed entries, not every file inside them (R1-F5).
- A repository legacy `config.json` is a `warn` (R1-F8.2).

#### Observations outside this feature's scope (behavior of other commands, for the author)

- A repository-only rule break is silently ignored at runtime.
- `claudine goose` does not read the repository config.
- An interactive `claudine claude` launch, with no prompt, does not convert a repository legacy `config.json`.
- Windows warns about unused imports in `cli/tests/l1/lifecycle_edit_action.rs`, a file this work did not touch.
- `cargo clippy -p claudine --all-targets --features work-counters` reports `result_large_err` in the untouched `lib/tests/l1/invalid_trigger_findings.rs`. `just lint` does not run that combination.

#### Final verification

| Command | Result | Verifies |
| --- | --- | --- |
| `just test` (claudine) | 12897 passed, 16 skipped, none at or above 5 s | all findings |
| `just lint` (claudine) | exit 0, no warnings | all findings |
| `just check-tier-coverage claudine` | 0 stranded | new tests' tiers |
| `just test` (darkmatter) | 17862 passed, 10 skipped, none at or above 5 s | R1-F2, R1-F6.4 |
| `just lint` (darkmatter) / `just check-tier-coverage darkmatter` | exit 0 / 0 stranded | R1-F2, R1-F6.4 |
| `just test` (sniff) | 3904 passed, 38 skipped, none at or above 5 s | R1-F4, R1-F7 |
| `just lint` (sniff) / `just check-tier-coverage sniff` | exit 0 / 0 stranded | R1-F4, R1-F7 |
| `scripts/cross-check.sh --os windows claudine registration doctor setup config` | 835 passed | R1-F1, R1-F2, R1-F3, R1-F5, R1-F7, R1-F8 |
| `scripts/cross-check.sh --os windows claudine-cli doctor setup_command registrations test_placement` | 52 passed | R1-F4, R1-F5, R1-F6, R1-F8 |
| `scripts/cross-check.sh --os windows sniff windows_shells identity` | 75 passed | R1-F4, R1-F7 |
| macOS cross-check (R1-F7) | not run: bolt's `bash -lc` lacks `just` on `PATH` (known rig issue) | the pull request's macOS CI leg supplies it |

The Windows runs were filtered to the affected tests. They are targeted evidence, not a full-suite observation. `os-failing` was not set, because no Windows test failed.

#### Changed files

- Claudine library:
        - `lib/src/composition/registration.rs`, plus new `registration/entries.rs`, `registration/lock.rs`, `registration/removal.rs`
        - `registration/inspect.rs`, `registration/legacy.rs`, `registration/repair.rs`, `registration/repair_tests.rs`
        - `lib/src/config/load.rs`, `config/setup.rs`, `config/wire.rs`, `config/wire_tests.rs`
        - `lib/src/doctor/config.rs`, `config_tests.rs`, `path.rs`, `permissions.rs`, `permissions_tests.rs`, `registration.rs`, `registration_tests.rs`
        - `lib/tests/l1/installed_registration.rs`
- Claudine CLI: `cli/src/commands/init_wizard.rs`, `cli/src/commands/registrations.rs`, `cli/tests/l1/doctor.rs`, `cli/tests/l1/setup_command.rs`
- Claudine docs: `docs/cli/doctor.md`, `docs/cli/registrations.md`, `docs/cli/setup.md`, `docs/topics/configuration.md`, `docs/topics/schemas.md`
- Darkmatter: `lib/src/config/registration.rs`, `lib/src/config/registration/bundle.rs`, `lib/tests/l1/config_fields.rs`, `lib/tests/l1/host_registration_snapshot.rs`, `docs/topics/host-registration.md`
- Sniff: `lib/Cargo.toml`, `lib/src/filesystem/query/identity.rs`, `lib/src/programs/windows_shells.rs`, `lib/src/programs/windows_shells_tests.rs`, `docs/topics/filesystem-query.md`, `docs/topics/windows-shells.md`, `docs/dependencies.md`
- Repository: `docs/dependencies.md`; skills `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/SKILL.md`, `.claude/skills/os/SKILL.md`, `.claude/skills/os/windows.md`, `.claude/skills/sniff/programs.md`

### Review 2 verification (2026-11-24)

- The completion report previously stated that a changed manifest went through
  Darkmatter's verifier without mentioning Doctor's earlier admission gate.
  In the same temporary published bundle, changing a file reference and its
  digest key from `./schemas/action.yaml` to
  `../darkmatter/schemas/action.yaml` made `claudine doctor --json` exit 1.
  Darkmatter loaded that bundle in `md schema validate`. Both refused a
  reference that resolved outside the managed directory.
- The completion report previously generalized the syntax and migration remedy
  proofs to every remedy. In temporary fixtures, `claudine actions` printed
  an invalid sound-setting error, while it exited 0 without the mapper error
  that Doctor said it would print. `claudine setup` completed the user
  handoff and left the repository handoff unchanged. `md init --inline`
  replaced a broken user reference and left the same repository case
  unchanged. `md compose` printed invalid YAML and permission errors, while
  its shell-document run on `register: 3` reached shell approval without a
  registration diagnostic.

## Implementation of Review Findings #2

> **started at:** 2026-11-24T05:27:48+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- findings are handled serially, one foreground subagent per finding; each subagent updates this log as it works

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1 | A failed user permissions repair skips the repository repair | fixed | `repair_permissions` continues past a failed scope, scope-prefixed reasons, completed changes kept; red check (early `break` restored) fails the new test; `cargo nextest run -p claudine --lib -E 'test(/doctor::/)'` 128/128 |
| R2-F1.1 | User lock held: user stays inline with failure reported; repository handoff completes in same invocation | fixed | `doctor::permissions::tests::a_failed_scope_does_not_stop_the_other_scopes_handoff` (user lock, 50 ms `lock_deadline` seam, 0.21 s): `error` with `user scope: …`, user inline kept, repository two writes listed; retry ok |
| R2-F1.c1 | Repository lock held: user completes, repository inline kept, retry completes | clean | same test, repository-lock case: user handoff completes, repo inline kept, retry completes |
| R2-F1.c2 | Both scopes available: exit 0, four replacements | clean | `both_scopes_are_handed_off_together_when_nothing_blocks_them`: ok, four `Repaired:` lines |
| R2-F1.c3 | Registration callback failure does not block permissions callback | clean | `mod.rs` runner unchanged and continues across callbacks; `doctor::tests::a_failed_repair_with_no_change_says_so_and_unrelated_repairs_still_run` passes |
| R2-F1.c4 | Ordered writes within one handoff/publication stop dependent writes | clean | `an_interrupted_transfer_keeps_the_inline_block_and_a_retry_completes_it` and registration interruption tests pass; one scope's two writes still stop together |
| R2-F2 | Doctor refuses loadable manifest member references | fixed | new Darkmatter `bundle::admit_installed_manifest` (confined installed-member resolution, no reads) used by `Installation::judge_manifest` for load verdict and digest notes; red check (lexical admission restored) fails re-entry cases |
| R2-F2.1 | Schema library `action.yaml`: in-tree re-entry spelling warns; escape errors | fixed | `doctor::registration` `schema_library_spelling::{the_canonical_spelling_beside_a_manifest_comment_warns, a_spelling_that_reenters_the_bundle_warns, a_spelling_that_leaves_the_bundle_is_an_error}` |
| R2-F2.2 | Schema trigger `composition.yaml`: same | fixed | `schema_trigger_spelling::*` same three verdicts |
| R2-F2.3 | Standalone YAML kind `formal-sequence-file.yaml`: same | fixed | `standalone_yaml_schema_spelling::*` same three verdicts |
| R2-F2.4 | Namespace schema `group.yaml`: same | fixed | `namespace_schema_spelling::*` same three verdicts |
| R2-F2.5 | Lifecycle artifact `globals.lifecycle.json` and function-pair category: same | fixed | `lifecycle_artifact_spelling::*`; Darkmatter `installed_admission_resolves_every_member_category_as_the_reader_does` covers lifecycle, function catalog, facts re-entry and escape |
| R2-F2.6 | Digest-note attribution uses same confined resolution for both spellings | fixed | `a_member_that_disagrees_with_its_digest_is_attributed_under_either_spelling::{canonical, reentry}` both report the digest mismatch |
| R2-F2.c1 | Config `register` identity and post-publish verification | clean | `entry_registered` (Darkmatter `manifest_identity`) and `repair.rs` byte verification unchanged; `an_equivalent_spelling_of_the_entry_passes`, repair tests, `installed_registration` 56/56 pass |
| R2-F2.c2 | Retirement keeps conservative lexical admission | clean | `previous_member_paths` still uses lexical `bundle::admit_manifest`; Darkmatter `admission_refuses_a_path_that_climbs_out_and_back_in` passes |
| R2-F2.d1 | Claudine repo skill's stale `inspect_installation(…, Some(context))` description | fixed | `.claude/skills/claudine/SKILL.md` describes `inspect_installation` then `Installation::judge_manifest(home, context)` + `admit_installed_manifest`; Darkmatter skill and `host-registration.md` updated |
| R2-F3 | Merged configuration errors can name the wrong file | fixed | `ClaudineConfig::validate_settings` returns the failing `ValidatedSetting`; doctor `source_of` follows `merge_repo_override`; `ConfigObservations::superseded_user_problem` feeds `tts` and the user handoff; forcing user attribution fails six config tests; `cargo nextest run -p claudine --lib -E 'test(/doctor::/) | test(/config::/) | test(/messag/)'` 787/787 |
| R2-F3.1 | Effective messenger selection: failing repo override names repo selection + user route catalog | fixed | `config_tests::a_failing_repository_selection_names_the_repository_selection_and_the_user_routes`: repo `team` passes; missing repo route → error at repo file `active_messenger` with note naming user `messenger.configurations`, no `claudine actions` remedy |
| R2-F3.2 | Effective action mapper: bad repo `after_tool` names repository file | fixed | `an_action_mapper_failure_names_the_file_that_authors_the_event`: bad repo `after_tool` at repo file; valid replacement passes; unlisted user event stays user's |
| R2-F3.5 | Effective exit expressions: name file whose effective setting failed | fixed | `an_exit_expression_failure_names_the_file_whose_declaration_is_effective`: valid replacement passes; unknown-agent scope at repo file; valid-user control names repo |
| R2-F3.3 | Speech prerequisite: explain user-only reason in `tts` row when absent from `config` | fixed | `tts_tests::a_user_only_problem_the_repository_replaces_is_explained_in_the_speech_row`: `info`, user file, names the setting without value; outside repository still defers to `config` |
| R2-F3.4 | Handoff prerequisite: explain which user-only setting blocks it | fixed | `permissions_tests::a_user_only_rule_the_repository_replaces_blocks_the_handoff_and_is_explained_there`: refused, nothing written, note names the setting |
| R2-F3.c1 | Repository scope with otherwise valid user config names both files | clean | `a_repository_selection_with_no_user_route_names_both_files`, `a_valid_repository_selection_of_a_defined_user_route_passes` |
| R2-F3.c2 | User-only semantic settings keep user attribution | clean | `user_only_settings_keep_the_user_attribution_beside_any_override` (also when merged check stops at the repo selection first) |
| R2-F3.c3 | Schema-invalid files and stray-key projection keep file identity | clean | schema/stray-key code untouched; invalid-repo, nested-typo, flat-entry tests pass unchanged |
| R2-F4 | The speech report confuses highest quality with runtime selection | fixed | automatic and fallback summaries say "best available … tries X first"; "uses" only when best == first; `cargo nextest run -p claudine --lib -E 'test(/doctor::tts/)'` 16/16 |
| R2-F4.1 | Enabled automatic, Good+: "best available" Piper, eSpeak NG "tries first" | fixed | `doctor::tts::tests::a_preferred_first_provider_changes_what_is_tried_first_but_not_the_best`; updated `automatic_selection_reports_the_highest_rated_and_the_runtime_first_candidate` (no "uses Kokoro") |
| R2-F4.2 | Unrecognized provider fallback detail distinguishes best vs first | fixed | `an_unrecognized_provider_falls_back_and_separates_the_best_from_the_first`: warning leads, detail names best and first, never "uses Piper" |
| R2-F4.c1 | Enabled automatic below Good keeps both descriptions | clean | `below_good_automatic_selection_warns_and_names_the_best_and_the_first` |
| R2-F4.c2 | Pinned provider names only the pin | clean | `a_pinned_provider_is_named_alone_whatever_runtime_order_prefers`; `a_missing_pinned_provider_is_an_error_despite_an_available_fallback` |
| R2-F4.c3 | Disabled and no-candidate branches unchanged | clean | `disabled_speech_claims_no_runtime_selection`, `disabled_hook_speech_is_informational_and_names_lifecycle_say`; no-candidate tests unchanged |
| R2-F5 | Some remedies cannot act on the reported problem | fixed | every `.remedy(` in `lib/src/doctor/` swept and chosen by category and scope (verified against built `claudine`/`md` in temp homes); `cargo nextest run -p claudine --lib -E 'test(/doctor::/)'` 163/163, `-p claudine-cli --features test-fixtures -E 'test(/doctor/)'` 24/24 |
| R2-F5.1 | User mapper semantic error: name event/action/setting, omit `actions` command | fixed | `a_user_mapper_remedy_names_the_setting_and_no_command`; CLI `claudine_actions_reports_a_broken_rule_but_not_a_broken_mapper` (sound exits 1, mapper exits 0); superseded mapper: `a_superseded_user_mapper_is_corrected_in_place_without_a_command` |
| R2-F5.2 | Darkmatter `register` finding: give correction using Doctor's location | fixed | `a_darkmatter_remedy_names_md_compose_only_for_problems_a_compose_prints` (user and repository): `/register` gets a direct correction; YAML, inline permissions, broken reference keep `md` |
| R2-F5.3 | Repository handoff remedy: offer Doctor's fix, not `setup` | fixed | `setup_is_offered_only_for_a_user_handoff_it_performs`; CLI `a_repository_handoff_is_left_by_setup_and_repaired_by_doctor_fix` |
| R2-F5.4 | Broken repository permissions reference: recommend correcting the reference/target | fixed | `a_broken_repository_reference_is_corrected_without_a_user_scoped_command`; user control keeps both `md` suggestions; garbled `UnusableMapping` remedy also corrected |
| R2-F5.c1 | Syntax, legacy, and missing-file remedies preserved | clean | original syntax, legacy, missing-file CLI cases pass; setup still creates missing files |
| R2-F5.c2 | Manual conflicts / repo-merge mapper errors keep generic guidance | clean | conflict, manual-mapping, repository-merge remedies keep direct guidance (`a_mapping_that_fails_the_scoped_rules_is_manual`) |
| R2-F5.d1 | Doctor reference doc + source comment "reads no configuration" corrected | fixed | `docs/cli/doctor.md` and the shared-wording comment in `doctor/config.rs` no longer claim Doctor reads no configuration; `main.rs`/`commands/doctor.rs` only say it runs before configuration loads (true) |
| R2-F5.x1 | Discovered during R2-F5 sweep: setup/handoff planning treats an explicit `permissions: null` in `.darkmatter.yaml` as a transferable block (moves `null` into Claudine's file and leaves a broken reference, or reports differing blocks) | fixed | `config/setup.rs::decide_for_setup` reads `permissions: null` as no block (Darkmatter's loader applies the default policy to null), in both files; `darkmatter_setup` `null_handoff_cases!` five `setup_null_*` cases failed before, pass now; doctor's R2-F5 null special case removed, `setup_is_offered_only_for_a_user_handoff_it_performs` updated |
| R2-F6 | Flat entries hide independent configuration diagnostics | fixed | `config/load.rs::project` collects schema and migration problems and always runs both document rules; keyed-branch schema refusal sibling fixed too; old early returns fail 11 of 20 `independent_rule_cases!` rows; `cargo nextest run -p claudine --lib -E 'test(/config::/) | test(/doctor::/)'` 622/622 |
| R2-F6.1 | User actions flat form keeps preferred-agent / exit-expression diagnostics | fixed | `doctor::config::tests` `independent_rule_cases!` user keyed/flat rows: migration at `/actions/session_start/0` plus `/favorite_agent` (2:17), `/exit_expressions/0`, `/exit_expressions/rules/0` (3:7) |
| R2-F6.2 | Repository actions flat form keeps exit-expression diagnostic | fixed | repository keyed/flat rows with both, neither, and `rules` patterns |
| R2-F6.3 | User messenger routes flat form keeps both diagnostics | fixed | user keyed/flat Slack route rows: migration at `/messenger/configurations/team` plus the agent and `rules` pointers |
| R2-F6.c1 | Nested schema diagnostics and wrong scope unchanged | clean | existing flat typo/wrong-type and repository `messenger` refusal tests pass in the 622-test run |
| R2-F6.c2 | Legacy JSON migration still validates candidate | clean | `config_json_conversion::a_converted_candidate_is_judged_by_the_document_rules` and `malformed_legacy_files_are_refused_and_kept`: refused, nothing written |
| R2-F7 | Permissions repair is offered for an invalid Darkmatter file | fixed | a handoff that would edit a `.darkmatter.yaml` with `ConfigFile::registration_findings()` is a manual `error` pointing at `config`; writer keeps its own validation; 7 regression tests fail with the check off; `cargo nextest run -p claudine --lib -E 'test(/doctor::/) | test(/config::setup/)'` 202/202 |
| R2-F7.1 | User handoff: invalid `register` shapes make handoff manual; valid controls repairable | fixed | `doctor::permissions::tests` `register_shape_cases!` user rows: omitted/null/empty/valid repair to `ok`; scalar/mixed/all-invalid not fixable, `--fix` leaves tree byte- and mtime-identical, no sidecar |
| R2-F7.2 | Repository handoff: same matrix, independent of user work | fixed | `register_shape_cases!` repository rows name the repository file; `an_invalid_repository_register_leaves_the_user_handoff_repairable` |
| R2-F7.c1 | Registration repair preflight unchanged | clean | `doctor::registration::tests::only_a_valid_register_setting_admits_the_entry_repair`: omitted/null/`[]` repaired; `3`/`[3]` blocked, nothing written |
| R2-F7.c2 | Already-referenced permissions keep independent verdict | clean | `an_existing_reference_keeps_its_verdict_beside_an_invalid_register` (both scopes): `ok`, no edit |
| R2-F8 | Darkmatter suggests user initialization for broken repository references | fixed | all six broken-target branches in `darkmatter/lib/src/config/load/reference.rs` use one `Source::remedy(fix)` keyed on the declaring `ConfigRevision`; `darkmatter/docs/topics/configuration.md` corrected; `cargo nextest run -p darkmatter -E 'test(/reference/) | test(/permission/) | test(/config/) | test(/init/)'` 1464 passed |
| R2-F8.1 | Missing target: user keeps `md init --inline`; repository gets manual scoped correction | fixed | `config_permissions::missing_target_remedy_{user,worktree,head}` |
| R2-F8.2 | Invalid target YAML: same scope distinction | fixed | `invalid_yaml_target_remedy_{user,worktree,head}` |
| R2-F8.3 | Chained reference: same | fixed | `chained_target_remedy_{user,worktree,head}`; chained diagnosis kept |
| R2-F8.4 | Nonmapping / wrong block type / missing block: same | fixed | `nonmapping_target_remedy_*`, `wrong_block_type_remedy_*`, `missing_block_remedy_*`; CLI `config_notices::a_broken_repository_reference_names_a_direct_correction_not_md_init` |
| R2-F8.5 | Committed repository policy remedy explains commit boundary | fixed | `*_remedy_head` cases: HEAD errors say the correction must be committed |
| R2-F8.c1 | Valid mapping emits no initializer remedy | clean | `a_valid_referenced_mapping_reports_no_initializer_in_either_scope`; equivalence `an_equivalent_reference_spelling_keeps_the_remedy` (`./policy.yaml` vs `./sub/../policy.yaml`) |

### Progress

- starting the work on 'R2-F1' at 05:28:52
        - defect class: early termination of an independent repair batch; `repair_permissions` returned at the first failed scope, so a held user writer lock left the independent repository handoff undone
        - sibling sites swept:
                - `lib/src/doctor/permissions.rs::repair_permissions`: defective (scopes are independent); fixed
                - `lib/src/doctor/mod.rs` repair runner: clean; every fixable row's callback runs regardless of earlier failures
                - `lib/src/doctor/registration.rs::repair_registration` / `composition/registration/repair.rs`: clean; one ordered pipeline (publish, extras, verify, entry, verify, legacy cleanup) whose later steps depend on earlier ones, so stopping is correct
                - `lib/src/config/setup.rs::setup_claudine`: clean; a handshake failure does not stop the registration entry, and legacy cleanup is gated on the entry by design
                - `lib/src/config/setup.rs::commit_permissions_handoff`: clean; one two-file transaction, ordered writes within one scope
        - changes: `repair_permissions` continues to the next scope after a failure, prefixes each reason with its scope (`user scope: …`, `repository scope: …`), joins them with `; `, and keeps every completed change; one report row as before; doc comment added; `docs/cli/doctor.md` (Permissions repair) describes independent scopes with an example
        - tests: added `doctor::permissions::tests::a_failed_scope_does_not_stop_the_other_scopes_handoff` (user lock held, then repository lock held; 50 ms `lock_deadline` seam; 0.21 s) and `both_scopes_are_handed_off_together_when_nothing_blocks_them` (four replacements; 0.08 s); updated `an_interrupted_transfer_keeps_the_inline_block_and_a_retry_completes_it` for the scope prefix; the new test fails with the early stop restored; `cargo nextest run -p claudine --lib -E 'test(/doctor::/)'` 128/128 passed; `just lint` clean
- work completed for 'R2-F1' at 05:34:08
- starting the work on 'R2-F2' at 05:38:38
        - defect class: diagnostic and runtime admission rules disagree; Doctor admitted a changed manifest's members with Darkmatter's lexical `bundle::admit_manifest`, which refuses a `../darkmatter/…` re-entry the installed reader (`ReadManifestV2::members`, confined resolution) loads
        - sibling sites swept:
                - `composition/registration/inspect.rs::manifest_load` (load verdict): defective; now uses Darkmatter's installed-member resolution
                - `inspect.rs::mark_digest_mismatches` (digest-note reader): defective; now uses the same resolution, and runs from `Installation::judge_manifest` (which has the context) instead of `inspect_installation`
                - `composition/registration.rs::previous_member_paths` (retirement): legitimately differs; keeps lexical `admit_manifest`, so an ambiguous old manifest retires nothing (deletion stays conservative)
                - `repair.rs` post-publish verification: legitimately differs; byte equality with the inventory just written, no manifest admission
                - `inspect.rs::scan` (extras): legitimately differs; walks the inventory's names, never the manifest's
                - `inspect.rs::entry_registered` (config `register` identity): clean; Darkmatter's `manifest_identity`, unchanged
        - changes: Darkmatter `bundle::admit_installed_manifest(manifest, bytes, context)` (shared `admit_with` with `admit_manifest`; reads no member, checks no link; name is outside the `registrations_command.rs` loader guard, so no allowlist change); Claudine `Installation` keeps the manifest's and changed members' bytes (`read`), `judge_manifest` admits once with the confined resolution and uses it for digest notes and the load verdict; docs: `darkmatter/docs/topics/host-registration.md` (new function with example), `claudine/docs/cli/doctor.md` (member spellings), `.claude/skills/claudine/SKILL.md` (stale `inspect_installation(…, Some(context))` text replaced with `judge_manifest`), `.claude/skills/darkmatter/SKILL.md` (new function)
        - tests: `doctor/registration_tests.rs` `member_spelling_cases!` generates, for `action.yaml`, `composition.yaml`, `formal-sequence-file.yaml`, `group.yaml`, `globals.lifecycle.json`: `the_canonical_spelling_beside_a_manifest_comment_warns`, `a_spelling_that_reenters_the_bundle_warns`, `a_spelling_that_leaves_the_bundle_is_an_error` (15 tests, each under 0.8 s); `a_member_that_disagrees_with_its_digest_is_attributed_under_either_spelling` (canonical, reentry); Darkmatter `host_registration_snapshot::installed_admission_resolves_every_member_category_as_the_reader_does` (files, lifecycle, function catalog and facts; escape refusal equals the reader's); `repair_tests.rs::a_digest_listed_under_another_case_is_checked_where_the_volume_resolves_it` now calls `judge_manifest`; with lexical admission temporarily restored in `judge_manifest`, the re-entry digest-note case and the lifecycle and namespace re-entry cases failed before fail-fast stopped the run
        - commands: `cargo nextest run -p claudine --lib -E 'test(/doctor::registration/) | test(/composition::registration/)'` 72/72 passed (max 0.88 s); `cargo nextest run -p claudine --test l1 -E 'test(/installed_registration/) | test(/registration_inventory/)'` 56/56 passed; `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/registrations_command/) | test(/l1::doctor::/) | test(/^doctor::/)'` 30/30 passed; `cargo nextest run -p darkmatter -E 'test(/host_registration_snapshot/) | test(/admission/)'` 68/68 passed; `cargo clippy -p darkmatter -p claudine --lib --tests -- -D warnings` clean
- work completed for 'R2-F2' at 05:44:00
- starting the work on 'R2-F3' at 05:45:12
        - defect class: loss of a setting's scope during merge and reporting; the merged semantic verdict carried no setting identity, so `typed` attributed a merged failure to the user file whenever any user-only check failed, and the user-only readers (`tts`, user permissions handoff) pointed at a `config` row that the plan keeps clear
        - sibling sites and verdicts:
                - `ClaudineConfig::validate` (protect, messenger selection, route definitions, three default sounds, exit expressions): fixed; now `validate_settings` tags the first failing setting (`ValidatedSetting`), `validate` keeps its order and errors
                - merged settings per `merge_repo_override`: actions per event (fixed, repository when it lists the event), `exit_expressions` (fixed, repository when it declares them), `active_messenger` (fixed, both files when the user has a `messenger` block), `canonical_provider` / `matchers` / `guard_settings` / `steering` (clean: no semantic rule judges them), protect / default sounds / route definitions (clean: user-only, stay user), preferred agent / tts (clean: no merged override and no semantic rule)
                - `config` row semantic findings (`doctor/config.rs::typed`): fixed; also reports a user-only failure the overrides do not replace when the merged check stopped at another setting first
                - `tts` row (`doctor/tts.rs::assess`): fixed; explains a superseded user-only problem itself (still `info`)
                - permissions handoff user scope `ManualReason::ClaudineInvalid` (`doctor/permissions.rs::refused`): fixed; refusal kept, note names the blocking setting; repository scope unchanged (its only refusal is the repository typing failure that `config` reports under the repository file)
                - registration row, path row: clean; neither reads `ConfigObservations.user_config` or the semantic verdict
                - schema-invalid file identity and stray-key projection: clean, unchanged (scope assigned before merge)
        - changes: `lib/src/config/claudine_config.rs` (`validate_settings`, `ValidatedSetting`, boxed error for `result_large_err`), `lib/src/config/messaging_block.rs` (selection and route checks split; the combined `validate` had no other caller and was removed), `lib/src/doctor/config.rs` (`SemanticProblem::Rule { setting: ValidatedSetting }`, `Mapper { event: AgenticEvent }`, `Source` + `source_of`, `setting_name`, `describe`, `ConfigObservations::superseded_user_problem`, `superseded_note`, `SUPERSEDED_REMEDY`; message-parsing `named_setting` removed), `lib/src/doctor/tts.rs` (`assess(&ConfigObservations, …)`), `lib/src/doctor/permissions.rs`, `docs/cli/doctor.md` (new "Which file a rule names" section, `tts` and handoff notes)
        - tests: `doctor/config_tests.rs`: `a_failing_repository_selection_names_the_repository_selection_and_the_user_routes`, `an_action_mapper_failure_names_the_file_that_authors_the_event`, `an_exit_expression_failure_names_the_file_whose_declaration_is_effective`, `a_valid_repository_selection_of_a_defined_user_route_passes`, `user_only_settings_keep_the_user_attribution_beside_any_override`, `a_user_problem_the_overrides_replace_is_superseded_and_left_to_its_readers`; updated `a_repository_selection_with_no_user_route_names_both_files` (now names `active_messenger` and the user's `messenger.configurations`) and `a_semantic_rule_names_the_setting_without_its_value`; `doctor/tts_tests.rs::a_user_only_problem_the_repository_replaces_is_explained_in_the_speech_row`; `doctor/permissions_tests.rs::a_user_only_rule_the_repository_replaces_blocks_the_handoff_and_is_explained_there`; every case asserts a `SECRET` sentinel value never reaches the report; with `source_of` forced to `User`, six config tests failed
        - commands: `cargo nextest run -p claudine --lib -E 'test(/doctor::/) | test(/config::/) | test(/messag/)'` 787/787 passed; `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/doctor/)'` 22/22 passed; `just lint` (claudine) clean
        - decisions:
                - setting identity comes from the validator (`ValidatedSetting`) rather than parsing its message; alternative was extending `named_setting`'s message heuristic, refused because the selection message has no plain lead and quotes the route name
                - a repository selection failure is one finding at the repository file whose note names the user file's `messenger.configurations`; alternative of two findings (one per file) would duplicate one failure
                - a superseded user-only problem is explained only in the rows that read the user file alone, and only when `config` has no user-scope finding; alternative of always repeating it there was refused to keep the existing "config row says why" when `config` does say why
                - repository-attributed findings keep the remedy without a command, since a launch falls back to defaults on a merged rule failure
- work completed for 'R2-F3' at 05:59:30
- starting the work on 'R2-F4' at 06:00:03
        - defect class: reporting a quality ranking as a runtime choice; the good-quality summary said "hook speech uses <best>" even when biscuit-speaks' runtime order (e.g. `TTS_PROVIDER=espeak`) tries another provider first
        - sibling sites and verdicts (`doctor/tts.rs::assess`, enabled/disabled x none/below-good/good x automatic/pinned/unrecognized):
                - enabled automatic, Good+, best differs from first (R2-F4.1): fixed; summary now "the best available speech provider is <best> (<q> quality); hook speech tries <first> first", plus the existing "automatic selection tries <first> (<q> quality) first" note
                - enabled automatic, Good+, best is first: clean; "hook speech uses <best>" is true there and kept
                - unrecognized provider fallback (R2-F4.2): fixed by the same branch; the warning and fallback note are unchanged
                - enabled automatic below Good (R2-F4.c1): clean; already "the best available speech provider, <best>, has <q> quality" plus the tries-first note
                - pinned, any quality (R2-F4.c2): clean; candidates are filtered to the pin, so no first-candidate note and "uses <pin>" is the runtime choice; missing pin stays `error`
                - disabled, every quality, and every no-candidate branch (R2-F4.c3): clean; disabled says "<best> is available to lifecycle `say`" (availability, not selection) and no-candidate rows name no provider
                - other doctor rows (`config`, `permissions`, `registration`, `path`): clean; none phrases a provider-selection claim
        - changes: `lib/src/doctor/tts.rs` (`tried_first` computed once, used by the summary and the note; module doc says best-rated plus tried-first), `docs/cli/doctor.md` (`tts` section: "uses" only when best is first, with a `TTS_PROVIDER=espeak` example); severities unchanged; caller skill `use-claudine/cli.md` quotes no wording, not edited; CLI tests assert only the `tts` severity, not edited
        - tests (`lib/src/doctor/tts_tests.rs`): updated `automatic_selection_reports_the_highest_rated_and_the_runtime_first_candidate` (no longer requires "uses Kokoro" with eSpeak first) and `quality_below_good_warns_and_good_or_better_passes_when_enabled` (Piper-first slice for "uses Piper"); added `a_preferred_first_provider_changes_what_is_tried_first_but_not_the_best`, `an_unrecognized_provider_falls_back_and_separates_the_best_from_the_first`, `below_good_automatic_selection_warns_and_names_the_best_and_the_first`, `a_pinned_provider_is_named_alone_whatever_runtime_order_prefers`, `disabled_speech_claims_no_runtime_selection`
        - commands: `cargo nextest run -p claudine --lib -E 'test(/doctor::tts/)'` 16/16 passed (each < 0.1 s); `just lint` in claudine exit 0
- work completed for 'R2-F4' at 06:02:36
- starting the work on 'R2-F5' at 06:03:00 (approximate; the start time was not captured when work began)
        - defect class: a remedy claims behavior outside the target command's validation or edit scope; the remedy is now chosen by issue category and scope, and where no command is verified to handle the category, it names the location to correct (file, key, event) with no raw value
        - remedy sites swept (`lib/src/doctor/`) and verdicts, verified against `target/debug/{claudine,md}` with a temporary HOME and `git init` repository under `/tmp/r2f5/`:
                - `config.rs::semantic_finding`, user `Rule` (default sound, both `exit_expressions` failures, messenger selection; protect and routes go through the same `ClaudineConfig::validate` in `load_claudine_config_with_notices`) and `Typed` (the same loader's `from_effective`): clean; `claudine actions` exits 1 with the error, so the command is kept (R2-F5.1 control)
                - `config.rs::semantic_finding`, user `Mapper` (R2-F5.1): fixed; `claudine actions` exits 0 and lists the action (no load compiles mappers); the remedy is now "correct `actions.<event>[<i>].call.mapper.pattern` so it is a valid regular expression" (only `call` has a mapper, only `regex` can fail)
                - `config.rs::semantic_finding`, repository scope (merged rule or repository mapper): clean (R2-F5.c2); generic "correct the setting" kept
                - `config.rs::SUPERSEDED_REMEDY` (used by `tts.rs`): fixed, same class; now `superseded_remedy(problem)`; a superseded rule keeps `claudine actions` (verified: user `exit_expressions` replaced by the repository, `actions` from the repository still exits 1 with it, since it reads the user file alone), a superseded mapper names the setting in `~/.claudine/config.yaml`
                - `config.rs::darkmatter_problem_finding` (R2-F5.2): fixed; `md compose` of a `::shell` document prints syntax, not-a-mapping, empty file, unknown key, invalid inline `permissions`, and lexical `permissions` references (remote) in both scopes, so those keep `MD_FULL_MESSAGE`; `register: 3` in either scope reaches shell approval without printing it, so a pointer under `/register` now gets "correct the `register` setting at that position: it is a list of manifest references; then run `claudine doctor` again"; any other kind (read, migration, snapshot) gets the direct correction (not verified with `md`). The schema declares only `permissions` and `register`, so every other `InvalidSetting` is a `permissions` problem
                - `config.rs` claudine syntax/invalid-value remedies (`USER_FULL_MESSAGE`, `REPOSITORY_FULL_MESSAGE`), legacy remedies (`claudine setup` user, `REPOSITORY_LEGACY_REMEDY`), missing user files (`claudine setup`): clean (R2-F5.c1); existing CLI cases still pass, and `claudine setup` was rerun creating a missing `~/.darkmatter.yaml` and a missing user config
                - `config.rs` unknown-key, unreadable, context and HOME remedies, `stray_finding`: clean; direct corrections
                - `permissions.rs::FIX` on a repairable handoff (R2-F5.3): fixed; `claudine setup` is user-only (`cli/src/commands/setup.rs` passes `ConfigScope::User`), so the repository remedy is "run `claudine doctor --fix`" alone; user omitted/inline keep "or `claudine setup`" (verified with publication available); new sibling: user `permissions: null` — setup refuses it as a differing block when Claudine holds one, and with none it moves `null` into Claudine's file and leaves a broken reference, so the null remedy no longer offers setup
                - `permissions.rs::refused` `BrokenReference` (R2-F5.4): fixed; repository scope now "correct the reference or the file it names, then run `claudine doctor` again", with no `md compose` pointer (Darkmatter's broken-reference message suggests the user-only `md init --inline`; R2-F8) and no initializer; user scope keeps both (verified `md` prints it and `md init --inline` rewrites the user file)
                - `permissions.rs::refused` `UnusableMapping`: fixed, same class; the old text interpolated `FIX` inside backticks (it rendered as a nested, broken code span), offered setup at repository scope, and claimed `md` prints a mapping that is not yet in use; now "correct its `permissions` mapping for the <scope> scope, then run `claudine doctor --fix`"
                - `permissions.rs::refused` `Darkmatter` (invalid inline block or lexical reference): clean; verified printed by `md compose` in both scopes
                - `permissions.rs::refused` conflicts, `ClaudineUnusable`, `ClaudineInvalid`, and obstruction blockers: clean (R2-F5.c2); direct corrections plus doctor itself
                - `registration.rs` (`FIX`, blockers, preconditions, missing `~/.darkmatter.yaml` via `claudine setup`): clean; user-only doctor repair, verified by existing CLI repair cases and a setup rerun
                - `path.rs`, `tts.rs` provider remedies: clean; direct actions, no command claims
        - R2-F5.d1: `docs/cli/doctor.md` "Where the full message is printed" no longer says doctor reads no configuration (it reads the files only to diagnose them and never stops on one), and lists the mapper and `register` exceptions; the `darkmatter-permissions` section says when setup is offered and why repository references are corrected by hand; the source comment above the shared wording in `lib/src/doctor/config.rs` made the same false claim and was corrected (the `main.rs` and `commands/doctor.rs` comments only say doctor dispatches before loading, which is true, and were left)
        - changes: `lib/src/doctor/config.rs` (`semantic_finding`, `mapper_remedy`, `superseded_remedy`, `darkmatter_problem_finding`, `REGISTER_REMEDY`, shared-wording comment, `MD_FULL_MESSAGE` doc), `lib/src/doctor/tts.rs` (uses `superseded_remedy`), `lib/src/doctor/permissions.rs` (`FIX`/`FIX_OR_SETUP`, scope-aware `refused`, `UnusableMapping` wording), `docs/cli/doctor.md`
        - tests:
                - `doctor/config_tests.rs`: `a_user_mapper_remedy_names_the_setting_and_no_command`, `a_darkmatter_remedy_names_md_compose_only_for_problems_a_compose_prints` (user and repository; four printed categories and `register: 3`)
                - `doctor/tts_tests.rs`: `a_superseded_user_mapper_is_corrected_in_place_without_a_command`
                - `doctor/permissions_tests.rs`: `setup_is_offered_only_for_a_user_handoff_it_performs` (user inline, user null, repository inline), `a_broken_repository_reference_is_corrected_without_a_user_scoped_command` (with the user-scope control); `a_mapping_that_fails_the_scoped_rules_is_manual` now asserts its remedy
                - `cli/tests/l1/doctor.rs`: `claudine_actions_reports_a_broken_rule_but_not_a_broken_mapper` (sound exits 1 with the setting, mapper exits 0 without it), `a_repository_handoff_is_left_by_setup_and_repaired_by_doctor_fix` (setup exits 0 and leaves the repository block; `doctor --fix` writes the reference); each < 0.3 s
                - `md` behavior is not spawned from Claudine's tests (no `md` binary target in this package); it was verified by hand as listed above and the remedy text per category is pinned by the lib tests
        - commands: `cargo nextest run -p claudine --lib -E 'test(/doctor::/)'` 163/163 passed; `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/doctor/)'` 24/24 passed; `just lint` in claudine exit 0
        - decisions:
                - a register finding gets a direct correction rather than a Darkmatter command: no `md` command was found that prints it, and the registration repair requires a valid `~/.darkmatter.yaml`
                - the repository broken-reference remedy names no `md` command at all, rather than keeping `md compose` for the message: that message currently recommends the user-scoped `md init --inline` (R2-F8 fixes Darkmatter's side)
                - setup's handling of `permissions: null` (refusing it as a differing block, or moving `null` into Claudine's file and leaving a broken reference) is a setup defect, not a remedy; it was not changed here and is reported to the caller for a decision
- work completed for 'R2-F5' at 06:14:40
- starting the work on 'R2-F6' at 06:16:19
        - defect class: a migration diagnostic (or a schema refusal) suppresses independent validation evidence; `load.rs::project` returned before the document-level rules (both agent spellings; exit expression with both/neither `pattern`/`patterns`, list and `rules` forms)
        - sibling sites and verdicts:
                - `load.rs::project` flat branch (`return Err(problems)` after judging the keyed view): defect, fixed
                - `load.rs::project` keyed branch (`Err(failure) => return Err(...)` on a schema refusal): same class (a misspelled key hid both pattern rules), fixed
                - `load.rs::project_value` / `document_from_bytes`: thin wrappers over `project`, inherit the fix
                - `wire::flat_entries` / `upgrade_flat` / `FlatEntry::authored`: no early return; the document rules read the authored map, whose `exit_expressions`/agent keys are never inside a flat entry, so their coordinates are already authored
                - `json_conversion.rs`: upgrades flat entries then calls `project_value`, so it gains the rules beside a schema refusal; still fails without writing (test added)
                - `doctor/config.rs::observe_claudine`: passive reader over `read_document`, inherits the fix; the semantic checks (`typed`) need a valid document by construction (an invalid one cannot be typed), not an independent short-circuit; unchanged
                - `persist.rs::validate_config_text`: goes through `document_from_bytes`, inherits the fix
                - input robustness: `exit_expression_problems` treats `null` as unwritten (matches the schema's null-as-absent); the agent rule tests key presence, matching serde's `alias = "favorite_agent"`, which refuses both keys whatever their values; no change
        - changes: `lib/src/config/load.rs` (`project` collects schema/migration problems, always runs both document rules, returns the combined list), `docs/topics/configuration.md` (flat-entry section names the document rules and their pointers), `docs/cli/doctor.md` (`stray-config-keys` flat paragraph)
        - tests:
                - `lib/src/doctor/config_tests.rs`: `independent_rule_cases!` (20 rows): user action keyed/flat × both agents, both patterns, no pattern, both `rules` patterns; misspelled keyed action × `rules`, misspelled flat action × agents; repository action keyed/flat × both, none, `rules`; user route keyed/flat × agents, `rules`. Each asserts the exact `(kind, pointer)` set from `read_document` (independent + migration + unknown key), the independent problem's authored line/column, and the doctor `config` row text; 11 of 20 (all flat and both misspelled rows) failed with the early returns restored, all pass now (~0.05 s each)
                - `lib/tests/l1/config_json_conversion.rs`: `a_converted_candidate_is_judged_by_the_document_rules` (flat action + both patterns; schema refusal + both patterns; nothing written); `assert_refused` now returns the pointers
        - commands: `cargo nextest run -p claudine --lib -E 'test(/config::/) | test(/doctor::/)'` 622/622 passed; `cargo nextest run -p claudine --test l1 -E 'test(/config/)'` 67/67 passed; `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/doctor/) | test(/config/)'` 247/247 passed; `just lint` in claudine exit 0
- work completed for 'R2-F6' at 06:21:51
- starting the work on 'R2-F7' at 06:22:10
        - defect class: a repair preflight accepts a file that its writer deterministically refuses; the permissions handoff treated a readable `.darkmatter.yaml` as ready while its `register` setting was invalid, and Darkmatter's writer (`dm_init::check_config_text`, strict whole-file schema) refused every candidate
        - sibling sites and verdicts:
                - `doctor/permissions.rs::assess`, user scope (R2-F7.1): defect, fixed
                - `doctor/permissions.rs::assess`, repository scope (R2-F7.2): defect, fixed by the same check, keyed on the scope's own Darkmatter file
                - Claudine-side edit of the handoff (`decide_for_doctor`): clean; an edited Claudine file is already judged by its full validation (`validate_config_text` on the observed bytes, `ManualReason::ClaudineInvalid`) and the block it receives by `check_referenced_permissions` (`UnusableMapping`), the same two judges the writer runs; an invalid Claudine file is skipped by `prerequisites`
                - other whole-file Darkmatter refusals: clean; unknown keys and root problems make the observation `DarkmatterState::Invalid` (scope skipped), an invalid `permissions` is `HandoffRefusal::Darkmatter`; `register` was the only setting the loader reports per entry while the writer refuses the whole file
                - registration entry repair (R2-F7.c1): clean; `inspect.rs::entry_precondition` already refuses on `registration_findings()` and an invalid inline `permissions`; test added
                - already-referenced permissions (R2-F7.c2): clean by construction; the check runs only after the planner returns a plan that edits, so a reference keeps its `ok`/`info` verdict; test added
                - setup mode: not a `fixable` claim; setup's writer refuses the same file and the CLI warns; unchanged
        - changes: `lib/src/doctor/permissions.rs` (`darkmatter_file`; after a plan that edits, a non-empty `ConfigFile::registration_findings()` of the scope's `.darkmatter.yaml` settles the scope as a manual `error` at that file: same summary, note "the repair cannot edit this file while its `register` setting is invalid, which is reported under `config`", remedy "correct the `register` setting, then run `claudine doctor --fix`"); the writer's own validation is unchanged; `docs/cli/doctor.md` (table row, `register` paragraph, Permissions repair), `docs/topics/configuration.md` (doctor paragraph), `.claude/skills/claudine/SKILL.md`
        - tests:
                - `doctor/permissions_tests.rs`: `register_shape_cases!` (14 rows: user and repository × omitted, `null`, `[]`, valid list repairable; scalar `3`, mixed `[valid, 3]`, all-invalid `[3, '']` manual). Manual rows assert `fixable: false`, the file named and the remedy, `--fix` leaving the whole tree identical (bytes, modification times, no lock sidecar), and that committing the same plan directly is `ConfigWriteError::ValidationFailed` with the inline block kept; repairable rows assert `--fix` reaches `ok` with the block moved and `register` kept. All 6 manual rows failed with the check disabled
                - `an_invalid_repository_register_leaves_the_user_handoff_repairable` (R2-F7.2 independence; failed with the check disabled): the user scope is repaired (2 writes), the repository pair is byte-identical, no repair failure is reported
                - `an_existing_reference_keeps_its_verdict_beside_an_invalid_register` (R2-F7.c2, user and repository): `ok`, no edit, tree identical
                - `doctor/registration_tests.rs`: `only_a_valid_register_setting_admits_the_entry_repair` (R2-F7.c1): omitted, `null`, `[]` repaired; `3` and `[3]` manual with nothing written
                - each case 0.05–0.11 s
        - commands: `cargo nextest run -p claudine --lib -E 'test(/doctor::/) | test(/config::setup/)'` 202/202 passed; `cargo nextest run -p claudine --test l1 -E 'test(/darkmatter_setup/) | test(/installed_registration/)'` 65/65 passed; `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/doctor/) | test(/setup_command/)'` 30/30 passed; `just lint` in claudine exit 0
        - decisions:
                - the verdict is the observation `config` already holds (`ConfigFile::registration_findings()`), the same source `entry_precondition` uses, rather than running `check_config_text` on a reconstructed candidate; alternative rejected: judging a re-serialized candidate would re-derive the writer's input and differ from the edited YAML text
                - consequence recorded: a schema-valid but unusable entry (a remote URL, an unparsable reference) is also a registration finding, so it makes the handoff manual although the writer would accept it; kept, because it is an error the user must correct in that file anyway and both repairs then agree on one rule
                - the check applies only to plans that edit `.darkmatter.yaml` (every doctor edit does), so a reference's verdict is never touched
- work completed for 'R2-F7' at 06:30:40
- starting the work on 'R2-F5.x1' at 06:24:56
        - defect class: same as R2-F7 (handoff eligibility admits a candidate that cannot be handed off correctly); setup's planner read `permissions: null` as an inline block
        - reproduction (before the fix, all failed): `lib/tests/l1/darkmatter_setup.rs::null_handoff_cases!` — Darkmatter `null` with no Claudine block (setup reported `Transferred` and wrote `null` into Claudine's file, leaving a reference to a file with no mapping), Darkmatter `null` with a Claudine block (refused as `Conflict`), and the Claudine-side twins: Claudine `null` with Darkmatter omitted (no default block written, broken reference) and with an inline block (refused as `Conflict`, since `effective_permissions(null)` is `None`), plus `null` on both sides
        - authority: Darkmatter's loader applies the `permissions` schema default to a `null` value (`observe` returns `BlockState::Inline(Null)` only when `file.permissions()` is `Ok`), and Claudine's schema defaults replace `null` too; so `null` is no authored block on either side
        - sibling sites and verdicts:
                - `config/setup.rs::decide_for_setup`, Darkmatter `Inline(Null)`: defect, fixed (handled as omitted: reference only; Claudine gets the default block when it has none)
                - `decide_for_setup`, Claudine's `block_now` / `block_before` holding `null`: same class, fixed (filtered as no block, as doctor already did)
                - `decide_for_setup` reference arm with Claudine `null`: same class, fixed by the same filter (the default block is written, so the reference resolves)
                - `decide_for_doctor`: clean; it already read `null` on both sides as omitted (`the_doctor_planner_reads_null_and_omitted_as_handoff_requests`)
                - `doctor/permissions.rs` R2-F5 remedy special case for `null`: removed; the user `null` remedy offers `claudine setup` again, since setup now performs exactly doctor's handoff
        - changes: `lib/src/config/setup.rs` (`decide_for_setup`; planner doc table row), `lib/src/doctor/permissions.rs` (`FIX_OR_SETUP` doc, scope-only remedy), `docs/cli/doctor.md` (null paragraph; the "except when `permissions` is `null`" clause removed), `docs/topics/configuration.md` (setup table and `null` paragraph; also corrected the existing drift that a reference to Claudine's file changes "nothing": setup has always given Claudine's file the default block when it had none), `.claude/skills/claudine/SKILL.md`
        - tests: the five `darkmatter_setup::setup_null_*` cases (each asserts the handoff kind, the reference, Claudine's block never `null`, Claudine validation, and that Darkmatter's policy loads from Claudine's file); `permissions_tests.rs::setup_is_offered_only_for_a_user_handoff_it_performs` now expects setup offered for a user `null`; each < 0.1 s
        - commands: as for R2-F7 above (same runs)
        - decisions: `null` follows omitted (Darkmatter's own reading); alternatives rejected: refusing `null` as manual (contradicts the loader, which runs with the default policy) and copying `null` (leaves a broken reference)
- work completed for 'R2-F5.x1' at 06:30:40
- starting the work on 'R2-F8' at 06:32:09
        - defect class: a repair command whose write scope differs from the error's scope (`md init --inline` writes `~/.darkmatter.yaml`, so for a repository reference it exits 0 and leaves the error)
        - sibling sites and verdicts:
                - `darkmatter/lib/src/config/load/reference.rs`, missing target, invalid YAML, chained reference, nonmapping document, wrong block type, missing block (six branches): defect, fixed through one shared `Source::remedy(fix)` keyed on the declaring `ConfigRevision`
                - `reference.rs` `invalid_target` (block fails the schema or entry checks) and `follow` errors (outside the repository, link cycles, unresolvable reference): clean, they name no command
                - `darkmatter/lib/src/config/load.rs` `dm.config.missing` ("`md init` creates it"): clean, user file and user-scoped command; same for the DMLS session notice (`dmls/src/overlay/hosts.rs`, `router.rs`)
                - other `md` remedies in the lib/CLI (`md clean`, `md hash --save`): document scope, not configuration; clean
                - `darkmatter/docs/cli/init.md` "use `--inline` when the problem is a reference": describes `md init`'s own behavior, accurate; unchanged
                - `darkmatter/docs/topics/configuration.md` reference rule "suggesting `md init --inline`": drift, fixed (scope table and example)
                - `darkmatter/docs/use-skills/`: quotes no remedy; nothing to publish
                - Claudine: no code or test quotes Darkmatter's remedy text; the doctor repository remedy is already a direct correction and stays; the comment above it claimed `md`'s message acts on the user file, now stale, reworded (comment only)
        - changes: `darkmatter/lib/src/config/load/reference.rs` (`Source::remedy`, `WRITE_MAPPING`; user keeps `md init --inline`, repository working copy names a direct correction in the declaring file, `HEAD` adds that the correction must be committed), `darkmatter/docs/topics/configuration.md`, `claudine/lib/src/doctor/permissions.rs` (comment)
        - tests: `darkmatter/lib/tests/l1/config_permissions.rs` `scoped_remedy_tests!` (18 tests: `{missing_target,invalid_yaml_target,chained_target,nonmapping_target,wrong_block_type,missing_block}_remedy_{user,worktree,head}`), `an_equivalent_reference_spelling_keeps_the_remedy` (`./policy.yaml` vs `./sub/../policy.yaml`, all three revisions), `a_valid_referenced_mapping_reports_no_initializer_in_either_scope`; `darkmatter/cli/tests/l1/config_notices.rs::a_broken_repository_reference_names_a_direct_correction_not_md_init` (`md compose --output json` wiring); each < 0.1 s
        - commands: `cargo nextest run -p darkmatter -E 'test(/reference/) | test(/permission/) | test(/config/) | test(/init/)'` 1464 passed; `cargo nextest run -p darkmatter-cli -E 'test(/init/) | test(/permission/) | test(/config/)'` 70 passed; `cargo nextest run -p claudine --lib -E 'test(/doctor::/)'` 200 passed; `just lint` (darkmatter) clean
- work completed for 'R2-F8' at 06:40:05
- phase gates (orchestrator, run after every subagent finished, with no subagent running)
        - `just test` in `claudine`: 12979 passed, 16 skipped; "none at or above 5.0s"
        - `just test` in `darkmatter`: 17884 passed, 10 skipped; "none at or above 5.0s"
        - `just lint` in `claudine` and `darkmatter`: exit 0
        - `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter`: exit 0
        - `just cross-check claudine --os windows`: pass (7173 passed)
        - `just cross-check darkmatter --os windows` attempt 1: one failure, `host_trigger_merge::a_conflict_between_named_triggers_shows_both_labels`. That test was not touched this cycle. The shared fixture `host_fixtures::Fixture::path` joined `host/schemas/a.yaml` as one string, so the expected path displayed `…\host/schemas/a.yaml` while the error shows native separators
                - fix: `darkmatter/lib/tests/host_fixtures/mod.rs::Fixture::path` now joins one segment at a time; Linux `cargo nextest run -p darkmatter --test l1` 6142/6142 passed and `just lint` in darkmatter exit 0 after the change
                - attempt 2: pass (12966 passed)
        - Windows-rig timings above 5 s, none in code this cycle touched and none above 5 s in the Linux gate: darkmatter `glob_implementation_guard::file_references_have_one_glob_implementation` 10.6 s, `invalid_triggers::a_denied_child_only_trigger_stops_a_pre_approved_{body,frontmatter}_shell` 9.5–9.6 s, `link_normalization::{env,home,repo}_anchor_preserves_every_unsafe_category` 6.0–7.8 s, `invalid_triggers::a_denied_child_only_trigger_stops_the_nested_gate` 7.5 s, `host_trigger_merge::a_recursive_reference_and_its_inline_unfolding_coalesce_at_every_nested_position` 6.7 s, `file_tree_roots::a_relative_link_that_leaves_the_repository_is_an_error` 5.6 s; claudine `composition::preflight::tests::a_saved_choice_answers_the_next_run_without_a_prompt_and_is_not_written_twice` 5.0 s. These are archive runs on build-win. The listed durations include L1 timing failures even where nextest reported passing assertions

### Completion Report

- **outcome: complete.** Started 05:27:48 UTC, finished 07:19 UTC, about 1 h 51 min elapsed
- parent findings: total 8 = fixed 8 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - one discovered defect (R2-F5.x1, setup's handling of `permissions: null`) was added to the ledger and fixed; it is not counted as a review parent
- verified repairs
        - R2-F1: the permissions repair continues to the other scope after one scope fails, and keeps completed changes and scope-prefixed reasons
        - R2-F2: Doctor admits manifest members through Darkmatter's confined installed-member resolution (`bundle::admit_installed_manifest`) for both the load verdict and digest notes. Retirement keeps the lexical admission
        - R2-F3: semantic failures name the file that sets the effective value (`ValidatedSetting` + `source_of`), or both files for a repository messenger selection. `tts` and the user handoff explain a user-only problem that the repository supersedes
        - R2-F4: automatic speech reports "best available" and "tries first"; "uses" appears only when they are the same provider
        - R2-F5: the review 2 remedy fixtures verified issue category and scope. The user setup suggestion was verified with a working publisher. Setup can stop before the handoff if publication is obstructed
        - R2-F6: `config/load.rs::project` always runs the two document-level rules beside migration and schema problems
        - R2-F7: a handoff that would edit a `.darkmatter.yaml` with `register` problems is manual, not fixable
        - R2-F8: Darkmatter's broken-reference remedies are scope-aware; repository errors name a direct correction, and errors at HEAD say the correction must be committed
        - R2-F5.x1: setup and Doctor read `permissions: null` as no block, as Darkmatter's loader does
- open findings: none
- decisions recorded (ledger only; none met the worth-the-author's-time test)
        - R2-F7: a `register` entry that has the right type but cannot be used (for example a remote URL) also makes the handoff manual. This is the same rule the registration repair uses, and the user must correct that entry anyway
        - R2-F5.x1: explicit `null` follows Darkmatter's loader (default policy). The handoff writes the reference, creates a default mapping when Claudine has none, and never copies `null`
- changed files this cycle
        - claudine library: `lib/src/doctor/{config,permissions,tts}.rs`, `lib/src/doctor/{config,permissions,tts,registration}_tests.rs`, `lib/src/config/{load,setup,claudine_config,messaging_block}.rs`, `lib/src/composition/registration/{inspect,repair_tests}.rs`, `lib/tests/l1/{darkmatter_setup,config_json_conversion}.rs`
        - claudine CLI: `cli/tests/l1/doctor.rs`
        - darkmatter: `lib/src/config/registration/bundle.rs`, `lib/src/config/load/reference.rs`, `lib/tests/l1/{host_registration_snapshot,config_permissions}.rs`, `lib/tests/host_fixtures/mod.rs`, `cli/tests/l1/config_notices.rs`
        - docs and skills: `claudine/docs/cli/doctor.md`, `claudine/docs/topics/configuration.md`, `darkmatter/docs/topics/{host-registration,configuration}.md`, `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/SKILL.md`
- final verification
        - claudine `just test` 12979 passed, no slow tests (all findings); darkmatter `just test` 17884 passed, no slow tests (R2-F2, R2-F8)
        - `just lint` claudine and darkmatter exit 0; `just check-tier-coverage` claudine and darkmatter exit 0
        - Windows cross-check: claudine pass; darkmatter pass on attempt 2, after the separator fix in the shared test fixture

### Review 3 verification (2026-11-24)

- The completion summary generalized the remedy tests to every case.
  In a temporary home with equal inline policies, I replaced a published
  schema with a link to the same bytes. Doctor offered setup, but setup
  exited 1 during publication and left the inline policy intact. With
  the same files, `claudine doctor --fix --json` wrote the user reference
  while still reporting the registration obstruction. An ordinary
  published home passed the setup control. A link at the publisher's
  lock also stopped setup when the registration row was healthy.
- The null decision shortened the handoff to a reference-only write.
  The Claudine setup-null integration tests passed and showed that a
  missing Claudine mapping receives a default mapping before Darkmatter
  refers to it. An existing mapping can be reused.

## Implementation of Review Findings #3

> **started at:** 2026-11-24T07:26:50+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- findings are handled serially, one subagent per finding; each subagent reports log items back to the orchestrator, which records them below

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | Doctor offers permissions repairs whose reference the writer refuses | fixed | all seven children verified; assessment and writer share `HandoffPlan::reference_in_scope` |
| R3-F1.1 | User handoff, `config.yaml` linked inside/outside home: outside is manual with user target and remedy, before and after `--fix` | fixed | `permissions_tests::a_reference_only_handoff_through_a_link_repairs_inside_its_scope_and_is_manual_outside` (user file link, omitted/null/equal inline): inside repairs; outside not fixable before and after `--fix`, names the user target, tree unchanged |
| R3-F1.2 | User handoff, linked `.claudine` folder inside/outside: same verdict as direct link | fixed | same test, linked `.claudine` folder case: same verdict as the file link |
| R3-F1.3 | Repository handoff, file link inside/outside repository: outside manual with repository target | fixed | same test, repository file link; `a_repository_link_outside_its_scope_leaves_the_user_handoff_repairable` keeps the user scope repairable |
| R3-F1.4 | Repository handoff, linked `.claudine` folder inside/outside: same verdict | fixed | same test, repository linked folder case |
| R3-F1.c1 | Default/transfer with Claudine write, linked file manual before edit, both scopes | clean | `a_handoff_that_writes_claudines_file_through_a_link_is_manual_inside_and_outside_its_scope` (both scopes × file/folder × inside/outside); unlinked control in `a_linked_claudine_folder_is_manual_…` writes both files |
| R3-F1.c2 | Existing references validated independently of proposed-reference check | clean | `an_authored_reference_to_a_linked_file_keeps_darkmatters_own_verdict`: user external passes, repository external fails as broken, none fixable; the new check returns true when the plan does not edit `.darkmatter.yaml` |
| R3-F1.c3 | Registration publication and entry writer preconditions preserved | clean | registration tests pass; the registration writer uses the fixed `MANIFEST_REFERENCE` and `entry_precondition`, with no scope-relative target |
| R3-F2 | Doctor misses broken Darkmatter references when a Claudine file is unavailable | fixed | all six children plus discovered siblings verified; `config` follows every valid reference through `darkmatter::config::init::observe` |
| R3-F2.1 | Repository, no Claudine file: broken targets reported in `config`, exit 1; valid spellings ok; handoff skip kept | fixed | `config_tests` `broken_target_cases!` (missing, invalid YAML, chained, null, scalar, no block, non-mapping) and `a_link_out_of_the_repository_is_a_config_error_and_allowed_for_the_user`; valid `./`/`././` controls; CLI `a_broken_repository_reference_without_a_claudine_file_is_a_located_error_in_both_formats` exit 1, located, `--fix` changes nothing |
| R3-F2.2 | Repository with aliased or malformed Claudine counterpart: broken repository policy still reported | fixed | same cases with an aliased and a malformed repository counterpart; malformed case reports both errors |
| R3-F2.3 | User with absent/malformed Claudine file: Darkmatter error reported beside Claudine error | fixed | same cases in user scope with absent and malformed Claudine file; Claudine error and handoff skip kept; outside link allowed for the user |
| R3-F2.c1 | Valid pair, both scopes: broken target stays in permissions row, not duplicated | clean | valid-pair cases: permissions row stays `error` with a pointer note; `config` carries the diagnostic exactly once |
| R3-F2.c2 | Declaration errors (blank, inline scalar) kept; targets via same resolver | clean | `the_permissions_declaration_is_judged_once_before_any_target` for every counterpart state in both scopes |
| R3-F2.c3 | Darkmatter runtime/repair consumers: reuse working-copy resolver, no repair, alias rule kept | clean | `a_broken_target_is_left_by_fix_and_an_aliased_darkmatter_file_is_judged_once`; HEAD policy not read |
| R3-F2.x1 | Discovered: user `.darkmatter.yaml` reference beside a legacy-only home is reported | fixed | the Legacy counterpart runs in every user-scope `broken_target_cases!` case |
| R3-F2.x2 | Discovered: broken target beside an invalid `register` both reported | clean | `a_broken_target_and_an_invalid_register_are_both_reported` |
| R3-F2.x3 | Discovered: registration row does not mask the reference result | clean | `entry_precondition` blocks only invalid inline block or `register`; `--fix` cases show both results |
| R3-F2.x4 | Discovered: `repo-config`, `stray-config-keys`, `tts` | clean | they read only Claudine files |
| R3-F3 | Setup guidance overlooks the publication that must run first | fixed | all four instances and three controls verified; the handoff remedy is `claudine doctor --fix` in both scopes |
| R3-F3.1 | User handoff remedy with linked registration path offers `claudine doctor --fix` | fixed | `a_linked_registration_path_leaves_the_handoff_to_doctors_fix` (folder, schema folder, manifest, member): setup returns `Publish`, row names only `--fix`, `--fix` writes the reference, registration still `error` |
| R3-F3.2 | User handoff remedy with linked shared parent offers `claudine doctor --fix` | fixed | `a_linked_shared_parent_leaves_the_handoff_to_doctors_fix` (`~/.claudine`, `registrations`) |
| R3-F3.3 | User handoff remedy with linked publisher lock offers `claudine doctor --fix` | fixed | `a_linked_publisher_lock_hidden_by_a_healthy_registration_row_leaves_the_handoff_to_doctors_fix`: registration `ok` before and after |
| R3-F3.4 | User handoff remedy with retirement obstruction offers `claudine doctor --fix` | fixed | `a_linked_former_member_that_publication_retires_leaves_the_handoff_to_doctors_fix` |
| R3-F3.c1 | Setup behavior with healthy publisher/later cleanup preserved | clean | setup unchanged; `darkmatter_setup`, `installed_registration`, `just test-cli setup` (44) pass |
| R3-F3.c2 | Missing/legacy setup guidance and repository handoff guidance preserved | clean | binary probe: missing `~/.darkmatter.yaml` still created and legacy JSON still converted before publication; repository guidance already `--fix` |
| R3-F3.c3 | Other remedy decision sites keep category and scope distinctions | clean | sweep of semantic/mapper, syntax, repository legacy, Darkmatter register/reference, missing `~/.darkmatter.yaml`, registration, path, tts remedies: none depends on publication (except R3-F3.x1) |
| R3-F3.x1 | Discovered: missing user `config.yaml` remedy omitted that setup publishes first | fixed | remedy now names publication and retry; `config_tests::a_missing_user_file_behind_an_obstructed_publisher_is_named_in_the_setup_remedy` |

### Progress

- starting the work on 'R3-F1 Doctor offers permissions repairs whose reference the writer refuses' at 07:27:18
        - defect class: repair eligibility omits a deterministic precondition that the writer enforces, so the report and writer apply different rules to the same proposed change
        - shared rule: `config/setup.rs::HandoffPlan::reference_in_scope` (now `pub(crate)`) is called by `doctor/permissions.rs::assess` after `obstruction()` and again by `commit_permissions_handoff` immediately before the transaction
        - an out-of-scope reference is now a manual `error` at the scope's `.darkmatter.yaml` naming the resolved target; remedy: replace the link with the file it names, then `claudine doctor --fix` (user scope also: author the reference yourself, which Darkmatter's user policy accepts)
        - tests added in `lib/src/doctor/permissions_tests.rs`: `a_reference_only_handoff_through_a_link_repairs_inside_its_scope_and_is_manual_outside` (both scopes × file/folder link × omitted/null/equal inline), `a_repository_link_outside_its_scope_leaves_the_user_handoff_repairable`, `a_handoff_that_writes_claudines_file_through_a_link_is_manual_inside_and_outside_its_scope`, `an_authored_reference_to_a_linked_file_keeps_darkmatters_own_verdict` (all `#[cfg(unix)]` like the file's existing link tests)
        - sweep: permissions (both scopes share one `assess`; register, obstruction, scope, and mapping checks all mirror the writer) and registration repair (invalid settings, path blockers, fixed `MANIFEST_REFERENCE`). Those controls passed. Review 4 found independent scope and YAML layout prerequisites that this sweep missed
        - docs: `claudine/docs/cli/doctor.md` (out-of-scope link section; corrected claim that a linked `.claudine` always blocks), `claudine/docs/topics/configuration.md`, `.claude/skills/claudine/SKILL.md`
        - scoped verification: `cargo nextest run -p claudine -E 'test(/doctor::/) | test(/darkmatter_setup/) | test(/setup/) | test(/registration/)'` 340/340, slowest 2.94 s; `just test-cli doctor` 24/24; `just lint` claudine exit 0
- work completed for 'R3-F1 Doctor offers permissions repairs whose reference the writer refuses' at 07:34:20
- starting the work on 'R3-F2 Doctor misses broken Darkmatter references when a Claudine file is unavailable' at 07:34:20
        - defect class: an unrelated prerequisite (availability of the Claudine counterpart) suppresses an independent validation result (the Darkmatter file's reference target)
        - shared resolver: `doctor/config.rs::observe` now follows every valid user and repository `.darkmatter.yaml` reference through `darkmatter::config::init::observe`, the passive working-copy reader the handoff planner already uses; no resolution is reimplemented
        - new Darkmatter accessor `ConfigFile::permissions_position()` (`darkmatter/lib/src/config/load.rs`) locates the finding; neither the authored reference nor the target path is printed
        - dedup decision: `config` always carries the full broken-target diagnostic; the permissions row keeps its `error` and scope and says "the reference is reported under `config`"
                - rejected alternative: have `config` report only when the permissions row will not; that would require `config` to predict handoff prerequisites, the same coupling that caused the defect
        - reference field shapes: absent/null → not a reference (handoff repair); blank → one declaration error; number/boolean → one declaration error; valid string → followed, broken target is a `config` error
        - discovered siblings: R3-F2.x1 (user `.darkmatter.yaml` beside a legacy-only home) fixed by the same change; R3-F2.x2 (broken target beside invalid `register`) clean, both reported; R3-F2.x3 registration row clean; R3-F2.x4 `repo-config`/`stray-config-keys`/`tts` clean (read only Claudine files)
        - fixture correction: `config_tests`' default `~/.darkmatter.yaml` referred to a Claudine file with no `permissions` block (itself broken); it now refers to its own `~/policy.yaml`. Two CLI tests now expect `(+1 more)` because a broken Claudine file referenced by `~/.darkmatter.yaml` is now also a reference error
        - tests: `config_tests` `broken_target_cases!` (7 shapes × counterpart states, both scopes), `a_link_out_of_the_repository_is_a_config_error_and_allowed_for_the_user`, `a_valid_target_passes_in_both_spellings_whatever_the_counterpart`, `the_permissions_declaration_is_judged_once_before_any_target`, `a_broken_target_is_left_by_fix_and_an_aliased_darkmatter_file_is_judged_once`, `a_broken_target_and_an_invalid_register_are_both_reported`; CLI `a_broken_repository_reference_without_a_claudine_file_is_a_located_error_in_both_formats`
        - docs: `claudine/docs/cli/doctor.md` new subsection on broken references; `.claude/skills/claudine/SKILL.md`; drifted `config.rs` module comment rewritten
        - scoped verification: claudine doctor/setup filter 275 passed; `just test-cli doctor` 25 passed; darkmatter `test(/config/)` 257 passed; `just lint` claudine and darkmatter exit 0; no test ≥ 2 s
- work completed for 'R3-F2 Doctor misses broken Darkmatter references when a Claudine file is unavailable' at 07:52:03
- starting the work on 'R3-F3 Setup guidance overlooks the publication that must run first' at 07:52:03
        - defect class: a suggested command has a mandatory earlier step that can prevent it from acting on the named problem
        - repair: `doctor/permissions.rs` drops the per-scope `FIX_OR_SETUP` choice; every repairable handoff names `claudine doctor --fix` in both scopes; setup's ordering is unchanged and no publisher preflight was added to the report
        - shared test helper `assert_doctor_fix_hands_off_past` (`#[cfg(unix)]`) proves, per obstruction, that in-process `setup_claudine` returns `SetupError::Publish` with `.darkmatter.yaml` unchanged, that the row names only `claudine doctor --fix`, and that the `--fix` registry run writes the reference. The existing registration verdict is unchanged, including `ok` when only the publisher lock is linked in an otherwise healthy installation
        - tests: `every_repairable_handoff_offers_doctors_fix_and_never_setup` (renamed from `setup_is_offered_only_for_a_user_handoff_it_performs`), `a_linked_registration_path_leaves_the_handoff_to_doctors_fix`, `a_linked_shared_parent_leaves_the_handoff_to_doctors_fix`, `a_linked_publisher_lock_hidden_by_a_healthy_registration_row_leaves_the_handoff_to_doctors_fix`, `a_linked_former_member_that_publication_retires_leaves_the_handoff_to_doctors_fix`; all five fail against the old remedy text
        - discovered sibling R3-F3.x1: the missing user `config.yaml` remedy said "run `claudine setup`", which writes the file only after publication; fixed with a remedy that names the publication step and its retry; test `config_tests::a_missing_user_file_behind_an_obstructed_publisher_is_named_in_the_setup_remedy`. There is no independent repair for a missing file, so setup stays the remedy
        - sweep of other remedy sites (semantic/mapper, syntax, repository legacy, Darkmatter register/reference, missing `~/.darkmatter.yaml` created before publication, registration row, path, tts): clean, none depends on publication
        - docs: `claudine/docs/cli/doctor.md`; caller skill source unchanged (its statements remain true), so `just publish-skills` is not needed
        - scoped verification: claudine doctor/setup/registration filter 358 passed (slowest 2.55 s); `just test-cli doctor` 25 passed; `just test-cli setup` 44 passed; `just lint` exit 0
- work completed for 'R3-F3 Setup guidance overlooks the publication that must run first' at 08:00:34
- starting phase verification at 08:00:34: full `just test` and `just lint` in claudine and darkmatter, `just check-tier-coverage`
        - first full `just test` in claudine failed one guard: `registrations_command::only_the_registration_module_locates_or_loads_an_installed_bundle`. The R3-F3 tests (`permissions_tests.rs`, `config_tests.rs`) had called the registration locators `output_root`, `registrations_dir`, and `lock_path` outside the registration module. They now spell the paths as constants, as `registration_tests.rs` does; the guard and the doctor tests (222) pass
- phase verification completed at 08:33:45

### Completion Report

- **outcome: complete.** Started 07:26:50 UTC, finished 08:33 UTC, about 66 min elapsed
- parent findings: total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: 22 required rows (12 fixed, 10 clean); 5 discovered rows (R3-F2.x1 and R3-F3.x1 fixed, R3-F2.x2–x4 clean). Discovered rows are not counted as parents
- verified repairs
        - R3-F1: Doctor's handoff assessment calls the writer's own `HandoffPlan::reference_in_scope`. A reference through a link that leaves the scope is a manual error naming the target, and the other scope stays repairable
        - R3-F2: `config` follows every valid user and repository `.darkmatter.yaml` reference through Darkmatter's passive reader, whatever state the Claudine counterpart is in. A broken target is a located `config` error (exit 1); the permissions row points to it instead of repeating it
        - R3-F3: the remedy for a repairable handoff is `claudine doctor --fix` in both scopes. The missing-user-file remedy now says setup publishes first (R3-F3.x1)
- open findings: none
- decisions recorded (ledger and progress only; none met the worth-the-author's-time test)
        - R3-F1: expose the writer's existing scope check rather than add a second function or a richer error
        - R3-F2: `config` always carries the broken-target diagnostic and the permissions row refers to it. The alternative, having `config` predict the handoff, was rejected because it reintroduces the coupling
        - R3-F3: follow the reviewer's smaller repair, `--fix` as the remedy, with no publisher preflight in the report
- changed files this cycle
        - claudine library: `lib/src/config/setup.rs` (R3-F1), `lib/src/doctor/permissions.rs` (R3-F1, R3-F2, R3-F3), `lib/src/doctor/config.rs` (R3-F2, R3-F3.x1), `lib/src/doctor/registration.rs` (R3-F2 pattern update), `lib/src/doctor/permissions_tests.rs` (R3-F1, R3-F2.c1, R3-F3), `lib/src/doctor/config_tests.rs` (R3-F2, R3-F3.x1)
        - claudine CLI: `cli/tests/l1/doctor.rs` (R3-F2 wiring, two summary expectations)
        - darkmatter: `lib/src/config/load.rs` (`ConfigFile::permissions_position`, R3-F2)
        - docs and skills: `claudine/docs/cli/doctor.md` (all three), `claudine/docs/topics/configuration.md` (R3-F1), `.claude/skills/claudine/SKILL.md` (R3-F1, R3-F2). No caller skill source changed
- final verification (after the last change)
        - claudine `just test`: 13002 passed, 16 skipped, none at or above 5 s (all findings)
        - darkmatter `just test`: 17884 passed, 10 skipped, none at or above 5 s (R3-F2 accessor)
        - `just lint` in claudine and darkmatter: exit 0
        - `just check-tier-coverage claudine` and `darkmatter`: exit 0
        - `just cross-check claudine --os windows`: nextest reported 7185 passing assertions and exit 0. The L1 timing gate failed: `composition::preflight::tests::a_saved_choice_answers_the_next_run_without_a_prompt_and_is_not_written_twice` took 5.003 s on build-win, and the package run does not record qualifying host load. Review 2 also recorded it at 5.0 s on the Windows rig. The new link tests are `#[cfg(unix)]`, like the existing link tests
        - the first attempt with a 590 s shell timeout was cut off at the time limit, not by a failure; the rerun completed

### Review 4 verification (2026-11-24)

- The review 3 handoff progress note said that Doctor's registration row
  reports the obstruction in every setup failure fixture. The linked
  publisher lock is an exception when the bundle is healthy. I reran
  `a_linked_publisher_lock_hidden_by_a_healthy_registration_row_leaves_the_handoff_to_doctors_fix`.
  Setup stopped before the handoff, Doctor completed it, and the
  registration row stayed `ok`. The progress note now reflects both outcomes.
- The Windows verification was listed with successful gates. The archive
  reported exit 0 and 7,185 passing test assertions, but also recorded one
  L1 test at 5.003 s. I checked the raw nextest output and reran the test
  alone on the Windows rig, where it took 2.83 s. The package run did not
  document elevated host load, so its timing gate remains red. The review 2
  Windows timing note now also distinguishes assertions from the timing gate.
- The review 3 scope sweep said it found no sibling defect in the
  registration repair. A published-home CLI probe showed that the user
  registration edit failed when the launch repository shared the user
  configuration, even though the same user permissions and block-style
  register list repaired with a distinct repository file. A separate
  CLI probe showed that inserting into a nonempty inline register list
  failed, while the equivalent block list repaired. The sweep note now
  says which controls it established.

## Implementation of Review Findings #4

> **started at:** 2026-11-24T08:43:46+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/features/2026-11-22-claudine-doctor/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- findings are handled serially, one subagent per finding; each subagent reports log items back to the orchestrator, which records them below
- the review's recurrence list (R4-F1 and R4-F2 as siblings of R2-F7 and R3-F1; R4-F4 as a sibling of R3-F3) is the minimum sweep scope

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R4-F1 | The registration writer validates an aliased user file as repository policy | fixed | all required children verified; `darkmatter::config::init::commit_checked` now takes the caller's `(path, scope)` list and never infers scope from the target's location |
| R4-F1.1 | Doctor observation, eligibility, repair, verification: repository `.darkmatter.yaml` linked to the user file; user edit completes and verifies | fixed | `doctor::registration::tests::the_entry_repair_edits_the_user_file_wherever_doctor_runs` (linked repository `.darkmatter.yaml`, user-only `~/notes/*.md` grant and shared-policy control, comment and other host kept, entry verified); fails with the old inference |
| R4-F1.2 | Same when the home is the Git root, inside and outside it: user scope kept | fixed | same test with the home as the Git root, launched inside it |
| R4-F1.3 | Setup's shared registration writer: registers the user manifest under the same aliases | fixed | `installed_registration_registers_in_an_aliased_user_file_despite_a_permissions_conflict`: handoff reports `Conflict`, `registration` is `Updated` |
| R4-F1.c1 | Doctor permissions handoff keeps its explicit scope and alias skip | clean | doctor permissions tests unchanged and passing (361 in the `doctor::`/setup/registration filter); alias skip kept |
| R4-F1.c2 | Hidden publication, passive entry lookup, already-registered bundle stay independent of scope inference | clean | `registrations_command` CLI tests 16/16 and doctor entry-lookup tests pass unchanged; publication never writes configuration |
| R4-F1.c3 | Darkmatter `register_manifest` API uses the same explicit scope rule | fixed | `register_manifest` passes its own scope; Darkmatter `config_init::scoped_registration_writes_judge_an_aliased_user_file_as_the_user_file` |
| R4-F1.x1 | Discovered: Claudine handoff in setup mode inferred scope through `commit_checked` | fixed | now uses the plan's scope; control `darkmatter_setup::setup_hands_off_an_aliased_user_file_as_the_user_file` (passed before too) |
| R4-F1.x2 | Discovered: Darkmatter `init` judged each file by its path | fixed | each file judged by its own scope; `config_init::init_user_completes_an_aliased_user_file_as_the_user_file` fails with the old code |
| R4-F2 | Doctor offers repairs that the YAML editor refuses | fixed | all required children verified; one dry-run rule, `darkmatter::config::write::TargetChange::edit_refusal`, applied to the exact planned edits before a row is offered as fixable |
| R4-F2.1 | Registration insert/replace/dedupe in a nonempty inline list: supported edit or file- and setting-specific manual remedy before publication; layout kept until verified | fixed | `doctor::registration` `an_entry_edit_the_editor_cannot_make_leaves_the_registration_manual::{inline_list_append, inline_list_replace, inline_list_remove}`; `repair::tests::an_entry_edit_the_editor_refuses_stops_the_repair_before_publication::{same}` (nothing published, legacy kept); block controls `the_same_entry_edit_in_block_form_is_repaired::{block_list_*}`; Darkmatter `config_init::replace_manifest_refusal_predicts_the_registration_edit_before_publication`; CLI `doctor::an_inline_register_list_is_a_manual_registration_finding_before_and_after_fix` |
| R4-F2.2 | Registration in an inline root mapping: layout prerequisite reported before an edit | fixed | `…leaves_the_registration_manual::inline_root`, `…stops_the_repair_before_publication::inline_root`, control `the_same_entry_edit_in_block_form_is_repaired::block_root` |
| R4-F2.3 | Handoff writing Claudine's inline root, both scopes: prerequisite reported; original policy stays | fixed | `permissions_tests` `layout_cases!`: `{user,repository}_transfer_into_an_inline_claudine_root_is_manual` (both files unchanged), controls `{user,repository}_transfer_into_a_block_claudine_root_is_repaired` |
| R4-F2.4 | Handoff writing Darkmatter's inline root, both scopes: prerequisite reported; all-candidate check kept; other scope proceeds | fixed | `{user,repository}_reference_only_into_an_inline_darkmatter_root_is_manual` (equal Claudine block), `{user,repository}_two_file_handoff_into_an_inline_darkmatter_root_is_manual` (no Claudine block, Claudine file unchanged), `a_scope_blocked_by_its_layout_leaves_the_other_scope_repairable` |
| R4-F2.c1 | Registration omitted/null/`[]`/unchanged inline list keep succeeding | clean | `replace_manifest_refusal_predicts_…` omitted/null/`[]`/inline-already-current cases; `an_inline_list_that_needs_no_edit_is_kept_byte_for_byte_while_the_retired_layout_goes`; existing invalid-field refusals pass |
| R4-F2.c2 | Handoff inline policy value, empty `{}` root, unchanged inline Claudine root keep succeeding | clean | `{user,repository}_transfer_into_a_block_claudine_root_is_repaired` (inline `permissions` value), `{user,repository}_empty_inline_claudine_root_takes_its_first_key`, `{user,repository}_reference_only_leaves_an_inline_claudine_root_alone` |
| R4-F2.c3 | Setup's registration and handoff diagnostics preserved and share the edit rules | clean | setup flow unchanged, same staging; `installed_registration_setup_reports_an_inline_register_list_and_keeps_the_retired_layout` pins the located writer error and the kept legacy layout |
| R4-F2.x1 | Discovered: anchored, aliased, or tagged nodes the edit reaches are refused by the same editor | fixed | same `layout_of` classification; `replace_manifest_refusal_predicts_…` anchored block list case |
| R4-F3 | Recorded native Windows L1 tests exceeded the timing gate | fixed | all ten required rows and four discovered rows verified in native Windows package runs: `just cross-check claudine --os windows` 7,220 passed, none at or above 5 s; final `just cross-check darkmatter --os windows` 12,989 passed, none at or above 5 s |
| R4-F3.1 | Claudine saved approval choice test under 5 s in the Windows package run | fixed | split into `…::exact`/`…::template`/`…::command` with one shared request and guard `every_saving_choice_has_a_saved_choice_test`; Windows package run 1.168/1.170/1.168 s (was 5.003 s); Linux 0.23–0.25 s |
| R4-F3.2 | Darkmatter glob implementation guard | fixed | `source_scan::production_sources_where` narrows blanking to candidate files (planted cases fail without each pruning branch); final Windows package run 1.513 s (was 10.6 s) |
| R4-F3.3 | Darkmatter body shell trigger test | fixed | lib `test_support::request_in` anchors the empty anchor on the request directory; final Windows package run 0.435 s (was 9.5–9.6 s pair) |
| R4-F3.4 | Darkmatter frontmatter shell trigger test | fixed | same helper; final Windows package run 0.421 s |
| R4-F3.5 | Darkmatter nested trigger gate test | fixed | same helper; final Windows package run 0.218 s (was 7.5 s) |
| R4-F3.6 | Darkmatter environment anchor test | fixed | helper plus fixture/request hoisted out of the case loop; Windows package run `env_anchor_preserves_every_unsafe_category` 0.171 s in the final run (was 6.0–7.8 s group) |
| R4-F3.7 | Darkmatter home anchor test | fixed | `home_anchor_preserves_every_unsafe_category` 0.160 s in the final Windows run |
| R4-F3.8 | Darkmatter repository anchor test | fixed | `repo_anchor_preserves_every_unsafe_category` 0.113 s in the final Windows run |
| R4-F3.9 | Darkmatter recursive trigger merge test | fixed | split into 5 tests by nesting position reusing the loaded bundle (`Case::with_document`); final Windows package run at most 1.026 s per split test (was 6.7 s) |
| R4-F3.10 | Darkmatter repository boundary test | fixed | integration `request_support::request_in` anchors the options; final Windows package run 0.265 s (was 5.6 s) |
| R4-F3.c1 | Linux approval test and Doctor slices keep assertions and L1 selection | clean | assertions and L1 selection kept (no tier marker; `just check-tier-coverage` claudine and darkmatter 0 stranded); Linux claudine `just test` 13,038 passed and darkmatter 17,907 passed, none at or above 5 s |
| R4-F3.x1 | Discovered: sibling preflight tests rebuilt a request per row | fixed | `each_choice_at_a_resolved_site_saves_its_row`, `a_failed_save_at_a_resolved_site_still_approves_the_run`, `a_template_rule_approves_a_resolved_site_by_shape_and_an_exact_rule_wins` share one request (Linux 0.72/0.53/0.55 → 0.21/0.23/0.22 s) |
| R4-F3.x2 | Discovered in the first Windows package run: `global_catalog::the_old_embedded_baseline_is_deleted_and_never_included` 6.876 s | fixed | new shared `darkmatter/lib/tests/git_prefilter` skips tracked entries Git does not list as matching; an untracked offending file still fails. A tracked symbolic link with an offending target can be skipped (see review 5); final Windows package run 0.446 s |
| R4-F3.x3 | Discovered: `shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively` 6.408 s | fixed | Git prefilter on `$schema`, no YAML decode without `$schema`, split into 7 group tests with guard `inline_groups_cover_the_scope_once` (per-group floors sum to the old 500); same 894 lines of found schemas; final Windows package run at most 0.979 s |
| R4-F3.x4 | Discovered: `shipped_schema_corpus::shipped_markdown_named_type_references_resolve` 6.020 s | fixed | same prefilter and 7-group split (floors sum to the old 4); final Windows package run at most 0.836 s |
| R4-F4 | The guide says Doctor reports a publisher lock that it can leave unreported | fixed | `claudine/docs/cli/doctor.md` (Darkmatter permissions section) now says path obstructions stay an `error` in `registration` while a linked publisher lock beside a healthy registration stays `ok`, matching `permissions_tests::a_linked_publisher_lock_hidden_by_a_healthy_registration_row_leaves_the_handoff_to_doctors_fix` (Severity::Ok) and the four `Severity::Error` cases of `assert_doctor_fix_hands_off_past`; sweep of `claudine/docs`, the repo skill, and caller skills found no other such claim |

### Progress

- starting the work on 'R4-F1 The registration writer validates an aliased user file as repository policy' at 08:44:30
        - defect class: a write whose scope the caller already fixed re-derives the scope from the target's location, so the read, validation, write, and verification of one target disagree
        - shared rule: `darkmatter::config::init::commit_checked(transaction, context, scopes, other)` judges each target by the scope its caller names; a file named under both scopes is judged as the user file (the spec's alias rule); a target nobody names is refused. The path comparison against `{repo-root}/.darkmatter.yaml` is gone
        - sites swept: `init`, `register_manifest`, `replace_manifest_registration` (setup), `replace_manifest_registration_existing` (doctor repair), and Claudine's `commit_permissions_handoff` (doctor mode already explicit; setup mode now uses the plan's scope) were fixed. Claudine `doctor/config.rs` and `doctor/permissions.rs` `same_file` uses only detect read aliases and are clean. Darkmatter `load.rs`, `load/reference.rs`, `entries.rs`, `migration.rs`, and `widening.rs` take scope from the layer they read and are clean
        - decision: a required `scopes` argument rather than a second scoped function (which would leave the inferring fallback) or a closure per writer (the same logic five times); unnamed targets are refused rather than defaulted to user
        - tests: Darkmatter `config_init.rs` `scoped_registration_writes_judge_an_aliased_user_file_as_the_user_file`, `init_user_completes_an_aliased_user_file_as_the_user_file`, `commit_checked_judges_each_target_by_the_scope_its_caller_names`; Claudine `doctor/registration_tests.rs::the_entry_repair_edits_the_user_file_wherever_doctor_runs`, `installed_registration.rs::installed_registration_registers_in_an_aliased_user_file_despite_a_permissions_conflict`, `darkmatter_setup.rs::setup_hands_off_an_aliased_user_file_as_the_user_file` (control). The subagent confirmed the first five fail when the path-based inference is restored, with the reviewer's "reads from the home directory, which a repository cannot grant" refusal
        - docs: `darkmatter/docs/topics/host-registration.md` (the scope passed is the scope checked), `init.rs` module and function docs, `setup.rs::commit_permissions_handoff` doc
        - scoped verification: darkmatter `test(/init/) | test(/registration/) | test(/config/)` 616/616 (slowest 0.435 s); darkmatter-cli init/config 64/64; claudine doctor/setup/registration filter 361/361 (slowest 2.570 s); `just test-cli doctor` 25/25, `setup` 44/44, `registrations` 16/16; `just lint` darkmatter and claudine exit 0
- work completed for 'R4-F1 The registration writer validates an aliased user file as repository policy' at 08:59:39
- starting the work on 'R4-F2 Doctor offers repairs that the YAML editor refuses' at 08:59:39
        - defect class: repair eligibility is inferred from schema validity while the format-preserving editor has its own layout prerequisites (flow collections, flow roots, anchors, aliases, tags)
        - shared rule: a row is fixable only when a dry run of its exact planned edits on the observed bytes succeeds. Darkmatter `TargetChange::edit_refusal()` runs the editor staging uses and writes nothing; `init::replace_manifest_refusal` plans the registration edit before publication. Claudine maps a layout refusal to `EditLayout::{Root, Setting}` (`registration/inspect.rs::{edit_layout, layout_of, entry_layout}`); the handoff uses `HandoffPlan::layout_obstacle()` over both planned edits
        - registration: an infeasible entry edit makes the whole registration repair manual, checked in assessment and again before and under the publisher lock, so nothing is published or cleaned up. The writer's in-transaction check is kept (`a_list_rewritten_inline_after_the_checks_is_still_refused_by_the_writer`)
                - alternatives rejected: publish and remove extras but skip the entry and cleanup, or clean up partially; both break the rule that legacy cleanup follows a verified entry
        - handoff: an infeasible Claudine or Darkmatter edit makes that scope manual with both files unchanged; the other scope still repairs; the finding stays located at the scope's `.darkmatter.yaml` and names the file whose layout blocks it. The remedy says to rewrite the named root or setting in block form and rerun `claudine doctor --fix`; no values are quoted
        - sweep: doctor registration check and repair and doctor permissions `assess` fixed; setup handoff and `setup_claudine` registration clean (same staging, located error, cleanup withheld); `persist.rs` config TUI and `config set` edits clean (not a doctor offer; errors surface); `json_conversion` clean (generated text); `init` and `register_manifest` clean (not doctor-offered, located errors); no other doctor repair exists
        - tests: Darkmatter `config_init::replace_manifest_refusal_predicts_the_registration_edit_before_publication` (12 cases), `config_write::edit_refusal_predicts_the_commit_for_the_expected_bytes`, `config_write::edit_refusal_cannot_judge_bytes_it_does_not_know`; Claudine repair, doctor registration, and doctor permissions cases listed in the ledger; setup `installed_registration_setup_reports_an_inline_register_list_and_keeps_the_retired_layout`; CLI `doctor::an_inline_register_list_is_a_manual_registration_finding_before_and_after_fix`. With the rule disabled, all 16 instance tests and the CLI test failed, and the 12 controls passed either way
        - docs: `claudine/docs/cli/doctor.md` (new "Files the repairs cannot edit in place" section), `claudine/docs/topics/configuration.md`, `darkmatter/docs/topics/config-writes.md`, `darkmatter/docs/topics/host-registration.md`, `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/SKILL.md` and `library-surfaces.md`; no caller skill changed
        - scoped verification: darkmatter init/yaml_edit/config 482/482 (slowest 0.41 s); claudine doctor/setup/registration 390/390 (slowest 2.46 s); `just test-cli doctor` 26/26; `just test-cli setup` 44/44; `just lint` darkmatter and claudine clean
- work completed for 'R4-F2 Doctor offers repairs that the YAML editor refuses' at 09:17:17
- starting the work on 'R4-F3 Recorded native Windows L1 tests exceeded the timing gate' at 09:17:17
        - defect class: tests repeat expensive fixture setup, repository discovery from the process directory, or whole-repository scanning, so they exceed the L1 budget under package parallelism
        - two subagents ran in parallel (Claudine slice, Darkmatter slice); their items are grouped below. The Windows package-run measurement is done by the orchestrator afterward
        - Claudine slice (R4-F3.1)
                - cause: the test-only `request()` helper rebuilt `ComposeRequest::with_context` for each of nine passes, and each build reloads the passive Darkmatter configuration (90–200 ms on Linux); production code does no redundant work here
                - change: `resolve_site` takes a shared `&ComposeRequest`; the test is split by `saved_choice_tests!` into `…::exact`, `…::template`, `…::command` (one request each, same four assertions) with guard `every_saving_choice_has_a_saved_choice_test`
                - Linux: 1.280 s → 0.23–0.25 s per generated test
                - siblings R4-F3.x1: `each_choice_at_a_resolved_site_saves_its_row` 0.724 → 0.211 s, `a_failed_save_at_a_resolved_site_still_approves_the_run` 0.531 → 0.231 s, `a_template_rule_approves_a_resolved_site_by_shape_and_an_exact_rule_wins` 0.554 → 0.221 s; `each_choice_at_a_template_site_saves_its_row` builds no request and was left alone
                - verification: `cargo nextest run -p claudine -E 'test(/composition::preflight::tests/)'` 45/45; `just lint` exit 0
        - Darkmatter slice (R4-F3.2–R4-F3.10)
                - R4-F3.2 glob guard: `production_sources` blanked 2,024 files (38.8 MB) at opt-level 0; new `source_scan::production_sources_where(src, keep)` blanks only files whose raw text could hold a hit or mark one test-only; two planted cases (test module declared from a parent, reached through `#[path]`) each fail when their pruning branch is removed. 1.41 → 0.45 s alone
                - R4-F3.3–.5 `invalid_triggers`: each request discovered the repository from the process directory (about 70 ms × 6 requests); lib `test_support::request_in` now anchors an empty anchor on the request directory. 0.50/0.50/0.37 → 0.06/0.06/0.07 s
                - R4-F3.6–.8 `link_normalization` (Windows-only module): the same lookup ×8 cases; covered by the helper and fixture/request hoisted out of the case loop; compiled on Linux with the cfg temporarily disabled, measured only on Windows
                - R4-F3.9 recursive trigger merge: 60 cases in one test, each reloading the host bundle; split into 5 tests by nesting position and `Case::with_document` reuses the loaded bundle. 0.34 s → 0.07 s each
                - R4-F3.10 repository boundary: integration `request_support::request_in` anchors the options; 0.555 → 0.035 s; all five `file_tree_roots` tests now 0.02–0.03 s
                - verification: scoped filter 130 passed; whole `-p darkmatter` 12,592 passed, none SLOW; source-scan guards in darkmatter-cli, claudine, and claudine-cli 363 passed; `just lint` clean
                - drift found: `ComposeContext::capture_minimal`'s doc said every `ComposeRequest` re-anchors its empty anchor, but only `ComposeRequest::prepare` does; `with_context` keeps the options' context as given. Per the repository rule the code is taken as correct: the orchestrator corrected the doc comment in `darkmatter/lib/src/markdown/compose/context/runtime.rs`, and the subagent recorded the trap in `.claude/skills/darkmatter/requests.md`. Changing `with_context` would alter `ctx.cwd`/`current.repo` for library callers and is not needed to repair this finding
- starting the work on 'R4-F4 The guide says Doctor reports a publisher lock that it can leave unreported' at 09:39:06
        - defect class: documentation claims a diagnostic is reported in every listed case when one listed case is deliberately left `ok`
        - checked against the test helper: the shared folders, registration folder, manifest, and retired member cases assert `registration` = `error`; the publisher lock case asserts `ok`
        - repair: the guide sentence now separates the registration-path obstructions from the publisher lock and keeps the healthy-lock example; sweep of `claudine/docs/topics/schemas.md`, `.claude/skills/claudine/SKILL.md`, and `claudine/docs/use-skills/` found no other claim about the row
- work completed for 'R4-F4 The guide says Doctor reports a publisher lock that it can leave unreported' at 09:39:29
- starting phase verification at 09:39:29: full `just test` and `just lint` in claudine and darkmatter, `just check-tier-coverage`, then the Windows package runs
        - Linux: claudine `just test` 13,038 passed, 16 skipped, none at or above 5 s; `just lint` exit 0; `just check-tier-coverage claudine` exit 0, 0 stranded
        - Linux: darkmatter `just test` 17,894 passed, 10 skipped, none at or above 5 s; `just lint` exit 0; `just check-tier-coverage darkmatter` exit 0, 0 stranded
        - Windows `just cross-check claudine --os windows`: 7,220 passed, 6 skipped, none at or above 5 s. The R4-F3.1 tests ran at 1.168/1.170/1.168 s in the package run (was 5.003 s)
        - Windows `just cross-check darkmatter --os windows`: 12,976 passed, 66 skipped. Every R4-F3.2–.10 test was at or under 1.94 s in the package run (glob guard 1.243 s, body/frontmatter shell triggers 0.437/0.432 s, nested gate 0.223 s, anchors 0.076–0.092 s, recursive merge cases at most 1.934 s, repository boundary 0.205 s). The timing gate was still red on three other tests: `global_catalog::the_old_embedded_baseline_is_deleted_and_never_included` 6.876 s, `shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively` 6.408 s, `shipped_schema_corpus::shipped_markdown_named_type_references_resolve` 6.020 s. On Linux they take 0.135/0.513/0.576 s alone. Recorded as discovered rows R4-F3.x2–x4 and repaired below
        - discovered rows R4-F3.x2–x4 (one subagent, 10:01–10:22)
                - cause: every first open of a file on build-win costs about 1.3–1.8 ms; the `global_catalog` test opened all 1,400 `.rs` files in the darkmatter area, and each Markdown corpus test walked 3,848 files in 16 directories and YAML-decoded every frontmatter
                - splitting alone was measured and was not enough (each piece still 3.5–4.5 s, because the opens queue on a shared resource); the number of opens had to drop
                - change: `current_root_migration_guard.rs`'s Git prefilter moved into a shared `darkmatter/lib/tests/git_prefilter/mod.rs` and is used by all three; the Markdown tests skip YAML decoding without `$schema` and are split into 7 group tests each, with guard `inline_groups_cover_the_scope_once`; corpus-wide floors became per-group floors whose sums equal the old minimums; each file literal stays inside its own `#[test]`
                - Windows fact recorded in `.claude/skills/os/windows.md`: build-win stores `.claude/skills` symlinks as plain files, so the `skills` group finds 0 schemas there (floor 0, matching the old global check); also recorded that splitting a file-open-bound test does not help on Windows
                - sibling checked, unchanged: `loadable_shipped_schemas_resolve_and_lower` (2.6 s on Windows filtered; its time is schema resolution, not a walk; under the gate in the final package run)
        - rerun after the last change: darkmatter `just test` 17,907 passed, 10 skipped, none at or above 5 s; `just lint` exit 0; `just check-tier-coverage darkmatter` 0 stranded; `just cross-check darkmatter --os windows` 12,989 passed, 66 skipped, none at or above 5 s (slowest test 4.209 s, `expression_recovery_data::allowed_transport_failure_recovers::connection_refused`, untouched and under the gate). The R4-F3.x2–x4 tests ran at 0.446 s and at most 0.979 s per split test
        - claudine was not rerun after the x2–x4 repair because those changes are Darkmatter test files only, outside every Claudine target
- work completed for 'R4-F3 Recorded native Windows L1 tests exceeded the timing gate' at 10:29:00 (the Windows rows were closed in the phase verification above)
- phase verification completed at 10:29:00

### Completion Report

- **outcome: complete.** Started 08:43:46 UTC, finished about 10:30 UTC, about 1 h 46 min elapsed
- parent findings: total 4 = fixed 4 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: 26 required rows (19 fixed, 7 clean); 8 discovered rows (all fixed: R4-F1.x1–x2, R4-F2.x1, R4-F3.x1–x4). Discovered rows are not counted as parents
- verified repairs
        - R4-F1: `darkmatter::config::init::commit_checked` judges each target by the scope its caller names; a file named under both scopes is the user file. Doctor's repair, setup's registration, `register_manifest`, `init`, and both handoff modes no longer infer scope from the path
        - R4-F2: a repair is offered only when a dry run of its exact planned edits succeeds (`TargetChange::edit_refusal`). An inline layout the edit would reach makes the row manual and names the file and setting with a block-form remedy. Registration is checked before publication, and the handoff per scope
        - R4-F3: all ten recorded tests and three more found in the first Windows package run are under the gate in native Windows package runs; repeated request construction, repository discovery from the process directory, and repository-wide file opens were removed
        - R4-F4: the command guide separates the registration-path obstructions, which the row reports, from a linked publisher lock beside a healthy registration, which stays `ok`
- open findings: none
- decisions recorded in the ledger and progress notes; none met the worth-the-author's-time test, so `human-review` is unchanged
        - R4-F1: a required `scopes` argument; an unnamed target is refused rather than defaulted
        - R4-F2: an infeasible entry edit makes the whole registration repair manual so legacy cleanup always follows a verified entry; anchors, aliases, and tags are treated like flow collections
        - R4-F3: drifted `capture_minimal` doc comment corrected to match `with_context`, whose behavior was kept
        - the subagents used `--no-fail-fast` once each for the R4-F1 and R4-F2 red-before-green proof runs (filtered, local); no verification run used it
- changed files this cycle
        - darkmatter library: `lib/src/config/init.rs` (R4-F1, R4-F2), `lib/src/config/write.rs` (R4-F2), `lib/src/markdown/compose/context/request.rs` (R4-F3 `test_support` only), `lib/src/markdown/compose/context/runtime.rs` (R4-F3 doc drift), `lib/src/markdown/compose/link_normalization.rs` (R4-F3 Windows test module)
        - darkmatter tests: `lib/tests/l1/config_init.rs` (R4-F1, R4-F2), `lib/tests/l1/config_write.rs` (R4-F2), `lib/tests/l1/glob_implementation_guard.rs`, `lib/tests/l1/host_trigger_merge.rs`, `lib/tests/request_support/mod.rs` (R4-F3), `lib/tests/git_prefilter/mod.rs` (new), `lib/tests/l1/main.rs`, `lib/tests/l1/current_root_migration_guard.rs`, `lib/tests/l1/global_catalog.rs`, `lib/tests/l1/shipped_schema_corpus.rs` (R4-F3.x2–x4), `cli/tests/common/source_scan.rs` (R4-F3.2)
        - claudine library: `lib/src/config/setup.rs` (R4-F1, R4-F2), `lib/src/composition/registration.rs`, `lib/src/composition/registration/inspect.rs`, `lib/src/composition/registration/repair.rs`, `lib/src/doctor/registration.rs`, `lib/src/doctor/permissions.rs` (R4-F2)
        - claudine tests: `lib/src/doctor/registration_tests.rs` (R4-F1, R4-F2), `lib/src/doctor/permissions_tests.rs`, `lib/src/composition/registration/repair_tests.rs`, `cli/tests/l1/doctor.rs` (R4-F2), `lib/tests/l1/installed_registration.rs` (R4-F1, R4-F2), `lib/tests/l1/darkmatter_setup.rs` (R4-F1), `lib/src/composition/preflight/tests.rs` (R4-F3.1)
        - docs and skills: `claudine/docs/cli/doctor.md` (R4-F2, R4-F4), `claudine/docs/topics/configuration.md` (R4-F2), `darkmatter/docs/topics/host-registration.md` (R4-F1, R4-F2), `darkmatter/docs/topics/config-writes.md` (R4-F2), `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/SKILL.md`, `.claude/skills/darkmatter/library-surfaces.md` (R4-F2), `.claude/skills/darkmatter/requests.md`, `.claude/skills/darkmatter/testing.md`, `.claude/skills/os/windows.md` (R4-F3). No caller skill changed, so `just publish-skills` was not needed
- final verification (after the last change)
        - claudine `just test`: 13,038 passed, 16 skipped, none at or above 5 s (R4-F1, R4-F2, R4-F3.1, R4-F4)
        - darkmatter `just test`: 17,907 passed, 10 skipped, none at or above 5 s (R4-F1, R4-F2, R4-F3)
        - `just lint` in claudine and darkmatter: exit 0
        - `just check-tier-coverage claudine` and `darkmatter`: exit 0, 0 stranded
        - `just cross-check claudine --os windows`: 7,220 passed, 6 skipped, none at or above 5 s (R4-F3.1; cross-OS check of R4-F1/R4-F2 code)
        - `just cross-check darkmatter --os windows`: 12,989 passed, 66 skipped, none at or above 5 s (R4-F3.2–.10, x2–x4; cross-OS check of R4-F1/R4-F2 code)
        - no performance measurement was deferred


### Review 5 verification (2026-11-24)

- The review 4 ledger's scan description read as a guarantee about the
  bytes the test opens. The helper uses Git's negative match instead.
  In an isolated repository I staged symbolic links to fixtures outside
  the scanned directories, then unstaged the same links without changing
  any file. The compiled Darkmatter old-baseline guard, inline-schema
  parser test, and named-type resolution test all passed with the links
  tracked and reported the planted defects with the links untracked.
  The ledger now records that gap alongside the measured speedup.
- Several timing rows gave numbers from the first Windows Darkmatter
  package run beside a summary of the final run. I checked the individual
  nextest lines in `/tmp/r4/win-darkmatter-2.log` and updated those rows to
  give the final measurements. The earlier results stay in the progress
  history. Both final package logs still report zero tests at or above
  five seconds.
