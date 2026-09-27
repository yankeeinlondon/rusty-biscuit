//! THROWAWAY SPIKE: translator 2 — grammar descriptors → Shape.
//!
//! Reads the SimplifiedSchema AST directly (the function-call surface will
//! start from the same grammar). It must agree with translator 1 on every
//! grammar-authored case; the diff test asserts that.

use std::sync::Arc;

use serde_json::Value;

use super::core::{
    Additional, NumberShape, ObjectShape, Pattern, Residual, Shape, TextKind, TextShape,
};
use crate::markdown::schemas::format::{
    DARKMATTER_DATETIME_FORMAT, DARKMATTER_EXPRESSION_FORMAT, DARKMATTER_FILE_FORMAT,
    DARKMATTER_FILE_REFERENCE_FORMAT, DARKMATTER_JSON_FORMAT, DARKMATTER_SCHEMA_KEYWORD,
    DARKMATTER_TIME_FORMAT, DARKMATTER_TYPE_DEFINITION_KEYWORD, DARKMATTER_YAML_FORMAT,
};
use crate::markdown::schemas::simplified::{
    Constraint, PatternKey, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema,
    SimplifiedType, TypeExpr,
};
use crate::markdown::schemas::validate::build_validator;

pub fn translate_schema(schema: &SimplifiedSchema) -> Option<Shape> {
    match schema {
        SimplifiedSchema::Single(shape) => Some(object(shape, Additional::Allow)),
        SimplifiedSchema::Union(arms) => {
            let mut out = Vec::new();
            for a in arms {
                match a {
                    SchemaArm::Inline(s) => out.push(object(s, Additional::Allow)),
                    SchemaArm::FileRef(_) => return None,
                }
            }
            Some(Shape::Union(out))
        }
    }
}

fn object(shape: &SchemaShape, default_additional: Additional) -> Shape {
    let mut props = Vec::new();
    let mut required = Vec::new();
    for (name, def) in &shape.properties {
        if def.is_required() && !is_generated(def) {
            required.push(name.clone());
        }
        props.push((name.clone(), property(def)));
    }
    let mut patterns = Vec::new();
    let mut catch_all = None;
    for pk in &shape.pattern_keys {
        let value_shape = bare_def(&pk.def);
        match &pk.key {
            PatternKey::CatchAll => catch_all = Some(value_shape),
            PatternKey::Starting(p) => patterns.push((Pattern::new(&format!("^{}", regex::escape(p))).expect("re"), value_shape)),
            PatternKey::Ending(p) => patterns.push((Pattern::new(&format!("{}$", regex::escape(p))).expect("re"), value_shape)),
            PatternKey::Pattern(re) => {
                if let Some(p) = Pattern::new(re) {
                    patterns.push((p, value_shape));
                }
            }
        }
    }
    let additional = match catch_all {
        Some(s) => Additional::Shape(Box::new(s)),
        None if shape.pattern_keys.is_empty() => default_additional,
        None => Additional::Deny,
    };
    let (mut min_props, mut max_props) = (None, None);
    for c in &shape.constraints {
        match c {
            Constraint::MinKeys(n) => min_props = Some(*n),
            Constraint::MaxKeys(n) => max_props = Some(*n),
            _ => {}
        }
    }
    Shape::Object(ObjectShape {
        props,
        required,
        patterns,
        additional,
        min_props,
        max_props,
    })
}

fn is_generated(def: &PropertyDef) -> bool {
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(a) => vec![a],
        PropertyDef::Union(v) => v.iter().collect(),
    };
    atoms
        .iter()
        .any(|a| a.property_constraints().any(|c| matches!(c, Constraint::Generated)))
}

/// A property: optional ⇒ `T | null` (optional scalar `file` also admits "").
fn property(def: &PropertyDef) -> Shape {
    let required = def.is_required();
    let mut options = match def {
        PropertyDef::Single(a) => vec![atom(a, required)],
        PropertyDef::Union(atoms) => atoms.iter().map(|a| atom(a, required)).collect(),
    };
    if !required {
        options.insert(0, Shape::Null);
        // flatten the optional-file (""-sentinel) union
        let mut flat = Vec::new();
        for o in options {
            match o {
                Shape::Union(inner) if matches!(def, PropertyDef::Single(_)) => flat.extend(inner),
                other => flat.push(other),
            }
        }
        return Shape::Union(flat);
    }
    if options.len() == 1 {
        options.pop().expect("one")
    } else {
        Shape::Union(options)
    }
}

/// Pattern-key values: no nullable wrapper.
fn bare_def(def: &PropertyDef) -> Shape {
    match def {
        PropertyDef::Single(a) => atom(a, true),
        PropertyDef::Union(v) => Shape::Union(v.iter().map(|a| atom(a, true)).collect()),
    }
}

fn atom(a: &PropertyAtom, union_required: bool) -> Shape {
    let inner = match &a.ty {
        TypeExpr::Primitive(t) => primitive(*t, &a.constraints),
        TypeExpr::InlineObject(shape) => {
            let mut s = object(shape, Additional::Deny);
            if let Shape::Object(os) = &mut s {
                for c in &a.constraints {
                    match c {
                        Constraint::MinKeys(n) => os.min_props = Some(*n),
                        Constraint::MaxKeys(n) => os.max_props = Some(*n),
                        _ => {}
                    }
                }
            }
            s
        }
        TypeExpr::Imported { .. } => Shape::Any,
    };
    if a.is_array {
        let mut min_items = None;
        let mut max_items = None;
        let mut unique = false;
        for c in &a.array_constraints {
            match c {
                Constraint::MinItems(n) => min_items = Some(*n),
                Constraint::MaxItems(n) => max_items = Some(*n),
                Constraint::Unique => unique = true,
                _ => {}
            }
        }
        return Shape::List {
            items: Box::new(inner),
            min_items,
            max_items,
            unique,
        };
    }
    let optional_file = matches!(a.ty, TypeExpr::Primitive(SimplifiedType::File)) && !union_required;
    if optional_file {
        return Shape::Union(vec![
            Shape::Text(TextShape {
                one_of: Some(vec![String::new()]),
                ..TextShape::plain()
            }),
            inner,
        ]);
    }
    inner
}

fn text_with_format(f: &str) -> Shape {
    Shape::Text(TextShape {
        format: Some(f.to_string()),
        ..TextShape::plain()
    })
}

fn primitive(t: SimplifiedType, cs: &[Constraint]) -> Shape {
    match t {
        SimplifiedType::String => {
            let mut ts = TextShape::plain();
            for c in cs {
                match c {
                    Constraint::MinLen(n) => ts.min_len = Some(*n),
                    Constraint::MaxLen(n) => ts.max_len = Some(*n),
                    Constraint::NotEmpty => ts.pattern = Pattern::new(r"\S"),
                    Constraint::Pattern(p) => ts.pattern = Pattern::new(p),
                    _ => {}
                }
            }
            Shape::Text(ts)
        }
        SimplifiedType::Date => text_with_format("date"),
        SimplifiedType::DateTime => text_with_format(DARKMATTER_DATETIME_FORMAT),
        SimplifiedType::Time => text_with_format(DARKMATTER_TIME_FORMAT),
        SimplifiedType::Number => {
            let mut ns = NumberShape::default();
            for c in cs {
                match c {
                    Constraint::Min(n) => ns.min = Some(*n),
                    Constraint::Max(n) => ns.max = Some(*n),
                    Constraint::Integer => ns.integer = true,
                    _ => {}
                }
            }
            Shape::Number(ns)
        }
        SimplifiedType::NumberLike => Shape::NumberLike,
        SimplifiedType::Boolean => Shape::Boolean { literal: None },
        SimplifiedType::Boolish => Shape::Boolish,
        SimplifiedType::Object => Shape::Object(ObjectShape::opaque()),
        SimplifiedType::File => {
            let eager = cs.iter().any(|c| matches!(c, Constraint::Eager));
            text_with_format(if eager { DARKMATTER_FILE_FORMAT } else { DARKMATTER_FILE_REFERENCE_FORMAT })
        }
        SimplifiedType::Enum => Shape::Text(TextShape {
            one_of: cs.iter().find_map(|c| match c {
                Constraint::Members(m) => Some(m.clone()),
                _ => None,
            }),
            ..TextShape::plain()
        }),
        SimplifiedType::Url => Shape::Text(TextShape {
            format: Some("uri".into()),
            schemes: cs.iter().find_map(|c| match c {
                Constraint::Scheme(s) => Some(s.clone()),
                _ => None,
            }),
            ..TextShape::plain()
        }),
        SimplifiedType::Email => text_with_format("email"),
        SimplifiedType::Yaml => Shape::Text(TextShape {
            kind: TextKind::Yaml,
            format: Some(DARKMATTER_YAML_FORMAT.into()),
            ..TextShape::plain()
        }),
        SimplifiedType::Json => Shape::Text(TextShape {
            kind: TextKind::Json,
            format: Some(DARKMATTER_JSON_FORMAT.into()),
            ..TextShape::plain()
        }),
        SimplifiedType::Literal => {
            let v = cs.iter().find_map(|c| match c {
                Constraint::LiteralValue(v) => Some(v.clone()),
                _ => None,
            });
            match v {
                Some(Value::Bool(b)) => Shape::Boolean { literal: Some(b) },
                Some(Value::String(s)) => Shape::Text(TextShape {
                    one_of: Some(vec![s]),
                    ..TextShape::plain()
                }),
                Some(n @ Value::Number(_)) => Shape::Number(NumberShape {
                    one_of: Some(vec![n]),
                    ..NumberShape::default()
                }),
                _ => Shape::Any,
            }
        }
        SimplifiedType::Expression => text_with_format(DARKMATTER_EXPRESSION_FORMAT),
        SimplifiedType::TypeDefinition => semantic(DARKMATTER_TYPE_DEFINITION_KEYWORD),
        SimplifiedType::Schema => semantic(DARKMATTER_SCHEMA_KEYWORD),
        SimplifiedType::Any => Shape::Any,
    }
}

fn semantic(keyword: &str) -> Shape {
    let frag = serde_json::json!({ "type": ["string", "object", "array"], keyword: true });
    let validator = build_validator(&frag, None, None).ok().map(Arc::new);
    Shape::Opaque(Residual {
        label: "x-darkmatter-semantic".into(),
        check: Arc::new(move |v| validator.as_ref().is_some_and(|x| x.is_valid(v))),
    })
}
