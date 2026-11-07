use clap::{ArgAction, Args};
use clap_complete::engine::ArgValueCompleter;
use crate::args::policy::{self, PolicyCommand, PolicySelector, PolicyValueParser, PolicyVerb};
use crate::args::output::{ALL_TARGETS, target_parser};
use crate::args::{Feature, complete_indent_values, complete_markdown_files, parse_indent_size, parse_max_width};
use biscuit_terminal::output::OutputTarget;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::PathBuf;

/// Options shared by default rendering and the explicit `render` subcommand.
///
/// No field has a default: an absent option is `None`, an empty vector, or
/// `false`, so [`RenderOptions::has_explicit_arguments`] can tell a supplied
/// value from an omitted one. Defaults are resolved after dispatch.
#[derive(Clone, Debug, Args)]
pub struct RenderOptions {
    /// The input file (`-` for stdin) and `style…=VALUE` setters, in any order
    #[arg(value_name = "ARGS", num_args = 0.., add = ArgValueCompleter::new(complete_markdown_files))]
    pub args: Vec<OsString>,

    /// Arguments after `--`: always the input file, never a setter or a command
    // A separate positional so the classifier knows which tokens followed `--`.
    #[arg(value_name = "INPUT", num_args = 0.., last = true, add = ArgValueCompleter::new(complete_markdown_files))]
    pub literal_args: Vec<OsString>,

    /// A JSON, JSON5, YAML, or TOML style file (repeatable; later files merge
    /// over earlier ones)
    #[arg(long = "style", value_name = "FILE", action = ArgAction::Append)]
    pub style: Vec<OsString>,

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

    /// Render mermaid diagrams to terminal as images, or as text drawings
    /// where the terminal shows no images; other diagrams stay code blocks.
    #[arg(long)]
    pub mermaid: bool,
}

impl RenderOptions {
    /// The spelling of every render argument this value carries explicitly, in
    /// declaration order; empty when nothing was supplied.
    ///
    /// A root value before a subcommand is a usage error, so every field must
    /// be listed here; `args::render::tests` enumerates the clap definition to
    /// keep that true.
    pub fn has_explicit_arguments(&self) -> Vec<&'static str> {
        let Self { args, literal_args, style, output, show, max_width, indent, deny, features, mermaid } = self;
        [
            (!args.is_empty(), "[ARGS]"),
            (!literal_args.is_empty(), "-- [INPUT]"),
            (!style.is_empty(), "--style"),
            (output.is_some(), "--output"),
            (*show, "--show"),
            (max_width.is_some(), "--max-width"),
            (indent.is_some(), "--indent"),
            (!deny.is_empty(), "--deny"),
            (!features.is_empty(), "--feature"),
            (*mermaid, "--mermaid"),
        ]
        .into_iter()
        .filter_map(|(present, name)| present.then_some(name))
        .collect()
    }
}

/// One `style…=VALUE` word, not yet parsed: Phase 3's style preparation parses `key` with `PropertyPath`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStyleSetter {
    /// The whole word as written.
    pub argument: String,
    /// The text before the first `=`: `style` or `style.…`.
    pub key: String,
    /// The text after the first `=`, which may itself contain `=`.
    pub value: String,
}

/// The render positionals split into the one input and the style setters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderInput {
    /// The input file as the OS gave it, or `None` for no input.
    pub input: Option<PathBuf>,
    /// The setters in command-line order.
    pub setters: Vec<RawStyleSetter>,
}

/// A render positional list that names more than one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderArgsError {
    /// Every input token, in command-line order.
    MultipleInputs(Vec<OsString>),
}

impl RenderArgsError {
    /// This error as the usage error (exit 2) `md` reports.
    pub fn usage_error(&self) -> clap::Error {
        use clap::CommandFactory as _;
        crate::args::Cli::command().error(clap::error::ErrorKind::TooManyValues, self.to_string())
    }
}

impl fmt::Display for RenderArgsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MultipleInputs(inputs) => {
                let inputs: Vec<_> = inputs.iter().map(|input| input.to_string_lossy()).collect();
                write!(formatter, "expected at most one input file, but got: {}", inputs.join(", "))
            }
        }
    }
}

impl std::error::Error for RenderArgsError {}

impl From<RenderArgsError> for clap::Error {
    fn from(error: RenderArgsError) -> Self {
        error.usage_error()
    }
}

/// Splits render positionals into the input and the style setters.
///
/// A token of `args` is a setter only when it is Unicode and the text before
/// its first `=` is `style` or starts with `style.`; every other token, and
/// every token of `literal_args` (those after `--`), is an input kept as the
/// OS spelled it.
///
/// ## Errors
///
/// More than one input token.
pub fn classify_render_args(args: &[OsString], literal_args: &[OsString]) -> Result<RenderInput, RenderArgsError> {
    let mut inputs = Vec::new();
    let mut setters = Vec::new();
    for token in args {
        match as_setter(token) {
            Some(setter) => setters.push(setter),
            None => inputs.push(token.clone()),
        }
    }
    inputs.extend(literal_args.iter().cloned());
    if inputs.len() > 1 {
        return Err(RenderArgsError::MultipleInputs(inputs));
    }
    Ok(RenderInput { input: inputs.pop().map(PathBuf::from), setters })
}

fn as_setter(token: &OsStr) -> Option<RawStyleSetter> {
    let argument = token.to_str()?;
    let (key, value) = argument.split_once('=')?;
    (key == "style" || key.starts_with("style.")).then(|| RawStyleSetter {
        argument: argument.to_string(),
        key: key.to_string(),
        value: value.to_string(),
    })
}

/// The setter spelling to suggest for an input that did not resolve, when it
/// looks like a misspelled setter: its key (the text before the first `=`)
/// contains a `.`. The first key segment is replaced with `style`, so
/// `stlye.ul.x=4` suggests `style.ul.x=4`.
pub fn setter_hint(input: &OsStr) -> Option<String> {
    let text = input.to_str()?;
    let (key, value) = text.split_once('=')?;
    let (_, rest) = key.split_once('.')?;
    Some(format!("style.{rest}={value}"))
}

#[cfg(test)]
mod tests;
