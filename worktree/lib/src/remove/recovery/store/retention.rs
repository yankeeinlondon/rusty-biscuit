//! Expiry of finished operations: [`RecoveryStore::prune_completed`].
//!
//! Only a readable record marked [`Phase::Completed`] lets an operation
//! expire; a pending or unreadable one is kept forever, since it may be the
//! only list of what is left in a quarantine. Everything here is best effort:
//! a failure keeps the folder for the next run and is never reported.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::{LOCKS_DIR, REPORT_FILE, RecoveryStore};
use crate::remove::recovery::OperationId;
use crate::remove::recovery::record::Phase;
use crate::remove::recovery::storage;

/// How long a finished operation, or a staging folder a crash left, is kept.
pub const RETENTION: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// What a store child's name makes it. Anything else is left alone.
enum Child {
    Operation(OperationId),
    /// `.<id>.tmp`: an operation directory [`RecoveryStore::begin`] never
    /// published.
    Staging,
    /// `.<id>.expired`: an expired operation whose deletion was cut short.
    Expired,
}

fn classify(name: &str) -> Option<Child> {
    if let Some(id) = OperationId::parse(name) {
        return Some(Child::Operation(id));
    }
    let hidden = name.strip_prefix('.')?;
    if hidden.strip_suffix(".tmp").and_then(OperationId::parse).is_some() {
        return Some(Child::Staging);
    }
    if hidden.strip_suffix(".expired").and_then(OperationId::parse).is_some() {
        return Some(Child::Expired);
    }
    None
}

/// The only part of a saved report that dates its completion. Any report
/// version that carries `timeline` reads, whatever else it holds.
#[derive(Deserialize)]
struct Timeline {
    timeline: Vec<Event>,
}

#[derive(Deserialize)]
struct Event {
    at_ms: u64,
}

impl RecoveryStore {
    /// Deletes every completed operation finished more than [`RETENTION`]
    /// before `now`, and every older staging folder. An operation is deleted
    /// only while this call holds its worktree record's lock, taken without
    /// waiting; a busy lock keeps it.
    ///
    /// Completion is the last event of the saved report's timeline, or the
    /// folder's modification time when that cannot be read. Exactly
    /// [`RETENTION`] old, or dated after `now`, is kept.
    ///
    /// An expired operation is renamed to `.<id>.expired` before it is
    /// deleted, so a deletion cut short never leaves a folder named as an
    /// operation without its record, which would read as an unreadable
    /// pending one and block every later removal. A leftover `.<id>.expired`
    /// is deleted on the next call.
    pub fn prune_completed(&self, now: SystemTime) {
        let Ok(children) = fs::read_dir(self.root()) else { return };
        for child in children.flatten() {
            let name = child.file_name();
            if name == LOCKS_DIR {
                continue;
            }
            let Some(kind) = name.to_str().and_then(classify) else { continue };
            let path = child.path();
            match kind {
                Child::Operation(id) => self.expire_operation(&id, now),
                Child::Staging => {
                    if storage::validate_dir(&path).is_ok_and(|metadata| metadata.modified().is_ok_and(|at| expired(now, at))) {
                        remove(&path, self.root());
                    }
                }
                Child::Expired => {
                    if storage::validate_dir(&path).is_ok() {
                        remove(&path, self.root());
                    }
                }
            }
        }
    }

    fn expire_operation(&self, id: &OperationId, now: SystemTime) {
        let Ok(record) = self.read_record(id) else { return };
        if record.completed_phase != Phase::Completed {
            return;
        }
        let Ok(_lock) = self.lock(&record.repository) else { return };
        // Read again under the lock: only a record the lock covers, still
        // completed, may go.
        let Ok(locked) = self.read_record(id) else { return };
        if locked.completed_phase != Phase::Completed || locked.repository.admin_name() != record.repository.admin_name() {
            return;
        }
        let dir = self.operation_dir(id);
        let Some(completed_at) = self.completed_at(id) else { return };
        if !expired(now, completed_at) {
            return;
        }
        if storage::validate_dir(&dir).is_err() {
            return;
        }
        let doomed = self.root().join(format!(".{id}.expired"));
        if fs::rename(&dir, &doomed).is_err() {
            return;
        }
        let _ = storage::sync_dir(self.root());
        remove(&doomed, self.root());
    }

    /// When operation `id` completed: its report's last timeline event, or
    /// else the folder's modification time. `None` when neither is readable.
    fn completed_at(&self, id: &OperationId) -> Option<SystemTime> {
        let dir = self.operation_dir(id);
        let from_report = storage::read_private_file(&dir.join(REPORT_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Timeline>(&bytes).ok())
            .and_then(|report| report.timeline.last().map(|event| event.at_ms))
            .and_then(|at_ms| UNIX_EPOCH.checked_add(Duration::from_millis(at_ms)));
        from_report.or_else(|| storage::validate_dir(&dir).ok()?.modified().ok())
    }
}

/// Strictly more than [`RETENTION`] has passed from `at` to `now`.
fn expired(now: SystemTime, at: SystemTime) -> bool {
    now.duration_since(at).is_ok_and(|age| age > RETENTION)
}

fn remove(path: &Path, root: &Path) {
    if fs::remove_dir_all(path).is_ok() {
        let _ = storage::sync_dir(root);
    }
}
