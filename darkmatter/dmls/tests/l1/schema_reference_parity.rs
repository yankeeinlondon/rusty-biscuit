//! Validation, document links, and go-to-definition select the same `$schema` file.
//!
//! One fixture holds every selection shape a `$schema` string can take: a
//! bare name found in a later schema root (the `review-3.md` regression), a
//! clean miss, a document with no schema root folder at all, `SCHEMAS_DIR`
//! shadowing a package copy, a sibling-only bare name, an explicit relative
//! miss, a root-union string item, a malformed winning file, and folder
//! names with spaces, Markdown punctuation, and non-ASCII. Each case is
//! driven in-process through the provider registry as an editor request is,
//! and compared with what the validator's resolver selects from the same
//! schema roots. `frontmatter_response_contracts.rs` pins the same cases'
//! responses as a golden.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::schemas::SchemaRoots;
use darkmatter::markdown::schemas::resolve::resolve_schema_with_roots;
use dmls::capabilities::ClientProfile;
use dmls::config::DmlsConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{NoContexts, WorkspaceGraph};
use dmls::overlay::OverlayState;
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use lsp_types::{Diagnostic, DocumentLink, InitializeParams, Location, Position, Range as LspRange, Uri};
use simplified_schema::SchemaError;
use simplified_schema::resolve::{SelectionOutcome, select_schema_file};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create directory");
    std::fs::write(path, text).expect("write file");
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).expect("objects");
    std::fs::create_dir_all(root.join(".git/refs/heads")).expect("refs");
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

/// The folder of the documents with Markdown link punctuation, a space, and
/// non-ASCII in its name. No golden target lies inside it; a target in a
/// bracketed folder is
/// `a_target_in_a_bracketed_folder_has_a_link_and_definition`'s subject.
pub(crate) const UNUSUAL: &str = "[x] (y) café";

/// The spaced, parenthesized, non-ASCII folder name the `SCHEMAS_DIR` folder
/// and the explicit miss's folder carry, so their target URIs need
/// percent-encoding.
pub(crate) const SPACED: &str = "(y) café";

/// The file a request's link and definition target, and what the validator
/// selects for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Expect {
    /// The file exists and validates; the validator selects it.
    Found,
    /// The file exists but does not prepare; the validator's error names it.
    Malformed,
    /// Nothing exists; the target is where the file would be created, which
    /// following the link must not create.
    Creation,
}

/// One document and the target its `$schema` string must have.
pub(crate) struct Case {
    pub(crate) label: &'static str,
    pub(crate) document: PathBuf,
    pub(crate) text: String,
    /// The authored `$schema` string the link and definition range.
    pub(crate) reference: &'static str,
    /// Whether the session was launched with `SCHEMAS_DIR` set.
    pub(crate) schemas_dir: bool,
    pub(crate) target: PathBuf,
    pub(crate) expect: Expect,
}

/// What the server answered for one case, with the inputs the validator is
/// given for the same document.
pub(crate) struct Responses {
    pub(crate) links: Vec<DocumentLink>,
    pub(crate) definition: Vec<Location>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// The byte span of the authored reference.
    pub(crate) value_span: Range<usize>,
    pub(crate) context: FileResolutionContext,
    pub(crate) search_paths: Vec<PathBuf>,
}

/// The fixture's layout, under one canonical temporary root:
///
/// | path | contents |
/// |---|---|
/// | `repo/` | Git repository; workspace members `messenger/lib`, `area/pkg` |
/// | `repo/schemas/` | tree root: `feature-review.yaml`, malformed `broken.yaml` |
/// | `repo/messenger/schemas/` | package-area root, without `feature-review.yaml` |
/// | `repo/messenger/features/x/` | the documents, and `beside.yaml` |
/// | `repo/messenger/features/{UNUSUAL}/` | a document in a bracketed folder |
/// | `repo/messenger/features/notes {SPACED}/` | the explicit non-ASCII miss |
/// | `repo/area/pkg/schemas/shared.yaml` | the package copy `SCHEMAS_DIR` shadows |
/// | `dm defs {SPACED}/shared.yaml` | the `SCHEMAS_DIR` folder |
/// | `bare/` | a second repository with no `schemas/` folder anywhere |
/// | `home/` | an empty home, so `~/schemas` is absent |
pub(crate) struct NavigationFixture {
    _dir: tempfile::TempDir,
    pub(crate) root: PathBuf,
    plain: RepositoryContexts,
    with_schemas_dir: RepositoryContexts,
    config: DmlsConfig,
    registry: ProviderRegistry,
    graph: WorkspaceGraph,
    profile: ClientProfile,
}

impl NavigationFixture {
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let repo = root.join("repo");
        init_repository(&repo);
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"messenger/lib\", \"area/pkg\"]\n");
        for (folder, name) in [("messenger/lib", "messenger"), ("area/pkg", "pkg")] {
            write(
                &repo.join(folder).join("Cargo.toml"),
                &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
            );
            write(&repo.join(folder).join("src/lib.rs"), "");
        }
        write(&repo.join("schemas/feature-review.yaml"), "$schema:\n  title: string\n");
        write(&repo.join("schemas/broken.yaml"), "$schema:\n  title: string(min(\n");
        write(&repo.join("messenger/schemas/area-only.yaml"), "$schema:\n  area: string\n");
        write(&repo.join("messenger/features/x/beside.yaml"), "$schema:\n  beside: string\n");
        write(&repo.join("area/pkg/schemas/shared.yaml"), "$schema:\n  from_package: string\n");
        let defs = root.join(format!("dm defs {SPACED}"));
        write(&defs.join("shared.yaml"), "$schema:\n  from_defs: string\n");
        init_repository(&root.join("bare"));
        std::fs::create_dir_all(root.join("home")).expect("home");

        let snapshot = RequestSnapshot::new(&root).with_home(Some(root.join("home")));
        let with_defs = snapshot
            .clone()
            .with_env(HashMap::from([("SCHEMAS_DIR".to_string(), defs.to_string_lossy().into_owned())]));
        Self {
            _dir: dir,
            plain: RepositoryContexts::new(snapshot),
            with_schemas_dir: RepositoryContexts::new(with_defs),
            config: DmlsConfig::default(),
            registry: ProviderRegistry::with_substrate(),
            graph: WorkspaceGraph::build(&BTreeMap::new(), 1, &NoContexts),
            profile: ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16),
            root,
        }
    }

    pub(crate) fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    pub(crate) fn defs(&self) -> PathBuf {
        self.root.join(format!("dm defs {SPACED}"))
    }

    /// Every case, in a fixed order; the documents are written by
    /// [`NavigationFixture::request`].
    pub(crate) fn cases(&self) -> Vec<Case> {
        let repo = self.repo();
        let x = repo.join("messenger/features/x");
        let unusual = repo.join("messenger/features").join(UNUSUAL);
        let spaced = repo.join("messenger/features").join(format!("notes {SPACED}"));
        let scalar = |reference: &str| format!("---\n$schema: {reference}\ntitle: Review\n---\n\nBody\n");
        vec![
            Case {
                label: "bare name found in the tree root (review-3 regression)",
                document: x.join("review-3.md"),
                text: scalar("feature-review.yaml"),
                reference: "feature-review.yaml",
                schemas_dir: false,
                target: repo.join("schemas/feature-review.yaml"),
                expect: Expect::Found,
            },
            Case {
                label: "clean-miss bare name links into the first searchable root",
                document: x.join("clean-miss.md"),
                text: scalar("absent.yaml"),
                reference: "absent.yaml",
                schemas_dir: false,
                target: repo.join("messenger/schemas/absent.yaml"),
                expect: Expect::Creation,
            },
            Case {
                label: "bare name with no schema root folder links beside the document",
                document: self.root.join("bare/docs/no-root.md"),
                text: scalar("nowhere.yaml"),
                reference: "nowhere.yaml",
                schemas_dir: false,
                target: self.root.join("bare/docs/nowhere.yaml"),
                expect: Expect::Creation,
            },
            Case {
                label: "package copy without SCHEMAS_DIR",
                document: repo.join("area/pkg/docs/defs.md"),
                text: "---\n$schema: shared.yaml\n---\n".to_string(),
                reference: "shared.yaml",
                schemas_dir: false,
                target: repo.join("area/pkg/schemas/shared.yaml"),
                expect: Expect::Found,
            },
            Case {
                label: "SCHEMAS_DIR wins over the package copy",
                document: repo.join("area/pkg/docs/defs.md"),
                text: "---\n$schema: shared.yaml\n---\n".to_string(),
                reference: "shared.yaml",
                schemas_dir: true,
                target: self.defs().join("shared.yaml"),
                expect: Expect::Found,
            },
            Case {
                label: "sibling-only bare name links to the first-root creation path",
                document: x.join("sibling.md"),
                text: scalar("beside.yaml"),
                reference: "beside.yaml",
                schemas_dir: false,
                target: repo.join("messenger/schemas/beside.yaml"),
                expect: Expect::Creation,
            },
            Case {
                label: "explicit relative miss links beside the document",
                document: x.join("explicit.md"),
                text: scalar("./missing.yaml"),
                reference: "./missing.yaml",
                schemas_dir: false,
                target: x.join("missing.yaml"),
                expect: Expect::Creation,
            },
            Case {
                label: "root-union string item",
                document: x.join("union.md"),
                text: "---\n$schema: [feature-review.yaml, {status: string}]\ntitle: Review\n---\n".to_string(),
                reference: "feature-review.yaml",
                schemas_dir: false,
                target: repo.join("schemas/feature-review.yaml"),
                expect: Expect::Found,
            },
            Case {
                label: "malformed winning file still opens",
                document: x.join("malformed.md"),
                text: scalar("broken.yaml"),
                reference: "broken.yaml",
                schemas_dir: false,
                target: repo.join("schemas/broken.yaml"),
                expect: Expect::Malformed,
            },
            Case {
                label: "bare name from a punctuated non-ASCII folder",
                document: unusual.join("review.md"),
                text: scalar("feature-review.yaml"),
                reference: "feature-review.yaml",
                schemas_dir: false,
                target: repo.join("schemas/feature-review.yaml"),
                expect: Expect::Found,
            },
            Case {
                label: "explicit non-ASCII miss in a spaced non-ASCII folder",
                document: spaced.join("notes.md"),
                text: scalar("./café.yaml"),
                reference: "./café.yaml",
                schemas_dir: false,
                target: spaced.join("café.yaml"),
                expect: Expect::Creation,
            },
        ]
    }

    /// Writes `case`'s document and answers document links, the definition
    /// inside its reference, and diagnostics, through a fresh overlay cache.
    pub(crate) fn request(&self, case: &Case) -> Responses {
        write(&case.document, &case.text);
        let contexts = if case.schemas_dir { &self.with_schemas_dir } else { &self.plain };
        let resolution = contexts.for_document(&case.document);
        let context = resolution.context().expect("document context").clone();
        let search_paths = SchemaRoots::for_document(&context).expect("schema roots").search_paths().to_vec();
        let uri = uri_of(&case.document);
        let workspace = [self.repo()];
        let overlays = OverlayState::default();
        let overlay =
            overlays.for_document(&uri, &case.text, &case.document, &self.config, &workspace, &resolution);
        assert!(overlay.is_some(), "{}: the document has a frontmatter overlay", case.label);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(case.text.as_str()));
        let ctx = DocumentContext {
            uri: &uri,
            path: &case.document,
            text: &case.text,
            source_map: &source_map,
            graph: &self.graph,
            doc_id: None,
            config: &self.config,
            profile: &self.profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
        };
        let start = case
            .text
            .find(case.reference)
            .unwrap_or_else(|| panic!("{}: the reference is authored", case.label));
        let value_span = start..start + case.reference.len();
        Responses {
            links: self.registry.document_links(&ctx),
            definition: self.registry.definition(&ctx, start + 1),
            diagnostics: self.registry.diagnostics(&ctx),
            value_span,
            context,
            search_paths,
        }
    }
}

/// `path` as the `file` URI an editor sends: `url` leaves `[` and `]` raw in
/// a path, which an LSP `Uri` rejects, so they are percent-encoded.
pub(crate) fn uri_of(path: &Path) -> Uri {
    let url = url::Url::from_file_path(path).expect("file URL");
    url.as_str().replace('[', "%5B").replace(']', "%5D").parse().expect("URI")
}

/// The LSP (UTF-16) range of `span` in `text`, computed independently of the
/// server's source map.
pub(crate) fn utf16_range(text: &str, span: &Range<usize>) -> LspRange {
    let position = |offset: usize| {
        let before = &text[..offset];
        let line = before.matches('\n').count() as u32;
        let line_start = before.rfind('\n').map_or(0, |at| at + 1);
        Position::new(line, text[line_start..offset].encode_utf16().count() as u32)
    };
    LspRange::new(position(span.start), position(span.end))
}

/// The file the validator selects for `case`, from the same `$schema` value,
/// document folder, schema roots, and context the overlay is given: the one
/// referenced file of a schema that resolves, or the file a referenced-file
/// error names. `None` for a selection that finds no file.
fn validator_file(case: &Case, responses: &Responses) -> Option<PathBuf> {
    let yaml = case.text.split("---\n").nth(1).expect("a frontmatter block");
    let frontmatter: serde_json::Value = serde_yaml_ng::from_str(yaml).expect("frontmatter YAML");
    let value = &frontmatter["$schema"];
    let base_dir = case.document.parent().expect("document folder");
    match resolve_schema_with_roots(value, base_dir, &responses.search_paths, &responses.context) {
        Ok(resolved) => {
            let [file] = resolved.referenced_files.as_slice() else {
                panic!("{}: one referenced file, got {:?}", case.label, resolved.referenced_files);
            };
            Some(file.clone())
        }
        Err(error) => error_file(&error),
    }
}

fn error_file(error: &SchemaError) -> Option<PathBuf> {
    match error {
        SchemaError::ReferencedSchema { path, .. }
        | SchemaError::NotASchemaDocument { path, .. }
        | SchemaError::SchemaDocument { path, .. }
        | SchemaError::InvalidJsonSchema { path, .. }
        | SchemaError::UnsupportedJsonDialect { path, .. } => Some(path.clone()),
        SchemaError::Aggregate { errors } => errors.iter().find_map(error_file),
        _ => None,
    }
}

fn canonical(path: &Path) -> PathBuf {
    canonicalize_simplified(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn links_definition_and_validation_select_the_same_schema_file() {
    let fixture = NavigationFixture::new();
    for case in fixture.cases() {
        let label = case.label;
        let responses = fixture.request(&case);
        let range = utf16_range(&case.text, &responses.value_span);
        let target = uri_of(&case.target);

        // One link and one definition, both on the authored string and both
        // naming the expected file.
        let links: Vec<(&LspRange, Option<&Uri>)> =
            responses.links.iter().map(|link| (&link.range, link.target.as_ref())).collect();
        assert_eq!(links, [(&range, Some(&target))], "{label}: document links");
        let definition: Vec<(&Uri, &LspRange)> =
            responses.definition.iter().map(|location| (&location.uri, &location.range)).collect();
        assert_eq!(definition.len(), 1, "{label}: definition {definition:?}");
        assert_eq!(definition[0].0, &target, "{label}: definition target");

        // The selection the overlay reads comes from the validator's roots.
        let selection = select_schema_file(
            case.reference,
            case.document.parent().expect("folder"),
            &responses.search_paths,
            &responses.context,
        );
        let validator = validator_file(&case, &responses);
        match case.expect {
            Expect::Found => {
                assert!(matches!(&selection.outcome, SelectionOutcome::Found(path) if *path == case.target), "{label}: {selection:?}");
                let validator = validator.unwrap_or_else(|| panic!("{label}: the validator selects a file"));
                assert_eq!(canonical(&validator), canonical(&case.target), "{label}: validator file");
            }
            Expect::Malformed => {
                let validator = validator.unwrap_or_else(|| panic!("{label}: the validator's error names the file"));
                assert_eq!(canonical(&validator), canonical(&case.target), "{label}: validator file");
                // The diagnostic's related information opens the same file.
                let related: Vec<&Uri> = responses
                    .diagnostics
                    .iter()
                    .flat_map(|diagnostic| diagnostic.related_information.iter().flatten())
                    .map(|related| &related.location.uri)
                    .collect();
                assert!(!related.is_empty(), "{label}: {:?}", responses.diagnostics);
                assert!(related.iter().all(|uri| **uri == target), "{label}: related {related:?}");
            }
            Expect::Creation => {
                assert_eq!(validator, None, "{label}: the validator selects no file");
                let creation = match &selection.outcome {
                    SelectionOutcome::Missing { creation: Some(path), .. }
                    | SelectionOutcome::SiblingOnly { creation: path, .. } => path,
                    other => panic!("{label}: {other:?}"),
                };
                assert_eq!(creation, &case.target, "{label}: creation path");
                assert!(!case.target.exists(), "{label}: requesting links created {}", case.target.display());
                assert!(
                    case.target.parent().is_some_and(Path::is_dir),
                    "{label}: the creation folder is a fixture folder"
                );
            }
        }
    }
}

#[test]
fn the_fixture_has_the_schema_roots_each_case_relies_on() {
    let fixture = NavigationFixture::new();
    let repo = fixture.repo();
    let cases = fixture.cases();
    let roots_of = |label: &str| {
        let case = cases.iter().find(|case| case.label.starts_with(label)).expect("case");
        fixture.request(case).search_paths
    };
    assert_eq!(
        roots_of("bare name found in the tree root"),
        [repo.join("messenger/schemas"), repo.join("schemas")],
        "the review document searches its package area before the tree root"
    );
    assert_eq!(roots_of("bare name with no schema root folder"), Vec::<PathBuf>::new());
    assert_eq!(
        roots_of("SCHEMAS_DIR wins"),
        [fixture.defs(), repo.join("area/pkg/schemas"), repo.join("schemas")],
    );
}

/// A link or definition target inside a folder whose name has `[` or `]`
/// still gets a URI (`url::Url::from_file_path` leaves the brackets raw,
/// which `lsp_types::Uri` rejects, so `file_path_to_uri` percent-encodes
/// them). The validator selects the creation path here as for any explicit
/// miss.
#[test]
fn a_target_in_a_bracketed_folder_has_a_link_and_definition() {
    let fixture = NavigationFixture::new();
    let folder = fixture.repo().join("messenger/features").join(UNUSUAL);
    let case = Case {
        label: "explicit miss in a bracketed folder",
        document: folder.join("bracketed.md"),
        text: "---\n$schema: ./café.yaml\n---\n".to_string(),
        reference: "./café.yaml",
        schemas_dir: false,
        target: folder.join("café.yaml"),
        expect: Expect::Creation,
    };
    let responses = fixture.request(&case);
    let selection = select_schema_file(
        case.reference,
        &folder,
        &responses.search_paths,
        &responses.context,
    );
    assert!(
        matches!(&selection.outcome, SelectionOutcome::Missing { creation: Some(path), .. } if *path == case.target),
        "{selection:?}"
    );
    let target = uri_of(&case.target);
    let links: Vec<Option<&Uri>> = responses.links.iter().map(|link| link.target.as_ref()).collect();
    assert_eq!(links, [Some(&target)], "document links");
    let definition: Vec<&Uri> = responses.definition.iter().map(|location| &location.uri).collect();
    assert_eq!(definition, [&target], "definition");
    assert!(!case.target.exists(), "requesting links created {}", case.target.display());
}
