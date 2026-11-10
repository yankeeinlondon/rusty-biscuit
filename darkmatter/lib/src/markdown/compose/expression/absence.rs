//! Which variable reads state that a value may be absent (spec Requirement 4).
//!
//! A well-formed identifier that resolves to nothing is not an error, but it is
//! usually a typo, so full-document composition warns about it — unless the
//! author handled the absence explicitly. [`AbsenceScope`] is the one shared
//! classifier of those explicit constructs (Ruling R-2). Two rules decide it:
//!
//! - **Intentional absence handling**, a warning policy only: the left
//!   operand of `||` (`x || "d"`), and the operands of an `||` chain that is
//!   itself a left operand (`a || b || "d"`); the condition of a ternary
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
//! Member reads follow the same rules. [`static_member_reads`] records each
//! selector below a binding (a dotted segment after a variable's root, a
//! postfix member name, a string-literal bracket key) from the same walk, so
//! `server.nmae || "none"`, `is_null((server).nmae)`, and a read a branch
//! fact proves present are handled exactly as a variable in that position
//! would be, and a fact about `server.a` covers `server.a.b` but not
//! `server.c`.
//!
//! [`static_access_reads`] is the same read inventory without the absence
//! classification, for a host that filters reads by its own names: each read
//! is a variable plus the selectors applied to it, so `fm['key']` and
//! `state.a[0]` keep the path they read.

use simplified_schema::expression::ast::{
    CallArgument, SpannedCallArgument, Expr, SpannedBindingPattern, SpannedCallback, SpannedExpr, SpannedExprKind,
};
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
                (if facts.lexical().is_some() {
                    facts.subject(&super::Expr::Variable(path.to_string()))
                } else {
                    UNTYPED.variable_subject(path)
                })
                    .is_some_and(|subject| facts.guards_path_or_ancestor(&subject))
            })
    }

    /// [`Self::handles`] for the postfix or bracket access `expr`
    /// (`(x).a`, `f(x).a`, `x['a']`): the direct position, or a branch fact
    /// about the subject `expr` reads or one of its ancestors.
    pub(crate) fn handles_expr(&self, expr: &super::Expr) -> bool {
        self.handles_absence
            || self
                .facts
                .is_some_and(|facts| facts.subject(expr).is_some_and(|subject| facts.guards_path_or_ancestor(&subject)))
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

    /// The left operand of `||` or a ternary condition.
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
            // `try` takes `unknown | null` only so that a null operand is a
            // success; recovering from a failed read is not handling an
            // absent value, so its operands still warn.
            handles_absence: (logical || !self.consumed_by_call)
                && !is_recovery(function)
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

/// Whether `name` is the lazy recovery form `try`, as the evaluator
/// dispatches it.
fn is_recovery(name: &str) -> bool {
    super::functions::lazy_function(name).is_some_and(|(_, kind)| kind == super::functions::LazyKind::Recover)
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
    /// Whether the author handled this read's absence (`||` left operand,
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
/// and every `||` operand, since it cannot know which one runs, each
/// under its own side's branch facts. It does not parse or evaluate.
pub fn static_variable_reads(expr: &SpannedExpr) -> Vec<StaticVariableRead<'_>> {
    let mut reads = StaticReads::default();
    walk_static(expr, AbsenceScope::default(), &mut reads);
    reads.variables
}

/// One member selector found by [`static_member_reads`]: a dotted segment
/// of a variable after its root, a segment of a postfix member name, or a
/// string-literal bracket key.
#[derive(Debug, Clone)]
pub(crate) struct StaticMemberRead<'e> {
    /// The `Variable`, `MemberAccess`, or `Index` node holding the selector.
    pub node: &'e SpannedExpr,
    /// The 0-based selector within `node`: for a variable, the segment
    /// after its root (`server`, `fm.server`, `local.h`); for a postfix
    /// name, the segment of the name; always 0 for a bracket key.
    pub step: usize,
    /// Whether the read's absence is handled, by the rules that apply to
    /// variable reads: the direct position, or a branch fact about the path
    /// through this step or one of its ancestors.
    pub handles_absence: bool,
}

/// Every member selector of `expr`, classified by the same absence rules as
/// [`static_variable_reads`] and in the same static walk, every branch and
/// callback body included. Reads rooted in a lexical local are kept; the
/// selectors of one node are listed together, in step order.
pub(crate) fn static_member_reads(expr: &SpannedExpr) -> Vec<StaticMemberRead<'_>> {
    let mut reads = StaticReads::default();
    walk_static(expr, AbsenceScope::default(), &mut reads);
    reads.members
}

/// The number of leading segments of the variable `path` that name its
/// binding rather than a member: `fm.<key>` and `local.<name>` take two, bare
/// `fm` and `local` name no member, and any other root takes one.
pub(crate) fn variable_root_segments(path: &str) -> Option<usize> {
    match super::fm_namespace::fm_namespace_path(path) {
        Some(super::fm_namespace::FmPath::Whole) => return None,
        Some(super::fm_namespace::FmPath::Member(_)) => return Some(2),
        None => {}
    }
    if root_of(path) == super::binding::LOCAL_ROOT {
        return path.contains('.').then_some(2);
    }
    Some(1)
}

/// What [`walk_static`] collects.
#[derive(Default)]
struct StaticReads<'e> {
    variables: Vec<StaticVariableRead<'e>>,
    members: Vec<StaticMemberRead<'e>>,
}

impl<'e> StaticReads<'e> {
    fn member(&mut self, node: &'e SpannedExpr, step: usize, handles_absence: bool) {
        self.members.push(StaticMemberRead {
            node,
            step,
            handles_absence,
        });
    }
}

/// [`static_variable_reads`] for an expression evaluated inside lexical
/// scopes that bind `locals`.
///
/// A read is lexical, not a document read, when its root is one of `locals`
/// (the implicit spelling) or the reserved `local` root (the explicit one);
/// those reads are left out. Every other read is a free capture and is kept
/// with its absence classification: `fm.x`, `ctx.*`, `current.*`, host
/// globals, and document properties no local shadows. Pass every enclosing
/// scope's names, nearest and outer alike.
pub fn static_variable_reads_in_scope<'e>(expr: &'e SpannedExpr, locals: &[&str]) -> Vec<StaticVariableRead<'e>> {
    let mut reads = static_variable_reads(expr);
    reads.retain(|read| !is_lexical_read(read.path, locals));
    reads
}

/// The free reads of a callback body: [`static_variable_reads_in_scope`]
/// with the callback's own pattern names added to `outer_locals`.
///
/// This is what a callback contributes to document, context, and host
/// dependencies; its parameters contribute none.
pub fn static_callback_reads<'e>(callback: &'e SpannedCallback, outer_locals: &[&str]) -> Vec<StaticVariableRead<'e>> {
    let mut reads = StaticReads::default();
    callback_reads(callback, outer_locals, AbsenceScope::default(), &mut reads);
    reads.variables
}

/// Walks `callback`'s body into `sink`: its free variable reads, and every
/// member read, a parameter's included.
fn callback_reads<'e>(callback: &'e SpannedCallback, outer_locals: &[&str], scope: AbsenceScope<'_>, sink: &mut StaticReads<'e>) {
    let mut locals: Vec<&str> = outer_locals.to_vec();
    match &callback.pattern {
        SpannedBindingPattern::Identifier(name) => locals.push(&name.value),
        SpannedBindingPattern::Fields(fields) => locals.extend(fields.iter().map(|field| field.value.as_str())),
    }
    let facts = scope.facts();
    let bindings = locals.iter().map(|name| super::scope::TypedLocal::new(*name, super::scope::Shape::Unknown { non_null: false })).collect();
    let lexical = match facts.lexical() {
        Some(parent) => parent.child(bindings),
        None => super::scope::LexicalTypes::new(bindings),
    };
    let inner = facts.within(lexical);
    let mut reads = StaticReads::default();
    walk_static(&callback.body, scope.operand().with_facts(&inner), &mut reads);
    reads.variables.retain(|read| !is_lexical_read(read.path, &locals));
    sink.variables.extend(reads.variables);
    sink.members.extend(reads.members);
}

fn is_lexical_read(path: &str, locals: &[&str]) -> bool {
    let root = root_of(path);
    root == super::binding::LOCAL_ROOT || locals.contains(&root)
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

/// Free access reads from a subtree whose enclosing callback bindings are
/// supplied separately. Explicit `fm` reads remain document dependencies.
pub(crate) fn static_access_reads_in_scope<'e>(expr: &'e Expr, locals: &[&str]) -> Vec<StaticAccessRead<'e>> {
    let mut reads = static_access_reads(expr);
    reads.retain(|read| !is_lexical_read(read.variable, locals));
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
        | Expr::LogicalOr { left, right } => {
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
        Expr::CallbackCall { args, .. } => {
            for arg in args {
                match arg {
                    CallArgument::Value(value) => walk_access(value, reads),
                    CallArgument::Callback(callback) => {
                        let start = reads.len();
                        walk_access(&callback.body, reads);
                        let names: Vec<_> = callback.pattern.names().collect();
                        let mut free = reads.split_off(start);
                        free.retain(|read| !is_lexical_read(read.variable, &names));
                        reads.extend(free);
                    }
                }
            }
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
/// of `and`/`&&` or `or`/`||`, the true or false side of the
/// operands before it. These are the facts [`static_variable_reads`] reads
/// each variable under, applied to a typed scope so a static consumer (DMLS
/// hover) sees the type at that node. A `target` no node spans gets the facts
/// of the deepest node containing it. Passive, like [`TypeScope::assume`].
pub fn branch_scope<'d>(expr: &SpannedExpr, target: &SourceSpan, site: TypeScope<'d>) -> TypeScope<'d> {
    let holds = |node: &SpannedExpr| node.span.start <= target.start && target.end <= node.span.end;
    match &expr.kind {
        SpannedExprKind::LogicalOr { left, right } if holds(right) => {
            let facts = site.assume(&left.erase(), Polarity::False);
            branch_scope(right, target, facts)
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
        SpannedExprKind::CallbackCall { args, .. } => {
            for arg in args {
                match arg {
                    SpannedCallArgument::Value(value) if holds(value) => return branch_scope(value, target, site),
                    SpannedCallArgument::Callback(callback) if holds(&callback.body) => {
                        let inner = super::call_check::callback_scope(callback, args, &site).unwrap_or_else(|_| site.clone());
                        return branch_scope(&callback.body, target, inner);
                    }
                    _ => {}
                }
            }
            site
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
        SpannedExprKind::LogicalOr { left, .. } => vec![left],
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
    reads: &mut StaticReads<'e>,
) {
    let operand = scope.operand();
    match &expr.kind {
        SpannedExprKind::Variable(path) => {
            reads.variables.push(StaticVariableRead {
                path,
                span: expr.span.clone(),
                handles_absence: scope.handles(path),
            });
            if let Some(root) = variable_root_segments(path) {
                let mut through = 0;
                for (index, segment) in path.split('.').enumerate() {
                    through += segment.len() + usize::from(index > 0);
                    if index >= root {
                        reads.member(expr, index - root, scope.handles(&path[..through]));
                    }
                }
            }
        }
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
            if matches!(index.kind, SpannedExprKind::StringLiteral(_)) {
                reads.member(expr, 0, scope.handles_expr(&expr.erase()));
            }
            walk_static(index, operand, reads);
        }
        SpannedExprKind::MemberAccess { base, name } => {
            walk_static(base, scope, reads);
            let base = base.erase();
            let mut through = 0;
            for (step, segment) in name.split('.').enumerate() {
                through += segment.len() + usize::from(step > 0);
                let access = Expr::MemberAccess {
                    base: Box::new(base.clone()),
                    name: name[..through].to_string(),
                };
                reads.member(expr, step, scope.handles_expr(&access));
            }
        }
        SpannedExprKind::LogicalOr { left, right } => {
            walk_static(left, scope.absence_check(), reads);
            let facts = scope.facts().assume(&left.erase(), Polarity::False);
            walk_static(right, scope.with_facts(&facts), reads);
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
        SpannedExprKind::CallbackCall { name, args } => {
            for (index, arg) in args.iter().enumerate() {
                match arg {
                    SpannedCallArgument::Value(value) => walk_static(value, scope.call_argument(name, index, args.len()), reads),
                    SpannedCallArgument::Callback(callback) => callback_reads(callback, &[], scope, reads),
                }
            }
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
///
/// An observer that sets [`OBSERVES_MEMBERS`](Self::OBSERVES_MEMBERS) also
/// receives every member selector node the evaluator reached
/// ([`member_read`](Self::member_read)) and the root of each evaluated
/// expression once it finishes, successfully or not
/// ([`finish_expression`](Self::finish_expression)).
pub(crate) trait MissingRootObserver {
    /// `false` only for the no-op observer, so unobserved evaluation skips the
    /// classification check entirely.
    const OBSERVES: bool = true;

    /// Whether the evaluator reports reached member selectors; `false` keeps
    /// evaluation exactly as it was for every observer but [`Observation`].
    const OBSERVES_MEMBERS: bool = false;

    fn missing_root(&mut self, root: &str);

    /// A dotted variable, a postfix member, or a string-literal bracket key
    /// was reached, identified by its node's address in the evaluated tree.
    /// Recorded before the base is evaluated, so a read reached before a
    /// failure is kept.
    fn member_read(&mut self, _node: usize) {}

    /// The evaluation of `root` finished; the addresses recorded since the
    /// last call belong to it.
    fn finish_expression(&mut self, _root: &Expr) {}
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

/// The observation of a migrated composition surface: the missing roots, as
/// a `Vec<MissingRoot>` observer collects them, and the member selectors the
/// evaluator reached, as node ordinals of the evaluated expression
/// ([`expr_nodes`](super::member_check::expr_nodes)), which name the same
/// nodes as a static finding's `node_ordinal`.
#[derive(Debug, Default)]
pub(crate) struct Observation {
    pub(crate) roots: Vec<MissingRoot>,
    /// Ordinals of the expressions finished so far, each expression's
    /// distinct and ascending.
    reached: Vec<usize>,
    /// Node addresses reached in the expression still being evaluated.
    addresses: Vec<usize>,
}

impl Observation {
    /// The reached ordinals recorded since the last call.
    pub(crate) fn take_reached(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.reached)
    }
}

impl Extend<MissingRoot> for Observation {
    fn extend<I: IntoIterator<Item = MissingRoot>>(&mut self, roots: I) {
        self.roots.extend(roots);
    }
}

impl MissingRootObserver for Observation {
    const OBSERVES_MEMBERS: bool = true;

    fn missing_root(&mut self, root: &str) {
        self.roots.missing_root(root);
    }

    fn member_read(&mut self, node: usize) {
        self.addresses.push(node);
    }

    fn finish_expression(&mut self, root: &Expr) {
        self.reached.extend(super::member_check::reached_ordinals(root, &mut self.addresses));
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
    /// branches and every `||` operand are visited.
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

    fn callback(pattern: &str, body: &str) -> SpannedCallback {
        SpannedCallback {
            pattern_span: 0..pattern.len(),
            pattern: SpannedBindingPattern::Identifier(crate::markdown::span::Spanned::new(pattern.to_string(), 0..pattern.len())),
            arrow: 0..0,
            body: crate::markdown::compose::expression::parse_spanned(body).unwrap(),
        }
    }

    #[test]
    fn a_callback_contributes_free_captures_and_no_document_read_for_its_locals() {
        let callback = callback("p", "p.name || title || fm.p || local.q || ctx.today || current.branch || seq.n");
        let paths: Vec<&str> = static_callback_reads(&callback, &[]).iter().map(|read| read.path).collect();
        assert_eq!(paths, ["title", "fm.p", "ctx.today", "current.branch", "seq.n"]);
        let shadowed: Vec<&str> = static_callback_reads(&callback, &["title", "seq"]).iter().map(|read| read.path).collect();
        assert_eq!(shadowed, ["fm.p", "ctx.today", "current.branch"]);
    }

    #[test]
    fn lexical_exclusion_keeps_the_absence_classification_of_the_remaining_reads() {
        let expr = crate::markdown::compose::expression::parse_spanned("x ? title : other").unwrap();
        let reads = static_variable_reads_in_scope(&expr, &["x"]);
        let flags: Vec<(&str, bool)> = reads.iter().map(|read| (read.path, read.handles_absence)).collect();
        assert_eq!(flags, [("title", false), ("other", false)]);
        let plain = static_variable_reads(&expr);
        assert_eq!(plain.len(), 3, "the unscoped inventory still lists the shadowed name");
    }

    mod member_reads {
        use super::*;
        use crate::markdown::compose::expression::parse_spanned;

        /// Each member selector of `source` as `(name, handled)`, in walk
        /// order: a variable's segment after its root, a postfix segment, or
        /// a bracket key.
        fn members(source: &str) -> Vec<(String, bool)> {
            let expr = parse_spanned(source).unwrap_or_else(|error| panic!("{source}: {error}"));
            static_member_reads(&expr)
                .into_iter()
                .map(|read| {
                    let name = match &read.node.kind {
                        SpannedExprKind::Variable(path) => {
                            let root = variable_root_segments(path).expect("a variable record has a member");
                            path.split('.').nth(root + read.step).unwrap().to_string()
                        }
                        SpannedExprKind::MemberAccess { name, .. } => name.split('.').nth(read.step).unwrap().to_string(),
                        SpannedExprKind::Index { index, .. } => match &index.kind {
                            SpannedExprKind::StringLiteral(key) => key.clone(),
                            other => panic!("unexpected index {other:?}"),
                        },
                        other => panic!("unexpected node {other:?}"),
                    };
                    (name, read.handles_absence)
                })
                .collect()
        }

        fn rows(expected: &[(&str, bool)]) -> Vec<(String, bool)> {
            expected.iter().map(|(name, handled)| ((*name).to_string(), *handled)).collect()
        }

        /// The absence table of the member check, one row per expression,
        /// for `server: { name: string }`.
        #[test]
        fn member_reads_follow_the_absence_table() {
            for (source, expected) in [
                ("server.nmae", &[("nmae", false)][..]),
                ("server.nmae || \"none\"", &[("nmae", true)]),
                ("server.nmae ? 1 : 0", &[("nmae", true)]),
                ("is_null(server.nmae)", &[("nmae", true)]),
                ("min(server.nmae, 5)", &[("nmae", false)]),
                ("is_empty(trim(server.nmae))", &[("nmae", false)]),
                ("server.nmae == \"x\" ? 1 : 0", &[("nmae", false)]),
                ("server.nmae ? server.nmae : \"none\"", &[("nmae", true), ("nmae", true)]),
                ("server.nmae ? \"some\" : server.nmae", &[("nmae", true), ("nmae", false)]),
            ] {
                assert_eq!(members(source), rows(expected), "{source}");
            }
        }

        #[test]
        fn postfix_and_bracket_selectors_follow_the_same_rules() {
            for (source, expected) in [
                ("(server).nmae", &[("nmae", false)][..]),
                ("(server).nmae || \"none\"", &[("nmae", true)]),
                ("is_null(server[\"nmae\"])", &[("nmae", true)]),
                ("(server).nmae ? (server).nmae : 0", &[("nmae", true), ("nmae", true)]),
                ("server[\"nmae\"] ? 0 : server[\"nmae\"]", &[("nmae", true), ("nmae", false)]),
                ("f(x).a.b", &[("a", false), ("b", false)]),
                ("(server).a ? (server).a.b : 0", &[("a", true), ("a", true), ("b", true)]),
                ("(server).a.b ? (server).a : 0", &[("a", true), ("b", true), ("a", false)]),
            ] {
                assert_eq!(members(source), rows(expected), "{source}");
            }
        }

        #[test]
        fn a_fact_covers_its_descendants_and_never_a_sibling() {
            assert_eq!(members("server.a ? server.b : 0"), rows(&[("a", true), ("b", false)]));
            assert_eq!(members("server.a ? server.a.b : 0"), rows(&[("a", true), ("a", true), ("b", true)]));
            assert_eq!(members("server ? server.a.b : 0"), rows(&[("a", true), ("b", true)]));
            assert_eq!(members("is_null(server.a) ? 0 : server.a.b"), rows(&[("a", true), ("a", true), ("b", true)]));
        }

        /// A directive gate is walked from the default scope: a bare
        /// condition there is an ordinary read, not a ternary condition.
        #[test]
        fn a_gate_scope_handles_nothing() {
            assert_eq!(members("server.nmae"), rows(&[("nmae", false)]));
            assert!(!AbsenceScope::default().handles_expr(&parse("(server).nmae").unwrap()));
        }

        /// `try` recovers failures; it does not handle an absent read.
        #[test]
        fn try_arguments_are_ordinary_reads() {
            assert_eq!(members("try(server.nmae, \"x\")"), rows(&[("nmae", false)]));
            assert_eq!(members("try(server, {}).nmae"), rows(&[("nmae", false)]));
        }

        #[test]
        fn namespace_wrappers_and_locals_name_their_binding_first() {
            assert_eq!(members("fm.server.nmae"), rows(&[("nmae", false)]));
            assert_eq!(members("local.h.nmae"), rows(&[("nmae", false)]));
            assert!(members("fm").is_empty());
            assert!(members("fm.server").is_empty());
            assert!(members("local.h").is_empty());
            assert_eq!(members("fm[\"server\"].nmae"), rows(&[("server", false), ("nmae", false)]));
            assert_eq!(members("server[\"a.b\"]"), rows(&[("a.b", false)]));
            assert!(members("server[key]").is_empty(), "a computed key is no selector");
            assert!(members("hosts[0]").is_empty(), "a numeric index is no selector");
        }

        /// Member reads in a callback body are kept, a parameter's included,
        /// and its free variable reads are unchanged.
        #[test]
        fn callback_member_reads_include_the_parameters() {
            assert_eq!(members("map(hosts, h => h.nmae || h.name.x)"), rows(&[("nmae", true), ("name", false), ("x", false)]));
            let expr = parse_spanned("map(hosts, h => h.nmae || title)").unwrap();
            let paths: Vec<(&str, bool)> = static_variable_reads(&expr).iter().map(|read| (read.path, read.handles_absence)).collect();
            assert_eq!(paths, [("hosts", true), ("title", false)]);
        }

        /// Recording members does not change the variable inventory.
        #[test]
        fn static_variable_reads_are_unchanged_by_member_records() {
            for (source, expected) in [
                ("server.a.b || x", &[("server.a.b", true, 0..10), ("x", false, 14..15)][..]),
                ("(server).nmae ? y : z", &[("server", true, 1..7), ("y", false, 16..17), ("z", false, 20..21)]),
                ("is_null(a['k']) && b.c", &[("a", true, 8..9), ("b.c", true, 19..22)]),
                ("filter(xs, x => x.ok && fm.y)", &[("xs", true, 7..9), ("fm.y", true, 24..28)]),
            ] {
                let expr = parse_spanned(source).unwrap();
                let reads: Vec<(&str, bool, SourceSpan)> =
                    static_variable_reads(&expr).into_iter().map(|read| (read.path, read.handles_absence, read.span)).collect();
                let expected: Vec<(&str, bool, SourceSpan)> = expected.to_vec();
                assert_eq!(reads, expected, "{source}");
            }
        }
    }

    mod member_observation {
        use super::*;
        use crate::markdown::compose::expression::{EvaluationLookup, evaluate_observed};
        use serde_json::{Value, json};

        /// Resolves a dotted path through one JSON object.
        struct Json(Value);

        impl EvaluationLookup for Json {
            fn get(&self, path: &str) -> Option<Value> {
                path.split('.').try_fold(&self.0, |value, segment| value.get(segment)).cloned()
            }
        }

        fn state() -> Json {
            Json(json!({
                "server": { "name": "web" },
                "hosts": [{ "name": "a" }, { "name": "b" }, { "name": "c" }],
                "empty": [],
                "yes": true,
                "no": false,
            }))
        }

        /// The ordinals `source` reached, and the result.
        fn reached(source: &str) -> (Vec<usize>, Result<Value, String>) {
            let expr = parse(source).unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut observation = Observation::default();
            let result = evaluate_observed(&expr, &state(), &mut observation).map_err(|error| error.to_string());
            (observation.take_reached(), result)
        }

        #[test]
        fn each_selector_form_reports_its_own_node() {
            // Ordinals are pre-order: `(server).nmae` is MemberAccess 0,
            // Paren 1, Variable 2; `server["nmae"]` is Index 0, Variable 1,
            // the key 2; in `[server.name, server.nmae]` the items are 1, 2.
            for (source, expected) in [
                ("server.nmae", vec![0]),
                ("(server).nmae", vec![0]),
                ("server[\"nmae\"]", vec![0]),
                ("[server.name, server.nmae]", vec![1, 2]),
                ("server", vec![]),
                ("hosts[0]", vec![]),
            ] {
                assert_eq!(reached(source).0, expected, "{source}");
            }
        }

        #[test]
        fn unevaluated_operands_report_nothing() {
            // Ternary: condition 1, then 2, else 3.
            assert_eq!(reached("yes ? server.a : server.b").0, [2]);
            assert_eq!(reached("no ? server.a : server.b").0, [3]);
            // `||`: left 1, right 2; a truthy left skips the right.
            assert_eq!(reached("server.name || server.nmae").0, [1]);
            assert_eq!(reached("server.nmae || server.other").0, [1, 2]);
        }

        #[test]
        fn a_callback_reports_once_per_expression_and_never_over_nothing() {
            // `map(src, h => h.nmae)`: CallbackCall 0, source 1, body 2.
            assert_eq!(reached("map(empty, h => h.nmae)").0, Vec::<usize>::new());
            assert_eq!(reached("map(hosts, h => h.nmae)").0, [2]);
        }

        #[test]
        fn a_read_before_a_failure_is_kept() {
            let (ordinals, result) = reached("upper(server.nmae, 1, 2)");
            assert!(result.is_err(), "the call must fail: {result:?}");
            assert_eq!(ordinals, [1]);
        }

        #[test]
        fn each_expression_is_separate_and_take_drains() {
            let first = parse("server.a").unwrap();
            let second = parse("[1, server.b]").unwrap();
            let mut observation = Observation::default();
            evaluate_observed(&first, &state(), &mut observation).unwrap();
            evaluate_observed(&second, &state(), &mut observation).unwrap();
            assert_eq!(observation.take_reached(), [0, 2]);
            assert!(observation.take_reached().is_empty());
        }

        #[test]
        fn root_observers_are_unchanged() {
            let expr = parse("missing.nmae").unwrap();
            let mut observation = Observation::default();
            let mut roots: Vec<MissingRoot> = Vec::new();
            let observed = evaluate_observed(&expr, &state(), &mut observation).unwrap();
            assert_eq!(evaluate_observed(&expr, &state(), &mut roots).unwrap(), observed);
            assert_eq!(evaluate_observed(&expr, &state(), &mut ()).unwrap(), observed);
            assert_eq!(observation.roots, roots);
            assert_eq!(roots.len(), 1);
            assert_eq!(observation.take_reached(), [0]);
        }
    }
}
