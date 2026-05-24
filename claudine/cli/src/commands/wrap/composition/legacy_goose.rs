use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::path::Path;

use biscuit_terminal::terminal::Terminal;
use claudine::provider::Provider;
use color_eyre::eyre::Result;

use super::{CompositionExecutionMode, StructuredCodexOutput};
use crate::commands::wrap::exec;
use crate::commands::wrap::profile::WrapperProfile;
use crate::commands::wrap::subagent_watchdog::TimeoutConfig;

/// Run the legacy (non-structured) branch of [`super::execute_without_harness`].
///
/// Inline mode must capture the response so the closure plan can rewrite the
/// target file; direct mode just runs the child and lets it write to stdout
/// on its own.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_legacy_branch(
    mode: CompositionExecutionMode<'_>,
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &Path,
    stdin_seed: Option<&str>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    structured_codex_output: Option<&StructuredCodexOutput>,
    child_spawned: &mut bool,
    agent_perf_out: &mut Option<crate::perf::AgentExecutionPerf>,
    timeout_config: TimeoutConfig,
    term: &Terminal,
) -> Result<(i32, String, Option<super::CompositionStreamResult>)> {
    if timeout_config.any_enabled() {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::components::status::{Status, StatusState};
        let rendered = Status::new(
            "timeouts are only enforced in structured-stream mode; \
             ignoring for this non-structured attempt"
                .to_string(),
        )
        .state(StatusState::Warning)
        .render(term);
        eprintln!("{rendered}");
    }
    match mode {
        CompositionExecutionMode::Inline {
            session_interactive,
            ..
        } => {
            if session_interactive {
                let result = exec::run_child(
                    binary_path,
                    child_args,
                    child_env,
                    child_cwd,
                    None,
                    exec::ChildIoOptions {
                        stdout_noise_prefixes: stdout_noise,
                        stderr_noise_prefixes: stderr_noise,
                        stdin_seed,
                    },
                    child_spawned,
                )?;
                *agent_perf_out = Some(result.telemetry.into_agent_perf(None));
                let response = if provider == Provider::Codex {
                    if let Some(output) = structured_codex_output {
                        let text =
                            std::fs::read_to_string(&output.last_message_path).unwrap_or_default();
                        let _ = std::fs::remove_file(&output.last_message_path);
                        text
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                Ok((result.data, response, None))
            } else {
                let mut capture_args = child_args.to_vec();
                profile.prepare_captured_output(&mut capture_args);
                let capture = exec::run_child_capture(
                    binary_path,
                    &capture_args,
                    child_env,
                    child_cwd,
                    None,
                    exec::ChildIoOptions {
                        stdout_noise_prefixes: stdout_noise,
                        stderr_noise_prefixes: stderr_noise,
                        stdin_seed,
                    },
                    child_spawned,
                )?;
                *agent_perf_out = Some(capture.telemetry.into_agent_perf(None));
                let response = profile.parse_captured_output(&capture.data.stdout);
                if !response.trim().is_empty() {
                    if std::io::stdout().is_terminal() {
                        let rendered = crate::output::render_assistant_markdown(&response, term);
                        std::io::stdout().write_all(rendered.as_bytes())?;
                        if !rendered.ends_with('\n') {
                            std::io::stdout().write_all(b"\n")?;
                        }
                    } else {
                        std::io::stdout().write_all(response.as_bytes())?;
                        if !response.ends_with('\n') {
                            std::io::stdout().write_all(b"\n")?;
                        }
                    }
                    std::io::stdout().flush()?;
                }
                if !capture.data.stderr.trim().is_empty() {
                    eprintln!("{}", capture.data.stderr);
                }
                Ok((capture.data.exit_code, response, None))
            }
        }
        CompositionExecutionMode::Direct => {
            let result = exec::run_child(
                binary_path,
                child_args,
                child_env,
                child_cwd,
                None,
                exec::ChildIoOptions {
                    stdout_noise_prefixes: stdout_noise,
                    stderr_noise_prefixes: stderr_noise,
                    stdin_seed,
                },
                child_spawned,
            )?;
            *agent_perf_out = Some(result.telemetry.into_agent_perf(None));
            Ok((result.data, String::new(), None))
        }
    }
}
