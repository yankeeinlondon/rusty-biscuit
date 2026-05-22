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
//! Tests skip silently when `WEZTERM_UNIX_SOCKET` is not set or the
//! `wezterm` binary is missing. Set `DARKMATTER_LEVEL2_REQUIRED=1` in the
//! environment to convert that into a hard failure — CI jobs that provision
//! WezTerm should set this so Level 2 coverage is enforced, not nominal.

use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness, skip_with_reason};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter, fold_markdown_to_document,
};
use renderable::tree::{RenderStrictness, SourceDescriptor};
use serial_test::serial;
use std::fs;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn level2_required() -> bool {
    matches!(
        std::env::var("DARKMATTER_LEVEL2_REQUIRED").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

static SHARED_HARNESS: Mutex<Option<WezTermHarness>> = Mutex::new(None);
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
    write_doc_to_tempfile(doc, name)
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
    let source = SourceDescriptor::Virtual { name: name.into() };
    let md: Markdown = body.into();
    let (doc, diags) = fold_markdown_spanned_with_frontmatter(source, &md);
    assert!(
        diags.is_empty(),
        "Level 2 span-aware fixture must fold without diagnostics: {diags:?}"
    );
    write_doc_to_tempfile(doc, name)
}

/// Shared write-and-render path for both `render_tree_terminal_to_tempfile`
/// and `render_tree_terminal_spanned_to_tempfile`. Pins a 120-wide
/// optimistic terminal so the visible width is repeatable.
fn write_doc_to_tempfile(
    doc: renderable::tree::Document,
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let term = Terminal::new_optimistic(120);
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

/// Shared pane driver — the fold choice is decided by the caller-supplied
/// `render` closure.
fn drive_pane(
    body: &str,
    name: &str,
    render: fn(&str, &str) -> (tempfile::TempDir, std::path::PathBuf),
) -> Option<(CapturedFrame, tempfile::TempDir)> {
    if !WezTermHarness::available() {
        if level2_required() {
            panic!(
                "DARKMATTER_LEVEL2_REQUIRED=1 set but WezTerm is unavailable. \
                 Provision WezTerm in this environment or unset the variable."
            );
        }
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return None;
    }

    let (dir, path) = render(body, name);
    let mut guard = SHARED_HARNESS
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if guard.is_none() {
        let mut harness = WezTermHarness::new();
        harness.spawn_shell().expect("spawn_shell failed");
        *guard = Some(harness);
    }
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
/// source as visible text. Without the terminal renderer consuming the
/// `darkmatter.hr.*` hints, the rule degrades to the default dashed style;
/// this test enforces that the renderer honors the `style: waves` hint
/// (review-4 finding 2).
#[test]
#[serial(level2_terminal)]
fn level2_tree_hr_attributes_render_styled_rule_in_real_terminal() {
    let body = "Lead paragraph.\n\n--- { style: waves }\n\nTrailing paragraph.\n";
    let Some((frame, _dir)) = run_in_pane_spanned(body, "hr_attributes_spanned") else {
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

    // The waves rule glyph is `≋` (U+224B) in Unicode-capable terminals or
    // `~` in ASCII fallback. A plain (dashed) rule renders `─` (U+2500) or
    // `-`, so either waves marker is sufficient to prove the renderer
    // honored the `style: waves` hint instead of falling back to the
    // default.
    let has_waves_glyph = frame.plain.contains('\u{224B}') || frame.plain.contains('~');
    assert!(
        has_waves_glyph,
        "expected waves rule glyph (`≋` or `~`) in styled HR output; plain:\n{}",
        frame.plain
    );
    // Sanity: the plain dashed rule glyph must not dominate (the test would
    // also pass if Tea-Time Unicode font missing forces all-`~` ASCII, so
    // we only assert *positive* evidence of the waves style above).
}
