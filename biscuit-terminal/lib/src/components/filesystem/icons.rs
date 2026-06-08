//! Nerd Font icons for filesystem entries.
//!
//! All icons have both a Nerd Font variant and a Unicode fallback.
//! The Nerd Font icons require a patched font (e.g., from nerdfonts.com).

/// Nerd Font icons (require patched fonts).
pub mod nerd {
    /// Directory icons.
    pub mod dir {
        /// Base directory icon.
        pub const BASE: char = '\u{e5fe}';
        /// Directory at depth limit (contents not shown).
        pub const DEPTH_LIMIT: char = '\u{e652}';
        /// Directory with permission error (unreadable).
        pub const ERROR: char = '\u{f071}'; // Warning triangle
        /// `.git` directory.
        pub const GIT: char = '\u{e5fb}';
        /// `.github` directory.
        pub const GITHUB: char = '\u{e5fd}';
        /// `utils` or `util` directory.
        pub const UTILS: char = '\u{f19fc}';
        /// `docs` or `documentation` directory.
        pub const DOCS: char = '\u{ebdf}';
        /// Repository root directory (used when the root starts at the repo root).
        pub const REPO: char = '\u{f1d3}'; // Git icon
    }

    /// File icons.
    pub mod file {
        /// Base file icon.
        pub const BASE: char = '\u{ea7b}';
        /// Markdown files (`.md`).
        pub const MARKDOWN: char = '\u{f0354}';
        /// README files.
        pub const README: char = '\u{f02e}';
        /// CLAUDE.md file (depth 0 only).
        pub const CLAUDE: char = '\u{f0721}';
        /// SKILL.md file.
        pub const SKILL: char = '\u{f113c}';
        /// Agents.md, Gemini.md files (depth 0 only).
        pub const AGENTS: char = '\u{f21b}'; // Robot icon
        /// Symlink indicator.
        pub const SYMLINK: char = '\u{eaee}';
        /// `.gitignore` file.
        pub const GITIGNORE: char = '\u{e702}';
        /// `.env` file.
        pub const ENV: char = '\u{eafa}';
        /// `justfile`.
        pub const JUSTFILE: char = '\u{ee0d}';
        /// `.editorconfig` file.
        pub const EDITORCONFIG: char = '\u{e615}';
    }

    /// Extension-specific icons.
    pub mod ext {
        /// Rust files (`.rs`).
        pub const RUST: char = '\u{e7a8}';
        /// TypeScript files (`.ts`).
        pub const TYPESCRIPT: char = '\u{e8ca}';
        /// JavaScript files (`.js`).
        pub const JAVASCRIPT: char = '\u{e781}';
        /// TOML files (`.toml`).
        pub const TOML: char = '\u{e6b2}';
        /// YAML files (`.yaml`, `.yml`).
        pub const YAML: char = '\u{e8eb}';
        /// JSON files (`.json`).
        pub const JSON: char = '\u{eb0f}';
        /// PDF files (`.pdf`).
        pub const PDF: char = '\u{f1c1}';
        /// Word files (`.doc`, `.docx`).
        pub const WORD: char = '\u{f1c2}';
        /// Excel files (`.xls`, `.xlsx`).
        pub const EXCEL: char = '\u{f1c3}';
        /// Plain-text files (`.txt`).
        pub const TEXT: char = '\u{f1c9}';
    }

    /// Special file icons (deprecated, use `file` module instead).
    #[deprecated(since = "0.2.0", note = "Use icons::nerd::file instead")]
    pub mod special {
        /// `.gitignore` file.
        pub const GITIGNORE: char = super::file::GITIGNORE;
        /// `.env` file.
        pub const ENV: char = super::file::ENV;
        /// `justfile`.
        pub const JUSTFILE: char = super::file::JUSTFILE;
        /// `.editorconfig` file.
        pub const EDITORCONFIG: char = super::file::EDITORCONFIG;
    }
}

/// Unicode fallback icons (work in any terminal).
pub mod unicode {
    /// Directory icons.
    pub mod dir {
        /// Base directory icon.
        pub const BASE: char = '\u{1F4C2}'; // 📂
        /// Directory at depth limit.
        pub const DEPTH_LIMIT: char = '\u{1F4C1}'; // 📁
        /// Directory with permission error.
        pub const ERROR: char = '\u{26A0}'; // ⚠
    }

    /// File icons.
    pub mod file {
        /// Base file icon.
        pub const BASE: char = '\u{1F4C4}'; // 📄
        /// Symlink indicator.
        pub const SYMLINK: char = '@';
        /// Plain-text files (`.txt`).
        pub const TEXT: char = '\u{1F4DD}'; // 📝
        /// PDF files (`.pdf`).
        pub const PDF: char = '\u{1F4D5}'; // 📕
        /// Word documents (`.doc`, `.docx`).
        pub const WORD: char = '\u{1F4D8}'; // 📘
        /// Excel spreadsheets (`.xls`, `.xlsx`).
        pub const EXCEL: char = '\u{1F4D7}'; // 📗
    }
}
