//! Serialise a [`PropertyAtom`] back into a type-and-constraint string.
//!
//! Used by tests (especially `proptest` round-trip tests) to verify that the
//! parser and the AST agree. The output is canonical (deterministic key
//! ordering, minimal whitespace) and re-parses to an equal [`PropertyAtom`].

use std::fmt::Write;

use super::types::{Constraint, PropertyAtom, SimplifiedType, TypeExpr};

/// Serialise `atom` into a string suitable for re-parsing via
/// [`grammar::parse_type_expr`].
///
/// [`grammar::parse_type_expr`]: super::grammar::parse_type_expr
pub fn serialize_property_atom(atom: &PropertyAtom) -> String {
    let mut out = String::new();
    match &atom.ty {
        TypeExpr::Primitive(ty) => {
            out.push_str(ty.as_keyword());
            if !atom.constraints.is_empty() {
                out.push('(');
                write_constraints(&mut out, &atom.constraints, *ty);
                out.push(')');
            } else if matches!(ty, SimplifiedType::Enum) {
                // The parser rejects bare `enum` without members; if a caller
                // somehow constructs one, emit an empty paren list so the output is
                // still self-describing (it will fail to re-parse, which is the
                // desired round-trip signal).
                out.push_str("()");
            }
            if atom.is_array {
                out.push_str("[]");
                if !atom.array_constraints.is_empty() {
                    out.push('(');
                    write_constraints(&mut out, &atom.array_constraints, *ty);
                    out.push(')');
                }
            }
        }
        TypeExpr::InlineObject(shape) => {
            write_inline_object(&mut out, shape, atom.is_array);
            if atom.is_array && !atom.array_constraints.is_empty() {
                out.push('(');
                write_constraints(&mut out, &atom.array_constraints, SimplifiedType::String);
                out.push(')');
            }
            if !atom.is_array && !atom.constraints.is_empty() {
                out.push('(');
                write_constraints(&mut out, &atom.constraints, SimplifiedType::String);
                out.push(')');
            }
        }
        // Imported type reference: `name ('[]')? ('(' constraints ')')? '@' ref`
        // — the terminal-`@` grammar (O-B1). Postfix `[]`/constraints ride on
        // the enclosing atom.
        TypeExpr::Imported { name, reference } => {
            out.push_str(name);
            if atom.is_array {
                out.push_str("[]");
            }
            if !atom.constraints.is_empty() {
                out.push('(');
                write_constraints(&mut out, &atom.constraints, SimplifiedType::Any);
                out.push(')');
            }
            out.push('@');
            out.push_str(reference);
        }
    }

    if let Some(desc) = &atom.description {
        out.push_str(" -> ");
        out.push_str(desc);
    }

    out
}

fn write_inline_object(out: &mut String, shape: &super::types::SchemaShape, is_array: bool) {
    out.push('{');
    for (i, (name, def)) in shape.properties.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(name);
        out.push(':');
        out.push(' ');
        // The recursive inline object case: serialize the property's
        // PropertyAtom and prefix with a single space so the produced
        // `{ name: <atom> }` form re-parses. Whitespace inside braces is
        // stripped by the grammar parser (Decision #2).
        let serialized = serialize_property_atom(def_single_atom(def));
        out.push_str(&serialized);
    }
    out.push('}');
    if is_array {
        out.push_str("[]");
    }
}

fn def_single_atom(def: &super::types::PropertyDef) -> &PropertyAtom {
    match def {
        super::types::PropertyDef::Single(atom) => atom,
        // Property-level unions are not supported inside inline objects by
        // this feature. Fall back to using the first arm so serialization
        // stays total; the output may not round-trip on re-parse, which is
        // an acceptable signal.
        super::types::PropertyDef::Union(arms) => arms
            .first()
            .expect("property-level union inside an inline object must have at least one arm"),
    }
}

fn write_constraints(out: &mut String, constraints: &[Constraint], ty: SimplifiedType) {
    for (idx, c) in constraints.iter().enumerate() {
        if idx > 0 {
            out.push(';');
        }
        write_constraint(out, c, ty);
    }
}

fn write_constraint(out: &mut String, c: &Constraint, ty: SimplifiedType) {
    match c {
        Constraint::Required => out.push_str("required"),
        Constraint::Eager => out.push_str("eager"),
        Constraint::NotEmpty => out.push_str("not-empty"),
        Constraint::Integer => out.push_str("integer"),
        Constraint::Unique => out.push_str("unique"),
        Constraint::Generated => out.push_str("generated"),

        Constraint::Min(n) => {
            let _ = write!(out, "min({})", format_number(*n));
        }
        Constraint::Max(n) => {
            let _ = write!(out, "max({})", format_number(*n));
        }
        Constraint::MinLen(n) => {
            let _ = write!(out, "min({n})");
        }
        Constraint::MaxLen(n) => {
            let _ = write!(out, "max({n})");
        }
        Constraint::MinItems(n) => {
            let _ = write!(out, "min({n})");
        }
        Constraint::MaxItems(n) => {
            let _ = write!(out, "max({n})");
        }
        Constraint::MinKeys(n) => {
            let _ = write!(out, "min-keys({n})");
        }
        Constraint::MaxKeys(n) => {
            let _ = write!(out, "max-keys({n})");
        }

        Constraint::Pattern(p) => {
            out.push_str("pattern(");
            write_arg(out, p);
            out.push(')');
        }
        Constraint::Suggest(candidates) => {
            out.push_str("suggest(");
            for (i, candidate) in candidates.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let value = if matches!(ty, SimplifiedType::Number) {
                    candidate
                        .canonical_decimal
                        .as_deref()
                        .unwrap_or(&candidate.decoded)
                } else {
                    &candidate.decoded
                };
                write_arg(out, value);
            }
            out.push(')');
        }
        Constraint::Members(members) => {
            // Members are positional, no keyword.
            for (i, m) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_arg(out, m);
            }
        }
        Constraint::LiteralValue(value) => write_literal_value(out, value),
        Constraint::Match(globs) => {
            out.push_str("match(");
            for (i, g) in globs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_arg(out, g);
            }
            out.push(')');
        }
        Constraint::Scheme(schemes) => {
            out.push_str("scheme(");
            for (i, s) in schemes.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_arg(out, s);
            }
            out.push(')');
        }
        Constraint::Example(refs) => {
            out.push_str("example(");
            for (i, r) in refs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_arg(out, r);
            }
            out.push(')');
        }
        Constraint::Default(value) => {
            out.push_str("default(");
            write_default_value(out, value, ty);
            out.push(')');
        }
    }
}

fn write_default_value(out: &mut String, value: &serde_json::Value, _ty: SimplifiedType) {
    match value {
        serde_json::Value::Null => out.push_str("null"),
        serde_json::Value::Bool(b) => {
            let _ = write!(out, "{b}");
        }
        serde_json::Value::Number(n) => {
            let _ = write!(out, "{n}");
        }
        serde_json::Value::String(s) => write_arg(out, s),
        // Compound defaults are not supported by the v1 grammar.
        other => {
            let _ = write!(out, "\"{other}\"");
        }
    }
}

/// Write an argument, quoting it if it contains characters that would
/// terminate a bare word.
fn write_arg(out: &mut String, value: &str) {
    if needs_quoting(value) {
        write_quoted(out, value);
    } else {
        out.push_str(value);
    }
}

/// Serialize a `literal(...)` positional value with its lexed type, so it
/// re-parses to the same JSON value. Numbers/booleans emit their canonical
/// spelling; strings are quoted when a bare form would either terminate the
/// token or re-type on re-parse (`'2'`, `'true'`, `'a, b'`).
fn write_literal_value(out: &mut String, value: &serde_json::Value) {
    match value {
        serde_json::Value::Bool(b) => {
            let _ = write!(out, "{b}");
        }
        serde_json::Value::Number(n) => {
            let _ = write!(out, "{n}");
        }
        serde_json::Value::String(s) => {
            if needs_quoting(s) || !super::grammar::bare_literal_round_trips_as_string(s) {
                write_quoted(out, s);
            } else {
                out.push_str(s);
            }
        }
        // A literal value is only ever a scalar (string / number / boolean);
        // any other JSON shape is unreachable from the grammar. Serialize
        // defensively so the function stays total.
        other => {
            let _ = write!(out, "\"{other}\"");
        }
    }
}

/// Write `value` as a single-quoted SimplifiedSchema argument, escaping `\`
/// and `'`.
fn write_quoted(out: &mut String, value: &str) {
    out.push('\'');
    for c in value.chars() {
        if c == '\\' || c == '\'' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('\'');
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, ',' | ';' | '(' | ')' | '\'' | '"'))
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::parse_type_expr;
    use super::super::types::*;
    use super::*;

    fn round_trip(atom: PropertyAtom) {
        let s = serialize_property_atom(&atom);
        let parsed = parse_type_expr("test", &s)
            .unwrap_or_else(|e| panic!("re-parse of {s:?} failed: {e:?}"));
        assert_eq!(parsed, atom, "round-trip mismatch for {s:?}");
    }

    #[test]
    fn round_trip_bare_string() {
        round_trip(PropertyAtom::bare(SimplifiedType::String));
    }

    #[test]
    fn round_trip_required_string() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::Required],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_generated_string() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::Generated],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_generated_with_required() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::Generated, Constraint::Required],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_string_with_length_bounds() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::MinLen(5), Constraint::MaxLen(80)],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_array_with_array_constraints() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: true,
            constraints: vec![Constraint::Pattern("^[a-z]+$".into())],
            array_constraints: vec![
                Constraint::MinItems(1),
                Constraint::MaxItems(5),
                Constraint::Unique,
            ],
            description: None,
        });
    }

    #[test]
    fn round_trip_enum_members() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Enum),
            is_array: false,
            constraints: vec![Constraint::Members(vec![
                "red".into(),
                "green".into(),
                "blue".into(),
            ])],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_enum_with_quoted_member() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Enum),
            is_array: false,
            constraints: vec![Constraint::Members(vec![
                "needs, quoting".into(),
                "plain".into(),
            ])],
            array_constraints: vec![],
            description: None,
        });
    }

    fn literal_atom(value: serde_json::Value) -> PropertyAtom {
        PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Literal),
            is_array: false,
            constraints: vec![Constraint::LiteralValue(value)],
            array_constraints: vec![],
            description: None,
        }
    }

    #[test]
    fn serializes_string_literal() {
        assert_eq!(
            serialize_property_atom(&literal_atom(serde_json::json!("spec"))),
            "literal(spec)"
        );
    }

    #[test]
    fn serializes_typed_literals() {
        assert_eq!(
            serialize_property_atom(&literal_atom(serde_json::json!(2))),
            "literal(2)"
        );
        assert_eq!(
            serialize_property_atom(&literal_atom(serde_json::json!(false))),
            "literal(false)"
        );
    }

    #[test]
    fn quotes_literal_strings_that_would_re_type() {
        // A string that looks like a number / boolean / null must be quoted so
        // it re-parses as a string rather than the scalar it resembles.
        assert_eq!(
            serialize_property_atom(&literal_atom(serde_json::json!("2"))),
            "literal('2')"
        );
        assert_eq!(
            serialize_property_atom(&literal_atom(serde_json::json!("true"))),
            "literal('true')"
        );
        assert_eq!(
            serialize_property_atom(&literal_atom(serde_json::json!("a, b"))),
            "literal('a, b')"
        );
    }

    #[test]
    fn round_trip_literals() {
        for value in [
            serde_json::json!("spec"),
            serde_json::json!("2"),
            serde_json::json!("true"),
            serde_json::json!("a, b"),
            serde_json::json!(2),
            serde_json::json!(-5),
            serde_json::json!(2.5),
            serde_json::json!(true),
            serde_json::json!(false),
        ] {
            round_trip(literal_atom(value));
        }
    }

    #[test]
    fn round_trip_literal_with_required() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Literal),
            is_array: false,
            constraints: vec![
                Constraint::LiteralValue(serde_json::json!("spec")),
                Constraint::Required,
            ],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_literal_array() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Literal),
            is_array: true,
            constraints: vec![Constraint::LiteralValue(serde_json::json!("auto"))],
            array_constraints: vec![Constraint::MinItems(1)],
            description: None,
        });
    }

    #[test]
    fn round_trip_expression() {
        round_trip(PropertyAtom::bare(SimplifiedType::Expression));
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Expression),
            is_array: false,
            constraints: vec![Constraint::Required],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_url_with_scheme() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Url),
            is_array: false,
            constraints: vec![Constraint::Scheme(vec!["https".into(), "http".into()])],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_file_with_match() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::File),
            is_array: true,
            constraints: vec![Constraint::Match(vec!["*.png".into(), "!_*.png".into()])],
            array_constraints: vec![Constraint::MinItems(1)],
            description: None,
        });
    }

    #[test]
    fn round_trip_file_eager() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::File),
            is_array: false,
            constraints: vec![Constraint::Eager, Constraint::Required],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_file_eager_array_with_match() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::File),
            is_array: true,
            constraints: vec![Constraint::Eager, Constraint::Match(vec!["*.png".into()])],
            array_constraints: vec![Constraint::MinItems(1)],
            description: None,
        });
    }

    #[test]
    fn round_trip_with_description() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::Required],
            array_constraints: vec![],
            description: Some("The author's full name".into()),
        });
    }

    #[test]
    fn round_trip_default_string() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::Default(serde_json::Value::String(
                "hello".into(),
            ))],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_default_number() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Number),
            is_array: false,
            // An integer default parses back as an integer (exact-lexeme parse),
            // so the round-trip fixture must itself be an integer.
            constraints: vec![Constraint::Default(serde_json::json!(3))],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_default_boolean() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Boolean),
            is_array: false,
            constraints: vec![Constraint::Default(serde_json::Value::Bool(true))],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_string_suggest() {
        let atom = PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::String),
            is_array: false,
            constraints: vec![Constraint::Suggest(vec![
                super::super::types::SuggestionCandidate {
                    decoded: "red".into(),
                    interpreted: serde_json::Value::String("red".into()),
                    canonical_decimal: None,
                    span: 0..0,
                },
                super::super::types::SuggestionCandidate {
                    decoded: "blue gray".into(),
                    interpreted: serde_json::Value::String("blue gray".into()),
                    canonical_decimal: None,
                    span: 0..0,
                },
            ])],
            array_constraints: vec![],
            description: None,
        };
        let s = serialize_property_atom(&atom);
        let parsed = parse_type_expr("test", &s)
            .unwrap_or_else(|e| panic!("re-parse of {s:?} failed: {e:?}"));
        let parsed_candidates = parsed.constraints.iter().find_map(|c| match c {
            Constraint::Suggest(cands) => Some(cands),
            _ => None,
        });
        let orig_candidates = atom.constraints.iter().find_map(|c| match c {
            Constraint::Suggest(cands) => Some(cands),
            _ => None,
        });
        let (parsed_candidates, orig_candidates) =
            (parsed_candidates.unwrap(), orig_candidates.unwrap());
        assert_eq!(parsed_candidates.len(), orig_candidates.len());
        assert_eq!(parsed_candidates[0].decoded, "red");
        assert_eq!(parsed_candidates[0].interpreted, serde_json::json!("red"));
        assert_eq!(parsed_candidates[1].decoded, "blue gray");
        assert_eq!(parsed_candidates[1].interpreted, serde_json::json!("blue gray"));
    }

    #[test]
    fn round_trip_number_suggest() {
        let atom = PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Number),
            is_array: false,
            constraints: vec![Constraint::Suggest(vec![
                super::super::types::SuggestionCandidate {
                    decoded: "1".into(),
                    interpreted: serde_json::json!(1),
                    canonical_decimal: Some("1".into()),
                    span: 0..0,
                },
                super::super::types::SuggestionCandidate {
                    decoded: "2.5".into(),
                    interpreted: serde_json::json!(2.5),
                    canonical_decimal: Some("2.5".into()),
                    span: 0..0,
                },
            ])],
            array_constraints: vec![],
            description: None,
        };
        let s = serialize_property_atom(&atom);
        let parsed = parse_type_expr("test", &s)
            .unwrap_or_else(|e| panic!("re-parse of {s:?} failed: {e:?}"));
        let parsed_candidates = parsed.constraints.iter().find_map(|c| match c {
            Constraint::Suggest(cands) => Some(cands),
            _ => None,
        });
        let parsed_candidates = parsed_candidates.unwrap();
        assert_eq!(parsed_candidates.len(), 2);
        assert_eq!(parsed_candidates[0].decoded, "1");
        assert_eq!(parsed_candidates[0].interpreted, serde_json::json!(1));
        assert_eq!(parsed_candidates[0].canonical_decimal.as_deref(), Some("1"));
        assert_eq!(parsed_candidates[1].decoded, "2.5");
        assert_eq!(parsed_candidates[1].interpreted, serde_json::json!(2.5));
        assert_eq!(parsed_candidates[1].canonical_decimal.as_deref(), Some("2.5"));
    }

    #[test]
    fn round_trip_number_suggest_array() {
        let atom = PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Number),
            is_array: true,
            constraints: vec![Constraint::Suggest(vec![
                super::super::types::SuggestionCandidate {
                    decoded: "1".into(),
                    interpreted: serde_json::json!(1),
                    canonical_decimal: Some("1".into()),
                    span: 0..0,
                },
            ])],
            array_constraints: vec![],
            description: None,
        };
        let s = serialize_property_atom(&atom);
        let parsed = parse_type_expr("test", &s)
            .unwrap_or_else(|e| panic!("re-parse of {s:?} failed: {e:?}"));
        assert!(parsed.is_array);
        let has_suggest = parsed.constraints.iter().any(|c| matches!(c, Constraint::Suggest(_)));
        assert!(has_suggest, "parsed atom must retain suggest constraint");
    }
}
