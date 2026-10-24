//! The rule core: keep, convert once, or fail, for every value a shape
//! declares.

use std::collections::HashSet;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use super::error::{Constraint, FailureReason, PathSegment, ValueFailure};
use super::shape::{
    Additional, ArrayShape, ContentFormat, IntegerRule, NumberShape, NumericBounds, ObjectShape, PatternScope,
    Shape, TextShape, TupleShape,
};
use crate::markdown::numeric::{
    NumberTextError, admit_json_number, admit_number_text, format_number, json_number, parse_boolean_word,
};

/// The one direct conversion that produced a value, recorded per path so the
/// union tie-breakers can compare readings field by field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Conversion {
    TextToNumber,
    TextToBoolean,
    NumberToText,
    BooleanToText,
    ToJson,
    ToYaml,
}

pub(super) type Conversions = Vec<(Vec<PathSegment>, Conversion)>;
pub(super) type Failures = Vec<ValueFailure>;

/// What binding did with one value.
pub(super) enum Step {
    Kept,
    Converted(Value, Conversions),
}

/// One step of a path, borrowed from the value being walked.
#[derive(Clone, Copy)]
pub(super) enum Segment<'a> {
    Index(usize),
    Key(&'a str),
}

/// A path as a borrowed chain, so walking allocates nothing until a failure,
/// a conversion, or a deferral predicate needs the owned form.
#[derive(Clone, Copy)]
pub(super) struct Path<'p> {
    parent: Option<&'p Path<'p>>,
    segment: Option<Segment<'p>>,
    pub(super) depth: usize,
    /// References followed at this value since its last step down.
    hops: usize,
}

/// How many references in a row the engine follows at one value before it
/// stops with [`FailureReason::ResourceLimit`]. Translators close every
/// cycle through a value descent, so only a defective shape reaches it.
const MAX_REFERENCE_HOPS: usize = 64;

impl<'p> Path<'p> {
    pub(super) fn root() -> Self {
        Self {
            parent: None,
            segment: None,
            depth: 0,
            hops: 0,
        }
    }

    /// The same value, one reference further.
    fn hop(self) -> Self {
        Self {
            hops: self.hops + 1,
            ..self
        }
    }

    fn exhausted(&self, max_depth: usize) -> bool {
        self.depth > max_depth || self.hops > MAX_REFERENCE_HOPS
    }

    pub(super) fn child<'c>(&'c self, segment: Segment<'c>) -> Path<'c> {
        Path {
            parent: Some(self),
            segment: Some(segment),
            depth: self.depth + 1,
            hops: 0,
        }
    }

    pub(super) fn to_vec(self) -> Vec<PathSegment> {
        let mut segments = Vec::with_capacity(self.depth);
        let mut node = Some(&self);
        while let Some(current) = node {
            if let Some(segment) = current.segment {
                segments.push(match segment {
                    Segment::Index(index) => PathSegment::Index(index),
                    Segment::Key(key) => PathSegment::Key(key.to_string()),
                });
            }
            node = current.parent;
        }
        segments.reverse();
        segments
    }
}

pub(super) struct Engine<'o> {
    pub(super) defer: Option<&'o super::Defer>,
    pub(super) max_depth: usize,
    /// `false` for exact checks: any needed conversion is a failure.
    pub(super) convert: bool,
}

impl Engine<'_> {
    /// Whether `value` matches `shape` unchanged. Allocates no error.
    pub(super) fn exact(&self, shape: &Shape, value: &Value, path: Path<'_>) -> bool {
        if path.exhausted(self.max_depth) {
            return false;
        }
        if self.deferred(value, path) {
            return true;
        }
        match shape {
            Shape::Any => true,
            Shape::Null => value.is_null(),
            Shape::Number(number) => exact_number(number, value).is_ok(),
            Shape::Boolean { as_sent } => match value {
                Value::Bool(_) => true,
                Value::String(text) => *as_sent && parse_boolean_word(text).is_some(),
                _ => false,
            },
            Shape::Text(text_shape) => match value {
                Value::String(text) => text_constraint(text_shape, text).is_none(),
                _ => false,
            },
            Shape::Literal(literal) => same_value(value, literal),
            Shape::Content(format) => match value {
                Value::String(text) => content_error(*format, text).is_none(),
                _ => false,
            },
            Shape::Array(array) => match value {
                Value::Array(items) => {
                    array_constraint(array, items).is_none()
                        && items
                            .iter()
                            .enumerate()
                            .all(|(index, item)| self.exact(&array.items, item, path.child(Segment::Index(index))))
                }
                _ => false,
            },
            Shape::Tuple(tuple) => match value {
                Value::Array(items) => items.len() >= required_slots(tuple) && {
                    items.iter().enumerate().all(|(index, item)| {
                        tuple_slot(tuple, index).is_some_and(|slot| self.exact(slot, item, path.child(Segment::Index(index))))
                    })
                },
                _ => false,
            },
            Shape::Object(object) => match value {
                Value::Object(map) => {
                    object_count_constraint(object, map).is_none()
                        && missing_required(object, map).next().is_none()
                        && map.iter().all(|(key, item)| {
                            let child = path.child(Segment::Key(key));
                            match declarations(object, key) {
                                Declarations::None => match &object.additional {
                                    Additional::Any => true,
                                    Additional::Forbidden => false,
                                    Additional::Shape(shape) => self.exact(shape, item, child),
                                },
                                Declarations::Some(shapes) => shapes.iter().all(|shape| self.exact(shape, item, child)),
                            }
                        })
                }
                _ => false,
            },
            Shape::Union(options) => options.iter().any(|option| self.exact(option, value, path)),
            Shape::Assert(inner, assertion) => self.exact(inner, value, path) && assertion.holds(value),
            Shape::Ref(reference) => reference.target().is_some_and(|target| self.exact(&target, value, path.hop())),
        }
    }

    /// Binds `value` to `shape`: kept, converted once and checked, or every
    /// failing path.
    pub(super) fn walk(&self, shape: &Shape, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        if path.exhausted(self.max_depth) {
            return Err(fail(
                shape,
                value,
                path,
                None,
                FailureReason::ResourceLimit {
                    max_depth: self.max_depth,
                },
            ));
        }
        if self.deferred(value, path) {
            return Ok(Step::Kept);
        }
        match shape {
            Shape::Any => Ok(Step::Kept),
            Shape::Null if value.is_null() => Ok(Step::Kept),
            Shape::Null => Err(fail(shape, value, path, None, FailureReason::NoConversion)),
            Shape::Number(number) => self.number(shape, number, value, path),
            Shape::Boolean { as_sent } => self.boolean(shape, *as_sent, value, path),
            Shape::Text(text) => self.text(shape, text, value, path),
            Shape::Literal(literal) => self.literal(shape, literal, value, path),
            Shape::Content(format) => self.content(shape, *format, value, path),
            Shape::Array(array) => self.array(shape, array, value, path),
            Shape::Tuple(tuple) => self.tuple(shape, tuple, value, path),
            Shape::Object(object) => self.object(shape, object, value, path),
            Shape::Union(options) => self.union(shape, options, value, path),
            Shape::Assert(inner, assertion) => {
                let step = self.walk(inner, value, path).map_err(|mut failures| {
                    let here = path.to_vec();
                    for failure in failures.iter_mut().filter(|failure| failure.path == here) {
                        failure.expected = shape.describe();
                    }
                    failures
                })?;
                let (final_value, converted) = match &step {
                    Step::Kept => (value, None),
                    Step::Converted(converted, _) => (converted, Some(converted.clone())),
                };
                if assertion.holds(final_value) {
                    Ok(step)
                } else {
                    let description = assertion.description().to_string();
                    Err(fail(shape, value, path, converted, FailureReason::AssertionFailed { description }))
                }
            }
            Shape::Ref(reference) => match reference.target() {
                Some(target) => self.walk(&target, value, path.hop()),
                None => Err(fail(shape, value, path, None, FailureReason::NoConversion)),
            },
        }
    }

    fn deferred(&self, value: &Value, path: Path<'_>) -> bool {
        self.defer.is_some_and(|defer| defer(&path.to_vec(), value))
    }

    /// A failure for a value that needs a conversion: the conversion's own
    /// failure when converting, [`FailureReason::ConversionNotAllowed`] in an
    /// exact check.
    fn needs_conversion(&self, shape: &Shape, value: &Value, path: Path<'_>) -> Option<Failures> {
        (!self.convert).then(|| fail(shape, value, path, None, FailureReason::ConversionNotAllowed))
    }

    fn number(&self, shape: &Shape, number: &NumberShape, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        let text = match value {
            Value::Number(_) => {
                return exact_number(number, value)
                    .map(|()| Step::Kept)
                    .map_err(|reason| fail(shape, value, path, None, reason));
            }
            Value::String(text) => text,
            _ => return Err(fail(shape, value, path, None, FailureReason::NoConversion)),
        };
        if !number.as_sent
            && let Some(failures) = self.needs_conversion(shape, value, path)
        {
            return Err(failures);
        }
        let admitted = admit_number_text(text).map_err(|error| fail(shape, value, path, None, text_reason(error)))?;
        if let Some(constraint) = bounds_constraint(&number.bounds, admitted) {
            let converted = (!number.as_sent).then(|| stored_number(admitted));
            return Err(fail(shape, value, path, converted, FailureReason::ConstraintNotMet(constraint)));
        }
        if number.as_sent {
            return Ok(Step::Kept);
        }
        Ok(converted(stored_number(admitted), path, Conversion::TextToNumber))
    }

    fn boolean(&self, shape: &Shape, as_sent: bool, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        let text = match value {
            Value::Bool(_) => return Ok(Step::Kept),
            Value::String(text) => text,
            _ => return Err(fail(shape, value, path, None, FailureReason::NoConversion)),
        };
        if !as_sent
            && let Some(failures) = self.needs_conversion(shape, value, path)
        {
            return Err(failures);
        }
        let word = parse_boolean_word(text).ok_or_else(|| {
            let detail = "not a boolean word".to_string();
            fail(shape, value, path, None, FailureReason::MalformedText { detail })
        })?;
        if as_sent {
            Ok(Step::Kept)
        } else {
            Ok(converted(Value::Bool(word), path, Conversion::TextToBoolean))
        }
    }

    fn text(&self, shape: &Shape, text_shape: &TextShape, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        if let Value::String(text) = value {
            return match text_constraint(text_shape, text) {
                None => Ok(Step::Kept),
                Some(constraint) => Err(fail(shape, value, path, None, FailureReason::ConstraintNotMet(constraint))),
            };
        }
        if !text_shape.scalar_text {
            return Err(fail(shape, value, path, None, FailureReason::NoConversion));
        }
        let (text, conversion) = self.scalar_to_text(shape, value, path)?;
        if let Some(constraint) = text_constraint(text_shape, &text) {
            let converted = Some(Value::String(text));
            return Err(fail(shape, value, path, converted, FailureReason::ConstraintNotMet(constraint)));
        }
        Ok(converted(Value::String(text), path, conversion))
    }

    /// Number → text and boolean → text; every other type has no conversion
    /// into text.
    fn scalar_to_text(&self, shape: &Shape, value: &Value, path: Path<'_>) -> Result<(String, Conversion), Failures> {
        match value {
            Value::Number(number) => {
                let admitted = admit_json_number(number)
                    .map_err(|error| fail(shape, value, path, None, FailureReason::NumberNotAdmitted(error)))?;
                if let Some(failures) = self.needs_conversion(shape, value, path) {
                    return Err(failures);
                }
                Ok((format_number(admitted), Conversion::NumberToText))
            }
            Value::Bool(flag) => {
                if let Some(failures) = self.needs_conversion(shape, value, path) {
                    return Err(failures);
                }
                Ok((flag.to_string(), Conversion::BooleanToText))
            }
            _ => Err(fail(shape, value, path, None, FailureReason::NoConversion)),
        }
    }

    fn literal(&self, shape: &Shape, literal: &Value, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        if same_value(value, literal) {
            return Ok(Step::Kept);
        }
        let mismatch = |converted: Option<Value>| {
            fail(shape, value, path, converted, FailureReason::ConstraintNotMet(Constraint::Literal(literal.clone())))
        };
        let (candidate, conversion) = match (literal, value) {
            (Value::String(_), Value::Number(_) | Value::Bool(_)) => {
                let (text, conversion) = self.scalar_to_text(shape, value, path)?;
                (Value::String(text), conversion)
            }
            (Value::Number(_), Value::String(text)) => {
                if let Some(failures) = self.needs_conversion(shape, value, path) {
                    return Err(failures);
                }
                let admitted =
                    admit_number_text(text).map_err(|error| fail(shape, value, path, None, text_reason(error)))?;
                (stored_number(admitted), Conversion::TextToNumber)
            }
            (Value::Bool(_), Value::String(text)) => {
                if let Some(failures) = self.needs_conversion(shape, value, path) {
                    return Err(failures);
                }
                let word = parse_boolean_word(text).ok_or_else(|| {
                    let detail = "not a boolean word".to_string();
                    fail(shape, value, path, None, FailureReason::MalformedText { detail })
                })?;
                (Value::Bool(word), Conversion::TextToBoolean)
            }
            _ if std::mem::discriminant(literal) == std::mem::discriminant(value) => return Err(mismatch(None)),
            _ => return Err(fail(shape, value, path, None, FailureReason::NoConversion)),
        };
        if same_value(&candidate, literal) {
            Ok(converted(candidate, path, conversion))
        } else {
            Err(mismatch(Some(candidate)))
        }
    }

    fn content(&self, shape: &Shape, format: ContentFormat, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        match value {
            Value::String(text) => match content_error(format, text) {
                None => Ok(Step::Kept),
                Some(detail) => Err(fail(shape, value, path, None, FailureReason::MalformedText { detail })),
            },
            Value::Null => Err(fail(shape, value, path, None, FailureReason::NoConversion)),
            _ => {
                if let Some(failures) = self.needs_conversion(shape, value, path) {
                    return Err(failures);
                }
                let (serialized, conversion) = match format {
                    ContentFormat::Json => (serde_json::to_string(value).map_err(|error| error.to_string()), Conversion::ToJson),
                    ContentFormat::Yaml => (
                        serde_yaml_ng::to_string(value)
                            .map(|yaml| yaml.trim_end_matches('\n').to_string())
                            .map_err(|error| error.to_string()),
                        Conversion::ToYaml,
                    ),
                };
                let serialized = serialized
                    .map_err(|detail| fail(shape, value, path, None, FailureReason::MalformedText { detail }))?;
                Ok(converted(Value::String(serialized), path, conversion))
            }
        }
    }

    fn array(&self, shape: &Shape, array: &ArrayShape, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        let Value::Array(items) = value else {
            return Err(fail(shape, value, path, None, FailureReason::NoConversion));
        };
        let mut failures = Vec::new();
        if let Some(constraint) = array_count_constraint(array, items) {
            failures.extend(fail(shape, value, path, None, FailureReason::ConstraintNotMet(constraint)));
        }
        let mut changes = Changes::default();
        for (index, item) in items.iter().enumerate() {
            changes.record(index, self.walk(&array.items, item, path.child(Segment::Index(index))), &mut failures);
        }
        if !failures.is_empty() {
            return Err(failures);
        }
        let step = changes.apply_to_list(items);
        if array.unique_items {
            let final_items = match &step {
                Step::Converted(Value::Array(converted), _) => converted,
                _ => items,
            };
            if !all_unique(final_items) {
                let converted = matches!(step, Step::Converted(..)).then(|| Value::Array(final_items.clone()));
                return Err(fail(shape, value, path, converted, FailureReason::ConstraintNotMet(Constraint::UniqueItems)));
            }
        }
        Ok(step)
    }

    fn tuple(&self, shape: &Shape, tuple: &TupleShape, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        let Value::Array(items) = value else {
            return Err(fail(shape, value, path, None, FailureReason::NoConversion));
        };
        let mut failures = Vec::new();
        let mut changes = Changes::default();
        for (index, item) in items.iter().enumerate() {
            let child = path.child(Segment::Index(index));
            match tuple_slot(tuple, index) {
                Some(slot) => changes.record(index, self.walk(slot, item, child), &mut failures),
                None => failures.push(ValueFailure {
                    path: child.to_vec(),
                    expected: shape.describe(),
                    received: Some(item.clone()),
                    converted: None,
                    reason: FailureReason::UnexpectedElement,
                }),
            }
        }
        for (index, slot) in tuple.prefix.iter().enumerate().skip(items.len()) {
            if !slot.optional {
                failures.push(missing(&slot.shape, path.child(Segment::Index(index))));
            }
        }
        if failures.is_empty() {
            Ok(changes.apply_to_list(items))
        } else {
            Err(failures)
        }
    }

    fn object(&self, shape: &Shape, object: &ObjectShape, value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        let Value::Object(map) = value else {
            return Err(fail(shape, value, path, None, FailureReason::NoConversion));
        };
        let mut failures = Vec::new();
        if let Some(constraint) = object_count_constraint(object, map) {
            failures.extend(fail(shape, value, path, None, FailureReason::ConstraintNotMet(constraint)));
        }
        let mut converted_values: Vec<(&str, Value)> = Vec::new();
        let mut conversions = Vec::new();
        for (key, item) in map {
            let child = path.child(Segment::Key(key));
            let outcome = match declarations(object, key) {
                Declarations::None => match &object.additional {
                    Additional::Any => Ok(Step::Kept),
                    Additional::Forbidden => Err(vec![ValueFailure {
                        path: child.to_vec(),
                        expected: shape.describe(),
                        received: Some(item.clone()),
                        converted: None,
                        reason: FailureReason::UnexpectedProperty,
                    }]),
                    Additional::Shape(additional) => self.walk(additional, item, child),
                },
                Declarations::Some(shapes) if shapes.len() == 1 => self.walk(shapes[0], item, child),
                Declarations::Some(shapes) => self.all_of(&shapes, item, child),
            };
            match outcome {
                Ok(Step::Kept) => {}
                Ok(Step::Converted(converted, records)) => {
                    converted_values.push((key, converted));
                    conversions.extend(records);
                }
                Err(more) => failures.extend(more),
            }
        }
        for property in missing_required(object, map) {
            failures.push(missing(&property.shape, path.child(Segment::Key(&property.name))));
        }
        if !failures.is_empty() {
            return Err(failures);
        }
        if converted_values.is_empty() {
            return Ok(Step::Kept);
        }
        let mut copy = map.clone();
        for (key, converted) in converted_values {
            if let Some(slot) = copy.get_mut(key) {
                *slot = converted;
            }
        }
        Ok(Step::Converted(Value::Object(copy), conversions))
    }
}

/// Converted children of a list, applied to a copy only when one exists.
#[derive(Default)]
struct Changes {
    values: Vec<(usize, Value)>,
    conversions: Conversions,
}

impl Changes {
    fn record(&mut self, index: usize, outcome: Result<Step, Failures>, failures: &mut Failures) {
        match outcome {
            Ok(Step::Kept) => {}
            Ok(Step::Converted(value, records)) => {
                self.values.push((index, value));
                self.conversions.extend(records);
            }
            Err(more) => failures.extend(more),
        }
    }

    fn apply_to_list(self, items: &[Value]) -> Step {
        if self.values.is_empty() {
            return Step::Kept;
        }
        let mut copy = items.to_vec();
        for (index, value) in self.values {
            copy[index] = value;
        }
        Step::Converted(Value::Array(copy), self.conversions)
    }
}

/// The stored form of an admitted number. Admission yields only finite
/// values, for which `json_number` always returns a number.
fn stored_number(admitted: f64) -> Value {
    json_number(admitted).map_or(Value::Null, Value::Number)
}

fn converted(value: Value, path: Path<'_>, conversion: Conversion) -> Step {
    Step::Converted(value, vec![(path.to_vec(), conversion)])
}

pub(super) fn fail(
    shape: &Shape,
    value: &Value,
    path: Path<'_>,
    converted: Option<Value>,
    reason: FailureReason,
) -> Failures {
    vec![ValueFailure {
        path: path.to_vec(),
        expected: shape.describe(),
        received: Some(value.clone()),
        converted,
        reason,
    }]
}

fn missing(shape: &Shape, path: Path<'_>) -> ValueFailure {
    ValueFailure {
        path: path.to_vec(),
        expected: shape.describe(),
        received: None,
        converted: None,
        reason: FailureReason::MissingRequired,
    }
}

fn text_reason(error: NumberTextError) -> FailureReason {
    match error {
        NumberTextError::NotANumber => FailureReason::MalformedText {
            detail: "not a number".to_string(),
        },
        NumberTextError::Admission(reason) => FailureReason::NumberNotAdmitted(reason),
    }
}

/// A native number, or (for `numberlike`) number text, checked against the
/// target's bounds without conversion.
fn exact_number(number: &NumberShape, value: &Value) -> Result<(), FailureReason> {
    let admitted = match value {
        Value::Number(native) => admit_json_number(native).map_err(FailureReason::NumberNotAdmitted)?,
        Value::String(text) if number.as_sent => admit_number_text(text).map_err(text_reason)?,
        _ => return Err(FailureReason::NoConversion),
    };
    match bounds_constraint(&number.bounds, admitted) {
        None => Ok(()),
        Some(constraint) => Err(FailureReason::ConstraintNotMet(constraint)),
    }
}

fn bounds_constraint(bounds: &NumericBounds, value: f64) -> Option<Constraint> {
    if let Some(rule) = bounds.integer {
        let whole = value.fract() == 0.0;
        let in_range = rule == IntegerRule::Whole || value.abs() < 2f64.powi(64);
        if !whole || !in_range {
            return Some(Constraint::Integer(rule));
        }
    }
    if let Some(minimum) = bounds.minimum.filter(|minimum| value < *minimum) {
        return Some(Constraint::Minimum(minimum));
    }
    if let Some(maximum) = bounds.maximum.filter(|maximum| value > *maximum) {
        return Some(Constraint::Maximum(maximum));
    }
    if let Some(minimum) = bounds.exclusive_minimum.filter(|minimum| value <= *minimum) {
        return Some(Constraint::ExclusiveMinimum(minimum));
    }
    if let Some(maximum) = bounds.exclusive_maximum.filter(|maximum| value >= *maximum) {
        return Some(Constraint::ExclusiveMaximum(maximum));
    }
    if let Some(step) = bounds.multiple_of.filter(|step| *step > 0.0 && (value / step).fract() != 0.0) {
        return Some(Constraint::MultipleOf(step));
    }
    None
}

fn text_constraint(shape: &TextShape, text: &str) -> Option<Constraint> {
    if let Some(members) = shape.members.as_ref().filter(|members| !members.iter().any(|member| member == text)) {
        return Some(Constraint::Members(members.clone()));
    }
    if shape.min_length.is_some() || shape.max_length.is_some() {
        let length = text.chars().count();
        if let Some(minimum) = shape.min_length.filter(|minimum| length < *minimum) {
            return Some(Constraint::MinLength(minimum));
        }
        if let Some(maximum) = shape.max_length.filter(|maximum| length > *maximum) {
            return Some(Constraint::MaxLength(maximum));
        }
    }
    if let Some(pattern) = shape.pattern.as_ref().filter(|pattern| !pattern.is_match(text)) {
        return Some(Constraint::Pattern(pattern.source().to_string()));
    }
    None
}

fn array_count_constraint(array: &ArrayShape, items: &[Value]) -> Option<Constraint> {
    if let Some(minimum) = array.min_items.filter(|minimum| items.len() < *minimum) {
        return Some(Constraint::MinItems(minimum));
    }
    if let Some(maximum) = array.max_items.filter(|maximum| items.len() > *maximum) {
        return Some(Constraint::MaxItems(maximum));
    }
    None
}

fn array_constraint(array: &ArrayShape, items: &[Value]) -> Option<Constraint> {
    array_count_constraint(array, items).or_else(|| (array.unique_items && !all_unique(items)).then_some(Constraint::UniqueItems))
}

fn all_unique(items: &[Value]) -> bool {
    items
        .iter()
        .enumerate()
        .all(|(index, item)| items[..index].iter().all(|earlier| !same_value(earlier, item)))
}

fn required_slots(tuple: &TupleShape) -> usize {
    tuple.prefix.iter().filter(|slot| !slot.optional).count()
}

fn tuple_slot(tuple: &TupleShape, index: usize) -> Option<&Shape> {
    tuple
        .prefix
        .get(index)
        .map(|slot| &slot.shape)
        .or(tuple.rest.as_deref().filter(|_| index >= tuple.prefix.len()))
}

fn object_count_constraint(object: &ObjectShape, map: &Map<String, Value>) -> Option<Constraint> {
    if let Some(minimum) = object.min_properties.filter(|minimum| map.len() < *minimum) {
        return Some(Constraint::MinProperties(minimum));
    }
    if let Some(maximum) = object.max_properties.filter(|maximum| map.len() > *maximum) {
        return Some(Constraint::MaxProperties(maximum));
    }
    None
}

fn missing_required<'s>(
    object: &'s ObjectShape,
    map: &'s Map<String, Value>,
) -> impl Iterator<Item = &'s super::shape::PropertySlot> {
    object
        .properties
        .iter()
        .filter(move |property| property.required && !map.contains_key(&property.name))
}

/// The declarations that apply to one key.
enum Declarations<'s> {
    /// The key falls to [`ObjectShape::additional`].
    None,
    Some(Vec<&'s Shape>),
}

fn declarations<'s>(object: &'s ObjectShape, key: &str) -> Declarations<'s> {
    let mut shapes: Vec<&Shape> = object
        .properties
        .iter()
        .filter(|property| property.name == key)
        .map(|property| &property.shape)
        .collect();
    if shapes.is_empty() || object.pattern_scope == PatternScope::AllKeys {
        shapes.extend(
            object
                .patterns
                .iter()
                .filter(|pattern| pattern.matcher.matches(key))
                .map(|pattern| &pattern.shape),
        );
    }
    if shapes.is_empty() {
        Declarations::None
    } else {
        Declarations::Some(shapes)
    }
}

/// JSON-value equality with numbers compared by value, so `1` equals `1.0`.
pub(super) fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => match (left.as_i64(), right.as_i64()) {
            (Some(left), Some(right)) => left == right,
            _ => match (left.as_u64(), right.as_u64()) {
                (Some(left), Some(right)) => left == right,
                _ => left.as_f64() == right.as_f64(),
            },
        },
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(left, right)| same_value(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, value)| right.get(key).is_some_and(|other| same_value(value, other)))
        }
        _ => left == right,
    }
}

/// Why `text` is not valid content of `format`; `None` when it is.
///
/// JSON is strict: duplicate keys and numbers outside `f64` (`1e400`) are
/// rejected. YAML parses through `serde_yaml_ng::Value`, which rejects
/// duplicate keys.
pub fn content_error(format: ContentFormat, text: &str) -> Option<String> {
    match format {
        ContentFormat::Json => {
            let mut deserializer = serde_json::Deserializer::from_str(text);
            StrictJson::deserialize(&mut deserializer)
                .and_then(|_| deserializer.end())
                .err()
                .map(|error| format!("invalid JSON: {error}"))
        }
        ContentFormat::Yaml => serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text)
            .err()
            .map(|error| format!("invalid YAML: {error}")),
    }
}

/// Accepts any JSON value whose objects have no duplicate keys.
struct StrictJson;

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictJson)
    }
}

impl<'de> Visitor<'de> for StrictJson {
    type Value = Self;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self, A::Error> {
        while access.next_element::<StrictJson>()?.is_some() {}
        Ok(Self)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self, A::Error> {
        let mut seen = HashSet::new();
        while let Some(key) = access.next_key::<String>()? {
            if seen.contains(&key) {
                return Err(de::Error::custom(format!("duplicate key {key:?}")));
            }
            access.next_value::<StrictJson>()?;
            seen.insert(key);
        }
        Ok(Self)
    }
}
