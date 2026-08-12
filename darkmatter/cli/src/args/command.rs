use clap::Subcommand;
use clap_complete::engine::ArgValueCompleter;
use crate::args::{
    CodeBlockOutput, HashKind, OutputFormat, RemoteFreshness, SchemaTarget, ValidateTarget,
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
    Render {
        /// Input file path (use "-" for stdin)
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: Option<PathBuf>,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
        output: OutputFormat,

        /// Open output in the default app using a temp file
        #[arg(long)]
        show: bool,

        /// Normalize nested list indentation width (spaces per level)
        #[arg(
            long,
            value_name = "#",
            value_parser = parse_indent_size,
            add = ArgValueCompleter::new(complete_indent_values)
        )]
        indent: Option<usize>,
    },

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

        /// Remove all blank lines between list items
        #[arg(long, conflicts_with = "loose")]
        compact: bool,

        /// Add blank lines between all list items
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

        /// Preserve source single newlines instead of collapsing incidental wrapping
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
        #[arg(long, value_name = "PATH", conflicts_with = "no_baseline_schema")]
        baseline_schema: Option<PathBuf>,

        /// Disable the default Darkmatter baseline frontmatter schema
        #[arg(long)]
        no_baseline_schema: bool,

        /// Disable trigger discovery and bare-name schema-root lookup
        #[arg(long)]
        no_trigger_schemas: bool,
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

        /// Output format (default: markdown for compose)
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        output: OutputFormat,

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

        /// Allow missing hyperlink targets (emit content, report errors on stderr)
        #[arg(long)]
        allow_missing_hyperlinks: bool,

        /// Allow missing image reference targets (emit content, report errors on stderr)
        #[arg(long)]
        allow_missing_image_refs: bool,

        /// Allow missing transclusion targets (remove directive, report errors on stderr)
        #[arg(long)]
        allow_missing_transclusions: bool,

        /// Allow any missing reference (combines all --allow-missing-* flags)
        #[arg(long)]
        allow_any_missing_reference: bool,

        /// Allow non-object ctx frontmatter (downgrade error to warning)
        #[arg(long)]
        allow_ctx_override: bool,

        /// Allow invalid set= RHS on ::file directives (downgrade error to warning; sibling valid set clauses still apply)
        #[arg(long)]
        allow_invalid_frontmatter_assignment: bool,

        /// Allow duplicate set.NAME= assignments on ::file directives (downgrade error to warning; rightmost wins)
        #[arg(long)]
        allow_reassigned_frontmatter_property: bool,

        /// Baseline SimplifiedSchema YAML file for compose-time frontmatter validation
        #[arg(long, value_name = "PATH", conflicts_with = "no_baseline_schema")]
        baseline_schema: Option<PathBuf>,

        /// Disable the default Darkmatter baseline frontmatter schema
        #[arg(long)]
        no_baseline_schema: bool,

        /// Disable trigger discovery and bare-name schema-root lookup
        #[arg(long)]
        no_trigger_schemas: bool,

        /// Global shell command timeout in seconds (default: 10)
        #[arg(long, value_name = "SECONDS")]
        timeout: Option<u64>,

        /// Convert shell timeout failures into empty strings instead of errors
        #[arg(long)]
        allow_shell_timeout: bool,

        /// Report condition-blind shell approval candidates (every command that
        /// could run under any state, including dead branches) without executing them
        #[arg(long)]
        shell: bool,

        /// Emit a compose performance report to stderr after completion
        #[arg(long)]
        perf: bool,

        /// Allow remote reads from the specified host (repeatable)
        #[arg(long, value_name = "HOST")]
        allow_host: Vec<String>,

        /// Maximum concurrent remote fetches (default: 16, or $DARKMATTER_REMOTE_CONCURRENCY)
        #[arg(long, value_name = "N")]
        remote_concurrency: Option<usize>,

        /// Remote artifact TTL in seconds (default: use server cache headers)
        #[arg(long, value_name = "SECONDS")]
        remote_ttl: Option<u64>,

        /// Force revalidation of cached remote artifacts
        #[arg(long)]
        remote_refresh: bool,

        /// Remote freshness mode (default: fallback)
        #[arg(long, value_enum, default_value_t = RemoteFreshness::Fallback)]
        remote_freshness: RemoteFreshness,

        /// Persistent compose cache root (enables remote URL artifact caching)
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
    /// Resolves the file using biscuit-file's FileReference system (supports `@`, `!`,
    /// `vault:`, etc.). Creates the file if it doesn't exist. Blocks until the editor
    /// exits. Returns the fully qualified filename on success.
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
