---
phases: 6
created: 2026-05-09
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/lib.rs
  - darkmatter/lib/src/layout/mod.rs
  - darkmatter/lib/src/layout/error.rs
  - darkmatter/lib/src/layout/types.rs
  - darkmatter/lib/src/layout/page.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/layout/mod.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/page.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .opencode/skill/darkmatter/SKILL.md
source_files_during_phase_3:
  - darkmatter/lib/src/layout/mod.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/types.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/page.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/output.rs
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .opencode/skill/darkmatter/SKILL.md
source_files_during_phase_6:
  - darkmatter/lib/src/layout/mod.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/tests/layout_snapshots.rs
  - darkmatter/lib/tests/snapshots/layout_snapshots__end_to_end_example_snapshot.snap
  - darkmatter/lib/tests/snapshots/layout_snapshots__pronounced_background_snapshot.snap
  - darkmatter/lib/tests/snapshots/layout_snapshots__zero_config_prose_snapshot.snap
docs_updated_during_phase_6:
  - darkmatter/lib/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - darkmatter
---

# Darkmatter Layout Execution Plan

Source: `darkmatter/features/2026-05-08-darkmatter-layout/spec.md`

## Phase 1: Baseline Discovery And API Scaffolding

- [x] Confirm the current workspace package names with `cargo metadata --no-deps --format-version 1` and identify the exact `darkmatter` lib and CLI package names for targeted test commands.
- [x] Read the current terminal render path in `darkmatter/lib/src/markdown/output/terminal.rs`, including `TerminalOptions`, `for_terminal`, `write_terminal`, `LineWrapper`, horizontal-rule rendering, and existing component dispatch points.
- [x] Read `darkmatter/lib/src/markdown/output/code_block.rs` and identify how code block width, padding rows, line numbers, and background resets are currently emitted.
- [x] Read `darkmatter/lib/src/markdown/output/html.rs` and identify the lowest-risk insertion point for a page-level browser wrapper.
- [x] Read `darkmatter/cli/src/args.rs` and `darkmatter/cli/src/output.rs` to map existing render flags into `TerminalOptions`.
- [x] [parallelizable] Inspect `biscuit-terminal` `Renderable`, `BrowserRenderable`, `Terminal`, color mode, and layout helper APIs needed by `DarkmatterPage`.
- [x] Create `darkmatter/lib/src/layout/` and define the public module boundary without changing existing render behavior.
- [x] Add and export the new public layout types: `DarkmatterPage`, `PageMargin`, `PagePadding`, `PageBackground`, `PageComponent`, `PageAlignment`, `PageFill`, `WidthUnit`, and `PageRenderError`.
- [x] Implement default values that preserve current behavior: zero margin, zero padding, transparent background, no max width, no line numbers, left component alignment, and full component fill.
- [x] Implement builder methods for margin, padding, page background, max width, line numbers, component alignment, component fill, and the documented `TerminalOptions` passthrough methods.
- [x] Implement validation helpers for horizontal space, `max_width`, and percent width units, returning the documented `PageRenderError` variants.
- [x] Validation checkpoint: run `cargo test -p darkmatter layout --lib` or the closest targeted lib test command and confirm the new module compiles with focused unit tests for defaults and validation helpers.

## Phase 2: Terminal Page Rendering Shell

- [x] Add a layout context type, internal to darkmatter if possible, that carries effective terminal width, content width, effective render width, page background colors, component alignment, and component fill settings.
- [x] Implement `DarkmatterPage::new(&Terminal)` so it captures terminal width, color mode, and capability information by value and does not borrow the terminal.
- [x] Implement `DarkmatterPage::render(&self, &Markdown) -> Result<String, PageRenderError>` using the existing terminal renderer as the delegated markdown body renderer.
- [x] Derive `TerminalOptions` from the page state before delegation, including `max_width = Some(effective_cols)` and `include_line_numbers` from the page builder.
- [x] Implement `PageBackground::Pronounced` color-mode inversion before markdown rendering and keep the page surface color based on the original terminal color mode.
- [x] Implement row decoration around delegated terminal output: transparent top and bottom margin rows, background-filled top and bottom padding rows, and left/right margin plus padding per content row.
- [x] Preserve byte-for-byte output for zero-config rendering by bypassing row decoration when margin, padding, background, max width, line numbers, alignment, and fill are all at defaults.
- [x] Map underlying markdown render failures into `PageRenderError::Render(String)`.
- [x] Implement the `biscuit_terminal::renderable::Renderable` trait for `DarkmatterPage` using the same render path and error mapping expected by the trait.
- [x] Validation checkpoint: add snapshot or string tests proving zero-config `DarkmatterPage::new(&terminal).render(&md)` matches `for_terminal(&md, TerminalOptions::default())` for representative prose, heading, list, quote, code, table, image-link, and horizontal-rule fixtures.

## Phase 3: Component Layout Integration

- [x] Thread the layout context through `darkmatter/lib/src/markdown/output/terminal.rs` alongside `TerminalOptions` without changing the public `for_terminal` API for existing callers.
- [x] Add internal render entry points that accept an optional layout context and keep the old entry points delegating with no layout context.
- [x] Apply per-component `PageAlignment` to images, block quotes, tables, code blocks, and lists only; leave the main document stream left-aligned.
- [x] Apply `PageFill::Full`, `Pad`, `Indent`, `Max`, and `Explicit` semantics to the same component set, resolving `WidthUnit::Percent` against content width and capping by effective width.
- [x] Update code block rendering to honor effective component width and fill while preserving existing line-number alignment, top/bottom padding rows, syntax background behavior, and trailing newline behavior.
- [x] Ensure `Pad` and `Indent` still reduce component width when the page background is transparent, with reclaimed cells rendered as transparent whitespace.
- [x] Ensure explicit text background colors inside content still override page background, then reset back to the page background instead of terminal default when appropriate.
- [x] Keep horizontal-rule frontmatter margins additive inside the page content rectangle and verify `Layout::resolve_margin` behavior remains local to each rule.
- [x] [parallelizable] Add focused unit tests for `WidthUnit` resolution and `PageFill` width math independent of terminal ANSI rendering.
- [x] [parallelizable] Add focused terminal rendering tests for each component kind with left, center, and right alignment.
- [x] Validation checkpoint: run the darkmatter lib test subset covering terminal output, code blocks, horizontal rules, and new layout tests.

## Phase 4: Browser Rendering

- [x] Implement `BrowserRenderable` for `DarkmatterPage`.
- [x] Add or reuse an HTML render path that wraps markdown browser output in a page-level element with `margin`, `padding`, `max-width`, and `background-color` styles using the v1 `ch` unit mapping.
- [x] Translate `PageBackground::Transparent`, `Subtle`, and `Pronounced` into named color constants shared with terminal rendering where practical.
- [x] Translate per-component alignment into browser wrappers using `text-align` or auto margins, whichever matches the component behavior.
- [x] Translate `PageFill::Pad`, `Indent`, `Max`, and `Explicit` into component wrapper CSS padding, `max-width`, or `width` styles.
- [x] Preserve existing HTML output when no page layout settings are applied or make the new wrapper opt-in through `DarkmatterPage` only, so existing `md --output html` behavior does not change unless wired intentionally in the CLI.
- [x] [parallelizable] Add browser golden tests for margin, padding, max width, background, per-component alignment, and fill CSS output.
- [x] Validation checkpoint: run the browser HTML golden tests and confirm the generated styles match the spec's `ch` unit mapping.

## Phase 5: CLI Integration

- [x] Add CLI margin flags to `darkmatter/cli/src/args.rs`: `-m` / `--margin`, `--mx`, `--my`, `--mt`, `--mb`, `--ml`, and `--mr`, using unsigned parsing so negative numbers fail at parse time.
- [x] Add CLI padding flags: `--padding`, `--px`, `--py`, `--pt`, `--pb`, `--pl`, and `--pr`, mirroring margin behavior.
- [x] Add CLI page flags: `--page-bg` with `--page-background` alias, `--max-width`, and `--line-numbers` (with `--no-line-numbers` for explicit disable), rejecting `--max-width 0` at parse time.
- [x] Add CLI alignment flags: `--alignment`, `--align-images`, `--align-lists`, `--align-block-quotes`, `--align-tables`, and `--align-code-blocks`.
- [x] Add CLI fill flags: `--fill`, `--fill-images`, `--fill-lists`, `--fill-block-quotes`, `--fill-tables`, and `--fill-code-blocks`.
- [x] Implement the fill value parser for `full`, `pad=<n|n%>`, `indent=<n|n%>`, `max=<n|n%>`, and `explicit=<n|n%>`, rejecting unknown kinds, negative numbers, and percentages above `100`.
- [x] Implement CLI precedence resolution: margin shorthand, axis shorthand, then side-specific flags; padding shorthand, axis shorthand, then side-specific flags; global alignment then component-specific alignment; global fill then component-specific fill.
- [x] Update `darkmatter/cli/src/output.rs` so the default terminal render path constructs a `Terminal`, builds a `DarkmatterPage`, applies existing `TerminalOptions` knobs and new layout flags, and calls `.render(&md)`.
- [x] Preserve existing output behavior for `md doc.md` with no new flags.
- [x] [parallelizable] Add CLI integration tests for margin precedence, padding precedence, alignment precedence, fill grammar success cases, fill grammar failures, negative numeric rejection, and `--max-width 0` rejection.
- [x] Validation checkpoint: run the darkmatter CLI integration tests and manually compare `md doc.md` output before and after with no layout flags for a small fixture.

## Phase 6: Acceptance, Documentation, And Final Verification

- [x] Add or update snapshots for the end-to-end example described in the spec, covering transparent margin rows, subtle background padding rows, 100-column content, and bottom padding and margin rows.
- [x] Add unit tests proving every `PageRenderError` variant is reachable from public API calls: `MarginsExceedTerminalWidth`, `MaxWidthZero`, `InvalidPercent`, and `Render`.
- [x] Add a pronounced-background test on a controlled dark terminal context proving the effective render color mode becomes `Light` and the page surface uses the pronounced dark-terminal contrast color.
- [x] Add regression tests proving zero-config equivalence against the pre-existing `for_terminal` path for the representative fixture set.
- [x] Update public Rust docs for `darkmatter::layout` and its builder API, following the repo rustdoc convention: summary, `Examples`, `Returns`, `Errors`, `Panics`, `Safety`, `Notes` as applicable, and no `# H1` inside `///` blocks.
- [x] Update README or package documentation if the CLI or library public behavior is user-visible there.
- [x] Update `docs/dependencies.md` and the per-area dependency docs only if new crates were added or removed.
- [x] Update `.claude/skills/darkmatter/SKILL.md` if the new layout architecture changes the authoritative darkmatter workflow summary.
- [x] Run formatting for touched Rust and Markdown files with the repo's established commands.
- [x] Run targeted tests for `darkmatter` lib and CLI.
- [x] Run broader validation through the root `just test` or the relevant curated area test command if the targeted suite passes and runtime is acceptable.
- [x] Validation checkpoint: verify all acceptance criteria from the spec are covered by tests or documented manual checks before marking the feature complete.
