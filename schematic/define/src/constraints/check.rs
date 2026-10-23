//! Definition-time checks of authored and loaded constraints.

use std::fmt;

use serde_json::Value;

use super::{Constraints, PatternAdmission, admit_pattern};
use crate::{ApiRequest, Endpoint, FormFieldKind, QueryParamType, RestApi, TypeRef};

/// One constraint a definition cannot carry, located by endpoint and input.
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionContext, Endpoint, ParamDef, QueryParamType, RestApi};
/// use schematic_define::constraints::{Constraints, constraint_problems};
///
/// let ctx = DefinitionContext::types_in("::models");
/// let endpoint = Endpoint::get("List", "/items").describes("List").returns_empty()
///     .query(ParamDef::new("limit", QueryParamType::Integer).constraints(Constraints::new().maximum(100)))
///     .build()?;
/// let mut api = RestApi::builder(&ctx, "Items", "https://api.example.com")
///     .describes("Items").no_auth().endpoints([endpoint]).build()?;
/// assert!(constraint_problems(&api).is_empty());
///
/// // Fields stay public, so a later edit is caught by the next check.
/// let query = &mut api.endpoints[0].params.as_mut().unwrap().query[0];
/// query.constraints = Some(Box::new(Constraints::new().min_length(1)));
/// let problem = &constraint_problems(&api)[0];
/// assert_eq!((problem.path.as_str(), problem.keyword), ("query.limit", "minLength"));
/// # Ok::<(), schematic_define::DefinitionError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintProblem {
    /// The endpoint id.
    pub endpoint: String,
    /// The constrained input: `query.limit`, `path.id`, `header.x-mode`,
    /// `cookie.session`, `form.title`, or `body`; `[]` names array items.
    pub path: String,
    /// The JSON Schema keyword at fault (`maxLength`, `enum`, ...).
    pub keyword: &'static str,
    /// Why the definition cannot carry it.
    pub reason: String,
}

impl fmt::Display for ConstraintProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "endpoint {}: {} `{}`: {}", self.endpoint, self.path, self.keyword, self.reason)
    }
}

/// Every constraint problem of `api`'s endpoints, in endpoint order, then
/// path, query, header, and cookie parameters, form fields, and the body.
pub fn constraint_problems(api: &RestApi) -> Vec<ConstraintProblem> {
    api.endpoints.iter().flat_map(endpoint_problems).collect()
}

pub(crate) fn endpoint_problems(endpoint: &Endpoint) -> Vec<ConstraintProblem> {
    let mut found = Vec::new();
    let mut checked = |path: String, rules: &Constraints, target: &Target| {
        for (path, keyword, reason) in check(rules, target, &path) {
            found.push(ConstraintProblem { endpoint: endpoint.id.clone(), path, keyword, reason });
        }
    };
    if let Some(params) = &endpoint.params {
        let located = [("path", &params.path), ("query", &params.query), ("header", &params.header), ("cookie", &params.cookie)];
        for (location, params) in located {
            for param in params {
                if let Some(rules) = &param.constraints {
                    checked(format!("{location}.{}", param.name), rules, &Target::param(&param.param_type));
                }
            }
        }
    }
    if let Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) = &endpoint.request {
        for field in fields {
            if let Some(rules) = &field.constraints {
                let target = match &field.kind {
                    FormFieldKind::Text => Target::of(Kind::String),
                    FormFieldKind::Json(reference) => Target::type_ref(reference),
                    _ => Target::of(Kind::Refused("file contents have no JSON value to constrain")),
                };
                checked(format!("form.{}", field.name), rules, &target);
            }
        }
    }
    if let Some(rules) = &endpoint.request_constraints {
        let target = match &endpoint.request {
            None => Target::of(Kind::Refused("the endpoint declares no request body")),
            Some(ApiRequest::Json(TypeRef::Named(_))) => Target::of(Kind::Refused(COMPONENT)),
            Some(ApiRequest::Json(reference)) => Target::type_ref(reference),
            Some(ApiRequest::Text { .. }) => Target::of(Kind::String),
            Some(ApiRequest::Binary { .. }) => Target::of(Kind::Refused("a binary body has no JSON value to constrain")),
            Some(ApiRequest::FormData { .. } | ApiRequest::UrlEncoded { .. }) => {
                Target::of(Kind::Refused("a form body carries its rules on its fields"))
            }
        };
        checked("body".to_owned(), rules, &target);
    }
    found
}

const COMPONENT: &str = "the value is a schema component, whose rules are Schemars attributes on its model";

/// The JSON kind a constrained value has, as far as its declared type shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    String,
    Integer,
    Number,
    Boolean,
    Array(Box<Target>),
    Object,
    Null,
    /// Free-form JSON: every rule applies to the matching kind.
    Any,
    /// No rule may be attached, for the reason given.
    Refused(&'static str),
}

/// A constrained value's kind, and whether its type admits `null`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    kind: Kind,
    nullable: bool,
}

impl Target {
    pub(crate) fn of(kind: Kind) -> Self { Self { kind, nullable: false } }

    pub(crate) fn param(param_type: &QueryParamType) -> Self {
        Self::of(match param_type {
            QueryParamType::String | QueryParamType::Enum(_) => Kind::String,
            QueryParamType::Integer => Kind::Integer,
            QueryParamType::Number => Kind::Number,
            QueryParamType::Boolean => Kind::Boolean,
            QueryParamType::Array(item) => Kind::Array(Box::new(Self::param(item))),
            QueryParamType::Json => Kind::Any,
        })
    }

    pub(crate) fn type_ref(reference: &TypeRef) -> Self {
        use crate::PrimitiveType as P;
        Self::of(match reference {
            TypeRef::Primitive(P::String | P::Char) => Kind::String,
            TypeRef::Primitive(P::Integer | P::I8 | P::I16 | P::I32 | P::I128 | P::Isize
                | P::U8 | P::U16 | P::U32 | P::U64 | P::U128 | P::Usize) => Kind::Integer,
            TypeRef::Primitive(P::Number | P::F32) => Kind::Number,
            TypeRef::Primitive(P::Boolean) => Kind::Boolean,
            TypeRef::Primitive(P::Unit) => Kind::Null,
            TypeRef::Primitive(P::Bytes) => Kind::Refused("bytes have no JSON value to constrain"),
            TypeRef::Primitive(P::Json) | TypeRef::Unknown => Kind::Any,
            TypeRef::Array(item) => Kind::Array(Box::new(Self::type_ref(item))),
            TypeRef::Map(_) | TypeRef::KeyedMap { .. } => Kind::Object,
            TypeRef::Named(_) => Kind::Refused(COMPONENT),
            TypeRef::OneOf(_) | TypeRef::AnyOf(_) | TypeRef::AllOf(_) => {
                Kind::Refused("rules under a composition (`oneOf`, `anyOf`, `allOf`) cannot be enforced")
            }
            TypeRef::Optional(inner) => return Self { nullable: true, ..Self::type_ref(inner) },
        })
    }

    fn accepts(&self, group: Group) -> bool {
        matches!((&self.kind, group),
            (Kind::Any, _) | (_, Group::Value)
            | (Kind::String, Group::String | Group::Format)
            | (Kind::Integer | Kind::Number, Group::Number | Group::Format)
            | (Kind::Array(_), Group::Array))
    }

    fn describe(&self) -> &'static str {
        match self.kind {
            Kind::String => "a string",
            Kind::Integer => "an integer",
            Kind::Number => "a number",
            Kind::Boolean => "a boolean",
            Kind::Array(_) => "an array",
            Kind::Object => "an object",
            Kind::Null => "null",
            Kind::Any | Kind::Refused(_) => "a JSON value",
        }
    }

    /// Whether `value` is a value of this type.
    fn admits(&self, value: &Value) -> bool {
        match (&self.kind, value) {
            (Kind::Any, _) => true,
            (_, Value::Null) => self.nullable || self.kind == Kind::Null,
            (Kind::String, Value::String(_)) | (Kind::Boolean, Value::Bool(_)) | (Kind::Object, Value::Object(_)) => true,
            (Kind::Number, Value::Number(_)) => true,
            (Kind::Integer, Value::Number(number)) => {
                number.is_i64() || number.is_u64() || number.as_f64().is_some_and(|float| float.fract() == 0.0)
            }
            (Kind::Array(item), Value::Array(values)) => values.iter().all(|value| item.admits(value)),
            _ => false,
        }
    }
}

#[derive(Clone, Copy)]
enum Group { String, Format, Number, Array, Value }

/// Keywords in report order, with the kind group each belongs to.
fn present(rules: &Constraints) -> Vec<(&'static str, Group)> {
    let mut keywords = Vec::new();
    let mut add = |is: bool, keyword, group| if is { keywords.push((keyword, group)) };
    add(rules.min_length.is_some(), "minLength", Group::String);
    add(rules.max_length.is_some(), "maxLength", Group::String);
    add(rules.pattern.is_some() || rules.pattern_unenforced, "pattern", Group::String);
    add(rules.format.is_some(), "format", Group::Format);
    add(rules.minimum.is_some(), "minimum", Group::Number);
    add(rules.maximum.is_some(), "maximum", Group::Number);
    add(rules.exclusive_minimum.is_some(), "exclusiveMinimum", Group::Number);
    add(rules.exclusive_maximum.is_some(), "exclusiveMaximum", Group::Number);
    add(rules.multiple_of.is_some(), "multipleOf", Group::Number);
    add(rules.min_items.is_some(), "minItems", Group::Array);
    add(rules.max_items.is_some(), "maxItems", Group::Array);
    add(rules.unique_items, "uniqueItems", Group::Array);
    add(rules.items.is_some(), "items", Group::Array);
    add(rules.allowed_values.is_some(), "enum", Group::Value);
    add(rules.constant.is_some(), "const", Group::Value);
    keywords
}

/// Every problem of `rules` on a value of `target` at `path`, as
/// `(path, keyword, reason)`.
pub(crate) fn check(rules: &Constraints, target: &Target, path: &str) -> Vec<(String, &'static str, String)> {
    let mut found = Vec::new();
    let mut fail = |keyword: &'static str, reason: String| found.push((path.to_owned(), keyword, reason));

    if let Kind::Refused(reason) = target.kind {
        if let Some((keyword, _)) = present(rules).first() { fail(keyword, reason.to_owned()); }
        return found;
    }
    let mut compatible = true;
    for (keyword, group) in present(rules) {
        if !target.accepts(group) {
            fail(keyword, format!("does not apply to {}", target.describe()));
            compatible = false;
        }
    }
    if !compatible { return found; }

    for (keyword, low, high) in [("minLength", rules.min_length, rules.max_length), ("minItems", rules.min_items, rules.max_items)] {
        if let (Some(low), Some(high)) = (low, high)
            && low > high { fail(keyword, format!("{low} is above the maximum {high}")); }
    }
    if let Some(source) = &rules.pattern {
        if !rules.pattern_unenforced {
            match admit_pattern(source) {
                PatternAdmission::Enforced { .. } => {}
                PatternAdmission::Unsupported { reason } => fail("pattern", format!("is outside the enforced pattern subset: {reason}")),
                PatternAdmission::Invalid { reason } => fail("pattern", format!("is not a valid regular expression: {reason}")),
            }
        }
    } else if rules.pattern_unenforced {
        fail("pattern", "is marked unenforced but no pattern is declared".to_owned());
    }
    if rules.format.as_deref().is_some_and(str::is_empty) { fail("format", "names no format".to_owned()); }

    let numbers = [("minimum", &rules.minimum), ("maximum", &rules.maximum), ("exclusiveMinimum", &rules.exclusive_minimum),
        ("exclusiveMaximum", &rules.exclusive_maximum), ("multipleOf", &rules.multiple_of)];
    let mut parsed = Vec::new();
    for (keyword, number) in numbers {
        parsed.push(match number.as_ref().map(|number| number.exact()) {
            None => None,
            Some(Ok(exact)) => Some(exact),
            Some(Err(reason)) => { fail(keyword, format!("limit `{}` {reason}", number.as_ref().map_or("", |n| n.as_str()))); None }
        });
    }
    let [minimum, maximum, exclusive_minimum, exclusive_maximum, multiple_of] = <[_; 5]>::try_from(parsed).expect("five limits");
    if multiple_of.as_ref().is_some_and(|divisor| !divisor.is_positive()) { fail("multipleOf", "must be greater than zero".to_owned()); }
    // The tighter lower and upper bound; equal bounds are an empty interval
    // when either is exclusive.
    let lower = match (&minimum, &exclusive_minimum) {
        (Some(inclusive), Some(exclusive)) if exclusive >= inclusive => Some((exclusive, true)),
        (Some(inclusive), _) => Some((inclusive, false)),
        (None, Some(exclusive)) => Some((exclusive, true)),
        (None, None) => None,
    };
    let upper = match (&maximum, &exclusive_maximum) {
        (Some(inclusive), Some(exclusive)) if exclusive <= inclusive => Some((exclusive, true)),
        (Some(inclusive), _) => Some((inclusive, false)),
        (None, Some(exclusive)) => Some((exclusive, true)),
        (None, None) => None,
    };
    if let (Some((low, low_open)), Some((high, high_open))) = (lower, upper)
        && (low > high || (low == high && (low_open || high_open))) {
        let keyword = if low_open { "exclusiveMinimum" } else { "minimum" };
        fail(keyword, "leaves no number between the lower and upper bounds".to_owned());
    }

    if let Some(values) = &rules.allowed_values {
        if values.is_empty() { fail("enum", "must list at least one value".to_owned()); }
        if let Some(index) = values.iter().position(|value| !target.admits(value)) {
            fail("enum", format!("member {index} is not {}", target.describe_value()));
        }
    }
    if rules.constant.as_ref().is_some_and(|value| !target.admits(value)) {
        fail("const", format!("is not {}", target.describe_value()));
    }

    if let Some(item_rules) = &rules.items {
        let item = match &target.kind {
            Kind::Array(item) => (**item).clone(),
            _ => Target::of(Kind::Any),
        };
        found.extend(check(item_rules, &item, &format!("{path}[]")));
    }
    found
}

impl Target {
    fn describe_value(&self) -> String {
        if self.nullable { format!("{} or null", self.describe()) } else { self.describe().to_owned() }
    }
}
