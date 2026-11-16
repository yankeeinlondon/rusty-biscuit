//! A sequence document's views and its owner values, observed through the
//! public library entry points: `without_formal_sequence_keys`, the two
//! reloads, `prepare_sequence_document_values`, and `resolve_sequence_plan_with`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::composition::error::CompositionError;
use crate::composition::sequence::model::{SequencePlan, SequenceSource};
use crate::composition::sequence::{SequenceSourceOptions, resolve_sequence_plan_with};
use crate::composition::types::{CallerInputLayers, CompositionMode, ResolvedCompositionSource};
use crate::composition::{PrepareOptions, reload_authored_source, reload_composition_source, without_formal_sequence_keys};

use super::prepare_sequence_document_values;

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, text).unwrap();
    path
}

/// `path` loaded as the document Claudine was asked to run, authored view.
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

/// The same sequence document as a Markdown file and a YAML file.
fn both_formats(dir: &Path, stem: &str, yaml: &str) -> [PathBuf; 2] {
    [
        write(dir, &format!("{stem}.md"), &format!("---\n{yaml}---\nBody.\n")),
        write(dir, &format!("{stem}.yaml"), yaml),
    ]
}

fn frontmatter(source: &ResolvedCompositionSource) -> Value {
    Value::Object(source.markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

/// Every declaration `without_formal_sequence_keys` must drop, at every depth
/// it walks: the root, an inline container, a container inside it, and a
/// container that is an inline group's leg. A `template` key inside a plain
/// value and on a nested *reference* task is not a declaration of an inline
/// list, and stays for the parser or the document to judge.
const DECLARING_DOCUMENT: &str = r#"kind: sequence
$schema:
  reviewer: string(required)
reviewer: ada
config:
  template: not a declaration
$state:
  color: string
template:
  desc: "{{ color }}"
sequence:
  - name: outer
    $schema:
      reviewer: string(required)
    $state:
      rank: number
    template:
      tag: "{{ name }}"
    sequence:
      - name: inner
        $state: {}
        template: {}
        sequence:
          - leaf
  - name: fan
    group:
      tasks:
        - name: leg
          template:
            leg: "{{ name }}"
          sequence:
            - one
  - name: reference
    sequence: child.yaml
"#;

#[test]
fn both_formats_compose_without_their_declarations_and_keep_the_document_schema() {
    let dir = TempDir::new().unwrap();
    let views: Vec<Value> = both_formats(dir.path(), "seq", DECLARING_DOCUMENT)
        .iter()
        .map(|path| frontmatter(&without_formal_sequence_keys(&load(path))))
        .collect();
    assert_eq!(views[0], views[1], "Markdown and YAML compose the same frontmatter");

    let view = &views[0];
    assert_eq!(view["$schema"], json!({"reviewer": "string(required)"}), "the root `$schema` is the document schema");
    assert!(view.get("$state").is_none() && view.get("template").is_none(), "{view:#}");
    assert_eq!(view["config"], json!({"template": "not a declaration"}), "plain values are not walked");
    let outer = &view["sequence"][0];
    for key in ["$schema", "$state", "template"] {
        assert!(outer.get(key).is_none(), "outer container keeps `{key}`: {outer:#}");
        assert!(outer["sequence"][0].get(key).is_none(), "nested container keeps `{key}`");
    }
    assert_eq!(outer["sequence"][0]["sequence"], json!(["leaf"]));
    let leg = &view["sequence"][1]["group"]["tasks"][0];
    assert!(leg.get("template").is_none(), "a group leg's container keeps its template: {leg:#}");
    assert_eq!(leg["sequence"], json!(["one"]));
    assert_eq!(view["sequence"][2], json!({"name": "reference", "sequence": "child.yaml"}));

    // The projection keeps authored key order, root and nested.
    let source = without_formal_sequence_keys(&load(&dir.path().join("seq.md")));
    let keys: Vec<&String> = source.markdown.frontmatter().as_map().keys().collect();
    assert_eq!(keys, ["kind", "$schema", "reviewer", "config", "sequence"]);
    let outer_keys: Vec<&String> = view["sequence"][0].as_object().unwrap().keys().collect();
    assert_eq!(outer_keys, ["name", "sequence"]);
}

/// A document with nothing to drop is returned as it is, and a document that
/// declares no sequence keeps an ordinary `template` property.
#[test]
fn a_document_without_list_declarations_is_unchanged() {
    let dir = TempDir::new().unwrap();
    for text in ["template: a property\nagent: claude\n", "sequence: child.yaml\nkind: sequence\n", "sequence:\n  - a\n  - b\n"] {
        let source = load(&write(dir.path(), "plain.yaml", text));
        assert_eq!(frontmatter(&without_formal_sequence_keys(&source)), frontmatter(&source), "{text}");
    }
}

/// A reread for composition drops the declarations as the first read did; a
/// reread for the declarations keeps them, nested ones included, together
/// with the file's current text.
#[test]
fn the_two_rereads_differ_only_in_the_declarations() {
    let dir = TempDir::new().unwrap();
    for path in both_formats(dir.path(), "seq", DECLARING_DOCUMENT) {
        let first = load(&path);
        fs::write(&path, fs::read_to_string(&path).unwrap().replace("reviewer: ada", "reviewer: grace")).unwrap();

        let authored = reload_authored_source(&first).unwrap();
        let composed = reload_composition_source(&first).unwrap();
        assert_eq!(frontmatter(&authored)["reviewer"], json!("grace"), "{}", path.display());
        assert_eq!(frontmatter(&composed)["reviewer"], json!("grace"));
        assert_eq!(frontmatter(&authored)["template"], json!({"desc": "{{ color }}"}));
        assert!(frontmatter(&authored)["sequence"][0].get("$schema").is_some());
        assert_eq!(frontmatter(&composed), frontmatter(&without_formal_sequence_keys(&authored)));
        assert_eq!(authored.original_text, composed.original_text);
    }
}

/// Options preparing against a captured context and `setters` supplied in
/// `caller_dir`, the way a launch records them.
fn prepare_options(caller_dir: &Path, setters: Value) -> PrepareOptions {
    let layers = CallerInputLayers::from_caller_overrides(Some(setters), crate::test_support::context_at(caller_dir));
    let mut options = layers.apply_to(PrepareOptions::new(crate::test_support::context_at(caller_dir)));
    options.prepared_context = Some(darkmatter::markdown::compose::ComposeContext::capture_for_document(
        caller_dir,
        &darkmatter::markdown::Markdown::with_frontmatter(darkmatter::markdown::Frontmatter::new(), ""),
    ));
    options
}

/// A document input from a caller in another directory is converted by the
/// document's `$schema` before a derived property reads it, with every value
/// read from the frontmatter alone: no shell runs and the body is not read.
#[test]
fn document_values_are_converted_before_a_derived_property_reads_them() {
    let dir = TempDir::new().unwrap();
    let caller = dir.path().join("caller");
    write(&caller, "work/spec.md", "# spec\n");
    let marker = dir.path().join("ran");
    let yaml = format!(
        "kind: sequence\n$schema:\n  spec: file(required;eager) -> the spec\nplan: \"{{{{ dirname(spec) + '/plan.md' }}}}\"\nprobe: \"$(touch {})\"\nsequence: []\n",
        marker.display()
    );
    for path in both_formats(&dir.path().join("docs"), "complete", &yaml) {
        let source = load(&path);
        // A body that could not compose must not matter: it is never read.
        let prepared = prepare_sequence_document_values(
            &source,
            prepare_options(&caller, json!({"spec": "work/spec.md"})),
            CompositionMode::ChainedDocument,
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", path.display()));
        let spec_dir = biscuit_file::canonicalize_simplified(&caller.join("work")).unwrap();
        let plan = prepared.values["plan"].as_str().unwrap();
        assert_eq!(
            biscuit_file::canonicalize_simplified(Path::new(plan).parent().unwrap()).unwrap(),
            spec_dir,
            "{}: `plan` read the converted caller-relative `spec`: {plan}",
            path.display()
        );
        assert!(prepared.values["probe"].as_str().unwrap().starts_with("$("), "the shell value keeps its text");
        assert!(!marker.exists(), "no frontmatter shell ran");
        assert!(prepared.launch_schema.is_some() && !prepared.schema_verdict_deferred);
    }
}

/// Without the input, the verdict is the typed missing-property report a full
/// preparation gives, even though the document has no steps; a withheld
/// verdict instead returns the values.
#[test]
fn a_missing_document_input_is_typed_unless_the_verdict_is_withheld() {
    let dir = TempDir::new().unwrap();
    let yaml = "kind: sequence\n$schema:\n  spec: file(required;eager)\nsequence: []\n";
    for path in both_formats(dir.path(), "empty", yaml) {
        let source = load(&path);
        let error = prepare_sequence_document_values(&source, prepare_options(dir.path(), json!({})), CompositionMode::ChainedDocument)
            .unwrap_err();
        match error {
            CompositionError::MissingProperties { missing, source_path, .. } => {
                assert_eq!(missing.iter().map(|property| property.name.as_str()).collect::<Vec<_>>(), ["spec"]);
                assert_eq!(source_path, path);
            }
            other => panic!("{}: expected MissingProperties, got {other:?}", path.display()),
        }
        let mut withheld = prepare_options(dir.path(), json!({}));
        withheld.defer_schema_verdict = true;
        let prepared = prepare_sequence_document_values(&source, withheld, CompositionMode::ChainedDocument).unwrap();
        assert!(prepared.schema_verdict_deferred && prepared.values.get("spec").is_none());
    }
}

/// Resolve `path` with a captured context and `setters` from `caller_dir`.
fn plan_with(path: &Path, caller_dir: &Path, setters: Value) -> Result<SequencePlan, CompositionError> {
    let source = load(path);
    let options = prepare_options(caller_dir, setters);
    let layers = CallerInputLayers::from_options(&options);
    let context = options.prepared_context.clone().unwrap();
    let resolution = crate::test_support::context_for(path);
    resolve_sequence_plan_with(
        &source,
        SequenceSourceOptions::new(&resolution).with_compose_context(&context).with_caller_layers(&layers),
    )
    .map(|plan| plan.expect("declares a sequence"))
}

fn states(plan: &SequencePlan) -> Vec<Value> {
    plan.steps.iter().map(|step| step.state.to_value()).collect()
}

/// The spec's example in both formats: one template value reads an item-only
/// field (premature interpolation would fail or empty it) and one reads a
/// document input a setter supplies (missing layering would lose it).
const COLORS: &str = r#"kind: sequence
$schema:
  reviewer: string(required)
$state:
  color: string(required)
  rank: number(required)
  desc: string(required)
  signed: string(required)
template:
  desc: "{{ color }}({{ rank }})"
  signed: "{{ reviewer }}"
sequence:
  - name: blue
    color: blue
    rank: 5
  - red
"#;

#[test]
fn markdown_and_yaml_lists_are_filled_and_typed_alike() {
    let dir = TempDir::new().unwrap();
    let [markdown, yaml] = both_formats(dir.path(), "colors", &COLORS.replace("  - red\n", "  - name: red\n    color: red\n    rank: 3\n"));
    let from_markdown = plan_with(&markdown, dir.path(), json!({"reviewer": "ada"})).unwrap();
    let from_yaml = plan_with(&yaml, dir.path(), json!({"reviewer": "ada"})).unwrap();
    assert_eq!(states(&from_markdown).len(), 2);
    for (left, right) in states(&from_markdown).iter().zip(states(&from_yaml)) {
        let mut right = right;
        // The token is keyed to the invoking file, which differs by name.
        right["sequence_id"] = left["sequence_id"].clone();
        assert_eq!(*left, right);
    }
    assert_eq!(from_markdown.steps[0].state.extra["desc"], json!("blue(5)"));
    assert_eq!(from_markdown.steps[1].state.extra["signed"], json!("ada"));
    for plan in [&from_markdown, &from_yaml] {
        let scope = plan.document_scope.as_ref().expect("an authored list has its document");
        assert_eq!(scope.property, "sequence");
        assert!(scope.authored.contains_key("$state") && scope.authored.contains_key("template"));
    }

    // A Markdown `$state` judges its steps exactly as a YAML one: a scalar
    // step has no `color`, and the failure names the step.
    let [markdown, _] = both_formats(dir.path(), "partial", COLORS);
    match plan_with(&markdown, dir.path(), json!({"reviewer": "ada"})).unwrap_err() {
        CompositionError::SequenceStateSchemaViolation { index, id, .. } => assert_eq!((index, id.as_str()), (1, "red")),
        other => panic!("expected SequenceStateSchemaViolation, got {other:?}"),
    }
}

/// A referenced whole document is its own owner in every whole-document
/// format: its template reads its own properties under the caller's setters,
/// never the referring document's, and resolves files from its own folder.
#[test]
fn a_referenced_whole_document_fills_its_template_from_its_own_values() {
    let dir = TempDir::new().unwrap();
    let child_dir = dir.path().join("lists");
    write(&child_dir, "beside-child.txt", "x");
    let yaml = r#"kind: sequence
owner: child
tone: plain
template:
  owner: "{{ owner }}"
  tone: "{{ tone }}"
  found: "{{ file_exists('./beside-child.txt') }}"
sequence:
  - name: one
"#;
    let children = [
        write(&child_dir, "child.yaml", yaml),
        write(&child_dir, "child.md", &format!("---\n{yaml}---\n")),
        write(
            &child_dir,
            "child.json",
            r#"{"kind": "sequence", "owner": "child", "tone": "plain", "template": {"owner": "{{ owner }}", "tone": "{{ tone }}", "found": "{{ file_exists('./beside-child.txt') }}"}, "sequence": [{"name": "one"}]}"#,
        ),
        write(
            &child_dir,
            "child.json5",
            "// comments are JSON5\n{kind: 'sequence', owner: 'child', tone: 'plain', template: {owner: '{{ owner }}', tone: '{{ tone }}', found: \"{{ file_exists('./beside-child.txt') }}\"}, sequence: [{name: 'one'},],}\n",
        ),
    ];
    for child in children {
        let name = child.file_name().unwrap().to_str().unwrap().to_string();
        let referrer = write(dir.path(), "run.yaml", &format!("kind: sequence\nowner: referrer\ntone: referrer\nsequence: lists/{name}\n"));
        let plan = plan_with(&referrer, dir.path(), json!({"tone": "setter"})).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let state = &plan.steps[0].state.extra;
        assert_eq!(state["owner"], json!("child"), "{name}: the referrer's `owner` never reaches the child");
        assert_eq!(state["tone"], json!("setter"), "{name}: a setter wins over the child's own value");
        assert_eq!(state["found"], json!(true), "{name}: files resolve beside the declaring document");
        let scope = plan.document_scope.as_ref().expect("a whole referenced document has its scope");
        // Compared by trailing components: the resolver's path form (long or
        // short names, a verbatim prefix) is the platform's, not the test's.
        let declared = Path::new("lists").join(&name);
        assert!(scope.declaring_path.ends_with(&declared), "{name}: {}", scope.declaring_path.display());
        assert!(matches!(&plan.source, SequenceSource::External { path } if path.ends_with(&declared)), "{name}");
    }
}

/// `$state` judges each step passively: a default the contract declares
/// satisfies it, but nothing the contract converts or defaults is written
/// into the step.
#[test]
fn state_validation_never_writes_into_the_step() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "seq.yaml", "kind: sequence\n$state:\n  rank: number(default(3))\nsequence:\n  - plain\n");
    let plan = plan_with(&path, dir.path(), json!({})).unwrap();
    assert!(plan.steps[0].state.extra.get("rank").is_none(), "{:?}", plan.steps[0].state);
}

/// A selection out of a sequence document is data: the selected file's
/// `$state` and `template` are not applied, and the plan has no document.
#[test]
fn a_selection_ignores_the_selected_documents_declarations() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "child.yaml",
        "kind: sequence\n$state:\n  color: string(required)\ntemplate:\n  desc: filled\nsequence:\n  - name: one\n",
    );
    for reference in ["child.yaml -> sequence", "child.yaml -> sequence::name(name)"] {
        let referrer = write(dir.path(), "run.yaml", &format!("kind: sequence\nsequence: \"{reference}\"\n"));
        let plan = plan_with(&referrer, dir.path(), json!({})).unwrap_or_else(|error| panic!("{reference}: {error:?}"));
        assert_eq!(plan.steps.len(), 1, "{reference}");
        assert!(plan.steps[0].state.extra.get("desc").is_none(), "{reference}: {:?}", plan.steps[0].state);
        assert!(plan.document_scope.is_none(), "{reference}");
    }
}

/// Transitional: a value only the referrer defines still fills a referenced
/// document's template, as the shipped research fleets' shared rosters need,
/// but it is the lowest layer: the document's own value wins.
#[test]
fn a_referrer_value_is_only_a_lowest_layer_under_a_referenced_documents_own() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "roster.yaml",
        "kind: sequence\nshared: roster\ntemplate:\n  topic: \"{{ research_prompt }}\"\n  shared: \"{{ shared }}\"\nsequence:\n  - one\n",
    );
    let referrer = write(dir.path(), "fleet.yaml", "kind: sequence\nresearch_prompt: ./topic.md\nshared: fleet\nsequence: roster.yaml\n");
    let plan = plan_with(&referrer, dir.path(), json!({})).unwrap();
    assert_eq!(plan.steps[0].state.extra["topic"], json!("./topic.md"));
    assert_eq!(plan.steps[0].state.extra["shared"], json!("roster"));
}
