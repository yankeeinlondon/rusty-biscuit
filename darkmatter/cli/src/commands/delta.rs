//! `md delta` subcommand implementation.

use crate::args::Cli;
use crate::io::load_markdown;
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::request::MdRequest;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use color_eyre::eyre::{Context, Result};
use darkmatter::markdown::MarkdownDelta;
use darkmatter::markdown::delta::DeltaReport;
use darkmatter::markdown::output::command::{CommandValue, code_document};
use renderable::{OutputOptions, TerminalProfile, Width};
use std::path::PathBuf;

pub(super) fn run_delta(base: &PathBuf, updated: &PathBuf, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let run = CommandRun::begin(output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Object)?;
    let result = delta_value(base, updated, cli.verbose > 0, request);
    run.finish(result)
}

fn delta_value(base: &PathBuf, updated: &PathBuf, verbose: bool, request: &MdRequest) -> Result<Completed<CommandValue<MarkdownDelta>>> {
    let base_md = load_markdown(Some(base), request).wrap_err_with(|| format!("Failed to read base file: {base:?}"))?;
    let updated_md =
        load_markdown(Some(updated), request).wrap_err_with(|| format!("Failed to read updated file: {updated:?}"))?;
    let delta = base_md.delta(&updated_md);

    let mut report = DeltaReport::new(delta.clone()).with_documents(base_md, updated_md);
    if verbose {
        report = report.verbose();
    }
    let render = move |options: &OutputOptions| {
        // The report paints without asking its terminal.
        let text = report.render(&Terminal::from_output_options(options));
        if options.terminal_profile.degrades_to_plain() { strip_escape_codes(text) } else { text }
    };
    let plain = render(&OutputOptions::new(Width::Unlimited, TerminalProfile::plain()));
    let prose = code_document(None, plain.trim_end());
    Ok(Completed::success(CommandValue::new(delta, prose).with_terminal(render)))
}
