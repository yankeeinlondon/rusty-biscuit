# Composition Errors Implementation Record

This records what was built and the evidence for it. Decisions awaiting review are in `decisions_for_review` in `implementation-log.md`; the log holds per-phase detail.

## Completed tasks

All six phases of `plan.md` are complete. The fix is ready for review.

## Compatibility interpretation (plan rule 23)

A command's existing successful data shape outranks the general statement that every result has an `outcome`:

- `md get` and a single or merged `md schema detect` print the bare value on a clean success, and `{ outcome, diagnostics, value }` when a run has something to report.
- `md clean` keeps its version 1 `diagnostics` rows; run-level records are `run_diagnostics`.
- `md toc`, `md delta`, `md graph`, `md validate refs`, and `md schema validate` gain `outcome` and `diagnostics` only on a failure or run warning.
- A failure with no result is `{ outcome: "failure", diagnostics, value: null }`.
- `md compose` and `md render` data, and the outputs new in this change, always carry `outcome` and `diagnostics`.

The public statement is `darkmatter/docs/cli/data-contracts.md`.

## Validation evidence (final tree)

| Check | macOS | Linux | Windows | WSL2 |
|---|---|---|---|---|
| renderable L1 | 733 | 733 | 733 | 733 |
| biscuit-terminal L1 | 3998 | 3612 | 3574 | 3612 |
| darkmatter-cli L1 | within the next row | 1879 | 1869 | 1879 |
| darkmatter L1 | 13654, with the skip below | L1 target only: 3999 of 4000 | 9968 of 9969 | 10022 of 10023 |
| lint, doctest, workspace compile | pass | not run | not run | not run |
| L2 | biscuit-terminal 260, darkmatter 90 | not run | not run | not run |
| headless browser | biscuit-terminal 56, darkmatter 44 | not run | not run | not run |

Counts differ by OS because some tests are OS-gated. The macOS row runs all of Darkmatter's packages under one recipe.

**Evidence gaps.** The one failure in each darkmatter cell, and the skip on macOS, is `every_kinded_schema_artifact_passes_its_definition_through_the_registry`, which rejects an in-progress research schema unrelated to this fix. Linux has no darkmatter archive run, because linking it ran `build-linux` out of memory twice, so the library's unit tests ran on WSL2 Ubuntu but not native Linux. L2 and browser evidence is macOS only.

## Acceptance

Each item in the spec's Acceptance section, by group. "Artifact" names a file under `artifacts/`.

| Acceptance item | Evidence | Result |
|---|---|---|
| Report rows name a reason and construct; no `is not valid`, no `failure:` row | `reference_report::a_report_has_no_failure_row_verdict_or_kind_heading`, `report.plain.txt` | pass |
| Transcluded issue under its group with its trail; blank line before the summary; equal file names distinct | `a_child_included_twice_is_two_groups_and_one_document`, `a_deep_issue_shows_its_whole_trail_outermost_first`, `report.plain.txt` | pass |
| Markdown location title, HTML `href` with no fragment or `data-*`, `.md` unchanged | `html_query_order_is_heading_then_line_column_then_sorted`, `report.markdown.md`, `report.html.html` | pass |
| OSC 8 URI bare, parameters `line:column:heading` | `a_location_is_portable_through_spaces_unicode_and_crlf`, `a_prose_json5_title_becomes_osc8_params`, `report.styled.escaped.txt` | pass |
| No highlight metadata | no `highlight` in any report artifact | pass |
| Authored positions, LF and CRLF, multibyte; root 9:5, 15:9, 22:13, 24:8; directive 26; inserted 9:7 | `reference_provenance` (`crlf_line_endings_report_the_same_positions`, `crlf_and_a_byte_order_mark_report_the_same_positions_together`, `an_inserted_reference_points_at_its_construct_in_the_report`), `report.plain.txt` | pass |
| In-memory positions null; `prologue:`/`epilogue:`; composed section slug | `an_in_memory_issue_serializes_null_positions`, `a_prologue_problem_is_reported_at_its_value`, `a_section_uses_the_final_slug_of_the_innermost_prior_heading` | pass |
| Same child twice keeps separate trails | `a_child_included_twice_is_two_groups_and_one_document` | pass |
| Link titles: JSON5, plain, malformed, legacy, `md clean` migration and fixed point | `link_metadata` tests, `a_legacy_title_warns_at_its_authored_position_and_says_to_run_md_clean`, `a_migrated_link_has_the_attributes_the_old_parser_found`, `a_fixed_width_clean_of_a_migrated_title_is_a_fixed_point` | pass |
| `md validate refs` lists warnings and a summary when nothing is broken | `a_clean_document_prints_the_same_all_ok_summary_on_both`, `report.plain.txt` | pass |
| Output selection, defaults, removed flags and values, `--compact`, help | `every_leaf_declares_its_output_and_width`, parser and help tests in `output_commands`, `render_help_states_its_default`, `streams-and-status.txt` | pass |
| `md validate refs --graph` unchanged | `conforms_validate_refs_show_all_lists_references`, graph-mode tests in `output_commands` | pass |
| `markdown-plus` disclosure forms | `a_disclosure_stays_a_directive_in_markdown_and_is_details_in_markdown_plus` | pass |
| `--show` with data, stubbed opener, open failure | `assert_show_writes_the_value`, `assert_show_failure_is_one_payload`, `render_html_show_failure_names_the_path_on_stderr` | pass |
| `md get --output plain` equals `--raw` | `raw_values_keep_todays_layout`; bytes checked against the binary in Phase 5 | pass |
| One dispatch; old report functions gone | `every_leaf_has_a_schema_file_and_a_conformance_test`; `print_validation_report_text` and the graph footer no longer exist | pass |
| `terminal` on a pipe equals `plain`; no ESC; element rules; URLs whole; link and image rule | `terminal_degrades_to_exactly_plain_off_a_color_tty`, `prepare_terminal_on_a_pipe_equals_plain`, `plain_corpus` (40, 80, unlimited), `a_long_printed_url_stays_whole_on_its_own_line_at_40`, `plain-corpus.*.txt` | pass |
| `md schema detect` outputs | `conforms_schema_detect_*`, `schema_detect` tests | pass |
| Width values, precedence, unlimited, floats, `N%`, stderr width, data outputs, source defaults | `the_width_precedence_is_flag_then_output_columns_then_terminal`, `a_percentage_of_nothing_is_unlimited_and_of_output_columns_is_floored`, `a_fractional_width_warns_on_stderr_for_a_formatted_output`, `stderr_ignores_the_deliverable_width_and_uses_its_own`, `output_width_columns_wins_over_fixed_width_frontmatter`, `schema_about_honors_a_width_narrower_than_a_page` | pass |
| Data: one value, empty stderr, with `-v` and `--perf`; `perf` in the payload | `json_output_is_unchanged_and_stderr_empty_under_double_verbose`, `a_perf_payload_validates_against_the_schema`, `streams-and-status.txt` | pass |
| Status 0, 1, 2 | `a_report_that_fails_is_on_stdout_with_status_one`, `a_usage_error_passes_through_unreported`, `streams-and-status.txt` | pass |
| Status 130, no value | `an_interrupted_run_writes_no_value_and_keeps_its_status`, `ctrl_c_runs_cancelled_then_finalize_and_exits_130` | pass, after a race fix |
| `--deny` blocking exits 1; `validate_references` error fails compose | `a_denied_missing_link_inside_inserted_text_blocks`, `a_reference_check_that_cannot_run_fails_the_run` | pass |
| Lifecycle output in order in data, on stderr otherwise | `lifecycle_output_in_markdown_is_on_stderr_and_stdout_is_the_document`, `compose_data_output` | pass |
| TOML: null fields omitted, null in an array fails with its path | dispatch tests in `biscuit-terminal/lib/src/output/data.rs`, `report.toml.toml` | pass |
| Every data output has a schema and a conformance test | `every_leaf_has_a_schema_file_and_a_conformance_test`, `schema-conformance-inventory.md` | pass |
| Locations identical across OSes | `a_windows_file_uri_stays_forward_slashed_on_every_os`, `a_location_is_portable_through_spaces_unicode_and_crlf`, run on Windows | pass |
| Report on stdout in the requested format | `graph_and_validate_refs_print_the_same_report_body` | pass |
| Compose recipe bytes unchanged | `the_plain_recipe_keeps_its_bytes`, `the_frontmatter_recipe_keeps_its_bytes` | pass |
| Editor stubbed | `conforms_edit_*`, `edit_removes_*`, `edit_keeps_*` use recorder scripts; nothing launches | pass |

## Visual artifacts

Under `artifacts/`, rebuilt by `generate.sh <path-to-md>`. Only the fixture root and timing values are normalized.

| File | Shows |
|---|---|
| `report.plain.txt`, `report.markdown.md`, `report.markdown-plus.md`, `report.html.html` | the root and child report in each formatted output |
| `report.json.json`, `report.yaml.yaml`, `report.toml.toml` | the same report as data |
| `report.styled.escaped.txt` | a real pseudo-terminal capture, with ESC written as `\e` so control bytes are reviewable |
| `plain-corpus.40.txt`, `plain-corpus.80.txt`, `plain-corpus.unlimited.txt` | the plain corpus at each width |
| `streams-and-status.txt` | status and stderr size for the contract's cases |
| `schema-conformance-inventory.md` | each output schema and its conformance tests |
