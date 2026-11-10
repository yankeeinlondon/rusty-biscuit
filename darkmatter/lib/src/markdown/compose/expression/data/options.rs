//! `load_data`'s options object, `{format?, delimiter?, header?}`.
//!
//! The shared binder decides the object's shape (closed keys, field types,
//! the `format` enum, `null`); this module decides what a bound value means
//! and whether it can apply to the call's effective format. An alias
//! (`yml`, `ndjson`) is normalized here and never seen again: diagnostics and
//! memo keys name only the canonical [`DataFormat`]. Every message comes from
//! [`invalid_options_message`], so passive checking and execution report the
//! same text.

use biscuit_file::DataFormat;
use serde_json::Value;

use super::error::{DataError, format_name};

/// The reason an unusable `delimiter` value is refused.
const DELIMITER_REASON: &str =
    "must be one ASCII character other than a double quote, CR, or LF (write a tab as \"\\t\")";

/// The options a call chose; `None` is "not given" (absent or `null`), which
/// [`effective_delimiter`] and [`effective_header`] turn into the format's
/// default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DataOptions {
    pub(crate) format: Option<DataFormat>,
    pub(crate) delimiter: Option<u8>,
    pub(crate) header: Option<bool>,
}

impl DataOptions {
    /// The options a bound third argument holds. An omitted argument,
    /// `null`, and `{}` are the defaults, as is each key that is absent or
    /// `null`.
    ///
    /// ## Errors
    ///
    /// [`DataError::InvalidOptions`] for a value the binder should have
    /// refused (a non-object, an unknown key, a wrongly typed field, an
    /// unknown format name) and for a delimiter [`delimiter_byte`] refuses.
    pub(crate) fn from_bound(value: Option<&Value>) -> Result<Self, DataError> {
        let members = match value {
            None | Some(Value::Null) => return Ok(Self::default()),
            Some(Value::Object(members)) => members,
            Some(other) => {
                return Err(invalid("options", format!("must be an object, but received {}", kind_of(other))));
            }
        };
        let mut options = Self::default();
        for (key, value) in members {
            match (key.as_str(), value) {
                (_, Value::Null) if matches!(key.as_str(), "format" | "delimiter" | "header") => {}
                ("format", Value::String(name)) => {
                    options.format = Some(format_from_name(name).ok_or_else(|| {
                        invalid("format", format!("`{name}` is not a supported format; {}", FORMAT_NAMES))
                    })?);
                }
                ("delimiter", Value::String(text)) => options.delimiter = Some(delimiter_byte(text)?),
                ("header", Value::Bool(header)) => options.header = Some(*header),
                ("format", other) => return Err(invalid("format", format!("must be a string, but received {}", kind_of(other)))),
                ("delimiter", other) => {
                    return Err(invalid("delimiter", format!("must be a string, but received {}", kind_of(other))));
                }
                ("header", other) => {
                    return Err(invalid("header", format!("must be a boolean, but received {}", kind_of(other))));
                }
                (unknown, _) => {
                    return Err(invalid("options", format!("`{unknown}` is not an option; use `format`, `delimiter`, or `header`")));
                }
            }
        }
        Ok(options)
    }

    /// Whether these options can apply to data read as `format`, the call's
    /// effective format (chosen by `format` or by the file's extension).
    ///
    /// ## Errors
    ///
    /// [`DataError::InvalidOptions`] for a `delimiter`, then a `header`
    /// (`true` included), on a format other than CSV and TSV, and for a TSV
    /// delimiter other than a tab.
    pub(crate) fn check_applicability(&self, format: DataFormat) -> Result<(), DataError> {
        match format {
            DataFormat::Csv => Ok(()),
            DataFormat::Tsv => match self.delimiter {
                Some(delimiter) if delimiter != b'\t' => Err(invalid(
                    "delimiter",
                    "TSV data is tab-separated, so only \"\\t\" is accepted; read data separated by another character with format \"csv\"",
                )),
                _ => Ok(()),
            },
            _ => {
                let key = if self.delimiter.is_some() {
                    "delimiter"
                } else if self.header.is_some() {
                    "header"
                } else {
                    return Ok(());
                };
                Err(invalid(
                    key,
                    format!(
                        "applies only to CSV and TSV data, but this call reads {}; remove it or select a tabular format",
                        format_name(format)
                    ),
                ))
            }
        }
    }
}

/// Every accepted `format` name, for messages.
const FORMAT_NAMES: &str = "use one of yaml, yml, json, jsonl, ndjson, csv, tsv, toml, json5";

/// The format `name` selects: an exact, case-sensitive match of a canonical
/// name or an alias (`yml` is YAML, `ndjson` is JSON Lines). `None` for any
/// other text, untrimmed or differently cased spellings included.
pub(crate) fn format_from_name(name: &str) -> Option<DataFormat> {
    match name {
        "yaml" | "yml" => Some(DataFormat::Yaml),
        "json" => Some(DataFormat::Json),
        "jsonl" | "ndjson" => Some(DataFormat::Jsonl),
        "csv" => Some(DataFormat::Csv),
        "tsv" => Some(DataFormat::Tsv),
        "toml" => Some(DataFormat::Toml),
        "json5" => Some(DataFormat::Json5),
        _ => None,
    }
}

/// The byte a `delimiter` value names: exactly one ASCII character other
/// than a double quote, CR, or LF (the set the delimited reader accepts).
///
/// ## Errors
///
/// [`DataError::InvalidOptions`] for `delimiter` otherwise.
pub(crate) fn delimiter_byte(text: &str) -> Result<u8, DataError> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(only), None) if only.is_ascii() && !matches!(only, '"' | '\r' | '\n') => Ok(only as u8),
        _ => Err(invalid("delimiter", DELIMITER_REASON)),
    }
}

/// The delimiter a `format` read uses: the chosen one or a comma for CSV, a
/// tab for TSV, none for other formats. Omitted and explicit defaults are
/// equal, so they share memo entries.
pub(crate) fn effective_delimiter(format: DataFormat, chosen: Option<u8>) -> Option<u8> {
    match format {
        DataFormat::Csv => Some(chosen.unwrap_or(b',')),
        DataFormat::Tsv => Some(b'\t'),
        _ => None,
    }
}

/// Whether a `format` read treats its first record as a header: the chosen
/// value or `true` for CSV and TSV, none for other formats.
pub(crate) fn effective_header(format: DataFormat, chosen: Option<bool>) -> Option<bool> {
    match format {
        DataFormat::Csv | DataFormat::Tsv => Some(chosen.unwrap_or(true)),
        _ => None,
    }
}

/// The message of an invalid option, shared by [`DataError`]'s display and
/// the passive call check.
pub(crate) fn invalid_options_message(key: &str, reason: &str) -> String {
    format!("load_data(): option `{key}` is invalid: {reason}")
}

fn invalid(key: &'static str, reason: impl Into<String>) -> DataError {
    DataError::InvalidOptions { key, reason: reason.into() }
}

fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TREE_FORMATS: [DataFormat; 5] =
        [DataFormat::Yaml, DataFormat::Json, DataFormat::Jsonl, DataFormat::Toml, DataFormat::Json5];

    fn key_of(error: &DataError) -> &'static str {
        match error {
            DataError::InvalidOptions { key, .. } => key,
            other => panic!("not an options error: {other}"),
        }
    }

    #[test]
    fn defaults_from_absent_null_and_empty() {
        for value in [None, Some(json!(null)), Some(json!({})), Some(json!({"format": null, "delimiter": null, "header": null}))] {
            assert_eq!(DataOptions::from_bound(value.as_ref()).unwrap(), DataOptions::default(), "{value:?}");
        }
        let chosen = DataOptions::from_bound(Some(&json!({"format": "csv", "delimiter": ";", "header": false}))).unwrap();
        assert_eq!(chosen, DataOptions { format: Some(DataFormat::Csv), delimiter: Some(b';'), header: Some(false) });
    }

    #[test]
    fn aliases_normalize_to_canonical_formats() {
        assert_eq!(format_from_name("yml"), Some(DataFormat::Yaml));
        assert_eq!(format_from_name("ndjson"), Some(DataFormat::Jsonl));
        assert_eq!(format_from_name("yml"), format_from_name("yaml"));
        assert_eq!(format_from_name("ndjson"), format_from_name("jsonl"));
        let options = DataOptions::from_bound(Some(&json!({"format": "yml"}))).unwrap();
        assert_eq!(options, DataOptions::from_bound(Some(&json!({"format": "yaml"}))).unwrap());
        // The alias is gone: a later failure names the canonical format.
        let error = DataOptions { header: Some(true), ..options }.check_applicability(options.format.unwrap()).unwrap_err();
        assert!(error.to_string().contains("this call reads YAML"), "{error}");
    }

    #[test]
    fn format_names_are_exact() {
        for name in ["yaml", "json", "jsonl", "csv", "tsv", "toml", "json5"] {
            assert!(format_from_name(name).is_some(), "{name}");
        }
        for name in ["CSV", " csv", "csv ", "", "text", "markdown", "YML"] {
            assert_eq!(format_from_name(name), None, "{name:?}");
        }
    }

    #[test]
    fn valid_delimiters() {
        for (text, byte) in [(",", b','), (";", b';'), ("|", b'|'), ("\t", b'\t'), ("5", b'5')] {
            assert_eq!(delimiter_byte(text).unwrap(), byte, "{text:?}");
        }
    }

    #[test]
    fn invalid_delimiters_name_the_key() {
        for text in ["", ",,", "é", "\"", "\r", "\n", "\r\n"] {
            let error = delimiter_byte(text).unwrap_err();
            assert_eq!(key_of(&error), "delimiter", "{text:?}");
            assert_eq!(error.to_string(), invalid_options_message("delimiter", DELIMITER_REASON), "{text:?}");
            let bound = DataOptions::from_bound(Some(&json!({"delimiter": text}))).unwrap_err();
            assert_eq!(bound.to_string(), error.to_string());
        }
    }

    #[test]
    fn tabular_options_on_tree_formats_are_invalid() {
        for format in TREE_FORMATS {
            let delimiter = DataOptions { delimiter: Some(b','), ..DataOptions::default() };
            let error = delimiter.check_applicability(format).unwrap_err();
            assert_eq!(key_of(&error), "delimiter");
            assert!(error.to_string().contains(format_name(format)), "{error}");
            let header = DataOptions { header: Some(true), ..DataOptions::default() };
            assert_eq!(key_of(&header.check_applicability(format).unwrap_err()), "header");
            // A delimiter is reported before a header.
            let both = DataOptions { delimiter: Some(b';'), header: Some(false), ..DataOptions::default() };
            assert_eq!(key_of(&both.check_applicability(format).unwrap_err()), "delimiter");
            assert!(DataOptions::default().check_applicability(format).is_ok());
        }
    }

    #[test]
    fn tsv_accepts_only_a_tab_delimiter() {
        for delimiter in *b",;|" {
            let options = DataOptions { delimiter: Some(delimiter), ..DataOptions::default() };
            let error = options.check_applicability(DataFormat::Tsv).unwrap_err();
            assert_eq!(key_of(&error), "delimiter");
            assert!(error.to_string().contains("format \"csv\""), "{error}");
        }
        for options in [
            DataOptions::default(),
            DataOptions { delimiter: Some(b'\t'), ..DataOptions::default() },
            DataOptions { header: Some(false), ..DataOptions::default() },
        ] {
            assert!(options.check_applicability(DataFormat::Tsv).is_ok(), "{options:?}");
        }
    }

    #[test]
    fn csv_with_tab_stays_csv() {
        let options = DataOptions::from_bound(Some(&json!({"format": "csv", "delimiter": "\t"}))).unwrap();
        assert_eq!(options.format, Some(DataFormat::Csv));
        assert!(options.check_applicability(DataFormat::Csv).is_ok());
        assert_eq!(effective_delimiter(DataFormat::Csv, options.delimiter), Some(b'\t'));
    }

    #[test]
    fn effective_values_share_defaults() {
        assert_eq!(effective_delimiter(DataFormat::Csv, None), effective_delimiter(DataFormat::Csv, Some(b',')));
        assert_eq!(effective_delimiter(DataFormat::Tsv, None), effective_delimiter(DataFormat::Tsv, Some(b'\t')));
        assert_eq!(effective_delimiter(DataFormat::Tsv, None), Some(b'\t'));
        assert_eq!(effective_header(DataFormat::Csv, None), effective_header(DataFormat::Csv, Some(true)));
        assert_eq!(effective_header(DataFormat::Tsv, Some(false)), Some(false));
        for format in TREE_FORMATS {
            assert_eq!(effective_delimiter(format, None), None);
            assert_eq!(effective_header(format, None), None);
        }
    }

    #[test]
    fn unbound_shapes_are_invalid_options() {
        let cases = [
            (json!(5), "options"),
            (json!("csv"), "options"),
            (json!([]), "options"),
            (json!(true), "options"),
            (json!({"quote": ","}), "options"),
            (json!({"format": 5}), "format"),
            (json!({"format": ["csv"]}), "format"),
            (json!({"format": "CSV"}), "format"),
            (json!({"format": ""}), "format"),
            (json!({"delimiter": 5}), "delimiter"),
            (json!({"delimiter": [","]}), "delimiter"),
            (json!({"delimiter": {}}), "delimiter"),
            (json!({"header": "false"}), "header"),
            (json!({"header": [1]}), "header"),
            (json!({"header": 0}), "header"),
        ];
        for (value, key) in cases {
            let error = DataOptions::from_bound(Some(&value)).unwrap_err();
            assert_eq!(key_of(&error), key, "{value}");
            assert_eq!(error.code(), Some(super::super::error::INVALID_OPTIONS_CODE), "{value}");
        }
    }
}
