//! Shared assembly of the `OPENCODE_CONFIG_CONTENT` environment value.
//!
//! OpenCode reads a single inline JSON config from `OPENCODE_CONFIG_CONTENT` and
//! applies it **session-wide** — to the parent run *and* every subagent (Task)
//! session. Multiple Claudine producers contribute to that one value: the MCP
//! injector adds `{"mcp": …}`, the system-prompt path adds `{"instructions": …}`,
//! and (under YOLO) a `{"permission": …}` overlay. Because they target the same
//! env var, each writer must **merge** into the existing value rather than
//! overwrite it — overwriting is the latent clobber this module exists to remove.
//! Every runtime write site routes through [`merge_overlay`].

use std::ffi::OsStr;

use serde_json::{Map, Value};

use crate::error::{ClaudineError, Result};

/// Recursively merge `overlay` into `base`.
///
/// Objects merge key-by-key (recursing into nested objects); arrays and scalars
/// are **replaced** by the overlay. Array replacement is deliberate: the
/// `instructions` overlay must be able to set the full list, not append to a
/// stale one.
pub fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(base_value) => deep_merge(base_value, overlay_value),
                    None => {
                        base_map.insert(key, overlay_value);
                    }
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

/// The YOLO permission overlay applied to non-interactive OpenCode sessions.
///
/// `--dangerously-skip-permissions` only auto-approves the parent session;
/// subagent sessions fall back to the `"ask"` default and block forever without
/// a TTY. This config-level block makes YOLO authoritative session-wide. The
/// object is path-free and key-stable so it serializes byte-identically on every
/// platform.
pub fn yolo_permission_block() -> Value {
    let mut permission = Map::new();
    permission.insert("*".into(), Value::String("allow".into()));
    permission.insert("external_directory".into(), Value::String("allow".into()));
    permission.insert("doom_loop".into(), Value::String("allow".into()));

    let mut root = Map::new();
    root.insert("permission".into(), Value::Object(permission));
    Value::Object(root)
}

/// Merge `overlay` into the current `OPENCODE_CONFIG_CONTENT` value, returning
/// the compact JSON string to write back.
///
/// `current` is the raw existing env value as an [`OsStr`] (`None` or absent
/// starts from `{}`), which may carry a user-supplied config. Taking the raw
/// `OsStr` rather than a pre-decoded `&str` keeps the UTF-8 validity decision in
/// this one place, so all call sites reject a present-but-non-UTF-8 value
/// identically and cannot regress into silently replacing it. The result
/// preserves every unrelated key already present.
///
/// ## Errors
///
/// Returns [`ClaudineError::ConfigValidation`] when `current` is present but is
/// either not valid UTF-8, does not parse as JSON, or parses to a non-object
/// JSON value (such as `"42"` or `[1,2]`). Every message names
/// `OPENCODE_CONFIG_CONTENT` but is **redacted** — it never echoes the raw value
/// (or its bytes), which may contain secrets.
pub fn merge_overlay(current: Option<&OsStr>, overlay: Value) -> Result<String> {
    let mut base = match current {
        None => Value::Object(Map::new()),
        Some(os) => {
            // A non-UTF-8 value is present-but-undecodable, not absent. Rejecting
            // it here (rather than treating `to_str() == None` as "start fresh")
            // is what stops the user's existing config from being silently
            // clobbered. UTF-8 is only invalid on Unix-like targets; Windows env
            // values are Unicode.
            let raw = os.to_str().ok_or_else(|| {
                ClaudineError::ConfigValidation(
                    "existing OPENCODE_CONFIG_CONTENT is not valid UTF-8".to_string(),
                )
            })?;
            let parsed: Value = serde_json::from_str(raw).map_err(|_| {
                ClaudineError::ConfigValidation(
                    "existing OPENCODE_CONFIG_CONTENT is not valid JSON".to_string(),
                )
            })?;
            if !parsed.is_object() {
                return Err(ClaudineError::ConfigValidation(
                    "existing OPENCODE_CONFIG_CONTENT is not a JSON object".to_string(),
                ));
            }
            parsed
        }
    };

    deep_merge(&mut base, overlay);
    Ok(serde_json::to_string(&base)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deep_merge_merges_nested_objects() {
        let mut base = json!({ "a": { "x": 1 }, "keep": true });
        deep_merge(&mut base, json!({ "a": { "y": 2 } }));
        assert_eq!(base, json!({ "a": { "x": 1, "y": 2 }, "keep": true }));
    }

    #[test]
    fn deep_merge_later_array_replaces_earlier() {
        let mut base = json!({ "instructions": ["a", "b"] });
        deep_merge(&mut base, json!({ "instructions": ["c"] }));
        assert_eq!(base, json!({ "instructions": ["c"] }));
    }

    #[test]
    fn yolo_permission_block_has_exactly_three_keys() {
        let block = yolo_permission_block();
        let permission = block.get("permission").and_then(Value::as_object).unwrap();
        assert_eq!(permission.len(), 3);
        assert_eq!(permission["*"], json!("allow"));
        assert_eq!(permission["external_directory"], json!("allow"));
        assert_eq!(permission["doom_loop"], json!("allow"));
        // serde round-trip equality against the documented shape.
        assert_eq!(
            block,
            json!({ "permission": { "*": "allow", "external_directory": "allow", "doom_loop": "allow" } })
        );
    }

    #[test]
    fn merge_overlay_none_starts_from_empty_object() {
        let result = merge_overlay(None, yolo_permission_block()).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, yolo_permission_block());
    }

    #[test]
    fn merge_overlay_preserves_unrelated_keys() {
        let current = json!({ "instructions": ["x"], "theme": "dark" }).to_string();
        let result =
            merge_overlay(Some(OsStr::new(&current)), json!({ "mcp": { "srv": {} } })).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            parsed,
            json!({ "instructions": ["x"], "theme": "dark", "mcp": { "srv": {} } })
        );
    }

    #[test]
    fn merge_overlay_rejects_malformed_json_with_redacted_message() {
        let raw = "{not valid json secret=hunter2";
        let err = merge_overlay(Some(OsStr::new(raw)), yolo_permission_block()).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("OPENCODE_CONFIG_CONTENT"));
        assert!(!message.contains("hunter2"));
        assert!(!message.contains("not valid json"));
    }

    #[test]
    fn merge_overlay_rejects_non_object_json() {
        for raw in ["42", "[1,2]", "\"a string\""] {
            let err = merge_overlay(Some(OsStr::new(raw)), yolo_permission_block()).unwrap_err();
            assert!(err.to_string().contains("OPENCODE_CONFIG_CONTENT"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn merge_overlay_rejects_non_utf8_value_without_echoing_bytes() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        // 0x80 is an invalid UTF-8 continuation byte, so this OsString is present
        // but undecodable. It must be rejected (not silently replaced from `{}`),
        // and the message must name the var without echoing the raw bytes.
        let invalid = OsString::from_vec(vec![0x66, 0x80]);
        let err = merge_overlay(Some(invalid.as_os_str()), yolo_permission_block()).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("OPENCODE_CONFIG_CONTENT"));
        assert!(message.contains("UTF-8"));
        // The raw bytes (including the invalid 0x80) must never be echoed.
        assert!(!message.contains('\u{80}'));
        assert!(!message.as_bytes().contains(&0x80));
    }

    #[test]
    fn yolo_permission_block_serializes_independent_of_insertion_order() {
        // Build the same logical block with the three keys inserted in a
        // different order; serialization must be byte-identical cross-platform.
        let mut permission = Map::new();
        permission.insert("doom_loop".into(), Value::String("allow".into()));
        permission.insert("*".into(), Value::String("allow".into()));
        permission.insert("external_directory".into(), Value::String("allow".into()));
        let mut root = Map::new();
        root.insert("permission".into(), Value::Object(permission));
        let reordered = Value::Object(root);

        assert_eq!(
            serde_json::to_string(&yolo_permission_block()).unwrap(),
            serde_json::to_string(&reordered).unwrap()
        );
    }
}
