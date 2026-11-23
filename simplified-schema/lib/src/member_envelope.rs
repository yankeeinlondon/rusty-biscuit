//! Structural admission of one host registration member: which of the four
//! member kinds a file is, and whether its top-level envelope is the closed
//! key set that kind's definition (`simplified-schema/schemas/*.yaml`)
//! allows, each key holding a value of the right YAML type.
//!
//! This is the one place the envelope rules of a registration member are
//! written for readers that do not prepare the member: Darkmatter's
//! registration loader classifies every member with [`admit_member`] before
//! its full preparation, and Claudine's build script refuses an ill-formed
//! inventory with it before any code is generated. The full preparation
//! (envelope admission against the compiled definitions, payload grammar,
//! imports) stays with the loader and the generator; nothing here resolves
//! or parses a schema.
//!
//! Because Claudine's build script compiles this file through `#[path]`, it
//! depends on `std` and `serde_yaml_ng` only.
//!
//! | Kind | Keys | Required |
//! |---|---|---|
//! | `schema-trigger` | `kind`, `name`, `description`, `match`, `$schema`, `types` | `match` (a missing or `null` `$schema` is a matcher-only trigger) |
//! | `yaml-schema` | `kind`, `description`, `categories`, `$schema`, `types` | `$schema` |
//! | `namespace-schema` | `kind`, `namespace`, `description`, `$schema`, `types` | a string `namespace`, a `$schema` mapping |
//! | `schema` (types-only) | `kind`, `name`, `description`, `categories`, `$schema`, `types` | a non-empty `types` mapping; `$schema` absent or `null` |
//!
//! An optional scalar may be omitted; written as `null`, a `name` and a
//! trigger's or namespace's `description` are refused, while a
//! `yaml-schema`'s or library's `description` reads as absent, as every
//! load path of those documents reads it.

use serde_yaml_ng::Value;

/// What a member file is, by its own `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemberKind {
    /// `kind: schema-trigger`: selects and types Markdown documents.
    Trigger,
    /// `kind: yaml-schema`: defines a YAML document kind.
    YamlSchema,
    /// `kind: namespace-schema`: declares one expression global.
    Namespace,
    /// `kind: schema` with `types` and no `$schema`: reusable types.
    Library,
}

impl MemberKind {
    /// The `kind` value a member of this kind declares.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trigger => "schema-trigger",
            Self::YamlSchema => "yaml-schema",
            Self::Namespace => "namespace-schema",
            Self::Library => "schema",
        }
    }

    /// Every top-level key a member of this kind may write; the closed
    /// envelope of its definition.
    #[must_use]
    pub const fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Trigger => &["kind", "name", "description", "match", "$schema", "types"],
            Self::YamlSchema => &["kind", "description", "categories", "$schema", "types"],
            Self::Namespace => &["kind", "namespace", "description", "$schema", "types"],
            Self::Library => &["kind", "name", "description", "categories", "$schema", "types"],
        }
    }
}

/// A member whose envelope [`admit_member`] admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedMember {
    #[allow(missing_docs)]
    pub kind: MemberKind,
    /// The display `name` of a trigger or library, when it declares one.
    pub name: Option<String>,
}

/// Why a member's envelope is not admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRefusal {
    /// The member's kind, when its `kind` named one.
    pub kind: Option<MemberKind>,
    /// The top-level key the refusal is about; `None` for the document as a
    /// whole.
    pub field: Option<String>,
    /// Whether `field` is absent from the document: the refusal is about the
    /// mapping that lacks it, which is where a reader reports it, rather than
    /// about a value written under it.
    pub absent: bool,
    #[allow(missing_docs)]
    pub message: String,
}

impl std::fmt::Display for MemberRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// The member kinds, as a refusal lists them.
pub const MEMBER_KINDS: &str = "a `schema-trigger`, `yaml-schema`, `namespace-schema`, or types-only `schema`";

/// [`admit_member`] over `text`, read as one YAML document.
///
/// ## Errors
///
/// Text that is not one YAML document, and every refusal of
/// [`admit_member`].
pub fn admit_member_text(text: &str) -> Result<AdmittedMember, MemberRefusal> {
    let value: Value = serde_yaml_ng::from_str(text)
        .map_err(|cause| refusal(None, format!("the file is not one YAML document: {cause}")))?;
    admit_member(&value)
}

/// Admits `value`, one member's document, by its `kind` and closed
/// envelope (see the module table).
///
/// ## Errors
///
/// The first problem found: a document that is not a mapping, a missing or
/// unknown `kind`, a key the kind does not allow, a required key that is
/// missing or `null`, an optional scalar written as `null`, and a value of
/// the wrong YAML type. A missing `kind` or required key is refused
/// [`absent`](MemberRefusal::absent); a written `null` is not.
pub fn admit_member(value: &Value) -> Result<AdmittedMember, MemberRefusal> {
    let Value::Mapping(mapping) = value else {
        return Err(refusal(None, "the file is not a YAML mapping".to_string()));
    };
    let kind = match mapping.get("kind") {
        Some(Value::String(kind)) => match kind.as_str() {
            "schema-trigger" => MemberKind::Trigger,
            "yaml-schema" => MemberKind::YamlSchema,
            "namespace-schema" => MemberKind::Namespace,
            "schema" => MemberKind::Library,
            other => return Err(at("kind", format!("its `kind` is `{other}`; a member is {MEMBER_KINDS}"))),
        },
        Some(other) => return Err(at("kind", format!("its `kind` must be a string, got {}", yaml_kind(other)))),
        None => return Err(missing("kind", format!("it declares no `kind`; a member is {MEMBER_KINDS}"))),
    };
    admit_kind(mapping, kind).map_err(|refusal| MemberRefusal { kind: Some(kind), ..refusal })
}

fn admit_kind(mapping: &serde_yaml_ng::Mapping, kind: MemberKind) -> Result<AdmittedMember, MemberRefusal> {
    let label = kind_label(kind);
    for key in mapping.keys() {
        match key.as_str() {
            Some(key) if kind.keys().contains(&key) => {}
            Some(key) => {
                return Err(at(key, format!("`{key}` is not a field of {label}; it declares only {}", listed(kind.keys()))));
            }
            None => return Err(refusal(None, format!("{label} has a key that is not a string"))),
        }
    }
    let field = |key: &str| mapping.get(key);
    let name = match kind {
        MemberKind::Trigger | MemberKind::Library => name(field("name"))?,
        MemberKind::YamlSchema | MemberKind::Namespace => None,
    };
    // A `kind: schema` or `yaml-schema` document reads a null `description`
    // as absent on every load path; a trigger's and a namespace's refuse it.
    let kinded = matches!(kind, MemberKind::YamlSchema | MemberKind::Library);
    description(field("description"), Description { null_is_absent: kinded, not_empty: kinded })?;
    match kind {
        MemberKind::Trigger => {
            match field("match") {
                None => return Err(missing("match", format!("{label} declares a non-null `match`"))),
                Some(Value::Null) => return Err(at("match", format!("{label} declares a non-null `match`"))),
                Some(Value::Mapping(_) | Value::Sequence(_)) => {}
                Some(other) => {
                    return Err(at("match", format!("`match` must be a condition mapping or a list of them, got {}", yaml_kind(other))));
                }
            }
            match field("$schema") {
                // A missing or null `$schema` is a matcher-only trigger.
                None | Some(Value::Null) => {}
                Some(payload) => schema_carrier(payload)?,
            }
            types(field("types"), label, Types::Optional)?;
        }
        MemberKind::YamlSchema => {
            match field("$schema") {
                None => return Err(missing("$schema", format!("{label} declares a `$schema` mapping"))),
                Some(Value::Null) => return Err(at("$schema", format!("{label} declares a `$schema` mapping"))),
                Some(Value::Mapping(_)) => {}
                Some(other) => return Err(at("$schema", format!("`$schema` must be a mapping, got {}", yaml_kind(other)))),
            }
            categories(field("categories"))?;
            types(field("types"), label, Types::NullableNotEmpty)?;
        }
        MemberKind::Namespace => {
            match field("namespace") {
                Some(Value::String(_)) => {}
                None => return Err(missing("namespace", format!("{label} declares a string `namespace`"))),
                Some(other) => return Err(at("namespace", format!("`namespace` must be a string, got {}", yaml_kind(other)))),
            }
            match field("$schema") {
                Some(Value::Mapping(_)) => {}
                None => return Err(missing("$schema", format!("{label} declares a `$schema` mapping"))),
                Some(other) => return Err(at("$schema", format!("`$schema` must be a mapping, got {}", yaml_kind(other)))),
            }
            types(field("types"), label, Types::Optional)?;
        }
        MemberKind::Library => {
            if field("$schema").is_some_and(|payload| !payload.is_null()) {
                return Err(at("$schema", format!("{label} is a types-only library; it may not declare `$schema`")));
            }
            categories(field("categories"))?;
            types(field("types"), label, Types::RequiredNotEmpty)?;
        }
    }
    Ok(AdmittedMember { kind, name })
}

/// The display `name` `text` declares, read leniently so a reader that
/// refuses the file can still name it: a kebab-case string `name` on a
/// `kind: schema-trigger` document, or on a `kind: schema` document without
/// a `$schema` export (a types-only library). `None` for every other
/// document, including text that is not YAML; an invalid `name` is the
/// document's failure, never a label.
#[must_use]
pub fn declared_name(text: &str) -> Option<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let value: Value = serde_yaml_ng::from_str(text).ok()?;
    let map = value.as_mapping()?;
    let labeled = match map.get("kind").and_then(Value::as_str)? {
        "schema-trigger" => true,
        "schema" => map.get("$schema").is_none_or(Value::is_null),
        _ => false,
    };
    let name = map.get("name")?.as_str().filter(|name| is_kebab_case(name))?;
    labeled.then(|| name.to_string())
}

/// How a kind's `types` is written.
enum Types {
    /// May be omitted, never `null`; may be empty.
    Optional,
    /// May be omitted or `null`; never empty when written.
    NullableNotEmpty,
    /// Required, never empty.
    RequiredNotEmpty,
}

fn types(value: Option<&Value>, label: &str, rule: Types) -> Result<(), MemberRefusal> {
    match (value, rule) {
        (None, Types::Optional | Types::NullableNotEmpty) | (Some(Value::Null), Types::NullableNotEmpty) => Ok(()),
        (None, Types::RequiredNotEmpty) => Err(missing("types", format!("{label} declares a non-empty `types` mapping"))),
        (Some(Value::Null), Types::RequiredNotEmpty) => {
            Err(at("types", format!("{label} declares a non-empty `types` mapping")))
        }
        (Some(Value::Null), Types::Optional) => Err(not_null("types")),
        (Some(Value::Mapping(table)), Types::RequiredNotEmpty) if table.is_empty() => {
            Err(at("types", format!("{label} declares a non-empty `types` mapping")))
        }
        (Some(Value::Mapping(table)), Types::NullableNotEmpty) if table.is_empty() => {
            Err(at("types", "`types` must not be empty; omit it instead".to_string()))
        }
        (Some(Value::Mapping(_)), _) => Ok(()),
        (Some(other), _) => Err(at("types", format!("`types` must be a mapping of named types, got {}", yaml_kind(other)))),
    }
}

fn name(value: Option<&Value>) -> Result<Option<String>, MemberRefusal> {
    match value {
        None => Ok(None),
        Some(Value::Null) => Err(not_null("name")),
        Some(Value::String(name)) if is_kebab_case(name) => Ok(Some(name.clone())),
        Some(Value::String(name)) => Err(at("name", format!("its `name` `{name}` is not a kebab-case identifier"))),
        Some(other) => Err(at("name", format!("its `name` must be a string, got {}", yaml_kind(other)))),
    }
}

/// How a kind's `description` is written.
struct Description {
    null_is_absent: bool,
    not_empty: bool,
}

fn description(value: Option<&Value>, rule: Description) -> Result<(), MemberRefusal> {
    match value {
        None => Ok(()),
        Some(Value::Null) if rule.null_is_absent => Ok(()),
        Some(Value::Null) => Err(not_null("description")),
        Some(Value::String(text)) if rule.not_empty && text.is_empty() => {
            Err(at("description", "its `description` must not be empty; omit it instead".to_string()))
        }
        Some(Value::String(_)) => Ok(()),
        Some(other) => Err(at("description", format!("its `description` must be a string, got {}", yaml_kind(other)))),
    }
}

fn categories(value: Option<&Value>) -> Result<(), MemberRefusal> {
    match value {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Sequence(items)) if items.is_empty() => {
            Err(at("categories", "`categories` must not be empty; omit it instead".to_string()))
        }
        Some(Value::Sequence(items)) => match items.iter().position(|item| !matches!(item, Value::String(_))) {
            None => Ok(()),
            Some(index) => Err(at(
                "categories",
                format!("`categories[{index}]` must be a string, got {}", yaml_kind(&items[index])),
            )),
        },
        Some(other) => Err(at("categories", format!("`categories` must be a list of strings, got {}", yaml_kind(other)))),
    }
}

/// A trigger's `$schema` carrier: a reference, a mapping, or a root union,
/// never empty.
fn schema_carrier(payload: &Value) -> Result<(), MemberRefusal> {
    let empty = match payload {
        Value::String(text) => text.trim().is_empty(),
        Value::Mapping(map) => map.is_empty(),
        Value::Sequence(items) => items.is_empty(),
        other => {
            return Err(at(
                "$schema",
                format!("`$schema` must be a schema reference or a schema mapping, got {}", yaml_kind(other)),
            ));
        }
    };
    if empty {
        return Err(at("$schema", "`$schema` must not be empty; omit it for a matcher-only trigger".to_string()));
    }
    Ok(())
}

fn kind_label(kind: MemberKind) -> &'static str {
    match kind {
        MemberKind::Trigger => "a `schema-trigger`",
        MemberKind::YamlSchema => "a `yaml-schema`",
        MemberKind::Namespace => "a `namespace-schema`",
        MemberKind::Library => "a `kind: schema` member",
    }
}

fn listed(keys: &[&str]) -> String {
    let quoted: Vec<String> = keys.iter().map(|key| format!("`{key}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{}, and {last}", rest.join(", ")),
        _ => quoted.join(""),
    }
}

/// Lowercase letters, digits, and inner single dashes, starting with a
/// letter (`inline-compose-schema`).
fn is_kebab_case(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && !name.ends_with('-')
        && !name.contains("--")
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn yaml_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "sequence",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged value",
    }
}

fn not_null(key: &str) -> MemberRefusal {
    at(key, format!("its `{key}` must not be null; omit it instead"))
}

fn at(key: &str, message: String) -> MemberRefusal {
    refusal(Some(key.to_string()), message)
}

/// A refusal of the required `key`, which the document omits.
fn missing(key: &str, message: String) -> MemberRefusal {
    MemberRefusal { absent: true, ..at(key, message) }
}

fn refusal(field: Option<String>, message: String) -> MemberRefusal {
    MemberRefusal { kind: None, field, absent: false, message }
}
