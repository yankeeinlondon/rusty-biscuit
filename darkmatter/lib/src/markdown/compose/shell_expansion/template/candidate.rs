//! Discovery's per-site record of a shell command: its authored template,
//! where it was written, which of its placeholders are late, and whether it
//! may run as resolved bytes.
//!
//! Discovery keeps one [`ShellCandidate`] per command site, never merged by
//! bytes, because two sites that resolve to the same bytes can belong to
//! documents with different schemas and so have different template types.

use super::{
    ApprovalKey, CommandTemplate, HoleTiming, OfferedChoices, ShellPreApprovals, TemplateApprovalView,
    TemplateGrammarError, TemplateShape,
};
use crate::markdown::compose::expression::{Expr, ExpressionFinder, GuardOrigin, TypeScope, fm_namespace, parse};
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
///
/// A host that places commands where their values may change before they run
/// (a sequence task after the first) also sets
/// [`with_placeholders_template_required`](Self::with_placeholders_template_required):
/// every command with a placeholder is then [`ModeRequirement::TemplateRequired`],
/// even when all its values are known at approval.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DeferredShellRoots {
    roots: BTreeSet<String>,
    placeholders_template_required: bool,
}

impl DeferredShellRoots {
    pub fn new<I, S>(roots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            roots: roots.into_iter().map(Into::into).collect(),
            placeholders_template_required: false,
        }
    }

    /// The same roots, with every command that has a placeholder required to
    /// run as a template. Approval-time placeholders are still evaluated and
    /// checked for presence; only the resolved-byte mode is withdrawn.
    #[must_use]
    pub fn with_placeholders_template_required(mut self) -> Self {
        self.placeholders_template_required = true;
        self
    }

    /// Whether every command with a placeholder must run as a template.
    pub fn placeholders_template_required(&self) -> bool {
        self.placeholders_template_required
    }

    pub fn contains(&self, root: &str) -> bool {
        self.roots.contains(root)
    }

    /// Whether no root is declared late. Says nothing about
    /// [`placeholders_template_required`](Self::placeholders_template_required).
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.roots.iter().map(String::as_str)
    }

    /// The late roots `expression` reads: a declared root, or a key of
    /// `derived` (see [`derived_deferred_keys`]). `None` when it reads none,
    /// or does not parse. Static: nothing is evaluated.
    pub fn late_roots_of(&self, expression: &str, derived: &HashSet<String>) -> Option<Vec<String>> {
        if self.is_empty() && derived.is_empty() {
            return None;
        }
        let expr = parse(expression).ok()?;
        let mut roots = Vec::new();
        read_roots(&expr, &mut roots);
        let mut late: Vec<String> = roots
            .into_iter()
            .filter(|read| read.is_late(self, derived))
            .map(|read| read.root)
            .collect();
        late.dedup();
        (!late.is_empty()).then_some(late)
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
    /// The authored command, retained even when template grammar rejects it.
    pub authored_command: String,
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
    /// The valid template's authored command; byte-only sites retain their
    /// original text in [`authored_command`](Self::authored_command).
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
            prefix: (resolved || self.template.is_ok()) && self.program_words_fixed(),
        }
    }

    /// Whether every authored program word is literal, including when another
    /// part of the command is outside template grammar.
    pub fn program_words_fixed(&self) -> bool {
        match &self.template {
            Err(TemplateGrammarError::ProgramWordHole) => return false,
            // Native-shell adapters preserve line boundaries, while the template
            // grammar folds them into whitespace between arguments.
            Ok(_) if !self.authored_command.contains(['\n', '\r']) => return true,
            _ => {}
        }
        let holes = ExpressionFinder::scan_plain(&self.authored_command).expressions;
        let mut holes = holes.iter().peekable();
        let mut chars = self.authored_command.char_indices().peekable();
        let mut quote = None;
        let mut program = true;
        let mut word = false;
        while let Some((offset, ch)) = chars.next() {
            if let Some(hole) = holes.peek()
                && hole.start == offset
            {
                if program {
                    return false;
                }
                while chars.peek().is_some_and(|(offset, _)| *offset < hole.end) {
                    chars.next();
                }
                holes.next();
                word = true;
                continue;
            }
            if ch == '\\' && quote != Some('\'') {
                chars.next();
                word = true;
            } else if Some(ch) == quote {
                quote = None;
            } else if quote.is_some() {
                word = true;
            } else if matches!(ch, '\'' | '"') {
                quote = Some(ch);
                word = true;
            } else if matches!(ch, '\n' | '\r') {
                let line = self.authored_command[offset + ch.len_utf8()..]
                    .split(['\n', '\r']).next().unwrap_or("");
                if matches!(super::parse_authored(line), Err(TemplateGrammarError::ProgramWordHole)) {
                    return false;
                }
                program = true;
                word = false;
            } else if ch.is_whitespace() {
                if word { program = false; }
                word = false;
            } else if matches!(ch, '&' | '|' | ';') {
                program = true;
                word = false;
            } else {
                word = true;
            }
        }
        true
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

/// What discovery knows about the document that owns a command site. The
/// site's types (the owner's schema and host roots, narrowed by its enclosing
/// guards) are the [`TypeScope`] each [`build`](Self::build) call requires.
pub(crate) struct CandidateScope<'a> {
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
        self.deferred.late_roots_of(expression, self.derived)
    }

    /// Builds the candidate for one authored command at a site whose types
    /// are `types`.
    ///
    /// `evaluate` reads an approval-time placeholder against the document's
    /// discovery state; it returns `None` when the expression cannot be read
    /// there, which leaves the verdict to the compose pass.
    ///
    /// A placeholder whose value is `null` here waits for its command only
    /// when a guard in `types` proves that very expression present at this
    /// site: it becomes [`HoleTiming::Guarded`] and the command template
    /// required. A declaration never excuses a `null`, and a guard on one
    /// expression never excuses another. A late placeholder the guards prove
    /// present is [`HoleTiming::Guarded`] too, so no unconditional barrier
    /// reads it.
    ///
    /// ## Errors
    ///
    /// - [`ShellExpansionError::MissingValue`] for an approval-time
    ///   placeholder whose value is `null` and that no guard proves present.
    /// - [`ShellExpansionError::TemplateGrammar`] when a late or guarded
    ///   placeholder, or [`DeferredShellRoots::placeholders_template_required`],
    ///   requires template mode and the authored command is not a valid
    ///   template.
    pub(crate) fn build(
        &self,
        authored: &str,
        template: Result<CommandTemplate, TemplateGrammarError>,
        origin: ShellCommandOrigin,
        types: &TypeScope<'_>,
        evaluate: &dyn Fn(&Expr) -> Option<Value>,
    ) -> Result<ShellCandidate, ShellExpansionError> {
        let guard_of = |expression: &str| -> Option<GuardOrigin> {
            let expr = parse(expression).ok()?;
            types.type_of_expr(&expr)?.guard_presence().cloned()
        };
        let mut template = template.map(|mut template| {
            template.project_types(types);
            for placeholder in template.placeholders_mut() {
                if let Some(roots) = self.deferred_roots(&placeholder.expression) {
                    placeholder.timing = match guard_of(&placeholder.expression) {
                        Some(guard) => HoleTiming::Guarded { guard, roots },
                        None => HoleTiming::Deferred { roots },
                    };
                }
            }
            template
        });

        // Every placeholder of the command, valid grammar or not: a missing
        // value fails resolved-byte commands too.
        let holes: Vec<(String, bool)> = match &template {
            Ok(template) => template
                .placeholders()
                .map(|placeholder| (placeholder.expression.clone(), placeholder.timing.is_late()))
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

        let mut any_late = holes.iter().any(|(_, late)| *late);
        let mut guarded: Vec<(&str, GuardOrigin)> = Vec::new();
        for (expression, late) in &holes {
            if *late {
                continue;
            }
            let Ok(expr) = parse(expression) else {
                continue;
            };
            if matches!(evaluate(&expr), Some(Value::Null)) {
                let Some(guard) = guard_of(expression) else {
                    return Err(ShellExpansionError::MissingValue {
                        ctx: Box::new(self.ctx.clone()),
                        command: authored.to_string(),
                        expression: expression.clone(),
                        origin,
                    });
                };
                guarded.push((expression, guard));
                any_late = true;
            }
        }
        // Discovery keeps the authored expression; nothing it resolved for a
        // guarded hole stands in for the value read when the command runs.
        if let Ok(template) = &mut template {
            for placeholder in template.placeholders_mut() {
                if let Some((_, guard)) = guarded.iter().find(|(expression, _)| *expression == placeholder.expression) {
                    placeholder.timing = HoleTiming::Guarded {
                        guard: guard.clone(),
                        roots: Vec::new(),
                    };
                }
            }
        }

        let placed_late = self.deferred.placeholders_template_required() && !holes.is_empty();
        let requirement = if any_late || placed_late {
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
            authored_command: authored.to_string(),
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
pub fn derived_deferred_keys<'a>(
    frontmatter: impl Iterator<Item = (&'a String, &'a Value)>,
    deferred: &DeferredShellRoots,
    inherited: &HashSet<String>,
) -> HashSet<String> {
    let mut derived = inherited.clone();
    if deferred.is_empty() && derived.is_empty() {
        return derived;
    }
    let reads: Vec<(&String, Vec<RootRead>)> = frontmatter
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
            if roots.iter().any(|read| read.is_late(deferred, &derived)) {
                derived.insert((*key).clone());
                grew = true;
            }
        }
        if !grew {
            return derived;
        }
    }
}

/// The first path segment of every variable `value`'s expressions read, with
/// an `fm.` prefix removed: the keys [`derived_deferred_keys`] follows from
/// `value`, in authored order and with repeats.
///
/// Static, as [`derived_deferred_keys`] is.
pub fn value_read_roots(value: &Value) -> Vec<String> {
    let mut roots = Vec::new();
    value_roots(value, &mut roots);
    roots.into_iter().map(|read| read.root).collect()
}

/// Whether `expr` reads any of `roots` as a global, not through `fm.`.
pub(crate) fn reads_global_root(expr: &Expr, roots: &[&str]) -> bool {
    let mut reads = Vec::new();
    read_roots(expr, &mut reads);
    reads
        .iter()
        .any(|read| !read.frontmatter && roots.contains(&read.root.as_str()))
}

/// The first path segment of one variable an expression reads.
struct RootRead {
    root: String,
    /// Read through `fm.` (or `fm['…']`): a frontmatter key, never a host
    /// global of the same name.
    frontmatter: bool,
}

impl RootRead {
    /// A host-declared late root read as a global, or a frontmatter key
    /// derived from one read either way.
    fn is_late(&self, deferred: &DeferredShellRoots, derived: &HashSet<String>) -> bool {
        (!self.frontmatter && deferred.contains(&self.root)) || derived.contains(&self.root)
    }
}

fn value_roots(value: &Value, roots: &mut Vec<RootRead>) {
    match value {
        Value::String(text) => {
            for location in ExpressionFinder::find_all_plain(text) {
                crate::note_work!(DeferredRootScanParse);
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
fn read_roots(expr: &Expr, roots: &mut Vec<RootRead>) {
    match expr {
        Expr::Variable(path) => {
            let effective = fm_namespace::strip_fm_prefix(path);
            let root = effective.split('.').next().unwrap_or(effective);
            if !root.is_empty() {
                roots.push(RootRead {
                    root: root.to_string(),
                    frontmatter: effective.len() != path.len(),
                });
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
                roots.push(RootRead {
                    root: key.to_string(),
                    frontmatter: true,
                });
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

#[cfg(test)]
mod read_root_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn read_roots_follow_every_expression_at_any_depth_and_strip_fm() {
        let value = json!({
            "a": "{{ first.x }} and {{ fm.second }}",
            "b": ["plain", "{{ third + fm['fourth'] }}"],
            "c": 3,
        });

        assert_eq!(value_read_roots(&value), ["first", "second", "third", "fourth"]);
        assert!(value_read_roots(&json!("no expression")).is_empty());
    }
}
