//! The four working configuration files, and the `config`, `repo-config`,
//! and `stray-config-keys` checks over them.
//!
//! [`observe`] reads `~/.claudine/config.yaml`, `~/.darkmatter.yaml`, and,
//! inside a repository, `{repo-root}/.claudine/config.yaml` and
//! `{repo-root}/.darkmatter.yaml` with the readers and scope schemas normal
//! loads use, but passively: no legacy `config.json` is opened or
//! converted, no default file is created, no committed policy is selected,
//! and no `permissions` reference is followed. The typed configuration
//! goes through the same merge and semantic checks as a normal load
//! (`dispatch::loader::load_claudine_config_with_notices`) plus the runtime
//! compiler's passive mapper check.
//!
//! Reports name files, locations, and categories, never a value: the only
//! authored text shown is an unknown key's name.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use darkmatter::config::load::{ConfigError, ConfigErrorKind, ConfigFile, inspect_working_file};
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use serde_json::Value;

use super::report::{display_text, Finding, Scope};
use super::{CheckOutcome, Environment, Pass, Severity};
use crate::config::json_conversion::LEGACY_FILE_NAME;
use crate::config::load::{ConfigDocument, ConfigProblem, ConfigProblemKind, read_document};
use crate::config::merge::merge_repo_override;
use crate::config::schema::{ConfigScope, contract};
use crate::config::setup::same_file;
use crate::config::{ClaudineConfig, RepoOverrideConfig};
use crate::error::ClaudineError;

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
    /// speech settings and the permissions handoff read.
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
    /// Nothing is there; `legacy` when a `config.json` sits beside it.
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
    /// inside.
    Valid(Box<ConfigFile>),
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
    /// [`ClaudineConfig::validate`] refused it, naming the setting when it
    /// can be named safely.
    Rule { setting: Option<String> },
    /// The runtime compiler would refuse this action's mapper.
    Mapper { event: String, index: usize },
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
    let darkmatter_user = observe_darkmatter(environment, DarkmatterScope::User, None);
    let darkmatter_repo = repository
        .map(|_| observe_darkmatter(environment, DarkmatterScope::Repository, darkmatter_user.path.as_deref()));

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
            let legacy = scope == ConfigScope::User
                && std::fs::symlink_metadata(path.with_file_name(LEGACY_FILE_NAME)).is_ok();
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
            Ok(file) if file.is_present() => DarkmatterState::Valid(Box::new(file)),
            Ok(_) => DarkmatterState::Missing,
            Err(error) if error.kind() == ConfigErrorKind::Read => {
                DarkmatterState::Unreadable(Unreadable::Io(std::io::ErrorKind::Other))
            }
            Err(error) => DarkmatterState::Invalid(error),
        },
    };
    ObservedFile { path: Some(path), state }
}

type Typed = (Result<ClaudineConfig, Unusable>, Result<ClaudineConfig, Unusable>, Vec<Finding>);

/// The user configuration alone and merged with the repository overrides,
/// each judged as a normal load judges it, and the `config` findings of
/// their semantic failures.
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
            merge_repo_override(&mut merged, &overrides);
            judged(merged).map_err(|problem| {
                findings.push(match &user_only {
                    // The user file breaks the rule on its own.
                    Err(_) => semantic_finding(&problem, Scope::User, user_path),
                    // Only the merged pair does.
                    Ok(_) => semantic_finding(&problem, Scope::Repository, &repo_path).note(format!(
                        "the rule is broken only when these overrides are merged onto {}",
                        super::report::display_path(user_path)
                    )),
                });
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

/// `config` after the semantic checks of a normal load and the runtime
/// compiler's mapper check.
fn judged(config: ClaudineConfig) -> Result<ClaudineConfig, SemanticProblem> {
    if let Err(error) = config.validate() {
        let setting = match &error {
            ClaudineError::ConfigValidation(message) => named_setting(message),
            _ => None,
        };
        return Err(SemanticProblem::Rule { setting });
    }
    if let Some((event, index)) = crate::dispatch::loader::refused_action_mappers(&config).into_iter().next() {
        return Err(SemanticProblem::Mapper { event: event.to_string(), index });
    }
    Ok(config)
}

/// The setting a validation message leads with (`default_sounds.success:
/// …`), when it is written as a plain setting path; the rest of the message
/// can quote the value and is never kept.
fn named_setting(message: &str) -> Option<String> {
    let (lead, _) = message.split_once(':')?;
    let plain = !lead.is_empty()
        && lead.chars().all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-'));
    if !plain {
        return None;
    }
    let declared = declared_names(ConfigScope::User);
    Some(
        lead.split('.')
            .map(|segment| if declared.contains(segment) { segment } else { "*" })
            .collect::<Vec<_>>()
            .join("."),
    )
}

fn semantic_finding(problem: &SemanticProblem, scope: Scope, path: &Path) -> Finding {
    let summary = match problem {
        SemanticProblem::Typed => "the settings cannot be read as Claudine's configuration".to_string(),
        SemanticProblem::Rule { setting: Some(setting) } => format!("the setting `{setting}` breaks a configuration rule"),
        SemanticProblem::Rule { setting: None } => "a setting breaks a configuration rule".to_string(),
        SemanticProblem::Mapper { event, index } => {
            format!("the mapper of action {index} of `{event}` is not a valid regular expression")
        }
    };
    Finding::new(Severity::Error, scope, summary)
        .at(path)
        .remedy("correct the setting; any other `claudine` command prints the full message")
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
        ClaudineState::Missing { legacy: true } => Finding::new(
            Severity::Error,
            scope,
            "is missing; a legacy `config.json` beside it has not been converted",
        )
        .at(path)
        .note("doctor leaves the legacy file as it is")
        .remedy("run any other `claudine` command, which converts it, or run `claudine setup`"),
        ClaudineState::Missing { legacy: false } if scope == Scope::User => {
            Finding::new(Severity::Error, scope, "is missing").at(path).remedy("run `claudine setup`")
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
    Finding::new(Severity::Error, scope, summary)
        .at(path)
        .remedy("correct it; any other `claudine` command prints the full message")
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
        DarkmatterState::Valid(file) => {
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
    Finding::new(Severity::Error, scope, summary)
        .at(path)
        .remedy("correct it; `md` prints the full message for this file")
}

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
        ClaudineState::Missing { .. } | ClaudineState::Unavailable => {
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
                unknown.sort_by(|left, right| left.pointer.cmp(&right.pointer));
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

const fn config_scope_of(scope: Scope) -> ConfigScope {
    match scope {
        Scope::Repository => ConfigScope::Repository,
        Scope::General | Scope::User => ConfigScope::User,
    }
}

/// ` (line L, column C)`, or nothing when the position is unknown.
fn location(line: Option<usize>, column: Option<usize>) -> String {
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
