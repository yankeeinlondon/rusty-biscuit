use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use owo_colors::{OwoColorize, Style};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use tree_hugger_lib::{
    FileSummary, FunctionSignature, ImportSymbol, PackageSummary, ParameterInfo,
    ProgrammingLanguage, SymbolInfo, SymbolKind, TreeFile, TreeHuggerError,
};

#[derive(Parser, Debug)]
#[command(
    name = "hug",
    version,
    about = "Tree Hugger diagnostics and symbol tooling"
)]
struct Cli {
    /// Glob patterns for files to ignore
    #[arg(long, value_name = "GLOB", global = true)]
    ignore: Vec<String>,

    /// Force a specific language
    #[arg(long, value_enum, global = true)]
    language: Option<LanguageArg>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Disable colors and hyperlinks (plain text output)
    #[arg(long, global = true)]
    plain: bool,

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
    /// Glob patterns for files to include
    #[arg(value_name = "GLOB", num_args = 1..)]
    inputs: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// List functions in the file(s)
    Functions(CommonArgs),
    /// List types in the file(s)
    Types(CommonArgs),
    /// List all symbols in the file(s)
    Symbols(CommonArgs),
    /// List exported symbols in the file(s)
    Exports(CommonArgs),
    /// List imported symbols in the file(s)
    Imports(CommonArgs),
}

impl Command {
    /// Returns the input glob patterns from the subcommand.
    fn inputs(&self) -> &[String] {
        match self {
            Self::Functions(args)
            | Self::Types(args)
            | Self::Symbols(args)
            | Self::Exports(args)
            | Self::Imports(args) => &args.inputs,
        }
    }

    /// Returns the command kind for dispatching operations.
    fn kind(&self) -> CommandKind {
        match self {
            Self::Functions(_) => CommandKind::Functions,
            Self::Types(_) => CommandKind::Types,
            Self::Symbols(_) => CommandKind::Symbols,
            Self::Exports(_) => CommandKind::Exports,
            Self::Imports(_) => CommandKind::Imports,
        }
    }
}

/// The kind of command being executed (without the arguments).
#[derive(Debug, Clone, Copy)]
enum CommandKind {
    Functions,
    Types,
    Symbols,
    Exports,
    Imports,
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

impl OutputConfig {
    fn new(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Pretty => {
                // Check NO_COLOR environment variable and TTY
                let no_color = std::env::var("NO_COLOR").is_ok();
                let is_tty = std::io::stdout().is_terminal();
                let use_colors = !no_color && is_tty;
                Self {
                    use_colors,
                    use_hyperlinks: use_colors && is_tty,
                }
            }
            OutputFormat::Plain | OutputFormat::Json => Self {
                use_colors: false,
                use_hyperlinks: false,
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

/// Creates an OSC8 hyperlink for a file path with line number.
fn hyperlink(path: &Path, line: usize, text: &str) -> String {
    let path_str = path.to_string_lossy();
    let encoded = utf8_percent_encode(&path_str, NON_ALPHANUMERIC);
    format!(
        "\x1b]8;;file://{}#L{}\x1b\\{}\x1b]8;;\x1b\\",
        encoded, line, text
    )
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum LanguageArg {
    Rust,
    JavaScript,
    TypeScript,
    Go,
    Python,
    Java,
    Php,
    Perl,
    Bash,
    Zsh,
    C,
    Cpp,
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
    let language = cli.language.map(ProgrammingLanguage::from);
    let inputs = cli.command.inputs();
    let output_format = cli.output_format();
    let output_config = OutputConfig::new(output_format);

    let root_dir = current_dir()?;
    let files = collect_files(&root_dir, inputs, &cli.ignore, language)?;

    let command_kind = cli.command.kind();
    let mut summaries = Vec::new();
    for file in files {
        let tree_file = TreeFile::with_language(&file, language)?;
        let summary = summarize_file(&tree_file, command_kind)?;
        summaries.push(summary);
    }

    match output_format {
        OutputFormat::Json => {
            let package_language = language
                .or_else(|| summaries.first().map(|summary| summary.language))
                .unwrap_or(ProgrammingLanguage::Rust);

            let output = PackageSummary {
                root_dir,
                language: package_language,
                files: summaries,
            };

            let json =
                serde_json::to_string_pretty(&output).map_err(|source| TreeHuggerError::Io {
                    path: PathBuf::from("<stdout>"),
                    source: std::io::Error::other(source),
                })?;
            println!("{json}");
        }
        OutputFormat::Pretty | OutputFormat::Plain => {
            for summary in summaries {
                render_summary(&summary, command_kind, &output_config);
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

fn collect_files(
    root: &Path,
    inputs: &[String],
    ignores: &[String],
    language: Option<ProgrammingLanguage>,
) -> Result<Vec<PathBuf>, TreeHuggerError> {
    let mut overrides = OverrideBuilder::new(root);
    for input in inputs {
        overrides.add(input)?;
    }
    for ignore in ignores {
        overrides.add(&format!("!{}", ignore))?;
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

        if let Some(language) = language
            && ProgrammingLanguage::from_path(entry.path()) != Some(language)
        {
            continue;
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
    command: CommandKind,
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
            summary.symbols = tree_file
                .symbols()?
                .into_iter()
                .filter(|symbol| symbol.kind.is_function())
                .collect();
        }
        CommandKind::Types => {
            summary.symbols = tree_file
                .symbols()?
                .into_iter()
                .filter(|symbol| symbol.kind.is_type())
                .collect();
        }
        CommandKind::Symbols => {
            summary.symbols = tree_file.symbols()?;
            summary.imports = tree_file.imported_symbols()?;
            summary.exports = tree_file.exported_symbols()?;
            summary.locals = tree_file.local_symbols()?;
        }
        CommandKind::Exports => {
            summary.exports = tree_file.exported_symbols()?;
        }
        CommandKind::Imports => {
            summary.imports = tree_file.imported_symbols()?;
        }
    }

    Ok(summary)
}

fn render_summary(summary: &FileSummary, command: CommandKind, config: &OutputConfig) {
    // Render file header with optional hyperlink
    let file_display = summary.file.display().to_string();
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
        CommandKind::Exports => render_symbols(&summary.exports, config),
        CommandKind::Functions | CommandKind::Types | CommandKind::Symbols => {
            render_symbols(&summary.symbols, config)
        }
    }

    println!();
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
        let location = format!("[{}:{}]", symbol.range.start_line, symbol.range.start_column);
        let location_display = if config.use_hyperlinks {
            hyperlink(&symbol.file, symbol.range.start_line, &location)
        } else {
            location
        };

        // Format symbol name with signature for functions/methods
        let name_with_sig = format_symbol_name(symbol, symbol.language);

        if config.use_colors {
            let kind_style = style_for_kind(symbol.kind);
            println!(
                "  - {} {} {}",
                symbol.kind.to_string().style(kind_style),
                name_with_sig.bold(),
                location_display.dimmed()
            );
        } else {
            println!("  - {} {} {}", symbol.kind, name_with_sig, location_display);
        }
    }
}

/// Formats a symbol name with its signature for function-like symbols.
///
/// For functions/methods: `name(param1: T1, param2: T2) -> ReturnType`
/// For other symbols: just the name
fn format_symbol_name(symbol: &SymbolInfo, language: ProgrammingLanguage) -> String {
    if !symbol.kind.is_function() {
        return symbol.name.clone();
    }

    match &symbol.signature {
        Some(sig) => format_function_signature(&symbol.name, sig, language),
        None => symbol.name.clone(),
    }
}

/// Formats a function signature like: `name(param1: T1, param2: T2) -> ReturnType`
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

fn render_imports(imports: &[ImportSymbol], config: &OutputConfig) {
    if imports.is_empty() {
        if config.use_colors {
            println!("  {}", "(no imports)".dimmed());
        } else {
            println!("  (no imports)");
        }
        return;
    }

    for import in imports {
        let location = format!("[{}:{}]", import.range.start_line, import.range.start_column);
        let location_display = if config.use_hyperlinks {
            hyperlink(&import.file, import.range.start_line, &location)
        } else {
            location
        };

        if config.use_colors {
            println!(
                "  - {} {}",
                import.name.cyan(),
                location_display.dimmed()
            );
        } else {
            println!("  - {} {}", import.name, location_display);
        }
    }
}
