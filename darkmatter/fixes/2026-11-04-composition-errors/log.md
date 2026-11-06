---
implementation_1: "2026-11-05T20:10:43-08:00"
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
| R1-F2 | Diagnostic projection discards known source locations | pending | |
| R1-F2.1 | Schema preparation / unknown named type — expected: File-only source at minimum | pending | |
| R1-F2.2 | Schema validation / wrong scalar type — expected: Clean; preserve it | pending | |
| R1-F2.3 | Schema problems without a line — expected: Retain the file even without coordinates | pending | |
| R1-F2.4 | Page-block condition / parse and evaluation failures — expected: Preserve file; project authored coordinates where proven | pending | |
| R1-F2.5 | Transclusion condition / evaluation failure — expected: Clean; use this existing provenance pattern | pending | |
| R1-F2.6 | Transclusion condition / parse failure — expected: Retain known file and authored position | pending | |
| R1-F2.7 | Transclusion directive / omitted target — expected: Retain context; publish only proven authored coordinates | pending | |
| R1-F2.8 | Transclusion cycle / child includes itself — expected: Preserve the responsible child source rather than substituting the root | pending | |
| R1-F2.9 | Shell expansion / frontmatter approval refusal — expected: Retain file and known frontmatter location | pending | |
| R1-F2.10 | Other shell-expansion variants / shared projection — expected: Preserve each variant's available context; do not change execution policy | pending | |
| R1-F2.11 | Shell-block errors / source-file-bearing variants — expected: Retain known file; avoid treating prepared lines as authored | pending | |
| R1-F2.12 | Ordinary warnings / undeclared body and frontmatter properties — expected: Preserve known file/line in the record | pending | |
| R1-F2.13 | Condition context warning / `ctx.toady` after multiline interpolation — expected: Carry the authored directive location before projection | pending | |
| R1-F2.14 | Warnings already carrying a diagnostic / legacy link title — expected: Clean; retain the shared record | pending | |
| R1-F2.15 | Body and frontmatter interpolation errors / division by zero — expected: Clean | pending | |
| R1-F2.16 | Collection failure / undeclared iterable — expected: Clean | pending | |
| R1-F2.17 | Reference issue / missing Markdown target — expected: Clean | pending | |
| R1-F2.18 | Reference-error projection — expected: Preserve file-only information when carried or retain it at the failing call site | pending | |
| R1-F2.19 | Frontmatter parsing / malformed YAML — expected: Clean; retain file-only fallback when no parser position exists | pending | |
| R1-F2.20 | Interpolation/collection source references without a proven span — expected: Retain the known file; leave line/column null | pending | |
| R1-F2.21 | Parameter-binding / caller-classification and generic fallback branches — expected: Preserve source if the typed error/caller has it; null remains valid for an actually source-free foreign error | pending | |
| R1-F2.22 | Lifecycle error text with no typed source / `error: boom` — expected: Clean for a source-free error value; a typed lifecycle call finding already carries its own record | pending | |
| R1-F2.23 | Blocking-policy and lifecycle delivery paths — expected: Preserve the same corrected source records through every exit, including earlier warnings | pending | |
| R1-F3 | Unlimited component output still uses a million-column sentinel | pending | |
| R1-F3.1 | Bridge + horizontal rule / `Columns(40)` — expected: Bounded; control establishes fill behavior | pending | |
| R1-F3.2 | Same / `Source` — expected: Natural output, with no manufactured million-column width | pending | |
| R1-F3.3 | Same / `Unlimited` — expected: Natural output; plain rule should be `---` | pending | |
| R1-F3.4 | Generic terminal fold / rule, table, code corpus — expected: Clean; preserve this route | pending | |
| R1-F3.5 | Darkmatter page / plain corpus at 40, 80, unlimited — expected: Clean for the documented corpus | pending | |
| R1-F3.6 | CLI bridge callers: toc, delta, graph — expected: Clean for tested inputs; adapter still discards the enum | pending | |
| R1-F3.7 | CLI bridge callers: code-block, schema-about, schema-validate — expected: Clean for tested inputs; styled code-block also uses this adapter | pending | |
| R1-F3.8 | CLI bridge callers: clean saved-delta and schema-triggers — expected: Keep enum semantics through these overrides too; test their successful rendering paths | pending | |
| R1-F3.9 | Markdown/browser folds — expected: Clean by the typed option contract | pending | |
| R1-F4 | Frontmatter flow-list references do not report their value locations | pending | |
| R1-F4.1 | Prologue scalar — expected: Clean | pending | |
| R1-F4.2 | Epilogue scalar — expected: Clean | pending | |
| R1-F4.3 | Prologue block sequence — expected: Clean | pending | |
| R1-F4.4 | Epilogue block sequence — expected: Clean | pending | |
| R1-F4.5 | Prologue one-line flow sequence — expected: 2:12 and 2:20 | pending | |
| R1-F4.6 | Epilogue one-line flow sequence — expected: 2:12 and 2:20 | pending | |
| R1-F4.7 | Prologue multiline flow sequence — expected: 3:3 and 4:3 | pending | |
| R1-F4.8 | Epilogue multiline flow sequence — expected: 3:3 and 4:3 | pending | |
| R1-F4.9 | Prologue child traversal / multiline flow sequence of existing children with broken links — expected: Parent trail lines 3 and 4 | pending | |
| R1-F4.10 | Epilogue child traversal / same shape — expected: Parent trail lines 3 and 4 | pending | |
| R1-F4.11 | Report and diagnostic consumers — expected: Correct the shared location; preserve it in data and location links | pending | |
| R1-F5 | The new styled reference report lacks real-terminal verification | pending | |
| R1-F5.1 | Grouped report reason colors and dimmed location — expected: Level 2 capture through a real terminal backend | pending | |
| R1-F5.2 | Aligned report columns and narrow-width stacked fallback — expected: Level 2 capture retaining every reference/location at a representative narrow width | pending | |
| R1-F5.3 | Existing schema/expression/file-reference error blocks — expected: Clean for those older blocks; they do not cover the new grouped report | pending | |
| R1-F5.4 | Plain escape suppression, JSON streams, exit codes — expected: Clean; an emulator adds no evidence to these byte contracts | pending | |
| R1-F5.5 | Keyboard behavior — expected: No Level 3 test required | pending | |

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
