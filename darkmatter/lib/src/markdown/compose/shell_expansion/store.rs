//! The grammar of one shell rule: the text of a `permissions.shell-commands`
//! entry without its `!`.
//!
//! The configuration loader parses every entry through [`parse_rule_entry`];
//! nothing else reads rules from text. Where the rules live and how layers
//! combine is `darkmatter::config::permissions`.

use super::template::TemplateShape;
use super::types::ShellRuleEntry;

/// Parses one rule, already trimmed: `exact <command>`, `prefix
/// <executable>`, or `template <shape>`.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::shell_expansion::store::parse_rule_entry;
/// use darkmatter::markdown::compose::shell_expansion::types::ShellRuleEntry;
///
/// assert_eq!(parse_rule_entry("prefix ls"), Ok(ShellRuleEntry::Prefix("ls".to_string())));
/// assert!(parse_rule_entry("exact ").is_err());
/// assert!(parse_rule_entry("allow ls").is_err());
/// ```
///
/// ## Errors
///
/// The reason the rule is not one of those forms, names no command, or why
/// its template shape is rejected.
pub fn parse_rule_entry(trimmed: &str) -> Result<ShellRuleEntry, String> {
    let payload = |rest: &str| {
        let rest = rest.trim();
        if rest.is_empty() { Err(format!("`{}` names no command", trimmed.trim())) } else { Ok(rest.to_string()) }
    };
    if let Some(command) = trimmed.strip_prefix("exact ") {
        payload(command).map(ShellRuleEntry::Exact)
    } else if let Some(command) = trimmed.strip_prefix("prefix ") {
        payload(command).map(ShellRuleEntry::Prefix)
    } else if let Some(shape) = trimmed.strip_prefix("template ") {
        TemplateShape::parse_rule(&payload(shape)?)
            .map(ShellRuleEntry::Template)
            .map_err(|error| error.to_string())
    } else if matches!(trimmed.trim(), "exact" | "prefix" | "template") {
        Err(format!("`{}` names no command", trimmed.trim()))
    } else {
        Err("expected 'exact <command>', 'prefix <executable>', or 'template <shape>'".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::find_git_root_from;
    use tempfile::TempDir;

    #[test]
    fn each_form_parses() {
        assert_eq!(parse_rule_entry("exact echo hello"), Ok(ShellRuleEntry::Exact("echo hello".to_string())));
        assert_eq!(parse_rule_entry("prefix ls"), Ok(ShellRuleEntry::Prefix("ls".to_string())));
        assert!(matches!(parse_rule_entry("template git add {{ string }}"), Ok(ShellRuleEntry::Template(_))));
    }

    #[test]
    fn an_empty_payload_is_an_error() {
        for text in ["exact", "exact ", "exact   ", "prefix", "prefix  ", "template", "template  "] {
            assert!(parse_rule_entry(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn an_unknown_form_is_an_error() {
        assert!(parse_rule_entry("invalid line").is_err());
        assert!(parse_rule_entry("# comment").is_err());
    }

    #[test]
    fn find_git_root_finds_git_directory() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        assert_eq!(find_git_root_from(&subdir), Some(temp_dir.path().to_path_buf()));
    }

    #[test]
    fn find_git_root_returns_none_if_not_found() {
        let temp_dir = TempDir::new().unwrap();
        assert!(find_git_root_from(temp_dir.path()).is_none());
    }
}
