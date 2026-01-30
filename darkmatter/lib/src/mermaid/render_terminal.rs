//! Terminal rendering for Mermaid diagrams.
//!
//! This module provides thin wrappers around `biscuit_terminal::components::mermaid`
//! for rendering Mermaid diagrams in the terminal. The actual implementation
//! (mmdc CLI execution, viuer display) is handled by biscuit-terminal.
//!
//! ## CLI Detection
//!
//! The underlying implementation uses a fallback chain for finding the Mermaid CLI:
//! 1. **Direct `mmdc`**: If `mmdc` is in PATH, use it directly
//! 2. **npx fallback**: If `mmdc` is not found but `npx` is available, use `npx mmdc`
//!    with a warning to stderr explaining the temporary installation
//! 3. **Error**: If neither is available, return an error asking the user to install npm
//!
//! ## Re-exports
//!
//! This module re-exports the error type from biscuit-terminal for backward compatibility:
//! - [`MermaidRenderError`] - Error type for mermaid terminal rendering

// Re-export the error type from biscuit-terminal for API compatibility
pub use biscuit_terminal::components::mermaid::MermaidRenderError;

use biscuit_terminal::components::mermaid::MermaidRenderer;

/// Renders a Mermaid diagram to the terminal using the local mmdc CLI.
///
/// This function delegates to `biscuit_terminal::components::mermaid::MermaidRenderer`.
///
/// ## Examples
///
/// ```rust,no_run
/// use darkmatter_lib::mermaid::render_terminal::render_for_terminal;
///
/// fn example() -> Result<(), darkmatter_lib::mermaid::MermaidRenderError> {
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
/// - Diagram is too large (> 10KB)
/// - mmdc execution fails (invalid syntax, etc.)
#[tracing::instrument(skip(instructions))]
pub fn render_for_terminal(instructions: &str) -> Result<(), MermaidRenderError> {
    let renderer = MermaidRenderer::new(instructions);
    renderer.render_for_terminal()
}

/// Returns a fallback code block string for the given instructions.
///
/// This is used when terminal rendering fails or is not supported.
/// Returns the instructions formatted as a fenced code block.
///
/// ## Examples
///
/// ```rust
/// use darkmatter_lib::mermaid::render_terminal::fallback_code_block;
///
/// let output = fallback_code_block("flowchart LR\n    A --> B");
/// assert!(output.contains("```mermaid"));
/// ```
pub fn fallback_code_block(instructions: &str) -> String {
    biscuit_terminal::components::mermaid::fallback_code_block(instructions)
}

/// Renders a fallback code block for the given instructions.
///
/// This is used when terminal rendering fails or is not supported.
/// Prints the instructions as a fenced code block to stdout.
///
/// ## Examples
///
/// ```rust
/// use darkmatter_lib::mermaid::render_terminal::render_fallback_code_block;
///
/// render_fallback_code_block("flowchart LR\n    A --> B");
/// ```
pub fn render_fallback_code_block(instructions: &str) {
    biscuit_terminal::components::mermaid::print_fallback_code_block(instructions)
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

    // Error type tests - these verify the re-exported type works correctly
    #[test]
    fn test_error_display_mmdc_not_found() {
        let error = MermaidRenderError::MmdcNotFound;
        assert_eq!(
            error.to_string(),
            "mmdc CLI not found. Install with: npm install -g @mermaid-js/mermaid-cli"
        );
    }

    #[test]
    fn test_error_display_mmdc_execution_failed() {
        let error = MermaidRenderError::MmdcExecutionFailed {
            exit_code: 1,
            stderr: "Invalid syntax".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "mmdc execution failed (exit code 1): Invalid syntax"
        );
    }

    #[test]
    fn test_error_display_display_error() {
        let error = MermaidRenderError::DisplayError("display failed".to_string());
        assert_eq!(error.to_string(), "Failed to display image: display failed");
    }

    #[test]
    fn test_error_display_npm_not_found() {
        let error = MermaidRenderError::NpmNotFound;
        assert_eq!(
            error.to_string(),
            "npm not found. Install Node.js and npm to render Mermaid diagrams in the terminal"
        );
    }

    #[test]
    fn test_error_display_no_image_support() {
        let error = MermaidRenderError::NoImageSupport;
        assert!(error
            .to_string()
            .contains("does not support image rendering"));
    }

    #[test]
    fn test_error_display_content_too_large() {
        let error = MermaidRenderError::ContentTooLarge {
            size: 15000,
            max: 10000,
        };
        assert_eq!(
            error.to_string(),
            "Diagram too large (15000 bytes, max 10000)"
        );
    }
}
