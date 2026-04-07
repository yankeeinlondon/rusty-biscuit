//! Shared ANSI escape code constants for diff renderers.

// Text styles
pub(crate) const RESET: &str = "\x1b[0m";
pub(crate) const BOLD: &str = "\x1b[1m";
pub(crate) const DIM: &str = "\x1b[2m";
pub(crate) const UNDERLINE: &str = "\x1b[4m";

// Background colors (256-color mode)
pub(crate) const BG_REMOVED: &str = "\x1b[48;5;52m";
pub(crate) const BG_ADDED: &str = "\x1b[48;5;22m";
pub(crate) const BG_CHANGED_DEL: &str = "\x1b[48;5;88m";
pub(crate) const BG_CHANGED_ADD: &str = "\x1b[48;5;28m";
