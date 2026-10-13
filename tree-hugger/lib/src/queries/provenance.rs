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
            "the `if let`/`while let` pattern capture lists the pattern kinds whose identifier children bind (captured, `mut`, or, `ref`, reference, slice, tuple and tuple-struct patterns); upstream's any-pattern form also declared range bounds (`1..=MAX`), path segments (`E::A`) and generic-pattern paths",
            "the import patterns capture only the name each import binds: a bare `use a;`, a list entry's own path's last segment (`h` in `use a::{g::h}`), the list path's last segment for a `self` entry, an `extern crate`'s alias when it has one, and a glob's path's last segment as an on-demand import; upstream missed the first three and a glob, and captured the crate name of `extern crate y as z`",
        ],
        ProgrammingLanguage::Java => &[
            "import declarations capture both qualified-path final identifiers and direct identifiers (ordinary/static on-demand paths), without capturing qualified names outside imports",
            "classes captured as `@local.definition.class`; added `interface_declaration` and interface-constant (`constant_declaration`) patterns",
            "`package_declaration` also matches a dotted `scoped_identifier` name",
            "added explicit constructors (`constructor_declaration`) as `@local.definition.constructor` and enum constants as `@local.definition.enum_variant`, each with a context capture",
            "the import patterns capture only the final segment of an `import_declaration`'s own path, including its direct `identifier` alternative; upstream captured every identifier under any `scoped_identifier` behind a `#has-ancestor?` the runtime does not enforce, importing qualified annotation names, module names and module-directive operands",
        ],
        ProgrammingLanguage::Python => &[
            "classes captured as `@local.definition.class`",
            "removed the attribute-assignment (`self.x = …`, `obj.attr = …`) field pattern",
            "default parameters (`default_parameter`, `typed_default_parameter`) and the walrus operator (`named_expression`) capture only their `name:` field; upstream captured every identifier child, declaring a bare default value (`x=external`) or assigned value (`y := external`)",
            "class-body field patterns capture names inside a `pattern_list`, `tuple_pattern` or `list_pattern` target only; upstream's any-target form declared the object and subscript names of `obj.attr = …` and `d[k] = …`",
            "`import a.b` captures only its first segment `a` (the name it binds), not every segment; added `from __future__ import` names and on-demand `from m import *` (named by the module's last segment or relative prefix)",
        ],
        ProgrammingLanguage::JavaScript => &[
            "removed the `this.x = …` assignment pattern",
            "the shared ecma import patterns capture a default import and a specifier's alias, else its name; upstream captured both names of `{ a as b }` (importing the unbound original) and missed default imports",
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
            "`struct`/`union` names are definitions only with a body or written alone (`struct S;`, also as a member); upstream captured every elaborated use (`struct S x;`, a parameter `struct S *p`, `typedef struct S T;`) as a type record",
        ],
        ProgrammingLanguage::Cpp => &[
            "classes captured as `@local.definition.class`; `using` aliases as `@local.definition.type_alias`",
            "replaced the class-body method patterns with one member-function pattern under any return declarator (`int *m(int);`, `int (*m(int))(long) {}`), its context supplied by Tree Hugger; added a qualified function-name pattern behind an attribute or parentheses (`int S::f [[deprecated]] (int) {}`)",
            "added reference-declarator field and typedef-name patterns (`int &ref;`, `typedef int &Ref;`)",
            "`template_function`/`template_method` names are definitions only inside a function declarator (an explicit specialization, bare or qualified); upstream also captured every use (`f<int>()`), turning calls into function records",
            "`class` and qualified `struct` names are definitions only with a body or written alone (`class C;`, also in a namespace, class, template or linkage body); upstream captured every elaborated use (`class C *c;`, a parameter `struct S &s`) as a class or type record",
        ],
        ProgrammingLanguage::CSharp => &[
            "properties captured as `@local.definition.property`",
            "added block-scoped and file-scoped namespace patterns",
            "added explicit instance constructors (a `static` one excluded) as `@local.definition.constructor` and enum members as `@local.definition.enum_variant`, each with a context capture",
            "parameters capture only their `name:` field; upstream captured every identifier child, declaring a named type (`W x`) or a bare default value (`int y = external`)",
            "`using_directive` patterns capture an alias directive's `name:` and otherwise the final segment of the directive's path (identifier, generic, qualified or `global::`-qualified); the earlier form captured the whole path and every identifier child, importing an alias target (`F` in `using E = F;`) and, through the reader, dropping every qualified directive",
        ],
        ProgrammingLanguage::Php => &[
            "added a `const_declaration` pattern (class, interface, trait, enum and top-level constants) in the inherited `php_only` query; it captures only the first `name` of a `const_element`, never a constant read as its value (`const B = A;`)",
            "`simple_parameter` and `property_element` capture only their `name:` variable, never a variable written as the default value, in the inherited `php_only` query",
            "added enum cases (`enum_case`) as `@local.definition.enum_variant` and named namespaces (`namespace_definition`) as `@local.definition.namespace`, each with a context capture, in the inherited `php_only` query",
            "the `namespace_use_clause` patterns capture the alias when one is written, else the clause's last segment, including a one-segment `use A;` and the clauses of a group (`use A\\{B, C as D};`); upstream captured only a qualified path's last segment, so it missed both and reported an aliased path by its original name",
        ],
        ProgrammingLanguage::Go => &[
            "added type-alias patterns (`type A = B`, alone or grouped) as `@local.definition.type_alias`, and named fields of a named struct type as `@local.definition.field`",
            "a blank import's `_` is captured as its import name (the path beside it is skipped); upstream captured only the path, reporting `_ \"c/d\"` as an import named `d`",
            "import paths are captured as interpreted or raw strings (`` `c/d` ``); upstream captured only interpreted ones",
        ],
        ProgrammingLanguage::Scala => &[
            "added Scala 3 enum cases (`simple_enum_case`, `full_enum_case`) as `@local.definition.enum_variant` and concrete `type` definitions as `@local.definition.type_alias`, each with a context capture",
            "added grouped `val`/`var` definition names (`pattern: (identifiers (identifier))`, as in `val a, b: Int = 1`) as `@local.definition.var`, one capture per name",
            "the import patterns capture identifier and operator_identifier alternatives for braced selectors, braced/unbraced rename aliases, and every flat path segment of the declaration as a candidate; `TreeFile::imported_symbols` keeps only the segment an import binds (a path's last segment before `,` or at the end, and an on-demand import's (`a.b.*`, `a.b._`, `a.given`, a selector wildcard) path's last segment), stepping over comments a `.` anchor cannot; upstream captured every identifier of the declaration, importing every qualifier and missing every selector",
        ],
        ProgrammingLanguage::TypeScript => &[
            "inherits the ecma import patterns (see JavaScript); added `import q = require(\"m\")` and `import R = N.S` bindings as `@local.definition.import` (an entity alias's name is the identifier before `=`, unanchored, so a comment before it never hides it)",
        ],
        ProgrammingLanguage::Lua
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
        license: REPOSITORY_LICENSE.to_string(),
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
    fn every_record_carries_the_license_of_its_origin() {
        let all = all_query_provenance();
        let (originals, vendored): (Vec<_>, Vec<_>) = all
            .iter()
            .partition(|(_, prov)| prov.translation_status == TranslationStatus::Original);
        assert!(!originals.is_empty() && !vendored.is_empty());

        for ((language, kind), prov) in originals {
            assert_eq!(
                prov.license, "AGPL-3.0-only",
                "{language:?} {kind:?} is a Tree Hugger original"
            );
        }
        // A modified vendored query keeps its upstream license.
        for ((language, kind), prov) in vendored {
            assert_eq!(prov.source_project, "nvim-treesitter", "{language:?} {kind:?}");
            assert_eq!(prov.license, "Apache-2.0", "{language:?} {kind:?}");
        }
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
