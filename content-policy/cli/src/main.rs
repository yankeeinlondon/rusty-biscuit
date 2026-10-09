//! `policy` — check and renew Markdown content policies.
//!
//! Exit codes: `0` when a report or plan was produced (or written), `1` for
//! any error, `2` for usage errors (from clap).

mod args;
mod commands;
mod output;

use std::io::Write as _;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;

use crate::args::{Cli, Command};

fn main() -> ExitCode {
    CompleteEnv::with_factory(Cli::command).complete();
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Check(args) => commands::check(args),
        Command::Renew(args) => commands::renew(args),
    };
    match result {
        Ok(text) => {
            let mut stdout = std::io::stdout().lock();
            // A closed pipe (`policy check … | head`) is not an error.
            let _ = stdout.write_all(text.as_bytes()).and_then(|()| stdout.flush());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
