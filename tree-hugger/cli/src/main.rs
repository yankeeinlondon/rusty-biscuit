use std::io::IsTerminal;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Generator, Shell};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use owo_colors::{OwoColorize, Style};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use tree_hugger_lib::{
    Diagnostic, DiagnosticKind, DiagnosticSeverity, FieldInfo, FileSummary, FunctionSignature,
    ImportSymbol, LintDiagnostic, PackageSummary, ParameterInfo, ProgrammingLanguage,
    SourceContext, SymbolInfo, SymbolKind, SyntaxDiagnostic, TreeFile, TreeHuggerError,
    TypeMetadata, VariantInfo,
};
use serde::{Deserialize, Serialize};

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

/// Arguments for the classes command
#[derive(clap::Args, Debug, Clone)]
struct ClassArgs {
    /// Glob patterns for files to include
    #[arg(value_name = "GLOB", num_args = 1..)]
    inputs: Vec<String>,

    /// Filter by class name
    #[arg(long, short = 'n')]
    name: Option<String>,

    /// Show only static members
    #[arg(long)]
    static_only: bool,

    /// Show only instance members
    #[arg(long)]
    instance_only: bool,
}

/// Arguments for the lint command
#[derive(clap::Args, Debug, Clone)]
struct LintArgs {
    /// Glob patterns for files to include
    #[arg(value_name = "GLOB", num_args = 1..)]
    inputs: Vec<String>,

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
    /// List exported symbols in the file(s)
    Exports(CommonArgs),
    /// List imported symbols in the file(s)
    Imports(CommonArgs),
    /// List classes and their members
    Classes(ClassArgs),
    /// Run lint diagnostics on the file(s)
    Lint(LintArgs),
    /// Generate shell completions
    Completions(CompletionsArgs),
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
            Self::Lint(args) => &args.inputs,
            Self::Classes(args) => &args.inputs,
            Self::Completions(_) => &[],
        }
    }

    /// Returns the command kind for dispatching operations.
    fn kind(&self) -> Option<CommandKind> {
        match self {
            Self::Functions(_) => Some(CommandKind::Functions),
            Self::Types(_) => Some(CommandKind::Types),
            Self::Symbols(_) => Some(CommandKind::Symbols),
            Self::Exports(_) => Some(CommandKind::Exports),
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
    Exports,
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

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn display_path(path: &Path, root: Option<&Path>) -> String {
    if let Some(root) = root
        && let Ok(relative) = path.strip_prefix(root) {
            return relative.display().to_string();
        }
    path.display().to_string()
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

    // Handle completions command early (doesn't need file processing)
    if let Command::Completions(args) = &cli.command {
        print_completions(args.shell, &mut Cli::command());
        return Ok(());
    }

    let language = cli.language.map(ProgrammingLanguage::from);
    let inputs = cli.command.inputs();
    let output_format = cli.output_format();
    let output_config = OutputConfig::new(output_format);

    let root_dir = current_dir()?;
    let display_root = find_repo_root(&root_dir);
    let files = collect_files(&root_dir, inputs, &cli.ignore, language)?;

    let command_kind = cli.command.kind().expect("completions already handled");

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
            let class_summaries = extract_class_summaries(
                &tree_file,
                name_filter.as_deref(),
                *static_only,
                *instance_only,
            )?;
            if !class_summaries.is_empty() {
                all_class_summaries.push((tree_file.file.clone(), tree_file.language, class_summaries));
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
                    render_class_summaries(&file, lang, &summaries, &output_config, display_root.as_deref());
                }
            }
        }
        return Ok(());
    }

    let mut summaries = Vec::new();
    for file in files {
        let tree_file = TreeFile::with_language(&file, language)?;
        let summary = summarize_file(&tree_file, &command_kind)?;
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
                render_summary(&summary, &command_kind, &output_config, display_root.as_deref());
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
    clap_complete::generate(generator, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
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
    command: &CommandKind,
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
        CommandKind::Lint { .. } => {
            // Lint diagnostics are already populated above
        }
        CommandKind::Classes { .. } => {
            // Classes are handled separately in main()
        }
    }

    Ok(summary)
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
        CommandKind::Exports => render_symbols(&summary.exports, config),
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

        // Extract visibility for functions/methods
        let visibility = symbol
            .signature
            .as_ref()
            .and_then(|sig| sig.visibility.as_ref());

        if config.use_colors {
            let kind_style = style_for_kind(symbol.kind);

            // Format visibility (italicized) + kind + name
            let kind_part = match visibility {
                Some(vis) => format!(
                    "{} {}",
                    vis.to_string().italic(),
                    symbol.kind.to_string().style(kind_style)
                ),
                None => symbol.kind.to_string().style(kind_style).to_string(),
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
                Some(vis) => format!("{} {}", vis, symbol.kind),
                None => symbol.kind.to_string(),
            };
            println!("  - {} {} {}", kind_part, name_with_sig, location_display);
        }
    }
}

/// Formats a symbol name with its signature/metadata.
///
/// For functions/methods: `name(param1: T1, param2: T2) -> ReturnType`
/// For types: `name { field1: T1, field2: T2 }` or `name { Variant1, Variant2 }`
/// For other symbols: just the name
fn format_symbol_name(symbol: &SymbolInfo, language: ProgrammingLanguage) -> String {
    if symbol.kind.is_function() {
        return match &symbol.signature {
            Some(sig) => format_function_signature(&symbol.name, sig, language),
            None => symbol.name.clone(),
        };
    }

    if symbol.kind.is_type() {
        return match &symbol.type_metadata {
            Some(meta) => format_type_signature(&symbol.name, meta, language),
            None => symbol.name.clone(),
        };
    }

    symbol.name.clone()
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
fn format_type_signature(
    name: &str,
    meta: &TypeMetadata,
    language: ProgrammingLanguage,
) -> String {
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
            && let Some(original) = import.original_name.as_deref() {
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
                && (import.name == source
                    || import.original_name.as_deref() == Some(source))
        });
        let spec_list = imports.iter().map(|import| specs(import)).collect::<Vec<_>>().join(", ");
        if is_import_stmt {
            format!("import {}", spec_list)
        } else {
            format!("from {} import {}", source, spec_list)
        }
    } else {
        let spec_list = imports.iter().map(|import| specs(import)).collect::<Vec<_>>().join(", ");
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
        .map(|import| {
            import
                .source
                .as_deref()
                .unwrap_or(&import.name)
                .to_string()
        })
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
        .map(|import| {
            import
                .source
                .as_deref()
                .unwrap_or(&import.name)
                .to_string()
        })
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
            .filter(|m| {
                m.range.start_line > class_start && m.range.start_line < class_end
            })
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
        println!(
            "{} ({})",
            header.bold(),
            language.to_string().dimmed()
        );
    } else {
        println!("{} ({})", header, language);
    }

    for summary in summaries {
        render_class_summary(summary, language, config);
    }

    println!();
}

/// Renders a single class summary.
fn render_class_summary(summary: &ClassSummary, language: ProgrammingLanguage, config: &OutputConfig) {
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

    // Render static methods
    if !summary.static_methods.is_empty() {
        render_member_section("Static Methods", &summary.static_methods, language, config, true);
    }

    // Render instance methods
    if !summary.instance_methods.is_empty() {
        render_member_section("Instance Methods", &summary.instance_methods, language, config, true);
    }

    // Render static fields
    if !summary.static_fields.is_empty() {
        render_field_section("Static Fields", &summary.static_fields, language, config);
    }

    // Render instance fields
    if !summary.instance_fields.is_empty() {
        render_field_section("Instance Fields", &summary.instance_fields, language, config);
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
        let location = format!("[{}:{}]", method.range.start_line, method.range.start_column);
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
    }
}
