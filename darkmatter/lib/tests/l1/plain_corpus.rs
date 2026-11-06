//! The plain-output corpus: one document with every element the plain rules
//! name, rendered through `Markdown::as_terminal` and `DarkmatterPage`, with
//! and without a frame and a max-width, at 40 columns, 80 columns, and
//! unlimited.

use biscuit_terminal::output::{OutputTarget, render_output};
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageBackground};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions, ComposeRequest, RequestSnapshot};
use darkmatter::markdown::output::command::DocumentOutput;
use darkmatter::markdown::output::{ColorDepth, HyperlinkMode, TerminalOptions};
use renderable::color::{ColorDepth as ProfileColorDepth, ColorMode as ProfileColorMode};
use renderable::output::{OutputOptions, TerminalProfile, Width};

const LONG_PARAGRAPH: &str = "A long paragraph that keeps going well past any narrow width, so that \
it has to wrap more than once at forty columns and at eighty columns too, while under an unlimited \
width it stays on a single line from its first word all the way through to its very last word.";

const SOFT_BROKEN: &str = "A soft-broken paragraph\nthat continues here\nand ends here.";

fn corpus() -> String {
    format!(
        "# Plain Corpus\n\n{SOFT_BROKEN}\n\n{LONG_PARAGRAPH}\n\nRun `md compose` with **bold words** and *italic words*.\n\n\
         ```yaml\nname: string(required)\n```\n\n\
         | Name | Type |\n| --- | --- |\n| id | ulid |\n\n---\n\n> Quoted text keeps a visible marker.\n\n\
         - Item one\n    - Nested item\n- Item two\n\n1. First\n2. Second\n\n\
         See [the docs](https://example.com/docs) and <https://example.com/same>.\n\n\
         ![A diagram](./diagram.png)\n\n\
         ::disclosure\nDisclosure summary\n::details\nDisclosure body.\n::end-disclosure\n\n\
         ```mermaid\nflowchart LR\n    A --> B\n```\n"
    )
}

#[derive(Debug, Clone, Copy)]
enum Path {
    AsTerminal,
    Page,
    FramedPage,
    MaxWidthPage,
    FramedMaxWidthPage,
}

const MAX_WIDTH: u32 = 30;
const FRAME: u16 = 2;

fn plain_options(width: Width, verbosity: u8) -> OutputOptions {
    OutputOptions::new(width, TerminalProfile::plain()).with_verbosity(verbosity)
}

fn page(path: Path) -> DarkmatterPage {
    // The captured terminal is deliberately rich: nothing about it may leak
    // through a plain profile.
    let page = DarkmatterPage::new(&Terminal::new_optimistic(120));
    match path {
        Path::AsTerminal | Path::Page => page,
        Path::FramedPage => page.with_margin_x(FRAME),
        Path::MaxWidthPage => page.with_max_width(MAX_WIDTH),
        Path::FramedMaxWidthPage => page.with_margin_x(FRAME).with_max_width(MAX_WIDTH),
    }
}

fn render_markdown(source: &str, path: Path, width: Width, verbosity: u8) -> String {
    let document = Markdown::from(source);
    let options = plain_options(width, verbosity);
    match path {
        Path::AsTerminal => document
            .as_terminal(TerminalOptions::default().with_output_options(&options))
            .expect("as_terminal"),
        _ => page(path).with_output_options(&options).render(&document).expect("page render"),
    }
}

fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_plain_rules(output: &str) {
    assert!(!output.contains('\u{1b}'), "an escape byte:\n{output:?}");
    for line in output.lines() {
        assert_eq!(line, line.trim_end(), "trailing whitespace:\n{output}");
    }
    let lines: Vec<&str> = output.lines().map(str::trim).collect();
    assert!(lines.contains(&"---"), "a `---` rule:\n{output}");
    assert!(lines.contains(&"Plain Corpus"), "a heading without markers:\n{output}");
    assert!(!output.contains('#'), "no heading marker:\n{output}");
    assert!(output.contains("> Quoted text"), "a `> ` quote:\n{output}");
    assert!(!output.contains('▐'), "no quote bar:\n{output}");
    assert!(output.contains('┌') && output.contains("│ id"), "a box-drawn table:\n{output}");
    assert!(lines.contains(&"```yaml"), "a kept fence:\n{output}");
    assert!(lines.contains(&"name: string(required)"), "whole code lines:\n{output}");
    assert!(lines.contains(&"```mermaid"), "mermaid stays a fence:\n{output}");
    assert!(collapsed(output).contains("`md compose`"), "kept backticks:\n{output}");
    assert!(collapsed(output).contains("bold words"), "{output}");
    assert!(!output.contains("**"), "no bold markers:\n{output}");
    assert!(lines.contains(&"- Item one") && lines.contains(&"- Nested item"), "{output}");
    assert!(lines.contains(&"1. First"), "{output}");

    let summary = lines.iter().position(|line| *line == "Disclosure summary").expect("summary");
    let body = lines[summary + 1..].iter().find(|line| !line.is_empty()).expect("body");
    assert_eq!(*body, "Disclosure body.", "contents with no bar glyph:\n{output}");
    assert!(!output.contains('▉'), "no block placeholder:\n{output}");
}

fn assert_link_rule_default(output: &str) {
    let text = collapsed(output);
    assert!(text.contains("See the docs and https://example.com/same."), "{output}");
    assert!(!text.contains("example.com/docs"), "{output}");
    assert!(text.contains("IMAGE[A diagram]"), "{output}");
    assert!(!text.contains("diagram.png"), "{output}");
}

fn assert_link_rule_verbose(output: &str) {
    let text = collapsed(output);
    assert!(text.contains("See the docs <https://example.com/docs>"), "{output}");
    assert_eq!(text.matches("https://example.com/same").count(), 1, "the URL once:\n{output}");
    assert!(text.contains("image: A diagram <./diagram.png>"), "{output}");
    assert!(!text.contains("IMAGE["), "{output}");
    assert!(
        output.lines().any(|line| line.contains("<https://example.com/docs>")),
        "a printed URL stays whole:\n{output}"
    );
}

macro_rules! plain_rows {
    ($($name:ident: $path:expr, $width:expr;)*) => {
        $(
            #[test]
            fn $name() {
                let source = corpus();
                let output = render_markdown(&source, $path, $width, 0);
                assert_plain_rules(&output);
                assert_link_rule_default(&output);
                let verbose = render_markdown(&source, $path, $width, 1);
                assert_plain_rules(&verbose);
                assert_link_rule_verbose(&verbose);
            }
        )*
    };
}

plain_rows! {
    as_terminal_at_40: Path::AsTerminal, Width::Columns(40);
    as_terminal_at_80: Path::AsTerminal, Width::Columns(80);
    as_terminal_unlimited: Path::AsTerminal, Width::Unlimited;
    page_at_40: Path::Page, Width::Columns(40);
    page_at_80: Path::Page, Width::Columns(80);
    page_unlimited: Path::Page, Width::Unlimited;
    framed_page_at_40: Path::FramedPage, Width::Columns(40);
    framed_page_at_80: Path::FramedPage, Width::Columns(80);
    framed_page_unlimited: Path::FramedPage, Width::Unlimited;
    max_width_page_at_40: Path::MaxWidthPage, Width::Columns(40);
    max_width_page_at_80: Path::MaxWidthPage, Width::Columns(80);
    max_width_page_unlimited: Path::MaxWidthPage, Width::Unlimited;
    framed_max_width_page_at_40: Path::FramedMaxWidthPage, Width::Columns(40);
    framed_max_width_page_at_80: Path::FramedMaxWidthPage, Width::Columns(80);
    framed_max_width_page_unlimited: Path::FramedMaxWidthPage, Width::Unlimited;
}

macro_rules! unlimited_rows {
    ($($name:ident: $path:expr;)*) => {
        $(
            #[test]
            fn $name() {
                let source = corpus();
                let output = render_markdown(&source, $path, Width::Unlimited, 0);
                assert!(output.len() < source.len() * 2, "{} bytes:\n{output}", output.len());
                assert!(output.contains(LONG_PARAGRAPH), "nothing wraps:\n{output}");
                assert!(collapsed(&output).contains("A soft-broken paragraph that continues"), "{output}");
                assert!(
                    output.lines().any(|line| line.ends_with("and ends here.")
                        && line.contains("A soft-broken paragraph")),
                    "soft breaks are joined:\n{output}"
                );
            }
        )*
    };
}

unlimited_rows! {
    unlimited_as_terminal_is_natural_size: Path::AsTerminal;
    unlimited_page_is_natural_size: Path::Page;
    unlimited_framed_page_is_natural_size: Path::FramedPage;
}

macro_rules! prose_width_rows {
    ($($name:ident: $path:expr, $width:expr, $limit:expr;)*) => {
        $(
            #[test]
            fn $name() {
                let output = render_markdown(LONG_PARAGRAPH, $path, $width, 0);
                assert!(output.lines().count() > 1, "it wraps:\n{output}");
                for line in output.lines() {
                    assert!(line.chars().count() <= $limit, "over {}: {line:?}", $limit);
                }
            }
        )*
    };
}

prose_width_rows! {
    prose_width_as_terminal_40: Path::AsTerminal, Width::Columns(40), 40;
    prose_width_page_40: Path::Page, Width::Columns(40), 40;
    prose_width_page_80: Path::Page, Width::Columns(80), 80;
    prose_width_framed_page_40: Path::FramedPage, Width::Columns(40), 40;
    prose_width_max_width_page_40: Path::MaxWidthPage, Width::Columns(40), MAX_WIDTH as usize;
    prose_width_max_width_caps_unlimited: Path::MaxWidthPage, Width::Unlimited, MAX_WIDTH as usize;
}

#[test]
fn a_page_built_at_40_columns_renders_at_40_not_the_ambient_width() {
    let terminal = Terminal::builder().width(40).build();
    let output = DarkmatterPage::new(&terminal).render(&Markdown::from(LONG_PARAGRAPH)).expect("render");
    let stripped = biscuit_terminal::prelude::strip_escape_codes(&output);
    assert!(stripped.lines().count() > 1, "{stripped}");
    assert!(stripped.lines().all(|line| line.chars().count() <= 40), "{stripped}");
}

#[test]
fn a_plain_frame_pads_nothing_on_the_right_or_on_blank_rows() {
    let output = DarkmatterPage::new(&Terminal::new_optimistic(40))
        .with_margin(2)
        .with_padding(1)
        .with_page_background(PageBackground::Subtle)
        .with_output_options(&plain_options(Width::Columns(40), 0))
        .render(&Markdown::from("One paragraph.\n\nAnother paragraph."))
        .expect("render");
    assert_eq!(
        output,
        "\n\n\n   One paragraph.\n\n   Another paragraph.\n\n\n\n"
    );
}

#[test]
fn a_colorless_frame_without_a_background_adds_no_trailing_whitespace() {
    let output = DarkmatterPage::new(&Terminal::new_optimistic(40))
        .with_margin_x(2)
        .with_padding_x(1)
        .with_color_depth(ColorDepth::None)
        .render(&Markdown::from("One paragraph.\n\nAnother paragraph."))
        .expect("render");
    for line in output.lines() {
        assert_eq!(line, line.trim_end(), "{output:?}");
    }
    assert!(output.contains("\n\n"), "a blank row stays empty: {output:?}");
}

#[test]
fn an_unlimited_background_frame_keeps_the_left_side_and_fills_nothing() {
    let output = DarkmatterPage::new(&Terminal::new_optimistic(40))
        .with_margin_x(2)
        .with_padding_x(1)
        .with_page_background(PageBackground::Subtle)
        .with_output_options(&OutputOptions::new(Width::Unlimited, rich_profile()))
        .render(&Markdown::from("Short."))
        .expect("render");
    let line = output.lines().next().expect("a row");
    let stripped = biscuit_terminal::prelude::strip_escape_codes(line);
    assert_eq!(stripped, "   Short. ", "left margin and padding, right padding, no fill: {line:?}");
}

/// A rich profile with no pixels, so the render reads nothing from the host.
fn rich_profile() -> TerminalProfile {
    TerminalProfile {
        color_depth: ProfileColorDepth::TrueColor,
        color_mode: ProfileColorMode::Dark,
        text_color: None,
        background_color: None,
        is_tty: true,
        hyperlinks: true,
        italics: true,
        underline: true,
        graphics: false,
        unicode: true,
    }
}

const LINK: &str = "[the docs](https://example.com/docs)";
const OSC8: &str = "\u{1b}]8;";

/// A width above what a `u16` holds reaches the fold unclamped.
#[test]
fn a_very_wide_page_is_not_clamped() {
    let output = DarkmatterPage::new(&Terminal::new_optimistic(80))
        .with_output_options(&OutputOptions::new(Width::Columns(70_000), rich_profile()))
        .render(&Markdown::from("---"))
        .expect("render");
    let stripped = biscuit_terminal::prelude::strip_escape_codes(output);
    assert_eq!(stripped.trim().chars().count(), 70_000);
}

#[test]
fn hyperlink_mode_never_disables_osc8_on_as_terminal() {
    let render = |mode| {
        let mut options = TerminalOptions::default();
        options.width = Some(Width::Columns(80));
        options.color_depth = Some(ColorDepth::TrueColor);
        options.hyperlink_mode = mode;
        Markdown::from(LINK).as_terminal(options).expect("render")
    };
    assert!(!render(HyperlinkMode::Never).contains(OSC8));
    assert!(render(HyperlinkMode::Always).contains(OSC8));
}

#[test]
fn hyperlink_mode_never_disables_osc8_on_a_framed_page() {
    let render = |mode| {
        DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_margin_x(2)
            .with_hyperlink_mode(mode)
            .render(&Markdown::from(LINK))
            .expect("render")
    };
    assert!(!render(HyperlinkMode::Never).contains(OSC8));
    assert!(render(HyperlinkMode::Auto).contains(OSC8));
}

#[test]
fn no_hyperlink_mode_restores_osc8_under_a_plain_profile() {
    let options = plain_options(Width::Columns(80), 0);
    let mut terminal_options = TerminalOptions::default().with_output_options(&options);
    terminal_options.hyperlink_mode = HyperlinkMode::Always;
    let direct = Markdown::from(LINK).as_terminal(terminal_options).expect("render");
    assert!(!direct.contains('\u{1b}'), "{direct:?}");

    for page in [
        DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_hyperlink_mode(HyperlinkMode::Always)
            .with_output_options(&options),
        DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_output_options(&options)
            .with_hyperlink_mode(HyperlinkMode::Always)
            .with_color_depth(ColorDepth::TrueColor)
            .with_margin_x(1),
        DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_output_options(&options)
            .with_terminal_options(TerminalOptions::default()),
    ] {
        let output = page.render(&Markdown::from(LINK)).expect("render");
        assert!(!output.contains('\u{1b}'), "{output:?}");
    }
}

/// A width selects no capability: on `as_terminal` OSC 8 follows detection
/// alone, whatever the width.
#[test]
fn a_width_on_as_terminal_never_emits_osc8_the_terminal_does_not_support() {
    let detected = Terminal::default();
    let mut options = TerminalOptions::default();
    options.width = Some(Width::Columns(40));
    let output = Markdown::from(LINK).as_terminal(options).expect("render");
    assert_eq!(output.contains(OSC8), detected.osc_link_support, "{output:?}");
}

#[test]
fn a_framed_page_styles_with_its_own_terminal_not_an_optimistic_one() {
    let mut piped = Terminal::new_optimistic(80);
    piped.is_tty = false;
    piped.osc_link_support = false;
    piped.color_depth = biscuit_terminal::discovery::detection::ColorDepth::None;
    let output = DarkmatterPage::new(&piped)
        .with_margin_x(2)
        .render(&Markdown::from(LINK))
        .expect("render");
    assert!(!output.contains(OSC8), "{output:?}");
}

#[test]
fn an_explicit_profile_uses_exactly_that_profile() {
    let mut no_links = rich_profile();
    no_links.hyperlinks = false;
    for page in [
        DarkmatterPage::new(&Terminal::new_optimistic(80)),
        DarkmatterPage::new(&Terminal::new_optimistic(80)).with_margin_x(1),
    ] {
        let rich = page
            .clone()
            .with_output_options(&OutputOptions::new(Width::Columns(80), rich_profile()))
            .render(&Markdown::from(format!("{LINK} and `code`")))
            .expect("render");
        assert!(rich.contains(OSC8) && rich.contains("\u{1b}[48;2;"), "{rich:?}");

        let colored = page
            .with_output_options(&OutputOptions::new(Width::Columns(80), no_links.clone()))
            .render(&Markdown::from(format!("{LINK} and `code`")))
            .expect("render");
        assert!(!colored.contains(OSC8), "{colored:?}");
        assert!(colored.contains("\u{1b}[48;2;"), "{colored:?}");
    }
}

/// The non-clickable rule also applies to a color terminal without OSC 8.
#[test]
fn a_color_terminal_without_osc8_follows_the_link_rule() {
    let mut profile = rich_profile();
    profile.hyperlinks = false;
    let render = |verbosity| {
        DarkmatterPage::new(&Terminal::new_optimistic(80))
            .with_output_options(&OutputOptions::new(Width::Columns(80), profile.clone()).with_verbosity(verbosity))
            .render(&Markdown::from(LINK))
            .expect("render")
    };
    let quiet = biscuit_terminal::prelude::strip_escape_codes(render(0));
    let verbose = biscuit_terminal::prelude::strip_escape_codes(render(1));
    assert_eq!(quiet.trim(), "the docs");
    assert_eq!(verbose.trim(), "the docs <https://example.com/docs>");
}

#[test]
fn a_long_printed_url_stays_whole_on_its_own_line_at_40() {
    const URL: &str = "https://example.com/a/very/long/path/to/the/documentation/page.html";
    let source = format!("See [the docs]({URL}) for details.");
    for path in [Path::AsTerminal, Path::Page, Path::FramedPage] {
        let output = render_markdown(&source, path, Width::Columns(40), 1);
        let bracketed = format!("<{URL}>");
        assert!(
            output.lines().any(|line| line.trim() == bracketed),
            "{path:?}: the URL on a line of its own:\n{output}"
        );
    }
}

fn plain_equivalent_profiles() -> [TerminalProfile; 2] {
    let mut no_color = rich_profile();
    no_color.color_depth = ProfileColorDepth::None;
    let mut piped = rich_profile();
    piped.is_tty = false;
    [no_color, piped]
}

#[test]
fn a_degraded_terminal_request_equals_plain_bytes_through_render_output() {
    let document = Markdown::from(corpus());
    for frame in [Path::Page, Path::FramedPage] {
        let page = page(frame);
        for width in [Width::Columns(40), Width::Unlimited] {
            let plain_request = plain_options(width, 1);
            let plain = DocumentOutput::prepare(&document, &page, OutputTarget::Plain, &plain_request).expect("prepare");
            let plain = render_output(&plain, OutputTarget::Plain, &plain_request).expect("plain").text;

            for profile in plain_equivalent_profiles() {
                let request = OutputOptions::new(width, profile).with_verbosity(1);
                let terminal = DocumentOutput::prepare(&document, &page, OutputTarget::Terminal, &request).expect("prepare");
                let terminal = render_output(&terminal, OutputTarget::Terminal, &request).expect("terminal").text;
                assert_eq!(terminal, plain, "{frame:?} at {width:?}");
            }
        }
    }
}

fn snapshot_page() -> DarkmatterPage {
    DarkmatterPage::new(&Terminal::new_optimistic(80))
        .with_prose_theme("one-half")
        .with_code_theme("one-half")
}

#[test]
fn snapshot_of_the_plain_corpus() {
    let output = snapshot_page()
        .with_output_options(&plain_options(Width::Columns(60), 1))
        .render(&Markdown::from(corpus()))
        .expect("render");
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_of_the_styled_corpus() {
    let output = snapshot_page()
        .with_output_options(&OutputOptions::new(Width::Columns(60), rich_profile()))
        .render(&Markdown::from(corpus()))
        .expect("render");
    insta::assert_snapshot!(output);
}

const FIXED_WIDTH_DOCUMENT: &str = "---\nstyle:\n  markdown:\n    fixed-width: 20\n    unwrap: true\n---\n\
    The first soft-broken line\nof one paragraph that is longer than twenty columns.\n";

fn compose_body(width: Option<Width>) -> String {
    let directory = tempfile::tempdir().expect("temp dir");
    let mut options = ComposeOptions::new().only(&[ComposeOperation::Cleanup]);
    if let Some(width) = width {
        options = options.with_output_width(width);
    }
    let request = ComposeRequest::prepare(options, &RequestSnapshot::new(directory.path())).expect("request");
    let (composed, _) = Markdown::from(FIXED_WIDTH_DOCUMENT).compose_with(&request).expect("compose");
    composed.content().to_string()
}

#[test]
fn output_width_columns_wins_over_fixed_width_frontmatter() {
    let body = compose_body(Some(Width::Columns(30)));
    assert!(body.lines().count() > 1, "{body}");
    assert!(body.lines().all(|line| line.chars().count() <= 30), "{body}");
    assert!(body.lines().any(|line| line.chars().count() > 20), "not the document's 20: {body}");
}

#[test]
fn output_width_unlimited_unwraps_with_no_fixed_width() {
    let body = compose_body(Some(Width::Unlimited));
    assert_eq!(body.trim_end().lines().count(), 1, "{body}");
}

#[test]
fn output_width_source_keeps_authored_line_breaks() {
    let body = compose_body(Some(Width::Source));
    assert!(body.starts_with("The first soft-broken line\nof one paragraph"), "{body}");
}

#[test]
fn output_width_outranks_with_fixed_width() {
    let directory = tempfile::tempdir().expect("temp dir");
    let options = ComposeOptions::new()
        .only(&[ComposeOperation::Cleanup])
        .with_fixed_width(20)
        .with_output_width(Width::Unlimited);
    let request = ComposeRequest::prepare(options, &RequestSnapshot::new(directory.path())).expect("request");
    let (composed, _) = Markdown::from(FIXED_WIDTH_DOCUMENT).compose_with(&request).expect("compose");
    assert_eq!(composed.content().trim_end().lines().count(), 1);
}

#[test]
fn compose_without_an_output_width_is_unchanged() {
    let body = compose_body(None);
    let directory = tempfile::tempdir().expect("temp dir");
    let request = ComposeRequest::prepare(
        ComposeOptions::new().only(&[ComposeOperation::Cleanup]),
        &RequestSnapshot::new(directory.path()),
    )
    .expect("request");
    let (composed, _) = Markdown::from(FIXED_WIDTH_DOCUMENT).compose_with(&request).expect("compose");
    assert_eq!(body, composed.content());
}
