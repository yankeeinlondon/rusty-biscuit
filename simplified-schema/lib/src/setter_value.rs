//! The one reader for a value written on a command line, as in
//! `md set doc.md count 5` or `md compose doc.md count=5`.
//!
//! Reading has two moments. [`SetterValue::parse`] decides the value's form
//! from its text alone and keeps a scalar's spelling, because `1.10` is only a
//! number once something says so. [`SetterValue::bind`] then reads it through
//! the type declared for its place, which only the host can look up.
//!
//! Which document and schema apply, and what happens to the result, stay host
//! policy.

use serde_json::Value;

use crate::coercion::{self, Shape};
use crate::numeric::{admit_json5_text_numbers, is_yaml_admission_error, load_admitted_yaml};

/// A command-line value whose form is known and whose type is not yet.
#[derive(Debug, Clone, PartialEq)]
pub enum SetterValue {
    /// Unquoted scalar text, spelled as written: `1.10`, `true`, `Alice`, or a
    /// value no parser accepted.
    Text {
        /// The value exactly as written.
        raw: String,
    },
    /// An explicitly JSON5-quoted string, decoded once.
    Quoted(String),
    /// The JSON5 literal `null`.
    Null,
    /// An admitted object or array.
    Container(Value),
}

/// A value whose text parsed as a container that cannot be admitted. It never
/// falls back to text, because the author plainly wrote structure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SetterValueError {
    /// A key appears twice in one object.
    #[error("duplicate key `{key}` in the value")]
    DuplicateKey {
        /// The repeated key.
        key: String,
    },
    /// A number is non-finite, or no `f64` holds it exactly.
    #[error("invalid number in the value: {message}")]
    Number {
        /// Names the number, where it is, and why it failed.
        message: String,
    },
}

impl SetterValue {
    /// Reads a value's form from its text.
    ///
    /// JSON5 reads the whole text first. A value starting with `{` that JSON5
    /// rejects is tried as one YAML flow mapping, so `{ max-width: 50 }` needs
    /// no quoted key. JSON5 goes first because YAML reads `{a:1}` as a key named
    /// `a:1`. Anything else is text, including a tagged YAML value, a block
    /// document, and a `{{ template }}`. Empty input is the empty string.
    ///
    /// ## Examples
    ///
    /// ```
    /// use serde_json::json;
    /// use simplified_schema::SetterValue;
    ///
    /// assert_eq!(SetterValue::parse("1.10").unwrap(), SetterValue::Text { raw: "1.10".into() });
    /// assert_eq!(SetterValue::parse("\"Alice\"").unwrap(), SetterValue::Quoted("Alice".into()));
    /// assert_eq!(
    ///     SetterValue::parse("{ max-width: 50 }").unwrap(),
    ///     SetterValue::Container(json!({ "max-width": 50 })),
    /// );
    /// ```
    ///
    /// ## Errors
    ///
    /// A [`SetterValueError`] for a parsed container with a duplicate key, a
    /// non-finite number, an inexact integer, or an underflow.
    pub fn parse(raw: &str) -> Result<Self, SetterValueError> {
        match biscuit_file::Json5::from_str(raw) {
            Ok(parsed) => match parsed.value() {
                Value::Null if !is_inadmissible_number(raw) => Ok(Self::Null),
                Value::String(text) => Ok(Self::Quoted(text.clone())),
                container @ (Value::Object(_) | Value::Array(_)) => {
                    admit_json5_container(raw)?;
                    Ok(Self::Container(container.clone()))
                }
                _ => Ok(text(raw)),
            },
            // Grammatical, yet no value could be built: an integer beyond `u64`.
            // Inside a container that is an admission failure, and YAML must not
            // get to read `{a:99999999999999999999}` as a key. A bare one is text.
            Err(_) if biscuit_file::json5_is_well_formed(raw) => match admit_json5_text_numbers(raw) {
                Err(error) if !error.pointer.is_empty() => Err(SetterValueError::Number {
                    message: error.to_string(),
                }),
                _ => Ok(text(raw)),
            },
            Err(_) if raw.trim_start().starts_with('{') => match read_yaml_flow_mapping(raw)? {
                Some(container) => Ok(Self::Container(container)),
                None => Ok(text(raw)),
            },
            Err(_) => Ok(text(raw)),
        }
    }

    /// The final value, read through the type declared for its place.
    ///
    /// Under a declared type, text goes through the conversion engine as text:
    /// `1.10` under `string` stays `"1.10"`, and `5` under `number` is `5`. A
    /// parsed container binds recursively. A value that does not fit is returned
    /// unconverted, so schema validation is what reports it. With no declaration,
    /// or one that declares no structure (`unknown`), text is guessed from its
    /// JSON5 reading: `5` is the number `5`. A number admission would reject stays
    /// text there. Under `boolish`, which keeps a boolean as sent, an unquoted
    /// `true` or `false` is sent as that boolean, while `On` stays text.
    ///
    /// ## Examples
    ///
    /// ```
    /// use serde_json::json;
    /// use simplified_schema::SetterValue;
    /// use simplified_schema::coercion::Shape;
    ///
    /// let value = SetterValue::parse("1.10").unwrap();
    /// assert_eq!(value.bind(Some(&Shape::text())), json!("1.10"));
    /// assert_eq!(value.bind(Some(&Shape::number())), json!(1.1));
    /// assert_eq!(value.bind(None), json!(1.1));
    /// ```
    pub fn bind(&self, declared: Option<&Shape>) -> Value {
        let declared = declared.filter(|shape| !declares_nothing(shape));
        match (self, declared) {
            (Self::Text { raw }, None) => guess_scalar(raw),
            (Self::Text { raw }, Some(shape)) if keeps_booleans_as_sent(shape) && matches!(raw.as_str(), "true" | "false") => {
                Value::Bool(raw == "true")
            }
            (Self::Text { raw }, Some(shape)) => bound_or_original(shape, Value::String(raw.clone())),
            (Self::Quoted(text), _) => Value::String(text.clone()),
            (Self::Null, _) => Value::Null,
            (Self::Container(value), None) => value.clone(),
            (Self::Container(value), Some(shape)) => bound_or_original(shape, value.clone()),
        }
    }
}

fn text(raw: &str) -> SetterValue {
    SetterValue::Text { raw: raw.to_string() }
}

/// `NaN` and `Infinity` also read as a null `Value`, which cannot hold them, so
/// `null` must be told from them by its spelling.
fn is_inadmissible_number(raw: &str) -> bool {
    admit_json5_text_numbers(raw).is_err()
}

fn admit_json5_container(raw: &str) -> Result<(), SetterValueError> {
    if let Ok(scan) = biscuit_file::json5_scan(raw)
        && let Some(duplicate) = scan.duplicate_key
    {
        return Err(SetterValueError::DuplicateKey { key: duplicate.key });
    }
    admit_json5_text_numbers(raw).map_err(|error| SetterValueError::Number {
        message: error.to_string(),
    })
}

fn declares_nothing(shape: &Shape) -> bool {
    match shape {
        Shape::Any => true,
        Shape::Deferrable(inner) => declares_nothing(inner),
        _ => false,
    }
}

fn keeps_booleans_as_sent(shape: &Shape) -> bool {
    match shape {
        Shape::Boolean { as_sent } => *as_sent,
        Shape::Assert(inner, _) | Shape::Deferrable(inner) => keeps_booleans_as_sent(inner),
        _ => false,
    }
}

fn bound_or_original(shape: &Shape, value: Value) -> Value {
    match coercion::bind(shape, &value) {
        Ok(bound) => bound.into_value(),
        Err(_) => value,
    }
}

fn guess_scalar(raw: &str) -> Value {
    match biscuit_file::Json5::from_str(raw) {
        Ok(parsed) if admit_json5_text_numbers(raw).is_ok() => parsed.value().clone(),
        _ => Value::String(raw.to_string()),
    }
}

/// `raw` as one YAML flow mapping of plain data; `None` when it is not one.
fn read_yaml_flow_mapping(raw: &str) -> Result<Option<Value>, SetterValueError> {
    if !is_one_plain_flow_mapping(raw) {
        return Ok(None);
    }
    match load_admitted_yaml(raw) {
        Ok(value) => Ok(json_from_flow_yaml(&value)),
        Err(error) if is_yaml_admission_error(&error) => Err(SetterValueError::Number {
            message: error.to_string(),
        }),
        Err(error) => match duplicate_yaml_key(&error.to_string()) {
            Some(key) => Err(SetterValueError::DuplicateKey { key }),
            None => Ok(None),
        },
    }
}

/// The key named by `serde_yaml_ng`'s duplicate-entry error, which is the only
/// way it reports one.
fn duplicate_yaml_key(message: &str) -> Option<String> {
    let (_, rest) = message.split_once("duplicate entry with key \"")?;
    let (key, _) = rest.split_once('"')?;
    Some(key.to_string())
}

/// Whether `raw` is a single flow mapping with nothing after it but whitespace
/// or a comment, and no tag, anchor, or alias at any node.
///
/// The value tree cannot answer this: `{a: 1}: x` is a block document and
/// `{a: !!str 1}` has already been resolved by the time it is a value, so this
/// reads the text. A quote or indicator counts only where a node starts, since
/// `{a: it's}` and `{a: b!c}` are plain scalars.
fn is_one_plain_flow_mapping(raw: &str) -> bool {
    let body = raw.trim_start();
    let mut depth = 0usize;
    let mut at_node_start = true;
    let mut after_quoted = false;
    let mut characters = body.char_indices().peekable();
    while let Some((offset, character)) = characters.next() {
        match character {
            '"' | '\'' if at_node_start => {
                if !skip_quoted(&mut characters, character) {
                    return false;
                }
                at_node_start = false;
                after_quoted = true;
                continue;
            }
            '!' | '&' | '*' if at_node_start => return false,
            '{' | '[' if at_node_start => {
                depth += 1;
            }
            '}' | ']' => {
                let Some(remaining) = depth.checked_sub(1) else {
                    return false;
                };
                depth = remaining;
                if depth == 0 {
                    return only_trivia(&body[offset + character.len_utf8()..]);
                }
                at_node_start = false;
                after_quoted = false;
                continue;
            }
            ',' => at_node_start = true,
            '#' if preceded_by_whitespace(body, offset) => {
                while characters.next_if(|(_, next)| *next != '\n').is_some() {}
            }
            ':' if after_quoted || characters.peek().is_none_or(|(_, next)| next.is_whitespace()) => {
                at_node_start = true;
            }
            space if space.is_whitespace() => {}
            '?' if at_node_start => {}
            _ => at_node_start = false,
        }
        after_quoted = false;
    }
    false
}

/// Consumes a quoted scalar through its closing quote; `false` when it never
/// closes.
fn skip_quoted(characters: &mut std::iter::Peekable<std::str::CharIndices<'_>>, quote: char) -> bool {
    while let Some((_, character)) = characters.next() {
        match (quote, character) {
            ('"', '\\') => {
                characters.next();
            }
            ('\'', '\'') if characters.next_if(|(_, next)| *next == '\'').is_some() => {}
            _ if character == quote => return true,
            _ => {}
        }
    }
    false
}

fn preceded_by_whitespace(body: &str, offset: usize) -> bool {
    body[..offset].chars().next_back().is_some_and(char::is_whitespace)
}

fn only_trivia(rest: &str) -> bool {
    rest.lines().all(|line| {
        let line = line.trim_start();
        line.is_empty() || line.starts_with('#')
    })
}

/// The JSON value for an admitted flow mapping; `None` when a key is itself a
/// container, as the `{ x: null }` key of `{{ x }}` is. No JSON value holds one,
/// and such a value is a template rather than data.
fn json_from_flow_yaml(value: &biscuit_file::serde_yaml_ng::Value) -> Option<Value> {
    use biscuit_file::serde_yaml_ng::Value as Yaml;
    match value {
        Yaml::Null => Some(Value::Null),
        Yaml::Bool(flag) => Some(Value::Bool(*flag)),
        Yaml::Number(_) => serde_json::to_value(value).ok(),
        Yaml::String(text) => Some(Value::String(text.clone())),
        Yaml::Sequence(items) => items.iter().map(json_from_flow_yaml).collect::<Option<_>>().map(Value::Array),
        Yaml::Mapping(mapping) => {
            let mut object = serde_json::Map::with_capacity(mapping.len());
            for (key, child) in mapping {
                object.insert(flow_yaml_key(key)?, json_from_flow_yaml(child)?);
            }
            Some(Value::Object(object))
        }
        Yaml::Tagged(_) => None,
    }
}

/// A scalar key as text, the way a segment names it: `{1: a}` has the key `"1"`.
fn flow_yaml_key(key: &biscuit_file::serde_yaml_ng::Value) -> Option<String> {
    use biscuit_file::serde_yaml_ng::Value as Yaml;
    match key {
        Yaml::String(text) => Some(text.clone()),
        Yaml::Number(number) => Some(number.to_string()),
        Yaml::Bool(flag) => Some(flag.to_string()),
        Yaml::Null => Some("null".to_string()),
        Yaml::Sequence(_) | Yaml::Mapping(_) | Yaml::Tagged(_) => None,
    }
}
