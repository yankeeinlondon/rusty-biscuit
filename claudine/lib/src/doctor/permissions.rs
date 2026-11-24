//! The `darkmatter-permissions` check and its repair: how each scope's
//! `.darkmatter.yaml` gets its `permissions`, and the same handoff setup
//! performs.
//!
//! Both read the disk through setup's planner
//! ([`plan_permissions_handoff`] in [`HandoffMode::Doctor`]): the check
//! reports its decision, and the repair commits the same plan with setup's
//! committer, then reads the files again to verify the reference resolves to
//! the mapping it wrote. A finding names the declaring file and the setting,
//! never the reference as written or the block's contents, since both are
//! authored values.

use std::path::{Path, PathBuf};

use darkmatter::config::init::{self as dm_init, BlockState, InitError, effective_permissions};
use darkmatter::config::write::ConfigWriteError;

use super::config::{ClaudineState, ConfigObservations, DarkmatterState, ObservedFile, io_kind};
use super::report::display_path;
use super::{CheckOutcome, Finding, Pass, RepairOutcome, Scope, Severity};
use crate::composition::registration::config_target_blockers;
use crate::config::schema::ConfigScope;
use crate::config::setup::{
    ClaudineSide, ConflictKind, HandoffCommitError, HandoffMode, HandoffPlan, HandoffRefusal, ManualReason,
    PermissionsHandoff, commit_permissions_handoff, darkmatter_scope_of, plan_permissions_handoff, read_document,
    same_file,
};

const FIX: &str = "run `claudine doctor --fix`, or `claudine setup`";

/// Added to every repository-scope finding that was assessed.
const REPOSITORY_POLICY: &str = "this describes the handoff on disk; what a repository is granted still depends on Darkmatter's user, committed, and working-copy policy";

pub(crate) fn check_permissions(pass: &Pass<'_>) -> CheckOutcome {
    let mut outcome = CheckOutcome::new("Darkmatter takes its permissions from Claudine's configuration");
    for scope in [Scope::User, Scope::Repository] {
        match assess(pass, scope) {
            Some(Assessment::Settled(finding) | Assessment::Repairable { finding, .. }) => outcome.push(finding),
            None => {}
        }
    }
    outcome
}

pub(crate) fn repair_permissions(pass: &Pass<'_>) -> RepairOutcome {
    let mut changes = Vec::new();
    for scope in [Scope::User, Scope::Repository] {
        let Some(Assessment::Repairable { plan, .. }) = assess(pass, scope) else { continue };
        match commit(pass, *plan) {
            Ok(made) => changes.extend(made),
            Err((made, reason)) => {
                changes.extend(made);
                return RepairOutcome { changes, failure: Some(reason) };
            }
        }
    }
    RepairOutcome { changes, failure: None }
}

/// One scope's handoff, as the check and the repair both see it.
enum Assessment {
    /// Nothing the repair can do: the finding says how the scope stands.
    Settled(Finding),
    /// The repair commits `plan`; the finding is fixable.
    Repairable { finding: Finding, plan: Box<HandoffPlan> },
}

/// `None` for the repository scope outside a repository.
fn assess(pass: &Pass<'_>, scope: Scope) -> Option<Assessment> {
    let observed = pass.config();
    let (config_scope, label) = match scope {
        Scope::Repository if !observed.in_repository => return None,
        Scope::Repository => (ConfigScope::Repository, "repository"),
        Scope::General | Scope::User => (ConfigScope::User, "user"),
    };
    let settled = |finding: Finding| {
        let finding = if scope == Scope::Repository { finding.note(REPOSITORY_POLICY) } else { finding };
        Some(Assessment::Settled(finding))
    };
    let skipped = |summary: String| Some(Assessment::Settled(Finding::new(Severity::Info, scope, summary)));

    if !observed.home_available && scope == Scope::User {
        return skipped("user scope skipped: the home directory cannot be located".into());
    }
    let Ok(context) = pass.environment().context() else {
        return skipped(format!("{label} scope skipped: the launch directory's file-resolution context could not be built"));
    };
    let (claudine_path, darkmatter_path) = match prerequisites(observed, scope, label) {
        Ok(paths) => paths,
        Err(finding) => return Some(Assessment::Settled(finding)),
    };

    let darkmatter_scope = darkmatter_scope_of(config_scope);
    let unreadable = |path: &Path| {
        skipped(format!("{label} scope skipped: `{}` could not be read again", display_path(path)))
    };
    let Ok(observation) = dm_init::observe(context, darkmatter_scope) else { return unreadable(&darkmatter_path) };
    let Some((bytes, document)) = read_document(&claudine_path) else { return unreadable(&claudine_path) };
    let claudine = ClaudineSide { path: claudine_path.clone(), bytes, document, block_before: None };

    let at_darkmatter = |severity, summary: String| Finding::new(severity, scope, summary).at(&darkmatter_path);
    let at_claudine = |severity, summary: String| Finding::new(severity, scope, summary).at(&claudine_path);
    let claudine_shown = display_path(&claudine_path);

    let plan = match plan_permissions_handoff(context, config_scope, HandoffMode::Doctor, claudine, &observation) {
        Ok(plan) => plan,
        Err(refusal) => return settled(refused(refusal, label, &claudine_shown, at_darkmatter, at_claudine)),
    };
    if !(plan.edits_claudine() || plan.edits_darkmatter()) {
        return settled(match plan.permissions {
            PermissionsHandoff::ThirdParty { .. } => at_darkmatter(
                Severity::Info,
                format!("the {label} `permissions` refers to another program's file; Claudine leaves it in place"),
            )
            .note(format!("Darkmatter does not read the `permissions` of {claudine_shown}")),
            _ => at_darkmatter(Severity::Ok, format!("the {label} `permissions` refers to Claudine's configuration")),
        });
    }
    if let Some(link) = obstruction(&plan) {
        return settled(
            Finding::new(Severity::Error, scope, "is a symbolic link, which the repair does not write through")
                .at(&link)
                .remedy("hand `permissions` over by hand, or replace the link with the file, then run `claudine doctor --fix`"),
        );
    }

    let summary = match &observation.permissions {
        BlockState::Inline(serde_json::Value::Null) => format!("the {label} `permissions` is null instead of referring to Claudine's configuration"),
        BlockState::Inline(_) => format!("the {label} `permissions` is an inline block that Claudine's configuration should hold"),
        _ => format!("the {label} `.darkmatter.yaml` does not set `permissions` to Claudine's configuration"),
    };
    let what = if plan.edits_claudine() {
        format!("the repair writes the block into {claudine_shown}, then refers to it")
    } else {
        format!("{claudine_shown} already holds an equivalent block; the repair only refers to it")
    };
    let mut finding = at_darkmatter(Severity::Error, summary).note(what).remedy(FIX).fixable();
    if scope == Scope::Repository {
        finding = finding.note(REPOSITORY_POLICY);
    }
    Some(Assessment::Repairable { finding, plan: Box::new(plan) })
}

/// The scope's Claudine and Darkmatter files, when both exist as a valid,
/// distinct pair; otherwise the `info` finding naming what blocked it.
fn prerequisites(observed: &ConfigObservations, scope: Scope, label: &str) -> Result<(PathBuf, PathBuf), Finding> {
    let (claudine, darkmatter) = match scope {
        Scope::Repository => (observed.claudine_repo.as_ref(), observed.darkmatter_repo.as_ref()),
        Scope::General | Scope::User => (Some(&observed.claudine_user), Some(&observed.darkmatter_user)),
    };
    let skipped = |path: Option<&Path>, why: &str| {
        let finding = Finding::new(Severity::Info, scope, format!("{label} scope skipped: {why}"));
        match path {
            Some(path) => finding.at(path),
            None => finding,
        }
    };
    let claudine = claudine.ok_or_else(|| skipped(None, "the repository could not be determined"))?;
    let darkmatter = darkmatter.ok_or_else(|| skipped(None, "the repository could not be determined"))?;
    let claudine_path = file_ready(claudine, |state| match state {
        ClaudineState::Valid(_) => None,
        ClaudineState::Unavailable => Some("its location cannot be determined"),
        ClaudineState::Missing { .. } => Some("this Claudine configuration does not exist"),
        ClaudineState::AliasOfUser => Some(ALIAS),
        ClaudineState::Unreadable(_) | ClaudineState::Invalid(_) => Some(PROBLEMS),
    })
    .map_err(|(path, why)| skipped(path, why))?;
    let darkmatter_path = file_ready(darkmatter, |state| match state {
        DarkmatterState::Valid(_) => None,
        DarkmatterState::Unavailable => Some("its location cannot be determined"),
        DarkmatterState::Missing => Some("this `.darkmatter.yaml` does not exist"),
        DarkmatterState::AliasOfUser => Some(ALIAS),
        DarkmatterState::Unreadable(_) | DarkmatterState::Invalid(_) => Some(PROBLEMS),
    })
    .map_err(|(path, why)| skipped(path, why))?;
    Ok((claudine_path, darkmatter_path))
}

const ALIAS: &str = "this file is the user file, already assessed in the user scope";
const PROBLEMS: &str = "this file has problems, reported under `config`";

fn file_ready<S>(
    file: &ObservedFile<S>,
    blocked: impl Fn(&S) -> Option<&'static str>,
) -> Result<PathBuf, (Option<&Path>, &'static str)> {
    match (blocked(&file.state), &file.path) {
        (None, Some(path)) => Ok(path.clone()),
        (None, None) => Err((None, "its location cannot be determined")),
        (Some(why), path) => Err((path.as_deref(), why)),
    }
}

/// The finding for a scope no automatic handoff applies to.
fn refused(
    refusal: HandoffRefusal,
    label: &str,
    claudine_shown: &str,
    at_darkmatter: impl Fn(Severity, String) -> Finding,
    at_claudine: impl Fn(Severity, String) -> Finding,
) -> Finding {
    let correct = "correct it, then run `claudine doctor` again; `md` prints the full message for this file";
    match refusal {
        HandoffRefusal::Darkmatter(_) => at_darkmatter(Severity::Error, format!("the {label} `permissions` is invalid"))
            .note("it is also reported under `config`")
            .remedy(correct),
        HandoffRefusal::Conflict { kind: ConflictKind::DifferentBlocks, .. } => at_darkmatter(
            Severity::Error,
            format!("the {label} `permissions` block differs from the one in Claudine's configuration"),
        )
        .note(format!("both this file and {claudine_shown} author `permissions`"))
        .remedy(format!(
            "choose which setting to keep: delete the block from this file or from {claudine_shown}, or make them equal, then run `claudine doctor --fix`"
        )),
        HandoffRefusal::Conflict { kind: ConflictKind::ClaudineReference, .. } => at_darkmatter(
            Severity::Error,
            format!("the {label} `permissions` cannot refer to Claudine's configuration, whose own `permissions` is a reference"),
        )
        .note("Darkmatter does not follow a reference to a reference")
        .remedy(format!(
            "choose which setting to keep: write a block in {claudine_shown} in place of its reference, or set this file's `permissions` yourself"
        )),
        HandoffRefusal::Manual(ManualReason::BrokenReference { ours }) => {
            let names = if ours { "Claudine's configuration" } else { "a file" };
            at_darkmatter(
                Severity::Error,
                format!("the {label} `permissions` refers to {names}, which does not supply a valid `permissions` mapping"),
            )
            .remedy("correct the reference or the file it names; `md` prints the full message, and `md init --inline` replaces the reference with a block")
        }
        HandoffRefusal::Manual(ManualReason::DarkmatterMissing) => {
            at_darkmatter(Severity::Info, format!("{label} scope skipped: this `.darkmatter.yaml` does not exist"))
        }
        HandoffRefusal::Manual(ManualReason::ClaudineUnusable) => {
            at_claudine(Severity::Error, "its `permissions` setting cannot be handed to Darkmatter".into()).remedy(correct)
        }
        HandoffRefusal::Manual(ManualReason::ClaudineInvalid) => at_claudine(
            Severity::Error,
            "breaks a configuration rule, so the `permissions` handoff cannot edit it".into(),
        )
        .note("the rule is reported under `config`")
        .remedy("correct it, then run `claudine doctor --fix`"),
        HandoffRefusal::Manual(ManualReason::UnusableMapping) => at_claudine(
            Severity::Error,
            format!("the `permissions` mapping it would supply fails Darkmatter's checks for the {label} scope"),
        )
        .remedy(format!("correct the mapping (`md` prints the full message once it is in use), then run `{FIX}`")),
    }
}

/// The first file the plan writes, or its lock sidecar, that the writer
/// must not touch: a symbolic link or reparse point it would follow, or
/// something other than a regular file. The publisher's no-follow guard
/// ([`config_target_blockers`]) decides, as it does for the registration
/// repair's edit.
fn obstruction(plan: &HandoffPlan) -> Option<PathBuf> {
    let mut targets = Vec::new();
    if plan.edits_claudine() {
        targets.push(&plan.claudine_config);
    }
    if plan.edits_darkmatter() {
        targets.push(&plan.darkmatter_config);
    }
    targets.into_iter().find_map(|target| config_target_blockers(target).into_iter().next().map(|blocker| blocker.path))
}

/// Commits one scope's plan and verifies it: the changes made, or those
/// made before the failure and its reason.
fn commit(pass: &Pass<'_>, plan: HandoffPlan) -> Result<Vec<String>, (Vec<String>, String)> {
    let Ok(context) = pass.environment().context() else {
        return Err((Vec::new(), "the launch directory's file-resolution context could not be built".into()));
    };
    if let Some(link) = obstruction(&plan) {
        return Err((Vec::new(), format!("{} is a symbolic link; nothing was saved", display_path(&link))));
    }
    let (claudine, darkmatter) = (plan.claudine_config.clone(), plan.darkmatter_config.clone());
    let (scope, expected) = (darkmatter_scope_of(plan.scope()), plan.block.clone());
    // The writer reports resolved paths; the report names the files as found.
    let shown = |path: &Path| {
        if same_file(path, &claudine) { display_path(&claudine) } else { display_path(&darkmatter) }
    };
    let edited = |path: &Path| format!("{}: set `permissions`", shown(path));

    let changes: Vec<String> = match commit_permissions_handoff(context, plan) {
        Ok(outcome) => outcome.targets.iter().filter(|target| target.written).map(|target| edited(&target.path)).collect(),
        Err(error) => return Err(failure(error, &shown, &edited)),
    };

    let verified = dm_init::observe(context, scope).is_ok_and(|observation| match observation.permissions {
        BlockState::Reference { target: Some(target), block: Ok(block), .. } => {
            same_file(&target, &claudine)
                && expected.as_ref().is_some_and(|expected| {
                    effective_permissions(scope, expected).is_some()
                        && effective_permissions(scope, expected) == effective_permissions(scope, &block)
                })
        }
        _ => false,
    });
    if verified {
        Ok(changes)
    } else {
        Err((
            changes,
            format!(
                "after the edit, {} does not resolve to the `permissions` mapping in {}",
                display_path(&darkmatter),
                display_path(&claudine)
            ),
        ))
    }
}

/// The changes already published and a reason naming the operation, the
/// file, and the I/O kind; never the writer's or validator's text, which can
/// quote authored values.
fn failure(
    error: HandoffCommitError,
    shown: &dyn Fn(&Path) -> String,
    edited: &dyn Fn(&Path) -> String,
) -> (Vec<String>, String) {
    let nothing = "nothing was saved";
    let reason = match error {
        HandoffCommitError::Claudine(_) => "the edited Claudine configuration would be invalid; nothing was saved".to_string(),
        HandoffCommitError::UnusableTarget => {
            "the edited Claudine configuration would not supply a usable `permissions` mapping; nothing was saved".to_string()
        }
        HandoffCommitError::Darkmatter(error) => match *error {
            InitError::Write(ConfigWriteError::ReplacementFailed { path, source, published }) => {
                let made = published.iter().map(|path| edited(path)).collect();
                return (made, format!("could not replace {} ({})", shown(&path), io_kind(source.kind())));
            }
            InitError::Write(ConfigWriteError::LockTimeout { lock, .. }) => {
                format!("another writer held the lock {}; {nothing}", display_path(&lock))
            }
            InitError::Write(ConfigWriteError::Changed { path }) => {
                format!("{} changed after it was read; {nothing}", shown(&path))
            }
            InitError::Write(ConfigWriteError::Io { path, source }) => {
                format!("could not read or lock {} ({}); {nothing}", shown(&path), io_kind(source.kind()))
            }
            InitError::Write(ConfigWriteError::ValidationFailed { path, .. }) => {
                format!("the edited {} would be invalid; {nothing}", shown(&path))
            }
            InitError::Write(
                ConfigWriteError::PathResolution { path, .. } | ConfigWriteError::UnsupportedEdit { path, .. },
            ) => format!("could not edit {} in place; {nothing}", shown(&path)),
            _ => format!("could not save the `permissions` handoff; {nothing}"),
        },
    };
    (Vec::new(), reason)
}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod tests;
