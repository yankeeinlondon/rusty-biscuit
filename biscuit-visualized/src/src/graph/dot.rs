use super::{error::GraphError, expression::GraphExpression};
use std::collections::BTreeSet;

use layout::gv::parser::ast;

fn parse_dot_ast(dot_source: &str) -> Result<ast::Graph, GraphError> {
    let mut parser = layout::gv::DotParser::new(dot_source);
    parser
        .process()
        .map_err(|e| GraphError::DotParseFailed(e.to_string()))
}

/// Renders a DOT graph source to SVG format.
///
/// Uses the `layout-rs` library to parse DOT, apply layout algorithms,
/// and render to SVG.
///
/// ## Arguments
///
/// * `dot_source` - DOT format graph definition
///
/// ## Returns
///
/// SVG string representation of the rendered graph.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_visualized::graph::dot::render_dot_to_svg;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dot = r#"
///     digraph G {
///         A -> B;
///         B -> C;
///     }
/// "#;
/// let svg = render_dot_to_svg(dot)?;
/// assert!(svg.contains("<svg"));
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// Returns `GraphError::DotParseFailed` if the DOT source is invalid or cannot be parsed.
/// Returns `GraphError::RenderFailed` if the rendering operation fails.
pub fn render_dot_to_svg(dot_source: &str) -> Result<String, GraphError> {
    use layout::backends::svg::SVGWriter;

    let graph = parse_dot_ast(dot_source)?;

    // Build visual graph - GraphBuilder handles parsing and creating the VisualGraph
    let mut builder = layout::gv::GraphBuilder::new();
    builder.visit_graph(&graph);

    // Get the complete visual graph from the builder
    let mut vg = builder.get();

    // Apply layout and render to SVG
    let mut svg_writer = SVGWriter::new();
    vg.do_it(false, false, false, &mut svg_writer);

    // Get SVG output
    let svg = svg_writer.finalize();

    Ok(svg)
}

/// Validates DOT source for unsupported features.
///
/// Currently checks for:
/// - HTML labels containing `<TABLE>` tags
/// - HTML tags in labels (indicated by `<` character)
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::graph::dot::validate_dot;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// validate_dot("digraph { A -> B; }")?; // OK
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// Returns `GraphError::UnsupportedDotFeature` if unsupported constructs are detected.
pub fn validate_dot(source: &str) -> Result<(), GraphError> {
    // Check for HTML table labels
    if source.contains("<TABLE") || source.contains("<table") {
        return Err(GraphError::UnsupportedDotFeature(
            "HTML table labels are not supported".to_string(),
        ));
    }

    // Check for HTML tags in labels (basic heuristic)
    if source.contains("label=<") || source.contains("label = <") {
        return Err(GraphError::UnsupportedDotFeature(
            "HTML labels are not supported".to_string(),
        ));
    }

    let graph = parse_dot_ast(source)?;
    validate_subgraphs(&graph, false)?;

    Ok(())
}

fn validate_subgraphs(graph: &ast::Graph, inside_subgraph: bool) -> Result<(), GraphError> {
    for stmt in &graph.list.list {
        if let ast::Stmt::SubGraph(subgraph) = stmt {
            if inside_subgraph {
                return Err(GraphError::UnsupportedDotFeature(
                    "Nested subgraphs/clusters are not supported".to_string(),
                ));
            }

            validate_subgraphs(subgraph, true)?;
        }
    }

    Ok(())
}

pub(crate) fn dot_node_count(source: &str) -> Result<usize, GraphError> {
    let graph = parse_dot_ast(source)?;
    let mut names = BTreeSet::new();
    collect_node_names(&graph, &mut names);
    Ok(names.len())
}

fn collect_node_names(graph: &ast::Graph, names: &mut BTreeSet<String>) {
    for stmt in &graph.list.list {
        match stmt {
            ast::Stmt::Node(node) => {
                names.insert(node.id.name.clone());
            }
            ast::Stmt::Edge(edge) => {
                names.insert(edge.from.name.clone());
                for (node, _) in &edge.to {
                    names.insert(node.name.clone());
                }
            }
            ast::Stmt::SubGraph(subgraph) => collect_node_names(subgraph, names),
            ast::Stmt::Attribute(_) => {}
        }
    }
}

/// Converts a graph expression to DOT format.
///
/// ## Arguments
///
/// * `expr` - The parsed graph expression
/// * `directed` - Whether to generate a directed or undirected graph
///
/// ## Returns
///
/// DOT format string representation of the graph.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::graph::{GraphExpression, dot::expression_to_dot};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let expr = GraphExpression::parse("a -> b -> c")?;
/// let dot = expression_to_dot(&expr, true);
/// assert!(dot.contains("digraph"));
/// assert!(dot.contains("a -> b"));
/// # Ok(())
/// # }
/// ```
pub fn expression_to_dot(expr: &GraphExpression, directed: bool) -> String {
    let graph_type = if directed { "digraph" } else { "graph" };
    let edge_op = if directed { "->" } else { "--" };

    let mut dot = format!("{} G {{\n", graph_type);

    // Add nodes (only if they don't appear in edges, to avoid duplication)
    let nodes_in_edges: std::collections::HashSet<_> = expr
        .edges
        .iter()
        .flat_map(|e| vec![e.from.clone(), e.to.clone()])
        .collect();

    for node in &expr.nodes {
        if !nodes_in_edges.contains(node) {
            dot.push_str(&format!("    \"{}\";\n", node.replace('"', "\\\"")));
        }
    }

    // Add edges
    for edge in &expr.edges {
        let from = edge.from.replace('"', "\\\"");
        let to = edge.to.replace('"', "\\\"");
        dot.push_str(&format!("    \"{}\" {} \"{}\";\n", from, edge_op, to));
    }

    dot.push_str("}\n");
    dot
}
