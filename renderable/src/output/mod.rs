//! The shared command-output contract.
//!
//! A command result exposes two sources: [`CommandOutput::as_prose`], a
//! render-tree [`Document`] behind the **formatted** family (`terminal`,
//! `plain`, `markdown`, `markdown-plus`, `html`), and
//! [`CommandOutput::as_data`], a serializable value behind the **data** family
//! (`json`, `json5`, `yaml`, `toml`).
//!
//! This crate only defines the contract. The dispatch that folds a value to a
//! chosen target (`write_output`) and the target enum live in
//! `biscuit-terminal`, because the terminal fold lives there and that crate
//! depends on this one, not the reverse.
//!
//! [`Width`] in this module is the *output* width a command was asked to
//! render at. It is unrelated to [`crate::layout::Width`], the component
//! sizing policy; files that use both import this one as `Width` and the
//! layout one as `LayoutWidth`.

pub mod link_metadata;

use serde::{Deserialize, Serialize};

use crate::color::{ColorDepth, ColorMode};
use crate::tree::Document;

/// A command result that can be printed as any output target.
///
/// The four `render_*` methods are optional per-target overrides. `None`, which
/// every default returns, means "use the generic fold of [`as_prose`]"; the
/// dispatch performs that fallback. There is no `plain` override: `plain` is
/// the `terminal` path given [`TerminalProfile::plain`], so an override of
/// [`render_terminal`] must honor the profile in its options.
///
/// Fallible preparation (loading, composing, theme resolution) happens before
/// a value implementing this trait is built, so an override never has an
/// error to swallow.
///
/// [`as_prose`]: CommandOutput::as_prose
/// [`render_terminal`]: CommandOutput::render_terminal
pub trait CommandOutput {
    /// The serializable form behind json, json5, yaml, and toml.
    type Data: Serialize;

    /// The human-readable form behind the formatted family.
    fn as_prose(&self) -> Document;

    /// The serializable form behind the data family.
    fn as_data(&self) -> Self::Data;

    /// Overrides the `terminal` (and therefore `plain`) output.
    fn render_terminal(&self, _options: &OutputOptions) -> Option<String> {
        None
    }

    /// Overrides the `markdown` output.
    fn render_markdown(&self, _options: &OutputOptions) -> Option<String> {
        None
    }

    /// Overrides the `markdown-plus` output.
    fn render_markdown_plus(&self, _options: &OutputOptions) -> Option<String> {
        None
    }

    /// Overrides the `html` output.
    fn render_html(&self, _options: &OutputOptions) -> Option<String> {
        None
    }
}

/// The options every output path receives.
///
/// `terminal_profile` is the detected (or spoofed) terminal for `terminal`,
/// and always [`TerminalProfile::plain`] for `plain`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputOptions {
    /// The global `-v` count. Above zero, a link or image that cannot be
    /// clickable also prints its destination.
    pub verbosity: u8,
    /// The resolved output width.
    pub width: Width,
    /// The capabilities the formatted output may use.
    pub terminal_profile: TerminalProfile,
}

impl OutputOptions {
    /// Options for the given width and profile at verbosity zero.
    #[must_use]
    pub fn new(width: Width, terminal_profile: TerminalProfile) -> Self {
        Self {
            verbosity: 0,
            width,
            terminal_profile,
        }
    }

    /// Sets the verbosity.
    #[must_use]
    pub fn with_verbosity(mut self, verbosity: u8) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Returns these options with the plain profile substituted.
    ///
    /// This is how `plain` (and a `terminal` run that degrades) is produced:
    /// the substitution happens after every other option was chosen, so no
    /// option can restore a capability the plain profile removes.
    #[must_use]
    pub fn into_plain(mut self) -> Self {
        self.terminal_profile = TerminalProfile::plain();
        self
    }
}

/// The resolved output width.
///
/// Width is never a large sentinel number: a fold that fills to the width has
/// to say what it does when there is no width.
///
/// | Variant | Prose | Fills (rules, bands, right margins) |
/// |---|---|---|
/// | `Columns(n)` | soft breaks joined, wrapped at `n` | filled to `n` |
/// | `Unlimited` | soft breaks joined, never wrapped | natural width |
/// | `Source` | soft breaks kept as authored | natural width |
///
/// `Columns(0)` is never constructed by this workspace: width parsing rejects
/// a zero before building the variant, and [`Width::columns`] returns `None`
/// for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Width {
    /// Wrap and fill at this many columns (never zero).
    Columns(u32),
    /// Join soft breaks; never wrap and never fill.
    Unlimited,
    /// Keep authored line breaks; never wrap and never fill.
    Source,
}

impl Width {
    /// `Columns(n)`, or `None` when `n` is zero.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::output::Width;
    ///
    /// assert_eq!(Width::columns(72), Some(Width::Columns(72)));
    /// assert_eq!(Width::columns(0), None);
    /// ```
    #[must_use]
    pub const fn columns(n: u32) -> Option<Self> {
        if n == 0 { None } else { Some(Self::Columns(n)) }
    }

    /// The column count, when this width has one.
    #[must_use]
    pub const fn as_columns(self) -> Option<u32> {
        match self {
            Self::Columns(n) => Some(n),
            Self::Unlimited | Self::Source => None,
        }
    }

    /// Whether this width has a column count to wrap or fill to.
    #[must_use]
    pub const fn is_bounded(self) -> bool {
        matches!(self, Self::Columns(_))
    }

    /// The width left after `delta` columns are consumed (indentation, a
    /// gutter, a border). A bounded width never drops below one column; an
    /// unbounded one is unchanged.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::output::Width;
    ///
    /// assert_eq!(Width::Columns(10).saturating_sub(4), Width::Columns(6));
    /// assert_eq!(Width::Columns(3).saturating_sub(9), Width::Columns(1));
    /// assert_eq!(Width::Unlimited.saturating_sub(9), Width::Unlimited);
    /// ```
    #[must_use]
    pub const fn saturating_sub(self, delta: u32) -> Self {
        match self {
            Self::Columns(n) => {
                let left = n.saturating_sub(delta);
                Self::Columns(if left == 0 { 1 } else { left })
            }
            other => other,
        }
    }

    /// Caps a bounded width at `cap` columns; an unbounded width becomes
    /// `Columns(cap)`, because a cap (such as a component's `max-width`)
    /// still applies when nothing else bounds the output. A zero cap leaves
    /// the width unchanged.
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::output::Width;
    ///
    /// assert_eq!(Width::Columns(100).capped_at(60), Width::Columns(60));
    /// assert_eq!(Width::Columns(40).capped_at(60), Width::Columns(40));
    /// assert_eq!(Width::Unlimited.capped_at(60), Width::Columns(60));
    /// ```
    #[must_use]
    pub const fn capped_at(self, cap: u32) -> Self {
        if cap == 0 {
            return self;
        }
        match self {
            Self::Columns(n) if n <= cap => self,
            _ => Self::Columns(cap),
        }
    }
}

/// The capabilities a formatted output may use.
///
/// The color types are the target-neutral ones from [`crate::color`];
/// `biscuit-terminal` fills a profile from a detected terminal. Every output
/// path reads capabilities from here, never from the environment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerminalProfile {
    /// The color depth styling may use; [`ColorDepth::None`] emits no SGR.
    pub color_depth: ColorDepth,
    /// The background's light/dark mode.
    pub color_mode: ColorMode,
    /// The detected foreground color, when known.
    pub text_color: Option<(u8, u8, u8)>,
    /// The detected background color, when known.
    pub background_color: Option<(u8, u8, u8)>,
    /// Whether the output stream is a terminal.
    pub is_tty: bool,
    /// Whether OSC 8 hyperlinks may be written.
    pub hyperlinks: bool,
    /// Whether italic styling may be written.
    pub italics: bool,
    /// Whether underline styling may be written.
    pub underline: bool,
    /// Whether inline images may be written.
    pub graphics: bool,
    /// Whether Unicode glyphs (box drawing among them) may be written.
    pub unicode: bool,
}

impl TerminalProfile {
    /// The one built-in plain profile: every styling capability off, Unicode
    /// on (plain tables keep their box-drawing borders).
    ///
    /// ## Examples
    ///
    /// ```
    /// use renderable::output::TerminalProfile;
    ///
    /// let plain = TerminalProfile::plain();
    /// assert!(plain.is_plain());
    /// assert!(plain.unicode);
    /// ```
    #[must_use]
    pub const fn plain() -> Self {
        Self {
            color_depth: ColorDepth::None,
            color_mode: ColorMode::Unknown,
            text_color: None,
            background_color: None,
            is_tty: false,
            hyperlinks: false,
            italics: false,
            underline: false,
            graphics: false,
            unicode: true,
        }
    }

    /// Whether this profile writes no escape sequence at all.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        self.color_depth == ColorDepth::None
            && !self.hyperlinks
            && !self.italics
            && !self.underline
            && !self.graphics
    }

    /// Whether a `terminal` request on this profile degrades to `plain`: the
    /// stream is not a terminal, or the terminal has no color.
    #[must_use]
    pub fn degrades_to_plain(&self) -> bool {
        !self.is_tty || self.color_depth == ColorDepth::None
    }

    /// This profile with OSC 8 hyperlinks turned off (`HyperlinkMode::Never`).
    #[must_use]
    pub fn without_hyperlinks(mut self) -> Self {
        self.hyperlinks = false;
        self
    }
}

impl Default for TerminalProfile {
    /// The plain profile: a profile nobody detected must not style.
    fn default() -> Self {
        Self::plain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed;

    impl CommandOutput for Fixed {
        type Data = u32;

        fn as_prose(&self) -> Document {
            Document::new(crate::tree::RenderNode::root(Vec::new()))
        }

        fn as_data(&self) -> u32 {
            7
        }
    }

    #[test]
    fn overrides_default_to_none() {
        let options = OutputOptions::new(Width::Unlimited, TerminalProfile::plain());
        assert_eq!(Fixed.render_terminal(&options), None);
        assert_eq!(Fixed.render_markdown(&options), None);
        assert_eq!(Fixed.render_markdown_plus(&options), None);
        assert_eq!(Fixed.render_html(&options), None);
        assert_eq!(Fixed.as_data(), 7);
    }

    #[test]
    fn into_plain_replaces_a_rich_profile() {
        let rich = TerminalProfile {
            color_depth: ColorDepth::TrueColor,
            color_mode: ColorMode::Dark,
            text_color: Some((1, 2, 3)),
            background_color: None,
            is_tty: true,
            hyperlinks: true,
            italics: true,
            underline: true,
            graphics: true,
            unicode: true,
        };
        assert!(!rich.degrades_to_plain());
        let options = OutputOptions::new(Width::Columns(40), rich).with_verbosity(2);
        let plain = options.into_plain();
        assert_eq!(plain.terminal_profile, TerminalProfile::plain());
        assert_eq!(plain.width, Width::Columns(40));
        assert_eq!(plain.verbosity, 2);
    }

    #[test]
    fn a_profile_without_tty_or_color_degrades() {
        let mut profile = TerminalProfile::plain();
        assert!(profile.degrades_to_plain());
        profile.is_tty = true;
        assert!(profile.degrades_to_plain(), "no color still degrades");
        profile.color_depth = ColorDepth::Basic;
        assert!(!profile.degrades_to_plain());
        assert!(!profile.clone().without_hyperlinks().hyperlinks);
    }

    #[test]
    fn width_serializes_as_snake_case() {
        assert_eq!(serde_json::to_string(&Width::Unlimited).unwrap(), "\"unlimited\"");
        assert_eq!(serde_json::to_string(&Width::Source).unwrap(), "\"source\"");
        assert_eq!(serde_json::to_string(&Width::Columns(3)).unwrap(), "{\"columns\":3}");
    }
}
