//! The undeclared-member check: a member read whose base has a known object
//! type that does not declare it.
//!
//! A selector is reported when every non-null alternative of its base's type
//! at the read is a closed object and none declares the name
//! ([`declares_member`]), and the read does not handle its absence by the
//! rules variable reads follow ([`static_member_reads`]). An unknown, open,
//! pattern-key, scalar, or unresolved alternative keeps the read silent, and
//! a chain stops at its first undeclared or unknown step, so `server.nmae.x`
//! reports `nmae` once.
//!
//! Bases are typed as the call check types them ([`expression_type`] under
//! the branch facts at the read, [`branch_scope`]), so narrowing, callback
//! item types, `try(...)` unions, and literal `load_data` contracts apply
//! with no member-specific rule. Namespace wrappers (`fm`, `local`),
//! reserved namespaces, host globals, and `null` are never checked; a value
//! selected through `fm` or `local` is.
//!
//! The walk is static and passive: every branch and callback body is
//! checked, nothing is evaluated, and nothing is read.

use std::collections::HashMap;

use simplified_schema::expression::ast::{CallArgument, Expr, SpannedCallArgument, SpannedExpr, SpannedExprKind};

use super::absence::{StaticMemberRead, branch_scope, static_member_reads, variable_root_segments};
use super::binding::NULL_ROOT;
use super::call_check::expression_type;
use super::diagnostics::{ExpressionDiagnostic, Severity, UNDECLARED_MEMBER_CODE};
use super::fm_namespace::literal_index_key;
use super::scope::{Member, MemberDeclaration, Shape, TypeScope, VariableClass, declares_member, member_shape};
use crate::markdown::span::SourceSpan;

/// One member read the known object type of its base does not declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFinding {
    /// The selected name: one dotted segment, or a bracket key verbatim
    /// (`a.b` for `server["a.b"]`).
    pub member: String,
    /// The authored text of the base the member is read from: `server`,
    /// `fm.server`, `server.a`, `(server)`, `load_data("h.yaml")`.
    pub subject: String,
    /// The member name's byte span in the parsed text; a bracket key's
    /// includes its quotes.
    pub span: SourceSpan,
    /// The access node's position in [`spanned_nodes`] of the analyzed
    /// expression, which [`expr_nodes`] mirrors on the span-erased tree the
    /// evaluator runs, so a runtime observation can be joined to a finding.
    pub node_ordinal: usize,
}

impl MemberFinding {
    /// The library-owned message every surface shows.
    pub fn message(&self) -> String {
        format!(
            "Member '{}' is not declared by the known object type of '{}'.",
            self.member, self.subject
        )
    }

    /// The finding as a warning ([`UNDECLARED_MEMBER_CODE`]) ranged on the
    /// member name.
    pub fn diagnostic(&self) -> ExpressionDiagnostic {
        ExpressionDiagnostic {
            code: UNDECLARED_MEMBER_CODE,
            severity: Severity::Warning,
            message: self.message(),
            span: self.span.clone(),
            expected_type: None,
            actual_type: None,
        }
    }
}

/// Every undeclared member read of `expr`, in every branch and callback
/// body, typed under `site`, ordered by span. `text` is the exact text `expr`
/// was parsed from.
pub fn expression_member_findings(expr: &SpannedExpr, text: &str, site: &TypeScope<'_>) -> Vec<MemberFinding> {
    let reads = static_member_reads(expr);
    if reads.is_empty() {
        return Vec::new();
    }
    let ordinals: HashMap<*const SpannedExpr, usize> = spanned_nodes(expr)
        .into_iter()
        .enumerate()
        .map(|(ordinal, node)| (std::ptr::from_ref(node), ordinal))
        .collect();
    let mut findings = Vec::new();
    for records in reads.chunk_by(|left, right| std::ptr::eq(left.node, right.node)) {
        let node = records[0].node;
        let site_at = branch_scope(expr, &node.span, site.clone());
        if let Some(finding) = node_finding(node, records, text, &site_at) {
            findings.push(MemberFinding {
                node_ordinal: ordinals.get(&std::ptr::from_ref(node)).copied().unwrap_or_default(),
                ..finding
            });
        }
    }
    findings.sort_by_key(|finding| finding.span.start);
    findings
}

/// [`expression_member_findings`] as diagnostics.
pub fn expression_member_diagnostics(expr: &SpannedExpr, text: &str, site: &TypeScope<'_>) -> Vec<ExpressionDiagnostic> {
    expression_member_findings(expr, text, site).iter().map(MemberFinding::diagnostic).collect()
}

/// The finding of one access node, from its selector `records` in step
/// order: the first unhandled undeclared step, unless an earlier step is
/// undeclared or unknown.
fn node_finding(node: &SpannedExpr, records: &[StaticMemberRead<'_>], text: &str, site: &TypeScope<'_>) -> Option<MemberFinding> {
    let definitions = site.definitions();
    match &node.kind {
        SpannedExprKind::Variable(path) => {
            let root = variable_root_segments(path)?;
            if !variable_is_checked(path, site) {
                return None;
            }
            let segments: Vec<&str> = path.split('.').collect();
            let mut base: Option<Shape> = None;
            for record in records {
                let position = root + record.step;
                let name = *segments.get(position)?;
                let prefix_length = segments[..position].iter().map(|segment| segment.len()).sum::<usize>() + position - 1;
                let prefix = &path[..prefix_length];
                let shape = step_base(site, &Expr::Variable(prefix.to_string()), base.as_ref().zip(segments.get(position - 1)));
                match declares_member(&shape, name, definitions) {
                    MemberDeclaration::Declared => base = Some(shape),
                    MemberDeclaration::Unknown => return None,
                    MemberDeclaration::Undeclared if record.handles_absence => return None,
                    MemberDeclaration::Undeclared => {
                        let start = node.span.start + prefix_length + 1;
                        return Some(finding(name, prefix, start..start + name.len()));
                    }
                }
            }
            None
        }
        SpannedExprKind::MemberAccess { base: inner, name } => {
            if chain_root(inner).is_some_and(|path| !variable_is_checked(path, site)) {
                return None;
            }
            let segments: Vec<&str> = name.split('.').collect();
            let erased = inner.erase();
            let authored = authored_text(text, inner);
            let mut base: Option<Shape> = None;
            for record in records {
                let step = record.step;
                let segment = *segments.get(step)?;
                let shape = if step == 0 {
                    base_type(inner, site)
                } else {
                    let access = Expr::MemberAccess {
                        base: Box::new(erased.clone()),
                        name: segments[..step].join("."),
                    };
                    step_base(site, &access, base.as_ref().zip(segments.get(step - 1)))
                };
                match declares_member(&shape, segment, definitions) {
                    MemberDeclaration::Declared => base = Some(shape),
                    MemberDeclaration::Unknown => return None,
                    MemberDeclaration::Undeclared if record.handles_absence => return None,
                    MemberDeclaration::Undeclared => {
                        let later: usize = segments[step + 1..].iter().map(|later| later.len() + 1).sum();
                        let end = node.span.end - later;
                        let subject = std::iter::once(authored.as_str()).chain(segments[..step].iter().copied()).collect::<Vec<_>>().join(".");
                        return Some(finding(segment, &subject, end - segment.len()..end));
                    }
                }
            }
            None
        }
        SpannedExprKind::Index { base: inner, index } => {
            let (SpannedExprKind::StringLiteral(key), [record, ..]) = (&index.kind, records) else {
                return None;
            };
            if literal_index_key(&inner.erase(), &index.erase()).is_some()
                || chain_root(inner).is_some_and(|path| !variable_is_checked(path, site))
            {
                return None;
            }
            match declares_member(&base_type(inner, site), key, definitions) {
                MemberDeclaration::Undeclared if !record.handles_absence => {
                    Some(finding(key, &authored_text(text, inner), index.span.clone()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn finding(member: &str, subject: &str, span: SourceSpan) -> MemberFinding {
    MemberFinding {
        member: member.to_string(),
        subject: subject.to_string(),
        span,
        node_ordinal: 0,
    }
}

/// The type of the base `prefix` at `site`. A guard fact about the prefix
/// itself wins; otherwise the previous step's base, followed through its
/// member, keeps any narrowing of an ancestor (`is_string(x) ? … :
/// x.a.b` follows `a` from the narrowed `x`).
fn step_base(site: &TypeScope<'_>, prefix: &Expr, previous: Option<(&Shape, &&str)>) -> Shape {
    let scoped = site.type_of_expr(prefix);
    match (scoped, previous) {
        (Some(scoped), _) if scoped.is_refined() => scoped.shape().clone(),
        (_, Some((shape, name))) => member_shape(shape, &Member::Field((*name).to_string()), site.definitions()),
        (Some(scoped), None) => scoped.shape().clone(),
        (None, None) => Shape::Unknown { non_null: false },
    }
}

/// The type of the base expression `expr`: [`expression_type`], except that
/// constant member and index steps on a value with no stable subject (a
/// call result, a literal) follow the same member and item projection a
/// variable path does, so `load_data(…)[0].name` is typed like
/// `hosts[0].name`.
fn base_type(expr: &SpannedExpr, site: &TypeScope<'_>) -> Shape {
    let constant_step = match &expr.kind {
        SpannedExprKind::Paren(inner) => return base_type(inner, site),
        _ if site.subject(&expr.erase()).is_some() => None,
        SpannedExprKind::Index { base, index } => match &index.kind {
            SpannedExprKind::StringLiteral(key) => Some((base, vec![Member::Field(key.clone())])),
            SpannedExprKind::NumberLiteral(number) if number.fract() == 0.0 && *number <= i64::MAX as f64 => {
                Some((base, vec![Member::Index(*number as i64)]))
            }
            _ => None,
        },
        SpannedExprKind::MemberAccess { base, name } => {
            Some((base, name.split('.').map(|segment| Member::Field(segment.to_string())).collect()))
        }
        _ => None,
    };
    match constant_step {
        Some((base, steps)) => steps
            .iter()
            .fold(base_type(base, site), |shape, step| member_shape(&shape, step, site.definitions())),
        None => expression_type(expr, site),
    }
}

/// Whether reads below the variable `path` are checked: not `null`, a
/// reserved namespace, or a host global.
fn variable_is_checked(path: &str, site: &TypeScope<'_>) -> bool {
    super::absence::root_of(path) != NULL_ROOT
        && !matches!(
            site.declared().classify_variable(path),
            VariableClass::Namespace { .. } | VariableClass::Host { .. }
        )
}

/// The variable a postfix or bracket chain is rooted in, through member,
/// index, and parenthesis steps.
fn chain_root(expr: &SpannedExpr) -> Option<&str> {
    match &expr.kind {
        SpannedExprKind::Variable(path) => Some(path),
        SpannedExprKind::Paren(inner) | SpannedExprKind::MemberAccess { base: inner, .. } | SpannedExprKind::Index { base: inner, .. } => {
            chain_root(inner)
        }
        _ => None,
    }
}

/// The authored text of `node`, or its canonical spelling when `text` does
/// not hold its span.
fn authored_text(text: &str, node: &SpannedExpr) -> String {
    text.get(node.span.clone()).map_or_else(|| node.erase().to_string(), str::to_string)
}

/// Every node of `expr` in pre-order, children in the order [`expr_nodes`]
/// visits them: array items; object values; the inner expression of a
/// parenthesis, `!`, or unary `-`; left then right of an operator,
/// comparison, or `||`; an index's base then index; a member access's base;
/// a ternary's condition, then, else; call arguments in order, a callback
/// contributing its body.
pub(crate) fn spanned_nodes(expr: &SpannedExpr) -> Vec<&SpannedExpr> {
    let mut nodes = Vec::new();
    let mut pending = vec![expr];
    while let Some(node) = pending.pop() {
        nodes.push(node);
        let start = pending.len();
        match &node.kind {
            SpannedExprKind::Variable(_)
            | SpannedExprKind::StringLiteral(_)
            | SpannedExprKind::NumberLiteral(_)
            | SpannedExprKind::BoolLiteral(_) => {}
            SpannedExprKind::ArrayLiteral(items) | SpannedExprKind::FunctionCall { args: items, .. } => pending.extend(items),
            SpannedExprKind::ObjectLiteral(entries) => pending.extend(entries.iter().map(|(_, value)| value)),
            SpannedExprKind::Paren(inner) | SpannedExprKind::UnaryNot(inner) | SpannedExprKind::UnaryMinus(inner) => {
                pending.push(inner);
            }
            SpannedExprKind::Binary { left, right, .. }
            | SpannedExprKind::Comparison { left, right, .. }
            | SpannedExprKind::LogicalOr { left, right } => pending.extend([&**left, &**right]),
            SpannedExprKind::Index { base, index } => pending.extend([&**base, &**index]),
            SpannedExprKind::MemberAccess { base, .. } => pending.push(base),
            SpannedExprKind::Ternary {
                condition,
                then_branch,
                else_branch,
            } => pending.extend([&**condition, &**then_branch, &**else_branch]),
            SpannedExprKind::CallbackCall { args, .. } => pending.extend(args.iter().map(|arg| match arg {
                SpannedCallArgument::Value(value) => value,
                SpannedCallArgument::Callback(callback) => &callback.body,
            })),
        }
        pending[start..].reverse();
    }
    nodes
}

/// The ordinals in [`expr_nodes`] of `root` of the node addresses in
/// `addresses` (each recorded while `root` was evaluated), ascending and
/// distinct. Drains `addresses`.
pub(crate) fn reached_ordinals(root: &Expr, addresses: &mut Vec<usize>) -> Vec<usize> {
    if addresses.is_empty() {
        return Vec::new();
    }
    let reached: std::collections::HashSet<usize> = addresses.drain(..).collect();
    expr_nodes(root)
        .into_iter()
        .enumerate()
        .filter(|(_, node)| reached.contains(&std::ptr::from_ref(*node).addr()))
        .map(|(ordinal, _)| ordinal)
        .collect()
}

/// [`spanned_nodes`] for the span-erased tree: the same nodes in the same
/// order, so an ordinal names one node in both.
pub(crate) fn expr_nodes(expr: &Expr) -> Vec<&Expr> {
    let mut nodes = Vec::new();
    let mut pending = vec![expr];
    while let Some(node) = pending.pop() {
        nodes.push(node);
        let start = pending.len();
        match node {
            Expr::Variable(_) | Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
            Expr::ArrayLiteral(items) | Expr::FunctionCall { args: items, .. } => pending.extend(items),
            Expr::ObjectLiteral(entries) => pending.extend(entries.iter().map(|(_, value)| value)),
            Expr::Paren(inner) | Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) => pending.push(inner),
            Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } | Expr::LogicalOr { left, right } => {
                pending.extend([&**left, &**right]);
            }
            Expr::Index { base, index } => pending.extend([&**base, &**index]),
            Expr::MemberAccess { base, .. } => pending.push(base),
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => pending.extend([&**condition, &**then_branch, &**else_branch]),
            Expr::CallbackCall { args, .. } => pending.extend(args.iter().map(|arg| match arg {
                CallArgument::Value(value) => value,
                CallArgument::Callback(callback) => &callback.body,
            })),
        }
        pending[start..].reverse();
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::parse_spanned;

    /// The two enumerators visit the same nodes in the same order, for a
    /// corpus covering every node kind.
    #[test]
    fn ordinals_mirror_erase() {
        for source in [
            "a",
            "'s'",
            "1",
            "true",
            "[a, b.c, 1]",
            "{ k: a, m: { n: b } }",
            "(a).b.c",
            "!a",
            "-a",
            "a + b * c",
            "a == b",
            "a || b || c",
            "a ? b : c",
            "a[0]['k'][b]",
            "f(a, g(b), c.d)",
            "a && b",
            "map(xs, x => x.name)",
            "filter(map(xs, { a, b } => a + b), y => y > 1)",
            "try(load_data('h.yaml', '{ a: string }[]'), [])",
            "map(try(xs, []), h => h.nmae) || (y ? [z] : {k: -w})",
        ] {
            let spanned = parse_spanned(source).unwrap_or_else(|error| panic!("{source}: {error}"));
            let erased = spanned.erase();
            let left: Vec<Expr> = spanned_nodes(&spanned).into_iter().map(SpannedExpr::erase).collect();
            let right: Vec<Expr> = expr_nodes(&erased).into_iter().cloned().collect();
            assert_eq!(left, right, "{source}");
        }
    }
}
