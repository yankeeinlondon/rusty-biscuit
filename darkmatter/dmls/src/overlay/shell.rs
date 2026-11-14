//! Layer-3 shell overlay: read-only parsing and policy classification of
//! `::shell` / `::shell-block` / `$()` commands.
//!
//! DMLS never runs, approves, or saves a command, and never prompts. It splits
//! one into chain segments, then classifies each the way `md compose` decides
//! it: Darkmatter's built-in blacklist first, then the
//! `permissions.shell-commands` of the document's configuration, loaded
//! passively (the user file, and inside a repository its file as committed at
//! `HEAD` and in the working copy). Command-splitting is intentionally local
//! and lightweight (a passive analyzer needs the executable and args, not a
//! full shell grammar) so DMLS stays decoupled from the terminal-oriented
//! compose tokenizer, while the *policy rules* remain the library's
//! authority. A hover shows the parsed command and the verdict; a
//! `dm.security.disallowed_command` diagnostic fires only for the built-in
//! blacklist (a decision `md compose` would also refuse), so there are no
//! false positives against valid docs.

use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::config::permissions::PermissionPolicy;
use darkmatter::markdown::compose::shell_expansion::policy::{
    check_builtin_blacklist, check_user_blacklist, check_whitelist, normalize_command,
};

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
    /// The captured permissions pre-approve every command in the chain and
    /// no `!` entry matches one.
    Approved,
    /// A command matched the built-in dangerous-command blacklist.
    Denied(String),
    /// No rule pre-approves the chain, an exclusion matches it, or the
    /// permissions do not load — compose would prompt for approval.
    Unknown,
}

/// The document's captured permissions, or why there are none.
pub type CapturedPermissions = Result<Arc<PermissionPolicy>, String>;

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

/// Classifies a command: the built-in blacklist denies; otherwise the chain is
/// approved only when `permissions` loaded and pre-approve every segment with
/// no exclusion matching it. Invalid permissions never approve.
pub fn verdict(command: &ParsedCommand, permissions: &CapturedPermissions) -> PolicyVerdict {
    if let Some(reason) = builtin_refusal(command) {
        return PolicyVerdict::Denied(reason);
    }
    let Ok(policy) = permissions else {
        return PolicyVerdict::Unknown;
    };
    let preapproved = command.segments.iter().all(|segment| {
        let normalized = normalize_command(&segment.executable, &segment.args);
        policy
            .shell()
            .verdict(
                |allows| check_whitelist(allows, &segment.executable, &normalized),
                |excluded| check_user_blacklist(excluded, &segment.executable, &segment.args, &normalized),
            )
            .is_preapproved()
    });
    if preapproved { PolicyVerdict::Approved } else { PolicyVerdict::Unknown }
}

/// The built-in blacklist's reason for refusing a segment of `command`: the
/// only verdict that needs no configuration.
pub fn builtin_refusal(command: &ParsedCommand) -> Option<String> {
    command
        .segments
        .iter()
        .find_map(|segment| check_builtin_blacklist(&segment.executable, &segment.args))
}

/// The permissions of the configuration `context` names, loaded passively:
/// nothing is migrated, written, or prompted for.
pub fn load_permissions(context: &FileResolutionContext) -> CapturedPermissions {
    let snapshot = darkmatter::markdown::schemas::slots::session::load_passively(context).map_err(|error| error.to_string())?;
    snapshot.policy().map(Arc::clone).map_err(|error| error.to_string())
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

    /// The permissions of a user whose home holds `rules`, outside any
    /// repository.
    fn permissions(rules: &[&str]) -> (tempfile::TempDir, CapturedPermissions) {
        let home = tempfile::tempdir().unwrap();
        let list: Vec<String> = rules.iter().map(|rule| serde_json::to_string(rule).unwrap()).collect();
        std::fs::write(
            home.path().join(".darkmatter.yaml"),
            format!("permissions:\n  shell-commands: [{}]\n", list.join(", ")),
        )
        .unwrap();
        let context = FileResolutionContext::from_snapshot(home.path(), Some(home.path().to_path_buf()), Default::default());
        let loaded = load_permissions(&context);
        (home, loaded)
    }

    #[test]
    fn test_builtin_blacklist_denies_dangerous_command() {
        let (_home, permissions) = permissions(&["prefix rm"]);
        let command = parse_command("rm -rf /").expect("parses");
        assert!(matches!(verdict(&command, &permissions), PolicyVerdict::Denied(_)));
    }

    #[test]
    fn test_unknown_without_a_rule() {
        let (_home, permissions) = permissions(&[]);
        let command = parse_command("echo hello").expect("parses");
        assert_eq!(verdict(&command, &permissions), PolicyVerdict::Unknown);
    }

    #[test]
    fn test_approved_when_preapproved() {
        let (_home, permissions) = permissions(&["prefix echo"]);
        let command = parse_command("echo hi").expect("parses");
        assert_eq!(verdict(&command, &permissions), PolicyVerdict::Approved);
    }

    #[test]
    fn test_unknown_when_excluded_or_unloaded() {
        let (_home, excluded) = permissions(&["prefix echo", "!exact echo secret"]);
        let command = parse_command("echo secret").expect("parses");
        assert_eq!(verdict(&command, &excluded), PolicyVerdict::Unknown);
        assert_eq!(verdict(&command, &Err("invalid".to_string())), PolicyVerdict::Unknown);
    }

    #[test]
    fn test_chain_denied_if_any_link_dangerous() {
        let (_home, permissions) = permissions(&["prefix echo"]);
        let command = parse_command("echo ok && rm -rf /tmp/x").expect("parses");
        assert_eq!(command.segments.len(), 2);
        assert!(matches!(verdict(&command, &permissions), PolicyVerdict::Denied(_)));
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
