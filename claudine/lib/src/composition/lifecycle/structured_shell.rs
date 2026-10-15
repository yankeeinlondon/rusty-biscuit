//! Direct dispatch of a filled template command for the lifecycle runner.
//!
//! The contract is the lifecycle `shell` runner's, with only argument
//! construction changed: children inherit this process's standard streams and
//! working directory, start from the same environment the system-shell route
//! gives them, and get no deadline and no process group of their own. Each
//! program is resolved to a native executable before the first segment
//! starts; a target an interpreter would re-parse is refused, and nothing
//! falls back to a shell. `&&` and `||` short-circuit here, as a shell would.

use std::process::Stdio;

use darkmatter::markdown::compose::shell_expansion::template::{FilledChain, resolve_direct_target};
use darkmatter::markdown::compose::shell_expansion::types::{
    ChainOperator, RedirectionConfig, StderrTarget, StdoutTarget,
};

use super::executor::ShellRunError;
use crate::invocation_context::EnvBaseline;

/// Run `chain`, every program resolved before any starts, and return the exit
/// code of the last segment that ran.
///
/// `environment` is the complete child environment, or `None` to inherit this
/// process's; its `PATH` is the one each program is found on.
pub(super) fn run_filled_chain(
    chain: &FilledChain,
    environment: Option<&EnvBaseline>,
) -> Result<i32, ShellRunError> {
    let search_path = environment.and_then(|environment| environment.get("PATH"));
    // A relative program is found from the directory the child inherits,
    // which is Claudine's launch directory: the process never changes it.
    let cwd = crate::child_environment::initialize_process_launch_directory(
        crate::child_environment::LaunchDirectoryMode::Ordinary,
    )
    .map(std::path::Path::to_path_buf)
    .unwrap_or_default();
    let targets = chain
        .segments
        .iter()
        .map(|segment| {
            resolve_direct_target(&segment.program, search_path, &cwd).map_err(|reason| ShellRunError::Target {
                program: segment.program.clone(),
                reason: Box::new(reason),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut status = 0;
    for (segment, target) in chain.segments.iter().zip(targets) {
        let runs = match segment.operator {
            ChainOperator::None => true,
            ChainOperator::And => status == 0,
            ChainOperator::Or => status != 0,
        };
        if !runs {
            continue;
        }
        let spawn_failed = |source| ShellRunError::Spawn {
            // The program only: a filled argument may be a secret.
            command: segment.program.clone(),
            source,
        };
        let mut command = match environment {
            Some(environment) => {
                crate::child_environment::command_with_environment(&target, &environment.to_child_map())
            }
            None => crate::child_environment::command(&target),
        }
        .map_err(|error| spawn_failed(std::io::Error::other(error)))?;
        command.args(&segment.args);
        apply_redirection(&mut command, &segment.redirection);
        status = command.status().map_err(spawn_failed)?.code().unwrap_or(-1);
    }
    Ok(status)
}

/// The curated redirections over inherited streams: `> /dev/null` and
/// `2> /dev/null` discard, `>&2` and `2>&1` send one stream to the other.
fn apply_redirection(command: &mut std::process::Command, redirection: &RedirectionConfig) {
    match redirection.stdout {
        StdoutTarget::Capture => {}
        StdoutTarget::Null => {
            command.stdout(Stdio::null());
        }
        StdoutTarget::ToStderr => {
            command.stdout(std::io::stderr());
        }
    }
    match redirection.stderr {
        StderrTarget::Capture => {}
        StderrTarget::Null => {
            command.stderr(Stdio::null());
        }
        StderrTarget::ToStdout => {
            command.stderr(std::io::stdout());
        }
    }
}
