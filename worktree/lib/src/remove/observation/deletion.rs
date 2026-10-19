//! Checked deletion of a quarantined checkout.
//!
//! [`delete_content`] removes exactly the entries of the operation's
//! [`DeletionInventory`], children before directories and `.git` last, and
//! stops at the next entry boundary when anything it did not do shows up:
//!
//! - every entry is compared with its inventory record immediately before it
//!   is unlinked, through a handle on its parent directory whose own identity
//!   was checked on the way down, so a directory replaced by a link is never
//!   descended through and an approved link is removed, never its target;
//! - deletion goes one depth level at a time; each level is journaled in
//!   batches of one directory's entries (at most
//!   [`MAX_BATCH_ENTRIES`](crate::remove::recovery::journal::MAX_BATCH_ENTRIES)),
//!   and the observer is drained after each level (and every
//!   [`DeletionOptions::drain_every`] entries), so changes below a directory
//!   are delivered before that directory is removed (FSEvents may otherwise
//!   report the directory's removal in their place);
//! - each drained event is resolved by checking its exact path: an entry this
//!   operation deleted must still be absent, an approved entry not yet
//!   reached must still match its record, and anything else is activity;
//! - any [`CoverageLost`] (dropped notifications, a lost watch, a full queue,
//!   or a delivery wait that stalled or reached its ceiling) stops the
//!   operation, as does an interrupt request; the observer records the loss
//!   at its deletion stage, with the entries processed since the previous
//!   drain.
//!
//! The checks narrow the window in which a concurrent write can be lost; they
//! cannot close it. A write that lands between an entry's final check and its
//! unlink is deleted with it, and a writer can start after the final check.
//! [`ReportNote::ConcurrentWriteLimit`] states this in every report of a
//! removal that deleted anything.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use cap_std::ambient_authority;
use cap_std::fs::Dir;
use sniff::filesystem::query::NativeString;

use super::inventory::{self, Change, Content, DeletionInventory, InventoryEntry, InventoryRefusal, LookError, Shape};
use super::{CoverageLost, EventKind, Observation, ObservationStage, Scope};
use crate::remove::outcome::{ActivityKind, ActivityLocation, ObservedActivity, OperationReport, RemovalOutcome, ReportNote};
use crate::remove::recovery::journal::{EntryKind, JournalSink, JournalWriteError, JournalWriter, MAX_BATCH_ENTRIES, PlannedEntry, ResultState};

/// How many entries may be processed between two drains: 512 on macOS, 2,048
/// elsewhere. The interval is checked after each journal batch, so a drain
/// can come up to one batch minus one entry later.
///
/// macOS drains more often because each drain also waits for FSEvents to
/// catch up, which spreads a large deletion's events out: bursts of about
/// 7,000 deletions a second made FSEvents drop events. That pacing lowers
/// the pressure but cannot rule out a drop, which FSEvents shares with every
/// other process. A deleted entry is not a count of queued platform events,
/// since FSEvents coalesces and delivers late.
pub const DEFAULT_DRAIN_EVERY: usize = if cfg!(target_os = "macos") { 512 } else { 2048 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletionOptions {
    /// Entries deleted before the observer is drained mid-level. Drains also
    /// happen after every level.
    pub drain_every: usize,
    /// Whether an event's kinds are its path's whole recent history, as
    /// FSEvents reports them: a path this operation deleted can carry
    /// `Created` or `Modified` beside `Removed`. Elsewhere only `Removed` and
    /// `Metadata` (link count) explain a deleted file's events.
    pub cumulative_kinds: bool,
}

impl Default for DeletionOptions {
    fn default() -> Self {
        Self { drain_every: DEFAULT_DRAIN_EVERY, cumulative_kinds: cfg!(target_os = "macos") }
    }
}

/// Why deletion stopped before the quarantine was gone. Everything not
/// deleted is still in place.
#[derive(Debug, thiserror::Error)]
pub enum Stop {
    /// Something other than this operation changed either location.
    #[error("activity was observed at the checkout's locations")]
    Activity(Vec<ObservedActivity>),
    /// An event that this operation's own deletion and a concurrent write
    /// would both explain.
    #[error("an observed change could not be told apart from this removal's own")]
    Uncertain(Vec<ObservedActivity>),
    #[error("activity observation was lost: {0}")]
    CoverageLost(CoverageLost),
    #[error("the removal was interrupted")]
    Interrupted,
    #[error("{} could not be removed: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
    /// The journal could not record a batch; nothing more is deleted.
    #[error(transparent)]
    Journal(JournalWriteError),
}

impl Stop {
    /// The typed outcome of an operation that stopped this way.
    pub fn outcome(&self) -> RemovalOutcome {
        match self {
            Self::Activity(_) => RemovalOutcome::ActivityDetected,
            Self::Uncertain(_) | Self::CoverageLost(_) => RemovalOutcome::StateUnverified,
            Self::Interrupted | Self::Io { .. } | Self::Journal(_) => RemovalOutcome::CleanupFailed,
        }
    }
}

/// What [`delete_content`] did. Paths are relative to the quarantine.
#[derive(Debug)]
pub struct Deletion {
    /// Removed by this operation, in order.
    pub deleted: Vec<PathBuf>,
    /// Already gone when this operation reached them.
    pub externally_absent: Vec<PathBuf>,
    /// `None` when the quarantine is verified gone and nothing is at the
    /// original path.
    pub stop: Option<Stop>,
}

impl OperationReport {
    /// Records what deletion did: the deleted entries (absolute, capped like
    /// the other lists), the concurrent-write limit once anything was
    /// deleted, and any activity that stopped it.
    pub fn record_deletion(&mut self, quarantine: &Path, deletion: &Deletion) {
        self.set_deleted(deletion.deleted.iter().map(|path| NativeString::from(quarantine.join(path).as_path())).collect());
        if !deletion.deleted.is_empty() && !self.notes.contains(&ReportNote::ConcurrentWriteLimit) {
            self.notes.push(ReportNote::ConcurrentWriteLimit);
        }
        if let Some(Stop::Activity(activity) | Stop::Uncertain(activity)) = &deletion.stop {
            for observed in activity {
                self.push_activity(observed.clone());
            }
        }
    }
}

/// Deletes the inventory of the checkout `observation` watches, which must
/// already be at the quarantine, then the quarantine directory itself, and
/// confirms that neither location exists. `journal` records every batch.
/// `interrupted` is polled at every entry boundary and by every drain's wait.
pub fn delete_content<S: JournalSink>(
    observation: &mut Observation,
    journal: &mut JournalWriter<S>,
    interrupted: &dyn Fn() -> bool,
    options: DeletionOptions,
) -> Deletion {
    delete_content_with(observation, journal, interrupted, options, &mut |_| {})
}

/// [`delete_content`] with `before_batch`, called with each batch number
/// before its intent is written, for tests that act at a known point.
pub(crate) fn delete_content_with<S: JournalSink>(
    observation: &mut Observation,
    journal: &mut JournalWriter<S>,
    interrupted: &dyn Fn() -> bool,
    options: DeletionOptions,
    before_batch: &mut dyn FnMut(u64),
) -> Deletion {
    let inventory = std::sync::Arc::clone(journal.inventory());
    let quarantine = observation.targets().quarantine().to_path_buf();
    observation.enter_stage(ObservationStage::Deletion);
    let mut engine = Engine {
        quarantine: &quarantine,
        inventory: &inventory,
        observation,
        journal,
        interrupted,
        options,
        before_batch,
        batches: 0,
        gone: HashSet::new(),
        root_removed: false,
        since_drain: 0,
        deleted: Vec::new(),
        externally_absent: Vec::new(),
    };
    let stop = engine.run().err();
    Deletion { deleted: engine.deleted, externally_absent: engine.externally_absent, stop }
}

struct Engine<'a, S: JournalSink> {
    quarantine: &'a Path,
    inventory: &'a DeletionInventory,
    observation: &'a mut Observation,
    journal: &'a mut JournalWriter<S>,
    interrupted: &'a dyn Fn() -> bool,
    options: DeletionOptions,
    before_batch: &'a mut dyn FnMut(u64),
    batches: u64,
    /// Entries this operation deleted or found already gone.
    gone: HashSet<PathBuf>,
    root_removed: bool,
    since_drain: usize,
    deleted: Vec<PathBuf>,
    externally_absent: Vec<PathBuf>,
}

/// The entries of one directory, by depth then directory, deepest first.
type Levels<'a> = BTreeMap<usize, BTreeMap<PathBuf, Vec<&'a InventoryEntry>>>;

impl<S: JournalSink> Engine<'_, S> {
    fn run(&mut self) -> Result<(), Stop> {
        let root = self.open_root()?;
        // Anything between the move and now.
        self.drain_and_check(&root)?;

        let (content, git): (Vec<&InventoryEntry>, Vec<&InventoryEntry>) =
            self.inventory.entries().iter().partition(|entry| !entry.path.as_path().starts_with(".git"));
        for entries in [content, git] {
            for (_, directories) in levels(&entries).into_iter().rev() {
                for (_, children) in directories {
                    for batch in children.chunks(MAX_BATCH_ENTRIES) {
                        self.batch(&root, batch)?;
                        if self.since_drain >= self.options.drain_every {
                            self.drain_and_check(&root)?;
                        }
                    }
                }
                self.drain_and_check(&root)?;
            }
        }

        // Closed first: Windows cannot remove a directory this process holds.
        drop(root);
        self.remove_root()?;
        self.drain_and_check_optional(None)?;
        self.confirm_locations()
    }

    fn open_root(&self) -> Result<Dir, Stop> {
        inventory::open_root(self.quarantine, Some(self.inventory.root())).map(|(dir, _)| dir).map_err(|refusal| match refusal {
            InventoryRefusal::Uninspectable { path, source } => Stop::Io { path, source },
            _ => Stop::Activity(vec![activity(ActivityLocation::Quarantine, self.quarantine, ActivityKind::Replaced, None)]),
        })
    }

    /// Checks and deletes one batch: entries of one directory, all at one
    /// depth.
    fn batch(&mut self, root: &Dir, entries: &[&InventoryEntry]) -> Result<(), Stop> {
        if (self.interrupted)() {
            return Err(Stop::Interrupted);
        }
        self.batches += 1;
        (self.before_batch)(self.batches);
        let first = entries[0].path.as_path();
        let parent = match self.inventory.open_parent(root, self.quarantine, first) {
            Ok(Ok(parent)) => parent,
            Ok(Err((path, change))) => return Err(self.changed(&path, change)),
            Err(source) => return Err(Stop::Io { path: self.quarantine.join(first.parent().unwrap_or(first)), source }),
        };
        let planned = entries
            .iter()
            .map(|entry| PlannedEntry { path: entry.path.clone(), kind: EntryKind::of(&entry.shape) })
            .collect();
        self.journal.begin_batch(planned).map_err(Stop::Journal)?;

        let mut states = vec![ResultState::Kept; entries.len()];
        let mut stop = None;
        for (state, entry) in states.iter_mut().zip(entries) {
            if (self.interrupted)() {
                stop = Some(Stop::Interrupted);
                break;
            }
            let relative = entry.path.as_path();
            match self.delete_one(&parent, entry) {
                Ok(()) => {
                    *state = ResultState::Deleted;
                    self.deleted.push(relative.to_path_buf());
                    self.gone.insert(relative.to_path_buf());
                }
                Err(Outcome::Absent) => {
                    *state = ResultState::ExternallyAbsent;
                    self.externally_absent.push(relative.to_path_buf());
                    self.gone.insert(relative.to_path_buf());
                    stop = Some(self.changed(relative, Change::Vanished));
                    break;
                }
                Err(Outcome::Changed(change)) => {
                    stop = Some(self.changed(relative, change));
                    break;
                }
                Err(Outcome::NotEmpty) => {
                    stop = Some(Stop::Activity(vec![activity(
                        ActivityLocation::Quarantine,
                        &self.quarantine.join(relative),
                        ActivityKind::Created,
                        None,
                    )]));
                    break;
                }
                Err(Outcome::Failed(source)) => {
                    stop = Some(Stop::Io { path: self.quarantine.join(relative), source });
                    break;
                }
            }
        }
        self.since_drain += entries.len();
        let recorded = self.journal.finish_batch(&states);
        match (stop, recorded) {
            (Some(stop), _) => Err(stop),
            (None, Err(error)) => Err(Stop::Journal(error)),
            (None, Ok(())) => Ok(()),
        }
    }

    /// Revalidates `entry` in `parent` and unlinks it.
    fn delete_one(&self, parent: &Dir, entry: &InventoryEntry) -> Result<(), Outcome> {
        let relative = entry.path.as_path();
        let name = relative.file_name().expect("inventory paths have names");
        let full = self.quarantine.join(relative);
        let hash = matches!(entry.shape, Shape::File { content: Content::Blake3(_), .. });
        let now = match inventory::look(parent, name, &full, hash) {
            Ok(Some(now)) => now,
            Ok(None) => return Err(Outcome::Absent),
            Err(LookError::Ineligible(_)) => return Err(Outcome::Changed(Change::Replaced)),
            Err(LookError::Io(error)) => return Err(Outcome::Failed(error)),
        };
        if let Some(change) = inventory::compare(entry, &now) {
            return Err(Outcome::Changed(change));
        }
        let removed = match entry.shape {
            Shape::File { .. } => parent.remove_file(name),
            Shape::Directory { .. } => parent.remove_dir(name),
            Shape::Link { .. } => remove_link(parent, name),
        };
        removed.map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => Outcome::Absent,
            io::ErrorKind::DirectoryNotEmpty => Outcome::NotEmpty,
            _ => Outcome::Failed(error),
        })
    }

    fn remove_root(&mut self) -> Result<(), Stop> {
        if (self.interrupted)() {
            return Err(Stop::Interrupted);
        }
        let targets = self.observation.targets();
        let (parent_path, name) = (targets.parent().to_path_buf(), targets.quarantine().file_name().expect("named").to_os_string());
        let io_error = |source| Stop::Io { path: self.quarantine.to_path_buf(), source };
        let parent = Dir::open_ambient_dir(&parent_path, ambient_authority()).map_err(io_error)?;
        match inventory::look(&parent, &name, self.quarantine, false) {
            Ok(Some(now)) if now.identity == *self.inventory.root() && matches!(now.shape, Shape::Directory { .. }) => {}
            Ok(Some(_)) | Err(LookError::Ineligible(_)) => {
                return Err(Stop::Activity(vec![activity(ActivityLocation::Quarantine, self.quarantine, ActivityKind::Replaced, None)]));
            }
            Ok(None) => {
                return Err(Stop::Activity(vec![activity(ActivityLocation::Quarantine, self.quarantine, ActivityKind::Vanished, None)]));
            }
            Err(LookError::Io(source)) => return Err(io_error(source)),
        }
        match parent.remove_dir(&name) {
            Ok(()) => {
                self.root_removed = true;
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => Err(Stop::Activity(self.reappeared())),
            Err(source) => Err(io_error(source)),
        }
    }

    /// What is in the quarantine after every approved entry is gone.
    fn reappeared(&self) -> Vec<ObservedActivity> {
        let names: Vec<PathBuf> = std::fs::read_dir(self.quarantine)
            .map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()).collect())
            .unwrap_or_default();
        if names.is_empty() {
            return vec![activity(ActivityLocation::Quarantine, self.quarantine, ActivityKind::Created, None)];
        }
        names.iter().map(|path| activity(ActivityLocation::Quarantine, path, ActivityKind::Created, None)).collect()
    }

    fn drain_and_check(&mut self, root: &Dir) -> Result<(), Stop> {
        self.drain_and_check_optional(Some(root))
    }

    /// Drains the observer and resolves every event against the actual state
    /// of its exact path. `root` is `None` once the quarantine is removed.
    fn drain_and_check_optional(&mut self, root: Option<&Dir>) -> Result<(), Stop> {
        let processed = std::mem::take(&mut self.since_drain) as u64;
        let drain = self.observation.drain_after(processed, self.interrupted);
        if let Some(lost) = drain.lost {
            return Err(Stop::CoverageLost(lost));
        }
        if drain.interrupted {
            return Err(Stop::Interrupted);
        }
        let mut by_scope: HashMap<Scope, (Vec<EventKind>, SystemTime)> = HashMap::new();
        let mut order = Vec::new();
        for event in drain.events {
            let scope = match event.scope {
                Scope::Quarantine(relative) if !relative.as_os_str().is_empty() => Scope::Tree(relative),
                Scope::Quarantine(_) => Scope::Tree(PathBuf::new()),
                scope => scope,
            };
            let slot = by_scope.entry(scope.clone()).or_insert_with(|| {
                order.push(scope);
                (Vec::new(), event.received)
            });
            slot.0.push(event.kind);
        }

        let mut activities = Vec::new();
        let mut uncertain = Vec::new();
        for scope in order {
            let (kinds, received) = &by_scope[&scope];
            let received = Some(*received);
            match &scope {
                Scope::Admin(name) => {
                    let path = self.observation.targets().admin().join(name);
                    activities.push(activity(ActivityLocation::Admin, &path, ActivityKind::Modified, received));
                }
                Scope::Original(relative) => {
                    let original = self.observation.targets().original().to_path_buf();
                    let moved_away = relative.as_os_str().is_empty()
                        && own_move(kinds, self.options.cumulative_kinds)
                        && absent(&original);
                    if !moved_away {
                        activities.push(activity(ActivityLocation::Original, &original.join(relative), ActivityKind::Created, received));
                    }
                }
                Scope::Tree(relative) if relative.as_os_str().is_empty() => {
                    let intact = match root {
                        Some(_) => inventory::open_root(self.quarantine, Some(self.inventory.root())).is_ok(),
                        None => self.root_removed && absent(self.quarantine),
                    };
                    if !intact {
                        activities.push(activity(ActivityLocation::Quarantine, self.quarantine, ActivityKind::Replaced, received));
                    }
                }
                Scope::Tree(relative) => {
                    let full = self.quarantine.join(relative);
                    match self.resolve(root, relative, kinds) {
                        Resolution::Explained => {}
                        Resolution::Activity(kind) => activities.push(activity(ActivityLocation::Quarantine, &full, kind, received)),
                        Resolution::Uncertain => uncertain.push(activity(ActivityLocation::Quarantine, &full, ActivityKind::Modified, received)),
                        Resolution::Unreadable(source) => return Err(Stop::Io { path: full, source }),
                    }
                }
                Scope::Quarantine(_) => unreachable!("folded into Tree above"),
            }
        }
        if !activities.is_empty() {
            activities.extend(uncertain);
            return Err(Stop::Activity(activities));
        }
        if !uncertain.is_empty() {
            return Err(Stop::Uncertain(uncertain));
        }
        Ok(())
    }

    /// Whether the events `kinds` at `relative` are this operation's own.
    fn resolve(&self, root: Option<&Dir>, relative: &Path, kinds: &[EventKind]) -> Resolution {
        let deleted_here = relative.ancestors().find(|ancestor| self.gone.contains(*ancestor));
        if let Some(deleted) = deleted_here {
            if !absent(&self.quarantine.join(relative)) {
                return Resolution::Activity(ActivityKind::Created);
            }
            // Removing a directory's children changes the directory (Windows
            // reports that as a modification of it).
            let was_directory = deleted == relative
                && self.inventory.get(relative).is_some_and(|entry| matches!(entry.shape, Shape::Directory { .. }));
            let own = |kind: &EventKind| {
                matches!(kind, EventKind::Removed | EventKind::Metadata) || (was_directory && *kind == EventKind::Modified)
            };
            let explained = if self.options.cumulative_kinds {
                kinds.contains(&EventKind::Removed) || kinds.iter().all(own)
            } else {
                kinds.iter().all(own)
            };
            return if deleted != relative {
                // Children are deleted before their directory, so a path
                // below a deleted entry that is not itself deleted is new
                // content that came and went.
                Resolution::Activity(ActivityKind::Created)
            } else if explained {
                Resolution::Explained
            } else {
                Resolution::Uncertain
            };
        }
        let Some(root) = root else {
            return Resolution::Activity(ActivityKind::Created);
        };
        match self.inventory.check(root, self.quarantine, relative) {
            Ok(None) if self.inventory.get(relative).is_some() => Resolution::Explained,
            // Never recorded and absent: something came and went.
            Ok(None) => Resolution::Activity(ActivityKind::Created),
            Ok(Some(change)) => Resolution::Activity(activity_kind(change)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Resolution::Activity(ActivityKind::Vanished),
            Err(error) => Resolution::Unreadable(error),
        }
    }

    /// Neither the quarantine nor anything at the original path exists.
    fn confirm_locations(&self) -> Result<(), Stop> {
        let original = self.observation.targets().original();
        let mut activities = Vec::new();
        if !absent(self.quarantine) {
            activities.push(activity(ActivityLocation::Quarantine, self.quarantine, ActivityKind::Created, None));
        }
        if !absent(original) {
            activities.push(activity(ActivityLocation::Original, original, ActivityKind::Created, None));
        }
        if activities.is_empty() { Ok(()) } else { Err(Stop::Activity(activities)) }
    }

    fn changed(&self, relative: &Path, change: Change) -> Stop {
        Stop::Activity(vec![activity(ActivityLocation::Quarantine, &self.quarantine.join(relative), activity_kind(change), None)])
    }
}

enum Outcome {
    Absent,
    Changed(Change),
    NotEmpty,
    Failed(io::Error),
}

enum Resolution {
    Explained,
    Activity(ActivityKind),
    Uncertain,
    Unreadable(io::Error),
}

/// Whether `kinds` at the original path describe the operation's own move
/// away from it (or, with cumulative kinds, a history that includes it).
pub(crate) fn own_move(kinds: &[EventKind], cumulative_kinds: bool) -> bool {
    let away = |kind: &EventKind| matches!(kind, EventKind::MovedFrom | EventKind::Removed);
    if cumulative_kinds { kinds.iter().any(away) } else { kinds.iter().all(away) }
}

fn levels<'a>(entries: &[&'a InventoryEntry]) -> Levels<'a> {
    let mut levels: Levels<'a> = BTreeMap::new();
    for entry in entries {
        let path = entry.path.as_path();
        let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
        levels.entry(path.components().count()).or_default().entry(parent).or_default().push(entry);
    }
    levels
}

fn activity_kind(change: Change) -> ActivityKind {
    match change {
        Change::Appeared => ActivityKind::Created,
        Change::Vanished => ActivityKind::Vanished,
        Change::Replaced => ActivityKind::Replaced,
        Change::Modified => ActivityKind::Modified,
    }
}

fn activity(location: ActivityLocation, path: &Path, kind: ActivityKind, received: Option<SystemTime>) -> ObservedActivity {
    let received = received.unwrap_or_else(SystemTime::now);
    let received_at_ms = received.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64);
    ObservedActivity { location, path: NativeString::from(path), kind, received_at_ms }
}

/// Nothing exists at `path`, not even a dangling link.
fn absent(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

/// Removes the link `name` itself. On Windows a directory symlink or
/// junction is removed as a directory; its target is never touched.
fn remove_link(parent: &Dir, name: &OsStr) -> io::Result<()> {
    #[cfg(windows)]
    {
        use cap_std::fs::MetadataExt;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
        if parent.symlink_metadata(name)?.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0 {
            return parent.remove_dir(name);
        }
    }
    parent.remove_file(name)
}

#[cfg(test)]
mod tests;
