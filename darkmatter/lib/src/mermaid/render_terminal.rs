//! Terminal rendering for Mermaid diagrams.
//!
//! This module provides thin wrappers around `biscuit_terminal::components::mermaid`
//! for rendering Mermaid diagrams in the terminal. The actual implementation
//! (mmdc CLI execution, viuer display) is handled by biscuit-terminal.
//!
//! ## Re-exports
//!
//! This module re-exports the error type from biscuit-terminal:
//! - [`MermaidRenderError`] - Error type for mermaid terminal rendering

// Re-export the error type from biscuit-terminal for API compatibility
pub use biscuit_terminal::components::mermaid::MermaidRenderError;

use biscuit_terminal::components::mermaid::MermaidDiagram;

/// Renders a Mermaid diagram to the terminal.
///
/// Uses [`MermaidDiagram::try_render()`] for proper error reporting.
///
/// ## Examples
///
/// ```rust,no_run
/// use darkmatter::mermaid::render_terminal::render_for_terminal;
///
/// fn example() -> Result<(), darkmatter::mermaid::MermaidRenderError> {
///     render_for_terminal("flowchart LR\n    A --> B")?;
///     Ok(())
/// }
/// ```
///
/// ## Errors
///
/// Returns `MermaidRenderError` if:
/// - Terminal doesn't support image rendering
/// - mmdc is not installed or not in PATH
/// - mmdc execution fails (invalid syntax, etc.)
#[tracing::instrument(skip(instructions))]
pub fn render_for_terminal(instructions: &str) -> Result<(), MermaidRenderError> {
    let diagram = MermaidDiagram::new(instructions);
    let term = biscuit_terminal::terminal::Terminal::new();
    let result = diagram.try_render(&term)?;
    print!("{}", result.output);
    Ok(())
}

/// Returns a fallback code block string for the given instructions.
///
/// This is used when terminal rendering fails or is not supported.
/// Returns the instructions formatted as a fenced code block.
///
/// ## Examples
///
/// ```rust
/// use darkmatter::mermaid::render_terminal::fallback_code_block;
///
/// let output = fallback_code_block("flowchart LR\n    A --> B");
/// assert!(output.contains("```mermaid"));
/// ```
pub fn fallback_code_block(instructions: &str) -> String {
    MermaidDiagram::new(instructions).fallback_code_block()
}

/// Renders a fallback code block for the given instructions.
///
/// This is used when terminal rendering fails or is not supported.
/// Prints the instructions as a fenced code block to stdout.
///
/// ## Examples
///
/// ```rust
/// use darkmatter::mermaid::render_terminal::render_fallback_code_block;
///
/// render_fallback_code_block("flowchart LR\n    A --> B");
/// ```
pub fn render_fallback_code_block(instructions: &str) {
    println!("{}", fallback_code_block(instructions));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_renders_code_block() {
        // This test just ensures the function doesn't panic
        render_fallback_code_block("flowchart LR\n    A --> B");
    }

    #[test]
    fn test_fallback_code_block_format() {
        let instructions = "flowchart LR\n    A --> B";
        let output = fallback_code_block(instructions);

        assert!(output.starts_with("```mermaid\n"));
        assert!(output.ends_with("\n```"));
        assert!(output.contains(instructions));
    }

    // Error type tests - verify the re-exported type works correctly
    #[test]
    fn test_error_display_no_image_support() {
        let error = MermaidRenderError::NoImageSupport;
        assert!(error.to_string().contains("does not support"));
    }

    #[test]
    fn test_error_display_display_error() {
        let error = MermaidRenderError::DisplayError("display failed".to_string());
        assert!(error.to_string().contains("display failed"));
    }
}
