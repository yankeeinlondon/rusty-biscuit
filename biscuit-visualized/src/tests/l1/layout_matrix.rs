//! The diagram rows of the layout visual-test matrix.
//!
//! biscuit-terminal's layout matrix renders every component under the same
//! scenarios; its diagram rows live here because the diagram components do.
//! The scenarios and row builders are biscuit-terminal's own
//! `layout_matrix_support`, included unchanged (`source-inputs` in
//! Cargo.toml), and the per-row checks below are copied from its
//! `layout_matrix.rs`, so both packages hold every row to one contract.
//!
//! Regenerate snapshots after intentional changes with:
//! `INSTA_UPDATE=always cargo nextest run -p biscuit-visualized --all-features --test l1 layout_matrix::`
//! then review the `.snap` diff.

use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use biscuit_visualized::components::graph_expression::{GraphExpression, GraphInputSyntax};
use biscuit_visualized::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::renderable::TerminalRenderable;
use renderable::tree::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, RenderStrictness,
    render_browser_node, render_markdown_node,
};

use crate::layout_matrix_support::{
    ComponentCase, Scenario, scenarios, stacked_stripped, tree_only_case,
};

/// The diagram components exercised by the layout matrix.
fn component_cases() -> Vec<ComponentCase> {
    vec![
        tree_only_case(
            "GraphExpression",
            |s: &Scenario| {
                GraphExpression::parse("a -> b -> c", GraphInputSyntax::Auto)
                    .expect("parse graph")
                    .with_layout(s.layout.clone())
                    .with_style(s.style.clone())
            },
            "bespoke rasterized graph; outer placement parity is in graph_expression_parity.rs",
        ),
        tree_only_case(
            "MermaidDiagram",
            |s: &Scenario| {
                MermaidDiagram::new("flowchart LR\n    A --> B --> C")
                    .with_layout(s.layout.clone())
                    .with_style(s.style.clone())
            },
            "bespoke external render; outer placement parity is in mermaid_parity.rs",
        ),
    ]
}

#[test]
fn diagram_rows_use_the_shared_scenarios() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include in `main.rs`.
    let _ = include_str!("../../../../biscuit-terminal/lib/tests/layout_matrix_support/mod.rs");
    assert_eq!(scenarios().len(), 19);
    let names: Vec<&str> = component_cases().iter().map(|case| case.name).collect();
    assert_eq!(names, ["GraphExpression", "MermaidDiagram"]);
}

#[test]
fn every_case_renders_non_empty() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (via_render, via_tree_direct) = (case.render)(&scenario);
            assert!(
                !via_render.is_empty(),
                "{}/{}: render(&term) output was empty",
                case.name,
                scenario.name
            );
            assert!(
                !via_tree_direct.is_empty(),
                "{}/{}: via_tree_direct output was empty",
                case.name,
                scenario.name
            );
        }
    }
}

#[test]
#[ignore = "FIXME(table-width): unresolved whether a `Fixed` table width means fill-the-parent; Table hugs on Auto/FitContent but stretches its last column on any Fixed, unlike the shared Width contract"]
fn layout_matrix_snapshots() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (via_render, via_tree_direct) = (case.render)(&scenario);
            let block = stacked_stripped(&via_render, &via_tree_direct);
            insta::assert_snapshot!(format!("{}__{}", case.name, scenario.name), block);
        }
    }
}

/// Presence of the style-bearing SGR runs and border glyph that survive (or
/// are erased by) ANSI stripping. Compared between the two render paths so a
/// public `render(&term)` that ignores a user-set `Style` is caught even when
/// the difference is pure SGR.
fn appearance_signature(s: &str) -> (bool, bool, bool, bool) {
    (
        s.contains("\u{1b}[1m"),  // bold
        s.contains("\u{1b}[3m"),  // italic
        s.contains("\u{1b}[48;"), // background fill
        s.contains('│'),          // left border glyph
    )
}

#[test]
fn render_path_parity_for_every_matrix_cell() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (via_render, via_tree_direct) = (case.render)(&scenario);
            let render_stripped = strip_escape_codes(&via_render);
            let tree_stripped = strip_escape_codes(&via_tree_direct);
            assert_eq!(
                render_stripped.trim(),
                tree_stripped.trim(),
                "{} / {}: VIA_RENDER must equal VIA_TREE_DIRECT \
                 (ANSI-stripped and trimmed)",
                case.name,
                scenario.name,
            );
            assert_eq!(
                appearance_signature(&via_render),
                appearance_signature(&via_tree_direct),
                "{} / {}: public render(&term) must carry the same appearance \
                 markers (bold/italic/background/border) as the tree fold; a \
                 divergence here means the public path ignored the user-set style",
                case.name,
                scenario.name,
            );
        }
    }
}

#[test]
fn layout_matrix_browser_snapshots() {
    for case in component_cases() {
        let Some(project_tree) = case.project_tree.as_ref() else {
            continue;
        };
        for scenario in scenarios() {
            let node = (project_tree)(&scenario);
            let html = render_browser_node(&node, &BrowserRenderOptions::default())
                .expect("browser fold")
                .output
                .render();
            insta::assert_snapshot!(
                format!("{}__{}__browser", case.name, scenario.name),
                html
            );
        }
    }
}

#[test]
fn layout_matrix_markdown_snapshots() {
    for case in component_cases() {
        let Some(project_tree) = case.project_tree.as_ref() else {
            continue;
        };
        for scenario in scenarios() {
            let node = (project_tree)(&scenario);
            let md = render_markdown_node(
                &node,
                &MarkdownRenderOptions {
                    dialect: MarkdownDialect::Markdown,
                    strictness: RenderStrictness::Warn,
                    ..MarkdownRenderOptions::default()
                },
            )
            .expect("markdown fold")
            .output;
            insta::assert_snapshot!(
                format!("{}__{}__markdown", case.name, scenario.name),
                md
            );
        }
    }
}

#[test]
fn no_silent_noop_guard() {
    let markdown_opts = MarkdownRenderOptions {
        dialect: MarkdownDialect::Markdown,
        strictness: RenderStrictness::Warn,
        ..MarkdownRenderOptions::default()
    };

    for case in component_cases() {
        // Every terminal-only row must document why it has no tree projection.
        if case.project_tree.is_none() {
            assert!(
                !case.notes.is_empty(),
                "{}: terminal-only matrix row must carry a notes rationale",
                case.name
            );
            continue;
        }

        let project_tree = case.project_tree.as_ref().unwrap();

        for scenario in scenarios() {
            if scenario.name == "baseline" {
                continue;
            }
            let node = (project_tree)(&scenario);

            // D1 guard: Markdown never leaks appearance markup.
            let md = render_markdown_node(&node, &markdown_opts)
                .expect("markdown fold")
                .output;
            assert!(
                !md.contains('\u{1b}'),
                "{} / {}: ANSI escape leaked into Markdown",
                case.name,
                scenario.name
            );
            assert!(
                !md.contains("style=")
                    && !md.contains("background-color")
                    && !md.contains("border-left"),
                "{} / {}: CSS styling leaked into Markdown: {md:?}",
                case.name,
                scenario.name
            );

            // Style scenarios must leave a detectable artifact in the tree path.
            if !scenario.style.is_empty() {
                let tree = render_terminal_node(
                    &node,
                    &TerminalRenderOptions::new(
                        &Terminal::new_optimistic(scenario.width),
                        RenderStrictness::Warn,
                    ),
                )
                .expect("tree render")
                .output;
                match scenario.name {
                    "background_subtle" => {
                        assert!(
                            tree.contains("\u{1b}[48;"),
                            "{} / {}: background_subtle did not lower to a background SGR",
                            case.name,
                            scenario.name
                        );
                    }
                    "border_thin_left" => {
                        assert!(
                            tree.contains('│'),
                            "{} / {}: border_thin_left did not lower to a left border glyph",
                            case.name,
                            scenario.name
                        );
                    }
                    "emphasis_bold_italic" => {
                        assert!(
                            tree.contains("\u{1b}[1m") && tree.contains("\u{1b}[3m"),
                            "{} / {}: emphasis_bold_italic did not lower to bold+italic SGR",
                            case.name,
                            scenario.name
                        );
                    }
                    other => panic!("unknown style scenario: {other}"),
                }
            }
        }
    }
}
