//! Frontmatter-to-plan parser for harness configuration.
//!
//! Accepts `pre_checks` and `post_checks` in both list form (canonical) and
//! map form (shorthand), parses handler declarations, and builds a typed
//! [`HarnessPlan`].

use std::path::Path;

use serde_json::Value;
use tracing::debug;

use self::frontmatter::extract_frontmatter_text;
use self::handlers::parse_handlers;
use self::span::find_rule_spans;
use self::validations::parse_checks;
use crate::harness::error::HarnessError;
use crate::harness::model::{
    HarnessPlan, ValidationEvent, ValidationKind, ValidationPhase, ValidationRule, ValidationRuleId,
};
use crate::harness::resolve::HarnessResolutionContext;
use crate::harness::timeout::{format_duration, parse_timeout};

mod frontmatter;
mod handlers;
mod overlays;
mod shapes;
mod span;
mod validations;

/// Harness-relevant frontmatter keys.
const HARNESS_KEYS: &[&str] = &[
    "pre_checks",
    "post_checks",
    "timeout",
    "step_timeout",
    "timeout_warn",
    "step_timeout_warn",
    "handle",
];

/// Check whether composed frontmatter contains any harness-relevant keys.
///
/// Returns `true` if any of `pre_checks`, `post_checks`, `timeout`,
/// `step_timeout`, `timeout_warn`, `step_timeout_warn`, `handle`, or any
/// `handle_*` key is present.
pub fn has_harness_properties(frontmatter: &Value) -> bool {
    let Some(obj) = frontmatter.as_object() else {
        return false;
    };
    for key in obj.keys() {
        if HARNESS_KEYS.contains(&key.as_str()) || key.starts_with("handle_") {
            return true;
        }
    }
    false
}

/// Parse composed frontmatter into a [`HarnessPlan`].
///
/// ## Errors
///
/// Returns [`HarnessError`] for any structural or semantic issue found at parse
/// time, including unknown validation names, post-only validations in
/// `pre_checks`, invalid timeout strings, and missing handler fields.
pub fn parse_harness_plan(
    frontmatter: &Value,
    source_path: &Path,
    ctx: &HarnessResolutionContext<'_>,
) -> Result<HarnessPlan, HarnessError> {
    let obj = frontmatter
        .as_object()
        .ok_or_else(|| HarnessError::InvalidFrontmatter {
            source_path: source_path.to_path_buf(),
            property: "(root)".to_string(),
            detail: "frontmatter must be an object".to_string(),
        })?;

    let mut next_id: u32 = 0;
    let mut alloc_id = || {
        let id = ValidationRuleId(next_id);
        next_id += 1;
        id
    };

    // Best-effort source-span recovery. Read the markdown from disk and
    // extract the YAML frontmatter slice; on any IO or framing failure
    // fall back to an empty `SpanIndex` so `line_range` stays `None` and
    // `yaml_snippet` falls back to reconstructed YAML.
    let raw_source = std::fs::read_to_string(source_path).ok();
    let frontmatter_slice: Option<(&str, usize)> =
        raw_source.as_deref().and_then(extract_frontmatter_text);
    let span_index = frontmatter_slice
        .map(|(text, base_line)| find_rule_spans(text, base_line))
        .unwrap_or_default();

    // Parse pre_checks
    let pre_checks = if let Some(v) = obj.get("pre_checks") {
        parse_checks(
            v,
            true,
            source_path,
            ctx,
            &span_index,
            frontmatter_slice,
            &mut alloc_id,
        )?
    } else {
        Vec::new()
    };

    // Parse post_checks
    let post_checks = if let Some(v) = obj.get("post_checks") {
        parse_checks(
            v,
            false,
            source_path,
            ctx,
            &span_index,
            frontmatter_slice,
            &mut alloc_id,
        )?
    } else {
        Vec::new()
    };

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

    // Parse handlers
    let (handlers, programmatic_handler) = parse_handlers(obj, source_path, ctx)?;

    let plan = HarnessPlan {
        source_path: source_path.to_path_buf(),
        timeout,
        step_timeout,
        timeout_warn,
        step_timeout_warn,
        pre_checks,
        post_checks,
        handlers,
        programmatic_handler,
    };
    debug!(
        source = %source_path.display(),
        pre_checks = plan.pre_checks.len(),
        post_checks = plan.post_checks.len(),
        timeout_secs = plan.timeout.map(|d| d.as_secs()),
        "parsed harness plan",
    );
    Ok(plan)
}

/// Create a system-owned `has_write_permission` pre-check rule for the
/// source document itself.
///
/// Inline composition requires write access to the source file. When a
/// harness is active, this rule participates in the normal pre-check
/// pipeline so that handler recovery paths (redirect, deviate, etc.)
/// can respond to permission failures instead of hard-failing before
/// the handler system exists.
pub fn inline_writability_pre_check(source_path: &Path) -> ValidationRule {
    ValidationRule {
        id: ValidationRuleId(u32::MAX),
        event: ValidationEvent::HasWritePermission,
        phase: ValidationPhase::PreOnly,
        kind: ValidationKind::HasWritePermission {
            file: source_path.to_path_buf(),
        },
        message_template: None,
        subject_key: Some(source_path.display().to_string()),
        source: None,
    }
}
