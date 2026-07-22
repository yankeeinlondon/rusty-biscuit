//! The four closed facet enums of the diagnostic contract.
//!
//! These are the ratified, locked sets from
//! `claudine/features/_completed/2026-06-28-real-errors/error-catalog.md` §1 — 12
//! categories, 5 dispositions, 5 origins, and the operator-facing severity
//! (reusing [`BadgeSeverity`] as [`Severity`]). Each enum carries a stable
//! `snake_case` string projection: it is what an author matches on in a
//! `when:` clause, so the strings are a versioned public contract and may
//! only evolve additively (new variants are non-breaking; renames are
//! breaking).
//!
//! The `as_str` methods mirror the stability discipline already used by
//! [`crate::stream::semantic::SemanticErrorKind::as_str`] and exercised by its
//! stability test, so a string projection cannot silently drift.

use serde::{Deserialize, Serialize};

pub use crate::stream::badges::BadgeSeverity as Severity;

/// Coarse problem domain — the dotted prefix of every [`code`](super::Diagnostic::code).
///
/// `err.category == "cap"` is exactly `err.code` starting with `"cap."`, so a
/// category is always derivable from a code. Category is pure *domain*; it
/// does **not** imply a strategy (a `cap` may be `throttled` *or*
/// `correctable`). Strategy lives in [`Disposition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Identity / authorization with the provider.
    Auth,
    /// Usage and account limits (rate limits, plan/usage quotas, billing).
    Cap,
    /// The run exceeded a time budget (stream silence or wall clock).
    Timeout,
    /// The agent run itself — launch, execution, streaming, model behavior.
    Provider,
    /// The author's prompt document is malformed (Darkmatter compose/expression/schema).
    Composition,
    /// A document the agent produced or modified fails a postcondition.
    Document,
    /// Git-state postconditions about the agent's work.
    Vcs,
    /// Filesystem and network plumbing beneath everything else.
    Io,
    /// Claudine/user configuration is invalid (settings, not a prompt doc).
    Config,
    /// Misuse of the Rust API by an integrating caller.
    Usage,
    /// A content-guard tripped — the child flooded rather than failed.
    Runaway,
    /// A bug or unexpected invariant in Darkmatter/Claudine itself.
    Internal,
}

impl Category {
    /// Stable snake_case identifier — the dotted code prefix.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Auth => "auth",
            Category::Cap => "cap",
            Category::Timeout => "timeout",
            Category::Provider => "provider",
            Category::Composition => "composition",
            Category::Document => "document",
            Category::Vcs => "vcs",
            Category::Io => "io",
            Category::Config => "config",
            Category::Usage => "usage",
            Category::Runaway => "runaway",
            Category::Internal => "internal",
        }
    }

    /// Every category, in declaration order. Useful for introspection surfaces
    /// (`claudine errors`) and exhaustiveness tests.
    pub const ALL: &'static [Category] = &[
        Category::Auth,
        Category::Cap,
        Category::Timeout,
        Category::Provider,
        Category::Composition,
        Category::Document,
        Category::Vcs,
        Category::Io,
        Category::Config,
        Category::Usage,
        Category::Runaway,
        Category::Internal,
    ];
}

/// What class of response could resolve the error — the reuse-enabling facet.
///
/// This subsumes the integrated design's `is_authoring_fatal()`: "fatal in
/// lenient compose mode" is just a disposition that halts composition. A
/// handler that says "retry `Transient`, wait-and-retry `Throttled`, surface
/// the rest" is reusable across every domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// The same action may succeed if retried now.
    Transient,
    /// Will succeed *later*, at a known/estimable time (wait for `reset_at`).
    Throttled,
    /// Needs a *different* action; won't self-resolve.
    Correctable,
    /// Needs human input to decide the fix (defer to rendezvous when not interactive).
    NeedsInput,
    /// No generic resolution — stop and surface.
    Unrecoverable,
}

impl Disposition {
    /// Stable snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Disposition::Transient => "transient",
            Disposition::Throttled => "throttled",
            Disposition::Correctable => "correctable",
            Disposition::NeedsInput => "needs_input",
            Disposition::Unrecoverable => "unrecoverable",
        }
    }

    /// Every disposition, in declaration order.
    pub const ALL: &'static [Disposition] = &[
        Disposition::Transient,
        Disposition::Throttled,
        Disposition::Correctable,
        Disposition::NeedsInput,
        Disposition::Unrecoverable,
    ];

    /// The default operator-facing severity for this disposition.
    ///
    /// Per error-catalog §1: `transient`/`throttled` → `warning`,
    /// `unrecoverable`/`correctable` → `error`, `needs_input` → `warning`.
    /// A code may override this default.
    pub fn default_severity(self) -> Severity {
        match self {
            Disposition::Transient | Disposition::Throttled | Disposition::NeedsInput => {
                Severity::Warning
            }
            Disposition::Correctable | Disposition::Unrecoverable => Severity::Error,
        }
    }
}

/// Who must remediate the error.
///
/// `disposition × origin` is the strategy matrix that makes handler patterns
/// reusable: "re-prompt the agent on any `correctable` + `provider`", "notify
/// the author on any `author` error". There is deliberately no separate
/// `agent` value — the model's behavior is folded into `provider`, with
/// disposition carrying the infra-vs-behavior distinction (error-catalog §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// The agent run — platform, API, and model behavior.
    Provider,
    /// The prompt-document author.
    Author,
    /// The Rust API integrator.
    Caller,
    /// Host / filesystem / network / credentials.
    Environment,
    /// A bug in Darkmatter/Claudine.
    Internal,
}

impl Origin {
    /// Stable snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Provider => "provider",
            Origin::Author => "author",
            Origin::Caller => "caller",
            Origin::Environment => "environment",
            Origin::Internal => "internal",
        }
    }

    /// Every origin, in declaration order.
    pub const ALL: &'static [Origin] = &[
        Origin::Provider,
        Origin::Author,
        Origin::Caller,
        Origin::Environment,
        Origin::Internal,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_count_is_twelve() {
        assert_eq!(Category::ALL.len(), 12);
    }

    #[test]
    fn disposition_count_is_five() {
        assert_eq!(Disposition::ALL.len(), 5);
    }

    #[test]
    fn origin_count_is_five() {
        assert_eq!(Origin::ALL.len(), 5);
    }

    #[test]
    fn category_as_str_matches_serde() {
        for &cat in Category::ALL {
            let json = serde_json::to_string(&cat).unwrap();
            assert_eq!(json, format!("\"{}\"", cat.as_str()));
        }
    }

    #[test]
    fn disposition_as_str_matches_serde() {
        for &disp in Disposition::ALL {
            let json = serde_json::to_string(&disp).unwrap();
            assert_eq!(json, format!("\"{}\"", disp.as_str()));
        }
    }

    #[test]
    fn origin_as_str_matches_serde() {
        for &origin in Origin::ALL {
            let json = serde_json::to_string(&origin).unwrap();
            assert_eq!(json, format!("\"{}\"", origin.as_str()));
        }
    }

    #[test]
    fn severity_as_str_matches_serde() {
        // The `err.severity` facet projects this string; lock it to the wire
        // form so the projection cannot drift.
        for &severity in &[Severity::Info, Severity::Warning, Severity::Error] {
            let json = serde_json::to_string(&severity).unwrap();
            assert_eq!(json, format!("\"{}\"", severity.as_str()));
        }
    }

    #[test]
    fn category_as_str_is_stable() {
        // Locking the contract strings — a rename is a breaking change and
        // must fail this test deliberately, mirroring
        // `error_kind_as_str_is_stable` in stream/semantic.rs.
        assert_eq!(Category::Auth.as_str(), "auth");
        assert_eq!(Category::Cap.as_str(), "cap");
        assert_eq!(Category::Timeout.as_str(), "timeout");
        assert_eq!(Category::Provider.as_str(), "provider");
        assert_eq!(Category::Composition.as_str(), "composition");
        assert_eq!(Category::Document.as_str(), "document");
        assert_eq!(Category::Vcs.as_str(), "vcs");
        assert_eq!(Category::Io.as_str(), "io");
        assert_eq!(Category::Config.as_str(), "config");
        assert_eq!(Category::Usage.as_str(), "usage");
        assert_eq!(Category::Runaway.as_str(), "runaway");
        assert_eq!(Category::Internal.as_str(), "internal");
    }

    #[test]
    fn disposition_default_severity_follows_catalog() {
        assert_eq!(Disposition::Transient.default_severity(), Severity::Warning);
        assert_eq!(Disposition::Throttled.default_severity(), Severity::Warning);
        assert_eq!(Disposition::NeedsInput.default_severity(), Severity::Warning);
        assert_eq!(Disposition::Correctable.default_severity(), Severity::Error);
        assert_eq!(
            Disposition::Unrecoverable.default_severity(),
            Severity::Error
        );
    }

    #[test]
    fn needs_input_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&Disposition::NeedsInput).unwrap(),
            "\"needs_input\""
        );
    }
}
