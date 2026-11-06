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
  - claudine
  - darkmatter-lsp
  - claudine-cli
  - simplified-schema
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
  - |-
    Reference check failures now fail `md compose`, as the plan and spec require, and that exposes real errors the old code hid. These documents used to exit 0 and now exit 1: a hyperlink that points outside the repository, an invalid `::file` in a transcluded child, and `ignore_invalid: true` with an invalid target. Alternative: report an out-of-tree hyperlink as an ordinary broken-reference row instead of a run failure, which is gentler for authors but reverses an existing design note on reference resolution. Redo cost: about 20 lines plus tests. This is the decision most worth a reviewer's attention.
  - |-
    The reference check prepares documents in a lenient mode (`ComposeOptions::for_reference_check`, the same deferrals the shell-approval walk uses). The plan only said to propagate the error. A bare propagation failed 12 unrelated tests, because `{{ }}` and `$(...)` frontmatter values were never resolved during the check, so it had been silently doing nothing on those documents. With the lenient mode, only failures the check can prove reach the run; a child's own `{{ bad( }}` stays a tolerated notice. Alternative: make those fatal, which is one flag but changes the transclusion stopping rule. Graph preparation also gained frontmatter interpolation, which affects `md graph` and `md validate refs` too and makes them more accurate. Redo cost: one line, but those values then need another way to resolve.
  - |-
    Direct `md compose` now applies the four document-judging `--allow` recoveries (ctx-override, missing-transclusions, invalid-frontmatter-assignment, reassigned-frontmatter-property) before the reference check, not after. The plan did not mention this; it is a defect the old swallowed error hid, found when the full suite failed `--allow ctx-override`. Remote and cache options stay after the check on purpose, so the passive check does not start fetching. Alternative: run the check on the fully built request, as the lifecycle path does, which is simpler but risks network reads during a passive check. Redo cost: a few lines in `commands/compose.rs`.
  - |-
    Link-title warnings are produced by two scans: one over authored text before preparation (so a failed run keeps them and `--deny` stops before anything runs) and one after the inline stages (for titles a stage wrote). The plan did not name a stage. A consequence: a link inside a false `::block` still warns, and `md clean` migrates it too. Alternative: one late scan, which loses warnings when an inline stage fails. Redo cost: one call site.
  - |-
    `md render` also reports link-title warnings and accepts their codes with `--deny`, so `md render --deny all` now blocks on them. The plan said "composition/render" but render had no code registry, so `RENDER_DIAGNOSTIC_CODES` was added. Alternative: compose only. Redo cost: small.
  - |-
    `md clean` migrates titles in the clean-document step rather than the shared body cleanup, because compose and render share that body step and would otherwise start applying attributes silently. Three departures from strict migration: a legacy title with no supported key (`"Run x=1 first"`) becomes `{ title: "Run x=1 first" }` rather than `{}` so its text survives; titles holding `{{ … }}` stay legacy and keep warning, because JSON escaping could break the expression; a reference definition also used by an image stays legacy because image titles keep their old syntax. Alternatives: strict parity, which destroys text or warns forever. Redo cost: small per case.
  - |-
    Rule 30 said to update `compose.rs`'s report-view assertion. It was left unchanged: `warning_rows_put_each_extra_line_on_a_row_of_its_own` passes its own literal to `warning_rows`, which non-reference warnings still use, and never reads the report. The plan's statement is stale there. Redo cost: none.
  - |-
    Added a borderless table (`Table::borderless`, `TableTerminalHints.borderless`) to the shared crates, because the spec's report rows need aligned columns without hand padding and no such table existed. Its width overhead still reserves box-line columns, so a narrow terminal wraps about 4 columns early. A row with text only in its first cell spans the columns and is not measured, so a long detail line does not widen the first column. HTML and Markdown keep an empty header row, since GFM requires one. Redo cost: small.
  - |-
    The report's terminal form has its own tree through a `render_terminal` override (bare `file://` URI, heading in `qp`, no blank line a Markdown table would need), rather than sharing one tree across targets as the plan's "one builder" suggests. Alternative: Prose's link hint, which reads the process directory, which this repo forbids. Redo cost: moderate.
  - |-
    Report details the plan left open: the title line appears only when errors exist, so a warnings-only `md validate refs` shows groups and a summary with no "Broken references"; `md compose` prints nothing when there are no errors, so a warnings-only run does not announce "all ok"; ok is scanned minus errors, so ok + broken always equals the total and warnings count as ok; an in-memory source displays as `<stdin>`; group paths retry through canonical forms because `/var` versus `/private/var` printed `../../…` on macOS; `#` and `?` in a path become `%23`/`%3F` in Markdown and HTML destinations, a narrow exception to "the path is not rewritten" that keeps `c#/doc.md` one destination. Each is a one-line change.
  - |-
    Positions, special cases: a flow-list `prologue: [./a.md, ./b.md]` reports the key's line and column 1 because the existing locator cannot place list items (alternative: null); a literal prologue link takes its value's location with no section; a `::toc-linking` synthesized link reports `inserted: false` at its directive, since the spec's inserted rule names only expressions, shell steps, and `replace:`; a byte-order mark does not count as a column for references, while the older `AuthoredSpan::locate` still counts it, which matters only for a `when=` directive on line 1 of a BOM file with no frontmatter; a multi-line reference in a CRLF file keeps its whole span, so LF and CRLF agree, where the plan's literal wording would give CRLF a one-character span. Each is a few lines.
  - |-
    Fragment validation keeps today's slugs (it accepts `#setup` and rejects `#setup-1`), while `section` uses final duplicate-numbered slugs, as the spec requires. The snapshot therefore holds both and each document parses its headings twice. Alternative: validate against numbered slugs, which removes the second parse but changes which fragments count as broken. A test pins the current behavior.
  - |-
    Diagnostics: a named in-memory document reports positions in its own text rather than null, since that text is its frame; null applies to unknown sources and URLs. A schema problem with no line keeps `source: null` rather than the spec's file-only fallback, to stay surgical, and a missing column is now null rather than an invented 1. `ReferenceError::FileReference` has `source: null` and a directive parse error is file-only, because those errors carry no authored location; link-title warnings carry no `section`. `construct` strips `in ` from the report phrase to match the spec's values. Adding the missing locations means wrapping errors at three call sites.
  - |-
    Two defects found and deliberately left out, because fixing them changes behavior beyond this phase. A multi-line `<style>` block yields no `@import` or `@font-face` records (the HTML walk splits the node by lines), so those references are not checked at all; fixing it adds new missing-target errors. `::file when=` warnings, unlike the errors this phase fixed, still use the prepared line. Redo cost: small for each, plus their new test expectations.
  - |-
    `cutover_reference`'s block-quote snapshot failed on Windows because the test was not hermetic, not because of a platform difference: the page deliberately renders content that paints no color at ambient detection, so the snapshot depended on the host's `COLORTERM` (it fails on macOS too without it). I pinned the depth in the test helper and left production alone; Phase 3 owns how the page carries its terminal profile. If Phase 3 changes that, the pin can go.
  - |-
    Windows `os_failing` was not set on the spec. The one remaining Windows failure, `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry`, also fails on macOS and is not OS-specific: `claudine/docs/research/sandboxing/providers/sandboxing.yaml` uses `enum({{ as_csv(…) }}; required)`, syntax that belongs to the planned `2026-11-05-dynamic-schema-types`. I left that file alone. `just test` was run with this one test skipped. Alternative: mark the file or test as expected-failing until that feature lands, so the full gate is green without a skip.
  - |-
    Other sessions shared this worktree during Phase 2. Commits efe02f230, 57b2a76e5, f224928e4, and c845ac498 landed mid-phase under the author's signed identity from outside this run; they hold three of this phase's four tasks, and 57b2a76e5 also holds another session's schema-layer reporting work. So this phase's file lists come from `git diff f85552280 HEAD` plus the working tree, minus that session's files, classified by reading the diffs, not from the uncommitted tree the prompt assumed. Excluded as foreign: schema validate and schema layer sources, tests, and fixtures, `docs/inline/schema-validation.md`, `docs/topics/schemas/definition.md`, the loading-data spec and spikes, and an untracked `simplified-schema/docs/topics/using-prose-in-schemas.md`. A reviewer who wants one commit per concern would need to split 57b2a76e5.
  - |-
    Lifecycle `stdout:` actions and lifecycle command output now go to stderr in Markdown mode. They used to share stdout with the document. The spec and plan require it ("formatted lifecycle communication and shell output go to stderr"), though they also say "as today", which was not true of `stdout:`. As a consequence, a piped `stdout:` line is now prose-rendered (`hello **bold**` prints `hello bold`) rather than copied as authored. Alternative: keep `stdout:` on stdout, which breaks the rule that stdout holds only the deliverable. Redo cost: one method in `TerminalEffects` plus two tests. The homelab and claudine recipes use no lifecycle.
  - |-
    `md render --output markdown` or a data output, with no `--width` and no known width, keeps the source text. The plan's literal rules resolve that to `Unlimited`, which joins every paragraph, but the spec also says this output "prints the document's source text" and today's bytes are the source. With `OUTPUT_COLUMNS` or a real terminal it still reflows, as `auto` says. Explicit `--width unlimited` joins. Alternative: strict `auto`, which changes every `md render --output markdown > file`. Redo cost: one function in `commands/render.rs`.
  - |-
    An envelope's `document` omits the final newline `println!` adds to Markdown stdout, so "byte for byte" means `stdout == document + "\n"`. This matches what `report-json` carried. Alternative: include the newline in `document`. Redo cost: one line plus test helpers.
  - |-
    `--show` behavior left open by the spec. Data outputs print to stdout and also open the file, because a data output is always one value on stdout. `md render --output markdown|markdown-plus|html --show` opens and does not print, as today. `md render --show` for `terminal`/`plain` prints and opens the document's Markdown, because terminal text has no useful file form. On a failed data run, `--show` still writes and opens the failure payload (the spec says "the value"); this ends the old rule that a failed composition publishes no viewer file, for data mode only. Opener failure now fails the run in every mode, where it used to print and exit 0. Alternative: no viewer on failure. Redo cost: one condition in `ReportSink::finish_with`.
  - |-
    The opener seam is `artifact.rs::ArtifactOpener` with `ProcessOpener` reading the existing `MD_DRY_RUN`, plus a new value `MD_DRY_RUN=fail` that simulates no default app, so a spawned test can prove the one failure payload. A test hook in a production env var already existed; this adds one value. Alternative: unit tests of the seam only, losing the end-to-end proof. Redo cost: small.
  - |-
    A data output disables interactive shell approval: a command that would prompt fails with `ApprovalRequired`, because stderr must stay empty. `--debug`/`RUST_LOG` tracing is not initialized in data mode either. Alternative: prompt on `/dev/tty`. Redo cost: one condition.
  - |-
    `--width` is declared once as a global root flag, not per leaf as rule 22 words it. So in this phase commands not yet wired (`md clean`, `md toc`, ...) accept and ignore it, ahead of Phase 4. Global gives "no opt-outs" with one declaration, and it also covers the utility routes the rule says accept and ignore it. Phase 4's table-driven metadata still owns `--output`. Alternative: a per-leaf argument. Redo cost: a flag in each leaf.
  - |-
    `--max-width` moved from the root `Cli` onto `RenderOptions`, because the spec's acceptance names `md render --max-width` and that form exited 2 (the flag was root-only). Global would have put it on every command, which the spec forbids. `md FILE --max-width` and `md --max-width N render FILE` still work. The explicit subcommand's value is applied after `style:` frontmatter, so it wins as the root flag does. Redo cost: small; `2026-11-05-render-surface` moves the whole style surface anyway.
  - |-
    `md compose` never read `fixed-width:`/`unwrap:` frontmatter; its pipeline sets explicit cleanup values on each run. So the spec's "the document's own `fixed-width:` and `unwrap:` frontmatter still applies" describes `md clean`, not compose. `ComposeOptions::with_output_width` therefore overrides nothing for compose in practice, and the `--width source` frontmatter tests can only prove authored newlines stay. Left unchanged: reading that frontmatter in compose would break the no-`--width` byte promise. `md clean` is wired in Phase 4.
  - |-
    `md schema about` must stay styled on a pipe, which it only got from the width-selects-rich-profile leak (P2) that this phase removes. It now sets an explicit profile: detected on a TTY, optimistic otherwise. Phase 4 moves it to the shared selector, where the spec says it degrades to plain on a pipe; delete this then.
  - |-
    Removing the P2 leak changes two other paths. `claudine`'s rendering on a pipe loses styling that came only from setting a width. A framed `DarkmatterPage` with no profile now styles from its captured terminal rather than `new_optimistic`, so its bytes no longer depend on the host. `HyperlinkMode::Always` now really forces OSC 8 and `ItalicMode` is honored, both as their docs already said. A profile with graphics on still reads `Terminal::default()` for the app, route, and cell size, because a profile cannot name a protocol.
  - |-
    `markdown_title()`'s "multiple H1 headings" notice moved from `eprintln!` to `tracing::warn!`, because a library write to stderr breaks a data output's empty stderr. So a formatted run no longer shows it unless tracing is on. Alternative: thread it into `ComposeWarning`, which needs a warning sink through the function context. Redo cost: moderate.
  - |-
    `md render`'s unknown and deprecated `style:` keys are now reported: a warning row on stderr in a page output, `dm.style.unknown_key` / `dm.style.deprecated` in a data output's diagnostics. They were tracing-only before, so a document with an unknown key now prints a stderr row on each page render. `--output markdown` builds no page and reports nothing, as today; a data output runs the same check and `--deny style` fails it, but unparseable `style:` is not a failure there, because the source text it carries never reads it. Alternative: treat a data output like `--output markdown` and skip the check. Redo cost: one call in `commands/render.rs`.
  - |-
    `DocumentOutput::prepare` renders exactly one target, and overrides return only what was prepared; for any other target they return `None` and fall to the generic fold. The plan describes storing "validated inputs"; rendering once eagerly is the simpler way to keep "overrides never swallow rendering errors". `document_markdown` does not strip embedded metadata; compose strips it itself, because `md render` prints the source as it stands.
  - |-
    A refused reference in a lifecycle run reported an extra generic `dm.compose.failure` row with a null source that the same document without a lifecycle did not have. It is now classified with no diagnostics, so both routes report the same rows.
  - |-
    `claudine/cli/tests/level2/level2_typed_error_render_capture.rs` still asserted the `[text](url)` fallback Phase 1 removed; Phase 1 and 2 did not run L2, so it was stale. Updated to the non-clickable rule. Other L2 suites are still unexecuted until Phase 6 and may hold more of these.
  - |-
    `md rm` prints nothing on `terminal` and `plain` and reports on `markdown`, `markdown-plus`, and `html`.
    The spec gives `md rm` the `terminal` default and every family, and also says saves keep their existing no-output behavior. I kept the silence for the two terminal outputs (with `-v` still a stderr summary), and an explicit Markdown or HTML output, which someone asked for by name, gets one sentence.
    Alternative: print the summary on stdout for all formatted outputs, which is more uniform but changes the default for scripts. Redoing it is a one-line change in `rm_value`, plus one test.
  - |-
    `md clean`'s run-level diagnostics are in a new `run_diagnostics` field.
    Plan rule 23 says an existing object gains `outcome` and `diagnostics` when a run needs them, and rule 23 also says clean's own `diagnostics` must stay intact. They collide, because version 1 of clean already has a `diagnostics` array with a different row shape. I kept the published field and put a run's records (a fractional `--width`, a schema-load failure) in `run_diagnostics`.
    Alternative: bump clean's data to version 2 and rename. Redoing it touches `DataShape::status`, `clean.yaml`, and a few tests.
  - |-
    `md get` and a single or merged `md schema detect` wrap as `{ outcome, diagnostics, value }` whenever a run has something to say, even when the value is an object.
    Rule 23 lets an existing object gain the two fields directly, but these objects' keys are the user's: a frontmatter property called `outcome` would be overwritten. A clean success stays the bare value. The cost is that a script reading `md get doc.md a b` must check for `outcome` only on a warning or failure, which it must do anyway.
  - |-
    Most non-document commands' `markdown` and `html` outputs are simple projections, not bespoke designs.
    The spec requires them but does not design them. `md toc` is a nested list of links, `md schema about` is a reference built from its data, `md schema validate` and `md schema triggers` parse their plain report as Markdown (its rows are already list items), and `md delta`, `md graph`'s tree, and a saved `md clean`'s delta are a plain code block. A file name holding Markdown punctuation can therefore read oddly in `md schema validate --output markdown`, though nothing is lost in the terminal or data outputs. Better layouts are additive: each lives in one function.
  - |-
    Two spec-accepted-argument conflicts became usage errors that the spec does not list: `md hash <dir>` with `--save`, `--diff`, or a structured or detailed kind, and a bad `md code-block --highlight`. Both exited 1 and their own comments already called them usage errors; plan rule 27 and the Code results task ask for this. `--save` with stdin stays a runtime failure (exit 1), because whether an argument is stdin is decided the same way as any other failure to open, and the spec's table leaves it alone. Moving it is a small change in each of `clean`, `set`, and `hash`.
  - |-
    `--show` is gone from `md code-block` rather than preserved. The plan's Code results task mentions show handling, but the code read `cli.render.show`, and `md --show code-block x` has always been refused as a subcommand combined with a top-level option, so that branch could never run. The spec lists `--show` only for `md render` and `md compose`. Adding a real `md code-block --show` would be a small feature, built the way `md render` does it.
  - |-
    `md schema validate` failing on a file reports `outcome: failure` with an empty run-level `diagnostics`; the problems stay in each file's record.
    The spec says the records tell the failures apart, and copying each into a second list would duplicate them. Only an invocation-level problem (an unloadable baseline, `dm.schema.baseline`) has a run diagnostic. An alternative is to project every file's failure into `diagnostics`, as `md validate refs` does, which would make the two commands read alike; that is a small addition in `validate_all`.
  - |-
    `md schema validate -vv --output json` carries no timings.
    Formerly they went to stderr, which a data output must leave empty. The spec moves `--perf` timings into `perf` for compose only and says `-v` output disappears, so they are dropped rather than given a field this schema does not define.
  - |-
    Error blocks are made escape-free on stderr, and inside a plain report, by stripping after they render (`stderr_text`, `pretty_report`).
    `StatusBlock` paints regardless of the terminal it is given, which broke "plain never holds an ESC byte" for every error `main` prints. Fixing `StatusBlock` itself is the cleaner fix, but other CLIs and many snapshots rely on it, and this fix's scope leaves other CLIs' output alone. If a reviewer wants the component fixed, these two strip calls are removed in that change.
  - |-
    The `md edit` conformance success cases are `#[cfg(unix)]`, using a shell script as `EDITOR`.
    The editor has to write to the file, and the shared argv recorder does not, so a native stub would be a new test binary. The failure case runs on every OS, and `run_edit` has no OS-specific code of its own. Nightly Windows therefore proves only the failure path. Adding a writing mode to the recorder would close this.
  - |-
      When a report's rows cannot fit in the requested width, they now stack instead of being replaced by a notice.
      The plan did not cover this; the documentation audit found it. `md validate refs --width 30`, or one long reference at `--width 60`, printed "Table could not be rendered" where the rows belonged, and every location was lost. Any borderless table in the terminal fold now writes each cell on its own line, and only the first cell wraps, so a reference prints exactly as written and may overflow. Bordered tables keep the notice.
      Alternatives: give the report's columns break-anywhere wrapping, which inserts hyphens that are not in the reference and misstates it; or keep the notice and document it, which breaks the promise that a location is always visible. Redoing it means replacing one match arm and `emit_stacked_rows` in biscuit-terminal's `render_tree/render.rs` and one test.
  - |-
      A TOML output writes an integer above the signed 64-bit range as its decimal text.
      The plan says TOML preserves the command's data subject to the null and root rules, but `md toc --output toml` always failed, because its unsigned hashes do not fit a TOML integer. So one field can be an integer or a string depending on its value, which a typed TOML reader must allow for.
      Alternatives: keep the failure and document it, so a declared output never works for `md toc`; write every hash as text in all formats, which breaks today's JSON; or write them as floats, which silently loses digits. Redoing it means removing one match arm in biscuit-terminal's `output/data.rs` and its two tests.
  - |-
      A file argument that fails to open now reports its class in a data output: `failure` and `reference` on the `dm.input.read_failure` diagnostic.
      The plan's documentation task tells scripts to read the diagnostic's `failure` field, but this one case did not have it, so `md compose nofile.md --output json` could not say why. It now does for every command, using the fields reference diagnostics already have. `md schema validate` keeps its own per-file record. `md schema detect`'s formatted error still has no `failure:` row; the docs state that.
      Alternative: a separate `input_details` object, which would be more exact but adds a field to all 17 schemas for one case. Redoing it touches `DocumentArgumentError::diagnostic` and one test.
  - |-
      Documentation added beyond the plan's list: `darkmatter/docs/cli/developing.md` holds the source layout and test-fixture notes cut from the CLI README, and `darkmatter/docs/errors/composition-problems.md` holds the per-family stopping rules and authored positions, because no existing page fit both. `docs/cli-best-practices.md` was also corrected outside section 2.1 where other sections contradicted it. `cli/README.md` went from 438 lines to 67.
      Alternative: fold the two new pages into existing ones, which is a move plus link updates.
  - |-
      The component bridge `Terminal::from_output_options` still gives components 1,048,576 columns under `unlimited` and `source`; I left it and documented the limit.
      The plan says width carries no sentinel. The render-tree fold honors that, but a filling component rendered directly through this bridge would print about a million glyphs. No shipped `md` command I ran reaches it: output sizes under `--width unlimited` stayed normal for toc, delta, graph, code-block, schema about/validate/triggers, hash, and render. Fixing it means making a component's width an enum rather than a number, across the component trait, which is much larger than this documentation phase. If the reviewer wants it, it is one bridge plus each filling component.
  - |-
      A set `$EDITOR` or `$VISUAL` that names a missing program now fails `md edit` instead of opening a probed editor. The Phase 5 list called the silent fallthrough a defect; the plan did not settle it.
      `$EDITOR` still gives way to a valid `$VISUAL`, which a test and the documentation pinned. A blank variable counts as unset. This also changes Claudine's prompt editing, which shares `resolve_editor_command`.
      Alternative: keep falling through and print a warning, which still opens an editor the user did not choose. Redoing it means restoring the probe in that one function.
  - |-
      `render_terminal_output` now returns its diagnostics (`Rendered<String>`) and `render_output` carries them, but `md` does not print them. The Phase 5 list named the dropped diagnostics as a defect.
      I tried writing them to stderr, and `md validate refs --output markdown` then warned "inline span style has no plain Markdown equivalent" about the command's own report, which a user cannot act on.
      Alternative: forward them only for a document the user authored (`md render`). That needs a flag on `CommandValue`; the library side would not change.
  - |-
      A reference and a location in the report are now never split: their columns do not wrap, so a width that cannot hold a row stacks that group. The spec did not state this.
      Without it the report broke `./missing-one.md` at the hyphen in a narrow table. It also found a planner bug: a borderless table reserved a boxed table's overhead, so the split happened at every numeric width, even 200.
      Alternative: allow wrapping again and fix only the overhead, which gives more rows in columns at narrow widths and splits references there. Redoing it is one `with_word_wrap` per column in `rows_table`.
  - |-
      `md schema about` has no minimum width now. It kept a 64-column floor; `--width 30` must mean 30. A table that cannot fit becomes a list naming each column, for this command only.
      Alternative: stack in the shared `Table` component, so every bordered table gains it. That is the larger fix and changes other commands' output; I left it because the spec does not ask for it.
  - |-
      `md validate refs` still aborts with `interpolation failed` on a document whose inline code holds an expression that does not parse, such as `` `{{ a b }}` ``. Phase 5 listed this as a defect; I left it, because `md compose` rejects the same document with the same error, so the check tells the truth about what composition will do.
      Alternative: make the check defer parse errors and report the references anyway, which would then pass a document composition cannot build.
  - |-
      The summary `5 ok, 5 broken` and the data field `references_valid: 4` can differ on the same report, because a warning-severity reference counts as ok in the summary's arithmetic but not in the field, whose meaning the plan keeps from today's wire shape.
      I changed nothing. Alternative: count warnings apart in the summary (`4 ok, 5 broken, 1 warning`), which is one formatting change in the report and its pinned tests.
  - |-
      `every_kinded_schema_artifact_passes_its_definition_through_the_registry` still fails, and the gates ran with it skipped, as in every phase. It rejects `claudine/docs/research/sandboxing/providers/sandboxing.yaml`, an in-progress research schema with `{{ as_csv(...) }}` inside an enum. It is unrelated to this fix and not mine to rewrite.
      It will fail CI's darkmatter cell until the author changes that file or excludes it from the corpus.
source_files_during_phase_2:
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/render_tree/link.rs
  - biscuit-terminal/lib/src/render_tree/mod.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - claudine/lib/src/composition/lifecycle/tests/call_check.rs
  - darkmatter/cli/src/args/policy.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/compose/lifecycle.rs
  - darkmatter/cli/src/commands/compose/report.rs
  - darkmatter/cli/src/commands/graph.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/validate.rs
  - darkmatter/cli/tests/l1/clean_parity.rs
  - darkmatter/cli/tests/l1/compose_report_diagnostics.rs
  - darkmatter/cli/tests/l1/compose_report_json.rs
  - darkmatter/cli/tests/l1/graph.rs
  - darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
  - darkmatter/cli/tests/l1/link_titles.rs
  - darkmatter/cli/tests/l1/magic_root.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/cli/tests/l1/reference_report.rs
  - darkmatter/dmls/tests/l1/collection_editor.rs
  - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
  - darkmatter/features/_completed/2026-06-17-cli-atheist/baseline/json/graph_validate.json
  - darkmatter/features/_completed/2026-06-17-cli-atheist/baseline/json/validate_refs_errors.json
  - darkmatter/features/_completed/2026-06-17-cli-atheist/baseline/json/validate_refs_local.json
  - darkmatter/lib/src/markdown/cleanup/document.rs
  - darkmatter/lib/src/markdown/cleanup/link_titles.rs
  - darkmatter/lib/src/markdown/cleanup/mod.rs
  - darkmatter/lib/src/markdown/cleanup/structure.rs
  - darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs
  - darkmatter/lib/src/markdown/compose/body_origin.rs
  - darkmatter/lib/src/markdown/compose/call_precheck.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/link_title_warnings.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/policy.rs
  - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
  - darkmatter/lib/src/markdown/compose/site_scope.rs
  - darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
  - darkmatter/lib/src/markdown/compose/transclusion/conditions.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/reference/css.rs
  - darkmatter/lib/src/markdown/reference/file_tree/model.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/html.rs
  - darkmatter/lib/src/markdown/reference/local.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/report.rs
  - darkmatter/lib/src/markdown/reference/snapshot.rs
  - darkmatter/lib/src/markdown/reference/types.rs
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/src/markdown/render_tree/build_context.rs
  - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/render/image_ref.rs
  - darkmatter/lib/src/render/link.rs
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/tests/browser/browser_render.rs
  - darkmatter/lib/tests/common/clean_parity/mod.rs
  - darkmatter/lib/tests/fixtures/composition_errors/child.md
  - darkmatter/lib/tests/fixtures/composition_errors/leading-rule.md
  - darkmatter/lib/tests/fixtures/composition_errors/root.md
  - darkmatter/lib/tests/fixtures/composition_errors/when-error.md
  - darkmatter/lib/tests/l1/collection_analysis.rs
  - darkmatter/lib/tests/l1/collection_blocks/provenance.rs
  - darkmatter/lib/tests/l1/collection_discovery.rs
  - darkmatter/lib/tests/l1/cutover_reference.rs
  - darkmatter/lib/tests/l1/diagnostic_projection.rs
  - darkmatter/lib/tests/l1/error_snapshots/transclusion.rs
  - darkmatter/lib/tests/l1/kind_semantic_checks.rs
  - darkmatter/lib/tests/l1/link_title_migration.rs
  - darkmatter/lib/tests/l1/link_title_warnings.rs
  - darkmatter/lib/tests/l1/link_titles.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/l1/prelude_exports.rs
  - darkmatter/lib/tests/l1/reference_provenance.rs
  - darkmatter/lib/tests/l1/reference_report.rs
  - darkmatter/lib/tests/l1/report_diagnostics.rs
  - darkmatter/lib/tests/l1/shared_diagnostics.rs
  - darkmatter/lib/tests/l1/style_features_baseline.rs
  - darkmatter/lib/tests/l1/style_features_phase5.rs
  - darkmatter/lib/tests/l1/tree_features_characterization.rs
  - renderable/src/output/link_metadata.rs
  - renderable/src/tree/attrs.rs
  - renderable/src/tree/render/browser.rs
  - renderable/src/tree/render/markdown.rs
docs_updated_during_phase_2:
  - darkmatter/docs/structs/Markdown.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/cleanup.md
  - .claude/skills/darkmatter/compose.md
  - .claude/skills/darkmatter/testing.md
  - .claude/skills/os/SKILL.md
source_files_during_phase_3:
  - biscuit-terminal/lib/src/output/mod.rs
  - biscuit-terminal/lib/src/output/width.rs
  - claudine/cli/tests/level2/level2_typed_error_render_capture.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/render/final_message.rs
  - claudine/lib/src/render/prompt/formatting.rs
  - darkmatter/cli/schemas/output/compose.yaml
  - darkmatter/cli/schemas/output/render.yaml
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/args/enums.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/args/output.rs
  - darkmatter/cli/src/args/parsers.rs
  - darkmatter/cli/src/args/render.rs
  - darkmatter/cli/src/artifact.rs
  - darkmatter/cli/src/collection_failure.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/compose/lifecycle.rs
  - darkmatter/cli/src/commands/compose/report.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/src/lib.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/cli/src/output/mod.rs
  - darkmatter/cli/src/output/report.rs
  - darkmatter/cli/src/render.rs
  - darkmatter/cli/src/style_claims.rs
  - darkmatter/cli/tests/common/fixture.rs
  - darkmatter/cli/tests/common/protected_env.rs
  - darkmatter/cli/tests/l1/collection_blocks.rs
  - darkmatter/cli/tests/l1/compose_basic.rs
  - darkmatter/cli/tests/l1/compose_call_warnings.rs
  - darkmatter/cli/tests/l1/compose_collection_controls.rs
  - darkmatter/cli/tests/l1/compose_data_output.rs
  - darkmatter/cli/tests/l1/compose_lifecycle.rs
  - darkmatter/cli/tests/l1/compose_report_diagnostics.rs
  - darkmatter/cli/tests/l1/compose_report_json.rs
  - darkmatter/cli/tests/l1/compose_schema_enhancements.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/cli/tests/l1/compose_unknown_identifiers.rs
  - darkmatter/cli/tests/l1/compose_value_provenance.rs
  - darkmatter/cli/tests/l1/context_construction_guard.rs
  - darkmatter/cli/tests/l1/feature_flag.rs
  - darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
  - darkmatter/cli/tests/l1/layout_flags.rs
  - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
  - darkmatter/cli/tests/l1/link_titles.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/cli/tests/l1/mermaid_features.rs
  - darkmatter/cli/tests/l1/output_envelope.rs
  - darkmatter/cli/tests/l1/policy_verbs.rs
  - darkmatter/cli/tests/l1/reference_report.rs
  - darkmatter/cli/tests/l1/render_basic.rs
  - darkmatter/cli/tests/l1/schema_recursive.rs
  - darkmatter/lib/benches/phase8_render.rs
  - darkmatter/lib/benches/render_code_heavy.rs
  - darkmatter/lib/benches/render_pipeline.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/error.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/page/tests.rs
  - darkmatter/lib/src/lifecycle/md_host.rs
  - darkmatter/lib/src/markdown/code_block.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
  - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/output/code_block.rs
  - darkmatter/lib/src/markdown/output/command/document.rs
  - darkmatter/lib/src/markdown/output/command/mod.rs
  - darkmatter/lib/src/markdown/output/command/tests.rs
  - darkmatter/lib/src/markdown/output/mod.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
  - darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/cli_claims.rs
  - darkmatter/lib/tests/l1/blockquote_list_spacing.rs
  - darkmatter/lib/tests/l1/code_link.rs
  - darkmatter/lib/tests/l1/cutover_reference.rs
  - darkmatter/lib/tests/l1/horizontal_rule_integration.rs
  - darkmatter/lib/tests/l1/horizontal_rule_snapshots.rs
  - darkmatter/lib/tests/l1/layout_snapshots.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/l1/mermaid_automatic_mode.rs
  - darkmatter/lib/tests/l1/mermaid_syntax_highlighting.rs
  - darkmatter/lib/tests/l1/plain_corpus.rs
  - darkmatter/lib/tests/l1/prose_wrap_parity.rs
  - darkmatter/lib/tests/l1/snapshots/l1__cutover_reference__ref_page_margin_and_padding_terminal.snap
  - darkmatter/lib/tests/l1/snapshots/l1__plain_corpus__snapshot_of_the_plain_corpus.snap
  - darkmatter/lib/tests/l1/snapshots/l1__plain_corpus__snapshot_of_the_styled_corpus.snap
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_list_item_center_alignment_terminal.snap
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_list_item_right_alignment_terminal.snap
  - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
  - simplified-schema/docs/topics/schema-object.md
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/schema.md
  - .claude/skills/darkmatter/terminal.md
source_files_during_phase_4:
  - biscuit-terminal/lib/src/output/profile.rs
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/args/completion.rs
  - darkmatter/cli/src/args/enums.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/args/output.rs
  - darkmatter/cli/src/args/parsers.rs
  - darkmatter/cli/src/args/target.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/clean/json.rs
  - darkmatter/cli/src/commands/code_block.rs
  - darkmatter/cli/src/commands/frontmatter.rs
  - darkmatter/cli/src/commands/graph.rs
  - darkmatter/cli/src/commands/hash.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/src/commands/schema/detect.rs
  - darkmatter/cli/src/commands/schema/mod.rs
  - darkmatter/cli/src/commands/schema/triggers.rs
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/cli/src/commands/trigger_findings.rs
  - darkmatter/cli/src/commands/validate.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/cli/src/output/mod.rs
  - darkmatter/cli/src/output/report.rs
  - darkmatter/cli/tests/common/mod.rs
  - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
  - darkmatter/cli/tests/l1/clean.rs
  - darkmatter/cli/tests/l1/clean_fidelity.rs
  - darkmatter/cli/tests/l1/clean_frontmatter.rs
  - darkmatter/cli/tests/l1/clean_json.rs
  - darkmatter/cli/tests/l1/clean_parity.rs
  - darkmatter/cli/tests/l1/clean_rules.rs
  - darkmatter/cli/tests/l1/code_block.rs
  - darkmatter/cli/tests/l1/compose_data_output.rs
  - darkmatter/cli/tests/l1/delta.rs
  - darkmatter/cli/tests/l1/diagnostic_positions.rs
  - darkmatter/cli/tests/l1/entry_point_parity.rs
  - darkmatter/cli/tests/l1/frontmatter_key_order.rs
  - darkmatter/cli/tests/l1/get_set_rm.rs
  - darkmatter/cli/tests/l1/graph.rs
  - darkmatter/cli/tests/l1/hash_entry_precedence.rs
  - darkmatter/cli/tests/l1/hash_kind_save_diff.rs
  - darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
  - darkmatter/cli/tests/l1/lifecycle_schema_fallback.rs
  - darkmatter/cli/tests/l1/magic_root.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/cli/tests/l1/policy_verbs.rs
  - darkmatter/cli/tests/l1/reference_report.rs
  - darkmatter/cli/tests/l1/rm.rs
  - darkmatter/cli/tests/l1/schema_about.rs
  - darkmatter/cli/tests/l1/schema_detect.rs
  - darkmatter/cli/tests/l1/schema_grammar_types.rs
  - darkmatter/cli/tests/l1/schema_recursive.rs
  - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
  - darkmatter/cli/tests/l1/schema_target_recognition.rs
  - darkmatter/cli/tests/l1/schema_triggers.rs
  - darkmatter/cli/tests/l1/schema_union_matrix.rs
  - darkmatter/cli/tests/l1/schema_union_pipeline.rs
  - darkmatter/cli/tests/l1/schema_validate.rs
  - darkmatter/cli/tests/l1/schema_validate_baseline.rs
  - darkmatter/cli/tests/l1/schema_validate_baseline_selection.rs
  - darkmatter/cli/tests/l1/schema_validate_core_types.rs
  - darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs
  - darkmatter/cli/tests/l1/schema_validate_timings.rs
  - darkmatter/cli/tests/l1/schema_validate_yaml.rs
  - darkmatter/cli/tests/l1/standalone_baseline.rs
  - darkmatter/cli/tests/l1/toc.rs
  - darkmatter/cli/tests/l1/validate_refs.rs
  - darkmatter/lib/src/markdown/output/command/document.rs
  - darkmatter/lib/src/markdown/output/command/mod.rs
  - darkmatter/cli/schemas/output/clean.yaml
  - darkmatter/cli/schemas/output/code-block.yaml
  - darkmatter/cli/schemas/output/delta.yaml
  - darkmatter/cli/schemas/output/edit.yaml
  - darkmatter/cli/schemas/output/get.yaml
  - darkmatter/cli/schemas/output/graph.yaml
  - darkmatter/cli/schemas/output/hash.yaml
  - darkmatter/cli/schemas/output/rm.yaml
  - darkmatter/cli/schemas/output/schema-about.yaml
  - darkmatter/cli/schemas/output/schema-detect.yaml
  - darkmatter/cli/schemas/output/schema-triggers.yaml
  - darkmatter/cli/schemas/output/schema-validate.yaml
  - darkmatter/cli/schemas/output/set.yaml
  - darkmatter/cli/schemas/output/toc.yaml
  - darkmatter/cli/schemas/output/validate-refs.yaml
  - darkmatter/cli/src/commands/delta.rs
  - darkmatter/cli/src/commands/toc.rs
  - darkmatter/cli/src/output/command.rs
  - darkmatter/cli/tests/l1/output_commands.rs
  - darkmatter/lib/src/markdown/output/command/clean.rs
  - darkmatter/lib/src/markdown/output/command/data.rs
  - darkmatter/lib/src/markdown/output/command/schema_about.rs
  - darkmatter/lib/src/markdown/output/command/value.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/cli.md
  - .claude/skills/darkmatter/schema.md
source_files_during_phase_5:
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/output/data.rs
  - biscuit-terminal/lib/src/output/mod.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/commands/schema/detect.rs
  - darkmatter/cli/src/io/mod.rs
  - darkmatter/cli/src/output/report.rs
  - darkmatter/cli/tests/l1/output_commands.rs
  - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
  - darkmatter/lib/tests/l1/reference_report.rs
  - just/review.just
  - simplified-schema/lib/src/validation.rs
docs_updated_during_phase_5:
  - biscuit-terminal/README.md
  - biscuit-terminal/docs/components/prose.md
  - biscuit-terminal/docs/render-tree.md
  - biscuit-terminal/lib/README.md
  - darkmatter/README.md
  - darkmatter/cli/README.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/docs/cli/clean.md
  - darkmatter/docs/cli/compose.md
  - darkmatter/docs/cli/delta.md
  - darkmatter/docs/cli/get.md
  - darkmatter/docs/cli/graph.md
  - darkmatter/docs/cli/hash.md
  - darkmatter/docs/cli/index.md
  - darkmatter/docs/cli/policy.md
  - darkmatter/docs/cli/render.md
  - darkmatter/docs/cli/rm.md
  - darkmatter/docs/cli/set.md
  - darkmatter/docs/cli/toc.md
  - darkmatter/docs/dependencies.md
  - darkmatter/docs/errors/README.md
  - darkmatter/docs/errors/file-reference-failures.md
  - darkmatter/docs/inline/call-check.md
  - darkmatter/docs/inline/filter.md
  - darkmatter/docs/inline/link-validation.md
  - darkmatter/docs/inline/map.md
  - darkmatter/docs/inline/preflight-checks.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/rendering/disclosure.md
  - darkmatter/docs/rendering/mermaid.md
  - darkmatter/docs/rendering/popover.md
  - darkmatter/docs/structs/Link.md
  - darkmatter/docs/topics/compose-requests.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/delta.md
  - darkmatter/docs/topics/hyperlinking.md
  - darkmatter/docs/topics/image-previews.md
  - darkmatter/docs/topics/lifecycle/error-handling.md
  - darkmatter/docs/topics/lifecycle/index.md
  - darkmatter/docs/topics/markdown-formatting.md
  - darkmatter/docs/topics/output-formats.md
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/topics/schemas/dmls-schema-support.md
  - darkmatter/docs/topics/schemas/schema-triggers.md
  - docs/cli-best-practices.md
  - docs/dependencies.md
  - renderable/README.md
  - renderable/docs/tree-rendering.md
docs_created_during_phase_5:
  - biscuit-terminal/docs/output.md
  - darkmatter/docs/cli/code-block.md
  - darkmatter/docs/cli/data-contracts.md
  - darkmatter/docs/cli/developing.md
  - darkmatter/docs/cli/edit.md
  - darkmatter/docs/cli/output-and-width.md
  - darkmatter/docs/cli/schema.md
  - darkmatter/docs/cli/validate.md
  - darkmatter/docs/errors/composition-problems.md
  - renderable/docs/command-output.md
  - darkmatter/fixes/2026-11-04-composition-errors/implementation.md
skills_files_updated_during_phase_5:
  - .claude/skills/biscuit-terminal/SKILL.md
  - .claude/skills/biscuit-terminal/render-tree.md
  - .claude/skills/cli/SKILL.md
  - .claude/skills/cli/cli-best-practices.md
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/cleanup.md
  - .claude/skills/darkmatter/rendering.md
  - .claude/skills/renderable/SKILL.md
source_files_during_phase_6:
  - renderable/src/tree/render/markdown.rs
  - renderable/src/output/link_metadata.rs
  - biscuit-terminal/lib/src/components/prose/tokens.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/output/mod.rs
  - biscuit-terminal/lib/tests/l1/render_terminal_output.rs
  - biscuit-terminal/lib/tests/level2/level2_graphics_tmux.rs
  - biscuit-terminal/cli/tests/level2/level2_apple_terminal_prose.rs
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/policy.rs
  - darkmatter/lib/src/markdown/delta/mod.rs
  - darkmatter/lib/src/markdown/output/command/value.rs
  - darkmatter/lib/src/markdown/reference/report.rs
  - darkmatter/lib/tests/l1/link_titles.rs
  - darkmatter/lib/tests/l1/reference_report.rs
  - darkmatter/lib/tests/level2/level2_render_tree_terminal/code_panel.rs
  - darkmatter/cli/src/commands/code_block.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/frontmatter.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/src/commands/schema/triggers.rs
  - darkmatter/cli/src/render.rs
  - darkmatter/cli/tests/l1/delta.rs
  - darkmatter/cli/tests/l1/output_commands.rs
  - darkmatter/cli/tests/level2/level2_frontmatter_images.rs
  - darkmatter/lib/src/lifecycle/md_host.rs
  - darkmatter/lib/src/lifecycle/md_host/tests.rs
docs_updated_during_phase_6:
  - biscuit-terminal/docs/output.md
  - darkmatter/docs/cli/compose.md
  - darkmatter/docs/cli/edit.md
  - darkmatter/docs/cli/policy.md
  - darkmatter/docs/cli/render.md
  - darkmatter/docs/cli/validate.md
  - renderable/docs/command-output.md
  - darkmatter/fixes/2026-11-04-composition-errors/implementation.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/biscuit-terminal/render-tree.md
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/os/build-hosts.md
source_code:
  - Cargo.lock
  - biscuit-file/lib/src/json5/types.rs
  - biscuit-terminal/cli/src/commands/mermaid.rs
  - biscuit-terminal/cli/tests/level2/level2_apple_terminal_prose.rs
  - biscuit-terminal/lib/src/components/prose/markdown.rs
  - biscuit-terminal/lib/src/components/prose/mod.rs
  - biscuit-terminal/lib/src/components/prose/parity.rs
  - biscuit-terminal/lib/src/components/prose/prose.rs
  - biscuit-terminal/lib/src/components/prose/render.rs
  - biscuit-terminal/lib/src/components/prose/tokens.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/discovery/detection/color.rs
  - biscuit-terminal/lib/src/lib.rs
  - biscuit-terminal/lib/src/output/data.rs
  - biscuit-terminal/lib/src/output/mod.rs
  - biscuit-terminal/lib/src/output/profile.rs
  - biscuit-terminal/lib/src/output/width.rs
  - biscuit-terminal/lib/src/render_tree/link.rs
  - biscuit-terminal/lib/src/render_tree/mod.rs
  - biscuit-terminal/lib/src/render_tree/options.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/terminal.rs
  - biscuit-terminal/lib/src/utils/block_constraint.rs
  - biscuit-terminal/lib/tests/l1/level1_apple_terminal_prose.rs
  - biscuit-terminal/lib/tests/l1/main.rs
  - biscuit-terminal/lib/tests/l1/output_dispatch.rs
  - biscuit-terminal/lib/tests/l1/prose_destinations.rs
  - biscuit-terminal/lib/tests/l1/prose_grammar.rs
  - biscuit-terminal/lib/tests/l1/render_terminal_output.rs
  - biscuit-terminal/lib/tests/l1/render_tree_code_context.rs
  - biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs
  - biscuit-terminal/lib/tests/l1/tree_layout.rs
  - biscuit-terminal/lib/tests/level2/level2_graphics_tmux.rs
  - biscuit-visualized/src/src/components/mermaid.rs
  - biscuit-visualized/src/src/components/render_hook.rs
  - biscuit-visualized/src/tests/l1/display_size.rs
  - claudine/cli/tests/level2/level2_typed_error_render_capture.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/lifecycle/tests/call_check.rs
  - claudine/lib/src/render/final_message.rs
  - claudine/lib/src/render/prompt/formatting.rs
  - darkmatter/cli/schemas/output/clean.yaml
  - darkmatter/cli/schemas/output/code-block.yaml
  - darkmatter/cli/schemas/output/compose.yaml
  - darkmatter/cli/schemas/output/delta.yaml
  - darkmatter/cli/schemas/output/edit.yaml
  - darkmatter/cli/schemas/output/get.yaml
  - darkmatter/cli/schemas/output/graph.yaml
  - darkmatter/cli/schemas/output/hash.yaml
  - darkmatter/cli/schemas/output/render.yaml
  - darkmatter/cli/schemas/output/rm.yaml
  - darkmatter/cli/schemas/output/schema-about.yaml
  - darkmatter/cli/schemas/output/schema-detect.yaml
  - darkmatter/cli/schemas/output/schema-triggers.yaml
  - darkmatter/cli/schemas/output/schema-validate.yaml
  - darkmatter/cli/schemas/output/set.yaml
  - darkmatter/cli/schemas/output/toc.yaml
  - darkmatter/cli/schemas/output/validate-refs.yaml
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/args/completion.rs
  - darkmatter/cli/src/args/enums.rs
  - darkmatter/cli/src/args/mod.rs
  - darkmatter/cli/src/args/output.rs
  - darkmatter/cli/src/args/parsers.rs
  - darkmatter/cli/src/args/policy.rs
  - darkmatter/cli/src/args/render.rs
  - darkmatter/cli/src/args/target.rs
  - darkmatter/cli/src/artifact.rs
  - darkmatter/cli/src/collection_failure.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/src/commands/clean/json.rs
  - darkmatter/cli/src/commands/code_block.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/compose/lifecycle.rs
  - darkmatter/cli/src/commands/compose/report.rs
  - darkmatter/cli/src/commands/delta.rs
  - darkmatter/cli/src/commands/frontmatter.rs
  - darkmatter/cli/src/commands/graph.rs
  - darkmatter/cli/src/commands/hash.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/src/commands/schema/detect.rs
  - darkmatter/cli/src/commands/schema/mod.rs
  - darkmatter/cli/src/commands/schema/triggers.rs
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/cli/src/commands/toc.rs
  - darkmatter/cli/src/commands/trigger_findings.rs
  - darkmatter/cli/src/commands/validate.rs
  - darkmatter/cli/src/io/mod.rs
  - darkmatter/cli/src/lib.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/cli/src/output/command.rs
  - darkmatter/cli/src/output/mod.rs
  - darkmatter/cli/src/output/report.rs
  - darkmatter/cli/src/render.rs
  - darkmatter/cli/src/style_claims.rs
  - darkmatter/cli/tests/common/fixture.rs
  - darkmatter/cli/tests/common/mod.rs
  - darkmatter/cli/tests/common/protected_env.rs
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_valid/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_fileref/expected.pretty
  - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.pretty
  - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
  - darkmatter/cli/tests/l1/clean.rs
  - darkmatter/cli/tests/l1/clean_fidelity.rs
  - darkmatter/cli/tests/l1/clean_frontmatter.rs
  - darkmatter/cli/tests/l1/clean_json.rs
  - darkmatter/cli/tests/l1/clean_parity.rs
  - darkmatter/cli/tests/l1/clean_rules.rs
  - darkmatter/cli/tests/l1/code_block.rs
  - darkmatter/cli/tests/l1/collection_blocks.rs
  - darkmatter/cli/tests/l1/compose_basic.rs
  - darkmatter/cli/tests/l1/compose_call_warnings.rs
  - darkmatter/cli/tests/l1/compose_collection_controls.rs
  - darkmatter/cli/tests/l1/compose_data_output.rs
  - darkmatter/cli/tests/l1/compose_lifecycle.rs
  - darkmatter/cli/tests/l1/compose_report_diagnostics.rs
  - darkmatter/cli/tests/l1/compose_report_json.rs
  - darkmatter/cli/tests/l1/compose_schema_enhancements.rs
  - darkmatter/cli/tests/l1/compose_transclusion.rs
  - darkmatter/cli/tests/l1/compose_unknown_identifiers.rs
  - darkmatter/cli/tests/l1/compose_value_provenance.rs
  - darkmatter/cli/tests/l1/context_construction_guard.rs
  - darkmatter/cli/tests/l1/delta.rs
  - darkmatter/cli/tests/l1/diagnostic_positions.rs
  - darkmatter/cli/tests/l1/entry_point_parity.rs
  - darkmatter/cli/tests/l1/feature_flag.rs
  - darkmatter/cli/tests/l1/frontmatter_key_order.rs
  - darkmatter/cli/tests/l1/get_set_rm.rs
  - darkmatter/cli/tests/l1/graph.rs
  - darkmatter/cli/tests/l1/hash_entry_precedence.rs
  - darkmatter/cli/tests/l1/hash_kind_save_diff.rs
  - darkmatter/cli/tests/l1/invalid_trigger_warnings.rs
  - darkmatter/cli/tests/l1/layout_flags.rs
  - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
  - darkmatter/cli/tests/l1/lifecycle_schema_fallback.rs
  - darkmatter/cli/tests/l1/link_titles.rs
  - darkmatter/cli/tests/l1/magic_root.rs
  - darkmatter/cli/tests/l1/main.rs
  - darkmatter/cli/tests/l1/mermaid_features.rs
  - darkmatter/cli/tests/l1/output_commands.rs
  - darkmatter/cli/tests/l1/output_envelope.rs
  - darkmatter/cli/tests/l1/policy_verbs.rs
  - darkmatter/cli/tests/l1/reference_report.rs
  - darkmatter/cli/tests/l1/render_basic.rs
  - darkmatter/cli/tests/l1/rm.rs
  - darkmatter/cli/tests/l1/schema_about.rs
  - darkmatter/cli/tests/l1/schema_detect.rs
  - darkmatter/cli/tests/l1/schema_grammar_types.rs
  - darkmatter/cli/tests/l1/schema_recursive.rs
  - darkmatter/cli/tests/l1/schema_robustness_matrix.rs
  - darkmatter/cli/tests/l1/schema_target_recognition.rs
  - darkmatter/cli/tests/l1/schema_triggers.rs
  - darkmatter/cli/tests/l1/schema_union_matrix.rs
  - darkmatter/cli/tests/l1/schema_union_pipeline.rs
  - darkmatter/cli/tests/l1/schema_validate.rs
  - darkmatter/cli/tests/l1/schema_validate_baseline.rs
  - darkmatter/cli/tests/l1/schema_validate_baseline_selection.rs
  - darkmatter/cli/tests/l1/schema_validate_core_types.rs
  - darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs
  - darkmatter/cli/tests/l1/schema_validate_timings.rs
  - darkmatter/cli/tests/l1/schema_validate_yaml.rs
  - darkmatter/cli/tests/l1/standalone_baseline.rs
  - darkmatter/cli/tests/l1/toc.rs
  - darkmatter/cli/tests/l1/validate_refs.rs
  - darkmatter/cli/tests/level2/level2_frontmatter_images.rs
  - darkmatter/dmls/tests/l1/collection_editor.rs
  - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
  - darkmatter/features/_completed/2026-06-17-cli-atheist/baseline/json/graph_validate.json
  - darkmatter/features/_completed/2026-06-17-cli-atheist/baseline/json/validate_refs_errors.json
  - darkmatter/features/_completed/2026-06-17-cli-atheist/baseline/json/validate_refs_local.json
  - darkmatter/lib/benches/phase8_render.rs
  - darkmatter/lib/benches/render_code_heavy.rs
  - darkmatter/lib/benches/render_pipeline.rs
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/error.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/page/tests.rs
  - darkmatter/lib/src/lifecycle/md_host.rs
  - darkmatter/lib/src/lifecycle/md_host/tests.rs
  - darkmatter/lib/src/markdown/cleanup/document.rs
  - darkmatter/lib/src/markdown/cleanup/link_titles.rs
  - darkmatter/lib/src/markdown/cleanup/mod.rs
  - darkmatter/lib/src/markdown/cleanup/structure.rs
  - darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs
  - darkmatter/lib/src/markdown/code_block.rs
  - darkmatter/lib/src/markdown/compose/body_origin.rs
  - darkmatter/lib/src/markdown/compose/call_precheck.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/link_title_warnings.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/policy.rs
  - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/site_scope.rs
  - darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
  - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
  - darkmatter/lib/src/markdown/compose/transclusion/conditions.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/delta/mod.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/output/code_block.rs
  - darkmatter/lib/src/markdown/output/command/clean.rs
  - darkmatter/lib/src/markdown/output/command/data.rs
  - darkmatter/lib/src/markdown/output/command/document.rs
  - darkmatter/lib/src/markdown/output/command/mod.rs
  - darkmatter/lib/src/markdown/output/command/schema_about.rs
  - darkmatter/lib/src/markdown/output/command/tests.rs
  - darkmatter/lib/src/markdown/output/command/value.rs
  - darkmatter/lib/src/markdown/output/mod.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/markdown/reference/css.rs
  - darkmatter/lib/src/markdown/reference/file_tree/model.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/html.rs
  - darkmatter/lib/src/markdown/reference/local.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/report.rs
  - darkmatter/lib/src/markdown/reference/snapshot.rs
  - darkmatter/lib/src/markdown/reference/types.rs
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/src/markdown/render_tree/build_context.rs
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
  - darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/render/image_ref.rs
  - darkmatter/lib/src/render/link.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/style/cli_claims.rs
  - darkmatter/lib/tests/browser/browser_render.rs
  - darkmatter/lib/tests/common/clean_parity/mod.rs
  - darkmatter/lib/tests/fixtures/composition_errors/child.md
  - darkmatter/lib/tests/fixtures/composition_errors/leading-rule.md
  - darkmatter/lib/tests/fixtures/composition_errors/root.md
  - darkmatter/lib/tests/fixtures/composition_errors/when-error.md
  - darkmatter/lib/tests/l1/blockquote_list_spacing.rs
  - darkmatter/lib/tests/l1/code_link.rs
  - darkmatter/lib/tests/l1/collection_analysis.rs
  - darkmatter/lib/tests/l1/collection_blocks/provenance.rs
  - darkmatter/lib/tests/l1/collection_discovery.rs
  - darkmatter/lib/tests/l1/cutover_reference.rs
  - darkmatter/lib/tests/l1/diagnostic_projection.rs
  - darkmatter/lib/tests/l1/error_snapshots/transclusion.rs
  - darkmatter/lib/tests/l1/horizontal_rule_integration.rs
  - darkmatter/lib/tests/l1/horizontal_rule_snapshots.rs
  - darkmatter/lib/tests/l1/kind_semantic_checks.rs
  - darkmatter/lib/tests/l1/layout_snapshots.rs
  - darkmatter/lib/tests/l1/link_title_migration.rs
  - darkmatter/lib/tests/l1/link_title_warnings.rs
  - darkmatter/lib/tests/l1/link_titles.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/l1/mermaid_automatic_mode.rs
  - darkmatter/lib/tests/l1/mermaid_syntax_highlighting.rs
  - darkmatter/lib/tests/l1/mermaid_text_line_numbers.rs
  - darkmatter/lib/tests/l1/plain_corpus.rs
  - darkmatter/lib/tests/l1/prelude_exports.rs
  - darkmatter/lib/tests/l1/prose_wrap_parity.rs
  - darkmatter/lib/tests/l1/reference_provenance.rs
  - darkmatter/lib/tests/l1/reference_report.rs
  - darkmatter/lib/tests/l1/render_tree_hr_snapshots.rs
  - darkmatter/lib/tests/l1/render_tree_roundtrip.rs
  - darkmatter/lib/tests/l1/report_diagnostics.rs
  - darkmatter/lib/tests/l1/shared_diagnostics.rs
  - darkmatter/lib/tests/l1/snapshots/l1__cutover_reference__ref_page_margin_and_padding_terminal.snap
  - darkmatter/lib/tests/l1/snapshots/l1__plain_corpus__snapshot_of_the_plain_corpus.snap
  - darkmatter/lib/tests/l1/snapshots/l1__plain_corpus__snapshot_of_the_styled_corpus.snap
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_hyperlink_exact_width_terminal.snap
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_hyperlink_max_width_terminal.snap
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_list_item_center_alignment_terminal.snap
  - darkmatter/lib/tests/l1/snapshots/l1__tree_features_characterization__char_list_item_right_alignment_terminal.snap
  - darkmatter/lib/tests/l1/style_features_baseline.rs
  - darkmatter/lib/tests/l1/style_features_phase5.rs
  - darkmatter/lib/tests/l1/tree_features_characterization.rs
  - darkmatter/lib/tests/l1/yaml_block_parity.rs
  - darkmatter/lib/tests/level2/level2_render_tree_terminal/code_panel.rs
  - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
  - just/review.just
  - model-citizen/cli/src/output.rs
  - renderable/Cargo.toml
  - renderable/src/color/capability.rs
  - renderable/src/html/mod.rs
  - renderable/src/lib.rs
  - renderable/src/output/link_metadata.rs
  - renderable/src/output/mod.rs
  - renderable/src/tree/attrs.rs
  - renderable/src/tree/document.rs
  - renderable/src/tree/render/browser.rs
  - renderable/src/tree/render/markdown.rs
  - renderable/src/tree/validate.rs
  - renderable/tests/html_output_width.rs
  - renderable/tests/html_policy_comment_nodes.rs
  - renderable/tests/link_metadata_folds.rs
  - renderable/tests/markdown_container_line_endings.rs
  - renderable/tests/markdown_footnote_continuation.rs
  - renderable/tests/markdown_footnote_labels.rs
  - renderable/tests/markdown_html_contexts.rs
  - renderable/tests/markdown_line_endings_in_fields.rs
  - renderable/tests/markdown_literal_text.rs
  - renderable/tests/markdown_output_width.rs
  - renderable/tests/markdown_raw_html_payloads.rs
  - renderable/tests/markdown_single_line_raw_html.rs
  - renderable/tests/markdown_structural_breaks.rs
  - renderable/tests/markdown_table_cell_blocks.rs
  - renderable/tests/markdown_table_cell_pipes.rs
  - renderable/tests/markdown_unsupported_placeholder.rs
  - renderable/tests/support/adjacent_code_matrix.rs
  - simplified-schema/docs/topics/schema-object.md
  - simplified-schema/lib/src/validation.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/recent_commits_prose_layout.rs
  - sniff/cli/tests/l1/cli.rs
  - sniff/cli/tests/level2/level2_git_status_styling.rs
  - sniff/cli/tests/level2/level2_recent_commits_rendering.rs
documentation:
  - .claude/skills/biscuit-terminal/SKILL.md
  - .claude/skills/biscuit-terminal/image-rendering.md
  - .claude/skills/biscuit-terminal/render-tree.md
  - .claude/skills/cli/SKILL.md
  - .claude/skills/cli/cli-best-practices.md
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/cleanup.md
  - .claude/skills/darkmatter/cli.md
  - .claude/skills/darkmatter/compose.md
  - .claude/skills/darkmatter/rendering.md
  - .claude/skills/darkmatter/schema.md
  - .claude/skills/darkmatter/terminal.md
  - .claude/skills/darkmatter/testing.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/build-hosts.md
  - .claude/skills/renderable/SKILL.md
  - .claude/skills/renderable/tree.md
  - .claude/skills/worktree/list.md
  - biscuit-terminal/README.md
  - biscuit-terminal/docs/components/prose.md
  - biscuit-terminal/docs/components/terminal_image.md
  - biscuit-terminal/docs/output.md
  - biscuit-terminal/docs/render-tree.md
  - biscuit-terminal/lib/README.md
  - darkmatter/README.md
  - darkmatter/cli/README.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/docs/cli/clean.md
  - darkmatter/docs/cli/code-block.md
  - darkmatter/docs/cli/compose.md
  - darkmatter/docs/cli/data-contracts.md
  - darkmatter/docs/cli/delta.md
  - darkmatter/docs/cli/developing.md
  - darkmatter/docs/cli/edit.md
  - darkmatter/docs/cli/get.md
  - darkmatter/docs/cli/graph.md
  - darkmatter/docs/cli/hash.md
  - darkmatter/docs/cli/index.md
  - darkmatter/docs/cli/output-and-width.md
  - darkmatter/docs/cli/policy.md
  - darkmatter/docs/cli/render.md
  - darkmatter/docs/cli/rm.md
  - darkmatter/docs/cli/schema.md
  - darkmatter/docs/cli/set.md
  - darkmatter/docs/cli/toc.md
  - darkmatter/docs/cli/validate.md
  - darkmatter/docs/dependencies.md
  - darkmatter/docs/errors/README.md
  - darkmatter/docs/errors/composition-problems.md
  - darkmatter/docs/errors/file-reference-failures.md
  - darkmatter/docs/inline/call-check.md
  - darkmatter/docs/inline/filter.md
  - darkmatter/docs/inline/link-validation.md
  - darkmatter/docs/inline/map.md
  - darkmatter/docs/inline/preflight-checks.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/rendering/disclosure.md
  - darkmatter/docs/rendering/mermaid.md
  - darkmatter/docs/rendering/popover.md
  - darkmatter/docs/structs/Link.md
  - darkmatter/docs/structs/Markdown.md
  - darkmatter/docs/topics/compose-requests.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/delta.md
  - darkmatter/docs/topics/hyperlinking.md
  - darkmatter/docs/topics/image-previews.md
  - darkmatter/docs/topics/lifecycle/error-handling.md
  - darkmatter/docs/topics/lifecycle/index.md
  - darkmatter/docs/topics/markdown-formatting.md
  - darkmatter/docs/topics/output-formats.md
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/topics/schemas/dmls-schema-support.md
  - darkmatter/docs/topics/schemas/schema-triggers.md
  - darkmatter/fixes/2026-11-04-composition-errors/implementation.md
  - docs/cli-best-practices.md
  - docs/dependencies.md
  - renderable/README.md
  - renderable/docs/command-output.md
  - renderable/docs/tree-rendering.md
completed_phase: 6
implemented: true
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

## Phase 2

Ran as four serial foreground subagents (one per task) because all four edit one crate, then orchestrator integration. Everything below was verified on the final tree.

### Wave 4 — Reference provenance

- `ReferenceOrigin.authored` / `.section`, `ReferenceInsertion.authored_directive_line`, `ReferenceTrailStep`, `ReferenceIssue.included_via`; `ComposeReport.body_origin`; private `PreparedReferenceContent`; `BodyOrigin::producer_of` backed by producer segments carried through edits and splices; `project_whole`; `PreparedHeading` snapshot; CSS spans rebased to body offsets; prologue/epilogue located through the frontmatter leaf machinery; A10 `when=` errors carry authored line and source (`TransclusionError::ConditionEval.authored`).
- `Markdown::new` does not misread a leading `---`: pinned by `a_leading_rule_is_not_read_as_frontmatter`.
- Reconstructed spike fixtures in `lib/tests/fixtures/composition_errors/`. Checked by hand through the real binary: `md validate refs root.md --fragments` prints 9:5, 15:9, 22:13, 24:8, 28:1, child 7:9, 7:43, 9:7, 11:1, and `(transcluded at root.md:26)`.

### Wave 4 — Link migration

- `Link` parses titles through `parse_link_title`; `key=value` no longer applies attributes. `md clean` migrates by source span (`cleanup/link_titles.rs`); `compose/link_title_warnings.rs` produces located warnings; codes are in the compose and new render registries. `query_href` and `osc8_parameters` are shared by `Link` and the tree folds.

### Wave 5 — Report data

- `reference/report.rs`: `ValidationReportView` with `CommandOutput`, grouped by `(source, trail)`; `reason_phrase`/`construct_phrase`; `validation_category_label` and `validation_order` removed; trails populated during the walk; `Table::borderless`. CLI-atheist baselines updated (new fields; `line` values did not change because those fixtures have no frontmatter).

### Wave 5 — Diagnostic projection

- `DiagnosticSource` optional positions plus `section`/`inserted`; flattened `ReferenceDetails`; `check_references -> Result`, used with `?` in direct compose and lifecycle.
- Regression input: a hyperlink leaving the repository, which makes `validate_references` return Err. With the old line temporarily restored, the direct path failed with exit 0 where 1 is expected. The lifecycle leg is by reasoning only (same branch), because the test stopped at the first leg.

### Wave 6 — Reference checkpoint: defects found by the gate

Scoped subagent runs missed these; the first full `just test` caught them:

- **Real bug:** `--allow ctx-override` stopped working on direct compose. `run_compose` ran the check before applying the recoveries, which the old swallowed error hid. Fixed by moving the four document-judging recoveries ahead of the check (pinned by `policy_verbs::a_specific_recovery_beats_deny_all_in_either_order` and `an_independently_denied_diagnostic_still_blocks_a_recovered_command`).
- **Stale assertion:** `magic_root` looked for `Valid: 1`; now `across 2 documents`, which also proves `@doc.md` resolved.

Windows found, all test assumptions (no product change):

- `kind_semantic_checks` (4 tests, older than this phase): `str::ends_with("schemas/…")` against backslashes; now `Path::ends_with`.
- `reference_report::a_deep_issue…`, `cli reference_report::data_reports…`: compare as `Path`.
- `link_titles::render_warns…`: a long temp path wraps the stderr row; whitespace is collapsed first.
- `invalid_trigger_warnings::compose_reference_refusal…`: separators normalized on both sides; the full path is still required.
- `cutover_reference::reference_block_quote_width_and_left`: not Windows-specific; it depended on the host's `COLORTERM` and reproduces on macOS without it. The helper now pins the depth; the snapshot is unchanged and passes with and without `COLORTERM`.

### Gates (final tree, macOS unless stated)

| Gate | Result |
|---|---|
| `just test` darkmatter (`-- --skip every_kinded_schema_artifact_passes_its_definition_through_the_registry`) | 13221 passed, 11 skipped |
| `just lint` darkmatter, renderable, biscuit-terminal | pass |
| `just check-tier-coverage darkmatter` | pass, 0 stranded |
| `just test` renderable / biscuit-terminal | 731 / 3992 passed (55 skipped) |
| `cargo check --workspace --all-targets` | clean (run before the final test-only edits to darkmatter; those are covered by the `just test` above) |
| `just cross-check --os windows`: darkmatter-cli / darkmatter-lsp / renderable / biscuit-terminal | 1512 / 1724 / 731 / 3568 passed |
| `just cross-check darkmatter --os windows` | 9899 passed, 1 failed (the corpus failure below) |

Dependents, serially, filtered to `link\|render\|report\|diagnostic\|compose\|markdown\|prose\|table\|output`: biscuit-icon-cli 36, biscuit-speaks-cli 1, biscuit-terminal-cli 118, messenger 72, messenger-cli 19, playa-cli 5, research 150, sniff-detection-cli 521, claudine-gen 30. With the filter `compos\|link\|report\|diagnostic\|lifecycle\|call_check\|reference`: claudine 3002, claudine-cli 1779 (`test-fixtures,terminal-tests`). All pass. These ran before the final test-only edits, which touch no code they depend on.

Not run: `just test-l2` and `just test-browser`; `lib/tests/browser/browser_render.rs` inputs were migrated and compiled but not executed. Linux and WSL were not cross-checked; CI covers Linux, and the nightly covers WSL.

The scope shortcut was not used: source files changed this phase, so both gates ran. The prompt's premise that the uncommitted tree equals this phase's changes does not hold here (see `decisions_for_review`), so `affected_scope.py` over the uncommitted tree would understate the phase.

### Pre-existing and load-sensitive failures

- **Pre-existing, skipped on macOS, failing on Windows:** `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry`. Its input, `claudine/docs/research/sandboxing/providers/sandboxing.yaml`, uses syntax planned by `2026-11-05-dynamic-schema-types`. Not touched.
- **Load-sensitive:** with host load at 68 (other sessions were active), 8 existing tests reported 5.0–6.7 s in one full run (`schema_validate_legacy_json_output_is_byte_identical`, `test_compose_denied_host_never_reads_a_seeded_cache_entry`, `every_suggested_rule_approves_the_command`, five `darkmatter-lsp` parity tests). Each took under 3 s when rerun alone; the final full run reported 1 slow test. None was added by this phase.
- Subagents used `--no-fail-fast` on narrowed runs several times against their brief; no result recorded here came from those runs.

### Requirement to test mapping (Phase 2)

| Requirement | Tests |
|---|---|
| Authored line/column, LF/CRLF/BOM, multibyte | `reference_provenance::{spike_fixture_positions_and_sections_are_exact, crlf_line_endings_report_the_same_positions, a_byte_order_mark_reports_the_same_positions, an_image_after_a_multibyte_character_has_a_character_column}` |
| Prepared coordinates preserved | `reference_provenance::prepared_coordinates_are_unchanged`; existing file-tree tests |
| Inserted references name their producer (edge, adjacent, chained) | `body_origin::tests::producers::*` (13); `reference_provenance::{producers_at_both_body_edges_are_located, adjacent_producers_name_their_own_expression, an_edit_inside_produced_text_inherits_the_original_producer, a_replaced_key_is_the_producer_of_its_value, shell_output_is_located_at_its_directive}` |
| Mixed label is authored; in-memory and remote have null positions | `reference_provenance::{a_mixed_label_stays_authored_with_a_first_character_span, an_in_memory_document_has_a_section_and_no_authored_position, a_remote_document_keeps_its_url_and_has_no_authored_position}` |
| Sections use final slugs; removed and duplicate headings; leading rule | `reference_provenance::{a_section_uses_the_final_slug_of_the_innermost_prior_heading, a_leading_rule_is_not_read_as_frontmatter, fragment_lookup_keeps_the_table_of_contents_slugs}` |
| CSS rebasing; frontmatter values | `reference_provenance::{css_records_are_rebased_onto_the_body_and_include_their_opener, css_positions_follow_preparation_edits, frontmatter_values_locate_their_authored_position}` |
| A10 `when=` | `reference_provenance::a_transclusion_when_*`, `an_in_memory_when_failure_has_no_authored_source` |
| Title classification, `qp`, HTML and OSC 8 mapping, Windows URI, round trip | `link_titles::*` (17) |
| `md clean` migration, parity, idempotence, code left alone | `link_title_migration::*` (15); CLI `link_titles::clean_migrates_once_and_leaves_code_alone`; shared parity case `legacy-link-title` (lib, CLI, DMLS) |
| Located warnings, dedup by span, survive failure, `--deny` | `link_title_warnings::*` (16); CLI `link_titles::{report_json_locates_both_warnings_and_succeeds, a_failing_composition_reports_the_warning_too, compose_deny_names_each_code, render_warns_on_stderr_and_deny_blocks_it}` |
| Report grouping, trails, visibility, tallies, grammar, details, every phrase | `reference_report::*` (32) |
| No `is not valid`, kind headings, or `failure:` row | `reference_report::a_report_has_no_failure_row_verdict_or_kind_heading`; CLI `compose_reports_broken_references_on_stderr_without_the_old_wording` |
| Markdown, HTML, OSC 8, and `-v` location forms | `reference_report::{markdown_writes_the_exact_location_link, html_moves_the_section_and_position_into_the_query, osc8_links_carry_a_bare_uri_and_their_position_as_parameters, verbose_plain_prints_the_bare_file_uri}` |
| Graph and validate refs share a body | CLI `reference_report::{graph_and_validate_refs_print_the_same_report_body, a_clean_document_prints_the_same_all_ok_summary_on_both}` |
| Diagnostic `source`, details, trail, every family, null positions | CLI `compose_report_diagnostics::*` (10); `diagnostic_projection::*` (4) |
| Failed check is a failure, direct and lifecycle; no shell run | CLI `compose_report_diagnostics::{a_reference_check_that_cannot_run_fails_the_run, the_check_runs_no_shell_directive}` |
| `--allow` recoveries reach the check | `policy_verbs::{a_specific_recovery_beats_deny_all_in_either_order, an_independently_denied_diagnostic_still_blocks_a_recovered_command}`; `compose_state_set` recovery tests |

The Input Robustness Matrix does not apply (plan rule 29: Markdown and title grammars and output writers, not a configuration loader).

## Phase 3

Ran as two foreground subagents (Document rendering; test migration and contracts) with the CLI layer written by the orchestrator between them. All results below are from the final tree.

### Wave 7 — Document rendering

- P1, P2, P3, P6, P7, and P8 fixed on `Markdown::as_terminal` and `DarkmatterPage::render`: `capability_terminal` and `ambient_terminal_width` are gone; `Width` runs through page state, `LayoutContext`, `TerminalOptions`, and the code band; a right margin and padding with no background emit nothing; `HyperlinkMode` maps onto OSC 8; verbosity reaches the fold; the `▉` placeholder is gone.
- `with_output_options` on the page and on `TerminalOptions`; plain pinned at render time, so no builder can restore a capability.
- `markdown/output/command/`: `DocumentOutput`, `DocumentEnvelope`, `Outcome`, `PerfData`, `LifecycleOutputEntry`, `ReferenceSummary`, `document_markdown`. `ComposeOptions::with_output_width`.
- Snapshots changed only for a listed defect: P6 (`char_list_item_center/right_alignment_terminal`, now in the 60 columns given rather than the ambient 80) and P7 (`ref_page_margin_and_padding_terminal`).

### Wave 7 — Run collection

- `cli/src/output/mod.rs`: `OutputEnvironment` (the one read of `COLUMNS`/`OUTPUT_COLUMNS`/stream state), `resolve_output`, `stderr_terminal`, `stderr_report`, `write_stdout`. `cli/src/output/report.rs`: `ReportSink` generalized from compose's collector, lock-protected because lifecycle delivery is `Sync`; `ReportedExit`; `ClassifiedFailure`. `commands/compose/report.rs` removed.
- Lifecycle seam: `md_host::CommandOutput::command` (provided method) gives the host one finished command's stdout and stderr; `ComposeHost::event` supplies the stage.
- `artifact.rs`: `ArtifactOpener`, `ProcessOpener`, open failure is a run failure naming the path.
- `biscuit_terminal::output::stderr_terminal_columns` added, because `Terminal::detected_columns` measures stdout.

### Wave 8 — Compose and render

- `--output` through `args/output.rs::target_parser`; compose accepts exactly markdown/json/json5/yaml/toml; render accepts both families, default `terminal`. `--width` is real. `reject_width_flag`, `OutputFormat`, `ComposeOutputFormat`, and the AST artifact are gone. A refused reference exits 1, direct and lifecycle, through `ReportedExit`, not `process::exit`. Interrupted stays 130 with no object.
- `md compose` default path stays `println!` of the same text: no dispatch.
- Stderr now uses stderr's own detection everywhere on these paths, including `main`'s error rendering.

### Wave 9 — Envelope contracts

- `cli/schemas/output/{compose,render}.yaml`: the spec's text, unchanged; strict validation passes and a negative control proves it can refuse.
- `cli/tests/l1/output_envelope.rs` (170 tests); `compose_report_json.rs` became `compose_data_output.rs`.

### Gates (final tree, macOS unless stated)

| Gate | Result |
|---|---|
| `just test` darkmatter (`-- --skip every_kinded_schema_artifact_passes_its_definition_through_the_registry`) | 13468 passed, 11 skipped |
| `just lint` darkmatter, biscuit-terminal | pass |
| `just check-tier-coverage darkmatter` | pass, 0 stranded |
| `env -u COLORTERM`: `plain_corpus`, `cutover_reference`, `layout::page`, `tree_features` | 192 passed |
| `cargo test -p darkmatter --doc output::command` | pass |
| `cargo check --workspace --all-targets` | clean |
| biscuit-terminal `test(output)` | 85 passed (only an addition to its `width.rs`) |
| Dependents, filter `compos\|lifecycle\|render\|final_message` | claudine 2719 passed; claudine-cli `compos\|render` with `test-fixtures,terminal-tests` 1183 passed |
| `just cross-check darkmatter-cli --os windows` | 1690 passed, 1 failed (`schema_about_markdown_code_uses_inverted_theme...`); fixed; `schema_about` passes filtered, and `output_envelope` passes filtered on the final tree (170) |
| `just cross-check darkmatter --os windows` | 9963 passed, 2 failed: the corpus test below and `render_tree_terminal_emits_dim_sgr_for_dim_span`; fixed, `render_tree_terminal` rerun passes |

Both Windows failures were tests relying on the width-selects-rich leak this phase removed, reading ambient color on a host with no `COLORTERM`; neither was Windows-specific. Two fixes after those full Windows runs (the `about` profile and test pin) were rerun filtered, not full. Linux and WSL were not cross-checked; `just test-l2` and `just test-browser` were not run (Phase 6). One L2 test was run because a filter selected it, which is how the stale `claudine-cli` assertion was found.

The scope shortcut was not used: source files changed, so both gates ran.

### Pre-existing and load-sensitive failures

- **Pre-existing, skipped on macOS, failing on Windows:** `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` (planned syntax of `2026-11-05-dynamic-schema-types`).
- Nothing load-sensitive appeared. One subagent ran scoped filters twice with `--no-fail-fast` against its brief; no result here came from those runs.

### Requirement to test mapping (Phase 3)

| Requirement | Tests |
|---|---|
| Plain corpus at 40/80/unlimited, with and without frame and max-width, through `as_terminal` and `DarkmatterPage`: no ESC, no trailing whitespace, quotes, rule, tables, fences, headings, disclosure, link and image rule | `plain_corpus::*` (lib L1, one test per row); insta styled and plain snapshots |
| Unlimited has natural size; page width reaches the zero-config page (P6) | `plain_corpus` unlimited rows; `layout::page::tests::zero_config_render_uses_the_width_it_was_given` |
| P2 width never selects capabilities; P3 `HyperlinkMode` | `markdown/output/command/tests.rs`; entrypoints capability tests |
| `DocumentOutput` per target, plain parity, `document_markdown` per width, envelope field order, `perf` absent, nulls present | `markdown/output/command/tests.rs`, doctests |
| `with_output_width` against frontmatter | `markdown/output/command/tests.rs`; `output_envelope` width rows |
| Envelope conformance: success, failure, warnings, for compose and render | `output_envelope` conformance tests (strict, hand-written so `include_str!` maps per test) |
| JSON/JSON5/YAML/TOML parity, null omission, one value on stdout | `output_envelope` parity rows; `biscuit-terminal` `output_dispatch` (TOML null in an array, Phase 1) |
| Empty stderr with `-v`, `-vv`, `--perf`, `--debug`; `perf` present only when asked | `output_envelope` stream rows |
| Width precedence, percent, float, invalid `OUTPUT_COLUMNS`, stderr independence | `output::tests::*` (spoofed streams); `output_envelope` width rows; `layout_flags::layout_width_flag_wraps_prose_at_its_columns` |
| Lifecycle output: payload in run order, one entry per shell step, stderr in Markdown mode, stdout only the document | `output_envelope` lifecycle rows; `compose_lifecycle::stdout_holds_only_the_document_and_every_verb_uses_stderr_in_run_order` |
| Exit status: refused reference 1 with and without a lifecycle, `--deny` 1, usage 2 with empty stdout, removed values 2, interrupt 130 with no object | `output_envelope` exit rows; `compose_lifecycle`; `output::report::tests::an_interrupted_run_writes_no_value_and_keeps_its_status` |
| `--show` extension and content per format, open failure is one failure payload naming the path | `output_envelope` show rows; `output::report::tests::an_open_failure_fails_the_run` |
| Serialization failure names the path; usage error passes through | `output::report::tests::*` |
| Terminal equals plain piped, formats, disclosure in markdown vs markdown-plus, source text, `--max-width` in all three spellings | `output_envelope` render rows; `render_basic` |
| Style warnings: stderr in page outputs, diagnostics with empty stderr in data, `--deny style` fails one payload | `output_envelope::a_style_warning_*`, `a_denied_style_warning_fails_a_data_output_with_one_payload` |
| Recipe bytes: `md compose X --frontmatter` and `md compose X` | `output_envelope` recipe rows, literal strings matching the pre-change binary |
| Problem report on stderr, no ESC, unwrapped, even with `--width 20` | `output_envelope` formatted-diagnostic rows |

A piped run has no TTY, so "TTY-profile" parity for compose is by construction (its Markdown path never reads a terminal), and spoofed stream behavior is tested in-process through `OutputEnvironment`. Real-terminal coverage is Phase 6.

The Input Robustness Matrix does not apply (plan rule 29). The `OUTPUT_COLUMNS` and `--width` value parsers have their shapes tested at `biscuit-terminal` `output::width::tests`, and wiring at `output_envelope`.

## Phase 4

Ran with one foreground subagent, for migrating existing CLI tests to the contract; the product code, schemas, and new tests were written by the orchestrator. All results below are from the final tree.

### Wave 10 — Leaf arguments

- Every leaf has `--output` with its default in `--help`; `--width` was already global. `SchemaDetectOutput` adds `json-schema`. `args/output.rs::COMMAND_LEAVES` records path, default, schema file, and families. `Cli::requests_data_output` is exhaustive over leaves, so tracing stays off for every data output.
- Removed: `CodeBlockOutput`, `ValidateOutputFormat`, `SchemaValidateFormat`, `SchemaDetectFormat`, `parse_fixed_width`, every per-command format flag. The removed flags are simply undeclared, so clap reports them (status 2) and their tokens stay legal inside positional content and after `--`. `complete_fixed_width_values` became `complete_width_values` on `--width`.
- `validate_subcommand_usage` and the utility routes (`--completions`, `--list-themes` with `--output`) are usage errors now.

### Shared delivery

- `cli/src/output/command.rs`: `CommandRun`, `Completed`, `DataShape::{Typed, Object, Bare}`. Serialization failure and closed stdout behave as `ReportSink` does, without its `--show`, which no other command has.
- Library: `markdown/output/command/{value,data,clean,schema_about}.rs`. The clean wire types and `FindingRow` moved there from the CLI (rule 8). `DocumentOutput::with_markdown_source` keeps clean's and set's exact bytes.
- `Terminal::from_output_options` in `biscuit-terminal`: a component's terminal from the options it was given, with nothing detected. This is what lets one override serve `plain`.
- Interpretation of rule 23 as planned: an explicit existing shape outranks the general "every result has `outcome`" wording. Departures from the plan's letter are in `decisions_for_review`.

### Wave 11 and 12 — Commands

- Document: `clean` and `set` through `document_value`; a saved `clean` prints its delta; explicit `--width` lowers to the `fixed-width`/`unwrap` rule overrides (`u16` saturates, matching compose). `get`'s formatted output is today's raw layout, byte for byte, including the bare newline for null. `edit` reports `created` and the editor's executable.
- Reference: `toc` and `delta` moved to their own modules; `graph` and `validate refs` print the report on stdout and exit 1 through the value rather than `process::exit`; `--graph` stays the diagram alone (rule 25). `print_validation_report_text` and graph's footer were already gone.
- Schema: `validate` collects records before dispatch and has no `process::exit`; `detect` and `triggers` have data shapes; `about` builds its data from the catalogs, removing Phase 3's temporary profile. `-vv` timings go to stderr in formatted runs only.
- Hash and code: rule 24's fields; hash formatted bytes unchanged. `code-block` styled output uses the block's own panel, `plain` uses the fence.
- A defect the migration surfaced, fixed here: every error `main` printed held ESC bytes on a piped stderr. New tests pin it.

### Wave 13 — Contract inventory

`cli/tests/l1/output_commands.rs` (152 tests): strict conformance per leaf; the guard `every_leaf_has_a_schema_file_and_a_conformance_test` (clap leaves == table, schema directory == table, each schema spelled in a test); every leaf's families and global width; help defaults; removed spellings and conflicts; terminal == plain; width invariance; format parity. The conformance tests read shipped schema files with `include_str!` inside each `#[test]`.

### Gates (final tree, macOS unless stated)

| Gate | Result |
|---|---|
| `just test` darkmatter (`-- --skip every_kinded_schema_artifact_passes_its_definition_through_the_registry`) | 13635 passed, 11 skipped |
| `just lint` darkmatter | pass (including wasm32 zed-dmls) |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `cargo check --workspace --all-targets` | clean |
| `cargo test -p darkmatter --doc output::command`, `-p biscuit-terminal --doc from_output_options` | pass |
| biscuit-terminal `test(output)` | 87 passed |
| `just cross-check darkmatter-cli --os windows` | 1859 passed, 69 skipped |

After that Windows run the only changes were documentation and skills. No published Windows observation: the rig tested a patched tree, not this head. `just cross-check darkmatter --os windows` was not run, because the library change is additive. Linux and WSL were not cross-checked; `just test-l2` and `just test-browser` were not run (Phase 6).

Dependents: not run beyond the workspace compile. This phase changes no existing library symbol's behavior: `darkmatter` and `biscuit-terminal` gained items only, and `darkmatter-cli` has no dependents. `biscuit-terminal`'s full `just test` was not rerun, for the same reason; only its output tests.

The scope shortcut was not used: source files changed, so both gates ran.

### Pre-existing and load-sensitive failures

- **Pre-existing, skipped:** `shipped_schema_corpus::every_kinded_schema_artifact_passes_its_definition_through_the_registry` (planned syntax of `2026-11-05-dynamic-schema-types`).
- Nothing load-sensitive appeared. The full run reported no timeouts at load 18. The Windows run recorded one test at or above 5 s; I did not check which, or whether this phase added it.

### Tests deleted or moved by the migration

- Deleted: `code_block_theme_env_changes_terminal_output_without_flag`; a pipe cannot show a theme now, and library tests `terminal_honors_theme_env_when_no_override` and `terminal_honors_code_theme_env_when_no_override` cover it.
- Converted: `test_clean_subcommand_rejects_fixed_width_with_ignore_incidental_newlines` to `test_clean_subcommand_rejects_the_removed_width_flags`.
- Moved in-process, because styled output needs a terminal: `code_block` styling tests; four `schema_about` tests went to unit tests in `about.rs`. Each kept a CLI test asserting the piped form. Golden files: none changed; baselines compare after removing `outcome` and `diagnostics`, which are asserted separately.
- Not exercised against a real TTY on this path: styled output of the commands converted here. That is Phase 6's L2.

### Requirement to test mapping (Phase 4)

| Requirement | Tests |
|---|---|
| Families per leaf, `json-schema` only on detect, global `--width`, inventory | `output_commands::{every_leaf_declares_its_output_and_width, every_leaf_has_a_schema_file_and_a_conformance_test, json_schema_is_only_detect}` |
| `--help` shows each default | `output_commands::help_*`, `the_help_rows_cover_every_leaf`, `render_help_states_its_default` |
| Removed flags and values exit 2, with empty stdout and no effect | `output_commands::{clean_json, toc_json, delta_json, rm_json, graph_json, get_json5, get_yaml, get_toml, get_raw, get_plain, validate_format, schema_validate_format, schema_detect_format, clean_fixed_width, clean_incidental, output_auto, output_ast, output_text, output_report_json}` |
| Conflicts exit 2, `--compact` only with json and json5 | `output_commands::{compact_with_yaml, compact_with_plain, compact_prints_one_line_for_json_and_json5, subcommand_with_top_level_output, list_themes_with_output, completions_with_output, invalid_assignment, bad_highlight, hash_directory_save, hash_directory_kind, a_yaml_document_refuses_assignments_as_a_usage_error}` |
| `md get --output plain` is `--raw`: string, null, list, object | `output_commands::get_plain_*` |
| Every shape against its schema: success, failure, warning | `output_commands::conforms_*`; `every_schema_file_shares_one_diagnostic_type` |
| Terminal piped equals plain, no ESC | `output_commands::plain_*`; `a_formatted_failure_with_no_result_is_on_stderr_without_escapes`, `a_report_row_holding_an_error_block_has_no_escapes_on_a_pipe`, `an_unloadable_baseline_is_a_failure_on_stderr_without_escapes`; `delta` goldens |
| `--width` no effect without prose; accepted everywhere | `output_commands::{width_*, accepts_*}` |
| Unlimited has natural size; explicit width wraps | `output_commands::{a_report_has_its_natural_size_on_a_pipe_with_no_width, a_report_wraps_at_an_explicit_width}` |
| Explicit width overrides frontmatter, default keeps it; data matches Markdown | `output_commands::{explicit_width_reflows_clean_and_set_and_overrides_frontmatter, explicit_source_keeps_breaks_a_document_would_have_unwrapped}`; `commands::clean::tests::*`; `commands::frontmatter::tests::an_explicit_width_reflows_only_the_body` |
| Float width: stderr for formatted, payload for data | `output_commands::{a_fractional_width_warns_on_stderr_for_a_formatted_output, conforms_toc_fractional_width, conforms_get_warning_wraps_a_scalar, conforms_clean_with_a_run_warning_keeps_its_own_diagnostics, conforms_schema_validate_fractional_width}` |
| Failing report on stdout, status 1 | `output_commands::a_report_that_fails_is_on_stdout_with_status_one`; `conforms_graph_validate_failure`, `conforms_validate_refs_failure_has_one_diagnostic_per_problem`, `conforms_hash_diff_with_nothing_stored_fails`, `conforms_hash_save_then_diff_round_trips` |
| Save side effects, data after save, round trip | `output_commands::{conforms_set_save_reports_the_document_it_wrote, saving_prints_nothing_formatted_for_set_and_rm, the_top_level_save_alias_forwards_output_and_width}`; `conforms_hash_save_then_diff_round_trips` (write, read, change, read) |
| Edit: created, existing, editor, failure launches nothing | `output_commands::{conforms_edit_reports_a_created_file, conforms_edit_reports_an_existing_file, conforms_edit_failure_launches_nothing}` |
| Heterogeneous hash tuples through TOML, null omitted | `output_commands::conforms_hash_detailed_writes_its_mixed_tuples_as_toml` |
| Format parity | `output_commands::parity_*`, `a_toml_scalar_is_wrapped_as_value` |
| Detect forms, multiple and merged | `output_commands::{yaml_is_the_bare_schema_and_json_schema_is_its_draft, conforms_schema_detect_*}` |
| `--quiet` keeps data records; failure kinds distinct | `output_commands::conforms_schema_validate_keeps_a_failing_and_an_unopened_file_apart` |
| Code: plain long line whole, markdown and HTML bytes; styled panel | `output_commands::code_keeps_its_exact_markdown_and_a_long_line_is_whole_in_plain`; `code_block` in-process tests |
| `--graph` is the diagram alone | `output_commands::{conforms_validate_refs_graph_is_the_diagram_alone, a_graph_notation_is_its_source_in_a_terminal_and_a_fence_in_markdown}` |
| Run status shapes | `output::command::tests::*` |
| Terminal from options | `biscuit-terminal` `output::profile::tests::{a_terminal_from_a_plain_profile_keeps_no_capability, a_terminal_round_trips_a_rich_profile}` and doctest |

The Input Robustness Matrix does not apply (plan rule 29).

## Phase 5

Ran Wave 14 with three foreground documentation subagents, each owning its files; the orchestrator did `just/review.just`, the code fixes, the audit, and the gates.

### Wave 14 — Documentation

- CLI: new `output-and-width.md`, `data-contracts.md` (replaces `report-json.md`), `validate.md`, `schema.md`, `code-block.md`, `edit.md`, `developing.md`; every command page updated; READMEs cut to overviews. `just/review.just`: nine `md get --raw` calls became `md get --output plain`; bytes checked for scalar, null, list, and missing keys against the built binary.
- Topics: link titles, authored positions, stopping rules (new `errors/composition-problems.md`), lifecycle output order, exit statuses, removed flags. Comment fixes in `simplified-schema/lib/src/validation.rs` and `dmls/tests/l1/expression_call_diagnostics.rs`; `report_diagnostics.rs` and Claudine's error module had no stale mention left.
- Shared: new `renderable/docs/command-output.md` and `biscuit-terminal/docs/output.md`; `prose.md` degradation; `cli` skill and `docs/cli-best-practices.md`.

A subagent's `md edit` checks launched two windowless `nvim` processes on temp files (`EDITOR` naming a missing program falls through). It killed both and removed the temp directory.

### Wave 15 — Consistency audit

The sweep covered removed flags and values, old report wording, old exit claims, `report-json` links, and `key=value` titles across docs, skills, just files, and source comments. Remaining hits are Removed-spellings tables, usage-error exit 2, unrelated "is not valid" prose, image titles and pre-migration examples, parser-rejection tests, and historical specs. `docs/dependencies.md` still named `--format json` (fixed). Older doc links to specs in `inline/fm-shell-expansion.md`, `rendering/popover.md`, `rendering/style.md`, `rendering/code-highlighting.md`, and `topics/caching.md` predate this fix and were left. Comments corrected: `RenderedOutput::diagnostics` overstated what the terminal fold reports; `--width`'s help implied every command has `--max-width`.

### Defects the documentation pass found, fixed here

| Defect | Reproduced with | Fix |
|---|---|---|
| The report lost every row to a "Table could not be rendered" notice | `md validate refs --width 30`; a long reference at `--width 60` | borderless tables stack (`emit_stacked_rows`) |
| `md toc --output toml` always failed | exit 1 with `u64 value was too large` | above-range integers become decimal text in `toml_values` |
| A file argument's class was missing from data | `md compose nofile.md --output json`, `md get @ title` | `DocumentArgumentError::diagnostic` adds `failure` and `reference` |

### Requirement to test mapping

| Behavior | Test | Boundary, rung |
|---|---|---|
| Too-narrow borderless table keeps all cells, verbatim, no notice | `render_tree_tests::a_borderless_table_too_narrow_for_its_columns_stacks_every_cell` | terminal fold, F1 |
| Report keeps locations at 20/30/40/60 columns; a stacked reference is verbatim | `reference_report::a_width_too_narrow_for_the_columns_keeps_every_row` | library public `render_output`, F2 |
| TOML integer range edges (`u64::MAX`, `i64::MAX`, small, negative, in an array) | `output::data::tests::toml_writes_an_integer_above_its_range_as_decimal_text` | dispatcher, F1 |
| `md toc --output toml` succeeds and its hashes match JSON's digits | `output_commands::toc_toml_carries_its_hashes_without_failing` | real CLI, F3 |
| `failure` and `reference` on no-match and invalid-reference across get, compose, detect, strict schemas | `output_commands::a_file_argument_failure_carries_its_class_in_data` | real CLI, F3 |

Input Robustness Matrix: not applicable, per plan rule 29; no loader changed.

### Gates (macOS)

- `just test` in `darkmatter/` with `--skip every_kinded_schema_artifact_passes_its_definition_through_the_registry`: 13638 passed, 11 skipped, 5 slow. After it, a change to `emit_stacked_rows` reran `reference_report` (33), biscuit-terminal `render_tree|borderless` (311), and darkmatter-cli `validate|graph` (259): all passed.
- `just lint` in `darkmatter/`: pass (rerun after the last change). `just test` in `biscuit-terminal/`: 3996 passed, 55 skipped, before the same change. `just lint` there: pass.
- CI scope (`affected_scope.py` on the working tree): `change_class: package`, so gates were required. Its test-input cells, run here: claudine-cli `research_sequence_artifacts` (7), test-toolkit `the_ci_documentation_states_the_implemented_behavior` (1), tree-hugger-cli (2733): all passed.
- Doc-reading tests (`docs_corpus`, `trigger_spelling`, `current_root_*`, `schema_phase_validation`, `shipped_schema_corpus`) ran within the full gate.
- Not run: L2, browser, doctest, Windows, WSL2, Linux; Phase 6 owns them. The fixes carry no OS logic, so no cross-check was run. `just check-tier-coverage` was not run: new tests carry no tier marker.

### Pre-existing and load-sensitive failures

`every_kinded_schema_artifact_passes_its_definition_through_the_registry` still fails on `sandboxing.yaml`, unrelated and already logged. No load-sensitive failure this phase.

## Phase 6

Ran as the orchestrator with no subagents: the work was serial verification. A first pass reproduced the Phase 5 defect list against the built binary; each confirmed defect got a failing test first, then a fix.

### Wave 16 — Defects from the Phase 5 list

| Reported | Verdict | Change |
|---|---|---|
| `md validate refs` aborts on an expression in inline code | Not a defect. `md compose` rejects the same document with the same error, and the check reports what composition would do. Only an expression that does not parse does this. | None; in `decisions_for_review` |
| `md schema about --width 30` wraps near 60 | Confirmed: a 64-column floor | Floor removed; a table that cannot fit becomes a list naming each column |
| `md schema triggers --output markdown` runs rows together | Confirmed: it parsed its terminal text as Markdown | `trace_markdown` builds a real document; both outputs share `RootLine` |
| `md schema about --output markdown` leaves `&#32;` | Confirmed, two causes: the fold wrote each phrasing child of a tight list item as its own block, and a `<type>` keyword read as raw HTML | `render_list_item_blocks`; keyword is a code span |
| `md code-block --output markdown` adds a blank line | Confirmed | The file's last line ending is the fence's |
| `md clean --save` prints `No changes` above `Preamble: modified` | Confirmed in `compute_delta`, so `md delta` too: classification ignored the preamble | A changed preamble classifies as whitespace-only or minor. `delta_golden_preamble_change` had pinned the defect and moved |
| `md compose --shell --show` ignores `--show` | Confirmed | Usage error |
| `md render --deny` rejects `dm.style.unknown_key` | Confirmed: documented code, not in the catalog | Both style codes accepted, each denying only itself |
| `md edit` falls through on a missing `$EDITOR`, leaves an empty file | Confirmed; a test really launched `nvim` | Refuses when every set variable is missing, before creating anything; removes a file it created if the edit fails and it is still empty. A new `edit_refuses_…` test timed out on the launch before the fix |
| `render_terminal_output` drops diagnostics | Confirmed | Returns `Rendered<String>`; `RenderedOutput` carries them. `md` does not print them: see `decisions_for_review` |
| `link_metadata` names `dm.link.*` in a shared crate | Confirmed | `LinkTitleNotice::token()` is neutral; codes moved to `compose::policy` |

### Defects verification found beyond the list

| Defect | Found by | Fix |
|---|---|---|
| The report split `./missing-one.md` at the hyphen at every numeric width, even 200 | The styled artifact capture | A borderless table planned a boxed table's overhead (10 columns against 4 of gaps), and a later pass hands it its natural width. `Table::border_overhead` uses the gaps; the fold passes `borderless`; reference and location columns no longer wrap, so a row stacks instead |
| An interrupted lifecycle run could exit 1, not 130 | `ctrl_c_runs_cancelled_then_finalize_and_exits_130` on WSL2 | A group `SIGINT` kills the command as `md` receives it, and the handler sets its flag on another thread. A command ended by a signal now waits up to 250 ms for the flag (`wait_for_interrupt`) |
| My own test did not compile on Windows | `just cross-check darkmatter --os windows` | Gated `#[cfg(unix)]` like its neighbors |

Stale L2 tests corrected, none a product defect: `level2_apple_terminal_link_fallback_visible` and `…styled_link_fallback_escapes_bracket` asserted the removed `(url)` fallback; `level2_style_images_local_style_width_alignment_in_terminal` the removed `▉ IMAGE[`; `level2_page_no_double_blank_rows_between_code_blocks` did not read WezTerm's `CSI K` capture of a painted empty row; `level2_tmux_draws_sixel_itself_at_the_bottom_of_the_viewport` counted lines without the fixture's `tty_modes_restored` row, which predates this branch.

One first attempt was reversed. Forwarding fold diagnostics to stderr made `md validate refs --output markdown` warn about the command's own report, so the CLI forwarding was removed before the final gates. The phrasing-run fix also first applied to every container and broke `Section__baseline__markdown`; it is now list items only.

### Requirement to test mapping

| Behavior | Test | Boundary, rung |
|---|---|---|
| Tight list item is one block, and wraps under its marker | `markdown::tests::a_tight_list_item_writes_its_phrasing_as_one_block`, `…wraps_at_a_width_and_indents_under_its_marker` | fold, F1 |
| `schema about` Markdown rows, narrow width keeps content, no ESC | `output_commands::schema_about_markdown_keeps_each_entry_on_its_own_list_row`, `schema_about_honors_a_width_narrower_than_a_page` | real CLI, F3 |
| `schema triggers` Markdown rows | `output_commands::schema_triggers_markdown_keeps_its_rows_apart` | real CLI, F3 |
| Code block fence; a blank last line stays; data keeps bytes | `output_commands::code_block_markdown_has_no_blank_line_before_its_closing_fence` | real CLI, F3 |
| Preamble classification, all three variants | `delta::tests::a_changed_preamble_is_a_change`, `output_commands::delta_never_calls_a_changed_preamble_no_changes` | library F1, CLI F3 |
| `--shell --show` | `output_commands::compose_shell_refuses_show` | real CLI, F3 |
| Style code denial: denied, other code, unknown code, formatted and data | `output_commands::render_deny_takes_the_style_codes_it_reports` | real CLI, F3 |
| Editor resolution and cleanup, each cell | `editor::tests::a_configured_editor_that_is_missing_refuses_rather_than_probing`, `a_blank_editor_variable_counts_as_unset`, `output_commands::edit_refuses_…`, `edit_removes_…`, `edit_keeps_…` | library F1, CLI F3 |
| Terminal fold diagnostics | `render_terminal_output::the_generic_terminal_fold_reports_what_it_could_not_render` | public API, F2 |
| Borderless planning | `render_tree_tests::a_borderless_table_that_fits_exactly_wraps_nothing`, `reference_report::a_reference_is_never_split_at_any_width` | fold F1, public API F2 |
| Portable locations: spaces, Unicode, CRLF, drive-spelled URI on Windows | `reference_report::a_location_is_portable_through_spaces_unicode_and_crlf` | public API, F2, executed on Windows |
| Interrupt settling | `md_host::tests::a_late_interrupt_flag_is_seen_and_a_missing_one_gives_up`, `compose_lifecycle::ctrl_c_runs_cancelled_then_finalize_and_exits_130` | F1, real process and signal F3 |

Input Robustness Matrix: not applicable, per plan rule 29; no loader changed.

### Gates (final tree)

| Gate | Result |
|---|---|
| `just test`, renderable | 733 passed |
| `just test`, biscuit-terminal | 3998 run, all passed |
| `just test`, darkmatter, with the one logged skip | 13654 passed, 11 skipped |
| `just lint`, all three areas | pass |
| `just doctest`, all three areas | pass |
| `cargo check --workspace --all-targets` | pass |
| `just test-l2`, biscuit-terminal | 8 and 252 passed |
| `just test-l2`, darkmatter | 3, 18, and 69 passed |
| `just test-browser` | biscuit-terminal 56, darkmatter 44 passed |
| `just check-tier-coverage` darkmatter, biscuit-terminal | pass |
| Dependents, targeted | claudine-cli 17, simplified-schema 31 and 69, darkmatter-lsp 221, biscuit-icon 129, biscuit-visualized 19, messenger 226, tree-hugger-cli 2733, test-toolkit 1: all passed |
| `git diff --check` | clean |

WezTerm, tmux, kitty, Apple Terminal, and Google Chrome are all installed here, and the logs show no capability skip. L2 and browser passed after the last change each of them can reach, except `wait_for_interrupt`, which neither tier exercises.

CI scope (`affected_scope.py` on the working tree): `change_class: package`, `packages` non-empty, so the gates were required. It includes `simplified-schema`, from uncommitted topic-page changes this fix does not own.

### Cross-OS evidence

Declared hosts: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`.

| Package | Linux | Windows | WSL2 |
|---|---|---|---|
| renderable | 733 passed | 733 passed | 733 passed |
| biscuit-terminal | 3612 passed | 3574 passed | 3612 passed |
| darkmatter-cli | 1879 passed | 1869 passed | 1879 passed |
| darkmatter | L1 target only: 3999 of 4000 | 9968 of 9969 | 10022 of 10023 |

The one failure in each darkmatter cell is the pre-existing corpus test below. WSL2's darkmatter-cli first failed the interrupt race, then passed whole after the fix, and the test alone passed three more times. The darkmatter cells ran before `wait_for_interrupt` landed; darkmatter-cli's lifecycle tests reran on Windows and WSL2 after it.

**Gap:** Linux has no darkmatter archive run. Two attempts ended with `ld terminated with signal 9` on `build-linux`, which also stopped answering SSH, so the library's unit tests did not run there. WSL2 Ubuntu covers them; CI's Linux leg will too. Recorded in the `os` skill. No Windows or WSL2 observation was published from a filtered or patched run. L2 and browser ran on macOS only.

### Artifacts

`artifacts/` holds the report in every output, a styled capture with its control bytes shown as `\e`, the plain corpus at 40, 80, and unlimited, `streams-and-status.txt`, the conformance inventory, and `generate.sh`, which rebuilds them. `implementation.md` lists each acceptance item against its evidence.

### Pre-existing and load-sensitive failures

`every_kinded_schema_artifact_passes_its_definition_through_the_registry` fails on `claudine/docs/research/sandboxing/providers/sandboxing.yaml`, on every OS: an in-progress research schema with an expression in an enum. Left alone and in `decisions_for_review`. No load-sensitive failure on macOS.
