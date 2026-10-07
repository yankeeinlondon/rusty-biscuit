//! Scope-aware reference occurrences and coverage
//! (`TreeFile::reference_evidence`): shadowing and declaration order,
//! receivers and overloads, spans and owners, aliases, and the supported or
//! limited state of every language. User contract:
//! `tree-hugger/docs/references.md`.

use std::collections::HashSet;
use std::io::Write;

use tree_hugger::{
    Confidence, CoverageState, FileSymbolIndex, LimitReason, Occurrence, OccurrenceKind, ProgrammingLanguage,
    ReferenceEvidence, SymbolId, SymbolRecord, SymbolScope, TextSpan, TreeFile, TriState,
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

#[test]
fn owner_target_and_candidate_ids_are_the_complete_index_ids() {
    for language in ProgrammingLanguage::all() {
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

#[test]
fn occurrences_and_coverage_are_in_canonical_order() {
    for language in ProgrammingLanguage::all() {
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


// ---- Invocation forms ----------------------------------------------------------

const CALL: OccurrenceKind = OccurrenceKind::Call;
const USAGE: OccurrenceKind = OccurrenceKind::Usage;

/// `(language, extension, source, [(line, name, nth on that line, kind)])`.
type InvocationRow = (
    ProgrammingLanguage,
    &'static str,
    &'static str,
    &'static [(u32, &'static str, usize, OccurrenceKind)],
);

/// Every invocation form each pinned grammar writes, direct and through
/// transparent wrappers (parentheses, TypeScript `as`/`satisfies`/`<T>x`/
/// `x!`), with the negative controls on the same lines: an argument, a
/// receiver, a dereference, a sequence and a conditional are never the
/// callee. Binding confidence may stay limited; only `kind` is checked.
const INVOCATION_MATRIX: &[InvocationRow] = &[
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
];

#[test]
fn member_accesses_that_are_not_the_callee_are_never_calls() {
    for &(language, extension, source, checks) in NOT_CALLED_MATRIX {
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

#[test]
fn invocation_forms_are_calls_in_every_language() {
    for language in ProgrammingLanguage::all() {
        assert!(
            INVOCATION_MATRIX.iter().any(|row| row.0 == language),
            "{}: no invocation row",
            language.name()
        );
    }
    for &(language, extension, source, checks) in INVOCATION_MATRIX {
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
];

#[test]
fn written_paths_keep_every_root_qualifier_and_receiver() {
    for language in ProgrammingLanguage::all() {
        assert!(
            WRITTEN_PATH_MATRIX.iter().any(|row| row.1 == language),
            "{}: no written-path row",
            language.name()
        );
    }
    for &(label, _, extension, source, checks) in WRITTEN_PATH_MATRIX {
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
