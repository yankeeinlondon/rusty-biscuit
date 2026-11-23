//! `claudine doctor`: one passive report over Claudine's setup.
//!
//! [`run`] inspects the captured home and the repository containing the
//! launch directory and returns one [`DoctorRow`] per built-in check, in a
//! fixed order. An inspection writes nothing, makes no network request, and
//! synthesizes no speech; with `fix` it then attempts the repairs the
//! registry declares and reports the state it finds on disk afterwards.
//!
//! The CLI adapter (`claudine-cli`'s `commands/doctor.rs`) only renders the
//! rows; every finding and every severity is decided here.
//!
//! User-facing reference: `claudine/docs/cli/doctor.md`.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::RequestSnapshot;
use serde::Serialize;

pub(crate) mod config;
mod report;

#[cfg(test)]
mod tests;

pub(crate) use report::{CheckOutcome, Finding, RepairOutcome, Scope};

/// How much a row's findings matter, least to most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The check ran and found nothing wrong.
    Ok,
    /// An observation, or a check its prerequisites did not allow to run.
    Info,
    /// A problem that does not stop Claudine.
    Warn,
    /// A problem that does.
    Error,
}

impl Severity {
    /// The lowercase name the JSON report uses (`ok`, `info`, `warn`, `error`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One check's result, as both report formats show it.
///
/// Serializes to the stable JSON object of `claudine doctor --json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorRow {
    /// The check's stable identifier (`config`, `registration`, …).
    pub id: &'static str,
    /// The worst finding of the row.
    pub severity: Severity,
    /// One line, without a trailing period.
    pub summary: String,
    /// Per-file findings and their remedies, one per line; empty when unused.
    pub detail: String,
    /// Whether an automatic repair (`--fix`) applies to a finding of this row.
    pub fixable: bool,
}

/// The title of the check `id`, as the human report heads its row; `None`
/// for an unknown identifier.
#[must_use]
pub fn title_of(id: &str) -> Option<&'static str> {
    REGISTRY.iter().find(|check| check.id == id).map(|check| check.title)
}

/// Runs every check for `snapshot`'s home and launch repository, and, when
/// `fix` is set, the eligible repairs.
///
/// Always returns one row per check, in registry order, whether or not a
/// check could run.
#[must_use]
pub fn run(snapshot: &RequestSnapshot, fix: bool) -> Vec<DoctorRow> {
    let environment = Environment::capture(snapshot);
    run_with(&REGISTRY, &environment, fix)
}

/// What every pass of one invocation shares: where the home and repository
/// are, captured once.
#[derive(Debug)]
pub(crate) struct Environment {
    home: Option<PathBuf>,
    /// The launch file-resolution context, or why it could not be built.
    context: Result<FileResolutionContext, ContextUnavailable>,
}

/// The launch context could not be built; the reason is not shown, since
/// it can name authored values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextUnavailable;

impl Environment {
    /// Captures the launch context with Claudine's own repository discovery
    /// and magic roots, once per invocation.
    fn capture(snapshot: &RequestSnapshot) -> Self {
        let context = crate::composition::capture_file_resolution_context(snapshot).map_err(|_| ContextUnavailable);
        let home = match &context {
            Ok(context) => context.home_dir().map(Path::to_path_buf),
            Err(_) => snapshot.home().map(Path::to_path_buf),
        };
        Self { home, context }
    }

    #[cfg(test)]
    pub(crate) fn for_test(home: Option<PathBuf>, context: Result<FileResolutionContext, ContextUnavailable>) -> Self {
        Self { home, context }
    }

    /// The home directory, when it can be located.
    pub(crate) fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    pub(crate) fn context(&self) -> Result<&FileResolutionContext, ContextUnavailable> {
        self.context.as_ref().map_err(|unavailable| *unavailable)
    }

    /// The repository containing the launch directory, when there is one.
    pub(crate) fn repository_root(&self) -> Option<&Path> {
        self.context.as_ref().ok().and_then(FileResolutionContext::repository_root)
    }
}

/// One reading of the disk: the observations every check of the pass
/// shares, each collected the first time a check asks for it.
pub(crate) struct Pass<'a> {
    environment: &'a Environment,
    config: OnceCell<config::ConfigObservations>,
}

impl<'a> Pass<'a> {
    fn new(environment: &'a Environment) -> Self {
        Self { environment, config: OnceCell::new() }
    }

    #[cfg_attr(not(test), expect(dead_code, reason = "the registration and path checks read the environment"))]
    pub(crate) const fn environment(&self) -> &'a Environment {
        self.environment
    }

    /// The four configuration files and the typed configuration they yield.
    pub(crate) fn config(&self) -> &config::ConfigObservations {
        self.config.get_or_init(|| config::observe(self.environment))
    }
}

/// One built-in check.
pub(crate) struct CheckDescriptor {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    /// Checks whose rows a repair of this one can change, so they are read
    /// again after it is attempted.
    pub(crate) prerequisites: &'static [&'static str],
    pub(crate) check: fn(&Pass<'_>) -> CheckOutcome,
    pub(crate) repair: Option<fn(&Pass<'_>) -> RepairOutcome>,
}

/// The built-in checks, in report order.
static REGISTRY: [CheckDescriptor; 7] = [
    CheckDescriptor {
        id: "config",
        title: "Configuration files",
        prerequisites: &[],
        check: config::check_config,
        repair: None,
    },
    CheckDescriptor {
        id: "registration",
        title: "Darkmatter host registration",
        prerequisites: &["config"],
        check: pending_check,
        repair: None,
    },
    CheckDescriptor {
        id: "darkmatter-permissions",
        title: "Darkmatter permissions handoff",
        prerequisites: &["config"],
        check: pending_check,
        repair: None,
    },
    CheckDescriptor {
        id: "repo-config",
        title: "Repository configuration",
        prerequisites: &[],
        check: config::check_repo_config,
        repair: None,
    },
    CheckDescriptor {
        id: "stray-config-keys",
        title: "Unknown configuration keys",
        prerequisites: &[],
        check: config::check_stray_keys,
        repair: None,
    },
    CheckDescriptor {
        id: "tts",
        title: "Text-to-speech",
        prerequisites: &[],
        check: pending_check,
        repair: None,
    },
    CheckDescriptor {
        id: "path",
        title: "Executable path",
        prerequisites: &[],
        check: pending_check,
        repair: None,
    },
];

/// The stand-in for a check whose callback is not built yet.
fn pending_check(_: &Pass<'_>) -> CheckOutcome {
    CheckOutcome::from(vec![Finding::new(Severity::Info, Scope::General, "this check is not available yet")])
}

/// Inspects, then (with `fix`) repairs and inspects the affected rows again.
///
/// Every repair reads the disk afresh, in registry order, so an earlier
/// repair's edit of a shared file is what a later one starts from. After
/// all attempts, each attempted row and the rows it depends on are read once
/// more from a new pass.
pub(crate) fn run_with(registry: &[CheckDescriptor], environment: &Environment, fix: bool) -> Vec<DoctorRow> {
    let initial = Pass::new(environment);
    let mut outcomes: Vec<CheckOutcome> = registry.iter().map(|check| (check.check)(&initial)).collect();
    let mut attempts: Vec<Option<RepairOutcome>> = registry.iter().map(|_| None).collect();

    if fix {
        for (index, check) in registry.iter().enumerate() {
            let Some(repair) = check.repair else { continue };
            let fresh = Pass::new(environment);
            if !(check.check)(&fresh).is_fixable() {
                continue;
            }
            attempts[index] = Some(repair(&fresh));
        }
        let reread: Vec<usize> = registry
            .iter()
            .enumerate()
            .filter(|(index, check)| {
                attempts[*index].is_some()
                    || registry.iter().zip(&attempts).any(|(attempted, attempt)| {
                        attempt.is_some() && attempted.prerequisites.contains(&check.id)
                    })
            })
            .map(|(index, _)| index)
            .collect();
        if !reread.is_empty() {
            let after = Pass::new(environment);
            for index in reread {
                outcomes[index] = (registry[index].check)(&after);
            }
        }
    }

    registry
        .iter()
        .zip(outcomes)
        .zip(attempts)
        .map(|((check, outcome), attempt)| report::row(check.id, outcome, attempt.as_ref()))
        .collect()
}
