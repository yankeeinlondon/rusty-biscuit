//! Frontmatter-to-plan parser for harness configuration.
//!
//! Accepts timeout-related keys and builds a typed [`HarnessPlan`].

use std::path::Path;

use serde_json::Value;
use tracing::debug;

use crate::harness::error::HarnessError;
use crate::harness::model::HarnessPlan;
use crate::harness::timeout::{format_duration, parse_timeout};

/// Harness-relevant frontmatter keys.
const HARNESS_KEYS: &[&str] = &[
    "timeout",
    "step_timeout",
    "timeout_warn",
    "step_timeout_warn",
];

/// Check whether composed frontmatter contains any harness-relevant keys.
///
/// Returns `true` if any of `timeout`, `step_timeout`, `timeout_warn`, or
/// `step_timeout_warn` is present.
pub fn has_harness_properties(frontmatter: &Value) -> bool {
    let Some(obj) = frontmatter.as_object() else {
        return false;
    };
    obj.keys().any(|key| HARNESS_KEYS.contains(&key.as_str()))
}

/// Parse composed frontmatter into a [`HarnessPlan`].
///
/// ## Errors
///
/// Returns [`HarnessError`] for any structural or semantic issue found at parse
/// time, including invalid timeout strings.
pub fn parse_harness_plan(
    frontmatter: &Value,
    source_path: &Path,
) -> Result<HarnessPlan, HarnessError> {
    let obj = frontmatter
        .as_object()
        .ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "(root)".to_string(),
            detail: "frontmatter must be an object".to_string(),
        })?;

    // Parse timeout
    let timeout = if let Some(v) = obj.get("timeout") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "timeout".to_string(),
            detail: "timeout must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Parse step_timeout (silence deadline). Same syntax as `timeout`.
    let step_timeout = if let Some(v) = obj.get("step_timeout") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "step_timeout".to_string(),
            detail: "step_timeout must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Relational validation: step_timeout must not exceed the wall-clock
    // timeout when both are present. A step budget greater than the wall
    // clock is always unreachable.
    if let (Some(step), Some(total)) = (step_timeout, timeout)
        && step > total
    {
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: format_duration(step),
            detail: format!(
                "step_timeout ({}) must not exceed timeout ({})",
                format_duration(step),
                format_duration(total),
            ),
        });
    }

    // Parse timeout_warn (wall-clock warning threshold). Same syntax as
    // `timeout`.
    let timeout_warn = if let Some(v) = obj.get("timeout_warn") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "timeout_warn".to_string(),
            detail: "timeout_warn must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Parse step_timeout_warn (silence warning threshold). Same syntax as
    // `step_timeout`.
    let step_timeout_warn = if let Some(v) = obj.get("step_timeout_warn") {
        let raw = v.as_str().ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "step_timeout_warn".to_string(),
            detail: "step_timeout_warn must be a string (e.g. \"30s\", \"5m\")".to_string(),
        })?;
        Some(parse_timeout(raw, source_path)?)
    } else {
        None
    };

    // Relational validation: each `*_warn` must be strictly less than its
    // corresponding hard threshold when both are present. A warn equal
    // to or greater than the hard timeout would never fire before the
    // kill signal, defeating the purpose of an early warning. Rejection
    // is a preflight hard error and emits a `tracing::error!` adjacent
    // to the user-facing message per the timing feature spec.
    if let (Some(warn), Some(hard)) = (timeout_warn, timeout)
        && warn >= hard
    {
        tracing::error!(
            source_path = %source_path.display(),
            timeout_warn_secs = warn.as_secs(),
            timeout_secs = hard.as_secs(),
            "preflight rejection: timeout_warn must be less than timeout",
        );
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: format_duration(warn),
            detail: format!(
                "timeout_warn ({}) must be less than timeout ({})",
                format_duration(warn),
                format_duration(hard),
            ),
        });
    }
    if let (Some(warn), Some(hard)) = (step_timeout_warn, step_timeout)
        && warn >= hard
    {
        tracing::error!(
            source_path = %source_path.display(),
            step_timeout_warn_secs = warn.as_secs(),
            step_timeout_secs = hard.as_secs(),
            "preflight rejection: step_timeout_warn must be less than step_timeout",
        );
        return Err(HarnessError::InvalidTimeout {
            source_path: source_path.to_path_buf(),
            raw: format_duration(warn),
            detail: format!(
                "step_timeout_warn ({}) must be less than step_timeout ({})",
                format_duration(warn),
                format_duration(hard),
            ),
        });
    }

    let plan = HarnessPlan {
        source_path: source_path.to_path_buf(),
        timeout,
        step_timeout,
        timeout_warn,
        step_timeout_warn,
    };
    debug!(
        source = %source_path.display(),
        timeout_secs = plan.timeout.map(|d| d.as_secs()),
        "parsed harness plan",
    );
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::*;

    fn test_source_path() -> PathBuf {
        PathBuf::from("/tmp/test-prompt.md")
    }

    fn bare_plan(source_path: &Path) -> HarnessPlan {
        HarnessPlan {
            source_path: source_path.to_path_buf(),
            timeout: None,
            step_timeout: None,
            timeout_warn: None,
            step_timeout_warn: None,
        }
    }

    #[test]
    fn direct_bare_plan_unchanged() {
        let source = test_source_path();
        let plan = bare_plan(&source);

        assert_eq!(plan.timeout, None);
        assert_eq!(plan.step_timeout, None);
        assert_eq!(plan.timeout_warn, None);
        assert_eq!(plan.step_timeout_warn, None);
    }

    #[test]
    fn stall_timeout_is_not_a_frontmatter_property() {
        let source = test_source_path();
        let frontmatter_value = json!({ "stall_timeout": "1s" });

        assert!(!has_harness_properties(&frontmatter_value));
        let plan = parse_harness_plan(&frontmatter_value, &source).unwrap();
        assert_eq!(plan.timeout, None);
        assert_eq!(plan.step_timeout, None);
    }

    #[test]
    fn parses_timeouts() {
        let source = test_source_path();
        let frontmatter_value = json!({
            "timeout": "5m",
            "step_timeout": "30s",
            "timeout_warn": "1m",
            "step_timeout_warn": "10s",
        });

        let plan = parse_harness_plan(&frontmatter_value, &source)
            .expect("timeouts should parse");
        assert_eq!(plan.timeout, Some(std::time::Duration::from_secs(300)));
        assert_eq!(plan.step_timeout, Some(std::time::Duration::from_secs(30)));
        assert_eq!(plan.timeout_warn, Some(std::time::Duration::from_secs(60)));
        assert_eq!(plan.step_timeout_warn, Some(std::time::Duration::from_secs(10)));
    }

    #[test]
    fn rejects_step_timeout_greater_than_timeout() {
        let source = test_source_path();
        let frontmatter_value = json!({
            "timeout": "30s",
            "step_timeout": "1m",
        });

        let err = parse_harness_plan(&frontmatter_value, &source).unwrap_err();
        assert!(
            matches!(err, HarnessError::InvalidTimeout { .. }),
            "expected InvalidTimeout, got {err:?}"
        );
    }

    #[test]
    fn rejects_warn_equal_to_hard_timeout() {
        let source = test_source_path();
        let frontmatter_value = json!({
            "timeout": "30s",
            "timeout_warn": "30s",
        });

        let err = parse_harness_plan(&frontmatter_value, &source).unwrap_err();
        assert!(
            matches!(err, HarnessError::InvalidTimeout { .. }),
            "expected InvalidTimeout, got {err:?}"
        );
    }
}
