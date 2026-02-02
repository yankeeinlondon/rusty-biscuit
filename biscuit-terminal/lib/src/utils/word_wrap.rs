/// The **word_wrap** function follows the following logic:
///
/// 1. Split content into a vector of string so that we can work with
///    lines of text which have no explicit line breaks and each line
///    given an unlimited amount of space to be rendered to would represent
///    a single line of text.
///
/// 2. Iterate over each line of text and:
///
///     - if `plain_text_length(line)` fits into the available width we're done ...
///       the content does not need to be wrapped, truncated, etc.
///     - if we're
pub fn word_wrap<T: Into<String>>(content: T, strategy: WordWrap, width: u32) {
    let lines = split_lines(content);

    let _ = wrap_lines(lines, &strategy, width);
}

/// truncates the line with the `truncate_indicator` string used as the closing
/// part of the string and leaving the resultant string length equal to the `width`.
///
/// Note: this truncation must be smart and be aware of
pub fn truncate<T: Into<String>>(content: T, truncate_indicator: &String, width: &u32) -> String {
    let content = content.into();
    if *width == 0 {
        return String::new();
    }

    let indicator_width = visible_width(truncate_indicator);
    if *width <= indicator_width {
        let (head, _) = split_at_visible_width(truncate_indicator, *width);
        return head;
    }

    if visible_width(&content) <= *width {
        return content;
    }

    let target_width = width.saturating_sub(indicator_width);
    let (head, _) = split_at_visible_width(&content, target_width);
    format!("{}{}", head, truncate_indicator)
}
