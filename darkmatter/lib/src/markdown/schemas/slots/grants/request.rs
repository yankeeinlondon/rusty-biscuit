//! What an approval handler is asked about a slot's read, and what it may
//! answer.

use std::path::{Path, PathBuf};

use super::{Grant, GrantDecision, GrantScope, GrantTarget};
use crate::markdown::compose::expression::functions::ReadKind;
use crate::markdown::compose::expression::read_gate::{ReadLocation, ReadTarget};

/// One read a slot needs approval for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotGrantRequest {
    /// The function reading.
    pub function: String,
    /// Content, an existence probe, or a listing.
    pub kind: ReadKind,
    /// What is read (canonical for a local path).
    pub location: ReadLocation,
    /// The schema file the slot was written in; `None` for an inline
    /// document `$schema`.
    pub schema: Option<PathBuf>,
    /// The answers the handler may give, in the order a prompt lists them:
    /// allow for this run, "always allow" (a file only), deny for this run.
    /// A handler answering with anything else is treated as unanswered.
    pub choices: Vec<Grant>,
}

/// A handler's answer to one [`SlotGrantRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotGrantDecision {
    /// Allow reads of `target` for `scope`: this run, or always (a file,
    /// saved to the user's `permissions.file-read`).
    Allow { target: GrantTarget, scope: GrantScope },
    /// Deny reads of `target` for this run (`scope` is always `Once`). A
    /// canceled prompt or ended input is a deny for this run.
    Deny { target: GrantTarget, scope: GrantScope },
    /// No answer: the read stays `dm.schema.slot_approval_required`.
    Unanswered,
}

impl SlotGrantDecision {
    /// The grant this decision makes, if any.
    pub fn grant(&self) -> Option<Grant> {
        match self {
            Self::Allow { target, scope } => {
                Some(Grant { decision: GrantDecision::Allow, target: target.clone(), scope: *scope })
            }
            Self::Deny { target, scope } => {
                Some(Grant { decision: GrantDecision::Deny, target: target.clone(), scope: *scope })
            }
            Self::Unanswered => None,
        }
    }

    /// The decision that makes `grant`.
    pub fn from_grant(grant: &Grant) -> Self {
        match grant.decision {
            GrantDecision::Allow => Self::Allow { target: grant.target.clone(), scope: grant.scope },
            GrantDecision::Deny => Self::Deny { target: grant.target.clone(), scope: grant.scope },
        }
    }
}

impl SlotGrantRequest {
    /// The request for `target`, read by a slot written in `schema`.
    /// `persist` offers "always allow" for a file (the configuration's
    /// permissions are valid, so an approval can be saved to them).
    pub(crate) fn for_target(target: &ReadTarget, schema: Option<&Path>, persist: bool) -> Self {
        Self {
            function: target.function.clone(),
            kind: target.kind,
            location: target.location.clone(),
            schema: schema.map(Path::to_path_buf),
            choices: grant_choices(target, persist),
        }
    }
}

/// The answers a prompt offers for `target`, in order: allow for this run,
/// "always allow" (a file only, when `persist`), then deny for this run.
/// Empty for a listing or a glob, which is never asked about: it returns
/// only the files the slot may read without asking.
pub(crate) fn grant_choices(target: &ReadTarget, persist: bool) -> Vec<Grant> {
    let allowed = match &target.location {
        ReadLocation::Local(_) if target.kind == ReadKind::Listing => return Vec::new(),
        ReadLocation::Glob(_) => return Vec::new(),
        ReadLocation::Local(path) => GrantTarget::File(path.clone()),
        ReadLocation::Remote(url) => GrantTarget::Url(url.clone()),
    };
    let grant = |decision, scope| Grant { decision, target: allowed.clone(), scope };
    let mut choices = vec![grant(GrantDecision::Allow, GrantScope::Once)];
    if persist && matches!(allowed, GrantTarget::File(_)) {
        choices.push(grant(GrantDecision::Allow, GrantScope::Persisted));
    }
    choices.push(grant(GrantDecision::Deny, GrantScope::Once));
    choices
}
