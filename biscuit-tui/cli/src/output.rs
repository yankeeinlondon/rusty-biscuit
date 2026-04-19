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

#[cfg(test)]
mod tests {
    use super::*;

    fn capture(mode: OutputMode, value: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        write_scalar(&mut buf, value, mode).expect("write");
        buf
    }

    #[test]
    fn raw_mode_appends_newline() {
        let bytes = capture(OutputMode::Raw, "hello");
        assert_eq!(bytes, b"hello\n");
    }

    #[test]
    fn json_mode_quotes_and_escapes() {
        let bytes = capture(OutputMode::Json, "a\"b");
        assert_eq!(bytes, b"\"a\\\"b\"\n");
    }

    #[test]
    fn null_mode_terminates_with_nul_byte() {
        let bytes = capture(OutputMode::Null, "x");
        assert_eq!(bytes, b"x\0");
    }

    #[test]
    fn raw_mode_handles_empty_value() {
        let bytes = capture(OutputMode::Raw, "");
        assert_eq!(bytes, b"\n");
    }
}
