//! Strict JSON-with-comments parsing for `bun.lock`, `rush.json`, and Rush's
//! other configuration files.
//!
//! These files allow `//` and `/* */` comments and trailing commas, and
//! nothing else beyond JSON. `jsonc-parser`'s defaults are far looser, so the
//! options are spelled out (spike S2 of `2026-09-26-lockfile-corroboration`).

use serde::de::DeserializeOwned;

/// Comments and trailing commas only. A literal, not `Default`: the crate's
/// defaults accept single quotes, unquoted keys, and missing commas, and a new
/// option in a future release breaks this build rather than being inherited.
const OPTIONS: jsonc_parser::ParseOptions = jsonc_parser::ParseOptions {
    allow_comments: true,
    allow_trailing_commas: true,
    allow_loose_object_property_names: false,
    allow_missing_commas: false,
    allow_single_quoted_strings: false,
    allow_hexadecimal_numbers: false,
    allow_unary_plus_numbers: false,
};

/// Deserialize `content` straight from the scanner, with no intermediate
/// value tree. Trailing content after the single root value is an error.
pub(crate) fn from_str<T: DeserializeOwned>(content: &str) -> Result<T, String> {
    jsonc_parser::parse_to_serde_value(content, &OPTIONS).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Probe {
        a: u32,
    }

    #[test]
    fn accepts_comments_and_trailing_commas() {
        let content = "// lead\n{ /* inline */ \"a\": 1, }\n// trail\n";
        assert_eq!(from_str::<Probe>(content), Ok(Probe { a: 1 }));
    }

    #[test]
    fn rejects_everything_beyond_comments_and_trailing_commas() {
        for (label, content) in [
            ("single quotes", "{ 'a': 1 }"),
            ("unquoted key", "{ a: 1 }"),
            ("missing comma", "{ \"a\": 1 \"b\": 2 }"),
            ("hexadecimal", "{ \"a\": 0x1 }"),
            ("unary plus", "{ \"a\": +1 }"),
            ("trailing garbage", "{ \"a\": 1 } }}} x"),
            ("second document", "{ \"a\": 1 } {}"),
            ("unterminated block comment", "{ \"a\": 1 } /*"),
            ("empty input", ""),
        ] {
            assert!(from_str::<Probe>(content).is_err(), "{label}");
        }
    }
}
