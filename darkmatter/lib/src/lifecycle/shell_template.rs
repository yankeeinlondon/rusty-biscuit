//! Lifecycle shell sites that may run as their typed template.
//!
//! A lifecycle `shell` command, or a whole-value `$( … )` in a `set`, whose
//! authored text holds a placeholder is resolved at pre-flight to bytes and is
//! also kept as its authored template, each placeholder typed against the
//! owning document's schema as the guards enclosing the site narrow it
//! ([`LifecycleShellTemplate`]). Approval decides which of the two runs, and
//! records it on the site ([`LifecycleShellTemplate::settle`]):
//!
//! - its bytes are approved (an `exact` rule, a cached resolved-byte decision,
//!   a `prefix` rule, or the exact/prefix/once prompt choices): the action
//!   runs those bytes, exactly as before templates existed;
//! - otherwise its shape is approved (a `template` rule, a cached shape
//!   decision, or the "command shape" choice): the action runs its template.
//!   Each placeholder is read when the action executes, checked against its
//!   declared type and any guard refinement, and passed as exactly one
//!   argument to a program started directly, never through a shell.
//!
//! A site whose placeholder is missing at pre-flight, but which an enclosing
//! guard proves present, has no bytes: that placeholder is
//! [`HoleTiming::Guarded`] and the site runs only as its template, failing
//! with a missing value if the selected action still reads `null`.
//!
//! Sequence hosts withdraw resolved-byte eligibility from later tasks and late
//! reads, and attach the step barrier's validated values with `with_late_values`.
//! These sites always require template membership.
//!
//! A site approval has not settled runs in neither mode. The record is shared
//! by every clone of the parsed lifecycle, so a guard that copied the
//! configuration before its audit sees the audit's answer.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::registry::EventRegistry;
use crate::markdown::compose::ShellCommandOrigin;
use crate::markdown::compose::expression::{DeclaredTypes, GuardOrigin, TypeScope};
use crate::markdown::compose::shell_expansion::template::{
    CommandSite, CommandTemplate, HoleTiming, ModeRequirement, ShellCandidate, ShellPreApprovals, SiteValues,
    TemplateShape, parse_authored,
};

/// How an approved lifecycle shell site runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleShellMode {
    /// The bytes pre-flight resolved, through the system shell.
    ResolvedBytes,
    /// The authored template, filled from the state current when the action
    /// executes. `policy_root` holds the user blacklist the filled command is
    /// checked against before it starts.
    Template {
        /// The shell policy root the site was approved under.
        policy_root: Option<PathBuf>,
    },
}

/// A lifecycle shell site's typed template and the mode its approval chose.
#[derive(Debug, Clone)]
pub struct LifecycleShellTemplate {
    template: Arc<CommandTemplate>,
    resolved: Option<Vec<String>>,
    mode: Arc<Mutex<Option<LifecycleShellMode>>>,
    late: Option<Arc<SiteValues>>,
}

/// The approval record is not part of what was authored.
impl PartialEq for LifecycleShellTemplate {
    fn eq(&self, other: &Self) -> bool {
        self.template == other.template && self.resolved == other.resolved
    }
}

impl LifecycleShellTemplate {
    /// A site not yet approved in either mode.
    ///
    /// `resolved` holds the approval keys of the bytes pre-flight resolved
    /// for it, or `None` when a value made them unreadable as a command, so
    /// the site may run only as its template.
    pub fn new(template: CommandTemplate, resolved: Option<Vec<String>>) -> Self {
        Self {
            template: Arc::new(template),
            resolved,
            mode: Arc::new(Mutex::new(None)),
            late: None,
        }
    }

    /// A site whose shape a host approved elsewhere: it runs as `template`.
    ///
    /// For a host that approves lifecycle stacks outside
    /// [`settle`](Self::settle), such as a sequence task's `setup:` and
    /// `teardown:`.
    pub fn approved(template: CommandTemplate, policy_root: Option<PathBuf>) -> Self {
        let site = Self::new(template, None);
        site.record(Some(LifecycleShellMode::Template { policy_root }));
        site
    }

    /// This site, filling the holes `late` holds without evaluating them:
    /// the values a host validated at its step barrier
    /// ([`resolve_late_values`](crate::markdown::compose::shell_expansion::template::resolve_late_values)).
    /// Every other hole is still read when the action executes.
    #[must_use]
    pub fn with_late_values(mut self, late: SiteValues) -> Self {
        self.late = Some(Arc::new(late));
        self
    }

    /// The late values the site fills from, when a host supplied them.
    pub fn late_values(&self) -> Option<&SiteValues> {
        self.late.as_deref()
    }

    /// The authored command with its typed placeholders.
    pub fn template(&self) -> &CommandTemplate {
        &self.template
    }

    /// The shape a template approval of this site records.
    pub fn shape(&self) -> TemplateShape {
        TemplateShape::from_template(&self.template)
    }

    /// The approval keys of the site's resolved bytes; `None` when it may run
    /// only as its template.
    pub fn resolved(&self) -> Option<&[String]> {
        self.resolved.as_deref()
    }

    /// The mode approval chose, or `None` while the site is not approved.
    pub fn mode(&self) -> Option<LifecycleShellMode> {
        self.mode.lock().map(|mode| mode.clone()).unwrap_or(None)
    }

    /// The site as an approval candidate.
    pub fn candidate(&self, source_file: &Path, property: &str) -> ShellCandidate {
        ShellCandidate {
            authored_command: self.template.authored.clone(),
            site: CommandSite {
                source_file: source_file.to_path_buf(),
                origin: ShellCommandOrigin::Frontmatter {
                    key: property.to_string(),
                    line: None,
                },
                owner: None,
            },
            template: Ok(self.template.as_ref().clone()),
            resolved: self.resolved.clone(),
            requirement: match self.resolved {
                Some(_) => ModeRequirement::AnyMode,
                None => ModeRequirement::TemplateRequired,
            },
        }
    }

    /// Record the mode `approvals` grant: the resolved bytes when every key
    /// is an exact member, otherwise the template when its shape is a
    /// template member, otherwise none, and the site refuses to run.
    pub fn settle(&self, approvals: &ShellPreApprovals, policy_root: Option<&Path>) {
        let bytes = self
            .resolved
            .as_ref()
            .is_some_and(|keys| keys.iter().all(|key| approvals.contains_exact(key)));
        let mode = if bytes {
            Some(LifecycleShellMode::ResolvedBytes)
        } else if approvals.contains_template(&self.shape()) {
            Some(LifecycleShellMode::Template {
                policy_root: policy_root.map(Path::to_path_buf),
            })
        } else {
            None
        };
        self.record(mode);
    }

    fn record(&self, mode: Option<LifecycleShellMode>) {
        if let Ok(mut slot) = self.mode.lock() {
            *slot = mode;
        }
    }
}

/// The declared layer a lifecycle shell site of a document whose effective
/// schema is `schema` is typed against: its authored projection and the
/// recursive definitions that projection reaches, with the registry's
/// [host globals](EventRegistry::host_globals) as host roots. A placeholder
/// reading one is typed `string`.
pub fn lifecycle_declared_types<'a>(
    schema: Option<&'a crate::markdown::schemas::EffectiveSchema>,
    registry: &'a EventRegistry,
) -> DeclaredTypes<'a> {
    let declared = DeclaredTypes::new(schema.and_then(|schema| schema.simplified.as_ref()))
        .with_host_globals(registry.host_globals());
    match schema {
        Some(schema) => declared.with_definitions(schema.definitions()),
        None => declared,
    }
}

/// The authored command `raw` as a template typed in `scope`, the type scope
/// at its site, when it is a valid template with at least one placeholder.
///
/// A placeholder a guard enclosing the site narrows carries that refinement,
/// so the site's approval shape follows the guard, and its value is checked
/// against the refinement when the command is filled.
pub fn typed_template(raw: &str, scope: &TypeScope<'_>) -> Option<CommandTemplate> {
    let mut template = parse_authored(raw).ok()?;
    template.placeholders().next()?;
    template.project_types(scope);
    Some(template)
}

/// The guard in `scope` that proves the placeholder `expression` itself
/// present, if one does. A declaration is not a guard, and a guard on one
/// expression proves nothing about another.
pub fn placeholder_guard(expression: &str, scope: &TypeScope<'_>) -> Option<GuardOrigin> {
    let expr = crate::markdown::compose::expression::parse(expression).ok()?;
    scope.type_of_expr(&expr)?.guard_presence().cloned()
}

/// Mark each placeholder of `template` whose expression `guarded` names as
/// [`HoleTiming::Guarded`] by that guard: it is read and checked only when its
/// guarded action is reached, never at approval or by a barrier.
pub fn retime_guarded(template: &mut CommandTemplate, guarded: &[(String, GuardOrigin)]) {
    for placeholder in template.placeholders_mut() {
        if let Some((_, guard)) = guarded.iter().find(|(expression, _)| *expression == placeholder.expression) {
            placeholder.timing = HoleTiming::Guarded {
                guard: guard.clone(),
                roots: Vec::new(),
            };
        }
    }
}

/// Whether any lifecycle `shell` command or whole-value `set` `$( … )`
/// holds a placeholder, so pre-flight types one against the owner's schema.
pub fn lifecycle_shell_has_placeholders(lifecycle: &super::LifecycleConfig) -> bool {
    let mut found = false;
    for signal in super::LifecycleSignal::ALL {
        super::actions::for_each_action(lifecycle.stack(signal), &mut |action| match &action.kind {
            super::actions::LifecycleActionKind::Shell(shell) => {
                found |= matches!(&shell.command, crate::markdown::compose::expression::Expr::StringLiteral(raw) if raw.contains("{{"));
            }
            super::actions::LifecycleActionKind::RuntimeSet(set) => {
                found |= set.shell_values().any(|(_, value)| value.authored.contains("{{"));
            }
            _ => {}
        });
    }
    found
}

/// Whether resolved shell bytes still read as the command grammar a template
/// accepts. A value that inserted a pipe, `;`, or an unbalanced quote makes
/// them unreadable, and the site may then run only as its template.
pub fn reads_as_command(bytes: &str) -> bool {
    use crate::markdown::compose::shell_expansion::tokenize::{parse_pipeline, tokenize};
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("<lifecycle-shell>"),
        PathBuf::from("<lifecycle-shell>"),
        bytes.to_string(),
    );
    tokenize(bytes, &ctx).is_ok_and(|tokens| parse_pipeline(&tokens, &ctx).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unguarded(raw: &str) -> Option<CommandTemplate> {
        let declared = lifecycle_declared_types(None, crate::lifecycle::registry::tests::agent_host());
        typed_template(raw, &TypeScope::unguarded(&declared))
    }

    fn site(resolved: Option<&str>) -> LifecycleShellTemplate {
        LifecycleShellTemplate::new(
            unguarded("tool {{ x }}").expect("a template"),
            resolved.map(|bytes| vec![bytes.to_string()]),
        )
    }

    #[test]
    fn exact_bytes_win_then_the_shape_then_nothing() {
        let shaped = site(Some("tool a"));
        let mut approvals = ShellPreApprovals::new();
        approvals.insert_template(shaped.shape());
        shaped.settle(&approvals, None);
        assert_eq!(shaped.mode(), Some(LifecycleShellMode::Template { policy_root: None }));

        approvals.insert_exact("tool a");
        shaped.settle(&approvals, None);
        assert_eq!(shaped.mode(), Some(LifecycleShellMode::ResolvedBytes));

        shaped.settle(&ShellPreApprovals::new(), None);
        assert_eq!(shaped.mode(), None);
    }

    #[test]
    fn a_template_only_site_never_runs_as_bytes_and_clones_share_the_answer() {
        let template_only = site(None);
        let copy = template_only.clone();
        let mut approvals = ShellPreApprovals::new();
        approvals.insert_exact("tool a");
        template_only.settle(&approvals, None);
        assert_eq!(copy.mode(), None);

        approvals.insert_template(template_only.shape());
        template_only.settle(&approvals, Some(Path::new("/policy")));
        assert_eq!(
            copy.mode(),
            Some(LifecycleShellMode::Template {
                policy_root: Some(PathBuf::from("/policy"))
            })
        );
        assert_eq!(
            template_only.candidate(Path::new("doc.md"), "start[0].shell.command").requirement,
            ModeRequirement::TemplateRequired
        );
    }

    #[test]
    fn data_that_adds_command_syntax_is_not_a_command() {
        assert!(reads_as_command("tool 'a b'"));
        assert!(!reads_as_command("tool a; rm -rf ~"));
        assert!(!reads_as_command("tool a | cat"));
        assert!(unguarded("tool plain").is_none());
    }
}
