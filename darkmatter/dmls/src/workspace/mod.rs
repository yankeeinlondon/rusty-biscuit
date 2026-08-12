//! Workspace state: open documents, on-disk discovery, the worker-pool
//! startup indexer, watch handling, and the shared snapshot the protocol loop
//! reads from.

pub mod discover;
pub mod documents;
pub mod snapshot;
pub mod startup;
pub mod watch;

use std::path::PathBuf;

use lsp_types::{InitializeParams, Uri};

pub use discover::{DiscoveryReport, discover_workspace};
pub use documents::{DocumentError, DocumentStore, OpenDocument};
pub use snapshot::{Generation, SharedSnapshot};
pub use startup::{ProgressReporter, SilentProgress, collect_indices, index_workspace};
pub use watch::{CoalescedChange, WatchMode, coalesce_changes, watch_registration};

/// Converts a `file://` URI into a filesystem path.
///
/// Parsing goes through the `url` crate for its battle-tested percent
/// decoding and Windows drive-letter handling.
///
/// ## Returns
///
/// `None` for non-`file` schemes or malformed URIs.
pub fn uri_to_file_path(uri: &Uri) -> Option<PathBuf> {
    let parsed = url::Url::parse(uri.as_str()).ok()?;
    parsed.to_file_path().ok()
}

/// Converts a filesystem path into a `file://` URI.
///
/// ## Returns
///
/// `None` when the path is not absolute (file URIs require absolute paths).
pub fn file_path_to_uri(path: &std::path::Path) -> Option<Uri> {
    let url = url::Url::from_file_path(path).ok()?;
    url.as_str().parse().ok()
}

/// Extracts workspace root paths from `initialize` params.
///
/// Prefers `workspaceFolders`; falls back to the deprecated `rootUri` for
/// older clients. Non-`file` roots are skipped.
pub fn workspace_roots(params: &InitializeParams) -> Vec<PathBuf> {
    if let Some(folders) = params.workspace_folders.as_ref()
        && !folders.is_empty()
    {
        return folders
            .iter()
            .filter_map(|folder| uri_to_file_path(&folder.uri))
            .collect();
    }
    #[allow(deprecated)]
    params
        .root_uri
        .as_ref()
        .and_then(uri_to_file_path)
        .into_iter()
        .collect()
}

/// Resolves the R-8 wiki roots from the workspace folders and the optional
/// `wiki.wiki_root` config override.
///
/// With no override each workspace folder is a wiki root. A relative override
/// is joined onto each folder; an absolute override is used verbatim.
pub fn resolve_wiki_roots(
    workspace_folders: &[PathBuf],
    wiki_root: Option<&std::path::Path>,
) -> Vec<PathBuf> {
    match wiki_root {
        None => workspace_folders.to_vec(),
        Some(root) if root.is_absolute() => vec![root.to_path_buf()],
        Some(root) => workspace_folders
            .iter()
            .map(|folder| folder.join(root))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resolve_wiki_roots_defaults_to_folders() {
        let folders = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        assert_eq!(resolve_wiki_roots(&folders, None), folders);
    }

    #[test]
    fn test_resolve_wiki_roots_joins_relative_override() {
        let folders = vec![PathBuf::from("/a")];
        assert_eq!(
            resolve_wiki_roots(&folders, Some(std::path::Path::new("notes"))),
            vec![PathBuf::from("/a/notes")]
        );
    }

    #[test]
    fn test_uri_to_file_path_decodes_percent_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("with space").join("doc.md");
        let uri = file_path_to_uri(&path).unwrap();
        assert!(uri.as_str().contains("with%20space"));
        assert_eq!(uri_to_file_path(&uri), Some(path));
    }

    #[test]
    fn test_uri_to_file_path_rejects_non_file_scheme() {
        let uri: Uri = "https://example.com/doc.md".parse().unwrap();
        assert_eq!(uri_to_file_path(&uri), None);
    }

    #[cfg(windows)]
    #[test]
    fn test_uri_to_file_path_windows_drive_letter() {
        let uri: Uri = "file:///c%3A/notes/doc.md".parse().unwrap();
        assert_eq!(
            uri_to_file_path(&uri),
            Some(PathBuf::from("c:\\notes\\doc.md"))
        );
    }

    #[test]
    fn test_workspace_roots_prefers_folders() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = file_path_to_uri(&dir.path().join("legacy")).unwrap();
        let alpha = file_path_to_uri(&dir.path().join("alpha")).unwrap();
        let beta = file_path_to_uri(&dir.path().join("beta")).unwrap();
        let params: InitializeParams = serde_json::from_value(json!({
            "capabilities": {},
            "rootUri": legacy.as_str(),
            "workspaceFolders": [
                { "uri": alpha.as_str(), "name": "alpha" },
                { "uri": beta.as_str(), "name": "beta" }
            ]
        }))
        .unwrap();
        let roots = workspace_roots(&params);
        assert_eq!(roots.len(), 2);
        assert!(roots[0].ends_with("alpha"));
    }

    #[test]
    fn test_workspace_roots_falls_back_to_root_uri() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = file_path_to_uri(&dir.path().join("legacy")).unwrap();
        let params: InitializeParams = serde_json::from_value(json!({
            "capabilities": {},
            "rootUri": legacy.as_str()
        }))
        .unwrap();
        let roots = workspace_roots(&params);
        assert_eq!(roots.len(), 1);
        assert!(roots[0].ends_with("legacy"));
    }

    #[test]
    fn test_workspace_roots_empty_when_absent() {
        let params: InitializeParams =
            serde_json::from_value(json!({ "capabilities": {} })).unwrap();
        assert!(workspace_roots(&params).is_empty());
    }
}
