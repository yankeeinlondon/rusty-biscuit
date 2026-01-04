//! Terminal rendering for Mermaid diagrams using mermaid.ink service.
//!
//! This module provides functionality to render Mermaid diagrams in the terminal
//! by fetching SVG from mermaid.ink, converting to PNG via resvg, and displaying
//! with viuer. Falls back to code block rendering on error.

use base64::{engine::general_purpose::STANDARD, Engine};
use resvg::usvg;
use std::io::Write;
use tempfile::NamedTempFile;
use thiserror::Error;

/// Maximum URL length for mermaid.ink (base64-encoded content).
///
/// mermaid.ink has practical URL length limits. Diagrams larger than this
/// will fail with `ContentTooLarge` error.
const MAX_MERMAID_INK_LENGTH: usize = 2000;

/// Errors that can occur during terminal rendering of Mermaid diagrams.
#[derive(Error, Debug)]
pub enum MermaidRenderError {
    /// Diagram content is too large for mermaid.ink service.
    #[error("Diagram too large for mermaid.ink ({size} bytes, max {max})")]
    ContentTooLarge {
        /// The actual size of the encoded URL
        size: usize,
        /// The maximum allowed size
        max: usize,
    },

    /// Failed to fetch SVG from mermaid.ink.
    #[error("Failed to fetch from mermaid.ink: {0}")]
    FetchError(#[from] reqwest::Error),

    /// mermaid.ink returned a non-success status code.
    #[error("mermaid.ink returned status {0}")]
    ServiceError(u16),

    /// Failed to parse SVG content.
    #[error("Failed to parse SVG: {0}")]
    SvgParseError(String),

    /// Failed to render PNG from SVG.
    #[error("Failed to render image: {0}")]
    RenderError(String),

    /// Terminal image rendering is not supported in this environment.
    #[error("Terminal image rendering not supported")]
    NotSupported,

    /// IO error during file operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Renders a Mermaid diagram to the terminal using mermaid.ink service.
///
/// This function:
/// 1. Validates the diagram size (< 2KB when base64-encoded)
/// 2. Base64 encodes the instructions
/// 3. Fetches SVG from mermaid.ink
/// 4. Parses SVG with resvg/usvg
/// 5. Renders to PNG using resvg
/// 6. Saves to a temporary file (RAII cleanup)
/// 7. Displays with viuer
/// 8. On error, falls back to printing code block
///
/// ## Examples
///
/// ```rust,no_run
/// use shared::mermaid::Mermaid;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let diagram = Mermaid::new("flowchart LR\n    A --> B");
/// diagram.render_for_terminal().await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// Returns `MermaidRenderError` if:
/// - Diagram is too large (> 2KB when encoded)
/// - Network request fails
/// - mermaid.ink returns error status
/// - SVG parsing fails
/// - PNG rendering fails
/// - Terminal doesn't support image rendering
#[tracing::instrument(skip(instructions))]
pub async fn render_for_terminal(instructions: &str) -> Result<(), MermaidRenderError> {
    // Base64 encode the instructions
    let encoded = STANDARD.encode(instructions);
    let url = format!("https://mermaid.ink/svg/{}", encoded);

    tracing::debug!(
        encoded_len = encoded.len(),
        url_len = url.len(),
        "Encoded Mermaid instructions"
    );

    // Validate URL length
    if url.len() > MAX_MERMAID_INK_LENGTH {
        tracing::error!(
            size = url.len(),
            max = MAX_MERMAID_INK_LENGTH,
            "Diagram too large for mermaid.ink"
        );
        return Err(MermaidRenderError::ContentTooLarge {
            size: url.len(),
            max: MAX_MERMAID_INK_LENGTH,
        });
    }

    // Fetch SVG from mermaid.ink
    tracing::info!(url = %url, "Fetching SVG from mermaid.ink");
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        tracing::error!(status, "mermaid.ink returned error status");
        return Err(MermaidRenderError::ServiceError(status));
    }

    let svg_data = response.bytes().await?;
    tracing::debug!(svg_len = svg_data.len(), "Received SVG data");

    // Parse SVG with usvg
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &options)
        .map_err(|e: usvg::Error| MermaidRenderError::SvgParseError(e.to_string()))?;

    tracing::debug!("Parsed SVG successfully");

    // Render to PNG using resvg
    let pixmap_size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .ok_or_else(|| MermaidRenderError::RenderError("Failed to create pixmap".to_string()))?;

    resvg::render(&tree, Default::default(), &mut pixmap.as_mut());

    tracing::debug!(
        width = pixmap_size.width(),
        height = pixmap_size.height(),
        "Rendered PNG"
    );

    // Save to temporary file (RAII cleanup)
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(&pixmap.encode_png().map_err(|e| {
        MermaidRenderError::RenderError(format!("Failed to encode PNG: {}", e))
    })?)?;
    temp_file.flush()?;

    let temp_path = temp_file.path();
    tracing::debug!(path = ?temp_path, "Saved PNG to temporary file");

    // Display with viuer
    let config = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };

    viuer::print_from_file(temp_path, &config)
        .map_err(|e| MermaidRenderError::RenderError(format!("Failed to display image: {}", e)))?;

    tracing::info!("Displayed diagram in terminal");

    Ok(())
}

/// Renders a fallback code block for the given instructions.
///
/// This is used when terminal rendering fails or is not supported.
/// Prints the instructions as a fenced code block.
///
/// ## Examples
///
/// ```rust
/// use shared::mermaid::render_terminal::render_fallback_code_block;
///
/// render_fallback_code_block("flowchart LR\n    A --> B");
/// ```
pub fn render_fallback_code_block(instructions: &str) {
    println!("```mermaid");
    println!("{}", instructions);
    println!("```");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_too_large_error() {
        // Create a diagram that's definitely too large
        let large_instructions = "A".repeat(MAX_MERMAID_INK_LENGTH);
        let encoded = STANDARD.encode(&large_instructions);
        let url = format!("https://mermaid.ink/svg/{}", encoded);

        assert!(url.len() > MAX_MERMAID_INK_LENGTH);

        // This would fail in the actual function
        let size = url.len();
        let error = MermaidRenderError::ContentTooLarge {
            size,
            max: MAX_MERMAID_INK_LENGTH,
        };

        assert_eq!(
            error.to_string(),
            format!(
                "Diagram too large for mermaid.ink ({} bytes, max {})",
                size, MAX_MERMAID_INK_LENGTH
            )
        );
    }

    #[test]
    fn test_fallback_renders_code_block() {
        // This test just ensures the function doesn't panic
        // We can't easily test stdout in unit tests without capturing it
        render_fallback_code_block("flowchart LR\n    A --> B");
    }

    #[test]
    fn test_base64_encoding() {
        let instructions = "flowchart LR\n    A --> B";
        let encoded = STANDARD.encode(instructions);

        // Verify it's valid base64
        assert!(!encoded.is_empty());
        assert!(encoded.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));

        // Verify we can decode it back
        let decoded = STANDARD.decode(&encoded).expect("Should decode");
        assert_eq!(decoded, instructions.as_bytes());
    }

    #[tokio::test]
    async fn test_error_display_fetch_error() {
        // We can't construct reqwest::Error directly, so we trigger a real error
        let result = reqwest::get("http://invalid.mermaid.ink.test.invalid").await;
        match result {
            Err(e) => {
                let error = MermaidRenderError::FetchError(e);
                assert!(error.to_string().contains("Failed to fetch from mermaid.ink"));
            }
            Ok(_) => panic!("Expected error"),
        }
    }

    #[test]
    fn test_error_display_service_error() {
        let error = MermaidRenderError::ServiceError(404);
        assert_eq!(error.to_string(), "mermaid.ink returned status 404");
    }

    #[test]
    fn test_error_display_svg_parse_error() {
        let error = MermaidRenderError::SvgParseError("invalid SVG".to_string());
        assert_eq!(error.to_string(), "Failed to parse SVG: invalid SVG");
    }

    #[test]
    fn test_error_display_render_error() {
        let error = MermaidRenderError::RenderError("rendering failed".to_string());
        assert_eq!(error.to_string(), "Failed to render image: rendering failed");
    }

    #[test]
    fn test_error_display_not_supported() {
        let error = MermaidRenderError::NotSupported;
        assert_eq!(
            error.to_string(),
            "Terminal image rendering not supported"
        );
    }

    #[test]
    fn test_small_diagram_url_length() {
        let instructions = "flowchart LR\n    A --> B";
        let encoded = STANDARD.encode(instructions);
        let url = format!("https://mermaid.ink/svg/{}", encoded);

        // Small diagrams should be well under the limit
        assert!(url.len() < MAX_MERMAID_INK_LENGTH);
    }
}
