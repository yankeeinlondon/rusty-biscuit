//! Scope-aware reference occurrences and coverage
//! (`TreeFile::reference_evidence`): shadowing and declaration order,
//! receivers and overloads, spans and owners, aliases, and the supported or
//! limited state of every language. User contract:
//! `tree-hugger/docs/references.md`.

use std::collections::HashSet;
use std::io::Write;

use tree_hugger::{
    Confidence, CoverageState, FileSymbolIndex, LimitReason, Occurrence, OccurrenceKind, ProgrammingLanguage,
    ReferenceEvidence, SymbolId, SymbolRecord, TextSpan, TreeFile,
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
