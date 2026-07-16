//! Self-contained free helpers extracted from the module root: subagent
//! status-line composition and prose-markup escaping.

pub(super) fn subagent_description(arrow: char, name: &Option<String>) -> String {
    let name_part = name.as_deref().unwrap_or("(subagent)");
    format!("{arrow} {name_part}")
}

/// Escape user-controlled text so it can be safely interpolated into
/// biscuit-terminal prose markup without being parsed as tags / tokens.
///
/// Biscuit-terminal's `Prose` parser recognises backslash escapes for `<`,
/// `>`, `{`, and `\`; escaping those four characters is sufficient to
/// prevent arbitrary user strings (commands, paths, URLs, raw JSON) from
/// being interpreted as markup.
pub(crate) fn escape_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}
