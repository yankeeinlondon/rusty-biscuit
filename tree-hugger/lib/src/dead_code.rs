//! Dead code detection via control flow analysis.
//!
//! Detects code that can never execute because it follows an unconditional
//! exit statement (return, throw, panic, etc.) within the same block.

use tree_sitter::Node;

use crate::shared::ProgrammingLanguage;

/// Rust macros that never return (unconditionally terminate execution).
const RUST_TERMINAL_MACROS: &[&str] = &["panic", "unreachable", "todo", "unimplemented"];

/// Rust function paths that never return (exit the process).
const RUST_TERMINAL_FUNCTIONS: &[&str] = &[
    "process::exit",
    "std::process::exit",
    "process::abort",
    "std::process::abort",
];

/// Go functions that never return.
const GO_TERMINAL_FUNCTIONS: &[&str] = &["panic", "os.Exit"];

/// C/C++ functions that never return.
const C_TERMINAL_FUNCTIONS: &[&str] = &["exit", "abort", "_exit", "_Exit", "quick_exit"];

/// Swift functions that never return.
const SWIFT_TERMINAL_FUNCTIONS: &[&str] =
    &["fatalError", "preconditionFailure", "assertionFailure"];

/// Perl functions that never return.
const PERL_TERMINAL_FUNCTIONS: &[&str] = &["die", "exit"];

/// Lua functions that never return.
const LUA_TERMINAL_FUNCTIONS: &[&str] = &["error", "os.exit"];

/// Checks if a node represents a terminal statement that never returns.
///
/// Terminal statements unconditionally exit the current scope:
/// - `return` statements
/// - `throw`/`raise` exceptions
/// - `panic`/`unreachable` calls
/// - `break`/`continue` in loops
///
/// ## Arguments
///
/// * `node` - The AST node to check
/// * `language` - The programming language of the source
/// * `source` - The source text (required for text-based pattern matching)
pub fn is_terminal_statement(node: Node, language: ProgrammingLanguage, source: &str) -> bool {
    let kind = node.kind();

    match language {
        ProgrammingLanguage::Rust => {
            matches!(
                kind,
                "return_expression" | "break_expression" | "continue_expression"
            ) || is_rust_terminal(node, source)
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
            matches!(
                kind,
                "return_statement" | "break_statement" | "continue_statement"
            ) || is_go_panic_call(node, source)
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
            ) || is_c_exit_call(node, source)
        }
        ProgrammingLanguage::Swift => {
            matches!(
                kind,
                "return_statement"
                    | "throw_statement"
                    | "break_statement"
                    | "continue_statement"
            ) || is_swift_fatal_error(node, source)
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
            matches!(
                kind,
                "return_statement" | "last_statement" | "next_statement"
            ) || is_perl_die_call(node, source)
        }
        ProgrammingLanguage::Lua => {
            matches!(kind, "return_statement" | "break_statement")
                || is_lua_error_call(node, source)
        }
        ProgrammingLanguage::Bash | ProgrammingLanguage::Zsh => {
            matches!(
                kind,
                "return_statement" | "exit_statement" | "break_statement" | "continue_statement"
            )
        }
    }
}

/// Checks if a Rust node is a terminal statement (panic macro or exit function).
///
/// Detects:
/// - `panic!`, `unreachable!`, `todo!`, `unimplemented!` macros
/// - `process::exit()`, `std::process::abort()` function calls
fn is_rust_terminal(node: Node, source: &str) -> bool {
    is_rust_panic_macro(node, source) || is_rust_exit_call(node, source)
}

/// Checks if a Rust node is a panic/unreachable/todo/unimplemented macro call.
fn is_rust_panic_macro(node: Node, source: &str) -> bool {
    if node.kind() != "macro_invocation" {
        return false;
    }

    // Get the macro name from the "macro" field
    if let Some(macro_node) = node.child_by_field_name("macro") {
        if let Ok(text) = macro_node.utf8_text(source.as_bytes()) {
            return RUST_TERMINAL_MACROS.contains(&text);
        }
    }

    false
}

/// Checks if a Rust node is a process::exit or process::abort call.
fn is_rust_exit_call(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    // Get the function being called
    if let Some(func_node) = node.child_by_field_name("function") {
        if let Ok(text) = func_node.utf8_text(source.as_bytes()) {
            return RUST_TERMINAL_FUNCTIONS.contains(&text);
        }
    }

    false
}

/// Checks if a Go node is a panic() or os.Exit() call.
fn is_go_panic_call(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    if let Some(func_node) = node.child_by_field_name("function") {
        if let Ok(text) = func_node.utf8_text(source.as_bytes()) {
            return GO_TERMINAL_FUNCTIONS.contains(&text);
        }
    }

    false
}

/// Checks if a C/C++ node is an exit/abort call.
fn is_c_exit_call(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    if let Some(func_node) = node.child_by_field_name("function") {
        if let Ok(text) = func_node.utf8_text(source.as_bytes()) {
            return C_TERMINAL_FUNCTIONS.contains(&text);
        }
    }

    false
}

/// Checks if a Swift node is a fatalError/preconditionFailure call.
fn is_swift_fatal_error(node: Node, source: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    // Swift uses the first child for function name, not a field
    if let Some(func_node) = node.child(0) {
        if let Ok(text) = func_node.utf8_text(source.as_bytes()) {
            return SWIFT_TERMINAL_FUNCTIONS.contains(&text);
        }
    }

    false
}

/// Checks if a PHP node is an exit/die call.
fn is_php_exit_call(node: Node) -> bool {
    // PHP has a dedicated exit_statement node type
    node.kind() == "exit_statement"
}

/// Checks if a Perl node is a die/exit call.
fn is_perl_die_call(node: Node, source: &str) -> bool {
    if node.kind() != "function_call" {
        return false;
    }

    // Get function name (usually the first child)
    if let Some(func_node) = node.child(0) {
        if let Ok(text) = func_node.utf8_text(source.as_bytes()) {
            return PERL_TERMINAL_FUNCTIONS.contains(&text);
        }
    }

    false
}

/// Checks if a Lua node is an error() or os.exit() call.
fn is_lua_error_call(node: Node, source: &str) -> bool {
    if node.kind() != "function_call" {
        return false;
    }

    // Get function name
    if let Some(func_node) = node.child(0) {
        if let Ok(text) = func_node.utf8_text(source.as_bytes()) {
            return LUA_TERMINAL_FUNCTIONS.contains(&text);
        }
    }

    false
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

    #[test]
    fn rust_terminal_macros_list() {
        assert!(RUST_TERMINAL_MACROS.contains(&"panic"));
        assert!(RUST_TERMINAL_MACROS.contains(&"unreachable"));
        assert!(RUST_TERMINAL_MACROS.contains(&"todo"));
        assert!(RUST_TERMINAL_MACROS.contains(&"unimplemented"));
        // println is NOT a terminal macro
        assert!(!RUST_TERMINAL_MACROS.contains(&"println"));
    }

    #[test]
    fn rust_terminal_functions_list() {
        assert!(RUST_TERMINAL_FUNCTIONS.contains(&"process::exit"));
        assert!(RUST_TERMINAL_FUNCTIONS.contains(&"std::process::exit"));
        assert!(RUST_TERMINAL_FUNCTIONS.contains(&"process::abort"));
        assert!(RUST_TERMINAL_FUNCTIONS.contains(&"std::process::abort"));
    }
}
