//! Serialise a [`PropertyAtom`] back into a type-and-constraint string.
//!
//! Used by tests (especially `proptest` round-trip tests) to verify that the
//! parser and the AST agree. The output is canonical (deterministic key
//! ordering, minimal whitespace) and re-parses to an equal [`PropertyAtom`].

use std::fmt::Write;

use super::types::{Constraint, PropertyAtom, SimplifiedType};

/// Serialise `atom` into a string suitable for re-parsing via
/// [`grammar::parse_type_expr`].
///
/// [`grammar::parse_type_expr`]: super::grammar::parse_type_expr
pub fn serialize_property_atom(atom: &PropertyAtom) -> String {
    let mut out = String::new();
    out.push_str(atom.ty.as_keyword());

    if !atom.constraints.is_empty() {
        out.push('(');
        write_constraints(&mut out, &atom.constraints, atom.ty);
        out.push(')');
    } else if matches!(atom.ty, SimplifiedType::Enum) {
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
            write_constraints(&mut out, &atom.array_constraints, atom.ty);
            out.push(')');
        }
    }

    if let Some(desc) = &atom.description {
        out.push_str(" -> ");
        out.push_str(desc);
    }

    out
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
        Constraint::NotEmpty => out.push_str("not-empty"),
        Constraint::Integer => out.push_str("integer"),
        Constraint::Unique => out.push_str("unique"),

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

        Constraint::Pattern(p) => {
            out.push_str("pattern(");
            write_arg(out, p);
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
        out.push('\'');
        for c in value.chars() {
            if c == '\\' || c == '\'' {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('\'');
    } else {
        out.push_str(value);
    }
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value.chars().any(|c| {
            c.is_whitespace()
                || matches!(c, ',' | ';' | '(' | ')' | '\'' | '"')
        })
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
            ty: SimplifiedType::String,
            is_array: false,
            constraints: vec![Constraint::Required],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_string_with_length_bounds() {
        round_trip(PropertyAtom {
            ty: SimplifiedType::String,
            is_array: false,
            constraints: vec![Constraint::MinLen(5), Constraint::MaxLen(80)],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_array_with_array_constraints() {
        round_trip(PropertyAtom {
            ty: SimplifiedType::String,
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
            ty: SimplifiedType::Enum,
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
            ty: SimplifiedType::Enum,
            is_array: false,
            constraints: vec![Constraint::Members(vec![
                "needs, quoting".into(),
                "plain".into(),
            ])],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_url_with_scheme() {
        round_trip(PropertyAtom {
            ty: SimplifiedType::Url,
            is_array: false,
            constraints: vec![Constraint::Scheme(vec!["https".into(), "http".into()])],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_file_with_match() {
        round_trip(PropertyAtom {
            ty: SimplifiedType::File,
            is_array: true,
            constraints: vec![Constraint::Match(vec!["*.png".into(), "!_*.png".into()])],
            array_constraints: vec![Constraint::MinItems(1)],
            description: None,
        });
    }

    #[test]
    fn round_trip_with_description() {
        round_trip(PropertyAtom {
            ty: SimplifiedType::String,
            is_array: false,
            constraints: vec![Constraint::Required],
            array_constraints: vec![],
            description: Some("The author's full name".into()),
        });
    }

    #[test]
    fn round_trip_default_string() {
        round_trip(PropertyAtom {
            ty: SimplifiedType::String,
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
            ty: SimplifiedType::Number,
            is_array: false,
            constraints: vec![Constraint::Default(serde_json::json!(3.0))],
            array_constraints: vec![],
            description: None,
        });
    }

    #[test]
    fn round_trip_default_boolean() {
        round_trip(PropertyAtom {
            ty: SimplifiedType::Boolean,
            is_array: false,
            constraints: vec![Constraint::Default(serde_json::Value::Bool(true))],
            array_constraints: vec![],
            description: None,
        });
    }
}
