use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use owo_colors::{OwoColorize, Style};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use tree_hugger_lib::{
    FieldInfo, FileSummary, FunctionSignature, ImportSymbol, PackageSummary, ParameterInfo,
    ProgrammingLanguage, SymbolInfo, SymbolKind, TreeFile, TreeHuggerError, TypeMetadata,
    VariantInfo,
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
            Self::Classes(args) => &args.inputs,
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
            Self::Classes(args) => CommandKind::Classes {
                name_filter: args.name.clone(),
                static_only: args.static_only,
                instance_only: args.instance_only,
            },
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
                    render_class_summaries(&file, lang, &summaries, &output_config);
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
                render_summary(&summary, &command_kind, &output_config);
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
        CommandKind::Classes { .. } => {
            // Classes are handled separately in main()
        }
    }

    Ok(summary)
}

fn render_summary(summary: &FileSummary, command: &CommandKind, config: &OutputConfig) {
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

        // Format the import display: name [from source] [as alias]
        let import_display = format_import_display(import);

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

/// Formats an import for display, showing source and alias information.
///
/// Examples:
/// - `readFile from "fs"` - simple import with source
/// - `bar from "module" (originally foo)` - aliased import
/// - `* as ns from "module"` - namespace import
fn format_import_display(import: &ImportSymbol) -> String {
    let mut result = import.name.clone();

    // Add source information
    if let Some(source) = &import.source {
        result.push_str(" from ");
        // Determine if source looks like a path/module or a package
        if source.contains('/') || source.contains('\\') || source.contains("::") || source.contains('.') {
            result.push_str(source);
        } else {
            result.push('"');
            result.push_str(source);
            result.push('"');
        }
    }

    // Add alias information if present (shows what the original name was)
    if let Some(original) = &import.original_name {
        if original != &import.name && original != "*" {
            result.push_str(" (originally ");
            result.push_str(original);
            result.push(')');
        } else if original == "*" {
            // Namespace import: show as `* as name from "source"`
            result = format!("* as {}", import.name);
            if let Some(source) = &import.source {
                result.push_str(" from \"");
                result.push_str(source);
                result.push('"');
            }
        }
    }

    result
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
) {
    // Render file header
    let file_display = file.display().to_string();
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
