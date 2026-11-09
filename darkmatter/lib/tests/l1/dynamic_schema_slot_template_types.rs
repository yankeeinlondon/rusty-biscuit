//! Template types through Darkmatter's real slot host: every read a
//! returned definition causes (its imports, their imports, the candidates
//! probed to select them) passes the session's grants before it happens, is
//! recorded as a dependency of the resolution, and keeps its refusal code.
//!
//! Each case writes its own schema files into a temp folder outside any
//! repository (so no automatic grant applies) and resolves through a
//! [`SlotSession`] whose grants the case states. A slot whose expression is
//! a string literal reads nothing itself, so every read a case observes is
//! one the returned definition caused.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::DataRuntimeStats;
use darkmatter::markdown::compose::expression::read_gate::ReadLocation;
use darkmatter::markdown::compose::report_diagnostics::template_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeSource, DataRuntime, ShellExpansionError};
use darkmatter::markdown::schemas::slots::grants::{
    Grant, GrantDecision, GrantScope, GrantTarget, SlotGrantDecision, SlotGrantRequest,
};
use darkmatter::markdown::schemas::slots::{SessionMode, SlotSession};
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};
use serde_json::Value;
use simplified_schema::SchemaError;

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const DENIED: &str = "dm.schema.slot_denied";

/// A canonical temp folder holding `types.yaml` (a `Person` type) and a
/// document whose inline `$schema` gives `owner` the template type
/// `{{ 'Person@./types.yaml' }}`.
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        write(&root.join("types.yaml"), "kind: schema\ntypes:\n    Person:\n        name: string(required)\n");
        write(
            &root.join("doc.md"),
            "---\n$schema:\n    owner: \"{{ 'Person@./types.yaml' }}\"\nowner:\n    name: Ann\n---\n\nBody.\n",
        );
        Self { _temp: temp, root }
    }

    /// A canonical temp folder holding `files`, with `doc.md` among them.
    fn with(files: &[(&str, &str)]) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        for (relative, text) in files {
            write(&root.join(relative), text);
        }
        Self { _temp: temp, root }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(&self.root).with_env(HashMap::new())
    }

    fn markdown(&self) -> Markdown {
        self.markdown_at("doc.md")
    }

    fn markdown_at(&self, relative: &str) -> Markdown {
        let document = self.path(relative);
        Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document))
    }

    /// The portable read-set entry of `relative`.
    fn entry(&self, relative: &str) -> String {
        biscuit_file::to_portable_string(&self.path(relative))
    }

    /// A [`DarkmatterSchemas`] resolving through `session`, as one command.
    fn schemas(&self, session: SlotSession) -> DarkmatterSchemas {
        DarkmatterSchemas::new(self.context()).with_slot_session(Arc::new(session))
    }

    /// A Dir grant on the whole fixture folder.
    fn folder_grant(&self) -> Grant {
        Grant { decision: GrantDecision::Allow, target: GrantTarget::Dir(self.root.clone()), scope: GrantScope::Once }
    }
}

/// `owner: {type}`, then `owner: {value}`, as a document.
fn owner_document(owner_type: &str, value: &str) -> String {
    format!("---\n$schema:\n    owner: {owner_type}\nowner: {value}\n---\n\nBody.\n")
}

/// `types.yaml` whose `Person` imports `Address` from `address.yaml`.
const TRANSITIVE_TYPES: &str =
    "kind: schema\ntypes:\n    Person:\n        name: string(required)\n        address: Address@./address.yaml\n";
const ADDRESS_TYPES: &str = "kind: schema\ntypes:\n    Address:\n        city: string(required)\n";
const TEMPLATE_PERSON: &str = "\"{{ 'Person@./types.yaml' }}\"";

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).expect("write");
}

fn grant(decision: GrantDecision, path: &Path) -> Grant {
    Grant { decision, target: GrantTarget::File(path.to_path_buf()), scope: GrantScope::Once }
}

fn owner_schema(schemas: &DarkmatterSchemas, markdown: &Markdown) -> Result<Value, SchemaError> {
    let effective = schemas.effective_for(markdown)?.expect("a schema");
    Ok(effective.json_schema["properties"]["owner"].clone())
}

fn codes(error: &SchemaError, document: &Path) -> Vec<String> {
    template_diagnostics(error, document)
        .expect("a template failure")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// An approval handler answering each slot request with `answer`, recording
/// how often and about what it was asked.
struct Recording {
    answer: fn(&SlotGrantRequest) -> SlotGrantDecision,
    calls: AtomicUsize,
    asked: Mutex<Vec<SlotGrantRequest>>,
}

impl Recording {
    fn new(answer: fn(&SlotGrantRequest) -> SlotGrantDecision) -> Arc<Self> {
        Arc::new(Self { answer, calls: AtomicUsize::new(0), asked: Mutex::new(Vec::new()) })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn asked(&self) -> Vec<SlotGrantRequest> {
        self.asked.lock().expect("asked").clone()
    }
}

impl ShellApprovalHandler for Recording {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::Deny)
    }

    fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.asked.lock().expect("asked").extend(requests.iter().cloned());
        Ok(requests.iter().map(self.answer).collect())
    }
}

/// The "this file, once" choice of `request`, or no answer.
fn allow_file_once(request: &SlotGrantRequest) -> SlotGrantDecision {
    request
        .choices
        .iter()
        .find(|grant| {
            grant.decision == GrantDecision::Allow
                && grant.scope == GrantScope::Once
                && matches!(grant.target, GrantTarget::File(_))
        })
        .map_or(SlotGrantDecision::Unanswered, SlotGrantDecision::from_grant)
}

#[cfg(feature = "work-counters")]
fn slot_runs() -> u64 {
    darkmatter::work_counters::snapshot().get(darkmatter::work_counters::Work::SlotResolution)
}

#[test]
fn a_granted_import_resolves_and_is_observed() {
    let fixture = Fixture::new();
    let data = DataRuntime::new();
    let session = SlotSession::new(data.clone()).with_run_grants([grant(GrantDecision::Allow, &fixture.path("types.yaml"))]);
    let schemas = DarkmatterSchemas::new(fixture.context()).with_slot_session(Arc::new(session));
    let owner = owner_schema(&schemas, &fixture.markdown()).expect("resolves");
    // The same schema the static import gives.
    write(&fixture.path("static.md"), "---\n$schema:\n    owner: Person@./types.yaml\nowner:\n    name: Ann\n---\n");
    let static_document = fixture.path("static.md");
    let static_markdown =
        Markdown::try_from(static_document.as_path()).expect("static").with_source(ComposeSource::File(static_document.clone()));
    let expected = owner_schema(&DarkmatterSchemas::new(fixture.context()), &static_markdown).expect("static resolves");
    assert_eq!(owner, expected);
    assert!(owner.to_string().contains(r#""required":["name"]"#), "{owner}");
    let entry = biscuit_file::to_portable_string(&fixture.path("types.yaml"));
    assert!(data.read_set().contains(&entry), "{:?}", data.read_set());
}

#[test]
fn an_ungranted_import_needs_approval() {
    let fixture = Fixture::new();
    let data = DataRuntime::new();
    let schemas = DarkmatterSchemas::new(fixture.context()).with_slot_session(Arc::new(SlotSession::new(data.clone())));
    let error = owner_schema(&schemas, &fixture.markdown()).expect_err("refused");
    assert_eq!(codes(&error, &fixture.path("doc.md")), vec![APPROVAL_REQUIRED.to_string()], "{error}");
    assert!(data.read_set().is_empty(), "nothing read: {:?}", data.read_set());
}

#[test]
fn a_denied_import_keeps_the_denied_code() {
    let fixture = Fixture::new();
    let session = SlotSession::new(DataRuntime::new()).with_run_grants([grant(GrantDecision::Deny, &fixture.path("types.yaml"))]);
    let schemas = DarkmatterSchemas::new(fixture.context()).with_slot_session(Arc::new(session));
    let error = owner_schema(&schemas, &fixture.markdown()).expect_err("denied");
    assert_eq!(codes(&error, &fixture.path("doc.md")), vec![DENIED.to_string()], "{error}");
}

#[test]
fn a_reference_check_leaves_an_ungranted_template_type_unresolved() {
    let fixture = Fixture::new();
    let session = SlotSession::new(DataRuntime::new()).with_mode(SessionMode::ReferenceCheck);
    let schemas = DarkmatterSchemas::new(fixture.context()).with_slot_session(Arc::new(session));
    let error = owner_schema(&schemas, &fixture.markdown()).expect_err("deferred");
    assert!(matches!(error, SchemaError::UnresolvedSlots(_)), "{error:?}");
}

// ---------------------------------------------------------------------------
// Reads and grants
// ---------------------------------------------------------------------------

/// The gate decides before anything is probed, read, or looked up in the
/// command's data memo, so a refused import leaves no trace of a read.
#[test]
fn a_refused_import_is_decided_before_anything_is_read() {
    let fixture = Fixture::new();
    let data = DataRuntime::new();
    let mark = data.read_mark();
    let schemas = fixture.schemas(SlotSession::new(data.clone()));
    let error = owner_schema(&schemas, &fixture.markdown()).expect_err("refused");
    assert_eq!(codes(&error, &fixture.path("doc.md")), vec![APPROVAL_REQUIRED.to_string()], "{error}");
    assert!(data.read_set().is_empty(), "{:?}", data.read_set());
    assert!(data.observations_since(mark).is_empty(), "nothing probed: {:?}", data.observations_since(mark));
    assert_eq!(data.stats(), DataRuntimeStats::default(), "the data memo was not consulted");
}

/// A transitive import is the returned definition's read too: granting the
/// first file does not grant what it imports.
#[test]
fn a_transitive_import_needs_its_own_grant() {
    let fixture = Fixture::with(&[
        ("types.yaml", TRANSITIVE_TYPES),
        ("address.yaml", ADDRESS_TYPES),
        ("doc.md", &owner_document(TEMPLATE_PERSON, "{ name: Ann }")),
    ]);
    let data = DataRuntime::new();
    let session = SlotSession::new(data.clone()).with_run_grants([grant(GrantDecision::Allow, &fixture.path("types.yaml"))]);
    let error = owner_schema(&fixture.schemas(session), &fixture.markdown()).expect_err("address.yaml is not granted");
    assert_eq!(codes(&error, &fixture.path("doc.md")), vec![APPROVAL_REQUIRED.to_string()], "{error}");
    assert!(!data.read_set().contains(&fixture.entry("address.yaml")), "{:?}", data.read_set());
}

#[test]
fn granting_every_transitive_import_resolves_and_observes_each() {
    let files = [("types.yaml", TRANSITIVE_TYPES), ("address.yaml", ADDRESS_TYPES)];
    let fixture = Fixture::with(&files);
    write(&fixture.path("doc.md"), &owner_document(TEMPLATE_PERSON, "{ name: Ann }"));
    let by_file = [grant(GrantDecision::Allow, &fixture.path("types.yaml")), grant(GrantDecision::Allow, &fixture.path("address.yaml"))];
    for grants in [by_file.to_vec(), vec![fixture.folder_grant()]] {
        let data = DataRuntime::new();
        let session = SlotSession::new(data.clone()).with_run_grants(grants.clone());
        let owner = owner_schema(&fixture.schemas(session), &fixture.markdown())
            .unwrap_or_else(|error| panic!("{grants:?}: {error}"));
        assert!(owner.to_string().contains("city"), "{owner}");
        let read = data.read_set();
        assert!(read.contains(&fixture.entry("types.yaml")) && read.contains(&fixture.entry("address.yaml")), "{read:?}");
    }
}

/// A reference check never asks: an import nothing grants defers the
/// template rather than prompting or reading.
#[test]
fn a_reference_check_never_asks_and_reads_nothing() {
    let fixture = Fixture::new();
    let data = DataRuntime::new();
    let handler = Recording::new(allow_file_once);
    let session = SlotSession::new(data.clone()).with_mode(SessionMode::ReferenceCheck).with_handler(handler.clone());
    let error = owner_schema(&fixture.schemas(session), &fixture.markdown()).expect_err("deferred");
    assert!(matches!(error, SchemaError::UnresolvedSlots(_)), "{error:?}");
    assert_eq!(handler.calls(), 0);
    assert!(data.read_set().is_empty(), "{:?}", data.read_set());
}

/// An import named by returned data is a computed target: the handler is
/// asked about it while the slot resolves, and its answer holds for the run.
#[test]
fn a_computed_import_is_asked_once_and_remembered() {
    let fixture = Fixture::new();
    let handler = Recording::new(allow_file_once);
    let data = DataRuntime::new();
    let schemas = fixture.schemas(SlotSession::new(data.clone()).with_handler(handler.clone()));
    owner_schema(&schemas, &fixture.markdown()).expect("the handler allowed it");
    let asked = handler.asked();
    assert_eq!(handler.calls(), 1, "{asked:?}");
    assert!(
        asked.iter().all(|request| request.location == ReadLocation::Local(fixture.path("types.yaml"))),
        "{asked:?}"
    );
    assert!(asked.iter().all(|request| request.function == "schema import"), "{asked:?}");
    assert!(data.read_set().contains(&fixture.entry("types.yaml")), "{:?}", data.read_set());
    let calls = handler.calls();
    owner_schema(&schemas, &fixture.markdown()).expect("still resolves");
    assert_eq!(handler.calls(), calls, "nothing more is asked");
}

/// A refused template-type arm fails the whole union: a read the user did
/// not allow can never fall back to another arm.
#[test]
fn a_refused_union_arm_fails_the_union() {
    for (decision, code) in [(None, APPROVAL_REQUIRED), (Some(GrantDecision::Deny), DENIED)] {
        let fixture = Fixture::new();
        write(&fixture.path("doc.md"), &owner_document("\"{{ 'Person@./types.yaml' }} | string\"", "Ann"));
        let session = SlotSession::new(DataRuntime::new())
            .with_run_grants(decision.map(|decision| grant(decision, &fixture.path("types.yaml"))));
        let error = owner_schema(&fixture.schemas(session), &fixture.markdown()).expect_err("no fallback to `string`");
        assert_eq!(codes(&error, &fixture.path("doc.md")), vec![code.to_string()], "{error}");
    }
}

/// A missing import fails as a missing file, not as a grant question, and
/// the probe that found it missing is a dependency.
#[test]
fn a_missing_import_fails_and_observes_its_probe() {
    let fixture = Fixture::with(&[("doc.md", &owner_document("\"{{ 'Person@./missing.yaml' }}\"", "{ name: Ann }"))]);
    let data = DataRuntime::new();
    let session = SlotSession::new(data.clone()).with_run_grants([fixture.folder_grant()]);
    let error = owner_schema(&fixture.schemas(session), &fixture.markdown()).expect_err("missing");
    let codes = template_diagnostics(&error, &fixture.path("doc.md"))
        .map(|diagnostics| diagnostics.into_iter().map(|diagnostic| diagnostic.code).collect::<Vec<_>>())
        .unwrap_or_default();
    assert!(!codes.iter().any(|code| code == APPROVAL_REQUIRED || code == DENIED), "{codes:?}: {error}");
    assert!(data.read_set().contains(&fixture.entry("missing.yaml")), "{:?}", data.read_set());
}

// ---------------------------------------------------------------------------
// Cache and dependencies: each "command" is a new session on a new runtime
// ---------------------------------------------------------------------------

/// One command resolving `owner` with a folder grant: its schema (or
/// failure) and its read set.
fn command(fixture: &Fixture) -> (Result<Value, SchemaError>, Vec<String>) {
    let data = DataRuntime::new();
    let session = SlotSession::new(data.clone()).with_run_grants([fixture.folder_grant()]);
    let owner = owner_schema(&fixture.schemas(session), &fixture.markdown());
    (owner, data.read_set())
}

#[test]
fn a_warm_result_reports_the_reads_of_a_cold_one() {
    let fixture = Fixture::with(&[
        ("types.yaml", TRANSITIVE_TYPES),
        ("address.yaml", ADDRESS_TYPES),
        ("doc.md", &owner_document(TEMPLATE_PERSON, "{ name: Ann }")),
    ]);
    let (cold, cold_reads) = command(&fixture);
    #[cfg(feature = "work-counters")]
    let runs = slot_runs();
    let (warm, warm_reads) = command(&fixture);
    #[cfg(feature = "work-counters")]
    assert_eq!(slot_runs(), runs, "the warm command is served from the cache");
    assert_eq!(cold.expect("cold"), warm.expect("warm"));
    assert!(cold_reads.contains(&fixture.entry("types.yaml")), "{cold_reads:?}");
    assert_eq!(cold_reads, warm_reads);
}

/// Each edit grows the file, so the `stat` re-check notices it.
#[test]
fn editing_an_import_resolves_again() {
    let fixture = Fixture::with(&[
        ("types.yaml", TRANSITIVE_TYPES),
        ("address.yaml", ADDRESS_TYPES),
        ("doc.md", &owner_document(TEMPLATE_PERSON, "{ name: Ann }")),
    ]);
    let (first, _) = command(&fixture);
    assert!(!first.expect("first").to_string().contains("nickname"));
    write(&fixture.path("types.yaml"), &format!("{TRANSITIVE_TYPES}        nickname: string\n"));
    let (second, _) = command(&fixture);
    let second = second.expect("after editing types.yaml");
    assert!(second.to_string().contains("nickname"), "{second}");
    assert!(!second.to_string().contains("postcode"));
    write(&fixture.path("address.yaml"), &format!("{ADDRESS_TYPES}        postcode: string\n"));
    let (third, _) = command(&fixture);
    let third = third.expect("after editing address.yaml");
    assert!(third.to_string().contains("postcode"), "{third}");
}

#[test]
fn creating_a_missing_import_resolves_again() {
    let fixture = Fixture::with(&[("doc.md", &owner_document("\"{{ 'Person@./later.yaml' }}\"", "{ name: Ann }"))]);
    let (missing, _) = command(&fixture);
    missing.expect_err("later.yaml does not exist yet");
    #[cfg(feature = "work-counters")]
    let runs = slot_runs();
    let (cached, _) = command(&fixture);
    cached.expect_err("still missing");
    #[cfg(feature = "work-counters")]
    assert_eq!(slot_runs(), runs, "the failure is served from the cache while the file is missing");
    write(&fixture.path("later.yaml"), "kind: schema\ntypes:\n    Person:\n        name: string(required)\n");
    let (created, reads) = command(&fixture);
    let created = created.expect("resolves once created");
    assert!(created.to_string().contains(r#""required":["name"]"#), "{created}");
    assert!(reads.contains(&fixture.entry("later.yaml")), "{reads:?}");
}

/// A cached result is served only where its reads are granted again.
#[test]
fn a_warm_session_without_the_grant_needs_approval() {
    let fixture = Fixture::new();
    let (cold, _) = command(&fixture);
    cold.expect("granted");
    let data = DataRuntime::new();
    let error = owner_schema(&fixture.schemas(SlotSession::new(data.clone())), &fixture.markdown())
        .expect_err("not served from the cache");
    assert_eq!(codes(&error, &fixture.path("doc.md")), vec![APPROVAL_REQUIRED.to_string()], "{error}");
    assert!(data.read_set().is_empty(), "{:?}", data.read_set());
}

// ---------------------------------------------------------------------------
// Equivalence: a template type gives exactly the static type's schema and
// validation verdicts
// ---------------------------------------------------------------------------

/// The comparable part of a validation report: validity and each problem's
/// path, code, and message.
fn verdict(report: &ValidationReport) -> (bool, Vec<(String, String, String)>) {
    let problems = report
        .problems
        .iter()
        .map(|problem| (problem.path.clone(), format!("{:?}", problem.code), problem.message.clone()))
        .collect();
    (report.valid, problems)
}

fn assert_equivalent(template: &str, static_type: &str, valid: &str, invalid: &str) {
    let fixture = Fixture::new();
    let schemas = fixture.schemas(SlotSession::new(DataRuntime::new()).with_run_grants([fixture.folder_grant()]));
    let property = |name: &str, owner_type: &str, value: &str| {
        write(&fixture.path(name), &owner_document(owner_type, value));
        let markdown = fixture.markdown_at(name);
        let owner = owner_schema(&schemas, &markdown).unwrap_or_else(|error| panic!("{owner_type}: {error}"));
        let report = schemas.validate(&markdown).unwrap_or_else(|error| panic!("{owner_type}: {error}"));
        (owner, verdict(&report))
    };
    for (value, expect_valid) in [(valid, true), (invalid, false)] {
        let (template_owner, template_verdict) = property("template.md", template, value);
        let (static_owner, static_verdict) = property("static.md", static_type, value);
        assert_eq!(template_owner, static_owner, "{template} vs {static_type}");
        assert_eq!(template_verdict, static_verdict, "{template} vs {static_type} on {value}");
        assert_eq!(static_verdict.0, expect_valid, "{static_type} on {value}: {static_verdict:?}");
    }
}

macro_rules! equivalent {
    ($($name:ident: $template:expr, $static_type:expr, valid $valid:expr, invalid $invalid:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_equivalent($template, $static_type, $valid, $invalid);
            }
        )*
    };
}

equivalent! {
    equivalent_returned_string: "\"{{ 'string' }}\"", "string", valid "Ann", invalid "{ a: 1 }";
    equivalent_returned_list: "\"{{ ['string', 'number'] }}\"", "['string', 'number']", valid "3", invalid "[1]";
    equivalent_returned_pipe_union: "\"{{ 'string | number' }}\"", "string | number", valid "Ann", invalid "[1]";
    equivalent_returned_import: "\"{{ 'Person@./types.yaml' }}\"", "Person@./types.yaml", valid "{ name: Ann }", invalid "{ age: 3 }";
    equivalent_returned_array: "\"{{ 'string[]' }}\"", "string[]", valid "[a, b]", invalid "[{ a: 1 }]";
    equivalent_array_suffix: "\"{{ 'string' }}[]\"", "string[]", valid "[a, b]", invalid "[{ a: 1 }]";
}
