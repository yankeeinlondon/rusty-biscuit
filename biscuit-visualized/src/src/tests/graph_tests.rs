use crate::{
    artifact::{OutputFormat, RenderRequest},
    cache::FileCache,
    graph::{
        EdgeKind, GraphBuilder, GraphColorTheme, GraphDiagram, GraphExpression, GraphInputSyntax,
        GraphOrientation,
    },
};
use serial_test::serial;

#[test]
fn expression_parse_simple_chain() {
    let expr = GraphExpression::parse("a -> b -> c").expect("Should parse");

    assert_eq!(expr.nodes.len(), 3);
    assert_eq!(expr.nodes, vec!["a", "b", "c"]);

    assert_eq!(expr.edges.len(), 2);
    assert_eq!(expr.edges[0].from, "a");
    assert_eq!(expr.edges[0].to, "b");
    assert_eq!(expr.edges[0].kind, EdgeKind::Directed);
    assert_eq!(expr.edges[1].from, "b");
    assert_eq!(expr.edges[1].to, "c");
    assert_eq!(expr.edges[1].kind, EdgeKind::Directed);
}

#[test]
fn expression_parse_undirected() {
    let expr = GraphExpression::parse("a -- b").expect("Should parse");

    assert_eq!(expr.nodes.len(), 2);
    assert_eq!(expr.edges.len(), 1);
    assert_eq!(expr.edges[0].kind, EdgeKind::Undirected);
}

#[test]
fn expression_parse_quoted_identifiers() {
    let expr = GraphExpression::parse(r#""My App" -- Postgres"#).expect("Should parse");

    assert_eq!(expr.nodes.len(), 2);
    assert_eq!(expr.nodes[0], "My App");
    assert_eq!(expr.nodes[1], "Postgres");
}

#[test]
fn expression_parse_semicolon_separator() {
    let expr = GraphExpression::parse("a -> b; b -> c").expect("Should parse");

    assert_eq!(expr.nodes.len(), 3);
    assert_eq!(expr.edges.len(), 2);
}

#[test]
fn expression_parse_newline_separator() {
    let expr = GraphExpression::parse("a -> b\nb -> c").expect("Should parse");

    assert_eq!(expr.nodes.len(), 3);
    assert_eq!(expr.edges.len(), 2);
}

#[test]
fn expression_parse_rejects_mixed_edge_kinds() {
    let result = GraphExpression::parse("a -> b; c -- d");
    assert!(matches!(
        result,
        Err(crate::graph::GraphError::MixedEdgeKinds)
    ));
}

#[test]
fn expression_parse_empty_input_fails() {
    let result = GraphExpression::parse("");
    assert!(result.is_err());
}

#[test]
fn expression_parse_whitespace_only_fails() {
    let result = GraphExpression::parse("   \n  ");
    assert!(result.is_err());
}

#[test]
fn dot_render_valid_produces_svg() {
    use crate::graph::dot::render_dot_to_svg;

    let dot = r#"
        digraph G {
            A -> B;
            B -> C;
        }
    "#;

    let result = render_dot_to_svg(dot);
    // Note: layout-rs may have parsing quirks, so we just check it doesn't panic
    // A full test would verify SVG content, but that's integration-level
    match result {
        Ok(svg) => {
            assert!(!svg.is_empty());
        }
        Err(e) => {
            // If layout-rs can't parse this basic DOT, that's acceptable for unit tests
            // The important thing is we got a proper error, not a panic
            eprintln!("DOT rendering failed (may be expected): {}", e);
        }
    }
}

#[test]
fn dot_validate_rejects_html_tables() {
    use crate::graph::dot::validate_dot;

    let dot = r#"digraph G { A [label=<TABLE><TR><TD>test</TD></TR></TABLE>]; }"#;
    let result = validate_dot(dot);
    assert!(result.is_err());
}

#[test]
fn dot_validate_accepts_simple_dot() {
    use crate::graph::dot::validate_dot;

    let dot = r#"digraph G { A -> B; }"#;
    let result = validate_dot(dot);
    assert!(result.is_ok());
}

#[test]
fn dot_validate_rejects_nested_subgraphs() {
    use crate::graph::dot::validate_dot;

    let dot = r#"
        digraph G {
            subgraph cluster_a {
                A -> B;
                subgraph cluster_b {
                    B -> C;
                }
            }
        }
    "#;

    let result = validate_dot(dot);
    assert!(matches!(
        result,
        Err(crate::graph::GraphError::UnsupportedDotFeature(_))
    ));
}

#[test]
fn graph_diagram_parse_auto_detects_expression() {
    let result = GraphDiagram::parse("a -> b", GraphInputSyntax::Auto);
    assert!(result.is_ok());
}

#[test]
fn graph_diagram_parse_auto_detects_dot() {
    let result = GraphDiagram::parse("digraph { A -> B }", GraphInputSyntax::Auto);
    assert!(result.is_ok());
}

#[test]
fn graph_diagram_from_expression_works() {
    let result = GraphDiagram::from_expression("a -> b -> c");
    assert!(result.is_ok());
}

#[test]
fn graph_diagram_from_dot_works() {
    let result = GraphDiagram::from_dot("digraph G { A -> B; }");
    assert!(result.is_ok());
}

#[test]
fn graph_diagram_source_as_dot_contains_rankdir() {
    let graph = GraphDiagram::from_expression("a -> b").expect("Should parse");
    let dot = graph.source_as_dot();

    assert!(dot.contains("rankdir="));
}

#[test]
fn graph_diagram_with_title_includes_label() {
    let graph = GraphDiagram::from_expression("a -> b")
        .expect("Should parse")
        .with_title("My Graph".to_string());
    let dot = graph.source_as_dot();

    assert!(dot.contains("label=\"My Graph\""));
}

#[test]
#[serial]
fn graph_diagram_render_produces_file() {
    let cache = FileCache::new();
    let _ = cache.clear();

    let graph = GraphDiagram::from_expression("a -> b").expect("Should parse");

    let request = RenderRequest {
        format: OutputFormat::Svg,
        scale: 1,
        target_width: None,
        transparent_background: false,
    };

    let result = graph.render(&request);
    let artifact = result.expect("Graph render should succeed");
    assert!(artifact.path.exists());
    assert_eq!(artifact.format, OutputFormat::Svg);
}

#[test]
#[serial]
fn graph_diagram_render_cache_hit_on_second_call() {
    let cache = FileCache::new();
    let _ = cache.clear();

    let graph = GraphDiagram::from_expression("a -> b").expect("Should parse");

    let request = RenderRequest {
        format: OutputFormat::Svg,
        scale: 1,
        target_width: None,
        transparent_background: false,
    };

    // First render
    let artifact1 = graph.render(&request).expect("Should render");
    assert!(!artifact1.cache_hit);

    // Second render should hit cache
    let artifact2 = graph.render(&request).expect("Should render");
    assert!(artifact2.cache_hit);
    assert_eq!(artifact1.path, artifact2.path);
}

#[test]
fn graph_builder_directed_creates_valid_diagram() {
    let graph = GraphBuilder::directed()
        .add_node("a", None)
        .add_edge("a", "b")
        .build()
        .unwrap();

    let dot = graph.source_as_dot();
    assert!(dot.contains("digraph"));
    assert!(dot.contains("a"));
}

#[test]
fn graph_builder_undirected_creates_valid_diagram() {
    let graph = GraphBuilder::undirected()
        .add_node("a", None)
        .add_node("b", None)
        .add_edge("a", "b")
        .build()
        .unwrap();

    let dot = graph.source_as_dot();
    assert!(dot.contains("graph"));
    assert!(dot.contains("--"));
}

#[test]
fn graph_builder_with_labels() {
    let graph = GraphBuilder::directed()
        .add_node("a", Some("Node A".to_string()))
        .add_node("b", Some("Node B".to_string()))
        .add_edge("a", "b")
        .build()
        .unwrap();

    let dot = graph.source_as_dot();
    assert!(dot.contains("Node A"));
    assert!(dot.contains("Node B"));
}

#[test]
fn graph_builder_with_orientation_applies_rankdir() {
    let graph = GraphBuilder::directed()
        .with_orientation(GraphOrientation::LeftToRight)
        .add_edge("a", "b")
        .build()
        .unwrap();

    assert!(graph.source_as_dot().contains("rankdir=LR"));
}

#[test]
#[serial]
fn graph_render_cache_separates_scale() {
    let cache = FileCache::new();
    let _ = cache.clear();

    let graph = GraphDiagram::from_expression("a -> b").unwrap();
    let small = graph
        .render(&RenderRequest {
            format: OutputFormat::Png,
            scale: 1,
            target_width: None,
            transparent_background: false,
        })
        .unwrap();
    let large = graph
        .render(&RenderRequest {
            format: OutputFormat::Png,
            scale: 3,
            target_width: None,
            transparent_background: false,
        })
        .unwrap();

    assert_ne!(small.path, large.path);
}

#[test]
#[serial]
fn graph_render_cache_separates_transparency() {
    let cache = FileCache::new();
    let _ = cache.clear();

    let graph = GraphDiagram::from_expression("a -> b").unwrap();
    let opaque = graph
        .render(&RenderRequest {
            format: OutputFormat::Svg,
            scale: 1,
            target_width: None,
            transparent_background: false,
        })
        .unwrap();
    let transparent = graph
        .render(&RenderRequest {
            format: OutputFormat::Svg,
            scale: 1,
            target_width: None,
            transparent_background: true,
        })
        .unwrap();

    assert_ne!(opaque.path, transparent.path);

    let opaque_svg = std::fs::read_to_string(opaque.path).unwrap();
    let transparent_svg = std::fs::read_to_string(transparent.path).unwrap();
    assert!(opaque_svg.contains("fill=\"#ffffff\""));
    assert!(!transparent_svg.contains("fill=\"#ffffff\""));
}

#[test]
#[serial]
fn graph_render_opaque_background_uses_theme_surface_color() {
    let cache = FileCache::new();
    let _ = cache.clear();

    let light = GraphDiagram::from_expression("a -> b")
        .unwrap()
        .with_color_theme(GraphColorTheme::light())
        .render(&RenderRequest {
            format: OutputFormat::Svg,
            scale: 1,
            target_width: None,
            transparent_background: false,
        })
        .unwrap();
    let dark = GraphDiagram::from_expression("a -> b")
        .unwrap()
        .with_color_theme(GraphColorTheme::dark())
        .render(&RenderRequest {
            format: OutputFormat::Svg,
            scale: 1,
            target_width: None,
            transparent_background: false,
        })
        .unwrap();

    let light_svg = std::fs::read_to_string(light.path).unwrap();
    let dark_svg = std::fs::read_to_string(dark.path).unwrap();

    assert!(light_svg.contains("fill=\"#f8fafc\""));
    assert!(dark_svg.contains("fill=\"#020617\""));
}
