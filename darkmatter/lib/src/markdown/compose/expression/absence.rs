//! Which variable reads state that a value may be absent (spec Requirement 4).
//!
//! A well-formed identifier that resolves to nothing is not an error, but it is
//! usually a typo, so full-document composition warns about it — unless the
//! author handled the absence explicitly. [`AbsenceScope`] is the one shared
//! classifier of those explicit constructs (Ruling R-2). Two rules decide it:
//!
//! - **Intentional absence handling**, a warning policy only: the primary of a
//!   fallback (`x || "d"`), and the operands of a fallback chain that is
//!   itself a primary (`a || b || "d"`); the condition of a ternary
//!   (`x ? a : b`); and a direct argument to an absence predicate
//!   (`is_null(x)`, `is_empty(x)` or an alias). `is_empty(trim(x))` is not
//!   direct. Handling a read never proves its value present.
//! - **Shared branch facts**: a read the enclosing branch facts prove present
//!   ([`TypeScope::assume`], the one fact producer). A ternary branch gets
//!   its own side of the condition, so `x ? x : "none"` handles the second
//!   `x` but `x ? "some" : x` does not; an `and`/`&&` operand gets the earlier
//!   operands' true side, and an `or`/`||` operand their false side. A fact
//!   covers its path and the paths below it (`x` covers `x.name`), never a
//!   sibling (`x.a` does not cover `x.b`).
//!
//! "Direct" is the rule for the first: only a variable occupying the position
//! itself (through parentheses and path/index access on it) is handled. Any
//! other node — arithmetic, comparison, negation, a non-predicate call —
//! makes its operands ordinary again, so `{{ x == "a" ? … }}` still warns.
//!
//! The runtime evaluator applies these rules while it evaluates, so a warning
//! also requires evaluation reachability: an unchosen branch or a
//! short-circuited operand is never read and never warns. This deliberately
//! differs from the exhaustive `ctx.*` typo walk, which keeps its own
//! semantics.
//!
//! Static consumers (DMLS) cannot know which branch runs, so
//! [`static_variable_reads`] applies the same rules to every branch instead,
//! each under its own side's facts.

use super::ast::{SpannedExpr, SpannedExprKind};
use super::scope::{DeclaredTypes, Polarity, TypeScope};
use crate::markdown::span::SourceSpan;

/// The declared layer branch facts are kept against. Absence handling asks
/// only whether a document read is proven present, which needs no schema.
static UNTYPED: DeclaredTypes<'static> = DeclaredTypes::new(None);

/// How a variable read in one AST position treats absence.
///
/// Copy-cheap, built on the evaluator's call stack: the branch facts are
/// borrowed from the frame that assumed them.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AbsenceScope<'a> {
    /// A bare variable directly in this position is an explicit absence check.
    handles_absence: bool,
    /// The enclosing branch facts; `None` when no branch encloses the read.
    facts: Option<&'a TypeScope<'static>>,
}

impl<'a> AbsenceScope<'a> {
    /// Whether a missing value at `path` in this position is explicitly handled.
    pub(crate) fn handles(&self, path: &str) -> bool {
        self.handles_absence
            || self.facts.is_some_and(|facts| {
                UNTYPED
                    .variable_subject(path)
                    .is_some_and(|subject| facts.guards_path_or_ancestor(&subject))
            })
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

    /// The branch facts here, unguarded when no branch encloses this position.
    pub(crate) fn facts(&self) -> TypeScope<'static> {
        self.facts.cloned().unwrap_or_else(|| TypeScope::unguarded(&UNTYPED))
    }

    /// This position under `facts`, which extend [`Self::facts`].
    pub(crate) fn with_facts<'b>(self, facts: &'b TypeScope<'static>) -> AbsenceScope<'b>
    where
        'a: 'b,
    {
        AbsenceScope {
            handles_absence: self.handles_absence,
            facts: Some(facts),
        }
    }
}

/// Whether `name` is the lazy conjunction or disjunction, as the evaluator
/// dispatches them.
fn short_circuit_polarity(name: &str) -> Option<Polarity> {
    if name.eq_ignore_ascii_case("and") {
        Some(Polarity::True)
    } else if name.eq_ignore_ascii_case("or") {
        Some(Polarity::False)
    } else {
        None
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
    /// ternary condition, or direct absence-predicate argument), or the
    /// branch facts around it prove it present.
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
/// and every fallback operand, since it cannot know which one runs, each
/// under its own side's branch facts. It does not parse or evaluate.
pub fn static_variable_reads(expr: &SpannedExpr) -> Vec<StaticVariableRead<'_>> {
    let mut reads = Vec::new();
    walk_static(expr, AbsenceScope::default(), &mut reads);
    reads
}

/// `site` extended by the branch facts of `expr` that enclose the node
/// spanning `target` (a span in the text `expr` was parsed from): the side of
/// each enclosing ternary condition its branch is on, and, for a later operand
/// of `and`/`&&` or `or`/`||`/fallback, the true or false side of the
/// operands before it. These are the facts [`static_variable_reads`] reads
/// each variable under, applied to a typed scope so a static consumer (DMLS
/// hover) sees the type at that node. A `target` no node spans gets the facts
/// of the deepest node containing it. Passive, like [`TypeScope::assume`].
pub fn branch_scope<'d>(expr: &SpannedExpr, target: &SourceSpan, site: TypeScope<'d>) -> TypeScope<'d> {
    let holds = |node: &SpannedExpr| node.span.start <= target.start && target.end <= node.span.end;
    match &expr.kind {
        SpannedExprKind::Fallback { primary, fallback } if holds(fallback) => {
            let facts = site.assume(&primary.erase(), Polarity::False);
            branch_scope(fallback, target, facts)
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } if holds(then_branch) || holds(else_branch) => {
            let (branch, polarity) = if holds(then_branch) {
                (then_branch, Polarity::True)
            } else {
                (else_branch, Polarity::False)
            };
            let facts = site.assume(&condition.erase(), polarity);
            branch_scope(branch, target, facts)
        }
        SpannedExprKind::FunctionCall { name, args } => {
            let continues = short_circuit_polarity(name);
            let mut facts = site.clone();
            for arg in args {
                if holds(arg) {
                    return branch_scope(arg, target, facts);
                }
                if let Some(polarity) = continues {
                    facts = facts.assume(&arg.erase(), polarity);
                }
            }
            site
        }
        _ => match child_holding(expr, &holds) {
            Some(child) => branch_scope(child, target, site),
            None => site,
        },
    }
}

/// The direct child of `expr` that `holds` accepts, for the node kinds whose
/// children inherit their parent's facts unchanged.
fn child_holding<'e>(expr: &'e SpannedExpr, holds: &impl Fn(&SpannedExpr) -> bool) -> Option<&'e SpannedExpr> {
    let children: Vec<&SpannedExpr> = match &expr.kind {
        SpannedExprKind::Paren(inner) | SpannedExprKind::UnaryNot(inner) | SpannedExprKind::UnaryMinus(inner) => {
            vec![inner]
        }
        SpannedExprKind::Binary { left, right, .. } | SpannedExprKind::Comparison { left, right, .. } => {
            vec![left, right]
        }
        SpannedExprKind::Index { base, index } => vec![base, index],
        SpannedExprKind::MemberAccess { base, .. } => vec![base],
        SpannedExprKind::Fallback { primary, .. } => vec![primary],
        SpannedExprKind::Ternary { condition, .. } => vec![condition],
        SpannedExprKind::ArrayLiteral(items) => items.iter().collect(),
        SpannedExprKind::ObjectLiteral(entries) => entries.iter().map(|(_, value)| value).collect(),
        _ => Vec::new(),
    };
    children.into_iter().find(|child| holds(child))
}

/// Mirrors the scope transitions of the evaluator's `evaluate_expr`, visiting
/// every branch and operand.
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
            let facts = scope.facts().assume(&primary.erase(), Polarity::False);
            walk_static(fallback, scope.with_facts(&facts), reads);
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            walk_static(condition, scope.absence_check(), reads);
            let condition = condition.erase();
            let true_side = scope.facts().assume(&condition, Polarity::True);
            let false_side = scope.facts().assume(&condition, Polarity::False);
            walk_static(then_branch, scope.with_facts(&true_side), reads);
            walk_static(else_branch, scope.with_facts(&false_side), reads);
        }
        SpannedExprKind::FunctionCall { name, args } => {
            let argument = scope.call_argument(name);
            let continues = short_circuit_polarity(name);
            let mut facts = argument.facts();
            for arg in args {
                walk_static(arg, argument.with_facts(&facts), reads);
                if let Some(polarity) = continues {
                    facts = facts.assume(&arg.erase(), polarity);
                }
            }
        }
    }
}

/// The first dotted segment of a variable path: `user` for `user.name`.
pub(crate) fn root_of(path: &str) -> &str {
    path.split('.').next().unwrap_or(path)
}

/// Whether `root` is known whatever the state holds: a reserved namespace
/// ([`is_reserved_namespace`](super::binding::is_reserved_namespace)) or the
/// `null` literal, which shipped prompts use as such (`{{ ok ? path : null }}`).
pub(crate) fn is_reserved_root(root: &str) -> bool {
    super::binding::is_reserved_namespace(root) || root == super::binding::NULL_ROOT
}

/// Receives each evaluated variable read that found no value, is not handled
/// by its [`AbsenceScope`], and that resolved as an absent document property.
pub(crate) trait MissingRootObserver {
    /// `false` only for the no-op observer, so unobserved evaluation skips the
    /// classification check entirely.
    const OBSERVES: bool = true;

    fn missing_root(&mut self, root: &str);
}

impl MissingRootObserver for () {
    const OBSERVES: bool = false;

    fn missing_root(&mut self, _root: &str) {}
}

/// One unhandled read of an absent document property.
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
    use crate::markdown::compose::expression::scope::Polarity;

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
    fn branch_facts_cover_a_proven_path_and_its_descendants_only() {
        let outer = AbsenceScope::default();
        let facts = outer.facts().assume(&parse("color.name").unwrap(), Polarity::True);
        let branch = outer.with_facts(&facts);
        assert!(branch.handles("color.name"));
        assert!(branch.handles("color.name.first"));
        assert!(!branch.handles("color.other"), "a sibling inherits nothing");
        assert!(!branch.handles("color"), "a member proves nothing about its parent");
        assert!(!branch.handles("shade"));
        assert!(branch.handles("fm.color.name"), "bare and `fm.` reads share a binding");
        // The facts survive an operand reset inside the branch.
        assert!(branch.operand().handles("color.name"));
    }

    #[test]
    fn a_false_side_proves_only_what_the_guard_makes_present_there() {
        let outer = AbsenceScope::default();
        for (source, handled) in [("x", false), ("!x", true), ("x == 1", false), ("f(x)", false), ("is_null(x)", true)] {
            let facts = outer.facts().assume(&parse(source).unwrap(), Polarity::False);
            assert_eq!(outer.with_facts(&facts).handles("x"), handled, "{source}");
        }
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
        // A false branch does not inherit the condition's true-side fact.
        ("x ? \"some\" : x", &["x"]),
        ("!x ? \"none\" : x", &["x"]),
        // A sibling of a proven or exempt path is not covered.
        ("x.a ? x.b : \"n\"", &["x"]),
        ("x.a || x.b", &["x"]),
        // Comparisons, predicates, and `&&` give their branch facts too.
        ("x != null ? x : \"n\"", &["x", "null"]),
        ("is_string(x) ? x : \"n\"", &["x"]),
        ("x && x.name", &["x"]),
        ("and(x, y) ? y : z", &["x", "y", "z"]),
        ("x || y ? y : z", &["y", "z"]),
        ("or(x, y) ? z : x", &["x", "y", "z", "x"]),
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
    fn reserved_roots_are_the_catalog_namespaces_and_null() {
        for root in ["ctx", "env", "fm", "current", "null"] {
            assert!(is_reserved_root(root), "{root}");
        }
        for root in ["spec-name", "colour", "nul", "repo", "today", "branch", "doc", "current_env"] {
            assert!(!is_reserved_root(root), "{root}");
        }
    }

    #[test]
    fn nested_branch_facts_chain() {
        let scope = AbsenceScope::default();
        let in_outer = scope.facts().assume(&parse("a").unwrap(), Polarity::True);
        let in_inner = scope.with_facts(&in_outer).facts().assume(&parse("b").unwrap(), Polarity::True);
        let inner = scope.with_facts(&in_inner);
        assert!(inner.handles("a") && inner.handles("b"));
        assert!(!inner.handles("c"));
    }

    /// `branch_scope` gives each read the branch facts `static_variable_reads`
    /// classifies it under: a read it proves present is one the walk handles,
    /// and each side, operand, and sibling gets only its own facts.
    #[test]
    fn branch_scope_gives_each_read_its_own_side() {
        for (source, expected) in [
            ("x ? x : x", &[false, true, false][..]),
            ("x && x.name && x", &[false, true, true]),
            ("x.a && x.b", &[false, false]),
            ("x || x", &[false, false]),
            ("!x ? 'none' : x", &[false, true]),
            ("[a && a, b ? b : 0]", &[false, true, false, true]),
        ] {
            let expr = super::super::parse_spanned(source).unwrap();
            let reads = static_variable_reads(&expr);
            let proven: Vec<bool> = reads
                .iter()
                .map(|read| {
                    let site = branch_scope(&expr, &read.span, TypeScope::unguarded(&UNTYPED));
                    UNTYPED
                        .variable_subject(read.path)
                        .is_some_and(|subject| site.guards_path_or_ancestor(&subject))
                })
                .collect();
            assert_eq!(proven, expected, "`{source}`");
            for (read, proven) in reads.iter().zip(&proven) {
                assert!(!proven || read.handles_absence, "`{source}` at {:?}", read.span);
            }
        }
    }
}
