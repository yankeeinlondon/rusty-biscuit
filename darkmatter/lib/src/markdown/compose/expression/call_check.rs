//! Design-time call checking: the five argument-type categories, whole-call
//! classification, and the passive walk that finds every authored call.
//!
//! Composition and DMLS both read these answers, so an editor error and a
//! composition rejection cannot disagree. Everything here is passive: no
//! expression is evaluated, no handler is entered, nothing reads a file or a
//! provider. A literal is judged by binding its value through the runtime
//! binder; any other argument by its design-time type at the call site, read
//! from a [`TypeScope`] so guards narrow it.
//!
//! A recursive-typed argument keeps its named reference and is judged through
//! the definition table its scope carries
//! ([`DeclaredTypes::with_definitions`](super::DeclaredTypes::with_definitions)):
//! subtyping is coinductive and conversion fit reads the reference's body
//! without re-entering it, so every check terminates.
//!
//! The rules are the spec's (`2026-10-28-schema-enhancements`, "Argument-Type
//! Categories"); `docs/` describes them for authors.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::expression::{
//!     ArgumentCategory, DeclaredTypes, TypeScope, check_expression_calls, parse_spanned,
//! };
//! use darkmatter::markdown::schemas::parse_yaml_schema;
//!
//! let schema = parse_yaml_schema(&serde_yaml_ng::from_str("flag: boolean(required)").unwrap()).unwrap();
//! let declared = DeclaredTypes::new(Some(&schema));
//! let expr = parse_spanned("true ? 'ok' : min(flag, 5)").unwrap();
//! let checks = check_expression_calls(&expr, &TypeScope::unguarded(&declared));
//! assert_eq!(checks[0].category(), ArgumentCategory::Incompatible);
//! ```

use std::fmt;

use serde_json::Value;

use super::absence::branch_scope;
use super::ast::{Expr, SpannedCallArgument, SpannedCallback, SpannedExpr, SpannedExprKind};
use super::call_binding::{
    CallBindingError, CompiledParameter, CompiledSignature, FunctionRegistration, NoNamedTypes, SignatureProblem,
    shape_from_property_def_with_definitions,
};
use super::functions;
use super::prepared::PreparedValue;
use super::scope::{Polarity, Shape as ScopeShape, TypeScope};
use crate::markdown::coercion::{Additional, ContentFormat, NumberShape, Shape, ValueKind, bind};
use crate::markdown::schemas::simplified::serialize_property_atom;
use crate::markdown::schemas::{Constraint, DefinitionTable, PropertyAtom, PropertyDef, is_subtype_with_definitions};
use crate::markdown::span::SourceSpan;

/// How an argument's design-time type relates to a parameter's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgumentCategory {
    /// 1: a perfect match, or a conversion that is always safe.
    Compatible,
    /// 2: a perfect match if the value exists; it may be `null`, which the
    /// parameter rejects.
    NullableMatch,
    /// 3: might be the right type, might not.
    Uncertain,
    /// 4: the type is not known.
    Unknown,
    /// 5: known to be wrong.
    Incompatible,
}

impl ArgumentCategory {
    /// The category's number in the spec's table, 1 to 5.
    pub fn number(self) -> u8 {
        match self {
            Self::Compatible => 1,
            Self::NullableMatch => 2,
            Self::Uncertain => 3,
            Self::Unknown => 4,
            Self::Incompatible => 5,
        }
    }

    /// Order of concern: a proven error outranks a possible one, which
    /// outranks an unknown type, which outranks a possible `null`.
    fn rank(self) -> u8 {
        match self {
            Self::Compatible => 0,
            Self::NullableMatch => 1,
            Self::Unknown => 2,
            Self::Uncertain => 3,
            Self::Incompatible => 4,
        }
    }

    fn worst(self, other: Self) -> Self {
        if other.rank() > self.rank() { other } else { self }
    }
}

impl fmt::Display for ArgumentCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "category {}", self.number())
    }
}

/// An argument's design-time type.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentType {
    /// Inline syntax accepted only by a function parameter; never a JSON value.
    Callback,
    /// A literal, judged by its value.
    Literal(Value),
    /// A typed value: the narrowed type of a variable at the call site, a
    /// call's declared return type, or the type of a `||` or `?:` operand
    /// union.
    Typed(ScopeShape),
    /// No usable type: an undeclared or untyped variable, a call or operator
    /// expression whose type is `unknown`.
    Unknown,
}

impl ArgumentType {
    /// The design-time type of `arg` at the site `scope` describes.
    ///
    /// Literals (including lists and objects of literals, and a negated
    /// number) carry their value; anything else carries
    /// [`expression_type`].
    pub fn of(arg: &SpannedExpr, scope: &TypeScope<'_>) -> Self {
        if let Some(value) = literal_value(arg) {
            return Self::Literal(value);
        }
        match expression_type(arg, scope) {
            ScopeShape::Unknown { .. } => Self::Unknown,
            shape @ ScopeShape::Arms { .. } => Self::Typed(shape),
        }
    }
}

/// The design-time type of `expr` at the site `scope` describes, without
/// evaluating anything:
///
/// | Expression | Type |
/// |---|---|
/// | a literal | that value |
/// | a variable, or a constant path on one | its type in `scope` |
/// | an object literal | a closed object declaring each member at its own type (shorthand `{a, b}` is `{a: a, b: b}`) |
/// | an array literal | an array of the union of its items' types |
/// | a call | the union of the declared `returns` of every overload its argument count can select |
/// | `x \|\| y` | `x`'s type without `null`, joined with `y`'s type where `x` is falsy |
/// | `c ? a : b` | `a`'s type where `c` holds, joined with `b`'s where it does not |
/// | any other operator | `unknown` |
///
/// Joins use [`ScopeShape::union`]. A call's own arguments are checked as a
/// call of their own ([`check_expression_calls`]); typing the call never
/// suppresses their findings.
pub fn expression_type(expr: &SpannedExpr, scope: &TypeScope<'_>) -> ScopeShape {
    if let Some(value) = literal_value(expr)
        && !value.is_array() && !value.is_object()
    {
        return ScopeShape::of_literal(&value);
    }
    match &expr.kind {
        SpannedExprKind::Paren(inner) => expression_type(inner, scope),
        SpannedExprKind::ObjectLiteral(entries) => ScopeShape::object_of(
            entries
                .iter()
                .map(|(key, value)| (key.value.clone(), expression_type(value, scope)))
                .collect(),
        ),
        SpannedExprKind::ArrayLiteral(items) => ScopeShape::array_of(
            &items
                .iter()
                .map(|item| expression_type(item, scope))
                .reduce(|left, right| left.union(&right))
                .unwrap_or(ScopeShape::Unknown { non_null: false }),
        ),
        SpannedExprKind::FunctionCall { name, args } => call_returns(name, args.len()),
        SpannedExprKind::CallbackCall { name, args } => {
            let Some(SpannedCallArgument::Callback(callback)) = args.get(1) else {
                return call_returns(name, args.len());
            };
            let item = callback_item(args, scope);
            let Ok(inner) = item.enter_spanned(scope, scope.lexical(), &callback.pattern) else {
                return call_returns(name, args.len());
            };
            let result = if name.eq_ignore_ascii_case("map") {
                expression_type(&callback.body, &inner)
            } else if name.eq_ignore_ascii_case("filter") {
                let kept = inner.assume(&callback.body.erase(), Polarity::True);
                let rebound = item.enter(&kept, scope.lexical(), &super::BindingPattern::Identifier("__item".into()))
                    .expect("an identifier accepts every item shape");
                rebound.type_of(&item.subject()).shape().clone()
            } else {
                return call_returns(name, args.len());
            };
            ScopeShape::array_of(&result)
        },
        SpannedExprKind::Fallback { primary, fallback } => {
            let present = expression_type(primary, scope).without_null();
            present.union(&expression_type(fallback, &scope.assume(&primary.erase(), Polarity::False)))
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = condition.erase();
            let then_type = expression_type(then_branch, &scope.assume(&condition, Polarity::True));
            then_type.union(&expression_type(else_branch, &scope.assume(&condition, Polarity::False)))
        }
        _ => scope
            .type_of_expr(&expr.erase())
            .map_or(ScopeShape::Unknown { non_null: false }, |ty| ty.shape().clone()),
    }
}

/// The union of the declared `returns` of every overload of `name` that
/// takes `count` arguments; `unknown` for a name nothing dispatches or a
/// count no overload takes. Overloads are never selected by argument type.
fn call_returns(name: &str, count: usize) -> ScopeShape {
    functions::inspect_registration(name, |registration| {
        registration
            .overloads()
            .iter()
            .filter(|signature| signature.accepts_count(count))
            .map(|signature| ScopeShape::of_type(signature.declared_returns()))
            .reduce(|left, right| left.union(&right))
    })
    .flatten()
    .unwrap_or(ScopeShape::Unknown { non_null: false })
}

impl fmt::Display for ArgumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(value) => write!(f, "literal {value}"),
            Self::Callback => f.write_str("inline callback"),
            Self::Unknown | Self::Typed(ScopeShape::Unknown { .. }) => f.write_str("unknown"),
            Self::Typed(ScopeShape::Arms { arms, nullable }) => {
                let mut parts: Vec<String> = arms.iter().map(type_text).collect();
                if *nullable {
                    parts.push("null".to_string());
                }
                if parts.is_empty() {
                    parts.push("never".to_string());
                }
                f.write_str(&parts.join(" | "))
            }
        }
    }
}

/// An alternative's type as an argument has it: presence markers describe
/// the property, not the value.
fn type_text(atom: &PropertyAtom) -> String {
    let mut bare = atom.clone();
    bare.description = None;
    bare.constraints.retain(|constraint| !matches!(constraint, Constraint::Required));
    serialize_property_atom(&bare)
}

/// The category of passing an argument of design-time type `source` to
/// `parameter`, ignoring every other argument of the call.
///
/// A literal binds through the runtime binder (category 1 or 5). For a typed
/// argument an exact fit is [`is_subtype`]; otherwise the engine's
/// conversion rules decide, judged on types: constraints and refinements are
/// checked at runtime and never warn. Registered source exclusions apply.
/// Use [`check_call`] for a whole call: arity, overloads, and numeric
/// recovery change an argument's verdict.
///
/// A recursive reference in `source` resolves in nothing here, so its
/// argument is category 4; use [`classify_argument_with_definitions`].
pub fn classify_argument(source: &ArgumentType, parameter: &CompiledParameter) -> ArgumentCategory {
    classify_argument_with_definitions(source, parameter, &DefinitionTable::default())
}

/// [`classify_argument`] for a source whose recursive references point into
/// `definitions`.
pub fn classify_argument_with_definitions(
    source: &ArgumentType,
    parameter: &CompiledParameter,
    definitions: &DefinitionTable,
) -> ArgumentCategory {
    if matches!(source, ArgumentType::Callback) {
        return if parameter.callback().is_some_and(|callback| {
            let parameters = callback.parameters.as_slice();
            parameters.len() == 1 && !parameters[0].optional && !parameters[0].rest
        }) { ArgumentCategory::Compatible } else { ArgumentCategory::Incompatible };
    }
    if parameter.callback().is_some() {
        return ArgumentCategory::Incompatible;
    }
    match source {
        ArgumentType::Callback => unreachable!("callbacks classified above"),
        ArgumentType::Literal(value) => {
            if parameter.binds(value) {
                ArgumentCategory::Compatible
            } else {
                ArgumentCategory::Incompatible
            }
        }
        // `unknown | null` takes every value, so even an unknown one fits.
        ArgumentType::Unknown | ArgumentType::Typed(ScopeShape::Unknown { .. })
            if parameter.admits_null() && accepts_every_value(parameter.shape()) =>
        {
            ArgumentCategory::Compatible
        }
        ArgumentType::Unknown | ArgumentType::Typed(ScopeShape::Unknown { .. }) => ArgumentCategory::Unknown,
        ArgumentType::Typed(ScopeShape::Arms { arms, nullable }) => {
            let null_fits = parameter.admits_null();
            let mut fits = Vec::with_capacity(arms.len() + 1);
            for arm in arms {
                fits.push(arm_fit(arm, parameter, definitions));
            }
            if *nullable {
                fits.push(if null_fits { Fit::Fits } else { Fit::Wrong });
            }
            if fits.is_empty() {
                return ArgumentCategory::Compatible;
            }
            let non_null = &fits[..arms.len()];
            if fits.iter().all(|fit| *fit == Fit::Wrong) {
                ArgumentCategory::Incompatible
            } else if non_null.iter().all(|fit| *fit == Fit::Fits) {
                if *nullable && !null_fits {
                    if arms.is_empty() {
                        ArgumentCategory::Incompatible
                    } else {
                        ArgumentCategory::NullableMatch
                    }
                } else {
                    ArgumentCategory::Compatible
                }
            } else if non_null.contains(&Fit::Unknown) && !non_null.contains(&Fit::Wrong) && !non_null.contains(&Fit::Maybe) {
                ArgumentCategory::Unknown
            } else {
                ArgumentCategory::Uncertain
            }
        }
    }
}

fn accepts_every_value(shape: &Shape) -> bool {
    match shape {
        Shape::Any => true,
        Shape::Union(options) => options.iter().any(|option| matches!(option, Shape::Any)),
        _ => false,
    }
}

/// Whether one non-null source alternative fits a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fit {
    Fits,
    Maybe,
    Unknown,
    Wrong,
}

impl Fit {
    fn best(self, other: Self) -> Self {
        let rank = |fit: Self| match fit {
            Self::Fits => 3,
            Self::Maybe => 2,
            Self::Unknown => 1,
            Self::Wrong => 0,
        };
        if rank(other) > rank(self) { other } else { self }
    }

    /// Every value fits only if every part fits; every value is wrong only
    /// if every part is.
    fn all(fits: impl IntoIterator<Item = Self>) -> Self {
        let fits: Vec<Self> = fits.into_iter().collect();
        if fits.is_empty() || fits.iter().all(|fit| *fit == Self::Fits) {
            Self::Fits
        } else if fits.iter().all(|fit| *fit == Self::Wrong) {
            Self::Wrong
        } else if fits.iter().all(|fit| matches!(fit, Self::Fits | Self::Unknown)) {
            Self::Unknown
        } else {
            Self::Maybe
        }
    }
}

fn arm_fit(arm: &PropertyAtom, parameter: &CompiledParameter, definitions: &DefinitionTable) -> Fit {
    // A literal alternative (a guard narrowed to `literal('release')`) is a
    // value: the binder decides it.
    if let Some(value) = arm.literal_value().filter(|_| !arm.is_array) {
        return if parameter.binds(value) { Fit::Fits } else { Fit::Wrong };
    }
    let Ok(source) = shape_from_property_def_with_definitions(&PropertyDef::Single(arm.clone()), &NoNamedTypes, definitions)
    else {
        return Fit::Unknown;
    };
    let excluded = parameter.excluded_sources();
    if !excluded.is_empty() {
        let kinds = source_kinds(&source);
        if !kinds.is_empty() && kinds.iter().all(|kind| excluded.contains(kind)) {
            return Fit::Wrong;
        }
        if kinds.iter().any(|kind| excluded.contains(kind)) {
            return fit(&source, parameter.shape()).best(Fit::Wrong).min_maybe();
        }
    }
    if subtype_of_parameter(arm, parameter.declared(), definitions) {
        return Fit::Fits;
    }
    fit(&source, parameter.shape())
}

impl Fit {
    /// Some source values are excluded: at best uncertain.
    fn min_maybe(self) -> Self {
        match self {
            Self::Fits => Self::Maybe,
            other => other,
        }
    }
}

fn subtype_of_parameter(arm: &PropertyAtom, declared: &PropertyDef, definitions: &DefinitionTable) -> bool {
    match declared {
        PropertyDef::Single(target) => is_subtype_with_definitions(arm, target, definitions),
        PropertyDef::Union(targets) => {
            targets.iter().any(|target| is_subtype_with_definitions(arm, target, definitions))
        }
    }
}

/// The runtime value kinds a source shape can produce. `numberlike` and
/// `boolish` keep their value as sent, so they produce text as well as the
/// native scalar.
fn source_kinds(shape: &Shape) -> Vec<ValueKind> {
    match shape {
        Shape::Any => Vec::new(),
        Shape::Null => vec![ValueKind::Null],
        Shape::Number(number) if number.as_sent => vec![ValueKind::Number, ValueKind::Text],
        Shape::Number(_) => vec![ValueKind::Number],
        Shape::Boolean { as_sent: true } => vec![ValueKind::Boolean, ValueKind::Text],
        Shape::Boolean { .. } => vec![ValueKind::Boolean],
        Shape::Text(_) | Shape::Content(_) => vec![ValueKind::Text],
        Shape::Literal(value) => vec![ValueKind::of(value)],
        Shape::Array(_) | Shape::Tuple(_) => vec![ValueKind::List],
        Shape::Object(_) => vec![ValueKind::Object],
        Shape::Union(options) => {
            let mut kinds = Vec::new();
            for kind in options.iter().flat_map(source_kinds) {
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
            }
            kinds
        }
        Shape::Assert(inner, _) | Shape::Deferrable(inner) => source_kinds(inner),
        Shape::Ref(reference) => reference.target().map_or_else(Vec::new, |target| source_kinds(&target)),
    }
}

/// The scalar family of a source or target shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Number,
    Boolean,
    Text,
    Content(ContentFormat),
    List,
    Object,
    /// Retained `numberlike` text: admitted numeric text, kept as sent.
    /// Only ever a source family.
    NumberText,
    /// Retained `boolish` text: a boolean word, kept as sent. Only ever a
    /// source family.
    BooleanText,
}

fn literal_family(value: &Value) -> Option<Family> {
    match value {
        Value::Number(_) => Some(Family::Number),
        Value::Bool(_) => Some(Family::Boolean),
        Value::String(_) => Some(Family::Text),
        Value::Array(_) => Some(Family::List),
        Value::Object(_) => Some(Family::Object),
        Value::Null => None,
    }
}

/// Whether every value of `source` binds to `target`, some might, or none
/// can. Both are non-null source shapes; a target's constraints and
/// assertions are runtime checks and never decide a type-level verdict.
///
/// A source reference is judged by its target's body. That terminates:
/// translators close a cycle only through a value descent (an item, a
/// property, a tuple slot), and of those only an array item is compared, and
/// only against the target's own item shape, so every re-entry of a
/// reference is one level deeper into the finite `target`. A target-side
/// reference is never unfolded; it is judged by its family.
fn fit(source: &Shape, target: &Shape) -> Fit {
    match (source, target) {
        (Shape::Assert(inner, _) | Shape::Deferrable(inner), _) => fit(inner, target),
        (Shape::Ref(reference), _) => reference.target().map_or(Fit::Unknown, |body| fit(&body, target)),
        (Shape::Union(options), _) => Fit::all(options.iter().map(|option| fit(option, target))),
        // `numberlike` and `boolish` deliver either the native scalar or the
        // text as sent; every value fits only if both representations do.
        (Shape::Number(number), _) if number.as_sent => {
            let native = Shape::Number(NumberShape {
                as_sent: false,
                bounds: number.bounds.clone(),
            });
            Fit::all([fit(&native, target), retained_text_fit(Family::NumberText, target)])
        }
        (Shape::Boolean { as_sent: true }, _) => Fit::all([
            fit(&Shape::Boolean { as_sent: false }, target),
            retained_text_fit(Family::BooleanText, target),
        ]),
        (Shape::Any, _) => Fit::Unknown,
        (_, Shape::Any) => Fit::Fits,
        (_, Shape::Assert(inner, _) | Shape::Deferrable(inner)) => fit(source, inner),
        (_, Shape::Union(options)) => options.iter().map(|option| fit(source, option)).fold(Fit::Wrong, Fit::best),
        (Shape::Null, Shape::Null) => Fit::Fits,
        (Shape::Null, _) | (_, Shape::Null) => Fit::Wrong,
        (Shape::Literal(value), _) => {
            if bind(target, value).is_ok() {
                Fit::Fits
            } else {
                Fit::Wrong
            }
        }
        (_, Shape::Literal(value)) => match (family(source), literal_family(value)) {
            (Some(from), Some(to)) if from == to || family_fit(from, to, true) != Fit::Wrong => Fit::Maybe,
            _ => Fit::Wrong,
        },
        (Shape::Array(items), Shape::Array(target_items)) => match fit(&items.items, &target_items.items) {
            Fit::Fits => Fit::Fits,
            // An empty list binds whatever its element type.
            _ => Fit::Maybe,
        },
        (Shape::Array(_) | Shape::Tuple(_), Shape::Array(_) | Shape::Tuple(_)) => Fit::Maybe,
        (Shape::Object(_), Shape::Object(object)) => {
            if object.properties.is_empty()
                && object.patterns.is_empty()
                && matches!(object.additional, Additional::Any)
                && object.min_properties.is_none()
                && object.max_properties.is_none()
            {
                Fit::Fits
            } else {
                Fit::Maybe
            }
        }
        _ => match (family(source), family(target)) {
            (Some(from), Some(to)) => family_fit(from, to, scalar_text(target)),
            _ => Fit::Unknown,
        },
    }
}

/// Whether retained `numberlike` or `boolish` text (`text` is
/// [`Family::NumberText`] or [`Family::BooleanText`]) binds to `target`.
fn retained_text_fit(text: Family, target: &Shape) -> Fit {
    match target {
        Shape::Any => Fit::Fits,
        Shape::Null => Fit::Wrong,
        Shape::Assert(inner, _) | Shape::Deferrable(inner) => retained_text_fit(text, inner),
        Shape::Union(options) => options.iter().map(|option| retained_text_fit(text, option)).fold(Fit::Wrong, Fit::best),
        Shape::Literal(value) => match literal_family(value) {
            Some(to) if family_fit(text, to, true) != Fit::Wrong => Fit::Maybe,
            _ => Fit::Wrong,
        },
        _ => family(target).map_or(Fit::Unknown, |to| family_fit(text, to, scalar_text(target))),
    }
}

fn family(shape: &Shape) -> Option<Family> {
    match shape {
        Shape::Number(_) => Some(Family::Number),
        Shape::Boolean { .. } => Some(Family::Boolean),
        Shape::Text(_) => Some(Family::Text),
        Shape::Content(format) => Some(Family::Content(*format)),
        Shape::Array(_) | Shape::Tuple(_) => Some(Family::List),
        Shape::Object(_) => Some(Family::Object),
        Shape::Literal(value) => literal_family(value),
        Shape::Assert(inner, _) | Shape::Deferrable(inner) => family(inner),
        Shape::Ref(reference) => reference.target().and_then(|target| family(&target)),
        Shape::Any | Shape::Null | Shape::Union(_) => None,
    }
}

fn scalar_text(target: &Shape) -> bool {
    match target {
        Shape::Text(text) => text.scalar_text,
        Shape::Assert(inner, _) | Shape::Deferrable(inner) => scalar_text(inner),
        _ => true,
    }
}

/// The engine's direct conversions between families: number and boolean →
/// text always succeed (unless the text type refuses scalars); text →
/// number, boolean, or content may; a native value → `json`/`yaml` always
/// does.
///
/// Retained `numberlike`/`boolish` text is text, so it fits any text type,
/// and is narrower than arbitrary text: numeric text always converts to a
/// number and a boolean word to a boolean, while only `1` and `0` are both.
/// It is not always content: `+4` and `yes` are not JSON, and a tab-led
/// `\t4`, which admission trims, is not YAML.
fn family_fit(from: Family, to: Family, scalar_text: bool) -> Fit {
    use Family::{Boolean, BooleanText, Content, List, Number, NumberText, Object, Text};
    match (from, to) {
        (NumberText, Number) | (BooleanText, Boolean) | (NumberText | BooleanText, Text) => Fit::Fits,
        (NumberText, Boolean) | (BooleanText, Number) | (NumberText | BooleanText, Content(_)) => Fit::Maybe,
        (Number, Number) | (Boolean, Boolean) | (Text, Text) | (List, List) | (Object, Object) => Fit::Fits,
        (Content(_), Text) => Fit::Fits,
        (Content(left), Content(right)) if left == right => Fit::Fits,
        (Number | Boolean, Text) => {
            if scalar_text {
                Fit::Fits
            } else {
                Fit::Wrong
            }
        }
        (Number | Boolean | List | Object, Content(_)) => Fit::Fits,
        (Text | Content(_), Number | Boolean | Content(_)) => Fit::Maybe,
        _ => Fit::Wrong,
    }
}

/// The value of a literal argument: a scalar, `null`, a negated number, or
/// a list or object of literals.
fn literal_value(arg: &SpannedExpr) -> Option<Value> {
    match &arg.kind {
        SpannedExprKind::Paren(inner) => literal_value(inner),
        SpannedExprKind::StringLiteral(text) => Some(Value::String(text.clone())),
        SpannedExprKind::NumberLiteral(number) => serde_json::Number::from_f64(*number).map(Value::Number),
        SpannedExprKind::BoolLiteral(flag) => Some(Value::Bool(*flag)),
        SpannedExprKind::Variable(name) if name == super::binding::NULL_ROOT => Some(Value::Null),
        SpannedExprKind::UnaryMinus(inner) => match &inner.kind {
            SpannedExprKind::NumberLiteral(number) => serde_json::Number::from_f64(-*number).map(Value::Number),
            _ => None,
        },
        SpannedExprKind::ArrayLiteral(items) => items.iter().map(literal_value).collect::<Option<Vec<_>>>().map(Value::Array),
        SpannedExprKind::ObjectLiteral(entries) => entries
            .iter()
            .map(|(key, value)| literal_value(value).map(|value| (key.value.clone(), value)))
            .collect::<Option<serde_json::Map<_, _>>>()
            .map(Value::Object),
        _ => None,
    }
}

/// One argument's verdict in a [`CallCheck`].
#[derive(Debug, Clone, PartialEq)]
pub struct ArgumentCheck {
    /// The argument's position.
    pub index: usize,
    /// The parameter it binds to.
    pub parameter: String,
    /// The parameter's declared type as written.
    pub parameter_type: String,
    /// The argument's design-time type.
    pub argument_type: ArgumentType,
    pub category: ArgumentCategory,
    /// Whether a numeric recovery (`number(x, fallback)`) means this
    /// argument's own failure cannot reject the call.
    pub recovered: bool,
}

/// A problem that belongs to the whole call rather than one argument; always
/// category 5.
#[derive(Debug, Clone, PartialEq)]
pub enum CallProblem {
    /// A callback pattern cannot bind its projected input or uses a protected name.
    InvalidCallback { span: SourceSpan, message: String },
    /// No overload takes this many arguments.
    Arity {
        supplied: usize,
        /// Every overload, as `name(a: number, …): number`.
        signatures: Vec<String>,
    },
    /// More than one overload would bind the literal arguments.
    Ambiguous { signatures: Vec<String> },
    /// Several overloads take this many arguments and none can bind them.
    NoOverload { signatures: Vec<String> },
}

/// The design-time verdict on one call.
#[derive(Debug, Clone, PartialEq)]
pub struct CallCheck {
    /// The function name as written.
    pub function: String,
    /// The call's span (name through closing parenthesis).
    pub span: SourceSpan,
    /// One entry per supplied argument, against the overload the verdict is
    /// for; empty when [`Self::problem`] is set.
    pub arguments: Vec<ArgumentCheck>,
    /// The argument spans, in order, for ranging argument diagnostics.
    pub argument_spans: Vec<SourceSpan>,
    pub problem: Option<CallProblem>,
    /// The runtime binder's own error when every argument is a literal and
    /// the call does not bind: the type error composition would report.
    pub binding_error: Option<CallBindingError>,
    /// The argument whose authored literal text is not a well-formed file
    /// reference, bound to the parameter its function registers as
    /// [`false_for_malformed_reference`](super::call_binding::RegistrationBuilder::false_for_malformed_reference):
    /// the call is always `false`. Never set on a rejected call.
    pub always_false: Option<usize>,
}

/// The diagnostic code of a call that is always `false` because an authored
/// literal argument is not a well-formed file reference
/// (`file_exists("https://")`). Composition reports it as a warning, and the
/// editor shows it under the same code.
pub const ALWAYS_FALSE_CALL_CODE: &str = "dm.expression.always_false_call";

impl CallCheck {
    /// The call's category: 5 for a call problem, otherwise the most
    /// concerning argument category, recovered arguments excluded.
    pub fn category(&self) -> ArgumentCategory {
        if self.problem.is_some() {
            return ArgumentCategory::Incompatible;
        }
        self.arguments
            .iter()
            .filter(|argument| !argument.recovered)
            .map(|argument| argument.category)
            .fold(ArgumentCategory::Compatible, ArgumentCategory::worst)
    }

    /// Whether composition rejects the document because of this call.
    pub fn is_rejected(&self) -> bool {
        self.category() == ArgumentCategory::Incompatible
    }

    /// What composition reports when it rejects the document because of
    /// this call: the binder's own type error for a literal call, otherwise
    /// the first category 5 finding.
    pub fn rejection_message(&self) -> String {
        if let Some(error) = &self.binding_error {
            return error.to_string();
        }
        self.findings()
            .into_iter()
            .find(|finding| finding.category == ArgumentCategory::Incompatible)
            .map_or_else(|| format!("{}: the call cannot bind", self.function), |finding| finding.message)
    }

    /// The always-false advisory ([`Self::always_false`]), ranged at the
    /// malformed argument; `None` for any other call.
    pub fn always_false_finding(&self) -> Option<AlwaysFalseFinding> {
        let index = self.always_false?;
        Some(AlwaysFalseFinding {
            span: self.argument_spans.get(index).cloned().unwrap_or_else(|| self.span.clone()),
            argument: index,
            message: format!(
                "{}: argument {} is not a well-formed file reference, so this call is always false",
                self.function,
                index + 1
            ),
        })
    }

    /// The findings to show: the call problem ranged at the call, else each
    /// argument whose category is not 1 (and not recovered), ranged at the
    /// argument. Severity is the consumer's choice.
    pub fn findings(&self) -> Vec<CallFinding> {
        if let Some(problem) = &self.problem {
            return vec![CallFinding {
                span: match problem { CallProblem::InvalidCallback { span, .. } => span.clone(), _ => self.span.clone() },
                category: ArgumentCategory::Incompatible,
                argument: None,
                message: self.problem_message(problem),
            }];
        }
        self.arguments
            .iter()
            .filter(|argument| !argument.recovered && argument.category != ArgumentCategory::Compatible)
            .map(|argument| CallFinding {
                span: self.argument_spans.get(argument.index).cloned().unwrap_or_else(|| self.span.clone()),
                category: argument.category,
                argument: Some(argument.index),
                message: self.argument_message(argument),
            })
            .collect()
    }

    fn problem_message(&self, problem: &CallProblem) -> String {
        let function = &self.function;
        match problem {
            CallProblem::InvalidCallback { message, .. } => message.clone(),
            CallProblem::Arity { supplied, signatures } => format!(
                "{function}: no signature takes {supplied} argument{}; expected {}",
                if *supplied == 1 { "" } else { "s" },
                signatures.join(" or ")
            ),
            CallProblem::Ambiguous { signatures } => {
                format!("{function}: the arguments match more than one signature: {}", signatures.join(", "))
            }
            CallProblem::NoOverload { signatures } => {
                format!("{function}: no signature accepts these arguments; expected {}", signatures.join(" or "))
            }
        }
    }

    fn argument_message(&self, argument: &ArgumentCheck) -> String {
        let why = match argument.category {
            ArgumentCategory::Compatible => "it always binds",
            ArgumentCategory::NullableMatch => "it fits when present, but it may be null, which the parameter does not accept",
            ArgumentCategory::Uncertain => "some values of that type convert and others do not",
            ArgumentCategory::Unknown => "its type is not known until the document runs",
            ArgumentCategory::Incompatible => "no value of that type can bind",
        };
        format!(
            "{}: argument {} (`{}`) is {}, expected {} ({}: {why})",
            self.function,
            argument.index + 1,
            argument.parameter,
            argument.argument_type,
            argument.parameter_type,
            argument.category,
        )
    }
}

/// A call that can only return `false` ([`CallCheck::always_false_finding`]);
/// a warning under [`ALWAYS_FALSE_CALL_CODE`].
#[derive(Debug, Clone, PartialEq)]
pub struct AlwaysFalseFinding {
    /// The malformed argument's span.
    pub span: SourceSpan,
    pub argument: usize,
    /// Names the function and the argument.
    pub message: String,
}

/// One finding of a [`CallCheck`], ready to be shown with a severity.
#[derive(Debug, Clone, PartialEq)]
pub struct CallFinding {
    /// The argument's span, or the call's for a call problem.
    pub span: SourceSpan,
    pub category: ArgumentCategory,
    /// The argument's index; `None` when the finding ranges the call.
    pub argument: Option<usize>,
    /// Names the function, the parameter, both types, and why.
    pub message: String,
}

/// Whole-call classification of calling `name` with arguments of the given
/// design-time types: arity, overload selection, source exclusions, and
/// numeric recovery. `None` for a name nothing dispatches.
///
/// `span` and `argument_spans` locate the call's findings. A recursive
/// reference in an argument type resolves in nothing here; use
/// [`check_call_with_definitions`].
pub fn check_call(
    name: &str,
    arguments: &[ArgumentType],
    span: SourceSpan,
    argument_spans: Vec<SourceSpan>,
) -> Option<CallCheck> {
    check_call_with_definitions(name, arguments, span, argument_spans, &DefinitionTable::default())
}

/// [`check_call`] for argument types whose recursive references point into
/// `definitions`.
pub fn check_call_with_definitions(
    name: &str,
    arguments: &[ArgumentType],
    span: SourceSpan,
    argument_spans: Vec<SourceSpan>,
    definitions: &DefinitionTable,
) -> Option<CallCheck> {
    functions::inspect_registration(name, |registration| {
        let mut check = CallCheck {
            function: name.to_string(),
            span,
            arguments: Vec::new(),
            argument_spans,
            problem: None,
            binding_error: None,
            always_false: None,
        };
        classify_call(registration, arguments, definitions, &mut check);
        if !check.is_rejected() {
            check.always_false = registration.malformed_reference_parameter().filter(|index| {
                matches!(arguments.get(*index), Some(ArgumentType::Literal(Value::String(text)))
                    if !super::resolve_ctx::is_well_formed_reference(text))
            });
        }
        check
    })
}

fn classify_call(
    registration: &FunctionRegistration,
    arguments: &[ArgumentType],
    definitions: &DefinitionTable,
    check: &mut CallCheck,
) {
    let signatures = || registration.overloads().iter().map(ToString::to_string).collect::<Vec<_>>();
    let candidates: Vec<&CompiledSignature> =
        registration.overloads().iter().filter(|signature| signature.accepts_count(arguments.len())).collect();
    if candidates.is_empty() {
        check.problem = Some(CallProblem::Arity {
            supplied: arguments.len(),
            signatures: signatures(),
        });
        return;
    }

    let literals: Option<Vec<Value>> = arguments
        .iter()
        .map(|argument| match argument {
            ArgumentType::Literal(value) => Some(value.clone()),
            _ => None,
        })
        .collect();
    if let Some(values) = literals {
        classify_literal_call(registration, arguments, &values, &candidates, check);
        return;
    }

    let verdicts: Vec<Vec<ArgumentCheck>> =
        candidates.iter().map(|signature| classify_against(signature, arguments, definitions)).collect();
    let worst = |checks: &[ArgumentCheck]| {
        checks
            .iter()
            .filter(|argument| !argument.recovered)
            .map(|argument| argument.category)
            .fold(ArgumentCategory::Compatible, ArgumentCategory::worst)
    };
    let possible = verdicts.iter().filter(|checks| worst(checks) != ArgumentCategory::Incompatible).min_by_key(|checks| worst(checks).rank());
    match possible {
        Some(checks) => check.arguments = checks.clone(),
        None if verdicts.len() == 1 => check.arguments = verdicts.into_iter().next().unwrap_or_default(),
        None => match shared_failing_argument(&verdicts) {
            Some(index) => {
                // Every overload fails on this one argument: range it there.
                let mut checks = verdicts.into_iter().next().unwrap_or_default();
                for argument in &mut checks {
                    if argument.index != index {
                        argument.category = ArgumentCategory::Compatible;
                    }
                }
                check.arguments = checks;
            }
            None => {
                check.problem = Some(CallProblem::NoOverload {
                    signatures: candidates.iter().map(ToString::to_string).collect(),
                })
            }
        },
    }
}

/// The one argument index that every overload rejects, when there is one.
fn shared_failing_argument(verdicts: &[Vec<ArgumentCheck>]) -> Option<usize> {
    let failing = |checks: &Vec<ArgumentCheck>| -> Vec<usize> {
        checks
            .iter()
            .filter(|argument| !argument.recovered && argument.category == ArgumentCategory::Incompatible)
            .map(|argument| argument.index)
            .collect()
    };
    let first = failing(verdicts.first()?);
    match first.as_slice() {
        [index] if verdicts.iter().all(|checks| failing(checks) == [*index]) => Some(*index),
        _ => None,
    }
}

/// A call whose every argument is a literal binds exactly as it will at
/// runtime, recovery and ambiguity included, without entering a handler.
fn classify_literal_call(
    registration: &FunctionRegistration,
    arguments: &[ArgumentType],
    values: &[Value],
    candidates: &[&CompiledSignature],
    check: &mut CallCheck,
) {
    let template = candidates[0];
    let base = |category: ArgumentCategory, index: usize, recovered: bool| {
        let parameter = template.parameter_for(index);
        ArgumentCheck {
            index,
            parameter: parameter.map_or_else(String::new, |parameter| parameter.name().to_string()),
            parameter_type: parameter.map_or_else(String::new, |parameter| parameter.type_text().to_string()),
            argument_type: arguments[index].clone(),
            category,
            recovered,
        }
    };
    match registration.bind_call(values) {
        Ok(call) => {
            let recovered = call.args().recovery().map(|recovery| recovery.subject);
            check.arguments = (0..values.len())
                .map(|index| base(ArgumentCategory::Compatible, index, recovered == Some(index)))
                .collect();
        }
        Err(error) => {
            match &error {
                CallBindingError::AmbiguousOverload { signatures, .. } => {
                    check.problem = Some(CallProblem::Ambiguous {
                        signatures: signatures.clone(),
                    });
                }
                CallBindingError::Type { candidates: failures, .. } => {
                    let failing: Vec<Vec<usize>> = failures
                        .iter()
                        .filter_map(|failure| match &failure.problem {
                            SignatureProblem::Arguments(errors) => {
                                Some(errors.iter().map(|error| error.index).collect())
                            }
                            SignatureProblem::Arity { .. } => None,
                        })
                        .collect();
                    match failing.first() {
                        Some(first) if failing.iter().all(|indexes| indexes == first) => {
                            check.arguments = (0..values.len())
                                .map(|index| {
                                    let category = if first.contains(&index) {
                                        ArgumentCategory::Incompatible
                                    } else {
                                        ArgumentCategory::Compatible
                                    };
                                    base(category, index, false)
                                })
                                .collect();
                        }
                        _ => {
                            check.problem = Some(CallProblem::NoOverload {
                                signatures: candidates.iter().map(ToString::to_string).collect(),
                            });
                        }
                    }
                }
                CallBindingError::Arity { supplied, .. } => {
                    check.problem = Some(CallProblem::Arity {
                        supplied: *supplied,
                        signatures: registration.overloads().iter().map(ToString::to_string).collect(),
                    });
                }
                CallBindingError::ImplementationContract(_) => {}
            }
            check.binding_error = Some(error);
        }
    }
}

/// Every argument's category against one overload, with numeric recovery
/// applied: a subject that may fail is recovered as far as its fallback is
/// certain to bind.
fn classify_against(
    signature: &CompiledSignature,
    arguments: &[ArgumentType],
    definitions: &DefinitionTable,
) -> Vec<ArgumentCheck> {
    let mut checks: Vec<ArgumentCheck> = arguments
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            let parameter = signature.parameter_for(index).expect("the overload accepts this many arguments");
            ArgumentCheck {
                index,
                parameter: parameter.name().to_string(),
                parameter_type: parameter.type_text().to_string(),
                argument_type: argument.clone(),
                category: classify_argument_with_definitions(argument, parameter, definitions),
                recovered: false,
            }
        })
        .collect();
    if let Some((subject, fallback)) = signature.recovery()
        && let Some(fallback_category) = checks.get(fallback).map(|check| check.category)
        && fallback_category != ArgumentCategory::Incompatible
        && let Some(subject_check) = checks.get_mut(subject)
        && subject_check.category != ArgumentCategory::Compatible
    {
        if fallback_category == ArgumentCategory::Compatible {
            subject_check.recovered = true;
        } else if fallback_category.rank() < subject_check.category.rank() {
            // Recovered exactly when the fallback binds.
            subject_check.category = fallback_category;
        }
    }
    checks
}

/// Checks every function call in `expr`, in every branch, including the
/// branches a run would never take and the operands a lazy function would
/// skip. Each argument's type is read at its own site: `site` refined by
/// the guards that enclose it ([`branch_scope`]).
///
/// `site` must be built over the declaration-only layer
/// ([`DeclaredTypes::for_schema`](super::DeclaredTypes::for_schema)): static
/// frontmatter values are runtime values, not design-time types. Recursive-typed arguments resolve in the layer's
/// [`DeclaredTypes::definitions`](super::DeclaredTypes::definitions). Unknown functions are skipped
/// ([`validate_expression`](super::validate_expression) reports them).
pub fn check_expression_calls(expr: &SpannedExpr, site: &TypeScope<'_>) -> Vec<CallCheck> {
    let mut checks = Vec::new();
    visit_call_nodes(expr, &mut |call| {
        let local_site = branch_scope(expr, &call.span, site.clone());
        let (name, types, spans) = match &call.kind {
            SpannedExprKind::FunctionCall { name, args } => (
                name,
                args.iter().map(|arg| ArgumentType::of(arg, &branch_scope(call, &arg.span, local_site.clone()))).collect::<Vec<_>>(),
                args.iter().map(|arg| arg.span.clone()).collect(),
            ),
            SpannedExprKind::CallbackCall { name, args } => (
                name,
                args.iter().map(|arg| match arg {
                    SpannedCallArgument::Value(value) => ArgumentType::of(value, &local_site),
                    SpannedCallArgument::Callback(_) => ArgumentType::Callback,
                }).collect(),
                args.iter().map(|arg| arg.span().clone()).collect(),
            ),
            _ => return,
        };
        if let Some(mut check) = check_call_with_definitions(name, &types, call.span.clone(), spans, local_site.definitions()) {
            if let SpannedExprKind::CallbackCall { args, .. } = &call.kind {
                for (index, arg) in args.iter().enumerate() {
                    let SpannedCallArgument::Callback(callback) = arg else { continue; };
                    let accepts_callback = functions::inspect_registration(name, |registration| {
                        registration.overloads().iter().filter(|signature| signature.accepts_count(args.len()))
                            .any(|signature| signature.parameter_for(index)
                                .is_some_and(|parameter| parameter.callback().is_some()))
                    }).unwrap_or(false);
                    if check.problem.is_none() && !accepts_callback {
                        check.problem = Some(CallProblem::InvalidCallback {
                            span: callback.span(), message: format!("{name}: this argument position does not accept an inline callback"),
                        });
                        break;
                    }
                    let error = super::check_local_names(callback.pattern.erase().names(), local_site.declared().binding_view())
                        .err().map(|error| error.to_string())
                        .or_else(|| callback_scope(callback, args, &local_site).err().map(|error| error.to_string()));
                    if let Some(message) = error {
                        check.problem = Some(CallProblem::InvalidCallback { span: callback.pattern_span.clone(), message });
                        break;
                    }
                }
            }
            checks.push(check);
        }
    });
    checks
}

/// Whether the passive check proves that calling `name` with `args`, at the
/// site `scope` describes, cannot bind: the call is category 5 (an arity
/// problem, an overload no argument set fits, or an argument no value of its
/// type can bind). An argument whose type is unknown, uncertain, or maybe
/// null does not prove a rejection. `false` for a name nothing dispatches.
///
/// [`TypeScope::assume`] reads this so a rejected call grants no fact: the
/// spec's "facts apply only after a call is accepted by the normal call
/// checker". Each argument is typed in `scope` itself, which already holds
/// the facts of the guards enclosing the call, as
/// [`check_expression_calls`]'s `branch_scope` would give it.
pub(crate) fn call_is_rejected(name: &str, args: &[&Expr], scope: &TypeScope<'_>) -> bool {
    let types: Vec<ArgumentType> = args.iter().map(|arg| ArgumentType::of(&unlocated(arg), scope)).collect();
    check_call_with_definitions(name, &types, 0..0, Vec::new(), scope.declared().definitions())
        .is_some_and(|check| check.is_rejected())
}

/// `expr` as a [`SpannedExpr`] whose spans are all empty: the typing rules
/// read only the node kinds, never a span.
fn unlocated(expr: &Expr) -> SpannedExpr {
    let boxed = |inner: &Expr| Box::new(unlocated(inner));
    let kind = match expr {
        Expr::Variable(name) => SpannedExprKind::Variable(name.clone()),
        Expr::StringLiteral(text) => SpannedExprKind::StringLiteral(text.clone()),
        Expr::NumberLiteral(number) => SpannedExprKind::NumberLiteral(*number),
        Expr::BoolLiteral(flag) => SpannedExprKind::BoolLiteral(*flag),
        Expr::ArrayLiteral(items) => SpannedExprKind::ArrayLiteral(items.iter().map(unlocated).collect()),
        Expr::ObjectLiteral(entries) => SpannedExprKind::ObjectLiteral(
            entries
                .iter()
                .map(|(key, value)| (crate::markdown::span::Spanned::new(key.clone(), 0..0), unlocated(value)))
                .collect(),
        ),
        Expr::UnaryNot(inner) => SpannedExprKind::UnaryNot(boxed(inner)),
        Expr::UnaryMinus(inner) => SpannedExprKind::UnaryMinus(boxed(inner)),
        Expr::Paren(inner) => SpannedExprKind::Paren(boxed(inner)),
        Expr::Binary { op, left, right } => SpannedExprKind::Binary {
            op: *op,
            left: boxed(left),
            right: boxed(right),
        },
        Expr::Index { base, index } => SpannedExprKind::Index {
            base: boxed(base),
            index: boxed(index),
        },
        Expr::MemberAccess { base, name } => SpannedExprKind::MemberAccess {
            base: boxed(base),
            name: name.clone(),
        },
        Expr::Fallback { primary, fallback } => SpannedExprKind::Fallback {
            primary: boxed(primary),
            fallback: boxed(fallback),
        },
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => SpannedExprKind::Ternary {
            condition: boxed(condition),
            then_branch: boxed(then_branch),
            else_branch: boxed(else_branch),
        },
        Expr::Comparison { left, op, right } => SpannedExprKind::Comparison {
            left: boxed(left),
            op: *op,
            right: boxed(right),
        },
        Expr::FunctionCall { name, args } => SpannedExprKind::FunctionCall {
            name: name.clone(),
            args: args.iter().map(unlocated).collect(),
        },
        Expr::CallbackCall { name, args } => SpannedExprKind::CallbackCall {
            name: name.clone(),
            args: args.iter().map(|argument| match argument {
                super::ast::CallArgument::Value(value) => SpannedCallArgument::Value(unlocated(value)),
                super::ast::CallArgument::Callback(callback) => {
                    let binding = |name: &String| crate::markdown::span::Spanned::new(name.clone(), 0..0);
                    let pattern = match &callback.pattern {
                        super::ast::BindingPattern::Identifier(name) => {
                            super::ast::SpannedBindingPattern::Identifier(binding(name))
                        }
                        super::ast::BindingPattern::Fields(fields) => {
                            super::ast::SpannedBindingPattern::Fields(fields.iter().map(binding).collect())
                        }
                    };
                    SpannedCallArgument::Callback(SpannedCallback {
                        pattern,
                        pattern_span: 0..0,
                        arrow: 0..0,
                        body: unlocated(&callback.body),
                    })
                }
            }).collect(),
        },
    };
    SpannedExpr::new(kind, 0..0)
}

/// A [`CallCheck`] in a prepared value, located in its string leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCallCheck {
    /// JSON pointer of the string leaf holding the expression.
    pub pointer: String,
    /// The expression text.
    pub expression: String,
    /// The check, with spans relative to the string leaf.
    pub check: CallCheck,
}

/// [`check_expression_calls`] for every expression of `prepared`, with spans
/// mapped onto their string leaves. Only checks whose category is not 1, or
/// that are always false, are returned.
pub fn check_prepared_calls(prepared: &PreparedValue, site: &TypeScope<'_>) -> Vec<PreparedCallCheck> {
    let mut found = Vec::new();
    for expression in prepared.expressions() {
        for mut check in check_expression_calls(expression.expr(), site) {
            if check.category() == ArgumentCategory::Compatible && check.always_false.is_none() {
                continue;
            }
            check.span = expression.leaf_span(&check.span);
            check.argument_spans = check.argument_spans.iter().map(|span| expression.leaf_span(span)).collect();
            if let Some(CallProblem::InvalidCallback { span, .. }) = &mut check.problem {
                *span = expression.leaf_span(span);
            }
            found.push(PreparedCallCheck {
                pointer: expression.pointer().to_string(),
                expression: expression.text().to_string(),
                check,
            });
        }
    }
    found
}

/// Calls `visit` for every function-call node in `expr`, outermost first.
pub(crate) fn visit_call_nodes<'e>(expr: &'e SpannedExpr, visit: &mut impl FnMut(&'e SpannedExpr)) {
    match &expr.kind {
        SpannedExprKind::Variable(_)
        | SpannedExprKind::StringLiteral(_)
        | SpannedExprKind::NumberLiteral(_)
        | SpannedExprKind::BoolLiteral(_) => {}
        SpannedExprKind::ArrayLiteral(items) => items.iter().for_each(|item| visit_call_nodes(item, visit)),
        SpannedExprKind::ObjectLiteral(entries) => entries.iter().for_each(|(_, value)| visit_call_nodes(value, visit)),
        SpannedExprKind::Paren(inner)
        | SpannedExprKind::UnaryNot(inner)
        | SpannedExprKind::UnaryMinus(inner)
        | SpannedExprKind::MemberAccess { base: inner, .. } => visit_call_nodes(inner, visit),
        SpannedExprKind::Binary { left, right, .. } | SpannedExprKind::Comparison { left, right, .. } => {
            visit_call_nodes(left, visit);
            visit_call_nodes(right, visit);
        }
        SpannedExprKind::Index { base, index } => {
            visit_call_nodes(base, visit);
            visit_call_nodes(index, visit);
        }
        SpannedExprKind::Fallback { primary, fallback } => {
            visit_call_nodes(primary, visit);
            visit_call_nodes(fallback, visit);
        }
        SpannedExprKind::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_call_nodes(condition, visit);
            visit_call_nodes(then_branch, visit);
            visit_call_nodes(else_branch, visit);
        }
        SpannedExprKind::CallbackCall { args, .. } => {
            visit(expr);
            for arg in args {
                visit_call_nodes(arg.expression(), visit);
            }
        }
        SpannedExprKind::FunctionCall { args, .. } => {
            visit(expr);
            args.iter().for_each(|arg| visit_call_nodes(arg, visit));
        }
    }
}

/// One invocation has one fresh item identity, independent of identically spelled captures.
pub(crate) fn callback_item(args: &[SpannedCallArgument], scope: &TypeScope<'_>) -> super::scope::ItemFrame {
    let shape = args.first().and_then(|arg| match arg {
        SpannedCallArgument::Value(value) => Some(expression_type(value, scope)),
        _ => None,
    }).unwrap_or(ScopeShape::Unknown { non_null: false });
    let entry = match super::scope::project_source(&shape, scope.definitions()) {
        super::scope::SourceProjection::Entries(entry) => entry,
        _ => ScopeShape::Unknown { non_null: false },
    };
    super::scope::ItemFrame::new(entry)
}

pub(crate) fn callback_scope<'d>(callback: &SpannedCallback, args: &[SpannedCallArgument], scope: &TypeScope<'d>) -> Result<TypeScope<'d>, super::scope::PatternMismatch> {
    callback_item(args, scope).enter_spanned(scope, scope.lexical(), &callback.pattern)
}

#[cfg(test)]
mod callback_tests {
    use super::*;
    use crate::markdown::compose::expression::{parse_spanned, static_access_reads, static_variable_reads};
    use crate::markdown::schemas::parse_yaml_schema;

    #[test]
    fn narrowing_predicates_accept_callback_results_in_guards() {
        let declared = super::super::DeclaredTypes::new(None);
        let scope = TypeScope::unguarded(&declared);
        for source in [
            "is_array(map([1], x => x)) ? min(1, 2) : 0",
            "is_array(map([{value: 1}], {value} -> value)) ? min(1, 2) : 0",
            "is_array(map([[1]], xs => filter(xs, n -> n > 0))) ? min(1, 2) : 0",
        ] {
            let checks = check_expression_calls(&parse_spanned(source).unwrap(), &scope);
            assert!(checks.iter().all(|check| !check.is_rejected()), "{source}: {checks:?}");
        }
    }

    #[test]
    fn callback_calls_check_dead_bodies_and_shadow_document_types() {
        let schema = parse_yaml_schema(&serde_yaml_ng::from_str("tags: boolean(required)").unwrap()).unwrap();
        let declared = super::super::DeclaredTypes::new(Some(&schema));
        let scope = TypeScope::unguarded(&declared);
        for source in ["map([], x => min(true, 5))", "map(null, x -> min(true, 5))"] {
            let expression = parse_spanned(source).unwrap();
            let checks = check_expression_calls(&expression, &scope);
            assert!(checks.iter().any(|check| check.function == "min" && check.is_rejected()), "{source}: {checks:?}");
        }
        for source in ["map([1, 2], tags => min(tags, 5))", "map([1], tags -> min(local.tags, 5))"] {
            let checks = check_expression_calls(&parse_spanned(source).unwrap(), &scope);
            assert!(checks.iter().all(|check| !check.is_rejected()), "{source}: {checks:?}");
        }
    }

    #[test]
    fn callback_signatures_and_patterns_reject_invalid_calls() {
        let declared = super::super::DeclaredTypes::new(None);
        let scope = TypeScope::unguarded(&declared);
        for source in ["map([1], true)", "map([1])", "map([1], x => x, 3)",
            "map(42, x => x)", "min(1, x => x)", "map([1], {key} => key)", "map(([1]), {key} => key)", "map([null], {key} => key)",
            "map([{a: 1}], {missing} => missing)"] {
            let checks = check_expression_calls(&parse_spanned(source).unwrap(), &scope);
            assert!(checks.iter().any(CallCheck::is_rejected), "{source}: {checks:?}");
        }
        for source in ["map(null, {key} => key)", "map([], {key} => key)",
            "map({a: 1}, {key,value} => min(value, 2))", "map([{key: null}], {key} => key)", "map([1], x => map([2], x -> min(x, 3)))"] {
            let checks = check_expression_calls(&parse_spanned(source).unwrap(), &scope);
            assert!(checks.iter().all(|check| !check.is_rejected()), "{source}: {checks:?}");
        }
    }

    #[test]
    fn callback_read_inventory_keeps_only_free_captures() {
        let expression = parse_spanned("map(missing, x => map(x.children, {value} -> local.value || x.name || fm.x || absent))").unwrap();
        let reads = static_variable_reads(&expression);
        assert_eq!(reads.iter().map(|read| read.path).collect::<Vec<_>>(), ["missing", "fm.x", "absent"]);
        assert!(reads[0].handles_absence);
        assert!(!reads[2].handles_absence);
        let erased = expression.erase();
        let access = static_access_reads(&erased);
        assert_eq!(access.iter().map(|read| read.static_path()).collect::<Vec<_>>(), ["missing", "fm.x", "absent"]);
        let guarded = parse_spanned("present ? map([], p => present.name) : []").unwrap();
        let guarded_reads = static_variable_reads(&guarded);
        assert_eq!(guarded_reads[1].path, "present.name");
        assert!(guarded_reads[1].handles_absence, "the callback keeps its enclosing capture's guard");
        for source in ["map([null], p => p ? fm.p : null)", "map([null], p => local.p ? fm.p : null)"] {
            let expression = parse_spanned(source).unwrap();
            let reads = static_variable_reads(&expression);
            assert!(!reads.iter().find(|read| read.path == "fm.p").unwrap().handles_absence, "{source}");
        }
    }

    #[test]
    fn callback_result_projection_preserves_members_nulls_and_filter_items() {
        let declared = super::super::DeclaredTypes::new(None);
        let scope = TypeScope::unguarded(&declared);
        for (source, expected) in [
            ("map([{name: 'Ada'}], p => p.name)", "literal(Ada)"),
            ("map({Ada: 90}, {value} => value)", "literal(90"),
            ("map(({Ada: 90}), {value} => value)", "literal(90"),
            ("filter([{name: 'Ada'}], {name} => name)", "name"),
            ("map([null], p => p)", "null"),
            ("filter(map(['./a.md'], x => x), f => is_file_type(f))", "file"),
            ("filter(map(['./a.md'], x => x), f => file_exists(f))", "file"),
        ] {
            let shape = expression_type(&parse_spanned(source).unwrap(), &scope);
            let text = ArgumentType::Typed(shape).to_string();
            assert!(text.contains(expected), "{source}: {text}");
            assert!(text.contains("[]"), "{source}: {text}");
        }
    }

    #[test]
    fn callback_patterns_protect_registered_loop_and_seq_globals() {
        use super::super::{Availability, BindingView, ScopeId, UnavailabilityReason};
        let view = BindingView::builder(ScopeId::new("callback"))
            .declare("seq", "sequence", Availability::Available)
            .declare("loop", "loop", Availability::Unavailable(UnavailabilityReason::new("host.loop")))
            .build().unwrap();
        let declared = super::super::DeclaredTypes::new(None).with_binding_view(&view);
        let scope = TypeScope::unguarded(&declared);
        for name in ["seq", "loop"] {
            let source = format!("map([], {name} => {name})");
            let checks = check_expression_calls(&parse_spanned(&source).unwrap(), &scope);
            assert_eq!(checks[0].rejection_diagnostic().unwrap().code, super::super::parser::INVALID_CALLBACK_CODE);
        }
    }

    #[test]
    fn callback_type_frames_retain_binding_spans_after_the_ast_is_dropped() {
        let declared = super::super::DeclaredTypes::new(None);
        let scope = TypeScope::unguarded(&declared);
        let retained = {
            let expression = parse_spanned("map([], {key,value} => key)").unwrap();
            let SpannedExprKind::CallbackCall { args, .. } = &expression.kind else { panic!("callback call"); };
            let SpannedCallArgument::Callback(callback) = &args[1] else { panic!("callback argument"); };
            callback_scope(callback, args, &scope).unwrap()
        };
        let frame = retained.lexical().unwrap();
        assert_eq!(frame.span_of_name("key"), Some(&(9..12)));
        assert_eq!(frame.span_of_name("value"), Some(&(13..18)));
        let shadow = frame.child(vec![super::super::scope::TypedLocal::new("key", ScopeShape::Unknown { non_null: false })]);
        assert_eq!(shadow.span_of_name("key"), None);
        assert_eq!(shadow.span_of_name("value"), Some(&(13..18)));
    }

    #[test]
    fn callback_declared_arrays_tuples_and_objects_keep_known_types() {
        let schema = parse_yaml_schema(&serde_yaml_ng::from_str(
            "people: '{name: string(required), active: boolean(required)}[](required)'\nt: tuple([string, number])\nscores:\n  Ada: number(required)\n  Grace: number(required)\n"
        ).unwrap()).unwrap();
        let declared = super::super::DeclaredTypes::new(Some(&schema));
        let scope = TypeScope::unguarded(&declared);
        for (source, parts) in [
            ("map(people, p => p.name)", vec!["string"]),
            ("filter(people, p => p.active)", vec!["name", "active"]),
            ("map(t, x => x)", vec!["string", "number"]),
            ("map(scores, {key,value} => value)", vec!["number"]),
        ] {
            let expr = parse_spanned(source).unwrap();
            let text = ArgumentType::Typed(expression_type(&expr, &scope)).to_string();
            for part in parts { assert!(text.contains(part), "{source}: {text}"); }
            assert!(!text.contains("unknown"), "{source}: {text}");
            assert!(check_expression_calls(&expr, &scope).iter().all(|check| !check.is_rejected()));
        }
    }

    #[test]
    fn callback_nested_scope_restores_outer_binding_and_separates_frontmatter_facts() {
        let schema = parse_yaml_schema(&serde_yaml_ng::from_str("x: string(required)").unwrap()).unwrap();
        let declared = super::super::DeclaredTypes::new(Some(&schema));
        let scope = TypeScope::unguarded(&declared);
        let source = "is_string(x) ? map([true], x => map([1], x => min(x, 5)) || min(x, 5)) : []";
        let checks = check_expression_calls(&parse_spanned(source).unwrap(), &scope);
        let min: Vec<_> = checks.iter().filter(|check| check.function == "min").collect();
        assert_eq!(min.len(), 2);
        assert!(!min[0].is_rejected());
        assert!(min[1].is_rejected());
        assert_eq!(scope.type_of_expr(&super::super::parse("x").unwrap()).unwrap().shape(),
            &expression_type(&parse_spanned("x").unwrap(), &scope));
    }
}
