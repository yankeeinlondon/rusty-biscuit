//! Which variable reads state that a value may be absent (spec Requirement 4).
//!
//! A well-formed identifier that resolves to nothing is not an error, but it is
//! usually a typo, so full-document composition warns about it — unless the
//! author handled the absence explicitly. [`AbsenceScope`] is the one shared
//! classifier of those explicit constructs (Ruling R-2):
//!
//! - the primary of a fallback (`x || "d"`), and the operands of a fallback
//!   chain that is itself a primary (`a || b || "d"`);
//! - the condition of a ternary (`x ? a : b`), and, when that condition is a
//!   bare variable, references to the same root in its branches
//!   (`x ? x : "none"`);
//! - a direct bare-variable argument to an absence predicate (`is_null(x)`,
//!   `is_empty(x)` or an alias). `is_empty(trim(x))` is not direct.
//!
//! "Direct" is the rule throughout: only a variable occupying the position
//! itself (through parentheses and path/index access on it) is handled. Any
//! other node — arithmetic, comparison, negation, a non-predicate call —
//! makes its operands ordinary again, so `{{ x == "a" ? … }}` still warns.
//!
//! The runtime evaluator applies these rules while it evaluates, so a warning
//! also requires evaluation reachability: an unchosen branch or a
//! short-circuited fallback is never read and never warns. This deliberately
//! differs from `collect_variable_roots` (subtree strict mode) and from the
//! exhaustive `ctx.*` typo walk, which both keep their own semantics.
//!
//! Static consumers (DMLS) cannot know which branch runs, so
//! [`static_variable_reads`] applies the same structural rules to every
//! branch instead.

use super::ast::{Expr, SpannedExpr, SpannedExprKind};
use crate::markdown::span::SourceSpan;

/// How a variable read in one AST position treats absence.
///
/// Copy-cheap, built on the evaluator's call stack: guarded roots form a
/// linked list of borrowed [`RootGuard`]s, so descending allocates nothing.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AbsenceScope<'a> {
    /// A bare variable directly in this position is an explicit absence check.
    handles_absence: bool,
    guards: Option<&'a RootGuard<'a>>,
}

/// A root that an enclosing ternary's bare-variable condition tested.
#[derive(Debug)]
pub(crate) struct RootGuard<'a> {
    root: &'a str,
    outer: Option<&'a RootGuard<'a>>,
}

impl<'a> AbsenceScope<'a> {
    /// Whether a missing value at `path` in this position is explicitly handled.
    pub(crate) fn handles(&self, path: &str) -> bool {
        if self.handles_absence {
            return true;
        }
        let root = root_of(path);
        let mut guard = self.guards;
        while let Some(current) = guard {
            if current.root == root {
                return true;
            }
            guard = current.outer;
        }
        false
    }

    /// An ordinary operand: arithmetic, comparison, negation, collection
    /// items, index expressions, and non-predicate call arguments.
    pub(crate) fn operand(self) -> Self {
        Self {
            handles_absence: false,
            ..self
        }
    }

    /// A fallback primary or ternary condition.
    pub(crate) fn absence_check(self) -> Self {
        Self {
            handles_absence: true,
            ..self
        }
    }

    /// An argument of a call to `function`.
    pub(crate) fn call_argument(self, function: &str) -> Self {
        Self {
            handles_absence: super::functions::is_absence_predicate(function),
            ..self
        }
    }

    /// A ternary branch, guarded by `guard` when the condition is a bare
    /// variable (see [`condition_guard`]).
    pub(crate) fn branch<'b>(self, guard: Option<&'b RootGuard<'b>>) -> AbsenceScope<'b>
    where
        'a: 'b,
    {
        AbsenceScope {
            handles_absence: self.handles_absence,
            guards: guard.or(self.guards),
        }
    }

    /// The guard a ternary with this `condition` places on its branches, or
    /// `None` when the condition is not a bare variable.
    pub(crate) fn condition_guard<'b>(self, condition: &'b Expr) -> Option<RootGuard<'b>>
    where
        'a: 'b,
    {
        match condition {
            Expr::Variable(path) => Some(RootGuard {
                root: root_of(path),
                outer: self.guards,
            }),
            Expr::Paren(inner) => self.condition_guard(inner),
            _ => None,
        }
    }

    /// [`Self::condition_guard`] over a span-carrying condition.
    fn spanned_condition_guard<'b>(self, condition: &'b SpannedExpr) -> Option<RootGuard<'b>>
    where
        'a: 'b,
    {
        match &condition.kind {
            SpannedExprKind::Variable(path) => Some(RootGuard {
                root: root_of(path),
                outer: self.guards,
            }),
            SpannedExprKind::Paren(inner) => self.spanned_condition_guard(inner),
            _ => None,
        }
    }
}

/// One `Variable` node found by [`static_variable_reads`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticVariableRead<'e> {
    /// The variable path as authored: `spec-name`, `user.name`, `ctx.today`.
    pub path: &'e str,
    /// Byte span of the `Variable` node in the parsed expression text.
    pub span: SourceSpan,
    /// Whether the author handled this read's absence (fallback primary,
    /// ternary condition, guarded branch root, or direct absence-predicate
    /// argument).
    pub handles_absence: bool,
}

impl StaticVariableRead<'_> {
    /// The first dotted segment of [`Self::path`].
    pub fn root(&self) -> &str {
        root_of(self.path)
    }
}

/// Every `Variable` node of `expr`, in source order, each classified by the
/// same absence rules the runtime applies.
///
/// Unlike the runtime, this walk is static: it visits both ternary branches
/// and every fallback operand, since it cannot know which one runs. It is
/// linear in the number of AST nodes and does not parse.
pub fn static_variable_reads(expr: &SpannedExpr) -> Vec<StaticVariableRead<'_>> {
    let mut reads = Vec::new();
    walk_static(expr, AbsenceScope::default(), &mut reads);
    reads
}

/// Mirrors the scope transitions of the evaluator's `evaluate_expr`.
fn walk_static<'e>(
    expr: &'e SpannedExpr,
    scope: AbsenceScope<'_>,
    reads: &mut Vec<StaticVariableRead<'e>>,
) {
    let operand = scope.operand();
    match &expr.kind {
        SpannedExprKind::Variable(path) => reads.push(StaticVariableRead {
            path,
            span: expr.span.clone(),
            handles_absence: scope.handles(path),
        }),
        SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => {}
        SpannedExprKind::ArrayLiteral(items) => {
            for item in items {
                walk_static(item, operand, reads);
            }
        }
        SpannedExprKind::ObjectLiteral(entries) => {
            for (_, value) in entries {
                walk_static(value, operand, reads);
            }
        }
        SpannedExprKind::Paren(inner) => walk_static(inner, scope, reads),
        SpannedExprKind::UnaryNot(inner) | SpannedExprKind::UnaryMinus(inner) => {
            walk_static(inner, operand, reads);
        }
        SpannedExprKind::Binary { left, right, .. }
        | SpannedExprKind::Comparison { left, right, .. } => {
            walk_static(left, operand, reads);
            walk_static(right, operand, reads);
        }
        SpannedExprKind::Index { base, index } => {
            walk_static(base, scope, reads);
            walk_static(index, operand, reads);
        }
        SpannedExprKind::MemberAccess { base, .. } => walk_static(base, scope, reads),
        SpannedExprKind::Fallback { primary, fallback } => {
            walk_static(primary, scope.absence_check(), reads);
            walk_static(fallback, scope, reads);
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            let guard = scope.spanned_condition_guard(condition);
            let branch_scope = scope.branch(guard.as_ref());
            walk_static(condition, scope.absence_check(), reads);
            walk_static(then_branch, branch_scope, reads);
            walk_static(else_branch, branch_scope, reads);
        }
        SpannedExprKind::FunctionCall { name, args } => {
            let argument = scope.call_argument(name);
            for arg in args {
                walk_static(arg, argument, reads);
            }
        }
    }
}

/// The first dotted segment of a variable path: `user` for `user.name`.
pub(crate) fn root_of(path: &str) -> &str {
    path.split('.').next().unwrap_or(path)
}

/// Whether `root` is known whatever the state holds: the reserved namespaces
/// `ctx`, `env`, `doc`, the lazy `current` / `current_env` roots, and `null`.
///
/// The grammar has no `null` literal; `null` lexes as a variable that resolves
/// to nothing, and shipped prompts use it as the literal
/// (`{{ ok ? path : null }}`). Reading it is never a typo.
pub(crate) fn is_reserved_root(root: &str) -> bool {
    matches!(root, "ctx" | "env" | "doc" | "null")
        || crate::markdown::compose::context::CurrentScope::is_reserved_root(root)
}

/// Whether `root` is known whatever the document holds: a reserved root (see
/// [`is_reserved_root`]) or a bare runtime-context name such as `repo`, which
/// the runtime's known-root check also accepts. Static consumers (DMLS) add
/// frontmatter and schema membership themselves.
pub fn is_statically_known_root(root: &str) -> bool {
    is_reserved_root(root)
        || crate::markdown::compose::context::catalog::CONTEXT_VARIABLE_DESCRIPTORS
            .iter()
            .any(|descriptor| descriptor.name == root)
}

/// Receives each evaluated variable read that found no value, is not handled
/// by its [`AbsenceScope`], and whose root the lookup does not know.
pub(crate) trait MissingRootObserver {
    /// `false` only for the no-op observer, so unobserved evaluation skips the
    /// known-root check entirely.
    const OBSERVES: bool = true;

    fn missing_root(&mut self, root: &str);
}

impl MissingRootObserver for () {
    const OBSERVES: bool = false;

    fn missing_root(&mut self, _root: &str) {}
}

/// One unhandled read of an unknown root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingRoot {
    pub(crate) root: String,
    /// Byte range of the enclosing `{{ … }}` in the text the caller scanned,
    /// when the reading surface knows it (authored body and frontmatter text;
    /// never a rescan of replacement output).
    pub(crate) span: Option<std::ops::Range<usize>>,
}

impl MissingRootObserver for Vec<MissingRoot> {
    fn missing_root(&mut self, root: &str) {
        self.push(MissingRoot {
            root: root.to_string(),
            span: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::parse;

    #[test]
    fn default_scope_handles_nothing() {
        assert!(!AbsenceScope::default().handles("x"));
    }

    #[test]
    fn an_absence_check_handles_any_direct_variable_until_an_operand_resets_it() {
        let scope = AbsenceScope::default().absence_check();
        assert!(scope.handles("x"));
        assert!(scope.handles("x.deep"));
        assert!(!scope.operand().handles("x"));
    }

    #[test]
    fn only_absence_predicate_arguments_are_handled() {
        let scope = AbsenceScope::default();
        assert!(scope.call_argument("is_null").handles("x"));
        assert!(scope.call_argument("isEmpty").handles("x"));
        assert!(!scope.call_argument("trim").handles("x"));
        // A handled scope does not leak into a non-predicate call's arguments.
        assert!(!scope.absence_check().call_argument("trim").handles("x"));
    }

    #[test]
    fn a_bare_variable_condition_guards_its_root_in_branches_only() {
        let condition = parse("color.name").unwrap();
        let outer = AbsenceScope::default();
        let guard = outer.condition_guard(&condition);
        let branch = outer.branch(guard.as_ref());
        assert!(branch.handles("color"));
        assert!(branch.handles("color.other"));
        assert!(!branch.handles("shade"));
        // The guard survives an operand reset inside the branch.
        assert!(branch.operand().handles("color"));
    }

    #[test]
    fn a_non_variable_condition_guards_nothing() {
        for source in ["!x", "x == 1", "f(x)"] {
            let condition = parse(source).unwrap();
            assert!(AbsenceScope::default().condition_guard(&condition).is_none(), "{source}");
        }
        let parenthesized = parse("(x)").unwrap();
        assert!(AbsenceScope::default().condition_guard(&parenthesized).is_some());
    }

    /// Unhandled roots, in source order, as DMLS would flag them.
    fn statically_unhandled(source: &str) -> Vec<String> {
        let expr = crate::markdown::compose::expression::parse_spanned(source).unwrap();
        static_variable_reads(&expr)
            .into_iter()
            .filter(|read| !read.handles_absence)
            .map(|read| read.root().to_string())
            .collect()
    }

    /// The Requirement 4 suppression table, read statically: both ternary
    /// branches and every fallback operand are visited.
    const STATIC_ROWS: &[(&str, &[&str])] = &[
        ("x", &["x"]),
        ("x || \"d\"", &[]),
        ("a || x", &["x"]),
        ("a || b || \"d\"", &[]),
        ("x ? a : b", &["a", "b"]),
        ("x ? x : b", &["b"]),
        ("(x) ? x.name : \"none\"", &[]),
        ("a ? x : b", &["x", "b"]),
        ("is_null(x)", &[]),
        ("isnull(x)", &[]),
        ("isEmpty(x)", &[]),
        ("is_empty(lower(x))", &["x"]),
        ("x == 1 ? y : z", &["x", "y", "z"]),
        ("!x ? y : z", &["x", "y", "z"]),
        ("x.deep || \"d\"", &[]),
        ("x[i] || \"d\"", &["i"]),
        ("a - b", &["a", "b"]),
        ("foo--bar", &["foo", "bar"]),
        ("lower(a) + upper(b)", &["a", "b"]),
        ("[a, b]", &["a", "b"]),
        ("spec-name", &["spec-name"]),
    ];

    #[test]
    fn static_reads_apply_the_suppression_rules_to_every_branch() {
        for (source, expected) in STATIC_ROWS {
            assert_eq!(statically_unhandled(source), *expected, "{source}");
        }
    }

    /// Editor/runtime parity: every root the runtime warns about for an
    /// all-missing lookup is also flagged statically. The static walk may
    /// flag more, only for positions the runtime did not evaluate.
    #[test]
    fn every_runtime_warning_is_also_a_static_warning() {
        use crate::markdown::compose::expression::{EvaluationLookup, evaluate_observed};

        struct Nothing;
        impl EvaluationLookup for Nothing {
            fn get(&self, _path: &str) -> Option<serde_json::Value> {
                None
            }
            fn is_known_variable_root(&self, _root: &str) -> bool {
                false
            }
        }

        let mut runtime_warnings = 0;
        for (source, _) in STATIC_ROWS {
            let mut observed: Vec<MissingRoot> = Vec::new();
            let _ = evaluate_observed(&parse(source).unwrap(), &Nothing, &mut observed);
            let statically = statically_unhandled(source);
            for missing in &observed {
                assert!(
                    statically.contains(&missing.root),
                    "{source}: runtime warns `{}` but the static walk does not ({statically:?})",
                    missing.root
                );
            }
            runtime_warnings += observed.len();
        }
        assert!(runtime_warnings > 10, "the runtime observer must actually fire");
    }

    #[test]
    fn static_reads_carry_their_own_spans() {
        let source = "a - spec-name ? f(ctx.today) : b";
        let expr = crate::markdown::compose::expression::parse_spanned(source).unwrap();
        let spans: Vec<(&str, &str)> = static_variable_reads(&expr)
            .iter()
            .map(|read| (read.path, &source[read.span.clone()]))
            .collect();
        assert_eq!(
            spans,
            [
                ("a", "a"),
                ("spec-name", "spec-name"),
                ("ctx.today", "ctx.today"),
                ("b", "b")
            ]
        );
    }

    #[test]
    fn statically_known_roots_match_the_runtime_reserved_and_context_names() {
        for root in ["ctx", "env", "doc", "current", "current_env", "null", "repo", "today"] {
            assert!(is_statically_known_root(root), "{root}");
        }
        for root in ["spec-name", "colour", "nul"] {
            assert!(!is_statically_known_root(root), "{root}");
        }
    }

    #[test]
    fn nested_guards_chain() {
        let outer_condition = parse("a").unwrap();
        let inner_condition = parse("b").unwrap();
        let scope = AbsenceScope::default();
        let outer_guard = scope.condition_guard(&outer_condition);
        let in_outer = scope.branch(outer_guard.as_ref());
        let inner_guard = in_outer.condition_guard(&inner_condition);
        let in_inner = in_outer.branch(inner_guard.as_ref());
        assert!(in_inner.handles("a") && in_inner.handles("b"));
        assert!(!in_inner.handles("c"));
    }
}
