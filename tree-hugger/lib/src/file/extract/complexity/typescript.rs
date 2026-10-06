//! TypeScript counting rules (tree-sitter-typescript 0.23.2): the
//! JavaScript decisions, with TypeScript's type declarations and namespaces
//! as further boundaries.
//!
//! Zero by design, beyond JavaScript's: `as`, `satisfies`, the non-null `!`,
//! and all type syntax in the body: annotations, type arguments, assertion
//! and `satisfies` targets, `typeof` queries and type-level conditionals,
//! so `let x: { [a ? 1 : 2]: string }` adds nothing.

use super::{Rules, TypeSide, javascript};

pub(super) static RULES: Rules = Rules {
    boundaries: &[
        "function_declaration",
        "generator_function_declaration",
        "function_expression",
        "generator_function",
        "arrow_function",
        "method_definition",
        "function_signature",
        "method_signature",
        "abstract_method_signature",
        "class_declaration",
        "abstract_class_declaration",
        "class",
        "interface_declaration",
        "type_alias_declaration",
        "enum_declaration",
        // A namespace declared in a body is a declaration of its own.
        "internal_module",
        "module",
    ],
    opaque: &[],
    // The `type` and `primary_type` supertypes minus `call_expression`,
    // `member_expression` and the `const` of `as const`, which are also
    // values (each covered by an annotation or a type side).
    types: &[
        "array_type",
        "conditional_type",
        "constructor_type",
        "existential_type",
        "flow_maybe_type",
        "function_type",
        "generic_type",
        "index_type_query",
        "infer_type",
        "intersection_type",
        "literal_type",
        "lookup_type",
        "nested_type_identifier",
        "object_type",
        "parenthesized_type",
        "predefined_type",
        "readonly_type",
        "template_literal_type",
        "this_type",
        "tuple_type",
        "type_identifier",
        "type_query",
        "union_type",
        // `let x: T`, `catch (e: T)`; `x is T`, `asserts x`; index signature
        // modifiers.
        "type_annotation",
        "type_predicate_annotation",
        "asserts_annotation",
        "adding_type_annotation",
        "omitting_type_annotation",
        "opting_type_annotation",
        // `f<T>()`, `new C<T>()`, `f<T>`, `<T>x`, `<C<T> />`.
        "type_arguments",
    ],
    type_sides: &[
        TypeSide { kind: "as_expression", token: "as" },
        TypeSide { kind: "satisfies_expression", token: "satisfies" },
    ],
    decisions: javascript::DECISIONS,
};
