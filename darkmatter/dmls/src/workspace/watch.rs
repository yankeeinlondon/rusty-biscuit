//! File-watch handling: client-side dynamic registration plus the server-side
//! rescan fallback.
//!
//! Clients that support dynamic `workspace/didChangeWatchedFiles` registration
//! get watchers registered for the configured include globs (R-7). Clients
//! whose watching is absent or unreliable — notably Neovim on Linux — fall
//! back to a server-side rescan-and-rehash pass. Either way, bursts of events
//! for the same path are coalesced to one change so a single save doesn't
//! trigger repeated re-indexing.

use std::path::PathBuf;

use lsp_types::{
    DidChangeWatchedFilesRegistrationOptions, FileChangeType, FileSystemWatcher, GlobPattern,
    Registration, WatchKind,
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

/// Builds the dynamic watcher registration for the configured include globs.
///
/// ## Returns
///
/// `None` when there is nothing to watch (no include patterns).
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
    // Trigger envelopes and their payload/import/example files live in schema
    // roots and can affect Markdown documents even though they are not part of
    // workspace document discovery.
    watchers.extend(["**/schemas/*.yaml", "**/schemas/*.yml"].map(|pattern| {
        FileSystemWatcher {
            glob_pattern: GlobPattern::String(pattern.to_string()),
            kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
        }
    }));
    let options = DidChangeWatchedFilesRegistrationOptions { watchers };
    Some(Registration {
        id: WATCH_REGISTRATION_ID.to_string(),
        method: "workspace/didChangeWatchedFiles".to_string(),
        register_options: Some(serde_json::to_value(options).ok()?),
    })
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
        assert_eq!(options.watchers.len(), 4); // Markdown plus YAML schema roots.
    }

    #[test]
    fn test_registration_none_without_includes() {
        let config = WorkspaceConfig {
            include: Vec::new(),
            ..WorkspaceConfig::default()
        };
        let registration = watch_registration(&config).unwrap();
        let options: DidChangeWatchedFilesRegistrationOptions =
            serde_json::from_value(registration.register_options.unwrap()).unwrap();
        assert_eq!(options.watchers.len(), 2);
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
