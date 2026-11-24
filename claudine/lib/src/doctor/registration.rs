//! The `registration` check and its repair: Claudine's published
//! `darkmatter-host` registration under `~/.claudine/registrations/` and the
//! user `register` entry that names it.
//!
//! The check compares every published file's exact bytes with the inventory
//! compiled into this binary, judges a changed manifest by whether
//! Darkmatter loads the whole bundle it lists, lists entries of the output
//! root the inventory does not have and the retired format 1 layout, and
//! looks for a user `register` entry naming the manifest by Darkmatter's
//! manifest identity, so any spelling of the same file passes, and for user
//! entries naming the retired format 1 manifest by the rule Darkmatter's
//! writer removes them with, whether or not that file still exists.
//! Other hosts' entries and repository `register` entries are not its
//! concern. Nothing is followed through a link and nothing is written.
//!
//! A finding is fixable exactly when [`repair_registration`] would address
//! it: the user `.darkmatter.yaml` is valid with a valid `register` setting,
//! the entry edit the repair needs (if any) is one Darkmatter's
//! format-preserving editor can make in place, as a dry run of that edit
//! decides, and nothing in the repair's set is a link, a special file, or
//! unreadable.

use std::io;
use std::path::Path;

use super::config::{DarkmatterState, ObservedFile, Unreadable, io_kind, location};
use super::{CheckOutcome, Environment, Finding, Pass, RepairOutcome, Scope, Severity};
use crate::composition::registration::{
    self as publisher, Blocker, BlockerProblem, CleanupError, EditFailure, EditLayout, EntryPrecondition, ExpectedState, Installation,
    ManifestLoad, PublishError, RegistrationInventory, RepairFailure, RepairOperation,
};

const FIX: &str = "run `claudine doctor --fix`";

pub(crate) fn check_registration(pass: &Pass<'_>) -> CheckOutcome {
    let environment = pass.environment();
    let Some(home) = environment.home() else {
        return CheckOutcome::from(vec![Finding::new(
            Severity::Info,
            Scope::General,
            "the home directory cannot be located, so the registration cannot be inspected",
        )]);
    };
    let darkmatter = &pass.config().darkmatter_user;
    let mut installation =
        publisher::inspect_installation(home, darkmatter.path.as_deref(), RegistrationInventory::claudine());
    if let Ok(context) = environment.context() {
        installation.judge_manifest(home, context);
    }
    let precondition = precondition(environment, darkmatter);

    let mut outcome = CheckOutcome::new("the registration is published and registered");
    let mut repairable = disk_findings(&installation);
    let mut manual: Vec<Finding> = installation.blockers.iter().map(blocker_finding).collect();
    manual.extend(installation.files.iter().filter_map(|file| {
        let summary = match file.state {
            ExpectedState::NotAFile => "is not a regular file".to_string(),
            ExpectedState::Unreadable(kind) => format!("cannot be read ({})", io_kind(kind)),
            _ => return None,
        };
        Some(Finding::new(Severity::Error, Scope::User, summary).at(&file.path).remedy("move or remove it yourself, then run `claudine doctor --fix`"))
    }));
    entry_findings(environment, home, darkmatter, &installation, precondition, &mut repairable, &mut manual);

    let needs_repair = !repairable.is_empty();
    if needs_repair {
        manual.extend(installation.target_blockers.iter().map(blocker_finding));
    }
    let eligible = precondition.is_ok() && manual.iter().all(|finding| finding.severity != Severity::Error);
    if needs_repair && let Err(blocked) = precondition {
        let mut finding = Finding::new(
            Severity::Info,
            Scope::User,
            format!("the automatic repair is unavailable: {}", precondition_text(blocked)),
        )
        .remedy(precondition_remedy(blocked));
        if let Some(path) = &darkmatter.path {
            finding = finding.at(path);
        }
        manual.push(finding);
    }
    for finding in repairable {
        outcome.push(if eligible { finding.remedy(FIX).fixable() } else { finding });
    }
    for finding in manual {
        outcome.push(finding);
    }
    outcome
}

/// Whether the repair could edit the user `register` list.
fn precondition(environment: &Environment, darkmatter: &ObservedFile<DarkmatterState>) -> Result<(), EntryPrecondition> {
    if environment.context().is_err() {
        return Err(EntryPrecondition::NoContext);
    }
    match &darkmatter.state {
        DarkmatterState::Valid(file, _) => publisher::entry_precondition(file).and_then(|()| {
            environment.context().map_or(Ok(()), publisher::entry_layout)
        }),
        DarkmatterState::Missing => Err(EntryPrecondition::Missing),
        DarkmatterState::Unreadable(_) => Err(EntryPrecondition::Unreadable),
        DarkmatterState::Invalid(_) => Err(EntryPrecondition::Invalid),
        DarkmatterState::Unavailable | DarkmatterState::AliasOfUser => Err(EntryPrecondition::NoContext),
    }
}

/// The findings of the published tree and the retired layout that the
/// repair addresses.
fn disk_findings(installation: &Installation) -> Vec<Finding> {
    let mut findings = Vec::new();
    let all_missing = installation.files.iter().all(|file| file.state == ExpectedState::Missing);
    if all_missing && !installation.root_present {
        findings.push(
            Finding::new(Severity::Error, Scope::User, "the published registration is missing").at(&installation.output_root),
        );
    } else {
        for file in &installation.files {
            let finding = match (file.state, file.manifest) {
                (ExpectedState::Missing, _) => Finding::new(Severity::Error, Scope::User, "is missing"),
                (ExpectedState::Changed, false) => {
                    let finding = Finding::new(Severity::Warn, Scope::User, "differs from the file this Claudine publishes");
                    if file.digest_mismatch {
                        finding.note("its bytes do not match the manifest's digest, which Darkmatter's format 2 loader refuses, so the host is not loaded until it is republished")
                    } else {
                        finding
                    }
                }
                (ExpectedState::Changed, true) => match &installation.manifest_load {
                    Some(ManifestLoad::Loads) => Finding::new(
                        Severity::Warn,
                        Scope::User,
                        "the manifest differs from the one this Claudine publishes",
                    )
                    .note("Darkmatter still loads the bundle it lists as Claudine's registration"),
                    Some(ManifestLoad::Refused { at }) => {
                        let finding = Finding::new(
                            Severity::Error,
                            Scope::User,
                            "the manifest differs from the one this Claudine publishes and Darkmatter refuses the bundle it lists as Claudine's registration",
                        );
                        match at {
                            Some(at) => finding.note(format!("Darkmatter's first refusal is in {}", at.display())),
                            None => finding,
                        }
                    }
                    None => Finding::new(
                        Severity::Warn,
                        Scope::User,
                        "the manifest differs from the one this Claudine publishes",
                    )
                    .note("whether Darkmatter loads it is judged once `claudine doctor` runs from a readable directory"),
                },
                _ => continue,
            };
            findings.push(finding.at(&file.path));
        }
    }
    for extra in &installation.extras {
        let summary = if extra.directory {
            "is a folder that is not part of the published registration"
        } else {
            "is not part of the published registration"
        };
        findings.push(Finding::new(Severity::Warn, Scope::User, summary).at(&extra.path));
    }
    for legacy in &installation.legacy {
        findings.push(
            Finding::new(Severity::Warn, Scope::User, "is left over from the retired format 1 registration")
                .at(legacy)
                .note("the repair removes it once the `register` entry names the published manifest"),
        );
    }
    findings
}

/// The finding of the user `register` entry, sorted into `repairable` or
/// `manual`.
fn entry_findings(
    environment: &Environment,
    home: &Path,
    darkmatter: &ObservedFile<DarkmatterState>,
    installation: &Installation,
    precondition: Result<(), EntryPrecondition>,
    repairable: &mut Vec<Finding>,
    manual: &mut Vec<Finding>,
) {
    let at = |finding: Finding| match &darkmatter.path {
        Some(path) => finding.at(path),
        None => finding,
    };
    let (Ok(context), DarkmatterState::Valid(file, _)) = (environment.context(), &darkmatter.state) else {
        let reason = match &darkmatter.state {
            DarkmatterState::Missing => "the file does not exist",
            DarkmatterState::Unreadable(Unreadable::DanglingLink) => "the file is a symbolic link to nothing",
            DarkmatterState::Unreadable(_) => "the file cannot be read",
            DarkmatterState::Invalid(_) => "the file is invalid (see the `config` row)",
            _ => "the user Darkmatter configuration cannot be located",
        };
        manual.push(at(Finding::new(
            Severity::Info,
            Scope::User,
            format!("the `register` entry cannot be verified: {reason}"),
        )
        .remedy("create or correct `~/.darkmatter.yaml` (`claudine setup` creates it), then run `claudine doctor --fix`")));
        return;
    };
    let manifest_is_file = installation
        .files
        .iter()
        .any(|file| file.manifest && matches!(file.state, ExpectedState::Current | ExpectedState::Changed));
    let registered = if manifest_is_file { publisher::entry_registered(context, home, file) } else { None };
    match registered {
        Some(true) => {}
        Some(false) => {
            let finding = at(Finding::new(Severity::Error, Scope::User, "no `register` entry names Claudine's manifest"));
            match precondition {
                Ok(()) => repairable.push(finding),
                Err(blocked) => manual.push(finding.remedy(precondition_remedy(blocked))),
            }
        }
        None => manual.push(at(Finding::new(
            Severity::Info,
            Scope::User,
            "the `register` entry is verified once the manifest is published",
        ))),
    }
    // Darkmatter warns at such an entry whether or not the retired files are
    // still there, and the repair's writer removes it either way.
    for entry in publisher::retired_entries(context, home, file) {
        let position = location(entry.at.as_ref().map(|at| at.line), entry.at.as_ref().map(|at| at.column));
        let finding = at(Finding::new(
            Severity::Warn,
            Scope::User,
            format!("`register` entry {} names the retired format 1 manifest{position}", entry.index + 1),
        ))
        .note("Darkmatter warns about it whenever it loads its configuration");
        match precondition {
            Ok(()) => repairable.push(finding),
            Err(blocked) => manual.push(finding.remedy(precondition_remedy(blocked))),
        }
    }
}

fn precondition_text(blocked: EntryPrecondition) -> String {
    let text = match blocked {
        EntryPrecondition::Layout(layout) => return layout_problem(layout, "`~/.darkmatter.yaml`"),
        EntryPrecondition::NoContext => "the user Darkmatter configuration cannot be located",
        EntryPrecondition::Missing => "`~/.darkmatter.yaml` does not exist",
        EntryPrecondition::Unreadable => "`~/.darkmatter.yaml` cannot be read",
        EntryPrecondition::Invalid => "`~/.darkmatter.yaml` is invalid",
        EntryPrecondition::RegisterInvalid => "the `register` setting of `~/.darkmatter.yaml` is invalid",
        EntryPrecondition::PermissionsInvalid => "the `permissions` setting of `~/.darkmatter.yaml` is invalid",
    };
    text.to_string()
}

fn precondition_remedy(blocked: EntryPrecondition) -> String {
    match blocked {
        EntryPrecondition::Missing => "run `claudine setup`, which creates it, then run `claudine doctor --fix`".to_string(),
        EntryPrecondition::NoContext => "run `claudine doctor` again from a readable directory".to_string(),
        EntryPrecondition::Layout(layout) => layout_remedy(layout, "`~/.darkmatter.yaml`"),
        _ => "correct the file (see the `config` row), then run `claudine doctor --fix`".to_string(),
    }
}

/// Why Darkmatter's editor cannot make a planned edit to `file`, naming the
/// setting but never its value. Only an inline root is refused at the root;
/// a setting's refusal does not say whether a flow collection or an anchor,
/// alias, or tag stopped it, so the reason names both kinds.
pub(super) fn layout_problem(layout: EditLayout, file: &str) -> String {
    match layout {
        EditLayout::Root => format!(
            "the root mapping of {file} is written inline, which Darkmatter's format-preserving editor cannot change in place"
        ),
        EditLayout::Setting(setting) => format!(
            "Darkmatter's format-preserving editor cannot change the `{setting}` setting of {file} in place, because the edit reaches an inline `{{…}}` or `[…]` collection, or a value with an anchor, alias, or tag"
        ),
    }
}

/// How to bring `file`'s layout within the editor's reach.
pub(super) fn layout_remedy(layout: EditLayout, file: &str) -> String {
    let what = match layout {
        EditLayout::Root => format!("the root mapping of {file}"),
        EditLayout::Setting("register") => format!("the `register` list of {file}"),
        EditLayout::Setting(setting) => format!("the `{setting}` setting of {file}"),
    };
    format!(
        "rewrite {what} in block form, one `key: value` or `- item` per line, without a `{{…}}` or `[…]` collection, anchor, alias, or tag, then run `claudine doctor --fix`"
    )
}

pub(super) fn blocker_text(problem: BlockerProblem) -> String {
    match problem {
        BlockerProblem::Link(obstruction) => format!("is {obstruction}, which doctor never follows or replaces"),
        BlockerProblem::NotADirectory => "is not a folder".to_string(),
        BlockerProblem::NotAFile => "is not a regular file".to_string(),
        BlockerProblem::Special => "is neither a regular file nor a folder".to_string(),
        BlockerProblem::Unreadable { operation, kind } => format!("cannot be {} ({})", past(operation), io_kind(kind)),
    }
}

fn blocker_finding(blocker: &Blocker) -> Finding {
    let remedy = match blocker.problem {
        BlockerProblem::Unreadable { .. } => "make it readable, then run `claudine doctor`",
        _ => "move or remove it yourself, then run `claudine doctor --fix`",
    };
    Finding::new(Severity::Error, Scope::User, blocker_text(blocker.problem)).at(&blocker.path).remedy(remedy)
}

fn past(operation: &str) -> &'static str {
    match operation {
        "list" => "listed",
        "read" => "read",
        "remove" => "removed",
        "write" => "written",
        _ => "inspected",
    }
}

pub(crate) fn repair_registration(pass: &Pass<'_>) -> RepairOutcome {
    let Ok(context) = pass.environment().context() else {
        return RepairOutcome {
            changes: Vec::new(),
            failure: Some(precondition_text(EntryPrecondition::NoContext)),
        };
    };
    let repair = publisher::repair_registration(context, RegistrationInventory::claudine());
    RepairOutcome {
        changes: repair.operations.iter().map(operation_text).collect(),
        failure: repair.failure.as_ref().map(failure_text),
    }
}

fn operation_text(operation: &RepairOperation) -> String {
    match operation {
        RepairOperation::Replaced(path) => format!("wrote {}", path.display()),
        RepairOperation::Removed(path) => format!("removed {}", path.display()),
        RepairOperation::Edited(path) => format!("updated the `register` entry in {}", path.display()),
    }
}

/// Why the repair stopped, from typed causes only: an operation, a path,
/// and an I/O kind, never an OS or validator message.
fn failure_text(failure: &RepairFailure) -> String {
    let io = |action: &str, path: &Path, kind: io::ErrorKind| format!("could not {action} {} ({})", path.display(), io_kind(kind));
    match failure {
        RepairFailure::NoHome => "the home directory cannot be located".to_string(),
        RepairFailure::Precondition(blocked) => format!("{}; nothing was changed", precondition_text(*blocked)),
        RepairFailure::Blocked(blocker) => format!("{} {}", blocker.path.display(), blocker_text(blocker.problem)),
        RepairFailure::Publish(error) => match error {
            PublishError::LockTimeout { lock, .. } => format!("another publisher held {}", lock.display()),
            PublishError::Io { action, path, source } => io(action, path, source.kind()),
            PublishError::InvalidPath { .. } => "the compiled registration cannot be published".to_string(),
            _ => "the publish stopped".to_string(),
        },
        RepairFailure::Remove { path, kind } => io("remove", path, *kind),
        RepairFailure::Unverified { path } => {
            format!("{} does not hold the published registration after publishing", path.display())
        }
        RepairFailure::RegisterEdit { path, cause } => {
            let cause = match cause {
                EditFailure::Changed => "the file changed while it was being edited".to_string(),
                EditFailure::LockTimeout => "another writer held its lock".to_string(),
                EditFailure::Invalid => "Darkmatter's writer refused the edit".to_string(),
                EditFailure::Io(kind) => io_kind(*kind).to_string(),
            };
            format!("could not update the `register` entry in {} ({cause}); the published registration is kept and the retired one left in place", path.display())
        }
        RepairFailure::EntryUnverified { path } => {
            format!("no `register` entry of {} names the manifest after the edit", path.display())
        }
        RepairFailure::RetiredEntryKept { path } => {
            format!("a `register` entry of {} still names the retired format 1 manifest after the edit", path.display())
        }
        RepairFailure::Legacy(error) => match error {
            CleanupError::Obstructed { path, obstruction } => {
                format!("{} {}", path.display(), blocker_text(BlockerProblem::Link(*obstruction)))
            }
            CleanupError::Unexpected { path } => {
                format!("{} {}", path.display(), blocker_text(BlockerProblem::Special))
            }
            CleanupError::Io { action, path, source } => io(action, path, source.kind()),
        },
        RepairFailure::Interrupted { step } => format!("the repair was stopped at {step}"),
    }
}

#[cfg(test)]
#[path = "registration_tests.rs"]
mod tests;
