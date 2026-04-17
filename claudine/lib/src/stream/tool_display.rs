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
///
/// `error_detail` carries a short, human-readable snippet describing *why*
/// an incoming event failed. Populated only when `status == Some(Error)`.
/// When present, the renderer appends it after the red `error` label so the
/// user sees the failure reason inline (e.g. `← Shell(error exit=1 · sed: …)`)
/// instead of a context-free `error` badge.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallDisplay {
    pub direction: ToolDirection,
    pub display_name: String,
    pub summary: Option<String>,
    pub status: Option<ToolStatus>,
    pub error_detail: Option<String>,
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
            error_detail: None,
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
        assert_eq!(
            humanize_tool_name("firecrawl_firecrawl_search"),
            "Firecrawl Search"
        );
    }

    #[test]
    fn google_web_search_maps_to_canonical_label() {
        assert_eq!(humanize_tool_name("google_web_search"), "Google Web Search");
    }

    #[test]
    fn claude_builtins_pass_through() {
        for name in [
            "Bash",
            "Edit",
            "Read",
            "Write",
            "Glob",
            "Grep",
            "WebFetch",
            "WebSearch",
            "Task",
        ] {
            assert_eq!(humanize_tool_name(name), name);
        }
    }

    #[test]
    fn concrete_shells_humanize_with_title_case() {
        // Codex now reports the concrete shell ("zsh", "bash", etc.) as the
        // tool name; the display layer must humanize those to a leading-
        // capital form so the render reads as `Zsh(...)` not `zsh(...)`.
        assert_eq!(humanize_tool_name("zsh"), "Zsh");
        assert_eq!(humanize_tool_name("fish"), "Fish");
        assert_eq!(humanize_tool_name("dash"), "Dash");
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
        // Task extractor: Claude's subagent-spawn tool carries the actual
        // work in one of several fields. Prefer the shortest human-friendly
        // field first and fall back through progressively more verbose
        // shapes so callers never see an arbitrary field like
        // `subagent_type` win over the task body.
        if tool_name == "Task" {
            for key in ["description", "subject", "prompt", "task"] {
                if let Some(Value::String(s)) = obj.get(key)
                    && !s.is_empty()
                {
                    return Some(s.clone());
                }
            }
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
            n if is_shell_tool(n) => Some("command"),
            "Read" | "Write" | "Edit" | "read_file" | "write_file" | "replace_file_content" => {
                Some("file_path")
            }
            "Glob" | "Grep" | "list_directory" => Some("pattern"),
            _ => None,
        };
        if let Some(key) = preferred_key
            && let Some(Value::String(s)) = obj.get(key)
        {
            // Shell-style tools prepend the shell name so users see which
            // shell actually ran, e.g. `bash ls -la` instead of only `ls -la`.
            if key == "command"
                && let Some(shell) = shell_name_for_prefix(tool_name)
            {
                return Some(format!("{shell} {s}"));
            }
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

/// Return `true` when `tool_name` is a shell-invoking tool. Matched across
/// the providers claudine currently parses (Claude's `Bash`, Codex's
/// `shell` / concrete-shell alias, and the generic `run_command` / `bash`
/// lowercase variant).
fn is_shell_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "Bash" | "bash" | "run_command" | "shell" | "zsh" | "sh" | "fish" | "dash" | "ksh"
    )
}

/// Normalized shell prefix for a shell-style tool — `"bash"` for
/// `Bash`/`bash`, `"shell"` for the generic `run_command` fallback, and
/// the concrete shell name for Codex's resolved-shell variants.
/// Returns `None` for non-shell tools.
fn shell_name_for_prefix(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "Bash" | "bash" => Some("bash"),
        "shell" | "run_command" => Some("shell"),
        "zsh" => Some("zsh"),
        "sh" => Some("sh"),
        "fish" => Some("fish"),
        "dash" => Some("dash"),
        "ksh" => Some("ksh"),
        _ => None,
    }
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
        assert_eq!(
            extract_tool_summary("Bash", &input).as_deref(),
            Some("bash ls -la")
        );
    }

    #[test]
    fn read_extracts_file_path() {
        let input = json!({"file_path": "/etc/hosts"});
        assert_eq!(
            extract_tool_summary("Read", &input).as_deref(),
            Some("/etc/hosts")
        );
    }

    #[test]
    fn unknown_tool_falls_back_to_first_string() {
        let input = json!({"weirdo": "interesting", "n": 5});
        assert_eq!(
            extract_tool_summary("custom_unknown", &input).as_deref(),
            Some("interesting")
        );
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

    #[test]
    fn bash_summary_prepends_shell_name() {
        let input = json!({"command": "ls -la"});
        assert_eq!(
            extract_tool_summary("Bash", &input).as_deref(),
            Some("bash ls -la"),
            "Bash summary must prepend the shell name so users see the invoked shell"
        );
    }

    #[test]
    fn lowercase_bash_summary_prepends_shell_name() {
        let input = json!({"command": "pwd"});
        assert_eq!(
            extract_tool_summary("bash", &input).as_deref(),
            Some("bash pwd")
        );
    }

    #[test]
    fn codex_shell_summary_prepends_shell_prefix() {
        let input = json!({"command": "echo hi"});
        assert_eq!(
            extract_tool_summary("shell", &input).as_deref(),
            Some("shell echo hi")
        );
    }

    #[test]
    fn codex_zsh_summary_prepends_zsh_prefix() {
        // When the Codex protocol layer detects a concrete shell from the
        // leading `/bin/zsh` token it hands us `zsh` as the tool name and a
        // pre-stripped command. The summary must prepend the detected shell
        // so the rendered line reads as `Zsh(zsh -lc '...')` instead of the
        // generic `Shell(shell /bin/zsh -lc '...')`.
        let input = json!({"command": "-lc 'sed -n 1,5p file'"});
        assert_eq!(
            extract_tool_summary("zsh", &input).as_deref(),
            Some("zsh -lc 'sed -n 1,5p file'")
        );
    }

    #[test]
    fn task_extracts_description_first() {
        let input = json!({
            "description": "Review the plan",
            "subject": "Planning review",
            "prompt": "Please review the plan...",
            "task": "review",
            "subagent_type": "general-purpose",
        });
        assert_eq!(
            extract_tool_summary("Task", &input).as_deref(),
            Some("Review the plan"),
            "Task extractor must prefer description over other fields"
        );
    }

    #[test]
    fn task_falls_back_to_subject_when_description_absent() {
        let input = json!({
            "subject": "Planning review",
            "prompt": "Please review the plan...",
            "task": "review",
        });
        assert_eq!(
            extract_tool_summary("Task", &input).as_deref(),
            Some("Planning review")
        );
    }

    #[test]
    fn task_falls_back_to_prompt_when_description_absent() {
        let input = json!({
            "prompt": "Please review the plan in detail.",
            "task": "review",
            "subagent_type": "general-purpose",
        });
        assert_eq!(
            extract_tool_summary("Task", &input).as_deref(),
            Some("Please review the plan in detail."),
            "Task extractor must fall back to prompt when description and subject are absent"
        );
    }

    #[test]
    fn task_falls_back_to_task_field_as_last_resort() {
        let input = json!({
            "task": "review the plan",
            "subagent_type": "general-purpose",
        });
        assert_eq!(
            extract_tool_summary("Task", &input).as_deref(),
            Some("review the plan")
        );
    }
}

impl ToolCallDisplay {
    /// Build an outgoing display from a `SemanticEvent::ToolCall`. Returns
    /// `None` for non-matching variants.
    pub fn from_call(event: &SemanticEvent) -> Option<Self> {
        let SemanticEvent::ToolCall {
            name, input, extra, ..
        } = event
        else {
            return None;
        };
        let raw_name = name
            .as_deref()
            .or_else(|| extra.get("tool_name").and_then(Value::as_str))
            .unwrap_or("");
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
            error_detail: None,
        })
    }

    /// Build an incoming display from a `SemanticEvent::ToolResult`. Per
    /// spec: status always wins over summary in the dim slot when present;
    /// summary is consulted as a fallback only when status is absent. When
    /// status resolves to [`ToolStatus::Error`], a short `error_detail`
    /// snippet is derived from `exit_code` + the tail of `output` so the
    /// user sees the failure reason alongside the red `error` label.
    pub fn from_result(event: &SemanticEvent) -> Option<Self> {
        let SemanticEvent::ToolResult {
            name,
            status,
            exit_code,
            output,
            extra,
            ..
        } = event
        else {
            return None;
        };
        let raw_name = name
            .as_deref()
            .or_else(|| extra.get("tool_name").and_then(Value::as_str))
            .unwrap_or("");
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
        let error_detail = if parsed_status == Some(ToolStatus::Error) {
            extract_error_detail(*exit_code, output.as_ref(), extra)
        } else {
            None
        };
        Some(Self {
            direction: ToolDirection::Incoming,
            display_name,
            summary,
            status: parsed_status,
            error_detail,
        })
    }
}

/// Derive a short (≤160 chars) error snippet for an incoming error event.
/// Prefers, in order:
///
/// 1. A structured `error.message` field (MCP-style failures).
/// 2. The last non-empty line of `output` / `result` / `content` / the
///    stringified body, optionally prefixed with `exit=N` when `exit_code`
///    is both present and non-zero.
/// 3. A lone `exit=N` marker when nothing else is available.
fn extract_error_detail(
    exit_code: Option<i32>,
    output: Option<&Value>,
    extra: &Value,
) -> Option<String> {
    // Structured error field (MCP tool failures): `error.message` or top-level
    // `error` string.
    if let Some(msg) = extra
        .get("error")
        .and_then(|e| match e {
            Value::String(s) => Some(s.clone()),
            Value::Object(map) => map.get("message").and_then(Value::as_str).map(String::from),
            _ => None,
        })
        .filter(|s| !s.is_empty())
    {
        return Some(with_exit_prefix(exit_code, trim_snippet(&msg)));
    }

    // Body-derived snippet: take the last non-empty line of a string output.
    let body_text = output.and_then(value_to_snippet_source);
    let snippet = body_text
        .as_deref()
        .and_then(last_meaningful_line)
        .map(trim_snippet);

    match (exit_code, snippet) {
        (Some(code), Some(line)) if code != 0 => Some(format!("exit={code} · {line}")),
        (_, Some(line)) => Some(line),
        (Some(code), None) if code != 0 => Some(format!("exit={code}")),
        _ => None,
    }
}

fn with_exit_prefix(exit_code: Option<i32>, snippet: String) -> String {
    match exit_code {
        Some(code) if code != 0 => format!("exit={code} · {snippet}"),
        _ => snippet,
    }
}

/// Extract a string body from the `output` value. Covers the three shapes
/// Codex, Claude, and MCP tools emit: a bare string, a `{aggregated_output}`
/// wrapper, or an array of `{type, text}` content blocks.
fn value_to_snippet_source(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    if let Some(obj) = value.as_object() {
        for key in ["aggregated_output", "stderr", "stdout", "message", "text"] {
            if let Some(Value::String(s)) = obj.get(key)
                && !s.is_empty()
            {
                return Some(s.clone());
            }
        }
        if let Some(Value::Array(parts)) = obj.get("content") {
            let collected: String = parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n");
            if !collected.is_empty() {
                return Some(collected);
            }
        }
    }
    if let Some(arr) = value.as_array() {
        let collected: String = arr
            .iter()
            .filter_map(|p| p.get("text").and_then(Value::as_str).or_else(|| p.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        if !collected.is_empty() {
            return Some(collected);
        }
    }
    None
}

fn last_meaningful_line(body: &str) -> Option<&str> {
    let trimmed = body.trim_end_matches(|c: char| c.is_whitespace());
    trimmed
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
}

fn trim_snippet(body: &str) -> String {
    const MAX: usize = 160;
    let single_line = body.lines().find(|l| !l.trim().is_empty()).unwrap_or(body);
    let trimmed = single_line.trim();
    if trimmed.chars().count() > MAX {
        let truncated: String = trimmed.chars().take(MAX - 1).collect();
        format!("{truncated}\u{2026}")
    } else {
        trimmed.to_string()
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
    fn from_result_uses_extra_tool_name_when_name_missing() {
        let event = SemanticEvent::ToolResult {
            name: None,
            id: None,
            status: Some("success".into()),
            exit_code: None,
            output: None,
            extra: json!({"tool_name": "Bash"}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(display.display_name, "Bash");
        assert_eq!(display.status, Some(ToolStatus::Success));
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
        assert_eq!(display.summary.as_deref(), Some("bash ls"));
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
        assert_eq!(display.summary.as_deref(), Some("bash ls"));
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
        assert_eq!(display.summary.as_deref(), Some("bash ls -la"));
    }

    #[test]
    fn from_result_error_detail_combines_exit_code_and_last_line() {
        let event = SemanticEvent::ToolResult {
            name: Some("shell".into()),
            id: None,
            status: Some("failure".into()),
            exit_code: Some(1),
            output: Some(json!(
                "sed: 1,260p: command not found\nsed: invalid range\n"
            )),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(display.status, Some(ToolStatus::Error));
        assert_eq!(
            display.error_detail.as_deref(),
            Some("exit=1 · sed: invalid range")
        );
    }

    #[test]
    fn from_result_error_detail_reads_aggregated_output_wrapper() {
        let event = SemanticEvent::ToolResult {
            name: Some("shell".into()),
            id: None,
            status: Some("failure".into()),
            exit_code: Some(2),
            output: Some(json!({"aggregated_output": "no matches\n"})),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(display.error_detail.as_deref(), Some("exit=2 · no matches"));
    }

    #[test]
    fn from_result_error_detail_falls_back_to_exit_code_when_output_empty() {
        let event = SemanticEvent::ToolResult {
            name: Some("shell".into()),
            id: None,
            status: Some("failure".into()),
            exit_code: Some(127),
            output: None,
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(display.error_detail.as_deref(), Some("exit=127"));
    }

    #[test]
    fn from_result_error_detail_reads_mcp_error_message() {
        let event = SemanticEvent::ToolResult {
            name: Some("memory".into()),
            id: None,
            status: Some("failed".into()),
            exit_code: None,
            output: None,
            extra: json!({"error": {"message": "user cancelled MCP tool call"}}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert_eq!(
            display.error_detail.as_deref(),
            Some("user cancelled MCP tool call")
        );
    }

    #[test]
    fn from_result_error_detail_absent_for_success_path() {
        let event = SemanticEvent::ToolResult {
            name: Some("shell".into()),
            id: None,
            status: Some("success".into()),
            exit_code: Some(0),
            output: Some(json!("file.txt\n")),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        assert!(display.error_detail.is_none());
    }

    #[test]
    fn from_result_error_detail_truncates_long_snippets() {
        let long = "x".repeat(300);
        let event = SemanticEvent::ToolResult {
            name: Some("shell".into()),
            id: None,
            status: Some("failure".into()),
            exit_code: Some(1),
            output: Some(json!(long)),
            extra: json!({}),
        };
        let display = ToolCallDisplay::from_result(&event).unwrap();
        let detail = display.error_detail.expect("error_detail");
        assert!(detail.starts_with("exit=1 · "));
        assert!(
            detail.chars().count() <= "exit=1 · ".chars().count() + 160,
            "snippet length must be capped, got {}",
            detail.chars().count()
        );
        assert!(detail.ends_with('\u{2026}'));
    }
}
