//! Discovery's per-site record of a shell command: its authored template,
//! where it was written, which of its placeholders are late, and whether it
//! may run as resolved bytes.
//!
//! Discovery keeps one [`ShellCandidate`] per command site, never merged by
//! bytes, because two sites that resolve to the same bytes can belong to
//! documents with different schemas and so have different template types.

use super::{
    ApprovalKey, ArgPart, CommandTemplate, HoleTiming, OfferedChoices, ShellPreApprovals,
    TemplateApprovalView, TemplateGrammarError, TemplateShape,
};
use crate::markdown::compose::expression::{Expr, ExpressionFinder, fm_namespace, parse};
use crate::markdown::compose::shell_expansion::types::{ShellCommandOrigin, ShellExpansionError};
use biscuit_terminal::errors::SourceContext;
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

/// The roots a host declares late for shell approval: values the run
/// produces, which do not exist when commands are approved.
///
/// A placeholder that reads one, directly or through a frontmatter value
/// derived from one, is [`HoleTiming::Deferred`]: discovery does not evaluate
/// it, and the command can be approved only as a template. Declaring a root
/// here does not make it readable; lexical availability stays with the
/// host's binding view.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DeferredShellRoots {
    roots: BTreeSet<String>,
}

impl DeferredShellRoots {
    pub fn new<I, S>(roots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            roots: roots.into_iter().map(Into::into).collect(),
        }
    }

    pub fn contains(&self, root: &str) -> bool {
        self.roots.contains(root)
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.roots.iter().map(String::as_str)
    }
}

/// Where a command was authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSite {
    /// The file the command is in (a URL's text for a remote child).
    pub source_file: PathBuf,
    pub origin: ShellCommandOrigin,
    /// The owning document's source key: its canonical path or URL. `None`
    /// for in-memory content. Its schema typed the placeholders.
    pub owner: Option<String>,
}

/// Which execution modes a candidate may be approved for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeRequirement {
    /// Resolved bytes or a template.
    AnyMode,
    /// Only a template: a placeholder is late, or the host placed the command
    /// where its values may change before it runs. An exact approval never
    /// covers it.
    TemplateRequired,
}

/// One discovered shell command site.
#[derive(Debug, Clone)]
pub struct ShellCandidate {
    pub site: CommandSite,
    /// The authored command parsed as a template, typed against the owner's
    /// schema, with each hole's timing. An error keeps the site eligible for
    /// resolved-byte approval where that mode is allowed.
    pub template: Result<CommandTemplate, TemplateGrammarError>,
    /// The normalized bytes of each chain segment as discovery resolved them,
    /// for a site that may run as resolved bytes. `None` when a value is late
    /// or the site could not be matched to its resolved form.
    pub resolved: Option<Vec<String>>,
    pub requirement: ModeRequirement,
}

impl ShellCandidate {
    /// The command as authored.
    pub fn authored(&self) -> Option<&str> {
        self.template.as_ref().ok().map(|template| template.authored.as_str())
    }

    /// The shape a template approval of this site records: `Some` when the
    /// grammar is valid and at least one argument holds a placeholder.
    pub fn shape(&self) -> Option<TemplateShape> {
        let template = self.template.as_ref().ok()?;
        let shape = TemplateShape::from_template(template);
        shape.has_holes().then_some(shape)
    }

    /// The prompt view of [`shape`](Self::shape).
    pub fn approval_view(&self) -> Option<TemplateApprovalView> {
        let template = self.template.as_ref().ok()?;
        self.shape().map(|_| TemplateApprovalView::of(template))
    }

    /// The choices a prompt may offer for this site.
    pub fn offered(&self) -> OfferedChoices {
        let resolved = self.requirement == ModeRequirement::AnyMode && self.resolved.is_some();
        OfferedChoices {
            exact: resolved,
            shape: self.shape().is_some(),
            prefix: resolved || self.template.is_ok(),
        }
    }

    /// Whether `approvals` authorize this site: its resolved bytes when it may
    /// run as resolved bytes, otherwise (or additionally) its shape.
    pub fn authorized_by(&self, approvals: &ShellPreApprovals) -> bool {
        let bytes = self.requirement == ModeRequirement::AnyMode
            && self
                .resolved
                .as_ref()
                .is_some_and(|keys| keys.iter().all(|key| approvals.contains_exact(key)));
        bytes || self.shape().is_some_and(|shape| approvals.contains_template(&shape))
    }

    /// The session-cache key of this site's template approval.
    pub fn template_key(&self, policy_root: Option<&std::path::Path>) -> Option<ApprovalKey> {
        self.shape().map(|shape| ApprovalKey::template(policy_root, shape))
    }

    /// The command text for diagnostics: the authored text, or the resolved
    /// bytes when the grammar failed.
    pub fn display_command(&self) -> String {
        match (&self.template, &self.resolved) {
            (Ok(template), _) => template.authored.clone(),
            (Err(_), Some(keys)) => keys.join(" && "),
            (Err(_), None) => String::new(),
        }
    }
}

/// What discovery knows about the document that owns a command site.
pub(crate) struct CandidateScope<'a> {
    pub schema: Option<&'a crate::markdown::schemas::SimplifiedSchema>,
    pub host_globals: Vec<String>,
    pub deferred: &'a DeferredShellRoots,
    /// Frontmatter keys whose value reads a late root, directly or through
    /// another such key.
    pub derived: &'a HashSet<String>,
    pub source_file: PathBuf,
    pub owner: Option<String>,
    pub ctx: &'a SourceContext,
}

impl CandidateScope<'_> {
    /// The late roots `expression` reads, or `None` when it reads none (or
    /// does not parse; the compose pass reports that).
    fn deferred_roots(&self, expression: &str) -> Option<Vec<String>> {
        if self.deferred.is_empty() && self.derived.is_empty() {
            return None;
        }
        let expr = parse(expression).ok()?;
        let mut roots = Vec::new();
        read_roots(&expr, &mut roots);
        let mut late: Vec<String> = roots
            .into_iter()
            .filter(|root| self.deferred.contains(root) || self.derived.contains(root))
            .collect();
        late.dedup();
        (!late.is_empty()).then_some(late)
    }

    /// Builds the candidate for one authored command.
    ///
    /// `evaluate` reads an approval-time placeholder against the document's
    /// discovery state; it returns `None` when the expression cannot be read
    /// there, which leaves the verdict to the compose pass.
    ///
    /// ## Errors
    ///
    /// - [`ShellExpansionError::MissingValue`] for an approval-time
    ///   placeholder whose value is `null`.
    /// - [`ShellExpansionError::TemplateGrammar`] when a late placeholder
    ///   requires template mode and the authored command is not a valid
    ///   template.
    pub(crate) fn build(
        &self,
        authored: &str,
        template: Result<CommandTemplate, TemplateGrammarError>,
        origin: ShellCommandOrigin,
        evaluate: &dyn Fn(&Expr) -> Option<Value>,
    ) -> Result<ShellCandidate, ShellExpansionError> {
        let host_globals: Vec<&str> = self.host_globals.iter().map(String::as_str).collect();
        let template = template.map(|mut template| {
            template.project_types(self.schema, &host_globals);
            for segment in &mut template.segments {
                for arg in &mut segment.args {
                    for part in &mut arg.parts {
                        if let ArgPart::Hole(placeholder) = part
                            && let Some(roots) = self.deferred_roots(&placeholder.expression)
                        {
                            placeholder.timing = HoleTiming::Deferred { roots };
                        }
                    }
                }
            }
            template
        });

        // Every placeholder of the command, valid grammar or not: a missing
        // value fails resolved-byte commands too.
        let holes: Vec<(String, bool)> = match &template {
            Ok(template) => template
                .placeholders()
                .map(|placeholder| {
                    (
                        placeholder.expression.clone(),
                        matches!(placeholder.timing, HoleTiming::Deferred { .. }),
                    )
                })
                .collect(),
            Err(_) => ExpressionFinder::scan_plain(authored)
                .expressions
                .into_iter()
                .map(|location| {
                    let late = self.deferred_roots(&location.expression).is_some();
                    (location.expression, late)
                })
                .collect(),
        };

        let any_late = holes.iter().any(|(_, late)| *late);
        for (expression, late) in &holes {
            if *late {
                continue;
            }
            let Ok(expr) = parse(expression) else {
                continue;
            };
            if matches!(evaluate(&expr), Some(Value::Null)) {
                return Err(ShellExpansionError::MissingValue {
                    ctx: Box::new(self.ctx.clone()),
                    command: authored.to_string(),
                    expression: expression.clone(),
                    origin,
                });
            }
        }

        let requirement = if any_late {
            ModeRequirement::TemplateRequired
        } else {
            ModeRequirement::AnyMode
        };
        if requirement == ModeRequirement::TemplateRequired
            && let Err(reason) = &template
        {
            return Err(ShellExpansionError::TemplateGrammar {
                ctx: Box::new(self.ctx.clone()),
                command: authored.to_string(),
                reason: reason.clone(),
                origin,
            });
        }

        Ok(ShellCandidate {
            site: CommandSite {
                source_file: self.source_file.clone(),
                origin,
                owner: self.owner.clone(),
            },
            template,
            resolved: None,
            requirement,
        })
    }
}

/// The frontmatter keys of `frontmatter` that read a late root, directly or
/// through another such key (including `inherited` keys of a parent).
///
/// Static: values are scanned for expressions and never evaluated.
pub(crate) fn derived_deferred_keys<'a>(
    frontmatter: impl Iterator<Item = (&'a String, &'a Value)>,
    deferred: &DeferredShellRoots,
    inherited: &HashSet<String>,
) -> HashSet<String> {
    let mut derived = inherited.clone();
    if deferred.is_empty() && derived.is_empty() {
        return derived;
    }
    let reads: Vec<(&String, Vec<String>)> = frontmatter
        .map(|(key, value)| {
            let mut roots = Vec::new();
            value_roots(value, &mut roots);
            (key, roots)
        })
        .collect();
    loop {
        let mut grew = false;
        for (key, roots) in &reads {
            if derived.contains(*key) {
                continue;
            }
            if roots.iter().any(|root| deferred.contains(root) || derived.contains(root)) {
                derived.insert((*key).clone());
                grew = true;
            }
        }
        if !grew {
            return derived;
        }
    }
}

fn value_roots(value: &Value, roots: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            for location in ExpressionFinder::find_all_plain(text) {
                if let Ok(expr) = parse(&location.expression) {
                    read_roots(&expr, roots);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| value_roots(item, roots)),
        Value::Object(map) => map.values().for_each(|item| value_roots(item, roots)),
        _ => {}
    }
}

/// The first path segment of every variable `expr` reads, with an `fm.`
/// prefix removed so `fm.out` and `out` name the same key.
fn read_roots(expr: &Expr, roots: &mut Vec<String>) {
    match expr {
        Expr::Variable(path) => {
            let effective = fm_namespace::strip_fm_prefix(path);
            let root = effective.split('.').next().unwrap_or(effective);
            if !root.is_empty() {
                roots.push(root.to_string());
            }
        }
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
        Expr::UnaryNot(inner)
        | Expr::UnaryMinus(inner)
        | Expr::Paren(inner)
        | Expr::MemberAccess { base: inner, .. } => read_roots(inner, roots),
        Expr::Fallback { primary, fallback } => {
            read_roots(primary, roots);
            read_roots(fallback, roots);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            read_roots(condition, roots);
            read_roots(then_branch, roots);
            read_roots(else_branch, roots);
        }
        Expr::Comparison { left, right, .. } | Expr::Binary { left, right, .. } => {
            read_roots(left, roots);
            read_roots(right, roots);
        }
        Expr::Index { base, index } => {
            if let Some(key) = fm_namespace::literal_index_key(base, index) {
                roots.push(key.to_string());
                return;
            }
            read_roots(base, roots);
            read_roots(index, roots);
        }
        Expr::FunctionCall { args, .. } => args.iter().for_each(|arg| read_roots(arg, roots)),
        Expr::ArrayLiteral(items) => items.iter().for_each(|item| read_roots(item, roots)),
        Expr::ObjectLiteral(entries) => entries.iter().for_each(|(_, value)| read_roots(value, roots)),
    }
}
