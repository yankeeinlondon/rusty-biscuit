//! `question` — CLI front end for the tui-chrome input components.
//!
//! Each supported component is a subcommand. The global `--output`
//! flag selects the serialisation format applied to the submitted
//! value.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod commands;
mod output;

use commands::boolean_switch::{BooleanSwitchArgs, run as run_boolean_switch};
use commands::choose_many::{ChooseManyArgs, run as run_choose_many};
use commands::choose_one::{ChooseOneArgs, run as run_choose_one};
use commands::text_area_input::{TextAreaInputArgs, run as run_text_area_input};
use commands::text_input::{TextInputArgs, run as run_text_input};
use output::OutputMode;

/// Interactive prompts backed by tui-chrome components.
#[derive(Debug, Parser)]
#[command(name = "question", version, about, long_about = None)]
struct Cli {
    /// Output serialisation applied to the submitted value.
    #[arg(long, value_enum, default_value_t = OutputMode::Raw, global = true)]
    output: OutputMode,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Single-line text input.
    TextInput(TextInputArgs),
    /// Multi-line text area editor.
    TextAreaInput(TextAreaInputArgs),
    /// Boolean toggle switch.
    BooleanSwitch(BooleanSwitchArgs),
    /// Single-selection list.
    ChooseOne(ChooseOneArgs),
    /// Multi-selection list.
    ChooseMany(ChooseManyArgs),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("question: {e}");
            ExitCode::from(1)
        }
    }
}

fn dispatch(cli: Cli) -> std::io::Result<i32> {
    match cli.command {
        Commands::TextInput(args) => run_text_input(args, cli.output),
        Commands::TextAreaInput(args) => run_text_area_input(args, cli.output),
        Commands::BooleanSwitch(args) => run_boolean_switch(args, cli.output),
        Commands::ChooseOne(args) => run_choose_one(args, cli.output),
        Commands::ChooseMany(args) => run_choose_many(args, cli.output),
    }
}
