use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use tree_hugger_lib::{
    FileSummary, ImportSymbol, PackageSummary, ProgrammingLanguage, SymbolInfo, TreeFile,
    TreeHuggerError,
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

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    /// Returns the output format based on flags.
    fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
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
    Pretty,
    Json,
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

    let root_dir = current_dir()?;
    let files = collect_files(&root_dir, inputs, &cli.ignore, language)?;

    let command_kind = cli.command.kind();
    let mut summaries = Vec::new();
    for file in files {
        let tree_file = TreeFile::with_language(&file, language)?;
        let summary = summarize_file(&tree_file, command_kind)?;
        summaries.push(summary);
    }

    match cli.output_format() {
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
        OutputFormat::Pretty => {
            for summary in summaries {
                render_summary(&summary, command_kind);
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

fn render_summary(summary: &FileSummary, command: CommandKind) {
    println!("{} ({})", summary.file.display(), summary.language);

    match command {
        CommandKind::Imports => render_imports(&summary.imports),
        CommandKind::Exports => render_symbols(&summary.exports),
        CommandKind::Functions | CommandKind::Types | CommandKind::Symbols => {
            render_symbols(&summary.symbols)
        }
    }

    println!();
}

fn render_symbols(symbols: &[SymbolInfo]) {
    if symbols.is_empty() {
        println!("  (no symbols)");
        return;
    }

    for symbol in symbols {
        println!(
            "  - {} {} [{}:{}]",
            symbol.kind, symbol.name, symbol.range.start_line, symbol.range.start_column
        );
    }
}

fn render_imports(imports: &[ImportSymbol]) {
    if imports.is_empty() {
        println!("  (no imports)");
        return;
    }

    for import in imports {
        println!(
            "  - {} [{}:{}]",
            import.name, import.range.start_line, import.range.start_column
        );
    }
}
