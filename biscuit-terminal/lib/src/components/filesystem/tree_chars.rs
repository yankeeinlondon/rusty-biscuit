//! Tree-drawing characters for box-drawing style output.

/// Branch connector for non-last items: `├── `
pub const BRANCH: &str = "├── ";
/// Branch connector for the last item: `└── `
pub const LAST_BRANCH: &str = "└── ";
/// Vertical continuation line: `│   `
pub const VERTICAL: &str = "│   ";
/// Indentation for after last item: `    ` (4 spaces)
pub const INDENT: &str = "    ";
