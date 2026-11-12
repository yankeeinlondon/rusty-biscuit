//! An effective schema's validators are cached under the layered identity of
//! the layers it was merged from (baseline, ordered triggers, document) and
//! the phase it is judged at, never a wrong neighbor's.
//!
//! Each isolation case pairs two schemas whose verdicts on one instance
//! differ, in one process sharing the process-wide validator cache, so a key
//! that ignored the difference would hand the second case the first one's
//! validator and flip its verdict. The sharing cases count compiled
//! validators, and the serialization cases count canonical schema
//! serializations (`Work::CanonicalSchemaSerialization`). Every context is
//! anchored in the test's own temporary folder.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeSource, DataRuntime, RequestSnapshot};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::SlotSession;
use simplified_schema::{SchemaPhase, parse_yaml_schema};

/// A temporary folder, canonical so resolution reports the paths compared.
struct Folder {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Folder {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        Self { _dir: dir, root }
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(&self.root, None, HashMap::new())
    }

    fn document(&self, relative: &str, frontmatter: &str) -> Markdown {
        let path = self.write(relative, &format!("---\n{frontmatter}---\nBody.\n"));
        Markdown::try_from(path.as_path()).expect("document").with_source(ComposeSource::File(path))
    }
}

fn valid(schemas: &DarkmatterSchemas, document: &Markdown) -> bool {
    schemas.validate(document).expect("the schema prepares").valid
}

fn baseline(yaml: &str) -> simplified_schema::SchemaRoot {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("baseline yaml");
    parse_yaml_schema(&value["$schema"]).expect("baseline schema")
}

const TRACK_MATCH: &str = "kind: schema-trigger\nmatch:\n  track: string(required)\n$schema:\n";

// ── Isolation ───────────────────────────────────────────────────────────────

/// The same two trigger payloads layered in either order are different
/// schemas: the later one owns `title`.
#[test]
fn reordered_conflicting_triggers_never_share_a_validator() {
    let numbers_last = Folder::new();
    numbers_last.write("schemas/a.yaml", &format!("{TRACK_MATCH}  title: boolean\n"));
    numbers_last.write("schemas/b.yaml", &format!("{TRACK_MATCH}  title: number\n"));
    let booleans_last = Folder::new();
    booleans_last.write("schemas/a.yaml", &format!("{TRACK_MATCH}  title: number\n"));
    booleans_last.write("schemas/b.yaml", &format!("{TRACK_MATCH}  title: boolean\n"));

    for (folder, expected) in [(&numbers_last, true), (&booleans_last, false), (&numbers_last, true)] {
        let schemas = DarkmatterSchemas::new(folder.context()).with_trigger_discovery().expect("triggers");
        assert!(schemas.discovery_findings().is_empty(), "{:?}", schemas.discovery_findings());
        let document = folder.document("doc.md", "track: x\ntitle: 5\n");
        assert_eq!(valid(&schemas, &document), expected, "{}", folder.root.display());
    }
}

/// A schema as the baseline and as the document's `$schema` play different
/// roles: the document's declaration of `title` wins either way.
#[test]
fn baseline_and_document_roles_are_part_of_the_identity() {
    let folder = Folder::new();
    let strings = "$schema:\n  title: string\n";
    let numbers = "$schema:\n  title: number\n";
    let document = |schema: &str| folder.document("doc.md", &format!("{schema}title: a\n"));

    let number_document =
        DarkmatterSchemas::new(folder.context()).with_baseline(baseline(strings)).expect("baseline");
    assert!(!valid(&number_document, &document(numbers)), "the number document schema wins");
    let string_document =
        DarkmatterSchemas::new(folder.context()).with_baseline(baseline(numbers)).expect("baseline");
    assert!(valid(&string_document, &document(strings)), "the string document schema wins");
}

/// The document layer is identified by its fully resolved schema, so a
/// changed import changes the identity though the importing file is
/// unchanged.
#[test]
fn a_changed_import_changes_the_identity() {
    let folder = Folder::new();
    folder.write("schema.yaml", "$schema:\n  value: choice@./types.yaml\n");
    folder.write("types.yaml", "$schema:\n  choice: 'enum(a, b)'\n");
    let document = || folder.document("doc.md", "$schema: ./schema.yaml\nvalue: a\n");
    let schemas = || DarkmatterSchemas::new(folder.context()).with_darkmatter_baseline_json_schema().expect("baseline");

    assert!(valid(&schemas(), &document()));
    folder.write("types.yaml", "$schema:\n  choice: 'enum(c, d)'\n");
    assert!(!valid(&schemas(), &document()), "the changed import is judged");
    folder.write("types.yaml", "$schema:\n  choice: 'enum(a, b)'\n");
    assert!(valid(&schemas(), &document()), "and restoring it is judged too");
}

/// A template's layer is the schema it resolved to: a changed data file
/// resolves the same `$schema` text to another schema.
#[test]
fn a_changed_template_result_changes_the_identity() {
    let folder = Folder::new();
    let home = tempfile::tempdir().expect("home");
    crate::dynamic_slots_fixtures::git(&folder.root, &["init", "-q", "-b", "main"]);
    let data = folder.write("data/os.yaml", "- macOS\n- Linux\n");
    let store = home.path().join(".config/darkmatter/schema-read-policy");
    std::fs::create_dir_all(store.parent().expect("store folder")).expect("store folder");
    std::fs::write(&store, format!("allow file {}\n", biscuit_file::to_portable_string(&data))).expect("grant");
    let path = folder.write(
        "doc.md",
        "---\n$schema:\n  os: enum({{ load_data(\"data/os.yaml\", \"string[]\") }})\nos: Linux\n---\nBody.\n",
    );
    let verdict = || {
        let snapshot =
            RequestSnapshot::new(&folder.root).with_home(Some(home.path().to_path_buf())).with_env(HashMap::new());
        let options = ComposeOptions::new().with_source_file(&path).with_data_runtime(DataRuntime::new());
        let request = ComposeRequest::prepare(options, &snapshot).expect("request");
        let schemas = DarkmatterSchemas::new(request.resolution_context().clone())
            .with_darkmatter_baseline_json_schema()
            .expect("baseline")
            .with_slot_session(Arc::new(SlotSession::for_request(&request)));
        let document = Markdown::try_from(path.as_path()).expect("document").with_source(ComposeSource::File(path.clone()));
        valid(&schemas, &document)
    };

    assert!(verdict(), "Linux is listed");
    folder.write("data/os.yaml", "- macOS\n");
    assert!(!verdict(), "the template now resolves to an enum without Linux");
}

/// Launch, completion, and unphased judgment of one effective schema are
/// three schemas: generated-required output is absent until completion.
#[test]
fn launch_and_completion_are_distinct_identities() {
    let folder = Folder::new();
    let document = folder.document("doc.md", "$schema:\n  artifact: string(generated; required)\n");
    let effective = DarkmatterSchemas::new(folder.context())
        .with_darkmatter_baseline_json_schema()
        .expect("baseline")
        .effective_for(&document)
        .expect("schema")
        .expect("effective");
    let empty = serde_json::json!({});
    for _ in 0..2 {
        assert!(effective.validate(&empty).valid);
        assert!(effective.validate_for_phase(&empty, SchemaPhase::Launch).expect("launch").valid);
        assert!(!effective.validate_for_phase(&empty, SchemaPhase::Completion).expect("completion").valid);
    }
}

/// A phase judges an eager file passively, so its validator is not the
/// unphased one that probes the file.
#[test]
fn the_passive_eager_file_projection_is_its_own_identity() {
    let folder = Folder::new();
    let document = folder.document("doc.md", "$schema:\n  source: file(eager)\nsource: missing.md\n");
    let effective = DarkmatterSchemas::new(folder.context())
        .with_darkmatter_baseline_json_schema()
        .expect("baseline")
        .effective_for(&document)
        .expect("schema")
        .expect("effective");
    let instance = serde_json::json!({"source": "missing.md"});
    assert!(!effective.validate(&instance).valid, "unphased validation probes the file");
    assert!(effective.validate_for_phase(&instance, SchemaPhase::Launch).expect("launch").valid);
    assert!(!effective.validate(&instance).valid, "and still does afterwards");
}

/// A file-judging effective schema stays keyed by what its files resolve
/// against: the document's folder and the repository.
#[test]
fn a_file_judging_identity_stays_keyed_by_folder_and_repository() {
    let folder = Folder::new();
    folder.write("with/spec.md", "# Spec\n");
    std::fs::create_dir_all(folder.root.join("without")).expect("folder");
    let schemas = DarkmatterSchemas::new(folder.context()).with_darkmatter_baseline_json_schema().expect("baseline");
    let frontmatter = "$schema:\n  spec: file(eager)\nspec: spec.md\n";
    assert!(valid(&schemas, &folder.document("with/doc.md", frontmatter)));
    assert!(!valid(&schemas, &folder.document("without/doc.md", frontmatter)), "another folder's validator");
    assert!(valid(&schemas, &folder.document("with/doc.md", frontmatter)));

    // A repository-relative value resolves only through the repository tier.
    let frontmatter = "$schema:\n  spec: file(eager)\nspec: with/spec.md\n";
    let without_repository = DarkmatterSchemas::new(folder.context());
    let with_repository = DarkmatterSchemas::new(folder.context().with_repository_root(&folder.root));
    let nested = || folder.document("docs/doc.md", frontmatter);
    assert!(!valid(&without_repository, &nested()));
    assert!(valid(&with_repository, &nested()), "the repository tier finds it");
    assert!(!valid(&without_repository, &nested()));
}

// ── Sharing and the work avoided ─────────────────────────────────────────────

#[cfg(feature = "work-counters")]
mod counted {
    use super::*;
    use darkmatter::work_counters::{Work, snapshot};

    /// Two documents with the same layers but different data share one
    /// validator: the second compiles nothing, and each verdict is its own.
    #[test]
    fn documents_with_identical_layers_share_a_validator() {
        let folder = Folder::new();
        let schemas = DarkmatterSchemas::new(folder.context()).with_darkmatter_baseline_json_schema().expect("baseline");
        let schema = "$schema:\n  count: number\n";
        assert!(valid(&schemas, &folder.document("a.md", &format!("{schema}count: 1\n"))));

        let before = snapshot();
        assert!(valid(&schemas, &folder.document("nested/b.md", &format!("{schema}count: 2\ntitle: Other\n"))));
        assert_eq!(snapshot().since(&before).get(Work::ValidatorCompile), 0, "the second document reuses the validator");
        assert!(!valid(&schemas, &folder.document("nested/c.md", &format!("{schema}count: [1]\n"))), "its own verdict");
    }

    /// The launch-area fallback is part of a file-judging validator's key,
    /// though it never changes a document-authored verdict: two fallbacks,
    /// two validators.
    #[test]
    fn the_fallback_folder_keeps_file_judging_validators_apart() {
        let folder = Folder::new();
        let document = folder.document("doc.md", "$schema:\n  spec: file(eager)\nspec: doc.md\n");
        let first = DarkmatterSchemas::new(folder.context()).with_file_ref_fallback_dir(folder.root.join("one"));
        let second = DarkmatterSchemas::new(folder.context()).with_file_ref_fallback_dir(folder.root.join("two"));
        assert!(valid(&first, &document));
        let before = snapshot();
        assert!(valid(&second, &document));
        assert_eq!(snapshot().since(&before).get(Work::ValidatorCompile), 1);
        let before = snapshot();
        assert!(valid(&first, &document));
        assert_eq!(snapshot().since(&before).get(Work::ValidatorCompile), 0, "each fallback keeps its own");
    }

    /// A fresh process judging a document with no `$schema` against the
    /// shipped baseline finds its validator by the generated digest: the
    /// baseline is never serialized.
    #[test]
    fn the_shipped_baseline_is_never_serialized_for_its_key() {
        let folder = Folder::new();
        let document = folder.document("doc.md", "title: Plain\n");
        let before = snapshot();
        let schemas = DarkmatterSchemas::new(folder.context()).with_darkmatter_baseline_json_schema().expect("baseline");
        assert!(valid(&schemas, &document));
        assert_eq!(snapshot().since(&before).get(Work::ValidatorCompile), 1, "the baseline's validator");
        // Reporting a problem compiles what describes it, but never
        // serializes the baseline either.
        assert!(!valid(&schemas, &folder.document("bad.md", "title: [1]\n")), "the baseline still judges");
        assert_eq!(snapshot().since(&before).get(Work::CanonicalSchemaSerialization), 0);
    }

    /// A document layer is serialized once per distinct schema; the merged
    /// schema, hundreds of kilobytes with the baseline, never is. A repeated
    /// document reuses the merge and its identity.
    #[test]
    fn only_the_document_layer_is_serialized_and_only_once() {
        let folder = Folder::new();
        let schemas = DarkmatterSchemas::new(folder.context()).with_darkmatter_baseline_json_schema().expect("baseline");
        let document = || folder.document("doc.md", "$schema:\n  count: number\ncount: 1\n");
        let before = snapshot();
        assert!(valid(&schemas, &document()));
        assert_eq!(snapshot().since(&before).get(Work::CanonicalSchemaSerialization), 1, "the document layer alone");
        let before = snapshot();
        assert!(valid(&schemas, &document()));
        let work = snapshot().since(&before);
        assert_eq!(work.get(Work::CanonicalSchemaSerialization), 0, "the merge memo keeps its identity");
        assert_eq!(work.get(Work::ValidatorCompile), 0);
    }

    /// A root-union document has no single merged layer to identify: its
    /// arms are wrapped as new schemas and found by their canonical hashes,
    /// and they still judge correctly.
    #[test]
    fn a_root_union_document_takes_the_canonical_route() {
        let folder = Folder::new();
        let schemas = DarkmatterSchemas::new(folder.context()).with_darkmatter_baseline_json_schema().expect("baseline");
        let union = "$schema:\n  - a: string(required)\n  - b: number(required)\n";
        let before = snapshot();
        assert!(valid(&schemas, &folder.document("a.md", &format!("{union}a: x\n"))));
        assert!(snapshot().since(&before).get(Work::CanonicalSchemaSerialization) >= 2, "each arm is hashed");
        assert!(!valid(&schemas, &folder.document("none.md", &format!("{union}c: 1\n"))));
        assert!(valid(&schemas, &folder.document("b.md", &format!("{union}b: 2\n"))));
    }

    /// After a cache reset the shipped data stays loaded (nothing decodes or
    /// prepares again) while validators and merges are cold, then warm.
    #[test]
    fn a_reset_empties_dynamic_caches_and_keeps_shipped_data() {
        let folder = Folder::new();
        let schemas = DarkmatterSchemas::new(folder.context()).with_darkmatter_baseline_json_schema().expect("baseline");
        let document = || folder.document("doc.md", "$schema:\n  count: number\ncount: 1\n");
        assert!(valid(&schemas, &document()));

        darkmatter::markdown::schemas::reset_process_caches_for_bench();
        let before = snapshot();
        assert!(valid(&schemas, &document()));
        let cold = snapshot().since(&before);
        assert_eq!(cold.get(Work::ValidatorCompile), 1, "the validator cache was emptied");
        assert_eq!(cold.get(Work::CanonicalSchemaSerialization), 1, "the merge memo was emptied");
        assert_eq!(cold.get(Work::ArtifactDecode), 0, "the shipped artifact stays loaded");
        assert_eq!(cold.get(Work::ShippedRuntimePreparation), 0);

        let before = snapshot();
        assert!(valid(&schemas, &document()));
        let warm = snapshot().since(&before);
        assert_eq!(warm.get(Work::ValidatorCompile), 0);
        assert_eq!(warm.get(Work::CanonicalSchemaSerialization), 0);
    }
}
