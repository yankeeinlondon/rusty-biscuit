//! The passive subtype relation among string-like SimplifiedSchema types.
//!
//! [`is_subtype`] answers "does every non-null value the underlying type A
//! accepts also satisfy the underlying type B?" from typed, **resolved** atoms.
//! Only a type and its identity (`alphanumeric` options, `enum` members, a
//! `literal` value) count; ordinary constraints, presence and timing markers,
//! defaults, and descriptions are ignored. The relation is bounded reasoning
//! over the listed types: it never inspects user regexes, converts scalars,
//! reads files, or resolves references, and it is no substitute for runtime
//! value checks or shell quoting.

use std::collections::BTreeSet;

use super::alphanumeric::AlphanumericOptions;
use super::types::{Constraint, PropertyAtom, SimplifiedType, TypeExpr};

/// Whether every non-null value accepted by the underlying type of `a` is
/// accepted by the underlying type of `b`.
///
/// - Two identical types (the same type with the same identity) are always
///   subtypes, whatever the type, including arrays and inline objects.
/// - `alphanumeric(X)` ⊆ `alphanumeric(Y)` when X's allowed characters and
///   allowed first characters are subsets of Y's, and X guarantees any
///   prohibition of `..` that Y makes.
/// - Every always-text type is a subtype of `string`.
/// - Every other pair, including an unresolved reference, is not.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
/// use darkmatter::markdown::schemas::is_subtype;
///
/// let atom = |text| parse_type_expr("p", text).unwrap();
/// let rule = atom("alphanumeric(allow-slashes)");
/// assert!(is_subtype(&atom("alphanumeric(min(3))"), &rule));
/// assert!(!is_subtype(&atom("string"), &rule));
/// assert!(is_subtype(&atom("email"), &atom("string")));
/// ```
pub fn is_subtype(a: &PropertyAtom, b: &PropertyAtom) -> bool {
    if a.is_array || b.is_array {
        return a.is_array && b.is_array && identity(a) == identity(b);
    }
    match (&a.ty, &b.ty) {
        (TypeExpr::Primitive(SimplifiedType::Alphanumeric), TypeExpr::Primitive(SimplifiedType::Alphanumeric)) => {
            alphanumeric_subset(options(a), options(b))
        }
        (TypeExpr::Primitive(_), TypeExpr::Primitive(SimplifiedType::String)) => {
            identity(a) == identity(b) || is_always_text(a)
        }
        _ => identity(a) == identity(b),
    }
}

/// Whether `atom` is a single string-like type: not an array, and every value
/// it accepts is a JSON string (the left column of the `string` row of
/// [`is_subtype`]'s table).
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
/// use darkmatter::markdown::schemas::is_string_like;
///
/// let atom = |text| parse_type_expr("p", text).unwrap();
/// assert!(is_string_like(&atom("alphanumeric(allow-slashes)")));
/// assert!(!is_string_like(&atom("number")));
/// assert!(!is_string_like(&atom("string[]")));
/// ```
pub fn is_string_like(atom: &PropertyAtom) -> bool {
    !atom.is_array && is_always_text(atom)
}

fn alphanumeric_subset(x: AlphanumericOptions, y: AlphanumericOptions) -> bool {
    x.allowed_characters().is_subset(y.allowed_characters())
        && x.first_characters().is_subset(y.first_characters())
        && (!y.forbids_double_period() || x.forbids_double_period())
}

fn options(atom: &PropertyAtom) -> AlphanumericOptions {
    atom.alphanumeric_options().unwrap_or_else(AlphanumericOptions::base)
}

/// Whether every value of the atom's type is a JSON string.
fn is_always_text(atom: &PropertyAtom) -> bool {
    match &atom.ty {
        TypeExpr::Primitive(ty) => match ty {
            SimplifiedType::String
            | SimplifiedType::Date
            | SimplifiedType::DateTime
            | SimplifiedType::Time
            | SimplifiedType::File
            | SimplifiedType::Enum
            | SimplifiedType::Url
            | SimplifiedType::Email
            | SimplifiedType::Yaml
            | SimplifiedType::Json
            | SimplifiedType::Expression
            | SimplifiedType::Alphanumeric
            | SimplifiedType::Shell => true,
            SimplifiedType::Literal => atom.literal_value().is_some_and(serde_json::Value::is_string),
            SimplifiedType::Number
            | SimplifiedType::NumberLike
            | SimplifiedType::Boolean
            | SimplifiedType::Boolish
            | SimplifiedType::Object
            | SimplifiedType::TypeDefinition
            | SimplifiedType::Schema
            | SimplifiedType::Unknown
            | SimplifiedType::Null => false,
        },
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } | TypeExpr::Union(_) => false,
    }
}

/// The parts of an atom that name its type, with ordinary constraints left
/// out. An unresolved reference has no identity, so it equals nothing.
enum Identity<'a> {
    Primitive {
        ty: SimplifiedType,
        options: Option<AlphanumericOptions>,
        members: Option<BTreeSet<&'a str>>,
        literal: Option<&'a serde_json::Value>,
    },
    Structured(&'a TypeExpr),
    Unresolved,
}

fn identity(atom: &PropertyAtom) -> Identity<'_> {
    match &atom.ty {
        TypeExpr::Primitive(ty) => Identity::Primitive {
            ty: *ty,
            options: atom.alphanumeric_options(),
            members: atom.constraints.iter().find_map(|constraint| match constraint {
                Constraint::Members(members) => Some(members.iter().map(String::as_str).collect()),
                _ => None,
            }),
            literal: atom.literal_value(),
        },
        TypeExpr::Imported { .. } => Identity::Unresolved,
        ty @ (TypeExpr::InlineObject(_) | TypeExpr::Union(_)) => Identity::Structured(ty),
    }
}

impl PartialEq for Identity<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Primitive { ty, options, members, literal },
                Self::Primitive { ty: other_ty, options: other_options, members: other_members, literal: other_literal },
            ) => ty == other_ty && options == other_options && members == other_members && literal == other_literal,
            (Self::Structured(left), Self::Structured(right)) => left == right,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::grammar::parse_type_expr;

    fn atom(text: &str) -> PropertyAtom {
        parse_type_expr("p", text).unwrap_or_else(|error| panic!("{text}: {error}"))
    }

    #[track_caller]
    fn assert_subtype(a: &str, b: &str, expected: bool) {
        assert_eq!(is_subtype(&atom(a), &atom(b)), expected, "{a} ⊆ {b}");
    }

    #[test]
    fn identical_types_are_subtypes_whatever_the_type() {
        for text in [
            "number",
            "boolean",
            "unknown",
            "null",
            "object",
            "string[]",
            "number[]",
            "alphanumeric(allow-dashes)[]",
            "enum(a, b)",
            "literal(3)",
            "{ a: string }",
            "shell",
            "alphanumeric(allow-slashes)",
        ] {
            assert_subtype(text, text, true);
        }
        assert_subtype("enum(a, b)", "enum(b, a)", true);
        assert_subtype("enum(a, b)", "enum(a)", false);
        assert_subtype("literal(3)", "literal(4)", false);
        assert_subtype("any", "unknown", true);
    }

    #[test]
    fn ordinary_constraints_and_markers_are_ignored() {
        assert_subtype("alphanumeric(min(3); pattern('^a'); required)", "alphanumeric", true);
        assert_subtype("alphanumeric", "alphanumeric(max(1); not-empty)", true);
        assert_subtype("number(min(1); required)", "number(max(2))", true);
        assert_subtype("string(pattern('^a'))", "string(min(9))", true);
        assert_subtype("string[](min(1))", "string(eager)[](max(3))", true);
    }

    #[test]
    fn redundant_and_reordered_options_compare_canonically() {
        assert_subtype("alphanumeric(block-leading-dash)", "alphanumeric", true);
        assert_subtype("alphanumeric", "alphanumeric(block-leading-dash)", true);
        assert_subtype(
            "alphanumeric(allow-periods, allow-dashes)",
            "alphanumeric(allow-dashes, allow-periods)",
            true,
        );
        assert_subtype(
            "alphanumeric(leading-alpha, block-leading-dash, allow-dashes)",
            "alphanumeric(allow-dashes, leading-alpha)",
            true,
        );
    }

    #[test]
    fn alphanumeric_compares_what_the_options_permit() {
        let rule = "alphanumeric(allow-slashes)";
        assert_subtype(rule, rule, true);
        assert_subtype("alphanumeric", rule, true);
        assert_subtype("string", rule, false);
        assert_subtype("email", rule, false);
        assert_subtype(rule, "alphanumeric", false);

        // Implied restrictions count without their keyword.
        assert_subtype("alphanumeric", "alphanumeric(allow-periods, block-double-periods)", true);
        assert_subtype(
            "alphanumeric(leading-alpha, allow-dashes)",
            "alphanumeric(allow-dashes, block-leading-dash)",
            true,
        );
        assert_subtype(
            "alphanumeric(allow-dashes, block-leading-dash)",
            "alphanumeric(leading-alpha, allow-dashes)",
            false,
        );
        // A `..` prohibition must be guaranteed by the narrower type.
        assert_subtype("alphanumeric(allow-periods)", "alphanumeric(allow-periods, block-double-periods)", false);
        assert_subtype("alphanumeric(allow-periods, block-double-periods)", "alphanumeric(allow-periods)", true);
        // First-character sets.
        assert_subtype("alphanumeric(allow-slashes)", "alphanumeric(allow-slashes, block-leading-slash)", false);
        assert_subtype("alphanumeric(allow-slashes, block-leading-slash)", "alphanumeric(allow-slashes)", true);
        assert_subtype("alphanumeric(allow-whitespace)", "alphanumeric(allow-underscores)", false);
    }

    #[test]
    fn always_text_types_are_subtypes_of_string() {
        for text in [
            "alphanumeric",
            "alphanumeric(allow-dashes, allow-periods)",
            "url",
            "enum(a, b)",
            "date",
            "datetime",
            "time",
            "email",
            "shell",
            "expression",
            "file",
            "yaml",
            "json",
            "literal('x')",
            "string",
        ] {
            assert_subtype(text, "string", true);
        }
        for text in [
            "unknown",
            "null",
            "number",
            "numberlike",
            "boolean",
            "boolish",
            "object",
            "literal(3)",
            "string[]",
            "alphanumeric[]",
            "{ a: string }",
            "type-definition",
        ] {
            assert_subtype(text, "string", false);
        }
    }

    #[test]
    fn containers_are_never_promoted_and_other_pairs_are_not_subtypes() {
        assert_subtype("alphanumeric[]", "string[]", false);
        assert_subtype("string", "string[]", false);
        assert_subtype("string", "alphanumeric", false);
        assert_subtype("email", "url", false);
        assert_subtype("shell", "expression", false);
        assert_subtype("string", "email", false);
        assert_subtype("number", "numberlike", false);
        assert_subtype("slug", "slug", false);
        assert_subtype("slug", "string", false);
    }
}
