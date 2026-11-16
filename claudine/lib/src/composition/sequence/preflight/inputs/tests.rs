//! Input discovery observed through [`discover_sequence_inputs`] over real
//! files: which documents own inputs, what each one is missing, and what the
//! walk leaves alone.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, text).unwrap();
    biscuit_file::canonicalize_simplified(&path).unwrap()
}

fn load(path: &Path) -> ResolvedCompositionSource {
    if path.extension().is_some_and(|extension| extension == "md") {
        crate::composition::resolve_fixture_source(path.to_str().unwrap()).unwrap()
    } else {
        ResolvedCompositionSource {
            original_ref: path.display().to_string(),
            resolved_path: path.to_path_buf(),
            original_text: fs::read_to_string(path).unwrap(),
            markdown: crate::composition::resolve::load_yaml_document(path).unwrap(),
        }
    }
}

/// Discover `path`'s inputs with `setters` supplied from `caller_dir`.
fn discover(path: &Path, caller_dir: &Path, setters: Value) -> Result<SequenceInputInventory, CompositionError> {
    let resolution = crate::test_support::context_at(caller_dir);
    let layers = CallerInputLayers::from_caller_overrides(Some(setters), resolution.clone());
    let context = ComposeContext::capture_for_content(caller_dir, "");
    discover_sequence_inputs(&load(path), InputDiscoveryOptions::new(&resolution, &context).with_caller_layers(&layers))
}

fn missing_names(owner: &InputOwner) -> Vec<String> {
    owner.missing.iter().flat_map(|entry| entry.missing.iter().map(|property| property.name.clone())).collect()
}

/// A root, a nested whole document with no steps, a group leg's whole
/// document, and a whole document inside a `kind: task` file, plus an inline
/// container whose `$schema` and `$state` are not inputs.
fn owners_fixture(dir: &Path, root_name: &str, root_text: &str) -> PathBuf {
    write(dir, "p.md", "---\ntitle: p\n---\nP.\n");
    write(dir, "nested/child.yaml", "kind: sequence\n$schema:\n  spec: string(required) -> the spec\nsequence: []\n");
    write(dir, "legs/leg.yaml", "kind: sequence\n$schema:\n  leg_input: string(required)\nsequence:\n  - name: ask\n    prompt: ./q.md\n");
    write(dir, "legs/q.md", "---\ntitle: q\n---\nQ.\n");
    write(dir, "tasks/task.yaml", "kind: task\nsequence: inner.yaml\n");
    write(dir, "tasks/inner.yaml", "kind: sequence\nsequence:\n  - name: one\n    shell: echo one\n");
    write(dir, root_name, root_text)
}

const ROOT_YAML: &str = r#"kind: sequence
$schema:
  reviewer: string(required) -> who signs off
sequence:
  - name: plan
    prompt: ./p.md
  - name: nested
    sequence: nested/child.yaml
  - name: fan
    group:
      tasks:
        - name: leg
          sequence: legs/leg.yaml
  - name: external
    task: tasks/task.yaml
  - name: checked
    $schema:
      inline_only: string(required)
    $state:
      color: string(required)
    sequence:
      - name: blue
        color: blue
        shell: echo blue
"#;

#[test]
fn every_owning_document_is_inventoried_with_its_file_and_container() {
    let dir = TempDir::new().unwrap();
    for (name, text) in [("root.yaml", ROOT_YAML.to_string()), ("root.md", format!("---\n{ROOT_YAML}---\nBody.\n"))] {
        let root = owners_fixture(dir.path(), name, &text);
        let inventory = discover(&root, dir.path(), json!({})).unwrap();

        let owners: Vec<(InputOwnerRole, &Path, Option<&Path>, &str)> = inventory.owners.iter()
            .map(|owner| (owner.role, owner.path.as_path(), owner.referenced_from.as_deref(), owner.property.as_str()))
            .collect();
        let canonical = |relative: &str| biscuit_file::canonicalize_simplified(&dir.path().join(relative)).unwrap();
        let (child, leg, task, inner) = (canonical("nested/child.yaml"), canonical("legs/leg.yaml"), canonical("tasks/task.yaml"), canonical("tasks/inner.yaml"));
        assert_eq!(owners, [
            (InputOwnerRole::Root, root.as_path(), None, ""),
            (InputOwnerRole::Nested, child.as_path(), Some(root.as_path()), "sequence[1].sequence"),
            (InputOwnerRole::Nested, leg.as_path(), Some(root.as_path()), "sequence[2].group.tasks[0].sequence"),
            (InputOwnerRole::Nested, inner.as_path(), Some(task.as_path()), "sequence"),
        ], "{name}");

        // Each owner reports only its own inputs; the inline container's
        // `$schema` and `$state` requirements are not inputs.
        let missing: Vec<Vec<String>> = inventory.owners.iter().map(missing_names).collect();
        assert_eq!(missing, [vec!["reviewer"], vec!["spec"], vec!["leg_input"], vec![]], "{name}");
        let Some(CompositionError::SequenceMissingProperties { failures, failure_count }) = inventory.missing_error() else {
            panic!("{name}: expected the combined report");
        };
        assert_eq!(failure_count, 3);
        let subjects: Vec<_> = failures.iter().map(|entry| (&entry.subject, entry.source_path.clone())).collect();
        assert_eq!(subjects, [
            (&SequenceMissingSubject::Document { property: String::new() }, root.clone()),
            (&SequenceMissingSubject::Document { property: "sequence[1].sequence".into() }, child.clone()),
            (&SequenceMissingSubject::Document { property: "sequence[2].group.tasks[0].sequence".into() }, leg.clone()),
        ]);

        assert_eq!(inventory.inline_containers, [InlineContainer {
            source_path: root.clone(), property: "sequence[4]".into(), owner: 0, checks: true,
        }]);
        let prompts: Vec<(PathBuf, usize)> = inventory.prompts.iter().map(|prompt| (prompt.path.clone(), prompt.owner)).collect();
        assert_eq!(prompts, [(canonical("p.md"), 0), (canonical("legs/q.md"), 2)], "a prompt belongs to its nearest owner");
        assert!(inventory.pending.is_empty());
    }
}

/// Independent owners are judged independently in one pass: supplying one
/// owner's input clears only that owner, and supplying all of them clears
/// the report (next test).
#[test]
fn a_supplied_input_clears_only_the_owner_that_declares_it() {
    let dir = TempDir::new().unwrap();
    let root = owners_fixture(dir.path(), "root.yaml", ROOT_YAML);
    let partial = discover(&root, dir.path(), json!({"spec": "s.md"})).unwrap();
    let missing: Vec<Vec<String>> = partial.owners.iter().map(missing_names).collect();
    assert_eq!(missing, [vec!["reviewer"], vec![], vec!["leg_input"], vec![]]);
    assert_eq!(partial.owners[1].values["spec"], json!("s.md"), "the setter reaches the referenced owner's values");
}

#[test]
fn supplying_every_owners_inputs_clears_the_report() {
    let dir = TempDir::new().unwrap();
    let root = owners_fixture(dir.path(), "root.yaml", ROOT_YAML);
    let complete = discover(&root, dir.path(), json!({"spec": "s.md", "reviewer": "ada", "leg_input": "x"})).unwrap();
    assert!(complete.missing_error().is_none(), "{:?}", complete.missing_entries());
}

/// The spec's reproduction: a reference filled from a `file(eager)` input
/// stays pending while that input is missing, so the report names the input
/// rather than a file it could not find; supplied from another directory, it
/// is converted before the template reads it, and the reference resolves.
#[test]
fn a_reference_reading_a_missing_input_is_pending_until_the_input_is_supplied() {
    let dir = TempDir::new().unwrap();
    let docs = dir.path().join("docs");
    let root = write(&docs, "complete.yaml", r#"kind: sequence
$schema:
  spec: file(required;eager) -> the spec
plan: "{{ dirname(spec) + '/plan.md' }}"
template:
  prompt: "{{ dirname(spec) + '/step.md' }}"
sequence:
  - name: templated
  - name: nested
    prompt: "{{ plan }}"
  - name: literal
    prompt: ./literal.md
"#);
    write(&docs, "literal.md", "---\ntitle: literal\n---\nL.\n");

    let pending = discover(&root, dir.path(), json!({})).unwrap();
    assert_eq!(missing_names(&pending.owners[0]), ["spec"]);
    let properties: Vec<&str> = pending.pending.iter().map(|reference| reference.property.as_str()).collect();
    assert_eq!(properties, ["sequence[0].prompt", "sequence[1].prompt"]);
    assert_eq!(pending.prompts.len(), 1, "a literal reference is still resolved: {:?}", pending.prompts);

    let caller = dir.path().join("caller");
    write(&caller, "work/spec.md", "# spec\n");
    write(&caller, "work/step.md", "---\ntitle: step\n---\nS.\n");
    let resolved = discover(&root, &caller, json!({"spec": "work/spec.md"}));
    // Supplied, nothing is pending: the template default resolves, and an
    // authored reference is read as the literal file name it always was.
    assert!(matches!(
        resolved,
        Err(CompositionError::SequenceExternalLoad { ref context, .. }) if context.contains("{{ plan }}")
    ), "{resolved:?}");
    fs::write(&root, fs::read_to_string(&root).unwrap().replace("  - name: nested\n    prompt: \"{{ plan }}\"\n", "")).unwrap();
    let resolved = discover(&root, &caller, json!({"spec": "work/spec.md"})).unwrap();
    assert!(resolved.pending.is_empty() && resolved.missing_error().is_none());
    let names: Vec<&str> = resolved.prompts.iter().filter_map(|prompt| prompt.path.file_name()?.to_str()).collect();
    assert_eq!(names, ["step.md", "literal.md"]);
}

/// A root that references a whole document is judged by that document's
/// inputs as well as its own, in any whole-document format; a selection out
/// of a data file owns nothing.
#[test]
fn a_root_reference_is_an_owner_and_a_selection_is_not() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "steps.json5", "{ kind: 'sequence', $schema: { spec: 'string(required)' }, sequence: [ 'one', ], }\n");
    write(dir.path(), "data.yaml", "rows:\n  - name: one\n");
    for (target, owners) in [("steps.json5", 2), ("data.yaml -> rows", 1)] {
        let root = write(dir.path(), "root.yaml", &format!("kind: sequence\nsequence: {target}\n"));
        let inventory = discover(&root, dir.path(), json!({})).unwrap();
        assert_eq!(inventory.owners.len(), owners, "{target}");
        if owners == 2 {
            let owner = &inventory.owners[1];
            assert_eq!((owner.role, owner.property.as_str()), (InputOwnerRole::RootReference, "sequence"));
            assert_eq!(missing_names(owner), ["spec"]);
        }
    }
}

/// A malformed declaration fails discovery even behind a false guard, and an
/// owner's invalid supplied value is its typed schema error, not a missing
/// input.
#[test]
fn malformed_declarations_and_invalid_values_fail_discovery() {
    let dir = TempDir::new().unwrap();
    let guarded = write(dir.path(), "guarded.yaml", "kind: sequence\nsequence:\n  - name: off\n    when: \"false\"\n    $state: 42\n    sequence: []\n");
    assert!(matches!(
        discover(&guarded, dir.path(), json!({})),
        Err(CompositionError::SequenceDeclarationShape { ref property, .. }) if property == "sequence[0].$state"
    ));

    write(dir.path(), "typed.yaml", "kind: sequence\n$schema:\n  count: number(required)\nsequence: []\n");
    let root = write(dir.path(), "root.yaml", "kind: sequence\nsequence:\n  - name: typed\n    sequence: typed.yaml\n");
    assert!(matches!(
        discover(&root, dir.path(), json!({"count": "many"})),
        Err(CompositionError::SchemaValidation { .. })
    ));

    let cycle = write(dir.path(), "cycle.yaml", "kind: sequence\nsequence:\n  - name: again\n    sequence: cycle.yaml\n");
    assert!(matches!(discover(&cycle, dir.path(), json!({})), Err(CompositionError::SequenceReferenceCycle { .. })));
}

/// Discovery runs no lifecycle action, frontmatter shell, or prompt-document
/// shell: it only reads.
#[test]
fn discovery_runs_nothing() {
    let dir = TempDir::new().unwrap();
    let marker = |name: &str| biscuit_file::to_portable_string(&dir.path().join(name));
    write(dir.path(), "prompt.md", &format!("---\nprobe: \"$(touch {})\"\n---\nP.\n", marker("prompt-shell")));
    write(dir.path(), "child.yaml", &format!(
        "kind: sequence\nprobe: \"$(touch {})\"\ninitialize:\n  - ensure_file: {}\nsequence:\n  - name: ask\n    prompt: ./prompt.md\n",
        marker("child-shell"), marker("child-initialize"),
    ));
    let root = write(dir.path(), "root.yaml", &format!(
        "kind: sequence\nprobe: \"$(touch {})\"\ninitialize:\n  - ensure_file: {}\nsequence:\n  - name: nested\n    sequence: child.yaml\n",
        marker("root-shell"), marker("root-initialize"),
    ));
    let inventory = discover(&root, dir.path(), json!({})).unwrap();
    assert_eq!((inventory.owners.len(), inventory.prompts.len()), (2, 1));
    for name in ["prompt-shell", "child-shell", "child-initialize", "root-shell", "root-initialize"] {
        assert!(!dir.path().join(name).exists(), "{name} ran");
    }
}

#[cfg(feature = "work-counters")]
mod counted {
    use super::*;
    use crate::work_counters::{Work, snapshot};

    /// The graph walk that follows discovery reuses discovery's parse of a
    /// referenced document whose bytes are unchanged.
    #[test]
    fn the_graph_walk_reuses_the_parses_discovery_made() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "child.yaml", "kind: sequence\nsequence:\n  - name: one\n    shell: echo one\n");
        let root = write(dir.path(), "root.md", "---\nsequence:\n  - name: nested\n    sequence: child.yaml\n---\nBody.\n");
        let source = load(&root);
        let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir.path()).unwrap();
        let source_context = invocation.derive_source(&source.resolved_path).unwrap();
        let context = ComposeContext::capture_for_content(dir.path(), "");
        let resolution = source_context.file_resolution_context().clone();

        let before = snapshot();
        let inventory = discover_sequence_inputs(
            &source, InputDiscoveryOptions::new(&resolution, &context).with_invocation(&invocation, &source_context),
        ).unwrap();
        let plan = crate::composition::sequence::resolve_sequence_plan(&source, &resolution).unwrap().unwrap();
        let graph = crate::composition::build_preflight_graph_after_discovery(
            &plan, &source, context.clone(), &invocation, &source_context, None, &inventory,
        ).unwrap();
        assert_eq!(graph.steps.len(), 1);
        assert_eq!(inventory.owners.len(), 2);
        assert_eq!(snapshot().since(&before).get(Work::ReferencedDocumentParse), 1, "child.yaml is parsed once for both walks");
    }
}

/// A referenced document's `template:` reads its referrer's values below its
/// own, as the plan does, so a shared roster parameterized by its referrer
/// resolves its rows in discovery too.
#[test]
fn a_referenced_template_reads_the_referrers_values_below_its_own() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "fleet/research.md", "---\ntitle: r\n---\nResearch.\n");
    write(dir.path(), "fleet/own.md", "---\ntitle: o\n---\nOwn.\n");
    write(dir.path(), "roster.yaml", "kind: sequence\ntemplate:\n  prompt: \"{{ research_prompt }}\"\nsequence:\n  - name: one\n");
    write(dir.path(), "own.yaml", "kind: sequence\nresearch_prompt: ./own.md\ntemplate:\n  prompt: \"{{ research_prompt }}\"\nsequence:\n  - name: one\n");
    for (roster, expected) in [("../roster.yaml", "research.md"), ("../own.yaml", "own.md")] {
        let root = write(
            dir.path(),
            "fleet/_fleet.md",
            &format!("---\nsequence: \"{roster}\"\nresearch_prompt: ./research.md\n---\n"),
        );
        let inventory = discover(&root, dir.path(), json!({})).unwrap();
        let prompts: Vec<_> = inventory.prompts.iter().map(|prompt| prompt.path.file_name().unwrap().to_owned()).collect();
        assert_eq!(prompts, vec![std::ffi::OsString::from(expected)], "{roster}");
    }
}

/// Under `leaving_verdicts`, an owner whose own preparation fails for a
/// reason other than a missing input is recorded instead of failing
/// discovery; without it, the same discovery fails.
#[test]
fn leaving_verdicts_records_a_failed_owner_instead_of_failing() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "typed.yaml", "kind: sequence\n$schema:\n  count: number(required)\nsequence: []\n");
    let root = write(dir.path(), "root.yaml", "kind: sequence\nsequence:\n  - name: typed\n    sequence: typed.yaml\n");
    let resolution = crate::test_support::context_at(dir.path());
    let layers = CallerInputLayers::from_caller_overrides(Some(json!({"count": "many"})), resolution.clone());
    let context = ComposeContext::capture_for_content(dir.path(), "");
    let options = InputDiscoveryOptions::new(&resolution, &context).with_caller_layers(&layers);

    let inventory = discover_sequence_inputs(&load(&root), options.leaving_verdicts()).unwrap();
    assert_eq!(inventory.failed_owners, vec![1]);
    assert_eq!(inventory.owners[1].role, InputOwnerRole::Nested);
    assert!(inventory.owners[1].missing.is_none());
    assert!(inventory.missing_error().is_none());
    assert!(matches!(discover_sequence_inputs(&load(&root), options), Err(CompositionError::SchemaValidation { .. })));
}

/// Discover `path`'s inputs as the CLI does: through an invocation launched
/// in `caller_dir`, with `setters` supplied from there.
fn discover_invoked(path: &Path, caller_dir: &Path, setters: Value) -> Result<SequenceInputInventory, CompositionError> {
    let source = load(path);
    let invocation = crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), caller_dir).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let resolution = crate::test_support::context_at(caller_dir);
    let layers = CallerInputLayers::from_caller_overrides(Some(setters), resolution.clone());
    let context = ComposeContext::capture_for_content(caller_dir, "");
    let options = InputDiscoveryOptions::new(&resolution, &context)
        .with_caller_layers(&layers)
        .with_invocation(&invocation, &source_context);
    discover_sequence_inputs(&source, options)
}

/// A referenced owner's authored `file(eager)` value resolves from its own
/// document's folder, never the caller's, and keeps its authored text; a
/// setter for the same input is converted from the caller's folder.
#[test]
fn a_referenced_owners_authored_file_value_keeps_its_text_and_a_setter_its_origin() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "nested/child.yaml", "kind: sequence\n$schema:\n  spec: file(required;eager)\nspec: work/spec.md\nsequence: []\n");
    let root = write(dir.path(), "root/root.yaml", "kind: sequence\nsequence:\n  - name: nested\n    sequence: ../nested/child.yaml\n");
    let caller = dir.path().join("caller");
    write(dir.path(), "nested/work/spec.md", "# beside the declaring document\n");

    let inventory = discover_invoked(&root, &caller, json!({})).unwrap();
    assert!(inventory.missing_error().is_none());
    assert_eq!(inventory.owners[1].values["spec"], json!("work/spec.md"), "the authored text is kept");

    write(&caller, "elsewhere/spec.md", "# supplied by the caller\n");
    let inventory = discover_invoked(&root, &caller, json!({ "spec": "elsewhere/spec.md" })).unwrap();
    let supplied = Path::new(inventory.owners[1].values["spec"].as_str().unwrap()).to_path_buf();
    assert_eq!(
        biscuit_file::canonicalize_simplified(&supplied).unwrap(),
        biscuit_file::canonicalize_simplified(&caller.join("elsewhere/spec.md")).unwrap(),
        "a setter resolves from the caller"
    );
}

/// The negative control: an authored `file(eager)` value whose file exists
/// only beside the caller is refused at the declaring document, from that
/// document's folder.
#[test]
fn a_referenced_owners_authored_file_value_is_not_found_beside_the_caller() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "nested/child.yaml", "kind: sequence\n$schema:\n  spec: file(required;eager)\nspec: work/spec.md\nsequence: []\n");
    let root = write(dir.path(), "root/root.yaml", "kind: sequence\nsequence:\n  - name: nested\n    sequence: ../nested/child.yaml\n");
    let caller = dir.path().join("caller");
    write(&caller, "work/spec.md", "# beside the caller\n");

    let error = discover_invoked(&root, &caller, json!({})).unwrap_err();
    let message = error.to_string();
    assert!(
        matches!(&error, CompositionError::SchemaValidation { source_path, problems, .. }
            if source_path.ends_with("nested/child.yaml") && problems == &["/spec"]),
        "{error:?}"
    );
    let declaring = biscuit_file::canonicalize_simplified(&dir.path().join("nested")).unwrap();
    assert!(message.ends_with(&format!("while resolving from `{}`", declaring.display())), "{message}");
}
