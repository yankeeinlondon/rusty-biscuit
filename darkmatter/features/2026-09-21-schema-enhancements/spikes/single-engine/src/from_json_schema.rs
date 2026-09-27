//! THROWAWAY SPIKE: translator 1 — compiled (or raw) JSON Schema → Shape.
//!
//! Translates the keywords Darkmatter emits plus the common raw keywords.
//! Anything else becomes a validate-only residual (checked with `jsonschema`
//! after conversion, never converting). `notes` records every keyword that
//! fell back that way so the diff can report it.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Map, Value};

use super::core::{
    Additional, NumberShape, ObjectShape, Pattern, Residual, Shape, TextKind, TextShape,
};
use crate::markdown::schemas::format::{
    DARKMATTER_JSON_FORMAT, DARKMATTER_SCHEMA_KEYWORD, DARKMATTER_TYPE_DEFINITION_KEYWORD,
    DARKMATTER_URL_SCHEME_KEYWORD, DARKMATTER_YAML_FORMAT,
};
use crate::markdown::schemas::simplified::convert::{BOOLISH_VALUES, NUMBERLIKE_PATTERN};
use crate::markdown::schemas::validate::build_validator;

const ANNOTATIONS: &[&str] = &[
    "$schema",
    "$id",
    "$comment",
    "$anchor",
    "$defs",
    "definitions",
    "title",
    "description",
    "default",
    "examples",
    "deprecated",
    "readOnly",
    "writeOnly",
];

pub struct JsonTranslator<'a> {
    root: &'a Value,
    base_dir: Option<PathBuf>,
    pub notes: RefCell<Vec<String>>,
}

impl<'a> JsonTranslator<'a> {
    pub fn new(root: &'a Value, base_dir: Option<PathBuf>) -> Self {
        Self {
            root,
            base_dir,
            notes: RefCell::new(Vec::new()),
        }
    }

    pub fn translate_root(&self) -> Shape {
        self.translate(self.root, 0)
    }

    fn residual(&self, label: &str, fragment: Value) -> Residual {
        let mut frag = fragment;
        if let Value::Object(m) = &mut frag {
            m.insert(
                "$schema".into(),
                Value::String("https://json-schema.org/draft/2020-12/schema".into()),
            );
            // Local refs resolve against the root's definitions.
            for k in ["$defs", "definitions"] {
                if let Some(d) = self.root.get(k) {
                    m.entry(k).or_insert(d.clone());
                }
            }
        }
        let validator = build_validator(&frag, self.base_dir.as_deref(), None).ok().map(Arc::new);
        self.notes.borrow_mut().push(label.to_string());
        Residual {
            label: label.to_string(),
            check: Arc::new(move |v: &Value| validator.as_ref().is_some_and(|val| val.is_valid(v))),
        }
    }

    pub fn translate(&self, schema: &Value, depth: usize) -> Shape {
        if depth > 32 {
            return Shape::Opaque(self.residual("depth-limit", schema.clone()));
        }
        let obj = match schema {
            Value::Bool(true) => return Shape::Any,
            Value::Bool(false) => return Shape::Opaque(self.residual("false-schema", schema.clone())),
            Value::Object(o) => o,
            _ => return Shape::Any,
        };

        // Darkmatter-emitted numberlike / boolish fragments (exact shapes).
        if let Some(s) = recognize_numberlike_boolish(obj) {
            return s;
        }

        // Semantic keywords: validate-only, as today.
        if obj.contains_key(DARKMATTER_TYPE_DEFINITION_KEYWORD) || obj.contains_key(DARKMATTER_SCHEMA_KEYWORD) {
            return Shape::Opaque(self.residual("x-darkmatter-semantic", schema.clone()));
        }

        // Local $ref: translate the target inline; siblings (2020-12) apply too.
        if let Some(r) = obj.get("$ref").and_then(Value::as_str) {
            let target = r.strip_prefix('#').and_then(|p| self.root.pointer(p));
            return match target {
                Some(t) => {
                    let mut rest = obj.clone();
                    rest.remove("$ref");
                    let inner = self.translate(t, depth + 1);
                    if rest.keys().all(|k| ANNOTATIONS.contains(&k.as_str())) {
                        inner
                    } else {
                        Shape::WithResidual(Box::new(inner), self.residual("$ref-siblings", Value::Object(rest)))
                    }
                }
                None => Shape::Opaque(self.residual("$ref-remote", schema.clone())),
            };
        }

        // Keywords this translator never models → validate-only residual.
        let unsupported: Vec<&String> = obj
            .keys()
            .filter(|k| {
                matches!(
                    k.as_str(),
                    "allOf"
                        | "not"
                        | "if"
                        | "then"
                        | "else"
                        | "dependentSchemas"
                        | "dependentRequired"
                        | "propertyNames"
                        | "contains"
                        | "minContains"
                        | "maxContains"
                        | "unevaluatedItems"
                        | "unevaluatedProperties"
                        | "multipleOf"
                        | "contentEncoding"
                        | "contentMediaType"
                        | "contentSchema"
                        | "$dynamicRef"
                )
            })
            .collect();
        let residual = if unsupported.is_empty() {
            None
        } else {
            let mut frag = Map::new();
            for k in &unsupported {
                frag.insert((*k).clone(), obj[k.as_str()].clone());
            }
            let label = unsupported.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("+");
            Some(self.residual(&label, Value::Object(frag)))
        };
        let wrap = |s: Shape| match &residual {
            Some(r) => Shape::WithResidual(Box::new(s), r.clone()),
            None => s,
        };

        if let Some(arms) = obj.get("anyOf").and_then(Value::as_array) {
            let union = Shape::Union(arms.iter().map(|a| self.translate(a, depth + 1)).collect());
            let base = self.merge_siblings_with_union(obj, union, depth);
            return wrap(base);
        }
        if let Some(arms) = obj.get("oneOf").and_then(Value::as_array) {
            // Same union rule; oneOf's exclusivity is checked after binding.
            let union = Shape::Union(arms.iter().map(|a| self.translate(a, depth + 1)).collect());
            let r = self.residual("oneOf-exclusivity", Value::Object(obj.clone()));
            return wrap(Shape::WithResidual(Box::new(union), r));
        }

        if let Some(c) = obj.get("const") {
            return wrap(const_shape(c, self));
        }
        if let Some(Value::Array(members)) = obj.get("enum") {
            return wrap(enum_shape(members, obj.get("type"), self));
        }

        let types: Vec<&str> = match obj.get("type") {
            Some(Value::String(t)) => vec![t.as_str()],
            Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        if types.is_empty() {
            // Typeless: only honor it if nothing type-specific is present.
            let typed_kw = [
                "properties", "items", "prefixItems", "pattern", "minimum", "maximum", "minLength",
                "maxLength", "format", "required", "additionalProperties", "patternProperties",
            ];
            if obj.keys().any(|k| typed_kw.contains(&k.as_str())) {
                return Shape::Opaque(self.residual("typeless-constraints", schema.clone()));
            }
            return wrap(Shape::Any);
        }
        let shapes: Vec<Shape> = types.iter().map(|t| self.typed(t, obj, depth)).collect();
        let s = if shapes.len() == 1 {
            shapes.into_iter().next().expect("one")
        } else {
            Shape::Union(shapes)
        };
        wrap(s)
    }

    /// `anyOf` with typed siblings (rare): keep the union and validate the
    /// siblings after binding.
    fn merge_siblings_with_union(&self, obj: &Map<String, Value>, union: Shape, _depth: usize) -> Shape {
        let siblings: Map<String, Value> = obj
            .iter()
            .filter(|(k, _)| k.as_str() != "anyOf" && !ANNOTATIONS.contains(&k.as_str()) && !k.starts_with("x-"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if siblings.is_empty() {
            union
        } else {
            Shape::WithResidual(Box::new(union), self.residual("anyOf-siblings", Value::Object(siblings)))
        }
    }

    fn typed(&self, t: &str, obj: &Map<String, Value>, depth: usize) -> Shape {
        match t {
            "null" => Shape::Null,
            "boolean" => Shape::Boolean { literal: None },
            "number" | "integer" => Shape::Number(NumberShape {
                integer: t == "integer",
                min: obj.get("minimum").and_then(Value::as_f64),
                max: obj.get("maximum").and_then(Value::as_f64),
                exclusive_min: obj.get("exclusiveMinimum").and_then(Value::as_f64),
                exclusive_max: obj.get("exclusiveMaximum").and_then(Value::as_f64),
                one_of: None,
            }),
            "string" => {
                let format = obj.get("format").and_then(Value::as_str).map(str::to_string);
                let kind = match format.as_deref() {
                    Some(DARKMATTER_JSON_FORMAT) => TextKind::Json,
                    Some(DARKMATTER_YAML_FORMAT) => TextKind::Yaml,
                    _ => TextKind::Plain,
                };
                Shape::Text(TextShape {
                    kind,
                    format,
                    min_len: obj.get("minLength").and_then(Value::as_u64).map(|n| n as usize),
                    max_len: obj.get("maxLength").and_then(Value::as_u64).map(|n| n as usize),
                    pattern: obj.get("pattern").and_then(Value::as_str).and_then(Pattern::new),
                    one_of: None,
                    schemes: obj.get(DARKMATTER_URL_SCHEME_KEYWORD).and_then(Value::as_array).map(|a| {
                        a.iter().filter_map(Value::as_str).map(str::to_string).collect()
                    }),
                })
            }
            "array" => {
                let min_items = obj.get("minItems").and_then(Value::as_u64).map(|n| n as usize);
                if let Some(Value::Array(prefix)) = obj.get("prefixItems") {
                    let rest = match obj.get("items") {
                        Some(Value::Bool(false)) => None,
                        Some(s) => Some(Box::new(self.translate(s, depth + 1))),
                        None => Some(Box::new(Shape::Any)),
                    };
                    return Shape::Tuple {
                        prefix: prefix.iter().map(|p| self.translate(p, depth + 1)).collect(),
                        required: min_items.unwrap_or(0),
                        rest,
                    };
                }
                Shape::List {
                    items: Box::new(obj.get("items").map_or(Shape::Any, |i| self.translate(i, depth + 1))),
                    min_items,
                    max_items: obj.get("maxItems").and_then(Value::as_u64).map(|n| n as usize),
                    unique: obj.get("uniqueItems").and_then(Value::as_bool).unwrap_or(false),
                }
            }
            "object" => {
                let props = obj
                    .get("properties")
                    .and_then(Value::as_object)
                    .map(|p| {
                        p.iter()
                            .map(|(k, v)| (k.clone(), self.translate(v, depth + 1)))
                            .collect()
                    })
                    .unwrap_or_default();
                let patterns = obj
                    .get("patternProperties")
                    .and_then(Value::as_object)
                    .map(|p| {
                        p.iter()
                            .filter_map(|(k, v)| Some((Pattern::new(k)?, self.translate(v, depth + 1))))
                            .collect()
                    })
                    .unwrap_or_default();
                let additional = match obj.get("additionalProperties") {
                    None | Some(Value::Bool(true)) => Additional::Allow,
                    Some(Value::Bool(false)) => Additional::Deny,
                    Some(s) => Additional::Shape(Box::new(self.translate(s, depth + 1))),
                };
                Shape::Object(ObjectShape {
                    props,
                    required: obj
                        .get("required")
                        .and_then(Value::as_array)
                        .map(|r| r.iter().filter_map(Value::as_str).map(str::to_string).collect())
                        .unwrap_or_default(),
                    patterns,
                    additional,
                    min_props: obj.get("minProperties").and_then(Value::as_u64).map(|n| n as usize),
                    max_props: obj.get("maxProperties").and_then(Value::as_u64).map(|n| n as usize),
                })
            }
            _ => Shape::Any,
        }
    }
}

fn recognize_numberlike_boolish(obj: &Map<String, Value>) -> Option<Shape> {
    let arms = obj.get("anyOf")?.as_array()?;
    if arms.len() != 2 {
        return None;
    }
    let is_type = |a: &Value, t: &str| a.get("type").and_then(Value::as_str) == Some(t);
    let numberlike_arm = |a: &Value| {
        is_type(a, "string")
            && (a.get("pattern").and_then(Value::as_str) == Some(NUMBERLIKE_PATTERN)
                || a.get("format").and_then(Value::as_str) == Some(super::numberlike_format::NUMBERLIKE_FORMAT))
    };
    let boolish_arm = |a: &Value| {
        a.get("enum").and_then(Value::as_array).is_some_and(|m| {
            m.len() == BOOLISH_VALUES.len() && BOOLISH_VALUES.iter().all(|b| m.iter().any(|x| x.as_str() == Some(*b)))
        }) || (is_type(a, "string")
            && a.get("format").and_then(Value::as_str) == Some(super::numberlike_format::BOOLISH_FORMAT))
    };
    if arms.iter().any(|a| is_type(a, "number")) && arms.iter().any(numberlike_arm) {
        return Some(Shape::NumberLike);
    }
    if arms.iter().any(|a| is_type(a, "boolean")) && arms.iter().any(boolish_arm) {
        return Some(Shape::Boolish);
    }
    None
}

fn const_shape(c: &Value, t: &JsonTranslator<'_>) -> Shape {
    match c {
        Value::Null => Shape::Null,
        Value::Bool(b) => Shape::Boolean { literal: Some(*b) },
        Value::Number(_) => Shape::Number(NumberShape {
            one_of: Some(vec![c.clone()]),
            ..NumberShape::default()
        }),
        Value::String(s) => Shape::Text(TextShape {
            one_of: Some(vec![s.clone()]),
            ..TextShape::plain()
        }),
        _ => Shape::Opaque(t.residual("const-structured", serde_json::json!({ "const": c }))),
    }
}

fn enum_shape(members: &[Value], ty: Option<&Value>, t: &JsonTranslator<'_>) -> Shape {
    let strings: Vec<String> = members.iter().filter_map(|m| m.as_str().map(str::to_string)).collect();
    let mut options = Vec::new();
    if !strings.is_empty() {
        options.push(Shape::Text(TextShape {
            one_of: Some(strings),
            ..TextShape::plain()
        }));
    }
    let numbers: Vec<Value> = members.iter().filter(|m| m.is_number()).cloned().collect();
    if !numbers.is_empty() {
        options.push(Shape::Number(NumberShape {
            one_of: Some(numbers),
            ..NumberShape::default()
        }));
    }
    for m in members {
        match m {
            Value::Bool(b) => options.push(Shape::Boolean { literal: Some(*b) }),
            Value::Null => options.push(Shape::Null),
            Value::Array(_) | Value::Object(_) => {
                options.push(Shape::Opaque(t.residual("enum-structured", serde_json::json!({ "const": m }))))
            }
            _ => {}
        }
    }
    let _ = ty; // a `type` sibling only narrows; members already carry types
    if options.len() == 1 {
        options.pop().expect("one")
    } else {
        Shape::Union(options)
    }
}
