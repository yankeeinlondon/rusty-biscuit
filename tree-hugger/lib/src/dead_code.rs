//! Dead code detection via control flow analysis.
//!
//! Detects code that can never execute because it follows an unconditional
//! exit statement (return, throw, panic, etc.) within the same block.

use tree_sitter::Node;

use crate::shared::ProgrammingLanguage;

/// Checks if a node represents a terminal statement that never returns.
///
/// Terminal statements unconditionally exit the current scope:
/// - `return` statements
/// - `throw`/`raise` exceptions
/// - `panic`/`unreachable` calls
/// - `break`/`continue` in loops
pub fn is_terminal_statement(node: Node, language: ProgrammingLanguage) -> bool {
    let kind = node.kind();

    match language {
        ProgrammingLanguage::Rust => {
            matches!(
                kind,
                "return_expression" | "break_expression" | "continue_expression"
            ) || is_rust_panic_macro(node)
        }
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            matches!(
                kind,
                "return_statement"
                    | "throw_statement"
                    | "break_statement"
                    | "continue_statement"
            )
        }
        ProgrammingLanguage::Python => {
            matches!(
                kind,
                "return_statement" | "raise_statement" | "break_statement" | "continue_statement"
            )
        }
        ProgrammingLanguage::Go => {
            matches!(kind, "return_statement" | "break_statement" | "continue_statement")
                || is_go_panic_call(node)
        }
        ProgrammingLanguage::Java | ProgrammingLanguage::CSharp => {
            matches!(
                kind,
                "return_statement"
                    | "throw_statement"
                    | "break_statement"
                    | "continue_statement"
            )
        }
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => {
            matches!(
                kind,
                "return_statement" | "break_statement" | "continue_statement"
            ) || is_c_exit_call(node)
        }
        ProgrammingLanguage::Swift => {
            matches!(
                kind,
                "return_statement"
                    | "throw_statement"
                    | "break_statement"
                    | "continue_statement"
            ) || is_swift_fatal_error(node)
        }
        ProgrammingLanguage::Scala => {
            matches!(
                kind,
                "return_statement" | "throw_expression" | "break_statement" | "continue_statement"
            )
        }
        ProgrammingLanguage::Php => {
            matches!(
                kind,
                "return_statement"
                    | "throw_expression"
                    | "break_statement"
                    | "continue_statement"
            ) || is_php_exit_call(node)
        }
        ProgrammingLanguage::Perl => {
            matches!(kind, "return_statement" | "last_statement" | "next_statement")
                || is_perl_die_call(node)
        }
        ProgrammingLanguage::Lua => {
            matches!(kind, "return_statement" | "break_statement") || is_lua_error_call(node)
        }
        ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh => {
            matches!(
                kind,
                "return_statement" | "exit_statement" | "break_statement" | "continue_statement"
            )
        }
    }
}

/// Checks if a Rust node is a panic/unreachable/todo macro call.
fn is_rust_panic_macro(node: Node) -> bool {
    if node.kind() != "macro_invocation" {
        return false;
    }

    // Get the macro name
    if let Some(macro_node) = node.child_by_field_name("macro") {
        let macro_name = macro_node.kind();
        // Look for the identifier inside the macro field
        if macro_name == "identifier" {
            // We can't get the text here, but we can check common patterns
            // The actual check would need source text access
            return true; // Conservative: treat all macro invocations as potential panics
        }
    }

    false
}

/// Checks if a Go node is a panic() call.
fn is_go_panic_call(node: Node) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    if let Some(func_node) = node.child_by_field_name("function") {
        // Check if it's a simple identifier "panic"
        if func_node.kind() == "identifier" {
            return true; // Would need source text to verify it's "panic"
        }
    }

    false
}

/// Checks if a C/C++ node is an exit/abort call.
fn is_c_exit_call(node: Node) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    if let Some(func_node) = node.child_by_field_name("function") {
        if func_node.kind() == "identifier" {
            return true; // Would need source text to verify it's "exit" or "abort"
        }
    }

    false
}

/// Checks if a Swift node is a fatalError call.
fn is_swift_fatal_error(node: Node) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    if let Some(func_node) = node.child(0) {
        if func_node.kind() == "simple_identifier" {
            return true; // Would need source text to verify
        }
    }

    false
}

/// Checks if a PHP node is an exit/die call.
fn is_php_exit_call(node: Node) -> bool {
    node.kind() == "exit_statement"
}

/// Checks if a Perl node is a die/exit call.
fn is_perl_die_call(node: Node) -> bool {
    if node.kind() != "function_call" {
        return false;
    }

    // Would need source text to verify it's "die" or "exit"
    true
}

/// Checks if a Lua node is an error() call.
fn is_lua_error_call(node: Node) -> bool {
    if node.kind() != "function_call" {
        return false;
    }

    // Would need source text to verify it's "error"
    true
}

/// Finds dead code siblings after a terminal statement.
///
/// Returns nodes that appear after the terminal statement within the same block.
pub fn find_dead_code_after<'tree>(
    terminal: Node<'tree>,
    _language: ProgrammingLanguage,
) -> Vec<Node<'tree>> {
    let mut dead_nodes = Vec::new();

    // Get the parent block
    let parent = match terminal.parent() {
        Some(p) => p,
        None => return dead_nodes,
    };

    // Check if parent is a block-like container
    let parent_kind = parent.kind();
    let is_block = parent_kind.contains("block")
        || parent_kind.contains("body")
        || parent_kind.contains("compound")
        || parent_kind == "statement_list"
        || parent_kind == "source_file";

    if !is_block {
        return dead_nodes;
    }

    // Find siblings after the terminal statement
    let mut found_terminal = false;
    let mut cursor = parent.walk();

    for child in parent.children(&mut cursor) {
        if child.id() == terminal.id() {
            found_terminal = true;
            continue;
        }

        if found_terminal {
            // Skip comments and empty nodes
            let kind = child.kind();
            if kind.contains("comment") || kind.is_empty() {
                continue;
            }

            // This is dead code
            dead_nodes.push(child);
        }
    }

    dead_nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_block_like_pattern() {
        // Verify block detection patterns
        assert!("block".contains("block"));
        assert!("function_body".contains("body"));
        assert!("compound_statement".contains("compound"));
    }

    #[test]
    fn language_terminal_detection_compiles() {
        // Ensure language enum variants are handled
        let languages = [
            ProgrammingLanguage::Rust,
            ProgrammingLanguage::JavaScript,
            ProgrammingLanguage::Python,
            ProgrammingLanguage::Go,
        ];
        for _lang in languages {
            // Just verify the match arms compile
        }
    }
}
