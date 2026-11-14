//! Claudine's installed Darkmatter integration, end to end over a temporary
//! home: `setup_claudine` publishes the host revision, writes Claudine's
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
    HostRevision, MANIFEST_REFERENCE, PublishOutcome, manifest_path, publish, revisions_dir,
};
use claudine::composition::{TriggerFindings, pre_validate_schema, resolve_composition_source_in_context};
use claudine::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};
use claudine::config::persist::{SaveOutcome, initialize_config};
use claudine::config::schema::ConfigScope;
use claudine::config::setup::{ClaudineSetup, PermissionsHandoff, SetupError, setup_claudine, setup_with_darkmatter};
use darkmatter::config::init::{InitAction, InitError, InitOptions, init_user};
use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::registration::{HostSelection, RegisteredHosts, select};
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

    /// The non-staging revision directory names.
    fn revision_names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(revisions_dir(&self.home()))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
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
    assert!(manifest.is_file(), "the stable manifest exists");
    assert!(setup.publication.revision_dir().join("schemas").is_dir(), "the revision exists");
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
fn installed_registration_same_manifest_spelled_absolutely_is_not_duplicated() {
    let fixture = Fixture::new();
    let absolute = manifest_path(&fixture.home());
    let quoted = serde_json::to_string(absolute.to_str().unwrap()).unwrap();
    fixture.write(&fixture.darkmatter(), &format!("register:\n  - {quoted}\n"));

    let setup = fixture.setup().unwrap();

    assert_eq!(setup.registration.unwrap().action, InitAction::Unchanged);
    assert_eq!(document(&fixture.darkmatter())["register"], json!([absolute.to_str().unwrap()]));
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
    fixture.write(&revisions_dir(&fixture.home()), "an obstruction");

    let error = fixture.setup().unwrap_err();

    assert!(matches!(error, SetupError::Publish(_)), "{error}");
    assert!(!fixture.claudine().exists(), "Claudine's configuration is not written");
    assert!(!fixture.darkmatter().exists(), "Darkmatter's configuration is not written");
    assert!(!manifest_path(&fixture.home()).exists(), "no manifest names a missing revision");

    std::fs::remove_file(revisions_dir(&fixture.home())).unwrap();
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
fn installed_registration_upgraded_binary_refresh_changes_the_facts() {
    let fixture = Fixture::new();
    fixture.setup().unwrap();
    fixture.compose(MAPPING_MODEL, Some(fixture.hosts(&fixture.context()))).expect_err("the installed revision rejects it");
    let original = fixture.revision_names();
    let original_dir = revisions_dir(&fixture.home()).join(&original[0]);
    let original_files: BTreeMap<_, _> =
        fixture.files().into_iter().filter(|(path, _)| path.starts_with(&original_dir)).collect();
    let (_, claudine_yaml) = HostRevision::claudine().files().find(|(name, _)| *name == "claudine.yaml").unwrap();
    let declaration = "\"string -> the model to run\"";
    assert!(claudine_yaml.contains(declaration), "the embedded schema declares the model as a string");
    let upgraded = HostRevision::claudine()
        .clone()
        .with_version("99.0.0")
        .with_file("claudine.yaml", claudine_yaml.replace(declaration, "\"object -> the model to run\""));

    publish(&fixture.home(), &upgraded).unwrap();

    let hosts = fixture.hosts(&fixture.context());
    assert_one_claudine_profile(&hosts);
    assert_eq!(hosts.loaded().profiles[0].version(), "99.0.0");
    if let Err(error) = fixture.compose(MAPPING_MODEL, Some(hosts)) {
        panic!("the upgraded revision accepts a mapping model: {error:?}");
    }
    assert_eq!(fixture.revision_names().len(), 2, "{:?}", fixture.revision_names());
    let retained: BTreeMap<_, _> =
        fixture.files().into_iter().filter(|(path, _)| path.starts_with(&original_dir)).collect();
    assert_eq!(retained, original_files, "the old revision is retained byte for byte");
    assert!(!fixture.home().join("schemas").exists(), "nothing is copied into ~/schemas");
}

/// A repository registration of a hand-written `claudine` host whose only
/// trigger rejects a string `model`, which Claudine's own schema accepts.
fn install_hostile_claudine(fixture: &Fixture) {
    let schemas = fixture.repo().join(".hostile/schemas");
    for (name, text) in HostRevision::claudine().files() {
        if ["claudine-document.yaml", "globals.yaml", "globals.lifecycle.json"].contains(&name) {
            fixture.write(&schemas.join(name), text);
        }
    }
    fixture.write(
        &schemas.join("hostile.yaml"),
        "kind: schema-trigger\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    model: \"number -> a hostile model type\"\n",
    );
    fixture.write(
        &fixture.repo().join(".hostile/darkmatter-host.yaml"),
        "kind: darkmatter-host\nformat_version: 1\nhost: claudine\nversion: 6.6.6\n\
         identify: ./schemas/claudine-document.yaml\ntriggers:\n    - ./schemas/hostile.yaml\n\
         globals: ./schemas/globals.yaml\nlifecycle: ./schemas/globals.lifecycle.json\nfunctions: []\n",
    );
    fixture.write(&fixture.repo().join(".darkmatter.yaml"), "register:\n  - ./.hostile/darkmatter-host.yaml\n");
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
