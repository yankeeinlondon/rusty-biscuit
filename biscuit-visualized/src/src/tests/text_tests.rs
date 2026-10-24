//! Merman's text target: eligibility from Merman's metadata (D1, R-F2,
//! AC-F3), the layout and fit rules (D7, AC-F11), the projection guard
//! (AC-F2), exact drawings in both character sets (AC-F14), and the work a
//! text render may do (AC-F13: no artifact, no font chain, at most one
//! request).
//!
//! Boundary: this crate ↔ Merman's `ascii` target, in process (F2, L1). The
//! drawings are Merman's real output at the pinned rev; a rev bump that
//! changes them shows here.

use std::time::Duration;

use merman::ascii::{
    AsciiLayoutProfile, AsciiPrimaryProjection, AsciiResourceLimitId, AsciiResourcePolicy, AsciiSemanticCoverage,
    OverflowPolicy, ascii_capabilities,
};
use serial_test::serial;

use super::golden_tests::golden_chain;
use crate::artifact::{OutputFormat, RenderRequest};
use crate::cache::file_cache::take_stored_files;
use crate::fonts::chain::take_default_chain_requests;
use crate::mermaid::render::take_text_requests;
use crate::mermaid::text::{eligibility_of, judge};
use crate::mermaid::{
    MermaidDiagram, MermaidError, TextCharset, TextCoverage, TextLayout, TextOptions, TextProjection, TextRender,
    TextReport, TextSource, text_eligibility,
};
use crate::tests::{capture_warnings, isolated_cache_dir};

/// The spec's flowchart: its `click` link is one of the features text drops.
const SPEC_FLOWCHART: &str = "flowchart LR
    A[Write spec] --> B{Approved?}
    B -- yes --> C[Plan]
    B -- no --> A
    click C \"https://example.com\"";

/// Canonically 66 cells wide, compactly 58.
const CHAIN_FLOWCHART: &str = "flowchart LR
    A[Alpha] --> B[Beta] --> C[Gamma] --> D[Delta] --> E[Epsilon]";

/// Canonical only (state admits no `auto` layout); 12 cells wide.
const STATE_DIAGRAM: &str = "stateDiagram-v2
    [*] --> Idle
    Idle --> Running : start
    Running --> Finished : done
    Finished --> [*]";

const DENSE_CLASS: &str = "classDiagram
    Animal <|-- Duck
    Animal <|-- Fish
    Animal : +int age
    Duck : +swim()";

fn render(source: &str, options: TextOptions) -> TextRender {
    MermaidDiagram::new(source).render_text(options).expect("the text render succeeds")
}

fn drawing(source: &str, options: TextOptions) -> String {
    match render(source, options) {
        TextRender::Drawing(drawing) => drawing.into_text(),
        TextRender::Source(reason) => panic!("expected a drawing, got the source: {reason:?}"),
    }
}

fn too_wide(result: TextRender) -> TextReport {
    match result {
        TextRender::Source(TextSource::TooWide(report)) => report,
        other => panic!("expected the source for an over-wide drawing, got {other:?}"),
    }
}

fn failure(result: Result<TextRender, MermaidError>) -> String {
    match result {
        Err(MermaidError::RenderFailed(reason)) => reason,
        other => panic!("expected a render failure, got {other:?}"),
    }
}

// --- Eligibility (D1, R-F2, AC-F3) ----------------------------------------

/// R-F2: the eligibility derived from every capability row at the pinned rev.
/// A Merman rev bump that changes any row changes this table in review.
#[test]
fn eligibility_table_at_the_pinned_rev() {
    let table: Vec<String> = ascii_capabilities()
        .iter()
        .map(|capability| {
            let layouts: Vec<String> = capability.layout_profiles.iter().map(|layout| format!("{layout:?}")).collect();
            let eligible = match eligibility_of(capability) {
                Some(eligibility) => format!("eligible {:?} auto={}", eligibility.coverage(), eligibility.admits_auto_layout()),
                None => "source".to_string(),
            };
            format!(
                "{} | {:?} | {:?} | [{}] | {eligible}",
                capability.diagram_type,
                capability.primary_projection,
                capability.semantic_coverage,
                layouts.join(", ")
            )
        })
        .collect();
    let expected = "\
agentflow | None | None | [] | source
architecture | None | None | [] | source
block | None | None | [] | source
c4 | None | None | [] | source
class | Diagrammatic | Some(Partial) | [Canonical] | eligible Partial auto=false
cynefin | None | None | [] | source
er | Diagrammatic | Some(Partial) | [Canonical] | eligible Partial auto=false
eventmodeling | None | None | [] | source
flowchart | Diagrammatic | Some(Partial) | [Canonical, Compact, Auto] | eligible Partial auto=true
gantt | StructuredText | Some(Partial) | [Canonical] | source
gitgraph | StructuredText | Some(Partial) | [Canonical] | source
info | None | None | [] | source
ishikawa | None | None | [] | source
journey | StructuredText | Some(Partial) | [Canonical] | source
kanban | StructuredText | Some(Partial) | [Canonical] | source
mindmap | StructuredText | Some(Partial) | [Canonical] | source
packet | StructuredText | Some(Partial) | [Canonical] | source
pie | None | None | [] | source
quadrantchart | None | None | [] | source
radar | None | None | [] | source
railroad | None | None | [] | source
requirement | None | None | [] | source
sankey | None | None | [] | source
sequence | Diagrammatic | Some(Partial) | [Canonical, Compact, Auto] | eligible Partial auto=true
state | Diagrammatic | Some(Partial) | [Canonical] | eligible Partial auto=false
swimlane | Diagrammatic | Some(Partial) | [Canonical, Compact, Auto] | eligible Partial auto=true
timeline | StructuredText | Some(Partial) | [Canonical] | source
treeView | StructuredText | Some(Partial) | [Canonical] | source
treemap | None | None | [] | source
usecase | None | None | [] | source
venn | None | None | [] | source
wardley | None | None | [] | source
xychart | Diagrammatic | Some(Partial) | [Canonical] | eligible Partial auto=false
zenuml | None | None | [] | source";
    assert_eq!(table.join("\n"), expected);
}

/// AC-F3: exactly seven families are eligible at the delivered rev (D1 as
/// amended for swimlane), each through the public lookup.
#[test]
fn seven_families_are_eligible_at_the_pinned_rev() {
    let eligible: Vec<&str> = ascii_capabilities()
        .iter()
        .filter_map(|capability| text_eligibility(capability.diagram_type))
        .map(|eligibility| eligibility.family())
        .collect();
    assert_eq!(eligible, ["class", "er", "flowchart", "sequence", "state", "swimlane", "xychart"]);
}

/// D1, one edit per cell on the real flowchart row: the control row is
/// eligible; each edited field decides the outcome alone.
#[test]
fn eligibility_is_decided_by_projection_coverage_and_layouts() {
    let flowchart = *ascii_capabilities()
        .iter()
        .find(|capability| capability.diagram_type == "flowchart")
        .expect("Merman has a flowchart row");

    let control = eligibility_of(&flowchart).expect("the unedited flowchart row is eligible");
    assert_eq!(control.family(), "flowchart");
    assert_eq!(control.coverage(), TextCoverage::Partial);
    assert!(control.admits_auto_layout());

    let mut no_coverage = flowchart;
    no_coverage.semantic_coverage = None;
    assert_eq!(eligibility_of(&no_coverage).map(|e| e.coverage()), Some(TextCoverage::Partial));

    let mut full = flowchart;
    full.semantic_coverage = Some(AsciiSemanticCoverage::Full);
    assert_eq!(eligibility_of(&full).map(|e| e.coverage()), Some(TextCoverage::Full));

    let mut structured = flowchart;
    structured.primary_projection = AsciiPrimaryProjection::StructuredText;
    assert_eq!(eligibility_of(&structured), None);

    let mut nothing = flowchart;
    nothing.primary_projection = AsciiPrimaryProjection::None;
    assert_eq!(eligibility_of(&nothing), None);

    let mut canonical_only = flowchart;
    canonical_only.layout_profiles = &[AsciiLayoutProfile::Canonical];
    assert_eq!(eligibility_of(&canonical_only).map(|e| e.admits_auto_layout()), Some(false));
}

#[test]
fn a_family_without_a_capability_row_is_not_eligible() {
    assert_eq!(text_eligibility("not-a-family"), None);
    assert_eq!(text_eligibility(""), None);
    // The registry's family id, not the row's metadata id.
    assert_eq!(text_eligibility("flowchart-v2"), None);
}

/// AC-F1, AC-F3: a diagram's eligibility comes from Merman's own detection of
/// its parsed header, so a comment naming another family is not read.
#[test]
fn diagram_eligibility_follows_merman_detection() {
    let family = |source: &str| MermaidDiagram::new(source).text_eligibility().map(|e| e.family());
    assert_eq!(family("%% mindmap\nflowchart LR\n    A --> B"), Some("flowchart"));
    assert_eq!(family("graph TD\n    A --> B"), Some("flowchart"));
    assert_eq!(family("---\ntitle: T\n---\nstateDiagram\n    [*] --> A"), Some("state"));
    assert_eq!(family("erDiagram\n    A ||--o{ B : has"), Some("er"));
    assert_eq!(family("mindmap\n  root((Merman))\n    Text"), None);
    assert_eq!(family("pie\n    \"A\" : 1"), None);
    assert_eq!(family("gitGraph\n    commit"), None);
    assert_eq!(family("not a diagram"), None);
    assert_eq!(family(""), None);
}

// --- Exact drawings (AC-F14) ----------------------------------------------

#[test]
fn spec_flowchart_draws_in_unicode() {
    let expected = "\
┌────────────┐       ╭───────────╮        ┌──────┐
│            │       ╱           ╲        │      │
│ Write spec ├──────►< Approved? >──yes──►│ Plan │
│            │       ╲           ╱        │      │
└────────────┘       ╰───────────╯        └──────┘
       ▲                   │                      
       └────────no─────────┘                      ";
    assert_eq!(drawing(SPEC_FLOWCHART, TextOptions::new(80)).trim_end_matches('\n'), expected);
}

#[test]
fn spec_flowchart_draws_in_ascii() {
    let text = drawing(SPEC_FLOWCHART, TextOptions::new(80).with_charset(TextCharset::Ascii));
    assert!(text.is_ascii(), "{text}");
    assert!(text.contains("| Write spec |"), "{text}");
    assert!(text.contains("< Approved? >--yes-->| Plan |"), "{text}");
}

/// ASCII changes the borders and connectors only: authored Unicode labels
/// keep every character, and both drawings have the same shape.
#[test]
fn authored_unicode_labels_survive_both_charsets() {
    let source = "flowchart LR\n    A[café ✓] --> B[日本語]";
    let unicode = "\
┌────────┐     ┌────────┐
│        │     │        │
│ café ✓ ├────►│ 日本語 │
│        │     │        │
└────────┘     └────────┘";
    let ascii = "\
+--------+     +--------+
|        |     |        |
| café ✓ |---->| 日本語 |
|        |     |        |
+--------+     +--------+";
    assert_eq!(drawing(source, TextOptions::new(80)).trim_end_matches('\n'), unicode);
    let options = TextOptions::new(80).with_charset(TextCharset::Ascii);
    assert_eq!(drawing(source, options).trim_end_matches('\n'), ascii);
}

/// Significant spaces: centering and the trailing spaces that pad every row
/// to the drawing's width are kept exactly.
#[test]
fn state_diagram_keeps_its_significant_spaces() {
    let text = drawing(STATE_DIAGRAM, TextOptions::new(80));
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows[0], "╭──────────╮");
    assert_eq!(rows[1], "│     ●    │");
    assert_eq!(rows[3], "      │     ");
    assert_eq!(rows[10], "│   Idle   │");
    assert_eq!(rows[15], "    start   ");
    assert_eq!(rows[20], "│ Running  │");
    assert_eq!(rows.last().copied(), Some("╰──────────╯"));
    assert!(rows.iter().all(|row| row.chars().count() == 12), "{text}");
}

#[test]
fn multiline_label_draws_on_separate_rows() {
    let expected = "\
┌─────┐     ┌───┐
│     │     │   │
│ one │     │   │
│     ├────►│ B │
│ two │     │   │
│     │     │   │
└─────┘     └───┘";
    let source = "flowchart LR\n    A[\"one<br/>two\"] --> B";
    assert_eq!(drawing(source, TextOptions::new(80)).trim_end_matches('\n'), expected);
}

/// A label's escape and bell characters are shown escaped, never emitted.
#[test]
fn label_control_characters_never_reach_the_terminal() {
    let source = "flowchart LR\n    A[\"red\u{1b}[31mX\u{7}\"] --> B";
    let text = drawing(source, TextOptions::new(80));
    assert!(!text.chars().any(|c| c.is_control() && c != '\n'), "{text:?}");
    assert!(text.contains(r"red\u{1B}[31mX\u{7}"), "{text}");
}

// --- Width and layout (D7, AC-F11) ----------------------------------------

/// The fit check is in display cells, inclusive: 50 columns fit the
/// 50-cell drawing, 49 do not (and its compact layout is no narrower).
#[test]
fn a_drawing_fits_exactly_its_own_width() {
    let TextRender::Drawing(fits) = render(SPEC_FLOWCHART, TextOptions::new(50)) else {
        panic!("a 50-cell drawing fits 50 columns");
    };
    assert_eq!(fits.report().width, 50);
    assert_eq!(fits.coverage(), TextCoverage::Partial);
    let narrower = too_wide(render(SPEC_FLOWCHART, TextOptions::new(49)));
    assert!(narrower.width > 49 && narrower.overflowed, "{narrower:?}");
}

/// Wide characters count two cells each, by this crate's own check. Merman's
/// real drawing is cut to its label row (22 characters, 25 cells) and its
/// overflow flag cleared, so only this crate's measurement can decide: the
/// source at 22 columns, a drawing at 25.
#[test]
fn wide_characters_are_measured_in_cells() {
    let source = "flowchart LR\n    A[café ✓] --> B[日本語]";
    let eligibility = text_eligibility("flowchart").expect("flowchart is eligible");
    let mut output = match merman::Renderer::new().render(merman::RenderRequest::ascii(
        source,
        merman::OperationControl::new(),
        merman::AsciiRequest::default(),
    )) {
        Ok(merman::RenderOutput::Ascii(Some(output))) => output,
        other => panic!("expected Merman's text output, got {other:?}"),
    };
    let label_row = output.text.lines().find(|row| row.contains("日本語")).expect("the label row").to_string();
    assert_eq!(label_row.chars().count(), 22);
    output.text = label_row;
    output.overflowed = false;

    match judge(&output, eligibility, TextOptions::new(22)) {
        TextRender::Source(TextSource::TooWide(report)) => assert_eq!(report.width, 25),
        other => panic!("expected the source at 22 columns, got {other:?}"),
    }
    assert!(matches!(judge(&output, eligibility, TextOptions::new(25)), TextRender::Drawing(_)));
}

#[test]
fn flowchart_too_wide_canonically_draws_compactly() {
    take_text_requests();
    let TextRender::Drawing(compact) = render(CHAIN_FLOWCHART, TextOptions::new(60)) else {
        panic!("the compact layout fits 60 columns");
    };
    assert_eq!(compact.report().requested_layout, TextLayout::Auto);
    assert_eq!(compact.report().layout, Some(TextLayout::Compact));
    assert_eq!(compact.report().width, 58);
    assert!(compact.text().contains("│ Alpha ├──►│ Beta ├──►│"), "{}", compact.text());
    assert_eq!(take_text_requests(), 1, "auto is one request");

    let TextRender::Drawing(canonical) = render(CHAIN_FLOWCHART, TextOptions::new(66)) else {
        panic!("the canonical layout fits 66 columns");
    };
    assert_eq!(canonical.report().layout, Some(TextLayout::Canonical));
}

#[test]
fn flowchart_too_wide_for_both_layouts_shows_source() {
    let report = too_wide(render(CHAIN_FLOWCHART, TextOptions::new(50)));
    assert_eq!(report.requested_layout, TextLayout::Auto);
    assert_eq!(report.layout, Some(TextLayout::Compact));
    assert!(report.overflowed);
    assert_eq!(report.width, 58);
}

#[test]
fn state_diagram_too_wide_canonically_shows_source_after_one_request() {
    take_text_requests();
    let report = too_wide(render(STATE_DIAGRAM, TextOptions::new(11)));
    assert_eq!(report.requested_layout, TextLayout::Canonical);
    assert_eq!(report.layout, Some(TextLayout::Canonical));
    assert_eq!(report.width, 12);
    assert_eq!(take_text_requests(), 1);
}

#[test]
fn zero_columns_show_source_without_a_request() {
    take_text_requests();
    assert_eq!(render(SPEC_FLOWCHART, TextOptions::new(0)), TextRender::Source(TextSource::NoColumns));
    assert_eq!(take_text_requests(), 0);
}

// --- Projection guard (D1, AC-F2) -----------------------------------------

/// AC-F2: a dense class diagram at a narrow width shows the source. With the
/// overflow allowed (as `render_text` requests), Merman reports the
/// overflow; it never substitutes its report.
#[test]
fn dense_class_diagram_at_a_narrow_width_shows_source() {
    let report = too_wide(render(DENSE_CLASS, TextOptions::new(20)));
    assert_eq!(report.projection, Some(TextProjection::Diagrammatic));
    assert!(report.overflowed);
}

/// AC-F2: Merman's real structured report for a diagrammatic family (what it
/// returns when asked to fall back on overflow) is judged as the source.
#[test]
fn structured_text_result_of_a_diagrammatic_family_shows_source() {
    let eligibility = text_eligibility("class").expect("class is eligible");
    let options = TextOptions::new(20);
    let mut request = merman::AsciiRequest::default();
    request.viewport.max_width = Some(20);
    request.viewport.overflow = OverflowPolicy::Fallback;
    let output = match merman::Renderer::new().render(merman::RenderRequest::ascii(
        DENSE_CLASS,
        merman::OperationControl::new(),
        request,
    )) {
        Ok(merman::RenderOutput::Ascii(Some(output))) => output,
        other => panic!("expected Merman's text output, got {other:?}"),
    };

    match judge(&output, eligibility, options) {
        TextRender::Source(TextSource::NotDiagrammatic(report)) => {
            assert_eq!(report.projection, Some(TextProjection::StructuredText));
        }
        other => panic!("expected the source for a structured report, got {other:?}"),
    }
}

/// One edit per cell on Merman's real output: the control is a drawing;
/// emptied text is the source.
#[test]
fn empty_output_shows_source() {
    let eligibility = text_eligibility("flowchart").expect("flowchart is eligible");
    let options = TextOptions::new(80);
    let output = match merman::Renderer::new().render(merman::RenderRequest::ascii(
        SPEC_FLOWCHART,
        merman::OperationControl::new(),
        merman::AsciiRequest::default(),
    )) {
        Ok(merman::RenderOutput::Ascii(Some(output))) => output,
        other => panic!("expected Merman's text output, got {other:?}"),
    };
    assert!(matches!(judge(&output, eligibility, options), TextRender::Drawing(_)));

    let mut empty = output;
    empty.text = " \n\n".to_string();
    assert!(matches!(judge(&empty, eligibility, options), TextRender::Source(TextSource::Empty(_))));
}

// --- Ineligible input and failures (AC-F1, AC-F14) -------------------------

#[test]
fn ineligible_families_show_source_without_a_request() {
    take_text_requests();
    for source in ["mindmap\n  root((Merman))\n    Text", "pie\n    \"A\" : 1", "zenuml\n    A.b()"] {
        assert_eq!(render(source, TextOptions::new(80)), TextRender::Source(TextSource::Ineligible), "{source}");
    }
    assert_eq!(take_text_requests(), 0);
}

#[test]
fn invalid_source_is_an_error_after_one_request() {
    take_text_requests();
    let reason = failure(MermaidDiagram::new("flowchart LR\n    A[unclosed --> B").render_text(TextOptions::new(80)));
    assert!(reason.contains("parse error"), "{reason}");
    assert_eq!(take_text_requests(), 1);
}

/// A zero budget is past its deadline at Merman's first checkpoint, so the
/// cancellation is reached without sleeping; it is not retried.
#[test]
fn exceeded_time_budget_is_an_error_with_a_warning_and_no_retry() {
    take_text_requests();
    let diagram = MermaidDiagram::new(SPEC_FLOWCHART).with_time_budget(Duration::ZERO);
    let (result, warnings) = capture_warnings(|| diagram.render_text(TextOptions::new(80)));
    let reason = failure(result);
    assert!(reason.contains("time budget"), "{reason}");
    assert!(warnings.iter().any(|warning| warning.contains("time budget")), "{warnings:?}");
    assert_eq!(take_text_requests(), 1);
}

/// A resource limit set below the drawing's size is Merman's real limit
/// failure; it is not retried.
#[test]
fn reached_resource_limit_is_an_error_and_no_retry() {
    take_text_requests();
    let resources = AsciiResourcePolicy::default()
        .with_limit(AsciiResourceLimitId::MaxOutputBytes, 16)
        .expect("16 bytes is above the limit's minimum");
    let reason = failure(MermaidDiagram::new(SPEC_FLOWCHART).render_text_with(TextOptions::new(80), resources));
    assert!(reason.contains("max_ascii_output_bytes"), "{reason}");
    assert_eq!(take_text_requests(), 1);
}

// --- Work done (R-F10, AC-F13, backend half) -------------------------------

#[test]
#[serial]
fn text_render_writes_no_artifact_and_asks_for_no_font_chain() {
    let cache = isolated_cache_dir();
    take_stored_files();
    take_default_chain_requests();
    take_text_requests();

    assert!(matches!(render(SPEC_FLOWCHART, TextOptions::new(80)), TextRender::Drawing(_)));

    assert_eq!(take_text_requests(), 1);
    assert_eq!(take_stored_files(), 0);
    assert_eq!(take_default_chain_requests(), 0);
    let written = std::fs::read_dir(cache.path()).expect("the cache dir is readable").count();
    assert_eq!(written, 0, "nothing is written under the cache dir");
}

/// Control for the counters above: an image render is counted by both.
#[test]
#[serial]
fn the_work_counters_count_image_work() {
    let _cache = isolated_cache_dir();
    take_stored_files();
    take_default_chain_requests();

    let request = RenderRequest { format: OutputFormat::Svg, ..RenderRequest::default() };
    MermaidDiagram::new(SPEC_FLOWCHART).render_in(golden_chain(), &request).expect("the SVG renders");
    assert_eq!(take_stored_files(), 1);

    crate::fonts::default_chain();
    assert_eq!(take_default_chain_requests(), 1);
}
