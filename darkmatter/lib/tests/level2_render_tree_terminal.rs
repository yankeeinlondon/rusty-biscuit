//! Level-2 (real-terminal) tests for the render-tree terminal renderer.
//!
//! These tests run inside a WezTerm pane via the shared `biscuit-test-harness`
//! so we observe the actual rendered bytes — ANSI styling, visible widths,
//! line wrapping — for output produced through the tree pipeline
//! (`fold_markdown_to_document` + `render_terminal_document`).
//!
//! Without Level 2 coverage the parity suite can only assert that the tokens
//! survive ANSI-stripped string comparison; it cannot catch regressions where
//! the rendered bytes *look* correct as a string but fail in an actual
//! terminal pane (column width, SGR ordering, wrapping behaviour).
//!
//! ## Mechanism
//!
//! Each test:
//! 1. Folds a small fixture through the render-tree pipeline directly (so we
//!    test the canonical tree → terminal path, not the legacy
//!    `for_terminal` event-stream serializer).
//! 2. Writes the rendered output to a temp file.
//! 3. Spawns a WezTerm pane and runs `cat <tempfile>` so the bytes are
//!    actually emitted into a real terminal.
//! 4. Captures the rendered frame and asserts on visible structure.
//!
//! Tests skip cleanly when `WEZTERM_UNIX_SOCKET` is not set or the
//! `wezterm` binary is missing. Set `BISCUIT_TEST_LEVEL_REQUIRED=2` in the
//! environment to convert that into a hard failure — CI jobs that provision
//! WezTerm should set this so Level 2 coverage is enforced, not nominal.
//!
//! ## Bespoke page-path coverage
//!
//! Beyond the render-tree pipeline, this file also drives the **bespoke
//! [`DarkmatterPage`] path** — the path the reported code-block defects live in
//! (theme inversion, right-margin gap, pill/body boundary). Those tests render
//! the real repro layout at the pane's true width and assert on the captured
//! *cell grid* (inverted code-panel background, contiguous rectangle, shared
//! right boundary, blank-line rhythm) — properties an in-process ANSI string
//! cannot verify. See `run_page_in_pane` and the `level2_page_*` tests.

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};
use darkmatter::markdown::output::{ColorDepth, TerminalOptions};
use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter, fold_markdown_to_document,
};
use renderable::tree::{RenderStrictness, SourceDescriptor};
use serial_test::serial;
use std::fs;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, LevelDecision, evaluate_level};

/// Gating decision for Level-2 WezTerm tests.
fn wezterm_decision() -> LevelDecision {
    evaluate_level(Level::L2, WezTermHarness::available(), "WezTerm")
}

static SHARED_HARNESS: SharedHarness<WezTermHarness> = SharedHarness::new();
static SENTINEL_COUNTER: AtomicU32 = AtomicU32::new(0);
const SENTINEL_TIMEOUT: Duration = Duration::from_secs(30);

fn wait_for_sentinel(
    harness: &mut WezTermHarness,
    sentinel: &str,
) -> Result<CapturedFrame, CapturedFrame> {
    let deadline = Instant::now() + SENTINEL_TIMEOUT;
    let mut last = CapturedFrame::from_raw(String::new());
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            if frame.plain.lines().any(|l| l.trim() == sentinel) {
                return Ok(frame);
            }
            last = frame;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(last)
}

fn run_with_sentinel(harness: &mut WezTermHarness, cmd: &str) -> CapturedFrame {
    let id = SENTINEL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let sentinel = format!("__DM_TREE_LVL2_DONE_{id}__");
    let wrapped = format!("{cmd}; printf '\\n{sentinel}\\n'");
    harness
        .send_command_with_env(&wrapped, &[])
        .expect("send_command_with_env failed");
    match wait_for_sentinel(harness, &sentinel) {
        Ok(frame) => frame,
        Err(last) => panic!(
            "timed out waiting for sentinel {sentinel} after {SENTINEL_TIMEOUT:?}. \
             last plain capture:\n{}",
            last.plain
        ),
    }
}

/// Renders `body` through the render-tree terminal pipeline using the
/// **plain** fold (`fold_markdown_to_document`) and writes the resulting
/// bytes to a temp file.
fn render_tree_terminal_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let source = SourceDescriptor::Virtual { name: name.into() };
    let (doc, diags) = fold_markdown_to_document(source, body);
    assert!(
        diags.is_empty(),
        "Level 2 fixture must fold without diagnostics: {diags:?}"
    );
    write_doc_to_tempfile(doc, name, None)
}

/// Renders `body` through the render-tree terminal pipeline using the
/// **span-aware** fold (`fold_markdown_spanned_with_frontmatter`) and writes
/// the resulting bytes to a temp file. Use this for fixtures that exercise
/// darkmatter-inline syntax: `==mark==`, `⌄dim⌄`, and `--- { style: ... }`
/// HR-attribute paragraphs. Closes review-3 finding 3.
fn render_tree_terminal_spanned_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    write_doc_to_tempfile(fold_spanned_doc(body, name), name, None)
}

/// Like [`render_tree_terminal_spanned_to_tempfile`] but renders at the
/// **text tier** (`ImageSupport::None`). `HorizontalRule` emits an embedded
/// image on an image-capable terminal, which the text-capture harness cannot
/// observe; disabling images forces the glyph form so a styled rule (e.g. the
/// `≋` waves glyph) is visible in the captured pane.
fn render_tree_terminal_spanned_text_tier_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    write_doc_to_tempfile(
        fold_spanned_doc(body, name),
        name,
        Some(ImageSupport::None),
    )
}

/// Like [`render_tree_terminal_spanned_to_tempfile`] but pins
/// [`GraphicsMode::Vector`](renderable::tree::GraphicsMode::Vector) while
/// keeping the optimistic terminal's (image-capable) capabilities. This proves
/// the HR image tier is suppressed by **policy** at `Vector` even when the
/// terminal *could* rasterize — finding 1 (review-1).
fn render_tree_terminal_spanned_vector_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let doc = fold_spanned_doc(body, name);
    let term = Terminal::new_optimistic(120);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = renderable::tree::GraphicsMode::Vector;
    let opts = TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    };
    let rendered = render_terminal_document(&doc, &opts).expect("tree terminal render");

    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered.output).unwrap();
    (dir, path)
}

/// Folds `body` through the **span-aware** fold, asserting it folds cleanly.
fn fold_spanned_doc(body: &str, name: &str) -> renderable::tree::Document {
    let source = SourceDescriptor::Virtual { name: name.into() };
    let md: Markdown = body.into();
    let (doc, diags) = fold_markdown_spanned_with_frontmatter(source, &md);
    assert!(
        diags.is_empty(),
        "Level 2 span-aware fixture must fold without diagnostics: {diags:?}"
    );
    doc
}

/// Shared write-and-render path for the render-tree terminal fixtures. Pins a
/// 120-wide optimistic terminal so the visible width is repeatable.
/// `image_override` forces a specific [`ImageSupport`] tier (e.g.
/// [`ImageSupport::None`] to make glyph-based components observable as text);
/// `None` keeps the optimistic terminal's default capabilities.
fn write_doc_to_tempfile(
    doc: renderable::tree::Document,
    name: &str,
    image_override: Option<ImageSupport>,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let mut term = Terminal::new_optimistic(120);
    if let Some(support) = image_override {
        term.image_support = support;
    }
    // Wire darkmatter's code renderer so fenced code blocks reproduce the
    // syntax-highlighted code-block path the production entry point uses,
    // not the render tree's plain-fence fallback (review-10 finding 2).
    let opts = TerminalRenderOptions {
        context: TerminalRenderContext::from_terminal(&term),
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    };
    let rendered = render_terminal_document(&doc, &opts).expect("tree terminal render");

    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered.output).unwrap();
    (dir, path)
}

/// Drives the shared WezTerm pane with the **plain** fold path. Returns
/// `None` when WezTerm is unavailable (the test should skip).
fn run_in_pane(body: &str, name: &str) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_to_tempfile)
}

/// Drives the shared WezTerm pane with the **span-aware** fold path.
fn run_in_pane_spanned(body: &str, name: &str) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_spanned_to_tempfile)
}

/// Drives the shared WezTerm pane with the **span-aware** fold at the text
/// tier (images disabled) so glyph-based components render as observable text.
fn run_in_pane_spanned_text_tier(
    body: &str,
    name: &str,
) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_spanned_text_tier_to_tempfile)
}

/// Drives the shared WezTerm pane with the **span-aware** fold at
/// [`GraphicsMode::Vector`](renderable::tree::GraphicsMode::Vector) on an
/// image-capable terminal, so the glyph form proves policy suppression.
fn run_in_pane_spanned_vector(
    body: &str,
    name: &str,
) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_spanned_vector_to_tempfile)
}

/// Shared pane driver — the fold choice is decided by the caller-supplied
/// `render` closure.
fn drive_pane(
    body: &str,
    name: &str,
    render: fn(&str, &str) -> (tempfile::TempDir, std::path::PathBuf),
) -> Option<(CapturedFrame, tempfile::TempDir)> {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let (dir, path) = render(body, name);
    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    // Reset visible region between tests.
    run_with_sentinel(harness, "clear");

    let cmd = format!("cat {}", path.display());
    let frame = run_with_sentinel(harness, &cmd);
    Some((frame, dir))
}

/// Extracts every background-color descriptor set by the SGR sequences in
/// `row`, normalized so two rows from the *same* capture can be compared
/// regardless of WezTerm's true-color wire form.
///
/// `wezterm cli get-text --escapes` re-emits each cell's attributes, so a code
/// line's background reaches the capture here. WezTerm uses the ITU **colon**
/// form for true color (`48:2::r:g:b`); other terminals emit the legacy
/// **semicolon** form (`48;2;r;g;b`). Both are handled so the highlighted line's
/// background can be compared against a non-highlighted line's.
fn background_colors(row: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(start) = rest.find("\u{1b}[") {
        let after = &rest[start + 2..];
        let Some(mpos) = after.find('m') else {
            break;
        };
        let params = &after[..mpos];
        // Only inspect well-formed SGR parameter lists (digits / `;` / `:`).
        if params
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ';' | ':'))
        {
            let fields: Vec<&str> = params.split(';').collect();
            // Colon form: the background introducer and its components arrive
            // as a single field such as `48:2::r:g:b` or `48:5:n`.
            for field in &fields {
                if field.starts_with("48:") {
                    out.push((*field).to_string());
                }
            }
            // Semicolon form: `48;2;r;g;b` / `48;5;n` — the introducer and its
            // components are separate fields.
            let mut i = 0;
            while i < fields.len() {
                if fields[i] == "48" {
                    let span = match fields.get(i + 1).copied() {
                        Some("2") => 5,
                        Some("5") => 3,
                        _ => 1,
                    };
                    let end = (i + span).min(fields.len());
                    out.push(fields[i..end].join(";"));
                    i = end;
                    continue;
                }
                i += 1;
            }
        }
        rest = &after[mpos + 1..];
    }
    out
}

/// Extracts every foreground-color descriptor set by the SGR sequences in
/// `row`, normalized like [`background_colors`] so two captures can be compared
/// regardless of WezTerm's true-color wire form. The foreground introducer is
/// `38` (colon form `38:2::r:g:b` / `38:5:n`; semicolon form `38;2;r;g;b` /
/// `38;5;n`).
fn foreground_colors(row: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(start) = rest.find("\u{1b}[") {
        let after = &rest[start + 2..];
        let Some(mpos) = after.find('m') else {
            break;
        };
        let params = &after[..mpos];
        if params
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ';' | ':'))
        {
            let fields: Vec<&str> = params.split(';').collect();
            for field in &fields {
                if field.starts_with("38:") {
                    out.push((*field).to_string());
                }
            }
            let mut i = 0;
            while i < fields.len() {
                if fields[i] == "38" {
                    let span = match fields.get(i + 1).copied() {
                        Some("2") => 5,
                        Some("5") => 3,
                        _ => 1,
                    };
                    let end = (i + span).min(fields.len());
                    out.push(fields[i..end].join(";"));
                    i = end;
                    continue;
                }
                i += 1;
            }
        }
        rest = &after[mpos + 1..];
    }
    out
}

/// The sorted, de-duplicated set of foreground colors across every row of a
/// captured frame — the semantic comparison key for two real-terminal renders.
fn foreground_color_set(frame: &CapturedFrame) -> Vec<String> {
    let mut set: Vec<String> = frame.raw.lines().flat_map(foreground_colors).collect();
    set.sort();
    set.dedup();
    set
}

/// Review-13: rich fenced code blocks are a user-observable terminal feature
/// (info-string title, line-number gutter, highlighted lines) but were only
/// verified at Level 1 (in-process string assertions in `entrypoints.rs` and
/// `render_tree_parity.rs`). This Level 2 test drives the **same wired
/// [`TerminalCodeRenderer`]** through a real WezTerm pane and asserts the
/// title, body, line-number layout, and highlighted-line styling survive a
/// real terminal — closing the review's remaining production blocker.
///
/// The fixture mirrors the `code_block_rich` parity fixture: a `rust` block
/// with `title="Demo Snippet"`, `line-numbering=true`, and `highlight=2`.
#[test]
#[serial(level2_terminal)]
fn level2_tree_rich_code_block_title_gutter_and_highlight_survive_real_terminal() {
    let body = "```rust title=\"Demo Snippet\" line-numbering=true highlight=2\n\
                fn parity_demo() {\n    println!(\"render tree\");\n}\n```\n";
    let Some((frame, _dir)) = run_in_pane(body, "code_block_rich") else {
        return;
    };

    // 1. The info-string title surfaces in the code-block header row.
    assert!(
        frame.plain.contains("Demo Snippet"),
        "code-block title missing from real-terminal capture. plain:\n{}",
        frame.plain
    );

    // 2. The code body survives syntax highlighting + the real-terminal trip.
    for token in &["parity_demo", "render tree"] {
        assert!(
            frame.plain.contains(token),
            "code body token {token:?} missing from capture. plain:\n{}",
            frame.plain
        );
    }

    // 3. Line-number gutter: the renderer emits a right-aligned line number
    //    followed by ` │ `. Three source lines produce gutters `1 │`, `2 │`,
    //    `3 │`. The `│` separator appears only in the gutter (the code body
    //    has no pipes), so each numbered separator proves the gutter layout
    //    survived to the pane.
    for gutter in &["1 \u{2502}", "2 \u{2502}", "3 \u{2502}"] {
        assert!(
            frame.plain.contains(gutter),
            "line-number gutter {gutter:?} missing from capture. plain:\n{}",
            frame.plain
        );
    }

    // 4. Syntax highlighting: foreground color SGRs must survive into the real
    //    pane (the plain no-color fallback emits none). WezTerm re-emits true
    //    color in the colon form; accept colon, semicolon, or 256-color.
    assert!(
        frame.raw.contains("\u{1b}[38;2;")
            || frame.raw.contains("\u{1b}[38:2:")
            || frame.raw.contains("\u{1b}[38;5;"),
        "expected foreground syntax-highlight SGRs in code-block capture; raw:\n{}",
        frame.raw
    );

    // 5. Highlighted-line styling: line 2 (`highlight=2`) is painted with a
    //    background distinct from the other code lines. WezTerm re-emits each
    //    cell's background, and `plain`/`raw` lines are index-aligned, so we can
    //    isolate the highlighted row by its text. WezTerm *omits* a background
    //    SGR when the cell background equals the pane default, so the
    //    non-highlighted code lines (whose background matches the theme/pane
    //    default) carry no explicit background sequence — but the highlighted
    //    line, whose background is the theme default plus the highlight delta,
    //    must carry an explicit background SGR that the non-highlighted line
    //    lacks. Comparing two rows from the *same* capture is normalization-proof.
    let plain_rows: Vec<&str> = frame.plain.lines().collect();
    let raw_rows: Vec<&str> = frame.raw.lines().collect();
    let row_index = |needle: &str| {
        plain_rows
            .iter()
            .position(|p| p.contains(needle))
            .unwrap_or_else(|| {
                panic!(
                    "could not locate row {needle:?} in capture.\nplain:\n{}",
                    frame.plain
                )
            })
    };
    let plain_row = row_index("parity_demo"); // line 1 — not highlighted
    let highlighted_row = row_index("render tree"); // line 2 — highlight=2

    let plain_bgs = background_colors(raw_rows[plain_row]);
    let highlight_bgs = background_colors(raw_rows[highlighted_row]);
    assert!(
        !highlight_bgs.is_empty(),
        "highlighted line (`highlight=2`) must carry an explicit background SGR; raw row:\n{}",
        raw_rows[highlighted_row]
    );
    assert_ne!(
        plain_bgs, highlight_bgs,
        "highlighted line (`highlight=2`) must carry a background distinct from a \
         non-highlighted line.\nnon-highlighted raw:\n{}\nhighlighted raw:\n{}",
        raw_rows[plain_row], raw_rows[highlighted_row]
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_heading_text_survives_real_terminal() {
    let body = "# Level Two Marker\n\nSecond line of body content.\n";
    let Some((frame, _dir)) = run_in_pane(body, "heading") else {
        return;
    };

    // The heading text must appear in the rendered pane. ANSI styling is
    // applied by the renderer; the harness's `plain` field has it stripped
    // for matching.
    assert!(
        frame.plain.contains("Level Two Marker"),
        "heading text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("Second line of body content."),
        "paragraph text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_inline_styles_render_with_ansi_in_real_terminal() {
    let body = "Mixing *emphasis* and **strong** in real terminal output.\n";
    let Some((frame, _dir)) = run_in_pane(body, "inline_styles") else {
        return;
    };

    // The styled words must survive the real-terminal round-trip even
    // though SGR runs surround them. The harness strips ANSI for the
    // `plain` view.
    assert!(
        frame.plain.contains("emphasis"),
        "italic word missing. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("strong"),
        "strong word missing. plain:\n{}",
        frame.plain
    );

    // The raw frame should contain at least one SGR sequence — otherwise
    // we've degraded to a no-color rendering even though the fixture is
    // styled.
    assert!(
        frame.raw.contains("\u{1b}["),
        "expected SGR styling in the raw capture but found none. raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_table_cells_visible_in_real_terminal() {
    let body = "| Fruit | Quantity |\n|:------|---------:|\n| apples | 3 |\n| pears | 12 |\n";
    let Some((frame, _dir)) = run_in_pane(body, "table") else {
        return;
    };

    for token in &["Fruit", "Quantity", "apples", "pears", "12"] {
        assert!(
            frame.plain.contains(token),
            "table cell token {token:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }
}

// ---------------------------------------------------------------------------
// Span-aware Level 2 coverage (review-3 finding 3)
//
// These tests exercise the **span-aware** fold path so user-visible darkmatter
// inline behavior — mark highlighting, dim styling, and styled horizontal
// rules — is verified against a real terminal, not just as in-process strings.
// ---------------------------------------------------------------------------

/// `==marked==` must render the visible text and apply reverse-video SGR
/// (the terminal renderer's `mark`-class treatment via `<reverse>`). Without
/// the span-aware fold the `==` delimiters would leak through as literals.
#[test]
#[serial(level2_terminal)]
fn level2_tree_mark_renders_reverse_video_in_real_terminal() {
    let body = "Plain text with ==marked phrase== inside.\n";
    let Some((frame, _dir)) = run_in_pane_spanned(body, "mark_spanned") else {
        return;
    };

    assert!(
        frame.plain.contains("marked phrase"),
        "mark text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains("=="),
        "raw `==` delimiters must not leak through; plain:\n{}",
        frame.plain
    );
    // The reverse-video SGR open code is `ESC [ 7 m`. Accept either the
    // standalone form, a leading-position combined run (`ESC [ 7 ; …`), or
    // a trailing-position combined run (`ESC [ … ; 7 m`) — biscuit-terminal's
    // Prose emitter often collapses a reset and the layer into one sequence
    // like `\x1b[0;7m`.
    let has_reverse = frame.raw.contains("\u{1b}[7m")
        || frame.raw.contains("\u{1b}[7;")
        || frame.raw.contains(";7m")
        || frame.raw.contains(";7;");
    assert!(
        has_reverse,
        "expected reverse-video SGR for mark span; raw:\n{}",
        frame.raw
    );
}

/// `⌄dimmed⌄` must render the visible text and apply the dim SGR
/// (`ESC [ 2 m`) from `Style.emphasis.dim` set by the span-aware fold.
#[test]
#[serial(level2_terminal)]
fn level2_tree_dim_renders_dim_sgr_in_real_terminal() {
    let body = "Plain text with \u{2304}dimmed phrase\u{2304} inside.\n";
    let Some((frame, _dir)) = run_in_pane_spanned(body, "dim_spanned") else {
        return;
    };

    assert!(
        frame.plain.contains("dimmed phrase"),
        "dim text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains('\u{2304}'),
        "raw `⌄` delimiters must not leak through; plain:\n{}",
        frame.plain
    );
    // Dim SGR is `ESC [ 2 m`; accept any position within a combined run.
    let has_dim = frame.raw.contains("\u{1b}[2m")
        || frame.raw.contains("\u{1b}[2;")
        || frame.raw.contains(";2m")
        || frame.raw.contains(";2;");
    assert!(
        has_dim,
        "expected dim SGR (\\x1b[2m) for dim span; raw:\n{}",
        frame.raw
    );
}

/// `--- { style: waves }` must render the **styled** horizontal rule —
/// distinguishable from a plain rule — and must not leak the raw markdown
/// source as visible text. Without the terminal renderer consuming the typed
/// `thematic_break` styling, the rule degrades to the default dashed style;
/// this test enforces that the renderer honors the `style: waves` attribute
/// (review-4 finding 2).
#[test]
#[serial(level2_terminal)]
fn level2_tree_hr_attributes_render_styled_rule_in_real_terminal() {
    let body = "Lead paragraph.\n\n--- { style: waves }\n\nTrailing paragraph.\n";
    // Render at the text tier: an image-capable terminal would emit the rule
    // as an embedded image the text-capture harness cannot see, so the glyph
    // form is forced to make the styled rule observable.
    let Some((frame, _dir)) = run_in_pane_spanned_text_tier(body, "hr_attributes_spanned") else {
        return;
    };

    assert!(
        frame.plain.contains("Lead paragraph"),
        "lead paragraph missing; plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("Trailing paragraph"),
        "trailing paragraph missing; plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains("style: waves"),
        "raw HR markdown source leaked through; plain:\n{}",
        frame.plain
    );

    // Isolate the rule line and require it to be a contiguous run of the
    // waves glyph — `≋` (U+224B) in Unicode-capable terminals, `~` in ASCII
    // fallback. A plain (dashed) rule renders `─` (U+2500) or `-`, so a line
    // made entirely of waves glyphs proves the renderer honored `style: waves`
    // (normalized to the `kind` hint) instead of falling back to the default.
    //
    // A bare `frame.plain.contains('~')` was a false-positive risk: a stray
    // `~` anywhere in the pane (a shell prompt, a `~/path` cwd) satisfied it
    // with no styled rule present. Matching a whole line of glyphs excludes
    // those, and the length floor excludes a lone prompt tilde.
    let is_waves_glyph = |c: char| c == '\u{224B}' || c == '~';
    let waves_rule_line = frame.plain.lines().find(|line| {
        let trimmed = line.trim();
        trimmed.chars().count() >= 10 && trimmed.chars().all(is_waves_glyph)
    });
    assert!(
        waves_rule_line.is_some(),
        "expected a styled waves rule line (a run of `≋` or `~`); plain:\n{}",
        frame.plain
    );
}

/// Finding 1 (review-1): `GraphicsMode::Vector` must suppress the HR image
/// tier by **policy**, not capability. This renders the `style: waves` rule on
/// an *image-capable* optimistic terminal (the same one that rasterizes at
/// `Rich`) but with the policy pinned to `Vector`, and asserts the captured
/// pane shows the waves **glyph** line — proving the rule degraded to text and
/// emitted no image payload even though the terminal could have rasterized.
#[test]
#[serial(level2_terminal)]
fn level2_tree_hr_vector_mode_renders_glyph_not_image() {
    let body = "Lead paragraph.\n\n--- { style: waves }\n\nTrailing paragraph.\n";
    let Some((frame, _dir)) = run_in_pane_spanned_vector(body, "hr_vector_mode") else {
        return;
    };

    assert!(
        !frame.plain.contains("style: waves"),
        "raw HR markdown source leaked through; plain:\n{}",
        frame.plain
    );
    let is_waves_glyph = |c: char| c == '\u{224B}' || c == '~';
    let waves_rule_line = frame.plain.lines().find(|line| {
        let trimmed = line.trim();
        trimmed.chars().count() >= 10 && trimmed.chars().all(is_waves_glyph)
    });
    assert!(
        waves_rule_line.is_some(),
        "Vector mode must render the waves glyph (text tier), not an image; plain:\n{}",
        frame.plain
    );
}

// ---------------------------------------------------------------------------
// Bespoke page-path Level 2 coverage (review-1 finding 1)
//
// The reported code-block rendering defects (#0 theme inversion, #1 right-margin
// gap, #2 pill/body right-boundary mismatch) live in the **bespoke**
// `DarkmatterPage` path under page decoration, and were only verified at Level 1
// (in-process ANSI-string inspection). String inspection cannot tell whether the
// emitted SGR sequences actually paint the intended *cell grid* — e.g. an
// `\x1b[K` clear-to-edge versus background-padded spaces look different on the
// real grid but both "contain a background". These tests drive the real repro
// configuration (`--ml 4 --mr 4`, github theme, dark page) into a WezTerm pane
// and assert on the captured cell grid: the inverted (light) code-panel
// background, a single contiguous background rectangle (no gap), and a shared
// right boundary across all panel rows.
// ---------------------------------------------------------------------------

/// The github *light* theme background (the variant a dark page inverts to for
/// code-block contrast). The renderer emits exactly this RGB when truecolor is
/// forced, so the capture can match it cell-for-cell.
fn github_light_bg() -> (u8, u8, u8) {
    let bg = CodeHighlighter::new(ThemePair::Github, ColorMode::Light)
        .theme()
        .settings
        .background
        .expect("github light theme has a background");
    (bg.r, bg.g, bg.b)
}

/// One run of visible cells in a captured row that share a background
/// classification (target background or not).
struct BgRun {
    is_target: bool,
    width: usize,
}

/// Updates the active background color `cur` from one SGR parameter list.
///
/// Handles the colon true-color form (`48:2::r:g:b`, WezTerm's wire form), the
/// legacy semicolon form (`48;2;r;g;b`), the 256-color introducer (`48;5;n`,
/// tracked as a non-target color), and the resets (`0`, empty, `49`).
fn update_bg(cur: &mut Option<(u8, u8, u8)>, params: &str) {
    // Colon true-color form: a single field like `48:2::r:g:b`.
    for field in params.split(';') {
        if let Some(rest) = field.strip_prefix("48:2:") {
            let comps: Vec<&str> = rest.split(':').filter(|s| !s.is_empty()).collect();
            if comps.len() >= 3
                && let (Ok(r), Ok(g), Ok(b)) =
                    (comps[0].parse(), comps[1].parse(), comps[2].parse())
            {
                *cur = Some((r, g, b));
            }
        }
    }
    // Semicolon forms and resets.
    let f: Vec<&str> = params.split(';').collect();
    let mut k = 0;
    while k < f.len() {
        match f[k] {
            "0" | "" | "49" => *cur = None,
            "48" => match f.get(k + 1) {
                Some(&"2") => {
                    if let (Some(r), Some(g), Some(b)) = (f.get(k + 2), f.get(k + 3), f.get(k + 4))
                        && let (Ok(r), Ok(g), Ok(b)) = (r.parse(), g.parse(), b.parse())
                    {
                        *cur = Some((r, g, b));
                    }
                    k += 5;
                    continue;
                }
                Some(&"5") => {
                    // 256-color background: not our truecolor target.
                    *cur = Some((0, 0, 0));
                    k += 3;
                    continue;
                }
                _ => {}
            },
            _ => {}
        }
        k += 1;
    }
}

/// Walks the SGR sequences in `row`, splitting the visible text into runs tagged
/// by whether the active background equals `target`. Non-`m` CSI sequences are
/// skipped without consuming visible cells. Visible width is counted as `char`s
/// (the fixtures are ASCII, so this equals the cell width).
fn bg_runs(row: &str, target: (u8, u8, u8)) -> Vec<BgRun> {
    let bytes = row.as_bytes();
    let mut runs: Vec<BgRun> = Vec::new();
    let mut cur: Option<(u8, u8, u8)> = None;
    let mut i = 0;
    let mut text_start = 0;

    let flush = |runs: &mut Vec<BgRun>, text: &str, is_target: bool| {
        let width = text.chars().count();
        if width == 0 {
            return;
        }
        if let Some(last) = runs.last_mut()
            && last.is_target == is_target
        {
            last.width += width;
            return;
        }
        runs.push(BgRun { is_target, width });
    };

    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            flush(&mut runs, &row[text_start..i], cur == Some(target));
            // Find the CSI final byte (0x40..=0x7e).
            let mut j = i + 2;
            while j < bytes.len() && !(0x40..=0x7e).contains(&bytes[j]) {
                j += 1;
            }
            if j < bytes.len() {
                if bytes[j] == b'm' {
                    update_bg(&mut cur, &row[i + 2..j]);
                }
                i = j + 1;
                text_start = i;
            } else {
                i = bytes.len();
                text_start = i;
            }
        } else {
            i += 1;
        }
    }
    flush(&mut runs, &row[text_start..], cur == Some(target));
    runs
}

/// `(start_col, end_col, target_run_count)` for the `target` background in
/// `row`, where columns are 0-based visible offsets and `end_col` is exclusive.
/// `target_run_count == 1` is a clean rectangle; `>= 2` means an interior gap.
/// Returns `None` when the row carries no `target` background at all.
fn target_extent(row: &str, target: (u8, u8, u8)) -> Option<(usize, usize, usize)> {
    let runs = bg_runs(row, target);
    let mut col = 0usize;
    let mut start: Option<usize> = None;
    let mut end = 0usize;
    let mut count = 0usize;
    let mut prev_target = false;
    for run in &runs {
        if run.is_target {
            if !prev_target {
                count += 1;
            }
            start.get_or_insert(col);
            end = col + run.width;
        }
        prev_target = run.is_target;
        col += run.width;
    }
    start.map(|s| (s, end, count))
}

/// Renders `md_src` through the **bespoke `DarkmatterPage` path** (the one the
/// reported defects live in) at the pane's real column width, then `cat`s it
/// into the shared WezTerm pane. Returns the captured frame and the pane width
/// so column assertions line up with the layout margins. Skips (returns `None`)
/// when WezTerm is unavailable.
#[allow(clippy::too_many_arguments)]
fn run_page_in_pane(
    md_src: &str,
    name: &str,
    ml: u16,
    mr: u16,
    mt: u16,
    mb: u16,
) -> Option<(CapturedFrame, usize)> {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();
    let cols = harness
        .pane_size()
        .map(|s| s.cols as usize)
        .unwrap_or(80)
        .max(40);

    // Force truecolor + dark mode so the inverted (light) github panel background
    // is emitted as an exact `48;2;r;g;b` the capture can match cell-for-cell.
    let term = Terminal::new_optimistic(cols as u32);
    let md: Markdown = md_src.into();
    let rendered = DarkmatterPage::new(&term)
        .with_color_mode(ColorMode::Dark)
        .with_color_depth(ColorDepth::TrueColor)
        .with_code_theme("github")
        .with_margin_left(ml)
        .with_margin_right(mr)
        .with_margin_top(mt)
        .with_margin_bottom(mb)
        .render(&md)
        .expect("DarkmatterPage render");

    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered).unwrap();

    run_with_sentinel(harness, "clear");
    let frame = run_with_sentinel(harness, &format!("cat {}", path.display()));
    Some((frame, cols))
}

/// Review-1 finding 1: under the repro layout (`--ml 4 --mr 4`, github theme,
/// dark page), the code panel must render in a real terminal as a single
/// contiguous **inverted (light)** background rectangle spanning exactly the
/// content columns `[left, width - right)` on every panel row — and prose must
/// not carry that background. This verifies Defects #0/#1/#2 against the actual
/// cell grid, not just the ANSI byte string.
#[test]
#[serial(level2_terminal)]
fn level2_page_code_panel_is_contiguous_inverted_rectangle() {
    let (ml, mr, mt, mb) = (4u16, 4u16, 1u16, 1u16);
    let body =
        "# A Heading\n\nLead prose paragraph.\n\n```rust\nfn main() {\n    let x = 1;\n}\n```\n";
    let Some((frame, cols)) = run_page_in_pane(body, "page_code_panel", ml, mr, mt, mb) else {
        return;
    };

    let target = github_light_bg();
    let left = ml as usize;
    let right_edge = cols - mr as usize;
    let content_width = right_edge - left;

    let plain_rows: Vec<&str> = frame.plain.lines().collect();
    let raw_rows: Vec<&str> = frame.raw.lines().collect();

    // Every code-panel row must be a single contiguous background run (no
    // `\x1b[K` gap, Defect #1) that ends on the content right boundary
    // (Defect #2 — pill and body coherent). Full-bleed rows (body + padding)
    // additionally fill from the left margin; the right-aligned language pill is
    // narrow chrome that legitimately opens mid-line (spec Stage 4.2).
    let mut full_bleed_rows = 0;
    let mut pill_rows = 0;
    for (idx, raw) in raw_rows.iter().enumerate() {
        if let Some((start, end, count)) = target_extent(raw, target) {
            let plain = plain_rows.get(idx).copied().unwrap_or("");
            assert_eq!(
                count, 1,
                "code panel row must be one contiguous background rectangle (no \\x1b[K gap); \
                 row {idx} plain={plain:?} raw={raw:?}"
            );
            assert_eq!(
                end, right_edge,
                "code panel background must end at the content right edge (width - right = {right_edge}); \
                 row {idx} plain={plain:?}"
            );
            if end - start == content_width {
                assert_eq!(
                    start, left,
                    "full-bleed code row must start at the left margin ({left}); \
                     row {idx} plain={plain:?}"
                );
                full_bleed_rows += 1;
            } else {
                pill_rows += 1;
            }
        }
    }
    assert!(
        full_bleed_rows >= 3,
        "expected >= 3 full-bleed code-panel rows (body lines + top/bottom padding) filling \
         the content rectangle; found {full_bleed_rows}. plain:\n{}",
        frame.plain
    );
    assert!(
        pill_rows >= 1,
        "expected the right-aligned language pill row (narrow chrome ending at the right edge); \
         found {pill_rows}. plain:\n{}",
        frame.plain
    );

    // Contrast: the heading (prose) row must NOT carry the inverted code-panel
    // background — prose follows the real (dark) mode, code inverts.
    let heading_idx = plain_rows
        .iter()
        .position(|p| p.contains("A Heading"))
        .unwrap_or_else(|| panic!("heading row missing from capture. plain:\n{}", frame.plain));
    assert!(
        target_extent(raw_rows[heading_idx], target).is_none(),
        "prose heading row must not carry the inverted code-panel background; raw:\n{}",
        raw_rows[heading_idx]
    );
}

/// Review-1 finding 1 (blank-line behavior): between two code blocks the
/// rendered pane must not contain a run of two or more consecutive *blank* rows
/// (the Markdown vertical-rhythm invariant). A background-filled panel padding
/// row is **not** blank; "blank" means visibly empty with no code-panel
/// background. This checks the rhythm on the real terminal grid.
#[test]
#[serial(level2_terminal)]
fn level2_page_no_double_blank_rows_between_code_blocks() {
    let (ml, mr, mt, mb) = (4u16, 4u16, 0u16, 0u16);
    let body = "```rust\nfn a() {}\n```\n\n\n\n```rust\nfn b() {}\n```\n";
    let Some((frame, _cols)) = run_page_in_pane(body, "page_rhythm", ml, mr, mt, mb) else {
        return;
    };

    let target = github_light_bg();
    let plain_rows: Vec<&str> = frame.plain.lines().collect();
    let raw_rows: Vec<&str> = frame.raw.lines().collect();

    // Restrict the scan to the span between the first and last code-panel row so
    // the shell prompt / sentinel framing is excluded.
    let panel_idxs: Vec<usize> = raw_rows
        .iter()
        .enumerate()
        .filter(|(_, raw)| target_extent(raw, target).is_some())
        .map(|(i, _)| i)
        .collect();
    assert!(
        panel_idxs.len() >= 2,
        "expected two code panels in the capture. plain:\n{}",
        frame.plain
    );
    let (first, last) = (panel_idxs[0], *panel_idxs.last().unwrap());

    let is_blank = |idx: usize| -> bool {
        let visibly_empty = plain_rows.get(idx).is_none_or(|p| p.trim().is_empty());
        let no_panel_bg = target_extent(raw_rows[idx], target).is_none();
        visibly_empty && no_panel_bg
    };

    let mut consecutive_blanks = 0;
    for idx in first..=last {
        if is_blank(idx) {
            consecutive_blanks += 1;
            assert!(
                consecutive_blanks < 2,
                "found a run of >= 2 consecutive blank rows between code blocks (row {idx}); \
                 Markdown vertical-rhythm invariant violated. plain:\n{}",
                frame.plain
            );
        } else {
            consecutive_blanks = 0;
        }
    }
}

/// A valid 2×2 RGB PNG, embedded so the image-node Level-2 test needs no
/// on-disk fixture or `image`-crate dependency.
const TINY_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 2, 0,
    0, 0, 253, 212, 154, 115, 0, 0, 0, 16, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 0, 68, 12,
    16, 10, 0, 31, 238, 3, 253, 139, 95, 20, 212, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

/// Review-3 finding 3: a `Rich` image node through the render-tree path is
/// user-visible terminal graphics, but was only verified for the missing-file
/// and `Off`/`Vector` alt-text fallbacks. This Level-2 test renders a real
/// image node through `render_terminal_document` (the production tree path),
/// confirms the iTerm2 image protocol bytes are emitted, then `cat`s them into
/// a real WezTerm pane to confirm the terminal does not fall back to literal
/// `[cat]` alt text.
///
/// ## Verification scope — protocol + anti-regression
///
/// This test deliberately verifies only two things via text capture:
///
/// 1. **Level 1 (in-process):** the production tree path emits the iTerm2
///    image protocol (OSC 1337) at `Rich`, not the alt-text fallback.
/// 2. **Level 2 (real terminal):** WezTerm consumes those bytes without
///    surfacing the `[cat]` alt-text fallback in its text capture.
///
/// `wezterm cli get-text` strips image-protocol bytes, so the pane assertion is
/// the **absence** of the alt-text fallback — which a malformed, ignored, or
/// zero-visible image payload could also satisfy. Proof that the image is
/// actually decoded and painted lives in
/// [`level2_tree_rich_image_node_paints_distinctive_pixels`], which screen-
/// captures the pane and samples the rendered pixels.
#[test]
#[serial(level2_terminal)]
fn level2_tree_rich_image_node_emits_protocol_and_renders_in_real_terminal() {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    // Fixture image + rendered bytes share one temp dir so the relative
    // `pic.png` resolves against `image_base_path` at render time.
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("pic.png"), TINY_PNG).unwrap();

    let source = SourceDescriptor::Virtual {
        name: "rich_image".into(),
    };
    let (doc, diags) = fold_markdown_to_document(source, "![cat](pic.png)\n");
    assert!(diags.is_empty(), "image fixture must fold cleanly: {diags:?}");

    // Rich tier on an iTerm2-capable TTY (WezTerm renders iTerm2 graphics).
    let mut term = Terminal::new_optimistic(120);
    term.image_support = ImageSupport::ITerm;
    term.is_tty = true;
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = renderable::tree::GraphicsMode::Rich;
    context.image_base_path = Some(dir.path().to_path_buf());
    let opts = TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    };
    let rendered = render_terminal_document(&doc, &opts).expect("tree terminal render");

    // In-process proof the production tree path emitted the inline image
    // protocol (iTerm2 OSC 1337), not the alt-text fallback.
    assert!(
        rendered.output.contains("\u{1b}]1337;File="),
        "Rich image node must emit the iTerm2 image protocol; rendered bytes:\n{:?}",
        rendered.output,
    );
    assert!(
        !rendered.output.contains("[cat]"),
        "successful image render must not emit the bracketed alt-text fallback",
    );

    let path = dir.path().join("rich_image.ansi");
    fs::write(&path, rendered.output).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();
    run_with_sentinel(harness, "clear");
    let frame = run_with_sentinel(harness, &format!("cat {}", path.display()));

    // Anti-regression only: the real terminal must not surface the `[cat]`
    // alt-text fallback. This does NOT prove the image was painted — the
    // harness cannot inspect rendered pixels (see the verification-scope note
    // on this fn). A dropped/ignored payload would also pass this assertion.
    assert!(
        !frame.plain.contains("[cat]"),
        "real terminal showed the alt-text fallback instead of consuming the image. plain:\n{}",
        frame.plain,
    );
}

/// Encodes a `size`×`size` opaque PNG filled with a single RGB color.
fn write_solid_png(path: &std::path::Path, size: u32, rgb: [u8; 3]) {
    let img = image::RgbImage::from_pixel(size, size, image::Rgb(rgb));
    img.save_with_format(path, image::ImageFormat::Png)
        .expect("encode probe PNG");
}

/// Counts pixels in `png` that are near `target` RGB (per-channel within
/// `tol`), and the total pixels that are not near-black. Returns
/// `(near_target, non_black, total)`.
fn classify_pixels(png: &[u8], target: [u8; 3], tol: i32) -> (u64, u64, u64) {
    let img = image::load_from_memory(png).expect("decode screen capture");
    let rgb = img.to_rgb8();
    let mut near_target = 0u64;
    let mut non_black = 0u64;
    let total = (rgb.width() as u64) * (rgb.height() as u64);
    for px in rgb.pixels() {
        let [r, g, b] = px.0;
        let near = (r as i32 - target[0] as i32).abs() <= tol
            && (g as i32 - target[1] as i32).abs() <= tol
            && (b as i32 - target[2] as i32).abs() <= tol;
        if near {
            near_target += 1;
        }
        if r > 30 || g > 30 || b > 30 {
            non_black += 1;
        }
    }
    (near_target, non_black, total)
}

/// Validates the pixel-classification pipeline used by
/// [`level2_tree_rich_image_node_paints_distinctive_pixels`] without a terminal:
/// a solid-magenta PNG must classify as (near-)all magenta and all non-black,
/// while a solid-black PNG must register as near-zero non-black (the signature
/// the paint test treats as "capture blocked → skip"). This guards the decode +
/// threshold logic independently of the WezTerm/`screencapture` environment.
#[test]
fn pixel_classification_distinguishes_magenta_from_black() {
    const MAGENTA: [u8; 3] = [255, 0, 255];
    let dir = tempdir().unwrap();

    let magenta_path = dir.path().join("m.png");
    write_solid_png(&magenta_path, 64, MAGENTA);
    let (near, non_black, total) = classify_pixels(&fs::read(&magenta_path).unwrap(), MAGENTA, 60);
    assert_eq!(total, 64 * 64);
    assert_eq!(near, total, "every magenta pixel must classify as near-target");
    assert_eq!(non_black, total, "magenta is not black");

    let black_path = dir.path().join("b.png");
    write_solid_png(&black_path, 64, [0, 0, 0]);
    let (near, non_black, _) = classify_pixels(&fs::read(&black_path).unwrap(), MAGENTA, 60);
    assert_eq!(near, 0, "black has no magenta");
    assert_eq!(non_black, 0, "black capture must read as blocked/empty");
}

/// Review-5 finding 3: the previous Level-2 image test proves only that the
/// iTerm2 protocol bytes are emitted and that WezTerm does not surface the
/// `[cat]` alt-text fallback — a dropped or malformed payload passes it too,
/// because text capture strips graphics bytes. This test closes that gap with
/// **pixel-readback**: it renders a `240×240` solid-magenta image through the
/// production tree path, paints it into a real WezTerm pane, screen-captures the
/// window via `screencapture`, and asserts the distinctive magenta is actually
/// on screen. Magenta (`#ff00ff`) does not occur in terminal chrome, text, or
/// the theme background, so its presence proves the image was decoded and
/// painted — not merely that bytes were consumed.
///
/// ## Skips cleanly
///
/// Skips when WezTerm is unavailable (like the other Level-2 tests), when the
/// harness cannot capture the window region (off macOS, or `screencapture`
/// fails), or when the capture comes back essentially black — the signature of
/// missing Screen Recording permission, which cannot be distinguished from a
/// genuine paint failure and so must not hard-fail. Set
/// `BISCUIT_TEST_LEVEL_REQUIRED=2` to enforce the WezTerm prerequisite; the
/// pixel assertion still self-skips on a black capture.
#[test]
#[serial(level2_terminal)]
fn level2_tree_rich_image_node_paints_distinctive_pixels() {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    const MAGENTA: [u8; 3] = [255, 0, 255];

    let dir = tempdir().unwrap();
    let png_path = dir.path().join("probe.png");
    write_solid_png(&png_path, 240, MAGENTA);

    let source = SourceDescriptor::Virtual {
        name: "rich_image_pixels".into(),
    };
    let (doc, diags) = fold_markdown_to_document(source, "![probe](probe.png)\n");
    assert!(diags.is_empty(), "image fixture must fold cleanly: {diags:?}");

    let mut term = Terminal::new_optimistic(120);
    term.image_support = ImageSupport::ITerm;
    term.is_tty = true;
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = renderable::tree::GraphicsMode::Rich;
    context.image_base_path = Some(dir.path().to_path_buf());
    let opts = TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    };
    let rendered = render_terminal_document(&doc, &opts).expect("tree terminal render");
    assert!(
        rendered.output.contains("\u{1b}]1337;File="),
        "Rich image node must emit the iTerm2 image protocol",
    );

    let path = dir.path().join("rich_image_pixels.ansi");
    fs::write(&path, rendered.output).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();
    run_with_sentinel(harness, "clear");
    let _ = run_with_sentinel(harness, &format!("cat {}", path.display()));

    let Some(png) = harness.capture_window_png().expect("screen-capture call") else {
        eprintln!(
            "skipping pixel assertion: window-region capture unavailable (non-macOS or screencapture failed)"
        );
        return;
    };

    let (magenta, non_black, total) = classify_pixels(&png, MAGENTA, 60);
    // A near-black capture means screen recording is blocked (no permission); we
    // cannot tell that from a paint failure, so skip rather than hard-fail.
    if non_black * 100 < total {
        eprintln!(
            "skipping pixel assertion: capture is essentially black ({non_black}/{total} non-black) \
             — Screen Recording permission likely not granted to the parent terminal"
        );
        return;
    }

    // A real, non-black capture with no magenta means the image did not paint.
    assert!(
        magenta > 1000,
        "Rich image node did not paint: only {magenta} magenta pixels in a {total}-pixel \
         capture ({non_black} non-black). The image protocol bytes were emitted but the \
         terminal did not render the decoded image.",
    );
}

// ---------------------------------------------------------------------------
// Public post-cutover entry-point Level 2 coverage (review-1 finding 7)
//
// Every other test in this file drives the lower-level `render_terminal_document`
// directly, or the decorated `DarkmatterPage::render` path. None drove
// the PUBLIC, post-flip terminal entry points users actually call —
// `Markdown::as_terminal` and zero-config `DarkmatterPage::render` — through a
// real terminal. Those entry points map options and wire the code renderer at an
// adapter boundary the direct-renderer tests bypass; these two tests close that
// integration gap by capturing the public output in a real WezTerm pane.
// ---------------------------------------------------------------------------

/// Renders `body` through the public [`Markdown::as_terminal`] entry point
/// (post-cutover: the render-tree terminal document renderer), pinning width +
/// TrueColor so the capture is deterministic, and writes the bytes to a temp
/// file.
fn render_public_as_terminal_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let md: Markdown = body.into();
    let mut opts = TerminalOptions::default();
    opts.max_width = Some(120);
    opts.color_depth = Some(ColorDepth::TrueColor);
    let rendered = md.as_terminal(opts).expect("public Markdown::as_terminal render");

    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered).unwrap();
    (dir, path)
}

/// Renders `body` through zero-config [`DarkmatterPage::render`] (no builder
/// calls → the default-layout tree path) and writes the bytes to a temp file.
fn render_zero_config_page_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let term = Terminal::new_optimistic(120);
    let md: Markdown = body.into();
    let rendered = DarkmatterPage::new(&term)
        .render(&md)
        .expect("zero-config DarkmatterPage::render");

    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered).unwrap();
    (dir, path)
}

/// Renders `body` through a baseline [`DarkmatterPage`] with no component
/// policy, pinning TrueColor so the capture is deterministic regardless of the
/// test process's ambient detection. Paired with
/// [`render_unmatched_policy_page_to_tempfile`] for the review-4 parity test.
fn render_no_policy_page_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let term = Terminal::new_optimistic(120);
    let md: Markdown = body.into();
    let rendered = DarkmatterPage::new(&term)
        .with_color_depth(ColorDepth::TrueColor)
        .render(&md)
        .expect("no-policy DarkmatterPage::render");

    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered).unwrap();
    (dir, path)
}

/// Renders `body` through a [`DarkmatterPage`] carrying an *unmatched* colored
/// `Tables` policy, pinning TrueColor to match
/// [`render_no_policy_page_to_tempfile`].
///
/// The document has no table, so the policy bakes nothing; per the review-4 fix
/// the unmatched policy must not change the capability profile of unrelated
/// content. The fenced code block's TrueColor syntax highlighting must therefore
/// be byte-identical to the no-policy render.
fn render_unmatched_policy_page_to_tempfile(
    body: &str,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    use renderable::color::{Color, Tailwind};
    use renderable::style::PaintColor;

    let term = Terminal::new_optimistic(120);
    let md: Markdown = body.into();
    let rendered = DarkmatterPage::new(&term)
        .with_color_depth(ColorDepth::TrueColor)
        .with_component_color(
            darkmatter::layout::PageComponent::Tables,
            PaintColor::new(Color::Tailwind(Tailwind::Red500)),
        )
        .render(&md)
        .expect("unmatched-policy DarkmatterPage::render");

    let dir = tempdir().unwrap();
    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered).unwrap();
    (dir, path)
}

/// Finding 7: the public `Markdown::as_terminal` entry must survive a real
/// terminal — heading, prose, fenced-code body + language header, and SGR
/// styling all reach the pane through the post-cutover tree path.
#[test]
#[serial(level2_terminal)]
fn level2_public_as_terminal_entry_renders_in_real_terminal() {
    let body = "# Public Entry\n\nBody paragraph via the public API.\n\n\
                ```rust\nfn demo() {}\n```\n";
    let Some((frame, _dir)) = drive_pane(body, "public_as_terminal", render_public_as_terminal_to_tempfile)
    else {
        return;
    };

    for token in &["Public Entry", "Body paragraph via the public API.", "demo"] {
        assert!(
            frame.plain.contains(token),
            "public as_terminal token {token:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }
    // The cutover emits a language-label header pill for every fenced block.
    assert!(
        frame.plain.to_lowercase().contains("rust"),
        "fenced-code language header missing from public as_terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.raw.contains("\u{1b}["),
        "expected SGR styling in the public as_terminal capture. raw:\n{}",
        frame.raw
    );
}

/// Review-4 finding 1 (High): an *unmatched* component policy must not change
/// the capability profile of unrelated content — the real-terminal render must
/// be the same with and without the policy.
///
/// Both pages pin TrueColor and render the same fixture (prose + a syntax-
/// highlighted code block); one also carries a colored `Tables` policy the
/// document never matches. Driving both through a real WezTerm pane and
/// comparing the captured foreground-color sets proves the policy changes no
/// capability: the colored code is identical. Before the fix the unmatched
/// policy routed the render through the optimistic terminal, so its colored
/// output diverged from the no-policy baseline. This is the Level 2 parity
/// companion to the Level 1 byte test in `page.rs`.
#[test]
#[serial(level2_terminal)]
fn level2_unmatched_policy_matches_no_policy_color_in_real_terminal() {
    let body = "Lead prose paragraph.\n\n```rust\nfn demo() { let x = 1; }\n```\n";
    let Some((no_policy_frame, _d1)) =
        drive_pane(body, "no_policy_color", render_no_policy_page_to_tempfile)
    else {
        return;
    };
    let Some((unmatched_frame, _d2)) = drive_pane(
        body,
        "unmatched_policy_color",
        render_unmatched_policy_page_to_tempfile,
    ) else {
        return;
    };

    // The code body must survive both round-trips.
    for frame in [&no_policy_frame, &unmatched_frame] {
        assert!(
            frame.plain.contains("demo"),
            "code body token missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }

    let no_policy_fg = foreground_color_set(&no_policy_frame);
    let unmatched_fg = foreground_color_set(&unmatched_frame);

    // Premise: the baseline render is actually colored, so the comparison is
    // meaningful rather than two empty sets.
    assert!(
        !no_policy_fg.is_empty(),
        "test premise: the no-policy code block must carry foreground color in the pane; \
         raw:\n{}",
        no_policy_frame.raw
    );
    // Parity: the unmatched policy must produce the same foreground colors.
    assert_eq!(
        no_policy_fg, unmatched_fg,
        "an unmatched policy must not change the real-terminal foreground colors of \
         unrelated content"
    );
}

/// Finding 7: zero-config `DarkmatterPage::render` (the other public post-flip
/// entry) must likewise survive a real terminal through the default-layout tree
/// path.
#[test]
#[serial(level2_terminal)]
fn level2_zero_config_page_render_renders_in_real_terminal() {
    let body = "# Zero Config Page\n\nNo builder calls means the default-layout tree path.\n";
    let Some((frame, _dir)) = drive_pane(body, "zero_config_page", render_zero_config_page_to_tempfile)
    else {
        return;
    };

    for token in &["Zero Config Page", "No builder calls means the default-layout tree path."] {
        assert!(
            frame.plain.contains(token),
            "zero-config page token {token:?} missing from real-terminal capture. plain:\n{}",
            frame.plain
        );
    }
}

/// Review-2 finding (High): percentage **page-frame** margin and max-width must
/// resolve to the correct *visible* offset and content width in a real
/// terminal, not just in-process. Level 1 covers the cell arithmetic
/// (`page.rs::percent_frame_browser_emits_percent_terminal_resolves_cells`);
/// this drives the resolved bytes through a real WezTerm pane so a regression in
/// the decorated-frame offset/width is observable on the cell grid.
///
/// Built at the pane's true column width with `style.page.left-margin: 25%` and
/// `style.page.max-width: 50%`, the captured paragraph rows must (a) begin at
/// the 25%-of-width left offset and (b) cap their content at 50% of the
/// post-margin width — mirroring [`length_to_cells`]'s `f32::round`.
#[test]
#[serial(level2_terminal)]
fn level2_percent_page_frame_offset_and_width_in_real_terminal() {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();
    let cols = harness
        .pane_size()
        .map(|s| s.cols as usize)
        .unwrap_or(80)
        .max(40);

    // 25% left margin resolves against the full terminal width; 50% max-width
    // resolves against the post-margin content width.
    let expected_left = ((cols as f32) * 0.25).round() as usize;
    let content_base = cols - expected_left;
    let expected_max = ((content_base as f32) * 0.50).round() as usize;

    let sentinel = "Sentinel_pct_frame";
    let body = format!(
        "---\nstyle:\n    page:\n        left-margin: 25%\n        max-width: 50%\n---\n\n\
         {sentinel} lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod \
         tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam.\n"
    );

    let md = Markdown::try_from_content(&body).expect("parse markdown with style frontmatter");
    let (style, _warnings) =
        darkmatter::style::from_frontmatter(md.frontmatter()).expect("parse style frontmatter");
    let term = Terminal::new_optimistic(cols as u32);
    let page = darkmatter::style::apply_page_style(
        DarkmatterPage::new(&term),
        &style,
        darkmatter::style::PageStyleOverrides::default(),
    )
    .expect("apply percentage page style");
    let rendered = page.render(&md).expect("decorated percent frame must render");

    let dir = tempdir().unwrap();
    let path = dir.path().join("percent_frame.ansi");
    fs::write(&path, rendered).unwrap();

    run_with_sentinel(harness, "clear");
    let frame = run_with_sentinel(harness, &format!("cat {}", path.display()));

    // The rendered paragraph is the sentinel row plus its wrapped continuations;
    // every content row carries the resolved left margin as leading spaces.
    let content_rows: Vec<&str> = frame
        .plain
        .lines()
        .skip_while(|l| !l.contains(sentinel))
        .take_while(|l| !l.trim().is_empty())
        .collect();

    assert!(
        content_rows.len() >= 2,
        "50% max-width ({expected_max} cols of {content_base}) must wrap the paragraph onto \
         multiple rows; got {} row(s). plain:\n{}",
        content_rows.len(),
        frame.plain
    );

    for (i, row) in content_rows.iter().enumerate() {
        let leading = row.chars().take_while(|c| *c == ' ').count();
        assert_eq!(
            leading, expected_left,
            "row +{i} must begin at the 25% left offset ({expected_left} cols of {cols}); \
             got {leading}. row: {row:?}"
        );
        let content_width = row.trim_end().chars().count() - leading;
        assert!(
            content_width <= expected_max,
            "row +{i} content width {content_width} exceeds the 50% cap ({expected_max} cols of \
             {content_base}). row: {row:?}"
        );
    }
}
