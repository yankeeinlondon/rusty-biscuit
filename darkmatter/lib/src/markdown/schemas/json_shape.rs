//! The JSON Schema translator: compiled or raw Draft 2020-12 JSON Schema →
//! the conversion core's neutral [`Shape`].
//!
//! The translator maps syntax only. It never decides whether a value is kept,
//! converted, or rejected; [`crate::markdown::coercion`] does. Raw schemas
//! stay authoritative: a keyword the shape cannot express exactly (`allOf`,
//! `not`, `if`/`then`/`else`, `dependent*`, `propertyNames`, `contains`,
//! `unevaluated*`, `oneOf`'s exactly-one rule, a `$ref` with constraining
//! siblings, a cyclic or over-deep reference, Darkmatter's validating custom
//! keywords) drives no conversion, and the subschema that carries it becomes a
//! passive [`Assertion`] checked by `jsonschema` on the final value, so it is
//! never unconstrained success. A recursive reference expands a few levels
//! ([`CYCLE_EXPANSIONS`]); below that the value is validation-only. Callers
//! still validate the final document with the complete schema.
//!
//! Local references resolve through the document's own identity: `$id`
//! changes the base URI of the subschema that declares it, `$anchor` names a
//! node within its resource, and a `#/...` fragment is a JSON Pointer into the
//! resource the reference resolves to, not necessarily the document root. A
//! reference that resolves to nothing in the document (a remote or unknown
//! URI) is never fetched; it stays validation-only, and building the
//! validator reports it as before.
//!
//! Pattern keys use [`PatternScope::AllKeys`], JSON Schema's rule. The
//! SimplifiedSchema "literal keys win" convenience is already encoded in the
//! compiled `patternProperties` regex (a negative lookahead over the declared
//! names), so the compiled form translates without a special case.
//!
//! The rules were decided in `2026-10-28-schema-enhancements` ("Raw JSON
//! Schema Conversion Boundaries").

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use jsonschema::{Uri, Validator};
use serde_json::{Map, Value};

use super::format::{
    DARKMATTER_BOOLISH_FORMAT, DARKMATTER_INTEGER_KEYWORD, DARKMATTER_JSON_FORMAT, DARKMATTER_NUMBERLIKE_FORMAT, DARKMATTER_SCHEMA_KEYWORD,
    DARKMATTER_TYPE_ANNOTATION, DARKMATTER_TYPE_DEFINITION_KEYWORD, DARKMATTER_URL_SCHEME_KEYWORD,
    DARKMATTER_YAML_FORMAT,
};
use super::file_match::DARKMATTER_MATCH_KEYWORD;
use super::simplified::{DRAFT_2020_12, SimplifiedType};
use super::validate::build_structural_validator;
use crate::markdown::coercion::{
    Additional, ArrayShape, Assertion, ContentFormat, IntegerRule, KeyMatcher, NumberShape, NumericBounds,
    ObjectShape, PatternScope, PatternSlot, PropertySlot, Shape, TextPattern, TextShape, TupleShape, TupleSlot,
};

/// The base URI `jsonschema` gives a document without `$id`.
const DOCUMENT_BASE: &str = "json-schema:///";

/// How many references one translation path expands before the rest of the
/// path becomes validation-only.
const MAX_REFERENCE_DEPTH: usize = 32;

/// How many times one translation path may expand the same reference target:
/// a recursive schema converts this many levels deep, and below that the
/// value is validation-only.
const CYCLE_EXPANSIONS: usize = 4;

/// How many reference expansions one translation performs in total, so a
/// schema whose references branch cannot grow the shape exponentially.
const REFERENCE_BUDGET: usize = 4096;

/// Keywords that hold one subschema.
const SINGLE_SCHEMA_KEYWORDS: &[&str] = &[
    "items",
    "additionalItems",
    "additionalProperties",
    "not",
    "if",
    "then",
    "else",
    "contains",
    "propertyNames",
    "unevaluatedItems",
    "unevaluatedProperties",
    "contentSchema",
];

/// Keywords that hold a list of subschemas.
const SCHEMA_LIST_KEYWORDS: &[&str] = &["prefixItems", "allOf", "anyOf", "oneOf"];

/// Keywords that hold a map of name → subschema.
const SCHEMA_MAP_KEYWORDS: &[&str] = &["properties", "patternProperties", "$defs", "definitions", "dependentSchemas"];

/// Validating keywords the shape does not express; their presence makes the
/// subschema an assertion.
const UNMODELED_KEYWORDS: &[&str] = &[
    "allOf",
    "not",
    "if",
    "then",
    "else",
    "dependentRequired",
    "dependentSchemas",
    "dependencies",
    "propertyNames",
    "contains",
    "minContains",
    "maxContains",
    "unevaluatedItems",
    "unevaluatedProperties",
    "additionalItems",
    "$dynamicRef",
    "$recursiveRef",
    DARKMATTER_URL_SCHEME_KEYWORD,
    DARKMATTER_TYPE_DEFINITION_KEYWORD,
    DARKMATTER_SCHEMA_KEYWORD,
    // Judged structurally here: an absolute path by its full name, a value
    // that needs a context by its syntax. The caller's own validator judges
    // the rest in its request context.
    DARKMATTER_MATCH_KEYWORD,
];

const STRING_KEYWORDS: &[&str] = &["minLength", "maxLength", "pattern", "format"];
const NUMBER_KEYWORDS: &[&str] = &["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum", "multipleOf"];
const ARRAY_KEYWORDS: &[&str] = &["items", "prefixItems", "minItems", "maxItems", "uniqueItems"];
const OBJECT_KEYWORDS: &[&str] = &[
    "properties",
    "required",
    "patternProperties",
    "additionalProperties",
    "minProperties",
    "maxProperties",
];

/// Translates a JSON Schema document into a [`Shape`].
///
/// Works on what [`super::simplified::convert`] emits and on raw Draft
/// 2020-12 schemas alike. Pure and side-effect-free: nothing is fetched or
/// read, and a validator is compiled only when an assertion first runs.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::coercion::bind;
/// use darkmatter::markdown::schemas::shape_from_json_schema;
/// use serde_json::json;
///
/// let schema = json!({
///     "$defs": { "count": { "type": "number" } },
///     "type": "object",
///     "properties": { "n": { "$ref": "#/$defs/count" } }
/// });
/// let shape = shape_from_json_schema(&schema);
/// let value = json!({ "n": "3" });
/// assert_eq!(bind(&shape, &value).unwrap().value(), &json!({ "n": 3 }));
/// ```
pub fn shape_from_json_schema(schema: &Value) -> Shape {
    let index = Index::build(schema);
    let translator = Translator {
        index,
        budget: std::cell::Cell::new(REFERENCE_BUDGET),
        document: Arc::new(Document {
            root: schema.clone(),
            embedded: OnceLock::new(),
        }),
    };
    translator.node(schema, &mut Vec::new())
}

/// The document an assertion validates against when its subschema holds a
/// reference.
struct Document {
    root: Value,
    /// The root with an absolute `$id`, embedded so a wrapper can `$ref` any
    /// node by pointer; built on first use.
    embedded: OnceLock<(Value, String)>,
}

impl Document {
    fn embedded(&self, root_base: &str) -> &(Value, String) {
        self.embedded.get_or_init(|| {
            let mut root = self.root.clone();
            if let Value::Object(map) = &mut root {
                map.insert("$id".into(), Value::String(root_base.to_string()));
            }
            (root, root_base.to_string())
        })
    }
}

/// Resource and anchor identities of one document, and every subschema's base
/// URI and JSON Pointer.
struct Index<'s> {
    resources: HashMap<String, &'s Value>,
    anchors: HashMap<String, &'s Value>,
    bases: HashMap<*const Value, String>,
    pointers: HashMap<*const Value, String>,
}

impl<'s> Index<'s> {
    fn build(root: &'s Value) -> Self {
        let mut index = Self {
            resources: HashMap::new(),
            anchors: HashMap::new(),
            bases: HashMap::new(),
            pointers: HashMap::new(),
        };
        index.resources.insert(DOCUMENT_BASE.to_string(), root);
        index.walk(root, DOCUMENT_BASE, String::new());
        index
    }

    fn walk(&mut self, node: &'s Value, base: &str, pointer: String) {
        let mut base = base.to_string();
        if let Value::Object(map) = node {
            if let Some(Value::String(id)) = map.get("$id")
                && let Some(resolved) = resolve_reference(&base, id)
            {
                base = strip_fragment(&resolved).to_string();
                self.resources.insert(base.clone(), node);
            }
            for keyword in ["$anchor", "$dynamicAnchor"] {
                if let Some(Value::String(anchor)) = map.get(keyword) {
                    self.anchors.insert(format!("{base}#{anchor}"), node);
                }
            }
        }
        self.bases.insert(node as *const Value, base.clone());
        self.pointers.insert(node as *const Value, pointer.clone());
        let Value::Object(map) = node else {
            return;
        };
        for (keyword, child) in map {
            let child_pointer = format!("{pointer}/{}", escape_pointer_token(keyword));
            if SCHEMA_MAP_KEYWORDS.contains(&keyword.as_str()) {
                if let Value::Object(entries) = child {
                    for (name, schema) in entries {
                        self.walk(schema, &base, format!("{child_pointer}/{}", escape_pointer_token(name)));
                    }
                }
            } else if SCHEMA_LIST_KEYWORDS.contains(&keyword.as_str()) || keyword == "items" {
                match child {
                    Value::Array(items) => {
                        for (position, schema) in items.iter().enumerate() {
                            self.walk(schema, &base, format!("{child_pointer}/{position}"));
                        }
                    }
                    schema if keyword == "items" => self.walk(schema, &base, child_pointer),
                    _ => {}
                }
            } else if SINGLE_SCHEMA_KEYWORDS.contains(&keyword.as_str()) {
                self.walk(child, &base, child_pointer);
            }
        }
    }

    /// The node a `$ref` written at `from` names, if it is in this document.
    fn resolve(&self, from: &Value, reference: &str) -> Option<&'s Value> {
        let base = self.bases.get(&(from as *const Value))?;
        let target = resolve_reference(base, reference)?;
        let (resource_uri, fragment) = match target.split_once('#') {
            Some((resource, fragment)) => (resource, fragment),
            None => (target.as_str(), ""),
        };
        let resource = self.resources.get(resource_uri)?;
        let fragment = percent_decode(fragment)?;
        if fragment.is_empty() {
            Some(resource)
        } else if fragment.starts_with('/') {
            resource.pointer(&fragment)
        } else {
            self.anchors.get(&format!("{resource_uri}#{fragment}")).copied()
        }
    }
}

struct Translator<'s> {
    index: Index<'s>,
    /// Reference expansions left; see [`REFERENCE_BUDGET`].
    budget: std::cell::Cell<usize>,
    document: Arc<Document>,
}

/// The JSON types a `type` keyword names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Null,
    Boolean,
    Number,
    Integer,
    String,
    Array,
    Object,
}

impl Kind {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "null" => Self::Null,
            "boolean" => Self::Boolean,
            "number" => Self::Number,
            "integer" => Self::Integer,
            "string" => Self::String,
            "array" => Self::Array,
            "object" => Self::Object,
            _ => return None,
        })
    }

    fn of(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(_) => Self::Boolean,
            Value::Number(_) => Self::Number,
            Value::String(_) => Self::String,
            Value::Array(_) => Self::Array,
            Value::Object(_) => Self::Object,
        }
    }
}

/// A translated part of one subschema and whether it captures everything that
/// part asserts.
struct Part {
    shape: Shape,
    exact: bool,
}

impl Part {
    fn exact(shape: Shape) -> Self {
        Self { shape, exact: true }
    }
}

impl<'s> Translator<'s> {
    fn node(&self, node: &'s Value, references: &mut Vec<*const Value>) -> Shape {
        match node {
            Value::Bool(true) => Shape::Any,
            Value::Bool(false) => Shape::Union(Vec::new()),
            Value::Object(map) => self.object_node(node, map, references),
            // Not a schema; building the validator reports it.
            _ => Shape::Any,
        }
    }

    fn object_node(&self, node: &'s Value, map: &'s Map<String, Value>, references: &mut Vec<*const Value>) -> Shape {
        // The `type-definition` and `schema` meta types hold declarations, not
        // data: `true` is not the bare schema name `"true"`.
        if map.contains_key(DARKMATTER_TYPE_DEFINITION_KEYWORD) || map.contains_key(DARKMATTER_SCHEMA_KEYWORD) {
            return Shape::Any.assert(self.node_assertion(node));
        }
        let mut exact = !map.keys().any(|keyword| UNMODELED_KEYWORDS.contains(&keyword.as_str()));

        let typed = map.contains_key("type");
        let base = self.base(map, references);
        exact &= base.exact;

        let mut applicators: Vec<Shape> = Vec::new();
        if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
            applicators.push(self.reference(node, reference, references));
        } else if map.contains_key("$ref") {
            exact = false;
        }
        if let Some(options) = map.get("anyOf").and_then(Value::as_array) {
            applicators.push(union(options.iter().map(|option| self.node(option, references)).collect()));
        }
        if let Some(options) = map.get("oneOf").and_then(Value::as_array) {
            // Exactly-one is not a union rule: the assertion decides it.
            exact = false;
            applicators.push(union(options.iter().map(|option| self.node(option, references)).collect()));
        }

        let mut parts: Vec<Shape> = Vec::new();
        if typed {
            parts.push(base.shape);
            parts.extend(applicators);
        } else {
            parts.extend(applicators);
            parts.push(base.shape);
        }
        parts.retain(|part| !matches!(part, Shape::Any));
        if parts.len() > 1 {
            exact = false;
        }
        let driver = parts.into_iter().next().unwrap_or(Shape::Any);
        if exact {
            driver
        } else {
            driver.assert(self.node_assertion(node))
        }
    }

    /// The `type`, `const`, `enum`, and per-kind keywords of one subschema.
    fn base(&self, map: &'s Map<String, Value>, references: &mut Vec<*const Value>) -> Part {
        let declared: Option<Vec<Kind>> = match map.get("type") {
            None => None,
            Some(Value::String(name)) => match Kind::parse(name) {
                Some(kind) => Some(vec![kind]),
                None => return Part { shape: Shape::Any, exact: false },
            },
            Some(Value::Array(names)) => {
                let kinds: Option<Vec<Kind>> = names.iter().map(|name| name.as_str().and_then(Kind::parse)).collect();
                match kinds {
                    Some(kinds) => Some(kinds),
                    None => return Part { shape: Shape::Any, exact: false },
                }
            }
            Some(_) => return Part { shape: Shape::Any, exact: false },
        };

        if let Some(constant) = map.get("const") {
            let admitted = declared.as_ref().is_none_or(|kinds| admits(kinds, constant));
            let only_type = map.keys().all(|keyword| keyword == "const" || keyword == "type" || is_annotation(keyword));
            return Part {
                shape: Shape::Literal(constant.clone()),
                exact: admitted && only_type,
            };
        }
        if let Some(members) = map.get("enum").and_then(Value::as_array) {
            return self.enumeration(map, members, declared.as_deref());
        }

        let mentioned = |keywords: &[&str]| keywords.iter().any(|keyword| map.contains_key(*keyword));
        let mut unconstrained_rest = None;
        let kinds = match declared {
            Some(kinds) => kinds,
            None => {
                // A typeless subschema constrains only values of the kinds its
                // keywords name; a value of any other kind passes unchanged,
                // never by conversion.
                let constrained: Vec<Kind> = [
                    (STRING_KEYWORDS, Kind::String),
                    (NUMBER_KEYWORDS, Kind::Number),
                    (ARRAY_KEYWORDS, Kind::Array),
                    (OBJECT_KEYWORDS, Kind::Object),
                ]
                .into_iter()
                .filter(|(keywords, _)| mentioned(keywords))
                .map(|(_, kind)| kind)
                .collect();
                if constrained.is_empty() {
                    return Part::exact(Shape::Any);
                }
                unconstrained_rest = Some(other_kinds(constrained.clone()));
                constrained
            }
        };
        let mut kinds = dedupe(kinds);
        if kinds.contains(&Kind::Number) {
            kinds.retain(|kind| *kind != Kind::Integer);
        }

        let format = map.get("format").and_then(Value::as_str);
        let mut exact = true;
        let mut options = Vec::new();
        let numeric = kinds.iter().any(|kind| matches!(kind, Kind::Number | Kind::Integer));
        if format == Some(DARKMATTER_NUMBERLIKE_FORMAT) && numeric && kinds.contains(&Kind::String) {
            let (bounds, bounds_exact) = numeric_bounds(map, kinds.contains(&Kind::Integer));
            exact &= bounds_exact && !mentioned(&["minLength", "maxLength", "pattern"]);
            kinds.retain(|kind| !matches!(kind, Kind::Number | Kind::Integer | Kind::String));
            options.push(Shape::Number(NumberShape { as_sent: true, bounds }));
        }
        if format == Some(DARKMATTER_BOOLISH_FORMAT) && kinds.contains(&Kind::Boolean) && kinds.contains(&Kind::String) {
            exact &= !mentioned(&["minLength", "maxLength", "pattern"]);
            kinds.retain(|kind| !matches!(kind, Kind::Boolean | Kind::String));
            options.push(Shape::Boolean { as_sent: true });
        }
        for kind in kinds {
            let part = match kind {
                Kind::Null => Part::exact(Shape::Null),
                Kind::Boolean => Part::exact(Shape::boolean()),
                Kind::Number | Kind::Integer => {
                    let (bounds, bounds_exact) = numeric_bounds(map, kind == Kind::Integer);
                    Part {
                        shape: Shape::Number(NumberShape { as_sent: false, bounds }),
                        exact: bounds_exact,
                    }
                }
                Kind::String => self.text(map),
                Kind::Array => self.array(map, references),
                Kind::Object => self.object(map, references),
            };
            exact &= part.exact;
            options.push(part.shape);
        }
        options.extend(unconstrained_rest);
        Part {
            shape: union(options),
            exact,
        }
    }

    fn enumeration(&self, map: &Map<String, Value>, members: &[Value], declared: Option<&[Kind]>) -> Part {
        let admitted: Vec<&Value> = members
            .iter()
            .filter(|member| declared.is_none_or(|kinds| admits(kinds, member)))
            .collect();
        let texts: Option<Vec<String>> = admitted.iter().map(|member| member.as_str().map(str::to_string)).collect();
        let allowed = |extra: &[&str]| {
            map.keys().all(|keyword| {
                keyword == "enum" || keyword == "type" || extra.contains(&keyword.as_str()) || is_annotation(keyword)
            })
        };
        match texts {
            Some(texts) if !texts.is_empty() => {
                let Part { shape, exact } = self.text(map);
                let shape = match shape {
                    Shape::Text(text) => Shape::Text(TextShape {
                        members: Some(texts),
                        ..text
                    }),
                    other => other,
                };
                let exact = exact
                    && matches!(shape, Shape::Text(_))
                    && allowed(&["minLength", "maxLength", "pattern"])
                    && admitted.len() == members.len();
                Part { shape, exact }
            }
            _ => Part {
                shape: union(admitted.into_iter().map(|member| Shape::Literal(member.clone())).collect()),
                exact: allowed(&[]),
            },
        }
    }

    fn text(&self, map: &Map<String, Value>) -> Part {
        let format = map.get("format").and_then(Value::as_str);
        let length_or_pattern = ["minLength", "maxLength", "pattern"].iter().any(|keyword| map.contains_key(*keyword));
        match format {
            Some(DARKMATTER_JSON_FORMAT) => {
                return Part {
                    shape: Shape::Content(ContentFormat::Json),
                    exact: !length_or_pattern,
                };
            }
            Some(DARKMATTER_YAML_FORMAT) => {
                return Part {
                    shape: Shape::Content(ContentFormat::Yaml),
                    exact: !length_or_pattern,
                };
            }
            _ => {}
        }
        let mut exact = true;
        let mut length = |keyword: &str| match map.get(keyword) {
            None => None,
            Some(value) => {
                let count = value.as_u64().and_then(|count| usize::try_from(count).ok());
                exact &= count.is_some();
                count
            }
        };
        let min_length = length("minLength");
        let max_length = length("maxLength");
        let pattern = match map.get("pattern") {
            None => None,
            Some(Value::String(source)) => match TextPattern::new(source.as_str()) {
                Ok(pattern) => Some(pattern),
                Err(_) => {
                    exact = false;
                    None
                }
            },
            Some(_) => {
                exact = false;
                None
            }
        };
        let alphanumeric =
            map.get(DARKMATTER_TYPE_ANNOTATION).and_then(Value::as_str) == Some(SimplifiedType::Alphanumeric.as_keyword());
        let shape = Shape::Text(TextShape {
            min_length,
            max_length,
            pattern,
            members: None,
            scalar_text: !alphanumeric,
        });
        let shape = match format {
            Some(name) => shape.assert(format_assertion(name)),
            None => shape,
        };
        Part { shape, exact }
    }

    fn array(&self, map: &'s Map<String, Value>, references: &mut Vec<*const Value>) -> Part {
        let mut exact = true;
        let mut count = |keyword: &str| match map.get(keyword) {
            None => None,
            Some(value) => {
                let count = value.as_u64().and_then(|count| usize::try_from(count).ok());
                exact &= count.is_some();
                count
            }
        };
        let min_items = count("minItems");
        let max_items = count("maxItems");
        let unique_items = match map.get("uniqueItems") {
            None => false,
            Some(Value::Bool(unique)) => *unique,
            Some(_) => {
                exact = false;
                false
            }
        };
        if let Some(prefix) = map.get("prefixItems").and_then(Value::as_array) {
            let required = min_items.unwrap_or(0);
            exact &= max_items.is_none() && !unique_items && required <= prefix.len();
            let prefix = prefix
                .iter()
                .enumerate()
                .map(|(position, schema)| TupleSlot {
                    shape: self.node(schema, references),
                    optional: position >= required,
                })
                .collect();
            let rest = match map.get("items") {
                None => Some(Box::new(Shape::Any)),
                Some(Value::Bool(false)) => None,
                Some(schema) => Some(Box::new(self.node(schema, references))),
            };
            return Part {
                shape: Shape::Tuple(TupleShape { prefix, rest }),
                exact,
            };
        }
        let items = match map.get("items") {
            None => Shape::Any,
            // The pre-2020-12 tuple form.
            Some(Value::Array(_)) => {
                exact = false;
                Shape::Any
            }
            Some(schema) => self.node(schema, references),
        };
        Part {
            shape: Shape::Array(ArrayShape {
                items: Box::new(items),
                min_items,
                max_items,
                unique_items,
            }),
            exact,
        }
    }

    fn object(&self, map: &'s Map<String, Value>, references: &mut Vec<*const Value>) -> Part {
        let mut exact = true;
        let required: HashSet<&str> = match map.get("required") {
            None => HashSet::new(),
            Some(Value::Array(names)) => {
                // A non-string entry is not a name; the assertion judges it.
                exact &= names.iter().all(Value::is_string);
                names.iter().filter_map(Value::as_str).collect()
            }
            Some(_) => {
                exact = false;
                HashSet::new()
            }
        };
        let declared = map.get("properties").and_then(Value::as_object);
        // `required` does not declare a property, so a required name with no
        // `properties` entry cannot become a slot without changing which keys
        // `additionalProperties` covers.
        if required.iter().any(|name| declared.is_none_or(|properties| !properties.contains_key(*name))) {
            exact = false;
        }
        let properties = declared
            .into_iter()
            .flatten()
            .map(|(name, schema)| PropertySlot {
                name: name.clone(),
                shape: self.node(schema, references),
                required: required.contains(name.as_str()),
            })
            .collect();
        let mut patterns = Vec::new();
        if let Some(entries) = map.get("patternProperties").and_then(Value::as_object) {
            for (source, schema) in entries {
                match TextPattern::new(source.as_str()) {
                    Ok(pattern) => patterns.push(PatternSlot {
                        matcher: KeyMatcher::Pattern(pattern),
                        shape: self.node(schema, references),
                    }),
                    Err(_) => exact = false,
                }
            }
        }
        let additional = match map.get("additionalProperties") {
            None | Some(Value::Bool(true)) => Additional::Any,
            Some(Value::Bool(false)) => Additional::Forbidden,
            Some(schema) => match self.node(schema, references) {
                Shape::Any => Additional::Any,
                shape => Additional::Shape(Box::new(shape)),
            },
        };
        let mut count = |keyword: &str| match map.get(keyword) {
            None => None,
            Some(value) => {
                let count = value.as_u64().and_then(|count| usize::try_from(count).ok());
                exact &= count.is_some();
                count
            }
        };
        let min_properties = count("minProperties");
        let max_properties = count("maxProperties");
        Part {
            shape: Shape::Object(ObjectShape {
                properties,
                patterns,
                pattern_scope: PatternScope::AllKeys,
                additional,
                min_properties,
                max_properties,
            }),
            exact,
        }
    }

    /// The target of a `$ref`, or a validation-only shape for an unresolved,
    /// cyclic, or over-deep reference.
    fn reference(&self, from: &'s Value, reference: &str, references: &mut Vec<*const Value>) -> Shape {
        let Some(target) = self.index.resolve(from, reference) else {
            return Shape::Any.assert(self.node_assertion(from));
        };
        let key = target as *const Value;
        let repeats = references.iter().filter(|seen| **seen == key).count();
        if repeats >= CYCLE_EXPANSIONS || references.len() >= MAX_REFERENCE_DEPTH || self.budget.get() == 0 {
            return Shape::Any.assert(self.node_assertion(target));
        }
        self.budget.set(self.budget.get() - 1);
        references.push(key);
        let shape = self.node(target, references);
        references.pop();
        shape
    }

    /// A passive check of `node` by `jsonschema`, compiled on first use.
    fn node_assertion(&self, node: &'s Value) -> Assertion {
        // A reference can land on a node the index never walked (a pointer
        // into `enum` data, say); such a node is judged by itself.
        let pointer = self.index.pointers.get(&(node as *const Value)).cloned();
        let description = match pointer.as_deref() {
            Some("") => "the schema".to_string(),
            Some(pointer) => format!("the schema at #{pointer}"),
            None => "the referenced schema".to_string(),
        };
        let source = match pointer {
            Some(pointer) if holds_reference(node) => AssertionSource::InDocument {
                document: self.document.clone(),
                root_base: self.root_base(),
                pointer,
            },
            _ => AssertionSource::Standalone(node.clone()),
        };
        let validator: Arc<OnceLock<Option<Validator>>> = Arc::new(OnceLock::new());
        Assertion::new(description, move |value| {
            match validator.get_or_init(|| source.build()) {
                Some(validator) => validator.is_valid(value),
                // A subschema that does not compile alone is left to the
                // complete schema, which every caller validates the final
                // value against.
                None => true,
            }
        })
    }

    /// The document root's base URI: its resolved `$id`, else the default.
    fn root_base(&self) -> String {
        let root = self.index.resources[DOCUMENT_BASE];
        self.index
            .bases
            .get(&(root as *const Value))
            .cloned()
            .unwrap_or_else(|| DOCUMENT_BASE.to_string())
    }
}

/// What an assertion validator is compiled from.
enum AssertionSource {
    /// A subschema with no reference compiles by itself.
    Standalone(Value),
    /// A subschema with a reference compiles through a wrapper that embeds the
    /// whole document under its resolved identity, so every reference
    /// resolves exactly as it does in the document's own validator.
    InDocument {
        document: Arc<Document>,
        root_base: String,
        pointer: String,
    },
}

impl AssertionSource {
    fn build(&self) -> Option<Validator> {
        let schema = match self {
            Self::Standalone(node) => {
                let mut node = node.clone();
                if let Value::Object(map) = &mut node {
                    map.entry("$schema").or_insert_with(|| Value::String(DRAFT_2020_12.into()));
                }
                node
            }
            Self::InDocument {
                document,
                root_base,
                pointer,
            } => {
                let (embedded, base) = document.embedded(root_base);
                let target = if pointer.is_empty() {
                    base.clone()
                } else {
                    format!("{}#{}", strip_fragment(base), percent_encode_fragment(pointer))
                };
                serde_json::json!({
                    "$schema": DRAFT_2020_12,
                    "$id": "urn:darkmatter:assertion",
                    "$ref": target,
                    "$defs": { "document": embedded },
                })
            }
        };
        build_structural_validator(&schema).ok()
    }
}

/// A passive check that a string satisfies `format`, through the validator's
/// registered format so the two agree.
fn format_assertion(format: &str) -> Assertion {
    static VALIDATORS: LazyLock<Mutex<HashMap<String, Arc<Option<Validator>>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let validator = {
        let mut cache = VALIDATORS.lock().expect("format validator cache poisoned");
        cache
            .entry(format.to_string())
            .or_insert_with(|| {
                Arc::new(build_structural_validator(&serde_json::json!({ "format": format })).ok())
            })
            .clone()
    };
    Assertion::new(format!("format({format})"), move |value| {
        validator.as_ref().as_ref().is_none_or(|validator| validator.is_valid(value))
    })
}

/// Any value whose JSON type is none of `constrained`, kept unchanged.
fn other_kinds(constrained: Vec<Kind>) -> Shape {
    let names: Vec<&str> = constrained
        .iter()
        .map(|kind| match kind {
            Kind::Null => "null",
            Kind::Boolean => "boolean",
            Kind::Number | Kind::Integer => "number",
            Kind::String => "string",
            Kind::Array => "array",
            Kind::Object => "object",
        })
        .collect();
    let description = format!("not {}", names.join(" or "));
    Shape::Any.assert(Assertion::new(description, move |value| {
        let kind = Kind::of(value);
        !constrained.contains(&kind)
    }))
}

fn numeric_bounds(map: &Map<String, Value>, integer: bool) -> (NumericBounds, bool) {
    let mut exact = true;
    let mut bound = |keyword: &str| match map.get(keyword) {
        None => None,
        Some(value) => {
            let number = value.as_f64();
            exact &= number.is_some();
            number
        }
    };
    let bounds = NumericBounds {
        minimum: bound("minimum"),
        maximum: bound("maximum"),
        exclusive_minimum: bound("exclusiveMinimum"),
        exclusive_maximum: bound("exclusiveMaximum"),
        multiple_of: bound("multipleOf"),
        integer: integer.then_some(if map.get(DARKMATTER_INTEGER_KEYWORD) == Some(&Value::Bool(true)) {
            IntegerRule::WholeBelow2Pow64
        } else {
            IntegerRule::Whole
        }),
    };
    (bounds, exact)
}

fn union(mut options: Vec<Shape>) -> Shape {
    if options.len() == 1 {
        options.remove(0)
    } else {
        Shape::Union(options)
    }
}

fn dedupe(kinds: Vec<Kind>) -> Vec<Kind> {
    let mut seen = HashSet::new();
    kinds.into_iter().filter(|kind| seen.insert(*kind)).collect()
}

fn admits(kinds: &[Kind], value: &Value) -> bool {
    let kind = Kind::of(value);
    kinds.contains(&kind)
        || (kind == Kind::Number && kinds.contains(&Kind::Integer) && value.as_f64().is_some_and(|n| n.fract() == 0.0))
}

/// Keywords that never constrain a value: core identifiers, metadata, content
/// annotations, and every `x-` extension except Darkmatter's validating ones
/// (listed in [`UNMODELED_KEYWORDS`]). Unknown keywords are annotations in
/// Draft 2020-12.
fn is_annotation(keyword: &str) -> bool {
    !UNMODELED_KEYWORDS.contains(&keyword)
        && !matches!(
            keyword,
            "type"
                | "const"
                | "enum"
                | "$ref"
                | "anyOf"
                | "oneOf"
                | "format"
                | "minLength"
                | "maxLength"
                | "pattern"
                | "minimum"
                | "maximum"
                | "exclusiveMinimum"
                | "exclusiveMaximum"
                | "multipleOf"
                | "items"
                | "prefixItems"
                | "minItems"
                | "maxItems"
                | "uniqueItems"
                | "properties"
                | "required"
                | "patternProperties"
                | "additionalProperties"
                | "minProperties"
                | "maxProperties"
        )
}

/// Whether a reference occurs anywhere in `node`'s subschemas.
fn holds_reference(node: &Value) -> bool {
    match node {
        Value::Object(map) => map.iter().any(|(keyword, child)| {
            matches!(keyword.as_str(), "$ref" | "$dynamicRef" | "$recursiveRef")
                || (!matches!(keyword.as_str(), "const" | "enum" | "default" | "examples") && holds_reference(child))
        }),
        Value::Array(items) => items.iter().any(holds_reference),
        _ => false,
    }
}

fn resolve_reference(base: &str, reference: &str) -> Option<String> {
    let base = Uri::parse(base).ok()?;
    jsonschema::uri::resolve_against(&base, reference)
        .ok()
        .map(|resolved| resolved.as_str().to_string())
}

fn strip_fragment(uri: &str) -> &str {
    uri.split_once('#').map_or(uri, |(resource, _)| resource)
}

fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// Percent-encodes a JSON Pointer for use as a URI fragment.
fn percent_encode_fragment(pointer: &str) -> String {
    let mut out = String::with_capacity(pointer.len());
    for byte in pointer.bytes() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.' | b'_' | b'~' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';'
                    | b'=' | b':' | b'@' | b'/' | b'?'
            );
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn percent_decode(fragment: &str) -> Option<String> {
    let bytes = fragment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position] == b'%' {
            let hex = fragment.get(position + 1..position + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            position += 3;
        } else {
            out.push(bytes[position]);
            position += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::markdown::coercion::bind;

    fn converts(schema: Value, value: Value) -> Result<Value, String> {
        let shape = shape_from_json_schema(&schema);
        bind(&shape, &value).map(|bound| bound.into_value()).map_err(|error| error.to_string())
    }

    #[test]
    fn type_array_and_typeless_keywords_follow_their_kinds() {
        assert_eq!(converts(json!({"type": ["number", "null"]}), json!("3")), Ok(json!(3)));
        assert_eq!(converts(json!({"type": ["number", "string"]}), json!("3")), Ok(json!("3")));
        // A typeless numeric keyword leaves text alone.
        assert_eq!(converts(json!({"minimum": 5}), json!("3")), Ok(json!("3")));
    }

    #[test]
    fn anchors_and_ids_resolve_within_their_resource() {
        let schema = json!({
            "$id": "https://example.com/root.json",
            "$defs": {
                "inner": {
                    "$id": "inner.json",
                    "$defs": { "count": { "$anchor": "count", "type": "number" } },
                    "type": "object",
                    "properties": { "n": { "$ref": "#count" } }
                }
            },
            "type": "object",
            "properties": { "child": { "$ref": "inner.json" } }
        });
        assert_eq!(converts(schema, json!({"child": {"n": "4"}})), Ok(json!({"child": {"n": 4}})));
    }

    #[test]
    fn a_pointer_fragment_is_relative_to_the_resolved_resource() {
        let schema = json!({
            "$defs": {
                "count": { "type": "boolean" },
                "inner": {
                    "$id": "https://example.com/inner.json",
                    "$defs": { "count": { "type": "number" } },
                    "type": "object",
                    "properties": { "n": { "$ref": "#/$defs/count" } }
                }
            },
            "type": "object",
            "properties": { "child": { "$ref": "#/$defs/inner" } }
        });
        assert_eq!(converts(schema, json!({"child": {"n": "1"}})), Ok(json!({"child": {"n": 1}})));
    }

    #[test]
    fn unresolved_reference_is_validation_only() {
        let schema = json!({"type": "object", "properties": {"n": {"$ref": "https://example.com/remote.json"}}});
        assert_eq!(converts(schema, json!({"n": "3"})), Ok(json!({"n": "3"})));
    }

    #[test]
    fn pointer_fragments_round_trip_through_percent_encoding() {
        let pointer = "/patternProperties/^(?!(?:a b)$).*";
        assert_eq!(percent_decode(&percent_encode_fragment(pointer)).as_deref(), Some(pointer));
    }
}
