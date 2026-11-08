use clap::Subcommand;
use clap_complete::engine::ArgValueCompleter;
use crate::args::policy::{self, PolicyCommand, PolicySelector, PolicyValueParser, PolicyVerb};
use crate::args::output::{ALL_TARGETS, COMPOSE_TARGETS, target_parser};
use crate::args::path_completion;
use biscuit_terminal::output::OutputTarget;
use crate::args::{
    HashKind, RemoteFreshness, SchemaTarget, ValidateTarget,
    complete_indent_values,
    complete_markdown_files, complete_theme_names, parse_indent_size, parse_positive_limit,
    parse_theme_name,
};
use darkmatter::markdown::highlighting::ThemePair;
use std::path::PathBuf;

/// CLI subcommands.
#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Render a markdown document (same as default behavior without a subcommand).
    Render(crate::args::RenderOptions),

    /// Clean up markdown formatting.
    Clean {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: Option<PathBuf>,

        /// Save cleaned markdown in place and report delta-style changes
        #[arg(long)]
        save: bool,

        /// Normalize nested list indentation width (spaces per level)
        #[arg(
            long,
            value_name = "#",
            value_parser = parse_indent_size,
            add = ArgValueCompleter::new(complete_indent_values)
        )]
        indent: Option<usize>,

        /// Remove blank lines between sibling list items (`list-item-padding: none`)
        #[arg(long, conflicts_with = "loose")]
        compact: bool,

        /// Put one blank line between sibling list items (`list-item-padding: one`)
        #[arg(long, conflicts_with = "compact")]
        loose: bool,

        /// What to print: the cleaned document, or its frontmatter diagnostics
        /// in a data format
        #[arg(long, value_name = "TARGET", default_value = "markdown", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,

        /// Explicit schema replacing the document's own `$schema` layer
        #[arg(long, value_name = "PATH")]
        schema: Option<PathBuf>,

        /// Baseline SimplifiedSchema YAML file for frontmatter validation
        #[arg(long, value_name = "PATH")]
        baseline_schema: Option<PathBuf>,

        /// Stop a feature from running (repeatable)
        #[arg(
            long = "disable",
            value_name = "ID",
            value_parser = PolicyValueParser::new(PolicyCommand::Clean, PolicyVerb::Disable),
            long_help = policy::long_help(PolicyCommand::Clean, PolicyVerb::Disable),
            add = ArgValueCompleter::new(|current: &std::ffi::OsStr| policy::complete(PolicyCommand::Clean, PolicyVerb::Disable, current))
        )]
        disable: Vec<PolicySelector>,
    },

    /// Compose a document through the compose pipeline.
    Compose {
        /// Positional arguments: input path and/or key=value setters
        #[arg(
            value_name = "ARGS",
            num_args = 0..,
            add = ArgValueCompleter::new(path_completion::complete_compose_word)
        )]
        args: Vec<String>,

        /// Default values as JSON; fills in null/missing frontmatter keys without overriding existing values
        #[arg(long, value_name = "JSON")]
        state: Option<String>,

        /// Override values as JSON; overwrites existing frontmatter keys with the provided values
        #[arg(long, value_name = "JSON")]
        set: Option<String>,

        /// What to print: the composed Markdown, or the envelope (the document and
        /// every problem) in a data format; `md render` turns Markdown into anything else
        #[arg(long, value_name = "TARGET", default_value = "markdown", value_parser = target_parser(COMPOSE_TARGETS))]
        output: OutputTarget,

        /// Print the output AND open it in the default app
        #[arg(long)]
        show: bool,

        /// Include frontmatter in the output (default: body only)
        #[arg(long, visible_alias = "fm")]
        frontmatter: bool,

        /// Remove all blank lines between list items
        #[arg(long, conflicts_with = "loose")]
        compact: bool,

        /// Add blank lines between all list items
        #[arg(long, conflicts_with = "compact")]
        loose: bool,

        /// Normalize nested list indentation width (spaces per level)
        #[arg(
            long,
            value_name = "#",
            value_parser = parse_indent_size,
            add = ArgValueCompleter::new(complete_indent_values)
        )]
        indent: Option<usize>,

        /// Baseline SimplifiedSchema YAML file for compose-time frontmatter validation
        #[arg(long, value_name = "PATH")]
        baseline_schema: Option<PathBuf>,

        /// Permit a recovery or reads from a named host (repeatable; `all` for every recovery)
        #[arg(
            long = "allow",
            value_name = "ID",
            value_parser = PolicyValueParser::new(PolicyCommand::Compose, PolicyVerb::Allow),
            long_help = policy::long_help(PolicyCommand::Compose, PolicyVerb::Allow),
            add = ArgValueCompleter::new(|current: &std::ffi::OsStr| policy::complete(PolicyCommand::Compose, PolicyVerb::Allow, current))
        )]
        allow: Vec<PolicySelector>,

        /// Make the selected diagnostics blocking (repeatable; `all` for every warning)
        #[arg(
            long = "deny",
            value_name = "ID",
            value_parser = PolicyValueParser::new(PolicyCommand::Compose, PolicyVerb::Deny),
            long_help = policy::long_help(PolicyCommand::Compose, PolicyVerb::Deny),
            add = ArgValueCompleter::new(|current: &std::ffi::OsStr| policy::complete(PolicyCommand::Compose, PolicyVerb::Deny, current))
        )]
        deny: Vec<PolicySelector>,

        /// Stop a feature from running (repeatable)
        #[arg(
            long = "disable",
            value_name = "ID",
            value_parser = PolicyValueParser::new(PolicyCommand::Compose, PolicyVerb::Disable),
            long_help = policy::long_help(PolicyCommand::Compose, PolicyVerb::Disable),
            add = ArgValueCompleter::new(|current: &std::ffi::OsStr| policy::complete(PolicyCommand::Compose, PolicyVerb::Disable, current))
        )]
        disable: Vec<PolicySelector>,

        /// Global shell command timeout in seconds (default: 10)
        #[arg(long, value_name = "SECONDS")]
        timeout: Option<u64>,

        /// Source items one request may visit across every `::map` / `::filter` (library default applies when omitted)
        #[arg(
            long,
            value_name = "N",
            value_parser = parse_positive_limit,
            allow_negative_numbers = true
        )]
        max_iteration_visits: Option<std::num::NonZeroU64>,

        /// Bytes of collection expansion one request may materialize (library default applies when omitted)
        #[arg(
            long,
            value_name = "N",
            value_parser = parse_positive_limit,
            allow_negative_numbers = true
        )]
        max_expansion_bytes: Option<std::num::NonZeroU64>,

        /// Report condition-blind shell approval candidates (every command that
        /// could run under any state, including dead branches) without executing them
        #[arg(long)]
        shell: bool,

        /// Emit a compose performance report to stderr after completion
        #[arg(long)]
        perf: bool,

        /// Maximum concurrent remote fetches (default: 16, or $DARKMATTER_REMOTE_CONCURRENCY)
        #[arg(long, value_name = "N")]
        remote_concurrency: Option<usize>,

        /// Freshness lifetime in seconds for transport-cached remote bodies (default: server `max-age`); never makes a `no-store` response storable or a `no-cache` response fresh
        #[arg(long, value_name = "SECONDS")]
        remote_ttl: Option<u64>,

        /// Revalidate every transport-cached remote body with a conditional GET, even when fresh
        #[arg(long)]
        remote_refresh: bool,

        /// Remote freshness mode (default: fallback)
        #[arg(long, value_enum, default_value_t = RemoteFreshness::Fallback)]
        remote_freshness: RemoteFreshness,

        /// Transport-artifact cache root: persists raw remote URL response bodies only
        ///
        /// Semantic results (composed documents, `::file` children, `::code` /
        /// `::toc-linking` output) are never persisted. Nothing is created on
        /// disk until a storable remote response is written. A
        /// `Cache-Control: no-store` response is never written. A cache root
        /// never authorizes a host; use `--allow host=HOST`.
        #[arg(long, value_name = "DIR")]
        cache_root: Option<PathBuf>,
    },

    /// List the themes `style.page.theme` and `style.page.code.theme` accept.
    ListThemes {
        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Show markdown table of contents.
    Toc {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT")]
        input: Option<PathBuf>,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Compare two markdown documents.
    Delta {
        /// Base/original markdown file
        #[arg(value_name = "BASE")]
        base: PathBuf,

        /// Updated markdown file
        #[arg(value_name = "UPDATED")]
        updated: PathBuf,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Get frontmatter values from a markdown document, by property path.
    Get {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Property paths to retrieve: `author.name`, `items.0`, and `a\.b` for a key with a dot in its name
        #[arg(value_name = "PROP", required = true, num_args = 1.., add = ArgValueCompleter::new(path_completion::complete_get_path))]
        props: Vec<String>,

        /// What to print. `plain` writes raw values: strings unquoted, null as
        /// empty, lists as one item per line, objects as "key: value" lines
        #[arg(long, value_name = "TARGET", default_value = "json", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,

        /// Print arrays and objects on a single line; valid only with `--output json` or `--output json5`
        #[arg(long)]
        compact: bool,
    },

    /// Set the value at a frontmatter property path in a markdown document.
    Set {
        /// Input file path (use "-" for stdin; outputs to stdout)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Property path to set: `author.name`, `items.0`, and `a\.b` for a key with a dot in its name
        #[arg(value_name = "PROP", add = ArgValueCompleter::new(path_completion::complete_set_path))]
        prop: String,

        /// Value to set: JSON5 or a YAML flow mapping, otherwise text, read through the type the schema declares
        #[arg(value_name = "VALUE", add = ArgValueCompleter::new(path_completion::complete_set_value))]
        value: String,

        /// Write the change back to the source file (no formatted output)
        #[arg(long)]
        save: bool,

        /// What to print: the edited document, or its result in a data format
        #[arg(long, value_name = "TARGET", default_value = "markdown", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Remove the values at one or more frontmatter property paths from a markdown document.
    Rm {
        /// Input file path (supports @ file references)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Property paths to remove: `author.name`, `items.0`, and `a\.b` for a key with a dot in its name
        #[arg(value_name = "PROP", required = true, num_args = 1.., add = ArgValueCompleter::new(path_completion::complete_rm_path))]
        props: Vec<String>,

        /// What to print; a data format reports the removed, remaining, and filename fields
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Open a markdown file in your preferred editor.
    ///
    /// Resolves the file using biscuit-file's FileReference system (supports `@`, `&`,
    /// `^`, `vault:`, etc.). Creates the file if it doesn't exist. Blocks until the
    /// editor exits. Returns the fully qualified filename on success.
    ///
    /// Editor priority: $EDITOR > $VISUAL > first installed from default list.
    Edit {
        /// File path or reference to edit
        #[arg(value_name = "FILE", add = ArgValueCompleter::new(complete_markdown_files))]
        file: String,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Validate references in a markdown document.
    Validate {
        /// Validation target
        #[command(subcommand)]
        target: ValidateTarget,
    },

    /// Hash a markdown document's frontmatter and body.
    Hash {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: Option<PathBuf>,

        /// Hash kind to compute
        #[arg(long, value_enum, conflicts_with_all = ["body", "frontmatter"])]
        kind: Option<HashKind>,

        /// Only output the body/prose hash (shorthand for `--kind body`)
        #[arg(long, conflicts_with = "frontmatter")]
        body: bool,

        /// Only output the frontmatter hash (shorthand for `--kind fm`)
        #[arg(long, visible_alias = "fm", conflicts_with = "body")]
        frontmatter: bool,

        /// Write the computed hash back into the document's frontmatter
        #[arg(long, conflicts_with = "diff")]
        save: bool,

        /// Report how the document differs from its stored hash (exits 1 on difference)
        #[arg(long, conflicts_with = "save")]
        diff: bool,

        /// Strict mode: no whitespace normalization or key reordering
        #[arg(long)]
        strict: bool,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Schema authoring and validation commands.
    Schema {
        /// Schema sub-command
        #[command(subcommand)]
        target: SchemaTarget,
    },

    /// Visualize a markdown file's dependency graph.
    Graph {
        /// Input file path or file reference
        #[arg(value_name = "FILE", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Recursively expand followable transclusions
        #[arg(long, visible_alias = "compose")]
        follow: bool,

        /// Validate references and show inline status
        #[arg(long)]
        validate: bool,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },

    /// Render a single code block from a file or literal content.
    ///
    /// Builds a [`CodeBlock`](darkmatter::markdown::CodeBlock) directly and
    /// renders it, bypassing the Markdown fold. The `INPUT` positional is
    /// either a file path or literal content; by default the CLI prefers a
    /// filesystem match (so a real file path wins) and falls back to literal
    /// content when the path does not exist. `--file` and `--content` force
    /// the interpretation when the heuristic would be wrong (e.g. the
    /// literal content happens to match an existing filename).
    CodeBlock {
        /// Source file path or literal code content.
        #[arg(value_name = "INPUT")]
        input: String,

        /// Force `INPUT` to be treated as a file path (error if missing).
        #[arg(long, conflicts_with = "content")]
        file: bool,

        /// Force `INPUT` to be treated as literal content.
        #[arg(long, conflicts_with = "file")]
        content: bool,

        /// Language for the code block (e.g. `rust`, `py`, `yaml`).
        ///
        /// Resolved through
        /// [`LanguageGrammar::from_token_or_plain_text`](darkmatter::markdown::LanguageGrammar::from_token_or_plain_text);
        /// aliases such as `yml`→`yaml`, `py`→`python`, `ts`→`typescript` are
        /// honored. Overrides the extension-derived language for file input.
        #[arg(long, value_name = "LANG")]
        language: Option<String>,

        /// Code theme override (kebab-case name).
        #[arg(long, value_name = "THEME", value_parser = parse_theme_name, add = ArgValueCompleter::new(complete_theme_names))]
        theme: Option<ThemePair>,

        /// Optional title for the code block (printed in the header row).
        #[arg(long, value_name = "TITLE")]
        title: Option<String>,

        /// Show line numbers in the code block.
        #[arg(long)]
        line_numbering: bool,

        /// Highlighted line ranges, e.g. `1,4-6` (single lines and ranges
        /// may be mixed).
        #[arg(long, value_name = "RANGE")]
        highlight: Option<String>,

        /// What to print
        #[arg(long, value_name = "TARGET", default_value = "terminal", value_parser = target_parser(ALL_TARGETS))]
        output: OutputTarget,
    },
}
