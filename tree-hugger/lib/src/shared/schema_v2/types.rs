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
    /// Shape before symbol kinds, declaration visibility, static markers and
    /// container kinds were preserved; no longer emitted.
    pub const V2_0: Self = Self { major: 2, minor: 0 };
    /// Shape with record timestamps and the container named
    /// `identity.module_path`; no longer emitted.
    pub const V3_0: Self = Self { major: 3, minor: 0 };
    /// Shape with no timestamps, `identity.container_name`,
    /// `identity.keyword` and a filled `source.signature_text`, before
    /// `metrics.lines`; no longer emitted.
    pub const V4_0: Self = Self { major: 4, minor: 0 };
    /// Shape with the required `metrics.lines` key; no longer emitted.
    pub const V4_1: Self = Self { major: 4, minor: 1 };
    /// Shape with unclassified bases and optional relation source spans,
    /// before `metrics.cyclomatic`; no longer emitted.
    pub const V4_2: Self = Self { major: 4, minor: 2 };
    /// Shape with the required `metrics.cyclomatic` key, whose reference
    /// summaries bound the first same-named declaration; no longer emitted.
    pub const V4_3: Self = Self { major: 4, minor: 3 };
    /// Reference summaries and `semantics.is_recursive` are derived from
    /// scoped per-occurrence evidence, and a file index may carry
    /// `occurrences` and `reference_coverage`; no longer emitted.
    pub const V5_0: Self = Self { major: 5, minor: 0 };
    /// Shape with an occurrence's selected import name span; no longer
    /// emitted.
    pub const V5_1: Self = Self { major: 5, minor: 1 };
    /// Shape with the required `imports[].reexported` marker and the optional
    /// file-index `reexports` report.
    pub const V5_2: Self = Self { major: 5, minor: 2 };
    /// The shape records, file indexes, CLI envelopes and cache entries are
    /// written in, and the only one the cache serves.
    pub const CURRENT: Self = Self::V5_2;
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
///
/// Prerequisites run first: parse before bind, bind before semantic, parse
/// before docs and diagnostics. A pass appears in an index's
/// `completed_passes` only when it ran, so a missing `Diagnostics` is never
/// evidence that a file is clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnalysisPass {
    Parse,
    Bind,
    Semantic,
    Docs,
    /// Lint (pattern and semantic) and syntax diagnostics.
    Diagnostics,
}

impl AnalysisPass {
    /// Returns canonical lowercase label used in provenance.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Bind => "bind",
            Self::Semantic => "semantic",
            Self::Docs => "docs",
            Self::Diagnostics => "diagnostics",
        }
    }
}
