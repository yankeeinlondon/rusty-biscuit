//! Type definitions for custom inline markdown extensions.
//!
//! This module defines the types needed to extend pulldown-cmark's event model
//! with custom inline elements that are not natively supported.

use pulldown_cmark::Event;

/// Custom inline tags not natively supported by pulldown-cmark.
///
/// These tags represent inline formatting elements that are commonly used
/// in extended Markdown flavors but not part of CommonMark or GFM.
///
/// ## Examples
///
/// ```
/// use shared::markdown::inline::InlineTag;
///
/// let tag = InlineTag::Mark;
/// assert_eq!(format!("{:?}", tag), "Mark");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlineTag {
    /// Highlighted/marked text (`==text==`).
    ///
    /// Renders as `<mark>` in HTML or with a yellow background in terminal.
    /// This syntax is supported by popular editors like Obsidian and Typora.
    Mark,
    // Future extensions:
    // Underline,    // __text__ or ++text++
    // Subscript,    // ~text~
    // Superscript,  // ^text^
}

/// Wrapper for pulldown-cmark events with custom inline extensions.
///
/// This enum wraps standard pulldown-cmark events while adding support
/// for custom inline tags. It allows the `MarkProcessor` iterator adapter
/// to emit both standard and custom events in a unified stream.
///
/// ## Examples
///
/// ```
/// use pulldown_cmark::Event;
/// use shared::markdown::inline::{InlineEvent, InlineTag};
///
/// // Standard events are wrapped
/// let text = Event::Text("Hello".into());
/// let event: InlineEvent = text.into();
/// assert!(matches!(event, InlineEvent::Standard(_)));
///
/// // Custom inline tags have dedicated variants
/// let start = InlineEvent::Start(InlineTag::Mark);
/// let end = InlineEvent::End(InlineTag::Mark);
/// ```
#[derive(Debug, Clone)]
pub enum InlineEvent<'a> {
    /// Standard pulldown-cmark event (passed through unchanged).
    Standard(Event<'a>),

    /// Start of a custom inline tag.
    Start(InlineTag),

    /// End of a custom inline tag.
    End(InlineTag),
}

impl<'a> From<Event<'a>> for InlineEvent<'a> {
    fn from(event: Event<'a>) -> Self {
        InlineEvent::Standard(event)
    }
}

impl InlineEvent<'_> {
    /// Returns `true` if this is a standard pulldown-cmark event.
    #[inline]
    pub fn is_standard(&self) -> bool {
        matches!(self, InlineEvent::Standard(_))
    }

    /// Returns `true` if this is a custom inline start event.
    #[inline]
    pub fn is_start(&self) -> bool {
        matches!(self, InlineEvent::Start(_))
    }

    /// Returns `true` if this is a custom inline end event.
    #[inline]
    pub fn is_end(&self) -> bool {
        matches!(self, InlineEvent::End(_))
    }

    /// Returns the inner standard event if this is a Standard variant.
    pub fn as_standard(&self) -> Option<&Event<'_>> {
        match self {
            InlineEvent::Standard(e) => Some(e),
            _ => None,
        }
    }

    /// Returns the inline tag if this is a Start or End variant.
    pub fn inline_tag(&self) -> Option<InlineTag> {
        match self {
            InlineEvent::Start(tag) | InlineEvent::End(tag) => Some(*tag),
            InlineEvent::Standard(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::CowStr;

    #[test]
    fn test_inline_tag_debug() {
        let tag = InlineTag::Mark;
        assert_eq!(format!("{:?}", tag), "Mark");
    }

    #[test]
    fn test_inline_tag_equality() {
        assert_eq!(InlineTag::Mark, InlineTag::Mark);
    }

    #[test]
    fn test_inline_tag_clone() {
        let tag = InlineTag::Mark;
        let cloned = tag;
        assert_eq!(tag, cloned);
    }

    #[test]
    fn test_inline_event_from_standard() {
        let text = Event::Text(CowStr::from("Hello"));
        let event: InlineEvent = text.into();
        assert!(event.is_standard());
        assert!(!event.is_start());
        assert!(!event.is_end());
    }

    #[test]
    fn test_inline_event_start() {
        let event = InlineEvent::Start(InlineTag::Mark);
        assert!(!event.is_standard());
        assert!(event.is_start());
        assert!(!event.is_end());
        assert_eq!(event.inline_tag(), Some(InlineTag::Mark));
    }

    #[test]
    fn test_inline_event_end() {
        let event = InlineEvent::End(InlineTag::Mark);
        assert!(!event.is_standard());
        assert!(!event.is_start());
        assert!(event.is_end());
        assert_eq!(event.inline_tag(), Some(InlineTag::Mark));
    }

    #[test]
    fn test_inline_event_as_standard() {
        let text = Event::Text(CowStr::from("Hello"));
        let event: InlineEvent = text.into();
        assert!(event.as_standard().is_some());

        let start = InlineEvent::Start(InlineTag::Mark);
        assert!(start.as_standard().is_none());
    }

    #[test]
    fn test_inline_event_inline_tag() {
        let text = Event::Text(CowStr::from("Hello"));
        let event: InlineEvent = text.into();
        assert_eq!(event.inline_tag(), None);

        let start = InlineEvent::Start(InlineTag::Mark);
        assert_eq!(start.inline_tag(), Some(InlineTag::Mark));

        let end = InlineEvent::End(InlineTag::Mark);
        assert_eq!(end.inline_tag(), Some(InlineTag::Mark));
    }
}
