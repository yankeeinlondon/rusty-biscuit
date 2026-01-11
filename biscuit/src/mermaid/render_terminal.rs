//! Terminal rendering for Mermaid diagrams using mmdc CLI.
//!
//! This module provides functionality to render Mermaid diagrams in the terminal
//! by executing the `mmdc` CLI tool locally and displaying the output with viuer.
//! Falls back to code block rendering on error.
//!
//! ## CLI Detection
//!
//! The module uses a fallback chain for finding the Mermaid CLI:
//! 1. **Direct `mmdc`**: If `mmdc` is in PATH, use it directly
//! 2. **npx fallback**: If `mmdc` is not found but `npx` is available, use `npx mmdc`
//!    with a warning to stderr explaining the temporary installation
//! 3. **Error**: If neither is available, return an error asking the user to install npm

use std::io::Write;
use std::process::Command;
use tempfile::Builder;
use thiserror::Error;

/// Maximum input size for mmdc (10KB should be plenty for diagrams).
///
/// This limit prevents accidentally passing excessively large content to mmdc.
/// Most Mermaid diagrams are well under this size.
const MAX_DIAGRAM_SIZE: usize = 10_000;

/// Checks if a command exists in the system PATH.
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Errors that can occur during terminal rendering of Mermaid diagrams.
#[derive(Error, Debug)]
pub enum MermaidRenderError {
    /// mmdc CLI not found in PATH (and npx fallback not used).
    #[error("mmdc CLI not found. Install with: npm install -g @mermaid-js/mermaid-cli")]
    MmdcNotFound,

    /// npm/npx not found - cannot render mermaid diagrams.
    #[error("npm not found. Install Node.js and npm to render Mermaid diagrams in the terminal")]
    NpmNotFound,

    /// mmdc execution failed.
    #[error("mmdc execution failed (exit code {exit_code}): {stderr}")]
    MmdcExecutionFailed {
        /// The exit code from mmdc
        exit_code: i32,
        /// The stderr output from mmdc
        stderr: String,
    },

    /// Diagram content is too large.
    #[error("Diagram too large ({size} bytes, max {max})")]
    ContentTooLarge {
        /// The actual size of the diagram
        size: usize,
        /// The maximum allowed size
        max: usize,
    },

    /// Failed to display image in terminal.
    #[error("Failed to display image: {0}")]
    DisplayError(String),

    /// IO error during file operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Renders a Mermaid diagram to the terminal using the local mmdc CLI.
///
/// This function:
/// 1. Validates diagram size (< 10KB)
/// 2. Creates temporary input file with diagram instructions
/// 3. Executes mmdc with dark theme and icon pack support
/// 4. Displays the output PNG with viuer
/// 5. Cleans up temporary files
///
/// ## Icon Pack Support
///
/// This function enables the following icon packs via `--iconPacks`:
/// - `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
/// - `@iconify-json/lucide` - Lucide icons
/// - `@iconify-json/carbon` - Carbon Design icons
/// - `@iconify-json/system-uicons` - System UI icons
///
/// Icons can be used in diagrams like: `A[icon:fa7-brands:github]`
///
/// ## Examples
///
/// ```rust,no_run
/// use shared::mermaid::Mermaid;
///
/// fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let diagram = Mermaid::new("flowchart LR\n    A --> B");
///     diagram.render_for_terminal()?;
///     Ok(())
/// }
/// ```
///
/// ## Errors
///
/// Returns `MermaidRenderError` if:
/// - mmdc is not installed or not in PATH
/// - Diagram is too large (> 10KB)
/// - mmdc execution fails (invalid syntax, etc.)
/// - Terminal doesn't support image rendering
#[tracing::instrument(skip(instructions))]
pub fn render_for_terminal(instructions: &str) -> Result<(), MermaidRenderError> {
    // 1. Validate size
    if instructions.len() > MAX_DIAGRAM_SIZE {
        tracing::error!(
            size = instructions.len(),
            max = MAX_DIAGRAM_SIZE,
            "Diagram too large for mmdc"
        );
        return Err(MermaidRenderError::ContentTooLarge {
            size: instructions.len(),
            max: MAX_DIAGRAM_SIZE,
        });
    }

    // 2. Create temp files with tempfile crate (RAII cleanup for input)
    let input_file = Builder::new().suffix(".mmd").tempfile()?;

    tracing::debug!(path = ?input_file.path(), "Created temporary input file");

    // Write instructions to input file
    std::fs::write(input_file.path(), instructions)?;

    // Output path (alongside input, will be cleaned up manually)
    let output_path = input_file.path().with_extension("png");

    tracing::debug!(
        input = ?input_file.path(),
        output = ?output_path,
        "Prepared file paths for mmdc"
    );

    // 3. Determine how to run mmdc (direct or via npx)
    let use_npx = if command_exists("mmdc") {
        tracing::debug!("Found mmdc in PATH, using directly");
        false
    } else if command_exists("npx") {
        tracing::info!("mmdc not found, falling back to npx");
        // Print warning to stderr about temporary installation
        let _ = writeln!(
            std::io::stderr(),
            "- Mermaid diagrams require mmdc to render to the terminal\n\
             - You do not have the Mermaid CLI installed, using npx to install temporarily\n\
             - To install permanently: npm install -g @mermaid-js/mermaid-cli"
        );
        true
    } else {
        tracing::error!("Neither mmdc nor npx found in PATH");
        return Err(MermaidRenderError::NpmNotFound);
    };

    // 4. Build and execute mmdc command
    // Note: The npm package is @mermaid-js/mermaid-cli but the binary is mmdc.
    // When using npx, we use -p to specify the package and then the binary name.
    tracing::info!(use_npx, "Executing mmdc CLI");
    let mut cmd = if use_npx {
        let mut c = Command::new("npx");
        c.args(["-p", "@mermaid-js/mermaid-cli", "mmdc"]);
        c
    } else {
        Command::new("mmdc")
    };

    let output = cmd
        .args(["-i", input_file.path().to_str().unwrap()])
        .args(["-o", output_path.to_str().unwrap()])
        .args(["--theme", "dark"])
        .args([
            "--iconPacks",
            "@iconify-json/fa7-brands",
            "@iconify-json/lucide",
            "@iconify-json/carbon",
            "@iconify-json/system-uicons",
        ])
        .output();

    // 5. Handle errors
    let output = match output {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // This shouldn't happen since we checked above, but handle it anyway
            tracing::error!("Command not found despite prior check");
            return Err(if use_npx {
                MermaidRenderError::NpmNotFound
            } else {
                MermaidRenderError::MmdcNotFound
            });
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to execute mmdc");
            return Err(MermaidRenderError::IoError(e));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        tracing::error!(
            exit_code,
            stderr = %stderr,
            "mmdc execution failed"
        );

        // Clean up output file if it exists
        let _ = std::fs::remove_file(&output_path);

        return Err(MermaidRenderError::MmdcExecutionFailed { exit_code, stderr });
    }

    tracing::debug!(
        exit_code = output.status.code().unwrap_or(0),
        "mmdc execution succeeded"
    );

    // 6. Display with viuer
    let config = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };

    tracing::info!(path = ?output_path, "Displaying diagram in terminal");

    viuer::print_from_file(&output_path, &config)
        .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?;

    tracing::info!("Displayed diagram in terminal");

    // 6. Manual cleanup of output file (input cleaned by tempfile RAII)
    let _ = std::fs::remove_file(&output_path);
    tracing::debug!("Cleaned up temporary output file");

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
/// use shared::mermaid::render_terminal::fallback_code_block;
///
/// let output = fallback_code_block("flowchart LR\n    A --> B");
/// assert!(output.contains("```mermaid"));
/// ```
pub fn fallback_code_block(instructions: &str) -> String {
    format!("```mermaid\n{}\n```", instructions)
}

/// Renders a fallback code block for the given instructions.
///
/// This is used when terminal rendering fails or is not supported.
/// Prints the instructions as a fenced code block to stdout.
///
/// ## Examples
///
/// ```rust
/// use shared::mermaid::render_terminal::render_fallback_code_block;
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
    fn test_content_too_large_error() {
        // Create a diagram that's definitely too large
        let large_instructions = "A".repeat(MAX_DIAGRAM_SIZE + 1);

        let size = large_instructions.len();
        let error = MermaidRenderError::ContentTooLarge {
            size,
            max: MAX_DIAGRAM_SIZE,
        };

        assert_eq!(
            error.to_string(),
            format!("Diagram too large ({} bytes, max {})", size, MAX_DIAGRAM_SIZE)
        );
    }

    #[test]
    fn test_fallback_renders_code_block() {
        // This test just ensures the function doesn't panic
        // We can't easily test stdout in unit tests without capturing it
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
    fn test_command_exists_with_common_command() {
        // 'which' should exist on all Unix systems
        assert!(command_exists("which"));
    }

    #[test]
    fn test_command_exists_with_nonexistent_command() {
        assert!(!command_exists("this_command_definitely_does_not_exist_xyz123"));
    }

    #[test]
    fn test_small_diagram_size() {
        let instructions = "flowchart LR\n    A --> B";
        // Small diagrams should be well under the limit
        assert!(instructions.len() < MAX_DIAGRAM_SIZE);
    }

    #[test]
    fn test_max_diagram_size_constant() {
        // Verify the constant is set to a reasonable value
        assert_eq!(MAX_DIAGRAM_SIZE, 10_000);
    }
}
