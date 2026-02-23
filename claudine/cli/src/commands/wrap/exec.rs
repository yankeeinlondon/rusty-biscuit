use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use color_eyre::eyre::Result;

/// Spawn the provider child process and return its exit code.
pub(crate) fn run_child(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
) -> Result<i32> {
    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = command.status()?;
    Ok(exit_code_from_status(status))
}

fn exit_code_from_status(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    1
}
