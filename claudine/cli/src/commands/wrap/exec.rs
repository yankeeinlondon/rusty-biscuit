use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};

use color_eyre::eyre::Result;

/// Spawn the provider child process and return its exit code.
///
/// ## Environment
///
/// The `env` parameter must be the **complete** environment for the child
/// process. The child is launched with `env_clear()` followed by `envs(env)`,
/// so any variable not present in `env` will be absent from the child. This
/// is the only gate for environment sanitization — if a variable is missing
/// from `env`, it will not reach the child.
///
/// ## Signal Handling
///
/// If Claudine receives a second SIGINT (Ctrl-C) while waiting for the child,
/// it sends SIGTERM to the child. A third SIGINT sends SIGKILL.
///
/// ## Timeout
///
/// When `timeout` is `Some(seconds)`, the child is sent SIGTERM after the
/// specified duration, followed by SIGKILL after a 5-second grace period.
pub(crate) fn run_child(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
) -> Result<i32> {
    // Debug assertion: critical variables must be present.
    debug_assert!(
        env.contains_key(&OsString::from("PATH")),
        "child env is missing PATH — env::build_child_env likely has a bug"
    );
    debug_assert!(
        env.contains_key(&OsString::from("HOME")),
        "child env is missing HOME — env::build_child_env likely has a bug"
    );

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = command.spawn()?;

    let exit_code = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child)?
    };

    Ok(exit_code)
}

/// Wait for the child, forwarding SIGINT/SIGTERM on repeated Ctrl-C.
#[cfg(unix)]
fn wait_with_signal_handling(child: &mut Child) -> Result<i32> {
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::Arc;

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_pid = child.id();

    // Install a SIGINT handler that escalates on repeated presses.
    let counter = Arc::clone(&interrupt_count);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            match count {
                1 => {
                    // First Ctrl-C: let the default behavior propagate to child
                    // (process group signal). Do nothing extra.
                }
                2 => {
                    // Second Ctrl-C: send SIGTERM to child
                    libc::kill(child_pid as i32, libc::SIGTERM);
                }
                _ => {
                    // Third+ Ctrl-C: send SIGKILL to child
                    libc::kill(child_pid as i32, libc::SIGKILL);
                }
            }
        })
    }?;

    let status = child.wait()?;
    Ok(exit_code_from_status(status))
}

#[cfg(not(unix))]
fn wait_with_signal_handling(child: &mut Child) -> Result<i32> {
    let status = child.wait()?;
    Ok(exit_code_from_status(status))
}

/// Wait for the child with a timeout, sending SIGTERM then SIGKILL.
#[cfg(unix)]
fn wait_with_timeout(child: &mut Child, seconds: u64) -> Result<i32> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let grace_period = Duration::from_secs(5);

    loop {
        match child.try_wait()? {
            Some(status) => return Ok(exit_code_from_status(status)),
            None => {
                if Instant::now() >= deadline {
                    // Send SIGTERM
                    unsafe {
                        libc::kill(child.id() as i32, libc::SIGTERM);
                    }

                    // Wait for grace period
                    let kill_deadline = Instant::now() + grace_period;
                    loop {
                        match child.try_wait()? {
                            Some(status) => return Ok(exit_code_from_status(status)),
                            None => {
                                if Instant::now() >= kill_deadline {
                                    // Send SIGKILL
                                    unsafe {
                                        libc::kill(child.id() as i32, libc::SIGKILL);
                                    }
                                    let status = child.wait()?;
                                    return Ok(exit_code_from_status(status));
                                }
                                std::thread::sleep(Duration::from_millis(100));
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

#[cfg(not(unix))]
fn wait_with_timeout(child: &mut Child, seconds: u64) -> Result<i32> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);

    loop {
        match child.try_wait()? {
            Some(status) => return Ok(exit_code_from_status(status)),
            None => {
                if Instant::now() >= deadline {
                    child.kill()?;
                    let status = child.wait()?;
                    return Ok(exit_code_from_status(status));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
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
