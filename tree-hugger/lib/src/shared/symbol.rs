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

        [
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
        ]
        .into_iter()
        .find(|language| {
            language
                .extensions()
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&ext))
        })
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

/// Visibility/access modifier for a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    /// Public access (e.g., `public` in Java/C#/TS, `pub` in Rust)
    Public,
    /// Protected access (e.g., `protected` in Java/C#/TS)
    Protected,
    /// Private access (e.g., `private` in Java/C#/TS)
    Private,
    /// Package-private/internal access (e.g., default in Java, `internal` in C#)
    Internal,
}

impl fmt::Display for Visibility {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
            Self::Internal => "internal",
        };
        formatter.write_str(label)
    }
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

    /// Returns true for class-like symbols that can have members.
    pub fn is_class(self) -> bool {
        matches!(self, Self::Class | Self::Type)
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

/// Context for displaying diagnostic source location.
///
/// Stores the line text and underline position for rendering diagnostics
/// with visual markers pointing to the problematic code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceContext {
    /// The full line of source text containing the diagnostic.
    pub line_text: String,
    /// Zero-based column offset where the underline starts.
    pub underline_column: usize,
    /// Length of the underline (matched text).
    pub underline_length: usize,
}

/// A code block captured from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub range: CodeRange,
    pub snippet: Option<String>,
}

/// Information about a function or method parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// The parameter name.
    pub name: String,
    /// The type annotation, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_annotation: Option<String>,
    /// The default value expression, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    /// Whether this is a variadic/rest parameter.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_variadic: bool,
}

impl ParameterInfo {
    /// Creates a new parameter with only a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: None,
            default_value: None,
            is_variadic: false,
        }
    }

    /// Creates a new parameter with a name and type.
    pub fn with_type(name: impl Into<String>, type_annotation: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: Some(type_annotation.into()),
            default_value: None,
            is_variadic: false,
        }
    }
}

/// Signature information for functions and methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// The list of parameters.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterInfo>,
    /// The return type, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    /// The visibility/access modifier (public, protected, private).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    /// Whether this is a static method or associated function.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_static: bool,
}

impl Default for FunctionSignature {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionSignature {
    /// Creates an empty function signature.
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
            return_type: None,
            visibility: None,
            is_static: false,
        }
    }

    /// Returns true if the signature has no information.
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
            && self.return_type.is_none()
            && self.visibility.is_none()
            && !self.is_static
    }
}

/// Information about a struct or class field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    /// The field name.
    pub name: String,
    /// The type annotation, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_annotation: Option<String>,
    /// Documentation comment for the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_comment: Option<String>,
    /// The visibility/access modifier (public, protected, private).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    /// Whether this is a static field.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_static: bool,
}

impl FieldInfo {
    /// Creates a new field with only a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: None,
            doc_comment: None,
            visibility: None,
            is_static: false,
        }
    }

    /// Creates a new field with a name and type.
    pub fn with_type(name: impl Into<String>, type_annotation: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_annotation: Some(type_annotation.into()),
            doc_comment: None,
            visibility: None,
            is_static: false,
        }
    }
}

/// Information about an enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantInfo {
    /// The variant name.
    pub name: String,
    /// For tuple variants, the field types.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tuple_fields: Vec<String>,
    /// For struct variants, the named fields.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub struct_fields: Vec<FieldInfo>,
    /// Documentation comment for the variant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_comment: Option<String>,
}

impl VariantInfo {
    /// Creates a unit variant (no payload).
    pub fn unit(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tuple_fields: Vec::new(),
            struct_fields: Vec::new(),
            doc_comment: None,
        }
    }

    /// Creates a tuple variant with field types.
    pub fn tuple(name: impl Into<String>, fields: Vec<String>) -> Self {
        Self {
            name: name.into(),
            tuple_fields: fields,
            struct_fields: Vec::new(),
            doc_comment: None,
        }
    }

    /// Creates a struct variant with named fields.
    pub fn with_fields(name: impl Into<String>, fields: Vec<FieldInfo>) -> Self {
        Self {
            name: name.into(),
            tuple_fields: Vec::new(),
            struct_fields: fields,
            doc_comment: None,
        }
    }
}

/// Metadata for type definitions (structs, enums, interfaces, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMetadata {
    /// For structs/classes: the list of fields.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldInfo>,
    /// For enums: the list of variants.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<VariantInfo>,
    /// Generic type parameters (e.g., T, U in Container<T, U>).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub type_parameters: Vec<String>,
}

impl Default for TypeMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeMetadata {
    /// Creates empty type metadata.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            variants: Vec::new(),
            type_parameters: Vec::new(),
        }
    }

    /// Returns true if the metadata has no information.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.variants.is_empty() && self.type_parameters.is_empty()
    }
}

/// A symbol extracted from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub range: CodeRange,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    /// Documentation comment associated with the symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_comment: Option<String>,
    /// Function/method signature information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<FunctionSignature>,
    /// Type metadata (fields, variants, etc.) for type-like symbols.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_metadata: Option<TypeMetadata>,
}

/// An imported symbol reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSymbol {
    /// The local name used in the file (the name you write when referencing the import).
    pub name: String,
    /// The original name from the source module (before aliasing).
    /// If not aliased, this is the same as `name`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_name: Option<String>,
    /// The alias if the import was renamed (e.g., `foo as bar` has alias `bar`).
    /// Only present when an explicit alias was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    pub range: CodeRange,
    /// The range of the full import statement (used for grouping).
    #[serde(skip)]
    pub statement_range: Option<CodeRange>,
    pub language: ProgrammingLanguage,
    pub file: PathBuf,
    /// The source module path (e.g., `"fs"`, `"typing"`, `"std::io"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A lint diagnostic captured from the source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub message: String,
    pub range: CodeRange,
    pub severity: DiagnosticSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    /// Source context for displaying the diagnostic with visual markers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<SourceContext>,
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
