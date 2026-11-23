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
    CleanupError, FileKind, LEGACY_MANIFEST_REFERENCE, MANIFEST_REFERENCE, Obstruction, PublishOutcome, PublishedFile,
    RegistrationInventory, legacy_manifest_path, legacy_revisions_dir, manifest_path, output_root, publish,
};
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

    assert!(matches!(error, SetupError::Publish(_)), "{error}");
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

#[test]
fn installed_registration_upgraded_binary_refresh_changes_the_facts() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    fixture.compose(MAPPING_MODEL, Some(fixture.hosts(&fixture.context()))).expect_err("the installed registration rejects it");
    let root = output_root(&fixture.home());
    let original_files: BTreeMap<_, _> = fixture.files().into_iter().filter(|(path, _)| path.starts_with(&root)).collect();
    let shared_yaml = RegistrationInventory::claudine().file("schemas/shared.yaml").unwrap().text().unwrap();
    let declaration = "\"string -> the model to run\"";
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

    assert!(matches!(error, SetupError::Publish(_)), "{error}");
    assert_eq!(fixture.files(), before, "byte for byte");
}

#[test]
fn installed_registration_user_setup_never_rewrites_a_repository_entry() {
    let fixture = Fixture::new();
    let context = fixture.repository();
    let repository = fixture.repo().join(".darkmatter.yaml");
    let text = format!("register:\n  - {LEGACY_MANIFEST_REFERENCE}\n");
    fixture.write(&repository, &text);

    setup_claudine(&context, ConfigScope::User, write_default).unwrap();

    assert_eq!(std::fs::read_to_string(&repository).unwrap(), text, "the repository file is not written");
    assert_eq!(register_list(&fixture), json!([MANIFEST_REFERENCE]));
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
    let error = setup.cleanup.expect("cleanup ran").unwrap_err();
    assert!(
        matches!(&error, CleanupError::Obstructed { path, obstruction: Obstruction::SymbolicLink } if *path == link),
        "{error}"
    );
    assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), "kept\n", "the link's target is untouched");
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink(), "the link stays");
    assert!(legacy_manifest_path(&fixture.home()).is_file(), "nothing was removed");
    assert!(legacy_revisions_dir(&fixture.home()).join("0123abcd/globals.yaml").is_file());
}
