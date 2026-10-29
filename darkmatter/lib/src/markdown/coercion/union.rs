//! Choosing among several shapes: union options, and overlapping object
//! declarations that must all hold.
//!
//! Every candidate is bound from the original value independently, so no
//! candidate's conversion feeds another and the written order of options or
//! declarations never changes a result.

use serde_json::Value;

use super::engine::{Conversion, Conversions, Engine, Failures, Path, Step, fail, same_value};
use super::error::{FailureReason, OptionFailure, PathSegment};
use super::shape::Shape;

/// A distinct converted result and the conversions that produced it.
type Reading = (Value, Conversions);

impl Engine<'_> {
    /// Exact match, else the one reachable option, else a tie-breaker, else
    /// an error naming every reading.
    pub(super) fn union(&self, shape: &Shape, options: &[Shape], value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        if options.iter().any(|option| self.exact(option, value, path)) {
            return Ok(Step::Kept);
        }
        let mut readings: Vec<Reading> = Vec::new();
        let mut option_failures = Vec::new();
        for option in options {
            match self.walk(option, value, path) {
                Ok(Step::Kept) => return Ok(Step::Kept),
                Ok(Step::Converted(converted, conversions)) => {
                    if !readings.iter().any(|(reading, _)| same_value(reading, &converted)) {
                        readings.push((converted, conversions));
                    }
                }
                Err(failures) => option_failures.push(OptionFailure {
                    expected: option.describe(),
                    failures,
                }),
            }
        }
        match readings.len() {
            0 => Err(fail(
                shape,
                value,
                path,
                None,
                FailureReason::NoOptionAccepts {
                    options: option_failures,
                },
            )),
            1 => {
                let (converted, conversions) = readings.remove(0);
                Ok(Step::Converted(converted, conversions))
            }
            _ => match tie_break(&readings, &path.to_vec()) {
                Some(winner) => {
                    let (converted, conversions) = readings.swap_remove(winner);
                    Ok(Step::Converted(converted, conversions))
                }
                None => {
                    let readings = readings.into_iter().map(|(reading, _)| reading).collect();
                    Err(fail(shape, value, path, None, FailureReason::AmbiguousUnion { readings }))
                }
            },
        }
    }

    /// A key under several declarations (a property and matching patterns,
    /// under [`super::PatternScope::AllKeys`]): keep the value when every
    /// declaration accepts it; otherwise commit the one converted reading
    /// that every declaration accepts exactly.
    pub(super) fn all_of(&self, declarations: &[&Shape], value: &Value, path: Path<'_>) -> Result<Step, Failures> {
        if declarations.iter().all(|shape| self.exact(shape, value, path)) {
            return Ok(Step::Kept);
        }
        let mut readings: Vec<Reading> = Vec::new();
        let mut failures = Vec::new();
        for shape in declarations {
            match self.walk(shape, value, path) {
                Ok(Step::Kept) => {}
                Ok(Step::Converted(converted, conversions)) => {
                    if !readings.iter().any(|(reading, _)| same_value(reading, &converted)) {
                        readings.push((converted, conversions));
                    }
                }
                Err(more) => failures.extend(more),
            }
        }
        if !self.convert {
            return Err(failures);
        }
        let mut accepted: Vec<Reading> = Vec::new();
        let mut rejected: Vec<Value> = Vec::new();
        for (reading, conversions) in readings {
            if declarations.iter().all(|shape| self.exact(shape, &reading, path)) {
                accepted.push((reading, conversions));
            } else {
                rejected.push(reading);
            }
        }
        if accepted.len() == 1 {
            let (converted, conversions) = accepted.remove(0);
            return Ok(Step::Converted(converted, conversions));
        }
        if accepted.is_empty() && !failures.is_empty() {
            return Err(failures);
        }
        let readings = accepted.into_iter().map(|(reading, _)| reading).chain(rejected).collect();
        let description = declarations.iter().map(|shape| shape.describe()).collect::<Vec<_>>().join(" & ");
        Err(vec![super::error::ValueFailure {
            path: path.to_vec(),
            expected: description,
            received: Some(value.clone()),
            converted: None,
            reason: FailureReason::ConflictingDeclarations { readings },
        }])
    }
}

/// The reading the tie-breakers favor over every other one, if any.
///
/// Readings are compared at every path where their values differ; a reading
/// beats another only when, at each such path, it holds the favored
/// conversion of the same original value: number over boolean, JSON over
/// YAML. At most one reading can beat all others.
fn tie_break(readings: &[Reading], base: &[PathSegment]) -> Option<usize> {
    (0..readings.len()).find(|&candidate| {
        readings
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != candidate)
            .all(|(_, other)| beats(&readings[candidate], other, base))
    })
}

fn beats(winner: &Reading, loser: &Reading, base: &[PathSegment]) -> bool {
    let mut differences = Vec::new();
    differing_paths(&winner.0, &loser.0, &mut base.to_vec(), &mut differences);
    !differences.is_empty()
        && differences.iter().all(|path| {
            matches!(
                (conversion_at(&winner.1, path), conversion_at(&loser.1, path)),
                (Some(Conversion::TextToNumber), Some(Conversion::TextToBoolean))
                    | (Some(Conversion::ToJson), Some(Conversion::ToYaml))
            )
        })
}

fn conversion_at(conversions: &Conversions, path: &[PathSegment]) -> Option<Conversion> {
    conversions
        .iter()
        .find(|(converted_at, _)| converted_at == path)
        .map(|(_, conversion)| *conversion)
}

/// The deepest paths at which two readings differ, descending through lists
/// of equal length and objects with the same keys.
fn differing_paths(left: &Value, right: &Value, path: &mut Vec<PathSegment>, out: &mut Vec<Vec<PathSegment>>) {
    match (left, right) {
        (Value::Array(left_items), Value::Array(right_items)) if left_items.len() == right_items.len() => {
            for (index, (left_item, right_item)) in left_items.iter().zip(right_items).enumerate() {
                path.push(PathSegment::Index(index));
                differing_paths(left_item, right_item, path, out);
                path.pop();
            }
        }
        (Value::Object(left_map), Value::Object(right_map))
            if left_map.len() == right_map.len() && left_map.keys().all(|key| right_map.contains_key(key)) =>
        {
            for (key, left_item) in left_map {
                path.push(PathSegment::Key(key.clone()));
                differing_paths(left_item, &right_map[key], path, out);
                path.pop();
            }
        }
        _ if same_value(left, right) => {}
        _ => out.push(path.clone()),
    }
}
