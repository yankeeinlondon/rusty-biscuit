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
