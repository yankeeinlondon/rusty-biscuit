---
total_phases: 6
created: 2026-11-05
phase: 2
agent: codex/gpt-6.1-sol
yolo: 'true'
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
---

# Composition Errors Implementation Plan

## Summary and Definition of Done

Implement `2026-11-04-composition-errors` as one change: source-aware reference reports, shared output dispatch, escape-free plain rendering, explicit width semantics, JSON5 link metadata and cleanup migration, and consistent output selection across `md`. Preserve the specification's sequence: shared foundations, reference report, compose/render, other commands, documentation. Execute all six phases without human checkpoints. The final state is implementation complete and ready for review; leave the fix in its active directory.

Successful completion means:

- Reference reports name the reason, construct, reference, containing document, authored character position, composed section, and inclusion trail. Prepared coordinates remain available for graph internals. In-memory documents never acquire invented authored coordinates.
- `renderable::CommandOutput` exposes the two sources and four optional overrides; `biscuit_terminal::write_output` is the shared formatted/data dispatcher. Plain uses the terminal path with one canonical plain profile.
- Every command accepts the declared output families and `--width`. Compose accepts its Markdown default and the four data targets. Defaults, removed arguments, streams, exit statuses, width precedence, and `--show` follow the specification and the explicit compatibility rules below.
- Both generic tree rendering and Darkmatter's page renderer pass the plain corpus at 40, 80, and unlimited columns, without ESC bytes, trailing whitespace, URL splitting, or width-sized allocation for unlimited output.
- Compose/render data matches the specified envelope. Every command has a schema and a real-command strict conformance test. JSON5, YAML, and TOML preserve the command's JSON data subject to the documented TOML null/root rules.
- Current topic/CLI documentation, shared rendering documentation, skills, affected comments, and `just/review.just` describe the shipped behavior. Appropriate L1, real-terminal, headless-browser, compile, and lint checks pass; evidence distinguishes executed tests from unavailable capabilities.

Repository paths below are relative to the starting worktree. Keep the existing unrelated edit in `darkmatter/features/2026-11-04-loading-data/spec.md` untouched. Do not commit, create a branch/worktree, run `cargo fmt`, move the fix, or invoke `just complete`.

## Phase 1: Shared Output and Width Foundations

### Necessary Rules

These decisions settle the specification's remaining technical details. They apply to every later phase.

1. **Module ownership.** Add `renderable/src/output.rs`, publicly re-exporting `CommandOutput`, `OutputOptions`, `TerminalProfile`, and `Width` from the crate root. Put dispatch, `OutputTarget`, `OutputError`, and width resolution in `biscuit-terminal/lib/src/output/`. `Width` here means `renderable::output::Width`, not the existing component-sizing `renderable::layout::Width`; import the latter as `LayoutWidth` in files using both. Evidence: `renderable/src/lib.rs`, `layout.rs`, `color/capability.rs`, and the spec's exact trait and dispatch signatures. Keep `renderable` independent of biscuit-terminal and Darkmatter.

2. **Exact shared contract.** Use the spec's `CommandOutput` signatures unchanged: associated serializable `Data`, `as_prose() -> renderable::Document`, `as_data() -> Self::Data`, and `render_terminal`, `render_markdown`, `render_markdown_plus`, `render_html`, each taking `&OutputOptions` and returning `Option<String>`. `None` invokes the dispatch fallback. `OutputOptions` has `verbosity: u8`, `width: Width`, and `terminal_profile: TerminalProfile`. Use `Width::{Columns(u32), Unlimited, Source}`; reject zero before constructing `Columns`. Fallible document/page preparation occurs before constructing a command output, so overrides never swallow rendering errors. Evidence: spec “The trait”; the page/artifact builders currently return `Result` in `darkmatter/cli/src/artifact.rs`.

3. **Canonical plain profile.** Choose `TerminalProfile::plain()`. Store target-neutral color depth/mode, foreground/background, TTY state, OSC 8, italics, underline, and graphics capabilities in that profile; keep Unicode enabled because plain tables retain box drawing. Add conversion helpers between a detected `Terminal` and the profile in biscuit-terminal. Apply plain normalization after all options/style resolution, including code/diagram hooks. Retain `HyperlinkMode` and implement `Never` by disabling OSC 8; no mode can restore capabilities removed by plain. Remove `ImagePlaceholder` and its two variants, updating every caller to the single specified image fallback. Evidence: `render_tree/options.rs`, Darkmatter `render_tree/entrypoints.rs`, and spike defects P2–P4/P10.

4. **Width without sentinels.** Carry `Width` through terminal tree contexts, available-width calculations, Darkmatter `TerminalOptions`, page state, and `TerminalCodeContext`; remove `u16` clamps on these rendering paths. Keep physical terminal measurement numeric, adding an optional detected-column accessor rather than changing unrelated device-query APIs. `Source` preserves existing soft breaks and does not imply a fill width; `Unlimited` joins paragraph soft breaks and never wraps/fills. Component `max-width` still caps a component, including under unlimited. Evidence: `biscuit-terminal/lib/src/terminal.rs`, `render_tree/options.rs`, `renderable/src/color/capability.rs`, `darkmatter/lib/src/layout/page.rs`, and the plain spike's measured 300 KB horizontal rule. No new performance spike is necessary.

5. **Width parsing and diagnostics.** Add `WidthRequest::{Auto, Source, Unlimited, Columns(u32), Percent(f64)}` and `resolve_width(request, output_columns, terminal_columns) -> Result<ResolvedWidth, WidthError>` in biscuit-terminal; `ResolvedWidth` carries `width` and warning text. A finite positive numeric argument floors to columns; a fractional argument produces `dm.output.fractional_width`. Reject a floored zero, negative/non-finite number, malformed percentage, and values outside `u32`; reject invalid/nonpositive `OUTPUT_COLUMNS` as usage errors when the selected width needs that variable. Percentages may exceed 100; floor the product, reject a known result below one or above `u32::MAX`, and use unlimited without a base. An explicit numeric/source/unlimited value does not consult `OUTPUT_COLUMNS`. Read the captured environment once; use terminal columns only for that stream's TTY. Evidence: spec width precedence and float behavior; `Cli` currently has a rejecting width stub in `args/cli.rs`. Stderr resolution ignores the deliverable's `--width` and `OUTPUT_COLUMNS`, using its own detected width or unlimited.

6. **No double reflow.** Give `MarkdownRenderOptions` a `width` defaulting to `Source`. Perform paragraph reflow over inline structure, keeping code spans, destinations, and autolinks atomic; leave tables/fences untouched. Darkmatter's source override uses `cleanup::reflow::reflow_to_width` for columns, paragraph unwrapping for unlimited, and unchanged source for source. An explicit width overrides `fixed-width:`/`unwrap:`; an omitted document-command width preserves today's frontmatter-driven cleanup and bytes. Resolve width once, apply it once, and reuse that Markdown for both stdout and a data envelope's `document`. Evidence: `renderable/src/tree/render/markdown.rs`, `cleanup/reflow.rs`, and spec “Markdown-output commands default to source.”

7. **Dispatch is buffered.** `write_output` renders/serializes completely before writing to its supplied writer, then writes the complete bytes. `OutputError` distinguishes render, serialization/conversion with a value path, and I/O failures. Use biscuit-file JSON5/YAML/TOML conversion facilities; recursively omit null object fields for TOML, reject null array elements with a JSON-pointer-style path, and wrap non-object roots as `{ value: ... }`, arrays included. Preserve heterogeneous array data rather than stringify it. Use `biscuit_file::Json5::as_toml` after the recursive null/root preprocessing; its serde-to-TOML conversion retains heterogeneous arrays, unlike the YAML conversion's default reject/stringify policy. A serialization failure becomes the command's failure payload before stdout is touched. A broken output writer returns an I/O failure without attempting another payload or printing data-mode stderr. Evidence: spec TOML rules, `frontmatter.rs::format_value`, and biscuit-file's conversion policies. No second serializer in command handlers.

8. **Library output values.** Add `darkmatter/lib/src/markdown/output/command/` for document output, typed command data, and report adapters; reuse existing domain types for TOC/delta/file-tree/schema/hash data. CLI code supplies invocation metadata and already-resolved page/theme configuration. Move the clean wire DTOs and `FindingRow` projection into this library output module and re-export them from their old CLI modules during migration; keep the v1 serialized fields unchanged. Do not pass `Cli` into the library. A prepared `DocumentOutput` stores the Markdown source, tree, page configuration, and validated fallible rendering inputs; it implements the three document overrides prescribed by the spec, with generic MarkdownPlus. Evidence: `artifact.rs::{html_page,markdown_artifact,markdown_plus_artifact}` and `commands/render.rs`; the trait cannot import CLI-specific types.

9. **Preserve coordinate frames.** Add `ReferenceOrigin.authored: Option<AuthoredReferenceLocation>` with `line: usize`, `column: usize`, `span: Range<usize>`, and `inserted: bool`; add `section: Option<String>` separately so an in-memory record still has a section. Keep `origin.line`, `origin.span`, and `ReferenceInsertion.directive_line` in prepared space. Add `authored_directive_line: Option<usize>` to insertions and `included_via: Vec<ReferenceTrailStep>` to issues, with `ReferenceTrailStep { file: ComposeSource, line: Option<usize> }`. Evidence: `file_tree/model.rs` matches prepared directive and origin lines; `reference/types.rs`, `reference/validate.rs`, and spike A1–A4. This is an intentional public API addition; update all struct literals across the workspace.

10. **Producer provenance is explicit.** Retain `body.origin` as `ComposeReport.body_origin` (`pub(crate)`). Replace `prepare_content`'s growing tuple with private `PreparedReferenceContent { content, body_data, body_origin, warnings }`. Add `BodyOrigin::producer_of(offset) -> Option<Range<usize>>`, backed by producer segments recorded while applying edits and carried through later edits/splices. Resolve each edit's replaced range through the pre-edit origin, inheriting the original producer for an edit inside produced data. Do not infer producers only from neighboring gaps: adjacent replacements and replacements at either body edge have no unique gap. Project the reference's first Unicode character for authored position; use the whole authored range for `span` only when it projects, otherwise that first character; inserted spans name the producing construct. Evidence: `compose/body_origin.rs::{after_edits,after_splices,project}` currently drops every replaced range and stores only copied segments. Add edge/adjacent/chained producer tests beyond the spike's three basic cases. No new hashing implementation; retain biscuit-hash-backed map fingerprints. A captured map with unknown producer provenance remains unlocated; a known file identity alone never proves a source range.

11. **Prepared headings are safe.** Use `markdown::toc::extract_headings(&prepared)` to obtain `heading_span.start`, final duplicate-numbered `slug`, title, and level; retain these in `PreparedHeadingSnapshot` along with slug lookup compatibility. Section lookup selects the last heading start at or before the reference byte offset. `Markdown::new` does not parse frontmatter, so the suspected leading-`---` misparse is absent; keep a regression fixture. Evidence: `Markdown::new` in `markdown/mod.rs` initializes empty frontmatter directly; `toc/mod.rs::extract_headings` is the existing slug/span authority. Removed blocks never enter the prepared heading set.

12. **Every extractor is projected.** Project records only after CSS spans have been rebased to body offsets; change CSS extractor inputs to accept the style-content byte base and include the construct opener in the span. Locate `prologue:`/`epilogue:` values through the existing frontmatter leaf-location/projection machinery, never line 0 or `usize::MAX` in public diagnostics. File-backed preparation must retain a describing origin map, guarded by `debug_assert`; release fallback is file-only with null positions rather than fabricated numbers. Remote documents use their URL as source identity and null authored-file coordinates. Evidence: `reference/css.rs`, `graph.rs` frontmatter/CSS extraction, `markdown/hash/write.rs::locate_frontmatter_leaves`, and the spike. `ReferenceRecord` serialization remains prepared-space graph data; issue/report serialization switches to authored positions.

13. **Trail and counts follow occurrences.** Accumulate trails during the validation walk, outermost first, retaining separate issues for repeated inclusion paths. Retain report document metadata from graph traversal, including empty/error-free documents, so grouping and totals do not infer scope from issues. Document count is distinct graph document identity; reference totals retain today's traversal-occurrence counting. Group visible issues by `(source, inclusion trail)` in depth-first insertion order, root first; repeated copies of a child receive separate groups with their own trails. Evidence: `reference/validate.rs` walk and the spike's `twice.md` result already emit issues per path; `ReferenceValidationReport` currently lacks document metadata.

14. **One reference view.** Retain and extend public `ValidationReportView`, adding root source, captured launch directory, and `ReferenceReportVisibility::{Errors, All}` builder inputs. Implement `CommandOutput` using a structural Document built from Prose and borderless three-column Table projections. Compose/graph use Errors; validate refs uses All. Share `ReferenceIssueCode::reason_phrase()` and `ReferenceSyntax::construct_phrase()` with diagnostic production. Remove `validation_category_label` and `validation_order`: the repository search found their sole consumer in the old view. Remaining reasons are `malformed HTML tag`, `malformed CSS import`, and `malformed meta tag`; remaining construct phrases are `in <link>`, `in <script>`, `in <style>`, `in CSS @import`, `in CSS @font-face`, and `in <meta>`. Evidence: exhaustive enums in `reference/types.rs` and `validate.rs` contain those variants beyond the spec's abbreviated tables.

15. **Report detail and ordering.** Use the specified reason/construct tables, visible filename and `:line:column`, full relative group paths, raw references as code, and dim details only for extra information. Tally displayed error reasons descending by count, breaking ties by first occurrence; append warning/info counts separately for All visibility. Count all error-severity issues as broken; warnings/info do not change validity. With no errors, print the all-ok summary, followed by warning/info counts for All visibility. Omit the reason tally for zero/one distinct error reason. In-memory rows show their source name without numeric suffix; remote rows show the URL. Evidence: spec report grouping, severity, success, and summary rules; `ReferenceValidationReport::is_valid` judges errors only.

16. **Portable location links.** Build absolute file URIs with `url::Url::from_file_path` and display paths with `biscuit_file::to_portable_string`; derive relative paths from the captured launch context with repository path utilities, never fresh CWD. A `qp` link's terminal destination loses query/fragment and carries safe parameters; ordinary links retain their destinations. Write `line`, `column`, `heading` first in that order, then other parameters sorted by key. HTML writes heading first, then line/column, then remaining keys, preserving existing unrelated query pairs; metadata wins colliding keys. Do not emit duplicate heading keys. Evidence: `biscuit-terminal/render_tree/link.rs` already uses `Url::from_file_path`; spec location links require these three facts on every OS.

17. **One link-title interpretation.** Introduce a target-neutral structured-title parser in `renderable::output::link_metadata` used by both Prose and Darkmatter; it returns structured metadata, plain title, malformed-object notice, or legacy notice. JSON5 object titles replace key=value attributes. Supported existing properties keep their string interpretation; unknown keys are ignored. `qp` is a map of string, finite number, or boolean values, normalized to strings for query/OSC transport and retained as original scalar data for Markdown round-trip; null/array/object entries and a non-object `qp` are ignored with `dm.link.malformed_structured_title`. A non-object valid JSON5 title is plain text. For Link, keep legacy detection and parsing solely for migration and warnings. Leave `render/image_ref.rs` and its existing image-title syntax unchanged; it also uses `reference_parse`, so keep the utility functions its parser still calls. Evidence: `render/link.rs::with_title_parsed`, `apply_structured_prop`, `render_tree/build_context.rs::parse_link_directive`, and the separate Prose reference grammar. Add `json-five = "0.3"` to renderable, matching biscuit-file (Cargo.lock currently resolves 0.3.1); update dependency docs. This avoids making renderable depend on biscuit-file or duplicating title semantics.

18. **Typed link metadata.** Add a typed link-metadata field to tree node attributes carrying query parameters; do not hide transport metadata in browser `data-*`. Populate it from Darkmatter `Link` and Prose projections; teach browser/Markdown/terminal folds to consume it. `Link::to_html` and terminal convenience methods use the same metadata lowering helpers as the tree path. Keep the existing encoded Markdown metadata-package decoder for round-trip compatibility, but emit the new JSON5 title form. Preserve old title/data/class/style/target/prompt values during cleanup using the old parser, then emit JSON5 and prove a second clean is unchanged. Evidence: `render/link.rs::{decode_markdown_metadata,to_markdown_metadata_package}`, `tree/attrs.rs`, and current typed `LinkBrowserAttrs` lowering.

19. **Link warning ownership.** Store title parse notices on parsed links, without printing. Deliver source-located `dm.link.malformed_structured_title` and `dm.link.legacy_structured_title` warnings through composition/render diagnostic collection and the existing blocking-policy registry. Deduplicate by source span within a pass, not by text across different links. Rendering warnings must survive successful and failed data runs. Cleanup migration consumes the legacy notice after a successful rewrite. Evidence: existing `compose/report_diagnostics.rs`, `COMPOSE_DIAGNOSTIC_CODES`, `args/policy.rs`, and failed-warning preservation described by the darkmatter skill. Preserve other problem-family stopping rules.

20. **Diagnostic API migration.** Change shared `DiagnosticSource.line`/`column` to `Option<usize>`, add optional `section` and `inserted: bool`, and add optional reference details/trail to `Diagnostic`. Initialize non-reference fields with no reference details and `inserted: false`. Update every workspace constructor, DMLS projection, and Claudine consumer to preserve known numeric coordinates and handle absent positions without inventing zero. DMLS reference extraction/ranges remain unchanged. Evidence: `compose/expression/diagnostics.rs` currently requires numeric positions, while the spec requires source objects with null positions; `report_diagnostics.rs::reference_issue_diagnostic` currently drops source/span entirely.

21. **Reference failures propagate.** Change `check_references` to `Result<ReferenceCheck>` and use `?` in `run_compose` and lifecycle `ComposeHost::try_prepare`. Convert `ReferenceError` through the shared typed diagnostic path. Keep shell expansion disabled in passive reference validation. Keep `--fail-fast` on `md validate refs` as an explicit existing user override; normal compose/graph checks collect all issues. Evidence: `compose.rs::check_references` currently returns Clean on `Err`; `args/target.rs` explicitly exposes fail-fast. No extra composition or shell execution for report generation.

22. **CLI parsing ownership.** Add a reusable output/width argument builder and per-leaf command metadata in `cli/src/args/output.rs`, backed by shared OutputTarget parsing without adding clap to shared libraries. Every actual command leaf gets `--output` and `--width`, including root render, edit, and schema about; `validate`/`schema` are containers. Resolve defaults from command metadata, keeping explicitness separately from the default. Compose's accepted output list is exactly markdown/json/json5/yaml/toml. Add browser as an HTML alias elsewhere. Retain clean/compose `--compact` as list-spacing controls; restrict only get's compact serializer modifier to JSON/JSON5. Evidence: `args/{cli,render,command,target,enums}.rs` currently duplicate selectors. Do not globally reject removed flag tokens inside positional content or after `--`. The existing top-level --save alias forwards resolved output and width to clean --save; do not redesign that alias. Help/version, shell completion generation, and --list-themes remain existing utility routes outside the command-leaf inventory; accept and ignore width there, and reject an explicit --output on those utility routes as an argument conflict rather than claim a typed command result.

23. **Existing wire shapes take precedence.** Preserve the command-specific successful data shapes explicitly listed in spec “Other commands,” including get's scalar/list/null roots, clean's v1 diagnostic object, graph's tree, and validate refs' report. These explicit shapes are exceptions to the general wording that every result contains an `outcome`; adding a wrapper would violate the more specific compatibility requirement. New outputs and compose/render always carry outcome/diagnostics. Existing object shapes may add optional `outcome`/run diagnostics only on a run that needs those fields; bare get and other non-object legacy results with run diagnostics use `{ outcome, diagnostics, value }`. Failure before a legacy result exists uses `{ outcome: "failure", diagnostics, value: null }`. Existing command failure results such as clean diagnostics or a reference report remain intact and gain outcome. Define schema unions covering these variants. Record this interpretation in the implementation log and data-contract docs; no human gate follows it. Evidence: the spec explicitly mandates unchanged get values, clean diagnostics, and today's toc/delta/graph/refs JSON, while its general stream section assumes an object. This rule preserves the explicit public contracts and still delivers diagnostics in one value.

24. **New data shapes are fixed.** Use the following structs and schema names, all with required `outcome` and `diagnostics` except the explicitly preserved schema-detect success value and schema-validate container. Nullable values serialize as null in JSON/YAML/JSON5; optional `perf` is omitted without the flag.

    | Command/schema file | Result fields beyond common status/diagnostics |
    |---|---|
    | `compose.yaml`, `render.yaml` | Exact spec envelope: file, document, summary, lifecycle_output, optional perf |
    | `set.yaml` | file, property, value, document, saved; document is the resulting Markdown, including save mode |
    | `edit.yaml` | file (resolved absolute portable path), editor (selected executable), created (boolean) |
    | `hash.yaml` | mode (`compute`, `diff`, `save`, `directory`), file, kind, computed, stored, comparison, saved; computed/stored use existing tagged hash/value representations, comparison exposes existing HashComparison fields and tagged detail; missing comparison is null |
    | `code-block.yaml` | file, code, language, title, line_numbering, highlight; highlight is an ordered list of `{start,end}` inclusive line ranges |
    | `schema-triggers.yaml` | file, roots (`kind`, `state`, `path`, `duplicate_of`, `value`, `reason`), shadowed (`path`, `winner`), skipped (`source`, `code`, `message`), triggers (`source`, `matched`, `conditions` with `index`, `matched`, `defeat`) |
    | `schema-about.yaml` | sections: ordered `{id,title,description,entries}`; entries are typed `{name,description,example,constraints,details}` projected from existing descriptor catalogs; detail records are `{name,value}` strings |
    | `schema-validate.yaml` | `{files:[...]}` preserving today's per-file JSON fields/order, plus optional run outcome/diagnostics for invocation-level problems |
    | `schema-detect.yaml` | Single/merged success is today's inferred SimplifiedSchema object; multiple unmerged inputs use `{files:[{file,schema}]}` in input order; run notices/errors use the compatibility envelope in rule 23 |

    Evidence: `commands/{frontmatter,hash,code_block}.rs`, `schema/triggers.rs::trace_registry` consumption, `schema/about.rs::SchemaAboutReport`, `schema/validate.rs::FileOutcome`, and `schema/detect.rs::run_detect`. Hash directory mode keeps today's aggregate hash as computed and does not invent per-file results. Schema-about data always includes the full compiled catalog; verbosity changes its formatted presentation only. For multi-file detect's `json-schema` extra, emit `{files:[{file,schema}]}` with each schema Draft 2020-12; a single/merged result is a bare Draft 2020-12 schema. This replaces today's multiple stdout documents with one value without removing multi-file inference.

25. **Reports and graph notation.** Preserve validate refs' `--show-all` and `--graph mermaid|dot`. The latter is a graph-only early-return mode today: `commands/validate.rs::run_validate` builds a graph, prints it, and returns before validation. Preserve that observable behavior under the spec's explicit “keeps its current meaning” ruling; do not add validation to that mode. Its formatted source is a diagram Code node, and its data is `{graph:{format,source}}` with optional outcome/diagnostics under rule 23. Without --graph, All visibility uses the common report and --show-all additionally includes the requested reference listing; data adds `references` only with --show-all. Never prepend diagram text to serialized data. Graph --validate appends the same reference view body after its tree and omits its old separate success footer. Evidence: the early return in `commands/validate.rs` contradicts the spec's descriptive phrase “in addition to the report”; the explicit no-behavior-change ruling governs. Record this source-confirmed interpretation in implementation.md and current docs.

26. **Run collection and lifecycle.** Generalize `commands/compose/report.rs::ReportSink` into CLI `output/report.rs`, retaining diagnostic ordering and failed/preflight warning deduplication. Collect command/stage timings and `LifecycleOutputEntry {stage,step,text}`. Add an observer seam at lifecycle executor action/command boundaries to supply stage and step identity; aggregate a shell step's captured stdout then stderr in the existing delivery order into one entry, preserving exact text. Use a lock-protected shared collector because `LifecycleEffects` and lifecycle `md_host::CommandOutput` are Sync. Import the existing lifecycle trait as `LifecycleCommandOutput` to avoid collision with renderable's trait. Formatted lifecycle communication and shell output go to stderr; data mode only records it. Evidence: `cli/commands/compose/lifecycle.rs::TerminalEffects` currently redirects report-mode output to stderr and has no stage identity; `lib/src/lifecycle/{effects.rs,md_host.rs,executor.rs}` own delivery and execution boundaries. Do not change execution ordering, permissions, cancellation, or external effect behavior.

27. **No early exits or chatter.** Return typed status/result from handlers; main alone applies 0/1/2, with interrupt 130 outside the payload contract. Remove handler `process::exit(2|3|64)` calls for operational failures; accepted-but-conflicting arguments become clap usage errors before effects. In data mode disable CLI tracing/progress printing and capture diagnostics instead; no render/style/schema warning writes directly to stderr. `--quiet` controls formatted success rows only, never removes data file records. Evidence: `main.rs`, `commands/{hash,graph,clean}.rs`, `schema/{detect,validate}.rs`, and the spec stream/exit contract. Data get's explicit raw-success exception follows rule 23.

28. **Save/open behavior.** Preserve save operations and their existing formatted acknowledgments/no-output behavior; data outputs always return their typed result after the save. Serialize once for stdout and `--show`, using matching extensions. Complete the attempted open before final payload emission so an opener failure becomes one failure result; include the attempted artifact path in its diagnostic. Add an injectable opener seam, use recording stubs in tests, and never launch an app/editor or take focus from a test. `md edit` tests use a recorder executable as EDITOR. Evidence: `artifact.rs::open_output_artifact`, `frontmatter.rs::{run_set,run_edit}`, and spec --show acceptance.

29. **Test boundaries.** The Input Robustness Matrix does not apply: this change alters Markdown/title grammars and output writers, not a configuration/file-format loader. Test malformed title content and qp shapes as grammar behavior through public rendering/composition results; do not add cross-format configuration matrices. Output schemas are test contracts loaded through unchanged SimplifiedSchema APIs. Evidence: the task's explicit Markdown/renderer exclusion and the spec's scope. Unit/in-process tests exercise shared logic once; CLI tests prove wiring and required schemas/streams. Use test-toolkit and existing CliProcessFixture, register new modules in test roots, use native argv-recorder shell fixtures, and keep browser L2 headless and terminal tests unfocused.

30. **Scope and known assertion outcomes.** Do not repeat either completed spike or add a broader benchmark. Include A10's authored transclusion `when=` errors; it is accepted work, not optional. Preserve DMLS reference extraction, root-alias/style redesign, other CLIs' output surfaces, and non-reference `failure:` rows. The `failure: no-match` assertions in `compose_state_set.rs` test input opening, the two exact rows in `literal_glob_hint.rs` test input opening, and `error_snapshots/toc_linking.rs` constructs a TocLinkingError directly: retain those rows. Update `compose.rs`'s report-view assertion and `reference/validate.rs`'s kind-heading assertion. Evidence: the named tests' actual call paths and the spec's out-of-scope list. Update the CLI-atheist validate-refs baseline for authored lines; keep historical benchmark artifacts unchanged.

### Wave 1 — Shared contract

- [x] **Output contract**
  - Implement rules 1–2 in renderable, including profile data, output Width, exports, and exact trait methods. Add the typed tree link-metadata field and shared title parser from rules 17–18 so downstream tasks have one stable representation.
  - Update renderable dependency docs for the workspace JSON5 parser. Cover JSON5 title classification, scalar qp normalization, unknown properties, legacy notice, malformed content, and metadata serialization round-trip.
  - This wave runs serially: every subsequent task depends on these shared types.

### Wave 2 — Independent rendering foundations

- [x] **Markdown width**
  - Own `renderable/src/tree/render/markdown.rs` and Markdown/browser tree metadata lowering. Implement Source/Columns/Unlimited and lossless JSON5 titles/HTML query metadata.
  - Add width to browser fold options as well: Columns constrains prose containers in `ch` units with ordinary word wrapping, Source preserves soft breaks, and Unlimited has no viewport-derived prose constraint. Keep code overflow and tables independent of prose reflow. Pass these options through generic dispatch and Darkmatter's HTML page path.
  - Verify soft/hard breaks, atomic inline constructs, unchanged code/tables, duplicate query collisions, and links without qp. Do not modify biscuit-terminal or Darkmatter in this task.

- [x] **Terminal dispatch**
  - Own biscuit-terminal's new output module, terminal/profile bridge, terminal tree renderer/options/link helper, Prose integration, and tests. Implement rules 3–7 and safe OSC 8 metadata. Route non-clickable links/images through one fallback and keep URL layout atoms unbreakable through outer Prose wrapping.
  - Cover generic dispatch and terminal overrides, plain degradation after option selection, serialization errors before writes, root wrapping/null rules, all width parsing outcomes, natural-width components, and styled-code truncation versus plain fences.
  - Update compilation callers of terminal context fields in the same task; coordinate shared renderable edits through the Phase 1 contract, not competing edits to its Markdown fold.
  - Extend Prose's `markdown.rs::Lifted::Destination` and link parsing in `tokens.rs` to retain a separately parsed title and typed query metadata. Today the lifted destination stores only href/source; do not put an entire destination-plus-title into an OSC URI.

### Wave 3 — Foundation checkpoint

- [x] **Foundation verification**
  - Integrate Wave 2 and run `just test` and `just lint` from the renderable and biscuit-terminal areas. Build the workspace with `cargo check --workspace --all-targets --color=never` to find and migrate public context/signature callers, including examples and benches.
  - Run dispatch tests proving a terminal override also serves plain; test all formatted fallbacks and data serializations. Prove unlimited has natural-size output rather than a numeric fill sentinel.
  - Pass the shared checks before beginning Phase 2. Fix failures within scope and keep behavior-changing comments synchronized.

## Phase 2: Authored Reference Data and Common Report

### Wave 4 — Independent provenance and link work

- [ ] **Reference provenance**
  - Own `compose/body_origin.rs`, `compose/context/report.rs`, pipeline handoff, `reference/{types,graph,snapshot,css}.rs`, and authored `when=` projection. Implement rules 9–12 and the producer map, including every extractor's constructors.
  - Reconstruct the spike fixtures under `lib/tests/fixtures/composition_errors/`. Pin root positions 9:5, 15:9, 22:13, 24:8, 28:1; directive line 26; child positions 7:9, 7:43, 11:1 and inserted 9:7. Add LF/CRLF/BOM, same-line/edge/adjacent producers, mixed authored/interpolated labels, CSS rebasing, frontmatter values, leading rule, duplicate/interpolated/removed headings, and in-memory tests.
  - Preserve prepared coordinates and verify existing file-tree matching and fragment snapshot freshness tests.

- [ ] **Link migration**
  - Own `render/link.rs`, `markdown/render_tree/build_context.rs`, cleanup title migration, warning production, and link tests. Consume Phase 1's shared title classifier/metadata rather than adding another parser.
  - Preserve existing structured attributes and encoded package round-trips; migrate key=value titles using source spans so code fences and unrelated strings stay intact. Prove semantic parity and idempotence. Cover HTML query mapping, OSC 8 safe-parameter omission, Windows file URI text, and malformed/legacy warnings through the public result.
  - Wire the new warning codes into existing diagnostic and policy catalogs; leave Phase 2's reference-report files to the next wave.

### Wave 5 — Report integration

- [ ] **Report data**
  - Own `reference/validate.rs` and its new report builder module. Add graph document metadata, authored issue serialization, per-path trail traversal, and reason/construct phrase methods; replace the old grouped-by-kind rendering with ValidationReportView's structural output.
  - Cover two equal basenames, two-level/repeated-child trails, root/remote/in-memory groups, all reason/syntax variants, error/warning/info visibility, mixed/single reason tallies, detail rows, singular grammar, empty/all-ok summary, and identical common report bodies for graph and validate refs.

- [ ] **Diagnostic projection**
  - Own `compose/expression/diagnostics.rs`, `compose/report_diagnostics.rs`, workspace diagnostic constructors/consumers, and the `check_references` Result migration in compose/lifecycle. Apply rules 20–21 and add authored source/span/reference details without dropping earlier warnings.
  - Keep DMLS's existing raw-reference extraction and protocol ranges intact. Test public diagnostics for reference, schema, expression, shell, lifecycle, and transclusion failures, including null positions and inserted provenance.
  - Run the `check_references` regression against a preparation error that makes `validate_references` return Err, then assert failure through both direct compose and lifecycle paths.

### Wave 6 — Reference checkpoint

- [ ] **Reference verification**
  - Integrate Wave 5, run `just test` in darkmatter, and run the affected shared-library tests. Update the report-view snapshots and authored-line baseline identified in rule 30; retain the untouched error surfaces' assertions.
  - Verify no reference report contains `is not valid`, old category headings, or a `failure:` row; diagnostic data retains the failure class. Verify reported spans index the loaded source and sections match the prepared heading snapshot.
  - Complete this checkpoint before changing CLI output orchestration.

## Phase 3: Darkmatter Rendering and Compose/Render Output

### Wave 7 — Independent page and run-output work

- [ ] **Document rendering**
  - Own `markdown/output/command/` document adapter, `markdown/output/terminal.rs`, `markdown/render_tree/entrypoints.rs`, `layout/page.rs`, and page tests. Carry resolved width/profile/verbosity through the default page path and framed page path.
  - Fix P1–P10 in the generic and Darkmatter paths, including ignored HyperlinkMode, ambient-width recapture, plain trailing spaces, natural-width code bands/tables/rules, and fixed-width/unwrap precedence. Keep Darkmatter's theme/style state on its document output value.
  - Verify corpus at 40/80/unlimited with/without frame and with/without max-width, through Markdown::as_terminal and DarkmatterPage. Snapshot styled and plain results as review artifacts; no manual visual gate.

- [ ] **Run collection**
  - Own new `cli/src/output/{mod,report}.rs`, lifecycle observer/capture, perf collection, and artifact opener seam. Generalize ReportSink without changing diagnostic ordering or executing composition twice.
  - Implement rules 23, 26–28, the exact compose/render envelope, and invocation-local error/status handling. Cover preflight failure warnings, denied warnings, errors before input load, lifecycle command/communication ordering, perf on/off, and serialization/open failures.
  - Keep application writes out of generic dispatch; handlers deliver values and main finalizes them. Add stdout/stderr ownership to the collector so a data-mode error never falls back to main's human renderer.

### Wave 8 — Compose/render wiring

- [ ] **Compose outputs**
  - Own `args/output.rs` foundations and compose's selector/width argument, `commands/compose.rs`, lifecycle integration, and compose tests. Replace report-json with all four data formats and use the new report collector.
  - Default Markdown remains the direct output path with unchanged bytes absent an explicit width. Reject terminal/plain/markdown-plus/html/browser/auto/ast/text/report-json with usage status 2. Emit failures as status 1, including reference validation Err, denied warnings, lifecycle failure, and baseline load failure.
  - Capture lifecycle/shell output in data mode and route it to stderr in Markdown mode. Preserve interruption 130 with no object. Serialize once for `--show`; stub open failures and assert a single valid failure payload.

- [ ] **Render outputs**
  - Own `commands/render.rs`, artifact document adapters, render/root-render selectors, and render tests. Use the library DocumentOutput for all formatted targets and the shared envelope for all data targets.
  - Replace reject_width_flag with actual width resolution, retain max-width as a content cap, and keep root-render behavior aligned for the flags in this change without implementing the separate root-alias redesign.
  - Cover terminal-to-plain byte parity, HTML/style warning collection, Markdown source output, MarkdownPlus disclosure output, data document/source parity, frontmatter, and show extensions/content.
  - Run concurrently with Compose outputs after the shared args/output.rs contract is integrated; give that file one owner and expose the needed constructors before splitting leaf wiring.

### Wave 9 — Compose/render checkpoint

- [ ] **Envelope contracts**
  - Add `cli/schemas/output/{compose,render}.yaml` with the spec's exact fields and strict real-command conformance cases for success, failure, and warnings. Use SimplifiedSchema::validate_strict through the crate's existing public API.
  - Test JSON/JSON5/YAML/TOML parity, null omission, perf presence, lifecycle ordering, width warnings, and empty data stderr with verbose/perf enabled. Test formatted diagnostics at independently spoofed stderr width.
  - Freeze byte controls equivalent to homelab/claudine compose recipes, including --frontmatter, and prove both TTY-profile and piped defaults match them. Run `just test` in darkmatter and applicable shared suites before Phase 4.

## Phase 4: Every Remaining Command and Schema

### Wave 10 — Shared command wiring

- [ ] **Leaf arguments**
  - Extend args/output.rs metadata to every command leaf with the defaults from the spec; add width everywhere and the schema-detect json-schema extra. Remove obsolete output enums/fields, get aliases/raw, clean fixed-width/incidental-newline flags, and output-auto/ast/text values.
  - Own `args/{cli,command,target,enums,render,completion,parsers}.rs`, command dispatch signatures, and reusable output argument helpers. Keep graph notation, fail-fast, show-all, save, list compact/loose, and hash semantics.
  - Add parser tests per leaf for families, default help, alias browser, width, removed flags/values, compact restrictions, and accepted-but-invalid conflicts. Use table-driven metadata instead of independent command-specific acceptance lists.

### Wave 11 — Parallel command groups

- [ ] **Document commands**
  - Own CLI `commands/{clean,frontmatter}.rs`, `commands/clean/json.rs`, corresponding library adapters, schemas `clean/get/set/rm/edit.yaml`, and conformance tests.
  - Preserve clean's v1 diagnostics and get's exact successful data shape/raw plain bytes; render clean/set formatted outputs through DocumentOutput. Preserve save side effects and add data acknowledgments. Stub edit's editor and verify created/existing file behavior.
  - Implement explicit-width cleanup precedence and legacy-title migration; emit no format-specific printing directly from handlers.

- [ ] **Reference commands**
  - Own `commands/{graph,validate}.rs`, toc/delta branches currently in commands/mod.rs, their library output adapters, schemas `graph/validate-refs/toc/delta.yaml`, and conformance tests. Move toc/delta leaf implementations into command modules to avoid concurrent edits to the dispatch match.
  - Preserve existing graph/TOC/delta JSON values, use the common reference report, and support both output families. Integrate show-all and auxiliary graph data per rule 25. Return status 1 for failed validation.
  - Delete print_validation_report_text and graph's handwritten summary footer. Test graph --validate and validate refs report-body parity without weakening their different severity visibility settings.

- [ ] **Schema commands**
  - Own `commands/schema/{validate,detect,triggers,about}.rs`, corresponding adapters, schemas `schema-validate/schema-detect/schema-triggers/schema-about.yaml`, and conformance tests.
  - Collect validate file results before dispatch; preserve per-file fields and diagnostic distinctions while normalizing operational exits to 1 and usage conflicts to 2. Preserve YAML versus Markdown selection behavior and keep schema-about passive.
  - Implement rule 24's new trigger/about projections and multi-file detect shapes; render detect as a YAML Code node for formatted output, bare inferred schema for single/merged data, and Draft 2020-12 only for json-schema. Cover `--quiet`, merge/unmerged inputs, and no-prose width invariance.

### Wave 12 — Hash and code

- [ ] **Hash results**
  - Own commands/hash.rs, typed library hash output projections, hash.yaml, and conformance tests. Load the biscuit-hash skill before editing hash functionality; preserve all hash computations, options, stored-ignore comparison, directory aggregate, and save behavior.
  - Emit rule 24's fields, retain existing formatted hash bytes, and return status 1 for missing stored hash or a difference. Width is accepted and leaves hash results unchanged. Test compute/diff/save/directory, including a detailed hash containing heterogeneous section tuples under TOML.

- [ ] **Code results**
  - Own commands/code_block.rs, the CodeBlock output adapter, code-block.yaml, and conformance tests. Preserve file/content resolution, language/theme/title/number/highlight behavior while replacing its three-way private renderer with shared dispatch.
  - Test both families, plain fences and long-line overflow, styled truncation, data fields, show artifact handling, and invalid argument versus operational failure status. Use native fixtures, never shell-dependent test commands.

### Wave 13 — Command checkpoint

- [ ] **Contract inventory**
  - Add one guard test enumerating clap leaf commands and their data-family acceptance; assert a matching schema file and registered executable conformance cases. Expected leaf schemas are compose, render, clean, toc, delta, get, set, rm, edit, validate-refs, hash, schema-validate, schema-detect, schema-triggers, schema-about, and code-block.
  - Register tests in existing CLI L1 roots and use the established process fixtures. Every leaf gets a control, runtime failure, and warning case for the warnings it can produce; all leaves get width acceptance and help/default tests. Runtime-failure payloads follow the settled compatibility rules, including scalar get and multi-file detect.
  - Test each serializer/dispatcher branch once at its public API, then prove command wiring. Run complete darkmatter L1, shared-library L1, and workspace all-target compile after integration.

## Phase 5: Documentation, Skills, and Drift Closure

### Wave 14 — Parallel documentation owners

- [ ] **CLI documentation**
  - Own darkmatter/docs/cli, both Darkmatter READMEs, and `just/review.just`. Add `docs/cli/output-and-width.md`; rename/rewrite report-json.md to `data-contracts.md` and update every live link. Keep READMEs as overviews linking to the references.
  - Update every command page; add pages for validate/schema/code-block/edit where references are absent. Document accepted families/defaults/extras, width, stream/exit behavior, legacy success-shape exceptions, multiple schema detection results, null/TOML errors, save output, show artifacts, and removed spellings with examples.
  - Replace all nine review.just get --raw calls with --output plain. Preserve compose recipes in homelab and claudine unchanged.

- [ ] **Topic documentation**
  - Own Darkmatter topic/inline/struct/error pages and Darkmatter skill topic files. Update hyperlinking/Link/DSL examples for JSON5, qp transport, malformed/legacy notices and clean migration; explain authored coordinates, inserted provenance, sections and trails, and unchanged family stopping rules.
  - Update lifecycle Output order, schemas' old output flags and exits, compose request/expression examples, cleanup rules, and file-reference-failures' obsolete reference-report row. Retain non-reference failure-row contracts.
  - Apply the complete file inventory in spec “Documentation,” including schema definition/DMLS support, policy, inline schema validation/call-check/map/filter/preflight-checks, and source comments in simplified-schema, Claudine, and DMLS. Live docs must never link to this fix.

- [ ] **Shared guidance**
  - Own renderable/biscuit-terminal docs and skills, `.claude/skills/cli/SKILL.md`, and docs/cli-best-practices.md §2.1. Document two sources, families, overrides, the canonical profile, Width versus layout Width, intrinsic unlimited behavior, code/link/image fallbacks, OSC parameters, and Markdown reflow.
  - Update biscuit-terminal/docs/components/prose.md graceful degradation. Update renderable and dependency docs for changed APIs/dependencies. Record the root-alias principle while keeping its Darkmatter implementation assigned to the separate render-surface work.

### Wave 15 — Drift checkpoint

- [ ] **Consistency audit**
  - Search active code/docs/skills/just files for removed flags/values, old reference headings/wording, obsolete exit 2/3/64 claims, report-json links, and structured key=value examples. Keep parser rejection tests, migration fixtures, historical specs, benchmark records, and unchanged failure surfaces intact.
  - Confirm comments describe each changed symbol's behavior, especially prepared versus authored spans, plain capability pinning, lifecycle streams, and schema-detect multi-file output. Update mismatches in the owning change; do not perform adjacent comment cleanup.
  - Add `implementation.md` beside this plan with completed tasks, actual validation evidence, produced visual artifacts, and the explicit compatibility interpretation from rule 23. Leave spec.md as its dated snapshot.

## Phase 6: Integrated Verification and Review Artifacts

### Wave 16 — Final automated validation

- [ ] **Behavior verification**
  - Run `just test` and `just lint` from each of renderable, biscuit-terminal, and darkmatter. Run `cargo check --workspace --all-targets --color=never` for public API migration coverage. Use nextest through recipes, never cargo test; do not invoke cargo fmt.
  - Run `just test-l2` for biscuit-terminal and darkmatter with harness settings that never activate windows; run `just test-browser` for both areas with headless protocol input. Cover actual OSC 8 bytes, no-color/plain profile, page width/frame behavior, HTML query attributes, and disclosure/code rendering. Follow rust-testing/biscuit-test-harness skills and record missing capability evidence rather than claiming skipped tests passed.
  - Run `just doctest` for changed public API docs in renderable, biscuit-terminal, and darkmatter. Re-run only suites affected by subsequent fixes.

- [ ] **Portable verification**
  - Exercise portable path/URI helpers with Windows-drive, Unicode, escaped-space, LF/CRLF/BOM fixtures through public results. Keep native eager file values distinct from portable display strings.
  - Use the OS skill's configured BUILD_LINUX/BUILD_WIN/BUILD_WSL hosts and canonical cross-check recipes to gather required behavioral evidence, reusing qualifying receipts. Execute SSH only through the documented unattended BatchMode workflow. A missing destination or provisioning failure is recorded with its exact missing environment; local cross-compilation is labeled compile-only and never substitutes for native behavior.
  - Do not change CI scheduling, add speculative matrix cells, push, or contact services as part of this implementation plan. Resolve code failures exposed by configured hosts; include unavoidable environment limitations in the final evidence.

### Wave 17 — Completion audit

- [ ] **Review artifacts**
  - Save deterministic examples under the fix's `artifacts/`: root/child reference report in plain/Markdown/HTML/data, representative styled report capture, plain corpus at 40/80/unlimited, and the schema-conformance inventory. Normalize only fixture-root prefixes and timing values in review snapshots; do not erase position, trail, query, control-byte, or stream evidence.
  - Check every acceptance item in spec.md against the corresponding test/artifact and list the result in implementation.md. Explicitly verify compose recipe byte parity, null-array serialization failure paths, error-return propagation, authored positions, same-child repeated trails, no data stderr, status 0/1/2/130, and opener/editor stubs.
  - Run `git diff --check`, review the final diff for unrelated changes, and check off completed plan tasks. Report implementation complete and ready for review, with tests and any environmental evidence gaps. Leave lifecycle movement and commits to the author.

Waves are globally numbered. Only tasks in the same wave are candidates for concurrent agents, with the file ownership stated above; later waves depend on the previous wave's integrated result. Every delegated brief must repeat that the entire session tree is non-interactive, must stay in this worktree, and must not run focus-taking tests, cargo fmt, commits, or lifecycle moves. Checkpoints are automated validation tasks, never approval gates.