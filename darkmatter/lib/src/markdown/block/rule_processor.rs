use crate::markdown::inline::{InlineEvent, HorizontalRuleAttrs};
use pulldown_cmark::{Event, Tag, TagEnd};
use std::collections::VecDeque;
use std::str::FromStr;

/// Iterator adapter that processes paragraph events for horizontal rule syntax with attributes.
///
/// `RuleProcessor` wraps a pulldown-cmark parser iterator and transforms
/// paragraphs that match the horizontal rule pattern (`--- { ... }`) into
/// `InlineEvent::HorizontalRule` events.
///
/// The pattern matches:
/// - `--- { style: waves }`
/// - `*** { placement: centered, weight: thick }`
/// - `___ { width: "50%", color: "red" }`
///
/// ## Examples
///
/// ```
/// use pulldown_cmark::Parser;
/// use darkmatter::markdown::block::RuleProcessor;
/// use darkmatter::markdown::inline::InlineEvent;
///
/// let parser = Parser::new("--- { style: waves }");
/// let mut events = RuleProcessor::new(parser);
///
/// // The paragraph will be converted to a HorizontalRule event
/// ```
pub struct RuleProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    inner: I,
    pending: VecDeque<InlineEvent<'a>>,
    /// Buffer to accumulate text content within a paragraph
    paragraph_buffer: Option<String>,
    /// Track if we're inside a paragraph
    in_paragraph: bool,
}

impl<'a, I> RuleProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    /// Creates a new `RuleProcessor` wrapping the given parser iterator.
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            paragraph_buffer: None,
            in_paragraph: false,
        }
    }

    /// Checks if the text matches the horizontal rule pattern with attributes.
    ///
    /// Pattern: `^([\-\_\*]{3,})\s*\{(.*)\}\s*$`
    fn matches_horizontal_rule_pattern(text: &str) -> Option<(String, String)> {
        let trimmed = text.trim();
        if trimmed.len() < 3 {
            return None;
        }

        // Check if it starts with 3 or more of the same character: -, _, or *
        let first_char = trimmed.chars().next()?;
        if !['-', '_', '*'].contains(&first_char) {
            return None;
        }

        // Find the first non-matching character
        let mut marker_end = 0;
        for (i, ch) in trimmed.char_indices() {
            if ch != first_char {
                marker_end = i;
                break;
            }
        }
        
        // If we didn't break, the entire string is markers
        if marker_end == 0 {
            marker_end = trimmed.len();
        }

        // Must have at least 3 markers
        if marker_end < 3 {
            return None;
        }

        // Check if there's a { ... } block after the markers
        let after_markers = &trimmed[marker_end..].trim_start();
        if !after_markers.starts_with('{') || !after_markers.ends_with('}') {
            return None;
        }

        let attributes = after_markers[1..after_markers.len()-1].trim();
        let marker_str = trimmed[..marker_end].to_string();
        
        Some((marker_str, attributes.to_string()))
    }

    /// Parses attributes from the attribute string.
    ///
    /// The attribute string should be in the format: `style: waves, placement: centered`
    /// This is a simplified JSON-like parser that handles basic key-value pairs.
    fn parse_attributes(attribute_str: &str) -> HorizontalRuleAttrs {
        let mut attrs = HorizontalRuleAttrs::default();
        
        if attribute_str.is_empty() {
            return attrs;
        }

        // Split by commas, but be careful about nested structures (not supported for now)
        let pairs: Vec<&str> = attribute_str.split(',').collect();
        
        for pair in pairs {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            
            // Split by colon
            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            
            let key = parts[0].trim();
            let value = parts[1].trim();
            
            // Remove quotes if present
            let clean_value = if value.len() >= 2 && 
                ((value.starts_with('"') && value.ends_with('"')) ||
                 (value.starts_with('\'') && value.ends_with('\''))) {
                value[1..value.len()-1].to_string()
            } else {
                value.to_string()
            };
            
            match key {
                "style" => attrs.style = Some(clean_value),
                "placement" => attrs.placement = Some(clean_value),
                "weight" => attrs.weight = Some(clean_value),
                "width" => attrs.width = Some(clean_value),
                "color" => attrs.color = Some(clean_value),
                _ => {} // Ignore unknown attributes
            }
        }
        
        attrs
    }

    /// Processes a paragraph that might be a horizontal rule with attributes.
    fn process_paragraph(&mut self, text: String) {
        if let Some((_, attributes)) = Self::matches_horizontal_rule_pattern(&text) {
            let attrs = Self::parse_attributes(&attributes);
            self.pending.push_back(InlineEvent::HorizontalRule(attrs));
        } else {
            // Not a horizontal rule pattern, emit the original paragraph events
            self.pending.push_back(InlineEvent::Standard(Event::Start(Tag::Paragraph)));
            self.pending.push_back(InlineEvent::Standard(Event::Text(text.into())));
            self.pending.push_back(InlineEvent::Standard(Event::End(TagEnd::Paragraph)));
        }
    }
}

impl<'a, I> Iterator for RuleProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = InlineEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Return pending events first
        if let Some(event) = self.pending.pop_front() {
            return Some(event);
        }

        // Get next event from inner iterator
        match self.inner.next() {
            Some(Event::Start(Tag::Paragraph)) => {
                self.in_paragraph = true;
                self.paragraph_buffer = Some(String::new());
                // Don't emit the paragraph start yet, wait to see if it's a horizontal rule
                self.next()
            }
            Some(Event::End(TagEnd::Paragraph)) if self.in_paragraph => {
                self.in_paragraph = false;
                if let Some(buffer) = self.paragraph_buffer.take() {
                    self.process_paragraph(buffer);
                    return self.pending.pop_front();
                }
                // Should not happen, but if it does, emit the end event
                Some(InlineEvent::Standard(Event::End(TagEnd::Paragraph)))
            }
            Some(Event::Text(text)) if self.in_paragraph => {
                if let Some(buffer) = &mut self.paragraph_buffer {
                    buffer.push_str(&text);
                }
                self.next()
            }
            Some(event) => {
                // For any other event, emit it as standard
                Some(InlineEvent::Standard(event))
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::Parser;

    fn process_text(input: &str) -> Vec<InlineEvent<'_>> {
        let parser = Parser::new(input);
        RuleProcessor::new(parser).collect()
    }

    #[test]
    fn test_simple_horizontal_rule() {
        let events = process_text("--- { style: waves }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.style, Some("waves".to_string()));
            assert_eq!(attrs.placement, None);
            assert_eq!(attrs.weight, None);
            assert_eq!(attrs.width, None);
            assert_eq!(attrs.color, None);
        }
    }

    #[test]
    fn test_horizontal_rule_with_multiple_attributes() {
        let events = process_text("--- { style: dots, placement: centered, weight: thick }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.style, Some("dots".to_string()));
            assert_eq!(attrs.placement, Some("centered".to_string()));
            assert_eq!(attrs.weight, Some("thick".to_string()));
            assert_eq!(attrs.width, None);
            assert_eq!(attrs.color, None);
        }
    }

    #[test]
    fn test_horizontal_rule_with_quoted_values() {
        let events = process_text("--- { width: \"50%\", color: \"#ff0000\" }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.width, Some("50%".to_string()));
            assert_eq!(attrs.color, Some("#ff0000".to_string()));
        }
    }

    #[test]
    fn test_regular_paragraph_not_affected() {
        let events = process_text("This is a regular paragraph.");
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
        assert!(matches!(events[1], InlineEvent::Standard(Event::Text(_))));
        assert!(matches!(events[2], InlineEvent::Standard(Event::End(TagEnd::Paragraph))));
    }

    #[test]
    fn test_insufficient_markers() {
        let events = process_text("-- { style: waves }");
        assert_eq!(events.len(), 3);
        // Should be treated as regular paragraph
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
    }

    #[test]
    fn test_invalid_marker_character() {
        let events = process_text("=== { style: waves }");
        assert_eq!(events.len(), 3);
        // Should be treated as regular paragraph
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
    }

    #[test]
    fn test_no_attributes() {
        let events = process_text("---");
        assert_eq!(events.len(), 3);
        // Should be treated as regular paragraph (no { } block)
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
    }

    #[test]
    fn test_malformed_attributes() {
        let events = process_text("--- { style waves }");
        assert_eq!(events.len(), 3);
        // Should be treated as regular paragraph (malformed attributes)
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
    }

    #[test]
    fn test_different_marker_types() {
        let events1 = process_text("*** { style: dashes }");
        let events2 = process_text("___ { style: dots }");
        
        assert_eq!(events1.len(), 1);
        assert_eq!(events2.len(), 1);
        
        assert!(matches!(events1[0], InlineEvent::HorizontalRule(_)));
        assert!(matches!(events2[0], InlineEvent::HorizontalRule(_)));
    }
}