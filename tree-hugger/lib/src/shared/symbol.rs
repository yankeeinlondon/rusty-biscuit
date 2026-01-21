use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tree_sitter::Language;

/// Programming languages supported by tree-hugger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
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

impl ProgrammingLanguage {
    /// Returns a human-readable name for the language.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Go => "Go",
            Self::Python => "Python",
            Self::Java => "Java",
            Self::Php => "PHP",
            Self::Perl => "Perl",
            Self::Bash => "Bash",
            Self::Zsh => "Zsh",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::CSharp => "C#",
            Self::Swift => "Swift",
            Self::Scala => "Scala",
            Self::Lua => "Lua",
        }
    }

    /// Returns the query folder name used by tree-sitter.
    pub fn query_name(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::Python => "python",
            Self::Java => "java",
            Self::Php => "php",
            Self::Perl => "perl",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::CSharp => "c_sharp",
            Self::Swift => "swift",
            Self::Scala => "scala",
            Self::Lua => "lua",
        }
    }

    /// Returns file extensions associated with the language.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Self::TypeScript => &["ts", "mts", "cts"],
            Self::Go => &["go"],
            Self::Python => &["py"],
            Self::Java => &["java"],
            Self::Php => &["php", "phtml", "php3", "php4", "php5"],
            Self::Perl => &["pl", "pm", "t"],
            Self::Bash => &["sh", "bash"],
            Self::Zsh => &["zsh"],
            Self::C => &["c", "h"],
            Self::Cpp => &["cc", "cpp", "cxx", "hpp", "hh", "hxx"],
            Self::CSharp => &["cs"],
            Self::Swift => &["swift"],
            Self::Scala => &["scala", "sc"],
            Self::Lua => &["lua"],
        }
    }

    /// Maps a file extension to a supported language.
    pub fn from_extension(extension: &str) -> Option<Self> {
        let ext = extension.to_ascii_lowercase();

        for language in [
            Self::Rust,
            Self::JavaScript,
            Self::TypeScript,
            Self::Go,
            Self::Python,
            Self::Java,
            Self::Php,
            Self::Perl,
            Self::Bash,
            Self::Zsh,
            Self::C,
            Self::Cpp,
            Self::CSharp,
            Self::Swift,
            Self::Scala,
            Self::Lua,
        ] {
            if language
                .extensions()
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&ext))
            {
                return Some(language);
            }
        }

        None
    }

    /// Detects the language from a filesystem path.
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        Self::from_extension(extension)
    }

    /// Returns the tree-sitter language definition.
    pub fn tree_sitter_language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Perl => tree_sitter_perl::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::Zsh => tree_sitter_zsh::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Swift => tree_sitter_swift::LANGUAGE.into(),
            Self::Scala => tree_sitter_scala::LANGUAGE.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
        }
    }

    /// Returns the tree-sitter language for an extension.
    pub fn tree_sitter_language_for_extension(extension: &str) -> Option<Language> {
        let ext = extension.to_ascii_lowercase();

        match ext.as_str() {
            "ts" | "mts" | "cts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
            _ => Self::from_extension(&ext).map(|language| language.tree_sitter_language()),
        }
    }

    /// Detects the language and tree-sitter grammar for a file.
    pub fn detect(path: &Path) -> Option<(Self, Language)> {
        let extension = path.extension()?.to_str()?;
        let language = Self::from_extension(extension)?;
        let tree_sitter_language = Self::tree_sitter_language_for_extension(extension)?;

        Some((language, tree_sitter_language))
    }
}

impl fmt::Display for ProgrammingLanguage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Categorizes a discovered symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Type,
    Class,
    Interface,
    Enum,
    Trait,
    Module,
    Namespace,
    Variable,
    Parameter,
    Field,
    Macro,
    Constant,
    Unknown,
}

impl SymbolKind {
    /// Returns true for function-like symbols.
    pub fn is_function(self) -> bool {
        matches!(self, Self::Function | Self::Method)
    }

    /// Returns true for type-like symbols.
    pub fn is_type(self) -> bool {
        matches!(
            self,
            Self::Type | Self::Class | Self::Interface | Self::Enum | Self::Trait
        )
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Type => "type",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Variable => "variable",
            Self::Parameter => "parameter",
            Self::Field => "field",
            Self::Macro => "macro",
            Self::Constant => "constant",
            Self::Unknown => "unknown",
        };
        formatter.write_str(label)
    }
}

/// Severity levels for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// A source code range with line/column and byte offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRange {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// A code block captured from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub range: CodeRange,
    pub snippet: Option<String>,
}

/// A symbol extracted from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
}

/// An imported symbol reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSymbol {
    pub name: String,
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    pub source: Option<String>,
}

/// A lint diagnostic captured from the source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub message: String,
    pub range: CodeRange,
    pub severity: DiagnosticSeverity,
    pub rule: Option<String>,
}

/// A syntax diagnostic derived from parse errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxDiagnostic {
    pub message: String,
    pub range: CodeRange,
    pub severity: DiagnosticSeverity,
}

/// JSON-serializable summary for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    pub file: PathBuf,
    pub language: ProgrammingLanguage,
    pub hash: String,
    pub symbols: Vec<SymbolInfo>,
    pub imports: Vec<ImportSymbol>,
    pub exports: Vec<SymbolInfo>,
    pub locals: Vec<SymbolInfo>,
    pub lint: Vec<LintDiagnostic>,
    pub syntax: Vec<SyntaxDiagnostic>,
}

/// JSON-serializable summary for a package run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSummary {
    pub root_dir: PathBuf,
    pub language: ProgrammingLanguage,
    pub files: Vec<FileSummary>,
}
