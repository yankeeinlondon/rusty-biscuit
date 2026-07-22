use std::path::Path;

use tracing::warn;

use crate::error::{ClaudineError, ConfigCause, Result};

/// Advisory list of commands that look suspicious in a bash action context.
///
/// This list is an **advisory speed bump**, not a boundary. Bash actions are
/// a user-authored escape hatch; any basename-based deny list can be
/// trivially bypassed with a rename, a symlink, a `sudo`/`doas` wrapper, or a
/// `bash -c` child. When a user configures one of these commands as an
/// action we emit a `tracing::warn!` so it shows up in logs, but we still
/// allow it to run.
///
/// The authoritative command-gating surface is
/// [`ProtectService`](crate::protect::ProtectService) (see
/// `claudine/docs/topics/protect-service.md`), which is regex-backed and
/// applies to provider tool invocations across every supported CLI.
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
    ///
    /// `interpreter` is the executable (e.g., `/usr/bin/env`).
    /// `interpreter_args` are additional args before the script
    /// (e.g., `["bun"]` for `#!/usr/bin/env bun`).
    Interpreted {
        interpreter: String,
        interpreter_args: Vec<String>,
        script: String,
    },
}

/// Validates a command string for use as a bash action.
///
/// A suspicious basename matching [`BLOCKED_COMMANDS`] only emits a
/// `tracing::warn!`; the real command-gating control is
/// [`ProtectService`](crate::protect::ProtectService). See the constant's
/// rustdoc for the reasoning.
///
/// ## Errors
///
/// Returns an error if the file does not exist, the command cannot be found
/// on PATH, or a JS/TS file has no usable interpreter.
pub fn validate_command(command: &str) -> Result<ValidatedCommand> {
    let path = Path::new(command);
    let base_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(command);

    if BLOCKED_COMMANDS.contains(&base_name) {
        warn!(
            command = base_name,
            "bash action uses a command on the advisory BLOCKED_COMMANDS list; \
             configure `protect` rules to enforce a real policy"
        );
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
    let content = std::fs::read_to_string(command).map_err(|error| {
        ClaudineError::ConfigValidationWithCause {
            message: format!("cannot read script `{command}`: {error}"),
            source: ConfigCause::Read(error),
        }
    })?;

    if let Some(first_line) = content.lines().next()
        && let Some(shebang) = first_line.strip_prefix("#!")
    {
        let mut parts = shebang.split_whitespace();
        let interpreter = parts.next().unwrap_or("").to_string();
        if interpreter.is_empty() {
            return Err(ClaudineError::ConfigValidation(format!(
                "script `{command}` has an empty shebang line"
            )));
        }
        // Capture remaining shebang tokens as interpreter args
        // (e.g., `#!/usr/bin/env bun` → interpreter="/usr/bin/env", args=["bun"])
        let interpreter_args: Vec<String> = parts.map(String::from).collect();

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
            interpreter_args,
            script: command.to_string(),
        });
    }

    if let Ok(_bun) = which::which("bun") {
        return Ok(ValidatedCommand::Interpreted {
            interpreter: "bun".to_string(),
            interpreter_args: vec![],
            script: command.to_string(),
        });
    }

    if matches!(extension, "js" | "mjs")
        && let Ok(_node) = which::which("node")
    {
        return Ok(ValidatedCommand::Interpreted {
            interpreter: "node".to_string(),
            interpreter_args: vec![],
            script: command.to_string(),
        });
    }

    Err(ClaudineError::ConfigValidation(format!(
        "no interpreter found for script `{command}`; install bun or node"
    )))
}

/// Wrap `value` in single quotes, escaping any embedded single quotes.
///
/// Provided for callers building `sh -c` strings. The standard dispatch path
/// does not call this — interpolated values flow through `shell_words::split`
/// and reach the child as discrete `argv` entries via `Command::args()`, which
/// preserves variable boundaries without shell interpretation.
pub fn shell_escape(value: &str) -> String {
    let escaped = value.replace('\'', "'\\''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advisory_blocked_commands_do_not_reject() {
        // `BLOCKED_COMMANDS` is an advisory speed bump only; these resolve
        // via `which::which` and succeed. The real gating is `ProtectService`.
        // If a basename can't be found on PATH, that's a separate error.
        if which::which("rm").is_ok() {
            assert!(validate_command("rm").is_ok());
        }
    }

    #[test]
    fn blocked_command_in_absolute_path_allowed() {
        // Advisory list no longer blocks; the absolute-path gate is just
        // whether the file exists on disk.
        if Path::new("/bin/rm").exists() {
            assert!(validate_command("/bin/rm").is_ok());
        }
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

    #[test]
    fn shebang_env_bun_parses_interpreter_args() {
        if !Path::new("/usr/bin/env").exists() {
            return;
        }

        let mut tmp = tempfile::Builder::new()
            .suffix(".ts")
            .tempfile()
            .expect("failed to create temp file");

        use std::io::Write;
        writeln!(tmp, "#!/usr/bin/env bun").unwrap();
        writeln!(tmp, "console.log('hello');").unwrap();

        let path = tmp.path().to_str().unwrap().to_string();
        let result = validate_command(&path).expect("validate_command should succeed");

        match result {
            ValidatedCommand::Interpreted {
                interpreter,
                interpreter_args,
                script,
            } => {
                assert_eq!(interpreter, "/usr/bin/env");
                assert_eq!(interpreter_args, vec!["bun".to_string()]);
                assert_eq!(script, path);
            }
            other => panic!("expected Interpreted, got {other:?}"),
        }
    }

    #[test]
    fn shebang_simple_interpreter_has_no_args() {
        if !Path::new("/usr/bin/env").exists() {
            return;
        }

        // Use /usr/bin/env as the single-token shebang interpreter since we
        // know it exists (checked above). Real-world shebangs like
        // `#!/usr/bin/bun` would work identically; we just need something
        // present on disk.
        let mut tmp = tempfile::Builder::new()
            .suffix(".ts")
            .tempfile()
            .expect("failed to create temp file");

        use std::io::Write;
        writeln!(tmp, "#!/usr/bin/env").unwrap();
        writeln!(tmp, "console.log('hello');").unwrap();

        let path = tmp.path().to_str().unwrap().to_string();
        let result = validate_command(&path).expect("validate_command should succeed");

        match result {
            ValidatedCommand::Interpreted {
                interpreter,
                interpreter_args,
                script,
            } => {
                assert_eq!(interpreter, "/usr/bin/env");
                assert!(
                    interpreter_args.is_empty(),
                    "expected empty interpreter_args, got {interpreter_args:?}"
                );
                assert_eq!(script, path);
            }
            other => panic!("expected Interpreted, got {other:?}"),
        }
    }
}
