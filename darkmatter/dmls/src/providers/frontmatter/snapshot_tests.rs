//! Snapshot reuse and invalidation as the server sees them: every request gets
//! a fresh [`DocumentOverlay`] from one long-lived [`OverlayState`], and its
//! public answers (diagnostics, hover, completion) must equal those of a state
//! that never saw an earlier input. Requests on the shared snapshot are
//! passive: they read, hash, parse, prepare, and discover nothing.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use darkmatter::work_counters::Work;
use lsp_types::{InitializeParams, Uri};

use crate::capabilities::ClientProfile;
use crate::config::DmlsConfig;
use crate::context::DocumentResolution;
use crate::graph::WorkspaceGraph;
use crate::overlay::{DocumentOverlay, FrontmatterAnalysis, OverlayState};
use crate::providers::{DocumentContext, ProviderRegistry};
use crate::request_timing::RequestTimings;
use crate::source_map::{PositionEncoding, SourceMap};
use crate::work_counters::{WorkCounts, work_counts};

const SHAPE: &str = "$schema:\n  guard: expression\n  ref: file\n";

/// A document whose `guard` reads `guard`, and whose last frontmatter line is
/// blank, for key completion.
fn document(guard: &str) -> String {
    format!("---\n$schema: ./shape.yaml\nprompt: hello\nguard: {guard}\nref: ./x.md\n\n---\n\nbody\n")
}

/// A document whose frontmatter is not YAML, `lead` lines below the fence.
fn malformed(lead: &str) -> String {
    format!("---\n{lead}$schema: ./shape.yaml\nprompt: hello\nguard: [\nref: ./x.md\n\n---\n\nbody\n")
}

struct Workspace {
    _dir: tempfile::TempDir,
    root: PathBuf,
    path: PathBuf,
    uri: Uri,
    graph: WorkspaceGraph,
    profile: ClientProfile,
    registry: ProviderRegistry,
}

impl Workspace {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::write(root.join("shape.yaml"), SHAPE).unwrap();
        let path = root.join("doc.md");
        let uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        Self {
            _dir: dir,
            root,
            path,
            uri,
            graph: WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts()),
            profile: ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16),
            registry: ProviderRegistry::with_substrate(),
        }
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// One request's inputs, as the server holds them for an open buffer.
#[derive(Clone, Copy)]
struct Inputs<'a> {
    revision: i32,
    text: &'a str,
    config: &'a DmlsConfig,
    resolution: &'a DocumentResolution,
}

/// The inputs of a request at `revision` on `text`.
fn inputs<'a>(revision: i32, text: &'a str, config: &'a DmlsConfig, resolution: &'a DocumentResolution) -> Inputs<'a> {
    Inputs { revision, text, config, resolution }
}

/// What a request answered, in comparable form.
#[derive(Debug, PartialEq)]
struct Answers {
    diagnostics: Vec<String>,
    hover: String,
    completion: Vec<String>,
}

impl Answers {
    fn mention(&self, needle: &str) -> bool {
        self.diagnostics.iter().any(|diagnostic| diagnostic.contains(needle))
    }
}

/// The overlay a request gets from `state`, after the preparation the server
/// does before building one.
fn overlay(state: &OverlayState, workspace: &Workspace, inputs: Inputs<'_>) -> DocumentOverlay {
    let roots = [workspace.root.clone()];
    state.prepare_for_document(inputs.text, &workspace.path, &roots, inputs.resolution);
    state
        .for_document_timed(
            &workspace.uri,
            Some(inputs.revision),
            inputs.text,
            &workspace.path,
            inputs.config,
            &roots,
            inputs.resolution,
            &RequestTimings::untimed(),
        )
        .expect("frontmatter overlay")
}

/// Diagnostics, then a hover on `guard`'s value, then key completion on the
/// blank frontmatter line, on `overlay`, with the frontmatter typing work they
/// did. Asserts that they did no I/O, parsing, preparation, or discovery.
fn answer(workspace: &Workspace, overlay: &DocumentOverlay, inputs: Inputs<'_>) -> (Answers, WorkCounts) {
    let text = inputs.text;
    let source_map = SourceMap::new(workspace.uri.clone(), inputs.revision, PositionEncoding::Utf16, Arc::from(text));
    let ctx = DocumentContext {
        uri: &workspace.uri,
        path: &workspace.path,
        text,
        source_map: &source_map,
        graph: &workspace.graph,
        doc_id: None,
        config: inputs.config,
        profile: &workspace.profile,
        overlay: Some(overlay),
        resolution: inputs.resolution,
    };
    let hover_at = text.find("guard: ").expect("guard") + "guard: ".len() + 1;
    let complete_at = text.find("\n\n---").expect("blank line") + 1;

    let contexts = crate::context::context_build_count();
    let darkmatter = darkmatter::work_counters::snapshot();
    #[cfg(feature = "effects-instrumentation")]
    let effects = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());
    let before = work_counts();
    let answers = Answers {
        diagnostics: workspace
            .registry
            .diagnostics(&ctx)
            .iter()
            .map(|diagnostic| format!("{:?} {:?} {:?} {}", diagnostic.range, diagnostic.severity, diagnostic.code, diagnostic.message))
            .collect(),
        hover: format!("{:?}", workspace.registry.hover(&ctx, hover_at)),
        completion: workspace.registry.completion(&ctx, complete_at).into_iter().map(|item| item.label).collect(),
    };
    let work = work_counts().since(before);
    assert_eq!(
        (work.reads, work.hashes, work.parses, work.enumerations, work.registry_scans),
        (0, 0, 0, 0, 0),
        "requests read, hash, parse, and scan nothing: {work:?}"
    );
    assert_eq!(crate::context::context_build_count(), contexts, "requests discover no context");
    let prepared = darkmatter::work_counters::snapshot().since(&darkmatter);
    assert_eq!(
        (prepared.get(Work::TriggerDiscovery), prepared.get(Work::EffectivePreparation)),
        (0, 0),
        "requests prepare no schema and discover no trigger"
    );
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(
        (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
        effects,
        "requests build no effect engine and make no network attempt"
    );
    (answers, work)
}

/// One request on `state`: its snapshot, its overlay, its answers, and its
/// typing work.
fn serve(
    state: &OverlayState,
    workspace: &Workspace,
    inputs: Inputs<'_>,
) -> (Arc<FrontmatterAnalysis>, DocumentOverlay, Answers, WorkCounts) {
    let overlay = overlay(state, workspace, inputs);
    let (answers, work) = answer(workspace, &overlay, inputs);
    (Arc::clone(&overlay.analysis), overlay, answers, work)
}

/// What a state that saw only `history` answers for its last input.
fn oracle(workspace: &Workspace, history: &[Inputs<'_>]) -> Answers {
    let state = OverlayState::default();
    let (last, earlier) = history.split_last().expect("an input");
    for inputs in earlier {
        overlay(&state, workspace, *inputs);
    }
    serve(&state, workspace, *last).2
}

/// Asserts that a request started a new snapshot: typed the document afresh
/// and answered as a state with no earlier inputs would.
fn assert_fresh(label: &str, work: WorkCounts, answers: &Answers, expected: &Answers) {
    assert_eq!((work.typing_walks, work.root_assemblies), (1, 1), "{label} starts a new snapshot: {work:?}");
    assert!(work.scalar_classifications > 0, "{label} classifies afresh: {work:?}");
    assert_eq!(answers, expected, "{label} answers from its own inputs");
}

#[test]
fn every_input_change_starts_a_snapshot_that_answers_from_its_own_inputs() {
    let workspace = Workspace::new();
    let state = OverlayState::default();
    let config = DmlsConfig::default();
    let mut strict = DmlsConfig::default();
    strict.schema.strict = !config.schema.strict;
    let in_repository = crate::context::test_support::resolution_in_repository(&workspace.root, &workspace.path);
    let plain_context = crate::context::test_support::resolution_in(&workspace.root, &workspace.path);
    let first_text = document("nope == 1");
    let second_text = document("other == 2");
    let (broken_text, shifted_text, fixed_text) = (malformed(""), malformed("# shifted\n"), document("third == 3"));
    workspace.write("doc.md", &first_text);
    let at = |revision: i32, text| inputs(revision, text, &config, &in_repository);
    let (first_text, second_text) = (first_text.as_str(), second_text.as_str());

    // A fresh overlay wrapper reuses the snapshot: no typing work at all.
    let (first, _, first_answers, work) = serve(&state, &workspace, at(1, first_text));
    assert_fresh("the first request", work, &first_answers, &oracle(&workspace, &[at(1, first_text)]));
    assert!(first_answers.mention("nope"), "{first_answers:#?}");
    let (reused, _, answers, work) = serve(&state, &workspace, at(1, first_text));
    assert!(Arc::ptr_eq(&first, &reused), "a fresh overlay shares the snapshot");
    assert_eq!(
        (work.typing_walks, work.scalar_classifications, work.root_assemblies, work.merged_shapes, work.item_syntheses),
        (0, 0, 0, 0, 0),
        "{work:?}"
    );
    assert_eq!(answers, first_answers);
    drop(reused);

    // The same text at a new revision, then an edit.
    let (revised, _, answers, work) = serve(&state, &workspace, at(2, first_text));
    assert_fresh("a new revision", work, &answers, &first_answers);
    assert_eq!(Arc::strong_count(&first), 1, "the cache keeps no replaced snapshot");
    let (edited, _, edited_answers, work) = serve(&state, &workspace, at(3, second_text));
    assert_fresh("an edit", work, &edited_answers, &oracle(&workspace, &[at(3, second_text)]));
    assert!(edited_answers.mention("other") && !edited_answers.mention("nope"), "{edited_answers:#?}");
    assert_eq!(Arc::strong_count(&revised), 1);

    // The imported schema changes under an unchanged buffer: `guard` is no
    // longer an expression, so its undeclared name is no longer reported.
    workspace.write("shape.yaml", "$schema:\n  guard: string\n  ref: file\n");
    let (reimported, _, answers, work) = serve(&state, &workspace, at(3, second_text));
    assert_fresh("an imported-schema change", work, &answers, &oracle(&workspace, &[at(3, second_text)]));
    assert!(!answers.mention("other"), "{answers:#?}");
    workspace.write("shape.yaml", SHAPE);
    let (restored, _, answers, work) = serve(&state, &workspace, at(3, second_text));
    assert_fresh("a restored import", work, &answers, &edited_answers);
    assert!(!Arc::ptr_eq(&edited, &restored), "a replaced snapshot never becomes current again");
    assert_eq!(Arc::strong_count(&reimported), 1);

    // Configuration, trigger registry, and captured context.
    let configured = Inputs { config: &strict, ..at(3, second_text) };
    let (_, _, answers, work) = serve(&state, &workspace, configured);
    assert_fresh("a configuration change", work, &answers, &oracle(&workspace, &[configured]));
    workspace.write("schemas/prompt.yaml", "$schema:\n  model: string(required)\n");
    workspace.write(
        "schemas/prompt.trigger.yaml",
        "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n",
    );
    state.invalidate_trigger_inputs(&[workspace.root.join("schemas/prompt.trigger.yaml")]);
    let (_, _, triggered, work) = serve(&state, &workspace, configured);
    assert_fresh("a trigger-registry change", work, &triggered, &oracle(&workspace, &[configured]));
    assert!(triggered.mention("model"), "the trigger's required property is reported: {triggered:#?}");
    assert!(triggered.completion.iter().any(|label| label == "model"), "{:?}", triggered.completion);
    let recontexted = Inputs { resolution: &plain_context, ..configured };
    let (_, _, answers, work) = serve(&state, &workspace, recontexted);
    assert_fresh("a context change", work, &answers, &oracle(&workspace, &[recontexted]));

    // Schema assembly fails, then is repaired.
    workspace.write("shape.yaml", "$schema:\n  guard: [\n");
    let (_, _, failed, work) = serve(&state, &workspace, configured);
    // Diagnostics report the failure and type no expression, so only the
    // hover classifies; the snapshot is still new.
    assert_eq!((work.typing_walks, work.root_assemblies, work.scalar_classifications), (0, 1, 1), "{work:?}");
    assert_eq!(failed, oracle(&workspace, &[configured]));
    assert_ne!(failed, triggered, "the failure is reported");
    workspace.write("shape.yaml", SHAPE);
    let (repaired, repaired_overlay, answers, work) = serve(&state, &workspace, configured);
    assert_fresh("a repaired schema", work, &answers, &triggered);

    // Malformed YAML keeps the last-good tree; each malformed buffer still
    // answers from its own text, and the repair from its own.
    let good = configured;
    let broken = Inputs { revision: 4, text: &broken_text, ..configured };
    let (stale, _, answers, work) = serve(&state, &workspace, broken);
    assert_fresh("a malformed buffer", work, &answers, &oracle(&workspace, &[good, broken]));
    assert!(Arc::ptr_eq(stale.ast().unwrap(), repaired.ast().unwrap()), "over the last-good tree");
    let shifted = Inputs { revision: 5, text: &shifted_text, ..configured };
    let (_, _, answers, work) = serve(&state, &workspace, shifted);
    assert_fresh("a second malformed buffer", work, &answers, &oracle(&workspace, &[good, shifted]));
    let fixed = Inputs { revision: 6, text: &fixed_text, ..configured };
    let (current, _, fixed_answers, work) = serve(&state, &workspace, fixed);
    assert_fresh("a repaired buffer", work, &fixed_answers, &oracle(&workspace, &[fixed]));
    assert!(fixed_answers.mention("third") && !fixed_answers.mention("other"), "{fixed_answers:#?}");

    // A request still holding the replaced snapshot finishes on it, and the
    // current snapshot is untouched by it.
    let (old_answers, _) = answer(&workspace, &repaired_overlay, configured);
    assert_eq!(old_answers, triggered, "the retained snapshot answers for its own buffer");
    drop(repaired_overlay);
    let (again, _, answers, work) = serve(&state, &workspace, fixed);
    assert!(Arc::ptr_eq(&current, &again));
    assert_eq!((work.typing_walks, work.scalar_classifications), (0, 0), "{work:?}");
    assert_eq!(answers, fixed_answers);
    drop(again);
    for (label, retired) in [("repaired", &repaired), ("stale", &stale)] {
        assert_eq!(Arc::strong_count(retired), 1, "the cache keeps no {label} history");
    }

    // Close drops the cache's owner; reopening starts over.
    assert_eq!(Arc::strong_count(&current), 2, "the cache owns the current snapshot");
    state.forget(&workspace.uri);
    assert_eq!(Arc::strong_count(&current), 1, "closing drops the cache's snapshot");
    let (reopened, _, answers, work) = serve(&state, &workspace, fixed);
    assert!(!Arc::ptr_eq(&current, &reopened));
    assert_fresh("a reopened buffer", work, &answers, &fixed_answers);

    assert_eq!(std::fs::read_to_string(&workspace.path).unwrap(), first_text, "no request edits the document");
}

/// An array whose items are one of two objects; `loud` decides whether the
/// second one also accepts a `say` item.
fn items_schema(second: &str) -> String {
    format!("$schema:\n  steps:\n    - \"{{ say: string(required) }}[]\"\n    - \"{{ {second} }}[]\"\n")
}

/// A hover on the `say` key of the first item, on a fresh overlay from
/// `state`, with the frontmatter typing work and the alternative matchers it
/// prepared.
fn item_hover(state: &OverlayState, workspace: &Workspace, inputs: Inputs<'_>) -> (String, WorkCounts, u64) {
    let overlay = overlay(state, workspace, inputs);
    let source_map = SourceMap::new(workspace.uri.clone(), inputs.revision, PositionEncoding::Utf16, Arc::from(inputs.text));
    let ctx = DocumentContext {
        uri: &workspace.uri,
        path: &workspace.path,
        text: inputs.text,
        source_map: &source_map,
        graph: &workspace.graph,
        doc_id: None,
        config: inputs.config,
        profile: &workspace.profile,
        overlay: Some(&overlay),
        resolution: inputs.resolution,
    };
    let at = inputs.text.find("say: hi").expect("an item") + 1;
    let darkmatter = darkmatter::work_counters::snapshot();
    let before = work_counts();
    let hover = format!("{:?}", workspace.registry.hover(&ctx, at));
    let prepared = darkmatter::work_counters::snapshot().since(&darkmatter).get(Work::AlternativeMatcherPrepare);
    (hover, work_counts().since(before), prepared)
}

/// A new snapshot reuses the synthesized item definition and the prepared
/// alternative matchers of an earlier one whose schema content is the same,
/// so a hover after an edit synthesizes and prepares nothing, yet answers as
/// a state that never saw the earlier buffer. When one alternative changes,
/// the item definition and that alternative's matcher are built afresh, and
/// the hover follows the new schema.
#[test]
fn item_definitions_and_matchers_are_shared_only_between_equal_schemas() {
    let workspace = Workspace::new();
    workspace.write("items.yaml", &items_schema("ask: number(required)"));
    let state = OverlayState::default();
    let config = DmlsConfig::default();
    let resolution = crate::context::test_support::resolution_in_repository(&workspace.root, &workspace.path);
    let first = "---\n$schema: ./items.yaml\nsteps:\n  - say: hi\n---\n\nbody\n";
    let edited = "---\n$schema: ./items.yaml\ntitle: edited\nsteps:\n  - say: hi\n---\n\nbody\n";
    let at = |revision: i32, text| inputs(revision, text, &config, &resolution);
    let oracle = |inputs: Inputs<'_>| item_hover(&OverlayState::default(), &workspace, inputs).0;

    let (unique, work, prepared) = item_hover(&state, &workspace, at(1, first));
    assert!(unique.contains("Item `steps[0]` matches alternative 1 of 2:"), "{unique}");
    assert_eq!((work.item_syntheses, prepared), (1, 2), "the first hover builds both: {work:?}");
    assert_eq!(unique, oracle(at(1, first)));

    let (after_edit, work, prepared) = item_hover(&state, &workspace, at(2, edited));
    assert_eq!(work.root_assemblies, 1, "the edit starts a new snapshot: {work:?}");
    assert_eq!((work.item_syntheses, prepared), (0, 0), "equal content is reused: {work:?}");
    let contents = |hover: &str| hover.split(", range:").next().unwrap_or_default().to_string();
    assert_eq!(contents(&after_edit), contents(&unique), "only the range moved with the edit");
    assert_eq!(after_edit, oracle(at(2, edited)));

    workspace.write("items.yaml", &items_schema("say: string(required), loud: boolean"));
    let (overlapping, work, prepared) = item_hover(&state, &workspace, at(2, edited));
    // The first alternative is unchanged, so only the second is prepared.
    assert_eq!((work.item_syntheses, prepared), (1, 1), "a changed alternative is built afresh: {work:?}");
    assert!(overlapping.contains("Item `steps[0]`: matches 2 of the 2 alternatives:"), "{overlapping}");
    assert_eq!(overlapping, oracle(at(2, edited)));
}
