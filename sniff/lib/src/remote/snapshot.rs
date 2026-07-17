//! Shared per-report remote evidence.
//!
//! A [`RemoteRepoSnapshot`] is the remote analogue of the filesystem observation
//! index: repository metadata, the default branch, and the repository tree are
//! resolved **once** per report and every projection reads from it. Before this
//! existed, `list_documents` and `detect_cicd` each re-fetched metadata purely to
//! learn the default branch and then re-fetched the identical recursive tree, so a
//! single report paid for the same evidence up to three times.
//!
//! See `sniff/features/2026-07-16-performance/phases/06-remote-network-and-subprocess/spec.md`.

use super::types::{DocumentCategory, DocumentRef, RepoMetadata};

/// One file observed in a remote repository tree.
///
/// Deliberately provider-neutral: GitHub and Gitea return recursive tree blobs,
/// GitLab a flat tree listing, and Bitbucket a paginated per-directory listing.
/// Normalizing at the edge is what lets the document and CI/CD projections be
/// shared rather than reimplemented per provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTreeFile {
    /// Path relative to the repository root.
    pub path: String,
    /// Size in bytes, when the provider reports one.
    pub size: Option<u64>,
}

/// The repository tree as observed for one report.
#[derive(Debug, Clone, Default)]
pub struct RemoteTree {
    /// Blob entries only; directory entries are dropped at the edge.
    pub files: Vec<RemoteTreeFile>,
    /// The provider reported its listing as incomplete.
    ///
    /// **Not the same as `files` being short.** GitHub truncates a recursive tree
    /// past ~100k entries or 7 MB and Bitbucket paginates; treating either as a
    /// complete listing silently reports a repository as having no docs and no CI.
    pub truncated: bool,
    /// Whether the tree was fetched at all.
    ///
    /// `false` means the request failed and the tree carries no evidence, so a
    /// projection must degrade rather than conclude "absent" from an empty list
    /// (R11.6). This is why the field exists instead of relying on `files`.
    pub available: bool,
}

impl RemoteTree {
    /// A tree that was fetched successfully.
    pub fn observed(files: Vec<RemoteTreeFile>, truncated: bool) -> Self {
        Self {
            files,
            truncated,
            available: true,
        }
    }

    /// A tree that could not be fetched.
    pub fn unavailable() -> Self {
        Self {
            files: Vec::new(),
            truncated: false,
            available: false,
        }
    }

    /// Whether any observed path sits under `prefix`.
    ///
    /// Compares on a `/`-terminated prefix so `.github/workflows` cannot match a
    /// file literally named `.github/workflows-old.yml`.
    pub fn has_path_under(&self, prefix: &str) -> bool {
        let prefix = format!("{}/", prefix.trim_end_matches('/'));
        self.files.iter().any(|f| f.path.starts_with(&prefix))
    }

    /// Whether an exact path was observed.
    pub fn contains(&self, path: &str) -> bool {
        self.files.iter().any(|f| f.path == path)
    }

    /// Merges continuation results, keeping the first entry for a duplicate path.
    ///
    /// Clears `truncated`: the caller resolved the paths it needed, so what remains
    /// missing is outside the projections this snapshot feeds.
    pub fn extend_from_continuation(&mut self, files: Vec<RemoteTreeFile>) {
        for file in files {
            if !self.files.iter().any(|f| f.path == file.path) {
                self.files.push(file);
            }
        }
        self.truncated = false;
    }
}

/// Repository evidence resolved once per report.
#[derive(Debug, Clone)]
pub struct RemoteRepoSnapshot {
    pub owner: String,
    pub repo: String,
    pub metadata: RepoMetadata,
    pub tree: RemoteTree,
}

impl RemoteRepoSnapshot {
    /// A snapshot carrying metadata but no tree evidence.
    ///
    /// The shape a provider that has not adopted the tree hook produces, and the
    /// shape a failed tree request degrades to.
    pub fn metadata_only(owner: &str, repo: &str, metadata: RepoMetadata) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            metadata,
            tree: RemoteTree::unavailable(),
        }
    }

    /// The branch every tree/content request for this report resolves against.
    pub fn default_branch(&self) -> &str {
        &self.metadata.default_branch
    }
}

/// Directory prefixes worth a bounded continuation request when a tree truncates.
///
/// A truncated tree cannot be completed cheaply, and completing it is not the goal
/// — preserving *document and CI/CD detection* is (R11.5). These are the only
/// top-level directories those two projections read, so continuing just them keeps
/// detection correct at a bounded, constant request cost.
///
/// ## Notes
///
/// **Every entry must be a single path component.** A continuation addresses a
/// subtree as `branch:prefix`, and the client percent-encodes that whole string
/// into one path segment — so a prefix containing `/` arrives as `%2F`, which
/// routers and proxies routinely reject or normalize. `.github` is therefore
/// continued whole and the recursive response supplies `workflows/ci.yml` beneath
/// it, rather than addressing `.github/workflows` directly.
pub(crate) const CONTINUATION_PREFIXES: &[&str] = &[".github", ".gitea", "docs", "doc"];

/// Categorize a file path as a document type.
pub(crate) fn categorize_document(path: &str) -> Option<DocumentCategory> {
    let lower = path.to_lowercase();
    let filename = path.rsplit('/').next().unwrap_or(path).to_lowercase();

    // Files in src/ directories (source documentation) - check first
    if lower.starts_with("src/") {
        // Only markdown/text files in src count as source docs
        if is_documentation_file(&filename) {
            return Some(DocumentCategory::SourceDoc);
        }
        return None;
    }

    // Files in docs/ or doc/ directories
    if lower.starts_with("docs/") || lower.starts_with("doc/") {
        return Some(DocumentCategory::DocsFolder);
    }

    // README files at root or in non-src directories
    if filename.starts_with("readme") {
        return Some(DocumentCategory::Readme);
    }

    // Other markdown/text files at root or elsewhere
    if is_documentation_file(&filename) {
        return Some(DocumentCategory::Other);
    }

    None
}

/// Check if a filename is a documentation file.
fn is_documentation_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".md")
        || lower.ends_with(".markdown")
        || lower.ends_with(".txt")
        || lower.ends_with(".rst")
        || lower == "license"
        || lower == "licence"
        || lower == "changelog"
        || lower == "changes"
        || lower == "history"
        || lower == "contributing"
        || lower == "authors"
        || lower == "contributors"
        || lower == "code_of_conduct"
}

/// Projects observed tree files into categorized documents.
///
/// Shared by every provider: the categorization rules are a property of the
/// *document model*, not of the hosting provider, and all four previously carried
/// byte-identical copies of it.
pub(crate) fn documents_from_tree(tree: &RemoteTree) -> Vec<DocumentRef> {
    tree.files
        .iter()
        .filter_map(|file| {
            categorize_document(&file.path).map(|category| DocumentRef {
                path: file.path.clone(),
                category,
                size: file.size,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(paths: &[&str]) -> RemoteTree {
        RemoteTree::observed(
            paths
                .iter()
                .map(|p| RemoteTreeFile {
                    path: (*p).to_string(),
                    size: None,
                })
                .collect(),
            false,
        )
    }

    #[test]
    fn prefix_matching_respects_a_path_boundary() {
        let t = tree(&[".github/workflows-old.yml", "docsite/index.md"]);

        assert!(!t.has_path_under(".github/workflows"));
        assert!(!t.has_path_under("docs"));

        let t = tree(&[".github/workflows/ci.yml"]);
        assert!(t.has_path_under(".github/workflows"));
    }

    /// An unavailable tree must not read as "observed and empty".
    #[test]
    fn an_unavailable_tree_is_distinguishable_from_an_empty_one() {
        let missing = RemoteTree::unavailable();
        let empty = RemoteTree::observed(Vec::new(), false);

        assert!(!missing.available);
        assert!(empty.available);
        assert_eq!(missing.files, empty.files);
    }

    #[test]
    fn test_categorize_readme() {
        assert_eq!(
            categorize_document("README.md"),
            Some(DocumentCategory::Readme)
        );
        assert_eq!(
            categorize_document("readme.txt"),
            Some(DocumentCategory::Readme)
        );
        assert_eq!(
            categorize_document("sub/README.md"),
            Some(DocumentCategory::Readme)
        );
    }

    #[test]
    fn test_categorize_docs_folder() {
        assert_eq!(
            categorize_document("docs/guide.md"),
            Some(DocumentCategory::DocsFolder)
        );
        assert_eq!(
            categorize_document("doc/api.md"),
            Some(DocumentCategory::DocsFolder)
        );
    }

    #[test]
    fn test_categorize_source_doc() {
        assert_eq!(
            categorize_document("src/README.md"),
            Some(DocumentCategory::SourceDoc)
        );
        // Non-doc files in src/ should be None
        assert_eq!(categorize_document("src/main.rs"), None);
    }

    #[test]
    fn test_categorize_other() {
        assert_eq!(
            categorize_document("CHANGELOG.md"),
            Some(DocumentCategory::Other)
        );
        assert_eq!(
            categorize_document("LICENSE"),
            Some(DocumentCategory::Other)
        );
        assert_eq!(
            categorize_document("CONTRIBUTING.md"),
            Some(DocumentCategory::Other)
        );
    }

    #[test]
    fn test_categorize_non_doc() {
        assert_eq!(categorize_document("main.rs"), None);
        assert_eq!(categorize_document("lib/utils.js"), None);
    }

    #[test]
    fn continuation_merges_without_duplicating_and_clears_truncation() {
        let mut t = RemoteTree::observed(
            vec![RemoteTreeFile {
                path: "README.md".to_string(),
                size: Some(1),
            }],
            true,
        );
        assert!(t.truncated);

        t.extend_from_continuation(vec![
            RemoteTreeFile {
                path: "README.md".to_string(),
                size: Some(999),
            },
            RemoteTreeFile {
                path: "docs/guide.md".to_string(),
                size: Some(2),
            },
        ]);

        assert_eq!(t.files.len(), 2, "the duplicate path must not be re-added");
        assert_eq!(t.files[0].size, Some(1), "the original entry wins");
        assert!(t.contains("docs/guide.md"));
        assert!(!t.truncated);
    }
}
