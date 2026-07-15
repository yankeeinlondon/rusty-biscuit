//! Syntax highlighting infrastructure for markdown code blocks and prose.
//!
//! This module provides theme enumeration, theme pairing (light/dark),
//! and grammar loading utilities for syntax highlighting using syntect.

pub(crate) mod grammars;
pub mod prose;
pub(crate) mod resolve;
pub(crate) mod scope_cache;
pub(crate) mod themes;

pub use themes::{
    CodeBlockMode, ColorMode, InvalidCodeBlockMode, InvalidThemeName, ThemePair, detect_code_theme,
    detect_color_mode, detect_prose_theme, get_code_theme_for_prose,
};

pub(crate) use resolve::Surface;

use crate::markdown::language_grammar::LanguageGrammar;

use std::fmt::Write as _;
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme as SyntectTheme;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Primary API for syntax highlighting with theme support.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
///
/// let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
/// ```
#[derive(Debug)]
pub struct CodeHighlighter {
    syntax_set: &'static SyntaxSet,
    theme: &'static SyntectTheme,
    color_mode: ColorMode,
}

impl CodeHighlighter {
    /// Creates a new code highlighter with the specified theme pair and color mode.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
    ///
    /// let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
    /// ```
    pub fn new(theme_pair: ThemePair, color_mode: ColorMode) -> Self {
        let syntax_set = grammars::load_syntax_set();
        let theme = themes::load_theme(theme_pair, color_mode);

        Self {
            syntax_set,
            theme,
            color_mode,
        }
    }

    /// Creates a [`CodeHighlighter`] from a fully-resolved
    /// [`themes::Theme`](super::highlighting::themes::Theme) variant and
    /// color mode.
    ///
    /// This is the single constructor used in production: callers feed
    /// `theme` and `color_mode` from a prior
    /// [`ThemePair::resolve_for_surface`](super::highlighting::themes::ThemePair::resolve_for_surface)
    /// call (the boundary resolver), so the page surface and the code panel
    /// share the same source of truth.
    pub(crate) fn from_theme(theme: themes::Theme, color_mode: ColorMode) -> Self {
        let syntax_set = grammars::load_syntax_set();
        let theme = themes::load_resolved_theme(theme);

        Self {
            syntax_set,
            theme,
            color_mode,
        }
    }

    /// Returns a reference to the syntax set.
    pub fn syntax_set(&self) -> &SyntaxSet {
        self.syntax_set
    }

    /// Returns a reference to the current theme.
    pub fn theme(&self) -> &SyntectTheme {
        self.theme
    }

    /// Returns the current color mode.
    pub fn color_mode(&self) -> ColorMode {
        self.color_mode
    }
}

impl Default for CodeHighlighter {
    fn default() -> Self {
        Self::new(ThemePair::Base16Ocean, ColorMode::Dark)
    }
}

/// Highlight raw YAML text into ANSI-styled lines, one per input line.
///
/// Returns one ANSI-styled string per input line, with no header row,
/// no background fill, and no fence borders. Each returned line ends
/// with a `\x1b[0m` reset so colors do not bleed into surrounding output.
///
/// Theme selection follows the same env-driven detection used elsewhere
/// in darkmatter (`detect_prose_theme` -> `detect_code_theme` plus
/// `detect_color_mode`), so output stays visually consistent with normal
/// markdown YAML fences in the host process.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::highlight_yaml_lines;
///
/// let lines = highlight_yaml_lines("foo: 1\nbar: 2");
/// assert_eq!(lines.len(), 2);
/// ```
///
/// ## Returns
///
/// A `Vec<String>` whose length matches the number of newline-separated
/// lines in `yaml`. If `yaml` is empty, returns a single empty styled line.
/// If syntax highlighting fails, falls back to the unstyled input lines.
///
/// ## Notes
///
/// The output never contains the literal `yaml` language label or the
/// dark code-block chrome that `YamlBlock::render` emits. This makes it
/// suitable for inline failure blocks and other render contexts where a
/// surrounding box would be visually heavy.
pub fn highlight_yaml_lines(yaml: &str) -> Vec<String> {
    let prose_theme = detect_prose_theme();
    let code_theme = detect_code_theme(prose_theme);
    let color_mode = detect_color_mode();
    highlight_yaml_lines_with_theme(yaml, code_theme, color_mode)
}

/// Like [`highlight_yaml_lines`] but with caller-controlled theme/color mode.
///
/// Useful in tests or in callers that have already resolved the theme via a
/// shared options struct (e.g. `TerminalOptions`) and want to avoid
/// re-running the env-driven detection path.
pub fn highlight_yaml_lines_with_theme(
    yaml: &str,
    theme_pair: ThemePair,
    color_mode: ColorMode,
) -> Vec<String> {
    let resolved = theme_pair.resolve_for_surface(
        Surface::Mode(color_mode),
        Some(theme_pair),
        CodeBlockMode::Same,
    );
    let highlighter = CodeHighlighter::from_theme(resolved.theme, resolved.color_mode);
    let syntax = LanguageGrammar::yaml()
        .resolve(highlighter.syntax_set())
        .unwrap_or_else(|_| highlighter.syntax_set().find_syntax_plain_text());
    let mut hl = HighlightLines::new(syntax, highlighter.theme());

    // `LinesWithEndings::from("")` yields nothing; emit one empty line in
    // that case so callers can rely on a 1:1 mapping for non-empty input
    // and a single empty entry for empty input.
    if yaml.is_empty() {
        return vec![String::new()];
    }

    let mut out: Vec<String> = Vec::new();
    for line in LinesWithEndings::from(yaml) {
        let line_no_newline = line.trim_end_matches('\n');
        let ranges = match hl.highlight_line(line, highlighter.syntax_set()) {
            Ok(r) => r,
            Err(_) => {
                // Fallback: unstyled line, no background fill.
                out.push(line_no_newline.to_string());
                continue;
            }
        };
        let mut buf = String::new();
        for (style, text) in &ranges {
            let text_no_newline = text.trim_end_matches('\n');
            if text_no_newline.is_empty() {
                continue;
            }
            let _ = write!(
                buf,
                "\x1b[38;2;{};{};{}m{}",
                style.foreground.r, style.foreground.g, style.foreground.b, text_no_newline
            );
        }
        // Reset SGR at end-of-line so styling does not bleed into anything
        // appended after the highlighted slice (e.g. trailing whitespace
        // padding the caller may add).
        buf.push_str("\x1b[0m");
        out.push(buf);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_highlighter_new() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        assert_eq!(highlighter.color_mode(), ColorMode::Dark);
    }

    #[test]
    fn test_code_highlighter_default() {
        let highlighter = CodeHighlighter::default();
        assert_eq!(highlighter.color_mode(), ColorMode::Dark);
    }

    #[test]
    fn test_syntax_set_available() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let syntax_set = highlighter.syntax_set();
        assert!(syntax_set.find_syntax_by_extension("rs").is_some());
    }

    #[test]
    fn test_theme_available() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let theme = highlighter.theme();
        assert!(theme.settings.background.is_some());
    }

    // -- highlight_yaml_lines --

    #[test]
    fn highlight_yaml_lines_returns_one_entry_per_input_line() {
        let yaml = "foo: 1\nbar: 2\nbaz: 3";
        let lines = highlight_yaml_lines_with_theme(yaml, ThemePair::Github, ColorMode::Dark);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn highlight_yaml_lines_emits_no_yaml_label() {
        let yaml = "foo: 1\nbar: 2";
        let lines = highlight_yaml_lines_with_theme(yaml, ThemePair::Github, ColorMode::Dark);
        let joined = lines.join("\n");
        // The chrome-free helper must never inject the literal `yaml`
        // language label that fenced code blocks use.
        assert!(
            !joined.contains("\nyaml\n"),
            "highlight_yaml_lines must not inject a 'yaml' header label, got {joined:?}"
        );
        // It also must not include the lone label as a standalone line.
        for line in &lines {
            assert_ne!(line.trim(), "yaml");
        }
    }

    #[test]
    fn highlight_yaml_lines_emits_no_background_fill() {
        let yaml = "foo: 1\nbar: 2";
        let lines = highlight_yaml_lines_with_theme(yaml, ThemePair::Github, ColorMode::Dark);
        // The chrome-free helper must not emit any `\x1b[48;` background SGR
        // sequence -- those are used only by the fenced code-block chrome
        // (top/bottom padding rows, full-width background fill).
        for line in &lines {
            assert!(
                !line.contains("\x1b[48;"),
                "highlight_yaml_lines must not emit a background-fill SGR, got {line:?}"
            );
        }
    }

    #[test]
    fn highlight_yaml_lines_handles_empty_input() {
        let lines = highlight_yaml_lines_with_theme("", ThemePair::Github, ColorMode::Dark);
        assert_eq!(lines, vec![String::new()]);
    }

    #[test]
    fn highlight_yaml_lines_emits_ansi_for_keys() {
        let yaml = "key: value";
        let lines = highlight_yaml_lines_with_theme(yaml, ThemePair::Github, ColorMode::Dark);
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].contains("\x1b["),
            "expected ANSI styling on highlighted YAML, got {:?}",
            lines[0]
        );
    }
}
