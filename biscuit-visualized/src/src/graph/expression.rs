use std::collections::HashSet;

use super::error::GraphError;

/// The type of edge in a graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeKind {
    Directed,
    Undirected,
}

/// A single edge in a graph expression.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

/// A parsed graph expression.
///
/// Contains the list of unique nodes and edges extracted from the expression syntax.
#[derive(Debug, Clone)]
pub struct GraphExpression {
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
}

impl GraphExpression {
    /// Parses a graph expression from a string.
    ///
    /// Supports a lightweight syntax for defining graphs:
    /// - Node identifiers: bare words (alphanumeric + underscore + hyphen) or `"quoted strings"`
    /// - Directed edge: `->`
    /// - Undirected edge: `--`
    /// - Chain support: `a -> b -> c` produces edges a->b and b->c
    /// - Statement separators: `;` or newline
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphExpression;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let expr = GraphExpression::parse("a -> b -> c")?;
    /// assert_eq!(expr.nodes.len(), 3);
    /// assert_eq!(expr.edges.len(), 2);
    ///
    /// let expr = GraphExpression::parse(r#""My App" -- Postgres"#)?;
    /// assert_eq!(expr.nodes[0], "My App");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `GraphError::ExpressionParseFailed` if the input is empty or contains
    /// invalid syntax.
    pub fn parse(input: &str) -> Result<Self, GraphError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(GraphError::ExpressionParseFailed("Empty input".to_string()));
        }

        let mut nodes = Vec::new();
        let mut seen_nodes = HashSet::new();
        let mut edges = Vec::new();

        // Split by semicolons and newlines into statements
        let statements = input
            .split([';', '\n'])
            .map(str::trim)
            .filter(|s| !s.is_empty());

        for statement in statements {
            let (stmt_nodes, stmt_edges) = Self::parse_statement(statement)?;

            // Add nodes (deduplicated)
            for node in stmt_nodes {
                if seen_nodes.insert(node.clone()) {
                    nodes.push(node);
                }
            }

            edges.extend(stmt_edges);
        }

        if nodes.is_empty() {
            return Err(GraphError::ExpressionParseFailed(
                "No nodes found in expression".to_string(),
            ));
        }

        let has_directed = edges.iter().any(|edge| edge.kind == EdgeKind::Directed);
        let has_undirected = edges.iter().any(|edge| edge.kind == EdgeKind::Undirected);
        if has_directed && has_undirected {
            return Err(GraphError::MixedEdgeKinds);
        }

        Ok(Self { nodes, edges })
    }

    fn parse_statement(statement: &str) -> Result<(Vec<String>, Vec<GraphEdge>), GraphError> {
        let tokens = Self::tokenize(statement)?;

        if tokens.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Identifier(id) => {
                    nodes.push(id.clone());

                    // Check for edge operator following this node
                    if i + 2 < tokens.len() {
                        let edge_kind = match &tokens[i + 1] {
                            Token::DirectedEdge => Some(EdgeKind::Directed),
                            Token::UndirectedEdge => Some(EdgeKind::Undirected),
                            _ => None,
                        };

                        if let Some(kind) = edge_kind {
                            if let Token::Identifier(to_id) = &tokens[i + 2] {
                                edges.push(GraphEdge {
                                    from: id.clone(),
                                    to: to_id.clone(),
                                    kind,
                                });
                                i += 2; // Skip edge operator and next node
                                continue;
                            }
                        }
                    }

                    i += 1;
                }
                _ => {
                    return Err(GraphError::ExpressionParseFailed(format!(
                        "Unexpected token: {:?}",
                        tokens[i]
                    )));
                }
            }
        }

        Ok((nodes, edges))
    }

    fn tokenize(input: &str) -> Result<Vec<Token>, GraphError> {
        let mut tokens = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(&ch) = chars.peek() {
            match ch {
                ' ' | '\t' => {
                    chars.next();
                }
                '"' => {
                    chars.next(); // consume opening quote
                    let mut id = String::new();
                    let mut found_closing = false;

                    for ch in chars.by_ref() {
                        if ch == '"' {
                            found_closing = true;
                            break;
                        }
                        id.push(ch);
                    }

                    if !found_closing {
                        return Err(GraphError::ExpressionParseFailed(
                            "Unterminated quoted identifier".to_string(),
                        ));
                    }

                    tokens.push(Token::Identifier(id));
                }
                '-' => {
                    chars.next();
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        tokens.push(Token::DirectedEdge);
                    } else if chars.peek() == Some(&'-') {
                        chars.next();
                        tokens.push(Token::UndirectedEdge);
                    } else {
                        // Hyphen is part of identifier
                        let mut id = String::from("-");
                        while let Some(&ch) = chars.peek() {
                            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                                id.push(ch);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        tokens.push(Token::Identifier(id));
                    }
                }
                _ if ch.is_alphanumeric() || ch == '_' => {
                    let mut id = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                            id.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::Identifier(id));
                }
                _ => {
                    return Err(GraphError::ExpressionParseFailed(format!(
                        "Unexpected character: '{}'",
                        ch
                    )));
                }
            }
        }

        Ok(tokens)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Identifier(String),
    DirectedEdge,
    UndirectedEdge,
}
