//! Mixed-text operands (`"status {{ fm. }}"`) have every interpolation span
//! parsed with the stack, in both branches, without being evaluated.
//!
//! Each reader that turns an authored operand into an expression is one row:
//! the malformed span is placed in an unselected `then` and an unselected
//! `else`, and the parse must fail with the invalid-lifecycle-action error
//! naming the operand's authored path.

use super::*;
use serde_json::Value;
use crate::composition::lifecycle::actions::{expression_function_signature, side_effect_signature};

const BAD: &str = "status {{ fm. }}";

/// One reader site: a label, the action holding [`BAD`], and the operand path
/// below the action's list position (`info.text`, `set.meta.notes[0]`).
struct Row {
    label: String,
    action: Value,
    operand: String,
}

fn row(label: impl Into<String>, action: Value, operand: impl Into<String>) -> Row {
    Row {
        label: label.into(),
        action,
        operand: operand.into(),
    }
}

/// Every reader that converts an authored operand to an expression.
fn reader_rows() -> Vec<Row> {
    let mut rows = Vec::new();

    for verb in [
        "say", "speak", "effect", "message", "notify", "stderr", "stdout", "info", "warn", "success",
    ] {
        rows.push(row(format!("{verb} short"), json!({ verb: BAD }), format!("{verb}.text")));
        rows.push(row(
            format!("{verb} long"),
            json!({ verb: { "text": BAD } }),
            format!("{verb}.text"),
        ));
    }

    rows.extend([
        row("error short", json!({ "error": BAD }), "error.reason"),
        row("error.reason", json!({ "error": { "reason": BAD } }), "error.reason"),
        row("proxy short", json!({ "proxy": BAD }), "proxy.target"),
        row("proxy.target", json!({ "proxy": { "target": BAD } }), "proxy.target"),
        row("retry short", json!({ "retry": BAD }), "retry.max_attempts"),
        row("retry.max_attempts", json!({ "retry": { "max_attempts": BAD } }), "retry.max_attempts"),
        row("retry.delay", json!({ "retry": { "delay": BAD } }), "retry.delay"),
        row("resume short", json!({ "resume": BAD }), "resume.message"),
        row("resume.message", json!({ "resume": { "message": BAD } }), "resume.message"),
        row(
            "resume.max_attempts",
            json!({ "resume": { "message": "again", "max_attempts": BAD } }),
            "resume.max_attempts",
        ),
        row("defer short", json!({ "defer": BAD }), "defer.delay"),
        row("defer.delay", json!({ "defer": { "delay": BAD } }), "defer.delay"),
        row("defer.reason", json!({ "defer": { "delay": "1m", "reason": BAD } }), "defer.reason"),
        row("set scalar", json!({ "set": { "note": BAD } }), "set.note"),
        row("set nested", json!({ "set": { "meta": { "notes": [BAD] } } }), "set.meta.notes[0]"),
    ]);

    for verb in ["proxy", "retry"] {
        let target = if verb == "proxy" { json!("@next.md") } else { Value::Null };
        let long = |with: Value| {
            let mut params = serde_json::Map::new();
            if !target.is_null() {
                params.insert("target".into(), target.clone());
            }
            params.insert("with".into(), with);
            json!({ verb: params })
        };
        rows.push(row(format!("{verb}.with scalar"), long(json!({ "x": BAD })), format!("{verb}.with.x")));
        rows.push(row(
            format!("{verb}.with nested"),
            long(json!({ "meta": { "list": [BAD] } })),
            format!("{verb}.with.meta.list[0]"),
        ));
    }

    for verb in [
        "set_frontmatter",
        "merge_frontmatter",
        "delete_frontmatter",
        "increment_frontmatter",
        "decrement_frontmatter",
        "append_frontmatter",
        "prepend_frontmatter",
        "ensure_file",
        "ensure_dir",
        "append_line",
        "append_jsonl",
        "http_post",
    ] {
        let signature = side_effect_signature(verb).unwrap_or_else(|| panic!("`{verb}` signature"));
        let last = signature.min_count() - 1;
        let args: Vec<Value> = (0..=last)
            .map(|i| if i == last { json!(BAD) } else { json!("x.md") })
            .collect();
        rows.push(row(format!("{verb} positional"), json!({ verb: args }), format!("{verb}.arg[{last}]")));
    }
    rows.push(row(
        "ensure_file.content",
        json!({ "ensure_file": { "file": "x.md", "content": BAD } }),
        "ensure_file.content",
    ));

    let length = expression_function_signature("length").expect("`length` signature");
    let param = length.overloads()[0].names().next().expect("`length` takes a parameter");
    rows.push(row("length positional", json!({ "length": BAD }), "length.arg[0]"));
    rows.push(row(
        "length named",
        json!({ "length": { param: BAD } }),
        format!("length.{param}"),
    ));

    rows
}

/// A conditional whose guard never selects `branch`, holding `action` there.
fn unselected(branch: &str, action: Value) -> Value {
    let other = if branch == "then" { "else" } else { "then" };
    json!({ "when": "fm.pick == 'neither'", branch: [action], other: [{ "info": "selected" }] })
}

fn expect_rejection(result: Result<impl std::fmt::Debug, CompositionError>, path: &str, label: &str) {
    match result {
        Err(CompositionError::LifecycleActionInvalidLongForm { message, .. }) => {
            assert!(message.contains(&format!("`{path}`")), "{label}: path `{path}` not named in {message}");
            assert!(message.contains("fm."), "{label}: span not quoted in {message}");
        }
        other => panic!("{label}: expected LifecycleActionInvalidLongForm at `{path}`, got {other:?}"),
    }
}

#[test]
fn every_reader_rejects_a_malformed_span_in_either_unselected_branch() {
    for row in reader_rows() {
        for branch in ["then", "else"] {
            let frontmatter = json!({ "failure": [unselected(branch, row.action.clone())] });
            let path = format!("failure[0].{branch}[0].{}", row.operand);
            expect_rejection(
                parse_lifecycle_config(&frontmatter, dummy_path()),
                &path,
                &format!("{} in {branch}", row.label),
            );
        }
    }
}

#[test]
fn every_stack_home_rejects_a_malformed_span_in_an_unselected_branch() {
    let bad = json!({ "info": BAD });
    for event in ["initialize", "start", "blocked", "success", "failure", "finalize"] {
        for branch in ["then", "else"] {
            let frontmatter = json!({ event: [unselected(branch, bad.clone())] });
            expect_rejection(
                parse_lifecycle_config(&frontmatter, dummy_path()),
                &format!("{event}[0].{branch}[0].info.text"),
                event,
            );
        }
    }

    let gate = json!({ "loop": { "while": "false", "gate": [unselected("else", bad.clone())] } });
    expect_rejection(parse_lifecycle_config(&gate, dummy_path()), "loop.gate[0].else[0].info.text", "loop.gate");
    let loose = json!({ "loop": { "while": "false", "info": BAD } });
    expect_rejection(parse_lifecycle_config(&loose, dummy_path()), "loop.info.text", "loop loose verb");

    for (stage, signal) in [("setup", LifecycleSignal::Start), ("teardown", LifecycleSignal::Finalize)] {
        let property = format!("tasks[0].{stage}");
        let stack = json!([unselected("then", bad.clone())]);
        expect_rejection(
            parse_task_action_stack(signal, &stack, dummy_path(), &property),
            &format!("{property}[0].then[0].info.text"),
            stage,
        );
    }
}

#[test]
fn the_whole_value_and_mixed_forms_share_one_typed_cause() {
    for (text, whole) in [("{{ fm. }}", true), (BAD, false)] {
        let error = parse_lifecycle_config(&json!({ "start": [{ "info": text }] }), dummy_path())
            .expect_err(text);
        let CompositionError::LifecycleActionInvalidLongForm {
            property,
            source: Some(cause),
            ..
        } = &error
        else {
            panic!("{text}: expected a typed cause, got {error:?}");
        };
        assert_eq!(property, "start[0].info");
        assert_eq!(matches!(cause, ActionExprError::Parse(_)), whole, "{text}: {cause:?}");
        assert_eq!(matches!(cause, ActionExprError::SpanParse { .. }), !whole, "{text}: {cause:?}");
    }
}

#[test]
fn valid_mixed_text_and_span_free_literals_are_accepted_unchanged() {
    for text in [
        "status {{ fm.title }} at {{ ctx.today }}",
        "no spans: 50% of } and { braces",
        "an unclosed {{ opener stays literal",
        "an escaped \\{{ fm. }} span is text",
        "literal braces {{{ fm. }}} are text",
    ] {
        let config = parse_lifecycle_config(
            &json!({ "failure": [unselected("then", json!({ "info": text }))] }),
            dummy_path(),
        )
        .unwrap_or_else(|error| panic!("`{text}` must parse: {error}"));
        let LifecycleNode::Conditional(conditional) = &config.stack(LifecycleSignal::Failure)[0] else {
            panic!("expected a conditional");
        };
        let LifecycleNode::Action(action) = &conditional.then[0] else { panic!("expected an action") };
        let LifecycleActionKind::Communication(comm) = &action.kind else { panic!("expected info") };
        assert_eq!(comm.text, Expr::StringLiteral(text.to_string()), "the literal is kept as authored");
    }
}
