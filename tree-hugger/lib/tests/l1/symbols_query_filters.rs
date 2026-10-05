//! `SymbolQuery` predicates and naming inputs
//! (`2026-10-03-symbol-filters-and-reports`, Phase 4): what each predicate
//! pushes into the plan, that predicates and prefixes are applied after the
//! cache and never reach an entry or an ID, how a wide result narrows to a
//! filtered query, and the naming-context and scan-root prefixes.
//!
//! The per-language behavior of every predicate is the selection-filter
//! matrix in `symbols_selection_filters`. Counter tests read process-wide
//! counts; nextest runs each test in its own process.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;
use tree_hugger::cache::{CacheLookup, PersistentCache, SymbolCache};
use tree_hugger::{
    Capability, NamePattern, NamingContext, SourceFile, SymbolKindV2, SymbolQuery,
    SymbolQueryResult, SymbolScope, VisibilityLevel, work_counters,
};

use crate::symbols_fixtures::{display_name, multi_fixtures};
use crate::symbols_kinds::deferred_kind_sources;

const FUNCTION_KINDS: [SymbolKindV2; 3] = [
    SymbolKindV2::Function,
    SymbolKindV2::Method,
    SymbolKindV2::Constructor,
];

fn write(dir: &Path, relative: &str, source: &str) -> PathBuf {
    let path = dir.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create dirs");
    std::fs::write(&path, source).expect("write source");
    path
}

fn query(path: &Path, query: &SymbolQuery) -> SymbolQueryResult {
    SourceFile::read(path).expect("read").query(query).expect("query")
}

fn names(result: &SymbolQueryResult) -> BTreeSet<String> {
    result.index.symbols.iter().map(|record| record.identity.qualified_name.clone()).collect()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// `(label, query, selected kinds, required capabilities)`.
type PushdownCase = (&'static str, SymbolQuery, Option<Vec<SymbolKindV2>>, Vec<Capability>);

#[test]
fn predicates_push_down_only_the_kinds_and_capabilities_they_read() {
    let declarations = [Capability::Declarations];
    let cases: [PushdownCase; 10] = [
        ("no predicate", SymbolQuery::new(), None, declarations.to_vec()),
        ("scope", SymbolQuery::new().scopes([SymbolScope::Item]), None, declarations.to_vec()),
        ("visibility", SymbolQuery::new().visibility(VisibilityLevel::Public), None, declarations.to_vec()),
        ("exclude tests", SymbolQuery::new().exclude_tests(true), None, vec![Capability::Declarations, Capability::Flags]),
        ("deprecated", SymbolQuery::new().deprecated(true), None, vec![Capability::Declarations, Capability::Flags]),
        ("undocumented", SymbolQuery::new().undocumented(true), None, vec![Capability::Declarations, Capability::Metrics]),
        ("async", SymbolQuery::new().is_async(true), Some(FUNCTION_KINDS.to_vec()), declarations.to_vec()),
        ("returns", SymbolQuery::new().returns("*"), Some(FUNCTION_KINDS.to_vec()), declarations.to_vec()),
        ("param type", SymbolQuery::new().param_type("*"), Some(FUNCTION_KINDS.to_vec()), declarations.to_vec()),
        ("min params", SymbolQuery::new().min_params(0), Some(FUNCTION_KINDS.to_vec()), vec![Capability::Declarations, Capability::Metrics]),
    ];
    for (label, query, kinds, capabilities) in cases {
        assert_eq!(query.selected_kinds(), kinds, "{label}: kinds");
        assert_eq!(query.required_capabilities(), capabilities, "{label}: capabilities");
    }

    // The narrowing holds whichever order the builder calls come in.
    let mixed = [SymbolKindV2::Function, SymbolKindV2::Class];
    assert_eq!(SymbolQuery::new().kinds(mixed).is_async(true).selected_kinds(), Some(vec![SymbolKindV2::Function]));
    assert_eq!(SymbolQuery::new().is_async(true).kinds(mixed).selected_kinds(), Some(vec![SymbolKindV2::Function]));
    assert_eq!(
        SymbolQuery::new().kinds([SymbolKindV2::Class]).returns("*").selected_kinds(),
        Some(vec![]),
        "a type-only selection with a shape predicate selects nothing"
    );
    assert_eq!(
        SymbolQuery::new().undocumented(true).capabilities([Capability::Containment]).required_capabilities(),
        [Capability::Declarations, Capability::Containment, Capability::Metrics],
        "replacing the capabilities keeps what a predicate reads"
    );
    // The last of the two test selections wins.
    let source = "#[test]\nfn checks() {}\nfn helper() {}\n";
    let dir = TempDir::new().expect("tempdir");
    let path = write(dir.path(), "lib.rs", source);
    let only = query(&path, &SymbolQuery::new().exclude_tests(true).tests_only(true));
    assert_eq!(names(&only), set(&["lib.rs#checks"]));
    let excluded = query(&path, &SymbolQuery::new().tests_only(true).exclude_tests(true));
    assert_eq!(names(&excluded), set(&["lib.rs#helper"]));
    let neither = query(&path, &SymbolQuery::new().tests_only(true).tests_only(false));
    assert_eq!(names(&neither), set(&["lib.rs#checks", "lib.rs#helper"]));
}

/// Filtering every `multi/` file by every metric and flag predicate runs one
/// definition query per file and never a diagnostics run.
#[test]
fn a_filtered_query_runs_no_diagnostics() {
    let files = multi_fixtures();
    let filtered = SymbolQuery::new()
        .undocumented(true)
        .exclude_tests(true)
        .min_params(1)
        .scopes([SymbolScope::Item, SymbolScope::Member]);
    let before = work_counters();
    for path in &files {
        let result = query(path, &filtered);
        assert_eq!(
            result.completed_capabilities,
            [Capability::Declarations, Capability::Flags, Capability::Metrics],
            "{}",
            display_name(path)
        );
    }
    let work = work_counters().since(before);
    assert_eq!(work.diagnostics_runs, 0);
    assert_eq!(work.definition_captures, files.len() as u64);
}

/// The entry a filtered, prefixed query writes holds every record of the
/// covered kinds under its in-file name; an unfiltered query from another
/// scan root is then a complete hit with the same IDs and its own prefix.
#[test]
fn predicates_and_prefixes_never_reach_the_cache_entry() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        dir.path(),
        "repo/src/shapes.rs",
        "pub struct Square;\nimpl Square {\n    pub fn area(&self) -> u32 { 4 }\n}\nfn helper() {}\n",
    );
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
    let filtered = SymbolQuery::new()
        .name(NamePattern::new("area"))
        .scan_root(dir.path().join("repo"));
    let first = cache.query(SourceFile::read(&path).expect("read"), &filtered).expect("cached query");
    assert!(matches!(first.lookup, CacheLookup::Miss(_)), "{:?}", first.lookup);
    assert_eq!(names(&first.result), set(&["src/shapes.rs#Square::area"]));
    assert_eq!(first.result.name_prefix(), "src/shapes.rs#");
    let uncached_filtered = query(&path, &filtered);
    assert_eq!(
        serde_json::to_value(&first.result.index).expect("serialize"),
        serde_json::to_value(&uncached_filtered.index).expect("serialize")
    );

    let entries: Vec<PathBuf> = walk_json(&dir.path().join("cache"));
    assert_eq!(entries.len(), 1, "{entries:#?}");
    let entry: Value = serde_json::from_slice(&std::fs::read(&entries[0]).expect("read entry")).expect("entry");
    let stored: BTreeSet<String> = entry["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|record| record["identity"]["qualified_name"].as_str().expect("name").to_string())
        .collect();
    assert_eq!(stored, set(&["Square", "Square::area", "Square::area::self", "helper"]), "unfiltered in-file names");

    let other_root = SymbolQuery::new().scan_root(dir.path().join("repo/src"));
    let second = cache.query(SourceFile::read(&path).expect("read"), &other_root).expect("cached query");
    assert_eq!(second.lookup, CacheLookup::Hit);
    assert_eq!(
        names(&second.result),
        set(&["shapes.rs#Square", "shapes.rs#Square::area", "shapes.rs#Square::area::self", "shapes.rs#helper"])
    );
    let area = |result: &SymbolQueryResult| {
        result.index.symbols.iter().find(|record| record.identity.name == "area").expect("area").id.clone()
    };
    assert_eq!(area(&first.result), area(&second.result), "the ID ignores the scan root");
}

fn walk_json(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).expect("read dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "json") {
                found.push(path);
            }
        }
    }
    found
}

/// Kinds are settled before kind selection: a narrow query sees the same
/// `Constructor` records a full query with the same capabilities does,
/// class expressions included, and `Method` excludes them. Parameters,
/// selected while their constructor is extracted light, match too.
#[test]
fn constructor_kind_is_settled_before_kind_selection() {
    let source = "export class Widget {\n  constructor(initialValue: number) {}\n  render(): void {}\n}\nexport const Anonymous = class {\n  constructor(seed: string) {}\n};\n";
    let dir = TempDir::new().expect("tempdir");
    let path = write(dir.path(), "widget.ts", source);
    let kinds_of = |result: &SymbolQueryResult| -> Vec<(String, SymbolKindV2)> {
        result.index.symbols.iter().map(|record| (record.identity.name.clone(), record.kind)).collect()
    };
    let capability_sets: [&[Capability]; 3] =
        [&[Capability::Declarations], &[Capability::Containment], &Capability::ALL];
    for capabilities in capability_sets {
        let full = query(&path, &SymbolQuery::new().capabilities(capabilities.iter().copied()));
        for kinds in [&[SymbolKindV2::Constructor][..], &[SymbolKindV2::Method], &[SymbolKindV2::Parameter]] {
            let narrow = query(
                &path,
                &SymbolQuery::new().kinds(kinds.iter().copied()).capabilities(capabilities.iter().copied()),
            );
            let expected: Vec<Value> = full
                .index
                .symbols
                .iter()
                .filter(|record| kinds.contains(&record.kind))
                .map(|record| serde_json::to_value(record).expect("serialize"))
                .collect();
            let actual: Vec<Value> = narrow
                .index
                .symbols
                .iter()
                .map(|record| serde_json::to_value(record).expect("serialize"))
                .collect();
            assert_eq!(actual, expected, "{kinds:?} with {capabilities:?}");
        }

        let constructors = query(
            &path,
            &SymbolQuery::new().kinds([SymbolKindV2::Constructor]).capabilities(capabilities.iter().copied()),
        );
        let constructor = (String::from("constructor"), SymbolKindV2::Constructor);
        assert_eq!(kinds_of(&constructors), [constructor.clone(), constructor], "{capabilities:?}");
        let methods = query(
            &path,
            &SymbolQuery::new().kinds([SymbolKindV2::Method]).capabilities(capabilities.iter().copied()),
        );
        assert_eq!(kinds_of(&methods), [(String::from("render"), SymbolKindV2::Method)], "{capabilities:?}");
    }
}

/// The kinds the deferred declarations report.
const DEFERRED_KINDS: [SymbolKindV2; 5] = [
    SymbolKindV2::Constructor,
    SymbolKindV2::EnumVariant,
    SymbolKindV2::TypeAlias,
    SymbolKindV2::Field,
    SymbolKindV2::Namespace,
];

/// For every source of the deferred kind matrix, a wide result narrows to
/// each deferred kind (alone, re-rooted, and with scope and name predicates)
/// exactly as an uncached run of that query.
#[test]
fn a_wide_result_narrows_to_each_deferred_kind() {
    let dir = TempDir::new().expect("tempdir");
    let mut compared = [0usize; DEFERRED_KINDS.len()];
    for (ordinal, (language, source)) in deferred_kind_sources().into_iter().enumerate() {
        let path = write(dir.path(), &format!("src/deferred_{ordinal}.{}", language.extensions()[0]), source);
        let label = format!("{language:?} {source:?}");
        let wide = query(&path, &SymbolQuery::new().capabilities([Capability::Containment]));
        for (kind_idx, kind) in DEFERRED_KINDS.into_iter().enumerate() {
            let queries = [
                SymbolQuery::new().kinds([kind]),
                SymbolQuery::new().kinds([kind]).scan_root(dir.path()),
                SymbolQuery::new()
                    .kinds([kind])
                    .scopes([SymbolScope::Item, SymbolScope::Member])
                    .exclude_name(NamePattern::new("no_declaration_has_this_name"))
                    .capabilities([Capability::Containment]),
            ];
            for narrow in queries {
                let narrowed = wide
                    .clone()
                    .narrowed(&narrow)
                    .unwrap_or_else(|| panic!("{label}: {narrow:?} is covered"));
                let uncached = query(&path, &narrow);
                assert_eq!(
                    serde_json::to_value(&narrowed.index).expect("serialize"),
                    serde_json::to_value(&uncached.index).expect("serialize"),
                    "{label}: {narrow:?}"
                );
                assert_eq!(narrowed.name_prefix(), uncached.name_prefix(), "{label}: {narrow:?}");
                compared[kind_idx] += uncached.index.symbols.len();
            }
        }
    }
    for (kind, count) in DEFERRED_KINDS.iter().zip(compared) {
        assert!(count > 0, "{kind:?}: no record was compared");
    }
}

/// A captured namespace is named and selected like any other declaration:
/// a PHP namespace and its members are namespace-rooted (no file prefix,
/// `App` never doubled), a TypeScript one is file-qualified, and `--in`
/// selects a namespace's members by its name.
#[test]
fn captured_namespaces_keep_their_members_names_under_a_scan_root() {
    let dir = TempDir::new().expect("tempdir");
    let php = write(dir.path(), "repo/src/app.php", "<?php\nnamespace App;\n\nfunction inside() {}\n");
    let braced = write(dir.path(), "repo/src/braced.php", "<?php\nnamespace App\\Braced {\n    function inside() {}\n}\n");
    let ts = write(dir.path(), "repo/src/space.ts", "namespace Space {\n  export function inside() {}\n}\n");
    let root = dir.path().join("repo");
    let rooted = || SymbolQuery::new().scan_root(&root);

    assert_eq!(names(&query(&php, &rooted())), set(&["App", "App\\inside"]));
    assert_eq!(names(&query(&braced, &rooted())), set(&["App\\Braced", "App\\Braced\\inside"]));
    assert_eq!(names(&query(&ts, &rooted())), set(&["src/space.ts#Space", "src/space.ts#Space.inside"]));

    let namespaces = rooted().kinds([SymbolKindV2::Namespace]);
    assert_eq!(names(&query(&php, &namespaces)), set(&["App"]));
    assert_eq!(names(&query(&ts, &namespaces)), set(&["src/space.ts#Space"]));

    // Containment links a braced namespace's members; a semicolon namespace
    // contains nothing, and neither is its own container.
    let members_of = |path: &Path, pattern: &str| names(&query(path, &rooted().in_container(NamePattern::new(pattern))));
    assert_eq!(members_of(&ts, "Space"), set(&["src/space.ts#Space.inside"]));
    assert_eq!(members_of(&braced, "App\\Braced"), set(&["App\\Braced\\inside"]));
    assert_eq!(members_of(&php, "App"), set(&[]));
}

/// A wide result narrows to a filtered, re-rooted query exactly as an
/// uncached run of it; a result filtered by other predicates cannot answer.
#[test]
fn a_wide_result_narrows_to_a_filtered_query_with_its_own_prefix() {
    for path in multi_fixtures() {
        let label = display_name(&path);
        let root = path.parent().expect("parent").parent().expect("grandparent").to_path_buf();
        let wide = query(&path, &SymbolQuery::new().capabilities([Capability::Flags, Capability::Metrics]));
        let filtered = SymbolQuery::new()
            .kinds(FUNCTION_KINDS)
            .scopes([SymbolScope::Item, SymbolScope::Member])
            .exclude_name(NamePattern::new("helper"))
            .scan_root(&root);
        let narrowed = wide.clone().narrowed(&filtered).unwrap_or_else(|| panic!("{label}: covered"));
        let uncached = query(&path, &filtered);
        assert_eq!(
            serde_json::to_value(&narrowed.index).expect("serialize"),
            serde_json::to_value(&uncached.index).expect("serialize"),
            "{label}"
        );
        assert_eq!(narrowed.name_prefix(), uncached.name_prefix(), "{label}");
        // A filtered result answers only the same predicates under the same
        // prefix: a qualified-name predicate matched under that prefix.
        assert!(narrowed.clone().narrowed(&filtered).is_some(), "{label}: same query");
        assert!(
            narrowed.clone().narrowed(&filtered.clone().scan_root(path.parent().expect("parent"))).is_none(),
            "{label}: another scan root"
        );
        // An unfiltered result re-prefixes for another root.
        let other_root = SymbolQuery::new().kinds(FUNCTION_KINDS).scan_root(path.parent().expect("parent"));
        let reprefixed = wide.clone().narrowed(&other_root).unwrap_or_else(|| panic!("{label}: covered"));
        assert_eq!(
            serde_json::to_value(&reprefixed.index).expect("serialize"),
            serde_json::to_value(&query(&path, &other_root).index).expect("serialize"),
            "{label}: re-prefixed"
        );
        assert!(
            narrowed.narrowed(&SymbolQuery::new().kinds(FUNCTION_KINDS)).is_none(),
            "{label}: a filtered result cannot answer an unfiltered query"
        );
    }
}

#[test]
fn a_naming_context_names_modules_without_changing_ids() {
    let dir = TempDir::new().expect("tempdir");
    let rust = write(
        dir.path(),
        "crate/src/shared/symbol.rs",
        "pub struct SymbolInfo;\nimpl SymbolInfo {\n    pub fn new() -> Self { SymbolInfo }\n}\n",
    );
    let python = write(
        dir.path(),
        "py/package/module.py",
        "class Class:\n    def method(self):\n        pass\n",
    );
    let java = write(dir.path(), "java/Point.java", "package com.example;\nclass Point {}\n");

    let new_id = |result: &SymbolQueryResult, name: &str| {
        let record = result.index.symbols.iter().find(|record| record.identity.name == name).expect(name);
        (record.identity.qualified_name.clone(), record.id.clone())
    };
    let crate_context = SymbolQuery::new().naming_context(NamingContext::new(dir.path().join("crate/src"), "crate"));
    let with_context = query(&rust, &crate_context);
    let without = query(&rust, &SymbolQuery::new().scan_root(dir.path().join("crate")));
    let (context_name, context_id) = new_id(&with_context, "new");
    let (file_name, file_id) = new_id(&without, "new");
    assert_eq!(context_name, "crate::shared::symbol::SymbolInfo::new");
    assert_eq!(with_context.name_prefix(), "crate::shared::symbol::");
    assert_eq!(file_name, "src/shared/symbol.rs#SymbolInfo::new");
    assert_eq!(context_id, file_id, "the ID derives from the in-file name only");

    let package_context = SymbolQuery::new().naming_context(NamingContext::new(dir.path().join("py"), ""));
    assert_eq!(new_id(&query(&python, &package_context), "method").0, "package.module.Class.method");
    assert_eq!(new_id(&query(&python, &SymbolQuery::new()), "method").0, "module.py#Class.method");

    // A namespace-rooted name takes no prefix of either kind.
    let java_context = SymbolQuery::new().naming_context(NamingContext::new(dir.path().join("java"), "ignored"));
    assert_eq!(new_id(&query(&java, &java_context), "Point").0, "com.example.Point");
    assert_eq!(new_id(&query(&java, &SymbolQuery::new()), "Point").0, "com.example.Point");
}

/// Smart case over non-ASCII names, through the record path.
#[test]
fn non_ascii_names_match_with_smart_case() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(dir.path(), "names.py", "def ünï():\n    pass\n\nclass Ünï:\n    pass\n\ndef straße():\n    pass\n");
    let matching = |pattern: &str| names(&query(&path, &SymbolQuery::new().name(NamePattern::new(pattern))));
    assert_eq!(matching("ünï"), set(&["names.py#ünï", "names.py#Ünï"]));
    assert_eq!(matching("Ünï"), set(&["names.py#Ünï"]));
    assert_eq!(matching("ünï!"), set(&["names.py#ünï", "names.py#Ünï"]));
    assert_eq!(matching("Ünï!"), set(&["names.py#Ünï"]));
    assert_eq!(matching("STRASSE"), set(&[]));
    assert_eq!(matching("ße"), set(&["names.py#straße"]));
    assert_eq!(matching("names.py#*ï"), set(&["names.py#ünï", "names.py#Ünï"]));
}

/// A qualified `--in` pattern tells repeated container names apart.
#[test]
fn qualified_container_patterns_disambiguate_repeated_names() {
    let dir = TempDir::new().expect("tempdir");
    let path = write(
        dir.path(),
        "shapes.cpp",
        "namespace a { struct S { void f() {} }; }\nnamespace b { struct S { void f() {} }; }\n",
    );
    let members = |pattern: &str| {
        let selected = SymbolQuery::new().kinds(FUNCTION_KINDS).in_container(NamePattern::new(pattern));
        names(&query(&path, &selected))
    };
    assert_eq!(members("S"), set(&["a::S::f", "b::S::f"]));
    assert_eq!(members("b::S"), set(&["b::S::f"]));
    assert_eq!(members("b::S!"), set(&["b::S::f"]));
    assert_eq!(members("s"), set(&["a::S::f", "b::S::f"]), "smart case applies to `--in`");
    assert_eq!(members("Missing"), set(&[]));
}
