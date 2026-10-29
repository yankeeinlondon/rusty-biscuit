//! `md`'s Mermaid options: `--mermaid` selects automatic mode, the three
//! fallback features fold into the page's fallback policy (a flag beats
//! `MERMAID_FALLBACK`; with no flag the variable applies), and
//! `no-mermaid-rendering` keeps every fence a code block in every output.
//!
//! Terminal output only renders on a TTY, so the terminal cases build the
//! page `md` renders (`render::terminal_page`) in-process; the HTML, Markdown,
//! and JSON cases spawn `md`.

use crate::common::{CliProcessFixture, md_file};
use biscuit_terminal::terminal::Terminal;
use biscuit_visualized::components::mermaid::PARTIAL_NOTE_ID;
use biscuit_visualized::components::{MERMAID_FALLBACK_ENV, MermaidFallbackPolicy, TextFallback};
use clap::Parser;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::MermaidMode;
use darkmatter::markdown::output::terminal::ColorDepth;
use darkmatter_cli::args::{Command, Feature};
use darkmatter_cli::render::terminal_page;
use darkmatter_cli::Cli;
use test_toolkit::EnvGuard;

const FLOWCHART_DOC: &str = "# Diagram\n\n```mermaid\nflowchart LR\n    Alpha --> Beta\n```\n";
/// The flowchart's source line: present when the fence renders as code.
const FLOWCHART_SOURCE: &str = "Alpha --> Beta";
const PIE_DOC: &str = "```mermaid\npie title Pets\n    \"Dogs\" : 386\n```\n";

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("md").chain(args.iter().copied())).expect("valid arguments")
}

/// The features `md` hands the renderer: the top-level list plus `render`'s.
fn dispatched_features(cli: &Cli) -> Vec<Feature> {
    match cli.command_or_render() {
        Command::Render(options) => options.features,
        _ => cli.render.features.clone(),
    }
}

/// Pins the process the terminal page reads its capabilities from: no
/// graphics, and `MERMAID_FALLBACK` as given.
fn terminal_process(fallback: Option<&str>) -> [EnvGuard; 2] {
    [
        EnvGuard::set_safe("BISCUIT_GRAPHICS", "none"),
        match fallback {
            Some(value) => EnvGuard::set_safe(MERMAID_FALLBACK_ENV, value),
            None => EnvGuard::remove_safe(MERMAID_FALLBACK_ENV),
        },
    ]
}

fn page_for(args: &[&str], doc: &str) -> (darkmatter::layout::DarkmatterPage, Markdown) {
    let cli = parse(args);
    let md = Markdown::try_from_content(doc).unwrap();
    let term = Terminal::new_optimistic(80);
    // A default-layout page detects color from the process, not from `term`,
    // unless its depth is set: pin it so `NO_COLOR` or `TERM=dumb` cannot
    // strip the highlighting these tests assert.
    let page = terminal_page(&md, None, &cli, &term, false, &dispatched_features(&cli))
        .expect("terminal page")
        .with_color_depth(ColorDepth::TrueColor);
    (page, md)
}

/// The terminal output `md` writes for `args`, with SGR sequences kept.
fn terminal_raw(args: &[&str], doc: &str) -> String {
    let (page, md) = page_for(args, doc);
    page.render(&md).expect("terminal render")
}

fn terminal(args: &[&str], doc: &str) -> String {
    let raw = terminal_raw(args, doc);
    let mut plain = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // CSI ... final byte; the renderer emits SGR only.
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            plain.push(ch);
        }
    }
    plain
}

fn is_drawn(output: &str) -> bool {
    !output.contains(FLOWCHART_SOURCE) && output.contains("Alpha") && output.contains("Beta")
}

#[test]
fn mermaid_flag_selects_automatic_mermaid_mode() {
    let _env = terminal_process(None);

    for args in [&["--mermaid", "doc.md"][..], &["--mermaid", "render", "doc.md"][..]] {
        let (page, _) = page_for(args, FLOWCHART_DOC);
        assert_eq!(page.terminal_options().mermaid_mode, MermaidMode::Automatic, "{args:?}");
    }
    let (page, _) = page_for(&["doc.md"], FLOWCHART_DOC);
    assert_eq!(page.terminal_options().mermaid_mode, MermaidMode::Off);

    // Without images, automatic mode draws the flowchart as text and keeps the
    // pie, which has no text drawing, as code.
    let flowchart = terminal(&["--mermaid", "doc.md"], FLOWCHART_DOC);
    assert!(is_drawn(&flowchart), "{flowchart}");
    let pie = terminal(&["--mermaid", "doc.md"], PIE_DOC);
    assert!(pie.contains("pie title Pets"), "{pie}");
    assert!(terminal(&["doc.md"], FLOWCHART_DOC).contains(FLOWCHART_SOURCE));
}

#[test]
fn mermaid_render_without_a_flag_follows_the_fallback_env() {
    let drawn = {
        let _env = terminal_process(None);
        terminal(&["--mermaid", "render", "doc.md"], FLOWCHART_DOC)
    };
    assert!(is_drawn(&drawn), "{drawn}");

    let _env = terminal_process(Some("Code-Block"));
    let source = terminal(&["--mermaid", "render", "doc.md"], FLOWCHART_DOC);
    assert!(source.contains(FLOWCHART_SOURCE), "{source}");
}

#[test]
fn mermaid_no_fallbacks_flag_beats_the_fallback_env() {
    for fallback in [None, Some("text")] {
        let _env = terminal_process(fallback);
        for feature in ["no-mermaid-fallbacks", "no-mermaid-partial-fallbacks"] {
            let output = terminal(&["--mermaid", "render", "doc.md", "--feature", feature], FLOWCHART_DOC);
            assert!(output.contains(FLOWCHART_SOURCE), "{feature} with {fallback:?}:\n{output}");
        }
    }
}

#[test]
fn mermaid_partial_warning_feature_hides_the_partial_note() {
    let _env = terminal_process(None);

    let with_note = terminal(&["--mermaid", "doc.md"], FLOWCHART_DOC);
    assert!(is_drawn(&with_note) && with_note.contains(PARTIAL_NOTE_ID), "{with_note}");

    let without = terminal(&["--mermaid", "--feature", "no-mermaid-partial-warning", "doc.md"], FLOWCHART_DOC);
    assert!(is_drawn(&without) && !without.contains(PARTIAL_NOTE_ID), "{without}");
}

#[test]
fn mermaid_fallback_features_fold_into_the_page_policy() {
    let policy = |args: &[&str]| page_for(args, FLOWCHART_DOC).0.terminal_options().mermaid_fallback_policy;

    assert_eq!(policy(&["--mermaid", "doc.md"]), MermaidFallbackPolicy::default());
    assert_eq!(
        policy(&["--feature", "no-mermaid-partial-fallbacks,no-mermaid-fallbacks", "doc.md"]),
        MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Never)
    );
    assert_eq!(
        policy(&["render", "doc.md", "--feature", "no-mermaid-partial-warning"]),
        MermaidFallbackPolicy::default().with_show_partial_note(false)
    );
}

#[test]
fn mermaid_fallback_features_without_the_mermaid_flag_change_nothing() {
    let _env = terminal_process(None);
    let features = "no-mermaid-fallbacks,no-mermaid-partial-fallbacks,no-mermaid-partial-warning";

    assert_eq!(
        terminal_raw(&["--feature", features, "doc.md"], FLOWCHART_DOC),
        terminal_raw(&["doc.md"], FLOWCHART_DOC)
    );
}

#[test]
fn no_mermaid_rendering_keeps_the_terminal_fence_a_code_block_over_mermaid() {
    let _env = terminal_process(None);

    let (page, _) = page_for(&["--mermaid", "--feature", "no-mermaid-rendering", "doc.md"], FLOWCHART_DOC);
    assert_eq!(page.terminal_options().mermaid_mode, MermaidMode::Text);

    let code = terminal_raw(&["--mermaid", "render", "doc.md", "--feature", "no-mermaid-rendering"], FLOWCHART_DOC);
    assert!(code.contains("\x1b[38;2;"), "the code block is highlighted:\n{code:?}");
    assert_eq!(code, terminal_raw(&["doc.md"], FLOWCHART_DOC), "the same code block as without --mermaid");
}

/// Spawns `md` on `doc` with `args` and returns stdout.
fn spawn(fixture: &CliProcessFixture, doc: &tempfile::NamedTempFile, args: &[&str]) -> String {
    let output = fixture.command().args(args).arg(doc.path()).output().unwrap();
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn no_mermaid_rendering_html_is_a_highlighted_code_block_without_mermaid_js() {
    let fixture = CliProcessFixture::named("no_mermaid_rendering_html");
    let doc = md_file(FLOWCHART_DOC);

    let interactive = spawn(&fixture, &doc, &["--output", "html"]);
    assert!(interactive.contains(r#"class="mermaid""#), "{interactive}");
    assert!(interactive.contains("mermaid.esm.min.mjs"), "{interactive}");

    let code = spawn(&fixture, &doc, &["--output", "html", "--feature", "no-mermaid-rendering"]);
    assert!(code.contains(r#"class="language-mermaid""#), "{code}");
    assert!(!code.contains(r#"class="mermaid""#) && !code.contains("mermaid.esm"), "{code}");
    assert!(!code.contains("<svg"), "{code}");
    let colors: std::collections::BTreeSet<&str> = code
        .match_indices("<span style=\"color: #")
        .map(|(at, prefix)| &code[at + prefix.len()..at + prefix.len() + 6])
        .collect();
    assert!(colors.len() > 1, "the Mermaid source is highlighted:\n{code}");

    let over_everything = spawn(
        &fixture,
        &doc,
        &["--mermaid", "--output", "html", "--feature", "embed-mermaid-font,no-mermaid-rendering"],
    );
    assert_eq!(over_everything, code);
    assert!(!over_everything.contains("@font-face"));
}

#[test]
fn no_mermaid_rendering_keeps_the_markdown_and_json_mermaid_fence() {
    let fixture = CliProcessFixture::named("no_mermaid_rendering_text_outputs");
    let doc = md_file(FLOWCHART_DOC);
    let feature = ["--mermaid", "--feature", "no-mermaid-rendering"];

    let markdown = spawn(&fixture, &doc, &[&["--output", "markdown"][..], &feature].concat());
    assert!(markdown.contains("```mermaid\nflowchart LR\n    Alpha --> Beta\n```"), "{markdown}");
    assert_eq!(markdown, spawn(&fixture, &doc, &["--output", "markdown"]));

    let json = spawn(&fixture, &doc, &[&["--output", "json"][..], &feature].concat());
    let value: serde_json::Value = serde_json::from_str(&json).expect("json output");
    let text = value.to_string();
    assert!(text.contains(r#""mermaid""#) && text.contains("flowchart LR"), "{json}");
    assert_eq!(json, spawn(&fixture, &doc, &["--output", "json"]));
}
