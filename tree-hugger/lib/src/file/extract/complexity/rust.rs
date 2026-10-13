//! Rust counting rules (tree-sitter-rust 0.24.0).
//!
//! Zero by design: plain `else`, `loop`, `?`, `return`/`break`/`continue`,
//! `&`/`|`/`&=`/`|=`, or-patterns (one arm however many alternatives).
//! `async`, labeled, `unsafe` and `const` blocks are part of the function.
//! Type syntax adds nothing: an array type's length (`let x: [u8; if …]`),
//! a cast target, and generic arguments, a turbofish's included.

use super::{Counts, Decision, Rules};

pub(super) static RULES: Rules = Rules {
    boundaries: &[
        "closure_expression",
        "function_item",
        "function_signature_item",
        "struct_item",
        "enum_item",
        "union_item",
        "trait_item",
        "impl_item",
        "mod_item",
        "type_item",
        "macro_definition",
        "foreign_mod_item",
        // Compile-time items, not code that runs on each call.
        "const_item",
        "static_item",
    ],
    // A macro's arguments are tokens: an `if` or `&&` inside is not syntax.
    opaque: &["token_tree"],
    // `_type` minus `macro_invocation` and `metavariable`, which are also
    // values; a macro's tokens are opaque anyway.
    types: &[
        "abstract_type",
        "array_type",
        "bounded_type",
        "dynamic_type",
        "function_type",
        "generic_type",
        "never_type",
        "pointer_type",
        "primitive_type",
        "reference_type",
        "removed_trait_bound",
        "scoped_type_identifier",
        "tuple_type",
        "type_identifier",
        "unit_type",
        // `make::<…>()`: the turbofish of a `generic_function`, whose
        // `function` is the value.
        "type_arguments",
        // `<[u8; N] as Trait>::f`: a type as a path's first segment.
        "bracketed_type",
    ],
    type_sides: &[],
    decisions: &[
        // Else-if is an `else_clause` holding another `if_expression`.
        Decision { kind: "if_expression", counts: Counts::Always },
        Decision { kind: "while_expression", counts: Counts::Always },
        Decision { kind: "for_expression", counts: Counts::Always },
        Decision { kind: "binary_expression", counts: Counts::Operator(&["&&", "||"]) },
        // `if let A = x && b`: the chain's `&&` has no `operator` field.
        Decision { kind: "let_chain", counts: Counts::Tokens(&["&&"]) },
        // `let … else`: a refutable pattern with an early exit.
        Decision { kind: "let_declaration", counts: Counts::WithField("alternative") },
        // A `match` is exhaustive, so its last arm is the default.
        Decision { kind: "match_arm", counts: Counts::UnlessLast },
        // A guard is a separate decision from its arm.
        Decision { kind: "match_pattern", counts: Counts::WithField("condition") },
    ],
};
