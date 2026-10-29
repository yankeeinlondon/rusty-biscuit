//! The conservative projection of a raw JSON Schema: the literal top-level
//! properties it declares, each with the type the declared layer can prove.
//!
//! An effective schema's merged JSON Schema holds every layer, including the
//! ones no SimplifiedSchema represents (the baseline, a raw document schema,
//! a raw trigger payload), so this is the declared layer's lowest layer.
//! Full JSON Schema inference is not attempted: a construct whose values
//! cannot be named exactly projects to `unknown`, which absorbs every
//! alternative it joins, so the result is never narrower than the schema.
//!
//! | JSON Schema | Projects to |
//! |---|---|
//! | `properties` (with `required`) | one declaration per literal key; optional adds `null` |
//! | root `anyOf` / `oneOf` | each arm's declarations, folded like a root union (an arm without the key gives `unknown`) |
//! | root `allOf` | the members' declarations; a key typed differently by two members is `unknown`; `required` in any member removes `null` |
//! | `type` (string or list, `"null"` included) | the union of the named types; the file formats project to `file` |
//! | `const`, `enum` | the literal values |
//! | `items` | the item type as an array |
//! | `$ref`, `patternProperties`, `additionalProperties`, anything else | `unknown`, or no declaration for an unnamed key |

use indexmap::IndexMap;
use serde_json::{Map, Value};

use super::Shape;
use crate::markdown::schemas::{
    PropertyAtom, PropertyDef, SchemaShape, SimplifiedType, TypeExpr,
};

/// How deep the projection descends before a value is `unknown`.
const DEPTH_LIMIT: usize = 32;

/// The JSON Schema formats Darkmatter lowers `file` to.
const FILE_FORMATS: &[&str] = &["darkmatter-file-reference", "darkmatter-file"];

/// The literal top-level properties a raw JSON Schema declares, with their
/// projected types. See the [module documentation](self).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawDeclarations {
    /// Each declared name with its definition; `None` is a declared name
    /// whose type is `unknown`.
    properties: IndexMap<String, Option<PropertyDef>>,
}

impl RawDeclarations {
    /// Projects `schema`. Passive: reads only the value given.
    pub fn from_json_schema(schema: &Value) -> Self {
        let properties = root_members(schema, 0)
            .map(|members| {
                let required = members.required;
                members
                    .properties
                    .into_iter()
                    .map(|(name, shape)| {
                        let shape = present_if(shape, required.contains(&name));
                        (name, shape.to_definition())
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { properties }
    }

    /// The declaration of `name`: `None` when no literal key declares it,
    /// `Some(None)` when it is declared with an `unknown` type.
    pub fn get(&self, name: &str) -> Option<Option<&PropertyDef>> {
        self.properties.get(name).map(Option::as_ref)
    }

    /// Every declared name, in declaration order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.properties.keys().map(String::as_str)
    }
}

/// The declarations of one object-level schema: each literal key's value
/// type, and the keys that must be present. A key not required adds `null`
/// when the declarations are read ([`present_if`]).
#[derive(Default)]
struct Members {
    properties: IndexMap<String, Shape>,
    required: Vec<String>,
}

/// `shape` as a property's type: unchanged when the property must be
/// present, with `null` when it may be absent.
fn present_if(shape: Shape, required: bool) -> Shape {
    if required { shape } else { shape.union(&null_shape()) }
}

/// The literal declarations of `schema` as an object, or `None` when it
/// names none (a `$ref`, a boolean schema, too deep).
fn root_members(schema: &Value, depth: usize) -> Option<Members> {
    let map = schema.as_object()?;
    if depth > DEPTH_LIMIT {
        return None;
    }
    let mut parts: Vec<Members> = Vec::new();
    if map.contains_key("properties") || map.contains_key("required") {
        parts.push(own_members(map, depth));
    }
    for keyword in ["anyOf", "oneOf"] {
        if let Some(arms) = map.get(keyword).and_then(Value::as_array) {
            parts.push(fold_arms(arms, depth));
        }
    }
    if let Some(all) = map.get("allOf").and_then(Value::as_array) {
        parts.extend(all.iter().filter_map(|member| root_members(member, depth + 1)));
    }
    (!parts.is_empty()).then(|| intersect(parts))
}

fn own_members(map: &Map<String, Value>, depth: usize) -> Members {
    let required: Vec<String> = map
        .get("required")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    let mut properties = IndexMap::new();
    if let Some(declared) = map.get("properties").and_then(Value::as_object) {
        for (name, property) in declared {
            properties.insert(name.clone(), value_shape(property, depth + 1));
        }
    }
    Members { properties, required }
}

/// A root union: every key any arm declares, typed by the fold of every
/// arm with that arm's optionality. An arm that does not declare a key, or
/// names nothing, gives `unknown` for it. The folded types already carry
/// their `null`, so every key is reported required.
fn fold_arms(arms: &[Value], depth: usize) -> Members {
    let projected: Vec<Option<Members>> = arms.iter().map(|arm| root_members(arm, depth + 1)).collect();
    let mut names: Vec<&String> = Vec::new();
    for members in projected.iter().flatten() {
        for name in members.properties.keys() {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    let mut properties = IndexMap::new();
    for name in names {
        let mut folded: Option<Shape> = None;
        for members in &projected {
            let arm = members
                .as_ref()
                .and_then(|members| {
                    let shape = members.properties.get(name)?.clone();
                    Some(present_if(shape, members.required.contains(name)))
                })
                .unwrap_or(Shape::Unknown { non_null: false });
            folded = Some(folded.map_or_else(|| arm.clone(), |folded| folded.union(&arm)));
        }
        properties.insert(name.clone(), folded.unwrap_or(Shape::Unknown { non_null: false }));
    }
    let required = properties.keys().cloned().collect();
    Members { properties, required }
}

/// Simultaneous constraints: a key one part declares keeps that type, a key
/// two parts type differently is `unknown` (their intersection is not
/// represented), and a key any part requires must be present.
fn intersect(parts: Vec<Members>) -> Members {
    let required: Vec<String> = parts.iter().flat_map(|part| part.required.iter().cloned()).collect();
    let mut properties: IndexMap<String, Shape> = IndexMap::new();
    for part in parts {
        for (name, shape) in part.properties {
            match properties.get(&name) {
                None => {
                    properties.insert(name, shape);
                }
                Some(existing) if *existing == shape => {}
                Some(_) => {
                    properties.insert(name, Shape::Unknown { non_null: false });
                }
            }
        }
    }
    Members { properties, required }
}

fn null_shape() -> Shape {
    Shape::of_literal(&Value::Null)
}

/// The values one property schema admits, as far as the projection can
/// name them.
fn value_shape(schema: &Value, depth: usize) -> Shape {
    let unknown = Shape::Unknown { non_null: false };
    let Some(map) = schema.as_object() else {
        return unknown;
    };
    if depth > DEPTH_LIMIT || map.contains_key("$ref") || map.contains_key("$dynamicRef") {
        return unknown;
    }
    if let Some(value) = map.get("const") {
        return Shape::of_literal(value);
    }
    if let Some(values) = map.get("enum").and_then(Value::as_array) {
        return fold(values.iter().map(Shape::of_literal)).unwrap_or(unknown);
    }
    // Alternatives are a supertype of the schema they sit in (sibling
    // keywords only constrain them further), so a sibling `type` is not
    // needed for a sound result.
    for keyword in ["anyOf", "oneOf"] {
        if let Some(arms) = map.get(keyword).and_then(Value::as_array) {
            return fold(arms.iter().map(|arm| value_shape(arm, depth + 1))).unwrap_or(unknown);
        }
    }
    if let Some(all) = map.get("allOf").and_then(Value::as_array) {
        return match all.as_slice() {
            [only] if map.get("type").is_none() => value_shape(only, depth + 1),
            _ => unknown,
        };
    }
    let types: Vec<&str> = match map.get("type") {
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(names)) => {
            let names: Vec<&str> = names.iter().filter_map(Value::as_str).collect();
            if names.is_empty() {
                return unknown;
            }
            names
        }
        _ => return unknown,
    };
    fold(types.into_iter().map(|name| typed_shape(name, map, depth))).unwrap_or(unknown)
}

fn typed_shape(name: &str, map: &Map<String, Value>, depth: usize) -> Shape {
    let atom = |ty: SimplifiedType| Shape::of_type(&PropertyDef::Single(PropertyAtom::bare(ty)));
    match name {
        "null" => null_shape(),
        "string" => match map.get("format").and_then(Value::as_str) {
            Some(format) if FILE_FORMATS.contains(&format) => atom(SimplifiedType::File),
            _ => atom(SimplifiedType::String),
        },
        "number" | "integer" => atom(SimplifiedType::Number),
        "boolean" => atom(SimplifiedType::Boolean),
        "object" => object_shape(map, depth),
        "array" => array_shape(map, depth),
        _ => Shape::Unknown { non_null: false },
    }
}

/// An object with literal properties is an inline object of them; any other
/// object is the bare `object` type.
fn object_shape(map: &Map<String, Value>, depth: usize) -> Shape {
    let bare = Shape::of_type(&PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Object)));
    let Some(declared) = map.get("properties").and_then(Value::as_object) else {
        return bare;
    };
    let members = own_members(map, depth);
    let mut shape = SchemaShape::new();
    for name in declared.keys() {
        let def = members
            .properties
            .get(name)
            .and_then(|value| present_if(value.clone(), members.required.contains(name)).to_definition())
            .unwrap_or_else(|| PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Unknown)));
        shape.properties.insert(name.clone(), def);
    }
    let mut atom = PropertyAtom::bare(SimplifiedType::Object);
    atom.ty = TypeExpr::InlineObject(shape);
    Shape::of_type(&PropertyDef::Single(atom))
}

/// An array whose item type the projection can name exactly as one array
/// alternative; any other array is `unknown[]`.
fn array_shape(map: &Map<String, Value>, depth: usize) -> Shape {
    let mut any = PropertyAtom::bare(SimplifiedType::Unknown);
    any.is_array = true;
    let any_list = Shape::Arms { arms: vec![any], nullable: false };
    if map.contains_key("prefixItems") {
        return any_list;
    }
    let Some(items) = map.get("items") else {
        return any_list;
    };
    match value_shape(items, depth + 1) {
        Shape::Arms { arms, nullable: false } if !arms.is_empty() && arms.iter().all(|arm| !arm.is_array) => {
            let mut list = match arms.as_slice() {
                [single] => single.clone(),
                _ => {
                    let mut atom = PropertyAtom::bare(SimplifiedType::Unknown);
                    atom.ty = TypeExpr::Union(arms);
                    atom
                }
            };
            list.is_array = true;
            Shape::Arms { arms: vec![list], nullable: false }
        }
        _ => any_list,
    }
}

fn fold(shapes: impl IntoIterator<Item = Shape>) -> Option<Shape> {
    shapes.into_iter().reduce(|left, right| left.union(&right))
}
