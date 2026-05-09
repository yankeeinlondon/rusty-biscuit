---
phases: 6
created: 2026-05-09
start_phase: 1
---

# Darkmatter Layout Execution Plan

Source: `darkmatter/features/2026-05-08-darkmatter-layout/spec.md`

## Phase 1: Baseline Discovery And API Scaffolding

- [ ] Confirm the current workspace package names with `cargo metadata --no-deps --format-version 1` and identify the exact `darkmatter` lib and CLI package names for targeted test commands.
- [ ] Read the current terminal render path in `darkmatter/lib/src/markdown/output/terminal.rs`, including `TerminalOptions`, `for_terminal`, `write_terminal`, `LineWrapper`, horizontal-rule rendering, and existing component dispatch points.
- [ ] Read `darkmatter/lib/src/markdown/output/code_block.rs` and identify how code block width, padding rows, line numbers, and background resets are currently emitted.
- [ ] Read `darkmatter/lib/src/markdown/output/html.rs` and identify the lowest-risk insertion point for a page-level browser wrapper.
- [ ] Read `darkmatter/cli/src/args.rs` and `darkmatter/cli/src/output.rs` to map existing render flags into `TerminalOptions`.
- [ ] [parallelizable] Inspect `biscuit-terminal` `Renderable`, `BrowserRenderable`, `Terminal`, color mode, and layout helper APIs needed by `DarkmatterPage`.
- [ ] Create `darkmatter/lib/src/layout/` and define the public module boundary without changing existing render behavior.
- [ ] Add and export the new public layout types: `DarkmatterPage`, `PageMargin`, `PagePadding`, `PageBackground`, `PageComponent`, `PageAlignment`, `PageFill`, `WidthUnit`, and `PageRenderError`.
- [ ] Implement default values that preserve current behavior: zero margin, zero padding, transparent background, no max width, no line numbers, left component alignment, and full component fill.
- [ ] Implement builder methods for margin, padding, page background, max width, line numbers, component alignment, component fill, and the documented `TerminalOptions` passthrough methods.
- [ ] Implement validation helpers for horizontal space, `max_width`, and percent width units, returning the documented `PageRenderError` variants.
- [ ] Validation checkpoint: run `cargo test -p darkmatter layout --lib` or the closest targeted lib test command and confirm the new module compiles with focused unit tests for defaults and validation helpers.

## Phase 2: Terminal Page Rendering Shell

- [ ] Add a layout context type, internal to darkmatter if possible, that carries effective terminal width, content width, effective render width, page background colors, component alignment, and component fill settings.
- [ ] Implement `DarkmatterPage::new(&Terminal)` so it captures terminal width, color mode, and capability information by value and does not borrow the terminal.
- [ ] Implement `DarkmatterPage::render(&self, &Markdown) -> Result<String, PageRenderError>` using the existing terminal renderer as the delegated markdown body renderer.
- [ ] Derive `TerminalOptions` from the page state before delegation, including `max_width = Some(effective_cols)` and `include_line_numbers` from the page builder.
- [ ] Implement `PageBackground::Pronounced` color-mode inversion before markdown rendering and keep the page surface color based on the original terminal color mode.
- [ ] Implement row decoration around delegated terminal output: transparent top and bottom margin rows, background-filled top and bottom padding rows, and left/right margin plus padding per content row.
- [ ] Preserve byte-for-byte output for zero-config rendering by bypassing row decoration when margin, padding, background, max width, line numbers, alignment, and fill are all at defaults.
- [ ] Map underlying markdown render failures into `PageRenderError::Render(String)`.
- [ ] Implement the `biscuit_terminal::renderable::Renderable` trait for `DarkmatterPage` using the same render path and error mapping expected by the trait.
- [ ] Validation checkpoint: add snapshot or string tests proving zero-config `DarkmatterPage::new(&terminal).render(&md)` matches `for_terminal(&md, TerminalOptions::default())` for representative prose, heading, list, quote, code, table, image-link, and horizontal-rule fixtures.

## Phase 3: Component Layout Integration

- [ ] Thread the layout context through `darkmatter/lib/src/markdown/output/terminal.rs` alongside `TerminalOptions` without changing the public `for_terminal` API for existing callers.
- [ ] Add internal render entry points that accept an optional layout context and keep the old entry points delegating with no layout context.
- [ ] Apply per-component `PageAlignment` to images, block quotes, tables, code blocks, and lists only; leave the main document stream left-aligned.
- [ ] Apply `PageFill::Full`, `Pad`, `Indent`, `Max`, and `Explicit` semantics to the same component set, resolving `WidthUnit::Percent` against content width and capping by effective width.
- [ ] Update code block rendering to honor effective component width and fill while preserving existing line-number alignment, top/bottom padding rows, syntax background behavior, and trailing newline behavior.
- [ ] Ensure `Pad` and `Indent` still reduce component width when the page background is transparent, with reclaimed cells rendered as transparent whitespace.
- [ ] Ensure explicit text background colors inside content still override page background, then reset back to the page background instead of terminal default when appropriate.
- [ ] Keep horizontal-rule frontmatter margins additive inside the page content rectangle and verify `Layout::resolve_margin` behavior remains local to each rule.
- [ ] [parallelizable] Add focused unit tests for `WidthUnit` resolution and `PageFill` width math independent of terminal ANSI rendering.
- [ ] [parallelizable] Add focused terminal rendering tests for each component kind with left, center, and right alignment.
- [ ] Validation checkpoint: run the darkmatter lib test subset covering terminal output, code blocks, horizontal rules, and new layout tests.

## Phase 4: Browser Rendering

- [ ] Implement `BrowserRenderable` for `DarkmatterPage`.
- [ ] Add or reuse an HTML render path that wraps markdown browser output in a page-level element with `margin`, `padding`, `max-width`, and `background-color` styles using the v1 `ch` unit mapping.
- [ ] Translate `PageBackground::Transparent`, `Subtle`, and `Pronounced` into named color constants shared with terminal rendering where practical.
- [ ] Translate per-component alignment into browser wrappers using `text-align` or auto margins, whichever matches the component behavior.
- [ ] Translate `PageFill::Pad`, `Indent`, `Max`, and `Explicit` into component wrapper CSS padding, `max-width`, or `width` styles.
- [ ] Preserve existing HTML output when no page layout settings are applied or make the new wrapper opt-in through `DarkmatterPage` only, so existing `md --output html` behavior does not change unless wired intentionally in the CLI.
- [ ] [parallelizable] Add browser golden tests for margin, padding, max width, background, per-component alignment, and fill CSS output.
- [ ] Validation checkpoint: run the browser HTML golden tests and confirm the generated styles match the spec's `ch` unit mapping.

## Phase 5: CLI Integration

- [ ] Add CLI margin flags to `darkmatter/cli/src/args.rs`: `-m` / `--margin`, `--mx`, `--my`, `--mt`, `--mb`, `--ml`, and `--mr`, using unsigned parsing so negative numbers fail at parse time.
- [ ] Add CLI padding flags: `--padding`, `--px`, `--py`, `--pt`, `--pb`, `--pl`, and `--pr`, mirroring margin behavior.
- [ ] Add CLI page flags: `--page-bg` with `--page-background` alias, `--max-width`, and `--line-numbers <true|false>`, rejecting `--max-width 0` at parse time.
- [ ] Add CLI alignment flags: `--alignment`, `--align-images`, `--align-lists`, `--align-block-quotes`, `--align-tables`, and `--align-code-blocks`.
- [ ] Add CLI fill flags: `--fill`, `--fill-images`, `--fill-lists`, `--fill-block-quotes`, `--fill-tables`, and `--fill-code-blocks`.
- [ ] Implement the fill value parser for `full`, `pad=<n|n%>`, `indent=<n|n%>`, `max=<n|n%>`, and `explicit=<n|n%>`, rejecting unknown kinds, negative numbers, and percentages above `100`.
- [ ] Implement CLI precedence resolution: margin shorthand, axis shorthand, then side-specific flags; padding shorthand, axis shorthand, then side-specific flags; global alignment then component-specific alignment; global fill then component-specific fill.
- [ ] Update `darkmatter/cli/src/output.rs` so the default terminal render path constructs a `Terminal`, builds a `DarkmatterPage`, applies existing `TerminalOptions` knobs and new layout flags, and calls `.render(&md)`.
- [ ] Preserve existing output behavior for `md doc.md` with no new flags.
- [ ] [parallelizable] Add CLI integration tests for margin precedence, padding precedence, alignment precedence, fill grammar success cases, fill grammar failures, negative numeric rejection, and `--max-width 0` rejection.
- [ ] Validation checkpoint: run the darkmatter CLI integration tests and manually compare `md doc.md` output before and after with no layout flags for a small fixture.

## Phase 6: Acceptance, Documentation, And Final Verification

- [ ] Add or update snapshots for the end-to-end example described in the spec, covering transparent margin rows, subtle background padding rows, 100-column content, and bottom padding and margin rows.
- [ ] Add unit tests proving every `PageRenderError` variant is reachable from public API calls: `MarginsExceedTerminalWidth`, `MaxWidthZero`, `InvalidPercent`, and `Render`.
- [ ] Add a pronounced-background test on a controlled dark terminal context proving the effective render color mode becomes `Light` and the page surface uses the pronounced dark-terminal contrast color.
- [ ] Add regression tests proving zero-config equivalence against the pre-existing `for_terminal` path for the representative fixture set.
- [ ] Update public Rust docs for `darkmatter::layout` and its builder API, following the repo rustdoc convention: summary, `Examples`, `Returns`, `Errors`, `Panics`, `Safety`, `Notes` as applicable, and no `# H1` inside `///` blocks.
- [ ] Update README or package documentation if the CLI or library public behavior is user-visible there.
- [ ] Update `docs/dependencies.md` and the per-area dependency docs only if new crates were added or removed.
- [ ] Update `.claude/skills/darkmatter/SKILL.md` if the new layout architecture changes the authoritative darkmatter workflow summary.
- [ ] Run formatting for touched Rust and Markdown files with the repo's established commands.
- [ ] Run targeted tests for `darkmatter` lib and CLI.
- [ ] Run broader validation through the root `just test` or the relevant curated area test command if the targeted suite passes and runtime is acceptable.
- [ ] Validation checkpoint: verify all acceptance criteria from the spec are covered by tests or documented manual checks before marking the feature complete.
