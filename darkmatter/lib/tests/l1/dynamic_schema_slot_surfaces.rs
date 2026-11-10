//! The library surfaces a host uses to approve, defer, and report schema
//! slot reads: compose pre-flight's slot approvals, a session's batched ask,
//! the reference-check session, `grants_needed` on approval records, the
//! bare-`SchemaError` projection, and `md clean`'s schema configuration;
//! then a template type through the same paths.
//!
//! Every case builds a real Git repository from the sandboxing-shaped
//! fixture (or, to have no automatic grant, a temp folder outside any
//! repository), anchors its request or context there with a temp home (so
//! the user's real grant store is never read), and stubs only the approval
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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
        let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
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

/// A template type (a slot supplying a property's whole type definition)
/// follows the same command paths a template constraint does: composition,
/// `md clean`, reference checks, and the pending report.
mod template_types {
    use darkmatter::markdown::compose::report_diagnostics::template_slot_records;
    use serde_json::Value;

    use super::*;

    const NOT_A_TYPE: &str = "dm.schema.slot_not_a_type";
    const PENDING: &str = "dm.schema.slot_pending";

    /// A canonical temp folder outside any repository, so no read is
    /// granted automatically.
    struct Folder {
        _temp: tempfile::TempDir,
        root: PathBuf,
    }

    impl Folder {
        fn new() -> Self {
            let temp = tempfile::tempdir().expect("temp dir");
            let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
            Self { _temp: temp, root }
        }

        /// Writes `doc.md` with `frontmatter` and returns its path.
        fn document(&self, frontmatter: &str) -> PathBuf {
            let path = self.root.join("doc.md");
            write(&path, &format!("---\n{frontmatter}---\n\nBody.\n"));
            path
        }

        fn context(&self) -> FileResolutionContext {
            FileResolutionContext::new(&self.root).with_env(HashMap::new())
        }
    }

    fn compose_in(document: &Path, home: &Path) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
        compose(document, &request(document, home))
    }

    /// The composed value of `key`; `None` when composition left it absent.
    fn composed_value(document: &Path, key: &str) -> Option<Value> {
        let home = empty_home();
        let (composed, _) =
            compose_in(document, home.path()).unwrap_or_else(|failure| panic!("composes: {}", failure.error));
        composed.frontmatter().as_map().get(key).cloned()
    }

    fn failure_codes(failure: &ComposeFailure) -> Vec<String> {
        error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect()
    }

    mod composition {
        use super::*;

        /// Composition materializes an absent optional key as `null` only
        /// when it has no default, so the returned default must be known
        /// before that decision.
        #[test]
        fn an_inline_template_type_default_is_known_before_the_default_decision() {
            let folder = Folder::new();
            let defaulted = folder.document("$schema:\n    count: \"{{ 'number(default(3))' }}\"\n");
            assert_eq!(composed_value(&defaulted, "count"), None);

            let plain = folder.document("$schema:\n    count: \"{{ 'number' }}\"\n");
            assert_eq!(composed_value(&plain, "count"), Some(Value::Null));
        }

        #[test]
        fn a_referenced_schema_file_template_type_coerces_the_value() {
            let folder = Folder::new();
            write(&folder.root.join("schema.yaml"), "kind: schema\n$schema:\n    count: \"{{ 'number' }}\"\n");
            let document = folder.document("$schema: ./schema.yaml\ncount: \"4\"\n");

            assert_eq!(composed_value(&document, "count"), Some(json!(4)));
        }

        #[test]
        fn a_required_suffix_makes_the_missing_key_a_validation_failure() {
            let folder = Folder::new();
            let document = folder.document("$schema:\n    title: \"{{ 'string' }}(required)\"\n");
            let home = empty_home();

            let failure = compose_in(&document, home.path()).expect_err("title is required");

            let codes = failure_codes(&failure);
            assert!(!codes.iter().any(|code| code.starts_with("dm.schema.slot_")), "{codes:?}");
            let messages: String =
                error_diagnostics(&failure.error).into_iter().map(|record| record.message).collect();
            assert!(messages.contains("title"), "{messages}");
        }

        #[test]
        fn a_required_suffix_over_a_null_or_unknown_definition_stays_optional() {
            for definition in ["null", "unknown"] {
                let folder = Folder::new();
                let document = folder.document(&format!("$schema:\n    note: \"{{{{ '{definition}' }}}}(required)\"\n"));
                let home = empty_home();

                let composed = compose_in(&document, home.path());

                assert!(composed.is_ok(), "{definition}: {}", composed.err().map(|f| f.error.to_string()).unwrap_or_default());
            }
        }

        #[test]
        fn a_refused_read_fails_the_composition_with_approval_required() {
            let folder = Folder::new();
            write(&folder.root.join("status-type.yaml"), "- enum(draft, published)\n");
            let document =
                folder.document("$schema:\n    status: \"{{ load_data('./status-type.yaml', 'string[]') }}\"\nstatus: draft\n");
            let home = empty_home();

            let failure = compose_in(&document, home.path()).expect_err("no grant covers the read");

            assert_eq!(failure_codes(&failure), vec![APPROVAL_REQUIRED.to_string()]);
        }

        #[test]
        fn a_malformed_returned_definition_fails_the_composition_as_not_a_type() {
            let folder = Folder::new();
            let document = folder.document("$schema:\n    status: \"{{ 123 }}\"\nstatus: draft\n");
            let home = empty_home();

            let failure = compose_in(&document, home.path()).expect_err("123 is not a type definition");

            assert_eq!(failure_codes(&failure), vec![NOT_A_TYPE.to_string()]);
        }
    }

    mod clean {
        use super::*;

        fn codes(error: &SchemaError, document: &Path) -> Vec<String> {
            template_diagnostics(error, document)
                .expect("a template failure")
                .into_iter()
                .map(|record| record.code)
                .collect()
        }

        fn analyze(folder: &Folder, document: &Path) -> Result<bool, SchemaError> {
            let context = folder.context();
            let clean = CleanSchemaConfig::new(context.clone())
                .with_slot_session(Arc::new(session(&context)))
                .resolve(Some(document))
                .expect("clean context");
            let yaml = std::fs::read_to_string(document).expect("document");
            let yaml = yaml.split("---\n").nth(1).expect("frontmatter").to_string();
            clean.analyze(&markdown(document), &yaml).map(|analysis| analysis.is_clean())
        }

        #[test]
        fn a_resolved_template_type_is_the_type_clean_checks_against() {
            let folder = Folder::new();
            let matching = folder.document("$schema:\n    status: \"{{ 'enum(draft, published)' }}\"\nstatus: draft\n");
            assert!(analyze(&folder, &matching).expect("resolves"));

            let outside = folder.document("$schema:\n    status: \"{{ 'enum(draft, published)' }}\"\nstatus: bogus\n");
            assert!(!analyze(&folder, &outside).expect("resolves"), "bogus is outside the returned enum");
        }

        #[test]
        fn a_failed_template_type_is_never_reported_clean() {
            let folder = Folder::new();
            let malformed = folder.document("$schema:\n    status: \"{{ 123 }}\"\nstatus: draft\n");
            let error = analyze(&folder, &malformed).expect_err("123 is not a type definition");
            assert_eq!(codes(&error, &malformed), vec![NOT_A_TYPE.to_string()]);

            write(&folder.root.join("status-type.yaml"), "- enum(draft, published)\n");
            let refused =
                folder.document("$schema:\n    status: \"{{ load_data('./status-type.yaml', 'string[]') }}\"\nstatus: draft\n");
            let error = analyze(&folder, &refused).expect_err("no grant covers the read");
            assert_eq!(codes(&error, &refused), vec![APPROVAL_REQUIRED.to_string()]);
        }
    }

    mod reference_check {
        use super::*;

        const FRONTMATTER: &str = "$schema:\n    status: \"{{ load_data('./status-type.yaml', 'string[]') }}\"\nstatus: bogus\n";

        fn reference_check_schemas(context: FileResolutionContext) -> DarkmatterSchemas {
            let session = session(&context).with_mode(SessionMode::ReferenceCheck);
            DarkmatterSchemas::new(context).with_slot_session(Arc::new(session))
        }

        fn validate(document: &Path, request: &ComposeRequest) -> bool {
            Markdown::try_from(document)
                .expect("document")
                .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(request)))
                .is_ok()
        }

        #[test]
        fn an_ungranted_template_type_stays_deferred() {
            let folder = Folder::new();
            write(&folder.root.join("status-type.yaml"), "- enum(draft, published)\n");
            let document = folder.document(FRONTMATTER);

            let effective = reference_check_schemas(folder.context()).effective_for(&markdown(&document));
            assert!(matches!(effective, Err(SchemaError::UnresolvedSlots(_))), "{:?}", effective.map(|e| e.is_some()));

            let home = empty_home();
            let request = request(&document, home.path());
            let session = Arc::new(SlotSession::for_request(&request).with_mode(SessionMode::ReferenceCheck));
            let checked = request.clone().map_options(|options| options.with_slot_session(session));
            assert!(validate(&document, &checked), "a deferred template type is not a failure");
            assert!(!validate(&document, &request), "without the session the same check stops at the read");
        }

        #[test]
        fn a_granted_template_type_resolves_and_the_returned_type_validates() {
            let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
            let repo = SlotRepo::build(&source, RepoState::Clean);
            let folder = folder_of(&repo.path(DOCUMENT_FILE));
            write(&folder.join("status-type.yaml"), "- enum(draft, published)\n");
            let document = folder.join("status.md");
            write(&document, &format!("---\n{FRONTMATTER}---\n\nBody.\n"));
            git(repo.root(), &["add", "-A"]);
            git(repo.root(), &["commit", "-q", "-m", "status type"]);
            let home = empty_home();

            let effective = reference_check_schemas(context(&repo, home.path()))
                .effective_for(&markdown(&document))
                .expect("a clean committed read is granted")
                .expect("a schema applies");

            assert!(!effective.validate(&json!({ "status": "bogus" })).valid);
            assert!(effective.validate(&json!({ "status": "draft" })).valid);
        }
    }

    mod pending_report {
        use super::*;

        #[test]
        fn each_pending_record_names_its_property_and_slot_role() {
            let folder = Folder::new();
            let document = folder.document(
                "$schema:\n    status: \"{{ 'enum(draft, published)' }}\"\n    os: \"enum({{ ['macOS', 'Linux'] }})\"\n",
            );
            let context = folder.context();
            let passive = session(&context).with_mode(SessionMode::Passive);
            let error = DarkmatterSchemas::new(context)
                .with_slot_session(Arc::new(passive))
                .effective_for(&markdown(&document))
                .err()
                .expect("a passive session leaves the template unresolved");
            assert!(matches!(error, SchemaError::UnresolvedSlots(_)), "{error:?}");

            let records = template_slot_records(&error).expect("a template failure");

            let messages: Vec<&str> = records.iter().map(|(_, record)| record.message.as_str()).collect();
            assert_eq!(records.len(), 2, "{messages:?}");
            assert!(records.iter().all(|(_, record)| record.code == PENDING), "{records:?}");
            assert!(
                messages.iter().any(|message| message.starts_with("property `status` has a template type, so its type is pending: ")),
                "{messages:?}"
            );
            assert!(
                messages.iter().any(|message| message.starts_with("property `os` has a template constraint: ")),
                "{messages:?}"
            );
        }
    }
}
