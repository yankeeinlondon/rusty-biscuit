//! Where a whole-YAML kind check reports a declaration that does not
//! resolve or parse: at the failing declaration itself, following union-arm
//! indices and the section (`$schema` or `types`) that wrote it, never at
//! the carrier's first node, a same-named declaration in the other section,
//! or a valid sibling of a multi-line payload.

use simplified_schema::yaml_documents::load_yaml_document;

/// The 1-based line and column of byte `offset` in `text`.
fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    (before.matches('\n').count() + 1, before[before.rfind('\n').map_or(0, |at| at + 1)..].chars().count() + 1)
}

/// The one problem of `text` checked as a whole YAML document: its pointer
/// and its position, from the problem's own coordinates when it has them,
/// else from the document's position map.
fn the_problem(text: &str) -> (String, Option<(usize, usize)>) {
    let tmp = tempfile::tempdir().unwrap();
    let problems = crate::envelope_null_carriers::whole_yaml_problems(text, tmp.path());
    let [problem] = problems.as_slice() else { panic!("one problem: {problems:#?}") };
    let at = match (problem.line, problem.column) {
        (Some(line), Some(column)) => Some((line as usize, column as usize)),
        _ => {
            let loaded = load_yaml_document(text, &tmp.path().join("subject.yaml")).unwrap();
            loaded.byte_range(problem).map(|range| line_column(text, range.start))
        }
    };
    (problem.path.clone(), at)
}

fn at(text: &str, needle: &str) -> Option<(usize, usize)> {
    Some(line_column(text, text.find(needle).unwrap_or_else(|| panic!("`{needle}` is in {text:?}"))))
}

#[test]
fn an_unresolved_type_in_a_namespace_union_arm_is_located_at_that_arm() {
    let text = "kind: namespace-schema\nnamespace: run\n$schema:\n    count:\n        - \"number -> The pass.\"\n        - \"Missing -> Or this.\"\n    label: string\n";
    assert_eq!(the_problem(text), ("/$schema/count/1".to_string(), at(text, "\"Missing")));
}

#[test]
fn an_unresolved_payload_type_is_located_in_the_payload_not_a_same_named_local_type() {
    let text = "kind: namespace-schema\nnamespace: run\ntypes:\n    count: \"number -> A count.\"\n$schema:\n    label: string\n    count: Missing\n";
    assert_eq!(the_problem(text), ("/$schema/count".to_string(), at(text, "Missing\n")));
}

#[test]
fn an_unresolved_type_in_a_yaml_kind_is_located_at_its_nested_declaration() {
    let text = "kind: yaml-schema\n$schema:\n    kind: literal(part; required)\n    name: string\n    parts:\n        size: number\n        steps: \"Missing[](required)\"\n";
    assert_eq!(the_problem(text), ("/$schema/parts/steps".to_string(), at(text, "\"Missing[]")));
}

#[test]
fn an_unresolved_type_in_a_library_union_arm_is_located_at_that_arm() {
    let text = "kind: schema\ntypes:\n    widget:\n        size: number\n    gadget:\n        - \"string -> A name.\"\n        - \"Missing[] -> Or its parts.\"\n";
    assert_eq!(the_problem(text), ("/types/gadget/1".to_string(), at(text, "\"Missing[]")));
}

/// A trigger file's payload is only parsed, by its definition's `schema`
/// carrier; a grammar failure in the second root-union arm is located at the
/// parser's rejected token inside that arm, not at the carrier, the first
/// arm, or the `$extends` before it.
#[test]
fn a_grammar_failure_in_a_trigger_root_union_arm_is_located_inside_that_arm() {
    let text = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    - $extends: widget@./library.yaml\n      gadget: string(required)\n    - $extends: widget@./library.yaml\n      gadget: string(requird)\n";
    assert_eq!(the_problem(text), ("/$schema/1/gadget".to_string(), at(text, "requird)")));
}

/// A YAML kind whose `kind` claim goes through a named type that claims no
/// finite set is reported at the `kind` declaration in `$schema`.
#[test]
fn a_named_kind_claim_that_claims_nothing_is_located_at_the_kind_declaration() {
    let text = "kind: yaml-schema\n$schema:\n    kind: kind-token(required)\n    name: string\ntypes:\n    kind-token: string\n";
    assert_eq!(the_problem(text), ("/$schema/kind".to_string(), at(text, "kind-token(required)")));
}
