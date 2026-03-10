use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;

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
///
/// ## Stdout Filtering
///
/// When `stdout_noise_prefixes` is non-empty, stdout is piped through a
/// filter that suppresses lines starting with any of the given prefixes.
/// This is used in non-interactive mode to strip provider debug noise
/// (e.g. Gemini CLI's hook execution logs) from the response.
pub(crate) fn run_child(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    stdout_noise_prefixes: &[&str],
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

    let filtering = !stdout_noise_prefixes.is_empty();

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(if filtering {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stderr(Stdio::inherit());

    let mut child = command.spawn()?;

    // Spawn a filter thread that reads child stdout line-by-line and
    // suppresses lines matching any noise prefix.
    let filter_handle = if filtering {
        let pipe = child.stdout.take().expect("stdout was set to piped");
        let prefixes: Vec<String> = stdout_noise_prefixes.iter().map(|s| s.to_string()).collect();
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut out = std::io::stdout().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
                }
                let _ = writeln!(out, "{line}");
            }
        }))
    } else {
        None
    };

    let exit_code = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child)?
    };

    if let Some(handle) = filter_handle {
        let _ = handle.join();
    }

    Ok(exit_code)
}

/// Wait for the child, forwarding SIGINT/SIGTERM on repeated Ctrl-C.
#[cfg(unix)]
fn wait_with_signal_handling(child: &mut Child) -> Result<i32> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_pid = child.id();

    // Install a SIGINT handler that escalates on repeated presses.
    let counter = Arc::clone(&interrupt_count);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            match count {
                1 => {
                    // First Ctrl-C: forward SIGINT to the child process.
                    // Registering this handler replaced the default behavior
                    // (which would propagate to the process group), so we
                    // must explicitly forward the signal.
                    libc::kill(child_pid as i32, libc::SIGINT);
                }
                2 => {
                    // Second Ctrl-C: escalate to SIGTERM
                    libc::kill(child_pid as i32, libc::SIGTERM);
                }
                _ => {
                    // Third+ Ctrl-C: force kill
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
