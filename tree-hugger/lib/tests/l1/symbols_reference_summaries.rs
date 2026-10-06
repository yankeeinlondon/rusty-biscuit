//! What the reference occurrences feed: the `references`, `dependencies`
//! and `referenced_by` summaries, `semantics.is_recursive`, and the
//! `Occurrences` capability (whole-file evidence whatever kinds are
//! selected, one references run and one resolution per analysis). User
//! contract: `tree-hugger/docs/references.md`.

use std::io::Write;

use tree_hugger::{
    Capability, Confidence, FileSymbolIndex, OccurrenceKind, ProgrammingLanguage, SymbolKindV2, SymbolQuery,
    SymbolRecord, SymbolRef, TreeFile, TriState, WorkCounters, work_counters,
};

use crate::symbols_fixtures::{all_fixtures, display_name, primary_fixture};

fn counted<T>(work: impl FnOnce() -> T) -> (T, WorkCounters) {
    let before = work_counters();
    let value = work();
    (value, work_counters().since(before))
}

/// `source` parsed from a temporary `.{extension}` file.
fn parse(source: &str, extension: &str) -> (tempfile::NamedTempFile, TreeFile) {
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .expect("create temp source");
    file.write_all(source.as_bytes()).expect("write temp source");
    let tree_file = TreeFile::new(file.path()).expect("parse inline source");
    (file, tree_file)
}

fn complete(source: &str, extension: &str) -> FileSymbolIndex {
    parse(source, extension).1.symbol_index_v2().expect("complete index")
}

/// The only record called `name` of a function-like kind, else the only
/// record called `name`.
fn record<'a>(index: &'a FileSymbolIndex, name: &str) -> &'a SymbolRecord {
    let named: Vec<&SymbolRecord> = index.symbols.iter().filter(|record| record.identity.name == name).collect();
    let callables: Vec<&SymbolRecord> = named
        .iter()
        .copied()
        .filter(|record| matches!(record.kind, SymbolKindV2::Function | SymbolKindV2::Method | SymbolKindV2::Constructor))
        .collect();
    match (callables.as_slice(), named.as_slice()) {
        ([only], _) | ([], [only]) => only,
        _ => panic!("no single record `{name}`: {named:#?}"),
    }
}

/// Each reference as `(name, target ID present)`.
fn names(refs: &[SymbolRef]) -> Vec<(&str, bool)> {
    refs.iter().map(|reference| (reference.name.as_str(), reference.id.is_some())).collect()
}

/// `(label, extension, source, expected is_recursive by callable name)`.
type RecursionRow = (&'static str, &'static str, &'static str, &'static [(&'static str, TriState)]);

use TriState::{No, Unknown, Yes};

/// The direct-recursion rules, one row per rule: `Yes` only for a `Scoped`
/// call to the callable's own ID (it wins over any other uncertainty); `No`
/// only with complete call coverage and no call that might be to itself;
/// otherwise `Unknown`.
const RECURSION_MATRIX: &[RecursionRow] = &[
    (
        "direct self-call",
        "rs",
        "fn countdown(n: u32) -> u32 { if n == 0 { 0 } else { countdown(n - 1) } }\nfn leaf() -> u32 { 1 }\n",
        &[("countdown", Yes), ("leaf", No)],
    ),
    ("self-value reference is not a call", "rs", "fn f() { let _keep = f; }\n", &[("f", No)]),
    ("call through a parameter", "rs", "fn f(callback: fn()) { callback(); }\n", &[("f", Unknown)]),
    (
        "a variable holding the function is not the function",
        "rs",
        "fn f() { let again = f; again(); }\n",
        &[("f", Unknown)],
    ),
    (
        "proven self-call wins over an indirect call",
        "rs",
        "fn f(callback: fn()) { callback(); f(callback); }\n",
        &[("f", Yes)],
    ),
    ("method call on a receiver", "rs", "struct W;\nimpl W { fn m(&self) { self.m(); } }\n", &[("m", Unknown)]),
    (
        "`Self::f` establishes the one inherent method",
        "rs",
        "struct W;\nimpl W { fn build() -> W { Self::build() } }\n",
        &[("build", Yes)],
    ),
    (
        "unresolved call of the callable's own name",
        "rs",
        "struct W;\nimpl W { fn m(&self) { m(); } }\n",
        &[("m", Unknown)],
    ),
    (
        "a call whose target no capture names",
        "rs",
        "fn f() { (pick())(); }\nfn pick() -> fn() { f }\n",
        &[("f", Unknown), ("pick", No)],
    ),
    ("unexpanded macro", "rs", "fn f() { println!(\"x\"); }\n", &[("f", Unknown)]),
    ("parse error", "rs", "fn f() { let x = ; }\nfn g() {}\n", &[("f", Unknown), ("g", No)]),
    (
        "a closure's call is the closure's, not the enclosing function's",
        "rs",
        "fn f() { let _later = || f(); }\n",
        &[("f", No)],
    ),
    (
        "a nested function's call is its own",
        "rs",
        "fn f() { fn inner() { f(); } }\n",
        &[("f", No), ("inner", No)],
    ),
    ("mutual recursion is not direct", "rs", "fn a() { b() }\nfn b() { a() }\n", &[("a", No), ("b", No)]),
    (
        "bodyless declaration",
        "rs",
        "trait T { fn m(&self); fn d(&self) {} }\n",
        &[("m", Unknown), ("d", No)],
    ),
    (
        "an import alias may name the function itself",
        "rs",
        "use crate::f as again;\nfn f() { again(); }\n",
        &[("f", Unknown)],
    ),
    (
        "an import alias of another name is not the function",
        "rs",
        "use crate::other as again;\nfn f() { again(); }\n",
        &[("f", No)],
    ),
    (
        "typescript direct self-call",
        "ts",
        "function fact(n: number): number { return n <= 1 ? 1 : n * fact(n - 1); }\nfunction leaf(): number { return 1; }\n",
        &[("fact", Yes), ("leaf", No)],
    ),
    (
        "typescript block `let` shadows the function before its declaration",
        "ts",
        "function f(): void { { f(); let f = 1; } }\n",
        &[("f", Unknown)],
    ),
    ("typescript `this` call", "ts", "class C { m(): void { this.m(); } }\n", &[("m", Unknown)]),
    (
        "constructing its own class",
        "ts",
        "class C { constructor() { new C(); } }\n",
        &[("constructor", Unknown)],
    ),
    (
        "constructor calling another function",
        "ts",
        "class D { constructor() { helper(); } }\nfunction helper(): void {}\n",
        &[("constructor", No), ("helper", No)],
    ),
    ("javascript direct self-call", "js", "function f(n) { return n && f(n - 1); }\n", &[("f", Yes)]),
    ("python is limited", "py", "def f(n):\n    return f(n - 1)\n\n\ndef g():\n    pass\n", &[("f", Unknown), ("g", Unknown)]),
    ("go is limited", "go", "package p\n\nfunc f() { f() }\n", &[("f", Unknown)]),
];

#[test]
fn recursion_matrix_holds_on_complete_and_narrowed_queries() {
    for &(label, extension, source, expected) in RECURSION_MATRIX {
        let (_file, tree_file) = parse(source, extension);
        let full = tree_file.symbol_index_v2().expect("complete index");
        let narrowed = tree_file
            .query(
                &SymbolQuery::new()
                    .kinds([SymbolKindV2::Function, SymbolKindV2::Method, SymbolKindV2::Constructor])
                    .capabilities([Capability::Semantics]),
            )
            .expect("narrowed query")
            .index;
        for &(name, value) in expected {
            assert_eq!(record(&full, name).semantics.is_recursive, Some(value), "{label}: `{name}`");
            assert_eq!(
                record(&narrowed, name).semantics.is_recursive,
                Some(value),
                "{label}: `{name}` in a functions-only query"
            );
        }
    }
}

#[test]
fn is_recursive_is_absent_without_semantics_and_only_on_callables() {
    let source = "fn countdown(n: u32) -> u32 { if n == 0 { 0 } else { countdown(n - 1) } }\nstruct Holder;\n";
    let (_file, tree_file) = parse(source, "rs");
    let full = tree_file.symbol_index_v2().expect("complete index");
    assert_eq!(record(&full, "n").semantics.is_recursive, None, "a parameter has no semantics");
    assert_eq!(record(&full, "Holder").semantics.is_recursive, None, "a type has no semantics");

    for capabilities in [&[Capability::References][..], &[Capability::Occurrences]] {
        let without = tree_file
            .query(&SymbolQuery::new().capabilities(capabilities.iter().copied()))
            .expect("query");
        assert!(
            without.index.symbols.iter().all(|record| record.semantics.is_recursive.is_none()),
            "{capabilities:?}: no semantics requested, no value"
        );
    }
}

const SUMMARY_SOURCE: &str = "\
use std::mem::swap as exchange;
fn a() {}
fn b() {}
fn main() { b(); a(); b(); exchange(&mut 1, &mut 2); }
fn shadow() { let a = 1; let _ = a; }
struct A;
struct B;
impl A { fn run(&self) {} }
impl B { fn run(&self) {} }
fn go(x: A) { x.run(); }
fn closure_owner() { let _c = || a(); }
const TOP: () = b();
";

/// `references`/`dependencies`/`referenced_by` hold only established
/// targets and bound imports, in first-occurrence order without repeats;
/// shadowed names, receiver calls and ownerless occurrences make no edge,
/// while every occurrence stays in the evidence.
#[test]
fn summaries_hold_only_established_targets_in_first_occurrence_order() {
    let index = complete(SUMMARY_SOURCE, "rs");
    let (a, b, main) = (record(&index, "a"), record(&index, "b"), record(&index, "main"));

    assert_eq!(names(&main.relations.references), [("b", true), ("a", true), ("exchange", false)]);
    assert_eq!(main.relations.references[0].id.as_ref(), Some(&b.id));
    assert_eq!(main.relations.references[1].id.as_ref(), Some(&a.id));
    assert_eq!(names(&main.relations.dependencies), [("exchange", false)], "only the bound import");
    let import = &main.relations.dependencies[0];
    let recorded = index.imports.iter().find(|import| import.name == "exchange").expect("import record");
    assert_eq!(import.qualified_name, recorded.symbol.qualified_name, "the import record's original name");

    fn owners(record: &SymbolRecord) -> Vec<&str> {
        record.relations.referenced_by.iter().map(|owner| owner.name.as_str()).collect()
    }
    assert_eq!(owners(a), ["main"], "not `shadow` (its own `a`), not the closure");
    assert_eq!(owners(b), ["main"], "not the ownerless `TOP` initializer");
    assert_eq!(main.relations.referenced_by, [], "nothing calls `main`");

    let shadow = record(&index, "shadow");
    assert_eq!(shadow.relations.references.len(), 1, "{:#?}", shadow.relations.references);
    assert_ne!(shadow.relations.references[0].id.as_ref(), Some(&a.id), "the local `a`, not the function");

    let go = record(&index, "go");
    assert_eq!(
        names(&go.relations.references),
        [("A", true), ("x", true)],
        "the parameter's type and the receiver; `x.run()` binds neither `run`"
    );
    for run in index.symbols.iter().filter(|record| record.identity.name == "run") {
        assert!(run.relations.referenced_by.is_empty(), "a receiver call makes no reverse edge");
    }
    assert!(record(&index, "closure_owner").relations.references.is_empty(), "the closure owns `a()`");

    let kept = |name: &str, confidence: Confidence| {
        index.occurrences.iter().any(|occurrence| occurrence.target.name == name && occurrence.confidence == confidence)
    };
    assert!(kept("run", Confidence::NameOnly), "the uncertain receiver call stays an occurrence");
    let ownerless_b = index
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.target.name == "b" && occurrence.owner.is_none())
        .count();
    assert_eq!(ownerless_b, 1, "the `TOP` initializer call stays an occurrence");
    let a_calls = index.occurrences.iter().filter(|occurrence| occurrence.target.id.as_ref() == Some(&a.id)).count();
    assert_eq!(a_calls, 2, "main's call and the closure's call are both kept");
}

/// An aliased TypeScript import is a dependency of every owner that calls
/// it, once per owner, and is never attached to another owner.
#[test]
fn bound_imports_are_dependencies_of_their_owners_only() {
    let index = complete(
        "import { readFile as rf } from \"fs\";\nimport { join } from \"path\";\n\
         function g(): void { rf(\"x\"); rf(\"y\"); join(); }\nfunction h(): void {}\n",
        "ts",
    );
    let g = record(&index, "g");
    let dependencies: Vec<&str> = g.relations.dependencies.iter().map(|dependency| dependency.name.as_str()).collect();
    assert_eq!(dependencies.len(), 2, "{:#?}", g.relations.dependencies);
    assert_eq!(g.relations.references, g.relations.dependencies, "only imports are referenced");
    for dependency in &g.relations.dependencies {
        assert!(dependency.id.is_none(), "an import has no in-file ID");
        assert!(
            index.imports.iter().any(|import| import.name == dependency.name
                && import.symbol.qualified_name == dependency.qualified_name),
            "{dependency:?} is an import record"
        );
    }
    assert!(record(&index, "h").relations.dependencies.is_empty());
}

/// A language without resolver rules establishes nothing, so it has no
/// summary edges; its occurrences remain, none of them `Scoped`.
#[test]
fn limited_languages_bind_no_edges_but_keep_their_occurrences() {
    let index = complete(
        "from os import path\nfrom sys import argv\n\n\ndef helper():\n    pass\n\n\ndef run():\n    path.join(\"a\")\n    helper()\n    return argv\n",
        "py",
    );
    for record in &index.symbols {
        assert!(record.relations.references.is_empty(), "{}: references", record.identity.name);
        assert!(record.relations.dependencies.is_empty(), "{}: dependencies", record.identity.name);
        assert!(record.relations.referenced_by.is_empty(), "{}: referenced_by", record.identity.name);
    }
    let written: Vec<&str> = index.occurrences.iter().map(|occurrence| occurrence.target.name.as_str()).collect();
    for name in ["path", "helper", "argv"] {
        assert!(written.contains(&name), "`{name}` is still an occurrence: {written:?}");
    }
    assert!(index.occurrences.iter().all(|occurrence| occurrence.confidence != Confidence::Scoped));
}

/// For every primary fixture: a kind-narrowed query's occurrences and
/// coverage equal the complete index's (whole file, possibly naming
/// unselected declarations), its records equal the complete index's records
/// of those kinds, `References` alone keeps the edges but no evidence, and
/// one analysis runs one references query and one resolution.
#[test]
fn occurrences_describe_the_whole_file_whatever_kinds_are_selected() {
    let mut named_unselected = 0;
    for language in ProgrammingLanguage::all() {
        let path = primary_fixture(language).unwrap_or_else(|| panic!("{language:?}: no primary fixture"));
        let label = display_name(&path);
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let (full, work) = counted(|| tree_file.symbol_index_v2().expect("complete index"));
        assert_eq!((work.reference_captures, work.occurrence_resolutions), (1, 1), "{label}: complete index");
        assert!(!full.reference_coverage.is_empty(), "{label}: coverage");

        let kinds = [SymbolKindV2::Function, SymbolKindV2::Method];
        let query = SymbolQuery::new().kinds(kinds).capabilities([Capability::Semantics]);
        let (narrow, work) = counted(|| tree_file.query(&query).expect("narrow query").index);
        assert_eq!((work.reference_captures, work.occurrence_resolutions), (1, 1), "{label}: narrow query");
        assert_eq!(narrow.occurrences, full.occurrences, "{label}: occurrences");
        assert_eq!(narrow.reference_coverage, full.reference_coverage, "{label}: coverage");
        let wide = tree_file
            .query(&SymbolQuery::new().capabilities([Capability::Semantics]))
            .expect("wide query")
            .index;
        let expected: Vec<&SymbolRecord> = wide.symbols.iter().filter(|record| kinds.contains(&record.kind)).collect();
        let narrow_records: Vec<&SymbolRecord> = narrow.symbols.iter().collect();
        assert_eq!(
            serde_json::to_value(&narrow_records).expect("serialize"),
            serde_json::to_value(&expected).expect("serialize"),
            "{label}: records"
        );
        let selected: Vec<_> = narrow.symbols.iter().map(|record| &record.id).collect();
        named_unselected += narrow
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.target.id.as_ref().is_some_and(|id| !selected.contains(&id)))
            .count();

        let references = tree_file
            .query(&SymbolQuery::new().capabilities([Capability::References]))
            .expect("references query")
            .index;
        assert!(references.occurrences.is_empty() && references.reference_coverage.is_empty(), "{label}");
        for (with_evidence, without) in full.symbols.iter().zip(&references.symbols) {
            assert_eq!(
                serde_json::to_value(&with_evidence.relations).expect("serialize"),
                serde_json::to_value(&without.relations).expect("serialize"),
                "{label}: {}",
                with_evidence.identity.name
            );
        }

        for capabilities in [&[Capability::Declarations][..], &[Capability::Containment, Capability::Flags]] {
            let (index, work) = counted(|| {
                tree_file
                    .query(&SymbolQuery::new().capabilities(capabilities.iter().copied()))
                    .expect("declaration query")
                    .index
            });
            assert_eq!(
                (work.reference_captures, work.occurrence_resolutions),
                (0, 0),
                "{label}: {capabilities:?} skips reference work"
            );
            assert!(index.occurrences.is_empty() && index.reference_coverage.is_empty(), "{label}");
        }
    }
    assert!(named_unselected > 0, "some occurrence names a declaration the query did not select");
}

/// Sharing the references run with the occurrences changes no diagnostic:
/// the complete index's diagnostics equal those of the standalone
/// diagnostics path for every fixture.
#[test]
fn diagnostics_are_unchanged_by_the_shared_references_run() {
    for path in all_fixtures() {
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let index = tree_file.symbol_index_v2().expect("complete index");
        assert_eq!(
            serde_json::to_value(&index.diagnostics).expect("serialize"),
            serde_json::to_value(tree_file.try_diagnostics().expect("diagnostics")).expect("serialize"),
            "{}",
            display_name(&path)
        );
    }
}

/// Calls and usages both feed the summaries, but only calls decide
/// recursion: a callable whose only self-reference is a usage is `No`.
#[test]
fn usages_feed_summaries_but_not_recursion() {
    let index = complete("fn f() { let _keep = f; }\nfn g() { f(); }\n", "rs");
    let f = record(&index, "f");
    assert_eq!(names(&f.relations.references), [("f", true)], "the self-value reference is an edge");
    assert_eq!(f.semantics.is_recursive, Some(TriState::No));
    let usage = index
        .occurrences
        .iter()
        .find(|occurrence| occurrence.target.id.as_ref() == Some(&f.id) && occurrence.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(&f.id))
        .expect("the self-value occurrence");
    assert_eq!(usage.kind, OccurrenceKind::Usage);
    let owners: Vec<&str> = f.relations.referenced_by.iter().map(|owner| owner.name.as_str()).collect();
    assert_eq!(owners, ["f", "g"]);
}
