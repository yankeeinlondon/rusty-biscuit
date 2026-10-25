//! Schema-trigger discovery recovers per trigger: an invalid trigger is left
//! out of the registry and reported as an [`InvalidTriggerFinding`], every
//! valid neighbor is still installed, and only infrastructure failures fail
//! the scan.
//!
//! Each test builds an isolated repository in a temporary directory and
//! judges discovery through the public scan and [`DarkmatterSchemas`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::triggers::{
    self, PayloadMemoCounts, PayloadMemoTally, TriggerRegistry,
};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EmbeddedSchemaBundle, InvalidTriggerFinding, SchemaError, TriggerSource,
};

/// A valid trigger selecting documents with a `title`, layering `level: number`.
const GOOD_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  level: number\n";

/// A trigger selecting documents with a `title` whose payload file is absent.
const MISSING_PAYLOAD_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: ./missing.yaml\n";

/// A trigger selecting documents with a `title` whose payload is `payload`.
fn trigger_with_payload(payload: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: {payload}\n")
}

/// A temporary repository with a `docs/` folder for documents.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        std::fs::create_dir_all(root.join("docs")).expect("docs folder");
        let repo = Self { _dir: dir, root };
        for (relative, contents) in files {
            repo.write(relative, contents);
        }
        repo
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, contents).expect("write fixture");
    }

    /// The context of a document in `docs/`.
    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.path("docs"), None, HashMap::new()).with_repository_root(&self.root)
    }

    fn scan(&self) -> TriggerRegistry {
        triggers::scan(&self.context()).expect("an invalid trigger never fails discovery")
    }

    fn schemas(&self) -> DarkmatterSchemas {
        DarkmatterSchemas::new(self.context())
            .with_trigger_discovery()
            .expect("an invalid trigger never fails discovery")
    }

    /// Writes `docs/{name}` with `frontmatter` and loads it.
    fn document(&self, name: &str, frontmatter: &str) -> Markdown {
        let path = self.path("docs").join(name);
        std::fs::write(&path, format!("---\n{frontmatter}---\nBody.\n")).expect("write document");
        Markdown::try_from(path.as_path()).expect("document").with_source(ComposeSource::File(path))
    }
}

fn file_sources(registry: &TriggerRegistry) -> Vec<PathBuf> {
    registry.triggers.iter().filter_map(|trigger| trigger.source.path().map(Path::to_path_buf)).collect()
}

fn finding_sources(findings: &[InvalidTriggerFinding]) -> Vec<TriggerSource> {
    findings.iter().map(|finding| finding.source().clone()).collect()
}

fn the_finding(findings: &[InvalidTriggerFinding]) -> &InvalidTriggerFinding {
    let [finding] = findings else {
        panic!("one finding expected: {findings:?}");
    };
    finding
}

/// The effective schema's property names for `markdown`.
fn effective_properties(schemas: &DarkmatterSchemas, markdown: &Markdown) -> Vec<String> {
    schemas
        .effective_for(markdown)
        .expect("effective schema")
        .and_then(|effective| effective.json_schema["properties"].as_object().map(|map| map.keys().cloned().collect()))
        .unwrap_or_default()
}

#[test]
fn an_envelope_with_unknown_keys_is_a_finding_and_validation_stays_vacuous() {
    let trigger = "schemas/memory.yaml";
    let repo = Repo::new(&[(trigger, "kind: schema-trigger\nname: Memory Trigger\ndescription: TODO\nmatch:\n")]);

    let schemas = repo.schemas();

    assert!(schemas.trigger_registry().expect("registry").triggers.is_empty());
    let finding = the_finding(schemas.discovery_findings());
    assert_eq!(finding.source(), &TriggerSource::File(repo.path(trigger)));
    assert_eq!(finding.code(), "dm.schema.invalid_trigger");
    let message = finding.message();
    assert!(message.contains("ignoring invalid schema trigger"), "{message}");
    assert!(message.contains(&repo.path(trigger).display().to_string()), "{message}");
    assert!(message.contains("unknown keys: name, description"), "the unknown keys are named: {message}");

    let document = repo.document("prompt.md", "title: hello\n");
    assert!(schemas.effective_for(&document).expect("no effective schema").is_none());
    let report = schemas.validate(&document).expect("validation succeeds");
    assert!(report.valid && report.problems.is_empty(), "{report:?}");
    assert_eq!(report.trigger_findings, schemas.discovery_findings());
}

fn assert_valid_trigger_applies_beside_invalid(good: &str, bad: &str) {
    let repo = Repo::new(&[(good, GOOD_TRIGGER), (bad, MISSING_PAYLOAD_TRIGGER)]);

    let schemas = repo.schemas();

    let registry = schemas.trigger_registry().expect("registry");
    assert_eq!(file_sources(registry), [repo.path(good)]);
    assert_eq!(finding_sources(schemas.discovery_findings()), [TriggerSource::File(repo.path(bad))]);
    let accepted = repo.document("ok.md", "title: t\nlevel: 3\n");
    assert!(effective_properties(&schemas, &accepted).contains(&"level".to_string()));
    assert!(schemas.validate(&accepted).expect("validates").valid);
    let rejected = repo.document("bad.md", "title: t\nlevel: high\n");
    let report = schemas.validate(&rejected).expect("validates");
    assert!(!report.valid, "the valid trigger's payload is still enforced: {report:?}");
    assert!(report.problems.iter().any(|problem| problem.instance_path.first() == Some("level")), "{report:?}");
    assert_eq!(report.trigger_findings.len(), 1);
}

#[test]
fn a_valid_trigger_after_an_invalid_one_still_applies() {
    assert_valid_trigger_applies_beside_invalid("schemas/b-good.yaml", "schemas/a-bad.yaml");
}

#[test]
fn a_valid_trigger_before_an_invalid_one_still_applies() {
    assert_valid_trigger_applies_beside_invalid("schemas/a-good.yaml", "schemas/z-bad.yaml");
}

#[test]
fn malformed_claimed_yaml_is_a_positioned_finding() {
    let trigger = "schemas/broken.yaml";
    let repo = Repo::new(&[(trigger, "kind: schema-trigger\nmatch: [unclosed\n")]);

    let registry = repo.scan();

    let finding = the_finding(registry.findings());
    assert_eq!(finding.source(), &TriggerSource::File(repo.path(trigger)));
    assert!(matches!(finding.cause(), SchemaError::TriggerEnvelopeYaml { .. }), "{finding:?}");
    assert!(finding.position().is_some(), "{finding:?}");
}

#[test]
fn malformed_unclaimed_yaml_is_silently_ignored() {
    let repo = Repo::new(&[("schemas/notes.yaml", "title: [unclosed\n")]);

    let registry = repo.scan();

    assert!(registry.triggers.is_empty());
    assert!(registry.findings().is_empty(), "{:?}", registry.findings());
}

#[test]
fn a_missing_payload_is_searched_not_a_dependency_and_creating_it_installs_the_trigger() {
    let trigger = "schemas/host.yaml";
    let repo = Repo::new(&[(trigger, MISSING_PAYLOAD_TRIGGER)]);

    let registry = repo.scan();

    the_finding(registry.findings());
    let missing = repo.path("schemas/missing.yaml");
    assert!(registry.searched_locations().any(|path| path == missing), "the missing payload is searched");
    let dependencies: Vec<&Path> = registry.dependencies().collect();
    assert!(dependencies.contains(&repo.path(trigger).as_path()), "{dependencies:?}");
    assert!(!dependencies.contains(&missing.as_path()), "{dependencies:?}");

    repo.write("schemas/missing.yaml", "$schema:\n  level: number\n");
    let repaired = repo.scan();

    assert_eq!(file_sources(&repaired), [repo.path(trigger)]);
    assert!(repaired.findings().is_empty(), "{:?}", repaired.findings());
}

#[test]
fn creating_a_missing_payload_and_its_parent_folder_installs_the_trigger() {
    let trigger = "schemas/host.yaml";
    let repo = Repo::new(&[(trigger, &trigger_with_payload("./sub/payload.yaml"))]);

    let registry = repo.scan();

    the_finding(registry.findings());
    let payload = repo.path("schemas/sub/payload.yaml");
    assert!(registry.searched_locations().any(|path| path == payload), "the missing payload is searched");

    repo.write("schemas/sub/payload.yaml", "$schema:\n  level: number\n");
    let repaired = repo.scan();

    assert_eq!(file_sources(&repaired), [repo.path(trigger)]);
    assert!(repaired.findings().is_empty(), "{:?}", repaired.findings());
}

#[test]
fn a_broken_import_depends_on_the_files_read_and_repairing_it_installs_the_trigger() {
    let trigger = "schemas/host.yaml";
    let repo = Repo::new(&[
        (trigger, &trigger_with_payload("./payload.yaml")),
        ("schemas/payload.yaml", "$schema:\n  owner: \"Name@./types.yaml\"\n"),
        ("schemas/types.yaml", "kind: schema\ntypes: [unclosed\n"),
    ]);

    let registry = repo.scan();

    let finding = the_finding(registry.findings());
    assert!(registry.triggers.is_empty());
    let dependencies: Vec<&Path> = registry.dependencies().collect();
    let payload = repo.path("schemas/payload.yaml");
    assert!(dependencies.contains(&payload.as_path()), "the payload read before the failure: {dependencies:?}");
    assert!(finding.message().contains("types.yaml"), "{}", finding.message());

    repo.write("schemas/types.yaml", "kind: schema\ntypes:\n  Name: string\n");
    let repaired = repo.scan();

    assert_eq!(file_sources(&repaired), [repo.path(trigger)]);
    assert!(repaired.findings().is_empty(), "{:?}", repaired.findings());
}

#[test]
fn a_raw_payload_that_fails_to_compile_is_skipped_even_when_a_later_layer_redeclares_its_property() {
    let trigger = "schemas/host.yaml";
    let payload = "schemas/bad.json";
    let repo = Repo::new(&[
        (trigger, &trigger_with_payload("./bad.json")),
        (payload, r#"{"type":"object","properties":{"x":{"type":12}}}"#),
    ]);

    let schemas = repo.schemas();

    assert!(schemas.trigger_registry().expect("registry").triggers.is_empty());
    let finding = the_finding(schemas.discovery_findings());
    assert!(matches!(finding.cause(), SchemaError::TriggerPayloadCompile { .. }), "{finding:?}");
    assert_eq!(finding.cause_path(), Some(repo.path(payload).as_path()));

    let redeclared = repo.document("matched.md", "title: t\n$schema:\n  x: string\nx: hello\n");
    let report = schemas.validate(&redeclared).expect("the document's own schema validates");
    assert!(report.valid, "{report:?}");
    assert_eq!(report.trigger_findings, schemas.discovery_findings());
    let unmatched = repo.document("unmatched.md", "other: 1\n");
    let report = schemas.validate(&unmatched).expect("validates");
    assert_eq!(report.trigger_findings, schemas.discovery_findings());
}

#[test]
fn a_root_union_payload_is_not_mergeable_and_a_valid_neighbor_installs() {
    let repo = Repo::new(&[
        ("schemas/a-union.yaml", &trigger_with_payload("./union.yaml")),
        ("schemas/union.yaml", "$schema:\n  - { a: string }\n  - { b: string }\n"),
        ("schemas/b-good.yaml", GOOD_TRIGGER),
    ]);

    let registry = repo.scan();

    let finding = the_finding(registry.findings());
    assert_eq!(finding.source(), &TriggerSource::File(repo.path("schemas/a-union.yaml")));
    assert!(matches!(finding.cause(), SchemaError::TriggerPayloadNotMergeable { .. }), "{finding:?}");
    assert_eq!(file_sources(&registry), [repo.path("schemas/b-good.yaml")]);
}

#[test]
fn a_payload_cycle_is_a_finding_and_a_valid_neighbor_installs() {
    let repo = Repo::new(&[
        ("schemas/a-self.yaml", &trigger_with_payload("./a-self.yaml")),
        ("schemas/b-good.yaml", GOOD_TRIGGER),
    ]);

    let registry = repo.scan();

    let finding = the_finding(registry.findings());
    assert!(matches!(finding.cause(), SchemaError::TriggerPayloadCycle { .. }), "{finding:?}");
    assert_eq!(file_sources(&registry), [repo.path("schemas/b-good.yaml")]);
}

#[test]
fn two_triggers_sharing_a_broken_payload_are_two_findings() {
    let repo = Repo::new(&[
        ("schemas/one.yaml", &trigger_with_payload("./broken.yaml")),
        ("schemas/two.yaml", &trigger_with_payload("./broken.yaml")),
        ("schemas/broken.yaml", "$schema:\n  n: [unclosed\n"),
    ]);

    let registry = repo.scan();

    let [one, two] = registry.findings() else {
        panic!("two findings expected: {:?}", registry.findings());
    };
    assert_ne!(one.identity(), two.identity());
    assert_eq!(
        finding_sources(registry.findings()),
        [TriggerSource::File(repo.path("schemas/one.yaml")), TriggerSource::File(repo.path("schemas/two.yaml"))]
    );
}

#[test]
fn a_nearer_invalid_trigger_shadows_a_same_named_valid_one_until_deleted() {
    let repo = Repo::new(&[("pkg/schemas/x.yaml", MISSING_PAYLOAD_TRIGGER), ("schemas/x.yaml", GOOD_TRIGGER)]);
    std::fs::create_dir_all(repo.path("pkg/docs")).expect("package docs");
    let context = || {
        FileResolutionContext::from_snapshot(repo.path("pkg/docs"), None, HashMap::new())
            .with_repository_root(&repo.root)
            .with_package_root(repo.path("pkg"))
    };

    let registry = triggers::scan(&context()).expect("discovery");

    assert!(registry.triggers.is_empty(), "the shadowed valid trigger is not installed");
    assert_eq!(finding_sources(registry.findings()), [TriggerSource::File(repo.path("pkg/schemas/x.yaml"))]);
    let shadowed: Vec<_> = registry.shadowed.iter().map(|file| (file.path.clone(), file.shadowed_by.clone())).collect();
    assert_eq!(shadowed, [(repo.path("schemas/x.yaml"), repo.path("pkg/schemas/x.yaml"))]);

    std::fs::remove_file(repo.path("pkg/schemas/x.yaml")).expect("delete the invalid trigger");
    let after = triggers::scan(&context()).expect("discovery");

    assert!(after.findings().is_empty(), "{:?}", after.findings());
    assert_eq!(file_sources(&after), [repo.path("schemas/x.yaml")]);
}

const INVALID_BUNDLE: EmbeddedSchemaBundle =
    EmbeddedSchemaBundle::new("b1", &[("t.yaml", "kind: schema-trigger\nmatch:\n")]);
const VALID_BUNDLE: EmbeddedSchemaBundle = EmbeddedSchemaBundle::new("b2", &[("t.yaml", GOOD_TRIGGER)]);

#[test]
fn an_invalid_embedded_trigger_is_a_finding_that_keeps_its_name_claimed() {
    let repo = Repo::new(&[]);

    let registry = triggers::scan_with_embedded(&repo.context(), &[INVALID_BUNDLE, VALID_BUNDLE]).expect("discovery");

    assert!(registry.triggers.is_empty(), "a later bundle never stands in for a skipped name");
    let finding = the_finding(registry.findings());
    assert_eq!(finding.source(), &TriggerSource::Embedded { bundle: "b1".into(), name: "t.yaml".into() });
    assert_eq!(finding.cause_path(), None);
}

#[test]
fn an_invalid_embedded_bundle_structure_still_fails_discovery() {
    const BAD_NAME: EmbeddedSchemaBundle = EmbeddedSchemaBundle::new("a/b", &[("t.yaml", GOOD_TRIGGER)]);
    let repo = Repo::new(&[]);

    let result = triggers::scan_with_embedded(&repo.context(), &[BAD_NAME]);

    assert!(matches!(result, Err(SchemaError::EmbeddedBundle { .. })), "{result:?}");
}

#[test]
fn findings_are_in_filename_order_with_disk_before_embedded() {
    let repo = Repo::new(&[
        ("schemas/c.yaml", "kind: schema-trigger\nmatch: [unclosed\n"),
        ("schemas/a.yaml", MISSING_PAYLOAD_TRIGGER),
        ("schemas/b.yaml", "kind: schema-trigger\nmatch:\n"),
    ]);

    let registry = triggers::scan_with_embedded(&repo.context(), &[INVALID_BUNDLE]).expect("discovery");

    assert_eq!(
        finding_sources(registry.findings()),
        [
            TriggerSource::File(repo.path("schemas/a.yaml")),
            TriggerSource::File(repo.path("schemas/b.yaml")),
            TriggerSource::File(repo.path("schemas/c.yaml")),
            TriggerSource::Embedded { bundle: "b1".into(), name: "t.yaml".into() },
        ]
    );
}

#[cfg(unix)]
#[test]
fn an_unreadable_schema_root_still_fails_discovery() {
    use std::os::unix::fs::PermissionsExt;

    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }

    let repo = Repo::new(&[("schemas/a.yaml", GOOD_TRIGGER)]);
    let schemas = repo.path("schemas");
    std::fs::set_permissions(&schemas, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let _restore = Restore(schemas.clone());
    match std::fs::read_dir(&schemas) {
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {}
        other => {
            eprintln!("skipping: {} is still readable after chmod 000 ({other:?})", schemas.display());
            return;
        }
    }

    let result = triggers::scan(&repo.context());

    assert!(matches!(&result, Err(SchemaError::Io { path, .. }) if *path == schemas), "{result:?}");
}

#[test]
fn an_accepted_payload_is_resolved_once_across_scans() {
    let repo = Repo::new(&[
        ("schemas/host.yaml", &trigger_with_payload("./payload.yaml")),
        ("schemas/payload.yaml", "$schema:\n  level: number\n"),
    ]);
    let first = repo.scan();
    assert_eq!(first.triggers.len(), 1);

    let tally = PayloadMemoTally::start();
    let second = repo.scan();

    assert_eq!(second.triggers.len(), 1);
    assert_eq!(tally.counts(), PayloadMemoCounts { hits: 1, misses: 0 });
}
