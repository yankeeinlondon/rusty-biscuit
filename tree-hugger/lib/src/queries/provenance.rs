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
/// Every vendored query except Perl's originates from nvim-treesitter; its
/// translation status and notes record Tree Hugger's changes per language.
/// Perl's query lives under `vendor/` but was written by Tree Hugger, since
/// nvim-treesitter has no Perl locals query.
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

    if language == ProgrammingLanguage::Perl {
        return Some(QueryProvenance {
            source_project: "tree-hugger".to_string(),
            upstream_revision: env!("CARGO_PKG_VERSION").to_string(),
            upstream_path: local_path.clone(),
            local_path,
            license: REPOSITORY_LICENSE.to_string(),
            translation_status: TranslationStatus::Original,
            notes: vec![
                "written for tree-sitter-perl 1.1.2; nvim-treesitter has no Perl locals query"
                    .to_string(),
            ],
        });
    }

    let changes = vendored_locals_changes(language);
    let mut notes = vec![
        "@definition.* renamed to @local.definition.*".to_string(),
        "@reference.* renamed to @local.reference.*".to_string(),
    ];
    notes.extend(changes.iter().map(|change| (*change).to_string()));

    Some(QueryProvenance {
        source_project: "nvim-treesitter".to_string(),
        upstream_revision: "0.10.4".to_string(),
        upstream_path,
        local_path,
        license: "Apache-2.0".to_string(),
        translation_status: if changes.is_empty() {
            TranslationStatus::CapturesRenamed
        } else {
            TranslationStatus::MultipleChanges
        },
        notes,
    })
}

/// The license of queries Tree Hugger writes itself, which is the
/// repository's license.
const REPOSITORY_LICENSE: &str = "AGPL-3.0-only";

/// Tree Hugger's changes to a vendored `locals.scm` beyond renaming captures:
/// added or removed patterns and reclassified captures. Empty when renaming
/// is the only change recorded.
fn vendored_locals_changes(language: ProgrammingLanguage) -> &'static [&'static str] {
    match language {
        ProgrammingLanguage::Rust => &[
            "added bodyless trait method (`function_signature_item`), `static`, `union`, `type` and associated `type` patterns",
            "enum variants captured as `@local.definition.enum_variant`; `mod` items as `@local.definition.module` with a context capture",
        ],
        ProgrammingLanguage::Java => &[
            "classes captured as `@local.definition.class`; added `interface_declaration` and interface-constant (`constant_declaration`) patterns",
            "`package_declaration` also matches a dotted `scoped_identifier` name",
            "added explicit constructors (`constructor_declaration`) as `@local.definition.constructor` and enum constants as `@local.definition.enum_variant`, each with a context capture",
        ],
        ProgrammingLanguage::Python => &[
            "classes captured as `@local.definition.class`",
            "removed the attribute-assignment (`self.x = …`, `obj.attr = …`) field pattern",
        ],
        ProgrammingLanguage::JavaScript => &[
            "removed the `this.x = …` assignment pattern",
        ],
        ProgrammingLanguage::Swift => &[
            "enum cases captured as `@local.definition.enum_variant`",
            "added protocol function and property requirement patterns",
            "added initializers (`init`, `init?`, `init!`, named by the `init` keyword) as `@local.definition.constructor` and `typealias` declarations as `@local.definition.type_alias`, each with a context capture",
        ],
        ProgrammingLanguage::C => &[
            "added `union_specifier` and field-name patterns under any pointer, array or parenthesized declarator (`**p`, `a[3]`, `(*callback)(int)`), replacing the field-level pointer pattern",
            "typedefs captured as `@local.definition.type_alias`; enumerators as `@local.definition.enum_variant` with a context capture (C++ inherits both)",
            "added name patterns under an attributed declarator (`int x [[maybe_unused]];`) and variable names under a parenthesized declarator; a name whose declarator declares a function (`int f [[deprecated]] (int);`, `int (f)(int);`) is reclassified as a function",
            "added typedef-name patterns under pointer, array, function, parenthesized and attributed declarators (`typedef int *P;`, `typedef int (*Fn)(int);`)",
        ],
        ProgrammingLanguage::Cpp => &[
            "classes captured as `@local.definition.class`; `using` aliases as `@local.definition.type_alias`",
            "replaced the class-body method patterns with one member-function pattern under any return declarator (`int *m(int);`, `int (*m(int))(long) {}`), its context supplied by Tree Hugger; added a qualified function-name pattern behind an attribute or parentheses (`int S::f [[deprecated]] (int) {}`)",
            "added reference-declarator field and typedef-name patterns (`int &ref;`, `typedef int &Ref;`)",
        ],
        ProgrammingLanguage::CSharp => &[
            "properties captured as `@local.definition.property`",
            "added block-scoped and file-scoped namespace patterns",
            "added explicit instance constructors (a `static` one excluded) as `@local.definition.constructor` and enum members as `@local.definition.enum_variant`, each with a context capture",
        ],
        ProgrammingLanguage::Php => &[
            "added a `const_declaration` pattern (class, interface, trait, enum and top-level constants) in the inherited `php_only` query",
            "added enum cases (`enum_case`) as `@local.definition.enum_variant` and named namespaces (`namespace_definition`) as `@local.definition.namespace`, each with a context capture, in the inherited `php_only` query",
        ],
        ProgrammingLanguage::Go => &[
            "added type-alias patterns (`type A = B`, alone or grouped) as `@local.definition.type_alias`, and named fields of a named struct type as `@local.definition.field`",
        ],
        ProgrammingLanguage::Scala => &[
            "added Scala 3 enum cases (`simple_enum_case`, `full_enum_case`) as `@local.definition.enum_variant` and concrete `type` definitions as `@local.definition.type_alias`, each with a context capture",
        ],
        ProgrammingLanguage::TypeScript
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => &[],
    }
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
    fn perl_locals_are_a_tree_hugger_original() {
        let prov = query_provenance(ProgrammingLanguage::Perl, QueryKind::Locals)
            .expect("perl locals provenance");
        assert_eq!(prov.source_project, "tree-hugger");
        assert_eq!(prov.license, REPOSITORY_LICENSE);
        assert_eq!(prov.translation_status, TranslationStatus::Original);
        assert_eq!(prov.local_path, "queries/vendor/perl/locals.scm");
    }

    #[test]
    fn changed_vendor_queries_record_their_changes() {
        for language in [
            ProgrammingLanguage::Rust,
            ProgrammingLanguage::Java,
            ProgrammingLanguage::Python,
            ProgrammingLanguage::JavaScript,
            ProgrammingLanguage::Swift,
            ProgrammingLanguage::C,
            ProgrammingLanguage::Cpp,
            ProgrammingLanguage::CSharp,
            ProgrammingLanguage::Php,
            ProgrammingLanguage::Go,
            ProgrammingLanguage::Scala,
        ] {
            let prov = vendor_query_provenance(language, QueryKind::Locals).expect("provenance");
            assert_eq!(
                prov.translation_status,
                TranslationStatus::MultipleChanges,
                "{language}"
            );
            assert!(prov.notes.len() > 2, "{language}: {:?}", prov.notes);
        }

        let unchanged = vendor_query_provenance(ProgrammingLanguage::Lua, QueryKind::Locals)
            .expect("lua provenance");
        assert_eq!(unchanged.translation_status, TranslationStatus::CapturesRenamed);
        assert_eq!(unchanged.source_project, "nvim-treesitter");
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
