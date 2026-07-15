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

/// Display-ready tool event. Status and summary can both be populated on
/// incoming events so successful tool results can surface the same slot
/// content the paired outgoing `→ Name(...)` arrow used. The formatter
/// NEVER writes a glyph literally — it populates a biscuit-terminal
/// `Status::ToolUse` instead.
///
/// `error_detail` carries a short, human-readable snippet describing *why*
/// an incoming event failed. Populated only when `status == Some(Error)`.
/// When present, the renderer appends it after the red `error` label so the
/// user sees the failure reason inline (e.g. `← Shell(error exit=1 · sed: …)`)
/// instead of a context-free `error` badge.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallDisplay {
    pub direction: ToolDirection,
    /// Raw provider-side tool name (e.g. `"Read"`, `"read_file"`,
    /// `"Bash"`). Preserved alongside [`Self::display_name`] so renderers
    /// can classify the tool (file vs shell vs generic) without having
    /// to reverse-engineer the humanized label.
    pub raw_name: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub status: Option<ToolStatus>,
    pub error_detail: Option<String>,
}

#[cfg(test)]
mod tests;

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
mod humanize_tests;

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
/// `Bash`/`bash`, `"shell"` for the generic `run_command` fallback.
///
/// Returns `None` for concrete shell names (`zsh`, `sh`, `fish`, …) and
/// for non-shell tools. Concrete shells already appear in the humanized
/// tool name (e.g. `Zsh(...)`), so repeating the shell inside the
/// parentheses would duplicate information: `Zsh(zsh -lc '…')` reads
/// worse than `Zsh(-lc '…')`.
fn shell_name_for_prefix(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "Bash" | "bash" => Some("bash"),
        "shell" | "run_command" => Some("shell"),
        _ => None,
    }
}

#[cfg(test)]
mod summary_tests;

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
            raw_name: raw_name.to_string(),
            display_name,
            summary,
            status: None,
            error_detail: None,
        })
    }

    /// Build an incoming display from a `SemanticEvent::ToolResult`. Status
    /// and summary can co-exist so that successful incoming events surface
    /// the same slot content (e.g. file path, shell command) that the
    /// paired outgoing `→ Name(...)` arrow used. The summary is derived
    /// from `extra["input"]` when present, then from `output`, so unknown
    /// tool shapes still degrade gracefully to a slot-less rendering.
    ///
    /// When status resolves to [`ToolStatus::Error`], a short `error_detail`
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
        // Prefer the structured input captured alongside the paired tool
        // call so successful results render with the same slot content the
        // outgoing arrow used. Fall back to the raw output body when no
        // input summary is available — `extract_tool_summary` returns
        // `None` gracefully for shapes it does not know, which keeps the
        // slot empty for tools that never had a meaningful input.
        let summary = extra
            .get("input")
            .and_then(|v| extract_tool_summary(raw_name, v))
            .or_else(|| {
                output
                    .as_ref()
                    .and_then(|v| extract_tool_summary(raw_name, v))
            });
        let error_detail = if parsed_status == Some(ToolStatus::Error) {
            extract_error_detail(*exit_code, output.as_ref(), extra)
        } else {
            None
        };
        Some(Self {
            direction: ToolDirection::Incoming,
            raw_name: raw_name.to_string(),
            display_name,
            summary,
            status: parsed_status,
            error_detail,
        })
    }

    /// Return `true` when the tool operates on a file path that the
    /// renderer should turn into an OSC8 link (e.g. `Read`, `Write`,
    /// `Edit`, Codex's `read_file` / `write_file`, Gemini's
    /// `replace_file_content`).
    pub fn is_file_tool(&self) -> bool {
        is_file_tool_name(&self.raw_name)
    }
}

/// Canonical set of file-path tool names across the providers Claudine
/// parses. Centralised so rendering + summary extraction share one list.
pub fn is_file_tool_name(raw: &str) -> bool {
    matches!(
        raw,
        "Read"
            | "Write"
            | "Edit"
            | "NotebookEdit"
            | "read_file"
            | "write_file"
            | "edit_file"
            | "replace_file_content"
    )
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
mod from_event_tests;
