//! Claudine's half of the setup handshake with Darkmatter's configuration.
//!
//! When Claudine is set up it owns the `permissions` block, and Darkmatter's
//! `.darkmatter.yaml` of the same scope points at Claudine's file
//! (`permissions: ~/.claudine/config.yaml`). [`setup_with_darkmatter`] runs
//! Claudine's own configuration write first, then:
//!
//! | `.darkmatter.yaml` | Result |
//! | --- | --- |
//! | absent, or without `permissions` | created or completed with the reference; Claudine's file gets the default block when it has none |
//! | an inline block | the block moves into Claudine's file, then `.darkmatter.yaml` is rewritten to the reference |
//! | an inline block that differs from a block Claudine already had | nothing moves: [`SetupError::Conflict`] names both files |
//! | a reference to Claudine's file | left as it is |
//! | a reference to any other file | left as it is; Claudine does not take it over |
//!
//! The transfer is one locked [`ConfigTransaction`] over both files: locks in
//! path order, Claudine's file replaced first, `.darkmatter.yaml` last, so a
//! failure between the two leaves the original inline block in force.
//! Every other value and comment in both files is kept, and running setup
//! again changes nothing.
//!
//! [`setup_claudine`] is the whole of `claudine init`'s part: it publishes
//! Claudine's host revision first, then runs this handshake, then adds the
//! published manifest to the user's `register` list, so Darkmatter (md and
//! DMLS) recognizes Claudine documents outside `claudine/`.
//!
//! User-facing guide: `claudine/docs/topics/configuration.md` ("Darkmatter's
//! permissions").

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, PathIdentity, canonicalize_simplified};
use darkmatter::config::init::{
    self as dm_init, BlockState, InitAction, InitError, RegisterReport, default_permissions, effective_permissions,
};
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use darkmatter::config::write::{ConfigTransaction, SourceExpectation, TargetChange, ValidationRefusal};
use serde_json::Value;
use simplified_schema::yaml_documents::load_yaml_document;

use super::persist::{PropertyPath, SaveOutcome, YamlEdit, validate_config_text};
use super::schema::{CONFIG_DIR, CONFIG_FILE_NAME, ConfigScope};
use crate::composition::registration::{self, HostRevision, PublishError, PublishOutcome};
use crate::error::ClaudineError;

/// What happened to Darkmatter's `permissions`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PermissionsHandoff {
    /// `.darkmatter.yaml` had no block of its own and now refers to
    /// Claudine's file.
    Referenced,
    /// Its inline block moved into Claudine's file, and it now refers there.
    Transferred,
    /// It already referred to Claudine's file.
    AlreadyReferenced,
    /// It refers to another program's file, which Claudine leaves in place.
    ThirdParty {
        /// The reference as written.
        reference: String,
    },
}

/// What [`setup_with_darkmatter`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DarkmatterSetup {
    /// Claudine's configuration file.
    pub claudine_config: PathBuf,
    /// Claudine's own write.
    pub claudine_saved: SaveOutcome,
    /// Darkmatter's configuration file of the same scope.
    pub darkmatter_config: PathBuf,
    /// What happened to `.darkmatter.yaml`.
    pub darkmatter_action: InitAction,
    #[allow(missing_docs)]
    pub permissions: PermissionsHandoff,
}

/// Why setup stopped.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SetupError {
    /// There is no home directory for the user scope; nothing was written.
    #[error("there is no home directory, so neither `~/{CONFIG_DIR}/{CONFIG_FILE_NAME}` nor `~/.darkmatter.yaml` can be written; nothing was written")]
    NoHome,
    /// The repository scope was asked for outside a repository; nothing was
    /// written.
    #[error("`{}` is not inside a repository, so there is no repository configuration to set up; nothing was written", .dir.display())]
    NotARepository {
        /// The directory the context resolves from.
        dir: PathBuf,
    },
    /// Both files hold different `permissions` blocks. Claudine's own
    /// settings were saved; neither `permissions` was changed.
    #[error(
        "`{}` and `{}` both hold a `permissions` block and they differ; neither was changed. Keep one: delete the other block, or make them equal, then run setup again",
        .darkmatter.display(),
        .claudine.display()
    )]
    Conflict {
        /// `.darkmatter.yaml`, with its inline block.
        darkmatter: PathBuf,
        /// Claudine's file, with its own block.
        claudine: PathBuf,
    },
    /// Darkmatter's file is invalid, or the handshake's write failed.
    #[error(transparent)]
    Darkmatter(InitError),
    /// Claudine's own write failed.
    #[error(transparent)]
    Claudine(ClaudineError),
    /// Claudine's host registration could not be published; nothing else
    /// was written.
    #[error(transparent)]
    Publish(PublishError),
}

/// What [`setup_claudine`] did, step by step.
#[derive(Debug)]
pub struct ClaudineSetup {
    /// The published (or already current) host revision.
    pub publication: PublishOutcome,
    /// Claudine's configuration write and the `permissions` handshake. Only
    /// a failure of Claudine's own write stops setup; every other failure
    /// is reported here and leaves Darkmatter's file as it was.
    pub darkmatter: Result<DarkmatterSetup, SetupError>,
    /// The `register` entry for Claudine's manifest in `~/.darkmatter.yaml`.
    pub registration: Result<RegisterReport, InitError>,
}

/// The whole of Claudine's setup with Darkmatter, in order: publish
/// Claudine's host revision ([`registration::publish`]), write Claudine's
/// configuration of `scope` and complete the `permissions` handshake
/// ([`setup_with_darkmatter`]), then register the published manifest in
/// the user's `~/.darkmatter.yaml` (an entry naming the same manifest, in
/// any spelling, is kept as the one entry). Registration is always the
/// user's: the manifest lives under the home, whichever scope was set up.
///
/// ## Errors
///
/// - [`SetupError::NoHome`] or [`SetupError::NotARepository`] before
///   anything is written;
/// - [`SetupError::Publish`] when the revision cannot be published; nothing
///   else is written;
/// - [`SetupError::Claudine`] when Claudine's own write fails.
pub fn setup_claudine(
    context: &FileResolutionContext,
    scope: ConfigScope,
    write_claudine: impl FnOnce(&Path) -> crate::error::Result<SaveOutcome>,
) -> Result<ClaudineSetup, SetupError> {
    let home = context.home_dir().ok_or(SetupError::NoHome)?.to_path_buf();
    claudine_config_path(context, scope)?;
    let publication = registration::publish(&home, HostRevision::claudine()).map_err(SetupError::Publish)?;
    let darkmatter = match setup_with_darkmatter(context, scope, write_claudine) {
        Err(SetupError::Claudine(error)) => return Err(SetupError::Claudine(error)),
        other => other,
    };
    let registration = dm_init::register_manifest(context, DarkmatterScope::User, registration::MANIFEST_REFERENCE);
    Ok(ClaudineSetup { publication, darkmatter, registration })
}

/// Claudine's configuration file of `scope` that `context` names.
///
/// ## Errors
///
/// [`SetupError::NoHome`] or [`SetupError::NotARepository`].
pub fn claudine_config_path(context: &FileResolutionContext, scope: ConfigScope) -> Result<PathBuf, SetupError> {
    let base = match scope {
        ConfigScope::User => context.home_dir().ok_or(SetupError::NoHome)?,
        ConfigScope::Repository => context
            .repository_root()
            .ok_or_else(|| SetupError::NotARepository { dir: context.cwd().to_path_buf() })?,
    };
    Ok(base.join(CONFIG_DIR).join(CONFIG_FILE_NAME))
}

/// Runs `write_claudine` (Claudine's own write of its configuration file of
/// `scope`, given that file's path), then completes the handshake described
/// in the module documentation.
///
/// ## Errors
///
/// - [`SetupError::NoHome`] or [`SetupError::NotARepository`] before
///   anything is written, `write_claudine` included;
/// - [`SetupError::Claudine`] from `write_claudine`;
/// - [`SetupError::Conflict`] when both files hold different blocks;
/// - [`SetupError::Darkmatter`] when `.darkmatter.yaml` is invalid or the
///   handshake's write fails; a failure leaves the original inline block in
///   force.
pub fn setup_with_darkmatter(
    context: &FileResolutionContext,
    scope: ConfigScope,
    write_claudine: impl FnOnce(&Path) -> crate::error::Result<SaveOutcome>,
) -> Result<DarkmatterSetup, SetupError> {
    let darkmatter_scope = match scope {
        ConfigScope::User => DarkmatterScope::User,
        ConfigScope::Repository => DarkmatterScope::Repository,
    };
    let claudine_config = claudine_config_path(context, scope)?;
    let darkmatter_config = dm_init::config_path(context, darkmatter_scope).map_err(SetupError::Darkmatter)?;

    let block_before = read_document(&claudine_config).and_then(|(_, document)| document.get("permissions").cloned());
    let claudine_saved = write_claudine(&claudine_config).map_err(SetupError::Claudine)?;

    let observation = dm_init::observe(context, darkmatter_scope).map_err(SetupError::Darkmatter)?;
    let (claudine_bytes, claudine_document) = read_document(&claudine_config)
        .ok_or_else(|| SetupError::Claudine(ClaudineError::ConfigNotFound(claudine_config.clone())))?;
    let block_now = claudine_document.get("permissions").cloned();
    let reference = match scope {
        ConfigScope::User => format!("~/{CONFIG_DIR}/{CONFIG_FILE_NAME}"),
        ConfigScope::Repository => format!("./{CONFIG_DIR}/{CONFIG_FILE_NAME}"),
    };

    // What Claudine's block becomes (`None`: unchanged), what Darkmatter's
    // `permissions` becomes (`None`: unchanged), and how to report it.
    let (claudine_block, darkmatter_permissions, permissions) = match &observation.permissions {
        BlockState::Invalid(error) => return Err(SetupError::Darkmatter(InitError::Invalid(error.clone()))),
        BlockState::Reference { reference: written, target, .. } => {
            let ours = target.as_deref().is_some_and(|target| same_file(target, &claudine_config));
            if ours {
                let block = block_now.is_none().then(|| default_permissions(darkmatter_scope));
                (block, None, PermissionsHandoff::AlreadyReferenced)
            } else {
                (None, None, PermissionsHandoff::ThirdParty { reference: written.clone() })
            }
        }
        BlockState::Inline(block) => {
            if let Some(before) = &block_before
                && effective_permissions(darkmatter_scope, before) != effective_permissions(darkmatter_scope, block)
            {
                return Err(SetupError::Conflict { darkmatter: observation.path.clone(), claudine: claudine_config });
            }
            let moved = (block_now.as_ref() != Some(block)).then(|| block.clone());
            (moved, Some(Value::String(reference)), PermissionsHandoff::Transferred)
        }
        BlockState::MissingFile | BlockState::Omitted => {
            let block = block_now.is_none().then(|| default_permissions(darkmatter_scope));
            (block, Some(Value::String(reference)), PermissionsHandoff::Referenced)
        }
    };

    let mut transaction = ConfigTransaction::new();
    if let Some(block) = claudine_block {
        let path = PropertyPath::from_segments(["permissions"]).expect("`permissions` is a property path");
        let change = TargetChange::edit(
            &claudine_config,
            SourceExpectation::Bytes(claudine_bytes),
            vec![YamlEdit::Set { path, value: block }],
        );
        transaction.add(change).map_err(|error| SetupError::Darkmatter(InitError::Write(error)))?;
    }
    let darkmatter_change = dm_init::setup_change(&observation, darkmatter_permissions).map_err(SetupError::Darkmatter)?;
    let darkmatter_action = match (&darkmatter_change, observation.bytes.is_some()) {
        (None, _) => InitAction::Unchanged,
        (Some(_), false) => InitAction::Created,
        (Some(_), true) => InitAction::Updated,
    };
    if let Some(change) = darkmatter_change {
        // The reference is published only once Claudine's file holds the block.
        transaction.add(change.published_last()).map_err(|error| SetupError::Darkmatter(InitError::Write(error)))?;
    }
    let claudine_identity = identity(&claudine_config);
    // A refusal of Claudine's candidate is reported as Claudine's own typed
    // error; the transaction's refusal only carries text.
    let refused: RefCell<Option<ClaudineError>> = RefCell::new(None);
    let committed = dm_init::commit_checked(transaction, context, &|candidate| {
        (identity(candidate.target) == claudine_identity).then(|| {
            validate_config_text(candidate.candidate, candidate.target, scope).map_err(|error| {
                let refusal = ValidationRefusal::new(format!("{} is invalid", candidate.target.display()));
                *refused.borrow_mut() = Some(error);
                refusal
            })
        })
    });
    if let Err(error) = committed {
        return Err(match refused.into_inner() {
            Some(claudine) => SetupError::Claudine(claudine),
            None => SetupError::Darkmatter(error),
        });
    }

    Ok(DarkmatterSetup { claudine_config, claudine_saved, darkmatter_config, darkmatter_action, permissions })
}

/// The file's bytes and its document, when it exists and is a YAML mapping.
fn read_document(path: &Path) -> Option<(Vec<u8>, serde_json::Map<String, Value>)> {
    let bytes = std::fs::read(path).ok()?;
    let text = std::str::from_utf8(&bytes).ok()?;
    match load_yaml_document(text, path).ok()?.json {
        Value::Object(document) => Some((bytes, document)),
        _ => None,
    }
}

fn identity(path: &Path) -> PathIdentity {
    PathIdentity::new(&canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn same_file(left: &Path, right: &Path) -> bool {
    identity(left) == identity(right)
}
