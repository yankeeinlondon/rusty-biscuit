use std::borrow::Cow;

use serde_json::Value;
use tracing::warn;

use crate::events::{AgenticEvent, EventMeta, ToolName};

use super::catalog::ScanSurface;
use super::service::ProtectRequest;

/// Outcome of attempting to extract a [`ProtectRequest`] from an event.
///
/// `NoOpinion` means the event was inspected and nothing relevant was found.
/// `Unparsed` means the tool looked command- or write-shaped but a payload
/// could not be extracted; the dispatch boundary decides how to handle it.
#[derive(Debug)]
pub enum ProtectObservation<'a> {
    Request(ProtectRequest<'a>),
    NoOpinion,
    Unparsed {
        surface: ScanSurface,
        reason: &'static str,
    },
}

/// Extract a protect observation from event context.
///
/// Returns [`ProtectObservation::NoOpinion`] for events that don't map to any
/// scan surface, and [`ProtectObservation::Unparsed`] for command- or write-
/// shaped tools whose payload could not be extracted.
pub fn extract_protect_request<'a>(event: &AgenticEvent, meta: &'a EventMeta) -> ProtectObservation<'a> {
    match event {
        AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => extract_before_tool_request(meta),
        AgenticEvent::AfterTool | AgenticEvent::AfterModel => {
            extract_mcp_response_request(meta).map_or(ProtectObservation::NoOpinion, ProtectObservation::Request)
        }
        _ => ProtectObservation::NoOpinion,
    }
}

fn is_bash_like_tool(tool_name: &str) -> bool {
    let lowered = tool_name.to_ascii_lowercase();
    lowered.contains("bash")
        || lowered.contains("shell")
        || lowered.contains("exec")
        || lowered == "run_command"
        || lowered == "terminal"
}

fn is_write_like_tool(tool_name: &str) -> bool {
    let lowered = tool_name.to_ascii_lowercase();
    lowered.contains("write")
        || lowered.contains("edit")
        || lowered.contains("create")
        || lowered.contains("delete")
}

fn extract_before_tool_request<'a>(meta: &'a EventMeta) -> ProtectObservation<'a> {
    let tool_name = meta.tool_name.as_deref().unwrap_or("");
    let input = meta.tool_input.as_ref();

    if is_bash_like_tool(tool_name) {
        return match input.and_then(extract_command_string) {
            Some(command) => ProtectObservation::Request(ProtectRequest::BashCommand { command }),
            None => ProtectObservation::Unparsed {
                surface: ScanSurface::BashCommand,
                reason: "bash-shaped tool with no extractable command",
            },
        };
    }

    if is_write_like_tool(tool_name) {
        let paths = input.map(extract_path_strings).unwrap_or_default();
        return if paths.is_empty() {
            ProtectObservation::Unparsed {
                surface: ScanSurface::WritePath,
                reason: "write-shaped tool with no extractable path",
            }
        } else {
            ProtectObservation::Request(ProtectRequest::WritePath {
                paths,
                cwd: meta.cwd.as_deref(),
            })
        };
    }

    ProtectObservation::NoOpinion
}

/// Caps applied when scanning untrusted MCP response payloads.
///
/// The built-in deny patterns run in linear time — Rust's `regex` has no
/// catastrophic backtracking — but the cost is still
/// `O(payload_bytes × pattern_count)` per response. A hostile MCP server can
/// return a multi-megabyte body, so these caps bound the work per response: at
/// most [`MAX_SCAN_LEAVES`] string leaves totalling [`MAX_SCAN_BYTES`], with any
/// single leaf truncated to [`MAX_LEAF_BYTES`]. Truncation only shortens what is
/// scanned; a match in the surviving prefix still blocks.
const MAX_SCAN_LEAVES: usize = 10_000;
const MAX_SCAN_BYTES: usize = 1024 * 1024;
const MAX_LEAF_BYTES: usize = 64 * 1024;

fn extract_mcp_response_request<'a>(meta: &'a EventMeta) -> Option<ProtectRequest<'a>> {
    // Only scan responses from MCP-backed tools
    let tool_name = meta.tool_name.as_deref()?;
    if !ToolName(tool_name.to_string()).is_mcp_tool() {
        return None;
    }

    let response = meta.tool_response.as_ref()?;
    let mut collector = LeafCollector::new();
    collect_json_strings(response, &mut collector);
    if collector.leaves.is_empty() {
        return None;
    }
    if collector.truncated {
        warn!(
            tool_name,
            leaves = collector.leaves.len(),
            scanned_bytes = collector.bytes,
            "MCP response exceeded protect scan cap; scanning a truncated prefix"
        );
    }
    Some(ProtectRequest::McpResponse {
        payloads: collector.leaves.into_iter().map(Cow::Borrowed).collect(),
    })
}

/// Bounded accumulator for the string leaves of an MCP response.
///
/// Stops collecting once the leaf-count or total-byte cap is reached and
/// truncates any single leaf to [`MAX_LEAF_BYTES`] (or the remaining total
/// budget, whichever is smaller). `truncated` records whether any cap clipped
/// the input so the caller can log it.
struct LeafCollector<'a> {
    leaves: Vec<&'a str>,
    bytes: usize,
    truncated: bool,
}

impl<'a> LeafCollector<'a> {
    fn new() -> Self {
        Self {
            leaves: Vec::new(),
            bytes: 0,
            truncated: false,
        }
    }

    fn is_full(&self) -> bool {
        self.leaves.len() >= MAX_SCAN_LEAVES || self.bytes >= MAX_SCAN_BYTES
    }

    fn push(&mut self, s: &'a str) {
        if self.is_full() {
            self.truncated = true;
            return;
        }
        let budget = MAX_LEAF_BYTES.min(MAX_SCAN_BYTES - self.bytes);
        let slice = if s.len() > budget {
            self.truncated = true;
            truncate_on_char_boundary(s, budget)
        } else {
            s
        };
        self.bytes += slice.len();
        self.leaves.push(slice);
    }
}

/// Truncate `s` to at most `max_bytes`, backing up to the nearest UTF-8
/// character boundary so the result is always a valid `&str`.
fn truncate_on_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Recursively collect string leaves from a JSON value into a bounded collector,
/// stopping early once a cap is reached.
fn collect_json_strings<'a>(value: &'a Value, collector: &mut LeafCollector<'a>) {
    if collector.is_full() {
        return;
    }
    match value {
        Value::String(s) => collector.push(s.as_str()),
        Value::Array(arr) => {
            for item in arr {
                if collector.is_full() {
                    break;
                }
                collect_json_strings(item, collector);
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                if collector.is_full() {
                    break;
                }
                collect_json_strings(v, collector);
            }
        }
        _ => {}
    }
}

fn join_string_array(arr: &[Value]) -> Option<String> {
    let mut parts = Vec::with_capacity(arr.len());
    for item in arr {
        let s = item.as_str()?;
        parts.push(s);
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join(" "))
}

fn extract_command_string(input: &Value) -> Option<Cow<'_, str>> {
    match input {
        Value::String(s) => Some(Cow::Borrowed(s.as_str())),
        Value::Array(arr) => join_string_array(arr).map(Cow::Owned),
        Value::Object(map) => {
            for key in ["command", "cmd", "script", "input"] {
                if let Some(value) = map.get(key) {
                    match value {
                        Value::String(s) => return Some(Cow::Borrowed(s.as_str())),
                        Value::Array(arr) => {
                            if let Some(joined) = join_string_array(arr) {
                                return Some(Cow::Owned(joined));
                            }
                        }
                        _ => {}
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Collect every candidate write path from a tool input.
///
/// Single-string keys yield one path; the `paths` array yields every string
/// element. All candidates are returned so the protect service can block when
/// any one of them is sensitive, rather than only inspecting the first.
fn extract_path_strings(input: &Value) -> Vec<&str> {
    match input {
        Value::String(s) => vec![s.as_str()],
        Value::Object(map) => {
            for key in ["path", "file_path", "file", "target", "filename", "dest", "paths"] {
                if let Some(value) = map.get(key) {
                    match value {
                        Value::String(s) => return vec![s.as_str()],
                        Value::Array(arr) if key == "paths" => {
                            let paths: Vec<&str> =
                                arr.iter().filter_map(Value::as_str).collect();
                            if !paths.is_empty() {
                                return paths;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
