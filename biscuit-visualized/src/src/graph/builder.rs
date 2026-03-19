use super::render::GraphDiagram;

/// A builder for constructing graphs programmatically.
///
/// Provides a fluent API for adding nodes and edges, then generates
/// a `GraphDiagram` that can be rendered.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::graph::GraphBuilder;
///
/// let graph = GraphBuilder::directed()
///     .add_node("app", Some("My App".to_string()))
///     .add_node("db", Some("PostgreSQL".to_string()))
///     .add_edge("app", "db")
///     .build();
/// ```
pub struct GraphBuilder {
    directed: bool,
    nodes: Vec<(String, Option<String>)>,
    edges: Vec<(String, String)>,
}

impl GraphBuilder {
    /// Creates a new builder for a directed graph.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphBuilder;
    ///
    /// let builder = GraphBuilder::directed();
    /// ```
    pub fn directed() -> Self {
        Self {
            directed: true,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Creates a new builder for an undirected graph.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphBuilder;
    ///
    /// let builder = GraphBuilder::undirected();
    /// ```
    pub fn undirected() -> Self {
        Self {
            directed: false,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds a node to the graph.
    ///
    /// ## Arguments
    ///
    /// * `id` - Unique identifier for the node
    /// * `label` - Optional display label (defaults to the id if None)
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphBuilder;
    ///
    /// let builder = GraphBuilder::directed()
    ///     .add_node("a", None)
    ///     .add_node("b", Some("Node B".to_string()));
    /// ```
    pub fn add_node(mut self, id: impl Into<String>, label: Option<String>) -> Self {
        self.nodes.push((id.into(), label));
        self
    }

    /// Adds an edge to the graph.
    ///
    /// The edge direction is determined by whether this is a directed or
    /// undirected graph builder.
    ///
    /// ## Arguments
    ///
    /// * `from` - Source node identifier
    /// * `to` - Target node identifier
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphBuilder;
    ///
    /// let builder = GraphBuilder::directed()
    ///     .add_edge("a", "b")
    ///     .add_edge("b", "c");
    /// ```
    pub fn add_edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push((from.into(), to.into()));
        self
    }

    /// Builds the graph into a `GraphDiagram`.
    ///
    /// Generates DOT format internally and creates a diagram that can be rendered.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphBuilder;
    ///
    /// let graph = GraphBuilder::directed()
    ///     .add_node("a", None)
    ///     .add_edge("a", "b")
    ///     .build();
    /// ```
    pub fn build(self) -> GraphDiagram {
        let dot_source = self.to_dot();
        // Safe to unwrap because we're generating valid DOT
        GraphDiagram::from_dot(dot_source).expect("Generated DOT should be valid")
    }

    fn to_dot(&self) -> String {
        let graph_type = if self.directed { "digraph" } else { "graph" };
        let edge_op = if self.directed { "->" } else { "--" };

        let mut dot = format!("{} G {{\n", graph_type);

        // Add nodes with labels
        for (id, label) in &self.nodes {
            let escaped_id = id.replace('"', "\\\"");
            if let Some(label) = label {
                let escaped_label = label.replace('"', "\\\"");
                dot.push_str(&format!(
                    "    \"{}\" [label=\"{}\"];\n",
                    escaped_id, escaped_label
                ));
            } else {
                dot.push_str(&format!("    \"{}\";\n", escaped_id));
            }
        }

        // Add edges
        for (from, to) in &self.edges {
            let escaped_from = from.replace('"', "\\\"");
            let escaped_to = to.replace('"', "\\\"");
            dot.push_str(&format!(
                "    \"{}\" {} \"{}\";\n",
                escaped_from, edge_op, escaped_to
            ));
        }

        dot.push_str("}\n");
        dot
    }
}
