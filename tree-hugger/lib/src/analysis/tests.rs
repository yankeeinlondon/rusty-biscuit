use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::TreeFile;
use crate::analysis::{
    NameIndex, RefSet, agrees_with_owner_path, analyze_file, ends_with_segments,
    ExportTargets, export_record, find_container_symbol, last_name_segment, owner_path,
    qualifies, resolve_containers,
};
use crate::shared::{
    AnalysisPass, DocsFacet, ExportRecord, IdentityFacet, ProgrammingLanguage, ProvenanceFacet,
    SchemaVersion, SemanticFacet, SourceFacet, SymbolId, SymbolKindData, SymbolKindV2,
    SymbolRecord, SymbolRef, TextPoint, TextSpan, TypeFacet, VisibilityFacet,
};

#[test]
fn pipeline_marks_completed_passes() {
    let dir = TempDir::new().expect("tempdir");
    let file = dir.path().join("sample.rs");
    fs::write(
        &file,
        "/// says hello\nfn hello(name: &str) -> String { format!(\"hi {name}\") }",
    )
    .expect("write fixture");

    let tree_file = TreeFile::new(&file).expect("parse");
    let index = analyze_file(
        &tree_file,
        &[
            AnalysisPass::Parse,
            AnalysisPass::Bind,
            AnalysisPass::Semantic,
            AnalysisPass::Docs,
        ],
    )
    .expect("analyze");

    assert_eq!(
        index.completed_passes,
        [
            AnalysisPass::Parse,
            AnalysisPass::Bind,
            AnalysisPass::Semantic,
            AnalysisPass::Docs,
        ]
    );
    assert!(!index.symbols.is_empty());
}

fn test_symbol(name: &str, start_byte: u32, end_byte: u32) -> SymbolRecord {
    SymbolRecord {
        schema_version: SchemaVersion::CURRENT,
        id: SymbolId(name.to_string()),
        language: ProgrammingLanguage::Rust,
        kind: SymbolKindV2::Function,
        scope: crate::shared::SymbolScope::Item,
        identity: IdentityFacet {
            name: name.to_string(),
            display_name: name.to_string(),
            qualified_name: name.to_string(),
            is_namespace_rooted: false,
            container_name: None,
            keyword: None,
            container_kind: None,
            stable_key: name.to_string(),
        },
        source: SourceFacet {
            declaration_span: TextSpan {
                start: TextPoint::default(),
                end: TextPoint::default(),
                start_byte,
                end_byte,
            },
            ..SourceFacet::default()
        },
        visibility: VisibilityFacet::default(),
        docs: DocsFacet::default(),
        attributes: Vec::new(),
        modifiers: Vec::new(),
        type_info: TypeFacet::default(),
        relations: Default::default(),
        semantics: SemanticFacet::default(),
        metrics: None,
        provenance: ProvenanceFacet {
            extractor: "test".to_string(),
            extractor_version: "test".to_string(),
            parse_pass: AnalysisPass::Parse.as_str().to_string(),
        },
        kind_data: SymbolKindData::Unknown,
        extensions: Default::default(),
    }
}

/// Every symbol fixture (primary files, extras and the `multi/` tree).
fn fixture_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read fixture dir") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                walk(&path, found);
            } else if ProgrammingLanguage::from_path(&path).is_some() {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(
        &biscuit_test_harness::manifest_dir!().join("tests/fixtures/symbols"),
        &mut found,
    );
    found.sort();
    found
}

/// The first record with the name, found by a full scan instead of the name
/// index.
fn scan_reference_target(symbols: &[SymbolRecord], name: &str) -> Option<usize> {
    symbols.iter().position(|symbol| symbol.identity.name == name)
}

/// `find_container_symbol` by a full scan instead of the name index.
fn scan_container_symbol(
    symbols: &[SymbolRecord],
    child_idx: usize,
    container_name: &str,
) -> Option<usize> {
    let child = &symbols[child_idx];
    let bare = if symbols.iter().any(|symbol| symbol.identity.name == container_name) {
        container_name
    } else {
        last_name_segment(container_name)
    };
    let enclosing = symbols
        .iter()
        .enumerate()
        .filter(|(idx, candidate)| {
            *idx != child_idx
                && candidate.identity.name == bare
                && candidate.source.declaration_span.start_byte
                    <= child.source.declaration_span.start_byte
                && candidate.source.declaration_span.end_byte
                    >= child.source.declaration_span.end_byte
        })
        .min_by_key(|(_, candidate)| {
            candidate
                .source
                .declaration_span
                .end_byte
                .saturating_sub(candidate.source.declaration_span.start_byte)
        })
        .map(|(idx, _)| idx);

    enclosing.or_else(|| {
        let container_kind = child.identity.container_kind?;
        let of_kind: Vec<usize> = (0..symbols.len())
            .filter(|&idx| {
                idx != child_idx
                    && symbols[idx].identity.name == bare
                    && symbols[idx].kind == container_kind
            })
            .collect();
        scan_nearest_qualifying_owner(symbols, child, &of_kind).or_else(|| {
            let named: Vec<usize> = of_kind
                .into_iter()
                .filter(|&idx| {
                    let owner = &symbols[idx].identity.qualified_name;
                    (bare == container_name || ends_with_segments(owner, container_name))
                        && agrees_with_owner_path(owner, &child.identity.qualified_name)
                })
                .collect();
            (named.len() == 1).then(|| named[0])
        })
    })
}

/// The scan of every candidate `nearest_qualifying_owner` did before the
/// owner index: the longest qualifying name, then (where one owner may be
/// declared again) the nearest declaration before the child, else the first
/// after it; several of the longest name elsewhere are no owner.
fn scan_nearest_qualifying_owner(
    symbols: &[SymbolRecord],
    child: &SymbolRecord,
    candidates: &[usize],
) -> Option<usize> {
    let child_start = i64::from(child.source.declaration_span.start_byte);
    let qualifying: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|&idx| qualifies(&symbols[idx].identity.qualified_name, &child.identity.qualified_name))
        .collect();
    let longest = qualifying.iter().map(|&idx| symbols[idx].identity.qualified_name.len()).max()?;
    let tied = qualifying.iter().filter(|&&idx| symbols[idx].identity.qualified_name.len() == longest).count();
    if tied > 1 && !super::repeats_one_owner(child.language) {
        return None;
    }
    qualifying.into_iter().max_by_key(|&idx| {
        let start = i64::from(symbols[idx].source.declaration_span.start_byte);
        let nearness = if start <= child_start { start } else { -start - 1 };
        (symbols[idx].identity.qualified_name.len(), start <= child_start, nearness)
    })
}

/// Every child's container by the scan, in record order, as
/// `resolve_containers` reports it.
fn scan_containers(symbols: &[SymbolRecord]) -> Vec<(usize, usize)> {
    symbols
        .iter()
        .enumerate()
        .filter_map(|(child_idx, symbol)| {
            let container_name = symbol.identity.container_name.as_deref()?;
            Some((child_idx, scan_container_symbol(symbols, child_idx, container_name)?))
        })
        .collect()
}

/// The full scan of every record an export target lookup did before the
/// name index.
fn scan_export_target(symbols: &[SymbolRecord], export: &ExportRecord) -> Option<usize> {
    symbols.iter().position(|symbol| {
        symbol.identity.name == export.name
            && symbol.source.name_span.as_ref().is_some_and(|span| {
                span.start_byte == export.span.start_byte && span.end_byte == export.span.end_byte
            })
    })
}

crate::language_cases::language_tests!(name_index_selects_the_same_targets_as_a_full_scan => check_name_index_targets);

fn check_name_index_targets(language: ProgrammingLanguage) {
    let (mut references, mut containers, mut exports) = (0, 0, 0);
    for path in fixture_files().into_iter().filter(|path| ProgrammingLanguage::from_path(path) == Some(language)) {
        let label = path.display().to_string();
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let symbols = tree_file.symbol_records().expect("records");
        let names = NameIndex::new(&symbols);

        let mut lookups: Vec<String> = tree_file
            .referenced_symbols()
            .expect("references")
            .into_iter()
            .map(|reference| reference.name)
            .collect();
        lookups.extend(symbols.iter().map(|symbol| symbol.identity.name.clone()));
        lookups.push("no_such_name_anywhere".to_string());
        for name in &lookups {
            assert_eq!(
                names.named(name).first().copied(),
                scan_reference_target(&symbols, name),
                "{label}: reference target for `{name}`"
            );
            references += 1;
        }

        for (child_idx, symbol) in symbols.iter().enumerate() {
            let Some(container_name) = symbol.identity.container_name.as_deref() else {
                continue;
            };
            assert_eq!(
                find_container_symbol(&symbols, &names, child_idx, container_name),
                scan_container_symbol(&symbols, child_idx, container_name),
                "{label}: container of `{}`",
                symbol.identity.name
            );
            containers += 1;
        }
        assert_eq!(resolve_containers(&symbols, &names), scan_containers(&symbols), "{label}: containers");

        for export in tree_file.exported_symbols().expect("exports") {
            let export = export_record(&export);
            assert_eq!(
                ExportTargets::new(&symbols).find(&export),
                scan_export_target(&symbols, &export),
                "{label}: export target for `{}`",
                export.name
            );
            exports += 1;
        }
    }

    let minimum = crate::language_cases::corpus_minimums(language);
    assert!(references >= minimum.lookups && containers >= minimum.containers && exports >= minimum.exports,
        "{language}: {references} lookups, {containers} containers, {exports} exports");
}

/// Repeated and nested same-named owners, where one file's children share
/// the owner index and the enclosing sweep, resolve as the scan does.
#[test]
fn owner_index_resolves_repeated_owners_as_the_scan_does() {
    let dir = TempDir::new().expect("temp dir");
    let mut rust = String::from("struct Probe;\nimpl Probe { fn m(&self) {} }\n");
    let mut lua = String::new();
    let mut cpp = String::new();
    for i in 0..12 {
        rust.push_str(&format!(
            "mod r{i} {{ struct Probe; impl Probe {{ fn m(&self) {{}} }} \
             mod Probe {{ fn Probe() {{ fn Probe() {{}} }} }} impl super::Probe {{ fn n(&self) {{}} }} }}\n"
        ));
        lua.push_str(&format!(
            "local Probe = {{}}\nfunction Probe:m() return {i} end\ndo local Probe = {{}} function Probe.f() end end\n"
        ));
        cpp.push_str(&format!(
            "namespace Probe {{ struct Probe {{ void m(); }}; }}\nvoid Probe::Probe::m() {{}}\nstruct Probe{i} {{}};\n"
        ));
    }
    lua.push_str("function Probe:late() end\nreturn Probe\n");
    let mut containers = 0;
    for (file_name, source) in [("owners.rs", rust), ("tables.lua", lua), ("owners.cpp", cpp)] {
        let path = dir.path().join(file_name);
        fs::write(&path, source).expect("write source");
        let symbols = TreeFile::new(&path).expect("parse").symbol_records().expect("records");
        let names = NameIndex::new(&symbols);
        let resolved = resolve_containers(&symbols, &names);
        assert_eq!(resolved, scan_containers(&symbols), "{file_name}: containers");
        containers += resolved.len();
    }
    assert!(containers > 100, "only {containers} containers resolved");
}

#[test]
fn name_index_keeps_the_first_of_equal_candidates() {
    // Two same-named containers of equal size both enclose the child; a
    // third same-named record comes first but does not enclose it.
    let mut symbols = vec![
        test_symbol("Owner", 300, 400),
        test_symbol("Owner", 0, 100),
        test_symbol("Owner", 0, 100),
        test_symbol("child", 10, 20),
    ];
    symbols[3].identity.container_name = Some("Owner".to_string());
    let names = NameIndex::new(&symbols);

    assert_eq!(names.named("Owner"), [0, 1, 2]);
    assert_eq!(names.named("missing"), [] as [usize; 0]);
    assert_eq!(find_container_symbol(&symbols, &names, 3, "Owner"), Some(1));
    assert_eq!(scan_container_symbol(&symbols, 3, "Owner"), Some(1));

    // Not enclosed: of the recorded kind, the record whose qualified name
    // qualifies the child's, never merely the first of the name.
    symbols[3].source.declaration_span.start_byte = 500;
    symbols[3].source.declaration_span.end_byte = 510;
    symbols[3].identity.container_kind = Some(SymbolKindV2::Function);
    symbols[0].identity.qualified_name = "left::Owner".to_string();
    symbols[1].identity.qualified_name = "right::Owner".to_string();
    symbols[2].identity.qualified_name = "right::Ownerx".to_string();
    symbols[3].identity.qualified_name = "right::Owner::child".to_string();
    let names = NameIndex::new(&symbols);
    assert_eq!(find_container_symbol(&symbols, &names, 3, "Owner"), Some(1));
    assert_eq!(scan_container_symbol(&symbols, 3, "Owner"), Some(1));

    // Several candidates and none qualifies the child: no container.
    symbols[3].identity.qualified_name = "elsewhere::Owner::child".to_string();
    assert_eq!(find_container_symbol(&symbols, &names, 3, "Owner"), None);
    assert_eq!(scan_container_symbol(&symbols, 3, "Owner"), None);

    // The only candidate of the kind is the container only when its
    // qualified name agrees with the child's owner path: a sole `left::Owner`
    // never owns `elsewhere::Owner::child`.
    symbols[3].identity.container_kind = Some(SymbolKindV2::Class);
    symbols[0].kind = SymbolKindV2::Class;
    let names = NameIndex::new(&symbols);
    assert_eq!(find_container_symbol(&symbols, &names, 3, "Owner"), None);
    assert_eq!(scan_container_symbol(&symbols, 3, "Owner"), None);
    symbols[3].identity.qualified_name = "Owner::child".to_string();
    assert_eq!(find_container_symbol(&symbols, &names, 3, "Owner"), Some(0));
    assert_eq!(scan_container_symbol(&symbols, 3, "Owner"), Some(0));
    symbols[3].identity.qualified_name = "child".to_string();
    assert_eq!(find_container_symbol(&symbols, &names, 3, "Owner"), Some(0));

    // A qualified container name is matched by its tail segments.
    symbols[3].identity.qualified_name = "left::Owner::child".to_string();
    symbols[0].identity.qualified_name = "outer::left::Owner".to_string();
    assert_eq!(find_container_symbol(&symbols, &names, 3, "left::Owner"), Some(0));
    assert_eq!(find_container_symbol(&symbols, &names, 3, "right::Owner"), None);
    symbols[3].identity.container_kind = Some(SymbolKindV2::Enum);
    assert_eq!(find_container_symbol(&symbols, &names, 3, "Owner"), None);
}

#[test]
fn qualified_name_segments_match_whole_segments_only() {
    assert!(qualifies("a::B", "a::B::f"));
    assert!(qualifies("Left.Repeated", "Left.Repeated.a"));
    assert!(qualifies("N", "N:f"));
    assert!(qualifies("Ns", "Ns\\f"));
    assert!(!qualifies("a::B", "a::Bc::f"));
    assert!(!qualifies("a::B", "a::B"));
    assert!(!qualifies("a::B", "a::B::"));
    assert!(ends_with_segments("Outer.Left.Repeated", "Left.Repeated"));
    assert!(ends_with_segments("Left.Repeated", "Left.Repeated"));
    assert!(!ends_with_segments("XLeft.Repeated", "Left.Repeated"));
    assert_eq!(last_name_segment("Left.Repeated"), "Repeated");
    assert_eq!(last_name_segment("a::b::C"), "C");
    assert_eq!(last_name_segment("C"), "C");
    assert_eq!(owner_path("other::W::probe"), Some("other::W"));
    assert_eq!(owner_path("Outer.Left.f"), Some("Outer.Left"));
    assert_eq!(owner_path("M:m"), Some("M"));
    assert_eq!(owner_path("Ns\\f"), Some("Ns"));
    assert_eq!(owner_path("probe"), None);
    assert!(agrees_with_owner_path("W", "W::probe"));
    assert!(agrees_with_owner_path("a::W", "W::probe"));
    assert!(agrees_with_owner_path("right::W", "right::W::m::self"));
    assert!(agrees_with_owner_path("W", "probe"));
    assert!(!agrees_with_owner_path("W", "other::W::probe"));
    assert!(!agrees_with_owner_path("left::W", "right::W::m"));
}

fn reference(id: Option<&str>, name: &str, qualified_name: Option<&str>) -> SymbolRef {
    SymbolRef {
        source_span: None,
        id: id.map(|id| SymbolId(id.to_string())),
        name: name.to_string(),
        qualified_name: qualified_name.map(str::to_string),
    }
}

#[test]
fn ref_set_keeps_distinct_unresolved_references_and_dedupes_resolved_ones_by_id() {
    let mut set = RefSet::default();
    // Two unresolved references with missing IDs are distinct by name.
    set.push(reference(None, "path", None));
    set.push(reference(None, "argv", None));
    // ...and by qualified name.
    set.push(reference(None, "path", Some("os.path")));
    // A repeat of an unresolved reference is dropped.
    set.push(reference(None, "argv", None));
    // Resolved references compare by ID only: a same-named reference with
    // another ID is kept, the same ID under another name is a repeat.
    set.push(reference(Some("a"), "helper", Some("A::helper")));
    set.push(reference(Some("b"), "helper", Some("A::helper")));
    set.push(reference(Some("a"), "renamed", None));
    // A resolved and an unresolved reference never repeat each other.
    set.push(reference(None, "helper", Some("A::helper")));

    let kept: Vec<(Option<&str>, &str, Option<&str>)> = set
        .refs
        .iter()
        .map(|r| (r.id.as_ref().map(|id| id.0.as_str()), r.name.as_str(), r.qualified_name.as_deref()))
        .collect();
    assert_eq!(
        kept,
        [
            (None, "path", None),
            (None, "argv", None),
            (None, "path", Some("os.path")),
            (Some("a"), "helper", Some("A::helper")),
            (Some("b"), "helper", Some("A::helper")),
            (None, "helper", Some("A::helper")),
        ]
    );
}
