//! TypeScript counting rules (tree-sitter-typescript 0.23.2): the
//! JavaScript decisions, with TypeScript's type declarations and namespaces
//! as further boundaries.
//!
//! Zero by design, beyond JavaScript's: a type-level conditional
//! (`conditional_type`, never a `ternary_expression`), `as`, `satisfies` and
//! the non-null `!`.

use super::{Rules, javascript};

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
    decisions: javascript::DECISIONS,
};
