//! `ToolCallDisplay` — protocol-level model for rendering a tool invocation
//! (request or response) in a single, provider-agnostic way.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
/// the dim-italic slot. Best-effort — falls back to the first non-empty
/// string value; returns `None` if no string can be extracted. Width
/// handling is the caller's responsibility.
pub fn extract_tool_summary(tool_name: &str, input: &Value) -> Option<String> {
    if let Some(s) = input.as_str() {
        return Some(s.to_string());
    }
    let obj = input.as_object()?;
    // Per-tool hooks first.
    let preferred_key = match tool_name {
        n if n.contains("search") || n == "WebSearch" || n == "WebFetch" || n == "google_web_search" => Some("query"),
        "Bash" => Some("command"),
        "Read" | "Write" | "Edit" => Some("file_path"),
        "Glob" | "Grep" => Some("pattern"),
        _ => None,
    };
    if let Some(key) = preferred_key
        && let Some(Value::String(s)) = obj.get(key)
    {
        return Some(s.clone());
    }
    // Generic well-known keys.
    for key in ["command", "path", "file_path", "dir_path", "pattern", "query", "url", "message"] {
        if let Some(Value::String(s)) = obj.get(key) {
            return Some(s.clone());
        }
    }
    // First non-empty string value.
    for (_, v) in obj.iter() {
        if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
            return Some(s.to_string());
        }
    }
    None
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
    fn returns_none_for_object_with_no_strings() {
        let input = json!({"a": 1, "b": [1,2]});
        assert!(extract_tool_summary("custom_unknown", &input).is_none());
    }
}
