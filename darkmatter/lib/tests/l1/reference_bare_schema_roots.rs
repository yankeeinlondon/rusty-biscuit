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
    fn request(&self, parent: &Path, triggers: bool) -> ComposeRequest {
        let mut context = FileResolutionContext::from_snapshot(parent, Some(self.home()), std::collections::HashMap::new());
        if parent == self.repo() {
            context = context.with_repository_root(parent);
        }
        crate::request_support::request_in(
            ComposeOptions::new_with_context(ComposeContext::capture_for_content(parent, ""))
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
        let md = self.load(&parent.join(name));
        let request = self.request(parent, false);
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
        let (composed, report) = self
            .load(&parent.join(name))
            .compose_with(&self.request(parent, triggers))
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

/// R6-F1.6 control: the note checked directly refuses the bare name too.
#[test]
fn external_note_checked_directly_refuses_a_bare_schema() {
    let fixture = HomeNote::new();
    for spelling in External::ALL {
        let schema = spelling.reference(&fixture.home(), "review.yaml");
        fixture.write_schema_note(&schema, "{{ fm.description }}");
        let md = fixture.load(&fixture.home().join("review.md"));
        let graph_options = ReferenceGraphOptions::with_compose(&fixture.request(&fixture.repo(), false));
        let result = md.reference_graph(graph_options).map(|graph| graph.node_count());
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
    assert_compose_names(fixture.compose(&fixture.repo(), "inner.md", true), "version", "repository child");
    fixture
        .compose(&fixture.repo(), "inner.md", false)
        .expect("without trigger schemas the repository child composes");
}
