//! A trigger whose payload cannot load is skipped with the same finding
//! whether or not any document matches it.
//!
//! Payloads are resolved when the registry is built, before any document is
//! matched, so a known-bad payload is never installed and never waits for a
//! matching document to surface. Discovery itself succeeds: the trigger is
//! reported as an [`InvalidTriggerFinding`] and left out of the registry.
//! Each case builds discovery once from the folder of a document the trigger
//! matches and once from one it does not.

use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EmbeddedSchemaBundle, InvalidTriggerFinding, SchemaError, TriggerSource,
};

/// A trigger matching any document with a `prompt`.
fn trigger(payload: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: {payload}\n")
}

/// A repository holding `files`, a document the trigger matches in
/// `matched/`, and one it does not match in `unmatched/`.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        let documents = [("matched/doc.md", "---\nprompt: x\n---\n"), ("unmatched/doc.md", "---\ntitle: x\n---\n")];
        for (relative, contents) in files.iter().chain(&documents) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, contents).expect("write fixture");
        }
        Self { _dir: dir, root }
    }

    /// The findings of discovery for the document in `folder`, which
    /// succeeds and installs no trigger.
    fn discovery_findings(&self, folder: &str, bundle: Option<EmbeddedSchemaBundle>) -> Vec<InvalidTriggerFinding> {
        let context = FileResolutionContext::from_snapshot(self.root.join(folder), None, HashMap::new())
            .with_repository_root(&self.root);
        let discovered = DarkmatterSchemas::new(context).with_trigger_discovery();
        let loaded = match bundle {
            Some(bundle) => discovered.and_then(|api| api.with_embedded_trigger_bundle(bundle)),
            None => discovered,
        };
        let schemas = loaded.unwrap_or_else(|error| panic!("discovery from `{folder}` must succeed: {error}"));
        assert!(schemas.trigger_registry().expect("registry").triggers.is_empty(), "a bad payload is never installed");
        schemas.discovery_findings().to_vec()
    }

    /// The findings for the matched and the unmatched document, asserted
    /// equal.
    fn same_findings_for_both(&self, bundle: Option<EmbeddedSchemaBundle>) -> Vec<InvalidTriggerFinding> {
        let matched = self.discovery_findings("matched", bundle);
        let unmatched = self.discovery_findings("unmatched", bundle);
        assert_eq!(matched, unmatched);
        matched
    }

    /// The cause of the one finding, which names `trigger`.
    fn payload_error<'a>(&self, findings: &'a [InvalidTriggerFinding], trigger: &str) -> &'a SchemaError {
        let [finding] = findings else {
            panic!("one finding expected: {findings:?}");
        };
        assert_eq!(finding.source(), &TriggerSource::File(self.root.join(trigger)), "the finding names the trigger file");
        assert_eq!(finding.code(), "dm.schema.invalid_trigger");
        finding.cause()
    }
}

#[test]
fn a_payload_file_that_is_not_yaml_is_skipped_for_matched_and_unmatched_documents() {
    let repo = Repo::new(&[
        ("schemas/host.trigger.yaml", &trigger("./payload.yaml")),
        ("schemas/payload.yaml", "$schema:\n  n: [unclosed\n"),
    ]);

    let findings = repo.same_findings_for_both(None);

    let source = repo.payload_error(&findings, "schemas/host.trigger.yaml");
    assert!(source.to_string().contains("payload.yaml"), "names the payload file: {source}");
    let message = findings[0].message();
    assert!(message.contains("host.trigger.yaml") && message.contains("payload.yaml"), "{message}");
}

#[test]
fn an_unresolvable_payload_reference_is_skipped_for_matched_and_unmatched_documents() {
    let repo = Repo::new(&[("schemas/host.trigger.yaml", &trigger("./missing.yaml"))]);

    let findings = repo.same_findings_for_both(None);

    let source = repo.payload_error(&findings, "schemas/host.trigger.yaml");
    assert!(source.to_string().contains("missing.yaml"), "names the reference: {source}");
}

#[test]
fn a_payload_cycle_skips_each_trigger_for_matched_and_unmatched_documents() {
    let repo = Repo::new(&[
        ("schemas/host.trigger.yaml", &trigger("./other.trigger.yaml")),
        ("schemas/other.trigger.yaml", &trigger("./host.trigger.yaml")),
    ]);

    let findings = repo.same_findings_for_both(None);

    // Each trigger of the cycle is its own finding, in discovery order.
    let sources: Vec<_> = findings.iter().map(|finding| finding.source().clone()).collect();
    assert_eq!(
        sources,
        [
            TriggerSource::File(repo.root.join("schemas/host.trigger.yaml")),
            TriggerSource::File(repo.root.join("schemas/other.trigger.yaml")),
        ]
    );
    for finding in &findings {
        assert!(
            matches!(finding.cause(), SchemaError::TriggerPayloadCycle { .. }),
            "a payload naming a trigger is a cycle: {finding:?}"
        );
    }
}

#[test]
fn an_embedded_trigger_with_an_unresolvable_payload_is_skipped_for_matched_and_unmatched_documents() {
    const HOST: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: ./missing.yaml\n";
    let bundle = EmbeddedSchemaBundle::new("host-bundle", &[("host.yaml", HOST)]);
    let repo = Repo::new(&[]);

    let findings = repo.same_findings_for_both(Some(bundle));

    let [finding] = findings.as_slice() else {
        panic!("one finding expected: {findings:?}");
    };
    assert_eq!(
        finding.source(),
        &TriggerSource::Embedded { bundle: "host-bundle".into(), name: "host.yaml".into() },
        "names the bundle trigger"
    );
    assert_eq!(finding.identity(), finding.source(), "an embedded identity is its bundle and document");
    assert_eq!(finding.cause_path(), None, "an embedded identity never becomes a file");
    assert!(finding.message().contains("embedded:host-bundle/host.yaml"), "{}", finding.message());
}
