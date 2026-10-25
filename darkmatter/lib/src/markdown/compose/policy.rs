//! Which reported diagnostics make a composition fail, and the catalog of
//! codes a composition can report.
//!
//! A [`BlockingPolicy`] changes a command's outcome, never a diagnostic: a
//! blocked warning keeps its code, severity, and message, and the policy only
//! decides whether the command still succeeds. The `md` CLI builds one from
//! its `--deny` and `--allow` selections; the library consults it where a
//! diagnostic becomes known, so a finding of passive analysis stops the
//! document before any expression, shell, or effect runs.
//!
//! See `darkmatter/docs/cli/policy.md` for the command-line verbs.

use std::collections::BTreeSet;

use super::context::report::ComposeWarning;
use super::expression::Severity;
use crate::markdown::reference::types::ReferenceKind;

/// A missing hyperlink target, kept in the output under its recovery.
pub const MISSING_HYPERLINK_CODE: &str = "dm.reference.missing_hyperlink";
/// A missing image target, kept in the output under its recovery.
pub const MISSING_IMAGE_CODE: &str = "dm.reference.missing_image";
/// A missing transclusion target, removed from the output under its recovery.
pub const MISSING_TRANSCLUSION_CODE: &str = "dm.reference.missing_transclusion";
/// A non-object `ctx` frontmatter value replaced by the runtime context.
pub const CTX_OVERRIDE_CODE: &str = "dm.context.ctx_override";
/// An invalid `set=` assignment on a `::file` directive, skipped while its
/// valid siblings still apply.
pub const INVALID_FRONTMATTER_ASSIGNMENT_CODE: &str = "dm.transclusion.invalid_frontmatter_assignment";
/// A `::file` directive that assigns one property twice; the rightmost wins.
pub const REASSIGNED_FRONTMATTER_PROPERTY_CODE: &str = "dm.transclusion.reassigned_frontmatter_property";
/// A shell command that timed out and was replaced by an empty string.
pub const SHELL_TIMEOUT_CODE: &str = "dm.shell.timeout_recovered";

/// The code of a missing reference target of `kind`, when a recovery
/// governs that kind.
pub fn missing_reference_code(kind: ReferenceKind) -> Option<&'static str> {
    match kind {
        ReferenceKind::Hyperlink => Some(MISSING_HYPERLINK_CODE),
        ReferenceKind::Image => Some(MISSING_IMAGE_CODE),
        ReferenceKind::Transclusion => Some(MISSING_TRANSCLUSION_CODE),
        _ => None,
    }
}

/// Every stable code a composition reports, with what it reports.
///
/// The `md` CLI accepts exactly these as `--deny` codes on `md compose`. A
/// new coded warning family adds its code here.
pub const COMPOSE_DIAGNOSTIC_CODES: &[(&str, &str)] = &[
    (super::expression::INCOMPATIBLE_CALL_CODE, "a call proven wrong (always an error)"),
    (super::expression::UNCERTAIN_CALL_CODE, "a call argument that might not bind"),
    (super::expression::ALWAYS_FALSE_CALL_CODE, "a predicate call that is always false"),
    (super::expression::host::INCOMPLETE_ANALYSIS_CODE, "a host call the analysis cannot judge"),
    (ComposeWarning::UNDECLARED_PROPERTY_CODE, "a read of a property no schema declares"),
    (ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE, "a read of an unknown `ctx` group"),
    (ComposeWarning::EXPRESSION_PARSE_FAILURE_CODE, "an expression that did not parse"),
    (ComposeWarning::EXPRESSION_EVALUATION_FAILURE_CODE, "an expression that did not evaluate"),
    (ComposeWarning::GLOB_SKIPPED_SYMLINK_CODE, "a symlink a glob left out"),
    (super::RemoteFetchStats::CACHE_WARNING_CODE, "a remote cache read or write that failed"),
    (crate::markdown::schemas::SchemaAdvisory::MISSING_SIMPLIFIED_ENVELOPE_CODE, "a schema file without its `$schema` envelope"),
    (MISSING_HYPERLINK_CODE, "a missing hyperlink target"),
    (MISSING_IMAGE_CODE, "a missing image target"),
    (MISSING_TRANSCLUSION_CODE, "a missing transclusion target"),
    (CTX_OVERRIDE_CODE, "a non-object `ctx` replaced by the runtime context"),
    (INVALID_FRONTMATTER_ASSIGNMENT_CODE, "an invalid `set=` assignment on a `::file` directive"),
    (REASSIGNED_FRONTMATTER_PROPERTY_CODE, "a property a `::file` directive assigns twice"),
    (SHELL_TIMEOUT_CODE, "a shell command timeout replaced by an empty string"),
];

/// Which reported diagnostics block a composition.
///
/// The empty policy (the default) blocks nothing beyond the errors that
/// already fail a document.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::policy::{BlockingPolicy, SHELL_TIMEOUT_CODE};
/// use darkmatter::markdown::compose::expression::Severity;
///
/// // `--deny all --allow shell-timeout`
/// let policy = BlockingPolicy::default().deny_all().exempt_code(SHELL_TIMEOUT_CODE);
/// assert!(policy.blocks(None, Severity::Warning));
/// assert!(!policy.blocks(Some(SHELL_TIMEOUT_CODE), Severity::Warning));
/// assert!(!policy.blocks(None, Severity::Info));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockingPolicy {
    deny_all: bool,
    denied: BTreeSet<String>,
    exempt: BTreeSet<String>,
}

impl BlockingPolicy {
    /// Every reported warning and error blocks, except an exempt code.
    #[must_use]
    pub fn deny_all(mut self) -> Self {
        self.deny_all = true;
        self
    }

    /// A diagnostic with `code` blocks, whatever else the policy says.
    #[must_use]
    pub fn deny_code(mut self, code: impl Into<String>) -> Self {
        self.denied.insert(code.into());
        self
    }

    /// A diagnostic with `code` does not block under [`Self::deny_all`]: the
    /// code reports a recovery the caller selected by name.
    #[must_use]
    pub fn exempt_code(mut self, code: impl Into<String>) -> Self {
        self.exempt.insert(code.into());
        self
    }

    /// `true` when the policy blocks nothing.
    pub fn is_empty(&self) -> bool {
        !self.deny_all && self.denied.is_empty()
    }

    /// Whether a diagnostic with `code` and `severity` blocks. An
    /// informational diagnostic never does; an uncoded one blocks only under
    /// [`Self::deny_all`].
    pub fn blocks(&self, code: Option<&str>, severity: Severity) -> bool {
        if severity == Severity::Info {
            return false;
        }
        match code {
            Some(code) if self.denied.contains(code) => true,
            Some(code) if self.exempt.contains(code) => false,
            _ => self.deny_all,
        }
    }

    /// Whether `warning` blocks: its shared diagnostic's severity when it
    /// carries one, else a warning.
    pub fn blocks_warning(&self, warning: &ComposeWarning) -> bool {
        let severity = warning.diagnostic.as_deref().map_or(Severity::Warning, |diagnostic| diagnostic.severity);
        self.blocks(warning.code.as_deref(), severity)
    }

    /// The warnings of `warnings` that block, in order.
    pub fn blocking<'w>(&self, warnings: &'w [ComposeWarning]) -> Vec<&'w ComposeWarning> {
        if self.is_empty() {
            return Vec::new();
        }
        warnings.iter().filter(|warning| self.blocks_warning(warning)).collect()
    }

    /// A stable encoding of the policy, for option identity.
    pub(crate) fn identity(&self) -> String {
        let join = |codes: &BTreeSet<String>| codes.iter().map(String::as_str).collect::<Vec<_>>().join(",");
        format!("{}|{}|{}", self.deny_all, join(&self.denied), join(&self.exempt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_specific_denial_blocks_and_an_exemption_holds_only_under_deny_all() {
        let denied = BlockingPolicy::default().deny_code(SHELL_TIMEOUT_CODE);
        assert!(denied.blocks(Some(SHELL_TIMEOUT_CODE), Severity::Warning));
        assert!(!denied.blocks(Some(CTX_OVERRIDE_CODE), Severity::Warning));
        assert!(!denied.blocks(None, Severity::Warning));

        let all = BlockingPolicy::default().deny_all().exempt_code(CTX_OVERRIDE_CODE);
        assert!(all.blocks(Some(SHELL_TIMEOUT_CODE), Severity::Warning));
        assert!(all.blocks(Some(SHELL_TIMEOUT_CODE), Severity::Error));
        assert!(!all.blocks(Some(CTX_OVERRIDE_CODE), Severity::Warning));
        assert!(!all.blocks(Some(SHELL_TIMEOUT_CODE), Severity::Info));
    }

    #[test]
    fn the_default_policy_blocks_nothing() {
        let policy = BlockingPolicy::default();
        assert!(policy.is_empty());
        assert!(!policy.blocks(Some(SHELL_TIMEOUT_CODE), Severity::Error));
        assert!(policy.blocking(&[ComposeWarning::new("stage", "message")]).is_empty());
    }

    #[test]
    fn the_catalog_lists_each_code_once() {
        let mut codes: Vec<&str> = COMPOSE_DIAGNOSTIC_CODES.iter().map(|(code, _)| *code).collect();
        let count = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), count);
        assert!(codes.iter().all(|code| code.starts_with("dm.")));
    }
}
