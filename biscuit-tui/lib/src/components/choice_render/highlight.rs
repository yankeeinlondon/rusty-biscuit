use ratatui::{style::Style, text::Span};

/// Splits `label` into `Span`s that highlight char-indexed matches
/// with `match_style` and renders the remaining text with
/// `base_style`. `highlights` must be sorted-ascending char offsets.
pub fn build_highlighted_spans(
    label: &str,
    highlights: &[u32],
    base_style: Style,
    match_style: Style,
) -> Vec<Span<'static>> {
    if highlights.is_empty() {
        return vec![Span::styled(label.to_string(), base_style)];
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_is_match = false;
    for (char_idx, ch) in label.chars().enumerate() {
        let is_match = highlights.binary_search(&(char_idx as u32)).is_ok();
        if current.is_empty() {
            current_is_match = is_match;
            current.push(ch);
            continue;
        }
        if is_match == current_is_match {
            current.push(ch);
        } else {
            let style = if current_is_match {
                match_style
            } else {
                base_style
            };
            spans.push(Span::styled(std::mem::take(&mut current), style));
            current_is_match = is_match;
            current.push(ch);
        }
    }
    if !current.is_empty() {
        let style = if current_is_match {
            match_style
        } else {
            base_style
        };
        spans.push(Span::styled(current, style));
    }
    spans
}
