//! Restored tool-written JSON import evidence, exercised through public cache
//! queries. Each fixture independently states its binding and metadata.

use std::path::{Path, PathBuf};
use serde_json::{Value, json};
use tempfile::TempDir;
use tree_hugger::cache::{CacheLookup, CacheMiss, PersistentCache, SymbolCache};
use tree_hugger::{Capability, SourceFile, SymbolQuery, SymbolQueryResult, SymbolKindV2, TriState, work_counters};

type ImportCase = (&'static str, &'static str, &'static str, Option<&'static str>, Option<&'static str>, Option<&'static str>);
const CASES: &[ImportCase] = &[
    ("rs", "use a::f as D; fn f(){D();}", "D", Some("D"), Some("a"), Some("f")),
    ("ts", "import { f as D } from 'm'; function f(){D();}", "D", Some("D"), Some("m"), Some("f")),
    ("js", "import { f as D } from 'm'; function f(){D();}", "D", Some("D"), Some("m"), Some("f")),
    ("java", "import java.util.*; class W { void f(){} }", "util", Some("util"), Some("java.util"), Some("*")),
    ("cs", "using D = A.B; class W { void f(){} }", "D", Some("D"), Some("A.B"), Some("B")),
    ("php", "<?php use A\\B as D; function f(){}", "D", Some("D"), Some("A\\B"), Some("B")),
    ("py", "from a import f as D\ndef f():\n    D()\n", "D", Some("D"), Some("a"), Some("f")),
    ("go", "package p\nimport D \"a/f\"\nfunc f(){D.F()}\n", "D", Some("D"), Some("a/f"), Some("f")),
    ("scala", "import a.f as D\ndef f()=D()\n", "D", Some("D"), Some("a.f"), Some("f")),
    ("swift", "import struct A.B\nfunc f() {}\n", "B", None, Some("A.B"), None),
    ("rs", "use a::{g::f as D}; fn f(){D();}", "D", Some("D"), Some("a::g"), Some("f")),
    ("scala", "import a.{f => D}\ndef f()=D()\n", "D", Some("D"), Some("a.f"), Some("f")),
    ("php", "<?php use A\\{B as D}; function f(){}", "D", Some("D"), Some("A\\B"), Some("B")),
    ("cs", "using D = A.B<C>; class W { void f(){} }", "D", Some("D"), Some("A.B<C>"), Some("B")),
    ("java", "import p.C; class W { void f(){} }", "C", None, Some("p"), None),
    ("java", "import static C.f; class W { void f(){} }", "f", None, Some("C"), None),
];
const FIELDS: &[&str] = &[
    "name", "alias", "source", "span", "statement_span", "symbol",
    "symbol/name", "symbol/qualified_name", "symbol/id", "symbol/source_span",
    "span/start_byte", "span/end_byte", "span/start", "span/end",
    "span/start/line", "span/start/column", "span/end/line", "span/end/column",
    "statement_span/start_byte", "statement_span/end_byte", "statement_span/start", "statement_span/end",
    "statement_span/start/line", "statement_span/start/column", "statement_span/end/line", "statement_span/end/column",
];
fn source(path: &Path) -> SourceFile { SourceFile::read(path).expect("source") }
fn query(cache: &SymbolCache, path: &Path, request: &SymbolQuery) -> (CacheLookup, SymbolQueryResult) {
    let outcome = cache.query(source(path), request).expect("query");
    assert!(outcome.write_error.is_none());
    (outcome.lookup, outcome.result)
}
fn entry(dir: &TempDir) -> PathBuf {
    fn walk(path: &Path, found: &mut Vec<PathBuf>) {
        for item in std::fs::read_dir(path).expect("cache directory") {
            let path = item.expect("entry").path();
            if path.is_dir() { walk(&path, found); }
            else if path.extension().is_some_and(|ext| ext == "json") { found.push(path); }
        }
    }
    let mut found = Vec::new(); walk(&dir.path().join("cache"), &mut found);
    assert_eq!(found.len(), 1); found.remove(0)
}
fn assert_facts(result: &SymbolQueryResult, case: ImportCase) { assert_facts_for_source(result, case, case.1); }
fn assert_facts_for_source(result: &SymbolQueryResult, case: ImportCase, written: &str) {
    let (extension, _, name, alias, module, original) = case;
    let imports = &result.index.imports;
    assert_eq!(imports.len(), 1, "{extension}");
    let import = &imports[0];
    assert_eq!(import.name, name); assert_eq!(import.symbol.name, name);
    assert_eq!(import.alias.as_deref(), alias); assert_eq!(import.source.as_deref(), module);
    assert_eq!(import.symbol.qualified_name.as_deref(), original);
    assert!(import.symbol.id.is_none()); assert!(import.symbol.source_span.is_none());
    let start = if extension == "java" && name == "p" { written.find("p.").or_else(|| written.find("p /*")).expect("independent import path") } else { written.find(name).expect("independent name token") };
    assert_eq!(import.span.start_byte as usize, start);
    assert_eq!(import.span.end_byte as usize, start + name.len());
    let point = |byte: usize| {
        let before = &written[..byte];
        (before.matches('\n').count() + 1, byte - before.rfind('\n').map_or(0, |at| at + 1) + 1)
    };
    assert_eq!((import.span.start.line as usize, import.span.start.column as usize), point(start));
    assert_eq!((import.span.end.line as usize, import.span.end.column as usize), point(start + name.len()));
    let statement = import.statement_span.as_ref().expect("import statement");
    let keyword = match extension { "rs" | "php" => "use ", "cs" => "using ", "py" => "from ", _ => "import " };
    let statement_start = written.find(keyword).expect("independent statement start");
    let statement_end = written[statement_start..].find(';').map(|at| statement_start + at + 1)
        .or_else(|| written[statement_start..].find('\n').map(|at| statement_start + at)).expect("independent statement end");
    assert_eq!(statement.start_byte as usize, statement_start);
    assert_eq!(statement.end_byte as usize, statement_end);
    assert_eq!((statement.start.line as usize, statement.start.column as usize), point(statement.start_byte as usize));
    assert_eq!((statement.end.line as usize, statement.end.column as usize), point(statement.end_byte as usize));
}
fn run_matrix(case: ImportCase) {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join(format!("source.{}", case.0)); std::fs::write(&path, case.1).expect("fixture");
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
    let request = SymbolQuery::new().capabilities([Capability::Occurrences]);
    let (lookup, control) = query(&cache, &path, &request);
    assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent)); assert_facts(&control, case);
    let entry = entry(&dir); let bytes = std::fs::read_to_string(&entry).expect("written entry");
    let written: Value = serde_json::from_str(&bytes).expect("JSON");
    for field in std::iter::once("imports").chain(FIELDS.iter().copied()) {
        let pointer = if field == "imports" { "/imports".to_string() } else { format!("/imports/0/{field}") };
        let value = written.pointer(&pointer).expect("written field").clone();
        for shape in ["control", "absent", "null", "wrong", "wrong object", "wrong list", "empty", "zero", "huge", "contradiction", "duplicate key", "trailing", "bad element", "all bad elements", "duplicate record"] {
            if matches!(shape, "bad element" | "all bad elements" | "duplicate record") && field != "imports" { continue; }
            if matches!(shape, "zero" | "huge") && !value.is_number() { continue; }
            let mut changed = written.clone();
            let (parent, key) = pointer.rsplit_once('/').expect("path");
            let mut malformed = None;
            match shape {
                "control" => {}
                "absent" => { changed.pointer_mut(parent).expect("parent").as_object_mut().expect("object").remove(key); }
                "null" => *changed.pointer_mut(&pointer).expect("field") = Value::Null,
                "wrong" => *changed.pointer_mut(&pointer).expect("field") = if value.is_number() { json!("123") } else { json!(123) },
                "wrong object" => *changed.pointer_mut(&pointer).expect("field") = json!({}),
                "wrong list" => *changed.pointer_mut(&pointer).expect("field") = json!([]),
                "empty" => *changed.pointer_mut(&pointer).expect("field") = if field == "imports" { json!([]) } else if value.is_number() { json!(0) } else if value.is_object() { json!({}) } else { json!("") },
                "zero" => *changed.pointer_mut(&pointer).expect("field") = json!(0),
                "huge" => *changed.pointer_mut(&pointer).expect("field") = json!(999999999),
                "contradiction" => *changed.pointer_mut(&pointer).expect("field") = if field == "symbol/source_span" { written["imports"][0]["span"].clone() } else if field == "symbol/id" { json!("0".repeat(64)) } else if value.is_number() { json!(999999999) } else { json!("contradictory") },
                "duplicate key" => {
                    let object = serde_json::to_string(changed.pointer(parent).expect("parent")).expect("JSON");
                    let duplicate = format!("{{\"{key}\":{},{}", serde_json::to_string(&value).expect("value"), &object[1..]);
                    malformed = Some(serde_json::to_string(&changed).expect("JSON").replacen(&object, &duplicate, 1));
                }
                "trailing" => malformed = Some(format!("{bytes} garbage")),
                "bad element" => changed["imports"] = json!([written["imports"][0].clone(), 123]),
                "all bad elements" => changed["imports"] = json!([123]),
                "duplicate record" => changed["imports"] = json!([written["imports"][0].clone(), written["imports"][0].clone()]),
                _ => unreachable!(),
            }
            std::fs::write(&entry, malformed.unwrap_or_else(|| serde_json::to_string(&changed).expect("JSON"))).expect("restore and edit");
            let unchanged = shape == "control" || matches!(shape, "empty" | "zero") && value == json!(0)
                || matches!(shape, "absent" | "null") && value.is_null();
            let (lookup, repaired) = query(&cache, &path, &request);
            if unchanged { assert_eq!(lookup, CacheLookup::Hit, "{} {field} {shape}", case.0); }
            else { assert!(matches!(lookup, CacheLookup::Miss(_)), "{} {field} {shape}: {lookup:?}", case.0); }
            assert_facts(&repaired, case);
            let before = work_counters(); let (lookup, warm) = query(&cache, &path, &request);
            assert_eq!(lookup, CacheLookup::Hit); assert_eq!(work_counters().since(before).parser_constructions, 0);
            assert_facts(&warm, case);
            let semantic = SymbolQuery::new().capabilities([Capability::Semantics]);
            let before = work_counters(); let (lookup, wide) = query(&cache, &path, &semantic);
            assert_eq!(lookup, CacheLookup::WidenedCapabilities { parsed: false });
            assert_eq!(work_counters().since(before).parser_constructions, 0);
            assert_facts(&wide, case);
            let owner = wide.index.symbols.iter().find(|record| record.identity.name == "f").expect("independently named owner");
            assert_eq!(owner.semantics.is_recursive, Some(TriState::Unknown));
        }
    }
}
macro_rules! corpus_tests {
    ($($name:ident => $index:expr),+ $(,)?) => { const REGISTERED_CASES: &[usize] = &[$($index),+]; $(#[test] fn $name() { run_matrix(CASES[$index]); })+ };
}
corpus_tests! { imports_json_rs => 0, imports_json_ts => 1, imports_json_js => 2, imports_json_java => 3,
    imports_json_cs => 4, imports_json_php => 5, imports_json_py => 6, imports_json_go => 7,
    imports_json_scala => 8, imports_json_swift => 9,
    imports_json_rust_group => 10, imports_json_scala_group => 11, imports_json_php_group => 12, imports_json_cs_generic => 13, imports_json_java_shallow => 14, imports_json_java_static => 15 }
#[test]
fn imports_json_corpus_has_every_producer_and_geometry_leaf() {
    assert_eq!(CASES.iter().map(|case| case.0).collect::<std::collections::BTreeSet<_>>().len(), 10);
    assert_eq!(FIELDS.len(), 26);
    assert_eq!(REGISTERED_CASES, (0..CASES.len()).collect::<Vec<_>>());
    for prefix in ["span", "statement_span"] {
        for suffix in ["start_byte", "end_byte", "start", "end", "start/line", "start/column", "end/line", "end/column"] {
            assert!(FIELDS.contains(&format!("{prefix}/{suffix}").as_str()));
        }
    }
}

#[test]
fn imports_json_empty_producers_reject_invented_records_and_keep_empty_hits() {
    let seed = TempDir::new().expect("temp"); let seed_path = seed.path().join("seed.rs");
    std::fs::write(&seed_path, CASES[0].1).expect("seed");
    let seed_cache = SymbolCache::new(PersistentCache::open(seed.path().join("cache"), true));
    let request = SymbolQuery::new().capabilities([Capability::Occurrences]);
    query(&seed_cache, &seed_path, &request);
    let seed_json: Value = serde_json::from_slice(&std::fs::read(entry(&seed)).expect("seed entry")).expect("JSON");
    for (extension, written) in [("c", "void f(){}"), ("cpp", "void f(){}"), ("lua", "function f() end"),
        ("pl", "sub f {}"), ("sh", "f(){ :; }"), ("zsh", "f(){ :; }")] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(format!("empty.{extension}"));
        std::fs::write(&path, written).expect("fixture");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let (_, control) = query(&cache, &path, &request); assert!(control.index.imports.is_empty());
        assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
        let path_to_entry = entry(&dir); let mut changed: Value = serde_json::from_slice(&std::fs::read(&path_to_entry).expect("entry")).expect("JSON");
        changed["imports"] = json!([seed_json["imports"][0].clone()]);
        std::fs::write(&path_to_entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
        let (lookup, repaired) = query(&cache, &path, &request); assert!(matches!(lookup, CacheLookup::Miss(_)), "{extension}");
        assert!(repaired.index.imports.is_empty()); assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
    }
}

#[test]
fn imports_json_metadata_validation_precedes_every_capability_and_kind_projection() {
    for capability in [Capability::Declarations, Capability::References, Capability::Occurrences, Capability::Semantics] {
        for kinds in [None, Some([SymbolKindV2::Function]), Some([SymbolKindV2::Type])] {
            let dir = TempDir::new().expect("temp"); let path = dir.path().join("case.ts");
            std::fs::write(&path, "namespace N { export function S(){D();} }\nimport D /*c*/ = N. /*c*/ S;\nclass C{}\n").expect("source");
            let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
            let mut request = SymbolQuery::new().capabilities([capability]); if let Some(kinds) = kinds { request = request.kinds(kinds); }
            let (_, control) = query(&cache, &path, &request);
            assert_eq!(control.index.imports[0].symbol.qualified_name.as_deref(), Some("N.S"));
            assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
            let entry = entry(&dir); let written: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
            for field in ["imports", "alias", "source", "symbol/qualified_name"] {
                let mut changed = written.clone();
                if field == "imports" { changed["imports"] = json!([]); }
                else { *changed.pointer_mut(&format!("/imports/0/{field}")).expect("field") = if field == "source" { json!("invented") } else { Value::Null }; }
                std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
                let (lookup, repaired) = query(&cache, &path, &request); assert!(matches!(lookup, CacheLookup::Miss(_)), "{capability:?} {kinds:?} {field}: {lookup:?}");
                assert_eq!(repaired.index.imports[0].symbol.qualified_name.as_deref(), Some("N.S"));
                assert_eq!(repaired.index.imports[0].alias.as_deref(), Some("D")); assert!(repaired.index.imports[0].source.is_none());
                if capability == Capability::Semantics && kinds.is_none() {
                    let owner = repaired.index.symbols.iter().find(|record| record.identity.name == "S").expect("owner");
                    assert_eq!(owner.semantics.is_recursive, Some(TriState::Unknown));
                    assert_eq!(owner.relations.dependencies[0].qualified_name.as_deref(), Some("N.S"));
                }
                assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
            }
        }
    }
}

#[test]
fn imports_json_legitimate_nullable_and_empty_controls_are_actual_hits() {
    for (file, written, expected) in [
        ("lifetime.rs", "fn f<'a>() {}\nextern /*one*/ crate\n x /*two*/ as y;\n", 1),
        ("static.rs", "const X: &'static str = \"s\";\nextern crate x as y;\n", 1),
        ("future.py", "from __future__ import (\n  annotations, # comment\n)\n", 1),
        ("require.ts", "import q /*c*/ = require('m');\n", 1),
        ("entity.ts", "import D /*c*/ = N. /*c*/ S;\n", 1),
        ("generic.cs", "using D = A.B<C>;\n", 1),
        ("predefined.cs", "using D = int;\n", 1),
        ("unalias.rs", "use a;\n", 1),
        ("root.rs", "use ::a;\n", 1),
        ("self_glob.rs", "use a::{self, *};\n", 1),
        ("self_alias.rs", "use a::{self as D};\n", 1),
        ("unalias.py", "import a.b\n", 1),
        ("dot.go", "package p\nimport . `a/f`\n", 1),
        ("wild.java", "import a.b.*; class C{}", 1),
        ("wild.scala", "import a.b._\n", 1),
        ("side.ts", "import 'm';\n", 0),
        ("empty.rs", "use a::{};\n", 0),
        ("hidden.scala", "import a.{b => _}\n", 0),
        ("trait.php", "<?php trait T{} class C { use T; }", 0),
        ("declaration.cs", "class C { void M(){using Stream stream = null;} }", 0),
        ("regex.js", "const r = /import f/; function f(){}", 0),
        ("regex.ts", "const r = /import f/; function f(){}", 0),
        ("triple.py", "s = \"\"\"\nfrom a import f\n\"\"\"\ndef f(): pass\n", 0),
        ("heredoc.php", "<?php $s = <<<TXT\nuse A\\B;\nTXT;\nfunction f(){}", 0),
        ("nowdoc.php", "<?php $s = <<<'TXT'\nuse A\\B;\nTXT;\nfunction f(){}", 0),
        ("raw.rs", "const S: &str = r#\" quote \" use a::f; \"#; fn f(){}", 0),
        ("macro.rs", "macro_rules! m { () => { use a::f; } } fn f(){}", 0),
        ("macro_call.rs", "m!{use a::f;} fn f(){}", 0),
        ("macro_parens.rs", "m!(use a::f;); fn f(){}", 0),
        ("macro_brackets.rs", "m![use a::f;]; fn f(){}", 0),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(file); std::fs::write(&path, written).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let request = SymbolQuery::new().capabilities([Capability::Occurrences]);
        let (_, control) = query(&cache, &path, &request); assert_eq!(control.index.imports.len(), expected, "{file}");
        let before = work_counters(); let (lookup, warm) = query(&cache, &path, &request);
        assert_eq!(lookup, CacheLookup::Hit, "{file}"); assert_eq!(work_counters().since(before).parser_constructions, 0);
        assert_eq!(warm.index.imports.len(), expected, "{file}");
    }
}

#[test]
fn imports_json_nullable_statement_producers_still_validate_metadata() {
    for (file, written, name, alias, original, module) in [
        ("extern.rs", "fn f<'a>(){}\nextern /*c*/ crate\n x /*c*/ as y;\n", "y", Some("y"), Some("x"), None),
        ("future.py", "from __future__ import (\n annotations, # comment\n)\n", "annotations", None, None, Some("__future__")),
        ("plain.rs", "use a;\n", "a", None, None, None),
        ("root.rs", "use ::a;\n", "a", None, None, None),
        ("self_glob.rs", "use a::{self, *};\n", "a", None, None, None),
        ("self_alias.rs", "use a::{self as D};\n", "D", Some("D"), Some("a"), None),
        ("plain.py", "import a.b\n", "a", None, None, Some("a.b")),
        ("alias.py", "import a.b as c\n", "c", Some("c"), Some("a.b"), Some("a.b")),
        ("plain.ts", "import D from 'm';\n", "D", None, None, Some("m")),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(file); std::fs::write(&path, written).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let request = SymbolQuery::new().capabilities([Capability::Declarations]);
        let (_, control) = query(&cache, &path, &request); let import = &control.index.imports[0];
        assert_eq!(import.name, name); assert_eq!(import.alias.as_deref(), alias);
        assert_eq!(import.source.as_deref(), module); assert_eq!(import.symbol.qualified_name.as_deref(), original);
        let entry = entry(&dir); let bytes = std::fs::read(&entry).expect("entry"); let saved: Value = serde_json::from_slice(&bytes).expect("JSON");
        for field in ["alias", "source", "symbol/qualified_name"] {
            for replacement in [Value::Null, json!(""), json!("contradictory"), json!(name)] {
                let pointer = format!("/imports/0/{field}"); let saved_value = saved.pointer(&pointer).expect("field");
                let mut changed = saved.clone(); *changed.pointer_mut(&pointer).expect("field") = replacement.clone();
                std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
                let (lookup, repaired) = query(&cache, &path, &request);
                if replacement == *saved_value { assert_eq!(lookup, CacheLookup::Hit, "{file} {field}"); }
                else { assert!(matches!(lookup, CacheLookup::Miss(_)), "{file} {field} {replacement}: {lookup:?}"); }
                let import = &repaired.index.imports[0];
                assert_eq!(import.name, name); assert_eq!(import.alias.as_deref(), alias);
                assert_eq!(import.source.as_deref(), module); assert_eq!(import.symbol.qualified_name.as_deref(), original);
                assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
            }
        }
    }
}

#[test]
fn imports_json_division_before_later_import_does_not_hide_deleted_evidence() {
    let dir = TempDir::new().expect("temp"); let path = dir.path().join("division.ts");
    std::fs::write(&path, "const a=4,b=2; const x=a/b; import {f as D} from 'a/b'; function f(){D();}").expect("source");
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true)); let request = SymbolQuery::new();
    let (_, control) = query(&cache, &path, &request); assert_eq!(control.index.imports[0].source.as_deref(), Some("a/b"));
    let entry = entry(&dir); let mut changed: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
    changed["imports"] = json!([]); std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
    let (lookup, repaired) = query(&cache, &path, &request); assert!(matches!(lookup, CacheLookup::Miss(_)));
    assert_eq!(repaired.index.imports[0].source.as_deref(), Some("a/b")); assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
}

#[test]
fn imports_json_absolute_paths_and_encoded_originals_keep_hits_and_repair_fact_loss() {
    for (file, written, module, original) in [
        ("absolute.rs", "use ::a::f as D; fn f(){D();}", "::a", "f"),
        ("absolute.php", "<?php use \\A\\B as D; function f(){}", "\\A\\B", "B"),
        ("raw_name.rs", "use m::r#f as D; fn f(){D();}", "m", "r#f"),
        ("escaped.ts", r"import { \u0066 as D } from 'm'; function f(){D();}", "m", r"\u0066"),
        ("codepoint.ts", r"import { \u{66} as D } from 'm'; function f(){D();}", "m", r"\u{66}"),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(file); std::fs::write(&path, written).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true)); let request = SymbolQuery::new();
        let (_, control) = query(&cache, &path, &request); assert_eq!(control.index.imports.len(), 1);
        assert_eq!(control.index.imports[0].source.as_deref(), Some(module));
        assert_eq!(control.index.imports[0].symbol.qualified_name.as_deref(), Some(original));
        assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit, "{file}");
        let entry = entry(&dir); let saved: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        for field in ["imports", "source", "symbol/qualified_name"] {
            let mut changed = saved.clone(); if field == "imports" { changed["imports"] = json!([]); }
            else { *changed.pointer_mut(&format!("/imports/0/{field}")).expect("field") = Value::Null; }
            std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
            let (lookup, repaired) = query(&cache, &path, &request); assert!(matches!(lookup, CacheLookup::Miss(_)), "{file} {field}: {lookup:?}");
            assert_eq!(repaired.index.imports[0].source.as_deref(), Some(module)); assert_eq!(repaired.index.imports[0].symbol.qualified_name.as_deref(), Some(original));
            assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
        }
    }
}

#[test]
fn imports_json_missing_separate_statement_is_recomputed() {
    let written = "import { f as D } from 'a'; import { g as E } from 'b'; function f(){}";
    let dir = TempDir::new().expect("temp"); let path = dir.path().join("separate.ts"); std::fs::write(&path, written).expect("source");
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true)); let request = SymbolQuery::new();
    let (_, control) = query(&cache, &path, &request);
    assert_eq!(control.index.imports.iter().map(|import| (import.name.as_str(), import.source.as_deref(), import.symbol.qualified_name.as_deref())).collect::<Vec<_>>(), [("D", Some("a"), Some("f")), ("E", Some("b"), Some("g"))]);
    let entry = entry(&dir); let mut changed: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
    changed["imports"].as_array_mut().expect("imports").remove(1);
    std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
    let (lookup, repaired) = query(&cache, &path, &request); assert!(matches!(lookup, CacheLookup::Miss(_)));
    assert_eq!(repaired.index.imports.iter().map(|import| (import.name.as_str(), import.source.as_deref(), import.symbol.qualified_name.as_deref())).collect::<Vec<_>>(), [("D", Some("a"), Some("f")), ("E", Some("b"), Some("g"))]);
    assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
}

#[test]
fn imports_json_missing_grouped_binding_is_recomputed() {
    for (extension, written, modules, originals) in [
        ("rs", "use a::{f as D, g as E}; fn f(){}", ["a", "a"], ["f", "g"]),
        ("ts", "import { f as D, g as E } from 'a'; function f(){}", ["a", "a"], ["f", "g"]),
        ("js", "import { f as D, g as E } from 'a'; function f(){}", ["a", "a"], ["f", "g"]),
        ("py", "from a import f as D, g as E\ndef f(): pass\n", ["a", "a"], ["f", "g"]),
        ("go", "package p\nimport ( D \"a/f\"; E \"b/g\" )\nfunc f(){}", ["a/f", "b/g"], ["f", "g"]),
        ("scala", "import a.{f => D, g => E}\ndef f()=1\n", ["a.f", "a.g"], ["f", "g"]),
        ("php", "<?php use A\\{F as D, G as E}; function f(){}", ["A\\F", "A\\G"], ["F", "G"]),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(format!("grouped.{extension}"));
        std::fs::write(&path, written).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true)); let request = SymbolQuery::new();
        let (_, control) = query(&cache, &path, &request);
        assert_eq!(control.index.imports.iter().map(|import| (import.name.as_str(), import.source.as_deref(), import.symbol.qualified_name.as_deref())).collect::<Vec<_>>(), [("D", Some(modules[0]), Some(originals[0])), ("E", Some(modules[1]), Some(originals[1]))], "{extension}");
        assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit, "valid {extension}");
        let entry = entry(&dir); let mut changed: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        changed["imports"].as_array_mut().expect("imports").remove(1);
        std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
        let (lookup, repaired) = query(&cache, &path, &request);
        assert!(matches!(lookup, CacheLookup::Miss(_)), "{extension}");
        assert_eq!(repaired.index.imports.iter().map(|import| (import.name.as_str(), import.source.as_deref(), import.symbol.qualified_name.as_deref())).collect::<Vec<_>>(), [("D", Some(modules[0]), Some(originals[0])), ("E", Some(modules[1]), Some(originals[1]))]);
        assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
        let saved: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        for replacement in [None, Some(Value::Null), Some(json!("")), Some(json!("changed"))] {
            let mut changed = saved.clone();
            if let Some(value) = replacement { changed["imports"][1]["source"] = value; }
            else { changed["imports"][1].as_object_mut().expect("import").remove("source"); }
            std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
            let (lookup, repaired) = query(&cache, &path, &request); assert!(matches!(lookup, CacheLookup::Miss(_)), "source {extension}");
            assert_eq!(repaired.index.imports.iter().map(|import| (import.name.as_str(), import.source.as_deref(), import.symbol.qualified_name.as_deref())).collect::<Vec<_>>(), [("D", Some(modules[0]), Some(originals[0])), ("E", Some(modules[1]), Some(originals[1]))]);
            assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
        }
    }
}

#[test]
fn imports_json_source_consistent_statement_extension_is_recomputed() {
    let written = "import { f as D } from 'a'; function f(){}";
    let dir = TempDir::new().expect("temp"); let path = dir.path().join("extended.ts"); std::fs::write(&path, written).expect("source");
    let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true)); let request = SymbolQuery::new();
    let (_, control) = query(&cache, &path, &request);
    assert_eq!(control.index.imports[0].statement_span.as_ref().expect("statement").end_byte as usize, written.find(';').expect("end") + 1);
    let entry = entry(&dir); let mut changed: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
    changed["imports"][0]["statement_span"]["end_byte"] = json!(written.len());
    changed["imports"][0]["statement_span"]["end"] = json!({"line":1,"column":written.len()+1});
    std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
    let (lookup, repaired) = query(&cache, &path, &request);
    assert!(matches!(lookup, CacheLookup::Miss(_)));
    assert_eq!(repaired.index.imports[0].statement_span.as_ref().expect("statement").end_byte as usize, written.find(';').expect("end") + 1);
    assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
}

#[test]
fn imports_json_producer_inventory_witness_is_required_and_strict() {
    for case in CASES.iter().copied() {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(format!("witness.{}", case.0));
        std::fs::write(&path, case.1).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let request = SymbolQuery::new().capabilities([Capability::Occurrences]);
        let (_, control) = query(&cache, &path, &request); assert_facts(&control, case);
        let entry = entry(&dir); let saved: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        for shape in ["control", "absent", "null", "number", "object", "list", "empty", "changed", "duplicate", "trailing"] {
            let mut changed = saved.clone(); let mut malformed = None;
            match shape {
                "control" => {}
                "absent" => { changed.as_object_mut().expect("object").remove("import_inventory"); }
                "null" => changed["import_inventory"] = Value::Null,
                "number" => changed["import_inventory"] = json!(123),
                "object" => changed["import_inventory"] = json!({}),
                "list" => changed["import_inventory"] = json!([123]),
                "empty" => changed["import_inventory"] = json!(""),
                "changed" => changed["import_inventory"] = json!("0".repeat(64)),
                "duplicate" => malformed = Some(format!("{{\"import_inventory\":{},{}", saved["import_inventory"], &serde_json::to_string(&saved).expect("JSON")[1..])),
                "trailing" => malformed = Some(format!("{} garbage", serde_json::to_string(&saved).expect("JSON"))),
                _ => unreachable!(),
            }
            std::fs::write(&entry, malformed.unwrap_or_else(|| serde_json::to_string(&changed).expect("JSON"))).expect("edit");
            let (lookup, repaired) = query(&cache, &path, &request);
            if shape == "control" { assert_eq!(lookup, CacheLookup::Hit); }
            else { assert!(matches!(lookup, CacheLookup::Miss(_)), "{} {shape}", case.0); }
            assert_facts(&repaired, case);
            let before = work_counters(); assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
            assert_eq!(work_counters().since(before).parser_constructions, 0);
        }
    }
}

#[test]
fn imports_json_mixed_and_type_only_inventories_restore_every_binding() {
    for (file, written, expected) in [
        ("mixed.ts", "import D,{g as E} from 'a'; function f(){}", vec![("D", None, Some("a"), None), ("E", Some("E"), Some("a"), Some("g"))]),
        ("mixed.js", "import D,* as E from 'a'; function f(){}", vec![("D", None, Some("a"), None), ("E", Some("E"), Some("a"), Some("*"))]),
        ("mixed.py", "import a as D,b as E\ndef f(): pass\n", vec![("D", Some("D"), Some("a"), Some("a")), ("E", Some("E"), Some("b"), Some("b"))]),
        ("mixed.php", "<?php use A\\F as D,B\\G as E; function f(){}", vec![("D", Some("D"), Some("A\\F"), Some("F")), ("E", Some("E"), Some("B\\G"), Some("G"))]),
        ("mixed.scala", "import a.f,b.g\ndef f()=1\n", vec![("f", None, Some("a.f"), None), ("g", None, Some("b.g"), None)]),
        ("type.ts", "import type {F} from 'a'; function f(){}", vec![("F", None, Some("a"), None)]),
        ("commented.ts", "import /*a*/ type /*b*/ {F} from 'a'; function f(){}", vec![("F", None, Some("a"), None)]),
        ("specifier.ts", "import {type F} from 'a'; function f(){}", vec![("F", None, Some("a"), None)]),
        ("default.ts", "import type F from 'a'; function f(){}", vec![("F", None, Some("a"), None)]),
        ("namespace.ts", "import type * as F from 'a'; function f(){}", vec![("F", Some("F"), Some("a"), Some("*"))]),
        ("require.ts", "import type F = require('a'); function f(){}", vec![("F", None, Some("a"), None)]),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(file); std::fs::write(&path, written).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let request = SymbolQuery::new().capabilities([Capability::Occurrences]).kinds([SymbolKindV2::Function]);
        let facts = |result: &SymbolQueryResult| {
            assert_eq!(result.index.imports.iter().map(|i| (i.name.as_str(), i.alias.as_deref(), i.source.as_deref(), i.symbol.qualified_name.as_deref())).collect::<Vec<_>>(), expected, "{file}");
            for i in &result.index.imports {
                assert_eq!(&written[i.span.start_byte as usize..i.span.end_byte as usize], i.name);
                assert!(!result.index.occurrences.iter().any(|o| o.span.start_byte == i.span.start_byte), "{file}: imported declaration token is not a read");
            }
        };
        let (_, control) = query(&cache, &path, &request); facts(&control);
        let entry = entry(&dir); let saved: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        for remove_all in [false, true] {
            let mut changed = saved.clone(); if remove_all { changed["imports"] = json!([]); }
            else { changed["imports"].as_array_mut().expect("imports").pop(); }
            std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
            let (lookup, repaired) = query(&cache, &path, &request); assert!(matches!(lookup, CacheLookup::Miss(_)), "{file}"); facts(&repaired);
            assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
            let before = work_counters(); let (lookup, semantic) = query(&cache, &path, &SymbolQuery::new().capabilities([Capability::Semantics]).kinds([SymbolKindV2::Function]));
            assert_eq!(lookup, CacheLookup::WidenedCapabilities { parsed: false }); assert_eq!(work_counters().since(before).parser_constructions, 0); facts(&semantic);
            let before = work_counters(); let (lookup, wide) = query(&cache, &path, &SymbolQuery::complete_index());
            assert_eq!(lookup, CacheLookup::WidenedKinds); assert_eq!(work_counters().since(before).parser_constructions, 1); facts(&wide);
            assert_eq!(query(&cache, &path, &SymbolQuery::complete_index()).0, CacheLookup::Hit);
            let (lookup, projected) = query(&cache, &path, &request); assert_eq!(lookup, CacheLookup::Hit); facts(&projected);
        }
    }
}

#[test]
fn imports_json_actual_statement_boundaries_and_roles_survive_every_projection() {
    for case in CASES[..10].iter().copied() {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(format!("boundary.{}", case.0));
        let written = format!("\n{}", case.1); std::fs::write(&path, &written).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let request = SymbolQuery::new().capabilities([Capability::Occurrences]);
        let (_, control) = query(&cache, &path, &request);
        assert_facts_for_source(&control, case, &written);
        let statement = control.index.imports[0].statement_span.as_ref().expect("statement");
        let keyword = match case.0 { "rs" | "php" => "use ", "cs" => "using ", "py" => "from ", _ => "import " };
        assert_eq!(statement.start_byte as usize, written.find(keyword).expect("keyword"));
        let point = |byte: usize| { let before = &written[..byte]; json!({"line":before.matches('\n').count()+1,"column":byte-before.rfind('\n').map_or(0, |at| at+1)+1}) };
        let entry = entry(&dir); let saved: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        for capability in [Capability::Declarations, Capability::References, Capability::Occurrences, Capability::Semantics] {
            for kind in [SymbolKindV2::Function, SymbolKindV2::Class] {
                for mutation in ["start", "end", "role"] {
                    let mut changed = saved.clone();
                    match mutation {
                        "start" => { changed["imports"][0]["statement_span"]["start_byte"] = json!(0); changed["imports"][0]["statement_span"]["start"] = point(0); }
                        "end" => { changed["imports"][0]["statement_span"]["end_byte"] = json!(written.len()); changed["imports"][0]["statement_span"]["end"] = point(written.len()); }
                        "role" => {
                            let name_start = written.rfind("f(").expect("function"); let span = json!({"start_byte":name_start,"end_byte":name_start+1,"start":point(name_start),"end":point(name_start+1)});
                            changed["imports"].as_array_mut().expect("imports").push(json!({"name":"f","alias":null,"source":null,"span":span,"statement_span":span,"symbol":{"name":"f","qualified_name":null,"id":null,"source_span":null}}));
                        }
                        _ => unreachable!(),
                    }
                    std::fs::write(&entry, serde_json::to_vec(&changed).expect("JSON")).expect("edit");
                    let narrowed = SymbolQuery::new().capabilities([capability]).kinds([kind]);
                    let (lookup, repaired) = query(&cache, &path, &narrowed); assert!(matches!(lookup, CacheLookup::Miss(_)), "{} {mutation} {capability:?} {kind:?}", case.0);
                    assert_facts_for_source(&repaired, case, &written);
                    assert_eq!(query(&cache, &path, &narrowed).0, CacheLookup::Hit);
                }
            }
        }
    }
}

#[test]
fn imports_json_later_php_sources_walk_the_complete_metadata_shape_matrix() {
    for (written, expected) in [
        ("<?php use A\\F as D,B\\G as E; function f(){}", vec![("D", Some("D"), "A\\F", Some("F")), ("E", Some("E"), "B\\G", Some("G"))]),
        ("<?php use A\\F,B\\G; function f(){}", vec![("F", None, "A\\F", None), ("G", None, "B\\G", None)]),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join("later.php"); std::fs::write(&path, written).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let request = SymbolQuery::new().capabilities([Capability::Occurrences]);
        let facts = |result: &SymbolQueryResult| {
            assert_eq!(result.index.imports.iter().map(|i| (i.name.as_str(), i.alias.as_deref(), i.source.as_deref().expect("module"), i.symbol.qualified_name.as_deref())).collect::<Vec<_>>(), expected);
            assert_eq!(result.index.imports[1].span.start_byte as usize, written.find(expected[1].0).expect("later token"));
        };
        let (_, control) = query(&cache, &path, &request); facts(&control);
        let entry = entry(&dir); let saved: Value = serde_json::from_slice(&std::fs::read(&entry).expect("entry")).expect("JSON");
        for shape in ["control", "absent", "null", "wrong", "object", "list", "empty", "changed", "duplicate", "trailing"] {
            let mut changed = saved.clone(); let mut malformed = None;
            match shape {
                "control" => {}
                "absent" => { changed["imports"][1].as_object_mut().expect("import").remove("source"); }
                "null" => changed["imports"][1]["source"] = Value::Null,
                "wrong" => changed["imports"][1]["source"] = json!(123),
                "object" => changed["imports"][1]["source"] = json!({}),
                "list" => changed["imports"][1]["source"] = json!([123]),
                "empty" => changed["imports"][1]["source"] = json!(""),
                "changed" => changed["imports"][1]["source"] = json!("invented"),
                "duplicate" => {
                    let old = serde_json::to_string(&saved["imports"][1]).expect("JSON");
                    let new = format!("{{\"source\":\"B\\\\G\",{}", &old[1..]);
                    malformed = Some(serde_json::to_string(&saved).expect("JSON").replacen(&old, &new, 1));
                }
                "trailing" => malformed = Some(format!("{} garbage", serde_json::to_string(&saved).expect("JSON"))),
                _ => unreachable!(),
            }
            std::fs::write(&entry, malformed.unwrap_or_else(|| serde_json::to_string(&changed).expect("JSON"))).expect("edit");
            let (lookup, repaired) = query(&cache, &path, &request);
            if shape == "control" { assert_eq!(lookup, CacheLookup::Hit); }
            else { assert!(matches!(lookup, CacheLookup::Miss(_)), "{shape}"); }
            facts(&repaired); assert_eq!(query(&cache, &path, &request).0, CacheLookup::Hit);
            let before = work_counters(); let (lookup, semantic) = query(&cache, &path, &SymbolQuery::new().capabilities([Capability::Semantics]));
            assert_eq!(lookup, CacheLookup::WidenedCapabilities { parsed: false }); assert_eq!(work_counters().since(before).parser_constructions, 0); facts(&semantic);
        }
    }
}

#[test]
fn java_single_segment_on_demand_imports_keep_public_facts_and_six_cache_stages() {
    use tree_hugger::{TreeFile, Confidence, CoverageState, LimitReason, OccurrenceKind};
    for (prefix, name, module, original, alias) in [
        ("import p.*;", "p", "p", Some("*"), Some("p")),
        ("import static p.*;", "p", "p", Some("*"), Some("p")),
        ("import p /*c*/ . /*d*/ *;", "p", "p", Some("*"), Some("p")),
        ("import static p /*c*/ . /*d*/ *;", "p", "p", Some("*"), Some("p")),
        ("import p.q.*;", "q", "p.q", Some("*"), Some("q")),
        ("import static p.C.*;", "C", "p.C", Some("*"), Some("C")),
        ("import p.C;", "C", "p", None, None),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join("on_demand.java");
        let written = format!("{prefix}\nclass {name} {{ void f() {{ {name} x; f(); missing(); }} }}");
        std::fs::write(&path, &written).expect("source"); let tree = TreeFile::new(&path).expect("tree");
        assert!(tree.syntax_diagnostics().is_empty());
        let imported = tree.imported_symbols().expect("imports"); assert_eq!(imported.len(), 1); assert_eq!(imported[0].name, name);
        assert_eq!(imported[0].source.as_deref(), Some(module)); assert_eq!(imported[0].alias.as_deref(), alias); assert_eq!(imported[0].original_name.as_deref(), original);
        let reads = tree.referenced_symbols().expect("references");
        assert_eq!(reads.iter().filter(|r| r.name == name).count(), 1, "body read only: {prefix}");
        if name != "p" { assert!(reads.iter().any(|r| r.name == "p"), "qualifier survives: {prefix}"); }
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let narrow = SymbolQuery::new().kinds([SymbolKindV2::Method]).capabilities([Capability::Occurrences]);
        let semantic = SymbolQuery::new().kinds([SymbolKindV2::Method]).capabilities([Capability::Semantics]); let full = SymbolQuery::complete_index();
        for (stage, request, expected, parses) in [
            ("narrow miss", &narrow, CacheLookup::Miss(CacheMiss::Absent), 1), ("narrow hit", &narrow, CacheLookup::Hit, 0),
            ("semantic widening", &semantic, CacheLookup::WidenedCapabilities { parsed: false }, 0),
            ("kind widening", &full, CacheLookup::WidenedKinds, 1), ("complete hit", &full, CacheLookup::Hit, 0), ("projected hit", &semantic, CacheLookup::Hit, 0),
        ] {
            let before = work_counters(); let (lookup, result) = query(&cache, &path, request);
            assert_eq!(lookup, expected, "{prefix} {stage}"); assert_eq!(work_counters().since(before).parser_constructions, parses);
            assert_eq!(result.index.imports.len(), 1); let import = &result.index.imports[0];
            assert_eq!((import.name.as_str(), import.source.as_deref(), import.symbol.qualified_name.as_deref(), import.alias.as_deref()), (name, Some(module), original, alias));
            let binding = prefix.rfind(name).expect("bound token"); assert_eq!(import.span.start_byte as usize, binding); assert_eq!(import.span.end_byte as usize, binding + name.len());
            assert_eq!(import.statement_span.as_ref().expect("statement").end_byte as usize, prefix.len());
            assert!(!result.index.occurrences.iter().any(|o| o.span.start_byte as usize == binding));
            assert_eq!(result.index.occurrences.iter().filter(|o| o.target.name == name).count(), 1, "{prefix} {stage}: body type read survives");
            let read = result.index.occurrences.iter().find(|o| o.target.name == name).expect("body type"); assert_eq!(read.kind, OccurrenceKind::Usage);
            assert_eq!(read.confidence, Confidence::NameOnly);
            assert!(read.target.id.is_none()); assert_eq!(read.candidates.len(), 1); assert_eq!(read.candidates[0].name, name);
            let missing = result.index.occurrences.iter().find(|o| o.target.name == "missing").expect("unknown call"); assert_eq!(missing.confidence, Confidence::Unresolved);
            if name != "p" { assert!(result.index.occurrences.iter().any(|o| o.target.name == "p" && o.span.start_byte < import.span.start_byte)); }
            assert!(result.index.reference_coverage.iter().all(|c| c.state == CoverageState::Limited && c.reasons.contains(&LimitReason::UnsupportedLanguage)));
            for record in &result.index.symbols { assert!(record.relations.references.is_empty()); assert!(record.relations.referenced_by.is_empty()); assert!(record.relations.dependencies.is_empty()); }
            if stage != "narrow miss" && stage != "narrow hit" { let owner = result.index.symbols.iter().find(|s| s.identity.name == "f").expect("method"); assert_eq!(owner.semantics.is_recursive, Some(TriState::Unknown)); }
        }
    }
}

#[test]
fn on_demand_and_single_name_import_producers_preserve_parity() {
    for case in [
        ("rs", "use a::*; fn f(){}", "a", Some("a"), Some("a"), Some("*")), ("rs", "use a::{*}; fn f(){}", "a", Some("a"), Some("a"), Some("*")),
        ("scala", "import a._\ndef f()=1\n", "a", Some("a"), Some("a"), Some("*")), ("scala", "import a.*\ndef f()=1\n", "a", Some("a"), Some("a"), Some("*")), ("scala", "import a.{_}\ndef f()=1\n", "a", Some("a"), Some("a"), Some("*")),
        ("py", "from a import *\ndef f(): pass\n", "a", Some("a"), Some("a"), Some("*")), ("go", "package p\nimport . \"a\"\nfunc f(){}\n", "a", Some("a"), Some("a"), Some("*")),
        ("ts", "import * as p from 'a'; function f(){}", "p", Some("p"), Some("a"), Some("*")), ("js", "import * as p from 'a'; function f(){}", "p", Some("p"), Some("a"), Some("*")),
        ("cs", "using static C; class W { void f(){} }", "C", None, Some("C"), None), ("php", "<?php use A; function f(){}", "A", None, Some("A"), None), ("swift", "import A\nfunc f(){}\n", "A", None, Some("A"), None),
    ] {
        let dir = TempDir::new().expect("temp"); let path = dir.path().join(format!("parity.{}", case.0)); std::fs::write(&path, case.1).expect("source");
        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true)); let request = SymbolQuery::complete_index();
        let (lookup, result) = query(&cache, &path, &request); assert_eq!(lookup, CacheLookup::Miss(CacheMiss::Absent)); assert_eq!(result.index.imports.len(), 1, "{case:?}"); let import = &result.index.imports[0];
        assert_eq!((import.name.as_str(), import.alias.as_deref(), import.source.as_deref(), import.symbol.qualified_name.as_deref()), (case.2, case.3, case.4, case.5), "{case:?}");
        let before = work_counters(); let (lookup, warm) = query(&cache, &path, &request); assert_eq!(lookup, CacheLookup::Hit); assert_eq!(work_counters().since(before).parser_constructions, 0); assert_eq!(serde_json::to_value(&warm.index.imports).expect("imports"), serde_json::to_value(&result.index.imports).expect("imports"));
    }
}

#[test]
fn java_single_segment_on_demand_imports_keep_the_strict_json_shape_matrix() {
    run_matrix(("java", "import p.*; class W { void f(){} }", "p", Some("p"), Some("p"), Some("*")));
}

#[test]
fn java_static_single_segment_on_demand_imports_keep_the_strict_json_shape_matrix() {
    run_matrix(("java", "import static p.*; class W { void f(){} }", "p", Some("p"), Some("p"), Some("*")));
}

#[test]
fn java_commented_single_segment_on_demand_imports_keep_the_strict_json_shape_matrix() {
    run_matrix(("java", "import p /*c*/ . /*d*/ *; class W { void f(){} }", "p", Some("p"), Some("p"), Some("*")));
}
