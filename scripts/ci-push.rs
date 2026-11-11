//! `ci-push`: the pre-push hook's decision maker.
//!
//! Invoked as `ci-push [<remote name> [<remote url>]]` with Git's ref-update
//! lines on stdin. It reads the environment and the complete stdin, hands them
//! to [`repo_deps::ci_push::run`], which reports as it goes, and exits with
//! the outcome's status.

use std::env;
use std::io::{self, Read};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use repo_deps::ci_push::{self, HostServices, PushRequest, process};

fn request() -> Result<PushRequest> {
    let mut arguments = env::args_os().skip(1);
    let mut text = |what: &str| -> Result<Option<String>> {
        match arguments.next() {
            None => Ok(None),
            Some(value) => match value.into_string() {
                Ok(value) if value.is_empty() => Ok(None),
                Ok(value) => Ok(Some(value)),
                Err(value) => bail!("the {what} {value:?} is not valid UTF-8"),
            },
        }
    };
    let remote_name = text("remote name")?.unwrap_or_else(|| "origin".to_owned());
    let remote_url = text("remote URL")?;
    let mut input = Vec::new();
    io::stdin()
        .lock()
        .read_to_end(&mut input)
        .context("cannot read the ref updates on stdin")?;
    Ok(PushRequest {
        cwd: env::current_dir().context("cannot read the current directory")?,
        remote_name,
        remote_url,
        input,
        mode: env::var_os("RUSTY_BISCUIT_PRE_PUSH"),
        selection: env::var_os("RUSTY_BISCUIT_PRE_PUSH_AREAS"),
    })
}

fn main() -> ExitCode {
    // Before the first child: a signal must reach ci-push, which ends and
    // reaps its children and removes its temporary trees before exiting.
    if let Err(error) = process::install_cancellation_handlers() {
        eprintln!("Push blocked: cannot install the interruption handlers: {error}");
        return ExitCode::FAILURE;
    }
    let outcome = request().and_then(|request| ci_push::run(request, &mut HostServices));
    let code = match outcome {
        Ok(outcome) => outcome.exit_code,
        Err(error) => {
            eprintln!("Push blocked: {error:#}");
            1
        }
    };
    // An interruption outranks whatever the run concluded.
    ExitCode::from(process::exit_code_for_cancellation().unwrap_or(code))
}
