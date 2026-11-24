//! Claudine's installed Darkmatter integration, end to end over a temporary
//! home: `setup_claudine` publishes the host registration, writes Claudine's
//! configuration, hands off `permissions`, and registers the manifest in the
//! user's `~/.darkmatter.yaml`; Darkmatter's own loader and selection (the
//! library path `md` and DMLS share: `RegisteredHosts::load`, `select`,
//! `with_registered_hosts`) then recognize Claudine documents outside
//! `claudine/`. Claudine's own launch verdict never reads a registration.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::composition::registration::{
    FileKind, LEGACY_MANIFEST_REFERENCE, MANIFEST_REFERENCE, PublishOutcome, PublishedFile, PublishError, PublishHooks,
    RegistrationInventory, legacy_manifest_path, legacy_revisions_dir, manifest_path, output_root, publish, publish_with,
};
#[cfg(unix)]
use claudine::composition::registration::{CleanupError, Obstruction};
use claudine::composition::{TriggerFindings, pre_validate_schema, resolve_composition_source_in_context};
use claudine::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};
use claudine::config::persist::{SaveOutcome, initialize_config};
use claudine::config::schema::ConfigScope;
use claudine::config::setup::{ClaudineSetup, PermissionsHandoff, SetupError, setup_claudine, setup_with_darkmatter};
use darkmatter::config::init::{InitAction, InitError, InitOptions, init_user};
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::registration::{HostSelection, RegisteredHosts, select};
use darkmatter::lifecycle::md_host;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};
use simplified_schema::yaml_documents::load_yaml_document;

/// A Claudine document by its `start` key, with a `model` that Claudine's
/// shipped schema (a string, or a list of strings) rejects. A number would
/// not do: it is coerced to the string `"7"`.
const MAPPING_MODEL: &str = "---\nstart: [stop]\nmodel: { name: sonnet }\n---\nBody\n";

/// Shared Darkmatter events only: never a Claudine document on its own.
const SHARED_EVENTS_ONLY: &str = "---\nfinalize: [stop]\n---\nBody\n";

/// A temporary root holding `home/`, `work/`, and `repo/`; nothing is read
/// from the process environment or the checkout.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        for name in ["home", "work", "repo"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        Self { _dir: dir, root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn darkmatter(&self) -> PathBuf {
        self.home().join(".darkmatter.yaml")
    }

    fn claudine(&self) -> PathBuf {
        self.home().join(".claudine/config.yaml")
    }

    fn write(&self, path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.work(), Some(self.home()), HashMap::new())
    }

    /// Runs `git` in `repo/` with a fixed identity and no signing.
    fn git(&self, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(["-c", "user.name=test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(self.repo())
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    /// `repo/` as a `git init`-ed repository, and a context anchored in it.
    fn repository(&self) -> FileResolutionContext {
        self.git(&["init", "--quiet"]);
        FileResolutionContext::from_snapshot(self.repo(), Some(self.home()), HashMap::new())
            .with_repository_root(self.repo())
    }

    /// User-scope setup with Claudine's default configuration.
    fn setup(&self) -> Result<ClaudineSetup, SetupError> {
        setup_claudine(&self.context(), ConfigScope::User, write_default)
    }

    /// The registrations Darkmatter loads in `context`.
    fn hosts(&self, context: &FileResolutionContext) -> Arc<RegisteredHosts> {
        let snapshot = ConfigSnapshot::load(context, None, ConfigLoadMode::Passive).unwrap();
        Arc::new(RegisteredHosts::load(&snapshot, context))
    }

    /// Writes `text` as `relative` under the root and returns its selection
    /// among `hosts`, as `md` and DMLS select it.
    fn selection(&self, hosts: &RegisteredHosts, relative: &str, text: &str) -> HostSelection {
        let path = self.root.join(relative);
        self.write(&path, text);
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let frontmatter = darkmatter::markdown::schemas::frontmatter_as_json(&markdown);
        let selected = select(Some(hosts), &frontmatter, Some(&path), &self.context().for_source(&path));
        assert!(selected.diagnostics.is_empty(), "{:#?}", selected.diagnostics);
        selected.selection
    }

    /// Composes `text` as `work/doc.md` through Darkmatter, with `hosts`
    /// registered when given.
    fn compose(&self, text: &str, hosts: Option<Arc<RegisteredHosts>>) -> Result<Markdown, MarkdownError> {
        let path = self.work().join("doc.md");
        self.write(&path, text);
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let mut options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.work(), ""))
            .with_source_file(&path)
            .with_trigger_schemas(true);
        if let Some(hosts) = hosts {
            options = options.with_registered_hosts(hosts);
        }
        let request = ComposeRequest::with_context(options, self.context().for_source(&path)).expect("a request");
        markdown.compose_with(&request).map(|(composed, _)| composed)
    }

    /// Every file under the root, with its bytes.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = BTreeMap::new();
        let mut pending = vec![self.root.clone()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    files.insert(path.clone(), std::fs::read(&path).unwrap());
                }
            }
        }
        files
    }
}

fn write_default(path: &Path) -> claudine::error::Result<SaveOutcome> {
    initialize_config(path, ConfigScope::User, &ClaudineConfig::default(), &ClaudineConfig::default())
}

fn document(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap();
    load_yaml_document(&text, path).unwrap().json
}

/// The one loaded profile is Claudine's, and nothing was refused.
fn assert_one_claudine_profile(hosts: &RegisteredHosts) {
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());
    let profiles = &hosts.loaded().profiles;
    assert_eq!(profiles.len(), 1, "one profile");
    assert_eq!(profiles[0].host(), "claudine");
}

fn schema_problem_paths(error: &MarkdownError) -> Vec<String> {
    let MarkdownError::SchemaValidationFailed { problems, .. } = error else { panic!("a schema failure: {error:?}") };
    problems.iter().map(|problem| problem.path.clone()).collect()
}

#[test]
fn installed_registration_setup_publishes_configures_and_registers() {
    let fixture = Fixture::new();

    let setup = fixture.setup().unwrap();

    let manifest = manifest_path(&fixture.home());
    assert_eq!(setup.publication.manifest(), manifest);
    assert!(manifest.is_file(), "the manifest exists");
    assert!(output_root(&fixture.home()).join("schemas").is_dir(), "the members exist");
    let darkmatter = setup.darkmatter.as_ref().unwrap();
    assert_eq!(darkmatter.permissions, PermissionsHandoff::Referenced);
    assert!(document(&fixture.claudine())["permissions"].is_object(), "Claudine holds the block");
    let registered = document(&fixture.darkmatter());
    assert_eq!(registered["permissions"], "~/.claudine/config.yaml");
    assert_eq!(registered["register"], json!([MANIFEST_REFERENCE]));
    assert_eq!(setup.registration.as_ref().unwrap().path, fixture.darkmatter());

    assert_one_claudine_profile(&fixture.hosts(&fixture.context()));
    assert!(!fixture.home().join("schemas").exists(), "nothing is copied into ~/schemas");
}

#[test]
fn installed_registration_md_init_in_either_order_ends_equal() {
    let md_first = Fixture::new();
    init_user(&md_first.context(), InitOptions::default()).unwrap();
    md_first.setup().unwrap();

    let claudine_first = Fixture::new();
    claudine_first.setup().unwrap();
    init_user(&claudine_first.context(), InitOptions::default()).unwrap();

    assert_eq!(document(&md_first.darkmatter()), document(&claudine_first.darkmatter()));
    assert_eq!(document(&md_first.darkmatter())["register"], json!([MANIFEST_REFERENCE]), "one registration");
    for fixture in [&md_first, &claudine_first] {
        assert_one_claudine_profile(&fixture.hosts(&fixture.context()));
    }
}

#[test]
fn installed_registration_repeated_setup_writes_nothing() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let before = fixture.files();

    let again = fixture.setup().unwrap();

    assert!(matches!(again.publication, PublishOutcome::Current { .. }));
    assert_eq!(again.darkmatter.unwrap().darkmatter_action, InitAction::Unchanged);
    assert_eq!(again.registration.unwrap().action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before, "byte for byte");
}

#[test]
fn installed_registration_keeps_existing_entries_and_comments() {
    let fixture = Fixture::new();
    fixture.write(&fixture.darkmatter(), "register:\n  # my own host\n  - ~/hosts/mine.yaml   # kept\n");

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.registration.unwrap().action, InitAction::Updated);
    let text = std::fs::read_to_string(fixture.darkmatter()).unwrap();
    assert!(text.contains("  # my own host\n  - ~/hosts/mine.yaml   # kept\n"), "{text}");
    assert_eq!(document(&fixture.darkmatter())["register"], json!(["~/hosts/mine.yaml", MANIFEST_REFERENCE]));
}

#[test]
fn installed_registration_same_manifest_spelled_absolutely_becomes_the_one_canonical_entry() {
    let fixture = Fixture::new();
    let absolute = manifest_path(&fixture.home());
    let quoted = serde_json::to_string(absolute.to_str().unwrap()).unwrap();
    fixture.write(&fixture.darkmatter(), &format!("register:\n  - {quoted}\n"));

    let setup = fixture.setup().unwrap();

    let registration = setup.registration.unwrap();
    assert_eq!(registration.action, InitAction::Updated);
    assert_eq!(registration.replaced, vec![absolute.to_str().unwrap().to_string()]);
    assert_eq!(document(&fixture.darkmatter())["register"], json!([MANIFEST_REFERENCE]));
    assert_one_claudine_profile(&fixture.hosts(&fixture.context()));
}

#[test]
fn installed_registration_repository_scope_registers_in_the_user_file_only() {
    let fixture = Fixture::new();
    let context = fixture.repository();

    let setup = setup_claudine(&context, ConfigScope::Repository, |path| {
        initialize_config(path, ConfigScope::Repository, &RepoOverrideConfig::default(), &RepoOverrideConfig::default())
    })
    .unwrap();

    let darkmatter = setup.darkmatter.unwrap();
    assert_eq!(darkmatter.permissions, PermissionsHandoff::Referenced);
    let repository = document(&fixture.repo().join(".darkmatter.yaml"));
    assert_eq!(repository["permissions"], "./.claudine/config.yaml");
    assert_eq!(repository["register"], json!([]), "the repository file registers nothing");
    assert!(document(&fixture.repo().join(".claudine/config.yaml"))["permissions"].is_object());
    assert_eq!(setup.registration.unwrap().path, fixture.darkmatter());
    assert_eq!(document(&fixture.darkmatter())["register"], json!([MANIFEST_REFERENCE]));
    assert!(!fixture.claudine().exists(), "the user's Claudine configuration is not written");
}

#[test]
fn installed_registration_failed_publication_writes_no_configuration() {
    let fixture = Fixture::new();
    fixture.write(&output_root(&fixture.home()), "an obstruction");

    let error = fixture.setup().unwrap_err();

    assert!(matches!(error, SetupError::Publish { .. }), "{error}");
    assert!(!fixture.claudine().exists(), "Claudine's configuration is not written");
    assert!(!fixture.darkmatter().exists(), "Darkmatter's configuration is not written");
    assert!(!manifest_path(&fixture.home()).exists(), "no manifest is published");

    std::fs::remove_file(output_root(&fixture.home())).unwrap();
    let setup = fixture.setup().unwrap();
    assert!(setup.darkmatter.is_ok() && setup.registration.is_ok(), "{setup:?}");
    assert_one_claudine_profile(&fixture.hosts(&fixture.context()));
}

/// No configuration shape lets the handshake succeed while registration
/// refuses: a `register` scalar fails the handshake's own validated write
/// too. The registration's write is failed instead, after a handshake that
/// has nothing left to write.
#[test]
fn installed_registration_failed_registration_is_reported_after_claudine_is_configured() {
    let fixture = Fixture::new();
    setup_with_darkmatter(&fixture.context(), ConfigScope::User, write_default).unwrap();
    let darkmatter_before = std::fs::read(fixture.darkmatter()).unwrap();
    let claudine_before = std::fs::read(fixture.claudine()).unwrap();

    let setup = {
        let _hook = darkmatter::config::write::test_hooks::fail_next_replacement();
        fixture.setup().unwrap()
    };

    assert!(matches!(setup.publication, PublishOutcome::Published { .. }), "{:?}", setup.publication);
    assert_eq!(setup.darkmatter.as_ref().unwrap().permissions, PermissionsHandoff::AlreadyReferenced);
    let error = setup.registration.unwrap_err();
    assert!(matches!(error, InitError::Write(_)), "{error}");
    assert_eq!(std::fs::read(fixture.claudine()).unwrap(), claudine_before, "Claudine's configuration stands");
    assert_eq!(std::fs::read(fixture.darkmatter()).unwrap(), darkmatter_before, "the failed write left the file");

    let retried = fixture.setup().unwrap();
    assert_eq!(retried.registration.unwrap().action, InitAction::Updated, "a retry registers");
    assert_one_claudine_profile(&fixture.hosts(&fixture.context()));
}

#[test]
fn installed_registration_identifies_claudine_documents_outside_claudine() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let hosts = fixture.hosts(&fixture.context());

    let selection = fixture.selection(&hosts, "work/notes/run.md", MAPPING_MODEL);
    let HostSelection::Registered(profile) = selection else { panic!("claudine: {selection:?}") };
    assert_eq!(profile.host(), "claudine");
    assert!(
        matches!(fixture.selection(&hosts, "work/notes/shared.md", SHARED_EVENTS_ONLY), HostSelection::Md),
        "shared events alone are an md document"
    );

    let error = fixture.compose(MAPPING_MODEL, Some(Arc::clone(&hosts))).expect_err("Claudine's schema judges it");
    assert_eq!(schema_problem_paths(&error), ["/model"]);
    if let Err(error) = fixture.compose(MAPPING_MODEL, None) {
        panic!("without a registration nothing types `model`: {error:?}");
    }
}

#[test]
fn installed_registration_md_compose_judges_an_event_it_never_runs() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let hosts = fixture.hosts(&fixture.context());
    let text = "---\nagent: claude\nloop: [1, 2]\n---\nBody\n";
    let HostSelection::Registered(profile) = fixture.selection(&hosts, "work/doc.md", text) else {
        panic!("`agent` identifies Claudine")
    };
    let path = fixture.work().join("doc.md");
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    // As `md compose` prepares a Claudine document: Claudine's event keys
    // (`loop` among them) deferred, the registered layer, the selected view.
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&fixture.work(), ""))
        .with_source_file(&path)
        .with_trigger_schemas(true)
        .with_exclude_keys(md_host::foreign_deferred_keys(profile.lifecycle().registry()))
        .with_registered_hosts(Arc::clone(&hosts))
        .with_selected_host(profile);
    let request = ComposeRequest::with_context(options, fixture.context().for_source(&path)).expect("a request");

    let error = markdown.compose_with(&request).expect_err("Claudine's schema rejects `loop: [1, 2]`");

    assert!(schema_problem_paths(&error).iter().any(|path| path == "/loop"), "{error:?}");
}

/// The problem paths of `text`, written as `work/doc.md`, as the passive
/// reader (`md schema validate`, DMLS) judges it with `hosts` registered.
fn passive_problem_paths(fixture: &Fixture, hosts: &Arc<RegisteredHosts>, text: &str) -> Vec<String> {
    let path = fixture.work().join("doc.md");
    fixture.write(&path, text);
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let schemas = darkmatter::markdown::schemas::DarkmatterSchemas::new(fixture.context().for_source(&path))
        .with_registered_hosts(Arc::clone(hosts));
    let report = schemas.validate(&markdown).expect("the schema prepares");
    report.problems.iter().map(|problem| problem.path.clone()).collect()
}

/// `{selector}` identifies Claudine, and both readers judge the document by
/// Claudine's main schema: `pointer` is rejected passively and in composition.
fn assert_main_schema_rejects(selector: &str, invalid: &str, pointer: &str) {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let hosts = fixture.hosts(&fixture.context());
    let text = format!("---\n{selector}\n{invalid}\n---\nBody\n");
    let HostSelection::Registered(profile) = fixture.selection(&hosts, "work/doc.md", &text) else {
        panic!("`{selector}` identifies Claudine")
    };
    assert_eq!(profile.host(), "claudine");

    let passive = passive_problem_paths(&fixture, &hosts, &text);
    assert!(passive.iter().any(|path| path == pointer), "passive reader, `{selector}`: {passive:?}");

    let error = fixture.compose(&text, Some(Arc::clone(&hosts))).expect_err("composition rejects it");
    let composed = schema_problem_paths(&error);
    assert!(composed.iter().any(|path| path == pointer), "composition, `{selector}`: {composed:?}");
}

/// One test per identifying key: each selects Claudine's main schema, which
/// types `timeout` as a string.
macro_rules! main_schema_rejects_timeout {
    ($($name:ident: $selector:literal,)+) => {$(
        #[test]
        fn $name() {
            assert_main_schema_rejects($selector, "timeout: [1, 2]", "/timeout");
        }
    )+};
}

main_schema_rejects_timeout! {
    installed_registration_sequence_selects_the_main_schema: "sequence: []",
    installed_registration_start_selects_the_main_schema: "start: []",
    installed_registration_blocked_selects_the_main_schema: "blocked: []",
    installed_registration_success_selects_the_main_schema: "success: []",
    installed_registration_failure_selects_the_main_schema: "failure: []",
    installed_registration_loop_selects_the_main_schema: "loop: { max: 1 }",
    installed_registration_prompt_selects_the_main_schema: "prompt: hi",
    installed_registration_agent_selects_the_main_schema: "agent: claude",
    installed_registration_model_selects_the_main_schema: "model: sonnet",
}

#[test]
fn installed_registration_sequence_document_rejects_the_md_event_spelling() {
    assert_main_schema_rejects("sequence: []", "composed: [stop]", "/composed");
}

#[test]
fn installed_registration_agent_document_rejects_the_md_event_spelling() {
    assert_main_schema_rejects("agent: claude", "composed: [stop]", "/composed");
}

#[test]
fn installed_registration_shared_events_alone_keep_the_md_contract() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let hosts = fixture.hosts(&fixture.context());
    let text = "---\nfinalize: [stop]\ntimeout: [1, 2]\n---\nBody\n";
    assert!(matches!(fixture.selection(&hosts, "work/doc.md", text), HostSelection::Md), "an md document");
    let passive = passive_problem_paths(&fixture, &hosts, text);
    assert!(passive.is_empty(), "no Claudine schema types `timeout`: {passive:?}");
    if let Err(error) = fixture.compose(text, Some(hosts)) {
        panic!("no Claudine schema types `timeout`: {error:?}");
    }
}

/// A native number or boolean `sequence` selects Claudine and is refused at
/// `/sequence` by the passive reader and by composition: the reference arm
/// never converts a scalar to reference text. A list, written reference text,
/// and a quoted number are accepted, a mapping is refused, and `null` alone
/// leaves an md document.
#[test]
fn installed_registration_refuses_a_scalar_sequence_source() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let hosts = fixture.hosts(&fixture.context());
    for value in ["42", "true", "{}"] {
        let text = format!("---\nsequence: {value}\n---\nBody\n");
        let HostSelection::Registered(profile) = fixture.selection(&hosts, "work/doc.md", &text) else {
            panic!("`sequence: {value}` identifies Claudine")
        };
        assert_eq!(profile.host(), "claudine");
        assert_eq!(passive_problem_paths(&fixture, &hosts, &text), ["/sequence"], "passive reader, `{value}`");
        let error = fixture.compose(&text, Some(Arc::clone(&hosts))).expect_err("composition refuses it");
        assert_eq!(schema_problem_paths(&error), ["/sequence"], "composition, `{value}`");
    }
    for value in ["[]", "./steps.yaml", "\"42\""] {
        let text = format!("---\nsequence: {value}\n---\nBody\n");
        let passive = passive_problem_paths(&fixture, &hosts, &text);
        assert!(passive.is_empty(), "passive reader, `{value}`: {passive:?}");
        if let Err(error) = fixture.compose(&text, Some(Arc::clone(&hosts))) {
            panic!("composition, `{value}`: {error:?}");
        }
    }
    let text = "---\nsequence: null\n---\nBody\n";
    assert!(matches!(fixture.selection(&hosts, "work/doc.md", text), HostSelection::Md), "`null` alone is md");
}

/// Every Claudine text property whose runtime reader refuses a non-string
/// (`PromptPropertyWrongType`, `AgentHintWrongType`, `ModelHintWrongType`,
/// the harness timeout parser, the `exit_expressions` reader) is refused at
/// its own path by the passive reader and by composition, so a native number
/// or boolean is never converted to text the runtime would not accept.
#[test]
fn installed_registration_a_closed_document_schema_holds_over_the_inline_union() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let hosts = fixture.hosts(&fixture.context());
    let text = |rest: &str| format!("---\n$schema:\n  $constraints: {{ closed: true }}\n  prompt: string\nprompt: hi\n{rest}---\nBody\n");

    let refused = text("extra: hi\n");
    let HostSelection::Registered(profile) = fixture.selection(&hosts, "work/doc.md", &refused) else {
        panic!("`prompt` identifies Claudine")
    };
    assert_eq!(profile.host(), "claudine");
    let path = fixture.work().join("doc.md");
    fixture.write(&path, &refused);
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let report = darkmatter::markdown::schemas::DarkmatterSchemas::new(fixture.context().for_source(&path))
        .with_registered_hosts(Arc::clone(&hosts))
        .validate(&markdown)
        .expect("the schema prepares");
    let refused_keys: Vec<_> = report.problems.iter().map(|problem| problem.offending_property.as_deref()).collect();
    assert_eq!(refused_keys, [Some("extra")], "passive reader: {:#?}", report.problems);
    fixture.compose(&refused, Some(Arc::clone(&hosts))).expect_err("composition refuses `extra`");

    let accepted = text("");
    assert!(passive_problem_paths(&fixture, &hosts, &accepted).is_empty(), "the closed root admits what some layer declares");
    if let Err(error) = fixture.compose(&accepted, Some(Arc::clone(&hosts))) {
        panic!("composition: {error:?}");
    }
}

#[test]
fn installed_registration_refuses_a_scalar_where_claudine_reads_text() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    let hosts = fixture.hosts(&fixture.context());
    let refused = [
        ("prompt: 42", "/prompt"),
        ("prompt: true", "/prompt"),
        ("agent: 7", "/agent"),
        ("agent: [claude, 7]", "/agent"),
        ("model: 5", "/model"),
        ("model: [true]", "/model"),
        ("agent: claude\ntimeout: 30", "/timeout"),
        ("agent: claude\nstep_timeout: 30", "/step_timeout"),
        ("agent: claude\ntimeout_warn: 30", "/timeout_warn"),
        ("agent: claude\nstep_timeout_warn: 30", "/step_timeout_warn"),
        ("agent: claude\nexit_expressions: [{ pattern: 5 }]", "/exit_expressions"),
    ];
    for (frontmatter, pointer) in refused {
        let text = format!("---\n{frontmatter}\n---\nBody\n");
        let HostSelection::Registered(profile) = fixture.selection(&hosts, "work/doc.md", &text) else {
            panic!("`{frontmatter}` identifies Claudine")
        };
        assert_eq!(profile.host(), "claudine");
        let passive = passive_problem_paths(&fixture, &hosts, &text);
        assert!(!passive.is_empty() && passive.iter().all(|path| path.starts_with(pointer)), "passive reader, `{frontmatter}`: {passive:?}");
        let error = fixture.compose(&text, Some(Arc::clone(&hosts))).expect_err("composition refuses it");
        let composed = schema_problem_paths(&error);
        assert!(!composed.is_empty() && composed.iter().all(|path| path.starts_with(pointer)), "composition, `{frontmatter}`: {composed:?}");
    }
    let accepted = [
        "prompt: hi",
        "prompt: \"42\"",
        "agent: claude\nmodel: \"5\"",
        "agent: [claude, codex]",
        "agent: claude\ntimeout: 30m",
        "agent: claude\nexit_expressions: [{ pattern: \"5\" }]",
    ];
    for frontmatter in accepted {
        let text = format!("---\n{frontmatter}\n---\nBody\n");
        let passive = passive_problem_paths(&fixture, &hosts, &text);
        assert!(passive.is_empty(), "passive reader, `{frontmatter}`: {passive:?}");
        if let Err(error) = fixture.compose(&text, Some(Arc::clone(&hosts))) {
            panic!("composition, `{frontmatter}`: {error:?}");
        }
    }
    // The inline payload still requires exactly one of `prompt` and `sections`.
    for frontmatter in ["prompt: hi\nsections: { intro: text }", "kind: inline-compose"] {
        let text = format!("---\n{frontmatter}\n---\nBody\n");
        assert!(!passive_problem_paths(&fixture, &hosts, &text).is_empty(), "passive reader, `{frontmatter}`");
    }
    let text = "---\nsections: { intro: text }\n---\nBody\n";
    assert!(passive_problem_paths(&fixture, &hosts, text).is_empty(), "sections alone");
}

#[test]
fn installed_registration_upgraded_binary_refresh_changes_the_facts() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    fixture.compose(MAPPING_MODEL, Some(fixture.hosts(&fixture.context()))).expect_err("the installed registration rejects it");
    let root = output_root(&fixture.home());
    let original_files: BTreeMap<_, _> = fixture.files().into_iter().filter(|(path, _)| path.starts_with(&root)).collect();
    let shared_yaml = RegistrationInventory::claudine().file("schemas/shared.yaml").unwrap().text().unwrap();
    let declaration = "\"string(no-coerce) -> the model to run\"";
    assert!(shared_yaml.contains(declaration), "the embedded schema declares the model as a string");
    let upgraded = RegistrationInventory::claudine().clone().with_version("99.0.0").with_member(PublishedFile::new(
        "schemas/shared.yaml",
        FileKind::Library,
        None::<&'static str>,
        shared_yaml.replace(declaration, "\"object -> the model to run\"").into_bytes(),
    ));

    let outcome = publish(&fixture.home(), &upgraded).unwrap();

    let hosts = fixture.hosts(&fixture.context());
    assert_one_claudine_profile(&hosts);
    assert_eq!(hosts.loaded().profiles[0].version(), "99.0.0");
    if let Err(error) = fixture.compose(MAPPING_MODEL, Some(hosts)) {
        panic!("the upgraded registration accepts a mapping model: {error:?}");
    }
    assert_eq!(outcome.changes().unwrap().written, ["schemas/shared.yaml", "manifest.yaml"]);
    let after: BTreeMap<_, _> = fixture.files().into_iter().filter(|(path, _)| path.starts_with(&root)).collect();
    assert_eq!(after.keys().collect::<Vec<_>>(), original_files.keys().collect::<Vec<_>>(), "the same fixed paths");
    for (path, bytes) in &original_files {
        if !path.ends_with("shared.yaml") && !path.ends_with("manifest.yaml") {
            assert_eq!(&after[path], bytes, "{} is unchanged", path.display());
        }
    }
    assert!(!fixture.home().join("schemas").exists(), "nothing is copied into ~/schemas");
}

/// A repository registration of a hand-written `claudine` host: Claudine's
/// published set with its triggers replaced by one that rejects a string
/// `model`, which Claudine's own schema accepts.
fn install_hostile_claudine(fixture: &Fixture) {
    let hostile = RegistrationInventory::claudine()
        .clone()
        .with_version("6.6.6")
        .without_member("schemas/composition.yaml")
        .without_member("schemas/inline-composition.yaml")
        .without_member("schemas/sequence-document.yaml")
        .with_member(PublishedFile::new(
            "schemas/hostile.yaml",
            FileKind::Trigger,
            None::<&'static str>,
            b"kind: schema-trigger\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    model: \"number -> a hostile model type\"\n".to_vec(),
        ));
    for file in hostile.files() {
        fixture.write(&fixture.repo().join(".hostile").join(file.path()), file.text().unwrap());
    }
    fixture.write(&fixture.repo().join(".darkmatter.yaml"), "register:\n  - ./.hostile/manifest.yaml\n");
}

#[test]
fn installed_registration_claudine_runtime_ignores_a_hostile_repository_registration() {
    let fixture = Fixture::new();
    fixture.repository();
    install_hostile_claudine(&fixture);
    fixture.git(&["add", "."]);
    fixture.git(&["commit", "--quiet", "-m", "hostile registration"]);
    let path = fixture.repo().join("docs/run.md");
    fixture.write(&path, "---\nstart: [stop]\nmodel: sonnet\n---\nBody\n");
    let snapshot = RequestSnapshot::new(fixture.repo().join("docs")).with_home(Some(fixture.home()));

    // Darkmatter loads the hostile registration, and its trigger rejects
    // the document.
    let request = ComposeRequest::prepare(ComposeOptions::new().with_source_file(&path), &snapshot).expect("a request");
    let context = request.resolution_context().for_source(&path);
    let hosts = Arc::new(RegisteredHosts::load(request.config_snapshot(), request.resolution_context()));
    assert!(hosts.diagnostics().is_empty(), "{:#?}", hosts.diagnostics());
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let frontmatter = darkmatter::markdown::schemas::frontmatter_as_json(&markdown);
    let selected = select(Some(&hosts), &frontmatter, Some(&path), &context);
    let HostSelection::Registered(profile) = &selected.selection else { panic!("hostile: {:?}", selected.selection) };
    assert_eq!(profile.version(), "6.6.6");
    let effective = darkmatter::markdown::schemas::DarkmatterSchemas::new(context)
        .with_registered_hosts(Arc::clone(&hosts))
        .effective_for(&markdown)
        .unwrap()
        .expect("the hostile trigger applies");
    let report = effective.validate(&frontmatter);
    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/model"], "the hostile trigger rejects a string model");

    // Claudine's launch verdict judges with its embedded bundle only.
    let context = build_resolution_context(&snapshot).expect("fixture context");
    assert_eq!(context.repository_root(), Some(fixture.repo().as_path()), "the launch sees the repository");
    let verdict = || {
        let source = resolve_composition_source_in_context(path.to_str().unwrap(), &context).expect("the document resolves");
        // The rendered verdict: `CompositionError` is too large for a closure's `Err`.
        pre_validate_schema(&source, None, None, &context, &TriggerFindings::default())
            .map(drop)
            .map_err(|error| error.to_string())
    };
    if let Err(error) = verdict() {
        panic!("Claudine's own schema accepts a string model: {error}");
    }
    std::fs::write(&path, MAPPING_MODEL).unwrap();
    let error = verdict().expect_err("Claudine's own schema still rejects a mapping model");
    assert!(error.contains("/model"), "{error}");
}

// ── migration from the retired format 1 registration ────────────────────────

/// A home as the format 1 publisher left it: the retired manifest, a
/// revision tree two folders deep, and `~/.darkmatter.yaml` listing the
/// retired manifest between two entries of its own. Returns the
/// configuration text.
fn with_legacy_registration(fixture: &Fixture) -> String {
    let home = fixture.home();
    fixture.write(&legacy_manifest_path(&home), "kind: darkmatter-host\nformat_version: 1\n");
    let revisions = legacy_revisions_dir(&home);
    fixture.write(&revisions.join("0123abcd/schemas/claudine.yaml"), "kind: schema-trigger\n");
    fixture.write(&revisions.join("0123abcd/globals.yaml"), "kind: schema\n");
    fixture.write(&home.join("hosts/mine.yaml"), "kind: darkmatter-host\n");
    let text = format!(
        "# mine\nregister:\n  - ~/hosts/mine.yaml   # kept\n  - {LEGACY_MANIFEST_REFERENCE}   # Claudine's\n  # an unknown one\n  - ~/hosts/gone.yaml\n"
    );
    fixture.write(&fixture.darkmatter(), &text);
    text
}

/// `text` with the retired entry spelled as the new one.
fn migrated(text: &str) -> String {
    text.replace(LEGACY_MANIFEST_REFERENCE, MANIFEST_REFERENCE)
}

/// The `register` list of `~/.darkmatter.yaml`, as written.
fn register_list(fixture: &Fixture) -> Value {
    document(&fixture.darkmatter())["register"].clone()
}

#[test]
fn installed_registration_setup_migrates_a_legacy_home_and_removes_the_retired_files() {
    let fixture = Fixture::new();
    let original = with_legacy_registration(&fixture);
    let unrelated = fixture.home().join(".claudine/mcp/catalog.json");
    fixture.write(&unrelated, "{}\n");

    let setup = fixture.setup().unwrap();

    let registration = setup.registration.unwrap();
    assert_eq!(registration.action, InitAction::Updated);
    assert_eq!(registration.replaced, vec![LEGACY_MANIFEST_REFERENCE.to_string()]);
    let text = std::fs::read_to_string(fixture.darkmatter()).unwrap();
    assert!(text.starts_with(&migrated(&original)), "the entry is replaced in place, every other byte kept:\n{text}");
    assert_eq!(register_list(&fixture), json!(["~/hosts/mine.yaml", MANIFEST_REFERENCE, "~/hosts/gone.yaml"]));
    let cleanup = setup.cleanup.expect("cleanup ran").unwrap();
    assert_eq!(cleanup.removed, vec![legacy_revisions_dir(&fixture.home()), legacy_manifest_path(&fixture.home())]);
    assert!(!legacy_manifest_path(&fixture.home()).exists());
    assert!(!legacy_revisions_dir(&fixture.home()).exists());
    assert_eq!(std::fs::read_to_string(&unrelated).unwrap(), "{}\n", "nothing else under ~/.claudine is touched");

    let again = fixture.setup().unwrap();
    assert_eq!(again.registration.unwrap().action, InitAction::Unchanged);
    assert_eq!(again.cleanup.unwrap().unwrap().removed, Vec::<PathBuf>::new());
}

/// R4-F2.c3: setup stages the same registration edit as doctor's repair. An
/// inline `register` list it must append to is refused by the editor with a
/// located error naming the list, the retired layout is kept, and the block
/// form of the same list registers (the migration test above).
#[test]
fn installed_registration_setup_reports_an_inline_register_list_and_keeps_the_retired_layout() {
    let fixture = Fixture::new();
    with_legacy_registration(&fixture);
    fixture.write(&fixture.darkmatter(), &format!("register: [~/hosts/mine.yaml, {LEGACY_MANIFEST_REFERENCE}]\n"));

    let setup = fixture.setup().unwrap();

    let error = setup.registration.unwrap_err();
    let InitError::Write(darkmatter::config::write::ConfigWriteError::UnsupportedEdit { path, source }) = &error else {
        panic!("{error:?}")
    };
    assert_eq!(*path, fixture.darkmatter());
    assert_eq!(source.kind, darkmatter::yaml_edit::YamlEditErrorKind::FlowCollection);
    assert!(error.to_string().contains("a flow collection at `register`"), "{error}");
    assert!(setup.cleanup.is_none(), "nothing is cleaned up before the entry is written");
    assert!(legacy_manifest_path(&fixture.home()).exists() && legacy_revisions_dir(&fixture.home()).exists());
    assert_eq!(register_list(&fixture), json!(["~/hosts/mine.yaml", LEGACY_MANIFEST_REFERENCE]));
}

#[test]
fn installed_registration_setup_replaces_a_legacy_entry_whose_manifest_is_gone() {
    let fixture = Fixture::new();
    let absolute = biscuit_file::to_portable_string(&legacy_manifest_path(&fixture.home()));
    fixture.write(&fixture.darkmatter(), &format!("register:\n  - {absolute}\n  - {LEGACY_MANIFEST_REFERENCE}\n"));

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.registration.unwrap().replaced.len(), 2);
    assert_eq!(register_list(&fixture), json!([MANIFEST_REFERENCE]));
    assert_eq!(setup.cleanup.unwrap().unwrap().removed, Vec::<PathBuf>::new(), "nothing was left to remove");
    assert_one_claudine_profile(&fixture.hosts(&fixture.context()));
}

/// R5-F1.c2: beside the current entry and another host's, a retired entry
/// whose manifest is gone is removed and the other two are kept.
#[test]
fn installed_registration_setup_removes_a_retired_entry_beside_the_current_one_after_its_files_are_gone() {
    let fixture = Fixture::new();
    fixture.write(&fixture.home().join("hosts/mine.yaml"), "kind: darkmatter-host\n");
    fixture.write(
        &fixture.darkmatter(),
        &format!("register:\n  - ~/hosts/mine.yaml\n  - {MANIFEST_REFERENCE}\n  - {LEGACY_MANIFEST_REFERENCE}\n"),
    );

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.registration.unwrap().replaced, vec![LEGACY_MANIFEST_REFERENCE.to_string()]);
    assert_eq!(register_list(&fixture), json!(["~/hosts/mine.yaml", MANIFEST_REFERENCE]));
}

/// An interrupted `~/.darkmatter.yaml` write leaves the new manifest
/// published, the retired entry listed, and the retired files in place;
/// the next setup finishes the migration.
#[test]
fn installed_registration_setup_converges_after_an_interrupted_registration_write() {
    let fixture = Fixture::new();
    setup_with_darkmatter(&fixture.context(), ConfigScope::User, write_default).unwrap();
    let configured = std::fs::read_to_string(fixture.darkmatter()).unwrap();
    let legacy = configured.replace("register: []", &format!("register:\n    - {LEGACY_MANIFEST_REFERENCE}"));
    assert_ne!(legacy, configured, "the handshake wrote an empty list: {configured}");
    fixture.write(&fixture.darkmatter(), &legacy);
    fixture.write(&legacy_manifest_path(&fixture.home()), "kind: darkmatter-host\n");

    let interrupted = {
        let _hook = darkmatter::config::write::test_hooks::fail_next_replacement();
        fixture.setup().unwrap()
    };

    assert!(matches!(interrupted.registration, Err(InitError::Write(_))), "{:?}", interrupted.registration);
    assert!(interrupted.cleanup.is_none(), "nothing is removed before the entry is replaced");
    assert!(manifest_path(&fixture.home()).is_file(), "the new manifest is published");
    assert_eq!(std::fs::read_to_string(fixture.darkmatter()).unwrap(), legacy);
    assert!(legacy_manifest_path(&fixture.home()).is_file());

    let rerun = fixture.setup().unwrap();

    assert!(matches!(rerun.publication, PublishOutcome::Current { .. }));
    assert_eq!(rerun.registration.unwrap().action, InitAction::Updated);
    assert_eq!(register_list(&fixture), json!([MANIFEST_REFERENCE]));
    assert!(!legacy_manifest_path(&fixture.home()).exists());
    assert_one_claudine_profile(&fixture.hosts(&fixture.context()));
}

#[test]
fn installed_registration_permissions_conflict_keeps_both_blocks_and_still_registers() {
    let fixture = Fixture::new();
    let inline = "permissions:\n  network: allow\nregister: []\n";
    fixture.write(&fixture.darkmatter(), inline);
    fixture.write(&fixture.claudine(), "permissions:\n  network: deny\n");

    let setup = fixture.setup().unwrap();

    assert!(matches!(setup.darkmatter, Err(SetupError::Conflict { .. })), "{:?}", setup.darkmatter);
    assert_eq!(document(&fixture.darkmatter())["permissions"], json!({ "network": "allow" }), "the inline block stays");
    assert_eq!(document(&fixture.claudine())["permissions"], json!({ "network": "deny" }));
    assert_eq!(register_list(&fixture), json!([MANIFEST_REFERENCE]));
}

/// Where setup runs relative to the user file: a separate repository with
/// no `.darkmatter.yaml` (control), one whose `.darkmatter.yaml` links to
/// the user file, or the home itself as the repository root.
#[derive(Debug, Clone, Copy)]
enum Launch {
    SeparateRepository,
    #[cfg(unix)]
    LinkedRepositoryFile,
    HomeIsRepository,
}

fn launches() -> Vec<Launch> {
    let mut launches = vec![Launch::SeparateRepository, Launch::HomeIsRepository];
    #[cfg(unix)]
    launches.push(Launch::LinkedRepositoryFile);
    launches
}

impl Fixture {
    /// A context launched in the repository `launch` lays out.
    fn launched(&self, launch: Launch) -> FileResolutionContext {
        let root = match launch {
            Launch::SeparateRepository => self.repo(),
            #[cfg(unix)]
            Launch::LinkedRepositoryFile => {
                std::os::unix::fs::symlink(self.darkmatter(), self.repo().join(".darkmatter.yaml")).unwrap();
                self.repo()
            }
            Launch::HomeIsRepository => self.home(),
        };
        FileResolutionContext::from_snapshot(root.clone(), Some(self.home()), HashMap::new()).with_repository_root(&root)
    }
}

#[test]
fn installed_registration_registers_in_an_aliased_user_file_despite_a_permissions_conflict() {
    // A user-only grant (a home reference no repository file may hold) and
    // a policy both scopes accept.
    let user_only = "permissions:\n    file-read:\n        - ~/notes/*.md\n";
    let shared = "permissions:\n    network: deny\n";
    for launch in launches() {
        for policy in [user_only, shared] {
            let case = format!("{launch:?}, {policy:?}");
            let fixture = Fixture::new();
            fixture.write(&fixture.home().join(".other/manifest.yaml"), "kind: darkmatter-host\n");
            let before = format!("register:\n    # the other host\n    - ~/.other/manifest.yaml\n{policy}");
            fixture.write(&fixture.darkmatter(), &before);
            fixture.write(&fixture.claudine(), "permissions:\n    file-read:\n        - ~/elsewhere/*.txt\n");
            let context = fixture.launched(launch);

            let setup = setup_claudine(&context, ConfigScope::User, write_default).unwrap();

            assert!(matches!(setup.darkmatter, Err(SetupError::Conflict { .. })), "{case}: {:?}", setup.darkmatter);
            let registration = setup.registration.unwrap_or_else(|error| panic!("{case}: {error}"));
            assert_eq!(registration.action, InitAction::Updated, "{case}");
            let expected = format!("register:\n    # the other host\n    - ~/.other/manifest.yaml\n    - {MANIFEST_REFERENCE}\n{policy}");
            assert_eq!(std::fs::read_to_string(fixture.darkmatter()).unwrap(), expected, "{case}");
        }
    }
}

#[test]
fn installed_registration_third_party_permissions_reference_is_kept_and_the_manifest_registered() {
    let fixture = Fixture::new();
    fixture.write(&fixture.home().join(".other/config.yaml"), "permissions:\n  network: deny\n");
    fixture.write(&fixture.darkmatter(), "permissions: ~/.other/config.yaml\nregister: []\n");

    let setup = fixture.setup().unwrap();

    let handoff = setup.darkmatter.unwrap().permissions;
    assert_eq!(handoff, PermissionsHandoff::ThirdParty { reference: "~/.other/config.yaml".to_string() });
    assert_eq!(document(&fixture.darkmatter())["permissions"], "~/.other/config.yaml");
    assert_eq!(register_list(&fixture), json!([MANIFEST_REFERENCE]));
}

#[test]
fn installed_registration_inline_block_moves_to_an_absent_or_matching_destination_and_registers() {
    for claudine_before in [None, Some("permissions:\n  network: allow\n")] {
        let fixture = Fixture::new();
        fixture.write(&fixture.darkmatter(), "permissions:\n  network: allow\nregister: []\n");
        if let Some(text) = claudine_before {
            fixture.write(&fixture.claudine(), text);
        }

        let setup = fixture.setup().unwrap();

        assert_eq!(setup.darkmatter.unwrap().permissions, PermissionsHandoff::Transferred, "{claudine_before:?}");
        assert_eq!(document(&fixture.claudine())["permissions"]["network"], "allow");
        assert_eq!(document(&fixture.darkmatter())["permissions"], "~/.claudine/config.yaml");
        assert_eq!(register_list(&fixture), json!([MANIFEST_REFERENCE]), "{claudine_before:?}");
    }
}

#[test]
fn installed_registration_failed_publication_leaves_a_legacy_home_untouched() {
    let fixture = Fixture::new();
    with_legacy_registration(&fixture);
    fixture.write(&fixture.claudine(), "permissions:\n  network: deny\n");
    fixture.write(&output_root(&fixture.home()), "an obstruction");
    let before = fixture.files();

    let error = fixture.setup().unwrap_err();

    assert!(matches!(error, SetupError::Publish { .. }), "{error}");
    assert_eq!(fixture.files(), before, "byte for byte");
}

#[test]
fn installed_registration_setup_holds_the_publisher_lock_through_the_handoff() {
    let fixture = Fixture::new();
    with_legacy_registration(&fixture);
    let home = fixture.home();
    // Claudine's own write runs between the publish and the legacy cleanup;
    // a publisher started there must find the lock taken.
    let setup = setup_claudine(&fixture.context(), ConfigScope::User, |path| {
        let mut observer = |_: &_| std::ops::ControlFlow::Continue(());
        let hooks = PublishHooks { lock_deadline: std::time::Duration::from_millis(50), observer: &mut observer };
        let concurrent = publish_with(&home, RegistrationInventory::claudine(), hooks);
        assert!(matches!(concurrent, Err(PublishError::LockTimeout { .. })), "{concurrent:?}");
        write_default(path)
    })
    .unwrap();

    assert!(setup.cleanup.expect("cleanup ran").is_ok());
    assert!(!legacy_manifest_path(&home).exists());
    // Released once setup returns.
    assert!(matches!(publish(&home, RegistrationInventory::claudine()), Ok(PublishOutcome::Current { .. })));
}

#[test]
fn installed_registration_user_setup_never_rewrites_a_repository_entry() {
    let fixture = Fixture::new();
    let context = fixture.repository();
    let repository = fixture.repo().join(".darkmatter.yaml");
    let text = format!("register:\n  - {LEGACY_MANIFEST_REFERENCE}\n");
    fixture.write(&repository, &text);
    fixture.write(&legacy_manifest_path(&fixture.home()), "kind: darkmatter-host\nformat_version: 1\nhost: claudine\n");
    fixture.git(&["add", "."]);
    fixture.git(&["commit", "--quiet", "-m", "an old registration"]);
    // What `md` reads in the repository, with its committed tree captured.
    let diagnostics = || {
        let snapshot = RequestSnapshot::new(fixture.repo()).with_home(Some(fixture.home()));
        let request = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).expect("a request");
        RegisteredHosts::load(request.config_snapshot(), request.resolution_context()).diagnostics()
    };
    // Before setup, the passive reader refuses the format 1 manifest with the
    // unsupported-version upgrade instruction.
    let before = diagnostics();
    assert!(
        before.iter().any(|diagnostic| diagnostic.file == legacy_manifest_path(&fixture.home())
            && diagnostic.message.contains("unsupported host registration format version 1")
            && diagnostic.message.contains("republish the registration")),
        "{before:#?}"
    );

    setup_claudine(&context, ConfigScope::User, write_default).unwrap();

    assert_eq!(std::fs::read_to_string(&repository).unwrap(), text, "the repository file is not written");
    assert_eq!(register_list(&fixture), json!([MANIFEST_REFERENCE]));
    // After setup, the repository's own entry is reported at the file its
    // owners edit.
    let after = diagnostics();
    assert!(
        after.iter().any(|diagnostic| canonicalize_simplified(&diagnostic.file).ok() == canonicalize_simplified(&repository).ok()
            && diagnostic.message.contains("names no existing file")),
        "{after:#?}"
    );
}

#[cfg(unix)]
#[test]
fn installed_registration_cleanup_refuses_a_link_in_the_retired_tree_and_leaves_its_target() {
    let fixture = Fixture::new();
    with_legacy_registration(&fixture);
    let sentinel = fixture.root.join("outside/keep.yaml");
    fixture.write(&sentinel, "kept\n");
    let link = legacy_revisions_dir(&fixture.home()).join("0123abcd/linked");
    std::os::unix::fs::symlink(sentinel.parent().unwrap(), &link).unwrap();

    let setup = fixture.setup().unwrap();

    assert_eq!(register_list(&fixture), json!(["~/hosts/mine.yaml", MANIFEST_REFERENCE, "~/hosts/gone.yaml"]));
    let failure = setup.cleanup.expect("cleanup ran").unwrap_err();
    assert!(failure.removed.is_empty(), "{failure}");
    let error = failure.error;
    assert!(
        matches!(&error, CleanupError::Obstructed { path, obstruction: Obstruction::SymbolicLink } if *path == link),
        "{error}"
    );
    assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), "kept\n", "the link's target is untouched");
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink(), "the link stays");
    assert!(legacy_manifest_path(&fixture.home()).is_file(), "nothing was removed");
    assert!(legacy_revisions_dir(&fixture.home()).join("0123abcd/globals.yaml").is_file());
}

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn installed_registration_setup_upgrades_frozen_v2_in_place_and_repeats_as_noop() {
    let v2_source = biscuit_test_harness::manifest_dir!()
        .join("../../darkmatter/lib/tests/fixtures/host_registration_v2/darkmatter");
    let fixture = Fixture::new();
    copy_tree(&v2_source, &output_root(&fixture.home()));
    let manifest_abs = manifest_path(&fixture.home());
    let initial_dm = format!(
        "# user settings\nregister:\n  # my own host\n  - ~/hosts/mine.yaml   # kept\n  - {}\n",
        manifest_abs.display()
    );
    fixture.write(&fixture.darkmatter(), &initial_dm);

    // Before setup, Darkmatter refuses the v2 manifest with its version.
    let before = fixture.hosts(&fixture.context());
    assert!(
        before.diagnostics().iter().any(|d| d.message.contains("unsupported host registration format version 2")),
        "{:#?}",
        before.diagnostics()
    );

    let setup = fixture.setup().unwrap();
    let changes = setup.publication.changes().expect("v2 upgraded to v3");
    assert_eq!(changes.written.last().map(String::as_str), Some("manifest.yaml"), "manifest written last: {changes:?}");
    let upgraded_manifest = std::fs::read_to_string(&manifest_abs).unwrap();
    assert!(upgraded_manifest.contains("format_version: 3\n"), "{upgraded_manifest}");
    assert!(!upgraded_manifest.contains("functions:"), "{upgraded_manifest}");

    // Equivalent register entry, unrelated entry, and comments survive;
    // Darkmatter accepts the upgraded Claudine v3 profile (only the unrelated
    // placeholder entry is absent on disk).
    let dm_text = std::fs::read_to_string(fixture.darkmatter()).unwrap();
    assert!(dm_text.contains("# user settings\n"), "{dm_text}");
    assert!(dm_text.contains("  # my own host\n  - ~/hosts/mine.yaml   # kept\n"), "{dm_text}");
    let after = fixture.hosts(&fixture.context());
    assert!(
        after.diagnostics().iter().all(|d| d.entry.reference == "~/hosts/mine.yaml"),
        "no diagnostic for Claudine's upgraded v3 profile: {:#?}",
        after.diagnostics()
    );
    let profiles = &after.loaded().profiles;
    assert_eq!(profiles.len(), 1, "one loaded profile");
    assert_eq!(profiles[0].host(), "claudine");

    // Second setup with current bytes makes no change.
    let before_repeat = fixture.files();
    let repeat = fixture.setup().unwrap();
    assert!(matches!(repeat.publication, PublishOutcome::Current { .. }), "{repeat:?}");
    assert_eq!(repeat.registration.unwrap().action, InitAction::Unchanged);
    assert_eq!(fixture.files(), before_repeat, "second setup writes nothing");
}
