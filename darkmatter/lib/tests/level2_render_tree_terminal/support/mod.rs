pub(super) use biscuit_terminal::discovery::detection::ImageSupport;
pub(super) use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
pub(super) use biscuit_terminal::terminal::Terminal;
pub(super) use biscuit_test_harness::shared::SharedHarness;
pub(super) use biscuit_test_harness::wezterm::WezTermHarness;
pub(super) use biscuit_test_harness::{CapturedFrame, TerminalHarness};
pub(super) use darkmatter::layout::{ComponentPolicy, DarkmatterPage, PageComponent};
pub(super) use darkmatter::markdown::Markdown;
pub(super) use darkmatter::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};
pub(super) use darkmatter::markdown::output::{ColorDepth, TerminalOptions};
pub(super) use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter, fold_markdown_to_document,
};
pub(super) use renderable::layout::{Alignment, Length, TargetValue};
pub(super) use renderable::tree::{RenderStrictness, SourceDescriptor};
pub(super) use serial_test::serial;
pub(super) use std::fs;
pub(super) use std::rc::Rc;
pub(super) use std::sync::atomic::{AtomicU32, Ordering};
pub(super) use std::time::{Duration, Instant};
pub(super) use tempfile::tempdir;
pub(super) use test_toolkit::{Level, LevelDecision, evaluate_level};

/// Gating decision for Level-2 WezTerm tests.
pub(super) fn wezterm_decision() -> LevelDecision {
    evaluate_level(Level::L2, WezTermHarness::available(), "WezTerm")
}

pub(super) static SHARED_HARNESS: SharedHarness<WezTermHarness> = SharedHarness::new();
pub(super) static SENTINEL_COUNTER: AtomicU32 = AtomicU32::new(0);
pub(super) const SENTINEL_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn wait_for_sentinel(
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

pub(super) fn run_with_sentinel(harness: &mut WezTermHarness, cmd: &str) -> CapturedFrame {
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
pub(super) fn render_tree_terminal_to_tempfile(
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
pub(super) fn render_tree_terminal_spanned_to_tempfile(
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
pub(super) fn render_tree_terminal_spanned_text_tier_to_tempfile(
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
pub(super) fn render_tree_terminal_spanned_vector_to_tempfile(
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
pub(super) fn fold_spanned_doc(body: &str, name: &str) -> renderable::tree::Document {
    let source = SourceDescriptor::Virtual { name: name.into() };
    let md: Markdown = body.into();
    let (doc, diags) = fold_markdown_spanned_with_frontmatter(source, &md)
        .expect("span-aware fold must succeed");
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
pub(super) fn write_doc_to_tempfile(
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
pub(super) fn run_in_pane(body: &str, name: &str) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_to_tempfile)
}

/// Drives the shared WezTerm pane with the **span-aware** fold path.
pub(super) fn run_in_pane_spanned(body: &str, name: &str) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_spanned_to_tempfile)
}

/// Drives the shared WezTerm pane with the **span-aware** fold at the text
/// tier (images disabled) so glyph-based components render as observable text.
pub(super) fn run_in_pane_spanned_text_tier(
    body: &str,
    name: &str,
) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_spanned_text_tier_to_tempfile)
}

/// Drives the shared WezTerm pane with the **span-aware** fold at
/// [`GraphicsMode::Vector`](renderable::tree::GraphicsMode::Vector) on an
/// image-capable terminal, so the glyph form proves policy suppression.
pub(super) fn run_in_pane_spanned_vector(
    body: &str,
    name: &str,
) -> Option<(CapturedFrame, tempfile::TempDir)> {
    drive_pane(body, name, render_tree_terminal_spanned_vector_to_tempfile)
}

/// Shared pane driver — the fold choice is decided by the caller-supplied
/// `render` closure.
pub(super) fn drive_pane(
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
pub(super) fn background_colors(row: &str) -> Vec<String> {
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
pub(super) fn foreground_colors(row: &str) -> Vec<String> {
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
pub(super) fn foreground_color_set(frame: &CapturedFrame) -> Vec<String> {
    let mut set: Vec<String> = frame.raw.lines().flat_map(foreground_colors).collect();
    set.sort();
    set.dedup();
    set
}

/// Environment variable that switches this test binary into render-probe mode.
/// `drive_render_probe` sets it (to the variant) when it re-execs the binary
/// inside the pane; [`level2_render_probe_entrypoint`] reads it.
pub(super) const RENDER_PROBE_ENV: &str = "DM_L2_RENDER_PROBE";

/// A table (the layout-only policy target) plus unrelated capability-bearing
/// content: a syntax-highlighted code block and an OSC8 hyperlink. The probe
/// renders this through `DarkmatterPage::render` so the capture can compare the
/// `no-policy` and `matched` variants' real-terminal capability output.
pub(super) const RENDER_PROBE_FIXTURE: &str = "| A | B |\n|---|---|\n| 1 | 2 |\n\n\
                                    ```rust\nfn demo() { let x = 1; }\n```\n\n\
                                    [link](https://example.com)\n";

/// Renders [`RENDER_PROBE_FIXTURE`] to stdout for `variant` (`no-policy` /
/// `matched`).
///
/// `Terminal::default()` detects the ambient terminal (width, color depth,
/// OSC8). Pinning TrueColor keeps the color axis deterministic; the OSC8 axis is
/// left to ambient detection on purpose — that is the capability this probe
/// exists to exercise against a real pane. The `matched` variant adds a
/// *matched* layout-only `Tables` policy (the document's table matches it) that
/// caps the table width and centers it. The `max-width` cap is load-bearing:
/// since `Width::Auto` now fills the available width, center alignment alone
/// leaves a full-width table with no visible offset — the cap makes the table
/// narrower than the pane so centering shifts it right, which
/// [`level2_matched_layout_policy_matches_no_policy_capabilities_in_real_terminal`]
/// relies on as proof the policy actually matched. Both are layout-only and bake
/// no color, so a correct renderer leaves the unrelated code block's color and
/// the link's OSC8 behavior identical to `no-policy`.
pub(super) fn render_probe_to_stdout(variant: &str) {
    let term = Terminal::default();
    let md: Markdown = RENDER_PROBE_FIXTURE.into();

    let mut page = DarkmatterPage::new(&term).with_color_depth(ColorDepth::TrueColor);
    if variant == "matched" {
        let mut policy = ComponentPolicy::default();
        policy.layout.alignment = Alignment::Center;
        policy.layout.max_width = Some(TargetValue::universal(Length::Ch(40)));
        page = page.with_component_policy(PageComponent::Tables, policy);
    }

    let rendered = page.render(&md).expect("DarkmatterPage::render");
    print!("{rendered}");
}

/// Render-probe entry point. When this test binary is spawned with
/// [`RENDER_PROBE_ENV`] set (as `drive_render_probe` does inside the pane), it
/// renders the named variant straight to the real-tty stdout and exits, so
/// `DarkmatterPage::render` resolves OSC8 against the actual terminal. With the
/// variable unset (an ordinary suite run) it is an inert pass.
///
/// Living inside the integration-test executable keeps the probe test-only — it
/// adds no production `bin` target to the Darkmatter library package.
/// Runs the render probe for `variant` (`no-policy` / `matched`) by re-executing
/// **this test binary** as a foreground command inside the shared WezTerm pane,
/// with [`RENDER_PROBE_ENV`] set so [`level2_render_probe_entrypoint`] renders
/// the page to the pane's real tty instead of running the suite. Returns `None`
/// when WezTerm is unavailable (the test should skip).
pub(super) fn drive_render_probe(variant: &str) -> Option<CapturedFrame> {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let exe = std::env::current_exe().expect("resolve current test executable");

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    run_with_sentinel(harness, "clear");
    // `--nocapture` lets the probe's `print!` reach the pane stdout; `--exact`
    // selects only the probe entry point so the rest of the suite never runs in
    // the spawned process.
    let cmd = format!(
        "{RENDER_PROBE_ENV}={variant} {} --exact public_entry_points::level2_render_probe_entrypoint --nocapture --test-threads=1",
        exe.display()
    );
    Some(run_with_sentinel(harness, &cmd))
}

/// Every OSC8 hyperlink **opener** carrying a URI — `ESC ] 8 ; ; <uri>` up to
/// its `ESC`/ST terminator — found in `raw`. `wezterm cli get-text --escapes`
/// re-emits the full opener including the URI, so this exposes the hyperlink
/// *metadata* the ANSI-stripped `plain` view hides. The empty closer
/// (`ESC ] 8 ; ;` with no URI) is dropped so only real links are compared.
pub(super) fn osc8_openers(raw: &str) -> Vec<String> {
    const INTRO: &str = "\u{1b}]8;;";
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find(INTRO) {
        let after = &rest[start..];
        // The opener ends at the next ESC (ST is `ESC \`).
        let end = after[1..]
            .find('\u{1b}')
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let opener = &after[..end];
        if opener.len() > INTRO.len() {
            out.push(opener.to_string());
        }
        rest = &after[end..];
    }
    out.sort();
    out.dedup();
    out
}

// Review-13: rich fenced code blocks are a user-observable terminal feature
// (info-string title, line-number gutter, highlighted lines) but were only
// verified at Level 1 (in-process string assertions in `entrypoints.rs` and
// `render_tree_parity.rs`). This Level 2 test drives the **same wired
// [`TerminalCodeRenderer`]** through a real WezTerm pane and asserts the
// title, body, line-number layout, and highlighted-line styling survive a
// real terminal — closing the review's remaining production blocker.
//
// The fixture mirrors the `code_block_rich` parity fixture: a `rust` block
// with `title="Demo Snippet"`, `line-numbering=true`, and `highlight=2`.








// ---------------------------------------------------------------------------
// Span-aware Level 2 coverage (review-3 finding 3)
//
// These tests exercise the **span-aware** fold path so user-visible darkmatter
// inline behavior — mark highlighting, dim styling, and styled horizontal
// rules — is verified against a real terminal, not just as in-process strings.
// ---------------------------------------------------------------------------

// `==marked==` must render the visible text and apply reverse-video SGR
// (the terminal renderer's `mark`-class treatment via `<reverse>`). Without
// the span-aware fold the `==` delimiters would leak through as literals.
// `⌄dimmed⌄` must render the visible text and apply the dim SGR
// (`ESC [ 2 m`) from `Style.emphasis.dim` set by the span-aware fold.
// `--- { style: waves }` must render the **styled** horizontal rule —
// distinguishable from a plain rule — and must not leak the raw markdown
// source as visible text. Without the terminal renderer consuming the typed
// `thematic_break` styling, the rule degrades to the default dashed style;
// this test enforces that the renderer honors the `style: waves` attribute
// (review-4 finding 2).
// Finding 1 (review-1): `GraphicsMode::Vector` must suppress the HR image
// tier by **policy**, not capability. This renders the `style: waves` rule on
// an *image-capable* optimistic terminal (the same one that rasterizes at
// `Rich`) but with the policy pinned to `Vector`, and asserts the captured
// pane shows the waves **glyph** line — proving the rule degraded to text and
// emitted no image payload even though the terminal could have rasterized.


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
pub(super) fn github_light_bg() -> (u8, u8, u8) {
    let bg = CodeHighlighter::new(ThemePair::Github, ColorMode::Light)
        .theme()
        .settings
        .background
        .expect("github light theme has a background");
    (bg.r, bg.g, bg.b)
}

/// One run of visible cells in a captured row that share a background
/// classification (target background or not).
pub(super) struct BgRun {
    is_target: bool,
    width: usize,
}

/// Updates the active background color `cur` from one SGR parameter list.
///
/// Handles the colon true-color form (`48:2::r:g:b`, WezTerm's wire form), the
/// legacy semicolon form (`48;2;r;g;b`), the 256-color introducer (`48;5;n`,
/// tracked as a non-target color), and the resets (`0`, empty, `49`).
pub(super) fn update_bg(cur: &mut Option<(u8, u8, u8)>, params: &str) {
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
pub(super) fn bg_runs(row: &str, target: (u8, u8, u8)) -> Vec<BgRun> {
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
pub(super) fn target_extent(row: &str, target: (u8, u8, u8)) -> Option<(usize, usize, usize)> {
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
pub(super) fn run_page_in_pane(
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
/// Review-1 finding 1 (blank-line behavior): between two code blocks the
/// rendered pane must not contain a run of two or more consecutive *blank* rows
/// (the Markdown vertical-rhythm invariant). A background-filled panel padding
/// row is **not** blank; "blank" means visibly empty with no code-panel
/// background. This checks the rhythm on the real terminal grid.
/// A valid 2×2 RGB PNG, embedded so the image-node Level-2 test needs no
/// on-disk fixture or `image`-crate dependency.
pub(super) const TINY_PNG: &[u8] = &[
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
/// actually decoded and painted lives in the Level-3 test
/// `level3_rich_image_node_paints_distinctive_pixels`
/// (`darkmatter/lib/tests/level3_image_painting.rs`), which screen-captures
/// the pane and samples the rendered pixels. Pixel-readback is gated to L3
/// because `screencapture` requires raising the WezTerm window to the
/// foreground, which the harness contract reserves for L3.
///
/// Encodes a `size`×`size` opaque PNG filled with a single RGB color.
pub(super) fn write_solid_png(path: &std::path::Path, size: u32, rgb: [u8; 3]) {
    let img = image::RgbImage::from_pixel(size, size, image::Rgb(rgb));
    img.save_with_format(path, image::ImageFormat::Png)
        .expect("encode probe PNG");
}

/// Counts pixels in `png` that are near `target` RGB (per-channel within
/// `tol`), and the total pixels that are not near-black. Returns
/// `(near_target, non_black, total)`.
pub(super) fn classify_pixels(png: &[u8], target: [u8; 3], tol: i32) -> (u64, u64, u64) {
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

// Validates the pixel-classification pipeline used by
// `level3_rich_image_node_paints_distinctive_pixels`
// (`darkmatter/lib/tests/level3_image_painting.rs`) without a terminal:
// a solid-magenta PNG must classify as (near-)all magenta and all non-black,
// while a solid-black PNG must register as near-zero non-black (the signature
// the paint test treats as "capture blocked → skip"). This guards the decode +
// threshold logic independently of the WezTerm/`screencapture` environment.
// Review-5 finding 3: the previous Level-2 image test proves only that the
// iTerm2 protocol bytes are emitted and that WezTerm does not surface the
// `[cat]` alt-text fallback — a dropped or malformed payload passes it too,
// because text capture strips graphics bytes. This test closes that gap with
// **pixel-readback**: it renders a `240×240` solid-magenta image through the
// production tree path, paints it into a real WezTerm pane, screen-captures the
// window via `screencapture`, and asserts the distinctive magenta is actually
// on screen. Magenta (`#ff00ff`) does not occur in terminal chrome, text, or
// the theme background, so its presence proves the image was decoded and
// painted — not merely that bytes were consumed.
//
// ## Skips cleanly
//
// Skips when WezTerm is unavailable (like the other Level-2 tests), when the
// harness cannot capture the window region (off macOS, or `screencapture`
// fails), or when the capture comes back essentially black — the signature of
// missing Screen Recording permission, which cannot be distinguished from a
// genuine paint failure and so must not hard-fail. Set
// `BISCUIT_TEST_LEVEL_REQUIRED=2` to enforce the WezTerm prerequisite; the
// pixel assertion still self-skips on a black capture.


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
pub(super) fn render_public_as_terminal_to_tempfile(
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
pub(super) fn render_zero_config_page_to_tempfile(
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
pub(super) fn render_no_policy_page_to_tempfile(
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
pub(super) fn render_unmatched_policy_page_to_tempfile(
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

// Finding 7: the public `Markdown::as_terminal` entry must survive a real
// terminal — heading, prose, fenced-code body + language header, and SGR
// styling all reach the pane through the post-cutover tree path.
// Review-4 finding 1 (High): an *unmatched* component policy must not change
// the capability profile of unrelated content — the real-terminal render must
// be the same with and without the policy.
//
// Both pages pin TrueColor and render the same fixture (prose + a syntax-
// highlighted code block); one also carries a colored `Tables` policy the
// document never matches. Driving both through a real WezTerm pane and
// comparing the captured foreground-color sets proves the policy changes no
// capability: the colored code is identical. Before the fix the unmatched
// policy routed the render through the optimistic terminal, so its colored
// output diverged from the no-policy baseline. This is the Level 2 parity
// companion to the Level 1 byte test in `page.rs`.
// Review-5 finding 1 (High): a *matched* layout-only component policy must not
// change the renderer-wide capability profile — neither rendered color nor
// hyperlink behavior — of unrelated content. This is the real-terminal
// companion to the Level 1 capability-signature test in `page.rs`.
//
// ## Why the render runs *inside* the pane
//
// The no-geometry page path resolves hyperlink (OSC8) capability through
// `Terminal::default()` detection at render time, which returns "no OSC8"
// whenever `is_tty()` is false (`biscuit-terminal`'s `osc8_link_support`). A
// cargo-test process has no controlling tty, so an *in-process* render — even
// when its bytes are later `cat` into a pane — has already decided OSC8 is off:
// the real terminal never participates in the hyperlink decision (review-6
// finding 1). The earlier version compared only the ANSI-stripped `plain` view
// for the substring `example.com`, which a capture that discards hyperlink
// metadata, or a regression that degrades both renders identically, would still
// pass.
//
// So both variants are rendered by [`drive_render_probe`] — which re-execs this
// test binary's [`level2_render_probe_entrypoint`] as a foreground command
// *inside* the WezTerm pane (a real tty). Each renders the identical fixture (a
// table + a syntax-highlighted code block + a hyperlink); `matched` also caps
// the table width and centers it via a layout-only policy the table matches.
// The assertions
// then compare the captured foreground colors **and** the actual OSC8 hyperlink
// openers (`wezterm cli get-text --escapes` re-emits the full opener including
// the URI, verified), proving the matched layout changes no capability: the
// colored code is byte-identical and the link emits the same real OSC8 escape.
//
// ## What this does and does not catch
//
// In a fully OSC8-capable terminal both variants emit OSC8, so this test does
// not by itself catch the *promotion* regression (a matched policy forcing the
// optimistic profile) — that regression is only observable where the ambient
// terminal lacks a capability the optimistic profile would force, which is the
// Level 1 `capability_signature` test's job (it renders with `is_tty()` false
// so ambient and optimistic diverge). This test supplies the complementary
// real-terminal evidence the Level 1 test cannot: that `DarkmatterPage::render`
// emits a well-formed OSC8 hyperlink a real terminal honors, and that the
// matched layout policy leaves it — and the unrelated code color — untouched.
// Finding 7: zero-config `DarkmatterPage::render` (the other public post-flip
// entry) must likewise survive a real terminal through the default-layout tree
// path.
// Review-2 finding (High): percentage **page-frame** margin and max-width must
// resolve to the correct *visible* offset and content width in a real
// terminal, not just in-process. Level 1 covers the cell arithmetic
// (`page.rs::percent_frame_browser_emits_percent_terminal_resolves_cells`);
// this drives the resolved bytes through a real WezTerm pane so a regression in
// the decorated-frame offset/width is observable on the cell grid.
//
// Built at the pane's true column width with `style.page.left-margin: 25%` and
// `style.page.max-width: 50%`, the captured paragraph rows must (a) begin at
// the 25%-of-width left offset and (b) cap their content at 50% of the
// post-margin width — mirroring [`length_to_cells`]'s `f32::round`.


// ---------------------------------------------------------------------------
// `::file-links` directive Level 2 coverage (review-1 findings 1 + 2)
//
// The directive's user-observable output — a styled FileSystem tree with a
// dimmed root prefix, a highlighted target directory, and OSC8 file links —
// is produced by composing the directive and then rendering the composed
// document. The compose path now carries the FileSystem render subtree
// losslessly via `renderable::tree::embed`, so this drives the FULL pipeline
// (compose → fold → terminal) into a real WezTerm pane and asserts the visible
// hierarchy, the dimmed-prefix SGR, and the OSC8 link bytes survive — coverage
// the Level 1 component-config tests could not provide.
// ---------------------------------------------------------------------------

/// Builds a temp repo fixture, composes a `::file-links` directive through the
/// full default pipeline, renders the composed document to terminal bytes, and
/// `cat`s them into the shared WezTerm pane. Returns the captured frame and the
/// fixture tempdir (kept alive so the `.ansi` file outlives the `cat`). Skips
/// (returns `None`) when WezTerm is unavailable.
pub(super) fn run_file_links_in_pane(name: &str) -> Option<(CapturedFrame, tempfile::TempDir)> {
    use darkmatter::markdown::compose::ComposeOptions;
    use darkmatter::markdown::output::terminal::{DimMode, HyperlinkMode, TerminalImageMode};

    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    let topics = dir.path().join("docs").join("topics");
    fs::create_dir_all(&topics).unwrap();
    // Representative document extensions (distinct Unicode glyphs), a dotfile
    // (italic), and a gitignored document (dim) so the capture can verify the
    // full presentation contract — not just plain `.md` hierarchy.
    fs::write(topics.join("alpha.md"), "# Alpha\n").unwrap();
    fs::write(topics.join("beta.md"), "# Beta\n").unwrap();
    fs::write(topics.join("notes.txt"), "notes\n").unwrap();
    fs::write(topics.join("report.pdf"), "pdf\n").unwrap();
    fs::write(topics.join("sheet.xlsx"), "xls\n").unwrap();
    fs::write(topics.join("memo.docx"), "doc\n").unwrap();
    fs::write(topics.join(".hidden.md"), "# Hidden\n").unwrap();
    // `.gitignore` itself is not a document extension, so it never appears in
    // the tree; it only causes `ignored.md` to render dim.
    fs::write(topics.join(".gitignore"), "ignored.md\n").unwrap();
    fs::write(topics.join("ignored.md"), "# Ignored\n").unwrap();
    // A nested subtree with its OWN `.gitignore`. The nested rule must dim
    // `sub/buried.md` (proving `.gitignore` files below the component root are
    // evaluated with directory-scoped Git semantics) while `sub/nested.md`
    // exercises a per-file OSC8 link at depth.
    let sub = topics.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".gitignore"), "buried.md\n").unwrap();
    fs::write(sub.join("nested.md"), "# Nested\n").unwrap();
    fs::write(sub.join("buried.md"), "# Buried\n").unwrap();

    let root = dir.path().join("root.md");
    fs::write(&root, "# Root\n\n::file-links --dir docs/topics --depth 1\n").unwrap();

    let md = Markdown::try_from(root.as_path()).unwrap();
    let (composed, _report) = md
        .compose_with(ComposeOptions::new().with_source_file(&root))
        .expect("compose ::file-links");

    // `TerminalOptions` is `#[non_exhaustive]`; build from the default and force
    // the capabilities the assertions depend on (truecolor, dim, OSC8 links).
    let mut options = TerminalOptions::default();
    options.color_mode = ColorMode::Dark;
    options.color_depth = Some(ColorDepth::TrueColor);
    options.image_mode = TerminalImageMode::Never;
    options.dim_mode = DimMode::Always;
    options.hyperlink_mode = HyperlinkMode::Always;
    options.max_width = Some(100);
    let rendered = composed.as_terminal(options).expect("render composed ::file-links");

    let path = dir.path().join(format!("{name}.ansi"));
    fs::write(&path, rendered).unwrap();

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();
    run_with_sentinel(harness, "clear");
    let frame = run_with_sentinel(harness, &format!("cat {}", path.display()));
    Some((frame, dir))
}

/// Removes OSC sequences (`ESC ] … ST|BEL`) — including OSC8 hyperlinks and
/// their `file://` payloads — from `raw`, leaving SGR styling and visible text.
/// Style assertions run against this so a token (e.g. `topics`) keys off visible
/// output instead of the same substring inside a hyperlink URL.
pub(super) fn strip_osc(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b']') {
            i += 2;
            while i < bytes.len() {
                if bytes[i] == 0x07 {
                    i += 1;
                    break;
                }
                if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'\\') {
                    i += 2;
                    break;
                }
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// SGR attributes active at the first occurrence of `token` in OSC-stripped
/// `text`: bold = 1, dim = 2, italic = 3. Color introducers (`38;2;…`,
/// `38;5;…`, `48;…`) are parsed structurally so their parameters are never
/// mistaken for attributes (the `2` in `38;2` is not dim). Returns `None` when
/// `token` is absent.
pub(super) fn active_sgr_params(text: &str, token: &str) -> Option<std::collections::HashSet<u16>> {
    let idx = text.find(token)?;
    let prefix = &text[..idx];
    let bytes = prefix.as_bytes();
    let mut active: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'm' && bytes[j] != 0x1b {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'm' {
                apply_sgr_params(&prefix[start..j], &mut active);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    Some(active)
}

/// Folds one `ESC [ … m` parameter list into the active attribute set.
pub(super) fn apply_sgr_params(params: &str, active: &mut std::collections::HashSet<u16>) {
    if params.is_empty() {
        active.clear(); // `ESC[m` is an alias for `ESC[0m`.
        return;
    }
    let parts: Vec<&str> = params.split(';').collect();
    let mut k = 0;
    while k < parts.len() {
        match parts[k].parse::<u16>() {
            Ok(0) => active.clear(),
            Ok(1) => {
                active.insert(1);
            }
            Ok(2) => {
                active.insert(2);
            }
            Ok(3) => {
                active.insert(3);
            }
            Ok(22) => {
                active.remove(&1);
                active.remove(&2);
            }
            Ok(23) => {
                active.remove(&3);
            }
            // Skip the operands of an extended color so they are not read as
            // attributes: `38;5;n` consumes one, `38;2;r;g;b` consumes three.
            Ok(38) | Ok(48) => match parts.get(k + 1) {
                Some(&"5") => k += 2,
                Some(&"2") => k += 4,
                _ => {}
            },
            _ => {}
        }
        k += 1;
    }
}
