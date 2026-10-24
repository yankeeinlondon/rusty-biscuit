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
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
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

/// A terminal whose graphics route has no pixels.
fn no_images(width: u32) -> Terminal {
    Terminal::builder().width(width).image_support(ImageSupport::None).is_tty(false).build()
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
    let term = no_images(80);

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

    let output = diagram.render(&no_images(80));

    assert_eq!(strip_escape_codes(output.as_str()), SPEC_FLOWCHART_PLAIN.join("\n"));
    // `Note:` bold, the bracketed id dim and the id italic (here the
    // wrap falls between `id:` and the id).
    assert!(output.contains("\x1b[1mNote:\x1b[0m"), "{output:?}");
    assert!(output.contains("[\x1b[2mid:\n"), "{output:?}");
    assert!(output.contains(&format!("\x1b[3m{PARTIAL_NOTE_ID}")), "{output:?}");
    // Unwrapped, the italic id sits inside the dim run.
    let wide = diagram.render(&no_images(200));
    assert!(wide.contains(&format!("[\x1b[2mid: \x1b[3m{PARTIAL_NOTE_ID}\x1b[0m")), "{wide:?}");
}

#[test]
fn a_hidden_note_leaves_only_the_drawing() {
    let policy = ResolvedFallbackPolicy { show_partial_note: false, ..ALLOWED };
    let (route, nodes) = presented(&MermaidDiagram::new(SPEC_FLOWCHART), &no_images(80), policy);
    assert_eq!(route, "text");
    assert_eq!(shape(&nodes), ["code:txt"]);

    let diagram = MermaidDiagram::new(SPEC_FLOWCHART).with_fallback_policy(
        MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed).with_show_partial_note(false),
    );
    let plain = strip_escape_codes(diagram.render(&no_images(80)));
    assert_eq!(plain, SPEC_FLOWCHART_PLAIN[..9].join("\n"));
}

#[test]
fn a_terminal_without_unicode_gets_an_ascii_drawing() {
    let mut term = no_images(80);
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
    let term = no_images(400);
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
    assert!(matches!(diagram.present(&no_images(80)), MermaidPresentation::Source(_)));
    assert_eq!(take_text_requests(), 0);
}

#[test]
fn a_text_failure_shows_the_source() {
    let (route, nodes) = presented(&MermaidDiagram::new("flowchart LR\n    A[unclosed --> B"), &no_images(80), ALLOWED);
    assert_eq!((route, shape(&nodes)), ("source", vec!["code:mermaid".to_string()]));
}

// ── Title (Text presentation) ────────────────────────────────────────────

#[test]
fn an_explicit_title_leads_text_and_source_once() {
    let term = no_images(80);
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

// ── Width (D7 with the component's width limits) ─────────────────────────

/// The drawing gets the component's width less the code block's padding:
/// the chain is 58 cells compactly, so 60 columns fit and 59 do not.
#[test]
fn a_cell_width_less_the_code_block_padding_bounds_the_drawing() {
    let term = no_images(200);
    let fits = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Characters(60));
    assert_eq!(presented(&fits, &term, ALLOWED).0, "text");
    let narrow = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Characters(59));
    assert_eq!(presented(&narrow, &term, ALLOWED).0, "source");
}

#[test]
fn percent_and_margins_narrow_the_drawing() {
    let half = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Percent(0.5));
    assert_eq!(presented(&half, &no_images(120), ALLOWED).0, "text", "60 of 120");
    assert_eq!(presented(&half, &no_images(118), ALLOWED).0, "source", "59 of 118");

    let mut margined = MermaidDiagram::new(CHAIN_FLOWCHART).with_width(ImageWidth::Fill);
    margined.layout_mut().margin = Edges { left: TargetValue::universal(Length::ch(4)), ..Edges::default() };
    assert_eq!(presented(&margined, &no_images(64), ALLOWED).0, "text", "60 inside the margin");
    assert_eq!(presented(&margined, &no_images(63), ALLOWED).0, "source", "59 inside the margin");
}

/// Margins and alignment apply once: every line starts with exactly the
/// four-column margin.
#[test]
fn the_margin_applies_once_to_the_text_presentation() {
    let mut diagram = MermaidDiagram::new(SPEC_FLOWCHART)
        .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed));
    diagram.layout_mut().margin = Edges { left: TargetValue::universal(Length::ch(4)), ..Edges::default() };
    let plain = strip_escape_codes(diagram.render(&no_images(80)));
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

    let (route, _) = presented(&diagram, &no_images(80), ALLOWED);

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
