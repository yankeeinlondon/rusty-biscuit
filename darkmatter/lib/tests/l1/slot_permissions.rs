//! Schema-slot reads under `permissions.file-read`, through the public API
//! (`SlotSession` + `DarkmatterSchemas`), with the real gate, expression
//! engine, configuration loader, and approval saver. F2, L1.
//!
//! Each case's temp folder is the home: the schema in `schemas/`, its data
//! in `data/`, and the user's `.darkmatter.yaml`. Nothing reads the
//! developer's home, configuration, or the process directory. Only the
//! approval handler is stubbed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_file::FileResolutionContext;
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::network::NetworkPermission;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::template_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeSource, DataRuntime, ShellExpansionError};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::grants::{
    Grant, GrantDecision, GrantScope, GrantTarget, SlotGrantDecision, SlotGrantRequest,
};
use serde_json::{Value, json};
use simplified_schema::SchemaError;

use crate::slot_support::{git, user_permissions, write_user_permissions};

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const NETWORK_DENIED: &str = "dm.schema.slot_network_denied";
const OS_LIST: &str = "- macOS\n- Linux\n- Windows\n";

/// A temp home holding `schemas/sandbox.yaml` (reading `../data/os.yaml`)
/// and that data file.
struct Home {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Home {
    fn new() -> Self {
        Self::reading("load_data(\"../data/os.yaml\", \"string[]\")")
    }

    /// A home whose sandbox schema's `os` is `enum({{ expression }})`.
    fn reading(expression: &str) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let home = Self { _temp: temp, root };
        home.write("schemas/sandbox.yaml", &schema_with(expression));
        home.write("data/os.yaml", OS_LIST);
        home
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root.clone(), |path, part| path.join(part))
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("write");
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(&self.root, Some(self.root.clone()), HashMap::new())
    }

    /// The user configuration after writing `entries` as its
    /// `permissions.file-read`.
    fn permissions(&self, entries: &[&str]) -> Arc<ConfigSnapshot> {
        user_permissions(&self.root, entries)
    }

    /// The user configuration as `.darkmatter.yaml` holds it now.
    fn captured(&self) -> Arc<ConfigSnapshot> {
        Arc::new(ConfigSnapshot::load(&self.context(), None, ConfigLoadMode::Passive).expect("the configuration loads"))
    }

    /// A session of one command reading through `data` under `config`.
    fn session(&self, data: &DataRuntime, config: Arc<ConfigSnapshot>) -> SlotSession {
        SlotSession::new(data.clone()).with_config_snapshot(config)
    }

    /// `os`'s members as `session` resolves the document naming `schema`,
    /// or the failed slots' codes.
    fn os_with(&self, session: SlotSession, schema: &str) -> Result<Value, Vec<String>> {
        let document = self.path("doc.md");
        std::fs::write(&document, format!("---\n$schema: ./{schema}\nos: macOS\n---\n\nBody.\n")).expect("document");
        let markdown =
            Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document.clone()));
        let schemas = DarkmatterSchemas::new(self.context()).with_slot_session(Arc::new(session));
        match schemas.effective_for(&markdown) {
            Ok(effective) => Ok(members(&effective.expect("a schema").json_schema["properties"]["os"])),
            Err(error) => Err(codes(&error, &document)),
        }
    }

    fn os(&self, session: SlotSession) -> Result<Value, Vec<String>> {
        self.os_with(session, "schemas/sandbox.yaml")
    }
}

fn schema_with(expression: &str) -> String {
    format!("kind: schema\n$schema:\n    os: \"enum({{{{ {} }}}})\"\n", expression.replace('"', "\\\""))
}

/// The members of the first `enum` below `schema`.
fn members(schema: &Value) -> Value {
    fn find(value: &Value) -> Option<Value> {
        match value {
            Value::Object(map) => map.get("enum").cloned().or_else(|| map.values().find_map(find)),
            Value::Array(items) => items.iter().find_map(find),
            _ => None,
        }
    }
    find(schema).unwrap_or(Value::Null)
}

fn codes(error: &SchemaError, document: &Path) -> Vec<String> {
    match template_diagnostics(error, document) {
        Some(diagnostics) => diagnostics.into_iter().map(|diagnostic| diagnostic.code).collect(),
        None => vec![format!("not a template failure: {error}")],
    }
}

fn file_grant(decision: GrantDecision, path: &Path) -> Grant {
    Grant { decision, target: GrantTarget::File(path.to_path_buf()), scope: GrantScope::Once }
}

/// An approval handler answering every slot request with `answer`'s choice
/// (the first offered choice with that decision and scope), recording what
/// it was asked.
struct Answering {
    answer: Option<(GrantDecision, GrantScope)>,
    asked: Mutex<Vec<SlotGrantRequest>>,
}

impl Answering {
    fn new(answer: Option<(GrantDecision, GrantScope)>) -> Arc<Self> {
        Arc::new(Self { answer, asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<SlotGrantRequest> {
        self.asked.lock().expect("asked").clone()
    }
}

impl ShellApprovalHandler for Answering {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::Deny)
    }

    fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        self.asked.lock().expect("asked").extend(requests.iter().cloned());
        Ok(requests
            .iter()
            .map(|request| {
                let Some((decision, scope)) = self.answer else {
                    return SlotGrantDecision::Unanswered;
                };
                request
                    .choices
                    .iter()
                    .find(|grant| grant.decision == decision && grant.scope == scope)
                    .map_or(SlotGrantDecision::Unanswered, SlotGrantDecision::from_grant)
            })
            .collect())
    }
}

#[test]
fn a_user_entry_admits_the_file_without_asking() {
    let home = Home::new();
    let handler = Answering::new(None);
    let session = home.session(&DataRuntime::new(), home.permissions(&["~/data/*.yaml"])).with_handler(handler.clone());
    assert_eq!(home.os(session), Ok(json!(["macOS", "Linux", "Windows"])));
    assert!(handler.asked().is_empty(), "{:?}", handler.asked());
}

#[test]
fn the_user_default_admits_files_beside_the_reading_schema_only() {
    let home = Home::new();
    home.write("schemas/os.yaml", "- Haiku\n");
    home.write("schemas/beside.yaml", &schema_with("load_data(\"./os.yaml\", \"string[]\")"));
    // No `.darkmatter.yaml`: the user scope's default `./*.<ext>` applies,
    // judged from the schema file doing the read.
    let beside = home.os_with(home.session(&DataRuntime::new(), home.captured()), "schemas/beside.yaml");
    assert_eq!(beside, Ok(json!(["Haiku"])));
    assert_eq!(home.os(home.session(&DataRuntime::new(), home.captured())), Err(vec![APPROVAL_REQUIRED.to_string()]));
}

#[test]
fn always_allow_is_offered_only_when_permissions_can_hold_it() {
    let home = Home::new();
    let handler = Answering::new(None);
    let unconfigured = SlotSession::new(DataRuntime::new()).with_handler(handler.clone());
    assert_eq!(home.os(unconfigured), Err(vec![APPROVAL_REQUIRED.to_string()]));
    let configured = home.session(&DataRuntime::new(), home.permissions(&[])).with_handler(handler.clone());
    assert_eq!(home.os(configured), Err(vec![APPROVAL_REQUIRED.to_string()]));
    let data = GrantTarget::File(home.path("data/os.yaml"));
    let scopes: Vec<Vec<(GrantDecision, GrantScope)>> = handler
        .asked()
        .iter()
        .map(|request| {
            assert!(request.choices.iter().all(|grant| grant.target == data), "{request:?}");
            request.choices.iter().map(|grant| (grant.decision, grant.scope)).collect()
        })
        .collect();
    assert_eq!(
        scopes,
        [
            vec![(GrantDecision::Allow, GrantScope::Once), (GrantDecision::Deny, GrantScope::Once)],
            vec![
                (GrantDecision::Allow, GrantScope::Once),
                (GrantDecision::Allow, GrantScope::Persisted),
                (GrantDecision::Deny, GrantScope::Once),
            ],
        ]
    );
}

#[test]
fn an_approval_for_this_run_overrides_an_exclusion() {
    let home = Home::new();
    let config = home.permissions(&["~/data/*.yaml", "!~/data/os.yaml"]);
    assert_eq!(home.os(home.session(&DataRuntime::new(), Arc::clone(&config))), Err(vec![APPROVAL_REQUIRED.to_string()]));
    let approved = home
        .session(&DataRuntime::new(), config)
        .with_run_grants([file_grant(GrantDecision::Allow, &home.path("data/os.yaml"))]);
    assert_eq!(home.os(approved), Ok(json!(["macOS", "Linux", "Windows"])));
}

#[test]
fn a_deny_for_this_run_beats_the_permissions() {
    let home = Home::new();
    let denied = home
        .session(&DataRuntime::new(), home.permissions(&["~/data/*.yaml"]))
        .with_run_grants([file_grant(GrantDecision::Deny, &home.path("data/os.yaml"))]);
    assert_eq!(home.os(denied), Err(vec!["dm.schema.slot_denied".to_string()]));
}

#[test]
fn an_excluded_files_always_allow_applies_for_the_run_but_is_not_saved() {
    let home = Home::new();
    let config = home.permissions(&["~/data/*.yaml", "!~/data/os.yaml"]);
    let before = std::fs::read(home.path(".darkmatter.yaml")).expect("config");
    let handler = Answering::new(Some((GrantDecision::Allow, GrantScope::Persisted)));
    let session = Arc::new(home.session(&DataRuntime::new(), config).with_handler(handler.clone()));
    let document = home.path("doc.md");
    std::fs::write(&document, "---\n$schema: ./schemas/sandbox.yaml\nos: macOS\n---\n").expect("document");
    let markdown = Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document));
    let schemas = DarkmatterSchemas::new(home.context()).with_slot_session(Arc::clone(&session));
    let effective = schemas.effective_for(&markdown).expect("allowed for this run").expect("a schema");
    assert_eq!(members(&effective.json_schema["properties"]["os"]), json!(["macOS", "Linux", "Windows"]));
    assert_eq!(handler.asked().len(), 1);
    assert_eq!(std::fs::read(home.path(".darkmatter.yaml")).expect("config"), before, "the exclusion stays and nothing is saved");
    let warnings = session.take_not_saved_warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].code.as_deref(), Some("dm.config.not_saved"));
    assert!(warnings[0].message.contains("!~/data/os.yaml"), "{}", warnings[0].message);
}

#[test]
fn always_allow_saves_an_exact_portable_entry_and_keeps_every_other_byte() {
    let home = Home::reading("load_data(\"../data/a[1].yaml\", \"string[]\")");
    home.write("data/a[1].yaml", OS_LIST);
    let original = "# user settings\npermissions:\n    # schema reads\n    file-read:\n        - \"~/notes/*.md\" # notes\n    network: deny\n";
    home.write(".darkmatter.yaml", original);
    let handler = Answering::new(Some((GrantDecision::Allow, GrantScope::Persisted)));
    let session = home.session(&DataRuntime::new(), home.captured()).with_handler(handler.clone());
    assert_eq!(home.os(session), Ok(json!(["macOS", "Linux", "Windows"])), "allowed for this run");

    let saved = std::fs::read_to_string(home.path(".darkmatter.yaml")).expect("config");
    let entry = format!("~/data/{}", biscuit_file::GlobReference::escape("a[1].yaml"));
    let added: Vec<&str> = saved.lines().filter(|line| !original.lines().any(|kept| kept == *line)).collect();
    assert_eq!(added.len(), 1, "one line is added:\n{saved}");
    assert!(added[0].contains(&entry), "{entry} in {added:?}");
    let kept: Vec<&str> = saved.lines().filter(|line| original.lines().any(|kept| kept == *line)).collect();
    assert_eq!(kept, original.lines().collect::<Vec<_>>(), "every other line kept in order:\n{saved}");

    // Read again: the saved entry pre-approves the read for a new command.
    let asked_before = handler.asked().len();
    let again = home.session(&DataRuntime::new(), home.captured()).with_handler(handler.clone());
    assert_eq!(home.os(again), Ok(json!(["macOS", "Linux", "Windows"])));
    assert_eq!(handler.asked().len(), asked_before, "nothing asked once saved");
}

#[test]
fn invalid_permissions_refuse_every_read_even_an_approved_one() {
    let home = Home::new();
    home.write(".darkmatter.yaml", "permissions:\n    file-read:\n        - /absolute/data.yaml\n");
    let config = home.captured();
    assert!(config.policy().is_err(), "an absolute entry is invalid");
    let session = home
        .session(&DataRuntime::new(), Arc::clone(&config))
        .with_run_grants([file_grant(GrantDecision::Allow, &home.path("data/os.yaml"))]);
    let codes = home.os(session).expect_err("refused");
    assert!(codes.iter().all(|code| code.starts_with("dm.config.")), "{codes:?}");

    let listing = Home::reading("find_files(\"../data/*.yaml\")");
    listing.write(".darkmatter.yaml", "permissions:\n    file-read:\n        - /absolute/data.yaml\n");
    let codes = listing.os(listing.session(&DataRuntime::new(), listing.captured())).expect_err("a listing is refused too");
    assert!(codes.iter().all(|code| code.starts_with("dm.config.")), "{codes:?}");
}

#[test]
fn a_listing_returns_only_permitted_files_and_never_asks() {
    let home = Home::reading("find_files(\"../data/*.yaml\")");
    home.write("data/b.yaml", OS_LIST);
    let config = home.permissions(&["~/data/os.yaml"]);
    let handler = Answering::new(Some((GrantDecision::Allow, GrantScope::Once)));
    let entry = |name: &str| Value::String(biscuit_file::to_forward_slash_string(&home.path(&format!("data/{name}"))));

    let session = home.session(&DataRuntime::new(), Arc::clone(&config)).with_handler(handler.clone());
    assert_eq!(home.os(session), Ok(json!([entry("os.yaml")])));
    assert!(handler.asked().is_empty(), "nothing is asked about a listing or its files: {:?}", handler.asked());

    let approved = home
        .session(&DataRuntime::new(), config)
        .with_run_grants([file_grant(GrantDecision::Allow, &home.path("data/b.yaml"))]);
    assert_eq!(home.os(approved), Ok(json!([entry("b.yaml"), entry("os.yaml")])), "an approval for this run lists it too");
}

/// A read allowed for one command never reaches a later command without the
/// permission through the shared data runtime: the gate decides before the
/// memo (or a cached resolution) is consulted.
fn assert_decided_before_the_memo(expression: &str, files: &[(&str, &str)], allowed: &[&str], narrowed: &[&str]) {
    let home = Home::reading(expression);
    for (relative, text) in files {
        home.write(relative, text);
    }
    let data = DataRuntime::new();
    let first = home.os(home.session(&data, home.permissions(allowed)));
    assert!(first.is_ok(), "{expression}: allowed: {first:?}");
    let reads = data.stats().reads;
    let second = home.os(home.session(&data, home.permissions(narrowed)));
    assert_eq!(second, Err(vec![APPROVAL_REQUIRED.to_string()]), "{expression}: narrowed");
    assert_eq!(data.stats().reads, reads, "{expression}: nothing read again");
}

#[test]
fn a_content_read_is_decided_before_the_memo() {
    assert_decided_before_the_memo("load_data(\"../data/os.yaml\", \"string[]\")", &[], &["~/data/*.yaml"], &[]);
}

#[test]
fn a_probe_is_decided_before_the_memo() {
    assert_decided_before_the_memo(
        "file_exists(\"../data/os.yaml\") ? [\"yes\"] : [\"no\"]",
        &[],
        &["~/data/*.yaml"],
        &[],
    );
}

#[test]
fn a_contract_import_is_decided_before_the_memo() {
    assert_decided_before_the_memo(
        "load_data(\"../data/os.yaml\", \"Os@./types.yaml\")",
        &[("schemas/types.yaml", "kind: schema\ntypes:\n    Os: string[]\n")],
        &["~/data/*.yaml", "~/schemas/types.yaml"],
        &["~/data/*.yaml"],
    );
}

#[test]
fn a_validate_schema_read_is_decided_before_the_memo() {
    assert_decided_before_the_memo(
        "validate_schema(\"./page.md\") ? [\"valid\"] : [\"invalid\"]",
        &[
            ("schemas/page.md", "---\n$schema: ./page-schema.yaml\ntitle: Page\n---\n"),
            ("schemas/page-schema.yaml", "kind: schema\n$schema:\n    title: string(required)\n"),
        ],
        &["~/schemas/page*"],
        &["~/schemas/page.md"],
    );
}

/// The repository default `&**/*.yaml` admits an in-repository read with no
/// prompt; a widening that is not committed is not effective, and takes
/// effect once committed.
#[test]
fn the_repository_default_and_the_commit_rule_decide_in_repository_reads() {
    let home = Home::new();
    write_user_permissions(&home.root, &[]);
    let work = home.path("work");
    home.write("work/schemas/sandbox.yaml", &schema_with("load_data(\"../data/os.yaml\", \"string[]\")"));
    home.write("work/data/os.yaml", OS_LIST);
    home.write("work/README.md", "work\n");
    git(&work, &["init", "-q", "-b", "main"]);
    git(&work, &["add", "README.md"]);
    git(&work, &["commit", "-q", "-m", "readme"]);
    let context = FileResolutionContext::from_snapshot(&work, Some(home.root.clone()), HashMap::new()).with_repository_root(&work);
    let resolve = || {
        let document = work.join("doc.md");
        std::fs::write(&document, "---\n$schema: ./schemas/sandbox.yaml\nos: macOS\n---\n").expect("document");
        let markdown =
            Markdown::try_from(document.as_path()).expect("document").with_source(ComposeSource::File(document.clone()));
        let session = SlotSession::for_context(DataRuntime::new(), &context);
        match DarkmatterSchemas::new(context.clone()).with_slot_session(Arc::new(session)).effective_for(&markdown) {
            Ok(effective) => Ok(members(&effective.expect("a schema").json_schema["properties"]["os"])),
            Err(error) => Err(codes(&error, &document)),
        }
    };
    assert_eq!(resolve(), Ok(json!(["macOS", "Linux", "Windows"])), "the repository default, with no repository file");

    std::fs::write(work.join(".darkmatter.yaml"), crate::slot_support::NO_READS).expect("repository config");
    git(&work, &["add", ".darkmatter.yaml"]);
    git(&work, &["commit", "-q", "-m", "narrow"]);
    assert_eq!(resolve(), Err(vec![APPROVAL_REQUIRED.to_string()]), "a committed narrowing applies");

    std::fs::write(work.join(".darkmatter.yaml"), crate::slot_support::file_read_config(&["&**/*.yaml"])).expect("widen");
    assert_eq!(resolve(), Err(vec![APPROVAL_REQUIRED.to_string()]), "an uncommitted widening is not effective");
    git(&work, &["commit", "-q", "-am", "widen"]);
    assert_eq!(resolve(), Ok(json!(["macOS", "Linux", "Windows"])), "committed, the widening applies");
}

mod remote {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// URL reads follow the network consent and approvals for this run;
    /// nothing persists a URL, and a retired site approval is ignored.
    /// `network: ask` leaves an unconsented URL to approval (asking a
    /// handler); `network: deny` refuses it without asking, though an
    /// approval or host for this run still allows it.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_url_read_follows_network_consent_and_approvals_for_this_run() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/os.yaml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(OS_LIST))
            .mount(&server)
            .await;
        let url = url::Url::parse(&format!("{}/os.yaml", server.uri())).expect("url");
        let outcomes = tokio::task::spawn_blocking({
            let url = url.clone();
            move || {
                let home = Home::reading(&format!("load_data(\"{url}\", \"string[]\")"));
                let config = home.permissions(&[]);
                let session = || home.session(&DataRuntime::new(), Arc::clone(&config)).with_network(NetworkPermission::Ask);
                let denying = || home.session(&DataRuntime::new(), Arc::clone(&config));
                let once = |target: GrantTarget| {
                    session().with_run_grants([Grant { decision: GrantDecision::Allow, target, scope: GrantScope::Once }])
                };
                let site = GrantTarget::Site { scheme: "http".into(), host: "127.0.0.1".into(), port: url.port() };
                let handler = Answering::new(None);
                vec![
                    home.os(session()),
                    home.os(once(site)),
                    home.os(once(GrantTarget::Url(url.clone()))),
                    home.os(session().with_allowed_hosts(vec!["127.0.0.1".into()])),
                    home.os(session().with_network(NetworkPermission::Allow)),
                    home.os(session().with_handler(handler.clone())).map(|_| {
                        let choices: Vec<_> = handler.asked().iter().flat_map(|request| request.choices.clone()).collect();
                        json!(choices.iter().map(|grant| format!("{:?}/{:?}", grant.decision, grant.scope)).collect::<Vec<_>>())
                    }),
                    {
                        let handler = Answering::new(None);
                        let refused = home.os(session().with_handler(handler.clone()));
                        assert_eq!(refused, Err(vec![APPROVAL_REQUIRED.to_string()]));
                        let choices: Vec<_> = handler.asked().iter().flat_map(|request| request.choices.clone()).collect();
                        Ok(json!(choices.iter().map(|grant| format!("{:?}/{:?}", grant.decision, grant.scope)).collect::<Vec<_>>()))
                    },
                    {
                        let handler = Answering::new(None);
                        let refused = home.os(denying().with_handler(handler.clone()));
                        assert!(handler.asked().is_empty(), "`deny` never asks");
                        refused
                    },
                    home.os(denying().with_run_grants([Grant {
                        decision: GrantDecision::Allow,
                        target: GrantTarget::Url(url.clone()),
                        scope: GrantScope::Once,
                    }])),
                    home.os(denying().with_allowed_hosts(vec!["127.0.0.1".into()])),
                ]
            }
        })
        .await
        .expect("blocking resolutions");
        let refused = Err(vec![APPROVAL_REQUIRED.to_string()]);
        let fetched = Ok(json!(["macOS", "Linux", "Windows"]));
        assert_eq!(outcomes[0], refused, "no consent");
        assert_eq!(outcomes[1], refused, "a retired site approval is ignored");
        assert_eq!(outcomes[2], fetched, "the exact URL approved for this run");
        assert_eq!(outcomes[3], fetched, "--allow host=");
        assert_eq!(outcomes[4], fetched, "network: allow");
        assert_eq!(outcomes[5], refused, "unanswered");
        assert_eq!(outcomes[6], Ok(json!(["Allow/Once", "Deny/Once"])), "a URL is offered for this run only");
        assert_eq!(outcomes[7], Err(vec![NETWORK_DENIED.to_string()]), "`deny` refuses without asking");
        assert_eq!(outcomes[8], fetched, "`deny` with the exact URL approved for this run");
        assert_eq!(outcomes[9], fetched, "`deny` with --allow host=");
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 5, "a refused read never reaches the network");
    }
}
