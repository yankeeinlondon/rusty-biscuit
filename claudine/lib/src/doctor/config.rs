//! The four working configuration files, and the `config`, `repo-config`,
//! and `stray-config-keys` checks over them.
//!
//! [`observe`] reads `~/.claudine/config.yaml`, `~/.darkmatter.yaml`, and,
//! inside a repository, `{repo-root}/.claudine/config.yaml` and
//! `{repo-root}/.darkmatter.yaml` with the readers and scope schemas normal
//! loads use, but passively: no legacy `config.json` is opened or
//! converted, no default file is created, and no committed policy is
//! selected. A `.darkmatter.yaml` whose `permissions` is a reference has its
//! target judged by Darkmatter's passive working-copy resolver
//! ([`darkmatter::config::init::observe`]), whatever state the scope's
//! Claudine file is in, so a broken target is a `config` error even where
//! the `permissions` handoff is skipped. The typed configuration
//! goes through the same merge and semantic checks as a normal load
//! (`dispatch::loader::load_claudine_config_with_notices`) plus the runtime
//! compiler's passive mapper check.
//!
//! Reports name files, locations, and categories, never a value: the only
//! authored text shown is an unknown key's name.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use darkmatter::config::init::{self as dm_init, BlockState};
use darkmatter::config::load::{ConfigError, ConfigErrorKind, ConfigFile, PermissionsSetting, inspect_working_file};
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use serde_json::Value;

use super::report::{display_text, Finding, Scope};
use super::{CheckOutcome, Environment, Pass, Severity};
use crate::config::json_conversion::LEGACY_FILE_NAME;
use crate::config::load::{ConfigDocument, ConfigProblem, ConfigProblemKind, read_document};
use crate::config::merge::merge_repo_override;
use crate::config::schema::{ConfigScope, contract};
use crate::config::setup::same_file;
use crate::config::claudine_config::ValidatedSetting;
use crate::config::{ClaudineConfig, RepoOverrideConfig};
use crate::error::ClaudineError;
use crate::events::AgenticEvent;

/// What the doctor knows about the configuration, read once per pass.
#[derive(Debug)]
pub(crate) struct ConfigObservations {
    pub(crate) home_available: bool,
    pub(crate) context_available: bool,
    /// Whether the launch directory is inside a repository.
    pub(crate) in_repository: bool,
    pub(crate) claudine_user: ObservedFile<ClaudineState>,
    /// `None` outside a repository.
    pub(crate) claudine_repo: Option<ObservedFile<ClaudineState>>,
    pub(crate) darkmatter_user: ObservedFile<DarkmatterState>,
    /// `None` outside a repository.
    pub(crate) darkmatter_repo: Option<ObservedFile<DarkmatterState>>,
    /// The user file alone, typed and judged by every semantic rule; what
    /// speech settings and the permissions handoff read. Its problem is
    /// not in `config` when the repository overrides replace the setting
    /// ([`Self::superseded_user_problem`]).
    pub(crate) user_config: Result<ClaudineConfig, Unusable>,
    /// The user file with the repository overrides merged on top, judged
    /// again; the user file alone when there are no usable overrides.
    #[cfg_attr(not(test), expect(dead_code, reason = "only tests read the merged configuration; the config row reports its semantic findings"))]
    pub(crate) effective_config: Result<ClaudineConfig, Unusable>,
    /// The semantic findings of the `config` row.
    semantic: Vec<Finding>,
}

/// One configuration file and what was found there.
#[derive(Debug)]
pub(crate) struct ObservedFile<S> {
    /// Where the file is; `None` when its scope cannot be located.
    pub(crate) path: Option<PathBuf>,
    pub(crate) state: S,
}

/// Why a file's content could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unreadable {
    /// A symbolic link whose target does not exist.
    DanglingLink,
    /// Something other than a regular file (a directory, a device).
    NotAFile,
    /// Any other I/O failure.
    Io(std::io::ErrorKind),
}

/// A Claudine configuration file.
#[derive(Debug)]
pub(crate) enum ClaudineState {
    /// Its scope cannot be located (no home).
    Unavailable,
    /// Nothing is there; `legacy` when a `config.json` sits beside it (a
    /// repository at the home leaves the user's legacy file to the user
    /// scope).
    Missing { legacy: bool },
    /// The repository file is the user file; it is judged once, as a user
    /// file.
    AliasOfUser,
    Unreadable(Unreadable),
    /// Every problem its scope's schema found.
    Invalid(Vec<ConfigProblem>),
    Valid(Box<ConfigDocument>),
}

/// A Darkmatter configuration file.
#[derive(Debug)]
pub(crate) enum DarkmatterState {
    /// Its scope cannot be located (no home, or no launch context).
    Unavailable,
    Missing,
    AliasOfUser,
    Unreadable(Unreadable),
    /// A failure of the whole file.
    Invalid(ConfigError),
    /// A readable file; its `permissions` and `register` findings are
    /// inside, and, for a valid `permissions` reference, why its target
    /// cannot be used.
    Valid(Box<ConfigFile>, Option<BrokenReference>),
}

/// A `permissions` reference whose target does not supply a policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BrokenReference {
    pub(crate) target: BrokenTarget,
    /// Whether it names the scope's own Claudine configuration, whose
    /// problems `config` reports beside it.
    pub(crate) names_claudine: bool,
}

/// Why a syntactically valid `permissions` reference does not supply a
/// policy, as Darkmatter's working-copy resolver found it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrokenTarget {
    /// It cannot be followed to a file inside its scope (a repository
    /// reference leaving the repository, through a link or otherwise).
    Unresolved,
    /// It names no existing file.
    Missing,
    /// The file it names does not supply a valid `permissions` mapping.
    Unusable,
}

/// Why no typed configuration is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unusable {
    /// The user file is missing, unreadable, or invalid; the `config` row
    /// says which.
    FileUnavailable,
    /// The file is valid but a semantic rule refuses it.
    Semantic(SemanticProblem),
}

/// A semantic rule a schema-valid configuration breaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticProblem {
    /// The typed reader refused the judged document.
    Typed,
    /// [`ClaudineConfig::validate`] refused this setting first.
    Rule { setting: ValidatedSetting },
    /// The runtime compiler would refuse this action's mapper.
    Mapper { event: AgenticEvent, index: usize },
}

/// Reads the four working files for `environment`.
pub(crate) fn observe(environment: &Environment) -> ConfigObservations {
    let home = environment.home();
    let repository = environment.repository_root();
    let claudine_user = match home {
        Some(home) => observe_claudine(ConfigScope::file_under(home), ConfigScope::User, None),
        None => ObservedFile { path: None, state: ClaudineState::Unavailable },
    };
    let claudine_repo = repository.map(|root| {
        observe_claudine(ConfigScope::file_under(root), ConfigScope::Repository, claudine_user.path.as_deref())
    });
    let darkmatter_user = observe_darkmatter(environment, DarkmatterScope::User, None, claudine_user.path.as_deref());
    let darkmatter_repo = repository.map(|_| {
        let claudine = claudine_repo.as_ref().and_then(|repo| repo.path.as_deref());
        observe_darkmatter(environment, DarkmatterScope::Repository, darkmatter_user.path.as_deref(), claudine)
    });

    let (user_config, effective_config, semantic) = typed(&claudine_user, claudine_repo.as_ref());
    ConfigObservations {
        home_available: home.is_some(),
        context_available: environment.context().is_ok(),
        in_repository: repository.is_some(),
        claudine_user,
        claudine_repo,
        darkmatter_user,
        darkmatter_repo,
        user_config,
        effective_config,
        semantic,
    }
}

/// Whether anything is at `path`, without reading it.
fn presence(path: &Path) -> Result<bool, Unreadable> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Unreadable::Io(error.kind())),
        Ok(link) => {
            let metadata = if link.file_type().is_symlink() {
                match std::fs::metadata(path) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(Unreadable::DanglingLink),
                    Err(error) => return Err(Unreadable::Io(error.kind())),
                }
            } else {
                link
            };
            if metadata.is_file() { Ok(true) } else { Err(Unreadable::NotAFile) }
        }
    }
}

fn observe_claudine(path: PathBuf, scope: ConfigScope, user: Option<&Path>) -> ObservedFile<ClaudineState> {
    let state = match presence(&path) {
        Err(unreadable) => ClaudineState::Unreadable(unreadable),
        Ok(false) => {
            let legacy_path = path.with_file_name(LEGACY_FILE_NAME);
            let legacy = std::fs::symlink_metadata(&legacy_path).is_ok()
                && !user.is_some_and(|user| same_file(&legacy_path, &user.with_file_name(LEGACY_FILE_NAME)));
            ClaudineState::Missing { legacy }
        }
        Ok(true) if user.is_some_and(|user| same_file(&path, user)) => ClaudineState::AliasOfUser,
        Ok(true) => match read_document(&path, scope) {
            Ok(document) => ClaudineState::Valid(Box::new(document)),
            Err(ClaudineError::ConfigInvalid { problems }) => ClaudineState::Invalid(problems),
            Err(ClaudineError::Io(error)) => ClaudineState::Unreadable(Unreadable::Io(error.kind())),
            Err(ClaudineError::ConfigNotFound(_)) => ClaudineState::Missing { legacy: false },
            Err(_) => ClaudineState::Unreadable(Unreadable::Io(std::io::ErrorKind::Other)),
        },
    };
    ObservedFile { path: Some(path), state }
}

fn observe_darkmatter(
    environment: &Environment,
    scope: DarkmatterScope,
    user: Option<&Path>,
    claudine: Option<&Path>,
) -> ObservedFile<DarkmatterState> {
    let Ok(context) = environment.context() else {
        return ObservedFile { path: None, state: DarkmatterState::Unavailable };
    };
    let Ok(path) = darkmatter::config::init::config_path(context, scope) else {
        return ObservedFile { path: None, state: DarkmatterState::Unavailable };
    };
    let state = match presence(&path) {
        Err(unreadable) => DarkmatterState::Unreadable(unreadable),
        Ok(false) => DarkmatterState::Missing,
        Ok(true) if user.is_some_and(|user| same_file(&path, user)) => DarkmatterState::AliasOfUser,
        Ok(true) => match inspect_working_file(context, scope) {
            Ok(file) if file.is_present() => {
                let broken = broken_reference(context, scope, &file, claudine);
                DarkmatterState::Valid(Box::new(file), broken)
            }
            Ok(_) => DarkmatterState::Missing,
            Err(error) if error.kind() == ConfigErrorKind::Read => {
                DarkmatterState::Unreadable(Unreadable::Io(std::io::ErrorKind::Other))
            }
            Err(error) => DarkmatterState::Invalid(error),
        },
    };
    ObservedFile { path: Some(path), state }
}

/// Follows `file`'s `permissions` reference as `md` does, passively:
/// `None` when it is not a reference or its target supplies a valid block.
fn broken_reference(
    context: &biscuit_file::FileResolutionContext,
    scope: DarkmatterScope,
    file: &ConfigFile,
    claudine: Option<&Path>,
) -> Option<BrokenReference> {
    if !matches!(file.permissions(), Ok(PermissionsSetting::Reference(_))) {
        return None;
    }
    let BlockState::Reference { block: Err(_), target, .. } = dm_init::observe(context, scope).ok()?.permissions else {
        return None;
    };
    let names_claudine = target.as_deref().zip(claudine).is_some_and(|(target, claudine)| same_file(target, claudine));
    let target = match target {
        None => BrokenTarget::Unresolved,
        Some(target) if !target.is_file() => BrokenTarget::Missing,
        Some(_) => BrokenTarget::Unusable,
    };
    Some(BrokenReference { target, names_claudine })
}

type Typed = (Result<ClaudineConfig, Unusable>, Result<ClaudineConfig, Unusable>, Vec<Finding>);

/// The user configuration alone and merged with the repository overrides,
/// each judged as a normal load judges it, and the `config` findings of
/// their semantic failures.
///
/// A merged failure is reported under the file that authors the failing
/// setting's effective value ([`source_of`]). A user-only failure the
/// overrides replace is left out of `config`, because the merged
/// configuration does not have it; one they do not replace is reported
/// under the user file even when the merged check stopped at another
/// setting first.
fn typed(user: &ObservedFile<ClaudineState>, repo: Option<&ObservedFile<ClaudineState>>) -> Typed {
    let ClaudineState::Valid(document) = &user.state else {
        return (Err(Unusable::FileUnavailable), Err(Unusable::FileUnavailable), Vec::new());
    };
    let user_path = user.path.as_deref().unwrap_or(&document.path);
    let typed_user = ClaudineConfig::from_effective(document.effective.clone(), Some(user_path))
        .map_err(|_| Unusable::Semantic(SemanticProblem::Typed));
    let user_only = typed_user.clone().and_then(|config| judged(config).map_err(Unusable::Semantic));

    let overrides = repo.and_then(|repo| match &repo.state {
        ClaudineState::Valid(document) => Some((
            repo.path.clone().unwrap_or_else(|| document.path.clone()),
            RepoOverrideConfig::from_effective(document.effective.clone(), Some(&document.path)),
        )),
        _ => None,
    });

    let mut findings = Vec::new();
    let effective = match (typed_user, overrides) {
        (Ok(mut merged), Some((repo_path, Ok(overrides)))) => {
            let has_messenger = merged.messenger.is_some();
            merge_repo_override(&mut merged, &overrides);
            let source = |problem: &SemanticProblem| source_of(problem, &overrides, has_messenger);
            judged(merged).map_err(|problem| {
                findings.push(match source(&problem) {
                    Source::User => semantic_finding(&problem, Scope::User, user_path),
                    Source::Repository => semantic_finding(&problem, Scope::Repository, &repo_path),
                    Source::Both => semantic_finding(&problem, Scope::Repository, &repo_path).note(format!(
                        "the route it selects must be defined in `messenger.configurations` of {}",
                        super::report::display_path(user_path)
                    )),
                });
                if let Err(Unusable::Semantic(own)) = &user_only
                    && *own != problem
                    && source(own) == Source::User
                {
                    findings.push(semantic_finding(own, Scope::User, user_path));
                }
                Unusable::Semantic(problem)
            })
        }
        (_, overrides) => {
            if let Some((repo_path, Err(_))) = &overrides {
                findings.push(semantic_finding(&SemanticProblem::Typed, Scope::Repository, repo_path));
            }
            if let Err(Unusable::Semantic(problem)) = &user_only {
                findings.push(semantic_finding(problem, Scope::User, user_path));
            }
            user_only.clone()
        }
    };
    (user_only, effective, findings)
}

/// Which file authors the effective value of the setting `problem` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    User,
    Repository,
    /// A repository `active_messenger` selecting from the user's routes.
    Both,
}

/// Follows [`merge_repo_override`]: the repository replaces an event's
/// whole action list, the whole `exit_expressions` declaration, and the
/// route selection (only when the user file has a `messenger` block);
/// Protect, default sounds, and route definitions are user-only.
fn source_of(problem: &SemanticProblem, overrides: &RepoOverrideConfig, user_has_messenger: bool) -> Source {
    match problem {
        SemanticProblem::Mapper { event, .. } if overrides.actions.contains_key(event) => Source::Repository,
        SemanticProblem::Rule { setting: ValidatedSetting::ExitExpressions } if overrides.exit_expressions.is_some() => {
            Source::Repository
        }
        SemanticProblem::Rule { setting: ValidatedSetting::MessengerSelection }
            if overrides.active_messenger.is_some() && user_has_messenger =>
        {
            Source::Both
        }
        _ => Source::User,
    }
}

/// `config` after the semantic checks of a normal load and the runtime
/// compiler's mapper check.
fn judged(config: ClaudineConfig) -> Result<ClaudineConfig, SemanticProblem> {
    if let Err((setting, _)) = config.validate_settings() {
        return Err(SemanticProblem::Rule { setting });
    }
    if let Some((event, index)) = crate::dispatch::loader::refused_action_mappers(&config).into_iter().next() {
        return Err(SemanticProblem::Mapper { event, index });
    }
    Ok(config)
}

/// The setting's name as `scope`'s file writes it.
fn setting_name(setting: ValidatedSetting, scope: Scope) -> String {
    match setting {
        ValidatedSetting::Protect => "protect".into(),
        ValidatedSetting::MessengerSelection if scope == Scope::Repository => "active_messenger".into(),
        ValidatedSetting::MessengerSelection => "messenger.active_config".into(),
        ValidatedSetting::MessengerRoutes => "messenger.configurations".into(),
        ValidatedSetting::DefaultSound(name) => format!("default_sounds.{name}"),
        ValidatedSetting::ExitExpressions => "exit_expressions".into(),
    }
}

/// One line naming the failed setting of `scope`'s file, never its value.
fn describe(problem: &SemanticProblem, scope: Scope) -> String {
    match problem {
        SemanticProblem::Typed => "the settings cannot be read as Claudine's configuration".to_string(),
        SemanticProblem::Rule { setting } => {
            format!("the setting `{}` breaks a configuration rule", setting_name(*setting, scope))
        }
        SemanticProblem::Mapper { event, index } => {
            // `index` is the list offset its remedy's path uses; a person counts from 1.
            format!("the mapper of action {} of `{event}` is not a valid regular expression", index + 1)
        }
    }
}

fn semantic_finding(problem: &SemanticProblem, scope: Scope, path: &Path) -> Finding {
    // A repository finding is a rule the merged configuration breaks, which
    // no command reports (a launch falls back to the defaults), or a typing
    // failure a schema-valid file almost never has; neither earns a command.
    let remedy = match (scope, problem) {
        (Scope::Repository, _) => "correct the setting".to_string(),
        (_, SemanticProblem::Mapper { event, index }) => mapper_remedy(*event, *index, None),
        (Scope::General | Scope::User, _) => format!("correct the setting; {USER_FULL_MESSAGE}"),
    };
    Finding::new(Severity::Error, scope, describe(problem, scope)).at(path).remedy(remedy)
}

/// The correction for a refused mapper, naming the setting by its path in
/// the file (only a `call` action has a mapper, and only a regular
/// expression can fail to compile). No command names it: a load checks the
/// rules of [`ClaudineConfig::validate`] but not mappers, so `claudine
/// actions` lists the action and exits 0.
fn mapper_remedy(event: AgenticEvent, index: usize, file: Option<&str>) -> String {
    let within = file.map(|file| format!(" in {file}")).unwrap_or_default();
    format!("correct `actions.{event}[{index}].call.mapper.pattern`{within} so it is a valid regular expression")
}

impl ConfigObservations {
    /// The user file's own semantic problem when `config` does not report
    /// it, because the repository overrides replace that setting; the rows
    /// that read the user file alone explain it themselves.
    pub(crate) fn superseded_user_problem(&self) -> Option<&SemanticProblem> {
        match &self.user_config {
            Err(Unusable::Semantic(problem)) if !self.semantic.iter().any(|finding| finding.scope == Scope::User) => {
                Some(problem)
            }
            _ => None,
        }
    }
}

/// The note a user-only row gives for [`ConfigObservations::superseded_user_problem`].
pub(crate) fn superseded_note(problem: &SemanticProblem) -> String {
    format!(
        "in the user file alone, {}; the repository configuration replaces that setting, so `config` reports no problem",
        describe(problem, Scope::User)
    )
}

/// The remedy for [`superseded_note`]: `claudine actions` reads the user
/// file alone, so it reports the rule even though the repository replaces
/// the setting, but it never compiles a mapper.
pub(crate) fn superseded_remedy(problem: &SemanticProblem) -> String {
    match problem {
        SemanticProblem::Mapper { event, index } => mapper_remedy(*event, *index, Some("`~/.claudine/config.yaml`")),
        SemanticProblem::Typed | SemanticProblem::Rule { .. } => {
            format!("correct the setting in `~/.claudine/config.yaml`; {USER_FULL_MESSAGE}")
        }
    }
}

// ==========================================================================
// The `config` check
// ==========================================================================

pub(crate) fn check_config(pass: &Pass<'_>) -> CheckOutcome {
    let observed = pass.config();
    let mut outcome = CheckOutcome::new("every configuration file is valid");
    if !observed.context_available {
        outcome.push(
            Finding::new(Severity::Error, Scope::General, "the launch directory's file-resolution context could not be built")
                .note("the Darkmatter files cannot be checked")
                .remedy("run `claudine doctor` from a directory you can read"),
        );
    }
    if !observed.home_available {
        outcome.push(
            Finding::new(Severity::Error, Scope::General, "the home directory cannot be located")
                .note("the user configuration cannot be checked")
                .remedy("set `HOME` (`USERPROFILE` on Windows) to your home directory"),
        );
    }
    claudine_findings(&observed.claudine_user, Scope::User, &mut outcome);
    if let Some(repo) = &observed.claudine_repo {
        claudine_findings(repo, Scope::Repository, &mut outcome);
    }
    darkmatter_findings(&observed.darkmatter_user, Scope::User, &mut outcome);
    if let Some(repo) = &observed.darkmatter_repo {
        darkmatter_findings(repo, Scope::Repository, &mut outcome);
    }
    outcome.findings.extend(observed.semantic.iter().cloned());
    outcome
}

fn claudine_findings(file: &ObservedFile<ClaudineState>, scope: Scope, outcome: &mut CheckOutcome) {
    let Some(path) = file.path.as_deref() else { return };
    let finding = match &file.state {
        ClaudineState::Unavailable => return,
        ClaudineState::Missing { legacy: true } if scope == Scope::Repository => Finding::new(
            Severity::Warn,
            scope,
            "is not present; a legacy `config.json` beside it has not been converted",
        )
        .at(path)
        .note("doctor leaves the legacy file as it is")
        .remedy(REPOSITORY_LEGACY_REMEDY),
        ClaudineState::Missing { legacy: true } => Finding::new(
            Severity::Error,
            scope,
            "is missing; a legacy `config.json` beside it has not been converted",
        )
        .at(path)
        .note("doctor leaves the legacy file as it is")
        .remedy("run `claudine setup`, which converts it"),
        // Setup writes the file only after publishing the host registration,
        // and nothing else creates it, so the remedy names that step.
        ClaudineState::Missing { legacy: false } if scope == Scope::User => {
            Finding::new(Severity::Error, scope, "is missing").at(path).remedy(MISSING_USER_REMEDY)
        }
        ClaudineState::Missing { .. } => Finding::new(Severity::Ok, scope, "not present (optional)").at(path),
        ClaudineState::AliasOfUser => {
            Finding::new(Severity::Ok, scope, "is the user configuration; judged once, as the user file").at(path)
        }
        ClaudineState::Unreadable(unreadable) => unreadable_finding(*unreadable, scope, path),
        ClaudineState::Valid(_) => Finding::new(Severity::Ok, scope, "valid").at(path),
        ClaudineState::Invalid(problems) => {
            let unknown = problems.iter().filter(|problem| problem.kind == ConfigProblemKind::UnknownKey).count();
            if unknown > 0 {
                outcome.push(
                    Finding::new(Severity::Error, scope, plural(unknown, "unknown key", "unknown keys"))
                        .at(path)
                        .note("listed under `stray-config-keys`")
                        .remedy("remove or move each key"),
                );
            }
            let config_scope = config_scope_of(scope);
            for problem in problems.iter().filter(|problem| problem.kind != ConfigProblemKind::UnknownKey) {
                outcome.push(claudine_problem_finding(problem, config_scope, scope, path));
            }
            return;
        }
    };
    outcome.push(finding);
}

fn claudine_problem_finding(problem: &ConfigProblem, config_scope: ConfigScope, scope: Scope, path: &Path) -> Finding {
    let at = location(problem.line, problem.column);
    let summary = match problem.kind {
        ConfigProblemKind::Syntax => format!("is not one valid YAML mapping{at}"),
        _ if problem.pointer.is_empty() => format!("cannot be read as configuration{at}"),
        _ => format!("has an invalid value at `{}`{at}", redacted_pointer(&problem.pointer, config_scope)),
    };
    let full_message = match scope {
        Scope::Repository => REPOSITORY_FULL_MESSAGE,
        Scope::General | Scope::User => USER_FULL_MESSAGE,
    };
    Finding::new(Severity::Error, scope, summary).at(path).remedy(format!("correct it; {full_message}"))
}

fn darkmatter_findings(file: &ObservedFile<DarkmatterState>, scope: Scope, outcome: &mut CheckOutcome) {
    let Some(path) = file.path.as_deref() else { return };
    match &file.state {
        DarkmatterState::Unavailable => {}
        DarkmatterState::Missing if scope == Scope::User => outcome.push(
            Finding::new(Severity::Error, scope, "is missing").at(path).remedy("run `claudine setup`"),
        ),
        DarkmatterState::Missing => outcome.push(Finding::new(Severity::Ok, scope, "not present (optional)").at(path)),
        DarkmatterState::AliasOfUser => outcome.push(
            Finding::new(Severity::Ok, scope, "is the user configuration; judged once, as the user file").at(path),
        ),
        DarkmatterState::Unreadable(unreadable) => outcome.push(unreadable_finding(*unreadable, scope, path)),
        DarkmatterState::Invalid(error) => {
            for problem in error.problems() {
                outcome.push(darkmatter_problem_finding(error.kind(), problem, scope, path));
            }
        }
        DarkmatterState::Valid(file, broken) => {
            let mut clean = true;
            if let Err(error) = file.permissions() {
                clean = false;
                for problem in error.problems() {
                    let mut finding = darkmatter_problem_finding(error.kind(), problem, scope, path);
                    if error.kind() == ConfigErrorKind::InvalidReference {
                        finding = finding.note("the reference is also assessed under `darkmatter-permissions`");
                    }
                    outcome.push(finding);
                }
            }
            if let Some(broken) = broken {
                clean = false;
                outcome.push(broken_target_finding(*broken, file, scope, path));
            }
            for error in file.registration_findings() {
                clean = false;
                for problem in error.problems() {
                    outcome.push(darkmatter_problem_finding(error.kind(), problem, scope, path));
                }
            }
            if clean {
                outcome.push(Finding::new(Severity::Ok, scope, "valid").at(path));
            }
        }
    }
}

fn darkmatter_problem_finding(
    kind: ConfigErrorKind,
    problem: &darkmatter::config::load::ConfigProblem,
    scope: Scope,
    path: &Path,
) -> Finding {
    let at = location(problem.line, problem.column);
    let segments = simplified_schema::JsonPointer::parse(&problem.pointer).segments().to_vec();
    let declared = declared_names(ConfigScope::User);
    let summary = match kind {
        ConfigErrorKind::Syntax | ConfigErrorKind::NotAMapping => format!("is not one valid YAML mapping{at}"),
        ConfigErrorKind::UnknownKey => match segments.last() {
            Some(key) => format!("has an unknown key `{}`{at}", display_text(key)),
            None => format!("has an unknown key{at}"),
        },
        ConfigErrorKind::InvalidReference => format!("has an invalid `permissions` reference{at}"),
        ConfigErrorKind::Read => "cannot be read".to_string(),
        _ if segments.is_empty() => format!("has an invalid value{at}"),
        _ => {
            let shown: String = segments
                .iter()
                .map(|segment| {
                    let known = segment == "register" || declared.contains(segment.as_str());
                    if known || segment.parse::<usize>().is_ok() { format!("/{segment}") } else { "/*".to_string() }
                })
                .collect();
            format!("has an invalid value at `{shown}`{at}")
        }
    };
    let remedy = match kind {
        // `md` loads `register` only to find host registrations, so no
        // compose reports it; the summary already locates it.
        _ if segments.first().is_some_and(|segment| segment == "register") => REGISTER_REMEDY.to_string(),
        ConfigErrorKind::Syntax
        | ConfigErrorKind::NotAMapping
        | ConfigErrorKind::UnknownKey
        | ConfigErrorKind::InvalidSetting
        | ConfigErrorKind::InvalidReference => format!("correct it; {MD_FULL_MESSAGE}"),
        _ => "correct it, then run `claudine doctor` again".to_string(),
    };
    Finding::new(Severity::Error, scope, summary).at(path).remedy(remedy)
}

/// The finding for a reference whose target is unusable. It names the
/// declaring file and where the reference is written, never the reference
/// or the target's path, which spells it. The `darkmatter-permissions` row
/// refers to this finding rather than repeating it.
fn broken_target_finding(broken: BrokenReference, file: &ConfigFile, scope: Scope, path: &Path) -> Finding {
    let position = file.permissions_position();
    let at = location(position.map(|at| at.line), position.map(|at| at.column));
    let to = if broken.names_claudine { "to Claudine's configuration, which" } else { "to a file that" };
    let summary = match (broken.target, scope) {
        (BrokenTarget::Unresolved, Scope::Repository) => {
            format!("has a `permissions` reference that does not lead to a file inside the repository{at}")
        }
        (BrokenTarget::Unresolved, _) => format!("has a `permissions` reference that cannot be resolved{at}"),
        (BrokenTarget::Missing, _) => format!("has a `permissions` reference {to} does not exist{at}"),
        (BrokenTarget::Unusable, _) => {
            format!("has a `permissions` reference {to} does not supply a valid `permissions` mapping{at}")
        }
    };
    // `md init --inline` writes only the user file, so it cannot repair a
    // repository reference.
    let remedy = match scope {
        Scope::Repository => "correct the reference or the file it names, then run `claudine doctor` again".to_string(),
        Scope::General | Scope::User => format!(
            "correct the reference or the file it names ({MD_FULL_MESSAGE}); `md init --inline` replaces the reference with a block"
        ),
    };
    Finding::new(Severity::Error, scope, summary)
        .at(path)
        .note("Darkmatter refuses its permissions policy until the reference supplies a valid mapping")
        .remedy(remedy)
}

/// The correction for an invalid `register` setting or entry.
const REGISTER_REMEDY: &str =
    "correct the `register` setting at that position: it is a list of manifest references; then run `claudine doctor` again";

fn unreadable_finding(unreadable: Unreadable, scope: Scope, path: &Path) -> Finding {
    let summary = match unreadable {
        Unreadable::DanglingLink => "is a symbolic link to nothing".to_string(),
        Unreadable::NotAFile => "is not a regular file".to_string(),
        Unreadable::Io(kind) => format!("cannot be read ({})", io_kind(kind)),
    };
    Finding::new(Severity::Error, scope, summary).at(path).remedy("make it a readable file, or remove it")
}

pub(crate) fn io_kind(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::PermissionDenied => "permission denied",
        std::io::ErrorKind::InvalidData => "not valid text",
        _ => "an I/O error",
    }
}

// ==========================================================================
// The `repo-config` check
// ==========================================================================

pub(crate) fn check_repo_config(pass: &Pass<'_>) -> CheckOutcome {
    let observed = pass.config();
    let mut outcome = CheckOutcome::new("the repository has its own Claudine configuration");
    let (true, Some(repo)) = (observed.in_repository, &observed.claudine_repo) else {
        let summary = if observed.context_available {
            "not inside a repository"
        } else {
            "the repository could not be determined"
        };
        outcome.push(Finding::new(Severity::Info, Scope::Repository, summary));
        return outcome;
    };
    let Some(path) = repo.path.as_deref() else { return outcome };
    let finding = match &repo.state {
        ClaudineState::Missing { legacy: true } => Finding::new(
            Severity::Info,
            Scope::Repository,
            "no YAML configuration; a legacy `config.json` awaits conversion, reported under `config`",
        ),
        ClaudineState::Missing { legacy: false } | ClaudineState::Unavailable => {
            Finding::new(Severity::Info, Scope::Repository, "no repository configuration (it is optional)")
        }
        ClaudineState::AliasOfUser => {
            outcome.ok_summary = "the repository configuration is the user configuration".into();
            Finding::new(Severity::Ok, Scope::Repository, "present; it is the user configuration, judged once as the user file")
        }
        ClaudineState::Valid(_) => Finding::new(Severity::Ok, Scope::Repository, "present"),
        ClaudineState::Invalid(_) | ClaudineState::Unreadable(_) => {
            outcome.ok_summary = "the repository has its own Claudine configuration; it has problems".into();
            Finding::new(Severity::Ok, Scope::Repository, "present; its problems are reported under `config`")
        }
    };
    outcome.push(finding.at(path));
    outcome
}

// ==========================================================================
// The `stray-config-keys` check
// ==========================================================================

pub(crate) fn check_stray_keys(pass: &Pass<'_>) -> CheckOutcome {
    let observed = pass.config();
    let mut outcome = CheckOutcome::new("no unknown Claudine configuration keys");
    let files = std::iter::once((&observed.claudine_user, Scope::User))
        .chain(observed.claudine_repo.iter().map(|repo| (repo, Scope::Repository)));
    for (file, scope) in files {
        let Some(path) = file.path.as_deref() else { continue };
        match &file.state {
            ClaudineState::Invalid(problems) => {
                if problems.iter().any(|problem| problem.kind == ConfigProblemKind::Syntax) {
                    outcome.push(
                        Finding::new(Severity::Info, scope, "could not be inspected for unknown keys: it is not one valid YAML mapping")
                            .at(path),
                    );
                    continue;
                }
                let mut unknown: Vec<&ConfigProblem> =
                    problems.iter().filter(|problem| problem.kind == ConfigProblemKind::UnknownKey).collect();
                // Source order, so the list reads top to bottom like the file;
                // one key has one position, so equal pointers stay adjacent.
                unknown.sort_by(|left, right| {
                    (left.line, left.column, &left.pointer).cmp(&(right.line, right.column, &right.pointer))
                });
                unknown.dedup_by(|left, right| left.pointer == right.pointer);
                for problem in unknown {
                    outcome.push(stray_finding(problem, config_scope_of(scope), scope, path));
                }
            }
            ClaudineState::Unreadable(_) => {
                outcome.push(Finding::new(Severity::Info, scope, "could not be inspected for unknown keys: it cannot be read").at(path));
            }
            ClaudineState::Valid(_) => outcome.push(Finding::new(Severity::Ok, scope, "no unknown keys").at(path)),
            ClaudineState::Unavailable
            | ClaudineState::Missing { .. }
            | ClaudineState::AliasOfUser => {}
        }
    }
    outcome
}

fn stray_finding(problem: &ConfigProblem, config_scope: ConfigScope, scope: Scope, path: &Path) -> Finding {
    let key = problem.unknown_key().unwrap_or_default();
    let segments = simplified_schema::JsonPointer::parse(&problem.pointer).segments().to_vec();
    let parent = &segments[..segments.len().saturating_sub(1)];
    let within = if parent.is_empty() {
        "at the top level".to_string()
    } else {
        format!("in `{}`", redacted_pointer(&simplified_schema::yaml_documents::pointer_string(parent), config_scope))
    };
    let at = location(problem.line, problem.column);
    let user_setting = parent.is_empty()
        && declared_names(ConfigScope::User).contains(key.as_str())
        && !declared_names(ConfigScope::Repository).contains(key.as_str());
    let remedy = if scope == Scope::Repository && user_setting {
        "it is a user setting; move it to `~/.claudine/config.yaml` or remove it"
    } else {
        "remove it, or correct its spelling"
    };
    Finding::new(Severity::Error, scope, format!("unknown key `{}` {within}{at}", display_text(&key)))
        .at(path)
        .remedy(remedy)
}

// ==========================================================================
// Shared wording
// ==========================================================================

// A remedy names one command confirmed to report the problem's category in
// its scope, never "any command"; where none does, it says what to correct.
// Help, completion, and `registrations` read no configuration, and `doctor`
// reads the files only to diagnose them; `setup` and every command that
// needs configuration read only the user file, and check its rules but not
// its mappers; only a provider launch stops on a repository file's error,
// and a dry run does not. The same holds for converting a repository's
// legacy `config.json`: only a launch with a prompt converts it (`setup`
// does not, nor does an interactive launch or a dry run).
// `cli/tests/l1/doctor.rs` runs the Claudine commands named here.

/// Where a user-file problem is printed in full.
const USER_FULL_MESSAGE: &str = "`claudine actions` prints the full message";

/// Where a repository-file problem is printed in full; the launch stops
/// before the provider starts. Not every wrapper reads the file (`goose`
/// launches without it), so this names one that does.
const REPOSITORY_FULL_MESSAGE: &str = "`claudine claude` run inside the repository stops with the full message";

/// The remedy of a missing user Claudine file.
const MISSING_USER_REMEDY: &str = "run `claudine setup`, which writes it after publishing the host registration; if publication stops, correct what setup names, then run it again";

/// The command that converts a repository's legacy `config.json`.
const REPOSITORY_LEGACY_REMEDY: &str =
    "`claudine claude` with a prompt, run inside the repository, converts it before the provider starts";

/// Where `md` prints a Darkmatter configuration error in full: a load
/// problem stops every `md compose`, a `permissions` problem only one that
/// needs the policy for a shell command, and `md render` reports neither.
/// A `register` problem is never printed by a compose.
pub(crate) const MD_FULL_MESSAGE: &str = "`md compose` of a document that runs a shell command prints the full message";

const fn config_scope_of(scope: Scope) -> ConfigScope {
    match scope {
        Scope::Repository => ConfigScope::Repository,
        Scope::General | Scope::User => ConfigScope::User,
    }
}

/// ` (line L, column C)`, or nothing when the position is unknown.
pub(super) fn location(line: Option<usize>, column: Option<usize>) -> String {
    match (line, column) {
        (Some(line), Some(column)) => format!(" (line {line}, column {column})"),
        (Some(line), None) => format!(" (line {line})"),
        _ => String::new(),
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 { format!("has 1 {one}") } else { format!("has {count} {many}") }
}

/// `pointer` with every segment that is neither a setting `scope`'s schema
/// declares nor a list index written as `*`, so a name the user chose (a
/// messenger route, say) is not shown.
fn redacted_pointer(pointer: &str, scope: ConfigScope) -> String {
    let declared = declared_names(scope);
    simplified_schema::JsonPointer::parse(pointer)
        .segments()
        .iter()
        .map(|segment| {
            if declared.contains(segment.as_str()) || segment.parse::<usize>().is_ok() {
                format!("/{segment}")
            } else {
                "/*".to_string()
            }
        })
        .collect()
}

/// Every property name the scope's lowered schema declares, anywhere in it.
fn declared_names(scope: ConfigScope) -> &'static HashSet<String> {
    static USER: OnceLock<HashSet<String>> = OnceLock::new();
    static REPOSITORY: OnceLock<HashSet<String>> = OnceLock::new();
    let cell = match scope {
        ConfigScope::User => &USER,
        ConfigScope::Repository => &REPOSITORY,
    };
    cell.get_or_init(|| {
        let mut names = HashSet::new();
        collect_property_names(contract(scope).json_schema(), &mut names);
        names
    })
}

fn collect_property_names(schema: &Value, names: &mut HashSet<String>) {
    match schema {
        Value::Object(map) => {
            if let Some(Value::Object(properties)) = map.get("properties") {
                names.extend(properties.keys().cloned());
            }
            for value in map.values() {
                collect_property_names(value, names);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_property_names(item, names);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
