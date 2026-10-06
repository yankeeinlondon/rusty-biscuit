//! JavaScript counting rules (tree-sitter-javascript 0.25.0). TypeScript
//! shares [`DECISIONS`].
//!
//! Zero by design: plain `else`, `default`, `finally`, `for (;;)`, `?.`,
//! `return`/`break`/`continue`/`throw`, compound assignments other than
//! `&&=`/`||=`/`??=`. Object literal values run inline and are counted; the
//! literal's methods, getters and setters are boundaries.

use super::{Counts, Decision, Rules};

pub(super) static RULES: Rules = Rules {
    boundaries: &[
        "function_declaration",
        "generator_function_declaration",
        "function_expression",
        "generator_function",
        "arrow_function",
        "method_definition",
        // A class's field initializers and static blocks are not the call.
        "class_declaration",
        "class",
    ],
    opaque: &[],
    decisions: DECISIONS,
};

pub(super) const DECISIONS: &[Decision] = &[
    // Else-if is an `else_clause` holding another `if_statement`.
    Decision { kind: "if_statement", counts: Counts::Always },
    // `for (;;)` has `condition: empty_statement` and never tests anything.
    Decision {
        kind: "for_statement",
        counts: Counts::UnlessFieldKind { field: "condition", kind: "empty_statement" },
    },
    Decision { kind: "for_in_statement", counts: Counts::Always },
    Decision { kind: "while_statement", counts: Counts::Always },
    Decision { kind: "do_statement", counts: Counts::Always },
    Decision { kind: "ternary_expression", counts: Counts::Always },
    Decision { kind: "binary_expression", counts: Counts::Operator(&["&&", "||", "??"]) },
    Decision {
        kind: "augmented_assignment_expression",
        counts: Counts::Operator(&["&&=", "||=", "??="]),
    },
    // `case 1: case 2: …` is one arm: only the last label of a group has a
    // `body`. `switch_default` is not counted.
    Decision { kind: "switch_case", counts: Counts::Arm("body") },
    Decision { kind: "catch_clause", counts: Counts::Always },
];
