use std::path::Path;

use crate::error::{ClaudineError, Result};

/// Commands that are unconditionally blocked and may never be used as bash actions.
pub const BLOCKED_COMMANDS: &[&str] = &[
    "rm", "rmdir", "mkfs", "dd", "fdisk", "format", "shutdown", "reboot", "halt", "poweroff",
    "init", "kill", "killall", "pkill",
];

/// A resolved, validated command ready for execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedCommand {
    /// A directly executable binary at the resolved path.
    Direct(String),
    /// A script to be run through an interpreter.
    Interpreted { interpreter: String, script: String },
}

/// Validates a command string for use as a bash action.
///
/// ## Errors
///
/// Returns an error if the command is in the blocklist, the file does not exist,
/// the command cannot be found on PATH, or a JS/TS file has no usable interpreter.
pub fn validate_command(command: &str) -> Result<ValidatedCommand> {
    let path = Path::new(command);
    let base_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(command);

    if BLOCKED_COMMANDS.contains(&base_name) {
        return Err(ClaudineError::ConfigValidation(format!(
            "command `{base_name}` is blocked and may not be used as a bash action"
        )));
    }

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();

    if matches!(extension, "js" | "ts" | "mjs" | "mts") {
        return validate_js_ts(command, extension);
    }

    if path.is_absolute() {
        if !path.exists() {
            return Err(ClaudineError::ConfigValidation(format!(
                "command `{command}` does not exist"
            )));
        }
        return Ok(ValidatedCommand::Direct(command.to_string()));
    }

    match which::which(command) {
        Ok(resolved) => Ok(ValidatedCommand::Direct(
            resolved.to_string_lossy().into_owned(),
        )),
        Err(_) => Err(ClaudineError::ConfigValidation(format!(
            "command `{command}` not found on PATH"
        ))),
    }
}

/// Resolves the interpreter for a JS/TS script.
///
/// Checks for a shebang on the first line; if absent, falls back to `bun` then
/// `node` (node only for `.js`/`.mjs`).
fn validate_js_ts(command: &str, extension: &str) -> Result<ValidatedCommand> {
    let content = std::fs::read_to_string(command).map_err(|e| {
        ClaudineError::ConfigValidation(format!(
            "cannot read script `{command}`: {e}"
        ))
    })?;

    if let Some(first_line) = content.lines().next() {
        if let Some(shebang) = first_line.strip_prefix("#!") {
            let interpreter = shebang.split_whitespace().next().unwrap_or("").to_string();
            if interpreter.is_empty() {
                return Err(ClaudineError::ConfigValidation(format!(
                    "script `{command}` has an empty shebang line"
                )));
            }
            let interpreter_path = Path::new(&interpreter);
            if interpreter_path.is_absolute() {
                if !interpreter_path.exists() {
                    return Err(ClaudineError::ConfigValidation(format!(
                        "shebang interpreter `{interpreter}` not found for script `{command}`"
                    )));
                }
            } else {
                which::which(&interpreter).map_err(|_| {
                    ClaudineError::ConfigValidation(format!(
                        "shebang interpreter `{interpreter}` not found on PATH for script `{command}`"
                    ))
                })?;
            }
            return Ok(ValidatedCommand::Interpreted {
                interpreter,
                script: command.to_string(),
            });
        }
    }

    if let Ok(_bun) = which::which("bun") {
        return Ok(ValidatedCommand::Interpreted {
            interpreter: "bun".to_string(),
            script: command.to_string(),
        });
    }

    if matches!(extension, "js" | "mjs") {
        if let Ok(_node) = which::which("node") {
            return Ok(ValidatedCommand::Interpreted {
                interpreter: "node".to_string(),
                script: command.to_string(),
            });
        }
    }

    Err(ClaudineError::ConfigValidation(format!(
        "no interpreter found for script `{command}`; install bun or node"
    )))
}

/// Wraps a value in single quotes, escaping any embedded single quotes.
///
/// ## Examples
///
/// ```
/// use claudine::actions::bash_executor::shell_escape;
///
/// assert_eq!(shell_escape("hello"), "'hello'");
/// assert_eq!(shell_escape("it's"), "'it'\\''s'");
/// ```
pub fn shell_escape(value: &str) -> String {
    let escaped = value.replace('\'', "'\\''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_commands_rejected() {
        assert!(validate_command("rm").is_err());
        assert!(validate_command("shutdown").is_err());
        assert!(validate_command("killall").is_err());
    }

    #[test]
    fn blocked_command_in_absolute_path() {
        assert!(validate_command("/usr/bin/rm").is_err());
    }

    #[test]
    fn absolute_path_to_missing_file_rejected() {
        assert!(validate_command("/nonexistent/binary").is_err());
    }

    #[test]
    fn echo_on_path_accepted() {
        let result = validate_command("echo");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ValidatedCommand::Direct(_)));
    }

    #[test]
    fn shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn shell_escape_with_single_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn shell_escape_with_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn shell_escape_with_special_chars() {
        assert_eq!(shell_escape("$(rm -rf /)"), "'$(rm -rf /)'");
    }

    #[test]
    fn shell_escape_empty_string() {
        assert_eq!(shell_escape(""), "''");
    }
}
