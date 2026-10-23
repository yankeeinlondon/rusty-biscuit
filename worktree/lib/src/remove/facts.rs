//! Everything the `wt remove` confirmation question shows, gathered in one
//! pass before anything is changed: what is at the worktree's path
//! ([`Target`]), each risk the user should judge ([`Warning`], lock first),
//! information that is not a risk ([`Note`]), and the one condition that
//! refuses before any question ([`Blocker`], Windows only).
//!
//! [`gather`] reads the host only through [`Sources`], so tests inject every
//! input. It makes exactly one usage query and at most one `git status`;
//! their results are kept ([`Facts::usage`], [`Facts::inventory`]) for
//! enrichment and for the late-change list, never re-queried here.
//!
//! The rules come from `2026-10-22-confirmed-remove-always-deletes` (R1–R17
//! and its notes N7, N8, N11).
//!
//! Warning text is plain and leaves untrusted strings (process names, lock
//! reasons, paths, Git messages) raw; the caller escapes the rendered text
//! as a whole. A count is given only when it is known.

use std::fmt;
use std::path::{Path, PathBuf};

use sniff::filesystem::query::{CoverageStatus, EvidenceKind, Mechanism, NativeString, Outcome, PathUsageReport};

use crate::availability::{self, OtherCondition};
use crate::copy_record::canonical_worktree_path;
use crate::worktree::{WorktreeEntry, parse_worktree_list};

use super::admin_entry::{AdminEntry, AssociationError, admin_entry_for, read_back_reference};
use super::handoff::is_within;
use super::included::{Mark, classify_included};
use super::inventory::{Inventory, collect_inventory};
use super::mounts::{self, Mount, MountInfo};
use super::quarantine::association::leads_to_record;
use super::repair::RepairGit;
use super::submodules::{self, Finding, GitDirOwner, SubmoduleReport};
use super::usage::{CoverageSummary, EvidenceLocation, Holder, OwnedHandles, UsageRefusal, UsageSource, assess};
use super::{CheckoutState, PrepareRefusal, prepare};

/// How many names a list of files shows before "and N more".
pub const NAMED_FILES: usize = 5;

/// The question's own sentence about R14: what changes after the question is
/// answered is deleted too. It belongs under the warnings whenever a
/// question is shown, so no single warning carries it.
pub const LATE_CHANGES_NOTE: &str = "Files created or changed before deletion finishes are deleted too.";

/// The caller of `wt remove`, as the CLI observed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    /// The working directory `wt` inherited from the shell.
    pub cwd: PathBuf,
    /// The shell wrapper announced itself, so it will `cd` out first.
    pub shell_wrapper: bool,
    /// The parent process's name (N11), when known; names the shell in the
    /// Windows refusal.
    pub shell: Option<String>,
    /// The parent process's PID, when known. A parent that only stands in
    /// the checkout (a working directory, or on Windows, where that is how
    /// Sniff sees one, an open handle on a directory) is the caller's own
    /// shell, which the shell-inside warning (or the wrapper's move) covers,
    /// so it is not listed as a program using the worktree.
    pub parent: Option<u32>,
}

/// Host facts that are not about the worktree itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Host {
    /// Roots of folders a sync client mirrors to the cloud (OneDrive).
    pub synced_folders: Vec<PathBuf>,
}

impl Host {
    /// This host's facts: on Windows the OneDrive roots named by the
    /// `OneDrive`, `OneDriveCommercial`, and `OneDriveConsumer` variables;
    /// elsewhere none.
    pub fn detect() -> Self {
        #[cfg(windows)]
        {
            let mut synced_folders: Vec<PathBuf> = Vec::new();
            for name in ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"] {
                if let Some(root) = std::env::var_os(name).filter(|root| !root.is_empty()).map(PathBuf::from)
                    && !synced_folders.contains(&root)
                {
                    synced_folders.push(root);
                }
            }
            Self { synced_folders }
        }
        #[cfg(not(windows))]
        Self::default()
    }
}

/// Everything [`gather`] reads.
pub struct Sources<'a> {
    /// The base (main) checkout, from which every Git command runs.
    pub base: &'a Path,
    /// The resolved removal target; never the main checkout.
    pub entry: &'a WorktreeEntry,
    pub usage: &'a dyn UsageSource,
    pub mounts: &'a dyn MountInfo,
    /// Runs the `.git` repair [`prepare`] may make before consent.
    pub git: &'a dyn RepairGit,
    pub caller: &'a Caller,
    pub host: &'a Host,
}

/// What is at the worktree's recorded path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A directory, inspected as the checkout.
    Checkout { path: PathBuf },
    /// The recorded path is a link. When `own`, `resolved` is this
    /// worktree's real checkout, inspected as the checkout, and both are
    /// deleted; otherwise only the link and Git's record are.
    Link { link: PathBuf, resolved: LinkTarget, own: bool },
    /// The directory is gone; only Git's record remains.
    Missing,
    /// The checkout was there when gathering began and gone after the usage
    /// query (R17): already removed. Only the record's warnings (the lock)
    /// are kept.
    Vanished,
    /// A file is where the directory should be.
    NotADirectory,
    /// The path could not be inspected.
    Uninspectable,
}

/// Where a link leads, as resolved once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// The existing location, canonical.
    Resolved(PathBuf),
    /// The link's text; nothing exists there.
    Dangling(PathBuf),
    /// The link could not be read, with the reason.
    Unreadable(String),
}

impl fmt::Display for LinkTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkTarget::Resolved(path) => write!(f, "{}", path.display()),
            LinkTarget::Dangling(path) => write!(f, "{}, which does not exist", path.display()),
            LinkTarget::Unreadable(reason) => write!(f, "a location that cannot be read ({reason})"),
        }
    }
}

/// One risk shown in the confirmation question. Its [`Display`](fmt::Display)
/// is one plain item that names what was seen and what deleting will do to
/// it; [`Facts::warnings`] keeps them in the order they are shown.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// `git worktree lock` (R3): the trimmed content of `<admin>/locked`,
    /// `None` when empty. Its modification time is never shown.
    Locked { reason: Option<String> },
    /// Git's record of the worktree could not be found or read, so whether
    /// it is locked is unknown.
    RecordUnreadable { reason: String },
    /// The recorded path is a link (R9).
    Link { resolved: LinkTarget, own: bool },
    /// Git cannot read the checkout (R10).
    GitUnreadable { reason: String },
    /// A file is where the directory should be (R11).
    PathOccupied { path: PathBuf },
    /// The path could not be inspected (R11).
    Uninspectable { path: PathBuf, reason: String },
    /// The caller's shell stands in the checkout without the wrapper (R15,
    /// Unix; Windows refuses with [`Blocker::ShellInside`] instead).
    ShellInside,
    /// Other processes are using the checkout (R1). `root` is the checkout
    /// as the usage query resolved it; paths are shown relative to it.
    Processes { root: PathBuf, programs: Vec<Holder> },
    /// The usage query failed or was partial (N8).
    UsageIncomplete { reason: String },
    /// Dirty entries from `git status`, repository-relative.
    Uncommitted { files: Vec<PathBuf> },
    /// Ignored files `.worktreeinclude` selects that are new, changed since
    /// they were copied in, or could not be compared with their source.
    IncludedFiles { files: Vec<(PathBuf, Mark)> },
    /// The `.worktreeinclude` rules or their copies could not be checked.
    IncludedUnchecked { reason: String },
    /// `git status` failed (R13); no count is known.
    Unchecked { reason: String },
    /// The directory is gone and its surviving index stages these changes.
    StagedChanges { files: Vec<PathBuf> },
    /// The directory is gone and its index is missing or unreadable (R12).
    StagedUnknown { reason: String },
    /// Work that exists only in a submodule (R2); `dirty` is relative to the
    /// submodule.
    SubmoduleAtRisk { path: PathBuf, dirty: Vec<PathBuf>, unique_commits: u64, no_remote: bool },
    /// A submodule (`None`: all of them) could not be inspected.
    SubmoduleUnchecked { path: Option<PathBuf>, reason: String },
    /// Filesystems mounted inside the checkout (R7), outermost first.
    NestedMounts { mounts: Vec<Mount> },
    /// The mount table could not be read, so nested mounts (and drvfs) are
    /// unknown.
    MountsUnchecked { reason: String },
    /// The checkout is on a WSL drvfs mount of a Windows drive (R4).
    Drvfs { mount_point: PathBuf },
    /// The checkout is inside a synced folder (R6).
    SyncedFolder { root: PathBuf },
}

/// Information shown with the question that is no risk to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// A submodule keeps its Git data outside the worktree; it is retained.
    SubmoduleGitDataRetained { path: PathBuf, git_dir: PathBuf },
    /// How the included files were compared, when not as usual (a copy
    /// record that could not be read, for example).
    IncludedCheck { message: String },
}

/// Refuses before any question is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocker {
    /// Windows (R15, N11): the shell running `wt` stands in the checkout, and
    /// its working directory blocks deleting it. `shell` is the parent
    /// process's name, when known.
    ShellInside { shell: Option<String> },
}

/// What [`gather`] found.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    pub target: Target,
    /// The worktree's Git record, when it was found.
    pub admin: Option<AdminEntry>,
    /// What [`prepare`] made of a checkout it accepted.
    pub prepared: Option<CheckoutState>,
    /// In display order, [`Warning::Locked`] first.
    pub warnings: Vec<Warning>,
    pub notes: Vec<Note>,
    /// When set, nothing else was gathered.
    pub blocker: Option<Blocker>,
    /// The `git status` set the question showed, for R14's later diff.
    pub inventory: Option<Inventory>,
    /// The one usage report, kept for enrichment.
    pub usage: Option<PathUsageReport>,
}

impl Facts {
    /// Whether a non-interactive run needs `--force` (N8): any warning.
    pub fn needs_force(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Gathers the facts about `sources.entry` in one pass.
///
/// Only the steps a removal would take before consent change anything: an
/// unlinked checkout's `.git` is repaired by [`prepare`] as before.
pub fn gather(sources: &Sources<'_>) -> Facts {
    let entry = sources.entry;
    let mut facts = Facts {
        target: Target::Missing,
        admin: None,
        prepared: None,
        warnings: Vec::new(),
        notes: Vec::new(),
        blocker: None,
        inventory: None,
        usage: None,
    };
    // The warnings about the path itself; shown after the lock.
    let mut state = Vec::new();
    // The directory to inspect, and whether Git can read it.
    let mut checkout: Option<PathBuf> = None;
    let mut git_readable = true;
    let mut record = None;

    // Git never marks a locked worktree prunable, so a locked one whose
    // directory is gone classifies as healthy; its absence is confirmed
    // here and it is prepared as the missing checkout it is.
    let marked;
    let prepared_entry = if entry.prunable.is_none()
        && matches!(availability::inspect(&entry.path), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    {
        marked = WorktreeEntry { prunable: Some(String::new()), ..entry.clone() };
        &marked
    } else {
        entry
    };
    match prepare(sources.base, prepared_entry, sources.git) {
        Ok(prepared) => {
            match &prepared {
                CheckoutState::Healthy => checkout = Some(entry.path.clone()),
                CheckoutState::Repaired(report) => {
                    checkout = Some(entry.path.clone());
                    record = Some(Ok(report.admin.clone()));
                }
                CheckoutState::Missing(missing) => {
                    record = Some(Ok(missing.admin.clone()));
                    if !missing.staged.is_empty() {
                        state.push(Warning::StagedChanges { files: missing.staged.iter().map(|e| e.path.clone()).collect() });
                    }
                }
            }
            facts.prepared = Some(prepared);
        }
        Err(PrepareRefusal::Unavailable(OtherCondition::Link)) => {
            let link = link_facts(sources.base, entry);
            if link.own
                && let LinkTarget::Resolved(resolved) = &link.resolved
            {
                checkout = Some(resolved.clone());
            }
            state.push(Warning::Link { resolved: link.resolved.clone(), own: link.own });
            facts.target = Target::Link { link: entry.path.clone(), resolved: link.resolved, own: link.own };
            record = Some(link.record);
        }
        Err(PrepareRefusal::Unavailable(OtherCondition::NotADirectory)) => {
            facts.target = Target::NotADirectory;
            state.push(Warning::PathOccupied { path: entry.path.clone() });
        }
        Err(PrepareRefusal::Unavailable(OtherCondition::PathUninspectable(reason))) => {
            facts.target = Target::Uninspectable;
            state.push(Warning::Uninspectable { path: entry.path.clone(), reason });
        }
        Err(PrepareRefusal::Unavailable(OtherCondition::GitEntryUninspectable(reason))) => {
            checkout = Some(entry.path.clone());
            git_readable = false;
            state.push(Warning::GitUnreadable { reason: format!("its .git can't be inspected: {reason}") });
        }
        Err(PrepareRefusal::Unavailable(OtherCondition::GitEntryPresent)) => {
            checkout = Some(entry.path.clone());
            git_readable = false;
            let reason = match entry.prunable.as_deref().map(str::trim) {
                Some(reason) if !reason.is_empty() => reason.to_string(),
                _ => "Git marks it prunable although its .git exists".to_string(),
            };
            state.push(Warning::GitUnreadable { reason });
        }
        Err(PrepareRefusal::Missing(refusal)) => {
            state.push(Warning::StagedUnknown { reason: refusal.to_string() });
        }
        Err(PrepareRefusal::Repair(refusal)) => {
            checkout = Some(entry.path.clone());
            git_readable = false;
            state.push(Warning::GitUnreadable { reason: refusal.to_string() });
        }
    }
    if let Some(path) = &checkout
        && !matches!(facts.target, Target::Link { .. })
    {
        facts.target = Target::Checkout { path: path.clone() };
    }

    let record = record.unwrap_or_else(|| admin_entry_for(sources.base, &entry.path).map_err(|error| error.to_string()));
    match &record {
        Ok(admin) => match lock_of(admin) {
            Ok(Some(reason)) => facts.warnings.push(Warning::Locked { reason }),
            Ok(None) => {}
            Err(reason) => facts.warnings.push(Warning::RecordUnreadable { reason }),
        },
        Err(reason) => facts.warnings.push(Warning::RecordUnreadable { reason: reason.clone() }),
    }
    facts.admin = record.ok();

    let Some(checkout) = checkout else {
        facts.warnings.extend(state);
        return facts;
    };

    if !sources.caller.shell_wrapper && is_within(&sources.caller.cwd, &checkout) {
        if cfg!(windows) {
            facts.warnings.clear();
            facts.blocker = Some(Blocker::ShellInside { shell: sources.caller.shell.clone() });
            return facts;
        }
        state.push(Warning::ShellInside);
    }

    let queried = sources.usage.query(&checkout, None);
    if matches!(availability::inspect(&checkout), Err(error) if error.kind() == std::io::ErrorKind::NotFound) {
        facts.target = Target::Vanished;
        return facts;
    }
    facts.warnings.extend(state);
    match queried {
        Ok(report) => {
            let coverage = CoverageSummary::of(&report);
            let mut programs = match assess(report.clone(), &OwnedHandles::default()) {
                Err(UsageRefusal::InUse(in_use)) => in_use.holders,
                _ => Vec::new(),
            };
            programs.retain(|holder| Some(holder.pid) != sources.caller.parent || !only_stands_in(holder));
            if !programs.is_empty() {
                facts.warnings.push(Warning::Processes { root: report.target.resolved.as_path().to_path_buf(), programs });
            }
            if let Some(reason) = missed(&coverage) {
                facts.warnings.push(Warning::UsageIncomplete { reason });
            }
            facts.usage = Some(report);
        }
        Err(error) => facts.warnings.push(Warning::UsageIncomplete { reason: error.to_string() }),
    }

    if git_readable {
        match collect_inventory(sources.base, &checkout) {
            Ok(mut inventory) => {
                if !inventory.dirty.is_empty() {
                    facts.warnings.push(Warning::Uncommitted { files: inventory.dirty.iter().map(|e| e.path.clone()).collect() });
                }
                match classify_included(sources.base, &checkout, entry.branch.as_deref()) {
                    Ok(included) => {
                        if !included.needs_consent.is_empty() {
                            facts.warnings.push(Warning::IncludedFiles { files: included.needs_consent.clone() });
                        }
                        facts.notes.extend(included.warnings.iter().map(|message| Note::IncludedCheck { message: message.clone() }));
                        inventory.included = included;
                    }
                    Err(error) => facts.warnings.push(Warning::IncludedUnchecked { reason: error.to_string() }),
                }
                facts.inventory = Some(inventory);
            }
            Err(error) => facts.warnings.push(Warning::Unchecked { reason: error.to_string() }),
        }
        if let Some(admin) = &facts.admin {
            let report = submodules::inspect(sources.base, admin, &checkout);
            submodule_findings(report, &mut facts.warnings, &mut facts.notes);
        }
    }

    // Native tables spell real paths (macOS `/private/var`, Windows long
    // names), so the checkout is resolved the same way before comparing.
    let real = biscuit_file::canonicalize_simplified(&checkout).unwrap_or_else(|_| checkout.clone());
    match sources.mounts.mounts() {
        Ok(table) => {
            let nested: Vec<Mount> = mounts::mounts_within(&table, &real).into_iter().cloned().collect();
            if !nested.is_empty() {
                facts.warnings.push(Warning::NestedMounts { mounts: nested });
            }
            if let Some(mount) = mounts::containing(&table, &real).filter(|mount| mount.is_drvfs()) {
                facts.warnings.push(Warning::Drvfs { mount_point: mount.mount_point.clone() });
            }
        }
        Err(error) => facts.warnings.push(Warning::MountsUnchecked { reason: error.to_string() }),
    }

    if let Some(root) = sources.host.synced_folders.iter().find(|root| is_within(&checkout, root)) {
        facts.warnings.push(Warning::SyncedFolder { root: root.clone() });
    }
    facts
}

/// Whether every use `holder` makes of the checkout is standing in a
/// directory of it: a working directory, or an open handle on a directory.
fn only_stands_in(holder: &Holder) -> bool {
    holder.evidence.iter().all(|evidence| match evidence.kind {
        EvidenceKind::WorkingDirectory => true,
        EvidenceKind::OpenHandle => match &evidence.location {
            EvidenceLocation::Path(path) => path.as_path().is_dir(),
            _ => false,
        },
        _ => false,
    })
}

/// `<admin>/locked`'s trimmed content (`Some(None)` when empty), `None`
/// when unlocked. Git treats any entry named `locked` as a lock.
fn lock_of(admin: &AdminEntry) -> Result<Option<Option<String>>, String> {
    let lock = admin.dir.join("locked");
    match availability::inspect(&lock) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", lock.display())),
        Ok(_) => {
            let reason = std::fs::read(&lock).ok().map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string());
            Ok(Some(reason.filter(|reason| !reason.is_empty())))
        }
    }
}

struct LinkFacts {
    resolved: LinkTarget,
    own: bool,
    record: Result<AdminEntry, String>,
}

/// Resolves the link at `entry.path` once and decides whether it leads to
/// this worktree's own checkout: forward, the target's `.git` names this
/// worktree's record; backward, that record's `gitdir` names the link, which
/// resolves to the target; and the target is no other listed checkout (the
/// main one included) and encloses none. Anything unverifiable is not own.
fn link_facts(base: &Path, entry: &WorktreeEntry) -> LinkFacts {
    let link = &entry.path;
    let resolved = match biscuit_file::canonicalize_simplified(link) {
        Ok(path) => LinkTarget::Resolved(path),
        Err(_) => match std::fs::read_link(link) {
            Ok(text) => LinkTarget::Dangling(text),
            Err(error) => LinkTarget::Unreadable(error.to_string()),
        },
    };
    let record = link_record(base, link);
    let own = match (&resolved, &record) {
        (LinkTarget::Resolved(target), Ok(admin)) => {
            leads_to_record(target, &admin.dir).is_ok() && is_unregistered_elsewhere(base, entry, target)
        }
        _ => false,
    };
    LinkFacts { resolved, own, record }
}

/// The record whose `gitdir` names the link itself. Records are compared by
/// their spelling with only the final component left unresolved, so the
/// record of a checkout the link leads to never matches.
fn link_record(base: &Path, link: &Path) -> Result<AdminEntry, String> {
    let candidates = match admin_entry_for(base, link) {
        Ok(admin) => vec![admin.dir],
        Err(AssociationError::Ambiguous(dirs)) => dirs,
        Err(error) => return Err(error.to_string()),
    };
    let wanted = spelled(link);
    candidates
        .into_iter()
        .find(|dir| {
            read_back_reference(dir).ok().and_then(|named| named.parent().and_then(spelled)) == wanted && wanted.is_some()
        })
        .map(|dir| AdminEntry { dir })
        .ok_or_else(|| AssociationError::NotFound.to_string())
}

/// `path` with its parent resolved and its final component as spelled.
fn spelled(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?;
    canonical_worktree_path(path.parent()?).ok().map(|parent| parent.join(name))
}

/// Whether `target` is neither another listed checkout nor an ancestor of
/// one; an unreadable listing proves nothing.
fn is_unregistered_elsewhere(base: &Path, entry: &WorktreeEntry, target: &Path) -> bool {
    let Ok(listing) = crate::git::worktree_list_from(base, base) else {
        return false;
    };
    parse_worktree_list(&listing)
        .iter()
        .filter(|listed| listed.path != entry.path)
        .map(|listed| biscuit_file::canonicalize_simplified(&listed.path).unwrap_or_else(|_| listed.path.clone()))
        .all(|listed| !listed.starts_with(target))
}

/// What the usage query missed, in plain words, or `None` when it saw
/// what this host lets it see. A failed or unattempted mechanism, discovery
/// that is unavailable, and a budget that ran out are misses. A mechanism
/// the platform never supports (macOS has no inventory of FSEvents
/// subscriptions) and one only partly checked because some processes cannot
/// be inspected are this host's ordinary limits on every run: counting them
/// would warn about every removal, and so refuse every non-interactive one.
fn missed(coverage: &CoverageSummary) -> Option<String> {
    let mut gaps = Vec::new();
    match coverage.outcome {
        Outcome::Usable => {}
        Outcome::Unavailable => gaps.push("process discovery is unavailable".to_string()),
        Outcome::Unsupported => gaps.push("process discovery is not supported here".to_string()),
    }
    if coverage.budget_exhausted {
        gaps.push("the time allowed for the check ran out".to_string());
    }
    for gap in coverage.gaps.iter().filter(|gap| matches!(gap.status, CoverageStatus::Failed | CoverageStatus::NotAttempted)) {
        let mut text = format!("{} {}", mechanism_name(gap.mechanism), status_name(gap.status));
        if let Some(reason) = &gap.reason {
            text.push_str(&format!(" ({reason})"));
        }
        gaps.push(text);
    }
    (!gaps.is_empty()).then(|| gaps.join("; "))
}

fn mechanism_name(mechanism: Mechanism) -> &'static str {
    match mechanism {
        Mechanism::ProcessEnumeration => "listing processes",
        Mechanism::TreeIdentity => "reading the worktree's files",
        Mechanism::OpenHandles => "open files",
        Mechanism::WorkingDirectories => "working directories",
        Mechanism::Inotify => "inotify watches",
        Mechanism::Fanotify => "fanotify watches",
        Mechanism::Fsevents => "FSEvents watches",
        Mechanism::LoadedModules => "loaded programs",
        Mechanism::DirectoryChangeSubscriptions => "directory watches",
        Mechanism::Polling => "polling watchers",
    }
}

fn status_name(status: CoverageStatus) -> &'static str {
    match status {
        CoverageStatus::Complete => "complete",
        CoverageStatus::Partial => "partly checked",
        CoverageStatus::Unsupported => "not supported",
        CoverageStatus::Failed => "failed",
        CoverageStatus::NotAttempted => "not checked",
    }
}

fn submodule_findings(report: SubmoduleReport, warnings: &mut Vec<Warning>, notes: &mut Vec<Note>) {
    if let Some(reason) = report.unlisted {
        warnings.push(Warning::SubmoduleUnchecked { path: None, reason });
    }
    for submodule in report.submodules {
        match submodule.finding {
            Finding::Clean => {}
            Finding::AtRisk { dirty, unique_commits, no_remote } => {
                warnings.push(Warning::SubmoduleAtRisk { path: submodule.path.clone(), dirty, unique_commits, no_remote });
            }
            Finding::Unchecked(reason) => warnings.push(Warning::SubmoduleUnchecked { path: Some(submodule.path.clone()), reason }),
        }
        if let GitDirOwner::External(git_dir) = submodule.git_dir {
            notes.push(Note::SubmoduleGitDataRetained { path: submodule.path, git_dir });
        }
    }
}

/// `count` with its noun, `plural` when not one.
fn counted(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

/// Up to [`NAMED_FILES`] names, then "and N more".
fn named(paths: impl IntoIterator<Item = String>) -> String {
    let paths: Vec<String> = paths.into_iter().collect();
    let mut text = paths.iter().take(NAMED_FILES).cloned().collect::<Vec<_>>().join(", ");
    if paths.len() > NAMED_FILES {
        text.push_str(&format!(", and {} more", paths.len() - NAMED_FILES));
    }
    text
}

fn shown(paths: &[PathBuf]) -> String {
    named(paths.iter().map(|path| path.display().to_string()))
}

/// One process and what it is doing in the checkout, for example
/// `cargo (pid 4123, started 14:02, run from WezTerm) is writing target/out`.
/// The one place a holder is rendered. An open handle reads "has … open"
/// unless writing was observed on it (R27).
pub fn describe_process(holder: &Holder, root: &Path) -> String {
    let mut doing = Vec::new();
    for (kind, writing) in [
        (EvidenceKind::OpenHandle, true),
        (EvidenceKind::OpenHandle, false),
        (EvidenceKind::WorkingDirectory, false),
        (EvidenceKind::LoadedModule, false),
        (EvidenceKind::WatchRegistration, false),
    ] {
        let mut places: Vec<String> = Vec::new();
        for evidence in holder.evidence.iter().filter(|evidence| evidence.kind == kind && evidence.writing == writing) {
            let place = location(&evidence.location, root);
            if !places.contains(&place) {
                places.push(place);
            }
        }
        let Some(first) = places.first() else { continue };
        let more = match places.len() - 1 {
            0 => String::new(),
            more => format!(" (and {more} more)"),
        };
        doing.push(match kind {
            EvidenceKind::OpenHandle if writing => format!("is writing {first}{more}"),
            EvidenceKind::OpenHandle => format!("has {first}{more} open"),
            EvidenceKind::WorkingDirectory => format!("is standing in {first}{more}"),
            EvidenceKind::LoadedModule => format!("is running from {first}{more}"),
            EvidenceKind::WatchRegistration => format!("is watching {first}{more}"),
        });
    }
    format!("{} {}", identify(holder, chrono::Local::now()), doing.join(", "))
}

/// The holder's name and context: `cargo (pid 4123, started 14:02, run from
/// WezTerm)`. Sniff's app name leads when it says more than the process
/// name (`Node.js JavaScript Runtime (node.exe, pid 8124)`); without either,
/// only the PID is known. `now` decides whether the start needs a date.
fn identify<Tz: chrono::TimeZone>(holder: &Holder, now: chrono::DateTime<Tz>) -> String
where
    Tz::Offset: fmt::Display,
{
    let name = holder.name.as_ref().or(holder.executable.as_ref()).map(|name| name.display().into_owned());
    let app = holder.app_name.as_ref().map(|app| app.display().into_owned()).filter(|app| {
        name.as_deref().is_none_or(|name| !program_key(app).eq_ignore_ascii_case(&program_key(name)))
    });
    let mut context = Vec::new();
    let lead = match (app, name) {
        (Some(app), Some(name)) => {
            context.push(name);
            app
        }
        (Some(app), None) => app,
        (None, Some(name)) => name,
        (None, None) => "A program".to_string(),
    };
    context.push(format!("pid {}", holder.pid));
    if let Some(start) = holder.start_time {
        let start = start.with_timezone(&now.timezone());
        let format = if start.date_naive() == now.date_naive() { "%H:%M" } else { "%b %-d %H:%M" };
        context.push(format!("started {}", start.format(format)));
    }
    if let Some(origin) = run_from(holder) {
        context.push(format!("run from {origin}"));
    }
    format!("{lead} ({})", context.join(", "))
}

/// Terminals, editors, and agents a user recognizes by the process names
/// they run under, compared without case or an `.exe` suffix.
const KNOWN_ORIGINS: &[(&str, &str)] = &[
    ("code", "VS Code"),
    ("code helper", "VS Code"),
    ("cursor", "Cursor"),
    ("zed", "Zed"),
    ("idea", "IntelliJ IDEA"),
    ("rustrover", "RustRover"),
    ("nvim", "Neovim"),
    ("vim", "Vim"),
    ("emacs", "Emacs"),
    ("terminal", "Terminal"),
    ("iterm2", "iTerm2"),
    ("wezterm-gui", "WezTerm"),
    ("kitty", "kitty"),
    ("alacritty", "Alacritty"),
    ("ghostty", "Ghostty"),
    ("windowsterminal", "Windows Terminal"),
    ("tmux", "tmux"),
    ("screen", "screen"),
    ("sshd", "an SSH session"),
    ("claude", "Claude Code"),
    ("codex", "Codex"),
    ("gemini", "Gemini CLI"),
    ("goose", "Goose"),
    ("opencode", "OpenCode"),
];

/// Where the holder was started from: the nearest ancestor that is a known
/// terminal, editor, or agent, else the nearest one Sniff gave an app name
/// (a macOS app bundle, a described Windows program). `None` says nothing,
/// which is more honest than naming a bare shell.
fn run_from(holder: &Holder) -> Option<String> {
    let known = holder.ancestors.iter().find_map(|ancestor| {
        [ancestor.name.as_ref(), ancestor.app_name.as_ref()].into_iter().flatten().find_map(|name| {
            let key = program_key(&name.display());
            KNOWN_ORIGINS.iter().find(|(process, _)| key.eq_ignore_ascii_case(process)).map(|(_, shown)| (*shown).to_string())
        })
    });
    known.or_else(|| holder.ancestors.iter().find_map(|ancestor| ancestor.app_name.as_ref().map(|app| app.display().into_owned())))
}

/// A program name without its directory or `.exe` suffix.
fn program_key(name: &str) -> String {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let len = file.len();
    if len > 4 && file[len - 4..].eq_ignore_ascii_case(".exe") { file[..len - 4].to_string() } else { file.to_string() }
}

fn location(location: &EvidenceLocation, root: &Path) -> String {
    let relative = |path: &NativeString| match path.as_path().strip_prefix(root) {
        Ok(rest) if rest.as_os_str().is_empty() => "the worktree".to_string(),
        Ok(rest) => rest.display().to_string(),
        Err(_) => path.display().into_owned(),
    };
    match location {
        EvidenceLocation::Path(path) => relative(path),
        EvidenceLocation::Aliases(paths) => paths.first().map_or_else(|| "a file".to_string(), relative),
        EvidenceLocation::Unnamed { observed: Some(observed), .. } => observed.display().into_owned(),
        EvidenceLocation::Unnamed { observed: None, .. } => "a file no longer named in the worktree".to_string(),
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::Locked { reason: Some(reason) } => {
                write!(f, "This worktree is locked (git worktree lock), with the reason: {reason}. Deleting it unlocks it.")
            }
            Warning::Locked { reason: None } => {
                write!(f, "This worktree is locked (git worktree lock), with no reason given. Deleting it unlocks it.")
            }
            Warning::RecordUnreadable { reason } => {
                write!(f, "Git's record of this worktree could not be read ({reason}), so whether it is locked is unknown.")
            }
            Warning::Link { resolved, own: true } => {
                write!(f, "The path is a link to the worktree's real location at {resolved}; both will be deleted.")
            }
            Warning::Link { resolved, own: false } => {
                write!(f, "The path is a link to {resolved}, which is not touched; only the link and Git's record are deleted.")
            }
            Warning::GitUnreadable { reason } => write!(
                f,
                "Git cannot read this worktree: {reason}. Its uncommitted files cannot be listed, and everything in it will be deleted."
            ),
            Warning::PathOccupied { path } => write!(
                f,
                "A file is where the worktree's directory should be ({}). It will be deleted, with Git's record.",
                path.display()
            ),
            Warning::Uninspectable { path, reason } => write!(
                f,
                "The worktree's path {} cannot be inspected: {reason}. Whatever is there will be deleted, with Git's record.",
                path.display()
            ),
            Warning::ShellInside => write!(f, "Your shell is inside this worktree; cd out of it afterward."),
            Warning::Processes { root, programs } => {
                let verb = if programs.len() == 1 { "is" } else { "are" };
                writeln!(f, "{} {verb} using files in this worktree:", counted(programs.len(), "program", "programs"))?;
                for program in programs {
                    writeln!(f, "  {}", describe_process(program, root))?;
                }
                write!(f, "Deleting now may make them fail.")
            }
            Warning::UsageIncomplete { reason } => {
                write!(f, "Some programs could not be checked ({reason}); one using this worktree may not be listed.")
            }
            Warning::Uncommitted { files } => write!(
                f,
                "{} will be lost: {}",
                counted(files.len(), "uncommitted file", "uncommitted files"),
                shown(files)
            ),
            Warning::IncludedFiles { files } => {
                let shown = named(files.iter().map(|(path, mark)| format!("{} ({})", path.display(), included_state(mark))));
                write!(
                    f,
                    "{} copied in by .worktreeinclude will be lost: {shown}",
                    counted(files.len(), "included file", "included files")
                )
            }
            Warning::IncludedUnchecked { reason } => write!(
                f,
                "Files copied in by .worktreeinclude could not be checked ({reason}); they will be deleted."
            ),
            Warning::Unchecked { reason } => write!(
                f,
                "Uncommitted files could not be checked ({reason}); whatever is there will be deleted."
            ),
            Warning::StagedChanges { files } => write!(
                f,
                "The worktree's directory is gone; {} in Git's record will be lost: {}",
                counted(files.len(), "staged change", "staged changes"),
                shown(files)
            ),
            Warning::StagedUnknown { reason } => write!(
                f,
                "The worktree's directory is gone, and its staged changes cannot be checked ({reason}); Git's record will be removed."
            ),
            Warning::SubmoduleAtRisk { path, dirty, unique_commits, no_remote } => {
                let mut lost = Vec::new();
                if !dirty.is_empty() {
                    let files = dirty.iter().map(|file| path.join(file).display().to_string());
                    lost.push(format!("{} ({})", counted(dirty.len(), "uncommitted file", "uncommitted files"), named(files)));
                }
                if *unique_commits > 0 {
                    let commits = counted(usize::try_from(*unique_commits).unwrap_or(usize::MAX), "commit", "commits");
                    lost.push(if *no_remote {
                        format!("{commits} (no remote configured)")
                    } else {
                        format!("{commits} on no remote")
                    });
                }
                write!(f, "Submodule {} has work that exists only there and will be lost: {}.", path.display(), lost.join(" and "))
            }
            Warning::SubmoduleUnchecked { path: Some(path), reason } => write!(
                f,
                "Submodule {} could not be checked ({reason}); its work will be deleted.",
                path.display()
            ),
            Warning::SubmoduleUnchecked { path: None, reason } => {
                write!(f, "This worktree's submodules could not be checked ({reason}); their work will be deleted.")
            }
            Warning::NestedMounts { mounts } => {
                writeln!(f, "{} mounted inside this worktree:", counted(mounts.len(), "filesystem is", "filesystems are"))?;
                for mount in mounts {
                    writeln!(f, "  {} (from {})", mount.mount_point.display(), mount.source)?;
                }
                write!(f, "Nothing on them is deleted, and the removal stays incomplete until they are unmounted.")
            }
            Warning::MountsUnchecked { reason } => write!(
                f,
                "Filesystems mounted inside this worktree could not be checked ({reason}); nothing on one would be deleted."
            ),
            Warning::Drvfs { mount_point } => write!(
                f,
                "This worktree is on a Windows drive mounted into WSL ({}). Windows programs using it cannot be seen from WSL, \
                 and deleting over this mount is slow; removing it from Windows is faster.",
                mount_point.display()
            ),
            Warning::SyncedFolder { root } => write!(
                f,
                "This worktree is inside a synced folder ({}); deleting it removes the cloud copies too.",
                root.display()
            ),
        }
    }
}

/// What sets an included file apart from its source, in plain words.
fn included_state(mark: &Mark) -> String {
    match mark {
        Mark::New => "new".into(),
        Mark::Changed => "changed since it was copied".into(),
        Mark::Unknown(reason) => format!("could not be compared: {reason}"),
        Mark::Unchanged => "unchanged".into(),
        Mark::Missing => "missing".into(),
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Note::IncludedCheck { message } => f.write_str(message),
            Note::SubmoduleGitDataRetained { path, git_dir } => write!(
                f,
                "Submodule {} keeps its Git data outside this worktree, at {}; that data is not deleted.",
                path.display(),
                git_dir.display()
            ),
        }
    }
}

impl fmt::Display for Blocker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Blocker::ShellInside { shell } => {
                let shell = shell.as_deref().unwrap_or("your shell");
                write!(
                    f,
                    "Nothing was removed: {shell} is standing in this worktree, and Windows cannot delete a folder a program is using as its current directory. Leave the worktree and run wt remove again."
                )
            }
        }
    }
}

#[cfg(test)]
mod tests;
