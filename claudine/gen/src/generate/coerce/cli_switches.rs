//! agent-cli `cli_switches` → the catalog-shaped `CliSwitchCatalog`.
//!
//! Reads the topic frontmatter as the researcher wrote it, not Darkmatter's
//! coerced copy: coercion turns a `7` into `"7"` and lets `null` stand in for
//! an absent optional property. Every field here decides how Claudine reads a
//! command line, so each authored shape is judged on its own, and anything
//! the contract does not allow fails generation instead of being dropped.
//!
//! A document written before contract revision 2 carries no typed switches;
//! it generates an explicit whole-provider gap so every compiled provider has
//! either switch records or a stated reason for having none.

use std::collections::BTreeSet;

use claudine_catalog_types::{SwitchAttachment, SwitchValue};
use serde_json::{Map, Value, json};
use strum::VariantNames;

use crate::errors::GenError;

/// The agent-cli contract revision that introduced typed switch records.
pub(crate) const SWITCH_CONTRACT_REVISION: u64 = 2;

/// The gap a pre-revision-2 document generates.
pub(crate) const LEGACY_REVISION_GAP: &str = "The agent-cli research predates contract revision 2, \
     which records switch value types; re-research the provider to establish its switches.";

/// Projects the agent-cli topic frontmatter (as authored) into the
/// catalog-shaped switch catalog: `{"researched": [...]}` sorted by
/// canonical spelling and then by scope, or `{"unknown": {"gap": ...}}`.
///
/// ## Errors
///
/// [`GenError::CliSwitchInvalid`] for any record or document shape the
/// contract does not allow, including spellings that two records share at a
/// command path where both apply.
pub(crate) fn cli_switch_catalog(frontmatter: &Value) -> Result<Value, GenError> {
    let document = "the document";
    match field(frontmatter, "schema_revision") {
        Field::Absent => return Ok(unknown(LEGACY_REVISION_GAP)),
        Field::Present(value) => match value.as_u64() {
            Some(revision) if revision < SWITCH_CONTRACT_REVISION => {
                return Ok(unknown(LEGACY_REVISION_GAP));
            }
            Some(SWITCH_CONTRACT_REVISION) => {}
            _ => {
                return Err(invalid(
                    document,
                    format!(
                        "`schema_revision` is `{value}`; this generator reads switch records \
                         only at revision {SWITCH_CONTRACT_REVISION}"
                    ),
                ));
            }
        },
        Field::Null => return Err(invalid(document, "`schema_revision` is null".into())),
    }

    let records = match field(frontmatter, "cli_switches") {
        Field::Present(Value::Array(records)) => records,
        Field::Present(other) => {
            return Err(invalid(document, format!("`cli_switches` must be a list, got `{other}`")));
        }
        Field::Absent | Field::Null => {
            return Err(invalid(document, "`cli_switches` is required".into()));
        }
    };
    let gap = optional_text(field(frontmatter, "cli_switches_gap"), "cli_switches_gap", document)?;
    match (records.is_empty(), gap) {
        (true, Some(gap)) => return Ok(unknown(gap)),
        (true, None) => {
            return Err(invalid(
                document,
                "`cli_switches` is empty, so `cli_switches_gap` must say why".into(),
            ));
        }
        (false, Some(_)) => {
            return Err(invalid(
                document,
                "`cli_switches_gap` is present but `cli_switches` has records".into(),
            ));
        }
        (false, None) => {}
    }

    let mut parsed = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        parsed.push(parse_record(index, record)?);
    }
    reject_conflicts(&parsed)?;
    parsed.sort_by(|a, b| (a.flag.as_str(), &a.scope_key).cmp(&(b.flag.as_str(), &b.scope_key)));
    Ok(json!({ "researched": parsed.into_iter().map(|r| r.catalog).collect::<Vec<_>>() }))
}

fn unknown(gap: &str) -> Value {
    json!({ "unknown": { "gap": gap } })
}

/// One validated record plus what the cross-record checks need.
struct ParsedSwitch {
    flag: String,
    spellings: Vec<String>,
    scopes: Vec<Scope>,
    /// Sort key that orders records sharing a canonical spelling.
    scope_key: String,
    catalog: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Scope {
    Global,
    Command(Vec<String>),
}

impl Scope {
    fn catalog(&self) -> Value {
        match self {
            Scope::Global => json!("global"),
            Scope::Command(path) => json!({ "command": path }),
        }
    }
}

/// Two records apply together when either is global or both name the same
/// exact command path.
fn scopes_meet(left: &[Scope], right: &[Scope]) -> bool {
    left.iter().any(|l| {
        right
            .iter()
            .any(|r| *l == Scope::Global || *r == Scope::Global || l == r)
    })
}

fn reject_conflicts(records: &[ParsedSwitch]) -> Result<(), GenError> {
    for (index, left) in records.iter().enumerate() {
        for right in &records[index + 1..] {
            if !scopes_meet(&left.scopes, &right.scopes) {
                continue;
            }
            if let Some(shared) = left.spellings.iter().find(|s| right.spellings.contains(s)) {
                return Err(invalid(
                    &right.flag,
                    format!(
                        "spelling `{shared}` is also claimed by record `{}` at a command path \
                         where both apply",
                        left.flag
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn parse_record(index: usize, record: &Value) -> Result<ParsedSwitch, GenError> {
    let position = format!("#{}", index + 1);
    let Value::Object(record) = record else {
        return Err(invalid(&position, format!("expected a mapping, got `{record}`")));
    };
    let flag = match record_field(record, "flag") {
        Field::Present(Value::String(flag)) if is_spelling(flag) => flag.clone(),
        Field::Present(other) => {
            return Err(invalid(&position, format!("`flag` `{other}` is not a switch spelling")));
        }
        Field::Absent | Field::Null => return Err(invalid(&position, "`flag` is required".into())),
    };
    let label = flag.as_str();

    let aliases = match record_field(record, "aliases") {
        Field::Absent => Vec::new(),
        Field::Null => return Err(invalid(label, "`aliases` is null; omit it when there are none".into())),
        Field::Present(Value::Array(items)) => {
            let mut aliases = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::String(alias) if is_spelling(alias) => aliases.push(alias.clone()),
                    other => {
                        return Err(invalid(label, format!("alias `{other}` is not a switch spelling")));
                    }
                }
            }
            aliases
        }
        Field::Present(other) => {
            return Err(invalid(label, format!("`aliases` must be a list, got `{other}`")));
        }
    };
    let mut spellings = vec![flag.clone()];
    for alias in &aliases {
        if spellings.contains(alias) {
            return Err(invalid(label, format!("spelling `{alias}` is listed twice")));
        }
        spellings.push(alias.clone());
    }

    let value_type = member(record, "value_type", SwitchValue::VARIANTS, label)?;
    let value_optional = record_field(record, "value_optional");
    let variadic_min = record_field(record, "variadic_min");
    let scalar = matches!(value_type, "string" | "number");
    if !scalar {
        absent_unless(value_optional, "value_optional", "value_type is string or number", label)?;
    }
    if value_type != "variadic" {
        absent_unless(variadic_min, "variadic_min", "value_type is variadic", label)?;
    }
    let mut min_unknown = false;
    let value = match value_type {
        "none" | "unknown" => json!(value_type),
        "string" | "number" => match value_optional {
            Field::Present(Value::Bool(optional)) => json!({ value_type: { "optional": optional } }),
            Field::Present(other) => {
                return Err(invalid(label, format!("`value_optional` must be a boolean, got `{other}`")));
            }
            Field::Absent | Field::Null => {
                return Err(invalid(label, format!("`value_optional` is required for a {value_type} value")));
            }
        },
        _ => {
            let min = match variadic_min {
                Field::Present(Value::String(text)) if text == "unknown" => {
                    min_unknown = true;
                    json!("unknown")
                }
                Field::Present(number) => match number.as_u64() {
                    Some(min) if min >= 1 && u32::try_from(min).is_ok() => json!(min),
                    _ => {
                        return Err(invalid(
                            label,
                            format!("`variadic_min` must be an integer of at least 1 or `unknown`, got `{number}`"),
                        ));
                    }
                },
                Field::Absent | Field::Null => {
                    return Err(invalid(label, "`variadic_min` is required for a variadic value".into()));
                }
            };
            json!({ "variadic": { "min": min } })
        }
    };

    let attachments = member_list(record, "attachment", SwitchAttachment::VARIANTS, label)?;
    let takes_value = !matches!(value_type, "none" | "unknown");
    if takes_value && attachments.is_empty() {
        return Err(invalid(label, format!("a {value_type} value needs at least one `attachment` form")));
    }
    if !takes_value && !attachments.is_empty() {
        return Err(invalid(label, format!("`attachment` must be empty for a {value_type} value")));
    }
    if attachments.contains(&"short_attached") && !spellings.iter().any(|s| is_short(s)) {
        return Err(invalid(
            label,
            "`short_attached` needs a spelling of one dash and one character".into(),
        ));
    }

    let scopes = parse_scopes(record, label)?;
    let description = required_text(record, "description", label)?;
    let gap = optional_text(record_field(record, "gap"), "gap", label)?;
    let gap_needed = value_type == "unknown" || min_unknown;
    match (gap_needed, gap) {
        (true, None) => {
            return Err(invalid(label, "`gap` must describe what is unknown".into()));
        }
        (false, Some(_)) => {
            return Err(invalid(label, "`gap` is present but nothing is unknown".into()));
        }
        _ => {}
    }

    let scope_key = format!("{scopes:?}");
    let catalog = json!({
        "flag": flag,
        "aliases": aliases,
        "value": value,
        "attachments": attachments,
        "scopes": scopes.iter().map(Scope::catalog).collect::<Vec<_>>(),
        "description": description,
        "gap": gap,
    });
    Ok(ParsedSwitch {
        flag,
        spellings,
        scopes,
        scope_key,
        catalog,
    })
}

fn parse_scopes(record: &Map<String, Value>, label: &str) -> Result<Vec<Scope>, GenError> {
    let entries = match record_field(record, "invocation_scope") {
        Field::Present(Value::Array(entries)) if !entries.is_empty() => entries,
        Field::Present(Value::Array(_)) => {
            return Err(invalid(label, "`invocation_scope` needs at least one entry".into()));
        }
        Field::Present(other) => {
            return Err(invalid(label, format!("`invocation_scope` must be a list, got `{other}`")));
        }
        Field::Absent | Field::Null => {
            return Err(invalid(label, "`invocation_scope` is required".into()));
        }
    };
    let mut scopes = Vec::with_capacity(entries.len());
    for entry in entries {
        let Value::Object(entry) = entry else {
            return Err(invalid(label, format!("scope entry `{entry}` is not a mapping")));
        };
        let applies_to = member(entry, "applies_to", &["global", "command"], label)?;
        let command = record_field(entry, "command");
        let scope = if applies_to == "global" {
            absent_unless(command, "command", "applies_to is command", label)?;
            Scope::Global
        } else {
            match command {
                Field::Present(Value::Array(segments)) => {
                    let mut path = Vec::with_capacity(segments.len());
                    for segment in segments {
                        match segment {
                            Value::String(word) if is_command_word(word) => path.push(word.clone()),
                            other => {
                                return Err(invalid(label, format!("command word `{other}` is not a command name")));
                            }
                        }
                    }
                    Scope::Command(path)
                }
                Field::Present(other) => {
                    return Err(invalid(label, format!("`command` must be a list, got `{other}`")));
                }
                Field::Absent | Field::Null => {
                    return Err(invalid(label, "`command` is required when applies_to is command".into()));
                }
            }
        };
        if scopes.contains(&scope) {
            return Err(invalid(label, format!("scope `{}` is listed twice", scope.catalog())));
        }
        scopes.push(scope);
    }
    if scopes.len() > 1 && scopes.contains(&Scope::Global) {
        return Err(invalid(label, "a global scope already covers every command path".into()));
    }
    scopes.sort();
    Ok(scopes)
}

/// Absent, explicit null, and present are three different authored shapes.
#[derive(Clone, Copy)]
enum Field<'v> {
    Absent,
    Null,
    Present(&'v Value),
}

fn field<'v>(object: &'v Value, key: &str) -> Field<'v> {
    match object.get(key) {
        None => Field::Absent,
        Some(Value::Null) => Field::Null,
        Some(value) => Field::Present(value),
    }
}

fn record_field<'v>(record: &'v Map<String, Value>, key: &str) -> Field<'v> {
    match record.get(key) {
        None => Field::Absent,
        Some(Value::Null) => Field::Null,
        Some(value) => Field::Present(value),
    }
}

fn absent_unless(found: Field<'_>, key: &str, condition: &str, label: &str) -> Result<(), GenError> {
    match found {
        Field::Absent => Ok(()),
        Field::Null => Err(invalid(label, format!("`{key}` is null; it is present only when {condition}"))),
        Field::Present(_) => Err(invalid(label, format!("`{key}` is present only when {condition}"))),
    }
}

fn member<'m>(
    record: &Map<String, Value>,
    key: &str,
    members: &'m [&'m str],
    label: &str,
) -> Result<&'m str, GenError> {
    match record_field(record, key) {
        Field::Present(Value::String(text)) => members
            .iter()
            .find(|m| **m == text)
            .copied()
            .ok_or_else(|| invalid(label, format!("`{key}` `{text}` is not one of {}", members.join(", ")))),
        Field::Present(other) => Err(invalid(label, format!("`{key}` must be a string, got `{other}`"))),
        Field::Absent | Field::Null => Err(invalid(label, format!("`{key}` is required"))),
    }
}

fn member_list<'m>(
    record: &Map<String, Value>,
    key: &str,
    members: &'m [&'m str],
    label: &str,
) -> Result<Vec<&'m str>, GenError> {
    let items = match record_field(record, key) {
        Field::Present(Value::Array(items)) => items,
        Field::Present(other) => {
            return Err(invalid(label, format!("`{key}` must be a list, got `{other}`")));
        }
        Field::Absent | Field::Null => return Err(invalid(label, format!("`{key}` is required"))),
    };
    let mut seen = BTreeSet::new();
    let mut found = Vec::with_capacity(items.len());
    for item in items {
        let found_member = item
            .as_str()
            .and_then(|text| members.iter().find(|m| **m == text).copied())
            .ok_or_else(|| invalid(label, format!("`{key}` entry `{item}` is not one of {}", members.join(", "))))?;
        if !seen.insert(found_member) {
            return Err(invalid(label, format!("`{key}` lists `{found_member}` twice")));
        }
        found.push(found_member);
    }
    Ok(found)
}

fn required_text<'v>(record: &'v Map<String, Value>, key: &str, label: &str) -> Result<&'v str, GenError> {
    match record_field(record, key) {
        Field::Present(Value::String(text)) if !text.trim().is_empty() => Ok(text),
        Field::Present(other) => Err(invalid(label, format!("`{key}` must be non-empty text, got `{other}`"))),
        Field::Absent | Field::Null => Err(invalid(label, format!("`{key}` is required"))),
    }
}

fn optional_text<'v>(found: Field<'v>, key: &str, label: &str) -> Result<Option<&'v str>, GenError> {
    match found {
        Field::Absent => Ok(None),
        Field::Null => Err(invalid(label, format!("`{key}` is null; omit it instead"))),
        Field::Present(Value::String(text)) if !text.trim().is_empty() => Ok(Some(text)),
        Field::Present(other) => Err(invalid(label, format!("`{key}` must be non-empty text, got `{other}`"))),
    }
}

/// `-x` or `--name`: one or two dashes, then a letter or digit, then letters,
/// digits, `_`, `.`, or `-` (the contract's `flag` pattern).
fn is_spelling(text: &str) -> bool {
    let body = text
        .strip_prefix("--")
        .or_else(|| text.strip_prefix('-'))
        .unwrap_or("");
    let mut chars = body.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn is_short(spelling: &str) -> bool {
    spelling.len() == 2 && spelling.starts_with('-') && !spelling.starts_with("--")
}

/// The contract's command-word pattern.
fn is_command_word(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-'))
}

fn invalid(record: &str, message: String) -> GenError {
    GenError::CliSwitchInvalid {
        record: record.to_string(),
        message,
    }
}
