use std::collections::HashMap;

use crate::queries::QueryKind;
use crate::shared::ProgrammingLanguage;

/// Tracks the origin and translation status of a vendored or overlay query.
///
/// Every query loaded by Tree Hugger carries provenance so consumers can
/// understand where captures come from, whether they have been adapted from
/// another runtime, and what license governs reuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryProvenance {
    /// Source project (e.g., "nvim-treesitter").
    pub source_project: String,
    /// Upstream revision (tag, commit SHA, or version) when the query was
    /// vendored.
    pub upstream_revision: String,
    /// Original path in the upstream repository.
    pub upstream_path: String,
    /// Local path relative to the tree-hugger crate root.
    pub local_path: String,
    /// SPDX license identifier (e.g., "Apache-2.0").
    pub license: String,
    /// Translation status from upstream dialect to Tree Hugger.
    pub translation_status: TranslationStatus,
    /// Human-readable notes about adaptations made.
    pub notes: Vec<String>,
}

/// Describes how an upstream query was adapted for Tree Hugger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationStatus {
    /// No changes were needed; the query compiles as-is.
    Unchanged,
    /// Predicates or directives were adapted to tree-sitter Rust equivalents.
    PredicatesAdapted,
    /// Captures were renamed to match Tree Hugger conventions.
    CapturesRenamed,
    /// Both predicates and captures were modified.
    MultipleChanges,
    /// The query was written from scratch; no upstream source.
    Original,
}

impl std::fmt::Display for TranslationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unchanged => write!(f, "unchanged"),
            Self::PredicatesAdapted => write!(f, "predicates-adapted"),
            Self::CapturesRenamed => write!(f, "captures-renamed"),
            Self::MultipleChanges => write!(f, "multiple-changes"),
            Self::Original => write!(f, "original"),
        }
    }
}

/// Returns the provenance record for a vendored `locals.scm` query.
///
/// All vendor queries in Tree Hugger originate from nvim-treesitter.
pub fn vendor_query_provenance(
    language: ProgrammingLanguage,
    kind: QueryKind,
) -> Option<QueryProvenance> {
    let query_name = language.query_name();
    let (upstream_path, local_path) = match kind {
        QueryKind::Locals => (
            format!("queries/{query_name}/locals.scm"),
            format!("queries/vendor/{query_name}/locals.scm"),
        ),
        _ => return None,
    };

    Some(QueryProvenance {
        source_project: "nvim-treesitter".to_string(),
        upstream_revision: "0.10.4".to_string(),
        upstream_path,
        local_path,
        license: "Apache-2.0".to_string(),
        translation_status: TranslationStatus::CapturesRenamed,
        notes: vec![
            "@definition.* renamed to @local.definition.*".to_string(),
            "@reference.* renamed to @local.reference.*".to_string(),
        ],
    })
}

/// Returns the provenance record for a local overlay query.
pub fn overlay_query_provenance(
    language: ProgrammingLanguage,
    kind: QueryKind,
) -> Option<QueryProvenance> {
    let query_name = language.query_name();
    let local_path = match kind {
        QueryKind::Locals => format!("queries/{query_name}/locals.scm"),
        QueryKind::Lint => format!("queries/{query_name}/lint.scm"),
        QueryKind::References => format!("queries/{query_name}/references.scm"),
        QueryKind::Comments => format!("queries/{query_name}/comments.scm"),
        _ => return None,
    };

    Some(QueryProvenance {
        source_project: "tree-hugger".to_string(),
        upstream_revision: env!("CARGO_PKG_VERSION").to_string(),
        upstream_path: local_path.clone(),
        local_path,
        license: "MIT".to_string(),
        translation_status: TranslationStatus::Original,
        notes: Vec::new(),
    })
}

/// Looks up provenance for any query that tree-hugger can load.
pub fn query_provenance(language: ProgrammingLanguage, kind: QueryKind) -> Option<QueryProvenance> {
    match kind {
        QueryKind::Locals => vendor_query_provenance(language, kind)
            .or_else(|| overlay_query_provenance(language, kind)),
        QueryKind::Lint | QueryKind::References | QueryKind::Comments => {
            overlay_query_provenance(language, kind)
        }
        _ => None,
    }
}

/// Collects provenance for every supported language and query kind.
pub fn all_query_provenance() -> HashMap<(ProgrammingLanguage, QueryKind), QueryProvenance> {
    let mut map = HashMap::new();
    for language in ProgrammingLanguage::all() {
        for kind in [
            QueryKind::Locals,
            QueryKind::Lint,
            QueryKind::References,
            QueryKind::Comments,
        ] {
            if let Some(prov) = query_provenance(language, kind) {
                map.insert((language, kind), prov);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_provenance_has_expected_fields() {
        let prov = vendor_query_provenance(ProgrammingLanguage::Rust, QueryKind::Locals)
            .expect("rust locals should have vendor provenance");
        assert_eq!(prov.source_project, "nvim-treesitter");
        assert_eq!(prov.license, "Apache-2.0");
        assert!(!prov.upstream_path.is_empty());
        assert!(!prov.local_path.is_empty());
    }

    #[test]
    fn overlay_provenance_is_original() {
        let prov = overlay_query_provenance(ProgrammingLanguage::Rust, QueryKind::Lint)
            .expect("rust lint should have overlay provenance");
        assert_eq!(prov.source_project, "tree-hugger");
        assert_eq!(prov.translation_status, TranslationStatus::Original);
    }

    #[test]
    fn non_local_kinds_return_none_for_vendor() {
        assert!(vendor_query_provenance(ProgrammingLanguage::Rust, QueryKind::Lint).is_none());
        assert!(
            vendor_query_provenance(ProgrammingLanguage::Rust, QueryKind::References).is_none()
        );
    }

    #[test]
    fn all_provenance_includes_rust_locals() {
        let all = all_query_provenance();
        assert!(all.contains_key(&(ProgrammingLanguage::Rust, QueryKind::Locals)));
    }

    #[test]
    fn translation_status_display() {
        assert_eq!(TranslationStatus::Unchanged.to_string(), "unchanged");
        assert_eq!(
            TranslationStatus::PredicatesAdapted.to_string(),
            "predicates-adapted"
        );
        assert_eq!(TranslationStatus::Original.to_string(), "original");
    }
}
