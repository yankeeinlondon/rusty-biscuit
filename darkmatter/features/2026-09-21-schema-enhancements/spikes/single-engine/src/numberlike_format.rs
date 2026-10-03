//! THROWAWAY SPIKE (question 4): `numberlike` / `boolish` as custom
//! Darkmatter formats that call the rule core, versus regex patterns.

use jsonschema::{Draft, Validator};
use serde_json::{Value, json};

use super::core::{parse_boolean_text, parse_number_text};
use crate::markdown::schemas::format::register_darkmatter_formats_in_context;

pub const NUMBERLIKE_FORMAT: &str = "darkmatter-numberlike";
pub const BOOLISH_FORMAT: &str = "darkmatter-boolish";

/// Best regex approximation of the lenient number grammar. It cannot encode
/// the "whole-number text must fit i64/u64" rule.
pub const LENIENT_NUMBER_PATTERN: &str =
    r"^\s*[+-]?(?:\d(?:_?\d)*(?:\.\d(?:_?\d)*)?|\.\d(?:_?\d)*)(?:[eE][+-]?\d+)?\s*$";
pub const BOOLISH_PATTERN: &str = r"^\s*(?i:true|false|yes|no|on|off|1|0)\s*$";

pub fn numberlike_schema_format() -> Value {
    json!({ "anyOf": [ { "type": "number" }, { "type": "string", "format": NUMBERLIKE_FORMAT } ] })
}
pub fn boolish_schema_format() -> Value {
    json!({ "anyOf": [ { "type": "boolean" }, { "type": "string", "format": BOOLISH_FORMAT } ] })
}
pub fn numberlike_schema_pattern() -> Value {
    json!({ "anyOf": [ { "type": "number" }, { "type": "string", "pattern": LENIENT_NUMBER_PATTERN } ] })
}
pub fn boolish_schema_pattern() -> Value {
    json!({ "anyOf": [ { "type": "boolean" }, { "type": "string", "pattern": BOOLISH_PATTERN } ] })
}

/// Builds a validator with the Darkmatter formats plus the two new ones.
pub fn build(schema: &Value) -> Validator {
    let opts = jsonschema::options().with_draft(Draft::Draft202012);
    register_darkmatter_formats_in_context(opts, None, None, None)
        .with_format(NUMBERLIKE_FORMAT, |s: &str| parse_number_text(s).is_ok())
        .with_format(BOOLISH_FORMAT, |s: &str| parse_boolean_text(s).is_some())
        .should_validate_formats(true)
        .build(schema)
        .expect("numberlike/boolish spike schema builds")
}
