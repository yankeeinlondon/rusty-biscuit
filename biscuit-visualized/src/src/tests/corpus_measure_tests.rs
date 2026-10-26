//! AC-M1 and AC-M2 over the golden corpus: every family's golden SVG (the
//! `Default` fixed palette, bundled Inter, as `golden_tests` renders it).
//!
//! The input is the golden SVG rather than a bare Merman render because it is
//! what ships: the post-processed SVG with its painted background, so a label
//! drawn past the canvas edge counts as overflowing its box. The oracle and
//! survey draw with the golden's own chain, the faces it measured with.
//!
//! One module per family, each check its own test, so no test renders more
//! than one diagram.

use serial_test::serial;

use super::golden_tests::{GOLDEN_SOURCES, golden_chain, golden_svg_text};
use super::measure_tests::{Survey, compare_drawn_labels};
use crate::mermaid::{MermaidTheme, Palette};

/// Overflows traced to Merman, not to measurement (the measurer's width for
/// each equals usvg's drawn width; see each family's oracle test).
const MERMAN_CORPUS_OVERFLOWS: [(&str, &str, &str); 2] = [
    (
        "swimlane",
        "Needs code change",
        "Merman wraps a swimlane edge label at 120 px and sizes its box for two lines, but draws the text as one line",
    ),
    (
        "radar",
        "Intelligence",
        "Merman's radar canvas is a fixed 700 px whatever its axis labels' widths; the left axis label starts off the canvas",
    ),
];

/// Labels with no box, where the survey's backing shape is something else:
/// the radar legend sits beside the outer graticule circle, whose bounding
/// square the survey takes as the legend's box.
const NO_BOX_LABELS: [(&str, &str); 3] =
    [("radar", "Naruto"), ("radar", "Sasuke"), ("radar", "Ninja")];

fn golden_svg(fixture: &str) -> String {
    let (_, source) = GOLDEN_SOURCES
        .iter()
        .find(|(name, _)| *name == fixture)
        .unwrap_or_else(|| panic!("{fixture} is a golden fixture"));
    golden_svg_text(source, Palette::fixed(MermaidTheme::Default))
}

/// AC-M1: every comparable drawn label of `fixture`'s golden measures to
/// usvg's width with usvg's glyphs and faces; at least `min_labels` compared.
fn assert_widths_match_usvg(fixture: &str, min_labels: usize) {
    let svg = golden_svg(fixture);
    let (failures, compared, worst) = compare_drawn_labels(golden_chain(), &svg, fixture);

    assert!(
        compared >= min_labels,
        "{fixture}: compared {compared} labels, expected at least {min_labels}"
    );
    assert!(
        failures.is_empty(),
        "worst {worst} px:\n{}",
        failures.join("\n")
    );
}

/// AC-M2: no label of `fixture`'s golden is clipped or past its box, except
/// the allow-listed ones, each of which must still overflow.
fn assert_labels_fit_their_boxes(fixture: &str) {
    let svg = golden_svg(fixture);
    let allowed = |label: &str| {
        MERMAN_CORPUS_OVERFLOWS
            .iter()
            .any(|(name, allowed, _)| *name == fixture && *allowed == label)
            || NO_BOX_LABELS.contains(&(fixture, label))
    };
    let found = Survey::run(&svg, golden_chain());

    let unexpected: Vec<String> = found
        .iter()
        .filter(|overflow| !allowed(&overflow.label))
        .map(|overflow| {
            format!(
                "{fixture}: {:?} {} by {:.1} px",
                overflow.label, overflow.kind, overflow.px
            )
        })
        .collect();
    let listed = MERMAN_CORPUS_OVERFLOWS
        .iter()
        .map(|(name, label, _)| (*name, *label))
        .chain(NO_BOX_LABELS)
        .filter(|(name, _)| *name == fixture);
    for (_, label) in listed {
        assert!(
            found.iter().any(|overflow| overflow.label == label),
            "{fixture} {label:?} no longer overflows: remove it from its allow-list"
        );
    }
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

/// One module per family: `$min` is the fewest comparable labels its golden
/// draws (the count when the corpus landed), so the oracle cannot pass on an
/// empty or label-less render.
macro_rules! corpus_measures {
    ($($module:ident => ($fixture:literal, $min:literal)),+ $(,)?) => {
        const CORPUS_FIXTURES: &[(&str, usize)] = &[$(($fixture, $min)),+];

        $(
            mod $module {
                use super::*;

                #[test]
                #[serial]
                fn drawn_widths_match_usvg() {
                    assert_widths_match_usvg($fixture, $min);
                }

                #[test]
                #[serial]
                fn labels_fit_their_boxes() {
                    assert_labels_fit_their_boxes($fixture);
                }
            }
        )+
    };
}

corpus_measures! {
    flowchart => ("flowchart", 14),
    swimlane => ("swimlane", 13),
    mindmap => ("mindmap", 18),
    architecture => ("architecture", 5),
    sequence => ("sequence", 29),
    c4 => ("c4", 10),
    kanban => ("kanban", 34),
    class => ("class", 15),
    er => ("er", 29),
    gantt => ("gantt", 32),
    info => ("info", 1),
    pie => ("pie", 9),
    requirement => ("requirement", 9),
    timeline => ("timeline", 21),
    gitgraph => ("gitgraph", 14),
    state => ("state", 15),
    journey => ("journey", 10),
    quadrant_chart => ("quadrantchart", 15),
    sankey => ("sankey", 7),
    packet => ("packet", 49),
    xychart => ("xychart", 3),
    block => ("block", 5),
    event_modeling => ("eventmodeling", 12),
    tree_view => ("treeView", 11),
    radar => ("radar", 9),
    ishikawa => ("ishikawa", 13),
    treemap => ("treemap", 21),
    railroad => ("railroad", 17),
    venn => ("venn", 6),
    wardley => ("wardley", 13),
    cynefin => ("cynefin", 8),
    usecase => ("usecase", 14),
    agentflow => ("agentflow", 2),
}

/// The corpus checks cover exactly the golden corpus, and compare at least
/// 450 labels across it.
#[test]
fn corpus_measure_families_are_the_golden_corpus() {
    let mut corpus: Vec<&str> = CORPUS_FIXTURES.iter().map(|(name, _)| *name).collect();
    corpus.sort_unstable();
    let mut goldens: Vec<&str> = GOLDEN_SOURCES.iter().map(|(name, _)| *name).collect();
    goldens.sort_unstable();
    assert_eq!(corpus, goldens);

    let labels: usize = CORPUS_FIXTURES.iter().map(|(_, min)| min).sum();
    assert!(labels >= 450, "{labels} labels");
}

/// Every allow-list entry names a golden fixture.
#[test]
fn corpus_measure_allow_lists_name_golden_fixtures() {
    let names = MERMAN_CORPUS_OVERFLOWS
        .iter()
        .map(|(name, _, _)| *name)
        .chain(NO_BOX_LABELS.iter().map(|(name, _)| *name));
    for name in names {
        assert!(
            GOLDEN_SOURCES.iter().any(|(fixture, _)| *fixture == name),
            "{name}"
        );
    }
}
