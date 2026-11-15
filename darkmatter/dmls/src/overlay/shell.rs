//! Layer-3 shell overlay: read-only parsing and policy classification of
//! `::shell` / `::shell-block` / `$()` commands.
//!
//! DMLS never runs, approves, or saves a command, and never prompts. It
//! classifies each command the way `md compose`'s pre-flight decides its
//! site: Darkmatter's built-in blacklist first, then the
//! `permissions.shell-commands` of the document's configuration, loaded
//! passively (the user file, and inside a repository its file as committed at
//! `HEAD` and in the working copy). Every rule kind and every match is the
//! library's: the authored command is read with the template grammar
//! pre-flight uses and typed with the site's [`TypeScope`], so an `exact`,
//! `prefix`, or `template` rule, and a `!` entry of each, decides here as it
//! does there. A command the template grammar rejects (a pipeline, `;`) is
//! split locally into chain segments and judged by `exact`/`prefix` rules,
//! as its resolved bytes would be. A hover shows the parsed command and the
//! verdict; a `dm.security.disallowed_command` diagnostic fires only for the
//! built-in blacklist (a decision `md compose` would also refuse), so there
//! are no false positives against valid docs.

use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::config::permissions::PermissionPolicy;
use darkmatter::markdown::compose::expression::TypeScope;
use darkmatter::markdown::compose::shell_expansion::parser::authored_template;
use darkmatter::markdown::compose::shell_expansion::policy::{
    check_builtin_blacklist, check_template_blacklist, check_template_exclusion, check_user_blacklist,
    check_whitelist, normalize_command,
};
use darkmatter::markdown::compose::shell_expansion::template::{
    CommandTemplate, FilledChain, FilledSegment, ShapePart, TemplateShape, parse_authored,
};
use darkmatter::markdown::compose::shell_expansion::types::{ShellRuleEntry, ShellRuleSet};

/// One command in a chain: an executable and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSegment {
    /// The executable (the first word).
    pub executable: String,
    /// The remaining arguments.
    pub args: Vec<String>,
}

/// A parsed command chain (`a && b || c`), split at top-level operators.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    /// The chain segments in source order.
    pub segments: Vec<CommandSegment>,
    /// The authored command read with the template grammar, untyped; `None`
    /// when the grammar rejects it.
    template: Option<CommandTemplate>,
}

/// The policy classification of a parsed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyVerdict {
    /// The captured permissions pre-approve every command in the chain and
    /// no `!` entry matches one.
    Approved,
    /// A command matched the built-in dangerous-command blacklist.
    Denied(String),
    /// No rule pre-approves the chain, an exclusion matches it (or may match
    /// a value DMLS does not read), or the permissions do not load — compose
    /// would prompt for approval.
    Unknown,
}

/// The document's captured permissions, or why there are none.
pub type CapturedPermissions = Result<Arc<PermissionPolicy>, String>;

/// Splits a `::shell-block` line or a frontmatter `$()` command into chain
/// segments with executable + args.
///
/// Returns `None` for an empty command. Splitting is at top-level `&&`, `||`,
/// `|`, and `;`; each segment is whitespace-tokenized with simple single/double
/// quote handling.
pub fn parse_command(command: &str) -> Option<ParsedCommand> {
    parse_with(command, parse_authored(command.trim()).ok())
}

/// [`parse_command`] for a `::shell` directive's target: its template drops
/// the directive's own options (`--no-cache`, `--when-error …`,
/// `::timeout:<N>`), as pre-flight reads the site.
pub fn parse_directive_command(command: &str) -> Option<ParsedCommand> {
    parse_with(command, authored_template(command.trim()).ok())
}

fn parse_with(command: &str, template: Option<CommandTemplate>) -> Option<ParsedCommand> {
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
    (!segments.is_empty()).then_some(ParsedCommand { segments, template })
}

/// Classifies a command at a site whose types are `site`, as pre-flight
/// decides it: the built-in blacklist denies; otherwise the chain is approved
/// only when `permissions` loaded, no `!` entry matches it, and a rule covers
/// it. Invalid permissions never approve.
///
/// A command with a placeholder is judged by its typed shape, as pre-flight
/// judges a site whose values are late: a `template` rule covering the shape,
/// or a `prefix` rule for every program. DMLS does not evaluate placeholders,
/// so an `exact` allow never approves such a command, and an `exact` or
/// `template` exclusion naming one of its programs leaves it unknown (the
/// exclusion may match the value).
pub fn verdict(command: &ParsedCommand, site: &TypeScope<'_>, permissions: &CapturedPermissions) -> PolicyVerdict {
    if let Some(reason) = builtin_refusal(command) {
        return PolicyVerdict::Denied(reason);
    }
    let Ok(policy) = permissions else {
        return PolicyVerdict::Unknown;
    };
    let shell = policy.shell();
    let shape = command.template.clone().map(|mut template| {
        template.project_types(site);
        TemplateShape::from_template(&template)
    });
    let approved = match shape {
        Some(shape) if shape.has_holes() => {
            let excluded = !shell
                .verdict(|_| true, |rules| check_template_exclusion(&shape, rules) || reads_values(&shape, rules))
                .is_preapproved();
            !excluded
                && (shell.preapproves(|rules| rules.template_covers(&shape))
                    || shape.segments.iter().all(|segment| shell.preapproves(|rules| rules.matches_prefix(&segment.program))))
        }
        shape => {
            let segments = shape.as_ref().map_or_else(|| tokenized(command), literal_segments);
            let chain = shape.as_ref().map(|shape| filled_chain(shape, &segments));
            let excluded = !shell
                .verdict(
                    |_| true,
                    |rules| {
                        shape.as_ref().is_some_and(|shape| check_template_exclusion(shape, rules))
                            || chain.as_ref().is_some_and(|chain| rules.blocks_filled(chain))
                            || segments.iter().any(|(program, args)| {
                                check_user_blacklist(rules, program, args, &normalize_command(program, args))
                            })
                    },
                )
                .is_preapproved();
            !excluded
                && segments.iter().all(|(program, args)| {
                    shell.preapproves(|rules| check_whitelist(rules, program, &normalize_command(program, args)))
                })
        }
    };
    if approved { PolicyVerdict::Approved } else { PolicyVerdict::Unknown }
}

/// The built-in blacklist's reason for refusing a segment of `command`, or
/// its authored shape's known words: the only verdict that needs no
/// configuration.
pub fn builtin_refusal(command: &ParsedCommand) -> Option<String> {
    command
        .segments
        .iter()
        .find_map(|segment| check_builtin_blacklist(&segment.executable, &segment.args))
        .or_else(|| {
            let shape = TemplateShape::from_template(command.template.as_ref()?);
            if shape.has_holes() { check_template_blacklist(&shape) } else { None }
        })
}

/// The permissions of the configuration `context` names, loaded passively:
/// nothing is migrated, written, or prompted for.
pub fn load_permissions(context: &FileResolutionContext) -> CapturedPermissions {
    let snapshot = darkmatter::markdown::schemas::slots::session::load_passively(context).map_err(|error| error.to_string())?;
    snapshot.policy().map(Arc::clone).map_err(|error| error.to_string())
}

/// Whether an `exact` or `template` rule of `rules` names the program of a
/// segment of `shape` that holds a placeholder: such a rule may match the
/// value pre-flight reads, which DMLS does not, so it cannot rule it out.
fn reads_values(shape: &TemplateShape, rules: &ShellRuleSet) -> bool {
    let mut holed = shape.segments.iter().filter(|segment| {
        segment.args.iter().flat_map(|arg| &arg.parts).any(|part| matches!(part, ShapePart::Hole(_)))
    });
    holed.any(|segment| {
        rules.entries.iter().any(|rule| match rule {
            ShellRuleEntry::Exact(command) => command.split_whitespace().next() == Some(segment.program.as_str()),
            ShellRuleEntry::Template(rule) => rule.segments.iter().any(|named| named.program == segment.program),
            ShellRuleEntry::Prefix(_) => false,
        })
    })
}

/// The program and literal arguments of each segment of a shape with no
/// placeholder.
fn literal_segments(shape: &TemplateShape) -> Vec<(String, Vec<String>)> {
    shape
        .segments
        .iter()
        .map(|segment| {
            let args = segment.args.iter().filter_map(|arg| arg.as_literal().map(str::to_string)).collect();
            (segment.program.clone(), args)
        })
        .collect()
}

/// `segments` joined by `shape`'s operators, so a chain `template`
/// exclusion meets the chain it names.
fn filled_chain(shape: &TemplateShape, segments: &[(String, Vec<String>)]) -> FilledChain {
    FilledChain {
        segments: shape
            .segments
            .iter()
            .zip(segments)
            .map(|(authored, (program, args))| FilledSegment {
                operator: authored.operator,
                program: program.clone(),
                args: args.clone(),
                redirection: Default::default(),
            })
            .collect(),
    }
}

/// The locally tokenized segments of a command the template grammar rejects.
fn tokenized(command: &ParsedCommand) -> Vec<(String, Vec<String>)> {
    command
        .segments
        .iter()
        .map(|segment| (segment.executable.clone(), segment.args.clone()))
        .collect()
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
    use darkmatter::markdown::compose::expression::DeclaredTypes;

    /// [`verdict`] at a schema-free, unguarded site.
    fn judge(command: &ParsedCommand, permissions: &CapturedPermissions) -> PolicyVerdict {
        let declared = DeclaredTypes::new(None);
        verdict(command, &TypeScope::unguarded(&declared), permissions)
    }

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
        assert!(matches!(judge(&command, &permissions), PolicyVerdict::Denied(_)));
    }

    #[test]
    fn test_unknown_without_a_rule() {
        let (_home, permissions) = permissions(&[]);
        let command = parse_command("echo hello").expect("parses");
        assert_eq!(judge(&command, &permissions), PolicyVerdict::Unknown);
    }

    #[test]
    fn test_approved_when_preapproved() {
        let (_home, permissions) = permissions(&["prefix echo"]);
        let command = parse_command("echo hi").expect("parses");
        assert_eq!(judge(&command, &permissions), PolicyVerdict::Approved);
    }

    #[test]
    fn test_unknown_when_excluded_or_unloaded() {
        let (_home, excluded) = permissions(&["prefix echo", "!exact echo secret"]);
        let command = parse_command("echo secret").expect("parses");
        assert_eq!(judge(&command, &excluded), PolicyVerdict::Unknown);
        assert_eq!(judge(&command, &Err("invalid".to_string())), PolicyVerdict::Unknown);
    }

    #[test]
    fn test_chain_denied_if_any_link_dangerous() {
        let (_home, permissions) = permissions(&["prefix echo"]);
        let command = parse_command("echo ok && rm -rf /tmp/x").expect("parses");
        assert_eq!(command.segments.len(), 2);
        assert!(matches!(judge(&command, &permissions), PolicyVerdict::Denied(_)));
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
