use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsStr;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use clap_complete::env::{CompleteEnv, Shells};
use clap_complete::Shell;
use owo_colors::{OwoColorize, Style};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use tree_hugger::adapter::{AdapterConfig, ExternalDiagnosticAdapter, OxlintAdapter};
use tree_hugger::cache::{
    AnalyzerFingerprint, CacheConfig, FileCacheKey, InMemorySymbolCache, InProcessCache,
    PersistentCache, SymbolSnapshot,
};
use tree_hugger::{
    CodeRange, Diagnostic, DiagnosticKind, DiagnosticSeverity, FieldInfo,
    FileSummary, FileSymbolIndex, FunctionSignature, ImportSymbol, LintDiagnostic, ParameterInfo,
    ProgrammingLanguage, RuleRegistry, RuleSelector, SchemaVersion, SourceContext, SymbolInfo,
    SymbolKind, SyntaxDiagnostic, TreeFile, TreeHuggerError, TypeMetadata, VariantInfo,
    find_git_root, find_package_root,
};
mod import_format;
mod prelude;
mod scanner;

use import_format::*;
use prelude::*;
use scanner::*;


#[derive(Parser, Debug)]
#[command(
    name = "hug",
    version,
    about = "Tree Hugger diagnostics and symbol tooling"
)]
struct Cli {
    /// Glob patterns for files to exclude from scanning
    #[arg(
        long,
        value_name = "GLOB",
        global = true,
        display_order = 10,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    exclude_files: Vec<String>,

    /// Glob patterns for symbol names to exclude from output
    #[arg(long, value_name = "GLOB", global = true, display_order = 11)]
    exclude_symbols: Vec<String>,

    /// Force a specific language. Explicitly named files are parsed with this
    /// language even when their extension maps elsewhere or is absent; directory
    /// scans are restricted to this language's extensions.
    #[arg(long, value_enum, global = true)]
    language: Option<LanguageArg>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Disable colors and hyperlinks (plain text output)
    #[arg(long, global = true)]
    plain: bool,

    /// Disable persistent and in-process caching.
    ///
    /// All files are re-parsed and re-analyzed on every invocation.
    #[arg(long, global = true)]
    no_cache: bool,

    /// Show symbol-level documentation comments in output
    #[arg(long, global = true)]
    comments: bool,

    /// Group symbol output by file path
    #[arg(long, global = true)]
    group_by_file: bool,

    /// Group symbol output by module path (directory/module scope)
    #[arg(long, global = true)]
    group_by_module: bool,

    /// Sort symbols by kind before name
    #[arg(long, global = true)]
    sort_by_kind: bool,

    /// Sort symbols by module before other sort keys
    #[arg(long, global = true)]
    sort_by_module: bool,

    #[command(subcommand)]
    command: Command,
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
    /// or extensions such as `rs` / `*.rs`) limit which files are scanned.
    ///
    /// Remaining filters are symbol-name filters.
    ///
    /// `parse_width` => fuzzy contains match.
    ///
    /// `*width*` => wildcard match.
    ///
    /// `parse_width_spec!` => exact symbol name match.
    #[arg(
        value_name = "FILTER",
        num_args = 0..,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    filters: Vec<String>,

    /// Show only exported (public) symbols
    #[arg(long, conflicts_with = "prelude", display_order = 30)]
    exported: bool,

    /// Show only symbols explicitly exported by the prelude module
    #[arg(long, conflicts_with = "exported", display_order = 31)]
    prelude: bool,
}

/// Arguments for the classes command
#[derive(clap::Args, Debug, Clone)]
struct ClassArgs {
    /// File path or class-name filters
    ///
    /// File-like filters (paths, globs, or extensions) limit scanned files.
    ///
    /// Remaining filters are class-name filters.
    ///
    /// `Widget` => fuzzy contains match.
    ///
    /// `*Widget*` => wildcard match.
    ///
    /// `Widget!` => exact class name match.
    #[arg(
        value_name = "FILTER",
        num_args = 0..,
        value_hint = ValueHint::AnyPath,
        add = ArgValueCompleter::new(complete_source_path),
    )]
    filters: Vec<String>,

    /// Filter by class name
    #[arg(long, short = 'n')]
    name: Option<String>,

    /// Show only static members
    #[arg(long)]
    static_only: bool,

    /// Show only instance members
    #[arg(long)]
    instance_only: bool,

    /// Show only exported (public) classes
    #[arg(long, conflicts_with = "prelude")]
    exported: bool,

    /// Show only classes explicitly exported by the prelude module
    #[arg(long, conflicts_with = "exported")]
    prelude: bool,
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
}

/// Arguments for the completions command
#[derive(clap::Args, Debug, Clone)]
struct CompletionsArgs {
    /// The shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// List functions in the file(s)
    Functions(CommonArgs),
    /// List types in the file(s)
    Types(CommonArgs),
    /// List all symbols in the file(s)
    Symbols(CommonArgs),
    /// List imported symbols in the file(s)
    Imports(CommonArgs),
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
}

impl Command {
    /// Returns positional filter tokens from the subcommand.
    fn filters(&self) -> &[String] {
        match self {
            Self::Functions(args)
            | Self::Types(args)
            | Self::Symbols(args)
            | Self::Imports(args) => &args.filters,
            Self::Lint(args) => &args.filters,
            Self::Classes(args) => &args.filters,
            Self::Completions(_) => &[],
        }
    }

    /// Returns whether the `--exported` flag was set.
    fn exported(&self) -> bool {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => args.exported,
            Self::Classes(args) => args.exported,
            Self::Imports(_) | Self::Lint(_) | Self::Completions(_) => false,
        }
    }

    /// Returns whether the `--prelude` flag was set.
    fn prelude(&self) -> bool {
        match self {
            Self::Functions(args) | Self::Types(args) | Self::Symbols(args) => args.prelude,
            Self::Classes(args) => args.prelude,
            Self::Imports(_) | Self::Lint(_) | Self::Completions(_) => false,
        }
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
            }),
            Self::Classes(args) => Some(CommandKind::Classes {
                name_filter: args.name.clone(),
                static_only: args.static_only,
                instance_only: args.instance_only,
            }),
            Self::Completions(_) => None,
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
    },
    Classes {
        name_filter: Option<String>,
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

    fn fingerprint(&self) -> String {
        format!(
            "experimental_semantics={};deny={};warn={};allow={};strict={};external_adapters={}",
            self.experimental_semantics,
            selectors_fingerprint(&self.deny),
            selectors_fingerprint(&self.warn),
            selectors_fingerprint(&self.allow),
            self.strict,
            self.use_external_adapters,
        )
    }
}

fn selectors_fingerprint(selectors: &[RuleSelector]) -> String {
    let mut values = selectors
        .iter()
        .map(|selector| match selector {
            RuleSelector::All => "all".to_string(),
            RuleSelector::Rule(rule) => format!("rule:{rule}"),
            RuleSelector::Category(category) => format!("category:{category}"),
        })
        .collect::<Vec<_>>();
    values.sort();
    values.join(",")
}

/// Filter mode for symbol output.
#[derive(Debug, Clone)]
enum SymbolFilter {
    /// No filtering — show all symbols
    None,
    /// Show only exported (public) symbols
    Exported,
    /// Show only symbols whose name appears in the prelude
    Prelude(PreludeFilter),
}

/// Ordering/grouping switches for symbol rendering.
#[derive(Debug, Clone, Copy)]
struct SymbolRenderOptions {
    group_by_file: bool,
    group_by_module: bool,
    sort_by_kind: bool,
    sort_by_module: bool,
}

/// Summary of a class with its members partitioned by static/instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClassSummary {
    /// The class symbol
    pub class: SymbolInfo,
    /// Static methods
    pub static_methods: Vec<SymbolInfo>,
    /// Instance methods
    pub instance_methods: Vec<SymbolInfo>,
    /// Static fields
    pub static_fields: Vec<FieldInfo>,
    /// Instance fields
    pub instance_fields: Vec<FieldInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonOutput {
    schema_version: SchemaVersion,
    root_dir: PathBuf,
    language: ProgrammingLanguage,
    files: Vec<FileSummary>,
    symbol_indexes: Vec<tree_hugger::FileSymbolIndex>,
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
    show_comments: bool,
}

static SOURCE_LINE_CACHE: OnceLock<Mutex<HashMap<PathBuf, Vec<String>>>> = OnceLock::new();

impl OutputConfig {
    fn new(format: OutputFormat, show_comments: bool) -> Self {
        match format {
            OutputFormat::Pretty => {
                // Check NO_COLOR environment variable and TTY
                let no_color = std::env::var("NO_COLOR").is_ok();
                let is_tty = std::io::stdout().is_terminal();
                let use_colors = !no_color && is_tty;
                Self {
                    use_colors,
                    use_hyperlinks: use_colors && is_tty,
                    show_comments,
                }
            }
            OutputFormat::Plain | OutputFormat::Json => Self {
                use_colors: false,
                use_hyperlinks: false,
                show_comments,
            },
        }
    }
}

/// Returns the color style for a symbol kind.
fn style_for_kind(kind: SymbolKind) -> Style {
    match kind {
        SymbolKind::Function | SymbolKind::Method => Style::new().green(),
        SymbolKind::Type | SymbolKind::Class | SymbolKind::Interface => Style::new().magenta(),
        SymbolKind::Enum => Style::new().cyan(),
        SymbolKind::Trait => Style::new().yellow(),
        SymbolKind::Variable | SymbolKind::Parameter => Style::new().blue(),
        SymbolKind::Field => Style::new().cyan(),
        SymbolKind::Namespace | SymbolKind::Module => Style::new().yellow(),
        SymbolKind::Macro => Style::new().red(),
        SymbolKind::Constant => Style::new().bright_blue(),
        SymbolKind::Unknown => Style::new(),
    }
}

#[derive(Debug, Clone)]
struct SymbolPresentation {
    label: String,
    style_kind: SymbolKind,
}

fn symbol_presentation(symbol: &SymbolInfo) -> SymbolPresentation {
    if symbol.language == ProgrammingLanguage::Rust
        && symbol.kind == SymbolKind::Type
        && let Some(line) = source_line_for_symbol(symbol)
    {
        let trimmed = line.trim_start();
        let label = if contains_word(trimmed, "struct") {
            "struct"
        } else if contains_word(trimmed, "union") {
            "union"
        } else {
            "type"
        };
        return SymbolPresentation {
            label: label.to_string(),
            style_kind: SymbolKind::Type,
        };
    }

    if matches!(
        symbol.kind,
        SymbolKind::Variable | SymbolKind::Field | SymbolKind::Constant
    ) && let Some(token) = binding_token_for_symbol(symbol)
    {
        let style_kind = match token.as_str() {
            "const" | "static" => SymbolKind::Constant,
            "field" => SymbolKind::Field,
            _ => symbol.kind,
        };

        return SymbolPresentation {
            label: token,
            style_kind,
        };
    }

    SymbolPresentation {
        label: symbol.kind.to_string(),
        style_kind: symbol.kind,
    }
}

fn binding_token_for_symbol(symbol: &SymbolInfo) -> Option<String> {
    let line = source_line_for_symbol(symbol)?;
    let trimmed = line.trim_start();

    let token = match symbol.language {
        ProgrammingLanguage::Rust => {
            if symbol.name == "self" {
                Some("self")
            } else if trimmed.starts_with("const ") || trimmed.contains(" const ") {
                Some("const")
            } else if trimmed.starts_with("static ") || trimmed.contains(" static mut ") {
                Some("static")
            } else if trimmed.starts_with("let ") || trimmed.contains(" let ") {
                Some("let")
            } else if symbol.kind == SymbolKind::Field {
                Some("field")
            } else {
                None
            }
        }
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            if contains_word(trimmed, "const") {
                Some("const")
            } else if contains_word(trimmed, "let") {
                Some("let")
            } else if contains_word(trimmed, "var") {
                Some("var")
            } else if symbol.kind == SymbolKind::Field {
                Some("field")
            } else {
                None
            }
        }
        ProgrammingLanguage::Swift => {
            if contains_word(trimmed, "let") {
                Some("let")
            } else if contains_word(trimmed, "var") {
                Some("var")
            } else if symbol.kind == SymbolKind::Field {
                Some("field")
            } else {
                None
            }
        }
        ProgrammingLanguage::Go => {
            if contains_word(trimmed, "const") {
                Some("const")
            } else if contains_word(trimmed, "var") {
                Some("var")
            } else if trimmed.contains(":=") {
                Some(":=")
            } else {
                None
            }
        }
        ProgrammingLanguage::CSharp => {
            if contains_word(trimmed, "const") {
                Some("const")
            } else if contains_word(trimmed, "var") {
                Some("var")
            } else if symbol.kind == SymbolKind::Field
                || (symbol.kind == SymbolKind::Variable
                    && trimmed.ends_with(';')
                    && !trimmed.contains('('))
            {
                Some("field")
            } else {
                None
            }
        }
        ProgrammingLanguage::Python => {
            if trimmed.starts_with("for ") {
                Some("for")
            } else if trimmed.contains(":=") {
                Some(":=")
            } else if trimmed.contains('=') {
                Some("=")
            } else {
                None
            }
        }
        ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh => {
            if contains_word(trimmed, "local") {
                Some("local")
            } else if contains_word(trimmed, "declare") {
                Some("declare")
            } else if contains_word(trimmed, "typeset") {
                Some("typeset")
            } else if contains_word(trimmed, "readonly") {
                Some("readonly")
            } else if contains_word(trimmed, "export") {
                Some("export")
            } else {
                None
            }
        }
        _ => {
            if symbol.kind == SymbolKind::Constant {
                Some("const")
            } else if symbol.kind == SymbolKind::Field {
                Some("field")
            } else {
                None
            }
        }
    };

    token.map(str::to_string)
}

fn contains_word(line: &str, word: &str) -> bool {
    line.split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .any(|token| token == word)
}

fn source_line_for_symbol(symbol: &SymbolInfo) -> Option<String> {
    let cache = SOURCE_LINE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().ok()?;

    if !guard.contains_key(&symbol.file) {
        let source = std::fs::read_to_string(&symbol.file).ok()?;
        let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
        guard.insert(symbol.file.clone(), lines);
    }

    let lines = guard.get(&symbol.file)?;
    lines
        .get(symbol.range.start_line.saturating_sub(1))
        .cloned()
}

/// Creates an OSC8 hyperlink for a file path with line number.
fn hyperlink(path: &Path, line: usize, text: &str) -> String {
    const FILE_URL_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC.remove(b'/').remove(b':');
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|root| root.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let path_str = absolute_path.to_string_lossy();
    let encoded = utf8_percent_encode(&path_str, FILE_URL_ENCODE_SET);
    format!(
        "\x1b]8;;file://{}#L{}\x1b\\{}\x1b]8;;\x1b\\",
        encoded, line, text
    )
}

fn display_path(path: &Path, root: Option<&Path>) -> String {
    if let Some(root) = root
        && let Ok(relative) = path.strip_prefix(root)
    {
        return relative.display().to_string();
    }
    path.display().to_string()
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

    let language = cli.language.map(ProgrammingLanguage::from);
    let filters = cli.command.filters();
    let output_format = cli.output_format();
    let output_config = OutputConfig::new(output_format, cli.comments);
    let render_options = SymbolRenderOptions {
        group_by_file: cli.group_by_file,
        group_by_module: cli.group_by_module,
        sort_by_kind: cli.sort_by_kind,
        sort_by_module: cli.sort_by_module,
    };

    let root_dir = current_dir()?;
    let git_root = find_git_root(&root_dir).unwrap_or_else(|_| root_dir.clone());
    let pkg_root = find_package_root(&root_dir, &git_root);
    let display_root = Some(git_root.clone());

    let command_kind = cli.command.kind().expect("completions already handled");
    let analysis_options = AnalysisOptions::from_command(&command_kind);
    let scan_filters = classify_filters(filters, &command_kind, language);
    let excluded_symbol_globs = cli
        .exclude_symbols
        .iter()
        .filter_map(|glob| normalize_excluded_symbol_glob(glob))
        .collect::<Vec<_>>();

    let mut files = if scan_filters.file_filters.is_empty() {
        collect_files(&pkg_root, &[], &cli.exclude_files, language)?
    } else {
        collect_files(
            &pkg_root,
            &scan_filters.file_filters,
            &cli.exclude_files,
            language,
        )?
    };

    // Build symbol filter
    let symbol_filter = if cli.command.exported() {
        SymbolFilter::Exported
    } else if cli.command.prelude() {
        let prelude_filter = resolve_prelude_symbols(&pkg_root)?;
        SymbolFilter::Prelude(prelude_filter)
    } else {
        SymbolFilter::None
    };

    if matches!(command_kind, CommandKind::Symbols)
        && let SymbolFilter::Prelude(filter) = &symbol_filter
    {
        for prelude_file in filter.exports_by_file.keys() {
            if !files.iter().any(|file| file == prelude_file) {
                files.push(prelude_file.clone());
            }
        }
        files.sort();
    }

    let analysis_cache = InMemorySymbolCache::new(files.len().max(1));
    let in_process_cache = InProcessCache::with_config(CacheConfig {
        enabled: !cli.no_cache,
        persistent: false,
        capacity: files.len().max(1),
    });
    let persistent_cache = if cli.no_cache {
        None
    } else {
        let project_id = pkg_root
            .to_string_lossy()
            .replace(['/', '\\', ':', ' ', '.'], "_");
        Some(PersistentCache::default_cache(
            &project_id, true))
    };

    // Handle classes command separately due to different output structure
    if let CommandKind::Classes {
        name_filter,
        static_only,
        instance_only,
    } = &command_kind
    {
        let mut all_class_summaries: Vec<(PathBuf, ProgrammingLanguage, Vec<ClassSummary>)> =
            Vec::new();

        for file in files {
            let tree_file = TreeFile::with_language(&file, language)?;
            let index = analyze_tree_file(
                tree_file,
                &analysis_options,
                &analysis_cache,
                &in_process_cache,
                &persistent_cache,
            )?;
            let mut class_summaries = extract_class_summaries(
                &index,
                name_filter.as_deref(),
                *static_only,
                *instance_only,
            )?;

            // Apply symbol filter to class summaries
            match &symbol_filter {
                SymbolFilter::Exported => {
                    let exported: HashSet<String> = exported_symbol_infos(&index)
                        .into_iter()
                        .map(|s| s.name)
                        .collect();
                    class_summaries.retain(|cs| exported.contains(&cs.class.name));
                }
                SymbolFilter::Prelude(filter) => {
                    class_summaries.retain(|cs| filter.names.contains(&cs.class.name));
                }
                SymbolFilter::None => {}
            }

            if !scan_filters.symbol_globs.is_empty() {
                class_summaries.retain(|summary| {
                    matches_symbol_filters(&summary.class.name, &scan_filters.symbol_globs)
                });
            }
            if !excluded_symbol_globs.is_empty() {
                class_summaries.retain(|summary| {
                    !matches_symbol_filters(&summary.class.name, &excluded_symbol_globs)
                });
            }

            if !class_summaries.is_empty() {
                all_class_summaries.push((index.file.clone(), index.language, class_summaries));
            }
        }

        match output_format {
            OutputFormat::Json => {
                let json =
                    serde_json::to_string_pretty(&all_class_summaries).map_err(|source| {
                        TreeHuggerError::Io {
                            path: PathBuf::from("<stdout>"),
                            source: std::io::Error::other(source),
                        }
                    })?;
                println!("{json}");
            }
            OutputFormat::Pretty | OutputFormat::Plain => {
                for (file, lang, summaries) in all_class_summaries {
                    render_class_summaries(
                        &file,
                        lang,
                        &summaries,
                        &output_config,
                        display_root.as_deref(),
                    );
                }
            }
        }
        return Ok(());
    }

    let mut summaries = Vec::new();
    let mut symbol_indexes = Vec::new();
    for file in files {
        let tree_file = TreeFile::with_language(&file, language)?;
        let index = analyze_tree_file(
            tree_file,
            &analysis_options,
            &analysis_cache,
            &in_process_cache,
            &persistent_cache,
        )?;
        if matches!(output_format, OutputFormat::Json) {
            symbol_indexes.push(index.clone());
        }
        let mut summary = summarize_file(
            &index,
            &command_kind,
            &symbol_filter,
            &scan_filters.symbol_globs,
            &excluded_symbol_globs,
        )?;
        add_external_lint_diagnostics(
            &mut summary,
            &command_kind,
            &pkg_root,
            &analysis_options,
        )?;
        summaries.push(summary);
    }

    match output_format {
        OutputFormat::Json => {
            let package_language = language
                .or_else(|| summaries.first().map(|summary| summary.language))
                .unwrap_or(ProgrammingLanguage::Rust);

            let output = JsonOutput {
                schema_version: SchemaVersion::V2_0,
                root_dir,
                language: package_language,
                files: summaries.clone(),
                symbol_indexes,
            };

            let json =
                serde_json::to_string_pretty(&output).map_err(|source| TreeHuggerError::Io {
                    path: PathBuf::from("<stdout>"),
                    source: std::io::Error::other(source),
                })?;
            println!("{json}");
        }
        OutputFormat::Pretty | OutputFormat::Plain => {
            if matches!(
                command_kind,
                CommandKind::Functions | CommandKind::Types | CommandKind::Symbols
            ) {
                render_symbol_summaries(
                    &summaries,
                    &command_kind,
                    &output_config,
                    display_root.as_deref(),
                    &git_root,
                    render_options,
                );
            } else {
                for summary in &summaries {
                    render_summary(
                        summary,
                        &command_kind,
                        &output_config,
                        display_root.as_deref(),
                    );
                }
            }
        }
    }

    // Report cache stats
    let cache_stats = in_process_cache.stats();
    if cache_stats.parse_hits > 0 || cache_stats.symbol_hits > 0 {
        if output_config.use_colors {
            eprintln!(
                "{} cache: {} parse hits, {} symbol hits, {} entries",
                "▸".dimmed(),
                cache_stats.parse_hits,
                cache_stats.symbol_hits,
                cache_stats.entries
            );
        } else {
            eprintln!(
                "cache: {} parse hits, {} symbol hits, {} entries",
                cache_stats.parse_hits, cache_stats.symbol_hits, cache_stats.entries
            );
        }
    }

    // Exit with non-zero if lint found errors
    if matches!(command_kind, CommandKind::Lint { .. }) {
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
        if error_count > 0 {
            std::process::exit(1);
        }
    }

    Ok(())
}

fn current_dir() -> Result<PathBuf, TreeHuggerError> {
    std::env::current_dir().map_err(|source| TreeHuggerError::Io {
        path: PathBuf::from("."),
        source,
    })
}

/// Directory names skipped when completing filter paths (build/dep output).
const COMPLETION_SKIPPED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    "vendor",
    ".git",
];

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
            let mut suggestion = prefix.join(&raw).into_os_string();
            suggestion.push("/");
            candidates.push(CompletionCandidate::new(suggestion));
        } else if ProgrammingLanguage::from_path(&path).is_some() {
            let suggestion = prefix.join(&raw).into_os_string();
            candidates.push(CompletionCandidate::new(suggestion));
        }
    }
    candidates.sort_by(|a, b| a.get_value().cmp(b.get_value()));
    candidates
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

fn summarize_file(
    index: &FileSymbolIndex,
    command: &CommandKind,
    filter: &SymbolFilter,
    include_symbol_globs: &[String],
    exclude_symbol_globs: &[String],
) -> Result<FileSummary, TreeHuggerError> {
    let all_symbols = symbol_infos_from_records(&index.symbols);
    let imports = import_symbols_from_index(index);
    let exports = exported_symbol_infos(index);
    let locals = local_symbol_infos(index);
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
        CommandKind::Functions => {
            let symbols = match filter {
                SymbolFilter::Exported => exports
                    .clone()
                    .into_iter()
                    .filter(|s| s.kind.is_function())
                    .collect(),
                SymbolFilter::Prelude(filter) => all_symbols
                    .clone()
                    .into_iter()
                    .filter(|s| s.kind.is_function() && filter.names.contains(&s.name))
                    .collect(),
                SymbolFilter::None => all_symbols
                    .clone()
                    .into_iter()
                    .filter(|s| s.kind.is_function())
                    .collect(),
            };
            summary.symbols =
                apply_symbol_filters(symbols, include_symbol_globs, exclude_symbol_globs);
        }
        CommandKind::Types => {
            let symbols = match filter {
                SymbolFilter::Exported => exports
                    .clone()
                    .into_iter()
                    .filter(|s| s.kind.is_type())
                    .collect(),
                SymbolFilter::Prelude(filter) => all_symbols
                    .clone()
                    .into_iter()
                    .filter(|s| s.kind.is_type() && filter.names.contains(&s.name))
                    .collect(),
                SymbolFilter::None => all_symbols
                    .clone()
                    .into_iter()
                    .filter(|s| s.kind.is_type())
                    .collect(),
            };
            summary.symbols =
                apply_symbol_filters(symbols, include_symbol_globs, exclude_symbol_globs);
        }
        CommandKind::Symbols => {
            match filter {
                SymbolFilter::Exported => {
                    summary.symbols = exports.clone();
                    summary.exports = exports.clone();
                }
                SymbolFilter::Prelude(filter) => {
                    if let Some(prelude_exports) = filter.exports_by_file.get(&index.file) {
                        summary.symbols = prelude_exports.clone();
                        summary.exports = summary.symbols.clone();
                        summary.locals.clear();
                    } else {
                        let name_filter = if filter.exports_by_file.is_empty() {
                            &filter.names
                        } else {
                            &filter.env_only_names
                        };
                        summary.symbols = all_symbols
                            .clone()
                            .into_iter()
                            .filter(|s| name_filter.contains(&s.name))
                            .collect();
                        summary.exports = exports
                            .clone()
                            .into_iter()
                            .filter(|s| name_filter.contains(&s.name))
                            .collect();
                        summary.locals = locals
                            .clone()
                            .into_iter()
                            .filter(|s| name_filter.contains(&s.name))
                            .collect();
                    }
                }
                SymbolFilter::None => {
                    summary.symbols = all_symbols.clone();
                    summary.imports = imports.clone();
                    summary.exports = exports.clone();
                    summary.locals = locals.clone();
                }
            }
            summary.symbols =
                apply_symbol_filters(summary.symbols, include_symbol_globs, exclude_symbol_globs);
            summary.exports =
                apply_symbol_filters(summary.exports, include_symbol_globs, exclude_symbol_globs);
            summary.locals =
                apply_symbol_filters(summary.locals, include_symbol_globs, exclude_symbol_globs);
        }
        CommandKind::Imports => {
            summary.imports = imports;
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
        CommandKind::Classes { .. } => {
            // Classes are handled separately in main()
        }
    }

    Ok(summary)
}

fn add_external_lint_diagnostics(
    summary: &mut FileSummary,
    command: &CommandKind,
    project_root: &Path,
    options: &AnalysisOptions,
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

fn analyze_tree_file(
    mut tree_file: TreeFile,
    options: &AnalysisOptions,
    cache: &InMemorySymbolCache,
    in_process: &InProcessCache,
    persistent: &Option<PersistentCache>,
) -> Result<FileSymbolIndex, TreeHuggerError> {
    tree_file.experimental_semantics = options.experimental_semantics;
    let key = FileCacheKey {
        file_path: tree_file.file.clone(),
        language: tree_file.language,
        file_hash: tree_file.hash.clone(),
        analyzer_fingerprint: AnalyzerFingerprint {
            schema_version: SchemaVersion::V2_0,
            tree_hugger_version: env!("CARGO_PKG_VERSION").to_string(),
            grammar_fingerprint: tree_file.language.query_name().to_string(),
            query_fingerprint: "locals+imports+references".to_string(),
            config_fingerprint: options.fingerprint(),
        },
    };
    let stable_key = key.stable_key();

    // Try in-process cache first
    if let Some(index) = in_process.get_symbol_index(&stable_key) {
        return hydrate_diagnostics(&tree_file, index.as_ref().clone());
    }

    // Try persistent cache second
    if let Some(persistent) = persistent
        && let Some(snapshot) = persistent.get::<SymbolSnapshot>(&stable_key)
    {
        let index: FileSymbolIndex = snapshot.into();
        in_process.put_symbol_index(stable_key.clone(), symbol_cache_index(&index));
        return hydrate_diagnostics(&tree_file, index);
    }

    // Try legacy in-memory cache third
    if let Some(snapshot) = cache.get(&key) {
        let index: FileSymbolIndex = snapshot.as_ref().clone().into();
        let cache_index = symbol_cache_index(&index);
        in_process.put_symbol_index(stable_key.clone(), cache_index.clone());
        if let Some(persistent) = persistent {
            let _ = persistent.put(&stable_key, &SymbolSnapshot::from(cache_index));
        }
        return hydrate_diagnostics(&tree_file, index);
    }

    // Miss: compute and store
    let index = tree_file.symbol_index_v2()?;
    let cache_index = symbol_cache_index(&index);
    let snapshot = SymbolSnapshot::from(cache_index.clone());
    cache.put(snapshot.clone());
    in_process.put_symbol_index(stable_key.clone(), cache_index);
    if let Some(persistent) = persistent {
        let _ = persistent.put(&stable_key, &snapshot);
    }
    Ok(index)
}

fn symbol_cache_index(index: &FileSymbolIndex) -> FileSymbolIndex {
    let mut cache_index = index.clone();
    cache_index.diagnostics.clear();
    cache_index
}

fn hydrate_diagnostics(
    tree_file: &TreeFile,
    mut index: FileSymbolIndex,
) -> Result<FileSymbolIndex, TreeHuggerError> {
    index.diagnostics = tree_file.try_diagnostics()?;
    Ok(index)
}

fn symbol_infos_from_records(records: &[tree_hugger::SymbolRecord]) -> Vec<SymbolInfo> {
    records
        .iter()
        .filter_map(|record| SymbolInfo::try_from(record.clone()).ok())
        .collect()
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
        })
        .collect()
}

fn exported_symbol_infos(index: &FileSymbolIndex) -> Vec<SymbolInfo> {
    let exported_ids: HashSet<_> = index
        .exports
        .iter()
        .filter_map(|export| export.symbol.id.as_ref())
        .collect();

    index
        .symbols
        .iter()
        .filter(|symbol| exported_ids.contains(&symbol.id))
        .filter_map(|symbol| SymbolInfo::try_from(symbol.clone()).ok())
        .collect()
}

fn local_symbol_infos(index: &FileSymbolIndex) -> Vec<SymbolInfo> {
    let exported_ids: HashSet<_> = index
        .exports
        .iter()
        .filter_map(|export| export.symbol.id.as_ref())
        .collect();

    index
        .symbols
        .iter()
        .filter(|symbol| !exported_ids.contains(&symbol.id))
        .filter_map(|symbol| SymbolInfo::try_from(symbol.clone()).ok())
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

fn render_symbol_doc_comment(comment: Option<&str>, config: &OutputConfig, indent: &str) {
    if !config.show_comments {
        return;
    }

    let Some(comment) = comment else {
        return;
    };
    let trimmed = comment.trim();
    if trimmed.is_empty() {
        return;
    }

    for line in trimmed.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if config.use_colors {
            println!("{}{}", indent, line.dimmed());
        } else {
            println!("{indent}{line}");
        }
    }
}

fn render_summary(
    summary: &FileSummary,
    command: &CommandKind,
    config: &OutputConfig,
    display_root: Option<&Path>,
) {
    // Render file header with optional hyperlink
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

    match command {
        CommandKind::Imports => render_imports(&summary.imports, config),
        CommandKind::Functions | CommandKind::Types | CommandKind::Symbols => {
            render_symbols(&summary.symbols, config)
        }
        CommandKind::Lint {
            lint_only,
            syntax_only,
            deny,
            warn,
            allow,
            strict,
            experimental_semantics,
        } => {
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
            render_diagnostics_filtered(
                &filtered,
                &summary.syntax,
                &summary.file,
                config,
                *lint_only,
                *syntax_only,
            );
        }
        CommandKind::Classes { .. } => {
            // Classes are rendered separately
        }
    }

    println!();
}

fn render_symbol_summaries(
    summaries: &[FileSummary],
    command: &CommandKind,
    config: &OutputConfig,
    display_root: Option<&Path>,
    git_root: &Path,
    options: SymbolRenderOptions,
) {
    let packages = summarize_packages(summaries, git_root);
    let packages_with_symbols = packages
        .iter()
        .filter(|(_, package_summaries)| {
            package_summaries
                .iter()
                .any(|summary| !summary.symbols.is_empty())
        })
        .count();

    if packages.len() <= 1 || packages_with_symbols <= 1 {
        let package_summaries = summaries.iter().collect::<Vec<_>>();
        render_symbol_summaries_for_package(
            &package_summaries,
            command,
            config,
            display_root,
            options,
        );
        return;
    }

    for (package_root, package_summaries) in packages {
        render_package_heading(&package_root, config, display_root);
        render_symbol_summaries_for_package(
            &package_summaries,
            command,
            config,
            display_root,
            options,
        );
        println!();
    }
}

fn render_symbol_summaries_for_package(
    summaries: &[&FileSummary],
    command: &CommandKind,
    config: &OutputConfig,
    display_root: Option<&Path>,
    options: SymbolRenderOptions,
) {
    let mut symbols: Vec<SymbolInfo> = summaries
        .iter()
        .flat_map(|summary| summary.symbols.iter().cloned())
        .collect();

    if symbols.is_empty() {
        if config.use_colors {
            println!("{}", "(no symbols)".dimmed());
        } else {
            println!("(no symbols)");
        }
        return;
    }

    sort_symbols(&mut symbols, options, display_root);

    if options.group_by_module && options.group_by_file {
        render_symbols_grouped_by_module_and_file(&symbols, config, display_root);
    } else if options.group_by_module {
        render_symbols_grouped_by_module(&symbols, config, display_root);
    } else if options.group_by_file {
        render_symbols_grouped_by_file(&symbols, config, display_root);
    } else {
        render_flat_symbol_list(&symbols, command, config, display_root);
    }
}

fn summarize_packages<'a>(
    summaries: &'a [FileSummary],
    git_root: &Path,
) -> Vec<(PathBuf, Vec<&'a FileSummary>)> {
    let mut grouped: BTreeMap<PathBuf, Vec<&FileSummary>> = BTreeMap::new();

    for summary in summaries {
        let start = summary.file.parent().unwrap_or(summary.file.as_path());
        let package_root = find_package_root(start, git_root);
        grouped.entry(package_root).or_default().push(summary);
    }

    grouped.into_iter().collect()
}

fn render_package_heading(package_root: &Path, config: &OutputConfig, display_root: Option<&Path>) {
    let package_display = display_path(package_root, display_root);
    println!();
    if config.use_colors {
        println!("{}", package_display.blue().bold());
    } else {
        println!("{package_display}");
    }
    println!();
}

fn sort_symbols(
    symbols: &mut [SymbolInfo],
    options: SymbolRenderOptions,
    display_root: Option<&Path>,
) {
    symbols.sort_by(|left, right| {
        let left_name = left.name.to_ascii_lowercase();
        let right_name = right.name.to_ascii_lowercase();

        if options.sort_by_module {
            let left_module = module_key_for_symbol(left, display_root);
            let right_module = module_key_for_symbol(right, display_root);
            let module_cmp = left_module.cmp(&right_module);
            if !module_cmp.is_eq() {
                return module_cmp;
            }
        }

        if options.sort_by_kind {
            let kind_cmp = left.kind.to_string().cmp(&right.kind.to_string());
            if !kind_cmp.is_eq() {
                return kind_cmp;
            }
        }

        let name_cmp = left_name.cmp(&right_name);
        if !name_cmp.is_eq() {
            return name_cmp;
        }

        left.range
            .start_line
            .cmp(&right.range.start_line)
            .then_with(|| left.range.start_column.cmp(&right.range.start_column))
    });
}

fn module_key_for_symbol(symbol: &SymbolInfo, display_root: Option<&Path>) -> String {
    let display = display_path(&symbol.file, display_root);
    Path::new(&display)
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_default()
}

fn render_flat_symbol_list(
    symbols: &[SymbolInfo],
    _command: &CommandKind,
    config: &OutputConfig,
    display_root: Option<&Path>,
) {
    for symbol in symbols {
        render_symbol_line(symbol, config, display_root, true);
    }
}

fn render_symbols_grouped_by_file(
    symbols: &[SymbolInfo],
    config: &OutputConfig,
    display_root: Option<&Path>,
) {
    let mut grouped: BTreeMap<String, Vec<SymbolInfo>> = BTreeMap::new();
    for symbol in symbols {
        grouped
            .entry(display_path(&symbol.file, display_root))
            .or_default()
            .push(symbol.clone());
    }

    for (file, items) in grouped {
        if config.use_colors {
            println!("{}", file.bold());
        } else {
            println!("{file}");
        }
        for symbol in items {
            render_symbol_line(&symbol, config, display_root, false);
        }
        println!();
    }
}

fn render_symbols_grouped_by_module(
    symbols: &[SymbolInfo],
    config: &OutputConfig,
    display_root: Option<&Path>,
) {
    let mut grouped: BTreeMap<String, Vec<SymbolInfo>> = BTreeMap::new();
    for symbol in symbols {
        grouped
            .entry(module_key_for_symbol(symbol, display_root))
            .or_default()
            .push(symbol.clone());
    }

    for (module, items) in grouped {
        let title = if module.is_empty() {
            "(root)"
        } else {
            module.as_str()
        };
        if config.use_colors {
            println!("{}", title.bold());
        } else {
            println!("{title}");
        }
        for symbol in items {
            render_symbol_line(&symbol, config, display_root, true);
        }
        println!();
    }
}

fn render_symbols_grouped_by_module_and_file(
    symbols: &[SymbolInfo],
    config: &OutputConfig,
    display_root: Option<&Path>,
) {
    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<SymbolInfo>>> = BTreeMap::new();
    for symbol in symbols {
        grouped
            .entry(module_key_for_symbol(symbol, display_root))
            .or_default()
            .entry(display_path(&symbol.file, display_root))
            .or_default()
            .push(symbol.clone());
    }

    for (module, by_file) in grouped {
        let title = if module.is_empty() {
            "(root)"
        } else {
            module.as_str()
        };
        if config.use_colors {
            println!("{}", title.bold());
        } else {
            println!("{title}");
        }

        for (file, items) in by_file {
            if config.use_colors {
                println!("  {}", file.underline());
            } else {
                println!("  {file}");
            }
            for symbol in items {
                render_symbol_line(&symbol, config, display_root, false);
            }
        }

        println!();
    }
}

fn render_symbol_line(
    symbol: &SymbolInfo,
    config: &OutputConfig,
    display_root: Option<&Path>,
    include_file: bool,
) {
    let location = if include_file {
        let path = display_path(&symbol.file, display_root);
        format!(
            "{}:{}:{}",
            path, symbol.range.start_line, symbol.range.start_column
        )
    } else {
        format!(
            "[{}:{}]",
            symbol.range.start_line, symbol.range.start_column
        )
    };

    let location_display = if config.use_hyperlinks {
        hyperlink(&symbol.file, symbol.range.start_line, &location)
    } else {
        location
    };

    let name_with_sig = format_symbol_name(symbol, symbol.language);
    let presentation = symbol_presentation(symbol);
    let visibility = symbol
        .signature
        .as_ref()
        .and_then(|signature| signature.visibility.as_ref());

    if config.use_colors {
        let kind_style = style_for_kind(presentation.style_kind);
        let kind_part = match visibility {
            Some(visibility) => format!(
                "{} {}",
                visibility.to_string().italic(),
                presentation.label.style(kind_style)
            ),
            None => presentation.label.style(kind_style).to_string(),
        };
        println!(
            "  - {} {} {}",
            kind_part,
            name_with_sig.bold(),
            location_display.dimmed()
        );
    } else {
        let kind_part = match visibility {
            Some(visibility) => format!("{} {}", visibility, presentation.label),
            None => presentation.label,
        };
        println!("  - {} {} {}", kind_part, name_with_sig, location_display);
    }

    render_symbol_doc_comment(symbol.doc_comment.as_deref(), config, "    ");
}

fn render_symbols(symbols: &[SymbolInfo], config: &OutputConfig) {
    if symbols.is_empty() {
        if config.use_colors {
            println!("  {}", "(no symbols)".dimmed());
        } else {
            println!("  (no symbols)");
        }
        return;
    }

    for symbol in symbols {
        let location = format!(
            "[{}:{}]",
            symbol.range.start_line, symbol.range.start_column
        );
        let location_display = if config.use_hyperlinks {
            hyperlink(&symbol.file, symbol.range.start_line, &location)
        } else {
            location
        };

        // Format symbol name with signature for functions/methods
        let name_with_sig = format_symbol_name(symbol, symbol.language);
        let presentation = symbol_presentation(symbol);

        // Extract visibility for functions/methods
        let visibility = symbol
            .signature
            .as_ref()
            .and_then(|sig| sig.visibility.as_ref());

        if config.use_colors {
            let kind_style = style_for_kind(presentation.style_kind);

            // Format visibility (italicized) + kind + name
            let kind_part = match visibility {
                Some(vis) => format!(
                    "{} {}",
                    vis.to_string().italic(),
                    presentation.label.style(kind_style)
                ),
                None => presentation.label.style(kind_style).to_string(),
            };

            println!(
                "  - {} {} {}",
                kind_part,
                name_with_sig.bold(),
                location_display.dimmed()
            );
        } else {
            // Plain text: visibility + kind + name
            let kind_part = match visibility {
                Some(vis) => format!("{} {}", vis, presentation.label),
                None => presentation.label,
            };
            println!("  - {} {} {}", kind_part, name_with_sig, location_display);
        }

        render_symbol_doc_comment(symbol.doc_comment.as_deref(), config, "    ");
    }
}

/// Formats a symbol name with its signature/metadata.
///
/// For functions/methods: `name(param1: T1, param2: T2) -> ReturnType`
/// For types: `name { field1: T1, field2: T2 }` or `name { Variant1, Variant2 }`
/// For other symbols: just the name
fn format_symbol_name(symbol: &SymbolInfo, language: ProgrammingLanguage) -> String {
    let base_name = if symbol.kind.is_function() {
        match &symbol.signature {
            Some(sig) => format_function_signature(&symbol.name, sig, language),
            None => symbol.name.clone(),
        }
    } else if symbol.kind.is_type() {
        match &symbol.type_metadata {
            Some(meta) => format_type_signature(&symbol.name, meta, language),
            None => symbol.name.clone(),
        }
    } else {
        symbol.name.clone()
    };

    if let Some(container_name) = symbol.container_name.as_deref()
        && matches!(
            symbol.kind,
            SymbolKind::Method | SymbolKind::Function | SymbolKind::Field | SymbolKind::Parameter
        )
    {
        let container_label = match symbol.container_kind {
            Some(kind)
                if kind == SymbolKind::Type && container_name.trim_start().starts_with("impl ") =>
            {
                container_name.to_string()
            }
            Some(kind) => format!("{kind} {container_name}"),
            None => container_name.to_string(),
        };
        return format!("{base_name} [in {container_label}]");
    }

    base_name
}

/// Formats a function signature like: `name(param1: T1, param2: T2) -> ReturnType`
///
/// Note: Visibility is displayed separately before the kind (e.g., "public method").
fn format_function_signature(
    name: &str,
    sig: &FunctionSignature,
    language: ProgrammingLanguage,
) -> String {
    let params = format_parameters(&sig.parameters, language);
    let return_part = format_return_type(&sig.return_type, language);

    format!("{name}({params}){return_part}")
}

/// Formats function parameters as a comma-separated list.
fn format_parameters(params: &[ParameterInfo], language: ProgrammingLanguage) -> String {
    params
        .iter()
        .map(|p| format_parameter(p, language))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats a single parameter.
fn format_parameter(param: &ParameterInfo, language: ProgrammingLanguage) -> String {
    let mut result = String::new();

    // Variadic prefix
    if param.is_variadic {
        match language {
            ProgrammingLanguage::Python => result.push('*'),
            ProgrammingLanguage::Go => result.push_str("..."),
            ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
                result.push_str("...");
            }
            _ => {}
        }
    }

    result.push_str(&param.name);

    // Type annotation
    if let Some(ty) = &param.type_annotation {
        match language {
            ProgrammingLanguage::Go => {
                // Go: `name type`
                result.push(' ');
                result.push_str(ty);
            }
            _ => {
                // Most languages: `name: type`
                result.push_str(": ");
                result.push_str(ty);
            }
        }
    }

    // Default value
    if let Some(default) = &param.default_value {
        result.push_str(" = ");
        result.push_str(default);
    }

    result
}

/// Formats the return type with appropriate syntax.
fn format_return_type(return_type: &Option<String>, language: ProgrammingLanguage) -> String {
    match return_type {
        Some(ty) => {
            match language {
                ProgrammingLanguage::Go => {
                    // Go: ` type` (space before type, no arrow)
                    format!(" {ty}")
                }
                ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
                    // TypeScript: `: type`
                    format!(": {ty}")
                }
                _ => {
                    // Rust, Python: ` -> type`
                    format!(" -> {ty}")
                }
            }
        }
        None => String::new(),
    }
}

/// Formats a type signature showing its composition.
///
/// For structs: `name { field1: T1, field2: T2 }`
/// For enums: `name { Variant1, Variant2(T), Variant3 { f: T } }`
fn format_type_signature(name: &str, meta: &TypeMetadata, language: ProgrammingLanguage) -> String {
    let mut result = name.to_string();

    // Add generic type parameters if present
    if !meta.type_parameters.is_empty() {
        result.push('<');
        result.push_str(&meta.type_parameters.join(", "));
        result.push('>');
    }

    // Format based on whether it has fields (struct-like) or variants (enum-like)
    if !meta.variants.is_empty() {
        let variants = format_variants(&meta.variants);
        result.push_str(&format!(" {{ {variants} }}"));
    } else if !meta.fields.is_empty() {
        let fields = format_fields(&meta.fields, language);
        result.push_str(&format!(" {{ {fields} }}"));
    }

    result
}

/// Formats struct fields for display.
fn format_fields(fields: &[FieldInfo], language: ProgrammingLanguage) -> String {
    fields
        .iter()
        .map(|f| format_field(f, language))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats a single field.
fn format_field(field: &FieldInfo, language: ProgrammingLanguage) -> String {
    match &field.type_annotation {
        Some(ty) => match language {
            ProgrammingLanguage::Go => format!("{} {}", field.name, ty),
            _ => format!("{}: {}", field.name, ty),
        },
        None => field.name.clone(),
    }
}

/// Formats enum variants for display.
fn format_variants(variants: &[VariantInfo]) -> String {
    variants
        .iter()
        .map(format_variant)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Formats a single enum variant.
fn format_variant(variant: &VariantInfo) -> String {
    if !variant.tuple_fields.is_empty() {
        // Tuple variant: Variant(T1, T2)
        format!("{}({})", variant.name, variant.tuple_fields.join(", "))
    } else if !variant.struct_fields.is_empty() {
        // Struct variant: Variant { field: T }
        let fields: Vec<String> = variant
            .struct_fields
            .iter()
            .map(|f| match &f.type_annotation {
                Some(ty) => format!("{}: {}", f.name, ty),
                None => f.name.clone(),
            })
            .collect();
        format!("{} {{ {} }}", variant.name, fields.join(", "))
    } else {
        // Unit variant
        variant.name.clone()
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

    lint
        .iter()
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
        file.display(),
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

/// Extracts class summaries from a file.
fn extract_class_summaries(
    index: &FileSymbolIndex,
    name_filter: Option<&str>,
    static_only: bool,
    instance_only: bool,
) -> Result<Vec<ClassSummary>, TreeHuggerError> {
    let symbols_by_id: HashMap<_, _> = index
        .symbols
        .iter()
        .map(|symbol| (&symbol.id, symbol))
        .collect();

    let mut result = Vec::new();

    for class_record in index.symbols.iter().filter(|symbol| {
        matches!(
            symbol.kind,
            tree_hugger::SymbolKindV2::Class | tree_hugger::SymbolKindV2::Type
        )
    }) {
        let Ok(class) = SymbolInfo::try_from(class_record.clone()) else {
            continue;
        };
        if let Some(filter) = name_filter
            && !class.name.contains(filter)
        {
            continue;
        }

        let mut static_methods = Vec::new();
        let mut instance_methods = Vec::new();

        for member in &class_record.relations.members {
            let Some(member_id) = member.id.as_ref() else {
                continue;
            };
            let Some(member_record) = symbols_by_id.get(member_id) else {
                continue;
            };
            if !matches!(
                member_record.kind,
                tree_hugger::SymbolKindV2::Method | tree_hugger::SymbolKindV2::Constructor
            ) {
                continue;
            }

            let Ok(method) = SymbolInfo::try_from((*member_record).clone()) else {
                continue;
            };
            let is_static = method
                .signature
                .as_ref()
                .map(|s| s.is_static)
                .unwrap_or(false);
            if is_static {
                static_methods.push(method);
            } else {
                instance_methods.push(method);
            }
        }

        // Get fields from type metadata
        let mut static_fields = Vec::new();
        let mut instance_fields = Vec::new();

        if let Some(meta) = &class.type_metadata {
            for field in &meta.fields {
                if field.is_static {
                    static_fields.push(field.clone());
                } else {
                    instance_fields.push(field.clone());
                }
            }
        }

        // Apply filters
        if static_only {
            instance_methods.clear();
            instance_fields.clear();
        }
        if instance_only {
            static_methods.clear();
            static_fields.clear();
        }

        result.push(ClassSummary {
            class,
            static_methods,
            instance_methods,
            static_fields,
            instance_fields,
        });
    }

    Ok(result)
}

/// Renders class summaries for a file.
fn render_class_summaries(
    file: &Path,
    language: ProgrammingLanguage,
    summaries: &[ClassSummary],
    config: &OutputConfig,
    display_root: Option<&Path>,
) {
    // Render file header
    let file_display = display_path(file, display_root);
    let header = if config.use_hyperlinks {
        hyperlink(file, 1, &file_display)
    } else {
        file_display
    };

    if config.use_colors {
        println!("{} ({})", header.bold(), language.to_string().dimmed());
    } else {
        println!("{} ({})", header, language);
    }

    for summary in summaries {
        render_class_summary(summary, language, config);
    }

    println!();
}

/// Renders a single class summary.
fn render_class_summary(
    summary: &ClassSummary,
    language: ProgrammingLanguage,
    config: &OutputConfig,
) {
    let class = &summary.class;
    let location = format!("[{}:{}]", class.range.start_line, class.range.start_column);
    let location_display = if config.use_hyperlinks {
        hyperlink(&class.file, class.range.start_line, &location)
    } else {
        location
    };

    if config.use_colors {
        println!(
            "  {} {} {}",
            class.kind.to_string().magenta(),
            class.name.bold(),
            location_display.dimmed()
        );
    } else {
        println!("  {} {} {}", class.kind, class.name, location_display);
    }

    render_symbol_doc_comment(class.doc_comment.as_deref(), config, "    ");

    // Render static methods
    if !summary.static_methods.is_empty() {
        render_member_section(
            "Static Methods",
            &summary.static_methods,
            language,
            config,
            true,
        );
    }

    // Render instance methods
    if !summary.instance_methods.is_empty() {
        render_member_section(
            "Instance Methods",
            &summary.instance_methods,
            language,
            config,
            true,
        );
    }

    // Render static fields
    if !summary.static_fields.is_empty() {
        render_field_section("Static Fields", &summary.static_fields, language, config);
    }

    // Render instance fields
    if !summary.instance_fields.is_empty() {
        render_field_section(
            "Instance Fields",
            &summary.instance_fields,
            language,
            config,
        );
    }
}

/// Renders a section of methods.
fn render_member_section(
    title: &str,
    methods: &[SymbolInfo],
    language: ProgrammingLanguage,
    config: &OutputConfig,
    _is_method: bool,
) {
    if config.use_colors {
        println!("    {} ({})", title.yellow(), methods.len());
    } else {
        println!("    {} ({})", title, methods.len());
    }

    for method in methods {
        let location = format!(
            "[{}:{}]",
            method.range.start_line, method.range.start_column
        );
        let location_display = if config.use_hyperlinks {
            hyperlink(&method.file, method.range.start_line, &location)
        } else {
            location
        };

        let name_with_sig = format_symbol_name(method, language);

        // Get visibility
        let visibility = method
            .signature
            .as_ref()
            .and_then(|sig| sig.visibility.as_ref());

        if config.use_colors {
            let vis_str = match visibility {
                Some(vis) => format!("{} ", vis.to_string().italic()),
                None => String::new(),
            };
            println!(
                "      - {}{} {}",
                vis_str,
                name_with_sig.green(),
                location_display.dimmed()
            );
        } else {
            let vis_str = match visibility {
                Some(vis) => format!("{} ", vis),
                None => String::new(),
            };
            println!("      - {}{} {}", vis_str, name_with_sig, location_display);
        }

        render_symbol_doc_comment(method.doc_comment.as_deref(), config, "        ");
    }
}

/// Renders a section of fields.
fn render_field_section(
    title: &str,
    fields: &[FieldInfo],
    language: ProgrammingLanguage,
    config: &OutputConfig,
) {
    if config.use_colors {
        println!("    {} ({})", title.yellow(), fields.len());
    } else {
        println!("    {} ({})", title, fields.len());
    }

    for field in fields {
        let field_display = format_field(field, language);

        // Get visibility
        let visibility = field.visibility.as_ref();

        if config.use_colors {
            let vis_str = match visibility {
                Some(vis) => format!("{} ", vis.to_string().italic()),
                None => String::new(),
            };
            println!("      - {}{}", vis_str, field_display.cyan());
        } else {
            let vis_str = match visibility {
                Some(vis) => format!("{} ", vis),
                None => String::new(),
            };
            println!("      - {}{}", vis_str, field_display);
        }

        render_symbol_doc_comment(field.doc_comment.as_deref(), config, "        ");
    }
}
