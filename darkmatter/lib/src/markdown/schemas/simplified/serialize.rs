//! Serialise a [`PropertyAtom`] back into a type-and-constraint string, and
//! one [`Constraint`] into the text diagnostics show
//! ([`Constraint::to_schema_text`], [`Constraint::diagnostic_excerpt`]).
//!
//! Round-trip tests use the atom form to verify that the parser and the AST
//! agree. The output is canonical (deterministic key
//! ordering, minimal whitespace) and re-parses to an equal [`PropertyAtom`]
//! for every atom the passive parser produces, except that `alphanumeric`
//! options are written in canonical form, which re-parses to the same type
//! identity rather than the authored option order.

use std::fmt::Write;

use super::{
    alphanumeric::AlphanumericOptions,
    types::{
        CallExample, Constraint, ExampleOutcome, FunctionParameters, FunctionType, PropertyAtom,
        PropertyDef, SimplifiedType, TupleType, TypeExpr,
    },
};
use crate::markdown::numeric::{NumberTextError, admit_number_spelling, format_number};

/// The most Unicode scalar values a [`Constraint::diagnostic_excerpt`] holds,
/// including the final `…` of a shortened excerpt.
const DIAGNOSTIC_EXCERPT_LIMIT: usize = 160;

impl Constraint {
    /// This one constraint in canonical SimplifiedSchema spelling, as the
    /// serializer writes it inside a constraint list.
    ///
    /// `ty` is the constrained type, `None` when it is unknown; it only
    /// selects the canonical spelling of numeric `suggest(...)` candidates.
    /// An [`Constraint::Options`] set is written as the type it names:
    /// `alphanumeric(allow-dashes,allow-periods)` in canonical option order,
    /// or bare `alphanumeric` for the empty set. A default keeps its JSON
    /// type visible (`default(3)` versus `default('3')`, `default(null)`
    /// versus `default('null')`); an object or array default, which only Rust
    /// can construct, is written as `default(<compact JSON>)`, a diagnostic
    /// spelling the grammar does not parse.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::schemas::simplified::Constraint;
    ///
    /// assert_eq!(Constraint::MinLen(10).to_schema_text(None), "min(10)");
    /// assert_eq!(Constraint::Options(vec![]).to_schema_text(None), "alphanumeric");
    /// assert_eq!(
    ///     Constraint::Default(serde_json::json!("3")).to_schema_text(None),
    ///     "default('3')"
    /// );
    /// ```
    pub fn to_schema_text(&self, ty: Option<SimplifiedType>) -> String {
        let mut out = String::new();
        match self {
            Constraint::Options(options) => {
                out.push_str(SimplifiedType::Alphanumeric.as_keyword());
                let keywords = options
                    .iter()
                    .copied()
                    .collect::<AlphanumericOptions>()
                    .canonical()
                    .keywords()
                    .collect::<Vec<_>>();
                if !keywords.is_empty() {
                    let _ = write!(out, "({})", keywords.join(","));
                }
            }
            other => write_constraint(&mut out, other, ty),
        }
        out
    }

    /// The text every diagnostic shows for this constraint: the error's
    /// `Display`, the terminal status block, and editor related information.
    ///
    /// It is [`Constraint::to_schema_text`] with no type, made safe for one
    /// line of a message. Line feed, carriage return, and tab become `\n`,
    /// `\r`, and `\t`; every other control character, and the Unicode line
    /// and paragraph separators, become `\u{hex}` (ESC is `\u{1b}`). The
    /// escaped text is then shortened to at most 160 Unicode scalar values,
    /// the last of which is `…` when anything was cut; an escape is never
    /// split, so a shortened excerpt can be slightly under the limit.
    ///
    /// The excerpt is for reading, not reparsing: a backslash already in the
    /// text is not escaped. Markup escaping for a particular output (terminal
    /// Prose, Markdown code spans) is that renderer's job.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::schemas::simplified::Constraint;
    ///
    /// let tab = Constraint::Default(serde_json::json!("a\tb"));
    /// assert_eq!(tab.diagnostic_excerpt(), r"default('a\tb')");
    ///
    /// let long = Constraint::Default(serde_json::json!("é".repeat(300)));
    /// let excerpt = long.diagnostic_excerpt();
    /// assert_eq!(excerpt.chars().count(), 160);
    /// assert!(excerpt.ends_with('…'));
    /// ```
    pub fn diagnostic_excerpt(&self) -> String {
        let text = self.to_schema_text(None);
        let mut escaped = String::with_capacity(text.len());
        // Scalar count and byte length of `escaped` after each source char,
        // so shortening cuts only between escapes.
        let mut ends = Vec::with_capacity(text.len());
        let mut scalars = 0;
        for c in text.chars() {
            let before = escaped.len();
            push_excerpt_char(&mut escaped, c);
            scalars += escaped[before..].chars().count();
            ends.push((scalars, escaped.len()));
        }
        if scalars <= DIAGNOSTIC_EXCERPT_LIMIT {
            return escaped;
        }
        let keep = ends
            .iter()
            .rev()
            .find(|(count, _)| *count < DIAGNOSTIC_EXCERPT_LIMIT)
            .map_or(0, |(_, bytes)| *bytes);
        escaped.truncate(keep);
        escaped.push('…');
        escaped
    }
}

fn push_excerpt_char(out: &mut String, c: char) {
    match c {
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') => {
            let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
        }
        c => out.push(c),
    }
}

/// Serialise `atom` into a string suitable for re-parsing via
/// [`grammar::parse_type_expr`].
///
/// Only passively parsed atoms round-trip. A resolved [`TypeExpr::Union`]
/// without `[]` (an expanded postfixed union reference) serializes to its
/// options in parentheses, which re-parse as a union of those options rather
/// than one atom.
///
/// [`grammar::parse_type_expr`]: super::grammar::parse_type_expr
pub fn serialize_property_atom(atom: &PropertyAtom) -> String {
    let mut out = String::new();
    match &atom.ty {
        TypeExpr::Primitive(ty) => {
            out.push_str(ty.as_keyword());
            let constraints = render_constraints(&atom.constraints, Some(*ty));
            if !constraints.is_empty() {
                out.push('(');
                out.push_str(&constraints);
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
                    out.push_str(&render_constraints(&atom.array_constraints, Some(*ty)));
                    out.push(')');
                }
            }
        }
        TypeExpr::InlineObject(shape) => {
            write_inline_object(&mut out, shape, atom.is_array);
            if atom.is_array && !atom.array_constraints.is_empty() {
                out.push('(');
                out.push_str(&render_constraints(&atom.array_constraints, None));
                out.push(')');
            }
            if !atom.is_array && !atom.constraints.is_empty() {
                out.push('(');
                out.push_str(&render_constraints(&atom.constraints, None));
                out.push(')');
            }
        }
        // Named-type reference:
        // `name ('(' constraints ')')? ('[]' ('(' constraints ')')?)? ('@' ref)?`.
        // A local reference is written bare unless its name is a built-in
        // keyword, where only `@this` selects the local type.
        //
        // A resolved reference to a cyclic definition is written as a
        // back-reference by name, with no namespace.
        TypeExpr::Imported { name, .. } | TypeExpr::Ref(super::DefKey { name, .. }) => {
            out.push_str(name);
            let constraints = render_constraints(&atom.constraints, None);
            if !constraints.is_empty() {
                out.push('(');
                out.push_str(&constraints);
                out.push(')');
            }
            if atom.is_array {
                out.push_str("[]");
                if !atom.array_constraints.is_empty() {
                    out.push('(');
                    out.push_str(&render_constraints(&atom.array_constraints, None));
                    out.push(')');
                }
            }
            if let TypeExpr::Imported { reference, .. } = &atom.ty
                && (!atom.ty.is_local_reference() || SimplifiedType::is_keyword(name))
            {
                out.push('@');
                out.push_str(reference);
            }
        }
        // `(A | B)[]`, the parsed list of a union.
        TypeExpr::Union(arms) => {
            out.push('(');
            let rendered: Vec<String> = arms.iter().map(serialize_property_atom).collect();
            out.push_str(&rendered.join(" | "));
            out.push(')');
            write_array_suffix(&mut out, atom);
        }
        TypeExpr::Tuple(tuple) => {
            write_tuple(&mut out, tuple, &atom.constraints);
            write_array_suffix(&mut out, atom);
        }
        TypeExpr::Function(function) => {
            write_function(&mut out, function, &atom.constraints);
            write_array_suffix(&mut out, atom);
        }
    }

    if let Some(desc) = &atom.description {
        out.push_str(" -> ");
        out.push_str(desc);
    }

    out
}

/// Serialise `def` as a type expression: a [`PropertyDef::Union`] is written
/// with `|`, which re-parses to the same union. Only the last option's
/// description survives, as only it can be written in that form.
pub fn serialize_property_def(def: &PropertyDef) -> String {
    match def {
        PropertyDef::Single(atom) => serialize_property_atom(atom),
        PropertyDef::Union(arms) => arms
            .iter()
            .map(serialize_property_atom)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn write_array_suffix(out: &mut String, atom: &PropertyAtom) {
    if atom.is_array {
        out.push_str("[]");
        if !atom.array_constraints.is_empty() {
            out.push('(');
            out.push_str(&render_constraints(&atom.array_constraints, None));
            out.push(')');
        }
    }
}

/// A nested definition, parenthesized when `?` must apply to all of it.
fn write_optional_def(out: &mut String, def: &PropertyDef, optional: bool) {
    let text = serialize_property_def(def);
    if optional && matches!(def, PropertyDef::Union(_)) {
        out.push('(');
        out.push_str(&text);
        out.push(')');
    } else {
        out.push_str(&text);
    }
    if optional {
        out.push('?');
    }
}

fn write_tuple(out: &mut String, tuple: &TupleType, constraints: &[Constraint]) {
    out.push_str("tuple([");
    let mut first = true;
    for element in &tuple.elements {
        if !first {
            out.push_str(", ");
        }
        first = false;
        write_optional_def(out, &element.def, element.optional);
    }
    if let Some(spread) = &tuple.spread {
        if !first {
            out.push_str(", ");
        }
        out.push_str("...");
        out.push_str(&serialize_property_atom(spread));
    }
    out.push(']');
    if !constraints.is_empty() {
        out.push_str("; ");
        out.push_str(&render_constraints(constraints, None));
    }
    out.push(')');
}

fn write_function(out: &mut String, function: &FunctionType, constraints: &[Constraint]) {
    out.push_str("function(parameters(");
    match &function.parameters {
        FunctionParameters::Void => out.push_str("void"),
        FunctionParameters::List(parameters) => {
            for (index, parameter) in parameters.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                if parameter.rest {
                    out.push_str("...");
                }
                out.push_str(&parameter.name);
                if parameter.optional {
                    out.push('?');
                }
                out.push_str(": ");
                out.push_str(&serialize_property_def(&parameter.def));
            }
        }
    }
    out.push_str("); returns(");
    out.push_str(&serialize_property_def(&function.returns));
    out.push(')');
    if function.fallible {
        out.push_str("; fallible");
    }
    for example in &function.examples {
        out.push_str("; ");
        write_call_example(out, example);
    }
    if !constraints.is_empty() {
        out.push_str("; ");
        out.push_str(&render_constraints(constraints, None));
    }
    out.push(')');
}

fn write_call_example(out: &mut String, example: &CallExample) {
    out.push_str("example(");
    out.push_str(&example.call);
    out.push_str(" => ");
    match &example.expected {
        ExampleOutcome::Error => out.push_str("error"),
        ExampleOutcome::Value(value) => out.push_str(&value.to_string()),
    }
    if let Some(reason) = &example.display_only {
        out.push_str("; display-only: ");
        out.push_str(reason);
    }
    out.push(')');
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
        out.push_str(&serialize_property_def(def));
    }
    out.push('}');
    if is_array {
        out.push_str("[]");
    }
}

/// Renders a constraint list without its parentheses. `ty` is the target
/// type, `None` for a reference or inline object. Concrete `alphanumeric`
/// options are canonicalized; unresolved references retain their assertions
/// until the resolver can compare them with the target's option set.
fn render_constraints(constraints: &[Constraint], ty: Option<SimplifiedType>) -> String {
    let mut out = String::new();
    let options = constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Options(options) => Some(options.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect::<AlphanumericOptions>();
    let options = if ty == Some(SimplifiedType::Alphanumeric) {
        options.canonical()
    } else {
        options
    };
    out.push_str(&options.keywords().collect::<Vec<_>>().join(","));
    for c in constraints
        .iter()
        .filter(|constraint| !matches!(constraint, Constraint::Options(_)))
    {
        if !out.is_empty() {
            out.push(';');
        }
        write_constraint(&mut out, c, ty);
    }
    out
}

fn write_constraint(out: &mut String, c: &Constraint, ty: Option<SimplifiedType>) {
    match c {
        Constraint::Required => out.push_str("required"),
        Constraint::Eager => out.push_str("eager"),
        Constraint::NotEmpty => out.push_str("not-empty"),
        Constraint::Integer => out.push_str("integer"),
        Constraint::Unique => out.push_str("unique"),
        Constraint::Generated => out.push_str("generated"),
        Constraint::Deferred => out.push_str("deferred"),

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
        Constraint::UncountedKeys(keys) => {
            out.push_str("uncounted-keys(");
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_arg(out, key);
            }
            out.push(')');
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
                let value = if ty == Some(SimplifiedType::Number) {
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
        Constraint::Options(_) => unreachable!("options are rendered first, in canonical form"),
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
        Constraint::Category(name) => {
            let _ = write!(out, "category({name})");
        }
        Constraint::Operands(name) => {
            let _ = write!(out, "operands({name})");
        }
    }
}

fn write_default_value(out: &mut String, value: &serde_json::Value, _ty: Option<SimplifiedType>) {
    match value {
        serde_json::Value::Null => out.push_str("null"),
        serde_json::Value::Bool(b) => {
            let _ = write!(out, "{b}");
        }
        serde_json::Value::Number(n) => {
            let _ = write!(out, "{n}");
        }
        serde_json::Value::String(s) => {
            if needs_quoting(s) || !bare_default_round_trips_as_string(s) {
                write_quoted(out, s);
            } else {
                out.push_str(s);
            }
        }
        // The grammar has no compound default; only Rust can build one, and
        // it is shown as compact JSON so diagnostics still carry the value.
        other => {
            let _ = write!(out, "{other}");
        }
    }
}

/// Whether bare `value` re-parses as the same string `default(...)`: not a
/// `true`/`false`/`null` keyword and not any number spelling, which the
/// grammar either reads as a number or rejects as inadmissible.
fn bare_default_round_trips_as_string(value: &str) -> bool {
    !matches!(value, "true" | "false" | "null")
        && matches!(admit_number_spelling(value), Err(NumberTextError::NotANumber))
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

/// Whether `value` is not one bare argument word as `Lexer::read_word` reads
/// it, which also ends a word at the description arrow `->`.
fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value.contains("->")
        || value
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, ',' | ';' | '(' | ')' | '\'' | '"'))
}

#[cfg(test)]
mod tests {
    use super::super::grammar::parse_type_expr;
    use super::super::types::*;
    use super::*;

    #[test]
    fn universal_eager_and_array_placement_round_trip() {
        for source in [
            "string(eager)",
            "number(eager)",
            "object(eager)",
            "datetime(eager)",
            "file(eager)[]",
            "file[](eager)",
        ] {
            let parsed = super::super::grammar::parse_type_expr("value", source)
                .unwrap_or_else(|error| panic!("parse {source}: {error}"));
            let serialized = serialize_property_atom(&parsed);
            let reparsed = super::super::grammar::parse_type_expr("value", &serialized)
                .unwrap_or_else(|error| panic!("reparse {serialized}: {error}"));
            assert_eq!(reparsed, parsed, "{source} -> {serialized}");
        }
    }

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
    fn round_trip_object_with_uncounted_keys() {
        let atom = parse_type_expr("test", "{ info: string, when: string }(max-keys(1); uncounted-keys(when))")
            .expect("parses");
        assert!(atom.constraints.contains(&Constraint::UncountedKeys(vec!["when".to_string()])));
        round_trip(atom);
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

    /// A bare argument word ends at `->`, so a value containing one is quoted.
    #[test]
    fn round_trip_arguments_containing_the_description_arrow() {
        round_trip(PropertyAtom {
            ty: TypeExpr::Primitive(SimplifiedType::Enum),
            is_array: false,
            constraints: vec![Constraint::Members(vec!["a->b".into(), "plain".into()])],
            array_constraints: vec![],
            description: None,
        });
        round_trip(literal_atom(serde_json::Value::String("x->y".into())));
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

    fn default_atom(ty: SimplifiedType, value: serde_json::Value) -> PropertyAtom {
        PropertyAtom {
            ty: TypeExpr::Primitive(ty),
            is_array: false,
            constraints: vec![Constraint::Default(value)],
            array_constraints: vec![],
            description: None,
        }
    }

    #[test]
    fn default_strings_that_look_like_other_scalars_are_quoted_and_round_trip() {
        for (value, written) in [
            (serde_json::json!("3"), "string(default('3'))"),
            (serde_json::json!("-2.5"), "string(default('-2.5'))"),
            (serde_json::json!("1e3"), "string(default('1e3'))"),
            (serde_json::json!("1e999"), "string(default('1e999'))"),
            (serde_json::json!("null"), "string(default('null'))"),
            (serde_json::json!("true"), "string(default('true'))"),
            (serde_json::json!("false"), "string(default('false'))"),
            (serde_json::json!("draft"), "string(default(draft))"),
            (serde_json::json!("v1.2"), "string(default(v1.2))"),
        ] {
            let atom = default_atom(SimplifiedType::String, value);
            assert_eq!(serialize_property_atom(&atom), written);
            round_trip(atom);
        }
        for (ty, value, written) in [
            (SimplifiedType::Number, serde_json::json!(3), "number(default(3))"),
            (SimplifiedType::Number, serde_json::json!(-2.5), "number(default(-2.5))"),
            (SimplifiedType::Boolean, serde_json::json!(true), "boolean(default(true))"),
            (SimplifiedType::Unknown, serde_json::Value::Null, "unknown(default(null))"),
        ] {
            let atom = default_atom(ty, value);
            assert_eq!(serialize_property_atom(&atom), written);
            round_trip(atom);
        }
    }

    #[test]
    fn to_schema_text_writes_one_constraint_canonically() {
        use super::super::alphanumeric::AlphanumericOption::*;
        for (constraint, expected) in [
            (Constraint::Min(10.0), "min(10)"),
            (Constraint::Max(2.5), "max(2.5)"),
            (Constraint::MinLen(3), "min(3)"),
            (Constraint::MaxItems(5), "max(5)"),
            (Constraint::MinKeys(1), "min-keys(1)"),
            (Constraint::MaxKeys(9), "max-keys(9)"),
            (Constraint::Scheme(vec!["http".into(), "https".into()]), "scheme(http,https)"),
            (Constraint::Scheme(vec![]), "scheme()"),
            (Constraint::Default(serde_json::json!(3)), "default(3)"),
            (Constraint::Default(serde_json::json!("3")), "default('3')"),
            (Constraint::Default(serde_json::Value::Null), "default(null)"),
            (Constraint::Default(serde_json::json!("null")), "default('null')"),
            (Constraint::Default(serde_json::json!(false)), "default(false)"),
            (Constraint::Default(serde_json::json!("false")), "default('false')"),
            (Constraint::Default(serde_json::json!("a, b")), "default('a, b')"),
            (Constraint::Default(serde_json::json!({"k": [1, "x"]})), r#"default({"k":[1,"x"]})"#),
            (Constraint::Default(serde_json::json!([])), "default([])"),
            (Constraint::Options(vec![]), "alphanumeric"),
            (Constraint::Options(vec![AllowPeriods, AllowDashes]), "alphanumeric(allow-dashes,allow-periods)"),
            // Canonical: a block whose character is not allowed is dropped.
            (Constraint::Options(vec![BlockLeadingDash]), "alphanumeric"),
            (Constraint::Required, "required"),
        ] {
            assert_eq!(constraint.to_schema_text(None), expected, "{constraint:?}");
        }
    }

    #[test]
    fn diagnostic_excerpt_escapes_controls_and_line_separators() {
        let constraint = Constraint::Default(serde_json::json!("a\nb\r\tc\u{1b}d\u{7f}e\u{2028}f"));
        assert_eq!(
            constraint.diagnostic_excerpt(),
            r"default('a\nb\r\tc\u{1b}d\u{7f}e\u{2028}f')"
        );
        assert_eq!(Constraint::Min(10.0).diagnostic_excerpt(), "min(10)");
    }

    #[test]
    fn diagnostic_excerpt_keeps_at_most_160_scalars_and_never_splits_an_escape() {
        let exact = Constraint::Default(serde_json::json!("é".repeat(151)));
        assert_eq!(exact.diagnostic_excerpt().chars().count(), 160, "`default(` + 151 + `)` fits");
        assert!(!exact.diagnostic_excerpt().contains('…'));

        let over = Constraint::Default(serde_json::json!("é".repeat(152)));
        let excerpt = over.diagnostic_excerpt();
        assert_eq!(excerpt.chars().count(), 160);
        assert!(excerpt.ends_with("é…"), "{excerpt}");

        // `default('` is 9 scalars; 147 `x` and a space reach 157, and the
        // next escape (`\u{1b}`, 6 scalars) would pass 159, so it is
        // dropped whole.
        let escapes = Constraint::Default(serde_json::json!(format!("{} \u{1b}{}", "x".repeat(147), "y".repeat(20))));
        let excerpt = escapes.diagnostic_excerpt();
        assert!(excerpt.chars().count() <= 160, "{excerpt}");
        assert!(excerpt.ends_with(" …"), "the escape is not split: {excerpt}");
    }

    /// Serializes `source`, asserts the canonical spelling, and asserts that
    /// it re-parses to the same AST.
    fn assert_canonical(source: &str, expected: &str) {
        let parsed = parse_type_expr("value", source)
            .unwrap_or_else(|error| panic!("parse {source}: {error}"));
        let serialized = serialize_property_atom(&parsed);
        assert_eq!(serialized, expected, "{source}");
        let reparsed = parse_type_expr("value", &serialized)
            .unwrap_or_else(|error| panic!("reparse {serialized}: {error}"));
        assert_eq!(reparsed, parsed, "{source} -> {serialized}");
    }

    #[test]
    fn local_reference_serializes_bare_and_round_trips() {
        for (source, expected) in [
            ("person", "person"),
            ("person@this", "person"),
            ("person[]", "person[]"),
            ("person(required)@this", "person(required)"),
            ("person[](min(1))", "person[](min(1))"),
            ("person(min(2); max(5))", "person(min(2);max(5))"),
            ("person -> The author", "person -> The author"),
            ("person[]@this -> Reviewers", "person[] -> Reviewers"),
        ] {
            assert_canonical(source, expected);
        }
    }

    #[test]
    fn local_reference_with_a_keyword_name_keeps_the_explicit_spelling() {
        assert_canonical("string@this", "string@this");
        assert_canonical("file[](required)@this -> Files", "file[](required)@this -> Files");
        assert_eq!(
            serialize_property_atom(&parse_type_expr("value", "string").unwrap()),
            "string"
        );
    }

    #[test]
    fn cross_file_reference_serialization_is_unchanged() {
        assert_canonical("person@./people.yaml", "person@./people.yaml");
        assert_canonical(
            "person[](required)@./people.yaml -> People",
            "person[](required)@./people.yaml -> People",
        );
    }

    #[test]
    fn local_reference_inside_an_inline_object_round_trips() {
        assert_canonical(
            "{ author: person(required) -> The author, editor: string@this }",
            "{author: person(required) -> The author,editor: string@this}",
        );
    }
}
