//! Setting up Darkmatter's configuration files: what `md init` does, and the
//! pieces a host program's setup (Claudine's) shares with it.
//!
//! [`init`] creates `~/.darkmatter.yaml` (and, for
//! [`ConfigScope::Repository`], `{repo-root}/.darkmatter.yaml`) with every
//! property written at its schema default. An existing file keeps every
//! authored value, comment, reference, and `register` entry; only a missing
//! top-level property is added, so a second run writes nothing. A
//! `permissions` reference is left in place unless [`InitOptions::inline`]
//! asks for it to be replaced by the block it names, or by the schema
//! defaults (with an [`InitNotice`]) when that block cannot be read.
//!
//! Every write goes through one [`ConfigTransaction`]: each target is locked,
//! re-read, edited in place, and validated before it is replaced. Nothing is
//! written unless every scope asked for can be written: a missing home or a
//! context that names no repository fails before any file is touched.
//!
//! User-facing guide: `darkmatter/docs/topics/configuration.md` ("Setting
//! up").

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use serde_json::{Map, Value};
use simplified_schema::PropertyPath;
use simplified_schema::yaml_documents::load_yaml_document;

use super::load::{
    CONFIG_NOT_SAVED_CODE, ConfigError, ConfigRevision, check_permissions_text, follow_on_disk, observe_file,
};
use super::schema::{CONFIG_FILE_NAME, ConfigScope, contract, permissions_contract};
use super::write::{
    CommitOutcome, ConfigTransaction, ConfigWriteError, SourceExpectation, TargetChange, ValidationCandidate,
    ValidationRefusal,
};
use crate::yaml_edit::YamlEdit;

/// What `md init` was asked to do beyond creating missing settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InitOptions {
    /// Replace a `permissions` reference with the block it names, or with the
    /// schema defaults when that block cannot be read. An explicit repair;
    /// an ordinary run never does it.
    pub inline: bool,
}

/// What happened to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitAction {
    /// The file did not exist and was written with every default.
    Created,
    /// Missing settings were added, or a reference was written inline.
    Updated,
    /// Nothing needed to change; the file was not written.
    Unchanged,
}

impl InitAction {
    /// `created`, `updated`, or `unchanged`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Unchanged => "unchanged",
        }
    }
}

/// One configuration file setup looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitFile {
    #[allow(missing_docs)]
    pub path: PathBuf,
    #[allow(missing_docs)]
    pub scope: ConfigScope,
    #[allow(missing_docs)]
    pub action: InitAction,
}

/// Something the user should know about a file setup wrote, such as defaults
/// standing in for a block that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitNotice {
    /// The file the notice concerns.
    pub file: PathBuf,
    #[allow(missing_docs)]
    pub message: String,
}

/// What [`init`] did, scope by scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitReport {
    /// One entry per scope, in the order asked for.
    pub files: Vec<InitFile>,
    #[allow(missing_docs)]
    pub notices: Vec<InitNotice>,
}

/// Why setup wrote nothing.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InitError {
    /// The context has no home directory, so there is no user file.
    #[error("there is no home directory, so `~/{CONFIG_FILE_NAME}` cannot be created; nothing was written")]
    NoHome,
    /// The repository scope was asked for outside a repository.
    #[error("`{}` is not inside a repository, so there is no repository configuration to create; nothing was written", .dir.display())]
    NotARepository {
        /// The directory the context resolves from.
        dir: PathBuf,
    },
    /// An existing file is invalid. Setup repairs nothing but a reference
    /// (with [`InitOptions::inline`]); the file is left for the user to fix.
    #[error("{0}")]
    Invalid(ConfigError),
    /// Reading a file failed.
    #[error("cannot read {}: {source}", .path.display())]
    Read {
        #[allow(missing_docs)]
        path: PathBuf,
        #[allow(missing_docs)]
        #[source]
        source: std::io::Error,
    },
    /// The locked write refused or failed; see [`ConfigWriteError`] for what
    /// was published.
    #[error("{0}; nothing was saved")]
    Write(#[source] ConfigWriteError),
}

impl InitError {
    /// The diagnostic code: the configuration's own for an invalid file,
    /// `dm.config.not_saved` for everything that stopped a write.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Invalid(error) => error.code(),
            _ => CONFIG_NOT_SAVED_CODE,
        }
    }
}

/// A file's `permissions` setting as setup sees it, before anything is
/// written.
#[derive(Debug, Clone)]
pub enum BlockState {
    /// The file does not exist.
    MissingFile,
    /// The file exists and does not write `permissions`.
    Omitted,
    /// A valid inline block, as authored.
    Inline(Value),
    /// A syntactically valid reference.
    Reference {
        /// The reference as written.
        reference: String,
        /// The file it names on disk, when it could be resolved.
        target: Option<PathBuf>,
        /// The block it names, as authored there, or why it cannot be used.
        block: Result<Value, ConfigError>,
    },
    /// An invalid block or reference.
    Invalid(ConfigError),
}

/// One configuration file read for setup.
#[derive(Debug, Clone)]
pub struct ConfigObservation {
    /// The file's path.
    pub path: PathBuf,
    #[allow(missing_docs)]
    pub scope: ConfigScope,
    /// Its bytes; `None` when it does not exist.
    pub bytes: Option<Vec<u8>>,
    /// The document as written; `None` when it does not exist.
    pub authored: Option<Map<String, Value>>,
    #[allow(missing_docs)]
    pub permissions: BlockState,
}

/// The configuration file of `scope` that `context` names.
///
/// ## Errors
///
/// [`InitError::NoHome`] or [`InitError::NotARepository`].
pub fn config_path(context: &FileResolutionContext, scope: ConfigScope) -> Result<PathBuf, InitError> {
    match scope {
        ConfigScope::User => context.home_dir().map(|home| home.join(CONFIG_FILE_NAME)).ok_or(InitError::NoHome),
        ConfigScope::Repository => context
            .repository_root()
            .map(|root| root.join(CONFIG_FILE_NAME))
            .ok_or_else(|| InitError::NotARepository { dir: context.cwd().to_path_buf() }),
    }
}

/// The `user` scope alone; see [`init`].
///
/// ## Errors
///
/// As [`init`].
pub fn init_user(context: &FileResolutionContext, options: InitOptions) -> Result<InitReport, InitError> {
    init(context, &[ConfigScope::User], options)
}

/// The repository scope alone; see [`init`].
///
/// ## Errors
///
/// As [`init`].
pub fn init_repository(context: &FileResolutionContext, options: InitOptions) -> Result<InitReport, InitError> {
    init(context, &[ConfigScope::Repository], options)
}

/// Creates or completes the configuration file of every scope in `scopes`,
/// in one locked transaction.
///
/// ## Errors
///
/// - [`InitError::NoHome`] or [`InitError::NotARepository`] for a scope the
///   context cannot name, before anything is read or written;
/// - [`InitError::Invalid`] when an existing file is invalid (other than a
///   reference [`InitOptions::inline`] repairs);
/// - [`InitError::Read`] or [`InitError::Write`] when a file cannot be read
///   or saved.
///
/// On every error no file was changed.
pub fn init(context: &FileResolutionContext, scopes: &[ConfigScope], options: InitOptions) -> Result<InitReport, InitError> {
    for scope in scopes {
        config_path(context, *scope)?;
    }
    let mut report = InitReport::default();
    let mut transaction = ConfigTransaction::new();
    for scope in scopes {
        let observation = observe(context, *scope)?;
        let (permissions, notices) = match &observation.permissions {
            BlockState::Invalid(error) => return Err(InitError::Invalid(error.clone())),
            BlockState::Reference { block, reference, .. } if options.inline => match block {
                Ok(block) => (Some(block.clone()), Vec::new()),
                Err(error) => (
                    Some(default_permissions(*scope)),
                    vec![InitNotice {
                        file: observation.path.clone(),
                        message: format!(
                            "the `permissions` reference `{reference}` could not be read ({error}), so it was replaced with the schema defaults; this may change your permissions"
                        ),
                    }],
                ),
            },
            _ => (None, Vec::new()),
        };
        let change = setup_change(&observation, permissions)?;
        let action = match (&change, observation.bytes.is_some()) {
            (None, _) => InitAction::Unchanged,
            (Some(_), false) => InitAction::Created,
            (Some(_), true) => InitAction::Updated,
        };
        if let Some(change) = change {
            transaction.add(change).map_err(InitError::Write)?;
        }
        report.files.push(InitFile { path: observation.path, scope: *scope, action });
        report.notices.extend(notices);
    }
    commit_checked(transaction, context, &|_| None)?;
    Ok(report)
}

/// Reads the configuration file of `scope` that `context` names, and, for a
/// `permissions` reference, the file and block it names on disk.
///
/// ## Errors
///
/// [`InitError::NoHome`], [`InitError::NotARepository`], [`InitError::Read`],
/// or [`InitError::Invalid`] for a file that fails as a whole (not YAML, not
/// a mapping, an unknown key).
pub fn observe(context: &FileResolutionContext, scope: ConfigScope) -> Result<ConfigObservation, InitError> {
    let path = config_path(context, scope)?;
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => return Err(InitError::Read { path, source }),
    };
    let revision = match scope {
        ConfigScope::User => ConfigRevision::User,
        ConfigScope::Repository => ConfigRevision::RepositoryWorktree,
    };
    let file = observe_file(context, bytes.as_deref(), &path, revision, scope).map_err(InitError::Invalid)?;
    let authored = match file.authored() {
        Some(Value::Object(map)) => Some(map.clone()),
        _ => None,
    };
    let written = authored.as_ref().and_then(|document| document.get("permissions"));
    let permissions = match (&bytes, written, file.permissions()) {
        (None, _, _) => BlockState::MissingFile,
        (Some(_), None, _) => BlockState::Omitted,
        (Some(_), Some(Value::String(reference)), result) => {
            let followed = follow_on_disk(context, &file, scope);
            match (result, followed) {
                (Ok(_), Some(followed)) => {
                    let (target, block) = match followed {
                        Ok((target, Some(bytes))) => {
                            let block = referenced_block(context, &target, &bytes, scope)
                                .map_err(|error| reference_error(&path, revision, reference, &target, &error));
                            (Some(target), block)
                        }
                        Ok((target, None)) => {
                            let error = reference_error(&path, revision, reference, &target, "which does not exist");
                            (Some(target), Err(error))
                        }
                        Err(error) => (None, Err(error)),
                    };
                    BlockState::Reference { reference: reference.clone(), target, block }
                }
                // A reference the loader refused (remote, malformed, leaving the
                // repository): nothing to follow, but `--inline` can replace it.
                (Err(error), _) => {
                    BlockState::Reference { reference: reference.clone(), target: None, block: Err(error.clone()) }
                }
                (Ok(_), None) => BlockState::Omitted,
            }
        }
        (Some(_), Some(block), Ok(_)) => BlockState::Inline(block.clone()),
        (Some(_), Some(_), Err(error)) => BlockState::Invalid(error.clone()),
    };
    Ok(ConfigObservation { path, scope, bytes, authored, permissions })
}

/// The change that gives `observation`'s file every top-level setting:
/// missing ones at their schema default, and `permissions` set to
/// `permissions` when given (replacing what is there). `None` when nothing
/// would change. A missing file is created; an existing one is edited in
/// place and must still hold the bytes observed.
///
/// ## Errors
///
/// [`InitError::Invalid`] when a property path cannot be spelled (never for
/// the shipped schema).
pub fn setup_change(observation: &ConfigObservation, permissions: Option<Value>) -> Result<Option<TargetChange>, InitError> {
    let empty = Map::new();
    let authored = observation.authored.as_ref().unwrap_or(&empty);
    let mut edits = Vec::new();
    for (key, default) in top_level_defaults(observation.scope) {
        let value = match (&permissions, key.as_str()) {
            (Some(block), "permissions") if authored.get("permissions") != Some(block) => block.clone(),
            _ if authored.contains_key(&key) => continue,
            _ => default,
        };
        let path = PropertyPath::from_segments([key.clone()]).map_err(|error| {
            InitError::Invalid(ConfigError::unlocated(
                super::load::ConfigErrorKind::InvalidSetting,
                &observation.path,
                ConfigRevision::User,
                error.to_string(),
            ))
        })?;
        edits.push(YamlEdit::Set { path, value });
    }
    Ok(match &observation.bytes {
        None => Some(TargetChange::create(&observation.path, String::new(), edits)),
        Some(_) if edits.is_empty() => None,
        Some(bytes) => Some(TargetChange::edit(&observation.path, SourceExpectation::Bytes(bytes.clone()), edits)),
    })
}

/// Commits `transaction`, judging each candidate with `other` when it
/// returns a verdict for the target (a host's own file), and otherwise as a
/// Darkmatter configuration file with [`check_config_text`]: of the
/// repository scope when it is the repository file `context` names, of the
/// user scope otherwise.
///
/// ## Errors
///
/// [`InitError::Write`] with the transaction's error.
pub fn commit_checked(
    transaction: ConfigTransaction,
    context: &FileResolutionContext,
    other: &dyn Fn(&ValidationCandidate<'_>) -> Option<Result<(), ValidationRefusal>>,
) -> Result<CommitOutcome, InitError> {
    let repository = context.repository_root().map(|root| root.join(CONFIG_FILE_NAME));
    transaction
        .commit(&|candidate| {
            if let Some(verdict) = other(candidate) {
                return verdict;
            }
            let scope = match &repository {
                Some(path) if same_file(path, candidate.target) => ConfigScope::Repository,
                _ => ConfigScope::User,
            };
            check_config_text(candidate.candidate, candidate.target, scope, context)
        })
        .map_err(InitError::Write)
}

fn same_file(left: &Path, right: &Path) -> bool {
    let canonical = |path: &Path| {
        biscuit_file::canonicalize_simplified(path)
            .or_else(|_| {
                let parent = path.parent().ok_or_else(|| std::io::Error::other("no parent"))?;
                biscuit_file::canonicalize_simplified(parent).map(|parent| parent.join(path.file_name().unwrap_or_default()))
            })
            .unwrap_or_else(|_| path.to_path_buf())
    };
    biscuit_file::PathIdentity::new(&canonical(left)) == biscuit_file::PathIdentity::new(&canonical(right))
}

/// Judges `text` as the whole configuration file of `scope` at `path`: the
/// shipped schema, then, for an inline block, every entry with the loader's
/// own checks. The validation every setup write runs before it replaces a
/// file.
///
/// ## Errors
///
/// The first problem, as a [`ValidationRefusal`].
pub fn check_config_text(
    text: &str,
    path: &Path,
    scope: ConfigScope,
    context: &FileResolutionContext,
) -> Result<(), ValidationRefusal> {
    let loaded = load_yaml_document(text, path).map_err(|error| ValidationRefusal::new(error.to_string()))?;
    if let Err(failure) = contract(scope).validate_strict(&loaded.json) {
        return Err(ValidationRefusal::new(format!("{failure:?}")));
    }
    if loaded.json.get("permissions").is_some_and(Value::is_object) {
        check_permissions_text(text, path, scope, true, context).map_err(ValidationRefusal::new)?;
    }
    Ok(())
}

/// `scope`'s default `permissions` block, from the shipped schema: every
/// child at its default.
///
/// ## Panics
///
/// When the shipped schema declares no such default, which
/// `config::schema::tests` rules out.
#[must_use]
pub fn default_permissions(scope: ConfigScope) -> Value {
    top_level_defaults(scope)
        .into_iter()
        .find_map(|(key, value)| (key == "permissions").then_some(value))
        .expect("the shipped schema declares a `permissions` default")
}

/// `block` with every child it omits at `scope`'s default: the policy it
/// means, for comparing two blocks. `None` when `block` is not a valid
/// block of `scope`.
#[must_use]
pub fn effective_permissions(scope: ConfigScope, block: &Value) -> Option<Value> {
    let success = permissions_contract(scope).validate_strict(block).ok()?;
    let mut effective = block.clone();
    for default in &success.defaults {
        if let ([key], Value::Object(map)) = (default.path.segments(), &mut effective) {
            map.insert(key.clone(), default.value.clone());
        }
    }
    Some(effective)
}

/// What [`register_manifest`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterReport {
    /// The configuration file that lists the manifest.
    pub path: PathBuf,
    /// [`InitAction::Unchanged`] when an entry already named the manifest.
    pub action: InitAction,
}

/// Adds `reference` (a manifest reference, as it should be written) to the
/// `register` list of `scope`'s configuration file, unless an entry already
/// names the same manifest: entries are compared by the manifest file they
/// resolve to ([`manifest_identity`](super::registration::manifest_identity)),
/// not by spelling. Every existing entry, value, and comment is kept; a
/// missing file is created with every default and the one entry. The host
/// program publishes the manifest first, so `reference` must name an
/// existing file.
///
/// ## Errors
///
/// - [`InitError::NoHome`] or [`InitError::NotARepository`];
/// - [`InitError::Invalid`] when the file is invalid, its `register` value
///   is not a list, or `reference` names no existing local manifest;
/// - [`InitError::Read`] or [`InitError::Write`]. Nothing was changed.
pub fn register_manifest(
    context: &FileResolutionContext,
    scope: ConfigScope,
    reference: &str,
) -> Result<RegisterReport, InitError> {
    let observation = observe(context, scope)?;
    if let BlockState::Invalid(error) = &observation.permissions {
        return Err(InitError::Invalid(error.clone()));
    }
    let revision = match scope {
        ConfigScope::User => ConfigRevision::User,
        ConfigScope::Repository => ConfigRevision::RepositoryWorktree,
    };
    let invalid = |message: String| {
        InitError::Invalid(ConfigError::unlocated(
            super::load::ConfigErrorKind::InvalidSetting,
            &observation.path,
            revision,
            message,
        ))
    };
    let wanted = super::registration::manifest_identity(reference, &observation.path, context)
        .map_err(|why| invalid(format!("cannot register `{reference}`: {why}")))?;
    let register_path = |index: Option<usize>| {
        let mut segments = vec![String::from("register")];
        segments.extend(index.map(|index| index.to_string()));
        PropertyPath::from_segments(segments).map_err(|error| invalid(error.to_string()))
    };

    let change = match (&observation.bytes, &observation.authored) {
        (Some(bytes), Some(authored)) => {
            let edit = match authored.get("register") {
                None | Some(Value::Null) => {
                    YamlEdit::Set { path: register_path(None)?, value: Value::Array(vec![Value::from(reference)]) }
                }
                Some(Value::Array(entries)) => {
                    let registered = entries.iter().filter_map(Value::as_str).any(|entry| {
                        super::registration::manifest_identity(entry, &observation.path, context)
                            .is_ok_and(|identity| identity == wanted)
                    });
                    if registered {
                        return Ok(RegisterReport { path: observation.path, action: InitAction::Unchanged });
                    }
                    YamlEdit::Set { path: register_path(Some(entries.len()))?, value: Value::from(reference) }
                }
                Some(_) => return Err(invalid(String::from("`register` is not a list, so no host can be registered"))),
            };
            TargetChange::edit(&observation.path, SourceExpectation::Bytes(bytes.clone()), vec![edit])
        }
        _ => {
            let mut edits = Vec::new();
            for (key, default) in top_level_defaults(scope) {
                let value = if key == "register" { Value::Array(vec![Value::from(reference)]) } else { default };
                let path = PropertyPath::from_segments([key]).map_err(|error| invalid(error.to_string()))?;
                edits.push(YamlEdit::Set { path, value });
            }
            TargetChange::create(&observation.path, String::new(), edits)
        }
    };
    let action = if observation.bytes.is_some() { InitAction::Updated } else { InitAction::Created };
    let mut transaction = ConfigTransaction::new();
    transaction.add(change).map_err(InitError::Write)?;
    commit_checked(transaction, context, &|_| None)?;
    Ok(RegisterReport { path: observation.path, action })
}

/// Every top-level setting of `scope` with its schema default, in schema
/// order.
fn top_level_defaults(scope: ConfigScope) -> Vec<(String, Value)> {
    let empty = Value::Object(Map::new());
    let success = contract(scope)
        .validate_strict(&empty)
        .unwrap_or_else(|failure| panic!("the shipped schema accepts an empty configuration: {failure:?}"));
    success
        .defaults
        .into_iter()
        .filter_map(|default| match default.path.segments() {
            [key] => Some((key.clone(), default.value)),
            _ => None,
        })
        .collect()
}

/// The `permissions` mapping of a reference's target, as authored, once it
/// passes the same checks the policy applies.
fn referenced_block(context: &FileResolutionContext, target: &Path, bytes: &[u8], scope: ConfigScope) -> Result<Value, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "which is not valid UTF-8".to_string())?;
    check_permissions_text(text, target, scope, false, context)?;
    let loaded = load_yaml_document(text, target).map_err(|error| error.to_string())?;
    loaded.json.get("permissions").cloned().ok_or_else(|| "which has no `permissions` mapping".to_string())
}

fn reference_error(declared_in: &Path, revision: ConfigRevision, reference: &str, target: &Path, what: &str) -> ConfigError {
    ConfigError::unlocated(
        super::load::ConfigErrorKind::InvalidReference,
        declared_in,
        revision,
        format!("the `permissions` reference `{reference}` names `{}`, {what}", target.display()),
    )
}
