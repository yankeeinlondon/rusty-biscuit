//! The laid-out YAML writer: `to_yaml_string` and `Yaml::to_yaml_string`.
//!
//! Layout is asserted as exact text; value preservation is asserted
//! separately, by reparsing and comparing both the value and the mapping key
//! order (the backend's `Value` equality ignores order).

use std::collections::BTreeMap;
use std::num::NonZeroU8;

use biscuit_file::{IndentStyle, Indentation, Yaml, YamlError, to_yaml_string};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};

const WIDTHS: [u8; 6] = [1, 2, 4, 8, 10, 255];

fn spaces(width: u8) -> Indentation {
    Indentation::spaces(NonZeroU8::new(width).unwrap())
}

fn yaml(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap_or_else(|error| panic!("{error}:\n{text}"))
}

/// Writes `value` at `width` and checks it reparses to the backend's value in
/// the same order, then returns the text for layout assertions.
fn write<T: Serialize + ?Sized>(value: &T, width: u8) -> String {
    let out = to_yaml_string(value, spaces(width)).unwrap();
    let expected = yaml(&serde_yaml_ng::to_string(value).unwrap());
    let actual = yaml(&out);
    assert_eq!(actual, expected, "value changed at width {width}:\n{out}");
    assert_same_order(&expected, &actual);
    out
}

fn assert_same_order(expected: &Value, actual: &Value) {
    match (expected, actual) {
        (Value::Mapping(left), Value::Mapping(right)) => {
            let left_keys: Vec<_> = left.keys().collect();
            let right_keys: Vec<_> = right.keys().collect();
            assert_eq!(left_keys, right_keys, "mapping order changed");
            for (left, right) in left.values().zip(right.values()) {
                assert_same_order(left, right);
            }
        }
        (Value::Sequence(left), Value::Sequence(right)) => {
            for (left, right) in left.iter().zip(right) {
                assert_same_order(left, right);
            }
        }
        (Value::Tagged(left), Value::Tagged(right)) => {
            assert_same_order(&left.value, &right.value);
        }
        _ => {}
    }
}

/// `text` with each `>` at the start of a line's indentation standing for
/// one indentation level of `width` spaces.
fn levels(text: &str, width: u8) -> String {
    let unit = " ".repeat(usize::from(width));
    text.lines()
        .map(|line| {
            let depth = line.chars().take_while(|&c| c == '>').count();
            format!("{}{}\n", unit.repeat(depth), &line[depth..])
        })
        .collect()
}

#[test]
fn the_specification_example_indents_lists_one_level_under_their_key() {
    let value = yaml(
        "permissions:\n  file-read:\n  - ./*.txt\n  - ./*.md\n  servers:\n  - name: local\n    url: http://localhost\n",
    );
    assert_eq!(
        write(&value, 4),
        "permissions:\n    file-read:\n        - ./*.txt\n        - ./*.md\n    servers:\n        - name: local\n          url: http://localhost\n"
    );
}

#[test]
fn the_default_is_four_spaces_and_tabs_become_spaces() {
    assert_eq!(Indentation::default(), spaces(4));
    assert_eq!(Indentation::default().style, IndentStyle::Spaces);
    let value = yaml("a:\n  b:\n  - c\n");
    let tabs = Indentation::new(IndentStyle::Tabs, NonZeroU8::new(3).unwrap());
    let out = to_yaml_string(&value, tabs).unwrap();
    assert_eq!(out, "a:\n   b:\n      - c\n");
    assert!(!out.contains('\t'));
    assert_eq!(to_yaml_string(&value, Indentation::default()).unwrap(), write(&value, 4));
}

#[test]
fn nested_maps_and_scalar_lists_advance_one_level_per_key_at_every_width() {
    let value = yaml("a:\n  b:\n    c: 1\n  list:\n  - x\n  - y\nz: end\n");
    for width in WIDTHS {
        assert_eq!(
            write(&value, width),
            levels("a:\n>b:\n>>c: 1\n>list:\n>>- x\n>>- y\nz: end", width),
            "width {width}"
        );
    }
}

#[test]
fn compact_items_align_with_the_first_key_two_columns_past_the_dash_at_every_width() {
    let value = yaml(
        "servers:\n- name: local\n  url: http\n  tags:\n  - a\n- - x\n  - y\n  - - deep\n",
    );
    for width in WIDTHS {
        let unit = " ".repeat(usize::from(width));
        let expected = format!(
            "servers:\n{unit}- name: local\n{unit}  url: http\n{unit}  tags:\n{unit}  {unit}- a\n{unit}- - x\n{unit}  - y\n{unit}  - - deep\n"
        );
        assert_eq!(write(&value, width), expected, "width {width}");
    }
}

#[test]
fn a_root_sequence_starts_at_column_zero() {
    let value = yaml("- a: 1\n  b:\n    c: 2\n- [x]\n");
    assert_eq!(write(&value, 4), "- a: 1\n  b:\n      c: 2\n- - x\n");
}

#[test]
fn empty_collections_keep_their_flow_spelling() {
    let value = yaml("a: []\nb: {}\nc:\n- []\n- {}\nd: !T []\n");
    assert_eq!(write(&value, 4), "a: []\nb: {}\nc:\n    - []\n    - {}\nd: !T []\n");
    assert_eq!(write(&Vec::<u8>::new(), 4), "[]\n");
    assert_eq!(write(&BTreeMap::<String, u8>::new(), 4), "{}\n");
}

#[test]
fn tags_are_retained_on_scalars_and_collections() {
    #[derive(Serialize)]
    enum Shape {
        Circle { radius: u8 },
        Pair(u8, u8),
        Unit,
        Named(String),
    }
    let value = vec![
        Shape::Circle { radius: 2 },
        Shape::Pair(1, 2),
        Shape::Unit,
        Shape::Named("x".into()),
    ];
    assert_eq!(
        write(&value, 4),
        "- !Circle\n  radius: 2\n- !Pair\n  - 1\n  - 2\n- Unit\n- !Named x\n"
    );
    let tagged = yaml("k: !Tag\n  z: 1\nl: !L\n- 1\n");
    assert_eq!(write(&tagged, 3), "k: !Tag\n   z: 1\nl: !L\n   - 1\n");
}

#[test]
fn non_string_and_complex_keys_keep_their_type_and_use_explicit_entries() {
    let value = yaml("1: int\ntrue: bool\nnull: none\n'1': text\n? [a, b]\n: seq\n? {k: v}\n: map\n");
    assert_eq!(
        write(&value, 4),
        "1: int\ntrue: bool\nnull: none\n'1': text\n? - a\n  - b\n: seq\n? k: v\n: map\n"
    );
    let nested = yaml("outer:\n  ? [a]\n  : [b, c]\n");
    assert_eq!(write(&nested, 4), "outer:\n    ? - a\n    : - b\n      - c\n");
}

#[test]
fn multiline_keys_are_double_quoted_and_long_keys_stay_explicit() {
    let mut map = BTreeMap::new();
    map.insert("multi\nline key", 1);
    assert_eq!(write(&map, 4), "\"multi\\nline key\": 1\n");

    let long = "k".repeat(1100);
    let mut map = BTreeMap::new();
    map.insert(long.as_str(), 1);
    assert_eq!(write(&map, 4), format!("? {long}\n: 1\n"));
}

#[test]
fn non_finite_numbers_survive_the_equivalence_check() {
    let mut map = BTreeMap::new();
    map.insert("inf", f64::INFINITY);
    map.insert("nan", f64::NAN);
    map.insert("neg", f64::NEG_INFINITY);
    let out = to_yaml_string(&map, spaces(2)).unwrap();
    assert_eq!(out, "inf: .inf\nnan: .nan\nneg: -.inf\n");
    let back: BTreeMap<String, f64> = serde_yaml_ng::from_str(&out).unwrap();
    assert!(back["nan"].is_nan());
    assert_eq!(back["inf"], f64::INFINITY);
    assert_eq!(back["neg"], f64::NEG_INFINITY);
}

#[test]
fn quoted_scalars_keep_the_backend_spelling_and_their_type() {
    let value = yaml(
        "a: '123'\nb: 'true'\nc: 'null'\nd: 'x: y'\ne: '#hash'\nf: \"tab\\there\"\ng: 123\nh: true\ni: ''\n",
    );
    let out = write(&value, 4);
    assert_eq!(
        out,
        "a: '123'\nb: 'true'\nc: 'null'\nd: 'x: y'\ne: '#hash'\nf: \"tab\\there\"\ng: 123\nh: true\ni: ''\n"
    );
    assert_eq!(yaml(&out)["a"], Value::String("123".into()));
    assert_eq!(yaml(&out)["g"], Value::Number(123.into()));
}

#[test]
fn unicode_is_written_as_itself() {
    let mut map = BTreeMap::new();
    map.insert("café", "naïve ☕ 😀");
    map.insert("multi", "ünï\ncödé\n");
    assert_eq!(
        write(&map, 4),
        "café: naïve ☕ 😀\nmulti: |\n    ünï\n    cödé\n"
    );
}

#[test]
fn block_scalars_stay_block_scalars_with_their_chomping_at_every_width() {
    let mut map = BTreeMap::new();
    map.insert("clip", "line1\n\nline2\n");
    map.insert("keep", "a\n\n");
    map.insert("strip", "a: b\n- not a list\n# not a comment");
    for width in WIDTHS {
        let unit = " ".repeat(usize::from(width));
        assert_eq!(
            write(&map, width),
            format!(
                "clip: |\n{unit}line1\n\n{unit}line2\nkeep: |+\n{unit}a\n\nstrip: |-\n{unit}a: b\n{unit}- not a list\n{unit}# not a comment\n"
            ),
            "width {width}"
        );
    }
}

#[test]
fn block_scalars_in_sequences_and_compact_mappings_indent_from_their_indicator() {
    let value = yaml("- |\n  a\n  b\n- k: |-\n    x\n    y\n");
    assert_eq!(
        write(&value, 4),
        "- |\n    a\n    b\n- k: |-\n      x\n      y\n"
    );
}

#[test]
fn a_leading_space_takes_an_indentation_indicator_up_to_nine() {
    let mut map = BTreeMap::new();
    map.insert("lead", " lead\nx");
    map.insert("later", "\n  indented\n");
    assert_eq!(
        write(&map, 4),
        "later: |4\n\n      indented\nlead: |4-\n     lead\n    x\n"
    );
    assert_eq!(write(&map, 9), "later: |9\n\n           indented\nlead: |9-\n          lead\n         x\n");
}

#[test]
fn each_double_quoted_fallback_trigger_spells_the_content_escaped() {
    let mut map = BTreeMap::new();
    // An indentation indicator above nine.
    map.insert("indicator", " lead\nx");
    // Content of only line breaks, which chomping reads back as trailing breaks.
    map.insert("only_breaks", "\n\n");
    // A character a block scalar cannot carry as itself.
    map.insert("separator", "a\u{2028}b\n");
    let out = write(&map, 10);
    assert_eq!(
        out,
        "indicator: \" lead\\nx\"\nonly_breaks: \"\\n\\n\"\nseparator: \"a\\u2028b\\n\"\n"
    );

    // The root has no parent column for an indicator to count from.
    assert_eq!(write(" lead\nx", 4), "\" lead\\nx\"\n");
    // The choice is deterministic.
    assert_eq!(write(&map, 10), out);
}

#[test]
fn rewriting_the_output_is_a_fixed_point() {
    let value = yaml(
        "a:\n  - name: n\n    list: [1, 2]\n    text: |\n      x\n      y\n  - []\nb: !T {c: 1}\n",
    );
    for width in WIDTHS {
        let once = write(&value, width);
        assert_eq!(write(&yaml(&once), width), once, "width {width}");
    }
}

#[test]
fn the_yaml_wrapper_writes_its_value_and_keeps_its_source_text() {
    let source = "# comment\nkeys:\n- a   # trailing\n";
    let document = Yaml::from_str(source).unwrap();
    assert_eq!(document.to_yaml_string(spaces(2)).unwrap(), "keys:\n  - a\n");
    assert_eq!(document.source_text(), Some(source));

    let from_value = Yaml::from_value(yaml("k: v"));
    assert_eq!(from_value.to_yaml_string(Indentation::default()).unwrap(), "k: v\n");
}

#[test]
fn a_backend_serialization_failure_is_a_typed_serialize_error() {
    struct Refuses;
    impl Serialize for Refuses {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("refused by the value"))
        }
    }
    let error = to_yaml_string(&Refuses, Indentation::default()).unwrap_err();
    assert!(matches!(error, YamlError::Serialize(_)), "{error:?}");
    assert!(error.to_string().contains("refused by the value"), "{error}");
    assert_eq!(error.location(), None);
}

#[test]
fn the_yaml_corpus_round_trips_through_the_wrapper_at_every_width() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../corpus/yaml_corpus.json")).unwrap();
    let mut sources: Vec<(String, String)> = Vec::new();
    for case in corpus["yaml_test_suite"]["cases"].as_array().unwrap() {
        sources.push((case["id"].to_string(), case["source"].as_str().unwrap().to_string()));
    }
    for class in ["preserved", "repaired"] {
        for case in corpus[class].as_array().unwrap() {
            for field in ["source", "expected"] {
                if let Some(text) = case.get(field).and_then(serde_json::Value::as_str) {
                    sources.push((format!("{class}/{}/{field}", case["name"]), text.to_string()));
                }
            }
        }
    }

    let mut written = 0;
    for (name, source) in &sources {
        let Ok(document) = Yaml::from_str(source) else {
            continue;
        };
        for width in WIDTHS {
            let out = document
                .to_yaml_string(spaces(width))
                .unwrap_or_else(|error| panic!("[{name}] width {width}: {error}"));
            // Value and order, separately from layout.
            let back = yaml(&out);
            assert_eq!(&back, document.value(), "[{name}] width {width} value:\n{out}");
            assert_same_order(document.value(), &back);
            // Layout: no tab indentation, and a second write is unchanged.
            assert!(
                out.lines().all(|line| !line.starts_with('\t')),
                "[{name}] width {width} tab indentation:\n{out}"
            );
            assert_eq!(
                Yaml::from_value(back).to_yaml_string(spaces(width)).unwrap(),
                out,
                "[{name}] width {width} is not a fixed point"
            );
            written += 1;
        }
    }
    assert!(written >= 20 * WIDTHS.len(), "only {written} corpus writes ran");
}

#[test]
fn mapping_order_is_the_insertion_order() {
    let mut mapping = Mapping::new();
    mapping.insert("zeta".into(), 1.into());
    mapping.insert("alpha".into(), 2.into());
    assert_eq!(write(&Value::Mapping(mapping), 4), "zeta: 1\nalpha: 2\n");
}
