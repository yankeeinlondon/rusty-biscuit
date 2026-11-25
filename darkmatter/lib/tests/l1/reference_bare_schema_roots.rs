//! Standalone reference preparation (no trigger schemas) resolves a bare
//! `$schema` name and a bare `Name@file` import from the document's schema
//! roots, as composition does: a bare name and an absolute path naming the
//! same schema give the same result, and a name no root holds still fails
//! preparation.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, ComposeSource};
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceValidationOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use std::path::Path;
use tempfile::TempDir;

/// The three spellings of the review document's schema.
#[derive(Clone, Copy, Debug)]
enum Spelling {
    Bare,
    Absolute,
    Missing,
}

impl Spelling {
    const ALL: [Self; 3] = [Self::Bare, Self::Absolute, Self::Missing];

    /// The reference to `file` (held in the repository's `schemas/` root).
    fn reference(self, root: &Path, file: &str) -> String {
        match self {
            Self::Bare => file.to_owned(),
            Self::Absolute => root.join("schemas").join(file).to_string_lossy().into_owned(),
            Self::Missing => "absent.yaml".to_owned(),
        }
    }
}

/// A temporary repository whose tree-root `schemas/` folder holds the review
/// schema and a types-only library.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::create_dir_all(repo.path().join("schemas")).unwrap();
    std::fs::write(
        repo.path().join("schemas/review.yaml"),
        "$schema:\n  description: string(required)\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("schemas/types.yaml"),
        "kind: schema\ntypes:\n  entry:\n    name: string(required)\n",
    )
    .unwrap();
    std::fs::write(repo.path().join("sibling.md"), "# Sibling\n").unwrap();
    repo
}

fn write(repo: &TempDir, name: &str, content: &str) {
    std::fs::write(repo.path().join(name), content).unwrap();
}

/// A request in the repository's context with no trigger schemas, as the
/// `md validate refs` and `md graph` commands prepare documents. The content
/// context is supplied and frozen so nothing scans the real working tree.
fn request(root: &Path) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")),
        context,
    )
}

fn load(repo: &TempDir, name: &str) -> Markdown {
    Markdown::try_from(repo.path().join(name).as_path()).unwrap()
}

/// `(node count, reference issue codes)` of the graph and its validation.
fn check(repo: &TempDir, name: &str, fragments: bool) -> Result<(usize, Vec<ReferenceIssueCode>), MarkdownError> {
    let md = load(repo, name);
    let graph_options = ReferenceGraphOptions::with_compose(&request(repo.path()));
    let nodes = md.reference_graph(graph_options.clone())?.node_count();
    let report = md.validate_references(ReferenceValidationOptions {
        validate_fragments: fragments,
        ..ReferenceValidationOptions::with_graph(graph_options)
    })?;
    Ok((nodes, report.issues.iter().map(|issue| issue.code).collect()))
}

fn assert_schema_unresolved(result: Result<(usize, Vec<ReferenceIssueCode>), MarkdownError>, shape: &str) {
    match result {
        Err(error @ MarkdownError::SchemaValidationFailed { .. }) => {
            assert!(error.to_string().contains("absent.yaml"), "{shape}: {error}");
        }
        other => panic!("{shape}: a missing schema must fail preparation, got {other:?}"),
    }
}

/// R5-F3.1–.3: a root document's bare `$schema` resolves and its link is
/// checked exactly as with the absolute spelling.
#[test]
fn bare_root_schema_prepares_like_the_absolute_path() {
    let repo = repository();
    let mut verdicts = Vec::new();
    for spelling in Spelling::ALL {
        let schema = spelling.reference(repo.path(), "review.yaml");
        write(
            &repo,
            "review.md",
            &format!("---\n$schema: '{schema}'\ndescription: Review heading\n---\n\n[sibling](./sibling.md)\n"),
        );
        let result = check(&repo, "review.md", false);
        match spelling {
            Spelling::Missing => assert_schema_unresolved(result, "root"),
            _ => verdicts.push(result.unwrap_or_else(|error| panic!("{spelling:?}: {error}"))),
        }
    }
    assert_eq!(verdicts[0], (1, Vec::new()), "bare spelling");
    assert_eq!(verdicts[0], verdicts[1], "bare and absolute spellings must agree");
}

/// R5-F3.4: a document with a bare `$schema` included through `::file` is
/// prepared in its own context.
#[test]
fn bare_schema_in_an_included_document_prepares() {
    let repo = repository();
    write(&repo, "root.md", "::file review.md\n");
    let mut verdicts = Vec::new();
    for spelling in Spelling::ALL {
        let schema = spelling.reference(repo.path(), "review.yaml");
        write(
            &repo,
            "review.md",
            &format!("---\n$schema: '{schema}'\ndescription: Review heading\n---\n\n[sibling](./sibling.md)\n"),
        );
        let result = check(&repo, "root.md", false);
        match spelling {
            Spelling::Missing => assert_schema_unresolved(result, "inclusion"),
            _ => verdicts.push(result.unwrap_or_else(|error| panic!("{spelling:?}: {error}"))),
        }
    }
    assert_eq!(verdicts[0], (2, Vec::new()), "bare spelling");
    assert_eq!(verdicts[0], verdicts[1], "bare and absolute spellings must agree");
}

/// R5-F3.5: a fragment naming a heading generated from declared frontmatter
/// is found in a target whose schema is bare-named; a static heading still
/// resolves through the fallback when the target's schema is missing.
#[test]
fn fragment_target_with_bare_schema_finds_generated_heading() {
    let repo = repository();
    write(&repo, "root.md", "[go](./review.md#review-heading)\n");
    for spelling in Spelling::ALL {
        let schema = spelling.reference(repo.path(), "review.yaml");
        for heading in ["{{ fm.description }}", "Review heading"] {
            write(
                &repo,
                "review.md",
                &format!("---\n$schema: '{schema}'\ndescription: Review heading\n---\n\n# {heading}\n"),
            );
            let (_, issues) = check(&repo, "root.md", true)
                .unwrap_or_else(|error| panic!("{spelling:?} {heading}: {error}"));
            let missing = issues.contains(&ReferenceIssueCode::MissingFragmentTarget);
            let generated = heading.starts_with("{{");
            match spelling {
                Spelling::Missing if generated => {
                    assert!(missing, "an unprepared target cannot generate its heading: {issues:?}");
                }
                _ => assert!(issues.is_empty(), "{spelling:?} {heading}: {issues:?}"),
            }
        }
    }
}

/// R5-F3.6: a bare `Name@file` import resolves in standalone preparation as
/// the absolute import does; a missing library still fails.
#[test]
fn bare_named_type_import_prepares_like_the_absolute_path() {
    let repo = repository();
    let mut verdicts = Vec::new();
    for spelling in Spelling::ALL {
        let library = spelling.reference(repo.path(), "types.yaml");
        write(
            &repo,
            "evidence.md",
            &format!(
                "---\n$schema:\n  entry: 'entry@{library}'\nentry:\n  name: first\n---\n\n[sibling](./sibling.md)\n"
            ),
        );
        let result = check(&repo, "evidence.md", false);
        match spelling {
            Spelling::Missing => assert_schema_unresolved(result, "import"),
            _ => verdicts.push(result.unwrap_or_else(|error| panic!("{spelling:?}: {error}"))),
        }
    }
    assert_eq!(verdicts[0], (1, Vec::new()), "bare spelling");
    assert_eq!(verdicts[0], verdicts[1], "bare and absolute spellings must agree");
}

/// Composition with trigger schemas off (`--disable schema-triggers`) and on
/// gives a bare `$schema` the same verdict as the absolute path.
#[test]
fn compose_resolves_bare_schema_with_or_without_trigger_schemas() {
    let repo = repository();
    for spelling in Spelling::ALL {
        let schema = spelling.reference(repo.path(), "review.yaml");
        write(&repo, "review.md", &format!("---\n$schema: '{schema}'\ndescription: Review heading\n---\n\n# Body\n"));
        for triggers in [false, true] {
            let md = load(&repo, "review.md");
            let context = FileResolutionContext::from_snapshot(repo.path(), None, std::collections::HashMap::new())
                .with_repository_root(repo.path());
            let request = crate::request_support::request_in(
                ComposeOptions::new_with_context(ComposeContext::capture_for_content(repo.path(), ""))
                    .with_trigger_schemas(triggers),
                context,
            );
            let result = md.compose_with(&request);
            match spelling {
                Spelling::Missing => assert!(
                    matches!(result, Err(MarkdownError::SchemaValidationFailed { .. })),
                    "triggers={triggers}: a missing schema must fail: {result:?}"
                ),
                _ => {
                    result.unwrap_or_else(|error| panic!("{spelling:?} triggers={triggers}: {error}"));
                }
            }
        }
    }
}

// ── A note outside the repository, opened through `~` ─────────────────────
//
// Whether a document may search the schema roots is decided by the
// document's own context, never by the request that reached it: a note in
// the (isolated) home directory, outside every repository, refuses a bare
// schema name whether it is checked directly, included by a repository
// document, or linked to from one.

/// The four spellings of the external note's schema: bare, relative to the
/// note, absolute, and a name no root holds.
#[derive(Clone, Copy, Debug, PartialEq)]
enum External {
    Bare,
    Relative,
    Absolute,
    Missing,
}

impl External {
    const ALL: [Self; 4] = [Self::Bare, Self::Relative, Self::Absolute, Self::Missing];

    fn reference(self, home: &Path, file: &str) -> String {
        match self {
            Self::Bare => file.to_owned(),
            Self::Relative => format!("./schemas/{file}"),
            Self::Absolute => home.join("schemas").join(file).to_string_lossy().into_owned(),
            Self::Missing => "absent.yaml".to_owned(),
        }
    }

    /// Whether this spelling names the schema file explicitly.
    fn explicit(self) -> bool {
        matches!(self, Self::Relative | Self::Absolute)
    }
}

/// An isolated home holding `review.md` and its `schemas/` root, a
/// repository whose documents include and link to `~/review.md`, and a
/// folder outside Git with the same inclusion.
struct HomeNote {
    base: TempDir,
}

impl HomeNote {
    fn new() -> Self {
        let base = TempDir::new().unwrap();
        let fixture = Self { base };
        std::fs::create_dir_all(fixture.home().join("schemas")).unwrap();
        std::fs::create_dir_all(fixture.repo().join(".git")).unwrap();
        std::fs::create_dir_all(fixture.outside()).unwrap();
        std::fs::write(
            fixture.home().join("schemas/review.yaml"),
            "$schema:\n  description: string(required)\n",
        )
        .unwrap();
        std::fs::write(fixture.home().join("schemas/types.yaml"), "kind: schema\ntypes:\n  entry: string\n").unwrap();
        for parent in [fixture.repo(), fixture.outside()] {
            std::fs::write(parent.join("root.md"), "::file ~/review.md\n").unwrap();
            std::fs::write(parent.join("link.md"), "[go](~/review.md#review-heading)\n").unwrap();
        }
        fixture
    }

    fn home(&self) -> std::path::PathBuf {
        self.base.path().join("home")
    }

    fn repo(&self) -> std::path::PathBuf {
        self.base.path().join("repo")
    }

    fn outside(&self) -> std::path::PathBuf {
        self.base.path().join("outside")
    }

    fn write_note(&self, frontmatter: &str, heading: &str) {
        std::fs::write(self.home().join("review.md"), format!("---\n{frontmatter}---\n\n# {heading}\n")).unwrap();
    }

    /// A note whose `$schema` is `schema`.
    fn write_schema_note(&self, schema: &str, heading: &str) {
        self.write_note(&format!("$schema: '{schema}'\ndescription: Review heading\n"), heading);
    }

    /// A request launched in `parent` with the isolated home; a repository
    /// root only for the repository parent. Nothing reads the process home,
    /// environment, or working tree.
    fn request(&self, parent: &Path, source: &Path, triggers: bool) -> ComposeRequest {
        let mut context = FileResolutionContext::from_snapshot(parent, Some(self.home()), std::collections::HashMap::new());
        if parent == self.repo() {
            context = context.with_repository_root(parent);
        }
        crate::request_support::request_in(
            ComposeOptions::new_with_context(ComposeContext::capture_for_content(parent, ""))
                .with_source_file(source)
                .with_trigger_schemas(triggers),
            context,
        )
    }

    fn load(&self, path: &Path) -> Markdown {
        Markdown::try_from(path).unwrap().with_source(ComposeSource::File(path.to_path_buf()))
    }

    /// `(node count, reference issue codes)` for `name` in `parent`, as the
    /// reference commands prepare it (no trigger schemas).
    fn check(&self, parent: &Path, name: &str, fragments: bool) -> Result<(usize, Vec<ReferenceIssueCode>), MarkdownError> {
        let source = parent.join(name);
        let md = self.load(&source);
        let request = self.request(parent, &source, false);
        let graph_options = ReferenceGraphOptions::with_compose(&request);
        let nodes = md.reference_graph(graph_options.clone())?.node_count();
        let report = md.validate_references(ReferenceValidationOptions {
            validate_fragments: fragments,
            ..ReferenceValidationOptions::with_graph(graph_options)
        })?;
        Ok((nodes, report.issues.iter().map(|issue| issue.code).collect()))
    }

    /// The composed body of `name` in `parent`. A child whose schema cannot
    /// be prepared is a tolerated transclusion failure, so its refusal is the
    /// report's warning, returned here as an error.
    fn compose(&self, parent: &Path, name: &str, triggers: bool) -> Result<String, String> {
        let source = parent.join(name);
        let (composed, report) = self
            .load(&source)
            .compose_with(&self.request(parent, &source, triggers))
            .map_err(|error| format!("{error} {error:?}"))?;
        if report.transclusions_skipped > 0 {
            return Err(format!("{report:?}"));
        }
        Ok(composed.content().to_owned())
    }
}

/// A composition whose failure names `reference`.
fn assert_compose_names(result: Result<String, String>, reference: &str, shape: &str) {
    match result {
        Err(text) => assert!(text.contains(reference), "{shape}: the failure must name `{reference}`: {text}"),
        Ok(text) => panic!("{shape}: `{reference}` must not resolve, got {text:?}"),
    }
}

/// An error that names `reference`, the schema spelling a root search
/// refused to resolve.
fn assert_names<T: std::fmt::Debug>(result: Result<T, MarkdownError>, reference: &str, shape: &str) {
    match result {
        Err(error) => {
            let text = format!("{error} {error:?}");
            assert!(text.contains(reference), "{shape}: the failure must name `{reference}`: {text}");
        }
        Ok(value) => panic!("{shape}: `{reference}` must not resolve, got {value:?}"),
    }
}

/// R6-F1.1/.2: an external note included by a repository parent, and by a
/// parent outside Git, refuses its bare `$schema`; the explicit spellings
/// resolve under both parents and a missing name fails.
#[test]
fn external_included_note_refuses_a_bare_schema_under_any_parent() {
    let fixture = HomeNote::new();
    for parent in [fixture.repo(), fixture.outside()] {
        for spelling in External::ALL {
            let schema = spelling.reference(&fixture.home(), "review.yaml");
            fixture.write_schema_note(&schema, "{{ fm.description }}");
            let result = fixture.check(&parent, "root.md", false);
            let shape = format!("{spelling:?} under {}", parent.display());
            if spelling.explicit() {
                assert_eq!(result.unwrap_or_else(|error| panic!("{shape}: {error}")), (2, Vec::new()), "{shape}");
            } else {
                assert_names(result, &schema, &shape);
            }
        }
    }
}

/// R6-F1.6 control: the note checked directly, in its own context (as
/// `md validate refs ~/review.md` opens it), refuses the bare name too.
#[test]
fn external_note_checked_directly_refuses_a_bare_schema() {
    let fixture = HomeNote::new();
    for spelling in External::ALL {
        let schema = spelling.reference(&fixture.home(), "review.yaml");
        fixture.write_schema_note(&schema, "{{ fm.description }}");
        let result = fixture.check(&fixture.home(), "review.md", false).map(|(nodes, _)| nodes);
        if spelling.explicit() {
            assert_eq!(result.unwrap_or_else(|error| panic!("{spelling:?}: {error}")), 1);
        } else {
            assert_names(result, &schema, &format!("{spelling:?} direct"));
        }
    }
}

/// R6-F1.3: a repository link finds the heading the external note generates
/// only when its schema is spelled explicitly; a static heading still
/// resolves through the fallback when the schema is missing or bare.
#[test]
fn external_fragment_target_generates_its_heading_only_from_an_explicit_schema() {
    let fixture = HomeNote::new();
    for spelling in External::ALL {
        let schema = spelling.reference(&fixture.home(), "review.yaml");
        for heading in ["{{ fm.description }}", "Review heading"] {
            fixture.write_schema_note(&schema, heading);
            let (_, issues) = fixture
                .check(&fixture.repo(), "link.md", true)
                .unwrap_or_else(|error| panic!("{spelling:?} {heading}: {error}"));
            if spelling.explicit() || !heading.starts_with("{{") {
                assert!(issues.is_empty(), "{spelling:?} {heading}: {issues:?}");
            } else {
                assert_eq!(issues, vec![ReferenceIssueCode::MissingFragmentTarget], "{spelling:?} {heading}");
            }
        }
    }
}

/// R6-F1.4: a `Name@file` import in the external note's inline schema
/// resolves only when spelled explicitly, in the reference check of the
/// inclusion and in composition with trigger schemas off.
#[test]
fn external_note_refuses_a_bare_named_type_import() {
    let fixture = HomeNote::new();
    for spelling in External::ALL {
        let library = spelling.reference(&fixture.home(), "types.yaml");
        fixture.write_note(
            &format!("$schema:\n  description: 'entry@{library}'\ndescription: Review heading\n"),
            "{{ fm.description }}",
        );
        let checked = fixture.check(&fixture.repo(), "root.md", false);
        let composed = fixture.compose(&fixture.repo(), "root.md", false);
        if spelling.explicit() {
            assert_eq!(checked.unwrap_or_else(|error| panic!("{spelling:?}: {error}")), (2, Vec::new()));
            let text = composed.unwrap_or_else(|error| panic!("{spelling:?} compose: {error}"));
            assert!(text.contains("# Review heading"), "{spelling:?}: {text}");
        } else {
            assert_names(checked, &library, &format!("{spelling:?} check"));
            assert_compose_names(composed, &library, &format!("{spelling:?} compose"));
        }
    }
}

/// R6-F1.5: composing the repository parent, with and without trigger
/// schemas, renders the generated heading only for an explicit schema.
#[test]
fn composing_a_repository_parent_refuses_the_external_bare_schema() {
    let fixture = HomeNote::new();
    for spelling in External::ALL {
        let schema = spelling.reference(&fixture.home(), "review.yaml");
        fixture.write_schema_note(&schema, "{{ fm.description }}");
        for triggers in [false, true] {
            let result = fixture.compose(&fixture.repo(), "root.md", triggers);
            let shape = format!("{spelling:?} triggers={triggers}");
            if spelling.explicit() {
                let text = result.unwrap_or_else(|error| panic!("{shape}: {error}"));
                assert!(text.contains("# Review heading"), "{shape}: {text}");
            } else {
                assert_compose_names(result, &schema, &shape);
            }
        }
    }
}

/// R6-F1.7: a trigger in the home schema root that requires an absent
/// `version` never applies to the external note a repository parent
/// includes, while the same trigger still applies to a repository child,
/// whose schema roots include `~/schemas`.
#[test]
fn root_triggers_apply_to_repository_children_but_not_to_an_external_note() {
    let fixture = HomeNote::new();
    std::fs::write(
        fixture.home().join("schemas/gate.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  topic: literal(review; required)\n$schema:\n  version: string(required)\n",
    )
    .unwrap();
    let schema = External::Absolute.reference(&fixture.home(), "review.yaml");
    for topic in ["review", "other"] {
        fixture.write_note(&format!("$schema: '{schema}'\ndescription: Review heading\ntopic: {topic}\n"), "{{ fm.description }}");
        let text = fixture
            .compose(&fixture.repo(), "root.md", true)
            .unwrap_or_else(|error| panic!("external topic={topic}: {error}"));
        assert!(text.contains("# Review heading"), "external topic={topic}: {text}");
    }

    std::fs::write(fixture.repo().join("child.md"), "---\ntopic: review\n---\n\n# In repository\n").unwrap();
    std::fs::write(fixture.repo().join("inner.md"), "::file ./child.md\n").unwrap();
    // The trigger is the only schema layer the child has, so a refusal
    // with triggers on and success with them off is the trigger applying.
    assert_compose_names(fixture.compose(&fixture.repo(), "inner.md", true), "child.md", "repository child");
    fixture
        .compose(&fixture.repo(), "inner.md", false)
        .expect("without trigger schemas the repository child composes");
}

// ── A note in a second checkout, reached from the first ──────────────────
//
// A source's own repository decides its schema roots. A note in a separate
// Git checkout (`~/notes`), included by or linked from a document in the
// launch checkout (`~/app`), searches `~/notes/schemas` for a bare `$schema`
// name, a bare `Name@file` import, and root triggers, exactly as when it is
// opened directly; a trigger of the launch checkout never applies to it.

/// The three ways the launch checkout's parent includes the note.
const INCLUSIONS: [(&str, &str); 3] = [
    ("body.md", "::file ~/notes/review.md\n"),
    ("prologue.md", "---\nprologue: ~/notes/review.md\n---\n\n# Root\n"),
    ("epilogue.md", "---\nepilogue: ~/notes/review.md\n---\n\n# Root\n"),
];

/// An isolated home holding two Git checkouts: `app`, where every request
/// is launched, and `notes`, whose `schemas/` root holds the note's schema
/// and a types-only library.
struct TwoCheckouts {
    base: TempDir,
}

impl TwoCheckouts {
    fn new() -> Self {
        let fixture = Self { base: TempDir::new().unwrap() };
        for checkout in [fixture.app(), fixture.notes()] {
            // The smallest layout Git discovery accepts; no `git` process.
            std::fs::create_dir_all(checkout.join(".git/objects")).unwrap();
            std::fs::create_dir_all(checkout.join(".git/refs/heads")).unwrap();
            std::fs::create_dir_all(checkout.join("schemas")).unwrap();
            std::fs::write(checkout.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
            std::fs::write(checkout.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();
        }
        std::fs::write(
            fixture.notes().join("schemas/review.yaml"),
            "$schema:\n  description: string(required)\n  state: enum(open, done)\n",
        )
        .unwrap();
        std::fs::write(fixture.notes().join("schemas/types.yaml"), "kind: schema\ntypes:\n  entry: string\n").unwrap();
        for (name, text) in INCLUSIONS {
            std::fs::write(fixture.app().join(name), text).unwrap();
        }
        std::fs::write(fixture.app().join("link.md"), "[go](~/notes/review.md#review-heading)\n").unwrap();
        std::fs::write(fixture.app().join("toc.md"), "::toc-linking ~/notes/review.md\n").unwrap();
        fixture
    }

    fn home(&self) -> std::path::PathBuf {
        self.base.path().join("home")
    }

    fn app(&self) -> std::path::PathBuf {
        self.home().join("app")
    }

    fn notes(&self) -> std::path::PathBuf {
        self.home().join("notes")
    }

    /// `file` in the notes checkout's `schemas/` root, spelled `spelling`
    /// from the note.
    fn reference(&self, spelling: External, file: &str) -> String {
        spelling.reference(&self.notes(), file)
    }

    fn write_note(&self, frontmatter: &str, heading: &str) {
        std::fs::write(
            self.notes().join("review.md"),
            format!("---\n{frontmatter}---\n\n# {heading}\n\n## Static\n"),
        )
        .unwrap();
    }

    fn write_schema_note(&self, spelling: External, heading: &str) -> String {
        let schema = self.reference(spelling, "review.yaml");
        self.write_note(&format!("$schema: '{schema}'\ndescription: Review heading\n"), heading);
        schema
    }

    fn write_import_note(&self, spelling: External) -> String {
        let library = self.reference(spelling, "types.yaml");
        self.write_note(
            &format!("$schema:\n  description: 'entry@{library}'\ndescription: Review heading\n"),
            "{{ fm.description }}",
        );
        library
    }

    /// A request launched in `checkout` (its context names only that
    /// repository) with the isolated home. Nothing reads the process home,
    /// environment, or working tree.
    fn request(&self, checkout: &Path, source: &Path, triggers: bool) -> ComposeRequest {
        let context = FileResolutionContext::from_snapshot(checkout, Some(self.home()), std::collections::HashMap::new())
            .with_repository_root(checkout);
        crate::request_support::request_in(
            ComposeOptions::new_with_context(ComposeContext::capture_for_content(checkout, ""))
                .with_source_file(source)
                .with_trigger_schemas(triggers),
            context,
        )
    }

    fn load(&self, path: &Path) -> Markdown {
        Markdown::try_from(path).unwrap().with_source(ComposeSource::File(path.to_path_buf()))
    }

    /// `(node count, reference issue codes)` for `path`, prepared as the
    /// reference commands prepare it (no trigger schemas) from a request
    /// launched in `checkout`.
    fn check_in(&self, checkout: &Path, path: &Path, fragments: bool) -> Result<(usize, Vec<ReferenceIssueCode>), MarkdownError> {
        let md = self.load(path);
        let graph_options = ReferenceGraphOptions::with_compose(&self.request(checkout, path, false));
        let nodes = md.reference_graph(graph_options.clone())?.node_count();
        let report = md.validate_references(ReferenceValidationOptions {
            validate_fragments: fragments,
            ..ReferenceValidationOptions::with_graph(graph_options)
        })?;
        Ok((nodes, report.issues.iter().map(|issue| issue.code).collect()))
    }

    /// [`Self::check_in`] for an app document.
    fn check(&self, name: &str, fragments: bool) -> Result<(usize, Vec<ReferenceIssueCode>), MarkdownError> {
        self.check_in(&self.app(), &self.app().join(name), fragments)
    }

    /// The composed body of the app document `name`; a skipped child's
    /// refusal is returned as an error, as in [`HomeNote::compose`].
    fn compose(&self, name: &str, triggers: bool) -> Result<String, String> {
        let source = self.app().join(name);
        let (composed, report) = self
            .load(&source)
            .compose_with(&self.request(&self.app(), &source, triggers))
            .map_err(|error| format!("{error} {error:?}"))?;
        if report.transclusions_skipped > 0 {
            return Err(format!("{report:?}"));
        }
        Ok(composed.content().to_owned())
    }

    /// Command discovery before composition (`md compose --shell`) of the
    /// app document `name`.
    fn preflight(&self, name: &str) -> Result<(), String> {
        let source = self.app().join(name);
        self.load(&source)
            .compose_preflight(&self.request(&self.app(), &source, true))
            .map(drop)
            .map_err(|error| format!("{error} {error:?}"))
    }
}

/// Asserts the R7-F1 verdict for one spelling: every resolvable spelling
/// gives `expected`, and the missing one fails naming `reference`.
fn assert_resolves_unless_missing<T: std::fmt::Debug + PartialEq>(
    spelling: External,
    result: Result<T, String>,
    expected: impl FnOnce(T),
    reference: &str,
    shape: &str,
) {
    match (spelling, result) {
        (External::Missing, Err(text)) => assert!(text.contains(reference), "{shape}: the failure must name `{reference}`: {text}"),
        (External::Missing, Ok(value)) => panic!("{shape}: `{reference}` must not resolve, got {value:?}"),
        (_, Ok(value)) => expected(value),
        (_, Err(text)) => panic!("{shape}: {text}"),
    }
}

fn shown<T>(result: Result<T, MarkdownError>) -> Result<T, String> {
    result.map_err(|error| format!("{error} {error:?}"))
}

/// R7-F1.1, .2, .4: one test per inclusion form. The reference report and
/// graph, composition with and without trigger schemas, and command
/// discovery accept the bare spelling exactly like the explicit ones and
/// refuse a missing schema at the child.
macro_rules! second_checkout_inclusion_tests {
    ($($test:ident => $parent:literal),* $(,)?) => {$(
        #[test]
        fn $test() {
            let fixture = TwoCheckouts::new();
            for spelling in External::ALL {
                let schema = fixture.write_schema_note(spelling, "{{ fm.description }}");
                let shape = format!("{spelling:?} {}", $parent);
                assert_resolves_unless_missing(
                    spelling,
                    shown(fixture.check($parent, false)),
                    |verdict| assert_eq!(verdict, (2, Vec::new()), "{shape}"),
                    &schema,
                    &format!("{shape} check"),
                );
                for triggers in [true, false] {
                    assert_resolves_unless_missing(
                        spelling,
                        fixture.compose($parent, triggers),
                        |text| assert!(text.contains("# Review heading"), "{shape} triggers={triggers}: {text}"),
                        &schema,
                        &format!("{shape} compose triggers={triggers}"),
                    );
                }
                assert_resolves_unless_missing(spelling, fixture.preflight($parent), |()| (), &schema, &format!("{shape} pre-flight"));
            }
        }
    )*};
}

second_checkout_inclusion_tests! {
    second_checkout_body_inclusion_uses_the_notes_schema_roots => "body.md",
    second_checkout_prologue_uses_the_notes_schema_roots => "prologue.md",
    second_checkout_epilogue_uses_the_notes_schema_roots => "epilogue.md",
}

/// R7-F1.3: a fragment link finds the heading the note generates for every
/// resolvable spelling; a missing schema loses only the generated heading,
/// and the static heading still resolves through the fallback.
#[test]
fn second_checkout_fragment_target_generates_its_heading_from_a_bare_schema() {
    let fixture = TwoCheckouts::new();
    for spelling in External::ALL {
        for heading in ["{{ fm.description }}", "Review heading"] {
            fixture.write_schema_note(spelling, heading);
            let (_, issues) = fixture.check("link.md", true).unwrap_or_else(|error| panic!("{spelling:?} {heading}: {error}"));
            if spelling == External::Missing && heading.starts_with("{{") {
                assert_eq!(issues, vec![ReferenceIssueCode::MissingFragmentTarget], "{spelling:?} {heading}");
            } else {
                assert!(issues.is_empty(), "{spelling:?} {heading}: {issues:?}");
            }
        }
    }
}

/// R7-F1.3: a `::toc-linking` target in the notes checkout prepares in the
/// reference check (which `md compose` runs first) for every resolvable
/// spelling, and a missing schema fails that preparation.
#[test]
fn second_checkout_toc_target_prepares_with_a_bare_schema() {
    let fixture = TwoCheckouts::new();
    for spelling in External::ALL {
        let schema = fixture.write_schema_note(spelling, "{{ fm.description }}");
        assert_resolves_unless_missing(
            spelling,
            shown(fixture.check("toc.md", false)),
            |verdict| assert_eq!(verdict, (2, Vec::new()), "{spelling:?}"),
            &schema,
            &format!("{spelling:?} check"),
        );
    }
}

/// R7-F1.5: a `Name@file` import in the note's inline schema resolves from
/// the notes checkout's roots for the bare spelling, directly and in every
/// nested position that prepares the note; a missing library fails there,
/// and the static fragment still resolves.
#[test]
fn second_checkout_note_resolves_a_bare_named_type_import() {
    let fixture = TwoCheckouts::new();
    let note = fixture.notes().join("review.md");
    for spelling in External::ALL {
        let library = fixture.write_import_note(spelling);
        let shape = format!("{spelling:?}");
        assert_resolves_unless_missing(
            spelling,
            shown(fixture.check_in(&fixture.notes(), &note, false)),
            |verdict| assert_eq!(verdict, (1, Vec::new()), "{shape} direct"),
            &library,
            &format!("{shape} direct"),
        );
        assert_resolves_unless_missing(
            spelling,
            shown(fixture.check("body.md", false)),
            |verdict| assert_eq!(verdict, (2, Vec::new()), "{shape} inclusion"),
            &library,
            &format!("{shape} inclusion"),
        );
        assert_resolves_unless_missing(
            spelling,
            fixture.compose("body.md", true),
            |text| assert!(text.contains("# Review heading"), "{shape}: {text}"),
            &library,
            &format!("{shape} compose"),
        );
        assert_resolves_unless_missing(spelling, fixture.preflight("body.md"), |()| (), &library, &format!("{shape} pre-flight"));
        assert_resolves_unless_missing(spelling, shown(fixture.check("toc.md", false)), drop, &library, &format!("{shape} toc"));
        let (_, issues) = fixture.check("link.md", true).unwrap_or_else(|error| panic!("{shape} fragment: {error}"));
        if spelling == External::Missing {
            assert_eq!(issues, vec![ReferenceIssueCode::MissingFragmentTarget], "{shape} fragment");
        } else {
            assert!(issues.is_empty(), "{shape} fragment: {issues:?}");
        }
    }
}

/// R7-F1.6: composing a launch-checkout parent applies the notes
/// checkout's trigger to the note it includes and never the launch
/// checkout's own trigger. The note's explicit schema does not require
/// `version`, so a refusal with triggers on and success with them off is the
/// notes trigger applying; a matching note that writes `version` but not
/// `parent_only` composes, so the launch trigger never applied. Another
/// topic and the passive reference check are unaffected.
#[test]
fn second_checkout_note_takes_root_triggers_from_its_own_checkout() {
    let fixture = TwoCheckouts::new();
    let trigger = |field: &str| {
        format!("kind: schema-trigger\nmatch:\n  topic: literal(review; required)\n$schema:\n  {field}: string(required)\n")
    };
    std::fs::write(fixture.notes().join("schemas/gate.trigger.yaml"), trigger("version")).unwrap();
    std::fs::write(fixture.app().join("schemas/gate.trigger.yaml"), trigger("parent_only")).unwrap();
    let schema = fixture.reference(External::Absolute, "review.yaml");
    for (extra, refused) in [("topic: review\n", true), ("topic: review\nversion: '1'\n", false), ("topic: other\n", false)] {
        fixture.write_note(&format!("$schema: '{schema}'\ndescription: Review heading\n{extra}"), "{{ fm.description }}");
        for (parent, _) in INCLUSIONS {
            let shape = format!("{extra:?} {parent}");
            let composed = fixture.compose(parent, true);
            assert_eq!(composed.is_err(), refused, "{shape}: {composed:?}");
            assert!(fixture.compose(parent, false).is_ok(), "{shape} without triggers");
            assert_eq!(fixture.check(parent, false).unwrap_or_else(|error| panic!("{shape} check: {error}")), (2, Vec::new()));
        }
    }
}

/// R7-F1.7 control: the note checked in its own checkout's context, as `md`
/// opens it directly, accepts every resolvable spelling and refuses a
/// missing one.
#[test]
fn second_checkout_note_checked_directly_resolves_a_bare_schema() {
    let fixture = TwoCheckouts::new();
    let note = fixture.notes().join("review.md");
    for spelling in External::ALL {
        let schema = fixture.write_schema_note(spelling, "{{ fm.description }}");
        assert_resolves_unless_missing(
            spelling,
            shown(fixture.check_in(&fixture.notes(), &note, false)),
            |verdict| assert_eq!(verdict, (1, Vec::new()), "{spelling:?}"),
            &schema,
            &format!("{spelling:?} direct"),
        );
    }
}
