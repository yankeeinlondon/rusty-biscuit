//! The passive session of a dry run
//! ([`ComposeOptions::with_dry_run`](super::ComposeOptions::with_dry_run)).
//!
//! A dry run composes a document without any effect: no command runs, no
//! login shell is probed, no packet is sent, no approval is asked for, and no
//! cache is read or written. Work that needs one of those effects is
//! *deferred*: its authored text stays in the output as written, and the
//! session records a [`Deferral`] naming what was skipped, why, and where.
//!
//! Deferral propagates through evaluation. A frontmatter value that needs a
//! command keeps its authored text and is registered here; any expression
//! that reads that text, directly or through another value that read it,
//! fails with [`ExpressionError::Unavailable`](super::expression::ExpressionError::Unavailable)
//! instead of producing a value. That failure travels through lookups,
//! conditions, collections, function arguments, and paths like any other
//! evaluation error, and the surface that started the evaluation keeps its
//! own authored text: a `{{ … }}` span stays as written, a `::block` whose
//! condition is unavailable stays whole with its directive lines, and a
//! frontmatter value that read one is deferred in turn. Independent spans,
//! siblings, and branches still evaluate.
//!
//! The session is shared by every clone of the request's options, so a
//! transcluded child, a schema slot, a callback, and a nested `as_markdown`
//! composition all see the same deferred values and report into one list.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::markdown::compose::expression::ExpressionError;

/// What a dry run could not do, and so left as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeferralReason {
    /// The value comes from a command, which a dry run never runs.
    Command {
        /// The command as written.
        command: String,
    },
    /// A function would start the login shell to answer (`has_alias`,
    /// `can_execute`, …).
    ShellProbe {
        /// The function's name.
        function: String,
    },
    /// A function would send a network packet (`ping`, `ping_under`).
    NetworkProbe {
        /// The function's name.
        function: String,
    },
    /// A function would query a provider's API (`pr`, `cicd`, …).
    ProviderQuery {
        /// The function's name.
        function: String,
    },
    /// A URL the captured `permissions.network: ask` leaves needing an
    /// approval this run did not give; a dry run asks nothing, so the read
    /// is not made and what depends on it is not checked.
    NetworkNeedsApproval {
        /// The URL that would have been fetched.
        url: String,
    },
    /// A read the real run would ask the user to approve; a dry run asks
    /// nothing.
    ReadNeedsApproval {
        /// What would have been read.
        target: String,
    },
    /// A required input the caller gave no value for; a dry run never asks
    /// for one, so what reads it stays as written.
    MissingInput {
        /// The input's name.
        name: String,
    },
    /// The value reads another value that is itself deferred.
    DependsOn {
        /// The deferred value read, as it was named (`branch`, `fm.branch`).
        name: String,
    },
}

impl DeferralReason {
    /// A short phrase for a report row ("runs a command", …).
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Command { command } => format!("needs the output of `{command}`, which a dry run does not run"),
            Self::ShellProbe { function } => {
                format!("`{function}()` starts the login shell, which a dry run does not do")
            }
            Self::NetworkProbe { function } => {
                format!("`{function}()` sends network packets, which a dry run does not do")
            }
            Self::ProviderQuery { function } => {
                format!("`{function}()` queries a provider's API, which a dry run does not do")
            }
            Self::NetworkNeedsApproval { url } => {
                format!("fetching `{url}` needs network approval (`permissions.network: ask`), which a dry run does not ask for")
            }
            Self::ReadNeedsApproval { target } => {
                format!("reading {target} needs approval, which a dry run does not ask for")
            }
            Self::MissingInput { name } => format!("reads `{name}`, which has no value"),
            Self::DependsOn { name } => format!("reads `{name}`, which is not known until the document runs"),
        }
    }
}

/// Which authored construct a deferral left as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeferredConstruct {
    /// A top-level frontmatter value.
    FrontmatterValue,
    /// A `{{ … }}` span in the body or in frontmatter text.
    Interpolation,
    /// A body `::shell` directive line.
    ShellDirective,
    /// A body `::shell-block` region.
    ShellBlock,
    /// A `::block when=…` region whose condition could not be decided.
    BlockCondition,
    /// A `$schema` template whose slots could not be resolved without
    /// asking; the schema's checks were not run.
    SchemaTemplate,
    /// A `::file`/`::code` directive, or a `prologue`/`epilogue` value,
    /// whose target was not read; its content is not in the output.
    Transclusion,
}

/// One thing a dry run left as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Deferral {
    /// The construct left as written.
    pub construct: DeferredConstruct,
    /// Why.
    pub reason: DeferralReason,
    /// The authored text of the construct (a `{{ … }}` span, a command line,
    /// a condition), as written.
    pub authored: String,
    /// The top-level frontmatter key, for a frontmatter value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The one-based line in the document, when the surface knows it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

#[derive(Debug, Default)]
struct State {
    /// Authored text of each deferred value, mapped to why it is deferred.
    values: BTreeMap<String, DeferralReason>,
    /// Top-level names with no value, mapped to why a read of one is
    /// unavailable.
    absent: BTreeMap<String, DeferralReason>,
    deferrals: Vec<Deferral>,
    seen: HashSet<Deferral>,
}

/// The invocation-local record of a dry run, shared by every clone of the
/// request; see the [module docs](self).
#[derive(Debug, Clone, Default)]
pub struct PassiveSession {
    inner: Arc<Mutex<State>>,
}

impl PassiveSession {
    /// A new, empty session.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every deferral recorded so far, in the order recorded, each once.
    #[must_use]
    pub fn deferrals(&self) -> Vec<Deferral> {
        self.lock().deferrals.clone()
    }

    /// How many deferrals have been recorded; a mark for
    /// [`Self::deferrals_since`].
    #[must_use]
    pub fn mark(&self) -> usize {
        self.lock().deferrals.len()
    }

    /// The deferrals recorded after `mark`.
    #[must_use]
    pub fn deferrals_since(&self, mark: usize) -> Vec<Deferral> {
        self.lock().deferrals.iter().skip(mark).cloned().collect()
    }

    /// Whether two handles share one session.
    #[must_use]
    pub fn same_session(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Why the value whose text is `text` is deferred, when it is one.
    #[must_use]
    pub fn deferred_value(&self, text: &str) -> Option<DeferralReason> {
        self.lock().values.get(text).cloned()
    }

    /// Declares that the top-level value `name` has no value in this dry run
    /// for `reason` (a missing input, a value only a run writes): a read of
    /// it, or below it, that finds nothing is unavailable instead of empty,
    /// and the schema stage does not report it missing.
    pub fn defer_absent(&self, name: &str, reason: DeferralReason) {
        self.lock().absent.entry(name.to_string()).or_insert(reason);
    }

    /// Why the top-level value `name` is declared absent, when it is.
    #[must_use]
    pub fn absent_value(&self, name: &str) -> Option<DeferralReason> {
        self.lock().absent.get(name).cloned()
    }

    /// Why a `{{ … }}` span `text` holds was left as written, when one was:
    /// the reason of the first such span recorded. A directive line holding
    /// one names no target until the value is known.
    #[must_use]
    pub fn deferred_span_in(&self, text: &str) -> Option<DeferralReason> {
        self.lock()
            .deferrals
            .iter()
            .find(|deferral| deferral.construct == DeferredConstruct::Interpolation && text.contains(&deferral.authored))
            .map(|deferral| deferral.reason.clone())
    }

    /// Why the `prologue`/`epilogue` value `text` is not known, when it is
    /// not: it is a deferred value itself, or holds a `{{ … }}` span left as
    /// written. Such a value names no target.
    #[must_use]
    pub fn deferred_reference(&self, text: &str) -> Option<DeferralReason> {
        self.deferred_value(text).or_else(|| self.deferred_span_in(text))
    }

    /// Records `deferral`, once.
    pub fn record(&self, deferral: Deferral) {
        let mut state = self.lock();
        if state.seen.insert(deferral.clone()) {
            state.deferrals.push(deferral);
        }
    }

    /// Records that the frontmatter value `key`, whose authored text stays
    /// `authored`, is deferred for `reason`; later reads of that text are
    /// unavailable.
    pub(crate) fn defer_value(&self, key: &str, authored: &str, reason: DeferralReason, line: Option<usize>) {
        self.lock().values.entry(authored.to_string()).or_insert_with(|| reason.clone());
        self.record(Deferral {
            construct: DeferredConstruct::FrontmatterValue,
            reason,
            authored: authored.to_string(),
            key: Some(key.to_string()),
            line,
        });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The deferral reason of reading `path`, whose value is `value`, in a dry
/// run: the value is the authored text of a deferred value. `None` outside a
/// dry run and for every other value.
pub(crate) fn unavailable_read(
    session: Option<&PassiveSession>,
    path: &str,
    value: Option<&serde_json::Value>,
) -> Option<DeferralReason> {
    let text = value?.as_str()?;
    session?.deferred_value(text)?;
    Some(DeferralReason::DependsOn { name: path.to_string() })
}

/// Refuses a read of `path`, which `lookup` resolved to `binding`, when it
/// reads a deferred value: the value itself, or a member below one.
///
/// ## Errors
///
/// [`ExpressionError::Unavailable`] for a read of a deferred value in a dry
/// run.
pub(crate) fn check_read<L: crate::markdown::compose::expression::EvaluationLookup + ?Sized>(
    lookup: &L,
    path: &str,
    binding: &crate::markdown::compose::expression::ResolvedBinding,
) -> Result<(), ExpressionError> {
    let Some(session) = lookup.resolution_context_ref().and_then(|context| context.passive.as_ref()) else {
        return Ok(());
    };
    if let Some(reason) = unavailable_read(Some(session), path, binding.value()) {
        return Err(ExpressionError::Unavailable(Box::new(reason)));
    }
    // A value declared absent has no binding to read; a read that finds one
    // (a default, a setter) is an ordinary read.
    let top = path.strip_prefix("fm.").unwrap_or(path);
    let top = top.split(['.', '[']).next().unwrap_or(top);
    if binding.value().is_none_or(serde_json::Value::is_null)
        && let Some(reason) = session.absent_value(top)
    {
        return Err(ExpressionError::Unavailable(Box::new(reason)));
    }
    if let Some(root) = dotted_root(path) {
        let root_path = if path.starts_with("fm.") { format!("fm.{root}") } else { root.to_string() };
        if let Ok(root_binding) = lookup.resolve(&root_path)
            && unavailable_read(Some(session), &root_path, root_binding.value()).is_some()
        {
            return Err(ExpressionError::Unavailable(Box::new(DeferralReason::DependsOn { name: root_path })));
        }
    }
    Ok(())
}

/// The top-level value a dotted `path` reads below (`fm.x.y` and `x.y` both
/// read below `x`); `None` for a path that names a top-level value itself.
pub(crate) fn dotted_root(path: &str) -> Option<&str> {
    let path = path.strip_prefix("fm.").unwrap_or(path);
    path.split_once('.').map(|(root, _)| root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deferral_is_recorded_once_and_its_value_becomes_unavailable() {
        let session = PassiveSession::new();
        let reason = DeferralReason::Command { command: "git branch".into() };
        session.defer_value("branch", "$(git branch)", reason.clone(), Some(2));
        session.defer_value("branch", "$(git branch)", reason, Some(2));

        assert_eq!(session.deferrals().len(), 1);
        let shared = session.clone();
        assert!(shared.same_session(&session));
        assert_eq!(
            unavailable_read(Some(&shared), "fm.branch", Some(&serde_json::json!("$(git branch)"))),
            Some(DeferralReason::DependsOn { name: "fm.branch".into() })
        );
        assert_eq!(unavailable_read(Some(&shared), "title", Some(&serde_json::json!("x"))), None);
        assert_eq!(unavailable_read(None, "branch", Some(&serde_json::json!("$(git branch)"))), None);
    }

    #[test]
    fn a_dotted_read_names_its_top_level_value() {
        assert_eq!(dotted_root("fm.server.name"), Some("server"));
        assert_eq!(dotted_root("server.name"), Some("server"));
        assert_eq!(dotted_root("fm.server"), None);
        assert_eq!(dotted_root("server"), None);
    }
}
