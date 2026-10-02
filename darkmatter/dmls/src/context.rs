//! One file-resolution context per repository.
//!
//! Every DMLS feature that resolves a file reference (schema validation, the
//! link graph, document links, go-to-definition, hover, code actions, anchor
//! completion) resolves through the context [`RepositoryContexts`] hands out
//! for the document. The context is built once per repository by
//! Darkmatter's `build_resolution_context`, with the repository root as the
//! request directory, so an editor answers `&`, `^`, and `@` the way
//! `md compose` run from that root does. A document in no repository gets a
//! context for its own folder.
//!
//! A failed build is cached like a success and reported as one diagnostic;
//! nothing falls back to a lexical join. Entries are dropped on a watched or
//! rescan-detected change below their key, or all at once on a configuration
//! change, and rebuilt lazily on the next request. `HOME` and the
//! environment come from the [`RequestSnapshot`] taken at startup and never
//! change for the server's lifetime.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_file::{CandidatePlanOrder, FileReference, FileResolutionContext, ResolutionFailure};
use darkmatter::markdown::compose::{ContextBuildError, RequestSnapshot, build_resolution_context};

use crate::graph::DocumentContexts;

/// Context builds performed by every [`RepositoryContexts`] in this process.
static CONTEXT_BUILDS: AtomicUsize = AtomicUsize::new(0);

/// The number of file-resolution contexts DMLS has built in this process.
///
/// A work counter: tests read a before/after delta to prove that documents
/// in one repository share one build and that invalidation rebuilds.
pub fn context_build_count() -> usize {
    CONTEXT_BUILDS.load(Ordering::SeqCst)
}

/// Why a document has no file-resolution context.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ContextFailure {
    /// The builder rejected the document's repository or folder.
    #[error(transparent)]
    Build(Arc<ContextBuildError>),
    /// An untitled buffer needs exactly one repository across the workspace
    /// folders to borrow a context from.
    #[error(
        "an untitled buffer resolves file references only when the workspace folders lie in \
         exactly one repository; found {repositories} across {}",
        folder_list(folders)
    )]
    UntitledWorkspace {
        /// Distinct repositories containing a workspace folder.
        repositories: usize,
        /// The workspace folders that were counted.
        folders: Vec<PathBuf>,
    },
}

impl ContextFailure {
    /// The failure class, as file-reference errors report it.
    ///
    /// An untitled buffer with no single repository is
    /// [`ResolutionFailure::MissingContext`]: the repository anchor is absent.
    pub fn resolution_failure(&self) -> ResolutionFailure {
        match self {
            Self::Build(error) => error.resolution_failure(),
            Self::UntitledWorkspace { .. } => ResolutionFailure::MissingContext,
        }
    }
}

fn folder_list(folders: &[PathBuf]) -> String {
    if folders.is_empty() {
        return "no workspace folders".to_string();
    }
    folders
        .iter()
        .map(|folder| format!("`{}`", folder.display()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A document's context, or why it has none.
#[derive(Debug, Clone)]
pub struct DocumentResolution {
    generation: u64,
    outcome: Result<FileResolutionContext, ContextFailure>,
}

impl DocumentResolution {
    /// The derived context, when the build succeeded.
    pub fn context(&self) -> Option<&FileResolutionContext> {
        self.outcome.as_ref().ok()
    }

    /// The cached build failure, when there is one.
    pub fn failure(&self) -> Option<&ContextFailure> {
        self.outcome.as_ref().err()
    }

    /// Identifies the cached build this resolution came from. A rebuilt
    /// context has a new generation, so caches keyed on it re-derive.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// A resolution with a fixed context, for callers and tests that hold a
    /// context already.
    pub fn from_context(context: FileResolutionContext) -> Self {
        Self { generation: 0, outcome: Ok(context) }
    }

    /// A resolution that failed, for callers and tests that hold a failure.
    pub fn from_failure(failure: ContextFailure) -> Self {
        Self { generation: 0, outcome: Err(failure) }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    generation: u64,
    built: Result<Arc<FileResolutionContext>, Arc<ContextBuildError>>,
}

/// Where a folder's context is keyed.
#[derive(Debug, Clone)]
struct FolderKey {
    key: PathBuf,
    repository: bool,
}

#[derive(Debug, Default)]
struct State {
    entries: HashMap<PathBuf, Entry>,
    folders: HashMap<PathBuf, FolderKey>,
    next_generation: u64,
}

/// The per-repository context cache.
#[derive(Debug)]
pub struct RepositoryContexts {
    snapshot: RequestSnapshot,
    state: Mutex<State>,
}

impl RepositoryContexts {
    /// A cache whose builds take `HOME`, the environment, and extra `@` roots
    /// from `snapshot`. The snapshot's own request directory is never used:
    /// each build is anchored at a repository root or document folder.
    pub fn new(snapshot: RequestSnapshot) -> Self {
        Self { snapshot, state: Mutex::new(State::default()) }
    }

    /// The context for the document at `path`: its repository's context
    /// derived for the document, or the cached build failure.
    pub fn for_document(&self, path: &Path) -> DocumentResolution {
        let folder = path.parent().unwrap_or(path);
        let mut state = self.state.lock().expect("context cache lock poisoned");
        let key = self.folder_key(&mut state, folder).key;
        let entry = self.entry(&mut state, &key);
        DocumentResolution {
            generation: entry.generation,
            outcome: match entry.built {
                Ok(context) => Ok(context.for_source(path)),
                Err(error) => Err(ContextFailure::Build(error)),
            },
        }
    }

    /// The context for an untitled buffer.
    ///
    /// Each workspace folder counts the repository that contains it,
    /// discovered upward from the folder; repositories merely nested below a
    /// folder are not counted. With exactly one, the buffer uses that
    /// repository's context, whose current directory is the repository root.
    /// Otherwise the buffer gets [`ContextFailure::UntitledWorkspace`].
    pub fn for_untitled(&self, workspace_folders: &[PathBuf]) -> DocumentResolution {
        let mut state = self.state.lock().expect("context cache lock poisoned");
        let repositories: HashSet<PathBuf> = workspace_folders
            .iter()
            .map(|folder| self.folder_key(&mut state, folder))
            .filter(|folder| folder.repository)
            .map(|folder| folder.key)
            .collect();
        if repositories.len() != 1 {
            return DocumentResolution::from_failure(ContextFailure::UntitledWorkspace {
                repositories: repositories.len(),
                folders: workspace_folders.to_vec(),
            });
        }
        let key = repositories.into_iter().next().expect("exactly one repository");
        let entry = self.entry(&mut state, &key);
        DocumentResolution {
            generation: entry.generation,
            outcome: match entry.built {
                Ok(context) => Ok(context.as_ref().clone()),
                Err(error) => Err(ContextFailure::Build(error)),
            },
        }
    }

    /// Drops every entry a change at `changed` can affect.
    ///
    /// That is every entry keyed at an ancestor of `changed` (a manifest or
    /// file inside a repository), and every entry keyed inside the directory
    /// that owns it, so a repaired `.git/config` also drops the failures
    /// cached for folders of that repository.
    ///
    /// ## Returns
    ///
    /// `true` when an entry was dropped.
    pub fn invalidate(&self, changed: &Path) -> bool {
        let owner = owning_directory(changed);
        let mut state = self.state.lock().expect("context cache lock poisoned");
        let before = state.entries.len();
        state.entries.retain(|key, _| {
            !(changed.starts_with(key) || owner.as_deref().is_some_and(|owner| key.starts_with(owner)))
        });
        let dropped = state.entries.len() != before;
        if dropped || changed.components().any(|component| component.as_os_str() == ".git") {
            state.folders.clear();
        }
        if dropped {
            tracing::debug!(path = %changed.display(), "dropped file-resolution contexts");
        }
        dropped
    }

    /// Drops every entry (a configuration change).
    ///
    /// ## Returns
    ///
    /// `true` when there was an entry to drop.
    pub fn clear(&self) -> bool {
        let mut state = self.state.lock().expect("context cache lock poisoned");
        let had_entries = !state.entries.is_empty();
        state.entries.clear();
        state.folders.clear();
        had_entries
    }

    fn folder_key(&self, state: &mut State, folder: &Path) -> FolderKey {
        if let Some(known) = state.folders.get(folder) {
            return known.clone();
        }
        // A discovery error keys the folder itself; the builder then reports
        // the same error for it.
        let found = match biscuit_file::find_git_root(folder) {
            Ok(Some(root)) => FolderKey { key: root, repository: true },
            Ok(None) | Err(_) => FolderKey { key: folder.to_path_buf(), repository: false },
        };
        state.folders.insert(folder.to_path_buf(), found.clone());
        found
    }

    fn entry(&self, state: &mut State, key: &Path) -> Entry {
        if let Some(entry) = state.entries.get(key) {
            return entry.clone();
        }
        CONTEXT_BUILDS.fetch_add(1, Ordering::SeqCst);
        state.next_generation += 1;
        let built = match build_resolution_context(&self.snapshot.at_request_dir(key)) {
            Ok(context) => Ok(Arc::new(context)),
            Err(error) => {
                tracing::error!(
                    directory = %error.request_dir().display(),
                    failure = ?error.resolution_failure(),
                    "file-resolution context build failed: {error}"
                );
                Err(Arc::new(error))
            }
        };
        let entry = Entry { generation: state.next_generation, built };
        state.entries.insert(key.to_path_buf(), entry.clone());
        entry
    }
}

impl DocumentContexts for RepositoryContexts {
    fn context_for(&self, document: &Path) -> Option<FileResolutionContext> {
        self.for_document(document).outcome.ok()
    }
}

/// A [`DocumentContexts`] deriving every document's context from one fixed
/// request context, for analyzing documents outside a running server (tests
/// and tools). A document outside the context's tree resolves nothing.
#[derive(Debug, Clone)]
pub struct FixedContext(pub FileResolutionContext);

impl DocumentContexts for FixedContext {
    fn context_for(&self, document: &Path) -> Option<FileResolutionContext> {
        Some(self.0.for_source(document))
    }
}

/// The directory whose contexts a change at `path` can affect: the parent of
/// a `.git` directory the path lies in, else the path's own directory.
fn owning_directory(path: &Path) -> Option<PathBuf> {
    let mut owner = PathBuf::new();
    for component in path.components() {
        if component == Component::Normal(".git".as_ref()) {
            return Some(owner);
        }
        owner.push(component);
    }
    path.parent().map(Path::to_path_buf)
}

/// The outcome of resolving one authored reference against a document's
/// context, probing the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceTarget {
    /// An existing file the reference names.
    Found(PathBuf),
    /// No file matched.
    Missing {
        /// The first candidate the reference plans, where a new file would
        /// go; `None` when planning itself failed.
        candidate: Option<PathBuf>,
        /// The failure class.
        failure: ResolutionFailure,
    },
}

impl ReferenceTarget {
    /// The existing file, if any.
    pub fn found(&self) -> Option<&Path> {
        match self {
            Self::Found(path) => Some(path),
            Self::Missing { .. } => None,
        }
    }

    /// The existing file, else the first planned candidate: the target an
    /// editor navigates to, as `md compose` absolutizes a link to a file not
    /// yet created.
    pub fn navigation_target(&self) -> Option<&Path> {
        match self {
            Self::Found(path) => Some(path),
            Self::Missing { candidate, .. } => candidate.as_deref(),
        }
    }
}

/// Resolves `raw` against `context` the way composition does: the first
/// planned candidate that is an existing file.
pub fn resolve_reference(context: &FileResolutionContext, raw: &str) -> ReferenceTarget {
    let reference = match FileReference::new(raw) {
        Ok(reference) => reference,
        Err(error) => {
            return ReferenceTarget::Missing { candidate: None, failure: error.resolution_failure() };
        }
    };
    match reference.resolve_in_context(context) {
        Ok(Some(path)) => ReferenceTarget::Found(path),
        Ok(None) => ReferenceTarget::Missing {
            candidate: reference_candidates(context, raw).into_iter().next(),
            failure: ResolutionFailure::NoMatch,
        },
        Err(error) => ReferenceTarget::Missing {
            candidate: None,
            failure: error.resolution_failure(),
        },
    }
}

/// The candidates `raw` plans in `context`, in resolution order, without
/// probing the filesystem. Empty when the reference is malformed or a
/// required anchor is absent.
pub fn reference_candidates(context: &FileResolutionContext, raw: &str) -> Vec<PathBuf> {
    plan_reference(context, raw).unwrap_or_default()
}

/// The candidates `raw` plans in `context`, or the failure class when it
/// cannot be planned (a malformed reference or an absent anchor).
pub fn plan_reference(
    context: &FileResolutionContext,
    raw: &str,
) -> Result<Vec<PathBuf>, ResolutionFailure> {
    FileReference::new(raw)
        .and_then(|reference| {
            reference.candidate_plan_with_order(context, CandidatePlanOrder::Resolution)
        })
        .map(|plan| plan.into_iter().map(|candidate| candidate.path().to_path_buf()).collect())
        .map_err(|error| error.resolution_failure())
}

/// The `data` payload a file-reference diagnostic carries:
/// `{"resolution_failure": "<Class>"}`.
pub fn resolution_failure_data(failure: ResolutionFailure) -> serde_json::Value {
    serde_json::json!({ "resolution_failure": format!("{failure:?}") })
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Fixed contexts over the lexical `/w` paths unit tests use.

    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use biscuit_file::FileResolutionContext;

    use super::{DocumentResolution, FixedContext};

    /// `path` as an absolute path on this platform: `/w/a.md` stays as is on
    /// Unix and gains a drive (`C:/w/a.md`) on Windows, where a context
    /// rejects a rootless directory.
    pub(crate) fn abs(path: &str) -> PathBuf {
        if cfg!(windows) && path.starts_with('/') {
            PathBuf::from(format!("C:{path}"))
        } else {
            PathBuf::from(path)
        }
    }

    /// A request context at `/w` with no home and no repository.
    pub(crate) fn workspace_context() -> FileResolutionContext {
        FileResolutionContext::from_snapshot(abs("/w"), None, HashMap::new())
    }

    /// Every document resolves within `/w`.
    pub(crate) fn workspace_contexts() -> FixedContext {
        FixedContext(workspace_context())
    }

    /// The resolution for a document at `path` within `/w`. A rootless
    /// `/w/...` path is read as [`abs`] reads it.
    pub(crate) fn resolution_for(path: impl AsRef<Path>) -> DocumentResolution {
        let path = abs(&path.as_ref().to_string_lossy());
        DocumentResolution::from_context(workspace_context().for_source(path))
    }

    /// The resolution for a document at a real `path` inside `root`, with no
    /// repository.
    pub(crate) fn resolution_in(root: &Path, path: &Path) -> DocumentResolution {
        DocumentResolution::from_context(
            FileResolutionContext::from_snapshot(root, None, HashMap::new()).for_source(path),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RequestSnapshot {
        RequestSnapshot::new(std::env::temp_dir())
    }

    fn init_repository(root: &Path) {
        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::create_dir_all(root.join(".git/refs")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            root.join(".git/config"),
            "[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
        )
        .unwrap();
    }

    #[test]
    fn documents_in_one_repository_share_one_build() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        init_repository(&root);
        std::fs::create_dir_all(root.join("docs/deep")).unwrap();
        let contexts = RepositoryContexts::new(snapshot());

        let before = context_build_count();
        let first = contexts.for_document(&root.join("docs/a.md"));
        let second = contexts.for_document(&root.join("docs/deep/b.md"));
        assert_eq!(context_build_count() - before, 1);
        assert_eq!(first.generation(), second.generation());
        let first = first.context().expect("context");
        assert_eq!(first.request_cwd(), root.as_path());
        assert_eq!(first.cwd(), root.join("docs").as_path());
    }

    #[test]
    fn invalidation_drops_ancestor_and_owned_entries_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        let other = dir.path().join("other");
        init_repository(&root);
        init_repository(&other);
        let contexts = RepositoryContexts::new(snapshot());
        let in_root = contexts.for_document(&root.join("a.md")).generation();
        let in_other = contexts.for_document(&other.join("b.md")).generation();

        assert!(contexts.invalidate(&root.join("pkg/Cargo.toml")));
        assert_ne!(contexts.for_document(&root.join("a.md")).generation(), in_root);
        assert_eq!(contexts.for_document(&other.join("b.md")).generation(), in_other);
        assert!(!contexts.invalidate(&dir.path().join("elsewhere/x.md")));
    }

    #[test]
    fn a_git_config_change_drops_the_folder_entries_of_its_repository() {
        assert_eq!(
            owning_directory(Path::new("/w/repo/.git/config")),
            Some(PathBuf::from("/w/repo"))
        );
        assert_eq!(owning_directory(Path::new("/w/repo/pkg/Cargo.toml")), Some(PathBuf::from("/w/repo/pkg")));
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_failed_build_logs_one_error_naming_the_directory_and_class() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        init_repository(&root);
        std::fs::write(root.join(".git/config"), "[core\nthis is not = = valid\n").unwrap();
        let contexts = RepositoryContexts::new(snapshot());

        contexts.for_document(&root.join("a.md"));
        // A cached failure is not rebuilt, so it is not logged again.
        contexts.for_document(&root.join("b.md"));

        let shown = root.display().to_string();
        logs_assert(|lines: &[&str]| {
            let events: Vec<&&str> = lines
                .iter()
                .filter(|line| line.contains("file-resolution context build failed"))
                .collect();
            match events.as_slice() {
                [event]
                    if event.contains("ERROR")
                        && event.contains(&format!("directory={shown}"))
                        && event.contains("failure=MissingContext") =>
                {
                    Ok(())
                }
                other => Err(format!("expected one error naming {shown}, got {other:?}")),
            }
        });
    }

    #[test]
    fn untitled_buffers_need_exactly_one_repository() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        let other = dir.path().join("other");
        let plain = dir.path().join("plain");
        init_repository(&root);
        init_repository(&other);
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::create_dir_all(&plain).unwrap();
        let contexts = RepositoryContexts::new(snapshot());

        let one = contexts.for_untitled(&[root.join("a"), root.clone()]);
        assert_eq!(one.context().expect("one repository").request_cwd(), root.as_path());

        for folders in [vec![root.clone(), other.clone()], vec![plain.clone()], vec![]] {
            let failure = contexts.for_untitled(&folders);
            assert!(matches!(
                failure.failure(),
                Some(ContextFailure::UntitledWorkspace { .. })
            ), "{folders:?}");
            assert_eq!(
                failure.failure().unwrap().resolution_failure(),
                ResolutionFailure::MissingContext
            );
        }
    }

    #[test]
    fn a_corrupt_git_config_is_a_cached_failure_until_invalidated() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        init_repository(&root);
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join(".git/config"), "[core\nthis is not = = valid\n").unwrap();
        let contexts = RepositoryContexts::new(snapshot());

        let failed = contexts.for_document(&root.join("docs/a.md"));
        let failure = failed.failure().expect("discovery fails");
        assert_eq!(failure.resolution_failure(), ResolutionFailure::MissingContext);
        assert!(failure.to_string().contains(&root.join("docs").display().to_string()), "{failure}");

        init_repository(&root);
        assert!(contexts.for_document(&root.join("docs/a.md")).failure().is_some(), "cached");
        assert!(contexts.invalidate(&root.join(".git/config")));
        assert!(contexts.for_document(&root.join("docs/a.md")).context().is_some());
    }
}
