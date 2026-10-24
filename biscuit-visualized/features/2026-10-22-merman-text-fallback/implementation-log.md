---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-22-merman-text-fallback/spec.md
plan: biscuit-visualized/features/2026-10-22-merman-text-fallback/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - renderable
    - biscuit-terminal
    - biscuit-visualized
    - darkmatter
    - biscuit-terminal-cli
    - darkmatter-cli
decisions_for_review:
    - |-
      Swimlane is eligible for text fallback. The spec listed six eligible families (flowchart, sequence, state, class, ER, XY chart), taken from Merman's support-matrix document. Merman's own metadata (`ascii_capabilities()`) at the delivered revision lists a seventh, swimlane: diagrammatic, partial, and admitting the auto layout. The document folds it into the flowchart row. Because D1 says eligibility comes from the metadata and never from a hand-kept list, swimlane is eligible. I amended D1 and AC-F3 under D10, so the eligibility snapshot will list seven families. Alternative: exclude swimlane explicitly. That needs a family-name exclusion list, which D1 forbids. If the reviewer wants swimlane excluded, Phase 3 gains one exclusion and its snapshot loses one row.
    - |-
      Rulings A–H, which the plan left for the author, were made by the implementer, adopting each recommendation (no human was in the loop). A: the diagram hook returns a three-way outcome (drawn output, render-tree nodes, declined). B: darkmatter's Mermaid mode gains an automatic variant, which `md --mermaid` uses; the library's image mode keeps meaning image-only. C: the three fallback feature names and the "most restrictive wins" fold live in biscuit-visualized behind its `clap` feature; `md` adds them, plus `no-mermaid-rendering`, to the `--feature` enum it already has. D: clap already rejects empty and unknown names with the valid names listed (proved by a throwaway program), so no custom parser is needed. E: automatic mode chooses text at full graphics when the terminal reports no image protocol. F: drawing uses Unicode when the terminal's existing "supports Unicode" flag is set, and ASCII otherwise. G: `bt git-graph` accepts the flags with no visible effect. H: no extra measurement. Reversing A changes Phase 2's trait signature and its test doubles. Reversing B or C changes only Phase 5's CLI wiring.
    - |-
      Until Phase 4, biscuit-visualized's diagram hook declines below full graphics. The trait change forced an edit to the one real hook. The plan put that hook's automatic behavior in Phase 4, but it now receives the graphics ceiling, and left unchanged it would draw an image when only text is allowed. So for now it returns "declined" (show the source as a code block, no warning) whenever the ceiling is below full graphics. Alternatives: keep drawing images regardless (breaks the ceiling rule), or build the text route now (that is Phase 3-4 work). Phase 4 replaces the decline with the text route. Nothing else needs redoing if the reviewer prefers another interim.
    - |-
      A diagram hook's fragment is rendered with Mermaid promotion turned off. The spec says the hook may return a source fallback as a `mermaid` code node. Rendered normally in automatic mode, that node would be handed back to the hook, which would return it again, forever. The renderer therefore lays the fragment out with promotion off, so its `mermaid` node prints as code. Alternative: let hooks return a different language tag for the source, which would lose Mermaid highlighting (D8/D9 require `mermaid`). If the reviewer wants another guard, only `render_diagram_fragment` in biscuit-terminal's renderer changes.
    - |-
      A new biscuit-terminal docs page, `biscuit-terminal/docs/render-tree.md`, documents the code-renderer and diagram hooks. The plan asked to update "the biscuit-terminal render-tree topic page", but none existed; only a README section did, and READMEs are overviews in this repo. Alternative: put the content in the README, which goes against that rule. If declined, the page's content would move into the renderable doc, which already has the conceptual version.
    - |-
      The text render is available only with the crate's `terminal` feature. The spec requires the fit check to use biscuit-terminal's display-width measurement, and biscuit-terminal is a dependency only under `terminal`. The eligibility lookup needs no terminal and stays under `image`. Alternatives: measure with the `unicode-width` crate directly (the same rule biscuit-terminal uses, but a second copy of it), or make `image` depend on biscuit-terminal (a much wider build for every image-only user). Every caller of the text render (the terminal component, the Markdown hook) already has `terminal`. If the reviewer wants the text render under `image`, only the `#[cfg]` on `render_text`, `judge` and `ascii_request` and the width function change.
    - |-
      The text render takes no display-width profile; it always asks Merman for its standard (non-CJK) widths. The plan listed a display-width profile among the options, but biscuit-terminal, which measures fit, has no CJK width mode, so a CJK request would be measured differently from how it was laid out. The time budget is also not a text option: the diagram's existing `with_time_budget` applies to text requests, as the spec's "the prerequisite's configurable deadline" says. Alternative: add both options; a CJK option would need a CJK measurement in biscuit-terminal first.
    - |-
      Over-wide output is requested with Merman's "allow overflow" policy, so Merman reports the overflow and the render shows the source. The spec's AC-F2 expects a dense class or ER diagram to come back as a `relations:` report (`StructuredText`). At the delivered rev Merman returns that report for a drawable family only under its other policy, "fall back on overflow", which this code does not use. So in practice an over-wide class diagram is reported as too wide, not as a report; both show the source. The projection guard (D1) is kept and tested against Merman's real report, produced by a test with the fallback policy. Alternative: request the fallback policy. That adds a Merman error ("fallback unavailable") that would have to be mapped to "show the source", and gains nothing visible.
    - |-
      Class and ER diagrams that Merman cannot route are drawn as stacked boxes followed by a `relations:` list, and Merman still reports the result as a drawing (`Diagrammatic`). Following D1 (Merman's metadata and reported projection decide, nothing else), these are shown as text. A simple ER chain (A to B to C) is drawn with real connectors. If the reviewer finds the boxes-plus-list output not a faithful-enough drawing, excluding it would mean inspecting the output text, which D1 forbids; the cleaner fix would be upstream in Merman's reported projection.
    - |-
      The `\n` to `<br/>` label rewrite that the SVG path applies to class, ER, mindmap and requirement labels also applies to text (the spec: same label normalization). In Merman's class text a rewritten label prints as `one<br>two`; without the rewrite it prints `one\ntwo`. Neither draws two rows, so the rewrite was kept for parity. Changing it means skipping the rewrite in `render_text` only.
    - |-
      The diagram hook's request now says which Mermaid mode offered the fence (a new `mermaid_mode` field on biscuit-terminal's `DiagramRenderRequest`). Without it the hook could not tell "image only" mode from automatic mode: the spec requires image mode to stay unchanged (an image or an error), while automatic mode falls back to text on the same terminal. Alternatives: let image mode fall back to text too (changes a mode the spec says must not change), or add a second hook method per mode (a larger trait change). biscuit-terminal already carried the mode in its render context, so its library learns nothing new about Mermaid. If the reviewer prefers another shape, only the request struct, the one place the renderer fills it, and the hook's first lines change.
    - |-
      Whether a terminal can show the image is decided by a new biscuit-terminal function, `TerminalImage::route_has_pixels`, which gives exactly the answer the image display itself would give, before any image exists. The plan's ruling E said to check "image support is not None". That check disagrees with the display in one case: a Zellij pane that has not reported its cell size reports Kitty support, but the display falls back. Under ruling E the component would choose the image, fail, and show the source instead of text. Alternative: ruling E as written (simpler, wrong in that case). Reverting means replacing two calls with `image_support() != ImageSupport::None`.
    - |-
      A text drawing gets two fewer columns than its code block (`TEXT_BLOCK_CHROME_COLUMNS` = 2). The spec says to subtract the `txt` code block's padding and borders, but that padding belongs to whichever code renderer the program installs: the plain code block has none, and darkmatter's highlighter pads one column on the left. The hook cannot see which renderer is installed. Reserving two columns keeps a fitted drawing inside a padded, highlighted panel. Alternatives: zero (a drawing at the exact width overflows a highlighted panel by one column), or have the code renderer report its padding (a new method on a shared trait). Changing it means changing one constant and two width-boundary tests.
    - |-
      The component's source fallback is now a rendered `mermaid` code block (the spec's code node), so its exact text changed. The plain renderer dims the fence when the terminal has color and drops the source's trailing newline. Before, the component printed the raw ```` ```mermaid ```` fence string from `fallback_code_block()`, which is unchanged and still public. Three existing component tests now compare the output with its escape codes removed. Alternative: keep printing the raw string on the source route. That contradicts D9 and would leave `bt` unable to highlight the source in Phase 5.
    - |-
      An explicit component title is shown as a bold line above the text drawing or source block, unless a line of the source already gives that title (`title X`, `title: X` in frontmatter, or `pie title X`). The spec asks for the title once and not duplicated, but does not say how. This is a plain text check, used only for the title, never for eligibility. Alternative: ask Merman's parsed model for the diagram's title. That would add a parse to the source route, and not every family exposes a title the same way. Changing it touches only `source_gives_title` and its test.
    - |-
      `TerminalDiagramRenderer` changed from a unit struct to a struct holding the fallback policy, so it is now built with `TerminalDiagramRenderer::default()`. The one construction outside biscuit-visualized (darkmatter's `entrypoints.rs`) was updated in this phase so the workspace compiles. Phase 5 installs the policy folded from `md`'s `--feature` flags there.
    - |-
      The Mermaid highlighting syntax is kept in a small syntax set of its own instead of being added to the large set darkmatter already uses for every other language. Adding it to the large set means rebuilding that set at startup, which measured about 0.45 s (debug build) for every process that highlights code; the separate set costs about 6 ms. The spec said to "add it to darkmatter's syntax set". Consequence: the vendored syntax cannot borrow other languages, so JavaScript inside `%%{init}%%`, HTML in labels and YAML in `---` blocks get only their string/comment color, and any code that highlights a Mermaid block must ask darkmatter for the set that owns the grammar (`CodeHighlighter::syntax_set_for`). Alternative: merge into the large set (full embedded-language highlighting, about half a second slower per process). Reverting touches `grammars.rs`, three call sites in `code_block.rs`, and the syntax file.
    - |-
      The vendored Mermaid syntax is not a pure flattening. Upstream highlights only flowchart and sequence headers, so a header rule was added for every other diagram family (otherwise a gantt header, which AC-F12 requires, would not highlight). Upstream's `%%{init}%%` rule and twelve node-shape rules combine two syntax features that syntect misreads, so node text never ended; they were rewritten. Every edit is listed in `SOURCE.md` beside the file. Alternative: ship upstream as-is and accept the broken node highlighting and unhighlighted headers.
    - |-
      `md --feature no-mermaid-rendering` reuses darkmatter's existing "text" Mermaid mode (code in the terminal, code and no Mermaid.js in HTML) rather than adding a new mode. Markdown and JSON output already ignore Mermaid settings, and `md` has no static-SVG flag, so "wins over static SVG" holds by construction. The terminal path also never applied `--feature` before this phase (only HTML did); it does now. Alternative: a dedicated mode value; nothing would need redoing beyond the mapping.
    - |-
      The fallback policy reaches darkmatter's Markdown hook through a new public field on darkmatter's terminal options and a `DarkmatterPage::with_mermaid_fallback_policy` builder. Alternative: a separate Mermaid options type. Changing it touches the page builder, the options struct and the one place the hook is installed.
    - |-
      The plan named `bt mermaid` and `bt xy-chart`, which do not exist. `bt`'s nine Mermaid subcommands are `flowchart`, `state-diagram`, `erd`, `pie-chart`, `quadrant`, `timeline`, `bar-chart`, `line-chart` and `git-graph`; all nine take `--feature`, and the CLI tests the plan set for `bt mermaid` run against `bt flowchart`. No generic `bt mermaid` command was added (that would be new scope).
    - |-
      In `bt`, on a terminal that cannot show images, a diagram whose source does not parse now prints its source and exits successfully; before, it exited with an error whether or not images were available. This follows the spec's selection rule (a failure on the text route shows the source). With images available, a failure is still an error, and `--meta`/`--debug` still report only image renders. Keeping the error without images would need a second Merman request (the spec allows one) or loading the system fonts (seconds). If the reviewer wants a non-zero exit, `bt` would need the component to report why it chose the source.
    - |-
      `bt` uses darkmatter's deprecated `TerminalCodeRenderer` (the raw code-renderer hook) once, with a local `#[allow(deprecated)]`, as its deprecation note permits; it is built the same way darkmatter's public `CodeBlock` builds it. The component's `text_columns` became public so `bt` can lay the nodes out at exactly the component's width. Alternatives: a non-deprecated public constructor in darkmatter, or a component method that takes a code renderer (which would make `text_columns` unnecessary). `bt`'s highlighted blocks have no language header row, unlike `md`'s `CodeBlock`, because the component's code nodes ask for none.
    - |-
      Darkmatter's full test run failed on 14 tests that store JSON output (41 snapshots and 8 expected-output files for schema conversion and `md schema validate --format json`). None is in code this work touched. The stored files had alphabetically sorted keys, while darkmatter now deliberately keeps keys in insertion order (made a contract on 2026-10-23, and already forced on by Merman before that). I re-recorded them after checking that every new file parses to exactly the same JSON value as the old one, so only key order changed. Alternative: treat the order change as a regression of `md schema validate --format json` (whose test is named "byte identical") and make that output sort its keys again. That would mean reverting these 8 fixtures and adding sorting in the schema output path.
message_to_agent: |-
    Phase 5 landed darkmatter (vendored Mermaid syntax in its own syntax set, `md --mermaid` = automatic mode, `--feature` with the four names) and `bt` (`--feature` on all nine Mermaid subcommands, darkmatter highlighting); read "## Phase 5" in this log.

    For Phase 6 (docs and skills):
    (1) Phase 5 already wrote or updated: `biscuit-terminal/docs/cli.md` (new; `bt` reference incl. `--feature`, fallback flow, highlighting), `darkmatter/docs/cli/render.md`, `darkmatter/docs/rendering/mermaid.md`, `darkmatter/docs/topics/code-blocks.md`, both `dependencies.md` files, `darkmatter/cli/README.md`, `biscuit-terminal/cli/README.md`, `biscuit-terminal/README.md` (links `docs/cli.md`), `biscuit-visualized/docs/components/mermaid_diagram.md`, and `.claude/skills/darkmatter/{rendering,terminal}.md`. Review them against the spec's Documentation list rather than rewriting. The `cli` skill (D4 convention) and the `biscuit-terminal` skill (bt now depends on darkmatter; highlighting) are not yet updated.
    (2) bt's Mermaid subcommands are `flowchart`, `state-diagram`, `erd`, `pie-chart`, `quadrant`, `timeline`, `bar-chart`, `line-chart`, `git-graph` (no `bt mermaid`/`xy-chart`; the plan's names are wrong).
    (3) Highlighting a Mermaid block in darkmatter must use `CodeHighlighter::syntax_set_for(grammar)`; `syntax_set()` alone resolves Mermaid as plain text.
    (4) For Phase 7: `bt bar-chart 1 2 3` / `bt line-chart 1 2 3` show source on a no-image terminal because Merman's xychart text target needs an explicit `y-axis a --> b` range and bt's generated source has none (probed: adding `y-axis 0 --> 5` makes it draw). A drawn xychart also starts with a raw `xDomain: band categories=[bytes=1="a", ...]` line from Merman that looks like a debug artifact. Neither is promised or documented; decide whether bt should emit a y range and whether the xDomain line needs reporting upstream.
source_files_during_phase_2:
    - renderable/src/tree/mod.rs
    - biscuit-terminal/lib/src/render_tree/options.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - biscuit-terminal/lib/src/render_tree/mod.rs
    - biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs
    - biscuit-visualized/src/src/components/render_hook.rs
    - biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs
docs_updated_during_phase_2:
    - renderable/docs/tree-rendering.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-terminal/README.md
    - biscuit-terminal/lib/README.md
docs_created_during_phase_2:
    - biscuit-terminal/docs/render-tree.md
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-terminal/diagrams.md
    - .claude/skills/biscuit-visualized/diagram-components.md
    - .claude/skills/renderable/tree.md
source_files_during_phase_3:
    - biscuit-visualized/src/Cargo.toml
    - biscuit-visualized/src/src/mermaid/text.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/fonts/chain.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/text_tests.rs
docs_updated_during_phase_3:
    - biscuit-visualized/docs/dependencies.md
    - docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/mermaid-rendering.md
source_files_during_phase_4:
    - biscuit-visualized/src/src/components/fallback_policy.rs
    - biscuit-visualized/src/src/components/mermaid.rs
    - biscuit-visualized/src/src/components/mod.rs
    - biscuit-visualized/src/src/components/render_hook.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/component_fallback_tests.rs
    - biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs
    - biscuit-visualized/src/tests/l1/display_size.rs
    - biscuit-terminal/lib/src/components/terminal_image/protocol.rs
    - biscuit-terminal/lib/src/components/terminal_image/tests.rs
    - biscuit-terminal/lib/src/render_tree/options.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs
    - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
docs_updated_during_phase_4:
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-terminal/docs/render-tree.md
    - renderable/docs/tree-rendering.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/diagram-components.md
    - .claude/skills/biscuit-terminal/diagrams.md
source_files_during_phase_5:
    - Cargo.lock
    - biscuit-terminal/cli/Cargo.toml
    - biscuit-terminal/cli/src/args.rs
    - biscuit-terminal/cli/src/commands/block.rs
    - biscuit-terminal/cli/src/commands/erd.rs
    - biscuit-terminal/cli/src/commands/flowchart.rs
    - biscuit-terminal/cli/src/commands/git_graph.rs
    - biscuit-terminal/cli/src/commands/mermaid.rs
    - biscuit-terminal/cli/src/commands/pie.rs
    - biscuit-terminal/cli/src/commands/quadrant.rs
    - biscuit-terminal/cli/src/commands/quote.rs
    - biscuit-terminal/cli/src/commands/shared.rs
    - biscuit-terminal/cli/src/commands/state_diagram.rs
    - biscuit-terminal/cli/src/commands/timeline.rs
    - biscuit-terminal/cli/src/commands/xy_chart.rs
    - biscuit-terminal/cli/tests/l1/integration_test.rs
    - biscuit-terminal/cli/tests/l1/main.rs
    - biscuit-terminal/cli/tests/l1/mermaid_features.rs
    - biscuit-terminal/cli/tests/level2/diagrams.rs
    - biscuit-visualized/src/src/components/mermaid.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/args/cli.rs
    - darkmatter/cli/src/args/enums.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/render.rs
    - darkmatter/cli/tests/l1/feature_flag.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/mermaid_features.rs
    - darkmatter/lib/assets/syntaxes/mermaid/Mermaid.sublime-syntax
    - darkmatter/lib/src/layout/page.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/highlighting/grammars.rs
    - darkmatter/lib/src/markdown/highlighting/mod.rs
    - darkmatter/lib/src/markdown/language_grammar.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/output/code_block.rs
    - darkmatter/lib/src/markdown/output/terminal.rs
    - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
    - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/mermaid_automatic_mode.rs
    - darkmatter/lib/tests/l1/mermaid_syntax_highlighting.rs
    - darkmatter/lib/tests/l1/style_features_baseline.rs
    - darkmatter/lib/tests/l1/style_features_phase5.rs
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_valid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_fileref/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.json
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_string_with_pattern_and_array_constraints.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_any.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolean.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolish.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_date.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_datetime.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_email.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_enum_members.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_array_min_items.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_match.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number_integer_with_min.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_numberlike.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_object.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_min_max.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_not_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_pattern.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_time.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_scheme.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_default_and_required_atoms.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_enum_with_default_and_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_full_document_schema.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_as_union_arm.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_constrained_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_mixed_with_string_fallback.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_multi_line.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_nested.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_required_value.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_basic.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_matching_default.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_root_union_inline_only.snap
    - darkmatter/lib/tests/l1/snapshots/l1__suggest_constraint_phase3__conversion_snapshot_covers_valid_and_invalid_metadata.snap
docs_updated_during_phase_5:
    - biscuit-terminal/README.md
    - biscuit-terminal/cli/README.md
    - biscuit-terminal/docs/dependencies.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - darkmatter/cli/README.md
    - darkmatter/docs/cli/render.md
    - darkmatter/docs/dependencies.md
    - darkmatter/docs/rendering/mermaid.md
    - darkmatter/docs/topics/code-blocks.md
    - docs/dependencies.md
docs_created_during_phase_5:
    - biscuit-terminal/docs/cli.md
    - darkmatter/lib/assets/syntaxes/mermaid/SOURCE.md
    - darkmatter/lib/assets/syntaxes/mermaid/LICENSE
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/rendering.md
    - .claude/skills/darkmatter/terminal.md
    - .claude/skills/biscuit-visualized/diagram-components.md
---

# Implementation Log for 2026-10-22-merman-text-fallback (7 phases)

## Phase 1

Phase 1 changes no shipped code. It records the rulings, the prerequisite
state, and the exact API names Phases 2–5 use. Recon used a throwaway program
outside the repository (`/tmp/bv-recon-probe`, path-depending on the cargo git
checkout of the delivered Merman rev); nothing from it is committed.

### Rulings (2026-10-24, made by the implementer; see `decisions_for_review`)

- **Rule A: hook success shape.** Adopted. `DiagramRenderer::render_terminal_diagram`
  returns `Result<DiagramOutcome, DiagramRenderError>`, where `DiagramOutcome`
  is a generic `#[non_exhaustive]` enum: `Rendered(String)` (today's
  pre-formatted output), `Nodes(Vec<RenderNode>)` (a fragment laid out in
  place) and `Declined`. In-tree implementors to update:
  `biscuit-visualized/src/src/components/render_hook.rs:30`
  (`TerminalDiagramRenderer`), `biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs:52`
  (`RecordingDiagrams`) and `biscuit-terminal/lib/src/render_tree/render.rs:5567`
  (`FailingDiagrams`). Trait and request: `biscuit-terminal/lib/src/render_tree/options.rs:386-413`.
- **Rule B: darkmatter `MermaidMode`.** Adopted. Add `MermaidMode::Automatic`
  (`darkmatter/lib/src/markdown/output/terminal.rs:317`), map it to
  `TerminalMermaidMode::Automatic`, and have `md --mermaid`
  (`darkmatter/cli/src/render.rs:70-74`, today `Image`/`Off`) pass
  `Automatic`. `Image` keeps meaning image-only. The browser mappings
  (`entrypoints.rs:296`, `:604`) must also handle `Automatic`; the browser
  treats it as `Image` (static SVG at the HTML static path) or as `Off`
  (interactive). Phase 5 decides which, guided by "callers that set nothing keep
  today's behavior". `md --mermaid` today yields `Image`, which maps to
  `StaticSvg` at `:298` and `:607`, so mapping `Automatic` like `Image` keeps
  `md --mermaid --html` unchanged.
- **Rule C: where `--feature` parsing lives.** Adopted, with one adjustment
  from recon. `MermaidFallbackFeature` (clap `ValueEnum`) and the fold into the
  fallback policy live in biscuit-visualized behind its `clap` feature. `md`
  **already has** a `--feature` flag and a `Feature` `ValueEnum`
  (`darkmatter/cli/src/args/enums.rs`, `Feature::EmbedMermaidFont`), declared
  on both the implicit `md <INPUT>` form (`args/cli.rs`) and `md render`
  (`args/command.rs`). It is lowered by `apply_features`
  (`darkmatter/cli/src/render.rs:122`). This arrived in uncommitted
  working-tree changes from `2026-10-21-mermaid-embedded-font`'s review
  cycle, not from this plan. Phase 5 adds `NoMermaidFallbacks`,
  `NoMermaidPartialFallbacks`, `NoMermaidPartialWarning` and
  `NoMermaidRendering` to that enum, and maps the first three onto
  `MermaidFallbackFeature` for the shared fold. A clap `ValueEnum` cannot
  embed another enum's variants, so `md` repeats the three names. It does not
  wrap the type.
- **Rule D: empty and unknown names.** Confirmed with the throwaway program
  (clap 4, `value_enum`, `value_delimiter = ','`, `ignore_case = true`,
  `ArgAction::Append`):
    - `--feature no-mermaid-fallbacks,NO-MERMAID-PARTIAL-WARNING` → OK, both
      parsed (case-insensitive).
    - `--feature a,,b` and `--feature ""` → `ErrorKind::InvalidValue`: "a value
      is required for '--feature <FEATURE>' but none was supplied [possible
      values: …]". This is a usage error that lists the names.
    - `--feature bogus` → `InvalidValue`, "invalid value 'bogus' … [possible
      values: …]".
    - A repeated name yields a duplicate entry, which is harmless for a fold.
    - **No custom `value_parser` is needed.**
- **Rule E: image availability in Automatic mode.** Adopted. The hook trusts
  the request (no probing): `GraphicsMode::Rich` with
  `request.terminal.image_support() != ImageSupport::None`
  (`biscuit-terminal/lib/src/terminal.rs:505`, enum at
  `discovery/detection/image.rs:12`) → image. `Rich` with `ImageSupport::None`,
  or `Vector` → eligible text. `Off` → hook not called.
- **Rule F: drawing character set.** Adopted. Use the existing
  `Terminal::supports_unicode: bool` (`biscuit-terminal/lib/src/terminal.rs:309`;
  builder `TerminalBuilder::supports_unicode`). The render-tree context mirrors
  it as `TerminalRenderContext::supports_unicode` (`render_tree/options.rs:79`).
  `true` → `AsciiCharset::Unicode`, `false` → `AsciiCharset::Ascii`. Add no new
  capability. Display width:
  `biscuit_terminal::utils::block_constraint::visible_width(&str) -> u32`. It
  skips ANSI escapes and sums `unicode_width` per char (unknown → 1).
- **Rule G: `bt git-graph`.** Adopted. Recon refines its premise: `bt git-graph`
  does **not** use the typed `GitGraph` component. It builds a `gitGraph`
  Mermaid source and goes through `build_mermaid_diagram` + `display_mermaid`
  like the others (`biscuit-terminal/cli/src/commands/git_graph.rs:83-97`).
  gitGraph is `StructuredText`, so its text fallback always shows the source,
  and the flags have no visible effect. Accept and document them.
- **Rule H: wider measurement.** None.

### Task 1.1: prerequisite gate

- `2026-10-21-merman` has landed. `biscuit-visualized/src/Cargo.toml:76`:
  `merman = { git = "https://github.com/yankeeinlondon/merman.git", rev =
  "6e6b1e6554e8e75d468c1f991652c1c53a85bace", default-features = false,
  features = ["svg", "all-diagrams", "layout-cytoscape"], optional = true }`.
  The old renderer is gone (no `mermaid-rs-renderer` in the manifest).
  `src/src/mermaid/render.rs` owns the Merman requests: `renderer(&Prepared)`
  (`Renderer` with the site config), `control()` → `OperationControl::new().with_deadline(self.time_budget)`,
  `call_merman` (elapsed recording, `RenderError::Cancelled` → warning +
  `MermaidError::RenderFailed`), and `source_theme()`, which already runs Merman's
  `preprocess_diagram` + `registry().detect_type` and yields the family.
- **Delivered rev:** `6e6b1e6554e8` on the fork, branch `gitgraph-commit-spacing`.
  `git log` shows it is exactly `482fcde` plus one commit, "feat(gitgraph): add
  commitSpacing config", which touches three gitGraph render files only.
  `docs/rendering/ASCII_SUPPORT_MATRIX.md` is byte-identical between the two
  revs (`diff -q`). The spec's figures for AC-F6 and AC-F11 stand. AC-F3 was
  amended for swimlane (below).
- **Phase 3 is not blocked.**

### Task 1.2: Merman ascii API (rev `6e6b1e6`)

- **Cargo:** enable feature `"ascii"` on the existing `merman` dependency
  (`ascii = ["dep:merman-ascii"]`). The throwaway built offline with
  `["ascii", "all-diagrams", "layout-cytoscape"]`.
- **Module:** `merman::ascii` re-exports the `merman-ascii` types (listed
  below), plus `normalize_terminal_text(&str) -> Cow<str>` ("normalizes
  untrusted authored text for terminal display without changing printable
  text").
- **Capabilities:** `merman::ascii::ascii_capabilities() -> &'static [AsciiCapability]`,
  sorted by `diagram_type`. `AsciiCapability` (plain struct, public fields)
  has these fields:
    - `diagram_type: &'static str`
    - `display_name`
    - `semantic_coverage: Option<AsciiSemanticCoverage>` (`Full`/`Partial`,
      `#[non_exhaustive]`)
    - `primary_projection: AsciiPrimaryProjection` (`Diagrammatic`,
      `StructuredText`, `None`)
    - `structured_text_fallback: bool`
    - `layout_profiles: &'static [AsciiLayoutProfile]`
    - `width_profiles`
    - `encodings`
    - `support_level`
    - `supported_semantics`
    - `limits`
    - `evidence`

  It also has `supports_layout_profile(AsciiLayoutProfile) -> bool`.
  `ascii_diagrammatic_diagram_types()` is the `Diagrammatic` subset.
- **Matching the parsed family to a row:** take Merman's detected type (the
  `detect_type` result `source_theme()` already computes), then
  `merman::diagram_type_metadata_id(detected)` (public via `pub use
  merman_core::*`), then an exact match on `AsciiCapability::diagram_type`.
  Probed:

  | Source header | detected | metadata id | row |
  | --- | --- | --- | --- |
  | `flowchart LR`, `graph TD` | `flowchart-v2` | `flowchart` | Diagrammatic |
  | `sequenceDiagram` | `sequence` | `sequence` | Diagrammatic |
  | `stateDiagram`, `stateDiagram-v2` | `stateDiagram` | `state` | Diagrammatic |
  | `classDiagram` | `classDiagram` | `class` | Diagrammatic |
  | `erDiagram` | `er` | `er` | Diagrammatic |
  | `xychart-beta` | `xychart` | `xychart` | Diagrammatic |
  | `gitGraph` | `gitGraph` | `gitgraph` | StructuredText |
  | `mindmap` | `mindmap` | `mindmap` | StructuredText |
  | `pie` | `pie` | `pie` | None |
  | `quadrantChart` | `quadrantChart` | `quadrantchart` | None |

  Do **not** use `diagram_type_family_id`, which returns `gitGraph` and not
  the row's `gitgraph`. Merman's own `capability_for_diagram_type` is
  `pub(crate)`.
- **Derived table at `6e6b1e6`** (the R-F2 snapshot source):
    - **Diagrammatic, Partial:**
        - `class`, `er`, `state`, `xychart` with layouts `[Canonical]`.
        - `flowchart`, `sequence`, `swimlane` with layouts
          `[Canonical, Compact, Auto]`.
    - **StructuredText, Partial:** `gantt`, `gitgraph`, `journey`,
      `kanban`, `mindmap`, `packet`, `timeline`, `treeView`.
    - **None (no coverage):**
        - `agentflow`, `architecture`, `block`, `c4`, `cynefin`,
          `eventmodeling`, `info`, `ishikawa`, `pie`
        - `quadrantchart`, `radar`, `railroad`, `requirement`, `sankey`,
          `treemap`, `usecase`, `venn`, `wardley`, `zenuml`
- **Request:** `merman::RenderRequest::ascii(source, OperationControl, merman::AsciiRequest)`
  through the same `Renderer` (so the site config applies), giving
  `RenderOutput::Ascii(Option<AsciiOutput>)`. `AsciiRequest { options:
  AsciiRenderOptions, resources: AsciiResourcePolicy, viewport:
  AsciiViewportPolicy }` derives `Default`. `AsciiRenderOptions` is
  `#[non_exhaustive]`, so build it from `Default` and set these fields:
    - `charset: AsciiCharset` (`Unicode` default, `Ascii`)
    - `terminal_width_profile: TerminalWidthProfile` (`Unicode` default,
      `Cjk`)
    - `layout_profile: AsciiLayoutProfile` (`Canonical` default, `Compact`,
      `Auto`)
    - `color_mode: AsciiColorMode` (`Plain` default, so plain output needs
      nothing)

  `AsciiViewportPolicy { max_width: Option<usize>, overflow: OverflowPolicy
  (Allow default | Fallback | Error), trim: AsciiTrimPolicy }`.
- **Output:** `AsciiOutput` has these fields:
    - `projection: AsciiProjection` (`Diagrammatic`/`StructuredText`)
    - `text: String`
    - `primary_extent`/`emitted_extent: AsciiExtent { width, height }`
    - `layout_profile` (reported) and `requested_layout_profile`
    - `compact_attempted: bool`
    - `requested_max_width`
    - `overflowed: bool`
    - `outcome: AsciiOutputOutcome` (`Primary`, `WideAllowed`, `Fallback`,
      `Empty`)
    - `fallback: FallbackMetadata`
    - `lossiness`
- **Observed behavior** (spec flowchart example; output is byte-identical to
  the spec's figure):
    - Canonical, no max → `Diagrammatic`, layout `Canonical`,
      `overflowed=false`, `outcome=Primary`, extent 50×7.
    - Auto with `max_width=40` → requested `Auto`, `compact_attempted=true`,
      reported `Canonical`, `overflowed=true`, `outcome=WideAllowed`, still
      50 wide. With the default `OverflowPolicy::Allow`, over-width is
      **reported, not an error**. Phase 3 reads `overflowed` and also measures
      rows itself.
    - Canonical with `max_width=20` → `overflowed=true`, `WideAllowed`.
    - **Auto on a family that does not admit it** (state): `Err(RenderError::Ascii(InvalidOption
      { field: "layout_profile", … }))`. Only request `Auto` when
      `layout_profiles` contains it (D7), as planned.
    - **Unsupported family** (pie): `Err(RenderError::Ascii(UnsupportedDiagram {
      diagram_type: "pie" }))`. Decide eligibility before requesting, so an
      ineligible input makes zero requests.
- **Deadline/limit/cancellation:** the request's `OperationControl` deadline
  (as the SVG path) surfaces as `RenderError::Cancelled`. Resource limits
  surface as `RenderError::ResourceLimitExceeded`. Target errors surface as
  `RenderError::Ascii(AsciiError)`. The existing `call_merman` maps all of
  these.
- **biscuit-terminal accessors (Rule F):** `Terminal::supports_unicode`,
  `TerminalRenderContext::supports_unicode`, and
  `biscuit_terminal::utils::block_constraint::visible_width`.

### Task 1.3: CLI and workspace recon

- **`bt` Mermaid subcommands** (`biscuit-terminal/cli/src/commands/`):
    - The nine subcommands are `flowchart.rs`, `state_diagram.rs`,
      `erd.rs`, `pie.rs`, `quadrant.rs`, `timeline.rs`, `xy_chart.rs`,
      `git_graph.rs` and `mermaid.rs`.
    - Each builds its component with
      `commands::mermaid::build_mermaid_diagram` and shows it with
      `display_mermaid`.
    - `display_mermaid` calls the **fallible, image-only**
      `MermaidDiagram::try_render`. On `MermaidRenderError::NoImageSupport`,
      `handle_mermaid_error` prints a raw `` ```mermaid `` fence through
      `Prose` (`mermaid.rs:122-127`). That fence is the path the text fallback
      must replace in Phase 5.
    - An explicit image-export failure (`Visualization`, `DisplayError`)
      exits with an error and must keep doing so.
    - The one shared place to add `--feature` is the args each command
      passes to `build_mermaid_diagram`. Each command has its own args
      struct, so the flag is declared per command (or in a shared flattened
      args struct, like `LayoutArgs`).
- **`md` paths:**
    - The implicit `md <INPUT>` form reads its flags from `args/cli.rs`
      (`--mermaid` at `:52-55`, `--feature` at `:57-67`). `md render` reads
      them from `args/command.rs:38-48`.
    - Both reach `render_terminal_output` (`darkmatter/cli/src/render.rs:52`),
      which sets `with_mermaid_mode`, and `apply_features` (`:122`).
    - `--feature` works on both forms today.
- **Rule D:** proved (see above); the throwaway is not committed.
- **AC-F15 audit list** (exhaustive matches that must handle `TerminalMermaidMode::Automatic`
  or `MermaidMode::Automatic`):
    - `renderable/src/tree/mod.rs:101`: `TerminalMermaidMode` definition.
      There is **no** exhaustive `match` on it anywhere in the workspace. The
      only branch is an equality test at
      `biscuit-terminal/lib/src/render_tree/render.rs:1742`
      (`mermaid_mode == TerminalMermaidMode::Image`), which Phase 2 must
      widen for `Automatic`. That is a silent-behavior site, not a compile
      error, so it needs a test, not only `cargo check`.
    - `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:296`: darkmatter
      `MermaidMode` → `BrowserMermaidMode` (page path). Compile-breaking.
    - `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:604`: darkmatter
      `MermaidMode` → `BrowserMermaidMode` (HTML path). Compile-breaking.
    - `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:723`: darkmatter
      `MermaidMode` → `TerminalMermaidMode`. Compile-breaking.
    - `darkmatter/cli/src/render.rs:70-74`: constructs `Image`/`Off` from
      `--mermaid`. This is a behavior site (Rule B).
    - Constructors only (no change needed):
        - `TerminalMermaidMode::{Image,Code}` assignments in
          `biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs`
          (108, 118, 166-169, 231) and `render_tree_code_context.rs`
        - `biscuit-terminal/lib/src/render_tree/render.rs:5587`
        - `biscuit-terminal/lib/src/render_tree/options.rs` tests
          (496, 510)
        - `biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs`
          (29, 58) and `display_size.rs:177`
        - darkmatter tests in `lib/tests/l1/{render_invariants,mermaid_static_svg,mermaid_embedded_font}.rs`
          and `lib/tests/browser/browser_render.rs`
        - darkmatter defaults in `markdown/{mod,output/*,render_tree/code_renderer,compose/tests}.rs`.
    - `BrowserMermaidMode` matches in `renderable/src/tree/render/browser.rs`
      (1214, 1220, 2144, 2150) match `BrowserMermaidMode`, which this work
      does not extend.
- **How `bt` renders code blocks today:**
    - No `bt` command installs a `code_renderer`. Only `quote.rs:116` and
      `block.rs:272` build `TerminalRenderOptions` and render a tree.
    - Mermaid sources are printed as `Prose`-wrapped fences
      (`mermaid.rs:108`, `mermaid_source_block`, and the
      `NoImageSupport` arm).
    - `graph.rs:199` prints `GraphDiagram::fallback_code_block()` the same
      way.
    - So Phase 5.4 installs darkmatter's code renderer in `quote` and
      `block`, and in every Mermaid subcommand once it renders fallbacks as
      code nodes through `TerminalRenderOptions`.
    - `graph` keeps its own fallback and is out of this spec's scope; it is
      not a Mermaid command.

### Decided during implementation

- **D1 amended (swimlane eligible).** Evidence: `ascii_capabilities()` at the
  delivered rev returns a separate `swimlane` row, `Diagrammatic`, `Partial`,
  layouts `[Canonical, Compact, Auto]`, and `ascii_diagrammatic_diagram_types()`
  returns `["class", "er", "flowchart", "sequence", "state", "swimlane",
  "xychart"]`. Merman's comment on `ascii_capabilities` says "Flowchart and
  Swimlane have separate rows even though they use the same terminal
  projection". The support-matrix document, which the spec quoted, has no
  swimlane row.
    - The choice meets D10's three conditions:
        - it keeps D1's purpose (metadata decides, no name list);
        - it is within every other decision;
        - it adds no dependency, outward action or scope.
    - Spec edits: D1 gains an "Amended during implementation" bullet, and
      AC-F3's family list adds swimlane. The Evidence section's table is left
      as the review recorded it.
    - Flagged in `decisions_for_review`.

### Requirement-to-test mapping

Phase 1 changes no behavior, so it adds no tests. The throwaway program is the
evidence for Rule D and the API facts. Phases 2–5 own the AC-F tests listed in
the plan's checkpoints. The facts above fix their exact inputs:
- the seven-family snapshot (AC-F3)
- `InvalidOption` for `Auto` on state (AC-F11's "exactly one request")
- `overflowed=true` with `WideAllowed` (over-width → source)
- `UnsupportedDiagram` for pie (pie → source, with no request)

### Gates

- The skip check (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`)
  reported `packages: ["darkmatter", "darkmatter-cli", "test-toolkit"]`,
  `change_class: package`. The cause is the pre-existing uncommitted
  embedded-font work, not this phase. The gates were therefore run.
- `just lint` in `biscuit-visualized`: clean.
- `just test` in `biscuit-visualized`: 1070 passed, 4 skipped, 0 failed (13.1 s).
- There are no dependent-package tests, because no package source changed.
  There is no cross-OS check, because Phase 1 has no OS-specific code.
- The biscuit-visualized skill is unchanged; Phase 1 changed no architecture.

## Phase 2

Generic contracts only: `renderable` gains the automatic mode and
`biscuit-terminal`'s diagram hook gains the ceiling, the declined outcome and
node output. No Mermaid-specific type was added to `biscuit-terminal`.

### Task 2.1: `TerminalMermaidMode::Automatic`

- Added `Automatic` to `renderable/src/tree/mod.rs` with the D5 meaning; `Code`
  stays the default.
- AC-F15: `cargo check --workspace --all-targets` is clean. As Phase 1 found,
  there is no exhaustive `match` on the enum; the only consumer is the
  promotion test in `render.rs`, now a `matches!` on `(mode, ceiling)`, pinned
  by tests (below). Darkmatter's own `MermaidMode` is untouched (Phase 5).

### Task 2.2: hook contract

- `options.rs`: `DiagramRenderer::render_terminal_diagram` returns
  `Result<DiagramOutcome, DiagramRenderError>`; `DiagramOutcome`
  (`#[non_exhaustive]`, `Clone`, `PartialEq`) is `Rendered(String)`,
  `Nodes(Vec<RenderNode>)`, `Declined`. `DiagramRenderRequest` gains
  `graphics_mode: GraphicsMode`. Re-exported from `render_tree`.
- `render.rs` `render_code_node`: the hook is consulted for `Image` at `Rich`
  and for `Automatic` at `Rich`/`Vector`, never at `Off`. `Rendered` prints
  verbatim; `Declined` falls through to the code renderer/plain code block with
  no diagnostic; `Err` keeps today's strictness handling.
- `Nodes` goes through a new `render_diagram_fragment`, which renders the
  fragment with `render_blocks` at the fence's context (so indent and width are
  the fence's) and **with `mermaid_mode = Code`**. Without that, a `mermaid`
  source-fallback node in the fragment would be offered to the hook again and
  recurse forever in Automatic mode.
- Test doubles updated: `FailingDiagrams` (`render.rs`) and `RecordingDiagrams`
  (`lib/tests/l1/render_tree_diagram_hook.rs`).
- The one real implementor, `biscuit-visualized`'s `TerminalDiagramRenderer`,
  had to change for the build. It wraps its image in `Rendered` and returns
  `Declined` when `graphics_mode != Rich`, so it never draws an image above
  the ceiling until Phase 4 adds the text route (see `decisions_for_review`).

### Docs

- `renderable/docs/tree-rendering.md`: new "Terminal Mermaid promotion and the
  diagram hook" section (modes table, ceiling in the request, the four answers
  with a Mermaid flow, code nodes through `code_renderer`, example).
- New `biscuit-terminal/docs/render-tree.md`: the two render-tree hooks, what
  each receives, what the renderer does with each answer; linked from
  `biscuit-terminal/README.md` and `biscuit-terminal/lib/README.md`.
  biscuit-terminal had no render-tree topic page, so the plan's "biscuit-terminal
  render-tree topic page" was created.
- `biscuit-visualized/docs/components/mermaid_diagram.md`: when the hook is
  offered a fence now includes Automatic; at `Vector` it declines.
- Skills: `biscuit-terminal/diagrams.md` (hook contract),
  `biscuit-visualized/diagram-components.md` (real hook behavior),
  `renderable/tree.md` (new "Terminal Mermaid modes" section). **Drift fixed:**
  `renderable/tree.md`'s `CodeRenderer` signature showed `width: u32` and no
  `meta`; the code takes `meta` and a `TerminalCodeContext`. The skill was
  corrected to the code, and a stale note about a planned widening removed.

### Requirement-to-test mapping

Boundary: the generic renderer ↔ a caller's hook, across the crate boundary
(F2, L1, explicit `Terminal` capabilities, no real terminal).

| Behavior (checkpoint row) | Test (`biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs` unless noted) |
| --- | --- |
| Image mode unchanged | existing `diagram_hook_is_consulted_before_the_code_hook` (now also asserts `graphics_mode: Rich` in the request), `diagram_hook_failure_follows_the_render_strictness`, `diagram_hook_receives_the_forced_terminal_snapshot`, `nested_mermaid_fence_reaches_the_diagram_hook_at_its_narrowed_width`; `render.rs` `mermaid_promotion_failure_rejects_under_strict` |
| Not consulted: Code at Rich/Vector, Image at Off/Vector, Automatic at Off, other/no language | `diagram_hook_is_not_consulted_unless_promotion_is_requested` (8 cases, 4 new) |
| Automatic at Rich and Vector, ceiling carried | `automatic_mode_offers_the_fence_at_rich_and_vector_with_the_ceiling` |
| Automatic at Off → code, no hook call | `automatic_mode_at_graphics_off_renders_the_code_block_without_the_hook` |
| No hook in Automatic → ordinary code, even Strict | `automatic_mode_without_a_diagram_hook_renders_the_code_block` |
| `Declined` silent under Strict/Warn/Lossy (Automatic Rich/Vector, Image Rich) | `declined_renders_the_code_block_silently_under_every_strictness` (9 cells), `declined_fence_still_reaches_the_code_hook` |
| `Err` rejected/recorded/silent in Automatic | `automatic_mode_hook_failure_follows_the_render_strictness` |
| `Nodes` through installed code renderer, no re-entry of the hook | `nodes_outcome_lays_out_its_code_nodes_through_the_code_hook` |
| `Nodes` without a code renderer → plain code blocks | `nodes_outcome_without_a_code_hook_renders_plain_code_blocks` |
| `Nodes` at the fence's indent and width | `nodes_outcome_in_a_list_item_is_laid_out_at_the_items_indent_and_width` |
| Real hook never emits an image at `Vector` | `biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs` `hook_never_draws_an_image_below_the_rich_ceiling` |

The Input Robustness Matrix does not apply (no parser or config reader).

### Gates

- Skip check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [biscuit-terminal, biscuit-visualized, renderable, test-toolkit,
  biscuit-terminal-cli, darkmatter, tree-hugger-cli]`, `change_class: package`.
  Gates run.
- `cargo check --workspace --all-targets`: clean.
- `renderable`: `just lint` clean; `just test` 684 passed.
- `biscuit-terminal`: `just lint` clean (after replacing a `match` with
  `matches!`); `just test` 3890 passed, 55 skipped. Targeted rerun after the
  lint fix: 18/18.
- `biscuit-visualized`: `just lint` clean; `just test` 1071 passed, 4 skipped.
- `just check-tier-coverage` for all three areas: 0 stranded.
- Dependents: darkmatter (installs the hook), filtered to
  `-E 'test(/mermaid|diagram/)'` with `--all-features`: 89 passed. sniff-cli
  and worktree-cli use the components, not the hook; they compile
  (workspace check) and were not run.
- No cross-OS check: no path, `cfg`, or OS-specific code changed.
- No failures, no load-sensitive reruns.

## Phase 3

The text backend in `biscuit-visualized`: Merman's `ascii` feature, eligibility
from Merman's metadata, one bounded text request, and the fit judgment. Policy,
presentation, the component and the hook are Phase 4.

Starting state: Phase 2's changes were still uncommitted in the working tree,
so the skip check below also covers them. Phase 3's own files are the ones
listed in `source_files_during_phase_3`.

### Task 3.1: enable `ascii`

- Added `"ascii"` to the existing `merman` dependency's features (same pinned
  rev, `default-features = false`). `Cargo.lock` did not change:
  `merman-ascii` at the same rev was already locked.
- `biscuit-visualized/docs/dependencies.md` and the root `docs/dependencies.md`
  name the feature and what it is for.

### Task 3.2: eligibility (`src/src/mermaid/text.rs`)

- `text_eligibility(diagram_type) -> Option<TextEligibility>` looks up the row
  in `ascii_capabilities()`; `eligibility_of(row)` decides: `Diagrammatic`
  primary projection only; coverage `Full`/`Partial`, missing → `Partial`,
  unknown value → not eligible; `admits_auto_layout()` from the row's layout
  profiles. No family names appear in the path.
- `MermaidDiagram::text_eligibility()` uses Merman's preprocess + `detect_type`
  (already run by `source_theme`, whose `SourceTheme` now also records
  the metadata id as `diagram_type`), so a `%% mindmap` comment is not read as the header.

### Tasks 3.3 and 3.4: text render and test seams

- `MermaidDiagram::render_text(TextOptions) -> Result<TextRender, MermaidError>`
  (`terminal` feature; see `decisions_for_review`). Ineligible or zero columns
  → `Source` with no request. Otherwise one `RenderRequest::ascii` through the
  same `Renderer` and site config as SVG, after the same `\n` rewrite; plain
  output, standard width profile, `Auto` when the family admits it else
  `Canonical`, `max_width` = the columns with overflow allowed; the diagram's
  time budget applies, and a cancellation logs the same warning as SVG
  (shared `call_merman`). No palette, design, font chain or cache is touched.
- `text::judge` neutralizes control characters
  (`merman::ascii::normalize_terminal_text`; Merman had already escaped them),
  measures the widest row with `biscuit_terminal::utils::block_constraint::visible_width`,
  and returns `Drawing` or `Source(NotDiagrammatic | Empty | TooWide)` with a
  `TextReport` (requested and reported layout, projection, width, overflow).
- Seams (`#[cfg(test)]`, thread-local, reset on read): `take_text_requests()`
  (render.rs), `take_default_chain_requests()` (fonts/chain.rs, counts every
  `default_chain()` call), `take_stored_files()` (cache/file_cache.rs, every
  artifact or sidecar write). Limit injection: crate-internal
  `render_text_with(options, AsciiResourcePolicy)`; cancellation: a zero time
  budget, past its deadline at Merman's first checkpoint. Nothing sleeps.

### Merman behavior observed at `6e6b1e6` (inputs for Phase 4)

- Spec flowchart: 50 cells canonical; `CHAIN_FLOWCHART` (5 boxes): 66
  canonical, 58 compact. State example: 12 cells, canonical only.
- A drawable family returns `StructuredText` only under
  `OverflowPolicy::Fallback`; under `Allow` it reports `overflowed`.
- Unroutable class/ER output: boxes plus a `relations:` list, reported
  `Diagrammatic` (see `decisions_for_review`).
- Mermaid collapses runs of spaces in a label (`"a   b"` draws `a b`); padding
  and trailing spaces in rows are kept.
- A class label with `\n` prints `one<br>two` after the shared rewrite.
- The XY chart example (`bar [1, 5, 9]`) is 260 cells wide: XY charts will
  usually show the source on an ordinary terminal.

### Requirement-to-test mapping

Boundary: biscuit-visualized ↔ Merman's `ascii` target, in process (F2, L1;
`src/src/tests/text_tests.rs`, compiled under `--all-features`). Nested
position and code-block padding are Phase 4 (the backend takes the final
column count).

| Requirement | Test |
| --- | --- |
| R-F2 snapshot of the derived table | `eligibility_table_at_the_pinned_rev` |
| AC-F3 exactly seven eligible, public lookup | `seven_families_are_eligible_at_the_pinned_rev` |
| D1 field rules (control + one edit per cell: no coverage, full, StructuredText, None, no Auto) | `eligibility_is_decided_by_projection_coverage_and_layouts` |
| Missing row / registry id is not a row id | `a_family_without_a_capability_row_is_not_eligible` |
| Parsed header, not a scan; mindmap, pie, gitGraph, garbage, empty | `diagram_eligibility_follows_merman_detection` |
| AC-F14 exact Unicode drawing (spec figure, byte-identical) | `spec_flowchart_draws_in_unicode` |
| AC-F14 ASCII drawing | `spec_flowchart_draws_in_ascii` |
| AC-F14 authored Unicode in both charsets | `authored_unicode_labels_survive_both_charsets` |
| AC-F14 significant spaces | `state_diagram_keeps_its_significant_spaces` |
| AC-F14 multiline labels | `multiline_label_draws_on_separate_rows` |
| AC-F14 label control characters | `label_control_characters_never_reach_the_terminal` |
| Fit boundary (50 fits, 49 does not) | `a_drawing_fits_exactly_its_own_width` |
| Fit in display cells, by this crate's check | `wide_characters_are_measured_in_cells` |
| AC-F11 compact when canonical too wide; one request | `flowchart_too_wide_canonically_draws_compactly` |
| AC-F11 too wide for both → source | `flowchart_too_wide_for_both_layouts_shows_source` |
| AC-F11 state too wide → source, canonical, one request | `state_diagram_too_wide_canonically_shows_source_after_one_request` |
| Zero columns → source, no request | `zero_columns_show_source_without_a_request` |
| AC-F2 dense class at narrow width → source | `dense_class_diagram_at_a_narrow_width_shows_source` |
| AC-F2 real `StructuredText` result → source, projection asserted | `structured_text_result_of_a_diagrammatic_family_shows_source` |
| Empty output → source (control + edit) | `empty_output_shows_source` |
| AC-F1/AC-F14 unsupported family → source, no request | `ineligible_families_show_source_without_a_request` |
| AC-F14 invalid source → error, one request | `invalid_source_is_an_error_after_one_request` |
| AC-F14 cancellation → error, warning, no retry | `exceeded_time_budget_is_an_error_with_a_warning_and_no_retry` |
| AC-F14 resource limit → error, no retry | `reached_resource_limit_is_an_error_and_no_retry` |
| AC-F13 / R-F10 backend half | `text_render_writes_no_artifact_and_asks_for_no_font_chain` |
| Counters are load-bearing (control) | `the_work_counters_count_image_work` |

Doctests on `TextEligibility`, `TextOptions`, `MermaidDiagram::text_eligibility`
and `MermaidDiagram::render_text` pass (`cargo test --doc -- text`).

The Input Robustness Matrix does not apply: no file format or configuration
reader was added (Merman's capability table is compiled data).

### Gates

- `just lint` (biscuit-visualized): clean. `cargo clippy --features image`
  (without `terminal`): clean.
- `just test` (biscuit-visualized): 1097 passed, 4 skipped, 0 failed (12.9 s).
  New: 26 tests, all under 0.2 s.
- `just check-tier-coverage biscuit-visualized`: 0 stranded.
- Skip check (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [biscuit-terminal, biscuit-visualized, renderable, test-toolkit,
  biscuit-terminal-cli, darkmatter, tree-hugger-cli]`, `change_class: package`
  (includes Phase 2's uncommitted files). Gates run.
- Dependents: `cargo check --workspace --all-targets` clean; darkmatter
  `-E 'test(/mermaid|diagram/)'` with `--all-features`: 89 passed.
  biscuit-terminal-cli, sniff-cli and worktree-cli use only the unchanged
  component API; they compile (workspace check).
- No cross-OS check: no path, `cfg(target_os)`, or OS-specific code was added.
- No failures and no load-sensitive reruns.

## Phase 4

The fallback policy, the component's text route, and the Markdown hook's
automatic mode in `biscuit-visualized`, plus two small generic additions to
biscuit-terminal that the selection rules needed.

### Task 4.1: fallback policy (`src/src/components/fallback_policy.rs`)

- `MermaidFallbackPolicy` (two independent `Option` settings, builders,
  getters), `TextFallback::{Allowed (default), NoPartial, Never}`,
  `ResolvedFallbackPolicy`, `MERMAID_FALLBACK_ENV`.
- `resolve()` reads the process environment; `resolve_with(env)` is the
  injectable seam. The variable is consulted only when the text-fallback
  setting is unset, and never for `show_partial_note`. `code-block` matches
  case-insensitively; any other value, including empty, leaves the default.
- `MermaidFallbackFeature` (clap `ValueEnum`, feature `clap`) and
  `MermaidFallbackPolicy::from_features`: most restrictive wins in any order;
  repeats are harmless; no features means every setting stays unset, so the
  variable applies.
- Re-exported from `components`.

### Task 4.2: component (`src/src/components/mermaid.rs`)

- `MermaidDiagram::present(&Terminal) -> MermaidPresentation`
  (`Image(MermaidRenderResult)` | `Text(nodes)` | `Source(nodes)`), plus the
  crate-internal `present_with(term, resolved)` that tests use for
  hermeticity. `render` and `render_optimistic` go through it. The
  pre-existing `render_raw` was removed, which also removes a latent double
  layout on the optimistic path.
- Image availability: `TerminalImage::route_has_pixels(term)`, a new public
  function in biscuit-terminal that answers exactly as `render_with_route`'s
  encoder choice does, before any image exists (see decisions). On that route
  the component calls the unchanged `try_render`, and a failure shows the
  source (no text retry).
- Text route: columns = `resolve_scaled_dimensions_for(width, layout,
  term_width, None, None).image_width`, so `Scale` uses the available columns
  without SVG measurement, and margins and cell/percent limits apply. That
  number, less `TEXT_BLOCK_CHROME_COLUMNS` (2), goes to `render_text`. The
  charset follows `term.supports_unicode`. The nodes are a `txt` code node,
  then `partial_note()` when the drawing is partial and the note is shown.
  Policy `Never`, or `NoPartial` for a partial family, makes no request
  (checked via `text_eligibility()`).
- Source: one `mermaid` code node of the unmodified instructions. Every
  `Source(_)` and every text `Err` maps to it.
- Title: an explicit `with_title` leads the text and source nodes as a strong
  paragraph, unless a source line already gives it (`title X`, `title: X`,
  `pie title X`). The image route is unchanged (the title stays alt text).
- Nodes render with biscuit-terminal's tree renderer (no code renderer,
  `Lossy`) at the text columns; margins and alignment are applied once
  afterward. Phase 5 can call `present` and lay the nodes out with its own
  code renderer.
- New public items: `MermaidPresentation`, `partial_note()`,
  `PARTIAL_NOTE_ID`, `TEXT_BLOCK_CHROME_COLUMNS`,
  `with_fallback_policy`/`fallback_policy`; on the artifact type,
  `mermaid::MermaidDiagram::title()`.
- Existing unit tests that compared the image-route failure to
  `fallback_code_block()` now compare the escape-stripped output. The source
  is a rendered `mermaid` code node (D9), which the plain code block dims, and
  the time-budget test's source ends in a newline, which the code block trims.

### Task 4.3: hook (`src/src/components/render_hook.rs`)

- `TerminalDiagramRenderer` is now a struct holding a `MermaidFallbackPolicy`
  (`default()`, `with_fallback_policy`, `fallback_policy`). The one in-tree
  construction outside this crate (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:787`)
  was updated to `TerminalDiagramRenderer::default()`. Phase 5 installs the
  folded policy there.
- `Image` mode: image at `Rich` (`Rendered` or `Err`); declined below `Rich`.
  `Automatic` mode: at `Rich` with `route_has_pixels`, image (`Err` on
  failure); otherwise `text_nodes` → `Nodes`, `Declined` (policy, family,
  fit), or `Err` (parse, Merman, budget, limit).
- biscuit-terminal: `DiagramRenderRequest` gains `mermaid_mode` (see
  decisions); the renderer fills it. Its L1 recording hook asserts it for
  `Image` and `Automatic`.

### Decided during implementation

- Adding `mermaid_mode` to the hook request, and `route_has_pixels`, are
  recorded in `decisions_for_review`. Neither invokes D10: both are generic
  biscuit-terminal additions that keep its library Mermaid-agnostic in
  behavior (`TerminalMermaidMode` was already in its context type).

### Requirement-to-test mapping

Boundaries: the component ↔ the text backend and biscuit-terminal's tree
layout (F2, L1, in-crate so the AC-F13 counters are visible), and the real
hook ↔ biscuit-terminal's generic renderer (F2, L1, `tests/l1`). All tests
use explicit `Terminal` capabilities; nothing opens a window.

| Requirement | Test |
| --- | --- |
| AC-F5 explicit setting ignores env (all three values; env never read) | `component_fallback_tests::an_explicit_text_fallback_never_reads_the_environment` |
| AC-F5 env shapes: unset, empty, other, `code-block`, `Code-Block`; only `MERMAID_FALLBACK` asked | `an_unset_text_fallback_reads_mermaid_fallback` |
| AC-F5 env never changes the note | `mermaid_fallback_never_changes_the_partial_note` |
| Rule C fold: none → unset (env applies) | `features::no_feature_leaves_every_setting_to_the_environment` |
| Rule C fold: each feature sets only its setting | `features::each_feature_sets_only_its_own_setting` |
| D4 overlap → never, both orders; repeat harmless | `features::overlapping_features_take_the_more_restrictive_in_either_order` |
| AC-F5 flag beats variable | `features::a_flag_beats_the_variable` |
| AC-F1 flowchart → text; mindmap, pie → source (no note, source unmodified) | `a_flowchart_is_drawn_as_text_and_mindmap_and_pie_show_source` |
| AC-F4 exact plain snapshot (drawing with trailing spaces, blank line, wrapped note), bold/dim/italic spans | `the_text_fallback_renders_exactly_with_its_note` |
| AC-F4 `show_partial_note = false` → no note (nodes and output) | `a_hidden_note_leaves_only_the_drawing` |
| Rule F ASCII charset | `a_terminal_without_unicode_gets_an_ascii_drawing` |
| AC-F6 every eligible family (7 fixtures) under no-partial → source, zero requests; control | `no_partial_text_shows_the_source_for_every_eligible_family` |
| D3 never → source, zero requests; builder round trip | `never_text_shows_the_source_without_a_request` |
| Text failure (parse error) → source | `a_text_failure_shows_the_source` |
| Title once, on text and source; not duplicated when the source gives it (pie header, frontmatter) | `an_explicit_title_leads_text_and_source_once` |
| Width: cell limit less chrome (60 fits, 59 does not) | `a_cell_width_less_the_code_block_padding_bounds_the_drawing` |
| Width: percent and margins | `percent_and_margins_narrow_the_drawing` |
| Margin applied once | `the_margin_applies_once_to_the_text_presentation` |
| AC-F13 text route: 1 request, 0 font chain, 0 artifacts (default `Scale` width) | `the_text_route_does_no_image_work` |
| AC-F13 image route control; image failure → source, no text retry | `the_image_route_never_falls_back_through_text` |
| AC-F8 image at Rich with images | `tests/l1/render_tree_diagram_hook.rs::automatic_mode_draws_an_image_at_rich_with_image_support` |
| AC-F8 text at Vector; text at Rich without images (note included) | `automatic_mode_draws_text_below_rich_or_without_images` |
| AC-F8 code at Off | `automatic_mode_at_graphics_off_renders_code` |
| AC-F8 ineligible → code, no diagnostic, Strict/Warn/Lossy × Rich/Vector | `an_ineligible_fence_renders_as_code_without_a_diagnostic` |
| AC-F8 parse error: Strict rejects, Warn records, Lossy silent | `a_parse_error_follows_the_render_strictness` |
| AC-F8 no hook → code; Code mode unchanged | `automatic_mode_without_the_hook_and_code_mode_render_code` |
| AC-F8 Image mode unchanged (never text) | `image_mode_never_answers_with_text`, plus the existing Image-mode tests in that file |
| D3 policy on the hook builder (never → silent decline under Strict; note hidden) | `the_hooks_fallback_policy_applies` |
| Hook success output identical to the component's | `hook_text_is_exactly_the_components` |
| Nested available width (list item) | `a_nested_fence_is_drawn_within_its_own_width` |
| `route_has_pixels` matches the encoder | biscuit-terminal `route_dispatch::route_has_pixels_matches_the_routes_encoder` + doctest |
| Request carries `mermaid_mode` | biscuit-terminal `tests/l1/render_tree_diagram_hook.rs` (Image and Automatic assertions) |
| AC-F13 existing image tests untouched | the biscuit-visualized image/golden/component suites pass unchanged |

Doctests: `MermaidFallbackPolicy`, `from_features` and `TerminalDiagramRenderer`
(3 passed); biscuit-terminal `route_has_pixels` (1 passed).

The Input Robustness Matrix does not apply: no file format or configuration
reader was added. `MERMAID_FALLBACK` is one scalar; its shapes (unset, empty,
other, match, mixed case) are each tested once.

### Gates

- Skip check (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [biscuit-terminal, biscuit-visualized, darkmatter, test-toolkit]`,
  `change_class: package`. Gates run.
- biscuit-visualized: `just lint` clean; `just test` 1127 passed, 4 skipped;
  `just check-tier-coverage biscuit-visualized`: 0 stranded.
- biscuit-terminal (changed directly): `just lint` clean; `just test` 3891
  passed, 55 skipped.
- `cargo check --workspace --all-targets`: clean.
- Dependents: darkmatter `just lint` clean;
  `-E 'test(/mermaid|diagram/)'` with `--all-features`: 89 passed.
  sniff-cli, worktree-cli and biscuit-terminal-cli compile (workspace check);
  they use only the component's unchanged image API.
- No cross-OS check: no path, `cfg`, or OS-specific code changed.
- Failures fixed during the phase: none load-sensitive. One ownership
  compile error was fixed in the new biscuit-terminal test (`ImageSupport`
  is not `Copy`).

## Phase 5

darkmatter (vendored Mermaid syntax, `md --mermaid` → automatic mode,
`--feature` names) and `bt` (`--feature` on every Mermaid subcommand,
darkmatter highlighting). Two parallel subagents: one on darkmatter (5.1, 5.2),
one on `biscuit-terminal/cli` (5.3, 5.4); the orchestrator ran the gates.

### Task 5.1: vendored Mermaid syntax (`darkmatter/lib/assets/syntaxes/mermaid/`)

- SublimeText/Mermaid at `a5c6d45d1b0a99f8d4e0fdb2b5ccd7fd07809859`, fetched
  into an empty temp directory (nothing run from it), flattened into
  `Mermaid.sublime-syntax` (base + Flowchart + SequenceDiagram, no `extends`,
  no CSS extension; `file_extensions` include `mermaid` and `mmd`). Upstream
  MIT `LICENSE` and `SOURCE.md` (commit, what was flattened, every edit) sit
  beside it.
- Edits beyond flattening (listed in `SOURCE.md`): a header rule for every
  other diagram family (upstream highlights only flowchart and sequence
  headers, so a gantt header would not highlight), and the `%%{init}%%` rule
  plus 12 node-shape rules rewritten, because they combine `embed` with
  `pop`/`set`, which syntect misreads (node text never closed).
- **Syntax-set cost (debug, measured):** two-face's set alone ≈ 24 ms; merged
  with Mermaid via `into_builder` + `add` + `build` ≈ 457 ms (split between
  the two calls; optimizing syntect in the test profile did not help); a
  separate set of plain text + Mermaid ≈ 6 ms. So Mermaid lives in its own
  lazily built set, and `CodeHighlighter::syntax_set_for(grammar)` returns the
  set that owns a grammar. The terminal and HTML highlighters in
  `markdown/output/code_block.rs` resolve through it. Because the set is
  separate, the vendored file references no other syntax: JavaScript inside
  `%%{init}%%`, HTML in labels and YAML in `---` blocks keep only their
  string/comment colors. See `decisions_for_review`.
- MIT notice recorded in `darkmatter/docs/dependencies.md` ("Vendored Mermaid
  Syntax") and the root `docs/dependencies.md`.
- Three existing darkmatter assertions looked for a whole source line in raw
  terminal output; highlighting now splits it across colored spans, so they
  compare visible text: `entrypoints.rs::render_tree_terminal_draws_mermaid_through_the_installed_hook`,
  `style_features_baseline.rs::terminal_mermaid_defaults_to_code`,
  `style_features_phase5.rs::explicit_text_mermaid_mode_renders_code_not_interactive`.

### Task 5.2: automatic mode and features in `md`

- `MermaidMode::Automatic` (`markdown/output/terminal.rs`); terminal mapping →
  `TerminalMermaidMode::Automatic`; both browser mappings treat it as `Image`
  (`StaticSvg`). `md --mermaid` (`cli/src/render.rs`) now passes `Automatic`.
  `md --mermaid --html` is unchanged: `md`'s HTML path never reads
  `--mermaid` (interactive Mermaid.js), so the browser mapping only matters
  to library callers.
- Policy plumbing: new public `TerminalOptions::mermaid_fallback_policy` and
  `DarkmatterPage::with_mermaid_fallback_policy`; `entrypoints.rs` installs
  `TerminalDiagramRenderer::default().with_fallback_policy(..)`.
- `Feature` (`cli/src/args/enums.rs`) gains `NoMermaidFallbacks`,
  `NoMermaidPartialFallbacks`, `NoMermaidPartialWarning`,
  `NoMermaidRendering`; `Feature::mermaid_fallback()` maps the first three onto
  `MermaidFallbackFeature` for `MermaidFallbackPolicy::from_features`.
  darkmatter-cli depends on biscuit-visualized with `clap` directly (already
  in its graph through darkmatter, so no new crate builds).
- `no-mermaid-rendering` lowers to the existing `MermaidMode::Text` (code in
  the terminal, code and no Mermaid.js in both HTML paths) and wins over
  `--mermaid` and `embed-mermaid-font`. Markdown and JSON output never read
  Mermaid settings and keep the fence. `md` has no static-SVG flag, so "wins
  over static SVG" holds by construction.
- New public `terminal_page(...)` in the CLI library (mirrors `html_page`), so
  tests drive the CLI's real lowering in-process. `render_terminal_output`
  takes the features; **before this the terminal path never applied
  `--feature` at all** (a latent defect: `--feature embed-mermaid-font` only
  reached HTML, which was its only consumer, so nothing visible changed).

### Tasks 5.3 and 5.4: `bt`

- **Plan correction:** `bt` has no `mermaid` or `xy-chart` subcommand. The
  nine Mermaid subcommands are `flowchart`, `state-diagram`, `erd`,
  `pie-chart`, `quadrant`, `timeline`, `bar-chart`, `line-chart` and
  `git-graph` (`xy_chart.rs` holds two). All nine take `--feature` through a
  shared flattened `MermaidFeatureArgs` (`cli/src/args.rs`; clap
  `value_delimiter=','`, `ignore_case`, `Append`, value enum
  `MermaidFallbackFeature`), so `no-mermaid-rendering` is a usage error. The
  plan's "`bt mermaid`" CLI tests run against `bt flowchart`.
- `display_mermaid` takes the unchanged image path (`try_render`) only when
  `TerminalImage::route_has_pixels` is true, so an image-export failure still
  exits non-zero and `--meta`/`--debug` stay image-only. Otherwise
  `render_mermaid_fallback` takes `MermaidDiagram::present` and lays the
  nodes out with darkmatter's code renderer at the component's
  `text_columns` (now public on the component), with margins applied once.
  The old raw-fence branch is gone; the route hint is still printed.
- Highlighting: `darkmatter_code_renderer(term)` in `commands/shared.rs` is the
  one local `#[allow(deprecated)]` use of `TerminalCodeRenderer::for_terminal(term,
  CodeBlockMode::default())`, built the way darkmatter's public `CodeBlock`
  builds it. Installed in the Mermaid fallback, `quote` and `block`.
  darkmatter has no default features, so nothing to trim.
- `tests/level2/diagrams.rs::level2_diagram_fallback_when_no_image_protocol`
  asserted the old raw fence; it now asserts the text drawing and note and
  that the source is absent (1 passed under real tmux, 1.3 s).
- Dependency graph: `cargo tree -p biscuit-visualized -p biscuit-terminal`
  (with and without dev edges and all features) lists no `darkmatter` and no
  `syntect`; darkmatter appears only under `biscuit-terminal-cli`. No
  dependency-graph test pattern exists in the repo, so this is a recorded
  command, not a test.
- Observed: `bt bar-chart 1 2 3` and `bt line-chart 1 2 3` show their source
  on a no-image terminal although xychart is eligible. Probed with `md
  --mermaid` under a pseudo-terminal (`BISCUIT_GRAPHICS=none`): an
  `xychart-beta` with `x-axis [1, 2, 3]` / `bar [1, 2, 3]` shows source, and
  adding `y-axis 0 --> 5` makes Merman draw it (string or numeric categories
  alike). So Merman's text target needs an explicit y range, and `bt`'s
  generated source has none. A drawn xychart also begins with a raw
  `xDomain: band categories=[bytes=1="1", …]` line from Merman, which looks
  like a debug artifact. Both are Merman/`bt` source-generation behavior, not
  this phase's selection logic; the `bt` docs page promises no text for those
  two commands. Handed to Phase 7.

### Decided during implementation

- None invoke D10. The separate Mermaid syntax set keeps D8's purpose
  (highlighting for every Mermaid block, in terminal and HTML) and changes no
  other decision; the loss of embedded-language highlighting is recorded in
  `decisions_for_review`. The bt subcommand list is a plan error, corrected
  above.

### Requirement-to-test mapping

Boundaries: syntect loading of the shipped syntax file (F2, L1, darkmatter
library), the `md` CLI's argument lowering (in-process via `terminal_page`,
F2) and the spawned `md`/`bt` binaries for wiring (F3, L1), and `bt`'s
fallback layout through darkmatter's renderer (in-crate, F2, L1). Spawned
tests force `BISCUIT_GRAPHICS=none` (md) or `--plain` and explicit env (bt),
and set or clear `MERMAID_FALLBACK`; nothing opens a window.

| Requirement | Test |
| --- | --- |
| AC-F12 no `extends`, loads with syntect, self-contained | `darkmatter/lib/tests/l1/mermaid_syntax_highlighting.rs::vendored_mermaid_syntax_is_flat_and_self_contained` |
| AC-F12 gantt header, comment, `%%{init}%%` (terminal and HTML) | `…::mermaid_gantt_highlights_its_header_comment_and_init_directive` |
| AC-F12 flowchart nodes and arrows | `…::mermaid_flowchart_highlights_nodes_and_arrows` |
| AC-F12 sequence participants and arrows | `…::mermaid_sequence_highlights_participants_and_arrows` |
| `mermaid` and `mmd` tokens | `…::mermaid_and_mmd_fences_highlight_identically` |
| AC-F12 MIT notice | docs: `darkmatter/docs/dependencies.md`, `docs/dependencies.md` |
| AC-F7 md: `a,b` = repeated, any case; declared set | `darkmatter/cli/tests/l1/feature_flag.rs::mermaid_feature_names_parse_comma_separated_repeated_and_in_any_case`, `::feature_names_are_the_declared_set` |
| AC-F7 md: unknown / empty → usage error listing names | `::unknown_name_is_a_usage_error_listing_valid_names`, `::empty_name_is_a_usage_error_listing_valid_names`, `::unknown_feature_exits_with_usage_error` (spawned) |
| AC-F7 md: completion | `::completion_offers_feature_names` |
| AC-F9 `md --mermaid` → automatic | `darkmatter/cli/tests/l1/mermaid_features.rs::mermaid_flag_selects_automatic_mermaid_mode`; library: `darkmatter/lib/tests/l1/mermaid_automatic_mode.rs::{automatic_mermaid_mode_draws_an_eligible_flowchart_as_text, automatic_mermaid_mode_keeps_an_ineligible_pie_as_code, automatic_mermaid_fallback_policy_never_keeps_the_flowchart_source, automatic_mermaid_mode_renders_the_image_mode_browser_page}` |
| AC-F5 md: no flag follows env (unset → drawing, `Code-Block` → source) | `mermaid_features.rs::mermaid_render_without_a_flag_follows_the_fallback_env` |
| AC-F5 md: flag beats variable | `::mermaid_no_fallbacks_flag_beats_the_fallback_env` |
| md features fold; partial note hidden | `::mermaid_fallback_features_fold_into_the_page_policy`, `::mermaid_partial_warning_feature_hides_the_partial_note` |
| md fallback features inert without `--mermaid` | `::mermaid_fallback_features_without_the_mermaid_flag_change_nothing` |
| AC-F10 terminal (also with `--mermaid`) | `::no_mermaid_rendering_keeps_the_terminal_fence_a_code_block_over_mermaid` |
| AC-F10 HTML: highlighted, no Mermaid.js, wins over `--mermaid` and `embed-mermaid-font` | `::no_mermaid_rendering_html_is_a_highlighted_code_block_without_mermaid_js` (spawned) |
| AC-F10 Markdown and JSON keep the fence | `::no_mermaid_rendering_keeps_the_markdown_and_json_mermaid_fence` (spawned) |
| AC-F5 bt: no flag, no variable → text | `biscuit-terminal/cli/tests/l1/mermaid_features.rs::without_a_flag_or_the_variable_the_diagram_is_drawn_as_text` |
| AC-F5 bt: `MERMAID_FALLBACK=Code-Block` → source | `::without_a_flag_mermaid_fallback_code_block_shows_the_source` |
| AC-F5 bt: flag → source with variable unset | `::a_flag_shows_the_source_with_the_variable_unset` |
| bt: each feature's effect | `::no_partial_fallbacks_shows_the_source`, `::no_partial_warning_hides_the_note` |
| AC-F7 bt: `a,b` = repeated; case | `::comma_separated_features_equal_repeated_flags`, `::feature_names_match_case_insensitively` |
| AC-F7 bt: unknown, `a,,b`, `""`, `no-mermaid-rendering` → exit 2 listing names | `::an_unknown_or_empty_name_is_a_usage_error_listing_the_names` |
| AC-F7 bt: completion | `::completion_offers_the_feature_names` (dynamic), `::the_static_completion_script_lists_the_feature_names` (bash) |
| bt: all nine commands take `--feature` | `::every_mermaid_command_accepts_the_feature_flag` |
| AC-F17 source fallback highlighted (24-bit SGR) | `biscuit-terminal/cli/src/commands/mermaid.rs::tests::a_source_fallback_is_highlighted_by_darkmatter` |
| AC-F17 text fallback highlighted | `…::a_text_fallback_is_highlighted_by_darkmatter` |
| AC-F17 same renderer as `md` | `…::a_source_fallback_matches_darkmatters_code_block` (body compared; `CodeBlock` adds a header row the component's node does not ask for) |
| no color → no escapes | `…::without_color_fallbacks_are_plain_fences` |
| image-export failure stays an error | `…::an_image_export_failure_is_still_an_error` (calls the error handler directly; a real export failure loads every system font first, 2.2–3 s, too near the 5 s budget) |
| `quote` highlights fenced code | `biscuit-terminal/cli/tests/l1/integration_test.rs::test_quote_highlights_fenced_code` |
| AC-F17 dependency graph | `cargo tree -p biscuit-visualized -p biscuit-terminal`: no darkmatter, no syntect |

The highlighting assertions look for 24-bit foreground codes (`\x1b[38;2;`):
the plain code block already emits a dim code, so "any SGR" would prove
nothing.

The Input Robustness Matrix does not apply: the only new reader is the
vendored syntax file, a grammar loaded by syntect, not configuration. The
`--feature` value shapes (valid, mixed case, repeated, comma list, unknown,
empty element, empty value) are each tested once per CLI.

### AC-F15 audit (complete)

- `TerminalMermaidMode`: no exhaustive `match` anywhere in the workspace
  (Phase 1); the one equality site in biscuit-terminal's renderer was widened
  in Phase 2.
- darkmatter `MermaidMode`: three matches, all in
  `darkmatter/lib/src/markdown/render_tree/entrypoints.rs` — page browser
  mapping and HTML browser mapping (`Image | Automatic → StaticSvg`), terminal
  mapping (`Automatic → TerminalMermaidMode::Automatic`). Behavior site
  `darkmatter/cli/src/render.rs` (`--mermaid` → `Automatic`). No other crate
  matches on `MermaidMode`.
- `cargo check --workspace --all-targets` (Gates below) compiles every site.

### Gates

- Skip check (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [biscuit-terminal, biscuit-terminal-cli, biscuit-visualized, test-toolkit, darkmatter, sniff]`,
  `change_class: package`. Gates run. (Run after the author committed the
  darkmatter half as `f55a9c6da` mid-phase; the full working tree was
  wider before that.)
- biscuit-visualized: `just lint` clean; `just test` 1127 passed, 4 skipped;
  `just check-tier-coverage biscuit-visualized`: 0 stranded.
- biscuit-terminal (library + `bt`): `just lint` clean; `just test` 3908
  passed, 55 skipped; tier coverage 0 stranded. Subagent: `just test-l2
  level2_diagram_fallback_when_no_image_protocol` 1 passed (real tmux).
- darkmatter: `just lint` clean; `just test` **failed first** on 12, then 1,
  then 1 pre-existing tests, none in code this phase touched:
  `l1::schemas_convert_snapshots::*` (40 snapshots),
  `l1::suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`,
  and darkmatter-cli `l1::schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`
  (8 `expected.json` fixtures). Cause: the stored outputs have alphabetically
  sorted JSON keys, but serde_json's `preserve_order` (a darkmatter contract
  since `bbde223e6`, 2026-10-23, and pulled in by Merman before that) emits
  insertion order. `.snap.new` files from 13:01 show the darkmatter subagent's
  run hit them too. Fixed by re-recording: every new snapshot and fixture was
  checked to parse to JSON **equal** to the old one (0 value changes; key
  order only), the fixtures regenerated from the real `md schema validate
  --format json`. Then `just test`: 10708 passed, 10 skipped. Tier coverage 0
  stranded. See `decisions_for_review`.
- `cargo check --workspace --all-targets`: clean (AC-F15; also proves no other
  crate builds darkmatter's `TerminalOptions` by struct literal).
- Dependents: worktree-cli (the only other crate whose tests touch Mermaid)
  `-E 'test(/git_graph/)'` 40 passed. sniff-cli and the rest compile in the
  workspace check; they use darkmatter's or biscuit-visualized's unchanged APIs.
- No cross-OS check: no path, `cfg` or OS-specific code changed. The new
  spawned CLI tests set explicit environment and parse plain stdout.
- Load sensitivity: none observed. One test was reworked by the `bt`
  subagent to avoid a 2.2–3 s system-font load (see the mapping table).
- `bt` subagent `cargo nextest` (targeted): 123 passed; darkmatter subagent
  targeted runs: 771 + 532 (library), 135 + 217 (CLI, guards, help,
  completions) passed.
