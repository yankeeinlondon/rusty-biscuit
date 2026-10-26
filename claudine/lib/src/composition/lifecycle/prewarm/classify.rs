//! Decide whether a phrase is known at preparation time, and resolve it.
//!
//! A phrase is **early** when it is literal text, or every root it reads is
//! frontmatter (`fm`, or a bare name the binding view classifies as a document
//! property), `ctx`, or `env`. Every other root the lifecycle binding view
//! knows (a declared global such as `err`, or a late namespace such as
//! `current`) makes it **late**: it can only be synthesized when its event
//! fires. Roots come from Darkmatter's [`static_variable_reads`], which visits
//! every branch, so a late read in an untaken branch still makes the phrase
//! late.
//!
//! Any authored shell expansion or function call makes a phrase ineligible,
//! even where it would not run: some read-only functions probe a shell, the
//! filesystem, or a remote service, so nothing is executed to decide.

use std::collections::{BTreeSet, HashMap};

use darkmatter::markdown::compose::expression::{
    AuthoredMode, BindingView, Expr, RootClass, SpannedExpr, SpannedExprKind, prepare_value,
    scalar_string, static_variable_reads,
};
use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};
use serde_json::{Map, Value};

use super::super::bindings::{
    EnclosingLoop, LifecycleScope, LifecycleValues, runtime_bindings_enclosed,
};
use super::super::executor::evaluate_operand_in;

/// The frontmatter namespace root.
const FRONTMATTER_ROOT: &str = "fm";

/// The reserved namespaces other than `fm` whose values a document captures
/// once, before any event fires.
const CAPTURED_ROOTS: [&str; 2] = ["ctx", "env"];

/// An owned snapshot of what a phrase may be resolved against.
///
/// Built by the caller from the document's own captured state; the classifier
/// never captures context, reads live runtime state, or reads the process
/// environment.
#[derive(Debug, Clone)]
pub struct PrewarmInputs {
    /// The effective frontmatter the phrase's document was prepared with.
    pub(crate) frontmatter: Map<String, Value>,
    /// The `ctx`/`env` binding the phrase would be spoken with.
    pub(crate) context: DocumentContext,
    /// Top-level frontmatter keys a sequence injects per task. They are late
    /// even though they read as frontmatter.
    pub(crate) sequence_injected_keys: BTreeSet<String>,
}

impl PrewarmInputs {
    /// A prepared document's inputs: its effective frontmatter and the
    /// `ctx`/`env` binding its lifecycle events are spoken with.
    ///
    /// A non-object `frontmatter` reads as empty.
    #[must_use]
    pub fn captured(frontmatter: &Value, context: ComposeContext) -> Self {
        Self {
            frontmatter: frontmatter.as_object().cloned().unwrap_or_default(),
            context: DocumentContext::Captured(context),
            sequence_injected_keys: BTreeSet::new(),
        }
    }
}

/// Whether a document's `ctx`/`env` capture exists yet.
#[derive(Debug, Clone)]
pub(crate) enum DocumentContext {
    /// The document's own captured binding.
    Captured(ComposeContext),
    /// The document's capture does not exist yet (sequence preflight). The
    /// carried context only satisfies the evaluator and is never read: a
    /// phrase reading `ctx` or `env` is skipped, and reconsidered when the
    /// document is prepared.
    Pending(ComposeContext),
}

/// The outcome for one phrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Classification {
    /// Known now; carries the text playback would speak.
    Early(String),
    /// Not prepared ahead of its event.
    Skip(SkipReason),
}

/// Why a phrase is not prepared ahead of its event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkipReason {
    /// The authored text holds `$(` or a backtick.
    ShellExpansion,
    /// The phrase calls a function, in any branch.
    FunctionCall,
    /// The phrase reads a root only its event can supply.
    LateRoot(String),
    /// The phrase reads a frontmatter key a sequence injects per task.
    SequenceInjected(String),
    /// A computed frontmatter index or a whole-`fm` read could include a
    /// sequence-injected key.
    TaskDependentRead,
    /// The phrase reads `ctx` or `env` before the document's capture exists.
    ContextUnavailable,
    /// The phrase's expressions could not be recovered or parsed.
    Unparseable,
    /// Speculative resolution failed; the event resolves it again.
    ResolutionFailed,
    /// The phrase resolves to empty or whitespace-only text.
    Empty,
}

/// Classify a phrase spoken in `scope` and, when it is early, resolve it.
pub(crate) fn classify(scope: LifecycleScope, text: &Expr, inputs: &PrewarmInputs) -> Classification {
    match early_reads(scope, text, inputs) {
        Ok(view_and_globals) => resolve(text, view_and_globals, inputs),
        Err(reason) => Classification::Skip(reason),
    }
}

type Bindings = (
    std::sync::Arc<BindingView>,
    HashMap<String, darkmatter::markdown::compose::expression::RuntimeBinding<'static>>,
);

/// Check every eligibility rule without evaluating anything, returning the
/// scope's bindings for resolution when the phrase is early.
fn early_reads(scope: LifecycleScope, text: &Expr, inputs: &PrewarmInputs) -> Result<Bindings, SkipReason> {
    let authored = authored_text(text);
    if authored.contains("$(") || authored.contains('`') {
        return Err(SkipReason::ShellExpansion);
    }
    let prepared = prepare_value(&Value::String(authored), AuthoredMode::InterpolatedValue)
        .map_err(|_| SkipReason::Unparseable)?;
    if !matches!(text, Expr::StringLiteral(_)) {
        // A whole-value phrase reaches us parsed; its authored text is
        // rebuilt from `Display`. Only analyze it when that text parses back
        // to the very expression playback evaluates.
        match prepared.expressions() {
            [only] if only.expr().erase() == *text => {}
            _ => return Err(SkipReason::Unparseable),
        }
    }

    // `loop` is execution-dependent before a loop's first iteration; its
    // availability does not matter here, only that it is declared.
    let (view, globals) =
        runtime_bindings_enclosed(scope, LifecycleValues::default(), EnclosingLoop::Pending);
    let mut reads_context = false;
    for expression in prepared.expressions() {
        let expr = expression.expr();
        if contains_call(expr) {
            return Err(SkipReason::FunctionCall);
        }
        for read in static_variable_reads(expr) {
            let root = read.root();
            match view.classify_root(root) {
                RootClass::Namespace if root == FRONTMATTER_ROOT => {}
                RootClass::Namespace if CAPTURED_ROOTS.contains(&root) => reads_context = true,
                RootClass::Namespace | RootClass::Global(_) => {
                    return Err(SkipReason::LateRoot(root.to_string()));
                }
                RootClass::Document => {}
            }
        }
        if !inputs.sequence_injected_keys.is_empty() {
            let mut reads = Vec::new();
            frontmatter_reads(expr, &view, &mut reads);
            for read in reads {
                match read {
                    FrontmatterRead::Unbounded => return Err(SkipReason::TaskDependentRead),
                    FrontmatterRead::Key(key) if inputs.sequence_injected_keys.contains(&key) => {
                        return Err(SkipReason::SequenceInjected(key));
                    }
                    FrontmatterRead::Key(_) => {}
                }
            }
        }
    }
    if reads_context && matches!(inputs.context, DocumentContext::Pending(_)) {
        return Err(SkipReason::ContextUnavailable);
    }
    Ok((view, globals))
}

/// The phrase as an interpolated value: literal and template bodies are
/// authored text already; a whole-value expression is wrapped back into its
/// single span.
fn authored_text(text: &Expr) -> String {
    match text {
        Expr::StringLiteral(body) => body.clone(),
        other => format!("{{{{ {other} }}}}"),
    }
}

/// Resolve an early phrase exactly as lifecycle playback would.
fn resolve(text: &Expr, (view, globals): Bindings, inputs: &PrewarmInputs) -> Classification {
    let (DocumentContext::Captured(context) | DocumentContext::Pending(context)) = &inputs.context;
    let Ok(state) = EffectiveStateBuilder::new()
        .with_frontmatter(inputs.frontmatter.clone())
        .with_context(context.clone())
        .with_allow_ctx_override(true)
        .build()
    else {
        return Classification::Skip(SkipReason::ResolutionFailed);
    };
    // No function is called, so no file reference needs resolving.
    match evaluate_operand_in(text, &state, view, globals, None) {
        Ok(value) => {
            let spoken = scalar_string(&value);
            if spoken.trim().is_empty() {
                Classification::Skip(SkipReason::Empty)
            } else {
                Classification::Early(spoken)
            }
        }
        Err(_) => Classification::Skip(SkipReason::ResolutionFailed),
    }
}

/// Whether `expr` calls a function anywhere, including untaken branches.
fn contains_call(expr: &SpannedExpr) -> bool {
    match &expr.kind {
        SpannedExprKind::FunctionCall { .. } => true,
        SpannedExprKind::Variable(_)
        | SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => false,
        SpannedExprKind::ArrayLiteral(items) => items.iter().any(contains_call),
        SpannedExprKind::ObjectLiteral(entries) => entries.iter().any(|(_, value)| contains_call(value)),
        SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::Paren(inner)
        | SpannedExprKind::MemberAccess { base: inner, .. } => contains_call(inner),
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. } => contains_call(left) || contains_call(right),
        SpannedExprKind::Index { base, index } => contains_call(base) || contains_call(index),
        SpannedExprKind::Fallback { primary, fallback } => contains_call(primary) || contains_call(fallback),
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => contains_call(condition) || contains_call(then_branch) || contains_call(else_branch),
    }
}

/// A frontmatter dependency of an expression.
#[derive(Debug, PartialEq)]
enum FrontmatterRead {
    /// A known top-level key: `fm.key`, `fm["key"]`, or a bare `key`.
    Key(String),
    /// A computed `fm[…]` index or the whole `fm`: any key may be read.
    Unbounded,
}

/// Collect every frontmatter dependency of `expr`, in every branch.
fn frontmatter_reads(expr: &SpannedExpr, view: &BindingView, reads: &mut Vec<FrontmatterRead>) {
    let is_fm = |node: &SpannedExpr| matches!(&node.kind, SpannedExprKind::Variable(path) if path == FRONTMATTER_ROOT);
    match &expr.kind {
        SpannedExprKind::Variable(path) => {
            let mut segments = path.split('.');
            let root = segments.next().unwrap_or(path);
            if root == FRONTMATTER_ROOT {
                reads.push(match segments.next() {
                    Some(key) => FrontmatterRead::Key(key.to_string()),
                    None => FrontmatterRead::Unbounded,
                });
            } else if view.names_document_property(root) {
                reads.push(FrontmatterRead::Key(root.to_string()));
            }
        }
        SpannedExprKind::Index { base, index } if is_fm(base) => {
            reads.push(match &index.kind {
                SpannedExprKind::StringLiteral(key) => FrontmatterRead::Key(key.clone()),
                _ => FrontmatterRead::Unbounded,
            });
            frontmatter_reads(index, view, reads);
        }
        SpannedExprKind::MemberAccess { base, name } if is_fm(base) => {
            let key = name.split('.').next().unwrap_or(name);
            reads.push(FrontmatterRead::Key(key.to_string()));
        }
        SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => {}
        SpannedExprKind::ArrayLiteral(items) => {
            for item in items {
                frontmatter_reads(item, view, reads);
            }
        }
        SpannedExprKind::ObjectLiteral(entries) => {
            for (_, value) in entries {
                frontmatter_reads(value, view, reads);
            }
        }
        SpannedExprKind::FunctionCall { args, .. } => {
            for arg in args {
                frontmatter_reads(arg, view, reads);
            }
        }
        SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::Paren(inner)
        | SpannedExprKind::MemberAccess { base: inner, .. } => frontmatter_reads(inner, view, reads),
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. }
        | SpannedExprKind::Index { base: left, index: right }
        | SpannedExprKind::Fallback {
            primary: left,
            fallback: right,
        } => {
            frontmatter_reads(left, view, reads);
            frontmatter_reads(right, view, reads);
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            frontmatter_reads(condition, view, reads);
            frontmatter_reads(then_branch, view, reads);
            frontmatter_reads(else_branch, view, reads);
        }
    }
}

#[cfg(test)]
mod tests;
