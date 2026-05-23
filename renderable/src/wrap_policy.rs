use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordWrap {
    /// **WrapProse**`(StartLooking, HangingIndent)`
    ///
    /// Will attempt to wrap words on wrap characters (e.g., whitespace,
    /// `-`, etc.) but if unable to find a break character in the text
    /// body then the text will be hyphenated at a hard break point.
    ///
    /// When the word wrapping logic is engaged _start_ looking for a good
    /// place to break the line a certain number of characters before
    /// max-width is reached.
    ///
    /// By default (e.g., when wrap gets `None`) we start looking for a line break
    /// 8 characters before the reaching the end of the line but you can
    /// override that with whatever you want.
    ///
    /// When a value is provided in the second parameter we use that to
    /// indent lines after the first by that many spaces.
    WrapProse(Option<u32>, Option<u32>),

    /// **BespokeProse**`(StartLooking, BreakChars, HangingIndent)`
    ///
    /// If you want to explicitly state the valid "break characters" which
    /// you hope to break on you can use this over the `WrapProse` option.
    ///
    /// When a value is provided in the third parameter we use that to
    /// indent lines after the first by that many spaces.
    BespokeProse(Option<u32>, Vec<char>, Option<u32>),

    /// Instead of "wrapping", we will truncate any content that moves
    /// beyond the end of the line. You can specify a string -- often an
    /// ellipsis -- which should be added to the end to give a visual queue
    /// that content has been truncated.
    ///
    /// The last "real" character on this line will be a character which
    /// allows the addition of this optional string to be presented to
    /// the terminal without extending beyond the renderable window.
    Truncate(Option<String>),
    /// no word wrap, when the end of line (e.g., max-width) is reached
    /// a new line is started but without any `-` or other markings to
    /// indicate a "continuation" and no attempt is made to break at a
    /// clean break character.
    ///
    /// This is also a hint to components like Table to try other columns
    /// before those marked as NOT having word wrap.
    None,
}

impl Default for WordWrap {
    fn default() -> Self {
        WordWrap::WrapProse(Some(8), None)
    }
}

impl WordWrap {
    /// Set the hanging indent, replacing any previously configured value.
    ///
    /// Continuation lines after a word wrap will be indented by `indent`
    /// spaces. `Truncate` and `None` variants are upgraded to
    /// `WrapProse` with the default start-looking window.
    pub fn with_hanging_indent(self, indent: u32) -> Self {
        match self {
            WordWrap::None => WordWrap::WrapProse(Some(8), Some(indent)),
            WordWrap::WrapProse(offset, _) => WordWrap::WrapProse(offset, Some(indent)),
            WordWrap::BespokeProse(offset, chars, _) => {
                WordWrap::BespokeProse(offset, chars, Some(indent))
            }
            WordWrap::Truncate(_) => self,
        }
    }

    /// Set the hanging indent only if one is not already configured.
    ///
    /// Like [`with_hanging_indent`](Self::with_hanging_indent) but
    /// respects an existing explicit value. `None` is upgraded to
    /// `WrapProse` with the default start-looking window.
    pub fn with_hanging_indent_if_none(self, indent: u32) -> Self {
        match self {
            WordWrap::None => WordWrap::WrapProse(Some(8), Some(indent)),
            WordWrap::WrapProse(offset, None) => WordWrap::WrapProse(offset, Some(indent)),
            WordWrap::BespokeProse(offset, chars, None) => {
                WordWrap::BespokeProse(offset, chars, Some(indent))
            }
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_hanging_indent_on_none() {
        let wrap = WordWrap::None.with_hanging_indent(4);
        assert_eq!(wrap, WordWrap::WrapProse(Some(8), Some(4)));
    }

    #[test]
    fn test_with_hanging_indent_on_wrap_prose() {
        let wrap = WordWrap::WrapProse(Some(5), Some(2)).with_hanging_indent(6);
        assert_eq!(wrap, WordWrap::WrapProse(Some(5), Some(6)));
    }

    #[test]
    fn test_with_hanging_indent_on_bespoke_prose() {
        let wrap = WordWrap::BespokeProse(Some(10), vec![' ', ','], Some(2)).with_hanging_indent(6);
        assert_eq!(
            wrap,
            WordWrap::BespokeProse(Some(10), vec![' ', ','], Some(6))
        );
    }

    #[test]
    fn test_with_hanging_indent_on_truncate_is_noop() {
        let wrap = WordWrap::Truncate(Some("...".into())).with_hanging_indent(4);
        assert_eq!(wrap, WordWrap::Truncate(Some("...".into())));
    }

    #[test]
    fn test_with_hanging_indent_if_none_sets_when_missing() {
        let wrap = WordWrap::WrapProse(Some(5), None).with_hanging_indent_if_none(4);
        assert_eq!(wrap, WordWrap::WrapProse(Some(5), Some(4)));
    }

    #[test]
    fn test_with_hanging_indent_if_none_preserves_existing() {
        let wrap = WordWrap::WrapProse(Some(5), Some(2)).with_hanging_indent_if_none(4);
        assert_eq!(wrap, WordWrap::WrapProse(Some(5), Some(2)));
    }

    #[test]
    fn test_with_hanging_indent_if_none_on_bespoke() {
        let wrap = WordWrap::BespokeProse(Some(10), vec![' '], None).with_hanging_indent_if_none(3);
        assert_eq!(wrap, WordWrap::BespokeProse(Some(10), vec![' '], Some(3)));
    }

    #[test]
    fn test_with_hanging_indent_if_none_bespoke_preserves_existing() {
        let wrap =
            WordWrap::BespokeProse(Some(10), vec![' '], Some(2)).with_hanging_indent_if_none(4);
        assert_eq!(wrap, WordWrap::BespokeProse(Some(10), vec![' '], Some(2)));
    }
}
