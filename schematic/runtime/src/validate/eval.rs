//! Checking values against a [`Plan`].

use serde_json::Value;

use super::decimal::Decimal;
use super::plan::{Literal, Plan};
use super::violation::{Keyword, PathSegment, Violation, Violations};

/// How many references in a row evaluation follows without descending into
/// the value. A generated plan never cycles without descending, so the limit
/// only stops a malformed plan from looping; the rules beyond it are
/// unenforced.
const MAX_REFERENCE_HOPS: u8 = 32;

/// Checks a JSON value (a serialized request body, or a JSON form field)
/// against `plan`, appending each violation under `path`.
///
/// Object entries are visited in bytewise key order, never in the map's own
/// iteration order, so the report is the same for every `serde_json` build.
///
/// ## Examples
///
/// ```
/// use schematic_runtime::validate::{self, PathSegment, Plan, Property, Violations};
///
/// static BODY: Plan = Plan::new().properties(&[
///     Property::new("tags", &Plan::new().max_items(2).unique_items()),
/// ]);
///
/// let mut violations = Violations::new();
/// let body = serde_json::json!({"tags": ["a", "b", "a"]});
/// validate::check_value(&body, &BODY, &[PathSegment::Name("body")], &mut violations);
/// let keywords: Vec<_> = violations.iter().map(|v| v.keyword().as_str()).collect();
/// assert_eq!(keywords, ["maxItems", "uniqueItems"]);
/// ```
pub fn check_value(value: &Value, plan: &Plan, path: &[PathSegment], out: &mut Violations) {
    let mut walker = Walker {
        path: path.to_vec(),
        out,
    };
    walker.value(value, plan, 0);
}

/// Checks a string parameter against `plan`'s string, `enum`, and `const`
/// rules.
///
/// ## Examples
///
/// ```
/// use schematic_runtime::validate::{self, PathSegment, Plan, Violations};
///
/// const NAME: Plan = Plan::new().min_length(1).max_length(3);
/// let path = [PathSegment::Name("query"), PathSegment::Name("name")];
///
/// let mut violations = Violations::new();
/// validate::check_str("ab", &NAME, &path, &mut violations);
/// validate::check_str("日本語", &NAME, &path, &mut violations); // three code points
/// assert!(violations.is_empty());
///
/// validate::check_str("abcd", &NAME, &path, &mut violations);
/// assert_eq!(violations.to_string(), "query.name violates maxLength: must be at most 3 characters long");
/// ```
pub fn check_str(value: &str, plan: &Plan, path: &[PathSegment], out: &mut Violations) {
    let mut walker = Walker {
        path: path.to_vec(),
        out,
    };
    walker.string(value, plan);
    walker.whole(plan, |literal| matches!(literal, Literal::String(text) if *text == value));
}

/// Checks a numeric parameter against `plan`'s numeric, `enum`, and `const`
/// rules, exactly.
///
/// A float is read through its shortest round-trip text, so `0.3_f64` is
/// `0.3`. A NaN or infinite float violates every numeric rule `plan`
/// declares.
///
/// ## Examples
///
/// ```
/// use schematic_runtime::validate::{self, PathSegment, Plan, Violations};
///
/// const LIMIT: Plan = Plan::new().minimum("1").maximum("100");
/// let path = [PathSegment::Name("query"), PathSegment::Name("limit")];
///
/// let mut violations = Violations::new();
/// validate::check_number(100_u32, &LIMIT, &path, &mut violations);
/// assert!(violations.is_empty());
///
/// validate::check_number(500_u32, &LIMIT, &path, &mut violations);
/// assert_eq!(violations.to_string(), "query.limit violates maximum: must be at most 100");
/// ```
pub fn check_number(value: impl Numeric, plan: &Plan, path: &[PathSegment], out: &mut Violations) {
    let mut walker = Walker {
        path: path.to_vec(),
        out,
    };
    let number = value.to_decimal();
    walker.number(number.as_ref(), plan);
    if let Some(number) = number {
        walker.whole(plan, |literal| matches!(literal, Literal::Number(text) if parses_to(text, &number)));
    }
}

/// Checks an array parameter's item count against `plan`'s `minItems` and
/// `maxItems`, before its items are joined or exploded into a query.
///
/// Check each item with [`check_str`] or [`check_number`] under an
/// [`Index`](PathSegment::Index) segment.
///
/// ## Examples
///
/// ```
/// use schematic_runtime::validate::{self, PathSegment, Plan, Violations};
///
/// const IDS: Plan = Plan::new().min_items(1);
/// let mut violations = Violations::new();
/// validate::check_array_len(0, &IDS, &[PathSegment::Name("query"), PathSegment::Name("ids")], &mut violations);
/// assert_eq!(violations.to_string(), "query.ids violates minItems: must have at least 1 item");
/// ```
pub fn check_array_len(len: usize, plan: &Plan, path: &[PathSegment], out: &mut Violations) {
    let mut walker = Walker {
        path: path.to_vec(),
        out,
    };
    walker.item_count(len as u64, plan);
}

/// Checks a number against inclusive bounds known only at run time, such as
/// a pager strategy's page-size limits, with the same comparison and
/// reasons as a plan's `minimum` and `maximum`.
pub(crate) fn check_range(
    value: impl Numeric,
    minimum: Option<Decimal>,
    maximum: Option<Decimal>,
    path: &[PathSegment],
    out: &mut Violations,
) {
    let mut walker = Walker {
        path: path.to_vec(),
        out,
    };
    let number = value.to_decimal();
    walker.bound_at(number.as_ref(), Keyword::Minimum, minimum, "must be at least", |value, bound| value >= bound);
    walker.bound_at(number.as_ref(), Keyword::Maximum, maximum, "must be at most", |value, bound| value <= bound);
}

/// A primitive number a parameter can hold.
///
/// Implemented for every integer type, `f32`, and `f64`; sealed.
pub trait Numeric: sealed::Sealed + Copy {
    /// The exact value, or `None` for a NaN or infinite float.
    fn to_decimal(self) -> Option<Decimal>;
}

mod sealed {
    pub trait Sealed {}
}

macro_rules! numeric {
    ($($kind:ty),*) => {$(
        impl sealed::Sealed for $kind {}
        impl Numeric for $kind {
            fn to_decimal(self) -> Option<Decimal> {
                // Rust prints integers and finite floats as plain decimal
                // text, the shortest that round-trips for a float.
                (self as f64).is_finite().then(|| Decimal::parse(&self.to_string()).ok()).flatten()
            }
        }
    )*};
}

numeric!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

struct Walker<'a> {
    path: Vec<PathSegment>,
    out: &'a mut Violations,
}

impl Walker<'_> {
    fn value(&mut self, value: &Value, plan: &Plan, hops: u8) {
        match value {
            Value::String(text) => self.string(text, plan),
            Value::Number(number) => self.number(Decimal::from_json(number).ok().as_ref(), plan),
            Value::Array(items) => {
                self.item_count(items.len() as u64, plan);
                if plan.unique_items
                    && let Some((first, second)) = first_repeat(items)
                {
                    self.report(
                        Keyword::UniqueItems,
                        format!("must not repeat an item; items {first} and {second} are equal"),
                    );
                }
            }
            _ => {}
        }
        self.whole(plan, |literal| literal_eq(literal, value));

        match value {
            Value::Array(items) => {
                if let Some(item_plan) = plan.items {
                    for (index, item) in items.iter().enumerate() {
                        self.path.push(PathSegment::Index(index));
                        self.value(item, item_plan, 0);
                        self.path.pop();
                    }
                }
            }
            Value::Object(members) if !plan.properties.is_empty() || plan.values.is_some() => {
                let mut keys: Vec<&String> = members.keys().collect();
                keys.sort();
                for key in keys {
                    let declared = plan.properties.iter().find(|property| property.name == key.as_str());
                    let (segment, member_plan) = match (declared, plan.values) {
                        (Some(property), _) => (PathSegment::Name(property.name), property.plan),
                        (None, Some(values)) => (PathSegment::RedactedKey, values),
                        (None, None) => continue,
                    };
                    self.path.push(segment);
                    self.value(&members[key.as_str()], member_plan, 0);
                    self.path.pop();
                }
            }
            _ => {}
        }

        // A nullable plan's reference is the non-null alternative of an
        // `anyOf` with null (or a nullable `$ref`), which null never has to
        // satisfy; the plan's own `enum` and `const` above still apply.
        if let Some(target) = plan.reference
            && hops < MAX_REFERENCE_HOPS
            && !(plan.nullable && value.is_null())
        {
            self.value(value, target, hops + 1);
        }
    }

    fn string(&mut self, text: &str, plan: &Plan) {
        if plan.min_length.is_none() && plan.max_length.is_none() && plan.pattern.is_none() {
            return;
        }
        let length = text.chars().count() as u64;
        if let Some(min) = plan.min_length
            && length < min
        {
            self.report(Keyword::MinLength, format!("must be at least {} long", plural(min, "character")));
        }
        if let Some(max) = plan.max_length
            && length > max
        {
            self.report(Keyword::MaxLength, format!("must be at most {} long", plural(max, "character")));
        }
        if let Some(pattern) = plan.pattern
            && pattern.is_match(text) == Some(false)
        {
            self.report(Keyword::Pattern, "must match the declared pattern".to_owned());
        }
    }

    /// `None` is a NaN or infinite float, or a number beyond the exponent
    /// bound, which fails every declared rule.
    fn number(&mut self, number: Option<&Decimal>, plan: &Plan) {
        self.bound(number, Keyword::Minimum, plan.minimum, "must be at least", |value, bound| value >= bound);
        self.bound(number, Keyword::ExclusiveMinimum, plan.exclusive_minimum, "must be greater than", |value, bound| {
            value > bound
        });
        self.bound(number, Keyword::Maximum, plan.maximum, "must be at most", |value, bound| value <= bound);
        self.bound(number, Keyword::ExclusiveMaximum, plan.exclusive_maximum, "must be less than", |value, bound| {
            value < bound
        });
        if let Some(divisor) = plan.multiple_of.and_then(|text| Decimal::parse(text).ok())
            && let Some(holds) = number.map_or(Some(false), |number| number.is_multiple_of(&divisor))
            && !holds
        {
            self.report(Keyword::MultipleOf, format!("must be a multiple of {divisor}"));
        }
    }

    fn bound(
        &mut self,
        number: Option<&Decimal>,
        keyword: Keyword,
        bound: Option<&'static str>,
        wording: &str,
        holds: fn(&Decimal, &Decimal) -> bool,
    ) {
        self.bound_at(number, keyword, bound.and_then(|text| Decimal::parse(text).ok()), wording, holds);
    }

    fn bound_at(
        &mut self,
        number: Option<&Decimal>,
        keyword: Keyword,
        bound: Option<Decimal>,
        wording: &str,
        holds: fn(&Decimal, &Decimal) -> bool,
    ) {
        let Some(bound) = bound else {
            return;
        };
        if !number.is_some_and(|number| holds(number, &bound)) {
            self.report(keyword, format!("{wording} {bound}"));
        }
    }

    fn item_count(&mut self, count: u64, plan: &Plan) {
        if let Some(min) = plan.min_items
            && count < min
        {
            self.report(Keyword::MinItems, format!("must have at least {}", plural(min, "item")));
        }
        if let Some(max) = plan.max_items
            && count > max
        {
            self.report(Keyword::MaxItems, format!("must have at most {}", plural(max, "item")));
        }
    }

    /// `enum` and `const`, with `equals` deciding whether a literal is the
    /// value.
    fn whole(&mut self, plan: &Plan, equals: impl Fn(&Literal) -> bool) {
        if let Some(allowed) = plan.allowed
            && !allowed.iter().any(&equals)
        {
            let reason = match allowed.len() {
                1 => "must equal the one allowed value".to_owned(),
                count => format!("must be one of the {count} allowed values"),
            };
            self.report(Keyword::Enum, reason);
        }
        if let Some(constant) = plan.constant
            && !equals(constant)
        {
            self.report(Keyword::Const, "must equal the declared constant".to_owned());
        }
    }

    fn report(&mut self, keyword: Keyword, reason: String) {
        self.out.push(Violation::new(self.path.clone(), keyword, reason));
    }
}

fn plural(count: u64, noun: &str) -> String {
    if count == 1 { format!("1 {noun}") } else { format!("{count} {noun}s") }
}

/// The first pair of equal items, by the second item's position.
fn first_repeat(items: &[Value]) -> Option<(usize, usize)> {
    (1..items.len()).find_map(|second| {
        (0..second)
            .find(|&first| json_eq(&items[first], &items[second]))
            .map(|first| (first, second))
    })
}

fn json_eq(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => match (Decimal::from_json(left), Decimal::from_json(right)) {
            (Ok(left), Ok(right)) => left == right,
            _ => left == right,
        },
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(left, right)| json_eq(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, member)| right.get(key).is_some_and(|other| json_eq(member, other)))
        }
        _ => left == right,
    }
}

fn literal_eq(literal: &Literal, value: &Value) -> bool {
    match (literal, value) {
        (Literal::Null, Value::Null) => true,
        (Literal::Bool(expected), Value::Bool(actual)) => expected == actual,
        (Literal::Number(text), Value::Number(number)) => {
            Decimal::from_json(number).is_ok_and(|number| parses_to(text, &number))
        }
        (Literal::String(expected), Value::String(actual)) => expected == actual,
        (Literal::Array(expected), Value::Array(actual)) => {
            expected.len() == actual.len() && expected.iter().zip(actual).all(|(literal, item)| literal_eq(literal, item))
        }
        (Literal::Object(expected), Value::Object(actual)) => {
            expected.len() == actual.len()
                && expected
                    .iter()
                    .all(|(name, literal)| actual.get(*name).is_some_and(|member| literal_eq(literal, member)))
        }
        _ => false,
    }
}

fn parses_to(text: &str, number: &Decimal) -> bool {
    Decimal::parse(text).is_ok_and(|literal| literal == *number)
}
