//! The guard cases of the type-narrowing conformance matrix, shared by the
//! library's `scope_site_conformance.rs` and DMLS's `expression_scopes.rs` so
//! both read the same rows.
//!
//! A [`BlockCase`] is one read inside zero or more nested `::block when=`
//! guards, with whether the shared type scope proves the read present there.
//! Every row is written against [`SCHEMA`] with no frontmatter values, so the
//! read is always missing at analysis time: a proven read is what a guard
//! excuses, and an unproven one is what still fails. The rows prove site
//! wiring; the core algebra is covered row by row in `expression_type_scope.rs`.

/// Declares [`GuardCase`] and [`GuardCase::ALL`] from one list, so the list
/// cannot omit a variant.
macro_rules! guard_cases {
    ($($(#[$meta:meta])* $variant:ident),+ $(,)?) => {
        /// One behavior of a guard the matrix checks at every site.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum GuardCase {
            $($(#[$meta])* $variant),+
        }

        impl GuardCase {
            pub const ALL: &'static [GuardCase] = &[$(GuardCase::$variant),+];
        }
    };
}

guard_cases! {
    /// `when: x` proves `x` present.
    TrueGuard,
    /// `!x` (the false side) proves nothing about `x`.
    NegatedGuard,
    /// Nested guards compose: each enclosing guard proves its own value.
    NestedGuards,
    /// A later `&&` operand sees the earlier one, and both hold on the true side.
    ConjunctionOperand,
    /// `x || y` proves neither alone.
    DisjunctionJoin,
    /// Bare `x` and `fm.x` are one document binding.
    FmAlias,
    /// Predicates resolve through the registry, aliases and case included.
    PredicateAlias,
    /// `x != null` proves `x`, and so does `x == null` (only `''` equals
    /// `null`); the false side of `x != null` does not.
    NotNull,
    /// `x == 'v'` proves `x`; `x != 'v'` does not.
    Equality,
    /// An optional declaration does not make a missing value present.
    OptionalDeclaration,
    /// An undeclared value is proven by its guard and still fails without one.
    UnknownValue,
    /// A guard contradicting the declaration keeps the incoming type and
    /// gains no presence evidence.
    ContradictoryGuard,
    /// A fact on `x.a` proves nothing about `x.b` or `x`.
    SiblingOrParentPath,
    /// A guard on `x` does not defer a missing `y`.
    OtherValueNotDeferred,
    /// A write between the guard and the site ends the proof.
    WriteInvalidation,
    /// A false guard skips its site; a selected one runs with the live value.
    RuntimeSelection,
    /// Planned: a guard on a discriminant narrows the enclosing nested union.
    NestedUnionProjection,
    /// Planned: a guard on a function result narrows its argument.
    FunctionResultNarrowing,
    /// Planned: typed projections of the host globals `seq`, `loop`, `group`.
    HostGlobalTypes,
    /// Planned: `file_exists` facts that follow argument conversion.
    ConversionAwareFileExists,
}

/// The declarations every [`BlockCase`] reads against, as `$schema` lines.
pub const SCHEMA: &str = "x: string\ntarget: [string, number, \"null\"]\nchoice: \"enum(a, b)\"\n";

/// One read under nested `::block when=` guards, outermost first.
#[derive(Clone, Copy, Debug)]
pub struct BlockCase {
    pub case: GuardCase,
    pub guards: &'static [&'static str],
    pub read: &'static str,
    /// Whether the shared scope proves `read` present inside every guard.
    pub proven: bool,
}

const fn row(case: GuardCase, guards: &'static [&'static str], read: &'static str, proven: bool) -> BlockCase {
    BlockCase { case, guards, read, proven }
}

/// The rows for every case a body guard can express. Cases with no row here
/// (writes, runtime selection, planned projections) are covered elsewhere or
/// are planned; the matrix states which.
pub const BLOCK_CASES: &[BlockCase] = &[
    row(GuardCase::TrueGuard, &["x"], "x", true),
    row(GuardCase::NegatedGuard, &["!x"], "x", false),
    row(GuardCase::NestedGuards, &["x", "target"], "x", true),
    row(GuardCase::NestedGuards, &["x", "target"], "target", true),
    row(GuardCase::ConjunctionOperand, &["target && x"], "x", true),
    row(GuardCase::ConjunctionOperand, &["target && x"], "target", true),
    row(GuardCase::DisjunctionJoin, &["x || target"], "x", false),
    row(GuardCase::FmAlias, &["fm.x"], "x", true),
    row(GuardCase::FmAlias, &["x"], "fm.x", true),
    row(GuardCase::PredicateAlias, &["is_string(x)"], "x", true),
    row(GuardCase::PredicateAlias, &["isString(x)"], "x", true),
    row(GuardCase::PredicateAlias, &["file_exists(x)"], "x", true),
    row(GuardCase::PredicateAlias, &["fileExists(x)"], "x", true),
    row(GuardCase::PredicateAlias, &["!isNull(x)"], "x", true),
    row(GuardCase::PredicateAlias, &["is_null(x)"], "x", false),
    row(GuardCase::NotNull, &["x != null"], "x", true),
    // Two nulls are never equal, so `x == null` holds only for `''`: present.
    row(GuardCase::NotNull, &["x == null"], "x", true),
    row(GuardCase::NotNull, &["!(x != null)"], "x", false),
    row(GuardCase::Equality, &["x == 'v'"], "x", true),
    row(GuardCase::Equality, &["choice == 'a'"], "choice", true),
    row(GuardCase::Equality, &["x != 'v'"], "x", false),
    row(GuardCase::OptionalDeclaration, &[], "x", false),
    row(GuardCase::UnknownValue, &["u"], "u", true),
    row(GuardCase::UnknownValue, &[], "u", false),
    row(GuardCase::ContradictoryGuard, &["choice == 'z'"], "choice", false),
    row(GuardCase::SiblingOrParentPath, &["obj.a"], "obj.a", true),
    row(GuardCase::SiblingOrParentPath, &["obj.a"], "obj.b", false),
    row(GuardCase::SiblingOrParentPath, &["obj.a"], "obj", false),
    row(GuardCase::OtherValueNotDeferred, &["x"], "target", false),
];

/// The rows of `case`.
pub fn rows(case: GuardCase) -> impl Iterator<Item = &'static BlockCase> {
    BLOCK_CASES.iter().filter(move |row| row.case == case)
}

/// `$schema` frontmatter declaring [`SCHEMA`], followed by `extra` lines.
pub fn frontmatter(extra: &str) -> String {
    let schema: String = SCHEMA.lines().map(|line| format!("  {line}\n")).collect();
    format!("---\n$schema:\n{schema}{extra}---\n")
}

/// `inner` wrapped in `row`'s guards, outermost first.
pub fn wrap(row: &BlockCase, inner: &str) -> String {
    let open: String = row.guards.iter().map(|guard| format!("::block when=\"{guard}\"\n")).collect();
    let close = "::end-block\n".repeat(row.guards.len());
    format!("{open}{inner}{close}")
}
