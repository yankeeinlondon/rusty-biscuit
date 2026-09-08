### Runs

| Suite | Revision | Seq | Role | Wall | Build/setup | Runner elapsed | Summed | Tests | Passed | Failed | Timed out | Skipped | Leaks | Retries | Slow | Load before |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `just test` | baseline | 1 | warmup | 141.8 s | 98.8 s | 43.00 s | 674.50 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 3 | { 7.62 34.55 34.48 } |
| `just test` | candidate | 1 | warmup | 104.5 s | 69.4 s | 35.12 s | 544.94 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 67.91 43.80 37.77 } |
| `just test-rendezvous` | baseline | 1 | warmup | 149.2 s | 137.0 s | 12.23 s | 174.47 s | 272 | 272 | 0 | 0 | 2 | 0 | 0 | 1 | { 57.83 45.21 38.86 } |
| `just test-rendezvous` | candidate | 1 | warmup | 17.9 s | 4.7 s | 13.14 s | 188.44 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 1 | { 19.58 32.63 34.68 } |
| `l2-pty` | candidate | 1 | warmup | 15.8 s | 11.1 s | 4.66 s | 18.08 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 18.84 31.61 34.27 } |
| `just test` | baseline | 2 | alternating | 93.1 s | 4.5 s | 88.63 s | 1379.07 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 58 | { 17.96 30.81 33.94 } |
| `just test` | candidate | 2 | alternating | 54.3 s | 4.4 s | 49.89 s | 779.95 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 10 | { 79.59 47.55 40.02 } |
| `just test` | baseline | 3 | alternating | 56.6 s | 1.5 s | 55.12 s | 864.06 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 9 | { 74.56 52.53 42.43 } |
| `just test` | candidate | 3 | alternating | 37.6 s | 1.6 s | 36.01 s | 558.03 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 66.12 54.50 43.87 } |
| `just test` | baseline | 4 | alternating | 51.3 s | 1.6 s | 49.78 s | 781.51 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 7 | { 63.25 55.51 44.74 } |
| `just test` | candidate | 4 | alternating | 30.8 s | 1.6 s | 29.20 s | 453.03 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 58.64 54.88 45.09 } |
| `just test` | baseline | 5 | alternating | 50.4 s | 1.4 s | 48.98 s | 767.01 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 7 | { 53.44 53.79 45.07 } |
| `just test` | candidate | 5 | alternating | 37.9 s | 2.3 s | 35.63 s | 552.83 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 69.22 57.06 46.72 } |
| `just test` | baseline | 6 | alternating | 56.1 s | 2.6 s | 53.44 s | 836.68 s | 6861 | 6861 | 0 | 0 | 11 | 0 | 0 | 8 | { 50.22 53.51 45.88 } |
| `just test` | candidate | 6 | alternating | 49.6 s | 2.9 s | 46.79 s | 722.35 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 6 | { 47.93 52.65 46.10 } |
| `just test-rendezvous` | baseline | 2 | alternating | 25.1 s | 8.2 s | 16.86 s | 241.79 s | 272 | 272 | 0 | 0 | 2 | 0 | 0 | 3 | { 35.50 48.42 44.89 } |
| `just test-rendezvous` | candidate | 2 | alternating | 24.7 s | 7.7 s | 17.00 s | 247.74 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 3 | { 32.07 46.25 44.22 } |
| `just test-rendezvous` | baseline | 3 | alternating | 26.9 s | 7.0 s | 19.92 s | 292.84 s | 272 | 272 | 0 | 0 | 2 | 0 | 0 | 3 | { 31.06 44.75 43.73 } |
| `just test-rendezvous` | candidate | 3 | alternating | 17.9 s | 4.8 s | 13.19 s | 188.33 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 2 | { 27.78 42.68 43.02 } |
| `just test-rendezvous` | baseline | 4 | alternating | 15.8 s | 4.2 s | 11.60 s | 163.21 s | 272 | 272 | 0 | 0 | 2 | 0 | 0 | 4 | { 23.73 40.79 42.33 } |
| `just test-rendezvous` | candidate | 4 | alternating | 14.7 s | 4.6 s | 10.10 s | 139.24 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 1 | { 23.43 39.58 41.86 } |
| `just test-rendezvous` | baseline | 5 | alternating | 13.9 s | 4.2 s | 9.70 s | 135.83 s | 272 | 272 | 0 | 0 | 2 | 0 | 0 | 1 | { 19.48 37.92 41.23 } |
| `just test-rendezvous` | candidate | 5 | alternating | 13.7 s | 4.3 s | 9.34 s | 129.11 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 1 | { 19.86 36.83 40.76 } |
| `just test-rendezvous` | baseline | 6 | alternating | 13.0 s | 3.7 s | 9.35 s | 129.09 s | 272 | 272 | 0 | 0 | 2 | 0 | 0 | 2 | { 18.09 35.61 40.26 } |
| `just test-rendezvous` | candidate | 6 | alternating | 14.0 s | 3.9 s | 10.06 s | 141.34 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 5 | { 15.04 34.08 39.63 } |
| `just test` | candidate | 7 | load | 30.2 s | 1.9 s | 28.31 s | 438.79 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 13.34 32.77 39.06 } |
| `just test-rendezvous` | candidate | 7 | load | 13.9 s | 3.9 s | 10.01 s | 138.45 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 4 | { 18.46 32.14 38.61 } |
| `just test` | candidate | 8 | load | 30.5 s | 1.6 s | 28.98 s | 447.57 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 17.15 30.95 38.04 } |
| `just test-rendezvous` | candidate | 8 | load | 14.8 s | 4.2 s | 10.58 s | 149.03 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 4 | { 24.89 31.75 38.09 } |
| `just test` | candidate | 9 | load | 33.6 s | 1.6 s | 31.98 s | 497.11 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 21.79 30.72 37.62 } |
| `just test-rendezvous` | candidate | 9 | load | 13.8 s | 3.9 s | 9.90 s | 138.61 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 3 | { 29.65 31.90 37.74 } |
| `just test` | candidate | 10 | load | 32.0 s | 1.6 s | 30.37 s | 471.57 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 25.11 30.79 37.25 } |
| `just test-rendezvous` | candidate | 10 | load | 14.3 s | 3.8 s | 10.46 s | 146.21 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 4 | { 29.66 31.53 37.27 } |
| `just test` | candidate | 11 | load | 32.1 s | 1.5 s | 30.58 s | 472.20 s | 6873 | 6873 | 0 | 0 | 9 | 0 | 0 | 0 | { 25.34 30.50 36.80 } |
| `just test-rendezvous` | candidate | 11 | load | 15.0 s | 4.0 s | 11.00 s | 154.11 s | 273 | 273 | 0 | 0 | 2 | 0 | 0 | 2 | { 25.29 30.00 36.37 } |
| `l2-pty` | candidate | 2 | load | 5.8 s | 1.2 s | 4.61 s | 20.89 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 27.47 30.17 36.31 } |
| `l2-pty` | candidate | 3 | load | 5.8 s | 1.2 s | 4.60 s | 17.64 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 28.83 30.40 36.32 } |
| `l2-pty` | candidate | 4 | load | 5.8 s | 1.2 s | 4.61 s | 18.23 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 27.24 30.05 36.16 } |
| `l2-pty` | candidate | 5 | load | 5.9 s | 1.2 s | 4.63 s | 17.09 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 23.30 29.11 35.76 } |
| `l2-pty` | candidate | 6 | load | 6.0 s | 1.3 s | 4.70 s | 17.79 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 22.23 28.79 35.61 } |
| `l2-pty` | candidate | 7 | load | 5.9 s | 1.2 s | 4.71 s | 18.81 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 19.64 28.02 35.25 } |
| `l2-pty` | candidate | 8 | load | 5.9 s | 1.2 s | 4.66 s | 17.92 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 17.14 27.21 34.88 } |
| `l2-pty` | candidate | 9 | load | 5.9 s | 1.3 s | 4.65 s | 18.32 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 16.73 26.96 34.75 } |
| `l2-pty` | candidate | 10 | load | 5.8 s | 1.2 s | 4.62 s | 17.76 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 14.55 26.15 34.37 } |
| `l2-pty` | candidate | 11 | load | 5.9 s | 1.3 s | 4.60 s | 17.67 s | 19 | 19 | 0 | 0 | 0 | 0 | 0 | 0 | { 14.26 25.90 34.23 } |

### Per-revision spread (alternating runs only)

| Suite | Revision | Runs | Elapsed min / median / max | Elapsed drift | Summed min / median / max | Summed drift | Identities |
|---|---|---:|---|---:|---|---:|---:|
| `just test` | baseline | 5 | 48.98 / 53.44 / 88.63 s | 74.2% | 767.01 / 836.68 / 1379.07 s | 73.2% | 6861 |
| `just test` | candidate | 5 | 29.20 / 36.01 / 49.89 s | 57.4% | 453.03 / 558.03 / 779.95 s | 58.6% | 6873 |
| `just test-rendezvous` | baseline | 5 | 9.35 / 11.60 / 19.92 s | 91.1% | 129.09 / 163.21 / 292.84 s | 100.3% | 272 |
| `just test-rendezvous` | candidate | 5 | 9.34 / 10.10 / 17.00 s | 75.8% | 129.11 / 141.34 / 247.74 s | 83.9% | 273 |

### Baseline → candidate (medians; established only outside both drift brackets)

| Suite | Column | Baseline | Candidate | Delta | Established |
|---|---|---:|---:|---:|---|
| `just test` | elapsed | 53.44 s | 36.01 s | -17.43 s (-32.6%) | **no — inside drift** |
| `just test` | summed | 836.68 s | 558.03 s | -278.64 s (-33.3%) | **no — inside drift** |
| `just test` | identities | 6861 | 6873 | +28 / −16 | — |
| `just test-rendezvous` | elapsed | 11.60 s | 10.10 s | -1.50 s (-13.0%) | **no — inside drift** |
| `just test-rendezvous` | summed | 163.21 s | 141.34 s | -21.87 s (-13.4%) | **no — inside drift** |
| `just test-rendezvous` | identities | 272 | 273 | +1 / −0 | — |

#### Paired alternation (candidate ÷ baseline, same sequence number)

| Suite | Column | Pairs | Ratio min / median / max | Every pair improved |
|---|---|---:|---|---|
| `just test` | elapsed | 5 | 0.563 / 0.653 / 0.876 | yes |
| `just test` | summed | 5 | 0.566 / 0.646 / 0.863 | yes |
| `just test-rendezvous` | elapsed | 5 | 0.662 / 0.963 / 1.077 | **no** |
| `just test-rendezvous` | summed | 5 | 0.643 / 0.951 / 1.095 | **no** |

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

`just test-rendezvous` — added 1, removed 0

- added: `rendezvous-daemon::pairing_and_sync endpoints_are_stable_per_fixture_and_distinct_across_fixtures`

### Changed cohorts (from the alternating suite runs; medians of summed duration)

| Cohort | Baseline tests | Baseline summed (min / median / max) | Candidate tests | Candidate summed (min / median / max) | Delta (median) | Established |
|---|---:|---|---:|---|---:|---|
| `cli-integration (all `claudine-cli::*` binaries; every one compiles the changed `common/`)` | 2489 | 399.76 / 423.76 / 862.94 s | 2498 | 248.63 / 314.40 / 404.02 s | -109.37 s (-25.8%) | no — inside drift |
| `cli-phase5A compose family` | 69 | 18.31 / 22.52 / 65.36 s | 69 | 13.38 / 17.15 / 19.05 s | -5.36 s (-23.8%) | no — inside drift |
| `cli-phase5B sequence / loop family` | 155 | 47.64 / 51.44 / 101.36 s | 155 | 39.12 / 53.14 / 55.62 s | +1.70 s (3.3%) | no — inside drift |
| `cli-phase5C context / errors / completion / resources family` | 84 | 49.66 / 50.89 / 95.61 s | 83 | 18.86 / 23.29 / 24.55 s | -27.60 s (-54.2%) | no — inside drift |
| `cli-phase5D live-child and PTY cohort` | 11 | 20.01 / 20.27 / 29.17 s | 12 | 5.50 / 6.68 / 7.33 s | -13.59 s (-67.0%) | yes |
| `cli-guards, probes and fixture self-tests` | 62 | 67.91 / 71.73 / 216.00 s | 71 | 16.91 / 21.52 / 25.28 s | -50.21 s (-70.0%) | no — inside drift |
| `cli-context_command alone` | 27 | 40.85 / 42.45 / 73.52 s | 26 | 11.76 / 14.06 / 15.37 s | -28.39 s (-66.9%) | no — inside drift |
| `cli-error_guards alone` | 18 | 58.47 / 61.99 / 200.79 s | 8 | 2.27 / 3.36 / 3.95 s | -58.63 s (-94.6%) | no — inside drift |
| `cli-sequence_overlay_pty alone` | 7 | 17.70 / 18.09 / 22.80 s | 7 | 3.89 / 4.35 / 4.57 s | -13.75 s (-76.0%) | yes |
| `lib-unit (all `claudine` lib tests)` | 4040 | 307.23 / 387.36 / 432.65 s | 4042 | 177.46 / 210.73 / 411.04 s | -176.63 s (-45.6%) | no — inside drift |
| `lib composition::schema` | 75 | 40.25 / 57.33 / 62.65 s | 75 | 10.46 / 14.61 / 33.50 s | -42.72 s (-74.5%) | yes |
| `lib composition::sequence::preflight` | 44 | 38.26 / 49.43 / 55.19 s | 44 | 3.08 / 4.35 / 9.49 s | -45.08 s (-91.2%) | yes |
| `lib composition::sequence::task` | 106 | 64.25 / 78.37 / 80.94 s | 106 | 14.18 / 15.42 / 16.40 s | -62.95 s (-80.3%) | yes |
| `lib linking::paths` | 10 | 12.23 / 15.79 / 18.00 s | 11 | 1.84 / 2.18 / 6.17 s | -13.60 s (-86.2%) | yes |
| `lib render::event_renderer` | 2 | 0.05 / 0.07 / 0.11 s | 4 | 0.10 / 0.13 / 0.17 s | +0.06 s (91.3%) | no — inside drift |
| `lib stream::stderr` | 21 | 0.59 / 0.62 / 0.78 s | 20 | 0.56 / 0.73 / 0.81 s | +0.11 s (17.4%) | no — inside drift |
| `cli-unit (`claudine-cli` bin tests, untouched)` | 1694 | 80.37 / 82.24 / 164.75 s | 1694 | 61.96 / 82.00 / 148.79 s | -0.24 s (-0.3%) | no — inside drift |
| `claudine-gen signals_validation` | 8 | 0.40 / 0.55 / 0.96 s | 9 | 0.63 / 0.88 / 1.91 s | +0.33 s (60.4%) | no — inside drift |
| `claudine-contract / catalog-types / gen (all, untouched except metadata)` | 223 | 21.66 / 30.42 / 73.28 s | 224 | 22.18 / 29.83 / 66.30 s | -0.59 s (-2.0%) | no — inside drift |
| `rendezvous-daemon pairing_and_sync` | 3 | 3.37 / 5.10 / 6.95 s | 4 | 3.76 / 4.32 / 7.41 s | -0.77 s (-15.2%) | no — inside drift |
| `rendezvous (all three crates)` | 272 | 129.09 / 163.21 / 292.84 s | 273 | 129.11 / 141.34 / 247.74 s | -21.87 s (-13.4%) | no — inside drift |

### Changed timing / concurrency contracts (every candidate execution, all roles)

| Identity | Contract | Executions | Min | Median | Max | Non-passing | Retries |
|---|---|---:|---:|---:|---:|---:|---:|
| `claudine composition::sequence::task::tests::shell_tasks::a_successful_command_reaps_its_backgrounded_descendant` | reap wait: sleep(1600 ms) → observe descendant pid gone, 10 s deadline, 5 ms cadence | 11 | 0.041 s | 0.049 s | 0.090 s | 0 | 0 |
| `claudine composition::sequence::task::tests::shell_tasks::an_early_wait_error_still_reaps_the_whole_tree` | reap wait → pid observation; descendant staged before the first stdout byte | 11 | 0.040 s | 0.058 s | 0.082 s | 0 | 0 |
| `claudine composition::sequence::task::tests::shell_streaming::no_frames_arrive_after_a_command_completes` | reap wait → pid observation | 11 | 0.038 s | 0.067 s | 0.106 s | 0 | 0 |
| `claudine composition::sequence::task::tests::shell_streaming::a_wait_error_settles_readers_before_returning_and_nothing_follows_the_footer` | reap wait → pid observation; descendant staged before the first stdout byte | 11 | 0.041 s | 0.060 s | 0.097 s | 0 | 0 |
| `claudine composition::sequence::task::tests::shell_tasks::the_system_shell_interrupts_a_running_tree` | interrupt readiness: sleep(300 ms) → marker wait, 30 s deadline, 5 ms cadence, released-by-marker asserted | 11 | 0.062 s | 0.075 s | 0.088 s | 0 | 0 |
| `claudine-cli::sequence_overlay_pty pty_sequence_auto_selectable_skips_review_and_launches` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed (concurrent) | 11 | 0.342 s | 0.406 s | 0.564 s | 0 | 0 |
| `claudine-cli::sequence_overlay_pty pty_sequence_invalid_agent_shows_preprompt_before_review` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed (concurrent) | 11 | 0.266 s | 0.344 s | 0.382 s | 0 | 0 |
| `claudine-cli::sequence_overlay_pty pty_sequence_prompt_dedupes_and_launches_all_steps` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed (concurrent) | 11 | 0.581 s | 0.753 s | 1.112 s | 0 | 0 |
| `claudine-cli::sequence_overlay_pty pty_sequence_status_report_honors_setter_supplied_required` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed (concurrent) | 11 | 0.623 s | 0.685 s | 1.095 s | 0 | 0 |
| `claudine-cli::sequence_overlay_pty pty_sequence_stderr_tty_with_stdout_redirected_prompts` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed (concurrent) | 11 | 0.303 s | 0.377 s | 0.458 s | 0 | 0 |
| `claudine-cli::sequence_overlay_pty pty_sequence_step_overlay_satisfies_required_property` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed (concurrent) | 11 | 0.638 s | 0.692 s | 1.211 s | 0 | 0 |
| `claudine-cli::sequence_overlay_pty pty_sequence_zero_installed_list_shows_preprompt_before_review` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed (concurrent) | 11 | 0.340 s | 0.387 s | 0.453 s | 0 | 0 |
| `claudine-cli::level1_compose_autocomplete_failure_pty level1_pty_compose_autocomplete_no_match_errors` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed | 11 | 0.243 s | 0.320 s | 0.557 s | 0 | 0 |
| `claudine-cli::level1_compose_autocomplete_failure_pty level1_pty_compose_autocomplete_over_cap_errors` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed | 11 | 0.485 s | 0.587 s | 0.848 s | 0 | 0 |
| `claudine-cli::level1_inline_compose_mismatch_pty level1_pty_mismatch_emits_sgr_and_osc8_link` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed | 11 | 0.347 s | 0.393 s | 0.695 s | 0 | 0 |
| `claudine-cli::level1_inline_compose_mismatch_pty level1_pty_mismatch_takes_tty_branch_with_yaml_block` | PTY harness answers OSC 10/11 on observation; `serial(pty)` removed | 11 | 0.359 s | 0.406 s | 0.757 s | 0 | 0 |
| `rendezvous-daemon::pairing_and_sync endpoints_are_stable_per_fixture_and_distinct_across_fixtures` | endpoint isolation keyed on fixture root, not pid alone | 11 | 0.287 s | 0.354 s | 1.062 s | 0 | 0 |
| `claudine-cli::level2_dry_run_pty level2_pty_dry_run_approval_prompt_matches_normal_mode` | `serial(pty)` removed; runs concurrently with the other PTY binaries at -j 8 | 11 | 4.599 s | 4.632 s | 4.713 s | 0 | 0 |
| `claudine-cli::level2_dry_run_pty level2_pty_dry_run_shell_approval_prompt_appears_and_allows` | `serial(pty)` removed; runs concurrently with the other PTY binaries at -j 8 | 11 | 2.505 s | 3.026 s | 3.110 s | 0 | 0 |
| `claudine-cli::level2_provided_partial_file_pty level2_pty_provided_partial_file_array_array_confirms_and_launches` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.788 s | 0.806 s | 0.868 s | 0 | 0 |
| `claudine-cli::level2_provided_partial_file_pty level2_pty_provided_partial_file_array_scalar_confirms_and_launches` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.783 s | 0.807 s | 0.873 s | 0 | 0 |
| `claudine-cli::level2_provided_partial_file_pty level2_pty_provided_partial_single_match_confirms_and_launches` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.783 s | 0.812 s | 4.609 s | 0 | 0 |
| `claudine-cli::level2_provided_partial_file_pty level2_pty_provided_partial_zero_match_preserves_error` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.361 s | 0.390 s | 0.466 s | 0 | 0 |
| `claudine-cli::level2_pty_tests level2_pty_non_interactive_detection` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.342 s | 0.349 s | 0.424 s | 0 | 0 |
| `claudine-cli::level2_pty_tests level2_pty_wrapper_summary_shows_badges` | `serial(pty)` removed; concurrent at -j 8 | 11 | 2.409 s | 2.422 s | 2.498 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_inline_compose_frontmatter_interactive_collects_before_launch` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.422 s | 0.431 s | 0.964 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_inline_compose_interactive_flag_collects_before_launch` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.413 s | 0.430 s | 0.461 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_prompt_appears_even_when_no_interactive_overrides_frontmatter` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.406 s | 0.416 s | 0.530 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_prompt_collects_boolean` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.397 s | 0.425 s | 0.543 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_prompt_collects_enum_selection` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.396 s | 0.410 s | 0.549 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_prompt_collects_string_and_launches_provider` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.399 s | 0.425 s | 0.529 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_prompt_number_retries_on_invalid_input` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.451 s | 0.471 s | 0.610 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_prompt_precedes_provider_launch_with_frontmatter_interactive` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.381 s | 0.398 s | 0.553 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_prompt_precedes_provider_launch_with_interactive_flag` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.379 s | 0.396 s | 0.457 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_silent_suppresses_prompt_under_tty` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.279 s | 0.292 s | 0.372 s | 0 | 0 |
| `claudine-cli::level2_schema_prompt_pty level2_pty_schema_status_does_not_report_templated_enum_as_invalid` | `serial(pty)` removed; concurrent at -j 8 | 11 | 0.396 s | 0.415 s | 0.564 s | 0 | 0 |

