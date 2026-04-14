//! `ToolCallDisplay` — protocol-level model for rendering a tool invocation
//! (request or response) in a single, provider-agnostic way.

use serde::{Deserialize, Serialize};

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
        let mut parts = rest.splitn(2, "__");
        let server = parts.next()?;
        let tool = parts.next()?;
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
