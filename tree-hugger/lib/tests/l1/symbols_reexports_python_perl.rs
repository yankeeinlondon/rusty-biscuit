//! Re-export extraction tests: Python and Perl.
//!
//! Python reports `declared` entries (literal `__all__`, redundant aliases,
//! star imports) and `exposed` entries (every public import of a module with
//! no `__all__`). Perl reports names in literal Exporter lists that the same
//! package binds through `use Module qw(..)`. The audit samples under
//! `tests/fixtures/reexports/` are the fixtures.

use tree_hugger::{
    ReexportCoverageState as State, ReexportEntry, ReexportEvidence as Ev, ReexportForm as Form,
    ReexportLimitReason as Why, ReexportReport,
};

use crate::reexports_support::{Snippet, snippet};

/// The text of one audit sample, read at compile time inside the test that
/// uses it.
macro_rules! sample {
    ($lang:literal, $file:literal) => {
        include_str!(concat!(
            "../fixtures/reexports/",
            $lang,
            "/",
            $file
        ))
    };
}

/// Every `(file, text)` pair of a list of samples of one language.
macro_rules! samples {
    ($lang:literal: $($file:literal),* $(,)?) => {
        [$(($file, sample!($lang, $file))),*]
    };
}

/// What one entry must say; unspecified roles are checked to be empty.
struct Want {
    form: Form,
    evidence: Ev,
    exported: Option<&'static str>,
    original: Option<&'static str>,
    source: Option<&'static str>,
    scope: Option<&'static str>,
    local: Option<&'static str>,
    conditional: bool,
    linked: bool,
    span: &'static str,
    statement: &'static str,
}

impl Want {
    fn conditional(mut self) -> Self {
        self.conditional = true;
        self
    }
}

fn py_named(
    evidence: Ev,
    exported: &'static str,
    original: &'static str,
    source: &'static str,
    span: &'static str,
    statement: &'static str,
) -> Want {
    Want {
        form: Form::Named,
        evidence,
        exported: Some(exported),
        original: Some(original),
        source: Some(source),
        scope: None,
        local: None,
        conditional: false,
        linked: true,
        span,
        statement,
    }
}

fn py_module(
    evidence: Ev,
    exported: &'static str,
    source: &'static str,
    span: &'static str,
    statement: &'static str,
) -> Want {
    Want { form: Form::Namespace, original: None, ..py_named(evidence, exported, "", source, span, statement) }
}

fn py_star(source: &'static str, statement: &'static str) -> Want {
    Want {
        form: Form::Wildcard,
        exported: None,
        original: None,
        ..py_named(Ev::Declared, "", "", source, "*", statement)
    }
}

/// A Perl export: `exported == original == local`, always `declared`. Perl
/// has no import records, so there is no import span.
fn pl(
    name: &'static str,
    source: &'static str,
    scope: Option<&'static str>,
    span: &'static str,
    statement: &'static str,
) -> Want {
    Want {
        form: Form::Named,
        evidence: Ev::Declared,
        exported: Some(name),
        original: Some(name),
        source: Some(source),
        scope,
        local: Some(name),
        conditional: false,
        linked: false,
        span,
        statement,
    }
}

fn check(snip: &Snippet, entry: &ReexportEntry, want: &Want) {
    let context = format!("entry for span {:?}", snip.text(&entry.span));
    assert_eq!(entry.form, want.form, "{context}: form");
    assert_eq!(entry.evidence, want.evidence, "{context}: evidence");
    assert_eq!(entry.exported_name.as_deref(), want.exported, "{context}: exported");
    assert_eq!(entry.original_name.as_deref(), want.original, "{context}: original");
    assert_eq!(entry.local_name.as_deref(), want.local, "{context}: local");
    assert_eq!(entry.source.as_deref(), want.source, "{context}: source");
    assert_eq!(entry.scope.as_deref(), want.scope, "{context}: scope");
    assert_eq!(entry.conditional, want.conditional, "{context}: conditional");
    assert_eq!(entry.restriction, None, "{context}: restriction");
    assert!(!entry.type_only, "{context}: type_only");
    assert!(entry.excluded_names.is_empty(), "{context}: excluded");
    assert!(entry.caveats.is_empty(), "{context}: caveats");
    assert_eq!(snip.text(&entry.span), want.span, "{context}: span text");
    assert_eq!(snip.text(&entry.statement_span), want.statement, "{context}: statement text");
    if want.linked {
        let import_span = entry.import_span.as_ref().unwrap_or_else(|| panic!("{context}: import_span"));
        let record = snip.indexed().imports.into_iter().find(|record| &record.span == import_span);
        assert!(record.is_some(), "{context}: import_span matches an import record");
    } else {
        assert!(entry.import_span.is_none(), "{context}: import_span");
    }
}

fn check_all(snip: &Snippet, report: &ReexportReport, wants: &[Want]) {
    assert_eq!(report.entries.len(), wants.len(), "entry count: {:#?}", report.entries);
    for (entry, want) in report.entries.iter().zip(wants) {
        check(snip, entry, want);
    }
}

fn complete(report: &ReexportReport) {
    assert_eq!(report.coverage.state, State::Complete, "limits: {:?}", report.coverage.limits);
    assert!(report.coverage.limits.is_empty());
}

fn limits(snip: &Snippet, report: &ReexportReport) -> Vec<(Why, String)> {
    report.coverage.limits.iter().map(|limit| (limit.reason, snip.text(&limit.span).to_owned())).collect()
}

/// `(import name span text, reexported)` for each import record, in order.
fn markers(snip: &Snippet) -> Vec<(String, bool)> {
    let index = snip.indexed();
    assert!(index.reexports.is_some(), "the planner computed the report");
    index.imports.iter().map(|record| (snip.text(&record.span).to_owned(), record.reexported)).collect()
}

fn run(file: &str, source: &str) -> (Snippet, ReexportReport) {
    let snip = snippet(file, source);
    let report = snip.report();
    (snip, report)
}

// ---------------------------------------------------------------- Python

#[test]
fn python_plain_from_import_is_exposed_and_never_marks_the_import() {
    let (snip, report) = run("m.py", sample!("python", "pos-from-import.py"));
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a")]);
    assert_eq!(markers(&snip), [("a".to_owned(), false)]);
}

#[test]
fn python_from_import_alias_exports_the_alias_and_keeps_the_original() {
    let (snip, report) = run("m.py", sample!("python", "pos-from-import-alias.py"));
    complete(&report);
    check_all(
        &snip,
        &report,
        &[py_named(Ev::Exposed, "b", "a", "m", "a as b", "from m import a as b")],
    );
}

#[test]
fn python_multi_and_parenthesized_lists_give_one_entry_per_name_in_order() {
    let (snip, report) = run("m.py", sample!("python", "pos-from-import-multi.py"));
    complete(&report);
    let statement = "from m import a, b as c, d";
    check_all(
        &snip,
        &report,
        &[
            py_named(Ev::Exposed, "a", "a", "m", "a", statement),
            py_named(Ev::Exposed, "c", "b", "m", "b as c", statement),
            py_named(Ev::Exposed, "d", "d", "m", "d", statement),
        ],
    );
    let (snip, report) = run("m.py", sample!("python", "pos-from-import-paren.py"));
    complete(&report);
    let statement = "from m import (a, b)";
    check_all(
        &snip,
        &report,
        &[
            py_named(Ev::Exposed, "a", "a", "m", "a", statement),
            py_named(Ev::Exposed, "b", "b", "m", "b", statement),
        ],
    );
}

#[test]
fn python_module_imports_are_namespaces_and_dotted_imports_expose_the_first_segment() {
    let (snip, report) = run("m.py", sample!("python", "pos-import-module.py"));
    complete(&report);
    check_all(&snip, &report, &[py_module(Ev::Exposed, "m", "m", "m", "import m")]);

    let (snip, report) = run("m.py", sample!("python", "pos-import-alias.py"));
    check_all(&snip, &report, &[py_module(Ev::Exposed, "n", "m", "m as n", "import m as n")]);

    let (snip, report) = run("m.py", sample!("python", "pos-import-dotted.py"));
    check_all(&snip, &report, &[py_module(Ev::Exposed, "pkg", "pkg.sub", "pkg.sub", "import pkg.sub")]);

    let (snip, report) = run("m.py", sample!("python", "pos-import-dotted-alias.py"));
    check_all(
        &snip,
        &report,
        &[py_module(Ev::Exposed, "s", "pkg.sub", "pkg.sub as s", "import pkg.sub as s")],
    );
}

#[test]
fn python_relative_sources_are_recorded_as_written() {
    let (snip, report) = run("m.py", sample!("python", "pos-relative.py"));
    check_all(&snip, &report, &[py_named(Ev::Exposed, "a", "a", ".m", "a", "from .m import a")]);
    let (snip, report) = run("m.py", sample!("python", "pos-relative-dot.py"));
    check_all(&snip, &report, &[py_named(Ev::Exposed, "x", "x", ".", "x", "from . import x")]);
    let (snip, report) = run("m.py", sample!("python", "pos-relative-parent.py"));
    check_all(
        &snip,
        &report,
        &[py_named(Ev::Exposed, "b", "a", "..m", "a as b", "from ..m import a as b")],
    );
}

#[test]
fn python_star_import_is_a_declared_wildcard_that_marks_the_import() {
    let (snip, report) = run("m.py", sample!("python", "pos-star.py"));
    complete(&report);
    check_all(&snip, &report, &[py_star("m", "from m import *")]);
    assert!(markers(&snip).iter().all(|(_, marked)| *marked));

    let (snip, report) = run("m.py", sample!("python", "pos-relative-star.py"));
    check_all(&snip, &report, &[py_star(".m", "from .m import *")]);
    assert!(markers(&snip).iter().all(|(_, marked)| *marked));

    // A star import is declared whatever `__all__` says, and carries conditions.
    let source = "try:\n    from m import *\nexcept ImportError:\n    pass\n__all__ = []\n";
    let (snip, report) = run("m.py", source);
    complete(&report);
    check_all(&snip, &report, &[py_star("m", "from m import *").conditional()]);
}

#[test]
fn python_literal_all_declares_the_imports_it_lists() {
    let listing = samples!("python":
        "pos-all-list.py", "pos-all-tuple.py", "pos-all-annotated.py", "pos-all-augassign.py",
        "pos-all-append.py", "pos-all-extend.py", "pos-all-concat-literal.py");
    for (file, source) in listing {
        let (snip, report) = run("m.py", source);
        assert_eq!(report.coverage.state, State::Complete, "{file}");
        assert_eq!(report.entries.len(), 1, "{file}");
        check(&snip, &report.entries[0], &py_named(Ev::Declared, "a", "a", "m", "a", "from m import a"));
        assert_eq!(markers(&snip), [("a".to_owned(), true)], "{file}");
    }
}

#[test]
fn python_names_outside_a_literal_all_are_not_reported() {
    let source = "from m import a, c\nimport q\n__all__ = [\"a\"]\n";
    let (snip, report) = run("m.py", source);
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "a", "a", "m", "a", "from m import a, c")]);
    assert_eq!(
        markers(&snip),
        [("a".to_owned(), true), ("c".to_owned(), false), ("q".to_owned(), false)]
    );
}

#[test]
fn python_all_remove_and_unbound_names() {
    let source = "from m import a, b\n__all__ = [\"a\", \"b\", \"zzz\"]\n__all__.remove(\"b\")\n";
    let (snip, report) = run("m.py", source);
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "a", "a", "m", "a", "from m import a, b")]);

    let (_, report) = run("m.py", "__all__ = [\"zzz\"]\n");
    complete(&report);
    assert!(report.entries.is_empty());
}

#[test]
fn python_redundant_alias_and_all_listing_of_one_site_collapse_to_one_entry() {
    let source = "import a as a\n__all__ = [\"a\", \"a\"]\n";
    let (snip, report) = run("m.py", source);
    complete(&report);
    check_all(&snip, &report, &[py_module(Ev::Declared, "a", "a", "a as a", "import a as a")]);
    assert_eq!(markers(&snip), [("a".to_owned(), true)]);
}

#[test]
fn python_redundant_aliases_are_declared_with_or_without_all() {
    let source = "from m import X as X\nimport q as q\nfrom . import y as y\nimport p.r as r\n";
    let (snip, report) = run("m.py", source);
    complete(&report);
    check_all(
        &snip,
        &report,
        &[
            py_named(Ev::Declared, "X", "X", "m", "X as X", "from m import X as X"),
            py_module(Ev::Declared, "q", "q", "q as q", "import q as q"),
            py_named(Ev::Declared, "y", "y", ".", "y as y", "from . import y as y"),
            // A different dotted name aliased to its last segment is not redundant.
            py_module(Ev::Exposed, "r", "p.r", "p.r as r", "import p.r as r"),
        ],
    );
    let marked: Vec<bool> = markers(&snip).iter().map(|(_, marked)| *marked).collect();
    assert_eq!(marked, [true, true, true, false]);

    // `__all__` present: redundant aliases stay declared, plain imports drop out.
    let (snip, report) = run("m.py", "from m import X as X, plain\n__all__ = []\n");
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "X", "X", "m", "X as X", "from m import X as X, plain")]);
}

#[test]
fn python_underscore_future_function_and_class_imports_are_not_reported() {
    for (file, source) in samples!("python":
        "neg-underscore.py", "neg-future.py", "neg-import-in-class.py", "neg-import-in-function.py")
    {
        let (_, report) = run("m.py", source);
        assert_eq!(report.coverage.state, State::Complete, "{file}");
        assert!(report.entries.is_empty(), "{file}: {:?}", report.entries);
    }
    // An explicit declaration of a private name is still a declaration.
    let (snip, report) = run("m.py", "from m import _p\n__all__ = [\"_p\"]\n");
    check_all(&snip, &report, &[py_named(Ev::Declared, "_p", "_p", "m", "_p", "from m import _p")]);
}

#[test]
fn python_all_inside_a_function_is_not_the_module_all() {
    let (snip, report) = run("m.py", sample!("python", "neg-all-in-function.py"));
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a")]);
}

#[test]
fn python_assignment_aliases_and_dynamic_imports_are_not_forwards() {
    let (snip, report) = run("m.py", sample!("python", "neg-assign-alias.py"));
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a")]);

    let (snip, report) = run("m.py", sample!("python", "neg-importlib.py"));
    complete(&report);
    check_all(&snip, &report, &[py_module(Ev::Exposed, "importlib", "importlib", "importlib", "import importlib")]);
}

#[test]
fn python_no_all_at_all_is_a_complete_exposure_report() {
    let (snip, report) = run("m.py", sample!("python", "neg-all-absent.py"));
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a")]);
}

#[test]
fn python_try_if_and_type_checking_imports_are_conditional_and_stay_separate() {
    let (snip, report) = run("m.py", sample!("python", "pos-try-import.py"));
    complete(&report);
    check_all(
        &snip,
        &report,
        &[
            py_named(Ev::Exposed, "a", "a", "fast", "a", "from fast import a").conditional(),
            py_named(Ev::Exposed, "a", "a", "slow", "a", "from slow import a").conditional(),
        ],
    );

    let (snip, report) = run("m.py", sample!("python", "pos-if-import.py"));
    check_all(
        &snip,
        &report,
        &[
            py_module(Ev::Exposed, "sys", "sys", "sys", "import sys"),
            py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a").conditional(),
        ],
    );

    let (snip, report) = run("m.py", sample!("python", "pos-import-typing-if.py"));
    check_all(
        &snip,
        &report,
        &[
            py_named(Ev::Exposed, "TYPE_CHECKING", "TYPE_CHECKING", "typing", "TYPE_CHECKING", "from typing import TYPE_CHECKING"),
            py_named(Ev::Exposed, "A", "A", "m", "A", "from m import A").conditional(),
        ],
    );

    // `with` always runs its body, so it is not a condition.
    let (snip, report) = run("m.py", "with ctx:\n    import a\n");
    check_all(&snip, &report, &[py_module(Ev::Exposed, "a", "a", "a", "import a")]);
}

#[test]
fn python_reassigned_or_redefined_import_limits_coverage_and_is_withheld() {
    let rebound = samples!("python": "neg-rebound.py", "neg-rebound-def.py", "neg-rebound-all.py");
    for (file, source) in rebound {
        let (snip, report) = run("m.py", source);
        assert_eq!(report.coverage.state, State::Limited, "{file}");
        assert!(report.entries.is_empty(), "{file}");
        assert_eq!(limits(&snip, &report), [(Why::ReboundBinding, "a".to_owned())], "{file}");
        assert_eq!(markers(&snip), [("a".to_owned(), false)], "{file}");
    }
    // The same through a conditional branch, the `except` fallback idiom.
    let (snip, report) = run("m.py", "try:\n    from fast import a\nexcept ImportError:\n    a = None\n");
    assert_eq!(limits(&snip, &report), [(Why::ReboundBinding, "a".to_owned())]);
    assert!(report.entries.is_empty());
}

#[test]
fn python_bindings_that_do_not_rebind_the_import_keep_it_reported() {
    // Two imports of one name, a definition before the import, a class
    // attribute and a function local are not module-scope rebinding after it.
    let source = "def a(): pass\nfrom m import a\nimport os\nimport os\nclass C:\n    os = 1\ndef f():\n    a = 1\n";
    let (snip, report) = run("m.py", source);
    complete(&report);
    assert_eq!(report.entries.len(), 3, "{:?}", report.entries);
    check(&snip, &report.entries[0], &py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a"));
}

#[test]
fn python_a_later_named_import_of_the_same_name_replaces_the_earlier_one() {
    // Control: the unmodified sample forwards its one import.
    let control = sample!("python", "pos-all-list.py");
    let (snip, report) = run("m.py", control);
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "a", "a", "m", "a", "from m import a")]);
    assert_eq!(markers(&snip), [("a".to_owned(), true)]);

    // One inserted import of the same name: only the final binding is
    // forwarded, and the replaced import is not marked.
    let replaced = control.replacen("from m import a\n", "from m import a\nfrom n import a\n", 1);
    let (snip, report) = run("m.py", &replaced);
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "a", "a", "n", "a", "from n import a")]);
    assert_eq!(markers(&snip), [("a".to_owned(), false), ("a".to_owned(), true)]);
}

#[test]
fn python_sequential_namespace_aliases_forward_only_the_final_module() {
    let (snip, report) = run("m.py", "import first as m\nimport second as m\n");
    complete(&report);
    check_all(&snip, &report, &[py_module(Ev::Exposed, "m", "second", "second as m", "import second as m")]);

    let (snip, report) = run("m.py", "import first as m\nimport second as m\n__all__ = [\"m\"]\n");
    complete(&report);
    check_all(&snip, &report, &[py_module(Ev::Declared, "m", "second", "second as m", "import second as m")]);
    assert_eq!(markers(&snip), [("m".to_owned(), false), ("m".to_owned(), true)]);

    // Two unaliased imports of one package bind the same module object.
    let (_, report) = run("m.py", "import pkg.a\nimport pkg.b\n__all__ = [\"pkg\"]\n");
    complete(&report);
    let sources: Vec<_> = report.entries.iter().map(|entry| entry.source.as_deref()).collect();
    assert_eq!(sources, [Some("pkg.a"), Some("pkg.b")]);
}

#[test]
fn python_fallback_imports_in_exclusive_branches_both_stay_conditional() {
    let source = "try:\n    from old import read\nexcept ImportError:\n    from new import read\n__all__ = [\"read\"]\n";
    let (snip, report) = run("m.py", source);
    complete(&report);
    check_all(
        &snip,
        &report,
        &[
            py_named(Ev::Declared, "read", "read", "old", "read", "from old import read").conditional(),
            py_named(Ev::Declared, "read", "read", "new", "read", "from new import read").conditional(),
        ],
    );
    assert_eq!(markers(&snip), [("read".to_owned(), true), ("read".to_owned(), true)]);

    let branches = "if fast:\n    from a import read\nelif slow:\n    from b import read\nelse:\n    from c import read\n";
    let (_, report) = run("m.py", branches);
    complete(&report);
    let sources: Vec<_> = report.entries.iter().map(|entry| entry.source.as_deref()).collect();
    assert_eq!(sources, [Some("a"), Some("b"), Some("c")]);

    // An unconditional import after the alternatives replaces both.
    let (snip, report) = run("m.py", &format!("{source}from final import read\n"));
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "read", "read", "final", "read", "from final import read")]);
    assert_eq!(
        markers(&snip),
        [("read".to_owned(), false), ("read".to_owned(), false), ("read".to_owned(), true)]
    );
}

#[test]
fn python_a_conditional_later_import_makes_the_earlier_one_a_rebound_limit() {
    let (snip, report) = run("m.py", "from old import read\nif fast:\n    from new import read\n__all__ = [\"read\"]\n");
    assert_eq!(limits(&snip, &report), [(Why::ReboundBinding, "read".to_owned())]);
    assert_eq!(report.coverage.limits[0].span.start_byte, 16, "the limit is on the earlier import");
    check_all(
        &snip,
        &report,
        &[py_named(Ev::Declared, "read", "read", "new", "read", "from new import read").conditional()],
    );
    assert_eq!(markers(&snip), [("read".to_owned(), false), ("read".to_owned(), true)]);

    // A try body followed by its `else` is sequential, not an alternative.
    let source = "try:\n    from old import read\nexcept ImportError:\n    pass\nelse:\n    from new import read\n";
    let (snip, report) = run("m.py", source);
    assert_eq!(limits(&snip, &report), [(Why::ReboundBinding, "read".to_owned())]);
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].source.as_deref(), Some("new"));

    // `finally` always runs after the body, so it replaces outright.
    let (_, report) = run("m.py", "try:\n    from old import read\nfinally:\n    from new import read\n");
    complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].source.as_deref(), Some("new"));
}

#[test]
fn python_star_import_then_a_named_import_keeps_both_forwards() {
    let (snip, report) = run("m.py", "from a import *\nfrom b import x\n");
    complete(&report);
    check_all(
        &snip,
        &report,
        &[py_star("a", "from a import *"), py_named(Ev::Exposed, "x", "x", "b", "x", "from b import x")],
    );
}

#[test]
fn python_computed_all_limits_coverage_and_yields_no_all_derived_entries() {
    let computed = samples!("python":
        "neg-all-call.py", "neg-all-computed.py", "neg-all-dunder-del.py", "neg-all-nonstring.py",
        "neg-all-splat.py", "neg-all-sub-assign.py");
    for (file, source) in computed {
        let (snip, report) = run("m.py", source);
        assert_eq!(report.coverage.state, State::Limited, "{file}");
        assert!(report.entries.iter().all(|entry| entry.evidence != Ev::Declared), "{file}");
        assert!(report.entries.is_empty(), "{file}: exposure is unknowable when __all__ is computed");
        let found = limits(&snip, &report);
        assert!(!found.is_empty() && found.iter().all(|(why, _)| *why == Why::ComputedExportList), "{file}");
    }
}

#[test]
fn python_all_set_under_a_condition_is_computed() {
    let (snip, report) = run("m.py", sample!("python", "neg-all-shadow-cond.py"));
    assert_eq!(
        limits(&snip, &report),
        [
            (Why::ComputedExportList, "__all__ = [\"a\"]".to_owned()),
            (Why::ComputedExportList, "__all__ = [\"b\"]".to_owned()),
        ]
    );
    assert!(report.entries.is_empty());
}

#[test]
fn python_all_edited_under_a_condition_is_computed_whatever_the_edit() {
    let guarded = [
        ("if append", "__all__ = []\nif enabled:\n    __all__.append(\"read\")\n", "__all__.append(\"read\")"),
        ("try append", "__all__ = []\ntry:\n    __all__.append(\"read\")\nexcept E:\n    pass\n", "__all__.append(\"read\")"),
        ("if extend", "__all__ = []\nif enabled:\n    __all__.extend([\"read\"])\n", "__all__.extend([\"read\"])"),
        ("if +=", "__all__ = []\nif enabled:\n    __all__ += [\"read\"]\n", "__all__ += [\"read\"]"),
        ("if assign", "__all__ = []\nif enabled:\n    __all__ = [\"read\"]\n", "__all__ = [\"read\"]"),
        ("if concat", "__all__ = []\nif enabled:\n    __all__ = [] + [\"read\"]\n", "__all__ = [] + [\"read\"]"),
        ("if remove", "__all__ = [\"read\"]\nif enabled:\n    __all__.remove(\"read\")\n", "__all__.remove(\"read\")"),
        ("and append", "__all__ = []\nenabled and __all__.append(\"read\")\n", "__all__.append(\"read\")"),
        ("def append", "__all__ = []\ndef f():\n    __all__.append(\"read\")\nf()\n", "__all__.append(\"read\")"),
        ("def global", "__all__ = []\ndef f():\n    global __all__\n    __all__ = [\"read\"]\n", "global __all__"),
        ("lambda", "__all__ = []\ng = lambda: __all__.append(\"read\")\n", "__all__.append(\"read\")"),
        ("comprehension", "__all__ = []\nx = [__all__.append(\"read\") for _ in y]\n", "__all__.append(\"read\")"),
    ];
    for (shape, list, edit) in guarded {
        let (snip, report) = run("m.py", &format!("from m import read\n{list}"));
        assert_eq!(limits(&snip, &report), [(Why::ComputedExportList, edit.to_owned())], "{shape}");
        assert!(report.entries.is_empty(), "{shape}: no list-derived forward");
        assert_eq!(markers(&snip), [("read".to_owned(), false)], "{shape}");
    }
}

#[test]
fn python_unguarded_append_and_remove_are_definite() {
    let (snip, report) = run("m.py", "from m import read\n__all__ = []\n__all__.append(\"read\")\n");
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "read", "read", "m", "read", "from m import read")]);
    assert_eq!(markers(&snip), [("read".to_owned(), true)]);

    let (snip, report) = run("m.py", "from m import read\n__all__ = [\"read\"]\n__all__.remove(\"read\")\n");
    complete(&report);
    assert!(report.entries.is_empty());
    assert_eq!(markers(&snip), [("read".to_owned(), false)]);
}

#[test]
fn python_computed_all_keeps_redundant_alias_and_star_declarations() {
    let source = "import q as q\nfrom m import a\nfrom n import *\n__all__ = make()\n";
    let (snip, report) = run("m.py", source);
    assert_eq!(limits(&snip, &report), [(Why::ComputedExportList, "__all__ = make()".to_owned())]);
    check_all(
        &snip,
        &report,
        &[py_module(Ev::Declared, "q", "q", "q as q", "import q as q"), py_star("n", "from n import *")],
    );
}

#[test]
fn python_all_extended_from_another_module_is_a_cross_file_limit() {
    let source = "from m import a, b\nimport sub\n__all__ = [\"a\"]\n__all__ += sub.__all__\n__all__.extend(sub.__all__)\n";
    let (snip, report) = run("m.py", source);
    assert_eq!(
        limits(&snip, &report),
        [
            (Why::CrossFileExportList, "__all__ += sub.__all__".to_owned()),
            (Why::CrossFileExportList, "__all__.extend(sub.__all__)".to_owned()),
        ]
    );
    // The literal part still counts.
    check_all(&snip, &report, &[py_named(Ev::Declared, "a", "a", "m", "a", "from m import a, b")]);
}

#[test]
fn python_parse_recovery_keeps_valid_entries_and_limits_only_damaged_forwarding_syntax() {
    // Unrelated damage: the entry survives and coverage stays complete.
    let (snip, report) = run("m.py", "from m import a\nx = = 3\n");
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a")]);

    let (snip, report) = run("m.py", "from m import a\n__all__ = [\"a\"]\nclass C(:\n  pass\n");
    complete(&report);
    check_all(&snip, &report, &[py_named(Ev::Declared, "a", "a", "m", "a", "from m import a")]);

    // Damaged import statement: limited, the intact import is still reported.
    let (snip, report) = run("m.py", "from m import a\nfrom n import (b,\n");
    assert_eq!(limits(&snip, &report), [(Why::ParseError, "from n import (b,".to_owned())]);
    check_all(&snip, &report, &[py_named(Ev::Exposed, "a", "a", "m", "a", "from m import a")]);

    // Damaged `__all__`: never a complete negative.
    let (snip, report) = run("m.py", "from m import a\n__all__ = [\"a\",\n");
    assert_eq!(limits(&snip, &report), [(Why::ParseError, "__all__ = [\"a\",".to_owned())]);

    let (snip, report) = run("m.py", "import\nfrom m import a\n");
    assert_eq!(limits(&snip, &report), [(Why::ParseError, "import".to_owned())]);
}

#[test]
fn python_entries_follow_source_order_and_the_query_path_agrees() {
    let source = "from z import z1\nimport a\nfrom m import b as bb\n";
    let (snip, report) = run("m.py", source);
    let order: Vec<_> = report.entries.iter().map(|entry| snip.text(&entry.span).to_owned()).collect();
    assert_eq!(order, ["z1", "a", "b as bb"]);
    assert_eq!(snip.indexed().reexports, Some(report));
}

#[test]
fn python_every_audit_sample_extracts_with_consistent_invariants() {
    let corpus = samples!("python": "neg-all-absent.py","neg-all-call.py","neg-all-computed.py","neg-all-dunder-del.py","neg-all-in-function.py","neg-all-nonstring.py","neg-all-shadow-cond.py","neg-all-splat.py","neg-all-sub-assign.py","neg-assign-alias.py","neg-future.py","neg-import-in-class.py","neg-import-in-function.py","neg-importlib.py","neg-rebound-all.py","neg-rebound-def.py","neg-rebound.py","neg-underscore.py","pos-all-annotated.py","pos-all-append.py","pos-all-augassign.py","pos-all-concat-literal.py","pos-all-extend.py","pos-all-list.py","pos-all-tuple.py","pos-from-import-alias.py","pos-from-import-multi.py","pos-from-import-paren.py","pos-from-import.py","pos-if-import.py","pos-import-alias.py","pos-import-dotted-alias.py","pos-import-dotted.py","pos-import-module.py","pos-import-typing-if.py","pos-relative-dot.py","pos-relative-parent.py","pos-relative-star.py","pos-relative.py","pos-star.py","pos-try-import.py");
    assert_eq!(corpus.len(), 41);
    for (file, source) in corpus {
        assert_invariants(file, &run(file, source).1);
    }
}

fn assert_invariants(file: &str, report: &ReexportReport) {
    let starts: Vec<_> = report.entries.iter().map(|entry| entry.span.start_byte).collect();
    assert!(starts.windows(2).all(|pair| pair[0] < pair[1]), "{file}: entries ascend with unique spans");
    let keys: Vec<_> = report.coverage.limits.iter().map(|l| (l.span.start_byte, l.reason, l.span.end_byte)).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{file}: limits ascend without duplicates");
    assert_eq!(
        report.coverage.state == State::Limited,
        !report.coverage.limits.is_empty(),
        "{file}: limited iff limits"
    );
    assert_ne!(report.coverage.state, State::NotApplicable, "{file}");
}

// ------------------------------------------------------------------ Perl

#[test]
fn perl_exporter_markers_all_unlock_the_same_export() {
    let helper = |span, statement| pl("helper", "Other::Mod", Some("My::Mod"), span, statement);
    let markers = [
        (sample!("perl", "pos-export-ok-imported.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-use-base.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-use-parent-qw.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-use-exporter-import.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-use-exporter-import-sq.pl"), "'helper'", "our @EXPORT_OK = ('helper')"),
        (sample!("perl", "pos-use-version-list.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-isa-push.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-vars-isa.pl"), "helper", "@EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-export-fail.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
        (sample!("perl", "pos-block-package.pl"), "helper", "our @EXPORT_OK = qw(helper)"),
    ];
    for (source, span, statement) in markers {
        let (snip, report) = run("My.pm", source);
        complete(&report);
        check_all(&snip, &report, &[helper(span, statement)]);
    }
}

#[test]
fn perl_default_and_optional_lists_both_report_in_source_order() {
    let (snip, report) = run("My.pm", sample!("perl", "pos-export-imported-isa.pl"));
    complete(&report);
    check_all(
        &snip,
        &report,
        &[
            pl("helper", "Other::Mod", Some("My::Mod"), "helper", "our @EXPORT = qw(helper)"),
            pl("other", "Other::Mod", Some("My::Mod"), "other", "our @EXPORT_OK = qw(other)"),
        ],
    );
}

#[test]
fn perl_push_onto_an_export_list_is_read() {
    let (snip, report) = run("My.pm", sample!("perl", "pos-push-export.pl"));
    complete(&report);
    check_all(
        &snip,
        &report,
        &[pl("helper", "Other::Mod", Some("My::Mod"), "helper", "push @EXPORT_OK, qw(helper)")],
    );
    let source = "package P;\nuse parent 'Exporter';\nuse Q qw(a);\npush(@EXPORT, 'a');\n";
    let (snip, report) = run("P.pm", source);
    complete(&report);
    check_all(&snip, &report, &[pl("a", "Q", Some("P"), "'a'", "push(@EXPORT, 'a')")]);
}

#[test]
fn perl_sigils_are_kept_except_the_sub_ampersand() {
    let (snip, report) = run("My.pm", sample!("perl", "pos-sigil-var.pl"));
    complete(&report);
    let statement = "our @EXPORT_OK = qw($config @list &func)";
    check_all(
        &snip,
        &report,
        &[
            pl("$config", "Other::Mod", Some("My::Mod"), "$config", statement),
            pl("@list", "Other::Mod", Some("My::Mod"), "@list", statement),
            pl("func", "Other::Mod", Some("My::Mod"), "&func", statement),
        ],
    );
}

#[test]
fn perl_parenthesized_list_reports_only_the_bound_names() {
    let (snip, report) = run("My.pm", sample!("perl", "neg-export-paren-list.pl"));
    complete(&report);
    check_all(
        &snip,
        &report,
        &[pl("helper", "Other::Mod", Some("My::Mod"), "\"helper\"", "our @EXPORT_OK = (\"helper\", \"other\")")],
    );
}

#[test]
fn perl_export_tags_add_a_name_only_when_no_list_has_it() {
    // The literal tag lists name nothing new here.
    let (snip, report) = run("My.pm", sample!("perl", "pos-export-tags-literal.pl"));
    complete(&report);
    assert_eq!(report.entries.len(), 2);
    for entry in &report.entries {
        assert_eq!(snip.text(&entry.statement_span), "our @EXPORT_OK = qw(helper other)");
    }
    let source = "package P;\nuse parent 'Exporter';\nuse Q qw(a b);\nour %EXPORT_TAGS = (all => [qw(a b)]);\n";
    let (snip, report) = run("P.pm", source);
    complete(&report);
    let statement = "our %EXPORT_TAGS = (all => [qw(a b)])";
    check_all(&snip, &report, &[pl("a", "Q", Some("P"), "a", statement), pl("b", "Q", Some("P"), "b", statement)]);
}

#[test]
fn perl_statement_modifiers_and_blocks_make_an_export_conditional() {
    let (snip, report) = run("My.pm", sample!("perl", "neg-export-cond.pl"));
    complete(&report);
    check_all(
        &snip,
        &report,
        &[pl("helper", "Other::Mod", Some("My::Mod"), "helper", "our @EXPORT_OK = qw(helper)").conditional()],
    );
    let source = "package P;\nuse parent 'Exporter';\nuse Q qw(a);\nif ($x) { our @EXPORT = qw(a); }\n";
    let (snip, report) = run("P.pm", source);
    complete(&report);
    check_all(&snip, &report, &[pl("a", "Q", Some("P"), "a", "our @EXPORT = qw(a)").conditional()]);
    let source = "package P;\nuse parent 'Exporter';\nuse Q qw(a);\nour @EXPORT = qw(a) unless $x;\n";
    let (snip, report) = run("P.pm", source);
    check_all(&snip, &report, &[pl("a", "Q", Some("P"), "a", "our @EXPORT = qw(a)").conditional()]);
}

#[test]
fn perl_guarded_push_unshift_and_tags_stay_conditional() {
    let guarded = [
        ("if ($x) { push @EXPORT_OK, 'a'; }\n", "'a'", "push @EXPORT_OK, 'a'"),
        ("push @EXPORT_OK, 'a' if $x;\n", "'a'", "push @EXPORT_OK, 'a'"),
        ("if ($x) { unshift @EXPORT, 'a'; }\n", "'a'", "unshift @EXPORT, 'a'"),
        ("if ($x) { our %EXPORT_TAGS = (all => [qw(a)]); }\n", "a", "our %EXPORT_TAGS = (all => [qw(a)])"),
    ];
    for (edit, span, statement) in guarded {
        let (snip, report) = run("P.pm", &format!("package P;\nuse parent 'Exporter';\nuse Q qw(a);\n{edit}"));
        complete(&report);
        check_all(&snip, &report, &[pl("a", "Q", Some("P"), span, statement).conditional()]);
    }
}

#[test]
fn perl_packages_are_matched_separately_and_main_has_no_scope() {
    let source = "package A;\nuse parent 'Exporter';\nuse X qw(f);\nour @EXPORT = qw(f);\n\
                  package B;\nuse parent 'Exporter';\nuse Y qw(g);\nour @EXPORT = qw(g f);\n";
    let (snip, report) = run("AB.pm", source);
    complete(&report);
    check_all(
        &snip,
        &report,
        &[
            pl("f", "X", Some("A"), "f", "our @EXPORT = qw(f)"),
            pl("g", "Y", Some("B"), "g", "our @EXPORT = qw(g f)"),
        ],
    );
    let source = "use parent 'Exporter';\nuse Q qw(a);\nour @EXPORT = qw(a);\n";
    let (snip, report) = run("main.pl", source);
    check_all(&snip, &report, &[pl("a", "Q", None, "a", "our @EXPORT = qw(a)")]);

    let (_, report) = run("My.pm", sample!("perl", "neg-export-other-package.pl"));
    complete(&report);
    assert!(report.entries.is_empty());
}

#[test]
fn perl_unreported_shapes_stay_complete_negatives() {
    let silent = samples!("perl":
        "neg-no-exporter.pl", "neg-local-sub.pl", "neg-export-not-imported.pl", "neg-use-empty-list.pl",
        "neg-use-pragma.pl");
    for (file, source) in silent {
        let (_, report) = run("My.pm", source);
        assert_eq!(report.coverage.state, State::Complete, "{file}");
        assert!(report.entries.is_empty(), "{file}: {:?}", report.entries);
    }
    // `no Module qw(..)` unimports; a plain `use Module;` with nothing listed is harmless.
    let source = "package P;\nuse parent 'Exporter';\nno Q qw(a);\nuse R;\nour @EXPORT = qw(a);\n";
    let (snip, report) = run("P.pm", source);
    assert_eq!(limits(&snip, &report), [(Why::CrossFileExportList, "use R;".to_owned())]);
    assert!(report.entries.is_empty());
    let (_, report) = run("P.pm", "package P;\nuse parent 'Exporter';\nuse R;\n");
    complete(&report);
}

#[test]
fn perl_later_sub_definition_of_an_imported_name_is_rebinding() {
    let (snip, report) = run("My.pm", sample!("perl", "neg-export-shadow-redef.pl"));
    assert_eq!(limits(&snip, &report), [(Why::ReboundBinding, "helper".to_owned())]);
    assert!(report.entries.is_empty());
    // A sub defined before the `use` is overwritten by the import.
    let source = "package P;\nuse parent 'Exporter';\nsub a { 1 }\nuse Q qw(a);\nour @EXPORT = qw(a);\n";
    let (snip, report) = run("P.pm", source);
    complete(&report);
    check_all(&snip, &report, &[pl("a", "Q", Some("P"), "a", "our @EXPORT = qw(a)")]);
}

#[test]
fn perl_computed_export_lists_limit_coverage() {
    let computed = samples!("perl":
        "neg-export-computed.pl", "neg-export-grep.pl", "neg-export-var-interp.pl", "amb-export-tags-deref.pl",
        "neg-export-in-sub.pl");
    for (file, source) in computed {
        let (snip, report) = run("My.pm", source);
        assert_eq!(report.coverage.state, State::Limited, "{file}");
        let found = limits(&snip, &report);
        assert_eq!(found.len(), 1, "{file}: {found:?}");
        assert_eq!(found[0].0, Why::ComputedExportList, "{file}");
        assert!(found[0].1.contains("EXPORT_OK"), "{file}");
    }
    let (_, report) = run("My.pm", sample!("perl", "neg-export-computed.pl"));
    assert!(report.entries.is_empty());
}

#[test]
fn perl_dynamic_import_routes_limit_coverage() {
    let dynamic = samples!("perl":
        "neg-custom-import.pl", "neg-export-to-level.pl", "amb-require-import.pl", "neg-export-glob.pl");
    for (file, source) in dynamic {
        let (_, report) = run("My.pm", source);
        assert_eq!(report.coverage.state, State::Limited, "{file}");
        assert!(report.coverage.limits.iter().all(|l| l.reason == Why::DynamicForwarding), "{file}");
    }
    let (snip, report) = run("My.pm", sample!("perl", "neg-export-glob.pl"));
    assert_eq!(limits(&snip, &report), [(Why::DynamicForwarding, "*helper = \\&Other::Mod::helper".to_owned())]);
    let (snip, report) = run("My.pm", sample!("perl", "amb-require-import.pl"));
    assert_eq!(limits(&snip, &report), [(Why::DynamicForwarding, "Other::Mod->import(\"helper\")".to_owned())]);
}

#[test]
fn perl_names_bound_without_a_literal_list_are_cross_file_limits() {
    let (snip, report) = run("My.pm", sample!("perl", "amb-use-no-list.pl"));
    assert_eq!(limits(&snip, &report), [(Why::CrossFileExportList, "use Other::Mod;".to_owned())]);
    assert!(report.entries.is_empty());
    let (snip, report) = run("My.pm", sample!("perl", "amb-use-tag-import.pl"));
    assert_eq!(limits(&snip, &report), [(Why::CrossFileExportList, "use Other::Mod qw(:all);".to_owned())]);
}

#[test]
fn perl_qualified_export_variables_are_a_grammar_gap_even_though_they_parse_cleanly() {
    let (snip, report) = run("My.pm", sample!("perl", "gap-qualified-var.pl"));
    assert_eq!(limits(&snip, &report), [(Why::GrammarGap, "@My::Mod::EXPORT_OK".to_owned())]);
    assert!(report.entries.is_empty());
    let source = "package P;\nuse parent 'Exporter';\nuse Q qw(a);\npush @P::EXPORT_OK, 'a';\n";
    let (snip, report) = run("P.pm", source);
    assert_eq!(limits(&snip, &report), [(Why::GrammarGap, "@P::EXPORT_OK".to_owned())]);
}

#[test]
fn perl_parse_recovery_keeps_valid_entries_and_limits_only_damaged_forwarding_syntax() {
    let source = "package P;\nuse parent 'Exporter';\nuse Q qw(a);\nmy $x = ;\nour @EXPORT_OK = qw(a);\n";
    let (snip, report) = run("P.pm", source);
    complete(&report);
    check_all(&snip, &report, &[pl("a", "Q", Some("P"), "a", "our @EXPORT_OK = qw(a)")]);

    let source = "package P;\nuse parent 'Exporter';\nuse Q qw(a);\nour @EXPORT_OK = qw(a\n";
    let (snip, report) = run("P.pm", source);
    assert_eq!(report.coverage.state, State::Limited);
    assert!(limits(&snip, &report).iter().all(|(why, _)| *why == Why::ParseError));
}

#[test]
fn perl_exports_never_mark_imports_they_cannot_link() {
    // Perl `use` bindings produce no import records, so nothing can be marked.
    let (snip, report) = run("My.pm", sample!("perl", "pos-export-ok-imported.pl"));
    complete(&report);
    assert!(report.entries.iter().all(|entry| entry.import_span.is_none()));
    assert!(markers(&snip).iter().all(|(_, marked)| !*marked));
}

#[test]
fn perl_every_audit_sample_extracts_with_consistent_invariants() {
    let corpus = samples!("perl": "amb-export-tags-deref.pl","amb-require-import.pl","amb-use-no-list.pl","amb-use-tag-import.pl","gap-qualified-var.pl","neg-custom-import.pl","neg-export-computed.pl","neg-export-cond.pl","neg-export-glob.pl","neg-export-grep.pl","neg-export-in-sub.pl","neg-export-not-imported.pl","neg-export-other-package.pl","neg-export-paren-list.pl","neg-export-shadow-redef.pl","neg-export-to-level.pl","neg-export-var-interp.pl","neg-local-sub.pl","neg-no-exporter.pl","neg-use-empty-list.pl","neg-use-pragma.pl","pos-block-package.pl","pos-export-fail.pl","pos-export-imported-isa.pl","pos-export-ok-imported.pl","pos-export-tags-literal.pl","pos-isa-push.pl","pos-push-export.pl","pos-sigil-var.pl","pos-use-base.pl","pos-use-exporter-import-sq.pl","pos-use-exporter-import.pl","pos-use-parent-qw.pl","pos-use-version-list.pl","pos-vars-isa.pl");
    assert_eq!(corpus.len(), 35);
    for (file, source) in corpus {
        assert_invariants(file, &run(file, source).1);
    }
}
