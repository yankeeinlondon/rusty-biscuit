//! The error envelope: the owned projection of the diagnostic selected to speak
//! for a failure.
//!
//! Concrete Rust error values never cross a process, wire, or persistence
//! boundary. At the last in-process boundary they project **once** into a
//! [`DiagnosticSnapshot`], and every downstream consumer reads that shared
//! shape rather than rebuilding facets from the error itself: the lifecycle
//! `err.*` projection embeds one ([`LifecycleErrorInfo`]), and a host's
//! terminal rendering and machine output read the same value.
//!
//! The envelope is plain data. Which error in a chain speaks, and which facets
//! a code carries, is the host's diagnostic catalog; a host supplies it through
//! [`DiagnosticCatalog`].
//!
//! Two rules make the snapshot survive version skew in both directions:
//!
//! - **Facets are owned strings, not closed enums.** A newer producer may know
//!   a code an older consumer cannot name; the older consumer must still read,
//!   store, and forward it. Re-narrowing a string to an enum is the
//!   *consumer's* choice, made where an unknown value can be handled — never a
//!   deserialization failure.
//! - **`detail` is an opaque [`Value`].** A catalog evolves additively, so a
//!   field added after a consumer shipped round-trips through it untouched.
//!
//! [`LifecycleErrorInfo`]: super::context::LifecycleErrorInfo

use std::any::Any;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

use biscuit_terminal::discovery::eval::strip_ansi_codes;
use biscuit_terminal::terminal::Terminal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schema version stamped into every [`DiagnosticSnapshot`].
///
/// Bumped only for a **non-additive** change — a removed or re-typed field.
/// Adding a facet, a detail key, or a new code is additive by construction and
/// does not move this number, because both sides already tolerate values they
/// do not know.
pub const DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Upper bound, in characters, of a [`concise_message`].
pub const CONCISE_MESSAGE_MAX_CHARS: usize = 240;

/// The public projection of the diagnostic selected to speak for a failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    /// [`DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION`] at the time of projection.
    pub schema_version: u32,

    /// Coarse domain slug — always the dotted prefix of `code`.
    pub category: String,
    /// Stable dotted code (`composition.invalid_file_reference`).
    pub code: String,
    /// Generic-strategy slug (`correctable`).
    pub disposition: String,
    /// Who must remediate (`author`).
    pub origin: String,
    /// Operator-facing severity slug (`info`/`warning`/`error`).
    pub severity: String,

    /// The catalog-shaped instance payload: every field `code` declares is a
    /// present key, with an unavailable optional carried as `null` rather than
    /// omitted.
    pub detail: Value,

    /// Concise, notification-safe message — single-line, escape-free, and
    /// length-clamped (see [`concise_message`]), so a TTS route, a webhook, or a
    /// desktop notification can use it verbatim. It is *not* the multi-line
    /// rendered block.
    pub message: String,

    /// The next registered diagnostic below this one, when the chain has one.
    ///
    /// One level only: [`DiagnosticCause`] carries no cause of its own, so
    /// `cause.cause` is unrepresentable in v1 rather than merely undocumented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<DiagnosticCause>,

    /// In-process source appendix retained across an intentional snapshot
    /// boundary. It is never serialized: frontmatter remains absent from
    /// `err.*`, machine output, persistence, and wire representations.
    #[serde(skip)]
    pub frontmatter_excerpt: Option<SourceExcerpt>,
}

/// The one-level cause projection of a [`DiagnosticSnapshot`].
///
/// Structurally identical to a snapshot minus the schema version (the parent
/// stamps it once) and minus a nested cause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticCause {
    /// Coarse domain slug.
    pub category: String,
    /// Stable dotted code.
    pub code: String,
    /// Generic-strategy slug.
    pub disposition: String,
    /// Who must remediate.
    pub origin: String,
    /// Operator-facing severity slug.
    pub severity: String,
    /// The catalog-shaped instance payload.
    pub detail: Value,
    /// Concise, notification-safe message.
    pub message: String,
}

/// A host's diagnostic catalog: the policy that turns an error into a
/// [`DiagnosticSnapshot`].
///
/// The envelope's catalog-dependent constructors
/// ([`LifecycleErrorInfo::from_error_or_action_with`] and friends) take one, so
/// the projection logic exists once while the codes, facets, and selection
/// walk stay the host's.
///
/// [`LifecycleErrorInfo::from_error_or_action_with`]: super::context::LifecycleErrorInfo::from_error_or_action_with
pub trait DiagnosticCatalog: Sync {
    /// Project the diagnostic the host selects to speak for `error`'s chain.
    ///
    /// ## Returns
    ///
    /// `None` when nothing in the chain is a registered diagnostic.
    fn select(&self, error: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot>;

    /// Project a catalog `code` with `message` and no typed source.
    ///
    /// ## Returns
    ///
    /// `None` for a code absent from the catalog.
    fn snapshot_for_code(&self, code: &str, message: String) -> Option<DiagnosticSnapshot>;

    /// Project a synthesized failure label (an `error_kind` such as a provider
    /// timeout) with `message`.
    ///
    /// ## Returns
    ///
    /// `None` when the label names no catalog code; a generic action verb
    /// (`shell`) is not a failure label.
    fn snapshot_for_error_kind(&self, label: &str, message: String) -> Option<DiagnosticSnapshot>;
}

/// A source appendix rendered after a diagnostic, such as the frontmatter
/// lines an error is rooted in.
///
/// The envelope carries it opaquely so the host keeps the logic that locates
/// and renders the lines.
pub trait SourceAppendix: Any + fmt::Debug + Send + Sync {
    /// Render the appendix for `term`, including its leading separation from
    /// the diagnostic it follows.
    fn render_appendix(&self, term: &Terminal) -> String;
}

/// A shared, in-process [`SourceAppendix`] carried by a [`DiagnosticSnapshot`].
///
/// Two excerpts are equal when they hold equal values of the same type.
#[derive(Clone)]
pub struct SourceExcerpt {
    appendix: Arc<dyn SourceAppendix>,
    same: fn(&dyn Any, &dyn Any) -> bool,
}

impl SourceExcerpt {
    /// Wrap `appendix`.
    pub fn new<T: SourceAppendix + PartialEq>(appendix: T) -> Self {
        Self {
            appendix: Arc::new(appendix),
            same: |left, right| match (left.downcast_ref::<T>(), right.downcast_ref::<T>()) {
                (Some(left), Some(right)) => left == right,
                _ => false,
            },
        }
    }

    /// Render the appendix for `term`.
    pub fn render_appendix(&self, term: &Terminal) -> String {
        self.appendix.render_appendix(term)
    }

    /// The wrapped appendix, when it is a `T`.
    pub fn downcast_ref<T: SourceAppendix>(&self) -> Option<&T> {
        (self.appendix.as_ref() as &dyn Any).downcast_ref::<T>()
    }
}

impl fmt::Debug for SourceExcerpt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.appendix.fmt(f)
    }
}

impl PartialEq for SourceExcerpt {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.appendix, &other.appendix)
            || (self.same)(self.appendix.as_ref() as &dyn Any, other.appendix.as_ref() as &dyn Any)
    }
}

/// Reduce arbitrary error text to a concise, notification-safe line.
///
/// Escapes are stripped, the text is collapsed to one line, and it is clamped
/// to [`CONCISE_MESSAGE_MAX_CHARS`] characters. Every message that can reach
/// TTS, messaging, or a desktop notification goes through this one rule. The
/// pass is idempotent.
///
/// ## Examples
///
/// ```
/// use darkmatter::lifecycle::envelope::concise_message;
///
/// assert_eq!(concise_message("first\n  second\t"), "first second");
/// ```
pub fn concise_message(text: &str) -> String {
    concise_message_with_suffix(text, "")
}

/// [`concise_message`] with `suffix` appended after clamping.
///
/// The suffix's characters are reserved from the budget, so a suffix such as
/// `" (attempt 2)"` survives truncation rather than being cut off.
pub fn concise_message_with_suffix(base: &str, suffix: &str) -> String {
    let single_line = collapse_to_line(&strip_ansi_codes(base));
    let budget = CONCISE_MESSAGE_MAX_CHARS.saturating_sub(suffix.chars().count());
    format!("{}{suffix}", clamp_chars(&single_line, budget))
}

/// Fold a (possibly multi-line) string to a single line: split on any control
/// character (newlines, tabs, stray escape bytes), trim each segment, and
/// rejoin with a single space.
fn collapse_to_line(text: &str) -> String {
    text.split(|c: char| c.is_control())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Clamp `line` to at most `budget` characters, reserving one character for
/// the trailing `…` when truncation occurs so the returned string never
/// exceeds `budget`.
fn clamp_chars(line: &str, budget: usize) -> String {
    if line.chars().count() <= budget {
        return line.to_string();
    }
    let clamped: String = line.chars().take(budget.saturating_sub(1)).collect();
    format!("{clamped}…")
}

#[cfg(test)]
mod tests;
