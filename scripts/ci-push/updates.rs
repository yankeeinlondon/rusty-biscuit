//! The ref updates Git hands a pre-push hook on stdin, one
//! `<local ref> <local oid> <remote ref> <remote oid>` line each.
//!
//! The whole list is read and validated before any update is reviewed: one
//! update can change another's pull-request base, and a malformed line must
//! not leave the push half reviewed.

use std::collections::HashMap;
use std::fmt;

use gix::ObjectId;
use gix::hash::Kind;

/// One ref update, in the order Git supplied it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdate {
    /// 1-based line number in the hook's input.
    pub line: usize,
    /// What was pushed, as Git names it: a full ref, `HEAD`, a revision
    /// expression, or `(delete)`.
    pub local_ref: String,
    pub local_oid: ObjectId,
    pub remote_ref: String,
    /// The destination's current tip on the remote; null when Git believes
    /// the push creates it.
    pub remote_oid: ObjectId,
}

/// Why an update triggers no run and is not reviewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Deletion,
    /// The destination is a tag, note, or other ref outside `refs/heads/`.
    NotABranch,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            SkipReason::Deletion => "a deletion triggers no run",
            SkipReason::NotABranch => "only a branch update triggers a run",
        })
    }
}

/// The GitHub event an update triggers, which decides the plan's environments
/// and the comparison base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerEvent {
    /// A push to `main`: compared against the remote's previous tip.
    Push,
    /// Any other branch: one run per open pull request from it.
    PullRequest,
}

impl TriggerEvent {
    /// The planner's `--event` spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            TriggerEvent::Push => "push",
            TriggerEvent::PullRequest => "pull_request",
        }
    }
}

impl RefUpdate {
    /// `None` when the update is reviewed.
    pub fn skip_reason(&self) -> Option<SkipReason> {
        if self.local_oid.is_null() {
            Some(SkipReason::Deletion)
        } else if self.remote_branch().is_none() {
            Some(SkipReason::NotABranch)
        } else {
            None
        }
    }

    pub fn is_reviewed(&self) -> bool {
        self.skip_reason().is_none()
    }

    /// The branch the update writes on the remote.
    pub fn remote_branch(&self) -> Option<&str> {
        self.remote_ref.strip_prefix("refs/heads/")
    }

    /// The local branch pushed, when the source is a branch at all.
    pub fn local_branch(&self) -> Option<&str> {
        self.local_ref.strip_prefix("refs/heads/")
    }

    pub fn event(&self) -> TriggerEvent {
        if self.remote_ref == "refs/heads/main" {
            TriggerEvent::Push
        } else {
            TriggerEvent::PullRequest
        }
    }

    /// `<local ref> -> <remote ref>`, as diagnostics name an update.
    pub fn label(&self) -> String {
        format!("{} -> {}", self.local_ref, self.remote_ref)
    }
}

/// A ref-update line that blocks the push before anything is reviewed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputError {
    pub line: usize,
    /// The line as given, lossily decoded and without its line ending.
    pub text: String,
    pub problem: String,
}

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ref update line {} {}: {:?}",
            self.line, self.problem, self.text
        )
    }
}

impl std::error::Error for InputError {}

/// Parses the hook's complete input for a repository using `hash`.
///
/// Lines end in LF or CRLF, the last may lack one, and blank or
/// whitespace-only lines are ignored. Two updates of one destination are
/// malformed: Git never sends them, and reviewing either alone would be wrong.
pub fn parse_updates(input: &[u8], hash: Kind) -> Result<Vec<RefUpdate>, InputError> {
    let mut updates: Vec<RefUpdate> = Vec::new();
    let mut destinations: HashMap<String, usize> = HashMap::new();
    for (index, raw) in input.split(|byte| *byte == b'\n').enumerate() {
        let line = index + 1;
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        let malformed = |problem: String| InputError {
            line,
            text: String::from_utf8_lossy(raw).into_owned(),
            problem,
        };
        let text = std::str::from_utf8(raw).map_err(|_| malformed("is not valid UTF-8".into()))?;
        let tokens: Vec<&str> = text.split_ascii_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let [local_ref, local_oid, remote_ref, remote_oid] = tokens[..] else {
            return Err(malformed(format!(
                "has {} fields; expected <local ref> <local oid> <remote ref> <remote oid>",
                tokens.len()
            )));
        };
        let local_oid = parse_oid(local_oid, hash).map_err(|problem| malformed(format!("local oid {problem}")))?;
        let remote_oid = parse_oid(remote_oid, hash).map_err(|problem| malformed(format!("remote oid {problem}")))?;
        if !remote_ref.starts_with("refs/") || gix::refs::FullName::try_from(remote_ref).is_err() {
            return Err(malformed(format!("remote ref {remote_ref:?} is not a full ref name")));
        }
        if let Some(first) = destinations.insert(remote_ref.to_owned(), line) {
            return Err(malformed(format!("updates {remote_ref} again (line {first} already updates it)")));
        }
        updates.push(RefUpdate {
            line,
            local_ref: local_ref.to_owned(),
            local_oid,
            remote_ref: remote_ref.to_owned(),
            remote_oid,
        });
    }
    Ok(updates)
}

fn parse_oid(text: &str, hash: Kind) -> Result<ObjectId, String> {
    let expected = hash.len_in_hex();
    if text.len() != expected || !text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(format!(
            "{text:?} is not {expected} lowercase hex digits ({hash} object ids)"
        ));
    }
    ObjectId::from_hex(text.as_bytes()).map_err(|error| format!("{text:?} is not an object id: {error}"))
}
