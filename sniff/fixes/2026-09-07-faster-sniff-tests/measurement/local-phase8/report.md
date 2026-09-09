### Runs

| Suite | Revision | Seq | Role | Wall | Build/setup | Runner elapsed | Summed | Tests | Passed | Failed | Timed out | Skipped | Leaks | Retries | Slow | Load before |
|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `l1-local` | baseline | 1 | warmup | 80.4 s | 62.4 s | 18.04 s | 266.33 s | 2599 | 2599 | 0 | 0 | 23 | 0 | 0 | 3 | { 43.36 31.66 21.81 } |
| `l1-local` | candidate | 1 | warmup | 46.2 s | 4.9 s | 41.31 s | 627.10 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 0 | { 60.43 35.76 24.02 } |
| `sanity` | baseline | 1 | warmup | 9.9 s | 2.4 s | 7.45 s | 94.18 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 48.88 36.21 24.84 } |
| `sanity` | candidate | 1 | warmup | 19.6 s | 3.7 s | 15.93 s | 237.94 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 59.94 38.91 25.93 } |
| `l2-tmux` | candidate | 1 | warmup | 14.8 s | 14.1 s | 0.68 s | 0.67 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 56.02 39.70 26.59 } |
| `l1-local` | baseline | 2 | alternating | 26.1 s | 1.1 s | 25.01 s | 386.29 s | 2599 | 2599 | 0 | 0 | 23 | 0 | 0 | 4 | { 52.81 39.86 26.88 } |
| `l1-local` | candidate | 2 | alternating | 55.9 s | 1.1 s | 54.82 s | 847.34 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 10 | { 85.01 48.74 30.54 } |
| `sanity` | baseline | 2 | alternating | 11.9 s | 2.5 s | 9.44 s | 136.28 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 63.97 49.00 31.78 } |
| `sanity` | candidate | 2 | alternating | 21.6 s | 2.4 s | 19.17 s | 289.29 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 3 | { 54.45 47.62 31.59 } |
| `l1-local` | baseline | 3 | alternating | 24.6 s | 1.2 s | 23.41 s | 357.40 s | 2599 | 2599 | 0 | 0 | 23 | 0 | 0 | 5 | { 61.55 49.97 32.90 } |
| `l1-local` | candidate | 3 | alternating | 45.7 s | 1.1 s | 44.59 s | 682.84 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 1 | { 114.59 62.48 37.85 } |
| `sanity` | baseline | 3 | alternating | 10.4 s | 2.4 s | 7.98 s | 108.61 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 69.59 58.40 37.76 } |
| `sanity` | candidate | 3 | alternating | 19.3 s | 2.4 s | 16.94 s | 254.82 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 90.81 63.26 39.72 } |
| `l1-local` | baseline | 4 | alternating | 22.4 s | 1.1 s | 21.26 s | 321.80 s | 2599 | 2599 | 0 | 0 | 23 | 0 | 0 | 3 | { 84.20 63.63 40.40 } |
| `l1-local` | candidate | 4 | alternating | 45.1 s | 1.0 s | 44.05 s | 671.84 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 1 | { 125.90 74.55 44.98 } |
| `sanity` | baseline | 4 | alternating | 10.5 s | 2.6 s | 7.94 s | 104.80 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 121.70 80.99 49.04 } |
| `sanity` | candidate | 4 | alternating | 19.4 s | 2.3 s | 17.03 s | 252.09 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 2 | { 134.10 84.98 50.83 } |
| `l1-local` | baseline | 5 | alternating | 22.7 s | 1.2 s | 21.53 s | 324.99 s | 2599 | 2599 | 0 | 0 | 23 | 0 | 0 | 3 | { 123.26 86.07 52.19 } |
| `l1-local` | candidate | 5 | alternating | 46.9 s | 1.2 s | 45.72 s | 699.42 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 0 | { 133.40 91.73 55.23 } |
| `sanity` | baseline | 5 | alternating | 11.1 s | 3.3 s | 7.79 s | 103.09 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 92.85 86.40 55.12 } |
| `sanity` | candidate | 5 | alternating | 21.1 s | 3.5 s | 17.58 s | 262.67 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 85.82 85.15 55.23 } |
| `l1-local` | baseline | 6 | alternating | 22.4 s | 1.4 s | 21.08 s | 321.77 s | 2599 | 2599 | 0 | 0 | 23 | 0 | 0 | 3 | { 62.78 79.80 54.16 } |
| `l1-local` | candidate | 6 | alternating | 46.6 s | 1.8 s | 44.80 s | 684.81 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 0 | { 105.30 88.29 57.96 } |
| `sanity` | baseline | 6 | alternating | 10.8 s | 3.0 s | 7.78 s | 102.51 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 89.90 85.79 58.55 } |
| `sanity` | candidate | 6 | alternating | 19.5 s | 2.8 s | 16.73 s | 251.15 s | 1820 | 1820 | 0 | 0 | 23 | 0 | 0 | 0 | { 90.39 86.09 59.14 } |
| `l1-local` | candidate | 7 | load | 45.7 s | 1.0 s | 44.69 s | 682.19 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 1 | { 72.09 82.17 58.36 } |
| `l1-local` | candidate | 8 | load | 45.5 s | 1.7 s | 43.75 s | 667.65 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 0 | { 63.10 77.70 58.03 } |
| `l1-local` | candidate | 9 | load | 45.4 s | 1.7 s | 43.68 s | 666.87 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 0 | { 55.16 72.60 57.11 } |
| `l1-local` | candidate | 10 | load | 44.4 s | 1.4 s | 42.98 s | 654.18 s | 2609 | 2609 | 0 | 0 | 24 | 0 | 0 | 0 | { 65.00 73.07 58.20 } |
| `l2-tmux` | candidate | 2 | load | 11.0 s | 10.3 s | 0.74 s | 0.74 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 89.88 77.52 60.54 } |
| `l2-tmux` | candidate | 3 | load | 10.2 s | 9.6 s | 0.67 s | 0.67 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 76.73 75.15 59.99 } |
| `l2-tmux` | candidate | 4 | load | 10.2 s | 9.5 s | 0.68 s | 0.68 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 68.25 73.37 59.54 } |
| `l2-tmux` | candidate | 5 | load | 10.4 s | 9.6 s | 0.72 s | 0.72 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 58.75 71.05 58.95 } |
| `l2-tmux` | candidate | 6 | load | 9.9 s | 9.2 s | 0.67 s | 0.67 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 51.42 69.07 58.39 } |
| `l2-tmux` | candidate | 7 | load | 14.3 s | 13.7 s | 0.67 s | 0.67 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 44.37 66.64 57.71 } |
| `l2-tmux` | candidate | 8 | load | 9.6 s | 8.9 s | 0.68 s | 0.67 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 46.92 66.10 57.67 } |
| `l2-tmux` | candidate | 9 | load | 9.6 s | 8.9 s | 0.68 s | 0.68 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 42.73 64.58 57.23 } |
| `l2-tmux` | candidate | 10 | load | 9.1 s | 8.4 s | 0.71 s | 0.71 s | 2 | 2 | 0 | 0 | 802 | 0 | 0 | 0 | { 39.53 63.16 56.81 } |

### Per-revision spread (alternating runs only)

| Suite | Revision | Runs | Elapsed min / median / max | Elapsed drift | Summed min / median / max | Summed drift | Identities |
|---|---|---:|---|---:|---|---:|---:|
| `l1-local` | baseline | 5 | 21.08 / 21.53 / 25.01 s | 18.2% | 321.77 / 324.99 / 386.29 s | 19.9% | 2599 |
| `l1-local` | candidate | 5 | 44.05 / 44.80 / 54.82 s | 24.1% | 671.84 / 684.81 / 847.34 s | 25.6% | 2609 |
| `sanity` | baseline | 5 | 7.78 / 7.94 / 9.44 s | 21.0% | 102.51 / 104.80 / 136.28 s | 32.2% | 1820 |
| `sanity` | candidate | 5 | 16.73 / 17.03 / 19.17 s | 14.3% | 251.15 / 254.82 / 289.29 s | 15.0% | 1820 |

### Baseline → candidate (medians; established only outside both drift brackets)

| Suite | Column | Baseline | Candidate | Delta | Established |
|---|---|---:|---:|---:|---|
| `l1-local` | elapsed | 21.53 s | 44.80 s | +23.27 s (108.1%) | yes |
| `l1-local` | summed | 324.99 s | 684.81 s | +359.82 s (110.7%) | yes |
| `l1-local` | identities | 2599 | 2609 | +13 / −3 | — |
| `sanity` | elapsed | 7.94 s | 17.03 s | +9.09 s (114.5%) | yes |
| `sanity` | summed | 104.80 s | 254.82 s | +150.02 s (143.1%) | yes |
| `sanity` | identities | 1820 | 1820 | +0 / −0 | — |

#### Paired alternation (candidate ÷ baseline, same sequence number)

| Suite | Column | Pairs | Ratio min / median / max | Every pair improved |
|---|---|---:|---|---|
| `l1-local` | elapsed | 5 | 1.905 / 2.124 / 2.192 | **no** |
| `l1-local` | summed | 5 | 1.911 / 2.128 / 2.194 | **no** |
| `sanity` | elapsed | 5 | 2.030 / 2.145 / 2.257 | **no** |
| `sanity` | summed | 5 | 2.123 / 2.405 / 2.548 | **no** |

#### Identity changes

`l1-local` — added 13, removed 3

- added: `sniff-cli::cli_process_fixture ambient_context_accepts_only_existing_fixture_directories`
- added: `sniff-cli::cli_process_fixture assert_and_raw_surfaces_produce_the_same_effective_environment`
- added: `sniff-cli::cli_process_fixture canonical_checkout_containment_rejects_nested_roots`
- added: `sniff-cli::cli_process_fixture canonical_checkout_containment_rejects_symlink_spelling`
- added: `sniff-cli::cli_process_fixture inherited_git_sniff_and_rendering_inputs_are_scrubbed`
- added: `sniff-cli::cli_process_fixture intentional_override_after_policy_wins_over_the_scrub`
- added: `sniff-cli::cli_process_fixture owned_command_keeps_its_disposable_launch_directory_alive`
- added: `sniff-cli::cli_process_fixture path_modes_are_bounded_and_keep_fixture_stubs_first`
- added: `sniff-cli::spawn_site_guard detector_finds_all_raw_spawn_forms_and_ignores_prose_and_strings`
- added: `sniff-cli::spawn_site_guard l1_tests_spawn_sniff_through_the_fixture_builder_or_an_explicit_entry`
- added: `sniff-cli::spawn_site_guard reconciliation_rejects_stale_and_unexplained_entries`
- added: `sniff-cli::spawn_site_guard restored_detector_rejects_a_violation_that_a_neutered_detector_misses`
- added: `sniff::benchmark_workloads seeded_git_execution_and_projection_do_not_rediscover_the_repository`
- removed: `sniff-cli::install_interactive_pty install_dry_run_emits_announcement_and_success_under_pty`
- removed: `sniff::foo bar`
- removed: `sniff::remote_providers shorthand_tests::from_shorthand_tries_github_when_token_set`

`sanity` — added 0, removed 0


### Changed cohorts (from the alternating suite runs; medians of summed duration)

| Cohort | Baseline tests | Baseline summed (min / median / max) | Candidate tests | Candidate summed (min / median / max) | Delta (median) | Established |
|---|---:|---|---:|---|---:|---|
| `all sniff-cli integration binaries using the shared process fixture` | 389 | 109.14 / 115.54 / 153.36 s | 400 | 89.67 / 94.63 / 120.92 s | -20.91 s (-18.1%) | no — inside drift |
| `CLI fixture isolation and structural guard` | 0 | 0.00 / 0.00 / 0.00 s | 12 | 0.26 / 0.26 / 0.30 s | +0.26 s (Infinity%) | yes |
| `library requested-work and representative fixture changes` | 9 | 21.65 / 23.45 / 29.48 s | 10 | 4.24 / 4.30 / 5.02 s | -19.15 s (-81.7%) | yes |
| `library inherited-Git fixture repairs` | 130 | 28.07 / 29.89 / 38.61 s | 130 | 49.70 / 51.17 / 64.64 s | +21.28 s (71.2%) | yes |
| `remote provider ownership audit` | 71 | 4.35 / 5.07 / 6.23 s | 70 | 157.08 / 158.90 / 174.84 s | +153.83 s (3036.0%) | yes |
| `strengthened embedded equality contracts` | 2 | 0.04 / 0.05 / 0.06 s | 2 | 0.04 / 0.04 / 0.07 s | -0.00 s (-3.3%) | no — inside drift |

### Changed timing / concurrency contracts (every candidate execution, all roles)

| Identity | Contract | Executions | Min | Median | Max | Non-passing | Retries |
|---|---|---:|---:|---:|---:|---:|---:|
| `sniff-cli::tty os_subcommand_runs_in_pty` | the real CLI emits its OS heading and reaches EOF within the explicit expect deadline while the session owns cleanup | 10 | 0.304 s | 0.364 s | 0.446 s | 0 | 0 |
| `sniff-cli::level2_cicd_styling level2_cicd_status_cells_render_styled_in_tmux` | bounded polling observes the complete final visible and styled CI/CD frame in a real tmux terminal | 10 | 0.317 s | 0.332 s | 0.378 s | 0 | 0 |
| `sniff-cli::level2_git_status_styling level2_git_status_headers_and_links_render_styled_in_tmux` | bounded polling observes the complete final visible, styled, linked, and laid-out Git frame in a real tmux terminal | 10 | 0.340 s | 0.345 s | 0.374 s | 0 | 0 |

