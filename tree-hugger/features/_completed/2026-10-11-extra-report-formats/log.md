---
feature: 2026-10-11-extra-report-formats
review: review-1.md
implementation_1: "2026-10-08T15:21:07-07:00"
implementation_2: "2026-10-08T17:30:59-07:00"
deferred_perf_measurement: false
---

# Review Implementation Log: Extra Report Formats

## Implementation of Review Findings #1

> **started at:** 2026-10-08T15:21:07-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-11-extra-report-formats/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the input robustness matrix does not apply: the review records that this feature adds output writers, not a file-format or configuration reader

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|----|-----------------------------------------------------------|-------------|----------------------------------|
| R1-F1 | Markdown and Prose discard required listing content and layout | fixed | All children verified; listing/outline/class routines now write through `render::Layout` (Terminal/Prose/Markdown); orchestrator reran the review's `Example.java` reproduction |
| R1-F1.M1 | Markdown `functions --tree`: methods nested under owner context | fixed | `markdown_functions_tree_nests_methods_under_their_owner_context` |
| R1-F1.P1 | Prose `functions --tree`: owner context and indentation preserved | fixed | `prose_matches_plain_functions_tree` |
| R1-F1.M2 | Markdown `types --tree --signatures none`: type outline with declared bases | fixed | `markdown_types_tree_keeps_declared_bases_without_signatures`; manual run shows `Box extends Base` |
| R1-F1.P2 | Prose same request: same relation/outline content as plain | fixed | `prose_matches_plain_types_tree` |
| R1-F1.M3 | Markdown `symbols --tree` with filtered owner: context heading, no added declaration | fixed | `markdown_symbols_tree_heads_a_filtered_owner_as_context_only` |
| R1-F1.P3 | Prose same requests: outline structure/context | fixed | `prose_matches_plain_symbols_tree` (`--name open`) |
| R1-F1.M4 | Markdown `classes` all/static/instance: method and field sections | fixed | `markdown_classes_keep_method_and_field_sections_for_every_selection` (field section under `--signatures none`) |
| R1-F1.P4 | Prose same requests: same sections as class listing | fixed | `prose_matches_plain_classes`, `_static`, `_instance`, `_by_name` |
| R1-F1.M5 | Markdown file/module groups, packages, empty sections: headings and package sections | fixed | `markdown_groups_and_packages_become_headings_with_empty_sections` |
| R1-F1.P5 | Prose same requests: existing layout | fixed | `prose_matches_plain_functions_by_file`, `_by_module`, `_by_module_and_file`, `symbols_by_file`, `symbols_tree_by_module` |
| R1-F1.M6 | Markdown ungrouped `--sort name` across files: package-wide sort | fixed | `markdown_ungrouped_name_sort_orders_the_whole_package` |
| R1-F1.P6 | Prose same request: package-wide sort | fixed | `prose_matches_plain_functions_by_name`, `types_by_kind`, `symbols_by_kind` |
| R1-F1.M7 | Markdown declaration projection: shared declaration text, one full header, semantic emphasis | fixed | `markdown_declarations_share_the_listing_text_in_every_signature_mode`, `markdown_comments_continue_their_item_as_literal_text` |
| R1-F1.P7 | Prose flat projection/comments: plain indentation and doc continuation layout | fixed | 19 `prose_matches_plain_*` tests (both spec comparisons, all signature modes, with/without `--comments`); a wrong-indent mutation failed 13 of them |
| R1-F1.C1 | TSV/ctags: no document layout (sibling check) | clean | `symbols.tsv`/`symbols.ctags` goldens unchanged; `export_tsv`/`export_ctags` pass |
| R1-F1.C2 | Existing plain listing/outline/classes unchanged (sibling check) | clean | plain/tree/table/stats/full-signature goldens byte-identical; `just test-l2` 7/7 |
| R1-F2 | Plain Prose deletes escaped source text | fixed | Escape-unaware scan removed by R1-F1 (style/link nodes dropped after parsing); all children pinned by rendered-literal tests; sibling `god-files` sites fixed |
| R1-F2.D1 | Full declaration header with generic type retained literally | fixed | `plain_prose_keeps_generic_headers_literal`; reintroducing the old scan fails it |
| R1-F2.D2 | Short signature and class summary text preserved | fixed | `plain_prose_keeps_generic_short_signatures_and_class_summaries_literal` |
| R1-F2.D3 | Raw doc lines with tags / unmatched `<` retained | fixed | `plain_prose_keeps_doc_tags_emphasis_and_unmatched_angles_literal` |
| R1-F2.D4 | Source/owner names with angle brackets retained | fixed | `plain_prose_keeps_generic_owner_names_literal` (`[in Other<T>]`, `class Box<T>`) |
| R1-F2.C1 | Location path `x<tag>.rs` preserved (sibling check) | clean | `plain_prose_keeps_angle_brackets_in_paths_literal` (unix) |
| R1-F2.C2 | Styled Prose, TSV, Markdown generic text preserved (sibling check) | clean | `styled_prose_tsv_and_markdown_keep_generic_text` |
| R1-F2.X1 | Discovered sibling: `god-files` escaped source with `Prose::escape_text`, so `--plain` ESC stripping deleted filename text | fixed | six `main.rs` sites use `render::literal`; `test_god_files_plain_output_keeps_escapes_and_markup_in_paths_literal` |
| R1-F3 | Source controls escape neutralization in Markdown and tags warnings | fixed | All children verified; stderr sweep fixed every source-derived warning/error site |
| R1-F3.M1 | Markdown path with LF + blank line + indent: inert literal | fixed | `markdown_newline_file_name_is_text_inside_its_item_not_a_code_block`, `markdown_source_newlines_and_markup_in_paths_stay_inert` (no CodeBlock event) |
| R1-F3.M2 | Markdown source name / declaration LF cannot create structure | fixed | Markdown escaper backstop shows any control as `\u{..}`; `controls_in_a_text_node_are_shown_as_their_literal_escapes`, `hostile_docs_and_signatures_stay_inert_in_every_format` |
| R1-F3.M3 | Markdown docs: only intentional continuation structure | clean | doc breaks come only from the renderer's own split; parsed comment test |
| R1-F3.T1 | Tags warning path LF / C1 CSI: visible escaped diagnostic | fixed | `declarations::skipped_tag` neutralizes; `ctags_warnings_for_hostile_paths_are_one_visible_line_per_skipped_tag`, `skipped_tag_warning_shows_path_controls_as_one_visible_line` |
| R1-F3.C1 | TSV cell encoder (sibling check) | clean | raw-byte checks in `export_controls.rs` pass |
| R1-F3.C2 | Prose literal boundary (sibling check) | clean | styled and `--plain` Prose pass raw-byte checks with hostile filenames |
| R1-F3.C3 | Tags stdout field controls rejected (sibling check) | clean | existing control corpus still passes, now asserting the `Control` reason |
| R1-F3.C4 | Markdown non-LF controls inert (sibling check) | fixed | was inert via `literal`; escaper backstop added and tested |
| R1-F3.X1 | Discovered siblings: per-file `error:` line (`declarations::settle`), package-catalog and `--changed` errors, `--verbose` cache lines, lint/imports/refs per-file errors | fixed | all use `render::neutralized`; a revert failed 4 of 6 new tests. Only `settle` and `skipped_tag` have dedicated regression tests |
| R1-F4 | Prelude exports mix analysis-file paths with declaration locations | fixed | All children verified by `prelude_exports_locate_cross_file_aliases_at_their_declaration_in_every_format` |
| R1-F4.T1 | TSV synthetic alias: target path + coordinates | fixed | row `src/alpha.rs 2 1 type Alias`; old code fails with `src/prelude.rs` |
| R1-F4.P1 | Prose alias link href and label both target declaration | fixed | exact `<a href="…/src/alpha.rs#L2">src/alpha.rs:2:1</a>` asserted |
| R1-F4.M1 | Markdown alias location (sibling check) | clean | exact `- struct **Alias** src/alpha.rs:2:1` asserted |
| R1-F4.C1 | Tags synthetic alias omitted (sibling check) | clean | header-only output asserted |
| R1-F4.C2 | Ordinary declarations and class members (sibling check) | clean | `Original\tsrc/alpha.rs\t2;"` asserted from `src/` |
| R1-F4.X1 | Discovered sibling: ctags re-relativized projected paths against the working directory | fixed | from `src/` tags read `src/src/alpha.rs`; now asserted correct from a subdirectory |
| R1-F5 | Class record exports do not sort the flattened file | fixed | `flatten_selected` gathers, dedupes by ID, sorts once; `export_order.rs` failed 2/3 on old code |
| R1-F5.T1 | Classes TSV `--sort name`: whole file name order | fixed | `Alpha, Zebra, aardvark, zulu` (byte-order name compare) |
| R1-F5.C1 | Classes ctags `--sort name`: same order as TSV | fixed | same test, same sequence |
| R1-F5.T2 | Classes TSV `--sort kind`: whole-file kind order | fixed | `Zebra, Alpha, aardvark, zulu` |
| R1-F5.C2 | Classes ctags `--sort kind`: same | fixed | same test |
| R1-F5.S1 | Non-class symbols TSV/ctags ordering (sibling check) | clean | `symbols_exports_order_the_same_declarations_identically` |
| R1-F5.S2 | Class exports `--sort file` source order (sibling check) | clean | `Zebra, aardvark, Alpha, zulu` |
| R1-F5.S3 | Markdown/Prose class documents keep section order | clean | go through `write_classes`, not `flatten_selected`; manual Markdown/Prose run |
| R1-F5.S4 | Member selectors / dedupe membership unchanged (sibling check) | clean | `--static-only`/`--instance-only` sequences asserted; unit test covers dedupe, no field rows, synthetic filter |
| R1-F6 | Tags encoding changes real filenames and qualified names | fixed | name/filename/`class:` written raw (Neovim decodes no escapes); reader verifies; old doubling/separator code fails L1 (4 tests) and the reader |
| R1-F6.N1 | Tags name with PHP namespace backslash: exact recorded name | fixed | L1 exact bytes, length 11; L2 reader resolves `Outer\Inner` |
| R1-F6.V1 | Tags container with qualified owner: exact recorded owner | fixed | `class:Outer\Inner\Account`, 19 bytes, L1 and reader |
| R1-F6.P1 | Tags filename with literal POSIX backslash | fixed | `back\slash.rs` written raw; reader opens the real file at line 2 (unix) |
| R1-F6.S1 | Spaces, Unicode, leading space, `%`, `$`, `#` filenames navigate (sibling check) | clean | reader fixture extended with ` lead.rs`, `pct%.rs`, `cost$.rs`, `hash#.rs` |
| R1-F6.S2 | Duplicate names navigate (sibling check) | clean | reader `duplicate` #1/#2 navigation |
| R1-F6.S3 | Outside-root filename navigates (sibling check) | clean | reader `../outside.rs:2` |
| R1-F6.S4 | Unicode / unqualified owner (sibling check) | clean | reader `Ω` / `Same` cases |
| R1-F6.S5 | Kind mapping completeness (sibling check) | clean | exhaustive match; golden unchanged |
| R1-F6.S6 | Controls rejected from tags fields (sibling check) | clean | control matrix asserts `Control` reason |
| R1-F6.X1 | Discovered siblings: filenames Neovim expands (`~` prefix, backtick, `[`, `$` + identifier) and `!_` names read as pseudo-tags | fixed | skipped with warning + exit 1; `paths_neovim_would_expand_are_skipped_and_the_rest_are_written`, `pseudo_tag_names_are_skipped`, L2 skipped-sources run |
| R1-F7 | The tags reader test passes failed assertions and never navigates | fixed | All children verified; deliberate wrong-line and missing-file breaks fail the test |
| R1-F7.A1 | Reader assertion failure fails the test | fixed | `pcall(dofile)` + `cquit 1`; built-in self-check asserts a failing script exits non-zero |
| R1-F7.A2 | Lookup plus actual file/line navigation | fixed | `:{n}tag!` then canonical file + cursor line asserted per case |
| R1-F7.A3 | Tags saved at scan root | fixed | tags written to `repo/root`; nvim `cd`s there |
| R1-F7.C1 | L1 tags string assertions (sibling check) | clean | `export_ctags` byte tests unchanged and passing |
| R1-F7.C2 | Reader target/tier registration (sibling check) | clean | `required-features = ["terminal-tests"]`, CI features match; `just check-tier-coverage tree-hugger-cli`: 0 stranded |
| R1-F7.X1 | Sibling sweep of other tests that run scripted child processes | clean | git/tmux/hug spawns assert in Rust; none run in-child assertions |

### Progress

- findings are worked serially in the order R1-F1, R1-F2, R1-F3, R1-F4, R1-F5, R1-F7, R1-F6
        - R1-F1 goes first because it restructures the Markdown/Prose writers that R1-F2/R1-F3/R1-F4 also touch
        - R1-F7 precedes R1-F6 so the repaired headless reader harness can verify R1-F6's encoding decisions
- starting the work on 'R1-F1 — Markdown and Prose discard required listing content and layout' at 15:21:48
- R1-F1 defect class: independent export serializers bypass the shared listing structure and therefore lose accepted layout options and required declaration content
        - sibling sites: `declarations::write_export` was the only Markdown/Prose entry; it fed `export::flatten_selected` rows to `prose::write_file` and `markdown::write_file` for all four commands, so every layout option (flat, `--group-by-file`, `--group-by-module`, both, package sections, `--sort name|kind`, `--prelude`, `--tree`, `--tree --group-by-module`, `classes` all/`--static-only`/`--instance-only`) was bypassed in both formats; TSV and ctags share the same flattening by design (flat rows/tags, no document layout) and are left unchanged
        - probe (temporary unit test, removed): `Prose::to_render_nodes` maps generated tags to Strong/Emphasis/Span/Link nodes with source text in Text nodes; leading spaces survive Prose rendering; `UnorderedList` renders `- ` bullets with 2-column continuation lines; a hanging row keeps `\n` doc lines flush with the row
        - design: the renderer's layout routines (`ListingStream`, package listing groups, outline, class sections) now write to a `render::Layout` trait instead of a byte writer; the terminal, Prose and Markdown outputs are three implementations, so all three share buffering, ordering, ownership, relation labels, class sections and declaration markup
        - implemented: `render::Layout` (heading/row/list/note/blank) with `TerminalLayout` (render.rs), `ProseLayout` (prose.rs) and `MarkdownLayout` (markdown.rs); `declarations::write_listing`/`write_tree`/`write_classes` take a `Presentation`, so Markdown/Prose run through `ListingStream`, `Outline` and the class sections; `write_export` now serves only TSV and ctags; `ReportFormat::Export` carries `tree` instead of `containment` (containment = `--tree` or ctags; `classes` already asks for it)
        - Markdown parses each generated markup line with `Prose::to_render_nodes` (new runtime dependency on the workspace `renderable` crate for the node types) and escapes text nodes for CommonMark at that boundary; Prose `--plain` drops style/link nodes from the parsed line and re-escapes the text, so it no longer scans the escaped string for `<` (incidentally repairs R1-F2); source LF can no longer reach Markdown structure because every source string already went through `render::literal` (incidentally repairs R1-F3.M1/M2); Prose/Markdown locations and links now come from each record's own `source.file_path` (incidentally repairs R1-F4.P1/M1)
        - tests: `export_prose_markdown` rewritten — 9 parsed-Markdown shape tests (tree nesting, bases under `--signatures none`, filtered owner context, classes all/static/instance with field sections, file/module/both groups, three Cargo packages incl. an empty one, empty selections, package-wide `--sort name`, three signature modes, comments as hard-break continuations, newline/markup filenames) and 19 Prose comparison tests (both spec comparisons over every command, 3 signature modes, `--comments`, selections incl. filtered owner and empty, groups, tree, sort); a temporary wrong-indent mutation failed 13 of the 19
        - goldens `symbols.markdown`/`symbols.prose` regenerated with `HUG_UPDATE_GOLDEN=1`: they now show the package sections, indentation, emphasis and headings of the listing
        - sibling checks: C1 — `symbols.tsv`/`symbols.ctags` goldens and `export_tsv`/`export_ctags` tests unchanged and passing (TSV/ctags still flatten through `write_export`); C2 — `symbols.plain`/`.tree`/`.table`/`.stats`/`-signatures-full.plain` goldens byte-identical, `outline`/`classes`/`reports` tests and `just test-l2` (tmux and WezTerm ran; Kitty entries finished in ~0.01s, a clean skip) pass
        - Markdown departures recorded for the docs: the scan root's own package heading (empty in the terminal) is written `## .`; an empty `classes` report prints `(no symbols)` in Markdown only, since terminal and Prose print nothing there
        - docs: `symbol-reports.md` Markdown/Prose section rewritten (layout list, Mermaid flow, Prose and Markdown examples taken from real output) and **planned** markers removed; CLI README and tree-hugger skill updated (the skill's "`--tree` is terminal-only" claim was stale); `renderable` edge documented in `tree-hugger/docs/dependencies.md` and root `docs/dependencies.md`
        - checks: `just lint` clean; `just test` 2000 passed, 0 failed; `just test-l2` 7 passed
        - R1-F1 implementation complete, ready for review
- orchestrator verification of R1-F1: rebuilt `hug` and reran the review's `Example.java` reproduction
        - Markdown `symbols --tree --signatures none` nests `build`/`open`/`field` under `Box extends Base`; Prose matches plain indentation
        - Markdown `classes` emits Static Methods / Instance Methods / Instance Fields nested sections
- work completed for 'R1-F1 — Markdown and Prose discard required listing content and layout' at 15:50:14
- starting the work on 'R1-F2 — Plain Prose deletes escaped source text' at 15:50:14
        - the R1-F1 subagent reported R1-F2 repaired as a side effect of the restructure (plain Prose drops style/link nodes after parsing); a dedicated subagent verifies every child row and sweeps siblings
- R1-F2 defect class: removing generated markup with an escape-unaware character scan also removes literal source content
        - verified independently: `prose::unstyled` parses each generated line with `Prose::to_render_nodes`, keeps only text/code node values and re-escapes them with `render::literal`; no `<` scan remains in `prose.rs`, `markdown.rs` or `render.rs`
        - sibling sweep of `cli/src` for `<` scans, `strip_*`, tag regexes and ANSI strippers: the remaining strippers are `strip_escape_codes` after rendering (`render::TerminalLayout::write_indented`, `stats::write_rendered`, `main::prose_line`); the first two only see source text that went through `render::literal`, so source never contains a raw ESC; `main::prose_line` (god-files) rendered names, paths, notes, doc summaries and refactor-hint names escaped with `Prose::escape_text` only, so under `--plain` a raw ESC sequence in a filename or doc was deleted with `strip_escape_codes` (and emitted raw when styled)
        - fixed: the six god-files sites now use `render::literal`; regression `cli::test_god_files_plain_output_keeps_escapes_and_markup_in_paths_literal` (unix) fails with `Prose::escape_text` restored and passes with the fix
        - tests added in `export_prose_markdown` (rendered literal equality on a no-color/no-link terminal): `plain_prose_keeps_generic_headers_literal` (D1), `plain_prose_keeps_generic_short_signatures_and_class_summaries_literal` (D2), `plain_prose_keeps_doc_tags_emphasis_and_unmatched_angles_literal` (D3), `plain_prose_keeps_generic_owner_names_literal` (D4, `[in Other<T>]`, `class Box<T>`), `plain_prose_keeps_angle_brackets_in_paths_literal` (C1, unix), `styled_prose_tsv_and_markdown_keep_generic_text` (C2)
        - proof: temporarily reinstating the old `strip_generated_tags` scan inside `prose::unstyled` failed all five plain-Prose tests (e.g. `Vec\)`, `literal \ and **bold**`, `x\.rs:1:1`); reverted and confirmed byte-identical
        - checks: `just test` 2007 passed, 0 failed; `just lint` exit 0
        - R1-F2 implementation complete, ready for review
- work completed for 'R1-F2 — Plain Prose deletes escaped source text' at 15:57:11
        - D1–D4 already fixed by the R1-F1 restructure and now pinned by rendered-literal tests; C1/C2 clean; reintroducing the old scan failed all five plain-Prose tests
        - sibling defect fixed: six `god-files` sites in `main.rs` escaped source with `Prose::escape_text` only, so `--plain` ESC stripping deleted filename text; they now use `render::literal`
- starting the work on 'R1-F3 — Source controls escape neutralization in Markdown and tags warnings' at 15:57:11
- R1-F3 defect class: some output boundaries preserve source control characters instead of making them inert
        - verified independently: every Markdown/Prose source string reaches markup through `render::literal`; `MarkdownLayout::item` splits only generated doc-line breaks; the review's `prefix\n\n        injected.java` filename parses as one list item whose text holds `prefix\u{a}\u{a}        injected.java:1:1`, with no code block (M1); names, signatures and docs reach Markdown the same way, and the Markdown escaper now also shows any control left in a text node as its `\u{..}` escape, so the invariant holds at the output boundary itself, not only upstream (M2, C4); doc continuation lines come only from the renderer's own line split (M3)
        - fixed T1: `declarations::skipped_tag` writes the ctags warning through `render::neutralized` (one visible line; `errors` keeps the raw path)
        - sibling stderr sites fixed with `render::neutralized`: per-file `error:` lines in `declarations::settle` (every declaration format, incl. JSON/JSONL/count/stats), the catalog error, the `--changed` Git error, `--verbose` cache miss/write lines (`scan::CacheStats::record`, shared by every command), the lint/imports per-file `error:` lines in `main.rs`, and `refs` per-file errors and its ambiguity list path
        - siblings clean: TSV cell encoder (C1); Prose styled and plain (C2); ctags stdout rejection of name/container/path controls (C3); table cells (`neutralized`); terminal/plain listings (`literal`); errors returned from `main` print through `Debug`, which escapes controls
        - not changed: `--json`/`--json-full`/`--jsonl` escape C0 controls (serde_json writes `\u001b`, `\n`), but U+0080–U+009F are valid unescaped JSON and are written raw (observed: `bad\xc2\x9b31m` in `errors[].file`); JSON is outside the new-format contract, reported for a decision
        - tests: `export_controls` (symbols binary): hostile docs/signatures in every export format and listing, cross-platform; Unix-only hostile filenames (LF + blank + indent, ESC/C1 CSI) asserting raw-byte inertness on stdout and stderr across TSV/Markdown/Prose both modes/ctags/plain/table, the ctags warning lines, the parsed Markdown item, and a failing hostile-path file's single stderr line across every mode; unit tests `markdown::tests::controls_in_a_text_node_are_shown_as_their_literal_escapes` and `declarations::export_write_tests::skipped_tag_warning_shows_path_controls_as_one_visible_line` keep Windows coverage of the logic
        - proof: reverting the `settle` and `skipped_tag` neutralization failed 4 of the 6 new tests; restored byte-identical
        - docs: `symbol-reports.md` tags section and failed-files section state the stderr contract
        - checks: `just test` 2014 passed, 0 failed; `just lint` exit 0
        - R1-F3 implementation complete, ready for review
- work completed for 'R1-F3 — Source controls escape neutralization in Markdown and tags warnings' at 16:06:03
        - T1 fixed via `declarations::skipped_tag` (warning neutralized, `errors` entry keeps raw path); M1/M2 already fixed by R1-F1 and pinned by parsed-event tests; Markdown escaper gained a control backstop (C4)
        - sibling stderr sites fixed: per-file `error:` line in `declarations::settle`, package-catalog and `--changed` errors, `--verbose` cache lines, lint/imports/refs per-file errors
        - note outside this spec: `--json` writes U+0080–U+009F raw (valid JSON; serde_json escapes only < 0x20). The spec's inert-control rule names the new formats only, so this is recorded as an observation, not an open finding of this review
- starting the work on 'R1-F4 — Prelude exports mix analysis-file paths with declaration locations' at 16:06:03
- R1-F4 defect class: a batch's analysis filename is substituted for a selected record's actual declaration filename
        - fixed T1: `tsv::write_rows` no longer takes the batch `display`; each `file` cell is the record's own `source.file_path`, projected once from the native path by `report::trim_record`
        - sibling defect fixed (C3): `ctags::write_record` re-projected the already scan-root-relative `file_path` against the working directory, so from a subdirectory of the package every tag read `src/src/alpha.rs`; it now writes the projected path as is and drops its `root` argument
        - already fixed by the `render::Layout` restructure, now pinned: Prose label and href both come from the record's native declaration path (P1); Markdown labels use the record's path (M1)
        - siblings clean: ctags omits synthetic aliases (C1); ordinary declarations and class members (C2: path agrees with batch); `--format table` joins the projected path onto `scan_root`; JSON/JSONL envelope `file` is batch ownership by contract while each record keeps its own `source.file_path`
        - test: `prelude::prelude_exports_locate_cross_file_aliases_at_their_declaration_in_every_format` (symbols binary, L1): exact TSV row, exact Prose `<a href="{native URL}#L2">src/alpha.rs:2:1</a>`, exact Markdown line, header-only ctags, and TSV/ctags run from `src/`
        - proof: restoring the batch path in TSV and the re-projection in ctags fails the test (`src/prelude.rs` vs `src/alpha.rs`); restored
        - docs: `symbol-reports.md` TSV section states `file`/`line`/`column` are the declaration's, from any working directory
        - checks: `just test` 2015 passed, 0 failed; `just lint` exit 0
        - R1-F4 implementation complete, ready for review
- work completed for 'R1-F4 — Prelude exports mix analysis-file paths with declaration locations' at 16:11:58
        - TSV now projects each record's own declaration path; Prose/Markdown already did after R1-F1 and are now pinned by `prelude_exports_locate_cross_file_aliases_at_their_declaration_in_every_format`
        - sibling defect fixed: ctags re-relativized already-projected paths against the working directory (`src/src/alpha.rs` when run from `src/`)
- starting the work on 'R1-F5 — Class record exports do not sort the flattened file' at 16:11:58
- R1-F5 defect class: sorting containers and each member set separately is mistaken for sorting the combined record export
        - fixed (T1, C1, T2, C2): `export::flatten_selected` gathers containers and displayed members, drops synthetic records when asked, deduplicates by ID, then sorts the one vector with `order::compare`; the non-class path is the same code with no members
        - `--sort name` is `String::cmp` (case-sensitive bytes, upper case first), so `Order.java` gives `Alpha, Zebra, aardvark, zulu`; `--sort kind` gives `Zebra, Alpha, aardvark, zulu` (class rank, then declaration position)
        - sibling sites checked, clean by contract: `selected_records` (JSON `members` map, JSONL `members` list, table), `write_classes` (terminal/Markdown/Prose) and `render::Outline` sort containers and keep members in source order under each, as `symbol-reports.md` documents for hierarchical outputs; non-class TSV/ctags already sorted once (S1)
        - tests: unit `export::tests::class_flattening_deduplicates_members_omits_summary_fields_and_sorts_the_whole_file` now asserts the combined name and kind orders; new `export_order` module (symbols binary, L1) asserts exact TSV and ctags name sequences for `classes` and `symbols` under name/kind/file sort, plus `--static-only`/`--instance-only` membership
        - proof: with the committed `export.rs` restored, `export_order` failed 2 of 3 (`Alpha, zulu, Zebra, aardvark`, the review's observation); the `symbols` sibling passed; fix restored
        - docs: `symbol-reports.md` TSV and tags sections state the single per-file order, and the `classes` members paragraph names TSV/ctags as the exception; tree-hugger skill notes it
        - no golden changed (the export goldens run `symbols`)
        - checks: `just test` 2018 passed, 0 failed; `just lint` exit 0
        - R1-F5 implementation complete, ready for review
- work completed for 'R1-F5 — Class record exports do not sort the flattened file' at 16:17:52
        - `flatten_selected` now gathers, dedupes by ID and sorts once; new `tests/symbols/export_order.rs` failed 2/3 against the old block order
        - sibling sites (classes JSON/JSONL/table, terminal/Markdown/Prose classes, outline) are clean: their documented contract is the grouped container layout
- starting the work on 'R1-F7 — The tags reader test passes failed assertions and never navigates' at 16:17:52
- R1-F7 defect class: a reader check mistakes successful process shutdown for successful assertions and substitutes metadata inspection for the required operation
        - A1 fixed: the reader script runs under `pcall(dofile, …)` and a failure exits through `cquit 1`; the test also requires a completion line that is printed only after the last case. Probe on this host (NVIM 0.12.1): the old shape (`lua assert(false)` then `qa!`) prints E5108 and exits 0; the `pcall` + `cquit 1` shape exits 1 on failure and 0 on success
        - A2 fixed: each case checks the `taglist()` entry's line, `kind:` and `class:` (exact value or absent), then runs `:{n}tag! name` and compares the resolved opened file with the Rust-canonicalized path (macOS `/private/var`) and `line('.')` with the declaration line. Cases: `duplicate` #1 `one.rs:1` and #2 `two.rs:2`, `Ω` and `inside` (`class:Ω`) in `dir with space/λ.rs`, and `outside_fn` at `../outside.rs:2` (selected with an absolute `--file`)
        - A3 fixed: the scan root is `repo/root`, `tags` is saved there, Neovim `chdir`s to it, and `tags` is set to its absolute escaped path
        - the cases are data (`READER_SOURCES`, `READER_CASES` and the runner `run_tags_reader` in `cli/tests/level2_declarations.rs`), so R1-F6 can add filename/name cases without touching the Lua
        - the test checks its own harness: a wrong metadata line (`address 2, expected 1`) and a wrong navigation file (`opened …`) must each exit non-zero without the completion line
        - proof: changing the expected `two.rs` line to 3 fails (`case 2 (duplicate #2): address 2, expected 3`); rewriting `two.rs` to `missing.rs` in the saved tags fails at navigation (`E429: File "missing.rs" does not exist`); both restored
        - C1 clean: `export_ctags` L1 tests make byte assertions in the Rust process
        - C2 clean: `level2_declarations` has `required-features = ["terminal-tests"]`, CI `features = ["terminal-tests"]`; `just check-tier-coverage tree-hugger-cli` reports 0 stranded; `just test-l2` selects the test
        - sibling sweep (processes spawned by tree-hugger tests): `git init` in both fixtures, `tmux capture-pane` in `level2_declarations`/`level2_god_files`, `run_scoped` pane commands, and `hug` spawns in `symbols`/`changed`/`cli`/`report` pipe tests all assert in Rust on status or captured output; no other test runs scripted assertions in a child process. Clean
        - docs: the `symbol-reports.md` tags section now lists what the reader test checks, says backslash names and paths are not covered, and says outside-root files get `../` paths so the tags file belongs at the scan root
        - checks: `just test-l2` 7 passed; `just test` 2018 passed, 0 failed; `just lint` exit 0; clippy with `terminal-tests` on `level2_declarations` exit 0
        - R1-F7 implementation complete, ready for review
- work completed for 'R1-F7 — The tags reader test passes failed assertions and never navigates' at 16:26:55
        - reader now runs `pcall(dofile, …)` + `cquit 1`, navigates with `:{n}tag!` and asserts canonical file + cursor line, and saves `tags` at the scan root; a built-in self-check proves a wrong expectation exits non-zero
        - deliberate breaks (wrong line; renamed file → E429) both made the test fail; reverted
- starting the work on 'R1-F6 — Tags encoding changes real filenames and qualified names' at 16:26:55
- R1-F6 defect class: unverified escaping and separator conversion alter values consumed by the named tags reader
        - per-field probe, headless NVIM 0.12.1 on macOS (hand-written tags files, `pcall` + exact `taglist()`/`:tag` checks): the name, file name and `class:` value are read byte for byte; no escape is decoded (`A\tB`, `A\\B`, `end\` stay as written). `Outer\Inner` is 11 bytes, `class:Outer\Inner\Account` 19, and `:tag` through `nvim_cmd` args navigates a backslash name. `kind:` is a fixed word list
        - file names: raw `back\slash.rs`, `a\\b.rs`, `\lead.rs`, `end\`, ` lead.rs`, `%`, `#`, `*`, `?`, `{…}`, inner `~`, `;"`, `|`, `!`, `'`, and `$` before `.`/space/`/`/end all open the file as written
        - new reader-side alterations found (same class, reader side): Neovim expands a tag file name holding a wildcard. `a$x.rs` opened an existing `aXX.rs` (WRONG), `$HOME.rs` → `$HOME` value, `a$$.rs` → PID, `a$-.rs` → shell flags, ``a`echo b`c.rs`` ran the command (`abc.rs`), `a[b].rs` opened an existing `ab.rs`, leading `~/f.rs` opened `$HOME/f.rs` and `~root/g.rs` → `/var/root/g.rs`. `\$` is not an escape (`dollar\$x.rs` → E429)
        - pseudo-tags: Bash/Zsh accept `!_TAG_X() { :; }`; hug wrote the line and Neovim dropped it (`taglist` 0, E426), exit 0. A function named `!_TAG_FILE_SORTED` would also be read as the sorted flag
        - projection: on Unix `PathIdentity::relative_from` always finds a route between two absolute paths and `route_text` keeps each component verbatim, so the trimmed record path still holds a literal `\`; the `to_portable_string` fallback (which converts `\`) is reached only with no route (another Windows drive/share). The writer's own `replace('\\', "/")` was the only fabrication. No native path needs carrying; TSV/JSON paths unchanged
        - fix (`cli/src/ctags.rs`): all three fields written raw; `Unsupported { Control, PseudoTagName, ExpandedPath }` returned by `write_record`; `reader_expands` rejects a leading `~`, a backtick, `[`, and `$` before anything but `.`/space/`/`/end; names starting with `!_` are skipped. `declarations::skipped_tag` names the reason (`ctags skipped {reason} in {path} at line {line}`), still `ctags_unsupported`, exit 1
        - N1 fixed, V1 fixed, P1 fixed (POSIX literal backslash is written raw and navigates to the real file). S1 clean + fixture extended (leading space, `%`, `cost$.rs`, `#` added to the reader). S2 clean, S3 clean, S4 clean (existing reader cases still pass). S5 clean (exhaustive match on 16 languages × 19 kinds; golden unchanged, it has no backslash). S6 clean (control matrix now also asserts the `Control` reason)
        - tests: L1 `ctags::tests::{backslashes_in_name_container_and_path_are_written_raw, pseudo_tag_names_are_skipped, paths_neovim_would_expand_are_skipped_and_the_rest_are_written}`; CLI `export_ctags::{ctags_writes_php_qualified_names_and_owners_with_single_backslashes, ctags_writes_a_literal_backslash_file_name_as_written (cfg(unix)), ctags_skips_pseudo_tag_names_and_expanded_file_names_with_warning_and_failure_status}`; L2 reader adds `Outer\Inner`, `deposit` (`class:Outer\Inner\Account`), ` lead.rs`, `pct%.rs`, `cost$.rs`, `hash#.rs`, Unix `back\slash.rs`, plus a skipped-sources run (`a$HOME.rs`, `a[b].rs`, `~home.rs`, `pseudo.sh`) asserting header-only stdout, one warning each, exit 1. `reader_path` keeps native spelling on Unix
        - proof: reinstating the old doubling/separator lines failed 4 L1/CLI tests; in L2 each alone failed: name doubling `case 6 (Outer\Inner #1): 0 taglist entries`, class doubling `case 7 (deposit #1): class Outer\\Inner\\Account`, path replace `E429: File "back/slash.rs" does not exist`; restored
        - docs: `symbol-reports.md` tags section gains "What is written as is, and what is skipped" (skip table, warnings, reader cases); CLI README tags paragraph; tree-hugger skill ctags row; `os` skill `windows-paths.md` item 9 (Unix `\` vs `to_portable_string`)
        - accepted boundary: Neovim is the reader; Universal Ctags-style decoders read a `\` in `class:` differently. Next.js-style `[id].tsx` files are skipped (exit 1) because a `[…]` name can open a sibling; needs the author's attention if that is too strict. Windows Neovim expansion was not probed
        - checks: `just test` 2024 passed on three consecutive runs (one earlier run went red with 357 tests not run; its failing test was not captured and did not recur); `just lint` exit 0; `just test-l2` 7 passed; clippy `--features terminal-tests --test level2_declarations -D warnings` exit 0
        - `just cross-check tree-hugger-cli --os windows`: every ctags test passed on native Windows (5 `export_ctags` incl. the skip test creating `$`/`[`/backtick/`~` names, 5 `ctags::tests`); the run is red on `changed ownership::*` (e.g. `dot_parent_staged`: `alias.rs` vs `clean.rs`), in `tests/changed/ownership.rs`, which this change does not touch
        - R1-F6 implementation complete, ready for review
- work completed for 'R1-F6 — Tags encoding changes real filenames and qualified names' at 17:06:23
        - headless Neovim 0.12.1 decodes no escapes in name, filename or `class:`; the writer now emits them raw (N1, V1, P1 fixed; old doubling/separator code fails the reader)
        - sibling hazards found by probing and fixed with skip + warning + exit 1: filenames Neovim expands (`~` prefix, backtick, `[`, `$` followed by an identifier character) and `!_`-prefixed names read as pseudo-tags
        - author decision surfaced (nonblocking): skipping any `[` filename makes Next.js-style `[id].tsx` repos exit 1 under `--format ctags`; this follows the spec's skip-or-prove rule
- reconciliation started at 17:06:23
- reconciliation completed at 17:17:30
        - inspected the combined diff of `ctags.rs`, `export.rs`, `tsv.rs` and confirmed each against its finding; no doc still carries a **planned** marker or names a feature directory

### Completion Report

- **outcome:** complete
- **elapsed:** about 1 h 56 min (15:21:07 → 17:17:30 local)
- **parent findings:** total 7 = fixed 7 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0
        - instance rows: 64 total, of which 39 fixed and 25 clean; none open
        - discovered sibling rows (`X1`) are included in those counts: `god-files` literal escaping, stderr control neutralization across commands, ctags working-directory path, Neovim-expanded filenames and `!_` pseudo-tag names, and the scripted-child-process test sweep
- **verified repairs**
        - R1-F1: Markdown and Prose now go through the shared listing, outline and class-section code via `render::Layout`; TSV/ctags keep flat rows
        - R1-F2: plain Prose drops parsed style/link nodes instead of scanning escaped text
        - R1-F3: tags warnings and every source-derived stderr line are neutralized; Markdown escaper has a control backstop
        - R1-F4: TSV uses each record's declaration path; Prose/Markdown verified; ctags no longer re-relativizes against the working directory
        - R1-F5: class TSV/ctags exports sort the combined per-file record set once
        - R1-F6: tags fields are written raw (empirically what Neovim reads back); expanded filenames and pseudo-tag names are skipped with warning + exit 1
        - R1-F7: reader test fails on any failed assertion, really navigates, and stores tags at the scan root
- **open finding IDs:** none
- **nonblocking notes for the author / next review**
        - skipping every filename containing `[` (to avoid Neovim glob matching a different file) makes `--format ctags` exit 1 on Next.js-style `[id].tsx` routes; this follows the spec's skip-or-prove rule, but the author may prefer accepting `[` and documenting the wrong-file risk
        - `--json` writes U+0080–U+009F raw (valid JSON); the spec's inert-control rule names only the new formats, so JSON was not changed
        - several stderr sites fixed under R1-F3.X1 (`--verbose` cache lines, `--changed`/catalog errors, refs) have no dedicated regression tests
        - `just cross-check tree-hugger-cli --os windows` (run by the R1-F6 subagent) passed every ctags test on native Windows but was red in `changed ownership::*` (`tests/changed/ownership.rs`); no `changed` source or test was touched in this cycle, so that failure is unrelated to these findings
- **changed files**
        - `tree-hugger/cli/src/`: `render.rs`, `prose.rs`, `markdown.rs`, `declarations.rs`, `main.rs` (R1-F1, R1-F2, R1-F3); `scan.rs`, `refs.rs` (R1-F3); `tsv.rs` (R1-F4); `export.rs` (R1-F5); `ctags.rs` (R1-F4, R1-F6)
        - `tree-hugger/cli/Cargo.toml`, `Cargo.lock` (R1-F1: `renderable` workspace dependency, already in the build graph via biscuit-terminal)
        - tests: `tests/symbols/export_prose_markdown.rs` (R1-F1, R1-F2), `tests/symbols/export_controls.rs` new (R1-F3), `tests/symbols/prelude.rs` (R1-F4), `tests/symbols/export_order.rs` new (R1-F5), `tests/symbols/export_ctags.rs` (R1-F6), `tests/symbols/main.rs`, `tests/level2_declarations.rs` (R1-F6, R1-F7), `tests/cli.rs` (R1-F2), goldens `symbols.markdown` and `symbols.prose` (R1-F1)
        - docs: `tree-hugger/docs/symbol-reports.md`, `tree-hugger/cli/README.md`, `tree-hugger/docs/dependencies.md`, `docs/dependencies.md`, `.claude/skills/tree-hugger/SKILL.md`, `.claude/skills/os/windows-paths.md`
- **final verification (orchestrator, after the last change)**
        - `just lint` in `tree-hugger/`: exit 0 (all findings)
        - `just test` in `tree-hugger/`: first run 1699 passed, 2 `cache_robustness` tests timed out at 30 s while host load average was about 81; those 13 tests passed alone (25 s), and the rerun `just test --no-fail-fast --test-threads 6` passed 2024 of 2024 (all findings)
        - `just test-l2` in `tree-hugger/`: 7 of 7 passed, including the repaired headless Neovim reader (R1-F6, R1-F7, R1-F1.C2)
        - `cargo clippy -p tree-hugger-cli --features terminal-tests --test level2_declarations -- -D warnings`: exit 0 (R1-F6, R1-F7)
        - `just check-tier-coverage tree-hugger-cli`: 0 stranded (R1-F7.C2 and every new test)
- implementation complete, ready for review

## Implementation of Review Findings #2

> **started at:** 2026-10-08T17:30:59-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-11-extra-report-formats/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the input robustness matrix does not apply: the review records that this feature adds output writers, not a file-format or configuration reader
- inventory: one parent finding (R2-F1), seven failing shapes (P1–P7), seven controls (C1–C7), nine consumer/decision rows (D1–D9) and the documentation repair (DOC1); the nonblocking recommendations are recorded separately as N1/N2

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|----|-----------------------------------------------------------|-------------|----------------------------------|
| R2-F1 | Tags silently change filenames that are not valid Unicode | fixed | every child row below is fixed or clean; full `just test` 2026/2026, `just lint` clean |
| R2-F1.P1 | Declaration filename contains byte `FF`: reject, warn, exit 1, keep valid entries | fixed | `cli/tests/symbols/export_ctags_paths.rs` `linux::a_file_name_that_is_not_utf8_is_skipped_and_other_files_are_kept`, `linux::a_report_whose_every_entry_is_unrepresentable_is_header_only`; Linux rig 8/8 |
| R2-F1.P2 | Parent directory contains byte `FF`: same unsupported-entry behavior | fixed | `linux::a_directory_name_that_is_not_utf8_is_skipped`; Linux rig pass |
| R2-F1.P3 | Outside-root declaration filename contains byte `FF`: validate native relative route | fixed | `linux::an_outside_root_route_that_is_not_utf8_is_skipped` (reached through a valid in-root link to `../bad\xff.java`, since `hug` arguments are UTF-8); Linux rig pass |
| R2-F1.P4 | Lossy replacement filename exists with different source: never point at it | fixed | `linux::an_existing_replacement_spelling_never_receives_another_files_tags`: the decoy is tagged only with its own `Decoy` entries, canonical identities compared |
| R2-F1.P5 | Valid symlink resolving to non-UTF-8 target: validate recorded native route | fixed | `linux::a_link_to_a_file_name_that_is_not_utf8_is_skipped`; Linux rig pass |
| R2-F1.P6 | Windows filename with unpaired U+D800: reject, warn, exit 1 | fixed | `windows::a_file_name_with_an_unpaired_surrogate_is_skipped_and_other_files_are_kept` (mixed and header-only); native Windows rig pass (subagent cross-check 21/21) |
| R2-F1.P7 | Windows parent directory with unpaired U+D800: reject | fixed | `windows::a_directory_name_with_an_unpaired_surrogate_is_skipped`; native Windows rig pass |
| R2-F1.C1 | Non-UTF-8 bytes only in scan root: keep `Example.java` tags | clean | `linux::a_scan_root_that_is_not_utf8_keeps_its_valid_routes`: exact accepted tags, exit 0, tags navigate |
| R2-F1.C2 | Actual filename containing valid U+FFFD: accept | clean | `a_real_replacement_character_file_name_is_tagged` (all OSes, macOS run) plus lib `the_lossless_projection_agrees_with_the_display_projection_on_unicode_names` |
| R2-F1.C3 | Names/containers: exact values, current unsupported policies unchanged | clean | name/container checks in `ctags::write_tag` unchanged; existing ctags unit and `export_ctags` byte tests pass |
| R2-F1.C4 | TSV and Markdown labels keep display projection | clean | TSV/Markdown/JSON still use `selected_records` → `trim_record` display projection; their goldens unchanged and passing |
| R2-F1.C5 | Prose href keeps native URL boundary | clean | Prose href path untouched; `export_prose_markdown` passes |
| R2-F1.C6 | Synthetic/pathless tags omitted before path checks | clean | `ctags::write_record` returns before path projection for synthetic/pathless records; prelude regression and `export::tests::ctags_flattening_can_drop_synthetic_records_while_tsv_keeps_them` pass |
| R2-F1.C7 | Unpaired surrogate only in Windows scan root: keep relative tags | clean | `windows::a_scan_root_with_an_unpaired_surrogate_keeps_its_valid_routes`; native Windows rig pass |
| R2-F1.D1 | Native discovery/read/query keep native paths | clean | native `source.file_path` kept until the writers (`native_selected_records`) |
| R2-F1.D2 | `functions` export: skip/warn/status through CLI | fixed | every case table runs `functions` with exact stdout/stderr/exit |
| R2-F1.D3 | `types` export: same | fixed | same tables run `types` |
| R2-F1.D4 | `symbols` export: default and broader scope, independent names | fixed | same tables run `symbols` and `symbols --all-scopes` with independently written entries |
| R2-F1.D5 | `classes` container projection keeps native validation | fixed | same tables run `classes`; containers decided from native paths |
| R2-F1.D6 | `classes` displayed members: static/instance filtering keeps safe records, rejects unsupported | fixed | same tables run `classes --static-only` and `--instance-only` |
| R2-F1.D7 | Flattening/sorting/ID dedup: carry representability, no order change | clean | `export::flatten_selected` unchanged (order never reads paths); `export_order` and `export::tests` pass |
| R2-F1.D8 | Unsupported decision before lossy conversion; header-only and mixed reports exact | fixed | new `ctags::Unsupported::UnrepresentablePath` decided from `lossless_portable_relative` before any lossy text; exact mixed and header-only assertions, exit 1 |
| R2-F1.D9 | Neovim navigation never sees unsupported entries; controls retained | fixed | `assert_tags_navigate` in every test joins each tag path onto the scan root and compares canonical identity with the declaring file |
| R2-F1.DOC1 | CLI README, `docs/symbol-reports.md`, tree-hugger skill describe the filename boundary | fixed | `cli/README.md`, `docs/symbol-reports.md`, `.claude/skills/tree-hugger/SKILL.md` and `api-reference.md` describe the boundary; `.claude/skills/os/windows-paths.md` records the surrogate-directory fact |
| R2-F1.X1 | Discovered sibling: `--format table` OSC 8 Location link built from the trimmed lossy path | fixed | table now displays the projected path but links the native record path (`render.rs`); `reports::table_location_links_resolve_against_the_scan_root` passes |
| N1 | Nonblocking: permanent all-export write-failure/stop-work probes and backtick corpus | deferred | explicitly nonblocking in review-2; the existing write-error/pipe tests already cover the behavior; next edit of those tests should add the all-export probes |
| N2 | Nonblocking: accepted skip policy for Neovim-expanded paths unchanged | clean | accepted Neovim-expansion skip policy unchanged; `reader_expands` untouched |

### Progress

- starting the work on 'R2-F1 — Tags silently change filenames that are not valid Unicode' at 17:31:14
        - defect class: a lossy display-path conversion is reused as a navigation path without validating that the emitted route still identifies the declaration file
        - design: `tree_hugger::lossless_portable_relative` (lib `file/naming.rs`) returns `portable_relative`'s text only when no name on the emitted route (or emitted absolute fallback) was replaced; both public functions share one private `project`, so they never choose different routes; the scan root's own spelling is never checked
        - design: the ctags export no longer receives `trim_record`-projected records; it flattens the native selection (`declarations::native_selected_records`, the same selection `selected_records` trims) and `ctags::write_record` derives the tag path from the native `source.file_path` and the scan root, so the representability decision is made before any lossy conversion; flatten/sort/dedup are unchanged because `order::compare` reads no path
        - completed: lib unit tests for the lossless projection (all-OS agreement incl. real U+FFFD; Linux byte filename/directory/outside-root/root-only; Windows surrogate filename/directory/root-only); `cargo test -p tree-hugger --lib naming` 6 passed on macOS
        - completed: `ctags::Unsupported::UnrepresentablePath` ("file name that is not valid Unicode"); `ctags::write_record(out, record, scan_root)` projects the tag path with `lossless_portable_relative` after the synthetic/pathless omission and before every other check; the warning names the display projection, the existing `skipped_tag`/`ctags_unsupported` path turns it into stderr + exit 1
        - sibling found and fixed: `--format table`'s Location OSC 8 link joined the `trim_record`-projected (lossy) path onto the scan root, so a non-Unicode name linked to its replacement spelling; `write_table` now passes the native selection and `Renderer::table` shows `portable_relative` but links the native path (the same target the listing's Prose links use; `reports::table_location_links_resolve_against_the_scan_root` still asserts table and listing links agree)
        - sibling sweep, no change: TSV (`tsv.rs` lossy `file_path`, display contract C4), Markdown (no links), Prose/listing hrefs (native path, C5), `--json`/`--jsonl`/`--json-full` (display contract), qualified-name `file#` prefix (names unchanged, C3), `refs` (JSON/text display only), `main.rs` completion candidates and the completer's own exe path (not tag/link targets)
        - completed: CLI tests `symbols::export_ctags_paths` (C2 on every OS; Linux P1 mixed + header-only, P2, P3 via a valid link outside the scan root, P4 decoy, P5 alias, C1; Windows P6 mixed + header-only, P7, C7), each over functions, types, symbols, symbols --all-scopes, classes, classes --static-only and classes --instance-only with literal expected entries, exact stdout/stderr/exit and a canonical-identity navigation check
        - completed (DOC1): `docs/symbol-reports.md` (skip table row, native-layout boundary table, decoy rule, warning example, display formats vs links; table Location link from the native path), `cli/README.md` tags paragraph, `.claude/skills/tree-hugger/SKILL.md` (ctags row with `UnrepresentablePath`, the `portable_relative` sibling, the corrected `selected_records`/`native_selected_records` rule that replaced the stale "join it onto `scan_root`" table guidance), `.claude/skills/tree-hugger/api-reference.md`
        - verified: `just test ctags` 15/15 then full `just test` 2026/2026 passed on macOS (a first full run under host load average ~140 from other sessions had 4 `cache_robustness`/`cache_lookup` 30 s timeouts in tests that never reach ctags or the table; `symbol_commands_run_no_diagnostics` passed alone and the full rerun at normal load passed with no timeouts); `just lint` clean; `just check-tier-coverage tree-hugger-cli` 0 stranded; doctest for `lossless_portable_relative` passed
        - verified cross-OS: `just cross-check tree-hugger-cli --os linux export_ctags ctags:: export:: table_location_links export_controls` 31/31 passed (all seven Linux `export_ctags_paths` tests); same filter `--os windows` 21/21 passed (all three Windows tests); `just cross-check tree-hugger --os linux naming::` 7/7 and `--os windows naming::` 7/7 passed
        - os skill: added to `windows-paths.md` that a surrogate-named directory works as a child process cwd including `hug`'s Git discovery
- work completed for 'R2-F1 — Tags silently change filenames that are not valid Unicode' at 18:15:48
        - orchestrator reconciliation: inspected the `naming.rs`, `ctags.rs` and `declarations.rs` diffs and the new test file (exact stdout/stderr/exit assertions); reran the Linux path tests on the rig (8/8 passed) and the full local suite

### Completion Report

- **outcome: complete**, elapsed 2026-10-08T17:30:59-07:00 → 18:16 (about 45 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0
        - instance rows: 26 required child rows (P1–P7, C1–C7, D1–D9, DOC1, discovered X1): 14 fixed, 12 clean, 0 open
        - nonblocking recommendations, counted separately: N1 deferred (nonblocking per review), N2 clean
- verified repairs:
        - `tree_hugger::lossless_portable_relative` (lib `file/naming.rs`) shares one projection with `portable_relative` and returns `None` when any emitted route name (or emitted absolute fallback) is not valid Unicode; scan-root spelling is never checked, and a real U+FFFD name is kept
        - `ctags::write_record` now takes the native record and the scan root, decides the tag path itself and refuses an unrepresentable route as `Unsupported::UnrepresentablePath` through the existing `ctags_unsupported` warning and exit 1; `declarations::native_selected_records` feeds ctags, while TSV/Markdown/JSON keep the trimmed display projection
        - discovered sibling R2-F1.X1: the `--format table` Location link now targets the native path instead of the lossy projected text
- open finding/child IDs: none (N1 is an explicitly nonblocking recommendation, deferred to the next edit of the write-error tests)
- performance measurement: none required; `deferred_perf_measurement` stays `false`
- changed files (this iteration):
        - code: `tree-hugger/lib/src/file/naming.rs`, `tree-hugger/lib/src/file/mod.rs`, `tree-hugger/lib/src/lib.rs`, `tree-hugger/cli/src/ctags.rs`, `tree-hugger/cli/src/declarations.rs`, `tree-hugger/cli/src/render.rs`
        - tests: `tree-hugger/cli/tests/symbols/export_ctags_paths.rs` (new), `tree-hugger/cli/tests/symbols/main.rs`
        - docs: `tree-hugger/cli/README.md`, `tree-hugger/docs/symbol-reports.md`, `.claude/skills/tree-hugger/SKILL.md`, `.claude/skills/tree-hugger/api-reference.md`, `.claude/skills/os/windows-paths.md`
- final verification:
        - `just lint` (tree-hugger): clean for both packages — all rows
        - `just test --test-threads 4 --no-fail-fast` (macOS): 2026/2026 passed — all rows; a preceding fail-fast run timed out one unrelated lib test (`symbols_query::declaration_facts_of_narrow_queries_equal_the_complete_index::shared_rows_4`) at host load average 173 and passed on the rerun
        - `just check-tier-coverage tree-hugger-cli`: 0 stranded — new test registration
        - `just cross-check tree-hugger-cli --os linux export_ctags_paths` (orchestrator): 8/8 passed — P1–P5, C1, C2
        - subagent: `just cross-check tree-hugger-cli --os linux|windows` with ctags/export/table filters 31/31 and 21/21; `just cross-check tree-hugger --os linux|windows naming::` 7/7 each — P6, P7, C7 and the lib projection
        - full Linux, Windows and WSL2 suites were not run locally; CI covers them
