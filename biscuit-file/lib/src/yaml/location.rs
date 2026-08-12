//! Structured YAML parse-location projection.
//!
//! [`serde_yaml_ng::Error`] carries byte index, line, and column data through
//! `Error::location()`, but only as an opaque value. [`YamlLocation`] projects
//! that data into a plain, serde-capable struct so diagnostics and repair
//! analysis never have to re-parse a rendered error string.

use serde::{Deserialize, Serialize};

/// Structured location of a YAML parse event.
///
/// This is a faithful projection of what the parser reports: `byte` is a
/// 0-indexed byte offset and `line` is 1-indexed, but `column` is the
/// parser's 1-indexed **character** column (libyaml counts characters, not
/// bytes, so the two diverge after multibyte content on the same line).
/// Consumers that need the shared 1-indexed **byte** column convention must
/// derive it from `byte` against the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct YamlLocation {
    /// 0-indexed byte offset into the source.
    pub byte: usize,
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed character column within the line, as reported by the parser.
    pub column: usize,
}

impl From<serde_yaml_ng::Location> for YamlLocation {
    fn from(location: serde_yaml_ng::Location) -> Self {
        Self {
            byte: location.index(),
            line: location.line(),
            column: location.column(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_from_parser_error() {
        let input = "key: [unclosed";
        let error = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(input).unwrap_err();
        let location = error.location().map(YamlLocation::from).unwrap();
        assert_eq!(location.line, 2);
        assert_eq!(location.byte, input.len());
    }
}
