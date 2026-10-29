use clap::Subcommand;
use clap_complete::engine::ArgValueCompleter;
use crate::args::policy::{self, PolicyCommand, PolicySelector, PolicyValueParser, PolicyVerb};
use crate::args::{
    CodeBlockOutput, ComposeOutputFormat, HashKind, RemoteFreshness, SchemaTarget, ValidateTarget,
    complete_compose_args, complete_fixed_width_values, complete_indent_values,
    complete_markdown_files, complete_theme_names, parse_fixed_width, parse_indent_size,
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

        /// Re-wrap prose to the specified display width in columns
        #[arg(
            long,
            value_name = "#",
            value_parser = parse_fixed_width,
            conflicts_with = "ignore_incidental_newlines",
            add = ArgValueCompleter::new(complete_fixed_width_values)
        )]
        fixed_width: Option<usize>,

        /// Keep source single newlines, overriding a document's `unwrap` and `fixed-width`
        ///
        /// The original spec proposed `--ignore-incidental-carraige-returns`;
        /// this ships as `--ignore-incidental-newlines` because Markdown uses
        /// line feeds semantically and the original spelling had a typo.
        #[arg(long, conflicts_with = "fixed_width")]
        ignore_incidental_newlines: bool,

        /// Emit frontmatter diagnostics as JSON instead of the cleaned document
        #[arg(long)]
        json: bool,

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
            add = ArgValueCompleter::new(complete_compose_args)
        )]
        args: Vec<String>,

        /// Default values as JSON; fills in null/missing frontmatter keys without overriding existing values
        #[arg(long, value_name = "JSON")]
        state: Option<String>,

        /// Override values as JSON; overwrites existing frontmatter keys with the provided values
        #[arg(long, value_name = "JSON")]
        set: Option<String>,

        /// Output format (default: markdown; `report-json` prints the composition report)
        #[arg(long, value_enum, default_value_t = ComposeOutputFormat::Markdown)]
        output: ComposeOutputFormat,

        /// Render composed content to stdout AND open in default app
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

    /// Show markdown table of contents.
    Toc {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT")]
        input: Option<PathBuf>,

        /// Output TOC as JSON
        #[arg(long)]
        json: bool,
    },

    /// Compare two markdown documents.
    Delta {
        /// Base/original markdown file
        #[arg(value_name = "BASE")]
        base: PathBuf,

        /// Updated markdown file
        #[arg(value_name = "UPDATED")]
        updated: PathBuf,

        /// Output delta as JSON
        #[arg(long)]
        json: bool,
    },

    /// Get frontmatter properties from a markdown document.
    Get {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Frontmatter property names to retrieve
        #[arg(value_name = "PROP", required = true, num_args = 1..)]
        props: Vec<String>,

        /// Output as JSON5
        #[arg(long)]
        json5: bool,

        /// Output as YAML
        #[arg(long)]
        yaml: bool,

        /// Output as TOML
        #[arg(long)]
        toml: bool,

        /// Output raw values: strings unquoted, null as empty, lists as one item per line, objects as "key: value" lines
        #[arg(long, conflicts_with_all = ["json5", "yaml", "toml", "compact"])]
        raw: bool,

        /// Output JSON on a single line for arrays and objects
        #[arg(long, conflicts_with_all = ["json5", "yaml", "toml", "raw"])]
        compact: bool,
    },

    /// Set a frontmatter property on a markdown document.
    Set {
        /// Input file path (use "-" for stdin; outputs to stdout)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Frontmatter property name to set
        #[arg(value_name = "PROP")]
        prop: String,

        /// Value to set (parsed as JSON if valid, otherwise treated as a string)
        #[arg(value_name = "VALUE")]
        value: String,

        /// Write the change back to the source file (no output)
        #[arg(long)]
        save: bool,
    },

    /// Remove one or more frontmatter properties from a markdown document.
    Rm {
        /// Input file path (supports @ file references)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Property names to remove
        #[arg(value_name = "PROP", required = true, num_args = 1..)]
        props: Vec<String>,

        /// Output result as JSON with removed, remaining, and filename fields
        #[arg(long)]
        json: bool,
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

        /// Report how the document differs from its stored hash (exits 2 on difference)
        #[arg(long, conflicts_with = "save")]
        diff: bool,

        /// Strict mode: no whitespace normalization or key reordering
        #[arg(long)]
        strict: bool,
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

        /// Output as JSON instead of a terminal tree
        #[arg(long)]
        json: bool,
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

        /// Output format.
        #[arg(long, value_enum, default_value_t = CodeBlockOutput::Terminal, value_name = "FORMAT")]
        output: CodeBlockOutput,
    },
}
