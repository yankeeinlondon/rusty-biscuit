//! Owner-scoped graphs: every inline list is filled and typed by its own
//! container's declarations, an inline container's `$schema` is a check
//! prepared beside it, and a nested reread keeps the frozen rows.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::composition::sequence::resolve_sequence_plan;

fn write(dir: &Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.display().to_string()
}

fn build(path: &str) -> Result<(PreflightGraph, SequencePlan), CompositionError> {
    let source = crate::composition::resolve_fixture_source(path)?;
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&source, &resolution)?.expect("a sequence");
    let graph = build_preflight_graph(&plan, &source, &resolution)?;
    Ok((graph, plan))
}

fn container(task: Option<&PreflightTask>) -> &PreflightSequence {
    match &task.expect("a task").action {
        PreflightAction::Sequence(sequence) => sequence,
        other => panic!("expected a nested sequence, got {other:?}"),
    }
}

fn root_container(graph: &PreflightGraph, step: usize) -> &PreflightSequence {
    container(graph.steps[step].task.as_ref())
}

fn states(sequence: &PreflightSequence) -> Vec<Value> {
    sequence.plan.steps.iter().map(|step| step.state.to_value()).collect()
}

/// An inline container fills its rows from its own `template:`, reading the
/// enclosing document's values below each row's fields; the root's
/// `template:` fills only the root's rows, a container row included.
#[test]
fn each_inline_list_is_filled_by_its_own_template_from_the_enclosing_values() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "seq.md", r#"---
reviewer: ada
template:
  owner: root
sequence:
  - name: outer
    template:
      desc: "{{ reviewer }}:{{ name }}"
    sequence:
      - name: blue
        shell: echo blue
      - name: red
        desc: authored
        shell: echo red
---
Body.
"#);
    let (graph, plan) = build(&path).unwrap();
    assert_eq!(plan.steps[0].state.extra.get("owner"), Some(&json!("root")), "the root row is the root's");
    let outer = root_container(&graph, 0);
    let rows = states(outer);
    assert_eq!(rows[0]["desc"], json!("ada:blue"));
    assert_eq!(rows[1]["desc"], json!("authored"), "an authored field wins over a default");
    assert!(rows.iter().all(|row| row.get("owner").is_none()), "the root template stops at the container: {rows:?}");
}

/// A container's `$state` types its own rows, and the parent's `$state`
/// judges the container row itself but none of its children, inline, in a
/// group leg, and in a `kind: task` file; each failure names its file and
/// full container path.
#[test]
fn each_state_contract_judges_only_its_immediate_rows() {
    let dir = TempDir::new().unwrap();
    let parent = "$state:\n  color: string(required)\n";
    let child = "    $state:\n      rank: number(required)\n";

    // The parent's contract reaches the container row but not its children.
    let path = write(dir.path(), "a.md", &format!(
        "---\n{parent}sequence:\n  - name: outer\n    color: blue\n{child}    sequence:\n      - name: one\n        rank: 1\n        shell: echo one\n---\nBody.\n"
    ));
    let (graph, _) = build(&path).unwrap();
    assert_eq!(states(root_container(&graph, 0))[0]["rank"], json!(1));

    let path = write(dir.path(), "b.md", &format!(
        "---\n{parent}sequence:\n  - name: outer\n{child}    sequence:\n      - name: one\n        rank: 1\n        shell: echo one\n---\nBody.\n"
    ));
    assert!(matches!(
        build(&path).unwrap_err(),
        CompositionError::SequenceStateSchemaViolation { index: 0, ref container, .. } if container == "sequence"
    ), "the parent judges the container row's own state");

    let violation = |error: CompositionError| match error {
        CompositionError::SequenceStateSchemaViolation { source_path, container, property, .. } => (source_path, container, property),
        other => panic!("expected a state violation, got {other:?}"),
    };
    let path = write(dir.path(), "c.md", &format!(
        "---\nsequence:\n  - name: outer\n{child}    sequence:\n      - name: one\n        shell: echo one\n---\nBody.\n"
    ));
    let (file, at, property) = violation(build(&path).unwrap_err());
    assert_eq!((file.file_name().unwrap().to_str().unwrap(), at.as_str(), property.as_str()), ("c.md", "tasks[0].sequence", "rank"));

    let path = write(dir.path(), "d.md", "---\nsequence:\n  - name: fan\n    group:\n      tasks:\n        - name: leg\n          $state:\n            rank: number(required)\n          sequence:\n            - name: one\n              shell: echo one\n---\nBody.\n");
    let (_, at, _) = violation(build(&path).unwrap_err());
    assert_eq!(at, "tasks[0].group.tasks[0].sequence");

    write(dir.path(), "task.yaml", "kind: task\n$state:\n  rank: number(required)\nsequence:\n  - name: one\n    shell: echo one\n");
    let path = write(dir.path(), "e.md", "---\nsequence:\n  - name: external\n    task: task.yaml\n---\nBody.\n");
    let (file, at, _) = violation(build(&path).unwrap_err());
    assert_eq!((file.file_name().unwrap().to_str().unwrap(), at.as_str()), ("task.yaml", "sequence"));
}

/// An inline container's `$schema` is prepared beside its container and
/// located there; it is not a document schema, and an unloadable one is a
/// typed declaration error. A whole referenced document has no check: its
/// `$schema` belongs to its document scope.
#[test]
fn an_inline_check_is_prepared_beside_its_container_and_not_a_document_schema() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "seq.md", "---\nsequence:\n  - name: outer\n    $schema:\n      ready: boolean(required)\n    sequence:\n      - name: one\n        shell: echo one\n---\nBody.\n");
    let (graph, _) = build(&path).unwrap();
    let outer = root_container(&graph, 0);
    let check = outer.inline_schema.as_ref().expect("the check is prepared");
    assert_eq!(check.property(), "tasks[0].$schema", "a root row is located as the graph locates it");
    assert!(check.validate(&json!({})).is_some_and(|report| !report.problems.is_empty()));
    assert!(check.validate(&json!({"ready": true})).is_some_and(|report| report.problems.is_empty()));
    assert!(outer.document_scope().is_none(), "an inline list owns no document");

    write(dir.path(), "child.yaml", "kind: sequence\n$schema:\n  spec: string\nsequence:\n  - name: one\n    shell: echo one\n");
    let path = write(dir.path(), "ref.md", "---\nsequence:\n  - name: outer\n    sequence: child.yaml\n---\nBody.\n");
    let (graph, _) = build(&path).unwrap();
    let referenced = root_container(&graph, 0);
    assert!(referenced.inline_schema.is_none());
    assert!(referenced.document_scope().is_some_and(|scope| scope.declaring_path.ends_with("child.yaml")));

    let path = write(dir.path(), "bad.md", "---\nsequence:\n  - name: outer\n    $schema: ./missing-schema.yaml\n    sequence: []\n---\nBody.\n");
    assert!(matches!(
        build(&path).unwrap_err(),
        CompositionError::SequenceDeclarationLoad { ref property, .. } if property == "tasks[0].$schema"
    ));
}

/// Reread a nested container the way a container start does.
fn reread(graph: &PreflightGraph, frozen: &PreflightSequence) -> Result<PreflightSequence, CompositionError> {
    let task = graph.steps[0].task.as_ref().unwrap();
    let context = ComposeContext::capture_for_content(task.origin_dir.as_path(), "");
    let state = reapproval::early_binding_state(&SharedFrontmatter::default(), context.clone())?;
    let resolution = crate::test_support::context_for(&task.origin_path);
    prepare_nested_sequence(task, frozen, &state, Map::new(), context, &resolution, false, graph.nested_sources())
        .map(|(sequence, _)| sequence)
}

/// A reread keeps the frozen rows, ids, and template-filled state even when
/// the source's `template:` and `$state` changed, judges the reread
/// declarations' syntax, and takes the reread container check.
#[test]
fn a_reread_judges_declarations_but_keeps_the_frozen_rows() {
    let dir = TempDir::new().unwrap();
    let text = |template: &str, state: &str, check: &str| format!(
        "---\nsequence:\n  - name: outer\n    template:\n      tag: {template}\n    $state: {state}\n    $schema: {check}\n    sequence:\n      - name: one\n        shell: echo one\n---\nBody.\n"
    );
    let path = write(dir.path(), "seq.md", &text("v1", "{tag: string(required)}", "{a: string}"));
    let (graph, _) = build(&path).unwrap();
    let frozen = root_container(&graph, 0).clone();

    write(dir.path(), "seq.md", &text("v2", "{tag: number(required)}", "{b: string(required)}"));
    let fresh = reread(&graph, &frozen).unwrap();
    assert_eq!(states(&fresh), states(&frozen), "rows are neither refilled nor retyped");
    assert_eq!(states(&fresh)[0]["tag"], json!("v1"));
    let check = fresh.inline_schema.as_ref().unwrap();
    assert!(check.validate(&json!({})).is_some_and(|report| !report.problems.is_empty()), "the reread check applies");

    write(dir.path(), "seq.md", &text("v2", "42", "{b: string}"));
    assert!(matches!(
        reread(&graph, &frozen).unwrap_err(),
        CompositionError::SequenceDeclarationShape { ref property, .. } if property == "tasks[0].$state"
    ));
}

#[cfg(feature = "work-counters")]
mod counted {
    use super::*;
    use crate::work_counters::{Work, snapshot};

    /// One `$state` judges every row of its container, prepared once however
    /// many rows there are, and an unchanged container check is reused by a
    /// reread rather than prepared again.
    #[test]
    fn declarations_are_prepared_once_per_owner_revision() {
        let dir = TempDir::new().unwrap();
        let rows: String = (0..12).map(|index| format!("      - name: row{index}\n        rank: {index}\n        shell: echo {index}\n")).collect();
        let path = write(dir.path(), "seq.md", &format!(
            "---\nsequence:\n  - name: outer\n    $state:\n      rank: number(required)\n    $schema:\n      ready: boolean\n    sequence:\n{rows}---\nBody.\n"
        ));
        let before = snapshot();
        let (graph, _) = build(&path).unwrap();
        assert_eq!(snapshot().since(&before).get(Work::SequenceSchemaPrepare), 2, "one `$state` and one check for 12 rows");

        let frozen = root_container(&graph, 0).clone();
        let before = snapshot();
        for _ in 0..3 {
            reread(&graph, &frozen).unwrap();
        }
        assert_eq!(snapshot().since(&before).get(Work::SequenceSchemaPrepare), 0, "an unchanged check is reused");
    }

    /// A referenced document's `$state` is one contract for all of its rows,
    /// and so is a root's; an unchanged reread prepares nothing, parses
    /// nothing, and neither refills the template nor regenerates a row, even
    /// after the document's template changed on disk.
    #[test]
    fn a_referenced_owner_resolves_one_contract_and_a_reread_regenerates_nothing() {
        let dir = TempDir::new().unwrap();
        let rows: String = (0..20).map(|index| format!("  - name: row{index}\n    rank: {index}\n    shell: echo {index}\n")).collect();
        let child = |tag: &str| format!("kind: sequence\n$state:\n  rank: number(required)\ntemplate:\n  tag: {tag}\nsequence:\n{rows}");
        write(dir.path(), "child.yaml", &child("v1"));
        let path = write(dir.path(), "seq.md", &format!(
            "---\n$state:\n  rank: number(required)\nsequence:\n  - name: nested\n    rank: 0\n    sequence: child.yaml\n{}---\nBody.\n",
            (1..20).map(|index| format!("  - name: root{index}\n    rank: {index}\n    shell: echo root{index}\n")).collect::<String>(),
        ));
        let before = snapshot();
        let source = crate::composition::resolve_fixture_source(&path).unwrap();
        let resolution = crate::test_support::context_for(&source.resolved_path);
        let plan = resolve_sequence_plan(&source, &resolution).unwrap().unwrap();
        assert_eq!(snapshot().since(&before).get(Work::SequenceSchemaPrepare), 1, "the root's 20 rows share one contract");
        let before = snapshot();
        let graph = build_preflight_graph(&plan, &source, &resolution).unwrap();
        assert_eq!(snapshot().since(&before).get(Work::SequenceSchemaPrepare), 1, "the child's 20 rows share one contract");

        let frozen = root_container(&graph, 0).clone();
        assert_eq!(states(&frozen).len(), 20);
        let before = snapshot();
        for _ in 0..3 {
            assert_eq!(states(&reread(&graph, &frozen).unwrap()), states(&frozen));
        }
        let work = snapshot().since(&before);
        assert_eq!(work.get(Work::SequenceSchemaPrepare), 0, "an unchanged contract is reused");
        assert_eq!(work.get(Work::ReferencedDocumentParse), 0, "an unchanged document is not parsed again");

        write(dir.path(), "child.yaml", &child("v2"));
        let fresh = reread(&graph, &frozen).unwrap();
        assert!(states(&fresh).iter().all(|row| row["tag"] == json!("v1")), "rows are never refilled");
    }

    /// A check whose referenced schema file changed is prepared again, and
    /// the reread container carries the new contract.
    #[test]
    fn a_changed_schema_file_invalidates_the_prepared_check() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "check.schema.yaml", "kind: schema\n$schema:\n  ready: string\n");
        let path = write(dir.path(), "seq.md", "---\nsequence:\n  - name: outer\n    $schema: ./check.schema.yaml\n    sequence:\n      - name: one\n        shell: echo one\n---\nBody.\n");
        let (graph, _) = build(&path).unwrap();
        let frozen = root_container(&graph, 0).clone();
        assert!(frozen.inline_schema.as_ref().unwrap().validate(&json!({})).is_some_and(|report| report.problems.is_empty()));

        write(dir.path(), "check.schema.yaml", "kind: schema\n$schema:\n  ready: string(required)\n");
        let before = snapshot();
        let fresh = reread(&graph, &frozen).unwrap();
        assert_eq!(snapshot().since(&before).get(Work::SequenceSchemaPrepare), 1);
        assert!(fresh.inline_schema.as_ref().unwrap().validate(&json!({})).is_some_and(|report| !report.problems.is_empty()));
    }
}

/// A whole referenced document fills its template from its own prepared
/// values: a caller's `file(eager)` setter is converted by the document's
/// `$schema` before the template reads it.
#[test]
fn a_referenced_document_fills_its_rows_from_its_own_prepared_values() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("nested/work")).unwrap();
    write(dir.path(), "nested/work/spec.md", "# spec\n");
    write(dir.path(), "nested/child.yaml", "kind: sequence\n$schema:\n  spec: file(eager)\ntemplate:\n  where: \"{{ spec }}\"\nsequence:\n  - name: one\n    shell: echo one\n");
    let path = write(dir.path(), "seq.md", "---\nsequence:\n  - name: outer\n    sequence: nested/child.yaml\n---\nBody.\n");
    let source = crate::composition::resolve_fixture_source(&path).unwrap();
    let plan = resolve_sequence_plan(&source, &crate::test_support::context_for(&source.resolved_path)).unwrap().unwrap();
    let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir.path()).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let context = ComposeContext::capture_for_content(dir.path(), "");
    let graph = build_preflight_graph_with_invocation(
        &plan, &source, context, &invocation, &source_context, Some(&json!({"spec": "nested/work/spec.md"})),
    ).unwrap();
    let filled = states(root_container(&graph, 0))[0]["where"].as_str().unwrap().to_string();
    let filled = Path::new(&filled);
    assert!(filled.is_absolute() && filled.ends_with("nested/work/spec.md"), "{}", filled.display());
}

/// An inline container's `$state` and `$schema` resolve references from
/// their declaring file, not the caller's directory, and an unloadable one
/// fails the graph at that container with the schema layer's cause. The
/// declaring file, the caller, and the schema files are in three folders.
#[test]
fn inline_declarations_resolve_from_their_declaring_file() {
    let dir = TempDir::new().unwrap();
    for folder in ["caller/deep", "decl", "schemas"] {
        fs::create_dir_all(dir.path().join(folder)).unwrap();
    }
    write(&dir.path().join("schemas"), "types.yaml", "kind: schema\ntypes:\n  shade: \"enum(red, blue)\"\n");
    write(&dir.path().join("schemas"), "check.schema.yaml", "kind: schema\n$schema:\n  ready: boolean(required)\n");
    let caller = dir.path().join("caller/deep");
    let graph_of = |state: &str, check: &str| {
        let path = write(&dir.path().join("decl"), "seq.md", &format!(
            "---\nsequence:\n  - name: outer\n    $state: {state}\n    $schema: {check}\n    sequence:\n      - name: one\n        color: red\n        shell: echo one\n---\nBody.\n"
        ));
        let source = crate::composition::resolve_fixture_source(&path)?;
        let resolution = crate::test_support::context_at(&caller);
        let plan = resolve_sequence_plan(&source, &resolution)?.expect("a sequence");
        build_preflight_graph(&plan, &source, &resolution)
    };

    let graph = graph_of("{ color: shade(required)@../schemas/types.yaml }", "../schemas/check.schema.yaml").unwrap();
    let check = root_container(&graph, 0).inline_schema.as_ref().expect("prepared");
    assert!(check.validate(&json!({})).is_some_and(|report| !report.problems.is_empty()), "the check file's contract applies");
    assert!(graph_of("{ color: shade(required)@../schemas/types.yaml }", "{}").is_ok(), "an open check");

    for (state, check, property, named) in [
        ("{ color: tint@../schemas/types.yaml }", "{}", "tasks[0].$state", "tint"),
        ("{}", "./check.schema.yaml", "tasks[0].$schema", "check.schema.yaml"),
    ] {
        let error = graph_of(state, check).unwrap_err();
        let CompositionError::SequenceDeclarationLoad { property: found, source_path, source } = &error else {
            panic!("{state} / {check}: expected SequenceDeclarationLoad, got {error:?}");
        };
        assert_eq!(found, property);
        assert!(source_path.ends_with("decl/seq.md"), "{}", source_path.display());
        assert!(source.to_string().contains(named), "{source}");
    }

    // The container's `$state` judges its row with the imported type.
    let path = write(&dir.path().join("decl"), "bad.md", "---\nsequence:\n  - name: outer\n    $state: { color: shade(required)@../schemas/types.yaml }\n    sequence:\n      - name: one\n        color: green\n        shell: echo one\n---\nBody.\n");
    let source = crate::composition::resolve_fixture_source(&path).unwrap();
    let resolution = crate::test_support::context_at(&caller);
    let plan = resolve_sequence_plan(&source, &resolution).unwrap().unwrap();
    assert!(matches!(
        build_preflight_graph(&plan, &source, &resolution).unwrap_err(),
        CompositionError::SequenceStateSchemaViolation { ref property, ref source_path, .. }
            if property == "/color" && source_path.ends_with("decl/bad.md")
    ));
}

/// A parent's `$state` and `template` stop at a referenced child: they judge
/// and fill the container row, never the child document's rows.
#[test]
fn a_parents_declarations_stop_at_a_referenced_child() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.yaml", "kind: sequence\nsequence:\n  - name: one\n    shell: echo one\n");
    let parent = |row: &str| write(dir.path(), "seq.md", &format!(
        "---\n$state:\n  color: string(required)\ntemplate:\n  tone: root\nsequence:\n  - name: nested\n{row}    sequence: child.yaml\n---\nBody.\n"
    ));
    let (graph, plan) = build(&parent("    color: blue\n")).unwrap();
    assert_eq!(plan.steps[0].state.extra.get("tone"), Some(&json!("root")), "the container row is the parent's");
    let rows = states(root_container(&graph, 0));
    assert!(rows[0].get("tone").is_none() && rows[0].get("color").is_none(), "{rows:?}");

    assert!(matches!(
        build(&parent("")).unwrap_err(),
        CompositionError::SequenceStateSchemaViolation { index: 0, ref property, .. } if property == "color"
    ));
}
