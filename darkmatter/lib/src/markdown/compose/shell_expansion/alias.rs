//! Shell alias resolution.
//!
//! Resolves shell aliases to their underlying commands by querying the user's
//! login shell (`$SHELL`). This allows `::shell` directives to use common
//! aliases like `ll`, `la`, etc.
//!
//! ## Job-control hazard
//!
//! Reading aliases requires an *interactive* shell (`-i`), because that is what
//! makes a shell source the rc file the aliases are defined in. Interactive mode
//! also enables job control, and a job-control shell whose process group is not
//! the foreground process group of its controlling terminal signals itself with
//! `SIGTTIN` and is stopped by the kernel — it never exits, so a caller waiting
//! on its stdout waits forever. The shell signals its whole process group, which
//! it inherits from us, so an unprotected caller can be stopped alongside it.
//! Being in the background is the normal case for
//! anything spawned by a test harness or a subprocess chain (nextest, for one,
//! puts every test binary in its own process group), which is why
//! [`spawn_alias_query`] gives the child its own session via `setsid`.
//!
//! The hazard is invisible without a terminal: with no controlling terminal at
//! all (CI, most agent harnesses) job control cannot engage and the shell exits
//! in microseconds. Re-verify by hand under a PTY:
//!
//! ```text
//! script -qc "cargo nextest run -p claudine-cli -E 'test(compose_preflight_discovers_shell_inside_false_block)'" /dev/null
//! ```

use super::executor::{WaitOutcome, wait_with_timeout};
use super::tokenize::ShellToken;
use super::tokenize::tokenize;
use shared_child::SharedChild;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

/// Upper bound on how long the shell may take to report an alias definition.
///
/// Generous for the work involved — the shell sources the user's rc files and
/// runs one builtin, a couple of milliseconds on a typical host. The bound
/// exists so that a future job-control regression degrades to "not an alias"
/// (the same outcome the caller already handles for a genuine non-alias) rather
/// than hanging a compose forever.
const ALIAS_LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);

/// Result of resolving a shell alias.
#[derive(Debug, Clone)]
pub struct ResolvedAlias {
    /// The alias name that was resolved (e.g., "ll").
    pub alias_name: String,
    /// The resolved executable (e.g., "eza").
    pub executable: String,
    /// The resolved arguments from the alias definition.
    pub args: Vec<String>,
    /// The raw alias definition string (e.g., "eza -lhga --git").
    pub definition: String,
}

/// Attempts to resolve a shell alias to its underlying command.
///
/// Queries the user's login shell for the alias definition, tokenizes
/// it, and verifies the resolved executable exists on PATH.
///
/// ## Returns
///
/// `Some(ResolvedAlias)` if the name is a valid shell alias that resolves
/// to an executable on PATH. `None` if the name is not an alias, the shell
/// cannot be queried, or the resolved command is not found.
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::alias::resolve_alias;
///
/// if let Some(resolved) = resolve_alias("ll") {
///     println!("ll resolves to: {} {:?}", resolved.executable, resolved.args);
/// }
/// ```
pub fn resolve_alias(name: &str) -> Option<ResolvedAlias> {
    let shell = std::env::var("SHELL").ok()?;
    resolve_alias_with_shell(&shell, name, ALIAS_LOOKUP_TIMEOUT)
}

/// [`resolve_alias`] with the shell and time budget injected, so tests can drive
/// both the success and the timeout arm against a stub shell.
fn resolve_alias_with_shell(shell: &str, name: &str, timeout: Duration) -> Option<ResolvedAlias> {
    if !is_valid_alias_name(name) {
        return None;
    }

    let alias_output = query_alias(shell, name, timeout)?;
    let definition = parse_alias_value(&alias_output, name)?;

    // Tokenize the alias value using our safe tokenizer (rejects metacharacters)
    let synthetic_ctx = biscuit_terminal::errors::SourceContext::new(
        std::path::PathBuf::from("<alias>"),
        std::path::PathBuf::from("<alias>"),
        definition.clone(),
    );
    let shell_tokens = tokenize(&definition, &synthetic_ctx).ok()?;
    let tokens: Vec<String> = shell_tokens
        .into_iter()
        .filter_map(|t| match t {
            ShellToken::Word(w) => Some(w),
            _ => None,
        })
        .collect();
    if tokens.is_empty() {
        return None;
    }

    if which::which(&tokens[0]).is_err() {
        return None;
    }

    Some(ResolvedAlias {
        alias_name: name.to_string(),
        executable: tokens[0].clone(),
        args: tokens[1..].to_vec(),
        definition,
    })
}

/// Asks the shell for one alias definition, or `None` if it cannot answer.
///
/// Every failure — spawn error, non-zero exit, timeout, empty output — collapses
/// to `None`, which callers already treat as "not an alias".
fn query_alias(shell: &str, name: &str, timeout: Duration) -> Option<String> {
    let child = spawn_alias_query(shell, name).map(Arc::new).ok()?;

    let stdout = child.take_stdout();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stdout) = stdout {
            let _ = stdout.read_to_end(&mut buf);
        }
        buf
    });

    // On the timeout arm `wait_with_timeout` has already killed and reaped the
    // child; leaving `reader` unjoined lets it observe the resulting EOF on its
    // own rather than making this function's bound depend on it.
    match wait_with_timeout(&child, timeout) {
        Ok(WaitOutcome::Exited(status)) if status.success() => {}
        _ => return None,
    }

    let stdout = reader.join().ok()?;
    let alias_output = String::from_utf8_lossy(&stdout).trim().to_string();
    if alias_output.is_empty() {
        return None;
    }
    Some(alias_output)
}

/// Spawns `<shell> -ic "alias <name>"` detached from any controlling terminal.
///
/// See the module docs for why the detachment is load-bearing. `setsid` is the
/// mechanism because it is the only one that removes the controlling terminal:
/// merely giving the child a new *process group* (`CommandExt::process_group`)
/// leaves the terminal attached, so the shell still finds itself outside the
/// foreground process group and still stops. Measured under a PTY, both the
/// unmodified spawn and the `process_group(0)` variant reach state `T` within
/// 20ms; the `setsid` variant exits normally.
///
/// A session with no controlling terminal cannot support job control, so the
/// shell disables it (warning on stderr, which is discarded) and proceeds to run
/// the builtin. `-i` still selects interactive mode, so rc files are still
/// sourced and aliases are still defined.
fn spawn_alias_query(shell: &str, name: &str) -> std::io::Result<SharedChild> {
    let mut cmd = Command::new(shell);
    cmd.args(["-ic", &format!("alias {}", name)])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: `pre_exec` runs in the forked child between `fork` and `exec`,
        // where only async-signal-safe calls are legal. `setsid` is on POSIX's
        // async-signal-safe list, and nothing else here allocates, locks, or
        // touches inherited state. `setsid` can only fail with `EPERM` when the
        // caller is already a process group leader, which a freshly forked child
        // never is.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    SharedChild::spawn(&mut cmd)
}

/// Validates that a name is safe to use in an alias lookup command.
///
/// Only allows alphanumeric characters, hyphens, underscores, and dots.
fn is_valid_alias_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Extracts the alias value from shell `alias` command output.
///
/// Handles both bash and zsh formats:
/// - zsh: `ll='eza -lhga --git'` or `ll=eza`
/// - bash: `alias ll='eza -lhga --git'`
fn parse_alias_value(output: &str, name: &str) -> Option<String> {
    // Strip optional "alias " prefix (bash format)
    let def = output.strip_prefix("alias ").unwrap_or(output);

    // Find value after "name="
    let prefix = format!("{}=", name);
    let value = def.strip_prefix(&prefix)?;

    // Strip surrounding quotes
    let value = value.trim();
    let unquoted = if (value.starts_with('\'') && value.ends_with('\''))
        || (value.starts_with('"') && value.ends_with('"'))
    {
        &value[1..value.len() - 1]
    } else {
        value
    };

    if unquoted.is_empty() {
        return None;
    }

    Some(unquoted.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_alias_names() {
        assert!(is_valid_alias_name("ll"));
        assert!(is_valid_alias_name("ls"));
        assert!(is_valid_alias_name("my-alias"));
        assert!(is_valid_alias_name("my_alias"));
        assert!(is_valid_alias_name("alias.name"));
        assert!(is_valid_alias_name("ls2"));
    }

    #[test]
    fn invalid_alias_names() {
        assert!(!is_valid_alias_name(""));
        assert!(!is_valid_alias_name("ls;rm"));
        assert!(!is_valid_alias_name("foo bar"));
        assert!(!is_valid_alias_name("$(whoami)"));
        assert!(!is_valid_alias_name("foo|bar"));
        assert!(!is_valid_alias_name("a>b"));
    }

    #[test]
    fn parse_zsh_single_quoted_alias() {
        let output = "ll='eza -lhga --git --hyperlink --group'";
        let value = parse_alias_value(output, "ll");
        assert_eq!(
            value.as_deref(),
            Some("eza -lhga --git --hyperlink --group")
        );
    }

    #[test]
    fn parse_zsh_double_quoted_alias() {
        let output = "ll=\"eza -lhga --git\"";
        let value = parse_alias_value(output, "ll");
        assert_eq!(value.as_deref(), Some("eza -lhga --git"));
    }

    #[test]
    fn parse_zsh_unquoted_alias() {
        let output = "ll=eza";
        let value = parse_alias_value(output, "ll");
        assert_eq!(value.as_deref(), Some("eza"));
    }

    #[test]
    fn parse_bash_format_alias() {
        let output = "alias ll='eza -lhga --git --hyperlink --group'";
        let value = parse_alias_value(output, "ll");
        assert_eq!(
            value.as_deref(),
            Some("eza -lhga --git --hyperlink --group")
        );
    }

    #[test]
    fn parse_alias_wrong_name_returns_none() {
        let output = "la='ls -la'";
        let value = parse_alias_value(output, "ll");
        assert!(value.is_none());
    }

    #[test]
    fn parse_alias_empty_value_returns_none() {
        let output = "ll=''";
        let value = parse_alias_value(output, "ll");
        assert!(value.is_none());
    }

    /// Writes an executable stand-in for `$SHELL` that ignores its `-ic` args.
    #[cfg(unix)]
    fn stub_shell(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("stub-shell");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn resolves_an_alias_reported_by_the_shell() {
        let dir = tempfile::tempdir().unwrap();
        let shell = stub_shell(dir.path(), "echo \"ll='ls -l'\"");

        let resolved =
            resolve_alias_with_shell(shell.to_str().unwrap(), "ll", Duration::from_secs(10))
                .expect("stub shell reports ll as an alias");

        assert_eq!(resolved.alias_name, "ll");
        assert_eq!(resolved.executable, "ls");
        assert_eq!(resolved.args, vec!["-l".to_string()]);
        assert_eq!(resolved.definition, "ls -l");
    }

    /// A shell that never exits must not stall the caller — the regression that
    /// a stopped, job-control-seeking interactive shell used to produce.
    #[cfg(unix)]
    #[test]
    fn a_shell_that_never_answers_times_out_to_none() {
        let dir = tempfile::tempdir().unwrap();
        let shell = stub_shell(dir.path(), "sleep 120");

        let started = std::time::Instant::now();
        let resolved =
            resolve_alias_with_shell(shell.to_str().unwrap(), "ll", Duration::from_millis(200));

        assert!(resolved.is_none());
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "alias lookup did not honor its timeout: {:?}",
            started.elapsed()
        );
    }

    /// `setsid` must not cost us the shell's stdout: a child in a fresh session
    /// still writes down the same pipe.
    #[cfg(unix)]
    #[test]
    fn detached_child_output_still_reaches_the_caller() {
        let dir = tempfile::tempdir().unwrap();
        let shell = stub_shell(dir.path(), "echo \"alias ll='ls -l'\"");

        let output = query_alias(shell.to_str().unwrap(), "ll", Duration::from_secs(10));

        assert_eq!(output.as_deref(), Some("alias ll='ls -l'"));
    }

    #[cfg(unix)]
    #[test]
    fn shell_exiting_non_zero_is_not_an_alias() {
        let dir = tempfile::tempdir().unwrap();
        let shell = stub_shell(dir.path(), "exit 1");

        assert!(query_alias(shell.to_str().unwrap(), "ll", Duration::from_secs(10)).is_none());
    }

    /// Integration test: resolve a known alias from the current shell.
    /// Ignored by default since it depends on the user's shell configuration.
    #[test]
    #[ignore]
    fn resolve_alias_ll() {
        if let Some(resolved) = resolve_alias("ll") {
            println!(
                "ll -> {} {:?} (definition: {})",
                resolved.executable, resolved.args, resolved.definition
            );
            assert!(!resolved.executable.is_empty());
        } else {
            println!("ll is not aliased in the current shell");
        }
    }
}
