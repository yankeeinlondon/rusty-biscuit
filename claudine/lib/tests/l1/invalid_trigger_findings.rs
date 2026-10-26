//! Claudine's preliminary schema checks carry the schema triggers discovery
//! skipped as invalid, through every outcome, into the invocation's
//! [`TriggerFindings`] record — never into a `CompositionError`.
//!
//! The incident: a prompt with no `$schema` beside an unfinished
//! `schemas/memory.yaml` (unsupported `name`/`description` keys, null
//! `match:`). Each fixture puts that trigger in the fixture home's
//! `~/schemas` root and builds its context from a `RequestSnapshot`, so
//! neither the checkout nor the process environment is consulted.

use std::path::{Path, PathBuf};

use claudine::composition::{
    CompositionError, TriggerFindings, build_schema_status_report, drop_invalid_optionals,
    pre_validate_schema, resolve_composition_source_in_context,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::TriggerSource;

const MEMORY_TRIGGER: &str = "kind: schema-trigger\nname: Memory Trigger\ndescription: TODO\nmatch:\n";

/// A valid trigger: a document setting `gate` must carry a numeric `count`.
const GATE_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  gate: string(required)\n$schema:\n  count: number(required)\n";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    /// The incident trigger at `~/schemas/memory.yaml`.
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("docs")).expect("docs dir");
        let fixture = Self { _dir: dir, root };
        fixture.write_trigger("memory.yaml", MEMORY_TRIGGER);
        fixture
    }

    fn write_trigger(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join("home/schemas").join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("schema root");
        std::fs::write(&path, text).expect("write trigger");
        path
    }

    fn trigger(&self, name: &str) -> TriggerSource {
        TriggerSource::File(self.root.join("home/schemas").join(name))
    }

    fn snapshot(&self) -> RequestSnapshot {
        RequestSnapshot::new(self.root.join("docs")).with_home(Some(self.root.join("home")))
    }

    fn document(&self, name: &str, frontmatter: &str) -> PathBuf {
        let path = self.root.join("docs").join(name);
        std::fs::write(&path, format!("---\n{frontmatter}\n---\nbody\n")).expect("write document");
        path
    }

    fn source(&self, document: &Path) -> claudine::composition::ResolvedCompositionSource {
        let context = build_resolution_context(&self.snapshot()).expect("fixture context");
        resolve_composition_source_in_context(document.to_str().expect("UTF-8 path"), &context)
            .expect("the document resolves")
    }

    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    fn verdict(
        &self,
        document: &Path,
        findings: &TriggerFindings,
    ) -> Result<claudine::composition::PreValidatedSchema, CompositionError> {
        let context = build_resolution_context(&self.snapshot()).expect("fixture context");
        pre_validate_schema(&self.source(document), None, None, &context, findings)
    }
}

fn sources(findings: &TriggerFindings) -> Vec<TriggerSource> {
    findings.warnings().into_iter().map(|warning| warning.source).collect()
}

#[test]
fn a_check_with_no_effective_schema_records_the_incident_trigger() {
    let fixture = Fixture::new();
    let document = fixture.document("merge-conflicts.md", "title: Merge");
    let findings = TriggerFindings::default();

    fixture.verdict(&document, &findings).expect("the prompt passes its checks");

    let [warning] = &findings.warnings()[..] else {
        panic!("one skipped trigger expected: {:?}", findings.warnings());
    };
    assert_eq!(warning.source, fixture.trigger("memory.yaml"));
    assert!(warning.message.starts_with("ignoring invalid schema trigger"), "{}", warning.message);
    assert!(warning.message.contains("found unknown keys: name, description"), "{}", warning.message);
    assert!(!warning.message.contains("merge-conflicts.md"), "never blames the prompt: {}", warning.message);
}

#[test]
fn a_prompt_rejected_by_a_valid_trigger_keeps_the_finding() {
    let fixture = Fixture::new();
    fixture.write_trigger("a-gate.yaml", GATE_TRIGGER);
    let missing = fixture.document("missing.md", "gate: yes");
    let invalid = fixture.document("invalid.md", "gate: yes\ncount: many");
    let findings = TriggerFindings::default();

    let error = fixture.verdict(&missing, &findings).expect_err("`count` is required");
    assert!(matches!(error, CompositionError::MissingProperties { .. }), "{error:?}");
    let error = fixture.verdict(&invalid, &findings).expect_err("`count` must be a number");
    assert!(matches!(error, CompositionError::SchemaValidation { .. }), "{error:?}");

    assert_eq!(sources(&findings), [fixture.trigger("memory.yaml")], "recorded once, before either rejection");
}

#[test]
fn an_authored_schema_that_cannot_load_is_still_an_error_and_keeps_the_finding() {
    let fixture = Fixture::new();
    let document = fixture.document("authored.md", "$schema: ./missing.yaml");
    let findings = TriggerFindings::default();

    let error = fixture.verdict(&document, &findings).expect_err("the authored schema cannot load");

    assert!(matches!(error, CompositionError::SchemaLoad { .. }), "{error:?}");
    assert!(!error.to_string().contains("memory.yaml"), "the error is about the authored reference: {error}");
    assert_eq!(sources(&findings), [fixture.trigger("memory.yaml")]);
}

#[test]
fn an_authored_schema_naming_the_skipped_trigger_still_fails() {
    let fixture = Fixture::new();
    let trigger = fixture.root.join("home/schemas/memory.yaml");
    let document = fixture.document(
        "names-trigger.md",
        &format!("$schema: {}", biscuit_file::to_portable_string(&trigger)),
    );

    let error = fixture.verdict(&document, &TriggerFindings::default()).expect_err("a trigger file is no document schema");

    assert!(matches!(error, CompositionError::SchemaLoad { .. }), "{error:?}");
}

/// Every preliminary check and reload runs discovery again; each trigger is
/// recorded once, in discovery order, and delivered once. A new invocation's
/// record reports it again.
#[test]
fn repeated_checks_record_each_trigger_once_in_discovery_order() {
    let fixture = Fixture::new();
    fixture.write_trigger("a-unfinished.yaml", "kind: schema-trigger\nmatch: 42\n");
    let document = fixture.document("repeat.md", "$schema:\n  title: string\ntitle: Repeat");
    let context = build_resolution_context(&fixture.snapshot()).expect("fixture context");
    let findings = TriggerFindings::default();

    for _ in 0..2 {
        fixture.verdict(&document, &findings).expect("the prompt passes its checks");
    }
    let _ = drop_invalid_optionals(fixture.source(&document), None, None, &context, &findings);
    build_schema_status_report(&fixture.source(&document), None, None, &context, &findings)
        .expect("the status report loads")
        .expect("the document has a schema");

    let expected = [fixture.trigger("a-unfinished.yaml"), fixture.trigger("memory.yaml")];
    assert_eq!(sources(&findings), expected, "once each, in filename order");
    assert_eq!(findings.take_undelivered().len(), 2);
    fixture.verdict(&document, &findings).expect("the prompt passes its checks");
    assert!(findings.take_undelivered().is_empty(), "never delivered twice");

    let next_invocation = TriggerFindings::default();
    fixture.verdict(&document, &next_invocation).expect("the prompt passes its checks");
    assert_eq!(next_invocation.take_undelivered().len(), 2, "a new invocation reports again");
}

/// The trigger a composition report warns about and the one a preliminary
/// check found are recognized as the same trigger.
#[test]
fn a_composition_warning_and_a_check_finding_are_one_trigger() {
    let fixture = Fixture::new();
    let document = fixture.document("composed.md", "title: Composed");
    let findings = TriggerFindings::default();
    fixture.verdict(&document, &findings).expect("the prompt passes its checks");

    let request = ComposeRequest::prepare(
        ComposeOptions::new().with_source_file(&document).with_trigger_schemas(true),
        &fixture.snapshot(),
    )
    .expect("the request prepares");
    let (_, report) = Markdown::try_from(document.as_path())
        .expect("the document parses")
        .compose_with_warnings(&request)
        .expect("the prompt composes");
    findings.record_compose_warnings(&report.warnings);

    assert_eq!(sources(&findings), [fixture.trigger("memory.yaml")]);
}
