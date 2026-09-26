---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-25-worktree-file/spec.md
plan: worktree/fixes/2026-09-25-worktree-file/plan.md
implemented_by: codex/default
started_phase: 1
source_files_during_phase_1:
    - worktree/fixes/2026-09-25-worktree-file/spike_git.py
    - worktree/fixes/2026-09-25-worktree-file/spike_windows.ps1
docs_updated_during_phase_1:
    - worktree/fixes/2026-09-25-worktree-file/plan.md
    - worktree/fixes/2026-09-25-worktree-file/spec.md
docs_created_during_phase_1:
    - worktree/fixes/2026-09-25-worktree-file/spikes.md
    - worktree/fixes/2026-09-25-worktree-file/implementation-log.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/windows.md
packages:
    - worktree
    - worktree-cli
source_files_during_phase_2:
    - worktree/cli/src/exit.rs
    - worktree/lib/src/cache.rs
    - worktree/lib/src/compare.rs
    - worktree/lib/src/copy_record.rs
    - worktree/lib/src/error.rs
    - worktree/lib/src/git.rs
    - worktree/lib/src/include/copy.rs
    - worktree/lib/src/include/mod.rs
    - worktree/lib/src/include/rules.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/remove/inventory.rs
    - worktree/lib/src/remove/test_support.rs
docs_updated_during_phase_2:
    - docs/dependencies.md
    - worktree/fixes/2026-09-25-worktree-file/implementation-log.md
    - worktree/fixes/2026-09-25-worktree-file/plan.md
    - worktree/fixes/2026-09-25-worktree-file/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/os/windows.md
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_3:
    - worktree/lib/src/worktree.rs
    - worktree/cli/src/commands/create.rs
    - worktree/cli/tests/create_include.rs
docs_updated_during_phase_3:
    - worktree/fixes/2026-09-25-worktree-file/plan.md
    - worktree/fixes/2026-09-25-worktree-file/implementation-log.md
    - worktree/fixes/2026-09-25-worktree-file/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_4:
    - worktree/lib/src/remove/included.rs
    - worktree/lib/src/remove/inventory.rs
    - worktree/lib/src/remove/handoff.rs
    - worktree/lib/src/remove/mod.rs
    - worktree/lib/src/copy_record.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/src/commands/remove/policy.rs
    - worktree/cli/src/commands/remove/report.rs
    - worktree/cli/tests/remove.rs
docs_updated_during_phase_4:
    - worktree/fixes/2026-09-25-worktree-file/plan.md
    - worktree/fixes/2026-09-25-worktree-file/implementation-log.md
    - worktree/fixes/2026-09-25-worktree-file/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/worktree/SKILL.md
---

# Implementation Log for 2026-09-25-worktree-file (5 phases)

## Phase 1

- Confirmed R1 and R2 from the author's spec decisions. Confirmed R3 and
  R5–R12; amended R4 to use a registration nonce and corrected R7 so
  non-UTF-8 rule bytes are allowed. No production code was changed.
- S1 target: the include candidate and standard-ignore intersection across
  Git rule forms, representations, and repository boundaries. `spike_git.py`
  asserts the selected set and the absolute fallback rules location on macOS
  and Linux. `spike_windows.ps1` asserts the NUL pipeline on Windows. The
  initially failing Windows PowerShell text-pipeline probe exposed a BOM;
  replacing that with byte streams made it pass. Directory-pattern results
  require a Phase 2 kind filter. A second Windows probe initially failed
  because Git traversed a junction and listed its child; Phase 2 must inspect
  every candidate ancestor for reparse points.
- S2 target: clone capability, mode, and independence. `spike_git.py` runs
  native clone commands: APFS succeeded and preserved `0600`; Linux's temp
  volume denied cloning. `spikes.md` records the selected crate, dependency
  cost, publication rule, and Windows caveat.
- S3 target: stale records after same-path re-registration. Both probes assert
  that Git reuses the admin directory name and removes a marker in that
  directory. A random marker is the chosen identity contract.
- S4 target: Windows link and junction handling. `spike_windows.ps1` asserts
  junction reparse attributes and reports symlink creation. The host permits
  links, so the error-1314 denial path is grounded in platform documentation
  and remains a Phase 2 injected failure case.
- No crate, CLI, parser, or persisted value was changed in Phase 1; the
  shipped-artifact corpus, end-to-end command, and read/write/read tests
  belong to the integration phases where those behaviors are implemented.
- Phase 1 targeted probes: `python3 spike_git.py` on macOS and build-linux;
  `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File
  spike_windows.ps1` on build-win-native. All passed after the probe fix.
- Broader gates: `just test` in `worktree/` passed (331/331 selected L1
  tests; 17 tier-filtered tests skipped). `just lint` passed for `worktree`
  and `worktree-cli`. No pre-existing failures were observed. `just test-l2`
  and cross-OS crate tests were not run because Phase 1 changes no package
  code or terminal behavior; the three probe hosts supplied the relevant
  platform evidence.

## Phase 2

- Test map before implementation: Git byte helper → NUL stdin and exact stdout test; rules/discovery → fixture repository tests for Git's two-stage intersection, rule states, path variants, and repository boundaries; comparison → counted reads and same-size edits, links, kind, missing/error tests; record store → repeated read/write/read, corruption/version/identity/prune/concurrent-key tests; copy engine → filesystem outcome tests for clone/fallback, no-replace, modes, links, conflicts, and cleanup. All are L1 library tests compiled by the worktree lib target. Phase 2 adds no CLI behavior or shipped parser artifact; the fixture's actual `.worktreeinclude` is exercised through the normal Git invocation.
- Shared scaffolding: `git_from_bytes` now accepts optional NUL-delimited stdin and drains output concurrently; `git_from_bytes_preserves_nul_input_and_output` tests the public byte result. Added exit-1 handling only for `check-ignore`'s valid no-match result. The two new include errors map to CLI exit 1. `TestRepo` gained include, ignored-file, linked-worktree, global-exclude, and injectable cache-path helpers. Added `reflink-copy` and a Windows-only `windows-sys` dependency; updated `Cargo.lock` and `docs/dependencies.md`.
- Include membership: `rules_states_and_git_intersection`, `include_pattern_without_standard_ignore_selects_nothing`, `global_nested_and_path_variants`, `excluded_parent_and_nested_repository_stay_out`, `anchored_rule_and_non_utf8_rule_content_use_git_matching`, `rules_symlink_must_resolve_inside_checkout`, `unreadable_rules_are_indeterminate`, `linked_worktree_uses_base_rules_file_against_its_own_paths`, `nested_linked_worktree_files_are_not_selected`, `submodule_files_are_not_selected`, `linked_directory_is_reported_but_not_selected`, and Linux-only `non_utf8_filename_keeps_its_git_bytes` exercise Git's actual matcher and filesystem boundaries. The initial negated-child test expectation failed; Git correctly kept the child selected when the parent was excluded, as the Phase 1 probe predicted. The assertion now records that behavior.
- Comparison: `size_change_avoids_read_and_same_size_edit_is_hashed` and `large_same_size_file_is_read` restore mtime after same-size edits and assert counted reads; `read_failure_is_unknown_and_kind_change_is_changed`, `symlink_target_change_is_changed_without_following`, and `untrusted_copy_baseline_is_unknown` cover the negative results. Included files use full BLAKE3 reads; size differences avoid them.
- Copy records: `repeated_round_trip_and_identity_failures` proves two writes and reads, mode 0600, corruption, version, marker, nonce, and malformed path rejection. `separate_worktree_records_survive_concurrent_writes_and_prune` and `unreadable_cache_directory_is_an_error` cover store isolation and pruning. Contract review found that Phase 3 must delete a stale record before its destination exists; `destination_key_is_stable_before_and_after_creation` now proves the key stays the same across creation and removal.
- Copy engine: `clone_success_does_not_call_byte_copy`, `fallback_copies_without_replacing_and_preserves_independence`, `failed_clone_cleans_partial_temp_and_keeps_destination_absent`, `index_and_ancestor_conflicts_skip_without_writing`, `modes_and_dangling_links_are_preserved`, `source_ancestor_link_is_never_followed`, `destination_ancestor_link_is_never_followed`, `denied_link_creation_is_skipped`, `source_change_during_copy_has_no_trusted_baseline`, `windows_privilege_error_is_a_link_skip`, and `filesystem_copy_succeeds_or_reports_clone_capability` cover copy behavior and dependent filesystem state. The filesystem capability test initially had a `real_` prefix, which stranded it behind a stub tier; it was renamed, and `just check-tier-coverage worktree` now reports zero stranded tests.
- Native Windows L1 initially exposed two implementation failures: Git rejected a verbatim `--exclude-from` path and NTFS returned HRESULT 0x80070001 for clone refusal. `canonicalize_simplified` and the clone fallback fixed them; the OS skill records both facts. The second Windows cross-check passed 153/153 library tests. Linux cross-check passed 168/168, including the raw non-UTF-8 filename test. The first Windows failure is resolved; no pre-existing test failure was found. A pre-existing Windows unused-import warning in touched `inventory.rs` was fixed with a target gate.
- Local gates after the final boundary tests and record-key change: `just test` passed 363/363 selected L1 tests (17 tier-filtered tests skipped); `just lint` passed for `worktree` and `worktree-cli`; `just check-tier-coverage worktree` found zero stranded tests. No `cargo fmt` was run. No L2 suite was run in this phase because no terminal behavior changed; prompt coverage belongs to Phase 5.
- WSL2 cross-check ran the nextest archive path and passed 174/174 library tests. Because source files changed while the remote run compiled, the cross-check did not publish a receipt for the final tree; its runtime result is still useful OS evidence. The post-run change was the `record_path` missing-destination key behavior, which has a targeted local test and passed the later native Windows cross-check (157/157).
- Final native Windows rerun passed 157/157 library tests, including the before/after destination-key test. Final `just test` passed 363/363 selected L1 tests with 17 tier-filtered tests skipped; `just lint` passed both area packages; `just check-tier-coverage worktree` found zero stranded tests; `git diff --check` passed. No pre-existing failures remain. The only skipped suite was L2, which is outside this library-only phase.

## Phase 3

- Before changing implementation code, mapped source resolution to a pure selection test plus real repository cases; copy and record to a create/read test; post-add failures to malformed rules, unreadable source, and blocked record-path tests; stale identity and list pruning to a remove/re-add/list test; and CLI output to real binary tests for copied paths, warnings, shell-wrapper stdout, and control characters. The first source and copy tests were added before wiring the flow. All tests use declared L1 targets and names selected by `just test`.
- `copy_source_prefers_checkout_and_handles_reuse_and_ambiguity` covers a checked-out fork source, fallback to the main checkout, a reused branch, and two matching entries. `create_copies_ignored_file_and_persists_observation` uses `.gitignore` and `.worktreeinclude` through Git's normal path, verifies destination content and the record's size and digest. `create_uses_checked_out_fork_source_and_base_for_reuse` checks source choice against two live checkouts and the actual copied bytes. `create_returns_ok_when_rules_are_indeterminate_and_clears_stale_record` checks an invalid rules path and no inherited record. `create_keeps_checkout_when_record_cannot_be_written` blocks the unique record path with a directory and proves the copy and warning remain. Unix-only `create_keeps_checkout_when_a_source_file_cannot_be_read` verifies a per-file failure leaves the worktree in place; it exits early on hosts whose user can still open a mode-000 file. `create_reuses_path_with_a_new_registration_and_list_prunes_removed_record` verifies changed copied content, a new registration marker, and pruning after a successful listing.
- `cli/tests/create_include.rs` adds `create_reports_copied_paths_on_stderr_and_preserves_stdout_protocol`, `create_warns_after_success_when_rules_are_invalid`, Unix-only `create_makes_control_characters_visible_in_the_report`, `create_reports_a_file_copy_failure_and_keeps_the_worktree`, and `create_reports_an_unsupported_link_without_copying_its_target`. The tests invoke the shipped `wt` binary with a real `.worktreeinclude` and Git repo; they assert copied bytes, escaped spaces, control and markup characters, failure warnings, stderr text, exactly empty stdout without the wrapper and exactly one `cd:` line with it. The first CLI run exposed that the existing shell protocol includes a trailing separator and that a fork from `main` labels its source `main`; expectations were corrected to those observed contracts. No shipped `.worktreeinclude` template exists in this repository, so there is no shipped-artifact corpus to enumerate. The repeated record read/write/read and corruption tests from Phase 2 remain in `copy_record`; this phase's path-reuse test checks persistence across a second creation.
- `create_worktree` selects the source and removes the old destination record before `git worktree add`, then locates rules, resolves membership, reads the destination index, copies without replacement, and writes observations with the Git registration identity. After-add errors become `CreateResult.include` warnings. `wt create` renders the copy line with `Prose` and warnings with `UnorderedList`; byte paths use visible escapes for control and non-ASCII bytes. `fill_worktree_statuses` prunes copy records using canonical paths from the successful listing. Updated the changed function docs and the worktree skill. Phase 4 still owns removal consent and record deletion.
- Local targeted tests: seven library tests and five CLI tests passed; the three Unix-only cases passed on macOS. The final `just test` passed 375/375 selected L1 tests (17 tier-filtered tests skipped). `just lint` passed both packages. `just check-tier-coverage worktree` reported zero stranded tests. Native Windows cross-check passed 181/181 selected `worktree-cli` tests (26 tier-filtered tests skipped) on the final Windows-applicable source and 163/163 `worktree` tests. The two CLI tests added after the final Windows run are Unix-only. A scratch repo with committed `.gitignore` and `.worktreeinclude` plus ignored `.env` printed `Copied from main: .env`; the copied bytes matched and `wt list` showed the new branch. An attempted `just test --quiet` invocation failed because the recipe passes that flag to nextest, which does not accept it; the exact `just test` command then passed. No pre-existing failures were observed. L2 and Linux/WSL2 cross-checks were not run in Phase 3; Phase 5 schedules the cross-OS sweep and L2 prompt work. No `cargo fmt` was run.

## Phase 4

- Test map before implementation: included-file consent and dependent deletion state → real `wt create`/`wt remove` CLI regressions; own/missing/empty/bad rules → library Git-fixture test and CLI exit-1 test; record and no-record comparison → CLI tests with copy records, corrupt records, distinct source, and self-source; handoff bindings and content → v2 record unit test plus rules, record, source, and repeated changed-content CLI handoffs; summary and policy → report unit test, expanded pure policy matrix, and CLI output checks. The first unchanged-copy CLI test was added and observed failing with exit 3 before implementation. No `.worktreeinclude` template or other shipped rules artifact exists to enumerate; the end-to-end tests use an actual `.worktreeinclude` in a Git checkout through the shipped `wt` binary. Earlier copy-record tests already cover repeated read/write/read persistence.
- `classify_included` now evaluates the target's own `.worktreeinclude`, falling back to the base checkout only when missing. It resolves the Git include set, compares with a registration-bound record or a distinct fork-source checkout, and marks new, changed, unknown, unchanged, and missing files. Corrupt records warn and fall back to the source. Empty rules do not fall back; indeterminate rules stop removal with exit 1. `Inventory` requires consent only for dirty or protected included files, summarizes other ignored names once, and fingerprints protected file content, rule contents and location, copy-record identity and contents, source observations, dirty content, and the full index. A disposable build artifact no longer changes a handoff. The old grouped-ignored display helper was removed.
- The handoff format is v3. Explicit rules and baseline bindings make a changed rules file, record, or no-record source refuse even when membership or the classification label changes. `copy_record::delete_for` runs immediately after successful directory removal; cleanup failure is a warning. The report names protected included paths and marks and uses one dim disposable-ignored summary. The prompt names protected files.
- Exact targeted L1 tests added to `cli/tests/remove.rs`: `unchanged_included_copy_and_other_ignored_files_remove_without_force`, `changed_included_copy_requires_force_and_preserves_state_on_refusal`, `same_size_included_edit_with_restored_mtime_requires_consent_at_both_sizes`, `no_record_uses_distinct_source_and_new_file_needs_consent`, `no_record_source_change_refuses_handoff`, `no_record_uses_the_checked_out_fork_parent_as_source`, `changed_include_rules_refuse_handoff_even_when_file_leaves_set`, `changed_copy_record_refuses_handoff_and_keeps_registration`, `changed_included_contents_refuse_handoff_even_when_classification_stays_changed`, `corrupt_copy_record_falls_back_to_source_without_consent`, `unavailable_distinct_source_marks_included_file_unknown`, and `invalid_include_rules_fail_before_removal`. Library `own_empty_rules_do_not_fall_back_to_base_and_bad_rules_refuse` tests own-rule precedence and errors. Existing `compare` tests prove size changes avoid reads and a large same-size file is read. `policy_matrix_follows_the_rules` now includes included-only, dirty-plus-included, unknown, and unchanged cases. These tests are compiled by declared targets, selected by L1, and use no feature gate.
- Old expectations changed deliberately: `ignored_entries_need_consent_like_dirty_files` became `ordinary_ignored_entries_are_disposable` and now expects exit 0 and the summary; `a_new_ignored_entry_between_the_runs_refuses` became `a_new_disposable_ignored_entry_between_the_runs_does_not_refuse` and expects a successful handoff. Five existing dirty-content handoff assertions now look for “uncommitted or included files” instead of “uncommitted or ignored files”; their exit-3 and no-deletion assertions remain. The inventory ignored-entry fingerprint assertion now expects no change for a disposable ignored file. The old report grouped-count assertion now checks the one-line summary.
- Verification: `just test` passed 388/388 L1 tests, with 17 tier-filtered tests skipped. `just check-tier-coverage worktree` found zero stranded tests. Native Windows cross-check passed 192/192 `worktree-cli` L1 tests before the final fork-parent test was added (26 tier-filtered skipped), 164/164 `worktree` L1 tests, and all three `level2_powershell_remove` tests (5.98–6.74 seconds each, so none was a skip). macOS `just test-l2 level2_remove` passed both existing removal tests. The full macOS `just test-l2` run stopped at an unrelated Kitty graph screenshot test because the screenshot contained no window contents; nine later tests were not reached in that run. `git diff --check` passed. `just lint` passed after the final source edits for both packages. No `cargo fmt`, staging, or commit was performed.
