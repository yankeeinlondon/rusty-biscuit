//! The stored baseline (`baseline.json`) and its strict loader.
//!
//! Every shape the loader does not accept is an error that names the JSON
//! path it found it at (`families.flowchart.warm.median_ms`), and a rejected
//! baseline is never compared against, flagged from, or overwritten. Duplicate
//! keys are rejected at every depth: serde_json's own maps keep the last
//! duplicate silently, so the document is first read into [`Strict`], a value
//! tree whose map visitor refuses a repeated key.
//!
//! ```json
//! {
//!   "reference_host": "build-linux",
//!   "families": {
//!     "flowchart": {
//!       "warm": { "median_ms": 12.5, "p95_ms": 14.0 },
//!       "cold": { "median_ms": 40.0, "p95_ms": 45.0 }
//!     }
//!   }
//! }
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

/// Median and p95 of one measurement, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub median_ms: f64,
    pub p95_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FamilyBaseline {
    pub warm: Stats,
    pub cold: Stats,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Baseline {
    /// Hostname of the machine the baseline was recorded on; a run on a host
    /// with this name enforces the limits and the regression rule.
    pub reference_host: String,
    pub families: BTreeMap<String, FamilyBaseline>,
}

/// What the baseline says about one family of a run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lookup<'a> {
    Recorded(&'a FamilyBaseline),
    /// The family is in the corpus but absent from the baseline. Reported as
    /// its own state, never as a pass.
    NoBaseline,
}

impl Baseline {
    pub fn lookup(&self, family: &str) -> Lookup<'_> {
        match self.families.get(family) {
            Some(recorded) => Lookup::Recorded(recorded),
            None => Lookup::NoBaseline,
        }
    }

    pub fn to_json(&self) -> String {
        let stats = |stats: &Stats| serde_json::json!({ "median_ms": round(stats.median_ms), "p95_ms": round(stats.p95_ms) });
        let families: serde_json::Map<String, serde_json::Value> = self
            .families
            .iter()
            .map(|(family, recorded)| {
                (family.clone(), serde_json::json!({ "warm": stats(&recorded.warm), "cold": stats(&recorded.cold) }))
            })
            .collect();
        let document =
            serde_json::json!({ "reference_host": self.reference_host, "families": families });
        let mut text =
            serde_json::to_string_pretty(&document).expect("a baseline always serializes");
        text.push('\n');
        text
    }
}

/// Three decimals (microseconds) is finer than the run-to-run noise.
fn round(ms: f64) -> f64 {
    (ms * 1000.0).round() / 1000.0
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineError(pub String);

impl fmt::Display for BaselineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BaselineError {}

/// Reads and validates the baseline at `path`. `known` is the corpus; a
/// family outside it is an error, so a misspelled entry cannot hide.
pub fn load(path: &Path, known: &[&str]) -> Result<Baseline, BaselineError> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        BaselineError(format!("cannot read baseline {}: {error}", path.display()))
    })?;
    parse(&text, known)
        .map_err(|BaselineError(message)| BaselineError(format!("{}: {message}", path.display())))
}

pub fn parse(text: &str, known: &[&str]) -> Result<Baseline, BaselineError> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let root = StrictSeed { path: "" }
        .deserialize(&mut deserializer)
        .map_err(|error| BaselineError(format!("invalid baseline JSON: {error}")))?;
    deserializer.end().map_err(|error| {
        BaselineError(format!("invalid baseline JSON after the document: {error}"))
    })?;

    let fields = object(&root, "(document)")?;
    only_keys(fields, "", &["reference_host", "families"])?;
    let reference_host = match field(fields, "", "reference_host")? {
        Strict::String(host) if !host.is_empty() => host.clone(),
        other => {
            return Err(wrong(
                "reference_host",
                other,
                "a non-empty hostname string",
            ));
        }
    };
    let mut families = BTreeMap::new();
    for (family, value) in object(field(fields, "", "families")?, "families")? {
        let path = format!("families.{family}");
        if !known.contains(&family.as_str()) {
            return Err(BaselineError(format!(
                "{path}: not a family of the benchmark corpus"
            )));
        }
        let entry = object(value, &path)?;
        only_keys(entry, &path, &["warm", "cold"])?;
        let warm = stats(field(entry, &path, "warm")?, &format!("{path}.warm"))?;
        let cold = stats(field(entry, &path, "cold")?, &format!("{path}.cold"))?;
        families.insert(family.clone(), FamilyBaseline { warm, cold });
    }
    Ok(Baseline {
        reference_host,
        families,
    })
}

fn stats(value: &Strict, path: &str) -> Result<Stats, BaselineError> {
    let fields = object(value, path)?;
    only_keys(fields, path, &["median_ms", "p95_ms"])?;
    let median_ms = milliseconds(
        field(fields, path, "median_ms")?,
        &format!("{path}.median_ms"),
    )?;
    let p95_ms = milliseconds(field(fields, path, "p95_ms")?, &format!("{path}.p95_ms"))?;
    if p95_ms < median_ms {
        return Err(BaselineError(format!(
            "{path}: p95_ms {p95_ms} is below median_ms {median_ms}"
        )));
    }
    Ok(Stats { median_ms, p95_ms })
}

/// A duration must be a finite number above zero: a zero baseline would make
/// every run a regression.
fn milliseconds(value: &Strict, path: &str) -> Result<f64, BaselineError> {
    match value {
        Strict::Number(ms) if ms.is_finite() && *ms > 0.0 => Ok(*ms),
        Strict::Number(ms) => Err(BaselineError(format!(
            "{path}: is {ms}, expected a positive number of milliseconds"
        ))),
        other => Err(wrong(path, other, "a positive number of milliseconds")),
    }
}

fn object<'a>(value: &'a Strict, path: &str) -> Result<&'a [(String, Strict)], BaselineError> {
    match value {
        Strict::Object(fields) => Ok(fields),
        other => Err(wrong(path, other, "an object")),
    }
}

fn field<'a>(
    fields: &'a [(String, Strict)],
    path: &str,
    key: &str,
) -> Result<&'a Strict, BaselineError> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
        .ok_or_else(|| BaselineError(format!("{}: absent", join(path, key))))
}

fn only_keys(
    fields: &[(String, Strict)],
    path: &str,
    allowed: &[&str],
) -> Result<(), BaselineError> {
    match fields
        .iter()
        .find(|(name, _)| !allowed.contains(&name.as_str()))
    {
        Some((name, _)) => Err(BaselineError(format!(
            "{}: unknown key (expected one of {allowed:?})",
            join(path, name)
        ))),
        None => Ok(()),
    }
}

fn wrong(path: &str, found: &Strict, expected: &str) -> BaselineError {
    BaselineError(format!("{path}: is {}, expected {expected}", found.kind()))
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

/// A JSON value whose objects keep their keys in order and reject duplicates.
#[derive(Debug, Clone, PartialEq)]
enum Strict {
    Null,
    Bool,
    Number(f64),
    String(String),
    Array,
    Object(Vec<(String, Strict)>),
}

impl Strict {
    fn kind(&self) -> &'static str {
        match self {
            Strict::Null => "null",
            Strict::Bool => "a boolean",
            Strict::Number(_) => "a number",
            Strict::String(_) => "a string",
            Strict::Array => "an array",
            Strict::Object(_) => "an object",
        }
    }
}

/// Carries the JSON path down so a duplicate key's error names where it is.
struct StrictSeed<'p> {
    path: &'p str,
}

impl<'de> DeserializeSeed<'de> for StrictSeed<'_> {
    type Value = Strict;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Strict, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for StrictSeed<'_> {
    type Value = Strict;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
        Ok(Strict::Null)
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Strict, E> {
        Ok(Strict::Bool)
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Strict, E> {
        Ok(Strict::Number(value as f64))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Strict, E> {
        Ok(Strict::Number(value as f64))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Strict, E> {
        Ok(Strict::Number(value))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Strict, E> {
        Ok(Strict::String(value.to_owned()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Strict, A::Error> {
        let mut index = 0;
        loop {
            let path = format!("{}[{index}]", self.path);
            if seq.next_element_seed(StrictSeed { path: &path })?.is_none() {
                return Ok(Strict::Array);
            }
            index += 1;
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Strict, A::Error> {
        let mut entries: Vec<(String, Strict)> = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            let path = join(self.path, &key);
            if entries.iter().any(|(seen, _)| *seen == key) {
                return Err(de::Error::custom(format!("{path}: duplicate key")));
            }
            let value = map.next_value_seed(StrictSeed { path: &path })?;
            entries.push((key, value));
        }
        Ok(Strict::Object(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN: &[&str] = &["flowchart", "pie"];

    const VALID_FLOWCHART: &str =
        r#"{"warm": {"median_ms": 10, "p95_ms": 12}, "cold": {"median_ms": 40, "p95_ms": 50}}"#;

    /// A baseline whose flowchart `warm.median_ms` is the JSON text `value`;
    /// `None` leaves the field out.
    fn with_warm_median(value: Option<&str>) -> String {
        let median = match value {
            Some(value) => format!(r#""median_ms": {value}, "#),
            None => String::new(),
        };
        format!(
            r#"{{"reference_host": "h", "families": {{"flowchart": {{"warm": {{{median}"p95_ms": 12}}, "cold": {{"median_ms": 40, "p95_ms": 50}}}}}}}}"#
        )
    }

    /// Same for `cold.p95_ms`.
    fn with_cold_p95(value: Option<&str>) -> String {
        let p95 = match value {
            Some(value) => format!(r#", "p95_ms": {value}"#),
            None => String::new(),
        };
        format!(
            r#"{{"reference_host": "h", "families": {{"flowchart": {{"warm": {{"median_ms": 10, "p95_ms": 12}}, "cold": {{"median_ms": 40{p95}}}}}}}}}"#
        )
    }

    fn error(text: &str) -> String {
        parse(text, KNOWN).expect_err("baseline must be rejected").0
    }

    #[test]
    fn committed_baseline_loads() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("baseline.json");
        let known: Vec<&str> = crate::corpus::names().collect();
        let baseline = load(&path, &known).expect("committed baseline loads");
        assert!(
            known
                .iter()
                .all(|family| matches!(baseline.lookup(family), Lookup::Recorded(_)))
        );
    }

    #[test]
    fn valid_baseline_parses_and_roundtrips() {
        let text =
            format!(r#"{{"reference_host": "h", "families": {{"flowchart": {VALID_FLOWCHART}}}}}"#);
        let baseline = parse(&text, KNOWN).expect("valid");
        assert_eq!(
            baseline.lookup("flowchart"),
            Lookup::Recorded(&FamilyBaseline {
                warm: Stats {
                    median_ms: 10.0,
                    p95_ms: 12.0
                },
                cold: Stats {
                    median_ms: 40.0,
                    p95_ms: 50.0
                },
            })
        );
        assert_eq!(parse(&baseline.to_json(), KNOWN), Ok(baseline));
    }

    #[test]
    fn median_absent_is_located_error() {
        assert_eq!(
            error(&with_warm_median(None)),
            "families.flowchart.warm.median_ms: absent"
        );
    }

    #[test]
    fn median_null_is_located_error() {
        assert_eq!(
            error(&with_warm_median(Some("null"))),
            "families.flowchart.warm.median_ms: is null, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn median_string_is_wrong_type() {
        assert_eq!(
            error(&with_warm_median(Some(r#""10""#))),
            "families.flowchart.warm.median_ms: is a string, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn median_empty_array_is_wrong_type() {
        assert_eq!(
            error(&with_warm_median(Some("[]"))),
            "families.flowchart.warm.median_ms: is an array, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn median_empty_object_is_wrong_type() {
        assert_eq!(
            error(&with_warm_median(Some("{}"))),
            "families.flowchart.warm.median_ms: is an object, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn median_negative_is_located_error() {
        assert_eq!(
            error(&with_warm_median(Some("-1.5"))),
            "families.flowchart.warm.median_ms: is -1.5, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn median_zero_is_located_error() {
        assert_eq!(
            error(&with_warm_median(Some("0"))),
            "families.flowchart.warm.median_ms: is 0, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn median_nan_literal_is_parse_error() {
        assert!(error(&with_warm_median(Some("NaN"))).starts_with("invalid baseline JSON: "));
    }

    #[test]
    fn median_nan_string_is_wrong_type() {
        assert_eq!(
            error(&with_warm_median(Some(r#""NaN""#))),
            "families.flowchart.warm.median_ms: is a string, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn p95_absent_is_located_error() {
        assert_eq!(
            error(&with_cold_p95(None)),
            "families.flowchart.cold.p95_ms: absent"
        );
    }

    #[test]
    fn p95_null_is_located_error() {
        assert_eq!(
            error(&with_cold_p95(Some("null"))),
            "families.flowchart.cold.p95_ms: is null, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn p95_string_is_wrong_type() {
        assert_eq!(
            error(&with_cold_p95(Some(r#""50""#))),
            "families.flowchart.cold.p95_ms: is a string, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn p95_empty_array_is_wrong_type() {
        assert_eq!(
            error(&with_cold_p95(Some("[]"))),
            "families.flowchart.cold.p95_ms: is an array, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn p95_empty_object_is_wrong_type() {
        assert_eq!(
            error(&with_cold_p95(Some("{}"))),
            "families.flowchart.cold.p95_ms: is an object, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn p95_negative_is_located_error() {
        assert_eq!(
            error(&with_cold_p95(Some("-50"))),
            "families.flowchart.cold.p95_ms: is -50, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn p95_nan_literal_is_parse_error() {
        assert!(error(&with_cold_p95(Some("NaN"))).starts_with("invalid baseline JSON: "));
    }

    #[test]
    fn p95_nan_string_is_wrong_type() {
        assert_eq!(
            error(&with_cold_p95(Some(r#""NaN""#))),
            "families.flowchart.cold.p95_ms: is a string, expected a positive number of milliseconds"
        );
    }

    #[test]
    fn p95_below_median_is_located_error() {
        assert_eq!(
            error(&with_cold_p95(Some("39"))),
            "families.flowchart.cold: p95_ms 39 is below median_ms 40"
        );
    }

    #[test]
    fn family_missing_from_baseline_is_no_baseline_state() {
        let text =
            format!(r#"{{"reference_host": "h", "families": {{"flowchart": {VALID_FLOWCHART}}}}}"#);
        let baseline = parse(&text, KNOWN).expect("valid");
        assert_eq!(baseline.lookup("pie"), Lookup::NoBaseline);
    }

    #[test]
    fn empty_families_object_is_valid_and_every_family_has_no_baseline() {
        let baseline = parse(r#"{"reference_host": "h", "families": {}}"#, KNOWN).expect("valid");
        assert!(
            KNOWN
                .iter()
                .all(|family| baseline.lookup(family) == Lookup::NoBaseline)
        );
    }

    #[test]
    fn empty_document_is_located_error() {
        assert_eq!(error("{}"), "reference_host: absent");
    }

    #[test]
    fn families_absent_is_located_error() {
        assert_eq!(error(r#"{"reference_host": "h"}"#), "families: absent");
    }

    #[test]
    fn family_not_an_object_is_located_error() {
        assert_eq!(
            error(r#"{"reference_host": "h", "families": {"flowchart": 3}}"#),
            "families.flowchart: is a number, expected an object"
        );
    }

    #[test]
    fn family_without_cold_is_located_error() {
        assert_eq!(
            error(
                r#"{"reference_host": "h", "families": {"flowchart": {"warm": {"median_ms": 1, "p95_ms": 2}}}}"#
            ),
            "families.flowchart.cold: absent"
        );
    }

    #[test]
    fn unknown_family_is_located_error() {
        assert_eq!(
            error(&format!(
                r#"{{"reference_host": "h", "families": {{"flowchrt": {VALID_FLOWCHART}}}}}"#
            )),
            "families.flowchrt: not a family of the benchmark corpus"
        );
    }

    #[test]
    fn unknown_field_is_located_error() {
        assert_eq!(
            error(
                r#"{"reference_host": "h", "families": {"flowchart": {"warm": {"median_ms": 1, "p95_ms": 2, "mean_ms": 1}, "cold": {"median_ms": 1, "p95_ms": 2}}}}"#
            ),
            r#"families.flowchart.warm.mean_ms: unknown key (expected one of ["median_ms", "p95_ms"])"#
        );
    }

    #[test]
    fn trailing_garbage_is_error() {
        let text = format!(
            r#"{{"reference_host": "h", "families": {{"flowchart": {VALID_FLOWCHART}}}}} trailing"#
        );
        assert!(
            error(&text)
                .starts_with("invalid baseline JSON after the document: trailing characters")
        );
    }

    #[test]
    fn duplicate_family_is_error() {
        let text = format!(
            r#"{{"reference_host": "h", "families": {{"flowchart": {VALID_FLOWCHART}, "flowchart": {VALID_FLOWCHART}}}}}"#
        );
        assert!(
            error(&text).starts_with("invalid baseline JSON: families.flowchart: duplicate key")
        );
    }

    #[test]
    fn duplicate_field_in_family_is_error() {
        let text = r#"{"reference_host": "h", "families": {"flowchart": {"warm": {"median_ms": 1, "median_ms": 100, "p95_ms": 200}, "cold": {"median_ms": 1, "p95_ms": 2}}}}"#;
        assert!(error(text).starts_with(
            "invalid baseline JSON: families.flowchart.warm.median_ms: duplicate key"
        ));
    }

    #[test]
    fn duplicate_top_level_key_is_error() {
        let text = r#"{"reference_host": "h", "families": {}, "families": {}}"#;
        assert!(error(text).starts_with("invalid baseline JSON: families: duplicate key"));
    }

    #[test]
    fn missing_file_is_error_naming_the_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("absent.json");
        let message = load(&path, KNOWN).expect_err("missing file").0;
        assert!(
            message.starts_with(&format!("cannot read baseline {}", path.display())),
            "{message}"
        );
    }
}
