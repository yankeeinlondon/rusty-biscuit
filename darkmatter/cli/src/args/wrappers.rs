use clap::ValueEnum;
use darkmatter::layout::PageBackground;
use darkmatter::markdown::highlighting::CodeBlockMode;
use renderable::layout;

/// CLI-usable [`PageBackground`] wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PageBackgroundArg {
    /// Transparent (default).
    Transparent,
    /// Slightly off-background fill.
    Subtle,
    /// High-contrast inverse fill.
    Pronounced,
}

impl From<PageBackgroundArg> for PageBackground {
    fn from(arg: PageBackgroundArg) -> Self {
        match arg {
            PageBackgroundArg::Transparent => PageBackground::Transparent,
            PageBackgroundArg::Subtle => PageBackground::Subtle,
            PageBackgroundArg::Pronounced => PageBackground::Pronounced,
        }
    }
}

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

/// CLI-usable alignment wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PageAlignmentArg {
    /// Left-aligned.
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

impl From<PageAlignmentArg> for layout::Alignment {
    fn from(arg: PageAlignmentArg) -> Self {
        match arg {
            PageAlignmentArg::Left => layout::Alignment::Left,
            PageAlignmentArg::Center => layout::Alignment::Center,
            PageAlignmentArg::Right => layout::Alignment::Right,
        }
    }
}

/// CLI-local fill descriptor that maps directly onto renderable [`Layout`]
/// properties.
#[derive(Clone, Debug, PartialEq)]
pub enum CliFill {
    /// Default. Component may use the full content width.
    Full,
    /// Symmetric padding on both sides.
    Pad(layout::Length),
    /// One-sided padding driven by the component's alignment.
    Indent(layout::Length),
    /// Cap on the component's render width.
    Max(layout::Length),
    /// Explicit render width.
    Explicit(layout::Length),
}
