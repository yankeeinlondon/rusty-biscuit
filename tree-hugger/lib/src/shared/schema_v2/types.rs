use serde::{Deserialize, Serialize};

/// Stable identifier for a symbol record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

/// Stable identifier for a type record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(pub String);

/// Explicit schema version for serialized records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
}

impl SchemaVersion {
    pub const V2_0: Self = Self { major: 2, minor: 0 };
}

/// 1-based text location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextPoint {
    pub line: u32,
    pub column: u32,
}

/// Text span including line/column and byte offsets.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextSpan {
    pub start: TextPoint,
    pub end: TextPoint,
    pub start_byte: u32,
    pub end_byte: u32,
}

/// Pipeline stage for symbol analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnalysisPass {
    Parse,
    Bind,
    Semantic,
    Docs,
}

impl AnalysisPass {
    /// Returns canonical lowercase label used in provenance.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Bind => "bind",
            Self::Semantic => "semantic",
            Self::Docs => "docs",
        }
    }
}
