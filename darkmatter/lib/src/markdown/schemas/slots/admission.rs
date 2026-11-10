//! Passive admission of a schema slot: which bindings and functions its
//! expression may reach.
//!
//! A schema describes documents, so a slot may depend only on the schema's
//! own context, never on the document being validated. The walk visits
//! every node of the expression, both branches of a ternary, every `||`
//! operand, and every callback body, whether or not evaluation would reach
//! it, and refuses:
//!
//! - `fm` and any bare identifier (which falls through to a document
//!   property), and host globals;
//! - `ctx.cwd` (the launch directory), the document identity keys
//!   `ctx.self`, `ctx.last_updated`, `ctx.hash`, the per-run nonces `ctx.id`
//!   and `ctx.sid`, and a bare or computed `ctx` that could reach them;
//! - `current.*` and `local.*` outside a callback body;
//! - a call to a function whose slot class is
//!   [`SlotClass::Rejected`] (nested composition, a login shell, a provider
//!   API, an ICMP probe).
//!
//! Allowed: `ctx.<key>` for any other key, `env`, the literals, a callback's
//! own parameters, and `current.*`/`local.*` inside a callback body. Nothing
//! here reads, evaluates, or resolves.

use simplified_schema::expression::{SpannedCallArgument, SpannedExpr, SpannedExprKind};
use simplified_schema::span::SourceSpan;

use super::codes::{SLOT_FORBIDDEN_BINDING_CODE, SLOT_FORBIDDEN_FUNCTION_CODE};
use crate::markdown::compose::expression::functions::{SlotClass, slot_class};

/// `ctx` keys a slot may never read.
const FORBIDDEN_CTX_KEYS: &[(&str, &str)] = &[
    ("cwd", "the launch directory is not part of a schema's context"),
    ("self", "the document's identity is not part of a schema's context"),
    ("last_updated", "the document's identity is not part of a schema's context"),
    ("hash", "the document's identity is not part of a schema's context"),
    ("id", "a per-run nonce would make every resolution different"),
    ("sid", "a per-run nonce would make every resolution different"),
];

/// One refused part of a slot's expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rejection {
    /// [`SLOT_FORBIDDEN_BINDING_CODE`] or [`SLOT_FORBIDDEN_FUNCTION_CODE`].
    pub(crate) code: &'static str,
    /// What is refused and why.
    pub(crate) message: String,
    /// The refused node, in the parsed expression's coordinates.
    pub(crate) span: SourceSpan,
}

/// What the passive walk found in one slot's expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotClassReport {
    /// The greatest class of any call the expression can reach;
    /// [`SlotClass::Pure`] for an expression without calls.
    pub(crate) class: SlotClass,
    /// Every refused read or call, in source order.
    pub(crate) rejections: Vec<Rejection>,
}

/// Walks `expr` passively, joining the class of every call it can reach and
/// collecting every refused binding and call.
pub(crate) fn slot_class_report(expr: &SpannedExpr) -> SlotClassReport {
    let mut walk = Walk { class: SlotClass::Pure, rejections: Vec::new() };
    walk.expr(expr, &Scope::default());
    walk.rejections.sort_by_key(|rejection| (rejection.span.start, rejection.span.end));
    SlotClassReport { class: walk.class, rejections: walk.rejections }
}

#[derive(Debug, Clone, Default)]
struct Scope<'e> {
    /// Names the enclosing callbacks bind, nearest last.
    locals: Vec<&'e str>,
}

impl Scope<'_> {
    fn in_callback(&self) -> bool {
        !self.locals.is_empty()
    }
}

struct Walk {
    class: SlotClass,
    rejections: Vec<Rejection>,
}

impl Walk {
    fn expr<'e>(&mut self, expr: &'e SpannedExpr, scope: &Scope<'e>) {
        match &expr.kind {
            SpannedExprKind::Variable(path) => self.variable(path, &expr.span, scope),
            SpannedExprKind::StringLiteral(_) | SpannedExprKind::NumberLiteral(_) | SpannedExprKind::BoolLiteral(_) => {}
            SpannedExprKind::ArrayLiteral(items) => items.iter().for_each(|item| self.expr(item, scope)),
            SpannedExprKind::ObjectLiteral(entries) => entries.iter().for_each(|(_, value)| self.expr(value, scope)),
            SpannedExprKind::UnaryNot(inner) | SpannedExprKind::UnaryMinus(inner) | SpannedExprKind::Paren(inner) => {
                self.expr(inner, scope);
            }
            SpannedExprKind::Binary { left, right, .. }
            | SpannedExprKind::Comparison { left, right, .. }
            | SpannedExprKind::LogicalOr { left, right } => {
                self.expr(left, scope);
                self.expr(right, scope);
            }
            SpannedExprKind::Ternary { condition, then_branch, else_branch } => {
                self.expr(condition, scope);
                self.expr(then_branch, scope);
                self.expr(else_branch, scope);
            }
            SpannedExprKind::MemberAccess { base, name } => match bare_variable(base) {
                Some(path) => self.variable(&format!("{path}.{name}"), &expr.span, scope),
                None => self.expr(base, scope),
            },
            SpannedExprKind::Index { base, index } => match (bare_variable(base), &index.kind) {
                (Some(path), SpannedExprKind::StringLiteral(key)) => {
                    self.variable(&format!("{path}.{key}"), &expr.span, scope);
                }
                (Some(path), _) => {
                    self.computed_member(path, &base.span, scope);
                    self.expr(index, scope);
                }
                (None, _) => {
                    self.expr(base, scope);
                    self.expr(index, scope);
                }
            },
            SpannedExprKind::FunctionCall { name, args } => {
                self.call(name, &expr.span);
                args.iter().for_each(|arg| self.expr(arg, scope));
            }
            SpannedExprKind::CallbackCall { name, args } => {
                self.call(name, &expr.span);
                for arg in args {
                    match arg {
                        SpannedCallArgument::Value(value) => self.expr(value, scope),
                        SpannedCallArgument::Callback(callback) => {
                            let mut inner = scope.clone();
                            match &callback.pattern {
                                simplified_schema::expression::SpannedBindingPattern::Identifier(name) => {
                                    inner.locals.push(&name.value);
                                }
                                simplified_schema::expression::SpannedBindingPattern::Fields(fields) => {
                                    inner.locals.extend(fields.iter().map(|field| field.value.as_str()));
                                }
                            }
                            self.expr(&callback.body, &inner);
                        }
                    }
                }
            }
        }
    }

    fn call(&mut self, name: &str, span: &SourceSpan) {
        match slot_class(name) {
            Some(SlotClass::Rejected) => self.rejections.push(Rejection {
                code: SLOT_FORBIDDEN_FUNCTION_CODE,
                message: format!(
                    "a schema slot cannot call {name}(): it composes a document, runs a shell, calls a provider, or \
                     sends a network probe"
                ),
                span: span.clone(),
            }),
            Some(class) => self.class = self.class.max(class),
            // An unknown name fails when evaluated; admission has nothing to
            // judge.
            None => {}
        }
    }

    fn variable(&mut self, path: &str, span: &SourceSpan, scope: &Scope<'_>) {
        let mut segments = path.split('.');
        let root = segments.next().unwrap_or(path);
        if scope.locals.contains(&root) {
            return;
        }
        let refusal = match root {
            "null" | "true" | "false" | "env" => None,
            "ctx" => match segments.next() {
                None => Some("a whole `ctx` reaches keys a schema slot may not read; name the key".to_string()),
                Some(key) => FORBIDDEN_CTX_KEYS
                    .iter()
                    .find(|(forbidden, _)| *forbidden == key)
                    .map(|(_, why)| format!("a schema slot cannot read `ctx.{key}`: {why}")),
            },
            "current" | "local" if scope.in_callback() => None,
            "current" | "local" => {
                Some(format!("a schema slot reads `{root}.*` only inside a callback that binds it"))
            }
            "fm" => Some("a schema slot cannot read the document's frontmatter (`fm`)".to_string()),
            _ => Some(format!(
                "a schema slot cannot read `{root}`: a bare name reads the document or a host global, never the \
                 schema's own context"
            )),
        };
        if let Some(message) = refusal {
            self.rejections.push(Rejection { code: SLOT_FORBIDDEN_BINDING_CODE, message, span: span.clone() });
        }
    }

    /// `root[expr]`: the key is not known until evaluation.
    fn computed_member(&mut self, root_path: &str, span: &SourceSpan, scope: &Scope<'_>) {
        if root_path == "ctx" {
            self.rejections.push(Rejection {
                code: SLOT_FORBIDDEN_BINDING_CODE,
                message: "a computed `ctx[...]` key could reach a key a schema slot may not read; name the key"
                    .to_string(),
                span: span.clone(),
            });
        } else {
            self.variable(root_path, span, scope);
        }
    }
}

/// The variable path `expr` reads directly, through parentheses.
fn bare_variable(expr: &SpannedExpr) -> Option<&str> {
    match &expr.kind {
        SpannedExprKind::Variable(path) => Some(path),
        SpannedExprKind::Paren(inner) => bare_variable(inner),
        _ => None,
    }
}
