//! The on-disk literal token `{{!data:v1:<payload>}}`, owned by
//! `simplified_schema::pending` and re-exported here at Darkmatter's path (one
//! implementation, same types). Deciding that a value is pending, and
//! composition's decoding of tokens, stay in Darkmatter.

pub use simplified_schema::pending::{
    LiteralTokenError, TOKEN_PREFIX, TOKEN_VERSION, TokenError, decode, decode_leaf, decode_literal_tokens, encode,
    encode_yaml_scalar, holds_pending_syntax,
};
pub(crate) use simplified_schema::pending::first_token;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::Value;

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
    fn the_yaml_scalar_loads_back_from_frontmatter_as_the_bare_token() {
        let scalar = encode_yaml_scalar("title: {{ x }}\n\"quoted\" $(rm -rf x) \\");
        assert_eq!(
            yaml_value(&scalar),
            Value::String(encode("title: {{ x }}\n\"quoted\" $(rm -rf x) \\"))
        );
    }

    proptest! {
        #[test]
        fn the_yaml_scalar_round_trips_through_frontmatter(value in "(\\{\\{|\\}\\}|\\$\\(|\\\\|\n|\r\n|\"|'|\\{\\{!data:v1:|.){0,12}") {
            let loaded = yaml_value(&encode_yaml_scalar(&value));
            prop_assert_eq!(decode_literal_tokens(&loaded).unwrap(), Value::String(value));
        }
    }
}
