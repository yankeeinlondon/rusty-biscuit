//! A read of a retired `_loop_*` name is removed grammar, found before launch
//! wherever the document's expressions are discoverable; text that only names
//! one stays data.

use super::*;
use crate::composition::prepare::prepare_direct;

/// Prepare `frontmatter` + `body` the way every composition run does.
fn prepare(
    frontmatter: &[(&str, Value)],
    body: &str,
) -> Result<crate::composition::PreparedComposition, CompositionError> {
    let source = make_source_with_body(frontmatter, body);
    prepare_direct(&source, PrepareOptions::new(crate::test_support::context()))
}

/// The `(path, removed, replacement)` of a removed-grammar failure.
fn removed(result: Result<crate::composition::PreparedComposition, CompositionError>) -> (String, String, String) {
    match result {
        Err(CompositionError::RemovedGrammar {
            path,
            removed,
            replacement,
            ..
        }) => (path, removed, replacement),
        Err(other) => panic!("expected RemovedGrammar, got {other:?}"),
        Ok(_) => panic!("expected RemovedGrammar, but the document prepared"),
    }
}

#[test]
fn lifecycle_reads_of_every_retired_name_are_rejected_with_path_and_replacement() {
    for (frontmatter, path, name, member) in [
        (
            ("start", json!([{"info": "pass {{ _loop_count }}"}])),
            "start[0].info.text",
            "_loop_count",
            "count",
        ),
        (
            ("success", json!([{"when": "_loop_is_first", "then": ["stop"]}])),
            "success[0].when",
            "_loop_is_first",
            "is_first",
        ),
        (
            ("failure", json!([{"when": "x", "else": [{"warn": "{{ fm._loop_is_last }}"}], "then": ["stop"]}])),
            "failure[0].else[0].warn.text",
            "_loop_is_last",
            "is_last",
        ),
        (
            ("finalize", json!([{"when": "fm['_loop_last_output'] != ''", "then": ["stop"]}])),
            "finalize[0].when",
            "_loop_last_output",
            "last_output",
        ),
        (
            ("loop", json!({"while": "true", "gate": [{"info": "code {{ _loop_last_exit_code }}"}]})),
            "loop.gate[0].info.text",
            "_loop_last_exit_code",
            "last_exit_code",
        ),
    ] {
        let (got_path, got_name, replacement) = removed(prepare(&[frontmatter], "Body"));
        assert_eq!(got_path, path);
        assert_eq!(got_name, name);
        assert_eq!(replacement, format!("use `loop.{member}`"));
    }
}

#[test]
fn loop_conditions_actions_and_other_frontmatter_are_checked() {
    for (frontmatter, path, name) in [
        (
            vec![("loop", json!({"while": "_loop_count < 3"}))],
            "loop.while",
            "_loop_count",
        ),
        (
            vec![("loop", json!({"until": "fm._loop_is_last"}))],
            "loop.until",
            "_loop_is_last",
        ),
        (
            vec![(
                "loop",
                json!({"while": "true", "action": ["set(stamp, '{{ _loop_count }}')"]}),
            )],
            "loop.action[0]",
            "_loop_count",
        ),
        (
            vec![("title", json!("Pass {{ _loop_count }}"))],
            "title",
            "_loop_count",
        ),
    ] {
        let (got_path, got_name, _) = removed(prepare(&frontmatter, "Body"));
        assert_eq!((got_path.as_str(), got_name.as_str()), (path, name));
    }
}

#[test]
fn a_body_span_read_names_its_file_line() {
    // `---`, the key, and `---` put the body's first line on line 4.
    let (path, name, replacement) = removed(prepare(
        &[("phase", json!(1))],
        "Phase {{ phase }}\n\nLast exit: {{ _loop_last_exit_code }}\n",
    ));
    assert_eq!(path, "body:6");
    assert_eq!(name, "_loop_last_exit_code");
    assert_eq!(replacement, "use `loop.last_exit_code`");

    let (path, ..) = removed(prepare(&[], "{{ fm['_loop_count'] }}"));
    assert!(path.starts_with("body:"), "{path}");
}

#[test]
fn literals_markdown_code_examples_and_unrelated_names_stay_data() {
    let body = "\
The `_loop_count` value was renamed; write {{ '_loop_count' }} literally.

```yaml
start:
    - info: \"{{ _loop_count }}\"
```

Notes: {{ _loop_notes }} and {{ fm._loop_notes }}.
";
    let prepared = prepare(
        &[
            ("_loop_notes", json!("kept")),
            ("start", json!([{"info": "_loop_count is now loop.count"}])),
            ("success", json!([{"when": "'_loop_count' != ''", "then": [{"info": "{{ '_loop_is_last' }}"}]}])),
            ("title", json!("About _loop_count")),
        ],
        body,
    )
    .unwrap_or_else(|error| panic!("data is never a retired read: {error}"));
    assert!(prepared.prompt.contains("Notes: kept and kept."), "{}", prepared.prompt);
    assert!(prepared.prompt.contains("write _loop_count literally"), "{}", prepared.prompt);
}
