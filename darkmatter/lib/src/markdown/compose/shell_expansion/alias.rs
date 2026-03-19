//! Shell alias resolution.
//!
//! Resolves shell aliases to their underlying commands by querying the user's
//! login shell (`$SHELL`). This allows `::shell` directives to use common
//! aliases like `ll`, `la`, etc.

use super::tokenize::tokenize;
use std::process::{Command, Stdio};

/// Result of resolving a shell alias.
#[derive(Debug, Clone)]
pub struct ResolvedAlias {
    /// The alias name that was resolved (e.g., "ll").
    pub alias_name: String,
    /// The resolved executable (e.g., "eza").
    pub executable: String,
    /// The resolved arguments from the alias definition.
    pub args: Vec<String>,
    /// The raw alias definition string (e.g., "eza -lhga --git").
    pub definition: String,
}

/// Attempts to resolve a shell alias to its underlying command.
///
/// Queries the user's login shell for the alias definition, tokenizes
/// it, and verifies the resolved executable exists on PATH.
///
/// ## Returns
///
/// `Some(ResolvedAlias)` if the name is a valid shell alias that resolves
/// to an executable on PATH. `None` if the name is not an alias, the shell
/// cannot be queried, or the resolved command is not found.
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::alias::resolve_alias;
///
/// if let Some(resolved) = resolve_alias("ll") {
///     println!("ll resolves to: {} {:?}", resolved.executable, resolved.args);
/// }
/// ```
pub fn resolve_alias(name: &str) -> Option<ResolvedAlias> {
    if !is_valid_alias_name(name) {
        return None;
    }

    let shell = std::env::var("SHELL").ok()?;

    let output = Command::new(&shell)
        .args(["-ic", &format!("alias {}", name)])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let alias_output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if alias_output.is_empty() {
        return None;
    }

    let definition = parse_alias_value(&alias_output, name)?;

    // Tokenize the alias value using our safe tokenizer (rejects metacharacters)
    let tokens = tokenize(&definition).ok()?;
    if tokens.is_empty() {
        return None;
    }

    // Verify the resolved executable actually exists on PATH
    if which::which(&tokens[0]).is_err() {
        return None;
    }

    Some(ResolvedAlias {
        alias_name: name.to_string(),
        executable: tokens[0].clone(),
        args: tokens[1..].to_vec(),
        definition,
    })
}

/// Validates that a name is safe to use in an alias lookup command.
///
/// Only allows alphanumeric characters, hyphens, underscores, and dots.
fn is_valid_alias_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Extracts the alias value from shell `alias` command output.
///
/// Handles both bash and zsh formats:
/// - zsh: `ll='eza -lhga --git'` or `ll=eza`
/// - bash: `alias ll='eza -lhga --git'`
fn parse_alias_value(output: &str, name: &str) -> Option<String> {
    // Strip optional "alias " prefix (bash format)
    let def = output.strip_prefix("alias ").unwrap_or(output);

    // Find value after "name="
    let prefix = format!("{}=", name);
    let value = def.strip_prefix(&prefix)?;

    // Strip surrounding quotes
    let value = value.trim();
    let unquoted = if (value.starts_with('\'') && value.ends_with('\''))
        || (value.starts_with('"') && value.ends_with('"'))
    {
        &value[1..value.len() - 1]
    } else {
        value
    };

    if unquoted.is_empty() {
        return None;
    }

    Some(unquoted.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_alias_names() {
        assert!(is_valid_alias_name("ll"));
        assert!(is_valid_alias_name("ls"));
        assert!(is_valid_alias_name("my-alias"));
        assert!(is_valid_alias_name("my_alias"));
        assert!(is_valid_alias_name("alias.name"));
        assert!(is_valid_alias_name("ls2"));
    }

    #[test]
    fn invalid_alias_names() {
        assert!(!is_valid_alias_name(""));
        assert!(!is_valid_alias_name("ls;rm"));
        assert!(!is_valid_alias_name("foo bar"));
        assert!(!is_valid_alias_name("$(whoami)"));
        assert!(!is_valid_alias_name("foo|bar"));
        assert!(!is_valid_alias_name("a>b"));
    }

    #[test]
    fn parse_zsh_single_quoted_alias() {
        let output = "ll='eza -lhga --git --hyperlink --group'";
        let value = parse_alias_value(output, "ll");
        assert_eq!(
            value.as_deref(),
            Some("eza -lhga --git --hyperlink --group")
        );
    }

    #[test]
    fn parse_zsh_double_quoted_alias() {
        let output = "ll=\"eza -lhga --git\"";
        let value = parse_alias_value(output, "ll");
        assert_eq!(value.as_deref(), Some("eza -lhga --git"));
    }

    #[test]
    fn parse_zsh_unquoted_alias() {
        let output = "ll=eza";
        let value = parse_alias_value(output, "ll");
        assert_eq!(value.as_deref(), Some("eza"));
    }

    #[test]
    fn parse_bash_format_alias() {
        let output = "alias ll='eza -lhga --git --hyperlink --group'";
        let value = parse_alias_value(output, "ll");
        assert_eq!(
            value.as_deref(),
            Some("eza -lhga --git --hyperlink --group")
        );
    }

    #[test]
    fn parse_alias_wrong_name_returns_none() {
        let output = "la='ls -la'";
        let value = parse_alias_value(output, "ll");
        assert!(value.is_none());
    }

    #[test]
    fn parse_alias_empty_value_returns_none() {
        let output = "ll=''";
        let value = parse_alias_value(output, "ll");
        assert!(value.is_none());
    }

    /// Integration test: resolve a known alias from the current shell.
    /// Ignored by default since it depends on the user's shell configuration.
    #[test]
    #[ignore]
    fn resolve_alias_ll() {
        if let Some(resolved) = resolve_alias("ll") {
            println!(
                "ll -> {} {:?} (definition: {})",
                resolved.executable, resolved.args, resolved.definition
            );
            assert!(!resolved.executable.is_empty());
        } else {
            println!("ll is not aliased in the current shell");
        }
    }
}
