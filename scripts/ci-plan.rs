//! Render a resolved CI plan for a human about to trigger a workflow.
//!
//! A thin CLI over [`repo_deps::ci_plan_render::render_document`]: it reads the
//! plan file named on the command line and prints the rendering.
//!
//! ## Notes
//!
//! `--plan` uses this renderer only when it is already built, because a
//! pre-trigger review must not start a build.

use std::env;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use biscuit_terminal::prelude::Terminal;
use repo_deps::ci_plan_render::render_document;

fn plan_path() -> Result<String> {
    env::args()
        .nth(1)
        .context("usage: ci-plan <resolved-plan.json>")
}

fn main() -> ExitCode {
    let term = Terminal::new();
    let rendered = match plan_path().and_then(|path| {
        // Read as text so a non-UTF-8 file is still reported as a read failure.
        let text = fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        render_document(text.as_bytes(), &term).with_context(|| format!("parsing {path}"))
    }) {
        Ok(rendered) => rendered,
        Err(error) => {
            eprintln!("ci-plan: {error:#}");
            return ExitCode::from(2);
        }
    };
    print!("{rendered}");
    // The refusal belongs to the caller: `just ci-local --plan` owns the exit
    // status so the rule holds whether or not this renderer is installed.
    ExitCode::SUCCESS
}
