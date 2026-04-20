//! Output formatting for the `question` CLI.
//!
//! Each subcommand produces one or more string values after a
//! successful submission. The global `--output` flag controls how they
//! are serialised to stdout.

use std::io::{self, Write};

use clap::ValueEnum;

/// Output serialisation mode selected via `--output`.
///
/// ## Notes
///
/// - [`OutputMode::Raw`] — the per-component default: scalars emit
///   `value\n`; `ChooseMany` emits newline-separated values.
/// - [`OutputMode::Json`] — JSON for every component. A scalar becomes
///   a JSON string; a list becomes a JSON array.
/// - [`OutputMode::Null`] — NUL-separated values for multi-value
///   outputs (pairs with `xargs -0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
#[clap(rename_all = "kebab-case")]
pub enum OutputMode {
    /// Plain text, one value per line.
    #[default]
    Raw,
    /// JSON-encoded value.
    Json,
    /// NUL-separated list (mainly useful for multi-value outputs).
    Null,
}

/// Writes a scalar submission result to `writer` per `mode`.
///
/// ## Errors
///
/// Returns any I/O error produced while writing to `writer`.
pub fn write_scalar<W: Write>(writer: &mut W, value: &str, mode: OutputMode) -> io::Result<()> {
    match mode {
        OutputMode::Raw => {
            writer.write_all(value.as_bytes())?;
            writer.write_all(b"\n")
        }
        OutputMode::Json => {
            let encoded = serde_json::to_string(value).unwrap_or_else(|_| String::from("null"));
            writer.write_all(encoded.as_bytes())?;
            writer.write_all(b"\n")
        }
        OutputMode::Null => {
            writer.write_all(value.as_bytes())?;
            writer.write_all(&[0])
        }
    }
}

/// Writes a list of submitted values to `writer` per `mode`.
///
/// ## Returns
///
/// An empty list emits nothing in `Raw`/`Null` modes and `[]` in
/// `Json` mode.
///
/// ## Errors
///
/// Returns any I/O error produced while writing to `writer`.
pub fn write_list<W: Write>(writer: &mut W, values: &[String], mode: OutputMode) -> io::Result<()> {
    match mode {
        OutputMode::Raw => {
            for value in values {
                writer.write_all(value.as_bytes())?;
                writer.write_all(b"\n")?;
            }
            Ok(())
        }
        OutputMode::Json => {
            let encoded = serde_json::to_string(values).unwrap_or_else(|_| String::from("[]"));
            writer.write_all(encoded.as_bytes())?;
            writer.write_all(b"\n")
        }
        OutputMode::Null => {
            for value in values {
                writer.write_all(value.as_bytes())?;
                writer.write_all(&[0])?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture_scalar(mode: OutputMode, value: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        write_scalar(&mut buf, value, mode).expect("write");
        buf
    }

    fn capture_list(mode: OutputMode, values: &[&str]) -> Vec<u8> {
        let owned: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        let mut buf = Vec::new();
        write_list(&mut buf, &owned, mode).expect("write");
        buf
    }

    #[test]
    fn raw_mode_appends_newline() {
        let bytes = capture_scalar(OutputMode::Raw, "hello");
        assert_eq!(bytes, b"hello\n");
    }

    #[test]
    fn json_mode_quotes_and_escapes() {
        let bytes = capture_scalar(OutputMode::Json, "a\"b");
        assert_eq!(bytes, b"\"a\\\"b\"\n");
    }

    #[test]
    fn null_mode_terminates_with_nul_byte() {
        let bytes = capture_scalar(OutputMode::Null, "x");
        assert_eq!(bytes, b"x\0");
    }

    #[test]
    fn raw_mode_handles_empty_value() {
        let bytes = capture_scalar(OutputMode::Raw, "");
        assert_eq!(bytes, b"\n");
    }

    #[test]
    fn write_list_raw_emits_one_line_per_value() {
        let bytes = capture_list(OutputMode::Raw, &["a", "b", "c"]);
        assert_eq!(bytes, b"a\nb\nc\n");
    }

    #[test]
    fn write_list_json_emits_json_array() {
        let bytes = capture_list(OutputMode::Json, &["a", "b"]);
        assert_eq!(bytes, b"[\"a\",\"b\"]\n");
    }

    #[test]
    fn write_list_null_emits_nul_separated_values() {
        let bytes = capture_list(OutputMode::Null, &["a", "b"]);
        assert_eq!(bytes, b"a\0b\0");
    }

    #[test]
    fn write_list_empty_in_raw_is_empty() {
        let bytes = capture_list(OutputMode::Raw, &[]);
        assert_eq!(bytes, b"");
    }

    #[test]
    fn write_list_empty_in_json_is_empty_array() {
        let bytes = capture_list(OutputMode::Json, &[]);
        assert_eq!(bytes, b"[]\n");
    }
}
