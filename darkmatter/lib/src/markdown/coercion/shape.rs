//! The neutral description of a target type that the rule core binds against.
//!
//! A [`Shape`] carries structure and constraints only. It holds no rule about
//! keeping, converting, or rejecting a value: translators build shapes from
//! SimplifiedSchema descriptors or raw JSON Schema, and the engine alone
//! decides what happens to a value.

use std::fmt;
use std::ops::Deref;
use std::sync::{Arc, OnceLock, Weak};

use serde_json::Value;

use crate::markdown::numeric::format_number;

/// A target type.
#[derive(Debug, Clone)]
pub enum Shape {
    /// No declared structure: any value, including `null` and any nested
    /// contents, is kept unchanged.
    Any,
    /// Only `null`.
    Null,
    Number(NumberShape),
    Boolean {
        /// `boolish`: accepts what `boolean` accepts and keeps it as sent.
        as_sent: bool,
    },
    Text(TextShape),
    /// A single allowed value (`const`, `literal(x)`).
    Literal(Value),
    /// Text holding `json` or `yaml` content.
    Content(ContentFormat),
    Array(ArrayShape),
    Tuple(TupleShape),
    Object(ObjectShape),
    /// Order-independent: the written order of the options never changes a
    /// result.
    Union(Vec<Shape>),
    /// The inner shape plus a passive check on the final value. An assertion
    /// never converts; it only rejects.
    Assert(Box<Shape>, Assertion),
    /// A use of a reusable target shape: how a recursive type refers to
    /// itself, so a value converts at any depth the engine walks.
    Ref(ShapeRef),
    /// The inner shape at a place the schema marks `deferred`: a value here
    /// may be supplied later (a `{{ … }}` template). The engine treats a
    /// value here as an exact match when [`BindOptions::defer_marked`]
    /// accepts it, and otherwise binds it to the inner shape; without that
    /// predicate the wrapper changes nothing. It is described as its inner
    /// shape.
    ///
    /// [`BindOptions::defer_marked`]: super::BindOptions::defer_marked
    Deferrable(Box<Shape>),
}

/// A use of a reusable target shape, built through a [`RefSlot`].
///
/// A recursive type is a cycle, which a tree of owned shapes cannot hold.
/// The use that first reaches a target owns it; every use inside the
/// target's own body refers back to it weakly, so dropping the shape that
/// holds the owning use frees the whole cycle. A back reference therefore
/// resolves only while an owning use is alive: a fragment cloned out of a
/// recursive body and kept after its root is dropped matches no value.
///
/// Translators only close a cycle through a value descent (a property, an
/// item, a tuple slot, a dictionary value), so walking a reference always
/// reaches a deeper value before it can reach the same reference again; the
/// engine still bounds consecutive reference hops as a backstop.
#[derive(Clone)]
pub struct ShapeRef {
    name: Arc<str>,
    link: Link,
}

#[derive(Clone)]
enum Link {
    Owner(Arc<OnceLock<Shape>>),
    Back(Weak<OnceLock<Shape>>),
}

impl ShapeRef {
    /// The target's name, used as its expected-type text: a recursive
    /// target is described by name, never expanded.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The target shape; `None` when no owning use is alive.
    pub fn target(&self) -> Option<RefTarget> {
        let cell = match &self.link {
            Link::Owner(cell) => cell.clone(),
            Link::Back(cell) => cell.upgrade()?,
        };
        cell.get().is_some().then_some(RefTarget(cell))
    }

    /// Whether both uses name the same target.
    pub fn same_target(&self, other: &Self) -> bool {
        std::ptr::eq(self.cell_ptr(), other.cell_ptr())
    }

    /// Whether this use owns its target rather than referring back to it.
    pub fn is_owner(&self) -> bool {
        matches!(self.link, Link::Owner(_))
    }

    fn cell_ptr(&self) -> *const OnceLock<Shape> {
        match &self.link {
            Link::Owner(cell) => Arc::as_ptr(cell),
            Link::Back(cell) => cell.as_ptr(),
        }
    }
}

impl fmt::Debug for ShapeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The target may contain this reference; printing it would not end.
        f.debug_struct("ShapeRef")
            .field("name", &self.name)
            .field("owner", &self.is_owner())
            .finish_non_exhaustive()
    }
}

/// A resolved [`ShapeRef`] target, kept alive while it is borrowed.
pub struct RefTarget(Arc<OnceLock<Shape>>);

impl Deref for RefTarget {
    type Target = Shape;

    fn deref(&self) -> &Shape {
        // `ShapeRef::target` returns a target only once its cell is set, and
        // a `OnceLock` is never unset.
        self.0.get().expect("a resolved reference target is set")
    }
}

/// A reusable target under construction.
///
/// Create the slot before translating the target's body, hand out
/// [`Self::back_reference`]s for uses inside that body, then
/// [`Self::finish`] it with the body to get the owning use.
pub struct RefSlot {
    name: Arc<str>,
    cell: Arc<OnceLock<Shape>>,
}

impl RefSlot {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            cell: Arc::new(OnceLock::new()),
        }
    }

    /// A weak use of the target, for a use inside the target's own body.
    pub fn back_reference(&self) -> ShapeRef {
        ShapeRef {
            name: self.name.clone(),
            link: Link::Back(Arc::downgrade(&self.cell)),
        }
    }

    /// Whether any back reference was handed out, i.e. the body recurses.
    pub fn is_referenced(&self) -> bool {
        Arc::weak_count(&self.cell) > 0
    }

    /// Sets the target's body and returns the owning use.
    pub fn finish(self, body: Shape) -> ShapeRef {
        // A fresh slot's cell is set exactly once, here.
        let _ = self.cell.set(body);
        ShapeRef {
            name: self.name,
            link: Link::Owner(self.cell),
        }
    }
}

/// A `number`, `integer`, or `numberlike` target.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NumberShape {
    /// `numberlike`: accepts what `number` accepts and keeps it as sent.
    pub as_sent: bool,
    pub bounds: NumericBounds,
}

/// Numeric constraints, checked on the admitted `f64`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NumericBounds {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_minimum: Option<f64>,
    pub exclusive_maximum: Option<f64>,
    pub multiple_of: Option<f64>,
    pub integer: Option<IntegerRule>,
}

/// Which whole-number rule a target carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerRule {
    /// Raw JSON Schema `integer`: no fractional part.
    Whole,
    /// SimplifiedSchema `number(integer)`: no fractional part and a magnitude
    /// below 2^64.
    WholeBelow2Pow64,
}

/// A text target (`string`, enums, refinements built with [`Shape::Assert`]).
#[derive(Debug, Clone)]
pub struct TextShape {
    /// Minimum length in Unicode scalar values.
    pub min_length: Option<usize>,
    /// Maximum length in Unicode scalar values.
    pub max_length: Option<usize>,
    pub pattern: Option<TextPattern>,
    /// The allowed values (`enum(...)`); `None` allows any text.
    pub members: Option<Vec<String>>,
    /// Whether a native number or boolean converts into this target. `false`
    /// only for a type whose own rule refuses them (`alphanumeric`).
    pub scalar_text: bool,
}

impl Default for TextShape {
    fn default() -> Self {
        Self {
            min_length: None,
            max_length: None,
            pattern: None,
            members: None,
            scalar_text: true,
        }
    }
}

/// A compiled regular expression with its source, searched (not anchored)
/// like JSON Schema's `pattern`.
///
/// Compiled with `fancy_regex`, the engine `jsonschema` uses, so the engine
/// and the validator agree on what a pattern matches.
#[derive(Debug, Clone)]
pub struct TextPattern {
    source: String,
    regex: fancy_regex::Regex,
}

impl TextPattern {
    /// ## Errors
    ///
    /// The `fancy_regex` compile error for an invalid pattern.
    pub fn new(source: impl Into<String>) -> Result<Self, Box<fancy_regex::Error>> {
        let source = source.into();
        let regex = fancy_regex::Regex::new(&source).map_err(Box::new)?;
        Ok(Self { source, regex })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// A pattern whose search exceeds the backtracking limit does not match.
    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text).unwrap_or(false)
    }
}

/// The content format of a `json` or `yaml` target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentFormat {
    Json,
    Yaml,
}

/// A list whose every element has the same shape.
#[derive(Debug, Clone)]
pub struct ArrayShape {
    pub items: Box<Shape>,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub unique_items: bool,
}

impl ArrayShape {
    pub fn of(items: Shape) -> Self {
        Self {
            items: Box::new(items),
            min_items: None,
            max_items: None,
            unique_items: false,
        }
    }
}

/// A list with positional element shapes.
#[derive(Debug, Clone)]
pub struct TupleShape {
    /// Required slots first, then optional ones.
    pub prefix: Vec<TupleSlot>,
    /// The shape of every element after the prefix; `None` forbids extra
    /// elements.
    pub rest: Option<Box<Shape>>,
}

#[derive(Debug, Clone)]
pub struct TupleSlot {
    pub shape: Shape,
    /// A missing optional slot stays missing; it is never filled with `null`.
    pub optional: bool,
}

/// A structured object.
#[derive(Debug, Clone)]
pub struct ObjectShape {
    pub properties: Vec<PropertySlot>,
    pub patterns: Vec<PatternSlot>,
    pub pattern_scope: PatternScope,
    /// Applies only to keys that no property and no applicable pattern
    /// covers.
    pub additional: Additional,
    pub min_properties: Option<usize>,
    pub max_properties: Option<usize>,
}

impl ObjectShape {
    /// An object with no declared structure: any object, contents kept.
    pub fn opaque() -> Self {
        Self {
            properties: Vec::new(),
            patterns: Vec::new(),
            pattern_scope: PatternScope::AllKeys,
            additional: Additional::Any,
            min_properties: None,
            max_properties: None,
        }
    }

    /// Whether the object declares no structure, so its contents are kept
    /// as they are.
    pub fn is_opaque(&self) -> bool {
        self.properties.is_empty() && self.patterns.is_empty() && matches!(self.additional, Additional::Any)
    }
}

#[derive(Debug, Clone)]
pub struct PropertySlot {
    pub name: String,
    pub shape: Shape,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct PatternSlot {
    pub matcher: KeyMatcher,
    pub shape: Shape,
}

/// Which keys a [`PatternSlot`] applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternScope {
    /// JSON Schema: every matching key, including a declared property; a key
    /// must satisfy every declaration that applies to it.
    AllKeys,
    /// SimplifiedSchema dictionaries: only keys that are not declared
    /// properties.
    UndeclaredKeys,
}

/// What a key matches against in a [`PatternSlot`].
#[derive(Debug, Clone)]
pub enum KeyMatcher {
    /// Every key (`<string>`).
    Any,
    /// Keys starting with the text (`<starting::P>`).
    Prefix(String),
    /// Keys ending with the text (`<ending::S>`).
    Suffix(String),
    /// Keys the pattern finds a match in (`<pattern::RE>`, `patternProperties`).
    Pattern(TextPattern),
}

impl KeyMatcher {
    pub fn matches(&self, key: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Prefix(prefix) => key.starts_with(prefix.as_str()),
            Self::Suffix(suffix) => key.ends_with(suffix.as_str()),
            Self::Pattern(pattern) => pattern.is_match(key),
        }
    }
}

/// The rule for keys covered by no property and no pattern.
#[derive(Debug, Clone)]
pub enum Additional {
    /// Kept as they are.
    Any,
    /// An error at the key's path.
    Forbidden,
    Shape(Box<Shape>),
}

/// A passive predicate on a final value, with the description used in the
/// expected-type text and the failure reason.
#[derive(Clone)]
pub struct Assertion {
    description: String,
    predicate: Arc<dyn Fn(&Value) -> bool + Send + Sync>,
}

impl Assertion {
    pub fn new(description: impl Into<String>, predicate: impl Fn(&Value) -> bool + Send + Sync + 'static) -> Self {
        Self {
            description: description.into(),
            predicate: Arc::new(predicate),
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn holds(&self, value: &Value) -> bool {
        (self.predicate)(value)
    }
}

impl fmt::Debug for Assertion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Assertion").field("description", &self.description).finish_non_exhaustive()
    }
}

impl Shape {
    /// A plain `number`.
    pub fn number() -> Self {
        Self::Number(NumberShape::default())
    }

    /// A plain `boolean`.
    pub fn boolean() -> Self {
        Self::Boolean { as_sent: false }
    }

    /// A plain `string`.
    pub fn text() -> Self {
        Self::Text(TextShape::default())
    }

    /// `items[]` with no list constraints.
    pub fn array(items: Shape) -> Self {
        Self::Array(ArrayShape::of(items))
    }

    /// Wraps `self` in a passive assertion.
    pub fn assert(self, assertion: Assertion) -> Self {
        Self::Assert(Box::new(self), assertion)
    }

    /// Whether the shape admits `null` by declaring it: `null` itself, or a
    /// union (seen through assertions) with a `null` option.
    ///
    /// [`Shape::Any`] keeps `null` inside the engine, but a parameter binder
    /// admits a null argument only when this is `true`: only an explicit
    /// `| null` admits null, even for `any`.
    pub fn admits_null_explicitly(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Union(options) => options.iter().any(Self::admits_null_explicitly),
            Self::Assert(inner, _) | Self::Deferrable(inner) => inner.admits_null_explicitly(),
            Self::Ref(reference) => reference.target().is_some_and(|target| target.admits_null_explicitly()),
            _ => false,
        }
    }

    /// The expected-type text, constraints included, in SimplifiedSchema-like
    /// spelling (`number(integer; min(1))`, `tuple([string, number?])`).
    ///
    /// Text beyond [`DESCRIPTION_LIMIT`] bytes ends in `…`: every failure
    /// describes its shape, and formatting stops at the limit so the cost
    /// does not grow with a large schema (a whole unrolled trigger schema).
    /// [`fmt::Display`] writes the full text.
    pub fn describe(&self) -> String {
        let mut writer = BoundedText {
            text: String::new(),
            truncated: false,
        };
        if fmt::write(&mut writer, format_args!("{self}")).is_err() && writer.truncated {
            writer.text.push('…');
        }
        writer.text
    }

    /// Whether this shape's description needs parentheses before `[]`, `?`,
    /// or `&`.
    fn is_compound(&self) -> bool {
        match self {
            Self::Union(options) => options.len() > 1,
            Self::Assert(..) => true,
            Self::Deferrable(inner) => inner.is_compound(),
            _ => false,
        }
    }

    /// Marks `self` as a place whose value may be supplied later
    /// ([`Shape::Deferrable`]); marking twice is marking once.
    pub fn deferrable(self) -> Self {
        match self {
            Self::Deferrable(_) => self,
            other => Self::Deferrable(Box::new(other)),
        }
    }
}

/// The longest expected-type text [`Shape::describe`] writes before
/// cutting it short.
pub const DESCRIPTION_LIMIT: usize = 240;

/// A writer that refuses text past [`DESCRIPTION_LIMIT`], ending formatting
/// early.
struct BoundedText {
    text: String,
    truncated: bool,
}

impl fmt::Write for BoundedText {
    fn write_str(&mut self, piece: &str) -> fmt::Result {
        let room = DESCRIPTION_LIMIT.saturating_sub(self.text.len());
        if piece.len() <= room {
            self.text.push_str(piece);
            return Ok(());
        }
        let cut = (0..=room).rev().find(|&index| piece.is_char_boundary(index)).unwrap_or(0);
        self.text.push_str(&piece[..cut]);
        self.truncated = true;
        Err(fmt::Error)
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.write_str("any"),
            Self::Null => f.write_str("null"),
            Self::Number(number) => fmt_number(number, f),
            Self::Boolean { as_sent: false } => f.write_str("boolean"),
            Self::Boolean { as_sent: true } => f.write_str("boolish"),
            Self::Text(text) => fmt_text(text, f),
            Self::Literal(value) => write!(f, "literal({})", compact(value)),
            Self::Content(ContentFormat::Json) => f.write_str("json"),
            Self::Content(ContentFormat::Yaml) => f.write_str("yaml"),
            Self::Array(array) => {
                fmt_grouped(&array.items, f)?;
                f.write_str("[]")?;
                let mut constraints = Vec::new();
                push_opt(&mut constraints, "min", array.min_items);
                push_opt(&mut constraints, "max", array.max_items);
                if array.unique_items {
                    constraints.push("unique".to_string());
                }
                fmt_constraints(&constraints, f)
            }
            Self::Tuple(tuple) => {
                f.write_str("tuple([")?;
                let mut first = true;
                for slot in &tuple.prefix {
                    separator(&mut first, f)?;
                    if slot.optional {
                        fmt_grouped(&slot.shape, f)?;
                        f.write_str("?")?;
                    } else {
                        write!(f, "{}", slot.shape)?;
                    }
                }
                if let Some(rest) = &tuple.rest {
                    separator(&mut first, f)?;
                    f.write_str("...")?;
                    fmt_grouped(rest, f)?;
                    f.write_str("[]")?;
                }
                f.write_str("])")
            }
            Self::Object(object) => fmt_object(object, f),
            Self::Union(options) if options.is_empty() => f.write_str("never"),
            Self::Union(options) => {
                for (index, option) in options.iter().enumerate() {
                    if index > 0 {
                        f.write_str(" | ")?;
                    }
                    write!(f, "{option}")?;
                }
                Ok(())
            }
            Self::Assert(inner, assertion) => {
                fmt_grouped(inner, f)?;
                write!(f, " & {}", assertion.description)
            }
            Self::Ref(reference) => f.write_str(reference.name()),
            Self::Deferrable(inner) => write!(f, "{inner}"),
        }
    }
}

fn fmt_number(number: &NumberShape, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let bounds = &number.bounds;
    let mut constraints = Vec::new();
    let keyword = match (number.as_sent, bounds.integer) {
        (true, _) => "numberlike",
        (false, Some(IntegerRule::Whole)) => "integer",
        (false, _) => "number",
    };
    if bounds.integer == Some(IntegerRule::WholeBelow2Pow64) || (number.as_sent && bounds.integer.is_some()) {
        constraints.push("integer".to_string());
    }
    for (name, bound) in [
        ("min", bounds.minimum),
        ("max", bounds.maximum),
        ("exclusive-min", bounds.exclusive_minimum),
        ("exclusive-max", bounds.exclusive_maximum),
        ("multiple-of", bounds.multiple_of),
    ] {
        if let Some(bound) = bound {
            constraints.push(format!("{name}({})", format_number(bound)));
        }
    }
    f.write_str(keyword)?;
    fmt_constraints(&constraints, f)
}

fn fmt_text(text: &TextShape, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut constraints = Vec::new();
    push_opt(&mut constraints, "min", text.min_length);
    push_opt(&mut constraints, "max", text.max_length);
    if let Some(pattern) = &text.pattern {
        constraints.push(format!("pattern({})", pattern.source()));
    }
    match &text.members {
        Some(members) => {
            f.write_str("enum(")?;
            for (index, member) in members.iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{}", compact(&Value::String(member.clone())))?;
            }
            f.write_str(")")?;
        }
        None if text.scalar_text => f.write_str("string")?,
        None => f.write_str("alphanumeric")?,
    }
    fmt_constraints(&constraints, f)
}

fn fmt_object(object: &ObjectShape, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if object.is_opaque() {
        f.write_str("object")?;
    } else {
        f.write_str("{ ")?;
        let mut first = true;
        for property in &object.properties {
            separator(&mut first, f)?;
            let marker = if property.required { "" } else { "?" };
            write!(f, "{}{marker}: {}", property.name, property.shape)?;
        }
        for pattern in &object.patterns {
            separator(&mut first, f)?;
            match &pattern.matcher {
                KeyMatcher::Any => f.write_str("<string>")?,
                KeyMatcher::Prefix(prefix) => write!(f, "<starting::{prefix}>")?,
                KeyMatcher::Suffix(suffix) => write!(f, "<ending::{suffix}>")?,
                KeyMatcher::Pattern(pattern) => write!(f, "<pattern::{}>", pattern.source())?,
            }
            write!(f, ": {}", pattern.shape)?;
        }
        match &object.additional {
            Additional::Any => {}
            Additional::Forbidden => {
                separator(&mut first, f)?;
                f.write_str("no other keys")?;
            }
            Additional::Shape(shape) => {
                separator(&mut first, f)?;
                write!(f, "...: {shape}")?;
            }
        }
        f.write_str(" }")?;
    }
    let mut constraints = Vec::new();
    push_opt(&mut constraints, "min-keys", object.min_properties);
    push_opt(&mut constraints, "max-keys", object.max_properties);
    fmt_constraints(&constraints, f)
}

fn fmt_grouped(shape: &Shape, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if shape.is_compound() {
        write!(f, "({shape})")
    } else {
        write!(f, "{shape}")
    }
}

fn fmt_constraints(constraints: &[String], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if constraints.is_empty() {
        return Ok(());
    }
    write!(f, "({})", constraints.join("; "))
}

fn push_opt(constraints: &mut Vec<String>, name: &str, value: Option<usize>) {
    if let Some(value) = value {
        constraints.push(format!("{name}({value})"));
    }
}

fn separator(first: &mut bool, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if !std::mem::take(first) {
        f.write_str(", ")?;
    }
    Ok(())
}

/// Compact JSON text for a value in a description or message. A float
/// number reads as interpolation presents it (`0.0` is `0`), since an
/// expression literal `0` is stored as a float.
pub(crate) fn compact(value: &Value) -> String {
    if let Value::Number(number) = value
        && let Some(float) = number.as_f64().filter(|_| number.is_f64())
    {
        return crate::markdown::numeric::format_number(float);
    }
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::markdown::coercion::{FailureReason, bind};

    /// `{ n: number, next: <self> }`, owned by the returned shape.
    fn linked_list() -> Shape {
        let slot = RefSlot::new("node");
        let body = Shape::Object(ObjectShape {
            properties: vec![
                PropertySlot {
                    name: "n".into(),
                    shape: Shape::number(),
                    required: true,
                },
                PropertySlot {
                    name: "next".into(),
                    shape: Shape::Ref(slot.back_reference()),
                    required: false,
                },
            ],
            ..ObjectShape::opaque()
        });
        Shape::Ref(slot.finish(body))
    }

    #[test]
    fn a_reference_converts_at_every_level_it_reaches() {
        let value = (0..20).fold(json!({ "n": "0" }), |next, _| json!({ "n": "1", "next": next }));
        let expected = (0..20).fold(json!({ "n": 0 }), |next, _| json!({ "n": 1, "next": next }));
        assert_eq!(bind(&linked_list(), &value).expect("binds").value(), &expected);
    }

    #[test]
    fn dropping_the_owning_shape_frees_the_cycle() {
        let shape = linked_list();
        let Shape::Ref(owner) = &shape else { unreachable!() };
        let back = match &*owner.target().expect("set") {
            Shape::Object(object) => object.properties[1].shape.clone(),
            other => panic!("{other:?}"),
        };
        let Shape::Ref(back) = back else { unreachable!() };
        assert!(back.target().is_some() && back.same_target(owner));
        drop(shape);
        assert!(back.target().is_none(), "a back reference never keeps its target alive");
    }

    #[test]
    fn a_reference_loop_without_a_descent_stops_at_the_resource_limit() {
        let slot = RefSlot::new("loop");
        let body = Shape::Union(vec![Shape::Ref(slot.back_reference())]);
        let shape = Shape::Ref(slot.finish(body));
        let error = bind(&shape, &json!(1)).expect_err("a defective shape never binds");
        let text = format!("{error}");
        fn has_limit(failures: &[crate::markdown::coercion::ValueFailure]) -> bool {
            failures.iter().any(|failure| match &failure.reason {
                FailureReason::ResourceLimit { .. } => true,
                FailureReason::NoOptionAccepts { options } => options.iter().any(|option| has_limit(&option.failures)),
                _ => false,
            })
        }
        assert!(has_limit(&error.failures), "{text}");
        assert_eq!(shape.describe(), "loop");
    }
}
