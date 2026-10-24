//! A trigger whose payload cannot load fails discovery with the same error
//! whether or not any document matches it.
//!
//! Payloads are resolved when the registry is built, before any document is
//! matched, so a known-bad payload is never installed and never waits for a
//! matching document to surface. Each case builds discovery once from the
//! folder of a document the trigger matches and once from one it does not.

use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EmbeddedSchemaBundle, SchemaError};

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

    /// The discovery error for the document in `folder`.
    fn discovery_error(&self, folder: &str, bundle: Option<EmbeddedSchemaBundle>) -> SchemaError {
        let context = FileResolutionContext::from_snapshot(self.root.join(folder), None, HashMap::new())
            .with_repository_root(&self.root);
        let discovered = DarkmatterSchemas::new(context).with_trigger_discovery();
        let loaded = match bundle {
            Some(bundle) => discovered.and_then(|api| api.with_embedded_trigger_bundle(bundle)),
            None => discovered,
        };
        match loaded {
            Ok(_) => panic!("discovery from `{folder}` must fail"),
            Err(error) => error,
        }
    }

    /// The error for the matched and the unmatched document, asserted equal.
    fn same_error_for_both(&self, bundle: Option<EmbeddedSchemaBundle>) -> SchemaError {
        let matched = self.discovery_error("matched", bundle);
        let unmatched = self.discovery_error("unmatched", bundle);
        assert_eq!(matched.to_string(), unmatched.to_string());
        assert_eq!(format!("{matched:?}"), format!("{unmatched:?}"));
        matched
    }

    /// The payload error inside the `TriggerLoad` naming `trigger`.
    fn payload_error(&self, error: SchemaError, trigger: &str) -> SchemaError {
        match error {
            SchemaError::TriggerLoad { path, source } => {
                assert_eq!(path, self.root.join(trigger), "the error names the trigger file");
                *source
            }
            other => panic!("expected TriggerLoad, got {other:?}"),
        }
    }
}

#[test]
fn a_payload_file_that_is_not_yaml_fails_discovery_for_matched_and_unmatched_documents() {
    let repo = Repo::new(&[
        ("schemas/host.trigger.yaml", &trigger("./payload.yaml")),
        ("schemas/payload.yaml", "$schema:\n  n: [unclosed\n"),
    ]);

    let error = repo.same_error_for_both(None);

    let source = repo.payload_error(error, "schemas/host.trigger.yaml");
    assert!(source.to_string().contains("payload.yaml"), "names the payload file: {source}");
}

#[test]
fn an_unresolvable_payload_reference_fails_discovery_for_matched_and_unmatched_documents() {
    let repo = Repo::new(&[("schemas/host.trigger.yaml", &trigger("./missing.yaml"))]);

    let error = repo.same_error_for_both(None);

    let source = repo.payload_error(error, "schemas/host.trigger.yaml");
    assert!(source.to_string().contains("missing.yaml"), "names the reference: {source}");
}

#[test]
fn a_payload_cycle_fails_discovery_for_matched_and_unmatched_documents() {
    let repo = Repo::new(&[
        ("schemas/host.trigger.yaml", &trigger("./other.trigger.yaml")),
        ("schemas/other.trigger.yaml", &trigger("./host.trigger.yaml")),
    ]);

    let error = repo.same_error_for_both(None);

    assert!(
        matches!(&error, SchemaError::TriggerLoad { source, .. } if matches!(**source, SchemaError::TriggerPayloadCycle { .. })),
        "a payload naming a trigger is a cycle: {error:?}"
    );
}

#[test]
fn an_embedded_trigger_with_an_unresolvable_payload_fails_for_matched_and_unmatched_documents() {
    const HOST: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: ./missing.yaml\n";
    let bundle = EmbeddedSchemaBundle::new("host-bundle", &[("host.yaml", HOST)]);
    let repo = Repo::new(&[]);

    let error = repo.same_error_for_both(Some(bundle));

    assert!(
        matches!(&error, SchemaError::EmbeddedTriggerLoad { bundle, name, .. } if bundle == "host-bundle" && name == "host.yaml"),
        "names the bundle trigger: {error:?}"
    );
}
