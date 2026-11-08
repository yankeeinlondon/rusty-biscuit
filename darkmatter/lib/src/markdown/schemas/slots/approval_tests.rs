//! The slot approval model end to end, in-process.
//!
//! Each case builds a real Git repository in a temp dir (spawned `git` with a
//! fixture identity), prepares the schema template with `simplified-schema`'s
//! public resolver, and resolves it through [`DarkmatterSlotHost`] with the
//! run's gates ([`RunGates`] over [`SlotGrants`]): the real automatic grant
//! (Sniff's per-path tracked state), the real grant store, and the real
//! expression engine. Nothing is stubbed except the approval handler.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use simplified_schema::SchemaError;
use simplified_schema::slots::{PendingSchema, SlotResolutionError, SlotRunRequest};

use super::codes::*;
use super::evaluator::SlotEvaluator;
use super::grants::gate::{RunGates, SlotGrants};
use super::grants::{
    Grant, GrantBook, GrantDecision, GrantScope, GrantStore, GrantTarget, SlotGrantDecision, SlotGrantRequest,
};
use super::host::DarkmatterSlotHost;
use crate::markdown::compose::DataRuntime;
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::expression::read_gate::GateRefusal;
use crate::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};

const OS_LIST: &str = "- macOS\n- Linux\n- Windows\n";
const SCHEMA: &str = "schemas/sandbox.yaml";
const DATA: &str = "data/os.yaml";

fn schema_reading(argument: &str) -> String {
    format!("kind: schema\n$schema:\n    os: enum({{{{ load_data({argument}, \"string[]\") }}}}; required)\n")
}

/// Runs Git in `root` with a fixture identity and no host configuration.
fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" })
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed in {}", root.display());
}

/// What Git knows about the data file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Clean,
    Dirty,
    Untracked,
    Ignored,
    Lfs,
    NoRepository,
}

/// A repository holding the sandbox schema and its data file, in `state`.
struct Repo {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new(state: State) -> Self {
        Self::with_schema(state, &schema_reading("\"../data/os.yaml\""))
    }

    fn with_schema(state: State, schema: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("repo");
        let repo = Self { _temp: temp, root };
        repo.write(SCHEMA, schema);
        repo.write(DATA, OS_LIST);
        if state == State::NoRepository {
            return repo;
        }
        git(&repo.root, &["init", "-q", "-b", "main"]);
        match state {
            State::Untracked => git(&repo.root, &["add", "--", SCHEMA]),
            State::Ignored => {
                repo.write(".gitignore", "data/\n");
                git(&repo.root, &["add", "--", SCHEMA, ".gitignore"]);
            }
            State::Lfs => {
                repo.write(".gitattributes", "data/os.yaml filter=lfs diff=lfs merge=lfs -text\n");
                git(&repo.root, &["add", "-A"]);
            }
            _ => git(&repo.root, &["add", "-A"]),
        }
        git(&repo.root, &["commit", "-q", "-m", "fixture"]);
        if state == State::Dirty {
            repo.write(DATA, "- macOS\n- Linux\n- Windows\n- FreeBSD\n");
        }
        repo
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root.clone(), |path, part| path.join(part))
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn grants(&self, book: GrantBook) -> Arc<SlotGrants> {
        Arc::new(SlotGrants::new(book, Some(&self.root), None))
    }
}

fn context(root: &Path) -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::new(root).with_env(HashMap::new())
}

fn pending_at(root: &Path, schema: &Path) -> PendingSchema {
    let reference = serde_yaml_ng::Value::String(biscuit_file::to_portable_string(schema));
    match simplified_schema::resolve::resolve_yaml_schema(
        &reference,
        root,
        &context(root),
        crate::markdown::schemas::embedded::SHIPPED_SOURCES,
    ) {
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("a pending handle"),
        other => panic!("expected a template, got {other:?}"),
    }
}

fn host(grants: &Arc<SlotGrants>, data: &DataRuntime) -> DarkmatterSlotHost {
    DarkmatterSlotHost::new(SlotEvaluator::new(data.clone()).with_gates(Arc::new(RunGates(Arc::clone(grants)))))
}

/// Resolves the repository's schema with `grants`; the members of `os`, or
/// the failed slot's code and refusal.
fn resolve_with(repo: &Repo, schema: &Path, grants: &Arc<SlotGrants>) -> Result<Value, (&'static str, Option<GateRefusal>)> {
    let data = DataRuntime::new();
    match pending_at(&repo.root, schema).resolve(&host(grants, &data), &SlotRunRequest { context: context(&repo.root) }) {
        Ok(resolved) => Ok(members(&resolved.json_schema)),
        Err(SlotResolutionError::Failed(failed)) => {
            let failure = &failed[0].1;
            let refusal = match failure.downcast_ref::<ExpressionError>().map(ExpressionError::without_callbacks) {
                Some(ExpressionError::SlotReadRefused { refusal, .. }) => Some(refusal.clone()),
                _ => None,
            };
            Err((failure.code.unwrap_or("none"), refusal))
        }
        Err(other) => panic!("unexpected {other:?}"),
    }
}

fn resolve(repo: &Repo, book: GrantBook) -> Result<Value, (&'static str, Option<GateRefusal>)> {
    resolve_with(repo, &repo.path(SCHEMA), &repo.grants(book))
}

/// The members of `os`'s enum, wherever its lowering put them (an optional
/// property is a union with `null`).
fn members(schema: &Value) -> Value {
    fn find(value: &Value) -> Option<Value> {
        match value {
            Value::Object(map) => map.get("enum").cloned().or_else(|| map.values().find_map(find)),
            Value::Array(items) => items.iter().find_map(find),
            _ => None,
        }
    }
    find(&schema["properties"]["os"]).unwrap_or(Value::Null)
}

fn approval_required(result: Result<Value, (&'static str, Option<GateRefusal>)>) -> Vec<Grant> {
    match result {
        Err((SLOT_APPROVAL_REQUIRED_CODE, Some(GateRefusal::ApprovalRequired { needed }))) => needed,
        other => panic!("expected approval required, got {other:?}"),
    }
}

fn store_with(home: &Path, lines: &str) -> GrantStore {
    let path = GrantStore::path_for_home(Some(home)).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, lines).unwrap();
    GrantStore::load(Some(&path)).unwrap()
}

/// An approval handler answering each slot request with `answer`, and
/// recording every request and the order shell and slot prompts came in.
struct Scripted {
    answer: Box<dyn Fn(&SlotGrantRequest) -> SlotGrantDecision + Send + Sync>,
    slot_requests: Mutex<Vec<SlotGrantRequest>>,
    order: Mutex<Vec<&'static str>>,
}

impl Scripted {
    fn new(answer: impl Fn(&SlotGrantRequest) -> SlotGrantDecision + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self { answer: Box::new(answer), slot_requests: Mutex::new(Vec::new()), order: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<SlotGrantRequest> {
        self.slot_requests.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for Scripted {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.order.lock().unwrap().push("shell");
        Ok(ShellApprovalDecision::AllowOnce)
    }

    fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        self.order.lock().unwrap().push("slots");
        self.slot_requests.lock().unwrap().extend(requests.iter().cloned());
        Ok(requests.iter().map(|request| (self.answer)(request)).collect())
    }
}

fn first_choice(scope: GrantScope) -> impl Fn(&SlotGrantRequest) -> SlotGrantDecision {
    move |request| {
        let grant = request.choices.iter().find(|grant| grant.decision == GrantDecision::Allow && grant.scope == scope);
        grant.map_or(SlotGrantDecision::Unanswered, SlotGrantDecision::from_grant)
    }
}

mod automatic {
    use super::*;

    #[test]
    fn a_clean_committed_file_in_the_schemas_repository_is_read_without_a_grant() {
        let repo = Repo::new(State::Clean);
        assert_eq!(resolve(&repo, GrantBook::empty()), Ok(json!(["macOS", "Linux", "Windows"])));
    }

    #[test]
    fn a_dirty_untracked_ignored_or_filtered_file_needs_a_grant() {
        for state in [State::Dirty, State::Untracked, State::Ignored, State::Lfs] {
            let repo = Repo::new(state);
            let needed = approval_required(resolve(&repo, GrantBook::empty()));
            let data = repo.path(DATA);
            assert!(
                needed.contains(&Grant { decision: GrantDecision::Allow, target: GrantTarget::File(data.clone()), scope: GrantScope::Once }),
                "{state:?}: {needed:?}"
            );
            assert!(
                needed.iter().any(|grant| grant.target == GrantTarget::Dir(data.parent().unwrap().to_path_buf())),
                "{state:?}: the file's folder is offered"
            );
            assert!(needed.iter().all(|grant| grant.decision == GrantDecision::Allow), "{state:?}");
        }
    }

    #[test]
    fn no_repository_means_no_automatic_grant() {
        let repo = Repo::new(State::NoRepository);
        let grants = Arc::new(SlotGrants::new(GrantBook::empty(), None, None));
        approval_required(resolve_with(&repo, &repo.path(SCHEMA), &grants));
    }

    #[test]
    fn a_file_of_another_repository_needs_a_grant() {
        let other = Repo::new(State::Clean);
        let argument = format!("\"{}\"", biscuit_file::to_portable_string(&other.path(DATA)));
        let repo = Repo::with_schema(State::Clean, &schema_reading(&argument));
        approval_required(resolve(&repo, GrantBook::empty()));
    }

    #[test]
    fn a_schema_outside_the_requests_repository_gets_no_automatic_grant() {
        let repo = Repo::new(State::Clean);
        let outside = tempfile::tempdir().unwrap();
        let outside_schema = biscuit_file::canonicalize_simplified(outside.path()).unwrap().join("sandbox.yaml");
        let argument = format!("\"{}\"", biscuit_file::to_portable_string(&repo.path(DATA)));
        std::fs::write(&outside_schema, schema_reading(&argument)).unwrap();
        approval_required(resolve_with(&repo, &outside_schema, &repo.grants(GrantBook::empty())));
    }

    // A Windows symlink needs a privilege the CI runners lack, and a
    // junction is a folder link the rule handles the same way; the Unix case
    // proves the canonicalization.
    #[cfg(unix)]
    #[test]
    fn a_symlink_escaping_the_repository_is_judged_by_where_it_points() {
        let repo = Repo::new(State::Clean);
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("os.yaml");
        std::fs::write(&target, OS_LIST).unwrap();
        std::os::unix::fs::symlink(&target, repo.path("data/link.yaml")).unwrap();
        git(&repo.root, &["add", "data/link.yaml"]);
        git(&repo.root, &["commit", "-q", "-m", "link"]);
        repo.write(SCHEMA, &schema_reading("\"../data/link.yaml\""));
        git(&repo.root, &["commit", "-q", "-am", "schema"]);
        let needed = approval_required(resolve(&repo, GrantBook::empty()));
        let canonical = biscuit_file::canonicalize_simplified(&target).unwrap();
        assert!(needed.iter().any(|grant| grant.target == GrantTarget::File(canonical.clone())), "{needed:?}");
    }

    #[test]
    fn an_unsaved_buffer_that_differs_from_disk_needs_a_grant() {
        let repo = Repo::new(State::Clean);
        let mut buffers = simplified_schema::resolve::buffers::SourceBuffers::default();
        buffers.insert(&repo.path(DATA), Arc::from("- Plan9\n"));
        let result = simplified_schema::resolve::buffers::with(&buffers, || resolve(&repo, GrantBook::empty()));
        approval_required(result);

        let mut same = simplified_schema::resolve::buffers::SourceBuffers::default();
        same.insert(&repo.path(DATA), Arc::from(OS_LIST));
        let result = simplified_schema::resolve::buffers::with(&same, || resolve(&repo, GrantBook::empty()));
        assert_eq!(result, Ok(json!(["macOS", "Linux", "Windows"])), "a buffer equal to disk changes nothing");
    }

    #[test]
    fn bytes_read_that_differ_from_the_granted_ones_are_refused() {
        use crate::markdown::compose::expression::functions::ReadKind;
        use crate::markdown::compose::expression::read_gate::{ReadLocation, ReadTarget};
        use crate::markdown::schemas::slots::evaluator::SlotGates;
        let repo = Repo::new(State::Clean);
        let gates = RunGates(repo.grants(GrantBook::empty()));
        let gate = gates.gate_for(&repo.path(SCHEMA));
        let mut target = ReadTarget {
            function: "load_data".into(),
            kind: ReadKind::Content,
            location: ReadLocation::Local(repo.path(DATA)),
            observed_hash: None,
        };
        assert_eq!(gate.check(&target), Ok(()), "granted before the read");
        // The file changed between Git's answer and the slot's own read.
        target.observed_hash = Some(biscuit_hash::blake3_hash_bytes(b"- Plan9\n"));
        assert!(matches!(gate.check(&target), Err(GateRefusal::ApprovalRequired { .. })));
        target.observed_hash = None;
        assert!(gate.check(&target).is_err(), "the file is treated as dirty from then on");
    }

    #[test]
    fn an_existence_probe_of_an_untracked_path_in_the_repository_is_automatic() {
        let schema = "kind: schema\n$schema:\n    os: 'enum({{ file_exists(\"../data/new.yaml\") ? [\"yes\"] : [\"no\"] }})'\n";
        let repo = Repo::with_schema(State::Clean, schema);
        assert_eq!(resolve(&repo, GrantBook::empty()), Ok(json!(["no"])));
        repo.write("data/new.yaml", "x\n");
        assert_eq!(resolve(&repo, GrantBook::empty()), Ok(json!(["yes"])), "untracked, still a probe");
    }

    #[test]
    fn a_folder_listing_is_never_automatic() {
        let schema = "kind: schema\n$schema:\n    os: enum({{ find_files(\"../data/*.yaml\") }})\n";
        let repo = Repo::with_schema(State::Clean, schema);
        assert!(matches!(resolve(&repo, GrantBook::empty()), Err((SLOT_APPROVAL_REQUIRED_CODE, _))));
    }

    #[test]
    fn a_missing_file_in_the_repository_fails_as_not_found_without_a_grant() {
        let repo = Repo::with_schema(State::Clean, &schema_reading("\"../data/missing.yaml\""));
        let (code, refusal) = resolve(&repo, GrantBook::empty()).unwrap_err();
        assert_ne!(code, SLOT_APPROVAL_REQUIRED_CODE);
        assert_eq!(refusal, None, "the ordinary typed not-found failure, not a refusal");
    }

    #[test]
    fn a_missing_file_outside_the_repository_needs_a_grant() {
        let outside = tempfile::tempdir().unwrap();
        let missing = biscuit_file::canonicalize_simplified(outside.path()).unwrap().join("missing.yaml");
        let argument = format!("\"{}\"", biscuit_file::to_portable_string(&missing));
        let repo = Repo::with_schema(State::Clean, &schema_reading(&argument));
        approval_required(resolve(&repo, GrantBook::empty()));
    }
}

mod granted {
    use super::*;

    #[test]
    fn a_persisted_file_grant_allows_a_dirty_file() {
        let repo = Repo::new(State::Dirty);
        let home = tempfile::tempdir().unwrap();
        let store = store_with(home.path(), &format!("allow file {}\n", repo.path(DATA).display()));
        let members = resolve(&repo, GrantBook::new(store, Vec::new())).unwrap();
        assert_eq!(members, json!(["macOS", "Linux", "Windows", "FreeBSD"]));
    }

    #[test]
    fn a_folder_grant_for_this_run_allows_every_file_below_it() {
        let repo = Repo::new(State::Untracked);
        let mut book = GrantBook::empty();
        book.add_once(GrantDecision::Allow, GrantTarget::Dir(repo.root.join("data")));
        assert!(resolve(&repo, book).is_ok());
    }

    #[test]
    fn a_deny_beats_the_automatic_grant_and_every_allow() {
        let repo = Repo::new(State::Clean);
        let home = tempfile::tempdir().unwrap();
        let data = repo.path(DATA);
        let store = store_with(home.path(), &format!("deny file {}\n", data.display()));
        let mut book = GrantBook::new(store, Vec::new());
        book.add_once(GrantDecision::Allow, GrantTarget::File(data));
        assert_eq!(resolve(&repo, book), Err((SLOT_DENIED_CODE, Some(GateRefusal::Denied))));

        let mut once = GrantBook::empty();
        once.add_once(GrantDecision::Deny, GrantTarget::Dir(repo.root.clone()));
        assert_eq!(resolve(&repo, once), Err((SLOT_DENIED_CODE, Some(GateRefusal::Denied))), "deny for this run");
    }

    #[test]
    fn a_computed_target_asks_the_handler_during_evaluation() {
        let schema = schema_reading("lower(env.NAME || \"../DATA/OS.YAML\")");
        let repo = Repo::with_schema(State::Untracked, &schema);
        for (answer, expected) in [
            (SlotGrantDecision::Unanswered, Err(SLOT_APPROVAL_REQUIRED_CODE)),
            (SlotGrantDecision::Deny { target: GrantTarget::File(repo.path(DATA)), scope: GrantScope::Once }, Err(SLOT_DENIED_CODE)),
            // Not one of the request's choices: unanswered.
            (SlotGrantDecision::Allow { target: GrantTarget::Dir(repo.root.clone()), scope: GrantScope::Once }, Err(SLOT_APPROVAL_REQUIRED_CODE)),
            (SlotGrantDecision::Allow { target: GrantTarget::File(repo.path(DATA)), scope: GrantScope::Once }, Ok(())),
        ] {
            let handler = Scripted::new({
                let answer = answer.clone();
                move |_| answer.clone()
            });
            let grants = Arc::new(SlotGrants::new(GrantBook::empty(), Some(&repo.root), Some(handler.clone())));
            let result = resolve_with(&repo, &repo.path(SCHEMA), &grants).map(|_| ()).map_err(|(code, _)| code);
            assert_eq!(result, expected, "{answer:?}");
            let asked = handler.asked();
            assert_eq!(asked.len(), 1, "{answer:?}");
            assert_eq!(asked[0].schema.as_deref(), Some(repo.path(SCHEMA).as_path()));
        }
    }

    #[test]
    fn a_persisted_answer_is_written_to_the_users_store() {
        let repo = Repo::with_schema(State::Untracked, &schema_reading("lower(\"../DATA/OS.YAML\")"));
        let home = tempfile::tempdir().unwrap();
        let path = GrantStore::path_for_home(Some(home.path())).unwrap();
        let store = GrantStore::load(Some(&path)).unwrap();
        let handler = Scripted::new(first_choice(GrantScope::Persisted));
        let grants = Arc::new(SlotGrants::new(GrantBook::new(store, Vec::new()), Some(&repo.root), Some(handler)));
        let resolved = resolve_with(&repo, &repo.path(SCHEMA), &grants);
        assert!(resolved.is_ok(), "{resolved:?}");
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(written, format!("allow file {}\n", repo.path(DATA).display()));
        // Read back: the next run needs no handler.
        let reloaded = GrantBook::new(GrantStore::load(Some(&path)).unwrap(), Vec::new());
        assert!(resolve(&repo, reloaded).is_ok());
    }
}

mod remote {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn server() -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/os.yaml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(OS_LIST))
            .mount(&server)
            .await;
        server
    }

    fn resolve_remote(repo: &Repo, book: GrantBook) -> Result<Value, &'static str> {
        let grants = repo.grants(book);
        let data = DataRuntime::new();
        let runtime = grants.remote_runtime(None);
        let host = DarkmatterSlotHost::new(
            SlotEvaluator::new(data).with_gates(Arc::new(RunGates(Arc::clone(&grants)))).with_remote(runtime),
        );
        pending_at(&repo.root, &repo.path(SCHEMA))
            .resolve(&host, &SlotRunRequest { context: context(&repo.root) })
            .map(|resolved| members(&resolved.json_schema))
            .map_err(|error| match error {
                SlotResolutionError::Failed(failed) => failed[0].1.code.unwrap_or("none"),
                other => panic!("{other:?}"),
            })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_url_is_fetched_only_under_a_grant_naming_its_scheme_host_and_port() {
        let server = server().await;
        let url = url::Url::parse(&format!("{}/os.yaml", server.uri())).unwrap();
        let repo = Repo::with_schema(State::Clean, &schema_reading(&format!("\"{url}\"")));
        let port = url.port().unwrap();
        let site = |port: u16| GrantTarget::Site { scheme: "http".into(), host: "127.0.0.1".into(), port: Some(port) };
        let once = |target: GrantTarget| {
            let mut book = GrantBook::empty();
            book.add_once(GrantDecision::Allow, target);
            book
        };

        let repo_ref = &repo;
        let outcomes = tokio::task::spawn_blocking({
            let repo_root = repo_ref.root.clone();
            let url = url.clone();
            move || {
                let repo = Repo { _temp: tempfile::tempdir().unwrap(), root: repo_root };
                vec![
                    resolve_remote(&repo, GrantBook::empty()),
                    resolve_remote(&repo, once(site(port.wrapping_add(1)))),
                    resolve_remote(&repo, once(GrantTarget::Url(url.clone()))),
                    resolve_remote(&repo, once(site(port))),
                    resolve_remote(&repo, GrantBook::new(GrantStore::empty(), vec!["127.0.0.1".into()])),
                ]
            }
        })
        .await
        .unwrap();
        assert_eq!(outcomes[0], Err(SLOT_APPROVAL_REQUIRED_CODE), "no grant");
        assert_eq!(outcomes[1], Err(SLOT_APPROVAL_REQUIRED_CODE), "another port of the host");
        let fetched = Ok(json!(["macOS", "Linux", "Windows"]));
        assert_eq!(outcomes[2], fetched, "the exact URL");
        assert_eq!(outcomes[3], fetched, "the site");
        assert_eq!(outcomes[4], fetched, "--allow host=");
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3, "a refused read never reaches the network");
    }
}

mod preflight {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::compose::context::request::RequestSnapshot;
    use crate::markdown::compose::{ComposeOperation, ComposeOptions, ComposeRequest, ComposeSource};
    use crate::markdown::schemas::slots::grants::targets::{CollectedRead, collect_targets};

    /// A document beside the schema, naming it, with `extra` frontmatter.
    fn document(repo: &Repo, extra: &str) -> (Markdown, ComposeRequest) {
        let path = repo.path("docs/page.md");
        repo.write("docs/page.md", &format!("---\n$schema: ../{SCHEMA}\nos: macOS\n{extra}---\nbody\n"));
        let markdown = Markdown::from(std::fs::read_to_string(&path).unwrap())
            .with_source(ComposeSource::File(path.clone()));
        (markdown, request(repo, &path))
    }

    fn request(repo: &Repo, document: &Path) -> ComposeRequest {
        let home = repo.root.parent().unwrap().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell_policy_root(repo.root.parent().unwrap())
            .with_source_file(document);
        let snapshot = RequestSnapshot::new(document.parent().unwrap()).with_home(Some(home)).with_env(HashMap::new());
        ComposeRequest::prepare(options, &snapshot).unwrap()
    }

    #[test]
    fn slot_reads_and_shell_commands_are_asked_in_one_batch_slots_first() {
        let repo = Repo::new(State::Dirty);
        let (markdown, request) = document(&repo, "greeting: \"$(echo hi)\"\n");
        let handler = Scripted::new(first_choice(GrantScope::Once));
        let approvals = markdown.compose_preflight_approvals(&request, Some(handler.clone())).unwrap();
        assert_eq!(*handler.order.lock().unwrap(), ["slots", "shell"]);
        let asked = handler.asked();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].location, crate::markdown::compose::expression::read_gate::ReadLocation::Local(repo.path(DATA)));
        assert_eq!(approvals.stats.slot_reads_asked, 1);
        assert_eq!(
            approvals.slot_grants,
            [Grant { decision: GrantDecision::Allow, target: GrantTarget::File(repo.path(DATA)), scope: GrantScope::Once }]
        );
    }

    #[test]
    fn a_read_the_automatic_grant_covers_is_not_asked() {
        let repo = Repo::new(State::Clean);
        let (markdown, request) = document(&repo, "");
        let handler = Scripted::new(first_choice(GrantScope::Once));
        let approvals = markdown.compose_preflight_approvals(&request, Some(handler.clone())).unwrap();
        assert!(handler.asked().is_empty());
        assert_eq!(approvals.stats.slot_reads_asked, 0);
    }

    #[test]
    fn without_a_handler_nothing_is_asked_and_the_read_stays_unapproved() {
        let repo = Repo::new(State::Dirty);
        let (markdown, request) = document(&repo, "");
        let approvals = markdown.compose_preflight_approvals(&request, None).unwrap();
        assert_eq!(approvals.stats.slot_reads_asked, 1);
        assert!(approvals.slot_grants.is_empty());
    }

    #[test]
    fn repository_content_cannot_grant() {
        let repo = Repo::new(State::Dirty);
        let line = format!("allow file {}\n", repo.path(DATA).display());
        // A committed store at the repository root and beside the document
        // is not the user's store.
        repo.write(".config/darkmatter/schema-read-policy", &line);
        repo.write("docs/.config/darkmatter/schema-read-policy", &line);
        repo.write(".dmls.toml", "[slots]\ngrant_store = \".config/darkmatter/schema-read-policy\"\n");
        let (markdown, request) = document(&repo, "");
        let handler = Scripted::new(|_| SlotGrantDecision::Unanswered);
        markdown.compose_preflight_approvals(&request, Some(handler.clone())).unwrap();
        assert_eq!(handler.asked().len(), 1, "the read is still asked about");
    }

    #[test]
    fn an_invalid_grant_store_stops_pre_flight_at_its_line() {
        let repo = Repo::new(State::Dirty);
        let (markdown, request) = document(&repo, "");
        let home = repo.root.parent().unwrap().join("home");
        let store = GrantStore::path_for_home(Some(&home)).unwrap();
        std::fs::create_dir_all(store.parent().unwrap()).unwrap();
        std::fs::write(&store, "# mine\nallow file relative/path\n").unwrap();
        let error = markdown.compose_preflight_approvals(&request, None).unwrap_err();
        let ShellExpansionError::SlotGrantStore(invalid) = &error else { panic!("{error:?}") };
        assert_eq!((invalid.path.as_path(), invalid.line), (store.as_path(), 2));
        let diagnostics = crate::markdown::compose::report_diagnostics::error_diagnostics(
            &crate::markdown::MarkdownError::ShellExpansion(Box::new(error)),
        );
        assert_eq!(diagnostics[0].code, SLOT_GRANT_STORE_INVALID_CODE);
        let source = diagnostics[0].source.as_ref().unwrap();
        assert_eq!((source.file.as_deref(), source.line), (Some(store.as_path()), Some(2)));
    }

    #[test]
    fn an_unselected_trigger_payload_with_slots_asks_for_nothing() {
        let repo = Repo::new(State::Dirty);
        repo.write(
            "docs/schemas/other.yaml",
            &format!("kind: schema-trigger\n$match:\n    kind: never\n{}", schema_reading("\"../../data/os.yaml\"").replace("kind: schema\n", "")),
        );
        let path = repo.path("docs/plain.md");
        repo.write("docs/plain.md", "---\ntitle: plain\n---\nbody\n");
        let markdown = Markdown::from(std::fs::read_to_string(&path).unwrap())
            .with_source(ComposeSource::File(path.clone()));
        let handler = Scripted::new(first_choice(GrantScope::Once));
        markdown.compose_preflight_approvals(&request(&repo, &path), Some(handler.clone())).unwrap();
        assert!(handler.asked().is_empty());
    }

    #[test]
    fn a_target_of_literals_and_pure_functions_is_known_before_evaluation() {
        let repo = Repo::with_schema(
            State::Clean,
            "kind: schema\n$schema:\n    a: enum({{ load_data(lower(\"../DATA/OS.YAML\"), \"string[]\") }})\n    b: enum({{ load_data(env.DATA, \"string[]\") }})\n    c: 'enum({{ true ? [\"x\"] : load_data(\"https://example.com/x.yaml\") }})'\n",
        );
        let reads: Vec<CollectedRead> = collect_targets(&pending_at(&repo.root, &repo.path(SCHEMA)), &context(&repo.root))
            .into_iter()
            .flat_map(|slot| slot.reads)
            .collect();
        use crate::markdown::compose::expression::read_gate::ReadLocation;
        let locations: Vec<Option<ReadLocation>> = reads
            .iter()
            .map(|read| match read {
                CollectedRead::Known(target) => Some(target.location.clone()),
                CollectedRead::Computed { .. } => None,
            })
            .collect();
        assert_eq!(
            locations,
            [
                Some(ReadLocation::Local(repo.path(DATA))),
                None,
                Some(ReadLocation::Remote(url::Url::parse("https://example.com/x.yaml").unwrap())),
            ],
            "a branch that would not run still counts"
        );
    }
}
