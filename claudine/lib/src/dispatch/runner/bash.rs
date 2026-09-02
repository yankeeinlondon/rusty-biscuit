use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::actions::bash_executor;
use crate::dispatch::template::interpolate;
use crate::error::Result;
use crate::events::EventMeta;

/// Matches any Handlebars-style `{{...}}` placeholder so we can count
/// them when diagnosing whitespace-split interpolated values.
static BASH_PARAMS_PLACEHOLDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{[^{}]*\}\}").expect("bash-params placeholder regex is valid")
});

/// Sentinel token substituted for each `{{...}}` during the diagnostic
/// "expected token count" render. Chosen to be unlikely to appear in any
/// real config and free of shell metacharacters.
const BASH_PARAMS_SENTINEL: &str = "__CLAUDINE_TEMPLATE_SENTINEL__";

/// Upper bound on how long a single bash action may run before the
/// dispatch loop gives up on it. Bash actions used to be fire-and-forget
/// via `tokio::spawn`, which silently lost work whenever Claudine was
/// invoked as a `handle` hook under [`CLAUDINE_HANDLE_DEADLINE_SECONDS`].
/// Awaiting inline with this bound lets the hook handler complete the
/// action reliably while still protecting long-lived wrapper sessions
/// from a hung bash action stalling the event pipeline.
pub(super) const BASH_ACTION_TIMEOUT: Duration = Duration::from_secs(3);

/// Execute a validated command asynchronously using direct spawning.
///
/// ## Interpolation Contract
///
/// Template placeholders in `command` and `params` are expanded by
/// [`interpolate`] as raw string substitution. The rendered `params`
/// string is then split by [`shell_words::split`] into discrete `argv`
/// entries. This means:
///
/// - Interpolated values that contain spaces **will** split into
///   multiple arguments unless the config author quoted them in the
///   `params` template (e.g., `--message '{{tool_name}}'`).
/// - No shell metacharacter interpretation occurs because the command
///   is spawned directly via `Command::new().args()`, not through
///   `sh -c`.
/// - The `shell_escape()` helper in `bash_executor` is intentionally
///   not used here — it is for callers that build `sh -c` strings.
pub(super) async fn execute_bash(command: &str, params: &str, meta: &EventMeta) {
    let cmd = interpolate(command, meta);

    let validated = match bash_executor::validate_command(&cmd) {
        Ok(v) => v,
        Err(error) => {
            warn!(%cmd, %error, "Bash action blocked by validation");
            return;
        }
    };

    let rendered_params = interpolate(params, meta);

    // Parse rendered_params using shell-words to correctly handle quoted arguments
    // and interpolated values containing spaces (e.g., `--message 'hello world'`).
    let param_args: Vec<String> = if rendered_params.is_empty() {
        vec![]
    } else {
        match shell_words::split(&rendered_params) {
            Ok(args) => args,
            Err(e) => {
                warn!(%rendered_params, %e, "Failed to parse bash action params");
                return;
            }
        }
    };

    // Diagnostic: if the rendered params split into more tokens than the
    // same template with every placeholder replaced by a single sentinel
    // word, an interpolated value contained whitespace that the author
    // likely did not intend (e.g. `--message {{tool_name}}` with
    // `tool_name = "my tool"` silently becomes `["--message", "my",
    // "tool"]`). The canonical fix is to quote the placeholder in the
    // template. We only warn, so existing configs keep working.
    warn_on_silent_token_split(params, &rendered_params, &param_args);

    debug!(?validated, ?param_args, "Awaiting bash action inline");

    // Run inline with a bounded timeout. Previously this was a
    // `tokio::spawn(...)` fire-and-forget, which silently lost work
    // whenever the dispatch completed before the spawned task ran
    // (notably the `handle` hook path under `CLAUDINE_HANDLE_DEADLINE_SECONDS`).
    let action = async {
        match &validated {
            bash_executor::ValidatedCommand::Direct(executable) => {
                let mut command = Command::new(executable);
                command.args(&param_args);
                if let Err(error) = crate::child_environment::contribute_child_environment(&mut command) {
                    return Err(std::io::Error::other(error));
                }
                command.output().await
            }
            bash_executor::ValidatedCommand::Interpreted {
                interpreter,
                interpreter_args,
                script,
            } => {
                let mut cmd = Command::new(interpreter);
                cmd.args(interpreter_args);
                cmd.arg(script);
                cmd.args(&param_args);
                if let Err(error) = crate::child_environment::contribute_child_environment(&mut cmd) {
                    return Err(std::io::Error::other(error));
                }
                cmd.output().await
            }
        }
    };
    match tokio::time::timeout(BASH_ACTION_TIMEOUT, action).await {
        Ok(Ok(output)) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(?validated, %stderr, "Bash action exited with non-zero status");
        }
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            warn!(?validated, %error, "Bash action failed to spawn");
        }
        Err(_) => {
            warn!(
                ?validated,
                timeout_ms = BASH_ACTION_TIMEOUT.as_millis() as u64,
                "Bash action timed out",
            );
        }
    }
}

/// Emit a diagnostic `warn!` when an interpolated bash-action param split
/// on whitespace in a way the template author likely did not intend.
///
/// We render the template a second time with each `{{...}}` replaced by
/// [`BASH_PARAMS_SENTINEL`], split both forms via `shell_words`, and
/// compare token counts. A mismatch means the interpolation introduced
/// extra argv slots — useful to surface because the resulting command
/// runs silently with the wrong arguments.
pub(super) fn warn_on_silent_token_split(template: &str, rendered: &str, actual: &[String]) {
    if template.is_empty() {
        return;
    }
    let sentinel_template = BASH_PARAMS_PLACEHOLDER_RE
        .replace_all(template, BASH_PARAMS_SENTINEL)
        .into_owned();
    let Ok(expected_tokens) = shell_words::split(&sentinel_template) else {
        return;
    };
    if expected_tokens.len() != actual.len() {
        warn!(
            template,
            rendered,
            expected_tokens = expected_tokens.len(),
            actual_tokens = actual.len(),
            "Bash action param template split differently after interpolation. \
             Quote your placeholders (e.g. `'{{value}}'`) to keep each one in a \
             single argv slot, or migrate the config to an array-form `params` \
             so interpolation happens per-item."
        );
    }
}

pub(super) async fn run_command_blocking(
    command: &str,
    args: Option<&[String]>,
) -> Result<super::CommandOutput> {
    if sniff::programs::find_program(command).is_none() {
        return Err(crate::error::ClaudineError::LinkingError(format!(
            "command not found on PATH: {command}"
        )));
    }

    let mut cmd = Command::new(command);
    if let Some(args) = args {
        cmd.args(args);
    }
    crate::child_environment::contribute_child_environment(&mut cmd)
        .map_err(std::io::Error::other)?;

    let output = cmd.output().await?;
    Ok(super::CommandOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_words_preserves_spaces_in_quoted_params() {
        let raw = "--message 'my tool'";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "my tool"]);
    }

    #[test]
    fn shell_words_splits_unquoted_spaces() {
        let raw = "--message my tool";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "my", "tool"]);
    }

    #[test]
    fn shell_words_handles_metacharacters_safely() {
        let raw = "--path /tmp/$(whoami)";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--path", "/tmp/$(whoami)"]);
    }

    #[test]
    fn shell_words_handles_quotes_in_values() {
        let raw = r#"--message "it's a test""#;
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "it's a test"]);
    }

    #[test]
    fn shell_words_empty_params_produces_no_args() {
        let raw = "";
        let args: Vec<String> = if raw.is_empty() {
            vec![]
        } else {
            shell_words::split(raw).unwrap()
        };
        assert!(args.is_empty());
    }

    #[tokio::test]
    async fn blocking_command_child_observes_agent_cwd() {
        let expected = crate::child_environment::initialize_process_launch_directory(
            crate::child_environment::LaunchDirectoryMode::Ordinary,
        )
        .unwrap()
        .to_string_lossy()
        .into_owned();
        #[cfg(windows)]
        let output = run_command_blocking(
            "cmd.exe",
            Some(&["/D".to_string(), "/C".to_string(), "echo %AGENT_CWD%".to_string()]),
        )
        .await
        .unwrap();
        #[cfg(not(windows))]
        let output = run_command_blocking(
            "sh",
            Some(&["-c".to_string(), "printf %s \"$AGENT_CWD\"".to_string()]),
        )
        .await
        .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout.trim(), expected);
    }

    // =========================================================================
    // Bash execution security tests
    // =========================================================================

    #[test]
    fn validate_command_advisory_for_rm() {
        // `BLOCKED_COMMANDS` is an advisory speed bump only; `rm` is no
        // longer rejected here. Real command gating belongs to
        // `ProtectService`.
        if which::which("rm").is_ok() {
            assert!(bash_executor::validate_command("rm").is_ok());
        }
    }

    #[test]
    fn validate_command_allows_echo() {
        assert!(bash_executor::validate_command("echo").is_ok());
    }

    #[test]
    fn shell_escape_neutralizes_injection() {
        let escaped = bash_executor::shell_escape("$(rm -rf /)");
        assert_eq!(escaped, "'$(rm -rf /)'");
    }

    #[test]
    fn shell_escape_handles_semicolon_injection() {
        let escaped = bash_executor::shell_escape("; rm -rf /");
        assert_eq!(escaped, "'; rm -rf /'");
    }

    #[test]
    fn shell_escape_handles_backtick_injection() {
        let escaped = bash_executor::shell_escape("`rm -rf /`");
        assert_eq!(escaped, "'`rm -rf /`'");
    }
}
