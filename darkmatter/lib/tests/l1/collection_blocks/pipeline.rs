//! Output bytes, stage schedule, data provenance, and failure policy, run for
//! both kinds.

use std::sync::{Arc, Mutex};

use darkmatter::markdown::compose::collection_blocks::CollectionKind;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeOperation, ShellExpansionError};
use serde_json::{Value, json};
use tempfile::TempDir;

use super::support::{KINDS, closer, composed_raw, opener, row_tests};
use crate::collection_execution::{any_items, codes, collection_error, compose, compose_configured, compose_with_options, composed};

/// A document whose `data` holds `items` and whose body is `template` with
/// `{OPEN}` / `{CLOSE}` replaced by the kind's directive lines.
fn templated(kind: CollectionKind, items: Value, template: &str) -> String {
    let body = template.replace("{OPEN}", &opener(kind)).replace("{CLOSE}", &closer(kind));
    any_items(items, &body)
}

// ── exact bytes with cleanup disabled ────────────────────────────────────

/// `(row, body template, items, exact output)`. Every row runs for both kinds
/// with no callback (map is the identity, filter retains falsey items).
const EXACT: &[(&str, &str, &str, &str)] = &[
    ("tabs_and_trailing_spaces", "{OPEN} data\n  x\t{{i}}  \n{CLOSE}\n", r#"["a","b"]"#, "  x\ta  \n  x\tb  \n"),
    ("blank_lines_inside_the_body", "{OPEN} data\nx\n\n{{i}}\n{CLOSE}\n", r#"["a","b"]"#, "x\n\na\nx\n\nb\n"),
    ("surrounding_text_gets_no_separators", "before\n{OPEN} data\n{{i}}\n{CLOSE}\nafter\n", r#"["a","b"]"#, "before\na\nb\nafter"),
    ("an_empty_source_removes_only_its_own_lines", "before\n{OPEN} data\n{{i}}\n{CLOSE}\nafter\n", "[]", "before\nafter"),
    ("an_empty_body_removes_only_its_own_lines", "before\n{OPEN} data\n{CLOSE}\nafter\n", r#"["a"]"#, "before\nafter"),
    ("a_closer_with_trailing_whitespace", "{OPEN} data\n{{i}}\n{CLOSE} \t \nafter\n", r#"["a"]"#, "a\nafter"),
    ("a_final_line_without_a_newline", "{OPEN} data\n{{i}}\n{CLOSE}", r#"["a","b"]"#, "a\nb\n"),
    ("indented_list_container", "- item\n  {OPEN} data\n  - {{i}}\n  {CLOSE}\n", r#"["a","b"]"#, "- item\n  - a\n  - b\n"),
    ("block_quote_container", "> {OPEN} data\n> {{i}}\n> {CLOSE}\n", r#"["a","b"]"#, "> a\n> b\n"),
    ("a_body_without_a_final_blank_line_joins_the_next_copy", "{OPEN} data\nx{{i}}\n{CLOSE}\n", r#"["a","b"]"#, "xa\nxb\n"),
];

fn exact_row(name: &str) {
    let (_, template, items, expected) = EXACT.iter().find(|(row, ..)| *row == name).unwrap();
    let items: Value = serde_json::from_str(items).unwrap();
    for kind in KINDS {
        let document = templated(kind, items.clone(), template);
        assert_eq!(composed_raw(&document), *expected, "{name} {kind:?}");
    }
}

row_tests! { EXACT_TESTS, exact_row;
    tabs_and_trailing_spaces, blank_lines_inside_the_body, surrounding_text_gets_no_separators,
    an_empty_source_removes_only_its_own_lines, an_empty_body_removes_only_its_own_lines,
    a_closer_with_trailing_whitespace, a_final_line_without_a_newline, indented_list_container,
    block_quote_container, a_body_without_a_final_blank_line_joins_the_next_copy,
}

#[test]
fn every_generated_exact_test_has_a_table_row_and_back() {
    let table: Vec<_> = EXACT.iter().map(|(name, ..)| *name).collect();
    assert_eq!(EXACT_TESTS, table.as_slice());
}

#[test]
fn crlf_bodies_repeat_with_their_line_endings_when_cleanup_is_off() {
    for kind in KINDS {
        let document = templated(kind, json!(["a", "b"]), "{OPEN} data\n- {{i}}\n{CLOSE}\n").replace('\n', "\r\n");
        let output = composed_raw(&document);
        assert!(output.contains("- a") && output.contains("- b"), "{kind:?}: {output:?}");
        assert!(!output.contains("::"), "{kind:?}: {output:?}");
    }
}

#[test]
fn normal_cleanup_runs_after_the_expansion_for_both_kinds() {
    for kind in KINDS {
        let document = templated(kind, json!(["a", "b"]), "# Title\n{OPEN} data\n- {{i}}\n{CLOSE}\n\nText\n");
        assert_eq!(composed(&document), "# Title\n\n- a\n- b\n\nText", "{kind:?}");
    }
}

// ── data provenance ──────────────────────────────────────────────────────

/// Records every shell approval request and denies it, so no command runs.
struct Recorder(Mutex<Vec<String>>);

impl ShellApprovalHandler for Recorder {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.lock().unwrap().push(request.raw_command);
        Ok(ShellApprovalDecision::Deny)
    }
}

fn composed_recording(document: &str) -> (Result<String, darkmatter::markdown::MarkdownError>, Vec<String>) {
    let recorder = Arc::new(Recorder(Mutex::default()));
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    let handler = recorder.clone();
    let output = compose_with_options(&root, document, |options| {
        options
            .disable(ComposeOperation::Cleanup)
            .with_shell_policy_root(&root)
            .with_shell_working_directory(&root)
            .with_shell_approval_handler(handler)
    });
    let requests = recorder.0.lock().unwrap().clone();
    (output, requests)
}

#[test]
fn inserted_directive_closer_and_fence_text_stays_data_for_both_kinds() {
    let payloads = [
        "::end-map",
        "::end-filter",
        "::map data",
        "::filter data",
        "::block when=\"false\"",
        "::end-block",
        "```",
        "~~~",
        "::file ./missing.md",
        "::shell echo INJECTED",
        "::shell-block",
    ];
    for kind in KINDS {
        let document = templated(kind, json!(payloads), "{OPEN} data\n- {{i}}\n{CLOSE}\n");
        let (output, requests) = composed_recording(&document);
        let expected: String = payloads.iter().map(|payload| format!("- {payload}\n")).collect();
        assert_eq!(output.unwrap_or_else(|error| panic!("{kind:?}: {error}")), expected, "{kind:?}");
        assert!(requests.is_empty(), "{kind:?}: inserted text asked for approval: {requests:?}");
    }
}

#[test]
fn an_inserted_interpolation_is_not_evaluated_again() {
    for kind in KINDS {
        let document = format!(
            "---\n$schema:\n  data: string[](required)\n  tmpl: string(required)\ndata: [a]\ntmpl: \"@{{ 1 + 1 }}\"\n---\n{}",
            super::support::region(kind, "data", "- {{i}} {{ fm.tmpl }}\n"),
        );
        let output = composed_raw(&document);
        assert!(!output.contains("@2"), "{kind:?}: {output:?}");
        assert!(output.starts_with("- a "), "{kind:?}: {output:?}");
    }
}

#[test]
fn an_authored_shell_directive_in_a_body_asks_once_per_template_not_per_item_for_both_kinds() {
    for kind in KINDS {
        let document = templated(kind, json!(["one", "two", "three"]), "{OPEN} data\n::shell echo {{ i }}\n{CLOSE}\n");
        let (output, requests) = composed_recording(&document);
        assert!(output.is_err(), "{kind:?}: the recorder denied the command");
        assert_eq!(requests.len(), 1, "{kind:?}: {requests:?}");
        assert!(requests[0].starts_with("echo"), "{kind:?}: {requests:?}");
    }
}

#[test]
fn an_authored_shell_block_closes_on_its_own_closer_inside_either_kind() {
    for kind in KINDS {
        let document = templated(
            kind,
            json!(["one", "two"]),
            "{OPEN} data\n::shell-block\necho {{ i }}\n::end-block\nafter {{ i }}\n{CLOSE}\n",
        );
        let (output, requests) = composed_recording(&document);
        assert!(output.is_err(), "{kind:?}");
        assert_eq!(requests.len(), 1, "{kind:?}: {requests:?}");
    }
}

// ── operation subsets ────────────────────────────────────────────────────

#[test]
fn the_collection_operation_alone_repeats_the_body_without_interpolating_it() {
    for kind in KINDS {
        let document = templated(kind, json!(["a", "b"]), "{OPEN} data\n- {{i}}\n{CLOSE}\n");
        let output = compose_configured(&document, |options| options.only(&[ComposeOperation::CollectionBlocks])).unwrap();
        assert_eq!(output, "- {{i}}\n- {{i}}\n", "{kind:?}");
    }
}

#[test]
fn without_page_blocks_a_conditional_inside_a_body_stays_text_but_the_collection_runs() {
    for kind in KINDS {
        let document = templated(kind, json!(["a"]), "{OPEN} data\n::block when=\"false\"\nx\n::end-block\n{CLOSE}\n");
        let output = compose_configured(&document, |options| options.disable(ComposeOperation::PageBlocks)).unwrap();
        assert!(output.contains("::block when=\"false\"") && output.contains("::end-block"), "{kind:?}: {output:?}");
        assert!(!output.contains(&opener(kind)), "{kind:?}: {output:?}");
    }
}

#[test]
fn the_other_kinds_operation_is_one_switch_for_both_directives() {
    for kind in KINDS {
        let document = templated(kind, json!(["a"]), "{OPEN} data\n- {{i}}\n{CLOSE}\n");
        let output = compose_configured(&document, |options| options.disable(ComposeOperation::CollectionBlocks)).unwrap();
        assert!(output.contains(&opener(kind)) && output.contains(&closer(kind)), "{kind:?}: {output:?}");
    }
}

// ── failure policy ───────────────────────────────────────────────────────

fn child_dir(child: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::write(root.join("child.md"), child).unwrap();
    (dir, root)
}

#[test]
fn collection_failures_in_a_child_stay_fatal_through_lenient_transclusion_for_both_kinds() {
    for kind in KINDS {
        let static_failure = format!("---\nnote: text\n---\n{}", super::support::region(kind, "note", "- {{i}}\n"));
        // `any[]` makes the pattern a runtime check, so the failure is reached.
        let runtime_failure = format!(
            "---\n$schema:\n  data: any[](required)\ndata: [1]\n---\n{}",
            // The input pattern fails at runtime: `1` has no field `a`.
            super::support::region(kind, &format!("data \"{}\"", if kind == CollectionKind::Map { "{a} => a" } else { "{a} => true" }), "- x\n"),
        );
        for (name, child, code) in [
            ("static", static_failure, "dm.collection.untyped_map_source"),
            ("runtime", runtime_failure, "dm.collection.pattern_mismatch"),
        ] {
            let (_dir, root) = child_dir(&child);
            let error = compose_with_options(&root, "before\n\n::file ./child.md\n", |options| {
                options.with_ignore_invalid_references(Some(true))
            })
            .unwrap_err();
            let expected = if name == "static" && kind == CollectionKind::Filter {
                "dm.collection.untyped_filter_source"
            } else {
                code
            };
            assert_eq!(codes(&error), [expected], "{name} {kind:?}: {error}");
        }
    }
}

#[test]
fn an_ordinary_missing_child_keeps_its_read_policy_inside_a_body() {
    for kind in KINDS {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::write(root.join("a.md"), "child A\n").unwrap();
        std::fs::write(root.join("b.md"), "child B\n").unwrap();
        let document = templated(kind, json!(["a.md", "gone.md", "b.md"]), "{OPEN} data\n::file {{i}}\n{CLOSE}\n");
        assert!(compose_with_options(&root, &document, |options| options).is_err(), "{kind:?}: strict reads fail");
        let lenient = compose_with_options(&root, &document, |options| options.with_ignore_invalid_references(Some(true)))
            .unwrap_or_else(|error| panic!("{kind:?}: {error}"));
        assert!(lenient.contains("child A") && lenient.contains("child B"), "{kind:?}: {lenient}");
    }
}

#[test]
fn a_collection_failure_inside_nested_composition_stays_typed_through_a_handler_while_ordinary_failures_do_not() {
    for kind in KINDS {
        let nested = format!("---\\nnote: text\\n---\\n::{} note\\n- {{{{i}}}}\\n::{}\\n", kind.opener(), kind.closer());
        let document = any_items(json!([1]), &format!("{{{{ as_markdown(\"{nested}\") || 'fallback' }}}}\n"));
        let error = compose(&document).unwrap_err();
        let expected = match kind {
            CollectionKind::Map => "dm.collection.untyped_map_source",
            CollectionKind::Filter => "dm.collection.untyped_filter_source",
        };
        assert_eq!(codes(&error), [expected], "{kind:?}: the collection finding survives the handler: {error}");
    }
    let ordinary = any_items(json!([1]), "{{ as_markdown(\"---\\nx: 1\\n---\\n::file ./gone.md\\n\") || 'fallback' }}\n");
    let error = compose(&ordinary).unwrap_err();
    assert!(matches!(error, darkmatter::markdown::MarkdownError::Interpolation { .. }), "{error:?}");
}

#[test]
fn budget_exhaustion_cannot_be_absorbed_by_a_fallback_for_either_kind_or_budget() {
    for kind in KINDS {
        let nested = format!("---\\n$schema:\\n  d: 'string[](required)'\\nd: [a, b, c]\\n---\\n::{} d\\n- {{{{i}}}}\\n::{}\\n", kind.opener(), kind.closer());
        let document = any_items(json!([1]), &format!("{{{{ as_markdown(\"{nested}\") || 'fallback' }}}}\n"));
        let visits = compose_configured(&document, |options| options.with_max_iteration_visits(std::num::NonZeroU64::new(2).unwrap()))
            .unwrap_err();
        assert_eq!(codes(&visits), ["dm.iteration.visit_limit"], "{kind:?}");
        let bytes = compose_configured(&document, |options| options.with_max_expansion_bytes(std::num::NonZeroU64::new(5).unwrap()))
            .unwrap_err();
        assert_eq!(codes(&bytes), ["dm.iteration.expansion_limit"], "{kind:?}");
    }
}

#[test]
fn warnings_reported_before_a_failure_are_retained_once() {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};

    for kind in KINDS {
        let document = format!(
            "---\n$schema:\n  data: any[](required)\nitems: [1]\ndata: [1]\n---\n{}{}",
            super::support::region(kind, "items", "- {{i}}\n"),
            // The input pattern fails at runtime: `1` has no field `a`.
            super::support::region(kind, &format!("data \"{}\"", if kind == CollectionKind::Map { "{a} => a" } else { "{a} => true" }), "- x\n"),
        );
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let path = root.join("doc.md");
        std::fs::write(&path, &document).unwrap();
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&root, ""))
            .with_source_file(path.clone())
            .with_allow_untyped_map_variables(true)
            .with_allow_untyped_filter_variables(true);
        let failure = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with_warnings(&crate::request_support::request_at(&root, options))
            .expect_err("the second region fails at runtime");
        assert_eq!(codes(&failure.error), ["dm.collection.pattern_mismatch"], "{kind:?}");
        let expected = match kind {
            CollectionKind::Map => "dm.collection.untyped_map_source",
            CollectionKind::Filter => "dm.collection.untyped_filter_source",
        };
        let kept: Vec<_> = failure.warnings.iter().filter(|warning| warning.code.as_deref() == Some(expected)).collect();
        assert_eq!(kept.len(), 1, "{kind:?}: {:?}", failure.warnings);
    }
}

#[test]
fn the_failing_position_after_a_sort_is_the_original_index_for_both_kinds_and_orders() {
    for kind in KINDS {
        for order in ["asc", "desc"] {
            let document = templated(
                kind,
                json!([3, 1, null, 2]),
                &format!("{{OPEN}} data sort=\"v => v\" sort-order=\"{order}\"\nx\n{{CLOSE}}\n"),
            );
            let error = compose(&document).unwrap_err();
            assert_eq!(collection_error(&error).iteration_path().to_string(), "data[2]", "{kind:?} {order}");
        }
    }
}

#[test]
fn a_runtime_pattern_failure_names_the_original_item_for_both_kinds() {
    for kind in KINDS {
        let document = templated(kind, json!([{"a": 1}, {"a": 2}, {"b": 3}]), "{OPEN} data \"{a} => {a}\"\nx\n{CLOSE}\n");
        let document = if kind == CollectionKind::Filter { document.replace("=> {a}", "=> true") } else { document };
        let error = compose(&document).unwrap_err();
        assert_eq!(collection_error(&error).iteration_path().to_string(), "data[2]", "{kind:?}");
    }
}

// ── stage schedule (needs the logging probes) ────────────────────────────

#[cfg(feature = "effects-instrumentation")]
mod instrumented {
    use super::*;
    use crate::collection_effects::{log, probes};

    #[test]
    fn frontmatter_is_resolved_once_however_many_items_a_region_repeats() {
        for kind in KINDS {
            let (_probe, _boom) = probes();
            let document = format!(
                "---\n$schema:\n  data: any[](required)\n  title: string(required)\ndata: [1, 2, 3]\ntitle: \"{{{{ probe('frontmatter', 'x') }}}}\"\n---\n{}",
                super::super::support::region(kind, "data", "- {{title}} {{i}}\n"),
            );
            let output = composed(&document);
            assert_eq!(output, "- x 1\n- x 2\n- x 3\n", "{kind:?}");
            assert_eq!(log().iter().filter(|entry| entry.starts_with("frontmatter:")).count(), 1, "{kind:?}: {:?}", log());
        }
    }

    #[test]
    fn a_frontmatter_only_request_leaves_regions_dormant_and_runs_no_callback_for_both_kinds() {
        for kind in KINDS {
            let (_probe, _boom) = probes();
            let callback = if kind == CollectionKind::Map { "i => probe('cb', i)" } else { "i => probe('cb', true)" };
            let document = templated(
                kind,
                json!([1, 2]),
                &format!("{{OPEN}} data \"{callback}\" when=\"probe('gate', true)\"\n- {{{{ probe('body', i) }}}}\n{{CLOSE}}\n"),
            );
            let output = compose_configured(&document, |options| options.only_frontmatter_surface()).unwrap();
            assert!(output.contains(&opener(kind)) && output.contains(&closer(kind)), "{kind:?}: {output}");
            assert!(log().is_empty(), "{kind:?}: {:?}", log());
        }
    }

    #[test]
    fn a_false_gate_in_either_kind_runs_neither_callbacks_nor_the_body() {
        for kind in KINDS {
            let (_probe, _boom) = probes();
            let callback = if kind == CollectionKind::Map { "i => probe('cb', i)" } else { "i => probe('cb', true)" };
            let document = templated(
                kind,
                json!([1, 2]),
                &format!("{{OPEN}} data \"{callback}\" when=\"false\"\n- {{{{ probe('body', i) }}}}\n{{CLOSE}}\n"),
            );
            assert_eq!(composed_raw(&document), "", "{kind:?}");
            assert!(log().is_empty(), "{kind:?}: {:?}", log());
        }
    }

    #[test]
    fn every_callback_for_every_item_finishes_before_the_first_body_expression_in_either_kind() {
        for kind in KINDS {
            let (_probe, _boom) = probes();
            let callback = if kind == CollectionKind::Map { "i => probe('cb', i)" } else { "i => probe('cb', true)" };
            let document = templated(
                kind,
                json!([1, 2, 3]),
                &format!("{{OPEN}} data \"{callback}\" sort=\"i => i\" sort-order=\"desc\"\n- {{{{ probe('body', i) }}}}\n{{CLOSE}}\n"),
            );
            composed(&document);
            let entries = log();
            let last_callback = entries.iter().rposition(|entry| entry.starts_with("cb:")).unwrap();
            let first_body = entries.iter().position(|entry| entry.starts_with("body:")).unwrap();
            assert!(last_callback < first_body, "{kind:?}: {entries:?}");
        }
    }
}

// ── semantic caches ──────────────────────────────────────────────────────

#[test]
fn a_cached_child_is_never_replayed_for_a_different_local_binding() {
    for kind in KINDS {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::write(root.join("child.md"), "---\n$schema:\n  who: string(required)\n---\nhello {{ who }}\n").unwrap();
        let document = templated(kind, json!(["ada", "bob", "ada"]), "{OPEN} data\n::file ./child.md set.who=\"{{ i }}\"\n{CLOSE}\n");
        let output = compose_with_options(&root, &document, |options| options.disable(ComposeOperation::Cleanup))
            .unwrap_or_else(|error| panic!("{kind:?}: {error}"));
        let greetings: Vec<_> = output.lines().filter(|line| line.starts_with("hello")).collect();
        assert_eq!(greetings, ["hello ada", "hello bob", "hello ada"], "{kind:?}: {output}");
    }
}
