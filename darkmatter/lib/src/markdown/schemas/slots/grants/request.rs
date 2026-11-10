//! What an approval handler is asked about a slot's read, and what it may
//! answer.

use std::path::{Component, Path, PathBuf};

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
    /// The grants the handler may answer with, in the order a prompt lists
    /// them. A handler answering with anything else is treated as
    /// unanswered.
    pub choices: Vec<Grant>,
}

/// A handler's answer to one [`SlotGrantRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotGrantDecision {
    /// Allow reads of `target` for `scope`.
    Allow { target: GrantTarget, scope: GrantScope },
    /// Deny reads of `target` for `scope`. A canceled prompt or ended input
    /// is a deny for this run (`Once`).
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
    /// `persist` offers persisted choices (there is a grant store).
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

/// The grants that would allow `target`, each for this run and (when
/// `persist`) persisted, followed by a persisted deny and a deny for this
/// run. Empty when no grant can allow the read (a relative glob).
///
/// - a file: this file, this file's folder;
/// - a folder listing: this folder;
/// - an absolute glob: the folder its literal leading components name;
/// - a URL: this exact URL, this URL whatever its query (only when it has
///   one), this site.
pub(crate) fn grant_choices(target: &ReadTarget, persist: bool) -> Vec<Grant> {
    let allows: Vec<GrantTarget> = match &target.location {
        ReadLocation::Local(path) if target.kind == ReadKind::Listing => vec![GrantTarget::Dir(path.clone())],
        ReadLocation::Local(path) => {
            let mut targets = vec![GrantTarget::File(path.clone())];
            if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
                targets.push(GrantTarget::Dir(parent.to_path_buf()));
            }
            targets
        }
        ReadLocation::Glob(pattern) => match glob_folder(pattern) {
            Some(folder) => vec![GrantTarget::Dir(folder)],
            None => return Vec::new(),
        },
        ReadLocation::Remote(url) => {
            let mut targets = vec![GrantTarget::Url(url.clone())];
            if url.query().is_some() {
                let mut stripped = url.clone();
                stripped.set_query(None);
                targets.push(GrantTarget::UrlAnyQuery(stripped));
            }
            if let Some(host) = url.host_str() {
                let port = url.port().filter(|port| Some(*port) != default_port(url.scheme()));
                targets.push(GrantTarget::Site { scheme: url.scheme().to_string(), host: host.to_string(), port });
            }
            targets
        }
    };
    let scopes: &[GrantScope] = if persist { &[GrantScope::Once, GrantScope::Persisted] } else { &[GrantScope::Once] };
    let mut choices: Vec<Grant> = allows
        .iter()
        .flat_map(|target| {
            scopes.iter().map(|scope| Grant { decision: GrantDecision::Allow, target: target.clone(), scope: *scope })
        })
        .collect();
    let denied = allows[0].clone();
    if persist {
        choices.push(Grant { decision: GrantDecision::Deny, target: denied.clone(), scope: GrantScope::Persisted });
    }
    choices.push(Grant { decision: GrantDecision::Deny, target: denied, scope: GrantScope::Once });
    choices
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    }
}

/// The folder an absolute glob's literal leading components name; `None`
/// for a relative pattern or one using `.` or `..`.
fn glob_folder(pattern: &str) -> Option<PathBuf> {
    let path = Path::new(pattern);
    if !path.is_absolute() {
        return None;
    }
    let mut folder = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir | Component::ParentDir => return None,
            Component::Normal(part) if part.to_str().is_some_and(|text| text.contains(['*', '?', '[', '{'])) => break,
            other => folder.push(other.as_os_str()),
        }
    }
    Some(folder)
}
