---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/fixes/2026-11-04-composition-errors/spec.md
plan: darkmatter/fixes/2026-11-04-composition-errors/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
  - Cargo.lock
  - renderable/Cargo.toml
  - renderable/src/lib.rs
  - renderable/src/output/mod.rs
  - renderable/src/output/link_metadata.rs
  - renderable/src/color/capability.rs
  - renderable/src/html/mod.rs
  - renderable/src/tree/attrs.rs
  - renderable/src/tree/document.rs
  - renderable/src/tree/validate.rs
  - renderable/src/tree/render/markdown.rs
  - renderable/src/tree/render/browser.rs
  - renderable/tests/html_output_width.rs
  - renderable/tests/link_metadata_folds.rs
  - renderable/tests/markdown_output_width.rs
  - renderable/tests/html_policy_comment_nodes.rs
  - renderable/tests/markdown_container_line_endings.rs
  - renderable/tests/markdown_footnote_continuation.rs
  - renderable/tests/markdown_footnote_labels.rs
  - renderable/tests/markdown_html_contexts.rs
  - renderable/tests/markdown_line_endings_in_fields.rs
  - renderable/tests/markdown_literal_text.rs
  - renderable/tests/markdown_raw_html_payloads.rs
  - renderable/tests/markdown_single_line_raw_html.rs
  - renderable/tests/markdown_structural_breaks.rs
  - renderable/tests/markdown_table_cell_blocks.rs
  - renderable/tests/markdown_table_cell_pipes.rs
  - renderable/tests/markdown_unsupported_placeholder.rs
  - renderable/tests/support/adjacent_code_matrix.rs
  - biscuit-file/lib/src/json5/types.rs
  - biscuit-terminal/cli/src/commands/mermaid.rs
  - biscuit-terminal/lib/src/lib.rs
  - biscuit-terminal/lib/src/output/mod.rs
  - biscuit-terminal/lib/src/output/data.rs
  - biscuit-terminal/lib/src/output/profile.rs
  - biscuit-terminal/lib/src/output/width.rs
  - biscuit-terminal/lib/src/components/prose/markdown.rs
  - biscuit-terminal/lib/src/components/prose/mod.rs
  - biscuit-terminal/lib/src/components/prose/parity.rs
  - biscuit-terminal/lib/src/components/prose/prose.rs
  - biscuit-terminal/lib/src/components/prose/render.rs
  - biscuit-terminal/lib/src/components/prose/tokens.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/discovery/detection/color.rs
  - biscuit-terminal/lib/src/render_tree/link.rs
  - biscuit-terminal/lib/src/render_tree/mod.rs
  - biscuit-terminal/lib/src/render_tree/options.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/terminal.rs
  - biscuit-terminal/lib/src/utils/block_constraint.rs
  - biscuit-terminal/lib/tests/l1/main.rs
  - biscuit-terminal/lib/tests/l1/output_dispatch.rs
  - biscuit-terminal/lib/tests/l1/render_terminal_output.rs
  - biscuit-terminal/lib/tests/l1/level1_apple_terminal_prose.rs
  - biscuit-terminal/lib/tests/l1/prose_destinations.rs
  - biscuit-terminal/lib/tests/l1/prose_grammar.rs
  - biscuit-terminal/lib/tests/l1/render_tree_code_context.rs
  - biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs
  - biscuit-terminal/lib/tests/l1/tree_layout.rs
  - biscuit-visualized/src/src/components/mermaid.rs
  - biscuit-visualized/src/src/components/render_hook.rs
  - biscuit-visualized/src/tests/l1/display_size.rs
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/cli/tests/l1/entry_point_parity.rs
  - darkmatter/cli/tests/l1/magic_root.rs
  - darkmatter/cli/tests/l1/schema_validate_baseline.rs
  - darkmatter/cli/tests/l1/schema_validate_timings.rs
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_valid/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_fileref/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union/expected.pretty
  - darkmatter/lib/src/layout/page/tests.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/tests/l1/mermaid_text_line_numbers.rs
  - darkmatter/lib/tests/l1/render_tree_hr_snapshots.rs
  - darkmatter/lib/tests/l1/render_tree_roundtrip.rs
  - darkmatter/lib/tests/l1/yaml_block_parity.rs
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_hyperlink_exact_width_terminal.snap
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_hyperlink_max_width_terminal.snap
  - model-citizen/cli/src/output.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/recent_commits_prose_layout.rs
  - sniff/cli/tests/l1/cli.rs
  - sniff/cli/tests/level2/level2_git_status_styling.rs
  - sniff/cli/tests/level2/level2_recent_commits_rendering.rs
docs_updated_during_phase_1:
  - docs/dependencies.md
  - renderable/docs/tree-rendering.md
  - biscuit-terminal/docs/components/prose.md
  - biscuit-terminal/docs/components/terminal_image.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/biscuit-terminal/SKILL.md
  - .claude/skills/biscuit-terminal/render-tree.md
  - .claude/skills/biscuit-terminal/image-rendering.md
  - .claude/skills/renderable/SKILL.md
  - .claude/skills/renderable/tree.md
  - .claude/skills/worktree/list.md
  - .claude/skills/darkmatter/terminal.md
packages:
  - renderable
  - biscuit-terminal
  - biscuit-terminal-cli
  - biscuit-file
  - biscuit-visualized
  - darkmatter
  - darkmatter-cli
  - model-citizen-cli
  - sniff-detection-cli
decisions_for_review:
  - |-
    Width parsing gained a sixth request kind. The plan listed Auto, Source, Unlimited, Columns and Percent; I added Fractional(f64) so a value like 72.9 is kept until resolution, which floors it to 72 columns and returns the "takes whole columns" warning text. Alternative: floor while parsing and return the warning beside the request. Redoing it means changing the parse return type in biscuit-terminal's output/width.rs and its callers (none outside tests yet).
  - |-
    The warning code for a fractional width (dm.output.fractional_width) is not produced by biscuit-terminal; ResolvedWidth carries only the warning text, so the shared library holds no Darkmatter-prefixed code. Darkmatter attaches the code when it reports the warning (Phase 3/4). Alternative: put the code in biscuit-terminal. Redo cost: one field.
  - |-
    Plain-only structure (quotes as "> ", rule as "---", headings without "#", disclosure bodies as plain blocks) applies when the plain profile is pinned, not whenever a terminal merely has no color. The dispatch pins plain for --output plain and for a terminal request off a color TTY, which is what the spec's degradation rule needs; a direct component render on a colorless terminal keeps today's structure without color. Alternative: key these rules on color depth None, which would change every component rendered to a NO_COLOR terminal. Redo cost: one condition in the terminal fold plus snapshot updates.
  - |-
    TerminalRenderContext::from_terminal keeps Columns(terminal.width()), including the existing 80-column fallback off a TTY, so components rendered without the new output path behave as before. Only from_profile / render_terminal_output take an explicit Width (the CLI resolves width to Unlimited when nothing is known). Alternative: make from_terminal use detected_columns() and fall back to Unlimited, which changes every Prose/component render on a pipe across the monorepo. Redo cost: one constructor plus many expectations.
  - |-
    Terminal::detected_columns() does not read COLUMNS (neither did the existing detection, despite the spec's wording). Plan rule 5 says the CLI reads its captured environment once, so COLUMNS (like OUTPUT_COLUMNS) should be passed in by the CLI when Phase 3/4 wire width resolution. Alternative: read COLUMNS ambiently inside biscuit-terminal. Redo cost: small.
  - |-
    Browser width uses a page-stylesheet rule on prose elements (:where(p, li, blockquote, dl, h1..h6){max-width: Nch}) rather than a max-width on the body, so code blocks and tables are not narrowed (the plan requires that). Side effect: a code block nested in a list item is still bounded by the item. Alternative: a wrapper element around prose runs. Redo cost: renderable html/browser only.
  - |-
    Markdown Columns(n) reflow refuses a break that would start a block construct and moves it earlier (or lets the line overflow) instead of escaping the character, keeping text exact for round trips. The refused set is deliberately broad (including ":" for Darkmatter directives and "<" for HTML). Alternative: escape. Redo cost: renderable markdown.rs only.
  - |-
    A styled terminal cuts code rows at Columns(n) with no ellipsis (hook output and built-in fallback alike); plain never cuts. A Mermaid text drawing's rows are not cut so the hook's fit check still works. Alternative: an ellipsis marker. Redo cost: one helper.
  - |-
    The -v URL a non-clickable link prints is the authored destination, not the resolved file:// URL Prose would hand to OSC 8. The image rule applies whenever an image falls back, regardless of OSC 8 support. Alternative: print the resolved URL. Redo cost: the Link arm of the terminal fold.
  - |-
    Because piped links now print text only (spec rule, accepted trade-off), two CLIs needed their existing -v count forwarded to Prose so tests could still observe resolved paths: md schema validate (linked document rows) and sniff's repo hash header (render_hash_section already received verbose but ignored it). Their tests now pass -v. sniff's git-status CLI test can no longer see which short hashes link to the remote on a pipe; that distinction is still proven through `repo hash -v`. Alternative: leave those CLIs untouched and drop the assertions. Redo cost: two call sites.
  - |-
    The legacy key=value detector in renderable is today's reference_parse::is_structured unchanged, so a prose title such as "a = b" raises the legacy notice (today it was treated as structured attributes). Alternative: tighten the detector (for example require a quoted value). Redo cost: one function and its test.
  - |-
    Pretty TOML from biscuit-file writes arrays across lines; the dispatch uses that writer as-is rather than adding a second TOML formatter.
  - |-
    Added Json5::from_value to biscuit-file (public, additive) so the dispatch converts an in-memory value without serializing and re-parsing it.
message_to_agent: |-
  Phase 1 is complete; the shared foundations Phase 2+ build on are:
  - renderable::output::{CommandOutput, OutputOptions, TerminalProfile, Width} (also re-exported at the renderable root) and renderable::output::link_metadata::{parse_link_title, StructuredTitle, LinkMetadata, QueryParams, QueryValue, LinkTitleNotice (code(): dm.link.malformed_structured_title / dm.link.legacy_structured_title), is_legacy_structured_title}. NodeAttrs::set_link_metadata/link_metadata() is the typed qp carrier on Link nodes; every fold already lowers it (Markdown JSON5 title, HTML query + heading=, OSC 8 params).
  - biscuit_terminal::output::{write_output, render_output (returns text + diagnostics), OutputTarget (FromStr, `browser` alias), OutputError, serialize_data, terminal_target_options, parse_width_request, resolve_width, ResolvedWidth, WidthError, resolve_stderr_width} and Terminal::output_profile()/detected_columns(). render_tree::render_terminal_output(&Document, &OutputOptions, strictness) is the generic terminal fold; TerminalRenderContext::from_profile/pin_plain exist.
  - Darkmatter itself was only compile-migrated: TerminalOptions.max_width is still u16, HyperlinkMode::Never is still ignored on the render-tree path, DarkmatterPage still re-detects ambient width, page decoration still pads (P2/P3/P6/P7/P8 remain for Phase 3's Document rendering task). Darkmatter's render/link.rs still uses the old key=value parser; switch it to parse_link_title in Phase 2's Link migration (keep reference_parse::parse_structured only for md clean migration).
  - Prose has with_verbosity(u8); md's global -v now reaches md schema validate's linked rows. Wire OutputOptions.verbosity from -v everywhere else when commands move to write_output.
  - Pre-existing failure to expect: darkmatter::l1 shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry (claudine/docs/research/sandboxing/providers/sandboxing.yaml). Run darkmatter's suite with `just test -- --skip every_kinded_schema_artifact_passes_its_definition_through_the_registry`; `just test -E '...'` with parentheses breaks the recipe's bash array.
  - When running claudine-cli tests directly, enable its recipe features (test-fixtures, terminal-tests) or the drain-budget test waits the production 10 s and fails.
---

# Implementation Log for 2026-11-04-composition-errors (6 phases)

## Phase 1

### Wave 1 — Output contract (orchestrator)

- Added `renderable/src/output/mod.rs`: `CommandOutput` (exact spec signatures), `OutputOptions { verbosity, width, terminal_profile }`, output `Width::{Columns(u32), Unlimited, Source}` (helpers `columns`, `as_columns`, `saturating_sub`, `capped_at`; no `Default`, so every site chooses), `TerminalProfile` (target-neutral capabilities; `plain()` is the canonical plain profile, `degrades_to_plain()`, `without_hyperlinks()`), `OutputOptions::into_plain()`. Re-exported from the crate root.
- Added `renderable/src/output/link_metadata.rs`: one structured-title interpreter `parse_link_title` (JSON5 object → `StructuredTitle`; `{`-but-not-object → plain + `Malformed` notice; legacy `key=value` → plain + `Legacy` notice; anything else plain), `QueryParams` (authored order, last value wins at first position), `QueryValue` (string / finite number / bool, original scalar kept for round trip, `to_transport()` for query/OSC), `LinkMetadata`, `StructuredTitle::to_json5()` (fixed key order; `qp` keeps authored order). Order is preserved by a private ordered visitor rather than relying on serde_json `preserve_order` being feature-unified.
- `NodeAttrs.link_metadata: Option<Box<LinkMetadata>>` + `set_link_metadata`/`link_metadata()`; validation rejects it on non-Link nodes.
- `Document::new(root)` convenience constructor (every `as_prose()` builds one).
- `MarkdownRenderOptions.width` and `BrowserRenderOptions.width` (default `Source`) added up front so the Wave 2 agents start from a compiling tree; 27 struct-literal call sites across renderable/darkmatter tests were given `Width::Source` mechanically.
- `json-five = "0.3"` added to renderable (already in the lockfile via biscuit-file); `docs/dependencies.md` updated.
- Tests: 23 targeted renderable tests pass (`cargo nextest run -E 'test(/output|link_metadata/)'`).
- Note: the ported legacy detector is today's `reference_parse::is_structured` unchanged, so a prose title like `a = b` is (as today) detected as legacy `key=value`.

### Wave 2 — dispatch (orchestrator, B1)

- `biscuit-terminal/lib/src/output/{mod,profile,data,width}.rs`: `OutputTarget` (9 values, `browser` alias, `family`, `extension`), `OutputError::{Render, Serialize{path}, Io}`, `render_output` (fully buffered; returns text + generic-fold diagnostics) and `write_output` (spec signature), `terminal_target_options` (plain pinning / degradation), `Terminal::output_profile()`, `serialize_data` (JSON pretty, JSON5/YAML/TOML via biscuit-file; TOML null/root rules with JSON-pointer paths), `WidthRequest`/`parse_width_request`/`resolve_width`/`ResolvedWidth`/`WidthError`/`resolve_stderr_width`.
- `biscuit-file`: added `Json5::from_value` so data conversion needs no parse round trip.

### Wave 2 — Markdown width (subagent A, renderable)

- `MarkdownRenderOptions.width`: `Source` byte-identical to before (all 705 pre-existing renderable tests unchanged); `Unlimited` joins soft breaks; `Columns(n)` joins and wraps paragraphs only, at recorded literal-space break points, so code spans, link/image destinations+titles, autolinks and raw HTML never break; a break that would start a block construct (`#`, `>`, list markers, fences, `|`, `:`, `<`, …) or create a backslash hard break moves earlier; width measured with `unicode-width` (new edge, already in the lockfile).
- Links carrying `link_metadata` write `StructuredTitle::to_json5()` in `'…'` delimiters (round-trip proven through pulldown-cmark + `parse_link_title`).
- Browser: `qp` → query parameters (existing pairs first, then `heading` from the fragment, `line`, `column`, rest sorted; metadata wins collisions; one `heading`; no fragment, no `data-*`). `BrowserRenderOptions.width = Columns(n)` adds one page rule `:where(p, li, blockquote, dl, h1…h6){max-width: {n}ch;}`; `Source`/`Unlimited` add nothing.
- Tests: `renderable/tests/{markdown_output_width,link_metadata_folds,html_output_width}.rs` (26 tests). Docs: `renderable/docs/tree-rendering.md`, `.claude/skills/renderable/tree.md`.

### Wave 2 — Terminal render tree (subagent B2, biscuit-terminal + compile migration)

- `TerminalRenderContext.width`/`available_width` are `renderable::Width`; new `verbosity`, `plain`; `from_profile`, `from_output_options`, `profile()`, `pin_plain()`; `TerminalRenderOptions::{from_output_options, pin_plain}`; `render_terminal_output(&Document, &OutputOptions, strictness)`; `TerminalCodeContext` carries `Width`; `DiagramRenderRequest.available_width: Width`; `ImagePlaceholder` removed; `Terminal::detected_columns()`; `From<renderable::ColorDepth>` for the detection enum; `Table::natural_render_width`.
- Rules landed: unbounded widths never fill (3-glyph styled rule, natural-width tables, code band hugging its longest line, `max-width` still caps); plain pinning (no ESC byte, no trailing whitespace, `> ` quotes, `---` rule, headings without `#`, disclosure body as blocks, fences kept and never cut); styled code cut at `Columns(n)`; non-clickable link/image rule (`text`, `text <url>` at -v, URL once when text == URL, `IMAGE[alt]`, `image: alt <path>`); printed URLs are unbreakable atoms through Prose's outer wrap; OSC 8 with `qp` metadata uses a bare URI and `line:column:heading` params then sorted keys, dropping unsafe pairs; Prose parses destination and title separately and routes titles through `parse_link_title`.
- Tests: `biscuit-terminal/lib/tests/l1/render_terminal_output.rs` (15) plus unit tests in `link.rs`, `block_constraint.rs`, `options.rs`, and darkmatter `code_renderer.rs`. Expectations encoding the removed `[text](url)` / `[alt]` / `▉ IMAGE[alt]` fallbacks were updated (listed in its report: prose unit/parity tests, `level1_apple_terminal_prose`, `prose_destinations`, `prose_grammar`, `render.rs` text-layout tests, darkmatter `execution_failed_rendered_diagnostic_includes_linked_path`, `local_image_alignment_pads_terminal_fallback`, two hyperlink snapshots).

### Wave 3 — Foundation checkpoint (orchestrator)

- Registered `biscuit_terminal::output`; added `biscuit-terminal/lib/tests/l1/output_dispatch.rs` (11 tests: plain = generic terminal fold with plain profile; terminal off a color TTY == plain bytes; terminal override also serves plain with the plain profile/width/verbosity; per-target override fallback; Markdown fold at Source/Unlimited and markdown-plus parity; HTML ch constraint; data parity across JSON/JSON5/YAML/TOML; TOML null-array failure writes nothing and names `/items/1`; broken writer → `OutputError::Io`; unlimited has natural size).
- Biscuit-file's pretty TOML writes arrays multi-line; expectations follow that writer rather than reformatting.
- Added `Prose::with_verbosity(u8)` (spec: the `-v` count reaches the fold). `md schema validate` now forwards its `-v` count to its linked rows, and sniff's `render_hash_section` forwards its already-plumbed (previously unused) `verbose` to the commit header.
- Tests updated for the spec's non-clickable link rule (piped links print text alone; `-v` adds ` <url>`): darkmatter `entry_point_parity` (SchemaValidate cell passes `-v`, parses `<file://…>`), `magic_root` (schema validate route passes `-v`), `schema_validate_baseline` (`-v`; 8 `expected.pretty` snapshots now `doc.md <{DOC_URL}>`), `schema_validate_timings` (stdout compared with link destinations removed; the `-v` test now asserts the destination appears and the plain run has none), `mermaid_text_line_numbers` (styled code rows are cut at the width, so the numbered fence shows `stateDiagr`), model-citizen-cli `search_result_cells_render_as_single_line_inline_text`, sniff `git_status_case_a…`, `links_fall_back_to_markdown_without_osc8_support` → `links_print_their_text_without_osc8_support`, `cli.rs` (file link, worktree label, `repo hash` now `-v`), and sniff L2 `level2_git_status_styling` / `level2_recent_commits_rendering` (compiled, not executed here: L2).
- Docs/skills: `biscuit-terminal/docs/components/prose.md` (non-clickable links, structured titles), `terminal_image.md` (one image fallback), biscuit-terminal skill (`SKILL.md` dispatch guidance, `render-tree.md`, `image-rendering.md`), renderable skill placeholder text, worktree skill note.

#### Gates run (macOS)

| Gate | Result |
|---|---|
| `cargo check --workspace --all-targets` | clean |
| `just lint` renderable / biscuit-terminal / darkmatter / biscuit-file | pass (darkmatter needed `#[allow(clippy::too_many_arguments)]` on `emit_outcome`, matching `run_validate` in the same file) |
| `just test` renderable | 731/731 |
| `just test` biscuit-terminal | 3991 passed, 55 skipped |
| `just test` darkmatter (`-- --skip every_kinded_schema_artifact_passes_its_definition_through_the_registry`) | 13057 passed, 11 skipped, none ≥ 5 s |
| `just check-tier-coverage` renderable / biscuit-terminal / darkmatter / sniff | pass |
| Dependents: biscuit-visualized (1158), biscuit-tui + biscuit-icon + biscuit-icon-cli + simplified-schema (2229), tree-hugger-cli + content-policy-cli + git-wt-cli + homelab-cli + messenger-cli + playa-cli + model-citizen-cli + unchained-ai-cli + sniff-detection-cli + biscuit-speaks-cli (4681), repo-deps (496), claudine/claudine-cli/claudine-gen rendering/output/error/drain filter with the recipe's features (1784), biscuit-file json5 (54 + doctests) | all pass |
| `just cross-check renderable --os windows` / `biscuit-terminal --os windows` | pass (731 / 3567) |

#### Pre-existing and load-sensitive failures

- **Pre-existing:** `darkmatter::l1 shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` fails on `claudine/docs/research/sandboxing/providers/sandboxing.yaml` (`enum({{ as_csv(...) }}; required)` is not a valid enum member list). Neither the file (last changed in `0caa06d25`) nor SimplifiedSchema is touched by this phase; skipped in the final darkmatter run.
- **Load-sensitive:** repo-deps `ci-build` `archive::tests::{an_archive_of_more_programs_than_the_plan_resolved_is_refused, an_archive_missing_a_declared_binary_is_refused_before_any_test_starts, the_fixture_archive_carries_every_declared_payload_class}` and `tests::the_javascript_publisher_reads_a_fixture_generated_from_this_report` failed only when run beside ten other packages; each passes alone (67/67 archive tests, 496/496 repo-deps).
- A first claudine run without the recipe's `test-fixtures` feature failed `stalled_deliveries_share_one_drain_budget…` (it waited the production 10 s budget); with the recipe's features it passes.

#### Requirement → test mapping (Phase 1)

| Requirement | Tests |
|---|---|
| Exact `CommandOutput` contract, `OutputOptions`, plain profile | `renderable output::tests::*`; `output_dispatch::{a_terminal_override_also_serves_plain…, overrides_apply_per_target…}` |
| Structured titles: JSON5 object, malformed `{`, legacy `key=value`, qp scalar normalization, unknown keys, round trip | `renderable output::link_metadata::tests::*` (16); `link_metadata_folds::*`; `render_terminal_output::{a_prose_json5_title_becomes_osc8_params, a_prose_plain_title_stays_a_title}` |
| Typed link metadata on Link nodes only | `tree::validate::tests::link_metadata_is_valid_only_on_a_link` |
| Markdown width Source/Unlimited/Columns, atomic constructs, code/tables untouched | `markdown_output_width::*` |
| HTML qp/heading query mapping and ch width | `link_metadata_folds::html_*`, `html_output_width::*`, `output_dispatch::html_columns_constrain_prose` |
| Terminal width enum, natural size under Unlimited, rule/band/table fills | `render_terminal_output::*`, `output_dispatch::unlimited_terminal_output_has_its_natural_size` |
| Plain escape-free corpus at 40/80/unlimited | `render_terminal_output::plain_corpus_follows_the_plain_rules_at_every_width`, `output_dispatch::plain_is_the_generic_terminal_fold…` |
| Non-clickable link/image rule, unbreakable URLs, OSC 8 params | `render_terminal_output::{non_clickable_links_and_images…, a_printed_url_is_one_unbreakable_word, a_printed_url_survives_outer_prose_wrapping, osc8_link_metadata_rides_in_the_params_with_a_bare_uri}`, `link.rs` unit tests; `schema_validate_timings::single_verbose_output_is_unchanged` (end to end through `md`) |
| Dispatch buffering, serialization errors before writes, IO errors | `output_dispatch::{a_serialization_failure_writes_nothing, a_broken_writer_is_an_io_error}` |
| TOML null/root rules, heterogeneous arrays | `output::data::tests::*`, `output_dispatch::data_targets_serialize_the_same_data` |
| Width parsing/resolution outcomes | `output::width::tests::*` (9) |
| Terminal degradation to plain | `output::profile::tests::*`, `output_dispatch::terminal_degrades_to_exactly_plain_off_a_color_tty` |

#### Scope check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` reports `change_class: package` and `packages: [biscuit-file, biscuit-terminal, biscuit-terminal-cli, biscuit-visualized, darkmatter, darkmatter-cli, model-citizen-cli, renderable, sniff-detection-cli, test-toolkit, claudine, sniff]`, so the test and lint gates were run, not skipped.
