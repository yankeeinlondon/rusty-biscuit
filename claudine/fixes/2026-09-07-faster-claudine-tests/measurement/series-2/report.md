### Runs

| Suite | Revision | Seq | Role | Wall | Build/setup | Runner elapsed | Summed | Tests | Passed | Failed | Timed out | Skipped | Leaks | Retries | Slow | Load before |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `just test` | baseline | 1 | alternating | 39.0 s | 1.9 s | 37.14 s | 580.07 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 1 | { 10.40 19.61 29.98 } |
| `just test` | candidate | 1 | alternating | 28.0 s | 1.7 s | 26.36 s | 407.85 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 27.51 23.17 30.83 } |
| `just test` | baseline | 2 | alternating | 42.3 s | 1.5 s | 40.76 s | 637.20 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 1 | { 25.81 23.06 30.51 } |
| `just test` | candidate | 2 | alternating | 32.8 s | 1.6 s | 31.19 s | 482.75 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 55.79 31.06 33.05 } |
| `just test` | baseline | 3 | alternating | 53.8 s | 1.5 s | 52.30 s | 819.42 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 7 | { 52.91 32.85 33.61 } |
| `just test` | candidate | 3 | alternating | 39.7 s | 1.7 s | 38.03 s | 589.63 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 1 | { 58.42 37.67 35.34 } |
| `just test` | baseline | 4 | alternating | 60.5 s | 1.6 s | 58.91 s | 923.17 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 20 | { 48.12 37.60 35.42 } |
| `just test` | candidate | 4 | alternating | 39.5 s | 1.7 s | 37.89 s | 587.89 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 57.71 42.60 37.46 } |
| `just test` | baseline | 5 | alternating | 55.5 s | 1.5 s | 53.92 s | 844.45 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 9 | { 51.92 42.66 37.69 } |
| `just test` | candidate | 5 | alternating | 40.0 s | 1.7 s | 38.39 s | 597.39 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 46.33 42.92 38.13 } |

### Per-revision spread (alternating runs only)

| Suite | Revision | Runs | Elapsed min / median / max | Elapsed drift | Summed min / median / max | Summed drift | Identities |
|---|---|---:|---|---:|---|---:|---:|
| `just test` | baseline | 5 | 37.14 / 52.30 / 58.91 s | 41.6% | 580.07 / 819.42 / 923.17 s | 41.9% | 6861 |
| `just test` | candidate | 5 | 26.36 / 37.89 / 38.39 s | 31.7% | 407.85 / 587.89 / 597.39 s | 32.2% | 6873 |

### Baseline → candidate (medians; established only outside both drift brackets)

| Suite | Column | Baseline | Candidate | Delta | Established |
|---|---|---:|---:|---:|---|
| `just test` | elapsed | 52.30 s | 37.89 s | -14.42 s (-27.6%) | **no — inside drift** |
| `just test` | summed | 819.42 s | 587.89 s | -231.52 s (-28.3%) | **no — inside drift** |
| `just test` | identities | 6861 | 6873 | +28 / −16 | — |

#### Paired alternation (candidate ÷ baseline, same sequence number)

| Suite | Column | Pairs | Ratio min / median / max | Every pair improved |
|---|---|---:|---|---|
| `just test` | elapsed | 5 | 0.643 / 0.712 / 0.765 | yes |
| `just test` | summed | 5 | 0.637 / 0.707 / 0.758 | yes |

#### Identity changes

`just test` — added 28, removed 16

- added: `claudine linking::paths::tests::new_roots_the_table_at_the_process_home_and_the_resolved_repository`
- added: `claudine linking::paths::tests::repo_scope_target_paths_are_rooted_at_the_repository`
- added: `claudine render::event_renderer::tests::session_start_updates_the_auth_source_even_when_silent`
- added: `claudine render::event_renderer::tests::silent_verbosity_produces_no_render_units`
- added: `claudine-cli::cli_process_fixture a_parent_side_git_cannot_be_relocated_by_an_inherited_gitdir`
- added: `claudine-cli::cli_process_fixture both_command_surfaces_clear_the_environment_the_same_way`
- added: `claudine-cli::cli_process_fixture both_command_surfaces_disable_rendezvous_reporting_over_an_enabled_parent`
- added: `claudine-cli::cli_process_fixture both_command_surfaces_hand_the_child_the_same_environment`
- added: `claudine-cli::cli_process_fixture both_command_surfaces_keep_audio_out_of_the_developers_machine`
- added: `claudine-cli::cli_process_fixture the_raw_command_surface_keeps_the_ambient_context_escape`
- added: `claudine-cli::cli_process_fixture the_raw_command_surface_keeps_the_named_path_escapes`
- added: `claudine-cli::cli_process_fixture the_raw_command_surface_rejects_an_ambient_context_outside_the_workspace`
- added: `claudine-cli::contamination_probes a_checkout_ancestor_temp_dir_is_refused_at_construction`
- added: `claudine-cli::contamination_probes a_relocated_temp_dir_outside_the_checkout_does_not_change_the_result`
- added: `claudine-cli::contamination_probes an_exported_path_carrying_a_decoy_provider_does_not_change_the_result`
- added: `claudine-cli::contamination_probes an_uncontaminated_run_matches_the_expectations_every_probe_asserts`
- added: `claudine-cli::contamination_probes exported_claudine_application_variables_do_not_change_the_result`
- added: `claudine-cli::contamination_probes exported_git_plumbing_pointed_at_a_throwaway_repo_does_not_change_the_result`
- added: `claudine-cli::contamination_probes exported_home_and_cache_relocation_does_not_change_the_result`
- added: `claudine-cli::contamination_probes exported_render_width_and_color_do_not_change_the_result`
- added: `claudine-cli::error_guards a_failing_scan_backed_guard_is_reported_under_its_own_name`
- added: `claudine-cli::error_guards production_sources_pass_every_scan_backed_guard`
- added: `claudine-cli::spawn_site_guard deleting_a_spawn_entry_is_what_widens_the_isolation_population`
- added: `claudine-cli::spawn_site_guard detector_treats_the_builders_raw_command_path_as_a_sanctioned_form`
- added: `claudine-cli::spawn_site_guard isolation_detector_reads_a_raw_fixture_command_like_an_assert_cmd_one`
- added: `claudine-cli::spawn_site_guard the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site`
- added: `claudine-cli::wrap_sigint compose_sigint_during_prep_exits_130_with_notice`
- added: `claudine-gen::signals_validation shipped_corpus_builds_deterministically`
- removed: `claudine linking::paths::tests::repo_scope_target_paths_are_absolute`
- removed: `claudine stream::stderr::tests::silent_mode_produces_nothing`
- removed: `claudine-cli::context_command context_values_renders_non_null_for_canonical_keys`
- removed: `claudine-cli::error_guards a_diagnostic_claiming_a_registered_code_projects_a_detail`
- removed: `claudine-cli::error_guards detail_projections_write_only_declared_keys`
- removed: `claudine-cli::error_guards every_allowlist_entry_still_matches_a_live_site`
- removed: `claudine-cli::error_guards every_boxed_allowlist_entry_still_matches_a_live_site`
- removed: `claudine-cli::error_guards every_code_a_diagnostic_claims_is_a_registered_code`
- removed: `claudine-cli::error_guards every_diagnostic_impl_also_implements_block_error`
- removed: `claudine-cli::error_guards no_registered_diagnostic_is_reachable_only_through_a_box`
- removed: `claudine-cli::error_guards no_unallowlisted_typed_error_collapses`
- removed: `claudine-cli::error_guards registry_lists_every_diagnostic_impl`
- removed: `claudine-cli::error_guards the_boxed_scan_finds_the_sites_known_to_exist`
- removed: `claudine-cli::error_guards the_corpus_covers_every_code_a_diagnostic_can_return`
- removed: `claudine-cli::error_guards the_scan_finds_the_diagnostic_impls_known_to_exist`
- removed: `claudine-cli::spawn_site_guard an_empty_allowlist_leaves_every_live_site_unlisted`

### Changed cohorts (from the alternating suite runs; medians of summed duration)

| Cohort | Baseline tests | Baseline summed (min / median / max) | Candidate tests | Candidate summed (min / median / max) | Delta (median) | Established |
|---|---:|---|---:|---|---:|---|
| `cli-integration (all `claudine-cli::*` binaries; every one compiles the changed `common/`)` | 2489 | 295.03 / 417.36 / 470.93 s | 2498 | 225.69 / 310.66 / 338.68 s | -106.71 s (-25.6%) | no — inside drift |
| `cli-phase5A compose family` | 69 | 16.97 / 20.54 / 21.75 s | 69 | 13.78 / 16.34 / 17.36 s | -4.20 s (-20.5%) | no — inside drift |
| `cli-phase5B sequence / loop family` | 155 | 41.97 / 49.09 / 54.76 s | 155 | 40.94 / 46.66 / 47.65 s | -2.43 s (-5.0%) | no — inside drift |
| `cli-phase5C context / errors / completion / resources family` | 84 | 33.97 / 52.66 / 64.35 s | 83 | 16.29 / 21.36 / 23.54 s | -31.30 s (-59.4%) | yes |
| `cli-phase5D live-child and PTY cohort` | 11 | 18.93 / 20.46 / 22.35 s | 12 | 5.15 / 6.23 / 6.93 s | -14.23 s (-69.6%) | yes |
| `cli-guards, probes and fixture self-tests` | 62 | 44.05 / 69.82 / 95.52 s | 71 | 14.39 / 21.90 / 24.73 s | -47.92 s (-68.6%) | no — inside drift |
| `cli-context_command alone` | 27 | 27.14 / 43.79 / 54.81 s | 26 | 9.97 / 12.79 / 14.47 s | -31.00 s (-70.8%) | yes |
| `cli-error_guards alone` | 18 | 38.15 / 60.13 / 85.88 s | 8 | 2.02 / 3.25 / 3.58 s | -56.88 s (-94.6%) | yes |
| `cli-sequence_overlay_pty alone` | 7 | 17.43 / 18.11 / 19.26 s | 7 | 3.69 / 4.23 / 4.70 s | -13.88 s (-76.7%) | yes |
| `lib-unit (all `claudine` lib tests)` | 4040 | 261.82 / 361.31 / 412.00 s | 4042 | 156.35 / 220.35 / 242.31 s | -140.96 s (-39.0%) | no — inside drift |
| `lib composition::schema` | 75 | 34.63 / 58.95 / 67.35 s | 75 | 10.95 / 14.43 / 15.72 s | -44.52 s (-75.5%) | yes |
| `lib composition::sequence::preflight` | 44 | 31.15 / 45.30 / 54.32 s | 44 | 3.12 / 4.05 / 4.53 s | -41.25 s (-91.1%) | yes |
| `lib composition::sequence::task` | 106 | 58.34 / 70.38 / 80.00 s | 106 | 14.45 / 15.35 / 15.62 s | -55.04 s (-78.2%) | yes |
| `lib linking::paths` | 10 | 9.01 / 14.74 / 19.83 s | 11 | 1.13 / 2.09 / 3.39 s | -12.65 s (-85.8%) | yes |
| `lib render::event_renderer` | 2 | 0.03 / 0.05 / 0.06 s | 4 | 0.07 / 0.12 / 0.16 s | +0.07 s (144.0%) | no — inside drift |
| `lib stream::stderr` | 21 | 0.37 / 0.71 / 0.75 s | 20 | 0.41 / 0.62 / 0.73 s | -0.09 s (-12.6%) | no — inside drift |
| `cli-unit (`claudine-cli` bin tests, untouched)` | 1694 | 52.09 / 81.83 / 92.07 s | 1694 | 54.78 / 89.69 / 102.31 s | +7.86 s (9.6%) | no — inside drift |
| `claudine-gen signals_validation` | 8 | 0.42 / 0.53 / 0.81 s | 9 | 0.53 / 0.74 / 1.06 s | +0.21 s (40.0%) | no — inside drift |
| `claudine-contract / catalog-types / gen (all, untouched except metadata)` | 223 | 18.40 / 27.91 / 32.64 s | 224 | 21.71 / 29.35 / 41.95 s | +1.44 s (5.2%) | no — inside drift |
| `rendezvous-daemon pairing_and_sync` | — | — | — | — | — | — |
| `rendezvous (all three crates)` | — | — | — | — | — | — |

