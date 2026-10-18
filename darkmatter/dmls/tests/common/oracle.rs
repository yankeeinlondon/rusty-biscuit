//! Test-only full-workspace oracle for the DMLS index store.
//!
//! The store narrows a request to a few candidate files and a small
//! `WorkspaceGraph` decides over them; its answers must equal those of one
//! graph built over **every** listed Markdown file. [`Oracle`] is that graph,
//! answered through the same public provider functions the router calls for
//! references, heading rename, `workspace/willRenameFiles`, and
//! `workspace/symbol`. Every answer is normalized to a sorted, comparable
//! value so a store test can `assert_eq!(store_answer, oracle_answer)`; the
//! `normalize_*` functions are public so the store side normalizes the same
//! way.
//!
//! The oracle owns its own sequential read and parse loop (`index_document`
//! per listed file) and does not use the production full-index build, which
//! the index store replaces.
//!
//! [`Corpus::materialize`] copies the static corpus under
//! `tests/fixtures/index_store_corpus/` into a fresh temporary directory and
//! adds what cannot be committed: the `.git` markers of its two repositories,
//! the absolute-path and `file://` referrer (`ws/docs/absolute.md`), and the
//! `ws/linked -> ws/docs` directory symlink. The workspace roots are `ws`,
//! the nested `ws/nested`, and the separate repository `other`.
//!
//! | Corpus file | Covers |
//! | --- | --- |
//! | `ws/docs/guide.md` | `./`, `../`, bare, repository-root-relative bare, `%20`, `<...>`, anchor, directory, case, NFD, and `/`-prefixed links |
//! | `ws/docs/absolute.md` (generated) | `file://` and absolute-path links |
//! | `ws/index.md` | bare links from the root, a root-level anchor link, a link into the nested root |
//! | `ws/notes/wiki.md` | extension-elided, nearest-match (`spec`), ambiguous (`topic`), heading, case, and NFD wiki links |
//! | `ws/docs/compose.md`, `ws/notes/also.md` | `::file` and `::code` transclusions |
//! | `ws/notes/symlinked.md` | a link through the `ws/linked` symlink |
//! | `ws/spec.md` / `ws/notes/spec.md`, `ws/alpha/topic.md` / `ws/beta/topic.md` | duplicate file names |
//! | `other/other.md` | a second root, and a wiki link into the first |

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::RequestSnapshot;
use dmls::capabilities::ClientProfile;
use dmls::config::ConfigState;
use dmls::context::RepositoryContexts;
use dmls::graph::{WorkspaceGraph, index_document};
use dmls::wiki::WikiUniverse;
use dmls::providers::rename::{RenameEnv, RenameError};
use dmls::providers::{DocumentContext, ProviderRegistry};
use dmls::source_map::{PositionEncoding, SourceMap};
use dmls::workspace::{
    DocumentStore, discover_workspace, file_path_to_uri, resolve_wiki_roots, uri_to_file_path,
};
use lsp_types::{
    DocumentChangeOperation, DocumentChanges, FileRename, InitializeParams, Location, NumberOrString, OneOf,
    Position, Range, SymbolKind, TextEdit, Uri, WorkspaceEdit, WorkspaceSymbol,
};

/// A location with its URI decoded to a path, ordered by path then range.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OracleLocation {
    pub path: PathBuf,
    /// `(line, character)` of the range start, zero-based.
    pub start: (u32, u32),
    /// `(line, character)` of the range end, zero-based.
    pub end: (u32, u32),
}

impl OracleLocation {
    /// The zero-width location at the start of zero-based `line`, the shape
    /// cross-document references take (`providers::location::line_range`).
    pub fn line(path: PathBuf, line: u32) -> Self {
        Self { path, start: (line, 0), end: (line, 0) }
    }
}

/// One text edit of a `WorkspaceEdit`, flattened and ordered.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OracleEdit {
    pub path: PathBuf,
    pub start: (u32, u32),
    pub end: (u32, u32),
    pub new_text: String,
}

/// One workspace symbol, ordered by location, then name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleSymbol {
    pub location: OracleLocation,
    pub name: String,
    pub kind: SymbolKind,
    pub container: Option<String>,
}

fn point(position: Position) -> (u32, u32) {
    (position.line, position.character)
}

fn path_of(uri: &Uri) -> PathBuf {
    uri_to_file_path(uri).unwrap_or_else(|| panic!("non-file URI in an answer: {}", uri.as_str()))
}

fn oracle_location(uri: &Uri, range: Range) -> OracleLocation {
    OracleLocation { path: path_of(uri), start: point(range.start), end: point(range.end) }
}

/// Sorted, path-decoded locations.
pub fn normalize_locations(locations: Vec<Location>) -> Vec<OracleLocation> {
    let mut out: Vec<OracleLocation> =
        locations.iter().map(|location| oracle_location(&location.uri, location.range)).collect();
    out.sort();
    out
}

/// Every text edit of `edit`, whichever of `changes` or `documentChanges`
/// carries them, sorted.
///
/// ## Panics
///
/// On a resource operation (create, rename, delete): no request the oracle
/// answers produces one, so its presence is a behavior change to notice.
pub fn normalize_workspace_edit(edit: &WorkspaceEdit) -> Vec<OracleEdit> {
    let mut out = Vec::new();
    let mut push = |uri: &Uri, edit: &TextEdit| {
        out.push(OracleEdit {
            path: path_of(uri),
            start: point(edit.range.start),
            end: point(edit.range.end),
            new_text: edit.new_text.clone(),
        });
    };
    if let Some(changes) = &edit.changes {
        for (uri, edits) in changes {
            edits.iter().for_each(|edit| push(uri, edit));
        }
    }
    let text_document_edits = match &edit.document_changes {
        None => Vec::new(),
        Some(DocumentChanges::Edits(edits)) => edits.iter().collect(),
        Some(DocumentChanges::Operations(operations)) => operations
            .iter()
            .map(|operation| match operation {
                DocumentChangeOperation::Edit(edit) => edit,
                DocumentChangeOperation::Op(op) => panic!("unexpected resource operation: {op:?}"),
            })
            .collect(),
    };
    for document in text_document_edits {
        for edit in &document.edits {
            let edit = match edit {
                OneOf::Left(edit) => edit,
                OneOf::Right(annotated) => &annotated.text_edit,
            };
            push(&document.text_document.uri, edit);
        }
    }
    out.sort();
    out
}

/// Sorted workspace symbols.
///
/// ## Panics
///
/// On a symbol located by URI only (`WorkspaceLocation`), which DMLS never
/// answers.
pub fn normalize_symbols(symbols: Vec<WorkspaceSymbol>) -> Vec<OracleSymbol> {
    let mut out: Vec<OracleSymbol> = symbols
        .into_iter()
        .map(|symbol| {
            let location = match &symbol.location {
                OneOf::Left(location) => oracle_location(&location.uri, location.range),
                OneOf::Right(location) => panic!("URI-only symbol location: {}", location.uri.as_str()),
            };
            OracleSymbol {
                location,
                name: symbol.name,
                kind: symbol.kind,
                container: symbol.container_name,
            }
        })
        .collect();
    out.sort_by(|left, right| {
        (&left.location, &left.name, &left.container)
            .cmp(&(&right.location, &right.name, &right.container))
            .then_with(|| symbol_kind_number(left.kind).cmp(&symbol_kind_number(right.kind)))
    });
    out
}

fn symbol_kind_number(kind: SymbolKind) -> i64 {
    serde_json::to_value(kind).ok().and_then(|value| value.as_i64()).unwrap_or_default()
}

/// One graph over every listed Markdown file of the workspace `roots`, built
/// the way the server builds its full index: the same discovery, config,
/// wiki roots, and per-repository contexts.
pub struct Oracle {
    roots: Vec<PathBuf>,
    config: ConfigState,
    profile: ClientProfile,
    contexts: RepositoryContexts,
    documents: DocumentStore,
    registry: ProviderRegistry,
    texts: BTreeMap<PathBuf, String>,
    graph: WorkspaceGraph,
}

/// Which files a [scoped](Oracle::scoped) graph matches wiki links against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniverseScope {
    /// Every listed file, as the index store supplies it.
    FileList,
    /// Only the scoped documents: what a graph derived for itself before the
    /// universe was external. A negative control.
    Documents,
}

impl Oracle {
    /// Lists, reads, and parses every Markdown file under `roots`, then
    /// builds the graph once.
    ///
    /// `snapshot` supplies the home and environment of every context, as the
    /// server's `RunOptions` snapshot does.
    ///
    /// ## Panics
    ///
    /// When the file list is incomplete or a listed file cannot be read: an
    /// oracle over part of the corpus would answer wrongly.
    pub fn build(roots: &[PathBuf], snapshot: RequestSnapshot) -> Self {
        Self::build_over(roots, snapshot, None, UniverseScope::FileList)
    }

    /// A request-scoped graph over only the listed files in `documents`,
    /// matching wiki links against `universe`, answered exactly like the
    /// oracle. With [`UniverseScope::FileList`] its answers must equal the
    /// full oracle's for any request whose referrers are all in `documents`.
    ///
    /// ## Panics
    ///
    /// As [`Oracle::build`], and when a path in `documents` is not listed.
    pub fn scoped(
        roots: &[PathBuf],
        snapshot: RequestSnapshot,
        documents: &[PathBuf],
        universe: UniverseScope,
    ) -> Self {
        Self::build_over(roots, snapshot, Some(documents), universe)
    }

    fn build_over(
        roots: &[PathBuf],
        snapshot: RequestSnapshot,
        documents: Option<&[PathBuf]>,
        scope: UniverseScope,
    ) -> Self {
        let config = ConfigState::load(roots.to_vec(), None);
        let report = discover_workspace(roots, &config.effective().workspace);
        assert!(report.is_complete(), "oracle file list incomplete: {:?}", report.errors);
        if let Some(documents) = documents {
            for path in documents {
                assert!(report.files.contains(path), "{} is not listed", path.display());
            }
        }
        let mut texts = BTreeMap::new();
        let mut indices = BTreeMap::new();
        for path in &report.files {
            if documents.is_some_and(|documents| !documents.contains(path)) {
                continue;
            }
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            indices.insert(path.clone(), index_document(path, &text));
            texts.insert(path.clone(), text);
        }
        let wiki_roots = resolve_wiki_roots(roots, config.effective().wiki.wiki_root.as_deref());
        let contexts = RepositoryContexts::new(snapshot);
        // The full oracle's universe is its whole file list: the reference
        // every request-scoped graph's answers must equal.
        let universe = match scope {
            UniverseScope::FileList => WikiUniverse::new(report.files.iter().cloned(), &wiki_roots),
            UniverseScope::Documents => WikiUniverse::new(indices.keys().cloned(), &wiki_roots),
        };
        let graph = WorkspaceGraph::build_with_roots(&indices, 1, universe, &contexts);
        Self {
            roots: roots.to_vec(),
            config,
            profile: ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16),
            contexts,
            documents: DocumentStore::new(PositionEncoding::Utf16),
            registry: ProviderRegistry::with_substrate(),
            texts,
            graph,
        }
    }

    /// The full graph.
    pub fn graph(&self) -> &WorkspaceGraph {
        &self.graph
    }

    /// Every listed Markdown file, sorted.
    pub fn files(&self) -> Vec<PathBuf> {
        self.texts.keys().cloned().collect()
    }

    /// The position of the first occurrence of `needle` in the listed file
    /// `path`.
    ///
    /// ## Panics
    ///
    /// When `path` is not listed or does not contain `needle`.
    pub fn position_of(&self, path: &Path, needle: &str) -> Position {
        let text = self.text(path);
        let offset = text
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} not found in {}", path.display()));
        self.source_map(path, text).byte_to_lsp(offset).expect("offset inside the text")
    }

    /// `textDocument/references` at `position` in the listed file `path`, as
    /// if `path` were open with its disk text: heading backlinks, link
    /// targets, wiki backlinks, and DSL transcluded-by.
    pub fn references(&self, path: &Path, position: Position, include_declaration: bool) -> Vec<OracleLocation> {
        let text = self.text(path);
        let uri = file_path_to_uri(path).expect("absolute document path");
        let source_map = self.source_map(path, text);
        let resolution = self.contexts.for_document(path);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &self.graph,
            doc_id: self.graph.document_id(path),
            config: self.config.effective(),
            profile: &self.profile,
            // No references provider reads the frontmatter overlay, and
            // assembling one would prepare schemas the oracle does not judge.
            overlay: None,
            resolution: &resolution,
        };
        let offset = source_map.lsp_to_byte(position).expect("position inside the document");
        normalize_locations(self.registry.references(&ctx, offset, include_declaration))
    }

    /// `textDocument/rename` of the heading at `position` in `path`.
    ///
    /// ## Errors
    ///
    /// The provider's [`RenameError`] (not on a heading, or ambiguous).
    pub fn rename(&self, path: &Path, position: Position, new_name: &str) -> Result<Vec<OracleEdit>, RenameError> {
        let uri = file_path_to_uri(path).expect("absolute document path");
        dmls::providers::rename::rename(&self.rename_env(), &uri, position, new_name)
            .map(|edit| normalize_workspace_edit(&edit))
    }

    /// `workspace/willRenameFiles` for `(old, new)` path pairs; `None` when
    /// the provider declines (nothing to edit, or an unsafe rename).
    pub fn will_rename_files(&self, renames: &[(PathBuf, PathBuf)]) -> Option<Vec<OracleEdit>> {
        let files: Vec<FileRename> = renames
            .iter()
            .map(|(old, new)| FileRename {
                old_uri: file_path_to_uri(old).expect("absolute old path").as_str().to_string(),
                new_uri: file_path_to_uri(new).expect("absolute new path").as_str().to_string(),
            })
            .collect();
        dmls::providers::rename::will_rename_files(&self.rename_env(), &files)
            .map(|edit| normalize_workspace_edit(&edit))
    }

    /// The wiki diagnostics of the listed file `path` as `(line, code,
    /// message)`, sorted. Related information is left out: it lists the
    /// ambiguous candidates that are documents of the graph, so a scoped
    /// graph lists fewer of them by design.
    pub fn wiki_diagnostics(&self, path: &Path) -> Vec<(u32, String, String)> {
        let text = self.text(path);
        let uri = file_path_to_uri(path).expect("absolute document path");
        let source_map = self.source_map(path, text);
        let resolution = self.contexts.for_document(path);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &self.graph,
            doc_id: self.graph.document_id(path),
            config: self.config.effective(),
            profile: &self.profile,
            overlay: None,
            resolution: &resolution,
        };
        let mut out: Vec<(u32, String, String)> = dmls::providers::wiki::diagnostics(&ctx)
            .into_iter()
            .map(|diagnostic| {
                let code = match diagnostic.code {
                    Some(NumberOrString::String(code)) => code,
                    Some(NumberOrString::Number(code)) => code.to_string(),
                    None => String::new(),
                };
                (diagnostic.range.start.line, code, diagnostic.message)
            })
            .collect();
        out.sort();
        out
    }

    /// `workspace/symbol` for `query` (after the registry's cap).
    pub fn workspace_symbols(&self, query: &str) -> Vec<OracleSymbol> {
        normalize_symbols(self.registry.workspace_symbols(query, &self.graph))
    }

    fn rename_env(&self) -> RenameEnv<'_> {
        RenameEnv {
            documents: &self.documents,
            graph: &self.graph,
            config: self.config.effective(),
            profile: &self.profile,
            roots: &self.roots,
            encoding: PositionEncoding::Utf16,
        }
    }

    fn text(&self, path: &Path) -> &str {
        self.texts
            .get(path)
            .unwrap_or_else(|| panic!("{} is not a listed document", path.display()))
    }

    fn source_map(&self, path: &Path, text: &str) -> SourceMap {
        let uri = file_path_to_uri(path).expect("absolute document path");
        SourceMap::new(uri, 1, PositionEncoding::Utf16, Arc::from(text))
    }
}

/// The corpus copied into its own temporary directory.
pub struct Corpus {
    _dir: tempfile::TempDir,
    /// The canonical temporary directory holding `ws`, `other`, and `home`.
    pub base: PathBuf,
    /// The main repository and first workspace root.
    pub ws: PathBuf,
    /// A workspace root inside `ws`.
    pub nested: PathBuf,
    /// The second repository and workspace root.
    pub other: PathBuf,
    /// Whether `ws/linked -> ws/docs` was created. Always on Unix; on
    /// Windows creating a directory symlink needs a privilege a host may
    /// lack.
    pub symlink_created: bool,
}

impl Corpus {
    /// Copies the static corpus at `source` and completes it (see the module
    /// docs).
    pub fn materialize(source: &Path) -> Self {
        let dir = tempfile::tempdir().expect("create corpus temp dir");
        // Canonical, so a document path and a resolved target agree on
        // aliased temporary directories (`/var` -> `/private/var` on macOS).
        let base = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        copy_tree(source, &base);
        let ws = base.join("ws");
        let other = base.join("other");
        for repository in [&ws, &other] {
            init_repository(repository);
        }
        std::fs::create_dir_all(base.join("home")).expect("create fixture home");
        let target = ws.join("docs/a.md");
        let url = url::Url::from_file_path(&target).expect("file URL");
        let absolute = target.to_string_lossy().replace('\\', "/");
        write(
            &ws.join("docs/absolute.md"),
            &format!("# Absolute Forms\n\n- [file url]({url})\n- [absolute]({absolute})\n"),
        );
        let symlink_created = symlink_dir(&ws.join("docs"), &ws.join("linked"));
        #[cfg(unix)]
        assert!(symlink_created, "a directory symlink is always creatable on Unix");
        Self { nested: ws.join("nested"), ws, other, base, symlink_created, _dir: dir }
    }

    /// The workspace roots: `ws`, the nested `ws/nested`, and `other`.
    pub fn roots(&self) -> Vec<PathBuf> {
        vec![self.ws.clone(), self.nested.clone(), self.other.clone()]
    }

    /// The fixture request snapshot: the corpus as request directory and a
    /// fixture home, so no context reads the developer's home.
    pub fn snapshot(&self) -> RequestSnapshot {
        RequestSnapshot::new(&self.base).with_home(Some(self.base.join("home")))
    }

    /// An oracle over [`Corpus::roots`].
    pub fn oracle(&self) -> Oracle {
        Oracle::build(&self.roots(), self.snapshot())
    }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create corpus directory");
    for entry in std::fs::read_dir(from).expect("read corpus directory") {
        let entry = entry.expect("corpus entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("corpus entry type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy corpus file");
        }
    }
}

/// The minimal `.git` a context recognizes as a repository.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).expect("create .git/objects");
    std::fs::create_dir_all(root.join(".git/refs/heads")).expect("create .git/refs/heads");
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    std::fs::write(path, text).expect("write corpus file");
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_dir(target, link).is_ok()
}
