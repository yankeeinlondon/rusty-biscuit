//! Number admission at load time, from each number's written spelling.
//!
//! The value loaders keep building today's values; these passes re-read the
//! same text to recover what a number's value tree no longer carries (its
//! spelling and location) and run it through the admission rules. YAML and
//! JSON reach a number's spelling differently:
//!
//! - YAML: a guided second pass over the text, shaped by the first-pass
//!   [`serde_yaml_ng::Value`], asks for each number node as a string, which
//!   `serde_yaml_ng` answers with the scalar's text. Its errors carry the
//!   property path and line/column.
//! - JSON: each value is read as a [`RawValue`], whose text is the written
//!   number and whose address locates it in the source.

use std::fmt;

use biscuit_file::serde_yaml_ng;
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use serde_json::value::RawValue;

use super::{NumberTextError, NumericAdmissionError, admit_f64, admit_integer, admit_number_spelling};

/// Appended to every YAML admission message; [`is_yaml_admission_error`]
/// recognizes the error by it.
const QUOTE_HINT: &str = "quote it to keep it as text";

/// The message for a written number that cannot be admitted.
fn admission_message(spelling: &str, reason: NumericAdmissionError) -> String {
    format!("number {spelling} {reason}; {QUOTE_HINT}")
}

/// Whether `error` is a YAML number-admission failure from this module.
pub(crate) fn is_yaml_admission_error(error: &serde_yaml_ng::Error) -> bool {
    error.to_string().contains(QUOTE_HINT)
}

/// Parses YAML text into a value whose every number is admitted.
///
/// ## Errors
///
/// The `serde_yaml_ng` parse error, or an admission error naming the
/// property path, line, column, and reason. An integer too large for the
/// value model (beyond `u64`/`i64`) fails the first parse; it is reported as
/// an admission error when admission rejects it.
pub(crate) fn load_admitted_yaml(text: &str) -> Result<serde_yaml_ng::Value, serde_yaml_ng::Error> {
    let value = match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) {
        Ok(value) => value,
        Err(error) => return Err(oversized_integer_error(text).unwrap_or(error)),
    };
    admit_yaml_numbers(text, &value)?;
    Ok(value)
}

/// Admits every number in `value`, which was parsed from `text`.
///
/// A document whose numbers are all admissible integers needs no spelling,
/// so the second pass runs only when a float or an inexact integer exists.
fn admit_yaml_numbers(text: &str, value: &serde_yaml_ng::Value) -> Result<(), serde_yaml_ng::Error> {
    if !needs_spelling_pass(value) {
        return Ok(());
    }
    YamlGuide { node: value }.deserialize(serde_yaml_ng::Deserializer::from_str(text))
}

fn needs_spelling_pass(value: &serde_yaml_ng::Value) -> bool {
    match value {
        serde_yaml_ng::Value::Number(number) => match (number.as_i64(), number.as_u64()) {
            (Some(integer), _) => admit_integer(i128::from(integer)).is_err(),
            (None, Some(integer)) => admit_integer(i128::from(integer)).is_err(),
            (None, None) => true,
        },
        serde_yaml_ng::Value::Mapping(mapping) => mapping.values().any(needs_spelling_pass),
        serde_yaml_ng::Value::Sequence(items) => items.iter().any(needs_spelling_pass),
        _ => false,
    }
}

/// Re-reads `text` looking for an integer beyond the value model's range; the
/// located admission error for the first one admission rejects.
fn oversized_integer_error(text: &str) -> Option<serde_yaml_ng::Error> {
    OversizedScan::deserialize(serde_yaml_ng::Deserializer::from_str(text))
        .err()
        .filter(is_yaml_admission_error)
}

/// The guided second pass: walks the text in step with the first-pass value.
struct YamlGuide<'a> {
    node: &'a serde_yaml_ng::Value,
}

impl<'de> DeserializeSeed<'de> for YamlGuide<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        match self.node {
            serde_yaml_ng::Value::Number(number) => deserializer.deserialize_str(NumberSpelling { number }),
            serde_yaml_ng::Value::Mapping(mapping) => deserializer.deserialize_map(YamlMapGuide { mapping }),
            serde_yaml_ng::Value::Sequence(items) => deserializer.deserialize_seq(YamlSeqGuide { items }),
            _ => IgnoredAny::deserialize(deserializer).map(|_| ()),
        }
    }
}

/// Admits one number node from its written text.
struct NumberSpelling<'a> {
    number: &'a serde_yaml_ng::Number,
}

impl NumberSpelling<'_> {
    fn admit(&self, spelling: &str) -> Result<(), NumericAdmissionError> {
        if let Some(integer) = self.number.as_i64() {
            return admit_integer(i128::from(integer)).map(|_| ());
        }
        if let Some(integer) = self.number.as_u64() {
            return admit_integer(i128::from(integer)).map(|_| ());
        }
        let parsed = self.number.as_f64().unwrap_or(f64::NAN);
        admit_f64(parsed)?;
        match admit_number_spelling(spelling) {
            Ok(_) => Ok(()),
            Err(NumberTextError::Admission(reason)) => Err(reason),
            // A YAML float spelling outside the decimal grammar (none is known
            // today) keeps the finiteness check alone.
            Err(NumberTextError::NotANumber) => Ok(()),
        }
    }
}

impl<'de> Visitor<'de> for NumberSpelling<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a number")
    }

    fn visit_str<E: de::Error>(self, spelling: &str) -> Result<(), E> {
        self.admit(spelling)
            .map_err(|reason| E::custom(admission_message(spelling, reason)))
    }
}

struct YamlMapGuide<'a> {
    mapping: &'a serde_yaml_ng::Mapping,
}

impl<'de> Visitor<'de> for YamlMapGuide<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a mapping")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        while let Some(key) = access.next_key::<serde_yaml_ng::Value>()? {
            match self.mapping.get(&key) {
                Some(node) => access.next_value_seed(YamlGuide { node })?,
                None => access.next_value::<IgnoredAny>().map(|_| ())?,
            }
        }
        Ok(())
    }
}

struct YamlSeqGuide<'a> {
    items: &'a [serde_yaml_ng::Value],
}

impl<'de> Visitor<'de> for YamlSeqGuide<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        for node in self.items {
            if access.next_element_seed(YamlGuide { node })?.is_none() {
                break;
            }
        }
        while access.next_element::<IgnoredAny>()?.is_some() {}
        Ok(())
    }
}

/// A scan that accepts any YAML and fails only on an integer beyond
/// `u64`/`i64` that admission rejects.
struct OversizedScan;

impl<'de> Deserialize<'de> for OversizedScan {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OversizedScan)
    }
}

impl OversizedScan {
    fn check<E: de::Error>(spelling: String) -> Result<Self, E> {
        match admit_number_spelling(&spelling) {
            Err(NumberTextError::Admission(reason)) => Err(E::custom(admission_message(&spelling, reason))),
            _ => Ok(Self),
        }
    }
}

impl<'de> Visitor<'de> for OversizedScan {
    type Value = Self;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any YAML value")
    }

    fn visit_u128<E: de::Error>(self, integer: u128) -> Result<Self, E> {
        Self::check(integer.to_string())
    }

    fn visit_i128<E: de::Error>(self, integer: i128) -> Result<Self, E> {
        Self::check(integer.to_string())
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

    fn visit_none<E: de::Error>(self) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self, D::Error> {
        Self::deserialize(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self, A::Error> {
        while access.next_element::<Self>()?.is_some() {}
        Ok(Self)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self, A::Error> {
        while access.next_entry::<Self, Self>()?.is_some() {}
        Ok(Self)
    }

    fn visit_enum<A: de::EnumAccess<'de>>(self, access: A) -> Result<Self, A::Error> {
        use serde::de::VariantAccess;
        let (_, variant) = access.variant::<IgnoredAny>()?;
        variant.newtype_variant::<Self>()
    }
}

/// A JSON number that admission rejects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsonNumberError {
    /// JSON Pointer to the number (`""` for a root number).
    pub pointer: String,
    /// The number as written.
    pub spelling: String,
    pub reason: NumericAdmissionError,
    /// 1-based line of the number's first character.
    pub line: usize,
    /// 1-based column (in characters) of the number's first character.
    pub column: usize,
}

impl fmt::Display for JsonNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pointer = if self.pointer.is_empty() { "/" } else { &self.pointer };
        write!(
            f,
            "{pointer}: number {} {} at line {} column {}",
            self.spelling, self.reason, self.line, self.column
        )
    }
}

/// Admits every number in a JSON document from its written spelling.
///
/// The caller has already parsed `text` successfully; a text that fails to
/// parse here reports no admission error. A number shadowed by a later
/// duplicate key is checked too.
///
/// ## Errors
///
/// The first number, in document order, that admission rejects.
pub(crate) fn admit_json_text_numbers(text: &str) -> Result<(), JsonNumberError> {
    let Ok(root) = serde_json::from_str::<&RawValue>(text) else {
        return Ok(());
    };
    admit_raw_json(text, root, &mut String::new())
}

fn admit_raw_json(source: &str, raw: &RawValue, pointer: &mut String) -> Result<(), JsonNumberError> {
    let text = raw.get();
    match text.as_bytes().first() {
        Some(b'{') => {
            let Ok(JsonEntries(entries)) = serde_json::from_str::<JsonEntries<'_>>(text) else {
                return Ok(());
            };
            for (key, child) in entries {
                let length = pointer.len();
                pointer.push('/');
                pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                admit_raw_json(source, child, pointer)?;
                pointer.truncate(length);
            }
            Ok(())
        }
        Some(b'[') => {
            let Ok(items) = serde_json::from_str::<Vec<&RawValue>>(text) else {
                return Ok(());
            };
            for (index, child) in items.into_iter().enumerate() {
                let length = pointer.len();
                pointer.push('/');
                pointer.push_str(&index.to_string());
                admit_raw_json(source, child, pointer)?;
                pointer.truncate(length);
            }
            Ok(())
        }
        Some(b'-' | b'0'..=b'9') => match admit_number_spelling(text) {
            Err(NumberTextError::Admission(reason)) => {
                let (line, column) = line_column(source, text);
                Err(JsonNumberError {
                    pointer: pointer.clone(),
                    spelling: text.to_string(),
                    reason,
                    line,
                    column,
                })
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

/// The 1-based line and column at which `slice`, a subslice of `source`,
/// begins; `(1, 1)` when it is not a subslice.
fn line_column(source: &str, slice: &str) -> (usize, usize) {
    let offset = (slice.as_ptr() as usize).wrapping_sub(source.as_ptr() as usize);
    let Some(before) = source.get(..offset) else {
        return (1, 1);
    };
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    (line, before[line_start..].chars().count() + 1)
}

/// An object's members in written order, duplicates included, each value
/// still borrowed as its raw text.
struct JsonEntries<'a>(Vec<(String, &'a RawValue)>);

impl<'de: 'a, 'a> Deserialize<'de> for JsonEntries<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = JsonEntries<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = access.next_entry::<String, &'de RawValue>()? {
                    entries.push(entry);
                }
                Ok(JsonEntries(entries))
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

/// Rejects a JSON5 document holding a non-finite number (`Infinity`, `NaN`,
/// or a literal such as `1e400` that overflows).
///
/// `serde_json::Value` cannot hold such a number, so [`biscuit_file::Json5`]
/// silently stores `null` for it; callers that turn JSON5 text into Darkmatter
/// values check the document with this first.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::numeric::reject_non_finite_json5;
///
/// let finite = biscuit_file::Json5::from_str("{ n: 1.5 }").unwrap();
/// assert!(reject_non_finite_json5(&finite).is_ok());
/// let infinite = biscuit_file::Json5::from_str("{ n: Infinity }").unwrap();
/// assert!(reject_non_finite_json5(&infinite).is_err());
/// ```
///
/// ## Errors
///
/// [`NumericAdmissionError::NotFinite`] when any number is not finite.
pub fn reject_non_finite_json5(json5: &biscuit_file::Json5) -> Result<(), NumericAdmissionError> {
    match json5.deserialize_raw::<FiniteScan>() {
        Ok(FiniteScan) => Ok(()),
        Err(error) if error.to_string().contains(NON_FINITE_MARKER) => Err(NumericAdmissionError::NotFinite),
        // The document already parsed into a value; any other failure of the
        // second read is not a numeric one.
        Err(_) => Ok(()),
    }
}

const NON_FINITE_MARKER: &str = "darkmatter: non-finite number";

/// Accepts any value and fails only on a non-finite float.
struct FiniteScan;

impl<'de> Deserialize<'de> for FiniteScan {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(FiniteScan)
    }
}

impl<'de> Visitor<'de> for FiniteScan {
    type Value = Self;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON5 value")
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self, E> {
        if value.is_finite() {
            Ok(Self)
        } else {
            Err(E::custom(NON_FINITE_MARKER))
        }
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

    fn visit_i128<E: de::Error>(self, _: i128) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_u128<E: de::Error>(self, _: u128) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self, E> {
        Ok(Self)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self, D::Error> {
        Self::deserialize(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self, A::Error> {
        while access.next_element::<Self>()?.is_some() {}
        Ok(Self)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self, A::Error> {
        while access.next_entry::<IgnoredAny, Self>()?.is_some() {}
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml_error(text: &str) -> String {
        load_admitted_yaml(text).expect_err(text).to_string()
    }

    #[test]
    fn yaml_inadmissible_numbers_fail_with_path_location_and_reason() {
        for (text, spelling, reason) in [
            ("n: 9007199254740993\n", "9007199254740993", "cannot be represented exactly"),
            ("n: -9007199254740993\n", "-9007199254740993", "cannot be represented exactly"),
            ("n: 9007199254740993.0\n", "9007199254740993.0", "cannot be represented exactly"),
            ("n: 9.007199254740993e15\n", "9.007199254740993e15", "cannot be represented exactly"),
            ("n: 99999999999999999999\n", "99999999999999999999", "cannot be represented exactly"),
            ("n: 1e-400\n", "1e-400", "cannot be represented exactly: a non-zero value would become 0"),
            ("n: .inf\n", ".inf", "not finite"),
            ("n: -.inf\n", "-.inf", "not finite"),
            ("n: .nan\n", ".nan", "not finite"),
        ] {
            let message = yaml_error(text);
            assert!(message.starts_with("n: "), "{message}");
            assert!(message.contains(&format!("number {spelling} {reason}")), "{message}");
            assert!(message.contains("line 1 column 4"), "{message}");
        }
    }

    #[test]
    fn yaml_nested_numbers_name_their_path() {
        let message = yaml_error("a:\n  b:\n    - 1\n    - 1e-400\n");
        assert!(message.starts_with("a.b[1]: "), "{message}");
        assert!(message.contains("line 4 column 7"), "{message}");
    }

    #[test]
    fn yaml_admissible_numbers_load_unchanged() {
        let text = "a: 0.1\nb: 1e3\nc: 9007199254740992\nd: -9007199254740992\ne: .5\nf: 4.0\ng: \"9007199254740993\"\nh: [1.5, 2]\n";
        let value = load_admitted_yaml(text).unwrap();
        assert_eq!(value, serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text).unwrap());
    }

    #[test]
    fn yaml_exact_oversized_integer_keeps_the_parse_error() {
        // 2^64 is exact, so admission does not claim the value model's error.
        let error = load_admitted_yaml("n: 18446744073709551616\n").unwrap_err();
        assert!(!is_yaml_admission_error(&error), "{error}");
    }

    #[test]
    fn json_numbers_are_admitted_from_their_spelling() {
        let text = "{\n  \"a\": 0.1,\n  \"b\": [1, 9007199254740993.0],\n  \"c\": 9007199254740992\n}";
        let error = admit_json_text_numbers(text).unwrap_err();
        assert_eq!(error.pointer, "/b/1");
        assert_eq!(error.spelling, "9007199254740993.0");
        assert_eq!(error.reason, NumericAdmissionError::Inexact);
        assert_eq!((error.line, error.column), (3, 12));
        assert_eq!(
            error.to_string(),
            "/b/1: number 9007199254740993.0 cannot be represented exactly at line 3 column 12"
        );
        assert_eq!(admit_json_text_numbers("{\"m\": 1e-400}").unwrap_err().reason, NumericAdmissionError::Underflow);
        assert_eq!(admit_json_text_numbers("{\"a/b\": [9007199254740993]}").unwrap_err().pointer, "/a~1b/0");
        assert!(admit_json_text_numbers("{\"a\": 0.1, \"b\": -9007199254740992, \"c\": \"1e400\"}").is_ok());
    }

    #[test]
    fn json5_non_finite_numbers_are_rejected() {
        for text in ["{ n: Infinity }", "{ n: -Infinity }", "{ n: NaN }", "{ n: 1e400 }", "[1, { m: [Infinity] }]"] {
            let json5 = biscuit_file::Json5::from_str(text).unwrap();
            assert_eq!(reject_non_finite_json5(&json5), Err(NumericAdmissionError::NotFinite), "{text}");
        }
        for text in ["{ n: 1.5, s: 'Infinity' }", "null", "{ n: null }", "[0x10, 1e300]"] {
            let json5 = biscuit_file::Json5::from_str(text).unwrap();
            assert_eq!(reject_non_finite_json5(&json5), Ok(()), "{text}");
        }
    }
}
