//! Fixing each lifecycle shell site at preflight: its resolved bytes, its
//! typed template, or both.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Map, Value};

use crate::markdown::compose::expression::{
    EvaluationSession, Expr, ExpressionError, ExpressionFinder, GuardOrigin, ResolutionContext, TypeScope,
};
use crate::markdown::compose::shell_expansion::template::{
    CommandTemplate, DeferredShellRoots, DocumentShellValues, HoleEvaluator, HoleTiming, LookupEvaluator,
    ShellPreApprovals, derived_deferred_keys, fill_template, parse_authored, resolve_late_values,
};
use crate::markdown::compose::subtree::SubtreeCompose;
use crate::markdown::compose::{
    ComposeContext, ContextBuildError, EffectiveState, EffectiveStateBuilder, ResolvedShellValue, ShellCommandOrigin,
    ShellExpansionError, authored_shell_template,
};
use crate::markdown::schemas::EffectiveSchema;
use crate::markdown::{MarkdownError, SourceRef};

use super::{PreflightError, ShellAuditBindings, unavailable_root};
use crate::lifecycle::actions::{LifecycleActionKind, SetShellValue, shell_value_error_message, walk_scoped};
use crate::lifecycle::registry::EventRegistry;
use crate::lifecycle::shell_template::{
    LifecycleShellTemplate, lifecycle_declared_types, placeholder_guard, reads_as_command, retime_guarded,
    typed_template,
};
use crate::lifecycle::{LifecycleConfig, LifecycleSignal};

/// What a sequence task's document carries into its lifecycle shell walk.
///
/// With no deferred roots the walk fixes every site at preflight as a
/// standalone document's.
#[derive(Clone, Copy, Debug)]
pub struct ShellWalkOptions<'a> {
    /// Roots read late, when the task runs, with the keys derived from them.
    pub deferred_shell_roots: &'a DeferredShellRoots,
    /// Lexical group variables of the referencing sequence.
    pub lifecycle_group: Option<&'a Map<String, Value>>,
    /// The complete lexical sequence root.
    pub sequence_binding: Option<&'a Value>,
    /// The sequence's closed approvals; a template outside them is refused.
    pub pre_approved_commands: Option<&'a ShellPreApprovals>,
    /// Validate late values now, before task setup, and record them.
    pub validate_deferred_shell_values: bool,
    /// The task's validated late values.
    pub document_shell_values: Option<&'a DocumentShellValues>,
}

/// One document's lifecycle shell sites, ready to be fixed at preflight.
///
/// Fixing them resolves the `{{ }}` interpolation inside every lifecycle
/// `shell` command and whole-value `$( … )` `set` value, stamping the resolved
/// bytes back into the parsed action so the approved command equals the
/// executed command. A command with a placeholder also keeps its authored text
/// as a template typed against `owner_schema`; the approval settles which of
/// the two runs (see [`LifecycleShellTemplate`]).
///
/// Bytes resolve in the host's lifecycle shell approval scope
/// ([`ShellAuditBindings`]): a late-binding read is refused, in any branch,
/// because its value does not yet exist.
///
/// Each site is typed in the scope of the guards that enclose it: a `then`
/// body under its `when` (as do actions authored beside the `when`), an
/// `else` body under its negation, nested conditionals composing. A `set`
/// value is checked under the scope that reaches the `set`; its destinations
/// then invalidate what guards proved about them for every later action,
/// across the join after a conditional too. Every event starts from the
/// declarations alone. A placeholder that reads `null` fails preflight
/// unless a guard proves that very placeholder present; then the command
/// keeps its authored text, has no bytes, and runs only as its template,
/// which reads and checks the value when the action is reached. No guard,
/// predicate, or mutation is evaluated.
///
/// Non-string command expressions and literals with no interpolation span are
/// left untouched: there is nothing to stamp.
pub struct LifecycleShellSites<'a> {
    pub effective_frontmatter: &'a Value,
    pub context: &'a ComposeContext,
    pub source_path: &'a Path,
    pub registry: &'a EventRegistry,
    pub owner_schema: Option<&'a EffectiveSchema>,
    pub bindings: &'a dyn ShellAuditBindings,
    pub options: Option<ShellWalkOptions<'a>>,
}

impl LifecycleShellSites<'_> {
    /// Fixes every lifecycle shell site of `lifecycle`, in both branches of
    /// every conditional: approval never depends on which branch runs.
    ///
    /// `resolution` builds the document's expression resolution context; it
    /// is called after the early-binding state is built.
    ///
    /// ## Errors
    ///
    /// [`PreflightError::LifecycleShellResolution`] when a command reads a
    /// late-binding global, fails to parse, calls an unknown function, reads a
    /// missing value no enclosing guard proves present, or must run as its
    /// template and is not a valid one. The error names the authored property
    /// path (e.g. `failure[0].else[1].shell.command`) and the raw command.
    pub fn stamp(
        &self,
        lifecycle: &mut LifecycleConfig,
        resolution: impl FnOnce() -> Result<ResolutionContext, ContextBuildError>,
    ) -> Result<(), PreflightError> {
        let frontmatter: HashMap<String, Value> = self
            .effective_frontmatter
            .as_object()
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let state = EffectiveStateBuilder::new()
            .with_frontmatter(frontmatter.clone())
            .with_context(self.context.clone())
            // A deferred lifecycle subtree never defines `ctx`; downgrade any
            // pathological `ctx` shape to a warning rather than aborting pre-flight.
            .with_allow_ctx_override(true)
            .build()
            .map_err(|e| PreflightError::PreFlightStateBuildFailed { source: e })?;

        let walk = Walk { sites: self, state: &state, resolution_ctx: &resolution()?, frontmatter: &frontmatter };

        // Each event's stack starts from the document's declarations alone: no
        // guard fact crosses an event, a retry, a resume, or a proxy handoff.
        let declared = lifecycle_declared_types(self.owner_schema, self.registry);
        for signal in LifecycleSignal::ALL {
            walk_scoped(lifecycle.stack_mut(signal), TypeScope::unguarded(&declared), &mut |action, scope| {
                let prefix = action.path.to_string();
                match &mut action.kind {
                    LifecycleActionKind::RuntimeSet(set) => set.shell_values_mut().try_for_each(|(key, shell)| {
                        let property = format!("{prefix}.{key}");
                        if let Some(site) = walk.sequence_lifecycle_template(&shell.authored, Some(key), &property, scope)? {
                            shell.resolved =
                                ResolvedShellValue::template_only(key, &shell.authored, "sequence template", self.context);
                            shell.template = Some(site);
                            return Ok(());
                        }
                        walk.resolve_set_shell_value(key, shell, &property, scope)
                    }),
                    LifecycleActionKind::Shell(shell) => {
                        if shell.authored_command.is_none()
                            && let Expr::StringLiteral(raw) = &shell.command
                        {
                            shell.authored_command = Some(raw.clone());
                        }
                        if let Expr::StringLiteral(raw) = &shell.command
                            && let Some(site) =
                                walk.sequence_lifecycle_template(raw, None, &format!("{prefix}.command"), scope)?
                        {
                            shell.template = Some(site);
                            shell.pre_resolved = false;
                            return Ok(());
                        }
                        let property = format!("{prefix}.command");
                        let command = walk.resolve_shell_text(&shell.command, &property, scope)?;
                        let on_error = match &shell.on_error {
                            Some(on_error) => walk.resolve_shell_text(on_error, &format!("{prefix}.on_error"), scope)?,
                            None => None,
                        };
                        // `on_error` is display text, not a command: a value a guard
                        // proves present but that is missing now renders as it does in
                        // any message.
                        if let (Some(on_error), Some(resolved)) = (&mut shell.on_error, on_error) {
                            *on_error = Expr::StringLiteral(resolved.bytes);
                        }
                        shell.pre_resolved = true;
                        let Some(command) = command else {
                            shell.template = None;
                            return Ok(());
                        };
                        let Expr::StringLiteral(raw) = &shell.command else {
                            unreachable!("only a string literal command resolves");
                        };
                        if command.guarded.is_empty() {
                            // The bytes run only when approval proves them; a
                            // shape approval runs the authored template instead.
                            let bytes = reads_as_command(&command.bytes).then(|| vec![command.bytes.clone()]);
                            shell.template =
                                typed_template(raw, scope).map(|template| LifecycleShellTemplate::new(template, bytes));
                            shell.command = Expr::StringLiteral(command.bytes);
                            return Ok(());
                        }
                        // A guard proves a missing value present: the command keeps
                        // its authored text, has no bytes, and runs only as its
                        // template, reading that value when the action is reached.
                        let template =
                            guarded_template(raw, raw, &command.guarded, &property, self.source_path, scope)?;
                        shell.template = Some(LifecycleShellTemplate::new(template, None));
                        Ok(())
                    }
                    _ => Ok(()),
                }
            })?;
        }
        Ok(())
    }
}

/// The state one [`LifecycleShellSites::stamp`] reads at every site.
struct Walk<'a> {
    sites: &'a LifecycleShellSites<'a>,
    state: &'a EffectiveState,
    resolution_ctx: &'a ResolutionContext,
    frontmatter: &'a HashMap<String, Value>,
}

/// The text one lifecycle shell field resolved to at pre-flight.
struct ShellText {
    /// The resolved text. A missing placeholder in `guarded` reads as empty.
    bytes: String,
    /// The placeholders that are missing now but that a guard enclosing the
    /// site proves present, each with that guard. When this is not empty the
    /// bytes are not the command the action runs.
    guarded: Vec<(String, GuardOrigin)>,
}

impl Walk<'_> {
    /// Fixes one lifecycle `set` shell value's command bytes.
    ///
    /// The authored text resolves against the same early-binding lookup as a
    /// `shell` action's command, with late-binding roots refused, and
    /// Darkmatter then fixes every pipeline the value can run. Those bytes are
    /// what approval sees and what the action runs; nothing re-interpolates
    /// them later.
    ///
    /// A value whose command holds a placeholder also keeps its authored
    /// command as a template typed in `scope`, the type scope where the `set`
    /// sits, so approval by shape can run it as one. When the resolved text no
    /// longer reads as a command (a value inserted command syntax), or a value
    /// a guard in `scope` proves present is missing now, it has no bytes and
    /// may run only as that template.
    fn resolve_set_shell_value(
        &self,
        key: &str,
        shell: &mut SetShellValue,
        property: &str,
        scope: &TypeScope<'_>,
    ) -> Result<(), PreflightError> {
        let source_path = self.sites.source_path;
        let context = self.sites.context;
        let resolution = self.resolve_shell_text(&Expr::StringLiteral(shell.authored.clone()), property, scope)?;
        let inner = authored_shell_template(
            &shell.authored,
            key,
            &biscuit_terminal::errors::SourceContext::new(
                source_path.to_path_buf(),
                source_path.to_path_buf(),
                shell.authored.clone(),
            ),
        )
        .map(|(inner, _)| inner);
        if let Some(resolution) = resolution.as_ref().filter(|resolution| !resolution.guarded.is_empty()) {
            // A guard proves a missing value present: the value runs only as its
            // template, reading that value when the action is reached.
            let inner = inner.unwrap_or_else(|| shell.authored.clone());
            let template = guarded_template(&inner, &shell.authored, &resolution.guarded, property, source_path, scope)?;
            shell.resolved = ResolvedShellValue::template_only(
                key,
                &shell.authored,
                "a guarded value is read when its action is reached",
                context,
            );
            shell.template = Some(LifecycleShellTemplate::new(template, None));
            return Ok(());
        }
        let resolved = resolution.map_or_else(|| shell.authored.clone(), |resolution| resolution.bytes);
        let template = inner.and_then(|inner| typed_template(&inner, scope));
        let resolution_failure = |error: ShellExpansionError| PreflightError::LifecycleShellResolution {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            raw: shell.authored.clone(),
            message: shell_value_error_message(&error),
            source: Some(Box::new(MarkdownError::ShellExpansion(Box::new(error)))),
        };
        let (value, site) = match ResolvedShellValue::resolve(
            key,
            &shell.authored,
            &resolved,
            || self.frontmatter.clone(),
            context,
            self.resolution_ctx.clone(),
        ) {
            Ok(value) => {
                let site = template.map(|template| LifecycleShellTemplate::new(template, Some(value.commands())));
                (value, site)
            }
            Err(error) => {
                let template_only = template.as_ref().and_then(|_| {
                    ResolvedShellValue::template_only(key, &shell.authored, &shell_value_error_message(&error), context)
                });
                match (template_only, template) {
                    (Some(value), Some(template)) => (value, Some(LifecycleShellTemplate::new(template, None))),
                    _ => return Err(resolution_failure(error)),
                }
            }
        };
        shell.resolved = Some(value);
        shell.template = site;
        Ok(())
    }

    /// Resolves the `{{ }}` spans inside one shell-command [`Expr`], or `None`
    /// when it is not a string literal carrying a span, so there is nothing to
    /// resolve.
    ///
    /// The command is first checked passively against the lifecycle shell
    /// approval scope, in every branch, so a late global (`err`, `timing`,
    /// `group`) or a late namespace (`current`) is refused by name before
    /// anything is evaluated; the bytes are then resolved through a session in
    /// the same scope, where `loop` follows the enclosing loop: a shell's bytes
    /// are fixed per iteration, so an iteration's values are known.
    ///
    /// A placeholder that reads `null` fails with a missing value unless a
    /// guard in `scope`, the type scope at the site, proves that placeholder
    /// present; such a placeholder is returned in [`ShellText::guarded`]
    /// instead. A non-null declaration never excuses a `null`.
    fn resolve_shell_text(
        &self,
        expr: &Expr,
        property: &str,
        scope: &TypeScope<'_>,
    ) -> Result<Option<ShellText>, PreflightError> {
        let source_path = self.sites.source_path;
        let Expr::StringLiteral(raw) = expr else {
            return Ok(None);
        };
        if !raw.contains("{{") {
            return Ok(None);
        }

        if let Some(finding) = self.sites.bindings.approval_findings(raw).into_iter().next() {
            let message = match unavailable_root(&finding) {
                Some(root) => format!(
                    "late-binding reference `{root}` is not available in shell commands; \
                     shell commands are resolved at pre-flight (before any event fires), so only \
                     early-binding values (`fm.*`, `ctx.*`, `env.*`, read-side functions) may be \
                     used here"
                ),
                None => finding.to_string(),
            };
            return Err(PreflightError::LifecycleShellResolution {
                source_path: source_path.to_path_buf(),
                property: property.to_string(),
                raw: raw.clone(),
                message,
                source: Some(Box::new(approval_failure(raw, finding))),
            });
        }

        let (view, globals) = self.sites.bindings.approval_bindings();

        // The command and each of its placeholders, as a whole value, resolve in
        // one subtree so the placeholders keep their types: a `null` is missing,
        // while an empty string, zero, and `false` are present.
        let placeholders: Vec<String> =
            ExpressionFinder::scan_plain(raw).expressions.into_iter().map(|location| location.expression).collect();
        let value = Value::Array(
            std::iter::once(Value::String(raw.clone()))
                .chain(placeholders.iter().map(|expression| Value::String(format!("{{{{ {expression} }}}}"))))
                .collect(),
        );
        let composed = SubtreeCompose::new(&value, self.state)
            .with_resolution_context(self.resolution_ctx.clone())
            .with_globals(globals)
            .with_binding_view(view)
            .compose()
            .map_err(|e| PreflightError::LifecycleShellResolution {
                source_path: source_path.to_path_buf(),
                property: property.to_string(),
                raw: raw.clone(),
                message: e.to_string(),
                source: Some(Box::new(e)),
            })?;
        let Value::Array(mut composed) = composed else {
            unreachable!("an array subtree composes to an array");
        };
        // Before a loop's first iteration `loop` has no value yet: it is unknown
        // here, not missing, and each iteration's preparation checks it.
        let pending_loop = self.sites.bindings.loop_pending().then(|| DeferredShellRoots::new(["loop"]));
        let mut guarded: Vec<(String, GuardOrigin)> = Vec::new();
        for (expression, read) in placeholders.iter().zip(composed.iter().skip(1)) {
            let pending =
                pending_loop.as_ref().is_some_and(|roots| roots.late_roots_of(expression, &Default::default()).is_some());
            if pending || !read.is_null() || guarded.iter().any(|(seen, _)| seen == expression) {
                continue;
            }
            let Some(guard) = placeholder_guard(expression, scope) else {
                return Err(missing_value(raw, expression, property, source_path));
            };
            guarded.push((expression.clone(), guard));
        }
        let resolved = composed.swap_remove(0);
        let bytes = match resolved {
            Value::String(s) => s,
            other => other.to_string(),
        };
        Ok(Some(ShellText { bytes, guarded }))
    }

    /// Sequence lifecycle sites retain authored expressions and the step
    /// barrier's values.
    ///
    /// `scope` is the type scope at the site. A late placeholder a guard there
    /// proves present, and an approval-time one that is missing now but a
    /// guard proves present, is [`HoleTiming::Guarded`]: neither the step
    /// barrier nor this discovery reads it; the reached action does.
    fn sequence_lifecycle_template(
        &self,
        raw: &str,
        set_key: Option<&str>,
        property: &str,
        scope: &TypeScope<'_>,
    ) -> Result<Option<LifecycleShellTemplate>, PreflightError> {
        let source = self.sites.source_path;
        let Some(options) = self.sites.options.filter(|options| !options.deferred_shell_roots.is_empty()) else {
            return Ok(None);
        };
        let roots = options.deferred_shell_roots;
        let context = biscuit_terminal::errors::SourceContext::new(source.to_path_buf(), source.to_path_buf(), raw.to_string());
        let inner = match set_key {
            Some(key) => authored_shell_template(raw, key, &context).map(|(inner, _)| inner),
            None => Some(raw.to_string()),
        };
        let Some(inner) = inner else { return Ok(None) };
        if !inner.contains("{{") {
            return Ok(None);
        }
        if let Some(finding) = self
            .sites
            .bindings
            .approval_findings(&inner)
            .into_iter()
            .find(|finding| unavailable_root(finding).is_none_or(|root| !roots.contains(root)))
        {
            return Err(PreflightError::LifecycleShellResolution {
                source_path: source.to_path_buf(),
                property: property.to_string(),
                raw: raw.to_string(),
                message: finding.to_string(),
                source: Some(Box::new(approval_failure(raw, finding))),
            });
        }
        let derived = derived_deferred_keys(self.state.data().iter(), roots, &Default::default());
        // Resolved-byte eligibility must not depend on the narrower template grammar.
        let reads_late = ExpressionFinder::scan_plain(&inner)
            .expressions
            .iter()
            .any(|location| roots.late_roots_of(&location.expression, &derived).is_some());
        if !roots.placeholders_template_required() && !reads_late {
            return Ok(None);
        }
        let Some(mut template) = typed_template(&inner, scope) else {
            return Err(PreflightError::LifecycleShellResolution {
                source_path: source.to_path_buf(),
                property: property.to_string(),
                raw: raw.to_string(),
                message: "invalid shell template grammar".to_string(),
                source: None,
            });
        };
        for placeholder in template.placeholders_mut() {
            if let Some(late) = roots.late_roots_of(&placeholder.expression, &derived) {
                placeholder.timing = match placeholder_guard(&placeholder.expression, scope) {
                    Some(guard) => HoleTiming::Guarded { guard, roots: late },
                    None => HoleTiming::Deferred { roots: late },
                };
            }
        }
        let (view, globals) = self.sites.bindings.task_setup_bindings(options.lifecycle_group, options.sequence_binding);
        let session = EvaluationSession::associate(view, self.state, globals)
            .expect("task bindings agree")
            .with_resolution_context(self.resolution_ctx.clone());
        let evaluator = LookupEvaluator::new(&session);
        // An approval-time value that is missing now but that a guard proves
        // present is read when its action is reached, not here.
        let guarded: Vec<(String, GuardOrigin)> = template
            .placeholders()
            .filter(|placeholder| matches!(placeholder.timing, HoleTiming::ApprovalTime))
            .filter(|placeholder| evaluator.evaluate(&placeholder.expression).is_ok_and(|value| value.is_null()))
            .filter_map(|placeholder| {
                Some((placeholder.expression.clone(), placeholder_guard(&placeholder.expression, scope)?))
            })
            .collect();
        retime_guarded(&mut template, &guarded);
        let mut site = LifecycleShellTemplate::new(template, None);
        let candidate = site.candidate(source, property);
        if let Some(approved) = options.pre_approved_commands
            && !candidate.authorized_by(approved)
        {
            return Err(PreflightError::LifecycleShellResolution {
                source_path: source.to_path_buf(),
                property: property.to_string(),
                raw: raw.to_string(),
                message: "the authored template was not approved at sequence preflight".to_string(),
                source: None,
            });
        }
        let origin = candidate.site.origin.clone();
        let fail = |error: crate::markdown::compose::shell_expansion::template::HoleError| {
            let message = error.to_string();
            PreflightError::LifecycleShellResolution {
                source_path: source.to_path_buf(),
                property: property.to_string(),
                raw: raw.to_string(),
                message,
                source: Some(Box::new(MarkdownError::ShellExpansion(Box::new(
                    error.into_shell_error(&context, raw, &origin),
                )))),
            }
        };
        let mut known = site.template().clone();
        for placeholder in known.placeholders_mut() {
            placeholder.timing = match &placeholder.timing {
                HoleTiming::Deferred { .. } => HoleTiming::ApprovalTime,
                HoleTiming::ApprovalTime => HoleTiming::Deferred { roots: Vec::new() },
                // Read only when its guarded action is reached.
                guarded @ HoleTiming::Guarded { .. } => guarded.clone(),
            };
        }
        resolve_late_values([(property, &known)], &evaluator).map_err(|error| fail(error.error))?;
        let saved = options.document_shell_values.and_then(|values| values.get(&candidate));
        let values = if options.validate_deferred_shell_values {
            let late = resolve_late_values([(property, site.template())], &evaluator).map_err(|error| fail(error.error))?;
            let values = late.site(property).cloned().unwrap_or_default();
            if let Some(snapshot) = options.document_shell_values {
                snapshot.insert(&candidate, values.clone());
            }
            Some(values)
        } else {
            saved
        };
        // Validate known values without evaluating deferred holes during
        // discovery. A guarded hole is read only by its reached action, and
        // filling would read it now, so such a site is filled only there.
        let has_guarded =
            site.template().placeholders().any(|placeholder| matches!(placeholder.timing, HoleTiming::Guarded { .. }));
        if options.validate_deferred_shell_values && !has_guarded {
            fill_template(site.template(), &evaluator, values.as_ref()).map_err(fail)?;
        }
        if let Some(values) = values {
            site = site.with_late_values(values);
        }
        Ok(Some(site))
    }
}

/// The pre-flight failure for a lifecycle shell placeholder that reads a
/// missing value no guard proves present.
fn missing_value(raw: &str, expression: &str, property: &str, source_path: &Path) -> PreflightError {
    let error = ShellExpansionError::MissingValue {
        ctx: Box::new(biscuit_terminal::errors::SourceContext::new(
            source_path.to_path_buf(),
            source_path.to_path_buf(),
            raw.to_string(),
        )),
        command: raw.to_string(),
        expression: expression.to_string(),
        origin: ShellCommandOrigin::Frontmatter { key: property.to_string(), line: None },
    };
    PreflightError::LifecycleShellResolution {
        source_path: source_path.to_path_buf(),
        property: property.to_string(),
        raw: raw.to_string(),
        message: format!(
            "shell command reads `{expression}`, which is missing; a shell command never runs with a missing value"
        ),
        source: Some(Box::new(MarkdownError::ShellExpansion(Box::new(error)))),
    }
}

/// The command `inner` (authored as `raw`) as a template typed in `scope`,
/// each placeholder in `guarded` read only when its guarded action is reached.
///
/// ## Errors
///
/// [`PreflightError::LifecycleShellResolution`] carrying a `TemplateGrammar`
/// cause when `inner` is not a valid template: a command that must run as its
/// template can run no other way.
fn guarded_template(
    inner: &str,
    raw: &str,
    guarded: &[(String, GuardOrigin)],
    property: &str,
    source_path: &Path,
    scope: &TypeScope<'_>,
) -> Result<CommandTemplate, PreflightError> {
    let mut template = match parse_authored(inner) {
        Ok(template) => template,
        Err(reason) => {
            let error = ShellExpansionError::TemplateGrammar {
                ctx: Box::new(biscuit_terminal::errors::SourceContext::new(
                    source_path.to_path_buf(),
                    source_path.to_path_buf(),
                    inner.to_string(),
                )),
                command: inner.to_string(),
                reason,
                origin: ShellCommandOrigin::Frontmatter { key: property.to_string(), line: None },
            };
            return Err(PreflightError::LifecycleShellResolution {
                source_path: source_path.to_path_buf(),
                property: property.to_string(),
                raw: raw.to_string(),
                message: format!(
                    "a guard proves `{}` present but it is missing now, so the command must run as its \
                     template, and it is not a valid template",
                    guarded.first().map_or("", |(expression, _)| expression.as_str())
                ),
                source: Some(Box::new(MarkdownError::ShellExpansion(Box::new(error)))),
            });
        }
    };
    template.project_types(scope);
    retime_guarded(&mut template, guarded);
    Ok(template)
}

/// A passive approval finding as the error composition would have raised for
/// `raw`, so callers read one typed cause either way.
pub fn approval_failure(raw: &str, cause: ExpressionError) -> MarkdownError {
    MarkdownError::Interpolation {
        key: None,
        expression: raw.to_string(),
        source: Box::new(SourceRef::Effective { rendered: raw.to_string(), origin_key: None }),
        cause: Box::new(cause),
    }
}
