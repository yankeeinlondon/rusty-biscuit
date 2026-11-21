//! The request every `md` route resolves file references against.
//!
//! `main` reads the process once ([`RequestSnapshot::from_process`]), adds the
//! `--magic-root` directories ([`with_magic_roots`]), and hands the resulting
//! [`MdRequest`] to every route. A route never reads the current
//! directory, home directory, or environment itself: document arguments,
//! transclusion targets, and schema file values all resolve against contexts
//! built from this one snapshot. See `darkmatter/docs/topics/compose-requests.md`.
//! Shell completion returns before `main` reaches that read, and takes its own
//! through [`completion_request`].
//!
//! The request also owns the invocation's configuration: the user file is
//! read once ([`MdRequest::user_config`]), and each repository a document
//! resolves in gets one projection of it ([`MdRequest::config_snapshot`]),
//! built from the committed tree the context builder's own discovery pinned.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileReference, FileResolutionContext, PathPosition};
use color_eyre::eyre::{Context, Result};
use darkmatter::config::load::{CommittedTreeCapture, ConfigError, ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::registration::RegisteredHosts;
use darkmatter::markdown::compose::{RequestSnapshot, capture_resolution_context};

/// A `--magic-root` directory that is not a directory on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicRootError {
    /// The directory as the caller spelled it.
    pub argument: PathBuf,
    /// The directory the argument names from the launch directory.
    pub resolved: PathBuf,
}

impl fmt::Display for MagicRootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "--magic-root {}: {} is not a directory",
            self.argument.display(),
            self.resolved.display()
        )
    }
}

impl std::error::Error for MagicRootError {}

/// `snapshot` with each `--magic-root` directory registered as an extra `@`
/// root, in the order given, ahead of its tier's default roots.
///
/// A relative directory is relative to the snapshot's (launch) directory. The
/// tier is inferred from where the directory lies: inside the launch
/// repository it is searched with the local roots, elsewhere with the user
/// roots before the home directory.
///
/// ## Errors
///
/// [`MagicRootError`] for the first directory that does not exist or is not a
/// directory; a root that cannot be searched is never silently skipped.
pub fn with_magic_roots(
    snapshot: RequestSnapshot,
    roots: &[PathBuf],
) -> Result<RequestSnapshot, MagicRootError> {
    let mut snapshot = snapshot;
    for argument in roots {
        let resolved = snapshot.request_dir().join(argument);
        if !resolved.is_dir() {
            return Err(MagicRootError { argument: argument.clone(), resolved });
        }
        snapshot = snapshot.with_magic_root(resolved, PathPosition::Start);
    }
    Ok(snapshot)
}

/// The request for one Tab press, from the `--magic-root` words before the cursor.
///
/// A directory that does not exist yet is left out, because completion never
/// reports a problem. `None` when the current directory cannot be read.
pub fn completion_request(magic_roots: &[PathBuf]) -> Option<MdRequest> {
    let snapshot = RequestSnapshot::from_process().ok()?;
    let usable: Vec<PathBuf> = magic_roots.iter().filter(|root| snapshot.request_dir().join(root).is_dir()).cloned().collect();
    with_magic_roots(snapshot, &usable).ok().map(MdRequest::new)
}

/// One `md` invocation's request snapshot and its lazily built launch context.
///
/// The launch context (repository discovery from the launch directory) is
/// built on first use, so a route that resolves nothing never pays for
/// discovery and never fails on a repository it does not need.
#[derive(Debug)]
pub struct MdRequest {
    snapshot: RequestSnapshot,
    launch: OnceCell<FileResolutionContext>,
    /// The committed tree each built context's discovery pinned, by the
    /// repository root that context names.
    trees: RefCell<HashMap<PathBuf, CommittedTreeCapture>>,
    /// The user file, loaded once with no repository.
    user: OnceCell<Result<Arc<ConfigSnapshot>, ConfigError>>,
    /// [`ConfigLoadMode::MigrateLegacy`], or [`ConfigLoadMode::Setup`] once
    /// [`Self::report_config_notices_for_init`] chose it before the load.
    load_mode: std::cell::Cell<ConfigLoadMode>,
    /// One projection per repository root (`None`: no repository).
    configs: RefCell<HashMap<Option<PathBuf>, Arc<ConfigSnapshot>>>,
    /// Whether run notices go to a data output's `diagnostics` (set by
    /// `main` with [`Self::report_config_notices`]); a repository
    /// projection's migration notices follow the same route.
    data_output: std::cell::Cell<bool>,
    /// The registered hosts of each configuration projection, by repository
    /// root, loaded once each.
    hosts: RefCell<HashMap<Option<PathBuf>, Arc<RegisteredHosts>>>,
    /// Every host-registration record already reported, so each reaches the
    /// output once per invocation.
    reported_hosts: RefCell<Vec<darkmatter::markdown::compose::expression::Diagnostic>>,
}

impl MdRequest {
    /// Wraps the invocation's snapshot.
    pub fn new(snapshot: RequestSnapshot) -> Self {
        Self {
            snapshot,
            launch: OnceCell::new(),
            trees: RefCell::default(),
            user: OnceCell::new(),
            load_mode: std::cell::Cell::new(ConfigLoadMode::MigrateLegacy),
            configs: RefCell::default(),
            data_output: std::cell::Cell::new(false),
            hosts: RefCell::default(),
            reported_hosts: RefCell::default(),
        }
    }

    /// The user configuration file, loaded once per invocation with
    /// [`ConfigLoadMode::MigrateLegacy`] (`md` is a writing CLI, so a missing
    /// file is created at its defaults; `md init` uses
    /// [`ConfigLoadMode::Setup`]) and no repository: what the run notices
    /// are judged from, and the bytes every [`Self::config_snapshot`]
    /// projection shares.
    ///
    /// ## Errors
    ///
    /// The load's [`ConfigError`], kept for every later caller.
    pub fn user_config(&self) -> Result<&Arc<ConfigSnapshot>, &ConfigError> {
        self.user
            .get_or_init(|| ConfigSnapshot::load_user(&self.snapshot, self.load_mode.get()).map(Arc::new))
            .as_ref()
    }

    /// The configuration snapshot for a request resolving through `context`:
    /// the user file's shared bytes projected onto the repository `context`
    /// names, cached per repository. `context` must come from this request
    /// ([`Self::launch_context`] or [`Self::document_context`]), whose
    /// discovery pinned that repository's committed tree; nothing is
    /// discovered again here.
    ///
    /// ## Errors
    ///
    /// The configuration's located [`ConfigError`] when the user or
    /// repository file cannot be loaded.
    pub fn config_snapshot(&self, context: &FileResolutionContext) -> Result<Arc<ConfigSnapshot>> {
        let key = context.repository_root().map(Path::to_path_buf);
        if let Some(snapshot) = self.configs.borrow().get(&key) {
            return Ok(Arc::clone(snapshot));
        }
        let user = self.user_config().map_err(|error| color_eyre::Report::new(error.clone()))?;
        let trees = self.trees.borrow();
        let tree = key.as_ref().and_then(|root| trees.get(root));
        let snapshot = Arc::new(user.with_repository_projection(context, tree)?);
        drop(trees);
        // The user scope's notices were reported once by `main`; a
        // repository's legacy-file migration is reported as it happens.
        if let Some(root) = &key {
            let migrated: Vec<_> = snapshot
                .diagnostics()
                .iter()
                .filter(|notice| notice.code == darkmatter::config::load::CONFIG_MIGRATION_CODE && notice.file.starts_with(root))
                .flat_map(darkmatter::markdown::compose::report_diagnostics::config_notice_diagnostics)
                .collect();
            crate::output::report_run_notices(migrated, self.data_output.get());
        }
        self.configs.borrow_mut().insert(key, Arc::clone(&snapshot));
        Ok(snapshot)
    }

    /// The registered hosts of the configuration `context` projects, loaded
    /// once per repository for the invocation; their load and conflict
    /// warnings are reported once, on stderr or in a data output's
    /// `diagnostics`. `None` when the configuration cannot be loaded: that
    /// error is reported by the commands that read it.
    ///
    /// Passive: it reuses a projection the command already loaded
    /// ([`Self::config_snapshot`]) and otherwise projects the repository read
    /// only, so selecting a host never migrates a legacy policy file.
    pub fn registered_hosts(&self, context: &FileResolutionContext) -> Option<Arc<RegisteredHosts>> {
        let key = context.repository_root().map(Path::to_path_buf);
        if let Some(hosts) = self.hosts.borrow().get(&key) {
            return Some(Arc::clone(hosts));
        }
        let loaded = self.configs.borrow().get(&key).cloned();
        let snapshot = match loaded {
            Some(snapshot) => snapshot,
            None => {
                let user = self.user_config().ok()?;
                let trees = self.trees.borrow();
                let tree = key.as_ref().and_then(|root| trees.get(root));
                Arc::new(user.with_passive_repository_projection(context, tree).ok()?)
            }
        };
        let hosts = Arc::new(RegisteredHosts::load(&snapshot, context));
        self.report_host_diagnostics(&hosts.diagnostics());
        self.hosts.borrow_mut().insert(key, Arc::clone(&hosts));
        Some(hosts)
    }

    /// Reports host-registration and selection warnings (`dm.host.*`) as
    /// run notices, each distinct record once per invocation.
    pub fn report_host_diagnostics(&self, diagnostics: &[darkmatter::config::registration::RegistrationDiagnostic]) {
        let mut reported = self.reported_hosts.borrow_mut();
        let fresh: Vec<_> = diagnostics
            .iter()
            .map(darkmatter::markdown::compose::report_diagnostics::registration_diagnostic)
            .filter(|diagnostic| !reported.contains(diagnostic))
            .collect();
        reported.extend(fresh.iter().cloned());
        drop(reported);
        crate::output::report_run_notices(fresh, self.data_output.get());
    }

    /// Reports [`Self::config_notices`] once, on stderr or (for a data
    /// output) in the result's `diagnostics`, and routes any later
    /// repository migration notice the same way.
    pub fn report_config_notices(&self, data_output: bool) {
        self.data_output.set(data_output);
        crate::output::report_run_notices(self.config_notices(), data_output);
    }

    /// [`Self::report_config_notices`] for `md init`, which creates the user
    /// file itself: the load uses [`ConfigLoadMode::Setup`], so nothing is
    /// written before `init`'s own transaction (which writes nothing when it
    /// fails), and a `dm.config.missing` warning would point at the very file
    /// it is about to write, so only the other notices are reported. Must run
    /// before anything else loads the user file.
    pub fn report_config_notices_for_init(&self, data_output: bool) {
        self.load_mode.set(ConfigLoadMode::Setup);
        self.data_output.set(data_output);
        let notices = self
            .config_notices()
            .into_iter()
            .filter(|notice| notice.code != darkmatter::config::load::CONFIG_MISSING_CODE)
            .collect();
        crate::output::report_run_notices(notices, data_output);
    }

    /// The run-level notices of the invocation's configuration: the created
    /// user file's `dm.config.created` info (or, when it could not be
    /// written, the `dm.config.missing` warning) and the user scope's
    /// `dm.config.migration` notices. A user file that
    /// fails to load is reported by the commands that read it, not here.
    pub fn config_notices(&self) -> Vec<darkmatter::markdown::compose::expression::Diagnostic> {
        let Ok(user) = self.user_config() else {
            return Vec::new();
        };
        user.diagnostics()
            .iter()
            .filter(|notice| {
                notice.code == darkmatter::config::load::CONFIG_MISSING_CODE
                    || notice.code == darkmatter::config::load::CONFIG_CREATED_CODE
                    || notice.code == darkmatter::config::load::CONFIG_MIGRATION_CODE
            })
            .flat_map(darkmatter::markdown::compose::report_diagnostics::config_notice_diagnostics)
            .collect()
    }

    /// Builds a context for `snapshot`, keeping the committed tree its
    /// discovery pinned for [`Self::config_snapshot`].
    fn build_context(&self, snapshot: &RequestSnapshot) -> Result<FileResolutionContext, darkmatter::markdown::compose::ContextBuildError> {
        let (context, tree) = capture_resolution_context(snapshot)?;
        if let (Some(root), Some(tree)) = (context.repository_root(), tree) {
            self.trees.borrow_mut().entry(root.to_path_buf()).or_insert(tree);
        }
        Ok(context)
    }

    /// The snapshot `main` captured.
    pub fn snapshot(&self) -> &RequestSnapshot {
        &self.snapshot
    }

    /// The directory `md` was launched from.
    pub fn launch_dir(&self) -> &Path {
        self.snapshot.request_dir()
    }

    /// The context built at the launch directory.
    ///
    /// ## Errors
    ///
    /// Returns the builder's error, for example when repository discovery at
    /// the launch directory fails.
    pub fn launch_context(&self) -> Result<&FileResolutionContext> {
        if let Some(context) = self.launch.get() {
            return Ok(context);
        }
        let context = self.build_context(&self.snapshot).wrap_err_with(|| {
            format!(
                "Failed to prepare file resolution at the launch directory {}",
                self.launch_dir().display()
            )
        })?;
        Ok(self.launch.get_or_init(|| context))
    }

    /// The context a document at `resolved`, opened as `opening`, resolves its
    /// own references against.
    ///
    /// A document inside the launch repository derives from the launch context
    /// (`for_source_reference` / `for_source`), so it keeps the launch
    /// repository's package catalog and `@` scope. Any other document gets a
    /// context built at its own directory, keeping this request's home and
    /// environment: when that build finds a repository (the launch directory
    /// is in another repository, or in none) the document derives from it as a
    /// trusted external source, so its links, `&` references, and trigger
    /// schemas see its own repository, while its `@` references keep the
    /// launch `@` scope (the launch tree and `--magic-root` directories). A
    /// document in no repository, outside the launch repository, also derives
    /// as a trusted external source; one in no repository launched from no
    /// repository keeps the launch derivation.
    ///
    /// ## Errors
    ///
    /// Returns the builder's error for the launch directory or, for a document
    /// outside the launch repository, for the document's directory.
    pub fn document_context(
        &self,
        opening: Option<&FileReference>,
        resolved: &Path,
    ) -> Result<FileResolutionContext> {
        let launch = self.launch_context()?;
        let derived = match opening {
            Some(reference) => launch.for_source_reference(reference, resolved),
            None => launch.for_source(resolved),
        };
        let derived_is_valid = derived.validate().is_ok();
        if derived_is_valid && launch.repository_root().is_some() {
            return Ok(derived);
        }
        let source_dir = resolved.parent().unwrap_or(self.launch_dir());
        let external = self
            .build_context(&self.snapshot.at_request_dir(source_dir))
            .wrap_err_with(|| {
                format!("Failed to prepare file resolution for {}", resolved.display())
            })?;
        // A launch directory outside every repository derives any document
        // without complaint, but its fallback tree knows no repository; only
        // the document's own discovery can tell whether one contains it.
        if derived_is_valid && external.repository_root().is_none() {
            return Ok(derived);
        }
        let source = match opening {
            Some(reference) => external.for_trusted_external_source_reference(reference, resolved),
            None => external.for_trusted_external_source(resolved),
        };
        // The rebuild re-anchors `./`, bare, `&`, and `^` on the document's
        // repository; `@` keeps searching the launch tree, as it does for a
        // source derived from the launch context.
        Ok(source.with_launch_magic_scope(launch.launch_magic_scope().clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_roots_resolve_from_the_launch_directory_in_order() {
        let launch = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(launch.path().join("first")).unwrap();
        let second = tempfile::tempdir().unwrap();

        let snapshot = with_magic_roots(
            RequestSnapshot::new(launch.path()),
            &[PathBuf::from("first"), second.path().to_path_buf()],
        )
        .unwrap();

        let roots: Vec<_> = snapshot.magic_roots().iter().map(|(path, position, _)| (path.clone(), *position)).collect();
        assert_eq!(
            roots,
            [
                (launch.path().join("first"), PathPosition::Start),
                (second.path().to_path_buf(), PathPosition::Start),
            ]
        );
    }

    #[test]
    fn a_missing_magic_root_is_an_error() {
        let launch = tempfile::tempdir().unwrap();
        std::fs::write(launch.path().join("file.md"), "").unwrap();

        for argument in ["missing", "file.md"] {
            let error = with_magic_roots(RequestSnapshot::new(launch.path()), &[PathBuf::from(argument)])
                .unwrap_err();
            assert_eq!(
                error,
                MagicRootError { argument: PathBuf::from(argument), resolved: launch.path().join(argument) }
            );
        }
    }
}
