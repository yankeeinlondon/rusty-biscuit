//! `claudine doctor`: one passive report over Claudine's setup.
//!
//! [`run`] inspects the captured home and the repository containing the
//! launch directory and returns one [`DoctorRow`] per built-in check, in a
//! fixed order. An inspection writes, creates, converts, or deletes none of
//! Claudine's or Darkmatter's configuration files, the published
//! registration, or anything in the home or user profile; it makes no
//! network request and synthesizes no speech. The one write is on native
//! Windows: each PowerShell probe of the `path` check gets a private folder
//! in the system temporary directory, removed when the probe ends. With
//! `fix`, `run` then attempts the repairs the registry declares and reports
//! the state it finds on disk afterwards.
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
mod path;
mod permissions;
mod registration;
mod report;
mod tts;

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
    /// The directory `claudine doctor` was started in; shells are probed
    /// there.
    launch_dir: PathBuf,
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
        Self { home, launch_dir: snapshot.request_dir().to_path_buf(), context }
    }

    /// An environment launched in the context's working directory, else the
    /// home, else the system temporary directory.
    #[cfg(test)]
    pub(crate) fn for_test(home: Option<PathBuf>, context: Result<FileResolutionContext, ContextUnavailable>) -> Self {
        let launch_dir = match (&context, &home) {
            (Ok(context), _) => context.cwd().to_path_buf(),
            (Err(_), Some(home)) => home.clone(),
            (Err(_), None) => std::env::temp_dir(),
        };
        Self { home, launch_dir, context }
    }

    /// The home directory, when it can be located.
    pub(crate) fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    pub(crate) fn launch_dir(&self) -> &Path {
        &self.launch_dir
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
        check: registration::check_registration,
        repair: Some(registration::repair_registration),
    },
    CheckDescriptor {
        id: "darkmatter-permissions",
        title: "Darkmatter permissions handoff",
        prerequisites: &["config"],
        check: permissions::check_permissions,
        repair: Some(permissions::repair_permissions),
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
        check: tts::check_tts,
        repair: None,
    },
    CheckDescriptor {
        id: "path",
        title: "Executable path",
        prerequisites: &[],
        check: path::check_path,
        repair: None,
    },
];

/// `relative`, written with `/`, as a path with this OS's separators, so a
/// test's expected text matches the native paths the report shows.
#[cfg(test)]
pub(crate) fn native_path(relative: &str) -> PathBuf {
    relative.split('/').collect()
}

/// The registry without the host-probing `tts` and `path` rows, for tests
/// of the file checks that should not detect speech engines or start shells.
#[cfg(test)]
pub(crate) fn file_checks() -> &'static [CheckDescriptor] {
    let (files, host) = REGISTRY.split_at(5);
    debug_assert!(host.iter().map(|check| check.id).eq(["tts", "path"]));
    files
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
