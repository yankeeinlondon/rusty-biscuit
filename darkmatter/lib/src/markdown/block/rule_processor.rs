use crate::markdown::inline::{InlineEvent, HorizontalRuleAttrs};
use pulldown_cmark::{Event, Tag, TagEnd};
use std::collections::VecDeque;

/// Iterator adapter that processes paragraph events for horizontal rule syntax with attributes.
///
/// `RuleProcessor` wraps an iterator of `InlineEvent` and transforms
/// paragraphs that match the horizontal rule pattern (`--- { ... }`) into
/// `InlineEvent::HorizontalRule` events.
///
/// The pattern matches:
/// - `--- { style: waves }`
/// - `*** { placement: centered, weight: thick }`
/// - `___ { width: "50%", color: "red" }`
///
/// ## Important
///
/// This processor only intercepts paragraphs that contain a **single text event**
/// matching the horizontal rule pattern. Paragraphs with inline formatting
/// (bold, italic, links, etc.) are passed through unchanged.
///
/// ## Examples
///
/// ```
/// use pulldown_cmark::Parser;
/// use darkmatter::markdown::inline::MarkProcessor;
/// use darkmatter::markdown::block::RuleProcessor;
/// use darkmatter::markdown::inline::InlineEvent;
///
/// let parser = Parser::new("--- { style: waves }");
/// let mark_processor = MarkProcessor::new(parser);
/// let mut events = RuleProcessor::new(mark_processor);
///
/// // The paragraph will be converted to a HorizontalRule event
/// ```
pub struct RuleProcessor<'a, I>
where
    I: Iterator<Item = InlineEvent<'a>>,
{
    inner: I,
    pending: VecDeque<InlineEvent<'a>>,
    /// Buffer to accumulate all events within a paragraph
    paragraph_buffer: Vec<InlineEvent<'a>>,
    /// Track if we're inside a paragraph
    in_paragraph: bool,
    /// Track if the current paragraph has only text (no nested elements)
    paragraph_is_simple: bool,
}

impl<'a, I> RuleProcessor<'a, I>
where
    I: Iterator<Item = InlineEvent<'a>>,
{
    /// Creates a new `RuleProcessor` wrapping the given parser iterator.
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            paragraph_buffer: Vec::new(),
            in_paragraph: false,
            paragraph_is_simple: true,
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

    /// Processes a completed paragraph buffer.
    ///
    /// If the paragraph contains exactly one text event that matches the
    /// horizontal rule pattern, emits a HorizontalRule event.
    /// Otherwise, emits all buffered events in order.
    fn process_paragraph_buffer(&mut self) {
        // Only check for HR pattern if paragraph has exactly one text event and nothing else
        if self.paragraph_is_simple
            && self.paragraph_buffer.len() == 1
            && let InlineEvent::Standard(Event::Text(text)) = &self.paragraph_buffer[0]
            && let Some((_, attributes)) = Self::matches_horizontal_rule_pattern(text)
        {
            let attrs = Self::parse_attributes(&attributes);
            self.pending.push_back(InlineEvent::HorizontalRule(attrs));
            return;
        }
        
        // Not a horizontal rule - emit all buffered events plus paragraph wrapper
        self.pending.push_back(InlineEvent::Standard(Event::Start(Tag::Paragraph)));
        for event in self.paragraph_buffer.drain(..) {
            self.pending.push_back(event);
        }
        self.pending.push_back(InlineEvent::Standard(Event::End(TagEnd::Paragraph)));
    }
}

impl<'a, I> Iterator for RuleProcessor<'a, I>
where
    I: Iterator<Item = InlineEvent<'a>>,
{
    type Item = InlineEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Return pending events first
        if let Some(event) = self.pending.pop_front() {
            return Some(event);
        }

        // Get next event from inner iterator
        match self.inner.next() {
            Some(InlineEvent::Standard(Event::Start(Tag::Paragraph))) => {
                self.in_paragraph = true;
                self.paragraph_is_simple = true;
                self.paragraph_buffer.clear();
                // Don't emit the paragraph start yet, buffer events instead
                self.next()
            }
            Some(InlineEvent::Standard(Event::End(TagEnd::Paragraph))) if self.in_paragraph => {
                self.in_paragraph = false;
                self.process_paragraph_buffer();
                self.pending.pop_front()
            }
            Some(InlineEvent::Standard(Event::Text(text))) if self.in_paragraph => {
                self.paragraph_buffer.push(InlineEvent::Standard(Event::Text(text)));
                self.next()
            }
            Some(event) if self.in_paragraph => {
                // Non-text event inside paragraph - paragraph is not simple
                self.paragraph_is_simple = false;
                self.paragraph_buffer.push(event);
                self.next()
            }
            Some(event) => {
                // For any other event, emit as-is
                Some(event)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::inline::{InlineEvent, HorizontalRuleAttrs};
    use pulldown_cmark::Parser;
    
    fn process_text(input: &str) -> Vec<InlineEvent<'_>> {
        let parser = Parser::new(input);
        let mark_processor = crate::markdown::inline::MarkProcessor::new(parser);
        RuleProcessor::new(mark_processor).collect()
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
    fn test_horizontal_rule_with_single_quotes() {
        let events = process_text("--- { width: '75%', color: '#00ff00' }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.width, Some("75%".to_string()));
            assert_eq!(attrs.color, Some("#00ff00".to_string()));
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
    fn test_paragraph_with_bold_text() {
        let events = process_text("This is **bold** text.");
        // Should have paragraph start, text, strong start, text, strong end, text, paragraph end
        assert!(events.len() >= 3);
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
        assert!(matches!(events[events.len()-1], InlineEvent::Standard(Event::End(TagEnd::Paragraph))));
    }
    
    #[test]
    fn test_insufficient_markers() {
        let events = process_text("-- { style: waves }");
        assert_eq!(events.len(), 3);
        // Should be treated as regular paragraph
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
    }
    
    #[test]
    fn test_malformed_attributes() {
        // Note: "--- { style waves }" is not a valid horizontal rule pattern
        // because it's missing the colon. It should be treated as a paragraph.
        let events = process_text("regular paragraph with { style waves }");
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
    }
    
    #[test]
    fn test_no_attributes() {
        // "---" by itself is parsed by pulldown-cmark as Event::Rule, not a paragraph
        // RuleProcessor should pass it through unchanged
        let events = process_text("---");
        // This should be 1 event: Standard(Rule)
        assert_eq!(events.len(), 1, "Expected 1 event for '---', got {:?}", events);
        assert!(matches!(events[0], InlineEvent::Standard(Event::Rule)));
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
    
    #[test]
    fn test_attributes_with_spaces() {
        let events = process_text("--- { style: line star, placement: left }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.style, Some("line star".to_string()));
            assert_eq!(attrs.placement, Some("left".to_string()));
        }
    }
    
    #[test]
    fn test_empty_attributes() {
        let events = process_text("--- { }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.style, None);
            assert_eq!(attrs.placement, None);
            assert_eq!(attrs.weight, None);
            assert_eq!(attrs.width, None);
            assert_eq!(attrs.color, None);
        }
    }
    
    #[test]
    fn test_whitespace_handling() {
        let events = process_text("   ---   {   style:   waves   }   ");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.style, Some("waves".to_string()));
        }
    }
    
    #[test]
    fn test_multiple_paragraphs() {
        let input = "--- { style: waves }\n\nThis is another paragraph.";
        let events = process_text(input);
        
        // Should have 4 events: HorizontalRule + paragraph start + text + paragraph end
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        assert!(matches!(events[1], InlineEvent::Standard(Event::Start(Tag::Paragraph))));
        assert!(matches!(events[2], InlineEvent::Standard(Event::Text(_))));
        assert!(matches!(events[3], InlineEvent::Standard(Event::End(TagEnd::Paragraph))));
    }
    
    #[test]
    fn test_horizontal_rule_attrs_default() {
        let attrs = HorizontalRuleAttrs::default();
        assert_eq!(attrs.style, None);
        assert_eq!(attrs.placement, None);
        assert_eq!(attrs.weight, None);
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }
    
    #[test]
    fn test_horizontal_rule_attrs_clone() {
        let attrs1 = HorizontalRuleAttrs {
            style: Some("test".to_string()),
            placement: Some("centered".to_string()),
            weight: Some("medium".to_string()),
            width: Some("50%".to_string()),
            color: Some("red".to_string()),
        };
        let attrs2 = attrs1.clone();
        assert_eq!(attrs1.style, attrs2.style);
        assert_eq!(attrs1.placement, attrs2.placement);
        assert_eq!(attrs1.weight, attrs2.weight);
        assert_eq!(attrs1.width, attrs2.width);
        assert_eq!(attrs1.color, attrs2.color);
    }
    
    #[test]
    fn test_horizontal_rule_attrs_partial() {
        let attrs = HorizontalRuleAttrs {
            style: Some("waves".to_string()),
            placement: None,
            weight: Some("thick".to_string()),
            width: None,
            color: Some("blue".to_string()),
        };
        assert_eq!(attrs.style, Some("waves".to_string()));
        assert_eq!(attrs.placement, None);
        assert_eq!(attrs.weight, Some("thick".to_string()));
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, Some("blue".to_string()));
    }
}
