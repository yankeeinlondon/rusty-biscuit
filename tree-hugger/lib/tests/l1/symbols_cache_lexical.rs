//! Grammar-derived imports honor opaque literals and comment boundaries.
use serde_json::{Value, json};
use tempfile::TempDir;
use tree_hugger::cache::{CacheLookup, CacheMiss, PersistentCache, SymbolCache};
use tree_hugger::{Capability, SourceFile, SymbolQuery, SymbolQueryResult, SymbolKindV2, TreeFile, TriState, work_counters};
#[path = "../fixtures/scoped_import_lexical.rs"]
mod corpus;
fn facts(result: &SymbolQueryResult, case: &corpus::Case) {
    let expected: Vec<_> = case.import.into_iter().collect();
    assert_eq!(result.index.imports.iter().map(|i| (i.name.as_str(), i.source.as_deref().expect("source"))).collect::<Vec<_>>(), expected, "{} {}", case.extension, case.label);
    for import in &result.index.imports {
        assert_eq!(&case.source[import.span.start_byte as usize..import.span.end_byte as usize], import.name);
        assert_eq!(import.span.start_byte as usize, case.source.find(&import.name).expect("written binding"));
        let expected_alias = if matches!(case.extension, "java" | "swift") { None } else { Some(import.name.as_str()) };
        assert_eq!(import.alias.as_deref(), expected_alias);
        let original = match case.extension { "ts" | "js" | "rs" | "scala" => Some("g"), "cs" | "php" => Some("B"), "go" => Some("a"), _ => None };
        assert_eq!(import.symbol.qualified_name.as_deref(), original);
        assert!(!result.index.occurrences.iter().any(|o| o.span.start_byte == import.span.start_byte));
        let statement = import.statement_span.as_ref().expect("statement");
        let keyword = match case.extension { "rs" | "php" => "use ", "cs" => "using ", _ => "import " };
        assert!(case.source[statement.start_byte as usize..statement.end_byte as usize].starts_with(keyword));
        let start = case.source.find(keyword).expect("statement keyword");
        let end = case.source[start..].find(';').map(|byte| start + byte + 1)
            .or_else(|| case.source[start..].find('\n').map(|byte| start + byte)).expect("statement end");
        assert_eq!((statement.start_byte as usize, statement.end_byte as usize), (start, end));
    }
    if case.import.is_none() {
        assert!(case.label.starts_with("macro") || !result.index.occurrences.iter().any(|o| matches!(o.target.name.as_str(), "D" | "B" | "A")), "{} {}: opaque read", case.extension, case.label);
        assert!(result.index.symbols.iter().all(|s| s.relations.dependencies.is_empty()));
    } else if matches!(case.extension, "rs" | "ts" | "js") {
        let call = result.index.occurrences.iter().find(|o| o.target.name == "D").expect("real import call");
        assert_eq!(call.import_span.as_ref(), Some(&result.index.imports[0].span));
    }
    if !matches!(case.extension, "rs" | "ts" | "js") {
        assert!(result.index.symbols.iter().all(|record| record.relations.dependencies.is_empty() && record.relations.references.is_empty()));
        assert!(result.index.occurrences.iter().all(|occurrence| occurrence.confidence != tree_hugger::Confidence::Scoped));
        assert!(result.index.reference_coverage.iter().any(|region| region.reasons.contains(&tree_hugger::LimitReason::UnsupportedLanguage)));
    }
    if case.extension == "scala" && case.import.is_some() {
        let call = result.index.occurrences.iter().find(|occurrence| occurrence.target.name == "D").expect("conservative real import read");
        assert_eq!(call.kind, tree_hugger::OccurrenceKind::Call);
        assert_eq!(call.confidence, tree_hugger::Confidence::Unresolved);
        assert!(call.import_span.is_none());
    }
    for record in &result.index.symbols {
        if record.identity.name == "f" && record.semantics.is_recursive.is_some() {
            let expected = if !matches!(case.extension, "rs" | "ts" | "js") || case.source.contains(".test(") { TriState::Unknown } else { TriState::No };
            assert_eq!(record.semantics.is_recursive, Some(expected), "{} {}: recursion", case.extension, case.label);
            if !matches!(case.extension, "rs" | "ts" | "js") {
                assert_eq!(record.semantics.is_recursive, Some(TriState::Unknown));
            }
            if case.import.is_some() && matches!(case.extension, "rs" | "ts" | "js") {
                assert_eq!(record.semantics.is_recursive, Some(TriState::No));
                assert_eq!(record.relations.dependencies.len(), 1);
                assert_eq!(record.relations.dependencies[0].name, "D");
            }
        }
    }
}
fn entry(path: &std::path::Path) -> std::path::PathBuf {
    for item in std::fs::read_dir(path).expect("dir") { let p = item.expect("entry").path();
        if p.is_dir() { return entry(&p); }
        if p.extension().is_some_and(|e| e == "json") { return p; }
    } panic!("missing entry")
}
fn run(extension: &'static str) {
    for case in corpus::cases(extension) {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(format!("source.{extension}"));
        std::fs::write(&path, &case.source).expect("source");
        let parsed = TreeFile::new(&path).expect("parse");
        assert!(parsed.syntax_diagnostics().is_empty(), "{} {}: {:?}", extension, case.label, parsed.syntax_diagnostics());
        assert_eq!(parsed.imported_symbols().expect("public imports").len(), usize::from(case.import.is_some()), "{extension} {}", case.label);
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let narrow = SymbolQuery::new().kinds([SymbolKindV2::Function]).capabilities([Capability::Occurrences]);
        let semantic = SymbolQuery::new().kinds([SymbolKindV2::Function]).capabilities([Capability::Semantics]);
        let complete = SymbolQuery::complete_index();
        let projected = SymbolQuery::new().kinds([SymbolKindV2::Class]).capabilities([Capability::Occurrences]);
        for (request, lookup) in [(&narrow, CacheLookup::Miss(CacheMiss::Absent)), (&narrow, CacheLookup::Hit),
            (&semantic, CacheLookup::WidenedCapabilities { parsed: false }), (&complete, CacheLookup::WidenedKinds),
            (&complete, CacheLookup::Hit), (&projected, CacheLookup::Hit)] {
            let before = work_counters(); let out = cache.query(SourceFile::read(&path).expect("source"), request).expect("query");
            assert_eq!(out.lookup, lookup, "{extension} {}", case.label);
            if matches!(lookup, CacheLookup::Hit | CacheLookup::WidenedCapabilities { parsed: false }) {
                assert_eq!(work_counters().since(before).parser_constructions, 0);
            }
            facts(&out.result, &case);
        }
        let entry = entry(&dir.path().join("cache")); let saved: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        let mut changed = saved.clone();
        if case.import.is_some() { changed["imports"] = json!([]); }
        else {
            let start = case.source.rfind("f").expect("source-aligned invented token");
            let point = |byte: usize| { let before = &case.source[..byte]; json!({"line":before.matches('\n').count()+1,"column":byte-before.rfind('\n').map_or(0, |at| at+1)+1}) };
            let span = json!({"start_byte":start,"end_byte":start+1,"start":point(start),"end":point(start+1)});
            changed["imports"] = json!([{"name":"f","alias":null,"source":null,"span":span,"statement_span":span,"symbol":{"name":"f","qualified_name":null,"id":null,"source_span":null}}]);
        }
        for request in [&narrow, &semantic, &complete, &projected] {
            std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
            let out = cache.query(SourceFile::read(&path).expect("source"), request).expect("repair");
            assert!(matches!(out.lookup, CacheLookup::Miss(_)), "{extension} {}", case.label); facts(&out.result, &case);
            let before = work_counters(); let out = cache.query(SourceFile::read(&path).expect("source"), request).expect("warm");
            assert_eq!(out.lookup, CacheLookup::Hit); assert_eq!(work_counters().since(before).parser_constructions, 0); facts(&out.result, &case);
        }
    }
}
macro_rules! lexical_tests { ($($name:ident => $ext:literal),+ $(,)?) => { $(#[test] fn $name(){run($ext);})+ }; }
lexical_tests! { lexical_inventory_ts => "ts", lexical_inventory_js => "js", lexical_inventory_swift => "swift",
    lexical_inventory_cs => "cs", lexical_inventory_rs => "rs", lexical_inventory_java => "java",
    lexical_inventory_go => "go", lexical_inventory_scala => "scala", lexical_inventory_py => "py",
    lexical_inventory_php => "php", lexical_inventory_c => "c", lexical_inventory_cpp => "cpp",
    lexical_inventory_lua => "lua", lexical_inventory_pl => "pl", lexical_inventory_sh => "sh", lexical_inventory_zsh => "zsh" }

#[test]
fn lexical_inventory_corpus_includes_all_languages_and_every_regex_context() {
    for (extension, count) in [("ts", 50), ("js", 50), ("swift", 6), ("cs", 5), ("rs", 7), ("java", 2), ("go", 2), ("scala", 2), ("py", 4), ("php", 4), ("c", 1), ("cpp", 1), ("lua", 1), ("pl", 1), ("sh", 1), ("zsh", 1)] {
        let cases = corpus::cases(extension);
        assert_eq!(cases.len(), count, "{extension}");
        assert_eq!(cases.iter().map(|case| &case.label).collect::<std::collections::BTreeSet<_>>().len(), count);
    }
}
