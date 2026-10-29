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
    CompositionError, DroppedOptionalSource, DroppedOptionalStage, TriggerFindings, build_schema_status_report, drop_invalid_optionals,
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

impl Fixture {
    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    fn verdict_with(
        &self,
        document: &Path,
        overrides: Option<&serde_json::Value>,
        launch_dir: Option<&Path>,
        findings: &TriggerFindings,
    ) -> Result<claudine::composition::PreValidatedSchema, CompositionError> {
        let context = build_resolution_context(&self.snapshot()).expect("fixture context");
        pre_validate_schema(&self.source(document), overrides, launch_dir, &context, findings)
    }
}

/// Discovery and effective-preparation counts of the work `body` does.
#[cfg(feature = "work-counters")]
fn counted<T>(body: impl FnOnce() -> T) -> (T, u64, u64) {
    use darkmatter::work_counters::{Work, snapshot};
    let before = snapshot();
    let value = body();
    let delta = snapshot().since(&before);
    (value, delta.get(Work::TriggerDiscovery), delta.get(Work::EffectivePreparation))
}

/// A trigger selected by a string or numeric `count`, requiring `ticket`. The
/// optional numeric `count` is declared by the document's own `$schema`:
/// cleanup drops only values the authored schema declares optional.
const COUNT_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  group:\n    - count: string(required)\n    - count: number(required)\n$schema:\n  ticket: string(required)\n";

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

#[cfg(feature = "work-counters")]
#[test]
fn a_matching_disk_trigger_is_discovered_once_and_prepared_twice() {
    let fixture = Fixture::new();
    fixture.write_trigger("a-gate.yaml", GATE_TRIGGER);
    let document = fixture.document("counted.md", "gate: yes\ncount: 3");
    let context = build_resolution_context(&fixture.snapshot()).expect("fixture context");
    let source = fixture.source(&document);
    let findings = TriggerFindings::default();

    for _ in 0..2 {
        let (verdict, discoveries, preparations) =
            counted(|| pre_validate_schema(&source, None, None, &context, &findings));
        verdict.expect("a valid `count` satisfies the trigger");
        assert_eq!((discoveries, preparations), (1, 2));
    }
}

#[test]
fn cleanup_that_removes_the_selecting_value_ends_the_trigger_for_the_verdict() {
    let fixture = Fixture::new();
    fixture.write_trigger("b-count.yaml", COUNT_TRIGGER);
    let valid = fixture.document("valid.md", "$schema:\n  count: number\ncount: 3");
    let wrong = fixture.document("wrong.md", "$schema:\n  count: number\ncount: many");
    let findings = TriggerFindings::default();

    let error = fixture.verdict(&valid, &findings).expect_err("`ticket` is required");
    assert!(matches!(error, CompositionError::MissingProperties { .. }), "{error:?}");

    #[cfg(feature = "work-counters")]
    let (verdict, discoveries, preparations) = counted(|| fixture.verdict(&wrong, &findings));
    #[cfg(not(feature = "work-counters"))]
    let verdict = fixture.verdict(&wrong, &findings);
    let pre = verdict.expect("the cleaned document no longer selects the trigger");
    #[cfg(feature = "work-counters")]
    assert_eq!((discoveries, preparations), (1, 2));

    assert!(!pre.source.markdown.frontmatter().as_map().contains_key("count"));
    let [dropped] = &pre.dropped_optionals[..] else {
        panic!("one drop expected: {:?}", pre.dropped_optionals);
    };
    assert_eq!(dropped.property, "count");
    assert_eq!(dropped.source, DroppedOptionalSource::Frontmatter);
    assert_eq!(dropped.stage, DroppedOptionalStage::PreValidation);
    assert!(!dropped.reason.is_empty());
}

#[test]
fn cleaned_overrides_rematch_and_surviving_caller_origins_stay_wired() {
    let fixture = Fixture::new();
    fixture.write_trigger("b-count.yaml", COUNT_TRIGGER);
    fixture.write_trigger(
        "c-input.yaml",
        "kind: schema-trigger\nmatch:\n  gate: string(required)\n$schema:\n  input: file(required;match(**/*spec*.md);eager)\n",
    );
    let document = fixture.document("overrides.md", "gate: yes\n$schema:\n  count: number");
    let launch_dir = fixture.root.join("launch");
    std::fs::create_dir_all(&launch_dir).expect("launch dir");
    std::fs::write(launch_dir.join("launch-spec.md"), "x\n").expect("launch file");
    let findings = TriggerFindings::default();

    let overrides = serde_json::json!({ "count": "many", "input": "launch-spec.md" });
    let pre = fixture
        .verdict_with(&document, Some(&overrides), Some(&launch_dir), &findings)
        .expect("the cleaned overrides no longer select the `ticket` trigger");
    assert_eq!(pre.set_overrides, Some(serde_json::json!({ "input": "launch-spec.md" })));
    let [dropped] = &pre.dropped_optionals[..] else {
        panic!("one drop expected: {:?}", pre.dropped_optionals);
    };
    assert_eq!(dropped.property, "count");
    assert_eq!(dropped.source, DroppedOptionalSource::Override);
    assert_eq!(dropped.stage, DroppedOptionalStage::PreValidation);

    let absent = serde_json::json!({ "count": "many", "input": "everywhere" });
    let error = fixture
        .verdict_with(&document, Some(&absent), Some(&launch_dir), &findings)
        .expect_err("an unmatched file reference stays a rejected `input`");
    assert!(
        matches!(&error, CompositionError::SchemaValidation { problems, .. } if problems == &["/input"]),
        "{error:?}"
    );
}

#[test]
fn an_unmatched_document_gets_no_effective_schema_and_is_returned_unchanged() {
    let fixture = Fixture::new();
    fixture.write_trigger("a-gate.yaml", GATE_TRIGGER);
    let document = fixture.document("plain.md", "title: Plain");
    let overrides = serde_json::json!({ "other": 1 });
    let original = fixture.source(&document);
    let findings = TriggerFindings::default();

    let pre = fixture
        .verdict_with(&document, Some(&overrides), None, &findings)
        .expect("nothing applies");

    assert_eq!(pre.source.markdown.frontmatter().as_map(), original.markdown.frontmatter().as_map());
    assert_eq!(pre.set_overrides, Some(overrides));
    assert!(pre.dropped_optionals.is_empty());
    assert_eq!(sources(&findings), [fixture.trigger("memory.yaml")], "the skipped trigger still warns");
}

#[test]
fn a_malformed_authored_schema_is_a_parse_error_beside_recorded_findings() {
    let fixture = Fixture::new();
    let document = fixture.document(
        "bad-grammar.md",
        "$schema:\n  spec: 'file(required, match(**/*spec*.md))'\nspec: x",
    );
    let findings = TriggerFindings::default();

    let error = fixture.verdict(&document, &findings).expect_err("the grammar is invalid");

    let CompositionError::SchemaParse { source_path, property, .. } = &error else {
        panic!("expected SchemaParse: {error:?}");
    };
    assert_eq!(source_path, &fixture.source(&document).resolved_path);
    assert_eq!(property.as_deref(), Some("spec"));
    assert_eq!(sources(&findings), [fixture.trigger("memory.yaml")], "recorded before the error");
}

#[test]
fn the_first_reason_for_a_trigger_survives_later_changes_to_its_file() {
    let fixture = Fixture::new();
    let document = fixture.document("reason.md", "title: Reason");
    let context = build_resolution_context(&fixture.snapshot()).expect("fixture context");
    let source = fixture.source(&document);
    let findings = TriggerFindings::default();
    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    let verdict = || pre_validate_schema(&source, None, None, &context, &findings);
    verdict().expect("passes");
    let first = findings.warnings()[0].message.clone();

    fixture.write_trigger("memory.yaml", "kind: schema-trigger\nmatch: 42\n");
    verdict().expect("passes");

    let [warning] = &findings.warnings()[..] else {
        panic!("one finding expected");
    };
    assert_eq!(warning.message, first);
    assert_eq!(findings.take_undelivered().len(), 1);
    verdict().expect("passes");
    assert!(findings.take_undelivered().is_empty());
}

/// Each call discovers afresh: creation, replacement, removal, and repair of
/// a trigger between synchronous calls sharing one source and context are all
/// seen.
#[test]
fn separate_calls_see_trigger_creation_replacement_removal_and_repair() {
    let fixture = Fixture::new();
    let document = fixture.document("fresh.md", "title: Fresh\ngate: yes");
    let context = build_resolution_context(&fixture.snapshot()).expect("fixture context");
    let source = fixture.source(&document);
    let findings = TriggerFindings::default();
    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    let verdict = || pre_validate_schema(&source, None, None, &context, &findings);
    let trigger = "need-ticket.yaml";

    verdict().expect("no trigger applies");

    fixture.write_trigger(trigger, "kind: schema-trigger\nmatch:\n  gate: string(required)\n$schema:\n  ticket: string(required)\n");
    let error = verdict().expect_err("`ticket` is required");
    assert!(matches!(error, CompositionError::MissingProperties { .. }), "{error:?}");

    fixture.write_trigger(trigger, "kind: schema-trigger\nmatch:\n  gate: string(required)\n$schema:\n  title: string(required)\n");
    verdict().expect("the replaced trigger is satisfied");

    std::fs::remove_file(fixture.root.join("home/schemas").join(trigger)).expect("remove");
    verdict().expect("the removed trigger no longer applies");

    fixture.write_trigger(trigger, "kind: schema-trigger\nmatch:\n");
    verdict().expect("a malformed trigger is skipped");
    assert_eq!(sources(&findings), [fixture.trigger("memory.yaml"), fixture.trigger(trigger)]);
    assert_eq!(findings.take_undelivered().len(), 2);

    fixture.write_trigger(trigger, "kind: schema-trigger\nmatch:\n  gate: string(required)\n$schema:\n  ticket: string(required)\n");
    let error = verdict().expect_err("the repaired trigger applies");
    assert!(matches!(error, CompositionError::MissingProperties { .. }), "{error:?}");
    assert!(findings.take_undelivered().is_empty(), "the earlier warning is not repeated");
}

/// A schema root file that is not UTF-8 fails the scan itself: nothing is
/// prepared, no partial findings are published, and standalone cleanup leaves
/// its input untouched. The next call recovers once the file is valid.
#[test]
fn a_scan_that_cannot_read_a_root_file_fails_once_and_recovers() {
    let fixture = Fixture::new();
    let broken = fixture.write_trigger("z-broken.yaml", "");
    std::fs::write(&broken, [0xff]).expect("write invalid UTF-8");
    let document = fixture.document("scan.md", "title: Scan\ngate: yes\ncount: 3");
    let context = build_resolution_context(&fixture.snapshot()).expect("fixture context");
    let source = fixture.source(&document);
    let findings = TriggerFindings::default();
    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    let verdict = || pre_validate_schema(&source, None, None, &context, &findings);

    #[cfg(feature = "work-counters")]
    let (failed, discoveries, preparations) = counted(verdict);
    #[cfg(not(feature = "work-counters"))]
    let failed = verdict();
    let error = failed.expect_err("the scan fails");
    assert!(matches!(error, CompositionError::SchemaLoad { .. }), "{error:?}");
    assert!(error.to_string().contains("z-broken.yaml"), "{error}");
    #[cfg(feature = "work-counters")]
    assert_eq!((discoveries, preparations), (1, 0));
    assert!(findings.warnings().is_empty(), "a failed scan publishes nothing");

    let overrides = serde_json::json!({ "count": "many" });
    let (cleaned, cleaned_overrides, dropped) =
        drop_invalid_optionals(source.clone(), Some(overrides.clone()), None, &context, &findings);
    assert_eq!(cleaned.markdown.frontmatter().as_map(), source.markdown.frontmatter().as_map());
    assert_eq!(cleaned_overrides, Some(overrides));
    assert!(dropped.is_empty());

    fixture.write_trigger("z-broken.yaml", GATE_TRIGGER);
    #[cfg(feature = "work-counters")]
    let (recovered, discoveries, preparations) = counted(verdict);
    #[cfg(not(feature = "work-counters"))]
    let recovered = verdict();
    recovered.expect("the repaired trigger's `count` is satisfied");
    #[cfg(feature = "work-counters")]
    assert_eq!((discoveries, preparations), (1, 2));
    assert_eq!(sources(&findings), [fixture.trigger("memory.yaml")]);
}
