pub mod file_cache;
pub use file_cache::FileCache;

/// The kind of visualization being cached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationKind {
    Mermaid,
    Graph,
}

impl VisualizationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::Graph => "graph",
        }
    }
}
