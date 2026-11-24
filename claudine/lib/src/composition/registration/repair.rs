//! Doctor's repair of the registration: publish, remove extras, verify,
//! point the user `register` entry at the manifest in place of any entry
//! naming the retired manifest, verify both (the manifest named, no retired
//! entry left), then remove the retired layout, all under one publisher
//! lock.
//!
//! Unlike a plain [`publish`](super::publish), the repair removes every
//! entry of the output root the inventory does not have. It starts only
//! when the user `.darkmatter.yaml` exists with a valid `register` setting
//! whose needed entry edit Darkmatter's format-preserving editor can make in
//! place (an inline `register: [...]` list or root mapping it would have to
//! edit makes the whole repair manual, so nothing is published first), and
//! nothing in its whole set (lock, tree, extras, the configuration file
//! and its writer's lock sidecar, the retired layout) is a link, a special
//! file, or unreadable; the same examination runs again under the lock and
//! each target is examined again just before it is changed.
//!
//! The retired layout is kept until the published tree and the new entry
//! both verify, so a failure at any step leaves the old registration usable
//! and the next repair converges.

use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::Duration;

use biscuit_file::FileResolutionContext;
use darkmatter::config::init::{self as dm_init, InitAction, InitError};
use darkmatter::config::load::{ConfigErrorKind, inspect_working_file};
use darkmatter::config::schema::ConfigScope as DarkmatterScope;

use super::inspect::{
    Blocker, BlockerProblem, EntryPrecondition, config_target_blockers, entry_layout, entry_precondition, entry_registered,
    inspect_installation, retired_entries,
};
use super::{
    CleanupError, Entry, LOCK_DEADLINE, Layout, MANIFEST_REFERENCE, PublishChanges, PublishError, PublishStep,
    Publisher, RegistrationInventory, access_log, entry_at, legacy_manifest_path, remove_entry,
};

/// What [`repair_registration`] did: every change in the order made, and
/// why it stopped when it did not finish.
#[derive(Debug, Default)]
pub(crate) struct RegistrationRepair {
    pub(crate) operations: Vec<RepairOperation>,
    pub(crate) failure: Option<RepairFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepairOperation {
    /// A published file was created or replaced.
    Replaced(PathBuf),
    /// A file or folder was removed.
    Removed(PathBuf),
    /// The `register` list of this configuration file was edited.
    Edited(PathBuf),
}

/// Why a repair stopped. Each carries only paths and typed causes, never
/// authored text or an OS message.
#[derive(Debug)]
pub(crate) enum RepairFailure {
    NoHome,
    /// The user `.darkmatter.yaml` cannot be edited; nothing was changed.
    Precondition(EntryPrecondition),
    /// An obstruction or a misplaced entry; see [`Blocker`].
    Blocked(Blocker),
    /// The publisher stopped; the previous manifest stands.
    Publish(PublishError),
    /// Removing the extra entry at `path` failed.
    Remove { path: PathBuf, kind: io::ErrorKind },
    /// After publishing, `path` still does not hold the inventory (or is
    /// still an extra).
    Unverified { path: PathBuf },
    /// The `register` edit of `path` was refused or failed; the published
    /// tree is kept for a retry.
    RegisterEdit { path: PathBuf, cause: EditFailure },
    /// After the edit, no `register` entry of `path` names the manifest.
    EntryUnverified { path: PathBuf },
    /// After the edit, a `register` entry of `path` still names the retired
    /// format 1 manifest.
    RetiredEntryKept { path: PathBuf },
    /// The retired layout could not be removed.
    Legacy(CleanupError),
    /// A test observer stopped the repair at this step.
    Interrupted { step: String },
}

/// Why Darkmatter's writer did not edit the `register` list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditFailure {
    /// The file changed while the edit was prepared.
    Changed,
    /// Another writer held the file's lock.
    LockTimeout,
    /// The file, or the edited text, is not a valid configuration.
    Invalid,
    Io(io::ErrorKind),
}

/// A point of a repair a test observer sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepairStep {
    /// A step of the publish.
    Publish(PublishStep),
    /// The tree is published and verified; the `register` edit is next.
    BeforeRegister,
    /// The entry verified; the retired layout's removal is next.
    BeforeLegacyCleanup,
}

/// Options for [`repair_registration_with`].
pub(crate) struct RepairHooks<'a> {
    /// Replaces [`LOCK_DEADLINE`].
    pub(crate) lock_deadline: Duration,
    /// Sees each [`RepairStep`]; `Break` stops the repair there.
    pub(crate) observer: &'a mut dyn FnMut(&RepairStep) -> ControlFlow<()>,
}

/// Repairs the registration under `context`'s home; see the module docs.
pub(crate) fn repair_registration(
    context: &FileResolutionContext,
    inventory: &RegistrationInventory,
) -> RegistrationRepair {
    let mut observer = |_: &RepairStep| ControlFlow::Continue(());
    repair_registration_with(context, inventory, RepairHooks { lock_deadline: LOCK_DEADLINE, observer: &mut observer })
}

/// [`repair_registration`] with a lock deadline and an observer.
pub(crate) fn repair_registration_with(
    context: &FileResolutionContext,
    inventory: &RegistrationInventory,
    hooks: RepairHooks<'_>,
) -> RegistrationRepair {
    let mut repair = RegistrationRepair::default();
    if let Err(failure) = run(context, inventory, hooks, &mut repair.operations) {
        repair.failure = Some(failure);
    }
    repair
}

fn run(
    context: &FileResolutionContext,
    inventory: &RegistrationInventory,
    hooks: RepairHooks<'_>,
    operations: &mut Vec<RepairOperation>,
) -> Result<(), RepairFailure> {
    let home = context.home_dir().ok_or(RepairFailure::NoHome)?;
    let config = dm_init::config_path(context, DarkmatterScope::User).map_err(|_| RepairFailure::NoHome)?;
    let observer = hooks.observer;

    editable(context)?;
    preflight(home, &config, inventory)?;
    let publisher =
        Publisher::acquire_with(home, inventory, hooks.lock_deadline).map_err(publish_failure)?;
    // Another caller may have changed anything while this one waited.
    editable(context)?;
    preflight(home, &config, inventory)?;

    let mut changes = PublishChanges::default();
    let published =
        publisher.publish_into(inventory, &mut |step| observer(&RepairStep::Publish(step.clone())), &mut changes);
    record_publish(&publisher, &changes, operations);
    published.map_err(publish_failure)?;

    remove_extras(home, &config, inventory, operations)?;
    // Byte equality with the inventory is the verification, so no changed
    // manifest is left to judge: Claudine's inventory is the bundle
    // `claudine_hosts` verifies and prepares in every process, and the
    // installed-registration tests load its published copy from disk.
    let installation = inspect_installation(home, Some(&config), inventory);
    if !installation.is_current() {
        let path = installation
            .files
            .iter()
            .find(|file| file.state != super::ExpectedState::Current)
            .map(|file| file.path.clone())
            .or_else(|| installation.extras.first().map(|extra| extra.path.clone()))
            .or_else(|| installation.blockers.first().map(|blocker| blocker.path.clone()))
            .unwrap_or(installation.output_root);
        return Err(RepairFailure::Unverified { path });
    }

    stop(RepairStep::BeforeRegister, observer)?;
    if let Some(blocker) = config_target_blockers(home, &config).into_iter().next() {
        return Err(RepairFailure::Blocked(blocker));
    }
    let report = dm_init::replace_manifest_registration_existing(
        context,
        DarkmatterScope::User,
        MANIFEST_REFERENCE,
        &legacy_manifest_path(home),
    )
    .map_err(|error| RepairFailure::RegisterEdit { path: config.clone(), cause: edit_failure(&error) })?;
    if report.action != InitAction::Unchanged {
        operations.push(RepairOperation::Edited(report.path));
    }
    let file = inspect_working_file(context, DarkmatterScope::User).ok();
    if file.as_ref().and_then(|file| entry_registered(context, home, file)) != Some(true) {
        return Err(RepairFailure::EntryUnverified { path: config });
    }
    if file.is_some_and(|file| !retired_entries(context, home, &file).is_empty()) {
        return Err(RepairFailure::RetiredEntryKept { path: config });
    }

    stop(RepairStep::BeforeLegacyCleanup, observer)?;
    let (removed, failure) = match publisher.finalize_legacy() {
        Ok(cleanup) => (cleanup.removed, None),
        Err(failure) => (failure.removed, Some(failure.error)),
    };
    operations.extend(removed.into_iter().map(RepairOperation::Removed));
    failure.map_or(Ok(()), |error| Err(RepairFailure::Legacy(error)))
}

fn stop(step: RepairStep, observer: &mut dyn FnMut(&RepairStep) -> ControlFlow<()>) -> Result<(), RepairFailure> {
    match observer(&step) {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(()) => Err(RepairFailure::Interrupted { step: format!("{step:?}") }),
    }
}

/// Refuses unless the user `.darkmatter.yaml` exists with a `register`
/// setting the writer can edit, and the entry edit the repair needs (if
/// any) is one the format-preserving editor can make in place.
fn editable(context: &FileResolutionContext) -> Result<(), RepairFailure> {
    match inspect_working_file(context, DarkmatterScope::User) {
        Ok(file) => entry_precondition(&file).and_then(|()| entry_layout(context)).map_err(RepairFailure::Precondition),
        Err(error) if error.kind() == ConfigErrorKind::Read => {
            Err(RepairFailure::Precondition(EntryPrecondition::Unreadable))
        }
        Err(_) => Err(RepairFailure::Precondition(EntryPrecondition::Invalid)),
    }
}

/// Refuses when anything in the repair's set stops it.
fn preflight(home: &Path, config: &Path, inventory: &RegistrationInventory) -> Result<(), RepairFailure> {
    let installation = inspect_installation(home, Some(config), inventory);
    match installation.manual_blockers().into_iter().next() {
        Some(blocker) => Err(RepairFailure::Blocked(blocker)),
        None => Ok(()),
    }
}

fn record_publish(publisher: &Publisher, changes: &PublishChanges, operations: &mut Vec<RepairOperation>) {
    let layout = Layout::new(publisher.home());
    operations.extend(changes.removed_temps.iter().cloned().map(RepairOperation::Removed));
    operations.extend(changes.written.iter().map(|path| RepairOperation::Replaced(layout.destination(path))));
    operations.extend(changes.retired.iter().map(|path| RepairOperation::Removed(layout.destination(path))));
    operations.extend(changes.removed_folders.iter().map(|path| RepairOperation::Removed(layout.destination(path))));
}

/// Removes every extra entry of the output root, children before their
/// folder, examining each again just before it is removed.
fn remove_extras(
    home: &Path,
    config: &Path,
    inventory: &RegistrationInventory,
    operations: &mut Vec<RepairOperation>,
) -> Result<(), RepairFailure> {
    let installation = inspect_installation(home, Some(config), inventory);
    if let Some(blocker) = installation.blockers.into_iter().next() {
        return Err(RepairFailure::Blocked(blocker));
    }
    for extra in installation.extras {
        let path = extra.path;
        let entry = entry_at(&path).map_err(|error| RepairFailure::Remove { path: path.clone(), kind: kind_of(&error) })?;
        let removed = match entry {
            Entry::Absent => continue,
            Entry::File if !extra.directory => remove_entry(&path, false),
            Entry::Directory if extra.directory => remove_entry(&path, true),
            Entry::Obstruction(obstruction) => {
                return Err(RepairFailure::Blocked(Blocker { path, problem: BlockerProblem::Link(obstruction) }));
            }
            Entry::File | Entry::Directory | Entry::Other => {
                return Err(RepairFailure::Blocked(Blocker { path, problem: BlockerProblem::Special }));
            }
        };
        access_log::note("remove", &path);
        match removed {
            Ok(()) => operations.push(RepairOperation::Removed(path)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(RepairFailure::Remove { path, kind: error.kind() }),
        }
    }
    Ok(())
}

fn kind_of(error: &PublishError) -> io::ErrorKind {
    match error {
        PublishError::Io { source, .. } => source.kind(),
        _ => io::ErrorKind::Other,
    }
}

/// A publisher refusal of a link or misplaced entry is the same manual
/// finding as the preflight's.
fn publish_failure(error: PublishError) -> RepairFailure {
    match error {
        PublishError::Obstructed { path, obstruction } => {
            RepairFailure::Blocked(Blocker { path, problem: BlockerProblem::Link(obstruction) })
        }
        PublishError::NotADirectory { path } => RepairFailure::Blocked(Blocker { path, problem: BlockerProblem::NotADirectory }),
        PublishError::NotAFile { path } => RepairFailure::Blocked(Blocker { path, problem: BlockerProblem::NotAFile }),
        PublishError::Interrupted { step } => RepairFailure::Interrupted { step },
        other => RepairFailure::Publish(other),
    }
}

fn edit_failure(error: &InitError) -> EditFailure {
    use darkmatter::config::write::ConfigWriteError;
    match error {
        InitError::Invalid(_) | InitError::NoHome | InitError::NotARepository { .. } => EditFailure::Invalid,
        InitError::Read { source, .. } => EditFailure::Io(source.kind()),
        InitError::Write(write) => match write {
            ConfigWriteError::Changed { .. } => EditFailure::Changed,
            ConfigWriteError::LockTimeout { .. } => EditFailure::LockTimeout,
            ConfigWriteError::Io { source, .. } | ConfigWriteError::ReplacementFailed { source, .. } => {
                EditFailure::Io(source.kind())
            }
            ConfigWriteError::ValidationFailed { .. } | ConfigWriteError::UnsupportedEdit { .. } => EditFailure::Invalid,
            _ => EditFailure::Io(io::ErrorKind::Other),
        },
        _ => EditFailure::Io(io::ErrorKind::Other),
    }
}

#[cfg(test)]
#[path = "repair_tests.rs"]
mod tests;
