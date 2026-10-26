//! Readers for the Schematic AsyncAPI extensions.
//!
//! An extension is optional metadata: a malformed one is a WARN at its
//! pointer, and the field reads as though the extension were absent. An
//! unknown member of an object-valued extension is INFO. The names are the
//! exporter's ([`crate::export::asyncapi`]).

use schematic_define::websocket::{EndpointRole, HeartbeatHints, MessageSelector};
use schematic_define::{ParamStyle, QueryParamType};
use serde_json::Value;

use super::reader::{Reader, child, kind};

/// Reads `x-schematic-selectors`. `None` means absent or rejected, and the
/// caller infers selectors from the payload; `Some(vec![])` is an explicit
/// empty list, which suppresses inference.
pub(super) fn selectors(reader: &mut Reader, value: Option<&Value>, at: &str) -> Option<Vec<MessageSelector>> {
    let value = value?;
    let Value::Array(items) = value else {
        reader.warn(at, format!("Expected an array of {{field, value}} selectors, found {}; the selectors are ignored", kind(value)));
        return None;
    };
    let mut selectors = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let item_at = child(at, &index.to_string());
        let Value::Object(object) = item else {
            reader.warn(&item_at, format!("Expected a {{field, value}} object, found {}; the selectors are ignored", kind(item)));
            return None;
        };
        for key in object.keys().filter(|key| *key != "field" && *key != "value") {
            reader.info(&child(&item_at, key), "Unknown selector member is ignored");
        }
        let field = match object.get("field") {
            Some(Value::String(field)) if !field.is_empty() => field,
            Some(Value::String(_)) => {
                reader.warn(&child(&item_at, "field"), "A selector field must not be empty; the selectors are ignored");
                return None;
            }
            other => {
                reader.warn(&child(&item_at, "field"), format!("Expected a field name string, found {}; the selectors are ignored", other.map_or("nothing", kind)));
                return None;
            }
        };
        // A selector value is a string: an empty string is a value.
        let Some(Value::String(selector_value)) = object.get("value") else {
            reader.warn(
                &child(&item_at, "value"),
                format!("Expected a string value, found {}; the selectors are ignored", object.get("value").map_or("nothing", kind)),
            );
            return None;
        };
        selectors.push(MessageSelector::new(field, selector_value));
    }
    Some(selectors)
}

/// Reads `x-schematic-fallback`; anything but a boolean is ignored.
pub(super) fn fallback(reader: &mut Reader, value: Option<&Value>, at: &str) -> bool {
    match value {
        None => false,
        Some(Value::Bool(flag)) => *flag,
        Some(other) => {
            reader.warn(at, format!("Expected a boolean, found {}; the message is not a fallback", kind(other)));
            false
        }
    }
}

/// Reads `x-schematic-role`, or `default` when it is absent or malformed.
pub(super) fn role(reader: &mut Reader, value: Option<&Value>, at: &str, default: EndpointRole) -> EndpointRole {
    let Some(value) = value else { return default };
    match value.as_str().map(str::parse::<EndpointRole>) {
        Some(Ok(role)) => role,
        _ => {
            reader.warn(at, format!("Expected `client`, `host`, or `both`, found {value}; the role is `{default}`"));
            default
        }
    }
}

/// Reads `x-schematic-heartbeat`. A malformed hint is dropped whole.
pub(super) fn heartbeat(reader: &mut Reader, value: Option<&Value>, at: &str) -> Option<HeartbeatHints> {
    let value = value?;
    let Value::Object(object) = value else {
        reader.warn(at, format!("Expected an object, found {}; the heartbeat hint is dropped", kind(value)));
        return None;
    };
    for key in object.keys().filter(|key| *key != "interval_secs" && *key != "timeout_secs") {
        reader.info(&child(at, key), "Unknown heartbeat member is ignored");
    }
    let seconds = |reader: &mut Reader, key: &str| -> Result<Option<u64>, ()> {
        match object.get(key) {
            None => Ok(None),
            Some(value) => match value.as_u64() {
                Some(0) => {
                    reader.warn(&child(at, key), "Must be at least 1 second; the heartbeat hint is dropped");
                    Err(())
                }
                Some(seconds) => Ok(Some(seconds)),
                None => {
                    reader.warn(&child(at, key), format!("Expected whole seconds, found {value}; the heartbeat hint is dropped"));
                    Err(())
                }
            },
        }
    };
    let interval = seconds(reader, "interval_secs").ok()?;
    let timeout_secs = seconds(reader, "timeout_secs").ok()?;
    let Some(interval_secs) = interval else {
        reader.warn(&child(at, "interval_secs"), "Missing required interval; the heartbeat hint is dropped");
        return None;
    };
    Some(HeartbeatHints { interval_secs, timeout_secs })
}

/// The parameter details `x-schematic-param` carries.
#[derive(Debug, Default)]
pub(super) struct ParamExtension {
    /// A path parameter's scalar type.
    pub(super) param_type: Option<QueryParamType>,
    pub(super) style: Option<ParamStyle>,
    pub(super) explode: bool,
    pub(super) rust_name: Option<String>,
    pub(super) secret: bool,
    pub(super) pager_managed: bool,
    pub(super) deprecated: bool,
}

/// Reads `x-schematic-param`. A malformed member is dropped alone.
pub(super) fn param(reader: &mut Reader, value: Option<&Value>, at: &str) -> Option<ParamExtension> {
    let value = value?;
    let Value::Object(object) = value else {
        reader.warn(at, format!("Expected an object, found {}; the parameter details are ignored", kind(value)));
        return None;
    };
    let mut extension = ParamExtension::default();
    for (key, member) in object {
        let member_at = child(at, key);
        let flag = |reader: &mut Reader| match member {
            Value::Bool(flag) => *flag,
            other => {
                reader.warn(&member_at, format!("Expected a boolean, found {}; it is ignored", kind(other)));
                false
            }
        };
        match key.as_str() {
            "type" => {
                extension.param_type = match member.as_str() {
                    Some("string") => Some(QueryParamType::String),
                    Some("integer") => Some(QueryParamType::Integer),
                    Some("number") => Some(QueryParamType::Number),
                    Some("boolean") => Some(QueryParamType::Boolean),
                    _ => {
                        reader.warn(&member_at, format!("Expected `string`, `integer`, `number`, or `boolean`, found {member}; it is ignored"));
                        None
                    }
                }
            }
            "style" => match serde_json::from_value::<ParamStyle>(member.clone()) {
                Ok(style) => extension.style = Some(style),
                Err(_) => reader.warn(&member_at, format!("Expected a parameter style such as `Form`, found {member}; it is ignored")),
            },
            "rust_name" => match member {
                Value::String(name) if !name.is_empty() => extension.rust_name = Some(name.clone()),
                other => reader.warn(&member_at, format!("Expected a nonempty name, found {other}; it is ignored")),
            },
            "explode" => extension.explode = flag(reader),
            "secret" => extension.secret = flag(reader),
            "pager_managed" => extension.pager_managed = flag(reader),
            "deprecated" => extension.deprecated = flag(reader),
            _ => reader.info(&member_at, "Unknown parameter detail is ignored"),
        }
    }
    Some(extension)
}

/// Reads an optional nonempty string extension of a security scheme.
pub(super) fn scheme_string(reader: &mut Reader, value: Option<&Value>, at: &str) -> Option<String> {
    match value? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        other => {
            reader.warn(at, format!("Expected a nonempty string, found {other}; it is ignored"));
            None
        }
    }
}
