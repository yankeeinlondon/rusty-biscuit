//! Character representations for the curated subset of icons.

/// A character representation of an icon: an optional plain Unicode codepoint
/// and an optional Nerd Font private-use codepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    /// A standard Unicode character, if a faithful one exists.
    pub unicode: Option<char>,
    /// A Nerd Font (private use area) character, if mapped.
    pub nerd_font: Option<char>,
}

impl Glyph {
    /// A glyph with only a Unicode character.
    #[must_use]
    pub const fn unicode(c: char) -> Self {
        Self { unicode: Some(c), nerd_font: None }
    }

    /// A glyph with only a Nerd Font character.
    #[must_use]
    pub const fn nerd(c: char) -> Self {
        Self { unicode: None, nerd_font: Some(c) }
    }

    /// A glyph with both representations.
    #[must_use]
    pub const fn both(unicode: char, nerd_font: char) -> Self {
        Self { unicode: Some(unicode), nerd_font: Some(nerd_font) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sets_each_representation() {
        let g = Glyph::both('\u{1F600}', '\u{f118}');
        assert_eq!(g.unicode, Some('\u{1F600}'));
        assert_eq!(g.nerd_font, Some('\u{f118}'));
    }
}
