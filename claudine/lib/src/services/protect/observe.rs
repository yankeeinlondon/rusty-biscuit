use serde_json::Value;

use crate::events::{AgenticEvent, EventMeta};

use super::service::ProtectRequest;

/// Extract a ProtectRequest from event context.
///
/// Returns None for events that don't map to any scan surface.
pub fn extract_protect_request<'a>(
    event: &AgenticEvent,
    meta: &'a EventMeta,
) -> Option<ProtectRequest<'a>> {
    match event {
        AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => {
            extract_before_tool_request(meta)
        }
        AgenticEvent::AfterTool | AgenticEvent::AfterModel => {
            extract_mcp_response_request(meta)
        }
        _ => None,
    }
}

fn extract_before_tool_request<'a>(meta: &'a EventMeta) -> Option<ProtectRequest<'a>> {
    let tool_name = meta.tool_name.as_deref().unwrap_or("");
    let lowered = tool_name.to_ascii_lowercase();

    // Bash command surface
    if lowered.contains("bash") || lowered.contains("shell") || lowered.contains("exec") {
        if let Some(command) = extract_command_string(meta.tool_input.as_ref()?) {
            return Some(ProtectRequest::BashCommand { command });
        }
    }

    // Write/Edit path surface
    if lowered.contains("write")
        || lowered.contains("edit")
        || lowered.contains("create")
        || lowered.contains("delete")
    {
        if let Some(path) = extract_path_string(meta.tool_input.as_ref()?) {
            return Some(ProtectRequest::WritePath {
                path,
                cwd: meta.cwd.as_deref(),
            });
        }
    }

    None
}

fn extract_mcp_response_request<'a>(meta: &'a EventMeta) -> Option<ProtectRequest<'a>> {
    let response = meta.tool_response.as_ref()?;
    match response {
        Value::String(s) => Some(ProtectRequest::McpResponse { payload: s.as_str() }),
        _ => None, // JSON responses handled in future enhancement
    }
}

fn extract_command_string(input: &Value) -> Option<&str> {
    match input {
        Value::String(s) => Some(s.as_str()),
        Value::Object(map) => map.get("command").and_then(Value::as_str),
        _ => None,
    }
}

fn extract_path_string(input: &Value) -> Option<&str> {
    if let Value::Object(map) = input {
        for key in ["path", "file_path", "file", "target"] {
            if let Some(path) = map.get(key).and_then(Value::as_str) {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Provider;
    use serde_json::json;

    fn meta_with_command(command: &str) -> EventMeta {
        let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::BeforeTool);
        meta.tool_name = Some("Bash".to_string());
        meta.tool_input = Some(json!({ "command": command }));
        meta
    }

    fn meta_with_write_path(path: &str) -> EventMeta {
        let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::BeforeTool);
        meta.tool_name = Some("Write".to_string());
        meta.tool_input = Some(json!({ "path": path }));
        meta
    }

    fn meta_with_mcp_response(text: &str) -> EventMeta {
        let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::AfterTool);
        meta.tool_response = Some(json!(text));
        meta
    }

    #[test]
    fn extracts_bash_command() {
        let meta = meta_with_command("ls -la");
        let request = extract_protect_request(&AgenticEvent::BeforeTool, &meta);
        assert!(matches!(
            request,
            Some(ProtectRequest::BashCommand { command }) if command == "ls -la"
        ));
    }

    #[test]
    fn extracts_write_path() {
        let meta = meta_with_write_path("/etc/hosts");
        let request = extract_protect_request(&AgenticEvent::BeforeTool, &meta);
        assert!(matches!(
            request,
            Some(ProtectRequest::WritePath { path, cwd }) if path == "/etc/hosts" && cwd.is_none()
        ));
    }

    #[test]
    fn extracts_mcp_text_response() {
        let meta = meta_with_mcp_response("some response text");
        let request = extract_protect_request(&AgenticEvent::AfterTool, &meta);
        assert!(matches!(request, Some(ProtectRequest::McpResponse { .. })));
    }

    #[test]
    fn returns_none_for_irrelevant_events() {
        let meta = EventMeta::new(Provider::Claude, AgenticEvent::SessionStart);
        let request = extract_protect_request(&AgenticEvent::SessionStart, &meta);
        assert!(request.is_none());
    }
}
