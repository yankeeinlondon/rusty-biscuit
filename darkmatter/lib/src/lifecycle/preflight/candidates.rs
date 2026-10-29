//! What approval sees of a lifecycle: its fixed commands and its typed sites.

use std::path::Path;

use crate::markdown::Markdown;
use crate::markdown::compose::expression::Expr;
use crate::markdown::compose::shell_expansion::normalize_command;
use crate::markdown::compose::shell_expansion::template::{
    CommandSite, DeferredShellRoots, ModeRequirement, ShellCandidate, ShellPreApprovals, derived_deferred_keys,
    parse_authored,
};
use crate::markdown::compose::shell_expansion::tokenize::{ShellToken, tokenize};
use crate::markdown::compose::ShellCommandOrigin;

use crate::lifecycle::actions::{LifecycleActionKind, for_each_action};
use crate::lifecycle::shell_template::LifecycleShellTemplate;
use crate::lifecycle::{LifecycleConfig, LifecycleSignal, collect_lifecycle_shell_commands_for};

/// The lifecycle shell commands and sites one audit approves.
#[derive(Debug, Default)]
pub struct LifecycleShellAudit {
    /// Each command with known bytes and its authored property path.
    pub commands: Vec<(String, String)>,
    /// Each site with a placeholder, as an approval candidate.
    pub candidates: Vec<ShellCandidate>,
}

/// Every lifecycle shell command and site reachable from `signals`,
/// condition-blind: both branches of every conditional and every catch stack
/// are collected, because a guard may evaluate differently under document
/// states the audit cannot predict.
///
/// It reads the lifecycle as preflight fixed it and asks nothing; approval is
/// the host's.
pub fn collect_lifecycle_shell_audit(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
    source_file: &Path,
) -> LifecycleShellAudit {
    LifecycleShellAudit {
        commands: collect_lifecycle_shell_commands_for(lifecycle, signals),
        candidates: lifecycle_shell_candidates(lifecycle, signals, source_file),
    }
}

/// Every lifecycle shell site reachable from `signals` that holds a
/// placeholder, as an approval candidate: its resolved bytes (when they still
/// read as a command) and its typed shape.
pub fn lifecycle_shell_candidates(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
    source_file: &Path,
) -> Vec<ShellCandidate> {
    let mut candidates = Vec::new();
    for_each_lifecycle_shell_site(lifecycle, signals, &mut |site, property| {
        candidates.push(site.candidate(source_file, &property));
    });
    for signal in signals {
        for_each_action(lifecycle.stack(*signal), &mut |action| {
            let LifecycleActionKind::Shell(shell) = &action.kind else {
                return;
            };
            let (None, Some(authored), Expr::StringLiteral(bytes)) =
                (&shell.template, &shell.authored_command, &shell.command)
            else {
                return;
            };
            if !authored.contains("{{") {
                return;
            }
            let parts = words(bytes).unwrap_or_else(|| vec![bytes.clone()]);
            let Some((program, args)) = parts.split_first() else {
                return;
            };
            candidates.push(ShellCandidate {
                authored_command: authored.clone(),
                site: CommandSite {
                    source_file: source_file.to_path_buf(),
                    origin: ShellCommandOrigin::Frontmatter { key: action.path.field("command").to_string(), line: None },
                    owner: None,
                },
                template: parse_authored(authored),
                resolved: Some(vec![normalize_command(program, args)]),
                requirement: ModeRequirement::AnyMode,
            });
        });
    }
    candidates
}

/// `raw`'s words, or `None` when it does not tokenize or holds a chain
/// operator or redirection.
fn words(raw: &str) -> Option<Vec<String>> {
    let context = biscuit_terminal::errors::SourceContext::new(
        std::path::PathBuf::from("<harness-shell>"),
        std::path::PathBuf::from("<harness-shell>"),
        raw.to_string(),
    );
    tokenize(raw, &context)
        .ok()?
        .into_iter()
        .map(|token| match token {
            ShellToken::Word(word) => Some(word),
            _ => None,
        })
        .collect()
}

/// Record on each lifecycle shell site the mode `approved` grants it: an
/// exact member runs its bytes, a shape member its template, and a site that
/// is neither runs in no mode.
pub fn settle_lifecycle_shell_modes(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
    approved: &ShellPreApprovals,
    policy_root: Option<&Path>,
) {
    for_each_lifecycle_shell_site(lifecycle, signals, &mut |site, _property| {
        site.settle(approved, policy_root);
    });
}

/// Visit every `shell` command and `set` value with a typed template,
/// condition-blind, with its authored property.
fn for_each_lifecycle_shell_site(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
    visit: &mut impl FnMut(&LifecycleShellTemplate, String),
) {
    for signal in LifecycleSignal::ALL.into_iter().filter(|signal| signals.contains(signal)) {
        for_each_action(lifecycle.stack(signal), &mut |action| match &action.kind {
            LifecycleActionKind::Shell(shell) => {
                if let Some(site) = &shell.template {
                    visit(site, action.path.field("command").to_string());
                }
            }
            LifecycleActionKind::RuntimeSet(set) => {
                for (key, value) in set.shell_values() {
                    if let Some(site) = &value.template {
                        visit(site, action.path.map_key(key.as_str()).to_string());
                    }
                }
            }
            _ => {}
        });
    }
}

/// `roots` extended with the keys `markdown`'s authored frontmatter derives
/// from them, so a derived input stays late after frontmatter interpolation
/// replaces its expression.
pub fn lifecycle_document_roots(markdown: &Markdown, roots: &DeferredShellRoots) -> DeferredShellRoots {
    let authored: serde_json::Map<String, serde_json::Value> =
        markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
    let derived = derived_deferred_keys(authored.iter(), roots, &Default::default());
    let extended = DeferredShellRoots::new(roots.iter().map(str::to_string).chain(derived));
    if roots.placeholders_template_required() { extended.with_placeholders_template_required() } else { extended }
}
