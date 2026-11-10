//! Schema-file resolution (`DarkmatterSchemas::resolve_schema_file`): a
//! schema file validated on its own resolves every slot it holds (in its
//! `$schema` and in its named `types`) through the instance's slot session,
//! with the same grants, refusal codes, and approval batch a document naming
//! the file gets.
//!
//! Each case writes its files into a temp folder outside any repository (so
//! no automatic grant applies) and states the session's grants or handler.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::expression::read_gate::ReadLocation;
use darkmatter::markdown::compose::report_diagnostics::template_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{DataRuntime, ShellExpansionError};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::grants::{
    Grant, GrantDecision, GrantScope, GrantTarget, SlotGrantDecision, SlotGrantRequest,
};
use darkmatter::markdown::schemas::slots::SlotSession;
use simplified_schema::SchemaError;
use simplified_schema::resolve::ResolvedSchema;

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const DENIED: &str = "dm.schema.slot_denied";
const NOT_A_TYPE: &str = "dm.schema.slot_not_a_type";

const TYPES: &str = "kind: schema\ntypes:\n    Person:\n        name: string(required)\n";
const TEMPLATE: &str = "kind: schema\n$schema:\n    owner: \"{{ 'Person@./types.yaml' }}\"\n";
const STATIC: &str = "kind: schema\n$schema:\n    owner: Person@./types.yaml\n";

/// A canonical temp folder holding `types.yaml` (a `Person` type).
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let fixture = Self { _temp: temp, root };
        fixture.write("types.yaml", TYPES);
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::write(&path, text).expect("write");
        path
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(&self.root).with_env(HashMap::new())
    }

    fn schemas(&self, session: SlotSession) -> DarkmatterSchemas {
        DarkmatterSchemas::new(self.context()).with_slot_session(Arc::new(session))
    }

    /// Resolves `text`, written as `name`, through `schemas`.
    fn resolve(&self, schemas: &DarkmatterSchemas, name: &str, text: &str) -> Result<ResolvedSchema, SchemaError> {
        let path = self.write(name, text);
        schemas.resolve_schema_file(&path, text)
    }
}

fn grant(decision: GrantDecision, path: &Path) -> Grant {
    Grant { decision, target: GrantTarget::File(path.to_path_buf()), scope: GrantScope::Once }
}

fn codes(error: &SchemaError, file: &Path) -> Vec<String> {
    template_diagnostics(error, file)
        .unwrap_or_else(|| panic!("not a template failure: {error:?}"))
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// An approval handler answering every slot read with "this file, once"
/// and recording what it was asked.
#[derive(Default)]
struct Recording {
    calls: AtomicUsize,
    asked: Mutex<Vec<SlotGrantRequest>>,
}

impl Recording {
    fn asked_paths(&self) -> Vec<PathBuf> {
        self.asked
            .lock()
            .expect("asked")
            .iter()
            .filter_map(|request| match &request.location {
                ReadLocation::Local(path) => Some(path.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ShellApprovalHandler for Recording {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::Deny)
    }

    fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.asked.lock().expect("asked").extend(requests.iter().cloned());
        Ok(requests
            .iter()
            .map(|request| {
                request
                    .choices
                    .iter()
                    .find(|grant| {
                        grant.decision == GrantDecision::Allow
                            && grant.scope == GrantScope::Once
                            && matches!(grant.target, GrantTarget::File(_))
                    })
                    .map_or(SlotGrantDecision::Unanswered, SlotGrantDecision::from_grant)
            })
            .collect())
    }
}

fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

fn mentions(paths: &[PathBuf], file: &Path) -> bool {
    let file = canonical(file);
    paths.iter().any(|path| canonical(path) == file)
}

#[test]
fn a_granted_template_type_resolves_to_the_static_equivalent() {
    let fixture = Fixture::new();
    let session = SlotSession::new(DataRuntime::new()).with_run_grants([grant(GrantDecision::Allow, &fixture.path("types.yaml"))]);
    let resolved = fixture.resolve(&fixture.schemas(session), "schema.yaml", TEMPLATE).expect("resolves");
    let expected = fixture
        .resolve(&DarkmatterSchemas::new(fixture.context()), "schema.yaml", STATIC)
        .expect("the static file resolves");
    assert_eq!(resolved.json_schema, expected.json_schema);
    assert!(resolved.json_schema.to_string().contains(r#""required":["name"]"#), "{}", resolved.json_schema);
}

#[test]
fn an_undecided_import_without_a_handler_needs_approval() {
    let fixture = Fixture::new();
    let error = fixture
        .resolve(&fixture.schemas(SlotSession::new(DataRuntime::new())), "schema.yaml", TEMPLATE)
        .expect_err("refused");
    assert!(matches!(error, SchemaError::SlotResolution(_)), "{error:?}");
    assert_eq!(codes(&error, &fixture.path("schema.yaml")), vec![APPROVAL_REQUIRED.to_string()], "{error}");
}

#[test]
fn a_denied_import_keeps_the_denied_code() {
    let fixture = Fixture::new();
    let session = SlotSession::new(DataRuntime::new()).with_run_grants([grant(GrantDecision::Deny, &fixture.path("types.yaml"))]);
    let error = fixture.resolve(&fixture.schemas(session), "schema.yaml", TEMPLATE).expect_err("denied");
    assert_eq!(codes(&error, &fixture.path("schema.yaml")), vec![DENIED.to_string()], "{error}");
}

#[test]
fn without_a_session_the_template_stays_unresolved() {
    let fixture = Fixture::new();
    let error = fixture.resolve(&DarkmatterSchemas::new(fixture.context()), "schema.yaml", TEMPLATE).expect_err("pending");
    assert!(matches!(error, SchemaError::UnresolvedSlots(_)), "{error:?}");
}

#[test]
fn a_template_constraint_resolves() {
    let fixture = Fixture::new();
    let resolved = fixture
        .resolve(
            &fixture.schemas(SlotSession::new(DataRuntime::new())),
            "schema.yaml",
            "kind: schema\n$schema:\n    os: \"enum({{ ['a', 'b'] }})\"\n",
        )
        .expect("resolves");
    let os = resolved.json_schema["properties"]["os"].to_string();
    assert!(os.contains(r#""a""#) && os.contains(r#""b""#), "{os}");
}

/// A returned value that is not type syntax is refused at the slot, never
/// lowered as some other type.
#[test]
fn a_returned_definition_that_is_not_a_type_is_refused() {
    let fixture = Fixture::new();
    for slot in ["{{ 123 }}", "{{ 'nonsense((' }}"] {
        let text = format!("kind: schema\n$schema:\n    owner: \"{slot}\"\n");
        let result = fixture.resolve(&fixture.schemas(SlotSession::new(DataRuntime::new())), "schema.yaml", &text);
        let error = result.expect_err(slot);
        assert_eq!(codes(&error, &fixture.path("schema.yaml")), vec![NOT_A_TYPE.to_string()], "{slot}: {error}");
    }
}

/// A `types`-only file has no export; its slot-bearing named types are
/// still resolved, and their failures surface.
#[test]
fn a_slot_in_a_named_type_of_a_types_only_file_resolves() {
    let fixture = Fixture::new();
    let schemas = fixture.schemas(SlotSession::new(DataRuntime::new()));
    let resolved = fixture
        .resolve(&schemas, "status.yaml", "kind: schema\ntypes:\n    Status: \"{{ 'enum(a, b)' }}\"\n")
        .expect("resolves");
    let status = resolved.json_schema["properties"]["Status"].to_string();
    assert!(status.contains(r#""a""#) && status.contains(r#""b""#), "{status}");

    let error = fixture
        .resolve(&schemas, "broken.yaml", "kind: schema\ntypes:\n    Status: \"{{ 123 }}\"\n")
        .expect_err("not a type");
    assert_eq!(codes(&error, &fixture.path("broken.yaml")), vec![NOT_A_TYPE.to_string()], "{error}");
}

#[test]
fn templates_are_listed_only_for_a_file_holding_slots() {
    let fixture = Fixture::new();
    let schemas = fixture.schemas(SlotSession::new(DataRuntime::new()));
    let template = fixture.write("schema.yaml", TEMPLATE);
    assert!(!schemas.schema_file_templates(&template, TEMPLATE).is_empty());
    let concrete = fixture.write("static.yaml", STATIC);
    assert!(schemas.schema_file_templates(&concrete, STATIC).is_empty());
}

/// Pre-flight asks about the reads the slots name in advance; an import
/// named only by returned data is unknowable then, so it is asked through
/// the handler when the slot evaluates.
#[test]
fn the_batch_asks_named_reads_and_a_computed_import_is_asked_at_resolution() {
    let fixture = Fixture::new();
    fixture.write("status.yaml", "type: enum(a, b)\n");
    fixture.write("owner.yaml", "type: Person@./types.yaml\n");
    let text = "kind: schema\n$schema:\n    status: \"{{ load_data('./status.yaml', '{ type: string }').type }}\"\n    owner: \"{{ load_data('./owner.yaml', '{ type: string }').type }}\"\n";
    let path = fixture.write("schema.yaml", text);
    let handler = Arc::new(Recording::default());
    let schemas = fixture.schemas(SlotSession::new(DataRuntime::new()).with_handler(handler.clone()));

    let templates = schemas.schema_file_templates(&path, text);
    assert!(!templates.is_empty());
    let asked = schemas.approve_template_reads(&templates).expect("the handler answers");
    let batch = handler.asked_paths();
    assert_eq!(asked, batch.len(), "{batch:?}");
    assert!(mentions(&batch, &fixture.path("status.yaml")), "{batch:?}");
    assert!(mentions(&batch, &fixture.path("owner.yaml")), "{batch:?}");
    assert!(!mentions(&batch, &fixture.path("types.yaml")), "a computed import is not in the batch: {batch:?}");

    let resolved = schemas.resolve_schema_file(&path, text).expect("resolves under the handler's answers");
    let after = handler.asked_paths();
    assert!(mentions(&after, &fixture.path("types.yaml")), "the computed import is asked when it is read: {after:?}");
    assert!(resolved.json_schema.to_string().contains(r#""required":["name"]"#), "{}", resolved.json_schema);
}

#[test]
fn text_that_is_not_a_schema_document_is_refused() {
    let fixture = Fixture::new();
    let error = fixture
        .resolve(&fixture.schemas(SlotSession::new(DataRuntime::new())), "plain.yaml", "title: hello\n")
        .expect_err("not a schema");
    assert!(matches!(error, SchemaError::NotASchemaDocument { .. }), "{error:?}");
}
