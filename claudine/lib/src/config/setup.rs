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
//! `claudine doctor --fix` makes the same handoff through the same planner
//! and committer (`plan_permissions_handoff`, `commit_permissions_handoff`)
//! in doctor mode: existing files only, `permissions` edits only.
//!
//! [`setup_claudine`] is the whole of Claudine's setup (the first-run wizard
//! and `claudine setup`): holding the publisher lock throughout, it publishes
//! Claudine's host registration first, then runs this handshake, then puts
//! the published manifest in the user's `register` list in place of the
//! retired `~/.claudine/darkmatter-host.yaml`, so Darkmatter (md and DMLS)
//! recognizes Claudine documents outside `claudine/`, and last removes the
//! retired manifest and its revision tree.
//!
//! User-facing guide: `claudine/docs/topics/configuration.md` ("Darkmatter's
//! permissions").

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, PathIdentity, canonicalize_simplified};
use darkmatter::config::init::{
    self as dm_init, BlockState, ConfigObservation, InitAction, InitError, RegisterReport, default_permissions,
    effective_permissions,
};
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use darkmatter::config::write::{CommitOutcome, ConfigTransaction, SourceExpectation, TargetChange, ValidationRefusal};
use serde_json::Value;
use simplified_schema::yaml_documents::load_yaml_document;

use super::persist::{PropertyPath, SaveOutcome, YamlEdit, validate_config_text};
use super::schema::{CONFIG_DIR, CONFIG_FILE_NAME, ConfigScope};
use crate::composition::registration::{
    self, CleanupError, LegacyCleanup, PublishError, PublishOutcome, Publisher, RegistrationInventory,
};
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
    /// The published (or already current) host registration.
    pub publication: PublishOutcome,
    /// Claudine's configuration write and the `permissions` handshake. Only
    /// a failure of Claudine's own write stops setup; every other failure
    /// is reported here and leaves Darkmatter's file as it was.
    pub darkmatter: Result<DarkmatterSetup, SetupError>,
    /// The `register` entry for Claudine's manifest in `~/.darkmatter.yaml`;
    /// [`RegisterReport::replaced`] lists the retired entries it replaced.
    pub registration: Result<RegisterReport, InitError>,
    /// The removal of the retired registration
    /// ([`registration::remove_legacy_registration`]); `None` when the
    /// `register` entry could not be written, so nothing was removed.
    pub cleanup: Option<Result<LegacyCleanup, CleanupError>>,
}

/// The whole of Claudine's setup with Darkmatter, in order, all under the
/// publisher lock: publish Claudine's host registration (as
/// [`registration::publish`] does), write Claudine's
/// configuration of `scope` and complete the `permissions` handshake
/// ([`setup_with_darkmatter`]), then register the published manifest in
/// the user's `~/.darkmatter.yaml`
/// ([`replace_manifest_registration`](dm_init::replace_manifest_registration):
/// entries naming the retired manifest or the new one, in any spelling,
/// become the one entry), and once that succeeds remove the retired
/// registration under `~/.claudine`. A handshake that failed (a
/// `permissions` conflict, say) still registers the manifest. Registration
/// is always the user's: the manifest lives under the home, whichever scope
/// was set up, and a repository's `register` list is never changed.
///
/// ## Errors
///
/// - [`SetupError::NoHome`] or [`SetupError::NotARepository`] before
///   anything is written;
/// - [`SetupError::Publish`] when the registration cannot be published; nothing
///   else is written;
/// - [`SetupError::Claudine`] when Claudine's own write fails.
pub fn setup_claudine(
    context: &FileResolutionContext,
    scope: ConfigScope,
    write_claudine: impl FnOnce(&Path) -> crate::error::Result<SaveOutcome>,
) -> Result<ClaudineSetup, SetupError> {
    let home = context.home_dir().ok_or(SetupError::NoHome)?.to_path_buf();
    claudine_config_path(context, scope)?;
    let inventory = RegistrationInventory::claudine();
    // The publisher lock is held until the retired layout is gone, so no
    // other publisher (doctor's repair, `claudine registrations publish`)
    // runs between the publish and the cleanup. Configuration locks are
    // taken after it, one transaction at a time.
    let publisher = Publisher::acquire(&home, inventory).map_err(SetupError::Publish)?;
    let publication = publisher.publish(inventory).map_err(SetupError::Publish)?;
    let darkmatter = match setup_with_darkmatter(context, scope, write_claudine) {
        Err(SetupError::Claudine(error)) => return Err(SetupError::Claudine(error)),
        other => other,
    };
    let registration = dm_init::replace_manifest_registration(
        context,
        DarkmatterScope::User,
        registration::MANIFEST_REFERENCE,
        &registration::legacy_manifest_path(&home),
    );
    let cleanup = registration.is_ok().then(|| publisher.finalize_legacy());
    drop(publisher);
    Ok(ClaudineSetup { publication, darkmatter, registration, cleanup })
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
    let darkmatter_scope = darkmatter_scope_of(scope);
    let claudine_config = claudine_config_path(context, scope)?;
    let darkmatter_config = dm_init::config_path(context, darkmatter_scope).map_err(SetupError::Darkmatter)?;

    let block_before = read_document(&claudine_config).and_then(|(_, document)| document.get("permissions").cloned());
    let claudine_saved = write_claudine(&claudine_config).map_err(SetupError::Claudine)?;

    let observation = dm_init::observe(context, darkmatter_scope).map_err(SetupError::Darkmatter)?;
    let (claudine_bytes, claudine_document) = read_document(&claudine_config)
        .ok_or_else(|| SetupError::Claudine(ClaudineError::ConfigNotFound(claudine_config.clone())))?;
    let claudine = ClaudineSide { path: claudine_config, bytes: claudine_bytes, document: claudine_document, block_before };

    let plan = plan_permissions_handoff(context, scope, HandoffMode::Setup, claudine, &observation).map_err(|refusal| {
        match refusal {
            HandoffRefusal::Darkmatter(error) => SetupError::Darkmatter(*error),
            HandoffRefusal::Conflict { claudine, .. } => {
                SetupError::Conflict { darkmatter: observation.path.clone(), claudine }
            }
            HandoffRefusal::Manual(reason) => unreachable!("setup plans no manual refusal ({reason:?})"),
        }
    })?;
    let (claudine_config, darkmatter_action, permissions) =
        (plan.claudine_config.clone(), plan.darkmatter_action, plan.permissions.clone());
    commit_permissions_handoff(context, plan).map_err(|error| match error {
        HandoffCommitError::Claudine(error) => SetupError::Claudine(error),
        HandoffCommitError::Darkmatter(error) => SetupError::Darkmatter(*error),
        HandoffCommitError::UnusableTarget => unreachable!("setup does not judge Claudine's file as a reference target"),
    })?;

    Ok(DarkmatterSetup { claudine_config, claudine_saved, darkmatter_config, darkmatter_action, permissions })
}

/// Who a handoff is planned for. The two differ only where
/// [`plan_permissions_handoff`] says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandoffMode {
    /// Setup: may create `.darkmatter.yaml` and completes its top-level
    /// settings ([`dm_init::setup_change`]); any reference is left in place.
    Setup,
    /// `claudine doctor --fix`: both files exist and are valid, and only
    /// their `permissions` settings are edited.
    Doctor,
}

/// Claudine's file of the scope being handed off, as read before planning.
#[derive(Debug, Clone)]
pub(crate) struct ClaudineSide {
    pub(crate) path: PathBuf,
    /// The bytes the edit expects to find under the lock.
    pub(crate) bytes: Vec<u8>,
    /// The document as written.
    pub(crate) document: serde_json::Map<String, Value>,
    /// Setup only: Claudine's `permissions` before its own write, which an
    /// inline Darkmatter block must equal. Doctor reads [`Self::document`].
    pub(crate) block_before: Option<Value>,
}

/// The writes one scope's handoff needs, for [`commit_permissions_handoff`].
#[derive(Debug, Clone)]
pub(crate) struct HandoffPlan {
    mode: HandoffMode,
    scope: ConfigScope,
    pub(crate) claudine_config: PathBuf,
    pub(crate) darkmatter_config: PathBuf,
    claudine_change: Option<TargetChange>,
    darkmatter_change: Option<TargetChange>,
    pub(crate) darkmatter_action: InitAction,
    pub(crate) permissions: PermissionsHandoff,
    /// Doctor only: the mapping Darkmatter's reference resolves to once the
    /// plan is committed.
    pub(crate) block: Option<Value>,
}

impl HandoffPlan {
    /// Claudine's scope the plan hands off.
    pub(crate) const fn scope(&self) -> ConfigScope {
        self.scope
    }

    /// Whether Claudine's file is edited.
    pub(crate) const fn edits_claudine(&self) -> bool {
        self.claudine_change.is_some()
    }

    /// Whether `.darkmatter.yaml` is edited (or created).
    pub(crate) const fn edits_darkmatter(&self) -> bool {
        self.darkmatter_change.is_some()
    }
}

/// Why no handoff can be planned.
#[derive(Debug)]
pub(crate) enum HandoffRefusal {
    /// `.darkmatter.yaml`'s `permissions` is invalid (or, never for the
    /// shipped schema, a property path cannot be spelled).
    Darkmatter(Box<InitError>),
    /// Both files author a `permissions` setting and they cannot both hold.
    Conflict {
        claudine: PathBuf,
        kind: ConflictKind,
    },
    /// Doctor only: the state needs the user's correction.
    Manual(ManualReason),
}

/// How the two `permissions` settings conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConflictKind {
    /// Two inline blocks that differ once defaults are filled in.
    DifferentBlocks,
    /// Claudine's own `permissions` is a reference: a reference to
    /// Claudine's file would need a second hop, which Darkmatter refuses.
    ClaudineReference,
}

/// Doctor's states that no automatic handoff applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManualReason {
    /// `.darkmatter.yaml` does not exist.
    DarkmatterMissing,
    /// Its reference does not yield a usable mapping; `ours` when it names
    /// Claudine's file.
    BrokenReference { ours: bool },
    /// Claudine's `permissions` is neither a mapping, a reference, nor
    /// absent.
    ClaudineUnusable,
    /// Claudine's file breaks a configuration rule, so it cannot be edited.
    ClaudineInvalid,
    /// The mapping Claudine would supply fails Darkmatter's rules for a
    /// reference target of the scope.
    UnusableMapping,
}

/// Decides one scope's `permissions` handoff from `observation` (Darkmatter's
/// file) and `claudine`, writing nothing.
///
/// | `.darkmatter.yaml` | Setup | Doctor |
/// | --- | --- | --- |
/// | missing | created with the reference | [`ManualReason::DarkmatterMissing`] |
/// | omitted (doctor: or `null`) | reference; Claudine gets the default block when it has none | the same, reusing a Claudine mapping; a Claudine reference is a conflict |
/// | inline block | moved unless it differs from Claudine's earlier block | moved unless Claudine already holds an effectively equal one; a different block or a Claudine reference is a conflict |
/// | reference to Claudine's file | left; Claudine gets the default block when it has none | passes when it resolves to a mapping, else [`ManualReason::BrokenReference`] |
/// | other reference | left | passes (third party) when it resolves, else broken |
///
/// Doctor also requires the mapping Claudine ends up holding to pass
/// Darkmatter's scoped checks for a reference target
/// ([`dm_init::check_referenced_permissions`]), and Claudine's file to pass
/// its own validation when it is edited.
pub(crate) fn plan_permissions_handoff(
    context: &FileResolutionContext,
    scope: ConfigScope,
    mode: HandoffMode,
    claudine: ClaudineSide,
    observation: &ConfigObservation,
) -> Result<HandoffPlan, HandoffRefusal> {
    let decision = match mode {
        HandoffMode::Setup => decide_for_setup(scope, &claudine, observation)?,
        HandoffMode::Doctor => decide_for_doctor(context, scope, &claudine, observation)?,
    };
    let reference = Value::String(darkmatter_reference(scope));
    let property = PropertyPath::from_segments(["permissions"]).expect("`permissions` is a property path");
    let claudine_change = decision.claudine_block.map(|block| {
        TargetChange::edit(
            &claudine.path,
            SourceExpectation::Bytes(claudine.bytes.clone()),
            vec![YamlEdit::Set { path: property.clone(), value: block }],
        )
    });
    let darkmatter_change = match mode {
        HandoffMode::Setup => dm_init::setup_change(observation, decision.refer.then_some(reference))
            .map_err(|error| HandoffRefusal::Darkmatter(Box::new(error)))?,
        HandoffMode::Doctor => match (&observation.bytes, decision.refer) {
            (Some(bytes), true) => Some(TargetChange::edit(
                &observation.path,
                SourceExpectation::Bytes(bytes.clone()),
                vec![YamlEdit::Set { path: property, value: reference }],
            )),
            _ => None,
        },
    };
    let darkmatter_action = match (&darkmatter_change, observation.bytes.is_some()) {
        (None, _) => InitAction::Unchanged,
        (Some(_), false) => InitAction::Created,
        (Some(_), true) => InitAction::Updated,
    };
    Ok(HandoffPlan {
        mode,
        scope,
        claudine_config: claudine.path,
        darkmatter_config: observation.path.clone(),
        claudine_change,
        // The reference is published only once Claudine's file holds the block.
        darkmatter_change: darkmatter_change.map(TargetChange::published_last),
        darkmatter_action,
        permissions: decision.permissions,
        block: decision.block,
    })
}

/// What a planner decided, before it is spelled as changes.
struct Decision {
    /// What Claudine's `permissions` becomes; `None`: unchanged.
    claudine_block: Option<Value>,
    /// Whether Darkmatter's `permissions` becomes the reference.
    refer: bool,
    permissions: PermissionsHandoff,
    /// The mapping the reference resolves to afterwards (doctor).
    block: Option<Value>,
}

fn decide_for_setup(
    scope: ConfigScope,
    claudine: &ClaudineSide,
    observation: &ConfigObservation,
) -> Result<Decision, HandoffRefusal> {
    let darkmatter_scope = darkmatter_scope_of(scope);
    let block_now = claudine.document.get("permissions");
    let decision = |claudine_block, refer, permissions| Decision { claudine_block, refer, permissions, block: None };
    Ok(match &observation.permissions {
        BlockState::Invalid(error) => {
            return Err(HandoffRefusal::Darkmatter(Box::new(InitError::Invalid(error.clone()))));
        }
        BlockState::Reference { reference: written, target, .. } => {
            if target.as_deref().is_some_and(|target| same_file(target, &claudine.path)) {
                let block = block_now.is_none().then(|| default_permissions(darkmatter_scope));
                decision(block, false, PermissionsHandoff::AlreadyReferenced)
            } else {
                decision(None, false, PermissionsHandoff::ThirdParty { reference: written.clone() })
            }
        }
        BlockState::Inline(block) => {
            if let Some(before) = &claudine.block_before
                && effective_permissions(darkmatter_scope, before) != effective_permissions(darkmatter_scope, block)
            {
                return Err(HandoffRefusal::Conflict {
                    claudine: claudine.path.clone(),
                    kind: ConflictKind::DifferentBlocks,
                });
            }
            let moved = (block_now != Some(block)).then(|| block.clone());
            decision(moved, true, PermissionsHandoff::Transferred)
        }
        BlockState::MissingFile | BlockState::Omitted => {
            let block = block_now.is_none().then(|| default_permissions(darkmatter_scope));
            decision(block, true, PermissionsHandoff::Referenced)
        }
    })
}

fn decide_for_doctor(
    context: &FileResolutionContext,
    scope: ConfigScope,
    claudine: &ClaudineSide,
    observation: &ConfigObservation,
) -> Result<Decision, HandoffRefusal> {
    let darkmatter_scope = darkmatter_scope_of(scope);
    let manual = |reason| Err(HandoffRefusal::Manual(reason));
    let conflict = |kind| Err(HandoffRefusal::Conflict { claudine: claudine.path.clone(), kind });
    // Omitted and `null` are the same to Darkmatter's loader: no authored block.
    let authored = claudine.document.get("permissions").filter(|block| !block.is_null());

    let (block, moved, permissions) = match &observation.permissions {
        BlockState::MissingFile => return manual(ManualReason::DarkmatterMissing),
        BlockState::Invalid(error) => {
            return Err(HandoffRefusal::Darkmatter(Box::new(InitError::Invalid(error.clone()))));
        }
        BlockState::Reference { reference, target, block } => {
            let ours = target.as_deref().is_some_and(|target| same_file(target, &claudine.path));
            return match (block, ours) {
                (Err(_), _) => manual(ManualReason::BrokenReference { ours }),
                (Ok(block), true) => Ok(Decision {
                    claudine_block: None,
                    refer: false,
                    permissions: PermissionsHandoff::AlreadyReferenced,
                    block: Some(block.clone()),
                }),
                (Ok(_), false) => Ok(Decision {
                    claudine_block: None,
                    refer: false,
                    permissions: PermissionsHandoff::ThirdParty { reference: reference.clone() },
                    block: None,
                }),
            };
        }
        BlockState::Omitted | BlockState::Inline(Value::Null) => match authored {
            None => (default_permissions(darkmatter_scope), true, PermissionsHandoff::Referenced),
            Some(mine @ Value::Object(_)) => (mine.clone(), false, PermissionsHandoff::Referenced),
            Some(Value::String(_)) => return conflict(ConflictKind::ClaudineReference),
            Some(_) => return manual(ManualReason::ClaudineUnusable),
        },
        BlockState::Inline(theirs) => match authored {
            None => (theirs.clone(), true, PermissionsHandoff::Transferred),
            Some(mine @ Value::Object(_)) => {
                let mine_effective = effective_permissions(darkmatter_scope, mine);
                if mine_effective.is_none() || mine_effective != effective_permissions(darkmatter_scope, theirs) {
                    return conflict(ConflictKind::DifferentBlocks);
                }
                // A retry after a partial write: Claudine already holds the block.
                (mine.clone(), false, PermissionsHandoff::Transferred)
            }
            Some(Value::String(_)) => return conflict(ConflictKind::ClaudineReference),
            Some(_) => return manual(ManualReason::ClaudineUnusable),
        },
    };

    // The mapping as the reference target it will become.
    let target_text = serde_json::json!({ "permissions": block }).to_string();
    if dm_init::check_referenced_permissions(&target_text, &claudine.path, darkmatter_scope, context).is_err() {
        return manual(ManualReason::UnusableMapping);
    }
    if moved {
        let valid = std::str::from_utf8(&claudine.bytes)
            .is_ok_and(|text| validate_config_text(text, &claudine.path, scope).is_ok());
        if !valid {
            return manual(ManualReason::ClaudineInvalid);
        }
    }
    Ok(Decision { claudine_block: moved.then(|| block.clone()), refer: true, permissions, block: Some(block) })
}

/// Why [`commit_permissions_handoff`] saved nothing (or, for
/// [`ConfigWriteError::ReplacementFailed`](darkmatter::config::write::ConfigWriteError::ReplacementFailed),
/// only Claudine's file).
#[derive(Debug)]
pub(crate) enum HandoffCommitError {
    /// Claudine's edited file is not a valid configuration.
    Claudine(ClaudineError),
    /// Doctor only: Claudine's edited `permissions` would not be a usable
    /// reference target.
    UnusableTarget,
    /// Darkmatter's candidate was refused, or the transaction failed.
    Darkmatter(Box<InitError>),
}

/// Commits `plan` in one locked [`ConfigTransaction`]: Claudine's file first,
/// `.darkmatter.yaml` last, each edit made only while its file still holds
/// the bytes the plan observed.
///
/// Claudine's candidate is judged by its own validation (and in doctor mode
/// as a reference target of the scope too); Darkmatter's by
/// [`dm_init::check_config_text`], of the plan's own scope in doctor mode
/// (even when the home is the repository root), and of the scope
/// [`dm_init::commit_checked`] infers in setup mode.
pub(crate) fn commit_permissions_handoff(
    context: &FileResolutionContext,
    plan: HandoffPlan,
) -> Result<CommitOutcome, HandoffCommitError> {
    let write = |error| HandoffCommitError::Darkmatter(Box::new(InitError::Write(error)));
    let mut transaction = ConfigTransaction::new();
    if let Some(change) = plan.claudine_change {
        transaction.add(change).map_err(write)?;
    }
    if let Some(change) = plan.darkmatter_change {
        transaction.add(change).map_err(write)?;
    }
    let darkmatter_scope = darkmatter_scope_of(plan.scope);
    let claudine_identity = identity(&plan.claudine_config);
    let darkmatter_identity = identity(&plan.darkmatter_config);
    // A refusal of Claudine's candidate is reported as Claudine's own typed
    // error; the transaction's refusal only carries text.
    let refused: RefCell<Option<HandoffCommitError>> = RefCell::new(None);
    let committed = dm_init::commit_checked(transaction, context, &|candidate| {
        let target = identity(candidate.target);
        if target == claudine_identity {
            let refuse = |error| {
                *refused.borrow_mut() = Some(error);
                ValidationRefusal::new(format!("{} is invalid", candidate.target.display()))
            };
            let verdict = validate_config_text(candidate.candidate, candidate.target, plan.scope)
                .map_err(|error| refuse(HandoffCommitError::Claudine(error)))
                .and_then(|()| match plan.mode {
                    HandoffMode::Setup => Ok(()),
                    HandoffMode::Doctor => {
                        dm_init::check_referenced_permissions(candidate.candidate, candidate.target, darkmatter_scope, context)
                            .map_err(|_| refuse(HandoffCommitError::UnusableTarget))
                    }
                });
            return Some(verdict);
        }
        (plan.mode == HandoffMode::Doctor && target == darkmatter_identity)
            .then(|| dm_init::check_config_text(candidate.candidate, candidate.target, darkmatter_scope, context))
    });
    committed.map_err(|error| refused.into_inner().unwrap_or_else(|| HandoffCommitError::Darkmatter(Box::new(error))))
}

/// Darkmatter's scope of the same name.
pub(crate) const fn darkmatter_scope_of(scope: ConfigScope) -> DarkmatterScope {
    match scope {
        ConfigScope::User => DarkmatterScope::User,
        ConfigScope::Repository => DarkmatterScope::Repository,
    }
}

/// The reference setup writes into `.darkmatter.yaml` of `scope`.
fn darkmatter_reference(scope: ConfigScope) -> String {
    match scope {
        ConfigScope::User => format!("~/{CONFIG_DIR}/{CONFIG_FILE_NAME}"),
        ConfigScope::Repository => format!("./{CONFIG_DIR}/{CONFIG_FILE_NAME}"),
    }
}

/// The file's bytes and its document, when it exists and is a YAML mapping.
pub(crate) fn read_document(path: &Path) -> Option<(Vec<u8>, serde_json::Map<String, Value>)> {
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

/// Whether `left` and `right` name one file, by canonical identity (each
/// path as written when it cannot be canonicalized).
pub(crate) fn same_file(left: &Path, right: &Path) -> bool {
    identity(left) == identity(right)
}
