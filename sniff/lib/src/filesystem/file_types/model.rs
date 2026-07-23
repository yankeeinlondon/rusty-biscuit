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
///
/// ## Notes
///
/// When [`truncated`](Self::truncated) is `true` the *selected subset* is
/// unspecified and may differ between runs over an unchanged tree: parallel
/// walker workers race to claim the bounded slots. Ordering is always
/// deterministic (classifications are sorted by path), and a complete result
/// — `truncated == false` — is fully deterministic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileInventory {
    pub scope: FileScanScope,
    /// Number of classifications represented in this result.
    ///
    /// Not an estimate of the tree's actual file count: when `truncated` is
    /// `true` the tree holds more files than this, and the count stops at
    /// `limit`.
    pub total_files_scanned: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classifications: Arc<Vec<FileClassification>>,
    /// Whether the accepted-classification cap was reached, leaving files in
    /// the tree unrepresented.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// The accepted-classification cap in force, present only when `truncated`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
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

/// ## Notes
///
/// `truncated`/`limit` mirror the [`FileInventory`] this summary was projected
/// from — every public inventory projection reports the same completeness
/// state, so a consumer never has to guess whether a summary is complete.
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
    /// See [`FileInventory::truncated`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// See [`FileInventory::limit`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl LanguageSummary {
    /// Adopt `inventory`'s completeness state.
    ///
    /// Every public projection of an inventory must report the same
    /// `truncated`/`limit` pair as the inventory it came from, so a consumer
    /// reading only the summary can still tell a complete result from a capped
    /// one.
    pub fn with_completeness_of(mut self, inventory: &FileInventory) -> Self {
        self.truncated = inventory.truncated;
        self.limit = inventory.limit;
        self
    }

    pub fn sorted(mut self) -> Self {
        for lang in &mut self.languages {
            lang.direct_files.sort();
            lang.framework_files.sort();
        }
        for fw in &mut self.frameworks {
            fw.files.sort();
        }
        self
    }
}

/// ## Notes
///
/// `truncated`/`limit` mirror the [`FileInventory`] this breakdown was
/// projected from. `total_files` is the number of classifications represented,
/// not the tree's file count — see [`FileInventory::total_files_scanned`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileAssociationBreakdown {
    pub total_files: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_association: Vec<FileAssociationStats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_language: Vec<ProgrammingLanguageStats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_framework: Vec<FrameworkStats>,
    /// See [`FileInventory::truncated`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
    /// See [`FileInventory::limit`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[cfg(test)]
mod completeness_serialization {
    use super::*;

    /// A complete inventory must serialize exactly as it did before the
    /// completeness fields existed.
    ///
    /// This is what makes the fields additive rather than a schema break: an
    /// existing `--json` consumer never sees a new key unless the result is
    /// actually truncated.
    #[test]
    fn complete_inventory_omits_the_completeness_fields() {
        let json = serde_json::to_value(FileInventory::default()).expect("serializes");
        assert!(json.get("truncated").is_none(), "got: {json}");
        assert!(json.get("limit").is_none(), "got: {json}");

        let summary = serde_json::to_value(LanguageSummary::default()).expect("serializes");
        assert!(summary.get("truncated").is_none(), "got: {summary}");
        assert!(summary.get("limit").is_none(), "got: {summary}");

        let breakdown =
            serde_json::to_value(FileAssociationBreakdown::default()).expect("serializes");
        assert!(breakdown.get("truncated").is_none(), "got: {breakdown}");
        assert!(breakdown.get("limit").is_none(), "got: {breakdown}");
    }

    #[test]
    fn truncated_inventory_reports_the_flag_and_the_accepted_cap() {
        let inventory = FileInventory {
            truncated: true,
            limit: Some(super::super::MAX_FILES),
            ..Default::default()
        };
        let json = serde_json::to_value(&inventory).expect("serializes");
        assert_eq!(json["truncated"], serde_json::json!(true));
        assert_eq!(json["limit"], serde_json::json!(super::super::MAX_FILES));

        // Every projection must agree with the inventory it came from, or a
        // consumer reading only the summary would call a capped result complete.
        let summary = LanguageSummary::default().with_completeness_of(&inventory);
        assert!(summary.truncated);
        assert_eq!(summary.limit, Some(super::super::MAX_FILES));
    }

    /// Absent fields must deserialize to "complete", so plans and results
    /// serialized before this phase keep round-tripping.
    #[test]
    fn legacy_json_without_the_fields_deserializes_as_complete() {
        let inventory: FileInventory =
            serde_json::from_value(serde_json::json!({ "scope": { "root": "/repo" }, "total_files_scanned": 3 }))
                .expect("legacy inventory JSON must still deserialize");
        assert!(!inventory.truncated);
        assert_eq!(inventory.limit, None);
    }
}
