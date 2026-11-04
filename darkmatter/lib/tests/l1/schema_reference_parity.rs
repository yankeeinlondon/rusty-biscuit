//! Every consumer of a bare schema-file name selects the same file: a
//! `$schema` value, a named-type import, an `example(...)` constraint, and a
//! trigger payload all take `SCHEMAS_DIR`'s copy over the package root's,
//! and a link to a missing name targets the first searched root.
//!
//! `{dir}/repo` is a Git repository holding package `area/pkg`; `{dir}/defs`
//! is the `SCHEMAS_DIR` folder. Every context comes from a `RequestSnapshot`
//! carrying the fixture home and environment.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaRootKind, SchemaRootState, SchemaRoots};
use simplified_schema::SchemaError;
use simplified_schema::resolve::{SelectionOutcome, select_schema_file};

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self { dir: tempfile::tempdir().expect("temp dir") };
        let repo = fixture.repo();
        std::fs::create_dir_all(&repo).unwrap();
        let status = Command::new("git").args(["init", "-q"]).current_dir(&repo).status().expect("run git");
        assert!(status.success());
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"area/pkg\"]\n");
        write(
            &repo.join("area/pkg/Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&repo.join("area/pkg/src/lib.rs"), "");
        std::fs::create_dir_all(fixture.root().join("home")).unwrap();
        fixture
    }

    fn root(&self) -> PathBuf {
        canonicalize_simplified(self.dir.path()).unwrap()
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    fn package_schemas(&self) -> PathBuf {
        self.repo().join("area/pkg/schemas")
    }

    fn defs(&self) -> PathBuf {
        self.root().join("defs")
    }

    fn document(&self, name: &str) -> PathBuf {
        self.repo().join("area/pkg/docs").join(name)
    }

    fn context(&self, document: &Path, schemas_dir: Option<&Path>) -> FileResolutionContext {
        let env: HashMap<String, String> = schemas_dir
            .map(|dir| HashMap::from([("SCHEMAS_DIR".to_string(), dir.to_string_lossy().into_owned())]))
            .unwrap_or_default();
        let folder = document.parent().unwrap();
        std::fs::create_dir_all(folder).unwrap();
        let snapshot = RequestSnapshot::new(folder).with_home(Some(self.root().join("home"))).with_env(env);
        build_resolution_context(&snapshot).expect("fixture context")
    }

    /// The effective JSON Schema of `document` (as text) with trigger
    /// discovery and `SCHEMAS_DIR` set to the defs folder.
    fn effective(&self, document: &Path, text: &str) -> Result<String, SchemaError> {
        write(document, text);
        let markdown = Markdown::try_from(document).unwrap().with_source(ComposeSource::File(document.to_path_buf()));
        let defs = self.defs();
        let effective = DarkmatterSchemas::new(self.context(document, Some(&defs)))
            .with_trigger_discovery()?
            .effective_for(&markdown)?
            .expect("an effective schema");
        Ok(effective.json_schema.to_string())
    }
}

/// Writes `name` into both the `SCHEMAS_DIR` folder and the package root,
/// each copy built by `contents` from its own tag.
fn write_both(fixture: &Fixture, name: &str, contents: impl Fn(&str) -> String) {
    write(&fixture.defs().join(name), &contents("defs"));
    write(&fixture.package_schemas().join(name), &contents("package"));
}

#[test]
fn every_bare_name_consumer_selects_the_schemas_dir_copy() {
    let fixture = Fixture::new();
    write_both(&fixture, "shared.yaml", |tag| format!("$schema:\n  from_{tag}: string\n"));
    write_both(&fixture, "types.yaml", |tag| {
        format!("$schema:\n  Fields:\n    type: object\n    properties:\n      field_from_{tag}: string\n")
    });
    write_both(&fixture, "example.yaml", |tag| {
        format!("kind: example\ninvocation: x\nreturns: \"example_from_{tag}\"\ndescription: d\n")
    });
    write_both(&fixture, "payload.yaml", |tag| format!("$schema:\n  payload_from_{tag}: string\n"));
    // The trigger itself lives only in the package root, so only its payload
    // is selected across roots.
    write(
        &fixture.package_schemas().join("only-payload.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  $path: \"**/*.md\"\n$schema: payload.yaml\n",
    );

    let cases = [
        ("value.md", "---\n$schema: shared.yaml\n---\n", "from_defs", "from_package"),
        ("import.md", "---\n$schema:\n  fields: Fields@types.yaml\n---\n", "field_from_defs", "field_from_package"),
        (
            "example.md",
            "---\n$schema:\n  greeting: string(example(example.yaml))\n---\n",
            "example_from_defs",
            "example_from_package",
        ),
        ("trigger.md", "---\ntitle: x\n---\n", "payload_from_defs", "payload_from_package"),
    ];
    for (name, text, selected, shadowed) in cases {
        let schema = fixture.effective(&fixture.document(name), text).expect(name);
        assert!(schema.contains(selected), "{name}: {schema}");
        assert!(!schema.contains(shadowed), "{name}: {schema}");
    }

}

#[test]
fn a_malformed_schemas_dir_file_is_reported_not_skipped() {
    let fixture = Fixture::new();
    write(&fixture.defs().join("shared.yaml"), "$schema: [unclosed\n");
    write(&fixture.package_schemas().join("shared.yaml"), "$schema:\n  from_package: string\n");

    let error = fixture
        .effective(&fixture.document("doc.md"), "---\n$schema: shared.yaml\n---\n")
        .expect_err("the malformed SCHEMAS_DIR file is selected");
    let message = error.to_string();
    assert!(message.contains(&fixture.defs().join("shared.yaml").display().to_string()), "{message}");
    assert!(!message.contains("from_package"), "{message}");
}

#[test]
fn the_creation_target_follows_the_searched_roots() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.package_schemas()).unwrap();
    std::fs::create_dir_all(fixture.defs()).unwrap();
    let document = fixture.document("doc.md");
    let creation = |context: &FileResolutionContext| {
        let roots = SchemaRoots::for_document(context).expect("schema roots");
        let selection = select_schema_file("absent.yaml", document.parent().unwrap(), roots.search_paths(), context);
        match selection.outcome {
            SelectionOutcome::Missing { bare_name: true, creation } => creation,
            other => panic!("expected a bare-name miss, got {other:?}"),
        }
    };
    let in_package = Some(fixture.package_schemas().join("absent.yaml"));

    // Set: the link targets SCHEMAS_DIR.
    let defs = fixture.defs();
    assert_eq!(creation(&fixture.context(&document, Some(&defs))), Some(defs.join("absent.yaml")));

    // Unset, relative (invalid), or missing: no SCHEMAS_DIR root, so no link
    // into it; the package root is first.
    assert_eq!(creation(&fixture.context(&document, None)), in_package);
    assert_eq!(creation(&fixture.context(&document, Some(Path::new("defs")))), in_package);
    let missing = fixture.root().join("no-such-folder");
    assert_eq!(creation(&fixture.context(&document, Some(&missing))), in_package);
    assert!(!missing.exists(), "planning a link creates nothing");

    // Aliasing the package root: one root, searched as SCHEMAS_DIR.
    let package = fixture.package_schemas();
    let context = fixture.context(&document, Some(&package));
    let roots = SchemaRoots::for_document(&context).expect("schema roots");
    assert!(matches!(
        roots.entries().iter().find(|entry| entry.kind == SchemaRootKind::Package).map(|entry| &entry.state),
        Some(SchemaRootState::Duplicate { of: SchemaRootKind::SchemasDir, .. })
    ));
    assert_eq!(creation(&context), in_package);
}
