//! The host seam: what the executor asks the application that embeds it.
//!
//! The executor owns evaluation order, `set` snapshot semantics, deferred
//! spans, control directives, and error routing. Everything that depends on
//! the host's own model — which globals a scope can read, where `set` values
//! accumulate, the loop a `break` ends, how `prep` runs a target, the user's
//! shell policy, and how a failure is classified — is a [`LifecycleHost`]
//! method.

use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::effects::EffectEngine;
use crate::markdown::compose::expression::{BindingView, ResolutionContext, RuntimeBinding};
use crate::markdown::compose::shell_expansion::{ShellExpansionError, ShellRuleSet};
use crate::markdown::compose::{ComposeContext, ContextBuildError};

use super::super::context::{LifecycleCause, LifecycleErrorInfo, LifecycleEvaluationReason};
use super::super::envelope::DiagnosticSnapshot;
use super::super::registry::EventRegistry;

/// The host's side of lifecycle execution.
///
/// One value serves one event (or one task stack): its answers may depend on
/// the scope the host evaluates that event in.
pub trait LifecycleHost {
    /// The host's events, which name the lifecycle keys a `with:` overlay
    /// must not fill with template syntax.
    fn registry(&self) -> &EventRegistry;

    /// The binding view and one runtime entry per lifecycle global the
    /// current scope declares.
    ///
    /// Unavailability is the view's decision, so an expression that reads a
    /// global the scope declares unavailable fails rather than reading `null`.
    fn bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>);

    /// Dotted frontmatter keys an expression may read by their full name as
    /// well as by path (the effective state's name-coercion keys).
    fn name_coercion_keys(&self) -> Vec<String>;

    /// The expression resolution context for expressions authored by
    /// `source_path`, reusing `prepared_context` when there is one.
    ///
    /// `file_ref_fallback_dir` is diagnostic metadata for an implicit
    /// reference, never a resolution candidate.
    ///
    /// ## Errors
    ///
    /// The [`ContextBuildError`] when `file_resolution_context` fails
    /// validation.
    fn resolution_context(
        &self,
        source_path: &Path,
        prepared_context: Option<&ComposeContext>,
        file_resolution_context: &biscuit_file::FileResolutionContext,
        file_ref_fallback_dir: Option<&Path>,
    ) -> Result<ResolutionContext, ContextBuildError>;

    /// Capture the `ctx` snapshot a whole `with:` overlay evaluates against
    /// when the event carries no prepared one: rooted at `base`, scanned
    /// against `content`.
    fn capture_overlay_context(&self, base: &Path, content: &str) -> ComposeContext;

    /// `value` as the host stores evaluated data written into frontmatter,
    /// so a later preparation reads it back as data rather than as a
    /// template.
    fn persisted_data(&self, value: &Value) -> Value;

    /// Commit one `set` action's `updates` to the invocation's runtime state
    /// and return each destination's prior value, selected against
    /// `prior_base` when the runtime state holds none.
    ///
    /// The commit is atomic: a refusal leaves no partial batch.
    ///
    /// ## Errors
    ///
    /// The host's refusal (a reserved or invalid destination), which the
    /// executor reports as a dispatch failure of `set`.
    fn commit_set(
        &self,
        engine: &EffectEngine,
        updates: &IndexMap<String, Value>,
        prior_base: &Map<String, Value>,
    ) -> Result<IndexMap<String, Value>, Box<dyn StdError + Send + Sync>>;

    /// Whether the evaluation runs inside a loop iteration, where `break` has
    /// a loop to end.
    fn in_loop(&self) -> bool;

    /// Record a `break` against the innermost loop. Called only after
    /// [`Self::in_loop`] answered `true` and the reason evaluated.
    fn record_break(&self, reason: Option<String>, code: u8);

    /// Run a `prep` target to completion and return.
    ///
    /// ## Errors
    ///
    /// [`PrepError::Unavailable`] when this thread has no runner, or
    /// [`PrepError::Failed`] when the target did not return successfully.
    fn prep(&self, call: &PrepRequest<'_>) -> Result<(), PrepError>;

    /// The user blacklist a template-mode command approved under
    /// `policy_root` must also pass once it is filled.
    fn user_blacklist(&self, policy_root: Option<&Path>) -> ShellRuleSet;

    /// Whether the user interrupted the run; the cancellation probe.
    fn interrupted(&self) -> bool;

    /// Whether an interrupt ends this event's stack before its next item, as
    /// `stop` would. A host whose cancellation cleanup runs in a separate
    /// event answers `true` for its normal events; by default an interrupt
    /// never cuts a stack short.
    fn stops_on_interrupt(&self) -> bool {
        false
    }

    /// Whether `name` is a sound effect the host can play.
    fn is_known_effect(&self, name: &str) -> bool;

    /// Classify an executor failure in the host's diagnostic catalog.
    fn failure(&self, failure: LifecycleRunFailure) -> LifecycleErrorInfo;
}

/// One evaluated `prep` call: the target and overlay have already resolved.
#[derive(Debug, Clone, Copy)]
pub struct PrepRequest<'a> {
    /// The evaluated target reference, as the directive resolved it.
    pub target: &'a str,
    /// The evaluated `with:` overlay: data, never templates.
    pub overlay: &'a IndexMap<String, Value>,
    /// The document that authored the directive; the target resolves
    /// relative to it.
    pub caller: &'a Path,
    /// The caller's captured file-resolution context.
    pub file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// The authored path of the directive, e.g. `initialize[0].prep`.
    pub property: &'a str,
}

/// Why a `prep` call did not return successfully.
#[derive(Debug, Clone, PartialEq)]
pub enum PrepError {
    /// No runner is installed, so nothing was called.
    Unavailable,
    /// The target ran and did not return successfully.
    Failed {
        /// What the target reported.
        message: String,
        /// The failure's own diagnostic identity, when it carried one.
        snapshot: Option<Box<DiagnosticSnapshot>>,
    },
}

/// A lifecycle failure whose classification belongs to the host's catalog:
/// one the executor raised, or one the run guard's cancellation cleanup
/// raised.
///
/// Each variant carries the facts its diagnostic names; the host turns it into
/// the [`LifecycleErrorInfo`] that `err.*`, rendering, and machine output read.
#[derive(Debug)]
pub enum LifecycleRunFailure {
    /// An `effect` resolved to a name the host cannot play.
    UnknownEffect {
        /// The event's property name.
        event: String,
        /// The resolved effect name.
        name: String,
    },
    /// A template-mode shell command was refused before it ran.
    ShellTemplateRefused {
        /// The document that authored it.
        source_path: PathBuf,
        /// The authored property path.
        property: String,
        /// The command as authored; filled arguments may hold secrets.
        command: String,
        /// The refusal's rendered message.
        message: String,
        /// The refusal.
        error: ShellExpansionError,
    },
    /// An event-time expression in a lifecycle surface raised.
    EvaluationFailed {
        /// The document that authored it.
        source_path: PathBuf,
        /// The event's property name.
        event: String,
        /// The surface that raised (`set`).
        surface: String,
        /// The cause's message.
        message: String,
        /// The authored property path.
        property: Option<String>,
        /// Why the evaluation failed.
        reason: LifecycleEvaluationReason,
        /// The typed cause.
        cause: Option<LifecycleCause>,
    },
    /// A `break` ran outside any loop.
    BreakWithoutLoop {
        /// The document that authored it.
        source_path: PathBuf,
        /// The authored property path.
        property: String,
    },
    /// A `prep` ran on a thread with no runner.
    PrepUnavailable {
        /// The document that authored it.
        source_path: PathBuf,
        /// The authored property path.
        property: String,
    },
    /// A `prep` target did not return successfully.
    PrepFailed {
        /// The document that authored it.
        source_path: PathBuf,
        /// The authored property path.
        property: String,
        /// The evaluated target.
        target: String,
        /// What the target reported.
        message: String,
        /// The target failure's own diagnostic identity.
        snapshot: Option<Box<DiagnosticSnapshot>>,
    },
    /// A `proxy.with` or `retry.with` value failed to evaluate.
    WithEvaluationFailed {
        /// The document that authored it.
        source_path: PathBuf,
        /// The authored directive.
        property: String,
        /// The value's path below the directive (`with.key[0]`).
        path: String,
        /// The document the overlay was built for.
        target: String,
        /// The cause's message.
        message: String,
        /// The typed cause.
        cause: Option<LifecycleCause>,
    },
    /// Cancellation cleanup requested a recovery (`retry`, `resume`, `proxy`,
    /// or `defer`), which cancellation refuses: the cancellation keeps the
    /// terminal outcome.
    CancellationRecovery {
        /// The document whose cleanup requested it.
        source_path: PathBuf,
        /// The refused directive's verb.
        action: String,
    },
}
