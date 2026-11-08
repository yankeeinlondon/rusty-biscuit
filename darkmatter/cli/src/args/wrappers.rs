use clap::ValueEnum;
use darkmatter::markdown::highlighting::CodeBlockMode;

/// CLI-usable [`CodeBlockMode`] wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CodeBlockArg {
    /// Opposite variant from the page (default): dark page -> light panel.
    Inverse,
    /// Always the dark variant.
    Dark,
    /// Always the light variant.
    Light,
    /// Same variant as the page.
    Same,
}

impl From<CodeBlockArg> for CodeBlockMode {
    fn from(arg: CodeBlockArg) -> Self {
        match arg {
            CodeBlockArg::Inverse => CodeBlockMode::Inverse,
            CodeBlockArg::Dark => CodeBlockMode::Dark,
            CodeBlockArg::Light => CodeBlockMode::Light,
            CodeBlockArg::Same => CodeBlockMode::Same,
        }
    }
}
