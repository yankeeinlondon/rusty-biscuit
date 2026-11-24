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

use biscuit_file::canonicalize_simplified;

use darkmatter::config::init::{self as dm_init, BlockState, InitError, effective_permissions};
use darkmatter::config::load::ConfigFile;
use darkmatter::config::write::ConfigWriteError;

use super::config::{
    ClaudineState, ConfigObservations, DarkmatterState, MD_FULL_MESSAGE, ObservedFile, SemanticProblem, io_kind,
    superseded_note,
};
use super::report::display_path;
use super::{CheckOutcome, Finding, Pass, RepairOutcome, Scope, Severity};
use super::registration::{blocker_text, layout_problem, layout_remedy};
use crate::composition::registration::BlockerProblem;
use crate::config::schema::ConfigScope;
use crate::config::setup::{
    ClaudineSide, ConflictKind, HandoffCommitError, HandoffMode, HandoffPlan, HandoffRefusal, ManualReason,
    PermissionsHandoff, commit_permissions_handoff, darkmatter_scope_of, plan_permissions_handoff, read_document,
    same_file,
};

/// The remedy of a repairable handoff in either scope. `claudine setup`
/// performs the user handoff too, but only after publishing the host
/// registration, so a publisher obstruction stops it first; doctor's repair
/// does not depend on publication.
const FIX: &str = "run `claudine doctor --fix`";

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

/// Each scope's handoff is independent: a failure in one is recorded with its
/// scope and the other scope is still repaired. Within one scope the two
/// writes stay ordered, so a failure there stops that scope's handoff.
pub(crate) fn repair_permissions(pass: &Pass<'_>) -> RepairOutcome {
    let mut changes = Vec::new();
    let mut failures = Vec::new();
    for (scope, label) in [(Scope::User, "user"), (Scope::Repository, "repository")] {
        let Some(Assessment::Repairable { plan, .. }) = assess(pass, scope) else { continue };
        match commit(pass, *plan) {
            Ok(made) => changes.extend(made),
            Err((made, reason)) => {
                changes.extend(made);
                failures.push(format!("{label} scope: {reason}"));
            }
        }
    }
    let failure = (!failures.is_empty()).then(|| failures.join("; "));
    RepairOutcome { changes, failure }
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
        Err(refusal) => {
            let superseded = if scope == Scope::Repository { None } else { observed.superseded_user_problem() };
            return settled(refused(refusal, label, &claudine_shown, superseded, at_darkmatter, at_claudine));
        }
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
    let summary = match &observation.permissions {
        BlockState::Inline(serde_json::Value::Null) => format!("the {label} `permissions` is null instead of referring to Claudine's configuration"),
        BlockState::Inline(_) => format!("the {label} `permissions` is an inline block that Claudine's configuration should hold"),
        _ => format!("the {label} `.darkmatter.yaml` does not set `permissions` to Claudine's configuration"),
    };
    // Darkmatter's writer judges the whole edited file, so a `register`
    // problem the edit leaves in place refuses it on every attempt.
    if darkmatter_file(observed, scope).is_some_and(|file| !file.registration_findings().is_empty()) {
        return settled(
            at_darkmatter(Severity::Error, summary)
                .note("the repair cannot edit this file while its `register` setting is invalid, which is reported under `config`")
                .remedy(format!("correct the `register` setting, then {FIX}")),
        );
    }
    if let Some(blocker) = plan.obstruction() {
        let remedy = match blocker.problem {
            BlockerProblem::Unreadable { .. } => "make it readable, then run `claudine doctor`",
            _ => "hand `permissions` over by hand, or replace it with a real folder or file, then run `claudine doctor --fix`",
        };
        return settled(Finding::new(Severity::Error, scope, blocker_text(blocker.problem)).at(&blocker.path).remedy(remedy));
    }
    // The writer refuses a reference whose target leaves the scope, which a
    // linked Claudine file or folder can do even when only Darkmatter's file
    // is edited.
    if !plan.reference_in_scope() {
        return settled(out_of_scope(scope, &claudine_path, at_darkmatter(Severity::Error, summary)));
    }

    // The editor keeps every authored byte, so it refuses an edit that has
    // to walk into a flow collection; both files stay as they are.
    if let Some((path, layout)) = plan.layout_obstacle() {
        let shown = display_path(path);
        return settled(
            at_darkmatter(Severity::Error, summary)
                .note(layout_problem(layout, &shown))
                .remedy(layout_remedy(layout, &shown)),
        );
    }

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
        DarkmatterState::Valid(..) => None,
        DarkmatterState::Unavailable => Some("its location cannot be determined"),
        DarkmatterState::Missing => Some("this `.darkmatter.yaml` does not exist"),
        DarkmatterState::AliasOfUser => Some(ALIAS),
        DarkmatterState::Unreadable(_) | DarkmatterState::Invalid(_) => Some(PROBLEMS),
    })
    .map_err(|(path, why)| skipped(path, why))?;
    Ok((claudine_path, darkmatter_path))
}

/// The scope's Darkmatter file as `config` observed it, when it was valid.
fn darkmatter_file(observed: &ConfigObservations, scope: Scope) -> Option<&ConfigFile> {
    let file = match scope {
        Scope::Repository => observed.darkmatter_repo.as_ref()?,
        Scope::General | Scope::User => &observed.darkmatter_user,
    };
    match &file.state {
        DarkmatterState::Valid(file, _) => Some(file),
        _ => None,
    }
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
    superseded: Option<&SemanticProblem>,
    at_darkmatter: impl Fn(Severity, String) -> Finding,
    at_claudine: impl Fn(Severity, String) -> Finding,
) -> Finding {
    let correct = "correct it, then run `claudine doctor` again";
    match refusal {
        HandoffRefusal::Darkmatter(_) => at_darkmatter(Severity::Error, format!("the {label} `permissions` is invalid"))
            .note("it is also reported under `config`")
            .remedy(format!("{correct}; {MD_FULL_MESSAGE}")),
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
            // `config` judges every reference target, whether or not this
            // scope's pair is complete, and carries the location and remedy.
            .note("the reference is reported under `config`")
            .remedy("correct it as `config` describes, then run `claudine doctor` again")
        }
        HandoffRefusal::Manual(ManualReason::DarkmatterMissing) => {
            at_darkmatter(Severity::Info, format!("{label} scope skipped: this `.darkmatter.yaml` does not exist"))
        }
        HandoffRefusal::Manual(ManualReason::ClaudineUnusable) => {
            at_claudine(Severity::Error, "its `permissions` setting cannot be handed to Darkmatter".into()).remedy(correct)
        }
        HandoffRefusal::Manual(ManualReason::ClaudineInvalid) => {
            let finding = at_claudine(
                Severity::Error,
                "breaks a configuration rule, so the `permissions` handoff cannot edit it".into(),
            );
            match superseded {
                // `config` judges the merged configuration, which the
                // repository's overrides repair; the handoff edits this file
                // alone.
                Some(problem) => finding.note(superseded_note(problem)),
                None => finding.note("the rule is reported under `config`"),
            }
            .remedy("correct it, then run `claudine doctor --fix`")
        }
        HandoffRefusal::Manual(ManualReason::UnusableMapping) => at_claudine(
            Severity::Error,
            format!("the `permissions` mapping it would supply fails Darkmatter's checks for the {label} scope"),
        )
        .remedy(format!("correct its `permissions` mapping for the {label} scope, then {FIX}")),
    }
}

/// The finding for a plan whose reference would leave its scope: it names
/// where Claudine's file resolves to and how to bring it back inside.
fn out_of_scope(scope: Scope, claudine: &Path, finding: Finding) -> Finding {
    let replace = format!("replace the link with the file it names, then {FIX}");
    let (root, remedy) = match scope {
        // Darkmatter's repository policy refuses such a reference too, so
        // writing it by hand is no alternative there.
        Scope::Repository => ("the repository", replace),
        Scope::General | Scope::User => (
            "your home",
            format!("{replace}; or set this file's `permissions` to refer to Claudine's configuration yourself"),
        ),
    };
    let resolved = match canonicalize_simplified(claudine) {
        Ok(target) => format!("{} resolves to {}, outside {root}", display_path(claudine), display_path(&target)),
        Err(_) => format!("{} cannot be resolved inside {root}", display_path(claudine)),
    };
    finding.note(format!("{resolved}; the repair refers only to a file inside its scope")).remedy(remedy)
}

/// Commits one scope's plan and verifies it: the changes made, or those
/// made before the failure and its reason.
fn commit(pass: &Pass<'_>, plan: HandoffPlan) -> Result<Vec<String>, (Vec<String>, String)> {
    let Ok(context) = pass.environment().context() else {
        return Err((Vec::new(), "the launch directory's file-resolution context could not be built".into()));
    };
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
        HandoffCommitError::Obstructed(blocker) => {
            format!("{} {}; nothing was saved", display_path(&blocker.path), blocker_text(blocker.problem))
        }
        HandoffCommitError::ReferenceOutOfScope => {
            "the reference would not name Claudine's configuration inside its scope; nothing was saved".to_string()
        }
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
