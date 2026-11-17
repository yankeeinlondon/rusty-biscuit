//! An inline container's `template:` reads its owning document's *prepared*
//! values: schema defaults (for an absent and an explicit `null` value
//! alike), derived expressions, and coercion apply exactly as they do for a
//! root template, wherever the container sits and however the graph is built.
//!
//! Every case is one owner document in four variants (a literal value, a
//! schema default for an absent value, the same default for `null`, and a
//! value derived from another property), placed at each site an inline list
//! can be authored. Each must fill the row `who: ada`, which the container's
//! `$state` then requires to be a string.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;

/// The owner's `reviewer` value, in each shape it can take.
const VARIANTS: [(&str, &str); 4] = [
    ("literal", "reviewer: ada\n"),
    ("absent with a default", "$schema:\n  reviewer: string(default(ada))\n"),
    ("null with a default", "$schema:\n  reviewer: string(default(ada))\nreviewer: null\n"),
    ("derived", "source: ada\nreviewer: \"{{ source }}\"\n"),
];

/// An inline container, indented to sit as a list item `indent` spaces in.
fn container(indent: usize) -> String {
    let lines = [
        "- name: nested",
        "  $state:",
        "    who: string(required)",
        "  template:",
        "    who: \"{{ reviewer }}\"",
        "  sequence:",
        "    - name: work",
        "      shell: \"echo {{ seq.who }}\"",
    ];
    lines.iter().map(|line| format!("{}{line}\n", " ".repeat(indent))).collect()
}

/// The sites an inline list can be authored at. Each writes its files under
/// `dir` with the owner header `owner` and returns the invoked document.
#[derive(Clone, Copy, Debug)]
enum Site {
    /// R1-F3.1: the invoked document's own list (the control).
    Root,
    /// R1-F3.2: an inline container in the invoked document.
    InlineNested,
    /// R1-F3.3: an inline container as a group leg.
    GroupLeg,
    /// R1-F3.4: an inline container declared by a `kind: task` file.
    TaskFile,
    /// R1-F3.5: an inline container inside a whole document a nested
    /// `sequence:` task references; the referenced document owns the values.
    NestedReference,
    /// R1-F3.5: the same, through the root's own `sequence:` reference.
    RootReference,
}

const SITES: [Site; 6] =
    [Site::Root, Site::InlineNested, Site::GroupLeg, Site::TaskFile, Site::NestedReference, Site::RootReference];

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, text).unwrap();
    biscuit_file::canonicalize_simplified(&path).unwrap()
}

impl Site {
    fn write(self, dir: &Path, owner: &str) -> PathBuf {
        let head = format!("kind: sequence\n{owner}");
        let text = match self {
            Self::Root => format!(
                "{head}$state:\n  who: string(required)\ntemplate:\n  who: \"{{{{ reviewer }}}}\"\nsequence:\n  - name: work\n    shell: \"echo {{{{ seq.who }}}}\"\n"
            ),
            Self::InlineNested => format!("{head}sequence:\n{}", container(2)),
            Self::GroupLeg => format!("{head}sequence:\n  - name: fan\n    group:\n      name: G\n      tasks:\n{}", container(8)),
            Self::TaskFile => {
                let task = container(0).lines().skip(1).map(|line| format!("{}\n", &line[2..])).collect::<String>();
                write(dir, "tasks/t.yaml", &format!("kind: task\n{task}"));
                format!("{head}sequence:\n  - name: delegated\n    task: ./tasks/t.yaml\n")
            }
            Self::NestedReference => {
                write(dir, "sub/child.yaml", &format!("{head}sequence:\n{}", container(2)));
                "kind: sequence\nsequence:\n  - name: outer\n    sequence: ./sub/child.yaml\n".to_string()
            }
            Self::RootReference => {
                write(dir, "sub/child.yaml", &format!("{head}sequence:\n{}", container(2)));
                "kind: sequence\nsequence: ./sub/child.yaml\n".to_string()
            }
        };
        write(dir, "root.yaml", &text)
    }
}

fn load(path: &Path) -> ResolvedCompositionSource {
    ResolvedCompositionSource {
        original_ref: path.display().to_string(),
        resolved_path: path.to_path_buf(),
        original_text: fs::read_to_string(path).unwrap(),
        markdown: crate::composition::resolve::load_yaml_document(path).unwrap(),
    }
}

/// How the graph is built.
#[derive(Clone, Copy, Debug)]
enum Build {
    /// As the CLI does: input discovery, then the graph reusing its owners.
    AfterDiscovery,
    /// Through an invocation, with no discovery: owners prepared on use.
    WithInvocation,
    /// With neither discovery nor an invocation.
    Plain,
}

/// The `who` of the first row of the innermost list `path` builds.
fn filled_who(path: &Path, dir: &Path, build: Build) -> Result<Value, CompositionError> {
    let source = load(path);
    let invocation =
        crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let context = ComposeContext::capture_for_content(dir, "");
    let resolution = source_context.file_resolution_context().clone();
    // The root plan is filled as the CLI fills it: from prepared values.
    let plan = crate::composition::sequence::resolve_sequence_plan_with(
        &source,
        crate::composition::SequenceSourceOptions::new(&resolution)
            .with_compose_context(&context)
            .with_invocation(&invocation),
    )?
    .expect("a sequence");
    let graph = match build {
        Build::AfterDiscovery => {
            let inventory = discover_sequence_inputs(
                &source,
                InputDiscoveryOptions::new(&resolution, &context).with_invocation(&invocation, &source_context),
            )?;
            build_preflight_graph_after_discovery(&plan, &source, context, &invocation, &source_context, None, &inventory)?
        }
        Build::WithInvocation => {
            build_preflight_graph_with_invocation(&plan, &source, context, &invocation, &source_context, None)?
        }
        Build::Plain => build_preflight_graph_with_context(&plan, &source, context, &resolution)?,
    };
    Ok(innermost_first_row(&plan, &graph))
}

fn innermost_first_row(plan: &SequencePlan, graph: &PreflightGraph) -> Value {
    fn inner(task: &PreflightTask) -> Option<&PreflightSequence> {
        match &task.action {
            PreflightAction::Sequence(sequence) => Some(
                sequence.steps.iter().filter_map(|step| step.task.as_ref()).find_map(inner).unwrap_or(sequence),
            ),
            PreflightAction::Group(group) => group.tasks.iter().find_map(inner),
            _ => None,
        }
    }
    let row = match graph.steps.iter().filter_map(|step| step.task.as_ref()).find_map(inner) {
        Some(sequence) => sequence.plan.steps[0].state.to_value(),
        None => plan.steps[0].state.to_value(),
    };
    row.get("who").cloned().unwrap_or(Value::Null)
}

#[test]
fn every_inline_template_reads_its_owners_prepared_values() {
    let mut failures = Vec::new();
    for site in SITES {
        for (variant, owner) in VARIANTS {
            for build in [Build::AfterDiscovery, Build::WithInvocation, Build::Plain] {
                let dir = TempDir::new().unwrap();
                let path = site.write(dir.path(), owner);
                match filled_who(&path, dir.path(), build) {
                    Ok(who) if who == json!("ada") => {}
                    Ok(who) => failures.push(format!("{site:?} / {variant} / {build:?}: who = {who}")),
                    Err(error) => failures.push(format!("{site:?} / {variant} / {build:?}: {error}")),
                }
            }
        }
    }
    assert!(failures.is_empty(), "{} case(s) did not fill `ada`:\n{}", failures.len(), failures.join("\n"));
}

/// A caller setter and a runtime write layer above the owner's own values,
/// and an item's own field still shadows them.
#[test]
fn setters_layer_over_prepared_values_and_an_item_field_shadows_them() {
    let dir = TempDir::new().unwrap();
    let path = Site::InlineNested.write(dir.path(), "$schema:\n  reviewer: string(default(ada))\n");
    let source = load(&path);
    let invocation =
        crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir.path()).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let context = ComposeContext::capture_for_content(dir.path(), "");
    let resolution = source_context.file_resolution_context().clone();
    let layers = crate::composition::CallerInputLayers::from_caller_overrides(
        Some(json!({ "reviewer": "grace" })), resolution.clone(),
    );
    let plan = crate::composition::sequence::resolve_sequence_plan_with(
        &source, crate::composition::SequenceSourceOptions::new(&resolution).with_compose_context(&context),
    ).unwrap().unwrap();
    let inventory = discover_sequence_inputs(
        &source,
        InputDiscoveryOptions::new(&resolution, &context).with_caller_layers(&layers).with_invocation(&invocation, &source_context),
    ).unwrap();
    let graph = build_preflight_graph_after_discovery(
        &plan, &source, context.clone(), &invocation, &source_context, Some(&layers), &inventory,
    ).unwrap();
    assert_eq!(innermost_first_row(&plan, &graph), json!("grace"), "a setter is above the default");

    let graph = build_preflight_graph_with_invocation(
        &plan, &source, context, &invocation, &source_context, Some(&json!({ "reviewer": "grace" })),
    ).unwrap();
    assert_eq!(innermost_first_row(&plan, &graph), json!("grace"), "without discovery too");

    let shadowed = fs::read_to_string(&path).unwrap().replace("      - name: work\n", "      - name: work\n        who: item\n");
    let path = write(dir.path(), "root.yaml", &shadowed);
    assert_eq!(filled_who(&path, dir.path(), Build::AfterDiscovery).unwrap(), json!("item"));
}

/// A template with no expression to fill prepares nothing: an owner whose
/// own preparation would fail is never prepared for it.
#[test]
fn a_literal_template_prepares_no_owner_values() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "root.yaml", "kind: sequence\n$schema:\n  spec: file(required;eager)\nspec: missing.md\nsequence:\n  - name: nested\n    template:\n      rank: 1\n    sequence:\n      - name: work\n        shell: echo hi\n");
    let source = load(&path);
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = crate::composition::sequence::resolve_sequence_plan(&source, &resolution).unwrap().unwrap();
    let context = ComposeContext::capture_for_content(dir.path(), "");
    let graph = build_preflight_graph_with_context(&plan, &source, context, &resolution).unwrap();
    let PreflightAction::Sequence(nested) = &graph.steps[0].task.as_ref().unwrap().action else { panic!("a container") };
    assert_eq!(nested.plan.steps[0].state.extra.get("rank"), Some(&json!(1)));
}
