//! Icon selection for FileTree reference and transclusion rows.
//!
//! Provides Nerd Font and Unicode fallback icons following the same
//! detection pattern as `biscuit_terminal::components::filesystem`.

use super::model::{FileTreeIconKind, FileTreeReferenceGroupKind, FileTreeTransclusionKind};

/// Nerd Font icon code points.
pub mod nerd {
    /// URL hyperlink icon.
    pub const HYPERLINK: char = '\u{eb15}';
    /// Text link / local source icon.
    pub const LOCAL_LINK: char = '\u{f15c}';
    /// Image/media icon.
    pub const IMAGE: char = '\u{f03e}';
    /// Vector image (SVG) icon.
    pub const VECTOR_IMAGE: char = '\u{f0ae8}';
    /// CSS import icon.
    pub const CSS: char = '\u{e74a}';
    /// Script import icon.
    pub const SCRIPT: char = '\u{ed0d}';
    /// Font icon.
    pub const FONT: char = '\u{f031}';
    /// Markdown source icon (matches biscuit_terminal filesystem component).
    pub const MARKDOWN: char = '\u{f0354}';
    /// Generic text/file icon.
    pub const FILE: char = '\u{f15c}';
    /// PDF document icon.
    pub const PDF: char = '\u{f1c1}';
    /// Word document icon.
    pub const WORD_DOC: char = '\u{f022c}';
    /// Excel document icon.
    pub const EXCEL_DOC: char = '\u{f138f}';
    /// Brain/agent icon (URL transclusions).
    pub const BRAIN: char = '\u{f0362}';
}

/// Unicode fallback icons.
pub mod unicode {
    pub const HYPERLINK: &str = "\u{1F517}"; // 🔗
    pub const LOCAL_LINK: &str = "\u{1F4C4}"; // 📄
    pub const IMAGE: &str = "\u{1F4F8}"; // 📸
    pub const VECTOR_IMAGE: &str = "\u{270E}"; // ✎
    pub const CSS: &str = "\u{1F4C4}"; // 📄
    pub const SCRIPT: &str = "\u{1F4C4}"; // 📄
    pub const FONT: &str = "\u{1F524}"; // 🔤
    pub const FILE: &str = "\u{1F4C4}"; // 📄
    pub const MARKDOWN: &str = "\u{1F4C4}"; // 📄
    pub const PDF: &str = "\u{1F4C3}"; // 📃
    pub const WORD_DOC: &str = "\u{1F4C3}"; // 📃
    pub const EXCEL_DOC: &str = "\u{1F4C3}"; // 📃
    pub const BRAIN: &str = "\u{1F9E0}"; // 🧠
}

/// Select icon string for a reference group kind.
///
/// Uses two trailing spaces after nerd font icons so the glyph renders
/// at full width — matching transclusion edge and file head icons.
pub fn reference_icon(kind: &FileTreeReferenceGroupKind, is_nerd_font: bool) -> String {
    if is_nerd_font {
        let ch = match kind {
            FileTreeReferenceGroupKind::RemoteHyperlinks => nerd::HYPERLINK,
            FileTreeReferenceGroupKind::LocalHyperlinks => nerd::MARKDOWN,
            FileTreeReferenceGroupKind::Images => nerd::IMAGE,
            FileTreeReferenceGroupKind::Media => nerd::IMAGE,
            FileTreeReferenceGroupKind::Iframes => nerd::HYPERLINK,
            FileTreeReferenceGroupKind::CssImports => nerd::CSS,
            FileTreeReferenceGroupKind::ScriptImports => nerd::SCRIPT,
            FileTreeReferenceGroupKind::FontImports => nerd::FONT,
            FileTreeReferenceGroupKind::OtherLocalDependencies => nerd::FILE,
        };
        format!("{ch}  ")
    } else {
        let s = match kind {
            FileTreeReferenceGroupKind::RemoteHyperlinks => unicode::HYPERLINK,
            FileTreeReferenceGroupKind::LocalHyperlinks => unicode::MARKDOWN,
            FileTreeReferenceGroupKind::Images => unicode::IMAGE,
            FileTreeReferenceGroupKind::Media => unicode::IMAGE,
            FileTreeReferenceGroupKind::Iframes => unicode::HYPERLINK,
            FileTreeReferenceGroupKind::CssImports => unicode::CSS,
            FileTreeReferenceGroupKind::ScriptImports => unicode::SCRIPT,
            FileTreeReferenceGroupKind::FontImports => unicode::FONT,
            FileTreeReferenceGroupKind::OtherLocalDependencies => unicode::FILE,
        };
        format!("{s} ")
    }
}

/// Select icon string for a file node (file head line).
///
/// Uses two trailing spaces after nerd font icons so the glyph renders
/// at full width — matching the filesystem component's layout.
pub fn file_icon(kind: &FileTreeIconKind, is_nerd_font: bool) -> String {
    if is_nerd_font {
        let ch = match kind {
            FileTreeIconKind::Markdown => nerd::MARKDOWN,
            FileTreeIconKind::GenericFile => nerd::FILE,
        };
        format!("{ch} ")
    } else {
        let s = match kind {
            FileTreeIconKind::Markdown => unicode::MARKDOWN,
            FileTreeIconKind::GenericFile => unicode::FILE,
        };
        format!("{s} ")
    }
}

/// Select icon string for a transclusion edge.
///
/// Adds an extra trailing space after the nerd font icon so it has room
/// to render at the same visual size as icons in the file head line.
pub fn transclusion_icon(kind: &FileTreeTransclusionKind, is_nerd_font: bool) -> String {
    if is_nerd_font {
        let ch = match kind {
            FileTreeTransclusionKind::Url => nerd::BRAIN,
            _ => nerd::MARKDOWN,
        };
        format!("{ch}  ")
    } else {
        let s = match kind {
            FileTreeTransclusionKind::Url => unicode::BRAIN,
            _ => unicode::MARKDOWN,
        };
        format!("{s} ")
    }
}
