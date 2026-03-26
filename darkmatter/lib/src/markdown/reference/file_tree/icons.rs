//! Icon selection for FileTree reference and transclusion rows.
//!
//! Provides Nerd Font and Unicode fallback icons following the same
//! detection pattern as `biscuit_terminal::components::filesystem`.

use super::model::{FileTreeIconKind, FileTreeReferenceGroupKind, FileTreeTransclusionKind};

/// Nerd Font icon code points.
pub mod nerd {
    /// Link/globe icon.
    pub const HYPERLINK: char = '\u{f0c1}';
    /// Image/media icon.
    pub const IMAGE: char = '\u{f03e}';
    /// CSS3 icon.
    pub const CSS: char = '\u{e749}';
    /// Code icon.
    pub const SCRIPT: char = '\u{e60c}';
    /// Font icon.
    pub const FONT: char = '\u{f031}';
    /// Markdown file icon.
    pub const MARKDOWN: char = '\u{f0354}';
    /// Generic file icon.
    pub const FILE: char = '\u{ea7b}';
    /// Brain/agent icon.
    pub const BRAIN: char = '\u{f0362}';
}

/// Unicode fallback icons.
pub mod unicode {
    pub const HYPERLINK: &str = "\u{1F517}"; // 🔗
    pub const IMAGE: &str = "\u{1F5BC}"; // 🖼
    pub const CSS: &str = "\u{1F3A8}"; // 🎨
    pub const SCRIPT: &str = "\u{1F4DC}"; // 📜
    pub const FONT: &str = "\u{1F524}"; // 🔤
    pub const FILE: &str = "\u{1F4C4}"; // 📄
    pub const BRAIN: &str = "\u{1F9E0}"; // 🧠
}

/// Select icon string for a reference group kind.
pub fn reference_icon(kind: &FileTreeReferenceGroupKind, is_nerd_font: bool) -> String {
    if is_nerd_font {
        let ch = match kind {
            FileTreeReferenceGroupKind::RemoteHyperlinks
            | FileTreeReferenceGroupKind::LocalHyperlinks => nerd::HYPERLINK,
            FileTreeReferenceGroupKind::Images => nerd::IMAGE,
            FileTreeReferenceGroupKind::CssImports => nerd::CSS,
            FileTreeReferenceGroupKind::ScriptImports => nerd::SCRIPT,
            FileTreeReferenceGroupKind::FontImports => nerd::FONT,
            FileTreeReferenceGroupKind::OtherLocalDependencies => nerd::FILE,
        };
        format!("{ch} ")
    } else {
        let s = match kind {
            FileTreeReferenceGroupKind::RemoteHyperlinks
            | FileTreeReferenceGroupKind::LocalHyperlinks => unicode::HYPERLINK,
            FileTreeReferenceGroupKind::Images => unicode::IMAGE,
            FileTreeReferenceGroupKind::CssImports => unicode::CSS,
            FileTreeReferenceGroupKind::ScriptImports => unicode::SCRIPT,
            FileTreeReferenceGroupKind::FontImports => unicode::FONT,
            FileTreeReferenceGroupKind::OtherLocalDependencies => unicode::FILE,
        };
        format!("{s} ")
    }
}

/// Select icon string for a file node.
pub fn file_icon(kind: &FileTreeIconKind, is_nerd_font: bool) -> String {
    if is_nerd_font {
        let ch = match kind {
            FileTreeIconKind::Markdown => nerd::MARKDOWN,
            FileTreeIconKind::GenericFile => nerd::FILE,
        };
        format!("{ch} ")
    } else {
        format!("{} ", unicode::FILE)
    }
}

/// Select icon string for a transclusion edge.
pub fn transclusion_icon(kind: &FileTreeTransclusionKind, is_nerd_font: bool) -> String {
    match kind {
        FileTreeTransclusionKind::Url => {
            if is_nerd_font {
                format!("{} ", nerd::BRAIN)
            } else {
                format!("{} ", unicode::BRAIN)
            }
        }
        _ => file_icon(&FileTreeIconKind::Markdown, is_nerd_font),
    }
}
