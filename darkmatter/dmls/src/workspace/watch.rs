//! File-watch handling: client-side dynamic registration plus the server-side
//! rescan fallback.
//!
//! Clients that support dynamic `workspace/didChangeWatchedFiles` registration
//! get watchers registered for the configured include globs (R-7). Clients
//! whose watching is absent or unreliable — notably Neovim on Linux — fall
//! back to a server-side rescan-and-rehash pass. Either way, bursts of events
//! for the same path are coalesced to one change so a single save doesn't
//! trigger repeated re-indexing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::{
    DidChangeWatchedFilesRegistrationOptions, FileChangeType, FileSystemWatcher, GlobPattern, OneOf,
    Registration, RelativePattern, WatchKind,
};

use crate::capabilities::ClientProfile;
use crate::config::WorkspaceConfig;

/// How DMLS learns about on-disk changes for a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchMode {
    /// The client watches and sends `workspace/didChangeWatchedFiles`.
    ClientWatched,
    /// DMLS rescans and re-hashes on demand (no reliable client watcher).
    ServerRescan,
}

impl WatchMode {
    /// Selects the mode from the client profile.
    pub fn for_profile(profile: &ClientProfile) -> Self {
        if profile.client_watches_files && !profile.needs_watch_fallback {
            WatchMode::ClientWatched
        } else {
            WatchMode::ServerRescan
        }
    }
}

/// The registration id DMLS uses for its watcher registration.
pub const WATCH_REGISTRATION_ID: &str = "dmls-watch-workspace";

/// Builds the dynamic watcher registration for the workspace: the configured
/// include globs, every YAML file, the discovery inputs, and the context
/// inputs. An empty `include` drops only the Markdown globs.
///
/// ## Returns
///
/// `None` when the registration cannot be serialized.
pub fn watch_registration(config: &WorkspaceConfig) -> Option<Registration> {
    let mut watchers: Vec<_> = config
        .include
        .iter()
        .map(|pattern| FileSystemWatcher {
            glob_pattern: GlobPattern::String(pattern.clone()),
            // Create/change/delete all matter for the graph.
            kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
        })
        .collect();
    // YAML is listed whatever `include` says, and schema-root files, which
    // shape every document, are YAML; the discovery inputs (ignore files and
    // the config file) rebuild the file list.
    let globs = ["**/*.yaml".to_string(), "**/*.yml".to_string()]
        .into_iter()
        .chain(DISCOVERY_INPUT_NAMES.iter().map(|name| format!("**/{name}")));
    watchers.extend(globs.map(|pattern| FileSystemWatcher {
        glob_pattern: GlobPattern::String(pattern),
        kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
    }));
    // Context inputs: a package manifest changes `^` resolution, and a
    // repository's `.git/config` decides whether discovery succeeds. Neither
    // is a workspace document; a change only drops cached contexts.
    watchers.extend(context_input_globs().map(|pattern| FileSystemWatcher {
        glob_pattern: GlobPattern::String(pattern),
        kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
    }));
    let options = DidChangeWatchedFilesRegistrationOptions { watchers };
    Some(Registration {
        id: WATCH_REGISTRATION_ID.to_string(),
        method: "workspace/didChangeWatchedFiles".to_string(),
        register_options: Some(serde_json::to_value(options).ok()?),
    })
}

/// File names whose change rebuilds the workspace file list: the ignore
/// files the walk honors and the DMLS config file. (`.git/info/exclude` sits
/// inside `.git`, which is never watched as a document; a change to it takes
/// effect on the next rescan or restart.)
pub const DISCOVERY_INPUT_NAMES: &[&str] = &[".gitignore", ".ignore", crate::config::CONFIG_FILE_NAME];

/// Whether `path` is an ignore file the walk honors.
pub fn is_ignore_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".gitignore" | ".ignore"))
}

/// Whether `path` is a DMLS config file.
pub fn is_config_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == crate::config::CONFIG_FILE_NAME)
}

/// The registration id DMLS uses for its schema-location watchers.
pub const SCHEMA_WATCH_REGISTRATION_ID: &str = "dmls-watch-schema-inputs";

/// Builds the watcher registration rooted at the folders trigger registries
/// read: schema-root folders (present or absent) and the folders holding
/// payload, import, and example files, wherever they live (one watch per
/// folder, not per file).
///
/// Each watch is rooted at the folder's parent with the pattern
/// `<folder>/*`, so a root folder that does not exist yet is still watched
/// for the files whose creation adds it. Requires the client's
/// `relativePatternSupport`.
///
/// ## Returns
///
/// `None` when there is no folder to watch.
pub fn schema_watch_registration<'a>(folders: impl IntoIterator<Item = &'a Path>) -> Option<Registration> {
    let watchers: Vec<FileSystemWatcher> = folders
        .into_iter()
        .filter_map(|folder| {
            let parent = folder.parent()?;
            let name = folder.file_name()?.to_str()?;
            let base_uri = crate::workspace::file_path_to_uri(parent)?;
            Some(FileSystemWatcher {
                glob_pattern: GlobPattern::Relative(RelativePattern {
                    base_uri: OneOf::Right(base_uri),
                    pattern: format!("{}/*", escape_glob(name)),
                }),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            })
        })
        .collect();
    if watchers.is_empty() {
        return None;
    }
    let options = DidChangeWatchedFilesRegistrationOptions { watchers };
    Some(Registration {
        id: SCHEMA_WATCH_REGISTRATION_ID.to_string(),
        method: "workspace/didChangeWatchedFiles".to_string(),
        register_options: Some(serde_json::to_value(options).ok()?),
    })
}

/// Escapes LSP glob metacharacters in a literal path segment.
fn escape_glob(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for character in segment.chars() {
        if matches!(character, '*' | '?' | '[' | ']' | '{' | '}') {
            out.push('[');
            out.push(character);
            out.push(']');
        } else {
            out.push(character);
        }
    }
    out
}

/// The watcher globs for context inputs: `**/<name>` for each package
/// manifest name sniff detects packages by, then `**/.git/config`.
pub fn context_input_globs() -> impl Iterator<Item = String> {
    sniff::filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES
        .iter()
        .map(|name| format!("**/{name}"))
        .chain(std::iter::once("**/.git/config".to_string()))
}

/// Whether `path` is a context input rather than a workspace document: a
/// package manifest or a file inside a `.git` directory.
pub fn is_context_input(path: &Path) -> bool {
    let is_manifest = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| sniff::filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES.contains(&name));
    is_manifest || path.components().any(|component| component.as_os_str() == ".git")
}

/// Content hashes of every package manifest under `roots`, for the
/// server-rescan fallback to detect a manifest change no client reported.
///
/// The walk honors `.gitignore` and skips hidden directories. Workspace
/// discovery, unlike this walk, descends into hidden directories.
pub fn scan_manifests(roots: &[PathBuf]) -> HashMap<PathBuf, u64> {
    let mut manifests = HashMap::new();
    for root in roots {
        for entry in ignore::WalkBuilder::new(root).build().flatten() {
            let path = entry.path();
            let is_manifest = path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
                sniff::filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES.contains(&name)
            });
            if is_manifest && let Ok(bytes) = crate::work_counters::read(path) {
                manifests.insert(path.to_path_buf(), crate::work_counters::hash_bytes(&bytes));
            }
        }
    }
    manifests
}

/// The manifests that were created, changed, or deleted between two
/// [`scan_manifests`] results.
pub fn changed_manifests(
    before: &HashMap<PathBuf, u64>,
    after: &HashMap<PathBuf, u64>,
) -> Vec<PathBuf> {
    let mut changed: Vec<PathBuf> = after
        .iter()
        .filter(|(path, hash)| before.get(*path) != Some(*hash))
        .map(|(path, _)| path.clone())
        .chain(before.keys().filter(|path| !after.contains_key(*path)).cloned())
        .collect();
    changed.sort();
    changed
}

/// A coalesced filesystem change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoalescedChange {
    /// The affected path.
    pub path: PathBuf,
    /// The net change type after coalescing a burst.
    pub kind: FileChangeType,
}

/// Coalesces a burst of `(path, change type)` events into one change per path.
///
/// The last event for a path wins, with two corrections that keep the graph
/// consistent: a path that is created and then deleted in the same burst
/// collapses to nothing, and a create-then-change collapses to a create.
pub fn coalesce_changes(
    events: impl IntoIterator<Item = (PathBuf, FileChangeType)>,
) -> Vec<CoalescedChange> {
    // Preserve first-seen order for determinism while letting later events
    // update the net kind. `None` in the map means "cancelled out".
    let mut order: Vec<PathBuf> = Vec::new();
    let mut net: std::collections::HashMap<PathBuf, Option<FileChangeType>> =
        std::collections::HashMap::new();

    for (path, kind) in events {
        match net.get(&path).copied() {
            None => {
                order.push(path.clone());
                net.insert(path, Some(kind));
            }
            Some(previous) => {
                net.insert(path, coalesce_pair(previous, kind));
            }
        }
    }

    order
        .into_iter()
        .filter_map(|path| {
            net.get(&path)
                .copied()
                .flatten()
                .map(|kind| CoalescedChange { path, kind })
        })
        .collect()
}

/// Nets two successive change types for the same path. `previous` of `None`
/// means the pair cancelled earlier; `None` result means it cancels now.
fn coalesce_pair(previous: Option<FileChangeType>, next: FileChangeType) -> Option<FileChangeType> {
    let Some(previous) = previous else {
        return Some(next);
    };
    match (previous, next) {
        // Created then deleted within the burst → no net change.
        (FileChangeType::CREATED, FileChangeType::DELETED) => None,
        // Created then changed is still a creation from our perspective.
        (FileChangeType::CREATED, FileChangeType::CHANGED) => Some(FileChangeType::CREATED),
        // Deleted then created is a net change.
        (FileChangeType::DELETED, FileChangeType::CREATED) => Some(FileChangeType::CHANGED),
        (_, next) => Some(next),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_mode_selection() {
        let mut profile = profile_with(true, false);
        assert_eq!(WatchMode::for_profile(&profile), WatchMode::ClientWatched);
        profile.needs_watch_fallback = true; // Neovim-on-Linux shape
        assert_eq!(WatchMode::for_profile(&profile), WatchMode::ServerRescan);
        let no_watch = profile_with(false, true);
        assert_eq!(WatchMode::for_profile(&no_watch), WatchMode::ServerRescan);
    }

    #[test]
    fn test_registration_covers_include_globs() {
        let config = WorkspaceConfig::default();
        let registration = watch_registration(&config).unwrap();
        assert_eq!(registration.method, "workspace/didChangeWatchedFiles");
        let options: DidChangeWatchedFilesRegistrationOptions =
            serde_json::from_value(registration.register_options.unwrap()).unwrap();
        // Markdown (2), YAML (2), discovery inputs (3), four package
        // manifests, `.git/config`.
        assert_eq!(options.watchers.len(), 12);
        let globs: Vec<String> = options
            .watchers
            .iter()
            .map(|watcher| match &watcher.glob_pattern {
                GlobPattern::String(glob) => glob.clone(),
                GlobPattern::Relative(_) => String::new(),
            })
            .collect();
        for name in sniff::filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES {
            assert!(globs.contains(&format!("**/{name}")), "{name} watched: {globs:?}");
        }
        assert!(globs.contains(&"**/.git/config".to_string()), "{globs:?}");
        for glob in ["**/*.yaml", "**/*.yml", "**/.gitignore", "**/.ignore", "**/.dmls.toml"] {
            assert!(globs.contains(&glob.to_string()), "{glob} watched: {globs:?}");
        }
    }

    #[test]
    fn test_empty_include_keeps_yaml_and_schema_watches() {
        let config = WorkspaceConfig {
            include: Vec::new(),
            ..WorkspaceConfig::default()
        };
        let registration = watch_registration(&config).unwrap();
        let options: DidChangeWatchedFilesRegistrationOptions =
            serde_json::from_value(registration.register_options.unwrap()).unwrap();
        assert_eq!(options.watchers.len(), 10);
        assert!(options.watchers.iter().any(|watcher| {
            matches!(&watcher.glob_pattern, GlobPattern::String(glob) if glob == "**/*.yaml")
        }));
    }

    #[test]
    fn test_schema_watch_registration_roots_each_folder_at_its_parent() {
        let temp = tempfile::tempdir().unwrap();
        let schemas = temp.path().join("schemas");
        let payloads = temp.path().join("odd[name]");
        let registration =
            schema_watch_registration([schemas.as_path(), payloads.as_path()]).unwrap();
        assert_eq!(registration.id, SCHEMA_WATCH_REGISTRATION_ID);
        let options: DidChangeWatchedFilesRegistrationOptions =
            serde_json::from_value(registration.register_options.unwrap()).unwrap();
        let patterns: Vec<(String, String)> = options
            .watchers
            .iter()
            .map(|watcher| match &watcher.glob_pattern {
                GlobPattern::Relative(relative) => match &relative.base_uri {
                    OneOf::Right(uri) => (uri.as_str().to_string(), relative.pattern.clone()),
                    OneOf::Left(folder) => (folder.uri.as_str().to_string(), relative.pattern.clone()),
                },
                GlobPattern::String(glob) => (String::new(), glob.clone()),
            })
            .collect();
        let base = crate::workspace::file_path_to_uri(temp.path()).unwrap().as_str().to_string();
        assert_eq!(
            patterns,
            vec![(base.clone(), "schemas/*".to_string()), (base, "odd[[]name[]]/*".to_string())]
        );
        assert!(schema_watch_registration(std::iter::empty()).is_none());
    }

    #[test]
    fn test_discovery_inputs_are_ignore_and_config_files() {
        assert!(is_ignore_file(Path::new("/w/.gitignore")));
        assert!(is_ignore_file(Path::new("/w/docs/.ignore")));
        assert!(!is_ignore_file(Path::new("/w/.gitignore.md")));
        assert!(is_config_file(Path::new("/w/.dmls.toml")));
        assert!(!is_config_file(Path::new("/w/dmls.toml")));
    }

    #[test]
    fn test_context_inputs_are_manifests_and_git_files() {
        assert!(is_context_input(Path::new("/w/pkg/Cargo.toml")));
        assert!(is_context_input(Path::new("/w/web/package.json")));
        assert!(is_context_input(Path::new("/w/.git/config")));
        assert!(!is_context_input(Path::new("/w/docs/a.md")));
        assert!(!is_context_input(Path::new("/w/schemas/s.yaml")));
    }

    #[test]
    fn test_manifest_scan_reports_created_changed_and_deleted() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::write(root.join("a/Cargo.toml"), "[package]\n").unwrap();
        std::fs::write(root.join("a/notes.md"), "# notes\n").unwrap();
        let before = scan_manifests(std::slice::from_ref(&root));
        assert_eq!(before.len(), 1);

        std::fs::write(root.join("a/Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        std::fs::create_dir_all(root.join("b")).unwrap();
        std::fs::write(root.join("b/package.json"), "{}").unwrap();
        let after = scan_manifests(std::slice::from_ref(&root));
        assert_eq!(
            changed_manifests(&before, &after),
            vec![root.join("a/Cargo.toml"), root.join("b/package.json")]
        );
        assert_eq!(changed_manifests(&after, &before), changed_manifests(&before, &after));
        assert!(changed_manifests(&after, &after).is_empty());
    }

    #[test]
    fn test_coalesce_dedupes_last_wins() {
        let events = vec![
            (PathBuf::from("a.md"), FileChangeType::CHANGED),
            (PathBuf::from("a.md"), FileChangeType::CHANGED),
            (PathBuf::from("b.md"), FileChangeType::CREATED),
        ];
        let coalesced = coalesce_changes(events);
        assert_eq!(coalesced.len(), 2);
        assert_eq!(coalesced[0].path, PathBuf::from("a.md"));
        assert_eq!(coalesced[0].kind, FileChangeType::CHANGED);
        assert_eq!(coalesced[1].kind, FileChangeType::CREATED);
    }

    #[test]
    fn test_coalesce_create_then_delete_cancels() {
        let events = vec![
            (PathBuf::from("t.md"), FileChangeType::CREATED),
            (PathBuf::from("t.md"), FileChangeType::DELETED),
        ];
        assert!(coalesce_changes(events).is_empty());
    }

    #[test]
    fn test_coalesce_create_then_change_is_create() {
        let events = vec![
            (PathBuf::from("t.md"), FileChangeType::CREATED),
            (PathBuf::from("t.md"), FileChangeType::CHANGED),
        ];
        let coalesced = coalesce_changes(events);
        assert_eq!(coalesced[0].kind, FileChangeType::CREATED);
    }

    fn profile_with(watches: bool, fallback: bool) -> ClientProfile {
        use crate::source_map::PositionEncoding;
        let params: lsp_types::InitializeParams =
            serde_json::from_value(serde_json::json!({ "capabilities": {} })).unwrap();
        let mut profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf16);
        profile.client_watches_files = watches;
        profile.needs_watch_fallback = fallback;
        profile
    }
}
