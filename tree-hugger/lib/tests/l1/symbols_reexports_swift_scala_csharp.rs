//! Re-export extraction tests for Swift (`@_exported import`), Scala 3
//! (`export`) and C# (`TypeForwardedTo`), through the public
//! `TreeFile::reexports` and `query` paths.

use tree_hugger::{
    ReexportCaveat, ReexportCoverageState, ReexportEntry, ReexportEvidence, ReexportForm,
    ReexportLimitReason, ReexportReport,
};

use crate::reexports_support::{Snippet, snippet};

/// One audit sample file, embedded by the invoking `#[test]`.
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

/// What an entry is expected to say; everything else must be empty/false.
#[derive(Default)]
struct Want<'a> {
    exported: Option<&'a str>,
    original: Option<&'a str>,
    local: Option<&'a str>,
    source: Option<&'a str>,
    excluded: &'a [&'a str],
    scope: Option<&'a str>,
    caveats: &'a [ReexportCaveat],
    conditional: bool,
    span: &'a str,
    statement: &'a str,
    linked_import: bool,
}

fn assert_entry(snip: &Snippet, got: &ReexportEntry, form: ReexportForm, want: Want<'_>) {
    assert_eq!(got.form, form, "form of {:?}", snip.text(&got.statement_span));
    assert_eq!(got.evidence, ReexportEvidence::Syntax);
    assert_eq!(got.exported_name.as_deref(), want.exported, "exported_name");
    assert_eq!(got.original_name.as_deref(), want.original, "original_name");
    assert_eq!(got.local_name.as_deref(), want.local, "local_name");
    assert_eq!(got.source.as_deref(), want.source, "source");
    assert_eq!(got.excluded_names, want.excluded, "excluded_names");
    assert_eq!(got.scope.as_deref(), want.scope, "scope");
    assert_eq!(got.restriction, None);
    assert!(!got.type_only);
    assert_eq!(got.conditional, want.conditional, "conditional");
    assert_eq!(got.caveats, want.caveats, "caveats");
    assert_eq!(got.import_span.is_some(), want.linked_import, "import_span presence");
    assert_eq!(snip.text(&got.span), want.span, "span text");
    assert_eq!(snip.text(&got.statement_span), want.statement, "statement text");
}

fn assert_complete(report: &ReexportReport) {
    assert_eq!(report.coverage.state, ReexportCoverageState::Complete);
    assert!(report.coverage.limits.is_empty());
}

fn assert_complete_and_empty(file_name: &str, source: &str) {
    let report = snippet(file_name, source).report();
    assert_complete(&report);
    assert!(report.entries.is_empty(), "{file_name}: {:?}", report.entries);
}

/// Invariants every report holds, whatever the input.
fn assert_report_invariants(label: &str, report: &ReexportReport) {
    let starts: Vec<u32> = report.entries.iter().map(|e| e.span.start_byte).collect();
    assert!(starts.windows(2).all(|w| w[0] <= w[1]), "{label}: entries out of order");
    let mut spans: Vec<_> = report.entries.iter().map(|e| (e.span.start_byte, e.span.end_byte)).collect();
    spans.dedup();
    assert_eq!(spans.len(), report.entries.len(), "{label}: duplicate spans");
    let keys: Vec<_> = report.coverage.limits.iter().map(|l| (l.span.start_byte, l.reason)).collect();
    assert!(keys.windows(2).all(|w| w[0] <= w[1]), "{label}: limits out of order");
    assert_eq!(
        report.coverage.state == ReexportCoverageState::Limited,
        !report.coverage.limits.is_empty(),
        "{label}: state vs limits"
    );
    for entry in &report.entries {
        assert!(entry.caveats.windows(2).all(|w| w[0] < w[1]), "{label}: caveats unsorted");
    }
}

fn assert_indexed_marks(snip: &Snippet, marked_names: &[&str]) {
    let index = snip.indexed();
    let report = index.reexports.as_ref().expect("reexports computed");
    for import in &index.imports {
        let forwarded = report.entries.iter().any(|e| e.forwards_import_at(&import.span));
        assert_eq!(import.reexported, forwarded, "marker of import {}", import.name);
    }
    let mut marked: Vec<&str> = index.imports.iter().filter(|i| i.reexported).map(|i| i.name.as_str()).collect();
    marked.sort_unstable();
    assert_eq!(marked, marked_names);
}

// ---------------------------------------------------------------- Swift --

#[test]
fn swift_exported_import_is_a_wildcard_with_the_unstable_caveat() {
    let source = include_str!(
        "../fixtures/reexports/swift/pos-exported-import.swift"
    );
    let snip = snippet("a.swift", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Wildcard,
        Want {
            source: Some("Foundation"),
            caveats: &[ReexportCaveat::UnstableFeature],
            span: "Foundation",
            statement: "@_exported import Foundation",
            linked_import: true,
            ..Want::default()
        },
    );
    assert_indexed_marks(&snip, &["Foundation"]);
}

#[test]
fn swift_exported_submodule_import_keeps_the_dotted_source() {
    let source = include_str!(
        "../fixtures/reexports/swift/pos-exported-submodule.swift"
    );
    let snip = snippet("a.swift", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Wildcard,
        Want {
            source: Some("Foo.Bar"),
            caveats: &[ReexportCaveat::UnstableFeature],
            span: "Foo.Bar",
            statement: "@_exported import Foo.Bar",
            linked_import: true,
            ..Want::default()
        },
    );
    assert_indexed_marks(&snip, &["Foo.Bar"]);
}

#[test]
fn swift_exported_selective_import_is_named() {
    let source = include_str!(
        "../fixtures/reexports/swift/pos-exported-selective.swift"
    );
    let snip = snippet("a.swift", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("S"),
            original: Some("S"),
            source: Some("Foo"),
            caveats: &[ReexportCaveat::UnstableFeature],
            span: "Foo.S",
            statement: "@_exported import struct Foo.S",
            linked_import: true,
            ..Want::default()
        },
    );
    assert_indexed_marks(&snip, &["S"]);
}

#[test]
fn swift_every_selective_kind_is_named_and_a_submodule_is_not() {
    let source = "@_exported import func Foo.f\n@_exported import typealias Foo.T\n@_exported import Foo.Sub\n";
    let snip = snippet("a.swift", source);
    let report = snip.report();
    let forms: Vec<_> = report.entries.iter().map(|e| (e.form, e.exported_name.clone())).collect();
    assert_eq!(
        forms,
        vec![
            (ReexportForm::Named, Some("f".to_owned())),
            (ReexportForm::Named, Some("T".to_owned())),
            (ReexportForm::Wildcard, None),
        ]
    );
}

#[test]
fn swift_exported_with_extra_modifiers_and_odd_spacing_is_still_read() {
    let with_access = include_str!(
        "../fixtures/reexports/swift/pos-access-with-attr.swift"
    );
    let snip = snippet("a.swift", with_access);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1, "only @_exported forwards");
    assert_eq!(snip.text(&report.entries[0].statement_span), "@_exported public import Foo");

    let spacing = include_str!(
        "../fixtures/reexports/swift/pos-exported-attr-spacing.swift"
    );
    let snip = snippet("a.swift", spacing);
    let report = snip.report();
    assert_complete(&report);
    let statements: Vec<&str> = report.entries.iter().map(|e| snip.text(&e.statement_span)).collect();
    assert_eq!(statements, ["@_exported\nimport Foo", "@_exported  import  Foo.Bar"]);
    let sources: Vec<_> = report.entries.iter().map(|e| e.source.as_deref()).collect();
    assert_eq!(sources, [Some("Foo"), Some("Foo.Bar")]);
}

#[test]
fn swift_access_level_and_plain_imports_are_complete_negatives() {
    assert_complete_and_empty(
        "a.swift",
        include_str!(
            "../fixtures/reexports/swift/pos-public-import.swift"
        ),
    );
    assert_complete_and_empty(
        "a.swift",
        include_str!(
            "../fixtures/reexports/swift/pos-access-imports.swift"
        ),
    );
    assert_complete_and_empty(
        "a.swift",
        include_str!(
            "../fixtures/reexports/swift/pos-selective-import.swift"
        ),
    );
    assert_complete_and_empty(
        "a.swift",
        include_str!(
            "../fixtures/reexports/swift/neg-plain-import.swift"
        ),
    );
    assert_complete_and_empty(
        "a.swift",
        include_str!(
            "../fixtures/reexports/swift/neg-typealias-public.swift"
        ),
    );
    assert_complete_and_empty(
        "a.swift",
        include_str!(
            "../fixtures/reexports/swift/neg-other-attr.swift"
        ),
    );
}

#[test]
fn swift_plain_and_access_imports_are_not_marked_forwarded() {
    let snip = snippet("a.swift", "public import A\nimport B\n@_exported import C\n@testable import D\n");
    assert_indexed_marks(&snip, &["C"]);
}

#[test]
fn swift_import_inside_if_is_conditional_and_after_endif_is_not() {
    let guarded = include_str!(
        "../fixtures/reexports/swift/pos-exported-ifconfig.swift"
    );
    let snip = snippet("a.swift", guarded);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Wildcard,
        Want {
            source: Some("Foo"),
            caveats: &[ReexportCaveat::UnstableFeature],
            conditional: true,
            span: "Foo",
            statement: "@_exported import Foo",
            linked_import: true,
            ..Want::default()
        },
    );

    let after = snippet("a.swift", "#if canImport(Foo)\n@_exported import Foo\n#endif\n@_exported import Bar\n");
    let report = after.report();
    let flags: Vec<_> = report.entries.iter().map(|e| e.conditional).collect();
    assert_eq!(flags, [true, false]);
}

#[test]
fn swift_duplicate_sites_stay_separate_and_order_is_source_order() {
    let snip = snippet("a.swift", "@_exported import B\n@_exported import A\n@_exported import B\n");
    let report = snip.report();
    assert_report_invariants("swift dup", &report);
    let sources: Vec<_> = report.entries.iter().map(|e| e.source.as_deref().unwrap()).collect();
    assert_eq!(sources, ["B", "A", "B"]);
}

#[test]
fn swift_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let snip = snippet("a.swift", "@_exported import Foo\nfunc broken( {{{\n");
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
}

#[test]
fn swift_damaged_exported_import_is_limited_never_a_complete_negative() {
    let snip = snippet("a.swift", "@_exported import\n");
    let report = snip.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited, "{report:?}");
    assert!(report.entries.is_empty());
    assert!(report.coverage.limits.iter().all(|l| l.reason == ReexportLimitReason::ParseError));
    assert_report_invariants("swift damaged", &report);

    let dangling = snippet("a.swift", "@_exported\n");
    assert_eq!(dangling.report().coverage.state, ReexportCoverageState::Limited);
}

#[test]
fn swift_samples_corpus_holds_the_report_invariants() {
    let samples = [
        ("neg-other-attr", sample!("swift", "neg-other-attr.swift")),
        ("neg-plain-import", sample!("swift", "neg-plain-import.swift")),
        ("neg-typealias-public", sample!("swift", "neg-typealias-public.swift")),
        ("pos-access-imports", sample!("swift", "pos-access-imports.swift")),
        ("pos-access-with-attr", sample!("swift", "pos-access-with-attr.swift")),
        ("pos-exported-attr-spacing", sample!("swift", "pos-exported-attr-spacing.swift")),
        ("pos-exported-ifconfig", sample!("swift", "pos-exported-ifconfig.swift")),
        ("pos-exported-import", sample!("swift", "pos-exported-import.swift")),
        ("pos-exported-selective", sample!("swift", "pos-exported-selective.swift")),
        ("pos-exported-submodule", sample!("swift", "pos-exported-submodule.swift")),
        ("pos-public-import", sample!("swift", "pos-public-import.swift")),
        ("pos-selective-import", sample!("swift", "pos-selective-import.swift")),
    ];
    for (label, source) in samples {
        let snip = snippet("a.swift", source);
        let report = snip.report();
        assert_report_invariants(label, &report);
        assert_eq!(report.coverage.state, ReexportCoverageState::Complete, "{label}");
        assert_eq!(report.entries.len(), source.matches("@_exported").count(), "{label}");
    }
}

// ---------------------------------------------------------------- Scala --

#[test]
fn scala_export_of_a_path_is_named_with_the_path_as_source() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-path.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("b"),
            original: Some("b"),
            source: Some("a"),
            span: "b",
            statement: "export a.b",
            ..Want::default()
        },
    );
    assert_indexed_marks(&snip, &[]);
}

#[test]
fn scala_selectors_rename_and_split_the_given_and_star_wildcards() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-selectors.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 6);
    let e = &report.entries;
    let statement = "export a.{b as c, given, *}";
    assert_entry(
        &snip,
        &e[0],
        ReexportForm::Named,
        Want { exported: Some("c"), original: Some("b"), source: Some("a"), span: "b as c", statement, ..Want::default() },
    );
    assert_entry(
        &snip,
        &e[1],
        ReexportForm::Wildcard,
        Want {
            source: Some("a"),
            caveats: &[ReexportCaveat::GivensOnly],
            span: "given",
            statement,
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &e[2],
        ReexportForm::Wildcard,
        Want { source: Some("a"), span: "*", statement, ..Want::default() },
    );
    assert_entry(
        &snip,
        &e[3],
        ReexportForm::Named,
        Want {
            exported: Some("c"),
            original: Some("b"),
            source: Some("a"),
            span: "b as c",
            statement: "export a.{b as c, d}",
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &e[4],
        ReexportForm::Named,
        Want {
            exported: Some("d"),
            original: Some("d"),
            source: Some("a"),
            span: "d",
            statement: "export a.{b as c, d}",
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &e[5],
        ReexportForm::Wildcard,
        Want { source: Some("a"), span: "*", statement: "export a.*", ..Want::default() },
    );
}

#[test]
fn scala_exclusion_lists_the_name_on_the_star_wildcard_and_makes_no_entry_for_it() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-exclusion.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 2, "one wildcard per clause, none named `_`");
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Wildcard,
        Want {
            source: Some("a"),
            excluded: &["x"],
            span: "*",
            statement: "export a.{x as _, *}",
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &report.entries[1],
        ReexportForm::Wildcard,
        Want {
            source: Some("a"),
            excluded: &["x"],
            span: "*",
            statement: "export a.{x => _, *}",
            ..Want::default()
        },
    );
}

#[test]
fn scala_given_selectors_are_wildcards_with_the_givens_only_caveat() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-givens.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    let shape: Vec<_> =
        report.entries.iter().map(|e| (e.form, snip.text(&e.span).to_owned(), e.caveats.clone())).collect();
    let givens = vec![ReexportCaveat::GivensOnly];
    assert_eq!(
        shape,
        vec![
            (ReexportForm::Wildcard, "given".to_owned(), givens.clone()),
            (ReexportForm::Wildcard, "T".to_owned(), givens.clone()),
            (ReexportForm::Wildcard, "U".to_owned(), givens.clone()),
            (ReexportForm::Wildcard, "given".to_owned(), givens),
            (ReexportForm::Named, "b".to_owned(), vec![]),
            (ReexportForm::Named, "b => c".to_owned(), vec![]),
        ]
    );
    let renamed = &report.entries[5];
    assert_eq!(renamed.exported_name.as_deref(), Some("c"));
    assert_eq!(renamed.original_name.as_deref(), Some("b"));
}

#[test]
fn scala_given_by_type_differs_from_a_plain_type_name_selector() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-given-vs-plain.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    let by_statement = |statement: &str| -> Vec<&ReexportEntry> {
        report.entries.iter().filter(|e| snip.text(&e.statement_span) == statement).collect()
    };
    let plain = by_statement("export a.{T}");
    assert_eq!(plain[0].form, ReexportForm::Named);
    assert_eq!(plain[0].exported_name.as_deref(), Some("T"));
    assert!(plain[0].caveats.is_empty());
    let given = by_statement("export a.{given T}");
    assert_eq!(given[0].form, ReexportForm::Wildcard);
    assert_eq!(given[0].caveats, [ReexportCaveat::GivensOnly]);
    let tail = by_statement("export a.b.given");
    assert_eq!(tail[0].source.as_deref(), Some("a.b"));
    assert_eq!(tail[0].caveats, [ReexportCaveat::GivensOnly]);
    let star = by_statement("export a.b.*");
    assert!(star[0].caveats.is_empty());
}

#[test]
fn scala_backticked_name_loses_its_quotes_but_the_source_keeps_the_path_as_written() {
    let snip = snippet("a.scala", "export `a b`.`weird name`\n");
    let report = snip.report();
    assert_complete(&report);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("weird name"),
            original: Some("weird name"),
            source: Some("`a b`"),
            span: "`weird name`",
            statement: "export `a b`.`weird name`",
            ..Want::default()
        },
    );
}

#[test]
fn scala_object_member_export_needs_no_import_and_is_scoped_to_the_object() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-member-no-import.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 3);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("run"),
            original: Some("run"),
            source: Some("impl"),
            scope: Some("O"),
            span: "run",
            statement: "export impl.run",
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &report.entries[2],
        ReexportForm::Named,
        Want {
            exported: Some("begin"),
            original: Some("start"),
            source: Some("impl"),
            scope: Some("O"),
            span: "start as begin",
            statement: "export impl.{stop, start as begin}",
            ..Want::default()
        },
    );
    assert_indexed_marks(&snip, &[]);
}

#[test]
fn scala_scope_is_the_enclosing_package_and_object_chain() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-top-level.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    let scopes: Vec<_> = report.entries.iter().map(|e| e.scope.as_deref()).collect();
    assert_eq!(scopes, [Some("p"), Some("p.X")]);
    assert_eq!(report.entries[1].source.as_deref(), Some("q.r"));

    let nested = snippet("a.scala", "object A:\n  object B:\n    export x.y\n");
    assert_eq!(nested.report().entries[0].scope.as_deref(), Some("A.B"));
    let bare = snippet("a.scala", "export x.y\n");
    assert_eq!(bare.report().entries[0].scope, None);
}

#[test]
fn scala_enum_export_is_a_plain_wildcard_and_named() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-case.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Wildcard,
        Want { source: Some("E"), scope: Some("O"), span: "*", statement: "export E.*", ..Want::default() },
    );
    assert_entry(
        &snip,
        &report.entries[1],
        ReexportForm::Named,
        Want {
            exported: Some("A"),
            original: Some("A"),
            source: Some("E"),
            scope: Some("O"),
            span: "A",
            statement: "export E.A",
            ..Want::default()
        },
    );
}

#[test]
fn scala_export_through_an_imported_name_links_the_import() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-import-then-export.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 3);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("a"),
            original: Some("a"),
            local: Some("Thing"),
            source: Some("Thing"),
            span: "a",
            statement: "export Thing.{a, b}",
            linked_import: true,
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &report.entries[2],
        ReexportForm::Named,
        Want {
            exported: Some("x"),
            original: Some("x"),
            local: Some("O"),
            source: Some("O"),
            span: "x",
            statement: "export O.x",
            linked_import: true,
            ..Want::default()
        },
    );
    // The linked entry points at the record of the `import`, not at the
    // record the export clause itself produces.
    let index = snip.indexed();
    let linked = report.entries[0].import_span.as_ref().unwrap();
    let record = index.imports.iter().find(|i| &i.span == linked).unwrap();
    assert_eq!(record.source.as_deref(), Some("lib.Thing"));
    assert!(record.reexported);
    assert_indexed_marks(&snip, &["O", "Thing"]);
}

#[test]
fn scala_export_through_a_name_a_member_definition_hides_does_not_link_the_import() {
    let control = include_str!(
        "../fixtures/reexports/scala/pos-import-then-export.scala"
    );
    assert_indexed_marks(&snippet("a.scala", control), &["O", "Thing"]);

    // The first export moves into an object whose own `val Thing` is the
    // nearer binding; the import stays at file level.
    let hidden =
        control.replacen("export Thing.{a, b}\n", "object Holder:\n  val Thing = new Impl\n  export Thing.{a, b}\n", 1);
    let snip = snippet("a.scala", &hidden);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 3, "{:?}", report.entries);
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("a"),
            original: Some("a"),
            source: Some("Thing"),
            scope: Some("Holder"),
            span: "a",
            statement: "export Thing.{a, b}",
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &report.entries[2],
        ReexportForm::Named,
        Want {
            exported: Some("x"),
            original: Some("x"),
            local: Some("O"),
            source: Some("O"),
            span: "x",
            statement: "export O.x",
            linked_import: true,
            ..Want::default()
        },
    );
    assert_indexed_marks(&snip, &["O"]);

    let wildcard = snippet("a.scala", "import lib.Thing\nobject Holder:\n  val Thing = new Impl\n  export Thing.*\n");
    let report = wildcard.report();
    assert_complete(&report);
    assert_entry(
        &wildcard,
        &report.entries[0],
        ReexportForm::Wildcard,
        Want { source: Some("Thing"), scope: Some("Holder"), span: "*", statement: "export Thing.*", ..Want::default() },
    );
    assert_indexed_marks(&wildcard, &[]);

    // A class parameter is a nearer binding too.
    let parameter = snippet("a.scala", "import lib.Thing\nclass C(Thing: Impl):\n  export Thing.x\n");
    assert!(parameter.report().entries[0].import_span.is_none());
    assert_indexed_marks(&parameter, &[]);
}

#[test]
fn scala_export_links_only_an_import_in_the_same_or_an_enclosing_scope() {
    let other_object = snippet("a.scala", "object A:\n  import lib.Thing\nobject B:\n  export Thing.x\n");
    let report = other_object.report();
    assert_complete(&report);
    assert_entry(
        &other_object,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("x"),
            original: Some("x"),
            source: Some("Thing"),
            scope: Some("B"),
            span: "x",
            statement: "export Thing.x",
            ..Want::default()
        },
    );
    assert_indexed_marks(&other_object, &[]);

    // An import after the export does not scope over it.
    let after = snippet("a.scala", "export Thing.x\nimport lib.Thing\n");
    assert!(after.report().entries[0].import_span.is_none());
    assert_indexed_marks(&after, &[]);

    // Control: an import in an enclosing object is in scope.
    let enclosing = snippet("a.scala", "object A:\n  import lib.Thing\n  object B:\n    export Thing.x\n");
    let report = enclosing.report();
    assert_eq!(report.entries[0].local_name.as_deref(), Some("Thing"));
    assert_indexed_marks(&enclosing, &["Thing"]);

    // The nearest import of the name is the one linked.
    let nearest = snippet("a.scala", "import far.Thing\nobject B:\n  import near.Thing\n  export Thing.x\n");
    let report = nearest.report();
    let linked = report.entries[0].import_span.as_ref().expect("linked");
    let index = nearest.indexed();
    let record = index.imports.iter().find(|import| &import.span == linked).expect("record");
    assert_eq!(record.source.as_deref(), Some("near.Thing"));
    let marked: Vec<_> = index.imports.iter().map(|import| (import.source.as_deref(), import.reexported)).collect();
    assert!(marked.contains(&(Some("far.Thing"), false)), "{marked:?}");
}

#[test]
fn scala_imports_and_local_definitions_are_complete_negatives() {
    for source in [
        include_str!("../fixtures/reexports/scala/neg-import-only.scala"),
        include_str!("../fixtures/reexports/scala/neg-local-def.scala"),
        include_str!("../fixtures/reexports/scala/scala2-import-contrast.scala"),
    ] {
        assert_complete_and_empty("a.scala", source);
    }
}

#[test]
fn scala_modified_or_annotated_export_is_a_parse_error_limit_and_never_a_public_entry() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-class-member.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    let limited: Vec<_> = report.coverage.limits.iter().map(|l| (l.reason, snip.text(&l.span))).collect();
    assert_eq!(
        limited,
        [(ReexportLimitReason::ParseError, "private"), (ReexportLimitReason::ParseError, "protected")]
    );
    let statements: Vec<_> = report.entries.iter().map(|e| snip.text(&e.statement_span)).collect();
    assert_eq!(statements, ["export impl.*", "export impl.{m as n}"]);
    assert!(report.entries.iter().all(|e| e.scope.as_deref() == Some("Facade")));

    let modifiers = include_str!(
        "../fixtures/reexports/scala/pos-export-modifiers.scala"
    );
    let snip = snippet("a.scala", modifiers);
    let report = snip.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert!(report.coverage.limits.iter().all(|l| l.reason == ReexportLimitReason::ParseError));
    assert!(report.entries.is_empty(), "no modified export is read as public: {:?}", report.entries);
}

#[test]
fn scala_inline_export_is_limited_while_neighboring_exports_are_kept() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-given-vs-plain.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert_eq!(report.coverage.limits.len(), 1);
    assert_eq!(snip.text(&report.coverage.limits[0].span), "inline export a.b");
    assert_eq!(report.entries.len(), 6);
}

#[test]
fn scala_multi_clause_export_is_a_grammar_gap_with_no_entry() {
    let source = include_str!(
        "../fixtures/reexports/scala/pos-export-multiple.scala"
    );
    let snip = snippet("a.scala", source);
    let report = snip.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert!(report.entries.is_empty());
    assert_eq!(report.coverage.limits.len(), 1);
    assert_eq!(report.coverage.limits[0].reason, ReexportLimitReason::GrammarGap);
    assert_eq!(snip.text(&report.coverage.limits[0].span), "export a.b, c.d.{e, f}");
}

#[test]
fn scala_two_export_lines_are_not_mistaken_for_one_multi_clause_export() {
    let snip = snippet("a.scala", "export a.b\nexport c.d\n");
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 2);
}

#[test]
fn scala2_export_identifier_is_damage_on_a_trigger_and_limits() {
    // `export` as an identifier is Scala 2 code; the Scala 3 grammar reads it
    // as a damaged `export`, so coverage cannot claim a negative.
    let source = include_str!(
        "../fixtures/reexports/scala/scala2-export-ident.scala"
    );
    let report = snippet("a.scala", source).report();
    assert!(report.entries.is_empty());
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert_eq!(report.coverage.limits[0].reason, ReexportLimitReason::ParseError);
}

#[test]
fn scala_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let snip = snippet("a.scala", "export a.b\nobject O { def f = 1 }\nval = = =\n");
    let report = snip.report();
    assert_eq!(report.entries.len(), 1, "{report:?}");
    assert_complete(&report);
}

#[test]
fn scala_damaged_export_is_limited_not_a_complete_negative() {
    let report = snippet("a.scala", "export\n").report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited, "{report:?}");
    assert!(report.entries.is_empty());
    assert_report_invariants("scala damaged", &report);
}

#[test]
fn scala_samples_corpus_holds_the_report_invariants() {
    let samples = [
        ("neg-import-only", sample!("scala", "neg-import-only.scala")),
        ("neg-local-def", sample!("scala", "neg-local-def.scala")),
        ("pos-export-case", sample!("scala", "pos-export-case.scala")),
        ("pos-export-class-member", sample!("scala", "pos-export-class-member.scala")),
        ("pos-export-exclusion", sample!("scala", "pos-export-exclusion.scala")),
        ("pos-export-givens", sample!("scala", "pos-export-givens.scala")),
        ("pos-export-member-no-import", sample!("scala", "pos-export-member-no-import.scala")),
        ("pos-export-modifiers", sample!("scala", "pos-export-modifiers.scala")),
        ("pos-export-multiple", sample!("scala", "pos-export-multiple.scala")),
        ("pos-export-path", sample!("scala", "pos-export-path.scala")),
        ("pos-export-selectors", sample!("scala", "pos-export-selectors.scala")),
        ("pos-export-top-level", sample!("scala", "pos-export-top-level.scala")),
        ("pos-given-vs-plain", sample!("scala", "pos-given-vs-plain.scala")),
        ("pos-import-then-export", sample!("scala", "pos-import-then-export.scala")),
        ("scala2-export-ident", sample!("scala", "scala2-export-ident.scala")),
        ("scala2-import-contrast", sample!("scala", "scala2-import-contrast.scala")),
    ];
    for (label, source) in samples {
        let report = snippet("a.scala", source).report();
        assert_report_invariants(label, &report);
        if label.starts_with("neg-") || label == "scala2-import-contrast" {
            assert_eq!(report.coverage.state, ReexportCoverageState::Complete, "{label}");
            assert!(report.entries.is_empty(), "{label}");
        }
    }
}

// ------------------------------------------------------------------- C# --

#[test]
fn csharp_type_forwarded_to_is_named_with_the_type_as_written() {
    let source = include_str!(
        "../fixtures/reexports/cs/pos-typeforward.cs"
    );
    let snip = snippet("a.cs", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 3, "InternalsVisibleTo is not a forward");
    let caveats = &[ReexportCaveat::SourceUnresolved];
    assert_entry(
        &snip,
        &report.entries[0],
        ReexportForm::Named,
        Want {
            exported: Some("Other.Lib.T"),
            original: Some("Other.Lib.T"),
            caveats,
            span: "TypeForwardedTo(typeof(Other.Lib.T))",
            statement: "[assembly: TypeForwardedTo(typeof(Other.Lib.T))]",
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &report.entries[1],
        ReexportForm::Named,
        Want {
            exported: Some("System.Collections.Generic.List<>"),
            original: Some("System.Collections.Generic.List<>"),
            caveats,
            span: "TypeForwardedTo(typeof(System.Collections.Generic.List<>))",
            statement: "[assembly: TypeForwardedTo(typeof(System.Collections.Generic.List<>))]",
            ..Want::default()
        },
    );
    assert_entry(
        &snip,
        &report.entries[2],
        ReexportForm::Named,
        Want {
            exported: Some("X"),
            original: Some("X"),
            caveats,
            span: "System.Runtime.CompilerServices.TypeForwardedTo(typeof(X))",
            statement: "[assembly: System.Runtime.CompilerServices.TypeForwardedTo(typeof(X))]",
            ..Want::default()
        },
    );
    assert_indexed_marks(&snip, &[]);
}

#[test]
fn csharp_module_target_and_other_attributes_are_not_forwards() {
    let source = "[module: TypeForwardedTo(typeof(A.B))]\n[assembly: InternalsVisibleTo(\"F\")]\n[assembly: TypeForwardedTo(someName)]\n";
    assert_complete_and_empty("a.cs", source);
}

#[test]
fn csharp_attribute_list_with_two_attributes_yields_two_entries() {
    let snip = snippet("a.cs", "[assembly: TypeForwardedTo(typeof(A.B)), TypeForwardedTo(typeof(C.D))]\n");
    let report = snip.report();
    assert_complete(&report);
    let names: Vec<_> = report.entries.iter().map(|e| e.exported_name.as_deref().unwrap()).collect();
    assert_eq!(names, ["A.B", "C.D"]);
    assert!(report.entries.iter().all(|e| snip.text(&e.statement_span).starts_with("[assembly:")));
}

#[test]
fn csharp_every_using_form_is_a_complete_negative() {
    for source in [
        include_str!("../fixtures/reexports/cs/pos-using.cs"),
        include_str!("../fixtures/reexports/cs/pos-using-alias-any.cs"),
        include_str!("../fixtures/reexports/cs/pos-using-in-namespace.cs"),
        include_str!("../fixtures/reexports/cs/pos-using-after-file-scoped.cs"),
        include_str!("../fixtures/reexports/cs/pos-file-scoped.cs"),
        include_str!("../fixtures/reexports/cs/neg-plain.cs"),
        include_str!("../fixtures/reexports/cs/neg-using-stmt.cs"),
    ] {
        assert_complete_and_empty("a.cs", source);
    }
}

#[test]
fn csharp_using_import_is_not_marked_forwarded_next_to_a_forward() {
    let snip = snippet("a.cs", "using System.Runtime.CompilerServices;\n[assembly: TypeForwardedTo(typeof(A.B))]\n");
    assert_indexed_marks(&snip, &[]);
    assert_eq!(snip.report().entries.len(), 1);
}

#[test]
fn csharp_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let snip = snippet("a.cs", "[assembly: TypeForwardedTo(typeof(A.B))]\nclass C { void M( { }\n");
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1, "{report:?}");
}

#[test]
fn csharp_damaged_forward_is_limited_never_a_complete_negative() {
    for source in [
        "[assembly: TypeForwardedTo(typeof(A.))]\n",
        "[assembly: TypeForwardedTo(\n",
        "[assembly: TypeForwardedTo\n",
    ] {
        let report = snippet("a.cs", source).report();
        assert_eq!(report.coverage.state, ReexportCoverageState::Limited, "{source}: {report:?}");
        assert!(report.entries.is_empty(), "{source}: {:?}", report.entries);
        assert_report_invariants(source, &report);
    }
}

#[test]
fn csharp_samples_corpus_holds_the_report_invariants() {
    let samples = [
        ("neg-plain", sample!("cs", "neg-plain.cs")),
        ("neg-using-stmt", sample!("cs", "neg-using-stmt.cs")),
        ("pos-file-scoped", sample!("cs", "pos-file-scoped.cs")),
        ("pos-typeforward", sample!("cs", "pos-typeforward.cs")),
        ("pos-using-after-file-scoped", sample!("cs", "pos-using-after-file-scoped.cs")),
        ("pos-using-alias-any", sample!("cs", "pos-using-alias-any.cs")),
        ("pos-using-in-namespace", sample!("cs", "pos-using-in-namespace.cs")),
        ("pos-using", sample!("cs", "pos-using.cs")),
    ];
    for (label, source) in samples {
        let report = snippet("a.cs", source).report();
        assert_report_invariants(label, &report);
        assert_eq!(report.coverage.state, ReexportCoverageState::Complete, "{label}");
        assert_eq!(report.entries.len(), usize::from(label == "pos-typeforward") * 3, "{label}");
    }
}
