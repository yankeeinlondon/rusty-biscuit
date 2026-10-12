//! Scope-aware reference occurrences and coverage
//! (`TreeFile::reference_evidence`): shadowing and declaration order,
//! receivers and overloads, spans and owners, aliases, and the supported or
//! limited state of every language. User contract:
//! `tree-hugger/docs/references.md`.

use std::collections::HashSet;
use std::io::Write;

use tree_hugger::{
    Confidence, CoverageState, FileSymbolIndex, LimitReason, Occurrence, OccurrenceKind, ProgrammingLanguage,
    ReferenceEvidence, SymbolId, SymbolKindV2, SymbolRecord, SymbolScope, TextSpan, TreeFile, TriState,
};

use crate::symbols_fixtures::{in_file_name, primary_fixture};

/// A file's evidence and its complete index (for record IDs).
struct Analyzed {
    source: String,
    evidence: ReferenceEvidence,
    index: FileSymbolIndex,
}

impl Analyzed {
    fn new(source: &str, extension: &str) -> Self {
        let mut file = tempfile::Builder::new()
            .suffix(&format!(".{extension}"))
            .tempfile()
            .expect("create temp source");
        file.write_all(source.as_bytes()).expect("write temp source");
        let tree_file = TreeFile::new(file.path()).expect("parse inline source");
        Self {
            source: source.to_string(),
            evidence: tree_file.reference_evidence().expect("reference evidence"),
            index: tree_file.symbol_index_v2().expect("complete index"),
        }
    }

    /// The `nth` (zero-based) occurrence written on `line` (one-based) with
    /// this name.
    fn at(&self, line: u32, name: &str, nth: usize) -> &Occurrence {
        self.evidence
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.span.start.line == line && occurrence.target.name == name)
            .nth(nth)
            .unwrap_or_else(|| panic!("no occurrence #{nth} of `{name}` on line {line}: {:#?}", self.evidence.occurrences))
    }

    /// Every occurrence of `name`.
    fn all(&self, name: &str) -> Vec<&Occurrence> {
        self.evidence
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.target.name == name)
            .collect()
    }

    /// The record with this in-file qualified name and kind label, whose
    /// name starts on `line`.
    fn record(&self, qualified: &str, line: u32) -> &SymbolRecord {
        self.index
            .symbols
            .iter()
            .find(|record| {
                in_file_name(record) == qualified
                    && record.source.name_span.as_ref().is_some_and(|span| span.start.line == line)
            })
            .unwrap_or_else(|| panic!("no record `{qualified}` on line {line}"))
    }

    /// The ID of the record called `name` whose name starts on `line`.
    fn named(&self, name: &str, line: u32) -> &SymbolId {
        self.index
            .symbols
            .iter()
            .find(|record| {
                record.identity.name == name
                    && record.source.name_span.as_ref().is_some_and(|span| span.start.line == line)
            })
            .map(|record| &record.id)
            .unwrap_or_else(|| panic!("no record called `{name}` on line {line}"))
    }

    fn id(&self, qualified: &str, line: u32) -> &SymbolId {
        &self.record(qualified, line).id
    }

    /// The text an occurrence's span covers.
    fn text(&self, occurrence: &Occurrence) -> &str {
        self.text_of_span(&occurrence.span)
    }

    fn text_of_span(&self, span: &TextSpan) -> &str {
        &self.source[span.start_byte as usize..span.end_byte as usize]
    }
}

fn assert_scoped_to(occurrence: &Occurrence, id: &SymbolId) {
    assert_eq!(occurrence.confidence, Confidence::Scoped, "{occurrence:#?}");
    assert_eq!(occurrence.target.id.as_ref(), Some(id), "{occurrence:#?}");
    assert!(occurrence.candidates.is_empty(), "{occurrence:#?}");
}

fn assert_uncertain(occurrence: &Occurrence) {
    assert_ne!(occurrence.confidence, Confidence::Scoped, "{occurrence:#?}");
    assert_eq!(occurrence.target.id, None, "{occurrence:#?}");
    match occurrence.confidence {
        Confidence::NameOnly => assert!(!occurrence.candidates.is_empty(), "{occurrence:#?}"),
        _ => assert!(occurrence.candidates.is_empty(), "{occurrence:#?}"),
    }
}

// ---- Shadowing and declaration order ---------------------------------------

const RUST_SHADOWING: &str = "\
fn f() -> u32 {
    let f = f();
    f
}
fn g() {
    let x = 1;
    {
        x;
        let x = \"inner\";
        x;
    }
    x;
}
";

#[test]
fn rust_let_initializer_reads_the_outer_binding_and_later_uses_the_let() {
    let file = Analyzed::new(RUST_SHADOWING, "rs");
    let function = file.id("f", 1);
    let local = file.id("f::f", 2);

    let call = file.at(2, "f", 0);
    assert_eq!(call.kind, OccurrenceKind::Call);
    assert_scoped_to(call, function);
    assert_eq!(call.owner.as_ref().and_then(|owner| owner.id.as_ref()), Some(function));

    assert_scoped_to(file.at(3, "f", 0), local);

    // An inner `let` is visible only after its statement, and only in its
    // block.
    let outer_x = file.id("g::x", 6);
    let inner_x = file.id("g::x", 9);
    assert_scoped_to(file.at(8, "x", 0), outer_x);
    assert_scoped_to(file.at(10, "x", 0), inner_x);
    assert_scoped_to(file.at(12, "x", 0), outer_x);
}

const ECMA_SHADOWING: &str = "\
function f() {
  {
    f();
    let f = () => 1;
    f();
  }
  return f();
}
";

#[test]
fn ecma_block_let_hides_the_outer_name_before_its_declaration() {
    for extension in ["ts", "js"] {
        let file = Analyzed::new(ECMA_SHADOWING, extension);
        let function = file.id("f", 1);
        let inner = file.named("f", 4);

        // The temporal dead zone: the block's `let f` decides even before
        // its declaration.
        assert_scoped_to(file.at(3, "f", 0), inner);
        assert_scoped_to(file.at(5, "f", 0), inner);
        assert_scoped_to(file.at(7, "f", 0), function);
        assert_ne!(inner, function, "{extension}");
    }
}

#[test]
fn ecma_var_is_function_scoped_and_hoisted() {
    let source = "\
function f() {
  v;
  {
    var v = 1;
  }
  v;
}
var v = 2;
";
    for extension in ["ts", "js"] {
        let file = Analyzed::new(source, extension);
        let local = file.named("v", 4);
        assert_scoped_to(file.at(2, "v", 0), local);
        assert_scoped_to(file.at(6, "v", 0), local);
    }
}

#[test]
fn python_local_by_assignment_is_limited_never_a_wrong_binding() {
    let source = "\
x = 1

def f():
    print(x)
    x = 2
    return x
";
    let file = Analyzed::new(source, "py");
    for occurrence in file.all("x") {
        assert_uncertain(occurrence);
    }
    assert!(!file.all("x").is_empty());
    for coverage in &file.evidence.reference_coverage {
        assert_eq!(coverage.state, CoverageState::Limited, "{coverage:#?}");
        assert!(coverage.reasons.contains(&LimitReason::UnsupportedLanguage), "{coverage:#?}");
    }
}

#[test]
fn hidden_bindings_block_outer_matches_even_without_a_record() {
    // `for` and `catch` bindings and destructured names are not reported as
    // records, but still shadow the outer `q`/`e`.
    let source = "\
function q() {}
function e() {}
function g(items: number[]) {
  for (const q of items) { q; }
  try {} catch (e) { e; }
}
";
    let file = Analyzed::new(source, "ts");
    for (line, name) in [(4, "q"), (5, "e")] {
        let occurrence = file.at(line, name, 0);
        assert_eq!(occurrence.confidence, Confidence::Scoped, "{occurrence:#?}");
        assert_eq!(occurrence.target.id, None, "a local without a record has no ID: {occurrence:#?}");
    }
}

#[test]
fn type_and_value_names_are_separate_categories() {
    let source = "\
struct Size { n: u32 }
fn Size() {}
fn use_both(_: Size) { Size(); }
";
    let file = Analyzed::new(source, "rs");
    assert_scoped_to(file.at(3, "Size", 0), file.id("Size", 1));
    let call = file.at(3, "Size", 1);
    assert_eq!(call.kind, OccurrenceKind::Call);
    assert_scoped_to(call, file.id("Size", 2));
}

// ---- Receivers, paths and overloads ----------------------------------------

const RUST_RECEIVERS: &str = "\
struct A;
struct B;
impl A {
    fn go(&self) {}
    fn method(&self, other: B) { other.method(); self.method(other); }
}
impl B {
    fn go(&self) {}
    fn method(&self) {}
}
fn caller(a: A) { a.go(); A::go(&a); }
";

#[test]
fn same_named_methods_on_two_types_stay_name_only() {
    let file = Analyzed::new(RUST_RECEIVERS, "rs");
    let a_go = file.id("A::go", 4);
    let b_go = file.id("B::go", 8);

    let method_call = file.at(11, "go", 0);
    assert_eq!(method_call.confidence, Confidence::NameOnly);
    assert_eq!(method_call.target.id, None);
    assert_eq!(method_call.target.qualified_name.as_deref(), Some("a.go"));
    let candidates: Vec<_> = method_call.candidates.iter().map(|candidate| candidate.id.as_ref()).collect();
    assert_eq!(candidates, [Some(a_go), Some(b_go)], "declaration order");

    // A path through one in-file type with one inherent `go` is established.
    let path_call = file.at(11, "go", 1);
    assert_eq!(path_call.target.qualified_name.as_deref(), Some("A::go"));
    assert_scoped_to(path_call, a_go);
}

#[test]
fn a_receiver_call_never_binds_to_the_caller() {
    let file = Analyzed::new(RUST_RECEIVERS, "rs");
    let caller = file.id("A::method", 5);
    for occurrence in file.all("method") {
        assert_eq!(occurrence.kind, OccurrenceKind::Call);
        assert_uncertain(occurrence);
        assert!(occurrence.target.id.as_ref() != Some(caller));
    }
    let other = file.at(5, "method", 0);
    assert_eq!(other.target.qualified_name.as_deref(), Some("other.method"));
    let own = file.at(5, "method", 1);
    assert_eq!(own.target.qualified_name.as_deref(), Some("self.method"), "self.f() is not proven");

    let region = file
        .evidence
        .reference_coverage
        .iter()
        .find(|coverage| coverage.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(caller))
        .expect("caller region");
    assert_eq!(region.state, CoverageState::Limited);
    assert!(region.reasons.contains(&LimitReason::IndirectDispatch));
}

#[test]
fn ecma_receivers_and_dynamic_access_stay_uncertain() {
    let source = "\
class Widget {
  run(): void {}
  go(obj: any, k: string): void { obj.run(); this.run(); obj[k](); }
}
";
    let file = Analyzed::new(source, "ts");
    let run = file.id("Widget.run", 2);
    for occurrence in file.all("run") {
        assert_eq!(occurrence.kind, OccurrenceKind::Call);
        assert_eq!(occurrence.confidence, Confidence::NameOnly);
        assert_eq!(occurrence.target.id, None);
        assert_eq!(occurrence.candidates[0].id.as_ref(), Some(run));
    }
    assert_eq!(file.all("run").len(), 2);
    // `obj[k]()` names no target; its region records the dynamic call.
    let go = file.id("Widget.go", 3);
    let region = file
        .evidence
        .reference_coverage
        .iter()
        .find(|coverage| coverage.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(go))
        .expect("go region");
    assert_eq!(region.reasons, [LimitReason::IndirectDispatch]);
    assert_scoped_to(file.at(3, "k", 0), file.named("k", 3));
}

#[test]
fn overloads_never_get_a_target_id() {
    let source = "\
function pick(a: string): string;
function pick(a: number): number;
function pick(a: any): any { return a; }
pick(1);
";
    let file = Analyzed::new(source, "ts");
    let call = file.at(4, "pick", 0);
    assert_eq!(call.kind, OccurrenceKind::Call);
    assert_uncertain(call);
    assert_eq!(call.confidence, Confidence::NameOnly);
}

#[test]
fn unresolved_qualified_targets_keep_their_written_qualifier() {
    let source = "\
fn f() { std::mem::swap(); Missing::make(); }
";
    let file = Analyzed::new(source, "rs");
    let swap = file.at(1, "swap", 0);
    assert_eq!(swap.target.qualified_name.as_deref(), Some("std::mem::swap"));
    assert_eq!(swap.confidence, Confidence::Unresolved);
    let make = file.at(1, "make", 0);
    assert_eq!(make.target.qualified_name.as_deref(), Some("Missing::make"));
    assert_eq!(make.confidence, Confidence::Unresolved);
}

// ---- Spans, owners and what is not an occurrence ---------------------------

#[test]
fn spans_cover_the_name_token_in_non_ascii_source() {
    let source = "\
fn größe() -> u32 { 1 }
fn main() { let é = \"ü\"; größe(); größe(); é; }
";
    let file = Analyzed::new(source, "rs");
    let calls = file.all("größe");
    assert_eq!(calls.len(), 2, "two calls on one line stay two records");
    let target = file.id("größe", 1);
    for call in &calls {
        assert_eq!(file.text(call), "größe");
        assert_eq!(call.span.start.line, 2);
        assert_eq!(call.span.end.line, 2);
        assert_scoped_to(call, target);
    }
    assert!(calls[0].span.start_byte < calls[1].span.start_byte);
    // Columns count bytes, one-based, like every other span.
    let first = calls[0];
    let line_start = source.find("fn main").expect("line 2");
    assert_eq!(first.span.start.column as usize, first.span.start_byte as usize - line_start + 1);
    assert_eq!(file.text(file.at(2, "é", 0)), "é");
}

#[test]
fn overlapping_captures_make_one_occurrence() {
    // TypeScript captures a called identifier with the broad pattern and
    // the call pattern; Rust a field-expression receiver with two.
    for (source, extension, line, name) in [
        ("function f() {}\nf();\n", "ts", 2, "f"),
        ("fn f(x: u32) { x.count_ones(); }\n", "rs", 1, "x"),
    ] {
        let file = Analyzed::new(source, extension);
        let spans: Vec<_> = file
            .all(name)
            .into_iter()
            .filter(|occurrence| occurrence.span.start.line == line)
            .map(|occurrence| occurrence.span.start_byte)
            .collect();
        assert_eq!(spans.len(), 1, "{extension}: {:#?}", file.evidence.occurrences);
    }
}

#[test]
fn constructors_are_calls() {
    let rust = Analyzed::new("struct Point(u32);\nfn f() { Point(1); }\n", "rs");
    let call = rust.at(2, "Point", 0);
    assert_eq!(call.kind, OccurrenceKind::Call);
    assert_scoped_to(call, rust.id("Point", 1));

    let ts = Analyzed::new("class Widget {}\nconst w = new Widget();\n", "ts");
    let call = ts.at(2, "Widget", 0);
    assert_eq!(call.kind, OccurrenceKind::Call);
    assert_scoped_to(call, ts.id("Widget", 1));
}

#[test]
fn file_level_code_and_initializers_keep_their_occurrences() {
    let source = "\
fn compute() -> u32 { 1 }
const LIMIT: u32 = compute();
fn f() -> u32 {
    const INNER: u32 = compute();
    INNER
}
";
    let file = Analyzed::new(source, "rs");
    let whole_file = &file.evidence.reference_coverage[0];
    assert_eq!(whole_file.owner, None);
    assert_eq!(whole_file.span.start_byte, 0);
    assert_eq!(whole_file.span.end_byte as usize, source.len());

    let file_level = file.at(2, "compute", 0);
    assert_eq!(file_level.owner, None);
    assert_eq!(file_level.owner_span, whole_file.span);
    assert_scoped_to(file_level, file.id("compute", 1));

    // A `const` inside a function is evaluated at compile time: it is not a
    // call made by `f`.
    let inner = file.at(4, "compute", 0);
    assert_eq!(inner.owner, None);
    assert_eq!(file.text_of_span(&inner.owner_span), "const INNER: u32 = compute();");
}

#[test]
fn anonymous_callables_are_distinct_owner_absent_regions() {
    for (source, extension) in [
        ("fn f() { let a = || g(); let b = || g(); }\nfn g() {}\n", "rs"),
        ("function f() { const a = [1].map(() => g()); const b = [1].map(() => g()); }\nfunction g() {}\n", "ts"),
    ] {
        let file = Analyzed::new(source, extension);
        let calls = file.all("g");
        let calls: Vec<_> = calls.iter().filter(|call| call.span.start.line == 1).collect();
        assert_eq!(calls.len(), 2, "{extension}");
        for call in &calls {
            assert_eq!(call.owner, None, "{extension}: {call:#?}");
            assert_eq!(call.kind, OccurrenceKind::Call);
        }
        assert_ne!(calls[0].owner_span, calls[1].owner_span, "{extension}");
        for call in &calls {
            assert!(
                file.evidence
                    .reference_coverage
                    .iter()
                    .any(|coverage| coverage.owner.is_none() && coverage.span == call.owner_span),
                "{extension}: each anonymous body has its own region"
            );
        }
    }
}

#[test]
fn nested_named_functions_own_their_calls() {
    let source = "\
fn outer() {
    fn find(n: u32) -> u32 { if n == 0 { 0 } else { find(n - 1) } }
    find(3);
}
";
    let file = Analyzed::new(source, "rs");
    let find = file.id("outer::find", 2);
    let outer = file.id("outer", 1);

    let inner = file.at(2, "find", 0);
    assert_scoped_to(inner, find);
    assert_eq!(inner.owner.as_ref().and_then(|owner| owner.id.as_ref()), Some(find));

    let outer_call = file.at(3, "find", 0);
    assert_scoped_to(outer_call, find);
    assert_eq!(outer_call.owner.as_ref().and_then(|owner| owner.id.as_ref()), Some(outer));
}

#[test]
fn named_bodies_have_coverage_even_without_occurrences() {
    let file = Analyzed::new("fn empty() {}\nfn other() { empty(); }\n", "rs");
    let empty = file.id("empty", 1);
    let region = file
        .evidence
        .reference_coverage
        .iter()
        .find(|coverage| coverage.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(empty))
        .expect("a region for the empty body");
    assert_eq!(region.state, CoverageState::Complete);
    assert!(region.reasons.is_empty());
}

#[test]
fn declaration_names_comments_and_strings_are_not_occurrences() {
    let source = "\
// find() in a comment
fn find() {
    let s = \"find()\";
    let t = s;
}
";
    let file = Analyzed::new(source, "rs");
    assert!(file.all("find").is_empty(), "{:#?}", file.evidence.occurrences);
    // The `let` names are bindings; only the read of `s` is an occurrence.
    assert_eq!(file.all("s").len(), 1);
    assert!(file.all("t").is_empty());
}

#[test]
fn macros_are_usages_that_limit_their_region() {
    let file = Analyzed::new("macro_rules! twice { () => {} }\nfn f() { twice!(); }\n", "rs");
    let usage = file.at(2, "twice", 0);
    assert_eq!(usage.kind, OccurrenceKind::Usage);
    assert_scoped_to(usage, file.id("twice", 1));
    let region = file
        .evidence
        .reference_coverage
        .iter()
        .find(|coverage| coverage.owner.as_ref().is_some_and(|owner| owner.name == "f"))
        .expect("f region");
    assert_eq!(region.reasons, [LimitReason::UnexpandedMacro]);
}

#[test]
fn parse_errors_limit_the_region_that_holds_them() {
    let file = Analyzed::new("fn ok() { g(); }\nfn broken() { let = ; }\nfn g() {}\n", "rs");
    let state = |name: &str| {
        file.evidence
            .reference_coverage
            .iter()
            .find(|coverage| coverage.owner.as_ref().is_some_and(|owner| owner.name == name))
            .map(|coverage| coverage.reasons.clone())
    };
    assert_eq!(state("ok"), Some(Vec::new()));
    assert_eq!(state("broken"), Some(vec![LimitReason::ParseError]));
}

// ---- Imports ---------------------------------------------------------------

#[test]
fn alias_imports_bind_the_alias_locally_without_an_id() {
    for (source, extension, alias, original, from) in [
        (
            "import { readFile as read } from \"fs\";\nread();\n",
            "ts",
            "read",
            "readFile",
            "fs",
        ),
        ("use std::fs::read_to_string as read;\nfn f() { read(); }\n", "rs", "read", "read_to_string", "std::fs"),
    ] {
        let file = Analyzed::new(source, extension);
        let call = file.at(2, alias, 0);
        assert_eq!(call.confidence, Confidence::Scoped, "{extension}: {call:#?}");
        assert_eq!(call.target.id, None, "{extension}");
        assert!(file.all(original).is_empty(), "{extension}: the original name is not an occurrence");

        let import = file
            .index
            .imports
            .iter()
            .find(|import| import.alias.as_deref() == Some(alias) || import.name == alias)
            .unwrap_or_else(|| panic!("{extension}: no import record: {:#?}", file.index.imports));
        let original_name = import.symbol.qualified_name.as_deref().unwrap_or(&import.name);
        assert!(original_name.ends_with(original), "{extension}: {import:#?}");
        assert!(import.source.as_deref().is_some_and(|source| source.contains(from)), "{extension}: {import:#?}");
    }
}

/// `(label, extension, source, owner, [(line, name, bound)], dependencies)`:
/// each later use named on its line is `Scoped` to an import (no ID) when
/// `bound`, else uncertain; `owner`'s dependencies are exactly the bound
/// imports.
type ImportBindingRow = (&'static str, &'static str, &'static str, &'static str, &'static [(u32, &'static str, bool)], &'static [&'static str]);

/// Imports in the resolved languages (review rows `R7-F3.S5.nn` and the
/// TS/JS sweep): every name an import binds is a `Scoped` binding with no
/// ID and a dependency of the callable that uses it; a glob, an aliased
/// original and an `extern crate` original bind nothing.
const IMPORT_BINDING_MATRIX: &[ImportBindingRow] = &[
    (
        "R7-F3.S5.01 Rust use forms, extern crate aliases and a glob",
        "rs",
        "use a;\nuse b::*;\nuse c::{g::h, d, e as f};\nextern crate y as z;\nuse k::{self, l};\nuse m::n::{self as nn};\n\
         fn run() { a::x(); h(); d(); f(); z::w(); k::v(); l(); nn::u(); q(); e(); y(); }\n",
        "run",
        &[
            (7, "a", true),
            (7, "h", true),
            (7, "d", true),
            (7, "f", true),
            (7, "z", true),
            (7, "k", true),
            (7, "l", true),
            (7, "nn", true),
            (7, "q", false),
            (7, "e", false),
            (7, "y", false),
        ],
        &["a", "d", "f", "h", "k", "l", "nn", "z"],
    ),
    (
        "R7-F3.ts default, aliased, namespace, require and alias imports",
        "ts",
        "import d, { a as b, c } from \"m\";\nimport * as ns from \"m\";\nimport q = require(\"m\");\nimport R = N.S;\n\
         function g() { d(); b(); c(); ns.f(); q(); R; a; }\n",
        "g",
        &[(5, "d", true), (5, "b", true), (5, "c", true), (5, "ns", true), (5, "q", true), (5, "R", true), (5, "a", false)],
        &["R", "b", "c", "d", "ns", "q"],
    ),
    (
        "R7-F3.js default and aliased imports",
        "js",
        "import d, { a as b } from \"m\";\nfunction g() { d(); b(); a; }\n",
        "g",
        &[(2, "d", true), (2, "b", true), (2, "a", false)],
        &["b", "d"],
    ),
];

#[test]
fn imports_bind_exactly_the_names_they_bring_in() {
    for &(label, extension, source, owner, uses, dependencies) in IMPORT_BINDING_MATRIX {
        let file = Analyzed::new(source, extension);
        for import in &file.index.imports {
            assert!(
                !file.evidence.occurrences.iter().any(|occurrence| occurrence.span.start_byte == import.span.start_byte),
                "{label}: the imported `{}` is an occurrence",
                import.name
            );
        }
        for &(line, name, bound) in uses {
            let occurrence = file.at(line, name, 0);
            if bound {
                assert_eq!(occurrence.confidence, Confidence::Scoped, "{label}: `{name}`: {occurrence:#?}");
                assert_eq!(occurrence.target.id, None, "{label}: `{name}` is bound to an import");
            } else {
                assert_uncertain(occurrence);
            }
        }
        let owner_record = file
            .index
            .symbols
            .iter()
            .find(|record| record.identity.name == owner)
            .unwrap_or_else(|| panic!("{label}: no `{owner}`"));
        let mut observed: Vec<&str> =
            owner_record.relations.dependencies.iter().map(|dependency| dependency.name.as_str()).collect();
        observed.sort_unstable();
        assert_eq!(observed, dependencies, "{label}: dependencies");
    }
}

/// `(label, extension, source, [(type record name, line)], [(line, column,
/// name, written path, NameOnly?)])`: the type-like records (Type, Class)
/// listed exactly, and every occurrence on the lines the reads name.
type ElaboratedTypeRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, u32)],
    &'static [(u32, u32, &'static str, Option<&'static str>, bool)],
);

/// C/C++ elaborated type uses (review rows `R7-F2.X1.nn`): a `struct`,
/// `union` or `class` specifier declares its name only with a body or
/// when written alone (`struct S;`, also as a member or in a namespace,
/// template or linkage body); an elaborated use in a declaration, a
/// parameter, a `typedef` or a member is a read and makes no record.
const ELABORATED_TYPE_MATRIX: &[ElaboratedTypeRow] = &[
    (
        "R7-F2.X1.01 C definitions, declarations alone and elaborated uses",
        "c",
        "struct S { int a; };\nstruct F;\nunion U;\nstruct S x;\nvoid f(struct S *p, union U u);\ntypedef struct S T;\nstruct O { struct S in; struct I; };\n",
        &[("S", 1), ("F", 2), ("U", 3), ("O", 7), ("I", 7)],
        &[
            (4, 8, "S", None, true),
            (5, 15, "S", None, true),
            (5, 27, "U", None, true),
            (6, 16, "S", None, true),
            (7, 19, "S", None, true),
        ],
    ),
    (
        "R7-F2.X1.02 C++ classes, qualified names and declaration-alone bodies",
        "cpp",
        "class C { int a; };\nclass F;\nclass C *c;\nvoid f(class C *p, struct S &s);\nnamespace N { struct M; }\n\
         template <class X> struct TT;\nclass O { class In; struct A::Q *q; };\nstruct A::Q {};\nextern \"C\" { struct E; }\n",
        &[("C", 1), ("F", 2), ("M", 5), ("X", 6), ("TT", 6), ("O", 7), ("In", 7), ("Q", 8), ("E", 9)],
        &[
            (3, 7, "C", None, true),
            (4, 14, "C", None, true),
            (4, 27, "S", None, false),
            (7, 28, "A", None, false),
            (7, 31, "Q", Some("A::Q"), true),
        ],
    ),
];

#[test]
fn elaborated_type_uses_are_reads_not_declarations() {
    for &(label, extension, source, types, reads) in ELABORATED_TYPE_MATRIX {
        let file = Analyzed::new(source, extension);
        let observed: Vec<(&str, u32)> = file
            .index
            .symbols
            .iter()
            .filter(|record| matches!(record.kind, SymbolKindV2::Type | SymbolKindV2::Class))
            .map(|record| {
                let line = record.source.name_span.as_ref().map_or(0, |span| span.start.line);
                (record.identity.name.as_str(), line)
            })
            .collect();
        assert_eq!(observed, types, "{label}: type records");

        let lines: HashSet<u32> = reads.iter().map(|&(line, ..)| line).collect();
        let observed: Vec<(u32, u32, &str, Option<&str>, bool)> = file
            .evidence
            .occurrences
            .iter()
            .filter(|occurrence| lines.contains(&occurrence.span.start.line))
            .map(|occurrence| {
                assert_eq!(occurrence.kind, USAGE, "{label}: {occurrence:#?}");
                assert_uncertain(occurrence);
                (
                    occurrence.span.start.line,
                    occurrence.span.start.column,
                    occurrence.target.name.as_str(),
                    occurrence.target.qualified_name.as_deref(),
                    occurrence.confidence == Confidence::NameOnly,
                )
            })
            .collect();
        assert_eq!(observed, reads, "{label}: reads");
    }
}

/// `(label, extension, source, [(line, column, name, kind, target record
/// as (name, line) when Scoped to it, else uncertain)], [(callable,
/// referenced names)])`: the whole file's occurrences, exactly, and each
/// listed callable's summary references (every other record has none and
/// is referenced only by those).
type ExportClauseRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(u32, u32, &'static str, OccurrenceKind, Option<(&'static str, u32)>)],
    &'static [(&'static str, &'static [&'static str])],
);

/// TS/JS export clauses (review rows `R7-F3.X2.nn`): every name in a
/// re-export from another module and every exported alias name the
/// module's exports, never a local binding, so none is an occurrence; a
/// local export's own name (`k` in `export { k }`, `export { k as j }`,
/// `export = k`) is a `Scoped` read.
const EXPORT_CLAUSE_MATRIX: &[ExportClauseRow] = &[
    (
        "R7-F3.X2.01 re-export from beside a local export",
        "ts",
        "function k() {}\nexport { k as j } from \"m\";\nexport { k };\nk();\n",
        &[(3, 10, "k", USAGE, Some(("k", 1))), (4, 1, "k", CALL, Some(("k", 1)))],
        &[],
    ),
    (
        "R7-F3.X2.02 an exported alias and a namespace re-export",
        "ts",
        "function k() {}\nconst j = 1;\nexport { k as j };\nexport * as k from \"m\";\n",
        &[(3, 10, "k", USAGE, Some(("k", 1)))],
        &[],
    ),
    (
        "R7-F3.X2.03 default re-exports and an export as default",
        "ts",
        "function k() {}\nexport { default } from \"m\";\nexport { k as default };\nexport { default as d } from \"m\";\n",
        &[(3, 10, "k", USAGE, Some(("k", 1)))],
        &[],
    ),
    (
        "R7-F3.X2.04 TypeScript type re-export, import alias, export assignment and UMD name",
        "ts",
        "export type { T } from \"m\";\nexport import A = N.S;\nfunction k() {}\nexport = k;\nexport as namespace X;\n",
        &[(2, 19, "N", USAGE, None), (4, 10, "k", USAGE, Some(("k", 3)))],
        &[],
    ),
    (
        "R7-F3.X2.05 JavaScript re-exports and an exported alias",
        "js",
        "function k() {}\nconst j = 1;\nexport { k as j } from \"m\";\nexport { k as j };\nexport * as k from \"m\";\nexport * from \"m\";\n",
        &[(4, 10, "k", USAGE, Some(("k", 1)))],
        &[],
    ),
    (
        "R7-F3.X2.06 summaries keep only the real call",
        "ts",
        "function k() {}\nfunction g() { k(); }\nexport { k as g };\nexport { g as k } from \"m\";\n",
        &[(2, 16, "k", CALL, Some(("k", 1))), (3, 10, "k", USAGE, Some(("k", 1)))],
        &[("g", &["k"])],
    ),
];

#[test]
fn export_clause_names_are_never_read_as_local_bindings() {
    for &(label, extension, source, expected, summaries) in EXPORT_CLAUSE_MATRIX {
        let file = Analyzed::new(source, extension);
        let observed: Vec<(u32, u32, &str, OccurrenceKind, Option<SymbolId>)> = file
            .evidence
            .occurrences
            .iter()
            .map(|occurrence| {
                if occurrence.target.id.is_none() {
                    assert_uncertain(occurrence);
                }
                (
                    occurrence.span.start.line,
                    occurrence.span.start.column,
                    occurrence.target.name.as_str(),
                    occurrence.kind,
                    occurrence.target.id.clone(),
                )
            })
            .collect();
        let expected: Vec<(u32, u32, &str, OccurrenceKind, Option<SymbolId>)> = expected
            .iter()
            .map(|&(line, column, name, kind, target)| {
                (line, column, name, kind, target.map(|(target, at)| file.id(target, at).clone()))
            })
            .collect();
        assert_eq!(observed, expected, "{label}: occurrences");

        for record in &file.index.symbols {
            let name = record.identity.name.as_str();
            let listed = summaries.iter().find(|(owner, _)| *owner == name).map_or(&[][..], |(_, names)| *names);
            let references: Vec<&str> = record.relations.references.iter().map(|target| target.name.as_str()).collect();
            assert_eq!(references, listed, "{label}: `{name}` references");
            let callers: Vec<&str> = record.relations.referenced_by.iter().map(|caller| caller.name.as_str()).collect();
            let expected_callers: Vec<&str> = summaries
                .iter()
                .filter(|(_, names)| names.contains(&name))
                .map(|(owner, _)| *owner)
                .collect();
            assert_eq!(callers, expected_callers, "{label}: `{name}` referenced by");
            assert!(record.relations.dependencies.is_empty(), "{label}: `{name}` has dependencies");
        }

        // The diagnostics references read the same tokens.
        let mut temp = tempfile::Builder::new().suffix(&format!(".{extension}")).tempfile().expect("temp");
        temp.write_all(source.as_bytes()).expect("write");
        let referenced: Vec<(usize, usize)> = TreeFile::new(temp.path())
            .expect("parse")
            .referenced_symbols()
            .expect("references")
            .iter()
            .map(|reference| (reference.range.start_line, reference.range.start_column))
            .collect();
        let read_positions: Vec<(usize, usize)> =
            expected.iter().map(|&(line, column, ..)| (line as usize, column as usize)).collect();
        for position in &referenced {
            assert!(read_positions.contains(position), "{label}: diagnostics reference at {position:?}");
        }
    }
}

// ---- Languages ---------------------------------------------------------------

/// The languages whose references resolve through lexical scopes.
const RESOLVED_LANGUAGES: [ProgrammingLanguage; 3] = [
    ProgrammingLanguage::Rust,
    ProgrammingLanguage::TypeScript,
    ProgrammingLanguage::JavaScript,
];

#[test]
fn every_language_reports_occurrences_with_an_explicit_state() {
    for language in ProgrammingLanguage::all() {
        let path = primary_fixture(language).unwrap_or_else(|| panic!("no fixture for {}", language.name()));
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let evidence = tree_file.reference_evidence().expect("evidence");
        let label = language.name();

        assert!(!evidence.occurrences.is_empty(), "{label}: a limitation is not an empty result");
        assert!(!evidence.reference_coverage.is_empty(), "{label}");
        assert_eq!(evidence.reference_coverage[0].owner, None, "{label}: the file region comes first");

        if RESOLVED_LANGUAGES.contains(&language) {
            assert!(
                evidence
                    .reference_coverage
                    .iter()
                    .all(|coverage| !coverage.reasons.contains(&LimitReason::UnsupportedLanguage)),
                "{label}"
            );
            assert!(
                evidence.occurrences.iter().any(|occurrence| occurrence.confidence == Confidence::Scoped),
                "{label}: a resolved language establishes bindings"
            );
        } else {
            for coverage in &evidence.reference_coverage {
                assert_eq!(coverage.state, CoverageState::Limited, "{label}");
                assert!(coverage.reasons.contains(&LimitReason::UnsupportedLanguage), "{label}");
            }
            for occurrence in &evidence.occurrences {
                assert_uncertain(occurrence);
            }
        }
    }
}

crate::language_cases::language_tests!(owner_target_and_candidate_ids_are_the_complete_index_ids => check_owner_target_and_candidate_ids_are_the_complete_index_ids);

fn check_owner_target_and_candidate_ids_are_the_complete_index_ids(language: ProgrammingLanguage) {
    for language in [language] {
        let path = primary_fixture(language).expect("fixture");
        let tree_file = TreeFile::new(&path).expect("parse fixture");
        let evidence = tree_file.reference_evidence().expect("evidence");
        let ids: HashSet<SymbolId> = tree_file
            .symbol_index_v2()
            .expect("index")
            .symbols
            .into_iter()
            .map(|record| record.id)
            .collect();
        let named = evidence
            .occurrences
            .iter()
            .flat_map(|occurrence| {
                occurrence
                    .owner
                    .iter()
                    .chain(std::iter::once(&occurrence.target))
                    .chain(&occurrence.candidates)
            })
            .chain(evidence.reference_coverage.iter().flat_map(|coverage| &coverage.owner))
            .filter_map(|reference| reference.id.as_ref());
        for id in named {
            assert!(ids.contains(id), "{}: {id:?} is not a record ID", language.name());
        }
    }
}

crate::language_cases::language_tests!(occurrences_and_coverage_are_in_canonical_order => check_occurrences_and_coverage_are_in_canonical_order);

fn check_occurrences_and_coverage_are_in_canonical_order(language: ProgrammingLanguage) {
    for language in [language] {
        let path = primary_fixture(language).expect("fixture");
        let evidence = TreeFile::new(&path).expect("parse").reference_evidence().expect("evidence");
        let spans: Vec<_> = evidence
            .occurrences
            .iter()
            .map(|occurrence| (occurrence.span.start_byte, occurrence.span.end_byte))
            .collect();
        let mut sorted = spans.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(spans, sorted, "{}: sorted by span, one occurrence per span", language.name());
        for occurrence in &evidence.occurrences {
            let mut candidates = occurrence.candidates.clone();
            candidates.dedup_by(|left, right| left.id == right.id);
            assert_eq!(candidates.len(), occurrence.candidates.len(), "distinct candidates");
        }
        for coverage in &evidence.reference_coverage {
            assert_eq!(coverage.reasons.is_empty(), coverage.state == CoverageState::Complete);
            assert!(coverage.reasons.windows(2).all(|pair| pair[0] < pair[1]));
        }
    }
}

/// Candidates are listed by their record's name start (then ID), each once,
/// whichever way they were gathered: every same-named record (a language
/// without binding rules), a filtered subset (method calls), or several
/// visible bindings in lookup order.
#[test]
fn name_only_candidates_are_in_declaration_order_and_distinct() {


    // Every same-named record: each `self` lists every method's `self`.
    let python = Analyzed::new(
        "class Big:\n    def c(self):\n        return self\n    def a(self):\n        return self\n    def b(self):\n        return self\n",
        "py",
    );
    let selves = [python.named("self", 2), python.named("self", 4), python.named("self", 6)];
    let uses = python.all("self");
    assert_eq!(uses.len(), 3, "{uses:#?}");
    for occurrence in uses {
        assert_eq!(occurrence.confidence, Confidence::NameOnly);
        let ids: Vec<_> = occurrence.candidates.iter().map(|candidate| candidate.id.as_ref().expect("ID")).collect();
        assert_eq!(ids, selves, "declaration order");
    }
    assert_candidate_order(&python.index, &python.evidence, "python");

    // A filtered subset: `x.m()` lists the in-file `m` methods.
    let rust = Analyzed::new(
        "struct B;\nimpl B { fn m(&self) {} }\nstruct A;\nimpl A { fn m(&self) {} }\nfn g(x: A) { x.m(); }\n",
        "rs",
    );
    let call = rust.at(5, "m", 0);
    assert_eq!(call.confidence, Confidence::NameOnly);
    let ids: Vec<_> = call.candidates.iter().map(|candidate| candidate.id.as_ref().expect("ID")).collect();
    assert_eq!(ids, [rust.named("m", 2), rust.named("m", 4)], "declaration order");

    // Several visible bindings: a body `var` that redeclares a parameter.
    let js = Analyzed::new("function g(f) {\n  var f = 1;\n  f;\n}\n", "js");
    let read = js.at(3, "f", 0);
    assert_eq!(read.confidence, Confidence::NameOnly);
    let ids: Vec<_> = read.candidates.iter().map(|candidate| candidate.id.as_ref().expect("ID")).collect();
    assert_eq!(ids, [js.named("f", 1), js.named("f", 2)], "declaration order");
}

#[test]
fn defaults_belong_to_the_function_and_decorators_to_the_enclosing_scope() {
    let source = "\
function seed(): number { return 1; }
function track(target: any): void {}
function make(n = seed()): number { return n; }
class Widget {
  @track
  run(): void {}
}
";
    let file = Analyzed::new(source, "ts");
    let default = file.at(3, "seed", 0);
    assert_eq!(default.kind, OccurrenceKind::Call);
    assert_eq!(default.owner.as_ref().and_then(|owner| owner.id.as_ref()), Some(file.id("make", 3)));
    assert_scoped_to(default, file.id("seed", 1));

    let decorator = file.at(5, "track", 0);
    assert_eq!(decorator.owner, None, "{decorator:#?}");
    assert_eq!(decorator.owner_span, file.evidence.reference_coverage[0].span);
    assert_scoped_to(decorator, file.id("track", 2));
}

#[test]
fn ecma_with_and_eval_limit_their_region() {
    let file = Analyzed::new("function f(o) { with (o) { g(); } eval(\"g()\"); }\nfunction g() {}\n", "js");
    let region = file
        .evidence
        .reference_coverage
        .iter()
        .find(|coverage| coverage.owner.as_ref().is_some_and(|owner| owner.name == "f"))
        .expect("f region");
    assert_eq!(
        region.reasons,
        [LimitReason::UnsupportedBindingForm, LimitReason::IndirectDispatch]
    );
}

// ---- Boundaries of lexical lookup ------------------------------------------

/// What one boundary row expects of an occurrence.
#[derive(Debug, Clone, Copy)]
enum Bound {
    /// `Scoped` to the record called `name` whose name starts on `line`.
    To(&'static str, u32),
    /// `Scoped`, whatever the target (a binding without a record).
    Established,
    /// Never `Scoped`, never an ID.
    Uncertain,
    /// `Scoped` with no ID: a binding no record reports (an expression's
    /// own name).
    Local,
    /// No occurrence of the name on the line at all (a declaration name).
    Absent,
}

/// `(label, extension, source, [(line, name, nth on that line, expected)])`.
type BoundaryRow = (&'static str, &'static str, &'static str, &'static [(u32, &'static str, usize, Bound)]);

/// One row per scope boundary: Rust items and inline modules, conditional
/// pattern bindings, ECMA `with` bodies and direct `eval`, each with the
/// controls that must stay established.
const LEXICAL_BOUNDARY_MATRIX: &[BoundaryRow] = &[
    (
        "an inline module cannot see its parent's items",
        "rs",
        "fn f() {}\nmod m { pub fn g() { f(); } }\n",
        &[(2, "f", 0, Bound::Uncertain)],
    ),
    (
        "an inline module's import binds the name (control)",
        "rs",
        "fn f() {}\nmod m { use super::f; pub fn g() { f(); } }\n",
        &[(2, "f", 0, Bound::Established)],
    ),
    (
        "an inline module sees an earlier `macro_rules!` (control)",
        "rs",
        "macro_rules! mm { () => {} }\nmod m { fn g() { mm!(); } }\n",
        &[(2, "mm", 0, Bound::To("mm", 1))],
    ),
    (
        "a nested function cannot capture an enclosing `let`",
        "rs",
        "fn f() {\n    let x = || {};\n    fn g() { x(); }\n}\n",
        &[(3, "x", 0, Bound::Uncertain)],
    ),
    (
        "a nested function sees the enclosing block's items (control)",
        "rs",
        "fn f() {\n    fn h() {}\n    fn g() { h(); }\n    struct S;\n    impl S { fn m() { h(); } }\n}\n",
        &[(3, "h", 0, Bound::To("h", 2)), (5, "h", 0, Bound::To("h", 2))],
    ),
    (
        "a nested function's own name wins over an enclosing `let`",
        "rs",
        "fn f() {\n    let g = 1;\n    fn g() { g(); }\n}\n",
        &[(3, "g", 0, Bound::To("g", 3))],
    ),
    (
        "a nested function cannot see an enclosing function's parameter",
        "rs",
        "fn f(p: fn()) {\n    fn g() { p(); }\n}\n",
        &[(2, "p", 0, Bound::Uncertain)],
    ),
    (
        "a nested function cannot see an enclosing generic parameter",
        "rs",
        "fn f<T>() {\n    fn g(_: T) {}\n}\n",
        &[(2, "T", 0, Bound::Uncertain)],
    ),
    (
        "a method sees its impl's generic parameter (control)",
        "rs",
        "struct S<T>(T);\nimpl<T> S<T> {\n    fn m(x: T) {}\n}\n",
        &[(3, "T", 0, Bound::Established)],
    ),
    (
        "a nested `const` cannot capture an enclosing `let`",
        "rs",
        "fn f() {\n    let x = || {};\n    const Y: () = x();\n}\n",
        &[(3, "x", 0, Bound::Uncertain)],
    ),
    (
        "a nested `static` cannot capture an enclosing `let`",
        "rs",
        "fn f() {\n    let x = || {};\n    static Y: () = x();\n}\n",
        &[(3, "x", 0, Bound::Uncertain)],
    ),
    (
        "a closure captures an enclosing `let` (control)",
        "rs",
        "fn f() {\n    let x = || {};\n    let g = || x();\n}\n",
        &[(3, "x", 0, Bound::To("x", 2))],
    ),
    (
        "an `if let` binding ends with its consequence",
        "rs",
        "fn f() {}\nfn g(x: Option<fn()>) {\n    if let Some(f) = x { f(); } else { f(); }\n    f();\n}\n",
        &[
            (3, "f", 0, Bound::To("f", 3)),
            (3, "f", 1, Bound::To("f", 1)),
            (4, "f", 0, Bound::To("f", 1)),
        ],
    ),
    (
        "a `let` chain's bindings end with its consequence",
        "rs",
        "fn f() {}\nfn g(x: Option<fn()>) {\n    if let Some(f) = x && true { f(); } else { f(); }\n    f();\n}\n",
        &[
            (3, "f", 0, Bound::To("f", 3)),
            (3, "f", 1, Bound::To("f", 1)),
            (4, "f", 0, Bound::To("f", 1)),
        ],
    ),
    (
        "a later `let` of a chain reads an earlier one (control)",
        "rs",
        "fn g(x: Option<u8>) {\n    if let Some(a) = x && let Some(b) = Some(a) { b; }\n}\n",
        &[(2, "a", 0, Bound::To("a", 2)), (2, "b", 0, Bound::To("b", 2))],
    ),
    (
        "an `else if let` sees only its own binding",
        "rs",
        "fn f() {}\nfn g(x: Option<fn()>) {\n    if true {} else if let Some(f) = x { f(); } else { f(); }\n}\n",
        &[(3, "f", 0, Bound::To("f", 3)), (3, "f", 1, Bound::To("f", 1))],
    ),
    (
        "a `while let` binding is the loop body's (control)",
        "rs",
        "fn f() {}\nfn g(x: Option<fn()>) {\n    while let Some(f) = x { f(); }\n    f();\n}\n",
        &[(3, "f", 0, Bound::To("f", 3)), (4, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a `for` binding is the loop body's (control)",
        "rs",
        "fn f() {}\nfn g(xs: Vec<fn()>) {\n    for f in xs { f(); }\n    f();\n}\n",
        &[(3, "f", 0, Bound::To("f", 3)), (4, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a `let … else` binds after the statement, not in its `else` (control)",
        "rs",
        "fn f() {}\nfn g(x: Option<fn()>) {\n    let Some(f) = x else { f(); return; };\n    f();\n}\n",
        &[(3, "f", 0, Bound::To("f", 1)), (4, "f", 0, Bound::To("f", 3))],
    ),
    (
        "a match arm's binding is the arm's own (control)",
        "rs",
        "fn f() {}\nfn g(x: Option<fn()>) {\n    match x { Some(f) => f(), None => f() }\n}\n",
        &[(3, "f", 0, Bound::To("f", 3)), (3, "f", 1, Bound::To("f", 1))],
    ),
    (
        "a typescript `with` body may supply any name",
        "ts",
        "function f(obj: any) { with (obj) { f(obj); } }\n",
        &[(1, "f", 0, Bound::Uncertain)],
    ),
    (
        "a javascript `with` body may supply any name",
        "js",
        "function f(obj) { with (obj) { f(obj); } }\n",
        &[(1, "f", 0, Bound::Uncertain)],
    ),
    (
        "a javascript `with` statement without a block",
        "js",
        "function f(obj) { with (obj) f(obj); }\n",
        &[(1, "f", 0, Bound::Uncertain)],
    ),
    (
        "a declaration inside a `with` body is established (control)",
        "js",
        "function f() {}\nfunction g(o) { with (o) { let f = 1; f; } }\nf();\n",
        &[(2, "f", 0, Bound::To("f", 2)), (3, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a direct `eval` may add a `var` to its function",
        "js",
        "function f() {}\nfunction g() { eval(\"var f = 1\"); f(); }\nf();\neval(\"1\");\nf();\n",
        &[
            (2, "f", 0, Bound::Uncertain),
            (3, "f", 0, Bound::To("f", 1)),
            (5, "f", 0, Bound::To("f", 1)),
        ],
    ),
    (
        "an optional or tagged `eval` adds no `var` (control)",
        "js",
        "function f() {}\nfunction g() { eval?.(\"var f = 1\"); f(); }\nfunction h() { eval`var f = 1`; f(); }\n",
        &[(2, "f", 0, Bound::To("f", 1)), (3, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a typescript default argument reads the outer scope (control)",
        "ts",
        "function f() {}\nfunction g(x = f()) { function f() {} }\n",
        &[(2, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a javascript default argument reads the outer scope (control)",
        "js",
        "function f() {}\nfunction g(x = f()) { function f() {} }\n",
        &[(2, "f", 0, Bound::To("f", 1))],
    ),
];

#[test]
fn lexical_lookup_stops_at_boundaries_that_establish_no_binding() {
    assert_bound_rows(LEXICAL_BOUNDARY_MATRIX);
}

/// How an `eval` call in [`EVAL_CALL_FORMS`] is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvalMode {
    /// A direct `eval`, which may add a `var` to its function.
    Direct,
    /// An optional call: an indirect eval in the global scope, which binds
    /// nothing here but may call anything.
    Optional,
    /// A tagged template, which evaluates nothing.
    Tagged,
    /// `eval` reached through another value (a sequence, a member, an
    /// alias): an indirect eval.
    Indirect,
}

impl EvalMode {
    /// The reasons the function holding the call is limited by.
    pub(crate) fn reasons(self) -> &'static [LimitReason] {
        match self {
            Self::Direct => &[LimitReason::UnsupportedBindingForm, LimitReason::IndirectDispatch],
            Self::Optional | Self::Indirect => INDIRECT,
            Self::Tagged => &[],
        }
    }
}

/// `(label, extensions, statement, mode)`: every way of invoking `eval`
/// the binding and coverage readers must tell apart, written in place of
/// line 4 of [`eval_source`]. Only a direct call may bind a name in the
/// function around it.
pub(crate) const EVAL_CALL_FORMS: &[(&str, &[&str], &str, EvalMode)] = &[
    ("direct", &["ts", "js"], "eval(\"var f = 1\");", EvalMode::Direct),
    ("parenthesized direct", &["ts", "js"], "((eval))(\"var f = 1\");", EvalMode::Direct),
    ("`as` direct", &["ts"], "(eval as any)(\"var f = 1\");", EvalMode::Direct),
    ("non-null direct", &["ts"], "eval!(\"var f = 1\");", EvalMode::Direct),
    ("optional", &["ts", "js"], "eval?.(\"var f = 1\");", EvalMode::Optional),
    ("parenthesized optional", &["ts", "js"], "(eval)?.(\"var f = 1\");", EvalMode::Optional),
    ("doubly parenthesized optional", &["ts", "js"], "((eval))?.(\"var f = 1\");", EvalMode::Optional),
    ("`as` optional", &["ts"], "(eval as any)?.(\"var f = 1\");", EvalMode::Optional),
    ("tagged", &["ts", "js"], "eval`var f = 1`;", EvalMode::Tagged),
    ("parenthesized tagged", &["ts", "js"], "(eval)`var f = 1`;", EvalMode::Tagged),
    ("non-null tagged", &["ts"], "(eval!)`var f = 1`;", EvalMode::Tagged),
    ("`as` tagged", &["ts"], "(eval as any)`var f = 1`;", EvalMode::Tagged),
    ("sequence", &["ts", "js"], "(0, eval)(\"var f = 1\");", EvalMode::Indirect),
    ("member", &["ts", "js"], "globalThis.eval(\"var f = 1\");", EvalMode::Indirect),
    ("alias", &["ts", "js"], "const e = eval; e(\"var f = 1\");", EvalMode::Indirect),
];

/// A nested `f` (line 3) holding `statement` (line 4), a class extending
/// the file's `Base` (line 5) and a call of its own name (line 6).
pub(crate) fn eval_source(statement: &str) -> String {
    format!(
        "class Base {{}}\nfunction outer() {{\n    function f() {{\n        {statement}\n        class D extends Base {{}}\n        f();\n    }}\n}}\n"
    )
}

/// Only a direct `eval` makes its function a lookup barrier: an optional
/// or tagged call of `eval`, or one through another value, leaves the
/// self-call and the base established, the function `Yes`, and limits the
/// function only as [`EvalMode::reasons`] says.
#[test]
fn only_a_direct_eval_stops_lookup_and_limits_binding() {
    for &(label, extensions, statement, mode) in EVAL_CALL_FORMS {
        for &extension in extensions {
            let label = format!("{extension} {label} eval");
            let file = Analyzed::new(&eval_source(statement), extension);
            let call = file.at(6, "f", 0);
            let base = file.at(5, "Base", 0);
            let f = file.named("f", 3);
            if mode == EvalMode::Direct {
                for occurrence in [call, base] {
                    assert_ne!(occurrence.confidence, Confidence::Scoped, "{label}: {occurrence:#?}");
                    assert_eq!(occurrence.target.id, None, "{label}: {occurrence:#?}");
                }
            } else {
                assert_eq!(call.kind, OccurrenceKind::Call, "{label}");
                assert_eq!((call.confidence, call.target.id.as_ref()), (Confidence::Scoped, Some(f)), "{label}");
                assert!(call.candidates.is_empty(), "{label}: {call:#?}");
                assert_eq!(
                    (base.confidence, base.target.id.as_ref()),
                    (Confidence::Scoped, Some(file.named("Base", 1))),
                    "{label}"
                );
            }
            let region = file
                .evidence
                .reference_coverage
                .iter()
                .find(|coverage| coverage.owner.as_ref().and_then(|owner| owner.id.as_ref()) == Some(f))
                .unwrap_or_else(|| panic!("{label}: no `f` region"));
            let state = if mode.reasons().is_empty() { CoverageState::Complete } else { CoverageState::Limited };
            assert_eq!((region.state, region.reasons.as_slice()), (state, mode.reasons()), "{label}");
            let recursive = file.index.symbols.iter().find(|record| &record.id == f).and_then(|record| record.semantics.is_recursive);
            let expected = if mode == EvalMode::Direct { TriState::Unknown } else { TriState::Yes };
            assert_eq!(recursive, Some(expected), "{label}");
        }
    }
}

fn assert_bound_rows(rows: &[BoundaryRow]) {
    for &(label, extension, source, checks) in rows {
        let file = Analyzed::new(source, extension);
        for &(line, name, nth, expected) in checks {
            if matches!(expected, Bound::Absent) {
                let written = file
                    .all(name)
                    .into_iter()
                    .filter(|occurrence| occurrence.span.start.line == line)
                    .collect::<Vec<_>>();
                assert!(written.is_empty(), "{label}: `{name}` on line {line} is no occurrence: {written:#?}");
                continue;
            }
            let occurrence = file.at(line, name, nth);
            match expected {
                Bound::To(target, target_line) => {
                    assert_eq!(occurrence.confidence, Confidence::Scoped, "{label}: {occurrence:#?}");
                    assert_eq!(
                        occurrence.target.id.as_ref(),
                        Some(file.named(target, target_line)),
                        "{label}: {occurrence:#?}"
                    );
                }
                Bound::Established => assert_eq!(occurrence.confidence, Confidence::Scoped, "{label}: {occurrence:#?}"),
                Bound::Local => {
                    assert_eq!(occurrence.confidence, Confidence::Scoped, "{label}: {occurrence:#?}");
                    assert_eq!(occurrence.target.id, None, "{label}: {occurrence:#?}");
                    assert!(occurrence.candidates.is_empty(), "{label}: {occurrence:#?}");
                }
                Bound::Uncertain => {
                    assert_ne!(occurrence.confidence, Confidence::Scoped, "{label}: {occurrence:#?}");
                    assert_eq!(occurrence.target.id, None, "{label}: {occurrence:#?}");
                }
                Bound::Absent => unreachable!("handled above"),
            }
        }
    }
}

/// Every name an expression declares for itself alone (ECMA named function,
/// generator and class expressions), in both languages, with the controls
/// that keep binding the outer declaration; and the Rust names in a
/// separate namespace (loop and block labels, lifetimes), which are never
/// occurrences. In each ECMA row the expression's own name (line 2) is no
/// occurrence, its inner use (line 3) binds the expression-local name with
/// no ID, and the final use (line 5) keeps the outer declaration.
const EXPRESSION_NAME_MATRIX: &[BoundaryRow] = &[
    (
        "a typescript named function expression",
        "ts",
        "function f() {}\nconst g = function f() {\n  f();\n};\nf();\n",
        &[(2, "f", 0, Bound::Absent), (3, "f", 0, Bound::Local), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a javascript named function expression",
        "js",
        "function f() {}\nconst g = function f() {\n  f();\n};\nf();\n",
        &[(2, "f", 0, Bound::Absent), (3, "f", 0, Bound::Local), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a typescript named generator expression",
        "ts",
        "function f() {}\nconst g = function* f() {\n  f();\n};\nf();\n",
        &[(2, "f", 0, Bound::Absent), (3, "f", 0, Bound::Local), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a javascript named generator expression",
        "js",
        "function f() {}\nconst g = function* f() {\n  f();\n};\nf();\n",
        &[(2, "f", 0, Bound::Absent), (3, "f", 0, Bound::Local), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a named function expression's parameters see its name",
        "js",
        "function f() {}\nconst g = function f(x = f) {\n  return x;\n};\nf();\n",
        &[(2, "f", 0, Bound::Local), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a typescript named generator expression's parameters see its name",
        "ts",
        "function f() {}\nconst g = function* f(x = f) {\n  yield x;\n};\nf();\n",
        &[(2, "f", 0, Bound::Local), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a named function expression passed as an argument",
        "ts",
        "function f() {}\n[1].map(function f() {\n  f();\n});\nf();\n",
        &[(2, "f", 0, Bound::Absent), (3, "f", 0, Bound::Local), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a nested function expression's name shadows only inside it",
        "js",
        "function f() {}\nfunction h() {\n  const g = function f() { f(); };\n  f();\n}\n",
        &[(3, "f", 0, Bound::Local), (4, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a typescript unnamed function expression (control)",
        "ts",
        "function f() {}\nconst g = function () {\n  f();\n};\nf();\n",
        &[(3, "f", 0, Bound::To("f", 1)), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a javascript unnamed function expression (control)",
        "js",
        "function f() {}\nconst g = function () {\n  f();\n};\nf();\n",
        &[(3, "f", 0, Bound::To("f", 1)), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a typescript unnamed generator expression (control)",
        "ts",
        "function f() {}\nconst g = function* () {\n  f();\n};\nf();\n",
        &[(3, "f", 0, Bound::To("f", 1)), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a javascript unnamed generator expression (control)",
        "js",
        "function f() {}\nconst g = function* () {\n  f();\n};\nf();\n",
        &[(3, "f", 0, Bound::To("f", 1)), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a typescript class expression (control)",
        "ts",
        "class F {}\nconst G = class F {\n  m() { return F; }\n};\nF;\n",
        &[(2, "F", 0, Bound::Absent), (3, "F", 0, Bound::Local), (5, "F", 0, Bound::To("F", 1))],
    ),
    (
        "a javascript class expression (control)",
        "js",
        "class F {}\nconst G = class F {\n  m() { return F; }\n};\nF;\n",
        &[(2, "F", 0, Bound::Absent), (3, "F", 0, Bound::Local), (5, "F", 0, Bound::To("F", 1))],
    ),
    (
        "a typescript class expression's heritage sees its own name",
        "ts",
        "class F {}\nconst G = class F extends F {\n  m() { return F; }\n};\nF;\n",
        &[(2, "F", 0, Bound::Local), (3, "F", 0, Bound::Local), (5, "F", 0, Bound::To("F", 1))],
    ),
    (
        "a javascript class expression's heritage sees its own name",
        "js",
        "class F {}\nconst G = class F extends F {\n  m() { return F; }\n};\nF;\n",
        &[(2, "F", 0, Bound::Local), (3, "F", 0, Bound::Local), (5, "F", 0, Bound::To("F", 1))],
    ),
    (
        "a typescript arrow parameter (control)",
        "ts",
        "function f() {}\nconst g = (f: () => void) => {\n  f();\n};\nf();\n",
        &[(2, "f", 0, Bound::Absent), (3, "f", 0, Bound::To("f", 2)), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a javascript arrow parameter (control)",
        "js",
        "function f() {}\nconst g = f => {\n  f();\n};\nf();\n",
        &[(2, "f", 0, Bound::Absent), (3, "f", 0, Bound::Established), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a rust loop label is no occurrence",
        "rs",
        "fn f() {}\nfn g() {\n    'f: loop { break 'f; }\n    f();\n}\n",
        &[(3, "f", 0, Bound::Absent), (4, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a rust block label is no occurrence",
        "rs",
        "fn f() {}\nfn g() {\n    'f: { break 'f; }\n    f();\n}\n",
        &[(3, "f", 0, Bound::Absent), (4, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a rust lifetime is no occurrence",
        "rs",
        "fn a() {}\nfn g<'a>(x: &'a u8) -> &'a u8 {\n    a();\n    x\n}\n",
        &[(2, "a", 0, Bound::Absent), (3, "a", 0, Bound::To("a", 1))],
    ),
];

#[test]
fn expression_local_names_bind_only_inside_their_expression() {
    assert_bound_rows(EXPRESSION_NAME_MATRIX);
}

// ---- Parameter and body environments ---------------------------------------

/// Every ECMA callable form, as `(label, header, closing)`: the header opens
/// the body on line 2 with a parameter default calling `f`.
const PARAMETER_DEFAULT_FORMS: &[(&str, &str, &str)] = &[
    ("function declaration", "function g(x = f()) {", "}"),
    ("arrow function", "const g = (x = f()) => {", "};"),
    ("function expression", "const g = function (x = f()) {", "};"),
    ("generator declaration", "function* g(x = f()) {", "}"),
    ("generator expression", "const g = function* (x = f()) {", "};"),
    ("method", "class C { g(x = f()) {", "} }"),
    ("constructor", "class C { constructor(x = f()) {", "} }"),
];

/// The body declarations of `f` on line 4, as `(label, line 4)`: `var` (at
/// the body's top level and in a nested block) is the finding, the others
/// are controls that were always block scoped.
const BODY_DECLARATIONS: &[(&str, &str)] = &[
    ("var", "  var f = () => {};"),
    ("var in a nested block", "  { var f = () => {}; }"),
    ("let (control)", "  let f = () => {};"),
    ("const (control)", "  const f = () => {};"),
    ("function (control)", "  function f() {}"),
];

/// A parameter default is evaluated before the body, in an environment
/// that holds the parameters but none of the body's declarations: the
/// default's `f()` (line 2) calls the outer `f`, while the body's calls
/// before and after the declaration (lines 3 and 5, hoisting) and inside a
/// closure in the body (line 6) bind the body's `f`.
#[test]
fn parameter_defaults_never_see_the_body_declarations() {
    for (form, header, closing) in PARAMETER_DEFAULT_FORMS {
        for (declaration, line4) in BODY_DECLARATIONS {
            let source = format!(
                "function f() {{}}\n{header}\n  f();\n{line4}\n  f();\n  const h = () => f();\n{closing}\nf();\n"
            );
            for extension in ["ts", "js"] {
                let label: &'static str = Box::leak(format!("{extension} {form} with a body {declaration}").into_boxed_str());
                let source: &'static str = Box::leak(source.clone().into_boxed_str());
                assert_bound_rows(&[(
                    label,
                    extension,
                    source,
                    &[
                        (2, "f", 0, Bound::To("f", 1)),
                        (3, "f", 0, Bound::To("f", 4)),
                        (5, "f", 0, Bound::To("f", 4)),
                        (6, "f", 0, Bound::To("f", 4)),
                        (8, "f", 0, Bound::To("f", 1)),
                    ],
                )]);
            }
        }
    }
}

/// The parameter environment's other readers and writers: parameters stay
/// visible to later defaults and to the body, every parameter expression
/// (a destructuring default, a computed key, a typed default, a closure in
/// a default) separates the body's `var`s, and a body's direct `eval` limits
/// only the body.
const PARAMETER_ENVIRONMENT_MATRIX: &[BoundaryRow] = &[
    (
        "a parameter is visible to a later default and to the body",
        "js",
        "function f() {}\nfunction g(f = 1, x = f) {\n  f;\n  var y = 1;\n}\n",
        &[(2, "f", 0, Bound::To("f", 2)), (3, "f", 0, Bound::To("f", 2))],
    ),
    (
        "a body `var` named like a parameter is the body's own binding",
        "js",
        "function g(f, x = 1) {\n  var f = 2;\n  f;\n}\n",
        &[(3, "f", 0, Bound::To("f", 2))],
    ),
    (
        "a destructuring default does not see a body `var`",
        "js",
        "function f() {}\nfunction g({ a = f() }) {\n  var f = () => {};\n}\n",
        &[(2, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a computed key does not see a body `var`",
        "ts",
        "function f() {}\nfunction g({ [f()]: a }) {\n  var f = () => {};\n}\n",
        &[(2, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a typed default does not see a body `var`",
        "ts",
        "function f() { return 1; }\nfunction g(x: number = f()) {\n  var f = () => 2;\n}\n",
        &[(2, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a closure in a default does not see a body `var`",
        "js",
        "function f() {}\nfunction g(x = () => f()) {\n  var f = () => {};\n}\n",
        &[(2, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a `for … in` `var` in the body stays in the body",
        "js",
        "function f() {}\nfunction g(x = f()) {\n  for (var f in {}) {}\n  f;\n}\n",
        &[(2, "f", 0, Bound::To("f", 1)), (4, "f", 0, Bound::Established)],
    ),
    (
        "a destructured body `var` stays in the body",
        "js",
        "function f() {}\nfunction g(x = f()) {\n  var { f } = {};\n  f;\n}\n",
        &[(2, "f", 0, Bound::To("f", 1)), (4, "f", 0, Bound::Established)],
    ),
    (
        "a direct `eval` in the body limits the body, not the defaults",
        "js",
        "function f() {}\nfunction g(x = f()) {\n  eval(\"var f = 1\");\n  f();\n}\n",
        &[(2, "f", 0, Bound::To("f", 1)), (4, "f", 0, Bound::Uncertain)],
    ),
    (
        "without parameter expressions a body `var` redeclares the parameter (control)",
        "js",
        "function g(f) {\n  var f = 1;\n  f;\n}\n",
        &[(3, "f", 0, Bound::Uncertain)],
    ),
    (
        "a body `var` is hoisted without parameter expressions (control)",
        "js",
        "function f() {}\nfunction g(x) {\n  f();\n  var f = () => {};\n}\n",
        &[(3, "f", 0, Bound::To("f", 4))],
    ),
];

#[test]
fn the_parameter_environment_holds_parameters_but_not_body_vars() {
    assert_bound_rows(PARAMETER_ENVIRONMENT_MATRIX);
}

// ---- Pattern names: bindings or constants ----------------------------------

/// A Rust identifier pattern binds or names a constant by what lookup finds
/// where it is written, never by its spelling: every row of the review's
/// table (a capitalized name binding a function-pointer value, a lowercase
/// constant read by a pattern), each pattern position (`let`, `let … else`,
/// parameters, closure parameters, `for`, `if let`, `while let`, `match`
/// arms and guards), nested forms (tuple-struct, struct shorthand, slice,
/// or, `@`, references, generic paths) and the names whose identity the
/// file cannot establish (an import or glob in view). A pattern name that
/// binds is no occurrence (`Absent`); one that reads is a usage. Every
/// source compiles with `rustc --edition 2024`.
const PATTERN_IDENTITY_MATRIX: &[BoundaryRow] = &[
    (
        "an `if let` capitalized name binds a new local",
        "rs",
        "fn F(x: fn()) {\n    if let F = x {\n        F();\n    }\n}\n",
        &[(2, "F", 0, Bound::Absent), (3, "F", 0, Bound::Local)],
    ),
    (
        "a `while let` capitalized name binds a new local",
        "rs",
        "fn F(x: fn()) {\n    while let F = x {\n        F();\n        break;\n    }\n}\n",
        &[(2, "F", 0, Bound::Absent), (3, "F", 0, Bound::Local)],
    ),
    (
        "a match arm capitalized name binds a new local",
        "rs",
        "fn F(x: fn()) {\n    match x {\n        F => {\n            F()\n        }\n    }\n}\n",
        &[(3, "F", 0, Bound::Absent), (4, "F", 0, Bound::Local)],
    ),
    (
        "a `mut` match binding binds a new local",
        "rs",
        "fn F(x: fn()) {\n    match x {\n        mut F => {\n            F()\n        }\n    }\n}\n",
        &[(3, "F", 0, Bound::Absent), (4, "F", 0, Bound::Local)],
    ),
    (
        "a `ref` match binding binds a new local",
        "rs",
        "fn F(x: fn()) {\n    match x {\n        ref F => {\n            F()\n        }\n    }\n}\n",
        &[(3, "F", 0, Bound::Absent), (4, "F", 0, Bound::Local)],
    ),
    (
        "a lowercase constant in a match pattern is read, not bound",
        "rs",
        "const f: u8 = 1;\npub fn g(x: u8) {\n    match x {\n        f => {\n            f;\n        }\n        _ => {}\n    }\n}\n",
        &[(4, "f", 0, Bound::To("f", 1)), (5, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a lowercase constant in `if let` and `while let` is read",
        "rs",
        "const f: u8 = 1;\npub fn g(x: u8) {\n    if let f = x {\n        f;\n    }\n    while let f = x {\n        f;\n        break;\n    }\n}\n",
        &[
            (3, "f", 0, Bound::To("f", 1)),
            (4, "f", 0, Bound::To("f", 1)),
            (6, "f", 0, Bound::To("f", 1)),
            (7, "f", 0, Bound::To("f", 1)),
        ],
    ),
    (
        "a lowercase constant before a match guard is read by both",
        "rs",
        "const f: u8 = 1;\npub fn g(x: u8) {\n    match x {\n        f if f == 1 => {}\n        _ => {}\n    }\n}\n",
        &[(4, "f", 0, Bound::To("f", 1)), (4, "f", 1, Bound::To("f", 1))],
    ),
    (
        "an uppercase constant in a match pattern is read (control)",
        "rs",
        "const F: u8 = 1;\npub fn g(x: u8) {\n    match x {\n        F => {\n            F;\n        }\n        _ => {}\n    }\n}\n",
        &[(4, "F", 0, Bound::To("F", 1)), (5, "F", 0, Bound::To("F", 1))],
    ),
    (
        "an explicit constant path stays qualified and uncertain (control)",
        "rs",
        "mod m {\n    pub const f: u8 = 1;\n}\npub fn g(x: u8) {\n    match x {\n        m::f => {\n            m::f;\n        }\n        _ => {}\n    }\n}\n",
        &[(6, "f", 0, Bound::Uncertain), (7, "f", 0, Bound::Uncertain)],
    ),
    (
        "captured tuple-struct bindings are locals, later calls the function (control)",
        "rs",
        "fn F(x: Option<fn()>) {\n    if let Some(F) = x {\n        F();\n    }\n    F(None);\n    while let Some(F) = x {\n        F();\n        break;\n    }\n    match x {\n        Some(F) => {\n            F()\n        }\n        None => F(None),\n    }\n}\n",
        &[
            (2, "F", 0, Bound::Absent),
            (3, "F", 0, Bound::To("F", 2)),
            (5, "F", 0, Bound::To("F", 1)),
            (6, "F", 0, Bound::Absent),
            (7, "F", 0, Bound::To("F", 6)),
            (11, "F", 0, Bound::Absent),
            (12, "F", 0, Bound::To("F", 11)),
            (14, "F", 0, Bound::To("F", 1)),
        ],
    ),
    (
        "a struct field shorthand binds a new local (control)",
        "rs",
        "pub struct S {\n    F: fn(),\n}\nfn F(s: S) {\n    if let S { F } = s {\n        F();\n    }\n    match s {\n        S { F } => {\n            F()\n        }\n    }\n}\n",
        &[
            (5, "F", 0, Bound::Absent),
            (6, "F", 0, Bound::Local),
            (9, "F", 0, Bound::Absent),
            (10, "F", 0, Bound::Local),
        ],
    ),
    (
        "irrefutable `let`, parameter and `for` names bind their records (control)",
        "rs",
        "fn F(x: fn()) {\n    let F = x;\n    F();\n}\nfn G(F: fn()) {\n    F();\n}\nfn H(xs: Vec<fn()>) {\n    for F in xs {\n        F();\n    }\n}\n",
        &[
            (2, "F", 0, Bound::Absent),
            (3, "F", 0, Bound::To("F", 2)),
            (5, "F", 0, Bound::Absent),
            (6, "F", 0, Bound::To("F", 5)),
            (9, "F", 0, Bound::Absent),
            (10, "F", 0, Bound::To("F", 9)),
        ],
    ),
    (
        "a closure parameter binds its record",
        "rs",
        "fn F() {\n    let c = |F: fn()| {\n        F();\n    };\n    c(F);\n}\n",
        &[(2, "F", 0, Bound::Absent), (3, "F", 0, Bound::To("F", 2)), (5, "F", 0, Bound::To("F", 1))],
    ),
    (
        "a unit struct in `let` and parameter patterns is read",
        "rs",
        "pub struct S;\npub fn f() {\n    let S = S;\n}\npub fn g(S: S) {}\n",
        &[
            (3, "S", 0, Bound::To("S", 1)),
            (3, "S", 1, Bound::To("S", 1)),
            (5, "S", 0, Bound::To("S", 1)),
            (5, "S", 1, Bound::To("S", 1)),
        ],
    ),
    (
        "a lowercase constant in `let … else` is read",
        "rs",
        "const f: u8 = 1;\npub fn g(x: u8) {\n    let f = x else {\n        return;\n    };\n    f;\n}\n",
        &[(3, "f", 0, Bound::To("f", 1)), (6, "f", 0, Bound::To("f", 1))],
    ),
    (
        "a struct field shorthand naming a constant is read",
        "rs",
        "pub struct S {\n    f: u8,\n}\nconst f: u8 = 1;\npub fn g(s: S) {\n    if let S { f } = s {\n        f;\n    }\n}\n",
        &[(6, "f", 0, Bound::To("f", 4)), (7, "f", 0, Bound::To("f", 4))],
    ),
    (
        "a local named like a function is shadowed by a new pattern binding",
        "rs",
        "fn f() {}\npub fn g(x: fn()) {\n    let f = x;\n    match x {\n        f => {\n            f();\n        }\n    }\n}\n",
        &[(5, "f", 0, Bound::Absent), (6, "f", 0, Bound::Local)],
    ),
    (
        "an `@` binding binds its name",
        "rs",
        "fn F(x: Option<fn()>) {\n    match x {\n        F @ Some(_) => {\n            F;\n        }\n        None => {}\n    }\n}\n",
        &[(3, "F", 0, Bound::Absent), (4, "F", 0, Bound::Local)],
    ),
    (
        "constants inside or, slice and reference patterns are read; `rest @ ..` binds",
        "rs",
        "const A: u8 = 1;\nconst C: u8 = 2;\npub fn g(x: u8, xs: &[u8]) {\n    match x {\n        A | 2 => {\n            A;\n        }\n        _ => {}\n    }\n    match xs {\n        [C, rest @ ..] => {\n            rest;\n        }\n        _ => {}\n    }\n    match &x {\n        &C => {}\n        _ => {}\n    }\n}\n",
        &[
            (5, "A", 0, Bound::To("A", 1)),
            (6, "A", 0, Bound::To("A", 1)),
            (11, "C", 0, Bound::To("C", 2)),
            (11, "rest", 0, Bound::Absent),
            (12, "rest", 0, Bound::Local),
            (17, "C", 0, Bound::To("C", 2)),
        ],
    ),
    (
        "a nested function's pattern reads the enclosing block's constant",
        "rs",
        "pub fn outer() {\n    const k: u8 = 1;\n    fn inner(x: u8) {\n        match x {\n            k => {}\n            _ => {}\n        }\n    }\n}\n",
        &[(5, "k", 0, Bound::To("k", 2))],
    ),
    (
        "the prelude's `None`, bare or with arguments, is never a binding",
        "rs",
        "pub fn g(x: Option<u8>) {\n    match x {\n        None::<u8> => {}\n        _ => {}\n    }\n    match x {\n        None => {}\n        Some(_) => {}\n    }\n}\n",
        &[(3, "None", 0, Bound::Uncertain), (7, "None", 0, Bound::Uncertain)],
    ),
    (
        "a glob in view leaves a refutable name unknown, an irrefutable one a binding",
        "rs",
        "mod m {\n    pub const f: u8 = 1;\n}\nuse m::*;\npub fn g(x: u8) {\n    match x {\n        f => {\n            f;\n        }\n        _ => {}\n    }\n    let y = x;\n    y;\n}\n",
        &[
            (7, "f", 0, Bound::Uncertain),
            (8, "f", 0, Bound::Uncertain),
            (12, "y", 0, Bound::Absent),
            (13, "y", 0, Bound::To("y", 12)),
        ],
    ),
    (
        "an imported name in a refutable pattern is unknown",
        "rs",
        "mod m {\n    pub const F: u8 = 1;\n}\nuse m::F;\npub fn g(x: u8) {\n    match x {\n        F => {\n            F;\n        }\n        _ => {}\n    }\n}\n",
        &[(7, "F", 0, Bound::Uncertain), (8, "F", 0, Bound::Uncertain)],
    ),
];

#[test]
fn bare_pattern_names_bind_or_read_by_lookup_never_by_spelling() {
    assert_bound_rows(PATTERN_IDENTITY_MATRIX);
}

/// `(label, extension, source, every local record as (name, line))`.
type PatternRecordRow = (&'static str, &'static str, &'static str, &'static [(&'static str, u32)]);

/// The local records pattern captures produce, which must agree with the
/// occurrence rule above: a pattern name that reads a constant, unit struct
/// or `const` parameter, or whose identity is unknown in a refutable
/// pattern, declares nothing; so does a tuple-struct pattern's path and a
/// TS/JS parameter default. Every Rust source compiles with
/// `rustc --edition 2024`.
const PATTERN_RECORD_MATRIX: &[PatternRecordRow] = &[
    (
        "`Some(C)` reading a constant in `if let`, `while let` and `match`",
        "rs",
        "const C: u8 = 1;\npub fn g(x: Option<u8>) {\n    if let Some(C) = x {}\n    while let Some(C) = x {\n        break;\n    }\n    \
         match x {\n        Some(C) => {}\n        _ => {}\n    }\n}\n",
        &[("x", 2)],
    ),
    (
        "a struct shorthand reading a constant",
        "rs",
        "pub struct S {\n    pub f: u8,\n}\nconst f: u8 = 1;\npub fn g(s: S) {\n    if let S { f } = s {}\n}\n",
        &[("s", 5)],
    ),
    (
        "a unit struct and a unit-typed constant in irrefutable patterns",
        "rs",
        "pub struct Unit;\nconst U: () = ();\npub fn g(Unit: Unit) {\n    let Unit = Unit;\n    let (a, U) = (1, ());\n    a;\n}\n",
        &[("a", 5)],
    ),
    (
        "an import in view leaves a refutable name undeclared, an irrefutable one declared",
        "rs",
        "mod m {\n    pub const D: u8 = 2;\n}\nuse m::D;\npub fn g(x: Option<u8>) {\n    match x {\n        Some(D) => {}\n        _ => {}\n    }\n    \
         let (a, b) = (1, 2);\n    a + b;\n}\n",
        &[("x", 5), ("a", 10), ("b", 10)],
    ),
    (
        "a glob in view leaves refutable names undeclared",
        "rs",
        "mod m {\n    pub fn helper() {}\n}\nuse m::*;\npub fn h(x: Option<u8>) {\n    if let Some(y) = x {\n        y;\n    }\n}\n",
        &[("x", 5)],
    ),
    (
        "`Some(x)` and `Some(C)` with no constant in view declare locals (control)",
        "rs",
        "pub fn h(x: Option<u8>) {\n    if let Some(y) = x {\n        y;\n    }\n    if let Some(C) = x {\n        C;\n    }\n}\n",
        &[("x", 1), ("y", 2), ("C", 5)],
    ),
    (
        "`Some(mut C)` binds, and the locals query records no moded name (control)",
        "rs",
        "pub fn h(x: Option<u8>) {\n    if let Some(mut C) = x {\n        C += 1;\n        C;\n    }\n}\n",
        &[("x", 1)],
    ),
    (
        "a typescript parameter default declares nothing",
        "ts",
        "const a = 1;\nexport function f(b = a) {\n  return b;\n}\n",
        &[("b", 2)],
    ),
    (
        "a javascript parameter default declares nothing",
        "js",
        "const a = 1;\nexport function f(b = a) {\n  return b;\n}\n",
        &[("b", 2)],
    ),
];

#[test]
fn pattern_names_that_bind_nothing_declare_no_record() {
    for &(label, extension, source, expected) in PATTERN_RECORD_MATRIX {
        let file = Analyzed::new(source, extension);
        let locals: Vec<(&str, u32)> = file
            .index
            .symbols
            .iter()
            .filter(|record| record.scope == SymbolScope::Local)
            .map(|record| {
                let line = record.source.name_span.as_ref().map_or(0, |span| span.start.line);
                (record.identity.name.as_str(), line)
            })
            .collect();
        assert_eq!(locals, expected, "{label}");
    }
}

/// `(label, extension, source, every record as (name, line), the names whose
/// occurrences are checked, every occurrence of those names as (line,
/// column, name, kind))`.
type ParameterCaptureRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, u32)],
    &'static [&'static str],
    &'static [(u32, u32, &'static str, OccurrenceKind)],
);

/// A parameter (or other binder) capture declares only the name it binds:
/// a parameter's type, its default value, a constant's value and a pattern's
/// path or range bound stay references in every language, resolving or
/// not, so they keep their occurrence and never add a record. Rows are
/// review rows `R5-F4.nn` and their sweep siblings `R5-F4.Sn`.
const PARAMETER_CAPTURE_MATRIX: &[ParameterCaptureRow] = &[
    (
        "R5-F4.01 python default",
        "py",
        "def external(): pass\ndef g(x=external): pass\n",
        &[("external", 1), ("g", 2), ("x", 2)],
        &["external", "x"],
        &[(2, 9, "external", USAGE)],
    ),
    (
        "R5-F4.02 python typed default",
        "py",
        "class W: pass\ndef external(): pass\ndef g(x: W=external): pass\n",
        &[("W", 1), ("external", 2), ("g", 3), ("x", 3)],
        &["external", "W", "x"],
        &[(3, 10, "W", USAGE), (3, 12, "external", USAGE)],
    ),
    (
        "R5-F4.03 python async, keyword-only and positional-only defaults",
        "py",
        "def external(): pass\nasync def a(x=external): pass\ndef k(*, x=external): pass\ndef p(x=external, /): pass\n",
        &[("external", 1), ("a", 2), ("x", 2), ("k", 3), ("x", 3), ("p", 4), ("x", 4)],
        &["external", "x"],
        &[(2, 15, "external", USAGE), (3, 12, "external", USAGE), (4, 9, "external", USAGE)],
    ),
    (
        "R5-F4.04 python method and initializer defaults",
        "py",
        "def external(): pass\nclass G:\n    def method(self, x=external): pass\n    def __init__(self, x: int=external): pass\n",
        &[("external", 1), ("G", 2), ("method", 3), ("self", 3), ("x", 3), ("__init__", 4), ("self", 4), ("x", 4)],
        &["external", "self", "x", "int"],
        &[(3, 24, "external", USAGE), (4, 27, "int", USAGE), (4, 31, "external", USAGE)],
    ),
    (
        "R5-F4.05 python lambda default",
        "py",
        "def external(): pass\nh = lambda x=external: x\n",
        &[("external", 1), ("h", 2), ("x", 2)],
        &["external", "x"],
        &[(2, 14, "external", USAGE), (2, 24, "x", USAGE)],
    ),
    (
        "R5-F4.06 python grouped, member and called defaults (control)",
        "py",
        "def external(): pass\ndef g(x=(external), y=obj.external, z=external()): pass\n",
        &[("external", 1), ("g", 2), ("x", 2), ("y", 2), ("z", 2)],
        &["external", "obj", "x", "y", "z"],
        &[(2, 10, "external", USAGE), (2, 23, "obj", USAGE), (2, 27, "external", USAGE), (2, 39, "external", CALL)],
    ),
    (
        "R5-F4.07 python annotations, plain and variadic parameters (control)",
        "py",
        "class W: pass\ndef g(x: W, n, *args, **kwargs): pass\n",
        &[("W", 1), ("g", 2), ("x", 2), ("n", 2), ("args", 2), ("kwargs", 2)],
        &["W", "x", "n", "args", "kwargs"],
        &[(2, 10, "W", USAGE)],
    ),
    (
        "R5-F4.S1 python class-body targets and the walrus operator",
        "py",
        "def external(): pass\nclass G:\n    obj.attr = 1\n    d[k] = 1\n    a, b = external, 2\nif (y := external):\n    pass\n",
        &[("external", 1), ("G", 2), ("a", 5), ("b", 5), ("y", 6)],
        &["external", "obj", "attr", "d", "k", "a", "b", "y"],
        &[
            (3, 5, "obj", USAGE),
            (3, 9, "attr", USAGE),
            (4, 5, "d", USAGE),
            (4, 7, "k", USAGE),
            (5, 12, "external", USAGE),
            (6, 10, "external", USAGE),
        ],
    ),
    (
        "R5-F4.08 c# method and interface signature types",
        "cs",
        "class W {}\ninterface I { void Sig(W s); }\nclass G { void g(W x) {} }\n",
        &[("W", 1), ("I", 2), ("Sig", 2), ("s", 2), ("G", 3), ("g", 3), ("x", 3)],
        &["W", "s", "x"],
        &[(2, 24, "W", USAGE), (3, 18, "W", USAGE)],
    ),
    (
        "R5-F4.09 c# constructor, primary constructor and record parameters",
        "cs",
        "class W {}\nclass G { G(W x) {} }\nclass Primary(W p) {}\nrecord Rec(W p);\n",
        &[("W", 1), ("G", 2), ("G", 2), ("x", 2), ("Primary", 3), ("p", 3), ("Rec", 4), ("p", 4)],
        &["W", "x", "p"],
        &[(2, 13, "W", USAGE), (3, 15, "W", USAGE), (4, 12, "W", USAGE)],
    ),
    (
        "R5-F4.10 c# local function and typed lambda parameters",
        "cs",
        "class W {}\nclass G {\n    void k() {\n        void Local(W q) {}\n        System.Func<W, W> f = (W q) => q;\n    }\n}\n",
        &[("W", 1), ("G", 2), ("k", 3), ("Local", 4), ("q", 4), ("f", 5), ("q", 5)],
        &["W", "q"],
        &[(4, 20, "W", USAGE), (5, 21, "W", USAGE), (5, 24, "W", USAGE), (5, 32, "W", USAGE), (5, 40, "q", USAGE)],
    ),
    (
        "R5-F4.11 c# delegate, indexer and operator parameters",
        "cs",
        "class W {}\ndelegate void D(W p);\nclass G {\n    int this[W p] => 0;\n    public static G operator +(G x, W y) => x;\n}\n",
        &[("W", 1), ("p", 2), ("G", 3), ("p", 4), ("x", 5), ("y", 5)],
        &["W", "p", "x", "y"],
        &[(2, 17, "W", USAGE), (4, 14, "W", USAGE), (5, 37, "W", USAGE), (5, 45, "x", USAGE)],
    ),
    (
        "R5-F4.12 c# default value",
        "cs",
        "class G {\n    const int external = 1;\n    void g(int y = external) {}\n}\n",
        &[("G", 1), ("external", 2), ("g", 3), ("y", 3)],
        &["external", "y"],
        &[(3, 20, "external", USAGE)],
    ),
    (
        "R5-F4.13 c# array, generic and grouped forms and an untyped lambda parameter (control)",
        "cs",
        "class W {}\nclass G {\n    const int external = 1;\n    void g(W[] a, List<W> l, int z = (external)) {}\n    \
         System.Func<int, int> u = q => q;\n}\n",
        &[("W", 1), ("G", 2), ("external", 3), ("g", 4), ("a", 4), ("l", 4), ("z", 4), ("u", 5)],
        &["W", "external", "q", "a", "l", "z"],
        &[(4, 12, "W", USAGE), (4, 24, "W", USAGE), (4, 39, "external", USAGE), (5, 36, "q", USAGE)],
    ),
    (
        "R5-F4.14 typescript defaults (control)",
        "ts",
        "const external = () => 1;\nexport function g(x = external, y = external()) {\n  return x + y;\n}\n",
        &[("external", 1), ("g", 2), ("x", 2), ("y", 2)],
        &["external", "x", "y"],
        &[(2, 23, "external", USAGE), (2, 37, "external", CALL), (3, 10, "x", USAGE), (3, 14, "y", USAGE)],
    ),
    (
        "R5-F4.14 javascript defaults (control)",
        "js",
        "const external = () => 1;\nexport function g(x = external, y = external()) {\n  return x + y;\n}\n",
        &[("external", 1), ("g", 2), ("x", 2), ("y", 2)],
        &["external", "x", "y"],
        &[(2, 23, "external", USAGE), (2, 37, "external", CALL), (3, 10, "x", USAGE), (3, 14, "y", USAGE)],
    ),
    (
        "R5-F4.15 c++ typed default (control)",
        "cpp",
        "struct W {};\nint external = 1;\nvoid g(W x, int y = external) {}\n",
        &[("W", 1), ("external", 2), ("g", 3), ("x", 3), ("y", 3)],
        &["W", "external", "x", "y"],
        &[(3, 8, "W", USAGE), (3, 21, "external", USAGE)],
    ),
    (
        "R5-F4.15 scala typed default (control)",
        "scala",
        "class W\nobject O {\n  val external = 1\n  def g(x: W, y: Int = external) = x\n}\n",
        &[("W", 1), ("O", 2), ("external", 3), ("g", 4), ("x", 4), ("y", 4)],
        &["W", "external", "x", "y"],
        &[(4, 12, "W", USAGE), (4, 24, "external", USAGE), (4, 36, "x", USAGE)],
    ),
    (
        "R5-F4.15 swift typed default (control)",
        "swift",
        "class W {}\nlet external = 1\nfunc g(x: W, y: Int = external) {}\n",
        &[("W", 1), ("external", 2), ("g", 3), ("x", 3), ("y", 3)],
        &["W", "external", "x", "y"],
        &[(3, 11, "W", USAGE), (3, 23, "external", USAGE)],
    ),
    (
        "R5-F4.15 php typed default; R5-F4.S2 php constant values and variable defaults",
        "php",
        "<?php\nconst A = 1;\nconst B = A;\nclass K {\n    const C = A;\n    public $p = 1;\n}\nfunction g(W $x, $y = A, $q = $z) {}\n",
        &[("A", 2), ("B", 3), ("K", 4), ("C", 5), ("p", 6), ("g", 8), ("x", 8), ("y", 8), ("q", 8)],
        &["A", "W", "x", "y", "q", "z"],
        &[(3, 11, "A", USAGE), (5, 15, "A", USAGE), (8, 12, "W", USAGE), (8, 23, "A", USAGE), (8, 32, "z", USAGE)],
    ),
    (
        "R5-F4.16 rust parameter (control)",
        "rs",
        "struct W;\nfn g(x: W) -> W {\n    x\n}\n",
        &[("W", 1), ("g", 2), ("x", 2)],
        &["W", "x"],
        &[(2, 9, "W", USAGE), (2, 15, "W", USAGE), (3, 5, "x", USAGE)],
    ),
    (
        "R5-F4.S3 rust `if let` path and range-bound patterns",
        "rs",
        "enum E {\n    A,\n}\nconst MAX: u8 = 3;\npub fn f(v: E, n: u8) {\n    if let E::A = v {}\n    if let 1..=MAX = n {}\n    \
         if let c @ 1..=MAX = n {\n        c;\n    }\n}\n",
        &[("E", 1), ("A", 2), ("MAX", 4), ("f", 5), ("v", 5), ("n", 5), ("c", 8)],
        &["E", "A", "MAX"],
        &[(5, 13, "E", USAGE), (6, 12, "E", USAGE), (6, 15, "A", USAGE), (7, 16, "MAX", USAGE), (8, 20, "MAX", USAGE)],
    ),
    (
        "R5-F4.16 go parameters (control)",
        "go",
        "package p\ntype W struct{}\nfunc g(x W, r ...W) {}\n",
        &[("p", 1), ("W", 2), ("g", 3), ("x", 3), ("r", 3)],
        &["W", "x", "r"],
        &[(3, 10, "W", USAGE), (3, 18, "W", USAGE)],
    ),
    (
        "R5-F4.16 java parameter (control)",
        "java",
        "class W {}\nclass G {\n    void g(W x) {}\n}\n",
        &[("W", 1), ("G", 2), ("g", 3), ("x", 3)],
        &["W", "x"],
        &[(3, 12, "W", USAGE)],
    ),
    (
        "R5-F4.16 c parameters (control)",
        "c",
        "typedef int W;\nint external = 1;\nvoid g(W x, int y[external]) {}\n",
        &[("W", 1), ("external", 2), ("g", 3), ("x", 3), ("y", 3)],
        &["W", "external", "x", "y"],
        &[(3, 8, "W", USAGE), (3, 19, "external", USAGE)],
    ),
    (
        "R5-F4.16 lua parameters (control)",
        "lua",
        "local external = 1\nlocal function g(x, y)\n  return external\nend\n",
        &[("external", 1), ("g", 2), ("x", 2), ("y", 2)],
        &["external", "x", "y"],
        &[(3, 10, "external", USAGE)],
    ),
    (
        "R5-F4.16 perl declarations (control)",
        "pl",
        "sub g {\n    my ($x, $y) = @_;\n    return $x;\n}\n",
        &[("g", 1), ("$x", 2), ("$y", 2)],
        &["$x", "$y"],
        &[(3, 12, "$x", USAGE)],
    ),
    (
        "R5-F4.16 bash assignment (control)",
        "sh",
        "x=1\ng() {\n  echo \"$x\"\n}\n",
        &[("x", 1), ("g", 2)],
        &["x"],
        &[(3, 10, "x", USAGE)],
    ),
    (
        "R5-F4.16 zsh assignment (control)",
        "zsh",
        "x=1\ng() {\n  echo \"$x\"\n}\n",
        &[("x", 1), ("g", 2)],
        &["x"],
        &[(3, 10, "x", USAGE)],
    ),
];

crate::language_cases::language_tests!(parameter_captures_declare_only_their_binding_name => check_parameter_captures_declare_only_their_binding_name);

fn check_parameter_captures_declare_only_their_binding_name(language: ProgrammingLanguage) {
    for &(label, extension, source, records, names, reads) in PARAMETER_CAPTURE_MATRIX.iter().filter(|row| ProgrammingLanguage::from_extension(row.1) == Some(language)) {
        let file = Analyzed::new(source, extension);
        let mut declared: Vec<(&str, u32)> = file
            .index
            .symbols
            .iter()
            .map(|record| {
                let line = record.source.name_span.as_ref().map_or(0, |span| span.start.line);
                (record.identity.name.as_str(), line)
            })
            .collect();
        declared.sort_by_key(|&(name, line)| (line, name));
        let mut expected_records = records.to_vec();
        expected_records.sort_by_key(|&(name, line)| (line, name));
        assert_eq!(declared, expected_records, "{label}: records");

        let occurrences: Vec<(u32, u32, &str, OccurrenceKind)> = file
            .evidence
            .occurrences
            .iter()
            .filter(|occurrence| names.contains(&occurrence.target.name.as_str()))
            .map(|occurrence| {
                (occurrence.span.start.line, occurrence.span.start.column, occurrence.target.name.as_str(), occurrence.kind)
            })
            .collect();
        assert_eq!(occurrences, reads, "{label}: occurrences");
    }
}

// ---- Declaration-name exclusion -------------------------------------------------

/// `(label, extension, source, the declaration tokens as (line, column,
/// name), every occurrence of the checked names as (line, column, name,
/// kind))`. The checked names are those of both lists; positions are one-based.
pub(crate) type DeclarationRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(u32, u32, &'static str)],
    &'static [(u32, u32, &'static str, OccurrenceKind)],
);

/// A token that declares a name is never an occurrence, whether or not the
/// declaration becomes a record, while the bounds, defaults, iterables,
/// initializers and later reads around it stay occurrences. Rows are review
/// rows `R5-F2.nn` (defective and control) and their sweep siblings
/// `R5-F2.Sn`.
pub(crate) const DECLARATION_EXCLUSION_MATRIX: &[DeclarationRow] = &[
    (
        "R5-F2.01 java class, interface and method type parameters",
        "java",
        "class Bound {}\nclass G<T> { T f; <U extends Bound> U m(U u) { return u; } }\ninterface I<T> {}\n",
        &[(2, 9, "T"), (2, 20, "U"), (3, 13, "T")],
        &[(2, 14, "T", USAGE), (2, 30, "Bound", USAGE), (2, 37, "U", USAGE), (2, 41, "U", USAGE)],
    ),
    (
        "R5-F2.02 java compact constructor",
        "java",
        "record R(int x) { R { } static R make() { return null; } }\n",
        &[(1, 19, "R")],
        &[(1, 32, "R", USAGE)],
    ),
    (
        "R5-F2.03 java statement label",
        "java",
        "class L { void m() { label: while (true) { break label; } } }\n",
        &[(1, 22, "label")],
        &[(1, 50, "label", USAGE)],
    ),
    (
        "R5-F2.04 scala class and function type parameters",
        "scala",
        "class Bound\nclass G[T, +U, -V <: Bound](t: T, u: U) { def f[W >: Bound](w: W): W = w }\n",
        &[(2, 9, "T"), (2, 13, "U"), (2, 17, "V"), (2, 49, "W")],
        &[(2, 22, "Bound", USAGE), (2, 32, "T", USAGE), (2, 38, "U", USAGE), (2, 54, "Bound", USAGE), (2, 64, "W", USAGE), (2, 68, "W", USAGE)],
    ),
    (
        "R5-F2.05 scala scalar enumerator",
        "scala",
        "object O { def a(xs: List[Int]) = for (x <- xs) use(x) }\n",
        &[(1, 40, "x")],
        &[(1, 45, "xs", USAGE), (1, 53, "x", USAGE)],
    ),
    (
        "R5-F2.06 scala tuple enumerator (control)",
        "scala",
        "object O { def b(xs: List[(Int, Int)]) = for ((p, q) <- xs) use(p) }\n",
        &[(1, 48, "p"), (1, 51, "q")],
        &[(1, 57, "xs", USAGE), (1, 65, "p", USAGE)],
    ),
    (
        "R5-F2.07 scala match binders, stable identifiers read",
        "scala",
        "object O { def c(o: Option[Int]) = o match { case Some(y) => use(y); case z @ Some(w) => use(z, w); case Stable => 0 } }\n",
        &[(1, 56, "y"), (1, 75, "z"), (1, 84, "w")],
        &[(1, 36, "o", USAGE), (1, 51, "Some", USAGE), (1, 66, "y", USAGE), (1, 79, "Some", USAGE), (1, 94, "z", USAGE), (1, 97, "w", USAGE), (1, 106, "Stable", USAGE)],
    ),
    (
        "R5-F2.08 scala value patterns",
        "scala",
        "object O {\n  val (m, n) = xy\n  val Some(k) = opt\n  def u = m + n + k\n}\n",
        &[(2, 8, "m"), (2, 11, "n"), (3, 12, "k")],
        &[(2, 16, "xy", USAGE), (3, 7, "Some", USAGE), (3, 17, "opt", USAGE), (4, 11, "m", USAGE), (4, 15, "n", USAGE), (4, 19, "k", USAGE)],
    ),
    (
        "R5-F2.09 scala abstract type member",
        "scala",
        "trait Tr { type Element; def get: Element }\n",
        &[(1, 17, "Element")],
        &[(1, 35, "Element", USAGE)],
    ),
    (
        "R5-F2.10 go type and function type parameters",
        "go",
        "package p\ntype Bound interface{}\ntype G[T Bound] struct{ f T }\nfunc f[U Bound](u U) U { return u }\n",
        &[(3, 8, "T"), (4, 8, "U")],
        &[(3, 10, "Bound", USAGE), (3, 27, "T", USAGE), (4, 10, "Bound", USAGE), (4, 19, "U", USAGE), (4, 22, "U", USAGE)],
    ),
    (
        "R5-F2.11 python PEP 695 type parameters",
        "py",
        "class Bound: pass\nclass G[T: Bound, *Ts, **P]:\n    f: T\ndef gen[U: (int, str)](u: U) -> U:\n    return u\n",
        &[(2, 9, "T"), (2, 20, "Ts"), (2, 26, "P"), (4, 9, "U")],
        &[(2, 12, "Bound", USAGE), (3, 8, "T", USAGE), (4, 13, "int", USAGE), (4, 18, "str", USAGE), (4, 27, "U", USAGE), (4, 33, "U", USAGE)],
    ),
    (
        "R5-F2.12 python type alias name",
        "py",
        "type Alias[V] = list[V]\n",
        &[(1, 6, "Alias"), (1, 12, "V")],
        &[(1, 17, "list", USAGE), (1, 22, "V", USAGE)],
    ),
    (
        "R5-F2.13 python lambda parameters",
        "py",
        "def g():\n    cb = lambda x: x\n    cb2 = lambda *ra, **kw: (ra, kw)\n",
        &[(2, 17, "x"), (3, 19, "ra"), (3, 25, "kw")],
        &[(2, 20, "x", USAGE), (3, 30, "ra", USAGE), (3, 34, "kw", USAGE)],
    ),
    (
        "R5-F2.14 python annotated variadics",
        "py",
        "class Bound: pass\ndef h(*args: Bound, **kwargs: Bound):\n    return args, kwargs\n",
        &[(2, 8, "args"), (2, 23, "kwargs")],
        &[(2, 14, "Bound", USAGE), (2, 31, "Bound", USAGE), (3, 12, "args", USAGE), (3, 18, "kwargs", USAGE)],
    ),
    (
        "R5-F2.15 python structural pattern captures",
        "py",
        "def m(subject):\n    match subject:\n        case [a1, *rest1]:\n            use(a1, rest1)\n        case {\"k\": v1, **others}:\n            use(v1, others)\n        case Point(px, y=py):\n            use(px, py)\n        case (1 | 2) as alias1:\n            use(alias1)\n",
        &[(3, 15, "a1"), (3, 20, "rest1"), (5, 20, "v1"), (5, 26, "others"), (7, 20, "px"), (7, 26, "py"), (9, 25, "alias1")],
        &[(2, 11, "subject", USAGE), (4, 17, "a1", USAGE), (4, 21, "rest1", USAGE), (6, 17, "v1", USAGE), (6, 21, "others", USAGE), (7, 14, "Point", USAGE), (8, 17, "px", USAGE), (8, 21, "py", USAGE), (10, 17, "alias1", USAGE)],
    ),
    (
        "R5-F2.16 python starred assignment",
        "py",
        "def s(xy):\n    a, *b = xy\n    use(a, b)\n",
        &[(2, 5, "a"), (2, 9, "b")],
        &[(2, 13, "xy", USAGE), (3, 9, "a", USAGE), (3, 12, "b", USAGE)],
    ),
    (
        "R5-F2.17 c++ defaulted, pack and template-template parameters",
        "cpp",
        "struct Bound {};\ntemplate <typename U = Bound, typename... Ts, template <typename> class TT = Box> struct S { U u; TT<Ts...> v; };\n",
        &[(2, 20, "U"), (2, 43, "Ts"), (2, 73, "TT")],
        &[(2, 24, "Bound", USAGE), (2, 78, "Box", USAGE), (2, 94, "U", USAGE), (2, 99, "TT", USAGE), (2, 102, "Ts", USAGE)],
    ),
    (
        "R5-F2.18 c++ plain, non-type and nested template parameters (control)",
        "cpp",
        "constexpr int K = 3;\ntemplate <typename T, int N = K, template <class> class TT2> struct S2 { T t; int a[N]; };\n",
        &[(2, 20, "T"), (2, 27, "N"), (2, 57, "TT2")],
        &[(2, 31, "K", USAGE), (2, 74, "T", USAGE), (2, 85, "N", USAGE)],
    ),
    (
        "R5-F2.19 c++ range declarations",
        "cpp",
        "void f(Vec xs) {\n  for (auto x : xs) { use(x); }\n  for (auto [a, b] : xs) { use(a, b); }\n}\n",
        &[(2, 13, "x"), (3, 14, "a"), (3, 17, "b")],
        &[(2, 17, "xs", USAGE), (2, 27, "x", USAGE), (3, 22, "xs", USAGE), (3, 32, "a", USAGE), (3, 35, "b", USAGE)],
    ),
    (
        "R5-F2.20 c++ structured declaration",
        "cpp",
        "void f(Pair xy) {\n  auto [c, d] = xy;\n  use(c, d);\n}\n",
        &[(2, 9, "c"), (2, 12, "d")],
        &[(2, 17, "xy", USAGE), (3, 7, "c", USAGE), (3, 10, "d", USAGE)],
    ),
    (
        "R5-F2.21 c++ namespace alias and inline destructor",
        "cpp",
        "namespace Other {}\nnamespace Alias = Other;\nclass C { ~C() {} };\n",
        &[(2, 11, "Alias"), (3, 12, "C")],
        &[(2, 19, "Other", USAGE)],
    ),
    (
        "R5-F2.22 c# catch and pattern declarations",
        "cs",
        "class C {\n  void M(object x) {\n    try {} catch (Exception e) { Use(e); }\n    if (x is C p) Use(p);\n    if (x is var v) Use(v);\n    switch (x) { case C q: Use(q); break; }\n  }\n}\n",
        &[(3, 29, "e"), (4, 16, "p"), (5, 18, "v"), (6, 25, "q")],
        &[(3, 19, "Exception", USAGE), (3, 38, "e", USAGE), (4, 9, "x", USAGE), (4, 14, "C", USAGE), (4, 23, "p", USAGE), (5, 9, "x", USAGE), (5, 25, "v", USAGE), (6, 13, "x", USAGE), (6, 23, "C", USAGE), (6, 32, "q", USAGE)],
    ),
    (
        "R5-F2.23 c# lambda, foreach, deconstruction and generic names (control)",
        "cs",
        "class G<T> where T : class {\n  void M(int[] xs, (int, int) tup) {\n    System.Func<int, int> f = z => z;\n    foreach (var it in xs) Use(it);\n    foreach (var (k1, k2) in ps) Use(k1);\n    var (t1, t2) = tup;\n    Use(t1);\n  }\n}\n",
        &[(1, 9, "T"), (3, 31, "z"), (4, 18, "it"), (5, 19, "k1"), (5, 23, "k2"), (6, 10, "t1"), (6, 14, "t2")],
        &[(1, 18, "T", USAGE), (3, 36, "z", USAGE), (4, 24, "xs", USAGE), (4, 32, "it", USAGE), (5, 38, "k1", USAGE), (6, 20, "tup", USAGE), (7, 9, "t1", USAGE)],
    ),
    (
        "R5-F2.24 c# delegate, destructor and label names",
        "cs",
        "delegate int D(W p);\nclass C {\n  ~C() {}\n  void M() { lbl: goto lbl; }\n}\n",
        &[(1, 14, "D"), (3, 4, "C"), (4, 14, "lbl")],
        &[(1, 16, "W", USAGE), (4, 24, "lbl", USAGE)],
    ),
    (
        "R5-F2.25 swift closure parameters",
        "swift",
        "let cb = { (x: Int) in x }\nlet cb2 = { y in y }\n",
        &[(1, 13, "x"), (2, 13, "y")],
        &[(1, 16, "Int", USAGE), (1, 24, "x", USAGE), (2, 18, "y", USAGE)],
    ),
    (
        "R5-F2.26 swift external labels and `_`",
        "swift",
        "func g(label local: Int, _ other: Int) -> Int { return local + other }\n",
        &[(1, 8, "label"), (1, 26, "_")],
        &[(1, 21, "Int", USAGE), (1, 35, "Int", USAGE), (1, 43, "Int", USAGE), (1, 56, "local", USAGE), (1, 64, "other", USAGE)],
    ),
    (
        "R5-F2.27 swift parameter pack",
        "swift",
        "struct Pack<each T> { func f(_ v: repeat each T) {} }\n",
        &[(1, 18, "T")],
        &[(1, 47, "T", USAGE)],
    ),
    (
        "R5-F2.28 swift generic, loop, catch, label and associatedtype names (control)",
        "swift",
        "struct G<T: Bound> { var u: T }\nfunc h(o: Int?, cs: [Int]) {\n  for c in cs { use(c) }\n  do { try f() } catch let err { use(err) }\n  outer: for c2 in cs { break outer }\n}\nprotocol P { associatedtype Item; init(v: Int) }\n",
        &[(1, 10, "T"), (3, 7, "c"), (4, 28, "err"), (5, 3, "outer"), (7, 29, "Item")],
        &[(1, 13, "Bound", USAGE), (1, 29, "T", USAGE), (3, 12, "cs", USAGE), (3, 21, "c", USAGE), (4, 38, "err", USAGE), (5, 20, "cs", USAGE), (5, 31, "outer", USAGE)],
    ),
    (
        "R5-F2.29 php catch variable",
        "php",
        "<?php\nfunction f() {\n  try { } catch (Exception $e) { useit($e); }\n}\n",
        &[(3, 29, "e")],
        &[(3, 18, "Exception", USAGE), (3, 41, "e", USAGE)],
    ),
    (
        "R5-F2.30 php destructuring targets, keys read",
        "php",
        "<?php\nfunction f($xs) {\n  [$a, $b] = $xs;\n  list($c, $d) = $xs;\n  [$k2 => $v2] = $xs;\n  useit($a, $b, $c, $d, $v2);\n}\n",
        &[(3, 5, "a"), (3, 9, "b"), (4, 9, "c"), (4, 13, "d"), (5, 12, "v2")],
        &[(3, 15, "xs", USAGE), (4, 19, "xs", USAGE), (5, 5, "k2", USAGE), (5, 19, "xs", USAGE), (6, 10, "a", USAGE), (6, 14, "b", USAGE), (6, 18, "c", USAGE), (6, 22, "d", USAGE), (6, 26, "v2", USAGE)],
    ),
    (
        "R5-F2.31 perl declared loop iterators; an undeclared one reads",
        "pl",
        "my @xs = (1);\nfor my $x (@xs) { print $x; }\nforeach our $y (@xs) { print $y; }\nfor $z (@xs) { print $z; }\n",
        &[(2, 8, "$x"), (3, 13, "$y")],
        &[(2, 12, "@xs", USAGE), (2, 25, "$x", USAGE), (3, 17, "@xs", USAGE), (3, 30, "$y", USAGE), (4, 5, "$z", USAGE), (4, 9, "@xs", USAGE), (4, 22, "$z", USAGE)],
    ),
    (
        "R5-F2.32 lua literal named field; computed key and member read",
        "lua",
        "local external = 1\nlocal M = { field = external, [key] = 2 }\nuse(M.field)\n",
        &[(2, 13, "field")],
        &[(2, 21, "external", USAGE), (2, 32, "key", USAGE), (3, 5, "M", USAGE), (3, 7, "field", USAGE)],
    ),
    (
        "R5-F2.33 rust parameters, patterns and generics; constant pattern read (control)",
        "rs",
        "const K: u32 = 1;\nfn f<T: Copy>(x: T, (a, b): (u32, u32)) -> T { match a { K => x, n => { let _ = (n, b); x } } }\n",
        &[(2, 6, "T"), (2, 15, "x"), (2, 22, "a"), (2, 25, "b"), (2, 66, "n")],
        &[(2, 18, "T", USAGE), (2, 44, "T", USAGE), (2, 54, "a", USAGE), (2, 58, "K", USAGE), (2, 63, "x", USAGE), (2, 82, "n", USAGE), (2, 85, "b", USAGE), (2, 89, "x", USAGE)],
    ),
    (
        "R5-F2.34 typescript parameters, defaults, patterns and generics (control)",
        "ts",
        "const d = 1;\nfunction g<T>(x: T, y = d, { p }: { p: number }): T { return x ?? p ?? y; }\n",
        &[(2, 12, "T"), (2, 15, "x"), (2, 21, "y"), (2, 30, "p"), (2, 37, "p")],
        &[(2, 18, "T", USAGE), (2, 25, "d", USAGE), (2, 51, "T", USAGE), (2, 62, "x", USAGE), (2, 67, "p", USAGE), (2, 72, "y", USAGE)],
    ),
    (
        "R5-F2.35 javascript parameters, defaults and patterns (control)",
        "js",
        "const d = 1;\nfunction g(x, y = d, { p }, [q]) { return [x, y, p, q]; }\n",
        &[(2, 12, "x"), (2, 15, "y"), (2, 24, "p"), (2, 30, "q")],
        &[(2, 19, "d", USAGE), (2, 44, "x", USAGE), (2, 47, "y", USAGE), (2, 50, "p", USAGE), (2, 53, "q", USAGE)],
    ),
    (
        "R5-F2.36 c parameters, loop variables and locals (control)",
        "c",
        "int f(int a) {\n  for (int i = 0; i < a; i++) { use(i); }\n  int b = a;\n  return b;\n}\n",
        &[(1, 11, "a"), (2, 12, "i"), (3, 7, "b")],
        &[(2, 19, "i", USAGE), (2, 23, "a", USAGE), (2, 26, "i", USAGE), (2, 37, "i", USAGE), (3, 11, "a", USAGE), (4, 10, "b", USAGE)],
    ),
    (
        "R5-F2.37 bash loop variable and assignment (control)",
        "sh",
        "for x in a b; do echo \"$x\"; done\ny=1\necho \"$y\"\n",
        &[(1, 5, "x"), (2, 1, "y")],
        &[(1, 25, "x", USAGE), (3, 8, "y", USAGE)],
    ),
    (
        "R5-F2.38 zsh loop variable and assignment (control)",
        "zsh",
        "for x in a b; do echo \"$x\"; done\ny=1\necho \"$y\"\n",
        &[(1, 5, "x"), (2, 1, "y")],
        &[(1, 25, "x", USAGE), (3, 8, "y", USAGE)],
    ),
    (
        "R5-F2.S1 java pattern binders: switch type pattern, record pattern components, instanceof record pattern",
        "java",
        "class S {\n  record P(int a, int b) {}\n  void m(Object o) {\n    switch (o) { case P(int x, int y) -> use(x); case String s -> use(s); default -> {} }\n    if (o instanceof P(var q, int w)) use(q, w);\n  }\n}\n",
        &[(4, 29, "x"), (4, 36, "y"), (4, 62, "s"), (5, 28, "q"), (5, 35, "w")],
        &[(4, 13, "o", USAGE), (4, 23, "P", USAGE), (4, 46, "x", USAGE), (4, 55, "String", USAGE), (4, 71, "s", USAGE), (5, 9, "o", USAGE), (5, 22, "P", USAGE), (5, 43, "q", USAGE), (5, 46, "w", USAGE)],
    ),
    (
        "R5-F2.S2 java try resource, annotation element and generic constructor parameter",
        "java",
        "class S {\n  <T> S(T t) { }\n  void m() { try (Res r = open()) { use(r); } }\n  @interface Ann { int val() default 0; }\n}\n",
        &[(2, 4, "T"), (3, 23, "r"), (4, 24, "val")],
        &[(2, 9, "T", USAGE), (3, 19, "Res", USAGE), (3, 41, "r", USAGE)],
    ),
    (
        "R5-F2.S3 java varargs, lambda, catch and enhanced-for binders (control)",
        "java",
        "class C { void m(int... r) { Runnable f = (a) -> use(a); try { } catch (Exception e) { use(e); } for (int k : r) { use(k); } } }\n",
        &[(1, 25, "r"), (1, 44, "a"), (1, 83, "e"), (1, 107, "k")],
        &[(1, 30, "Runnable", USAGE), (1, 54, "a", USAGE), (1, 73, "Exception", USAGE), (1, 92, "e", USAGE), (1, 111, "r", USAGE), (1, 120, "k", USAGE)],
    ),
    (
        "R5-F2.S4 scala given name, higher-kinded and polymorphic-function type parameters",
        "scala",
        "object S {\n  given g: Ord = ord\n  def c[F[_], G[X] <: Seq[X]](f: F[Int]): G[Int] = ???\n  val h: [T] => T => T = [U] => (t: U) => t\n}\n",
        &[(2, 9, "g"), (3, 9, "F"), (3, 15, "G"), (3, 17, "X"), (4, 11, "T"), (4, 27, "U")],
        &[(2, 12, "Ord", USAGE), (2, 18, "ord", USAGE), (3, 23, "Seq", USAGE), (3, 27, "X", USAGE), (3, 34, "F", USAGE), (3, 43, "G", USAGE), (4, 17, "T", USAGE), (4, 22, "T", USAGE), (4, 37, "U", USAGE)],
    ),
    (
        "R5-F2.S5 scala braced generators, value enumerators and case-lambda binders",
        "scala",
        "object S {\n  def d(xs: List[Int]) = for { x2 <- xs; y2 = x2 + 1 } yield y2\n  val fn = { case (i, j) => i + j }: ((Int, Int)) => Int\n  def e(x: Any) = x match { case n: Int => n; case Seq(h, t @ _*) => (h, t) }\n}\n",
        &[(2, 32, "x2"), (2, 42, "y2"), (3, 20, "i"), (3, 23, "j"), (4, 34, "n"), (4, 56, "h"), (4, 59, "t")],
        &[(2, 38, "xs", USAGE), (2, 47, "x2", USAGE), (2, 62, "y2", USAGE), (3, 29, "i", USAGE), (3, 33, "j", USAGE), (4, 44, "n", USAGE), (4, 52, "Seq", USAGE), (4, 71, "h", USAGE), (4, 74, "t", USAGE)],
    ),
    (
        "R5-F2.S6 go type switch alias and select receive; `=` receive assigns",
        "go",
        "package p\nfunc h(x any, ch chan int, m int) {\n	switch t := x.(type) { case int: use(t) }\n	select { case r := <-ch: use(r) }\n	select { case m = <-ch: use(m) }\n}\n",
        &[(3, 9, "t"), (4, 16, "r")],
        &[(3, 14, "x", USAGE), (3, 39, "t", USAGE), (4, 23, "ch", USAGE), (4, 31, "r", USAGE), (5, 16, "m", USAGE), (5, 22, "ch", USAGE), (5, 30, "m", USAGE)],
    ),
    (
        "R5-F2.S7 go range, receiver, named results, function literal and labels (control)",
        "go",
        "package p\ntype R struct{ f int }\nfunc (r *R) M(a []int) (res int) {\n	for k, v := range a { use(k, v) }\n	fn := func(q int) int { return q }\nL:\n	for { break L }\n	return fn(r.f)\n}\n",
        &[(3, 7, "r"), (3, 15, "a"), (3, 25, "res"), (4, 6, "k"), (4, 9, "v"), (5, 2, "fn"), (5, 13, "q")],
        &[(3, 10, "R", USAGE), (4, 20, "a", USAGE), (4, 28, "k", USAGE), (4, 31, "v", USAGE), (5, 33, "q", USAGE), (8, 9, "fn", CALL), (8, 12, "r", USAGE)],
    ),
    (
        "R5-F2.S8 python starred for target, list pattern, with tuple target and except alias",
        "py",
        "def s(xy, f):\n    [c, *d] = xy\n    for i, *j in xy: use(i, j)\n    with open(f) as (w1, w2): use(w1, w2)\n    try: pass\n    except E as err: use(c, d, err)\n",
        &[(2, 6, "c"), (2, 10, "d"), (3, 9, "i"), (3, 13, "j"), (4, 22, "w1"), (4, 26, "w2"), (6, 17, "err")],
        &[(2, 15, "xy", USAGE), (3, 18, "xy", USAGE), (3, 26, "i", USAGE), (3, 29, "j", USAGE), (4, 15, "f", USAGE), (4, 35, "w1", USAGE), (4, 39, "w2", USAGE), (6, 12, "E", USAGE), (6, 26, "c", USAGE), (6, 29, "d", USAGE), (6, 32, "err", USAGE)],
    ),
    (
        "R5-F2.S9 python del, global and augmented assignment stay occurrences",
        "py",
        "def g(xy):\n    global gg\n    xy += 1\n    del xy\n",
        &[],
        &[(2, 12, "gg", USAGE), (3, 5, "xy", USAGE), (4, 9, "xy", USAGE)],
    ),
    (
        "R5-F2.S10 c++ lambda init captures, declared and out-of-class destructors, if-init binding",
        "cpp",
        "struct C { ~C(); };\nC::~C() {}\nvoid f(Vec xs) {\n  auto l = [&r = xs, s = 1]() { return r + s; };\n  if (auto [c, d] = g(); c) use(d);\n}\n",
        &[(1, 13, "C"), (2, 5, "C"), (4, 14, "r"), (4, 22, "s"), (5, 13, "c"), (5, 16, "d")],
        &[(2, 1, "C", USAGE), (4, 18, "xs", USAGE), (4, 40, "r", USAGE), (4, 44, "s", USAGE), (5, 26, "c", USAGE), (5, 33, "d", USAGE)],
    ),
    (
        "R5-F2.S11 c++ type parameter pack; reference range-for (control)",
        "cpp",
        "template <typename... Us> void g(Us... us) {\n  for (const auto& x : xs) use(x);\n}\n",
        &[(1, 23, "Us"), (1, 40, "us"), (2, 20, "x")],
        &[(1, 34, "Us", USAGE), (2, 24, "xs", USAGE), (2, 32, "x", USAGE)],
    ),
    (
        "R5-F2.S12 c# recursive pattern designation, query clauses and tuple element names",
        "cs",
        "class S {\n  void M(object x, int[] xs) {\n    if (x is S { } named) Use(named);\n    var q = from i in xs let j = i * 2 join k in xs on i equals k into g select j;\n    (int f1, int f2) tup = (1, 2);\n  }\n}\n",
        &[(3, 20, "named"), (4, 18, "i"), (4, 30, "j"), (4, 45, "k"), (4, 72, "g"), (5, 10, "f1"), (5, 18, "f2")],
        &[(3, 9, "x", USAGE), (3, 14, "S", USAGE), (3, 31, "named", USAGE), (4, 23, "xs", USAGE), (4, 34, "i", USAGE), (4, 50, "xs", USAGE), (4, 56, "i", USAGE), (4, 65, "k", USAGE), (4, 81, "j", USAGE)],
    ),
    (
        "R5-F2.S13 c# positional pattern declarations, out var, using var and local function (control-adjacent)",
        "cs",
        "class S {\n  void M(object x) {\n    if (x is (int p1, int p2)) Use(p1, p2);\n    M2(out var ov); Use(ov);\n    using var res = Make(); Use(res);\n    int Local(int lp) => lp;\n  }\n}\n",
        &[(3, 19, "p1"), (3, 27, "p2"), (4, 16, "ov"), (5, 15, "res"), (6, 19, "lp")],
        &[(3, 9, "x", USAGE), (3, 36, "p1", USAGE), (3, 40, "p2", USAGE), (4, 25, "ov", USAGE), (5, 33, "res", USAGE), (6, 26, "lp", USAGE)],
    ),
    (
        "R5-F2.S14 swift optional binding, case patterns, tuple let and tuple for",
        "swift",
        "func h(o: Int?, pairs: [(Int, Int)]) {\n  if let a = o { use(a) }\n  guard let b = o else { return }\n  while let w = next() { use(w) }\n  if case let .some(e) = o { use(e) }\n  switch o { case let .some(c): use(c); case .some(z): use(z); default: break }\n  let (t1, t2) = tup\n  for (i, j) in pairs { use(i, j, b, t1, t2) }\n}\n",
        &[(2, 10, "a"), (3, 13, "b"), (4, 13, "w"), (5, 21, "e"), (6, 29, "c"), (7, 8, "t1"), (7, 12, "t2"), (8, 8, "i"), (8, 11, "j")],
        &[(2, 14, "o", USAGE), (2, 22, "a", USAGE), (3, 17, "o", USAGE), (4, 30, "w", USAGE), (5, 26, "o", USAGE), (5, 34, "e", USAGE), (6, 10, "o", USAGE), (6, 37, "c", USAGE), (6, 52, "z", USAGE), (6, 60, "z", USAGE), (7, 18, "tup", USAGE), (8, 17, "pairs", USAGE), (8, 29, "i", USAGE), (8, 32, "j", USAGE), (8, 35, "b", USAGE), (8, 38, "t1", USAGE), (8, 42, "t2", USAGE)],
    ),
    (
        "R5-F2.S15 swift setter and observer names, capture-list initializer, enum and subscript labels",
        "swift",
        "struct S {\n  var p: Int { get { 0 } set(nv) { use(nv) } }\n  var q: Int = 0 { willSet(wv) { use(wv) } didSet { use(oldValue) } }\n  func f(o: Int) { let cap = { [weak self, k = o] in k } }\n  subscript(index i: Int) -> Int { i }\n}\nenum E { case one(label: Int) }\n",
        &[(2, 30, "nv"), (3, 28, "wv"), (4, 44, "k"), (5, 13, "index"), (7, 19, "label")],
        &[(2, 40, "nv", USAGE), (3, 38, "wv", USAGE), (3, 57, "oldValue", USAGE), (4, 38, "self", USAGE), (4, 48, "o", USAGE), (4, 54, "k", USAGE), (5, 36, "i", USAGE)],
    ),
    (
        "R5-F2.S16 php static variables, foreach list targets, by-reference target; member and subscript targets read",
        "php",
        "<?php\nfunction f(array $xs, $o) {\n  static $s1, $s2 = 2;\n  foreach ($xs as $k => [$p, $q]) { useit($k, $p, $q); }\n  [&$r] = $xs;\n  [$o->prop, $arr[0]] = $xs;\n  useit($s1, $s2, $r);\n}\n",
        &[(3, 11, "s1"), (3, 16, "s2"), (4, 27, "p"), (4, 31, "q"), (5, 6, "r")],
        &[(4, 13, "xs", USAGE), (4, 44, "k", USAGE), (4, 48, "p", USAGE), (4, 52, "q", USAGE), (5, 12, "xs", USAGE), (6, 5, "o", USAGE), (6, 8, "prop", USAGE), (6, 15, "arr", USAGE), (6, 26, "xs", USAGE), (7, 10, "s1", USAGE), (7, 15, "s2", USAGE), (7, 20, "r", USAGE)],
    ),
    (
        "R5-F2.S17 perl signature parameters",
        "pl",
        "sub f ($x, $y) { return $x + $y; }\n",
        &[(1, 8, "$x"), (1, 12, "$y")],
        &[(1, 25, "$x", USAGE), (1, 30, "$y", USAGE)],
    ),
    (
        "R5-F2.S18 lua label and nested literal fields; goto and member reads",
        "lua",
        "local t = { a = 1, b = { c = 2 } }\n::lbl:: goto lbl\nuse(t.b.c)\n",
        &[(1, 13, "a"), (1, 20, "b"), (1, 26, "c"), (2, 3, "lbl")],
        &[(2, 14, "lbl", USAGE), (3, 5, "t", USAGE), (3, 7, "b", USAGE), (3, 9, "c", USAGE)],
    ),
    (
        "R5-F2.S19 lua generic for, parameters and implicit self (control)",
        "lua",
        "local t = {}\nfunction t:method(x) return self, x end\nfor k, v in pairs(t) do use(k, v) end\n",
        &[(2, 19, "x"), (3, 5, "k"), (3, 8, "v")],
        &[(2, 10, "t", USAGE), (2, 29, "self", USAGE), (2, 35, "x", USAGE), (3, 19, "t", USAGE), (3, 29, "k", USAGE), (3, 32, "v", USAGE)],
    ),

    (
        "R5-F2.S20 rust wildcard assignment target is no occurrence",
        "rs",
        "fn h() -> i32 { 1 }\nfn g() { let _ = h(); _ = h(); }\n",
        &[(2, 14, "_"), (2, 23, "_")],
        &[(2, 18, "h", CALL), (2, 27, "h", CALL)],
    ),
    (
        "R5-F2.S20 go blank identifier is no occurrence",
        "go",
        "package p\nfunc h() int { return 1 }\nfunc g() { _ = h() }\n",
        &[(3, 12, "_")],
        &[(3, 16, "h", CALL)],
    ),
    (
        "R5-F2.S20 swift wildcard assignment target is no occurrence",
        "swift",
        "func h() -> Int { 1 }\nfunc g() { _ = h() }\n",
        &[(2, 12, "_")],
        &[(2, 16, "h", CALL)],
    ),
    (
        "R5-F2.S20 scala wildcard binder and placeholder are no occurrences",
        "scala",
        "object O { def h() = 1\n  def g(xs: List[Int]) = { val _ = h(); xs.map(_ + 1) } }\n",
        &[(2, 32, "_"), (2, 48, "_")],
        &[(2, 36, "h", CALL)],
    ),
    (
        "R5-F2.S20 control: c# local named _ is read",
        "cs",
        "class G { void g() { var _ = 1; use(_); } }\n",
        &[(1, 26, "_")],
        &[(1, 37, "_", USAGE)],
    ),
    (
        "R5-F2.S20 control: python name _ is read",
        "py",
        "_ = 1\nprint(_)\n",
        &[(1, 1, "_")],
        &[(2, 7, "_", USAGE)],
    ),
    (
        "R7-F2.01 java nested annotation type; the element default and the annotation read",
        "java",
        "class A {}\nclass G { @interface A { int value() default external; } @A void g() {} }\n",
        &[(1, 7, "A"), (2, 22, "A"), (2, 30, "value")],
        &[(2, 46, "external", USAGE), (2, 59, "A", USAGE)],
    ),
    (
        "R7-F2.01 java top-level annotation type",
        "java",
        "@interface A { int value(); }\nclass U { @A int f; }\n",
        &[(1, 12, "A"), (1, 20, "value")],
        &[(2, 12, "A", USAGE)],
    ),
    (
        "R7-F2.02a java module",
        "java",
        "module m {}\n",
        &[(1, 8, "m")],
        &[],
    ),
    (
        "R7-F2.02a java open module; a directive operand reads",
        "java",
        "open module n { requires x; }\n",
        &[(1, 13, "n")],
        &[(1, 26, "x", USAGE)],
    ),
    (
        "R7-F2.02b java qualified module; directive operands read",
        "java",
        "module a.b { requires c.d; exports a.b; }\n",
        &[(1, 8, "a"), (1, 10, "b")],
        &[(1, 23, "c", USAGE), (1, 25, "d", USAGE), (1, 36, "a", USAGE), (1, 38, "b", USAGE)],
    ),
    (
        "R7-F2.03 java qualified package; later type reads",
        "java",
        "package a.b; class G { a x; b y; }\n",
        &[(1, 9, "a"), (1, 11, "b")],
        &[(1, 24, "a", USAGE), (1, 29, "b", USAGE)],
    ),
    (
        "R7-F2.03 java simple package (control)",
        "java",
        "package p; class H { p z; }\n",
        &[(1, 9, "p")],
        &[(1, 22, "p", USAGE)],
    ),
    (
        "R7-F2.04 c# static constructor beside an instance constructor; the field type reads",
        "cs",
        "class G { static G() {} G() {} G x; }\n",
        &[(1, 7, "G"), (1, 18, "G"), (1, 25, "G")],
        &[(1, 32, "G", USAGE)],
    ),
    (
        "R7-F2.05a c# accessor event beside a field-like event; types and the call read",
        "cs",
        "using System;\nclass G { event Action E {add{} remove{}} void f() { E(); } event Action F; }\n",
        &[(2, 24, "E"), (2, 74, "F")],
        &[(2, 17, "Action", USAGE), (2, 54, "E", CALL), (2, 67, "Action", USAGE)],
    ),
    (
        "R7-F2.05b c# explicit-interface event; the interface and types read",
        "cs",
        "interface IFace { event System.Action E; }\nclass G : IFace { event System.Action IFace.E {add{} remove{}} }\n",
        &[(1, 11, "IFace"), (1, 39, "E"), (2, 45, "E")],
        &[(1, 25, "System", USAGE), (1, 32, "Action", USAGE), (2, 11, "IFace", USAGE), (2, 25, "System", USAGE), (2, 32, "Action", USAGE), (2, 39, "IFace", USAGE)],
    ),
    (
        "R7-F2.06 c# extern aliases; qualified uses read",
        "cs",
        "extern alias A; extern alias B; class G { A::T t; B::U u; }\n",
        &[(1, 14, "A"), (1, 30, "B")],
        &[(1, 43, "A", USAGE), (1, 46, "T", USAGE), (1, 51, "B", USAGE), (1, 54, "U", USAGE)],
    ),
    (
        "R7-F2.07 c# qualified block namespace; later type reads",
        "cs",
        "namespace A.B { class G { A x; B y; } }\n",
        &[(1, 11, "A"), (1, 13, "B")],
        &[(1, 27, "A", USAGE), (1, 32, "B", USAGE)],
    ),
    (
        "R7-F2.08 c# qualified file-scoped namespace; later type reads",
        "cs",
        "namespace A.B;\nclass G { A x; B y; }\n",
        &[(1, 11, "A"), (1, 13, "B")],
        &[(2, 11, "A", USAGE), (2, 16, "B", USAGE)],
    ),
    (
        "R7-F2.08 c# simple file-scoped namespace (control)",
        "cs",
        "namespace C;\nclass G { C x; }\n",
        &[(1, 11, "C")],
        &[(2, 11, "C", USAGE)],
    ),
    (
        "R7-F2.09a c++ nested namespaces at two and three levels; qualified calls read",
        "cpp",
        "namespace A::B { void f(); } namespace C::D::E {} struct A { static void m(); }; void g() { A::m(); }\n",
        &[(1, 11, "A"), (1, 14, "B"), (1, 40, "C"), (1, 43, "D"), (1, 46, "E"), (1, 58, "A")],
        &[(1, 93, "A", USAGE), (1, 96, "m", CALL)],
    ),
    (
        "R7-F2.09b c++ nested inline namespace",
        "cpp",
        "namespace A::inline B { int x; }\n",
        &[(1, 11, "A"), (1, 21, "B")],
        &[],
    ),
    (
        "R7-F2.10a scala qualified package; later type reads",
        "scala",
        "package a.b\n\nclass G {\n  val x: a = null; val y: b = null\n}\n",
        &[(1, 9, "a"), (1, 11, "b")],
        &[(4, 10, "a", USAGE), (4, 27, "b", USAGE)],
    ),
    (
        "R7-F2.10b scala braced package; a later type read",
        "scala",
        "package a.b {\n  class G { val x: a = null }\n}\n",
        &[(1, 9, "a"), (1, 11, "b")],
        &[(2, 20, "a", USAGE)],
    ),
    (
        "R7-F2.10c scala stacked packages",
        "scala",
        "package a\npackage b\nclass G\n",
        &[(1, 9, "a"), (2, 9, "b")],
        &[],
    ),
    (
        "R7-F2.11a perl qualified package; same-named sub calls read",
        "pl",
        "package A::B; sub A {} sub B {} A(); B();\n",
        &[(1, 9, "A"), (1, 12, "B"), (1, 19, "A"), (1, 28, "B")],
        &[(1, 33, "A", CALL), (1, 38, "B", CALL)],
    ),
    (
        "R7-F2.11b perl versioned block package beside a simple package",
        "pl",
        "package A::B v1.0 { sub f {} }\npackage C;\n",
        &[(1, 9, "A"), (1, 12, "B"), (2, 9, "C")],
        &[],
    ),
    (
        "R7-F2.12a typescript dotted namespace; the type read remains",
        "ts",
        "class A {} namespace A.B { export function f(){} } let x:A;\n",
        &[(1, 7, "A"), (1, 22, "A")],
        &[(1, 58, "A", USAGE)],
    ),
    (
        "R7-F2.12a typescript three-segment namespace and legacy dotted module",
        "ts",
        "class A {} namespace A.B.C { } module A.D {} let x:A;\n",
        &[(1, 22, "A"), (1, 39, "A")],
        &[(1, 52, "A", USAGE)],
    ),
    (
        "R7-F2.12a typescript simple namespace (control)",
        "ts",
        "namespace N { export const k = 1; } let y = N.k;\n",
        &[(1, 11, "N")],
        &[(1, 45, "N", USAGE)],
    ),
    (
        "R7-F2.12b typescript dotted namespace inside a function",
        "ts",
        "class A {}; function g(){ namespace A.B{} }\n",
        &[(1, 7, "A"), (1, 37, "A")],
        &[],
    ),
    (
        "R7-F2.13 swift macro beside a same-named function; the call reads",
        "swift",
        "func foo() {}\nmacro foo() = #externalMacro(module: \"M\", type: \"T\")\nfoo()\n",
        &[(1, 6, "foo"), (2, 7, "foo")],
        &[(3, 1, "foo", CALL)],
    ),
    (
        "R7-F2.14 swift precedence groups; operator group and higherThan read",
        "swift",
        "infix operator +++: P\nprecedencegroup P { higherThan: Q }\nprecedencegroup Q {}\n",
        &[(2, 17, "P"), (3, 17, "Q")],
        &[(1, 21, "P", USAGE), (2, 33, "Q", USAGE)],
    ),
    (
        "R7-F2.15.typealias swift selective import binds its last name; the module and later reads read",
        "swift",
        "import typealias Foo.Bar\nlet z = Bar\n",
        &[(1, 22, "Bar")],
        &[(1, 18, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.15.class swift selective import binds its last name; the module and later reads read",
        "swift",
        "import class Foo.Bar\nlet z = Bar\n",
        &[(1, 18, "Bar")],
        &[(1, 14, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.15.enum swift selective import binds its last name; the module and later reads read",
        "swift",
        "import enum Foo.Bar\nlet z = Bar\n",
        &[(1, 17, "Bar")],
        &[(1, 13, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.15.protocol swift selective import binds its last name; the module and later reads read",
        "swift",
        "import protocol Foo.Bar\nlet z = Bar\n",
        &[(1, 21, "Bar")],
        &[(1, 17, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.15.struct swift selective import binds its last name; the module and later reads read",
        "swift",
        "import struct Foo.Bar\nlet z = Bar\n",
        &[(1, 19, "Bar")],
        &[(1, 15, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.15.func swift selective import binds its last name; the module and later reads read",
        "swift",
        "import func Foo.Bar\nlet z = Bar\n",
        &[(1, 17, "Bar")],
        &[(1, 13, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.15.var swift selective import binds its last name; the module and later reads read",
        "swift",
        "import var Foo.Bar\nlet z = Bar\n",
        &[(1, 16, "Bar")],
        &[(1, 12, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.15.let swift selective import binds its last name; the module and later reads read",
        "swift",
        "import let Foo.Bar\nlet z = Bar\n",
        &[(1, 16, "Bar")],
        &[(1, 12, "Foo", USAGE), (2, 9, "Bar", USAGE)],
    ),
    (
        "R7-F2.S1 swift whole-module import paths; a later module read",
        "swift",
        "import Foo\nimport Foo.Sub\nlet z = Foo\n",
        &[(1, 8, "Foo"), (2, 8, "Foo"), (2, 12, "Sub")],
        &[(3, 9, "Foo", USAGE)],
    ),
    (
        "R7-F2.S2 php qualified namespace; later type reads",
        "php",
        "<?php\nnamespace A\\B;\nclass C { function f(A $a, B $b) {} }\n",
        &[(2, 11, "A"), (2, 13, "B")],
        &[(3, 22, "A", USAGE), (3, 28, "B", USAGE)],
    ),
    (
        "R7-F2.S3 php promoted and variadic parameters; the body read remains",
        "php",
        "<?php\nclass K { function __construct(private int $pp, public &$rp) {} function v(int ...$vs) { return $vs; } }\n",
        &[(2, 45, "pp"), (2, 58, "rp"), (2, 84, "vs")],
        &[(2, 98, "vs", USAGE)],
    ),
    (
        "R7-F2.S4 scala package object; member read",
        "scala",
        "package object po { val pv = 1 }\nobject U { def u = po.pv }\n",
        &[(1, 16, "po"), (1, 25, "pv")],
        &[(2, 20, "po", USAGE), (2, 23, "pv", USAGE)],
    ),
    (
        "R7-F2.S5 scala import renames bind their alias; the imported name reads",
        "scala",
        "import a.{b => bb, c as cc}\nobject U { def u = bb + cc }\n",
        &[(1, 16, "bb"), (1, 25, "cc")],
        &[(1, 11, "b", USAGE), (1, 20, "c", USAGE), (2, 20, "bb", USAGE), (2, 25, "cc", USAGE)],
    ),
    (
        "R7-F2.S6 scala type lambda, named context bound and named tuple elements",
        "scala",
        "object O {\n  type L = [X] =>> List[X]\n  def f[T: Ordering as ord](t: T) = ord\n  type NT = (nx: Int, ny: Int)\n}\n",
        &[(2, 13, "X"), (3, 24, "ord"), (4, 14, "nx"), (4, 23, "ny")],
        &[(2, 25, "X", USAGE), (3, 37, "ord", USAGE)],
    ),
    (
        "R7-F2.S7 python future import names; the aliased feature and later uses read",
        "py",
        "from __future__ import annotations, division as dv\nprint(annotations, division, dv)\n",
        &[(1, 24, "annotations"), (1, 49, "dv")],
        &[(1, 37, "division", USAGE), (2, 7, "annotations", USAGE), (2, 20, "division", USAGE), (2, 30, "dv", USAGE)],
    ),
    (
        "R7-F2.S8 c++ forward, opaque and qualified type declarations; qualifiers and a specialization read",
        "cpp",
        "struct A { struct B; enum class E; union U; };\nstruct A::B { int x; };\nenum class A::E { e1 };\nunion A::U { int u; };\ntemplate <typename T> struct S {};\ntemplate <> struct S<int> {};\n",
        &[(1, 8, "A"), (1, 19, "B"), (1, 33, "E"), (1, 42, "U"), (2, 11, "B"), (3, 15, "E"), (4, 10, "U"), (5, 30, "S")],
        &[(2, 8, "A", USAGE), (3, 12, "A", USAGE), (4, 7, "A", USAGE), (6, 20, "S", USAGE)],
    ),
    (
        "R7-F2.S8 control: c forward and tag declarations",
        "c",
        "struct S;\nunion U2 { int u; };\nenum E3 { e3 };\n",
        &[(1, 8, "S"), (2, 7, "U2"), (3, 6, "E3")],
        &[],
    ),
];

crate::language_cases::language_tests!(declaration_tokens_are_never_occurrences_and_reads_remain => check_declaration_tokens_are_never_occurrences_and_reads_remain);

fn check_declaration_tokens_are_never_occurrences_and_reads_remain(language: ProgrammingLanguage) {
    let rows: Vec<_> = DECLARATION_EXCLUSION_MATRIX.iter().copied().filter(|row| ProgrammingLanguage::from_extension(row.1) == Some(language)).collect();
    assert_exclusion_rows(&rows);
}

/// `R7-F2.12a`/`.12b`: a TypeScript dotted namespace declares its prefix
/// `A`, so the prefix is excluded before binding: it never becomes a
/// `Scoped` occurrence of the class `A`, and a namespace inside `g` gives
/// `g` no `references` edge to `A` (nor `A` a `referenced_by` edge to `g`).
/// The genuine type read stays `Scoped` to the class.
#[test]
fn typescript_namespace_prefixes_never_bind_to_a_same_named_class() {
    let file = Analyzed::new("class A {} namespace A.B { export function f(){} } let x:A;\n", "ts");
    let class = file.named("A", 1).clone();
    let scoped: Vec<(u32, u32)> = file
        .all("A")
        .iter()
        .map(|occurrence| {
            assert_scoped_to(occurrence, &class);
            (occurrence.span.start.line, occurrence.span.start.column)
        })
        .collect();
    assert_eq!(scoped, [(1, 58)]);

    let nested = Analyzed::new("class A {}; function g(){ namespace A.B{} }\n", "ts");
    assert!(nested.all("A").is_empty(), "{:#?}", nested.evidence.occurrences);
    let class = nested.record("A", 1);
    let g = nested.record("g", 1);
    assert!(class.relations.referenced_by.is_empty(), "{:#?}", class.relations);
    assert!(g.relations.references.is_empty(), "{:#?}", g.relations);
}

/// A contextual syntax marker reads no binding, so it is never an
/// occurrence (and never a suggestion for a same-named declaration), while
/// the same spelling in a name role stays one. Rows are review rows
/// `R6-F3.nn` and their sweep siblings `R6-F3.Sn`; the excluded list holds
/// the markers and the declaration tokens beside them.
const MARKER_EXCLUSION_MATRIX: &[DeclarationRow] = &[
    (
        "R6-F3.01 lua const attribute; the initializer and later read remain",
        "lua",
        "local const=1\nlocal x <const> = const\nprint(const)\n",
        &[(1, 7, "const"), (2, 10, "const")],
        &[(2, 19, "const", USAGE), (3, 7, "const", USAGE)],
    ),
    (
        "R6-F3.02 lua close attribute; the initializer and later read remain",
        "lua",
        "local close=2\nlocal y <close> = close\nprint(close)\n",
        &[(1, 7, "close"), (2, 10, "close")],
        &[(2, 19, "close", USAGE), (3, 7, "close", USAGE)],
    ),
    (
        "R6-F3.03 control: lua const/close reads, table keys and members",
        "lua",
        "local const, close = 1, 2\nlocal t = { const = const, [close] = close }\nprint(t.const, t[close], const(close))\n",
        &[(1, 7, "const"), (1, 14, "close"), (2, 13, "const")],
        &[
            (2, 21, "const", USAGE),
            (2, 29, "close", USAGE),
            (2, 38, "close", USAGE),
            (3, 9, "const", USAGE),
            (3, 18, "close", USAGE),
            (3, 26, "const", CALL),
            (3, 32, "close", USAGE),
        ],
    ),
    (
        "R6-F3.04 java local inferred type beside a variable named var",
        "java",
        "record R() {\n    void g() { var x=factory(); int var=1; use(var); }\n}\n",
        &[(2, 16, "var"), (2, 37, "var")],
        &[(2, 22, "factory", CALL), (2, 48, "var", USAGE)],
    ),
    (
        "R6-F3.05 java inferred-type slots: local, final local, for, resource, lambda, record pattern",
        "java",
        "record P(int a) {}\nclass S {\n    void g(java.util.List<String> xs, Object o, int var) {\n        var v = id(xs);\n        final var w = id(v);\n        for (var y : xs) { use(y); }\n        try (var r = open(xs)) { use(r); }\n        java.util.function.Function<String, String> l = (var z) -> z;\n        if (o instanceof P(var a)) { use(a); }\n        for (var i = 0; i < var; i++) {}\n    }\n}\n",
        &[
            (3, 53, "var"),
            (4, 9, "var"),
            (5, 15, "var"),
            (6, 14, "var"),
            (7, 14, "var"),
            (8, 58, "var"),
            (9, 28, "var"),
            (10, 14, "var"),
        ],
        &[
            (4, 20, "xs", USAGE),
            (5, 26, "v", USAGE),
            (6, 22, "xs", USAGE),
            (6, 32, "y", USAGE),
            (7, 27, "xs", USAGE),
            (7, 38, "r", USAGE),
            (8, 68, "z", USAGE),
            (9, 42, "a", USAGE),
            (10, 29, "var", USAGE),
        ],
    ),
    (
        "R6-F3.06 control: java var outside inference slots, qualified, and as a variable stays read",
        "java",
        "class var {}\nclass L {\n    var f;\n    var m(var p) { String s = null; a.var q = null; return p; }\n    void n(int var) { use(var); }\n}\n",
        &[(1, 7, "var"), (5, 16, "var")],
        &[
            (3, 5, "var", USAGE),
            (4, 5, "var", USAGE),
            (4, 11, "var", USAGE),
            (4, 20, "String", USAGE),
            (4, 39, "var", USAGE),
            (5, 27, "var", USAGE),
        ],
    ),
    (
        "R6-F3.06 legacy: a file declaring a type var predates inference, so var in a type slot names it",
        "java",
        "class var {}\nclass R {\n    void g() { var x = make(); }\n    var make() { return null; }\n}\n",
        &[(1, 7, "var")],
        &[(3, 16, "var", USAGE), (4, 5, "var", USAGE)],
    ),
    (
        "R6-F3.06 legacy: a single-type import of var predates inference",
        "java",
        "import a.b.var;\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(1, 12, "var")],
        &[(3, 16, "var", USAGE)],
    ),
    (
        "R6-F3.06 legacy: a type parameter named var predates inference",
        "java",
        "class R<var> {\n    void g() { var x = null; }\n}\n",
        &[(1, 9, "var")],
        &[(2, 16, "var", USAGE)],
    ),
    (
        "R6-F3.06 control: an on-demand import names no type var, so var stays the marker",
        "java",
        "import a.b.*;\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(3, 16, "var")],
        &[],
    ),
    (
        "R7-F1.01-.07 java static field import of var is no type evidence: every inference slot stays a marker",
        "java",
        "import static p.Tools.var;\nrecord P(int a) {}\nclass S {\n    void g(java.util.List<String> xs, Object o, int var) {\n        var v = id(xs);\n        final var w = id(v);\n        for (var y : xs) { use(y); }\n        try (var r = open(xs)) { use(r); }\n        java.util.function.Function<String, String> l = (var z) -> z;\n        if (o instanceof P(var a)) { use(a); }\n        for (var i = 0; i < var; i++) {}\n    }\n}\n",
        &[
            (1, 23, "var"),
            (4, 53, "var"),
            (5, 9, "var"),
            (6, 15, "var"),
            (7, 14, "var"),
            (8, 14, "var"),
            (9, 58, "var"),
            (10, 28, "var"),
            (11, 14, "var"),
        ],
        &[
            (5, 17, "id", CALL),
            (5, 20, "xs", USAGE),
            (6, 23, "id", CALL),
            (6, 26, "v", USAGE),
            (7, 22, "xs", USAGE),
            (7, 32, "y", USAGE),
            (8, 22, "open", CALL),
            (8, 27, "xs", USAGE),
            (8, 38, "r", USAGE),
            (9, 68, "z", USAGE),
            (10, 13, "o", USAGE),
            (10, 26, "P", USAGE),
            (10, 42, "a", USAGE),
            (11, 29, "var", USAGE),
        ],
    ),
    (
        "R7-F1.08a java static field import beside a record and a variable named var",
        "java",
        "import static p.Tools.var;\nrecord G() {\n    void g() { var x = 1; int var = 2; use(var); }\n}\n",
        &[(1, 23, "var"), (3, 16, "var"), (3, 31, "var")],
        &[(3, 44, "var", USAGE)],
    ),
    (
        "R7-F1.08b java static method import: the marker is excluded, the call of var stays",
        "java",
        "import static p.Methods.var;\nrecord G() {\n    void g() { var x = var(); }\n}\n",
        &[(1, 25, "var"), (3, 16, "var")],
        &[(3, 24, "var", CALL)],
    ),
    (
        "R7-F1.09 control: a static on-demand import names no type var",
        "java",
        "import static p.Tools.*;\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(3, 16, "var")],
        &[(3, 24, "make", CALL)],
    ),
    (
        "R7-F1.09 control: a static import of another name",
        "java",
        "import static p.Tools.other;\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(3, 16, "var")],
        &[(3, 24, "make", CALL)],
    ),
    (
        "R7-F1.09 control: var as an ordinary import's qualifier names no type var; the qualifier stays",
        "java",
        "import p.var.T;\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(3, 16, "var")],
        &[(1, 10, "var", USAGE), (3, 24, "make", CALL)],
    ),
    (
        "R7-F1.09 control: var as a static import's qualifier names no type var; the qualifier stays",
        "java",
        "import static p.var.Tools.other;\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(3, 16, "var")],
        &[(1, 17, "var", USAGE), (3, 24, "make", CALL)],
    ),
    (
        "R7-F1.10 legacy: an interface named var predates inference",
        "java",
        "interface var {}\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(1, 11, "var")],
        &[(3, 16, "var", USAGE), (3, 24, "make", CALL)],
    ),
    (
        "R7-F1.10 legacy: an enum named var predates inference",
        "java",
        "enum var {}\nclass R {\n    void g() { var x = make(); }\n}\n",
        &[(1, 6, "var")],
        &[(3, 16, "var", USAGE), (3, 24, "make", CALL)],
    ),
    (
        "R7-F1.11a-d, .12a-c, .13 swift precedence directives and the associativity value; group references stay",
        "swift",
        SWIFT_PRECEDENCE_GROUPS,
        &[
            (2, 5, "associativity"),
            (2, 20, "left"),
            (3, 5, "assignment"),
            (4, 5, "higherThan"),
            (5, 5, "lowerThan"),
            (8, 5, "associativity"),
            (8, 20, "right"),
            (11, 5, "associativity"),
            (11, 20, "none"),
            (15, 9, "left"),
            (15, 19, "right"),
            (15, 30, "none"),
            (16, 9, "associativity"),
            (16, 28, "assignment"),
            (16, 44, "higherThan"),
            (16, 60, "lowerThan"),
        ],
        &[
            (4, 17, "AdditionPrecedence", USAGE),
            (5, 16, "MultiplicationPrecedence", USAGE),
            (13, 22, "TernaryPrecedence", USAGE),
            (17, 9, "left", USAGE),
            (17, 15, "right", USAGE),
            (17, 22, "none", USAGE),
            (17, 28, "associativity", USAGE),
            (17, 43, "assignment", USAGE),
            (17, 55, "higherThan", USAGE),
            (17, 67, "lowerThan", USAGE),
        ],
    ),
    (
        "R7-F1.S1-.S4 swift legacy operator body, availability platforms, @available arguments and #if operands; a wrapper argument stays",
        "swift",
        "infix operator +++ { associativity left precedence 140 }\n@available(iOS 13, *)\n@available(*, deprecated, message: \"m\")\nfunc f() {\n    if #available(iOS 13, macOS 10.15, *) {}\n}\n#if os(macOS) && DEBUG || canImport(UIKit)\nlet x = 1\n#endif\nstruct K { @W(message) var q: Int }\nfunc g() {\n    let left = 0, precedence = 1, iOS = 2, macOS = 3, DEBUG = 4, UIKit = 5, deprecated = 6, message = 7\n    use(left, precedence, iOS, macOS, DEBUG, UIKit, deprecated, message)\n}\n",
        &[
            (1, 22, "associativity"),
            (1, 36, "left"),
            (1, 41, "precedence"),
            (2, 12, "iOS"),
            (3, 15, "deprecated"),
            (3, 27, "message"),
            (5, 19, "iOS"),
            (5, 27, "macOS"),
            (7, 8, "macOS"),
            (7, 18, "DEBUG"),
            (7, 37, "UIKit"),
            (12, 9, "left"),
            (12, 19, "precedence"),
            (12, 35, "iOS"),
            (12, 44, "macOS"),
            (12, 55, "DEBUG"),
            (12, 66, "UIKit"),
            (12, 77, "deprecated"),
            (12, 93, "message"),
        ],
        &[
            (10, 15, "message", USAGE),
            (13, 9, "left", USAGE),
            (13, 15, "precedence", USAGE),
            (13, 27, "iOS", USAGE),
            (13, 32, "macOS", USAGE),
            (13, 39, "DEBUG", USAGE),
            (13, 46, "UIKit", USAGE),
            (13, 53, "deprecated", USAGE),
            (13, 65, "message", USAGE),
        ],
    ),
    (
        "R7-F1.S5.py keyword argument labels; the value after the label stays a read",
        "py",
        "def f(x): pass\nx = 1\nf(x=x)\ndict(x=1)\nclass K(metaclass=x): pass\n",
        &[(1, 7, "x"), (2, 1, "x"), (3, 3, "x"), (4, 6, "x"), (5, 9, "metaclass")],
        &[(3, 5, "x", USAGE), (5, 19, "x", USAGE)],
    ),
    (
        "R7-F1.S5.cs named argument and attribute labels; member assignments stay occurrences",
        "cs",
        "class A : System.Attribute { public int X; public A(int x) {} }\n[A(x: 1, X = 2)]\nclass C {\n    int X;\n    void f(int x) { int y = 1; f(x: y); f(y); var c = new C { X = 1 }; }\n}\n",
        &[(1, 41, "X"), (1, 57, "x"), (2, 4, "x"), (4, 9, "X"), (5, 16, "x"), (5, 25, "y"), (5, 34, "x")],
        &[(2, 10, "X", USAGE), (5, 37, "y", USAGE), (5, 43, "y", USAGE), (5, 63, "X", USAGE)],
    ),
    (
        "R7-F1.S5.php named argument labels; the value after the label stays a call",
        "php",
        "<?php\nfunction f($x) {}\nfunction x() {}\nf(x: x());\n",
        &[(3, 10, "x"), (4, 3, "x")],
        &[(4, 6, "x", CALL)],
    ),
    (
        "R7-F1.S5.swift call, function-reference, tuple, tuple-type and tuple-pattern labels; values, members and dictionary keys stay",
        "swift",
        "func f(x: Int) -> Int { x }\nfunc g(u: [Int: Int]) {\n    let x = 1, y = 2\n    _ = f(x: x)\n    let r = f(x:)\n    let t: (x: Int, y: Int) = (x: x, y: y)\n    _ = t.x\n    if case (x: let a, y: _) = t { use(a) }\n    switch t { case (x: 1, y: let b): use(b) default: break }\n    let (x: e, y: _) = t\n    _ = [x: y]\n}\n",
        &[(1, 8, "x"), (3, 9, "x"), (3, 16, "y"), (4, 11, "x"), (5, 15, "x"), (6, 13, "x"), (6, 21, "y"), (6, 32, "x"), (6, 38, "y"), (8, 14, "x"), (8, 24, "y"), (9, 22, "x"), (9, 28, "y"), (10, 10, "x"), (10, 16, "y")],
        &[(1, 25, "x", USAGE), (4, 14, "x", USAGE), (6, 35, "x", USAGE), (6, 41, "y", USAGE), (7, 11, "x", USAGE), (11, 10, "x", USAGE), (11, 13, "y", USAGE)],
    ),
    (
        "R7-F1.S5.scala named arguments; a parenthesized assignment and the values stay",
        "scala",
        "class C(x: Int)\nobject O {\n  var x = 1\n  def f(x: Int) = x\n  f(x = x)\n  new C(x = x)\n  f((x = 2))\n  f(x = x + 1)\n}\n",
        &[(1, 9, "x"), (3, 7, "x"), (4, 9, "x"), (5, 5, "x"), (6, 9, "x"), (8, 5, "x")],
        &[(4, 19, "x", USAGE), (5, 9, "x", USAGE), (6, 13, "x", USAGE), (7, 6, "x", USAGE), (8, 9, "x", USAGE)],
    ),
    (
        "R7-F1.S5.perl fat-comma words are quoted; values, hash subscripts and calls stay",
        "pl",
        "my $x = 1;\nmy %h = (x => $x);\nprint $h{x};\nmy $r = {x => 1};\nf(x => x());\n",
        &[(1, 4, "$x"), (2, 10, "x"), (3, 10, "x"), (4, 10, "x"), (5, 3, "x")],
        &[(2, 15, "$x", USAGE), (5, 8, "x", CALL)],
    ),
    (
        "R7-F1.S5.go a struct-literal field key is a label; a map key and a named-type key stay reads",
        "go",
        "package p\ntype T struct{ X int }\nvar X = 1\nvar m = map[int]int{X: 1}\nvar s = struct{ X int }{X: X}\nvar t = T{X: X}\n",
        &[(2, 16, "X"), (3, 5, "X"), (5, 17, "X"), (5, 25, "X")],
        &[(4, 21, "X", USAGE), (5, 28, "X", USAGE), (6, 11, "X", USAGE), (6, 14, "X", USAGE)],
    ),
    (
        "R7-F1.S5 control: c and cpp designators are never captured; the value stays",
        "c",
        "struct S { int x; };\nint x = 1;\nstruct S s = { .x = x };\n",
        &[(1, 16, "x"), (2, 5, "x"), (3, 17, "x")],
        &[(3, 21, "x", USAGE)],
    ),
    (
        "R7-F1.S5 control: cpp designators are never captured; the value stays",
        "cpp",
        "struct S { int x; };\nint x = 1;\nS s = { .x = x };\n",
        &[(1, 16, "x"), (2, 5, "x"), (3, 10, "x")],
        &[(3, 14, "x", USAGE)],
    ),
    (
        "R7-F1.S5 control: lua table keys are never captured; the value stays",
        "lua",
        "local x = 1\nlocal t = { x = x }\nprint(t.x)\n",
        &[(1, 7, "x"), (2, 13, "x")],
        &[(2, 17, "x", USAGE), (3, 9, "x", USAGE)],
    ),
    (
        "R7-F1.S6 swift built-in attribute names and keyword arguments; wrapper, global actor, Sendable, SPI group and derivative target stay",
        "swift",
        "@objc(run:with:) func h(a: Int, with b: Int) {}\n@inline(never) @_spi(Internal) @inlinable @discardableResult func i() -> Int { 1 }\nfunc k(cb: @escaping @convention(c) () -> Void) {}\n@propertyWrapper struct W { var wrappedValue: Int }\n@MainActor @Sendable func m() {}\nstruct S { @W var q: Int }\n@derivative(of: k) func d() {}\nfunc g() {\n    let objc = 1, run = 2, never = 3, inlinable = 4, discardableResult = 5, escaping = 6, c = 7\n    let propertyWrapper = 8, of = 9, MainActor = 10, Sendable = 11, Internal = 12, _spi = 13\n    use(objc, run, never, inlinable, discardableResult, escaping, c, propertyWrapper, of, MainActor, Sendable, Internal, _spi)\n}\n",
        &[(1, 2, "objc"), (1, 7, "run"), (2, 9, "never"), (2, 17, "_spi"), (2, 33, "inlinable"), (2, 44, "discardableResult"), (3, 6, "k"), (3, 13, "escaping"), (3, 23, "convention"), (3, 34, "c"), (4, 2, "propertyWrapper"), (4, 25, "W"), (7, 13, "of"), (9, 9, "objc"), (9, 19, "run"), (9, 28, "never"), (9, 39, "inlinable"), (9, 54, "discardableResult"), (9, 77, "escaping"), (9, 91, "c"), (10, 9, "propertyWrapper"), (10, 30, "of"), (10, 38, "MainActor"), (10, 54, "Sendable"), (10, 69, "Internal"), (10, 84, "_spi")],
        &[(2, 22, "Internal", USAGE), (5, 2, "MainActor", USAGE), (5, 13, "Sendable", USAGE), (6, 13, "W", USAGE), (7, 17, "k", USAGE), (11, 9, "objc", USAGE), (11, 15, "run", USAGE), (11, 20, "never", USAGE), (11, 27, "inlinable", USAGE), (11, 38, "discardableResult", USAGE), (11, 57, "escaping", USAGE), (11, 67, "c", USAGE), (11, 70, "propertyWrapper", USAGE), (11, 87, "of", USAGE), (11, 91, "MainActor", USAGE), (11, 102, "Sendable", USAGE), (11, 112, "Internal", USAGE), (11, 122, "_spi", USAGE)],
    ),
    (
        "R7-F3.S2 c# global alias in a using directive and a type path; the path segments read",
        "cs",
        "using global::System.Text;\nclass G { global::X.Y z; }\n",
        &[(1, 7, "global"), (1, 22, "Text"), (2, 11, "global")],
        &[(1, 15, "System", USAGE), (2, 19, "X", USAGE), (2, 21, "Y", USAGE)],
    ),
    (
        "R7-F3.S2 control: an extern alias qualifier reads",
        "cs",
        "extern alias E;\nclass G { E::T t; }\n",
        &[(1, 14, "E")],
        &[(2, 11, "E", USAGE), (2, 14, "T", USAGE)],
    ),
    (
        "R6-F3.S1 php self/static/parent class operands beside a method named static",
        "php",
        "<?php\nclass C {\n    function static() {}\n    function f(self|null $a, ?static $b): ?parent {\n        if ($a instanceof self) {}\n        $o = new STATIC();\n        $this->static();\n        static::static();\n        return new parent;\n    }\n}\n",
        &[
            (3, 14, "static"),
            (4, 16, "self"),
            (4, 31, "static"),
            (4, 44, "parent"),
            (5, 27, "self"),
            (6, 18, "STATIC"),
            (9, 20, "parent"),
        ],
        &[(7, 16, "static", CALL), (8, 17, "static", CALL)],
    ),
    (
        "R6-F3.S2 bash declaration builtin options; an ordinary command's option stays",
        "sh",
        "declare -i -r n=1\nlocal -- x\nexport -n y\ngrep -i x\n",
        &[(1, 9, "-i"), (1, 12, "-r"), (2, 7, "--"), (3, 8, "-n")],
        &[(4, 6, "-i", USAGE)],
    ),
    (
        "R6-F3.S2 zsh declaration builtin options; an ordinary command's option stays",
        "zsh",
        "declare -i -r n=1\nlocal -- x\ntypeset +x y\ngrep -i x\n",
        &[(1, 9, "-i"), (1, 12, "-r"), (2, 7, "--"), (3, 9, "+x")],
        &[(4, 6, "-i", USAGE)],
    ),
    (
        "R6-F3.X1.18 java inferred-type local with a bare-identifier initializer keeps the read",
        "java",
        "class S {\n    Object factory;\n    void g() { var x = factory; final var y = factory; }\n}\n",
        &[(2, 12, "factory"), (3, 16, "var"), (3, 39, "var")],
        &[(3, 24, "factory", USAGE), (3, 47, "factory", USAGE)],
    ),
    (
        "R6-F3.X2.01 zsh declaration builtin operands declare; expansions and the call stay",
        "zsh",
        "x() { :; }\nf() {\n  local x; typeset +x x; export x; declare -f x; readonly x; integer x\n  local -r z=1 x; local -- -n\n  echo $x ${x}; x\n}\n",
        &[
            (1, 1, "x"),
            (3, 9, "x"),
            (3, 20, "+x"),
            (3, 23, "x"),
            (3, 33, "x"),
            (3, 47, "x"),
            (3, 59, "x"),
            (3, 70, "x"),
            (4, 16, "x"),
            (4, 28, "-n"),
        ],
        &[(5, 9, "x", USAGE), (5, 13, "x", USAGE), (5, 17, "x", CALL)],
    ),
    (
        "R6-F3.X2.02 bash declaration builtin operands declare (parity); expansions and the call stay",
        "sh",
        "x() { :; }\nf() {\n  local x; typeset +x x; export x; declare -f x; readonly x\n  local -r z=1 x; local -- -n\n  echo $x ${x}; x\n}\n",
        &[
            (1, 1, "x"),
            (3, 9, "x"),
            (3, 20, "+x"),
            (3, 23, "x"),
            (3, 33, "x"),
            (3, 47, "x"),
            (3, 59, "x"),
            (4, 16, "x"),
            (4, 28, "-n"),
        ],
        &[(5, 9, "x", USAGE), (5, 13, "x", USAGE), (5, 17, "x", CALL)],
    ),
    (
        "R6-F3.X2.03 zsh word-shaped function operand declares; an ordinary command's argument stays",
        "zsh",
        "my-func() { :; }\ndeclare -f my-func\nexport my-func\nwhich my-func\nmy-func\n",
        &[(1, 1, "my-func"), (2, 12, "my-func"), (3, 8, "my-func")],
        &[(4, 7, "my-func", USAGE), (5, 1, "my-func", CALL)],
    ),
    (
        "R6-F3.X2.04 bash word-shaped function operand declares (parity); an ordinary command's argument stays",
        "sh",
        "my-func() { :; }\ndeclare -f my-func\nexport my-func\nwhich my-func\nmy-func\n",
        &[(1, 1, "my-func"), (2, 12, "my-func"), (3, 8, "my-func")],
        &[(4, 7, "my-func", USAGE), (5, 1, "my-func", CALL)],
    ),
    (
        "R6-F3.X2.05 control: bash integer is an ordinary command, its argument stays read",
        "sh",
        "i=1\ninteger i\necho $i\n",
        &[(1, 1, "i")],
        &[(2, 9, "i", USAGE), (3, 7, "i", USAGE)],
    ),
    (
        "R7-F1.14.rs modifier roles exclude keywords and preserve genuine names",
        "rs",
        "pub struct S {}\npub fn f(x: &mut S) { use_value(x); }\n",
        &[(1, 1, "pub"), (2, 1, "pub"), (2, 14, "mut")],
        &[(2, 18, "S", USAGE), (2, 33, "x", USAGE)],
    ),
    (
        "R7-F1.14.ts modifier roles exclude keywords and preserve genuine names",
        "ts",
        "class C { private static async f(x: C) { return x; } }\n",
        &[(1, 11, "private"), (1, 19, "static"), (1, 26, "async")],
        &[(1, 37, "C", USAGE), (1, 49, "x", USAGE)],
    ),
    (
        "R7-F1.14.js modifier roles exclude keywords and preserve genuine names",
        "js",
        "class C { static async f(x) { await x; } }\n",
        &[(1, 11, "static"), (1, 18, "async"), (1, 31, "await")],
        &[(1, 37, "x", USAGE)],
    ),
    (
        "R7-F1.14.py modifier roles exclude keywords and preserve genuine names",
        "py",
        "async def f(x):\n    await use(x)\nmatch = 1\nprint(match)\n",
        &[(1, 1, "async"), (2, 5, "await")],
        &[(2, 15, "x", USAGE), (4, 7, "match", USAGE)],
    ),
    (
        "R7-F1.14.go modifier roles exclude keywords and preserve genuine names",
        "go",
        "package p\nfunc f(x any) { go g(x); defer g(x) }\n",
        &[(2, 17, "go"), (2, 26, "defer")],
        &[(2, 10, "any", USAGE), (2, 22, "x", USAGE), (2, 34, "x", USAGE)],
    ),
    (
        "R7-F1.14.c modifier roles exclude keywords and preserve genuine names",
        "c",
        "struct S { int x; };\nstatic struct S a;\nextern struct S b;\n",
        &[(2, 1, "static"), (3, 1, "extern")],
        &[(2, 15, "S", USAGE), (3, 15, "S", USAGE)],
    ),
    (
        "R7-F1.14.cpp modifier roles exclude keywords and preserve genuine names",
        "cpp",
        "struct S {};\nstatic S x;\nconstexpr int k=1;\nauto y=k;\n",
        &[(2, 1, "static"), (3, 1, "constexpr"), (4, 1, "auto")],
        &[(2, 8, "S", USAGE), (4, 8, "k", USAGE)],
    ),
    (
        "R7-F1.14.cs modifier roles exclude keywords and preserve genuine names",
        "cs",
        "class G { readonly dynamic x; async void f() { var y=x; await h(); } }\n",
        &[(1, 11, "readonly"), (1, 31, "async"), (1, 48, "var"), (1, 57, "await")],
        &[(1, 20, "dynamic", USAGE), (1, 54, "x", USAGE)],
    ),
    (
        "R7-F1.14.scala modifier roles exclude keywords and preserve genuine names",
        "scala",
        "object O { lazy val x = external; inline def f(using y: Int) = x }\n",
        &[(1, 12, "lazy"), (1, 35, "inline"), (1, 48, "using")],
        &[(1, 25, "external", USAGE), (1, 57, "Int", USAGE), (1, 64, "x", USAGE)],
    ),
    (
        "R7-F1.14.pl modifier roles exclude keywords and preserve genuine names",
        "pl",
        "my $x = external();\nour $y = $x;\nstate $z = $x;\nlocal $w = $x;\n",
        &[(1, 1, "my"), (2, 1, "our"), (3, 1, "state"), (4, 1, "local")],
        &[(1, 9, "external", CALL), (2, 10, "$x", USAGE), (3, 12, "$x", USAGE), (4, 12, "$x", USAGE)],
    ),
];

crate::language_cases::language_tests!(contextual_syntax_markers_are_never_occurrences_and_reads_remain => check_contextual_syntax_markers_are_never_occurrences_and_reads_remain);

fn check_contextual_syntax_markers_are_never_occurrences_and_reads_remain(language: ProgrammingLanguage) {
    let rows: Vec<_> = MARKER_EXCLUSION_MATRIX.iter().copied().filter(|row| ProgrammingLanguage::from_extension(row.1) == Some(language)).collect();
    assert_exclusion_rows(&rows);
}

/// Rows `R7-F1.11`–`.13`: three precedence groups (associativity `left`,
/// `right`, `none`), an operator's group, and same-spelled locals read in
/// `g`. The referenced groups are the standard library's, so no
/// precedence-group declaration name is checked here.
const SWIFT_PRECEDENCE_GROUPS: &str = "\
precedencegroup A {
    associativity: left
    assignment: true
    higherThan: AdditionPrecedence
    lowerThan: MultiplicationPrecedence
}
precedencegroup B {
    associativity: right
}
precedencegroup C {
    associativity: none
}
infix operator +++ : TernaryPrecedence
func g() {
    let left = 1, right = 2, none = 3
    let associativity = 4, assignment = 5, higherThan = 6, lowerThan = 7
    use(left, right, none, associativity, assignment, higherThan, lowerThan)
}
";

/// Marker rows whose files hold a token another finding owns, so they
/// check single positions rather than every occurrence of a name.
#[test]
fn marker_exclusions_keep_legacy_reads_group_references_and_anonymous_regions() {
    // R7-F1.10: an annotation type named `var` predates inference.
    let annotation = Analyzed::new("@interface var {}\nclass R {\n    void g() { var x = make(); }\n}\n", "java");
    let read = annotation.at(3, "var", 0);
    assert_eq!((read.span.start.column, read.kind), (16, USAGE));

    // R7-F1.13: names after `higherThan`/`lowerThan` and an operator's
    // group name are references to in-file precedence groups.
    let groups = Analyzed::new(
        "precedencegroup Q {}\nprecedencegroup R {}\nprecedencegroup P {\n    associativity: left\n    higherThan: Q\n    lowerThan: R\n}\ninfix operator +++ : P\n",
        "swift",
    );
    for (line, name, column) in [(5, "Q", 17), (6, "R", 16), (8, "P", 22)] {
        let reference = groups.at(line, name, 0);
        assert_eq!((reference.span.start.column, reference.kind), (column, USAGE), "{name}");
    }
    assert!(
        !groups.evidence.occurrences.iter().any(|occurrence| matches!(
            occurrence.target.name.as_str(),
            "associativity" | "left" | "higherThan" | "lowerThan"
        )),
        "{:#?}",
        groups.evidence.occurrences
    );

    // R7-F1.05 / .C3: the lambda keeps its anonymous region after its
    // `var` marker is excluded under a static import.
    let java = Analyzed::new(
        "import static p.Tools.var;\nclass S {\n    void g() {\n        java.util.function.Function<String, String> l = (var z) -> z;\n    }\n}\n",
        "java",
    );
    let body = java.at(4, "z", 0);
    assert_eq!(body.owner, None);
    assert_eq!((body.owner_span.start.line, body.owner_span.start.column), (4, 57));
    assert!(
        java.evidence.reference_coverage.iter().any(|region| region.span == body.owner_span && region.owner.is_none()),
        "{:#?}",
        java.evidence.reference_coverage
    );
    assert!(!java.evidence.occurrences.iter().any(|occurrence| occurrence.target.name == "var"));
}

/// A declaration's initializer, default or assigned value is a read even
/// when it is a bare identifier, while the declared name beside it stays
/// excluded. Rows are `R6-F3.X1.nn`: Java (`.01`, `.02`) was the defective
/// language (`variable_declarator`'s `value` field was read as a name);
/// the others pin every language's bare-identifier value positions.
const INITIALIZER_READ_MATRIX: &[DeclarationRow] = &[
    (
        "R6-F3.X1.01 java field, local, inferred local and for-init initializers",
        "java",
        "class A {\n    int xs = 1; int ys = xs;\n    void g() { int k = xs; var v = xs; for (int i = xs; i < 1; i++) {} }\n}\n",
        &[(2, 9, "xs"), (2, 21, "ys"), (3, 20, "k"), (3, 32, "v")],
        &[(2, 26, "xs", USAGE), (3, 24, "xs", USAGE), (3, 36, "xs", USAGE), (3, 53, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.02 java static field, annotation element values and enum constant arguments",
        "java",
        "@interface Ann { int value(); }\nclass B { static final int xs = 1, k = xs; @Ann(xs) void h() {} @Ann(value = xs) void j() {} }\nenum E { P(xs), Q(B.xs); E(int a) {} }\n",
        &[(2, 28, "xs"), (2, 36, "k")],
        &[(2, 40, "xs", USAGE), (2, 49, "xs", USAGE), (2, 78, "xs", USAGE), (3, 12, "xs", USAGE), (3, 21, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.03 csharp field, local, inferred local and for-init initializers",
        "cs",
        "class A {\n    static int xs = 1; int ys = xs;\n    void g() { int k = xs; var v = xs; for (int i = xs; i < 1; i++) {} }\n}\n",
        &[(2, 16, "xs"), (2, 28, "ys"), (3, 20, "k"), (3, 32, "v")],
        &[(2, 33, "xs", USAGE), (3, 24, "xs", USAGE), (3, 36, "xs", USAGE), (3, 53, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.04 scala val, var and parameter default",
        "scala",
        "object A {\n  val xs = 1\n  val k = xs\n  var v = xs\n  def g(a: Int = xs) = a\n}\n",
        &[(2, 7, "xs"), (3, 7, "k"), (4, 7, "v")],
        &[(3, 11, "xs", USAGE), (4, 11, "xs", USAGE), (5, 18, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.05 swift let, var and parameter default",
        "swift",
        "let xs = 1\nlet k = xs\nvar v = xs\nfunc g(a: Int = xs) -> Int { return a }\n",
        &[(1, 5, "xs"), (2, 5, "k"), (3, 5, "v")],
        &[(2, 9, "xs", USAGE), (3, 9, "xs", USAGE), (4, 17, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.06 go var, const and short variable declaration",
        "go",
        "package a\nvar xs = 1\nvar k = xs\nconst c = xs\nfunc g() { v := xs; var w = xs; _, _ = v, w }\n",
        &[(2, 5, "xs"), (3, 5, "k"), (4, 7, "c")],
        &[(3, 9, "xs", USAGE), (4, 11, "xs", USAGE), (5, 17, "xs", USAGE), (5, 29, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.07 php assigned value, parameter default, class constant and property initializers",
        "php",
        "<?php\nfunction g($xs, $d = XS) { $k = $xs; }\nclass C { const XS = 1; const K = XS; public $q = XS; }\n",
        &[(2, 13, "xs"), (3, 17, "XS"), (3, 31, "K")],
        &[(2, 22, "XS", USAGE), (2, 34, "xs", USAGE), (3, 35, "XS", USAGE), (3, 51, "XS", USAGE)],
    ),
    (
        "R6-F3.X1.08 c global and local initializers",
        "c",
        "int xs = 1;\nint k = xs;\nvoid g(void) { int v = xs; }\n",
        &[(1, 5, "xs"), (2, 5, "k"), (3, 20, "v")],
        &[(2, 9, "xs", USAGE), (3, 24, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.09 cpp global, local, auto and brace initializers",
        "cpp",
        "int xs = 1;\nint k = xs;\nvoid g() { int v = xs; auto w = xs; int z{xs}; }\n",
        &[(1, 5, "xs"), (2, 5, "k"), (3, 16, "v"), (3, 29, "w"), (3, 41, "z")],
        &[(2, 9, "xs", USAGE), (3, 20, "xs", USAGE), (3, 33, "xs", USAGE), (3, 43, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.10 python assignment and parameter default",
        "py",
        "xs = 1\nk = xs\ndef g(a=xs):\n    v = xs\n",
        &[(1, 1, "xs"), (2, 1, "k"), (4, 5, "v")],
        &[(2, 5, "xs", USAGE), (3, 9, "xs", USAGE), (4, 9, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.11 lua local initializers",
        "lua",
        "local xs = 1\nlocal k = xs\nfunction g() local v = xs end\n",
        &[(1, 7, "xs"), (2, 7, "k"), (3, 20, "v")],
        &[(2, 11, "xs", USAGE), (3, 24, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.12 perl my initializers",
        "pl",
        "my $xs = 1;\nmy $k = $xs;\nsub g { my $v = $xs; }\n",
        &[(1, 4, "$xs"), (2, 4, "$k"), (3, 12, "$v")],
        &[(2, 9, "$xs", USAGE), (3, 17, "$xs", USAGE)],
    ),
    (
        "R6-F3.X1.13 bash assignment and local initializers",
        "sh",
        "xs=1\nk=$xs\ng() { local v=$xs; }\n",
        &[(1, 1, "xs"), (2, 1, "k"), (3, 13, "v")],
        &[(2, 4, "xs", USAGE), (3, 16, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.14 zsh assignment and local initializers",
        "zsh",
        "xs=1\nk=$xs\ng() { local v=$xs; }\n",
        &[(1, 1, "xs"), (2, 1, "k"), (3, 13, "v")],
        &[(2, 4, "xs", USAGE), (3, 16, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.15 rust const, static and let initializers",
        "rs",
        "const XS: i32 = 1;\nstatic K: i32 = XS;\nfn g() { let v = XS; }\n",
        &[(1, 7, "XS"), (2, 8, "K"), (3, 14, "v")],
        &[(2, 17, "XS", USAGE), (3, 18, "XS", USAGE)],
    ),
    (
        "R6-F3.X1.16 typescript const, let, field and parameter default",
        "ts",
        "const xs = 1;\nlet k = xs;\nclass C { f = xs; }\nfunction g(a = xs) { return a; }\n",
        &[(1, 7, "xs"), (2, 5, "k")],
        &[(2, 9, "xs", USAGE), (3, 15, "xs", USAGE), (4, 16, "xs", USAGE)],
    ),
    (
        "R6-F3.X1.17 javascript const, let, field and parameter default",
        "js",
        "const xs = 1;\nlet k = xs;\nclass C { f = xs; }\nfunction g(a = xs) { return a; }\n",
        &[(1, 7, "xs"), (2, 5, "k")],
        &[(2, 9, "xs", USAGE), (3, 15, "xs", USAGE), (4, 16, "xs", USAGE)],
    ),
];

crate::language_cases::language_tests!(bare_identifier_initializers_are_reads_beside_their_declared_names => check_bare_identifier_initializers_are_reads_beside_their_declared_names);

fn check_bare_identifier_initializers_are_reads_beside_their_declared_names(language: ProgrammingLanguage) {
    let rows: Vec<_> = INITIALIZER_READ_MATRIX.iter().copied().filter(|row| ProgrammingLanguage::from_extension(row.1) == Some(language)).collect();
    assert_exclusion_rows(&rows);
}

/// Every row's excluded tokens are absent, the checked names' occurrences
/// are exactly `reads`, and a limited language stays conservative.
fn assert_exclusion_rows(rows: &[DeclarationRow]) {
    for &(label, extension, source, declarations, reads) in rows {
        let lines: Vec<&str> = source.split('\n').collect();
        let written = |line: u32, column: u32, name: &str| {
            let text = lines[line as usize - 1];
            let start = column as usize - 1;
            assert_eq!(text.get(start..start + name.len()), Some(name), "{label}: no `{name}` at {line}:{column}");
        };
        for &(line, column, name) in declarations {
            written(line, column, name);
        }
        for &(line, column, name, _) in reads {
            written(line, column, name);
        }

        let file = Analyzed::new(source, extension);
        let checked: HashSet<&str> = declarations
            .iter()
            .map(|&(_, _, name)| name)
            .chain(reads.iter().map(|&(_, _, name, _)| name))
            .collect();
        let occurrences: Vec<(u32, u32, &str, OccurrenceKind)> = file
            .evidence
            .occurrences
            .iter()
            .filter(|occurrence| checked.contains(occurrence.target.name.as_str()))
            .map(|occurrence| {
                (occurrence.span.start.line, occurrence.span.start.column, occurrence.target.name.as_str(), occurrence.kind)
            })
            .collect();
        for &(line, column, name) in declarations {
            assert!(
                !occurrences.iter().any(|&(l, c, n, _)| (l, c, n) == (line, column, name)),
                "{label}: the excluded `{name}` at {line}:{column} is an occurrence"
            );
        }
        assert_eq!(occurrences, reads, "{label}: occurrences");

        // A limited language stays conservative: no binding, whole-file
        // limited coverage, no edges, recursion unknown.
        if !matches!(extension, "rs" | "ts" | "js") {
            for occurrence in &file.evidence.occurrences {
                assert_uncertain(occurrence);
            }
            for coverage in &file.evidence.reference_coverage {
                assert_eq!(coverage.state, CoverageState::Limited, "{label}");
                assert!(coverage.reasons.contains(&LimitReason::UnsupportedLanguage), "{label}");
            }
            for record in &file.index.symbols {
                assert!(record.relations.references.is_empty(), "{label}: `{}` has edges", record.identity.name);
                assert!(record.relations.referenced_by.is_empty(), "{label}: `{}` has edges", record.identity.name);
                if record.semantics.is_recursive.is_some() {
                    assert_eq!(record.semantics.is_recursive, Some(TriState::Unknown), "{label}: `{}`", record.identity.name);
                }
            }
        }
    }
}


// ---- Invocation forms ----------------------------------------------------------

const CALL: OccurrenceKind = OccurrenceKind::Call;
const USAGE: OccurrenceKind = OccurrenceKind::Usage;

/// R6-F1: Swift force-unwrapped invocations (`.01`–`.03`, `.17`) with their
/// controls (`.06`) on the lines below them. Line 3 onward is `g`'s body,
/// line 20 onward `h`'s.
const SWIFT_FORCE_UNWRAP: &str = "\
class X { var cb: (() -> Void)? }
func g(cb: (() -> Void)?, x: X, y: X?, a: [(() -> Void)?], c: Bool, other: (() -> Void)?) {
    cb!()
    (cb!)()
    ((cb)!)()
    x.cb!()
    (x.cb!)()
    ((x.cb)!)()
    cb?()
    y!.f()
    _ = cb!
    use(cb!)
    (c ? cb : other)!()
    (a[0]!)()
    x.cb?()
    cb()
    x.cb!
    use(x.cb!)
}
func h(cb: (() -> Void)??) {
    (cb!)!()
    ((cb!)!)()
    (cb!)?()
}
";

/// R6-F1: C# null-forgiving invocations (`.04`, `.05`, `.18`) with their
/// controls (`.07`) on the lines below them.
const CSHARP_NULL_FORGIVING: &str = "\
class G {
    System.Action F;
    void g(System.Action cb, G obj, System.Action[] arr, bool c, System.Action other, int i) {
        cb!();
        (cb!)();
        obj.F!();
        (obj.F!)();
        ((obj.F)!)();
        F!();
        cb!!();
        (cb!)!();
        ((cb!)!)();
        obj?.F!();
        obj.F!!();
        var v = cb!;
        consume(cb!);
        (c ? cb : other)!();
        (arr[0]!)();
        obj!.F();
        i++; i--;
        cb(); F();
        consume(obj.F!);
    }
}
";

/// `(language, extension, source, [(line, name, nth on that line, kind)])`.
type InvocationRow = (
    ProgrammingLanguage,
    &'static str,
    &'static str,
    &'static [(u32, &'static str, usize, OccurrenceKind)],
);

/// Every invocation form each pinned grammar writes, direct and through
/// transparent wrappers (parentheses, TypeScript `as`/`satisfies`/`<T>x`/
/// `x!`, Swift and C# `x!`), with the negative controls on the same lines: an argument, a
/// receiver, a dereference, a sequence and a conditional are never the
/// callee. Binding confidence may stay limited; only `kind` is checked.
const INVOCATION_MATRIX: &[InvocationRow] = &[
    (ProgrammingLanguage::TypeScript, "ts", "function f() {}\nfunction g() { (<any /* n */> /* b */ f /* a */)(); }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Perl, "pl", "sub f {}\nf # before\n ();\ndefined # note\n &f;\nexists # note\n &f;\n", &[
        (2, "f", 0, CALL),
        (5, "f", 0, USAGE),
        (7, "f", 0, USAGE),
    ]),
    (ProgrammingLanguage::Lua, "lua", "function f() end\nfunction g()\n ( -- before\n f -- after\n )()\nend\n", &[
        (4, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Python, "py", "def f(): pass\ndef g():\n ( # before\n f # after\n )()\n", &[
        (4, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Zsh, "zsh", "f() { :; }\nf # note\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Bash, "sh", "f() { :; }\nf # note\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Php, "php", "<?php\nfunction g() { (/* n */ $f /* n */)(); }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::C, "c", "void f() {}\nvoid g() { (/* n */ f /* n */)(); }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Go, "go", "package p\nfunc g() { (f /* n */ [int])() }\n", &[
        (2, "f", 0, USAGE),
    ]),
    (ProgrammingLanguage::Scala, "scala", "def f[T](): Unit = {}\ndef g(): Unit = { f /* n */ [Int]() }\n", &[
        (2, "f", 0, CALL),
        (2, "Int", 0, USAGE),
    ]),
    (ProgrammingLanguage::Cpp, "cpp", "template <typename T> void f() {}\nvoid g() { f /* n */ <int>(); }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::CSharp, "cs", "class G { void f<T>() {}\nvoid g() { f /* n */ <int>(); } }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Java, "java", "class C<T> {}\nclass G { void g() { new C /* n */ <String>(); } }\n", &[
        (2, "C", 0, CALL),
        (2, "String", 0, USAGE),
    ]),
    (ProgrammingLanguage::JavaScript, "js", "function f() {}\nfunction g() { (/* n */ f /* n */)(); }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::TypeScript, "ts", "function f<T>() {}\nfunction g() { (f /* n */ <number>)(); }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Rust, "rs", "fn f<T>() {}\nfn g() { f /* n */ :: /* n */ <i32>(); }\n", &[
        (2, "f", 0, CALL),
    ]),
    (ProgrammingLanguage::Swift, "swift", "class C<T> {}\nfunc run() {\n _ = C /* n */ <Int>()\n _ = C/*n*/<Int>()\n _ = N.C /* n */ <Int>()\n _ = C<Int /* n */>()\n _ = -. /* n */ some.f()\n _ = -.some /* n */ .f()\n _ = !. /* n */ some.f()\n}\nfunc annotation(x: C /* n */ <Int>) {}\n", &[
        (3, "C", 0, CALL),
        (3, "Int", 0, USAGE),
        (4, "C", 0, CALL),
        (4, "Int", 0, USAGE),
        (5, "N", 0, USAGE),
        (5, "C", 0, CALL),
        (5, "Int", 0, USAGE),
        (6, "C", 0, CALL),
        (6, "Int", 0, USAGE),
        (7, "some", 0, USAGE),
        (7, "f", 0, CALL),
        (8, "some", 0, USAGE),
        (8, "f", 0, CALL),
        (9, "some", 0, USAGE),
        (9, "f", 0, CALL),
        (11, "C", 0, USAGE),
        (11, "Int", 0, USAGE),
    ]),
    (
        ProgrammingLanguage::Rust,
        "rs",
        "fn f() {\n    f();\n    (f)();\n    ((f))();\n    g((f));\n    (obj).m();\n    (f::<u8>)();\n    (W::new)();\n    (*p)();\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "obj", 0, USAGE),
            (6, "m", 0, CALL),
            (7, "f", 0, CALL),
            (8, "W", 0, USAGE),
            (8, "new", 0, CALL),
            (9, "p", 0, USAGE),
        ],
    ),
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function f() {\n  f();\n  (f)();\n  ((f))();\n  (f as () => void)();\n  (<any>f)();\n  f!();\n  (f satisfies any)();\n  ((f as any)!)();\n  g((f));\n  (obj).m();\n  (obj.m)();\n  obj.m!();\n  (g, f)();\n  (c ? f : g)();\n  new (F)();\n}\n@f class K {}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "f", 0, CALL),
            (6, "f", 0, CALL),
            (7, "f", 0, CALL),
            (8, "f", 0, CALL),
            (9, "f", 0, CALL),
            (10, "g", 0, CALL),
            (10, "f", 0, USAGE),
            (11, "obj", 0, USAGE),
            (11, "m", 0, CALL),
            (12, "obj", 0, USAGE),
            (12, "m", 0, CALL),
            (13, "m", 0, CALL),
            (14, "g", 0, USAGE),
            (14, "f", 0, USAGE),
            (15, "c", 0, USAGE),
            (15, "f", 0, USAGE),
            (16, "F", 0, CALL),
            (18, "f", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::JavaScript,
        "js",
        "function f() {\n  f();\n  (f)();\n  ((f))();\n  g((f));\n  (obj).m();\n  (obj.m)();\n  (g, f)();\n  (c ? f : g)();\n  new (F)();\n  f`x`;\n}\n@f class K {}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "obj", 0, USAGE),
            (6, "m", 0, CALL),
            (7, "m", 0, CALL),
            (8, "f", 0, USAGE),
            (9, "f", 0, USAGE),
            (10, "F", 0, CALL),
            (11, "f", 0, CALL),
            (13, "f", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Python,
        "py",
        "def f():\n    f()\n    (f)()\n    ((f))()\n    g((f))\n    (obj).m()\n    (f if c else g)()\n@f\n@a.b\ndef h():\n    pass\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "obj", 0, USAGE),
            (6, "m", 0, CALL),
            (7, "f", 0, USAGE),
            (8, "f", 0, CALL),
            (9, "a", 0, USAGE),
            (9, "b", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Go,
        "go",
        "package p\n\nfunc f() {\n\tf()\n\t(f)()\n\t((f))()\n\tg((f))\n\t(obj).m()\n\tpkg.F()\n\tf[int]()\n}\n",
        &[
            (4, "f", 0, CALL),
            (5, "f", 0, CALL),
            (6, "f", 0, CALL),
            (7, "g", 0, CALL),
            (7, "f", 0, USAGE),
            (8, "obj", 0, USAGE),
            (8, "m", 0, CALL),
            (9, "pkg", 0, USAGE),
            (9, "F", 0, CALL),
            // Grammar limit: `f[int]` is an `index_expression`, the same
            // node as indexing `fs[0]`, so it is never read as the callee.
            (10, "f", 0, USAGE),
        ],
    ),
    (
        ProgrammingLanguage::C,
        "c",
        "void f() {\n    f();\n    (f)();\n    ((f))();\n    g((f));\n    (*f)();\n    s.cb();\n    p->cb();\n    (g, f)();\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "f", 0, USAGE),
            (7, "s", 0, USAGE),
            (7, "cb", 0, CALL),
            (8, "cb", 0, CALL),
            (9, "f", 0, USAGE),
        ],
    ),
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void f() {\n    f();\n    (f)();\n    ((f))();\n    g((f));\n    f<int>();\n    (f<int>)();\n    ns::f<int>();\n    obj.m();\n    (obj).m();\n    obj.template m<int>();\n    new F();\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "f", 0, CALL),
            (7, "f", 0, CALL),
            (8, "ns", 0, USAGE),
            (8, "f", 0, CALL),
            (9, "obj", 0, USAGE),
            (9, "m", 0, CALL),
            (10, "obj", 0, USAGE),
            (10, "m", 0, CALL),
            (11, "m", 0, CALL),
            (12, "F", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::CSharp,
        "cs",
        "class C {\n    void f() {\n        f();\n        (f)();\n        ((f))();\n        g((f));\n        f<int>();\n        (obj).m();\n        new F();\n        new List<int>();\n    }\n}\n",
        &[
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "f", 0, CALL),
            (6, "g", 0, CALL),
            (6, "f", 0, USAGE),
            (7, "f", 0, CALL),
            (8, "obj", 0, USAGE),
            (8, "m", 0, CALL),
            (9, "F", 0, CALL),
            (10, "List", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Java,
        "java",
        "class A {\n    void f() {\n        f();\n        g((f));\n        new F();\n        (obj).m();\n        this.<T>f();\n    }\n}\n",
        &[
            (3, "f", 0, CALL),
            (4, "g", 0, CALL),
            (4, "f", 0, USAGE),
            (5, "F", 0, CALL),
            (6, "obj", 0, USAGE),
            (6, "m", 0, CALL),
            (7, "f", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func f() {\n    f();\n    (f)();\n    ((f))();\n    g((f));\n    f { };\n    (obj).m();\n    obj.m();\n    F();\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "f", 0, CALL),
            (7, "obj", 0, USAGE),
            (7, "m", 0, CALL),
            (8, "obj", 0, USAGE),
            (8, "m", 0, CALL),
            (9, "F", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object O {\n  def f(): Unit = {\n    f();\n    (f)();\n    ((f))();\n    g((f));\n    new F();\n    f[Int]();\n    obj m 1;\n    (obj).m();\n    f { 1 };\n    obj m;\n  }\n}\n",
        &[
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "f", 0, CALL),
            (6, "g", 0, CALL),
            (6, "f", 0, USAGE),
            (7, "F", 0, CALL),
            (8, "f", 0, CALL),
            (9, "obj", 0, USAGE),
            (9, "m", 0, CALL),
            (10, "obj", 0, USAGE),
            (10, "m", 0, CALL),
            (11, "f", 0, CALL),
            (12, "m", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction f() {\n    f();\n    (f)();\n    g((f));\n    new F();\n    new F;\n    F::m();\n    $obj->m();\n    $obj?->m();\n    ($obj)->m();\n    N\\f();\n}\n",
        &[
            (3, "f", 0, CALL),
            // A bare name in parentheses is a constant: `(f)()` calls the
            // constant's value, never function `f`.
            (4, "f", 0, USAGE),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "F", 0, CALL),
            (7, "F", 0, CALL),
            (8, "F", 0, USAGE),
            (8, "m", 0, CALL),
            (9, "m", 0, CALL),
            (10, "m", 0, CALL),
            (11, "obj", 0, USAGE),
            (11, "m", 0, CALL),
            (12, "N", 0, USAGE),
            (12, "f", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Lua,
        "lua",
        "function f()\n  f()\n  ;(f)()\n  ;((f))()\n  g((f))\n  f \"s\"\n  f {}\n  obj:m()\n  ;(obj).m()\nend\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "g", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "f", 0, CALL),
            (7, "f", 0, CALL),
            (8, "obj", 0, USAGE),
            (8, "m", 0, CALL),
            (9, "obj", 0, USAGE),
            (9, "m", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Perl,
        "pl",
        "sub f {\n    f();\n    &f();\n    &f;\n    f 1;\n    f;\n    Pkg::f();\n    $obj->f();\n    my $r = \\&f;\n    defined &f;\n    goto &f;\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "f", 0, CALL),
            (6, "f", 0, CALL),
            (7, "f", 0, CALL),
            (8, "f", 0, CALL),
            (9, "f", 0, USAGE),
            (10, "f", 0, USAGE),
            (11, "f", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Bash,
        "sh",
        "f() {\n  f\n  X=1 f\n  f arg\n  echo f\n  v=$(f)\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (4, "arg", 0, USAGE),
            (5, "echo", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "f", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::Zsh,
        "zsh",
        "f() {\n  f\n  X=1 f\n  f arg\n  echo f\n  v=$(f)\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (4, "arg", 0, USAGE),
            (5, "echo", 0, CALL),
            (5, "f", 0, USAGE),
            (6, "f", 0, CALL),
        ],
    ),
    // TypeScript instantiation expressions (`f<number>` written apart from
    // its call), alone and behind every wrapper, with the direct generic
    // forms and the non-call uses as controls.
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function f<T>() {}\nclass W { m<T>() {} }\nclass C<T> {}\nfunction g(obj: W) {\n  (f<number>)();\n  ((f<number>))();\n  (f<number>)!();\n  (f<number> as () => void)();\n  (f<number> satisfies () => void)();\n  (<() => void>f<number>)();\n  (obj.m<number>)();\n  new (C<number>)();\n  f<number>();\n  obj.m<number>();\n  new C<number>();\n  const x = f<number>;\n  consume(f<number>);\n}\n",
        &[
            (5, "f", 0, CALL),
            (6, "f", 0, CALL),
            (7, "f", 0, CALL),
            (8, "f", 0, CALL),
            (9, "f", 0, CALL),
            (10, "f", 0, CALL),
            (11, "obj", 0, USAGE),
            (11, "m", 0, CALL),
            (12, "C", 0, CALL),
            (13, "f", 0, CALL),
            (14, "obj", 0, USAGE),
            (14, "m", 0, CALL),
            (15, "C", 0, CALL),
            (16, "f", 0, USAGE),
            (17, "consume", 0, CALL),
            (17, "f", 0, USAGE),
        ],
    ),
    // The other generic readers behind parentheses: C# and Scala
    // instantiations are calls; Go `f[int]` stays a usage (an
    // `index_expression`, the grammar limit above).
    (
        ProgrammingLanguage::CSharp,
        "cs",
        "class C {\n    void f<T>() {\n        (f<int>)();\n        ((f<int>))();\n    }\n}\n",
        &[(3, "f", 0, CALL), (4, "f", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object O {\n  def f[T](): Unit = {\n    (f[Int])();\n    ((f[Int]))();\n  }\n}\n",
        &[(3, "f", 0, CALL), (4, "f", 0, CALL)],
    ),
    (ProgrammingLanguage::Go, "go", "package p\n\nfunc f[T any]() {\n\t(f[int])()\n}\n", &[(4, "f", 0, USAGE)]),
    // Member callees behind any depth of wrappers.
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function f() {\n  ((obj.m))();\n  (obj.m as any)();\n  (obj.m!)();\n  (<any>obj.m)();\n  (((obj.m as any)!) satisfies any)();\n  new ((ns.W))();\n  ((obj).m)();\n}\n",
        &[
            (2, "obj", 0, USAGE),
            (2, "m", 0, CALL),
            (3, "m", 0, CALL),
            (4, "m", 0, CALL),
            (5, "m", 0, CALL),
            (6, "m", 0, CALL),
            (7, "W", 0, CALL),
            (8, "obj", 0, USAGE),
            (8, "m", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::JavaScript,
        "js",
        "function f() {\n  ((obj.m))();\n  new ((ns.W))();\n  (((obj.#p)))();\n}\n",
        &[(2, "m", 0, CALL), (3, "W", 0, CALL), (4, "#p", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Go,
        "go",
        "package p\n\nfunc f() {\n\t((obj.m))()\n\t((pkg.F))()\n}\n",
        &[(4, "obj", 0, USAGE), (4, "m", 0, CALL), (5, "F", 0, CALL)],
    ),
    (
        ProgrammingLanguage::C,
        "c",
        "void f() {\n    ((s.cb))();\n    ((p->cb))();\n}\n",
        &[(2, "s", 0, USAGE), (2, "cb", 0, CALL), (3, "cb", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void f() {\n    ((obj.m))();\n    ((obj.template m<int>))();\n    ((f<int>))();\n    ((ns::f<int>))();\n    ((p->m))();\n}\n",
        &[(2, "m", 0, CALL), (3, "m", 0, CALL), (4, "f", 0, CALL), (5, "f", 0, CALL), (6, "m", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Python,
        "py",
        "def f():\n    ((obj.m))()\n",
        &[(2, "obj", 0, USAGE), (2, "m", 0, CALL)],
    ),
    (
        ProgrammingLanguage::CSharp,
        "cs",
        "class C {\n    void f() {\n        ((obj.m))();\n    }\n}\n",
        &[(3, "obj", 0, USAGE), (3, "m", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func f() {\n    ((obj.m))();\n}\n",
        &[(2, "obj", 0, USAGE), (2, "m", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object O {\n  def f(): Unit = {\n    ((obj.m))();\n  }\n}\n",
        &[(3, "obj", 0, USAGE), (3, "m", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Lua,
        "lua",
        "function f()\n  ;((obj.m))()\nend\n",
        &[(2, "obj", 0, USAGE), (2, "m", 0, CALL)],
    ),
    // Review 4 (R4-F2.01–.02): a rooted name (`::f`) is the callee, at any
    // wrapper depth; as an argument it is not.
    (
        ProgrammingLanguage::Rust,
        "rs",
        "fn f() {\n    ::f();\n    (::f)();\n    ((::f))();\n    g((::f));\n}\n",
        &[(2, "f", 0, CALL), (3, "f", 0, CALL), (4, "f", 0, CALL), (5, "g", 0, CALL), (5, "f", 0, USAGE)],
    ),
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void f() {\n    ::f();\n    (::f)();\n    ((::f))();\n    (((::f)))();\n    g((::f));\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, CALL),
            (5, "f", 0, CALL),
            (6, "g", 0, CALL),
            (6, "f", 0, USAGE),
        ],
    ),
    // R4-F2.03–.06 and .15: a C# conditional member invocation calls the
    // member, generic or not; a chained conditional calls only the last
    // name. Direct, suppressed, generic and delegate forms are controls.
    (
        ProgrammingLanguage::CSharp,
        "cs",
        "class C {\n    void g(W obj, Action cb) {\n        obj?.F();\n        obj?.M<int>();\n        obj?.A();\n        obj?.A?.Invoke();\n        cb?.Invoke();\n        obj.F();\n        obj!.F();\n        obj.M<int>();\n        (obj.M<int>)();\n        cb();\n        (cb)();\n        obj?.A.Invoke();\n        g(obj?.A);\n    }\n}\n",
        &[
            (3, "obj", 0, USAGE),
            (3, "F", 0, CALL),
            (4, "M", 0, CALL),
            (5, "A", 0, CALL),
            (6, "A", 0, USAGE),
            (6, "Invoke", 0, CALL),
            (7, "cb", 0, USAGE),
            (7, "Invoke", 0, CALL),
            (8, "obj", 0, USAGE),
            (8, "F", 0, CALL),
            (9, "F", 0, CALL),
            (10, "M", 0, CALL),
            (11, "M", 0, CALL),
            (12, "cb", 0, CALL),
            (13, "cb", 0, CALL),
            (14, "A", 0, USAGE),
            (14, "Invoke", 0, CALL),
            (15, "g", 0, CALL),
            (15, "A", 0, USAGE),
        ],
    ),
    // R4-F2.07–.14, .16–.17 and the placeholder/unpacking pairs: PHP
    // callable creation (`f(...)`, the grammar's `variadic_placeholder`)
    // calls nothing, while unpacking (`f(...$args)`) calls, for every named,
    // qualified, static, instance and variable-valued callee. A variable's
    // value is invoked through any depth of parentheses; a constant
    // (`(f)()`) or a property value (`($o->m)()`) never calls `f` or `m`.
    (
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction g($cb, $o, $args) {\n    f(...);\n    (f(...));\n    \\Ns\\f(...);\n    W::s(...);\n    $o->m(...);\n    consume(f(...));\n    $cb(...);\n    ($cb)(...);\n    f(...$args);\n    \\Ns\\f(...$args);\n    W::s(...$args);\n    $o->m(...$args);\n    $cb(...$args);\n    ($cb)(...$args);\n    $cb();\n    ($cb)();\n    (($cb))();\n    new $C();\n    (f)();\n    ($o->m)();\n    f();\n    W::s();\n    $o->m();\n    $o?->m();\n    new F();\n    $o->$m();\n    consume($cb);\n}\n",
        &[
            (3, "f", 0, USAGE),
            (4, "f", 0, USAGE),
            (5, "Ns", 0, USAGE),
            (5, "f", 0, USAGE),
            (6, "W", 0, USAGE),
            (6, "s", 0, USAGE),
            (7, "o", 0, USAGE),
            (7, "m", 0, USAGE),
            (8, "consume", 0, CALL),
            (8, "f", 0, USAGE),
            (9, "cb", 0, USAGE),
            (10, "cb", 0, USAGE),
            (11, "f", 0, CALL),
            (11, "args", 0, USAGE),
            (12, "f", 0, CALL),
            (13, "s", 0, CALL),
            (14, "o", 0, USAGE),
            (14, "m", 0, CALL),
            (15, "cb", 0, CALL),
            (15, "args", 0, USAGE),
            (16, "cb", 0, CALL),
            (17, "cb", 0, CALL),
            (18, "cb", 0, CALL),
            (19, "cb", 0, CALL),
            (20, "C", 0, CALL),
            (21, "f", 0, USAGE),
            (22, "o", 0, USAGE),
            (22, "m", 0, USAGE),
            (23, "f", 0, CALL),
            (24, "s", 0, CALL),
            (25, "m", 0, CALL),
            (26, "m", 0, CALL),
            (27, "F", 0, CALL),
            // A variable in the method-name position holds the method's
            // name, not a callable.
            (28, "m", 0, USAGE),
            (29, "consume", 0, CALL),
            (29, "cb", 0, USAGE),
        ],
    ),
    // R4-F2.23–.24 and .21: a Perl code reference is called by `$cb->(…)`
    // (a `method_invocation` with no method name) and `&$cb(…)`; a value,
    // an argument, a named method's receiver, a reference and the existence
    // tests are not calls.
    (
        ProgrammingLanguage::Perl,
        "pl",
        "sub g {\n    $cb->();\n    $cb->(1);\n    &$cb();\n    &{$cb}(1);\n    $cb;\n    consume($cb);\n    $cb->m();\n    my $r = \\&$cb;\n    defined &$cb;\n    exists &$cb;\n    exists &f;\n}\n",
        &[
            (2, "$cb", 0, CALL),
            (3, "$cb", 0, CALL),
            (4, "$cb", 0, CALL),
            (5, "$cb", 0, CALL),
            (6, "$cb", 0, USAGE),
            (7, "consume", 0, CALL),
            (7, "$cb", 0, USAGE),
            (8, "$cb", 0, USAGE),
            (8, "m", 0, CALL),
            (9, "$cb", 0, USAGE),
            (10, "$cb", 0, USAGE),
            (11, "$cb", 0, USAGE),
            (12, "f", 0, USAGE),
        ],
    ),
    // R4-F2.18: a value callee (`cb()`, `(cb)()`) is a call in every
    // language whose parentheses restate their operand.
    (ProgrammingLanguage::Rust, "rs", "fn h(cb: fn()) {\n    cb();\n    (cb)();\n}\n", &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)]),
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function h(cb: () => void) {\n  cb();\n  (cb)();\n}\n",
        &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)],
    ),
    (ProgrammingLanguage::JavaScript, "js", "function h(cb) {\n  cb();\n  (cb)();\n}\n", &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)]),
    (
        ProgrammingLanguage::Go,
        "go",
        "package p\n\nfunc h(cb func()) {\n\tcb()\n\t(cb)()\n}\n",
        &[(4, "cb", 0, CALL), (5, "cb", 0, CALL)],
    ),
    (ProgrammingLanguage::Python, "py", "def h(cb):\n    cb()\n    (cb)()\n", &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)]),
    (
        ProgrammingLanguage::C,
        "c",
        "void h(void (*cb)(void)) {\n    cb();\n    (cb)();\n}\n",
        &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void h(void (*cb)()) {\n    cb();\n    (cb)();\n}\n",
        &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func h(cb: () -> Void) {\n    cb();\n    (cb)();\n}\n",
        &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object O {\n  def h(cb: () => Unit): Unit = {\n    cb();\n    (cb)();\n  }\n}\n",
        &[(3, "cb", 0, CALL), (4, "cb", 0, CALL)],
    ),
    (ProgrammingLanguage::Lua, "lua", "function h(cb)\n  cb()\n  ;(cb)()\nend\n", &[(2, "cb", 0, CALL), (3, "cb", 0, CALL)]),
    // R4-F2.19–.20: optional member calls call the member; a Java receiver
    // is never the callee.
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function h(obj: any) {\n  obj?.m();\n  obj.m?.();\n  obj?.m?.();\n}\n",
        &[(2, "obj", 0, USAGE), (2, "m", 0, CALL), (3, "m", 0, CALL), (4, "obj", 0, USAGE), (4, "m", 0, CALL)],
    ),
    (
        ProgrammingLanguage::JavaScript,
        "js",
        "function h(obj) {\n  obj?.m();\n  obj.m?.();\n  obj?.m?.();\n}\n",
        &[(2, "obj", 0, USAGE), (2, "m", 0, CALL), (3, "m", 0, CALL), (4, "obj", 0, USAGE), (4, "m", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func h(obj: W?) {\n    obj?.f();\n}\n",
        &[(2, "obj", 0, USAGE), (2, "f", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Java,
        "java",
        "class A {\n    void h(Runnable cb) {\n        cb.run();\n    }\n}\n",
        &[(3, "cb", 0, USAGE), (3, "run", 0, CALL)],
    ),
    // R4-F2.22: an unquoted variable command name is the command; the
    // variable it expands and the arguments are not.
    (
        ProgrammingLanguage::Bash,
        "sh",
        "h() {\n  $cmd arg\n  echo $x\n}\n",
        &[(2, "$cmd", 0, CALL), (2, "cmd", 0, USAGE), (2, "arg", 0, USAGE), (3, "echo", 0, CALL), (3, "x", 0, USAGE)],
    ),
    (
        ProgrammingLanguage::Zsh,
        "zsh",
        "h() {\n  $cmd arg\n  echo $x\n}\n",
        &[(2, "$cmd", 0, CALL), (2, "cmd", 0, USAGE), (2, "arg", 0, USAGE), (3, "echo", 0, CALL), (3, "x", 0, USAGE)],
    ),
    // Review 5 (R5-F1.01–.04): Java construction of a generic type calls
    // the constructed type's final name only; qualifier prefixes, type
    // arguments, a receiver and an annotation are usages.
    (
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        new C<String>();\n        new a.b.C<String>();\n        new C<>();\n        new a.b.C<>();\n        new a.b.C<String>.D<Integer>();\n        outer.new C<String>();\n        new C<String>() {};\n        new @A a.b.C<String>();\n    }\n}\n",
        &[
            (3, "C", 0, CALL),
            (3, "String", 0, USAGE),
            (4, "a", 0, USAGE),
            (4, "b", 0, USAGE),
            (4, "C", 0, CALL),
            (4, "String", 0, USAGE),
            (5, "C", 0, CALL),
            (6, "a", 0, USAGE),
            (6, "b", 0, USAGE),
            (6, "C", 0, CALL),
            (7, "a", 0, USAGE),
            (7, "b", 0, USAGE),
            (7, "C", 0, USAGE),
            (7, "String", 0, USAGE),
            (7, "D", 0, CALL),
            (7, "Integer", 0, USAGE),
            (8, "outer", 0, USAGE),
            (8, "C", 0, CALL),
            (8, "String", 0, USAGE),
            (9, "C", 0, CALL),
            (9, "String", 0, USAGE),
            (10, "A", 0, USAGE),
            (10, "a", 0, USAGE),
            (10, "b", 0, USAGE),
            (10, "C", 0, CALL),
            (10, "String", 0, USAGE),
        ],
    ),
    // R5-F1.05–.06: Scala construction of a generic type, with or without
    // an argument list or an anonymous body.
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object G {\n  def g(): Unit = {\n    new C[Int]()\n    new a.b.C[Int]()\n    new C[Int]\n    new a.b.C[Int] {}\n    new C[Int](y)\n  }\n}\n",
        &[
            (3, "C", 0, CALL),
            (3, "Int", 0, USAGE),
            (4, "a", 0, USAGE),
            (4, "b", 0, USAGE),
            (4, "C", 0, CALL),
            (4, "Int", 0, USAGE),
            (5, "C", 0, CALL),
            (5, "Int", 0, USAGE),
            (6, "a", 0, USAGE),
            (6, "b", 0, USAGE),
            (6, "C", 0, CALL),
            (6, "Int", 0, USAGE),
            (7, "C", 0, CALL),
            (7, "Int", 0, USAGE),
            (7, "y", 0, USAGE),
        ],
    ),
    // R5-F1.07–.09: Swift specialized construction (`constructor_expression`)
    // and implicit members (`.f()`, `.init()`, `.some(x)`) call the
    // constructed or invoked name; an implicit member that is not invoked,
    // the annotation type, a qualifier and an argument are usages.
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = C<Int>()\n    _ = N.C<Int>()\n    let _: C = .f()\n    let _: C = .f\n    let _: C = .init()\n    let _: E = .some(x)\n}\n",
        &[
            (2, "C", 0, CALL),
            (2, "Int", 0, USAGE),
            (3, "N", 0, USAGE),
            (3, "C", 0, CALL),
            (3, "Int", 0, USAGE),
            (4, "C", 0, USAGE),
            (4, "f", 0, CALL),
            (5, "C", 0, USAGE),
            (5, "f", 0, USAGE),
            (6, "C", 0, USAGE),
            (6, "init", 0, CALL),
            (7, "E", 0, USAGE),
            (7, "some", 0, CALL),
            (7, "x", 0, USAGE),
        ],
    ),
    // R5-F1.10–.12: C++ allocation of a template type, braced construction
    // (`compound_literal_expression`), and the functional-construction
    // controls that already called.
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    new C<int>();\n    new N::C<int>();\n    new C<int>;\n    new C<int>{};\n    Plain{};\n    N::Plain{};\n    C<int>{};\n    N::C<int>{x};\n    Plain();\n    C<int>();\n    (N::C<int>)();\n}\n",
        &[
            (2, "C", 0, CALL),
            (3, "N", 0, USAGE),
            (3, "C", 0, CALL),
            (4, "C", 0, CALL),
            (5, "C", 0, CALL),
            (6, "Plain", 0, CALL),
            (7, "N", 0, USAGE),
            (7, "Plain", 0, CALL),
            (8, "C", 0, CALL),
            (9, "N", 0, USAGE),
            (9, "C", 0, CALL),
            (9, "x", 0, USAGE),
            (10, "Plain", 0, CALL),
            (11, "C", 0, CALL),
            (12, "N", 0, USAGE),
            (12, "C", 0, CALL),
        ],
    ),
    // R5-F1.13: nongeneric construction controls keep their calls.
    (
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        new C();\n        new a.b.C(y);\n    }\n}\n",
        &[(3, "C", 0, CALL), (4, "a", 0, USAGE), (4, "b", 0, USAGE), (4, "C", 0, CALL), (4, "y", 0, USAGE)],
    ),
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object G {\n  def g(): Unit = {\n    new C\n    new a.b.C()\n  }\n}\n",
        &[(3, "C", 0, CALL), (4, "a", 0, USAGE), (4, "b", 0, USAGE), (4, "C", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = C()\n    _ = N.C()\n    _ = C.init()\n}\n",
        &[(2, "C", 0, CALL), (3, "N", 0, USAGE), (3, "C", 0, CALL), (4, "C", 0, USAGE), (4, "init", 0, CALL)],
    ),
    // R5-F1.14: Rust generic tuple-struct construction.
    (
        ProgrammingLanguage::Rust,
        "rs",
        "fn g(x: u8) {\n    C::<u8>(1);\n    N::C::<u8>(x);\n}\n",
        &[(2, "C", 0, CALL), (3, "N", 0, USAGE), (3, "C", 0, CALL), (3, "x", 0, USAGE)],
    ),
    // R5-F1.15: TypeScript and JavaScript construction, direct and wrapped.
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function g(x: number) {\n  new C<number>();\n  new N.C<number>(x);\n  new (C<number>)();\n  new C<number>;\n}\n",
        &[
            (2, "C", 0, CALL),
            (3, "N", 0, USAGE),
            (3, "C", 0, CALL),
            (3, "x", 0, USAGE),
            (4, "C", 0, CALL),
            (5, "C", 0, CALL),
        ],
    ),
    (
        ProgrammingLanguage::JavaScript,
        "js",
        "function g(x) {\n  new C(x);\n  new (C)();\n  new N.C();\n}\n",
        &[(2, "C", 0, CALL), (2, "x", 0, USAGE), (3, "C", 0, CALL), (4, "N", 0, USAGE), (4, "C", 0, CALL)],
    ),
    // R5-F1.16: C# generic construction, with arguments or an initializer.
    (
        ProgrammingLanguage::CSharp,
        "cs",
        "class G {\n    void g() {\n        new C<T>();\n        new N.C<T>();\n        new C();\n        new C<T> { X = y };\n    }\n}\n",
        &[
            (3, "C", 0, CALL),
            (3, "T", 0, USAGE),
            (4, "N", 0, USAGE),
            (4, "C", 0, CALL),
            (4, "T", 0, USAGE),
            (5, "C", 0, CALL),
            (6, "C", 0, CALL),
            (6, "T", 0, USAGE),
            (6, "y", 0, USAGE),
        ],
    ),
    // R5-F1.17: PHP construction by path and by variable; callable creation
    // calls nothing, unpacking calls.
    (
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction g($K, $a) {\n    new N\\C();\n    new \\N\\C();\n    new $K();\n    f(...);\n    f(...$a);\n}\n",
        &[
            (3, "N", 0, USAGE),
            (3, "C", 0, CALL),
            (4, "N", 0, USAGE),
            (4, "C", 0, CALL),
            (5, "K", 0, CALL),
            (6, "f", 0, USAGE),
            (7, "f", 0, CALL),
            (7, "a", 0, USAGE),
        ],
    ),
    // R5-F1.18: Python named, qualified and decorator calls.
    (
        ProgrammingLanguage::Python,
        "py",
        "@deco\n@a.b(x)\ndef g(x):\n    f(x)\n    a.b.f()\n",
        &[
            (1, "deco", 0, CALL),
            (2, "a", 0, USAGE),
            (2, "b", 0, CALL),
            (2, "x", 0, USAGE),
            (4, "f", 0, CALL),
            (4, "x", 0, USAGE),
            (5, "a", 0, USAGE),
            (5, "f", 0, CALL),
        ],
    ),
    // R5-F1.19: Go `pkg.F()` calls; `pkg.F[int]()` keeps the accepted
    // limitation (`F` is no call; see `NOT_CALLED_MATRIX`).
    (
        ProgrammingLanguage::Go,
        "go",
        "package p\n\nfunc g() {\n\tpkg.F()\n\tpkg.F[int]()\n}\n",
        &[(4, "pkg", 0, USAGE), (4, "F", 0, CALL), (5, "pkg", 0, USAGE)],
    ),
    // R5-F1.20: C function pointers, direct and wrapped; a dereference is
    // no callee.
    (
        ProgrammingLanguage::C,
        "c",
        "void g(void (*cb)(void)) {\n    cb();\n    (cb)();\n    (*cb)();\n}\n",
        &[(2, "cb", 0, CALL), (3, "cb", 0, CALL), (4, "cb", 0, USAGE)],
    ),
    // R5-F1.21: Lua, Perl, Bash and Zsh call/argument splits.
    (
        ProgrammingLanguage::Lua,
        "lua",
        "function g(x)\n  f(x)\n  x:m()\n  consume(f)\nend\n",
        &[(2, "f", 0, CALL), (2, "x", 0, USAGE), (3, "x", 0, USAGE), (3, "m", 0, CALL), (4, "consume", 0, CALL), (4, "f", 0, USAGE)],
    ),
    (
        ProgrammingLanguage::Perl,
        "pl",
        "sub g {\n    f();\n    $o->m();\n    consume(\\&f);\n}\n",
        &[(2, "f", 0, CALL), (3, "$o", 0, USAGE), (3, "m", 0, CALL), (4, "consume", 0, CALL), (4, "f", 0, USAGE)],
    ),
    (ProgrammingLanguage::Bash, "sh", "g() {\n  f arg\n}\n", &[(2, "f", 0, CALL), (2, "arg", 0, USAGE)]),
    (ProgrammingLanguage::Zsh, "zsh", "g() {\n  f arg\n}\n", &[(2, "f", 0, CALL), (2, "arg", 0, USAGE)]),
    // R5-F1.S2–.S3, .S10, .S13: a C++ placement `new` constructs; a Scala
    // mixin construction calls its class, not the trait; a Swift prefix
    // operator applies to the call's result (the grammar writes `!f()` as
    // `(!f)()`), while written parentheses call the operator's result.
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    new (place) C<int>();\n    new (place) Plain;\n}\n",
        &[(2, "place", 0, USAGE), (2, "C", 0, CALL), (3, "place", 0, USAGE), (3, "Plain", 0, CALL)],
    ),
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object G {\n  def g(): Unit = {\n    new C with T\n    new C[Int] with T\n  }\n}\n",
        &[(3, "C", 0, CALL), (3, "T", 0, USAGE), (4, "C", 0, CALL), (4, "Int", 0, USAGE), (4, "T", 0, USAGE)],
    ),
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = !f()\n    _ = -f()\n    _ = (-f)()\n    _ = -f\n    h(&f)\n    let _: C? = .some(.f())\n}\n",
        &[
            (2, "f", 0, CALL),
            (3, "f", 0, CALL),
            (4, "f", 0, USAGE),
            (5, "f", 0, USAGE),
            (6, "h", 0, CALL),
            (6, "f", 0, USAGE),
            (7, "some", 0, CALL),
            (7, "f", 0, CALL),
        ],
    ),
    // R5-F1.S17: tree-sitter-swift writes a subscript as a call with
    // bracketed arguments; a subscript invokes nothing, so its operand is a
    // usage behind an operator, a receiver, an optional chain, a label, a
    // trailing closure or another subscript, and calling a subscript's
    // result calls no name.
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = a[0]\n    _ = x.items[0]\n    _ = !a[0]\n    _ = a[0]()\n    _ = f()[0]\n    _ = a?[0]\n    _ = a[keyPath: k]\n    _ = b[0] { 1 }\n    _ = h { 1 }\n    _ = .m[0]\n    _ = c[0][1]\n}\n",
        &[
            (2, "a", 0, USAGE),
            (3, "x", 0, USAGE),
            (3, "items", 0, USAGE),
            (4, "a", 0, USAGE),
            (5, "a", 0, USAGE),
            (6, "f", 0, CALL),
            (7, "a", 0, USAGE),
            (8, "a", 0, USAGE),
            (9, "b", 0, USAGE),
            (10, "h", 0, CALL),
            (11, "m", 0, USAGE),
            (12, "c", 0, USAGE),
        ],
    ),
    // R5-F1.S18: Scala has no subscript syntax; `a(0)` is `a.apply(0)`, an
    // invocation of `a`.
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object G {\n  def g(): Unit = {\n    a(0)\n    x.items(0)\n    a(0)()\n  }\n}\n",
        &[(3, "a", 0, CALL), (4, "x", 0, USAGE), (4, "items", 0, CALL), (5, "a", 0, CALL)],
    ),
    // R6-F1.01–.03, .17: a Swift force unwrap restates its operand at any
    // grouping depth; optional chaining keeps its call and a receiver stays
    // a usage.
    (
        ProgrammingLanguage::Swift,
        "swift",
        SWIFT_FORCE_UNWRAP,
        &[
            (3, "cb", 0, CALL),
            (4, "cb", 0, CALL),
            (5, "cb", 0, CALL),
            (6, "x", 0, USAGE),
            (6, "cb", 0, CALL),
            (7, "x", 0, USAGE),
            (7, "cb", 0, CALL),
            (8, "x", 0, USAGE),
            (8, "cb", 0, CALL),
            (9, "cb", 0, CALL),
            (10, "y", 0, USAGE),
            (10, "f", 0, CALL),
            (15, "x", 0, USAGE),
            (15, "cb", 0, CALL),
            (16, "cb", 0, CALL),
            (21, "cb", 0, CALL),
            (22, "cb", 0, CALL),
            (23, "cb", 0, CALL),
        ],
    ),
    // R6-F1.04, .05, .18: a C# null-forgiving `!` restates its operand at
    // any depth, also after a conditional access; a receiver stays a usage.
    (
        ProgrammingLanguage::CSharp,
        "cs",
        CSHARP_NULL_FORGIVING,
        &[
            (4, "cb", 0, CALL),
            (5, "cb", 0, CALL),
            (6, "obj", 0, USAGE),
            (6, "F", 0, CALL),
            (7, "obj", 0, USAGE),
            (7, "F", 0, CALL),
            (8, "obj", 0, USAGE),
            (8, "F", 0, CALL),
            (9, "F", 0, CALL),
            (10, "cb", 0, CALL),
            (11, "cb", 0, CALL),
            (12, "cb", 0, CALL),
            (13, "obj", 0, USAGE),
            (13, "F", 0, CALL),
            (14, "obj", 0, USAGE),
            (14, "F", 0, CALL),
            (19, "obj", 0, USAGE),
            (19, "F", 0, CALL),
            (21, "cb", 0, CALL),
            (21, "F", 0, CALL),
        ],
    ),
];

/// `(language, extension, source, [(line, name)])`: names on these lines
/// that are never a `Call`, whether or not they are occurrences at all.
type NotCalledRow = (ProgrammingLanguage, &'static str, &'static str, &'static [(u32, &'static str)]);

/// Member accesses that are not the callee: an argument, a sequence, a
/// conditional, a dereference.
const NOT_CALLED_MATRIX: &[NotCalledRow] = &[
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function f() {\n  g((obj.m));\n  (g, obj.m)();\n  (c ? obj.m : obj.n)();\n  g(((obj.m as any)!));\n}\n",
        &[(2, "m"), (3, "m"), (4, "m"), (4, "n"), (5, "m")],
    ),
    // An instantiation that is not the callee invokes nothing.
    (
        ProgrammingLanguage::TypeScript,
        "ts",
        "function f<T>() {\n  const x = f<number>;\n  consume(f<number>);\n  g((obj.m<number>));\n  const y = (obj.m<number>)!;\n}\n",
        &[(2, "f"), (3, "f"), (4, "m"), (5, "m")],
    ),
    (
        ProgrammingLanguage::JavaScript,
        "js",
        "function f() {\n  g((obj.m));\n  (g, obj.m)();\n  (c ? obj.m : obj.n)();\n}\n",
        &[(2, "m"), (3, "m"), (4, "m"), (4, "n")],
    ),
    (ProgrammingLanguage::Go, "go", "package p\n\nfunc f() {\n\tg((obj.m))\n}\n", &[(4, "m")]),
    (
        ProgrammingLanguage::C,
        "c",
        "void f() {\n    g((s.cb));\n    (g, s.cb)();\n    (*s.cb)();\n}\n",
        &[(2, "cb"), (3, "cb"), (4, "cb")],
    ),
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void f() {\n    g((obj.m));\n    (g, obj.m)();\n    (c ? obj.m : obj.n)();\n}\n",
        &[(2, "m"), (3, "m"), (4, "m"), (4, "n")],
    ),
    // A Rust parenthesized field is the field's value, never a method.
    (ProgrammingLanguage::Rust, "rs", "fn f() {\n    (x.m)();\n}\n", &[(2, "m")]),    // R4-F2: wrapper traversal never promotes a rooted name or a
    // variable inside a sequence, a conditional or a dereference.
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void f() {\n    (g, ::f)();\n    (c ? ::f : g)();\n    (*::f)();\n    ((g, (::f)))();\n}\n",
        &[(2, "f"), (3, "f"), (4, "f"), (5, "f")],
    ),
    (
        ProgrammingLanguage::CSharp,
        "cs",
        "class C {\n    void f(bool c, Action g) {\n        (c ? f : g)();\n        ((c ? (f) : g))();\n    }\n}\n",
        &[(3, "f"), (4, "f")],
    ),
    (
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction g($cb, $d) {\n    ($cb ?: $d)();\n    (($c ? $cb : $d))();\n}\n",
        &[(3, "cb"), (4, "cb")],
    ),
    // R5-F1.22: a generic type that no construction holds is never called:
    // annotations, declarations initialized in place, array creation, a
    // declared base, an argument and a Swift implicit member value.
    (
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        C<String> x = make(y);\n        a.b.C<String> z = null;\n        Object w = new C<String>[2];\n        Object v = new int[3];\n    }\n}\n",
        &[(3, "C"), (3, "String"), (3, "y"), (4, "a"), (4, "b"), (4, "C"), (5, "C"), (5, "String")],
    ),
    (
        ProgrammingLanguage::Scala,
        "scala",
        "object G {\n  def g(): Unit = {\n    val x: C[Int] = make(y)\n    val z: a.b.C[Int] = null\n  }\n}\nclass X extends C[Int](1)\nclass Y extends C(1)\n",
        &[(3, "C"), (3, "Int"), (3, "y"), (4, "a"), (4, "b"), (4, "C"), (7, "C"), (7, "Int"), (8, "C")],
    ),
    (
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    let x: C<Int> = .f\n    let z: N.C<Int> = make(y)\n    h(.f)\n}\n",
        &[(2, "C"), (2, "Int"), (2, "f"), (3, "N"), (3, "C"), (3, "Int"), (3, "y"), (4, "f")],
    ),
    // R5-F1.S2, .S4, .S6: an array `new` writes no constructor call, a
    // declaration's braced initializer is no expression, and a C compound
    // literal constructs nothing.
    (
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    C<int> v;\n    N::C<int> w = make(y);\n    new C<int>[2];\n    new Plain[2];\n    Plain p{1};\n    C<int> z{};\n}\n",
        &[(2, "C"), (3, "N"), (3, "C"), (3, "y"), (4, "C"), (5, "Plain"), (6, "Plain"), (7, "C")],
    ),
    (ProgrammingLanguage::C, "c", "void g() {\n    (struct S){1};\n    (S){1};\n}\n", &[(2, "S"), (3, "S")]),
    // R5-F1.19: Go `pkg.F[int]()` is written like indexing.
    (ProgrammingLanguage::Go, "go", "package p\n\nfunc g() {\n\tpkg.F[int]()\n\tF[int]()\n}\n", &[(4, "F"), (5, "F")]),
    // R5-F1.S16: a Rust struct literal names a type and calls nothing.
    (ProgrammingLanguage::Rust, "rs", "fn g() {\n    C { x: 1 };\n    N::C::<u8> { x: 1 };\n}\n", &[(2, "C"), (3, "N"), (3, "C")]),
    // R5-F1.S19: indexing, and calling an element, invokes no name in any
    // grammar that writes an index node of its own.
    (ProgrammingLanguage::Rust, "rs", "fn g() {\n    a[0];\n    a[0]();\n    x.items[0];\n}\n", &[(2, "a"), (3, "a"), (4, "items")]),
    (ProgrammingLanguage::TypeScript, "ts", "function g() {\n  a[0];\n  a[0]();\n  a?.[0];\n  x.items[0];\n}\n", &[(2, "a"), (3, "a"), (4, "a"), (5, "items")]),
    (ProgrammingLanguage::JavaScript, "js", "function g() {\n  a[0];\n  a[0]();\n  x.items[0];\n}\n", &[(2, "a"), (3, "a"), (4, "items")]),
    (ProgrammingLanguage::Go, "go", "package p\n\nfunc g() {\n\t_ = a[0]\n\ta[0]()\n\t_ = x.Items[0]\n}\n", &[(4, "a"), (5, "a"), (6, "Items")]),
    (ProgrammingLanguage::Python, "py", "def g():\n    a[0]\n    a[0]()\n    x.items[0]\n", &[(2, "a"), (3, "a"), (4, "items")]),
    (ProgrammingLanguage::Java, "java", "class G {\n    void g() {\n        int r = a[0];\n        int s = x.items[0];\n    }\n}\n", &[(3, "a"), (4, "items")]),
    (ProgrammingLanguage::CSharp, "cs", "class G {\n    void g() {\n        var r = a[0];\n        a[0]();\n        var s = a?[0];\n        var t = x.Items[0];\n    }\n}\n", &[(3, "a"), (4, "a"), (5, "a"), (6, "Items")]),
    (ProgrammingLanguage::C, "c", "void g() {\n    a[0];\n    a[0]();\n    x.items[0];\n}\n", &[(2, "a"), (3, "a"), (4, "items")]),
    (ProgrammingLanguage::Cpp, "cpp", "void g() {\n    a[0];\n    a[0]();\n    x.items[0];\n}\n", &[(2, "a"), (3, "a"), (4, "items")]),
    (ProgrammingLanguage::Php, "php", "<?php\nfunction g() {\n    $a[0];\n    $a[0]();\n    $x->items[0];\n    A[0];\n}\n", &[(3, "a"), (4, "a"), (5, "items"), (6, "A")]),
    (ProgrammingLanguage::Lua, "lua", "function g()\n  local r = a[1]\n  a[1]()\n  local s = x.items[1]\nend\n", &[(2, "a"), (3, "a"), (4, "items")]),
    (ProgrammingLanguage::Perl, "pl", "sub g {\n    my $r = $a[0];\n    $a[0]->();\n    my $s = $x->{items}[0];\n}\n", &[(2, "$a"), (3, "$a"), (4, "items")]),
    // R6-F1.06: a Swift unwrapped value, argument, conditional or subscript
    // element is never the callee.
    (
        ProgrammingLanguage::Swift,
        "swift",
        SWIFT_FORCE_UNWRAP,
        &[(11, "cb"), (12, "cb"), (13, "cb"), (13, "other"), (14, "a"), (17, "cb"), (18, "cb")],
    ),
    // R6-F1.07: the same in C#, and postfix `++`/`--` are no null-forgiving
    // wrappers.
    (
        ProgrammingLanguage::CSharp,
        "cs",
        CSHARP_NULL_FORGIVING,
        &[(15, "cb"), (16, "cb"), (17, "cb"), (17, "other"), (18, "arr"), (20, "i"), (22, "F")],
    ),
];

crate::language_cases::language_tests!(member_accesses_that_are_not_the_callee_are_never_calls => check_member_accesses_that_are_not_the_callee_are_never_calls);

fn check_member_accesses_that_are_not_the_callee_are_never_calls(language: ProgrammingLanguage) {
    for &(language, extension, source, checks) in NOT_CALLED_MATRIX.iter().filter(|row| row.0 == language) {
        let file = Analyzed::new(source, extension);
        for &(line, name) in checks {
            let calls = file
                .all(name)
                .into_iter()
                .filter(|occurrence| occurrence.span.start.line == line && occurrence.kind == CALL)
                .collect::<Vec<_>>();
            assert!(calls.is_empty(), "{}: `{name}` on line {line}: {calls:#?}", language.name());
        }
    }
}

type KindRow = (&'static str, &'static str, &'static [(u32, &'static str, OccurrenceKind)]);

/// `(extension, source, [(line, name, kind)])`: R4-F2 repairs in languages
/// without binding rules.
const LIMITED_INVOCATION_ROWS: &[KindRow] = &[
    (
        "cs",
        "class W {\n    public void F() {}\n    void G(W obj) {\n        obj?.F();\n        obj?.G<int>();\n    }\n}\n",
        &[(4, "F", CALL), (5, "G", CALL)],
    ),
    (
        "php",
        "<?php\nfunction f() {}\nfunction g($cb) {\n    f(...);\n    $cb();\n    f(...$cb);\n}\n",
        &[(4, "f", USAGE), (5, "cb", CALL), (6, "f", CALL)],
    ),
    ("pl", "sub f {}\nsub g {\n    $cb->();\n    &$cb();\n    f();\n}\n", &[(3, "$cb", CALL), (4, "$cb", CALL), (5, "f", CALL)]),
    // R5-F1.C2–C3: the repaired construction and implicit-member calls stay
    // conservative in every language they touch.
    (
        "java",
        "class C<T> {\n    C() {}\n}\nclass G {\n    void g() {\n        new C<String>();\n        new a.b.C<>();\n    }\n}\n",
        &[(6, "C", CALL), (6, "String", USAGE), (7, "C", CALL)],
    ),
    ("scala", "class C[T]\nobject G {\n  def g(): Unit = {\n    new C[Int]()\n    new C[Int] with T\n  }\n}\n", &[(4, "C", CALL), (5, "C", CALL)]),
    (
        "swift",
        "class C<T> {\n    static func f() -> C<T> { C<T>() }\n}\nfunc g() {\n    _ = C<Int>()\n    let _: C<Int> = .f()\n    _ = !f()\n}\n",
        &[(5, "C", CALL), (6, "f", CALL), (7, "f", CALL)],
    ),
    (
        "cpp",
        "struct Plain { Plain() {} };\ntemplate <typename T> struct C { C() {} };\nvoid g() {\n    new C<int>();\n    Plain{};\n    C<int>{};\n}\n",
        &[(4, "C", CALL), (5, "Plain", CALL), (6, "C", CALL)],
    ),
];

/// R4-F2.C2–C3: a repaired call kind in a language without binding rules
/// keeps the conservative evidence: `NameOnly` over every same-named
/// record or `Unresolved`, no target ID, no edge, `unsupported_language`
/// coverage, and `Unknown` recursion.
#[test]
fn repaired_invocations_in_limited_languages_stay_conservative() {
    for &(extension, source, rows) in LIMITED_INVOCATION_ROWS {
        let file = Analyzed::new(source, extension);
        for &(line, name, kind) in rows {
            let occurrence = file.at(line, name, 0);
            assert_eq!(occurrence.kind, kind, "{extension}: `{name}` on line {line}");
            assert!(
                matches!(occurrence.confidence, Confidence::NameOnly | Confidence::Unresolved),
                "{extension}: `{name}` on line {line}: {occurrence:#?}"
            );
            assert!(occurrence.target.id.is_none(), "{extension}: `{name}` on line {line}: {occurrence:#?}");
        }
        assert!(
            file.evidence
                .reference_coverage
                .iter()
                .all(|coverage| coverage.reasons.contains(&LimitReason::UnsupportedLanguage)),
            "{extension}: {:#?}",
            file.evidence.reference_coverage
        );
        for record in &file.index.symbols {
            assert!(record.relations.references.is_empty(), "{extension}: `{}` has edges", record.identity.name);
            assert!(record.relations.referenced_by.is_empty(), "{extension}: `{}` has edges", record.identity.name);
            if record.semantics.is_recursive.is_some() {
                assert_eq!(record.semantics.is_recursive, Some(TriState::Unknown), "{extension}: `{}`", record.identity.name);
            }
        }
    }
}

/// `(label, extension, source, [(line, name, nth, written path, lines of
/// the same-named records the occurrence names as candidates)])`.
type NullAssertionRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(u32, &'static str, usize, Option<&'static str>, &'static [u32])],
);

/// R6-F1.C1–C3: the repaired calls keep their name-token spans and written
/// paths (`x.cb`, `obj?.F`) and the conservative evidence of a language
/// without binding rules: `NameOnly` over exactly the same-named records
/// the source declares (`Unresolved` when it declares none), no target ID,
/// no edge, `unsupported_language` coverage and `Unknown` recursion.
const NULL_ASSERTION_ROWS: &[NullAssertionRow] = &[
    (
        "R6-F1 swift force unwrap",
        "swift",
        SWIFT_FORCE_UNWRAP,
        &[
            (3, "cb", 0, None, &[1, 2, 20]),
            (5, "cb", 0, None, &[1, 2, 20]),
            (6, "cb", 0, Some("x.cb"), &[1, 2, 20]),
            (7, "cb", 0, Some("x.cb"), &[1, 2, 20]),
            (8, "cb", 0, Some("x.cb"), &[1, 2, 20]),
            (10, "f", 0, Some("y!.f"), &[]),
            (22, "cb", 0, None, &[1, 2, 20]),
        ],
    ),
    (
        "R6-F1 csharp null-forgiving",
        "cs",
        CSHARP_NULL_FORGIVING,
        &[
            (4, "cb", 0, None, &[3]),
            (6, "F", 0, Some("obj.F"), &[2]),
            (8, "F", 0, Some("obj.F"), &[2]),
            (9, "F", 0, None, &[2]),
            (12, "cb", 0, None, &[3]),
            (13, "F", 0, Some("obj?.F"), &[2]),
            (14, "F", 0, Some("obj.F"), &[2]),
        ],
    ),
];

#[test]
fn null_assertion_invocations_keep_their_paths_and_stay_conservative() {
    for &(label, extension, source, rows) in NULL_ASSERTION_ROWS {
        let file = Analyzed::new(source, extension);
        for &(line, name, nth, path, candidate_lines) in rows {
            let occurrence = file.at(line, name, nth);
            assert_eq!(occurrence.kind, CALL, "{label}: `{name}` on line {line}: {occurrence:#?}");
            assert_eq!(file.text(occurrence), name, "{label}: the span stays the name token");
            assert_eq!(occurrence.target.qualified_name.as_deref(), path, "{label}: `{name}` on line {line}");
            assert_eq!(occurrence.target.id, None, "{label}: `{name}` on line {line}");
            let expected: Vec<&SymbolId> = candidate_lines.iter().map(|&line| file.named(name, line)).collect();
            let candidates: Vec<&SymbolId> = occurrence.candidates.iter().filter_map(|candidate| candidate.id.as_ref()).collect();
            assert_eq!(candidates, expected, "{label}: `{name}` on line {line}");
            let confidence = if expected.is_empty() { Confidence::Unresolved } else { Confidence::NameOnly };
            assert_eq!(occurrence.confidence, confidence, "{label}: `{name}` on line {line}");
        }
        assert!(
            file.evidence
                .reference_coverage
                .iter()
                .all(|coverage| coverage.state == CoverageState::Limited
                    && coverage.reasons.contains(&LimitReason::UnsupportedLanguage)),
            "{label}: {:#?}",
            file.evidence.reference_coverage
        );
        for record in &file.index.symbols {
            assert!(record.relations.references.is_empty(), "{label}: `{}` has edges", record.identity.name);
            assert!(record.relations.referenced_by.is_empty(), "{label}: `{}` has edges", record.identity.name);
        }
        for callable in ["g", "h"] {
            for record in file.index.symbols.iter().filter(|record| record.identity.name == callable) {
                assert_eq!(record.semantics.is_recursive, Some(TriState::Unknown), "{label}: `{callable}`");
            }
        }
    }
}

/// R6-F1.17 grammar limit: tree-sitter-swift 0.7.1 lexes `cb!!()` and
/// `cb?!()` as the custom infix operators `!!`/`?!` with a missing operand,
/// so the region is limited by the parse error and no call is invented
/// from the misparse. `(cb!)!()` and `(cb!)?()` spell the same calls and
/// are calls (`SWIFT_FORCE_UNWRAP`).
#[test]
fn swift_adjacent_unwrap_operators_are_a_grammar_limit() {
    let file = Analyzed::new("func h(cb: (() -> Void)??) {\n    cb!!()\n}\nfunc k(cb: (() -> Void)??) {\n    cb?!()\n}\n", "swift");
    for line in [2, 5] {
        assert_eq!(file.at(line, "cb", 0).kind, USAGE, "line {line}");
    }
    assert!(
        file.evidence.reference_coverage.iter().any(|coverage| coverage.reasons.contains(&LimitReason::ParseError)),
        "{:#?}",
        file.evidence.reference_coverage
    );
}

crate::language_cases::language_tests!(invocation_forms_are_calls_in_every_language => check_invocation_forms_are_calls_in_every_language);

fn check_invocation_forms_are_calls_in_every_language(language: ProgrammingLanguage) {
    for language in ProgrammingLanguage::all() {
        assert!(
            INVOCATION_MATRIX.iter().any(|row| row.0 == language),
            "{}: no invocation row",
            language.name()
        );
    }
    for &(language, extension, source, checks) in INVOCATION_MATRIX.iter().filter(|row| row.0 == language) {
        let file = Analyzed::new(source, extension);
        for &(line, name, nth, kind) in checks {
            let occurrence = file.at(line, name, nth);
            assert_eq!(occurrence.kind, kind, "{}: `{name}` on line {line}: {occurrence:#?}", language.name());
        }
    }
}

/// A wrapped self-call keeps the binding a direct one has, in every
/// language that resolves bindings.
const WRAPPED_CALL_BINDINGS: &[BoundaryRow] = &[
    (
        "rust parentheses",
        "rs",
        "fn f() {\n    (f)();\n    ((f::<u8>))();\n}\n",
        &[(2, "f", 0, Bound::To("f", 1)), (3, "f", 0, Bound::To("f", 1))],
    ),
    (
        "typescript wrappers",
        "ts",
        "function f() {\n  ((f))();\n  (f as () => void)();\n  (<any>f)();\n  f!();\n  (f satisfies any)();\n}\n",
        &[
            (2, "f", 0, Bound::To("f", 1)),
            (3, "f", 0, Bound::To("f", 1)),
            (4, "f", 0, Bound::To("f", 1)),
            (5, "f", 0, Bound::To("f", 1)),
            (6, "f", 0, Bound::To("f", 1)),
        ],
    ),
    (
        "typescript instantiation expressions",
        "ts",
        "function f<T>() {\n  (f<number>)();\n  ((f<number>))();\n  (f<number>)!();\n  (f<number> as () => void)();\n  (f<number> satisfies () => void)();\n  (<() => void>f<number>)();\n}\n",
        &[
            (2, "f", 0, Bound::To("f", 1)),
            (3, "f", 0, Bound::To("f", 1)),
            (4, "f", 0, Bound::To("f", 1)),
            (5, "f", 0, Bound::To("f", 1)),
            (6, "f", 0, Bound::To("f", 1)),
            (7, "f", 0, Bound::To("f", 1)),
        ],
    ),
    ("javascript parentheses", "js", "function f() {\n  ((f))();\n}\n", &[(2, "f", 0, Bound::To("f", 1))]),
];

/// A member instantiated apart from its call (`(obj.m<number>)()`) is one
/// receiver call: name-only over the member methods, with the receiver a
/// usage, and `new (C<number>)()` constructs the established class.
#[test]
fn typescript_instantiation_expressions_are_calls_of_their_operand() {
    let file = Analyzed::new(
        "class W { m<T>() {} }\nclass C<T> {}\nfunction g(obj: W) {\n  (obj.m<number>)();\n  new (C<number>)();\n}\n",
        "ts",
    );
    let member = file.record("W.m", 1).id.clone();
    let calls: Vec<_> = file.all("m").into_iter().filter(|occurrence| occurrence.kind == CALL).collect();
    assert_eq!(calls.len(), 1, "{calls:#?}");
    let call = calls[0];
    assert_eq!(call.span.start.line, 4);
    assert_eq!(call.confidence, Confidence::NameOnly);
    assert_eq!(call.target.id, None);
    let candidates: Vec<_> = call.candidates.iter().map(|candidate| candidate.id.as_ref()).collect();
    assert_eq!(candidates, [Some(&member)]);
    assert_eq!(file.at(4, "obj", 0).kind, USAGE);

    let class = file.named("C", 2).clone();
    let construct = file.at(5, "C", 0);
    assert_eq!(construct.kind, CALL);
    assert_scoped_to(construct, &class);
}

#[test]
fn wrapped_calls_keep_their_binding() {
    assert_bound_rows(WRAPPED_CALL_BINDINGS);
    for &(label, extension, source, _) in WRAPPED_CALL_BINDINGS {
        let file = Analyzed::new(source, extension);
        assert!(
            file.all("f").iter().filter(|occurrence| occurrence.span.start.line > 1).all(|occurrence| occurrence.kind == CALL),
            "{label}"
        );
    }
}

/// `(label, extension, source, owner name, the owner region's reasons)`;
/// no reasons means `Complete`.
type CalleeCoverageRow = (&'static str, &'static str, &'static str, &'static str, &'static [LimitReason]);

const INDIRECT: &[LimitReason] = &[LimitReason::IndirectDispatch];

/// Coverage reads a callee through the same transparent wrappers and generic
/// instantiations as invocation (`INVOCATION_MATRIX`), so a wrapped call to
/// an established declaration limits nothing; every other callee shape keeps
/// its limit.
const CALLEE_COVERAGE_MATRIX: &[CalleeCoverageRow] = &[
    ("rust direct (control)", "rs", "fn f() {}\nfn g() { f(); }\n", "g", &[]),
    ("rust parentheses", "rs", "fn f() {}\nfn g() { (f)(); ((f))(); }\n", "g", &[]),
    ("rust turbofish", "rs", "fn f<T>() {}\nfn g() { (f::<u8>)(); }\n", "g", &[]),
    ("rust type path", "rs", "struct S;\nimpl S { fn f() {} }\nfn g() { (S::f)(); }\n", "g", &[]),
    ("rust generic type path", "rs", "struct S;\nimpl S { fn f<T>() {} }\nfn g() { (S::f::<u8>)(); }\n", "g", &[]),
    ("rust dereference", "rs", "fn f() {}\nfn g() { let p = &f; (*p)(); }\n", "g", INDIRECT),
    ("rust variable", "rs", "fn f() {}\nfn g() { let h = f; (h)(); }\n", "g", INDIRECT),
    ("rust field", "rs", "struct S { h: fn() }\nfn g(s: S) { (s.h)(); }\n", "g", INDIRECT),
    ("rust computed", "rs", "fn f() {}\nfn g() { (if true { f } else { f })(); }\n", "g", INDIRECT),
    ("typescript direct (control)", "ts", "function f() {}\nfunction g() { f(); }\n", "g", &[]),
    ("typescript parentheses", "ts", "function f() {}\nfunction g() { (f)(); ((f))(); }\n", "g", &[]),
    ("typescript as", "ts", "function f() {}\nfunction g() { (f as (() => void))(); (f as any as () => void)(); }\n", "g", &[]),
    ("typescript satisfies", "ts", "function f() {}\nfunction g() { (f satisfies (() => void))(); }\n", "g", &[]),
    ("typescript type assertion", "ts", "function f() {}\nfunction g() { (<(() => void)>f)(); }\n", "g", &[]),
    ("typescript non-null", "ts", "function f() {}\nfunction g() { f!(); (f!)(); }\n", "g", &[]),
    ("typescript optional call", "ts", "function f() {}\nfunction g() { f?.(); (f)?.(); }\n", "g", &[]),
    ("typescript tagged template", "ts", "function f(_: TemplateStringsArray) {}\nfunction g() { (f)`x`; }\n", "g", &[]),
    ("typescript new", "ts", "class F {}\nfunction g() { new F(); new (F)(); new ((F))(); new (F as typeof F)(); }\n", "g", &[]),
    ("typescript member", "ts", "const o = { f() {} };\nfunction g() { o.f(); (o.f)(); }\n", "g", INDIRECT),
    ("typescript variable", "ts", "function f() {}\nfunction g() { const h = f; (h)(); }\n", "g", INDIRECT),
    ("typescript conditional", "ts", "function f() {}\nfunction g(c: boolean) { (c ? f : f)(); }\n", "g", INDIRECT),
    ("typescript new member", "ts", "const ns = { F: class {} };\nfunction g() { new (ns.F)(); }\n", "g", INDIRECT),
    ("typescript generic (control)", "ts", "function f<T>() {}\nfunction g() { f<number>(); }\n", "g", &[]),
    ("typescript instantiation", "ts", "function f<T>() {}\nfunction g() { (f<number>)(); ((f<number>))(); }\n", "g", &[]),
    ("typescript instantiation non-null", "ts", "function f<T>() {}\nfunction g() { (f<number>)!(); }\n", "g", &[]),
    ("typescript instantiation as", "ts", "function f<T>() {}\nfunction g() { (f<number> as () => void)(); }\n", "g", &[]),
    (
        "typescript instantiation satisfies",
        "ts",
        "function f<T>() {}\nfunction g() { (f<number> satisfies () => void)(); }\n",
        "g",
        &[],
    ),
    ("typescript instantiation assertion", "ts", "function f<T>() {}\nfunction g() { (<() => void>f<number>)(); }\n", "g", &[]),
    ("typescript new instantiation", "ts", "class C<T> {}\nfunction g() { new C<number>(); new (C<number>)(); }\n", "g", &[]),
    ("typescript member instantiation", "ts", "class W { m<T>() {} }\nfunction g(obj: W) { (obj.m<number>)(); }\n", "g", INDIRECT),
    ("typescript instantiation value", "ts", "function f<T>() {}\nfunction g() { const h = f<number>; }\n", "g", &[]),
    ("javascript direct (control)", "js", "function f() {}\nfunction g() { f(); }\n", "g", &[]),
    ("javascript parentheses", "js", "function f() {}\nfunction g() { (f)(); ((f))(); f?.(); (f)`x`; }\n", "g", &[]),
    ("javascript new", "js", "class F {}\nfunction g() { new F(); new (F)(); new ((F))(); }\n", "g", &[]),
    ("javascript sequence", "js", "function f() {}\nfunction g() { (0, f)(); }\n", "g", INDIRECT),
    ("javascript new sequence", "js", "class F {}\nfunction g() { new (0, F)(); }\n", "g", INDIRECT),
    (
        "javascript parenthesized eval",
        "js",
        "function g() { (eval)(\"1\"); }\n",
        "g",
        &[LimitReason::UnsupportedBindingForm, LimitReason::IndirectDispatch],
    ),
    ("javascript indirect eval", "js", "function g() { (0, eval)(\"1\"); }\n", "g", INDIRECT),
    ("javascript optional eval", "js", "function g() { eval?.(\"1\"); (eval)?.(\"1\"); }\n", "g", INDIRECT),
    ("javascript tagged eval", "js", "function g() { eval`1`; (eval)`1`; }\n", "g", &[]),
    (
        "typescript parenthesized eval",
        "ts",
        "function g() { ((eval))(\"1\"); }\n",
        "g",
        &[LimitReason::UnsupportedBindingForm, LimitReason::IndirectDispatch],
    ),
    ("typescript optional eval", "ts", "function g() { eval?.(\"1\"); (eval as any)?.(\"1\"); }\n", "g", INDIRECT),
    ("typescript tagged eval", "ts", "function g() { eval`1`; (eval!)`1`; }\n", "g", &[]),
    ("typescript let callback", "ts", "function f() {}\nlet cb = f;\nfunction g() { cb(); }\n", "g", INDIRECT),
    ("typescript var callback", "ts", "function f() {}\nvar cb = f;\nfunction g() { cb(); }\n", "g", INDIRECT),
    ("typescript const value", "ts", "function f() {}\nconst cb = f;\nfunction g() { cb(); }\n", "g", INDIRECT),
    ("typescript const function expression", "ts", "const cb = () => {};\nfunction g() { cb(); }\n", "g", &[]),
    ("javascript let callback", "js", "function f() {}\nlet cb = f;\nfunction g() { cb(); }\n", "g", INDIRECT),
    ("javascript var callback", "js", "function f() {}\nvar cb = f;\nfunction g() { cb(); }\n", "g", INDIRECT),
    ("javascript const value", "js", "function f() {}\nconst cb = f;\nfunction g() { cb(); }\n", "g", INDIRECT),
    ("javascript const function expression", "js", "const cb = () => {};\nfunction g() { cb(); }\n", "g", &[]),
];

#[test]
fn wrapped_callees_limit_coverage_exactly_like_their_operand() {
    for &(label, extension, source, owner, reasons) in CALLEE_COVERAGE_MATRIX {
        let file = Analyzed::new(source, extension);
        let region = file
            .evidence
            .reference_coverage
            .iter()
            .find(|coverage| coverage.owner.as_ref().is_some_and(|region_owner| region_owner.name == owner))
            .unwrap_or_else(|| panic!("{label}: no `{owner}` region"));
        let state = if reasons.is_empty() { CoverageState::Complete } else { CoverageState::Limited };
        assert_eq!((region.state, region.reasons.as_slice()), (state, reasons), "{label}");
    }
}

/// How a qualified Rust name is bound in [`VALUE_CALL_MATRIX`].
#[derive(Clone, Copy)]
enum ValueBinding {
    /// `Scoped` to the record called `name` whose name starts on `line`.
    To(&'static str, u32),
    /// Never `Scoped`, with exactly the records of the name starting on
    /// these lines as candidates.
    Candidates(&'static [u32]),
}

/// `(label, source, [(line, name, nth on that line, kind, binding)], owner
/// name, the owner region's reasons)`.
type ValueCallRow = (
    &'static str,
    &'static str,
    &'static [(u32, &'static str, usize, OccurrenceKind, ValueBinding)],
    &'static str,
    &'static [LimitReason],
);

/// Calling a Rust value (a `const` or `static` holding a function pointer)
/// binds the value, never the function its initializer names, and limits
/// the caller with `indirect_dispatch`, whether the value is established or
/// only a candidate. Calls of functions and constructors, and reading a
/// value without calling it, limit nothing. Every source compiles with
/// `rustc --edition 2024`.
const VALUE_CALL_MATRIX: &[ValueCallRow] = &[
    (
        "inherent `Self::C()`",
        "struct S;\nimpl S {\n    const C: fn() = Self::f;\n    fn f() { Self::C(); }\n}\n",
        &[(4, "C", 0, CALL, ValueBinding::To("C", 3))],
        "f",
        INDIRECT,
    ),
    (
        "inherent `S::C()`",
        "struct S;\nimpl S {\n    const C: fn() = S::f;\n    fn f() { S::C(); }\n}\n",
        &[(4, "C", 0, CALL, ValueBinding::To("C", 3))],
        "f",
        INDIRECT,
    ),
    (
        "wrapped `(Self::C)()` and `((Self::C))()`",
        "struct S;\nimpl S {\n    const C: fn() = Self::f;\n    fn f() { (Self::C)(); ((Self::C))(); }\n}\n",
        &[(4, "C", 0, CALL, ValueBinding::To("C", 3)), (4, "C", 1, CALL, ValueBinding::To("C", 3))],
        "f",
        INDIRECT,
    ),
    (
        "wrapped `(S::C)()`",
        "struct S;\nimpl S {\n    const C: fn() = Self::f;\n    fn f() { (S::C)(); }\n}\n",
        &[(4, "C", 0, CALL, ValueBinding::To("C", 3))],
        "f",
        INDIRECT,
    ),
    (
        "generic impl `Self::C()`",
        "struct S<T>(T);\nimpl<T> S<T> {\n    const C: fn() = Self::f;\n    fn f() { Self::C(); }\n}\n",
        &[(4, "C", 0, CALL, ValueBinding::To("C", 3))],
        "f",
        INDIRECT,
    ),
    (
        "generic qualifier `S::<u8>::C()` keeps its candidate",
        "struct S<T>(T);\nimpl<T> S<T> {\n    const C: fn() = Self::f;\n    fn f() { S::<u8>::C(); }\n}\n",
        &[(4, "C", 0, CALL, ValueBinding::Candidates(&[3]))],
        "f",
        INDIRECT,
    ),
    (
        "enum inherent `Self::C()`",
        "enum E { A }\nimpl E {\n    const C: fn() = Self::f;\n    fn f() { Self::C(); }\n}\n",
        &[(4, "C", 0, CALL, ValueBinding::To("C", 3))],
        "f",
        INDIRECT,
    ),
    (
        "trait `Self::C()` keeps its candidates",
        "trait T {\n    const C: fn();\n    fn f() { Self::C(); }\n}\nstruct S;\nimpl T for S { const C: fn() = Self::f; }\n",
        &[(3, "C", 0, CALL, ValueBinding::Candidates(&[2, 6]))],
        "f",
        INDIRECT,
    ),
    (
        "trait method `S::C()` keeps its candidates",
        "trait T {\n    const C: fn();\n    fn f() { S::C(); }\n}\nstruct S;\nimpl T for S { const C: fn() = Self::f; }\n",
        &[(3, "C", 0, CALL, ValueBinding::Candidates(&[2, 6]))],
        "f",
        INDIRECT,
    ),
    (
        "an unrelated initializer is not followed",
        "fn h() {}\nstruct S;\nimpl S {\n    const C: fn() = h;\n    fn f() { Self::C(); }\n}\n",
        &[(5, "C", 0, CALL, ValueBinding::To("C", 4))],
        "f",
        INDIRECT,
    ),
    (
        "top-level `const` (control)",
        "const C: fn() = f;\nfn f() { C(); }\n",
        &[(2, "C", 0, CALL, ValueBinding::To("C", 1))],
        "f",
        INDIRECT,
    ),
    (
        "top-level `static` (control)",
        "static C: fn() = f;\nfn f() { C(); }\n",
        &[(2, "C", 0, CALL, ValueBinding::To("C", 1))],
        "f",
        INDIRECT,
    ),
    (
        "a member function (control)",
        "struct S;\nimpl S {\n    fn h() {}\n    fn f() { Self::h(); S::h(); }\n}\n",
        &[(4, "h", 0, CALL, ValueBinding::To("h", 3)), (4, "h", 1, CALL, ValueBinding::To("h", 3))],
        "f",
        &[],
    ),
    (
        "a variant constructor (control)",
        "enum E { V(u8) }\nfn f() { let _ = E::V(1); }\n",
        &[(2, "V", 0, CALL, ValueBinding::To("V", 1))],
        "f",
        &[],
    ),
    (
        "reading the value is not calling it (control)",
        "struct S;\nimpl S {\n    const C: fn() = Self::f;\n    fn f() { let _p = Self::C; }\n}\n",
        &[(4, "C", 0, USAGE, ValueBinding::To("C", 3))],
        "f",
        &[],
    ),
];

#[test]
fn qualified_value_calls_bind_the_value_and_limit_their_caller() {
    for &(label, source, checks, owner, reasons) in VALUE_CALL_MATRIX {
        let file = Analyzed::new(source, "rs");
        for &(line, name, nth, kind, binding) in checks {
            let occurrence = file.at(line, name, nth);
            assert_eq!(occurrence.kind, kind, "{label}: {occurrence:#?}");
            match binding {
                ValueBinding::To(target, target_line) => {
                    assert_eq!(occurrence.confidence, Confidence::Scoped, "{label}: {occurrence:#?}");
                    assert_eq!(occurrence.target.id.as_ref(), Some(file.named(target, target_line)), "{label}");
                }
                ValueBinding::Candidates(lines) => {
                    assert_ne!(occurrence.confidence, Confidence::Scoped, "{label}: {occurrence:#?}");
                    assert_eq!(occurrence.target.id, None, "{label}: {occurrence:#?}");
                    let expected: Vec<_> = lines.iter().map(|&line| Some(file.named(name, line))).collect();
                    let candidates: Vec<_> = occurrence.candidates.iter().map(|candidate| candidate.id.as_ref()).collect();
                    assert_eq!(candidates, expected, "{label}");
                }
            }
        }
        let region = file
            .evidence
            .reference_coverage
            .iter()
            .find(|coverage| coverage.owner.as_ref().is_some_and(|region_owner| region_owner.name == owner))
            .unwrap_or_else(|| panic!("{label}: no `{owner}` region"));
        // The call is never credited to the function an initializer names.
        let caller = region.owner.as_ref().and_then(|region_owner| region_owner.id.as_ref());
        assert!(caller.is_some(), "{label}: `{owner}` has an ID");
        assert!(
            file.evidence
                .occurrences
                .iter()
                .filter(|occurrence| occurrence.owner.as_ref().and_then(|owner| owner.id.as_ref()) == caller)
                .all(|occurrence| occurrence.target.id.as_ref() != caller),
            "{label}: a reference to `{owner}` was manufactured"
        );
        let state = if reasons.is_empty() { CoverageState::Complete } else { CoverageState::Limited };
        assert_eq!((region.state, region.reasons.as_slice()), (state, reasons), "{label}");
    }
}

/// A C++ template call (`f<int>()`, `obj.template m<int>()`) is a use, not a
/// declaration; an explicit specialization (`template <> void f<int>()`)
/// still declares `f`.
#[test]
fn cpp_template_calls_are_calls_not_declarations() {
    let file = Analyzed::new(
        "template <class T> void f() {}\nvoid g() { f<int>(); obj.template m<int>(); }\ntemplate <> void f<long>() {}\n",
        "cpp",
    );
    let declared = |name: &str| {
        file.index
            .symbols
            .iter()
            .filter(|record| record.identity.name == name)
            .filter_map(|record| record.source.name_span.as_ref().map(|span| span.start.line))
            .collect::<Vec<_>>()
    };
    assert_eq!(declared("f"), [1, 3]);
    assert!(declared("m").is_empty());
    assert_eq!(file.at(2, "f", 0).kind, CALL);
    assert_eq!(file.at(2, "m", 0).kind, CALL);
}

// ---- Written paths ----------------------------------------------------------

/// `(label, language, extension, source, [(line, name, nth on that line,
/// written path, kind)])`.
type WrittenPathRow = (
    &'static str,
    ProgrammingLanguage,
    &'static str,
    &'static str,
    &'static [(u32, &'static str, usize, Option<&'static str>, OccurrenceKind)],
);

/// Every path shape each pinned grammar writes around a captured name, with
/// the path known independently from the source: the bytes from the
/// outermost qualifier, root or receiver through the name token. Type
/// arguments after the name are outside it; those inside a qualifier stay.
/// The `R4-F1.nn` labels are the review rows (`.17`–`.28` the controls).
const WRITTEN_PATH_MATRIX: &[WrittenPathRow] = &[
    (
        "R4-F1.01 rust rooted names",
        ProgrammingLanguage::Rust,
        "rs",
        "fn f() {\n    ::f();\n    (::f)();\n    ::m::f();\n}\nfn g() { let f = || {}; ::f(); }\n",
        &[
            (2, "f", 0, Some("::f"), CALL),
            (3, "f", 0, Some("::f"), CALL),
            (4, "m", 0, Some("::m"), USAGE),
            (4, "f", 0, Some("::m::f"), CALL),
            (6, "f", 0, Some("::f"), CALL),
        ],
    ),
    (
        "R4-F1.02 c++ global root at every depth",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    ::f();\n    (::f)();\n    ((::f))();\n}\n",
        &[(2, "f", 0, Some("::f"), CALL), (3, "f", 0, Some("::f"), CALL), (4, "f", 0, Some("::f"), CALL)],
    ),
    (
        "R4-F1.03 c++ right-nested namespaces (with the R4-F1.28 `ns::f` control)",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    A::B::f();\n    ns::f();\n}\n",
        &[
            (2, "A", 0, None, USAGE),
            (2, "B", 0, Some("A::B"), USAGE),
            (2, "f", 0, Some("A::B::f"), CALL),
            (3, "f", 0, Some("ns::f"), CALL),
        ],
    ),
    (
        "R4-F1.04 c++ generic namespace function",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    r4::f<int>();\n    (r4::f<int>)();\n    auto x = r4::f<int>;\n}\n",
        &[(2, "f", 0, Some("r4::f"), CALL), (3, "f", 0, Some("r4::f"), CALL), (4, "f", 0, Some("r4::f"), USAGE)],
    ),
    (
        "R4-F1.05/.06/.07 c++ generic, dependent and pointer members",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    obj.m<int>();\n    (obj.m<int>)();\n    obj.m();\n    obj.template m<int>();\n    (obj.template m<int>)();\n    p->template m<int>();\n    p->m();\n}\n",
        &[
            (2, "m", 0, Some("obj.m"), CALL),
            (3, "m", 0, Some("obj.m"), CALL),
            (4, "m", 0, Some("obj.m"), CALL),
            (5, "m", 0, Some("obj.template m"), CALL),
            (6, "m", 0, Some("obj.template m"), CALL),
            (7, "m", 0, Some("p->template m"), CALL),
            (8, "m", 0, Some("p->m"), CALL),
        ],
    ),
    (
        "R4-F1.08/.09 c++ generic types and generic scopes",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    r4::C<int> v;\n    r4::C<int>();\n    N::C<int>::member();\n    A::B::C<int>::m();\n}\n",
        &[
            (2, "C", 0, Some("r4::C"), USAGE),
            (3, "C", 0, Some("r4::C"), CALL),
            (4, "C", 0, Some("N::C"), USAGE),
            (4, "member", 0, Some("N::C<int>::member"), CALL),
            (5, "B", 0, Some("A::B"), USAGE),
            (5, "C", 0, Some("A::B::C"), USAGE),
            (5, "m", 0, Some("A::B::C<int>::m"), CALL),
        ],
    ),
    (
        "R4-F1.10–.13 c# generic, alias-qualified and conditional names (with R4-F1.28 controls)",
        ProgrammingLanguage::CSharp,
        "cs",
        "class G { void g(W obj) {\n    obj.M<int>();\n    (obj.M<int>)();\n    N.C<int> c = new N.C<int>();\n    var d = new global::Example.Symbols.R4W<int>();\n    X::C e;\n    global::N.C h;\n    obj?.F();\n    obj?.M<int>();\n    obj?.A();\n    obj?.A?.Invoke();\n    var z = obj?.A;\n    obj.A();\n    System.Console.WriteLine();\n} }\n",
        &[
            (2, "M", 0, Some("obj.M"), CALL),
            (3, "M", 0, Some("obj.M"), CALL),
            (4, "C", 0, Some("N.C"), USAGE),
            (4, "C", 1, Some("N.C"), CALL),
            (5, "Example", 0, Some("global::Example"), USAGE),
            (5, "Symbols", 0, Some("global::Example.Symbols"), USAGE),
            (5, "R4W", 0, Some("global::Example.Symbols.R4W"), CALL),
            (6, "X", 0, None, USAGE),
            (6, "C", 0, Some("X::C"), USAGE),
            (7, "N", 0, Some("global::N"), USAGE),
            (7, "C", 0, Some("global::N.C"), USAGE),
            (8, "obj", 0, None, USAGE),
            (8, "F", 0, Some("obj?.F"), CALL),
            (9, "M", 0, Some("obj?.M"), CALL),
            (10, "A", 0, Some("obj?.A"), CALL),
            (11, "A", 0, Some("obj?.A"), USAGE),
            (11, "Invoke", 0, Some("obj?.A?.Invoke"), CALL),
            (12, "A", 0, Some("obj?.A"), USAGE),
            (13, "A", 0, Some("obj.A"), CALL),
            (14, "Console", 0, Some("System.Console"), USAGE),
            (14, "WriteLine", 0, Some("System.Console.WriteLine"), CALL),
        ],
    ),
    (
        "R4-F1.14 php nullsafe members (with R4-F1.28 controls and namespace segments)",
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction g($o) {\n    $o?->m();\n    ($o)?->m();\n    $x = $o?->p;\n    $y = ($o)?->p;\n    $o->m();\n    $o->p;\n    W::m();\n    \\Ns\\f();\n    \\A\\B\\f();\n    A\\B\\f();\n}\n",
        &[
            (3, "m", 0, Some("$o?->m"), CALL),
            (4, "m", 0, Some("($o)?->m"), CALL),
            (5, "p", 0, Some("$o?->p"), USAGE),
            (6, "p", 0, Some("($o)?->p"), USAGE),
            (7, "m", 0, Some("$o->m"), CALL),
            (8, "p", 0, Some("$o->p"), USAGE),
            (9, "m", 0, Some("W::m"), CALL),
            (10, "Ns", 0, Some("\\Ns"), USAGE),
            (10, "f", 0, Some("\\Ns\\f"), CALL),
            (11, "A", 0, Some("\\A"), USAGE),
            (11, "B", 0, Some("\\A\\B"), USAGE),
            (11, "f", 0, Some("\\A\\B\\f"), CALL),
            (12, "A", 0, None, USAGE),
            (12, "B", 0, Some("A\\B"), USAGE),
            (12, "f", 0, Some("A\\B\\f"), CALL),
        ],
    ),
    (
        "R4-F1.15/.16 perl package and receiver paths",
        ProgrammingLanguage::Perl,
        "pl",
        "R4::r4f();\n&R4::r4f();\nmy $r = \\&R4::r4f;\nR4->r4f();\n$obj->r4f();\n",
        &[
            (1, "r4f", 0, Some("R4::r4f"), CALL),
            (2, "r4f", 0, Some("R4::r4f"), CALL),
            (3, "r4f", 0, Some("R4::r4f"), USAGE),
            (4, "r4f", 0, Some("R4->r4f"), CALL),
            (5, "$obj", 0, None, USAGE),
            (5, "r4f", 0, Some("$obj->r4f"), CALL),
        ],
    ),
    (
        "R4-F1.17 rust qualified generic control",
        ProgrammingLanguage::Rust,
        "rs",
        "mod m { pub fn f<T>() {} }\nstruct W;\nimpl W { fn h<T>() {} fn k() {\n    Self::h::<u8>();\n} }\nfn g() {\n    m::f::<u8>();\n    (m::f::<u8>)();\n}\n",
        &[
            (4, "h", 0, Some("Self::h"), CALL),
            (7, "m", 0, None, USAGE),
            (7, "f", 0, Some("m::f"), CALL),
            (8, "f", 0, Some("m::f"), CALL),
        ],
    ),
    (
        "R4-F1.18 typescript control",
        ProgrammingLanguage::TypeScript,
        "ts",
        "function f() {\n  obj.m<number>();\n  (obj.m<number>)();\n  obj?.m<number>();\n  A.B.f();\n  let x: A.B<string>;\n}\n",
        &[
            (2, "m", 0, Some("obj.m"), CALL),
            (3, "m", 0, Some("obj.m"), CALL),
            (4, "m", 0, Some("obj?.m"), CALL),
            (5, "f", 0, Some("A.B.f"), CALL),
            (6, "B", 0, Some("A.B"), USAGE),
        ],
    ),
    (
        "R4-F1.19 javascript control",
        ProgrammingLanguage::JavaScript,
        "js",
        "function f() {\n  obj.m();\n  (obj.m)();\n  obj?.m();\n  A.B.f();\n}\n",
        &[
            (2, "m", 0, Some("obj.m"), CALL),
            (3, "m", 0, Some("obj.m"), CALL),
            (4, "m", 0, Some("obj?.m"), CALL),
            (5, "f", 0, Some("A.B.f"), CALL),
        ],
    ),
    (
        "R4-F1.20 java control and field-less type paths",
        ProgrammingLanguage::Java,
        "java",
        "class K { void f() {\n    obj.<String>m();\n    obj.m();\n    a.b.C x = new a.b.C();\n    java.util.List<String> l;\n} }\n",
        &[
            (2, "m", 0, Some("obj.<String>m"), CALL),
            (3, "m", 0, Some("obj.m"), CALL),
            (4, "b", 0, Some("a.b"), USAGE),
            (4, "C", 0, Some("a.b.C"), USAGE),
            (4, "C", 1, Some("a.b.C"), CALL),
            (5, "List", 0, Some("java.util.List"), USAGE),
        ],
    ),
    (
        "R4-F1.21 go control and package-qualified types",
        ProgrammingLanguage::Go,
        "go",
        "package p\n\nvar x pkg.T\nvar y pkg.G[int]\n\nfunc f() {\n\tpkg.F()\n}\n",
        &[
            (3, "T", 0, Some("pkg.T"), USAGE),
            (4, "G", 0, Some("pkg.G"), USAGE),
            (7, "pkg", 0, None, USAGE),
            (7, "F", 0, Some("pkg.F"), CALL),
        ],
    ),
    (
        "R4-F1.22 c control",
        ProgrammingLanguage::C,
        "c",
        "void f() {\n    obj.f();\n    p->f();\n}\n",
        &[(2, "f", 0, Some("obj.f"), CALL), (3, "f", 0, Some("p->f"), CALL)],
    ),
    (
        "R4-F1.23 python control",
        ProgrammingLanguage::Python,
        "py",
        "def f():\n    obj.f()\n    A.B.f()\n    x = A.B.f\n",
        &[
            (2, "f", 0, Some("obj.f"), CALL),
            (3, "B", 0, Some("A.B"), USAGE),
            (3, "f", 0, Some("A.B.f"), CALL),
            (4, "f", 0, Some("A.B.f"), USAGE),
        ],
    ),
    (
        "R4-F1.24 swift control and field-less type paths",
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    W.f(1)\n    obj?.f()\n    obj!.f()\n    A.B.f()\n    let y: Swift.Array<Int> = []\n}\n",
        &[
            (2, "f", 0, Some("W.f"), CALL),
            (3, "f", 0, Some("obj?.f"), CALL),
            (4, "f", 0, Some("obj!.f"), CALL),
            (5, "B", 0, Some("A.B"), USAGE),
            (5, "f", 0, Some("A.B.f"), CALL),
            (6, "Array", 0, Some("Swift.Array"), USAGE),
        ],
    ),
    (
        "R4-F1.25 scala control and field-less type paths",
        ProgrammingLanguage::Scala,
        "scala",
        "object O {\n  def g(): Unit = {\n    W.f[Int]()\n    (W.f[Int])()\n    val h = W.f\n    val x: a.b.C = new a.b.C()\n  }\n}\n",
        &[
            (3, "f", 0, Some("W.f"), CALL),
            (4, "f", 0, Some("W.f"), CALL),
            (5, "f", 0, Some("W.f"), USAGE),
            (6, "b", 0, Some("a.b"), USAGE),
            (6, "C", 0, Some("a.b.C"), USAGE),
            (6, "C", 1, Some("a.b.C"), CALL),
        ],
    ),
    (
        "R4-F1.26 lua control",
        ProgrammingLanguage::Lua,
        "lua",
        "local function g()\n  W.f()\n  W:f()\n  A.B.f()\nend\n",
        &[
            (2, "f", 0, Some("W.f"), CALL),
            (3, "f", 0, Some("W:f"), CALL),
            (4, "B", 0, Some("A.B"), USAGE),
            (4, "f", 0, Some("A.B.f"), CALL),
        ],
    ),
    (
        "R5-F1.01/.03/.04 java constructed generic types keep their written path",
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        new a.b.C<String>();\n        new a.b.C<String>.D<Integer>();\n        new @A a.b.C<>();\n        new C<String>();\n    }\n}\n",
        &[
            (3, "b", 0, Some("a.b"), USAGE),
            (3, "C", 0, Some("a.b.C"), CALL),
            (4, "C", 0, Some("a.b.C"), USAGE),
            (4, "D", 0, Some("a.b.C<String>.D"), CALL),
            (5, "C", 0, Some("a.b.C"), CALL),
            (6, "C", 0, None, CALL),
        ],
    ),
    (
        "R5-F1.05/.06 scala constructed generic types keep their written path",
        ProgrammingLanguage::Scala,
        "scala",
        "object G {\n  def g(): Unit = {\n    new a.b.C[Int]()\n    new a.b.C[Int] {}\n    new C[Int]\n  }\n}\n",
        &[(3, "C", 0, Some("a.b.C"), CALL), (4, "C", 0, Some("a.b.C"), CALL), (5, "C", 0, None, CALL)],
    ),
    (
        "R5-F1.07/.08 swift constructed types and implicit members",
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = N.C<Int>()\n    _ = C<Int>()\n    let _: C = .f()\n}\n",
        &[(2, "C", 0, Some("N.C"), CALL), (3, "C", 0, None, CALL), (4, "f", 0, None, CALL)],
    ),
    (
        "R5-F1.10/.11 c++ allocated and braced constructions",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    new N::C<int>();\n    N::Plain{};\n    N::C<int>{};\n    new C<int>{};\n}\n",
        &[
            (2, "C", 0, Some("N::C"), CALL),
            (3, "Plain", 0, Some("N::Plain"), CALL),
            (4, "C", 0, Some("N::C"), CALL),
            (5, "C", 0, None, CALL),
        ],
    ),
    (
        "R5-F3.01/.02/.03 java simple, parenthesized and `this` receivers before `new`",
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        outer.new C();\n        (outer).new C();\n        this.new C();\n    }\n}\n",
        &[
            (3, "outer", 0, None, USAGE),
            (3, "C", 0, Some("outer.new C"), CALL),
            (4, "outer", 0, None, USAGE),
            (4, "C", 0, Some("(outer).new C"), CALL),
            (5, "C", 0, Some("this.new C"), CALL),
        ],
    ),
    (
        "R5-F3.04/.05 java evaluated and chained receivers before `new`",
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        factory().new C();\n        holder.outer.new C();\n        a.new B().new C();\n    }\n}\n",
        &[
            (3, "factory", 0, None, CALL),
            (3, "C", 0, Some("factory().new C"), CALL),
            (4, "holder", 0, None, USAGE),
            (4, "outer", 0, Some("holder.outer"), USAGE),
            (4, "C", 0, Some("holder.outer.new C"), CALL),
            (5, "a", 0, None, USAGE),
            (5, "B", 0, Some("a.new B"), CALL),
            (5, "C", 0, Some("a.new B().new C"), CALL),
        ],
    ),
    (
        "R5-F3.06/.07 java generic receivers and an anonymous body stay outside the path",
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        outer.new C<String>();\n        (outer).new C<String>();\n        outer.new C() { void m() { x.y(); } };\n        outer.new C(arg.v);\n    }\n}\n",
        &[
            (3, "C", 0, Some("outer.new C"), CALL),
            (3, "String", 0, None, USAGE),
            (4, "C", 0, Some("(outer).new C"), CALL),
            (4, "String", 0, None, USAGE),
            (5, "C", 0, Some("outer.new C"), CALL),
            (5, "y", 0, Some("x.y"), CALL),
            (6, "C", 0, Some("outer.new C"), CALL),
            (6, "v", 0, Some("arg.v"), USAGE),
        ],
    ),
    (
        "R5-F3.08 java ordinary construction paths are unchanged",
        ProgrammingLanguage::Java,
        "java",
        "class G {\n    void g() {\n        new N.C();\n        new a.b.C<String>();\n        new a.b.C<String>.D<Integer>();\n        new C();\n    }\n}\n",
        &[
            (3, "C", 0, Some("N.C"), CALL),
            (4, "b", 0, Some("a.b"), USAGE),
            (4, "C", 0, Some("a.b.C"), CALL),
            (5, "C", 0, Some("a.b.C"), USAGE),
            (5, "D", 0, Some("a.b.C<String>.D"), CALL),
            (6, "C", 0, None, CALL),
        ],
    ),
    (
        "R5-F3.09 c++ constructed paths are unchanged",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void g() {\n    new N::C();\n    new C();\n    N::C<int>{};\n}\n",
        &[(2, "C", 0, Some("N::C"), CALL), (3, "C", 0, None, CALL), (4, "C", 0, Some("N::C"), CALL)],
    ),
    (
        "R5-F3.09 rust constructed paths are unchanged",
        ProgrammingLanguage::Rust,
        "rs",
        "fn g() {\n    N::C::new();\n    N::C { x: 1 };\n    C { x: 1 };\n}\n",
        &[
            (2, "C", 0, Some("N::C"), USAGE),
            (2, "new", 0, Some("N::C::new"), CALL),
            (3, "C", 0, Some("N::C"), USAGE),
            (4, "C", 0, None, USAGE),
        ],
    ),
    (
        "R5-F3.09 scala constructed paths are unchanged",
        ProgrammingLanguage::Scala,
        "scala",
        "object O {\n  def g(): Unit = {\n    new a.b.C[Int]()\n    new C()\n    outer.C()\n  }\n}\n",
        &[(3, "C", 0, Some("a.b.C"), CALL), (4, "C", 0, None, CALL), (5, "C", 0, Some("outer.C"), CALL)],
    ),
    (
        "R5-F3.09 typescript constructed paths are unchanged",
        ProgrammingLanguage::TypeScript,
        "ts",
        "function g() {\n  new N.C();\n  new C();\n  new N.C<number>();\n}\n",
        &[(2, "C", 0, Some("N.C"), CALL), (3, "C", 0, None, CALL), (4, "C", 0, Some("N.C"), CALL)],
    ),
    (
        "R5-F3.09 c# constructed paths are unchanged",
        ProgrammingLanguage::CSharp,
        "cs",
        "class G { void g() {\n    new N.C();\n    new C();\n    new N.C<int>();\n} }\n",
        &[(2, "C", 0, Some("N.C"), CALL), (3, "C", 0, None, CALL), (4, "C", 0, Some("N.C"), CALL)],
    ),
    (
        "R5-F3.09 php constructed paths are unchanged",
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction g() {\n    new \\N\\C();\n    new C();\n    new $c();\n}\n",
        &[(3, "C", 0, Some("\\N\\C"), CALL), (4, "C", 0, None, CALL), (5, "c", 0, None, CALL)],
    ),
    (
        "R5-F3.10 go composite literals keep their paths",
        ProgrammingLanguage::Go,
        "go",
        "package p\nfunc g() {\n    _ = pkg.C{}\n    _ = C{}\n}\n",
        &[(3, "C", 0, Some("pkg.C"), USAGE), (4, "C", 0, None, USAGE)],
    ),
    (
        "R5-F3.10 python qualified callees keep their paths",
        ProgrammingLanguage::Python,
        "py",
        "def g():\n    a.b.C()\n    C()\n",
        &[(2, "b", 0, Some("a.b"), USAGE), (2, "C", 0, Some("a.b.C"), CALL), (3, "C", 0, None, CALL)],
    ),
    (
        "R5-F3.S1 swift prefix operators apply to the whole member chain",
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = !x.isEmpty()\n    _ = -x.f()\n    _ = !x.y.f()\n    _ = !x?.f()\n    _ = !f()\n    _ = !-x.f()\n    _ = !x.isEmpty\n}\n",
        &[
            (2, "x", 0, None, USAGE),
            (2, "isEmpty", 0, Some("x.isEmpty"), CALL),
            (3, "f", 0, Some("x.f"), CALL),
            (4, "y", 0, Some("x.y"), USAGE),
            (4, "f", 0, Some("x.y.f"), CALL),
            (5, "f", 0, Some("x?.f"), CALL),
            (6, "f", 0, None, CALL),
            (7, "f", 0, Some("x.f"), CALL),
            (8, "isEmpty", 0, Some("x.isEmpty"), USAGE),
        ],
    ),
    (
        "R5-F3.S2 swift operators before calls, subscripts, postfix forms, `self` and `&`",
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = !x.f().g()\n    _ = !a[0].f()\n    _ = !x!.f()\n    _ = !self.f()\n    h(&x.f)\n}\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (2, "g", 0, Some("x.f().g"), CALL),
            (3, "f", 0, Some("a[0].f"), CALL),
            (4, "f", 0, Some("x!.f"), CALL),
            (5, "f", 0, Some("self.f"), CALL),
            (6, "f", 0, Some("x.f"), USAGE),
        ],
    ),
    (
        "R5-F3.S3 swift operators before implicit members; written parentheses stay",
        ProgrammingLanguage::Swift,
        "swift",
        "func g() {\n    _ = -.some.f()\n    _ = .some.f()\n    _ = !.a\n    _ = -.f()\n    _ = (!x).f()\n    _ = !(x.f())\n}\n",
        &[
            (2, "some", 0, None, USAGE),
            (2, "f", 0, Some(".some.f"), CALL),
            (3, "some", 0, None, USAGE),
            (3, "f", 0, Some(".some.f"), CALL),
            (4, "a", 0, None, USAGE),
            (5, "f", 0, None, CALL),
            (6, "f", 0, Some("(!x).f"), CALL),
            (7, "f", 0, Some("x.f"), CALL),
        ],
    ),
    (
        "R5-F3.S4 swift `try` and `await` stay outside the path (clean)",
        ProgrammingLanguage::Swift,
        "swift",
        "func g() async throws {\n    _ = try x.f()\n    _ = await x.f()\n}\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("x.f"), CALL),
        ],
    ),
    (
        "R5-F3.S5 rust prefix operators, `.await`, `?` and casts stay outside the path (clean)",
        ProgrammingLanguage::Rust,
        "rs",
        "fn g() {\n    !x.f();\n    -x.f();\n    *x.f();\n    &mut x.f();\n    x.f().await;\n    x.f()?;\n    !W::f();\n    (x as T).f();\n}\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("x.f"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
            (5, "f", 0, Some("x.f"), CALL),
            (6, "f", 0, Some("x.f"), CALL),
            (7, "f", 0, Some("x.f"), CALL),
            (8, "f", 0, Some("W::f"), CALL),
            (9, "f", 0, Some("(x as T).f"), CALL),
        ],
    ),
    (
        "R5-F3.S6 typescript prefix operators, `await` and type assertions stay outside the path (clean)",
        ProgrammingLanguage::TypeScript,
        "ts",
        "async function g() {\n  !x.f();\n  -x.f();\n  await x.f();\n  typeof x.f();\n  void x.f();\n  (<T>x).f();\n  (x as T).f();\n}\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("x.f"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
            (5, "f", 0, Some("x.f"), CALL),
            (6, "f", 0, Some("x.f"), CALL),
            (7, "f", 0, Some("(<T>x).f"), CALL),
            (8, "f", 0, Some("(x as T).f"), CALL),
        ],
    ),
    (
        "R5-F3.S6 javascript prefix operators and `await` stay outside the path (clean)",
        ProgrammingLanguage::JavaScript,
        "js",
        "async function g() {\n  !x.f();\n  -x.f();\n  await x.f();\n  typeof x.f();\n}\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("x.f"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
            (5, "f", 0, Some("x.f"), CALL),
        ],
    ),
    (
        "R5-F3.S7 c# prefix operators, `await` and casts stay outside the path (clean)",
        ProgrammingLanguage::CSharp,
        "cs",
        "class G { async void g() {\n    !x.F();\n    -x.F();\n    await x.F();\n    ((T)x).F();\n    (T)x.F();\n    x!.F();\n} }\n",
        &[
            (2, "F", 0, Some("x.F"), CALL),
            (3, "F", 0, Some("x.F"), CALL),
            (4, "F", 0, Some("x.F"), CALL),
            (5, "F", 0, Some("((T)x).F"), CALL),
            (6, "F", 0, Some("x.F"), CALL),
            (7, "F", 0, Some("x!.F"), CALL),
        ],
    ),
    (
        "R5-F3.S8 java prefix operators and casts stay outside the path (clean)",
        ProgrammingLanguage::Java,
        "java",
        "class G { void g() {\n    boolean a = !x.f();\n    int b = -x.f();\n    Object c = (T) x.f();\n    ((T) x).f();\n} }\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("x.f"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
            (5, "f", 0, Some("((T) x).f"), CALL),
        ],
    ),
    (
        "R5-F3.S9 python `not`, unary operators and `await` stay outside the path (clean)",
        ProgrammingLanguage::Python,
        "py",
        "async def g():\n    not x.f()\n    -x.f()\n    await x.f()\n    ~x.f()\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("x.f"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
            (5, "f", 0, Some("x.f"), CALL),
        ],
    ),
    (
        "R5-F3.S10 go unary operators and receive stay outside the path (clean)",
        ProgrammingLanguage::Go,
        "go",
        "package p\nfunc g() {\n    _ = !x.F()\n    _ = -x.F()\n    _ = <-x.F()\n    _ = *x.F()\n    _ = ^pkg.F()\n}\n",
        &[
            (3, "F", 0, Some("x.F"), CALL),
            (4, "F", 0, Some("x.F"), CALL),
            (5, "F", 0, Some("x.F"), CALL),
            (6, "F", 0, Some("x.F"), CALL),
            (7, "F", 0, Some("pkg.F"), CALL),
        ],
    ),
    (
        "R5-F3.S11 c prefix operators and casts stay outside the path (clean)",
        ProgrammingLanguage::C,
        "c",
        "void g() {\n    !x.f();\n    -p->f();\n    *p->f();\n    (T)x.f();\n}\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("p->f"), CALL),
            (4, "f", 0, Some("p->f"), CALL),
            (5, "f", 0, Some("x.f"), CALL),
        ],
    ),
    (
        "R5-F3.S11 c++ prefix operators, `co_await` and `throw` stay outside the path (clean)",
        ProgrammingLanguage::Cpp,
        "cpp",
        "void h() {\n    !N::f();\n    -obj.template m<int>();\n    co_await x.f();\n    throw x.f();\n}\n",
        &[
            (2, "f", 0, Some("N::f"), CALL),
            (3, "m", 0, Some("obj.template m"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
            (5, "f", 0, Some("x.f"), CALL),
        ],
    ),
    (
        "R5-F3.S12 scala prefix operators and ascriptions stay outside the path (clean)",
        ProgrammingLanguage::Scala,
        "scala",
        "object O {\n  def g(): Unit = {\n    !x.f()\n    -x.f()\n    (x: T).f()\n  }\n}\n",
        &[
            (3, "f", 0, Some("x.f"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
            (5, "f", 0, Some("(x: T).f"), CALL),
        ],
    ),
    (
        "R5-F3.S13 php prefix operators, `@`, casts and `clone` stay outside the path (clean)",
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction g() {\n    !$x->f();\n    -$x->f();\n    @$x->f();\n    (int)$x->f();\n    clone $x->f();\n    !\\N\\f();\n}\n",
        &[
            (3, "f", 0, Some("$x->f"), CALL),
            (4, "f", 0, Some("$x->f"), CALL),
            (5, "f", 0, Some("$x->f"), CALL),
            (6, "f", 0, Some("$x->f"), CALL),
            (7, "f", 0, Some("$x->f"), CALL),
            (8, "f", 0, Some("\\N\\f"), CALL),
        ],
    ),
    (
        "R5-F3.S14 lua `not`, `-` and `#` stay outside the path (clean)",
        ProgrammingLanguage::Lua,
        "lua",
        "local function g()\n  local a = not x.f()\n  local b = -x:f()\n  local c = #x.f()\nend\n",
        &[
            (2, "f", 0, Some("x.f"), CALL),
            (3, "f", 0, Some("x:f"), CALL),
            (4, "f", 0, Some("x.f"), CALL),
        ],
    ),
    (
        "R5-F3.S15 perl prefix operators stay outside the path (clean)",
        ProgrammingLanguage::Perl,
        "pl",
        "sub g {\n    !$x->f();\n    -$x->f();\n    !R4::f();\n    !&R4::f;\n}\n",
        &[
            (2, "f", 0, Some("$x->f"), CALL),
            (3, "f", 0, Some("$x->f"), CALL),
            (4, "f", 0, Some("R4::f"), CALL),
            (5, "f", 0, Some("R4::f"), CALL),
        ],
    ),
    (
        "R4-F1.27 bash control",
        ProgrammingLanguage::Bash,
        "sh",
        "cb() { :; }\nf() {\n  cb\n  other/external\n}\n",
        &[(3, "cb", 0, None, CALL), (4, "other/external", 0, None, CALL)],
    ),
    (
        "R4-F1.27 zsh control",
        ProgrammingLanguage::Zsh,
        "zsh",
        "cb() { :; }\nf() {\n  cb\n  other/external\n}\n",
        &[(3, "cb", 0, None, CALL), (4, "other/external", 0, None, CALL)],
    ),
    // In an assignment target the grammar writes `[*x.s]` as `(*x).s`;
    // the star unpacks the whole attribute, as in `f(*x.p)`.
    (
        "R5-X.06 python starred assignment targets keep the star outside the path",
        ProgrammingLanguage::Python,
        "py",
        "[*x.s] = r\n*x.s, y = r\n[*x.s.t] = r\n[*x[0].h] = r\nfor *x.s, y in r: pass\nf(*x.p, **x.q)\n",
        &[
            (1, "s", 0, Some("x.s"), USAGE),
            (2, "s", 0, Some("x.s"), USAGE),
            (3, "s", 0, Some("x.s"), USAGE),
            (3, "t", 0, Some("x.s.t"), USAGE),
            (4, "h", 0, Some("x[0].h"), USAGE),
            (5, "s", 0, Some("x.s"), USAGE),
            (6, "p", 0, Some("x.p"), USAGE),
            (6, "q", 0, Some("x.q"), USAGE),
        ],
    ),
    (
        "R5-X.10 php static members keep their class scope",
        ProgrammingLanguage::Php,
        "php",
        "<?php\nfunction t() {\n    $a = X::K;\n    $b = X::$f;\n    X::m();\n    $c = \\N\\X::K;\n    $d = $x::K;\n    $e = static::K;\n    ++X::$f;\n    X::$f = 1;\n    (X::$f)();\n    $g = X::$f->h;\n}\n",
        &[
            (3, "X", 0, None, USAGE),
            (3, "K", 0, Some("X::K"), USAGE),
            (4, "X", 0, None, USAGE),
            (4, "f", 0, Some("X::$f"), USAGE),
            (5, "m", 0, Some("X::m"), CALL),
            (6, "K", 0, Some("\\N\\X::K"), USAGE),
            (7, "K", 0, Some("$x::K"), USAGE),
            (8, "K", 0, Some("static::K"), USAGE),
            (9, "f", 0, Some("X::$f"), USAGE),
            (10, "f", 0, Some("X::$f"), USAGE),
            (11, "f", 0, Some("X::$f"), USAGE),
            (12, "f", 0, Some("X::$f"), USAGE),
            (12, "h", 0, Some("X::$f->h"), USAGE),
        ],
    ),
];

crate::language_cases::language_tests!(written_paths_keep_every_root_qualifier_and_receiver => check_written_paths_keep_every_root_qualifier_and_receiver);

fn check_written_paths_keep_every_root_qualifier_and_receiver(language: ProgrammingLanguage) {
    for language in ProgrammingLanguage::all() {
        assert!(
            WRITTEN_PATH_MATRIX.iter().any(|row| row.1 == language),
            "{}: no written-path row",
            language.name()
        );
    }
    for &(label, _, extension, source, checks) in WRITTEN_PATH_MATRIX.iter().filter(|row| ProgrammingLanguage::from_extension(row.2) == Some(language)) {
        let file = Analyzed::new(source, extension);
        for &(line, name, nth, path, kind) in checks {
            let occurrence = file.at(line, name, nth);
            assert_eq!(file.text(occurrence), name, "{label}: the span stays the name token");
            assert_eq!(occurrence.target.qualified_name.as_deref(), path, "{label}: `{name}` on line {line}");
            assert_eq!(occurrence.kind, kind, "{label}: `{name}` on line {line}: {occurrence:#?}");
            if let Some(path) = path {
                let end = occurrence.span.end_byte as usize;
                assert!(source[..end].ends_with(path), "{label}: `{path}` is the source ending at `{name}`");
            }
        }
        let spans: HashSet<(u32, u32)> = file
            .evidence
            .occurrences
            .iter()
            .map(|occurrence| (occurrence.span.start_byte, occurrence.span.end_byte))
            .collect();
        assert_eq!(spans.len(), file.evidence.occurrences.len(), "{label}: one occurrence per name token");
    }
}

/// `(label, source, [(line, column, name, written path, NameOnly?)],
/// [(import name, alias, source, line, column)])`. Every occurrence on a line
/// the row names is listed (all are Usages); lines it does not name (a
/// `module`/`package` declaration name) are left to the declaration
/// exclusions.
pub(crate) type QualifiedImportRow = (
    &'static str,
    &'static str,
    &'static [(u32, u32, &'static str, Option<&'static str>, bool)],
    &'static [ImportEntry<'static>],
);

/// `(import name, alias, source, line, column)`.
type ImportEntry<'a> = (&'a str, Option<&'a str>, Option<&'a str>, u32, u32);

/// Java qualified names outside an `import` (qualified annotations, module
/// directive operands, module and package names, type paths) are reads
/// with their written paths, never imports; a genuine `import` reports only
/// its final segment (review rows `R7-F3.nn`).
pub(crate) const JAVA_QUALIFIED_REFERENCE_MATRIX: &[QualifiedImportRow] = &[
    (
        "R7-F3.01 marker annotation (with an unqualified control)",
        "class A {}\n@p.q.A class G {}\n@A class H {}\n",
        &[
            (2, 2, "p", None, false),
            (2, 4, "q", Some("p.q"), false),
            (2, 6, "A", Some("p.q.A"), true),
            (3, 2, "A", None, true),
        ],
        &[],
    ),
    (
        "R7-F3.02 annotation with an argument",
        "class A {}\n@p.q.A(external) class G {}\n",
        &[
            (2, 2, "p", None, false),
            (2, 4, "q", Some("p.q"), false),
            (2, 6, "A", Some("p.q.A"), true),
            (2, 8, "external", None, false),
        ],
        &[],
    ),
    (
        "R7-F3.02 named annotation arguments",
        "class A {}\n@p.q.A(value=external) class G {}\n",
        &[
            (2, 2, "p", None, false),
            (2, 4, "q", Some("p.q"), false),
            (2, 6, "A", Some("p.q.A"), true),
            (2, 8, "value", None, false),
            (2, 14, "external", None, false),
        ],
        &[],
    ),
    (
        "R7-F3.03 requires (simple, qualified, transitive)",
        "module m {\n    requires a.b;\n    requires c;\n    requires transitive d.e.f;\n}\n",
        &[
            (2, 14, "a", None, false),
            (2, 16, "b", Some("a.b"), false),
            (3, 14, "c", None, false),
            (4, 25, "d", None, false),
            (4, 27, "e", Some("d.e"), false),
            (4, 29, "f", Some("d.e.f"), false),
        ],
        &[],
    ),
    (
        "R7-F3.04 exports a package",
        "module m {\n    exports p.q;\n}\n",
        &[(2, 13, "p", None, false), (2, 15, "q", Some("p.q"), false)],
        &[],
    ),
    (
        "R7-F3.05 exports to target modules",
        "module m {\n    exports p.q to x.y, z.w;\n}\n",
        &[
            (2, 13, "p", None, false),
            (2, 15, "q", Some("p.q"), false),
            (2, 20, "x", None, false),
            (2, 22, "y", Some("x.y"), false),
            (2, 25, "z", None, false),
            (2, 27, "w", Some("z.w"), false),
        ],
        &[],
    ),
    (
        "R7-F3.06 opens a package",
        "module m {\n    opens r.s;\n}\n",
        &[(2, 11, "r", None, false), (2, 13, "s", Some("r.s"), false)],
        &[],
    ),
    (
        "R7-F3.07 opens to a target module",
        "module m {\n    opens r.s to t.u;\n}\n",
        &[
            (2, 11, "r", None, false),
            (2, 13, "s", Some("r.s"), false),
            (2, 18, "t", None, false),
            (2, 20, "u", Some("t.u"), false),
        ],
        &[],
    ),
    (
        "R7-F3.08 uses a service (with an unqualified control)",
        "module m {\n    uses C;\n    uses a.b.C;\n}\nclass C {}\n",
        &[
            (2, 10, "C", None, true),
            (3, 10, "a", None, false),
            (3, 12, "b", Some("a.b"), false),
            (3, 14, "C", Some("a.b.C"), true),
        ],
        &[],
    ),
    (
        "R7-F3.09 provides a service (with an unqualified control)",
        "module m {\n    provides C with D;\n    provides a.b.C with D;\n}\nclass C {}\nclass D {}\n",
        &[
            (2, 14, "C", None, true),
            (2, 21, "D", None, true),
            (3, 14, "a", None, false),
            (3, 16, "b", Some("a.b"), false),
            (3, 18, "C", Some("a.b.C"), true),
            (3, 25, "D", None, true),
        ],
        &[],
    ),
    (
        "R7-F3.10 provider list",
        "module m {\n    provides a.b.C with d.e.D, f.g.F;\n}\nclass C {}\nclass D {}\nclass F {}\n",
        &[
            (2, 14, "a", None, false),
            (2, 16, "b", Some("a.b"), false),
            (2, 18, "C", Some("a.b.C"), true),
            (2, 25, "d", None, false),
            (2, 27, "e", Some("d.e"), false),
            (2, 29, "D", Some("d.e.D"), true),
            (2, 32, "f", None, false),
            (2, 34, "g", Some("f.g"), false),
            (2, 36, "F", Some("f.g.F"), true),
        ],
        &[],
    ),
    (
        "R7-F3.11 three-plus segments in every slot",
        "module m {\n    requires d.e.f.g;\n    exports p.q.r to x.y.z;\n    opens r.s.t to t.u.v;\n    uses a.b.c.C;\n    provides a.b.c.C with d.e.f.D;\n}\nclass C {}\nclass D {}\n@p.q.r.C class G {}\n",
        &[
            (2, 14, "d", None, false),
            (2, 16, "e", Some("d.e"), false),
            (2, 18, "f", Some("d.e.f"), false),
            (2, 20, "g", Some("d.e.f.g"), false),
            (3, 13, "p", None, false),
            (3, 15, "q", Some("p.q"), false),
            (3, 17, "r", Some("p.q.r"), false),
            (3, 22, "x", None, false),
            (3, 24, "y", Some("x.y"), false),
            (3, 26, "z", Some("x.y.z"), false),
            (4, 11, "r", None, false),
            (4, 13, "s", Some("r.s"), false),
            (4, 15, "t", Some("r.s.t"), false),
            (4, 20, "t", None, false),
            (4, 22, "u", Some("t.u"), false),
            (4, 24, "v", Some("t.u.v"), false),
            (5, 10, "a", None, false),
            (5, 12, "b", Some("a.b"), false),
            (5, 14, "c", Some("a.b.c"), false),
            (5, 16, "C", Some("a.b.c.C"), true),
            (6, 14, "a", None, false),
            (6, 16, "b", Some("a.b"), false),
            (6, 18, "c", Some("a.b.c"), false),
            (6, 20, "C", Some("a.b.c.C"), true),
            (6, 27, "d", None, false),
            (6, 29, "e", Some("d.e"), false),
            (6, 31, "f", Some("d.e.f"), false),
            (6, 33, "D", Some("d.e.f.D"), true),
            (10, 2, "p", None, false),
            (10, 4, "q", Some("p.q"), false),
            (10, 6, "r", Some("p.q.r"), false),
            (10, 8, "C", Some("p.q.r.C"), true),
        ],
        &[],
    ),
    (
        "R7-F3.12 genuine imports report their final segment only",
        "package z.y;\nimport a.b.C;\nimport a.b.*;\nimport static a.b.C.m;\nimport static a.b.C.*;\nimport d.e.f.g.H;\nclass K { C c; H h; }\nclass R { void k() { k(); } }\n",
        &[
            (2, 8, "a", None, false),
            (2, 10, "b", Some("a.b"), false),
            (3, 8, "a", None, false),
            (4, 15, "a", None, false),
            (4, 17, "b", Some("a.b"), false),
            (4, 19, "C", Some("a.b.C"), false),
            (5, 15, "a", None, false),
            (5, 17, "b", Some("a.b"), false),
            (6, 8, "d", None, false),
            (6, 10, "e", Some("d.e"), false),
            (6, 12, "f", Some("d.e.f"), false),
            (6, 14, "g", Some("d.e.f.g"), false),
            (7, 11, "C", None, false),
            (7, 16, "H", None, false),
        ],
        &[
            ("C", None, Some("a.b"), 2, 12),
            ("b", Some("b"), Some("a.b"), 3, 10),
            ("m", None, Some("a.b.C"), 4, 21),
            ("C", Some("C"), Some("a.b.C"), 5, 19),
            ("H", None, Some("d.e.f.g"), 6, 16),
        ],
    ),
    (
        "R7-F3.13 package declaration and qualified type path",
        "package a.b;\nclass A { p.q.A x; }\n",
        &[
            (2, 11, "p", None, false),
            (2, 13, "q", Some("p.q"), false),
            (2, 15, "A", Some("p.q.A"), true),
        ],
        &[],
    ),
    (
        "R7-F3.13 qualified module declaration",
        "module a.b {\n    requires c;\n}\n",
        &[(2, 14, "c", None, false)],
        &[],
    ),
];

#[test]
fn java_qualified_names_outside_an_import_are_reads_not_imports() {
    assert_import_rows(JAVA_QUALIFIED_REFERENCE_MATRIX, "java");
}

/// C# `using` directives (review rows `R7-F3.S1.nn`): a namespace or
/// `static` directive imports its path's final segment with the whole path
/// as `source`; an alias directive imports its alias with its target as
/// written. As in Java, every qualifier, and every name in an alias target,
/// is a read with its written path; the imported token never is.
const CSHARP_USING_MATRIX: &[QualifiedImportRow] = &[
    (
        "R7-F3.S1.01 namespace directives (with the one-segment control)",
        "using System;\nusing A.B;\nusing A.B.C;\n",
        &[(2, 7, "A", None, false), (3, 7, "A", None, false), (3, 9, "B", Some("A.B"), false)],
        &[
            ("System", None, Some("System"), 1, 7),
            ("B", None, Some("A.B"), 2, 9),
            ("C", None, Some("A.B.C"), 3, 11),
        ],
    ),
    (
        "R7-F3.S1.02 static directives",
        "using static C.D;\nusing static unsafe Q.R;\nusing static A.B<int>;\n",
        &[(1, 14, "C", None, false), (2, 21, "Q", None, false), (3, 14, "A", None, false)],
        &[
            ("D", None, Some("C.D"), 1, 16),
            ("R", None, Some("Q.R"), 2, 23),
            ("B", None, Some("A.B<int>"), 3, 16),
        ],
    ),
    (
        "R7-F3.S1.03 alias directives and a later use of the alias",
        "using E = F.G;\nusing F2 = F;\nclass G {}\nclass Z { E e; }\n",
        &[
            (1, 11, "F", None, false),
            (1, 13, "G", Some("F.G"), true),
            (2, 12, "F", None, false),
            (4, 11, "E", None, false),
        ],
        &[("E", Some("E"), Some("F.G"), 1, 7), ("F2", Some("F2"), Some("F"), 2, 7)],
    ),
    (
        "R7-F3.S1.04 generic, pointer and tuple alias targets",
        "using X = A.B<C>;\nusing unsafe P = int*;\nusing T = (int a, int b);\n",
        &[(1, 11, "A", None, false), (1, 13, "B", Some("A.B"), false), (1, 15, "C", None, false)],
        &[
            ("X", Some("X"), Some("A.B<C>"), 1, 7),
            ("P", Some("P"), Some("int*"), 2, 14),
            ("T", Some("T"), Some("(int a, int b)"), 3, 7),
        ],
    ),
    (
        "R7-F3.S1.05 global directive and an alias-qualified path",
        "global using H.I;\nusing global::System.Text;\n",
        &[(1, 14, "H", None, false)],
        &[("I", None, Some("H.I"), 1, 16), ("Text", None, Some("global::System.Text"), 2, 22)],
    ),
    (
        "R7-F3.S1.06 directives inside a namespace",
        "namespace N { using J.K; using L = M.N.O; class Z {} }\n",
        &[
            (1, 21, "J", None, false),
            (1, 36, "M", None, false),
            (1, 38, "N", Some("M.N"), true),
            (1, 40, "O", Some("M.N.O"), false),
        ],
        &[("K", None, Some("J.K"), 1, 23), ("L", Some("L"), Some("M.N.O"), 1, 32)],
    ),
];

#[test]
fn csharp_using_directives_import_their_binding_and_read_the_rest() {
    assert_import_rows(CSHARP_USING_MATRIX, "cs");
}

/// Scala imports (review rows `R7-F3.S6.nn`): only the bound name is an
/// import (a path's last segment, a selector, a rename's new name, an
/// on-demand import's opened path); every qualifier, a rename's original
/// and a hidden selector (`x => _`) are reads, with written paths along an
/// import path.
const SCALA_IMPORT_MATRIX: &[QualifiedImportRow] = &[
    (
        "R7-F3.S6.01 qualified and comma-separated paths, with later reads",
        "import a.b.C\nimport g.h, i.j\nobject O { val v = (C, h, j, b) }\nclass b\n",
        &[
            (1, 8, "a", None, false),
            (1, 10, "b", Some("a.b"), true),
            (2, 8, "g", None, false),
            (2, 13, "i", None, false),
            (3, 21, "C", None, false),
            (3, 24, "h", None, false),
            (3, 27, "j", None, false),
            (3, 30, "b", None, true),
        ],
        &[("C", None, Some("a.b.C"), 1, 12), ("h", None, Some("g.h"), 2, 10), ("j", None, Some("i.j"), 2, 15)],
    ),
    (
        "R7-F3.S6.02 braced selectors and renames",
        "import a.{D, E => F, G as H}\nobject O { val v = (D, F, H, E, G) }\n",
        &[
            (1, 8, "a", None, false),
            (1, 14, "E", None, false),
            (1, 22, "G", None, false),
            (2, 21, "D", None, false),
            (2, 24, "F", None, false),
            (2, 27, "H", None, false),
            (2, 30, "E", None, false),
            (2, 33, "G", None, false),
        ],
        &[("D", None, Some("a.D"), 1, 11), ("F", Some("F"), Some("a.E"), 1, 19), ("H", Some("H"), Some("a.G"), 1, 27)],
    ),
    (
        "R7-F3.S6.03 on-demand imports and a hidden selector",
        "import a.b.*\nimport c.d._\nimport e.given\nimport f.{given T, x => _}\nobject O { val v = (b, d, x) }\n",
        &[
            (1, 8, "a", None, false),
            (2, 8, "c", None, false),
            (4, 17, "T", None, false),
            (4, 20, "x", None, false),
            (5, 21, "b", None, false),
            (5, 24, "d", None, false),
            (5, 27, "x", None, false),
        ],
        &[
            ("b", Some("b"), Some("a.b"), 1, 10),
            ("d", Some("d"), Some("c.d"), 2, 10),
            ("e", Some("e"), Some("e"), 3, 8),
            ("f", Some("f"), Some("f"), 4, 8),
        ],
    ),
];

#[test]
fn scala_imports_bind_only_their_names_and_read_the_rest() {
    assert_import_rows(SCALA_IMPORT_MATRIX, "scala");
}

/// Python imports (review rows `R7-F3.S4.nn`): `import a.b` binds only
/// `a`, so `b` is a read written `a.b`, as every module path segment of an
/// `import … as` or a `from` import is; a future import and an on-demand
/// `from m import *` are imports.
const PYTHON_IMPORT_MATRIX: &[QualifiedImportRow] = &[
    (
        "R7-F3.S4.01 import statements",
        "import a.b\nimport c.d as e, f\ndef m():\n    return a.b, e, f, d\n",
        &[
            (1, 10, "b", Some("a.b"), false),
            (2, 8, "c", None, false),
            (2, 10, "d", Some("c.d"), false),
            (4, 12, "a", None, false),
            (4, 14, "b", Some("a.b"), false),
            (4, 17, "e", None, false),
            (4, 20, "f", None, false),
            (4, 23, "d", None, false),
        ],
        &[("a", None, Some("a.b"), 1, 8), ("e", Some("e"), Some("c.d"), 2, 15), ("f", None, Some("f"), 2, 18)],
    ),
    (
        "R7-F3.S4.02 from and future imports",
        "from g.h import i as j, k\nfrom __future__ import annotations\ndef m():\n    return j, k, i, annotations\n",
        &[
            (1, 6, "g", None, false),
            (1, 8, "h", Some("g.h"), false),
            (1, 17, "i", None, false),
            (4, 12, "j", None, false),
            (4, 15, "k", None, false),
            (4, 18, "i", None, false),
            (4, 21, "annotations", None, false),
        ],
        &[
            ("j", Some("j"), Some("g.h"), 1, 22),
            ("k", None, Some("g.h"), 1, 25),
            ("annotations", None, Some("__future__"), 2, 24),
        ],
    ),
    (
        "R7-F3.S4.03 on-demand imports",
        "from l.m import *\nfrom . import *\ndef f():\n    return m, l\n",
        &[(1, 6, "l", None, false), (4, 12, "m", None, false), (4, 15, "l", None, false)],
        &[("m", Some("m"), Some("l.m"), 1, 8), (".", Some("."), Some("."), 2, 6)],
    ),
];

#[test]
fn python_imports_bind_only_their_names_and_read_the_rest() {
    assert_import_rows(PYTHON_IMPORT_MATRIX, "py");
}

/// PHP `use` clauses (review rows `R7-F3.S3.nn`): a clause imports its
/// alias or else its last segment, also in a group and for `use
/// function`/`use const`; qualifiers and an aliased original are reads.
const PHP_USE_MATRIX: &[QualifiedImportRow] = &[(
    "R7-F3.S3.01 one-segment, aliased, grouped, function and const clauses",
    "<?php\nuse A;\nuse B\\C as D;\nuse E\\{F, G as H};\nuse function I\\j;\nuse const K\\L as M;\necho M, C, G, A;\n",
    &[
        (3, 5, "B", None, false),
        (3, 7, "C", Some("B\\C"), false),
        (4, 5, "E", None, false),
        (4, 11, "G", None, false),
        (5, 14, "I", None, false),
        (6, 11, "K", None, false),
        (6, 13, "L", Some("K\\L"), false),
        (7, 6, "M", None, false),
        (7, 9, "C", None, false),
        (7, 12, "G", None, false),
        (7, 15, "A", None, false),
    ],
    &[
        ("A", None, Some("A"), 2, 5),
        ("D", Some("D"), Some("B\\C"), 3, 12),
        ("F", None, Some("E\\F"), 4, 8),
        ("H", Some("H"), Some("E\\G"), 4, 16),
        ("j", None, Some("I\\j"), 5, 16),
        ("M", Some("M"), Some("K\\L"), 6, 18),
    ],
)];

#[test]
fn php_use_clauses_bind_only_their_names_and_read_the_rest() {
    assert_import_rows(PHP_USE_MATRIX, "php");
}

/// Go imports (review rows `R7-F3.X3.nn`): a raw-string path is the same
/// import as an interpreted one, named by its last segment, and a blank
/// import binds nothing; later uses of the package names (and of a path
/// segment, `c`) stay reads.
const GO_IMPORT_MATRIX: &[QualifiedImportRow] = &[(
    "R7-F3.X3.01 raw-string, aliased, blank and dot imports",
    "package p\nimport `c/d`\nimport x `e/f`\nimport _ `i/j`\nimport . `k/l`\nimport \"m/n\"\nfunc f() { _ = d; _ = x; _ = n; _ = c }\n",
    &[(7, 16, "d", None, false), (7, 23, "x", None, false), (7, 30, "n", None, false), (7, 37, "c", None, false)],
    &[
        ("d", None, Some("c/d"), 2, 8),
        ("x", Some("x"), Some("e/f"), 3, 8),
        ("_", Some("_"), Some("i/j"), 4, 8),
        ("l", Some("l"), Some("k/l"), 5, 10),
        ("n", None, Some("m/n"), 6, 8),
    ],
)];

#[test]
fn go_imports_bind_only_their_names_and_read_the_rest() {
    assert_import_rows(GO_IMPORT_MATRIX, "go");
}

/// Checks every row: the reads on the lines it names (exactly, all
/// Usages), the exact import list, no occurrence on an imported token, and
/// a limited language's conservative evidence.
fn assert_import_rows(rows: &[QualifiedImportRow], extension: &str) {
    for &(label, source, reads, imports) in rows {
        let lines: Vec<&str> = source.split('\n').collect();
        for &(line, column, name, _, _) in reads {
            let start = column as usize - 1;
            assert_eq!(lines[line as usize - 1].get(start..start + name.len()), Some(name), "{label}: no `{name}` at {line}:{column}");
        }
        let file = Analyzed::new(source, extension);
        for import in &file.index.imports {
            assert!(
                !file.evidence.occurrences.iter().any(|occurrence| occurrence.span.start_byte == import.span.start_byte),
                "{label}: the imported `{}` is an occurrence",
                import.name
            );
        }

        let checked_lines: HashSet<u32> = reads.iter().map(|&(line, ..)| line).collect();
        let observed: Vec<(u32, u32, &str, Option<&str>, bool)> = file
            .evidence
            .occurrences
            .iter()
            .filter(|occurrence| checked_lines.contains(&occurrence.span.start.line))
            .map(|occurrence| {
                assert_eq!(occurrence.kind, USAGE, "{label}: {occurrence:#?}");
                assert_uncertain(occurrence);
                let actual_candidates: HashSet<&SymbolId> = occurrence.candidates.iter().filter_map(|candidate| candidate.id.as_ref()).collect();
                let expected_candidates: HashSet<&SymbolId> = if occurrence.confidence == Confidence::NameOnly {
                    file.index.symbols.iter().filter(|record| record.identity.name == occurrence.target.name).map(|record| &record.id).collect()
                } else {
                    HashSet::new()
                };
                assert_eq!(actual_candidates, expected_candidates, "{label}: candidate identities");
                (
                    occurrence.span.start.line,
                    occurrence.span.start.column,
                    occurrence.target.name.as_str(),
                    occurrence.target.qualified_name.as_deref(),
                    occurrence.confidence == Confidence::NameOnly,
                )
            })
            .collect();
        assert_eq!(observed, reads, "{label}: occurrences");

        let observed_imports: Vec<ImportEntry<'_>> = file
            .index
            .imports
            .iter()
            .map(|import| {
                (import.name.as_str(), import.alias.as_deref(), import.source.as_deref(), import.span.start.line, import.span.start.column)
            })
            .collect();
        assert_eq!(observed_imports, imports, "{label}: imports");

        // The language stays limited: no binding, no edges, recursion unknown.
        for coverage in &file.evidence.reference_coverage {
            assert_eq!(coverage.state, CoverageState::Limited, "{label}");
            assert!(coverage.reasons.contains(&LimitReason::UnsupportedLanguage), "{label}");
        }
        for record in &file.index.symbols {
            assert!(record.relations.references.is_empty(), "{label}: `{}` has edges", record.identity.name);
            assert!(record.relations.referenced_by.is_empty(), "{label}: `{}` has edges", record.identity.name);
            assert!(record.relations.dependencies.is_empty(), "{label}: `{}` has dependencies", record.identity.name);
            if record.semantics.is_recursive.is_some() {
                assert_eq!(record.semantics.is_recursive, Some(TriState::Unknown), "{label}: `{}`", record.identity.name);
            }
        }
    }
}

/// `(label, extension, source, [(line, member name, written path and kind,
/// or `None` when the name is no occurrence on that line)])`.
type OperandMemberRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(u32, &'static str, Option<(&'static str, OccurrenceKind)>)],
);

/// A member access in an operand position (`delete`, `++`/`--`, compound
/// and plain assignment targets, address-of, dereference, unary operators,
/// casts, `del`/`unset`, `ref`/`out`) is captured exactly as the same access
/// in an argument (each row's first line): with its written path where the
/// language captures member names, and nowhere where only an invoked member
/// name is captured (TS/JS, C/C++, Go, Rust; Perl hash keys are no member
/// names). Lua's `x.y = 1` declares a table member, so it is no occurrence.
const OPERAND_MEMBER_MATRIX: &[OperandMemberRow] = &[
    (
        "R5-X.01 typescript",
        "ts",
        "function t() {\n  use(x.f);\n  delete x.f;\n  ++x.y;\n  x.y++;\n  x.y += 1;\n  x.y = 1;\n  typeof x.f;\n  void x.f;\n  x.m();\n}\n",
        &[(2, "f", None), (3, "f", None), (4, "y", None), (5, "y", None), (6, "y", None), (7, "y", None), (8, "f", None), (9, "f", None), (10, "m", Some(("x.m", CALL)))],
    ),
    (
        "R5-X.01 javascript",
        "js",
        "function t() {\n  use(x.f);\n  delete x.f;\n  ++x.y;\n  x.y++;\n  x.y += 1;\n  x.y = 1;\n  typeof x.f;\n  void x.f;\n  x.m();\n}\n",
        &[(2, "f", None), (3, "f", None), (4, "y", None), (5, "y", None), (6, "y", None), (7, "y", None), (8, "f", None), (9, "f", None), (10, "m", Some(("x.m", CALL)))],
    ),
    (
        "R5-X.02 c",
        "c",
        "void t() {\n  use(x.f);\n  use(&x.f);\n  ++x.y;\n  x.y++;\n  x.y += 1;\n  *x.f;\n  &p->q;\n  x.m();\n}\n",
        &[(2, "f", None), (3, "f", None), (4, "y", None), (5, "y", None), (6, "y", None), (7, "f", None), (8, "q", None), (9, "m", Some(("x.m", CALL)))],
    ),
    (
        "R5-X.02 c++",
        "cpp",
        "void t() {\n  use(x.f);\n  use(&x.f);\n  ++x.y;\n  x.y++;\n  x.y += 1;\n  *x.f;\n  &p->q;\n  x.m();\n}\n",
        &[(2, "f", None), (3, "f", None), (4, "y", None), (5, "y", None), (6, "y", None), (7, "f", None), (8, "q", None), (9, "m", Some(("x.m", CALL)))],
    ),
    (
        "R5-X.03 go",
        "go",
        "package p\n\nfunc t() {\n\tuse(x.F)\n\tuse(&x.F)\n\tx.Y++\n\tx.Y += 1\n\t_ = <-x.C\n\tx.M()\n}\n",
        &[(4, "F", None), (5, "F", None), (6, "Y", None), (7, "Y", None), (8, "C", None), (9, "M", Some(("x.M", CALL)))],
    ),
    (
        "R5-X.04 rust",
        "rs",
        "fn t() {\n    use_(x.f);\n    x.y += 1;\n    let _ = &x.f;\n    let _ = &mut x.f;\n    let _ = *x.f;\n    let _ = x.f?;\n}\n",
        &[(2, "f", None), (3, "y", None), (4, "f", None), (5, "f", None), (6, "f", None), (7, "f", None)],
    ),
    (
        "R5-X.05 python",
        "py",
        "def t():\n    use(x.f)\n    del x.f\n    x.y += 1\n    x.y = 1\n    not x.f\n    -x.f\n    for x.k in r: pass\n    print(*x.p, **x.q)\n",
        &[
            (2, "f", Some(("x.f", USAGE))),
            (3, "f", Some(("x.f", USAGE))),
            (4, "y", Some(("x.y", USAGE))),
            (5, "y", Some(("x.y", USAGE))),
            (6, "f", Some(("x.f", USAGE))),
            (7, "f", Some(("x.f", USAGE))),
            (8, "k", Some(("x.k", USAGE))),
            (9, "p", Some(("x.p", USAGE))),
            (9, "q", Some(("x.q", USAGE))),
        ],
    ),
    (
        "R5-X.07 java",
        "java",
        "class E { void t() {\n  use(x.f);\n  ++x.y;\n  x.y++;\n  x.y += 1;\n  boolean b = !x.f;\n  int u = ~x.f;\n  int w = (int) x.f;\n} }\n",
        &[
            (2, "f", Some(("x.f", USAGE))),
            (3, "y", Some(("x.y", USAGE))),
            (4, "y", Some(("x.y", USAGE))),
            (5, "y", Some(("x.y", USAGE))),
            (6, "f", Some(("x.f", USAGE))),
            (7, "f", Some(("x.f", USAGE))),
            (8, "f", Some(("x.f", USAGE))),
        ],
    ),
    (
        "R5-X.08 c#",
        "cs",
        "class E { unsafe void T() {\n  Use(x.F);\n  ++x.Y;\n  x.Y++;\n  x.Y ??= 1;\n  var p = &x.F;\n  Use(ref x.F);\n  var n = x.F!;\n} }\n",
        &[
            (2, "F", Some(("x.F", USAGE))),
            (3, "Y", Some(("x.Y", USAGE))),
            (4, "Y", Some(("x.Y", USAGE))),
            (5, "Y", Some(("x.Y", USAGE))),
            (6, "F", Some(("x.F", USAGE))),
            (7, "F", Some(("x.F", USAGE))),
            (8, "F", Some(("x.F", USAGE))),
        ],
    ),
    (
        "R5-X.09 php",
        "php",
        "<?php\nfunction t() {\n  use_($x->f);\n  ++$x->y;\n  $x->y++;\n  unset($x->f);\n  $r = &$x->f;\n  $d = @$x->f;\n  $i = isset($x->f);\n}\n",
        &[
            (3, "f", Some(("$x->f", USAGE))),
            (4, "y", Some(("$x->y", USAGE))),
            (5, "y", Some(("$x->y", USAGE))),
            (6, "f", Some(("$x->f", USAGE))),
            (7, "f", Some(("$x->f", USAGE))),
            (8, "f", Some(("$x->f", USAGE))),
            (9, "f", Some(("$x->f", USAGE))),
        ],
    ),
    (
        "R5-X.11 lua",
        "lua",
        "function t()\n  use(x.f)\n  local a = #x.f\n  local b = -x.f\n  local c = not x.f\n  local d = x.f .. x.g\n  x.y = 1\nend\n",
        &[
            (2, "f", Some(("x.f", USAGE))),
            (3, "f", Some(("x.f", USAGE))),
            (4, "f", Some(("x.f", USAGE))),
            (5, "f", Some(("x.f", USAGE))),
            (6, "f", Some(("x.f", USAGE))),
            (6, "g", Some(("x.g", USAGE))),
            (7, "y", None),
        ],
    ),
    (
        "R5-X.12 swift",
        "swift",
        "func t() {\n  use(x.f)\n  x.y += 1\n  _ = !x.f\n  h(&x.f)\n  _ = try x.f\n  _ = x.f!\n  _ = x?.f\n}\n",
        &[
            (2, "f", Some(("x.f", USAGE))),
            (3, "y", Some(("x.y", USAGE))),
            (4, "f", Some(("x.f", USAGE))),
            (5, "f", Some(("x.f", USAGE))),
            (6, "f", Some(("x.f", USAGE))),
            (7, "f", Some(("x.f", USAGE))),
            (8, "f", Some(("x?.f", USAGE))),
        ],
    ),
    (
        "R5-X.13 scala",
        "scala",
        "object E { def t() = {\n  use(x.f)\n  x.y += 1\n  !x.f\n  -x.f\n  x.f: Int\n} }\n",
        &[
            (2, "f", Some(("x.f", USAGE))),
            (3, "y", Some(("x.y", USAGE))),
            (4, "f", Some(("x.f", USAGE))),
            (5, "f", Some(("x.f", USAGE))),
            (6, "f", Some(("x.f", USAGE))),
        ],
    ),
    (
        "R5-X.14 perl",
        "pl",
        "sub t {\n  use_($x->{f});\n  ++$x->{f};\n  delete $x->{f};\n  my $w = !$x->m;\n}\n",
        &[(2, "f", None), (3, "f", None), (4, "f", None), (5, "m", Some(("$x->m", CALL)))],
    ),
];

crate::language_cases::language_tests!(operand_position_member_names_match_their_argument_capture => check_operand_position_member_names_match_their_argument_capture);

fn check_operand_position_member_names_match_their_argument_capture(language: ProgrammingLanguage) {
    for language in ProgrammingLanguage::all() {
        if matches!(language, ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh) {
            continue; // no member access
        }
        assert!(
            OPERAND_MEMBER_MATRIX.iter().any(|row| ProgrammingLanguage::from_extension(row.1) == Some(language)),
            "{}: no operand-position row",
            language.name()
        );
    }
    for &(label, extension, source, checks) in OPERAND_MEMBER_MATRIX.iter().filter(|row| ProgrammingLanguage::from_extension(row.1) == Some(language)) {
        let file = Analyzed::new(source, extension);
        for &(line, name, expected) in checks {
            let found: Vec<&Occurrence> =
                file.all(name).into_iter().filter(|occurrence| occurrence.span.start.line == line).collect();
            match expected {
                None => assert!(found.is_empty(), "{label}: `{name}` on line {line} is an occurrence: {found:#?}"),
                Some((path, kind)) => {
                    assert_eq!(found.len(), 1, "{label}: `{name}` on line {line}: {:#?}", file.evidence.occurrences);
                    assert_eq!(found[0].target.qualified_name.as_deref(), Some(path), "{label}: `{name}` on line {line}");
                    assert_eq!(found[0].kind, kind, "{label}: `{name}` on line {line}");
                }
            }
        }
    }
}

/// A Java construction's written receiver (`outer.new C()`) is evidence of
/// what the source wrote, never of a target: `C` stays a limited-language
/// guess over every record named `C`, the receiver is its own occurrence
/// (none for the keyword `this`), and no reference edge appears.
#[test]
fn java_construction_receivers_never_establish_a_target() {
    let source = "class G {\n    class C {}\n    void g(G outer) {\n        outer.new C();\n        this.new C();\n        factory().new C();\n    }\n    G factory() { return this; }\n}\n";
    let file = Analyzed::new(source, "java");
    let class = file.named("C", 2).clone();
    for line in [4, 5, 6] {
        let constructed = file.at(line, "C", 0);
        assert_eq!(constructed.confidence, Confidence::NameOnly, "line {line}: {constructed:#?}");
        assert_eq!(constructed.target.id, None, "line {line}");
        let candidates: Vec<_> = constructed.candidates.iter().map(|candidate| candidate.id.clone()).collect();
        assert_eq!(candidates, [Some(class.clone())], "line {line}");
    }
    assert!(file.all("this").is_empty(), "the keyword `this` is no occurrence");
    assert_eq!(file.at(4, "outer", 0).span.end_byte, file.at(4, "C", 0).span.start_byte - ".new ".len() as u32);
    assert_eq!(file.at(6, "factory", 0).kind, CALL);
    assert!(
        file.evidence.reference_coverage.iter().all(|region| region.state == CoverageState::Limited),
        "java stays limited"
    );
    let g = file.record("G.g", 3);
    assert!(g.relations.references.is_empty(), "a receiver makes no edge: {:#?}", g.relations.references);
    assert!(file.record("G.C", 2).relations.referenced_by.is_empty());
}

/// A rooted Rust name (`::f`) is never looked up lexically: it never binds
/// the shadowing `let f`, and stays uncertain over the item records of its
/// name, while the unrooted call beside it binds the local.
#[test]
fn rooted_rust_names_never_bind_a_lexical_local() {
    let file = Analyzed::new(
        "fn f() {\n    ::f();\n}\nfn g() {\n    let f = || {};\n    ::f();\n    f();\n}\n",
        "rs",
    );
    let function = file.named("f", 1).clone();
    let local = file.named("f", 5).clone();
    for line in [2, 6] {
        let rooted = file.at(line, "f", 0);
        assert_uncertain(rooted);
        assert_eq!(rooted.kind, CALL, "{rooted:#?}");
        let candidates: Vec<_> = rooted.candidates.iter().map(|candidate| candidate.id.clone()).collect();
        assert_eq!(candidates, [Some(function.clone())], "line {line}: the item, never the local");
    }
    assert_scoped_to(file.at(7, "f", 0), &local);
}

/// `(name, alias, original, source, line, column)` of one import record.
pub(crate) type ExpectedImport<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Option<&'a str>, u32, u32);

/// `(line, column, name, written path, kind, confidence)` of one occurrence.
pub(crate) type ExpectedRead<'a> = (u32, u32, &'a str, Option<&'a str>, OccurrenceKind, Confidence);

/// `(callable, [(import name, original)], recursion)`.
pub(crate) type ExpectedOwner = (&'static str, &'static [(&'static str, Option<&'static str>)], TriState);

/// One source with independently expected import identities and reads.
pub(crate) struct CommentedImportRow<'a> {
    pub(crate) label: &'a str,
    pub(crate) extension: &'a str,
    pub(crate) source: &'a str,
    /// The complete import list, in source order.
    pub(crate) imports: &'a [ExpectedImport<'a>],
    /// Every occurrence of the file, in source order: an imported token is
    /// never one, every qualifier and body read is.
    pub(crate) reads: &'a [ExpectedRead<'a>],
    /// For a resolved language: the callable whose dependencies are exactly
    /// `(import name, original)` and whose recursion is the given state.
    pub(crate) owner: Option<ExpectedOwner>,
}

const SCOPED: Confidence = Confidence::Scoped;
const NAME_ONLY: Confidence = Confidence::NameOnly;
const UNRESOLVED: Confidence = Confidence::Unresolved;

/// Comments in import syntax (review rows `R8-F1.nn`, siblings `.Sn`, clean
/// controls `.08`–`.10`): a comment never becomes an import's original
/// target, never ends its written path, never decides whether a path
/// segment binds, and never turns an imported token into a read. A path
/// written across a comment keeps it as source text; a TypeScript entity
/// alias's original drops it, since its last segment decides recursion.
pub(crate) const COMMENTED_IMPORT_MATRIX: &[CommentedImportRow] = &[
    CommentedImportRow {
        label: "R8-F2.07.ts string-named original",
        extension: "ts",
        source: "import { \"a-b\" as c } from \"m\";\nfunction run() { c(); }\n",
        imports: &[("c", Some("c"), Some("\"a-b\""), Some("m"), 1, 19)],
        reads: &[(2, 18, "c", None, CALL, SCOPED)],
        owner: Some(("run", &[("c", Some("\"a-b\""))], TriState::No)),
    },
    CommentedImportRow {
        label: "R8-F2.07.js string-named original",
        extension: "js",
        source: "import { \"a-b\" as c } from \"m\";\nfunction run() { c(); }\n",
        imports: &[("c", Some("c"), Some("\"a-b\""), Some("m"), 1, 19)],
        reads: &[(2, 18, "c", None, CALL, SCOPED)],
        owner: Some(("run", &[("c", Some("\"a-b\""))], TriState::No)),
    },
    CommentedImportRow {
        label: "R8-F1.01 TS entity alias, block comment before `=`",
        extension: "ts",
        source: "import D /* note */ = N.S;\nfunction S() { D(); }\n",
        imports: &[("D", Some("D"), Some("N.S"), None, 1, 8)],
        reads: &[(1, 23, "N", None, USAGE, UNRESOLVED), (2, 16, "D", None, CALL, SCOPED)],
        owner: Some(("S", &[("D", Some("N.S"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.01 TS entity alias, line comment before `=`",
        extension: "ts",
        source: "import D // note\n  = N.S;\nfunction S() { D(); }\n",
        imports: &[("D", Some("D"), Some("N.S"), None, 1, 8)],
        reads: &[(2, 5, "N", None, USAGE, UNRESOLVED), (3, 16, "D", None, CALL, SCOPED)],
        owner: Some(("S", &[("D", Some("N.S"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.01.export TS exported entity alias, comment before `=`",
        extension: "ts",
        source: "export import D /* note */ = N.S;\nfunction S() { D(); }\n",
        imports: &[("D", Some("D"), Some("N.S"), None, 1, 15)],
        reads: &[(1, 30, "N", None, USAGE, UNRESOLVED), (2, 16, "D", None, CALL, SCOPED)],
        owner: Some(("S", &[("D", Some("N.S"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.02 TS entity alias, block comment after `=`",
        extension: "ts",
        source: "import D = /* note */ N.S;\nfunction S() { D(); }\n",
        imports: &[("D", Some("D"), Some("N.S"), None, 1, 8)],
        reads: &[(1, 23, "N", None, USAGE, UNRESOLVED), (2, 16, "D", None, CALL, SCOPED)],
        owner: Some(("S", &[("D", Some("N.S"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.02 TS entity alias, line comment after `=`",
        extension: "ts",
        source: "import D = // note\n  N.S;\nfunction S() { D(); }\n",
        imports: &[("D", Some("D"), Some("N.S"), None, 1, 8)],
        reads: &[(2, 3, "N", None, USAGE, UNRESOLVED), (3, 16, "D", None, CALL, SCOPED)],
        owner: Some(("S", &[("D", Some("N.S"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.S1 TS entity alias, comment before the bound name",
        extension: "ts",
        source: "import /* note */ D = N.S;\nfunction S() { D(); }\n",
        imports: &[("D", Some("D"), Some("N.S"), None, 1, 19)],
        reads: &[(1, 23, "N", None, USAGE, UNRESOLVED), (2, 16, "D", None, CALL, SCOPED)],
        owner: Some(("S", &[("D", Some("N.S"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.S2 TS entity alias, comment inside the target path",
        extension: "ts",
        source: "import D = N. /* note */ S;\nimport E = N /* note */ .S;\nfunction S() { D(); E(); }\n",
        imports: &[("D", Some("D"), Some("N.S"), None, 1, 8), ("E", Some("E"), Some("N.S"), None, 2, 8)],
        reads: &[
            (1, 12, "N", None, USAGE, UNRESOLVED),
            (2, 12, "N", None, USAGE, UNRESOLVED),
            (3, 16, "D", None, CALL, SCOPED),
            (3, 21, "E", None, CALL, SCOPED),
        ],
        owner: Some(("S", &[("D", Some("N.S")), ("E", Some("N.S"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.09.ts clean TS import forms beside comments",
        extension: "ts",
        source: "import D = N.S; // trailing\nimport q = require(/* note */ \"m\");\nimport { a as b } from \"m\";\nimport def from \"m\";\n\
                 import * as ns from \"m\";\nimport R = N;\nfunction g() { D(); q(); b(); def(); ns.x(); R(); }\n",
        imports: &[
            ("D", Some("D"), Some("N.S"), None, 1, 8),
            ("q", None, None, Some("m"), 2, 8),
            ("b", Some("b"), Some("a"), Some("m"), 3, 15),
            ("def", None, None, Some("m"), 4, 8),
            ("ns", Some("ns"), Some("*"), Some("m"), 5, 13),
            ("R", Some("R"), Some("N"), None, 6, 8),
        ],
        reads: &[
            (1, 12, "N", None, USAGE, UNRESOLVED),
            (6, 12, "N", None, USAGE, UNRESOLVED),
            (7, 16, "D", None, CALL, SCOPED),
            (7, 21, "q", None, CALL, SCOPED),
            (7, 26, "b", None, CALL, SCOPED),
            (7, 31, "def", None, CALL, SCOPED),
            (7, 38, "ns", None, USAGE, SCOPED),
            (7, 41, "x", Some("ns.x"), CALL, UNRESOLVED),
            (7, 46, "R", None, CALL, SCOPED),
        ],
        owner: Some((
            "g",
            &[("D", Some("N.S")), ("R", Some("N")), ("b", Some("a")), ("def", None), ("ns", Some("*")), ("q", None)],
            TriState::Unknown,
        )),
    },
    CommentedImportRow {
        label: "R8-F1.09.ts named, default and namespace imports beside comments",
        extension: "ts",
        source: "import { c /* x */ as /* y */ d } from \"m\";\nimport /* a */ e from \"m\";\nimport * as /* b */ ns from \"m\";\n\
                 function f() { d(); e(); ns.x(); }\n",
        imports: &[
            ("d", Some("d"), Some("c"), Some("m"), 1, 31),
            ("e", None, None, Some("m"), 2, 16),
            ("ns", Some("ns"), Some("*"), Some("m"), 3, 21),
        ],
        reads: &[
            (4, 16, "d", None, CALL, SCOPED),
            (4, 21, "e", None, CALL, SCOPED),
            (4, 26, "ns", None, USAGE, SCOPED),
            (4, 29, "x", Some("ns.x"), CALL, UNRESOLVED),
        ],
        owner: Some(("f", &[("d", Some("c")), ("e", None), ("ns", Some("*"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.09.js named, default and namespace imports beside comments",
        extension: "js",
        source: "import { c /* x */ as /* y */ d } from \"m\";\nimport /* a */ e from \"m\";\nimport * as /* b */ ns from \"m\";\n\
                 function f() { d(); e(); ns.x(); }\n",
        imports: &[
            ("d", Some("d"), Some("c"), Some("m"), 1, 31),
            ("e", None, None, Some("m"), 2, 16),
            ("ns", Some("ns"), Some("*"), Some("m"), 3, 21),
        ],
        reads: &[
            (4, 16, "d", None, CALL, SCOPED),
            (4, 21, "e", None, CALL, SCOPED),
            (4, 26, "ns", None, USAGE, SCOPED),
            (4, 29, "x", Some("ns.x"), CALL, UNRESOLVED),
        ],
        owner: Some(("f", &[("d", Some("c")), ("e", None), ("ns", Some("*"))], TriState::Unknown)),
    },
    CommentedImportRow {
        label: "R8-F1.03 Scala block comment between path segments",
        extension: "scala",
        source: "import a /* note */ .b.C\nobject U { def C() = 1; def f() = C() }\n",
        imports: &[("C", None, None, Some("a /* note */ .b.C"), 1, 24)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 22, "b", Some("a /* note */ .b"), USAGE, UNRESOLVED),
            (2, 35, "C", None, CALL, NAME_ONLY),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.03 Scala line comment between path segments",
        extension: "scala",
        source: "import a // note\n  .b.C\nobject U { def C() = 1; def f() = C() }\n",
        imports: &[("C", None, None, Some("a // note\n  .b.C"), 2, 6)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (2, 4, "b", Some("a // note\n  .b"), USAGE, UNRESOLVED),
            (3, 35, "C", None, CALL, NAME_ONLY),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.03 Scala comment in a braced and an on-demand path",
        extension: "scala",
        source: "import a /* note */ .b.{C}\nimport a /* note */ .b._\nobject U { def b() = 1; def f() = (C, b()) }\n",
        imports: &[
            ("C", None, None, Some("a /* note */ .b.C"), 1, 25),
            ("b", Some("b"), Some("*"), Some("a /* note */ .b"), 2, 22),
        ],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 22, "b", Some("a /* note */ .b"), USAGE, NAME_ONLY),
            (2, 8, "a", None, USAGE, UNRESOLVED),
            (3, 36, "C", None, USAGE, UNRESOLVED),
            (3, 39, "b", None, CALL, NAME_ONLY),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.04a Scala comment before a plain selector group",
        extension: "scala",
        source: "import a.b. /* note */ {C}\nimport x.y. // note\n  {E}\nobject U { val v = (C, E) }\n",
        imports: &[("C", None, None, Some("a.b.C"), 1, 25), ("E", None, None, Some("x.y.E"), 3, 4)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (2, 8, "x", None, USAGE, UNRESOLVED),
            (2, 10, "y", Some("x.y"), USAGE, UNRESOLVED),
            (4, 21, "C", None, USAGE, UNRESOLVED),
            (4, 24, "E", None, USAGE, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.04b Scala comment before a renamed selector (`=>` and `as`)",
        extension: "scala",
        source: "import a.b. /* note */ {C => D}\nimport x.y. /* note */ {E as F}\nobject U { def f() = (D(), F()) }\n",
        imports: &[("D", Some("D"), Some("C"), Some("a.b.C"), 1, 30), ("F", Some("F"), Some("E"), Some("x.y.E"), 2, 30)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (1, 25, "C", None, USAGE, UNRESOLVED),
            (2, 8, "x", None, USAGE, UNRESOLVED),
            (2, 10, "y", Some("x.y"), USAGE, UNRESOLVED),
            (2, 25, "E", None, USAGE, UNRESOLVED),
            (3, 23, "D", None, CALL, UNRESOLVED),
            (3, 28, "F", None, CALL, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.05.star Scala comment before a `*` wildcard (block and line)",
        extension: "scala",
        source: "import a.b. /* note */ *\nimport c.d. // note\n  *\nobject U { def b() = 1; def d() = 2; def f() = (b(), d()) }\n",
        imports: &[("b", Some("b"), Some("*"), Some("a.b"), 1, 10), ("d", Some("d"), Some("*"), Some("c.d"), 2, 10)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (2, 8, "c", None, USAGE, UNRESOLVED),
            (4, 49, "b", None, CALL, NAME_ONLY),
            (4, 54, "d", None, CALL, NAME_ONLY),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.05.underscore Scala comment before a `_` wildcard",
        extension: "scala",
        source: "import a.b. /* note */ _\nobject U { def b() = 1; def f() = b() }\n",
        imports: &[("b", Some("b"), Some("*"), Some("a.b"), 1, 10)],
        reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (2, 35, "b", None, CALL, NAME_ONLY)],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.05.given Scala comment before a direct `given`",
        extension: "scala",
        source: "import a.b. /* note */ given\nobject U { def b() = 1; def f() = b() }\n",
        imports: &[("b", Some("b"), Some("given"), Some("a.b"), 1, 10)],
        reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (2, 35, "b", None, CALL, NAME_ONLY)],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.06.mixed Scala comment before `{_, given T}`",
        extension: "scala",
        source: "import a.b. /* note */ {_, given T}\nobject U { def b() = 1; def f() = b() }\n",
        imports: &[("b", Some("b"), Some("*"), Some("a.b"), 1, 10)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 34, "T", None, USAGE, UNRESOLVED),
            (2, 35, "b", None, CALL, NAME_ONLY),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.06.given Scala comment before `{given T}`",
        extension: "scala",
        source: "import a.b. /* note */ {given T}\nobject U { def b() = 1; def f() = b() }\n",
        imports: &[("b", Some("b"), Some("given"), Some("a.b"), 1, 10)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 31, "T", None, USAGE, UNRESOLVED),
            (2, 35, "b", None, CALL, NAME_ONLY),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.07.simple Scala comment before `,` after a one-segment path",
        extension: "scala",
        source: "import C /* note */, d.E\nobject U { def C() = 1; def f() = (C(), E) }\n",
        imports: &[("C", None, None, Some("C"), 1, 8), ("E", None, None, Some("d.E"), 1, 24)],
        reads: &[
            (1, 22, "d", None, USAGE, UNRESOLVED),
            (2, 36, "C", None, CALL, NAME_ONLY),
            (2, 41, "E", None, USAGE, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.07.qualified Scala comment before `,` after a qualified path",
        extension: "scala",
        source: "import a.b.C /* note */, d.E\nobject U { def C() = 1; def f() = (C(), E) }\n",
        imports: &[("C", None, None, Some("a.b.C"), 1, 12), ("E", None, None, Some("d.E"), 1, 28)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (1, 26, "d", None, USAGE, UNRESOLVED),
            (2, 36, "C", None, CALL, NAME_ONLY),
            (2, 41, "E", None, USAGE, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.08 Scala comments the binding never depended on",
        extension: "scala",
        source: "import a.b.C /* end */\nimport a.b.* /* star */\nimport a.b.C, /* after comma */ d.E\n\
                 import a.b.{C, /* in */ D}\nimport a.b.{C /* x */ => D}\nimport a.b.{C => /* y */ D}\nimport a.b.{C as /* z */ D}\n\
                 object U { def f() = C() }\n",
        imports: &[
            ("C", None, None, Some("a.b.C"), 1, 12),
            ("b", Some("b"), Some("*"), Some("a.b"), 2, 10),
            ("C", None, None, Some("a.b.C"), 3, 12),
            ("E", None, None, Some("d.E"), 3, 35),
            ("C", None, None, Some("a.b.C"), 4, 13),
            ("D", None, None, Some("a.b.D"), 4, 25),
            ("D", Some("D"), Some("C"), Some("a.b.C"), 5, 26),
            ("D", Some("D"), Some("C"), Some("a.b.C"), 6, 26),
            ("D", Some("D"), Some("C"), Some("a.b.C"), 7, 26),
        ],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (2, 8, "a", None, USAGE, UNRESOLVED),
            (3, 8, "a", None, USAGE, UNRESOLVED),
            (3, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (3, 33, "d", None, USAGE, UNRESOLVED),
            (4, 8, "a", None, USAGE, UNRESOLVED),
            (4, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (5, 8, "a", None, USAGE, UNRESOLVED),
            (5, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (5, 13, "C", None, USAGE, UNRESOLVED),
            (6, 8, "a", None, USAGE, UNRESOLVED),
            (6, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (6, 13, "C", None, USAGE, UNRESOLVED),
            (7, 8, "a", None, USAGE, UNRESOLVED),
            (7, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (7, 13, "C", None, USAGE, UNRESOLVED),
            (8, 22, "C", None, CALL, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.10.rs Rust use paths, lists, renames, globs and extern aliases with comments",
        extension: "rs",
        source: "use a /* c */ ::b::C;\nuse a::{/* c */ g::h, self as s};\nuse a::b::{c /* x */ as d};\nuse a::b::*;\n\
                 extern crate y /* c */ as z;\nfn f() { C(); h(); d(); }\n",
        imports: &[
            ("C", None, None, Some("a /* c */ ::b"), 1, 20),
            ("h", None, None, Some("a::g"), 2, 20),
            ("s", Some("s"), Some("a"), None, 2, 31),
            ("d", Some("d"), Some("c"), Some("a::b"), 3, 25),
            ("b", Some("b"), Some("*"), Some("a::b"), 4, 8),
            ("z", Some("z"), Some("y"), None, 5, 27),
        ],
        reads: &[(6, 10, "C", None, CALL, SCOPED), (6, 15, "h", None, CALL, SCOPED), (6, 20, "d", None, CALL, SCOPED)],
        owner: Some(("f", &[("C", None), ("d", Some("c")), ("h", None)], TriState::No)),
    },
    CommentedImportRow {
        label: "R8-F1.10.java Java qualified imports with comments",
        extension: "java",
        source: "import a /* c */ .b.C;\nimport a.b.D /* c */;\nclass X { C c; D d; }\n",
        imports: &[("C", None, None, Some("a /* c */ .b"), 1, 21), ("D", None, None, Some("a.b"), 2, 12)],
        reads: &[
            (1, 8, "a", None, USAGE, UNRESOLVED),
            (1, 19, "b", Some("a /* c */ .b"), USAGE, UNRESOLVED),
            (2, 8, "a", None, USAGE, UNRESOLVED),
            (2, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (3, 11, "C", None, USAGE, UNRESOLVED),
            (3, 16, "D", None, USAGE, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.10.cs C# alias, static and global usings with comments",
        extension: "cs",
        source: "using D /* c */ = a /* d */ .b.C;\nusing static System.Math;\nglobal using a.b;\nclass X { D d; }\n",
        imports: &[
            ("D", Some("D"), Some("C"), Some("a /* d */ .b.C"), 1, 7),
            ("Math", None, None, Some("System.Math"), 2, 21),
            ("b", None, None, Some("a.b"), 3, 16),
        ],
        reads: &[
            (1, 19, "a", None, USAGE, UNRESOLVED),
            (1, 30, "b", Some("a /* d */ .b"), USAGE, UNRESOLVED),
            (1, 32, "C", Some("a /* d */ .b.C"), USAGE, UNRESOLVED),
            (2, 14, "System", None, USAGE, UNRESOLVED),
            (3, 14, "a", None, USAGE, UNRESOLVED),
            (4, 11, "D", None, USAGE, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.10.php PHP use paths, aliases and groups with comments",
        extension: "php",
        source: "<?php\nuse A /* c */ \\B;\nuse A\\B /* c */ as D;\nuse A\\{B /* c */, C};\nfunction f(D $d, B $b) {}\n",
        imports: &[
            ("B", None, None, Some("A /* c */ \\B"), 2, 16),
            ("D", Some("D"), Some("B"), Some("A\\B"), 3, 20),
            ("B", None, None, Some("A\\B"), 4, 8),
            ("C", None, None, Some("A\\C"), 4, 19),
        ],
        reads: &[
            (2, 5, "A", None, USAGE, UNRESOLVED),
            (3, 5, "A", None, USAGE, UNRESOLVED),
            (3, 7, "B", Some("A\\B"), USAGE, UNRESOLVED),
            (4, 5, "A", None, USAGE, UNRESOLVED),
            (5, 12, "D", None, USAGE, UNRESOLVED),
            (5, 18, "B", None, USAGE, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.10.swift Swift selective and whole-module imports with comments",
        extension: "swift",
        source: "import struct Foo /* c */ .Bar\nimport Foundation\nlet x: Bar = Bar()\n",
        imports: &[("Bar", None, None, Some("Foo /* c */ .Bar"), 1, 28), ("Foundation", None, None, Some("Foundation"), 2, 8)],
        reads: &[(1, 15, "Foo", None, USAGE, UNRESOLVED), (3, 8, "Bar", None, USAGE, UNRESOLVED), (3, 14, "Bar", None, CALL, UNRESOLVED)],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.10.py Python parenthesized from-import with a line comment",
        extension: "py",
        source: "from m import (C,  # c\n    D as E)\nimport a.b\nfrom __future__ import annotations\nfrom n import *\nC(); E()\n",
        imports: &[
            ("C", None, None, Some("m"), 1, 16),
            ("E", Some("E"), Some("D"), Some("m"), 2, 10),
            ("a", None, None, Some("a.b"), 3, 8),
            ("annotations", None, None, Some("__future__"), 4, 24),
            ("n", Some("n"), Some("*"), Some("n"), 5, 6),
        ],
        reads: &[
            (1, 6, "m", None, USAGE, UNRESOLVED),
            (2, 5, "D", None, USAGE, UNRESOLVED),
            (3, 10, "b", Some("a.b"), USAGE, UNRESOLVED),
            (6, 1, "C", None, CALL, UNRESOLVED),
            (6, 6, "E", None, CALL, UNRESOLVED),
        ],
        owner: None,
    },
    CommentedImportRow {
        label: "R8-F1.10.go Go aliased, blank, dot and raw-string imports with comments",
        extension: "go",
        source: "package p\nimport (\n\tD /* c */ \"a/b\"\n\t_ \"a/c\"\n\t. \"a/d\"\n\tr `a/e`\n)\nfunc f() { D.F() }\n",
        imports: &[
            ("D", Some("D"), Some("b"), Some("a/b"), 3, 2),
            ("_", Some("_"), Some("c"), Some("a/c"), 4, 2),
            ("d", Some("d"), Some("*"), Some("a/d"), 5, 4),
            ("r", Some("r"), Some("e"), Some("a/e"), 6, 2),
        ],
        reads: &[(8, 12, "D", None, USAGE, UNRESOLVED), (8, 14, "F", Some("D.F"), CALL, UNRESOLVED)],
        owner: None,
    },
];

/// Each prefix is the source slice the import reader carries verbatim;
/// a selector/rename appends its original after that slice. This independently
/// defines which comment positions belong to the expected source path.
struct ScalaImportAlternative {
    row: CommentedImportRow<'static>,
    prefixes: &'static [(usize, usize)],
}

const SCALA_IMPORT_ALTERNATIVES: &[ScalaImportAlternative] = &[
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.01.single",
            extension: "scala",
            source: "import a.b.C as D\ndef D() = 0\ndef run() = D()\n",
            imports: &[("D", Some("D"), Some("C"), Some("a.b.C"), 1, 17)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED), (1, 12, "C", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 10)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.01.multiple",
            extension: "scala",
            source: "import a.b.C as D, x.y.E as F\ndef D() = 0\ndef run() = D()\n",
            imports: &[("D", Some("D"), Some("C"), Some("a.b.C"), 1, 17), ("F", Some("F"), Some("E"), Some("x.y.E"), 1, 29)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED), (1, 12, "C", None, USAGE, UNRESOLVED), (1, 20, "x", None, USAGE, UNRESOLVED), (1, 22, "y", Some("x.y"), USAGE, UNRESOLVED), (1, 24, "E", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 10), (19, 22)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.01.nested",
            extension: "scala",
            source: "object U { import a.b.C as D\ndef D() = 0\ndef run() = D()\n}\n",
            imports: &[("D", Some("D"), Some("C"), Some("a.b.C"), 1, 28)],
            reads: &[(1, 19, "a", None, USAGE, UNRESOLVED), (1, 21, "b", Some("a.b"), USAGE, UNRESOLVED), (1, 23, "C", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(18, 21)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.02.escaped",
            extension: "scala",
            source: "import a.b.C as `D`\ndef D() = 0\ndef run() = D()\n",
            imports: &[("`D`", Some("`D`"), Some("C"), Some("a.b.C"), 1, 17)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED), (1, 12, "C", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 10)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.02.original",
            extension: "scala",
            source: "import a.b.++ as sum\ndef D() = 0\ndef run() = D()\n",
            imports: &[("sum", Some("sum"), Some("++"), Some("a.b.++"), 1, 18)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 10)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.03.direct",
            extension: "scala",
            source: "import a.b.+\ndef D() = 0\ndef run() = D()\n",
            imports: &[("+", None, None, Some("a.b.+"), 1, 12)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 12)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.03.multiple",
            extension: "scala",
            source: "import a.b.+, c.d.**\ndef D() = 0\ndef run() = D()\n",
            imports: &[("+", None, None, Some("a.b.+"), 1, 12), ("**", None, None, Some("c.d.**"), 1, 19)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 10, "b", Some("a.b"), USAGE, UNRESOLVED), (1, 15, "c", None, USAGE, UNRESOLVED), (1, 17, "d", Some("c.d"), USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 12), (14, 20)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.03.braced",
            extension: "scala",
            source: "import a.{+, ** as mul}\ndef D() = 0\ndef run() = D()\n",
            imports: &[("+", None, None, Some("a.+"), 1, 11), ("mul", Some("mul"), Some("**"), Some("a.**"), 1, 20)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 8), (7, 8)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.04.arrow",
            extension: "scala",
            source: "import a.{C => ++}\ndef D() = 0\ndef run() = D()\n",
            imports: &[("++", Some("++"), Some("C"), Some("a.C"), 1, 16)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 11, "C", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 8)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.04.as",
            extension: "scala",
            source: "import a.{C as ++}\ndef D() = 0\ndef run() = D()\n",
            imports: &[("++", Some("++"), Some("C"), Some("a.C"), 1, 16)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 11, "C", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 8)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.04.as.mixed",
            extension: "scala",
            source: "import a.{C as D, ++ as sum, E as **}\ndef D() = 0\ndef run() = D()\n",
            imports: &[("D", Some("D"), Some("C"), Some("a.C"), 1, 16), ("sum", Some("sum"), Some("++"), Some("a.++"), 1, 25), ("**", Some("**"), Some("E"), Some("a.E"), 1, 35)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 11, "C", None, USAGE, UNRESOLVED), (1, 30, "E", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 8), (7, 8), (7, 8)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.05.direct.root",
            extension: "scala",
            source: "import +.C\ndef D() = 0\ndef run() = D()\n",
            imports: &[("C", None, None, Some("+.C"), 1, 10)],
            reads: &[(3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 10)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.05.direct",
            extension: "scala",
            source: "import a.+.C\ndef D() = 0\ndef run() = D()\n",
            imports: &[("C", None, None, Some("a.+.C"), 1, 12)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 12)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.05.braced",
            extension: "scala",
            source: "import a.+.{C}\ndef D() = 0\ndef run() = D()\n",
            imports: &[("C", None, None, Some("a.+.C"), 1, 13)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 10)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.05.wildcard",
            extension: "scala",
            source: "import a.+.*\ndef D() = 0\ndef run() = D()\n",
            imports: &[("+", Some("+"), Some("*"), Some("a.+"), 1, 10)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 10)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.06.hidden",
            extension: "scala",
            source: "import a.{C as _, ++ => sum}\ndef D() = 0\ndef run() = D()\n",
            imports: &[("sum", Some("sum"), Some("++"), Some("a.++"), 1, 25)],
            reads: &[(1, 8, "a", None, USAGE, UNRESOLVED), (1, 11, "C", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 8)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.06.backticks",
            extension: "scala",
            source: "import `a`.`b`.`C`\ndef D() = 0\ndef run() = D()\n",
            imports: &[("`C`", None, None, Some("`a`.`b`.`C`"), 1, 16)],
            reads: &[(1, 8, "`a`", None, USAGE, UNRESOLVED), (1, 12, "`b`", Some("`a`.`b`"), USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 18)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.06.this",
            extension: "scala",
            source: "import this.C\ndef D() = 0\ndef run() = D()\n",
            imports: &[("C", None, None, Some("this.C"), 1, 13)],
            reads: &[(1, 8, "this", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 13)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.06.super",
            extension: "scala",
            source: "import super.C\ndef D() = 0\ndef run() = D()\n",
            imports: &[("C", None, None, Some("super.C"), 1, 14)],
            reads: &[(1, 8, "super", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 14)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.06.root",
            extension: "scala",
            source: "import _root_.a.C\ndef D() = 0\ndef run() = D()\n",
            imports: &[("C", None, None, Some("_root_.a.C"), 1, 17)],
            reads: &[(1, 8, "_root_", None, USAGE, UNRESOLVED), (1, 15, "a", Some("_root_.a"), USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 17)],
    },
    ScalaImportAlternative {
        row: CommentedImportRow {
            label: "R8-F2.06.given",
            extension: "scala",
            source: "import a.{_, given T}\ndef D() = 0\ndef run() = D()\n",
            imports: &[("a", Some("a"), Some("*"), Some("a"), 1, 8)],
            reads: &[(1, 20, "T", None, USAGE, UNRESOLVED), (3, 13, "D", None, CALL, NAME_ONLY)],
            owner: None,
        },
        prefixes: &[(7, 8)],
    },
];

/// Visit the controls and single-comment edits at import token boundaries.
/// Expectations shift from the control's explicit byte positions, not from
/// the extractor under test. Line comments before `as` terminate a Scala
/// import (as does a line comment before a comma outside selectors), so
/// those are not alternate spellings of an unbraced rename.
pub(crate) fn for_each_import_row(mut check: impl FnMut(&CommentedImportRow<'_>)) {
    for row in COMMENTED_IMPORT_MATRIX {
        check(row);
    }
    for alternative in SCALA_IMPORT_ALTERNATIVES {
        let base = &alternative.row;
        check(base);
        let header_end = base.source.find('\n').expect("header");
        let header = &base.source[..header_end];
        let mut offsets = Vec::new();
        for (offset, ch) in header.char_indices() {
            if offset > 0 && (ch.is_whitespace() || ".{},".contains(ch) || header.as_bytes()[offset - 1] == b'.') {
                offsets.push(offset);
            }
        }
        offsets.push(header.len());
        offsets.sort_unstable();
        offsets.dedup();
        for offset in offsets {
            // Never insert inside a backticked identifier or a symbolic token.
            if header[..offset].bytes().filter(|byte| *byte == b'`').count() % 2 != 0 {
                continue;
            }
            for comment in [" /* n */ ", " // n\n"] {
                let import_start = header.find("import ").expect("import") + "import ".len();
                if offset < import_start {
                    continue;
                }
                let before = &header[import_start..offset];
                if comment.contains('\n') && offset != header_end && !before.trim_end().ends_with('.')
                    && before.bytes().filter(|ch| *ch == b'{').count()
                        <= before.bytes().filter(|ch| *ch == b'}').count() {
                    continue;
                }
                let source = format!("{}{}{}", &base.source[..offset], comment, &base.source[offset..]);
                let label = format!("{}: comment at {offset}: {comment:?}", base.label);
                let shifted = |byte: usize| byte + if byte >= offset { comment.len() } else { 0 };
                let coordinates = |byte: usize| {
                    let byte = shifted(byte);
                    (source[..byte].bytes().filter(|ch| *ch == b'\n').count() as u32 + 1,
                     (byte - source[..byte].rfind('\n').map_or(0, |line| line + 1)) as u32 + 1)
                };
                let sources: Vec<String> = base.imports.iter().zip(alternative.prefixes).map(|(import, &(start, end))| {
                    let suffix = &import.3.expect("source")[end - start..];
                    let prefix_end = end + if end > offset { comment.len() } else { 0 };
                    format!("{}{suffix}", &source[shifted(start)..prefix_end])
                }).collect();
                let imports: Vec<ExpectedImport<'_>> = base.imports.iter().zip(&sources).map(|(import, path)| {
                    let (line, column) = coordinates(byte_at(base.source, import.4, import.5));
                    (import.0, import.1, import.2, Some(path.as_str()), line, column)
                }).collect();
                let paths: Vec<Option<String>> = base.reads.iter().map(|read| read.3.map(|path| {
                    let end = byte_at(base.source, read.0, read.1) + read.2.len();
                    let start = end - path.len();
                    let end = end + if end > offset { comment.len() } else { 0 };
                    source[shifted(start)..end].to_string()
                })).collect();
                let reads: Vec<ExpectedRead<'_>> = base.reads.iter().zip(&paths).map(|(read, path)| {
                    let (line, column) = coordinates(byte_at(base.source, read.0, read.1));
                    (line, column, read.2, path.as_deref(), read.4, read.5)
                }).collect();
                check(&CommentedImportRow {
                    label: &label, extension: "scala", source: &source,
                    imports: &imports, reads: &reads, owner: None,
                });
            }
        }
    }
}

/// One-based `(line, column)` → byte offset in `source`.
fn byte_at(source: &str, line: u32, column: u32) -> usize {
    let line_start: usize = source.split_inclusive('\n').take(line as usize - 1).map(str::len).sum();
    line_start + column as usize - 1
}

/// Checks an import row against a complete index and its
/// evidence: the exact imports (name, alias, original, source, position),
/// every occurrence (an imported token is none), and for a resolved
/// language the owner's dependencies and recursion, else the limited
/// language's conservative evidence. Shared with the cache stages, whose
/// narrowed results may omit candidates' and the owner's records
/// (`every_kind` false): a candidate is then checked by name, and the
/// owner only when its record is present.
pub(crate) fn assert_commented_import_row(
    row: &CommentedImportRow,
    cell: &str,
    index: &FileSymbolIndex,
    evidence: &ReferenceEvidence,
    every_kind: bool,
) {
    let imports: Vec<ExpectedImport<'_>> = index
        .imports
        .iter()
        .map(|import| {
            (
                import.name.as_str(),
                import.alias.as_deref(),
                import.symbol.qualified_name.as_deref(),
                import.source.as_deref(),
                import.span.start.line,
                import.span.start.column,
            )
        })
        .collect();
    assert_eq!(imports, row.imports, "{cell}: imports");
    for import in &index.imports {
        assert!(
            !index.symbols.iter().any(|record| record.identity.name == import.name
                && record.source.declaration_span.start_byte <= import.span.start_byte
                && import.span.end_byte <= record.source.declaration_span.end_byte),
            "{cell}: the import `{}` became a declaration record",
            import.name
        );
        assert!(
            !index.exports.iter().any(|export| export.name == import.name),
            "{cell}: the import `{}` became an export",
            import.name
        );
    }
    let reads: Vec<ExpectedRead<'_>> = evidence
        .occurrences
        .iter()
        .map(|occurrence| {
            (
                occurrence.span.start.line,
                occurrence.span.start.column,
                occurrence.target.name.as_str(),
                occurrence.target.qualified_name.as_deref(),
                occurrence.kind,
                occurrence.confidence,
            )
        })
        .collect();
    assert_eq!(reads, row.reads, "{cell}: occurrences");
    for occurrence in &evidence.occurrences {
        if row.label.starts_with("R8-F2.") && row.extension == "scala" && occurrence.kind == CALL {
            assert_eq!(occurrence.owner.as_ref().map(|owner| owner.name.as_str()), Some("run"), "{cell}: body-call owner");
        }
        assert_eq!(occurrence.target.id, None, "{cell}: no target ID: {occurrence:#?}");
        assert_eq!(occurrence.candidates.is_empty(), occurrence.confidence != Confidence::NameOnly, "{cell}: {occurrence:#?}");
        assert!(
            occurrence.candidates.iter().all(|candidate| candidate.name == occurrence.target.name),
            "{cell}: candidates of {occurrence:#?}"
        );
        if every_kind {
            let expected: HashSet<&SymbolId> = if occurrence.confidence == Confidence::NameOnly {
                index.symbols.iter().filter(|record| record.identity.name == occurrence.target.name).map(|record| &record.id).collect()
            } else {
                HashSet::new()
            };
            let candidates: HashSet<&SymbolId> = occurrence.candidates.iter().filter_map(|candidate| candidate.id.as_ref()).collect();
            assert_eq!(candidates, expected, "{cell}: candidates of {occurrence:#?}");
        }
    }
    match row.owner {
        Some((owner, dependencies, recursion)) => {
            let record = index.symbols.iter().find(|record| record.identity.name == owner);
            let Some(record) = record else {
                assert!(!every_kind, "{cell}: no `{owner}`");
                return;
            };
            let mut observed: Vec<(&str, Option<&str>)> = record
                .relations
                .dependencies
                .iter()
                .map(|dependency| (dependency.name.as_str(), dependency.qualified_name.as_deref()))
                .collect();
            observed.sort_unstable();
            assert_eq!(observed, dependencies, "{cell}: dependencies");
            let mut references: Vec<(&str, Option<&str>)> = record
                .relations
                .references
                .iter()
                .filter(|reference| reference.id.is_none())
                .map(|reference| (reference.name.as_str(), reference.qualified_name.as_deref()))
                .collect();
            references.sort_unstable();
            assert_eq!(references, dependencies, "{cell}: import references");
            if index.completed_passes.contains(&tree_hugger::AnalysisPass::Semantic) {
                assert_eq!(record.semantics.is_recursive, Some(recursion), "{cell}: recursion of `{owner}`");
            }
        }
        None => {
            for coverage in &evidence.reference_coverage {
                assert_eq!(coverage.state, CoverageState::Limited, "{cell}");
                assert!(coverage.reasons.contains(&LimitReason::UnsupportedLanguage), "{cell}");
            }
            for record in &index.symbols {
                assert!(record.relations.references.is_empty(), "{cell}: `{}` has edges", record.identity.name);
                assert!(record.relations.dependencies.is_empty(), "{cell}: `{}` has dependencies", record.identity.name);
                if record.semantics.is_recursive.is_some() {
                    assert_eq!(record.semantics.is_recursive, Some(TriState::Unknown), "{cell}: `{}`", record.identity.name);
                }
            }
        }
    }
}

#[test]
fn comments_never_change_what_an_import_binds_or_reads() {
    for_each_import_row(|row| {
        let file = Analyzed::new(row.source, row.extension);
        assert_commented_import_row(row, row.label, &file.index, &file.evidence, true);
    });
}

/// The diagnostics reference reader (`TreeFile::referenced_symbols`) agrees:
/// an imported token is never a reference, and every read the occurrences
/// hold is one (but a TS/JS/Go/Rust member callee, which that reader never
/// reports).
#[test]
fn commented_imports_are_never_referenced_symbols() {
    for_each_import_row(|row| {
        let mut file = tempfile::Builder::new().suffix(&format!(".{}", row.extension)).tempfile().expect("temp source");
        file.write_all(row.source.as_bytes()).expect("write source");
        let tree_file = TreeFile::new(file.path()).expect("parse");
        assert!(tree_file.syntax_diagnostics().is_empty(), "{}: parses cleanly", row.label);
        let referenced = tree_file.referenced_symbols().expect("references");
        let references: HashSet<(usize, &str)> =
            referenced.iter().map(|reference| (reference.range.start_byte, reference.name.as_str())).collect();
        for &(name, _, _, _, line, column) in row.imports {
            let start = byte_at(row.source, line, column);
            assert!(!references.contains(&(start, name)), "{}: the imported `{name}` at {line}:{column} is a reference", row.label);
        }
        for &(line, column, name, path, ..) in row.reads {
            // The diagnostics reader reports no invoked member name in
            // the languages whose reference queries capture one only as a
            // callee.
            if path.is_some() && matches!(row.extension, "ts" | "js" | "go" | "rs") {
                continue;
            }
            let start = byte_at(row.source, line, column);
            assert!(references.contains(&(start, name)), "{}: the read `{name}` at {line}:{column} is no reference", row.label);
        }
    });
}

/// C, C++, Lua, Perl, Bash and Zsh have no import-definition capture, so no
/// comment can be mistaken for an imported name there (review row
/// `R8-F1.11`).
#[test]
fn languages_without_import_syntax_capture_no_imports() {
    use tree_hugger::queries::{GrammarRef, QueryKind, query_for};
    for language in ProgrammingLanguage::all() {
        let grammar = language.tree_sitter_language();
        let grammar_ref = GrammarRef { language, grammar: &grammar, id: language.query_name() };
        let query = query_for(grammar_ref, QueryKind::Imports).expect("imports query compiles");
        let captures_imports = query.capture_names().contains(&"local.definition.import");
        let without = matches!(
            language,
            ProgrammingLanguage::C
                | ProgrammingLanguage::Cpp
                | ProgrammingLanguage::Lua
                | ProgrammingLanguage::Perl
                | ProgrammingLanguage::Bash
                | ProgrammingLanguage::Zsh
        );
        assert_eq!(captures_imports, !without, "{language}: import-definition capture");
    }
}

#[test]
fn commented_swift_constructors_keep_conservative_evidence() {
    let file = Analyzed::new("class C<T> {}\nfunc run() {\n _ = C /* n */ <Int>()\n _ = C/*n*/<Int>()\n _ = N.C /* n */ <Int>()\n _ = C<Int /* n */>()\n}\n", "swift");
    let class = file.id("C", 1);
    let run = file.record("run", 2);
    for line in 3..=6 {
        let call = file.at(line, "C", 0);
        assert_eq!(call.kind, CALL);
        assert_eq!(file.text(call), "C");
        assert_eq!(call.confidence, Confidence::NameOnly);
        assert_eq!(call.target.id, None);
        assert_eq!(call.candidates.len(), 1);
        assert_eq!(call.candidates[0].id.as_ref(), Some(class));
        assert_eq!(call.owner.as_ref().and_then(|owner| owner.id.as_ref()), Some(&run.id));
        assert_eq!(file.at(line, "Int", 0).kind, USAGE);
    }
    assert_eq!(file.at(5, "C", 0).target.qualified_name.as_deref(), Some("N.C"));
    assert_eq!(run.semantics.is_recursive, Some(TriState::Unknown));
    assert!(run.relations.references.is_empty());
    assert!(file.evidence.reference_coverage.iter().all(|region| region.state == CoverageState::Limited && region.reasons.contains(&LimitReason::UnsupportedLanguage)));
}

#[test]
fn commented_patterns_and_asserted_eval_keep_lookup_boundaries() {
    for source in [
        "fn f() {}\nfn g(x: Option<fn()>) { match x { /* arm */ Some(/* binder */ f) => f(), _ => () } }\n",
        "fn f() {}\nfn g(x: Option<fn()>) { match x { Some(ref /* binder */ f) => f(), _ => () } }\n",
    ] {
        let file = Analyzed::new(source, "rs");
        let call = file.all("f").into_iter().find(|occ| occ.kind == CALL).expect("body call");
        assert_eq!(call.confidence, Confidence::Scoped);
        assert_ne!(call.target.id.as_ref(), Some(file.id("f", 1)));
        assert!(call.candidates.is_empty());
    }
    let file = Analyzed::new("function f() {}\nfunction g() { (<any /* n */> /* b */ eval /* a */)(s); f(); }\n", "ts");
    assert_eq!(file.at(2, "eval", 0).kind, CALL);
    assert_uncertain(file.at(2, "f", 0));
    assert_eq!(file.at(2, "f", 0).target.id, None);
}

#[test]
fn commented_swift_implicit_paths_preserve_written_qualifiers() {
    for (expression, path) in [
        ("-. /* n */ some.f()", ". /* n */ some.f"),
        ("-.some /* n */ .f()", ".some /* n */ .f"),
        ("!. /* n */ some.f()", ". /* n */ some.f"),
    ] {
        let file = Analyzed::new(&format!("func run() {{ _ = {expression} }}"), "swift");
        let call = file.at(1, "f", 0);
        assert_eq!(call.kind, CALL);
        assert_eq!(call.target.qualified_name.as_deref(), Some(path));
        assert_eq!(file.at(1, "some", 0).kind, USAGE);
    }
}

#[test]
fn commented_rust_generic_arguments_keep_name_and_primitive_roles() {
    let file = Analyzed::new("fn f<T>() {}\nfn g() { f /* n */ :: /* n */ <i32>(); f /* n */ :: /* n */ <Named>(); }\n", "rs");
    assert_eq!(file.at(2, "f", 0).kind, CALL);
    assert_eq!(file.at(2, "f", 1).kind, CALL);
    assert_eq!(file.at(2, "Named", 0).kind, USAGE);
    assert!(file.all("i32").is_empty(), "primitive_type tokens are not identifier references");
}


pub(crate) const TYPE_ONLY_IMPORT_MATRIX: &[(&str, &str, &str, bool)] = &[
    ("R9-F2.01", "import type { F } from './types';", "F", false),
    ("R9-F2.02", "import type { F as Alias } from './types';", "Alias", false),
    ("R9-F2.03", "import type Def from './types';", "Def", false),
    ("R9-F2.04", "import { type F } from './types';", "F", false),
    ("R9-F2.05", "import { type F as Alias } from './types';", "Alias", false),
    ("R9-F2.06", "import { type F as Alias, G as g } from './types';", "Alias", true),
    ("R9-F2.07.statement", "import /* a */ type /* b */ { F /* c */ as /* d */ Alias } from './types';", "Alias", false),
    ("R9-F2.07.specifier", "import { /* a */ type /* b */ F /* c */ as /* d */ Alias } from './types';", "Alias", false),
    ("R9-F2.07.default", "import /* a */ type /* b */ Def from './types';", "Def", false),
    ("R9-F2.08", "import type { F } from './types';", "F", false),
];

pub(crate) fn type_only_import_source(import: &str, name: &str, mixed: bool) -> String {
    let other = if mixed { " g();" } else { "" };
    format!("{import}\nfunction {name}(): void {{ {name}();{other} }}\nlet value: {name};\ntype ValueType = typeof {name};\n")
}

pub(crate) fn check_type_only_import_index(index: &FileSymbolIndex, name: &str, mixed: bool, semantics: bool) {
    assert_eq!(index.occurrences.iter().filter(|occ| occ.target.name == name).count(), 3, "call, annotation, and typeof only");
    let call = index.occurrences.iter().find(|occ| occ.target.name == name && occ.kind == CALL).expect("local call");
    let id = call.owner.as_ref().and_then(|owner| owner.id.as_ref()).expect("function owner ID");
    assert_scoped_to(call, id);
    let annotation = index.occurrences.iter().find(|occ| occ.target.name == name && occ.span.start.line == 3).expect("type read");
    assert_eq!(annotation.kind, USAGE);
    assert_eq!(annotation.confidence, Confidence::Scoped);
    assert_eq!(annotation.target.id, None, "type annotation retains the external binding");
    assert!(annotation.candidates.is_empty());
    let value_query = index.occurrences.iter().find(|occ| occ.target.name == name && occ.span.start.line == 4).expect("typeof value read");
    assert_scoped_to(value_query, id);
    assert_eq!(value_query.kind, USAGE);
    assert!(index.reference_coverage.iter().all(|region| region.state == CoverageState::Complete));
    if let Some(record) = index.symbols.iter().find(|record| &record.id == id) {
        assert_eq!(record.relations.references.iter().filter(|target| target.id.as_ref() == Some(id)).count(), 1);
        assert_eq!(record.relations.referenced_by.iter().filter(|owner| owner.id.as_ref() == Some(id)).count(), 1);
        assert_eq!(record.relations.dependencies.len(), usize::from(mixed));
        if semantics { assert_eq!(record.semantics.is_recursive, Some(TriState::Yes)); }
        if mixed {
            assert_eq!(record.relations.dependencies[0].name, "g");
            assert_eq!(record.relations.dependencies[0].qualified_name.as_deref(), Some("G"));
            assert_eq!(record.relations.dependencies[0].id, None);
        }
    }
    if mixed {
        let call = index.occurrences.iter().find(|occ| occ.target.name == "g").expect("ordinary value import call");
        assert_eq!(call.kind, CALL);
        assert_eq!(call.confidence, Confidence::Scoped);
        assert_eq!(call.target.id, None);
    }
}

#[test]
fn type_only_imports_preserve_local_values_and_external_type_reads() {
    for &(label, import, name, mixed) in TYPE_ONLY_IMPORT_MATRIX {
        let source = type_only_import_source(import, name, mixed);
        let file = Analyzed::new(&source, "ts");
        check_type_only_import_index(&file.index, name, mixed, true);
        assert_eq!(file.index.imports.len(), if mixed { 2 } else { 1 }, "{label}");
    }
}

#[test]
fn type_only_namespace_require_and_ordinary_import_controls() {
    for import in ["import type * as NS from './types';", "import type NS = require('./types');",
        "import /* a */ type /* b */ * as /* c */ NS from './types';",
        "import /* a */ type /* b */ NS = require('./types');"] {
        let file = Analyzed::new(&format!("{import}\nlet value: NS.F;\nfunction run() {{}}\n"), "ts");
        let namespace = file.at(2, "NS", 0);
        assert_eq!(namespace.confidence, Confidence::Scoped);
        assert_eq!(namespace.target.id, None);
        assert_eq!(namespace.kind, USAGE);
        assert_eq!(file.at(2, "F", 0).target.qualified_name.as_deref(), Some("NS.F"));
        assert_ne!(file.at(2, "F", 0).confidence, Confidence::Scoped);
        assert_eq!(file.record("run", 3).semantics.is_recursive, Some(TriState::No));
    }
    let file = Analyzed::new("import { type as g } from './types';\nfunction run() { g(); }\n", "ts");
    assert_eq!(file.at(2, "g", 0).confidence, Confidence::Scoped);
    assert_eq!(file.at(2, "g", 0).target.id, None);
    assert_eq!(file.record("run", 2).relations.dependencies[0].name, "g");
    assert_eq!(file.record("run", 2).relations.dependencies[0].qualified_name.as_deref(), Some("type"));
    assert_eq!(file.record("run", 2).semantics.is_recursive, Some(TriState::No));
    let file = Analyzed::new("import type { T } from './types';\nimport { G as g } from './types';\nfunction f(x: T) { f(x); g(); }\n", "ts");
    assert_scoped_to(file.at(3, "f", 0), file.id("f", 3));
    assert_eq!(file.at(3, "T", 0).confidence, Confidence::Scoped);
    assert_eq!(file.at(3, "T", 0).target.id, None);
    assert_eq!(file.record("f", 3).semantics.is_recursive, Some(TriState::Yes));
    assert_eq!(file.record("f", 3).relations.dependencies.iter().map(|target| target.name.as_str()).collect::<Vec<_>>(), ["T", "g"]);
    for import in ["import { F as g } from './types';", "import g from './types';", "import * as g from './types';"] {
        let file = Analyzed::new(&format!("{import}\nfunction run() {{ g(); }}\n"), "js");
        assert_eq!(file.at(2, "g", 0).confidence, Confidence::Scoped);
        assert_eq!(file.at(2, "g", 0).target.id, None);
        assert_eq!(file.record("run", 2).semantics.is_recursive, Some(TriState::No));
    }
}

/// Selected import identities, including aliases in unrelated nested scopes
/// and an unreported local binding that reuses an import's spelling.
pub(crate) const IMPORT_PROVENANCE_MATRIX: &[(&str, &str, &str, &str, &str, bool)] = &[
    ("R9-F4.01.rs", "aliases.rs", "mod other { pub fn g() {} pub fn f() {} }\nfn f() { use other::g as q; q(); }\nfn h() { use other::f as q; q(); }", "f", "q", false),
    ("R9-F4.01.rs.renamed", "aliases.rs", "mod other { pub fn g() {} pub fn f() {} }\nfn f() { use other::g as q; q(); }\nfn h() { use other::f as r; r(); }", "f", "q", false),
    ("R9-F4.02.ts", "aliases.ts", "namespace N { export function other() {} export function S() {} }\nimport D = N.other;\nfunction S() { D(); }\nnamespace M { import D = N.S; }", "S", "D", false),
    ("R9-F4.02.ts.renamed", "aliases.ts", "namespace N { export function other() {} export function S() {} }\nimport D = N.other;\nfunction S() { D(); }\nnamespace M { import E = N.S; }", "S", "D", false),
    ("R9-F4.03.ts", "shadow.ts", "import { G as D } from './types';\nfunction run(xs: (() => void)[]) { D(); for (const D of xs) { D(); } }", "run", "D", true),
    ("R9-F4.03.js", "shadow.js", "import { G as D } from './types';\nfunction run(xs) { D(); for (const D of xs) { D(); } }", "run", "D", true),
    ("R9-F4.03.rs", "shadow.rs", "use other::G as D;\nfn run(xs: Option<fn()>) { D(); match xs { Some(mut D) => D(), _ => () } }", "run", "D", true),
    ("R9-F4.05.rs.self", "self.rs", "fn f() { use other::f as q; q(); }", "f", "q", false),
    ("R9-F4.05.ts.self", "self.ts", "namespace N { export function S() {} }\nimport D = N.S;\nfunction S() { D(); }", "S", "D", false),
    ("R9-F4.05.rs.raw", "raw_import.rs", "fn f() { use other::g as r#q; q(); }", "f", "q", false),
    ("R9-F4.05.ts.escaped", "escaped_import.ts", r"import { G as \u0044 } from './types'; function run() { D(); }", "run", "D", false),
    ("R9-F4.05.js.escaped", "escaped_import.js", r"import { G as \u0044 } from './types'; function run() { D(); }", "run", "D", false),
    ("R9-F4.05.js.unique", "unique.js", "import { G as D } from './types';\nfunction run() { D(); }", "run", "D", false),
];

pub(crate) fn check_import_provenance(index: &FileSymbolIndex, label: &str, owner: &str, name: &str, shadow: bool, semantics: bool) {
    let calls: Vec<_> = index.occurrences.iter().filter(|o| o.kind == OccurrenceKind::Call && o.target.name == name && o.owner.as_ref().is_some_and(|r| r.name == owner)).collect();
    assert_eq!(calls.len(), if shadow { 2 } else { 1 }, "{label}: actual sites");
    let call = calls[0];
    let import = index.imports.iter().find(|i| Some(&i.span) == call.import_span.as_ref()).expect("selected import");
    let expected_original = if label.contains(".self") { if owner == "f" { "f" } else { "N.S" } }
        else if owner == "f" { "g" } else if owner == "S" { "N.other" } else { "G" };
    assert_eq!(import.symbol.qualified_name.as_deref(), Some(expected_original), "{label}");
    for call in &calls {
        assert_eq!(call.confidence, Confidence::Scoped, "{label}");
        assert!(call.target.id.is_none());
        assert!(call.candidates.is_empty());
        assert_eq!(call.owner, calls[0].owner, "same owner, distinct sites");
    }
    if shadow {
        assert!(calls[1].import_span.is_none(), "{label}: local binding never an import");
        assert_ne!(calls[0].span, calls[1].span);
    }
    if let Some(record) = index.symbols.iter().find(|r| Some(&r.id) == call.owner.as_ref().and_then(|r| r.id.as_ref())) {
        assert_eq!(record.relations.dependencies.len(), 1, "{label}: local call adds no dependency");
        assert_eq!(record.relations.dependencies[0].qualified_name.as_deref(), Some(expected_original));
        assert_eq!(record.relations.references.iter().filter(|r| r.id.is_none()).collect::<Vec<_>>(), record.relations.dependencies.iter().collect::<Vec<_>>(), "only bound import forms an absent-ID edge");
        if semantics {
            assert_eq!(record.semantics.is_recursive, Some(if shadow || label.contains(".self") { TriState::Unknown } else { TriState::No }), "{label}: recursion from selected original");
        }
    }
}

#[test]
fn selected_import_provenance_keeps_other_scopes_and_locals_independent() {
    for &(label, file, source, owner, name, shadow) in IMPORT_PROVENANCE_MATRIX {
        let dir = tempfile::TempDir::new().expect("temp");
        let path = dir.path().join(file);
        std::fs::write(&path, source).expect("source");
        let index = TreeFile::new(&path).expect("tree").symbol_index_v2().expect("index");
        check_import_provenance(&index, label, owner, name, shadow, true);
    }
}

crate::language_cases::language_tests!(unsupported_language_occurrences_never_claim_selected_imports => check_unsupported_language_occurrences_never_claim_selected_imports);

fn check_unsupported_language_occurrences_never_claim_selected_imports(language: ProgrammingLanguage) {
    for language in [language].into_iter().filter(|language| !matches!(language, ProgrammingLanguage::Rust | ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript)) {
        let path = primary_fixture(language).expect("primary fixture");
        let index = TreeFile::new(path).expect("tree").symbol_index_v2().expect("index");
        assert!(!index.occurrences.is_empty(), "{language:?}: positive occurrence control");
        for occurrence in &index.occurrences {
            assert_ne!(occurrence.confidence, Confidence::Scoped, "{language:?}");
            assert!(occurrence.import_span.is_none(), "{language:?}");
            assert!(occurrence.target.id.is_none(), "{language:?}");
        }
        assert!(index.reference_coverage.iter().all(|r| r.state == CoverageState::Limited && r.reasons.contains(&LimitReason::UnsupportedLanguage)), "{language:?}");
        for record in &index.symbols {
            assert!(record.relations.references.is_empty());
            assert!(record.relations.dependencies.is_empty());
            if matches!(record.kind, SymbolKindV2::Function | SymbolKindV2::Method | SymbolKindV2::Constructor) {
                assert_eq!(record.semantics.is_recursive, Some(TriState::Unknown), "{language:?}");
            }
        }
    }
}

fn assert_candidate_order(index: &FileSymbolIndex, evidence: &ReferenceEvidence, context: &str) {
    let key = |id: &SymbolId| {
        let record = index.symbols.iter().find(|record| &record.id == id).expect("candidate is a record");
        (record.source.name_span.as_ref().map_or(0, |span| span.start_byte), record.id.0.clone())
    };
    for occurrence in &evidence.occurrences {
        let keys: Vec<_> = occurrence.candidates.iter().map(|candidate| key(candidate.id.as_ref().expect("candidate ID"))).collect();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{context}: {occurrence:#?}");
    }
}

crate::language_cases::language_tests!(candidate_order_in_every_fixture => check_fixture_candidate_order);

fn check_fixture_candidate_order(language: ProgrammingLanguage) {
    let path = primary_fixture(language).expect("fixture");
    let tree_file = TreeFile::new(&path).expect("parse");
    let evidence = tree_file.reference_evidence().expect("evidence");
    let index = tree_file.symbol_index_v2().expect("index");
    assert_candidate_order(&index, &evidence, language.name());
}
