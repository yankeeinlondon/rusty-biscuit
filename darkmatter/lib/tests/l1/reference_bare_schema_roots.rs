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

// R8-F1: a Git checkout nested inside another checkout is its own repository.
// Git membership (the nearest `.git` directory or file), never containment
// under a checkout already known to the request, decides which schema roots
// and root triggers a source uses, so the order sources are visited in
// cannot change it.

/// An isolated home holding `app` (where every request is launched) with a
/// component checkout `app/vendor` made the way `git init --separate-git-dir`
/// and submodules make one (a `.git` file naming a Git directory elsewhere),
/// and a `notes` checkout with a nested `notes/vendor` checkout (a `.git`
/// directory) and an unrelated `notes/seed.md`. Both components hold
/// `review.md` and a `schemas/` root.
struct NestedCheckouts {
    base: TempDir,
}

/// Where the component note sits: nested in the launch checkout, or nested in
/// the `notes` checkout outside the launch tree.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Component {
    Launch,
    Foreign,
}

const NESTED_PARENTS: [(&str, Component, &str, usize); 9] = [
    ("body.md", Component::Launch, "::file ./vendor/review.md\n", 2),
    ("prologue.md", Component::Launch, "---\nprologue: ./vendor/review.md\n---\n\n# Root\n", 2),
    ("epilogue.md", Component::Launch, "---\nepilogue: ./vendor/review.md\n---\n\n# Root\n", 2),
    ("foreign.md", Component::Foreign, "::file ~/notes/vendor/review.md\n", 2),
    ("seed_first.md", Component::Foreign, "::file ~/notes/seed.md\n\n::file ~/notes/vendor/review.md\n", 3),
    ("seed_last.md", Component::Foreign, "::file ~/notes/vendor/review.md\n\n::file ~/notes/seed.md\n", 3),
    ("seed_prologue.md", Component::Foreign, "---\nprologue: ~/notes/vendor/review.md\n---\n\n::file ~/notes/seed.md\n", 3),
    ("seed_epilogue.md", Component::Foreign, "---\nepilogue: ~/notes/vendor/review.md\n---\n\n::file ~/notes/seed.md\n", 3),
    ("seed_and_prologue.md", Component::Foreign, "---\nprologue: ~/notes/seed.md\nepilogue: ~/notes/vendor/review.md\n---\n\n# Root\n", 3),
];

/// The smallest Git directory discovery accepts; no `git` process.
fn git_dir(dir: &Path) {
    std::fs::create_dir_all(dir.join("objects")).unwrap();
    std::fs::create_dir_all(dir.join("refs/heads")).unwrap();
    std::fs::write(dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(dir.join("config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();
}

impl NestedCheckouts {
    fn new() -> Self {
        let fixture = Self { base: TempDir::new().unwrap() };
        let (app, notes) = (fixture.home().join("app"), fixture.home().join("notes"));
        git_dir(&app.join(".git"));
        git_dir(&notes.join(".git"));
        git_dir(&notes.join("vendor/.git"));
        let separate = fixture.base.path().join("component.git");
        git_dir(&separate);
        std::fs::create_dir_all(app.join("vendor")).unwrap();
        std::fs::write(app.join("vendor/.git"), format!("gitdir: {}\n", separate.display())).unwrap();
        for checkout in [&app, &notes, &app.join("vendor"), &notes.join("vendor")] {
            std::fs::create_dir_all(checkout.join("schemas")).unwrap();
        }
        for component in [Component::Launch, Component::Foreign] {
            let root = fixture.component(component);
            std::fs::write(
                root.join("schemas/review.yaml"),
                "$schema:\n  description: string(required)\n  state: enum(open, done)\n",
            )
            .unwrap();
            std::fs::write(root.join("schemas/types.yaml"), "kind: schema\ntypes:\n  entry: string\n").unwrap();
        }
        std::fs::write(notes.join("seed.md"), "# Seed\n").unwrap();
        for (name, _, text, _) in NESTED_PARENTS {
            std::fs::write(app.join(name), text).unwrap();
        }
        std::fs::write(app.join("link_launch.md"), "[go](./vendor/review.md#review-heading)\n").unwrap();
        std::fs::write(
            app.join("link_foreign.md"),
            "[seed](~/notes/seed.md#seed)\n[go](~/notes/vendor/review.md#review-heading)\n",
        )
        .unwrap();
        std::fs::write(app.join("toc_launch.md"), "::toc-linking ./vendor/review.md\n").unwrap();
        std::fs::write(app.join("toc_foreign.md"), "::file ~/notes/seed.md\n\n::toc-linking ~/notes/vendor/review.md\n")
            .unwrap();
        fixture
    }

    fn home(&self) -> std::path::PathBuf {
        self.base.path().join("home")
    }

    fn app(&self) -> std::path::PathBuf {
        self.home().join("app")
    }

    fn component(&self, component: Component) -> std::path::PathBuf {
        match component {
            Component::Launch => self.app().join("vendor"),
            Component::Foreign => self.home().join("notes/vendor"),
        }
    }

    fn note(&self, component: Component) -> std::path::PathBuf {
        self.component(component).join("review.md")
    }

    fn write_note(&self, component: Component, frontmatter: &str) {
        std::fs::write(
            self.note(component),
            format!("---\n{frontmatter}---\n\n# {{{{ fm.description }}}}\n\n## Static\n"),
        )
        .unwrap();
    }

    /// Writes the note with its `$schema` spelled `spelling`; the reference.
    fn write_schema_note(&self, component: Component, spelling: External) -> String {
        let schema = spelling.reference(&self.component(component), "review.yaml");
        self.write_note(component, &format!("$schema: '{schema}'\ndescription: Review heading\n"));
        schema
    }

    fn write_import_note(&self, component: Component, spelling: External) -> String {
        let library = spelling.reference(&self.component(component), "types.yaml");
        self.write_note(component, &format!("$schema:\n  description: 'entry@{library}'\ndescription: Review heading\n"));
        library
    }

    /// A request launched in `app` (its context names only that
    /// repository) with the isolated home, composing `source`.
    fn request(&self, source: &Path, triggers: bool) -> ComposeRequest {
        let app = self.app();
        let context = FileResolutionContext::from_snapshot(&app, Some(self.home()), std::collections::HashMap::new())
            .with_repository_root(&app);
        crate::request_support::request_in(
            ComposeOptions::new_with_context(ComposeContext::capture_for_content(&app, ""))
                .with_source_file(source)
                .with_trigger_schemas(triggers),
            context,
        )
    }

    fn load(&self, path: &Path) -> Markdown {
        Markdown::try_from(path).unwrap().with_source(ComposeSource::File(path.to_path_buf()))
    }

    /// `(node count, reference issue codes)` for `path`, prepared as the
    /// reference commands prepare it, from a request launched in `app`.
    fn check_path(&self, path: &Path, fragments: bool) -> Result<(usize, Vec<ReferenceIssueCode>), String> {
        let md = self.load(path);
        let graph_options = ReferenceGraphOptions::with_compose(&self.request(path, false));
        let run = || -> Result<_, MarkdownError> {
            let nodes = md.reference_graph(graph_options.clone())?.node_count();
            let report = md.validate_references(ReferenceValidationOptions {
                validate_fragments: fragments,
                ..ReferenceValidationOptions::with_graph(graph_options.clone())
            })?;
            Ok((nodes, report.issues.iter().map(|issue| issue.code).collect()))
        };
        shown(run())
    }

    fn check(&self, name: &str, fragments: bool) -> Result<(usize, Vec<ReferenceIssueCode>), String> {
        self.check_path(&self.app().join(name), fragments)
    }

    /// The composed body of `path` launched from `app`; a skipped child's
    /// refusal is returned as an error.
    fn compose_path(&self, path: &Path, triggers: bool) -> Result<String, String> {
        let (composed, report) = self
            .load(path)
            .compose_with(&self.request(path, triggers))
            .map_err(|error| format!("{error} {error:?}"))?;
        if report.transclusions_skipped > 0 {
            return Err(format!("{report:?}"));
        }
        Ok(composed.content().to_owned())
    }

    fn compose(&self, name: &str, triggers: bool) -> Result<String, String> {
        self.compose_path(&self.app().join(name), triggers)
    }

    fn preflight(&self, name: &str) -> Result<(), String> {
        let source = self.app().join(name);
        self.load(&source)
            .compose_preflight(&self.request(&source, true))
            .map(drop)
            .map_err(|error| format!("{error} {error:?}"))
    }
}

/// R8-F1.2, .4: one test per parent. The reference report and graph,
/// composition with and without trigger schemas, and command discovery
/// accept the bare spelling exactly like the explicit ones, whether the
/// component is nested in the launch checkout or in an unrelated checkout
/// the request visits (`seed.md`) before or after it, and refuse a missing
/// schema at the component.
macro_rules! nested_checkout_inclusion_tests {
    ($($test:ident => $index:literal),* $(,)?) => {$(
        #[test]
        fn $test() {
            let (parent, component, _, nodes) = NESTED_PARENTS[$index];
            let fixture = NestedCheckouts::new();
            for spelling in External::ALL {
                let schema = fixture.write_schema_note(component, spelling);
                let shape = format!("{spelling:?} {parent}");
                assert_resolves_unless_missing(
                    spelling,
                    fixture.check(parent, false),
                    |verdict| assert_eq!(verdict, (nodes, Vec::new()), "{shape}"),
                    &schema,
                    &format!("{shape} check"),
                );
                for triggers in [true, false] {
                    assert_resolves_unless_missing(
                        spelling,
                        fixture.compose(parent, triggers),
                        |text| assert!(text.contains("# Review heading"), "{shape} triggers={triggers}: {text}"),
                        &schema,
                        &format!("{shape} compose triggers={triggers}"),
                    );
                }
                assert_resolves_unless_missing(spelling, fixture.preflight(parent), |()| (), &schema, &format!("{shape} pre-flight"));
            }
        }
    )*};
}

nested_checkout_inclusion_tests! {
    nested_checkout_body_inclusion_uses_the_component_schema_roots => 0,
    nested_checkout_prologue_uses_the_component_schema_roots => 1,
    nested_checkout_epilogue_uses_the_component_schema_roots => 2,
    foreign_nested_checkout_alone_uses_the_component_schema_roots => 3,
    foreign_nested_checkout_after_the_seed_uses_the_component_schema_roots => 4,
    foreign_nested_checkout_before_the_seed_uses_the_component_schema_roots => 5,
    foreign_nested_checkout_prologue_with_the_seed_uses_the_component_schema_roots => 6,
    foreign_nested_checkout_epilogue_after_the_seed_uses_the_component_schema_roots => 7,
    foreign_nested_checkout_after_a_seed_prologue_uses_the_component_schema_roots => 8,
}

/// R8-F1.1 (library side): the component note opened as the root of a
/// request launched in `app` accepts every resolvable spelling and refuses
/// a missing one, in the reference check and in composition.
#[test]
fn nested_checkout_note_opened_directly_from_the_launch_checkout_resolves_a_bare_schema() {
    let fixture = NestedCheckouts::new();
    for component in [Component::Launch, Component::Foreign] {
        let note = fixture.note(component);
        for spelling in External::ALL {
            let schema = fixture.write_schema_note(component, spelling);
            let shape = format!("{component:?} {spelling:?}");
            assert_resolves_unless_missing(
                spelling,
                fixture.check_path(&note, false),
                |verdict| assert_eq!(verdict, (1, Vec::new()), "{shape}"),
                &schema,
                &format!("{shape} direct check"),
            );
            assert_resolves_unless_missing(
                spelling,
                fixture.compose_path(&note, true),
                |text| assert!(text.contains("# Review heading"), "{shape}: {text}"),
                &schema,
                &format!("{shape} direct compose"),
            );
        }
    }
}

/// R8-F1.3: a fragment link and a `::toc-linking` target in a nested
/// checkout generate the heading for every resolvable spelling, including
/// after the seed note of the enclosing foreign checkout was visited; a
/// missing schema loses only the generated heading (the static one still
/// resolves) and fails the TOC target's preparation.
#[test]
fn nested_checkout_fragment_and_toc_targets_generate_their_heading_from_a_bare_schema() {
    let fixture = NestedCheckouts::new();
    for (component, link, toc, toc_nodes) in
        [(Component::Launch, "link_launch.md", "toc_launch.md", 2), (Component::Foreign, "link_foreign.md", "toc_foreign.md", 3)]
    {
        for spelling in External::ALL {
            let schema = fixture.write_schema_note(component, spelling);
            let shape = format!("{component:?} {spelling:?}");
            let (_, issues) = fixture.check(link, true).unwrap_or_else(|error| panic!("{shape} fragment: {error}"));
            if spelling == External::Missing {
                assert_eq!(issues, vec![ReferenceIssueCode::MissingFragmentTarget], "{shape} fragment");
            } else {
                assert!(issues.is_empty(), "{shape} fragment: {issues:?}");
            }
            assert_resolves_unless_missing(
                spelling,
                fixture.check(toc, false),
                |verdict| assert_eq!(verdict, (toc_nodes, Vec::new()), "{shape} toc"),
                &schema,
                &format!("{shape} toc"),
            );
        }
        std::fs::write(fixture.note(component), "---\n$schema: absent.yaml\ndescription: x\n---\n\n# Static\n").unwrap();
        // A portable spelling: a Windows path's backslashes are not a link target.
        let target = match component {
            Component::Launch => "./vendor/review.md",
            Component::Foreign => "~/notes/vendor/review.md",
        };
        std::fs::write(fixture.app().join("static.md"), format!("[go]({target}#static)\n")).unwrap();
        let (_, issues) = fixture.check("static.md", true).unwrap_or_else(|error| panic!("{component:?} static: {error}"));
        assert!(issues.is_empty(), "{component:?} static fragment: {issues:?}");
    }
}

/// R8-F1.5: an inline `Name@file` import resolves from the component's own
/// roots for the bare spelling, directly, through an inclusion, and after
/// the seed; a missing library fails.
#[test]
fn nested_checkout_note_resolves_a_bare_named_type_import() {
    let fixture = NestedCheckouts::new();
    for (component, parent, nodes) in [(Component::Launch, "body.md", 2), (Component::Foreign, "seed_first.md", 3)] {
        for spelling in External::ALL {
            let library = fixture.write_import_note(component, spelling);
            let shape = format!("{component:?} {spelling:?}");
            assert_resolves_unless_missing(
                spelling,
                fixture.check_path(&fixture.note(component), false),
                |verdict| assert_eq!(verdict, (1, Vec::new()), "{shape} direct"),
                &library,
                &format!("{shape} direct"),
            );
            assert_resolves_unless_missing(
                spelling,
                fixture.check(parent, false),
                |verdict| assert_eq!(verdict, (nodes, Vec::new()), "{shape} inclusion"),
                &library,
                &format!("{shape} inclusion"),
            );
            assert_resolves_unless_missing(
                spelling,
                fixture.compose(parent, true),
                |text| assert!(text.contains("# Review heading"), "{shape}: {text}"),
                &library,
                &format!("{shape} compose"),
            );
        }
    }
}

/// R8-F1.6: with an explicit schema path, a matching component note takes
/// the component checkout's trigger (`version`) and never the enclosing
/// checkout's (`parent_only`), directly and through every inclusion form in
/// both visit orders; another topic and disabled triggers compose, and the
/// passive reference check passes either way.
fn assert_nested_checkout_trigger(component: Component, extra: &str, refused: bool) {
    let fixture = NestedCheckouts::new();
    let trigger = |field: &str| {
        format!("kind: schema-trigger\nmatch:\n  topic: literal(review; required)\n$schema:\n  {field}: string(required)\n")
    };
    std::fs::write(fixture.component(component).join("schemas/gate.trigger.yaml"), trigger("version")).unwrap();
    for enclosing in [fixture.app(), fixture.home().join("notes")] {
        std::fs::write(enclosing.join("schemas/gate.trigger.yaml"), trigger("parent_only")).unwrap();
    }
    let schema = External::Relative.reference(&fixture.component(component), "review.yaml");
    fixture.write_note(component, &format!("$schema: '{schema}'\ndescription: Review heading\n{extra}"));
    let parents = NESTED_PARENTS.iter().filter(|(_, at, _, _)| *at == component).map(|(name, ..)| fixture.app().join(name));
    for path in std::iter::once(fixture.note(component)).chain(parents) {
        let shape = format!("{extra:?} {}", path.display());
        // The explicit schema requires neither field: a refusal is a
        // trigger applying, and the matching note that writes only
        // `version` composing proves `parent_only` never applied.
        let composed = fixture.compose_path(&path, true);
        assert_eq!(composed.is_err(), refused, "{shape}: {composed:?}");
        assert!(fixture.compose_path(&path, false).is_ok(), "{shape} without triggers");
        fixture.check_path(&path, false).unwrap_or_else(|error| panic!("{shape} check: {error}"));
    }
}

// One test per component and frontmatter case: the inclusion forms of one
// case already take seconds on a Windows runner.
macro_rules! nested_checkout_trigger_tests {
    ($($name:ident: $component:expr, $extra:expr, $refused:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_nested_checkout_trigger($component, $extra, $refused);
            }
        )*
    };
}

nested_checkout_trigger_tests! {
    nested_checkout_matching_note_takes_the_launch_component_trigger: Component::Launch, "topic: review\n", true;
    nested_checkout_launch_component_trigger_is_satisfied_by_version: Component::Launch, "topic: review\nversion: '1'\n", false;
    nested_checkout_launch_component_other_topic_matches_no_trigger: Component::Launch, "topic: other\n", false;
    nested_checkout_matching_note_takes_the_foreign_component_trigger: Component::Foreign, "topic: review\n", true;
    nested_checkout_foreign_component_trigger_is_satisfied_by_version: Component::Foreign, "topic: review\nversion: '1'\n", false;
    nested_checkout_foreign_component_other_topic_matches_no_trigger: Component::Foreign, "topic: other\n", false;
}

// R9-F1: a root document's checkout comes from where the document is, never
// from how a library caller supplied it. The component note `app/vendor/
// review.md` is given four ways to a request prepared in `app` (an absolute
// path on the request and the document, a relative path on both, an absolute
// document with a relative request path, and only the loaded document), and
// every entry point must resolve its schema roots and `&marker.md` in the
// component.

/// How a caller supplies the root document's source.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SourceForm {
    Absolute,
    Relative,
    AbsoluteDocumentRelativeRequest,
    LoadedDocumentOnly,
}

/// The component note relative to `app`, the request directory.
fn relative_note() -> std::path::PathBuf {
    Path::new("vendor").join("review.md")
}

impl SourceForm {
    fn request_source(self, note: &Path) -> Option<std::path::PathBuf> {
        match self {
            Self::Absolute => Some(note.to_path_buf()),
            Self::Relative | Self::AbsoluteDocumentRelativeRequest => Some(relative_note()),
            Self::LoadedDocumentOnly => None,
        }
    }

    fn document_source(self, note: &Path) -> std::path::PathBuf {
        match self {
            Self::Relative => relative_note(),
            _ => note.to_path_buf(),
        }
    }
}

/// [`NestedCheckouts`] with distinct `marker.md` files in `app` and in the
/// `app/vendor` component, each linking to its own URL, and requests
/// prepared through `ComposeRequest::prepare` the way an embedder prepares
/// one (real repository discovery at the request directory).
struct SourceForms {
    checkouts: NestedCheckouts,
}

impl SourceForms {
    fn new() -> Self {
        let checkouts = NestedCheckouts::new();
        std::fs::write(checkouts.app().join("marker.md"), "Launch marker [launch](https://launch.example/)\n").unwrap();
        std::fs::write(
            checkouts.component(Component::Launch).join("marker.md"),
            "Component marker [component](https://component.example/)\n",
        )
        .unwrap();
        Self { checkouts }
    }

    fn note(&self) -> std::path::PathBuf {
        self.checkouts.note(Component::Launch)
    }

    fn component(&self) -> std::path::PathBuf {
        self.checkouts.component(Component::Launch)
    }

    /// Writes the note with `frontmatter` and a `&marker.md` inclusion.
    fn write_note(&self, frontmatter: &str) {
        std::fs::write(self.note(), format!("---\n{frontmatter}---\n\n# {{{{ fm.description }}}}\n\n::file &marker.md\n"))
            .unwrap();
    }

    fn write_schema_note(&self, spelling: External) -> String {
        let schema = spelling.reference(&self.component(), "review.yaml");
        self.write_note(&format!("$schema: '{schema}'\ndescription: Review heading\n"));
        schema
    }

    fn write_import_note(&self, spelling: External) -> String {
        let library = spelling.reference(&self.component(), "types.yaml");
        self.write_note(&format!("$schema:\n  description: 'entry@{library}'\ndescription: Review heading\n"));
        library
    }

    fn snapshot_at(&self, dir: &Path) -> darkmatter::markdown::compose::RequestSnapshot {
        darkmatter::markdown::compose::RequestSnapshot::new(dir).with_home(Some(self.checkouts.home()))
    }

    fn options(&self, form: SourceForm, triggers: bool) -> ComposeOptions {
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.checkouts.app(), ""))
            .with_trigger_schemas(triggers);
        match form.request_source(&self.note()) {
            Some(source) => options.with_source_file(source),
            None => options,
        }
    }

    /// A request prepared in `app` with the source `form` puts on it.
    fn request(&self, form: SourceForm, triggers: bool) -> ComposeRequest {
        ComposeRequest::prepare(self.options(form, triggers), &self.snapshot_at(&self.checkouts.app())).unwrap()
    }

    /// The note as `form` loads it: read from disk, with `form`'s source.
    fn document(&self, form: SourceForm) -> Markdown {
        Markdown::try_from(self.note().as_path()).unwrap().with_source(ComposeSource::File(form.document_source(&self.note())))
    }

    fn graph_options(&self, form: SourceForm) -> ReferenceGraphOptions {
        ReferenceGraphOptions::with_compose(&self.request(form, false))
    }

    /// The composed body through the three compose entry points, which must
    /// agree.
    fn compose(&self, form: SourceForm, triggers: bool) -> Result<String, String> {
        let (document, request) = (self.document(form), self.request(form, triggers));
        let plain = shown(document.compose_with(&request).map(|(composed, _)| composed.content().to_owned()));
        let warnings = document
            .compose_with_warnings(&request)
            .map(|(composed, _)| composed.content().to_owned())
            .map_err(|failure| format!("{} {:?}", failure.error, failure.error));
        let mut hook = |_: &darkmatter::markdown::Frontmatter, _: &mut std::collections::HashMap<String, String>| Ok(());
        let hooked = shown(
            document.compose_with_frontmatter_hook(&request, &mut hook).map(|(composed, _)| composed.content().to_owned()),
        );
        assert_eq!(plain.is_ok(), warnings.is_ok(), "{form:?} compose_with_warnings: {warnings:?}");
        assert_eq!(plain.is_ok(), hooked.is_ok(), "{form:?} compose_with_frontmatter_hook: {hooked:?}");
        if let (Ok(plain), Ok(warnings), Ok(hooked)) = (&plain, &warnings, &hooked) {
            assert_eq!(plain, warnings, "{form:?}");
            assert_eq!(plain, hooked, "{form:?}");
        }
        plain
    }

    fn preflight(&self, form: SourceForm) -> Result<(), String> {
        shown(self.document(form).compose_preflight(&self.request(form, true)).map(drop))
    }

    /// The marker file every graph form reaches from the note.
    fn graph_marker(&self, form: SourceForm) -> Result<std::path::PathBuf, String> {
        let document = self.document(form);
        let transclusion = shown(document.transclusion_graph(self.graph_options(form)))?;
        let full = shown(document.reference_graph(self.graph_options(form)))?;
        let markers: Vec<_> = [&transclusion, &full]
            .into_iter()
            .map(|graph| match graph.nodes() {
                [node] => match &node.source {
                    ComposeSource::File(path) => path.clone(),
                    other => panic!("{form:?}: {other:?}"),
                },
                nodes => panic!("{form:?}: one marker node expected: {nodes:?}"),
            })
            .collect();
        assert_eq!(markers[0], markers[1], "{form:?}: both graphs reach one marker");
        let flattened = shown(document.composed_references(self.graph_options(form)))?;
        let urls: Vec<_> = flattened.records.iter().filter_map(|record| record.target.raw()).collect();
        assert!(urls.contains(&"https://component.example/"), "{form:?} flattened: {urls:?}");
        assert!(!urls.contains(&"https://launch.example/"), "{form:?} flattened: {urls:?}");
        Ok(markers[0].clone())
    }

    /// Reference validation, fresh and over a prebuilt graph: issue codes.
    fn validate(&self, form: SourceForm, prebuilt: bool) -> Result<Vec<ReferenceIssueCode>, String> {
        let document = self.document(form);
        let options = ReferenceValidationOptions::with_graph(self.graph_options(form));
        let report = if prebuilt {
            let graph = shown(document.reference_graph(options.graph.clone()))?;
            shown(document.validate_references_with_graph(&graph, options))?
        } else {
            shown(document.validate_references(options))?
        };
        Ok(report.issues.iter().map(|issue| issue.code).collect())
    }

    fn transclusion_target(&self, form: SourceForm) -> Result<String, String> {
        let refs = shown(self.document(form).transclusions_with_options(&self.request(form, false)))?;
        match refs.as_slice() {
            [only] => Ok(only.resolved_target.clone().unwrap_or_default()),
            other => panic!("{form:?}: one transclusion expected: {other:?}"),
        }
    }
}

fn is_component_marker(path: &Path) -> bool {
    path.ends_with(Path::new("vendor").join("marker.md"))
}

/// Which entry point a row exercises.
#[derive(Clone, Copy, Debug)]
enum EntryPoint {
    Compose,
    Preflight,
    Graph,
    Validate,
    ValidatePrebuilt,
    Transclusions,
    EffectiveSchema,
}

fn assert_source_form(form: SourceForm, entry: EntryPoint) {
    let fixture = SourceForms::new();
    for kind in ["schema", "import"] {
        for spelling in External::ALL {
            let reference =
                if kind == "schema" { fixture.write_schema_note(spelling) } else { fixture.write_import_note(spelling) };
            let shape = format!("{form:?} {entry:?} {kind} {spelling:?}");
            match entry {
                EntryPoint::Compose => assert_resolves_unless_missing(
                    spelling,
                    fixture.compose(form, true),
                    |text| {
                        assert!(text.contains("# Review heading"), "{shape}: {text}");
                        assert!(text.contains("Component marker"), "{shape}: {text}");
                    },
                    &reference,
                    &shape,
                ),
                EntryPoint::Preflight => {
                    assert_resolves_unless_missing(spelling, fixture.preflight(form), |()| (), &reference, &shape);
                }
                EntryPoint::Graph => assert_resolves_unless_missing(
                    spelling,
                    fixture.graph_marker(form),
                    |marker| assert!(is_component_marker(&marker), "{shape}: {}", marker.display()),
                    &reference,
                    &shape,
                ),
                EntryPoint::Validate | EntryPoint::ValidatePrebuilt => assert_resolves_unless_missing(
                    spelling,
                    fixture.validate(form, matches!(entry, EntryPoint::ValidatePrebuilt)),
                    |issues| assert!(issues.is_empty(), "{shape}: {issues:?}"),
                    &reference,
                    &shape,
                ),
                // A host's pre-run check: the schema composition will use.
                EntryPoint::EffectiveSchema => {
                    let result = darkmatter::markdown::compose::effective_schema_for(
                        &fixture.document(form),
                        &fixture.request(form, true),
                    )
                    .map(|(schema, _)| schema.is_some());
                    match (spelling, result) {
                        (External::Missing, Ok(true)) => panic!("{shape}: `{reference}` must not resolve"),
                        (External::Missing, _) | (_, Ok(true)) => {}
                        (_, other) => panic!("{shape}: {other:?}"),
                    }
                }
                // A passive local query: it prepares no schema, so only the
                // marker it resolves matters.
                EntryPoint::Transclusions => {
                    let target = fixture.transclusion_target(form).unwrap_or_else(|error| panic!("{shape}: {error}"));
                    assert!(is_component_marker(Path::new(&target)), "{shape}: {target}");
                }
            }
        }
    }
}

// One test per source form and entry point, so each stays fast on a
// Windows runner.
macro_rules! source_form_tests {
    ($($name:ident: $form:expr, $entry:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_source_form($form, $entry);
            }
        )*
    };
}

source_form_tests! {
    absolute_source_composes_in_the_component: SourceForm::Absolute, EntryPoint::Compose;
    relative_source_composes_in_the_component: SourceForm::Relative, EntryPoint::Compose;
    relative_request_source_composes_in_the_component: SourceForm::AbsoluteDocumentRelativeRequest, EntryPoint::Compose;
    loaded_document_source_composes_in_the_component: SourceForm::LoadedDocumentOnly, EntryPoint::Compose;
    absolute_source_preflight_uses_the_component: SourceForm::Absolute, EntryPoint::Preflight;
    relative_source_preflight_uses_the_component: SourceForm::Relative, EntryPoint::Preflight;
    relative_request_source_preflight_uses_the_component: SourceForm::AbsoluteDocumentRelativeRequest, EntryPoint::Preflight;
    loaded_document_source_preflight_uses_the_component: SourceForm::LoadedDocumentOnly, EntryPoint::Preflight;
    absolute_source_graphs_reach_the_component_marker: SourceForm::Absolute, EntryPoint::Graph;
    relative_source_graphs_reach_the_component_marker: SourceForm::Relative, EntryPoint::Graph;
    relative_request_source_graphs_reach_the_component_marker: SourceForm::AbsoluteDocumentRelativeRequest, EntryPoint::Graph;
    loaded_document_source_graphs_reach_the_component_marker: SourceForm::LoadedDocumentOnly, EntryPoint::Graph;
    absolute_source_validates_in_the_component: SourceForm::Absolute, EntryPoint::Validate;
    relative_source_validates_in_the_component: SourceForm::Relative, EntryPoint::Validate;
    relative_request_source_validates_in_the_component: SourceForm::AbsoluteDocumentRelativeRequest, EntryPoint::Validate;
    loaded_document_source_validates_in_the_component: SourceForm::LoadedDocumentOnly, EntryPoint::Validate;
    absolute_source_validates_a_prebuilt_graph: SourceForm::Absolute, EntryPoint::ValidatePrebuilt;
    relative_source_validates_a_prebuilt_graph: SourceForm::Relative, EntryPoint::ValidatePrebuilt;
    relative_request_source_validates_a_prebuilt_graph: SourceForm::AbsoluteDocumentRelativeRequest, EntryPoint::ValidatePrebuilt;
    loaded_document_source_validates_a_prebuilt_graph: SourceForm::LoadedDocumentOnly, EntryPoint::ValidatePrebuilt;
    absolute_source_transclusions_query_the_component_marker: SourceForm::Absolute, EntryPoint::Transclusions;
    relative_source_transclusions_query_the_component_marker: SourceForm::Relative, EntryPoint::Transclusions;
    relative_request_source_transclusions_query_the_component_marker: SourceForm::AbsoluteDocumentRelativeRequest, EntryPoint::Transclusions;
    loaded_document_source_transclusions_query_the_component_marker: SourceForm::LoadedDocumentOnly, EntryPoint::Transclusions;
    absolute_source_effective_schema_is_the_component_schema: SourceForm::Absolute, EntryPoint::EffectiveSchema;
    relative_source_effective_schema_is_the_component_schema: SourceForm::Relative, EntryPoint::EffectiveSchema;
    relative_request_source_effective_schema_is_the_component_schema: SourceForm::AbsoluteDocumentRelativeRequest, EntryPoint::EffectiveSchema;
    loaded_document_source_effective_schema_is_the_component_schema: SourceForm::LoadedDocumentOnly, EntryPoint::EffectiveSchema;
}

/// R9-F1.2: with checkout-specific triggers (the component requires
/// `version`, the launch checkout `parent_only`), a matching note is refused
/// until it writes `version`, whichever way its source was supplied.
fn assert_source_form_trigger(form: SourceForm) {
    let fixture = SourceForms::new();
    let trigger = |field: &str| {
        format!("kind: schema-trigger\nmatch:\n  topic: literal(review; required)\n$schema:\n  {field}: string(required)\n")
    };
    std::fs::write(fixture.component().join("schemas/gate.trigger.yaml"), trigger("version")).unwrap();
    std::fs::write(fixture.checkouts.app().join("schemas/gate.trigger.yaml"), trigger("parent_only")).unwrap();
    for (extra, refused) in [("topic: review\n", true), ("topic: review\nversion: '1'\n", false)] {
        fixture.write_note(&format!("$schema: ./schemas/review.yaml\ndescription: Review heading\n{extra}"));
        let composed = fixture.compose(form, true);
        assert_eq!(composed.is_err(), refused, "{form:?} {extra:?}: {composed:?}");
        if refused {
            let error = composed.unwrap_err();
            assert!(error.contains("version") && !error.contains("parent_only"), "{form:?}: {error}");
        }
    }
}

macro_rules! source_form_trigger_tests {
    ($($name:ident: $form:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_source_form_trigger($form);
            }
        )*
    };
}

source_form_trigger_tests! {
    absolute_source_takes_the_component_trigger: SourceForm::Absolute;
    relative_source_takes_the_component_trigger: SourceForm::Relative;
    relative_request_source_takes_the_component_trigger: SourceForm::AbsoluteDocumentRelativeRequest;
    loaded_document_source_takes_the_component_trigger: SourceForm::LoadedDocumentOnly;
}

/// R9-F1.6: the local expression context of an embedder's request has the
/// component's directory and checkout for the relative and absolute source,
/// through both APIs.
#[test]
fn local_expression_context_uses_the_component_for_either_source_spelling() {
    let fixture = SourceForms::new();
    fixture.write_schema_note(External::Bare);
    let app = fixture.checkouts.app();
    let context = darkmatter::markdown::compose::build_resolution_context(&fixture.snapshot_at(&app)).unwrap();
    let component = biscuit_file::canonicalize_simplified(&fixture.component()).unwrap();
    let canonical = |path: &Path| biscuit_file::canonicalize_simplified(path).unwrap();
    for form in [SourceForm::Absolute, SourceForm::Relative] {
        let local = ComposeRequest::local_expression_context(fixture.options(form, false), &context).unwrap();
        let prepared = fixture.request(form, false).local_expression_resolution_context();
        for (api, resolved) in [("local_expression_context", local), ("local_expression_resolution_context", prepared)] {
            assert!(resolved.cwd.is_absolute(), "{form:?} {api} cwd: {}", resolved.cwd.display());
            assert_eq!(canonical(&resolved.cwd), component, "{form:?} {api} cwd");
            let root = resolved.repository_root.as_deref().map(canonical);
            assert_eq!(root.as_deref(), Some(component.as_path()), "{form:?} {api} repository");
        }
    }
}

/// R9-F1.7: a request whose directory is the component establishes the
/// component's resolution root whatever repository its options observed:
/// fresh options, options captured in the component, and options captured
/// in the parent all render the component's marker, and the parent capture
/// keeps its own anchor. Control: the parent capture with the request in
/// the parent admits the absolute source into the component.
#[test]
fn a_captured_parent_observation_is_not_reused_for_a_component_request() {
    let fixture = SourceForms::new();
    fixture.write_schema_note(External::Bare);
    let (app, component, note) = (fixture.checkouts.app(), fixture.component(), fixture.note());
    let document = Markdown::try_from(note.as_path()).unwrap();
    // A capture that names `ctx.cwd` is anchored where it was taken.
    let capture = Markdown::new("{{ ctx.cwd }}\n");
    let captured = |anchor: &Path| ComposeOptions::for_document(anchor, &capture);
    let cases = [
        ("fresh", ComposeOptions::new(), &component),
        ("component capture", captured(&component), &component),
        ("parent capture", captured(&app), &component),
        ("parent capture, parent request", captured(&app), &app),
    ];
    let canonical = |path: &Path| biscuit_file::canonicalize_simplified(path).unwrap();
    for (case, options, request_dir) in cases {
        let anchor = options.context().anchor().to_path_buf();
        let request = ComposeRequest::prepare(options.with_source_file(&note), &fixture.snapshot_at(request_dir)).unwrap();
        if request_dir == &component {
            let root = request.resolution_context().repository_root().map(canonical);
            assert_eq!(root, Some(canonical(&component)), "{case}: resolution root");
        }
        if case.starts_with("parent") {
            assert_eq!(request.options().context().anchor(), anchor.as_path(), "{case}: caller-owned capture");
        }
        let (composed, _) = document.compose_with(&request).unwrap_or_else(|error| panic!("{case}: {error}"));
        assert!(composed.content().contains("Component marker"), "{case}: {}", composed.content());
        assert!(composed.content().contains("# Review heading"), "{case}: {}", composed.content());
    }
}

/// The nested checkouts with a Cargo workspace (`lib`, `cli`) in each of the
/// launch checkout `app` and its component `app/vendor`, whose `lib` packages
/// are `app-lib` and `component-lib`, and one data file in each tree.
struct SlotCheckouts {
    checkouts: NestedCheckouts,
}

impl SlotCheckouts {
    fn new() -> Self {
        let checkouts = NestedCheckouts::new();
        for (root, prefix) in [(checkouts.app(), "app"), (checkouts.component(Component::Launch), "component")] {
            std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = [\"lib\", \"cli\"]\n").unwrap();
            for member in ["lib", "cli"] {
                std::fs::create_dir_all(root.join(member).join("src")).unwrap();
                std::fs::write(
                    root.join(member).join("Cargo.toml"),
                    format!("[package]\nname = \"{prefix}-{member}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
                )
                .unwrap();
                std::fs::write(root.join(member).join("src/lib.rs"), "").unwrap();
            }
            std::fs::create_dir_all(root.join("data")).unwrap();
            std::fs::write(root.join("data/states.yaml"), "- open\n- done\n").unwrap();
        }
        Self { checkouts }
    }

    fn app(&self) -> std::path::PathBuf {
        self.checkouts.app()
    }

    fn component(&self) -> std::path::PathBuf {
        self.checkouts.component(Component::Launch)
    }

    /// Writes `slot.md` in `dir`, whose inline `$schema` slot reads `target`
    /// (or calls `package('./lib')`, shown in the body, when `None`).
    fn note(&self, dir: &Path, target: Option<&Path>) -> std::path::PathBuf {
        let schema = match target {
            Some(target) => format!("  state: \"enum({{{{ load_data('{}', 'string[]') }}}})\"\nstate: open\n", target.display()),
            None => "  pkg: \"string(default({{ package('./lib') }}))\"\n".to_owned(),
        };
        let note = dir.join("slot.md");
        std::fs::write(&note, format!("---\n$schema:\n{schema}---\n\npkg=[{{{{ fm.pkg }}}}]\n")).unwrap();
        note
    }

    /// The four option cases: fresh, captured in the component, captured in
    /// the parent, and captured in the parent with the request (and note)
    /// in the parent; each with its request directory.
    fn cases(&self) -> [(&'static str, ComposeOptions, std::path::PathBuf); 4] {
        // A capture that names `ctx.cwd` is anchored where it was taken.
        let capture = Markdown::new("{{ ctx.cwd }}\n");
        let captured = |anchor: &Path| ComposeOptions::for_document(anchor, &capture);
        let (app, component) = (self.app(), self.component());
        [
            ("fresh", ComposeOptions::new(), component.clone()),
            ("component capture", captured(&component), component.clone()),
            ("parent capture", captured(&app), component),
            ("parent capture, parent request", captured(&app), app.clone()),
        ]
    }

    /// Composes `note` with `options` in a request at `dir`: the body, or
    /// the failure's diagnostic codes.
    fn compose(&self, options: ComposeOptions, dir: &Path, note: &Path) -> Result<String, Vec<String>> {
        let snapshot = darkmatter::markdown::compose::RequestSnapshot::new(dir).with_home(Some(self.checkouts.home()));
        let anchor = options.context().anchor().to_path_buf();
        let request = ComposeRequest::prepare(options.with_source_file(note), &snapshot).unwrap();
        // Caller-owned `ctx.*` values keep the anchor they were captured at.
        if anchor != Path::new("") {
            assert_eq!(request.options().context().anchor(), anchor.as_path(), "caller-owned capture");
        }
        Markdown::try_from(note)
            .unwrap()
            .compose_with(&request)
            .map(|(composed, _)| composed.content().to_owned())
            .map_err(|error| {
                darkmatter::markdown::compose::report_diagnostics::error_diagnostics(&error)
                    .into_iter()
                    .map(|diagnostic| diagnostic.code)
                    .collect()
            })
    }
}

/// A schema slot's `package()` describes the checkout the request resolves
/// in: a request in the component `app/vendor` names `component-lib` with
/// fresh options, options captured in the component, and options captured
/// in the parent `app`, whose observation describes the wider parent tree
/// (it would name `app-lib`). Control: a parent request names `app-lib`.
#[test]
fn a_schema_slot_reads_the_package_layout_of_the_request_checkout() {
    let fixture = SlotCheckouts::new();
    for (case, options, dir) in fixture.cases() {
        let note = fixture.note(&dir, None);
        let expected = if dir == fixture.app() { "pkg=[app-lib]" } else { "pkg=[component-lib]" };
        let composed = fixture.compose(options, &dir, &note).unwrap_or_else(|codes| panic!("{case}: {codes:?}"));
        assert!(composed.contains(expected), "{case}: {composed}");
    }
}

/// A schema slot's automatic read grant is the repository default of the
/// request's own checkout: a request in the component reads a file inside
/// it (not beside the note, so only the repository default allows it) and
/// is refused a file in the parent tree outside it, whatever checkout its
/// options were captured in. Control: a parent request reads the parent
/// file.
#[test]
fn a_schema_slot_read_grant_is_the_request_checkout() {
    let fixture = SlotCheckouts::new();
    let parent_file = fixture.app().join("data/states.yaml");
    let component_file = fixture.component().join("data/states.yaml");
    for (case, options, dir) in fixture.cases() {
        let parent_request = dir == fixture.app();
        let note = fixture.note(&dir, Some(&parent_file));
        let parent = fixture.compose(options.clone(), &dir, &note);
        if parent_request {
            assert!(parent.is_ok(), "{case}: {parent:?}");
            continue;
        }
        assert_eq!(parent, Err(vec!["dm.schema.slot_approval_required".to_owned()]), "{case}");
        let note = fixture.note(&dir, Some(&component_file));
        let inside = fixture.compose(options, &dir, &note);
        assert!(inside.is_ok(), "{case}: {inside:?}");
    }
}
