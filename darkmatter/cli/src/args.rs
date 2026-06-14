use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use darkmatter::layout::PageBackground;
use darkmatter::markdown::highlighting::{CodeBlockMode, ThemePair};
use renderable::color::{BasicColor, Color, RgbColor, Tailwind};
use renderable::style::PaintColor;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Output format for top-level render mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    /// Auto: terminal rendering on TTY, markdown text on non-TTY.
    Auto,
    /// Markdown text output.
    #[value(alias = "text")]
    Markdown,
    /// Markdown output enhanced with disclosure blocks rendered as HTML
    /// details/summary elements. Only the browser render target produces these
    /// elements; on other targets this falls back to plain Markdown.
    MarkdownPlus,
    /// HTML output.
    #[value(alias = "browser")]
    Html,
    /// AST JSON output.
    #[value(alias = "ast")]
    Json,
}

/// Output format for `md code-block`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CodeBlockOutput {
    /// Render as ANSI-styled terminal output.
    Terminal,
    /// Render as a `<pre><code class="language-…">…</code></pre>` HTML fragment.
    Html,
    /// Re-emit as a Markdown fenced code block.
    Markdown,
}

/// Cache-staleness handling for remote URL artifacts.
///
/// Mirrors `darkmatter::markdown::compose::RemoteFreshnessMode`. A closed value
/// set so an unrecognized `--remote-freshness` argument fails fast with the
/// accepted values rather than silently degrading to a single mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum RemoteFreshness {
    /// Serve any cached artifact without revalidation, even when stale.
    Optimistic,
    /// Always revalidate with a conditional GET.
    Strict,
    /// Serve stale on network failure (the default).
    Fallback,
}

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

        /// Global shell command timeout in seconds (default: 10)
        #[arg(long, value_name = "SECONDS")]
        timeout: Option<u64>,

        /// Convert shell timeout failures into empty strings instead of errors
        #[arg(long)]
        allow_shell_timeout: bool,

        /// Report shell commands discovered in the compose tree without executing them
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
        /// [`LanguageGrammar::from_fence_token`](darkmatter::markdown::LanguageGrammar::from_fence_token);
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

/// Validation sub-targets.
#[derive(Clone, Debug, Subcommand)]
pub enum ValidateTarget {
    /// Validate all references (links, images, transclusions).
    Refs {
        /// Input file path
        #[arg(value_name = "INPUT", add = ArgValueCompleter::new(complete_markdown_files))]
        input: PathBuf,

        /// Enable remote URL validation
        #[arg(long)]
        remote: bool,

        /// Enable fragment validation
        #[arg(long)]
        fragments: bool,

        /// Remote validation timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,

        /// Stop on first error
        #[arg(long)]
        fail_fast: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = ValidateOutputFormat::Text)]
        format: ValidateOutputFormat,

        /// Show all references, not just issues
        #[arg(long = "show-all")]
        show_all: bool,

        /// Print transclusion graph as Mermaid or DOT
        #[arg(long, value_enum)]
        graph: Option<GraphFormat>,
    },
}

/// Output format for validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ValidateOutputFormat {
    /// Human-readable text.
    Text,
    /// JSON output.
    Json,
}

/// Schema sub-targets.
#[derive(Clone, Debug, Subcommand)]
pub enum SchemaTarget {
    /// Validate markdown frontmatter against a schema.
    ///
    /// Positional arguments are either Markdown file paths or
    /// `<prop>=<value>` assignments applied to every document's frontmatter
    /// before validation. An argument is treated as an assignment when it
    /// contains `=` and the text before the first `=` is a dot-separated
    /// property path (e.g. `title=Hello`, `user.email=ken@ken.net`).
    /// File paths that contain `=` should be disambiguated with `./` —
    /// e.g. `./weird=name.md`.
    Validate {
        /// Markdown file paths and `<prop>=<value>` assignments
        #[arg(
            value_name = "FILE_OR_PROP=VALUE",
            required = true,
            num_args = 1..,
            add = ArgValueCompleter::new(complete_compose_args)
        )]
        inputs: Vec<String>,

        /// Baseline schema path (YAML SimplifiedSchema or JSON Schema)
        #[arg(long, value_name = "PATH")]
        schema: Option<PathBuf>,

        /// Output format
        #[arg(long, value_enum, default_value_t = SchemaValidateFormat::Pretty)]
        format: SchemaValidateFormat,

        /// Suppress success lines; only failures print
        #[arg(long)]
        quiet: bool,
    },

    /// Detect a SimplifiedSchema from one or more markdown documents.
    Detect {
        /// Input markdown files
        #[arg(
            value_name = "FILE",
            required = true,
            num_args = 1..,
            add = ArgValueCompleter::new(complete_markdown_files)
        )]
        files: Vec<PathBuf>,

        /// Output format
        #[arg(long, value_enum, default_value_t = SchemaDetectFormat::Yaml)]
        format: SchemaDetectFormat,

        /// Merge multiple files: widen disagreeing types and mark
        /// properties required only if present in every input file.
        #[arg(long)]
        merge: bool,
    },

    /// Print the SimplifiedSchema language reference.
    ///
    /// Renders a human-readable report covering schema shapes, the type
    /// vocabulary, constraints, inline object syntax, validation behaviour,
    /// and compose-time coercion. The report is generated from the typed
    /// schema-language descriptor catalog in
    /// `darkmatter::markdown::schemas`; the same catalog is available to
    /// library callers via the `schema_type_descriptors()`,
    /// `schema_constraint_descriptors()`, `schema_shape_descriptors()`,
    /// `inline_object_rule_descriptors()`, `coercion_rule_descriptors()`,
    /// and `validation_behavior_descriptors()` functions.
    ///
    /// The command is documentation-only: it does not parse documents,
    /// capture context, construct an `EffectEngine`, resolve file
    /// references, or perform network access.
    About,
}

/// Output format for `md schema validate`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SchemaValidateFormat {
    /// Pretty (terminal) output.
    Pretty,
    /// Newline-delimited JSON, one object per file.
    Json,
}

/// Output format for `md schema detect`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SchemaDetectFormat {
    /// SimplifiedSchema YAML.
    Yaml,
    /// JSON Schema (Draft 2020-12).
    Json,
}

/// Graph visualization format.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum GraphFormat {
    /// Mermaid flowchart.
    Mermaid,
    /// DOT (Graphviz) graph.
    Dot,
}

/// Structural kind for `md hash --kind`.
///
/// CLI-side mirror of [`darkmatter::markdown::hash::MdHashKind`]; the tokens
/// match the library's serde/`FromStr` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum HashKind {
    /// Frontmatter only (keys and values).
    Fm,
    /// Body content only.
    Body,
    /// `{fm}-{body}` — the default.
    Simple,
    /// `{fm}-{fm_keys}-{body}-{body_structure}`.
    Structured,
    /// Nested object with per-section detail.
    Detailed,
}

impl From<HashKind> for darkmatter::markdown::hash::MdHashKind {
    fn from(value: HashKind) -> Self {
        use darkmatter::markdown::hash::MdHashKind;
        match value {
            HashKind::Fm => MdHashKind::Fm,
            HashKind::Body => MdHashKind::Body,
            HashKind::Simple => MdHashKind::Simple,
            HashKind::Structured => MdHashKind::Structured,
            HashKind::Detailed => MdHashKind::Detailed,
        }
    }
}

/// Command-line interface for the darkmatter markdown renderer.
///
/// Use `md --help` to see all available options.
#[derive(Parser)]
#[command(name = "md", about = "Darkmatter CLI", version)]
#[command(subcommand_precedence_over_arg = true, disable_help_subcommand = true)]
pub struct Cli {
    /// Input file path (reads from stdin if not provided, use "-" for explicit stdin)
    #[arg(add = ArgValueCompleter::new(complete_markdown_files))]
    pub input: Option<PathBuf>,

    /// Theme for prose content (kebab-case name)
    #[arg(long, value_parser = parse_theme_name, add = ArgValueCompleter::new(complete_theme_names))]
    pub theme: Option<ThemePair>,

    /// Theme for code blocks (overrides derived theme)
    #[arg(long, value_parser = parse_theme_name, add = ArgValueCompleter::new(complete_theme_names))]
    pub code_theme: Option<ThemePair>,

    /// Choose the code block theme variant relative to the page color mode:
    /// `inverse` (default, dark page -> light panel), `dark`, `light`, or `same`.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = CodeBlockArg::Inverse, global = true)]
    pub code_block: CodeBlockArg,

    /// List available themes
    #[arg(long)]
    pub list_themes: bool,

    /// Output format for top-level render mode (when no subcommand given)
    #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
    pub output: OutputFormat,

    /// Open selected output in the default app using a temp file
    #[arg(long)]
    pub show: bool,

    /// Shorthand for `clean --save` with top-level [INPUT]
    #[arg(long)]
    pub save: bool,

    /// Render mermaid diagrams to terminal as images.
    /// Falls back to code blocks if terminal doesn't support images.
    #[arg(long)]
    pub mermaid: bool,

    // ── Layout flags (Phase 5) ──────────────────────────────────────────
    /// Margin on all sides (cells)
    #[arg(short = 'm', long, value_name = "N")]
    pub margin: Option<u16>,

    /// Horizontal margin (left + right)
    #[arg(long, value_name = "N")]
    pub mx: Option<u16>,

    /// Vertical margin (top + bottom)
    #[arg(long, value_name = "N")]
    pub my: Option<u16>,

    /// Top margin
    #[arg(long, visible_alias = "margin-top", value_name = "N")]
    pub mt: Option<u16>,

    /// Bottom margin
    #[arg(long, visible_alias = "margin-bottom", value_name = "N")]
    pub mb: Option<u16>,

    /// Left margin
    #[arg(long, visible_alias = "margin-left", value_name = "N")]
    pub ml: Option<u16>,

    /// Right margin
    #[arg(long, visible_alias = "margin-right", value_name = "N")]
    pub mr: Option<u16>,

    /// Padding on all sides (cells)
    #[arg(long, value_name = "N")]
    pub padding: Option<u16>,

    /// Horizontal padding (left + right)
    #[arg(long, value_name = "N")]
    pub px: Option<u16>,

    /// Vertical padding (top + bottom)
    #[arg(long, value_name = "N")]
    pub py: Option<u16>,

    /// Top padding
    #[arg(long, visible_alias = "padding-top", value_name = "N")]
    pub pt: Option<u16>,

    /// Bottom padding
    #[arg(long, visible_alias = "padding-bottom", value_name = "N")]
    pub pb: Option<u16>,

    /// Left padding
    #[arg(long, visible_alias = "padding-left", value_name = "N")]
    pub pl: Option<u16>,

    /// Right padding
    #[arg(long, visible_alias = "padding-right", value_name = "N")]
    pub pr: Option<u16>,

    /// Page background style
    #[arg(
        long,
        visible_alias = "page-background",
        value_enum,
        value_name = "STYLE"
    )]
    pub page_bg: Option<PageBackgroundArg>,

    /// Explicit page background color (e.g. `#1e1e23` or `30,30,35`).
    ///
    /// Overrides the computed `PageBackground` color when set. Accepts a
    /// hex string (`#RGB` / `#RRGGBB`) or a comma-separated `R,G,B` triple
    /// in the 0-255 range. Tailwind palette names (`red-500`, `slate-50`,
    /// etc.) and CSS special keywords (`transparent`, `inherit`,
    /// `currentColor`) are also accepted.
    #[arg(long, value_name = "COLOR", value_parser = parse_page_bg_color)]
    pub page_bg_color: Option<PaintColor>,

    /// Max content width in columns (0 rejected)
    #[arg(long, value_name = "N", value_parser = parse_max_width)]
    pub max_width: Option<u16>,

    /// Intentionally unsupported width shorthand.
    ///
    /// The terminal-cell width is determined from the captured `Terminal`
    /// and the configured `--max-width`; a plain `--width` flag would
    /// shadow those for no reason. The flag is rejected with a clear
    /// error so callers know to use `--max-width` instead.
    #[arg(long, value_name = "N", value_parser = reject_width_flag)]
    pub width: Option<u16>,

    /// Include line numbers in code blocks
    ///
    /// Accepts `--line-numbers` (defaults to `true`) or
    /// `--line-numbers <true|false>` for explicit control.
    #[arg(
        long,
        value_name = "BOOL",
        num_args = 0..=1,
        default_missing_value = "true",
        require_equals = false,
    )]
    pub line_numbers: Option<bool>,

    /// Default alignment for all components
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub alignment: Option<PageAlignmentArg>,

    /// Image alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_images: Option<PageAlignmentArg>,

    /// List alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_lists: Option<PageAlignmentArg>,

    /// Unordered list alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_ul: Option<PageAlignmentArg>,

    /// Ordered list alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_ol: Option<PageAlignmentArg>,

    /// List item alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_li: Option<PageAlignmentArg>,

    /// Block quote alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_block_quotes: Option<PageAlignmentArg>,

    /// Table alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_tables: Option<PageAlignmentArg>,

    /// Code block alignment
    #[arg(long, value_enum, value_name = "ALIGN")]
    pub align_code_blocks: Option<PageAlignmentArg>,

    /// Default fill for all components
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill: Option<CliFill>,

    /// Image fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_images: Option<CliFill>,

    /// List fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_lists: Option<CliFill>,

    /// Unordered list fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_ul: Option<CliFill>,

    /// Ordered list fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_ol: Option<CliFill>,

    /// List item fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_li: Option<CliFill>,

    /// Block quote fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_block_quotes: Option<CliFill>,

    /// Table fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_tables: Option<CliFill>,

    /// Code block fill
    #[arg(long, value_name = "FILL", value_parser = parse_cli_fill)]
    pub fill_code_blocks: Option<CliFill>,

    /// Promote schema-validation warnings (unknown / deprecated keys) to errors.
    #[arg(long)]
    pub strict_style: bool,

    /// Increase verbosity for styled user-facing output (-v summary, -vv detailed)
    #[arg(
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count,
        global = true
    )]
    pub verbose: u8,

    /// Enable developer debug logging (1=INFO, 2=DEBUG, 3=TRACE, 4=TRACE+locations).
    /// Alternatively, set RUST_LOG environment variable.
    #[arg(long = "debug", value_name = "LEVEL", global = true, hide = true)]
    pub debug_level: Option<u8>,

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,

    /// Subcommand (read, clean, compose, toc, delta)
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Completes markdown files (`.md`, `.dm`) and directory paths.
fn complete_markdown_files(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_markdown_files_from(Path::new("."), current)
}

/// Completes compose positionals.
///
/// Tokens containing `=` are treated as shorthand setters, so file completion
/// is suppressed to avoid suggesting markdown paths for setter values.
fn complete_compose_args(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_compose_args_from(Path::new("."), current)
}

fn complete_compose_args_from(base_dir: &Path, current: &OsStr) -> Vec<CompletionCandidate> {
    if current.to_string_lossy().contains('=') {
        Vec::new()
    } else {
        complete_markdown_files_from(base_dir, current)
    }
}

fn complete_markdown_files_from(base_dir: &Path, current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();

    let is_markdown = |p: &Path| {
        p.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("dm"))
            .unwrap_or(false)
    };

    if !current_str.is_empty() && "-".starts_with(current_str.as_ref()) {
        seen.insert("-".to_string());
        candidates.push(CompletionCandidate::new("-"));
    }

    let has_trailing_sep = current_str.ends_with('/') || current_str.ends_with('\\');
    let current_path = Path::new(current_str.as_ref());
    let (dir_part, file_prefix) = if current_str.is_empty() {
        (PathBuf::new(), String::new())
    } else if has_trailing_sep {
        (PathBuf::from(current_str.as_ref()), String::new())
    } else {
        let parent = current_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let prefix = current_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        (parent, prefix)
    };

    let search_dir = if dir_part.as_os_str().is_empty() {
        base_dir.to_path_buf()
    } else if dir_part.is_absolute() {
        dir_part.clone()
    } else {
        base_dir.join(&dir_part)
    };

    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if !name.starts_with(&file_prefix) {
                continue;
            }

            let is_dir = path.is_dir();
            if !is_dir && !is_markdown(&path) {
                continue;
            }

            let mut display_path = if current_path.is_absolute() {
                path.to_string_lossy().to_string()
            } else {
                path.strip_prefix(base_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string()
            };

            if display_path.starts_with("./") {
                display_path = display_path.trim_start_matches("./").to_string();
            }

            if is_dir && !display_path.ends_with('/') {
                display_path.push('/');
            }

            if seen.insert(display_path.clone()) {
                candidates.push(CompletionCandidate::new(display_path));
            }
        }
    }

    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Completes supported list indentation widths.
fn complete_indent_values(current: &OsStr) -> Vec<CompletionCandidate> {
    let current_str = current.to_string_lossy();
    let mut candidates: Vec<_> = ["2", "4", "8"]
        .into_iter()
        .filter(|value| value.starts_with(current_str.as_ref()))
        .map(CompletionCandidate::new)
        .collect();
    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Completes theme names for `--theme` / `--code-theme`.
///
/// Enumerates every available [`ThemePair`](darkmatter::markdown::highlighting::ThemePair)
/// by its kebab-case name (the same set `--list-themes` prints), attaching each
/// theme's description as completion help. Without this completer the dynamic
/// completion engine has no value source for the theme flags, so `--theme <tab>`
/// offers nothing.
fn complete_theme_names(current: &OsStr) -> Vec<CompletionCandidate> {
    use darkmatter::markdown::highlighting::ColorMode;

    let current_str = current.to_string_lossy();
    darkmatter::markdown::highlighting::ThemePair::all()
        .iter()
        .filter(|pair| pair.kebab_name().starts_with(current_str.as_ref()))
        .map(|pair| {
            CompletionCandidate::new(pair.kebab_name())
                .help(Some(pair.description(ColorMode::Dark).into()))
        })
        .collect()
}

/// Parses and validates list indentation width.
pub fn parse_indent_size(s: &str) -> Result<usize, String> {
    let value = s
        .parse::<usize>()
        .map_err(|_| format!("'{s}' is not a valid integer"))?;

    match value {
        2 | 4 | 8 => Ok(value),
        _ => Err("indent must be one of: 2, 4, 8".to_string()),
    }
}

/// Parses a theme name string into ThemePair.
pub fn parse_theme_name(s: &str) -> Result<darkmatter::markdown::highlighting::ThemePair, String> {
    darkmatter::markdown::highlighting::ThemePair::try_from(s).map_err(|e| e.to_string())
}

// ── Layout argument types and parsers ─────────────────────────────────────

/// CLI-usable [`PageBackground`] wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PageBackgroundArg {
    /// Transparent (default).
    Transparent,
    /// Slightly off-background fill.
    Subtle,
    /// High-contrast inverse fill.
    Pronounced,
}

impl From<PageBackgroundArg> for PageBackground {
    fn from(arg: PageBackgroundArg) -> Self {
        match arg {
            PageBackgroundArg::Transparent => PageBackground::Transparent,
            PageBackgroundArg::Subtle => PageBackground::Subtle,
            PageBackgroundArg::Pronounced => PageBackground::Pronounced,
        }
    }
}

/// CLI-usable [`CodeBlockMode`] wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CodeBlockArg {
    /// Opposite variant from the page (default): dark page -> light panel.
    Inverse,
    /// Always the dark variant.
    Dark,
    /// Always the light variant.
    Light,
    /// Same variant as the page.
    Same,
}

impl From<CodeBlockArg> for CodeBlockMode {
    fn from(arg: CodeBlockArg) -> Self {
        match arg {
            CodeBlockArg::Inverse => CodeBlockMode::Inverse,
            CodeBlockArg::Dark => CodeBlockMode::Dark,
            CodeBlockArg::Light => CodeBlockMode::Light,
            CodeBlockArg::Same => CodeBlockMode::Same,
        }
    }
}

/// CLI-usable alignment wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PageAlignmentArg {
    /// Left-aligned.
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

impl From<PageAlignmentArg> for renderable::layout::Alignment {
    fn from(arg: PageAlignmentArg) -> Self {
        match arg {
            PageAlignmentArg::Left => renderable::layout::Alignment::Left,
            PageAlignmentArg::Center => renderable::layout::Alignment::Center,
            PageAlignmentArg::Right => renderable::layout::Alignment::Right,
        }
    }
}

/// CLI-local fill descriptor that maps directly onto renderable [`Layout`]
/// properties.
#[derive(Clone, Debug, PartialEq)]
pub enum CliFill {
    /// Default. Component may use the full content width.
    Full,
    /// Symmetric padding on both sides.
    Pad(renderable::layout::Length),
    /// One-sided padding driven by the component's alignment.
    Indent(renderable::layout::Length),
    /// Cap on the component's render width.
    Max(renderable::layout::Length),
    /// Explicit render width.
    Explicit(renderable::layout::Length),
}

/// Parses a [`CliFill`] string.
///
/// Grammar: `full`, `pad=<n|n%>`, `indent=<n|n%>`, `max=<n|n%>`, `explicit=<n|n%>`
pub fn parse_cli_fill(s: &str) -> Result<CliFill, String> {
    let s = s.trim();

    if s.eq_ignore_ascii_case("full") {
        return Ok(CliFill::Full);
    }

    let (kind, rest) = s
        .split_once('=')
        .ok_or_else(|| format!("fill must be 'full' or 'kind=value', got '{s}'"))?;

    let kind = kind.trim().to_ascii_lowercase();
    let rest = rest.trim();

    if rest.is_empty() {
        return Err(format!("fill value missing after '=' in '{s}'"));
    }

    let length = parse_cli_length(rest)?;

    match kind.as_str() {
        "pad" => Ok(CliFill::Pad(length)),
        "indent" => Ok(CliFill::Indent(length)),
        "max" => Ok(CliFill::Max(length)),
        "explicit" => Ok(CliFill::Explicit(length)),
        _ => Err(format!(
            "unknown fill kind '{kind}', expected pad/indent/max/explicit"
        )),
    }
}

/// Parses a length string (`n` or `n%`) into a renderable [`Length`].
fn parse_cli_length(s: &str) -> Result<renderable::layout::Length, String> {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('%') {
        let p: f32 = num
            .parse()
            .map_err(|_| format!("'{s}' is not a valid percentage"))?;
        if !(0.0..=100.0).contains(&p) {
            return Err(format!("percentage must be 0-100, got {p}"));
        }
        Ok(renderable::layout::Length::Percent(p))
    } else {
        let n: u16 = s
            .parse()
            .map_err(|_| format!("'{s}' is not a valid positive integer"))?;
        Ok(renderable::layout::Length::ch(u32::from(n)))
    }
}

/// Parses a boolean string (`true`/`false`/`1`/`0`/`yes`/`no`).
pub fn parse_bool_str(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "y" => Ok(true),
        "false" | "0" | "no" | "off" | "n" => Ok(false),
        _ => Err(format!("expected true/false, got '{s}'")),
    }
}

/// Parses `--max-width`, rejecting `0`.
pub fn parse_max_width(s: &str) -> Result<u16, String> {
    let value = s
        .parse::<u16>()
        .map_err(|_| format!("'{s}' is not a valid positive integer"))?;
    if value == 0 {
        Err("--max-width must be greater than 0".to_string())
    } else {
        Ok(value)
    }
}

/// Reject `--width` outright. Width is derived from the captured `Terminal`
/// and the configured `--max-width`; a separate `--width` flag would shadow
/// those for no reason. Callers should use `--max-width` for content-width
/// control.
pub fn reject_width_flag(_s: &str) -> Result<u16, String> {
    Err(
        "--width is not supported: page width is captured from the terminal; \
         use --max-width to cap the content box."
            .to_string(),
    )
}

/// Parses a `--page-bg-color` value into a [`PaintColor`].
///
/// Accepts:
/// - `#RGB` / `#RRGGBB` hex strings (e.g. `#1e1e23`).
/// - Comma-separated `R,G,B` triples in the 0-255 range (e.g. `30,30,35`).
/// - Tailwind palette names (e.g. `red-500`, `slate-50`).
/// - CSS special keywords: `transparent`, `currentColor`, `inherit`.
///
/// Falls back to [`BasicColor::Black`] for the RGB fallback channel.
pub fn parse_page_bg_color(s: &str) -> Result<PaintColor, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("page background color must not be empty".to_string());
    }

    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "transparent" => return Ok(PaintColor::new(Color::Tailwind(Tailwind::Transparent))),
        "currentcolor" => return Ok(PaintColor::new(Color::Tailwind(Tailwind::Current))),
        "inherit" => return Ok(PaintColor::new(Color::Tailwind(Tailwind::Inherit))),
        _ => {}
    }

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if s.contains(',') {
        return parse_rgb_triple(s);
    }

    if let Some(tw) = tailwind_from_str(s) {
        return Ok(PaintColor::new(Color::Tailwind(tw)));
    }

    Err(format!(
        "invalid page background color '{s}': expected #RGB / #RRGGBB hex, R,G,B triple, \
         Tailwind name (e.g. red-500), or one of transparent / currentColor / inherit"
    ))
}

fn parse_hex_color(hex: &str) -> Result<PaintColor, String> {
    let bytes = hex.as_bytes();

    let nibble = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let pair = |chunk: &[u8]| -> Result<u8, String> {
        if chunk.len() != 2 {
            return Err(format!(
                "hex component must be 2 chars, got '{}'",
                std::str::from_utf8(chunk).unwrap_or("")
            ));
        }
        let hi = nibble(chunk[0])
            .ok_or_else(|| format!("invalid hex char '{}'", chunk[0] as char))?;
        let lo = nibble(chunk[1])
            .ok_or_else(|| format!("invalid hex char '{}'", chunk[1] as char))?;
        Ok((hi << 4) | lo)
    };

    let (r, g, b) = match bytes.len() {
        3 => {
            // #RGB form — duplicate each nibble to get the full byte.
            let r = nibble(bytes[0])
                .ok_or_else(|| format!("invalid hex char '{}'", bytes[0] as char))?;
            let g = nibble(bytes[1])
                .ok_or_else(|| format!("invalid hex char '{}'", bytes[1] as char))?;
            let b = nibble(bytes[2])
                .ok_or_else(|| format!("invalid hex char '{}'", bytes[2] as char))?;
            ((r << 4) | r, (g << 4) | g, (b << 4) | b)
        }
        6 => (pair(&bytes[0..2])?, pair(&bytes[2..4])?, pair(&bytes[4..6])?),
        _ => {
            return Err(format!(
                "hex color must be #RGB or #RRGGBB, got '#{hex}'"
            ));
        }
    };
    Ok(PaintColor::new(Color::Rgb(RgbColor::new(
        r, g, b, BasicColor::Black,
    ))))
}

fn parse_rgb_triple(s: &str) -> Result<PaintColor, String> {
    let parts: Vec<&str> = s.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(format!(
            "R,G,B triple must have exactly three comma-separated values, got '{s}'"
        ));
    }
    let parse_component = |p: &str, name: &str| -> Result<u8, String> {
        let n: u16 = p
            .parse()
            .map_err(|_| format!("{name} component '{p}' is not a valid 0-255 integer"))?;
        if n > 255 {
            return Err(format!("{name} component '{p}' is out of range (0-255)"));
        }
        Ok(n as u8)
    };
    let r = parse_component(parts[0], "red")?;
    let g = parse_component(parts[1], "green")?;
    let b = parse_component(parts[2], "blue")?;
    Ok(PaintColor::new(Color::Rgb(RgbColor::new(
        r, g, b, BasicColor::Black,
    ))))
}

fn tailwind_from_str(s: &str) -> Option<Tailwind> {
    let normalized = s.trim().to_ascii_lowercase().replace('_', "-");
    // Match `<palette>-<weight>` for known palettes. A small allow-list keeps
    // the parser closed; callers can use hex / `R,G,B` for arbitrary colors.
    let (palette, weight) = normalized.split_once('-')?;
    let weight_value: u16 = weight.parse().ok()?;

    use Tailwind::*;
    Some(match (palette, weight_value) {
        ("slate", 50) => Slate50,
        ("slate", 100) => Slate100,
        ("slate", 200) => Slate200,
        ("slate", 300) => Slate300,
        ("slate", 400) => Slate400,
        ("slate", 500) => Slate500,
        ("slate", 600) => Slate600,
        ("slate", 700) => Slate700,
        ("slate", 800) => Slate800,
        ("slate", 900) => Slate900,
        ("slate", 950) => Slate950,
        ("gray", 50) => Gray50,
        ("gray", 100) => Gray100,
        ("gray", 200) => Gray200,
        ("gray", 300) => Gray300,
        ("gray", 400) => Gray400,
        ("gray", 500) => Gray500,
        ("gray", 600) => Gray600,
        ("gray", 700) => Gray700,
        ("gray", 800) => Gray800,
        ("gray", 900) => Gray900,
        ("gray", 950) => Gray950,
        ("zinc", 50) => Zinc50,
        ("zinc", 100) => Zinc100,
        ("zinc", 200) => Zinc200,
        ("zinc", 300) => Zinc300,
        ("zinc", 400) => Zinc400,
        ("zinc", 500) => Zinc500,
        ("zinc", 600) => Zinc600,
        ("zinc", 700) => Zinc700,
        ("zinc", 800) => Zinc800,
        ("zinc", 900) => Zinc900,
        ("zinc", 950) => Zinc950,
        ("neutral", 50) => Neutral50,
        ("neutral", 100) => Neutral100,
        ("neutral", 200) => Neutral200,
        ("neutral", 300) => Neutral300,
        ("neutral", 400) => Neutral400,
        ("neutral", 500) => Neutral500,
        ("neutral", 600) => Neutral600,
        ("neutral", 700) => Neutral700,
        ("neutral", 800) => Neutral800,
        ("neutral", 900) => Neutral900,
        ("neutral", 950) => Neutral950,
        ("stone", 50) => Stone50,
        ("stone", 100) => Stone100,
        ("stone", 200) => Stone200,
        ("stone", 300) => Stone300,
        ("stone", 400) => Stone400,
        ("stone", 500) => Stone500,
        ("stone", 600) => Stone600,
        ("stone", 700) => Stone700,
        ("stone", 800) => Stone800,
        ("stone", 900) => Stone900,
        ("stone", 950) => Stone950,
        ("red", 50) => Red50,
        ("red", 100) => Red100,
        ("red", 200) => Red200,
        ("red", 300) => Red300,
        ("red", 400) => Red400,
        ("red", 500) => Red500,
        ("red", 600) => Red600,
        ("red", 700) => Red700,
        ("red", 800) => Red800,
        ("red", 900) => Red900,
        ("red", 950) => Red950,
        ("orange", 50) => Orange50,
        ("orange", 100) => Orange100,
        ("orange", 200) => Orange200,
        ("orange", 300) => Orange300,
        ("orange", 400) => Orange400,
        ("orange", 500) => Orange500,
        ("orange", 600) => Orange600,
        ("orange", 700) => Orange700,
        ("orange", 800) => Orange800,
        ("orange", 900) => Orange900,
        ("orange", 950) => Orange950,
        ("amber", 50) => Amber50,
        ("amber", 100) => Amber100,
        ("amber", 200) => Amber200,
        ("amber", 300) => Amber300,
        ("amber", 400) => Amber400,
        ("amber", 500) => Amber500,
        ("amber", 600) => Amber600,
        ("amber", 700) => Amber700,
        ("amber", 800) => Amber800,
        ("amber", 900) => Amber900,
        ("amber", 950) => Amber950,
        ("yellow", 50) => Yellow50,
        ("yellow", 100) => Yellow100,
        ("yellow", 200) => Yellow200,
        ("yellow", 300) => Yellow300,
        ("yellow", 400) => Yellow400,
        ("yellow", 500) => Yellow500,
        ("yellow", 600) => Yellow600,
        ("yellow", 700) => Yellow700,
        ("yellow", 800) => Yellow800,
        ("yellow", 900) => Yellow900,
        ("yellow", 950) => Yellow950,
        ("lime", 50) => Lime50,
        ("lime", 100) => Lime100,
        ("lime", 200) => Lime200,
        ("lime", 300) => Lime300,
        ("lime", 400) => Lime400,
        ("lime", 500) => Lime500,
        ("lime", 600) => Lime600,
        ("lime", 700) => Lime700,
        ("lime", 800) => Lime800,
        ("lime", 900) => Lime900,
        ("lime", 950) => Lime950,
        ("green", 50) => Green50,
        ("green", 100) => Green100,
        ("green", 200) => Green200,
        ("green", 300) => Green300,
        ("green", 400) => Green400,
        ("green", 500) => Green500,
        ("green", 600) => Green600,
        ("green", 700) => Green700,
        ("green", 800) => Green800,
        ("green", 900) => Green900,
        ("green", 950) => Green950,
        ("emerald", 50) => Emerald50,
        ("emerald", 100) => Emerald100,
        ("emerald", 200) => Emerald200,
        ("emerald", 300) => Emerald300,
        ("emerald", 400) => Emerald400,
        ("emerald", 500) => Emerald500,
        ("emerald", 600) => Emerald600,
        ("emerald", 700) => Emerald700,
        ("emerald", 800) => Emerald800,
        ("emerald", 900) => Emerald900,
        ("emerald", 950) => Emerald950,
        ("teal", 50) => Teal50,
        ("teal", 100) => Teal100,
        ("teal", 200) => Teal200,
        ("teal", 300) => Teal300,
        ("teal", 400) => Teal400,
        ("teal", 500) => Teal500,
        ("teal", 600) => Teal600,
        ("teal", 700) => Teal700,
        ("teal", 800) => Teal800,
        ("teal", 900) => Teal900,
        ("teal", 950) => Teal950,
        ("cyan", 50) => Cyan50,
        ("cyan", 100) => Cyan100,
        ("cyan", 200) => Cyan200,
        ("cyan", 300) => Cyan300,
        ("cyan", 400) => Cyan400,
        ("cyan", 500) => Cyan500,
        ("cyan", 600) => Cyan600,
        ("cyan", 700) => Cyan700,
        ("cyan", 800) => Cyan800,
        ("cyan", 900) => Cyan900,
        ("cyan", 950) => Cyan950,
        ("sky", 50) => Sky50,
        ("sky", 100) => Sky100,
        ("sky", 200) => Sky200,
        ("sky", 300) => Sky300,
        ("sky", 400) => Sky400,
        ("sky", 500) => Sky500,
        ("sky", 600) => Sky600,
        ("sky", 700) => Sky700,
        ("sky", 800) => Sky800,
        ("sky", 900) => Sky900,
        ("sky", 950) => Sky950,
        ("blue", 50) => Blue50,
        ("blue", 100) => Blue100,
        ("blue", 200) => Blue200,
        ("blue", 300) => Blue300,
        ("blue", 400) => Blue400,
        ("blue", 500) => Blue500,
        ("blue", 600) => Blue600,
        ("blue", 700) => Blue700,
        ("blue", 800) => Blue800,
        ("blue", 900) => Blue900,
        ("blue", 950) => Blue950,
        ("indigo", 50) => Indigo50,
        ("indigo", 100) => Indigo100,
        ("indigo", 200) => Indigo200,
        ("indigo", 300) => Indigo300,
        ("indigo", 400) => Indigo400,
        ("indigo", 500) => Indigo500,
        ("indigo", 600) => Indigo600,
        ("indigo", 700) => Indigo700,
        ("indigo", 800) => Indigo800,
        ("indigo", 900) => Indigo900,
        ("indigo", 950) => Indigo950,
        ("violet", 50) => Violet50,
        ("violet", 100) => Violet100,
        ("violet", 200) => Violet200,
        ("violet", 300) => Violet300,
        ("violet", 400) => Violet400,
        ("violet", 500) => Violet500,
        ("violet", 600) => Violet600,
        ("violet", 700) => Violet700,
        ("violet", 800) => Violet800,
        ("violet", 900) => Violet900,
        ("violet", 950) => Violet950,
        ("purple", 50) => Purple50,
        ("purple", 100) => Purple100,
        ("purple", 200) => Purple200,
        ("purple", 300) => Purple300,
        ("purple", 400) => Purple400,
        ("purple", 500) => Purple500,
        ("purple", 600) => Purple600,
        ("purple", 700) => Purple700,
        ("purple", 800) => Purple800,
        ("purple", 900) => Purple900,
        ("purple", 950) => Purple950,
        ("fuchsia", 50) => Fuchsia50,
        ("fuchsia", 100) => Fuchsia100,
        ("fuchsia", 200) => Fuchsia200,
        ("fuchsia", 300) => Fuchsia300,
        ("fuchsia", 400) => Fuchsia400,
        ("fuchsia", 500) => Fuchsia500,
        ("fuchsia", 600) => Fuchsia600,
        ("fuchsia", 700) => Fuchsia700,
        ("fuchsia", 800) => Fuchsia800,
        ("fuchsia", 900) => Fuchsia900,
        ("fuchsia", 950) => Fuchsia950,
        ("pink", 50) => Pink50,
        ("pink", 100) => Pink100,
        ("pink", 200) => Pink200,
        ("pink", 300) => Pink300,
        ("pink", 400) => Pink400,
        ("pink", 500) => Pink500,
        ("pink", 600) => Pink600,
        ("pink", 700) => Pink700,
        ("pink", 800) => Pink800,
        ("pink", 900) => Pink900,
        ("pink", 950) => Pink950,
        ("rose", 50) => Rose50,
        ("rose", 100) => Rose100,
        ("rose", 200) => Rose200,
        ("rose", 300) => Rose300,
        ("rose", 400) => Rose400,
        ("rose", 500) => Rose500,
        ("rose", 600) => Rose600,
        ("rose", 700) => Rose700,
        ("rose", 800) => Rose800,
        ("rose", 900) => Rose900,
        ("rose", 950) => Rose950,
        ("black", _) => Black,
        ("white", _) => White,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completion_values(candidates: Vec<CompletionCandidate>) -> Vec<String> {
        candidates
            .into_iter()
            .map(|candidate| candidate.get_value().to_string_lossy().into_owned())
            .collect()
    }

    fn normalize_path(path: &str) -> String {
        path.replace('\\', "/")
    }

    #[test]
    fn test_complete_indent_values() {
        let values = completion_values(complete_indent_values(OsStr::new("")));
        assert_eq!(values, vec!["2", "4", "8"]);

        let values = completion_values(complete_indent_values(OsStr::new("4")));
        assert_eq!(values, vec!["4"]);
    }

    #[test]
    fn test_complete_theme_names() {
        // Empty prefix enumerates every theme `--list-themes` knows about.
        let values = completion_values(complete_theme_names(OsStr::new("")));
        let expected: Vec<String> = darkmatter::markdown::highlighting::ThemePair::all()
            .iter()
            .map(|pair| pair.kebab_name().to_string())
            .collect();
        assert_eq!(values, expected);
        assert!(values.contains(&"dracula".to_string()));
        assert!(values.contains(&"nord".to_string()));

        // A prefix narrows the candidates by kebab name.
        let values = completion_values(complete_theme_names(OsStr::new("gru")));
        assert_eq!(values, vec!["gruvbox"]);

        // A non-matching prefix yields nothing.
        assert!(completion_values(complete_theme_names(OsStr::new("zzz"))).is_empty());
    }

    #[test]
    fn test_parse_indent_size() {
        assert_eq!(parse_indent_size("2"), Ok(2));
        assert_eq!(parse_indent_size("4"), Ok(4));
        assert_eq!(parse_indent_size("8"), Ok(8));
        assert!(parse_indent_size("3").is_err());
        assert!(parse_indent_size("abc").is_err());
    }

    #[test]
    fn test_complete_markdown_files_from_supports_nested_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("README.md"), "# Root").unwrap();
        std::fs::write(temp_dir.path().join("notes.txt"), "ignore").unwrap();

        let docs_dir = temp_dir.path().join("docs");
        let deep_dir = docs_dir.join("deep");
        std::fs::create_dir_all(&deep_dir).unwrap();
        std::fs::write(docs_dir.join("guide.md"), "# Guide").unwrap();
        std::fs::write(deep_dir.join("nested.md"), "# Nested").unwrap();

        let root_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new(""),
        ));
        let root_values: Vec<_> = root_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(root_values.contains(&"README.md".to_string()));
        assert!(root_values.contains(&"docs/".to_string()));
        assert!(!root_values.iter().any(|value| value.ends_with("notes.txt")));

        let docs_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/"),
        ));
        let docs_values: Vec<_> = docs_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(docs_values.contains(&"docs/guide.md".to_string()));
        assert!(docs_values.contains(&"docs/deep/".to_string()));

        let deep_values = completion_values(complete_markdown_files_from(
            temp_dir.path(),
            OsStr::new("docs/deep/"),
        ));
        let deep_values: Vec<_> = deep_values
            .into_iter()
            .map(|value| normalize_path(&value))
            .collect();
        assert!(deep_values.contains(&"docs/deep/nested.md".to_string()));
    }

    #[test]
    fn compose_arg_completion_suggests_files_for_non_setter_tokens() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("README.md"), "# Root").unwrap();

        let values = completion_values(complete_compose_args_from(
            temp_dir.path(),
            OsStr::new("REA"),
        ));
        assert!(
            values
                .iter()
                .any(|value| normalize_path(value) == "README.md")
        );
    }

    #[test]
    fn compose_arg_completion_skips_file_suggestions_for_setters() {
        let values = completion_values(complete_compose_args(OsStr::new("name=Al")));
        assert!(values.is_empty());
    }

    #[test]
    fn compose_perf_flag_sets_true() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--perf"]).unwrap();
        match cli.command {
            Some(Command::Compose { perf, .. }) => assert!(perf),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_without_perf_defaults_false() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(Command::Compose { perf, .. }) => assert!(!perf),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn debug_flag_parses_level() {
        let cli = Cli::try_parse_from(["md", "--debug", "2", "doc.md"]).unwrap();
        assert_eq!(cli.debug_level, Some(2));
    }

    #[test]
    fn debug_flag_absent_is_none() {
        let cli = Cli::try_parse_from(["md", "doc.md"]).unwrap();
        assert_eq!(cli.debug_level, None);
    }

    #[test]
    fn compose_timeout_flag_parses() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "--timeout", "3"]).unwrap();
        match cli.command {
            Some(Command::Compose { timeout, .. }) => assert_eq!(timeout, Some(3)),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_allow_shell_timeout_flag_parses() {
        let cli =
            Cli::try_parse_from(["md", "compose", "doc.md", "--allow-shell-timeout"]).unwrap();
        match cli.command {
            Some(Command::Compose {
                allow_shell_timeout,
                ..
            }) => assert!(allow_shell_timeout),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_timeout_defaults_to_none() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(Command::Compose { timeout, .. }) => assert_eq!(timeout, None),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_allow_shell_timeout_defaults_false() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(Command::Compose {
                allow_shell_timeout,
                ..
            }) => assert!(!allow_shell_timeout),
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_args_captures_positional_tokens() {
        let cli = Cli::try_parse_from(["md", "compose", "doc.md", "key=value"]).unwrap();
        match cli.command {
            Some(Command::Compose { args, .. }) => {
                assert_eq!(args, vec!["doc.md", "key=value"]);
            }
            _ => panic!("Expected Compose command"),
        }
    }

    #[test]
    fn compose_args_empty_when_no_positionals() {
        let cli = Cli::try_parse_from(["md", "compose"]).unwrap();
        match cli.command {
            Some(Command::Compose { args, .. }) => {
                assert!(args.is_empty());
            }
            _ => panic!("Expected Compose command"),
        }
    }

    // ---------- Phase 5: layout parser tests ----------

    #[test]
    fn parse_bool_str_accepts_truthy_values() {
        for val in ["true", "1", "yes", "on", "y"] {
            assert!(parse_bool_str(val).unwrap(), "value: {val}");
        }
    }

    #[test]
    fn parse_bool_str_accepts_falsy_values() {
        for val in ["false", "0", "no", "off", "n"] {
            assert!(!parse_bool_str(val).unwrap(), "value: {val}");
        }
    }

    #[test]
    fn parse_bool_str_rejects_invalid() {
        assert!(parse_bool_str("maybe").is_err());
        assert!(parse_bool_str("").is_err());
    }

    #[test]
    fn parse_max_width_accepts_positive() {
        assert_eq!(parse_max_width("80").unwrap(), 80);
        assert_eq!(parse_max_width("1").unwrap(), 1);
    }

    #[test]
    fn parse_max_width_rejects_zero() {
        assert!(parse_max_width("0").is_err());
    }

    #[test]
    fn parse_max_width_rejects_negative() {
        assert!(parse_max_width("-1").is_err());
    }

    #[test]
    fn reject_width_flag_always_errors() {
        assert!(reject_width_flag("80").is_err());
        assert!(reject_width_flag("0").is_err());
    }

    #[test]
    fn parse_page_bg_color_hex_six() {
        let c = parse_page_bg_color("#1e1e23").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 0x1e);
                assert_eq!(rgb.green(), 0x1e);
                assert_eq!(rgb.blue(), 0x23);
            }
            other => panic!("expected RGB color, got {other:?}"),
        }
    }

    #[test]
    fn parse_page_bg_color_hex_three() {
        let c = parse_page_bg_color("#abc").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 0xaa);
                assert_eq!(rgb.green(), 0xbb);
                assert_eq!(rgb.blue(), 0xcc);
            }
            other => panic!("expected RGB color, got {other:?}"),
        }
    }

    #[test]
    fn parse_page_bg_color_rgb_triple() {
        let c = parse_page_bg_color("30,30,35").unwrap();
        match c.color {
            Color::Rgb(rgb) => {
                assert_eq!(rgb.red(), 30);
                assert_eq!(rgb.green(), 30);
                assert_eq!(rgb.blue(), 35);
            }
            other => panic!("expected RGB color, got {other:?}"),
        }
    }

    #[test]
    fn parse_page_bg_color_tailwind() {
        let c = parse_page_bg_color("red-500").unwrap();
        assert!(matches!(c.color, Color::Tailwind(Tailwind::Red500)));
    }

    #[test]
    fn parse_page_bg_color_special_keywords() {
        assert!(matches!(
            parse_page_bg_color("transparent").unwrap().color,
            Color::Tailwind(Tailwind::Transparent)
        ));
        assert!(matches!(
            parse_page_bg_color("currentColor").unwrap().color,
            Color::Tailwind(Tailwind::Current)
        ));
        assert!(matches!(
            parse_page_bg_color("inherit").unwrap().color,
            Color::Tailwind(Tailwind::Inherit)
        ));
    }

    #[test]
    fn parse_page_bg_color_rejects_invalid() {
        assert!(parse_page_bg_color("").is_err());
        assert!(parse_page_bg_color("not-a-color").is_err());
        assert!(parse_page_bg_color("#zzz").is_err());
        assert!(parse_page_bg_color("256,0,0").is_err());
        assert!(parse_page_bg_color("1,2").is_err());
    }

    #[test]
    fn parse_cli_fill_full() {
        assert_eq!(parse_cli_fill("full").unwrap(), CliFill::Full);
        assert_eq!(parse_cli_fill("FULL").unwrap(), CliFill::Full);
    }

    #[test]
    fn parse_cli_fill_pad_fixed() {
        assert_eq!(
            parse_cli_fill("pad=4").unwrap(),
            CliFill::Pad(renderable::layout::Length::ch(4))
        );
    }

    #[test]
    fn parse_cli_fill_pad_percent() {
        assert_eq!(
            parse_cli_fill("pad=10%").unwrap(),
            CliFill::Pad(renderable::layout::Length::Percent(10.0))
        );
    }

    #[test]
    fn parse_cli_fill_indent_max_explicit() {
        assert_eq!(
            parse_cli_fill("indent=2").unwrap(),
            CliFill::Indent(renderable::layout::Length::ch(2))
        );
        assert_eq!(
            parse_cli_fill("max=40").unwrap(),
            CliFill::Max(renderable::layout::Length::ch(40))
        );
        assert_eq!(
            parse_cli_fill("explicit=60").unwrap(),
            CliFill::Explicit(renderable::layout::Length::ch(60))
        );
    }

    #[test]
    fn parse_cli_fill_rejects_unknown_kind() {
        assert!(parse_cli_fill("unknown=4").is_err());
    }

    #[test]
    fn parse_cli_fill_rejects_percent_over_100() {
        assert!(parse_cli_fill("pad=150%").is_err());
    }

    #[test]
    fn parse_cli_fill_rejects_negative() {
        assert!(parse_cli_fill("pad=-1").is_err());
    }

    #[test]
    fn parse_cli_fill_rejects_malformed() {
        assert!(parse_cli_fill("pad").is_err());
        assert!(parse_cli_fill("=").is_err());
    }

    #[test]
    fn parse_cli_length_fixed() {
        assert_eq!(parse_cli_length("80").unwrap(), renderable::layout::Length::ch(80));
    }

    #[test]
    fn parse_cli_length_percent() {
        assert_eq!(parse_cli_length("50%").unwrap(), renderable::layout::Length::Percent(50.0));
    }

    #[test]
    fn parse_cli_length_rejects_out_of_range_percent() {
        assert!(parse_cli_length("150%").is_err());
        assert!(parse_cli_length("-10%").is_err());
    }

    #[test]
    fn cli_margin_flags_parse_correctly() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--margin", "4", "--mt", "1", "--mx", "2"])
            .unwrap();
        assert_eq!(cli.margin, Some(4));
        assert_eq!(cli.mt, Some(1));
        assert_eq!(cli.mx, Some(2));
    }

    #[test]
    fn cli_padding_flags_parse_correctly() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--padding", "2", "--px", "1"]).unwrap();
        assert_eq!(cli.padding, Some(2));
        assert_eq!(cli.px, Some(1));
    }

    #[test]
    fn cli_page_bg_flag_parses() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--page-bg", "subtle"]).unwrap();
        assert!(cli.page_bg.is_some());
    }

    #[test]
    fn cli_alignment_flags_parse() {
        let cli = Cli::try_parse_from([
            "md",
            "doc.md",
            "--alignment",
            "center",
            "--align-images",
            "left",
        ])
        .unwrap();
        assert!(cli.alignment.is_some());
        assert!(cli.align_images.is_some());
    }

    #[test]
    fn cli_fill_flags_parse() {
        let cli = Cli::try_parse_from([
            "md",
            "doc.md",
            "--fill",
            "pad=4",
            "--fill-code-blocks",
            "max=40",
        ])
        .unwrap();
        assert!(cli.fill.is_some());
        assert!(cli.fill_code_blocks.is_some());
    }

    #[test]
    fn cli_line_numbers_bare_flag_parses_as_true() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--line-numbers"]).unwrap();
        assert_eq!(cli.line_numbers, Some(true));
    }

    #[test]
    fn cli_line_numbers_true_parses() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--line-numbers", "true"]).unwrap();
        assert_eq!(cli.line_numbers, Some(true));
    }

    #[test]
    fn cli_line_numbers_false_parses() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--line-numbers", "false"]).unwrap();
        assert_eq!(cli.line_numbers, Some(false));
    }

    #[test]
    fn cli_line_numbers_omitted_is_none() {
        let cli = Cli::try_parse_from(["md", "doc.md"]).unwrap();
        assert_eq!(cli.line_numbers, None);
    }

    #[test]
    fn schema_about_parses_as_schema_target() {
        let cli = Cli::try_parse_from(["md", "schema", "about"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Schema {
                target: SchemaTarget::About,
            })
        ));
        assert_eq!(cli.code_block, CodeBlockArg::Inverse);
    }

    #[test]
    fn schema_about_accepts_code_block_flag() {
        // `--code-block` is a global flag, so it is accepted after `schema about`
        // and lands on the top-level `Cli`.
        let cli =
            Cli::try_parse_from(["md", "schema", "about", "--code-block", "light"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Schema {
                target: SchemaTarget::About,
            })
        ));
        assert_eq!(cli.code_block, CodeBlockArg::Light);
    }

    #[test]
    fn render_code_block_flag_defaults_to_inverse() {
        let cli = Cli::try_parse_from(["md", "doc.md"]).unwrap();
        assert_eq!(cli.code_block, CodeBlockArg::Inverse);
    }

    #[test]
    fn render_code_block_flag_parses_dark() {
        let cli = Cli::try_parse_from(["md", "doc.md", "--code-block", "dark"]).unwrap();
        assert_eq!(cli.code_block, CodeBlockArg::Dark);
    }
}
