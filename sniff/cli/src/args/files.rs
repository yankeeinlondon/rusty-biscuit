//! Files subcommand argument types.

use clap_complete::engine::ArgValueCandidates;

use super::{repo_package_area_candidates, repo_package_candidates};

/// Shared arguments for commands that list file paths.
#[derive(clap::Args, Debug, Clone)]
pub struct FileListArgs {
    /// Scope to a specific package
    #[arg(long, value_name = "PKG", add = ArgValueCandidates::new(repo_package_candidates))]
    pub package: Option<String>,

    /// Scope to a specific package area
    #[arg(long, value_name = "AREA", add = ArgValueCandidates::new(repo_package_area_candidates))]
    pub package_area: Option<String>,

    /// Output as bullet list (one item per line with `- ` prefix)
    #[arg(long, conflicts_with = "csv")]
    pub list: bool,

    /// Output as comma-separated values on a single line
    #[arg(long, conflicts_with = "list")]
    pub csv: bool,

    /// Show only basename (hide directory path)
    #[arg(long)]
    pub no_path: bool,

    /// Exit 0 with no output when no results found (default is exit 1)
    #[arg(long)]
    pub no_error: bool,

    /// Message to display when no results found
    #[arg(long, value_name = "MESSAGE")]
    pub on_error: Option<String>,

    /// Filter paths by substring match (OR logic)
    pub filter: Vec<String>,
}

/// Which change scope to inspect for blast-radius analysis.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum BlastRadiusScopeArg {
    #[default]
    Dirty,
    Staged,
    LastCommit,
}

/// Output shape for the `repo packages` subcommand.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PackagesFormat {
    /// Comma-separated values on a single line (default).
    #[default]
    Csv,
    /// Markdown unordered list (`- name` per line).
    Markdown,
    /// Plain list (one entry per line, no bullet).
    List,
}

/// Filter options for the files subcommand.
#[derive(Debug, Clone, Default)]
pub struct FilesFilter {
    pub association: Option<sniff::filesystem::FileAssociation>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum FileAssociationArg {
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
    Unknown,
}

impl From<FileAssociationArg> for sniff::filesystem::FileAssociation {
    fn from(value: FileAssociationArg) -> Self {
        match value {
            FileAssociationArg::ProgrammingLanguage => Self::ProgrammingLanguage,
            FileAssociationArg::FrameworkFile => Self::FrameworkFile,
            FileAssociationArg::Configuration => Self::Configuration,
            FileAssociationArg::Styling => Self::Styling,
            FileAssociationArg::Documentation => Self::Documentation,
            FileAssociationArg::Data => Self::Data,
            FileAssociationArg::Image => Self::Image,
            FileAssociationArg::Binary => Self::Binary,
            FileAssociationArg::BinaryExecutable => Self::BinaryExecutable,
            FileAssociationArg::Archive => Self::Archive,
            FileAssociationArg::Font => Self::Font,
            FileAssociationArg::Audio => Self::Audio,
            FileAssociationArg::Video => Self::Video,
            FileAssociationArg::Unknown => Self::Unknown,
        }
    }
}
