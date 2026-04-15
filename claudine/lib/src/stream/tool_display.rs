//! `ToolCallDisplay` — protocol-level model for rendering a tool invocation
//! (request or response) in a single, provider-agnostic way.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::stream::semantic::SemanticEvent;

/// Direction of a tool event from the assistant's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDirection {
    Outgoing,
    Incoming,
}

/// Outcome of an incoming tool event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    Pending,
}

/// Display-ready tool event. Per spec: status wins over summary on incoming
/// events; the formatter NEVER writes a glyph literally — it populates a
/// biscuit-terminal `Status::ToolUse` instead.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallDisplay {
    pub direction: ToolDirection,
    pub display_name: String,
    pub summary: Option<String>,
    pub status: Option<ToolStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ToolDirection::Outgoing).unwrap(),
            "\"outgoing\""
        );
        assert_eq!(
            serde_json::to_string(&ToolStatus::Success).unwrap(),
            "\"success\""
        );
    }

    #[test]
    fn struct_round_trips_via_clone_and_eq() {
        let display = ToolCallDisplay {
            direction: ToolDirection::Incoming,
            display_name: "Firecrawl Search".into(),
            summary: Some("NFL draft 2026 date".into()),
            status: Some(ToolStatus::Success),
        };
        let cloned = display.clone();
        assert_eq!(display, cloned);
    }
}

/// Resolve a raw tool id like `firecrawl_firecrawl_search` into a
/// human-readable display name. Two-tier strategy:
///
/// 1. Lookup table for known tools / prefixes.
/// 2. Algorithmic fallback (strip provider-redundant prefix, split on `_`,
///    Title Case).
///
/// As a last resort returns the raw id unchanged.
pub fn humanize_tool_name(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    if let Some(name) = humanize_known(raw) {
        return name;
    }
    humanize_algorithmic(raw)
}

fn humanize_known(raw: &str) -> Option<String> {
    // MCP-shape: mcp__<server>__<tool>
    if let Some(rest) = raw.strip_prefix("mcp__") {
        let (server, tool) = rest.split_once("__")?;
        return Some(format!(
            "{} {}",
            title_case_segments(server),
            title_case_segments(tool)
        ));
    }
    if let Some(rest) = raw.strip_prefix("firecrawl_firecrawl_") {
        return Some(format!("Firecrawl {}", title_case_segments(rest)));
    }
    if let Some(rest) = raw.strip_prefix("firecrawl_") {
        return Some(format!("Firecrawl {}", title_case_segments(rest)));
    }
    match raw {
        "google_web_search" => Some("Google Web Search".into()),
        "Bash" | "Edit" | "Read" | "Write" | "Glob" | "Grep" | "WebFetch" | "WebSearch"
        | "Task" => Some(raw.into()),
        _ => None,
    }
}

fn humanize_algorithmic(raw: &str) -> String {
    title_case_segments(raw)
}

fn title_case_segments(s: &str) -> String {
    s.split('_')
        .filter(|seg| !seg.is_empty())
        .map(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod humanize_tests {
    use super::*;

    #[test]
    fn firecrawl_double_prefix_collapses_to_single_firecrawl() {
        assert_eq!(humanize_tool_name("firecrawl_firecrawl_search"), "Firecrawl Search");
    }

    #[test]
    fn google_web_search_maps_to_canonical_label() {
        assert_eq!(humanize_tool_name("google_web_search"), "Google Web Search");
    }

    #[test]
    fn claude_builtins_pass_through() {
        for name in ["Bash", "Edit", "Read", "Write", "Glob", "Grep", "WebFetch", "WebSearch", "Task"] {
            assert_eq!(humanize_tool_name(name), name);
        }
    }

    #[test]
    fn mcp_prefix_renders_server_and_tool() {
        assert_eq!(
            humanize_tool_name("mcp__firecrawl__deep_research"),
            "Firecrawl Deep Research"
        );
    }

    #[test]
    fn unknown_snake_case_falls_through_to_title_case() {
        assert_eq!(humanize_tool_name("custom_local_tool"), "Custom Local Tool");
    }

    #[test]
    fn empty_input_returns_empty_string() {
        assert_eq!(humanize_tool_name(""), "");
    }
}

/// Extract the meaningful slice of a tool's input arguments for display in
/// the dim-italic slot. Best-effort with a "never lose information"
/// invariant: per the spec, unknown tool shapes fall back to compact raw
/// JSON rather than being hidden. Returns `None` only when the input is
/// null / an empty object.
pub fn extract_tool_summary(tool_name: &str, input: &Value) -> Option<String> {
    if input.is_null() {
        return None;
    }
    if let Some(s) = input.as_str() {
        return Some(s.to_string());
    }
    if let Some(obj) = input.as_object() {
        if obj.is_empty() {
            return None;
        }
        // Per-tool hooks first.
        let preferred_key = match tool_name {
            n if n.contains("search")
                || n == "WebSearch"
                || n == "WebFetch"
                || n == "google_web_search"
                || n == "search_file" =>
            {
                Some("query")
            }
            "Bash" | "bash" | "run_command" => Some("command"),
            "Read" | "Write" | "Edit"
            | "read_file" | "write_file" | "replace_file_content" => Some("file_path"),
            "Glob" | "Grep" | "list_directory" => Some("pattern"),
            _ => None,
        };
        if let Some(key) = preferred_key
            && let Some(Value::String(s)) = obj.get(key)
        {
            return Some(s.clone());
        }
        // Generic well-known keys.
        for key in [
            "command",
            "path",
            "file_path",
            "dir_path",
            "pattern",
            "query",
            "url",
            "message",
        ] {
            if let Some(Value::String(s)) = obj.get(key) {
                return Some(s.clone());
            }
        }
        // First non-empty top-level string value. Preferred over raw JSON
        // when present because a meaningful single-string parameter reads
        // better than a bag of keys.
        for (_, v) in obj.iter() {
            if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
                return Some(s.to_string());
            }
        }
    }
    // Last resort: compact raw JSON. Per spec, never hide the tool arguments
    // entirely — render them verbatim and let the sink's width/wrapping
    // rules handle long values.
    serde_json::to_string(input).ok()
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn web_search_extracts_query() {
        let input = json!({"query": "NFL draft 2026 date", "limit": 5});
        assert_eq!(
            extract_tool_summary("firecrawl_firecrawl_search", &input).as_deref(),
            Some("NFL draft 2026 date")
        );
    }

    #[test]
    fn bash_extracts_command() {
        let input = json!({"command": "ls -la"});
        assert_eq!(extract_tool_summary("Bash", &input).as_deref(), Some("ls -la"));
    }

    #[test]
    fn read_extracts_file_path() {
        let input = json!({"file_path": "/etc/hosts"});
        assert_eq!(extract_tool_summary("Read", &input).as_deref(), Some("/etc/hosts"));
    }

    #[test]
    fn unknown_tool_falls_back_to_first_string() {
        let input = json!({"weirdo": "interesting", "n": 5});
        assert_eq!(extract_tool_summary("custom_unknown", &input).as_deref(), Some("interesting"));
    }

    #[test]
    fn falls_back_to_raw_json_for_object_with_no_strings() {
        let input = json!({"a": 1, "b": [1, 2]});
        let rendered = extract_tool_summary("custom_unknown", &input).expect("raw JSON fallback");
        // Parse both ends and compare semantically so we don't depend on
        // serde_json's key-ordering behavior.
        let roundtrip: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(roundtrip, input);
    }

    #[test]
    fn returns_none_for_null_or_empty_object() {
        assert!(extract_tool_summary("custom_unknown", &json!(null)).is_none());
        assert!(extract_tool_summary("custom_unknown", &json!({})).is_none());
    }

    #[test]
    fn falls_back_to_raw_json_for_array_input() {
        let input = json!([1, 2, 3]);
        let rendered = extract_tool_summary("custom_unknown", &input).unwrap();
        assert_eq!(rendered, "[1,2,3]");
    }
}

impl ToolCallDisplay {
    /// Build an outgoing display from a `SemanticEvent::ToolCall`. Returns
    /// `None` for non-matching variants.
    pub fn from_call(event: &SemanticEvent) -> Option<Self> {
        let SemanticEvent::ToolCall { name, input, .. } = event else {
            return None;
        };
        let raw_name = name.as_deref().unwrap_or("");
        let display_name = if raw_name.is_empty() {
            "(tool)".into()
        } else {
            humanize_tool_name(raw_name)
        };
        let summary = input
            .as_ref()
            .and_then(|v| extract_tool_summary(raw_name, v));
        Some(Self {
            direction: ToolDirection::Outgoing,
            display_name,
            summary,
            status: None,
        })
    }

    /// Build an incoming display from a `SemanticEvent::ToolResult`. Per
    /// spec: status always wins over summary in the dim slot when present;
    /// summary is consulted as a fallback only when status is absent.
    pub fn from_result(event: &SemanticEvent) -> Option<Self> {
        let SemanticEvent::ToolResult {
            name,
            status,
            output,
            extra,
            ..
        } = event
        else {
            return None;
        };
        let raw_name = name.as_deref().unwrap_or("");
        let display_name = if raw_name.is_empty() {
            "(tool)".into()
        } else {
            humanize_tool_name(raw_name)
        };
        let parsed_status = status.as_deref().and_then(|s| match s {
            "success" | "completed" | "ok" => Some(ToolStatus::Success),
            "error" | "failure" | "failed" | "timeout" | "cancelled" | "aborted" => {
                Some(ToolStatus::Error)
            }
            "pending" | "running" | "in_progress" => Some(ToolStatus::Pending),
            _ => None,
        });
        let summary = if parsed_status.is_some() {
            None
        } else {
            // Status absent: fall back to a derived output summary.
            output
                .as_ref()
                .or_else(|| extra.get("input"))
                .and_then(|v| extract_tool_summary(raw_name, v))
        };
        Some(Self {
            direction: ToolDirection::Incoming,
            display_name,
            summary,
            status: parsed_status,
        })
    }
}

#[cfg(test)]
mod from_event_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn from_call_humanizes_and_extracts_query() {
        let event = SemanticEvent::ToolCall {
            name: Some("firecrawl_firecrawl_search".into()),
            id: None,
            input: Some(json!({"query": "NFL"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_call(&event).unwrap();
        assert_eq!(display.direction, ToolDirection::Outgoing);
        assert_eq!(display.display_name, "Firecrawl Search");
        assert_eq!(display.summary.as_deref(), Some("NFL"));
        assert!(display.status.is_none());
    }

    #[test]
    fn from_result_uses_status_and_drops_summary_when_status_present() {
        let event = SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: Some("success".into()),
            exit_code: None,
            output: Some(json!({"stdout": "ok"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(display.status, Some(ToolStatus::Success));
        assert!(display.summary.is_none(), "status wins over summary");
    }

    #[test]
    fn from_result_falls_back_to_summary_when_status_absent() {
        let event = SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: None,
            exit_code: None,
            output: Some(json!({"command": "ls"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert!(display.status.is_none());
        assert_eq!(display.summary.as_deref(), Some("ls"));
    }

    #[test]
    fn from_result_maps_timeout_and_cancelled_to_error() {
        for raw in ["timeout", "cancelled", "aborted"] {
            let event = SemanticEvent::ToolResult {
                name: Some("Bash".into()),
                id: None,
                status: Some(raw.into()),
                exit_code: None,
                output: None,
                extra: json!({}),
            };
            let display = ToolCallDisplay::from_result(&event).unwrap();
            assert_eq!(
                display.status,
                Some(ToolStatus::Error),
                "raw status {raw:?} should map to Error"
            );
        }
    }

    #[test]
    fn from_result_unknown_status_falls_through_to_summary() {
        let event = SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: Some("xyz".into()),
            exit_code: None,
            output: Some(json!({"command": "ls"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert!(display.status.is_none());
        assert_eq!(display.summary.as_deref(), Some("ls"));
    }

    #[test]
    fn from_result_uses_extra_input_when_output_absent() {
        let event = SemanticEvent::ToolResult {
            name: Some("Bash".into()),
            id: None,
            status: None,
            exit_code: None,
            output: None,
            extra: json!({"input": {"command": "ls -la"}}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(display.summary.as_deref(), Some("ls -la"));
    }
}
