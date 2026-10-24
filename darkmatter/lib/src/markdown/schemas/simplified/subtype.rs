//! The passive structural subtype relation among SimplifiedSchema types.
//!
//! [`is_subtype`] answers "does every non-null value the underlying type A
//! accepts also satisfy the underlying type B?" from typed, **resolved** atoms.
//! Only a type and its identity (`alphanumeric` options, `enum` members, a
//! `literal` value) count; ordinary constraints, presence and timing markers,
//! defaults, and descriptions are ignored. The relation is bounded reasoning
//! over the listed types: it never inspects user regexes, converts scalars,
//! reads files, or resolves imports, and it is no substitute for runtime
//! value checks or shell quoting. Because it ignores presence, passing an
//! optional atom to it is not a presence check; a caller that needs
//! nullability compares it separately.
//!
//! A recursive reference ([`TypeExpr::Ref`]) is compared by definition
//! identity and unfolded only through the [`DefinitionTable`] a caller passes
//! to [`is_subtype_with_definitions`]: the comparison is coinductive, so it
//! terminates on self- and mutually recursive definitions.

use std::collections::BTreeSet;

use super::alphanumeric::AlphanumericOptions;
use super::definitions::{DefKey, DefinitionTable};
use super::query::flattened_alternatives;
use super::types::{Constraint, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr};

/// Whether every non-null value accepted by the underlying type of `a` is
/// accepted by the underlying type of `b`.
///
/// - Two identical types (the same type with the same identity) are always
///   subtypes, whatever the type, including arrays and inline objects.
/// - `alphanumeric(X)` ⊆ `alphanumeric(Y)` when X's allowed characters and
///   allowed first characters are subsets of Y's, and X guarantees any
///   prohibition of `..` that Y makes.
/// - Every always-text type is a subtype of `string`.
/// - A `literal` is a subtype of its scalar family (`string`, `number`, or
///   `boolean`), and a string `literal` of an `enum` that has it as a member.
///   A numeric `literal(4)` is not a subtype of `enum('4')`, although the
///   evaluator compares the two equal.
/// - `enum(X)` ⊆ `enum(Y)` when X's members are a subset of Y's.
/// - A non-array union is a subtype when every arm is; a non-array union is a
///   supertype when one of its arms covers the source, so arm order never
///   matters. `unknown` is never a subtype of a concrete type.
/// - Every other pair, including an unresolved reference, is not.
///
/// A recursive reference is compared only by identity here; use
/// [`is_subtype_with_definitions`] to compare it through its definition.
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
/// assert!(is_subtype(&atom("literal(4)"), &atom("number")));
/// assert!(is_subtype(&atom("literal('a')"), &atom("enum(a, b, c)")));
/// ```
pub fn is_subtype(a: &PropertyAtom, b: &PropertyAtom) -> bool {
    is_subtype_with_definitions(a, b, &DefinitionTable::default())
}

/// [`is_subtype`] for resolved atoms whose [`TypeExpr::Ref`]s point into
/// `definitions` (`ResolvedSchema::definitions`, `EffectiveSchema::definitions`).
///
/// Two references to the same definition are the same type whatever their
/// use-site constraints, which are ordinary constraints. Any other pair with
/// a reference on either side compares the definition's body alternatives in
/// its place: every alternative of a source reference must be a subtype, and
/// one alternative of a destination reference must cover the source. The
/// comparison is coinductive: a `(source, destination)` pair met again while
/// its own comparison is in progress holds, so each pair descends into a body
/// at most once on a path and mutual recursion terminates. A reference
/// `definitions` does not hold proves nothing.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
/// use darkmatter::markdown::schemas::{PropertyDef, SimplifiedSchema, is_subtype_with_definitions};
///
/// let dir = std::env::temp_dir();
/// let context = biscuit_file::FileResolutionContext::new(&dir);
/// let yaml = "data: [string, number, '{ items: data[] }']\nvalue: data\ntext: string\n";
/// let yaml = serde_yaml_ng::from_str(yaml).unwrap();
/// let resolved = resolve_yaml_schema(&yaml, &dir, &context).unwrap();
/// let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified else { unreachable!() };
/// let atom = |name: &str| match &shape.properties[name] {
///     PropertyDef::Single(atom) => atom.clone(),
///     PropertyDef::Union(_) => unreachable!(),
/// };
/// assert!(is_subtype_with_definitions(&atom("text"), &atom("value"), &resolved.definitions));
/// assert!(!is_subtype_with_definitions(&atom("value"), &atom("text"), &resolved.definitions));
/// ```
pub fn is_subtype_with_definitions(a: &PropertyAtom, b: &PropertyAtom, definitions: &DefinitionTable) -> bool {
    Comparison::new(definitions).subtype(a, b)
}

/// One subtype question and the reference pairs whose comparison is in
/// progress on the current path.
struct Comparison<'t> {
    definitions: &'t DefinitionTable,
    in_progress: Vec<(Operand, Operand)>,
}

/// One side of a pair in progress: a definition by identity (use-site
/// constraints are ordinary constraints, so they never tell two uses apart),
/// or any other atom by address. Addresses are stable because every atom
/// compared is borrowed from the inputs or from the table for the whole
/// comparison.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operand {
    Definition(*const DefKey),
    Atom(*const PropertyAtom),
}

impl<'t> Comparison<'t> {
    fn new(definitions: &'t DefinitionTable) -> Self {
        Self { definitions, in_progress: Vec::new() }
    }

    fn subtype(&mut self, a: &PropertyAtom, b: &PropertyAtom) -> bool {
        if a.is_array == b.is_array && identity(a) == identity(b) {
            return true;
        }
        if let Some(arms) = scalar_union_arms(a) {
            return arms.iter().all(|arm| self.subtype(arm, b));
        }
        if let Some(key) = scalar_reference(a) {
            return self.unfolding(a, b, |this| match this.body(key) {
                Some(arms) => arms.iter().all(|arm| this.subtype(arm, b)),
                None => false,
            });
        }
        if let Some(arms) = scalar_union_arms(b) {
            return arms.iter().any(|arm| self.subtype(a, arm));
        }
        if let Some(key) = scalar_reference(b) {
            return self.unfolding(a, b, |this| match this.body(key) {
                Some(arms) => arms.iter().any(|arm| this.subtype(a, arm)),
                None => false,
            });
        }
        if a.is_array || b.is_array {
            return false;
        }
        match (&a.ty, &b.ty) {
            (TypeExpr::Primitive(SimplifiedType::Alphanumeric), TypeExpr::Primitive(SimplifiedType::Alphanumeric)) => {
                alphanumeric_subset(options(a), options(b))
            }
            (TypeExpr::Primitive(SimplifiedType::Enum), TypeExpr::Primitive(SimplifiedType::Enum)) => {
                match (members(a), members(b)) {
                    (Some(source), Some(destination)) => source.is_subset(&destination),
                    _ => false,
                }
            }
            (TypeExpr::Primitive(SimplifiedType::Literal), TypeExpr::Primitive(destination)) => {
                literal_within(a.literal_value(), *destination, b)
            }
            (TypeExpr::Primitive(_), TypeExpr::Primitive(SimplifiedType::String)) => is_always_text(a),
            _ => false,
        }
    }

    /// Runs `compare` for the pair `(a, b)` unless that pair is already in
    /// progress, which holds coinductively.
    fn unfolding(&mut self, a: &PropertyAtom, b: &PropertyAtom, compare: impl FnOnce(&mut Self) -> bool) -> bool {
        let pair = (operand(a), operand(b));
        if self.in_progress.contains(&pair) {
            return true;
        }
        self.in_progress.push(pair);
        let holds = compare(self);
        self.in_progress.pop();
        holds
    }

    /// The alternatives a definition's body is written as.
    fn body(&self, key: &DefKey) -> Option<&'t [PropertyAtom]> {
        Some(match &self.definitions.get(key)?.body {
            PropertyDef::Single(atom) => std::slice::from_ref(atom),
            PropertyDef::Union(atoms) => atoms,
        })
    }

    /// `def`'s leaf alternatives with each scalar reference replaced by its
    /// definition's leaves, every definition expanded at most once.
    fn leaves(&self, def: &PropertyDef) -> Vec<PropertyAtom> {
        let mut out = Vec::new();
        let mut expanded = Vec::new();
        self.push_leaves(def, &mut expanded, &mut out);
        out
    }

    fn push_leaves(&self, def: &PropertyDef, expanded: &mut Vec<DefKey>, out: &mut Vec<PropertyAtom>) {
        for alternative in flattened_alternatives(def) {
            let mut atom = alternative.atom.clone();
            atom.is_array |= alternative.is_array;
            let body = scalar_reference(&atom)
                .filter(|key| !expanded.contains(key))
                .and_then(|key| Some((key.clone(), &self.definitions.get(key)?.body)));
            match body {
                Some((key, body)) => {
                    expanded.push(key);
                    self.push_leaves(body, expanded, out);
                }
                None => out.push(atom),
            }
        }
    }
}

fn operand(atom: &PropertyAtom) -> Operand {
    match scalar_reference(atom) {
        Some(key) => Operand::Definition(key),
        None => Operand::Atom(atom),
    }
}

/// The definition a non-array reference names; `K[]` is a list and compares
/// by identity like every other array.
fn scalar_reference(atom: &PropertyAtom) -> Option<&DefKey> {
    match &atom.ty {
        TypeExpr::Ref(key) if !atom.is_array => Some(key),
        _ => None,
    }
}

/// [`is_subtype`] over whole property definitions: every leaf alternative of
/// `a` is covered by some leaf alternative of `b`. Presence is ignored, as it
/// is by [`is_subtype`]; a `null` or `unknown` alternative of `a` is covered
/// only by an identical one.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
/// use darkmatter::markdown::schemas::{PropertyDef, is_definition_subtype};
///
/// let atom = |text| parse_type_expr("p", text).unwrap();
/// let source = PropertyDef::Union(vec![atom("string"), atom("number")]);
/// let destination = PropertyDef::Union(vec![atom("boolean"), atom("number"), atom("string")]);
/// assert!(is_definition_subtype(&source, &destination));
/// assert!(!is_definition_subtype(&destination, &source));
/// ```
pub fn is_definition_subtype(a: &PropertyDef, b: &PropertyDef) -> bool {
    is_definition_subtype_with_definitions(a, b, &DefinitionTable::default())
}

/// [`is_definition_subtype`] for resolved definitions whose
/// [`TypeExpr::Ref`]s point into `definitions`: a reference alternative on
/// either side contributes its definition's leaf alternatives, and the leaves
/// compare by [`is_subtype_with_definitions`].
pub fn is_definition_subtype_with_definitions(a: &PropertyDef, b: &PropertyDef, definitions: &DefinitionTable) -> bool {
    let mut comparison = Comparison::new(definitions);
    let destination = comparison.leaves(b);
    let source = comparison.leaves(a);
    source
        .iter()
        .all(|source| destination.iter().any(|arm| comparison.subtype(source, arm)))
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

/// The arms of a non-array union type; an array of a union compares by
/// identity like every other array.
fn scalar_union_arms(atom: &PropertyAtom) -> Option<&[PropertyAtom]> {
    (!atom.is_array).then(|| atom.union_arms()).flatten()
}

fn members(atom: &PropertyAtom) -> Option<BTreeSet<&str>> {
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Members(members) => Some(members.iter().map(String::as_str).collect()),
        _ => None,
    })
}

/// Whether a `literal` with `value` is accepted by the destination type: its
/// own scalar family, or an `enum` with the same string member.
fn literal_within(value: Option<&serde_json::Value>, destination: SimplifiedType, atom: &PropertyAtom) -> bool {
    match (value, destination) {
        (Some(serde_json::Value::String(_)), SimplifiedType::String)
        | (Some(serde_json::Value::Number(_)), SimplifiedType::Number)
        | (Some(serde_json::Value::Bool(_)), SimplifiedType::Boolean) => true,
        (Some(serde_json::Value::String(text)), SimplifiedType::Enum) => {
            members(atom).is_some_and(|members| members.contains(text.as_str()))
        }
        _ => false,
    }
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
        TypeExpr::InlineObject(_)
        | TypeExpr::Imported { .. }
        | TypeExpr::Union(_)
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_) => false,
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
            members: members(atom),
            literal: atom.literal_value(),
        },
        TypeExpr::Imported { .. } => Identity::Unresolved,
        ty @ (TypeExpr::InlineObject(_)
        | TypeExpr::Union(_)
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_)) => Identity::Structured(ty),
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
    fn literals_are_subtypes_of_their_scalar_family_only() {
        assert_subtype("literal('release')", "string", true);
        assert_subtype("literal(4)", "number", true);
        assert_subtype("literal(4.5)", "number", true);
        assert_subtype("literal(true)", "boolean", true);
        assert_subtype("literal(4)", "string", false);
        assert_subtype("literal('4')", "number", false);
        assert_subtype("literal(true)", "string", false);
        assert_subtype("literal('true')", "boolean", false);
        // No family promotion beyond the literal's own JSON kind.
        assert_subtype("literal(4)", "numberlike", false);
        assert_subtype("literal('x')", "email", false);
        assert_subtype("literal('x')", "alphanumeric", false);
    }

    #[test]
    fn string_literals_are_subtypes_of_enums_that_list_them() {
        assert_subtype("literal('a')", "enum(a, b, c)", true);
        assert_subtype("literal('z')", "enum(a, b, c)", false);
        // The evaluator compares `4 == '4'` equal; the structure differs.
        assert_subtype("literal(4)", "enum('4')", false);
        assert_subtype("literal(true)", "enum('true')", false);
    }

    #[test]
    fn enum_subsets_are_subtypes() {
        assert_subtype("enum(a, b)", "enum(a, b, c)", true);
        assert_subtype("enum(b, a)", "enum(c, a, b)", true);
        assert_subtype("enum(a, d)", "enum(a, b, c)", false);
        assert_subtype("enum(a, b, c)", "enum(a, b)", false);
        assert_subtype("enum(a)[]", "enum(a, b)[]", false);
    }

    fn union(arms: &[&str]) -> PropertyAtom {
        let mut atom = PropertyAtom::bare(SimplifiedType::Unknown);
        atom.ty = TypeExpr::Union(arms.iter().map(|text| self::atom(text)).collect());
        atom
    }

    #[test]
    fn unions_compare_arm_by_arm_independent_of_order() {
        let literals = union(&["literal('a')", "literal('b')"]);
        assert!(is_subtype(&literals, &atom("enum(a, b, c)")));
        assert!(!is_subtype(&union(&["literal('a')", "literal('z')"]), &atom("enum(a, b, c)")));

        let source = union(&["string", "number"]);
        assert!(is_subtype(&source, &union(&["string", "number", "boolean"])));
        assert!(is_subtype(&source, &union(&["boolean", "number", "string"])));
        assert!(is_subtype(&union(&["number", "string"]), &source));
        assert!(!is_subtype(&union(&["string", "number", "boolean"]), &source));
        assert!(is_subtype(&atom("email"), &union(&["number", "string"])));
        assert!(!is_subtype(&atom("number"), &union(&["string", "boolean"])));
    }

    #[test]
    fn unknown_is_never_proof_of_a_concrete_type() {
        assert_subtype("unknown", "string", false);
        assert_subtype("unknown", "number", false);
        assert!(!is_subtype(&atom("unknown"), &union(&["string", "number", "boolean"])));
        assert!(!is_subtype(&union(&["string", "unknown"]), &atom("string")));
    }

    #[test]
    fn definitions_compare_every_leaf_alternative() {
        let def = |arms: &[&str]| match arms {
            [single] => PropertyDef::Single(atom(single)),
            arms => PropertyDef::Union(arms.iter().map(|text| atom(text)).collect()),
        };
        assert!(is_definition_subtype(&def(&["literal('a')", "literal('b')"]), &def(&["enum(a, b, c)"])));
        assert!(is_definition_subtype(&def(&["string", "number"]), &def(&["boolean", "number", "string"])));
        assert!(!is_definition_subtype(&def(&["string", "null"]), &def(&["string"])));
        assert!(is_definition_subtype(&def(&["string", "null"]), &def(&["null", "string"])));
        assert!(!is_definition_subtype(&def(&["string[]"]), &def(&["string"])));
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
