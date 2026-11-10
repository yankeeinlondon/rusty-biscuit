//! The bridge between a detected [`Terminal`] and the target-neutral
//! [`TerminalProfile`] every output path reads.

use renderable::{OutputOptions, TerminalProfile};

use super::OutputTarget;
use crate::discovery::detection::{GraphicsRoute, ImageSupport, UnderlineSupport};
use crate::discovery::osc_queries::types::RgbValue;
use crate::terminal::Terminal;

impl Terminal {
    /// The capabilities of this terminal as an output profile.
    ///
    /// This only copies what detection found; whether a `terminal` request
    /// degrades to `plain` is decided by [`terminal_target_options`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// let profile = Terminal::new_optimistic(80).output_profile();
    /// assert!(profile.hyperlinks);
    /// assert!(!profile.degrades_to_plain());
    /// ```
    #[must_use]
    pub fn output_profile(&self) -> TerminalProfile {
        let underline = &self.underline_support;
        TerminalProfile {
            color_depth: self.color_depth.into(),
            color_mode: self.color_mode.into(),
            text_color: self.text_color.map(|c| (c.r, c.g, c.b)),
            background_color: self.background_color.map(|c| (c.r, c.g, c.b)),
            is_tty: self.is_tty,
            hyperlinks: self.osc_link_support,
            italics: self.supports_italic,
            underline: underline.straight
                || underline.double
                || underline.curly
                || underline.dotted
                || underline.dashed,
            graphics: self.image_support() != ImageSupport::None,
            unicode: self.supports_unicode,
        }
    }

    /// A terminal holding exactly what `options` states, for a component that
    /// renders to a [`Terminal`] rather than through the render tree.
    ///
    /// Nothing is detected, so the result depends only on `options`, and a
    /// `plain` profile cannot pick up a capability from the host. The width
    /// keeps its state: under [`Width::Unlimited`](renderable::Width::Unlimited)
    /// and [`Width::Source`](renderable::Width::Source) the terminal has no
    /// [`columns`](Terminal::columns), so its components neither fill nor wrap.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::discovery::detection::ColorDepth;
    /// use biscuit_terminal::terminal::Terminal;
    /// use renderable::{OutputOptions, TerminalProfile, Width};
    ///
    /// let options = OutputOptions::new(Width::Columns(40), TerminalProfile::plain());
    /// let terminal = Terminal::from_output_options(&options);
    /// assert_eq!(terminal.width(), 40);
    /// assert_eq!(terminal.color_depth, ColorDepth::None);
    /// assert!(!terminal.osc_link_support);
    /// ```
    #[must_use]
    pub fn from_output_options(options: &OutputOptions) -> Terminal {
        let profile = &options.terminal_profile;
        let rgb = |(r, g, b): (u8, u8, u8)| RgbValue { r, g, b };
        let mut terminal = Terminal::new_optimistic(0);
        terminal.set_output_width(options.width);
        terminal.color_depth = profile.color_depth.into();
        terminal.color_mode = profile.color_mode.into();
        terminal.text_color = profile.text_color.map(rgb);
        terminal.background_color = profile.background_color.map(rgb);
        terminal.is_tty = profile.is_tty;
        terminal.osc_link_support = profile.hyperlinks;
        terminal.supports_italic = profile.italics;
        terminal.supports_unicode = profile.unicode;
        if !profile.underline {
            terminal.underline_support = UnderlineSupport {
                straight: false,
                double: false,
                curly: false,
                dotted: false,
                dashed: false,
                colored: false,
            };
        }
        if !profile.graphics {
            terminal.graphics_route = GraphicsRoute::from(ImageSupport::None);
        }
        terminal
    }
}

/// The options the `terminal` or `plain` path renders with.
///
/// `plain` always gets [`TerminalProfile::plain`]; `terminal` gets it too
/// when its profile is not a color terminal. The substitution happens after
/// every other option was chosen, so nothing in `options` can restore a
/// capability it removes. Any other target returns `options` unchanged.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::output::{OutputTarget, terminal_target_options};
/// use biscuit_terminal::terminal::Terminal;
/// use renderable::{OutputOptions, TerminalProfile, Width};
///
/// let rich = OutputOptions::new(Width::Columns(80), Terminal::new_optimistic(80).output_profile());
/// let plain = terminal_target_options(OutputTarget::Plain, &rich);
/// assert_eq!(plain.terminal_profile, TerminalProfile::plain());
/// assert_eq!(terminal_target_options(OutputTarget::Terminal, &rich), rich);
/// ```
#[must_use]
pub fn terminal_target_options(target: OutputTarget, options: &OutputOptions) -> OutputOptions {
    let degrade = match target {
        OutputTarget::Plain => true,
        OutputTarget::Terminal => options.terminal_profile.degrades_to_plain(),
        _ => false,
    };
    if degrade {
        options.clone().into_plain()
    } else {
        options.clone()
    }
}

#[cfg(test)]
mod tests {
    use renderable::Width;
    use renderable::color::ColorDepth;

    use super::*;

    #[test]
    fn a_terminal_from_a_plain_profile_keeps_no_capability() {
        let options = OutputOptions::new(Width::Unlimited, TerminalProfile::plain());
        let terminal = Terminal::from_output_options(&options);
        assert_eq!(terminal.output_profile().color_depth, ColorDepth::None);
        assert!(!terminal.is_tty);
        assert!(!terminal.osc_link_support);
        assert!(!terminal.output_profile().underline);
        assert!(!terminal.output_profile().graphics);
    }

    #[test]
    fn a_terminal_round_trips_a_rich_profile() {
        let profile = Terminal::new_optimistic(80).output_profile();
        let options = OutputOptions::new(Width::Columns(33), profile.clone());
        let terminal = Terminal::from_output_options(&options);
        assert_eq!(terminal.width(), 33);
        assert_eq!(terminal.output_profile(), profile);
    }

    #[test]
    fn a_terminal_without_a_tty_degrades_to_plain() {
        let mut terminal = Terminal::new_optimistic(80);
        terminal.is_tty = false;
        let options = OutputOptions::new(Width::Unlimited, terminal.output_profile());
        let resolved = terminal_target_options(OutputTarget::Terminal, &options);
        assert_eq!(resolved.terminal_profile, TerminalProfile::plain());
        assert_eq!(resolved.width, Width::Unlimited);
    }

    #[test]
    fn a_tty_without_color_degrades_to_plain() {
        let mut profile = Terminal::new_optimistic(80).output_profile();
        profile.color_depth = ColorDepth::None;
        let options = OutputOptions::new(Width::Columns(10), profile);
        let resolved = terminal_target_options(OutputTarget::Terminal, &options);
        assert!(resolved.terminal_profile.is_plain());
    }

    #[test]
    fn non_terminal_targets_keep_their_options() {
        let options = OutputOptions::new(Width::Source, Terminal::new_optimistic(80).output_profile());
        for target in [OutputTarget::Markdown, OutputTarget::Html, OutputTarget::Json] {
            assert_eq!(terminal_target_options(target, &options), options);
        }
    }
}
