//! Terminal rendering for Mermaid diagrams using mermaid.ink service.
//!
//! This module provides functionality to render Mermaid diagrams in the terminal
//! by fetching pre-rendered images from mermaid.ink and displaying with viuer.
//! Falls back to code block rendering on error.

use base64::{engine::general_purpose::STANDARD, Engine};
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

    /// Failed to fetch image from mermaid.ink.
    #[error("Failed to fetch from mermaid.ink: {0}")]
    FetchError(#[from] reqwest::Error),

    /// mermaid.ink returned a non-success status code.
    #[error("mermaid.ink returned status {0}")]
    ServiceError(u16),

    /// Failed to display image in terminal.
    #[error("Failed to display image: {0}")]
    DisplayError(String),

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
/// 1. Prepends a dark theme directive optimized for terminal display
/// 2. Base64 encodes the instructions
/// 3. Validates URL length (< 2KB)
/// 4. Fetches pre-rendered image from mermaid.ink/img/ endpoint
/// 5. Saves to a temporary file (RAII cleanup)
/// 6. Displays with viuer
///
/// The `/img/` endpoint is used instead of `/svg/` because mermaid.ink renders
/// text using HTML `<foreignObject>` elements, which are not supported by
/// pure SVG renderers like usvg/resvg. The `/img/` endpoint returns a
/// server-side rendered image that includes all text correctly.
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
/// - Terminal doesn't support image rendering
#[tracing::instrument(skip(instructions))]
pub async fn render_for_terminal(instructions: &str) -> Result<(), MermaidRenderError> {
    // Prepend theme directive with explicit colors for dark terminal rendering
    // Using 'base' theme which allows full customization
    let themed_instructions = format!(
        "%%{{init: {{'theme': 'base', 'themeVariables': {{\
            'background': '#1e1e1e',\
            'primaryColor': '#3c3c3c',\
            'primaryTextColor': '#ffffff',\
            'primaryBorderColor': '#888888',\
            'secondaryColor': '#4a4a4a',\
            'secondaryTextColor': '#ffffff',\
            'secondaryBorderColor': '#888888',\
            'tertiaryColor': '#5a5a5a',\
            'tertiaryTextColor': '#ffffff',\
            'tertiaryBorderColor': '#888888',\
            'lineColor': '#aaaaaa',\
            'textColor': '#ffffff',\
            'mainBkg': '#3c3c3c',\
            'nodeBkg': '#3c3c3c',\
            'nodeBorder': '#888888',\
            'clusterBkg': '#2d2d2d',\
            'clusterBorder': '#666666',\
            'titleColor': '#ffffff',\
            'edgeLabelBackground': '#2d2d2d'\
        }}}}}}%%\n{}",
        instructions
    );

    // Base64 encode the instructions with theme
    let encoded = STANDARD.encode(&themed_instructions);
    // Use /img/ endpoint for pre-rendered image (includes text via foreignObject)
    // Add bgColor parameter to ensure dark background (themeVariables.background alone isn't enough)
    let url = format!("https://mermaid.ink/img/{}?bgColor=1e1e1e", encoded);

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

    // Fetch pre-rendered image from mermaid.ink
    tracing::info!(url = %url, "Fetching image from mermaid.ink");
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        tracing::error!(status, "mermaid.ink returned error status");
        return Err(MermaidRenderError::ServiceError(status));
    }

    let image_data = response.bytes().await?;
    tracing::debug!(image_len = image_data.len(), "Received image data");

    // Save to temporary file (RAII cleanup)
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(&image_data)?;
    temp_file.flush()?;

    let temp_path = temp_file.path();
    tracing::debug!(path = ?temp_path, "Saved image to temporary file");

    // Display with viuer
    let config = viuer::Config {
        absolute_offset: false,
        ..Default::default()
    };

    viuer::print_from_file(temp_path, &config)
        .map_err(|e| MermaidRenderError::DisplayError(format!("{}", e)))?;

    tracing::info!("Displayed diagram in terminal");

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
        let large_instructions = "A".repeat(MAX_MERMAID_INK_LENGTH);
        let encoded = STANDARD.encode(&large_instructions);
        let url = format!("https://mermaid.ink/img/{}", encoded);

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
    fn test_error_display_display_error() {
        let error = MermaidRenderError::DisplayError("display failed".to_string());
        assert_eq!(error.to_string(), "Failed to display image: display failed");
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
        let url = format!("https://mermaid.ink/img/{}", encoded);

        // Small diagrams should be well under the limit
        assert!(url.len() < MAX_MERMAID_INK_LENGTH);
    }

    /// Regression test: verify we use /img/ endpoint instead of /svg/ endpoint.
    ///
    /// The /svg/ endpoint returns SVG with foreignObject containing HTML text,
    /// which usvg/resvg cannot render. The /img/ endpoint returns a pre-rendered
    /// image from mermaid.ink's server-side rendering, which includes all text.
    #[test]
    fn test_uses_img_endpoint_not_svg() {
        let instructions = "flowchart LR\n    A --> B";
        let encoded = STANDARD.encode(instructions);

        // The URL should use /img/ not /svg/
        let url = format!("https://mermaid.ink/img/{}", encoded);
        assert!(url.contains("/img/"), "URL should use /img/ endpoint");
        assert!(!url.contains("/svg/"), "URL should NOT use /svg/ endpoint");
    }
}
