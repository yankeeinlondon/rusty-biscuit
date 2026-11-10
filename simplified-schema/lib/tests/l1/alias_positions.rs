//! Authored positions for YAML containing aliases (`*name`), through the
//! public API: a problem at an alias, or anywhere in the value it expands
//! to, is located at the alias token, and its neighbors keep their own
//! positions. A layout the projection cannot prove keeps the top-level
//! fallback.

use biscuit_file::serde_yaml_ng;
use serde_json::Value;
use simplified_schema::positions::build_position_map;
use simplified_schema::{MappingOrders, PositionMap, SimplifiedSchema, ValidationFailure, ValidationProblem};

/// The problems of validating `yaml` strictly against the `$schema` body
/// `schema`, located by the map built from `yaml`.
#[track_caller]
fn located(schema: &str, yaml: &str) -> (PositionMap, Vec<ValidationProblem>) {
    let schema = SimplifiedSchema::try_from(format!("$schema:\n{schema}").as_str()).expect("schema parses");
    let value: Value = serde_yaml_ng::from_str(yaml).expect("data parses");
    let mut problems = match schema.validate_strict(&value) {
        Err(ValidationFailure::InvalidData { problems }) => problems,
        other => panic!("expected invalid data, got {other:?}"),
    };
    let map = build_position_map(yaml);
    map.locate_all(&mut problems);
    (map, problems)
}

/// `(pointer, line, column, range text)` of each problem, in report order.
fn rows<'a>(map: &PositionMap, yaml: &'a str, problems: &[ValidationProblem]) -> Vec<(String, u32, u32, Option<&'a str>)> {
    let mut rows: Vec<_> = problems
        .iter()
        .map(|problem| {
            (
                problem.path.clone(),
                problem.line.expect("located line"),
                problem.column.expect("located column"),
                map.byte_range(problem).map(|range| &yaml[range]),
            )
        })
        .collect();
    rows.sort();
    rows
}

fn row<'a>(pointer: &str, line: u32, column: u32, text: &'a str) -> (String, u32, u32, Option<&'a str>) {
    (pointer.to_string(), line, column, Some(text))
}

#[test]
fn a_document_without_an_alias_is_located_as_before() {
    let yaml = "base: {port: x}\nservers:\n  - {port: 1}\n  - {port: y}\n";
    let (map, problems) = located("  base: '{ port: number }'\n  servers: '{ port: number }[]'\n", yaml);
    assert_eq!(rows(&map, yaml, &problems), [row("/base/port", 1, 14, "x"), row("/servers/1/port", 4, 12, "y")]);
}

#[test]
fn an_unrelated_alias_keeps_the_failing_value_located() {
    let yaml = "base: &b {port: 1}\nservers:\n  - *b\n  - {port: x}\n";
    let (map, problems) = located("  base: '{ port: number }'\n  servers: '{ port: number }[]'\n", yaml);
    assert_eq!(rows(&map, yaml, &problems), [row("/servers/1/port", 4, 12, "x")]);
}

#[test]
fn every_use_of_a_failing_anchor_points_at_its_alias_token() {
    let yaml = "base: &b {port: x}\none: *b\ntwo: *b\n";
    let schema = "  base: '{ port: number }'\n  one: '{ port: number }'\n  two: '{ port: number }'\n";
    let (map, problems) = located(schema, yaml);
    assert_eq!(
        rows(&map, yaml, &problems),
        [row("/base/port", 1, 17, "x"), row("/one/port", 2, 6, "*b"), row("/two/port", 3, 6, "*b")]
    );
    let mut starts: Vec<_> = problems.iter().filter_map(|problem| map.byte_range(problem)).map(|r| r.start).collect();
    starts.sort();
    assert_eq!(starts, [16, 24, 32]);
}

#[test]
fn aliases_to_scalars_sequences_and_mappings_keep_neighbor_positions() {
    let yaml = "a: &s x\nb: *s\nc: &q [x]\nd: *q\ne: &m {p: x}\nf: *m\ng: x\n";
    let schema = "  a: string\n  b: number\n  c: 'string[]'\n  d: 'number[]'\n  e: '{ p: string }'\n  f: '{ p: number }'\n  g: number\n";
    let (map, problems) = located(schema, yaml);
    assert_eq!(
        rows(&map, yaml, &problems),
        [row("/b", 2, 4, "*s"), row("/d/0", 4, 4, "*q"), row("/f/p", 6, 4, "*m"), row("/g", 7, 4, "x")]
    );
}

#[test]
fn aliases_under_a_sequence_root_are_projected() {
    for (yaml, pointer, column, token) in [
        ("- &a x\n- *a\n", "/1", 3, "*a"),
        ("- base: &b\n    x: 1\n- *b\n", "/1/base/x", 3, "*b"),
    ] {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let map = PositionMap::project(yaml, 0..yaml.len(), 1, &MappingOrders::collect(&value)).expect("projects");
        let problem = problem_at(pointer);
        assert_eq!(map.byte_range(&problem).map(|range| &yaml[range]), Some(token), "{yaml:?}");
        let mut problems = [problem];
        map.locate_all(&mut problems);
        assert_eq!((problems[0].line, problems[0].column), (Some(yaml.lines().count() as u32), Some(column)));
    }
}

/// A type-mismatch problem at `pointer`, taken from a real validation so
/// every field is the validator's own.
fn problem_at(pointer: &str) -> ValidationProblem {
    let schema = SimplifiedSchema::try_from("$schema:\n  v: number\n").unwrap();
    let mut problem = schema.validate_strict(&serde_json::json!({ "v": "x" })).unwrap_err().problems()[0].clone();
    problem.path = pointer.to_string();
    problem.instance_path = simplified_schema::JsonPointer::parse(pointer);
    problem
}

#[test]
fn aliases_in_flow_collections_and_repeated_uses_are_located() {
    let yaml = "base: &b {port: x}\nlist: [*b, *b]\nmap: {k: *b, j: *b}\n";
    let schema = "  base: '{ port: string }'\n  list: '{ port: number }[]'\n  map: '{ k: { port: number }, j: { port: number } }'\n";
    let (map, problems) = located(schema, yaml);
    assert_eq!(
        rows(&map, yaml, &problems),
        [
            row("/list/0/port", 2, 8, "*b"),
            row("/list/1/port", 2, 12, "*b"),
            row("/map/j/port", 3, 17, "*b"),
            row("/map/k/port", 3, 10, "*b"),
        ]
    );
}

#[test]
fn a_missing_or_unknown_child_under_an_alias_points_at_the_alias() {
    let yaml = "base: &b {port: x, host: y}\none: *b\n";
    let schema = "  base: '{ port: string, host: string }'\n  one: '{ port: number, host: number }'\n";
    let (map, problems) = located(schema, yaml);
    assert_eq!(
        rows(&map, yaml, &problems),
        [row("/one/host", 2, 6, "*b"), row("/one/port", 2, 6, "*b")],
        "two failing children share the alias range with distinct pointers"
    );

    let schema = "  base: '{ port: string, host: string }'\n  one: '{ port: string, host: string, extra: number(required) }'\n";
    let (map, problems) = located(schema, yaml);
    let [missing] = problems.as_slice() else { panic!("one missing-key problem: {problems:?}") };
    assert_eq!((missing.path.as_str(), missing.property.as_deref()), ("/one", Some("extra")));
    assert_eq!(map.byte_range(missing).map(|r| &yaml[r]), Some("*b"));
    assert_eq!((missing.line, missing.column), (Some(2), Some(6)));

    let schema = "  base: '{ port: string, host: string }'\n  one: '{ port: string }'\n";
    let (map, problems) = located(schema, yaml);
    let [unknown] = problems.as_slice() else { panic!("one unknown-key problem: {problems:?}") };
    assert_eq!(unknown.path, "/one");
    assert_eq!(unknown.offending_property.as_deref(), Some("host"));
    assert_eq!(map.byte_range(unknown).map(|r| &yaml[r]), Some("*b"));
    assert_eq!((unknown.line, unknown.column), (Some(2), Some(6)));
}

#[test]
fn keys_with_slash_and_tilde_resolve_below_an_alias() {
    let yaml = "\"a/b\": &x {p: 1}\n\"a~b\": {q: x}\nc: *x\n";
    let schema = "  a/b: '{ p: number }'\n  a~b: '{ q: number }'\n  c: '{ p: string }'\n";
    let (map, problems) = located(schema, yaml);
    assert_eq!(rows(&map, yaml, &problems), [row("/a~0b/q", 2, 12, "x"), row("/c/p", 3, 4, "*x")]);
}

/// `<<` is an ordinary key: nothing is inherited, an explicit sibling
/// locates at itself, and a path below the merged alias points at its token.
#[test]
fn literal_merge_keys_are_ordinary_keys_located_at_their_alias() {
    for (yaml, merge_column, port_column) in [
        ("base: &b {port: x}\nserver: {<<: *b, port: 2}\n", 14, 24),
        ("base: &b {port: x}\nserver: {\"<<\": *b, port: 2}\n", 16, 26),
    ] {
        let schema = "  base: '{ port: number }'\n  server: '{ <string>: { port: number } }'\n";
        let (map, problems) = located(schema, yaml);
        assert_eq!(
            rows(&map, yaml, &problems),
            [row("/base/port", 1, 17, "x"), row("/server/<</port", 2, merge_column, "*b"), row("/server/port", 2, port_column, "2")],
            "{yaml:?}"
        );
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(MappingOrders::collect(&value).get("/server"), Some(["<<".to_string(), "port".to_string()].as_slice()));
    }

    let yaml = "a: &a {port: x}\nb: &b {port: y}\nserver:\n  <<: [*a, *b]\n";
    let schema = "  a: '{ port: string }'\n  b: '{ port: string }'\n  server: '{ <string>: { port: number }[] }'\n";
    let (map, problems) = located(schema, yaml);
    assert_eq!(rows(&map, yaml, &problems), [row("/server/<</0/port", 4, 8, "*a"), row("/server/<</1/port", 4, 12, "*b")]);
}

#[test]
fn an_unproven_layout_keeps_the_top_level_fallback() {
    let yaml = "&k a: 1\nb: x\n";
    let (map, problems) = located("  a: number\n  b: number\n", yaml);
    let [problem] = problems.as_slice() else { panic!("one problem: {problems:?}") };
    assert_eq!(map.byte_range(problem), None);
    assert_eq!((problem.line, problem.column), (Some(2), Some(1)));

    // An undefined alias does not parse, so nothing beyond top-level keys is located.
    let map = build_position_map("a: *b\n");
    assert_eq!(map.byte_range(&problem_at("/a")), None);
}

#[test]
fn an_alias_boundary_does_not_hide_a_mismatch_beside_it() {
    let yaml = "a: &x {p: 1}\nab: {q: 1}\nc: *x\n";
    let own: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(PositionMap::project(yaml, 0..yaml.len(), 1, &MappingOrders::collect(&own)).is_some());

    let other_keys: serde_yaml_ng::Value = serde_yaml_ng::from_str("a: {p: 1}\nab: {r: 1}\nc: {p: 1}\n").unwrap();
    assert!(PositionMap::project(yaml, 0..yaml.len(), 1, &MappingOrders::collect(&other_keys)).is_none());

    let reordered = "a: &x {p: 1}\nab: {q: 1, r: 2}\nc: *x\n";
    let swapped: serde_yaml_ng::Value = serde_yaml_ng::from_str("a: {p: 1}\nab: {r: 2, q: 1}\nc: {p: 1}\n").unwrap();
    assert!(PositionMap::project(reordered, 0..reordered.len(), 1, &MappingOrders::collect(&swapped)).is_none());
}

#[test]
fn columns_count_bytes_and_lines_follow_crlf_and_the_base_line() {
    let yaml = "base: &b {port: x}\none: {note: é, link: *b}\n";
    let schema = "  base: '{ port: string }'\n  one: '{ note: string, link: { port: number } }'\n";
    let (map, problems) = located(schema, yaml);
    assert_eq!(rows(&map, yaml, &problems), [row("/one/link/port", 2, 23, "*b")]);

    let crlf = "base: &b {port: x}\r\none: *b\r\ntwo: *b\r\n";
    let schema = "  base: '{ port: number }'\n  one: '{ port: number }'\n  two: '{ port: number }'\n";
    let (map, problems) = located(schema, crlf);
    assert_eq!(
        rows(&map, crlf, &problems),
        [row("/base/port", 1, 17, "x"), row("/one/port", 2, 6, "*b"), row("/two/port", 3, 6, "*b")]
    );

    let document = format!("---\n{crlf}---\nbody\n");
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(crlf).unwrap();
    let map = PositionMap::project(&document, 4..4 + crlf.len(), 1, &MappingOrders::collect(&value)).expect("projects");
    let mut problems = [problem_at("/two/port")];
    map.locate_all(&mut problems);
    assert_eq!((problems[0].line, problems[0].column), (Some(4), Some(6)));
    assert_eq!(map.byte_range(&problems[0]).map(|range| &document[range]), Some("*b"));
}
