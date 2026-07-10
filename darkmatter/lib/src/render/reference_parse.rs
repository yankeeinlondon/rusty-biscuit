use std::collections::BTreeMap;

pub(crate) fn parse_html_attributes(input: &str) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let mut chars = input.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) { chars.next(); }
        let mut key = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() || ch == '=' { break; }
            key.push(ch); chars.next();
        }
        if key.is_empty() { break; }
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) { chars.next(); }
        if chars.peek() != Some(&'=') {
            attrs.insert(key.to_ascii_lowercase(), String::new());
            continue;
        }
        chars.next();
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) { chars.next(); }
        let value = match chars.peek().copied() {
            Some('"') | Some('\'') => {
                let quote = chars.next().expect("peeked quote");
                let value: String = chars.by_ref().take_while(|&ch| ch != quote).collect();
                html_unescape(&value)
            }
            _ => {
                let value: String = chars.by_ref().take_while(|ch| !ch.is_whitespace()).collect();
                html_unescape(&value)
            }
        };
        attrs.insert(key.to_ascii_lowercase(), value);
    }
    attrs
}

pub(crate) fn find_closing_bracket(input: &str, start: usize) -> Option<usize> {
    find_balanced(input, start, '[', ']', false)
}

pub(crate) fn find_closing_paren(input: &str, start: usize) -> Option<usize> {
    find_balanced(input, start, '(', ')', true)
}

fn find_balanced(input: &str, start: usize, open: char, close: char, quotes: bool) -> Option<usize> {
    if !input.is_char_boundary(start) { return None; }
    let mut depth = 0usize;
    let mut escaped = false;
    let mut quote = None;
    for (offset, ch) in input[start..].char_indices() {
        let idx = start + offset;
        if escaped { escaped = false; continue; }
        if ch == '\\' { escaped = true; continue; }
        if quotes && matches!(ch, '"' | '\'') {
            if quote == Some(ch) { quote = None; } else if quote.is_none() { quote = Some(ch); }
            continue;
        }
        if quote.is_some() { continue; }
        if ch == open { depth += 1; }
        if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 { return Some(idx); }
        }
    }
    None
}

pub(crate) fn extract_markdown_url(content: &str) -> (String, &str) {
    let content = content.trim();
    if let Some(rest) = content.strip_prefix('<')
        && let Some(end) = rest.find('>')
    {
        return (rest[..end].to_string(), &rest[end + 1..]);
    }
    let end = content.char_indices().find_map(|(idx, ch)| ch.is_whitespace().then_some(idx))
        .unwrap_or(content.len());
    (content[..end].to_string(), &content[end..])
}

pub(crate) fn is_structured(content: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    for (idx, ch) in content.trim().char_indices() {
        if escaped { escaped = false; continue; }
        if ch == '\\' { escaped = true; continue; }
        if matches!(ch, '"' | '\'') {
            if quote == Some(ch) { quote = None; } else if quote.is_none() { quote = Some(ch); }
        } else if ch == '=' && quote.is_none() {
            let key = content[..idx].trim().split([' ', ',']).next_back().unwrap_or("");
            if !key.is_empty() && key.chars().all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_')) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn parse_structured(content: &str, mut apply: impl FnMut(&str, String)) {
    let mut chars = content.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|&ch| ch.is_whitespace() || ch == ',') { chars.next(); }
        let mut key = String::new();
        while let Some(&ch) = chars.peek() {
            if ch == '=' || ch.is_whitespace() || ch == ',' { break; }
            key.push(ch); chars.next();
        }
        if key.is_empty() { break; }
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) { chars.next(); }
        if chars.peek() != Some(&'=') { continue; }
        chars.next();
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) { chars.next(); }
        let value = if matches!(chars.peek(), Some('"' | '\'')) {
            let quote = chars.next().expect("peeked quote");
            let mut value = String::new();
            while let Some(ch) = chars.next() {
                if ch == '\\' { if let Some(next) = chars.next() { value.push(next); } }
                else if ch == quote { break; } else { value.push(ch); }
            }
            value
        } else {
            let mut value = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() || ch == ',' { break; }
                value.push(ch); chars.next();
            }
            value
        };
        apply(key.trim(), value);
    }
}

pub(crate) fn parse_title(value: &str) -> String {
    let value = value.trim();
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return String::new(); };
    if !matches!(first, '"' | '\'') { return value.to_string(); }
    let mut out = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped { out.push(ch); escaped = false; }
        else if ch == '\\' { escaped = true; }
        else if ch == first { break; }
        else { out.push(ch); }
    }
    out
}

pub(crate) fn normalize_optional(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub(crate) fn normalize_data_key(key: String) -> Option<String> {
    let key = key.trim().strip_prefix("data-").unwrap_or(key.trim());
    (!key.is_empty()).then(|| key.to_string())
}

pub(crate) fn html_escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&#x27;")
}

pub(crate) fn html_unescape(value: &str) -> String {
    value.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#x27;", "'").replace("&#39;", "'")
}

pub(crate) fn strip_ansi_sequences(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' { out.push(ch); continue; }
        match chars.peek().copied() {
            Some('[') => { chars.next(); for ch in chars.by_ref() { if ('@'..='~').contains(&ch) { break; } } }
            Some(']') => {
                chars.next(); let mut esc = false;
                for ch in chars.by_ref() { if ch == '\u{7}' || esc && ch == '\\' { break; } esc = ch == '\u{1b}'; }
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn decode_markdown_url(value: &str) -> String { value.replace("%28", "(").replace("%29", ")") }
pub(crate) fn escape_markdown_url(value: &str) -> String { value.replace('(', "%28").replace(')', "%29") }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{ImageRef, Link};

    #[test]
    fn reference_parsers_preserve_utf8_titles_and_text() {
        let cases = [
            ("[Café](guide.md \"Résumé 東京\")", "![Café](guide.png \"Résumé 東京\")"),
            ("<A HREF='guide.md' TITLE='Résumé 東京'>Café</A>", "<IMG SRC='guide.png' ALT='Café' TITLE='Résumé 東京'>"),
        ];

        for (link_source, image_source) in cases {
            let link = Link::try_from(link_source).expect("link should parse");
            let image = ImageRef::try_from(image_source).expect("image should parse");
            assert_eq!(link.display(), "Café");
            assert_eq!(image.alt(), "Café");
            assert_eq!(link.title(), Some("Résumé 東京"));
            assert_eq!(image.title(), Some("Résumé 東京"));
        }
    }

    #[test]
    fn shared_scanners_handle_escapes_nesting_and_malformed_input() {
        let cases = [
            ("[a\\]b](x)", true, true),
            ("[a](path_(nested) \"t\\\"itle\")", true, true),
            ("[unclosed", false, false),
        ];

        for (source, bracket, paren) in cases {
            assert_eq!(find_closing_bracket(source, 0).is_some(), bracket);
            let paren_start = source.find('(').unwrap_or(source.len());
            assert_eq!(find_closing_paren(source, paren_start).is_some(), paren);
        }
    }

    #[test]
    fn html_attribute_names_are_ascii_case_insensitive() {
        let attrs = parse_html_attributes("HREF='guide.md' TITLE='Résumé'");
        assert_eq!(attrs.get("href").map(String::as_str), Some("guide.md"));
        assert_eq!(attrs.get("title").map(String::as_str), Some("Résumé"));
    }
}
