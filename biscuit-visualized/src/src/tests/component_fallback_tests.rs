//! The terminal component's fallback: the policy and `MERMAID_FALLBACK`
//! (D3, AC-F5, AC-F6), what the component chooses on a terminal without
//! images (AC-F1, AC-F4), the width a drawing gets, and the work each route
//! does (AC-F13).
//!
//! Boundary: the component ↔ this crate's text backend and biscuit-terminal's
//! render-tree layout, in process (F2, L1, explicit `Terminal` capabilities).
//! These live in the crate's own tests because the work counters are
//! crate-internal.

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::terminal_image::ImageWidth;
use biscuit_terminal::discovery::detection::{ColorDepth, ImageSupport};
use biscuit_terminal::terminal::{Terminal, TerminalBuilder};
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use renderable::layout::{Edges, Length, TargetValue};
use renderable::tree::{NodeKind, RenderNode};
use serial_test::serial;

use crate::cache::file_cache::take_stored_files;
use crate::components::{
    MERMAID_FALLBACK_ENV, MermaidDiagram, MermaidFallbackPolicy, MermaidPresentation, PARTIAL_NOTE_ID,
    ResolvedFallbackPolicy, TextFallback,
};
use crate::fonts::chain::take_default_chain_requests;
use crate::mermaid::render::take_text_requests;
use crate::tests::isolated_cache_dir;

/// The spec's flowchart; canonically 50 cells wide.
const SPEC_FLOWCHART: &str = "flowchart LR
    A[Write spec] --> B{Approved?}
    B -- yes --> C[Plan]
    B -- no --> A
    click C \"https://example.com\"";

/// Canonically 66 cells wide, compactly 58.
const CHAIN_FLOWCHART: &str = "flowchart LR
    A[Alpha] --> B[Beta] --> C[Gamma] --> D[Delta] --> E[Epsilon]";

const MINDMAP: &str = "mindmap\n  root((Merman))\n    Rendering\n    Text";
const PIE: &str = "pie\n    \"Dogs\" : 50\n    \"Cats\" : 50";

const ALLOWED: ResolvedFallbackPolicy =
    ResolvedFallbackPolicy { text_fallback: TextFallback::Allowed, show_partial_note: true };

/// The exact plain output for [`SPEC_FLOWCHART`] on an 80-column terminal
/// without images: the drawing (rows keep their trailing spaces), a blank
/// line, and the note wrapped to the width.
const SPEC_FLOWCHART_PLAIN: &[&str] = &[
    "```txt",
    "┌────────────┐       ╭───────────╮        ┌──────┐",
    "│            │       ╱           ╲        │      │",
    "│ Write spec ├──────►< Approved? >──yes──►│ Plan │",
    "│            │       ╲           ╱        │      │",
    "└────────────┘       ╰───────────╯        └──────┘",
    "       ▲                   │                      ",
    "       └────────no─────────┘                      ",
    "```",
    "",
    "│ Note: this textual representation is provided because the current terminal",
    "│ settings do not allow for a graphical image to be used. This fallback",
    "│ representation, while never arbitrary or speculative, might be missing",
    "│ attributes that would have been visible in the image's representation [id:",
    "│ mermaid-partial-warning].",
];

/// A terminal whose graphics route has no pixels, with no color and Unicode
/// glyphs. Every capability an assertion reads is pinned, so the host's
/// `NO_COLOR`, `FORCE_COLOR`, `COLORTERM`, `TERM`, and locale change nothing.
fn no_images(width: u32) -> TerminalBuilder {
    Terminal::builder()
        .width(width)
        .image_support(ImageSupport::None)
        .is_tty(false)
        .color_depth(ColorDepth::None)
        .supports_italic(false)
        .supports_unicode(true)
}

/// [`no_images`] without color or styling.
fn plain_no_images(width: u32) -> Terminal {
    no_images(width).build()
}

/// [`no_images`] that can style text: the render tree emits bold, dim, and
/// italic only above [`ColorDepth::None`].
fn styled_no_images(width: u32) -> Terminal {
    no_images(width).color_depth(ColorDepth::TrueColor).supports_italic(true).build()
}

/// The language of each code node and the kind of every other node.
fn shape(nodes: &[RenderNode]) -> Vec<String> {
    nodes
        .iter()
        .map(|node| match &node.kind {
            NodeKind::Code { lang, .. } => format!("code:{}", lang.as_deref().unwrap_or("")),
            NodeKind::BlockQuote { .. } => "quote".to_string(),
            NodeKind::Paragraph { .. } => "paragraph".to_string(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn presented(diagram: &MermaidDiagram, term: &Terminal, policy: ResolvedFallbackPolicy) -> (&'static str, Vec<RenderNode>) {
    match diagram.present_with(term, policy) {
        MermaidPresentation::Image(_) => ("image", Vec::new()),
        MermaidPresentation::Text(nodes) => ("text", nodes),
        MermaidPresentation::Source(nodes) => ("source", nodes),
    }
}

/// The value of the one code node in `nodes`.
fn code_value(nodes: &[RenderNode]) -> &str {
    nodes
        .iter()
        .find_map(|node| match &node.kind {
            NodeKind::Code { value, .. } => Some(value.as_str()),
            _ => None,
        })
        .expect("a code node")
}

fn reset_counters() {
    take_text_requests();
    take_default_chain_requests();
    take_stored_files();
}

// ── Policy resolution (D3, AC-F5) ────────────────────────────────────────

#[test]
fn an_explicit_text_fallback_never_reads_the_environment() {
    for explicit in [TextFallback::Allowed, TextFallback::NoPartial, TextFallback::Never] {
        let policy = MermaidFallbackPolicy::default().with_text_fallback(explicit);
        let resolved = policy.resolve_with(|name| panic!("{name} read for an explicit setting"));
        assert_eq!(resolved.text_fallback, explicit);
    }
}

#[test]
fn an_unset_text_fallback_reads_mermaid_fallback() {
    let resolve = |value: Option<&str>| {
        MermaidFallbackPolicy::default()
            .resolve_with(|name| {
                assert_eq!(name, MERMAID_FALLBACK_ENV);
                value.map(str::to_string)
            })
            .text_fallback
    };
    assert_eq!(MERMAID_FALLBACK_ENV, "MERMAID_FALLBACK");
    assert_eq!(resolve(None), TextFallback::Allowed, "unset");
    assert_eq!(resolve(Some("")), TextFallback::Allowed, "empty");
    assert_eq!(resolve(Some("image")), TextFallback::Allowed, "another value");
    assert_eq!(resolve(Some("code-block")), TextFallback::Never);
    assert_eq!(resolve(Some("Code-Block")), TextFallback::Never, "mixed case");
}

#[test]
fn mermaid_fallback_never_changes_the_partial_note() {
    let code_block = |_: &str| Some("code-block".to_string());
    assert!(MermaidFallbackPolicy::default().resolve_with(code_block).show_partial_note, "default shown");
    let hidden = MermaidFallbackPolicy::default().with_show_partial_note(false);
    assert!(!hidden.resolve_with(code_block).show_partial_note);
    let shown = MermaidFallbackPolicy::default().with_show_partial_note(true);
    assert!(shown.resolve_with(|_| None).show_partial_note);
}

#[cfg(feature = "clap")]
mod features {
    use super::*;
    use crate::components::MermaidFallbackFeature::{
        NoMermaidFallbacks, NoMermaidPartialFallbacks, NoMermaidPartialWarning,
    };

    #[test]
    fn no_feature_leaves_every_setting_to_the_environment() {
        let policy = MermaidFallbackPolicy::from_features([]);
        assert_eq!(policy, MermaidFallbackPolicy::default());
        let resolved = policy.resolve_with(|_| Some("CODE-BLOCK".to_string()));
        assert_eq!(resolved.text_fallback, TextFallback::Never);
    }

    #[test]
    fn each_feature_sets_only_its_own_setting() {
        let never = MermaidFallbackPolicy::from_features([NoMermaidFallbacks]);
        assert_eq!((never.text_fallback(), never.show_partial_note()), (Some(TextFallback::Never), None));
        let no_partial = MermaidFallbackPolicy::from_features([NoMermaidPartialFallbacks]);
        assert_eq!(
            (no_partial.text_fallback(), no_partial.show_partial_note()),
            (Some(TextFallback::NoPartial), None)
        );
        let no_note = MermaidFallbackPolicy::from_features([NoMermaidPartialWarning]);
        assert_eq!((no_note.text_fallback(), no_note.show_partial_note()), (None, Some(false)));
    }

    #[test]
    fn overlapping_features_take_the_more_restrictive_in_either_order() {
        for features in [
            [NoMermaidFallbacks, NoMermaidPartialFallbacks],
            [NoMermaidPartialFallbacks, NoMermaidFallbacks],
        ] {
            let policy = MermaidFallbackPolicy::from_features(features);
            assert_eq!(policy.text_fallback(), Some(TextFallback::Never), "{features:?}");
        }
        let repeated = MermaidFallbackPolicy::from_features([NoMermaidPartialWarning, NoMermaidPartialWarning]);
        assert_eq!(repeated, MermaidFallbackPolicy::from_features([NoMermaidPartialWarning]));
    }

    /// A flag is an explicit setting, so it wins even where the variable
    /// asks for more.
    #[test]
    fn a_flag_beats_the_variable() {
        let policy = MermaidFallbackPolicy::from_features([NoMermaidPartialFallbacks]);
        let resolved = policy.resolve_with(|_| Some("code-block".to_string()));
        assert_eq!(resolved.text_fallback, TextFallback::NoPartial);
    }
}

// ── Selection on a terminal without images (AC-F1, AC-F4) ───────────────

#[test]
fn a_flowchart_is_drawn_as_text_and_mindmap_and_pie_show_source() {
    let term = plain_no_images(80);

    let (route, nodes) = presented(&MermaidDiagram::new(SPEC_FLOWCHART), &term, ALLOWED);
    assert_eq!(route, "text");
    assert_eq!(shape(&nodes), ["code:txt", "quote"]);
    assert!(code_value(&nodes).contains("│ Write spec ├"), "{}", code_value(&nodes));

    for source in [MINDMAP, PIE] {
        let (route, nodes) = presented(&MermaidDiagram::new(source), &term, ALLOWED);
        assert_eq!(route, "source", "{source}");
        assert_eq!(shape(&nodes), ["code:mermaid"], "no note with the source: {source}");
        assert_eq!(code_value(&nodes), source, "the source, unmodified");
    }
}

#[test]
fn the_text_fallback_renders_exactly_with_its_note() {
    let diagram = MermaidDiagram::new(SPEC_FLOWCHART)
        .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed));

    // Without color the output is exactly the plain text, with no SGR at all.
    assert_eq!(diagram.render(&plain_no_images(80)), SPEC_FLOWCHART_PLAIN.join("\n"));

    let output = diagram.render(&styled_no_images(80));
    assert_eq!(strip_escape_codes(output.as_str()), SPEC_FLOWCHART_PLAIN.join("\n"));
    // `Note:` bold, the bracketed id dim and the id italic (here the
    // wrap falls between `id:` and the id).
    assert!(output.contains("\x1b[1mNote:\x1b[0m"), "{output:?}");
    assert!(output.contains("[\x1b[2mid:\n"), "{output:?}");
    assert!(output.contains(&format!("\x1b[3m{PARTIAL_NOTE_ID}")), "{output:?}");
    // Unwrapped, the italic id sits inside the dim run.
    let wide = diagram.render(&styled_no_images(200));
    assert!(wide.contains(&format!("[\x1b[2mid: \x1b[3m{PARTIAL_NOTE_ID}\x1b[0m")), "{wide:?}");
}

#[test]
fn a_hidden_note_leaves_only_the_drawing() {
    let policy = ResolvedFallbackPolicy { show_partial_note: false, ..ALLOWED };
    let (route, nodes) = presented(&MermaidDiagram::new(SPEC_FLOWCHART), &plain_no_images(80), policy);
    assert_eq!(route, "text");
    assert_eq!(shape(&nodes), ["code:txt"]);

    let diagram = MermaidDiagram::new(SPEC_FLOWCHART).with_fallback_policy(
        MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed).with_show_partial_note(false),
    );
    let plain = strip_escape_codes(diagram.render(&plain_no_images(80)));
    assert_eq!(plain, SPEC_FLOWCHART_PLAIN[..9].join("\n"));
}

#[test]
fn a_terminal_without_unicode_gets_an_ascii_drawing() {
    let mut term = plain_no_images(80);
    term.supports_unicode = false;
    let (route, nodes) = presented(&MermaidDiagram::new(SPEC_FLOWCHART), &term, ALLOWED);
    assert_eq!(route, "text");
    let drawing = code_value(&nodes);
    assert!(drawing.contains("| Write spec |"), "{drawing}");
    assert!(!drawing.contains('│') && !drawing.contains('┌'), "{drawing}");
}

/// AC-F6: "no partial text" shows the source for every eligible family at
/// the pinned rev (all partial), without asking Merman for text.
#[test]
fn no_partial_text_shows_the_source_for_every_eligible_family() {
    let families = [
        ("flowchart", include_str!("fixtures/merman/families/flowchart.mmd")),
        ("sequence", include_str!("fixtures/merman/families/sequence.mmd")),
        ("state", include_str!("fixtures/merman/families/state.mmd")),
        ("class", include_str!("fixtures/merman/families/class.mmd")),
        ("er", include_str!("fixtures/merman/families/er.mmd")),
        ("xychart", include_str!("fixtures/merman/families/xychart.mmd")),
        ("swimlane", include_str!("fixtures/merman/families/swimlane.mmd")),
    ];
    let no_partial = ResolvedFallbackPolicy { text_fallback: TextFallback::NoPartial, ..ALLOWED };
    let term = plain_no_images(400);
    for (family, source) in families {
        let diagram = MermaidDiagram::new(source);
        let eligibility = crate::mermaid::MermaidDiagram::new(source).text_eligibility();
        assert_eq!(eligibility.map(|e| e.family()), Some(family));
        take_text_requests();
        let (route, nodes) = presented(&diagram, &term, no_partial);
        assert_eq!((route, shape(&nodes)), ("source", vec!["code:mermaid".to_string()]), "{family}");
        assert_eq!(take_text_requests(), 0, "{family}");
    }
    // Control: the flowchart fixture is drawn when partial text is allowed.
    let (route, _) = presented(&MermaidDiagram::new(families[0].1), &term, ALLOWED);
    assert_eq!(route, "text");
}

#[test]
fn never_text_shows_the_source_without_a_request() {
    let diagram = MermaidDiagram::new(SPEC_FLOWCHART)
        .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Never));
    assert_eq!(diagram.fallback_policy().text_fallback(), Some(TextFallback::Never));
    take_text_requests();
    assert!(matches!(diagram.present(&plain_no_images(80)), MermaidPresentation::Source(_)));
    assert_eq!(take_text_requests(), 0);
}

#[test]
fn a_text_failure_shows_the_source() {
    let (route, nodes) = presented(&MermaidDiagram::new("flowchart LR\n    A[unclosed --> B"), &plain_no_images(80), ALLOWED);
    assert_eq!((route, shape(&nodes)), ("source", vec!["code:mermaid".to_string()]));
}

// ── The fallible presentation (`try_present`) ────────────────────────────

const BROKEN_FLOWCHART: &str = "flowchart LR\n    A[unclosed --> B";

fn with_text(source: &str, text_fallback: TextFallback) -> MermaidDiagram {
    MermaidDiagram::new(source).with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(text_fallback))
}

/// The error `present` hides comes from the one text request.
#[test]
#[serial]
fn try_present_reports_a_text_failure_after_one_request() {
    let _cache = isolated_cache_dir();
    reset_counters();

    let result = with_text(BROKEN_FLOWCHART, TextFallback::Allowed).try_present(&plain_no_images(80));

    assert!(result.is_err(), "{result:?}");
    assert_eq!(take_text_requests(), 1);
    assert_eq!(take_default_chain_requests(), 0, "no font chain");
    assert_eq!(take_stored_files(), 0, "no artifact");
}

/// No text request is made, so the source is parsed instead: still no
/// fonts and no artifact.
#[test]
#[serial]
fn try_present_parses_a_source_it_never_requests_as_text() {
    let _cache = isolated_cache_dir();
    reset_counters();

    let never = with_text(BROKEN_FLOWCHART, TextFallback::Never).try_present(&plain_no_images(80));
    let broken_pie = with_text("pie\n    \"Dogs\" 50", TextFallback::Allowed).try_present(&plain_no_images(80));

    assert!(never.is_err(), "{never:?}");
    assert!(broken_pie.is_err(), "{broken_pie:?}");
    assert_eq!(take_text_requests(), 0);
    assert_eq!(take_default_chain_requests(), 0, "no font chain");
    assert_eq!(take_stored_files(), 0, "no artifact");
}

/// A valid flowchart under frontmatter Merman cannot preprocess: `try_present`
/// reports it, under every text policy, with no text request; the
/// infallible `present` shows the source.
#[test]
#[serial]
fn malformed_frontmatter_is_an_error_for_try_present_and_source_for_present() {
    let _cache = isolated_cache_dir();
    let malformed = "---\ntitle: [broken\n---\nflowchart LR\n A --> B";
    reset_counters();

    for text_fallback in [TextFallback::Allowed, TextFallback::Never] {
        let result = with_text(malformed, text_fallback).try_present(&plain_no_images(80));
        assert!(result.is_err(), "{text_fallback:?}: {result:?}");
    }
    let (route, nodes) = presented(&MermaidDiagram::new(malformed), &plain_no_images(80), ALLOWED);

    assert_eq!((route, code_value(&nodes)), ("source", malformed));
    assert_eq!(take_text_requests(), 0);
    assert_eq!(take_default_chain_requests(), 0, "no font chain");
    assert_eq!(take_stored_files(), 0, "no artifact");
}

/// A header Merman detects no diagram type in: `try_present` reports it,
/// under every text policy, with no text request; the infallible `present`
/// shows the source, as it does for ZenUML, a family with no text output.
#[test]
#[serial]
fn an_undetectable_diagram_type_is_an_error_for_try_present_and_source_for_present() {
    let _cache = isolated_cache_dir();
    let undetectable = "flowchar LR\n A --> B";
    let zenuml = "zenuml\n    A.b()";
    reset_counters();

    for text_fallback in [TextFallback::Allowed, TextFallback::Never] {
        let result = with_text(undetectable, text_fallback).try_present(&plain_no_images(80));
        assert!(result.is_err(), "{text_fallback:?}: {result:?}");
    }
    let (route, nodes) = presented(&MermaidDiagram::new(undetectable), &plain_no_images(80), ALLOWED);
    assert_eq!((route, code_value(&nodes)), ("source", undetectable));
    assert_eq!(take_text_requests(), 0);

    let (route, nodes) = presented(&MermaidDiagram::new(zenuml), &plain_no_images(80), ALLOWED);
    assert_eq!((route, code_value(&nodes)), ("source", zenuml));

    assert_eq!(take_text_requests(), 0);
    assert_eq!(take_default_chain_requests(), 0, "no font chain");
    assert_eq!(take_stored_files(), 0, "no artifact");
}

/// Parses, but the image route refuses it: Merman's math support is not
/// built in.
const MATH_FLOWCHART: &str = "flowchart LR\n    A[\"$$x^2$$\"] --> B";

/// Wherever `try_present` shows the source in place of a drawing (no text
/// request under the policy, or a drawing too wide), a math label is the
/// image route's missing-capability error, still with no fonts and no
/// artifact.
#[test]
#[serial]
fn try_present_reports_a_math_label_wherever_it_shows_the_source() {
    let _cache = isolated_cache_dir();
    let narrow = with_text(MATH_FLOWCHART, TextFallback::Allowed).with_width(ImageWidth::Characters(8));
    reset_counters();

    let cases = [
        ("never", with_text(MATH_FLOWCHART, TextFallback::Never)),
        ("no-partial", with_text(MATH_FLOWCHART, TextFallback::NoPartial)),
        ("too narrow", narrow),
    ];

    for (case, diagram) in cases {
        let reason = match diagram.try_present(&plain_no_images(80)) {
            Err(error) => error.to_string(),
            other => panic!("{case}: {other:?}"),
        };
        assert!(reason.contains("lacks capability `math`"), "{case}: {reason}");
    }
    assert_eq!(take_text_requests(), 1, "only the narrow case requests text");
    assert_eq!(take_default_chain_requests(), 0, "no font chain");
    assert_eq!(take_stored_files(), 0, "no artifact");
}

/// Merman draws a math label as its literal text, so a drawing that fits is
/// shown as partial text, not an error.
#[test]
fn try_present_draws_a_math_label_that_fits_as_partial_text() {
    let result = with_text(MATH_FLOWCHART, TextFallback::Allowed).try_present(&plain_no_images(80));

    let Ok(MermaidPresentation::Text(nodes)) = result else {
        panic!("{result:?}");
    };
    assert!(code_value(&nodes).contains("$$x^2$$"), "{}", code_value(&nodes));
    assert_eq!(shape(&nodes), ["code:txt", "quote"], "the partial note follows the drawing");
}

/// Controls: a valid source is `Ok` whether it is drawn, refused for its
/// family or policy, or too wide.
#[test]
fn try_present_succeeds_wherever_the_source_parses() {
    let term = plain_no_images(80);
    let narrow = with_text(CHAIN_FLOWCHART, TextFallback::Allowed).with_width(ImageWidth::Characters(59));

    let cases = [
        (with_text(SPEC_FLOWCHART, TextFallback::Allowed), "text"),
        (with_text(SPEC_FLOWCHART, TextFallback::Never), "source"),
        (with_text(PIE, TextFallback::Allowed), "source"),
        (narrow, "source"),
    ];

    for (diagram, expected) in cases {
        let route = match diagram.try_present(&term) {
            Ok(MermaidPresentation::Text(_)) => "text",
            Ok(MermaidPresentation::Source(_)) => "source",
            other => panic!("{}: {other:?}", diagram.instructions()),
        };
        assert_eq!(route, expected, "{}", diagram.instructions());
    }
}

/// Merman draws an XY chart as text only when its y-axis states a range, so
/// `bt bar-chart 1 2 3`'s source (no y-axis) shows as source. A Merman bump
/// that lifts this changes the documented `bt bar-chart` output.
#[test]
fn an_xychart_without_a_y_range_shows_the_source() {
    let bt_bar_chart = "%%{init: {\"xychart\": {\"showTitle\": false, \"xAxis\": {\"showLabel\": false}, \"yAxis\": {\"showLabel\": false}}}}%%\nxychart-beta\n    x-axis [1, 2, 3]\n    bar [1, 2, 3]";
    let (route, nodes) = presented(&MermaidDiagram::new(bt_bar_chart), &plain_no_images(80), ALLOWED);
    assert_eq!((route, shape(&nodes)), ("source", vec!["code:mermaid".to_string()]));

    let with_range = bt_bar_chart.replace("    bar", "    y-axis 0 --> 5\n    bar");
    let (route, nodes) = presented(&MermaidDiagram::new(&with_range), &plain_no_images(80), ALLOWED);
    assert_eq!((route, shape(&nodes)), ("text", vec!["code:txt".to_string(), "quote".to_string()]));
    assert!(code_value(&nodes).contains("5 ┤"), "{}", code_value(&nodes));
}

// ── Title (Text presentation) ────────────────────────────────────────────

#[test]
fn an_explicit_title_leads_text_and_source_once() {
    let term = plain_no_images(80);
    let (_, text) = presented(&MermaidDiagram::new(SPEC_FLOWCHART).with_title("Review flow"), &term, ALLOWED);
    assert_eq!(shape(&text), ["paragraph", "code:txt", "quote"]);
    let (_, source) = presented(&MermaidDiagram::new(PIE).with_title("Pets"), &term, ALLOWED);
    assert_eq!(shape(&source), ["paragraph", "code:mermaid"]);
    let plain = strip_escape_codes(MermaidDiagram::new(PIE).with_title("Pets").render(&term));
    assert_eq!(plain.matches("Pets").count(), 1, "{plain}");

    // The source already gives the same title: no second one.
    let titled = "pie title Pets\n    \"Dogs\" : 50";
    let (_, source) = presented(&MermaidDiagram::new(titled).with_title("Pets"), &term, ALLOWED);
    assert_eq!(shape(&source), ["code:mermaid"]);
    let titled = "---\ntitle: \"Pets\"\n---\npie\n    \"Dogs\" : 50";
    let (_, source) = presented(&MermaidDiagram::new(titled).with_title("Pets"), &term, ALLOWED);
    assert_eq!(shape(&source), ["code:mermaid"]);
}

/// `source` titled `title` under `text_fallback` (the partial note shown):
/// the route `present` takes on `term`, and the plain rendered output.
fn titled_output(source: &str, title: &str, text_fallback: TextFallback, term: &Terminal) -> (&'static str, String) {
    let diagram = MermaidDiagram::new(source).with_title(title).with_fallback_policy(
        MermaidFallbackPolicy::default().with_text_fallback(text_fallback).with_show_partial_note(true),
    );
    let route = match diagram.present(term) {
        MermaidPresentation::Image(_) => "image",
        MermaidPresentation::Text(_) => "text",
        MermaidPresentation::Source(_) => "source",
    };
    (route, strip_escape_codes(diagram.render(term)))
}

const FRONTMATTER_FLOWCHART: &str = "---\ntitle: Release\n---\nflowchart LR\n A --> B";

/// R1-F3.1, R1-F3.2: a text drawing never draws a frontmatter title, so
/// the explicit title leads it once whether or not the source gives it.
#[test]
fn an_explicit_title_equal_to_a_frontmatter_title_is_shown_once_with_text() {
    let term = plain_no_images(80);
    for source in [FRONTMATTER_FLOWCHART, "flowchart LR\n A --> B"] {
        let (route, plain) = titled_output(source, "Release", TextFallback::Allowed, &term);
        assert_eq!(route, "text", "{source}");
        assert_eq!(plain.matches("Release").count(), 1, "{source}\n{plain}");
        assert!(plain.starts_with("Release\n"), "the title paragraph leads: {plain}");
    }
}

/// A drawing that draws the same title itself (a sequence diagram's
/// `title` statement on its first row, an XY chart's title disclosure row)
/// gets no second one; a different statement title keeps the explicit one.
#[test]
fn a_drawing_that_draws_the_title_gets_no_second_one() {
    let term = plain_no_images(80);
    let chart = "xychart-beta\n title \"Release\"\n x-axis [a, b, c]\n y-axis 0 --> 5\n bar [1, 2, 3]";
    for source in ["sequenceDiagram\n title Release\n Alice->>Bob: Hi", chart] {
        let (route, plain) = titled_output(source, "Release", TextFallback::Allowed, &term);
        assert_eq!(route, "text", "{source}");
        assert_eq!(plain.matches("Release").count(), 1, "{source}\n{plain}");
        assert!(plain.starts_with("```txt"), "no title paragraph: {plain}");
    }

    let other = "---\ntitle: Release\n---\nsequenceDiagram\n title Other\n Alice->>Bob: Hi";
    let (_, plain) = titled_output(other, "Release", TextFallback::Allowed, &term);
    assert_eq!((plain.matches("Release").count(), plain.matches("Other").count()), (1, 1), "{plain}");
}

/// A node, lane, edge label, participant, or axis title equal to the title
/// is not the diagram's title: the explicit title is still shown, once,
/// beside that label.
#[test]
fn a_label_equal_to_the_title_does_not_stand_in_for_it() {
    let term = plain_no_images(80);
    for source in [
        "flowchart TD\n A -->|Release| B",
        "flowchart TD\n subgraph Release\n A --> B\n end",
        "sequenceDiagram\n participant Release\n Release->>Bob: Hi",
        "xychart-beta\n x-axis Release [a, b, c]\n y-axis 0 --> 5\n bar [1, 2, 3]",
        "swimlane-beta LR\n  subgraph Release\n    a[Start]\n  end\n  subgraph Other\n    b[End]\n  end\n  a --> b",
    ] {
        let (route, plain) = titled_output(source, "Release", TextFallback::Allowed, &term);
        assert_eq!(route, "text", "{source}");
        assert!(plain.starts_with("Release\n"), "{source}\n{plain}");
        assert_eq!(plain.matches("Release").count(), 2, "the title and the label: {source}\n{plain}");
    }
}

/// R1-F3.3, R1-F3.4: on every source route the shown source gives a
/// matching title once, and an untitled source is led by the explicit
/// title once.
#[test]
fn a_shown_source_gives_a_matching_title_once() {
    let term = plain_no_images(80);
    let pie = "---\ntitle: Release\n---\npie\n    \"Dogs\" : 50";
    let wide = "---\ntitle: Release\n---\nflowchart LR\n    A[Alpha] --> B[Beta] --> C[Gamma] --> D[Delta] --> E[Epsilon]";
    let cases = [
        ("ineligible", pie, TextFallback::Allowed, plain_no_images(80)),
        ("ineligible, statement", "pie title Release\n    \"Dogs\" : 50", TextFallback::Allowed, plain_no_images(80)),
        ("policy", FRONTMATTER_FLOWCHART, TextFallback::Never, plain_no_images(80)),
        ("too wide", wide, TextFallback::Allowed, plain_no_images(40)),
    ];
    for (why, source, text_fallback, term) in &cases {
        let (route, plain) = titled_output(source, "Release", *text_fallback, term);
        assert_eq!(route, "source", "{why}");
        assert_eq!(plain.matches("Release").count(), 1, "{why}\n{plain}");
        assert!(plain.starts_with("```mermaid"), "no title paragraph: {why}\n{plain}");

        let untitled = source.replace("title: Release\n", "").replace("pie title Release", "pie");
        let (route, plain) = titled_output(&untitled, "Release", *text_fallback, term);
        assert_eq!(route, "source", "{why}, untitled");
        assert_eq!(plain.matches("Release").count(), 1, "{why}, untitled\n{plain}");
        assert!(plain.starts_with("Release\n"), "the title paragraph leads: {why}\n{plain}");
    }

    // A double-quoted frontmatter title is read with its escapes.
    let escaped = "---\ntitle: \"a \\\"b\\\" \\\\ c\"\n---\npie\n    \"Dogs\" : 50";
    let (_, plain) = titled_output(escaped, "a \"b\" \\ c", TextFallback::Allowed, &term);
    assert!(plain.starts_with("```mermaid"), "no title paragraph: {plain}");
}

/// R1-F3.3 on the image route: a source shown after an image failure gives
/// a matching title once. One render: each image attempt builds the font
/// chain.
#[test]
#[serial]
fn a_source_shown_after_an_image_failure_gives_a_matching_title_once() {
    let _cache = isolated_cache_dir();
    let broken = "---\ntitle: Release\n---\nflowchart LR\n    A[unclosed --> B";
    let diagram = MermaidDiagram::new(broken).with_title("Release");

    let plain = strip_escape_codes(diagram.render(&Terminal::new_optimistic(80)));

    assert!(plain.starts_with("```mermaid\n---\ntitle: Release"), "the source, untitled: {plain}");
    assert_eq!(plain.matches("Release").count(), 1, "{plain}");
}

// ── Width (D7 with the component's width limits) ─────────────────────────

/// The drawing gets the component's width less the code block's padding:
/// the chain is 58 cells compactly, so 60 columns fit and 59 do not.
#[test]
fn a_cell_width_less_the_code_block_padding_bounds_the_drawing() {
    let term = plain_no_images(200);
    let fits = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Characters(60));
    assert_eq!(presented(&fits, &term, ALLOWED).0, "text");
    let narrow = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Characters(59));
    assert_eq!(presented(&narrow, &term, ALLOWED).0, "source");
}

#[test]
fn percent_and_margins_narrow_the_drawing() {
    let half = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Percent(0.5));
    assert_eq!(presented(&half, &plain_no_images(120), ALLOWED).0, "text", "60 of 120");
    assert_eq!(presented(&half, &plain_no_images(118), ALLOWED).0, "source", "59 of 118");

    let mut margined = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Fill);
    margined.layout_mut().margin = Edges { left: TargetValue::universal(Length::ch(4)), ..Edges::default() };
    assert_eq!(presented(&margined, &plain_no_images(64), ALLOWED).0, "text", "60 inside the margin");
    assert_eq!(presented(&margined, &plain_no_images(63), ALLOWED).0, "source", "59 inside the margin");
}

/// Margins and alignment apply once: every line starts with exactly the
/// four-column margin.
#[test]
fn the_margin_applies_once_to_the_text_presentation() {
    let mut diagram = MermaidDiagram::new(SPEC_FLOWCHART)
        .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed));
    diagram.layout_mut().margin = Edges { left: TargetValue::universal(Length::ch(4)), ..Edges::default() };
    let plain = strip_escape_codes(diagram.render(&plain_no_images(80)));
    for line in plain.lines().filter(|line| !line.is_empty()) {
        assert!(line.starts_with("    ") && !line.starts_with("     ```"), "{line:?}");
    }
    assert!(plain.contains("\n    ```txt") || plain.starts_with("    ```txt"), "{plain}");
    assert!(plain.contains("\n    │ Write spec ├"), "{plain}");
}

// ── Work per route (AC-F13) ──────────────────────────────────────────────

/// The default scaled width measures no SVG on the text route: one text
/// request, no font chain, no artifact.
#[test]
#[serial]
fn the_text_route_does_no_image_work() {
    let _cache = isolated_cache_dir();
    reset_counters();
    // The component's default width is `ImageWidth::Scale(1.0)`.
    let diagram = MermaidDiagram::new(SPEC_FLOWCHART);

    let (route, _) = presented(&diagram, &plain_no_images(80), ALLOWED);

    assert_eq!(route, "text");
    assert_eq!(take_text_requests(), 1);
    assert_eq!(take_default_chain_requests(), 0, "no font chain");
    assert_eq!(take_stored_files(), 0, "no artifact");
}

/// Control and the image half: an image-capable terminal takes the image
/// route (an artifact, the font chain, no text request); a failure there
/// shows the source and is not retried through text.
#[test]
#[serial]
fn the_image_route_never_falls_back_through_text() {
    let _cache = isolated_cache_dir();
    reset_counters();
    let kitty = Terminal::new_optimistic(80);

    let (route, _) = presented(&MermaidDiagram::new(SPEC_FLOWCHART), &kitty, ALLOWED);
    assert_eq!(route, "image");
    assert!(take_stored_files() > 0, "the PNG artifact");
    assert!(take_default_chain_requests() > 0, "the font chain");
    assert_eq!(take_text_requests(), 0);

    let (route, nodes) = presented(&MermaidDiagram::new("flowchart LR\n    A[unclosed --> B"), &kitty, ALLOWED);
    assert_eq!((route, shape(&nodes)), ("source", vec!["code:mermaid".to_string()]));
    assert_eq!(take_text_requests(), 0, "no text retry after an image failure");
}
