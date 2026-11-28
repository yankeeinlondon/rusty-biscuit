//! A provider prediction follows the authored reads of exactly the owners
//! that supplied it. A task `with:` value reads the enclosing sequence's
//! state, where early binding holds a derived value as its prepared literal;
//! the prediction still follows every authored link of that chain to a key a
//! lifecycle `set` writes. A prompt document's own key is never confused with
//! a sequence key of the same name, and a `with:` entry the document reads is
//! followed in the sequence's scope, where it is evaluated.
//!
//! Each case runs on every placement of the owner that declares the reading
//! rows, built as the CLI builds it (input discovery, then the graph), and
//! the dry run's recovering walk ([`check_sequence_tree`]) must predict every
//! reader alike: it consumes the same runtime-written keys.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::providers::{HintLayer, HintState, LaunchDecision, ProviderPrediction};
use super::*;

/// Where the owner declaring the reading rows sits.
#[derive(Clone, Copy, Debug)]
enum Site {
    /// The invoked document's own rows.
    Root,
    /// A whole document the root's `sequence:` names.
    RootReference,
    /// A whole document a root row's `sequence:` names.
    NestedReference,
    /// An inline `sequence:` list inside the invoked document.
    InlineChild,
    /// A whole document a group leg's `sequence:` names.
    GroupLeg,
    /// A whole document a `kind: task` file's `sequence:` names.
    TaskFile,
    /// An unrelated whole document beside the readers, which are root rows.
    Sibling,
}

/// The prompt documents every case reads, written beside each document that
/// may declare a row.
const PROMPTS: [(&str, &str); 13] = [
    ("writes.md", "---\nagent: goose\nsuccess:\n  - set:\n      pick: goose\n---\n\nWrite.\n"),
    ("plain.md", "---\ntitle: p\n---\n\nPlain.\n"),
    ("owned_agent_0.md", "---\nagent: \"{{ pick }}\"\n---\n\nOwned.\n"),
    ("owned_agent_1.md", "---\nm1: \"{{ pick }}\"\nagent: \"{{ m1 }}\"\n---\n\nOwned.\n"),
    ("owned_agent_3.md", "---\nm1: \"{{ pick }}\"\nm2: \"{{ m1 }}\"\nm3: \"{{ m2 }}\"\nagent: \"{{ m3 }}\"\n---\n\nOwned.\n"),
    ("owned_model_0.md", "---\nmodel: \"{{ pick }}\"\n---\n\nOwned.\n"),
    ("owned_model_1.md", "---\nm1: \"{{ pick }}\"\nmodel: \"{{ m1 }}\"\n---\n\nOwned.\n"),
    ("owned_model_3.md", "---\nm1: \"{{ pick }}\"\nm2: \"{{ m1 }}\"\nm3: \"{{ m2 }}\"\nmodel: \"{{ m3 }}\"\n---\n\nOwned.\n"),
    // The document's own `choice` is literal; its hint reads a `with:` entry.
    ("via_with_agent.md", "---\nchoice: claude\nagent: \"{{ who }}\"\n---\n\nVia.\n"),
    ("via_with_model.md", "---\nchoice: claude\nmodel: \"{{ who }}\"\n---\n\nVia.\n"),
    // The document's hint reads its own literal `choice`.
    ("reads_agent.md", "---\nchoice: claude\nagent: \"{{ choice }}\"\n---\n\nRead.\n"),
    ("reads_model.md", "---\nchoice: claude\nmodel: \"{{ choice }}\"\n---\n\nRead.\n"),
    ("owned_agent_2.md", "---\nm1: \"{{ pick }}\"\nm2: \"{{ m1 }}\"\nagent: \"{{ m2 }}\"\n---\n\nOwned.\n"),
];

/// The derived chains the `with:` cases read: `mid` is one link from
/// `pick`, `choice` two, `deep` three; `n2` is two links from `q`, which
/// nothing writes.
const CHAINS: &str = "pick: claude\nmid: \"{{ pick }}\"\nchoice: \"{{ mid }}\"\ndeep: \"{{ choice }}\"\nq: claude\nn1: \"{{ q }}\"\nn2: \"{{ n1 }}\"\n";

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, text).unwrap();
    biscuit_file::canonicalize_simplified(&path).unwrap()
}

fn owner_document(keys: &str, rows: &[Value]) -> String {
    format!("kind: sequence\n{keys}sequence: {}\n", serde_json::to_string(rows).unwrap())
}

impl Site {
    /// Write the files placing `readers` in an owner whose own values are
    /// `keys`, after a root row whose prompt writes `pick`; return the
    /// invoked document.
    fn write(self, dir: &Path, keys: &str, readers: &[Value]) -> PathBuf {
        for (name, text) in PROMPTS {
            write(dir, name, text);
            write(dir, &format!("sub/{name}"), text);
        }
        let writer = json!({ "name": "writer", "prompt": "writes.md" });
        let child = |rows: &[Value]| write(dir, "sub/child.yaml", &owner_document(keys, rows));
        let root = |rows: &[Value]| write(dir, "root.yaml", &owner_document("", rows));
        match self {
            Self::Root => write(dir, "root.yaml", &owner_document(keys, &[&[writer], readers].concat())),
            Self::RootReference => {
                child(&[&[writer], readers].concat());
                write(dir, "root.yaml", "kind: sequence\nsequence: ./sub/child.yaml\n")
            }
            Self::NestedReference => {
                child(readers);
                root(&[writer, json!({ "name": "outer", "sequence": "./sub/child.yaml" })])
            }
            Self::InlineChild => write(
                dir,
                "root.yaml",
                &owner_document(keys, &[writer, json!({ "name": "inner", "sequence": readers })]),
            ),
            Self::GroupLeg => {
                child(readers);
                root(&[writer, json!({ "name": "fan", "group": {
                    "name": "G", "tasks": [{ "name": "leg", "sequence": "./sub/child.yaml" }],
                } })])
            }
            Self::TaskFile => {
                child(readers);
                write(dir, "sub/t.yaml", "kind: task\nname: delegated\nsequence: ./child.yaml\n");
                root(&[writer, json!({ "name": "delegate", "task": "./sub/t.yaml" })])
            }
            Self::Sibling => {
                write(dir, "sub/sibling.yaml", &owner_document(keys, &[json!({ "name": "s", "prompt": "plain.md" })]));
                root(&[&[writer, json!({ "name": "sibling", "sequence": "./sub/sibling.yaml" })], readers].concat())
            }
        }
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

/// The graph as the CLI builds it: input discovery, then the graph reusing
/// its owners, with `setters` as the caller's values and `content` naming
/// the context groups captured (the CLI captures those its documents read).
fn graph(path: &Path, dir: &Path, setters: Option<&Value>, content: &str) -> PreflightGraph {
    let source = load(path);
    let invocation =
        crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let context = ComposeContext::capture_for_content(dir, content);
    let resolution = source_context.file_resolution_context().clone();
    let options = crate::composition::SequenceSourceOptions::new(&resolution)
        .with_compose_context(&context)
        .with_invocation(&invocation);
    let plan = crate::composition::sequence::resolve_sequence_plan_with(&source, options).unwrap().expect("a sequence");
    let inventory = discover_sequence_inputs(
        &source,
        InputDiscoveryOptions::new(&resolution, &context).with_invocation(&invocation, &source_context),
    )
    .unwrap();
    let layers = setters.map(|setters| {
        let mut layers = crate::composition::CallerInputLayers::new(resolution.clone());
        layers.set_overrides = Some(setters.clone());
        layers
    });
    build_preflight_graph_after_discovery(&plan, &source, context, &invocation, &source_context, layers.as_ref(), &inventory)
        .unwrap()
}

/// Every prompt task's prediction, by row name, at any depth.
fn predictions(graph: &PreflightGraph) -> HashMap<String, ProviderPrediction> {
    fn visit(name: Option<&str>, task: &PreflightTask, found: &mut HashMap<String, ProviderPrediction>) {
        if let (Some(name), Some(prediction)) = (name.or(task.name.as_deref()), &task.provider) {
            found.insert(name.to_string(), prediction.clone());
        }
        match &task.action {
            PreflightAction::Sequence(sequence) => visit_steps(&sequence.steps, found),
            PreflightAction::Group(group) => group.tasks.iter().for_each(|leg| visit(None, leg, found)),
            _ => {}
        }
    }
    fn visit_steps(steps: &[PreflightStep], found: &mut HashMap<String, ProviderPrediction>) {
        for step in steps {
            if let Some(task) = &step.task {
                visit(Some(&step.name), task, found);
            }
        }
    }
    let mut found = HashMap::new();
    visit_steps(&graph.steps, &mut found);
    found
}

/// Build `readers` placed at `site` under the owner values `keys` and return
/// every prompt task's prediction.
fn predict_at(site: Site, keys: &str, readers: &[Value], setters: Option<&Value>) -> HashMap<String, ProviderPrediction> {
    predict_capturing(site, keys, readers, setters, "")
}

/// [`predict_at`], capturing the context groups `content` reads.
fn predict_capturing(
    site: Site, keys: &str, readers: &[Value], setters: Option<&Value>, content: &str,
) -> HashMap<String, ProviderPrediction> {
    let dir = TempDir::new().unwrap();
    let path = site.write(dir.path(), keys, readers);
    let found = predictions(&graph(&path, dir.path(), setters, content));
    let checked = checked_predictions(&path, dir.path(), setters, content);
    for reader in readers {
        let name = reader["name"].as_str().unwrap();
        assert!(found.contains_key(name), "{site:?}: no prediction for `{name}` in {found:#?}");
        let walked = checked.get(name).unwrap_or_else(|| panic!("{site:?}: the check walk has no prediction for `{name}`"));
        for field in ["agent", "model"] {
            assert_eq!(
                shape(hint(&checked, name, field)),
                shape(hint(&found, name, field)),
                "{site:?}: the dry run predicts `{name}`'s {field} unlike the real graph: {walked:#?}"
            );
        }
    }
    found
}

/// Every prompt task's prediction as the dry run's recovering walk records
/// it, by row name.
fn checked_predictions(path: &Path, dir: &Path, setters: Option<&Value>, content: &str) -> HashMap<String, ProviderPrediction> {
    let source = load(path);
    let invocation =
        crate::invocation_context::InvocationContext::capture_at(&crate::test_support::snapshot(), dir).unwrap();
    let source_context = invocation.derive_source(&source.resolved_path).unwrap();
    let layers = setters.map(|setters| {
        let mut layers = crate::composition::CallerInputLayers::new(source_context.file_resolution_context().clone());
        layers.set_overrides = Some(setters.clone());
        layers
    });
    let check = check_sequence_tree(SequenceCheckRequest {
        source: &source,
        compose_context: ComposeContext::capture_for_content(dir, content),
        invocation: &invocation,
        source_context: &source_context,
        caller_layers: layers.as_ref(),
        launch_dir: Some(dir),
        session: None,
    });
    check.tasks.into_iter().filter_map(|task| Some((task.site.name, task.provider?))).collect()
}

/// A hint's class, value, and layer: what selection reads, without the
/// property path, which a check spells by its authored key.
fn shape(state: &HintState) -> String {
    match state {
        HintState::Absent => "absent".into(),
        HintState::Static { value, source } => format!("static {value} {:?}", source.layer),
        HintState::DecidedAtLaunch { source, reason } => format!("launch {reason:?} {:?}", source.layer),
        HintState::Failed { source, .. } => format!("failed {:?}", source.layer),
        HintState::Circular { source, variable } => format!("circular {variable} {:?}", source.layer),
        HintState::EmptyAgentList { source } => format!("empty {:?}", source.layer),
    }
}

fn reader(name: &str, prompt: &str, with: Value) -> Value {
    let mut row = json!({ "name": name, "prompt": prompt });
    if with != Value::Null {
        row["with"] = with;
    }
    row
}

/// `field` of `name`'s prediction.
fn hint<'p>(found: &'p HashMap<String, ProviderPrediction>, name: &str, field: &str) -> &'p HintState {
    let prediction = &found[name];
    if field == "agent" { &prediction.agent } else { &prediction.model }
}

fn assert_deferred(site: Site, found: &HashMap<String, ProviderPrediction>, name: &str, field: &str) {
    let state = hint(found, name, field);
    assert!(
        matches!(state, HintState::DecidedAtLaunch { reason: LaunchDecision::RuntimeWritten, .. }),
        "{site:?}: `{name}`'s {field} must wait for the runtime write: {state:?}"
    );
}

fn assert_static(site: Site, found: &HashMap<String, ProviderPrediction>, name: &str, field: &str, layer: HintLayer) {
    let state = hint(found, name, field);
    assert!(
        matches!(state, HintState::Static { value, source } if *value == json!("claude") && source.layer == layer),
        "{site:?}: `{name}`'s {field} must stay a static `claude` from {layer:?}: {state:?}"
    );
}

/// `with.agent`/`with.model` reading two- and three-link derived chains to a
/// key a prompt's `success` writes are decided at launch; a two-link chain to
/// a key nothing writes stays static.
fn check_derived_with_chains(site: Site) {
    let readers = [
        reader("agent_2", "plain.md", json!({ "agent": "{{ choice }}" })),
        reader("agent_3", "plain.md", json!({ "agent": "{{ deep }}" })),
        reader("model_2", "plain.md", json!({ "model": "{{ choice }}" })),
        reader("model_3", "plain.md", json!({ "model": "{{ deep }}" })),
        reader("unwritten_agent", "plain.md", json!({ "agent": "{{ n2 }}" })),
        reader("unwritten_model", "plain.md", json!({ "model": "{{ n2 }}" })),
    ];
    let found = predict_at(site, CHAINS, &readers, None);
    for (name, field) in [("agent_2", "agent"), ("agent_3", "agent"), ("model_2", "model"), ("model_3", "model")] {
        assert_deferred(site, &found, name, field);
    }
    assert_static(site, &found, "unwritten_agent", "agent", HintLayer::With);
    assert_static(site, &found, "unwritten_model", "model", HintLayer::With);
}

/// A `with:` value reading the written key directly, or through one link.
fn check_direct_and_one_link_with_reads(site: Site) {
    let readers = [
        reader("agent_0", "plain.md", json!({ "agent": "{{ pick }}" })),
        reader("agent_1", "plain.md", json!({ "agent": "{{ mid }}" })),
        reader("model_0", "plain.md", json!({ "model": "{{ pick }}" })),
        reader("model_1", "plain.md", json!({ "model": "{{ mid }}" })),
    ];
    let found = predict_at(site, CHAINS, &readers, None);
    for (name, field) in [("agent_0", "agent"), ("agent_1", "agent"), ("model_0", "model"), ("model_1", "model")] {
        assert_deferred(site, &found, name, field);
    }
}

/// A prompt document that owns its whole chain, zero to three links.
fn check_prompt_owned_chains(site: Site) {
    let mut readers = Vec::new();
    for links in 0..4 {
        readers.push(reader(&format!("owned_agent_{links}"), &format!("owned_agent_{links}.md"), Value::Null));
    }
    for links in [0, 1, 3] {
        readers.push(reader(&format!("owned_model_{links}"), &format!("owned_model_{links}.md"), Value::Null));
    }
    let found = predict_at(site, CHAINS, &readers, None);
    for links in 0..4 {
        assert_deferred(site, &found, &format!("owned_agent_{links}"), "agent");
    }
    for links in [0, 1, 3] {
        assert_deferred(site, &found, &format!("owned_model_{links}"), "model");
    }
}

/// A document hint reading a `with:` entry whose value reads a two-link
/// sequence chain is decided at launch: the entry is followed in the
/// sequence's scope, never through the document's own literal `choice`.
fn check_document_hint_through_with(site: Site) {
    let readers = [
        reader("via_agent", "via_with_agent.md", json!({ "who": "{{ choice }}" })),
        reader("via_model", "via_with_model.md", json!({ "who": "{{ choice }}" })),
        reader("via_unwritten", "via_with_agent.md", json!({ "who": "{{ n2 }}" })),
    ];
    let found = predict_at(site, CHAINS, &readers, None);
    assert_deferred(site, &found, "via_agent", "agent");
    assert_deferred(site, &found, "via_model", "model");
    assert_static(site, &found, "via_unwritten", "agent", HintLayer::Document);
}

/// The owner values of the R3 cases: a derived `choice`, unrelated to the
/// prompt's own literal `choice`, over the key the writer sets.
const DERIVED_CHOICE: &str = "pick: goose\nchoice: \"{{ pick }}\"\n";

/// A prompt's hint reading its own literal `choice` stays static, whatever
/// another owner's derived `choice` reads.
fn check_prompt_choice_is_its_own(site: Site) {
    let readers = [
        reader("reads_agent", "reads_agent.md", Value::Null),
        reader("reads_model", "reads_model.md", Value::Null),
    ];
    let found = predict_at(site, DERIVED_CHOICE, &readers, None);
    assert_static(site, &found, "reads_agent", "agent", HintLayer::Document);
    assert_static(site, &found, "reads_model", "model", HintLayer::Document);
}

/// The literal-owner control: nothing derived anywhere.
fn check_literal_owner_control(site: Site) {
    let readers = [
        reader("reads_agent", "reads_agent.md", Value::Null),
        reader("reads_model", "reads_model.md", Value::Null),
    ];
    let found = predict_at(site, "pick: goose\nchoice: goose\n", &readers, None);
    assert_static(site, &found, "reads_agent", "agent", HintLayer::Document);
    assert_static(site, &found, "reads_model", "model", HintLayer::Document);
}

/// A literal `with:` replaces the document expression.
fn check_literal_with_override(site: Site) {
    let readers = [
        reader("reads_agent", "reads_agent.md", json!({ "agent": "claude" })),
        reader("reads_model", "reads_model.md", json!({ "model": "claude" })),
    ];
    let found = predict_at(site, DERIVED_CHOICE, &readers, None);
    assert_static(site, &found, "reads_agent", "agent", HintLayer::With);
    assert_static(site, &found, "reads_model", "model", HintLayer::With);
}

/// A caller setter replaces the document expression.
fn check_caller_setter_override(site: Site) {
    let readers = [
        reader("reads_agent", "reads_agent.md", Value::Null),
        reader("reads_model", "reads_model.md", Value::Null),
    ];
    let setters = json!({ "agent": "claude", "model": "claude" });
    let found = predict_at(site, DERIVED_CHOICE, &readers, Some(&setters));
    assert_static(site, &found, "reads_agent", "agent", HintLayer::CallerSetter);
    assert_static(site, &found, "reads_model", "model", HintLayer::CallerSetter);
}

/// A `with:` hint reading an owner value derived from the launching
/// target's identity is circular, as the authored value reads: early binding
/// keeps that value's text rather than its prepared literal.
fn check_identity_derived_with_is_circular(site: Site) {
    let readers = [
        reader("identity_agent", "plain.md", json!({ "agent": "{{ who }}" })),
        reader("identity_model", "plain.md", json!({ "model": "{{ which }}" })),
    ];
    let keys = "who: \"{{ ctx.agent }}\"\nwhich: \"{{ ctx.model }}\"\n";
    // Captured, `ctx.agent` prepares to a literal (`unknown`), which early
    // binding would otherwise read in place of the identity read.
    let found = predict_capturing(site, keys, &readers, None, keys);
    for (name, field) in [("identity_agent", "agent"), ("identity_model", "model")] {
        let state = hint(&found, name, field);
        assert!(
            matches!(state, HintState::Circular { source, .. } if source.layer == HintLayer::With),
            "{site:?}: `{name}`'s {field} reads the target identity through a derived value: {state:?}"
        );
    }
}

/// One test per placement of `check`.
macro_rules! on_every_site {
    ($module:ident => $check:ident) => {
        mod $module {
            use super::Site;
            #[test] fn direct_root() { super::$check(Site::Root) }
            #[test] fn root_reference() { super::$check(Site::RootReference) }
            #[test] fn nested_reference() { super::$check(Site::NestedReference) }
            #[test] fn inline_child() { super::$check(Site::InlineChild) }
            #[test] fn group_leg() { super::$check(Site::GroupLeg) }
            #[test] fn task_file() { super::$check(Site::TaskFile) }
        }
    };
    ($module:ident => $check:ident, with_sibling) => {
        mod $module {
            use super::Site;
            #[test] fn direct_root() { super::$check(Site::Root) }
            #[test] fn root_reference() { super::$check(Site::RootReference) }
            #[test] fn nested_reference() { super::$check(Site::NestedReference) }
            #[test] fn inline_child() { super::$check(Site::InlineChild) }
            #[test] fn group_leg() { super::$check(Site::GroupLeg) }
            #[test] fn task_file() { super::$check(Site::TaskFile) }
            #[test] fn independent_sibling() { super::$check(Site::Sibling) }
        }
    };
}

on_every_site!(derived_with_chains_wait_for_the_write => check_derived_with_chains);
on_every_site!(direct_and_one_link_with_reads_wait_for_the_write => check_direct_and_one_link_with_reads);
on_every_site!(prompt_owned_chains_wait_for_the_write => check_prompt_owned_chains);
on_every_site!(a_document_hint_through_a_derived_with_entry_waits => check_document_hint_through_with);
on_every_site!(a_prompts_own_choice_is_not_another_owners => check_prompt_choice_is_its_own, with_sibling);
on_every_site!(literal_owner_controls_stay_static => check_literal_owner_control, with_sibling);
on_every_site!(literal_with_overrides_stay_static => check_literal_with_override, with_sibling);
on_every_site!(caller_setter_overrides_stay_static => check_caller_setter_override, with_sibling);
on_every_site!(an_identity_derived_with_read_is_circular => check_identity_derived_with_is_circular);
