//! Rust counting rules (tree-sitter-rust 0.24.0).
//!
//! Zero by design: plain `else`, `loop`, `?`, `return`/`break`/`continue`,
//! `&`/`|`/`&=`/`|=`, or-patterns (one arm however many alternatives).
//! `async`, labeled, `unsafe` and `const` blocks are part of the function.

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
