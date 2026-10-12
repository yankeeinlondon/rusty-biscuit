//! The five schema roots and `$path` triggers, read through public results:
//! the root list of a document's prepared context, the trigger registry a
//! scan installs, the effective schema a bare-name `$schema` resolves to, and
//! the verdicts `$path` and `match()` give for the same patterns.
//!
//! Every context is built by `build_resolution_context` from a
//! `RequestSnapshot` that carries the fixture `HOME` and environment; the
//! test process's environment is never read or mutated.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::file_match::FileMatchGlobs;
use darkmatter::markdown::schemas::triggers::{PathSubject, matches};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, InvalidSchemasDir, MatchExpr, PathGlobs, SchemaError, SchemaRootKind,
    SchemaRootState, SchemaRoots,
};
use darkmatter::markdown::Markdown;
use serde_json::json;

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

fn canonical(path: &Path) -> PathBuf {
    canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `{dir}/repo` is a Git repository holding package `area/pkg` in package
/// area `area`; `{dir}/home` is the fixture `HOME`; `{dir}/dm-defs` lies
/// outside both and is the `SCHEMAS_DIR` folder when a test sets one.
struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let fixture = Self { dir };
        let repo = fixture.repo();
        std::fs::create_dir_all(&repo).unwrap();
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .expect("run git");
        assert!(status.success());
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"area/pkg\"]\n");
        write(
            &repo.join("area/pkg/Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&repo.join("area/pkg/src/lib.rs"), "");
        std::fs::create_dir_all(fixture.home()).unwrap();
        fixture
    }

    fn root(&self) -> PathBuf {
        canonical(self.dir.path())
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    fn package(&self) -> PathBuf {
        self.repo().join("area/pkg")
    }

    fn area(&self) -> PathBuf {
        self.repo().join("area")
    }

    fn home(&self) -> PathBuf {
        self.root().join("home")
    }

    fn defs(&self) -> PathBuf {
        self.root().join("dm-defs")
    }

    /// The prepared context of a document at `document`, with the fixture
    /// home and `env` as the snapshot's environment.
    fn context(&self, document: &Path, env: &[(&str, &str)]) -> FileResolutionContext {
        self.context_with_home(document, Some(self.home()), env)
    }

    fn context_with_home(
        &self,
        document: &Path,
        home: Option<PathBuf>,
        env: &[(&str, &str)],
    ) -> FileResolutionContext {
        let env: HashMap<String, String> =
            env.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
        let folder = document.parent().expect("document folder");
        std::fs::create_dir_all(folder).expect("document folder");
        let snapshot = RequestSnapshot::new(folder)
            .with_home(home)
            .with_env(env);
        build_resolution_context(&snapshot).expect("fixture context")
    }
}

fn trigger(path_pattern: &str, payload: &str) -> String {
    format!("kind: trigger-schema\nmatch:\n  $path: \"{path_pattern}\"\n$schema: {payload}\n")
}

/// The properties of `document`'s effective schema with trigger discovery in
/// `context`.
fn effective_properties(context: &FileResolutionContext, document: &Path) -> Vec<String> {
    let markdown = Markdown::try_from(document).expect("document").with_source(ComposeSource::File(
        document.to_path_buf(),
    ));
    let schemas = DarkmatterSchemas::new(context.clone())
        .with_trigger_discovery()
        .expect("trigger discovery");
    let Some(effective) = schemas.effective_for(&markdown).expect("effective schema") else {
        return Vec::new();
    };
    let mut properties: Vec<String> = effective.json_schema["properties"]
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    properties.sort();
    properties
}

fn search_paths(context: &FileResolutionContext) -> Vec<PathBuf> {
    SchemaRoots::for_document(context).expect("schema roots")
        .search_paths()
        .iter()
        .map(|path| canonical(path))
        .collect()
}

fn state_of(roots: &SchemaRoots, kind: SchemaRootKind) -> SchemaRootState {
    roots
        .entries()
        .iter()
        .find(|root| root.kind == kind)
        .map(|root| root.state.clone())
        .expect("every kind is listed")
}

// ── Criterion 27: order ─────────────────────────────────────────────────

#[test]
fn schema_roots_are_package_area_tree_schemas_dir_then_home() {
    let fixture = Fixture::new();
    for folder in [
        fixture.package().join("schemas"),
        fixture.area().join("schemas"),
        fixture.repo().join("schemas"),
        fixture.defs(),
        fixture.home().join("schemas"),
    ] {
        std::fs::create_dir_all(folder).unwrap();
    }
    let document = fixture.package().join("docs/guide.md");
    let defs = fixture.defs();
    let context = fixture.context(&document, &[("SCHEMAS_DIR", defs.to_str().unwrap())]);

    let roots = SchemaRoots::for_document(&context).expect("schema roots");
    let kinds: Vec<SchemaRootKind> = roots.entries().iter().map(|root| root.kind).collect();
    assert_eq!(
        kinds,
        [
            SchemaRootKind::Package,
            SchemaRootKind::PackageArea,
            SchemaRootKind::Tree,
            SchemaRootKind::SchemasDir,
            SchemaRootKind::Home,
        ]
    );
    assert_eq!(
        search_paths(&context),
        [
            fixture.package().join("schemas"),
            fixture.area().join("schemas"),
            fixture.repo().join("schemas"),
            fixture.defs(),
            fixture.home().join("schemas"),
        ]
    );

    // The package and area are the document's, not the launch directory's: a
    // document at the repository root has neither.
    let top = fixture.context(&fixture.repo().join("top.md"), &[]);
    let roots = SchemaRoots::for_document(&top).expect("schema roots");
    assert_eq!(state_of(&roots, SchemaRootKind::Package), SchemaRootState::NotApplicable);
    assert_eq!(state_of(&roots, SchemaRootKind::PackageArea), SchemaRootState::NotApplicable);
    assert_eq!(search_paths(&top), [fixture.repo().join("schemas"), fixture.home().join("schemas")]);
}

// ── Wave 7 input matrix: SCHEMAS_DIR and home ───────────────────────────

#[test]
fn schemas_dir_and_home_input_matrix() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.defs()).unwrap();
    std::fs::create_dir_all(fixture.home().join("schemas")).unwrap();
    let document = fixture.repo().join("docs/doc.md");
    let defs = fixture.defs();
    let defs = defs.to_str().unwrap();
    let home_schemas = fixture.home().join("schemas");
    let missing = fixture.root().join("no-such-folder");

    let roots = |env: &[(&str, &str)]| {
        SchemaRoots::for_document(&fixture.context(&document, env)).expect("schema roots")
    };

    // Control: an absolute, existing folder is root 4, searched itself.
    let control = roots(&[("SCHEMAS_DIR", defs)]);
    assert_eq!(state_of(&control, SchemaRootKind::SchemasDir), SchemaRootState::Searched(fixture.defs()));

    // Absent from the snapshot: no fourth root. The context reads only the
    // snapshot, so the test process's own environment cannot add one.
    assert_eq!(state_of(&roots(&[]), SchemaRootKind::SchemasDir), SchemaRootState::NotApplicable);

    // Empty and whitespace-only: reported invalid, not treated as unset.
    for value in ["", "   "] {
        assert_eq!(
            state_of(&roots(&[("SCHEMAS_DIR", value)]), SchemaRootKind::SchemasDir),
            SchemaRootState::Invalid { value: value.to_string(), reason: InvalidSchemasDir::Empty },
            "SCHEMAS_DIR={value:?}"
        );
    }

    // Relative: reported invalid; it would depend on the launch directory.
    assert_eq!(
        state_of(&roots(&[("SCHEMAS_DIR", "dm-defs")]), SchemaRootKind::SchemasDir),
        SchemaRootState::Invalid { value: "dm-defs".into(), reason: InvalidSchemasDir::Relative }
    );

    // Absolute but missing: skipped as absent.
    assert_eq!(
        state_of(&roots(&[("SCHEMAS_DIR", missing.to_str().unwrap())]), SchemaRootKind::SchemasDir),
        SchemaRootState::Absent(missing.clone())
    );

    // The singular spelling is never read.
    assert_eq!(
        state_of(&roots(&[("SCHEMA_DIR", defs)]), SchemaRootKind::SchemasDir),
        SchemaRootState::NotApplicable
    );

    // Equal to `~/schemas`: one root, at the first position.
    let same = roots(&[("SCHEMAS_DIR", home_schemas.to_str().unwrap())]);
    assert_eq!(state_of(&same, SchemaRootKind::SchemasDir), SchemaRootState::Searched(home_schemas.clone()));
    assert_eq!(
        state_of(&same, SchemaRootKind::Home),
        SchemaRootState::Duplicate { path: home_schemas.clone(), of: SchemaRootKind::SchemasDir }
    );
    assert_eq!(
        same.search_paths().iter().filter(|path| canonical(path) == home_schemas).count(),
        1
    );

    // No home in the snapshot: root 5 does not apply, and nothing panics.
    let homeless = fixture.context_with_home(&document, None, &[]);
    assert_eq!(
        state_of(&SchemaRoots::for_document(&homeless).expect("schema roots"), SchemaRootKind::Home),
        SchemaRootState::NotApplicable
    );
}

// ── Criterion 28: shadowing ─────────────────────────────────────────────

#[test]
fn an_earlier_root_shadows_a_later_one_by_file_name() {
    let fixture = Fixture::new();
    let repo_schemas = fixture.repo().join("schemas");
    let home_schemas = fixture.home().join("schemas");
    write(&repo_schemas.join("claudine.yaml"), "$schema:\n  from_repo: string\n");
    write(&home_schemas.join("claudine.yaml"), "$schema:\n  from_home: string\n");
    write(&repo_schemas.join("notes.trigger.yaml"), &trigger("**/*.md", "repo-payload.yaml"));
    write(&repo_schemas.join("repo-payload.yaml"), "$schema:\n  repo_trigger: string\n");
    write(&home_schemas.join("notes.trigger.yaml"), &trigger("**/*.md", "home-payload.yaml"));
    write(&home_schemas.join("home-payload.yaml"), "$schema:\n  home_trigger: string\n");
    let document = fixture.repo().join("docs/doc.md");
    write(&document, "---\n$schema: claudine.yaml\n---\nBody\n");
    let context = fixture.context(&document, &[]);

    let properties = effective_properties(&context, &document);
    assert!(properties.contains(&"from_repo".to_string()), "{properties:?}");
    assert!(!properties.contains(&"from_home".to_string()), "{properties:?}");
    assert!(properties.contains(&"repo_trigger".to_string()), "{properties:?}");
    assert!(!properties.contains(&"home_trigger".to_string()), "shadowed trigger evaluated: {properties:?}");

    let registry = darkmatter::markdown::schemas::scan(&context).expect("scan");
    let shadowed: Vec<PathBuf> = registry.shadowed.iter().map(|file| canonical(&file.path)).collect();
    assert!(shadowed.contains(&home_schemas.join("notes.trigger.yaml")), "{shadowed:?}");

    // Removing the repository's files lets the home files win.
    std::fs::remove_file(repo_schemas.join("claudine.yaml")).unwrap();
    std::fs::remove_file(repo_schemas.join("notes.trigger.yaml")).unwrap();
    let properties = effective_properties(&context, &document);
    assert!(properties.contains(&"from_home".to_string()), "{properties:?}");
    assert!(properties.contains(&"home_trigger".to_string()), "{properties:?}");
}

// ── Criterion 29: SCHEMAS_DIR ───────────────────────────────────────────

#[test]
fn schemas_dir_names_the_schemas_folder_itself() {
    let fixture = Fixture::new();
    let defs = fixture.defs();
    write(&defs.join("docs.trigger.yaml"), &trigger("docs/*.md", "defs-payload.yaml"));
    write(&defs.join("defs-payload.yaml"), "$schema:\n  from_defs: string\n");
    write(&defs.join("team.yaml"), "$schema:\n  team: string\n");
    write(&defs.join("schemas/nested.trigger.yaml"), &trigger("**/*.md", "nested-payload.yaml"));
    write(&defs.join("schemas/nested-payload.yaml"), "$schema:\n  from_nested: string\n");
    write(&defs.join("schemas/nested.yaml"), "$schema:\n  nested: string\n");
    let set = [("SCHEMAS_DIR", defs.to_str().unwrap())];

    // A bare `docs/*.md` from SCHEMAS_DIR is judged from the document's
    // repository root.
    let at_root = fixture.repo().join("docs/x.md");
    write(&at_root, "---\ntitle: x\n---\n");
    let in_package = fixture.package().join("docs/x.md");
    write(&in_package, "---\ntitle: x\n---\n");
    assert_eq!(effective_properties(&fixture.context(&at_root, &set), &at_root), ["from_defs"]);
    assert!(effective_properties(&fixture.context(&in_package, &set), &in_package).is_empty());

    // Unset: the folder is not searched at all.
    assert!(effective_properties(&fixture.context(&at_root, &[]), &at_root).is_empty());

    // A bare-name schema directly in the folder resolves; one only in its
    // `schemas/` subfolder does not.
    let named = fixture.repo().join("docs/named.md");
    write(&named, "---\n$schema: team.yaml\n---\n");
    assert!(effective_properties(&fixture.context(&named, &set), &named).contains(&"team".to_string()));
    let nested = fixture.repo().join("docs/nested.md");
    write(&nested, "---\n$schema: nested.yaml\n---\n");
    let markdown = Markdown::try_from(nested.as_path())
        .unwrap()
        .with_source(ComposeSource::File(nested.clone()));
    let result = DarkmatterSchemas::new(fixture.context(&nested, &set))
        .with_trigger_discovery()
        .unwrap()
        .effective_for(&markdown);
    assert!(
        matches!(result, Err(SchemaError::NoMatch { bare_name: true, .. })),
        "a schema only in SCHEMAS_DIR/schemas/ must not resolve: {:?}",
        result.err()
    );
}

// ── Criterion 30: in-between folders ────────────────────────────────────

#[test]
fn in_between_schemas_folders_are_not_discovered() {
    let fixture = Fixture::new();
    let between = fixture.package().join("docs/schemas");
    write(&between.join("between.trigger.yaml"), &trigger("**/*.md", "between-payload.yaml"));
    write(&between.join("between-payload.yaml"), "$schema:\n  between: string\n");
    write(&between.join("local.yaml"), "$schema:\n  local: string\n");
    // A real root exists, so bare names are looked up in the roots only.
    std::fs::create_dir_all(fixture.repo().join("schemas")).unwrap();

    let plain = fixture.package().join("docs/plain.md");
    write(&plain, "---\ntitle: plain\n---\n");
    let context = fixture.context(&plain, &[]);
    assert!(effective_properties(&context, &plain).is_empty());
    assert!(!search_paths(&context).contains(&between));

    let named = fixture.package().join("docs/named.md");
    write(&named, "---\n$schema: local.yaml\n---\n");
    let markdown = Markdown::try_from(named.as_path())
        .unwrap()
        .with_source(ComposeSource::File(named.clone()));
    let result = DarkmatterSchemas::new(fixture.context(&named, &[]))
        .with_trigger_discovery()
        .unwrap()
        .effective_for(&markdown);
    assert!(
        matches!(result, Err(SchemaError::NoMatch { bare_name: true, .. })),
        "a bare name in an in-between folder must not resolve: {:?}",
        result.err()
    );
}

// ── Criterion 31: ~/schemas ─────────────────────────────────────────────

#[test]
fn home_schemas_apply_in_a_repository_with_no_schemas_folder() {
    let fixture = Fixture::new();
    let home_schemas = fixture.home().join("schemas");
    write(&home_schemas.join("notes.trigger.yaml"), &trigger("&**/*.md", "home-payload.yaml"));
    write(&home_schemas.join("home-payload.yaml"), "$schema:\n  from_home: string\n");
    write(&home_schemas.join("user.yaml"), "$schema:\n  user: string\n");

    let document = fixture.repo().join("docs/doc.md");
    write(&document, "---\n$schema: user.yaml\n---\n");
    let context = fixture.context(&document, &[]);
    assert_eq!(search_paths(&context), std::slice::from_ref(&home_schemas));
    assert_eq!(effective_properties(&context, &document), ["from_home", "user"]);
}

// ── Criterion 21: `$path` prefixes ──────────────────────────────────────

#[test]
fn path_triggers_read_reference_prefixes() {
    let fixture = Fixture::new();
    let repo_schemas = fixture.repo().join("schemas");
    write(&repo_schemas.join("pkg-docs.trigger.yaml"), &trigger("^docs/**", "pkg-docs.yaml"));
    write(&repo_schemas.join("pkg-docs.yaml"), "$schema:\n  pkg_docs: string\n");
    let package_schemas = fixture.package().join("schemas");
    write(&package_schemas.join("any-md.trigger.yaml"), &trigger("*.md", "any-md.yaml"));
    write(&package_schemas.join("any-md.yaml"), "$schema:\n  any_md: string\n");
    let home_schemas = fixture.home().join("schemas");
    write(&home_schemas.join("notes.trigger.yaml"), &trigger("~/notes/**", "notes.yaml"));
    write(&home_schemas.join("notes.yaml"), "$schema:\n  notes: string\n");

    // `^docs/**` from the repository root's schemas folder matches the
    // package's `docs/`; a package trigger's bare `*.md` matches at any depth
    // under the package.
    let package_doc = fixture.package().join("docs/deep/guide.md");
    write(&package_doc, "---\ntitle: x\n---\n");
    assert_eq!(
        effective_properties(&fixture.context(&package_doc, &[]), &package_doc),
        ["any_md", "pkg_docs"]
    );
    let package_top = fixture.package().join("README.md");
    write(&package_top, "---\ntitle: x\n---\n");
    assert_eq!(effective_properties(&fixture.context(&package_top, &[]), &package_top), ["any_md"]);

    // Outside the package neither applies.
    let repo_doc = fixture.repo().join("other/doc.md");
    write(&repo_doc, "---\ntitle: x\n---\n");
    assert!(effective_properties(&fixture.context(&repo_doc, &[]), &repo_doc).is_empty());

    // `~/notes/**` matches a document under the fixture home's `notes/`.
    let note = fixture.home().join("notes/today.md");
    write(&note, "---\ntitle: x\n---\n");
    assert_eq!(effective_properties(&fixture.context(&note, &[]), &note), ["notes"]);
}

#[test]
fn forbidden_path_prefixes_are_definition_errors_naming_the_pattern() {
    for pattern in [
        "@x/**",
        "%x.md",
        "vault:x/**",
        "{{X}}/**",
        "docs/{{X}}/*.md",
        "!@x/**",
        "!docs/{{X}}/*.md",
    ] {
        let error = PathGlobs::new(vec!["**/*.md".into(), pattern.into()])
            .expect_err(&format!("`{pattern}` must be refused"));
        let SchemaError::TriggerMatch { message } = &error else {
            panic!("`{pattern}`: expected a trigger match error, got {error:?}");
        };
        assert!(message.contains(&format!("`{pattern}`")), "`{pattern}`: {message}");
    }
    for pattern in ["~/notes/**", "&docs/**", "^docs/**", "./docs/*.md", "docs/*.md", "*.md"] {
        PathGlobs::new(vec![pattern.into()]).unwrap_or_else(|error| panic!("`{pattern}`: {error}"));
    }
}

// ── Wave 8: the `$path` field matrix ────────────────────────────────────

/// The trigger under test: one arm whose only condition is `$path`. Each row
/// replaces `$PATH_LINE` with one shape of the field.
const PATH_TRIGGER: &str =
    "kind: trigger-schema\nmatch:\n$PATH_LINE\n$schema: payload.yaml\n";

#[derive(Debug)]
enum Outcome {
    /// Loads, and applies to `docs/x.md`.
    Applies,
    /// A load error whose message contains every fragment.
    LoadError(&'static [&'static str]),
}

#[test]
fn path_field_input_matrix() {
    let fixture = Fixture::new();
    let schemas = fixture.repo().join("schemas");
    write(&schemas.join("payload.yaml"), "$schema:\n  from_trigger: string\n");
    let document = fixture.repo().join("docs/x.md");
    write(&document, "---\ntitle: x\n---\n");
    let context = fixture.context(&document, &[]);

    let rows: &[(&str, &str, Outcome)] = &[
        ("control", "  $path: \"^docs/**\"", Outcome::Applies),
        // No `$path` leaves an arm with no condition at all: the existing
        // vacuous-arm error, not "matches nothing".
        ("absent", "  {}", Outcome::LoadError(&["vacuous"])),
        ("explicit null", "  $path: null", Outcome::LoadError(&["$path must be", "got null"])),
        ("empty value", "  $path:", Outcome::LoadError(&["$path must be", "got null"])),
        ("wrong type, whole field", "  $path: 123", Outcome::LoadError(&["$path must be", "got number"])),
        (
            "wrong type, one element",
            "  $path: [\"^docs/**\", 123]",
            Outcome::LoadError(&["items must be strings", "got number"]),
        ),
        ("wrong type, every element", "  $path: [123]", Outcome::LoadError(&["items must be strings"])),
        ("empty", "  $path: []", Outcome::LoadError(&["at least one glob pattern"])),
        (
            "no positive pattern",
            "  $path: [\"!docs/**\"]",
            Outcome::LoadError(&["not a `!` exclusion"]),
        ),
        (
            "duplicate key",
            "  $path: \"^docs/**\"\n  $path: \"^other/**\"",
            Outcome::LoadError(&["duplicate"]),
        ),
        ("invalid glob", "  $path: \"^**/[x\"", Outcome::LoadError(&["`^**/[x`", "not a valid glob"])),
        ("forbidden @", "  $path: \"@x/**\"", Outcome::LoadError(&["`@x/**`", "`@`"])),
        ("forbidden %", "  $path: \"%x.md\"", Outcome::LoadError(&["`%x.md`"])),
        ("forbidden vault", "  $path: \"vault:x/**\"", Outcome::LoadError(&["`vault:x/**`", "vault"])),
        ("forbidden {{VAR}} prefix", "  $path: \"{{X}}/**\"", Outcome::LoadError(&["`{{X}}/**`", "{{VAR}}"])),
        (
            "forbidden {{VAR}} inside",
            "  $path: \"docs/{{X}}/*.md\"",
            Outcome::LoadError(&["`docs/{{X}}/*.md`", "{{VAR}}"]),
        ),
    ];

    // Source-text rows: the control trigger with one fragment appended. A
    // trigger file holds exactly one YAML document, so neither row may load
    // the valid leading document and ignore what follows it.
    //
    // | Shape                            | Outcome                            |
    // | -------------------------------- | ---------------------------------- |
    // | valid document plus a second one | load error: more than one document |
    // | valid document plus invalid YAML | load error at the appended line    |
    let control = PATH_TRIGGER.replace("$PATH_LINE", "  $path: \"^docs/**\"");
    let trailing: &[(&str, &str, Outcome)] = &[
        (
            "second document",
            "---\nkind: trigger-schema\nmatch:\n  $path: \"^other/**\"\n$schema: payload.yaml\n",
            Outcome::LoadError(&["more than one document"]),
        ),
        ("invalid trailing content", "extra: [unclosed\n", Outcome::LoadError(&["not valid YAML", "line 5"])),
    ];

    let cells = rows
        .iter()
        .map(|(shape, line, expected)| (*shape, PATH_TRIGGER.replace("$PATH_LINE", line), expected))
        .chain(trailing.iter().map(|(shape, suffix, expected)| (*shape, format!("{control}{suffix}"), expected)));
    for (shape, text, expected) in cells {
        write(&schemas.join("matrix.trigger.yaml"), &text);
        let scanned = darkmatter::markdown::schemas::scan(&context);
        match expected {
            Outcome::Applies => {
                scanned.unwrap_or_else(|error| panic!("{shape}: {error}"));
                assert_eq!(effective_properties(&context, &document), ["from_trigger"], "{shape}");
            }
            Outcome::LoadError(fragments) => {
                let error = scanned.expect_err(shape);
                let SchemaError::TriggerLoad { source, .. } = &error else {
                    panic!("{shape}: expected a trigger load error, got {error:?}");
                };
                let message = source.to_string();
                for fragment in *fragments {
                    assert!(message.contains(fragment), "{shape}: `{fragment}` not in {message:?}");
                }
            }
        }
    }

    // `~` is valid and matches a document under the fixture home's `notes/`.
    let note = fixture.home().join("notes/today.md");
    write(&note, "---\ntitle: x\n---\n");
    write(
        &fixture.home().join("schemas/matrix.trigger.yaml"),
        &PATH_TRIGGER.replace("$PATH_LINE", "  $path: \"~/notes/**\""),
    );
    write(&fixture.home().join("schemas/payload.yaml"), "$schema:\n  from_trigger: string\n");
    assert_eq!(effective_properties(&fixture.context(&note, &[]), &note), ["from_trigger"]);
}

// ── Criterion 21, last sentence: `$path` and `match()` agree ────────────

#[test]
fn path_triggers_and_match_validation_give_the_same_verdicts() {
    let fixture = Fixture::new();
    let package = fixture.package();
    let patterns: &[&[&str]] = &[
        &["*.md"],
        &["*.md", "!_*.md"],
        &["docs/*.md"],
        &["docs/**"],
        &["./docs/*.md"],
        &["^docs/**"],
        &["^*.md"],
        &["&area/**", "!&area/pkg/guide/**"],
        &["**/guide/*.md"],
        &["~/notes/**"],
        &["*.MD"],
    ];
    let documents = [
        package.join("README.md"),
        package.join("_draft.md"),
        package.join("docs/guide.md"),
        package.join("docs/_x.md"),
        package.join("docs/deep/more.md"),
        package.join("guide/intro.md"),
        package.join("guide/x.MD"),
        package.join("src/notes.txt"),
    ];

    // A context depends only on its document's folder, not on the pattern, so
    // build each once rather than once per pattern.
    let document_contexts: Vec<FileResolutionContext> =
        documents.iter().map(|document| fixture.context(document, &[])).collect();
    let value_context = fixture.context(&package.join("value.md"), &[]);

    for pattern_list in patterns {
        let authored: Vec<String> = pattern_list.iter().map(|p| p.to_string()).collect();
        let expr = MatchExpr::Path(PathGlobs::new(authored.clone()).expect("valid $path"));
        let globs = FileMatchGlobs::new(&authored).expect("valid match()");
        for (document, document_context) in documents.iter().zip(&document_contexts) {
            // The trigger sits in `{package}/schemas`, so its bare patterns are
            // read from the package; `match()` judges a value whose folder is
            // the package.
            let trigger = matches(
                &expr,
                &json!({}),
                &PathSubject::new(document, document_context).with_pattern_cwd(&package),
            );
            let validation = globs.matches(document, &value_context);
            assert_eq!(trigger, validation, "{pattern_list:?} on {}", document.display());
        }
    }
}

/// Criterion 26 for `$path`: an absolute pattern's authored directory names
/// are case-sensitive, and the trigger applies exactly when `match()` admits.
/// `$path` takes no `{{VAR}}`, so only `match()` has the interpolated row. On
/// a case-insensitive filesystem `DOCS` reaches `docs`; on a case-sensitive
/// one it does not exist.
#[test]
fn absolute_path_triggers_reject_a_directory_spelled_in_another_case() {
    assert_path_triggers_judge_directory_spelling("docs", "DOCS", None);
}

/// Criterion 26 for `$path` with Unicode aliases that lowercasing does not
/// reveal (`ς` for a stored `Σ`, `ß` for a stored `SS`). Where the alias
/// does not open, the checks still run.
#[test]
fn absolute_path_triggers_reject_a_unicode_case_alias() {
    for (stored, authored) in [("Σ", "ς"), ("SS", "ß")] {
        assert_path_triggers_judge_directory_spelling(stored, authored, None);
    }
}

/// Criterion 26 for `$path` below a traversal-only ancestor (mode `0111`):
/// being unable to list it must not approve the mismatched `DOCS`.
#[cfg(unix)]
#[test]
fn absolute_path_triggers_below_a_traversal_only_ancestor_stay_case_sensitive() {
    assert_path_triggers_judge_directory_spelling("locked/anchor/docs", "locked/anchor/DOCS", Some("locked"));
}

/// The `$path` trigger and `match()` reject absolute (and `{{PKG}}`)
/// patterns spelling the package's `stored` directory as `authored`, and
/// apply to the stored spelling, directly and through a scan. With
/// `traversal_only`, that package directory is set to mode `0111` first.
fn assert_path_triggers_judge_directory_spelling(stored: &str, authored: &str, traversal_only: Option<&str>) {
    let fixture = Fixture::new();
    let package = fixture.package();
    let document = package.join(stored).join("guide.md");
    write(&document, "---\ntitle: x\n---\n");
    crate::fs_capability::probe_directory_alias(&package, stored, authored);
    let package_text = biscuit_file::to_portable_string(&package);
    let env = [("PKG", package_text.as_str())];
    let document_context = fixture.context(&document, &env);
    let value_context = fixture.context(&package.join("value.md"), &env);
    let schemas = fixture.repo().join("schemas");
    write(&schemas.join("upper.trigger.yaml"), &trigger(&format!("{package_text}/{authored}/*.md"), "upper.yaml"));
    write(&schemas.join("upper.yaml"), "$schema:\n  upper: string\n");
    write(&schemas.join("lower.trigger.yaml"), &trigger(&format!("{package_text}/{stored}/*.md"), "lower.yaml"));
    write(&schemas.join("lower.yaml"), "$schema:\n  lower: string\n");
    #[cfg(unix)]
    let _guard = match traversal_only {
        Some(dir) => match Locked::traversal_only(&package.join(dir)) {
            Some(guard) => Some(guard),
            None => return,
        },
        None => None,
    };
    #[cfg(not(unix))]
    let _ = traversal_only;
    let admits = |pattern: &str| {
        FileMatchGlobs::new(&[pattern.to_string()]).expect("valid match()").matches(&document, &value_context)
    };

    for (pattern, expected) in [
        (format!("{package_text}/{authored}/*.md"), false),
        (format!("{package_text}/{stored}/*.md"), true),
    ] {
        let expr = MatchExpr::Path(PathGlobs::new(vec![pattern.clone()]).expect("valid $path"));
        let trigger = matches(
            &expr,
            &json!({}),
            &PathSubject::new(&document, &document_context).with_pattern_cwd(&package),
        );
        assert_eq!((trigger, admits(&pattern)), (expected, expected), "`{pattern}`");
    }
    assert!(!admits(&format!("{{{{PKG}}}}/{authored}/*.md")));
    assert!(admits(&format!("{{{{PKG}}}}/{stored}/*.md")));

    // Through a scan: the mismatched trigger does not apply.
    assert_eq!(effective_properties(&document_context, &document), ["lower"]);
}

// ── Inaccessible schema roots ───────────────────────────────────────────

/// A fixture whose `kind` root is `{locked}/anchor/schemas`, holding
/// `policy.yaml` (`hidden_rule: string(required)`) and a trigger applying it
/// to every document. A readable fixture home holds a same-named
/// `policy.yaml` (`fallback_rule: string`) unless `kind` is the home root.
/// The document names `$schema: policy.yaml`.
#[cfg(unix)]
struct InaccessibleRoot {
    dir: tempfile::TempDir,
    kind: SchemaRootKind,
}

#[cfg(unix)]
impl InaccessibleRoot {
    fn new(kind: SchemaRootKind) -> Self {
        let fixture = Self { dir: tempfile::tempdir().expect("temp dir"), kind };
        let repo = fixture.repo();
        std::fs::create_dir_all(&repo).unwrap();
        let status = Command::new("git").args(["init", "-q"]).current_dir(&repo).status().expect("run git");
        assert!(status.success());
        let member = match kind {
            SchemaRootKind::Package => Some("locked/anchor"),
            SchemaRootKind::PackageArea => Some("locked/anchor/pkg"),
            _ => None,
        };
        if let Some(member) = member {
            write(&repo.join("Cargo.toml"), &format!("[workspace]\nmembers = [\"{member}\"]\n"));
            write(
                &repo.join(member).join("Cargo.toml"),
                "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            );
            write(&repo.join(member).join("src/lib.rs"), "");
        }
        let schemas = fixture.schemas();
        write(&schemas.join("policy.yaml"), "$schema:\n  hidden_rule: string(required)\n");
        write(&schemas.join("hidden.trigger.yaml"), &trigger("**/*.md", "policy.yaml"));
        if kind != SchemaRootKind::Home {
            write(&fixture.root().join("home/schemas/policy.yaml"), "$schema:\n  fallback_rule: string\n");
        }
        write(&fixture.document(), "---\n$schema: policy.yaml\n---\nBody\n");
        fixture
    }

    fn root(&self) -> PathBuf {
        canonical(self.dir.path())
    }

    fn locked(&self) -> PathBuf {
        self.root().join(if matches!(self.kind, SchemaRootKind::Package | SchemaRootKind::PackageArea) {
            "repo/locked"
        } else {
            "locked"
        })
    }

    fn anchor(&self) -> PathBuf {
        self.locked().join("anchor")
    }

    fn schemas(&self) -> PathBuf {
        self.anchor().join("schemas")
    }

    fn repo(&self) -> PathBuf {
        if self.kind == SchemaRootKind::Tree { self.anchor() } else { self.root().join("repo") }
    }

    fn document(&self) -> PathBuf {
        match self.kind {
            SchemaRootKind::Package | SchemaRootKind::PackageArea => self.anchor().join("docs/doc.md"),
            _ => self.repo().join("docs/doc.md"),
        }
    }

    /// The document's context, built while every folder is readable.
    fn context(&self) -> FileResolutionContext {
        let home = if self.kind == SchemaRootKind::Home { self.anchor() } else { self.root().join("home") };
        let schemas = self.schemas();
        let mut env = HashMap::new();
        if self.kind == SchemaRootKind::SchemasDir {
            env.insert("SCHEMAS_DIR".to_string(), schemas.to_string_lossy().into_owned());
        }
        let snapshot = RequestSnapshot::new(self.document().parent().unwrap()).with_home(Some(home)).with_env(env);
        build_resolution_context(&snapshot).expect("fixture context")
    }
}

/// The document's effective properties after discovery, or the error.
#[cfg(unix)]
fn bare_name_result(context: &FileResolutionContext, markdown: &Markdown) -> Result<Vec<String>, SchemaError> {
    let effective = DarkmatterSchemas::new(context.clone()).with_trigger_discovery()?.effective_for(markdown)?;
    let mut properties: Vec<String> = effective
        .and_then(|effective| effective.json_schema["properties"].as_object().map(|map| map.keys().cloned().collect()))
        .unwrap_or_default();
    properties.sort();
    Ok(properties)
}

#[cfg(unix)]
fn is_io_error_naming(result: &Result<impl std::fmt::Debug, SchemaError>, expected: &Path) -> bool {
    matches!(result, Err(SchemaError::Io { path, .. }) if path == expected)
}

#[cfg(unix)]
#[test]
fn an_inaccessible_package_schema_root_is_an_error() {
    assert_inaccessible_root_is_an_error(SchemaRootKind::Package);
}

#[cfg(unix)]
#[test]
fn an_inaccessible_package_area_schema_root_is_an_error() {
    assert_inaccessible_root_is_an_error(SchemaRootKind::PackageArea);
}

#[cfg(unix)]
#[test]
fn an_inaccessible_tree_schema_root_is_an_error() {
    assert_inaccessible_root_is_an_error(SchemaRootKind::Tree);
}

#[cfg(unix)]
#[test]
fn an_inaccessible_schemas_dir_root_is_an_error() {
    assert_inaccessible_root_is_an_error(SchemaRootKind::SchemasDir);
}

#[cfg(unix)]
#[test]
fn an_inaccessible_home_schema_root_is_an_error() {
    assert_inaccessible_root_is_an_error(SchemaRootKind::Home);
}

/// A `kind` root behind an ancestor this user cannot search is an I/O error
/// naming it from the root list, the scan, and bare-name resolution, and
/// never lets the later same-named `policy.yaml` win. A directly unreadable
/// root fails at enumeration; a missing one is skipped and the later root
/// wins.
#[cfg(unix)]
fn assert_inaccessible_root_is_an_error(kind: SchemaRootKind) {
    let fixture = InaccessibleRoot::new(kind);
    let context = fixture.context();
    let document = fixture.document();
    let markdown =
        Markdown::try_from(document.as_path()).unwrap().with_source(ComposeSource::File(document.clone()));
    let schemas = fixture.schemas();

    // Readable control: this kind searches the folder, its trigger loads,
    // and the bare name resolves to it.
    let roots = SchemaRoots::for_document(&context).expect("readable roots");
    assert_eq!(state_of(&roots, kind), SchemaRootState::Searched(schemas.clone()), "{kind:?}");
    let registry = darkmatter::markdown::schemas::scan(&context).expect("readable scan");
    assert_eq!(registry.triggers.len(), 1, "{kind:?}");
    assert_eq!(bare_name_result(&context, &markdown).expect("readable"), ["hidden_rule"], "{kind:?}");

    // An ancestor without search permission.
    {
        let Some(_locked) = Locked::with_mode(&fixture.locked(), 0o000) else { return };
        let probe = std::fs::symlink_metadata(&schemas);
        assert!(
            matches!(&probe, Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied),
            "{kind:?}: {probe:?}"
        );
        let roots = SchemaRoots::for_document(&context);
        assert!(is_io_error_naming(&roots, &schemas), "{kind:?}: {roots:?}");
        let scanned = darkmatter::markdown::schemas::scan(&context);
        assert!(is_io_error_naming(&scanned, &schemas), "{kind:?}: {scanned:?}");
        let resolved = bare_name_result(&context, &markdown);
        assert!(is_io_error_naming(&resolved, &schemas), "{kind:?}: {resolved:?}");
    }

    // The folder itself unreadable, its ancestors searchable: listed as
    // searched, and enumeration fails.
    {
        let Some(_locked) = Locked::with_mode(&schemas, 0o000) else { return };
        let roots = SchemaRoots::for_document(&context).expect("inspectable roots");
        assert_eq!(state_of(&roots, kind), SchemaRootState::Searched(schemas.clone()), "{kind:?}");
        let scanned = darkmatter::markdown::schemas::scan(&context);
        assert!(is_io_error_naming(&scanned, &schemas), "{kind:?}: {scanned:?}");
        let resolved = bare_name_result(&context, &markdown);
        assert!(is_io_error_naming(&resolved, &schemas), "{kind:?}: {resolved:?}");
    }

    // Missing: skipped, so the later root's same-named schema applies
    // (home has no later root, so nothing matches).
    std::fs::remove_dir_all(&schemas).unwrap();
    let roots = SchemaRoots::for_document(&context).expect("roots without the folder");
    assert_eq!(state_of(&roots, kind), SchemaRootState::Absent(schemas.clone()), "{kind:?}: {roots:?}");
    let resolved = bare_name_result(&context, &markdown);
    if kind == SchemaRootKind::Home {
        assert!(matches!(resolved, Err(SchemaError::NoMatch { .. })), "{resolved:?}");
    } else {
        assert_eq!(resolved.expect("fallback"), ["fallback_rule"], "{kind:?}");
    }
}

/// Sets a directory to mode `0o111` (traversal only) and restores `0o755` on
/// drop. `None` when this user can still list it (a privileged user).
#[cfg(unix)]
struct Locked(PathBuf);

#[cfg(unix)]
impl Locked {
    fn traversal_only(dir: &Path) -> Option<Self> {
        Self::with_mode(dir, 0o111)
    }

    /// `dir` at `mode`, which must deny listing it.
    fn with_mode(dir: &Path, mode: u32) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).expect("chmod");
        let locked = Self(dir.to_path_buf());
        match std::fs::read_dir(dir) {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Some(locked),
            other => {
                eprintln!("skipping: {} is still listable after chmod {mode:o} ({other:?})", dir.display());
                None
            }
        }
    }
}

#[cfg(unix)]
impl Drop for Locked {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

// ── Criterion 34: the documented variable name ──────────────────────────

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/darkmatter/lib`.
    biscuit_test_harness::manifest_dir!()
        .ancestors()
        .nth(2)
        .expect("repository root is two levels above darkmatter/lib")
        .to_path_buf()
}

fn documentation_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry");
        let file_type = entry.file_type().expect("file type");
        let path = entry.path();
        // A symlink (the Claudine skill's `topics` link to `claudine/docs`)
        // points at a tree this walk already covers.
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            documentation_files(&path, files);
        } else if matches!(path.extension().and_then(|e| e.to_str()), Some("md" | "yaml" | "yml")) {
            files.push(path);
        }
    }
}

/// `SCHEMA_DIR` as a whole word, so `SCHEMAS_DIR` and `MY_SCHEMA_DIR_X` do
/// not count.
fn names_singular_variable(text: &str) -> bool {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices("SCHEMA_DIR").any(|(at, needle)| {
        let before = text[..at].chars().next_back();
        let after = text[at + needle.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

#[test]
fn no_doc_or_skill_names_the_singular_schema_variable() {
    assert!(names_singular_variable("set `SCHEMA_DIR` to"));
    assert!(!names_singular_variable("set `SCHEMAS_DIR` to"));
    assert!(!names_singular_variable("MY_SCHEMA_DIR_X"));

    let root = repo_root();
    let dirs = [
        root.join("darkmatter/docs"),
        root.join("claudine/docs"),
        root.join("biscuit-file/docs"),
        root.join(".claude/skills/darkmatter"),
        root.join(".claude/skills/claudine"),
        root.join(".claude/skills/biscuit-file"),
    ];
    let mut files = Vec::new();
    for dir in &dirs {
        documentation_files(dir, &mut files);
    }
    assert!(
        files.iter().any(|f| f.ends_with("topics/schemas/definition.md")),
        "the walk must reach the schema definition page"
    );
    let offenders: Vec<_> = files
        .iter()
        .filter(|file| std::fs::read_to_string(file).is_ok_and(|text| names_singular_variable(&text)))
        .map(|file| file.strip_prefix(&root).unwrap_or(file).display().to_string())
        .collect();
    assert!(offenders.is_empty(), "rename `SCHEMA_DIR` to `SCHEMAS_DIR` in: {offenders:?}");
}
