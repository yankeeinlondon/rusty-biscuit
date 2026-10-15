use super::*;
use serde_json::json;

#[test]
fn every_rename_row_is_typed_and_property_presence_preserves_data() {
    for (old, replacement) in RETIRED_SEQUENCE_NAMES.iter().copied().chain([
        ("state.id", "seq.id"), ("state.index", "seq.index"), ("state.count", "seq.count"),
        ("state.sequence_id", "seq.run_id"), ("state['sequence_id']", "seq.run_id"),
        ("state.is_first", "seq.is_first"), ("state.is_last", "seq.is_last"),
        ("previous['name']", "seq.previous.name"),
    ]) {
        let body = format!("before {{{{ {old} }}}} after");
        let check = |fm: &Value, supplied: Option<&Value>| reject_removed_sequence_reads(Path::new("prompt.md"),
            AuthoredDocument { frontmatter: fm, body: &body, body_line_offset: 3 }, supplied, None);
        let error = check(&json!({}), None).unwrap_err();
        assert!(matches!(error, CompositionError::RemovedGrammar { replacement: ref hint, ref path, .. }
            if hint == &format!("use `{replacement}`") && path == "body:4"), "{old}: {error}");
        let root = old.split(['.', '[']).next().unwrap();
        for value in [Value::Null, json!(false), json!(0), json!(""), json!([]), json!({})] {
            let fm = json!({root: value});
            assert!(check(&fm, None).is_ok(), "declared {root}");
            assert!(check(&json!({}), Some(&fm)).is_ok(), "supplied {root}");
        }
    }
}

#[test]
fn parsed_surfaces_include_conditions_operands_and_shell_values() {
    for frontmatter in [
        json!({"title": "{{ state.name }}"}),
        json!({"title": "$(echo {{ outputs[0] }})"}),
        json!({"loop": {"while": "step > 0"}}),
        json!({"start": [{"when": "state['index'] > 0", "then": [{"info": "ok"}]}]}),
        json!({"start": [{"shell": "echo {{ sequence_id }}"}]}),
        json!({"start": [{"set": {"value": "$(echo {{ previous }})"}}]}),
    ] {
        let config = crate::composition::lifecycle::parse_lifecycle_config(&frontmatter, Path::new("prompt.md")).unwrap();
        let error = reject_removed_sequence_reads(Path::new("prompt.md"), AuthoredDocument {
            frontmatter: &frontmatter, body: "", body_line_offset: 0,
        }, None, Some(&config)).unwrap_err();
        assert!(matches!(error, CompositionError::RemovedGrammar { .. }), "{error}");
    }
}

#[test]
fn explicit_frontmatter_unrelated_members_literals_and_code_are_not_renames() {
    for body in ["{{ fm.previous }}", "{{ fm['state'] }}", "{{ ctx.state }}", "{{ 'outputs' }}",
        "plain state previous outputs", "```md\n{{ state }}\n```", "{{ seq.previous }}"] {
        reject_removed_sequence_reads(Path::new("prompt.md"), AuthoredDocument {
            frontmatter: &json!({}), body, body_line_offset: 0,
        }, None, None).unwrap();
    }
}

#[test]
fn body_directive_conditions_are_checked_in_inactive_and_nested_branches() {
    for body in [
        "::file ./child.md when=\"false && previous\"\n",
        "::block when=\"false\"\n::block when=\"state['sequence_id']\"\nBody.\n::end-block\n::end-block\n",
    ] {
        let error = reject_removed_sequence_reads(Path::new("prompt.md"), AuthoredDocument {
            frontmatter: &json!({}), body, body_line_offset: 0,
        }, None, None).unwrap_err();
        assert!(matches!(error, CompositionError::RemovedGrammar { .. }), "{error}");
        reject_removed_sequence_reads(Path::new("prompt.md"), AuthoredDocument {
            frontmatter: &json!({"previous":null,"state":null}), body, body_line_offset: 0,
        }, None, None).unwrap();
    }
}
