//! Map a synthesized session `error_kind` label to a locked diagnostic code.
//!
//! Provider, cap, timeout, and runaway failures never reach the lifecycle `err`
//! global as a typed [`ClaudineError`]/[`HarnessError`]: they are surfaced
//! through the synthesized `session_end` summary's honest per-guard
//! `error_kind` string (e.g. `step_timeout`, `runaway_repetition`,
//! `rate_limit`) and carried into the lifecycle stack via
//! [`LifecycleErrorInfo::from_action_failure`]. This module is the seam that
//! lets that string still project the full [`Diagnostic`] facet surface
//! (`err.code` / `err.category` / `err.disposition` / `err.origin`) without a
//! detour through a typed error.
//!
//! The cap/auth half reuses the operator badge classifier
//! ([`badge_category_for_kind`]) so a kind classifies the same way for handling
//! as it does for the human badge.
//!
//! [`ClaudineError`]: crate::error::ClaudineError
//! [`HarnessError`]: crate::harness::error::HarnessError
//! [`LifecycleErrorInfo::from_action_failure`]: crate::composition::lifecycle_context::LifecycleErrorInfo::from_action_failure
//! [`Diagnostic`]: super::Diagnostic

use crate::stream::badges::{BadgeCategory, badge_category_for_kind};

/// Resolve a session `error_kind` label to its locked dotted code, if known.
///
/// Returns `None` for a label that is not a recognized provider/cap/timeout/
/// runaway condition — a generic lifecycle action verb (`shell`,
/// `set_frontmatter`) is not an error_kind and stays facet-less.
pub fn code_for_error_kind(kind: &str) -> Option<&'static str> {
    // Cap / auth conditions reuse the badge classifier and the ratified
    // BadgeCategory → code folding (error-catalog §5).
    if let Some(badge) = badge_category_for_kind(kind) {
        return Some(match badge {
            BadgeCategory::Auth => "auth.invalid",
            BadgeCategory::Permission => "auth.permission",
            BadgeCategory::Billing => "cap.billing",
            BadgeCategory::Quota => "cap.plan_limit",
            BadgeCategory::RateLimit => "cap.rate_limit",
            BadgeCategory::ContextPressure => "provider.context_pressure",
            BadgeCategory::Config => "config.invalid",
        });
    }
    // Timeout / runaway / provider guard labels (the synthesized
    // `EarlyTermination::error_kind` vocabulary plus the wrapper's fallbacks).
    let code = match kind {
        "timeout" => "timeout.wall_clock",
        "step_timeout" | "step_silence" | "stalled_generation" => "timeout.step_silence",
        "exit_expression" => "runaway.exit_expression",
        "runaway_repetition" => "runaway.repetition",
        "runaway_volume" => "runaway.volume",
        "repeated_stream_error" => "provider.stream_error",
        "context_pressure" => "provider.context_pressure",
        "interrupted" => "provider.interrupted",
        "launch_failed" => "provider.launch_failed",
        "agent_failure" => "provider.exited",
        _ => return None,
    };
    Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Category, code_spec};

    #[test]
    fn timeout_kinds_map_to_timeout_codes() {
        assert_eq!(code_for_error_kind("timeout"), Some("timeout.wall_clock"));
        assert_eq!(
            code_for_error_kind("step_timeout"),
            Some("timeout.step_silence")
        );
        assert_eq!(
            code_for_error_kind("stalled_generation"),
            Some("timeout.step_silence")
        );
    }

    #[test]
    fn runaway_kinds_map_to_runaway_codes() {
        assert_eq!(
            code_for_error_kind("runaway_repetition"),
            Some("runaway.repetition")
        );
        assert_eq!(
            code_for_error_kind("runaway_volume"),
            Some("runaway.volume")
        );
        assert_eq!(
            code_for_error_kind("exit_expression"),
            Some("runaway.exit_expression")
        );
    }

    #[test]
    fn cap_kinds_map_to_cap_codes() {
        assert_eq!(code_for_error_kind("rate_limit"), Some("cap.rate_limit"));
        assert_eq!(code_for_error_kind("billing_error"), Some("cap.billing"));
        assert_eq!(code_for_error_kind("quota_exceeded"), Some("cap.plan_limit"));
    }

    #[test]
    fn provider_kinds_map_to_provider_codes() {
        assert_eq!(code_for_error_kind("agent_failure"), Some("provider.exited"));
        assert_eq!(
            code_for_error_kind("repeated_stream_error"),
            Some("provider.stream_error")
        );
    }

    #[test]
    fn unknown_kind_is_unclassified() {
        assert_eq!(code_for_error_kind("shell"), None);
        assert_eq!(code_for_error_kind("set_frontmatter"), None);
    }

    #[test]
    fn every_mapped_code_is_in_the_locked_catalog() {
        // Every code this maps to must be a real catalog row, or the facet
        // projection would carry a code `claudine errors` cannot describe.
        for kind in [
            "timeout",
            "step_timeout",
            "stalled_generation",
            "exit_expression",
            "runaway_repetition",
            "runaway_volume",
            "repeated_stream_error",
            "interrupted",
            "launch_failed",
            "agent_failure",
            "rate_limit",
            "billing_error",
            "quota_exceeded",
            "unauthorized",
            "forbidden",
        ] {
            let code = code_for_error_kind(kind)
                .unwrap_or_else(|| panic!("`{kind}` should classify"));
            assert!(
                code_spec(code).is_some(),
                "`{kind}` → `{code}` is not a locked catalog code"
            );
        }
    }

    #[test]
    fn cap_and_runaway_land_in_expected_categories() {
        let cap = code_spec(code_for_error_kind("rate_limit").unwrap()).unwrap();
        assert_eq!(cap.category, Category::Cap);
        let runaway = code_spec(code_for_error_kind("runaway_volume").unwrap()).unwrap();
        assert_eq!(runaway.category, Category::Runaway);
    }
}
