//! The library surfaces a host uses to approve, defer, and report schema
//! slot reads: compose pre-flight's slot approvals, a session's batched ask,
//! the reference-check session, `grants_needed` on approval records, the
//! bare-`SchemaError` projection, and `md clean`'s schema configuration.
//!
//! Every case builds a real Git repository from the sandboxing-shaped
//! fixture, anchors its request or context there with a temp home (so the
//! user's real grant store is never read), and stubs only the approval
//! handler.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::read_gate::ReadLocation;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, template_diagnostics};
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{
    ComposeFailure, ComposeOptions, ComposeReport, ComposeRequest, ComposeSource, DataRuntime, RequestSnapshot,
    ShellExpansionError,
};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::clean::CleanSchemaConfig;
use darkmatter::markdown::schemas::slots::grants::{
    Grant, GrantDecision, GrantScope, GrantTarget, SlotGrantDecision, SlotGrantRequest,
};
use darkmatter::markdown::schemas::slots::{SessionMode, SlotSession};
use serde_json::json;
use simplified_schema::SchemaError;

use crate::dynamic_slots_fixtures::{DATA_FILE, DOCUMENT_FILE, RepoState, SCHEMA_FILE, SlotRepo, git};

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";

/// An approval handler that answers each slot request with `answer` and
/// records how often it was called and what it was asked.
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

/// The "this file, once" choice of `request`.
fn file_once(request: &SlotGrantRequest) -> Option<&Grant> {
    request.choices.iter().find(|grant| {
        grant.decision == GrantDecision::Allow
            && grant.scope == GrantScope::Once
            && matches!(grant.target, GrantTarget::File(_))
    })
}

fn allow_file_once(request: &SlotGrantRequest) -> SlotGrantDecision {
    file_once(request).map_or(SlotGrantDecision::Unanswered, SlotGrantDecision::from_grant)
}

/// A home with no grant store, so only the run's own grants apply.
fn empty_home() -> tempfile::TempDir {
    tempfile::tempdir().expect("temp home")
}

fn request(document: &Path, home: &Path) -> ComposeRequest {
    let snapshot = RequestSnapshot::new(document.parent().expect("document folder"))
        .with_home(Some(home.to_path_buf()))
        .with_env(HashMap::new());
    ComposeRequest::prepare(ComposeOptions::new().with_source_file(document), &snapshot).expect("request")
}

fn compose(document: &Path, request: &ComposeRequest) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    Markdown::try_from(document).expect("document").compose_with_warnings(request)
}

/// The document's resolution context: its folder, the fixture repository,
/// and `home`.
fn context(repo: &SlotRepo, home: &Path) -> FileResolutionContext {
    FileResolutionContext::new(repo.path(DOCUMENT_FILE).parent().expect("document folder"))
        .with_env(HashMap::new())
        .with_repository_root(repo.root())
        .with_home_dir(home)
}

/// The fixture document with its source set, as a validating host loads it.
fn markdown(document: &Path) -> Markdown {
    Markdown::try_from(document).expect("document").with_source(ComposeSource::File(document.to_path_buf()))
}

fn session(context: &FileResolutionContext) -> SlotSession {
    SlotSession::for_context(DataRuntime::new(), context)
}

fn folder_of(path: &Path) -> PathBuf {
    path.parent().expect("parent").to_path_buf()
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).expect("write");
}

mod compose_slot_approvals {
    use super::*;

    #[test]
    fn a_dirty_data_file_is_asked_once_and_its_grant_lets_the_composition_read_it() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let document = repo.path(DOCUMENT_FILE);
        // A command attaches its data runtime before building the session, so
        // the session's slot reads join the composition's read set.
        let request = request(&document, home.path()).map_options(|options| options.with_data_runtime(DataRuntime::new()));
        let handler = Recording::new(allow_file_once);

        let grants = Markdown::try_from(document.as_path())
            .expect("document")
            .compose_slot_approvals(&request, Some(handler.clone()))
            .expect("approvals");

        assert_eq!(handler.calls(), 1);
        let asked = handler.asked();
        assert_eq!(asked.len(), 1, "{asked:?}");
        assert_eq!(asked[0].location, ReadLocation::Local(repo.path(DATA_FILE)));
        assert_eq!(grants, vec![file_once(&asked[0]).expect("a file-once choice").clone()]);

        let session = Arc::new(SlotSession::for_request(&request).with_run_grants(grants));
        let granted = request.map_options(|options| options.with_slot_session(session));
        let (_, report) = compose(&document, &granted).unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        assert!(
            report.read_set.contains(&biscuit_file::to_portable_string(&repo.path(DATA_FILE))),
            "{:?}",
            report.read_set
        );
    }

    #[test]
    fn without_a_handler_nothing_is_granted_and_the_composition_needs_approval() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let document = repo.path(DOCUMENT_FILE);
        let request = request(&document, home.path());

        let grants = Markdown::try_from(document.as_path())
            .expect("document")
            .compose_slot_approvals(&request, None)
            .expect("approvals");

        assert_eq!(grants, Vec::<Grant>::new());
        let failure = compose(&document, &request).expect_err("an unapproved dirty read");
        let codes: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect();
        assert_eq!(codes, vec![APPROVAL_REQUIRED.to_string()]);
    }

    #[test]
    fn a_clean_committed_data_file_is_granted_automatically_without_asking() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::Clean);
        let home = empty_home();
        let document = repo.path(DOCUMENT_FILE);
        let request = request(&document, home.path());
        let handler = Recording::new(allow_file_once);

        let grants = Markdown::try_from(document.as_path())
            .expect("document")
            .compose_slot_approvals(&request, Some(handler.clone()))
            .expect("approvals");

        assert_eq!(handler.calls(), 0);
        assert!(grants.is_empty(), "{grants:?}");
    }
}

mod approve_slot_reads {
    use super::*;

    #[test]
    fn a_full_session_asks_once_and_the_answer_resolves_the_template() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let context = context(&repo, home.path());
        let handler = Recording::new(allow_file_once);
        let schemas = DarkmatterSchemas::new(context.clone())
            .with_slot_session(Arc::new(session(&context).with_handler(handler.clone())));
        let markdown = markdown(&repo.path(DOCUMENT_FILE));

        assert_eq!(schemas.approve_slot_reads(&markdown).expect("asked"), 1);
        assert_eq!(handler.calls(), 1);
        let effective = schemas.effective_for(&markdown);
        assert!(matches!(effective, Ok(Some(_))), "{:?}", effective.err());

        assert_eq!(schemas.approve_slot_reads(&markdown).expect("nothing left to ask"), 0);
        assert_eq!(handler.calls(), 1, "an answered read is not asked again");
    }

    #[test]
    fn a_reference_check_session_never_asks() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let context = context(&repo, home.path());
        let handler = Recording::new(allow_file_once);
        let session = session(&context).with_handler(handler.clone()).with_mode(SessionMode::ReferenceCheck);
        let schemas = DarkmatterSchemas::new(context).with_slot_session(Arc::new(session));

        assert_eq!(schemas.approve_slot_reads(&markdown(&repo.path(DOCUMENT_FILE))).expect("not asked"), 0);
        assert_eq!(handler.calls(), 0);
    }
}

mod reference_check {
    use super::*;

    #[test]
    fn an_ungranted_read_leaves_the_template_unresolved() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let context = context(&repo, home.path());
        let session = session(&context).with_mode(SessionMode::ReferenceCheck);
        let effective = DarkmatterSchemas::new(context)
            .with_slot_session(Arc::new(session))
            .effective_for(&markdown(&repo.path(DOCUMENT_FILE)));

        assert!(matches!(effective, Err(SchemaError::UnresolvedSlots(_))), "{:?}", effective.err());
    }

    #[test]
    fn an_automatically_granted_read_resolves_and_the_resolved_list_validates() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::Clean);
        let home = empty_home();
        let context = context(&repo, home.path());
        let session = session(&context).with_mode(SessionMode::ReferenceCheck);
        let effective = DarkmatterSchemas::new(context)
            .with_slot_session(Arc::new(session))
            .effective_for(&markdown(&repo.path(DOCUMENT_FILE)))
            .expect("resolves")
            .expect("a schema applies");

        let report =
            effective.validate(&json!({ "schema_revision": 5, "has_native_support": true, "os_support": ["BeOS"] }));
        assert!(!report.valid, "BeOS is not in the resolved list");
        assert!(!report.problems.is_empty());
    }

    #[test]
    fn a_reference_validation_with_a_reference_check_session_defers_the_unapproved_template() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let document = repo.path(DOCUMENT_FILE);
        let request = request(&document, home.path());
        let session = Arc::new(SlotSession::for_request(&request).with_mode(SessionMode::ReferenceCheck));
        let checked = request.clone().map_options(|options| options.with_slot_session(session));

        let validate = |request: &ComposeRequest| {
            Markdown::try_from(document.as_path())
                .expect("document")
                .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(request)))
        };

        let report = validate(&checked);
        assert!(report.is_ok(), "{:?}", report.err());
        // Without the session the same check stops at the unapproved read.
        let error = validate(&request).expect_err("the control stops");
        let codes: Vec<String> = error_diagnostics(&error).into_iter().map(|record| record.code).collect();
        assert_eq!(codes, vec![APPROVAL_REQUIRED.to_string()]);
    }
}

mod grants_needed {
    use super::*;

    #[test]
    fn an_approval_record_lists_the_grants_a_prompt_would_offer() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let document = repo.path(DOCUMENT_FILE);
        let failure = compose(&document, &request(&document, home.path())).expect_err("an unapproved dirty read");

        let records = error_diagnostics(&failure.error);
        assert_eq!(records.len(), 1, "{records:?}");
        let record = &records[0];
        assert_eq!(record.code, APPROVAL_REQUIRED);
        let file = biscuit_file::canonicalize_simplified(&repo.path(DATA_FILE)).expect("data file");
        let dir = biscuit_file::canonicalize_simplified(&folder_of(&repo.path(DATA_FILE))).expect("data folder");
        let needed: Vec<(&str, String, &str)> = record
            .grants_needed
            .as_ref()
            .expect("grants needed")
            .iter()
            .map(|grant| (grant.kind, grant.target.clone(), grant.scope))
            .collect();
        let file = file.display().to_string();
        let dir = dir.display().to_string();
        assert_eq!(
            needed,
            vec![
                ("file", file.clone(), "once"),
                ("file", file.clone(), "persisted"),
                ("dir", dir.clone(), "once"),
                ("dir", dir.clone(), "persisted"),
            ]
        );

        let serialized = serde_json::to_value(record).expect("serializes");
        assert_eq!(
            serialized["grants_needed"],
            json!([
                { "kind": "file", "target": file, "scope": "once" },
                { "kind": "file", "target": file, "scope": "persisted" },
                { "kind": "dir", "target": dir, "scope": "once" },
                { "kind": "dir", "target": dir, "scope": "persisted" },
            ])
        );
    }

    #[test]
    fn a_slot_failure_that_is_not_an_approval_names_no_grants() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::Clean);
        write(&repo.path(DATA_FILE), "- macOS\n- [Linux\n");
        git(repo.root(), &["commit", "-q", "-am", "malformed data"]);
        let home = empty_home();
        let document = repo.path(DOCUMENT_FILE);
        let failure = compose(&document, &request(&document, home.path())).expect_err("malformed data");

        let records = error_diagnostics(&failure.error);
        let record = records.first().expect("a record");
        assert!(record.code.starts_with("dm.data."), "{records:?}");
        assert_eq!(record.grants_needed, None);
        let serialized = serde_json::to_value(record).expect("serializes");
        assert!(serialized.get("grants_needed").is_none(), "{serialized}");
    }
}

mod schema_error_projection {
    use super::*;

    #[test]
    fn a_bare_slot_resolution_error_projects_to_positioned_slot_records() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let context = context(&repo, home.path());
        let error = DarkmatterSchemas::new(context.clone())
            .with_slot_session(Arc::new(session(&context)))
            .effective_for(&markdown(&repo.path(DOCUMENT_FILE)))
            .err()
            .expect("an unapproved dirty read");
        assert!(matches!(error, SchemaError::SlotResolution(_)), "{error:?}");

        let records = error_diagnostics(&error as &(dyn std::error::Error + 'static));

        assert!(!records.is_empty());
        for record in &records {
            assert_eq!(record.code, APPROVAL_REQUIRED, "{record:?}");
            let at = record.source.as_ref().expect("positioned");
            assert_eq!(at.file.as_deref(), Some(repo.path(SCHEMA_FILE).as_path()), "{record:?}");
            assert!(at.line.is_some(), "{record:?}");
        }
    }

    #[test]
    fn an_unresolved_template_is_one_pending_record_at_the_document() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::Clean);
        let home = empty_home();
        let document = repo.path(DOCUMENT_FILE);
        let error = DarkmatterSchemas::new(context(&repo, home.path()))
            .effective_for(&markdown(&document))
            .err()
            .expect("no session leaves the template unresolved");
        assert!(matches!(error, SchemaError::UnresolvedSlots(_)), "{error:?}");

        let records = template_diagnostics(&error, &document).expect("a template failure");

        assert_eq!(records.len(), 1, "{records:?}");
        assert_eq!(records[0].code, "dm.schema.slot_pending");
        assert_eq!(records[0].source.as_ref().and_then(|at| at.file.as_deref()), Some(document.as_path()));
    }
}

mod clean_schema_config {
    use super::*;

    #[test]
    fn a_dirty_read_without_a_handler_fails_the_resolution() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let context = context(&repo, home.path());
        let document = repo.path(DOCUMENT_FILE);
        let clean = CleanSchemaConfig::new(context.clone())
            .with_slot_session(Arc::new(session(&context)))
            .resolve(Some(&document))
            .expect("clean context");

        let effective = clean.effective_schema(&markdown(&document));

        assert!(matches!(effective, Err(SchemaError::SlotResolution(_))), "{:?}", effective.map(|e| e.is_some()));
    }

    #[test]
    fn an_allowing_handler_is_asked_once_for_every_resolution() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dynamic_slots/claudine");
        let repo = SlotRepo::build(&source, RepoState::DirtyData);
        let home = empty_home();
        let context = context(&repo, home.path());
        let document = repo.path(DOCUMENT_FILE);
        let handler = Recording::new(allow_file_once);
        let clean = CleanSchemaConfig::new(context.clone())
            .with_slot_session(Arc::new(session(&context).with_handler(handler.clone())))
            .resolve(Some(&document))
            .expect("clean context");
        let markdown = markdown(&document);

        let first = clean.effective_schema(&markdown);
        assert!(matches!(first, Ok(Some(_))), "{:?}", first.err());
        let second = clean.effective_schema(&markdown);
        assert!(matches!(second, Ok(Some(_))), "{:?}", second.err());

        assert_eq!(handler.calls(), 1);
    }
}
