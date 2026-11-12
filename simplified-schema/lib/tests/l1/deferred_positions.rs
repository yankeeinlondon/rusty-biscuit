//! Deferred position maps: a map made by `PositionMap::deferred` (the one
//! envelope admission returns) builds nothing until a coordinate is
//! requested, builds once however many clones or threads ask, and reports
//! exactly the coordinates the eager `build_position_map` reports over the
//! same text: 1-based lines, UTF-8 byte columns, and byte ranges of the text
//! as given (CRLF and a byte-order mark kept).

use std::sync::Arc;

use biscuit_file::serde_yaml_ng;
use serde_json::Value;
use simplified_schema::envelope::admit_envelope_text;
use simplified_schema::positions::build_position_map;
use simplified_schema::{JsonPointer, PositionMap, SimplifiedSchema, ValidationFailure, ValidationProblem};

const VALID_ENVELOPE: &str = "kind: schema\ndescription: Reviews\n$schema:\n  title: string\n";

const _: () = {
    const fn shareable<T: Send + Sync + Clone>() {}
    shareable::<PositionMap>();
};

/// Strict-validation problems of `yaml` against the `$schema` body `schema`,
/// not yet located.
#[track_caller]
fn unlocated_problems(schema: &str, yaml: &str) -> Vec<ValidationProblem> {
    let schema = SimplifiedSchema::try_from(format!("$schema:\n{schema}").as_str()).expect("schema parses");
    let value: Value = serde_yaml_ng::from_str(yaml.trim_start_matches('\u{feff}')).expect("data parses");
    match schema.validate_strict(&value) {
        Err(ValidationFailure::InvalidData { problems }) => problems,
        other => panic!("expected invalid data, got {other:?}"),
    }
}

/// `(pointer, line, column, byte range)` of one located problem.
type Row = (String, Option<u32>, Option<u32>, Option<std::ops::Range<usize>>);

/// The [`Row`] of each problem located by `map`.
fn located_rows(map: &PositionMap, problems: &[ValidationProblem]) -> Vec<Row> {
    let mut problems = problems.to_vec();
    map.locate_all(&mut problems);
    problems
        .iter()
        .map(|problem| (problem.path.clone(), problem.line, problem.column, map.byte_range(problem)))
        .collect()
}

/// A problem at `pointer`, for queries that need no validation.
fn problem_at(pointer: &str) -> ValidationProblem {
    let mut problem = unlocated_problems("  v: number\n", "v: x\n").remove(0);
    problem.path = pointer.to_string();
    problem.instance_path = JsonPointer::parse(pointer);
    problem
}

#[test]
fn deferred_and_eager_maps_agree_on_authored_bytes() {
    let cells: [(&str, &str, &str); 7] = [
        ("lf", "  a: number\n  b: number\n", "a: 1\nb: x\n"),
        ("crlf", "  a: number\n  b: number\n", "a: 1\r\nb: x\r\n"),
        ("bom", "  a: number\n  b: number\n", "\u{feff}a: x\nb: y\n"),
        ("bom + crlf", "  a: number\n  b: number\n", "\u{feff}a: x\r\nb: y\r\n"),
        ("multi-byte columns", "  m: '{ note: string, port: number }'\n", "m: {note: é漢, port: x}\n"),
        ("alias", "  base: '{ port: number }'\n  one: '{ port: number }'\n", "base: &b {port: x}\none: *b\n"),
        ("missing and unknown keys", "  m: '{ r: string(required) }'\n", "m:\n  q: 1\n"),
    ];
    for (label, schema, yaml) in cells {
        let problems = unlocated_problems(schema, yaml);
        assert!(!problems.is_empty(), "{label}");

        let eager = located_rows(&build_position_map(yaml), &problems);
        let deferred = located_rows(&PositionMap::deferred(Arc::from(yaml)), &problems);

        assert_eq!(deferred, eager, "{label}");
        assert!(eager.iter().any(|row| row.1.is_some()), "{label}: something is located: {eager:?}");
    }
}

#[test]
fn byte_columns_and_crlf_lines_address_the_original_text() {
    let yaml = "m: {note: é漢, port: x}\r\nn: 1\r\n";
    let problems = unlocated_problems("  m: '{ note: string, port: number }'\n  n: string\n", yaml);
    let map = PositionMap::deferred(Arc::from(yaml));

    let rows = located_rows(&map, &problems);

    let port = rows.iter().find(|row| row.0 == "/m/port").expect("port problem");
    // `é` is two bytes and `漢` three: byte column 24, character column 21.
    assert_eq!((port.1, port.2), (Some(1), Some(24)), "{rows:?}");
    assert_eq!(port.3.clone().map(|range| &yaml[range]), Some("x"));
    let n = rows.iter().find(|row| row.0 == "/n").expect("n problem");
    assert_eq!((n.1, n.2), (Some(2), Some(4)), "{rows:?}");
}

#[test]
fn an_admitted_envelope_locates_like_an_eager_map() {
    let admission = admit_envelope_text(VALID_ENVELOPE, "subject.yaml").expect("YAML").expect("recognized");
    assert!(admission.is_admitted());
    let eager = build_position_map(VALID_ENVELOPE);

    for pointer in ["/$schema/title", "/description", "/kind"] {
        let problem = problem_at(pointer);
        assert_eq!(admission.positions.byte_range(&problem), eager.byte_range(&problem), "{pointer}");
        assert!(admission.positions.byte_range(&problem).is_some(), "{pointer}");
    }
}

#[test]
fn a_rejected_envelope_is_located_like_an_eager_map() {
    for text in [
        "kind: schema\r\ndescriptoin: Reviews\r\n$schema:\r\n  title: string\r\n",
        "kind: schema\ndescriptoin: Réviews\n$schema:\n  title: string\n  kind: unknownish\n",
    ] {
        let admission = admit_envelope_text(text, "subject.yaml").expect("YAML").expect("recognized");
        assert!(!admission.is_admitted(), "{text:?}");

        let mut relocated: Vec<ValidationProblem> = admission.problems.clone();
        for problem in &mut relocated {
            (problem.line, problem.column) = (None, None);
        }
        let eager = build_position_map(text);
        eager.locate_all(&mut relocated);

        let coordinates = |problems: &[ValidationProblem]| -> Vec<_> {
            problems.iter().map(|problem| (problem.path.clone(), problem.line, problem.column)).collect()
        };
        assert_eq!(coordinates(&relocated), coordinates(&admission.problems), "{text:?}");
        assert!(admission.problems.iter().all(|problem| problem.line.is_some()), "{:?}", admission.problems);
        let ranges: Vec<_> = admission.problems.iter().map(|problem| admission.positions.byte_range(problem)).collect();
        let eager_ranges: Vec<_> = admission.problems.iter().map(|problem| eager.byte_range(problem)).collect();
        assert_eq!(ranges, eager_ranges, "{text:?}");
    }
}

#[cfg(feature = "work-counters")]
mod counted {
    use super::*;
    use simplified_schema::work_counters::{Work, snapshot};

    fn constructions_since(before: &simplified_schema::work_counters::WorkSnapshot) -> u64 {
        snapshot().since(before).get(Work::PositionMapConstruction)
    }

    #[test]
    fn a_valid_envelope_builds_no_map_until_a_coordinate_is_requested() {
        let before = snapshot();

        let admission = admit_envelope_text(VALID_ENVELOPE, "subject.yaml").expect("YAML").expect("recognized");
        admission.positions.locate_all(&mut []);
        let _ = format!("{admission:?}");

        assert!(admission.is_admitted());
        assert_eq!(constructions_since(&before), 0, "admission and debug output build no map");

        assert!(!admission.positions.is_empty());
        assert!(admission.positions.byte_range(&problem_at("/$schema/title")).is_some());
        assert_eq!(constructions_since(&before), 1, "the first query builds the map once");
    }

    #[test]
    fn an_eager_map_counts_once_at_construction() {
        let before = snapshot();

        let map = build_position_map(VALID_ENVELOPE);
        let _ = map.byte_range(&problem_at("/kind"));

        assert_eq!(constructions_since(&before), 1);
    }

    #[test]
    fn clones_share_one_construction() {
        let before = snapshot();
        let original = PositionMap::deferred(Arc::from(VALID_ENVELOPE));
        let early_clone = original.clone();

        let from_clone = early_clone.byte_range(&problem_at("/description"));
        let late_clone = original.clone();
        let from_original = original.byte_range(&problem_at("/description"));
        let from_late = late_clone.byte_range(&problem_at("/description"));

        assert_eq!(constructions_since(&before), 1);
        assert!(from_clone.is_some());
        assert_eq!(from_original, from_clone);
        assert_eq!(from_late, from_clone);
    }

    #[test]
    fn concurrent_lookups_build_once() {
        let before = snapshot();
        let map = PositionMap::deferred(Arc::from(VALID_ENVELOPE));
        let problem = problem_at("/$schema/title");

        let ranges: Vec<_> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4).map(|_| scope.spawn(|| map.clone().byte_range(&problem))).collect();
            handles.into_iter().map(|handle| handle.join().expect("lookup thread")).collect()
        });

        assert_eq!(constructions_since(&before), 1);
        assert!(ranges.iter().all(|range| range.is_some() && *range == ranges[0]), "{ranges:?}");
    }
}
