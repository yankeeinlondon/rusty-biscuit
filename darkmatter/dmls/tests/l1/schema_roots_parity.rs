//! `md` and DMLS agree on schema roots.
//!
//! A fixture monorepo holds package `area/pkg` in package area `area`, a
//! fixture `HOME`, and a `SCHEMAS_DIR` folder (`dm-defs`) outside both. For
//! each document, the context `md` prepares (launched from the document's
//! folder) and the context DMLS hands out (one per repository, derived for
//! the document) must give the same five roots, the same applied triggers,
//! and the same bare-name `$schema` file. A full session then shows the same
//! through published diagnostics, and a `$path` definition error carries the
//! message `md` reports.
//!
//! Every snapshot is built here; the test process's environment is never
//! read or mutated.

use crate::common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::triggers::scan;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, SchemaRootKind, SchemaRootState, SchemaRoots, parse_trigger_envelope_from_str,
};
use dmls::context::RepositoryContexts;
use dmls::overlay::SchemaOutcome;
use dmls::{DmlsConfig, OverlayState};
use serde_json::{Value, json};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn trigger(path_pattern: &str, payload: &str) -> String {
    format!("kind: trigger-schema\nmatch:\n  $path: \"{path_pattern}\"\n$schema: {payload}\n")
}

/// Writes a trigger named `{name}.trigger.yaml` with payload `{name}.yaml`
/// declaring the optional property `from_{name}`, into `folder`.
fn add_trigger(folder: &Path, name: &str, path_pattern: &str) {
    write(&folder.join(format!("{name}.trigger.yaml")), &trigger(path_pattern, &format!("{name}.yaml")));
    write(&folder.join(format!("{name}.yaml")), &format!("$schema:\n  from_{name}: string\n"));
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    /// Lays out the monorepo inside `workspace`:
    ///
    /// | folder | contents |
    /// |---|---|
    /// | `{pkg}/schemas` | trigger `pkg` (`**/*.md`) |
    /// | `{area}/schemas` | trigger `area` (`**/*.md`) |
    /// | `{repo}/schemas` | trigger `repo` (`docs/*.md`), `claudine.yaml` |
    /// | `dm-defs` | trigger `defs` (`docs/*.md`, judged from the repository root) |
    /// | `dm-defs/schemas` | trigger `nested`, never searched |
    /// | `{home}/schemas` | trigger `home`, a shadowed `repo.trigger.yaml`, `claudine.yaml`, `user.yaml` |
    /// | `{pkg}/docs/schemas` | trigger `between` and `between.yaml`, never searched |
    fn new(workspace: &Path) -> Self {
        let fixture = Self { root: canonicalize_simplified(workspace).unwrap() };
        let repo = fixture.repo();
        init_repository(&repo);
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"area/pkg\"]\n");
        write(
            &fixture.package().join("Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&fixture.package().join("src/lib.rs"), "");

        add_trigger(&fixture.package().join("schemas"), "pkg", "**/*.md");
        add_trigger(&repo.join("area/schemas"), "area", "**/*.md");
        add_trigger(&repo.join("schemas"), "repo", "docs/*.md");
        write(&repo.join("schemas/claudine.yaml"), "$schema:\n  from_repo_claudine: string\n");
        add_trigger(&fixture.defs(), "defs", "docs/*.md");
        add_trigger(&fixture.defs().join("schemas"), "nested", "**/*.md");
        let home_schemas = fixture.home().join("schemas");
        add_trigger(&home_schemas, "home", "**/*.md");
        write(&home_schemas.join("repo.trigger.yaml"), &trigger("**/*.md", "home-repo.yaml"));
        write(&home_schemas.join("home-repo.yaml"), "$schema:\n  from_home_repo: string\n");
        write(&home_schemas.join("claudine.yaml"), "$schema:\n  from_home_claudine: string\n");
        write(&home_schemas.join("user.yaml"), "$schema:\n  from_user: string\n");
        add_trigger(&fixture.package().join("docs/schemas"), "between", "**/*.md");
        fixture
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn package(&self) -> PathBuf {
        self.repo().join("area/pkg")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn defs(&self) -> PathBuf {
        self.root.join("dm-defs")
    }

    /// The snapshot both `md` and the server take: fixture home, and
    /// `SCHEMAS_DIR` naming `dm-defs` itself.
    fn snapshot(&self, request_dir: &Path) -> RequestSnapshot {
        RequestSnapshot::new(request_dir)
            .with_home(Some(self.home()))
            .with_env(HashMap::from([(
                "SCHEMAS_DIR".to_string(),
                self.defs().to_string_lossy().into_owned(),
            )]))
    }
}

/// What one side decided for a document.
#[derive(Debug, PartialEq)]
struct Verdict {
    roots: SchemaRoots,
    /// The `from_*` properties the effective schema declares, sorted, or the
    /// assembly error.
    properties: Result<Vec<String>, String>,
    /// The resolved `$schema` files.
    dependencies: Vec<PathBuf>,
}

fn from_properties(json_schema: &Value) -> Vec<String> {
    let mut names: Vec<String> = json_schema["properties"]
        .as_object()
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default();
    names.retain(|name| name.starts_with("from_"));
    names.sort();
    names
}

/// `md`'s verdict: a request launched from the document's folder, with
/// trigger discovery, as `md schema validate` assembles it.
fn md_verdict(fixture: &Fixture, document: &Path, text: &str) -> Verdict {
    let context: FileResolutionContext =
        build_resolution_context(&fixture.snapshot(document.parent().unwrap())).expect("md context");
    let context = context.for_source(document);
    let markdown = Markdown::from(text).with_source(ComposeSource::File(document.to_path_buf()));
    let effective = DarkmatterSchemas::new(context.clone())
        .with_trigger_discovery()
        .and_then(|schemas| schemas.effective_for(&markdown));
    let (properties, dependencies) = match effective {
        Ok(Some(effective)) => {
            (Ok(from_properties(&effective.json_schema)), effective.dependencies().to_vec())
        }
        Ok(None) => (Ok(Vec::new()), Vec::new()),
        Err(error) => (Err(error.to_string()), Vec::new()),
    };
    Verdict { roots: SchemaRoots::for_document(&context), properties, dependencies }
}

/// DMLS's verdict: the server's per-repository context for the document and
/// the overlay's effective schema.
fn dmls_verdict(fixture: &Fixture, document: &Path, text: &str) -> Verdict {
    let contexts = RepositoryContexts::new(fixture.snapshot(&fixture.root));
    let resolution = contexts.for_document(document);
    let context = resolution.context().expect("dmls context").clone();
    let uri: lsp_types::Uri =
        url::Url::from_file_path(document).unwrap().as_str().parse().unwrap();
    let overlay = OverlayState::default()
        .for_document(&uri, text, document, &DmlsConfig::default(), &[fixture.repo()], &resolution)
        .expect("overlay");
    let (properties, dependencies) = match &overlay.schema {
        SchemaOutcome::Ready(Some(bundle)) => (
            Ok(from_properties(&bundle.effective.json_schema)),
            bundle.effective.dependencies().to_vec(),
        ),
        SchemaOutcome::Ready(None) => (Ok(Vec::new()), Vec::new()),
        SchemaOutcome::Failed(error) => (Err(error.to_string()), Vec::new()),
    };
    Verdict { roots: SchemaRoots::for_document(&context), properties, dependencies }
}

fn names(names: &[&str]) -> Result<Vec<String>, String> {
    Ok(names.iter().map(|name| name.to_string()).collect())
}

#[test]
fn md_and_dmls_give_the_same_roots_triggers_and_bare_names() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let repo = fixture.repo();
    let package = fixture.package();

    // A package document: five searched roots in order; the repository's
    // `claudine.yaml` shadows the home one, and home's `repo.trigger.yaml` is
    // shadowed by the repository's; `dm-defs`' and the repository's
    // `docs/*.md` are judged from the repository root and miss it.
    let package_doc = package.join("docs/x.md");
    let package_text = "---\n$schema: claudine.yaml\n---\n";
    // A repository document: `dm-defs`' `docs/*.md` applies; `user.yaml`
    // resolves in home.
    let repo_doc = repo.join("docs/x.md");
    let repo_text = "---\n$schema: user.yaml\n---\n";
    // A schema only in an in-between `schemas/` folder does not resolve.
    let between_doc = package.join("docs/y.md");
    let between_text = "---\n$schema: between.yaml\n---\n";

    // The resolved bare name is a dependency; the same name in a later root
    // is not.
    struct Case<'a> {
        document: &'a Path,
        text: &'a str,
        properties: Result<Vec<String>, String>,
        resolved: PathBuf,
        not_resolved: PathBuf,
    }
    let cases = [
        Case {
            document: &package_doc,
            text: package_text,
            properties: names(&["from_area", "from_home", "from_pkg", "from_repo_claudine"]),
            resolved: repo.join("schemas/claudine.yaml"),
            not_resolved: fixture.home().join("schemas/claudine.yaml"),
        },
        Case {
            document: &repo_doc,
            text: repo_text,
            properties: names(&["from_defs", "from_home", "from_repo", "from_user"]),
            resolved: fixture.home().join("schemas/user.yaml"),
            not_resolved: fixture.package().join("docs/schemas/between.yaml"),
        },
    ];
    for case in cases {
        let document = case.document;
        write(document, case.text);
        let md = md_verdict(&fixture, document, case.text);
        let dmls = dmls_verdict(&fixture, document, case.text);
        assert_eq!(md, dmls, "{}", document.display());
        assert_eq!(md.properties, case.properties, "{}", document.display());
        let canonical: Vec<PathBuf> =
            md.dependencies.iter().map(|path| canonicalize_simplified(path).unwrap()).collect();
        assert!(canonical.contains(&case.resolved), "{canonical:?}");
        assert!(!canonical.contains(&case.not_resolved), "{canonical:?}");
    }

    let package_roots = md_verdict(&fixture, &package_doc, package_text).roots;
    let searched: Vec<(SchemaRootKind, PathBuf)> = package_roots
        .searched()
        .iter()
        .map(|root| (root.kind, canonicalize_simplified(&root.path).unwrap()))
        .collect();
    assert_eq!(
        searched,
        [
            (SchemaRootKind::Package, package.join("schemas")),
            (SchemaRootKind::PackageArea, repo.join("area/schemas")),
            (SchemaRootKind::Tree, repo.join("schemas")),
            (SchemaRootKind::SchemasDir, fixture.defs()),
            (SchemaRootKind::Home, fixture.home().join("schemas")),
        ]
    );
    assert!(
        package_roots.entries().iter().all(|root| matches!(root.state, SchemaRootState::Searched(_))),
        "{package_roots:?}"
    );

    write(&between_doc, between_text);
    let md = md_verdict(&fixture, &between_doc, between_text);
    let dmls = dmls_verdict(&fixture, &between_doc, between_text);
    assert_eq!(md, dmls);
    let error = md.properties.expect_err("an in-between schema does not resolve");
    assert!(error.contains("between.yaml"), "{error}");
}

fn initialize_params(folder: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Neovim", "version": "0.11.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "textDocument": {}
        },
        "workspaceFolders": [
            { "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "repo" }
        ]
    })
}

fn open(fixture: &LspFixture<'_>, uri: &str, language: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": uri, "languageId": language, "version": 1, "text": text }
        }),
    );
}

/// Which of `candidates` a `dm.schema.missing_required` diagnostic names.
fn missing_required(diagnostics: &[Value], candidates: &[&str]) -> Vec<String> {
    candidates
        .iter()
        .filter(|name| {
            diagnostics.iter().any(|diagnostic| {
                diagnostic["code"] == json!("dm.schema.missing_required")
                    && diagnostic["message"].as_str().is_some_and(|message| message.contains(*name))
            })
        })
        .map(|name| name.to_string())
        .collect()
}

/// The server reads `SCHEMAS_DIR` and `~/schemas` from the snapshot it was
/// launched with: a trigger in `dm-defs` and a bare-name `$schema` in home
/// apply to an open document, as `md schema validate` reports them.
#[test]
fn a_session_applies_schemas_dir_and_home_like_md() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    // Required properties make each applied layer visible as a diagnostic;
    // the server reports a missing one only in strict mode.
    write(&fixture.repo().join(".dmls.toml"), "[schema]\nstrict = true\n");
    write(&fixture.defs().join("defs.yaml"), "$schema:\n  from_defs: string(required)\n");
    write(&fixture.home().join("schemas/user.yaml"), "$schema:\n  from_user: string(required)\n");
    let document = fixture.repo().join("docs/x.md");
    let text = "---\n$schema: user.yaml\ntitle: x\n---\n\nBody\n";
    write(&document, text);

    let markdown = Markdown::from(text).with_source(ComposeSource::File(document.clone()));
    let md_context = build_resolution_context(&fixture.snapshot(document.parent().unwrap()))
        .unwrap()
        .for_source(&document);
    let report = DarkmatterSchemas::new(md_context)
        .with_trigger_discovery()
        .unwrap()
        .validate(&markdown)
        .unwrap();
    let md_missing: Vec<String> = ["from_defs", "from_user"]
        .into_iter()
        .filter(|name| report.problems.iter().any(|problem| format!("{problem:?}").contains(name)))
        .map(str::to_string)
        .collect();
    assert_eq!(md_missing, ["from_defs", "from_user"], "{:?}", report.problems);

    let mut session =
        LspFixture::start_with_snapshot(&workspace, fixture.snapshot(&fixture.root));
    session.initialize(initialize_params(&fixture.repo()));
    let uri = url::Url::from_file_path(&document).unwrap();
    open(&session, uri.as_str(), "markdown", text);
    let diagnostics = session.wait_for_diagnostics(uri.as_str());
    assert_eq!(missing_required(&diagnostics, &["from_defs", "from_user"]), md_missing, "{diagnostics:?}");
    session.shutdown();
}

/// A forbidden `$path` prefix is a definition error naming the pattern,
/// published on the trigger file with the message `md`'s scan reports.
#[test]
fn a_forbidden_path_prefix_is_the_same_definition_error_in_md_and_dmls() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let envelope_path = fixture.repo().join("schemas/bad.trigger.yaml");
    let envelope = trigger("@prompts/**", "bad.yaml");
    write(&envelope_path, &envelope);
    write(&fixture.repo().join("schemas/bad.yaml"), "$schema:\n  owner: string\n");
    let document = fixture.repo().join("docs/doc.md");
    let text = "---\ntitle: x\n---\n";
    write(&document, text);

    let definition_error =
        parse_trigger_envelope_from_str(&envelope).expect_err("`@` is refused").to_string();
    assert!(definition_error.contains("`@prompts/**`"), "{definition_error}");
    let md_context = build_resolution_context(&fixture.snapshot(document.parent().unwrap()))
        .unwrap()
        .for_source(&document);
    let md_error = scan(&md_context).expect_err("md's scan fails").to_string();
    assert!(md_error.contains(&definition_error), "{md_error}");

    let mut session =
        LspFixture::start_with_snapshot(&workspace, fixture.snapshot(&fixture.root));
    session.initialize(initialize_params(&fixture.repo()));
    let envelope_uri = url::Url::from_file_path(&envelope_path).unwrap();
    open(&session, envelope_uri.as_str(), "yaml", &envelope);
    let diagnostics = session.wait_for_diagnostics(envelope_uri.as_str());
    let messages: Vec<&str> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare"))
        .filter_map(|diagnostic| diagnostic["message"].as_str())
        .collect();
    assert!(
        messages.iter().any(|message| message.contains(&definition_error)),
        "expected `{definition_error}` in {messages:?}"
    );
    session.shutdown();
}
