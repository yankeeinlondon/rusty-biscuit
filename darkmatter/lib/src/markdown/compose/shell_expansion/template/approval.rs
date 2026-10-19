//! Structured approvals: what a caller hands composition as pre-approved, and
//! the mode-tagged keys a session caches decisions under.
//!
//! An approval is either for **resolved bytes** (one normalized command) or
//! for a **template** (a [`TemplateShape`] whose holes keep their approval
//! types). The two never authorize each other: an exact approval cannot cover
//! a template-mode command, and a shape approval is never turned into the
//! bytes some fill of it produced.

use super::{CommandTemplate, TemplateShape};
use crate::markdown::compose::shell_expansion::types::ShellPolicyPaths;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The approvals a caller computed at preflight, handed back to composition
/// through `ComposeOptions::with_pre_approved_commands`.
///
/// Execution must prove a command's membership: its freshly resolved bytes in
/// [`exact`](Self::contains_exact) for a resolved-byte site, or its re-parsed,
/// re-typed authored shape in [`templates`](Self::contains_template). Nothing
/// at execution inserts into either set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellPreApprovals {
    exact: HashSet<String>,
    templates: HashSet<TemplateShape>,
}

impl ShellPreApprovals {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_exact(&mut self, normalized: impl Into<String>) -> bool {
        self.exact.insert(normalized.into())
    }

    pub fn insert_template(&mut self, shape: TemplateShape) -> bool {
        self.templates.insert(shape)
    }

    pub fn contains_exact(&self, normalized: &str) -> bool {
        self.exact.contains(normalized)
    }

    /// Whether `shape` was approved. Shapes compare structurally, approval
    /// types included, so a changed declared type is not a member.
    pub fn contains_template(&self, shape: &TemplateShape) -> bool {
        self.templates.contains(shape)
    }

    /// Adds every approval in `other`.
    pub fn extend(&mut self, other: ShellPreApprovals) {
        self.exact.extend(other.exact);
        self.templates.extend(other.templates);
    }

    /// Adds resolved-byte approvals.
    pub fn extend_exact(&mut self, normalized: impl IntoIterator<Item = String>) {
        self.exact.extend(normalized);
    }

    pub fn exact(&self) -> impl Iterator<Item = &String> {
        self.exact.iter()
    }

    pub fn templates(&self) -> impl Iterator<Item = &TemplateShape> {
        self.templates.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.templates.is_empty()
    }

    /// The number of approvals of both modes.
    pub fn len(&self) -> usize {
        self.exact.len() + self.templates.len()
    }
}

impl From<HashSet<String>> for ShellPreApprovals {
    /// Resolved-byte approvals only.
    fn from(exact: HashSet<String>) -> Self {
        Self {
            exact,
            templates: HashSet::new(),
        }
    }
}

impl FromIterator<String> for ShellPreApprovals {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self::from(iter.into_iter().collect::<HashSet<_>>())
    }
}

/// A session-cache key for one approval decision, tagged with its execution
/// mode and scoped to the policy root whose files the decision was judged
/// against.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApprovalKey {
    /// Resolved bytes: one normalized command.
    Exact { policy_root: Option<PathBuf>, normalized: String },
    /// A template shape with its approval types.
    Template { policy_root: Option<PathBuf>, shape: TemplateShape },
}

impl ApprovalKey {
    pub fn exact(policy_root: Option<&Path>, normalized: impl Into<String>) -> Self {
        Self::Exact {
            policy_root: policy_root.map(Path::to_path_buf),
            normalized: normalized.into(),
        }
    }

    pub fn template(policy_root: Option<&Path>, shape: TemplateShape) -> Self {
        Self::Template {
            policy_root: policy_root.map(Path::to_path_buf),
            shape,
        }
    }

    /// The policy root of `paths`: the directory holding the whitelist.
    pub fn policy_root_of(paths: &ShellPolicyPaths) -> Option<&Path> {
        paths.whitelist.parent()
    }

    /// The key's command text: the normalized bytes, or the shape's canonical
    /// rule text.
    pub fn command_text(&self) -> String {
        match self {
            Self::Exact { normalized, .. } => normalized.clone(),
            Self::Template { shape, .. } => shape.canonical_text(),
        }
    }
}

/// Which approval choices a prompt may offer for one candidate.
///
/// Once, deny, and blacklist are offered whenever the candidate validated, so
/// they are not listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfferedChoices {
    /// "This exact command": the candidate's bytes are known and resolved-byte
    /// execution is eligible.
    pub exact: bool,
    /// "This command shape": the authored command has a placeholder and valid
    /// template grammar.
    pub shape: bool,
    /// "This program with any arguments": every program word is fixed.
    pub prefix: bool,
}

impl OfferedChoices {
    /// The choices of a resolved-byte request with no template: exact and
    /// prefix.
    pub fn resolved_bytes() -> Self {
        Self {
            exact: true,
            shape: false,
            prefix: true,
        }
    }
}

impl Default for OfferedChoices {
    fn default() -> Self {
        Self::resolved_bytes()
    }
}

/// The template half of an approval request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateApprovalView {
    /// The shape the "this command shape" choice persists.
    pub shape: TemplateShape,
    /// The authored command with each hole as `{{ expression: type }}`.
    pub display: String,
}

impl TemplateApprovalView {
    pub fn of(template: &CommandTemplate) -> Self {
        Self {
            shape: TemplateShape::from_template(template),
            display: template.display_text(),
        }
    }
}
