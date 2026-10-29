//! Numeric ingress spike (one-time, 2026-10-21). Throwaway; not production code.
//!
//! Uses the production loader crates at their Cargo.lock versions to show:
//! 1. what each loader hands Darkmatter today for each probe input;
//! 2. that a per-loader spelling sidecar (JSON Pointer -> authored numeric
//!    text) recovers a lossless input for every probe without changing the
//!    `serde_json::Value` tree, key order, or span sources;
//! 3. one exactness check over that spelling (`admit_spelled`).

use std::cell::RefCell;
use std::fmt::Write as _;

use serde::Deserialize;
use serde::de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

const INPUTS: &[&str] = &[
    "9007199254740992",
    "9007199254740993",
    "9007199254740993.0",
    "9.007199254740993e15",
    "-9007199254740992",
    "-9007199254740993",
    ".0",
    "1.0",
    "1e3",
    "0.1",
    "18446744073709551615",
    "99999999999999999999",
    "1e400",
    "1e-400",
];

// ---------------------------------------------------------------------------
// Admission: the exactness check over a decimal spelling
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Reject {
    NotDecimal,
    NotFinite,
    Underflow,
    Inexact,
}

/// Admits a decimal spelling (`[+-]digits[.digits][e[+-]digits]`, either side
/// of the point may be empty but not both). Whole-ness is decided on the
/// mathematical input; a whole input must equal the correctly rounded `f64`
/// exactly; a non-whole input takes the nearest `f64`; nonzero underflow and
/// overflow reject. Which spellings are *accepted as text* is the canonical
/// converter's policy, not this function's.
fn admit_spelled(spelling: &str) -> Result<f64, Reject> {
    let bytes = spelling.as_bytes();
    let mut index = 0;
    let negative = match bytes.first() {
        Some(b'-') => {
            index += 1;
            true
        }
        Some(b'+') => {
            index += 1;
            false
        }
        _ => false,
    };
    let mut digits = String::new();
    let mut fraction_len: i64 = 0;
    let mut seen_digit = false;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        digits.push(bytes[index] as char);
        seen_digit = true;
        index += 1;
    }
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            digits.push(bytes[index] as char);
            fraction_len += 1;
            seen_digit = true;
            index += 1;
        }
    }
    if !seen_digit {
        return Err(Reject::NotDecimal);
    }
    let mut exponent: i64 = 0;
    if index < bytes.len() && (bytes[index] == b'e' || bytes[index] == b'E') {
        index += 1;
        let exponent_text = &spelling[index..];
        if exponent_text.is_empty() {
            return Err(Reject::NotDecimal);
        }
        // Saturate absurd exponents; they are overflow/underflow either way.
        exponent = exponent_text.parse::<i64>().unwrap_or_else(|_| {
            if exponent_text.starts_with('-') { i64::MIN / 4 } else { i64::MAX / 4 }
        });
        if !exponent_text.trim_start_matches(['+', '-']).bytes().all(|b| b.is_ascii_digit()) {
            return Err(Reject::NotDecimal);
        }
        index = bytes.len();
    }
    if index != bytes.len() {
        return Err(Reject::NotDecimal);
    }
    let significant = digits.trim_start_matches('0');
    let parsed: f64 = spelling.parse().map_err(|_| Reject::NotDecimal)?;
    if significant.is_empty() {
        return Ok(if negative { -0.0 } else { 0.0 });
    }
    if !parsed.is_finite() {
        return Err(Reject::NotFinite);
    }
    if parsed == 0.0 {
        return Err(Reject::Underflow);
    }
    // Normalize: value = significant * 10^scale with no trailing zeros.
    let trimmed = significant.trim_end_matches('0');
    let scale = exponent - fraction_len + (significant.len() - trimmed.len()) as i64;
    if scale >= 0 {
        // Whole input. Compare its exact digits with the f64's exact digits.
        let mut expected = trimmed.to_string();
        for _ in 0..scale {
            expected.push('0');
        }
        let actual = format!("{:.0}", parsed.abs());
        if expected != actual {
            return Err(Reject::Inexact);
        }
    }
    Ok(parsed)
}

/// An already-decoded `f64` (LSP wire value, Rust caller): finiteness only.
fn admit_external(value: f64) -> Result<f64, Reject> {
    if value.is_finite() { Ok(value) } else { Err(Reject::NotFinite) }
}

fn verdict(result: Result<f64, Reject>) -> String {
    match result {
        Ok(value) => format!("admit {value:?}"),
        Err(reason) => format!("REJECT {reason:?}"),
    }
}

fn escape(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

// ---------------------------------------------------------------------------
// YAML: serde_yaml_ng 0.10.0 (lib frontmatter, schemas, catalogs)
// ---------------------------------------------------------------------------

type Spellings = RefCell<Vec<(String, String)>>;

/// Guided second pass: walks the same YAML text again with the shape of the
/// first-pass `serde_yaml_ng::Value`; at each `Number` node it asks the
/// deserializer for a *string*, which serde_yaml_ng answers with the scalar's
/// text without numeric resolution (de.rs `deserialize_str`).
struct YamlGuide<'a> {
    node: &'a serde_yaml_ng::Value,
    pointer: String,
    out: &'a Spellings,
}

impl<'de> DeserializeSeed<'de> for YamlGuide<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        match self.node {
            serde_yaml_ng::Value::Number(_) => {
                let text = String::deserialize(deserializer)?;
                self.out.borrow_mut().push((self.pointer, text));
                Ok(())
            }
            serde_yaml_ng::Value::Mapping(mapping) => deserializer.deserialize_map(YamlMap {
                mapping,
                pointer: self.pointer,
                out: self.out,
            }),
            serde_yaml_ng::Value::Sequence(items) => deserializer.deserialize_seq(YamlSeq {
                items,
                pointer: self.pointer,
                out: self.out,
            }),
            _ => IgnoredAny::deserialize(deserializer).map(|_| ()),
        }
    }
}

struct YamlMap<'a> {
    mapping: &'a serde_yaml_ng::Mapping,
    pointer: String,
    out: &'a Spellings,
}

impl<'de> Visitor<'de> for YamlMap<'_> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("mapping")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        while let Some(key) = access.next_key::<serde_yaml_ng::Value>()? {
            let child = self.mapping.get(&key).expect("first pass has the key");
            let segment = match &key {
                serde_yaml_ng::Value::String(text) => text.clone(),
                other => serde_yaml_ng::to_string(other).unwrap().trim().to_string(),
            };
            access.next_value_seed(YamlGuide {
                node: child,
                pointer: format!("{}/{}", self.pointer, escape(&segment)),
                out: self.out,
            })?;
        }
        Ok(())
    }
}

struct YamlSeq<'a> {
    items: &'a [serde_yaml_ng::Value],
    pointer: String,
    out: &'a Spellings,
}

impl<'de> Visitor<'de> for YamlSeq<'_> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("sequence")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        for (index, item) in self.items.iter().enumerate() {
            access.next_element_seed(YamlGuide {
                node: item,
                pointer: format!("{}/{index}", self.pointer),
                out: self.out,
            })?;
        }
        Ok(())
    }
}

/// The seam for YAML: today's two calls (frontmatter.rs:645-647) plus the
/// guided pass, run only when the first pass holds a `Number` that is not an
/// exact `u64`/`i64` (i.e. a float).
fn yaml_load_with_spellings(
    text: &str,
) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    let out = RefCell::new(Vec::new());
    if contains_yaml_float(&value) {
        let deserializer = serde_yaml_ng::Deserializer::from_str(text);
        YamlGuide { node: &value, pointer: String::new(), out: &out }
            .deserialize(deserializer)
            .map_err(|e| e.to_string())?;
    }
    let json: serde_json::Value = serde_yaml_ng::from_value(value).map_err(|e| e.to_string())?;
    Ok((json, out.into_inner()))
}

fn contains_yaml_float(value: &serde_yaml_ng::Value) -> bool {
    match value {
        serde_yaml_ng::Value::Number(number) => number.is_f64(),
        serde_yaml_ng::Value::Mapping(mapping) => mapping.values().any(contains_yaml_float),
        serde_yaml_ng::Value::Sequence(items) => items.iter().any(contains_yaml_float),
        serde_yaml_ng::Value::Tagged(tagged) => contains_yaml_float(&tagged.value),
        _ => false,
    }
}

/// Always runs the guided pass (for the per-input table).
fn yaml_spellings_always(text: &str) -> Result<Vec<(String, String)>, String> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    let out = RefCell::new(Vec::new());
    YamlGuide { node: &value, pointer: String::new(), out: &out }
        .deserialize(serde_yaml_ng::Deserializer::from_str(text))
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

// ---------------------------------------------------------------------------
// JSON: serde_json 1.0.149 (raw JSON Schema, transclusion replace, `md set`)
// ---------------------------------------------------------------------------

struct JsonGuide<'a> {
    node: &'a serde_json::Value,
    pointer: String,
    out: &'a Spellings,
}

impl<'de> DeserializeSeed<'de> for JsonGuide<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        match self.node {
            serde_json::Value::Number(_) => {
                let raw = <&serde_json::value::RawValue>::deserialize(deserializer)?;
                self.out.borrow_mut().push((self.pointer, raw.get().to_string()));
                Ok(())
            }
            serde_json::Value::Object(object) => deserializer.deserialize_map(JsonMap {
                object,
                pointer: self.pointer,
                out: self.out,
            }),
            serde_json::Value::Array(items) => deserializer.deserialize_seq(JsonSeq {
                items,
                pointer: self.pointer,
                out: self.out,
            }),
            _ => IgnoredAny::deserialize(deserializer).map(|_| ()),
        }
    }
}

struct JsonMap<'a> {
    object: &'a serde_json::Map<String, serde_json::Value>,
    pointer: String,
    out: &'a Spellings,
}

impl<'de> Visitor<'de> for JsonMap<'_> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        while let Some(key) = access.next_key::<String>()? {
            // Duplicate keys: last wins in the first pass; the guide records
            // both occurrences, which is where a duplicate policy would hook.
            let child = &self.object[&key];
            access.next_value_seed(JsonGuide {
                node: child,
                pointer: format!("{}/{}", self.pointer, escape(&key)),
                out: self.out,
            })?;
        }
        Ok(())
    }
}

struct JsonSeq<'a> {
    items: &'a [serde_json::Value],
    pointer: String,
    out: &'a Spellings,
}

impl<'de> Visitor<'de> for JsonSeq<'_> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("array")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        for (index, item) in self.items.iter().enumerate() {
            access.next_element_seed(JsonGuide {
                node: item,
                pointer: format!("{}/{index}", self.pointer),
                out: self.out,
            })?;
        }
        Ok(())
    }
}

fn json_spellings(text: &str) -> Result<(serde_json::Value, Vec<(String, String)>), String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let out = RefCell::new(Vec::new());
    let mut deserializer = serde_json::Deserializer::from_str(text);
    JsonGuide { node: &value, pointer: String::new(), out: &out }
        .deserialize(&mut deserializer)
        .map_err(|e| e.to_string())?;
    Ok((value, out.into_inner()))
}

// ---------------------------------------------------------------------------
// JSON5: json-five 0.3.1 (`md compose --state/--set`, `prop=value`, transclusion)
// ---------------------------------------------------------------------------

fn json5_spellings(text: &str) -> Result<Vec<(String, String)>, String> {
    use json_five::rt::parser::JSONValue as J;
    fn walk(node: &J, pointer: &str, sign: &str, out: &mut Vec<(String, String)>) {
        match node {
            J::Integer(s) | J::Float(s) | J::Exponent(s) => {
                out.push((pointer.to_string(), format!("{sign}{s}")))
            }
            J::Unary { operator, value } => {
                let sign = match format!("{operator:?}").as_str() {
                    "Minus" => "-",
                    _ => "",
                };
                walk(value, pointer, sign, out)
            }
            J::JSONObject { key_value_pairs, .. } => {
                for pair in key_value_pairs {
                    let key = match &pair.key {
                        J::DoubleQuotedString(s) | J::SingleQuotedString(s) | J::Identifier(s) => {
                            s.clone()
                        }
                        other => format!("{other:?}"),
                    };
                    walk(&pair.value, &format!("{pointer}/{}", escape(&key)), "", out);
                }
            }
            J::JSONArray { values, .. } => {
                for (index, item) in values.iter().enumerate() {
                    walk(&item.value, &format!("{pointer}/{index}"), "", out);
                }
            }
            _ => {}
        }
    }
    let text_model = json_five::rt::parser::from_str(text).map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::new();
    walk(&text_model.value, "", "", &mut out);
    Ok(out)
}

// ---------------------------------------------------------------------------
// rlsp-yaml-parser 0.11.1 (DMLS overlay): spelling and span on one node
// ---------------------------------------------------------------------------

fn rlsp_scalars(text: &str) -> Vec<(String, String, std::ops::Range<usize>)> {
    use rlsp_yaml_parser::Node;
    fn walk(node: &Node, pointer: &str, out: &mut Vec<(String, String, std::ops::Range<usize>)>) {
        match node {
            Node::Scalar { value, loc, .. } => {
                out.push((pointer.to_string(), value.clone(), loc.start as usize..loc.end as usize))
            }
            Node::Mapping { entries, .. } => {
                for (key, value) in entries {
                    let segment = match key {
                        Node::Scalar { value, .. } => value.clone(),
                        _ => "?".into(),
                    };
                    walk(value, &format!("{pointer}/{}", escape(&segment)), out);
                }
            }
            Node::Sequence { items, .. } => {
                for (index, item) in items.iter().enumerate() {
                    walk(item, &format!("{pointer}/{index}"), out);
                }
            }
            _ => {}
        }
    }
    let documents = rlsp_yaml_parser::load(text).expect("rlsp load");
    let mut out = Vec::new();
    walk(&documents[0].root, "", &mut out);
    out
}

// ---------------------------------------------------------------------------
// Mirrors of Darkmatter-internal grammars (copied logic, cited by file:line)
// ---------------------------------------------------------------------------

/// Mirror of `compose/expression/lexer.rs:890-923` `read_number`, entered only
/// on an ASCII digit (lexer.rs:763): `[0-9]+(\.[0-9]+)?` then `parse::<f64>`.
/// Returns (lexeme, f64, rest-of-input).
fn expression_lexer_mirror(input: &str) -> Option<(String, f64, String)> {
    if !input.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let integer_end = input.find(|c: char| !c.is_ascii_digit()).unwrap_or(input.len());
    let mut end = integer_end;
    let rest = &input[integer_end..];
    if rest.starts_with('.') && rest[1..].starts_with(|c: char| c.is_ascii_digit()) {
        let fraction = &rest[1..];
        end += 1 + fraction.find(|c: char| !c.is_ascii_digit()).unwrap_or(fraction.len());
    }
    let lexeme = input[..end].to_string();
    let value: f64 = lexeme.parse().ok()?;
    Some((lexeme, value, input[end..].to_string()))
}

fn main() {
    let mut report = String::new();
    let features = format!(
        "arbitrary_precision={} preserve_order={}",
        cfg!(feature = "ap"),
        cfg!(feature = "po")
    );
    writeln!(report, "# numeric-ingress spike run ({features})\n").unwrap();

    writeln!(report, "## A. Per-input: today's representation vs recovered spelling + admission\n").unwrap();
    writeln!(
        report,
        "| input | YAML today (serde_json::Value via serde_yaml_ng) | YAML sidecar spelling | JSON today | JSON sidecar (RawValue) | JSON5 today | JSON5 model spelling | expr lexer mirror | admission(spelling) |"
    )
    .unwrap();
    writeln!(report, "|---|---|---|---|---|---|---|---|---|").unwrap();
    for input in INPUTS {
        let yaml_text = format!("n: {input}\n");
        let yaml_today = match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&yaml_text) {
            Ok(value) => match serde_yaml_ng::from_value::<serde_json::Value>(value) {
                Ok(json) => format!("`{}`", json["n"]),
                Err(e) => format!("from_value error: {e}"),
            },
            Err(e) => format!("PARSE ERROR: {e}"),
        };
        let yaml_spelling = match yaml_spellings_always(&yaml_text) {
            Ok(spellings) => spellings
                .iter()
                .map(|(p, s)| format!("`{p}`=`{s}`"))
                .collect::<Vec<_>>()
                .join(", "),
            Err(e) => format!("n/a ({})", e.split(" at ").next().unwrap_or(&e)),
        };
        let yaml_spelling = if yaml_spelling.is_empty() {
            "none (not a Number; arrives as string)".to_string()
        } else {
            yaml_spelling
        };
        let json_text = format!("{{\"n\": {input}}}");
        let (json_today, json_spelling) = match json_spellings(&json_text) {
            Ok((value, spellings)) => (
                format!("`{}`", value["n"]),
                spellings.iter().map(|(p, s)| format!("`{p}`=`{s}`")).collect::<Vec<_>>().join(", "),
            ),
            Err(e) => (format!("PARSE ERROR: {e}"), "n/a".into()),
        };
        let json5_today = match json_five::from_str::<serde_json::Value>(&json_text) {
            Ok(value) => format!("`{}`", value["n"]),
            Err(e) => format!("ERROR: {e}"),
        };
        let json5_spelling = match json5_spellings(&json_text) {
            Ok(spellings) => spellings.iter().map(|(p, s)| format!("`{p}`=`{s}`")).collect::<Vec<_>>().join(", "),
            Err(e) => format!("n/a ({e})"),
        };
        let lexer = {
            let (sign, body) = match input.strip_prefix('-') {
                Some(body) => ("-", body),
                None => ("", *input),
            };
            match expression_lexer_mirror(body) {
                Some((lexeme, value, rest)) if rest.is_empty() => {
                    format!("{sign}`{lexeme}` -> {value:?}")
                }
                Some((lexeme, _, rest)) => format!("`{lexeme}` then `{rest}` (parse error)"),
                None => "not a literal (parse error)".into(),
            }
        };
        writeln!(
            report,
            "| `{input}` | {yaml_today} | {yaml_spelling} | {json_today} | {json_spelling} | {json5_today} | {json5_spelling} | {lexer} | {} |",
            verdict(admit_spelled(input))
        )
        .unwrap();
    }

    writeln!(report, "\n## B. External f64 (finiteness only)\n").unwrap();
    for value in [9007199254740992.0_f64, 0.1, f64::INFINITY, f64::NAN] {
        writeln!(report, "- `{value:?}` -> {}", verdict(admit_external(value))).unwrap();
    }

    writeln!(report, "\n## C. Order: nested mapping through the YAML seam\n").unwrap();
    let nested = "zeta: 1\nouter:\n  zulu: 9007199254740993.0\n  alpha: 2\n  mid:\n    b: 0.1\n    a: [3, 1e-400]\nbeta: 1.0\n";
    let (json, spellings) = yaml_load_with_spellings(nested).expect("nested yaml");
    writeln!(report, "- serde_json::Value (today's tree): `{json}`").unwrap();
    writeln!(report, "- sidecar spellings in visit order: {spellings:?}").unwrap();
    let first: serde_yaml_ng::Value = serde_yaml_ng::from_str(nested).unwrap();
    writeln!(
        report,
        "- serde_yaml_ng::Value `outer` key order (feeds MappingOrders): {:?}",
        first["outer"].as_mapping().unwrap().keys().map(|k| k.as_str().unwrap()).collect::<Vec<_>>()
    )
    .unwrap();
    for (pointer, spelling) in &spellings {
        writeln!(report, "  - `{pointer}` `{spelling}` -> {}", verdict(admit_spelled(spelling))).unwrap();
    }
    let integers_only = "a: 1\nb: {c: 9007199254740993}\n";
    let (_, none) = yaml_load_with_spellings(integers_only).unwrap();
    writeln!(report, "- integers-only document runs no guided pass: sidecar = {none:?}").unwrap();

    let anchored = "base: &b 9007199254740993.0\nuse: *b\n";
    writeln!(report, "- alias resolution in guided pass: {:?}", yaml_load_with_spellings(anchored).map(|r| r.1)).unwrap();
    let quoted = "q: '9007199254740993.0'\nt: !!float 1\n";
    writeln!(report, "- quoted/tagged: {:?}", yaml_load_with_spellings(quoted).map(|r| r.1)).unwrap();

    writeln!(report, "\n## D. JSON order and duplicates through the JSON seam\n").unwrap();
    let json_nested = r#"{"zeta": 1, "outer": {"zulu": 9007199254740993.0, "alpha": [1e-400, 0.1]}, "dup": 1.5, "dup": 2.5}"#;
    let (value, spellings) = json_spellings(json_nested).unwrap();
    writeln!(report, "- serde_json::Value: `{value}`").unwrap();
    writeln!(report, "- sidecar in source order: {spellings:?}").unwrap();

    writeln!(report, "\n## E. Spans: rlsp-yaml-parser (DMLS overlay loader) keeps spelling and span together\n").unwrap();
    for (pointer, value, span) in rlsp_scalars(nested) {
        writeln!(
            report,
            "- `{pointer}` value=`{value}` span={span:?} source[span]=`{}`",
            &nested[span.clone()]
        )
        .unwrap();
    }
    let located = serde_yaml_ng::from_str::<serde_yaml_ng::Value>("ok: 1\nbig: 99999999999999999999\n")
        .unwrap_err();
    writeln!(report, "- serde_yaml_ng u128 error carries a location: `{located}`").unwrap();

    writeln!(report, "\n## F. serde_json feature interactions\n").unwrap();
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    #[allow(dead_code)]
    enum NumberOrText {
        Number(f64),
        Text(String),
    }
    writeln!(report, "- untagged enum from `5`: {:?}", serde_json::from_str::<NumberOrText>("5").map_err(|e| e.to_string())).unwrap();
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Flattened {
        #[serde(flatten)]
        rest: std::collections::HashMap<String, u64>,
    }
    writeln!(report, "- flatten map<u64> from `{{\"a\":5}}`: {:?}", serde_json::from_str::<Flattened>(r#"{"a":5}"#).map_err(|e| e.to_string())).unwrap();
    let one: serde_json::Value = serde_json::from_str("1.0").unwrap();
    let one_again: serde_json::Value = serde_json::from_str("1.00").unwrap();
    writeln!(report, "- Value(1.0) == Value(1.00): {}", one == one_again).unwrap();
    let big: serde_json::Value = serde_json::from_str("9007199254740993.0").unwrap();
    writeln!(report, "- JSON 9007199254740993.0 as Value: `{big}`").unwrap();
    let via_yaml: serde_json::Value =
        serde_yaml_ng::from_value(serde_yaml_ng::from_str::<serde_yaml_ng::Value>("9007199254740993.0").unwrap()).unwrap();
    writeln!(report, "- YAML 9007199254740993.0 -> serde_json::Value: `{via_yaml}` (YAML visits f64 first)").unwrap();
    let via_json5: Result<serde_json::Value, _> = json_five::from_str("9007199254740993.0");
    writeln!(report, "- JSON5 9007199254740993.0 -> serde_json::Value: {via_json5:?}").unwrap();
    let inf_json5: Result<serde_json::Value, _> = json_five::from_str(r#"{"q": 1e400, "r": Infinity}"#);
    writeln!(report, "- JSON5 1e400 / Infinity -> serde_json::Value: {inf_json5:?}").unwrap();
    let inf_yaml: serde_json::Value =
        serde_yaml_ng::from_value(serde_yaml_ng::from_str::<serde_yaml_ng::Value>("{q: .inf, r: .nan}").unwrap()).unwrap();
    writeln!(report, "- YAML .inf / .nan -> serde_json::Value: `{inf_yaml}`").unwrap();
    let order: serde_json::Value = serde_json::from_str(r#"{"z":1,"a":2}"#).unwrap();
    writeln!(report, "- JSON object order {{z,a}} -> `{order}`").unwrap();

    print!("{report}");
}
