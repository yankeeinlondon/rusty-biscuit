use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use crate::args::policy::{self, PolicyCommand, PolicySelector, PolicyValueParser, PolicyVerb};
use crate::args::output::{ALL_TARGETS, target_parser};
use crate::args::{Feature, complete_indent_values, complete_markdown_files, parse_indent_size, parse_max_width};
use biscuit_terminal::output::OutputTarget;
use std::path::PathBuf;

/// Options shared by default rendering and the explicit `render` subcommand.
#[derive(Clone, Debug, Args)]
pub struct RenderOptions {
    /// Input file path (use "-" for stdin)
    #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
    pub input: Option<PathBuf>,

    /// What to print [default: terminal, which is the same text as `plain` off
    /// a color terminal]
    #[arg(long, value_name = "TARGET", value_parser = target_parser(ALL_TARGETS))]
    pub output: Option<OutputTarget>,

    /// Open output in the default app using a temp file; a data output is also
    /// printed, and a terminal output opens the document's Markdown
    #[arg(long)]
    pub show: bool,

    /// Max content width in columns (0 rejected); it caps the content inside
    /// `--width`
    #[arg(long, value_name = "N", value_parser = parse_max_width)]
    pub max_width: Option<u16>,

    /// Normalize nested list indentation width (spaces per level)
    #[arg(
        long,
        value_name = "#",
        value_parser = parse_indent_size,
        add = ArgValueCompleter::new(complete_indent_values)
    )]
    pub indent: Option<usize>,

    /// Make the selected diagnostics blocking (repeatable; `all` for every warning)
    #[arg(
        long = "deny",
        value_name = "ID",
        value_parser = PolicyValueParser::new(PolicyCommand::Render, PolicyVerb::Deny),
        long_help = policy::long_help(PolicyCommand::Render, PolicyVerb::Deny),
        add = ArgValueCompleter::new(|current: &std::ffi::OsStr| policy::complete(PolicyCommand::Render, PolicyVerb::Deny, current))
    )]
    pub deny: Vec<PolicySelector>,

    /// Turn on a named feature (repeatable and comma-separated; names are
    /// case-insensitive)
    #[arg(
        long = "feature",
        value_name = "NAME",
        value_enum,
        value_delimiter = ',',
        ignore_case = true,
        action = clap::ArgAction::Append
    )]
    pub features: Vec<Feature>,
}
