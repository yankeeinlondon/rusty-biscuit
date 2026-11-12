//! Prepared trigger bundles: a host's embedded bundle served from an accepted
//! artifact record instead of being prepared at discovery.
//!
//! Darkmatter ships no bundle, so these tests build small ones, generate an
//! in-memory artifact for each with `simplified_schema::shipped`, and accept
//! that artifact into a static the bundle's descriptor reads. Discovery with
//! the prepared bundle must give the same triggers and validation results as
//! the source-only bundle, without the payload checks generation already
//! ran; a changed same-name bundle, and the documents a disk file
//! suppresses, keep ordinary behavior; and a descriptor never changes which
//! source wins a name (disk files of any content, bundle order), where a
//! surviving trigger's imports resolve (its own bundle), or the context its
//! `$path` patterns are judged in (each registry's). The check counts need
//! `work-counters`, which `just test` enables; without it only the results
//! are compared.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::triggers::{
    PreparedBundleDescriptor, PreparedTriggerBundle, TriggerBundleRecord, TriggerOutcomeRecord, prepare_trigger_bundle,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EmbeddedSchemaBundle, TriggerOrigin, TriggerSource};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simplified_schema::{SchemaError, ValidationProblemCode};
use simplified_schema::embedded::EmbeddedDocument;
use simplified_schema::shipped::{
    AcceptedArtifact, ArtifactDescriptor, FallbackReason, FreshLower, PreparationMode, Refusal, accept, generate,
};

const OWNER: &str = "test-host";
const BUNDLE: &str = "test-host";

/// A trigger with an inline payload importing a sibling library and a
/// shipped partial's type.
const HOST: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
    $schema:\n  model: string\n  stack: stack@./lifecycle.yaml\n  retries: count@darkmatter:lifecycle\n";
const LIFECYCLE: &str = "kind: schema\ntypes:\n  stack: \"enum(say, stop)[]\"\n";
/// A trigger whose payload is a reference to a sibling schema document.
const NOTES: &str = "kind: schema-trigger\nmatch:\n  topic: string(required)\n$schema: ./notes-schema.yaml\n";
const NOTES_SCHEMA: &str = "$schema:\n  topic: string\n  pages: number(min(1))\n";
/// A matcher-only trigger.
const MARKER: &str = "kind: schema-trigger\nmatch:\n  marker: string(required)\n";

const FILES: &[(&str, &str)] = &[
    ("host.yaml", HOST),
    ("lifecycle.yaml", LIFECYCLE),
    ("marker.yaml", MARKER),
    ("notes-schema.yaml", NOTES_SCHEMA),
    ("notes.yaml", NOTES),
];

const SOURCE_ONLY: EmbeddedSchemaBundle = EmbeddedSchemaBundle::new(BUNDLE, FILES);
const DESCRIPTOR: PreparedBundleDescriptor = PreparedBundleDescriptor { owner: OWNER, provider: accepted_bundle };
const PREPARED: EmbeddedSchemaBundle = SOURCE_ONLY.with_prepared(&DESCRIPTOR);

/// The artifact payload of the test host.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestPayload {
    bundle: TriggerBundleRecord,
}

struct Generated {
    fresh: FreshLower,
    bytes: Vec<u8>,
}

/// One test host: its source-only bundle, that bundle's artifact, and the
/// accepted prepared bundle a descriptor's provider returns.
struct Host {
    source: EmbeddedSchemaBundle,
    generated: OnceLock<Generated>,
    accepted: OnceLock<AcceptedArtifact<PreparedTriggerBundle>>,
}

impl Host {
    const fn new(source: EmbeddedSchemaBundle) -> Self {
        Self { source, generated: OnceLock::new(), accepted: OnceLock::new() }
    }

    /// Calls `f` with the descriptor of the host's artifact: the bundle's
    /// documents over the freshly prepared library artifact.
    fn with_descriptor<R>(&self, fresh: &FreshLower, f: impl FnOnce(&ArtifactDescriptor<'_>) -> R) -> R {
        let documents = self.source.documents();
        let sources: Vec<EmbeddedDocument<'_>> =
            documents.iter().map(|(path, text)| EmbeddedDocument { path, text }).collect();
        let dependencies = [fresh.as_dependency()];
        f(&ArtifactDescriptor {
            owner: OWNER,
            preparation_revision: 1,
            sources: &sources,
            dependencies: &dependencies,
            configuration: &[],
        })
    }

    /// The host's artifact, generated once per process in uncached mode.
    fn generated(&self) -> &Generated {
        self.generated.get_or_init(|| {
            let fresh = FreshLower::prepare().expect("fresh library preparation");
            let record = prepare_trigger_bundle(self.source, PreparationMode::Uncached(&fresh)).expect("bundle prepares");
            let bytes = self
                .with_descriptor(&fresh, |descriptor| generate(descriptor, &TestPayload { bundle: record }))
                .expect("artifact generates");
            Generated { fresh, bytes }
        })
    }

    fn accept_bytes(&self, bytes: &[u8]) -> Result<AcceptedArtifact<PreparedTriggerBundle>, Refusal> {
        self.with_descriptor(&self.generated().fresh, |descriptor| {
            accept(descriptor, Some(bytes), |acceptance, payload: TestPayload| {
                PreparedTriggerBundle::from_record(acceptance, "/bundle", payload.bundle)
            })
        })
    }

    fn accepted(&'static self) -> Option<&'static PreparedTriggerBundle> {
        Some(self.accepted.get_or_init(|| self.accept_bytes(&self.generated().bytes).expect("artifact accepted")).payload())
    }
}

static HOST_BUNDLE: Host = Host::new(SOURCE_ONLY);

fn generated() -> &'static Generated {
    HOST_BUNDLE.generated()
}

fn accept_bytes(bytes: &[u8]) -> Result<AcceptedArtifact<PreparedTriggerBundle>, Refusal> {
    HOST_BUNDLE.accept_bytes(bytes)
}

/// [`DESCRIPTOR`]'s provider.
fn accepted_bundle() -> Option<&'static PreparedTriggerBundle> {
    HOST_BUNDLE.accepted()
}

// ── Counters ────────────────────────────────────────────────────────────────

/// `[merge checks, compile checks, shipped runtime preparations]`.
type Checks = [u64; 3];

#[cfg(feature = "work-counters")]
fn checks() -> Option<Checks> {
    use darkmatter::work_counters::{Work, snapshot};
    let now = snapshot();
    Some([
        now.get(Work::PayloadMergeCheck),
        now.get(Work::PayloadCompileCheck),
        now.get(Work::ShippedRuntimePreparation),
    ])
}

#[cfg(not(feature = "work-counters"))]
fn checks() -> Option<Checks> {
    None
}

/// The checks run since `before`, or `None` without `work-counters`.
fn checks_since(before: Option<Checks>) -> Option<Checks> {
    let (before, now) = (before?, checks()?);
    Some([now[0] - before[0], now[1] - before[1], now[2] - before[2]])
}

// ── Fixture ─────────────────────────────────────────────────────────────────

/// `{dir}/work` holds the document and its `schemas/` root; `{dir}/home` is
/// an empty fixture `HOME`. The context is built from that snapshot, never
/// the process directory.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    context: FileResolutionContext,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("work")).expect("work dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        let snapshot = RequestSnapshot::new(root.join("work"))
            .with_home(Some(root.join("home")))
            .with_env(HashMap::new());
        let context = build_resolution_context(&snapshot).expect("fixture context");
        Self { _dir: dir, root, context }
    }

    fn write_disk_schema(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.root.join("work/schemas").join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    fn schemas(&self, bundle: EmbeddedSchemaBundle) -> DarkmatterSchemas {
        DarkmatterSchemas::new(self.context.clone())
            .with_trigger_discovery()
            .and_then(|api| api.with_embedded_trigger_bundle(bundle))
            .expect("discovery")
    }
}

/// What a test compares across bundles: the installed trigger sources, the
/// findings' sources, and, per sample frontmatter, the effective JSON Schema
/// and the validation verdict (`None` when no schema applies).
#[derive(Debug, PartialEq)]
struct Discovered {
    installed: Vec<TriggerSource>,
    findings: Vec<TriggerSource>,
    results: Vec<(String, Option<(Value, bool)>)>,
}

const HOST_VALID: &str = "prompt: x\nmodel: m\nstack: [say, stop]\nretries: 2\n";
const HOST_BAD_STACK: &str = "prompt: x\nstack: [jump]\n";
const HOST_NUMERIC_MODEL: &str = "prompt: x\nmodel: 3\n";
const NOTES_VALID: &str = "topic: t\npages: 3\n";
const NOTES_ZERO_PAGES: &str = "topic: t\npages: 0\n";
const NOTES_TEXT_PAGES: &str = "topic: t\npages: x\n";

const SAMPLES: &[&str] = &[
    HOST_VALID,
    HOST_BAD_STACK,
    HOST_NUMERIC_MODEL,
    NOTES_VALID,
    NOTES_ZERO_PAGES,
    NOTES_TEXT_PAGES,
    "marker: m\n",
    "unrelated: true\n",
];

fn discover(api: &DarkmatterSchemas) -> Discovered {
    let registry = api.trigger_registry().expect("registry");
    Discovered {
        installed: registry.triggers.iter().map(|trigger| trigger.source.clone()).collect(),
        findings: api.discovery_findings().iter().map(|finding| finding.source().clone()).collect(),
        results: SAMPLES
            .iter()
            .map(|frontmatter| {
                let markdown = Markdown::from(format!("---\n{frontmatter}---\nbody\n").as_str());
                let result = api.effective_for(&markdown).expect("effective schema").map(|effective| {
                    let valid = api.validate(&markdown).expect("validation").valid;
                    ((*effective.json_schema).clone(), valid)
                });
                ((*frontmatter).to_string(), result)
            })
            .collect(),
    }
}

fn verdict(discovered: &Discovered, frontmatter: &str) -> Option<bool> {
    discovered
        .results
        .iter()
        .find(|(sample, _)| sample == frontmatter)
        .unwrap_or_else(|| panic!("`{frontmatter}` is a sample"))
        .1
        .as_ref()
        .map(|(_, valid)| *valid)
}

fn embedded(name: &str) -> TriggerSource {
    TriggerSource::Embedded { bundle: BUNDLE.into(), name: name.into() }
}

// ── Discovery ───────────────────────────────────────────────────────────────

#[test]
fn a_prepared_bundle_discovers_what_its_sources_discover_without_the_payload_checks() {
    let fixture = Fixture::new();
    let prepared = accepted_bundle().expect("accepted");
    assert!(prepared.matches(&PREPARED) && prepared.matches(&SOURCE_ONLY), "the bundle is the one prepared");
    assert_eq!(PREPARED, SOURCE_ONLY, "a descriptor does not change what the bundle is");

    let before = checks();
    let from_sources = discover(&fixture.schemas(SOURCE_ONLY));
    if let Some([merge, compile, _]) = checks_since(before) {
        assert_eq!((merge, compile), (2, 2), "the source-only bundle checks its two payloads");
    }

    let before = checks();
    let from_record = discover(&fixture.schemas(PREPARED));
    if let Some([merge, compile, _]) = checks_since(before) {
        assert_eq!((merge, compile), (0, 0), "accepted entries skip the payload checks");
    }

    assert_eq!(from_record, from_sources);
    assert_eq!(
        from_record.installed,
        [embedded("host.yaml"), embedded("marker.yaml"), embedded("notes.yaml")],
        "only the three triggers install, in file order"
    );
    assert!(from_record.findings.is_empty(), "{:?}", from_record.findings);
    assert_eq!(verdict(&from_record, HOST_VALID), Some(true));
    assert_eq!(verdict(&from_record, HOST_BAD_STACK), Some(false), "`stack` is typed by the bundle's library");
    assert_eq!(verdict(&from_record, NOTES_VALID), Some(true));
    assert_eq!(verdict(&from_record, NOTES_ZERO_PAGES), Some(false), "`pages` is typed by the referenced schema");
}

/// The test bundle with `host.yaml` typing `model` as a number.
const CHANGED_HOST: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
    $schema:\n  model: number\n  stack: stack@./lifecycle.yaml\n  retries: count@darkmatter:lifecycle\n";
const CHANGED_FILES: &[(&str, &str)] = &[
    ("host.yaml", CHANGED_HOST),
    ("lifecycle.yaml", LIFECYCLE),
    ("marker.yaml", MARKER),
    ("notes-schema.yaml", NOTES_SCHEMA),
    ("notes.yaml", NOTES),
];

#[test]
fn a_changed_bundle_of_the_same_name_takes_ordinary_preparation() {
    let fixture = Fixture::new();
    let changed = EmbeddedSchemaBundle::new(BUNDLE, CHANGED_FILES).with_prepared(&DESCRIPTOR);
    let prepared = accepted_bundle().expect("accepted");
    assert!(!prepared.matches(&changed), "one changed text refuses the whole bundle");
    // A missing document refuses it too.
    assert!(!prepared.matches(&EmbeddedSchemaBundle::new(BUNDLE, &FILES[..4])));

    let before = checks();
    let discovered = discover(&fixture.schemas(changed));
    if let Some([merge, compile, runtime]) = checks_since(before) {
        assert_eq!((merge, compile), (2, 2), "every payload is checked again");
        assert!(runtime >= FILES.len() as u64, "each document falls back ({runtime})");
    }
    assert_eq!(verdict(&discovered, HOST_NUMERIC_MODEL), Some(true), "the changed text applies: `model: 3`");
    assert_eq!(verdict(&discovered, HOST_VALID), Some(false), "`model: m` is no longer a number");
    assert_eq!(verdict(&discovered, NOTES_ZERO_PAGES), Some(false), "unchanged siblings still apply");
}

/// A disk trigger named like the bundle's `notes.yaml`.
const DISK_NOTES: &str = "kind: schema-trigger\nmatch:\n  topic: string(required)\n$schema:\n  pages: string\n";
/// A disk library named like the bundle's `lifecycle.yaml`, typing `stack`
/// differently.
const DISK_LIFECYCLE: &str = "kind: schema\ntypes:\n  stack: \"enum(jump)[]\"\n";

#[test]
fn a_disk_file_suppresses_its_prepared_document_while_its_siblings_stay_prepared() {
    let fixture = Fixture::new();
    let disk_notes = fixture.write_disk_schema("notes.yaml", DISK_NOTES);
    fixture.write_disk_schema("lifecycle.yaml", DISK_LIFECYCLE);
    accepted_bundle().expect("accepted");

    let before = checks();
    let from_record = discover(&fixture.schemas(PREPARED));
    if let Some([merge, compile, _]) = checks_since(before) {
        assert_eq!((merge, compile), (1, 1), "only the disk trigger's payload is checked");
    }
    assert_eq!(
        from_record.installed,
        [TriggerSource::File(disk_notes), embedded("host.yaml"), embedded("marker.yaml")],
        "the disk file claims `notes.yaml`; the library on disk is not a trigger"
    );
    assert_eq!(
        verdict(&from_record, NOTES_TEXT_PAGES),
        Some(true),
        "the disk `notes.yaml` payload applies, the embedded one does not"
    );
    assert_eq!(verdict(&from_record, HOST_VALID), Some(true), "`stack@./lifecycle.yaml` is still the bundle's");
    assert_eq!(verdict(&from_record, HOST_BAD_STACK), Some(false), "the disk `lifecycle.yaml` is never imported");

    assert_eq!(discover(&fixture.schemas(SOURCE_ONLY)), from_record, "the source-only bundle agrees");
}

// ── Source selection ────────────────────────────────────────────────────────

/// One validation problem: its instance path, code, and missing property.
type Problem = (String, ValidationProblemCode, Option<String>);

/// What judged one document: the trigger layers that applied, the trigger
/// each property's origin names (`None` for a non-trigger origin), and the
/// validation problems sorted by path. `None` when no schema applies.
#[derive(Debug, PartialEq)]
struct Judged {
    layers: Vec<TriggerSource>,
    origins: Vec<(String, Option<TriggerSource>)>,
    problems: Vec<Problem>,
}

fn judge(api: &DarkmatterSchemas, markdown: &Markdown) -> Option<Judged> {
    let effective = api.effective_for(markdown).expect("effective schema")?;
    let mut problems: Vec<Problem> = api
        .validate(markdown)
        .expect("validation")
        .problems
        .into_iter()
        .map(|problem| (problem.path, problem.code, problem.property))
        .collect();
    problems.sort();
    Some(Judged {
        layers: effective.layers().triggers.clone(),
        origins: effective
            .origins
            .iter()
            .map(|(property, origin)| (property.clone(), origin.trigger_source()))
            .collect(),
        problems,
    })
}

fn judge_frontmatter(api: &DarkmatterSchemas, frontmatter: &str) -> Option<Judged> {
    judge(api, &Markdown::from(format!("---\n{frontmatter}---\nbody\n").as_str()))
}

fn problem(path: &str, code: ValidationProblemCode) -> Problem {
    (path.to_string(), code, None)
}

/// Discovery of `bundle` in `fixture` and the checks it ran.
fn discover_counted(fixture: &Fixture, bundle: EmbeddedSchemaBundle) -> (DarkmatterSchemas, Option<Checks>) {
    let before = checks();
    let api = fixture.schemas(bundle);
    (api, checks_since(before))
}

/// Matches both the bundle's `host.yaml` and a `notes.yaml` trigger, with a
/// value each of them rejects.
const MIXED: &str = "prompt: x\ntopic: t\npages: [1]\nstack: [jump]\nmodel: m\n";

#[test]
fn a_mixed_registry_names_each_winning_trigger_and_checks_only_the_disk_payload() {
    let fixture = Fixture::new();
    let disk_notes = fixture.write_disk_schema("notes.yaml", DISK_NOTES);
    fixture.write_disk_schema("lifecycle.yaml", DISK_LIFECYCLE);
    accepted_bundle().expect("accepted");

    let (api, counted) = discover_counted(&fixture, PREPARED);
    if let Some(counted) = counted {
        assert_eq!(counted, [1, 1, 0], "the disk trigger alone is checked; nothing falls back");
    }
    let before = checks();
    let judged = judge_frontmatter(&api, MIXED).expect("both triggers apply");
    if let Some([_, _, runtime]) = checks_since(before) {
        assert_eq!(runtime, 0, "validation prepares no embedded document");
    }

    assert_eq!(judged.layers, [TriggerSource::File(disk_notes.clone()), embedded("host.yaml")], "disk root first");
    let origin = |property: &str| {
        judged
            .origins
            .iter()
            .find(|(name, _)| name == property)
            .unwrap_or_else(|| panic!("`{property}` is declared: {:?}", judged.origins))
            .1
            .clone()
    };
    assert_eq!(origin("pages"), Some(TriggerSource::File(disk_notes)), "the disk `notes.yaml` declares `pages`");
    for property in ["model", "stack", "retries"] {
        assert_eq!(origin(property), Some(embedded("host.yaml")), "`{property}` comes from the prepared entry");
    }
    assert!(
        judged.origins.iter().all(|(_, source)| *source != Some(embedded("notes.yaml"))),
        "the suppressed embedded `notes.yaml` declares nothing: {:?}",
        judged.origins
    );
    assert_eq!(
        judged.problems,
        [
            problem("/pages", ValidationProblemCode::TypeMismatch),
            problem("/stack/0", ValidationProblemCode::ConstraintViolation),
        ],
        "`pages` is the disk payload's string; `stack` is the bundle library's enum"
    );

    assert_eq!(judge_frontmatter(&fixture.schemas(SOURCE_ONLY), MIXED), Some(judged), "the source-only bundle agrees");
}

/// A non-trigger file named like the bundle's `host.yaml`.
const DISK_IGNORED_HOST: &str = "kind: schema\ntypes:\n  verb: string\n";
/// An invalid trigger named like the bundle's `marker.yaml`.
const DISK_INVALID_MARKER: &str = "kind: schema-trigger\nmatch: 42\n$schema: {}\n";

#[test]
fn an_ignored_or_invalid_same_name_disk_file_suppresses_its_prepared_trigger() {
    let fixture = Fixture::new();
    fixture.write_disk_schema("host.yaml", DISK_IGNORED_HOST);
    let disk_marker = fixture.write_disk_schema("marker.yaml", DISK_INVALID_MARKER);
    accepted_bundle().expect("accepted");

    let (api, counted) = discover_counted(&fixture, PREPARED);
    if let Some(counted) = counted {
        assert_eq!(counted, [0, 0, 0], "the surviving `notes.yaml` is served; the disk files check no payload");
    }
    let installed: Vec<TriggerSource> =
        api.trigger_registry().expect("registry").triggers.iter().map(|trigger| trigger.source.clone()).collect();
    assert_eq!(installed, [embedded("notes.yaml")], "neither suppressed embedded trigger installs");
    let [finding] = api.discovery_findings() else {
        panic!("one finding expected: {:?}", api.discovery_findings());
    };
    assert_eq!(finding.source(), &TriggerSource::File(disk_marker));
    assert!(matches!(finding.cause(), SchemaError::TriggerMatch { .. }), "{:?}", finding.cause());

    assert_eq!(judge_frontmatter(&api, HOST_BAD_STACK), None, "no `host.yaml` trigger applies");
    assert_eq!(judge_frontmatter(&api, "marker: m\n"), None, "no `marker.yaml` trigger applies");
    assert_eq!(
        judge_frontmatter(&api, NOTES_ZERO_PAGES).map(|judged| judged.problems),
        Some(vec![problem("/pages", ValidationProblemCode::ConstraintViolation)])
    );

    assert_eq!(discover(&fixture.schemas(SOURCE_ONLY)), discover(&api), "the source-only bundle agrees");
}

#[test]
fn a_prepared_trigger_imports_the_bundle_library_never_a_same_name_disk_file() {
    let fixture = Fixture::new();
    fixture.write_disk_schema("lifecycle.yaml", DISK_LIFECYCLE);
    accepted_bundle().expect("accepted");

    let (api, counted) = discover_counted(&fixture, PREPARED);
    if let Some(counted) = counted {
        assert_eq!(counted, [0, 0, 0], "`host.yaml` is served though its import's name is claimed on disk");
    }
    let judged = judge_frontmatter(&api, HOST_BAD_STACK).expect("`host.yaml` applies");
    assert_eq!(judged.layers, [embedded("host.yaml")]);
    assert_eq!(
        judged.problems,
        [problem("/stack/0", ValidationProblemCode::ConstraintViolation)],
        "`jump` is outside the bundle library's enum, though the disk library allows only it"
    );
    let accepted = "prompt: x\nstack: [say, stop]\n";
    assert_eq!(judge_frontmatter(&api, accepted).map(|judged| judged.problems), Some(Vec::new()));

    let from_sources = fixture.schemas(SOURCE_ONLY);
    assert_eq!(judge_frontmatter(&from_sources, HOST_BAD_STACK), Some(judged), "the source-only bundle agrees");
    assert_eq!(discover(&from_sources), discover(&api));
}

/// The test bundle with `lifecycle.yaml` typing `stack` as `enum(jump)[]`
/// and every trigger text unchanged.
const CHANGED_LIBRARY_FILES: &[(&str, &str)] = &[
    ("host.yaml", HOST),
    ("lifecycle.yaml", DISK_LIFECYCLE),
    ("marker.yaml", MARKER),
    ("notes-schema.yaml", NOTES_SCHEMA),
    ("notes.yaml", NOTES),
];

#[test]
fn a_bundle_whose_imported_library_changed_is_not_served() {
    let fixture = Fixture::new();
    let changed = EmbeddedSchemaBundle::new(BUNDLE, CHANGED_LIBRARY_FILES).with_prepared(&DESCRIPTOR);
    assert!(!accepted_bundle().expect("accepted").matches(&changed), "the library's text is fingerprinted");

    let (api, counted) = discover_counted(&fixture, changed);
    if let Some(counted) = counted {
        assert_eq!(counted, [2, 2, CHANGED_LIBRARY_FILES.len() as u64], "every document is prepared again");
    }
    assert_eq!(
        judge_frontmatter(&api, HOST_BAD_STACK).map(|judged| judged.problems),
        Some(Vec::new()),
        "the changed library admits `jump`"
    );
    let judged = judge_frontmatter(&api, HOST_VALID).expect("`host.yaml` applies");
    assert_eq!(judged.layers, [embedded("host.yaml")]);
    assert_eq!(
        judged.problems,
        [
            problem("/stack/0", ValidationProblemCode::ConstraintViolation),
            problem("/stack/1", ValidationProblemCode::ConstraintViolation),
        ],
        "the recorded library's `say` and `stop` no longer apply"
    );
}

/// The test bundle with one more trigger.
const EXTRA_FILES: &[(&str, &str)] = &[
    ("host.yaml", HOST),
    ("lifecycle.yaml", LIFECYCLE),
    ("marker.yaml", MARKER),
    ("notes-schema.yaml", NOTES_SCHEMA),
    ("notes.yaml", NOTES),
    ("zeta.yaml", "kind: schema-trigger\nmatch:\n  zeta: string(required)\n"),
];
/// The test bundle without `marker.yaml`.
const FEWER_FILES: &[(&str, &str)] = &[
    ("host.yaml", HOST),
    ("lifecycle.yaml", LIFECYCLE),
    ("notes-schema.yaml", NOTES_SCHEMA),
    ("notes.yaml", NOTES),
];

/// Discovers `files` as a same-name bundle carrying [`DESCRIPTOR`] and
/// asserts it takes ordinary preparation, installing `triggers`. Run once per
/// process: ordinary preparation is memoized for the process
/// (`EMBEDDED_LOADED`), so a second same-text bundle would skip the checks
/// this counts.
fn assert_not_served(files: &'static [(&'static str, &'static str)], triggers: &[&str]) {
    let fixture = Fixture::new();
    let custom = EmbeddedSchemaBundle::new(BUNDLE, files).with_prepared(&DESCRIPTOR);
    assert!(!accepted_bundle().expect("accepted").matches(&custom), "a different file set matched");

    let (api, counted) = discover_counted(&fixture, custom);
    if let Some(counted) = counted {
        assert_eq!(counted, [2, 2, files.len() as u64], "every document is prepared again");
    }
    let expected: Vec<TriggerSource> = triggers.iter().map(|name| embedded(name)).collect();
    assert_eq!(discover(&api).installed, expected);
    assert_eq!(
        judge_frontmatter(&api, HOST_BAD_STACK).map(|judged| judged.problems),
        Some(vec![problem("/stack/0", ValidationProblemCode::ConstraintViolation)])
    );
    assert_eq!(
        discover(&api),
        discover(&fixture.schemas(EmbeddedSchemaBundle::new(BUNDLE, files))),
        "the source-only bundle agrees"
    );
}

#[test]
fn a_same_name_bundle_with_an_added_file_is_not_served() {
    assert_not_served(EXTRA_FILES, &["host.yaml", "marker.yaml", "notes.yaml", "zeta.yaml"]);
}

#[test]
fn a_same_name_bundle_with_a_removed_file_is_not_served() {
    assert_not_served(FEWER_FILES, &["host.yaml", "notes.yaml"]);
}

/// A host whose one trigger matches only on `$path`.
const PATH_FILES: &[(&str, &str)] =
    &[("docs.yaml", "kind: schema-trigger\nmatch:\n  $path: \"docs/*.md\"\n$schema:\n  title: string(required)\n")];
const PATH_SOURCE_ONLY: EmbeddedSchemaBundle = EmbeddedSchemaBundle::new("path-host", PATH_FILES);
static PATH_HOST: Host = Host::new(PATH_SOURCE_ONLY);
const PATH_DESCRIPTOR: PreparedBundleDescriptor = PreparedBundleDescriptor { owner: OWNER, provider: path_accepted };

/// [`PATH_DESCRIPTOR`]'s provider.
fn path_accepted() -> Option<&'static PreparedTriggerBundle> {
    PATH_HOST.accepted()
}

#[test]
fn a_prepared_path_condition_matches_in_each_registry_s_own_context() {
    let fixtures = [Fixture::new(), Fixture::new()];
    let documents: Vec<Markdown> = fixtures
        .iter()
        .map(|fixture| {
            let path = fixture.context.base_dir().join("docs/page.md");
            Markdown::from("---\nother: 1\n---\nbody\n").with_source(ComposeSource::File(path))
        })
        .collect();
    PATH_HOST.accepted().expect("accepted");
    let docs = TriggerSource::Embedded { bundle: "path-host".into(), name: "docs.yaml".into() };

    for (label, bundle) in [("prepared", PATH_SOURCE_ONLY.with_prepared(&PATH_DESCRIPTOR)), ("source-only", PATH_SOURCE_ONLY)] {
        for (registry, fixture) in fixtures.iter().enumerate() {
            let (api, counted) = discover_counted(fixture, bundle);
            if let Some([merge, compile, runtime]) = counted
                && bundle.prepared().is_some()
            {
                assert_eq!([merge, compile, runtime], [0, 0, 0], "registry {registry}: the entry is served");
            }
            for (index, document) in documents.iter().enumerate() {
                let judged = judge(&api, document);
                if index == registry {
                    let judged = judged.unwrap_or_else(|| panic!("{label}: registry {registry} misses its own document"));
                    assert_eq!(judged.layers, std::slice::from_ref(&docs), "{label}");
                    assert_eq!(
                        judged.problems,
                        [(String::new(), ValidationProblemCode::MissingRequired, Some("title".into()))],
                        "{label}"
                    );
                } else {
                    assert_eq!(judged, None, "{label}: registry {registry} matched document {index} of another tree");
                }
            }
        }
    }
}

/// A second host bundle claiming `host.yaml` and adding `zeta.yaml`.
const SECOND_FILES: &[(&str, &str)] = &[
    ("host.yaml", "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  owner: string\n"),
    ("zeta.yaml", "kind: schema-trigger\nmatch:\n  zeta: string(required)\n"),
];
const SECOND: EmbeddedSchemaBundle = EmbeddedSchemaBundle::new("second", SECOND_FILES);

/// Discovery with `bundles` appended in order, as the comparable result and
/// the checks it ran.
fn discover_bundles(fixture: &Fixture, bundles: &[EmbeddedSchemaBundle]) -> (Discovered, Option<Checks>) {
    let before = checks();
    let mut api = DarkmatterSchemas::new(fixture.context.clone()).with_trigger_discovery().expect("discovery");
    for bundle in bundles {
        api = api.with_embedded_trigger_bundle(*bundle).expect("bundle registers");
    }
    let counted = checks_since(before);
    (discover(&api), counted)
}

#[test]
fn bundle_order_and_case_folding_are_unchanged_by_a_prepared_descriptor() {
    let fixture = Fixture::new();
    accepted_bundle().expect("accepted");
    let second = |name: &str| TriggerSource::Embedded { bundle: "second".into(), name: name.into() };

    // Registered first, the prepared bundle claims `host.yaml`.
    let (first, counted) = discover_bundles(&fixture, &[PREPARED, SECOND]);
    if let Some(counted) = counted {
        assert_eq!(counted, [0, 0, 0], "the prepared bundle is served; `second` adds a matcher-only trigger");
    }
    assert_eq!(
        first.installed,
        [embedded("host.yaml"), embedded("marker.yaml"), embedded("notes.yaml"), second("zeta.yaml")]
    );
    assert_eq!(first, discover_bundles(&fixture, &[SOURCE_ONLY, SECOND]).0, "the source-only bundle agrees");

    // Registered second, it defers `host.yaml` and serves its other entries.
    let (last, counted) = discover_bundles(&fixture, &[SECOND, PREPARED]);
    if let Some(counted) = counted {
        assert_eq!(counted, [1, 1, 0], "only `second`'s `host.yaml` payload is checked");
    }
    assert_eq!(
        last.installed,
        [second("host.yaml"), second("zeta.yaml"), embedded("marker.yaml"), embedded("notes.yaml")]
    );
    assert_eq!(verdict(&last, HOST_BAD_STACK), Some(true), "the prepared `stack` type does not apply");
    assert_eq!(last, discover_bundles(&fixture, &[SECOND, SOURCE_ONLY]).0, "the source-only bundle agrees");

    // A disk file differing only in case claims nothing: a name is claimed
    // exactly, with or without a descriptor.
    let disk_host = fixture.write_disk_schema(
        "HOST.yaml",
        "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  owner: string\n",
    );
    let (cased, _) = discover_bundles(&fixture, &[PREPARED]);
    assert_eq!(
        cased.installed,
        [TriggerSource::File(disk_host), embedded("host.yaml"), embedded("marker.yaml"), embedded("notes.yaml")]
    );
    assert_eq!(cased, discover_bundles(&fixture, &[SOURCE_ONLY]).0, "the source-only bundle agrees");

    // A bundle whose names collide after case-folding is refused either way.
    const COLLIDING: &[(&str, &str)] = &[("Host.yaml", HOST), ("host.yaml", HOST)];
    let refusal = |bundle: EmbeddedSchemaBundle| {
        match DarkmatterSchemas::new(fixture.context.clone()).with_embedded_trigger_bundle(bundle) {
            Err(error @ SchemaError::EmbeddedBundle { .. }) => error.to_string(),
            Err(other) => panic!("an unexpected refusal: {other:?}"),
            Ok(_) => panic!("a colliding bundle registered"),
        }
    };
    let colliding = EmbeddedSchemaBundle::new(BUNDLE, COLLIDING);
    assert_eq!(refusal(colliding.with_prepared(&DESCRIPTOR)), refusal(colliding));
}

// ── Generation ──────────────────────────────────────────────────────────────

#[test]
fn generation_records_each_document_in_file_order() {
    let record = prepare_trigger_bundle(SOURCE_ONLY, PreparationMode::Uncached(&generated().fresh)).expect("prepares");
    let outcomes: Vec<(&str, &str)> = record
        .documents
        .iter()
        .map(|document| {
            let outcome = match &document.outcome {
                TriggerOutcomeRecord::NotTrigger => "not-trigger",
                TriggerOutcomeRecord::MatcherOnly => "matcher-only",
                TriggerOutcomeRecord::Payload(_) => "payload",
            };
            (document.name.as_str(), outcome)
        })
        .collect();
    assert_eq!(
        outcomes,
        [
            ("host.yaml", "payload"),
            ("lifecycle.yaml", "not-trigger"),
            ("marker.yaml", "matcher-only"),
            ("notes-schema.yaml", "not-trigger"),
            ("notes.yaml", "payload"),
        ]
    );
    assert_eq!(record.bundle, BUNDLE);
}

const INVALID_FILES: &[(&str, &str)] = &[
    ("a.yaml", "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  model: string\n"),
    ("z.yaml", "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: ./missing.yaml\n"),
];

#[test]
fn generation_fails_on_an_invalid_trigger_after_running_the_checks() {
    let before = checks();
    let result =
        prepare_trigger_bundle(EmbeddedSchemaBundle::new(BUNDLE, INVALID_FILES), PreparationMode::Uncached(&generated().fresh));
    let Err(SchemaError::EmbeddedBundle { bundle, message }) = result else {
        panic!("generation must fail: {result:?}");
    };
    assert_eq!(bundle, BUNDLE);
    assert!(message.contains("`z.yaml`"), "the failing document is named: {message}");
    if let Some([merge, compile, _]) = checks_since(before) {
        assert!(merge > 0 && compile > 0, "uncached generation runs the payload checks ({merge}, {compile})");
    }
}

const TEMPLATE_FILES: &[(&str, &str)] = &[(
    "template.yaml",
    "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  pages: \"number(min({{ low }}))\"\n",
)];

#[test]
fn generation_refuses_a_template_payload() {
    let result =
        prepare_trigger_bundle(EmbeddedSchemaBundle::new(BUNDLE, TEMPLATE_FILES), PreparationMode::Uncached(&generated().fresh));
    let Err(SchemaError::EmbeddedBundle { message, .. }) = result else {
        panic!("a template is never prepared: {result:?}");
    };
    assert!(message.contains("`template.yaml`") && message.contains("slots"), "{message}");
}

// ── Record validation ───────────────────────────────────────────────────────

/// The generated artifact as JSON, `edit`ed, then offered for acceptance.
fn accept_edited(edit: impl FnOnce(&mut Value)) -> Result<(), Refusal> {
    let mut artifact: Value = serde_json::from_slice(&generated().bytes).expect("artifact JSON");
    edit(&mut artifact);
    let bytes = serde_json::to_vec_pretty(&artifact).expect("encode");
    accept_bytes(&bytes).map(|_| ())
}

fn documents(artifact: &mut Value) -> &mut Vec<Value> {
    artifact["payload"]["bundle"]["documents"].as_array_mut().expect("documents")
}

/// The record of `host.yaml`'s payload.
fn host_payload(artifact: &mut Value) -> &mut serde_json::Map<String, Value> {
    documents(artifact)[0]["outcome"]["Payload"].as_object_mut().expect("payload record")
}

/// One edit of the generated artifact.
type Edit = Box<dyn FnOnce(&mut Value)>;

#[test]
fn record_acceptance_refuses_each_invalid_shape() {
    assert_eq!(accept_edited(|_| {}), Ok(()), "the unedited artifact is accepted");
    assert_eq!(
        accept_edited(|artifact| {
            host_payload(artifact).insert("shape".into(), Value::Null);
        }),
        Ok(()),
        "an explicit null `shape` is `None`"
    );

    let cases: Vec<(&str, FallbackReason, Edit)> = vec![
        (
            "unknown document",
            FallbackReason::UnusableRecord,
            Box::new(|artifact| documents(artifact)[4]["name"] = "unknown.yaml".into()),
        ),
        (
            "duplicate document",
            FallbackReason::UnusableRecord,
            Box::new(|artifact| documents(artifact)[1]["name"] = "host.yaml".into()),
        ),
        (
            "document out of order",
            FallbackReason::UnusableRecord,
            Box::new(|artifact| documents(artifact).swap(0, 1)),
        ),
        (
            "missing document",
            FallbackReason::UnusableRecord,
            Box::new(|artifact| {
                documents(artifact).remove(2);
            }),
        ),
        (
            "unknown outcome tag",
            FallbackReason::MalformedArtifact,
            Box::new(|artifact| documents(artifact)[2]["outcome"] = "Template".into()),
        ),
        (
            "absent `shape`",
            FallbackReason::MalformedArtifact,
            Box::new(|artifact| {
                host_payload(artifact).remove("shape");
            }),
        ),
        (
            "non-object JSON Schema",
            FallbackReason::UnusableRecord,
            Box::new(|artifact| {
                host_payload(artifact).insert("json_schema".into(), Value::Bool(true));
            }),
        ),
        (
            "unknown record field",
            FallbackReason::MalformedArtifact,
            Box::new(|artifact| {
                host_payload(artifact).insert("dependencies".into(), Value::Array(Vec::new()));
            }),
        ),
    ];
    for (case, reason, edit) in cases {
        let refusal = accept_edited(edit).expect_err(case);
        assert_eq!(refusal.reason, reason, "{case}: {refusal}");
        assert!(refusal.pointer.as_deref().is_some_and(|pointer| pointer.starts_with("/payload")), "{case}: {refusal}");
    }
}
