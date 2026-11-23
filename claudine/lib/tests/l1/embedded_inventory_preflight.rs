//! The in-process launch entry points (inline, direct, and sequence
//! preflight) judge documents with the inventory compiled into this build:
//! the fixture home is empty (no installed registration, no `~/schemas`) and
//! the launch directory is outside any repository, so no schema file on disk
//! can supply a Claudine type.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::composition::{
    CompositionError, CompositionMode, PrepareOptions, TriggerFindings, build_preflight_graph, pre_validate_schema_for_mode, load_yaml_document, prepare_direct_with_schema,
    prepare_inline_with_schema, resolve_composition_source_in_context, resolve_sequence_plan,
};
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};

/// `{dir}/home` (empty) and `{dir}/work`, the launch directory, in a
/// temporary folder that is no repository.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        Self { _dir: dir, root }
    }

    fn context(&self) -> FileResolutionContext {
        let snapshot =
            RequestSnapshot::new(self.root.join("work")).with_home(Some(self.root.join("home"))).with_env(HashMap::new());
        build_resolution_context(&snapshot).unwrap()
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join("work").join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    #[allow(clippy::result_large_err, reason = "the entry point's own result, returned unchanged")]
    fn prepare(&self, name: &str, frontmatter: &str, inline: bool) -> Result<(), CompositionError> {
        let path = self.write(name, &format!("---\n{frontmatter}\n---\nbody\n"));
        let context = self.context();
        let source = resolve_composition_source_in_context(path.to_str().unwrap(), &context)?;
        let options = PrepareOptions::new(context);
        if inline { prepare_inline_with_schema(&source, options) } else { prepare_direct_with_schema(&source, options) }.map(drop)
    }

    #[allow(clippy::result_large_err, reason = "the entry point's own result, returned unchanged")]
    fn preflight(&self, path: &Path) -> Result<(), CompositionError> {
        let context = self.context();
        let source = claudine::composition::ResolvedCompositionSource {
            original_ref: path.display().to_string(),
            resolved_path: path.to_path_buf(),
            original_text: std::fs::read_to_string(path).unwrap(),
            markdown: load_yaml_document(path)?,
        };
        let plan = resolve_sequence_plan(&source, &context)?.expect("a sequence");
        build_preflight_graph(&plan, &source, &context).map(drop)
    }
}

fn schema_refusal(outcome: Result<(), CompositionError>, what: &str) -> String {
    match outcome {
        Err(CompositionError::SchemaValidation { message, .. }) => message,
        other => panic!("{what}: expected a schema refusal, got {other:?}"),
    }
}

#[test]
fn inline_direct_and_sequence_preflight_use_the_embedded_inventory() {
    let fixture = Fixture::new();
    assert!(std::fs::read_dir(fixture.root.join("home")).unwrap().next().is_none(), "the home is empty");

    fixture.prepare("inline.md", "prompt: hi", true).expect("an inline document prepares");
    // `mode` is typed only by Claudine's shared attributes (`shared.yaml`).
    let inline = schema_refusal(fixture.prepare("inline-bad.md", "prompt: hi\nmode: sideways", true), "inline");
    assert!(inline.contains("mode"), "{inline}");

    fixture.prepare("direct.md", "start: [stop]", false).expect("a direct document prepares");
    let direct = schema_refusal(fixture.prepare("direct-bad.md", "start: [stop]\nmode: sideways", false), "direct");
    assert!(direct.contains("mode"), "{direct}");

    let formal = fixture.write("run.yaml", "kind: sequence\ndescription: steps\nsequence:\n  - one\n");
    fixture.preflight(&formal).expect("a formal sequence passes preflight");
    let bad = fixture.write("bad.yaml", "kind: sequence\ndescription: [not, text]\nsequence:\n  - one\n");
    let refusal = schema_refusal(fixture.preflight(&bad), "sequence");
    assert!(refusal.contains("`kind: sequence`") && refusal.contains("/description"), "{refusal}");
}

/// The in-process inline launch judges with the registered
/// `inline-composition.yaml`, whose payload is a root union of a `prompt`
/// arm and a `sections` arm: a document writing both, and a
/// `kind: inline-compose` document writing neither, match no arm and are
/// refused before anything runs, while the `prompt` arm prepares and the
/// `sections` arm passes the schema.
#[test]
fn the_in_process_inline_launch_enforces_the_prompt_or_sections_union() {
    let fixture = Fixture::new();
    assert!(std::fs::read_dir(fixture.root.join("home")).unwrap().next().is_none(), "the home is empty");

    fixture.prepare("prompt.md", "prompt: Fix the bug.", true).expect("the prompt arm prepares");
    // A `sections` document has no prompt to launch with, but its schema
    // verdict passes: only the launch's own prompt check refuses it.
    if let Err(error) = fixture.prepare("sections.md", "sections:\n    Goal: Fix the bug.", true) {
        assert!(!matches!(error, CompositionError::SchemaValidation { .. }), "the sections arm passes the schema: {error:?}");
    }

    let both = schema_refusal(
        fixture.prepare("both.md", "prompt: Fix the bug.\nsections:\n    Goal: Fix the bug.", true),
        "prompt and sections",
    );
    assert!(both.contains("sections") || both.contains("prompt"), "the refusal names the union's keys: {both}");

    // The inline launch refuses a document with no prompt before judging
    // it; the direct launch judges the same document with the same
    // registered union, whose arms each require their key, so the verdict
    // reports the required key missing.
    let inline = fixture.prepare("neither.md", "kind: inline-compose\ntitle: x", true);
    assert!(inline.is_err(), "an inline launch with neither key is refused");
    match fixture.prepare("neither.md", "kind: inline-compose\ntitle: x", false) {
        Err(CompositionError::MissingProperties { missing, .. }) => {
            let names: Vec<&str> = missing.iter().map(|property| property.name.as_str()).collect();
            assert!(names.iter().all(|name| ["prompt", "sections"].contains(name)) && !names.is_empty(), "{names:?}");
        }
        other => panic!("kind: inline-compose with neither key: expected the union's required key, got {other:?}"),
    }
}

/// A document `$schema` over the registered inline union: the launch phase
/// projects the document into every arm, so a deferred required property
/// stays deferred while the union still refuses an invalid Claudine control
/// (it once replaced the whole effective schema with the document's). Judged
/// by the launch verdict `inline-compose` renders.
#[test]
#[allow(clippy::result_large_err, reason = "the entry point's own result, returned unchanged")]
fn a_document_schema_over_the_inline_union_keeps_the_union_at_launch() {
    let fixture = Fixture::new();
    let context = fixture.context();
    let verdict = |name: &str, frontmatter: &str| {
        let path = fixture.write(name, &format!("---\n{frontmatter}\n---\nbody\n"));
        let source = resolve_composition_source_in_context(path.to_str().unwrap(), &context).unwrap();
        pre_validate_schema_for_mode(
            &source,
            None,
            None,
            &context,
            CompositionMode::InlineFrontmatterPrompt,
            &TriggerFindings::default(),
        )
    };
    let deferred = "$schema:\n  topic: string(required)\nprompt: Write about the topic";

    let accepted = verdict("deferred.md", deferred).expect("a required non-eager property waits for completion");
    assert!(accepted.dropped_optionals.is_empty(), "nothing is dropped");
    let error = verdict("invalid.md", &format!("{deferred}\nmodel: {{x: 1}}")).expect_err("the union refuses the model");
    let CompositionError::SchemaValidation { problems, .. } = &error else {
        panic!("a schema verdict, got {error:?}");
    };
    assert_eq!(problems.iter().map(String::as_str).collect::<Vec<_>>(), ["/model"], "{error}");
}
