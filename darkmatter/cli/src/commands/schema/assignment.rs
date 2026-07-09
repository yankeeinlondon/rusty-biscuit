//! Parsing and application of `<prop>=<value>` assignments for
//! `md schema validate`.
//!
//! An assignment is a positional CLI argument of the form `path=value` where
//! `path` is a dot-separated property path (`title`, `user.email`) and
//! `value` is parsed as a YAML scalar or flow value. Assignments are applied
//! to each document's frontmatter map before validation, always overriding
//! existing keys.
//!
//! When the document has a SimplifiedSchema in scope, assignment values are
//! coerced to match the declared property type before being applied. This
//! avoids surprises around shell quoting: bare scalars like `bar=true` parse
//! as a YAML boolean by default, but when the schema declares `bar` as a
//! string (or another string-shaped type) the raw right-hand side is stored
//! verbatim as a string instead.

use darkmatter::markdown::FrontmatterMap;
use darkmatter::markdown::schemas::{
    EffectiveSchema, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType,
    TypeExpr,
};
use serde_json::{Map, Value};

/// A parsed `<prop>=<value>` assignment.
#[derive(Debug, Clone)]
pub struct Assignment {
    /// Property path (dot-separated identifiers).
    pub path: Vec<String>,
    /// Tentative YAML-parsed value, used when no schema disambiguates the
    /// expected type at [`Self::path`].
    pub value: Value,
    /// Original right-hand-side string, preserved for schema-aware string
    /// coercion (e.g. `bar=true` against `bar: string` → `"true"`).
    pub raw_value: String,
    /// The original `path=value` token, for diagnostics.
    pub raw: String,
}

/// Outcome of classifying a single positional argument.
pub enum PositionalKind {
    /// Token is a file path.
    File(String),
    /// Token parsed as an assignment.
    Assignment(Assignment),
    /// Token looked like an assignment (matched the property-path shape on
    /// the LHS) but the value side failed to parse. Reported as a usage
    /// error rather than silently treated as a file path.
    Invalid { raw: String, message: String },
}

/// Classifies a positional argument as a file path or assignment.
///
/// The disambiguation rule: the token is an assignment when it contains `=`
/// **and** the substring before the first `=` is a valid dot-separated
/// property path (each segment matches `[A-Za-z_][A-Za-z0-9_-]*`). Any
/// other token is treated as a file path.
pub fn classify(input: &str) -> PositionalKind {
    let Some(eq_idx) = input.find('=') else {
        return PositionalKind::File(input.to_string());
    };
    let (lhs, rhs_with_eq) = input.split_at(eq_idx);
    let rhs = &rhs_with_eq[1..];

    if !is_property_path(lhs) {
        return PositionalKind::File(input.to_string());
    }

    match parse_value(rhs) {
        Ok(value) => PositionalKind::Assignment(Assignment {
            path: lhs.split('.').map(str::to_string).collect(),
            value,
            raw_value: rhs.to_string(),
            raw: input.to_string(),
        }),
        Err(message) => PositionalKind::Invalid {
            raw: input.to_string(),
            message,
        },
    }
}

fn is_property_path(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.split('.').all(is_identifier)
}

fn is_identifier(seg: &str) -> bool {
    let mut chars = seg.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Parses the right-hand side of an assignment as a YAML scalar or flow
/// value, returning a JSON value suitable for inserting into a frontmatter
/// map. An empty RHS becomes an empty string rather than YAML null, which
/// would otherwise surprise users writing `key=`.
fn parse_value(rhs: &str) -> Result<Value, String> {
    if rhs.is_empty() {
        return Ok(Value::String(String::new()));
    }
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(rhs).map_err(|err| format!("invalid YAML value: {err}"))?;
    serde_json::to_value(yaml).map_err(|err| format!("value could not be converted to JSON: {err}"))
}

/// Applies all assignments to a frontmatter map, overriding existing values.
/// Intermediate path segments that are missing or non-object are replaced
/// with a new object before the descent continues.
///
/// When `schema` is `Some`, assignment values are coerced to match the
/// declared SimplifiedSchema type at each top-level property path before
/// insertion. See [`coerce_value`].
pub fn apply_all(
    map: &mut FrontmatterMap,
    assignments: &[Assignment],
    schema: Option<&EffectiveSchema>,
) {
    for a in assignments {
        let value = coerce_value(a, schema);
        apply_one(map, &a.path, value);
    }
}

/// Picks the final JSON value to insert for `assignment`, using `schema` to
/// disambiguate the expected type at top-level scalar paths.
///
/// Coercion is intentionally conservative: it only fires when the schema
/// declares the property as a string-shaped scalar type and the YAML-parsed
/// value is not already a string. Everything else falls back to the YAML
/// scalar/flow parse done at classification time, preserving the current
/// `count=5`, `flag=true`, `tags=[a, b, c]` behaviour.
fn coerce_value(assignment: &Assignment, schema: Option<&EffectiveSchema>) -> Value {
    if force_string_for_path(&assignment.path, schema) {
        if assignment.value.is_string() {
            assignment.value.clone()
        } else {
            Value::String(assignment.raw_value.clone())
        }
    } else {
        assignment.value.clone()
    }
}

/// Returns `true` when `schema` declares `path` as a string-shaped scalar
/// (string, date, datetime, time, url, email, enum, or file).
///
/// Coercion only applies to **top-level** scalar property paths. Nested
/// `user.email=...` paths land inside `object` typed properties which are
/// opaque in v1, so the original YAML parse is used. Root unions and raw
/// JSON Schema inputs lack a SimplifiedSchema projection and likewise fall
/// back to YAML.
fn force_string_for_path(path: &[String], schema: Option<&EffectiveSchema>) -> bool {
    if path.len() != 1 {
        return false;
    }
    let Some(effective) = schema else {
        return false;
    };
    let Some(simplified) = effective.simplified.as_ref() else {
        return false;
    };
    let SimplifiedSchema::Single(shape) = simplified else {
        return false;
    };
    let Some(def) = lookup_property(shape, &path[0]) else {
        return false;
    };
    match def {
        PropertyDef::Single(atom) => atom_prefers_string(atom),
        PropertyDef::Union(arms) => arms.iter().any(atom_prefers_string),
    }
}

fn lookup_property<'a>(shape: &'a SchemaShape, name: &str) -> Option<&'a PropertyDef> {
    shape.properties.get(name)
}

fn atom_prefers_string(atom: &PropertyAtom) -> bool {
    if atom.is_array {
        return false;
    }
    let primitive = match &atom.ty {
        TypeExpr::Primitive(p) => *p,
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } => return false,
    };
    matches!(
        primitive,
        SimplifiedType::String
            | SimplifiedType::Date
            | SimplifiedType::DateTime
            | SimplifiedType::Time
            | SimplifiedType::Url
            | SimplifiedType::Email
            | SimplifiedType::Enum
            | SimplifiedType::File
    )
}

fn apply_one(map: &mut FrontmatterMap, path: &[String], value: Value) {
    debug_assert!(!path.is_empty(), "assignment path must be non-empty");

    if path.len() == 1 {
        map.insert(path[0].clone(), value);
        return;
    }

    let head = &path[0];
    let entry = map
        .entry(head.clone())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    if let Value::Object(inner) = entry {
        insert_nested(inner, &path[1..], value);
    }
}

fn insert_nested(map: &mut Map<String, Value>, path: &[String], value: Value) {
    if path.len() == 1 {
        map.insert(path[0].clone(), value);
        return;
    }
    let head = &path[0];
    let entry = map
        .entry(head.clone())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    if let Value::Object(inner) = entry {
        insert_nested(inner, &path[1..], value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_assignment(input: &str, expected_path: &[&str], expected_value: Value) {
        match classify(input) {
            PositionalKind::Assignment(a) => {
                let segments: Vec<&str> = a.path.iter().map(String::as_str).collect();
                assert_eq!(segments, expected_path, "path for {input}");
                assert_eq!(a.value, expected_value, "value for {input}");
            }
            PositionalKind::File(_) => panic!("expected assignment, got file: {input}"),
            PositionalKind::Invalid { message, .. } => {
                panic!("expected assignment, got invalid for {input}: {message}")
            }
        }
    }

    fn assert_file(input: &str) {
        match classify(input) {
            PositionalKind::File(_) => {}
            other => panic!("expected file path, got {other:?} for {input}"),
        }
    }

    impl std::fmt::Debug for PositionalKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PositionalKind::File(p) => write!(f, "File({p:?})"),
                PositionalKind::Assignment(a) => write!(f, "Assignment({a:?})"),
                PositionalKind::Invalid { raw, message } => {
                    write!(f, "Invalid({raw:?}, {message:?})")
                }
            }
        }
    }

    #[test]
    fn parses_string_value() {
        assert_assignment("title=Hello", &["title"], json!("Hello"));
    }

    #[test]
    fn parses_integer_value() {
        assert_assignment("count=5", &["count"], json!(5));
    }

    #[test]
    fn parses_float_value() {
        assert_assignment("rating=2.5", &["rating"], json!(2.5));
    }

    #[test]
    fn parses_boolean_value() {
        assert_assignment("published=true", &["published"], json!(true));
    }

    #[test]
    fn parses_flow_sequence() {
        assert_assignment("tags=[a, b, c]", &["tags"], json!(["a", "b", "c"]));
    }

    #[test]
    fn parses_flow_mapping() {
        assert_assignment("user={n: Ken}", &["user"], json!({"n": "Ken"}));
    }

    #[test]
    fn empty_rhs_is_empty_string() {
        assert_assignment("title=", &["title"], json!(""));
    }

    #[test]
    fn nested_path() {
        assert_assignment(
            "user.email=ken@ken.net",
            &["user", "email"],
            json!("ken@ken.net"),
        );
    }

    #[test]
    fn deep_nested_path() {
        assert_assignment("a.b.c=1", &["a", "b", "c"], json!(1));
    }

    #[test]
    fn paths_with_slash_treated_as_file() {
        assert_file("./post.md");
        assert_file("docs/post.md");
        assert_file("./weird=name.md");
    }

    #[test]
    fn bare_filename_no_equals_treated_as_file() {
        assert_file("post.md");
    }

    #[test]
    fn empty_lhs_treated_as_file() {
        assert_file("=value");
    }

    #[test]
    fn invalid_identifier_lhs_treated_as_file() {
        // `1bad` starts with a digit, so not a valid identifier — treat as file.
        assert_file("1bad=value");
    }

    fn make_assignment(path: &[&str], value: Value, raw_value: &str) -> Assignment {
        Assignment {
            path: path.iter().map(|s| (*s).to_string()).collect(),
            value,
            raw_value: raw_value.to_string(),
            raw: format!("{}={}", path.join("."), raw_value),
        }
    }

    #[test]
    fn applies_top_level_assignment() {
        let mut map: FrontmatterMap = FrontmatterMap::new();
        map.insert("title".into(), json!("Old"));
        apply_all(
            &mut map,
            &[make_assignment(&["title"], json!("New"), "New")],
            None,
        );
        assert_eq!(map["title"], json!("New"));
    }

    #[test]
    fn applies_nested_assignment_creating_object() {
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(
            &mut map,
            &[make_assignment(
                &["user", "email"],
                json!("k@k.net"),
                "k@k.net",
            )],
            None,
        );
        assert_eq!(map["user"], json!({ "email": "k@k.net" }));
    }

    #[test]
    fn applies_nested_assignment_merging_object() {
        let mut map: FrontmatterMap = FrontmatterMap::new();
        map.insert("user".into(), json!({ "name": "Ken" }));
        apply_all(
            &mut map,
            &[make_assignment(
                &["user", "email"],
                json!("k@k.net"),
                "k@k.net",
            )],
            None,
        );
        assert_eq!(map["user"], json!({ "name": "Ken", "email": "k@k.net" }));
    }

    #[test]
    fn applies_nested_overwrites_non_object_parent() {
        let mut map: FrontmatterMap = FrontmatterMap::new();
        map.insert("user".into(), json!("string-instead-of-object"));
        apply_all(
            &mut map,
            &[make_assignment(
                &["user", "email"],
                json!("k@k.net"),
                "k@k.net",
            )],
            None,
        );
        assert_eq!(map["user"], json!({ "email": "k@k.net" }));
    }

    // ── schema-aware coercion ───────────────────────────────────────────────

    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::schemas::DarkmatterSchemas;

    fn effective_for(yaml_body: &str) -> darkmatter::markdown::schemas::EffectiveSchema {
        let md_text = format!("---\n{yaml_body}\n---\n");
        let md: Markdown = md_text.as_str().into();
        DarkmatterSchemas::new()
            .effective_for(&md)
            .expect("effective_for")
            .expect("schema present")
    }

    #[test]
    fn coerces_yaml_boolean_to_string_when_schema_says_string() {
        let eff = effective_for("$schema:\n  bar: 'string(required)'");
        let a = make_assignment(&["bar"], json!(true), "true");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["bar"], json!("true"));
    }

    #[test]
    fn coerces_yaml_number_to_string_when_schema_says_string() {
        let eff = effective_for("$schema:\n  bar: 'string(required)'");
        let a = make_assignment(&["bar"], json!(42), "42");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["bar"], json!("42"));
    }

    #[test]
    fn keeps_yaml_string_as_is_when_schema_says_string() {
        // Pre-quoted RHS that already parsed as a string must not be
        // re-wrapped with literal quotes.
        let eff = effective_for("$schema:\n  bar: 'string(required)'");
        let a = make_assignment(&["bar"], json!("true"), "\"true\"");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["bar"], json!("true"));
    }

    #[test]
    fn keeps_boolean_for_boolean_schema() {
        let eff = effective_for("$schema:\n  flag: 'boolean(required)'");
        let a = make_assignment(&["flag"], json!(true), "true");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["flag"], json!(true));
    }

    #[test]
    fn keeps_boolean_for_boolish_schema() {
        // boolish accepts boolean *or* the strings "true"/"false". When the
        // shell hands us a bare `true`, the boolean variant wins (per the
        // user's stated preference) — no coercion to string.
        let eff = effective_for("$schema:\n  flag: 'boolish(required)'");
        let a = make_assignment(&["flag"], json!(true), "true");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["flag"], json!(true));
    }

    #[test]
    fn keeps_number_for_number_schema() {
        let eff = effective_for("$schema:\n  count: 'number(required)'");
        let a = make_assignment(&["count"], json!(5), "5");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["count"], json!(5));
    }

    #[test]
    fn coerces_to_string_when_union_contains_string_arm() {
        // `flag: [boolean, string]` — string arm wins for ambiguous scalars
        // so the value is preserved losslessly.
        let eff = effective_for("$schema:\n  flag:\n    - boolean\n    - string");
        let a = make_assignment(&["flag"], json!(true), "true");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["flag"], json!("true"));
    }

    #[test]
    fn keeps_yaml_value_for_unknown_property() {
        // Schema exists but doesn't declare `bar` — fall back to YAML parse.
        let eff = effective_for("$schema:\n  title: 'string(required)'");
        let a = make_assignment(&["bar"], json!(true), "true");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["bar"], json!(true));
    }

    #[test]
    fn keeps_yaml_value_for_nested_path() {
        // Nested paths land in `object` typed properties which are opaque
        // in v1 — schema can't say anything about the leaf type, so YAML
        // parsing applies.
        let eff = effective_for("$schema:\n  user: 'object(required)'");
        let a = make_assignment(&["user", "verified"], json!(true), "true");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["user"], json!({ "verified": true }));
    }

    #[test]
    fn keeps_yaml_value_for_array_schema() {
        // `tags: string[]` with `tags=[a,b,c]` must remain a YAML array,
        // not get stringified to "[a,b,c]".
        let eff = effective_for("$schema:\n  tags: 'string[]'");
        let a = make_assignment(&["tags"], json!(["a", "b", "c"]), "[a, b, c]");
        let mut map: FrontmatterMap = FrontmatterMap::new();
        apply_all(&mut map, &[a], Some(&eff));
        assert_eq!(map["tags"], json!(["a", "b", "c"]));
    }
}
