//! The captured `permissions` one approval decides with, and saving an
//! "always allow" choice to the user's configuration.
//!
//! Every decision reads Darkmatter's evaluated [`PermissionPolicy`] of the
//! invocation's captured configuration; nothing is re-read from disk during a
//! run. User-facing guide: `darkmatter/docs/topics/configuration.md`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use biscuit_file::FileResolutionContext;
use darkmatter::config::load::{ConfigError, ConfigLoadMode, ConfigSnapshot};
use darkmatter::config::permissions::persist::{NotSaved, PersistentApproval, save_user_approval};
use darkmatter::config::permissions::{PermissionPolicy, PermissionVerdict, PolicyExclusion};
use darkmatter::markdown::compose::shell_expansion::template::FilledChain;
use darkmatter::markdown::compose::shell_expansion::{ShellRuleEntry, ShellRuleSet};

use crate::harness::error::HarnessError;

/// The `permissions` of the configuration captured for one source document.
#[derive(Debug, Clone, Default)]
pub enum CapturedPermissions {
    /// The caller captured no configuration (a library caller that supplies
    /// none): nothing is pre-approved, no `!` entry applies, and an approval
    /// cannot be saved. Every CLI route supplies [`Self::Policy`] or
    /// [`Self::Invalid`].
    #[default]
    Absent,
    /// The evaluated policy.
    Policy(Arc<PermissionPolicy>),
    /// The configuration, or its `permissions` block, is invalid: every
    /// command that reaches a policy decision fails with this error. Never
    /// replaced by an empty or default policy.
    Invalid(Arc<ConfigError>),
}

impl CapturedPermissions {
    /// The evaluated policy of `snapshot`.
    #[must_use]
    pub fn of_snapshot(snapshot: &ConfigSnapshot) -> Self {
        match snapshot.policy() {
            Ok(policy) => Self::Policy(Arc::clone(policy)),
            Err(error) => Self::Invalid(Arc::new(error)),
        }
    }

    /// The evaluated policy of a configuration load.
    #[must_use]
    pub fn of_load(load: Result<Arc<ConfigSnapshot>, ConfigError>) -> Self {
        match load {
            Ok(snapshot) => Self::of_snapshot(&snapshot),
            Err(error) => Self::Invalid(Arc::new(error)),
        }
    }

    /// The policy `invocation` captured for a document resolving through
    /// `context` (its repository's committed and working-copy layers over the
    /// invocation's user file).
    #[must_use]
    pub fn for_source(
        invocation: &crate::invocation_context::InvocationContext,
        context: &FileResolutionContext,
    ) -> Self {
        Self::of_load(invocation.darkmatter_config_for(context))
    }

    /// The policy a request built over `prepared` (its host capture) would
    /// read for `context`; without a host capture, a passive load of
    /// `context`, which needs no committed tree only outside a repository.
    #[must_use]
    pub fn for_prepared(
        prepared: Option<&darkmatter::markdown::compose::ComposeContext>,
        context: &FileResolutionContext,
    ) -> Self {
        match prepared.and_then(|prepared| prepared.config_source()).and_then(|source| source.config_snapshot_for(context)) {
            Some(load) => Self::of_load(load),
            None => Self::of_load(ConfigSnapshot::load_captured(context, None, ConfigLoadMode::Passive).map(Arc::new)),
        }
    }

    /// [`Self::for_source`] when the caller holds the invocation, else
    /// [`Self::for_prepared`].
    #[must_use]
    pub fn resolve(
        invocation: Option<&crate::invocation_context::InvocationContext>,
        prepared: Option<&darkmatter::markdown::compose::ComposeContext>,
        context: &FileResolutionContext,
    ) -> Self {
        match invocation {
            Some(invocation) => Self::for_source(invocation, context),
            None => Self::for_prepared(prepared, context),
        }
    }

    /// The policy, or `None` when no configuration was captured.
    ///
    /// ## Errors
    ///
    /// [`HarnessError::ShellPermissionsInvalid`] for an invalid capture.
    pub fn policy(&self) -> Result<Option<&Arc<PermissionPolicy>>, HarnessError> {
        match self {
            Self::Absent => Ok(None),
            Self::Policy(policy) => Ok(Some(policy)),
            Self::Invalid(error) => Err(HarnessError::ShellPermissionsInvalid { source: Box::new((**error).clone()) }),
        }
    }

    /// Every layer's `!` rules (without the `!`), for a filled template's
    /// check; none when no configuration was captured.
    ///
    /// ## Errors
    ///
    /// The configuration error of an invalid capture.
    pub fn exclusions(&self) -> Result<ShellRuleSet, ConfigError> {
        match self {
            Self::Absent => Ok(ShellRuleSet::default()),
            Self::Policy(policy) => Ok(policy.shell().exclusions().clone()),
            Self::Invalid(error) => Err((**error).clone()),
        }
    }
}

/// One approval's view of the captured policy plus the rules this run's
/// "always allow" choices added (a rule applies to the rest of the run
/// whether or not it could be saved).
pub(super) struct Decider<'a> {
    policy: Option<&'a Arc<PermissionPolicy>>,
    run_rules: &'a Mutex<ShellRuleSet>,
}

impl<'a> Decider<'a> {
    /// ## Errors
    ///
    /// [`HarnessError::ShellPermissionsInvalid`] for an invalid capture.
    pub(super) fn new(options: &'a super::ShellApprovalOptions) -> Result<Self, HarnessError> {
        Ok(Self { policy: options.permissions.policy()?, run_rules: &options.run_rules })
    }

    /// Whether `positive` holds for this run's saved rules, the user rules,
    /// or both repository layers. Exclusions are not consulted.
    pub(super) fn allows(&self, positive: impl Fn(&ShellRuleSet) -> bool) -> bool {
        positive(&self.run_rules.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
            || self.policy.is_some_and(|policy| policy.shell().preapproves(&positive))
    }

    /// The `!` entry `excluded` matches, judged layer by layer.
    pub(super) fn exclusion(&self, excluded: impl Fn(&ShellRuleSet) -> bool) -> Option<PolicyExclusion> {
        match self.policy?.shell().verdict(|_| true, excluded) {
            PermissionVerdict::ApprovalRequired { exclusion } => exclusion,
            PermissionVerdict::Preapproved => None,
        }
    }

    /// The user configuration file an "always allow" choice saves to.
    pub(super) fn save_target(&self) -> Option<PathBuf> {
        let policy = self.policy?;
        policy.home().map(|_| policy.user().block_file().to_path_buf())
    }

    /// Saves `rule` to the user configuration and to this run's rules. A rule
    /// that cannot be saved still approves the run; the reason is warned as
    /// `dm.config.not_saved`.
    pub(super) fn save(&self, rule: ShellRuleEntry, command: Option<FilledChain>) {
        if let Some(policy) = self.policy
            && let Err(not_saved) = save_user_approval(policy, &PersistentApproval::ShellRule { rule: rule.clone(), command })
        {
            report_not_saved(&not_saved);
        }
        let mut run_rules = self.run_rules.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !run_rules.entries.contains(&rule) {
            run_rules.entries.push(rule);
        }
    }
}

/// Warns that an approval applies to this run only.
fn report_not_saved(not_saved: &NotSaved) {
    tracing::warn!(code = not_saved.code(), file = ?not_saved.file, "{}", not_saved.message);
    crate::render::console::write_stderr_line(&format!("warning [{}]: {not_saved}", not_saved.code()));
}
