use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Generator, Shell};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use owo_colors::{OwoColorize, Style};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use tree_hugger::{
    CodeRange, Diagnostic, DiagnosticKind, DiagnosticSeverity, FieldInfo, FileSummary,
    FunctionSignature,
    ImportSymbol, LintDiagnostic, ParameterInfo, ProgrammingLanguage, SchemaVersion, SourceContext,
    SymbolInfo, SymbolKind, SyntaxDiagnostic, TreeFile, TreeHuggerError, TypeMetadata, VariantInfo,
    find_git_root, find_package_root,
};

#[derive(Parser, Debug)]
#[command(
    name = "hug",
    version,
    about = "Tree Hugger diagnostics and symbol tooling"
)]
struct Cli {
    /// Glob patterns for files to exclude from scanning
    #[arg(long, value_name = "GLOB", global = true, display_order = 10)]
    exclude_files: Vec<String>,

    /// Glob patterns for symbol names to exclude from output
    #[arg(long, value_name = "GLOB", global = true, display_order = 11)]
    exclude_symbols: Vec<String>,

    /// Force a specific language
    #[arg(long, value_enum, global = true)]
    language: Option<LanguageArg>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Disable colors and hyperlinks (plain text output)
    #[arg(long, global = true)]
    plain: bool,

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
    #[arg(value_name = "FILTER", num_args = 0..)]
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
    #[arg(value_name = "FILTER", num_args = 0..)]
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
    #[arg(value_name = "FILTER", num_args = 0..)]
    filters: Vec<String>,

    /// Show only lint diagnostics (pattern-based and semantic rules)
    #[arg(long, conflicts_with = "syntax_only")]
    lint_only: bool,

    /// Show only syntax diagnostics (parse errors)
    #[arg(long, conflicts_with = "lint_only")]
    syntax_only: bool,
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
    #[command(after_help = "\
Examples:
  # Bash (add to ~/.bashrc)
  hug completions bash >> ~/.bashrc

  # Zsh (add to ~/.zshrc, ensure fpath includes the directory)
  hug completions zsh > ~/.zfunc/_hug

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

/// The kind of command being executed (without the arguments).
#[derive(Debug, Clone)]
enum CommandKind {
    Functions,
    Types,
    Symbols,
    Imports,
    Lint {
        lint_only: bool,
        syntax_only: bool,
    },
    Classes {
        name_filter: Option<String>,
        static_only: bool,
        instance_only: bool,
    },
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

/// Resolved prelude export filter data.
#[derive(Debug, Clone)]
struct PreludeFilter {
    /// Symbol names explicitly listed by prelude exports (plus PRELUDE env var).
    names: HashSet<String>,
    /// Resolved prelude export symbols keyed by prelude file path.
    exports_by_file: HashMap<PathBuf, Vec<SymbolInfo>>,
}

#[derive(Debug, Clone)]
struct ResolvedPreludeMetadata {
    kind: SymbolKind,
    doc_comment: Option<String>,
    file: Option<PathBuf>,
    range: Option<CodeRange>,
}

/// Classification of positional filter tokens.
#[derive(Debug, Clone, Default)]
struct ScanFilters {
    file_filters: Vec<String>,
    symbol_globs: Vec<String>,
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
    let cli = Cli::parse();

    // Handle completions command early (doesn't need file processing)
    if let Command::Completions(args) = &cli.command {
        print_completions(args.shell, &mut Cli::command());
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
    let scan_filters = classify_filters(filters, &command_kind);
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
            let mut class_summaries = extract_class_summaries(
                &tree_file,
                name_filter.as_deref(),
                *static_only,
                *instance_only,
            )?;

            // Apply symbol filter to class summaries
            match &symbol_filter {
                SymbolFilter::Exported => {
                    let exported: HashSet<String> = tree_file
                        .exported_symbols()?
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
                all_class_summaries.push((
                    tree_file.file.clone(),
                    tree_file.language,
                    class_summaries,
                ));
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
        if matches!(output_format, OutputFormat::Json) {
            symbol_indexes.push(tree_file.symbol_index_v2()?);
        }
        let summary = summarize_file(
            &tree_file,
            &command_kind,
            &symbol_filter,
            &scan_filters.symbol_globs,
            &excluded_symbol_globs,
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
                files: summaries,
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
                    render_options,
                );
            } else {
                for summary in summaries {
                    render_summary(
                        &summary,
                        &command_kind,
                        &output_config,
                        display_root.as_deref(),
                    );
                }
            }
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

/// Prints shell completions to stdout.
fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    clap_complete::generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut std::io::stdout(),
    );
}

/// Resolves symbol names and direct export entries from a prelude module.
///
/// Looks for `{root}/src/prelude.rs` or `{root}/src/prelude/mod.rs`, parses
/// public `use` exports, and merges any additional names from the `PRELUDE`
/// environment variable (comma-separated).
fn resolve_prelude_symbols(root_dir: &Path) -> Result<PreludeFilter, TreeHuggerError> {
    let mut names = HashSet::new();
    let mut exports_by_file: HashMap<PathBuf, Vec<SymbolInfo>> = HashMap::new();
    let mut rust_symbol_cache: HashMap<PathBuf, Vec<SymbolInfo>> = HashMap::new();

    let candidates = [
        root_dir.join("src/prelude.rs"),
        root_dir.join("src/prelude/mod.rs"),
    ];
    for candidate in &candidates {
        if candidate.is_file() {
            let tree_file = TreeFile::with_language(candidate, None)?;
            let source = std::fs::read_to_string(candidate).map_err(|source| TreeHuggerError::Io {
                path: candidate.to_path_buf(),
                source,
            })?;
            let exports: Vec<ImportSymbol> = tree_file
                .imported_symbols()?
                .into_iter()
                .filter(|import| is_prelude_export(import, &source))
                .collect();
            let mut resolved_exports = Vec::with_capacity(exports.len());

            for import in &exports {
                names.insert(import.name.clone());
                let metadata =
                    resolve_prelude_export_metadata(import, root_dir, &mut rust_symbol_cache)?
                        .unwrap_or(ResolvedPreludeMetadata {
                            kind: SymbolKind::Unknown,
                            doc_comment: None,
                            file: None,
                            range: None,
                        });
                resolved_exports.push(symbol_from_prelude_export(
                    import,
                    metadata.kind,
                    metadata.doc_comment,
                    metadata.file,
                    metadata.range,
                ));
            }

            exports_by_file.insert(candidate.to_path_buf(), resolved_exports);
            break;
        }
    }

    if let Ok(env_val) = std::env::var("PRELUDE") {
        for name in env_val.split(',') {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                names.insert(trimmed.to_string());
            }
        }
    }

    Ok(PreludeFilter {
        names,
        exports_by_file,
    })
}

fn is_prelude_export(import: &ImportSymbol, file_source: &str) -> bool {
    if import.language != ProgrammingLanguage::Rust {
        return true;
    }

    let Some(statement_range) = import.statement_range.as_ref() else {
        return true;
    };
    let Some(statement) = file_source.get(statement_range.start_byte..statement_range.end_byte)
    else {
        return false;
    };

    is_public_rust_use_statement(statement)
}

fn is_public_rust_use_statement(statement: &str) -> bool {
    let trimmed = statement.trim_start();
    let Some(after_pub) = trimmed.strip_prefix("pub") else {
        return false;
    };
    let after_pub = after_pub.trim_start();
    if after_pub.starts_with("use ") {
        return true;
    }

    if let Some(after_scope) = after_pub.strip_prefix('(')
        && let Some(close_idx) = after_scope.find(')')
    {
        return after_scope[close_idx + 1..].trim_start().starts_with("use ");
    }

    false
}

fn resolve_prelude_export_metadata(
    import: &ImportSymbol,
    root_dir: &Path,
    rust_symbol_cache: &mut HashMap<PathBuf, Vec<SymbolInfo>>,
) -> Result<Option<ResolvedPreludeMetadata>, TreeHuggerError> {
    if import.language != ProgrammingLanguage::Rust {
        return Ok(None);
    }

    let Some(source) = import.source.as_deref() else {
        return Ok(None);
    };
    let target_name = import_target_symbol_name(import);

    for candidate in rust_module_source_candidates(root_dir, source) {
        if !candidate.is_file() {
            continue;
        }

        if !rust_symbol_cache.contains_key(&candidate) {
            let tree_file = TreeFile::with_language(&candidate, Some(ProgrammingLanguage::Rust))?;
            let mut symbols = tree_file.exported_symbols()?;
            if symbols.is_empty() {
                symbols = tree_file.symbols()?;
            }
            rust_symbol_cache.insert(candidate.clone(), symbols);
        }

        if let Some(symbol) = rust_symbol_cache
            .get(&candidate)
            .and_then(|symbols| symbols.iter().find(|symbol| symbol.name == target_name))
        {
            return Ok(Some(ResolvedPreludeMetadata {
                kind: symbol.kind,
                doc_comment: symbol.doc_comment.clone(),
                file: Some(symbol.file.clone()),
                range: Some(symbol.range.clone()),
            }));
        }

        // Fallback for symbols not captured by tree-sitter symbol queries (e.g., some type aliases).
        let source_text = std::fs::read_to_string(&candidate).map_err(|source| TreeHuggerError::Io {
            path: candidate.clone(),
            source,
        })?;
        if let Some(kind) = infer_rust_decl_kind(&source_text, target_name) {
            return Ok(Some(ResolvedPreludeMetadata {
                kind,
                doc_comment: None,
                file: None,
                range: None,
            }));
        }
    }

    Ok(None)
}

fn import_target_symbol_name(import: &ImportSymbol) -> &str {
    let raw = import.original_name.as_deref().unwrap_or(import.name.as_str());
    raw.rsplit("::").next().unwrap_or(raw)
}

fn infer_rust_decl_kind(source: &str, target_name: &str) -> Option<SymbolKind> {
    let patterns = [
        (SymbolKind::Trait, format!("pub trait {target_name}")),
        (SymbolKind::Enum, format!("pub enum {target_name}")),
        (SymbolKind::Type, format!("pub struct {target_name}")),
        (SymbolKind::Type, format!("pub type {target_name}")),
        (SymbolKind::Function, format!("pub fn {target_name}")),
        (SymbolKind::Constant, format!("pub const {target_name}")),
        (SymbolKind::Constant, format!("pub static {target_name}")),
    ];

    patterns
        .iter()
        .find_map(|(kind, marker)| source.contains(marker).then_some(*kind))
}

fn rust_module_source_candidates(root_dir: &Path, source: &str) -> Vec<PathBuf> {
    let normalized = source
        .trim()
        .trim_start_matches("crate::")
        .trim_start_matches("self::")
        .trim_start_matches("::")
        .trim_end_matches("::");

    if normalized.is_empty() || normalized.starts_with("super::") {
        return Vec::new();
    }

    let module = normalized.replace("::", "/");
    vec![
        root_dir.join("src").join(format!("{module}.rs")),
        root_dir.join("src").join(module).join("mod.rs"),
    ]
}

fn classify_filters(filters: &[String], command: &CommandKind) -> ScanFilters {
    match command {
        CommandKind::Functions
        | CommandKind::Types
        | CommandKind::Symbols
        | CommandKind::Classes { .. } => {
            let mut file_filters = Vec::new();
            let mut symbol_globs = Vec::new();
            for filter in filters {
                if is_file_filter_token(filter) {
                    file_filters.push(filter.clone());
                } else {
                    symbol_globs.push(normalize_symbol_glob(filter));
                }
            }
            ScanFilters {
                file_filters,
                symbol_globs,
            }
        }
        CommandKind::Imports | CommandKind::Lint { .. } => ScanFilters {
            file_filters: filters.to_vec(),
            symbol_globs: Vec::new(),
        },
    }
}

fn is_file_filter_token(token: &str) -> bool {
    if token.contains('/') || token.contains('\\') {
        return true;
    }

    let extension = Path::new(token).extension().and_then(|ext| ext.to_str());
    extension
        .and_then(ProgrammingLanguage::from_extension)
        .is_some()
}

fn normalize_symbol_glob(token: &str) -> String {
    if let Some(strict_name) = token.strip_suffix('!')
        && !strict_name.is_empty()
    {
        // Trailing `!` switches from fuzzy auto-wrapped matching to strict exact matching.
        // Example: `parse_width_spec!` matches only `parse_width_spec`.
        return strict_name.to_string();
    }

    if token.contains('*') {
        token.to_string()
    } else {
        format!("*{token}*")
    }
}

fn normalize_excluded_symbol_glob(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }

    if let Some(strict_name) = token.strip_suffix('!')
        && !strict_name.is_empty()
    {
        // Keep parity with positional filters: trailing `!` means strict name.
        return Some(strict_name.to_string());
    }

    Some(token.to_string())
}

fn matches_symbol_filters(name: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return true;
    }

    patterns
        .iter()
        .any(|pattern| wildcard_match(pattern.as_str(), name))
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let starts_with_wildcard = pattern.starts_with('*');
    let ends_with_wildcard = pattern.ends_with('*');
    let segments: Vec<&str> = pattern
        .split('*')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return true;
    }

    let mut index = 0usize;
    for (position, segment) in segments.iter().enumerate() {
        if position == 0 && !starts_with_wildcard {
            if !value[index..].starts_with(segment) {
                return false;
            }
            index += segment.len();
            continue;
        }

        match value[index..].find(segment) {
            Some(found) => {
                index += found + segment.len();
            }
            None => return false,
        }
    }

    if ends_with_wildcard {
        true
    } else {
        value.ends_with(segments.last().unwrap_or(&""))
    }
}

fn apply_symbol_filters(
    symbols: Vec<SymbolInfo>,
    include_symbol_globs: &[String],
    exclude_symbol_globs: &[String],
) -> Vec<SymbolInfo> {
    if include_symbol_globs.is_empty() && exclude_symbol_globs.is_empty() {
        return symbols;
    }

    symbols
        .into_iter()
        .filter(|symbol| {
            let included = include_symbol_globs.is_empty()
                || matches_symbol_filters(&symbol.name, include_symbol_globs);
            let excluded = !exclude_symbol_globs.is_empty()
                && matches_symbol_filters(&symbol.name, exclude_symbol_globs);
            included && !excluded
        })
        .collect()
}

fn collect_files(
    root: &Path,
    inputs: &[String],
    excluded_files: &[String],
    language: Option<ProgrammingLanguage>,
) -> Result<Vec<PathBuf>, TreeHuggerError> {
    let mut overrides = OverrideBuilder::new(root);
    for input in inputs {
        overrides.add(input)?;
    }
    for excluded_file in excluded_files {
        overrides.add(&format!("!{}", excluded_file))?;
    }

    let overrides = overrides.build()?;
    let mut files = Vec::new();

    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .overrides(overrides)
        .build();

    for entry in walker {
        let entry = entry.map_err(TreeHuggerError::Ignore)?;

        let is_file = entry
            .file_type()
            .map(|file| file.is_file())
            .unwrap_or(false);

        if !is_file {
            continue;
        }

        match language {
            Some(lang) if ProgrammingLanguage::from_path(entry.path()) != Some(lang) => continue,
            None if ProgrammingLanguage::from_path(entry.path()).is_none() => continue,
            _ => {}
        }

        files.push(entry.into_path());
    }

    files.sort();

    if files.is_empty() {
        return Err(TreeHuggerError::NoSourceFiles {
            path: root.to_path_buf(),
        });
    }

    Ok(files)
}

fn summarize_file(
    tree_file: &TreeFile,
    command: &CommandKind,
    filter: &SymbolFilter,
    include_symbol_globs: &[String],
    exclude_symbol_globs: &[String],
) -> Result<FileSummary, TreeHuggerError> {
    let mut summary = FileSummary {
        file: tree_file.file.clone(),
        language: tree_file.language,
        hash: tree_file.hash.clone(),
        symbols: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        locals: Vec::new(),
        lint: tree_file.lint_diagnostics(),
        syntax: tree_file.syntax_diagnostics(),
    };

    match command {
        CommandKind::Functions => {
            let symbols = match filter {
                SymbolFilter::Exported => tree_file
                    .exported_symbols()?
                    .into_iter()
                    .filter(|s| s.kind.is_function())
                    .collect(),
                SymbolFilter::Prelude(filter) => tree_file
                    .symbols()?
                    .into_iter()
                    .filter(|s| s.kind.is_function() && filter.names.contains(&s.name))
                    .collect(),
                SymbolFilter::None => tree_file
                    .symbols()?
                    .into_iter()
                    .filter(|s| s.kind.is_function())
                    .collect(),
            };
            summary.symbols = apply_symbol_filters(symbols, include_symbol_globs, exclude_symbol_globs);
        }
        CommandKind::Types => {
            let symbols = match filter {
                SymbolFilter::Exported => tree_file
                    .exported_symbols()?
                    .into_iter()
                    .filter(|s| s.kind.is_type())
                    .collect(),
                SymbolFilter::Prelude(filter) => tree_file
                    .symbols()?
                    .into_iter()
                    .filter(|s| s.kind.is_type() && filter.names.contains(&s.name))
                    .collect(),
                SymbolFilter::None => tree_file
                    .symbols()?
                    .into_iter()
                    .filter(|s| s.kind.is_type())
                    .collect(),
            };
            summary.symbols = apply_symbol_filters(symbols, include_symbol_globs, exclude_symbol_globs);
        }
        CommandKind::Symbols => {
            match filter {
                SymbolFilter::Exported => {
                    summary.symbols = tree_file.exported_symbols()?;
                    summary.exports = tree_file.exported_symbols()?;
                }
                SymbolFilter::Prelude(filter) => {
                    if filter.exports_by_file.is_empty() {
                        // Backwards-compatible fallback for PRELUDE env-only filtering.
                        summary.symbols = tree_file
                            .symbols()?
                            .into_iter()
                            .filter(|s| filter.names.contains(&s.name))
                            .collect();
                        summary.exports = tree_file
                            .exported_symbols()?
                            .into_iter()
                            .filter(|s| filter.names.contains(&s.name))
                            .collect();
                        summary.locals = tree_file
                            .local_symbols()?
                            .into_iter()
                            .filter(|s| filter.names.contains(&s.name))
                            .collect();
                    } else if let Some(prelude_exports) = filter.exports_by_file.get(&tree_file.file)
                    {
                        summary.symbols = prelude_exports.clone();
                        summary.exports = summary.symbols.clone();
                        summary.locals.clear();
                    } else {
                        summary.symbols.clear();
                        summary.exports.clear();
                        summary.locals.clear();
                    }
                }
                SymbolFilter::None => {
                    summary.symbols = tree_file.symbols()?;
                    summary.imports = tree_file.imported_symbols()?;
                    summary.exports = tree_file.exported_symbols()?;
                    summary.locals = tree_file.local_symbols()?;
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
            summary.imports = tree_file.imported_symbols()?;
        }
        CommandKind::Lint { .. } => {
            // Lint diagnostics are already populated above
        }
        CommandKind::Classes { .. } => {
            // Classes are handled separately in main()
        }
    }

    Ok(summary)
}

fn symbol_from_prelude_export(
    import: &ImportSymbol,
    kind: SymbolKind,
    doc_comment: Option<String>,
    resolved_file: Option<PathBuf>,
    resolved_range: Option<CodeRange>,
) -> SymbolInfo {
    SymbolInfo {
        name: import.name.clone(),
        kind,
        range: resolved_range.unwrap_or_else(|| import.range.clone()),
        language: import.language,
        file: resolved_file.unwrap_or_else(|| import.file.clone()),
        container_name: None,
        container_kind: None,
        doc_comment,
        signature: None,
        type_metadata: None,
    }
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
        } => render_diagnostics_filtered(
            &summary.lint,
            &summary.syntax,
            &summary.file,
            config,
            *lint_only,
            *syntax_only,
        ),
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

fn group_imports(imports: &[ImportSymbol]) -> Vec<Vec<&ImportSymbol>> {
    let mut groups: BTreeMap<(usize, usize), Vec<&ImportSymbol>> = BTreeMap::new();
    for import in imports {
        let key = import
            .statement_range
            .as_ref()
            .map(|range| (range.start_line, range.start_column))
            .unwrap_or((import.range.start_line, import.range.start_column));
        groups.entry(key).or_default().push(import);
    }

    let mut result = Vec::new();
    for (_, mut group) in groups {
        group.sort_by_key(|import| (import.range.start_line, import.range.start_column));
        result.push(group);
    }

    result
}

fn dedupe_import_group<'a>(imports: &'a [&'a ImportSymbol]) -> Vec<&'a ImportSymbol> {
    let mut alias_originals = HashSet::new();
    for import in imports {
        if import.alias.is_some()
            && let Some(original) = import.original_name.as_deref()
        {
            alias_originals.insert((import.source.as_deref(), original));
        }
    }

    let mut result = Vec::new();
    for import in imports {
        let is_alias_shadow = import.alias.is_none()
            && import.original_name.is_none()
            && alias_originals.contains(&(import.source.as_deref(), import.name.as_str()));
        if is_alias_shadow {
            continue;
        }
        result.push(*import);
    }

    result
}

fn format_import_locations(imports: &[&ImportSymbol]) -> (String, usize) {
    let mut positions: Vec<(usize, usize)> = imports
        .iter()
        .map(|import| (import.range.start_line, import.range.start_column))
        .collect();
    positions.sort();

    let (first_line, _) = positions.first().copied().unwrap_or((1, 1));
    let location = if positions.iter().all(|(line, _)| *line == first_line) {
        let columns = positions
            .iter()
            .map(|(_, column)| column.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}:{}]", first_line, columns)
    } else {
        let entries = positions
            .iter()
            .map(|(line, column)| format!("{}:{}", line, column))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", entries)
    };

    (location, first_line)
}

fn format_import_group_display(imports: &[&ImportSymbol]) -> String {
    let language = imports
        .first()
        .map(|import| import.language)
        .unwrap_or(ProgrammingLanguage::Rust);
    match language {
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            format_ecma_import_group(imports)
        }
        ProgrammingLanguage::Python => format_python_import_group(imports),
        ProgrammingLanguage::Rust => format_rust_import_group(imports),
        ProgrammingLanguage::Go => format_go_import_group(imports),
        ProgrammingLanguage::Java => format_java_import_group(imports),
        ProgrammingLanguage::CSharp => format_csharp_import_group(imports),
        ProgrammingLanguage::Php => format_php_import_group(imports),
        ProgrammingLanguage::Scala => format_scala_import_group(imports),
        ProgrammingLanguage::Swift => format_swift_import_group(imports),
        _ => format_generic_import_group(imports),
    }
}

fn format_ecma_import_group(imports: &[&ImportSymbol]) -> String {
    let source = imports.first().and_then(|import| import.source.as_deref());
    let is_namespace = imports.len() == 1
        && imports[0]
            .original_name
            .as_deref()
            .is_some_and(|name| name == "*");

    if is_namespace {
        let alias = &imports[0].name;
        if let Some(source) = source {
            return format!("import * as {} from \"{}\"", alias, source);
        }
        return format!("import * as {}", alias);
    }

    let specs = imports
        .iter()
        .map(|import| {
            if let Some(alias) = import.alias.as_deref() {
                let original = import.original_name.as_deref().unwrap_or(&import.name);
                format!("{} as {}", original, alias)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(source) = source {
        format!("import {{ {} }} from \"{}\"", specs, source)
    } else {
        format!("import {{ {} }}", specs)
    }
}

fn format_python_import_group(imports: &[&ImportSymbol]) -> String {
    let sources: HashSet<&str> = imports
        .iter()
        .filter_map(|import| import.source.as_deref())
        .collect();

    let specs = |import: &ImportSymbol| {
        if let Some(alias) = import.alias.as_deref() {
            let original = import.original_name.as_deref().unwrap_or(&import.name);
            format!("{} as {}", original, alias)
        } else {
            import.name.clone()
        }
    };

    if sources.len() == 1 {
        let source = *sources.iter().next().unwrap();
        let is_import_stmt = imports.iter().all(|import| {
            import.source.as_deref() == Some(source)
                && (import.name == source || import.original_name.as_deref() == Some(source))
        });
        let spec_list = imports
            .iter()
            .map(|import| specs(import))
            .collect::<Vec<_>>()
            .join(", ");
        if is_import_stmt {
            format!("import {}", spec_list)
        } else {
            format!("from {} import {}", source, spec_list)
        }
    } else {
        let spec_list = imports
            .iter()
            .map(|import| specs(import))
            .collect::<Vec<_>>()
            .join(", ");
        format!("import {}", spec_list)
    }
}

fn format_rust_import_group(imports: &[&ImportSymbol]) -> String {
    let source = imports.first().and_then(|import| import.source.as_deref());
    let specs = imports
        .iter()
        .map(|import| {
            if let Some(alias) = import.alias.as_deref() {
                let original = import.original_name.as_deref().unwrap_or(&import.name);
                let stripped = source
                    .and_then(|src| original.strip_prefix(&format!("{}::", src)))
                    .unwrap_or(original);
                format!("{} as {}", stripped, alias)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>();

    if let Some(source) = source {
        if specs.len() == 1 {
            let spec = &specs[0];
            if spec.contains("::") {
                format!("use {}", spec)
            } else {
                format!("use {}::{}", source, spec)
            }
        } else {
            format!("use {}::{{{}}}", source, specs.join(", "))
        }
    } else {
        format!("use {}", specs.join(", "))
    }
}

fn format_go_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let path = import.source.as_deref().unwrap_or(&import.name);
            let quoted = format!("\"{}\"", path);
            if let Some(alias) = import.alias.as_deref() {
                format!("{} {}", alias, quoted)
            } else {
                quoted
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

fn format_java_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            if import.original_name.as_deref() == Some("*") {
                if let Some(source) = import.source.as_deref() {
                    format!("{}.*", source)
                } else {
                    "*".to_string()
                }
            } else if let Some(source) = import.source.as_deref() {
                format!("{}.{}", source, import.name)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

fn format_csharp_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.source.as_deref().unwrap_or(&import.name).to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!("using {}", specs)
}

fn format_php_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| {
            let base = import.source.as_deref().unwrap_or(&import.name);
            if let Some(alias) = import.alias.as_deref() {
                format!("{} as {}", base, alias)
            } else {
                base.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("use {}", specs)
}

fn format_scala_import_group(imports: &[&ImportSymbol]) -> String {
    if let Some(source) = imports.first().and_then(|import| import.source.as_deref()) {
        return format!("import {}", source);
    }

    let specs = imports
        .iter()
        .map(|import| {
            if let Some(alias) = import.alias.as_deref() {
                let original = import.original_name.as_deref().unwrap_or(&import.name);
                format!("{} => {}", original, alias)
            } else {
                import.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

fn format_swift_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.source.as_deref().unwrap_or(&import.name).to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!("import {}", specs)
}

fn format_generic_import_group(imports: &[&ImportSymbol]) -> String {
    let specs = imports
        .iter()
        .map(|import| import.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    format!("import {}", specs)
}

/// Extracts class summaries from a file.
fn extract_class_summaries(
    tree_file: &TreeFile,
    name_filter: Option<&str>,
    static_only: bool,
    instance_only: bool,
) -> Result<Vec<ClassSummary>, TreeHuggerError> {
    let all_symbols = tree_file.symbols()?;

    // Find all class-like symbols, sorted by line number
    let mut classes: Vec<&SymbolInfo> = all_symbols
        .iter()
        .filter(|s| s.kind.is_class())
        .filter(|s| match name_filter {
            Some(filter) => s.name.contains(filter),
            None => true,
        })
        .collect();
    classes.sort_by_key(|s| s.range.start_line);

    // Find all methods, sorted by line number
    let mut methods: Vec<&SymbolInfo> = all_symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Method)
        .collect();
    methods.sort_by_key(|s| s.range.start_line);

    let mut result = Vec::new();

    for (i, class) in classes.iter().enumerate() {
        // Determine the range for this class's methods:
        // From the class declaration line to the next class declaration (or EOF)
        let class_start = class.range.start_line;
        let class_end = if i + 1 < classes.len() {
            classes[i + 1].range.start_line
        } else {
            usize::MAX
        };

        // Get methods that belong to this class (between this class and the next)
        let class_methods: Vec<&SymbolInfo> = methods
            .iter()
            .filter(|m| m.range.start_line > class_start && m.range.start_line < class_end)
            .copied()
            .collect();

        // Partition methods into static and instance
        let mut static_methods = Vec::new();
        let mut instance_methods = Vec::new();

        for method in class_methods {
            let is_static = method
                .signature
                .as_ref()
                .map(|s| s.is_static)
                .unwrap_or(false);
            if is_static {
                static_methods.push(method.clone());
            } else {
                instance_methods.push(method.clone());
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
            class: (*class).clone(),
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
