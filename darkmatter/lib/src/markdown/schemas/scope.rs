//! Local `$ref` scope for lowered and raw JSON Schemas.
//!
//! A stored named type (recursive, or a shared acyclic one) lowers to one
//! body under the root `$defs` of the schema that reaches it, and every use is a `#/$defs/<key>` pointer into
//! that root. Three things keep such a pointer bound to the body it was
//! written for:
//!
//! - [`SchemaScope`] resolves a local reference against its own root, and a
//!   missing target is an error rather than an unconstrained schema;
//! - [`with_root_definitions`] gives a sub-schema compiled away from its root
//!   (a root-union arm, a discriminated arm, a property union arm) that
//!   root's definitions;
//! - [`fold_definitions`] moves one schema's definitions into another's root
//!   when the two are joined (a baseline or trigger payload merged into a
//!   document, a file arm nested in a root union), re-keying a colliding key
//!   and rewriting only the references the moved schema wrote.
//!
//! [`check_joined_references`] is the admission rule for a schema about to
//! be joined: its local `#/$defs/…` references must resolve in its own
//! table, and, when it carries a table, it may use no other reference form
//! whose meaning depends on the root it ends up under.
//!
//! ```mermaid
//! flowchart LR
//!   incoming["incoming schema<br/>own $defs"] --> check["check_joined_references"]
//!   check --> plan["shared key? same body → share<br/>else → key-2, key-3, …"]
//!   plan --> rewrite["rewrite incoming #/$defs refs<br/>(schema keywords only)"]
//!   rewrite --> root["receiving root $defs"]
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::{Map, Value};

use super::SchemaError;

const DEFS_POINTER: &str = "#/$defs/";

/// Keywords whose value is a mapping of names to schemas.
const SCHEMA_MAPS: &[&str] = &["properties", "patternProperties", "$defs", "definitions", "dependentSchemas"];

/// Keywords whose value is one schema, or a list of schemas.
const SCHEMA_VALUES: &[&str] = &[
    "additionalProperties",
    "items",
    "prefixItems",
    "contains",
    "propertyNames",
    "unevaluatedItems",
    "unevaluatedProperties",
    "not",
    "if",
    "then",
    "else",
    "allOf",
    "anyOf",
    "oneOf",
    "contentSchema",
    "additionalItems",
];

/// Keywords that change which document a reference resolves in, or bind a
/// name that a merged root would share with other layers.
const SCOPE_KEYWORDS: &[&str] = &["$id", "$anchor", "$dynamicAnchor", "$dynamicRef"];

/// A schema document that local references resolve against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SchemaScope<'a> {
    root: &'a Value,
    label: &'a str,
}

impl<'a> SchemaScope<'a> {
    /// The scope rooted at `root`, named `the schema` in errors.
    pub(crate) fn new(root: &'a Value) -> Self {
        Self { root, label: "the schema" }
    }

    /// The scope rooted at `root`, named `label` in errors (for example
    /// `the baseline` or a trigger file).
    pub(crate) fn named(root: &'a Value, label: &'a str) -> Self {
        Self { root, label }
    }

    /// The target of `schema`'s `$ref` when it is local to this scope (`#`,
    /// `#/…`, or the empty reference), decoding `%XX` and `~0`/`~1`.
    ///
    /// `Ok(None)` when `schema` has no `$ref`, or its reference is not local
    /// (an external URI or an anchor), which only the validator can follow.
    /// The caller still applies `schema`'s sibling keywords; `location` is
    /// `schema`'s JSON Pointer, for the error.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::UnresolvedReference`] when the reference is not a
    /// string or names nothing in the root.
    pub(crate) fn resolve(&self, schema: &'a Value, location: &str) -> Result<Option<&'a Value>, SchemaError> {
        let Some(reference) = schema.get("$ref") else {
            return Ok(None);
        };
        let Some(text) = reference.as_str() else {
            return Err(self.unresolved(&reference.to_string(), location));
        };
        self.resolve_text(text, location)
    }

    /// [`Self::resolve`] for the text of a `$ref`, for a schema that does not
    /// live in this scope's root.
    pub(crate) fn resolve_text(&self, text: &str, location: &str) -> Result<Option<&'a Value>, SchemaError> {
        let Some(pointer) = local_pointer(text) else {
            return Ok(None);
        };
        self.root
            .pointer(&pointer)
            .map(Some)
            .ok_or_else(|| self.unresolved(text, location))
    }

    /// `schema` followed by the target of each local reference in turn, until
    /// a schema with no local `$ref`. A walker applies every element: each
    /// hop's siblings constrain the value along with its target.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::UnresolvedReference`] for a missing target, and for a
    /// chain that revisits a schema without reaching one that is not a
    /// reference (`a → b → a`), which constrains nothing and must not be
    /// read as an unconstrained schema.
    pub(crate) fn chain(&self, schema: &'a Value, location: &str) -> Result<Vec<&'a Value>, SchemaError> {
        let mut hops = vec![schema];
        let mut current = schema;
        while let Some(target) = self.resolve(current, location)? {
            if hops.iter().any(|seen| std::ptr::eq(*seen, target)) {
                let reference = current.get("$ref").and_then(Value::as_str).unwrap_or_default();
                return Err(self.unresolved(reference, location));
            }
            hops.push(target);
            current = target;
        }
        Ok(hops)
    }

    fn unresolved(&self, reference: &str, location: &str) -> SchemaError {
        SchemaError::UnresolvedReference {
            schema: self.label.to_string(),
            reference: reference.to_string(),
            location: display_location(location),
        }
    }
}

/// `sub` with `root`'s `$defs`, so a sub-schema compiled away from its root
/// keeps every `#/$defs/…` reference in scope. Generated keys live only at
/// the root, so the root's table is the one those references meant; `sub`
/// is returned unchanged when `root` has none.
pub(crate) fn with_root_definitions(sub: &Value, root: &Value) -> Value {
    let mut sub = sub.clone();
    if let (Some(defs), Some(map)) = (root.get("$defs"), sub.as_object_mut()) {
        map.insert("$defs".into(), defs.clone());
    }
    sub
}

/// Checks the references of `schema`, a schema about to be joined to
/// another root (`label` names it in errors).
///
/// Every `#/$defs/…` reference must resolve in `schema`'s own root: once
/// joined, a missing target could bind to another layer's definition. When
/// `schema` carries a root `$defs`, those definitions move to the joined
/// root, so a reference whose meaning depends on the root it sits under is
/// refused: any other local pointer (`#`, `#/properties/…`), an external
/// target, and the scope keywords `$id`, `$anchor`, `$dynamicAnchor`, and
/// `$dynamicRef`. Without a root `$defs` nothing moves, and those forms keep
/// the behavior they had.
///
/// ## Errors
///
/// [`SchemaError::UnresolvedReference`] or
/// [`SchemaError::UnsupportedMergeReference`], naming the schema location.
pub(crate) fn check_joined_references(schema: &Value, label: &str) -> Result<(), SchemaError> {
    let moves_definitions = schema.get("$defs").is_some();
    let scope = SchemaScope::named(schema, label);
    let mut outcome = Ok(());
    walk_schemas(schema, &mut String::new(), &mut |object, location| {
        if outcome.is_err() {
            return;
        }
        outcome = check_one(object, location, moves_definitions, &scope, label);
    });
    outcome
}

fn check_one(
    object: &Map<String, Value>,
    location: &str,
    moves_definitions: bool,
    scope: &SchemaScope<'_>,
    label: &str,
) -> Result<(), SchemaError> {
    if moves_definitions && let Some(keyword) = SCOPE_KEYWORDS.iter().find(|keyword| object.contains_key(**keyword)) {
        return Err(SchemaError::UnsupportedMergeReference {
            schema: label.to_string(),
            location: display_location(location),
            reason: format!("`{keyword}` changes the scope its definitions resolve in"),
        });
    }
    let Some(reference) = object.get("$ref") else {
        return Ok(());
    };
    let Some(text) = reference.as_str() else {
        return match moves_definitions {
            true => Err(scope.unresolved(&reference.to_string(), location)),
            false => Ok(()),
        };
    };
    if text.starts_with(DEFS_POINTER) {
        let pointer = local_pointer(text).expect("a `#/` reference is local");
        return match scope.root.pointer(&pointer) {
            Some(_) => Ok(()),
            None => Err(scope.unresolved(text, location)),
        };
    }
    if !moves_definitions {
        return Ok(());
    }
    let reason = match local_pointer(text) {
        Some(_) => format!("the local reference `{text}` would resolve against the merged root"),
        None => format!("the external reference `{text}` is outside the merged schema"),
    };
    Err(SchemaError::UnsupportedMergeReference {
        schema: label.to_string(),
        location: display_location(location),
        reason,
    })
}

/// Moves `incoming`'s root `$defs` into `receiving`, the `$defs` of the root
/// `incoming` (or its properties) is joined under.
///
/// A key `receiving` lacks keeps its name. A key `receiving` holds with the
/// same body, whose references reach only bodies that are shared too, is
/// shared. Any other colliding key gets the first free `key-2`, `key-3`, …,
/// and every `incoming` reference to it is rewritten, in schema-bearing
/// keywords only: literal data under `const`, `enum`, `default`,
/// `examples`, and extension keywords is never touched. `receiving`'s own
/// bodies and references never change, so two different bodies are never
/// merged by last-writer-wins.
///
/// Call [`check_joined_references`] on `incoming` first: a reference that
/// names nothing in its own table would otherwise bind to a receiving one.
pub(crate) fn fold_definitions(receiving: &mut Map<String, Value>, incoming: &mut Value) {
    let Some(object) = incoming.as_object_mut() else {
        return;
    };
    if !matches!(object.get("$defs"), Some(Value::Object(_))) {
        return;
    }
    let Some(Value::Object(defs)) = object.shift_remove("$defs") else {
        unreachable!("checked to be an object");
    };
    let renames = plan_renames(receiving, &defs);
    if !renames.is_empty() {
        rename_references(incoming, &renames);
    }
    for (key, mut body) in defs {
        if !renames.is_empty() {
            rename_references(&mut body, &renames);
        }
        let key = renames.get(&key).cloned().unwrap_or(key);
        receiving.entry(key).or_insert(body);
    }
}

/// Drops from `schema`'s root `$defs` every definition no property `kept`
/// admits reaches, directly or through another definition.
///
/// A merge copies only the lower layer's properties the higher one does not
/// declare; the definitions only an overridden property reached would
/// otherwise join the merged root unreferenced.
pub(crate) fn retain_reached_definitions(schema: &mut Value, kept: impl Fn(&str) -> bool) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    let mut reached = BTreeSet::new();
    let mut pending: Vec<String> = object
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(name, _)| kept(name))
        .flat_map(|(_, property)| referenced_definitions(property))
        .collect();
    let Some(Value::Object(defs)) = object.get_mut("$defs") else {
        return;
    };
    while let Some(key) = pending.pop() {
        if let Some(body) = defs.get(&key)
            && reached.insert(key)
        {
            pending.extend(referenced_definitions(body));
        }
    }
    defs.retain(|key, _| reached.contains(key));
}

/// The `incoming` keys that must move, with their new names.
fn plan_renames(receiving: &Map<String, Value>, incoming: &Map<String, Value>) -> BTreeMap<String, String> {
    let colliding: Vec<&String> = incoming.keys().filter(|key| receiving.contains_key(*key)).collect();
    let mut moved: BTreeSet<&str> = colliding
        .iter()
        .filter(|key| receiving[key.as_str()] != incoming[key.as_str()])
        .map(|key| key.as_str())
        .collect();
    // A shared body is the same body only if what it references is shared.
    loop {
        let before = moved.len();
        for key in &colliding {
            if !moved.contains(key.as_str())
                && referenced_definitions(&incoming[key.as_str()]).iter().any(|target| moved.contains(target.as_str()))
            {
                moved.insert(key.as_str());
            }
        }
        if moved.len() == before {
            break;
        }
    }
    let mut taken: BTreeSet<String> = receiving.keys().chain(incoming.keys()).cloned().collect();
    let mut renames = BTreeMap::new();
    for key in incoming.keys().filter(|key| moved.contains(key.as_str())) {
        let renamed = (2..)
            .map(|suffix| format!("{key}-{suffix}"))
            .find(|candidate| !taken.contains(candidate))
            .expect("an unbounded suffix range has a free key");
        taken.insert(renamed.clone());
        renames.insert(key.clone(), renamed);
    }
    renames
}

/// The `$defs` keys `schema`'s `#/$defs/…` references name, decoded.
fn referenced_definitions(schema: &Value) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    walk_schemas(schema, &mut String::new(), &mut |object, _| {
        if let Some((key, _)) = object.get("$ref").and_then(Value::as_str).and_then(split_definition_reference) {
            keys.insert(key);
        }
    });
    keys
}

/// Rewrites every `#/$defs/<old>…` reference in `schema` whose key `renames`
/// maps, keeping any pointer below the definition.
pub(crate) fn rename_references(schema: &mut Value, renames: &BTreeMap<String, String>) {
    walk_schemas_mut(schema, &mut |object| {
        let Some(Value::String(reference)) = object.get_mut("$ref") else {
            return;
        };
        let Some((key, rest)) = split_definition_reference(reference) else {
            return;
        };
        if let Some(renamed) = renames.get(&key) {
            *reference = format!("{}{rest}", super::simplified::definition_pointer(renamed));
        }
    });
}

/// The decoded first token and the remaining pointer of a `#/$defs/…`
/// reference.
fn split_definition_reference(reference: &str) -> Option<(String, String)> {
    let rest = reference.strip_prefix(DEFS_POINTER)?;
    let (token, below) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let token = percent_decode(token).replace("~1", "/").replace("~0", "~");
    Some((token, below.to_string()))
}

/// The JSON Pointer a local reference names: `""` for `#` and the empty
/// reference, the percent-decoded fragment for `#/…`; `None` otherwise.
fn local_pointer(reference: &str) -> Option<String> {
    match reference {
        "" | "#" => Some(String::new()),
        _ => reference.strip_prefix('#').filter(|fragment| fragment.starts_with('/')).map(percent_decode),
    }
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escape = (bytes[index] == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escape {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).unwrap_or_else(|_| text.to_string())
}

fn display_location(location: &str) -> String {
    if location.is_empty() { "/".to_string() } else { location.to_string() }
}

/// Appends `/` and `key` as an escaped JSON Pointer token, allocating
/// nothing for the walk's many keys that need no escape.
fn push_pointer_token(location: &mut String, key: &str) {
    location.push('/');
    for character in key.chars() {
        match character {
            '~' => location.push_str("~0"),
            '/' => location.push_str("~1"),
            other => location.push(other),
        }
    }
}

/// Visits every schema object reachable from `schema` through
/// schema-bearing keywords, with its JSON Pointer.
fn walk_schemas<'v>(schema: &'v Value, location: &mut String, visit: &mut dyn FnMut(&'v Map<String, Value>, &str)) {
    let Value::Object(object) = schema else {
        return;
    };
    visit(object, location);
    for (keyword, value) in object {
        let mark = location.len();
        push_pointer_token(location, keyword);
        if SCHEMA_MAPS.contains(&keyword.as_str()) {
            if let Value::Object(entries) = value {
                for (name, sub) in entries {
                    let inner = location.len();
                    push_pointer_token(location, name);
                    walk_schemas(sub, location, visit);
                    location.truncate(inner);
                }
            }
        } else if SCHEMA_VALUES.contains(&keyword.as_str()) {
            match value {
                Value::Array(items) => {
                    for (index, sub) in items.iter().enumerate() {
                        let inner = location.len();
                        let _ = write!(location, "/{index}");
                        walk_schemas(sub, location, visit);
                        location.truncate(inner);
                    }
                }
                other => walk_schemas(other, location, visit),
            }
        }
        location.truncate(mark);
    }
}

fn walk_schemas_mut(schema: &mut Value, visit: &mut dyn FnMut(&mut Map<String, Value>)) {
    let Value::Object(object) = schema else {
        return;
    };
    visit(object);
    for (keyword, value) in object.iter_mut() {
        if SCHEMA_MAPS.contains(&keyword.as_str()) {
            if let Value::Object(entries) = value {
                entries.values_mut().for_each(|sub| walk_schemas_mut(sub, visit));
            }
        } else if SCHEMA_VALUES.contains(&keyword.as_str()) {
            match value {
                Value::Array(items) => items.iter_mut().for_each(|sub| walk_schemas_mut(sub, visit)),
                other => walk_schemas_mut(other, visit),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_reference_resolves_in_its_own_root_and_a_missing_target_is_an_error() {
        let root = json!({"$defs": {"a/b": {"type": "string"}, "item": {"$ref": "#/$defs/a~1b"}}});
        let scope = SchemaScope::new(&root);
        let use_site = json!({"$ref": "#/$defs/item", "minLength": 1});
        let chain = scope.chain(&use_site, "/properties/p").expect("resolves");
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[2], &json!({"type": "string"}));
        let missing = json!({"$ref": "#/$defs/nope"});
        assert!(matches!(
            scope.resolve(&missing, "/properties/q"),
            Err(SchemaError::UnresolvedReference { location, .. }) if location == "/properties/q"
        ));
        assert!(scope.resolve(&json!({"$ref": "https://example.com/s.json"}), "").expect("external").is_none());
        assert!(scope.resolve(&json!({"type": "string"}), "").expect("no ref").is_none());
    }

    #[test]
    fn a_reference_chain_that_never_reaches_a_schema_is_an_error() {
        let root = json!({"$defs": {"a": {"$ref": "#/$defs/b"}, "b": {"$ref": "#/$defs/a"}}});
        let scope = SchemaScope::new(&root);
        assert!(matches!(
            scope.chain(&json!({"$ref": "#/$defs/a"}), "/properties/p"),
            Err(SchemaError::UnresolvedReference { .. })
        ));
    }

    #[test]
    fn a_sub_schema_takes_its_roots_definitions() {
        let root = json!({"anyOf": [{"$ref": "#/$defs/item"}], "$defs": {"item": {"type": "object"}}});
        let wrapped = with_root_definitions(&root["anyOf"][0], &root);
        assert_eq!(wrapped["$defs"], root["$defs"]);
        let plain = json!({"anyOf": [{"type": "string"}]});
        assert_eq!(with_root_definitions(&plain["anyOf"][0], &plain), json!({"type": "string"}));
    }

    #[test]
    fn folding_shares_equal_bodies_and_rekeys_different_ones() {
        let mut receiving = json!({
            "item": {"type": "object", "properties": {"next": {"$ref": "#/$defs/item"}}},
            "node": {"type": "string"},
        })
        .as_object()
        .cloned()
        .expect("object");
        let mut incoming = json!({
            "type": "object",
            "properties": {
                "a": {"$ref": "#/$defs/item"},
                "b": {"$ref": "#/$defs/node", "const": {"$ref": "#/$defs/node"}, "default": {"$defs": {}}},
            },
            "$defs": {
                "item": {"type": "object", "properties": {"next": {"$ref": "#/$defs/item"}}},
                "node": {"type": "array", "items": {"$ref": "#/$defs/node"}, "examples": [{"$ref": "#/$defs/node"}]},
            },
        });
        fold_definitions(&mut receiving, &mut incoming);
        assert!(incoming.get("$defs").is_none());
        assert_eq!(incoming["properties"]["a"]["$ref"], "#/$defs/item", "an equal body is shared");
        assert_eq!(incoming["properties"]["b"]["$ref"], "#/$defs/node-2");
        assert_eq!(incoming["properties"]["b"]["const"], json!({"$ref": "#/$defs/node"}), "literal data is untouched");
        assert_eq!(incoming["properties"]["b"]["default"], json!({"$defs": {}}));
        assert_eq!(receiving["node"], json!({"type": "string"}), "the receiving body never changes");
        assert_eq!(receiving["node-2"]["items"]["$ref"], "#/$defs/node-2");
        assert_eq!(receiving["node-2"]["examples"], json!([{"$ref": "#/$defs/node"}]));
    }

    #[test]
    fn an_equal_body_that_references_a_moved_key_moves_too() {
        let mut receiving = json!({
            "a": {"properties": {"b": {"$ref": "#/$defs/b"}}},
            "b": {"type": "string"},
        })
        .as_object()
        .cloned()
        .expect("object");
        let mut incoming = json!({
            "properties": {"x": {"$ref": "#/$defs/a"}},
            "$defs": {
                "a": {"properties": {"b": {"$ref": "#/$defs/b"}}},
                "b": {"type": "number"},
            },
        });
        fold_definitions(&mut receiving, &mut incoming);
        assert_eq!(incoming["properties"]["x"]["$ref"], "#/$defs/a-2");
        assert_eq!(receiving["a-2"]["properties"]["b"]["$ref"], "#/$defs/b-2");
        assert_eq!(receiving["b-2"], json!({"type": "number"}));
    }

    #[test]
    fn joined_references_must_stay_in_their_own_table() {
        let unresolved = json!({"properties": {"p": {"$ref": "#/$defs/item"}}});
        assert!(matches!(
            check_joined_references(&unresolved, "the baseline"),
            Err(SchemaError::UnresolvedReference { location, .. }) if location == "/properties/p"
        ));
        // Without a table nothing moves, so other forms keep their meaning.
        assert!(check_joined_references(&json!({"properties": {"p": {"$ref": "#/properties/q"}}}), "b").is_ok());
        for (schema, needle) in [
            (json!({"$defs": {"i": {"$id": "urn:x"}}, "properties": {}}), "$id"),
            (json!({"$defs": {}, "properties": {"p": {"$ref": "#"}}}), "local reference"),
            (json!({"$defs": {}, "properties": {"p": {"$ref": "other.json#/$defs/x"}}}), "external"),
        ] {
            match check_joined_references(&schema, "the baseline") {
                Err(SchemaError::UnsupportedMergeReference { reason, .. }) => {
                    assert!(reason.contains(needle), "{reason}");
                }
                other => panic!("{schema}: {other:?}"),
            }
        }
    }
}
