// The CLI maps `--align-*` / `--fill-*` arguments onto the deprecated
// page-layout types (`PageAlignment`, `PageFill`, `WidthUnit`), which remain
// the public construction path through the `DarkmatterPage` builder. The
// migration to `renderable::layout::Layout` is tracked separately.
#![allow(deprecated)]

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use darkmatter::layout::{PageAlignment, PageBackground, PageFill, WidthUnit};
use darkmatter::markdown::highlighting::ThemePair;
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
    /// HTML output.
    Html,
    /// AST JSON output.
    #[value(alias = "ast")]
    Json,
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

        /// Maximum concurrent remote fetches (default: 4)
        #[arg(long, value_name = "N", default_value_t = 4)]
        remote_concurrency: usize,

        /// Remote artifact TTL in seconds (default: use server cache headers)
        #[arg(long, value_name = "SECONDS")]
        remote_ttl: Option<u64>,

        /// Force revalidation of cached remote artifacts
        #[arg(long)]
        remote_refresh: bool,

        /// Remote freshness mode: optimistic, strict, or fallback (default)
        #[arg(long, value_name = "MODE", default_value = "fallback")]
        remote_freshness: String,

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
    #[arg(long, value_name = "N")]
    pub mt: Option<u16>,

    /// Bottom margin
    #[arg(long, value_name = "N")]
    pub mb: Option<u16>,

    /// Left margin
    #[arg(long, value_name = "N")]
    pub ml: Option<u16>,

    /// Right margin
    #[arg(long, value_name = "N")]
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
    #[arg(long, value_name = "N")]
    pub pt: Option<u16>,

    /// Bottom padding
    #[arg(long, value_name = "N")]
    pub pb: Option<u16>,

    /// Left padding
    #[arg(long, value_name = "N")]
    pub pl: Option<u16>,

    /// Right padding
    #[arg(long, value_name = "N")]
    pub pr: Option<u16>,

    /// Page background style
    #[arg(
        long,
        visible_alias = "page-background",
        value_enum,
        value_name = "STYLE"
    )]
    pub page_bg: Option<PageBackgroundArg>,

    /// Max content width in columns (0 rejected)
    #[arg(long, value_name = "N", value_parser = parse_max_width)]
    pub max_width: Option<u16>,

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
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill: Option<PageFill>,

    /// Image fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_images: Option<PageFill>,

    /// List fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_lists: Option<PageFill>,

    /// Unordered list fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_ul: Option<PageFill>,

    /// Ordered list fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_ol: Option<PageFill>,

    /// List item fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_li: Option<PageFill>,

    /// Block quote fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_block_quotes: Option<PageFill>,

    /// Table fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_tables: Option<PageFill>,

    /// Code block fill
    #[arg(long, value_name = "FILL", value_parser = parse_page_fill)]
    pub fill_code_blocks: Option<PageFill>,

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

/// CLI-usable [`PageAlignment`] wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PageAlignmentArg {
    /// Left-aligned.
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

impl From<PageAlignmentArg> for PageAlignment {
    fn from(arg: PageAlignmentArg) -> Self {
        match arg {
            PageAlignmentArg::Left => PageAlignment::Left,
            PageAlignmentArg::Center => PageAlignment::Center,
            PageAlignmentArg::Right => PageAlignment::Right,
        }
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

/// Parses a [`PageFill`] string.
///
/// Grammar: `full`, `pad=<n|n%>`, `indent=<n|n%>`, `max=<n|n%>`, `explicit=<n|n%>`
pub fn parse_page_fill(s: &str) -> Result<PageFill, String> {
    let s = s.trim();

    if s.eq_ignore_ascii_case("full") {
        return Ok(PageFill::Full);
    }

    let (kind, rest) = s
        .split_once('=')
        .ok_or_else(|| format!("fill must be 'full' or 'kind=value', got '{s}'"))?;

    let kind = kind.trim().to_ascii_lowercase();
    let rest = rest.trim();

    if rest.is_empty() {
        return Err(format!("fill value missing after '=' in '{s}'"));
    }

    let unit = parse_width_unit(rest)?;

    match kind.as_str() {
        "pad" => Ok(PageFill::Pad(unit)),
        "indent" => Ok(PageFill::Indent(unit)),
        "max" => Ok(PageFill::Max(unit)),
        "explicit" => Ok(PageFill::Explicit(unit)),
        _ => Err(format!(
            "unknown fill kind '{kind}', expected pad/indent/max/explicit"
        )),
    }
}

/// Parses a [`WidthUnit`] string (`n` or `n%`).
fn parse_width_unit(s: &str) -> Result<WidthUnit, String> {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('%') {
        let p: f32 = num
            .parse()
            .map_err(|_| format!("'{s}' is not a valid percentage"))?;
        if !(0.0..=100.0).contains(&p) {
            return Err(format!("percentage must be 0-100, got {p}"));
        }
        Ok(WidthUnit::Percent(p))
    } else {
        let n: u16 = s
            .parse()
            .map_err(|_| format!("'{s}' is not a valid positive integer"))?;
        Ok(WidthUnit::Fixed(n))
    }
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
    fn parse_page_fill_full() {
        assert_eq!(parse_page_fill("full").unwrap(), PageFill::Full);
        assert_eq!(parse_page_fill("FULL").unwrap(), PageFill::Full);
    }

    #[test]
    fn parse_page_fill_pad_fixed() {
        assert_eq!(
            parse_page_fill("pad=4").unwrap(),
            PageFill::Pad(WidthUnit::Fixed(4))
        );
    }

    #[test]
    fn parse_page_fill_pad_percent() {
        assert_eq!(
            parse_page_fill("pad=10%").unwrap(),
            PageFill::Pad(WidthUnit::Percent(10.0))
        );
    }

    #[test]
    fn parse_page_fill_indent_max_explicit() {
        assert_eq!(
            parse_page_fill("indent=2").unwrap(),
            PageFill::Indent(WidthUnit::Fixed(2))
        );
        assert_eq!(
            parse_page_fill("max=40").unwrap(),
            PageFill::Max(WidthUnit::Fixed(40))
        );
        assert_eq!(
            parse_page_fill("explicit=60").unwrap(),
            PageFill::Explicit(WidthUnit::Fixed(60))
        );
    }

    #[test]
    fn parse_page_fill_rejects_unknown_kind() {
        assert!(parse_page_fill("unknown=4").is_err());
    }

    #[test]
    fn parse_page_fill_rejects_percent_over_100() {
        assert!(parse_page_fill("pad=150%").is_err());
    }

    #[test]
    fn parse_page_fill_rejects_negative() {
        assert!(parse_page_fill("pad=-1").is_err());
    }

    #[test]
    fn parse_page_fill_rejects_malformed() {
        assert!(parse_page_fill("pad").is_err());
        assert!(parse_page_fill("=").is_err());
    }

    #[test]
    fn parse_width_unit_fixed() {
        assert_eq!(parse_width_unit("80").unwrap(), WidthUnit::Fixed(80));
    }

    #[test]
    fn parse_width_unit_percent() {
        assert_eq!(parse_width_unit("50%").unwrap(), WidthUnit::Percent(50.0));
    }

    #[test]
    fn parse_width_unit_rejects_out_of_range_percent() {
        assert!(parse_width_unit("150%").is_err());
        assert!(parse_width_unit("-10%").is_err());
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
}
