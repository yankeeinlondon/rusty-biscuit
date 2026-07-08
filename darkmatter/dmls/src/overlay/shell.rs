//! Layer-3 shell overlay: read-only parsing and policy classification of
//! `::shell` / `::shell-block` / `$()` commands.
//!
//! DMLS never runs a command. It splits one into chain segments, then classifies
//! each against Darkmatter's shell policy — the always-on **library** built-in
//! blacklist (dangerous commands), plus an optional workspace whitelist loaded
//! from `shell.policy_path`. Command-splitting is intentionally local and
//! lightweight (a passive analyzer needs the executable and args, not a full
//! shell grammar) so DMLS stays decoupled from the terminal-oriented compose
//! tokenizer, while the *policy rules* remain the library's authority. A hover
//! shows the parsed command and the verdict; a `dm.security.disallowed_command`
//! diagnostic fires only for the built-in blacklist (a decision `md compose`
//! would also refuse), so there are no false positives against valid docs.

use darkmatter::markdown::compose::shell_expansion::policy::{
    check_builtin_blacklist, check_whitelist, normalize_command,
};
use darkmatter::markdown::compose::shell_expansion::store::load_ruleset;
use darkmatter::markdown::compose::shell_expansion::types::ShellRuleSet;

use crate::config::DmlsConfig;

/// One command in a chain: an executable and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSegment {
    /// The executable (the first word).
    pub executable: String,
    /// The remaining arguments.
    pub args: Vec<String>,
}

/// A parsed command chain (`a && b || c`), split at top-level operators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    /// The chain segments in source order.
    pub segments: Vec<CommandSegment>,
}

/// The policy classification of a parsed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyVerdict {
    /// Every command in the chain matched the workspace whitelist.
    Approved,
    /// A command matched the built-in dangerous-command blacklist.
    Denied(String),
    /// No policy rule matched — compose would prompt for approval.
    Unknown,
}

/// Splits a raw command into chain segments with executable + args.
///
/// Returns `None` for an empty command. Splitting is at top-level `&&`, `||`,
/// `|`, and `;`; each segment is whitespace-tokenized with simple single/double
/// quote handling.
pub fn parse_command(command: &str) -> Option<ParsedCommand> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    let segments: Vec<CommandSegment> = split_chain(command)
        .into_iter()
        .filter_map(|segment| {
            let mut words = tokenize_words(segment).into_iter();
            let executable = words.next()?;
            Some(CommandSegment {
                executable,
                args: words.collect(),
            })
        })
        .collect();
    (!segments.is_empty()).then_some(ParsedCommand { segments })
}

/// Classifies a command against the built-in blacklist and an optional
/// workspace whitelist.
pub fn verdict(command: &ParsedCommand, whitelist: Option<&ShellRuleSet>) -> PolicyVerdict {
    for segment in &command.segments {
        if let Some(reason) = check_builtin_blacklist(&segment.executable, &segment.args) {
            return PolicyVerdict::Denied(reason);
        }
    }
    if let Some(whitelist) = whitelist {
        let all_whitelisted = command.segments.iter().all(|segment| {
            let normalized = normalize_command(&segment.executable, &segment.args);
            check_whitelist(whitelist, &segment.executable, &normalized)
        });
        if all_whitelisted {
            return PolicyVerdict::Approved;
        }
    }
    PolicyVerdict::Unknown
}

/// Loads the workspace shell whitelist from `shell.policy_path` (a directory
/// holding `.darkmatter-shell-whitelist`), if configured. Absent config or a
/// missing file yields `None` — the verdict then rests on the built-in
/// blacklist alone.
pub fn load_whitelist(config: &DmlsConfig) -> Option<ShellRuleSet> {
    let dir = config.shell.policy_path.as_ref()?;
    let path = dir.join(".darkmatter-shell-whitelist");
    match load_ruleset(&path) {
        Ok(ruleset) if !ruleset.entries.is_empty() => Some(ruleset),
        _ => None,
    }
}

/// Splits a command on top-level chain operators (`&&`, `||`, `|`, `;`),
/// leaving quoted spans intact.
fn split_chain(command: &str) -> Vec<&str> {
    let bytes = command.as_bytes();
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quote: Option<u8> = None;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(active) => {
                if byte == active {
                    quote = None;
                }
                index += 1;
            }
            None => match byte {
                b'"' | b'\'' => {
                    quote = Some(byte);
                    index += 1;
                }
                b';' => {
                    segments.push(&command[start..index]);
                    index += 1;
                    start = index;
                }
                b'&' | b'|' if index + 1 < bytes.len() && bytes[index + 1] == byte => {
                    segments.push(&command[start..index]);
                    index += 2;
                    start = index;
                }
                b'|' => {
                    segments.push(&command[start..index]);
                    index += 1;
                    start = index;
                }
                _ => index += 1,
            },
        }
    }
    segments.push(&command[start..]);
    segments
        .into_iter()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect()
}

/// Whitespace-tokenizes one command segment, stripping matched surrounding
/// single/double quotes from each word.
fn tokenize_words(segment: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut in_word = false;
    for ch in segment.chars() {
        match quote {
            Some(active) => {
                if ch == active {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                    in_word = true;
                } else if ch.is_whitespace() {
                    if in_word {
                        words.push(std::mem::take(&mut current));
                        in_word = false;
                    }
                } else {
                    current.push(ch);
                    in_word = true;
                }
            }
        }
    }
    if in_word {
        words.push(current);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::shell_expansion::types::ShellRuleEntry;

    #[test]
    fn test_builtin_blacklist_denies_dangerous_command() {
        let command = parse_command("rm -rf /").expect("parses");
        assert!(matches!(verdict(&command, None), PolicyVerdict::Denied(_)));
    }

    #[test]
    fn test_unknown_without_whitelist() {
        let command = parse_command("echo hello").expect("parses");
        assert_eq!(verdict(&command, None), PolicyVerdict::Unknown);
    }

    #[test]
    fn test_approved_when_whitelisted() {
        let mut whitelist = ShellRuleSet::default();
        whitelist.entries.push(ShellRuleEntry::Prefix("echo".to_string()));
        let command = parse_command("echo hi").expect("parses");
        assert_eq!(verdict(&command, Some(&whitelist)), PolicyVerdict::Approved);
    }

    #[test]
    fn test_chain_denied_if_any_link_dangerous() {
        let command = parse_command("echo ok && rm -rf /tmp/x").expect("parses");
        assert_eq!(command.segments.len(), 2);
        assert!(matches!(verdict(&command, None), PolicyVerdict::Denied(_)));
    }

    #[test]
    fn test_quoted_words_preserved() {
        let command = parse_command("git commit -m \"a b c\"").expect("parses");
        assert_eq!(command.segments[0].executable, "git");
        assert_eq!(command.segments[0].args, vec!["commit", "-m", "a b c"]);
    }

    #[test]
    fn test_empty_command_has_no_segments() {
        assert!(parse_command("   ").is_none());
    }
}
