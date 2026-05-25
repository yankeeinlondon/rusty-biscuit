use crate::markdown::inline::{HorizontalRuleAttrs, InlineEvent};
use crate::style::warning::{StyleWarning, StyleWarningKind};
use pulldown_cmark::{Event, Tag, TagEnd};
use std::collections::VecDeque;

/// Parses a darkmatter HR-attribute paragraph body — `---|***|___` followed by
/// an optional `{ ... }` attribute block — into [`HorizontalRuleAttrs`].
///
/// Returns `None` when `body` is not a recognized HR-attribute paragraph. The
/// span-aware fold uses this to detect and rewrite HR-attribute paragraphs
/// without re-implementing the matcher.
#[must_use]
pub fn try_parse_hr_attrs(body: &str) -> Option<HorizontalRuleAttrs> {
    // The matcher requires an attribute block (`{ ... }`); a bare `---`
    // already arrives from pulldown-cmark as `Event::Rule`, so this helper
    // intentionally returns `None` for it.
    let (_, attribute_str) = matches_horizontal_rule_pattern(body)?;
    Some(parse_attribute_block(&attribute_str))
}

/// Matches `body` against the HR-attribute paragraph pattern, returning
/// `(marker, attribute-string)` when the body is a single-line directive.
fn matches_horizontal_rule_pattern(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim();
    if trimmed.len() < 3 {
        return None;
    }
    let first_char = trimmed.chars().next()?;
    if !['-', '_', '*'].contains(&first_char) {
        return None;
    }
    let mut marker_end = 0;
    for (i, ch) in trimmed.char_indices() {
        if ch != first_char {
            marker_end = i;
            break;
        }
    }
    if marker_end == 0 {
        marker_end = trimmed.len();
    }
    if marker_end < 3 {
        return None;
    }
    let after_markers = trimmed[marker_end..].trim_start();
    if !after_markers.starts_with('{') || !after_markers.ends_with('}') {
        return None;
    }
    let attributes = after_markers[1..after_markers.len() - 1].trim();
    let marker_str = trimmed[..marker_end].to_string();
    Some((marker_str, attributes.to_string()))
}

/// Parses an HR attribute string (the content between `{` and `}`) into
/// [`HorizontalRuleAttrs`] using the same YAML-flow-mapping logic as the
/// in-place [`RuleProcessor::parse_attributes`] method.
fn parse_attribute_block(attribute_str: &str) -> HorizontalRuleAttrs {
    // This is a free-function clone of `RuleProcessor::parse_attributes` so
    // the span-aware fold can call it without instantiating the generic
    // processor.
    if attribute_str.trim().is_empty() {
        return HorizontalRuleAttrs::default();
    }

    let yaml_src = format!("{{ {attribute_str} }}");
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&yaml_src) {
        Ok(serde_yaml_ng::Value::Mapping(map)) => attrs_from_mapping(&map),
        _ => HorizontalRuleAttrs::default(),
    }
}

/// Builds [`HorizontalRuleAttrs`] from a YAML flow mapping, coercing scalars
/// to strings and dropping unknown keys.
fn attrs_from_mapping(map: &serde_yaml_ng::Mapping) -> HorizontalRuleAttrs {
    let mut attrs = HorizontalRuleAttrs::default();
    for (yaml_key, yaml_value) in map {
        let Some(key) = yaml_value_as_string(yaml_key) else {
            continue;
        };
        let Some(value) = yaml_value_as_string(yaml_value) else {
            continue;
        };
        match key.as_str() {
            "kind" => attrs.kind = Some(value),
            "style" => attrs.legacy_style = Some(value),
            "alignment" => attrs.alignment = Some(value),
            "weight" => attrs.weight = Some(value),
            "width" => attrs.width = Some(value),
            "color" => attrs.color = Some(value),
            _ => {}
        }
    }
    attrs
}

/// Coerces a YAML scalar to a string, returning `None` for non-scalar shapes.
fn yaml_value_as_string(value: &serde_yaml_ng::Value) -> Option<String> {
    match value {
        serde_yaml_ng::Value::String(s) => Some(s.clone()),
        serde_yaml_ng::Value::Number(n) => Some(n.to_string()),
        serde_yaml_ng::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Iterator adapter that processes paragraph events for horizontal rule syntax with attributes.
///
/// `RuleProcessor` wraps an iterator of `InlineEvent` and transforms
/// paragraphs that match the horizontal rule pattern (`--- { ... }`) into
/// `InlineEvent::HorizontalRule` events.
///
/// The pattern matches:
/// - `--- { style: waves }`
/// - `*** { alignment: centered, weight: thick }`
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
/// Scan markdown content for inline horizontal-rule deprecation warnings.
///
/// Runs the full inline event pipeline (pulldown-cmark → [`InlineStyleProcessor`]
/// → [`RuleProcessor`]) and returns any deprecation warnings emitted for legacy
/// inline `style` attributes (`--- { style: waves }`).
///
/// This is a preflight helper for `--strict-style`: callers can check the
/// returned warnings before rendering and promote them to errors when strict
/// mode is enabled.
pub fn scan_inline_hr_warnings(content: &str) -> Vec<StyleWarning> {
    let preprocessed = crate::markdown::inline::preprocess_escaped_markers(content);
    let parser = pulldown_cmark::Parser::new_ext(
        &preprocessed,
        pulldown_cmark::Options::ENABLE_TABLES | pulldown_cmark::Options::ENABLE_STRIKETHROUGH,
    );
    let mut processor = RuleProcessor::new(crate::markdown::inline::InlineStyleProcessor::new(
        parser,
    ));
    // Consume all events to populate warnings.
    for _ in &mut processor {}
    processor.warnings.into_iter().collect()
}

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
    /// Deprecation warnings collected while parsing inline HR attributes.
    warnings: Vec<StyleWarning>,
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
            warnings: Vec::new(),
        }
    }

    /// Returns any deprecation warnings collected while parsing.
    pub fn warnings(&self) -> &[StyleWarning] {
        &self.warnings
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

        let attributes = after_markers[1..after_markers.len() - 1].trim();
        let marker_str = trimmed[..marker_end].to_string();

        Some((marker_str, attributes.to_string()))
    }

    /// Parses attributes from the attribute string.
    ///
    /// The attribute string is the content between the `{` and `}` markers
    /// in a horizontal-rule directive such as `--- { style: waves }`. This
    /// content is parsed as a YAML flow mapping (e.g., `{ a: 1, b: "x, y" }`)
    /// using [`serde_yaml_ng`], which correctly handles quoted values with
    /// embedded commas or colons — capabilities the previous hand-rolled
    /// splitter lacked (see review item E2).
    ///
    /// ## Errors
    ///
    /// YAML parse errors (e.g., `{ style: }`) are non-fatal — the function
    /// emits a `tracing::warn!` and falls back to the legacy ad-hoc splitter
    /// so existing malformed-but-previously-accepted inputs keep working.
    ///
    /// ## Notes
    ///
    /// Unknown enum values (e.g., `style: dashse`) are captured verbatim on
    /// [`HorizontalRuleAttrs`] — the builder layer
    /// ([`build_rule`](crate::markdown::block::build_rule)) is responsible
    /// for emitting a warning and falling back to the default. Unknown
    /// **keys** are dropped here with a warning.
    fn parse_attributes(attribute_str: &str) -> HorizontalRuleAttrs {
        if attribute_str.trim().is_empty() {
            return HorizontalRuleAttrs::default();
        }

        // Wrap the captured inner content back into a YAML flow mapping so
        // `serde_yaml_ng` parses it as a single-level map.
        let yaml_src = format!("{{ {attribute_str} }}");

        match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&yaml_src) {
            Ok(serde_yaml_ng::Value::Mapping(map)) => Self::attrs_from_mapping(&map),
            Ok(other) => {
                // A bare scalar/sequence can happen when the user wrote
                // something like `--- { foo }`. Fall back to the legacy
                // splitter; it will tolerate many such shapes.
                tracing::warn!(
                    kind = ?std::mem::discriminant(&other),
                    "horizontal rule attributes did not parse as a YAML mapping; using legacy splitter"
                );
                Self::parse_attributes_legacy(attribute_str)
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    input = %attribute_str,
                    "malformed horizontal rule attributes; using legacy splitter"
                );
                Self::parse_attributes_legacy(attribute_str)
            }
        }
    }

    /// Converts a parsed YAML mapping into [`HorizontalRuleAttrs`], coercing
    /// each scalar value to a string and dropping unknown keys with a
    /// warning.
    fn attrs_from_mapping(map: &serde_yaml_ng::Mapping) -> HorizontalRuleAttrs {
        let mut attrs = HorizontalRuleAttrs::default();

        for (yaml_key, yaml_value) in map {
            // Keys must be scalar strings (or string-coercible scalars).
            let Some(key) = Self::yaml_value_as_string(yaml_key) else {
                tracing::warn!(
                    key = ?yaml_key,
                    "non-scalar key in horizontal rule attributes; ignoring"
                );
                continue;
            };

            let Some(value) = Self::yaml_value_as_string(yaml_value) else {
                tracing::warn!(
                    key = %key,
                    value = ?yaml_value,
                    "non-scalar value in horizontal rule attribute; ignoring"
                );
                continue;
            };

            match key.as_str() {
                "kind" => attrs.kind = Some(value),
                "style" => attrs.legacy_style = Some(value),
                "alignment" => attrs.alignment = Some(value),
                "weight" => attrs.weight = Some(value),
                "width" => attrs.width = Some(value),
                "color" => attrs.color = Some(value),
                other => {
                    tracing::warn!(
                        key = %other,
                        value = %value,
                        "unknown horizontal rule attribute; ignoring"
                    );
                }
            }
        }

        attrs
    }

    /// Coerces a YAML scalar to a Rust `String`, returning `None` for
    /// non-scalar shapes (sequences, mappings, null, etc.).
    fn yaml_value_as_string(value: &serde_yaml_ng::Value) -> Option<String> {
        match value {
            serde_yaml_ng::Value::String(s) => Some(s.clone()),
            serde_yaml_ng::Value::Number(n) => Some(n.to_string()),
            serde_yaml_ng::Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// Legacy ad-hoc splitter kept as a graceful fallback when the YAML
    /// parser rejects the input. Mirrors the previous behavior so
    /// malformed-but-previously-accepted attribute strings continue to
    /// parse instead of silently losing all fields.
    fn parse_attributes_legacy(attribute_str: &str) -> HorizontalRuleAttrs {
        let mut attrs = HorizontalRuleAttrs::default();

        if attribute_str.is_empty() {
            return attrs;
        }

        for pair in attribute_str.split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }

            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            let clean_value = if value.len() >= 2
                && ((value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\'')))
            {
                value[1..value.len() - 1].to_string()
            } else {
                value.to_string()
            };

            match key {
                "kind" => attrs.kind = Some(clean_value),
                "style" => attrs.legacy_style = Some(clean_value),
                "alignment" => attrs.alignment = Some(clean_value),
                "weight" => attrs.weight = Some(clean_value),
                "width" => attrs.width = Some(clean_value),
                "color" => attrs.color = Some(clean_value),
                other => {
                    tracing::warn!(
                        key = %other,
                        value = %clean_value,
                        "unknown horizontal rule attribute; ignoring"
                    );
                }
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
            if attrs.legacy_style.is_some() {
                self.warnings.push(StyleWarning::new(
                    "hr.inline.style",
                    StyleWarningKind::Deprecated {
                        replacement: "hr.inline.kind".into(),
                    },
                ));
            }
            self.pending.push_back(InlineEvent::HorizontalRule(attrs));
            return;
        }

        // Not a horizontal rule - emit all buffered events plus paragraph wrapper
        self.pending
            .push_back(InlineEvent::Standard(Event::Start(Tag::Paragraph)));
        for event in self.paragraph_buffer.drain(..) {
            self.pending.push_back(event);
        }
        self.pending
            .push_back(InlineEvent::Standard(Event::End(TagEnd::Paragraph)));
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
                self.paragraph_buffer
                    .push(InlineEvent::Standard(Event::Text(text)));
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
    use crate::markdown::inline::{HorizontalRuleAttrs, InlineEvent};
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
            assert_eq!(attrs.legacy_style, Some("waves".to_string()));
            assert_eq!(attrs.kind, None);
            assert_eq!(attrs.alignment, None);
            assert_eq!(attrs.weight, None);
            assert_eq!(attrs.width, None);
            assert_eq!(attrs.color, None);
        }
    }

    #[test]
    fn test_horizontal_rule_with_multiple_attributes() {
        let events = process_text("--- { style: dots, alignment: centered, weight: thick }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));

        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.legacy_style, Some("dots".to_string()));
            assert_eq!(attrs.kind, None);
            assert_eq!(attrs.alignment, Some("centered".to_string()));
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
        assert!(matches!(
            events[0],
            InlineEvent::Standard(Event::Start(Tag::Paragraph))
        ));
        assert!(matches!(events[1], InlineEvent::Standard(Event::Text(_))));
        assert!(matches!(
            events[2],
            InlineEvent::Standard(Event::End(TagEnd::Paragraph))
        ));
    }

    #[test]
    fn test_paragraph_with_bold_text() {
        let events = process_text("This is **bold** text.");
        // Should have paragraph start, text, strong start, text, strong end, text, paragraph end
        assert!(events.len() >= 3);
        assert!(matches!(
            events[0],
            InlineEvent::Standard(Event::Start(Tag::Paragraph))
        ));
        assert!(matches!(
            events[events.len() - 1],
            InlineEvent::Standard(Event::End(TagEnd::Paragraph))
        ));
    }

    #[test]
    fn test_insufficient_markers() {
        let events = process_text("-- { style: waves }");
        assert_eq!(events.len(), 3);
        // Should be treated as regular paragraph
        assert!(matches!(
            events[0],
            InlineEvent::Standard(Event::Start(Tag::Paragraph))
        ));
    }

    #[test]
    fn test_malformed_attributes() {
        // Note: "--- { style waves }" is not a valid horizontal rule pattern
        // because it's missing the colon. It should be treated as a paragraph.
        let events = process_text("regular paragraph with { style waves }");
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[0],
            InlineEvent::Standard(Event::Start(Tag::Paragraph))
        ));
    }

    #[test]
    fn test_no_attributes() {
        // "---" by itself is parsed by pulldown-cmark as Event::Rule, not a paragraph
        // RuleProcessor should pass it through unchanged
        let events = process_text("---");
        // This should be 1 event: Standard(Rule)
        assert_eq!(
            events.len(),
            1,
            "Expected 1 event for '---', got {:?}",
            events
        );
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
        let events = process_text("--- { style: line star, alignment: left }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));

        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.legacy_style, Some("line star".to_string()));
            assert_eq!(attrs.kind, None);
            assert_eq!(attrs.alignment, Some("left".to_string()));
        }
    }

    #[test]
    fn test_empty_attributes() {
        let events = process_text("--- { }");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));

        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.kind, None);
            assert_eq!(attrs.legacy_style, None);
            assert_eq!(attrs.alignment, None);
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
            assert_eq!(attrs.legacy_style, Some("waves".to_string()));
            assert_eq!(attrs.kind, None);
        }
    }

    #[test]
    fn test_multiple_paragraphs() {
        let input = "--- { style: waves }\n\nThis is another paragraph.";
        let events = process_text(input);

        // Should have 4 events: HorizontalRule + paragraph start + text + paragraph end
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        assert!(matches!(
            events[1],
            InlineEvent::Standard(Event::Start(Tag::Paragraph))
        ));
        assert!(matches!(events[2], InlineEvent::Standard(Event::Text(_))));
        assert!(matches!(
            events[3],
            InlineEvent::Standard(Event::End(TagEnd::Paragraph))
        ));
    }

    #[test]
    fn test_horizontal_rule_attrs_default() {
        let attrs = HorizontalRuleAttrs::default();
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, None);
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, None);
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, None);
    }

    #[test]
    fn test_horizontal_rule_attrs_clone() {
        let attrs1 = HorizontalRuleAttrs {
            kind: Some("test".to_string()),
            legacy_style: None,
            alignment: Some("centered".to_string()),
            weight: Some("medium".to_string()),
            width: Some("50%".to_string()),
            color: Some("red".to_string()),
        };
        let attrs2 = attrs1.clone();
        assert_eq!(attrs1.kind, attrs2.kind);
        assert_eq!(attrs1.legacy_style, attrs2.legacy_style);
        assert_eq!(attrs1.alignment, attrs2.alignment);
        assert_eq!(attrs1.weight, attrs2.weight);
        assert_eq!(attrs1.width, attrs2.width);
        assert_eq!(attrs1.color, attrs2.color);
    }

    #[test]
    fn test_horizontal_rule_attrs_partial() {
        let attrs = HorizontalRuleAttrs {
            kind: None,
            legacy_style: Some("waves".to_string()),
            alignment: None,
            weight: Some("thick".to_string()),
            width: None,
            color: Some("blue".to_string()),
        };
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, Some("waves".to_string()));
        assert_eq!(attrs.alignment, None);
        assert_eq!(attrs.weight, Some("thick".to_string()));
        assert_eq!(attrs.width, None);
        assert_eq!(attrs.color, Some("blue".to_string()));
    }

    // Phase 2 / Sub-spec #6 — inline `kind` is canonical; `style` is deprecated.

    #[test]
    fn test_inline_kind_parses_without_warning() {
        let mut processor = RuleProcessor::new(crate::markdown::inline::MarkProcessor::new(
            pulldown_cmark::Parser::new("--- { kind: waves }"),
        ));
        let events: Vec<_> = processor.by_ref().collect();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], InlineEvent::HorizontalRule(_)));
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.kind, Some("waves".to_string()));
            assert_eq!(attrs.legacy_style, None);
        }
        assert!(processor.warnings().is_empty());
    }

    #[test]
    fn test_inline_legacy_style_emits_deprecation_warning() {
        let mut processor = RuleProcessor::new(crate::markdown::inline::MarkProcessor::new(
            pulldown_cmark::Parser::new("--- { style: waves }"),
        ));
        let events: Vec<_> = processor.by_ref().collect();
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.legacy_style, Some("waves".to_string()));
            assert_eq!(attrs.kind, None);
        }
        assert_eq!(processor.warnings().len(), 1);
        assert_eq!(processor.warnings()[0].path, "hr.inline.style");
        assert!(
            matches!(
                &processor.warnings()[0].kind,
                StyleWarningKind::Deprecated { replacement } if replacement == "hr.inline.kind"
            ),
            "expected Deprecated warning for inline style, got {:?}",
            processor.warnings()[0].kind
        );
    }

    #[test]
    fn test_inline_kind_beats_legacy_style() {
        let mut processor = RuleProcessor::new(crate::markdown::inline::MarkProcessor::new(
            pulldown_cmark::Parser::new("--- { kind: dots, style: waves }"),
        ));
        let events: Vec<_> = processor.by_ref().collect();
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.kind, Some("dots".to_string()));
            assert_eq!(attrs.legacy_style, Some("waves".to_string()));
        }
        // Legacy key is still present, so deprecation warning fires even when
        // canonical wins.
        assert_eq!(processor.warnings().len(), 1);
    }

    // ----- scan_inline_hr_warnings preflight -----

    #[test]
    fn scan_inline_hr_warnings_empty_for_clean_doc() {
        let warnings = scan_inline_hr_warnings("# Hello\n\n---\n\nSome text.");
        assert!(warnings.is_empty());
    }

    #[test]
    fn scan_inline_hr_warnings_detects_legacy_style() {
        let warnings = scan_inline_hr_warnings("--- { style: waves }");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].path, "hr.inline.style");
        assert!(
            matches!(
                &warnings[0].kind,
                StyleWarningKind::Deprecated { replacement } if replacement == "hr.inline.kind"
            )
        );
    }

    #[test]
    fn scan_inline_hr_warnings_detects_multiple_legacy_rules() {
        let warnings = scan_inline_hr_warnings(
            "--- { style: waves }\n\nSome text.\n\n--- { style: dots }\n",
        );
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn scan_inline_hr_warnings_empty_for_canonical_kind() {
        let warnings = scan_inline_hr_warnings("--- { kind: waves }");
        assert!(warnings.is_empty());
    }

    // Phase 5 B1: validation — unknown enum values are captured verbatim so the
    // downstream builder can warn + fall back to defaults. Unknown keys are
    // dropped with a warn (see `build_rule`); we assert the resulting attrs.

    #[test]
    fn test_parse_attributes_unknown_style_is_captured_raw() {
        // Unknown enum values are retained on the attrs struct so the
        // builder layer (`build_rule`) can emit a warning and fall back.
        let events = process_text("--- { style: bogus }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.legacy_style, Some("bogus".to_string()));
            assert_eq!(attrs.kind, None);
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    #[test]
    fn test_parse_attributes_unknown_alignment_is_captured_raw() {
        let events = process_text("--- { alignment: diagonal }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.alignment, Some("diagonal".to_string()));
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    #[test]
    fn test_parse_attributes_unknown_weight_is_captured_raw() {
        let events = process_text("--- { weight: zzz }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.weight, Some("zzz".to_string()));
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    #[test]
    fn test_parse_attributes_unknown_key_is_dropped() {
        // Unknown keys are dropped with a warn; the resulting attrs struct
        // has all fields `None`.
        let events = process_text("--- { margin: 4 }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.kind, None);
            assert_eq!(attrs.legacy_style, None);
            assert_eq!(attrs.alignment, None);
            assert_eq!(attrs.weight, None);
            assert_eq!(attrs.width, None);
            assert_eq!(attrs.color, None);
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_parse_attributes_unknown_key_emits_warning() {
        // With tracing-test's subscriber in scope, the warn! call inside
        // `parse_attributes` becomes observable via `logs_contain`.
        let _events = process_text("--- { totally_bogus_key: 42 }");
        assert!(logs_contain("unknown horizontal rule attribute"));
        assert!(logs_contain("totally_bogus_key"));
    }

    // =====================================================================
    // Phase 6 / E2 — YAML flow-mapping parsing handles quoted separators
    // =====================================================================

    #[test]
    fn test_parse_attributes_quoted_color_with_embedded_comma() {
        // Quoted values must preserve embedded commas. The hand-rolled
        // splitter used to break on these; `serde_yaml_ng` handles them
        // correctly.
        let events = process_text("--- { color: \"rgb(255, 0, 0)\" }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.color, Some("rgb(255, 0, 0)".to_string()));
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    #[test]
    fn test_parse_attributes_quoted_value_with_embedded_colon() {
        // Quoted values must preserve embedded colons. The key `prefix` is
        // unknown, so it is dropped — but the test still exercises the
        // quoted-colon code path because the YAML parser has to succeed
        // before the drop-with-warn happens.
        let events = process_text("--- { prefix: \"a:b\" }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            // Unknown key is dropped with a warn.
            assert_eq!(attrs.kind, None);
            assert_eq!(attrs.legacy_style, None);
            assert_eq!(attrs.color, None);
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    #[test]
    fn test_parse_attributes_malformed_yaml_falls_back_gracefully() {
        // `{ style: }` is not valid YAML (missing value). The parser must
        // not panic; instead it falls back to the legacy splitter (which
        // also cannot parse this) and produces default attrs.
        let events = process_text("--- { style: }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.kind, None);
            assert_eq!(attrs.legacy_style, None);
            assert_eq!(attrs.alignment, None);
            assert_eq!(attrs.weight, None);
            assert_eq!(attrs.width, None);
            assert_eq!(attrs.color, None);
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    #[test]
    fn test_parse_attributes_yaml_number_scalar_coerced_to_string() {
        // Numeric YAML scalars must round-trip as strings so downstream
        // string-typed fields receive a sensible value.
        let events = process_text("--- { width: 50 }");
        assert_eq!(events.len(), 1);
        if let InlineEvent::HorizontalRule(attrs) = &events[0] {
            assert_eq!(attrs.width, Some("50".to_string()));
        } else {
            panic!("expected HorizontalRule event");
        }
    }

    // =====================================================================
    // Phase 6 / C1 — RuleProcessor edge cases (list / blockquote / code
    // block / mixed markers)
    // =====================================================================

    #[test]
    fn test_horizontal_rule_inside_list_item_is_not_transformed() {
        // `- --- { style: dots }` is a list item; the HR-like text lives
        // inside a List/Item scope, not a bare paragraph. It must not be
        // converted to `InlineEvent::HorizontalRule`.
        let events = process_text("- --- { style: dots }");
        let hr_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::HorizontalRule(_)))
            .count();
        assert_eq!(
            hr_count, 0,
            "HR-like text inside a list item must not become a HorizontalRule event: {events:?}"
        );
        // The event stream should still contain List/Item tags.
        let has_list = events.iter().any(|e| {
            matches!(
                e,
                InlineEvent::Standard(Event::Start(Tag::List(_)))
                    | InlineEvent::Standard(Event::Start(Tag::Item))
            )
        });
        assert!(has_list, "expected List/Item tags in output: {events:?}");
    }

    #[test]
    fn test_horizontal_rule_inside_blockquote_is_transformed_per_spec() {
        // `> --- { style: waves }` places a single-paragraph HR-like token
        // inside a BlockQuote. pulldown-cmark emits a Paragraph *inside* the
        // BlockQuote; RuleProcessor intercepts that simple single-text
        // paragraph and transforms it into a `HorizontalRule` event, which
        // stays wrapped by the surrounding BlockQuote start/end tags (the
        // HR is not promoted to document level).
        //
        // This behavior is codified in
        // `darkmatter/features/2026-04-18-hr/spec.md` §"Parsing Requirements"
        // — bare `---`/`___`/`***` inside a blockquote remains a
        // horizontal-rule block, and page-level `hr` frontmatter defaults
        // apply exactly as they do at the top level.
        let events = process_text("> --- { style: waves }");

        let hr_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::HorizontalRule(_)))
            .count();
        assert_eq!(
            hr_count, 1,
            "spec: HR-like text inside a blockquote IS transformed into a HorizontalRule event (see spec.md §Parsing Requirements): {events:?}"
        );

        // Confirm the HR is wrapped by BlockQuote start/end.
        let bq_start_idx = events
            .iter()
            .position(|e| matches!(e, InlineEvent::Standard(Event::Start(Tag::BlockQuote(_)))));
        let bq_end_idx = events
            .iter()
            .position(|e| matches!(e, InlineEvent::Standard(Event::End(TagEnd::BlockQuote(_)))));
        assert!(
            bq_start_idx.is_some(),
            "expected BlockQuote start: {events:?}"
        );
        assert!(bq_end_idx.is_some(), "expected BlockQuote end: {events:?}");
    }

    #[test]
    fn test_horizontal_rule_inside_fenced_code_block_passthrough() {
        // HR-like content inside a fenced code block must be preserved
        // as text; the RuleProcessor must not transform it.
        let input = "```\n--- { style: waves }\n```\n";
        let events = process_text(input);

        let hr_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::HorizontalRule(_)))
            .count();
        assert_eq!(
            hr_count, 0,
            "HR-like text inside a code block must not become a HorizontalRule event"
        );

        // There must be a CodeBlock start/end and the raw text inside it.
        let has_code_start = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Standard(Event::Start(Tag::CodeBlock(_)))));
        let has_code_end = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Standard(Event::End(TagEnd::CodeBlock))));
        assert!(has_code_start, "expected code block start: {events:?}");
        assert!(has_code_end, "expected code block end: {events:?}");

        let has_hr_text = events.iter().any(|e| {
            matches!(
                e,
                InlineEvent::Standard(Event::Text(t)) if t.contains("--- { style: waves }")
            )
        });
        assert!(
            has_hr_text,
            "expected the HR-like text to pass through as Text: {events:?}"
        );
    }

    #[test]
    fn test_mixed_marker_dash_star_dash_is_not_transformed() {
        // `-*-` mixes marker characters; the implementation requires three
        // or more of the *same* character, so this must not be a HR.
        let events = process_text("-*- { style: dots }");
        let hr_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::HorizontalRule(_)))
            .count();
        assert_eq!(hr_count, 0, "mixed markers must not become HR: {events:?}");
    }

    #[test]
    fn test_mixed_marker_dash_underscore_dash_is_not_transformed() {
        // `-_-` is another mixed-marker case; same invariant applies.
        let events = process_text("-_- { style: dots }");
        let hr_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::HorizontalRule(_)))
            .count();
        assert_eq!(hr_count, 0, "mixed markers must not become HR: {events:?}");
    }
}
