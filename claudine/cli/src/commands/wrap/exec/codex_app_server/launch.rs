//! Mapping a `codex exec` argv onto a managed `codex app-server` launch.
//!
//! `codex exec` is itself an in-process client of the app-server: it starts a
//! thread with `approvalPolicy: never`, runs one turn, and answers server
//! requests without a human. A managed launch reproduces that with the same
//! settings, so each `exec` option must have an exact app-server equivalent:
//!
//! - global config (`-c`/`--config`, `--enable`, `--disable`,
//!   `--strict-config`) passes to `app-server` unchanged;
//! - `--model`, `--sandbox`, `--dangerously-bypass-approvals-and-sandbox`,
//!   and `--ephemeral` become `thread/start` parameters, and `exec resume
//!   <id>` becomes `thread/resume`;
//! - `--output-last-message` is written by the session itself, as exec does;
//! - `--json` and `--color` only shape exec's own printing.
//!
//! Anything else (an image, an output schema, a profile, a local provider, a
//! positional prompt, an unknown flag) has no verified equivalent, so the
//! launch stays on `codex exec`. So does a run exec itself would refuse:
//! outside a Git repository without `--skip-git-repo-check` or the bypass
//! flag, exec exits before starting, and the app-server has no such check.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// A managed launch equivalent to one `codex exec` argv.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ManagedLaunch {
    /// The `app-server` argv.
    pub(crate) args: Vec<String>,
    /// The original `exec` argv: the fallback before the task is submitted.
    pub(crate) exec_args: Vec<String>,
    pub(crate) thread: ThreadRequest,
    /// Where exec would have written the final agent message.
    pub(crate) last_message: Option<PathBuf>,
}

/// How the managed launch obtains its thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ThreadRequest {
    Start { params: Value },
    Resume { thread_id: String, params: Value },
}

/// Why an argv stays on `codex exec`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Unmapped {
    #[error("`{0}` has no verified app-server equivalent")]
    Option(String),
    #[error("the argv is not a `codex exec` run")]
    NotExec,
    #[error("`exec {0}` has no verified app-server equivalent")]
    Subcommand(String),
    #[error("`{0}` needs a value")]
    MissingValue(String),
    #[error("`{0}` is not a sandbox mode")]
    Sandbox(String),
    #[error("exec refuses to run outside a Git repository without `--skip-git-repo-check`")]
    Untrusted,
}

/// Global options `app-server` takes exactly as `exec` does.
const PASSTHROUGH_VALUES: &[&str] = &["-c", "--config", "--enable", "--disable"];
const SANDBOX_MODES: &[&str] = &["read-only", "workspace-write", "danger-full-access"];

/// Maps `args` (a `codex exec` argv, without the binary) run in `cwd`.
pub(crate) fn plan(args: &[String], cwd: &Path) -> Result<ManagedLaunch, Unmapped> {
    let mut tokens = args.iter().map(String::as_str).peekable();
    if !matches!(tokens.next(), Some("exec" | "e")) {
        return Err(Unmapped::NotExec);
    }
    let mut resume = None;
    if let Some(&subcommand) = tokens.peek()
        && !subcommand.starts_with('-')
    {
        tokens.next();
        if subcommand != "resume" {
            return Err(Unmapped::Subcommand(subcommand.to_string()));
        }
        match tokens.next() {
            Some(id) if !id.starts_with('-') => resume = Some(id.to_string()),
            Some(flag) => return Err(Unmapped::Option(format!("resume {flag}"))),
            None => return Err(Unmapped::MissingValue("resume".into())),
        }
    }

    let mut server_args = vec!["app-server".to_string(), "--listen".to_string(), "stdio://".to_string()];
    let mut params = Map::new();
    let mut last_message = None;
    let mut skip_git_check = false;
    let mut bypass = false;
    let mut sandbox = None;
    while let Some(token) = tokens.next() {
        let (flag, inline) = match token.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_string())),
            _ => (token, None),
        };
        let mut value = || match &inline {
            Some(value) => Ok(value.clone()),
            None => tokens.next().map(str::to_string).ok_or_else(|| Unmapped::MissingValue(flag.to_string())),
        };
        match flag {
            "--json" => {}
            "--color" => {
                value()?;
            }
            "--skip-git-repo-check" => skip_git_check = true,
            "--dangerously-bypass-approvals-and-sandbox" => bypass = true,
            "--strict-config" => server_args.push(flag.to_string()),
            _ if PASSTHROUGH_VALUES.contains(&flag) => {
                let value = value()?;
                server_args.extend([flag.to_string(), value]);
            }
            "-m" | "--model" => {
                params.insert("model".into(), Value::from(value()?));
            }
            "-s" | "--sandbox" => {
                let mode = value()?;
                if !SANDBOX_MODES.contains(&mode.as_str()) {
                    return Err(Unmapped::Sandbox(mode));
                }
                sandbox = Some(mode);
            }
            "--ephemeral" => {
                params.insert("ephemeral".into(), Value::Bool(true));
            }
            "-o" | "--output-last-message" => last_message = Some(PathBuf::from(value()?)),
            _ => return Err(Unmapped::Option(flag.to_string())),
        }
    }
    if bypass {
        sandbox = Some("danger-full-access".to_string());
    } else if !skip_git_check && !in_git_repository(cwd) {
        return Err(Unmapped::Untrusted);
    }
    if let Some(sandbox) = sandbox {
        params.insert("sandbox".into(), Value::from(sandbox));
    }
    // exec never asks for approval; neither does its managed equivalent.
    params.insert("approvalPolicy".into(), Value::from("never"));
    if cwd.is_absolute() {
        params.insert("cwd".into(), Value::from(cwd.to_string_lossy().into_owned()));
    }
    let params = Value::Object(params);
    Ok(ManagedLaunch {
        args: server_args,
        exec_args: args.to_vec(),
        thread: match resume {
            Some(thread_id) => ThreadRequest::Resume { thread_id, params },
            None => ThreadRequest::Start { params },
        },
        last_message,
    })
}

/// exec's trust check: `cwd` or an ancestor holds a `.git` entry.
fn in_git_repository(cwd: &Path) -> bool {
    cwd.ancestors().any(|dir| dir.join(".git").exists())
}
