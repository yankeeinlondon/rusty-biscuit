//! The on-disk literal token: one frontmatter string stored as data.
//!
//! `{{!data:v1:<payload>}}` holds a single string value, where `<payload>` is
//! the value's UTF-8 bytes in unpadded URL-safe base64. The empty string is
//! `{{!data:v1:}}`. The payload alphabet (`A-Z a-z 0-9 - _`) holds no brace,
//! quote, `$`, `:`, `#`, backslash, or whitespace, so a token can neither close
//! early, open `$(`, nor need YAML escaping.
//!
//! A token is recognized only as an **entire** frontmatter string leaf, byte
//! for byte with no surrounding whitespace. Composition decodes it once, and
//! the decoded text is data: it is never scanned for `{{ … }}`, `{{{ … }}}`, or
//! `$( … )`. `{{!data:` anywhere else the expression scanner looks (mixed
//! text, the body, a leaf with padding) is a malformed token and fails; it
//! never falls back to expression parsing. An author writes the spelling
//! literally with the `{{{ … }}}` escape.
//!
//! Loaders keep tokens encoded. A reader that needs ordinary text calls
//! [`decode_literal_tokens`] on loaded frontmatter; composition decodes on its
//! own.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::literal_token::{decode, encode, encode_yaml_scalar};
//!
//! let token = encode("fixed {{ area }}");
//! assert!(token.starts_with("{{!data:v1:"));
//! assert_eq!(decode(&token).unwrap(), "fixed {{ area }}");
//! assert_eq!(encode_yaml_scalar(""), "\"{{!data:v1:}}\"");
//! ```

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::Value;

use crate::markdown::compose::expression::ExpressionFinder;

/// The bytes every token starts with, version excluded.
pub const TOKEN_PREFIX: &str = "{{!data:";

/// The only token version this build reads and writes.
pub const TOKEN_VERSION: &str = "v1";

const TOKEN_SUFFIX: &str = "}}";

/// Why a string is not a valid literal token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    /// The text does not start with [`TOKEN_PREFIX`].
    #[error("not a literal token: it does not start with `{TOKEN_PREFIX}`")]
    NotAToken,
    /// No closing `}}`.
    #[error("the literal token has no closing `}}}}`")]
    Unterminated,
    /// No `:` between the version and the payload.
    #[error("the literal token has no version (expected `{TOKEN_PREFIX}{TOKEN_VERSION}:…}}}}`)")]
    MissingVersion,
    /// A version other than [`TOKEN_VERSION`].
    #[error("unsupported literal token version `{version}` (this build reads `{TOKEN_VERSION}`)")]
    UnsupportedVersion {
        /// The version as written.
        version: String,
    },
    /// The payload is not canonical unpadded URL-safe base64.
    #[error("the literal token payload is not unpadded URL-safe base64: {detail}")]
    InvalidPayload {
        /// The decoder's description of the first bad byte.
        detail: String,
    },
    /// The payload decodes to bytes that are not UTF-8.
    #[error("the literal token payload does not decode to UTF-8 text")]
    InvalidUtf8,
    /// A token spelling that is not an entire frontmatter string value.
    #[error(
        "a literal token must be an entire frontmatter string value; write `{{{{{{…}}}}}}` \
         around the spelling to show it as text"
    )]
    Embedded,
}

/// A malformed token found by [`decode_literal_tokens`], and where.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{path}: {error}")]
pub struct LiteralTokenError {
    /// Dotted path from the frontmatter root to the string, with `[n]` for an
    /// array index (`notes[2].text`).
    pub path: String,
    /// What is wrong with the token.
    pub error: TokenError,
}

/// The bare token for `value`.
#[must_use]
pub fn encode(value: &str) -> String {
    format!(
        "{TOKEN_PREFIX}{TOKEN_VERSION}:{}{TOKEN_SUFFIX}",
        URL_SAFE_NO_PAD.encode(value.as_bytes())
    )
}

/// The token for `value` as a double-quoted YAML scalar.
///
/// Always quoted: a plain scalar starting with `{` would read as a flow
/// mapping.
#[must_use]
pub fn encode_yaml_scalar(value: &str) -> String {
    format!("\"{}\"", encode(value))
}

/// The string a bare token holds.
///
/// ## Errors
///
/// Returns a [`TokenError`] unless `token` is exactly one well-formed token of
/// [`TOKEN_VERSION`] whose payload is UTF-8.
pub fn decode(token: &str) -> Result<String, TokenError> {
    let rest = token.strip_prefix(TOKEN_PREFIX).ok_or(TokenError::NotAToken)?;
    let rest = rest.strip_suffix(TOKEN_SUFFIX).ok_or(TokenError::Unterminated)?;
    let (version, payload) = rest.split_once(':').ok_or(TokenError::MissingVersion)?;
    if version != TOKEN_VERSION {
        return Err(TokenError::UnsupportedVersion {
            version: version.to_string(),
        });
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| TokenError::InvalidPayload {
            detail: error.to_string(),
        })?;
    String::from_utf8(bytes).map_err(|_| TokenError::InvalidUtf8)
}

/// Classifies one frontmatter string leaf.
///
/// - `None`: the leaf holds no token and is ordinary text.
/// - `Some(Ok(text))`: the whole leaf is a token holding `text`.
/// - `Some(Err(_))`: the leaf holds a malformed or embedded token.
///
/// Only text the scanner would otherwise read as an expression counts: a token
/// spelling inside `{{{ … }}}` or behind `\` is text.
#[must_use]
pub fn decode_leaf(value: &str) -> Option<Result<String, TokenError>> {
    let (span, whole) = first_token(value)?;
    Some(if whole { decode(&value[span]) } else { Err(TokenError::Embedded) })
}

/// The span of the first token the scanner finds in `value`, and whether it is
/// the entire value.
pub(crate) fn first_token(value: &str) -> Option<(std::ops::Range<usize>, bool)> {
    if !value.contains(TOKEN_PREFIX) {
        return None;
    }
    let scan = ExpressionFinder::scan_plain(value);
    let first = scan.tokens.first()?;
    let whole = scan.tokens.len() == 1 && first.start == 0 && first.end == value.len();
    Some((first.start..first.end, whole))
}

/// Whether `value` holds template (`{{`) or shell (`$(`) syntax a later
/// composition pass could still resolve.
///
/// A whole-leaf literal token is stored data, so it is never pending; a
/// malformed or embedded token still is, and composition reports it. This is
/// lexical only: a caller that knows a value is data must not ask.
#[must_use]
pub fn holds_pending_syntax(value: &str) -> bool {
    (value.contains("{{") || value.contains("$(")) && !matches!(decode_leaf(value), Some(Ok(_)))
}

/// `value` with every whole-leaf token replaced by the string it holds.
///
/// For readers of **loaded** frontmatter (schema checks, reports, handoffs to
/// other tools). Do not pass the result back to composition as authored text:
/// the decoded strings are data, and composing them would scan them.
///
/// ## Errors
///
/// Returns the first malformed or embedded token, in document order, with its
/// path.
pub fn decode_literal_tokens(value: &Value) -> Result<Value, LiteralTokenError> {
    decode_at(value, &mut String::new())
}

fn decode_at(value: &Value, path: &mut String) -> Result<Value, LiteralTokenError> {
    match value {
        Value::String(text) => match decode_leaf(text) {
            None => Ok(value.clone()),
            Some(Ok(decoded)) => Ok(Value::String(decoded)),
            Some(Err(error)) => Err(LiteralTokenError {
                path: path.clone(),
                error,
            }),
        },
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let len = path.len();
                path.push_str(&format!("[{index}]"));
                let decoded = decode_at(item, path);
                path.truncate(len);
                decoded
            })
            .collect::<Result<_, _>>()
            .map(Value::Array),
        Value::Object(map) => map
            .iter()
            .map(|(key, item)| {
                let len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(key);
                let decoded = decode_at(item, path);
                path.truncate(len);
                decoded.map(|decoded| (key.clone(), decoded))
            })
            .collect::<Result<_, _>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    /// Parses `key: <scalar>` the way Darkmatter loads frontmatter.
    fn yaml_value(scalar: &str) -> Value {
        let doc = format!("---\nkey: {scalar}\n---\n");
        crate::markdown::Markdown::from(doc.as_str())
            .frontmatter()
            .as_map()
            .get("key")
            .cloned()
            .expect("the key parses")
    }

    #[test]
    fn the_empty_string_has_an_empty_payload() {
        assert_eq!(encode(""), "{{!data:v1:}}");
        assert_eq!(decode("{{!data:v1:}}").unwrap(), "");
    }

    #[test]
    fn the_payload_is_unpadded_url_safe_base64() {
        // `>>>?` exercises both URL-safe digits (`-`, `_`); one byte would pad.
        assert_eq!(encode(">>>?"), "{{!data:v1:Pj4-Pw}}");
        assert_eq!(encode("a"), "{{!data:v1:YQ}}");
    }

    #[test]
    fn malformed_tokens_name_what_is_wrong() {
        let cases = [
            ("{{ area }}", TokenError::NotAToken),
            (" {{!data:v1:YQ}}", TokenError::NotAToken),
            ("{{!data:v1:YQ", TokenError::Unterminated),
            ("{{!data:v1:YQ}} ", TokenError::Unterminated),
            ("{{!data:YQ}}", TokenError::MissingVersion),
            (
                "{{!data:v2:YQ}}",
                TokenError::UnsupportedVersion {
                    version: "v2".into(),
                },
            ),
            ("{{!data:v1:_w}}", TokenError::InvalidUtf8),
        ];
        for (input, expected) in cases {
            assert_eq!(decode(input), Err(expected), "{input:?}");
        }
        for input in ["{{!data:v1:YQ==}}", "{{!data:v1:Y+}}", "{{!data:v1:Y}}", "{{!data:v1:YR}}"] {
            assert!(
                matches!(decode(input), Err(TokenError::InvalidPayload { .. })),
                "{input:?}: {:?}",
                decode(input)
            );
        }
    }

    #[test]
    fn a_leaf_is_a_token_only_when_it_is_the_whole_value() {
        assert_eq!(decode_leaf("plain"), None);
        assert_eq!(decode_leaf("{{ area }}"), None);
        assert_eq!(decode_leaf("{{!data:v1:YQ}}"), Some(Ok("a".into())));
        assert_eq!(decode_leaf("see {{!data:v1:YQ}}"), Some(Err(TokenError::Embedded)));
        assert_eq!(decode_leaf(" {{!data:v1:YQ}}"), Some(Err(TokenError::Embedded)));
        assert_eq!(decode_leaf("{{!data:v1:YQ}} "), Some(Err(TokenError::Embedded)));
        assert_eq!(decode_leaf("{{!data:v1:YQ}}{{!data:v1:YQ}}"), Some(Err(TokenError::Embedded)));
        assert_eq!(decode_leaf("{{!data:v1:YQ"), Some(Err(TokenError::Unterminated)));
        assert_eq!(
            decode_leaf("{{!data:v2:YQ}}"),
            Some(Err(TokenError::UnsupportedVersion { version: "v2".into() }))
        );
        // The author's escapes show the spelling as text.
        assert_eq!(decode_leaf("{{{!data:v1:YQ}}}"), None);
        assert_eq!(decode_leaf(r"see \{{!data:v1:YQ}}"), None);
    }

    #[test]
    fn decoding_a_tree_replaces_whole_leaves_and_keeps_shape() {
        let loaded = json!({
            "title": encode("Fix: colons"),
            "n": 3,
            "notes": [encode("$(echo X)"), "plain", {"deep": encode("{{ area }}")}],
            "empty": encode(""),
        });
        assert_eq!(
            decode_literal_tokens(&loaded).unwrap(),
            json!({
                "title": "Fix: colons",
                "n": 3,
                "notes": ["$(echo X)", "plain", {"deep": "{{ area }}"}],
                "empty": "",
            })
        );
    }

    #[test]
    fn a_malformed_leaf_in_a_tree_reports_its_path() {
        let loaded = json!({"ok": encode("x"), "notes": ["a", {"text": "{{!data:v9:YQ}}"}]});
        let error = decode_literal_tokens(&loaded).unwrap_err();
        assert_eq!(error.path, "notes[1].text");
        assert_eq!(
            error.error,
            TokenError::UnsupportedVersion {
                version: "v9".into()
            }
        );
    }

    #[test]
    fn the_yaml_scalar_loads_back_as_the_bare_token() {
        let scalar = encode_yaml_scalar("title: {{ x }}\n\"quoted\" $(rm -rf x) \\");
        assert_eq!(
            yaml_value(&scalar),
            Value::String(encode("title: {{ x }}\n\"quoted\" $(rm -rf x) \\"))
        );
    }

    /// Strings built to hit the codec's edges: braces, `$(`, escapes, line
    /// breaks, quotes, and token look-alikes, mixed with arbitrary Unicode.
    fn tricky_string() -> impl Strategy<Value = String> {
        let pieces = prop_oneof![
            Just("{{".to_string()),
            Just("}}".to_string()),
            Just("{{{".to_string()),
            Just("$(".to_string()),
            Just("\\".to_string()),
            Just("\n".to_string()),
            Just("\r\n".to_string()),
            Just("\"".to_string()),
            Just("'".to_string()),
            Just("{{!data:v1:".to_string()),
            Just(encode("nested")),
            Just("{{!data:v1:}}".to_string()),
            any::<String>(),
        ];
        prop::collection::vec(pieces, 0..8).prop_map(|parts| parts.concat())
    }

    proptest! {
        #[test]
        fn decode_inverts_encode(value in any::<String>()) {
            prop_assert_eq!(decode(&encode(&value)).unwrap(), value);
        }

        #[test]
        fn decode_inverts_encode_on_tricky_strings(value in tricky_string()) {
            let token = encode(&value);
            prop_assert_eq!(decode(&token).unwrap(), value.clone());
            prop_assert_eq!(decode_leaf(&token), Some(Ok(value)));
        }

        #[test]
        fn the_yaml_scalar_round_trips(value in tricky_string()) {
            let loaded = yaml_value(&encode_yaml_scalar(&value));
            prop_assert_eq!(decode_literal_tokens(&loaded).unwrap(), Value::String(value));
        }

        #[test]
        fn a_token_is_one_scanner_token_and_never_an_expression(value in tricky_string()) {
            let token = encode(&value);
            let scan = ExpressionFinder::scan_plain(&token);
            prop_assert!(scan.expressions.is_empty());
            prop_assert!(scan.literals.is_empty());
            prop_assert_eq!(scan.tokens.len(), 1);
            prop_assert_eq!(scan.tokens[0].start, 0);
            prop_assert_eq!(scan.tokens[0].end, token.len());
        }
    }
}
