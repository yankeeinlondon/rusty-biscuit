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
//!   (`x ? a : b`); and a direct argument bound to a parameter whose
//!   declared type explicitly includes `null` (`is_null(x)`, `is_empty(x)`,
//!   `file_exists(x)`, `trim(x)`, a host function's `string | null`
//!   parameter), whatever the function returns. A plain `unknown` parameter
//!   or an optional non-null one does not handle it, and an overloaded
//!   function handles it only when every overload taking that many
//!   arguments does. Handling a read never proves its value present.
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
//! other node — arithmetic, comparison, negation, a call whose parameter
//! does not declare `null` —
//! makes its operands ordinary again, so `{{ x == "a" ? … }}` still warns.
//! A call that is itself a direct argument of another call handles none of
//! its own arguments: its result, not the variable, is what the outer call
//! receives, so `is_empty(trim(x))` still warns for `x` while `trim(x)`,
//! `trim(x) || "d"`, and `trim(x) == "a"` do not. The logical `and`/`or`
//! (`&&`/`||` between conditions) are operators here, neither consuming
//! their operands nor consumed: `is_null(x) && y` handles `x`, and so does
//! `is_empty(x && y)`.
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
//!
//! [`static_access_reads`] is the same read inventory without the absence
//! classification, for a host that filters reads by its own names: each read
//! is a variable plus the selectors applied to it, so `fm['key']` and
//! `state.a[0]` keep the path they read.

use super::ast::{Expr, SpannedExpr, SpannedExprKind};
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
    /// This position is a direct argument of a call other than `and`/`or`,
    /// so a call here (other than `and`/`or`) hands its result on instead of
    /// handling its own arguments' absence.
    consumed_by_call: bool,
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
    /// items, and index expressions.
    pub(crate) fn operand(self) -> Self {
        Self {
            handles_absence: false,
            consumed_by_call: false,
            ..self
        }
    }

    /// A fallback primary or ternary condition.
    pub(crate) fn absence_check(self) -> Self {
        Self {
            handles_absence: true,
            consumed_by_call: false,
            ..self
        }
    }

    /// The argument at `index` of a call to `function` with `count`
    /// arguments, handled as
    /// [`FunctionRegistration::handles_absence_at`](super::call_binding::FunctionRegistration::handles_absence_at)
    /// decides, unless this call is itself a direct argument of another call
    /// (see the module docs).
    /// An unknown function, or a host function whose analysis is incomplete,
    /// handles nothing.
    pub(crate) fn call_argument(self, function: &str, index: usize, count: usize) -> Self {
        let logical = short_circuit_polarity(function).is_some();
        Self {
            handles_absence: (logical || !self.consumed_by_call)
                && super::functions::inspect_registration(function, |registration| {
                    registration.handles_absence_at(index, count)
                })
                .unwrap_or(false),
            consumed_by_call: !logical,
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
            consumed_by_call: self.consumed_by_call,
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
    /// ternary condition, or direct argument to a parameter declaring
    /// `null` of a call that is not itself another call's argument), or the
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

/// One selector applied to a variable read by [`static_access_reads`],
/// outside the variable's own dotted path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessStep<'e> {
    /// `.name` after a parenthesized or bracketed base; `name` may itself be
    /// a dotted path, as the lexer folds it.
    Member(&'e str),
    /// `['key']`: a string-literal index.
    Key(&'e str),
    /// `[index]` with any other index: a number literal or a computed value.
    /// `access` is the whole `base[index]` node.
    Index {
        /// The index expression.
        index: &'e Expr,
        /// The access expression this selector ends.
        access: &'e Expr,
    },
}

/// One variable read found by [`static_access_reads`]: a variable and the
/// selectors applied to it, innermost first.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticAccessRead<'e> {
    /// The variable path as authored: `state`, `fm.cfg`, `ctx.today`.
    pub variable: &'e str,
    /// The selectors applied to the variable, innermost first, through
    /// parentheses: `(state).a['b'][0]` is `[Member(a), Key(b), Index(0)]`.
    pub steps: Vec<AccessStep<'e>>,
}

impl StaticAccessRead<'_> {
    /// The first dotted segment of [`Self::variable`].
    pub fn root(&self) -> &str {
        root_of(self.variable)
    }

    /// The read's longest static path, dotted: the variable, then each
    /// member and string key up to the first [`AccessStep::Index`].
    /// `state['a'].b[0]['c']` is `state.a.b`.
    pub fn static_path(&self) -> String {
        let mut path = self.variable.to_string();
        for step in &self.steps {
            match step {
                AccessStep::Member(name) | AccessStep::Key(name) => {
                    path.push('.');
                    path.push_str(name);
                }
                AccessStep::Index { .. } => break,
            }
        }
        path
    }
}

/// Every variable read of `expr`, in source order, each with the selectors
/// applied to it.
///
/// Like [`static_variable_reads`], the walk is static: it visits every
/// branch, operand, call argument, container value, and index expression,
/// and does not evaluate. A read inside an index expression follows the read
/// it indexes. Literals are data, so `'state'` reads nothing, and a selector
/// on a non-variable base (`f(x).state`) belongs to no read.
pub fn static_access_reads(expr: &Expr) -> Vec<StaticAccessRead<'_>> {
    let mut reads = Vec::new();
    walk_access(expr, &mut reads);
    reads
}

fn walk_access<'e>(expr: &'e Expr, reads: &mut Vec<StaticAccessRead<'e>>) {
    match expr {
        Expr::Variable(_) | Expr::MemberAccess { .. } | Expr::Index { .. } | Expr::Paren(_) => {
            walk_access_chain(expr, reads);
        }
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => {}
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) => walk_access(inner, reads),
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Fallback {
            primary: left,
            fallback: right,
        } => {
            walk_access(left, reads);
            walk_access(right, reads);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            walk_access(condition, reads);
            walk_access(then_branch, reads);
            walk_access(else_branch, reads);
        }
        Expr::FunctionCall { args: items, .. } | Expr::ArrayLiteral(items) => {
            for item in items {
                walk_access(item, reads);
            }
        }
        Expr::ObjectLiteral(entries) => {
            for (_, value) in entries {
                walk_access(value, reads);
            }
        }
    }
}

/// Unwinds the member, index, and parenthesis chain ending at `expr` onto
/// its base, then reads the base and each computed index in source order.
fn walk_access_chain<'e>(expr: &'e Expr, reads: &mut Vec<StaticAccessRead<'e>>) {
    let mut steps = Vec::new();
    let mut base = expr;
    loop {
        match base {
            Expr::MemberAccess { base: inner, name } => {
                steps.push(AccessStep::Member(name));
                base = inner;
            }
            Expr::Index { base: inner, index } => {
                steps.push(match index.as_ref() {
                    Expr::StringLiteral(key) => AccessStep::Key(key),
                    index => AccessStep::Index { index, access: base },
                });
                base = inner;
            }
            Expr::Paren(inner) => base = inner,
            _ => break,
        }
    }
    steps.reverse();
    let indexes: Vec<&'e Expr> = steps
        .iter()
        .filter_map(|step| match step {
            AccessStep::Index { index, .. } => Some(*index),
            AccessStep::Member(_) | AccessStep::Key(_) => None,
        })
        .collect();
    match base {
        Expr::Variable(variable) => reads.push(StaticAccessRead { variable, steps }),
        other => walk_access(other, reads),
    }
    for index in indexes {
        walk_access(index, reads);
    }
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
            let continues = short_circuit_polarity(name);
            let mut facts = scope.facts();
            for (index, arg) in args.iter().enumerate() {
                let argument = scope.call_argument(name, index, args.len());
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
    fn only_arguments_bound_to_a_parameter_declaring_null_are_handled() {
        let scope = AbsenceScope::default();
        // `x: unknown | null`, by canonical name or alias.
        assert!(scope.call_argument("is_null", 0, 1).handles("x"));
        assert!(scope.call_argument("isEmpty", 0, 1).handles("x"));
        assert!(scope.call_argument("file_exists", 0, 1).handles("x"));
        // `text: string | null`, whatever the function returns.
        assert!(scope.call_argument("trim", 0, 1).handles("x"));
        // `a: number`.
        assert!(!scope.call_argument("min", 0, 2).handles("x"));
        // No overload takes that many arguments; no such function.
        assert!(!scope.call_argument("is_null", 0, 3).handles("x"));
        assert!(!scope.call_argument("no_such_function", 0, 1).handles("x"));
        // A handled scope does not leak into another call's arguments.
        assert!(!scope.absence_check().call_argument("min", 0, 2).handles("x"));
    }

    #[test]
    fn a_call_that_is_another_calls_argument_handles_none_of_its_own() {
        let scope = AbsenceScope::default();
        let consumed = scope.call_argument("is_empty", 0, 1);
        assert!(!consumed.call_argument("trim", 0, 1).handles("x"));
        assert!(!consumed.call_argument("is_null", 0, 1).handles("x"));
        // An operator or fallback between the calls ends the consumption.
        assert!(consumed.operand().call_argument("trim", 0, 1).handles("x"));
        assert!(consumed.absence_check().call_argument("trim", 0, 1).handles("x"));
        // `and`/`or` neither consume their operands nor are consumed.
        assert!(scope.call_argument("and", 0, 2).call_argument("is_null", 0, 1).handles("x"));
        assert!(consumed.call_argument("and", 0, 2).handles("x"));
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
        ("is_string(x) ? x : \"n\"", &[]),
        // `and`/`or` operands bind `unknown | null`, so a direct variable
        // operand is handled like any such argument.
        ("x && x.name", &[]),
        ("and(x, y) ? y : z", &["z"]),
        ("x || y ? y : z", &["y", "z"]),
        ("or(x, y) ? z : x", &["z", "x"]),
        ("a ? x : b", &["x", "b"]),
        ("is_null(x)", &[]),
        ("isnull(x)", &[]),
        ("isEmpty(x)", &[]),
        ("is_empty(lower(x))", &["x"]),
        ("is_empty(trim(x))", &["x"]),
        ("is_empty((trim(x)))", &["x"]),
        ("file_exists(x)", &[]),
        ("min(x, 5)", &["x"]),
        ("lower(x)", &[]),
        ("trim(x)", &[]),
        ("trim(x) || \"d\"", &[]),
        ("trim(x) == \"a\" ? y : z", &["y", "z"]),
        ("is_empty(trim(x) || \"d\")", &[]),
        ("is_null(x) && is_empty(trim(y))", &["y"]),
        ("is_empty(x && y)", &[]),
        ("x == 1 ? y : z", &["x", "y", "z"]),
        ("!x ? y : z", &["x", "y", "z"]),
        ("x.deep || \"d\"", &[]),
        ("x[i] || \"d\"", &["i"]),
        ("a - b", &["a", "b"]),
        ("foo--bar", &["foo", "bar"]),
        ("lower(a) + upper(b)", &[]),
        ("upper(lower(a))", &["a"]),
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

    /// `(variable, static path, display of each computed index access)` of
    /// every read of `source`, in source order.
    fn access_reads(source: &str) -> Vec<(String, String, Vec<String>)> {
        let expr = parse(source).unwrap();
        static_access_reads(&expr)
            .iter()
            .map(|read| {
                let computed = read
                    .steps
                    .iter()
                    .filter_map(|step| match step {
                        AccessStep::Index { access, .. } => Some(access.to_string()),
                        AccessStep::Member(_) | AccessStep::Key(_) => None,
                    })
                    .collect();
                (read.variable.to_string(), read.static_path(), computed)
            })
            .collect()
    }

    #[test]
    fn access_reads_keep_each_static_path_through_members_keys_and_parentheses() {
        let none = Vec::<String>::new;
        for (source, expected) in [
            ("state", vec![("state", "state", none())]),
            ("state.a.b", vec![("state.a.b", "state.a.b", none())]),
            ("state['a']", vec![("state", "state.a", none())]),
            ("(state).a['b']", vec![("state", "state.a.b", none())]),
            ("(fm)['k']", vec![("fm", "fm.k", none())]),
            ("state['a'].b[0]['c']", vec![("state", "state.a.b", vec![r#"state["a"].b[0]"#.to_string()])]),
            (
                "ctx[k].x[j]",
                vec![
                    ("ctx", "ctx", vec!["ctx[k]".to_string(), "ctx[k].x[j]".to_string()]),
                    ("k", "k", none()),
                    ("j", "j", none()),
                ],
            ),
            ("(ctx)[0]", vec![("ctx", "ctx", vec!["(ctx)[0]".to_string()])]),
        ] {
            let expected: Vec<(String, String, Vec<String>)> = expected
                .into_iter()
                .map(|(variable, path, computed)| (variable.to_string(), path.to_string(), computed))
                .collect();
            assert_eq!(access_reads(source), expected, "`{source}`");
        }
    }

    #[test]
    fn access_reads_visit_every_position_in_source_order() {
        for (source, expected) in [
            ("x ? y : f(g(z))", &["x", "y", "z"][..]),
            ("a || -b + !c", &["a", "b", "c"]),
            ("[a, {k: b}] == c", &["a", "b", "c"]),
            ("a[b[c]].d", &["a", "b", "c"]),
            ("f(x).state", &["x"]),
            ("'state' + 1 + true", &[]),
        ] {
            let variables: Vec<String> = access_reads(source).into_iter().map(|(variable, ..)| variable).collect();
            assert_eq!(variables, expected, "`{source}`");
        }
    }

    /// Both read inventories visit the same variables in the same order.
    #[test]
    fn access_reads_and_absence_reads_visit_the_same_variables() {
        for (source, _) in STATIC_ROWS {
            let spanned = crate::markdown::compose::expression::parse_spanned(source).unwrap();
            let absence: Vec<&str> = static_variable_reads(&spanned).iter().map(|read| read.path).collect();
            let erased = spanned.erase();
            let access: Vec<&str> = static_access_reads(&erased).iter().map(|read| read.variable).collect();
            assert_eq!(access, absence, "`{source}`");
        }
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
