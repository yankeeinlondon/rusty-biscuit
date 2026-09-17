# Phase 7 Acceptance Evidence

Evidence collected on macOS on 2026-09-17. All commands ran from the named
package area. `CARGO_TARGET_DIR=/tmp/claudine-phase7-target` was a no-space
symlink to a session-owned target directory on a local volume because the
workspace volume ran out of free space; this changed only artifact placement.

## Requirement-to-test mapping

| Acceptance criterion | Public behavior and named evidence |
|---|---|
| AC1–AC3: initialize before discovery, guarded and unconditional generated includes | `compose_initialize_staged_boot::a_proxied_target_initialize_creates_a_file_the_target_body_includes`, `compose_initialize_staged_boot::compose_initialize_creates_a_file_the_body_includes`, `compose_initialize_acceptance::the_shipped_router_reaches_the_reported_guarded_log_include_at_each_phase` |
| AC4: preserve existing content and repeated read/write/read | `compose_initialize_acceptance::ensure_file_preserves_an_existing_include_the_prompt_reads`, `compose_initialize_acceptance::a_second_invocation_reads_the_persisted_file_and_initializes_once` |
| AC5: initialize and reread approval integrity | `compose_initialize_acceptance::a_shell_directive_initialize_writes_into_an_include_is_audited_before_it_runs`, `compose_initialize_acceptance::an_approved_shell_directive_initialize_writes_into_an_include_runs`, `compose_initialize_acceptance::a_false_condition_include_still_contributes_its_command_to_the_approval_set` |
| AC6: reread mutations | `compose_initialize_acceptance::an_include_initialize_rewrites_reaches_the_prompt_with_its_new_content`, `compose_initialize_acceptance::initialize_mutating_the_document_itself_is_prompted_and_audited_from_the_reread` |
| AC7: proxy and control-flow behavior | `compose_initialize_acceptance::an_initialize_proxy_chain_never_reads_an_abandoned_body`, `an_initialize_stop_ends_the_stack_and_the_run_continues_from_the_reread`, `an_initialize_error_fails_without_reading_the_body_or_launching`, `a_retry_rereads_the_include_without_initializing_again`, and `a_resume_continues_the_session_without_initializing_again`; `compose_initialize_staged_boot::an_initialize_skip_ends_the_run_before_the_body_is_discovered` |
| AC8: missing after initialization | `compose_initialize_staged_boot::a_file_still_missing_after_initialize_blocks_once_without_launching` |
| AC9: entry-path parity | `compose_initialize_staged_boot::compose_initialize_creates_a_file_the_body_includes`, `inline_compose_initialize_creates_a_file_the_prompt_includes`, `a_proxied_target_initialize_creates_a_file_the_target_body_includes`, `a_looping_target_first_iteration_includes_what_initialize_created`, and `a_sequence_task_proxy_target_initialize_creates_a_file_its_body_includes`; `compose_initialize_acceptance::inline_compose_proxy_target_initialize_creates_a_file_its_prompt_includes` |
| AC10: shipped artifacts | Passive corpus tests `shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions` and `shipped_prompt_route_drift::fixture_body_matches_the_shipped_body`; normal CLI end-to-end tests `compose_initialize_acceptance::the_shipped_router_implements_a_spec_whose_log_does_not_exist_yet` and `the_shipped_router_reaches_the_reported_guarded_log_include_at_each_phase` copy the shipped router and drift-guarded target, preserve the reported `spec=fixes/2026-09-14-cicd-improvements/spec.md` spelling, and invoke `claudine compose` with a fake provider |
| AC11: unchanged failure, retry/resume, dry run | `compose_initialize_staged_boot::a_document_without_initialize_still_fails_eagerly_on_a_missing_include`, `a_dry_run_never_runs_initialize_and_still_reports_the_missing_include`; the AC7 retry and resume tests above |
| AC12: reference variants and invalid input | `compose_initialize_acceptance::{an_interpolated_repository_root_reference_ensures_and_includes_one_file,an_unshadowed_implicit_reference_ensures_and_includes_one_file,an_explicit_relative_reference_ensures_and_includes_one_file,a_repository_root_reference_ensures_and_includes_one_file,a_repository_scoped_reference_ensures_and_includes_one_file,a_shadowed_implicit_reference_mutates_the_file_the_include_prefers,every_filesystem_effect_uses_the_document_reference_identity,a_repository_escape_is_rejected_before_any_file_or_provider_effect}`; Darkmatter unit tests `effects::fs_write::tests::{equivalent_symlinked_root_spellings_are_contained,an_in_root_symlink_cannot_redirect_a_new_file_outside}` |

The AC12 table covers missing and present targets, implicit and explicit
references, interpolation, repository-root and repository-scoped forms,
shadowing, a traversal error, equivalent macOS root spellings, and a symlink
escape. The all-effects row observes `ensure_dir`, both forms of `ensure_file`,
`append_line`, and `append_jsonl` through the normal CLI path. Assertions
include provider launch state, resulting file identity and contents, prompt
contents, and absence of decoy and out-of-root files.

## Regression proof and targeted runs

- Before the implementation change:
  `CARGO_TARGET_DIR=/tmp/claudine-phase7-target just test --test compose_initialize_acceptance --run-ignored ignored-only`
  ran the three previously ignored AC12 regressions and failed 3/3: explicit
  relative and repository-root creation failed, while an existing shadowed
  implicit target mutated the decoy identity.
- After the change:
  `CARGO_TARGET_DIR=/tmp/claudine-phase7-target CARGO_INCREMENTAL=0 just test --test compose_initialize_acceptance`
  passed 23/23, including the original reference strings and the new invalid
  `&../outside.md` case.
- Darkmatter containment regressions:
  `BISCUIT_TEST_FILTER='test(/effects::fs_write::tests::(equivalent_symlinked_root_spellings_are_contained|an_in_root_symlink_cannot_redirect_a_new_file_outside)/)' just _test_local_all 'darkmatter --features effects-instrumentation;' --lib`
  passed 2/2. The area-level matrix below also ran both tests.

## Full local matrix

| Area | Command | Result |
|---|---|---|
| Claudine L1 | `CARGO_TARGET_DIR=/tmp/claudine-phase7-target CARGO_INCREMENTAL=0 just test --no-fail-fast` | 7,257 run: 7,250 passed, 7 failed, 9 skipped. Every Phase 7 acceptance test passed. All seven failures are the pre-existing `commands::wrap::exec::spawn::tests::{captured,inherited}` HOME-overlay checks, which reject the inherited host value `HOME=/Users/ken/.claudine`; no Phase 7 path is in their failure output. |
| Claudine L2 | `CARGO_TARGET_DIR=/tmp/claudine-phase7-target CARGO_INCREMENTAL=0 just test-l2 --no-fail-fast` | Pass: `claudine-cli` 231/231 and `claudine-gen` 3/3. The generated-transclusion terminal suite passed all four tests, including direct denial, sequence guidance, and proxied guarded phases. |
| Claudine lint | `CARGO_TARGET_DIR=/tmp/claudine-phase7-target CARGO_INCREMENTAL=0 just lint` | Pass, including the scan-backed lifecycle diagnostic guard and all area packages. |
| Darkmatter L1 | `CARGO_TARGET_DIR=/tmp/claudine-phase7-target CARGO_INCREMENTAL=0 just test --no-fail-fast` | Pass: 7,937/7,937, 7 skipped. |
| Darkmatter L2 | `CARGO_TARGET_DIR=/tmp/claudine-phase7-target CARGO_INCREMENTAL=0 just test-l2 --no-fail-fast` | Pass: `darkmatter` 18/18, `darkmatter-cli` 69/69, and `dmls` 3/3. |
| Darkmatter lint | `CARGO_TARGET_DIR=/tmp/claudine-phase7-target CARGO_INCREMENTAL=0 just lint` | Pass, including `zed-dmls` for `wasm32-wasip2`. |

The Claudine L1 failures are an inherited host-configuration condition, not a
Phase 7 regression. The complete no-fail-fast run establishes that every other
L1 test passed. The host HOME was not rewritten because it is externally owned
session state and the failing tests specifically guard that invariant.

Two intermediate reruns did not provide product evidence. The repository's
shared `target/` cache rejected compiler output as not writable before any test
ran, so the command was repeated with the isolated target. The first draft of
the all-effects fixture tried to transclude `events.jsonl`; the expected
unsupported-file-type diagnostic failed that single test. The fixture now
writes the same JSONL record to `events.md`, allowing the normal Markdown
transclusion path to verify downstream content, and the resulting 23/23 run is
the qualifying targeted evidence.

## OS coverage gaps

`PATH=/opt/homebrew/bin:$PATH just cross-check claudine --os all` was attempted
after local targeted coverage passed. It produced no qualifying remote result:

- Linux (`build-linux`) was held by an externally owned
  `nightly-reward-spike` lock. The lock was not disturbed.
- Native Windows (`build-win-native`) could not receive the patch because the
  host filesystem reported `No space left on device`.
- WSL (`build-win`) reset the SSH connection.

The implementation uses `Path`, `PathBuf`, `OsString`, and `FileReference`
rather than separator string comparisons. The Unix-only symlink tests are
appropriately gated; reference identity acceptance tests are portable. CI must
provide the missing Linux, native Windows, and WSL qualification because none
of the available rigs returned reusable evidence.

## Phase 8 final validation

Phase 8 changes repository documentation, two Claudine skill pages, and Rust
comments only. No behavior or test was added or changed in this phase. The
requirement-to-test mapping above remains the acceptance matrix; the Phase 8
log maps each documented ordering guarantee to those exact existing suites.

All builds below use `CARGO_TARGET_DIR=/tmp/claudine-phase8-target
CARGO_INCREMENTAL=0 RUSTC_WRAPPER=`. The target is a no-space symlink to a
session-owned directory on `/Volumes/Fast Bastard`, selected after `sniff
storage --json` reported only about 7 GiB available on the workspace volume
and about 1.8 TB on that volume. A first ordinary `just test --no-fail-fast`
failed before tests because shared-target `.rmeta` files were read-only.
Shared cache permissions were not modified.

### Environment qualification

The first isolated-target Claudine `just test --no-fail-fast` ran 7,258 tests:
7,251 passed, seven failed, nine skipped. Every initialization acceptance test
passed. The seven failures repeat Phase 7's inherited-home condition at
`spawn/setup.rs:61`: `child env "HOME" is "/Users/ken/.claudine" — a provider
overlay must never move the user home`.

Exact failing tests, under `commands::wrap::exec::spawn::tests`:

- `captured::consecutive_spawns_produce_distinct_agent_pids`
- `captured::run_child_capture_propagates_claudine_pid_to_child_environment`
- `captured::run_child_capture_captures_agent_pid_after_successful_spawn`
- `captured::run_child_failed_spawn_returns_err_without_agent_pid`
- `captured::run_child_capture_wall_clock_timeout_reaps_child`
- `inherited::run_child_wall_clock_timeout_reaps_child`
- `inherited::run_child_captures_agent_pid_after_successful_spawn`

A diagnostic run removed `HOME` and `USERPROFILE` only from the test command's
inherited environment, exercising the existing fixture's temporary-directory
fallback without changing any source or persistent host setting:
`env -u HOME -u USERPROFILE …
BISCUIT_TEST_FILTER='test(/commands::wrap::exec::spawn::tests::(captured|inherited)/)'
just test --no-fail-fast`. It passed **9/9**; its 7,258 skips were the explicit
filter's exclusions, not acceptance evidence. A full clean-environment run
follows to qualify the entire area without that filter.

### Documentation checks

- Nine edited public/skill pages rendered to HTML with `md --output html`;
  no browser or terminal window was opened.
- All eight edited topic/skill pages pass `md validate refs --fragments`.
  The skill composition page had a pre-existing broken relative closure-source
  link, repaired in this phase.
- README-wide reference validation stops at its existing `claudine providers
  --plain` shell directive because approval is required. No command was
  approved/executed; the added initialization link targets the independently
  validated composition heading.
- `md hash --diff` passes for the three edited Markdown files with stored
  hashes (composition topic, execution-flow topic, composition skill).
- `git diff --check` passes. The Phase 8 Rust diff is verified comment-only:
  `cli/src/commands/wrap/harness_orch/loop_control.rs` and
  `lib/src/composition/preflight.rs`.

Phase 8 introduces no OS-dependent implementation. It does not claim new
Linux/native-Windows/WSL qualification; the Phase 7 gaps above remain explicit
and require the normal CI coverage.

### Final package-area gates

| Area | Command (with target environment above) | Result |
|---|---|---|
| Claudine L1 | `env -u HOME -u USERPROFILE … just test --no-fail-fast` | **7,258/7,258 pass**, nine existing opt-in skips. No test filter; every initialization acceptance test, shipped-corpus test, and shipped-route test passed. |
| Claudine lint | `just lint` | **Pass**, including transport/lifecycle-doc guards and all five area packages. |
| Claudine L2 | `just test-l2 --no-fail-fast` | **231/231 CLI and 3/3 generator pass**. All four `level2_initialize_generated_transclusion` regressions pass. The L2 filters exclude 2,743 CLI and 174 generator non-L2 tests; those are not missing L2 coverage. |
| Darkmatter L1 | `just test --no-fail-fast --status-level skip` | **7,937/7,937 pass**, seven existing skips. Includes `frontmatter_surface_projection` and the Phase 7 filesystem-containment tests. |
| Darkmatter lint | `just lint` | **Pass**, including all area packages and the `zed-dmls` `wasm32-wasip2` check. |
| Darkmatter L2 | `just test-l2 --no-fail-fast` | **18/18 library, 69/69 CLI, and 3/3 DMLS pass**; all three package gates pass. Standard L2 filters exclude 6,493 library, 714 CLI, and 715 DMLS non-L2 tests. |

The nine Claudine L1 skips are existing diagnostic/performance opt-ins:
`compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`;
`completion_perf::{perf_compose_empty_partial_meets_target,
perf_compose_long_prefix_meets_target,perf_inline_compose_empty_partial_meets_target,
perf_enter_compose_partial_meets_target}`; and
`system_prompt_perf_bench::{bench_system_prompt_resolution_cold_and_warm,
bench_resolve_and_prepare_step_by_step,bench_request_topology_probe_and_reuse,
bench_raw_darkmatter_compose_passes}`. Real-provider and terminal targets are
outside the ordinary L1 recipe's feature/tier scope; L2 is run separately.

Darkmatter L1's seven skips, as printed by `--status-level skip`:

- `layout::page::tests::finding_35_6::f35_6_rhythm_raw_samples` (opt-in measurement)
- `markdown::cleanup::perf_profile::f25_cleanup_profile_raw_samples` (opt-in measurement)
- `markdown::compose::shell_expansion::alias::tests::resolve_alias_ll` (requires a user-defined login-shell alias)
- `markdown::compose::tests::rendering::slow_compose_cleanup_preserves_quoted_marker_looking_indented_code` (local slow-test filter)
- `markdown::render_tree::build_context::finding_35_7::f35_7_link_policy_raw_samples` (opt-in measurement)
- `markdown::render_tree::code_renderer::tests::f23_code_surface_raw_samples` (opt-in measurement)
- `schema_phase_validation::real_shipped_inline_schema_uses_normal_resolution_and_phase_path` (existing `real_` name excluded by the ordinary L1 tier filter)

These pre-existing exclusions were not changed. No Phase 8 acceptance test was
added, ignored, or filtered out of the final Claudine L1 run.
