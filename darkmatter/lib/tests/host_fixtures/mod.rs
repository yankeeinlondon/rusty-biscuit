//! A temporary home, working directory, and valid format 3 `darkmatter-host`
//! bundle (the `acme` host, or the Claudine-shaped `relay` host) for
//! registration and selection tests. Nothing is read from the process or the
//! repository.
//!
//! A [`Bundle`] is a manifest's fields plus its members' texts; its manifest
//! is written with the real digests. A test that edits a member the way a
//! publisher would re-addresses the manifest ([`Fixture::write_member`]); a
//! test that breaks one boundary on purpose writes without re-addressing
//! ([`Fixture::corrupt_member`]).

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::load::{ConfigRevision, Registration};
use darkmatter::config::registration::namespaces::{member_digest, namespace_catalog_hash};
use darkmatter::config::registration::{LoadedRegistrations, load_entries, md_owned_names};
use darkmatter::lifecycle::host_profile::lifecycle_artifact;
use darkmatter::lifecycle::registry::EventRegistry;
use darkmatter::lifecycle::{LifecycleParseOptions, UnknownVerbPolicy};
use darkmatter::markdown::compose::expression::{HostFunctionCatalog, HostFunctions, HostHandler};
use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalRequirement};
use serde_json::json;

/// The manifest's file name inside an installed bundle folder.
pub const MANIFEST_NAME: &str = "darkmatter-host.yaml";

pub const WIDGET_TRIGGER: &str = "\
kind: schema-trigger
match:
    widget: object(required)
$schema:
    widget: widget@./library.yaml
";

pub const GADGET_TRIGGER: &str = "\
kind: schema-trigger
match:
    gadget: string(required)
$schema:
    gadget: gadget@./library.yaml
";

/// `gadget.yaml` with its condition in a nested `group` under `all`, shaped
/// like Claudine's published inline-composition trigger, and the payload
/// declaring the same `gadget` property, so a locator that loses the
/// condition's path cannot land on the right line by accident.
pub const GROUPED_GADGET_TRIGGER: &str = "\
kind: schema-trigger
match:
    all:
        - group:
              - gizmo: string(required)
              - gadget: string(required)
        - none:
              - sequence: string(required)
$schema:
    gadget: gadget@./library.yaml
";

pub const LIBRARY: &str = "\
kind: schema
types:
    widget:
        size: \"number -> The widget's size.\"
    gadget: \"string -> The gadget's name.\"
";

/// The `acme` host's one namespace: the `run` global.
pub const RUN_NAMESPACE: &str = "\
kind: namespace-schema
namespace: run
description: The current pass.
$schema:
    count: \"number -> The pass.\"
";

pub const FUNCTIONS: &str = "\
kind: expression-schema
categories:
    - Acme -> Ask the acme host.
types:
    acme_label: |-
        function(
            parameters(name: string);
            returns(string);
            category(Acme);
        ) -> The acme label of a name.
";

/// The `relay` host's trigger, shaped like Claudine's: it matches a document
/// with `sequence` or `start`, never a shared event alone, types the relay's
/// own keys, and types `composed`/`failed` as `null` with a description
/// naming the relay event to write instead.
pub const RELAY_TRIGGER: &str = "\
kind: schema-trigger
match:
    group:
        - sequence: \"unknown[](required)\"
        - start: \"unknown[](required)\"
$schema:
    sequence: \"{ name: string, prompt: string }[] -> the steps\"
    timeout: \"string -> the run's timeout, such as 30m\"
    start: \"unknown[] -> actions run before the agent starts\"
    blocked: \"unknown[] -> actions run when the agent is blocked\"
    success: \"unknown[] -> actions run after a successful run\"
    failure: \"unknown[] -> actions run after a failed run\"
    composed: \"null -> relay names this event `start`; write `start:` instead of `composed:`\"
    failed: \"null -> relay names this event `blocked`; write `blocked:` instead of `failed:`\"
";

/// The `relay` host's one namespace: the `seq` global.
pub const RELAY_NAMESPACE: &str = "\
kind: namespace-schema
namespace: seq
description: The current step.
$schema:
    name: \"string -> The current step's name.\"
";

/// The catalog requirement a namespace's global gives when a test builds the
/// host's own registry for its lifecycle artifact.
fn catalog_of(namespaces: &[(&str, &str)]) -> GlobalCatalog {
    // A host's registry only records that a catalog is registered; a
    // one-global catalog of the same name stands for the namespace files.
    let mut documents = String::from("kind: schema\n$schema:\n");
    let mut requirements = Vec::new();
    for (_, text) in namespaces {
        let name = namespace_name(text);
        documents.push_str(&format!("    {name}:\n        placeholder: \"string -> stands for the namespace\"\n"));
        requirements.push(GlobalRequirement::owned_shape(name));
    }
    GlobalCatalog::from_documents("catalog.yaml", &[("catalog.yaml", &documents)], &requirements).expect("the stand-in catalog loads")
}

/// The `namespace:` a namespace file declares.
pub fn namespace_name(text: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix("namespace: "))
        .expect("a namespace file declares `namespace:`")
        .trim()
        .to_string()
}

/// The `catalog_hash` a lifecycle artifact records for `namespaces` (path,
/// text) when none of them imports another file.
pub fn catalog_hash_of(namespaces: &[(&str, &str)]) -> String {
    let digests: BTreeMap<String, String> =
        namespaces.iter().map(|(path, text)| ((*path).to_string(), member_digest(text.as_bytes()))).collect();
    namespace_catalog_hash(&digests)
}

/// The lifecycle artifact the host `host` writes for its namespace files
/// `namespaces` (path, text), generated by the real writer so its hash
/// matches. Declares each namespace as a host global.
pub fn lifecycle_artifact_for(host: &str, namespaces: &[(&str, &str)]) -> String {
    let mut registry = EventRegistry::new(host);
    for (_, text) in namespaces {
        registry.declare_host_global(namespace_name(text)).expect("the namespace is unreserved");
    }
    registry.register_global_catalog(Arc::new(catalog_of(namespaces))).expect("not shared yet");
    registry.validate().expect("no outcome events");
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        authored_values: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: &registry,
    };
    lifecycle_artifact(&options, &[], &catalog_hash_of(namespaces))
}

/// The lifecycle artifact host `host` writes when its catalog declares
/// exactly `globals`, recording `catalog_hash` as given: for a catalog whose
/// namespaces import other bundled files, or an artifact that must disagree
/// with its catalog. Declares each of `globals` as a host global; with no
/// `globals`, registers no catalog (`global_catalog: false`).
pub fn lifecycle_artifact_declaring(host: &str, globals: &[&str], catalog_hash: &str) -> String {
    let mut documents = String::from("kind: schema\n$schema:\n");
    let mut requirements = Vec::new();
    let mut registry = EventRegistry::new(host);
    for name in globals {
        documents.push_str(&format!("    {name}:\n        placeholder: \"string -> stands for the namespace\"\n"));
        requirements.push(GlobalRequirement::owned_shape((*name).to_string()));
        registry.declare_host_global((*name).to_string()).expect("the namespace is unreserved");
    }
    if !globals.is_empty() {
        let catalog = GlobalCatalog::from_documents("catalog.yaml", &[("catalog.yaml", &documents)], &requirements)
            .expect("the stand-in catalog loads");
        registry.register_global_catalog(Arc::new(catalog)).expect("not shared yet");
    }
    registry.validate().expect("no outcome events");
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        authored_values: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: &registry,
    };
    lifecycle_artifact(&options, &[], catalog_hash)
}

/// The facts artifact the host's registrations give `FUNCTIONS`.
pub fn facts_artifact() -> String {
    let catalog = HostFunctionCatalog::load(FUNCTIONS, "functions.yaml").expect("the function catalog loads");
    let handler: HostHandler = Arc::new(|_| Ok(json!("label")));
    HostFunctions::builder(catalog)
        .function("acme_label", &[], |registration| registration, handler)
        .build()
        .expect("the host registers")
        .facts_artifact()
}

/// The relay's lifecycle artifact, generated by the real writer: it renames
/// `composed` to `start` and `failed` to `blocked`, registers
/// `success`/`failure` of its own, and declares the `seq` global.
pub fn relay_lifecycle_artifact() -> String {
    use darkmatter::lifecycle::registry::{CoreEvent, EventDescriptor, OutcomePair, Position, Role, generic_view};
    use darkmatter::markdown::compose::expression::Availability;

    let namespaces = [("schemas/seq.yaml", RELAY_NAMESPACE)];
    let mut registry = EventRegistry::new("relay");
    registry.rename(CoreEvent::Composed, "start").expect("`start` is free");
    registry.rename(CoreEvent::Failed, "blocked").expect("`blocked` is free");
    for name in ["success", "failure"] {
        let view = generic_view(&format!("relay.lifecycle.{name}"), Availability::Available);
        registry
            .register(EventDescriptor::new(name, Position::AfterComposition, Role::TerminalOutcome, view))
            .expect("the relay's events register");
    }
    registry.register_outcome_pair(OutcomePair::new("success", "failure")).expect("the pair registers");
    registry.declare_host_global(String::from("seq")).expect("seq is unreserved");
    registry.register_global_catalog(Arc::new(catalog_of(&namespaces))).expect("not shared yet");
    registry.validate().expect("the relay registry is complete");
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        authored_values: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: &registry,
    };
    lifecycle_artifact(&options, &[], &catalog_hash_of(&namespaces))
}

/// A format 3 bundle: its manifest's fields and its members' texts, by
/// bundle-relative path. An expression catalog is a `files` member; its
/// facts companion is a member the manifest digests but does not list.
#[derive(Debug, Clone)]
pub struct Bundle {
    pub host: String,
    pub version: String,
    /// The `files` members, in authored order.
    pub files: Vec<String>,
    pub lifecycle: String,
    /// Every member's text, by bundle-relative path.
    pub members: BTreeMap<String, String>,
}

/// The facts companion of the expression catalog at bundle path `catalog`:
/// its last `.yaml`/`.yml` extension replaced by `.facts.json`.
pub fn companion_of(catalog: &str) -> String {
    let stem = catalog.strip_suffix(".yaml").or_else(|| catalog.strip_suffix(".yml")).expect("a YAML catalog");
    format!("{stem}.facts.json")
}

impl Bundle {
    /// The valid `acme` bundle: two triggers sharing a library, the `run`
    /// namespace, a lifecycle artifact, and one expression catalog with its
    /// companion.
    pub fn acme() -> Self {
        Self::acme_named("acme")
    }

    /// [`Self::acme`] for host `host` (manifest and lifecycle artifact).
    pub fn acme_named(host: &str) -> Self {
        let mut members = BTreeMap::new();
        for (path, text) in [
            ("schemas/widget.yaml", WIDGET_TRIGGER),
            ("schemas/gadget.yaml", GADGET_TRIGGER),
            ("schemas/library.yaml", LIBRARY),
            ("schemas/run.yaml", RUN_NAMESPACE),
            ("schemas/functions.yaml", FUNCTIONS),
        ] {
            members.insert(path.to_string(), text.to_string());
        }
        members.insert("schemas/functions.facts.json".to_string(), facts_artifact());
        members.insert(
            "schemas/globals.lifecycle.json".to_string(),
            lifecycle_artifact_for(host, &[("schemas/run.yaml", RUN_NAMESPACE)]),
        );
        Self {
            host: host.to_string(),
            version: "1.2.0".to_string(),
            files: [
                "schemas/widget.yaml",
                "schemas/gadget.yaml",
                "schemas/library.yaml",
                "schemas/run.yaml",
                "schemas/functions.yaml",
            ]
            .map(str::to_string)
            .to_vec(),
            lifecycle: "schemas/globals.lifecycle.json".to_string(),
            members,
        }
    }

    /// The valid `relay` bundle, shaped like Claudine's.
    pub fn relay() -> Self {
        let mut members = BTreeMap::new();
        members.insert("schemas/relay.yaml".to_string(), RELAY_TRIGGER.to_string());
        members.insert("schemas/seq.yaml".to_string(), RELAY_NAMESPACE.to_string());
        members.insert("schemas/globals.lifecycle.json".to_string(), relay_lifecycle_artifact());
        members.insert("schemas/functions.yaml".to_string(), FUNCTIONS.to_string());
        members.insert("schemas/functions.facts.json".to_string(), facts_artifact());
        Self {
            host: "relay".to_string(),
            version: "0.3.0".to_string(),
            files: vec!["schemas/relay.yaml".to_string(), "schemas/seq.yaml".to_string(), "schemas/functions.yaml".to_string()],
            lifecycle: "schemas/globals.lifecycle.json".to_string(),
            members,
        }
    }

    /// Adds (or replaces) the `files` member `path` with `text`.
    #[must_use]
    pub fn with_file(mut self, path: &str, text: &str) -> Self {
        if !self.files.iter().any(|file| file == path) {
            self.files.push(path.to_string());
        }
        self.members.insert(path.to_string(), text.to_string());
        self
    }

    /// Replaces the text of the member at `path`, listed or not.
    #[must_use]
    pub fn with_member(mut self, path: &str, text: &str) -> Self {
        self.members.insert(path.to_string(), text.to_string());
        self
    }

    /// Removes `path` from `files` and from the members.
    #[must_use]
    pub fn without_file(mut self, path: &str) -> Self {
        self.files.retain(|file| file != path);
        self.members.remove(path);
        self
    }

    /// The `files` members that are expression catalogs.
    pub fn catalogs(&self) -> Vec<String> {
        self.files
            .iter()
            .filter(|path| {
                self.members.get(*path).is_some_and(|text| text.lines().any(|line| line.trim() == "kind: expression-schema"))
            })
            .cloned()
            .collect()
    }

    /// Every member path the manifest digests: `files`, the lifecycle
    /// artifact, then each catalog's companion.
    pub fn listed(&self) -> Vec<String> {
        let mut listed = self.files.clone();
        listed.push(self.lifecycle.clone());
        listed.extend(self.catalogs().iter().map(|catalog| companion_of(catalog)));
        listed
    }

    /// The manifest, with every listed member's real digest.
    pub fn manifest(&self) -> String {
        let digests: BTreeMap<String, String> = self
            .listed()
            .into_iter()
            .map(|path| {
                let text = self.members.get(&path).map_or("", String::as_str);
                (path, member_digest(text.as_bytes()))
            })
            .collect();
        manifest_text(self, &digests)
    }

    /// Writes the manifest and every member under `dir`.
    pub fn write_to(&self, dir: &Path) {
        for (path, text) in &self.members {
            write_file(&dir.join(path), text);
        }
        write_file(&dir.join(MANIFEST_NAME), &self.manifest());
    }

    /// The manifest and every member, by path relative to the bundle folder.
    pub fn files_with_manifest(&self) -> Vec<(String, String)> {
        let mut files: Vec<(String, String)> = vec![(MANIFEST_NAME.to_string(), self.manifest())];
        files.extend(self.members.iter().map(|(path, text)| (path.clone(), text.clone())));
        files
    }
}

/// A manifest for `bundle` with the given `digests` (by bundle path), which
/// need not match the members.
pub fn manifest_text(bundle: &Bundle, digests: &BTreeMap<String, String>) -> String {
    let mut text = format!(
        "kind: darkmatter-host\nformat_version: 3\nhost: {}\nversion: {}\nfiles:\n",
        bundle.host, bundle.version
    );
    if bundle.files.is_empty() {
        text = text.replace("files:\n", "files: []\n");
    }
    for file in &bundle.files {
        text.push_str(&format!("    - ./{file}\n"));
    }
    text.push_str(&format!("lifecycle: ./{}\n", bundle.lifecycle));
    text.push_str("digests:\n");
    for (path, digest) in digests {
        text.push_str(&format!("    ./{path}: \"{digest}\"\n"));
    }
    text
}

/// Every file of a valid relay install, relative to its folder.
pub fn relay_files() -> Vec<(String, String)> {
    Bundle::relay().files_with_manifest()
}

fn write_file(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A temporary home, working directory, and valid host install under
/// `host/`; nothing is read from the process.
pub struct Fixture {
    _dir: tempfile::TempDir,
    pub root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        let fixture = Self { _dir: dir, root };
        fixture.install("host");
        fixture
    }

    /// Writes a complete, valid `acme` bundle into `folder`.
    pub fn install(&self, folder: &str) {
        self.install_bundle(folder, &Bundle::acme());
    }

    /// Writes `bundle` into `folder`.
    pub fn install_bundle(&self, folder: &str, bundle: &Bundle) {
        bundle.write_to(&self.path(folder));
    }

    /// Writes a complete, valid host named `host` into `folder`: the `acme`
    /// bundle with its manifest's and lifecycle artifact's host renamed.
    pub fn install_host(&self, folder: &str, host: &str) {
        self.install_bundle(folder, &Bundle::acme_named(host));
    }

    /// Writes a valid relay install into `folder`.
    pub fn install_relay(&self, folder: &str) {
        self.install_bundle(folder, &Bundle::relay());
    }

    /// Joined one segment at a time so the path displays with the native
    /// separator, as the paths production reports do on Windows.
    pub fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root.clone(), |path, segment| path.join(segment))
    }

    pub fn write(&self, relative: &str, text: &str) {
        write_file(&self.path(relative), text);
    }

    pub fn manifest(&self) -> PathBuf {
        self.path(&format!("host/{MANIFEST_NAME}"))
    }

    /// The current manifest text of the bundle in `folder`.
    pub fn manifest_text(&self, folder: &str) -> String {
        std::fs::read_to_string(self.path(&format!("{folder}/{MANIFEST_NAME}"))).unwrap()
    }

    /// Replaces the `host/` manifest with `edit` applied to its current text,
    /// digests untouched.
    pub fn edit_manifest(&self, edit: impl FnOnce(&str) -> String) {
        let current = self.manifest_text("host");
        let edited = edit(&current);
        assert_ne!(edited, current, "the edit changes the manifest");
        self.write(&format!("host/{MANIFEST_NAME}"), &edited);
    }

    /// Writes the member `relative` of the bundle in `folder` and
    /// re-addresses its manifest: a legitimate republish of one member.
    pub fn write_member(&self, folder: &str, relative: &str, text: &str) {
        self.write(&format!("{folder}/{relative}"), text);
        self.readdress(folder);
    }

    /// Writes the member `relative` of the bundle in `folder` without
    /// touching the manifest, so its digest no longer matches.
    pub fn corrupt_member(&self, folder: &str, relative: &str, text: &str) {
        self.write(&format!("{folder}/{relative}"), text);
    }

    /// Rewrites every digest of the manifest in `folder` from the members'
    /// current bytes (a missing member keeps its old digest).
    pub fn readdress(&self, folder: &str) {
        let manifest = self.manifest_text(folder);
        let mut rewritten = String::with_capacity(manifest.len());
        let mut in_digests = false;
        for line in manifest.lines() {
            if line.starts_with("digests:") {
                in_digests = true;
                rewritten.push_str(line);
                rewritten.push('\n');
                continue;
            }
            if in_digests && line.starts_with("    ./") {
                let (key, _) = line.trim().split_once(": ").expect("a digest entry");
                let member = self.path(&format!("{folder}/{}", key.trim_start_matches("./")));
                match std::fs::read(&member) {
                    Ok(bytes) => rewritten.push_str(&format!("    {key}: \"{}\"\n", member_digest(&bytes))),
                    Err(_) => {
                        rewritten.push_str(line);
                        rewritten.push('\n');
                    }
                }
                continue;
            }
            in_digests = in_digests && line.starts_with(' ');
            rewritten.push_str(line);
            rewritten.push('\n');
        }
        self.write(&format!("{folder}/{MANIFEST_NAME}"), &rewritten);
    }

    /// A `globals.yaml` with a lifecycle artifact beside it, and a function
    /// catalog with its facts, in the schema root `folder`: what a host
    /// installed before registration. Nothing reads these as a host.
    pub fn root_catalog(&self, folder: &str, host: &str) {
        self.write(
            &format!("{folder}/globals.yaml"),
            "kind: schema\ndescription: the acme host's globals\n$schema:\n    run:\n        count: \"number -> The pass.\"\n",
        );
        self.write(&format!("{folder}/globals.lifecycle.json"), &lifecycle_artifact_for(host, &[("schemas/run.yaml", RUN_NAMESPACE)]));
        self.write(&format!("{folder}/functions.yaml"), FUNCTIONS);
        self.write(&format!("{folder}/functions.facts.json"), &facts_artifact());
    }

    pub fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.path("work"), Some(self.path("home")), HashMap::new())
    }

    /// A user-scope entry naming `reference`.
    pub fn entry(&self, reference: &str, index: usize) -> Registration {
        Registration {
            reference: reference.to_string(),
            index,
            declared_in: self.path("home/.darkmatter.yaml"),
            revision: ConfigRevision::User,
            at: None,
        }
    }

    pub fn load_references(&self, references: &[&str]) -> LoadedRegistrations {
        let entries: Vec<Registration> =
            references.iter().enumerate().map(|(index, reference)| self.entry(reference, index)).collect();
        load_entries(&entries, &self.context(), md_owned_names())
    }

    pub fn load(&self) -> LoadedRegistrations {
        self.load_references(&[&format!("../host/{MANIFEST_NAME}")])
    }

    /// The registered hosts `references` name, loaded in [`Self::context`].
    pub fn hosts(&self, references: &[&str]) -> Arc<darkmatter::config::registration::RegisteredHosts> {
        Arc::new(darkmatter::config::registration::RegisteredHosts::new(Arc::new(self.load_references(references))))
    }

    /// `work/` as a repository (marked with `.git`), with `package_area`
    /// (relative to `work/`) as its package area when given.
    pub fn repository(&self, package_area: Option<&str>) -> FileResolutionContext {
        std::fs::create_dir_all(self.path("work/.git")).unwrap();
        let work = self.path("work");
        let context = self.context().with_repository_root(&work);
        match package_area {
            Some(area) => context.with_package_area(work.join(area)),
            None => context,
        }
    }

    /// Writes the document `relative` (under the root) and returns it with
    /// its own context, derived from `context`.
    pub fn document(
        &self,
        context: &FileResolutionContext,
        relative: &str,
        text: &str,
    ) -> (darkmatter::markdown::Markdown, FileResolutionContext) {
        self.write(relative, text);
        let path = self.path(relative);
        let markdown = darkmatter::markdown::Markdown::try_from(path.as_path()).unwrap();
        (markdown, context.for_source(&path))
    }
}
