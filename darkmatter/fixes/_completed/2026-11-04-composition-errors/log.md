---
implementation_1: "2026-11-05T20:10:43-08:00"
implementation_2: "2026-11-05T22:51:09-08:00"
---

# Composition errors fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-05T20:10:43-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/fixes/2026-11-04-composition-errors/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review has no `human_in_the_loop` item and no blocked finding, so every finding is repairable in this cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | HTML reference discovery silently drops multiline tags and media targets | **fixed** | All rows below; scoped runs: `just test reference_` 514 passed, `just test reference_provenance::` 38 passed, `just test html` 227 passed, `just lint` clean |
| R1-F1.1 | HTML anchor: one line / attribute on next line — expected: One missing-target issue in both | **fixed** | `reference_html_discovery::anchor_on_one_line`, `anchor_attribute_on_next_line` (3:1) |
| R1-F1.2 | HTML image: same shapes — expected: One missing-target issue in both | **fixed** | `reference_html_discovery::image_on_one_line`, `image_attribute_on_next_line` (1:1), `image_after_text_on_its_line` (1:7) |
| R1-F1.3 | Script `src`: same shapes — expected: One missing-target issue in both | **fixed** | `reference_html_discovery::script_on_one_line`, `script_attribute_on_next_line` |
| R1-F1.4 | Stylesheet `<link>`: same shapes — expected: One missing-target issue in both | **fixed** | `reference_html_discovery::stylesheet_on_one_line`, `stylesheet_attribute_on_next_line` |
| R1-F1.5 | CSS `@import`: inline / multiline `<style>` — expected: Preserve the import and its issue | **fixed** | `reference_html_discovery::css_import_inline` (1:8), `css_import_multiline` (3:1), `css_import_after_multiline_open_tag` (2:17) |
| R1-F1.6 | CSS `@font-face` URL: same shapes — expected: Preserve the font URL and its issue | **fixed** | `reference_html_discovery::font_face_inline` (1:26), `font_face_multiline` (4:8), `font_face_unquoted_url` |
| R1-F1.7 | Video `src`: one line / multiline — expected: Discover and validate the target | **fixed** | `reference_html_discovery::video_on_one_line`, `video_attribute_on_next_line`; CLI: `<video\n src=...>` exit 1, kind `html_video`, construct `<video>` |
| R1-F1.8 | Audio `src`: same shapes — expected: Discover and validate the target | **fixed** | `reference_html_discovery::audio_on_one_line`, `audio_attribute_on_next_line` |
| R1-F1.9 | Source `src`: same shapes — expected: Discover and validate the target | **fixed** | `reference_html_discovery::source_on_one_line`, `source_attribute_on_next_line`, `source_nested_in_video` (2:3) |
| R1-F1.10 | Iframe `src`: same shapes — expected: Discover and validate the target | **fixed** | `reference_html_discovery::iframe_on_one_line`, `iframe_attribute_on_next_line` |
| R1-F1.11 | Meta tag: one line / multiline — expected: Preserve the metadata record; no missing-file error is required for an inline metadata value | **fixed** | `reference_html_discovery::a_meta_tag_is_one_inline_record_with_no_issue_in_either_shape`; CLI: multiline meta scanned 1, valid, exit 0 |
| R1-F1.12 | Markdown link and image controls — expected: Clean; these use separate Markdown extraction | **clean** | `reference_html_discovery::markdown_links_and_images_are_found_by_their_own_extraction` (extraction in `reference/local.rs` is untouched) |
| R1-F1.13 | Validation decision consuming the record set — expected: Missing targets make the report invalid and exit 1 | **fixed** | `reference_report::multiline_html_and_css_targets_report_invalid_and_exit_one` (CLI); manual: multiline image doc `valid: false`, exit 1, 1:1; multiline CSS doc exit 1, 2:1 and 4:8 |
| R1-F1.14 | Compose reference check and graph consumers — expected: They must receive the recovered records; do not fix only report rendering | **fixed** | `reference_html_discovery::a_child_documents_recovered_records_reach_the_graph_and_the_report`; the CLI test proves `md compose` and `md graph --validate` exit 1. Decision: a missing media or frame target has no `--allow` recovery, the same as CSS and script imports (alternative was stretching `missing-image-refs`, which names images only); documented in `docs/cli/policy.md` |
| R1-F1.15 | Discovered: a tag that is not first on its node line (`<p><img ...></p>`, several tags on a line) — expected: Each is its own record | **fixed** | `reference_html_discovery::image_inside_another_tag_on_its_line` (1:4), `each_tag_on_a_line_is_its_own_reference_with_its_own_position` |
| R1-F1.16 | Discovered: tag and attribute spelling (uppercase, tab after the name, unquoted value, value after a line break, whitespace around the target, `>` in a quoted value) dropped the record; `data-src` and text inside another attribute were read as the target — expected: Read as an HTML tokenizer does | **fixed** | `reference_html_discovery::uppercase_tag_and_attribute`, `tab_after_tag_name`, `unquoted_attribute`, `value_on_the_line_after_its_equals`, `whitespace_around_a_target`, `greater_than_in_earlier_attribute`, `attribute_named_like_the_target_with_a_prefix`, `target_text_inside_another_attribute` |
| R1-F1.17 | Discovered: `<img>` alone on a line of a multiline comment produced a false record; tag-like text in raw-text bodies is inert — expected: No record; fences keep their behavior | **fixed** | `reference_html_discovery::tag_on_its_own_line_of_a_multiline_comment`, `tag_in_one_line_comment`, `tag_in_a_fence`, `tag_in_inline_code`, `tag_in_an_inline_script_body`, `tag_in_a_textarea`, `tag_in_a_style_comment`, `import_in_a_css_comment` |
| R1-F1.18 | Discovered: a `<style>` body cut at the first `</`, an unclosed `<style>`, and a `<style>` in running text (CommonMark splits it into inline nodes) lost their CSS — expected: Body preserved | **fixed** | `reference_html_discovery::css_import_after_closing_tag_lookalike`, `css_import_in_unclosed_style`, `css_import_in_running_text` (1:13) |
| R1-F1.19 | Discovered: `@font-face` nested in `@media` was never visited — expected: Discovered | **fixed** | `reference_html_discovery::font_face_inside_media_query` (3:21) |
| R1-F1.20 | Discovered: CSS targets in a block quote or list fell back to the whole-tag position; CRLF, frontmatter, and non-ASCII columns — expected: Exact line and character column | **fixed** | `reference_html_discovery::css_in_block_quote` (2:3), `font_face_in_list_item` (3:10), `image_in_block_quote` (1:3), `second_tag_line_in_block_quote` (2:3), `crlf_line_endings`, `crlf_css_body`, `column_counts_characters_not_bytes`, `after_frontmatter`, `every_css_target_of_a_block_keeps_its_own_position`, `document_extraction_spans_index_the_text_it_was_given` |
| R1-F1.21 | Consequence of wiring media: `about:`, `blob:`, `javascript:` were classified as local paths and would read as missing files; an empty `src` aborted the whole run with `invalid-reference` — expected: Unsupported-scheme info, and no record for an empty target | **fixed** | `reference_html_discovery::iframe_about_blank`, `video_blob_url`, `anchor_javascript_url`, `empty_media_target`, `empty_anchor_target`. Decision: an empty target is no reference (a browser loads nothing); alternative was a report row, which needs a new issue code |
| R1-F1.22 | Remote, protocol-relative, `data:`, fragment-only, and `#t=` media targets — expected: Never a missing file; no fetch unless allowed | **clean** | `reference_html_discovery::video_remote_url`, `audio_protocol_relative_url`, `source_data_uri`, `a_fragment_only_media_target_is_a_fragment_not_a_file`, `a_media_fragment_does_not_hide_a_present_target` |
| R1-F1.23 | Discovered: `inline_css`, `css_imports`, `font_imports`, `inline_scripts`, `script_imports`, `meta_tags` over multiline blocks lost bodies and tags — expected: Same records as the graph | **fixed** | `reference_html_discovery::single_document_accessors_see_multiline_blocks_and_tags` |
| R1-F1.24 | Discovered: `set_meta_tag` ended a tag at a quoted `>` and appended a duplicate — expected: Replaces the tag | **fixed** | `reference_html_discovery::setting_a_meta_tag_replaces_a_multiline_tag_with_a_quoted_greater_than` |
| R1-F1.25 | Discovered: link resolve and normalization use these extractors, so a transcluded multiline tag kept a child-relative target — expected: Rewritten like a one-line tag. Span decision: an `<a>` record spans its open tag only, because the rewrite stages assume spans do not nest | **fixed** | `reference_html_discovery::composing_keeps_a_transcluded_multiline_tags_target_correct`; `just test link_` 235 passed |
| R1-F1.26 | DMLS index over `extract_document_references` — expected: Multiline link and image facts at document span and line | **fixed** | `graph::substrate::tests::a_multiline_html_tag_is_indexed_at_its_document_span_and_line`; `just test substrate` 27 passed |
| R1-F1.27 | `md graph` file tree: Media and Iframes groups existed but were never populated — expected: Rows appear | **fixed** | The CLI test in R1-F1.13 asserts `./missing.mp4` in the tree; manual `md graph --validate` shows Media and Iframes rows; `docs/cli/graph.md` table updated |
| R1-F1.28 | Syntax-kind consumers: `construct_phrase`, report construct text, `validate-refs.yaml`, `classify_reference_group`, `find_target_range` — expected: Handle media kinds | **clean** | `reference_report::every_syntax_has_its_construct`; `get_attribute_name_for_syntax` and `classify_reference_group` already list all four; `validate-refs.yaml` types `kind` as a plain string (`grep html_ cli/schemas` empty); manual JSON: `kind: html_video`, `construct: <video>` |
| R1-F1.29 | `<img src=./x.png>` — expected: Decide | **clean** | The Markdown parser does not make it an HTML node (`/` in an unquoted value), so it is paragraph text and Darkmatter never treats it as a tag; manual CLI: scanned 0. `<img src=x.png>` is found (`unquoted_attribute`) |
| R1-F1.30 | `srcset`, `poster`, `<track>`, `<embed>`, `<object>`, and `url()` outside `@font-face` — expected: bounded by the spec's construct contract | **clean** | The spec's exhaustive construct table names exactly the `ReferenceSyntax` variants now wired; no `ReferenceSyntax`, `ReferenceKind`, or extractor exists for these, so no authored record is dropped by a defect. Adding them is a feature extension; the limit is stated in `docs/cli/validate.md` and raised for author review |
| R1-F1.31 | Rewrite of an HTML target when another attribute holds the same value (`data-src="x" src="x"`, `SRC = "x"`) — expected: only the target attribute changes | **fixed** | `reference_html_discovery::composing_rewrites_the_target_attribute_not_one_sharing_its_value`; `find_target_range` now reads the value range from the HTML tokenizer (orchestrator repair after the subagent's hand-back) |
| R1-F2 | Diagnostic projection discards known source locations | fixed | Central projection in `compose/report_diagnostics.rs`; every row below dispositioned; 82 in-process tests in `lib/tests/l1/diagnostic_source_provenance.rs`, 12 CLI delivery tests in `cli/tests/l1/compose_report_diagnostics.rs` |
| R1-F2.1 | Schema preparation / unknown named type — expected: File-only source at minimum | fixed | `dm.schema.prepare` is file-only (`schema_preparation_names_its_file`; CLI `prepare.md`) |
| R1-F2.2 | Schema validation / wrong scalar type — expected: Clean; preserve it | clean | Authored 2:8 retained (`schema_validation_keeps_its_position`; CLI `schema.md`) |
| R1-F2.3 | Schema problems without a line — expected: Retain the file even without coordinates | fixed | A problem with no line keeps the enclosing path, null line/column (`a_schema_problem_with_no_line_keeps_its_file`) |
| R1-F2.4 | Page-block condition / parse and evaluation failures — expected: Preserve file; project authored coordinates where proven | fixed | `ConditionError` carries `authored`; parse and evaluation report the file and authored opener past a body rewrite, and the message names the file line (`a_block_condition_*`, `a_nested_block_condition_names_its_own_opener`, `a_block_condition_span_indexes_the_loaded_file`; CLI `block.md` 6:1) |
| R1-F2.5 | Transclusion condition / evaluation failure — expected: Clean; use this existing provenance pattern | clean | Authored 6:1 past a rewrite (`a_transclusion_condition_evaluation_keeps_its_position`; CLI `condition.md`) |
| R1-F2.6 | Transclusion condition / parse failure — expected: Retain known file and authored position | fixed | `TransclusionError::ConditionParse` carries `authored`; 6:1 past a rewrite (`a_transclusion_condition_parse_keeps_its_position`) |
| R1-F2.7 | Transclusion directive / omitted target — expected: Retain context; publish only proven authored coordinates | fixed | File-only: the parser's line is not proven authored, so it is not published (`an_omitted_transclusion_target_names_its_file`). See decisions |
| R1-F2.8 | Transclusion cycle / child includes itself — expected: Preserve the responsible child source rather than substituting the root | fixed | Names the chain's last document before the repeat, never the root; the chain line is not published (`a_cycle_names_the_document_that_closes_it`, `a_self_inclusion_names_the_child`, `a_cycle_chain_line_is_never_published`; CLI `transclusion.md` names `cycle.md`) |
| R1-F2.9 | Shell expansion / frontmatter approval refusal — expected: Retain file and known frontmatter location | fixed | File and key line, null column; no command runs (`a_frontmatter_command_names_its_key_line`; CLI `frontmatter_shell.md` 2:null) |
| R1-F2.10 | Other shell-expansion variants / shared projection — expected: Preserve each variant's available context; do not change execution policy | fixed | All 15 context variants through `ShellExpansionError::context`, each tested at a frontmatter origin and a body origin (`shell_*::at_a_frontmatter_key`, `shell_*::in_a_body`); `PolicyIo` names its policy file; execution policy untouched |
| R1-F2.11 | Shell-block errors / source-file-bearing variants — expected: Retain known file; avoid treating prepared lines as authored | fixed | File-only; a prepared line is never published (`a_shell_block_failure_names_its_file`, `a_shell_block_command_falls_back_to_its_command_file`, `a_shell_block_command_publishes_no_prepared_line`) |
| R1-F2.12 | Ordinary warnings / undeclared body and frontmatter properties — expected: Preserve known file/line in the record | fixed | `undeclared_property` records its document and proven line, null column (`an_undeclared_body_read_names_its_line` 5, `an_undeclared_frontmatter_read_names_its_key_line` 2; CLI `warnings_and_a_typeless_failure_name_the_document`) |
| R1-F2.13 | Condition context warning / `ctx.toady` after multiline interpolation — expected: Carry the authored directive location before projection | fixed | `ConditionSite::locate_warning` carries the authored opener from the producer at all three evaluators: page blocks (top and nested), transclusion, and the collection traversal (`a_*_condition_warning_*`, `a_condition_warning_span_indexes_the_loaded_file`) |
| R1-F2.14 | Warnings already carrying a diagnostic / legacy link title — expected: Clean; retain the shared record | clean | `warning_diagnostic` returns a carried record unchanged; `link_title_warnings` filter passed, authored 1:1 reproduced with `md render` |
| R1-F2.15 | Body and frontmatter interpolation errors / division by zero — expected: Clean | clean | `a_body_interpolation_keeps_its_position` 3:1, `a_frontmatter_interpolation_keeps_its_position` 3:9 |
| R1-F2.16 | Collection failure / undeclared iterable — expected: Clean | clean | `collection_analysis` located-failure test passed under the `collection` filter; `proven` still returns the site |
| R1-F2.17 | Reference issue / missing Markdown target — expected: Clean | clean | `reference_issue_diagnostic` output unchanged (only its file match moved to `ComposeSource::document_path`); `reference_diagnostics_are_authored_under_either_line_ending` passed |
| R1-F2.18 | Reference-error projection — expected: Preserve file-only information when carried or retain it at the failing call site | fixed | The directive branch keeps its file; a `FileReference`/`Io`/`Validation` error holds no file and takes the composed document at `with_document_fallback` (`a_reference_failure_keeps_what_it_holds`). See decisions for a child's failure |
| R1-F2.19 | Frontmatter parsing / malformed YAML — expected: Clean; retain file-only fallback when no parser position exists | clean | Located error retained (`each_failure_is_classified_by_its_typed_error` 3:1); the branch with no parser position gained the file-only fallback (`a_frontmatter_error_with_no_position_names_its_file`) |
| R1-F2.20 | Interpolation/collection source references without a proven span — expected: Retain the known file; leave line/column null | fixed | `located` maps `SourceRef::OnDisk` to the file with no span; `Supplied` and `Effective` stay source-free (`an_unproven_interpolation_keeps_its_file`) |
| R1-F2.21 | Parameter-binding / caller-classification and generic fallback branches — expected: Preserve source if the typed error/caller has it; null remains valid for an actually source-free foreign error | fixed | These errors hold no file, so `error_diagnostics` stays null for them and `with_document_fallback` supplies the caller's document; a foreign error with no caller stays null (`a_failure_holding_no_file_takes_the_composed_document`, `a_foreign_error_takes_a_family_code_and_no_source`) |
| R1-F2.22 | Lifecycle error text with no typed source / `error: boom` — expected: Clean for a source-free error value; a typed lifecycle call finding already carries its own record | clean | `error: boom` has a null source through the sink, which applies the fallback only to a failure with a typed cause (CLI `lifecycle.md`) |
| R1-F2.23 | Blocking-policy and lifecycle delivery paths — expected: Preserve the same corrected source records through every exit, including earlier warnings | fixed | Warnings are stamped as they enter the report, and stage-held warnings when a stop clones them, so every exit carries them (`a_policy_stop_keeps_every_warning_located`, `a_policy_stop_inside_a_stage_names_its_document`, `a_child_policy_stop_names_the_child`, `a_failure_keeps_located_earlier_warnings`; CLI `--deny all` case and `lifecycle_shell.md`) |
| R1-F2.24 | Discovered: `PageBlockError` directive variants (parse, unmatched end, unterminated) held a context but projected null | fixed | File-only; the parser counts the current body (`an_unmatched_end_block_names_its_file`, `an_unterminated_block_names_its_file`, `a_page_block_directive_failure_publishes_no_body_line`) |
| R1-F2.25 | Discovered: transclusion variants holding a context (`InvalidReference`, `InvalidFrontmatterAssignment`, `InvalidReassignedFrontmatterProperty`) projected null | fixed | File-only (`a_transclusion_failure_with_a_context_names_its_file`) |
| R1-F2.26 | Discovered: a warning merged from a child has no record of which document reported it | fixed | `ComposeReport::document` stamps each warning on entry, and a merge keeps the child's stamp (`a_child_warning_names_the_child`) |
| R1-F2.27 | Discovered: `frontmatter_key_line` matched an indented key or block-scalar text of the same name, giving a wrong line that this repair would have published | fixed | Unindented lines only (`frontmatter_key_line_skips_a_nested_key_of_the_same_name`, `a_frontmatter_command_is_not_located_at_a_nested_key_of_its_name`); also corrects undeclared-property lines |
| R1-F2.28 | Discovered: a frontmatter value's own `SourceContext` is named after its key, so projecting `ctx.display` would publish the key as a file | fixed | `context_file` requires an absolute path (`a_context_named_after_a_key_is_no_file`) |
| R1-F2.29 | Discovered: `with_iteration_context` filled only a null source, and a repeated gate now has its directive's opener, narrower site lost | fixed | A record with no line, or a gate's opener, takes the copy's proven site (`a_repeated_gate_keeps_its_expression_site`) |
| R1-F2.30 | Discovered: a schema record could name the `<stdin>` placeholder as its file, against the schema's "null for stdin" | fixed | `named` rejects placeholders (`a_placeholder_path_is_no_file`; CLI `stdin_names_no_file`) |
| R1-F2.31 | Discovered: other warnings carry `at_line` counted in the current body (unknown options, set errors, heading overflow, file-links, volatile command) | fixed | They report their document, null line/column; `line_number` is never projected (`an_uncoded_warning_counted_in_the_body_names_only_its_document`, `a_body_context_warning_names_its_document`, `a_warning_with_no_document_has_no_source`). See decisions |
| R1-F2.32 | Discovered: `dm.schema.invalid_trigger` built a null record for a local trigger with no position although its file is known | fixed | Local trigger is file-only, embedded stays null (`a_trigger_failure_without_a_position_names_only_its_file`, `a_payload_failure_names_the_payload_but_never_places_it_on_the_trigger`, `an_embedded_trigger_record_has_no_source_even_with_a_position`, CLI `invalid_trigger_warnings`); two pins and `docs/cli/data-contracts.md` updated |
| R1-F2.33 | Discovered: `md render` style diagnostics are built with no source | fixed | `ReportSink::document_diagnostics` (`a_render_style_warning_names_the_document`) |
| R1-F2.34 | Sweep: `Diagnostic` construction outside the projection | clean | `output_diagnostic` is about the run (`--width`, serialization, open failure), so null is correct; the other `source: None` hits under `lib/src` and `cli/src` are `SchemaError`, lifecycle error, or `raw_source` fields, not diagnostics |
| R1-F2.35 | Sweep: other commands' run diagnostics | **fixed** | The subagent's clean disposition was reversed at reconciliation: `docs/cli/data-contracts.md` allows a null `source` only when no file is known, and `md validate refs`, `md graph`, and `md schema triggers` hold one document. `CommandRun::set_document` applies the shared fallback; `compose_report_diagnostics::validate_refs_names_its_document_on_a_failure_with_no_file`, `::graph_names_its_document_on_a_failure_with_no_file`. Commands whose failures carry typed sources (`md toc` on bad YAML reports 3:1) or that span several files (`md delta`, directory `md hash`, `md schema validate` per-file records) do not name one document |
| R1-F3 | Unlimited component output still uses a million-column sentinel | **fixed** | `Terminal` keeps the width state (`unbounded_width`, `output_width()`, `columns()`) and `UNBOUNDED_COMPONENT_COLUMNS` is gone from both crates; every row below dispositioned; scoped filters, lint, and dependent `cargo check` all pass (see Progress) |
| R1-F3.1 | Bridge + horizontal rule / `Columns(40)` — expected: Bounded; control establishes fill behavior | **clean** | `output_component_bridge::a_rule_fills_columns_exactly`: 40 characters, 120 bytes, as the review observed. See decisions: this bridge control is 40 glyphs in a plain profile although the tree fold prints `---` at every width |
| R1-F3.2 | Same / `Source` — expected: Natural output, with no manufactured million-column width | **fixed** | `a_plain_rule_with_no_width_is_three_dashes` and `a_styled_rule_with_no_width_is_three_glyphs_and_never_an_image` at `Source`: `---`, and 3 glyphs styled |
| R1-F3.3 | Same / `Unlimited` — expected: Natural output; plain rule should be `---` | **fixed** | The same tests at `Unlimited`: exactly `---`, 3 bytes; `an_aligned_rule_with_no_width_has_no_slack_to_place_it_in`, `a_rule_keeps_its_left_margin_with_no_width` |
| R1-F3.4 | Generic terminal fold / rule, table, code corpus — expected: Clean; preserve this route | **clean** | Fold unchanged for bounded widths; `render_terminal_output` and `output_dispatch` under `just test output` 140 passed, `just test render_tree` 305 passed |
| R1-F3.5 | Darkmatter page / plain corpus at 40, 80, unlimited — expected: Clean for the documented corpus | **clean** | `darkmatter` `just test page::` 134 passed, `just test width` 273 passed, `just test output_` 462 passed; `DarkmatterPage::new` now also keeps an unbounded terminal (R1-F3.19) |
| R1-F3.6 | CLI bridge callers: toc, delta, graph — expected: Clean for tested inputs; adapter still discards the enum | **fixed** | The adapter retains the enum. `output_width_overrides::toc_at_no_width`, `toc_at_source`, `delta_at_no_width`, `graph_at_no_width`, control `graph_truncates_at_a_column_count` |
| R1-F3.7 | CLI bridge callers: code-block, schema-about, schema-validate — expected: Clean for tested inputs; styled code-block also uses this adapter | **fixed** | `code_block_at_no_width`, `schema_about_at_no_width`, control `schema_about_keeps_its_margins_at_a_column_count`, `schema_validate_at_no_width`; styled band `yaml_block::test_render_hugs_the_longest_line_with_no_width` (longest line plus padding, all rows equal) |
| R1-F3.8 | CLI bridge callers: clean saved-delta and schema-triggers — expected: Keep enum semantics through these overrides too; test their successful rendering paths | **fixed** | `clean_save_reports_its_delta_at_no_width`, `clean_save_reports_a_frontmatter_repair_at_no_width`, `schema_triggers_at_no_width`, control `schema_triggers_wraps_a_root_at_a_column_count`, all through the successful rendering path |
| R1-F3.9 | Markdown/browser folds — expected: Clean by the typed option contract | **clean** | Their options carry `Width` and never built this adapter; no change. `just test output_` covers both targets |
| R1-F3.10 | Discovered: `OutputEnvironment::stderr_terminal` held its own `1 << 20` copy for a piped stderr — expected: Same enum | **fixed** | `set_output_width(resolve_stderr_width(..))`; constant removed; `output::tests::stderr_ignores_the_deliverable_width_and_uses_its_own`, `just test stderr` 63 passed |
| R1-F3.11 | Discovered: `TerminalRenderContext::from_terminal` made every terminal `Columns(width())`, so every tree-routed component (`Table`, lists, `Section`, `TextBlock`, `Todo`, `StatusBlock`, default `BlockQuote`, `TwoColumn`) lost the state — expected: Natural | **fixed** | Reads `output_width()`; `a_table_hugs_its_content_with_no_width_and_fills_nothing`, `a_block_quote_stays_on_its_line_with_no_width`, `a_list_item_stays_on_its_line_and_a_nested_rule_stays_short`, `two_columns_take_their_content_width_with_no_width`, `source_keeps_a_soft_break_and_unlimited_joins_it` |
| R1-F3.12 | Discovered: the fold's own terminal snapshot answered a finite 80 (`from_profile`, Darkmatter `base_render_context`) or stayed stale (`narrowed_opts`) under an unbounded width — expected: Follows the state | **fixed** | All three use `set_output_width`; `a_custom_border_quote_gives_a_nested_rule_no_width`; `just test render_tree` 305 passed |
| R1-F3.13 | Discovered: `Prose` and `Status` post-wrap with `term.width()` — expected: No wrap | **fixed** | `wrapping_prose_stays_on_its_line_with_no_width`, `a_status_description_stays_on_its_line_with_no_width`, with `Columns(40)` controls |
| R1-F3.14 | Discovered: `BlockQuote::with_border` bespoke path wraps and gives a nested component a number — expected: Neither | **fixed** | `a_custom_border_quote_stays_on_its_line_with_no_width`, `a_custom_border_quote_gives_a_nested_rule_no_width` |
| R1-F3.15 | Discovered: truncation at a numeric width in `MetricsTree`, `FileSystem`, Darkmatter `FileTree`, and `apply_layout` wrapping in `FileTree` — expected: Never cut | **fixed** | `a_metric_label_keeps_every_character_with_no_width`, `a_file_name_keeps_every_character_with_no_width`, `graph_at_no_width` with truncating controls |
| R1-F3.16 | Discovered: `Table` cursor-aligned path and `plan_widths_for_terminal` read a number; a styled bridge terminal would move to a column of a filled width — expected: Tree path, natural plan | **fixed** | `a_cursor_aligned_table_with_no_width_has_no_column_to_move_to`, `a_table_plan_for_a_terminal_with_no_width_wraps_nothing` |
| R1-F3.17 | Discovered: `TerminalImage` Kitty, iTerm2, Sixel, half-block, and inline routes sized a fill against the number — expected: Natural size | **fixed** | `TerminalImage::resolve_unbounded_dimensions_for` through one `dimensions_on`; `an_image_with_no_width_to_fill_takes_its_natural_size`; `just test terminal_image` 223 passed |
| R1-F3.18 | Discovered: `MermaidDiagram` image size, text cap, and block layout; `GraphExpression` block layout — expected: Natural | **fixed** | `display_size::a_mermaid_image_with_no_width_takes_its_natural_size`, `component_fallback_tests::a_text_drawing_has_no_cap_with_no_width`; `just test mermaid` 214 passed |
| R1-F3.19 | Discovered: `DarkmatterPage::new` and `ValidationReportView`'s `TerminalRenderable::render` rebuilt a width from a number — expected: Keep the state | **fixed** | `a_page_keeps_a_terminal_with_no_width_unbounded`; `render` prefers `unbounded_width`, so `Source` survives; `just test graph` 275 passed |
| R1-F3.20 | Discovered: `md schema about` laid out in `width() - margins` with `render_in_width`, which would wrap at the numeric answer — expected: No fit under no width | **fixed** | `width` is `Option<u32>`, one `fit` helper; `schema_about_at_no_width`, control at 60; `just test schema_about` 33 passed |
| R1-F3.21 | Discovered: the compose perf report capped the sentinel at 120 so its columns stayed a page wide — expected: Decide | **fixed** | With no stderr width it now hugs its content, per the spec's stderr rule; a known stderr width is still capped at 120; `just test compose_perf` 14 passed, `perf_report` 3 passed |
| R1-F3.22 | Sweep: numeric reads that stay — `TwoColumn` holding a `TerminalImage`, `GraphExpression` image, `GitGraph` viewport — expected: Bounded, no manufactured large width | **clean** | They need a concrete budget before anything can be measured, so they take `Terminal::width()`'s 80 and say so at the site and in `biscuit-terminal/docs/output.md`; no CLI route renders them on a bridge terminal. Raised for review, see decisions |
| R1-F3.23 | Sweep: other `1 << 20`, `u32::MAX`, `usize::MAX`, `as_columns().unwrap_or`, and `from_output_options` callers — expected: Classified | **clean** | `TMUX_MESSAGE_BUDGET` is bytes; `natural_render_width` and the Mermaid hook's `u32::MAX` are measurement or no-cap inputs that never fill; `as_columns().unwrap_or(0)` is the tree's percent basis; `render_optimistic`'s 80 is its documented explicit route; all 9 Darkmatter CLI `from_output_options` callers go through this adapter |
| R1-F3.24 | Sweep: other crates reading a terminal width (`claudine`, `sniff`, `worktree`, `tree-hugger`, `biscuit-icon`, `bt`) — expected: Direct constructors unchanged | **clean** | They build detected or builder terminals, which are always numeric: `a_directly_built_terminal_stays_numeric`; `cargo check` of 15 dependent packages passes |
| R1-F4 | Frontmatter flow-list references do not report their value locations | **fixed** | Each value reports its own position in the graph, report, and diagnostics; the key fallback is gone; every row below dispositioned; `reference_provenance::` 98, `reference_report::` 46, `reference_` 577, `frontmatter` 1110 passed, lint clean (see Progress) |
| R1-F4.1 | Prologue scalar — expected: Clean | **clean** | `reference_provenance::a_scalar::prologue` 2:11 |
| R1-F4.2 | Epilogue scalar — expected: Clean | **clean** | `a_scalar::epilogue` 2:11 |
| R1-F4.3 | Prologue block sequence — expected: Clean | **clean** | `a_block_sequence::prologue` 3:5 and 4:5 |
| R1-F4.4 | Epilogue block sequence — expected: Clean | **clean** | `a_block_sequence::epilogue` 3:5 and 4:5 |
| R1-F4.5 | Prologue one-line flow sequence — expected: 2:12 and 2:20 | **fixed** | `a_one_line_flow::prologue` 2:12 and 2:20; `md validate refs` reproduced; the pinning test `a_flow_list_value_falls_back_to_its_key` is removed |
| R1-F4.6 | Epilogue one-line flow sequence — expected: 2:12 and 2:20 | **fixed** | `a_one_line_flow::epilogue` 2:12 and 2:20 |
| R1-F4.7 | Prologue multiline flow sequence — expected: 3:3 and 4:3 | **fixed** | `a_multiline_flow::prologue` 3:3 and 4:3; also CRLF, BOM, comments, trailing comma, repeated values, and a list opening its own line, each for both properties |
| R1-F4.8 | Epilogue multiline flow sequence — expected: 3:3 and 4:3 | **fixed** | `a_multiline_flow::epilogue` 3:3 and 4:3; `md validate refs` reproduced |
| R1-F4.9 | Prologue child traversal / multiline flow sequence of existing children with broken links — expected: Parent trail lines 3 and 4 | **fixed** | `a_prologue_flow_list_keeps_each_childs_trail_line`: insertions and report trails at lines 3 and 4; CLI `(transcluded at trail.md:3)` and `:4` |
| R1-F4.10 | Epilogue child traversal / same shape — expected: Parent trail lines 3 and 4 | **fixed** | `an_epilogue_flow_list_keeps_each_childs_trail_line`: lines 3 and 4 |
| R1-F4.11 | Report and diagnostic consumers — expected: Correct the shared location; preserve it in data and location links | **fixed** | Corrected at the shared `origin.authored`, which report rows, JSON, location links, and diagnostics read; CLI `a_flow_list_reports_each_value_in_data_and_text` (JSON 3:3, trail 4, text `root.md:3:3`, `md compose` `source` 3:3); terminal output shows `one.md:2:12` and `:2:20` |
| R1-F4.12 | Discovered: one multi-line or commented flow collection anywhere made `locate_frontmatter_value` fail for the whole frontmatter, so every schema problem in the document fell to its top-level key at column 1 | **fixed** | The tolerant locator reads flow collections across lines and past comments; `positions::a_multiline_flow_collection_keeps_every_position_in_the_document`; `md schema validate` reports `/nums/1` at 6:3; `docs/topics/schemas/definition.md` limit removed |
| R1-F4.13 | Discovered: a frontmatter interpolation error lost its position in, or merely beside, a multi-line flow | **fixed** | `a_frontmatter_interpolation_in_a_multiline_flow_keeps_its_position` 4:4, `a_multiline_flow_elsewhere_leaves_a_frontmatter_interpolation_its_position` 5:9 |
| R1-F4.14 | Discovered: a value with a tag, anchor, or alias fell back to its key (`[!!str ./a.md]`, `prologue: *x`, `prologue: &x ./a.md`) | **fixed** | Reports the token that opens the value: `a_tagged_entry_at_its_tag`, `an_anchor_and_its_alias`, `a_block_alias_at_its_token`, `a_block_anchor_at_its_token` |
| R1-F4.15 | Discovered: a comment after the key or the closing bracket lost the value (`prologue: # note`, `prologue: [./a.md] # note`) | **fixed** | `a_comment_after_the_key` 3:5, `a_comment_after_the_flow` 2:12 |
| R1-F4.16 | Discovered: an indented frontmatter root had a null position, since only the writer's locator was tried | **fixed** | `an_indented_root` 2:14 |
| R1-F4.17 | Single flow entry and a literal flow item's links | **fixed** | `a_single_flow_entry` 2:12; `a_literal_flow_item_locates_its_links_at_the_item` 3:3 |
| R1-F4.18 | Value text: quotes, `,`/`]` in quotes, trailing comma, multibyte, BOM, CRLF, repeated values, continued plain entry | **fixed** | `quoted_flow_entries`, `quoted_flow_punctuation` (2:12, 2:25), `a_trailing_comma`, `multibyte_earlier_in_the_line` (characters: 2:12, 2:20, 2:30), `a_byte_order_mark*`, `crlf_*`, `repeated_identical_values*` (index, not text search), `a_plain_entry_continued_on_a_line` (2:12, 3:9) |
| R1-F4.19 | A value that cannot be proven — expected: null, never a column or its key | **fixed** | `an_unreadable_sibling_gives_a_flow_item_no_position` null; the key fallback branch is deleted. `an_unreadable_sibling_leaves_a_scalar_its_position` 3:11 keeps what the per-property locator proves |
| R1-F4.20 | Sweep: `locate_frontmatter_leaves`, the writer Claudine's `persist.rs` uses, must keep refusing a multi-line flow now that the locator reads one | **clean** | `is_multi_line_flow` guard; `a_multi_line_flow_stays_refused_past_a_tag_or_anchor`, existing `quoted_flow_items_are_located_and_plain_ones_are_not`; `hash::write` 76 passed; `claudine` `just test persist` 41 passed. See decisions for the one widening |
| R1-F4.21 | Sweep: `included_via`, grouping by `(source, included_via)`, location links, JSON | **clean** | All read `origin.authored` or `authored_directive_line`, which both take this one location (`graph.rs` `value_location`); `reference_report::` 46 passed |
| R1-F4.22 | Sweep: `frontmatter_key_line` users (`dm.shell.failure`, preflight, undeclared property, call precheck) | **clean** | Not references. A `$( )` is a whole top-level value, so its value is on or under that key, and these report the key's line with a null column, as `docs/errors/composition-problems.md` states; the undeclared-property row now says so. `call_precheck` prefers the projected expression and gains R1-F4.13's positions |
| R1-F4.23 | Sweep: DMLS | **clean** | No reference report or `PositionMap` here: `problem_range` ranges through its own AST's `value_range`, and `frontmatter_prologue_epilogue` links scalar values at their value span, never a key |
| R1-F4.24 | Sweep: `Markdown::transclusions_with_options` builds prologue and epilogue origins with `authored: None` | **clean** | A null, not a substituted location; that API carries prepared-frame records and no report reads them |
| R1-F5 | The new styled reference report lacks real-terminal verification | **fixed** | `darkmatter/cli/tests/level2/level2_reference_report.rs` adds 7 Level 2 tests on tmux and WezTerm; each row below has its evidence |
| R1-F5.1 | Grouped report reason colors and dimmed location — expected: Level 2 capture through a real terminal backend | **fixed** | `level2_validate_refs_shows_styled_groups_in_aligned_columns` on tmux reads each cell's style by replaying the capture's SGR: groups in order, reason red, construct italic, reference set apart, file name not dim, `:line:col` dim, headings bold, trail dim, count bold red |
| R1-F5.2 | Aligned report columns and narrow-width stacked fallback — expected: Level 2 capture retaining every reference/location at a representative narrow width | **fixed** | Same test asserts reference and location columns line up at 120. `level2_narrow_pane_stacks_rows_and_keeps_every_reference_and_location_whole` resizes the pane to 30: reason at column 2, reference and location whole on their own rows at column 4, no row over 30, the reason and dim location styles kept. The capture showed no defect here |
| R1-F5.3 | Existing schema/expression/file-reference error blocks — expected: Clean for those older blocks; they do not cover the new grouped report | **clean** | Read `level2_errors.rs`: its 8 tests cover the OSC 8 header, schema block, invalid file reference, suggestions, and excerpt gutter, none the grouped report. `just test-l2 level2_errors`: 8 passed |
| R1-F5.4 | Plain escape suppression, JSON streams, exit codes — expected: Clean; an emulator adds no evidence to these byte contracts | **clean** | Level 1: `cli/tests/l1/reference_report.rs::graph_and_validate_refs_print_the_same_report_body` (exact escape-free bytes on a pipe, exit 1), `::compose_reports_broken_references_on_stderr_without_the_old_wording` (empty stdout, exit 1), `compose_data_output.rs::an_unresolved_reference_is_a_failure_that_exits_1` (JSON, empty stderr), `validate_refs.rs::validate_refs_nonzero_exit_on_errors`; rerun, passing |
| R1-F5.5 | Keyboard behavior — expected: No Level 3 test required | **clean** | The report reads no input; the new tests send only the shell command line through the harness, to background panes |
| R1-F5.6 | Sweep: `md graph --validate` — expected: the report after the tree, styled alike | **fixed** | `level2_graph_validate_shows_the_same_styled_report_after_its_tree` runs the styled-report and column assertions on the rows after the tree |
| R1-F5.7 | Sweep: `md compose` refusal on stderr — expected: judged by stderr's own terminal | **fixed** | A distinct route (`output::stderr_report`, its own TTY and columns). With stdout redirected, `level2_compose_refusal_styles_the_report_on_stderr_with_stdout_redirected` proves styling at 120 and an empty deliverable; `level2_compose_refusal_stacks_at_the_width_of_stderrs_pane` proves stacking at 30 |
| R1-F5.8 | Sweep: warning and info reason colors, dim hint and tally — expected: yellow and dim, per the spec's row rule | **fixed** | The capture found a real defect: a glob hint wrapped at 72 columns split `::file-links`. A borderless spanning row ignored its first column's wrap policy (`emit_borderless_table`); it now honors it, and the report's first column breaks only at spaces. Proof: `level2_reason_color_follows_severity_and_a_hint_wraps_between_words`, `lib/tests/l1/reference_report.rs::a_glob_hint_wraps_between_words_at_every_width`, and `render_tree_tests::a_spanning_row_wraps_by_its_first_columns_policy` |
| R1-F5.9 | Sweep: the location's OSC 8 link — expected: a terminal receives the link and its parameters | **fixed** | tmux's capture holds no hyperlinks, so `level2_location_reaches_the_terminal_as_a_hyperlink` uses a background WezTerm pane: `line`, `column`, and `heading` parameters and a bare `file://` destination |
| R1-F5.10 | Sweep: `--show-all` listing, `-v` destination, Markdown and HTML outputs — expected: no separate terminal path | **clean** | The listing goes through the same `rows_table` and `location_node`; `-v` is the fold's non-clickable link text, a byte contract in `lib/tests/l1/reference_report.rs::verbose_plain_prints_the_bare_file_uri` and `cli/tests/l1/reference_report.rs::verbose_prints_each_destination_beside_its_text`; Markdown and HTML are not terminal output, and the review requires no matrix per format |

### Progress

- starting the work on 'R1-F1' at 20:16:30
        - defect class: reference discovery drops records before validation can classify their targets
        - sites swept: every classifier and helper in `reference/html.rs`, `reference/css.rs`, `reference/meta.rs`; `extract_all_references` in `reference/graph.rs`; `classify_target`; the `Markdown` accessors in `reference/mod.rs`; `compose/link_resolve.rs`, `compose/link_normalization.rs`, `compose/util.rs`; `reference/validate.rs`, `reference/report.rs`, `reference/file_tree/`; `cli/src/commands/compose.rs`, `compose/policy.rs`; `dmls/src/graph/substrate.rs`; `cli/schemas/output/validate-refs.yaml`
        - the line split was one of several drops: `is_tag` also required a literal space, `extract_attribute` was a substring search, and `extract_inner_text` stopped at the first `</`
        - replaced the line split with `scan_elements` (each element of a node, past comments and raw-text bodies) and `LineMap` (node text back to source, past container prefixes and `\r`); `read_start_tag` reads attributes the way a tokenizer does
        - the record's position is its tag's `<`, or for CSS its `@import` or `url(`, matching the one-line shape and the spec's first-character rule, not the attribute value
        - discoveries are ledger rows R1-F1.15 through R1-F1.30
        - decisions: no `--allow` recovery for media (R1-F1.14); an empty target is no reference (R1-F1.21); `<a>` spans its open tag (R1-F1.25); `srcset` and similar stay outside the spec's construct table (R1-F1.30)
        - no comment or doc recorded the old multiline deferral outside the removed comment in `walk_html_nodes`
        - seen, not changed: `AuthoredLocator` counts only `\n`, so a CR-only file reports a found record at the wrong line (`x\r\r<img...` gives 1:4); this is not specific to HTML
- work completed for 'R1-F1' at 20:25:42
- starting the work on 'R1-F2' at 20:28:22
        - defect class: converting a typed warning or error to the public diagnostic loses available provenance
        - sites swept: `warning_diagnostic`, `error_diagnostics`, `lifecycle_failure_diagnostics`, `failure_diagnostics`, `located`, `leaf_diagnostic`, `with_iteration_context`, `shell_error_diagnostics`, the reference branch, `from_invalid_trigger`, `from_schema_advisory`, `undeclared_property`, every `at_line` caller, `ReportSink`, `CommandRun`, `md render`, `compose/lifecycle.rs`, every `source: None` in lib and cli
        - central design: `report_diagnostics` projects what a value holds (`context_file`, `located`, `shell_source`, `condition_source`), and `with_document_fallback` is the one place a caller's document fills an empty `source`; `ReportSink` applies it, after `composes()` names the opened document
        - discoveries are ledger rows R1-F2.24 through R1-F2.35
        - `ConditionSite` resolves the authored opener only for a finding, since a repeated gate evaluates once per copy and resolving reads the filesystem
        - decision: a `::shell`, `::shell-block`, `::file` parse, or `::block` parse failure is file-only rather than threaded through `BodyOrigin`; those parsers run where no map is held, and the probe `::shell` after a multiline value reports "line 8" for a directive on line 6, so publishing that line is wrong. Alternative was a new `authored` field on each variant and every constructor; the spec accepts file-only when a range cannot be proven
        - decision: a frontmatter command reports its key's line with a null column; computing the value's column needs the frontmatter leaf locator and adds nothing a reader cannot see on that line
        - decision: the fallback names the document on the command line, as the spec says ("the composed document"), so a typed error with no file raised in a transcluded child names the root. Alternative was a `MarkdownError` wrapper per document; every caller matching on a variant (tolerated-failure decisions, the CLI's shell wording) would need to see through it
        - decision: a malformed `--set` argument also reports the document, since the sink cannot tell it from any other untyped failure once the document is open
        - decision: warnings other than the condition family stay file-only (R1-F2.31); their `line_number` is a current-body count, and the spec accepts file-only
        - decision: `dm.schema.invalid_trigger` with no position changes from `null` to the trigger file (R1-F2.32); trigger warnings are named in "Other problem families", and two tests had pinned the old null
        - decision: `<stdin>` no longer appears as a file; the output schemas already documented null for stdin
        - `lib` warnings the root appends directly (`cache_compose_warnings`, ICMP, `current`) are not stamped; they are run-wide, and the sink's fallback gives them the document
        - seen, not changed: a transclusion condition parse failure keeps code `dm.transclusion.failure` while a page-block one is `dm.expression.parse_failure`; the code is not this finding's subject
        - docs corrected: `docs/errors/composition-problems.md`, `docs/cli/data-contracts.md`, the shared diagnostic text in all 17 `cli/schemas/output/*.yaml` (the shape is unchanged; line and column stay nullable), `.claude/skills/darkmatter/SKILL.md`, the `report_diagnostics` module docs
- work completed for 'R1-F2' at 21:05:33
- starting the work on 'R1-F3' at 21:08:28
        - defect class: an unbounded-width state is converted to a numeric fill width at an output adapter
        - sites swept: every `term.width()`, `fixed_width`, and `detected_columns` read in `biscuit-terminal`, `biscuit-visualized`, `darkmatter`, and the crates depending on them; every `from_output_options` caller; `1 << 20`, `u32::MAX`, `usize::MAX`, `as_columns().unwrap_or`
        - two instances of the sentinel, not one: the bridge and Darkmatter's stderr terminal (R1-F3.10)
        - the larger discovery is `from_terminal` (R1-F3.11): keeping the enum on the bridge was not enough, since every tree-routed component rebuilt `Columns(width())` from it
        - decision: `Terminal` gains a public `unbounded_width: Option<UnboundedWidth>` with a two-variant enum, and `output_width()`/`columns()`/`set_output_width()`. Alternatives: `Option<Width>` (admits a `Columns` that contradicts `fixed_width`); changing `width()` to return `Option` (touches every crate's direct-terminal callers, which the review asked to preserve). `fixed_width` outranks it, so a container's narrowing keeps working and the two fields cannot disagree
        - decision: `width()` answers 80 on a terminal with no width: the existing "size cannot be determined" answer, and what the fold's snapshot already held. Everything in this repository that fills or wraps reads `columns()`; a third-party reader gets a modest bounded number, never a large fill
        - decision: under no width a percentage image is its natural size rather than a percentage of it, and a percent margin is zero, matching the tree fold's "a percentage of nothing is nothing"
        - decision: a bridge terminal in a plain profile keeps a filled rule at `Columns(n)`; the review's control asks for it, though the spec's table says `---` in `plain` at every width. The tree fold already prints `---` there and no command renders a rule directly on this bridge. Alternative was making the component print `---` for any plain profile, which would also change a direct non-TTY terminal the review asked to preserve
        - decision: `TwoColumn` with an image, `GraphExpression` images, and `GitGraph` viewports keep a bounded numeric budget (R1-F3.22); giving them natural sizing needs a measurement before layout, which they do not have
        - docs corrected: `biscuit-terminal/docs/output.md` (the recorded limit replaced by a table and example), `darkmatter/docs/cli/output-and-width.md`, `components/table/README.md`, the `biscuit-terminal` skill (`terminal-struct.md`, `render-tree.md`); stale comments in `render_hook.rs` and `render_tree/options.rs`
- work completed for 'R1-F3' at 21:28:43
- starting the work on 'R1-F4' at 21:30:29
        - defect class: a container location is substituted for an individual frontmatter reference's authored value location
        - sites swept: `AuthoredLocator::frontmatter_location`, `value_location` and both property loops in `reference/graph.rs`; `reference/mod.rs` transclusion listing; `locate_frontmatter_leaves`; `FrontmatterExpressionLocation::project`; `PositionMap` and `positions_for`; every `frontmatter_key_line` user; `report.rs`, `validate.rs`, `report_diagnostics`; DMLS `dsl.rs` and `problem_range`
        - cause: the leaf locator is a writer's API, so it must refuse a plain flow item, whose bytes it cannot prove safe to replace, and the fallback turned that refusal into the key's position. The null came earlier: `locate_all_nodes` rejects a `]` at column 0
        - larger discovery: the tolerant locator read a flow collection only on one line with nothing after it, and its failure is whole-frontmatter, so one such list coarsened every schema problem and interpolation error in the document (R1-F4.12, R1-F4.13)
        - decision: extend the tolerant grammar rather than write a second flow scanner, reusing `flow_collection_end` and `split_flow_entries`. The strict schema-document grammar is unchanged
        - decision: `hash::locate_frontmatter_value_range` is a separate read-only answer rather than loosening the writer. Alternative was accepting plain flow items in `locate_frontmatter_leaves`; Claudine replaces those spans in place, and "where was this typed" does not need that guarantee
        - decision: the leaf locator runs first because it slices one property, so a construct elsewhere the whole-frontmatter read cannot model (`other: {a, b}`) still leaves a scalar its place. A flow item beside such a sibling is null (R1-F4.19). Alternative was a flow-aware per-property slice in `locate_all_nodes`, which the writer shares; not worth it for this shape
        - decision: proof is a decode of the located node equal to the reference's text, cut at the item's span so a plain item does not read on through the list; an alias is accepted at its token, as `FrontmatterExpressionLocation::project` already does
        - decision: a value with a tag or anchor reports the token opening it, since the scalar decoder skips them but they are where the author typed the value. Alternative was the content after them; the block form's `span.start` already starts at the property
        - decision: one writer widening accepted: a quoted item in a one-line flow followed by a comment, and a value after `key: # note`, are now located where they were refused. The span is exact and decode-checked, so replacing it is as safe as without the comment (`a_quoted_item_is_located_beside_a_comment`). A plain item stays refused
        - seen, not changed: `md validate refs` on `prologue: "{{ title }}.md"` fails with `dm.reference.failure` and a null `source`, since the `FileReference` grammar rejects it before any location exists; other commands not applying the document fallback is R1-F2.35's decision
        - seen, not changed: DMLS links only a scalar `prologue:`, so no list entry of either form is a document link; that is a missing feature, not a wrong location
        - docs corrected: `docs/errors/composition-problems.md`, `docs/topics/schemas/definition.md`, the `darkmatter` skill (`frontmatter.md`, `schema.md`); `frontmatter_location`, `locate_frontmatter_value`, `LocatorGrammar`, and `FrontmatterExpressionLocation::project` comments
- work completed for 'R1-F4' at 21:45:54
- starting the work on 'R1-F5' at 21:46:50
        - defect class: tests verify generated terminal bytes without crossing the terminal-rendering boundary required by the visual contract
        - sites swept: `md validate refs`, `md graph --validate`, the `md compose` refusal, severity colors, glob hint rows, the tally, the OSC 8 location, `--show-all`, `-v`, and `level2_errors`. New ledger rows R1-F5.6 to R1-F5.10
        - backends: tmux 3.8 (`/opt/homebrew/bin/tmux`) for six tests, WezTerm (reachable through `WEZTERM_UNIX_SOCKET`) for one, all in background panes; nothing was activated and no host keyboard event was sent
        - discovery, fixed: at 72 columns a glob hint split `::file-links`. `emit_borderless_table` wrapped a spanning row with `WordWrap::default()`, whatever its column declared. It now uses the first column's policy, and `rows_table` gives that column `BespokeProse(None, [' '], None)`. Only the report and the fold's own tests use `.borderless()` in the areas searched
        - discovery, not a defect: a reference renders dim, because inline code with no prose theme in the fold's context falls back to a dim run. The spec asks only for inline code, so the test accepts a dim or a background run and rejects red
        - decision: each tmux test owns its session (`TmuxHarness::new`), not `shared_or_spawn`, so a resize cannot reach a shared broker pane. Alternative was one shared pane with resize and restore, which breaks under `BISCUIT_L2_THREADS=1`
        - decision: a local cell replay of the capture rather than borrowing `tree-hugger`'s `terminal_report.rs`, which is that package's test support; asserting a contains on raw bytes would not show which cell a carried attribute applies to
        - decision: color checks accept a truecolor, 256-color, or basic form of red and yellow, since the depth belongs to the host and the claim is the hue. Alternative was the exact `38;2;251;44;54`, which would pin the theme's value
        - decision: WezTerm link parameters are compared as a set, because WezTerm reported them as `line=3:heading=root:column=1`, not in the emitted order
        - decision: the review named one narrow case; compose gets its own because stderr's width comes from a separate path. `--width` was left out, since the pane's real columns are the thing a capture adds
        - docs: `.claude/skills/darkmatter/testing.md` gains the suite; `docs/cli/validate.md` already states the behavior and is unchanged
        - seen, not changed: in `render.rs`, the doc comment beginning "Emits the table body" sits above `is_spanning_row`, not the function it describes
- work completed for 'R1-F5' at 22:04:36
- reconciliation by the orchestrator, finished at 22:43:47
        - reran the review's reproductions on a fresh `md` binary; each reported the expected result (multiline image 1:1 and exit 1, CSS 2:1 and 3:19, one-line video 1:1, flow list exact columns, multiline flow 3:3 and 4:3, schema, condition, and shell failures with a source)
        - reversed R1-F2.35 from clean to fixed: `md validate refs`, `md graph`, and `md schema triggers` now name their document through `CommandRun::set_document`
        - changed R1-F1.30 from unresolved to clean, since the spec's exhaustive construct table bounds discovery, and raised it for author review
        - added R1-F1.31 and repaired it: a rewrite could hit `data-src` instead of `src`
        - updated `compose_expression_failure_contract::an_unlocatable_value_that_is_no_alias_still_claims_no_span`, which pinned the multiline flow limit R1-F4 removed; added `a_multiline_flow_item_reports_its_exact_span`
        - native Windows found 36 failures in new tests from rootless fixture paths in `diagnostic_source_provenance`; repaired with a drive-qualified fixture root
        - moved the `emit_table` doc comment back to its function, because it sat above `is_spanning_row`

### Completion Report

**Status: complete.** Elapsed: about 2 h 35 min (20:10:43 to 22:43:47).

Parent findings: total 5 = fixed 5 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0. Ledger rows, parents included: 129 (95 fixed, 34 clean); no row is open.

- verified repairs
        - R1-F1: multiline tags, multiline style bodies, and video, audio, source, and iframe targets reach validation, compose, and the graph with exact positions
        - R1-F2: every projection branch keeps known provenance, with file-only records where no range is proven and a document fallback where only the document is known
        - R1-F3: `UNBOUNDED_COMPONENT_COLUMNS` is gone; `Terminal` retains the width state and filling components take natural size; the plain rule is `---` under `Source` and `Unlimited`
        - R1-F4: each flow-sequence value reports its own position, in reports, diagnostics, and transclusion trails
        - R1-F5: `level2_reference_report` runs on real tmux and WezTerm backends; it found and led to a repair of a hint wrapping defect in borderless tables
- accounted failure, unrelated to this cycle
        - `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` fails on macOS and native Windows on `claudine/docs/research/sandboxing/providers/sandboxing.yaml`, an in-progress research schema using planned `2026-11-05-dynamic-schema-types` syntax
        - this cycle did not cause or repair it; the review retains reporting it transparently, and `os_failing` is not set because the failure is not specific to Windows
        - the Darkmatter L1 count below excludes this one test, so it is not passing evidence
- repository state
        - another actor committed the R1-F1 and R1-F2 work during this cycle (`e9e499b82`, `7679afb0f`); this session made no commit, reset, or staging change
        - the rest of this cycle's work is uncommitted in the working tree, including three untracked test files
- evidence limits
        - L2 and browser runs are macOS only; Windows evidence is L1 only
        - the cross-check reported that its tested tree is not `HEAD`'s tree, so those Windows runs publish no CI observation

#### Final verification

| Command | Result | Verifies |
| --- | --- | --- |
| `just test -- --skip every_kinded_schema_artifact_passes_its_definition_through_the_registry` (darkmatter) | 13901 passed, 11 skipped | R1-F1, R1-F2, R1-F3, R1-F4 |
| `just test every_kinded_schema_artifact_passes` (darkmatter) | **1 failed**, the accounted failure | none; see above |
| `just lint` (darkmatter) | passed | all |
| `just test`, `just lint` (biscuit-terminal) | 4022 passed, 55 skipped; lint passed | R1-F3, R1-F5 |
| `just test`, `just lint` (biscuit-visualized) | 1165 passed, 1 skipped; lint passed | R1-F3 |
| `just test`, `just lint` (simplified-schema) | 1213 and 1401 passed; lint passed | R1-F4 |
| `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` (darkmatter) | 3, 18, and 76 passed | R1-F5 |
| `just test-l2` (biscuit-terminal) | 8 and 252 passed, with absent-backend skips allowed; the required-backend run stopped because no usable Kitty backend was available | R1-F3 |
| `just test-l2` (biscuit-visualized) | not applicable: the recipe is a stub, so this is no evidence | none |
| `BISCUIT_BROWSER_REQUIRED=1 just test-browser` (darkmatter) | 44 passed | R1-F3 |
| `just check-tier-coverage` (darkmatter, biscuit-terminal) | 0 stranded in each | test placement |
| `cargo check -p claudine -p sniff -p darkmatter-lsp` | passed | public `Terminal` change |
| `just cross-check biscuit-terminal --os windows` | 3598 passed | R1-F3 |
| `just cross-check darkmatter-cli --os windows` | 1889 passed | R1-F1, R1-F2, R1-F4 |
| `just cross-check darkmatter --os windows` | 10195 passed, 1 failed, the accounted failure | R1-F1, R1-F2, R1-F4 |

The biscuit-terminal L2 run, the browser run, and the Windows `biscuit-terminal` and `darkmatter-cli` runs came before the final edit, which changed only a test fixture and moved a doc comment.

#### Changed files

        - `.claude` (6 files)
                - `.claude/skills/biscuit-terminal/render-tree.md`
                - `.claude/skills/biscuit-terminal/terminal-struct.md`
                - `.claude/skills/darkmatter/SKILL.md`
                - `.claude/skills/darkmatter/frontmatter.md`
                - `.claude/skills/darkmatter/schema.md`
                - `.claude/skills/darkmatter/testing.md`
        - `biscuit-terminal` (23 files)
                - `biscuit-terminal/docs/output.md`
                - `biscuit-terminal/lib/src/components/block_quote.rs`
                - `biscuit-terminal/lib/src/components/filesystem/mod.rs`
                - `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs`
                - `biscuit-terminal/lib/src/components/metrics_tree.rs`
                - `biscuit-terminal/lib/src/components/prose/render.rs`
                - `biscuit-terminal/lib/src/components/status.rs`
                - `biscuit-terminal/lib/src/components/table/README.md`
                - `biscuit-terminal/lib/src/components/table/table.rs`
                - `biscuit-terminal/lib/src/components/terminal_image/half_block.rs`
                - `biscuit-terminal/lib/src/components/terminal_image/iterm.rs`
                - `biscuit-terminal/lib/src/components/terminal_image/kitty.rs`
                - `biscuit-terminal/lib/src/components/terminal_image/mod.rs`
                - `biscuit-terminal/lib/src/components/terminal_image/protocol.rs`
                - `biscuit-terminal/lib/src/components/terminal_image/sixel.rs`
                - `biscuit-terminal/lib/src/components/terminal_image/tests.rs`
                - `biscuit-terminal/lib/src/output/profile.rs`
                - `biscuit-terminal/lib/src/render_tree/options.rs`
                - `biscuit-terminal/lib/src/render_tree/render.rs`
                - `biscuit-terminal/lib/src/terminal.rs`
                - `biscuit-terminal/lib/src/utils/layout.rs`
                - `biscuit-terminal/lib/tests/l1/main.rs`
                - `biscuit-terminal/lib/tests/l1/output_component_bridge.rs`
        - `biscuit-visualized` (6 files)
                - `biscuit-visualized/src/src/components/git_graph.rs`
                - `biscuit-visualized/src/src/components/graph_expression.rs`
                - `biscuit-visualized/src/src/components/mermaid.rs`
                - `biscuit-visualized/src/src/components/render_hook.rs`
                - `biscuit-visualized/src/src/tests/component_fallback_tests.rs`
                - `biscuit-visualized/src/tests/l1/display_size.rs`
        - `darkmatter` (86 files)
                - `darkmatter/cli/schemas/output/clean.yaml`
                - `darkmatter/cli/schemas/output/code-block.yaml`
                - `darkmatter/cli/schemas/output/compose.yaml`
                - `darkmatter/cli/schemas/output/delta.yaml`
                - `darkmatter/cli/schemas/output/edit.yaml`
                - `darkmatter/cli/schemas/output/get.yaml`
                - `darkmatter/cli/schemas/output/graph.yaml`
                - `darkmatter/cli/schemas/output/hash.yaml`
                - `darkmatter/cli/schemas/output/render.yaml`
                - `darkmatter/cli/schemas/output/rm.yaml`
                - `darkmatter/cli/schemas/output/schema-about.yaml`
                - `darkmatter/cli/schemas/output/schema-detect.yaml`
                - `darkmatter/cli/schemas/output/schema-triggers.yaml`
                - `darkmatter/cli/schemas/output/schema-validate.yaml`
                - `darkmatter/cli/schemas/output/set.yaml`
                - `darkmatter/cli/schemas/output/toc.yaml`
                - `darkmatter/cli/schemas/output/validate-refs.yaml`
                - `darkmatter/cli/src/commands/compose.rs`
                - `darkmatter/cli/src/commands/compose/lifecycle.rs`
                - `darkmatter/cli/src/commands/graph.rs`
                - `darkmatter/cli/src/commands/render.rs`
                - `darkmatter/cli/src/commands/schema/about.rs`
                - `darkmatter/cli/src/commands/schema/triggers.rs`
                - `darkmatter/cli/src/commands/validate.rs`
                - `darkmatter/cli/src/output/command.rs`
                - `darkmatter/cli/src/output/mod.rs`
                - `darkmatter/cli/src/output/report.rs`
                - `darkmatter/cli/tests/l1/compose_report_diagnostics.rs`
                - `darkmatter/cli/tests/l1/invalid_trigger_warnings.rs`
                - `darkmatter/cli/tests/l1/main.rs`
                - `darkmatter/cli/tests/l1/output_width_overrides.rs`
                - `darkmatter/cli/tests/l1/reference_report.rs`
                - `darkmatter/cli/tests/level2/level2_reference_report.rs`
                - `darkmatter/cli/tests/level2/main.rs`
                - `darkmatter/dmls/src/graph/substrate.rs`
                - `darkmatter/docs/cli/data-contracts.md`
                - `darkmatter/docs/cli/graph.md`
                - `darkmatter/docs/cli/output-and-width.md`
                - `darkmatter/docs/cli/policy.md`
                - `darkmatter/docs/cli/validate.md`
                - `darkmatter/docs/errors/composition-problems.md`
                - `darkmatter/docs/structs/Markdown.md`
                - `darkmatter/docs/topics/schemas/definition.md`
                - `darkmatter/lib/src/layout/page.rs`
                - `darkmatter/lib/src/layout/page/tests.rs`
                - `darkmatter/lib/src/markdown/compose/call_precheck.rs`
                - `darkmatter/lib/src/markdown/compose/collection_blocks/engine.rs`
                - `darkmatter/lib/src/markdown/compose/conditions.rs`
                - `darkmatter/lib/src/markdown/compose/context/options.rs`
                - `darkmatter/lib/src/markdown/compose/context/report.rs`
                - `darkmatter/lib/src/markdown/compose/expression/diagnostics.rs`
                - `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
                - `darkmatter/lib/src/markdown/compose/inline/collection_blocks.rs`
                - `darkmatter/lib/src/markdown/compose/inline/page_blocks.rs`
                - `darkmatter/lib/src/markdown/compose/page_blocks/engine.rs`
                - `darkmatter/lib/src/markdown/compose/pipeline/mod.rs`
                - `darkmatter/lib/src/markdown/compose/report_diagnostics.rs`
                - `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
                - `darkmatter/lib/src/markdown/compose/site_scope.rs`
                - `darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs`
                - `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs`
                - `darkmatter/lib/src/markdown/compose/transclusion/engine.rs`
                - `darkmatter/lib/src/markdown/compose/transclusion/types.rs`
                - `darkmatter/lib/src/markdown/compose/util.rs`
                - `darkmatter/lib/src/markdown/errors/mod.rs`
                - `darkmatter/lib/src/markdown/hash/mod.rs`
                - `darkmatter/lib/src/markdown/hash/write.rs`
                - `darkmatter/lib/src/markdown/reference/css.rs`
                - `darkmatter/lib/src/markdown/reference/file_tree/mod.rs`
                - `darkmatter/lib/src/markdown/reference/file_tree/render.rs`
                - `darkmatter/lib/src/markdown/reference/graph.rs`
                - `darkmatter/lib/src/markdown/reference/html.rs`
                - `darkmatter/lib/src/markdown/reference/meta.rs`
                - `darkmatter/lib/src/markdown/reference/report.rs`
                - `darkmatter/lib/src/markdown/reference/types.rs`
                - `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`
                - `darkmatter/lib/src/markdown/yaml_block.rs`
                - `darkmatter/lib/tests/l1/compose_expression_failure_contract.rs`
                - `darkmatter/lib/tests/l1/diagnostic_source_provenance.rs`
                - `darkmatter/lib/tests/l1/error_snapshots/condition.rs`
                - `darkmatter/lib/tests/l1/error_snapshots/page_block.rs`
                - `darkmatter/lib/tests/l1/error_snapshots/transclusion.rs`
                - `darkmatter/lib/tests/l1/main.rs`
                - `darkmatter/lib/tests/l1/reference_html_discovery.rs`
                - `darkmatter/lib/tests/l1/reference_provenance.rs`
                - `darkmatter/lib/tests/l1/reference_report.rs`
        - `simplified-schema` (2 files)
                - `simplified-schema/lib/src/positions.rs`
                - `simplified-schema/lib/src/simplified/source.rs`


## Implementation of Review Findings #2

> **started at:** 2026-11-05T22:51:09-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/fixes/2026-11-04-composition-errors/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- the review has one parent finding (R1-F1, medium, 27 child rows), no `human_in_the_loop` item, and no blocked finding; `implemented` was already `false` and is left `false` until the cycle closes
- the review's other four findings from review 1 are verified fixed by the reviewer and carry no work here

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Raw-text contents split across Markdown nodes still become references | **fixed** | every child below is fixed or clean; full L1 run has no failure in this code (see Completion Report) |
| R1-F1.17.1 | Raw `script` body, block / after prose — exclude the image string in both; retain the script record | **fixed** | `reference_html_discovery::script_body_as_a_block`, `script_body_after_prose` (InlineScript retained) |
| R1-F1.17.2 | Raw `style` body, block / after prose — exclude comment text in both; retain CSS resource reads | **fixed** | `style_body_as_a_block`, `style_body_after_prose`, `style_comment_as_a_block`, `style_comment_after_prose` (InlineCss retained) |
| R1-F1.17.3 | Raw `textarea` body, block / after prose — no image reference | **fixed** | `textarea_body_as_a_block`, `textarea_body_after_prose` |
| R1-F1.17.4 | Raw `title` body, block / after prose — no image reference | **fixed** | `title_body_as_a_block`, `title_body_after_prose` |
| R1-F1.17.5 | Raw `xmp` body, block / after prose — no image reference | **fixed** | `xmp_body_at_line_start`, `xmp_body_after_prose` |
| R1-F1.17.6 | Raw `iframe` body, no outer `src`, block / after prose — no reference to the frame's text | **fixed** | `iframe_body_as_a_block`, `iframe_body_after_prose`, `frame_target_after_prose_with_a_body` |
| R1-F1.17.7 | Raw `noembed` body, block / after prose — no image reference | **fixed** | `noembed_body_at_line_start`, `noembed_body_after_prose` |
| R1-F1.17.8 | Raw `noframes` body, block / after prose — no image reference | **fixed** | `noframes_body_as_a_block`, `noframes_body_after_prose` |
| R1-F1.17.9 | HTML anchor in an inline textarea — no reference | **fixed** | `textarea_holding_an_anchor` |
| R1-F1.17.10 | HTML image in an inline textarea — no reference | **fixed** | `textarea_holding_an_image` |
| R1-F1.17.11 | HTML script in an inline textarea — no reference | **fixed** | `textarea_holding_a_script` |
| R1-F1.17.12 | HTML link in an inline textarea — no reference | **fixed** | `textarea_holding_a_stylesheet` |
| R1-F1.17.13 | HTML video in an inline textarea — no reference | **fixed** | `textarea_holding_a_video` |
| R1-F1.17.14 | HTML audio in an inline textarea — no reference | **fixed** | `textarea_holding_an_audio` |
| R1-F1.17.15 | HTML source in an inline textarea — no reference | **fixed** | `textarea_holding_a_source` |
| R1-F1.17.16 | HTML iframe in an inline textarea — no reference | **fixed** | `textarea_holding_an_iframe` |
| R1-F1.17.17 | HTML meta in an inline textarea — no metadata record | **fixed** | `textarea_holding_a_meta`, `public_accessors_do_not_read_raw_text_as_references` |
| R1-F1.17.18 | HTML style and both CSS readers in an inline textarea — no style, import, or font records | **fixed** | `textarea_holding_a_style_with_css_targets`: no style, import, or font record |
| R1-F1.17.19 | Markdown-native link in an inline textarea — no reference | **fixed** | `textarea_holding_a_markdown_link` |
| R1-F1.17.20 | Markdown-native image in an inline textarea — no reference | **fixed** | `textarea_holding_a_markdown_image` |
| R1-F1.17.21 | Validation decision on the inline script template — inline-script record only, no missing-link error | **fixed** | `a_script_opened_after_prose_is_one_record_holding_its_whole_body` (1 scanned, 0 issues); CLI `reference_report::validate_refs_accepts_a_script_template_after_prose` (exit 0) |
| R1-F1.17.22 | Graph classification and validated exit — inert contents not classified as resources | **fixed** | CLI `reference_report::graph_validate_accepts_a_script_template_after_prose`, `graph_validate_accepts_a_style_comment_after_prose` (exit 0) |
| R1-F1.17.23 | Compose reference gate — no refusal for strings/comments in raw bodies | **fixed** | CLI `reference_report::compose_accepts_a_script_template_after_prose`, `compose_accepts_a_style_comment_after_prose` (exit 0) |
| R1-F1.17.24 | Actual inline CSS resource — keep the import and its location, omit only the comment's image | **fixed** | `css_import_beside_a_comment_after_prose` (`./gone.css` at 1:43, no image), `a_css_comments_tag_is_inert_while_its_import_stays_in_every_accessor` |
| R1-F1.17.25 | HTML block / inline code / fence controls — remain clean | **clean** | `script_at_line_start_stays_clean`, `script_in_inline_code_stays_clean`, `script_in_a_fence_stays_clean`; the misnamed row became `tag_in_a_line_start_script_body` plus `tag_in_a_script_body_after_prose` |
| R1-F1.17.26 | Active HTML after prose — both real targets found at 1:6 and 1:26 | **clean** | `active_tags_after_prose_each_keep_their_own_position` (1:6, 1:26); CLI `a_missing_link_after_a_script_template_still_fails` (exit 1, only `./gone.md`) |
| R1-F1.17.27 | Public accessors, link rewriting, DMLS projection — correction at shared extraction, active spans and rewriting preserved | **fixed** | correction is at shared extraction; `composing_rewrites_active_targets_and_leaves_raw_text_bytes_alone`, `public_accessors_*`, DMLS `graph::substrate::tests::raw_text_after_prose_is_not_indexed_while_an_active_link_is` |
| R1-F1.17.28 | Discovered: `set_meta_tag` scanned raw source and overwrote a `<meta>` in a raw body, fence, or comment — expected: leave it alone and append | **fixed** | `setting_a_meta_tag_leaves_one_in_raw_text_or_code_alone` |
| R1-F1.17.29 | Discovered: `inline_html_links` and `inline_html_image_references` returned tags from raw bodies, even within one node — expected: none | **fixed** | `public_accessors_do_not_read_raw_text_as_references`, `public_accessors_keep_what_is_outside_raw_text` |
| R1-F1.17.30 | Discovered: `Markdown::links` and `image_references` returned Markdown links and images from spanning bodies — expected: none | **fixed** | same two tests |
| R1-F1.17.31 | Discovered in reconciliation: a closer in a code span cut `css_content` and `script_content` short, losing a real `@import` — expected: whole body read | **fixed** | failed first, then `a_closer_in_a_code_span_does_not_cut_a_raw_body_short` passes (`./gone.css` at 1:25) |
| R1-F1.17.32 | Discovered in reconciliation: masking a body with spaces indented its closing tag into a code block, hiding a real tag after it — expected: tag found | **fixed** | failed first, then `a_tag_after_a_closer_on_a_long_body_line_stays_active_in_every_accessor` passes |
| R1-F1.17.33 | Sibling: CSS `@import` and `@font-face` readers | **clean** | they read style records only, so a style in a raw body has none; real rules keep locations: `css_import_on_a_block_quote_line_after_prose` (2:3) |
| R1-F1.17.34 | Sibling: `::file` inside a raw body — expected: still a composition directive | **clean** | composition really runs it, so the graph must see it: `a_file_directive_in_a_raw_body_is_still_a_composition_directive` |
| R1-F1.17.35 | Sibling: shapes — block quote, CRLF, heading and table openers, case, escaped opener, opener or closer in code, consecutive elements, multi-paragraph bodies, resume after closer | **fixed** | `body_on_block_quote_lines`, `crlf_body`, `opener_in_a_heading`, `opener_in_a_table_cell`, `mixed_case_tags`, `escaped_opener_leaves_a_later_tag_active`, `closer_in_a_fence_does_not_close`, `tag_after_two_raw_elements`, `body_across_paragraphs`, `tag_after_the_closer_in_its_block` |
| R1-F1.17.36 | Decision: raw-text opener with no closer in any later HTML node — nothing past its node is inert. Alternative: run to end of document as a browser does, rejected because a prose mention of `<title>` would silently hide every later reference | **fixed** | `unclosed_opener_after_prose_leaves_a_later_link_active`, `unclosed_opener_after_prose_leaves_a_later_tag_active`, `unclosed_block_opener_leaves_the_next_block_active`; in `human_review_items` |
| R1-F1.17.37 | Decision: a heading in a multi-paragraph raw body remains in `Markdown::toc()`, so `#slug` validation can accept it. Alternative: mask bodies at each heading reader (`toc`, section lookup, `::toc-linking`), rejected as a wider change to rendering structure the review did not ask for | **clean** | not a reference record, so it adds no false record or error; the only effect is a possible false accept of an opt-in fragment check, a limit a browser reader would see; in `human_review_items` |

### Progress

- defect class: reference discovery forgets an enclosing inert-text range when extraction crosses Markdown node or extractor boundaries
- starting the work on 'R1-F1 Raw-text contents split across Markdown nodes' at 22:53:00
        - reproduction confirmed before the fix: new `reference_html_discovery` rows failed, including `composing_rewrites_active_targets_and_leaves_raw_text_bytes_alone`, where compose rewrote targets inside a `<textarea>` body
        - `reference/html.rs` now walks a document's HTML nodes in order (`visit_html_elements`); a raw-text element's closer is looked for in later HTML node values only, and `with_inline_raw_body` is gone
        - `inert_ranges` drops Markdown-native links and images that start in a spanning body; a link that starts before the opener stays
        - no closer in any later node: nothing past the opener's node is inert, pinned by the `unclosed_*` rows
        - siblings checked
                - fixed: `Markdown::links`, `Markdown::image_references` (filter on `inert_ranges`)
                - fixed: `inline_html.rs` fragment collector and its fallbacks (read `blank_raw_text_bodies`, which also covers bodies inside one node)
                - fixed: `meta.rs` `set_meta_tag` scanned raw source and overwrote a tag in a raw body, fence, or comment; it now replaces an extracted record's span
                - clean through shared extraction: CSS readers, `meta.rs` `parse_meta_tags`, `link_resolve`, `link_normalization`, graph, validate, compose gate, DMLS `substrate.rs`
                - clean by design: `::file` is a composition directive that runs inside a raw body, pinned by `a_file_directive_in_a_raw_body_is_still_a_composition_directive`
                - not this decision: `render_tree`, `cleanup`, `toc`, `highlighting`, `toc_linking` read Markdown structure for rendering or formatting and create no reference record
        - reconciliation by the orchestrator found and fixed R1-F1.17.31 and R1-F1.17.32, each with a test that failed first; added the missing CLI style-comment and DMLS raw-text tests
- work completed for 'R1-F1 Raw-text contents split across Markdown nodes' at 23:13:21

### Completion Report

**Status: partial.** Elapsed: about 22 min (22:51:09 to 23:13:21). Every finding is repaired; the only reason this is not complete is the required `just test` run, which still exits nonzero on one unrelated test.

Parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0. Child rows: 37 (32 fixed, 5 clean); no finding or child row is open.

- verified repair
        - R1-F1: a raw-text element split across Markdown nodes is one element at shared extraction, so its contents produce no HTML, CSS, or Markdown-native record; real script and style records, real CSS rules, active tags, spans, and rewriting are kept
        - the review's three commands (`md validate refs`, `md graph --validate`, `md compose`) exit 0 on the reproduction
- open check, no finding ID
        - `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` fails on `claudine/docs/research/sandboxing/providers/sandboxing.yaml` (invalid SimplifiedSchema for property `os`), a committed research schema using `{{ as_csv(...) }}` from the unimplemented `2026-11-05-dynamic-schema-types`
        - this cycle did not cause it and did not touch that file; iteration 1 recorded the same failure
        - not repaired here: changing the test or the author's draft would hide a true result, and implementing the syntax belongs to that other spec
        - next required action: land that spec or have the author correct or exclude the schema, then rerun `just test` in `darkmatter`
- decisions for the author: R1-F1.17.36 and R1-F1.17.37, both in the review's `human_review_items`
- changed files in this cycle
        - `darkmatter/lib/src/markdown/reference/html.rs`, `reference/local.rs`, `reference/meta.rs`, `inline_html.rs`, `markdown/mod.rs`
        - `darkmatter/lib/tests/l1/reference_html_discovery.rs`, `darkmatter/cli/tests/l1/reference_report.rs`, `darkmatter/dmls/src/graph/substrate.rs`
        - `darkmatter/docs/cli/validate.md`, `darkmatter/docs/structs/Markdown.md`, `.claude/skills/darkmatter/SKILL.md`
        - nothing committed
- final verification, after the last code change, for R1-F1 and all its rows
        - `just test --no-fail-fast` in `darkmatter`: **exit 100**, 13979 run, 13978 passed, 1 failed (the corpus test above), 10 skipped, none at or above 5 s
        - `just lint` in `darkmatter`: exit 0
        - `just check-tier-coverage darkmatter` from the repo root: zero stranded tests
        - `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2 level2_reference_report`: exit 0, 7 passed; run as a control for the reference report, since no terminal presentation changed
- not run
        - cross-OS check: the change is text scanning over byte offsets with no path or OS-specific code, and CRLF is covered by `crlf_body` and `link_after_a_crlf_body`; no Windows, Linux, or WSL2 evidence was produced for this cycle
        - browser tier: no browser rendering changed
- no performance measurement was deferred
