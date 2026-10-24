//! The `&'static` description of the constraints one value must satisfy.

use super::pattern::Pattern;

/// The constraints on one JSON value and, through its children, on the
/// values inside it.
///
/// A plan is data only. The generator emits it as a `static` (or an
/// associated constant) built with these `const` methods, so nothing is
/// assembled at run time. Each rule applies only to a value of the matching
/// JSON kind: string rules to strings, numeric rules to numbers, array rules
/// to arrays, and child plans to object entries and array items. `enum` and
/// `const` apply to whole values of any kind. A value of another kind passes
/// the rule; checking JSON types is the deserializer's job, not this one's.
///
/// Numeric limits are JSON number text, compared exactly as
/// [`Decimal`](super::Decimal)s, so a bound beyond `u64` or a divisor of
/// `0.1` keeps its meaning. Limit text that does not parse leaves its rule
/// unenforced; the generator writes only text that does.
///
/// Recursion goes through [`Plan::reference`] to a named `static`, which
/// evaluation follows only as deep as the submitted value goes:
///
/// ```
/// use schematic_runtime::validate::{self, Plan, Property, Violations};
///
/// // A tree whose nodes have a 1–20 character name and child nodes.
/// static NODE: Plan = Plan::new().properties(&[
///     Property::new("children", &Plan::new().items(&Plan::reference(&NODE))),
///     Property::new("name", &Plan::new().min_length(1).max_length(20)),
/// ]);
///
/// let tree = serde_json::json!({"name": "root", "children": [{"name": ""}]});
/// let mut violations = Violations::new();
/// validate::check_value(&tree, &NODE, &[validate::PathSegment::Name("body")], &mut violations);
/// assert_eq!(violations.to_string(), "body.children[0].name violates minLength: must be at least 1 character long");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub(super) nullable: bool,
    pub(super) min_length: Option<u64>,
    pub(super) max_length: Option<u64>,
    pub(super) pattern: Option<&'static Pattern>,
    pub(super) minimum: Option<&'static str>,
    pub(super) exclusive_minimum: Option<&'static str>,
    pub(super) maximum: Option<&'static str>,
    pub(super) exclusive_maximum: Option<&'static str>,
    pub(super) multiple_of: Option<&'static str>,
    pub(super) min_items: Option<u64>,
    pub(super) max_items: Option<u64>,
    pub(super) unique_items: bool,
    pub(super) items: Option<&'static Plan>,
    pub(super) properties: &'static [Property],
    pub(super) values: Option<&'static Plan>,
    pub(super) allowed: Option<&'static [Literal]>,
    pub(super) constant: Option<&'static Literal>,
    pub(super) reference: Option<&'static Plan>,
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(missing_docs)]
impl Plan {
    /// A plan with no rules: every value passes.
    pub const fn new() -> Self {
        Self {
            nullable: false,
            min_length: None,
            max_length: None,
            pattern: None,
            minimum: None,
            exclusive_minimum: None,
            maximum: None,
            exclusive_maximum: None,
            multiple_of: None,
            min_items: None,
            max_items: None,
            unique_items: false,
            items: None,
            properties: &[],
            values: None,
            allowed: None,
            constant: None,
            reference: None,
        }
    }

    /// A plan that checks a value against `target`, typically a named
    /// `static`; the route for local references and recursive models.
    ///
    /// Rules added to the returned plan apply too, before `target`'s.
    pub const fn reference(target: &'static Plan) -> Self {
        let mut plan = Self::new();
        plan.reference = Some(target);
        plan
    }

    /// JSON `null` skips this plan's [reference](Plan::reference): the
    /// schema admits null as an alternative to the referenced one (an
    /// `anyOf` of the value and null, or a nullable `$ref`).
    ///
    /// Null passes the kind rules either way, since it is not a string,
    /// number, or array. This plan's own `enum` and `const` still apply: a
    /// nullable type (`nullable: true`, `type: [T, "null"]`) admits the JSON
    /// kind but does not add null to an independently declared `enum`, so
    /// null must be one of its members.
    pub const fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }

    /// The fewest Unicode code points a string may have.
    pub const fn min_length(mut self, length: u64) -> Self {
        self.min_length = Some(length);
        self
    }

    /// The most Unicode code points a string may have.
    pub const fn max_length(mut self, length: u64) -> Self {
        self.max_length = Some(length);
        self
    }

    /// A pattern a string must contain a match of.
    pub const fn pattern(mut self, pattern: &'static Pattern) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// The inclusive lower bound of a number, as JSON number text.
    pub const fn minimum(mut self, bound: &'static str) -> Self {
        self.minimum = Some(bound);
        self
    }

    /// The exclusive lower bound of a number, as JSON number text.
    pub const fn exclusive_minimum(mut self, bound: &'static str) -> Self {
        self.exclusive_minimum = Some(bound);
        self
    }

    /// The inclusive upper bound of a number, as JSON number text.
    pub const fn maximum(mut self, bound: &'static str) -> Self {
        self.maximum = Some(bound);
        self
    }

    /// The exclusive upper bound of a number, as JSON number text.
    pub const fn exclusive_maximum(mut self, bound: &'static str) -> Self {
        self.exclusive_maximum = Some(bound);
        self
    }

    /// A positive number that a number must be an integral multiple of, as
    /// JSON number text.
    pub const fn multiple_of(mut self, divisor: &'static str) -> Self {
        self.multiple_of = Some(divisor);
        self
    }

    pub const fn min_items(mut self, count: u64) -> Self {
        self.min_items = Some(count);
        self
    }

    pub const fn max_items(mut self, count: u64) -> Self {
        self.max_items = Some(count);
        self
    }

    /// No two items of an array may be equal as JSON values: object members
    /// compare regardless of order, and numbers compare mathematically.
    pub const fn unique_items(mut self) -> Self {
        self.unique_items = true;
        self
    }

    /// The plan of every array item.
    pub const fn items(mut self, plan: &'static Plan) -> Self {
        self.items = Some(plan);
        self
    }

    /// Plans of an object's declared properties, keyed by serialized name.
    /// An absent property is not checked.
    pub const fn properties(mut self, properties: &'static [Property]) -> Self {
        self.properties = properties;
        self
    }

    /// The plan of every object entry that is not a declared property (a
    /// map's values); its keys are redacted in violation paths.
    pub const fn values(mut self, plan: &'static Plan) -> Self {
        self.values = Some(plan);
        self
    }

    /// The whole values a value must equal one of (`enum`).
    pub const fn allowed_values(mut self, values: &'static [Literal]) -> Self {
        self.allowed = Some(values);
        self
    }

    /// The whole value a value must equal (`const`).
    pub const fn constant(mut self, value: &'static Literal) -> Self {
        self.constant = Some(value);
        self
    }
}

/// A declared object property and its plan.
#[derive(Debug, Clone, Copy)]
pub struct Property {
    pub(super) name: &'static str,
    pub(super) plan: &'static Plan,
}

impl Property {
    /// The property serialized as `name`, checked against `plan` when
    /// present.
    pub const fn new(name: &'static str, plan: &'static Plan) -> Self {
        Self { name, plan }
    }
}

/// A JSON value written as constant data, for `enum` and `const`.
///
/// Equality with a submitted value is JSON equality: numbers compare as
/// exact decimals (`1` equals `1.0`), object members regardless of order,
/// and `Null` is the JSON value `null`, not absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Literal {
    Null,
    Bool(bool),
    /// JSON number text.
    Number(&'static str),
    String(&'static str),
    Array(&'static [Literal]),
    /// Members with distinct names.
    Object(&'static [(&'static str, Literal)]),
}
