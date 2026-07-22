//! Event- and support-policy-domain coercions: the platform-kind member
//! passthrough and the strict source-side validation of the facts
//! `display_policy` record.

use serde_json::Value;
use strum::VariantNames;

use claudine_catalog_types::{EventClass, PlatformKind, ToolResultSummary};

use crate::errors::GenError;
use crate::registry::RegistryEntry;

pub(super) fn platform_kind_member(entry: &RegistryEntry, raw: &Value) -> Result<Value, GenError> {
    let member = raw.as_str().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: format!("expected a platform kind enum member, got `{raw}`"),
    })?;
    if !PlatformKind::VARIANTS.contains(&member) {
        return Err(GenError::UnmappableValue {
            field: entry.field,
            message: format!("`{member}` is not a PlatformKind member"),
        });
    }
    Ok(raw.clone())
}

/// Strict source-side validation of the facts `display_policy` record.
///
/// The record passes through in catalog shape, but unlike the other
/// facts-shaped records every sub-key is checked here: an unknown sub-key
/// is a generation error (record-field strictness), the enum-valued
/// sub-keys must name catalog-types variants (strum `VariantNames` is the
/// authority), and the boolean / string-list sub-keys must carry their
/// declared shapes.
pub(super) fn display_policy_record(entry: &RegistryEntry, raw: &Value) -> Result<Value, GenError> {
    let record = raw.as_object().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: format!("expected a display-policy record, got `{raw}`"),
    })?;
    const SUB_KEYS: &[&str] = &[
        "tool_result_summary",
        "info_event_suppression",
        "collapse_task_progress",
        "suppress_subscription_rate_limit",
        "silent_extension_kinds",
        "stdout_noise_prefixes",
        "stderr_noise_prefixes",
    ];
    for key in record.keys() {
        if !SUB_KEYS.contains(&key.as_str()) {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("unknown display_policy sub-key `{key}`"),
            });
        }
    }
    let require = |key: &'static str| -> Result<&Value, GenError> {
        record.get(key).ok_or_else(|| GenError::MissingValue {
            field: entry.field,
            message: format!("display_policy record has no `{key}` key"),
        })
    };
    let summary = require("tool_result_summary")?;
    let member = summary.as_str().ok_or_else(|| GenError::UnmappableValue {
        field: entry.field,
        message: format!("expected a tool-result-summary member, got `{summary}`"),
    })?;
    if !ToolResultSummary::VARIANTS.contains(&member) {
        return Err(GenError::UnmappableValue {
            field: entry.field,
            message: format!("`{member}` is not a ToolResultSummary member"),
        });
    }
    let suppression = require("info_event_suppression")?;
    let classes = suppression
        .as_array()
        .ok_or_else(|| GenError::UnmappableValue {
            field: entry.field,
            message: format!("expected an event-class array, got `{suppression}`"),
        })?;
    for class in classes {
        let member = class.as_str().ok_or_else(|| GenError::UnmappableValue {
            field: entry.field,
            message: format!("expected an event-class member, got `{class}`"),
        })?;
        if !EventClass::VARIANTS.contains(&member) {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("`{member}` is not an EventClass member"),
            });
        }
    }
    for key in ["collapse_task_progress", "suppress_subscription_rate_limit"] {
        let value = require(key)?;
        if !value.is_boolean() {
            return Err(GenError::UnmappableValue {
                field: entry.field,
                message: format!("expected a boolean for `{key}`, got `{value}`"),
            });
        }
    }
    for key in [
        "silent_extension_kinds",
        "stdout_noise_prefixes",
        "stderr_noise_prefixes",
    ] {
        let value = require(key)?;
        let items = value.as_array().ok_or_else(|| GenError::UnmappableValue {
            field: entry.field,
            message: format!("expected a string array for `{key}`, got `{value}`"),
        })?;
        for item in items {
            if !item.is_string() {
                return Err(GenError::UnmappableValue {
                    field: entry.field,
                    message: format!("expected string elements in `{key}`, got `{item}`"),
                });
            }
        }
    }
    Ok(raw.clone())
}
