//! Self-contained free helpers extracted from the module root: subagent
//! status-line composition and prose-markup escaping.

pub(super) fn subagent_description(arrow: char, name: &Option<String>) -> String {
    let name_part = name.as_deref().unwrap_or("(subagent)");
    format!("{arrow} {name_part}")
}

/// Escape text for splicing into Prose markup, so it renders exactly as written.
pub(crate) fn escape_prose(input: &str) -> String {
    biscuit_terminal::components::prose::Prose::escape_text(input)
}
