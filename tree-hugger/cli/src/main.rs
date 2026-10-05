use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::IsTerminal;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use biscuit_file::to_portable_string;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::prelude::{Prose, Terminal, TerminalRenderable, WordWrap, strip_escape_codes};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::Shell;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use clap_complete::env::{CompleteEnv, Shells};
use owo_colors::{OwoColorize, Style};
use serde::{Deserialize, Serialize};
use tree_hugger::adapter::{
    AdapterConfig, AdapterMetadata, ExternalDiagnosticAdapter, OxlintAdapter,
};
use tree_hugger::cache::{PersistentCache, SymbolCache};
use tree_hugger::{
    Capability, CodeRange, Diagnostic, DiagnosticKind, DiagnosticSeverity, FileSummary,
    FileSymbolIndex, ImportSymbol, LintDiagnostic, ProgrammingLanguage, RuleRegistry,
    RuleSelector, SchemaVersion, SourceContext, SymbolQuery, SyntaxDiagnostic, TreeHuggerError,
    file_identity, find_git_root, find_package_root,
};
use tree_hugger::god_files::{GodAnalysis, GodFiles, RefactorHint, RiskBand, SymbolBlock};
use url::Url;
mod classes;
mod declarations;
mod import_format;
mod order;
mod prelude;
mod render;
mod report;
mod scan;
mod scanner;
mod selection;

use classes::{ClassSelection, MemberFilter};
use declarations::{DeclarationScan, Declarations, ReportFormat};
use import_format::*;
use order::SortOrder;
use render::{ListingOptions, color_tag_for_kind};
use scan::{Analyzer, CacheStats, in_file_order};
use scanner::*;
use selection::{ImportSelection, Selection, SelectionArgs, TargetArgs, resolve_file_selectors};

#[derive(Parser, Debug)]
#[command(
    name = "hug",
    version,
    about = "Tree Hugger diagnostics and symbol tooling"
)]
struct Cli {
    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Disable colors and hyperlinks (plain text output)
    #[arg(long, global = true)]
    plain: bool,

    /// Output every scanned file's complete index as JSON, diagnostics
    /// included (functions, types, symbols and classes only; symbol
    /// selectors are rejected)
    #[arg(long, global = true)]
    json_full: bool,

    /// Report persistent-cache hits, partial hits and misses, the parsing
    /// work done, and recoverable cache failures on stderr
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

/// Scan options shared by symbol-listing subcommands.
#[derive(clap::Args, Debug, Clone)]
struct ScanOptions {
    /// Glob patterns for files to exclude from scanning
    #[arg(
        long,
        value_name = "GLOB",
        display_order = 10,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    exclude_files: Vec<String>,

    /// Name patterns of symbols to exclude from output (the `--name` rules;
    /// a trailing `!` is exact). An exclusion wins over every inclusion
    #[arg(long, value_name = "PATTERN", display_order = 11)]
    exclude_symbols: Vec<String>,

    /// Force a specific language. Explicitly named files are parsed with this
    /// language even when their extension maps elsewhere or is absent; directory
    /// scans are restricted to this language's extensions.
    #[arg(long, value_enum)]
    language: Option<LanguageArg>,

    /// Neither read nor write the persistent symbol cache.
    ///
    /// Every file is parsed and analyzed; the output is the same as a cached
    /// run's.
    #[arg(long)]
    no_cache: bool,

    /// Show symbol-level documentation comments in output
    #[arg(long)]
    comments: bool,

    /// Group symbol output by file path
    #[arg(long)]
    group_by_file: bool,

    /// Group symbol output by module path (directory/module scope)
    #[arg(long)]
    group_by_module: bool,
}

/// Declaration order, accepted only by the declaration commands.
#[derive(clap::Args, Debug, Clone)]
struct SortArgs {
    /// Order of the listed declarations: `file` (by file, then source
    /// position), `name` (case-sensitive) or `kind`. Ties fall back to file,
    /// position, kind and ID. JSON keeps one entry per file, in path order,
    /// and sorts the records within each.
    #[arg(long, value_enum, value_name = "ORDER", default_value_t = SortOrder::File)]
    sort: SortOrder,
}

/// Arguments of `functions`, `types` and `symbols`.
#[derive(clap::Args, Debug, Clone)]
struct DeclarationArgs {
    #[clap(flatten)]
    common: CommonArgs,

    #[clap(flatten)]
    order: SortArgs,
}

impl Cli {
    /// Returns the output format based on flags.
    fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else if self.plain {
            OutputFormat::Plain
        } else {
            OutputFormat::Pretty
        }
    }
}

/// Common arguments for all subcommands
#[derive(clap::Args, Debug, Clone)]
struct CommonArgs {
    /// File path or symbol-name filters
    ///
    /// File-like filters (paths such as `src/lib.rs`, globs such as `src/**/*.rs`,
    /// or names with a source extension such as `lib.rs` / `*.rs`) limit which
    /// files are scanned.
    ///
    /// Remaining filters are symbol-name filters (as `--name`).
    ///
    /// `parse` => contains match; all-lowercase ignores case.
    ///
    /// `Parse` => any uppercase letter makes the match case-sensitive.
    ///
    /// `*width*` => wildcard match of the whole name.
    ///
    /// `parse_width_spec!` => exact symbol name match.
    #[arg(
        value_name = "FILTER",
        num_args = 0..,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    filters: Vec<String>,

    /// Show only exported symbols (`is_exported`, re-exports included)
    #[arg(long, conflicts_with = "prelude", display_order = 30)]
    exported: bool,

    /// Show only symbols explicitly exported by the prelude module
    #[arg(long, conflicts_with = "exported", display_order = 31)]
    prelude: bool,

    #[clap(flatten)]
    selection: SelectionArgs,

    #[clap(flatten)]
    scan: ScanOptions,
}

/// Arguments for the imports command
#[derive(clap::Args, Debug, Clone)]
struct ImportArgs {
    /// File path or glob filters (never import names; use `--name`)
    #[arg(
        value_name = "FILTER",
        num_args = 0..,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    filters: Vec<String>,

    #[clap(flatten)]
    targets: TargetArgs,

    #[clap(flatten)]
    scan: ScanOptions,
}

/// Arguments for the classes command
#[derive(clap::Args, Debug, Clone)]
struct ClassArgs {
    /// File path or class-name filters
    ///
    /// File-like filters (paths, globs, or names with a source extension)
    /// limit scanned files.
    ///
    /// Remaining filters are class-name filters (as `--name`).
    ///
    /// `widget` => contains match; all-lowercase ignores case.
    ///
    /// `*Widget*` => wildcard match of the whole name.
    ///
    /// `Widget!` => exact class name match.
    #[arg(
        value_name = "FILTER",
        num_args = 0..,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    filters: Vec<String>,

    /// Show only static members
    #[arg(long, conflicts_with = "instance_only")]
    static_only: bool,

    /// Show only instance members
    #[arg(long)]
    instance_only: bool,

    /// Show only exported classes (`is_exported`, re-exports included)
    #[arg(long, conflicts_with = "prelude")]
    exported: bool,

    /// Show only classes explicitly exported by the prelude module
    #[arg(long, conflicts_with = "exported")]
    prelude: bool,

    #[clap(flatten)]
    selection: SelectionArgs,

    #[clap(flatten)]
    scan: ScanOptions,

    #[clap(flatten)]
    order: SortArgs,
}

/// Arguments for the lint command
#[derive(clap::Args, Debug, Clone)]
struct LintArgs {
    /// File filters (glob patterns or paths)
    #[arg(
        value_name = "FILTER",
        num_args = 0..,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    filters: Vec<String>,

    /// Show only lint diagnostics (pattern-based and semantic rules)
    #[arg(long, conflicts_with = "syntax_only")]
    lint_only: bool,

    /// Show only syntax diagnostics (parse errors)
    #[arg(long, conflicts_with = "lint_only")]
    syntax_only: bool,

    /// Treat selected rules or categories as errors.
    ///
    /// Accepts rule IDs (e.g. `unwrap-call`) or category selectors
    /// (e.g. `category:correctness`). Use `all` to deny everything.
    #[arg(long, value_name = "RULE|CATEGORY", display_order = 40)]
    deny: Vec<String>,

    /// Treat selected rules or categories as warnings.
    ///
    /// Accepts rule IDs or category selectors. Overrides default severity.
    #[arg(long, value_name = "RULE|CATEGORY", display_order = 41)]
    warn: Vec<String>,

    /// Treat selected rules or categories as info (suppressed from exit code).
    ///
    /// Accepts rule IDs or category selectors.
    #[arg(long, value_name = "RULE|CATEGORY", display_order = 42)]
    allow: Vec<String>,

    /// Promote all warnings to errors (does not enable experimental rules).
    #[arg(long, display_order = 43)]
    strict: bool,

    /// Enable experimental semantic rules (undefined-symbol, unused-symbol,
    /// undefined-module).
    ///
    /// These rules are gated because they have high false-positive rates
    /// without project context.
    #[arg(long, display_order = 44)]
    experimental_semantics: bool,

    /// List registered lint rules and exit.
    #[arg(long, display_order = 45)]
    list_rules: bool,

    #[clap(flatten)]
    scan: ScanOptions,
}

/// Arguments for the completions command
#[derive(clap::Args, Debug, Clone)]
struct CompletionsArgs {
    /// The shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

/// Arguments for the god-files command
#[derive(clap::Args, Debug, Clone)]
struct GodFilesArgs {
    /// Directory to scan (defaults to current directory)
    #[arg(value_name = "DIR", value_hint = ValueHint::DirPath)]
    dir: Option<PathBuf>,

    /// Show only high-risk files
    #[arg(long)]
    high_risk: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// List functions in the file(s)
    Functions(DeclarationArgs),
    /// List types in the file(s)
    Types(DeclarationArgs),
    /// List all symbols in the file(s)
    Symbols(DeclarationArgs),
    /// List imported symbols in the file(s)
    Imports(ImportArgs),
    /// List classes and their members
    Classes(ClassArgs),
    /// Run lint diagnostics on the file(s)
    Lint(LintArgs),
    /// Generate shell completions
    ///
    /// Emits a dynamic-completion registration script. The script delegates
    /// each completion request back to the `hug` binary at runtime, so
    /// completions stay in sync with the installed CLI and can return
    /// context-aware candidates (e.g. only source files in the current
    /// directory).
    #[command(after_help = "\
Examples:
  # Bash (add to ~/.bashrc)
  echo 'source <(hug completions bash)' >> ~/.bashrc

  # Zsh (add to ~/.zshrc)
  echo 'source <(hug completions zsh)' >> ~/.zshrc

  # Fish
  hug completions fish > ~/.config/fish/completions/hug.fish

  # PowerShell (add to $PROFILE)
  hug completions powershell >> $PROFILE
")]
    Completions(CompletionsArgs),
    /// Identify oversized source files that are strong candidates for refactoring
    GodFiles(GodFilesArgs),
}

impl Command {
    /// Returns positional filter tokens from the subcommand.
    fn filters(&self) -> &[String] {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => &args.common.filters,
            Self::Imports(args) => &args.filters,
            Self::Lint(args) => &args.filters,
            Self::Classes(args) => &args.filters,
            Self::Completions(_) | Self::GodFiles(_) => &[],
        }
    }

    /// Returns whether the `--exported` flag was set.
    fn exported(&self) -> bool {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => args.common.exported,
            Self::Classes(args) => args.exported,
            Self::Imports(_) | Self::Lint(_) | Self::Completions(_) | Self::GodFiles(_) => false,
        }
    }

    /// Returns whether the `--prelude` flag was set.
    fn prelude(&self) -> bool {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => args.common.prelude,
            Self::Classes(args) => args.prelude,
            Self::Imports(_) | Self::Lint(_) | Self::Completions(_) | Self::GodFiles(_) => false,
        }
    }

    /// Returns the scan options of a scanning subcommand.
    fn scan_options(&self) -> Option<&ScanOptions> {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => Some(&args.common.scan),
            Self::Imports(args) => Some(&args.scan),
            Self::Classes(args) => Some(&args.scan),
            Self::Lint(args) => Some(&args.scan),
            Self::Completions(_) | Self::GodFiles(_) => None,
        }
    }

    /// Returns `--sort` of a declaration command.
    fn sort(&self) -> Option<SortOrder> {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => Some(args.order.sort),
            Self::Classes(args) => Some(args.order.sort),
            Self::Imports(_) | Self::Lint(_) | Self::Completions(_) | Self::GodFiles(_) => None,
        }
    }

    /// The record selectors of a symbol command.
    fn selection_args(&self) -> Option<&SelectionArgs> {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => Some(&args.common.selection),
            Self::Classes(args) => Some(&args.selection),
            Self::Imports(_) | Self::Lint(_) | Self::Completions(_) | Self::GodFiles(_) => None,
        }
    }

    /// `--file`, `--name` and `--exact`.
    fn targets(&self) -> Option<&TargetArgs> {
        match self {
            Self::Imports(args) => Some(&args.targets),
            _ => self.selection_args().map(|selection| &selection.targets),
        }
    }

    /// The flags given that narrow records rather than files, which
    /// `--json-full` rejects; positional symbol-name filters are checked by
    /// the caller, which classifies them.
    fn record_selector_flags(&self) -> Vec<&'static str> {
        let mut flags = Vec::new();
        if self.exported() {
            flags.push("--exported");
        }
        if self.prelude() {
            flags.push("--prelude");
        }
        if self.scan_options().is_some_and(|scan| !scan.exclude_symbols.is_empty()) {
            flags.push("--exclude-symbols");
        }
        if let Some(selection) = self.selection_args() {
            flags.extend(selection.given_flags());
        }
        if let Self::Classes(args) = self {
            if args.static_only {
                flags.push("--static-only");
            }
            if args.instance_only {
                flags.push("--instance-only");
            }
        }
        flags
    }

    /// Returns the command kind for dispatching operations.
    fn kind(&self) -> Option<CommandKind> {
        match self {
            Self::Functions(_) => Some(CommandKind::Functions),
            Self::Types(_) => Some(CommandKind::Types),
            Self::Symbols(_) => Some(CommandKind::Symbols),
            Self::Imports(_) => Some(CommandKind::Imports),
            Self::Lint(args) => Some(CommandKind::Lint {
                lint_only: args.lint_only,
                syntax_only: args.syntax_only,
                deny: parse_selectors(&args.deny),
                warn: parse_selectors(&args.warn),
                allow: parse_selectors(&args.allow),
                strict: args.strict,
                experimental_semantics: args.experimental_semantics,
                list_rules: args.list_rules,
            }),
            Self::Classes(args) => Some(CommandKind::Classes {
                static_only: args.static_only,
                instance_only: args.instance_only,
            }),
            Self::Completions(_) | Self::GodFiles(_) => None,
        }
    }
}

/// Parses rule selectors from CLI argument strings.
fn parse_selectors(args: &[String]) -> Vec<RuleSelector> {
    args.iter().filter_map(|s| RuleSelector::parse(s)).collect()
}

/// The kind of command being executed (without the arguments).
#[derive(Debug, Clone)]
pub(crate) enum CommandKind {
    Functions,
    Types,
    Symbols,
    Imports,
    Lint {
        lint_only: bool,
        syntax_only: bool,
        deny: Vec<RuleSelector>,
        warn: Vec<RuleSelector>,
        allow: Vec<RuleSelector>,
        strict: bool,
        experimental_semantics: bool,
        list_rules: bool,
    },
    Classes {
        static_only: bool,
        instance_only: bool,
    },
}

#[derive(Debug, Clone, Default)]
struct AnalysisOptions {
    experimental_semantics: bool,
    deny: Vec<RuleSelector>,
    warn: Vec<RuleSelector>,
    allow: Vec<RuleSelector>,
    strict: bool,
    use_external_adapters: bool,
}

impl AnalysisOptions {
    fn from_command(command: &CommandKind) -> Self {
        match command {
            CommandKind::Lint {
                deny,
                warn,
                allow,
                strict,
                experimental_semantics,
                ..
            } => Self {
                experimental_semantics: *experimental_semantics,
                deny: deny.clone(),
                warn: warn.clone(),
                allow: allow.clone(),
                strict: *strict,
                use_external_adapters: true,
            },
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonOutput {
    schema_version: SchemaVersion,
    root_dir: PathBuf,
    language: ProgrammingLanguage,
    files: Vec<FileSummary>,
    symbol_indexes: Vec<tree_hugger::FileSymbolIndex>,
    adapter_metadata: Vec<AdapterMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuleListItem {
    id: String,
    title: String,
    category: String,
    default_severity: DiagnosticSeverity,
    confidence: String,
    enabled_by_default: bool,
    requires_experimental_semantics: bool,
    languages: Vec<ProgrammingLanguage>,
    aliases: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    /// Colored output with hyperlinks (default when TTY)
    Pretty,
    /// Plain text without colors or hyperlinks
    Plain,
    /// JSON output
    Json,
}

/// Configuration for output styling.
struct OutputConfig {
    use_colors: bool,
    use_hyperlinks: bool,
}

/// Whether `CLICOLOR_FORCE` requests colored output regardless of TTY.
///
/// Follows the common convention: any value other than unset or `0` forces
/// color on. Lets non-interactive callers (and tests) opt into styled output.
fn color_forced() -> bool {
    std::env::var("CLICOLOR_FORCE").is_ok_and(|value| value != "0")
}

impl OutputConfig {
    fn new(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Pretty => {
                // Color is enabled for an interactive TTY (honoring NO_COLOR),
                // or forced on regardless of TTY by CLICOLOR_FORCE.
                let no_color = std::env::var("NO_COLOR").is_ok();
                let is_tty = std::io::stdout().is_terminal();
                let use_colors = color_forced() || (!no_color && is_tty);
                Self {
                    use_colors,
                    use_hyperlinks: use_colors,
                }
            }
            OutputFormat::Plain | OutputFormat::Json => Self {
                use_colors: false,
                use_hyperlinks: false,
            },
        }
    }
}

/// Creates an OSC8 hyperlink for a file path with line number.
///
/// Falls back to undecorated `text` when `path` has no `file://` spelling, so a
/// path a terminal could not follow anyway is never wrapped in a dead link.
fn hyperlink(path: &Path, line: usize, text: &str) -> String {
    let Some(url) = file_url(path) else {
        return text.to_string();
    };
    format!("\x1b]8;;{}#L{}\x1b\\{}\x1b]8;;\x1b\\", url, line, text)
}

/// Renders a path for display, relative to `root` when it lies beneath it.
///
/// Output is slash-separated on every platform: these strings are the CLI's
/// user-facing and grep-able surface, and Windows backslashes would make them
/// diverge from the paths callers typed and from the other platforms' output.
fn display_path(path: &Path, root: Option<&Path>) -> String {
    if let Some(root) = root
        && let Ok(relative) = path.strip_prefix(root)
    {
        return to_portable_string(relative);
    }
    to_portable_string(path)
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum LanguageArg {
    Rust,
    #[value(name = "javascript", alias = "js")]
    JavaScript,
    #[value(name = "typescript", alias = "ts")]
    TypeScript,
    Go,
    Python,
    Java,
    Php,
    Perl,
    Bash,
    Zsh,
    C,
    #[value(name = "c++", alias = "cpp")]
    Cpp,
    #[value(name = "c#", aliases = ["csharp", "c-sharp"])]
    CSharp,
    Swift,
    Scala,
    Lua,
}

impl From<LanguageArg> for ProgrammingLanguage {
    fn from(value: LanguageArg) -> Self {
        match value {
            LanguageArg::Rust => Self::Rust,
            LanguageArg::JavaScript => Self::JavaScript,
            LanguageArg::TypeScript => Self::TypeScript,
            LanguageArg::Go => Self::Go,
            LanguageArg::Python => Self::Python,
            LanguageArg::Java => Self::Java,
            LanguageArg::Php => Self::Php,
            LanguageArg::Perl => Self::Perl,
            LanguageArg::Bash => Self::Bash,
            LanguageArg::Zsh => Self::Zsh,
            LanguageArg::C => Self::C,
            LanguageArg::Cpp => Self::Cpp,
            LanguageArg::CSharp => Self::CSharp,
            LanguageArg::Swift => Self::Swift,
            LanguageArg::Scala => Self::Scala,
            LanguageArg::Lua => Self::Lua,
        }
    }
}

fn main() -> Result<(), TreeHuggerError> {
    // Handle dynamic completion requests (COMPLETE=<shell> hug ...). This must
    // run before any other stdout writes; it exits the process if a completion
    // request was served.
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    // Handle completions command early (doesn't need file processing)
    if let Command::Completions(args) = &cli.command {
        if let Err(err) = print_completions(args.shell) {
            eprintln!("failed to write completions: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let output_format = cli.output_format();

    let root_dir = current_dir()?;
    let git_root = find_git_root(&root_dir).unwrap_or_else(|_| root_dir.clone());
    let pkg_root = find_package_root(&root_dir, &git_root);
    let display_root = Some(git_root.clone());

    // Handle god-files command early (separate analysis pipeline)
    if let Command::GodFiles(args) = &cli.command {
        let dir = args.dir.as_deref().unwrap_or_else(|| Path::new("."));
        // Reject an invalid scan root up front. A missing path or a non-directory
        // is an invocation error, not a successful empty scan; collapsing it into
        // a 0/0 report would hide CI/configuration mistakes.
        if !dir.is_dir() {
            let reason = if dir.exists() {
                "god-files scan path is not a directory"
            } else {
                "god-files scan directory does not exist"
            };
            return Err(TreeHuggerError::Io {
                path: dir.to_path_buf(),
                source: std::io::Error::other(reason),
            });
        }
        let god_files = GodFiles::new(dir);
        let analyses = god_files.analysis();
        let output_config = OutputConfig::new(output_format);

        match output_format {
            OutputFormat::Json => {
                // `--high-risk` filters the emitted set for every output format,
                // not just the rendered report (spec §5.1).
                let emitted: Vec<&GodAnalysis> = if args.high_risk {
                    analyses
                        .iter()
                        .filter(|a| a.risk == RiskBand::High)
                        .collect()
                } else {
                    analyses.iter().collect()
                };
                let json =
                    serde_json::to_string_pretty(&emitted).map_err(|source| TreeHuggerError::Io {
                        path: PathBuf::from("<stdout>"),
                        source: std::io::Error::other(source),
                    })?;
                println!("{json}");
            }
            OutputFormat::Pretty | OutputFormat::Plain => {
                render_god_files(analyses, &output_config, args.high_risk);
            }
        }
        return Ok(());
    }

    let command_kind = cli.command.kind().expect("completions already handled");
    let analysis_options = AnalysisOptions::from_command(&command_kind);

    if matches!(
        command_kind,
        CommandKind::Lint {
            list_rules: true,
            ..
        }
    ) {
        render_rule_list(output_format)?;
        return Ok(());
    }

    // God-files and completions are handled earlier; every remaining
    // subcommand carries a `ScanOptions`.
    let scan = cli
        .command
        .scan_options()
        .expect("every scanning subcommand carries scan options");
    let language = scan.language.map(ProgrammingLanguage::from);
    let output_config = OutputConfig::new(output_format);

    let scan_filters = classify_filters(cli.command.filters(), &command_kind, language);
    if cli.json_full {
        reject_json_full_misuse(&cli.command, &scan_filters);
    }

    // `--file` values join the positional file filters as alternatives; a
    // value naming an existing file is scanned whatever the ignore rules say.
    let targets = cli.command.targets();
    let (file_inputs, explicit_files) = match targets {
        Some(targets) => resolve_file_selectors(&targets.files, &root_dir).unwrap_or_else(|message| {
            Cli::command()
                .error(clap::error::ErrorKind::InvalidValue, message)
                .exit()
        }),
        None => (Vec::new(), Vec::new()),
    };
    let file_filters: Vec<String> = scan_filters.file_filters.iter().cloned().chain(file_inputs).collect();
    let files = collect_selected_files(
        &pkg_root,
        &file_filters,
        &explicit_files,
        &scan.exclude_files,
        language,
    )?;

    // One cache directory for every scan: the entry key names the file by
    // its identity, so a file reached from two scan roots shares one entry.
    let symbol_cache = (!scan.no_cache)
        .then(|| SymbolCache::new(PersistentCache::default_cache(CACHE_PROJECT_ID, true)));
    let analyzer = Analyzer::new(symbol_cache, language, analysis_options.experimental_semantics);

    let selection = cli.command.selection_args().map(|args| {
        Selection::new(args, &scan_filters.symbol_names, &scan.exclude_symbols, cli.command.exported())
    });
    if let Some(command) = selection
        .as_ref()
        .and_then(|selection| declarations_command(&command_kind, selection))
    {
        let format = if cli.json_full {
            ReportFormat::JsonFull
        } else if matches!(output_format, OutputFormat::Json) {
            ReportFormat::Json
        } else {
            ReportFormat::Terminal {
                styled: output_config.use_colors,
                show_comments: scan.comments,
            }
        };
        let status = declarations::run(
            DeclarationScan {
                command,
                files,
                scan_root: pkg_root,
                git_root,
                selection: selection.unwrap_or_default(),
                prelude: cli.command.prelude(),
                listing: ListingOptions {
                    group_by_file: scan.group_by_file,
                    group_by_module: scan.group_by_module,
                    sort: cli.command.sort().unwrap_or_default(),
                },
                format,
                verbose: cli.verbose,
            },
            &analyzer,
        )?;
        if status != 0 {
            std::process::exit(status);
        }
        return Ok(());
    }

    // `imports` and `lint` keep their own report shapes. Lint asks for
    // diagnostics; imports need only the declarations pass, which carries
    // the import records.
    let query = match command_kind {
        CommandKind::Lint { .. } => SymbolQuery::new().capabilities([Capability::Diagnostics]),
        _ => SymbolQuery::new(),
    };
    let import_selection = match &cli.command {
        Command::Imports(args) => ImportSelection::new(&args.targets, &scan.exclude_symbols),
        _ => ImportSelection::default(),
    };
    let mut stats = CacheStats::new(analyzer.uses_cache());
    let mut stderr = std::io::stderr();
    let mut failed_files = 0usize;
    let mut summaries = Vec::new();
    let mut symbol_indexes = Vec::new();
    let mut adapter_metadata = Vec::new();
    let _ = in_file_order::<_, ()>(
        &files,
        |file| analyzer.analyze(file, &query),
        |file, analyzed| {
            let analyzed = match analyzed {
                Ok(analyzed) => analyzed,
                Err(error) => {
                    eprintln!("error: {error}");
                    failed_files += 1;
                    return ControlFlow::Continue(());
                }
            };
            stats.record(
                &display_path(file, display_root.as_deref()),
                &analyzed.cache,
                cli.verbose,
                &mut stderr,
            );
            let mut index = analyzed.result.index;
            if !import_selection.is_empty() {
                let language = index.language;
                index.imports.retain(|import| import_selection.matches(import, language));
            }
            let mut summary = summarize_file(&index, &command_kind);
            if let Err(error) = add_external_lint_diagnostics(
                &mut summary,
                &command_kind,
                &pkg_root,
                &analysis_options,
                &mut adapter_metadata,
            ) {
                eprintln!("error: {error}");
                failed_files += 1;
                return ControlFlow::Continue(());
            }
            if matches!(output_format, OutputFormat::Json) {
                symbol_indexes.push(index);
            }
            summaries.push(summary);
            ControlFlow::Continue(())
        },
    );

    match output_format {
        OutputFormat::Json => {
            let package_language = language
                .or_else(|| summaries.first().map(|summary| summary.language))
                .unwrap_or(ProgrammingLanguage::Rust);

            let output = JsonOutput {
                schema_version: SchemaVersion::V4_0,
                root_dir,
                language: package_language,
                files: summaries.clone(),
                symbol_indexes,
                adapter_metadata: adapter_metadata.clone(),
            };

            let json =
                serde_json::to_string_pretty(&output).map_err(|source| TreeHuggerError::Io {
                    path: PathBuf::from("<stdout>"),
                    source: std::io::Error::other(source),
                })?;
            println!("{json}");
        }
        OutputFormat::Pretty | OutputFormat::Plain => {
            render_adapter_warnings(&adapter_metadata, &output_config);
            // Multi-file lint scans hide clean files; a single explicit
            // target still shows `(no diagnostics)` as confirmation.
            let suppress_clean_lint =
                matches!(command_kind, CommandKind::Lint { .. }) && summaries.len() > 1;
            let mut rendered_any = false;
            for summary in &summaries {
                rendered_any |= render_summary(
                    summary,
                    &command_kind,
                    &output_config,
                    display_root.as_deref(),
                    suppress_clean_lint,
                );
            }
            if suppress_clean_lint && !rendered_any {
                let msg = format!("No lint diagnostics in {} files.", summaries.len());
                if output_config.use_colors {
                    println!("{}", msg.green());
                } else {
                    println!("{msg}");
                }
            }
        }
    }
    if cli.verbose {
        eprintln!("{}", stats.summary());
    }

    // Exit with non-zero if lint found errors or a file could not be analyzed
    let error_count: usize = summaries
        .iter()
        .map(|summary| {
            summary
                .lint
                .iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .count()
                + summary
                    .syntax
                    .iter()
                    .filter(|d| d.severity == DiagnosticSeverity::Error)
                    .count()
        })
        .sum();
    let lint_failed = matches!(command_kind, CommandKind::Lint { .. }) && error_count > 0;
    if lint_failed || failed_files > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// The declaration command `command` runs with `selection`, if it is one.
fn declarations_command(command: &CommandKind, selection: &Selection) -> Option<Declarations> {
    match command {
        CommandKind::Functions => Some(Declarations::Functions),
        CommandKind::Types => Some(Declarations::Types),
        CommandKind::Symbols => Some(Declarations::Symbols),
        CommandKind::Classes {
            static_only,
            instance_only,
        } => Some(Declarations::Classes(Box::new(ClassSelection::new(
            selection,
            MemberFilter {
                static_only: *static_only,
                instance_only: *instance_only,
            },
        )))),
        CommandKind::Imports | CommandKind::Lint { .. } => None,
    }
}

/// Exits with an argument error when `--json-full` is given to a command
/// without declaration indexes or combined with a symbol selector: a
/// complete index is never filtered.
fn reject_json_full_misuse(command: &Command, scan_filters: &ScanFilters) {
    let message = if command.selection_args().is_none() {
        Some("`--json-full` applies only to `functions`, `types`, `symbols` and `classes`".to_string())
    } else {
        let mut selectors = command.record_selector_flags();
        if !scan_filters.symbol_names.is_empty() {
            selectors.push("a symbol-name FILTER");
        }
        (!selectors.is_empty()).then(|| {
            format!(
                "`--json-full` reports complete indexes and cannot be combined with symbol selectors ({})",
                selectors.join(", ")
            )
        })
    };
    if let Some(message) = message {
        Cli::command()
            .error(clap::error::ErrorKind::ArgumentConflict, message)
            .exit();
    }
}

/// The working directory as a [`file_identity`], so the scan, git and
/// package roots are spelled like every analyzed file (Windows reports the
/// working directory in whatever spelling it was given, such as an 8.3 short
/// name, while file identities use long names).
fn current_dir() -> Result<PathBuf, TreeHuggerError> {
    std::env::current_dir()
        .map(|dir| file_identity(&dir))
        .map_err(|source| TreeHuggerError::Io {
            path: PathBuf::from("."),
            source,
        })
}

/// The persistent-cache directory `hug` uses under
/// `<temp>/tree-hugger/cache/`, the same for every scan root.
const CACHE_PROJECT_ID: &str = "hug";

/// Directory names skipped when completing filter paths (build/dep output).
const COMPLETION_SKIPPED_DIRS: &[&str] =
    &["target", "node_modules", "dist", "build", "vendor", ".git"];

/// Dynamic completer for FILTER positionals.
///
/// Lists entries under the directory implied by `current` (or the cwd when
/// `current` has no directory component), keeping only:
///
/// - directories `hug` would descend into (skipping build output / dotfiles)
/// - files whose extension maps to a supported [`ProgrammingLanguage`]
fn complete_source_path(current: &OsStr) -> Vec<CompletionCandidate> {
    let value = Path::new(current);
    let (prefix, partial) = split_completion_input(value);
    let partial = partial.to_string_lossy();

    let search_root = if prefix.as_os_str().is_empty() {
        match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(_) => return Vec::new(),
        }
    } else if prefix.is_absolute() {
        prefix.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(prefix),
            Err(_) => return Vec::new(),
        }
    };

    let mut candidates = Vec::new();
    let Ok(entries) = std::fs::read_dir(&search_root) else {
        return candidates;
    };
    for entry in entries.filter_map(Result::ok) {
        let raw = entry.file_name();
        let Some(name) = raw.to_str() else {
            continue;
        };
        if !name.starts_with(partial.as_ref()) {
            continue;
        }
        if name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        if path.is_dir() {
            if COMPLETION_SKIPPED_DIRS.contains(&name) {
                continue;
            }
            candidates.push(CompletionCandidate::new(format!(
                "{}/",
                completion_candidate(prefix, name)
            )));
        } else if ProgrammingLanguage::from_path(&path).is_some() {
            candidates.push(CompletionCandidate::new(completion_candidate(prefix, name)));
        }
    }
    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
}

/// Joins a completion prefix and an entry name into one candidate path.
///
/// `Path::join` would splice in the platform separator, so a Windows candidate
/// came back mixed — `tree-hugger\cli/` — which the shell cannot re-complete
/// against and which diverges from the slash-separated paths `hug` prints.
fn completion_candidate(prefix: &Path, name: &str) -> String {
    if prefix.as_os_str().is_empty() {
        return name.to_string();
    }
    let base = to_portable_string(prefix);
    if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}

/// Splits a partial path into `(directory_prefix, name_partial)`.
///
/// Mirrors clap_complete's internal helper: a trailing `/` keeps the whole
/// value as the directory and uses an empty partial; otherwise the final
/// component is the partial filename.
fn split_completion_input(path: &Path) -> (&Path, &OsStr) {
    let ends_with_sep = path
        .as_os_str()
        .as_encoded_bytes()
        .last()
        .is_some_and(|b| *b == b'/' || *b == b'\\');
    if ends_with_sep || path.as_os_str().is_empty() {
        (path, OsStr::new(""))
    } else {
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let name = path.file_name().unwrap_or_else(|| OsStr::new(""));
        (parent, name)
    }
}

/// Writes the dynamic-completion registration script for `shell` to stdout.
///
/// The emitted script registers a hook that re-invokes this binary with
/// `COMPLETE=<shell>` set, so completion logic stays in sync with the CLI
/// definition (no stale static scripts).
fn print_completions(shell: Shell) -> std::io::Result<()> {
    let shell_name = shell.to_string();
    let shells = Shells::builtins();
    let completer = shells
        .completer(&shell_name)
        .expect("clap_complete::Shell variant has a builtin EnvCompleter");

    let mut cmd = Cli::command();
    cmd.build();
    let name = cmd.get_name().to_owned();
    let bin = cmd
        .get_bin_name()
        .map(str::to_owned)
        .unwrap_or_else(|| name.clone());
    let completer_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| bin.clone());

    completer.write_registration(
        "COMPLETE",
        &name,
        &bin,
        &completer_path,
        &mut std::io::stdout(),
    )
}

/// The v1 view of one file for `imports` and `lint`: its imports, or its
/// diagnostics with the lint policy applied.
fn summarize_file(index: &FileSymbolIndex, command: &CommandKind) -> FileSummary {
    let (lint, syntax) = diagnostics_from_index(index);
    let mut summary = FileSummary {
        file: index.file.clone(),
        language: index.language,
        hash: index.file_hash.clone(),
        symbols: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        locals: Vec::new(),
        lint,
        syntax,
    };

    match command {
        CommandKind::Imports => {
            summary.imports = import_symbols_from_index(index);
        }
        CommandKind::Lint {
            deny,
            warn,
            allow,
            strict,
            experimental_semantics,
            ..
        } => {
            let registry = RuleRegistry::new();
            summary.lint = apply_lint_policy(
                &summary.lint,
                &registry,
                deny,
                warn,
                allow,
                *strict,
                *experimental_semantics,
            );
        }
        CommandKind::Functions
        | CommandKind::Types
        | CommandKind::Symbols
        | CommandKind::Classes { .. } => {
            unreachable!("declaration commands are reported by `declarations::run`")
        }
    }

    summary
}

fn add_external_lint_diagnostics(
    summary: &mut FileSummary,
    command: &CommandKind,
    project_root: &Path,
    options: &AnalysisOptions,
    adapter_metadata: &mut Vec<AdapterMetadata>,
) -> Result<(), TreeHuggerError> {
    if !options.use_external_adapters
        || !matches!(command, CommandKind::Lint { .. })
        || !matches!(
            summary.language,
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript
        )
    {
        return Ok(());
    }

    let adapter = OxlintAdapter::new();
    let result = adapter
        .run(
            std::slice::from_ref(&summary.file),
            project_root,
            summary.language,
            &AdapterConfig::default(),
        )
        .map_err(|source| TreeHuggerError::Io {
            path: summary.file.clone(),
            source: std::io::Error::other(source),
        })?;

    adapter_metadata.push(result.metadata.clone());

    if !result.success {
        return Ok(());
    }

    let registry = RuleRegistry::new();
    let external_lint = result
        .diagnostics
        .into_iter()
        .filter_map(lint_diagnostic_from_external)
        .collect::<Vec<_>>();
    let external_lint = apply_lint_policy(
        &external_lint,
        &registry,
        &options.deny,
        &options.warn,
        &options.allow,
        options.strict,
        options.experimental_semantics,
    );
    summary.lint.extend(external_lint);
    Ok(())
}

fn render_adapter_warnings(metadata: &[AdapterMetadata], config: &OutputConfig) {
    let mut warned = HashSet::new();
    for item in metadata.iter().filter(|item| !item.tool_available) {
        if !warned.insert(item.tool_name.clone()) {
            continue;
        }
        if config.use_colors {
            eprintln!(
                "{} external lint adapter '{}' is unavailable; native diagnostics were used",
                "warning:".yellow(),
                item.tool_name
            );
        } else {
            eprintln!(
                "warning: external lint adapter '{}' is unavailable; native diagnostics were used",
                item.tool_name
            );
        }
    }
}

fn lint_diagnostic_from_external(diagnostic: Diagnostic) -> Option<LintDiagnostic> {
    if diagnostic.metadata.as_ref()?.source != tree_hugger::DiagnosticSource::ExternalTool {
        return None;
    }

    Some(LintDiagnostic {
        message: diagnostic.message,
        range: diagnostic.range,
        severity: diagnostic.severity,
        rule: diagnostic.rule,
        context: diagnostic.context,
        metadata: diagnostic.metadata,
    })
}

fn import_symbols_from_index(index: &FileSymbolIndex) -> Vec<ImportSymbol> {
    index
        .imports
        .iter()
        .map(|import| ImportSymbol {
            name: import.name.clone(),
            original_name: import.symbol.qualified_name.clone(),
            alias: import.alias.clone(),
            range: CodeRange {
                start_line: import.span.start.line as usize,
                start_column: import.span.start.column as usize,
                end_line: import.span.end.line as usize,
                end_column: import.span.end.column as usize,
                start_byte: import.span.start_byte as usize,
                end_byte: import.span.end_byte as usize,
            },
            statement_range: import.statement_span.as_ref().map(|span| CodeRange {
                start_line: span.start.line as usize,
                start_column: span.start.column as usize,
                end_line: span.end.line as usize,
                end_column: span.end.column as usize,
                start_byte: span.start_byte as usize,
                end_byte: span.end_byte as usize,
            }),
            language: index.language,
            file: index.file.clone(),
            source: import.source.clone(),
            // The v2 ImportRecord carries no re-export marker; the `unused-import`
            // lint runs on the v1 `imported_symbols()` path, which sets this.
            is_reexport: false,
        })
        .collect()
}

fn diagnostics_from_index(index: &FileSymbolIndex) -> (Vec<LintDiagnostic>, Vec<SyntaxDiagnostic>) {
    let mut lint = Vec::new();
    let mut syntax = Vec::new();

    for diagnostic in &index.diagnostics {
        match diagnostic.kind {
            DiagnosticKind::Syntax => syntax.push(SyntaxDiagnostic {
                message: diagnostic.message.clone(),
                range: diagnostic.range.clone(),
                severity: diagnostic.severity,
                context: diagnostic.context.clone(),
                metadata: diagnostic.metadata.clone(),
            }),
            DiagnosticKind::Lint | DiagnosticKind::Semantic => lint.push(LintDiagnostic {
                message: diagnostic.message.clone(),
                range: diagnostic.range.clone(),
                severity: diagnostic.severity,
                rule: diagnostic.rule.clone(),
                context: diagnostic.context.clone(),
                metadata: diagnostic.metadata.clone(),
            }),
        }
    }

    (lint, syntax)
}

/// Prints the `path (Language)` file header, with an optional hyperlink.
fn print_file_header(summary: &FileSummary, config: &OutputConfig, display_root: Option<&Path>) {
    let file_display = display_path(&summary.file, display_root);
    let header = if config.use_hyperlinks {
        hyperlink(&summary.file, 1, &file_display)
    } else {
        file_display
    };

    if config.use_colors {
        println!(
            "{} ({})",
            header.bold(),
            summary.language.to_string().dimmed()
        );
    } else {
        println!("{} ({})", header, summary.language);
    }
}

/// Renders one file's results, returning whether anything was printed.
///
/// When `suppress_clean_lint` is set (multi-file lint scans), a file with no
/// visible diagnostics is skipped entirely — header included — so real findings
/// are not buried under a `(no diagnostics)` line per clean file. A single-file
/// lint still prints `(no diagnostics)` as explicit confirmation.
fn render_summary(
    summary: &FileSummary,
    command: &CommandKind,
    config: &OutputConfig,
    display_root: Option<&Path>,
    suppress_clean_lint: bool,
) -> bool {
    if let CommandKind::Lint {
        lint_only,
        syntax_only,
        deny,
        warn,
        allow,
        strict,
        experimental_semantics,
        ..
    } = command
    {
        let registry = RuleRegistry::new();
        let filtered = apply_lint_policy(
            &summary.lint,
            &registry,
            deny,
            warn,
            allow,
            *strict,
            *experimental_semantics,
        );
        let has_visible =
            (!*syntax_only && !filtered.is_empty()) || (!*lint_only && !summary.syntax.is_empty());
        if suppress_clean_lint && !has_visible {
            return false;
        }

        print_file_header(summary, config, display_root);
        render_diagnostics_filtered(
            &filtered,
            &summary.syntax,
            &summary.file,
            config,
            *lint_only,
            *syntax_only,
        );
        println!();
        return true;
    }

    print_file_header(summary, config, display_root);
    render_imports(&summary.imports, config);

    println!();
    true
}

fn render_rule_list(format: OutputFormat) -> Result<(), TreeHuggerError> {
    let rules = rule_list_items();

    match format {
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(&rules).map_err(|source| TreeHuggerError::Io {
                    path: PathBuf::from("<stdout>"),
                    source: std::io::Error::other(source),
                })?;
            println!("{json}");
        }
        OutputFormat::Pretty | OutputFormat::Plain => {
            for rule in rules {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    rule.id,
                    rule.category,
                    severity_label(rule.default_severity),
                    rule.confidence,
                    if rule.enabled_by_default {
                        "default-on"
                    } else {
                        "default-off"
                    },
                    rule.title
                );
            }
        }
    }

    Ok(())
}

fn rule_list_items() -> Vec<RuleListItem> {
    let registry = RuleRegistry::new();
    let mut ids = registry.rule_ids().cloned().collect::<Vec<_>>();
    ids.sort();

    ids.into_iter()
        .filter_map(|id| registry.get(&id))
        .map(|rule| RuleListItem {
            id: rule.id.clone(),
            title: rule.title.clone(),
            category: rule.category.to_string(),
            default_severity: rule.default_severity,
            confidence: rule.confidence.to_string(),
            enabled_by_default: rule.enabled_by_default,
            requires_experimental_semantics: rule.requires_experimental_semantics,
            languages: rule.languages.clone(),
            aliases: rule.aliases.clone(),
        })
        .collect()
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    }
}

/// Returns the color style for a diagnostic severity.
fn style_for_severity(severity: DiagnosticSeverity) -> Style {
    match severity {
        DiagnosticSeverity::Error => Style::new().red().bold(),
        DiagnosticSeverity::Warning => Style::new().yellow(),
        DiagnosticSeverity::Info => Style::new().blue(),
    }
}

/// Renders the source context with underline marker.
fn render_source_context(context: &SourceContext, line_number: usize, config: &OutputConfig) {
    let line_num_width = line_number.to_string().len().max(4);

    if config.use_colors {
        // Empty gutter line
        println!("{:>width$} {}", "", "|".blue(), width = line_num_width);

        // Source line
        println!(
            "{:>width$} {} {}",
            line_number.to_string().blue(),
            "|".blue(),
            context.line_text,
            width = line_num_width
        );

        // Underline line
        let padding = " ".repeat(context.underline_column);
        let underline = "^".repeat(context.underline_length.max(1));
        println!(
            "{:>width$} {} {}{}",
            "",
            "|".blue(),
            padding,
            underline.yellow(),
            width = line_num_width
        );
    } else {
        // Plain text version
        println!("{:>width$} |", "", width = line_num_width);
        println!(
            "{:>width$} | {}",
            line_number,
            context.line_text,
            width = line_num_width
        );
        let padding = " ".repeat(context.underline_column);
        let underline = "^".repeat(context.underline_length.max(1));
        println!(
            "{:>width$} | {}{}",
            "",
            padding,
            underline,
            width = line_num_width
        );
    }
}

/// Applies CLI policy to lint diagnostics.
///
/// Filters out experimental semantic rules unless `--experimental-semantics`
/// is enabled, and adjusts severity based on `--deny`, `--warn`, `--allow`,
/// and `--strict` selectors.
fn apply_lint_policy(
    lint: &[LintDiagnostic],
    registry: &RuleRegistry,
    deny: &[RuleSelector],
    warn: &[RuleSelector],
    allow: &[RuleSelector],
    strict: bool,
    experimental_semantics: bool,
) -> Vec<LintDiagnostic> {
    use tree_hugger::rule_registry::apply_policy;

    lint.iter()
        .cloned()
        .filter_map(|mut diagnostic| {
            let rule_id = diagnostic.rule.as_deref()?;
            let mut rule = registry.get_or_default(rule_id);
            if !registry.has_rule(rule_id)
                && let Some(metadata) = &diagnostic.metadata
            {
                rule.default_severity = metadata.default_severity;
                rule.confidence = metadata.confidence;
                rule.category = metadata.category;
            }

            if rule.requires_experimental_semantics && !experimental_semantics {
                return None;
            }

            // Off-by-default rules (e.g. the `restriction` category) are silent
            // unless explicitly opted in by a `--warn`/`--deny` selector that
            // targets them. `--allow` only demotes severity and `--strict` only
            // escalates warnings; neither enables a disabled rule. Experimental
            // rules are exempt — they are gated above by `--experimental-semantics`.
            let explicitly_enabled = warn
                .iter()
                .chain(deny.iter())
                .any(|selector| selector.matches(&rule));
            if !rule.requires_experimental_semantics
                && !rule.enabled_by_default
                && !explicitly_enabled
            {
                return None;
            }

            let effective = apply_policy(&rule, deny, warn, allow, strict);
            diagnostic.severity = effective;

            let mut metadata = diagnostic.metadata.clone().unwrap_or_default();
            metadata.category = rule.category;
            metadata.confidence = rule.confidence;
            if metadata.source != tree_hugger::DiagnosticSource::ExternalTool {
                metadata.source = if rule.requires_experimental_semantics {
                    tree_hugger::DiagnosticSource::SemanticAnalysis
                } else {
                    tree_hugger::DiagnosticSource::TreeSitterQuery
                };
            }
            metadata.default_severity = rule.default_severity;
            metadata.effective_severity = effective;
            metadata.is_enabled_by_default = rule.enabled_by_default;
            metadata.requires_experimental_semantics = rule.requires_experimental_semantics;
            diagnostic.metadata = Some(metadata);

            Some(diagnostic)
        })
        .collect()
}

/// Renders diagnostics with optional filtering by kind.
///
/// When `lint_only` is true, shows only Lint and Semantic diagnostics.
/// When `syntax_only` is true, shows only Syntax diagnostics.
/// When both are false, shows all diagnostics.
fn render_diagnostics_filtered(
    lint: &[LintDiagnostic],
    syntax: &[SyntaxDiagnostic],
    file: &Path,
    config: &OutputConfig,
    lint_only: bool,
    syntax_only: bool,
) {
    // Convert to unified diagnostics
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    if !syntax_only {
        for lint_diag in lint {
            diagnostics.push(Diagnostic::from_lint(lint_diag.clone()));
        }
    }

    if !lint_only {
        for syntax_diag in syntax {
            diagnostics.push(Diagnostic::from_syntax(syntax_diag.clone()));
        }
    }

    if diagnostics.is_empty() {
        let label = if lint_only {
            "(no lint diagnostics)"
        } else if syntax_only {
            "(no syntax diagnostics)"
        } else {
            "(no diagnostics)"
        };
        if config.use_colors {
            println!("  {}", label.dimmed());
        } else {
            println!("  {}", label);
        }
        return;
    }

    for diagnostic in &diagnostics {
        render_unified_diagnostic(diagnostic, file, config);
    }
}

/// Renders a single unified diagnostic with kind indicator.
fn render_unified_diagnostic(diagnostic: &Diagnostic, file: &Path, config: &OutputConfig) {
    let severity_label = match diagnostic.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
    };

    let kind_label = match diagnostic.kind {
        DiagnosticKind::Lint => "[lint]",
        DiagnosticKind::Semantic => "[semantic]",
        DiagnosticKind::Syntax => "[syntax]",
    };

    let rule_display = diagnostic
        .rule
        .as_ref()
        .map(|r| format!(" [{}]", r))
        .unwrap_or_default();

    // Location line: "  --> file:line:col"
    let location = format!(
        "{}:{}:{}",
        to_portable_string(file),
        diagnostic.range.start_line,
        diagnostic.range.start_column
    );

    let location_display = if config.use_hyperlinks {
        hyperlink(file, diagnostic.range.start_line, &location)
    } else {
        location
    };

    if config.use_colors {
        let severity_style = style_for_severity(diagnostic.severity);
        let kind_style = match diagnostic.kind {
            DiagnosticKind::Lint => Style::new().cyan(),
            DiagnosticKind::Semantic => Style::new().magenta(),
            DiagnosticKind::Syntax => Style::new().red(),
        };
        println!(
            "{} {}{}: {}",
            kind_label.style(kind_style),
            severity_label.style(severity_style),
            rule_display.dimmed(),
            diagnostic.message
        );
        println!("  {} {}", "-->".blue(), location_display);
    } else {
        println!(
            "{} {}{}: {}",
            kind_label, severity_label, rule_display, diagnostic.message
        );
        println!("  --> {}", location_display);
    }

    // Render source context if available
    if let Some(context) = &diagnostic.context {
        render_source_context(context, diagnostic.range.start_line, config);
    }

    println!();
}

fn render_imports(imports: &[ImportSymbol], config: &OutputConfig) {
    if imports.is_empty() {
        if config.use_colors {
            println!("  {}", "(no imports)".dimmed());
        } else {
            println!("  (no imports)");
        }
        return;
    }

    let groups = group_imports(imports);
    for group in groups {
        let group = dedupe_import_group(&group);
        let (location, start_line) = format_import_locations(&group);
        let location_display = if config.use_hyperlinks {
            hyperlink(&group[0].file, start_line, &location)
        } else {
            location
        };

        let import_display = format_import_group_display(&group);

        if config.use_colors {
            println!(
                "  - {} {}",
                import_display.cyan(),
                location_display.dimmed()
            );
        } else {
            println!("  - {} {}", import_display, location_display);
        }
    }
}

// ============================================================================
// God-files rendering
// ============================================================================

/// Build the [`Terminal`] used to render the god-files report.
///
/// A non-styling terminal (no color, no OSC8) is used for `--plain`/`--json`
/// so `Prose` emits clean, escape-free bytes; the styled path detects real
/// capabilities so color and hyperlink degradation are capability-aware.
fn god_terminal(config: &OutputConfig) -> Terminal {
    if !config.use_colors {
        Terminal::builder()
            .is_tty(false)
            .color_depth(ColorDepth::None)
            .osc_link_support(false)
            .build()
    } else if color_forced() {
        // Forced color may run without a real TTY (CI, captured output, tests);
        // an optimistic terminal guarantees full color + OSC8 capabilities.
        Terminal::new_optimistic(120)
    } else {
        Terminal::new()
    }
}

/// Render a single line of `Prose` markup at a fixed indentation.
///
/// Indentation is applied outside the markup so leading whitespace is never
/// consumed by the markup parser, and word-wrap is disabled to preserve the
/// report's hand-laid columns. `plain` strips any residual escapes (emphasis
/// SGR is not gated by color depth) so `--plain`/`--json` output is clean.
fn prose_line(term: &Terminal, plain: bool, indent: usize, markup: &str) {
    let rendered = Prose::new(markup)
        .with_word_wrap(WordWrap::None)
        .render(term);
    let rendered = if plain {
        strip_escape_codes(rendered)
    } else {
        rendered
    };
    println!("{}{}", " ".repeat(indent), rendered);
}

/// Builds a `file://` URL for `path`, resolving a relative path against the
/// process working directory.
///
/// `Url::from_file_path` owns the platform spelling: a Windows path becomes
/// `file:///C:/…`, not the `file://C:%5C…` a percent-encoder would produce from
/// the native separators. Returns `None` for a path it cannot express as a URL.
fn file_url(path: &Path) -> Option<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    Url::from_file_path(absolute).ok().map(String::from)
}

/// Render the full god-files report via biscuit-terminal `Prose`.
fn render_god_files(analyses: &[GodAnalysis], config: &OutputConfig, high_risk_only: bool) {
    let term = god_terminal(config);
    let plain = !config.use_colors;

    let high_count = analyses.iter().filter(|a| a.risk == RiskBand::High).count();
    let moderate_count = analyses
        .iter()
        .filter(|a| a.risk == RiskBand::Moderate)
        .count();

    // Report heading. The high-risk count is always printed (including the 0/0
    // case for an empty scan). The moderate count is suppressed under
    // `--high-risk`, where moderate files are out of scope entirely.
    if !high_risk_only {
        prose_line(
            &term,
            plain,
            0,
            &format!(
                "- There are <yellow>{moderate_count}</yellow> files with moderate risk of being considered _god files_"
            ),
        );
    }
    prose_line(
        &term,
        plain,
        0,
        &format!(
            "- There are <red>{high_count}</red> files with <b>high risk</b> of being considered _god files_"
        ),
    );

    // High-risk section (omitted entirely when empty).
    if high_count > 0 {
        println!();
        prose_line(&term, plain, 0, "<b><uu>High risk</uu></b>");
        for analysis in analyses.iter().filter(|a| a.risk == RiskBand::High) {
            render_god_analysis(&term, plain, analysis);
        }
    }

    // Moderate-risk section. Suppressed in full (heading and bodies) under
    // `--high-risk`; otherwise omitted only when empty.
    if !high_risk_only && moderate_count > 0 {
        println!();
        prose_line(&term, plain, 0, "<b><uu>Moderate risk</uu></b>");
        for analysis in analyses.iter().filter(|a| a.risk == RiskBand::Moderate) {
            render_god_analysis(&term, plain, analysis);
        }
    }
}

/// Render a single god-file analysis.
fn render_god_analysis(term: &Terminal, plain: bool, analysis: &GodAnalysis) {
    let color = match analysis.risk {
        RiskBand::High => "red",
        RiskBand::Moderate => "yellow",
    };

    let rel = Prose::escape_text(&to_portable_string(&analysis.relative_path));
    let label = match file_url(Path::new(analysis.file.raw())).filter(|_| term.osc_link_support) {
        Some(url) => format!("<a href={}>{rel}</a>", Prose::quoted_attr(&url)),
        None => rel,
    };

    prose_line(
        term,
        plain,
        0,
        &format!(
            "- the {label} file is <b><{color}>{sloc}</{color}></b> lines of code",
            sloc = analysis.effective_sloc
        ),
    );

    // Diagnostic note for degraded (e.g. unparseable) analyses.
    if let Some(note) = &analysis.note {
        prose_line(
            term,
            plain,
            2,
            &format!("- <dim>note: {}</dim>", Prose::escape_text(note)),
        );
    }

    // Largest blocks.
    if !analysis.blocks.is_empty() {
        prose_line(
            term,
            plain,
            2,
            "- the largest blocks in this file are composed by these symbols:",
        );
        for block in &analysis.blocks {
            render_symbol_block(term, plain, block);
        }
        if analysis.blocks_truncated > 0 {
            prose_line(
                term,
                plain,
                4,
                &format!("- <dim>…and {} more</dim>", analysis.blocks_truncated),
            );
        }
    }

    // Compact signals line.
    prose_line(
        term,
        plain,
        2,
        &format!(
            "- <dim>top-level symbols: {tl} · max depth: {depth} · imports: {imports} · TODO/FIXME: {todo} · comments: {density:.0}%</dim>",
            tl = analysis.top_level_symbol_count,
            depth = analysis.max_nesting_depth,
            imports = analysis.import_fan_out,
            todo = analysis.todo_fixme_count,
            density = analysis.comment_density * 100.0,
        ),
    );

    // Refactor hints.
    for hint in &analysis.refactor_hints {
        prose_line(
            term,
            plain,
            2,
            &format!("- <dim>{}</dim>", format_refactor_hint(hint)),
        );
    }
}

/// Render a single symbol block line plus any container call-out.
fn render_symbol_block(term: &Terminal, plain: bool, block: &SymbolBlock) {
    let kind = block.kind.to_string();
    let kind_markup = match color_tag_for_kind(block.kind) {
        Some(tag) => format!("<{tag}>{kind}</{tag}>"),
        None => kind,
    };
    let name = Prose::escape_text(&block.name);

    let members = match &block.many_members {
        Some(callout) => format!("  ({} members)", callout.member_count),
        None => String::new(),
    };
    let doc = match &block.doc_summary {
        Some(summary) => format!(" <dim>— {}</dim>", Prose::escape_text(summary)),
        None => String::new(),
    };

    prose_line(
        term,
        plain,
        4,
        &format!(
            "- {kind_markup} <b>{name}</b>  <dim>[{start}–{end}]</dim>  {sloc} sloc{members}{doc}",
            start = block.start_line,
            end = block.end_line,
            sloc = block.sloc,
        ),
    );

    if let Some(callout) = &block.many_members {
        for member in &callout.members {
            let m_kind = member.kind.to_string();
            let m_kind_markup = match color_tag_for_kind(member.kind) {
                Some(tag) => format!("<{tag}>{m_kind}</{tag}>"),
                None => m_kind,
            };
            let m_name = Prose::escape_text(&member.name);
            prose_line(
                term,
                plain,
                6,
                &format!("- {m_kind_markup} {m_name}  {} sloc", member.sloc),
            );
        }
        let remaining = callout.member_count.saturating_sub(callout.members.len());
        if remaining > 0 {
            prose_line(term, plain, 6, &format!("- <dim>…and {remaining} more</dim>"));
        }
    }
}

/// Format a refactor hint as a short human-readable sentence (spec §5.3).
fn format_refactor_hint(hint: &RefactorHint) -> String {
    match hint {
        RefactorHint::DominatedBySingleSymbol { name, share } => {
            format!(
                "likely refactor: `{}` holds {:.0}% of the code — split by responsibility",
                Prose::escape_text(name),
                share * 100.0
            )
        }
        RefactorHint::ManyUnrelatedTopLevel { count } => {
            format!("likely refactor: {count} unrelated top-level symbols — split by responsibility")
        }
        RefactorHint::DeeplyNested { depth } => {
            format!("likely refactor: deeply nested (depth {depth}) — extract / flatten")
        }
        RefactorHint::HighCoupling { import_fan_out } => {
            format!(
                "likely refactor: high coupling ({import_fan_out} imports) — expect wide refactor blast radius"
            )
        }
        RefactorHint::LowCodeDensity { comment_density } => {
            format!(
                "likely refactor: low code density ({:.0}% comments) — mostly comments/docs",
                comment_density * 100.0
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_relativizes_beneath_the_root() {
        let root = Path::new("/repo");
        assert_eq!(
            display_path(Path::new("/repo/src/main.rs"), Some(root)),
            "src/main.rs"
        );
    }

    #[test]
    fn display_path_keeps_paths_outside_the_root_whole() {
        assert_eq!(
            display_path(Path::new("/elsewhere/x.rs"), Some(Path::new("/repo"))),
            "/elsewhere/x.rs"
        );
    }

    /// The regression the `hug` CLI shipped on Windows: relative output paths
    /// came back `tree-hugger\cli\src\main.rs`, diverging from the paths the
    /// caller typed and from the other platforms' output.
    #[test]
    #[cfg(windows)]
    fn display_path_renders_windows_separators_as_slashes() {
        let root = Path::new(r"C:\repo");
        assert_eq!(
            display_path(
                Path::new(r"C:\repo\tree-hugger\cli\src\main.rs"),
                Some(root)
            ),
            "tree-hugger/cli/src/main.rs"
        );
    }

    /// A percent-encoder over the native spelling produced `file://C:%5Crepo…`,
    /// which no terminal resolves.
    #[test]
    #[cfg(windows)]
    fn file_url_uses_the_windows_drive_uri_shape() {
        assert_eq!(
            file_url(Path::new(r"C:\repo\a b.rs")).as_deref(),
            Some("file:///C:/repo/a%20b.rs")
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn file_url_percent_encodes_reserved_characters() {
        assert_eq!(
            file_url(Path::new("/repo/a b.rs")).as_deref(),
            Some("file:///repo/a%20b.rs")
        );
    }

    #[test]
    fn completion_candidate_without_a_prefix_is_the_bare_name() {
        assert_eq!(
            completion_candidate(Path::new(""), "tree-hugger"),
            "tree-hugger"
        );
    }

    #[test]
    fn completion_candidate_does_not_double_a_trailing_separator() {
        assert_eq!(
            completion_candidate(Path::new("tree-hugger/"), "cli"),
            "tree-hugger/cli"
        );
    }

    /// A `Path::join` here produced `tree-hugger\cli` on Windows, giving the
    /// shell a mixed-separator candidate once the trailing `/` was appended.
    #[test]
    fn completion_candidate_joins_with_a_slash() {
        assert_eq!(
            completion_candidate(Path::new("tree-hugger"), "cli"),
            "tree-hugger/cli"
        );
    }

    #[test]
    fn hyperlink_wraps_text_in_an_osc8_file_link() {
        let absolute = if cfg!(windows) {
            r"C:\repo\x.rs"
        } else {
            "/repo/x.rs"
        };
        let linked = hyperlink(Path::new(absolute), 12, "x.rs");
        assert!(linked.contains("\x1b]8;;file:///"), "{linked}");
        assert!(linked.contains("#L12"), "{linked}");
        assert!(linked.ends_with("x.rs\x1b]8;;\x1b\\"), "{linked}");
    }
}
