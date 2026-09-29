//! `policy` — check and renew Markdown content policies.

use clap::Parser;

/// Check and renew Markdown content policies.
#[derive(Debug, Parser)]
#[command(name = "policy", version, about)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
