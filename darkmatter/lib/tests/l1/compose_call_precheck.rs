//! Composition rejects a document whose authored syntax makes a call the
//! design-time check proves wrong (category 5), before any of its expressions
//! is evaluated, in every branch, on the surface it composes.
//!
//! Every test composes a real document in its own temporary directory through
//! the public compose API; test-registered functions count handler entries to
//! prove that nothing ran.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
use darkmatter::markdown::compose::expression::{
    CallCheck, DeclaredTypes, ExpressionError, TypeScope, check_expression_calls, functions, parse_spanned,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::parse_yaml_schema;
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::json;
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn compose_in(dir: &Path, content: &str, frontmatter_only: bool) -> Result<Markdown, MarkdownError> {
    let path = write(dir, "doc.md", content);
    let mut options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone());
    if frontmatter_only {
        options = options.only_frontmatter_surface();
    }
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir, options))
        .map(|(composed, _)| composed)
}

/// Composes `content` as `doc.md` in a fresh directory.
fn compose(content: &str) -> Result<Markdown, MarkdownError> {
    let dir = TempDir::new().unwrap();
    compose_in(dir.path(), content, false)
}

/// The call check a rejection carries, wherever the stage reports it.
fn call_check(error: &MarkdownError) -> Option<&CallCheck> {
    error.rejected_call()
}

/// Composes `content` and returns the call check that rejected it.
#[track_caller]
fn rejected(content: &str) -> CallCheck {
    match compose(content) {
        Ok(composed) => panic!("{content:?} composed:\n{}", composed.content()),
        Err(error) => call_check(&error)
            .unwrap_or_else(|| panic!("{content:?} failed without a call check: {error}"))
            .clone(),
    }
}

#[track_caller]
fn composed(content: &str) -> String {
    compose(content).unwrap_or_else(|error| panic!("{content:?} failed: {error}")).content().trim_end().to_string()
}

/// A zero-argument test function counting its entries.
fn effect() -> TestFunctionGuard {
    let registration = FunctionRegistration::builder("effect", "function(parameters(void); returns(boolean))")
        .build()
        .expect("the effect signature compiles");
    hooks::register(registration, Arc::new(|_| Ok(json!(true))))
}

const FLAG: &str = "$schema:\n  flag: boolean(required)\n  size: number(required)\nflag: true\nsize: 3\n";

/// A document with `FLAG`'s schema, a sibling effect in frontmatter and
/// body, and `extra` frontmatter and `body`.
fn document(extra: &str, body: &str) -> String {
    format!("---\n{FLAG}before: \"{{{{ effect() }}}}\"\n{extra}---\n{{{{ effect() }}}}\n{body}\n")
}

/// Rejected with `effect` and every built-in handler unentered.
#[track_caller]
fn rejected_before_evaluation(content: &str) -> CallCheck {
    let _effect = effect();
    hooks::reset_handler_entries();
    let check = rejected(content);
    assert_eq!(hooks::handler_entries("effect"), 0, "a sibling expression ran: {content:?}");
    assert_eq!(hooks::handler_entries("min"), 0, "min ran: {content:?}");
    check
}

mod rejection {
    use super::*;

    /// Criterion 37: `min(flag, 5)` on a required boolean.
    #[test]
    fn a_proven_bad_call_rejects_before_any_expression_runs() {
        let check = rejected_before_evaluation(&document("bad: \"{{ min(flag, 5) }}\"\n", "text"));
        assert_eq!(check.function, "min");
        assert!(check.is_rejected());
    }

    /// The rejection reads like the runtime type error and names where the
    /// expression is.
    #[test]
    fn the_message_reads_like_the_type_error_and_names_the_key() {
        let _effect = effect();
        let error = compose(&document("bad: \"{{ min(flag, 5) }}\"\n", "text")).unwrap_err();
        let check = call_check(&error).expect("a call check").clone();
        let text = error.to_string();
        assert!(text.contains(&check.rejection_message()), "{text}");
        assert!(text.contains("min(flag, 5)"), "{text}");
        assert!(matches!(&error, MarkdownError::Interpolation { key: Some(key), .. } if key == "bad"), "{error:?}");
    }

    #[test]
    fn a_bad_body_call_rejects_before_frontmatter_runs() {
        let check = rejected_before_evaluation(&document("", "{{ min(flag, 5) }}"));
        assert_eq!(check.function, "min");
    }

    /// Criterion 50: `fm.flag` and `fm["flag"]` are the property `flag`.
    #[test]
    fn frontmatter_namespace_access_rejects_like_the_bare_name() {
        rejected_before_evaluation(&document("", "{{ min(fm.flag, 5) }}"));
        rejected_before_evaluation(&document("", "{{ min(fm[\"flag\"], 5) }}"));
    }

    /// Criterion 37, generated: every registered function, called in a way
    /// the compatibility API proves wrong, rejects composition, and no
    /// handler is entered.
    #[test]
    fn every_registered_function_rejects_a_proven_bad_call() {
        let started = std::time::Instant::now();
        let schema_yaml = "b: boolean(required)\nn: number(required)\ns: string(required)\n";
        let schema = parse_yaml_schema(&serde_yaml_ng::from_str(schema_yaml).unwrap()).unwrap();
        let declared = DeclaredTypes::new(Some(&schema));
        let site = TypeScope::unguarded(&declared);
        let probes = ["b", "n", "s", "'pear'", "true", "null", "[1]"];

        let dir = TempDir::new().unwrap();
        let mut unproven = Vec::new();
        let mut failures = Vec::new();
        let names = functions::dispatchable_canonical_names();
        assert!(names.len() > 50, "{} functions", names.len());
        for name in &names {
            // Typed and literal arguments first; a wrong count last.
            let candidates = (1..=4)
                .flat_map(|count| probes.iter().map(move |probe| vec![*probe; count]))
                .chain([vec![], vec!["n"; 9]])
                .map(|arguments| format!("{name}({})", arguments.join(", ")));
            let proven = candidates.into_iter().find(|call| {
                parse_spanned(call).is_ok_and(|expr| {
                    check_expression_calls(&expr, &site).first().is_some_and(CallCheck::is_rejected)
                })
            });
            let Some(call) = proven else {
                unproven.push(*name);
                continue;
            };
            hooks::reset_handler_entries();
            let content = format!("---\n$schema:\n  b: boolean(required)\n  n: number(required)\n  s: string(required)\nb: true\nn: 1\ns: x\n---\n{{{{ {call} }}}}\n");
            match compose_in(dir.path(), &content, false) {
                Ok(_) => failures.push(format!("{call}: composed")),
                Err(error) if call_check(&error).is_none() => failures.push(format!("{call}: {error}")),
                Err(_) => {}
            }
            if hooks::handler_entries(name) != 0 {
                failures.push(format!("{call}: the handler ran"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
        // `and` and `or` take any number of operands of any type, so no call
        // to them is wrong; every other function has one.
        assert_eq!(unproven, ["and", "or"], "no proven-bad call");
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "took {:?}", started.elapsed());
    }
}

mod branches {
    use super::*;

    /// Criterion 49: calls in branches that never run.
    #[test]
    fn a_dead_ternary_branch_is_checked() {
        rejected_before_evaluation(&document("", "{{ true ? 'ok' : min(flag, 5) }}"));
    }

    #[test]
    fn a_short_circuited_operand_is_checked() {
        rejected_before_evaluation(&document("", "{{ size || min(flag, 5) }}"));
    }

    #[test]
    fn the_body_of_a_false_block_is_checked() {
        rejected_before_evaluation(&document("", "::block when=\"false\"\n{{ min(flag, 5) }}\n::end-block"));
    }

    #[test]
    fn a_block_condition_is_checked() {
        let check = rejected_before_evaluation(&document("", "::block when=\"min(flag, 5) > 1\"\nx\n::end-block"));
        assert_eq!(check.function, "min");
    }

    #[test]
    fn a_transclusion_condition_is_checked() {
        let check = rejected_before_evaluation(&document("", "::file ./missing.md when=\"min(flag, 5) > 1\""));
        assert_eq!(check.function, "min");
    }

    /// Criterion 53: lazy operands are not entered, but are checked.
    #[test]
    fn lazy_operands_stay_lazy_and_are_still_checked() {
        let _effect = effect();
        hooks::reset_handler_entries();
        let rendered = composed(&format!("---\n{FLAG}---\n{{{{ and(false, effect()) }}}} {{{{ or(true, effect()) }}}}\n"));
        assert_eq!(rendered, "false true");
        assert_eq!(hooks::handler_entries("effect"), 0);
        rejected_before_evaluation(&document("", "{{ and(false, min(flag, 5)) }}"));
        rejected_before_evaluation(&document("", "{{ or(true, min(flag, 5)) }}"));
    }

    /// Rows f–i: narrowing and conservative typing do not reject.
    #[test]
    fn conservative_rows_are_not_rejected() {
        let schema = "$schema:\n  count: string\n  maybe: number\n  foo:\n    - string(required)\n    - number(required)\nfoo: 7\n";
        for (expression, expected) in [
            ("min(length(count), 5)", None),
            ("is_number(foo) ? min(foo, 5) : 0", Some("5")),
            ("is_string(foo) ? 0 : min(foo, 5)", Some("5")),
            ("min(maybe || 0, 5)", Some("0")),
        ] {
            match compose(&format!("---\n{schema}---\n{{{{ {expression} }}}}\n")) {
                Ok(composed) => {
                    if let Some(expected) = expected {
                        assert_eq!(composed.content().trim_end(), expected, "{expression}");
                    }
                }
                Err(error) => assert!(call_check(&error).is_none(), "{expression} was rejected: {error}"),
            }
        }
    }
}

mod conversions {
    use super::*;

    /// Criterion 38: number → text is category 1; the constraint is a
    /// runtime type error.
    #[test]
    fn a_category_1_conversion_composes_and_fails_by_value_at_runtime() {
        let registration =
            FunctionRegistration::builder("three_letter_acronym", "function(parameters(tla: string(min(3); max(3))); returns(string))")
                .build()
                .unwrap();
        let _tla = hooks::register(registration, Arc::new(|call| Ok(json!(call.args().text(0).unwrap()))));
        let document = |lookup: &str| {
            format!(
                "---\n$schema:\n  lookup: number(required)\nlookup: {lookup}\ndefinition: \"{{{{ three_letter_acronym(lookup) }}}}\"\n---\n{{{{ definition }}}}\n"
            )
        };
        assert_eq!(composed(&document("123")), "123");

        let error = compose(&document("1234")).unwrap_err();
        assert!(call_check(&error).is_none(), "rejected at design time: {error}");
        assert!(
            matches!(&error, MarkdownError::Interpolation { cause, .. } if matches!(cause.as_ref(), ExpressionError::Call(_))),
            "{error:?}"
        );
    }

    /// Criterion 51: numeric recovery agrees with the runtime binder.
    #[test]
    fn numeric_recovery_composes_and_an_unrecoverable_call_is_rejected() {
        for call in [
            "number(\"pear\", 7)",
            "number(true, 7)",
            "number(null, 7)",
            "round(\"pear\", 2.7)",
            "number(flag, 7)",
            "round(flag, 2.7)",
        ] {
            composed(&format!("---\n{FLAG}---\n{{{{ {call} }}}}\n"));
        }
        for call in ["number(\"4\", \"x\")", "number(\"pear\")", "number()", "number(flag, \"x\")"] {
            rejected_before_evaluation(&document("", &format!("{{{{ {call} }}}}")));
        }
    }
}

mod surface {
    use super::*;

    const BAD: &str = "{{ min(flag, 5) }}";

    /// Criterion 54: fenced examples, literals, literal tokens, and inserted
    /// data are not the document's expressions.
    #[test]
    fn non_executable_syntax_is_not_checked() {
        let token = darkmatter::markdown::literal_token::encode(BAD);
        let rendered = composed(&format!(
            "---\n{FLAG}example: \"{token}\"\nescaped: \"{{{{{{ min(flag, 5) }}}}}}\"\n---\n\
             ```md\n{BAD}\n```\n\n{{{{{{ min(flag, 5) }}}}}}\n\n{{{{ example }}}}|{{{{ escaped }}}}\n"
        ));
        assert_eq!(rendered.matches(BAD).count(), 4, "{rendered}");
    }

    #[test]
    fn frontmatter_only_composition_leaves_an_invalid_body_untouched() {
        let dir = TempDir::new().unwrap();
        let content = format!("---\n{FLAG}---\n{BAD}\n");
        let projected = compose_in(dir.path(), &content, true).expect("the frontmatter surface composes");
        assert!(projected.content().contains(BAD));

        let error = compose_in(dir.path(), &content, false).unwrap_err();
        assert!(call_check(&error).is_some(), "{error}");
    }

    #[test]
    fn frontmatter_only_composition_still_checks_frontmatter() {
        let dir = TempDir::new().unwrap();
        let error = compose_in(dir.path(), &format!("---\n{FLAG}bad: \"{BAD}\"\n---\nbody\n"), true).unwrap_err();
        assert!(call_check(&error).is_some(), "{error}");
    }
}

mod transclusion {
    use super::*;

    /// A child is checked against its own schema before its own evaluation.
    #[test]
    fn a_child_with_its_own_schema_making_a_bad_call_is_rejected() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "child.md", &format!("---\n{FLAG}---\n{{{{ min(flag, 5) }}}}\n"));
        let error = compose_in(dir.path(), "---\nn: 3\n---\n{{ min(n, 5) }}\n::file ./child.md\n", false).unwrap_err();
        let check = call_check(&error).unwrap_or_else(|| panic!("no call check: {error:?}"));
        assert!(error.to_string().contains("child.md") || format!("{error:?}").contains("child.md"), "{error}");
        assert_eq!(check.function, "min");
    }

    #[test]
    fn a_fine_parent_with_an_unschematized_child_composes() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "child.md", "child {{ min(n, 5) }}\n");
        let composed = compose_in(dir.path(), "---\nn: 3\n---\n{{ min(n, 5) }}\n\n::file ./child.md\n", false)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(composed.content().contains("child 3"), "{}", composed.content());
    }
}
