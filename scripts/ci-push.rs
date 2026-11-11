//! `ci-push`: the pre-push hook's decision maker.
//!
//! Invoked as `ci-push [<remote name> [<remote url>]]` with Git's ref-update
//! lines on stdin. It reads the environment and the complete stdin, hands them
//! to [`repo_deps::ci_push::run`], prints the outcome, and exits with its
//! status.

use std::env;
use std::io::{self, Read};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use repo_deps::ci_push::{self, HostServices, PushRequest};

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
    let outcome = request().and_then(|request| ci_push::run(request, &mut HostServices));
    match outcome {
        Ok(outcome) => {
            for diagnostic in &outcome.diagnostics {
                eprintln!("{diagnostic}");
            }
            ExitCode::from(outcome.exit_code)
        }
        Err(error) => {
            eprintln!("Push blocked: {error:#}");
            ExitCode::FAILURE
        }
    }
}
