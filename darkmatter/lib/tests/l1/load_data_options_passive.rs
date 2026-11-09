//! `load_data` options judged passively, before anything runs: literal
//! option mistakes located at the third argument, the headerless object-row
//! conflict, which checks a computed options object defers, and result typing
//! at both arities.
//!
//! Every check takes no filesystem context, so it cannot read. The one
//! composition test writes its fixture into a temporary directory to obtain
//! the execution message for the same option.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::{
    DataError, INVALID_OPTIONS_CODE, NOT_A_CONTAINER_CODE, ROW_SHAPE_CONFLICT_CODE,
};
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, ArgumentType, CallCheck, DeclaredTypes, ExpressionError, INCOMPATIBLE_CALL_CODE, TypeScope,
    check_expression_calls, expression_call_diagnostics, expression_type, parse_spanned,
};
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions, DataRuntime};
use tempfile::TempDir;

fn checks(source: &str) -> Vec<CallCheck> {
    let declared = DeclaredTypes::new(None);
    let parsed = parse_spanned(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    check_expression_calls(&parsed, &TypeScope::unguarded(&declared))
}

/// The check of the `load_data` call in `source`.
fn load_check(source: &str) -> CallCheck {
    checks(source)
        .into_iter()
        .find(|check| check.function == "load_data")
        .unwrap_or_else(|| panic!("{source}: no load_data call"))
}

/// Asserts the call in `source` passes the passive check.
fn assert_accepted(source: &str) {
    let check = load_check(source);
    assert!(!check.is_rejected(), "{source}: {check:#?}");
    assert_eq!(check.data_problem, None, "{source}");
}

/// Asserts the call in `source` is rejected with `code` ranged at argument
/// `argument`, and returns the rejection message.
fn assert_rejected_at(source: &str, code: &str, argument: usize) -> String {
    let check = load_check(source);
    assert!(check.is_rejected(), "{source}: {check:#?}");
    let problem = check.data_problem.as_ref().unwrap_or_else(|| panic!("{source}: no data problem: {check:#?}"));
    assert_eq!((problem.code, problem.argument), (code, argument), "{source}");
    let diagnostic = check.rejection_diagnostic().unwrap_or_else(|| panic!("{source}: no rejection"));
    assert_eq!(diagnostic.code, code, "{source}: {diagnostic:#?}");
    assert_eq!(diagnostic.span, check.argument_spans[argument], "{source}");
    assert!(
        check.diagnostics(true).iter().all(|diagnostic| diagnostic.code != INCOMPATIBLE_CALL_CODE),
        "{source}: the data code replaces the generic one"
    );
    assert_eq!(check.rejection_message(), diagnostic.message, "{source}");
    assert!(diagnostic.message.starts_with("load_data(): "), "{source}: {}", diagnostic.message);
    diagnostic.message
}

/// Asserts `source` is `dm.data.invalid_options` at the options literal and
/// that the message names option `key`.
fn assert_invalid_option(source: &str, key: &str) -> String {
    let message = assert_rejected_at(source, INVALID_OPTIONS_CODE, 2);
    let check = load_check(source);
    assert!(source[check.argument_spans[2].clone()].starts_with('{'), "{source}: the span is the options literal");
    assert!(message.contains(&format!("option `{key}`")), "{source}: {message}");
    message
}

/// Asserts the binder refuses `source` at argument 2, with no data problem
/// of the passive check's own, and returns the rejection message.
fn assert_binding_error_at_options(source: &str) -> String {
    let check = load_check(source);
    assert!(check.is_rejected(), "{source}: {check:#?}");
    assert_eq!(check.data_problem, None, "{source}: the binder's error is the rejection");
    let diagnostic = check.rejection_diagnostic().unwrap_or_else(|| panic!("{source}: no rejection"));
    assert_eq!(diagnostic.code, INCOMPATIBLE_CALL_CODE, "{source}: {diagnostic:#?}");
    assert_eq!(diagnostic.span, check.argument_spans[2], "{source}: {diagnostic:#?}");
    let options = check.arguments.iter().find(|argument| argument.index == 2).expect("an options argument");
    assert_eq!(options.category, ArgumentCategory::Incompatible, "{source}: {check:#?}");
    // The binder runs only when every argument is a literal.
    if let Some(binding) = &check.binding_error {
        assert!(binding.argument_errors().all(|error| error.index == 2), "{source}: {binding:#?}");
        assert!(binding.argument_errors().next().is_some(), "{source}: {binding:#?}");
    }
    check.rejection_message()
}

fn type_text(source: &str) -> String {
    let declared = DeclaredTypes::new(None);
    let scope = TypeScope::unguarded(&declared);
    ArgumentType::Typed(expression_type(&parse_spanned(source).unwrap(), &scope)).to_string()
}

mod headerless {
    use super::*;

    #[test]
    fn header_false_with_object_contract_is_rejected_at_as() {
        for file in ["'rows.csv'", "'rows.tsv'"] {
            let source = format!("load_data({file}, '{{name: string}}[]', {{header: false}})");
            let message = assert_rejected_at(&source, ROW_SHAPE_CONFLICT_CODE, 1);
            assert!(message.contains("name: string"), "{message}");
        }
        // The same contract with a header reads object rows.
        assert_accepted("load_data('rows.csv', '{name: string}[]', {header: true})");
        assert_accepted("load_data('rows.csv', '{name: string}[]')");
    }

    #[test]
    fn header_false_with_literal_csv_format_and_computed_file_is_rejected() {
        assert_rejected_at("load_data(f, '{name: string}[]', {format: 'csv', header: false})", ROW_SHAPE_CONFLICT_CODE, 1);
        assert_rejected_at("load_data(f, '{name: string}[]', {format: 'tsv', header: false})", ROW_SHAPE_CONFLICT_CODE, 1);
        // Without a literal format the computed file proves nothing.
        assert_accepted("load_data(f, '{name: string}[]', {header: false})");
    }

    #[test]
    fn header_false_with_provisional_import_is_not_judged() {
        assert_accepted("load_data('rows.csv', 'Row@./row.yaml', {header: false})");
        assert_accepted("load_data('rows.csv', 'Row[]@./row.yaml', {header: false})");
        assert_accepted("load_data(f, 'Row@./row.yaml', {format: 'csv', header: false})");
    }

    #[test]
    fn header_false_with_list_rows_passes() {
        assert_accepted("load_data('rows.csv', 'tuple([string, string])[]', {header: false})");
        assert_accepted("load_data('rows.tsv', 'tuple([string, number])[]', {header: false})");
    }

    /// The row-shape rule applies only to tabular formats: `format: "json"`
    /// overrides a `.csv` extension, so an object contract is accepted.
    #[test]
    fn json_format_on_a_csv_file_makes_the_row_rule_inapplicable() {
        assert_accepted("load_data('rows.csv', '{name: string}', {format: 'json'})");
        assert_accepted("load_data('rows.csv', 'string[]', {format: 'json'})");
        assert_rejected_at("load_data('rows.csv', 'string[]')", ROW_SHAPE_CONFLICT_CODE, 1);
        // `header` cannot apply to JSON at all, whatever the extension says.
        assert_invalid_option("load_data('rows.csv', '{name: string}[]', {format: 'json', header: false})", "header");
    }
}

mod option_values {
    use super::*;

    #[test]
    fn tsv_with_non_tab_delimiter_is_invalid_options() {
        for source in [
            "load_data(f, 'tuple([string])[]', {format: 'tsv', delimiter: ','})",
            "load_data('rows.tsv', 'tuple([string])[]', {delimiter: ';'})",
            "load_data('rows.csv', 'tuple([string])[]', {format: 'tsv', delimiter: '|'})",
        ] {
            let message = assert_invalid_option(source, "delimiter");
            assert!(message.contains("format \"csv\""), "{source}: {message}");
        }
        assert_accepted("load_data(f, 'tuple([string])[]', {format: 'tsv', delimiter: '\\t'})");
        assert_accepted("load_data('rows.csv', 'tuple([string])[]', {delimiter: '\\t'})");
    }

    #[test]
    fn each_invalid_delimiter_is_invalid_options_at_argument_2() {
        let mut messages = Vec::new();
        // Escapes are the expression language's, so `\\"` is a quote.
        for delimiter in ["''", "',,'", "'é'", "'\\\"'", "'\\r'", "'\\n'", "'\\r\\n'"] {
            for file in ["'rows.csv'", "f"] {
                let source = format!("load_data({file}, 'tuple([string])[]', {{delimiter: {delimiter}}})");
                let message = assert_invalid_option(&source, "delimiter");
                assert!(message.contains("one ASCII character"), "{source}: {message}");
                messages.push(message);
            }
        }
        messages.dedup();
        assert_eq!(messages.len(), 1, "every bad delimiter shares one message: {messages:#?}");
        for good in ["','", "';'", "'|'", "'\\t'", "'5'"] {
            assert_accepted(&format!("load_data('rows.csv', 'tuple([string])[]', {{delimiter: {good}}})"));
        }
    }

    #[test]
    fn tabular_options_on_json_are_invalid_options() {
        for (options, key) in [
            ("{delimiter: ';'}", "delimiter"),
            ("{header: true}", "header"),
            ("{header: false}", "header"),
            ("{delimiter: ',', header: false}", "delimiter"),
        ] {
            for (file, format) in [("'rows.json'", ""), ("'rows.csv'", "format: 'json', "), ("f", "format: 'json', ")] {
                let options = if format.is_empty() { options.to_string() } else { options.replacen('{', &format!("{{{format}"), 1) };
                let source = format!("load_data({file}, 'string[]', {options})");
                let message = assert_invalid_option(&source, key);
                assert!(message.contains("this call reads JSON"), "{source}: {message}");
            }
        }
    }

    #[test]
    fn null_header_on_a_json_file_is_accepted() {
        assert_accepted("load_data('rows.json', 'string[]', {header: null})");
        assert_accepted("load_data('rows.json', 'string[]', {delimiter: null, header: null, format: null})");
        assert_accepted("load_data('rows.json', 'string[]', null)");
        assert_accepted("load_data('rows.json', 'string[]', {})");
    }

    /// An alias never reaches a message: the canonical format is named.
    #[test]
    fn yml_alias_with_a_delimiter_names_yaml() {
        let message = assert_invalid_option("load_data(f, 'string[]', {format: 'yml', delimiter: ','})", "delimiter");
        assert!(message.contains("this call reads YAML"), "{message}");
        assert!(!message.contains("yml"), "{message}");
        let ndjson = assert_invalid_option("load_data(f, 'string[]', {format: 'ndjson', header: true})", "header");
        assert!(!ndjson.contains("ndjson"), "{ndjson}");
    }

    /// Step 1 of the verdict order never looks at the file, and a literal
    /// format makes applicability provable beside a `null` file.
    #[test]
    fn literal_mistake_with_null_file_is_rejected() {
        assert_invalid_option("load_data(null, 'tuple([string])[]', {delimiter: ';;'})", "delimiter");
        assert_invalid_option("load_data(null, 'string[]', {format: 'json', delimiter: ';'})", "delimiter");
        assert_rejected_at("load_data(null, '{a: string}[]', {format: 'csv', header: false})", ROW_SHAPE_CONFLICT_CODE, 1);
        // With no literal format, a null file proves no format.
        assert_accepted("load_data(null, 'string[]', {delimiter: ';'})");
        assert_accepted("load_data(null, 'string[]', {format: 'json'})");
        // A literal tabular format applies the row rule too.
        assert_rejected_at("load_data(null, 'string[]', {format: 'csv'})", ROW_SHAPE_CONFLICT_CODE, 1);
    }

    #[test]
    fn a_dead_ternary_branch_with_a_bad_literal_delimiter_is_still_rejected() {
        for source in [
            "false ? load_data('rows.csv', 'tuple([string])[]', {delimiter: ';;'}) : null",
            "true ? null : load_data(f, 'tuple([string])[]', {delimiter: ''})",
        ] {
            let check = load_check(source);
            assert!(check.is_rejected(), "{source}: {check:#?}");
            let problem = check.data_problem.as_ref().expect("a data problem");
            assert_eq!((problem.code, problem.argument), (INVALID_OPTIONS_CODE, 2), "{source}");
            assert!(problem.message.contains("option `delimiter`"), "{}", problem.message);
        }
    }
}

mod binder {
    use super::*;

    #[test]
    fn unknown_option_key_is_a_binding_error_at_argument_2() {
        for source in [
            "load_data('rows.csv', 'tuple([string])[]', {quote: '\"'})",
            "load_data('rows.json', 'string[]', {quote: ',', delimiter: ';'})",
            "load_data(f, 'string[]', {Format: 'csv'})",
        ] {
            let message = assert_binding_error_at_options(source);
            assert!(message.contains("argument 3"), "{source}: {message}");
        }
    }

    #[test]
    fn unbindable_option_field_is_a_binding_error_at_argument_2() {
        for source in [
            "load_data('rows.csv', 'tuple([string])[]', {delimiter: [',']})",
            "load_data('rows.csv', 'tuple([string])[]', {delimiter: {}})",
            "load_data('rows.csv', 'tuple([string])[]', {header: [1]})",
            "load_data('rows.csv', 'tuple([string])[]', {format: ['csv']})",
            "load_data('rows.csv', 'tuple([string])[]', 5)",
            "load_data('rows.csv', 'tuple([string])[]', [])",
        ] {
            assert_binding_error_at_options(source);
        }
    }

    #[test]
    fn invalid_format_names_are_binding_errors() {
        for name in ["CSV", " csv", "csv ", "", "text", "YML", "markdown"] {
            let source = format!("load_data('rows.json', 'string[]', {{format: '{name}', header: true}})");
            assert_binding_error_at_options(&source);
        }
    }
}

mod deferral {
    use super::*;

    /// A computed options value may set `format`, overriding the extension.
    #[test]
    fn computed_options_beside_a_literal_json_file_are_not_judged() {
        assert_accepted("load_data('rows.json', 'string[]', opts)");
        assert_accepted("load_data('rows.json', '{a: string}', opts)");
        assert_accepted("load_data('rows.csv', 'string[]', opts)");
        // The contract itself is still judged.
        assert_rejected_at("load_data('rows.json', 'string', opts)", NOT_A_CONTAINER_CODE, 1);
    }

    #[test]
    fn partial_object_with_computed_format_defers_format_checks() {
        for source in [
            "load_data('rows.json', 'string[]', {format: x, delimiter: ';'})",
            "load_data('rows.json', 'string[]', {format: x, header: true})",
            "load_data('rows.csv', '{a: string}[]', {format: x, header: false})",
            "load_data('rows.csv', 'string[]', {format: x})",
            "load_data('rows.tsv', 'tuple([string])[]', {format: x, delimiter: ','})",
        ] {
            assert_accepted(source);
        }
        // The format-free delimiter rule is not deferred.
        assert_invalid_option("load_data('rows.json', 'string[]', {format: x, delimiter: ';;'})", "delimiter");
    }

    #[test]
    fn literal_delimiter_beside_computed_member_is_still_judged() {
        assert_invalid_option("load_data('rows.json', 'string[]', {header: h, delimiter: ';'})", "delimiter");
        assert_invalid_option("load_data('rows.tsv', 'tuple([string])[]', {header: h, delimiter: ','})", "delimiter");
        assert_invalid_option("load_data('rows.json', 'string[]', {delimiter: d, header: false})", "header");
        assert_rejected_at("load_data('rows.csv', '{a: string}[]', {delimiter: d, header: false})", ROW_SHAPE_CONFLICT_CODE, 1);
        // A computed header is judged at execution.
        assert_accepted("load_data('rows.csv', '{a: string}[]', {header: h, delimiter: ';'})");
    }
}

mod messages {
    use super::*;

    fn write(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    /// The `DataError` a composition of `content` in `dir` fails with.
    fn execution_error(dir: &Path, content: &str) -> String {
        let doc = write(dir, "doc.md", content);
        let options = ComposeOptions::new().with_source_file(&doc).with_data_runtime(DataRuntime::new());
        let request = crate::request_support::request(options);
        let failure: Box<ComposeFailure> =
            Markdown::try_from(doc.as_path()).unwrap().compose_with_warnings(&request).expect_err("the options fail");
        let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&failure.error);
        while let Some(error) = current {
            let expression = error
                .downcast_ref::<ExpressionError>()
                .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref));
            if let Some(ExpressionError::Data(data)) = expression.map(ExpressionError::without_callbacks) {
                assert!(matches!(**data, DataError::InvalidOptions { .. }), "{data:?}");
                return data.to_string();
            }
            current = error.source();
        }
        panic!("no data error in: {}", failure.error);
    }

    /// The literal option is caught passively; the same value read from
    /// frontmatter (so computed) fails at execution with the same text.
    #[test]
    fn passive_and_execution_messages_are_identical() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "rows.json", "[\"a\"]\n");
        write(dir.path(), "rows.tsv", "a\tb\n");
        for (file, contract, options, computed) in [
            ("rows.json", "string[]", "{delimiter: ';'}", "---\nd: \";\"\n---\n{{ load_data('rows.json', 'string[]', {delimiter: d}) }}\n"),
            ("rows.json", "string[]", "{header: false}", "---\nh: false\n---\n{{ load_data('rows.json', 'string[]', {header: h}) }}\n"),
            ("rows.tsv", "tuple([string, string])[]", "{delimiter: ','}", "---\nd: \",\"\n---\n{{ load_data('rows.tsv', 'tuple([string, string])[]', {delimiter: d}) }}\n"),
            ("rows.json", "string[]", "{delimiter: ';;'}", "---\nd: \";;\"\n---\n{{ load_data('rows.json', 'string[]', {delimiter: d}) }}\n"),
        ] {
            let source = format!("load_data('{file}', '{contract}', {options})");
            let passive = load_check(&source).data_problem.expect("a passive problem").message;
            assert_eq!(passive, execution_error(dir.path(), computed), "{source}");
        }
    }
}

mod typing {
    use super::*;

    #[test]
    fn result_type_is_the_contract_type_for_both_arities() {
        for (file, contract) in
            [("'rows.csv'", "tuple([string, number])[]"), ("'rows.json'", "{a: string}[]"), ("'rows.yaml'", "string[] | null")]
        {
            let two = type_text(&format!("load_data({file}, '{contract}')"));
            assert!(!two.contains("unknown"), "{two}");
            for options in ["{}", "null", "opts", "{format: x}", "{header: null}"] {
                assert_eq!(type_text(&format!("load_data({file}, '{contract}', {options})")), two, "{options}");
            }
        }
        let headerless = type_text("load_data('rows.csv', 'tuple([string, number])[]', {header: false, delimiter: ';'})");
        assert_eq!(headerless, type_text("load_data('rows.csv', 'tuple([string, number])[]')"));
        assert_eq!(type_text("load_data('rows.csv', t, {header: false})"), "unknown");
    }

    #[test]
    fn null_file_typing_is_unchanged_with_options() {
        for options in ["{format: 'csv'}", "{delimiter: ';'}", "opts", "null"] {
            assert_eq!(type_text(&format!("load_data(null, 'string[]', {options})")), "null", "{options}");
            assert_eq!(
                type_text(&format!("load_data(f, '{{a: string}}[]', {options})")),
                type_text("load_data(f, '{a: string}[]')"),
                "{options}"
            );
        }
        assert!(type_text("load_data(f, '{a: string}[]', opts)").ends_with(" | null"));
    }

    #[test]
    fn callback_member_typing_is_unchanged_with_header_false() {
        let declared = DeclaredTypes::new(None);
        let csv = "load_data('rows.csv', 'tuple([string, number])[]', {header: false})";
        assert_eq!(type_text(&format!("map({csv}, r => r)")), type_text("map(load_data('rows.csv', 'tuple([string, number])[]'), r => r)"));
        // Object rows keep member typing through options that apply.
        let object = "load_data('rows.csv', '{ name: string, port: number }[]', {delimiter: ';', header: true})";
        let named = type_text(&format!("map({object}, r => r.name)"));
        assert!(named.contains("string") && named.ends_with("[]"), "{named}");
        let source = format!("map({object}, r => r.nmae)");
        let found = expression_call_diagnostics(&parse_spanned(&source).unwrap(), &TypeScope::unguarded(&declared), true);
        assert!(found.is_empty(), "{found:#?}");
        let min_of = |callback: &str| {
            let source = format!("map(load_data('rows.csv', '{{ flag: boolean(required), port: number(required) }}[]', {{delimiter: ';'}}), r => {callback})");
            checks(&source).into_iter().find(|check| check.function == "min").expect("a min call").category()
        };
        assert_eq!(min_of("min(r.flag, 5)"), ArgumentCategory::Incompatible);
        assert_eq!(min_of("min(r.port, 5)"), ArgumentCategory::Compatible);
    }
}
