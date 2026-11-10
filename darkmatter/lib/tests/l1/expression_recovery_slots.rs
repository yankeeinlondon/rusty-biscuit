//! `try` inside dynamic schema slots: the slot gate's refusals propagate, a
//! missing in-repository file recovers without a grant, and a slot's class
//! is the join over both operands.
//!
//! Every case builds a miniature Git repository in a temp dir (`repo/`, with
//! `schema.yaml` and a document naming it; a file outside the repository
//! sits next to `repo/`), anchors its resolution context there with a temp
//! home (so the user's real grant store is never read), and resolves the
//! template through `DarkmatterSchemas` with a real `SlotSession`.

use std::collections::HashMap;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::ReadMark;
use darkmatter::markdown::compose::expression::read_gate::ReadLocation;
use darkmatter::markdown::compose::expression::Diagnostic;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{
    ComposeOptions, ComposeRequest, ComposeSource, DataRuntime, RequestSnapshot, ShellExpansionError,
};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::grants::{SlotGrantDecision, SlotGrantRequest};
use darkmatter::markdown::schemas::slots::{SessionMode, SlotSession};
use serde_json::{Value, json};
use simplified_schema::SchemaError;

use crate::dynamic_slots_fixtures::git;

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const DENIED: &str = "dm.schema.slot_denied";
const FORBIDDEN_BINDING: &str = "dm.schema.slot_forbidden_binding";
const FORBIDDEN_FUNCTION: &str = "dm.schema.slot_forbidden_function";
const EMPTY: &str = "dm.schema.slot_empty";
/// The line of `schema.yaml` that holds the `os` slot.
const SLOT_LINE: usize = 3;

/// A committed repository whose `schema.yaml` types `os` as `os_type`.
struct Repo {
    _temp: tempfile::TempDir,
    /// The folder holding `repo/`; files here are outside the repository.
    outer: PathBuf,
    root: PathBuf,
    home: tempfile::TempDir,
    /// The data runtime of the latest command; each resolution is a new
    /// command with its own (cold) data memo.
    runtime: RefCell<DataRuntime>,
}

impl Repo {
    /// Writes `schema.yaml`, `doc.md`, and `files` (relative to the
    /// repository root) and commits them all. `OUTSIDE` in `os_type` names
    /// the folder holding the repository, as an absolute path: a relative
    /// `..` out of the repository is a `RepositoryEscape`, not a read.
    fn new(os_type: &str, files: &[(&str, &str)]) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let outer = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let root = outer.join("repo");
        let repo = Self {
            _temp: temp,
            outer,
            root,
            home: tempfile::tempdir().expect("temp home"),
            runtime: RefCell::new(DataRuntime::new()),
        };
        let os_type = os_type.replace("OUTSIDE", &biscuit_file::to_portable_string(&repo.outer));
        repo.write("schema.yaml", &format!("kind: schema\n$schema:\n    os: {os_type}\n"));
        repo.write("doc.md", "---\n$schema: ./schema.yaml\nos: a\n---\n\nBody.\n");
        for (name, text) in files {
            repo.write(name, text);
        }
        git(&repo.root, &["init", "-q", "-b", "main"]);
        repo.commit();
        repo
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("write");
    }

    fn commit(&self) {
        git(&self.root, &["add", "-A"]);
        git(&self.root, &["commit", "-q", "-m", "fixture"]);
    }

    /// Writes `name` next to the repository, outside it.
    fn write_outside(&self, name: &str, text: &str) -> PathBuf {
        let path = self.outer.join(name);
        std::fs::write(&path, text).expect("write outside");
        path
    }

    /// Persists `line` as the home's whole grant store.
    fn persist_grant(&self, line: &str) {
        let store = self.home.path().join(".config/darkmatter/schema-read-policy");
        std::fs::create_dir_all(store.parent().expect("store folder")).expect("store folder");
        std::fs::write(store, line).expect("grant store");
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(&self.root)
            .with_env(HashMap::new())
            .with_repository_root(&self.root)
            .with_home_dir(self.home.path())
    }

    fn markdown(&self) -> Markdown {
        let document = self.path("doc.md");
        Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document))
    }

    fn runtime(&self) -> DataRuntime {
        self.runtime.borrow().clone()
    }

    /// Resolves the document's schema in a new command: a new session of
    /// `mode` over a new data runtime.
    fn resolve(&self, mode: SessionMode) -> Result<Value, SchemaError> {
        let context = self.context();
        self.runtime.replace(DataRuntime::new());
        let session = SlotSession::for_context(self.runtime(), &context).with_mode(mode);
        DarkmatterSchemas::new(context)
            .with_slot_session(Arc::new(session))
            .effective_for(&self.markdown())
            .map(|effective| (*effective.expect("a schema applies").json_schema).clone())
    }

    /// The resolved `os` members of a full session's resolution.
    fn members(&self) -> Value {
        let schema = self.resolve(SessionMode::Full).unwrap_or_else(|error| panic!("resolves: {error}"));
        // An optional property lowers to `anyOf: [null, {enum}]`.
        schema.pointer("/properties/os/anyOf/1/enum").cloned().unwrap_or_else(|| panic!("an enum: {schema}"))
    }

    /// The diagnostics of a full session's failed resolution.
    fn failure(&self) -> Vec<Diagnostic> {
        let error = self.resolve(SessionMode::Full).expect_err("the resolution fails");
        assert!(matches!(error, SchemaError::SlotResolution(_)), "{error:?}");
        error_diagnostics(&error as &(dyn std::error::Error + 'static))
    }

    /// Whether anything observed `path` (read its bytes or probed it).
    fn observed(&self, path: &Path) -> bool {
        let entry = biscuit_file::to_portable_string(path);
        self.runtime().observations_since(ReadMark::default()).iter().any(|observation| observation.entry == entry)
    }

    fn read_set_has(&self, path: &Path) -> bool {
        self.runtime().read_set().contains(&biscuit_file::to_portable_string(path))
    }
}

/// Asserts `records` is one `code` record at the slot's line of the schema.
fn assert_slot_record(repo: &Repo, records: &[Diagnostic], code: &str) {
    let codes: Vec<&str> = records.iter().map(|record| record.code.as_str()).collect();
    assert_eq!(codes, [code], "{records:?}");
    let at = records[0].source.as_ref().expect("positioned");
    assert_eq!(at.file.as_deref(), Some(repo.path("schema.yaml").as_path()), "{records:?}");
    assert_eq!(at.line, Some(SLOT_LINE), "{records:?}");
}

/// Q21: a slot read of an absent file inside the repository is a probe that
/// needs no grant, and its typed not-found error is what `try` recovers.
#[test]
fn slot_try_recovers_missing_in_repo_file() {
    let repo = Repo::new(r#"enum(a, {{ try(load_data("missing.yaml", "string[]"), []) }})"#, &[]);
    assert_eq!(repo.members(), json!(["a"]));
    assert!(repo.observed(&repo.path("missing.yaml")), "the probe is an input of the resolution");

    // Unguarded, the same miss fails the slot instead of contributing nothing.
    let control = Repo::new(r#"enum(a, {{ load_data("missing.yaml", "string[]") }})"#, &[]);
    let records = control.failure();
    let codes: Vec<&str> = records.iter().map(|record| record.code.as_str()).collect();
    assert_eq!(codes, ["dm.schema.slot_failed"], "a miss, not a refusal: {records:?}");
    let failure = records[0].reference_details.as_ref().and_then(|details| details.failure.as_deref());
    assert_eq!(failure, Some("no-match"), "{records:?}");
}

/// Outside the repository the absent file is refused before `try` sees the
/// miss, so the refusal propagates.
#[test]
fn slot_try_missing_outside_repo_needs_grant() {
    let repo = Repo::new(r#"enum(a, {{ try(load_data("OUTSIDE/missing.yaml", "string[]"), []) }})"#, &[]);
    assert_slot_record(&repo, &repo.failure(), APPROVAL_REQUIRED);

    // With the folder granted, the read happens, misses, and recovers.
    repo.persist_grant(&format!("allow dir {}\n", biscuit_file::to_portable_string(&repo.outer)));
    assert_eq!(repo.members(), json!(["a"]));
}

#[test]
fn slot_try_propagates_approval_required() {
    let sentinel = r#"load_data("sentinel.yaml", "string[]")"#;
    let repo = Repo::new(
        &format!(r#"enum(a, {{{{ try(load_data("OUTSIDE/outside.yaml", "string[]"), {sentinel}) }}}})"#),
        &[("sentinel.yaml", "- fallback\n")],
    );
    let outside = repo.write_outside("outside.yaml", "- b\n");
    assert_slot_record(&repo, &repo.failure(), APPROVAL_REQUIRED);
    assert!(!repo.observed(&repo.path("sentinel.yaml")), "the fallback never ran");
    assert!(!repo.read_set_has(&outside), "the refused file was not read");

    // Granted, the same slot reads the file and never runs the fallback.
    repo.persist_grant(&format!("allow file {}\n", biscuit_file::to_portable_string(&outside)));
    assert_eq!(repo.members(), json!(["a", "b"]));
    assert!(!repo.observed(&repo.path("sentinel.yaml")));
}

#[test]
fn slot_try_propagates_denied() {
    let sentinel = r#"load_data("sentinel.yaml", "string[]")"#;
    let repo = Repo::new(
        &format!(r#"enum(a, {{{{ try(load_data("OUTSIDE/outside.yaml", "string[]"), {sentinel}) }}}})"#),
        &[("sentinel.yaml", "- fallback\n")],
    );
    let outside = repo.write_outside("outside.yaml", "- b\n");
    repo.persist_grant(&format!("deny file {}\n", biscuit_file::to_portable_string(&outside)));
    assert_slot_record(&repo, &repo.failure(), DENIED);
    assert!(!repo.observed(&repo.path("sentinel.yaml")), "the fallback never ran");
    assert!(!repo.read_set_has(&outside), "the denied file was not read");
}

/// Admission walks both operands and rejects before anything is evaluated,
/// so not even the (readable) primary operand is read.
#[test]
fn slot_try_forbidden_binding_in_fallback_rejected() {
    let repo = Repo::new(
        r#"enum(a, {{ try(load_data("names.yaml", "string[]"), fm.platforms) }})"#,
        &[("names.yaml", "- b\n")],
    );
    assert_slot_record(&repo, &repo.failure(), FORBIDDEN_BINDING);
    assert!(!repo.observed(&repo.path("names.yaml")), "nothing was evaluated");

    let control = Repo::new(r#"enum(a, {{ try(load_data("names.yaml", "string[]"), []) }})"#, &[("names.yaml", "- b\n")]);
    assert_eq!(control.members(), json!(["a", "b"]));
}

#[test]
fn slot_try_forbidden_function_in_fallback_rejected() {
    let repo = Repo::new(
        r#"enum(a, {{ try(load_data("names.yaml", "string[]"), as_markdown("names.yaml")) }})"#,
        &[("names.yaml", "- b\n")],
    );
    assert_slot_record(&repo, &repo.failure(), FORBIDDEN_FUNCTION);
    assert!(!repo.observed(&repo.path("names.yaml")), "nothing was evaluated");
}

#[test]
fn slot_try_empty_fallback_contributes_nothing() {
    let os_type = r#"enum(a, {{ try(load_data("extra.yaml", "string[]"), []) }}, b)"#;
    let repo = Repo::new(os_type, &[]);
    assert_eq!(repo.members(), json!(["a", "b"]));

    let control = Repo::new(os_type, &[("extra.yaml", "- c\n")]);
    assert_eq!(control.members(), json!(["a", "c", "b"]));
}

/// A fallback `[]` that leaves the whole list empty is still the empty-list
/// failure, at the slot.
#[test]
fn slot_try_only_fallback_empty_list_is_slot_empty() {
    let repo = Repo::new(r#"enum({{ try(load_data("missing.yaml", "string[]"), []) }})"#, &[]);
    assert_slot_record(&repo, &repo.failure(), EMPTY);

    let control = Repo::new(r#"enum({{ try(load_data("missing.yaml", "string[]"), ['z']) }})"#, &[]);
    assert_eq!(control.members(), json!(["z"]));
}

#[test]
fn slot_try_records_only_selected_reads() {
    let files = [("first.yaml", "- b\n"), ("second.yaml", "- c\n")];
    let repo = Repo::new(
        r#"enum(a, {{ try(load_data("first.yaml", "string[]"), load_data("second.yaml", "string[]")) }})"#,
        &files,
    );
    assert_eq!(repo.members(), json!(["a", "b"]));
    assert!(repo.read_set_has(&repo.path("first.yaml")));
    assert!(!repo.observed(&repo.path("second.yaml")), "the unselected fallback recorded no read");

    // When the primary misses, its probe and the fallback's read are both
    // recorded, and the fallback supplies the members.
    let control = Repo::new(
        r#"enum(a, {{ try(load_data("missing.yaml", "string[]"), load_data("second.yaml", "string[]")) }})"#,
        &files,
    );
    assert_eq!(control.members(), json!(["a", "c"]));
    assert!(control.read_set_has(&control.path("second.yaml")));
    assert!(control.observed(&control.path("missing.yaml")));
}

/// The static class join is crate-private (`admission::slot_class_report`,
/// whose only consumer is admission's rejection list), so each pair is
/// observed through its public consequence: whether the slot is admitted,
/// what it reads, and whether the resolution re-checks its inputs.
#[test]
fn slot_try_class_is_join_of_arguments() {
    // Pure/pure: admitted, and the resolution depends on nothing.
    let pure = Repo::new("enum(a, {{ try(['b'], ['c']) }})", &[]);
    assert_eq!(pure.members(), json!(["a", "b"]));
    assert_eq!(pure.runtime().observations_since(ReadMark::default()), Vec::new());

    // Reading/`[]`: input-reading, so the resolution records the read and a
    // later command re-checks it and sees the edit. The file is outside the
    // repository under a persisted grant, so the edit needs no commit.
    let reading = Repo::new(r#"enum(a, {{ try(load_data("OUTSIDE/names.yaml", "string[]"), []) }})"#, &[]);
    let names = reading.write_outside("names.yaml", "- b\n");
    reading.persist_grant(&format!("allow file {}\n", biscuit_file::to_portable_string(&names)));
    assert_eq!(reading.members(), json!(["a", "b"]));
    assert!(reading.read_set_has(&names));
    std::fs::write(&names, "- b\n- c\n").expect("edit");
    assert_eq!(reading.members(), json!(["a", "b", "c"]));

    // Pure/volatile: a volatile fallback is admitted (it is not a rejected
    // class), and the selected pure operand supplies the members.
    let volatile = Repo::new("enum(a, {{ try(['b'], recent_commits(1)) }})", &[]);
    assert_eq!(volatile.members(), json!(["a", "b"]));
}

/// Answers every slot read unanswered and records what it was asked.
#[derive(Default)]
struct Recording {
    asked: Mutex<Vec<SlotGrantRequest>>,
}

impl ShellApprovalHandler for Recording {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::Deny)
    }

    fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        self.asked.lock().expect("asked").extend(requests.iter().cloned());
        Ok(requests.iter().map(|_| SlotGrantDecision::Unanswered).collect())
    }
}

/// Compose pre-flight collects the target of each operand, so a host can
/// approve the fallback's read before either runs.
#[test]
fn slot_try_preflight_collects_both_targets() {
    let repo = Repo::new(
        r#"enum(a, {{ try(load_data("OUTSIDE/first.yaml", "string[]"), load_data("OUTSIDE/second.yaml", "string[]")) }})"#,
        &[],
    );
    let first = repo.write_outside("first.yaml", "- b\n");
    let second = repo.write_outside("second.yaml", "- c\n");
    let document = repo.path("doc.md");
    let snapshot =
        RequestSnapshot::new(&repo.root).with_home(Some(repo.home.path().to_path_buf())).with_env(HashMap::new());
    let options = ComposeOptions::new().with_source_file(&document).with_data_runtime(repo.runtime());
    let request = ComposeRequest::prepare(options, &snapshot).expect("request");
    let handler = Arc::new(Recording::default());

    let grants = Markdown::try_from(document.as_path())
        .expect("document")
        .compose_slot_approvals(&request, Some(handler.clone()))
        .expect("approvals");

    assert!(grants.is_empty(), "{grants:?}");
    let asked: Vec<ReadLocation> =
        handler.asked.lock().expect("asked").iter().map(|request| request.location.clone()).collect();
    assert!(asked.contains(&ReadLocation::Local(first.clone())), "{asked:?}");
    assert!(asked.contains(&ReadLocation::Local(second.clone())), "{asked:?}");
    assert!(!repo.observed(&first) && !repo.observed(&second), "collecting read nothing");
}

/// D4: a reference check never asks, so an unapproved read defers the
/// property's judgment; it never settles for the fallback.
#[test]
fn slot_try_reference_check_defers_instead_of_fallback() {
    let repo = Repo::new(r#"enum(a, {{ try(load_data("OUTSIDE/outside.yaml", "string[]"), ['fallback']) }})"#, &[]);
    repo.write_outside("outside.yaml", "- b\n");

    let checked = repo.resolve(SessionMode::ReferenceCheck);
    assert!(matches!(checked, Err(SchemaError::UnresolvedSlots(_))), "{checked:?}");

    // A full session reports the refusal the reference check deferred.
    assert_slot_record(&repo, &repo.failure(), APPROVAL_REQUIRED);
}

/// D9: the recovered miss is an input of the cached resolution, so creating
/// the probed file makes the next command resolve with its data.
#[test]
fn slot_try_resolution_cache_recovers_after_file_created() {
    let repo = Repo::new(r#"enum(a, {{ try(load_data("later.yaml", "string[]"), []) }})"#, &[]);
    assert_eq!(repo.members(), json!(["a"]));
    assert_eq!(repo.members(), json!(["a"]), "a repeat is served the same recovered resolution");

    repo.write("later.yaml", "- b\n");
    repo.commit();
    assert_eq!(repo.members(), json!(["a", "b"]));
    assert!(repo.read_set_has(&repo.path("later.yaml")));
}
