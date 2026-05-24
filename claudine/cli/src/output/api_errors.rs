use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

/// Try to reformat a raw error line into a human-readable message.
///
/// Recognizes multiple patterns:
/// - `API Error: NNN {json}` — provider API errors with structured JSON
/// - `error: {message}` — generic CLI errors
/// - `Error: {message}` — capitalised generic errors
/// - `fatal: {message}` — fatal errors
/// - Lines containing `unrecognized argument`, `unknown flag`, `missing required argument`
///
/// Returns `None` if the line doesn't match any known pattern.
pub(crate) fn try_format_api_error(line: &str, term: &Terminal) -> Option<String> {
    if let Some(result) = try_format_structured_api_error(line, term) {
        return Some(result);
    }

    try_format_cli_error(line, term)
}

/// Format structured API error JSON (e.g. `API Error: 529 {"type":"error",...}`).
fn try_format_structured_api_error(line: &str, term: &Terminal) -> Option<String> {
    let rest = line.strip_prefix("API Error: ")?;

    let (status_str, json_part) = rest.split_once(' ')?;
    let status: u16 = status_str.parse().ok()?;

    let friendly = if let Ok(obj) = serde_json::from_str::<serde_json::Value>(json_part) {
        let error_type = obj
            .get("error")
            .and_then(|e| e.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");
        let message = obj
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        let request_id = obj.get("request_id").and_then(|r| r.as_str());

        let mut parts = vec![format!(
            "<red><bold>API Error ({status}):</bold></red> {message}"
        )];

        match error_type {
            "overloaded_error" => {
                parts.push("<dim>The API is temporarily overloaded. This is usually transient — retrying the command may succeed.</dim>".to_string());
            }
            "api_error" if status == 500 => {
                parts.push("<dim>An internal server error occurred. This is usually transient — retrying the command may succeed.</dim>".to_string());
            }
            "rate_limit_error" => {
                parts.push(
                    "<dim>Rate limit exceeded. Wait a moment before retrying.</dim>".to_string(),
                );
            }
            "authentication_error" => {
                parts.push(
                    "<dim>Authentication failed. Check your API key and provider credentials.</dim>"
                        .to_string(),
                );
            }
            "permission_error" | "forbidden_error" => {
                parts.push(
                    "<dim>Permission denied. The API key may not have access to this resource or model.</dim>"
                        .to_string(),
                );
            }
            "invalid_request_error" => {
                parts.push(
                    "<dim>The request was malformed. Check prompt length and parameter values.</dim>"
                        .to_string(),
                );
            }
            "not_found_error" => {
                parts.push(
                    "<dim>The requested resource (model, thread, etc.) was not found.</dim>"
                        .to_string(),
                );
            }
            _ => {}
        }

        if let Some(rid) = request_id {
            parts.push(format!("<dim>request: {rid}</dim>"));
        }

        parts.join("\n")
    } else {
        format!(
            "<red><bold>API Error ({status}):</bold></red> {}",
            json_part.trim()
        )
    };

    Some(Prose::new(friendly).render(term))
}

/// Format common CLI error patterns that are not API JSON errors.
///
/// Recognised line-start prefixes (`error: `, `Error: `, `fatal: `) are stripped
/// from the rendered body so that the styled `Error:` label added by this
/// function is not doubled with a plain-text `error:` already inside the line.
fn try_format_cli_error(line: &str, term: &Terminal) -> Option<String> {
    let lower = line.to_lowercase();

    let stripped_prefix = line
        .strip_prefix("error: ")
        .or_else(|| line.strip_prefix("Error: "))
        .or_else(|| line.strip_prefix("fatal: "));

    let is_cli_error = stripped_prefix.is_some()
        || lower.contains("unrecognized argument")
        || lower.contains("unknown flag")
        || lower.contains("unknown option")
        || lower.contains("unexpected argument")
        || lower.contains("missing required argument")
        || lower.contains("required argument")
        || lower.contains("the following required arguments were not provided")
        || lower.contains("permission denied")
        || lower.contains("not authorized")
        || lower.contains("authentication failed");

    if !is_cli_error {
        return None;
    }

    let body = stripped_prefix.unwrap_or(line);
    Some(Prose::new(format!("<red><bold>Error:</bold></red> {body}")).render(term))
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::eval::strip_ansi_codes;
    use biscuit_terminal::terminal::Terminal;

    fn test_terminal() -> Terminal {
        Terminal::new_optimistic(80)
    }

    #[test]
    fn try_format_api_error_parses_overloaded() {
        let term = test_terminal();
        let line = r#"API Error: 529 {"type":"error","error":{"type":"overloaded_error","message":"Overloaded. https://docs.claude.com/en/api/errors"},"request_id":"req_abc123"}"#;
        let result = try_format_api_error(line, &term).unwrap();
        assert!(result.contains("API Error (529)"));
        assert!(result.contains("Overloaded"));
        assert!(result.contains("transient"));
        assert!(result.contains("req_abc123"));
    }

    #[test]
    fn try_format_api_error_parses_500() {
        let term = test_terminal();
        let line = r#"API Error: 500 {"type":"error","error":{"type":"api_error","message":"Internal server error"},"request_id":"req_xyz"}"#;
        let result = try_format_api_error(line, &term).unwrap();
        assert!(result.contains("API Error (500)"));
        assert!(result.contains("Internal server error"));
    }

    #[test]
    fn try_format_api_error_returns_none_for_non_match() {
        let term = test_terminal();
        assert!(try_format_api_error("some random line", &term).is_none());
        assert!(try_format_api_error("completely unrelated output", &term).is_none());
    }

    #[test]
    fn try_format_api_error_catches_error_prefix() {
        let term = test_terminal();
        let result = try_format_api_error("Error: something went wrong", &term);
        assert!(result.is_some());
        assert!(result.unwrap().contains("something went wrong"));
    }

    #[test]
    fn try_format_api_error_catches_unknown_flag() {
        let term = test_terminal();
        let result = try_format_api_error("error: unrecognized argument '--foo'", &term);
        assert!(result.is_some());
    }

    #[test]
    fn try_format_api_error_catches_missing_required() {
        let term = test_terminal();
        let result = try_format_api_error(
            "error: the following required arguments were not provided:",
            &term,
        );
        assert!(result.is_some());
    }

    #[test]
    fn try_format_cli_error_strips_lowercase_error_prefix() {
        let term = test_terminal();
        let rendered = try_format_api_error("error: unrecognized argument '--foo'", &term).unwrap();
        let plain = strip_ansi_codes(&rendered);

        assert!(
            !plain.to_lowercase().contains("error: error:"),
            "styled label must not duplicate the stripped prefix: {plain}"
        );
        assert!(
            plain.contains("unrecognized argument '--foo'"),
            "body should still include the underlying message: {plain}"
        );
    }

    #[test]
    fn try_format_cli_error_strips_capital_error_prefix() {
        let term = test_terminal();
        let rendered = try_format_api_error("Error: something went wrong", &term).unwrap();
        let plain = strip_ansi_codes(&rendered);

        assert!(
            !plain.contains("Error: Error:"),
            "styled label must not duplicate the stripped prefix: {plain}"
        );
        assert!(plain.contains("something went wrong"));
    }

    #[test]
    fn try_format_cli_error_strips_fatal_prefix() {
        let term = test_terminal();
        let rendered = try_format_api_error("fatal: boom", &term).unwrap();
        let plain = strip_ansi_codes(&rendered);

        assert!(!plain.contains("Error: fatal:"));
        assert!(!plain.contains("fatal: fatal:"));
        assert!(plain.contains("boom"));
    }

    #[test]
    fn try_format_cli_error_keeps_line_when_prefix_not_at_start() {
        let term = test_terminal();
        let rendered = try_format_api_error("unknown flag --bar", &term).unwrap();
        let plain = strip_ansi_codes(&rendered);

        assert!(plain.contains("unknown flag --bar"));
    }
}
