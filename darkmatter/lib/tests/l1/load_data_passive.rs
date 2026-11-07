//! The passive check and static typing of `load_data`.
//!
//! A literal `as` contract (and a literal CSV/TSV file) is judged before
//! anything runs: a scalar, a document-local reference, or a contract that
//! cannot describe tabular rows is rejected at the `as` argument under its
//! `dm.data.*` code, while a file import is accepted provisionally. A literal
//! contract also types the call's result, so an inline callback's item is
//! typed. The checks take no filesystem context, so they cannot read; the
//! composition tests prove the rejection lands before any expression runs.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::contract::ROW_SHAPE_SUGGESTION;
use darkmatter::markdown::compose::expression::data::{
    NOT_A_CONTAINER_CODE, ROW_SHAPE_CONFLICT_CODE, THIS_IMPORT_REJECTED_CODE,
};
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, ArgumentType, CallCheck, DeclaredTypes, INCOMPATIBLE_CALL_CODE, TypeScope,
    check_expression_calls, expression_call_diagnostics, expression_type, parse_spanned,
};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeOptions, DataRuntime};
use tempfile::TempDir;

fn checks(source: &str) -> Vec<CallCheck> {
    let declared = DeclaredTypes::new(None);
    check_expression_calls(&parse_spanned(source).unwrap_or_else(|error| panic!("{source}: {error}")), &TypeScope::unguarded(&declared))
}

/// The check of the `load_data` call (by any spelling) in `source`.
fn load_check(source: &str) -> CallCheck {
    checks(source)
        .into_iter()
        .find(|check| matches!(check.function.to_ascii_lowercase().as_str(), "load_data" | "loaddata"))
        .unwrap_or_else(|| panic!("{source}: no load_data call"))
}

fn call(file: &str, contract: &str) -> String {
    format!("load_data({file}, \"{contract}\")")
}

/// Asserts the call in `source` passes the passive check.
fn assert_accepted(source: &str) {
    let check = load_check(source);
    assert!(!check.is_rejected(), "{source}: {check:#?}");
    assert_eq!(check.data_problem, None, "{source}");
}

/// Asserts the call in `source` is rejected with `code` at its `as`
/// argument, and returns the rejection message.
fn assert_rejected(source: &str, code: &str) -> String {
    let check = load_check(source);
    assert!(check.is_rejected(), "{source}: {check:#?}");
    let as_span = check.argument_spans[1].clone();
    let diagnostic = check.rejection_diagnostic().unwrap_or_else(|| panic!("{source}: no rejection"));
    assert_eq!(diagnostic.code, code, "{source}: {diagnostic:#?}");
    assert_eq!(diagnostic.span, as_span, "{source}");
    assert!(source[as_span].starts_with('"'), "{source}: the span is the `as` literal");
    assert!(
        check.diagnostics(true).iter().all(|diagnostic| diagnostic.code != INCOMPATIBLE_CALL_CODE),
        "{source}: the data code replaces the generic one"
    );
    assert_eq!(check.rejection_message(), diagnostic.message, "{source}");
    assert!(diagnostic.message.starts_with("load_data(): "), "{source}: {}", diagnostic.message);
    diagnostic.message
}

fn type_text(source: &str) -> String {
    let declared = DeclaredTypes::new(None);
    let scope = TypeScope::unguarded(&declared);
    ArgumentType::Typed(expression_type(&parse_spanned(source).unwrap(), &scope)).to_string()
}

mod container_rule {
    use super::*;

    /// Spec R2: every "yes" row of both tables passes, before CSV/TSV notes.
    #[test]
    fn every_container_row_passes() {
        for contract in [
            "string[]",
            "{ name: string, port: number }[]",
            "tuple([string, number])",
            "{ name: string, tags: string[] }",
            "{ <string>: number }",
            "string[] | null",
            "unknown[]",
            "object",
            "{a: string}[] | {b: number}[]",
            "string[] | {a: string}",
        ] {
            assert_accepted(&call("\"d.yaml\"", contract));
        }
    }

    /// Spec R2: every "no" row is `dm.data.not_a_container` at `as`, with a
    /// literal or a computed file.
    #[test]
    fn every_scalar_row_is_not_a_container_at_the_as_span() {
        for contract in ["string", "number", "enum(a, b)", "unknown", "null"] {
            for file in ["\"d.yaml\"", "f"] {
                let message = assert_rejected(&call(file, contract), NOT_A_CONTAINER_CODE);
                assert!(message.contains(contract), "{message}");
            }
        }
    }

    /// `[string, number]` is not inline type syntax (only a YAML sequence
    /// spells that union), so it fails as malformed syntax rather than as a
    /// scalar union; either way it is rejected before anything runs.
    #[test]
    fn a_bracketed_scalar_union_is_rejected_as_malformed_syntax() {
        let check = load_check(&call("\"d.yaml\"", "[string, number]"));
        assert!(check.is_rejected(), "{check:#?}");
        assert_eq!(check.rejection_diagnostic().unwrap().code, INCOMPATIBLE_CALL_CODE);
    }

    /// Spec R10: the signature's null passthrough does not excuse a wrong
    /// literal contract.
    #[test]
    fn a_literal_null_file_still_rejects_an_invalid_contract() {
        assert_rejected(&call("null", "string"), NOT_A_CONTAINER_CODE);
        assert_rejected(&call("null", "Os@this"), THIS_IMPORT_REJECTED_CODE);
        assert_accepted(&call("null", "string[]"));
    }

    #[test]
    fn every_spelling_of_the_function_is_judged() {
        for name in ["LOAD_DATA", "Load_Data", "loaddata", "LoadData"] {
            assert_rejected(&format!("{name}(\"d.yaml\", \"string\")"), NOT_A_CONTAINER_CODE);
        }
    }

    /// Plan: malformed contract syntax keeps the existing incompatible-call
    /// diagnostic.
    #[test]
    fn malformed_syntax_keeps_the_incompatible_call_code() {
        let check = load_check(&call("\"d.yaml\"", "strng[]["));
        assert!(check.is_rejected(), "{check:#?}");
        assert_eq!(check.data_problem, None);
        assert_eq!(check.rejection_diagnostic().unwrap().code, INCOMPATIBLE_CALL_CODE);
    }

    #[test]
    fn a_computed_contract_is_judged_only_at_execution() {
        assert_accepted("load_data(\"d.yaml\", t)");
        assert_accepted("load_data(\"d.csv\", t)");
    }

    #[test]
    fn a_wrong_file_argument_keeps_its_own_finding_beside_the_data_problem() {
        let check = load_check(&call("map([1], x => x)", "string"));
        let codes: Vec<_> = check.diagnostics(false).into_iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(codes, [INCOMPATIBLE_CALL_CODE, NOT_A_CONTAINER_CODE], "{check:#?}");
    }
}

mod imports {
    use super::*;

    /// Spec R1: a document-local reference is rejected at any depth.
    #[test]
    fn a_document_local_reference_is_rejected_at_any_depth() {
        for contract in ["Os@this", "{ a: Os@this }[]", "Os[]"] {
            let message = assert_rejected(&call("\"os.yaml\"", contract), THIS_IMPORT_REJECTED_CODE);
            assert!(message.contains("`Os`"), "{message}");
        }
    }

    /// Spec R1: a file import passes provisionally. The check takes no
    /// filesystem context, so the absent schema file cannot be opened;
    /// `the_precheck_rejects_before_anything_runs` proves it end to end.
    #[test]
    fn a_file_import_is_provisionally_accepted() {
        assert_accepted(&call("\"os.yaml\"", "os-list@./schemas/os.yaml"));
        assert_accepted(&call("\"os.yaml\"", "{ os: Os@./schemas/os.yaml }[]"));
    }

    /// `interpolation` is a shipped string, `count` a union of scalars, and
    /// `say-long` an object.
    #[test]
    fn a_shipped_import_is_judged_from_the_embedded_sources() {
        for contract in ["interpolation@darkmatter:lifecycle", "count@darkmatter:lifecycle"] {
            assert_rejected(&call("\"d.yaml\"", contract), NOT_A_CONTAINER_CODE);
        }
        for contract in ["say-long@darkmatter:lifecycle", "interpolation[]@darkmatter:lifecycle"] {
            assert_accepted(&call("\"d.yaml\"", contract));
        }
    }

    /// An unknown shipped type is not a syntax error; execution reports it.
    #[test]
    fn an_unknown_shipped_type_is_left_to_execution() {
        assert_accepted(&call("\"d.yaml\"", "nope@darkmatter:lifecycle"));
    }
}

mod tabular {
    use super::*;

    /// Spec R4: arms that disagree are named.
    #[test]
    fn disagreeing_arms_are_a_row_shape_conflict_naming_both() {
        let message = assert_rejected(&call("\"d.csv\"", "{a: string}[] | tuple([string])[]"), ROW_SHAPE_CONFLICT_CODE);
        assert!(message.contains("a: string") && message.contains("tuple("), "{message}");
    }

    /// Spec R4: a scalar or `unknown` element, or a top-level object or
    /// dictionary, gets the suggestion.
    #[test]
    fn a_non_row_element_or_top_level_object_suggests_a_row_type() {
        for contract in ["string[]", "unknown[]", "object", "{ <string>: string }"] {
            for file in ["\"d.csv\"", "\"d.tsv\""] {
                let message = assert_rejected(&call(file, contract), ROW_SHAPE_CONFLICT_CODE);
                assert!(message.contains(ROW_SHAPE_SUGGESTION), "{message}");
            }
        }
    }

    #[test]
    fn object_and_list_rows_pass() {
        for contract in ["{ name: string, port: number }[]", "tuple([string, number])[]"] {
            assert_accepted(&call("\"d.csv\"", contract));
            assert_accepted(&call("\"d.tsv\"", contract));
        }
    }

    /// The extension is matched case-insensitively, and a URL by its path.
    #[test]
    fn the_extension_is_read_from_a_local_path_or_a_url_path() {
        for file in ["\"DATA.CSV\"", "\"./rows/d.Tsv\"", "\"https://example.com/d.csv?page=2#top\""] {
            assert_rejected(&call(file, "string[]"), ROW_SHAPE_CONFLICT_CODE);
        }
        assert_accepted(&call("\"https://example.com/d.json?as=.csv#.tsv\"", "string[]"));
    }

    #[test]
    fn other_formats_and_computed_files_skip_the_row_rule() {
        assert_accepted(&call("\"d.json\"", "string[]"));
        assert_accepted(&call("f", "string[]"));
        assert_accepted(&call("null", "string[]"));
    }

    /// An unresolved file import decides the row mode only at execution.
    #[test]
    fn an_import_decided_element_is_provisional() {
        assert_accepted(&call("\"d.csv\"", "Hosts@./hosts.yaml"));
        assert_accepted(&call("\"d.csv\"", "({ a: string } | Host@./hosts.yaml)[]"));
    }

    /// A shipped import is expanded, so its element type decides passively.
    #[test]
    fn a_shipped_element_is_judged_passively() {
        assert_accepted(&call("\"d.csv\"", "say-long[]@darkmatter:lifecycle"));
        assert_rejected(&call("\"d.csv\"", "interpolation[]@darkmatter:lifecycle"), ROW_SHAPE_CONFLICT_CODE);
    }
}

mod typing {
    use super::*;

    /// Spec R10: the denoted type, joined with `null` exactly when the file
    /// may be null.
    #[test]
    fn the_result_follows_the_contract_and_the_file_nullability() {
        let present = type_text(&call("\"h.yaml\"", "{ a: string }[]"));
        assert!(present.ends_with("[]") && present.contains("a: string"), "{present}");
        assert!(!present.contains("null") && !present.contains("unknown"), "{present}");
        assert_eq!(type_text(&call("f", "{ a: string }[]")), format!("{present} | null"));
        assert_eq!(type_text(&call("null", "{ a: string }[]")), "null");
        assert_eq!(type_text(&call("\"h.yaml\"", "string[] | null")), "string[] | null");
    }

    /// Spec R10 limits: a named reference projects to `unknown`; inline
    /// structure around it survives.
    #[test]
    fn a_named_reference_projects_to_unknown() {
        assert_eq!(type_text(&call("\"h.yaml\"", "os-list@./schemas/os.yaml")), "unknown");
        assert_eq!(type_text(&call("\"h.yaml\"", "say-long@darkmatter:lifecycle")), "unknown");
        let nested = type_text(&call("\"h.yaml\"", "{ host: Host@./h.yaml, port: number }[]"));
        assert!(nested.contains("host: unknown") && nested.contains("port: number"), "{nested}");
    }

    #[test]
    fn a_computed_or_invalid_contract_is_unknown() {
        assert_eq!(type_text("load_data(\"h.yaml\", t)"), "unknown");
        assert_eq!(type_text(&call("\"h.yaml\"", "string")), "unknown");
    }

    /// Spec R10: no member-level check, so a misspelled member is silent.
    #[test]
    fn a_misspelled_member_is_not_reported() {
        let source = "map(load_data(\"h.yaml\", \"{ name: string, port: number }[]\"), h => h.nmae)";
        let declared = DeclaredTypes::new(None);
        let found = expression_call_diagnostics(&parse_spanned(source).unwrap(), &TypeScope::unguarded(&declared), true);
        assert!(found.is_empty(), "{found:#?}");
        let named = type_text("map(load_data(\"h.yaml\", \"{ name: string, port: number }[]\"), h => h.name)");
        assert!(named.contains("string") && named.ends_with("[]"), "{named}");
    }

    /// The callback item carries the element type into the calls it makes.
    #[test]
    fn a_callback_call_is_checked_against_the_element_type() {
        let min_of = |contract: &str, callback: &str| {
            let source = format!("map(load_data(\"h.yaml\", {contract}), h => {callback})");
            checks(&source).into_iter().find(|check| check.function == "min").expect("a min call").category()
        };
        assert_eq!(min_of("\"{ flag: boolean(required) }[]\"", "min(h.flag, 5)"), ArgumentCategory::Incompatible);
        assert_eq!(min_of("\"{ port: number(required) }[]\"", "min(h.port, 5)"), ArgumentCategory::Compatible);
        assert_eq!(min_of("t", "min(h.flag, 5)"), ArgumentCategory::Unknown);
        let kept = type_text("filter(load_data(\"h.yaml\", \"{ flag: boolean(required) }[]\"), h => h.flag)");
        assert!(kept.contains("flag: boolean"), "{kept}");
    }
}

mod composition {
    use super::*;

    fn write(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    /// Plan Phase 5 checkpoint: a scalar contract is rejected under
    /// `dm.data.not_a_container` before any expression runs, with no data,
    /// import, or earlier file read, while an earlier file import passes.
    #[test]
    fn the_precheck_rejects_before_anything_runs() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "a.md", "---\ntitle: Ay\n---\nbody\n");
        write(dir.path(), "d.yaml", "- x\n");
        let doc = write(
            dir.path(),
            "doc.md",
            "{{ frontmatter('a.md') }} {{ file_exists('b.md') }}\n\n\
             {{ load_data('d.yaml', 'os-list@./schemas/os.yaml') }}\n\n\
             {{ load_data(\"d.yaml\", \"string\") }}\n",
        );
        let runtime = DataRuntime::new();
        let options = ComposeOptions::new().with_source_file(&doc).with_data_runtime(runtime.clone());
        let request = crate::request_support::request(options);
        let failure = Markdown::try_from(doc.as_path())
            .unwrap()
            .compose_with_warnings(&request)
            .expect_err("a scalar contract rejects the document");
        let rejected = failure.error.rejected_call().expect("the call check rejected it");
        assert_eq!(rejected.data_problem.as_ref().map(|problem| problem.code), Some(NOT_A_CONTAINER_CODE));
        let record = failure.error.call_diagnostic().expect("a located record");
        assert_eq!(record.code, NOT_A_CONTAINER_CODE, "{record:#?}");
        let source = record.source.as_ref().expect("located");
        assert_eq!((source.line, source.column), (Some(5), Some(24)), "{record:#?}");
        assert_eq!(error_diagnostics(&failure.error), vec![record]);
        assert!(failure.read_set.is_empty(), "{:?}", failure.read_set);
        assert!(runtime.read_set().is_empty());
        let stats = runtime.stats();
        assert_eq!((stats.reads, stats.parses, stats.contract_preparations), (0, 0, 0), "{stats:?}");
    }

    #[test]
    fn a_frontmatter_row_shape_conflict_is_published_under_its_code() {
        let dir = TempDir::new().unwrap();
        let doc = write(dir.path(), "doc.md", "---\nrows: \"{{ load_data('d.csv', 'string[]') }}\"\n---\nbody\n");
        let request = crate::request_support::request(ComposeOptions::new().with_source_file(&doc));
        let failure = Markdown::try_from(doc.as_path()).unwrap().compose_with_warnings(&request).expect_err("rejected");
        let codes: Vec<_> = error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(codes, [ROW_SHAPE_CONFLICT_CODE]);
        assert!(failure.read_set.is_empty(), "{:?}", failure.read_set);
    }
}
