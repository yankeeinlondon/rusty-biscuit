use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum FileAssociation {
    ProgrammingLanguage,
    FrameworkFile,
    Configuration,
    Styling,
    Documentation,
    Data,
    Image,
    Binary,
    BinaryExecutable,
    Archive,
    Font,
    Audio,
    Video,
    #[default]
    Unknown,
}

impl FileAssociation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProgrammingLanguage => "Programming language",
            Self::FrameworkFile => "Framework file",
            Self::Configuration => "Configuration",
            Self::Styling => "Styling",
            Self::Documentation => "Documentation",
            Self::Data => "Data",
            Self::Image => "Image",
            Self::Binary => "Binary",
            Self::BinaryExecutable => "Binary executable",
            Self::Archive => "Archive",
            Self::Font => "Font",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for FileAssociation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammingLanguageType {
    #[default]
    CompiledBinary,
    CompiledIntermediate,
    Script,
    ShellScript,
}

impl ProgrammingLanguageType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CompiledBinary => "Compiled binary",
            Self::CompiledIntermediate => "Compiled intermediate",
            Self::Script => "Script",
            Self::ShellScript => "Shell script",
        }
    }
}

impl fmt::Display for ProgrammingLanguageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammingLanguage {
    Bash,
    Batch,
    C,
    Clojure,
    Cpp,
    CSharp,
    Css,
    Fish,
    FSharp,
    Go,
    Java,
    JavaScript,
    Jsonnet,
    Kotlin,
    Lua,
    Php,
    PowerShell,
    Python,
    Ruby,
    Rust,
    Scala,
    Shell,
    Swift,
    TypeScript,
    Wat,
    Zig,
    Zsh,
    #[default]
    Unknown,
}

impl ProgrammingLanguage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "Bash",
            Self::Batch => "Batch",
            Self::C => "C",
            Self::Clojure => "Clojure",
            Self::Cpp => "C++",
            Self::CSharp => "C#",
            Self::Css => "CSS",
            Self::Fish => "Fish",
            Self::FSharp => "F#",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::JavaScript => "JavaScript",
            Self::Jsonnet => "Jsonnet",
            Self::Kotlin => "Kotlin",
            Self::Lua => "Lua",
            Self::Php => "PHP",
            Self::PowerShell => "PowerShell",
            Self::Python => "Python",
            Self::Ruby => "Ruby",
            Self::Rust => "Rust",
            Self::Scala => "Scala",
            Self::Shell => "Shell",
            Self::Swift => "Swift",
            Self::TypeScript => "TypeScript",
            Self::Wat => "WebAssembly",
            Self::Zig => "Zig",
            Self::Zsh => "Zsh",
            Self::Unknown => "Unknown",
        }
    }

    pub const fn language_type(self) -> ProgrammingLanguageType {
        match self {
            Self::Rust | Self::Go | Self::C | Self::Cpp | Self::Swift | Self::Zig => {
                ProgrammingLanguageType::CompiledBinary
            }
            Self::Java
            | Self::Kotlin
            | Self::Scala
            | Self::Clojure
            | Self::CSharp
            | Self::FSharp
            | Self::Wat => ProgrammingLanguageType::CompiledIntermediate,
            Self::Shell | Self::Bash | Self::Zsh | Self::Fish | Self::PowerShell | Self::Batch => {
                ProgrammingLanguageType::ShellScript
            }
            Self::JavaScript
            | Self::TypeScript
            | Self::Python
            | Self::Ruby
            | Self::Lua
            | Self::Php
            | Self::Jsonnet
            | Self::Css
            | Self::Unknown => ProgrammingLanguageType::Script,
        }
    }

    pub fn from_hyperpolyglot(name: &str) -> Option<Self> {
        match name {
            "Bash" => Some(Self::Bash),
            "Batch" => Some(Self::Batch),
            "C" => Some(Self::C),
            "C#" => Some(Self::CSharp),
            "C++" => Some(Self::Cpp),
            "Clojure" => Some(Self::Clojure),
            "CSS" => Some(Self::Css),
            "Fish" => Some(Self::Fish),
            "F#" => Some(Self::FSharp),
            "Go" => Some(Self::Go),
            "Java" => Some(Self::Java),
            "JavaScript" => Some(Self::JavaScript),
            "Jsonnet" => Some(Self::Jsonnet),
            "Kotlin" => Some(Self::Kotlin),
            "Lua" => Some(Self::Lua),
            "PHP" => Some(Self::Php),
            "PowerShell" => Some(Self::PowerShell),
            "Python" => Some(Self::Python),
            "Ruby" => Some(Self::Ruby),
            "Rust" => Some(Self::Rust),
            "Scala" => Some(Self::Scala),
            "Shell" => Some(Self::Shell),
            "Swift" => Some(Self::Swift),
            "TypeScript" => Some(Self::TypeScript),
            "WebAssembly" => Some(Self::Wat),
            "Zig" => Some(Self::Zig),
            "Zsh" => Some(Self::Zsh),
            _ => None,
        }
    }
}

impl fmt::Display for ProgrammingLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkKind {
    Vue,
    Svelte,
    Astro,
    AngularTemplate,
    RemixRouteModule,
    NextAppRouter,
    #[default]
    Unknown,
}

impl FrameworkKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vue => "Vue",
            Self::Svelte => "Svelte",
            Self::Astro => "Astro",
            Self::AngularTemplate => "Angular template",
            Self::RemixRouteModule => "Remix route module",
            Self::NextAppRouter => "Next app router",
            Self::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for FrameworkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationConfidence {
    Exact,
    High,
    Medium,
    #[default]
    Low,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationSource {
    ExactFilename,
    Extension,
    Shebang,
    EmbeddedLanguageHint,
    BinarySignature,
    #[default]
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeDescriptor {
    pub association: FileAssociation,
    pub language: Option<ProgrammingLanguage>,
    pub language_type: Option<ProgrammingLanguageType>,
    pub framework: Option<FrameworkKind>,
    pub is_text: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileClassification {
    pub path: PathBuf,
    pub association: FileAssociation,
    pub language: Option<ProgrammingLanguage>,
    pub language_type: Option<ProgrammingLanguageType>,
    pub framework: Option<FrameworkKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_languages: Vec<ProgrammingLanguage>,
    pub confidence: ClassificationConfidence,
    pub source: ClassificationSource,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileScanScope {
    pub root: PathBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_roots: Vec<PathBuf>,
}

/// Shared file inventory with reference-counted classifications to avoid clones.
///
/// The `classifications` field uses `Arc<Vec<...>>` so that filtering operations
/// can share the underlying data instead of cloning every classification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileInventory {
    pub scope: FileScanScope,
    pub total_files_scanned: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classifications: Arc<Vec<FileClassification>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAssociationStats {
    pub association: FileAssociation,
    pub file_count: usize,
    pub percentage: f64,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammingLanguageStats {
    pub language: ProgrammingLanguage,
    pub language_type: ProgrammingLanguageType,
    pub direct_file_count: usize,
    pub framework_file_count: usize,
    pub total_file_count: usize,
    pub signal: f64,
    pub percentage: f64,
    pub direct_files: Vec<PathBuf>,
    pub framework_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkStats {
    pub framework: FrameworkKind,
    pub file_count: usize,
    pub explicit_file_count: usize,
    pub inferred_file_count: usize,
    pub related_languages: Vec<ProgrammingLanguage>,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageSummary {
    pub primary: Option<ProgrammingLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary: Vec<ProgrammingLanguage>,
    pub total_files_scanned: usize,
    pub total_language_files: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<ProgrammingLanguageStats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<FrameworkStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileAssociationBreakdown {
    pub total_files: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_association: Vec<FileAssociationStats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_language: Vec<ProgrammingLanguageStats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_framework: Vec<FrameworkStats>,
}
