//! Loading Darkmatter's configuration files into one shareable snapshot.
//!
//! [`ConfigSnapshot::load`] reads the user file `~/.darkmatter.yaml` (the
//! home comes from the supplied context, never the process) and, when the
//! context names a repository, `{repo-root}/.darkmatter.yaml` twice: as
//! committed at the captured `HEAD` and as in the working copy. Each is
//! loaded with the admitted-YAML loader and judged only by the shipped
//! configuration schema ([`super::schema`]): no trigger, slot, network
//! access, or permission takes part.
//!
//! Failures are attributed as narrowly as the file allows:
//!
//! - a file that is not one YAML mapping, or that has a key the schema does
//!   not declare, fails the whole load;
//! - an invalid `permissions` value fails only [`ConfigSnapshot::permissions`],
//!   never falling back to defaults;
//! - an invalid `register` entry is a finding that omits only that entry.
//!
//! An absent setting takes its scope's default from the schema, through the
//! shared default engine; each typed value records whether the file wrote it.
//!
//! User-facing guide: `darkmatter/docs/topics/configuration.md`.

mod entries;
mod error;
mod reference;

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext};
use serde_json::Value;
use simplified_schema::yaml_documents::{LoadedYaml, load_yaml_document, locating_problem};
use simplified_schema::{AppliedDefault, SchemaError, ValidationFailure, ValidationProblem};
use sniff::filesystem::git::{CommittedEntry, CommittedTreeSnapshot};

use super::migration_receipt::content_hash;
use super::permissions::{PermissionPolicy, PolicyLayer, RepositoryLayers};
use super::schema::{CONFIG_FILE_NAME, ConfigScope, contract, permissions_contract};

pub(crate) use reference::follow_on_disk;
pub use entries::{
    FileReadEntry, NetworkAccess, PermissionsReference, PermissionsSetting, ScopePermissions, Setting,
    ShellCommandEntry,
};
pub use error::{
    CONFIG_CREATED_CODE, CONFIG_INVALID_CODE, CONFIG_MIGRATION_CODE, CONFIG_MISSING_CODE, CONFIG_NOT_SAVED_CODE, CONFIG_REFERENCE_CODE,
    ConfigDiagnostic, ConfigError, ConfigErrorKind, ConfigProblem, ConfigRevision,
};

/// The committed tree a request's capture took of its repository, or the
/// typed Git error that stopped it. Shared by every derivation of the
/// request, so the tree is read once.
pub type CommittedTreeCapture = Result<Arc<CommittedTreeSnapshot>, Arc<sniff::SniffError>>;

/// A host's captured configuration, projected for each request context it
/// builds: the means by which an embedder's captured context
/// ([`ComposeContext::with_config_source`]) supplies
/// [`ComposeRequest::with_context`] the snapshot for whichever repository a
/// file-resolution context names.
///
/// Implementations answer from evidence retained for the invocation (the user
/// file already read, the committed trees already pinned), never from a new
/// discovery.
///
/// [`ComposeContext::with_config_source`]: crate::markdown::compose::ComposeContext::with_config_source
/// [`ComposeRequest::with_context`]: crate::markdown::compose::ComposeRequest::with_context
pub trait ConfigSnapshotSource: Send + Sync + std::fmt::Debug {
    /// The snapshot for a request resolving through `context`; `None` when
    /// this source holds no capture for the repository `context` names.
    ///
    /// ## Errors
    ///
    /// The configuration's [`ConfigError`] when it cannot be loaded.
    fn config_snapshot_for(&self, context: &FileResolutionContext) -> Option<Result<Arc<ConfigSnapshot>, ConfigError>>;
}

/// Whether a load may migrate legacy policy files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLoadMode {
    /// Read only. Passive tools (DMLS, validators, completion) use this.
    Passive,
    /// A writing CLI's startup load: before reading anything, legacy policy
    /// files of the scopes the context names are converted into missing
    /// `permissions` properties and then deleted
    /// ([`super::migration`]); the notices lead [`ConfigSnapshot::diagnostics`].
    /// A [`Self::Passive`] load reports those files instead. A missing
    /// `~/.darkmatter.yaml` is then created with every setting at its default
    /// ([`super::init::init_user`]) and loaded, reported once as
    /// [`CONFIG_CREATED_CODE`]; only when it cannot be written does the load
    /// fall back to the defaults with [`CONFIG_MISSING_CODE`].
    MigrateLegacy,
    /// [`Self::MigrateLegacy`] without creating the user file: the load of a
    /// setup command (`md init`) whose own transaction creates it, and which
    /// must write nothing when that transaction fails.
    Setup,
}

/// One configuration file the load observed: its path, which revision, and
/// the BLAKE3 hash of its bytes (`None` when it was absent). A change to any
/// of these invalidates the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDependency {
    #[allow(missing_docs)]
    pub revision: ConfigRevision,
    #[allow(missing_docs)]
    pub path: PathBuf,
    /// [`content_hash`] of the bytes read, or `None` for an absent file.
    pub content_hash: Option<String>,
    /// For the target of a followed `permissions` reference, the
    /// configuration file that refers to it; `None` for a configuration
    /// file itself. `revision` is the referring file's.
    pub referenced_from: Option<PathBuf>,
}

/// One observed configuration file and its independently judged settings.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    revision: ConfigRevision,
    path: PathBuf,
    authored: Option<Value>,
    effective: Value,
    defaults: Vec<AppliedDefault>,
    permissions: Result<PermissionsSetting, ConfigError>,
    /// Where the `permissions` value is written, when it is.
    permissions_at: Option<SourcePosition>,
    registrations: Vec<Registration>,
    findings: Vec<ConfigError>,
}

#[allow(missing_docs)]
impl ConfigFile {
    #[must_use]
    pub const fn revision(&self) -> ConfigRevision {
        self.revision
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the file exists in this revision.
    #[must_use]
    pub const fn is_present(&self) -> bool {
        self.authored.is_some()
    }

    /// The document as written; `None` for an absent file.
    #[must_use]
    pub const fn authored(&self) -> Option<&Value> {
        self.authored.as_ref()
    }

    /// The document with every default its valid settings rely on in place.
    /// An invalid setting keeps its authored value.
    #[must_use]
    pub const fn effective(&self) -> &Value {
        &self.effective
    }

    /// The defaults [`Self::effective`] holds, parents before children.
    #[must_use]
    pub fn defaults(&self) -> &[AppliedDefault] {
        &self.defaults
    }

    /// This file's `permissions` setting, or why it is invalid.
    pub fn permissions(&self) -> Result<&PermissionsSetting, &ConfigError> {
        self.permissions.as_ref()
    }

    /// This file's valid `register` entries.
    #[must_use]
    pub fn registrations(&self) -> &[Registration] {
        &self.registrations
    }

    /// This file's invalid `register` entries (or the invalid list itself).
    #[must_use]
    pub fn registration_findings(&self) -> &[ConfigError] {
        &self.findings
    }
}

/// One valid `register` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// The manifest reference as written.
    pub reference: String,
    /// Its position in the file's `register` list.
    pub index: usize,
    /// The file that lists it.
    pub declared_in: PathBuf,
    #[allow(missing_docs)]
    pub revision: ConfigRevision,
    /// Where the entry is written in [`Self::declared_in`], when known.
    pub at: Option<SourcePosition>,
}

/// A 1-based line and character column in a file that was read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePosition {
    #[allow(missing_docs)]
    pub line: usize,
    /// Counted in Unicode characters, a leading byte order mark excluded.
    pub column: usize,
    /// The line's text before [`Self::column`]: what an editor counts the
    /// column from in its own unit (UTF-8 bytes, UTF-16 code units) without
    /// reading the file again.
    pub line_prefix: String,
}

impl SourcePosition {
    /// The first character of a file.
    pub const START: Self = Self { line: 1, column: 1, line_prefix: String::new() };
}

/// The valid registrations of the user file and the repository working
/// copy, and every finding that omitted an entry.
#[derive(Debug, Clone, Default)]
pub struct Registrations {
    /// Valid entries: the user file's first, then the repository's.
    pub entries: Vec<Registration>,
    /// One finding per omitted entry (or per invalid `register` list).
    pub findings: Vec<ConfigError>,
}

/// The repository's two policy projections. Combining them (the narrower
/// of the two) and with the user's is planned permission evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPermissions {
    /// As committed at the captured `HEAD` (defaults when absent there).
    pub head: PermissionsSetting,
    /// As in the working copy (defaults when absent there).
    pub worktree: PermissionsSetting,
}

/// Every scope projection of the `permissions` setting, each valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePermissions {
    /// The user file's block.
    pub user: PermissionsSetting,
    /// The repository's blocks; `None` outside a repository.
    pub repository: Option<RepositoryPermissions>,
}

/// The repository file in both revisions.
#[derive(Debug, Clone)]
pub struct RepositoryConfig {
    /// The repository root the context named.
    pub root: PathBuf,
    /// The file as committed at `HEAD`.
    pub head: ConfigFile,
    /// The file in the working copy.
    pub worktree: ConfigFile,
}

/// Darkmatter's configuration as one request captured it. `Send + Sync`;
/// share it with `Arc`.
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    mode: ConfigLoadMode,
    user: ConfigFile,
    repository: Option<RepositoryConfig>,
    permissions: Result<EffectivePermissions, ConfigError>,
    policy: Result<Arc<PermissionPolicy>, ConfigError>,
    registrations: Registrations,
    dependencies: Vec<ConfigDependency>,
    diagnostics: Vec<ConfigDiagnostic>,
    user_missing: Option<PathBuf>,
    user_created: Option<PathBuf>,
    /// The user file's bytes as read, shared by every projection
    /// ([`Self::with_repository_projection`]) so a request reads it once.
    user_bytes: Option<Arc<[u8]>>,
}

impl ConfigSnapshot {
    /// Loads the configuration `context` names.
    ///
    /// `repository` is the captured committed tree of the repository
    /// `context` names, and must be supplied exactly when it names one; it
    /// is the only source of the committed (`HEAD`) policy.
    ///
    /// ## Errors
    ///
    /// - [`ConfigErrorKind::RepositorySnapshotRequired`] when `context` names
    ///   a repository and `repository` is `None`;
    /// - [`ConfigErrorKind::Read`] when a file or the committed tree cannot
    ///   be read (an absent file is not an error);
    /// - [`ConfigErrorKind::Syntax`], [`ConfigErrorKind::NotAMapping`], or
    ///   [`ConfigErrorKind::UnknownKey`] when a file fails as a whole.
    pub fn load(
        context: &FileResolutionContext,
        repository: Option<&CommittedTreeSnapshot>,
        mode: ConfigLoadMode,
    ) -> Result<Self, ConfigError> {
        let migrated = match mode {
            ConfigLoadMode::MigrateLegacy | ConfigLoadMode::Setup => super::migration::migrate(context)?,
            ConfigLoadMode::Passive => Vec::new(),
        };
        let user_path = context.home_dir().map(|home| home.join(CONFIG_FILE_NAME));
        let mut user_bytes = match &user_path {
            Some(path) => read_worktree(path, ConfigRevision::User)?.map(Arc::from),
            None => None,
        };
        let mut created = None;
        let mut not_created = None;
        if mode == ConfigLoadMode::MigrateLegacy
            && user_bytes.is_none()
            && let Some(path) = &user_path
        {
            match super::init::init_user(context, super::init::InitOptions::default()) {
                Ok(_) => {
                    user_bytes = read_worktree(path, ConfigRevision::User)?.map(Arc::from);
                    created = Some(path.clone());
                }
                Err(error) => not_created = Some(error),
            }
        }
        let mut snapshot = Self::assemble(context, repository, mode, user_bytes)?;
        if let (Some(error), Some(path)) = (not_created, &user_path)
            && let Some(missing) = snapshot.diagnostics.iter_mut().find(|notice| notice.code == CONFIG_MISSING_CODE)
        {
            missing.message = format!(
                "`{}` does not exist and could not be created ({error}), so every setting takes its default",
                path.display()
            );
        }
        let mut notices = match mode {
            ConfigLoadMode::MigrateLegacy | ConfigLoadMode::Setup => migrated,
            ConfigLoadMode::Passive => super::migration::passive_notices(context, &snapshot),
        };
        if let Some(path) = &created {
            notices.push(ConfigDiagnostic {
                code: CONFIG_CREATED_CODE,
                file: path.clone(),
                message: format!("created `{}` with every setting at its default", path.display()),
                problems: Vec::new(),
            });
        }
        snapshot.diagnostics.splice(0..0, notices);
        snapshot.user_created = created;
        Ok(snapshot)
    }

    /// The user file of the home `snapshot` names, with no repository: what a
    /// CLI loads once per invocation before it knows which repositories its
    /// documents live in, then projects with
    /// [`Self::with_repository_projection`]. Discovers nothing.
    ///
    /// ## Errors
    ///
    /// As [`Self::load`] for the user file.
    pub fn load_user(
        snapshot: &crate::markdown::compose::RequestSnapshot,
        mode: ConfigLoadMode,
    ) -> Result<Self, ConfigError> {
        let user_only = FileResolutionContext::from_snapshot(
            snapshot.request_dir(),
            snapshot.home().map(Path::to_path_buf),
            snapshot.env().clone(),
        );
        Self::load(&user_only, None, mode)
    }

    /// [`Self::load`] with the committed tree as a request's capture holds
    /// it: `None` when the capture took none, or the Git error that stopped
    /// it.
    ///
    /// ## Errors
    ///
    /// As [`Self::load`]; a capture error is [`ConfigErrorKind::Read`] on the
    /// repository file at `HEAD`.
    pub fn load_captured(
        context: &FileResolutionContext,
        repository: Option<&CommittedTreeCapture>,
        mode: ConfigLoadMode,
    ) -> Result<Self, ConfigError> {
        Self::load(context, captured_tree(context, repository)?, mode)
    }

    /// This snapshot's user file, re-read from the bytes it already holds,
    /// with the repository `context` names: the projection a source in
    /// another repository (or in none) gets within the same request.
    ///
    /// `repository` follows [`Self::load_captured`]'s rules for `context`.
    ///
    /// ## Errors
    ///
    /// As [`Self::load_captured`].
    pub fn with_repository_projection(
        &self,
        context: &FileResolutionContext,
        repository: Option<&CommittedTreeCapture>,
    ) -> Result<Self, ConfigError> {
        let tree = captured_tree(context, repository)?;
        // A writing CLI loads the user file once, before it knows its
        // sources' repositories; each repository's legacy files migrate here.
        let migrated = match (self.mode, context.repository_root()) {
            (ConfigLoadMode::MigrateLegacy | ConfigLoadMode::Setup, Some(_)) => {
                super::migration::migrate_repository(context)?
            }
            _ => Vec::new(),
        };
        let mut projected = Self::assemble(context, tree, self.mode, self.user_bytes.clone())?;
        projected.diagnostics.splice(0..0, migrated);
        Ok(projected)
    }

    /// [`Self::with_repository_projection`] read only, whatever this
    /// snapshot's mode: no legacy file is migrated, and one is reported
    /// instead. For a passive reader inside a writing CLI (host selection),
    /// which must not write.
    ///
    /// ## Errors
    ///
    /// As [`Self::load_captured`].
    pub fn with_passive_repository_projection(
        &self,
        context: &FileResolutionContext,
        repository: Option<&CommittedTreeCapture>,
    ) -> Result<Self, ConfigError> {
        let tree = captured_tree(context, repository)?;
        Self::assemble(context, tree, ConfigLoadMode::Passive, self.user_bytes.clone())
    }

    fn assemble(
        context: &FileResolutionContext,
        repository: Option<&CommittedTreeSnapshot>,
        mode: ConfigLoadMode,
        user_bytes: Option<Arc<[u8]>>,
    ) -> Result<Self, ConfigError> {
        let portable = context.portable_variables();
        let mut dependencies = Vec::new();
        let mut diagnostics = Vec::new();

        let user_path = context.home_dir().map(|home| home.join(CONFIG_FILE_NAME));
        let user_missing = match (&user_path, &user_bytes) {
            (None, _) => {
                let path = PathBuf::from("~").join(CONFIG_FILE_NAME);
                diagnostics.push(ConfigDiagnostic {
                    code: CONFIG_MISSING_CODE,
                    file: path.clone(),
                    message: format!(
                        "there is no home directory, so `{}` cannot be read and every user setting takes its default; nothing can be saved to it",
                        path.display()
                    ),
                    problems: Vec::new(),
                });
                None
            }
            (Some(path), None) => {
                diagnostics.push(ConfigDiagnostic {
                    code: CONFIG_MISSING_CODE,
                    file: path.clone(),
                    message: format!(
                        "`{}` does not exist, so every setting takes its default; `md init` creates it",
                        path.display()
                    ),
                    problems: Vec::new(),
                });
                Some(path.clone())
            }
            _ => None,
        };
        let tree = repository;
        let user_path = user_path.unwrap_or_else(|| PathBuf::from("~").join(CONFIG_FILE_NAME));
        dependencies.push(dependency(ConfigRevision::User, &user_path, user_bytes.as_deref()));
        let observe = Observe { context, portable: &portable };
        let user = observe.file(user_bytes.as_deref(), &user_path, ConfigRevision::User, ConfigScope::User)?;

        let repository = match (context.repository_root(), repository) {
            (None, _) => None,
            (Some(root), None) => {
                return Err(ConfigError::unlocated(
                    ConfigErrorKind::RepositorySnapshotRequired,
                    &root.join(CONFIG_FILE_NAME),
                    ConfigRevision::RepositoryHead,
                    format!(
                        "the context names the repository `{}` but no committed tree was captured for it",
                        root.display()
                    ),
                ));
            }
            (Some(root), Some(tree)) => {
                let path = root.join(CONFIG_FILE_NAME);
                let head_bytes = read_committed(tree, &path)?;
                let worktree_bytes = read_worktree(&path, ConfigRevision::RepositoryWorktree)?;
                dependencies.push(dependency(ConfigRevision::RepositoryHead, &path, head_bytes.as_deref()));
                dependencies.push(dependency(ConfigRevision::RepositoryWorktree, &path, worktree_bytes.as_deref()));
                let head =
                    observe.file(head_bytes.as_deref(), &path, ConfigRevision::RepositoryHead, ConfigScope::Repository)?;
                let worktree = observe.file(
                    worktree_bytes.as_deref(),
                    &path,
                    ConfigRevision::RepositoryWorktree,
                    ConfigScope::Repository,
                )?;
                Some(RepositoryConfig { root: root.to_path_buf(), head, worktree })
            }
        };

        let permissions = effective_permissions(&user, repository.as_ref());
        let policy = match &permissions {
            Ok(_) => observe.policy(&user, repository.as_ref(), tree, &mut dependencies).map(Arc::new),
            Err(error) => Err(error.clone()),
        };
        // An invalid raw block is already the policy's error; report it once.
        if let Err(error) = permissions.as_ref().and(policy.as_ref()) {
            diagnostics.push(diagnostic(error));
        }
        let mut registrations = Registrations::default();
        for file in std::iter::once(&user).chain(repository.as_ref().map(|repository| &repository.worktree)) {
            registrations.entries.extend(file.registrations.iter().cloned());
            registrations.findings.extend(file.findings.iter().cloned());
        }
        diagnostics.extend(registrations.findings.iter().map(diagnostic));

        Ok(Self {
            mode,
            user,
            repository,
            permissions,
            policy,
            registrations,
            dependencies,
            diagnostics,
            user_missing,
            user_created: None,
            user_bytes,
        })
    }

    /// The mode the snapshot was loaded in.
    #[must_use]
    pub const fn mode(&self) -> ConfigLoadMode {
        self.mode
    }

    /// Every scope projection of `permissions`, or the error that blocks
    /// every permission consumer. Never a default standing in for an
    /// invalid block.
    ///
    /// ## Errors
    ///
    /// The invalid settings of every projection, merged into one error.
    pub fn permissions(&self) -> Result<&EffectivePermissions, ConfigError> {
        self.permissions.as_ref().map_err(Clone::clone)
    }

    /// The evaluated permission policy: every layer's block resolved (a
    /// `permissions` reference followed to its target) and combined by
    /// [`PermissionPolicy::verdict`]. Unlike [`Self::permissions`], which
    /// returns the raw per-file settings, this fails when a reference's
    /// target is missing or invalid.
    ///
    /// ## Errors
    ///
    /// The [`Self::permissions`] error, or the reference error
    /// ([`ConfigErrorKind::InvalidReference`]) naming both files.
    pub fn policy(&self) -> Result<&Arc<PermissionPolicy>, ConfigError> {
        self.policy.as_ref().map_err(Clone::clone)
    }

    /// The valid registrations and the findings that omitted the others.
    #[must_use]
    pub const fn registrations(&self) -> &Registrations {
        &self.registrations
    }

    /// The user file's projection.
    #[must_use]
    pub const fn user(&self) -> &ConfigFile {
        &self.user
    }

    /// The repository file's projections; `None` outside a repository.
    #[must_use]
    pub const fn repository(&self) -> Option<&RepositoryConfig> {
        self.repository.as_ref()
    }

    /// The files the load read, with their content hashes.
    #[must_use]
    pub fn dependencies(&self) -> &[ConfigDependency] {
        &self.dependencies
    }

    /// The path of the user file when it does not exist, so a caller can
    /// report [`CONFIG_MISSING_CODE`] once per invocation.
    #[must_use]
    pub fn user_config_missing(&self) -> Option<&Path> {
        self.user_missing.as_deref()
    }

    /// The path of the user file when this load created it
    /// ([`ConfigLoadMode::MigrateLegacy`]), so a caller can report
    /// [`CONFIG_CREATED_CODE`] once per invocation. A repository projection
    /// never reports it.
    #[must_use]
    pub fn user_config_created(&self) -> Option<&Path> {
        self.user_created.as_deref()
    }

    /// Everything a caller reports: the missing-user-file notice, the
    /// permissions error, and each registration finding.
    #[must_use]
    pub fn diagnostics(&self) -> &[ConfigDiagnostic] {
        &self.diagnostics
    }
}

/// What every file's validation shares.
struct Observe<'a> {
    context: &'a FileResolutionContext,
    portable: &'a BTreeSet<String>,
}

impl Observe<'_> {
    /// Judges one revision of a file; `bytes` is `None` when it is absent.
    fn file(
        &self,
        bytes: Option<&[u8]>,
        path: &Path,
        revision: ConfigRevision,
        scope: ConfigScope,
    ) -> Result<ConfigFile, ConfigError> {
        let located = |message: String, pointer: &str, range: Option<Range<usize>>, text: &str| {
            problem_at(path, revision, pointer, range.map(|range| line_column(text, range.start)), message)
        };
        let (text, loaded) = match bytes {
            None => (String::new(), None),
            Some(bytes) => {
                let text = String::from_utf8(bytes.to_vec()).map_err(|error| {
                    let at = error.utf8_error().valid_up_to();
                    let text = String::from_utf8_lossy(bytes).into_owned();
                    ConfigError::one(
                        ConfigErrorKind::Syntax,
                        located("the file is not valid UTF-8".into(), "", Some(at..at), &text),
                    )
                })?;
                let loaded = load_yaml_document(&text, path).map_err(|error| {
                    let (message, span) = match error {
                        SchemaError::SchemaDocument { message, span, .. } => (message, span),
                        other => (other.to_string(), None),
                    };
                    ConfigError::one(ConfigErrorKind::Syntax, located(message, "", span, &text))
                })?;
                (text, Some(loaded))
            }
        };
        let locate = |pointer: &[String]| -> Option<SourcePosition> {
            let loaded = loaded.as_ref()?;
            byte_range(loaded, pointer).map(|range| line_column(&text, range.start))
        };

        let authored = loaded.as_ref().map(|loaded| loaded.json.clone());
        let document = match &authored {
            None => serde_json::Map::new(),
            Some(Value::Object(map)) => map.clone(),
            Some(other) => {
                let what = if other.is_null() { "is empty" } else { "is not a mapping" };
                return Err(ConfigError::one(
                    ConfigErrorKind::NotAMapping,
                    problem_at(
                        path,
                        revision,
                        "",
                        Some(SourcePosition::START),
                        format!("the configuration {what}; write `{{}}` for one that keeps every default"),
                    ),
                ));
            }
        };
        // The schema is closed: a key it does not declare fails the file.
        if let Err(failure) = contract(scope).validate_strict(&Value::Object(document.clone())) {
            let mut root = Vec::new();
            let mut unknown = Vec::new();
            if let ValidationFailure::InvalidData { problems } = &failure {
                for problem in problems.iter().filter(|problem| problem.instance_path.segments().is_empty()) {
                    let keys: Vec<&String> = if problem.offending_properties.is_empty() {
                        problem.offending_property.iter().collect()
                    } else {
                        problem.offending_properties.iter().collect()
                    };
                    if keys.is_empty() {
                        root.push(problem_at(path, revision, "", Some(SourcePosition::START), message(problem)));
                    }
                    for key in keys {
                        let at = std::slice::from_ref(key);
                        unknown.push(problem_at(
                            path,
                            revision,
                            &pointer_of(at),
                            locate(at),
                            format!("`{key}` is not a configuration setting"),
                        ));
                    }
                }
            }
            if !unknown.is_empty() {
                return Err(ConfigError::new(ConfigErrorKind::UnknownKey, unknown));
            }
            if !root.is_empty() {
                return Err(ConfigError::new(ConfigErrorKind::InvalidSetting, root));
            }
        }

        let mut effective = Value::Object(document.clone());
        let mut defaults = Vec::new();
        let context = SettingContext { path, revision, scope, locate: &locate };

        let permissions = match judge_setting(scope, &document, "permissions") {
            Ok(applied) => {
                apply_defaults(&mut effective, &applied);
                let value = &effective["permissions"];
                let result = self.permissions(value, document.get("permissions"), &context);
                defaults.extend(applied);
                result
            }
            Err(mut problems) => {
                // A written block that fails as a whole failed every arm of
                // the mapping-or-reference union; judge it as the block it
                // is meant to be.
                if let Some(block @ Value::Object(_)) = document.get("permissions")
                    && problems.iter().any(|(segments, _)| segments.len() == 1)
                    && let Err(failure) = permissions_contract(scope).validate_strict(block)
                {
                    problems = failure_problems(&failure)
                        .into_iter()
                        .map(|(mut segments, message)| {
                            segments.insert(0, "permissions".to_string());
                            (segments, message)
                        })
                        .collect();
                }
                let kind = if document.get("permissions").is_some_and(Value::is_string) {
                    ConfigErrorKind::InvalidReference
                } else {
                    ConfigErrorKind::InvalidSetting
                };
                Err(ConfigError::new(kind, context.problems(problems, document.get("permissions"))))
            }
        };

        let (registrations, findings) = match judge_setting(scope, &document, "register") {
            Ok(applied) => {
                apply_defaults(&mut effective, &applied);
                defaults.extend(applied);
                registrations(&effective["register"], &BTreeSet::new(), &context)
            }
            Err(problems) => {
                let mut whole = Vec::new();
                let mut by_entry: BTreeSet<usize> = BTreeSet::new();
                let mut findings = Vec::new();
                for (segments, message) in problems {
                    match segments.get(1).and_then(|index| index.parse::<usize>().ok()) {
                        Some(index) => {
                            by_entry.insert(index);
                            findings.push(ConfigError::one(
                                ConfigErrorKind::InvalidSetting,
                                context.problem(&segments, message),
                            ));
                        }
                        None => whole.push(context.problem(&segments, message)),
                    }
                }
                if whole.is_empty() {
                    let (valid, more) = registrations(&document["register"], &by_entry, &context);
                    findings.extend(more);
                    (valid, findings)
                } else {
                    findings.push(ConfigError::new(ConfigErrorKind::InvalidSetting, whole));
                    (Vec::new(), findings)
                }
            }
        };

        Ok(ConfigFile {
            revision,
            path: path.to_path_buf(),
            authored,
            effective,
            defaults,
            permissions,
            permissions_at: locate(&["permissions".to_string()]),
            registrations,
            findings,
        })
    }

    /// Types a `permissions` value the schema accepted (defaults in place).
    fn permissions(
        &self,
        value: &Value,
        authored: Option<&Value>,
        context: &SettingContext<'_>,
    ) -> Result<PermissionsSetting, ConfigError> {
        if let Value::String(reference) = value {
            return entries::permissions_reference(reference, context.path, context.scope)
                .map(PermissionsSetting::Reference)
                .map_err(|message| {
                    ConfigError::one(
                        ConfigErrorKind::InvalidReference,
                        context.problem(&["permissions".to_string()], message),
                    )
                });
        }
        let written = |key: &str| authored.and_then(|block| block.get(key)).is_some();
        let mut problems = Vec::new();
        let mut file_read = Vec::new();
        for (index, entry) in strings(&value["file-read"]) {
            match entries::file_read_entry(entry, context.scope, self.context, self.portable) {
                Ok(entry) => file_read.push(entry),
                Err(message) => problems.push(context.problem(&entry_path("file-read", index), message)),
            }
        }
        let mut shell_commands = Vec::new();
        for (index, entry) in strings(&value["shell-commands"]) {
            match entries::shell_command_entry(entry) {
                Ok(entry) => shell_commands.push(entry),
                Err(message) => problems.push(context.problem(&entry_path("shell-commands", index), message)),
            }
        }
        if !problems.is_empty() {
            return Err(ConfigError::new(ConfigErrorKind::InvalidSetting, problems));
        }
        // The schema admitted only `allow` or `deny`.
        let network = if value["network"] == "allow" { NetworkAccess::Allow } else { NetworkAccess::Deny };
        Ok(PermissionsSetting::Inline(ScopePermissions {
            file_read: Setting { value: file_read, authored: written("file-read") },
            shell_commands: Setting { value: shell_commands, authored: written("shell-commands") },
            network: Setting { value: network, authored: written("network") },
        }))
    }
}

impl Observe<'_> {
    /// Resolves every layer's block and builds the policy. Every layer is
    /// resolved, so all reference errors are reported together.
    fn policy(
        &self,
        user: &ConfigFile,
        repository: Option<&RepositoryConfig>,
        tree: Option<&CommittedTreeSnapshot>,
        dependencies: &mut Vec<ConfigDependency>,
    ) -> Result<PermissionPolicy, ConfigError> {
        let user_layer = self.layer(user, reference::Where::User, dependencies);
        let repository_layers = match (repository, tree) {
            (Some(repository), Some(tree)) => {
                let root = repository.root.as_path();
                let head = self.layer(&repository.head, reference::Where::Head { root, tree }, dependencies);
                let worktree = self.layer(&repository.worktree, reference::Where::Worktree { root }, dependencies);
                Some((repository.root.clone(), head, worktree))
            }
            _ => None,
        };
        let mut errors: Vec<&ConfigError> = user_layer.as_ref().err().into_iter().collect();
        if let Some((_, head, worktree)) = &repository_layers {
            errors.extend(head.as_ref().err());
            errors.extend(worktree.as_ref().err());
        }
        if let Some(first) = errors.first() {
            let problems = errors.iter().flat_map(|error| error.problems().iter().cloned()).collect();
            return Err(ConfigError::new(first.kind(), problems));
        }
        let user = user_layer?;
        let repository = match repository_layers {
            Some((root, head, worktree)) => Some(RepositoryLayers { root, head: head?, worktree: worktree? }),
            None => None,
        };
        Ok(PermissionPolicy::new(user, repository, self.context))
    }
}

/// Judges one revision of the file at `path` (`bytes`, `None` when absent)
/// as [`ConfigSnapshot::load`] does, without following its reference: what
/// setup ([`super::init`]) reads before it edits a file.
///
/// ## Errors
///
/// As [`ConfigSnapshot::load`] for a file that fails as a whole.
pub(crate) fn observe_file(
    context: &FileResolutionContext,
    bytes: Option<&[u8]>,
    path: &Path,
    revision: ConfigRevision,
    scope: ConfigScope,
) -> Result<ConfigFile, ConfigError> {
    let portable = context.portable_variables();
    Observe { context, portable: &portable }.file(bytes, path, revision, scope)
}

/// Where `scope`'s file is: in the context's home or repository root;
/// `None` when the context has no home or names no repository.
pub(crate) fn working_path(context: &FileResolutionContext, scope: ConfigScope) -> Option<PathBuf> {
    match scope {
        ConfigScope::User => context.home_dir().map(|home| home.join(CONFIG_FILE_NAME)),
        ConfigScope::Repository => context.repository_root().map(|root| root.join(CONFIG_FILE_NAME)),
    }
}

/// The working-copy revision of `scope`'s file.
pub(crate) const fn working_revision(scope: ConfigScope) -> ConfigRevision {
    match scope {
        ConfigScope::User => ConfigRevision::User,
        ConfigScope::Repository => ConfigRevision::RepositoryWorktree,
    }
}

/// Reads and judges the working copy of `scope`'s file, as
/// [`inspect_working_file`] describes, keeping the bytes read (`None` when
/// the file is absent).
///
/// ## Errors
///
/// As [`inspect_working_file`].
pub(crate) fn read_working_file(
    context: &FileResolutionContext,
    scope: ConfigScope,
) -> Result<(Option<Vec<u8>>, ConfigFile), ConfigError> {
    let revision = working_revision(scope);
    let path = working_path(context, scope).ok_or_else(|| match scope {
        ConfigScope::User => ConfigError::unlocated(
            ConfigErrorKind::Read,
            &PathBuf::from("~").join(CONFIG_FILE_NAME),
            revision,
            "there is no home directory, so the user configuration cannot be read".into(),
        ),
        ConfigScope::Repository => ConfigError::unlocated(
            ConfigErrorKind::Read,
            &context.cwd().join(CONFIG_FILE_NAME),
            revision,
            format!("`{}` is not inside a repository, so there is no repository configuration", context.cwd().display()),
        ),
    })?;
    let bytes = read_worktree(&path, revision)?;
    let file = observe_file(context, bytes.as_deref(), &path, revision, scope)?;
    Ok((bytes, file))
}

/// Judges the one working-copy configuration file of `scope` exactly as
/// [`ConfigSnapshot::load`] judges it, passively: the user file
/// `~/.darkmatter.yaml` of the context's home, or the repository's
/// `{repo-root}/.darkmatter.yaml` as in the working copy, against that
/// scope's schema and defaults.
///
/// It reads that one file and nothing else: no committed (`HEAD`) policy,
/// no `permissions` reference followed (the setting is the reference as
/// written), no installed registration, and it never writes: a missing file
/// is not created and a legacy policy file is neither migrated nor deleted.
///
/// An absent file is a [`ConfigFile`] whose [`ConfigFile::is_present`] is
/// `false`. A problem confined to one setting is inside the returned file:
/// an invalid `permissions` value is [`ConfigFile::permissions`]'s error,
/// an invalid `register` entry is one of
/// [`ConfigFile::registration_findings`].
///
/// ## Errors
///
/// A file that fails as a whole ([`ConfigErrorKind::Syntax`],
/// [`ConfigErrorKind::NotAMapping`], [`ConfigErrorKind::UnknownKey`]),
/// located in its file; [`ConfigErrorKind::Read`] when it cannot be read, or
/// when the context has no home (user scope) or names no repository
/// (repository scope).
pub fn inspect_working_file(context: &FileResolutionContext, scope: ConfigScope) -> Result<ConfigFile, ConfigError> {
    read_working_file(context, scope).map(|(_, file)| file)
}

/// The repository-scope policy layer of one committed policy file at the
/// absolute `path` inside `root` (`bytes`, `None` when the commit has no
/// file there), judged as [`ConfigSnapshot::load`] judges `HEAD`: its inline
/// block, or the block its reference names in `tree`, with the repository
/// defaults in place, and every target it followed.
///
/// With `whole_config` the file is a `.darkmatter.yaml`, judged whole
/// against the repository contract. Without it the file belongs to another
/// program (Claudine's `.claudine/config.yaml`) and only its top-level
/// `permissions` value is judged; the rest is that program's.
///
/// ## Errors
///
/// As [`ConfigSnapshot::load`]: a file or block that fails, or a reference
/// that cannot be followed. Never a default in place of a failure.
pub(crate) fn committed_layer(
    context: &FileResolutionContext,
    tree: &CommittedTreeSnapshot,
    root: &Path,
    path: &Path,
    bytes: Option<&[u8]>,
    whole_config: bool,
) -> Result<(PolicyLayer, Vec<ConfigDependency>), ConfigError> {
    let portable = context.portable_variables();
    let observe = Observe { context, portable: &portable };
    let revision = ConfigRevision::RepositoryHead;
    let projected;
    let bytes = match bytes {
        Some(bytes) if !whole_config => {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                ConfigError::unlocated(ConfigErrorKind::Syntax, path, revision, "the file is not valid UTF-8".into())
            })?;
            let loaded = load_yaml_document(text, path).map_err(|error| {
                ConfigError::unlocated(ConfigErrorKind::Syntax, path, revision, error.to_string())
            })?;
            let Value::Object(document) = loaded.json else {
                return Err(ConfigError::unlocated(
                    ConfigErrorKind::NotAMapping,
                    path,
                    revision,
                    "the configuration is not a mapping".into(),
                ));
            };
            // JSON is YAML: the one setting, alone, judged as a whole file.
            let mut alone = serde_json::Map::new();
            if let Some(value) = document.get("permissions") {
                alone.insert("permissions".to_string(), value.clone());
            }
            projected = serde_json::to_vec(&Value::Object(alone)).unwrap_or_default();
            Some(projected.as_slice())
        }
        other => other,
    };
    let file = observe.file(bytes, path, revision, ConfigScope::Repository)?;
    let mut dependencies = vec![dependency(revision, path, bytes)];
    // The path the reference spells, which a committed link may redirect
    // without changing either target.
    if let Ok(PermissionsSetting::Reference(reference)) = &file.permissions {
        dependencies.push(ConfigDependency {
            revision,
            path: super::permissions::lexically_normal(&reference.base_dir.join(&reference.reference)),
            content_hash: None,
            referenced_from: Some(path.to_path_buf()),
        });
    }
    let layer = observe.layer(&file, reference::Where::Head { root, tree }, &mut dependencies)?;
    Ok((layer, dependencies))
}

/// Types the permissions block `value` of `scope` (defaults already in
/// place) with the loader's own entry checks; `path` names it in errors.
///
/// ## Errors
///
/// A [`ConfigError`] naming every invalid entry, or the value when it is not
/// an inline block.
pub(crate) fn typed_block(
    context: &FileResolutionContext,
    value: &Value,
    path: &Path,
    scope: ConfigScope,
) -> Result<ScopePermissions, ConfigError> {
    let portable = context.portable_variables();
    let observe = Observe { context, portable: &portable };
    let locate = |_: &[String]| None;
    let setting = SettingContext { path, revision: ConfigRevision::RepositoryHead, scope, locate: &locate };
    match observe.permissions(value, Some(value), &setting)? {
        PermissionsSetting::Inline(block) => Ok(block),
        PermissionsSetting::Reference(_) => Err(ConfigError::unlocated(
            ConfigErrorKind::InvalidSetting,
            path,
            ConfigRevision::RepositoryHead,
            "the default `permissions` is a reference, not a block".into(),
        )),
    }
}

/// Judges the `permissions` block of `text`, the whole file at `path`, as
/// a writer's candidate: for a configuration file (`whole_config`) the whole
/// document against `scope`'s contract, for a reference's target only its
/// `permissions` mapping; then every entry with the loader's own checks.
///
/// ## Errors
///
/// The first problem found, as a message naming its pointer.
pub(crate) fn check_permissions_text(
    text: &str,
    path: &Path,
    scope: ConfigScope,
    whole_config: bool,
    context: &FileResolutionContext,
) -> Result<ScopePermissions, String> {
    let loaded = load_yaml_document(text, path).map_err(|error| error.to_string())?;
    let first = |problems: Vec<(Vec<String>, String)>| {
        problems
            .into_iter()
            .next()
            .map_or_else(|| "the configuration is invalid".to_string(), |(segments, message)| format!("{}: {message}", pointer_of(&segments)))
    };
    if whole_config && let Err(failure) = contract(scope).validate_strict(&loaded.json) {
        return Err(first(failure_problems(&failure)));
    }
    let Some(block @ Value::Object(_)) = loaded.json.get("permissions") else {
        return Err("`permissions` is not a mapping".to_string());
    };
    let success = permissions_contract(scope).validate_strict(block).map_err(|failure| first(failure_problems(&failure)))?;
    let mut effective = block.clone();
    apply_defaults(&mut effective, &success.defaults);
    let portable = context.portable_variables();
    let observe = Observe { context, portable: &portable };
    let locate = |_: &[String]| None;
    let setting = SettingContext { path, revision: ConfigRevision::User, scope, locate: &locate };
    match observe.permissions(&effective, Some(block), &setting) {
        Ok(PermissionsSetting::Inline(block)) => Ok(block),
        Ok(PermissionsSetting::Reference(_)) => Err("`permissions` is not a mapping".to_string()),
        Err(error) => Err(error.problems().iter().map(|problem| format!("{}: {}", problem.pointer, problem.message)).collect::<Vec<_>>().join("; ")),
    }
}

/// Where the value at a path is written, when it has a position.
type Locate<'a> = dyn Fn(&[String]) -> Option<SourcePosition> + 'a;

/// Where a setting's problems are located.
struct SettingContext<'a> {
    path: &'a Path,
    revision: ConfigRevision,
    scope: ConfigScope,
    locate: &'a Locate<'a>,
}

impl SettingContext<'_> {
    fn problem(&self, segments: &[String], message: String) -> ConfigProblem {
        problem_at(self.path, self.revision, &pointer_of(segments), (self.locate)(segments), message)
    }

    fn problems(&self, problems: Vec<(Vec<String>, String)>, authored: Option<&Value>) -> Vec<ConfigProblem> {
        problems
            .into_iter()
            .map(|(segments, message)| {
                let message = if segments.len() == 1 { describe_permissions(message, authored) } else { message };
                self.problem(&segments, message)
            })
            .collect()
    }
}

/// Judges the one setting `key` of `document` against the scope's contract,
/// with every other setting left out (so it takes its default and cannot
/// fail). The defaults it relied on, or every problem under `key`.
fn judge_setting(
    scope: ConfigScope,
    document: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<AppliedDefault>, Vec<(Vec<String>, String)>> {
    let mut alone = serde_json::Map::new();
    if let Some(value) = document.get(key) {
        alone.insert(key.to_string(), value.clone());
    }
    match contract(scope).validate_strict(&Value::Object(alone)) {
        Ok(success) => Ok(success
            .defaults
            .into_iter()
            .filter(|default| default.path.segments().first().is_some_and(|first| first == key))
            .collect()),
        Err(failure) => Err(failure_problems(&failure)),
    }
}

/// `(instance path segments, message)` of every problem in `failure`.
fn failure_problems(failure: &ValidationFailure) -> Vec<(Vec<String>, String)> {
    match failure {
        ValidationFailure::InvalidData { problems } | ValidationFailure::UnavailableCheck { problems } => {
            problems.iter().map(|problem| (segments(problem), message(problem))).collect()
        }
        ValidationFailure::Unprepared { error } => vec![(Vec::new(), error.to_string())],
        other => vec![(Vec::new(), format!("{other:?}"))],
    }
}

fn segments(problem: &ValidationProblem) -> Vec<String> {
    problem.instance_path.segments().to_vec()
}

fn message(problem: &ValidationProblem) -> String {
    match &problem.offending_property {
        Some(key) if problem.message.contains("Additional properties") => {
            format!("`{key}` is not a setting here")
        }
        _ => problem.message.clone(),
    }
}

/// A union failure at `permissions` itself, worded for the value written.
fn describe_permissions(message: String, authored: Option<&Value>) -> String {
    match authored {
        Some(Value::String(_)) => "the `permissions` reference is blank".to_string(),
        Some(Value::Object(_)) => message,
        Some(_) => "`permissions` must be a mapping or a file reference".to_string(),
        None => message,
    }
}

/// The valid entries of a `register` list the schema accepted, skipping
/// `skip` (entries the schema refused), and a finding for each entry whose
/// reference is unusable.
fn registrations(
    list: &Value,
    skip: &BTreeSet<usize>,
    context: &SettingContext<'_>,
) -> (Vec<Registration>, Vec<ConfigError>) {
    let mut valid = Vec::new();
    let mut findings = Vec::new();
    for (index, reference) in strings(list) {
        if skip.contains(&index) {
            continue;
        }
        let refused = match FileReference::new(reference) {
            Err(error) => Some(format!("`{reference}` is not a file reference: {error}")),
            Ok(parsed) if parsed.class().kind == FileReferenceKind::Url => {
                Some(format!("`{reference}` is remote; a manifest must be a local file"))
            }
            Ok(_) => None,
        };
        match refused {
            Some(message) => findings.push(ConfigError::one(
                ConfigErrorKind::InvalidSetting,
                context.problem(&entry_path("register", index), message),
            )),
            None => valid.push(Registration {
                reference: reference.clone(),
                index,
                declared_in: context.path.to_path_buf(),
                revision: context.revision,
                at: (context.locate)(&entry_path("register", index)),
            }),
        }
    }
    (valid, findings)
}

/// The string items of a list, with their indexes; a non-string item (which
/// the schema refused) is never one.
fn strings(list: &Value) -> impl Iterator<Item = (usize, &String)> {
    list.as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, item)| match item {
            Value::String(text) => Some((index, text)),
            _ => None,
        })
}

/// The path of entry `index` of `list`: `register/2` or
/// `permissions/file-read/2`.
fn entry_path(list: &str, index: usize) -> Vec<String> {
    let mut path = if list == "register" { Vec::new() } else { vec!["permissions".to_string()] };
    path.extend([list.to_string(), index.to_string()]);
    path
}

/// Inserts each default at its path, parents first.
fn apply_defaults(document: &mut Value, defaults: &[AppliedDefault]) {
    for default in defaults {
        let Some((last, parents)) = default.path.segments().split_last() else {
            continue;
        };
        let mut target = &mut *document;
        for segment in parents {
            target = &mut target[segment.as_str()];
        }
        if let Value::Object(map) = target {
            map.insert(last.clone(), default.value.clone());
        }
    }
}

/// The merged permissions error of every projection, or the valid
/// projections.
fn effective_permissions(
    user: &ConfigFile,
    repository: Option<&RepositoryConfig>,
) -> Result<EffectivePermissions, ConfigError> {
    let files = std::iter::once(user)
        .chain(repository.into_iter().flat_map(|repository| [&repository.head, &repository.worktree]));
    let errors: Vec<&ConfigError> = files.filter_map(|file| file.permissions.as_ref().err()).collect();
    if let Some(first) = errors.first() {
        let problems = errors.iter().flat_map(|error| error.problems().iter().cloned()).collect();
        return Err(ConfigError::new(first.kind(), problems));
    }
    let valid = |file: &ConfigFile| file.permissions.clone().expect("errors were returned above");
    Ok(EffectivePermissions {
        user: valid(user),
        repository: repository
            .map(|repository| RepositoryPermissions { head: valid(&repository.head), worktree: valid(&repository.worktree) }),
    })
}

fn diagnostic(error: &ConfigError) -> ConfigDiagnostic {
    let file = error.problems().first().map(|problem| problem.file.clone()).unwrap_or_default();
    ConfigDiagnostic { code: error.code(), file, message: error.to_string(), problems: error.problems().to_vec() }
}

/// The tree a capture holds, or its Git error as a read failure of the
/// repository file at `HEAD`.
fn captured_tree<'a>(
    context: &FileResolutionContext,
    repository: Option<&'a CommittedTreeCapture>,
) -> Result<Option<&'a CommittedTreeSnapshot>, ConfigError> {
    match repository {
        None => Ok(None),
        Some(Ok(tree)) => Ok(Some(tree)),
        Some(Err(error)) => {
            let path = context.repository_root().map_or_else(|| PathBuf::from(CONFIG_FILE_NAME), |root| root.join(CONFIG_FILE_NAME));
            Err(ConfigError::unlocated(
                ConfigErrorKind::Read,
                &path,
                ConfigRevision::RepositoryHead,
                format!("the committed tree cannot be read: {error}"),
            ))
        }
    }
}

fn dependency(revision: ConfigRevision, path: &Path, bytes: Option<&[u8]>) -> ConfigDependency {
    ConfigDependency { revision, path: path.to_path_buf(), content_hash: bytes.map(content_hash), referenced_from: None }
}

/// The working-copy bytes at `path`, or `None` when nothing is there.
fn read_worktree(path: &Path, revision: ConfigRevision) -> Result<Option<Vec<u8>>, ConfigError> {
    read_working_bytes(path)
        .map_err(|error| ConfigError::unlocated(ConfigErrorKind::Read, path, revision, format!("cannot be read: {error}")))
}

/// The working-copy bytes at `path`, or `None` when nothing is there; the
/// I/O error kept for setup, which reports it as its own read failure.
pub(crate) fn read_working_bytes(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The committed bytes of the repository file, or `None` when `HEAD` has
/// none (an unborn `HEAD` included).
fn read_committed(tree: &CommittedTreeSnapshot, path: &Path) -> Result<Option<Vec<u8>>, ConfigError> {
    let refuse = |message: String| {
        Err(ConfigError::unlocated(ConfigErrorKind::Read, path, ConfigRevision::RepositoryHead, message))
    };
    match tree.read_file(CONFIG_FILE_NAME) {
        Ok(CommittedEntry::Blob { bytes, .. }) => Ok(Some(bytes)),
        Ok(CommittedEntry::Absent) => Ok(None),
        Ok(CommittedEntry::Symlink { .. }) => refuse("is committed as a symbolic link, which is never followed".into()),
        Ok(CommittedEntry::Directory | CommittedEntry::Submodule { .. }) => {
            refuse("is committed as a directory or submodule, not a file".into())
        }
        Ok(_) => refuse("is committed as an entry this version cannot read".into()),
        Err(error) => refuse(format!("the committed tree cannot be read: {error}")),
    }
}

fn byte_range(loaded: &LoadedYaml, segments: &[String]) -> Option<Range<usize>> {
    loaded.byte_range(&locating_problem(segments))
}

/// The one-based position of byte `offset` in `text`, its column counted in
/// characters (a leading BOM excluded). Registration, configuration, and the
/// other authored records built from it publish this column; a frontmatter
/// schema validation problem (`dm.schema.*`) does not, and keeps the
/// validator's byte column.
pub fn line_column(text: &str, offset: usize) -> SourcePosition {
    let offset = offset.min(text.len());
    let before = &text[..text.floor_char_boundary(offset)];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    let line_prefix = before[line_start..].trim_start_matches('\u{feff}');
    SourcePosition { line, column: line_prefix.chars().count() + 1, line_prefix: line_prefix.to_string() }
}

/// The position a parser reported as a 1-based `line` and `column` in
/// `text`, the column counted in characters (a YAML parser's) or, when
/// `bytes`, in bytes (`serde_json`'s); `None` past the last line.
pub(crate) fn reported_position(text: &str, line: usize, column: usize, bytes: bool) -> Option<SourcePosition> {
    let start = match line {
        0 | 1 => 0,
        _ => text.match_indices('\n').nth(line - 2)?.0 + 1,
    };
    let rest = &text[start..];
    let content = &rest[..rest.find('\n').unwrap_or(rest.len())];
    let skipped = if start == 0 && !bytes { content.len() - content.trim_start_matches('\u{feff}').len() } else { 0 };
    let within = if bytes {
        column.saturating_sub(1).min(content.len())
    } else {
        content[skipped..].char_indices().nth(column.saturating_sub(1)).map_or(content.len(), |(at, _)| skipped + at)
    };
    Some(line_column(text, start + within))
}

fn pointer_of(segments: &[String]) -> String {
    simplified_schema::yaml_documents::pointer_string(segments)
}

fn problem_at(
    path: &Path,
    revision: ConfigRevision,
    pointer: &str,
    at: Option<SourcePosition>,
    message: String,
) -> ConfigProblem {
    ConfigProblem {
        file: path.to_path_buf(),
        revision,
        pointer: pointer.to_string(),
        line: at.as_ref().map(|at| at.line),
        column: at.as_ref().map(|at| at.column),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_column_count_characters_and_skip_a_byte_order_mark() {
        let text = "\u{feff}a: 1\nbé: [x]\n";
        let at = |offset: usize| {
            let position = line_column(text, offset);
            (position.line, position.column, position.line_prefix)
        };
        assert_eq!(at(3), (1, 1, String::new()));
        assert_eq!(at(text.find('[').unwrap()), (2, 5, "bé: ".to_string()));
    }

    #[test]
    fn a_reported_position_is_read_in_its_parsers_unit() {
        let text = "\u{feff}é: 1\n🧪 x: [y]\n";
        let at = |line, column, bytes| reported_position(text, line, column, bytes).map(|position| (position.line, position.column));
        assert_eq!(at(1, 3, false), Some((1, 3)), "characters, past the byte order mark");
        assert_eq!(at(2, 6, true), Some((2, 3)), "bytes: `🧪 ` is five");
        assert_eq!(at(2, 3, false), Some((2, 3)));
        assert_eq!(at(9, 1, false), None);
    }

    #[test]
    fn the_snapshot_is_shareable_across_threads() {
        fn shareable<T: Send + Sync>() {}
        shareable::<ConfigSnapshot>();
    }
}
