//! `question` — CLI front end for the tui-chrome input components.
//!
//! Phase 0 scaffold: parses args and prints help. Individual commands
//! (text-input, boolean-switch, choose-one, ...) land in later phases.

use clap::{Parser, Subcommand};

/// Interactive prompts backed by tui-chrome components.
#[derive(Debug, Parser)]
#[command(name = "question", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {}

fn main() {
    let _cli = Cli::parse();
}
