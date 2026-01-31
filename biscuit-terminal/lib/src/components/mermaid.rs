//! Mermaid diagram rendering for terminals.
//!
//! This module provides functionality to render Mermaid diagrams in the terminal
//! by executing the `mmdc` CLI tool locally and displaying the output with viuer.
//! Falls back to code block rendering when image rendering is not supported.
//!
//! ## CLI Detection
//!
//! The module uses a fallback chain for finding the Mermaid CLI:
//!
//! 1. **Direct `mmdc`**: If `mmdc` is in PATH, use it directly
//! 2. **npx fallback**: If `mmdc` is not found but `npx` is available, use `npx mmdc`
//!    with a warning to stderr explaining the temporary installation
//! 3. **Error**: If neither is available, return an error asking the user to install npm
//!
//! ## Image Support
//!
//! This module requires the `viuer` feature to be enabled for actual image display.
//! When `viuer` is not available, only the fallback code block is provided.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::mermaid::MermaidRenderer;
//!
//! fn example() -> Result<(), biscuit_terminal::components::mermaid::MermaidRenderError> {
//!     let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
//!     renderer.render_for_terminal()?;
//!     Ok(())
//! }
//! ```

use std::io::Write;
use std::path::Path;
use std::process::Command;

use thiserror::Error;

/// Maximum input size for mmdc (10KB should be plenty for diagrams).
///
/// This limit prevents accidentally passing excessively large content to mmdc.
/// Most Mermaid diagrams are well under this size.
const MAX_DIAGRAM_SIZE: usize = 10_000;

/// Icon packs to enable for Mermaid diagrams.
///
/// These icon packs are passed to mmdc via `--iconPacks`:
/// - `@iconify-json/fa7-brands` - Font Awesome 7 brand icons
/// - `@iconify-json/lucide` - Lucide icons
/// - `@iconify-json/carbon` - Carbon Design icons
/// - `@iconify-json/system-uicons` - System UI icons
const ICON_PACKS: &[&str] = &[
    "@iconify-json/fa7-brands",
    "@iconify-json/lucide",
    "@iconify-json/carbon",
    "@iconify-json/system-uicons",
];

/// Default scale factor for rendering (2x for better resolution on modern displays).
const DEFAULT_SCALE: u32 = 2;

/// Mermaid theme options.
///
/// These correspond to the built-in themes available in mermaid-cli.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MermaidTheme {
    /// Dark theme - light text on dark background (default for dark terminals)
    #[default]
    Dark,
    /// Default/light theme - dark text on light background
    Default,
    /// Forest theme - green tones
    Forest,
    /// Neutral theme - grayscale, works well with transparent backgrounds
    Neutral,
}

impl MermaidTheme {
    /// Returns the theme string for mmdc CLI.
    pub fn as_str(&self) -> &'static str {
        match self {
            MermaidTheme::Dark => "dark",
            MermaidTheme::Default => "default",
            MermaidTheme::Forest => "forest",
            MermaidTheme::Neutral => "neutral",
        }
    }

    /// Returns the appropriate theme for a given color mode.
    ///
    /// - Dark terminals get `Dark` theme
    /// - Light terminals get `Default` theme
    /// - Unknown defaults to `Dark`
    pub fn for_color_mode(mode: crate::discovery::detection::ColorMode) -> Self {
        use crate::discovery::detection::ColorMode;
        match mode {
            ColorMode::Light => MermaidTheme::Default,
            ColorMode::Dark | ColorMode::Unknown => MermaidTheme::Dark,
        }
    }

    /// Returns the inverse theme (for solid background rendering).
    ///
    /// - Dark → Default (light)
    /// - Default → Dark
    /// - Forest → Dark
    /// - Neutral → Dark
    pub fn inverse(self) -> Self {
        match self {
            MermaidTheme::Dark => MermaidTheme::Default,
            MermaidTheme::Default => MermaidTheme::Dark,
            MermaidTheme::Forest => MermaidTheme::Dark,
            MermaidTheme::Neutral => MermaidTheme::Dark,
        }
    }
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

    /// Terminal does not support image rendering.
    #[error("Terminal does not support image rendering (use fallback_code_block instead)")]
    NoImageSupport,
}

/// Checks if a command exists in the system PATH.
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A Mermaid diagram renderer for terminal output.
///
/// This struct handles rendering Mermaid diagrams to the terminal using the
/// `mmdc` CLI tool. It supports automatic CLI detection with npx fallback,
/// icon packs, and graceful fallback to code blocks when image rendering
/// is not available.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::mermaid::MermaidRenderer;
///
/// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
///
/// // Try to render as an image, or get the fallback code block
/// match renderer.render_for_terminal() {
///     Ok(()) => println!("Diagram rendered successfully!"),
///     Err(e) => {
///         eprintln!("Render failed: {}", e);
///         println!("{}", renderer.fallback_code_block());
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MermaidRenderer {
    /// The Mermaid diagram instructions
    instructions: String,
    /// Optional title for alt text
    title: Option<String>,
    /// Theme for rendering
    theme: MermaidTheme,
    /// Scale factor for output resolution (default: 2)
    scale: u32,
    /// Use transparent background
    transparent_background: bool,
}

impl MermaidRenderer {
    /// Creates a new MermaidRenderer with the given diagram instructions.
    ///
    /// Uses default settings: dark theme, 2x scale, opaque background.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// ```
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        Self {
            instructions: instructions.into(),
            title: None,
            theme: MermaidTheme::default(),
            scale: DEFAULT_SCALE,
            transparent_background: false,
        }
    }

    /// Creates a MermaidRenderer configured for the current terminal.
    ///
    /// Automatically detects color mode and sets appropriate theme.
    /// Uses transparent background for better terminal integration.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
    /// // Theme and background are automatically configured
    /// ```
    pub fn for_terminal<S: Into<String>>(instructions: S) -> Self {
        use crate::terminal::Terminal;

        let color_mode = Terminal::color_mode();
        Self {
            instructions: instructions.into(),
            title: None,
            theme: MermaidTheme::for_color_mode(color_mode),
            scale: DEFAULT_SCALE,
            transparent_background: true,
        }
    }

    /// Sets the theme for rendering.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidTheme};
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_theme(MermaidTheme::Neutral);
    /// ```
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the scale factor for output resolution.
    ///
    /// Higher values produce sharper images but larger files.
    /// Default is 2 (good for most modern displays).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_scale(3); // Extra sharp
    /// ```
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale.max(1); // Minimum scale of 1
        self
    }

    /// Enables transparent background for better terminal integration.
    ///
    /// When enabled, the diagram background will be transparent,
    /// allowing it to blend with the terminal's background color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_transparent_background(true);
    /// ```
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.transparent_background = transparent;
        self
    }

    /// Sets a title for the diagram (used for alt text).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_title("My Flowchart");
    /// ```
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Returns the diagram instructions.
    pub fn instructions(&self) -> &str {
        &self.instructions
    }

    /// Returns the title if set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns alt text for the diagram.
    ///
    /// Uses the explicit title if set, otherwise detects the diagram type
    /// from the first line of instructions.
    pub fn alt_text(&self) -> String {
        if let Some(ref title) = self.title {
            title.clone()
        } else {
            detect_diagram_type(&self.instructions)
        }
    }

    /// Returns a fallback code block string for the diagram.
    ///
    /// This is used when terminal rendering fails or is not supported.
    /// Returns the instructions formatted as a fenced code block.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// let fallback = renderer.fallback_code_block();
    /// assert!(fallback.contains("```mermaid"));
    /// ```
    pub fn fallback_code_block(&self) -> String {
        format!("```mermaid\n{}\n```", self.instructions)
    }

    /// Prints the fallback code block to stdout.
    ///
    /// This is a convenience method for when terminal rendering fails.
    pub fn print_fallback(&self) {
        println!("{}", self.fallback_code_block());
    }

    /// Checks if the current terminal supports image rendering.
    ///
    /// Returns `true` if either Kitty or iTerm2 image protocols are supported.
    #[cfg(feature = "viuer")]
    pub fn terminal_supports_images() -> bool {
        use crate::discovery::detection::ImageSupport;
        use crate::terminal::Terminal;

        let term = Terminal::new();
        !matches!(term.image_support, ImageSupport::None)
    }

    /// Checks if the current terminal supports image rendering.
    ///
    /// Always returns `false` when viuer feature is disabled.
    #[cfg(not(feature = "viuer"))]
    pub fn terminal_supports_images() -> bool {
        false
    }

    /// Renders the diagram to the terminal using the local mmdc CLI.
    ///
    /// This method:
    /// 1. Validates diagram size (< 10KB)
    /// 2. Checks if the terminal supports image rendering
    /// 3. Creates temporary input file with diagram instructions
    /// 4. Executes mmdc with dark theme and icon pack support
    /// 5. Displays the output PNG with viuer
    /// 6. Cleans up temporary files
    ///
    /// ## Icon Pack Support
    ///
    /// This method enables the following icon packs via `--iconPacks`:
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
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// fn example() -> Result<(), biscuit_terminal::components::mermaid::MermaidRenderError> {
    ///     let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    ///     renderer.render_for_terminal()?;
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
    #[cfg(feature = "viuer")]
    #[tracing::instrument(skip(self))]
    pub fn render_for_terminal(&self) -> Result<(), MermaidRenderError> {
        // 1. Validate size
        if self.instructions.len() > MAX_DIAGRAM_SIZE {
            tracing::error!(
                size = self.instructions.len(),
                max = MAX_DIAGRAM_SIZE,
                "Diagram too large for mmdc"
            );
            return Err(MermaidRenderError::ContentTooLarge {
                size: self.instructions.len(),
                max: MAX_DIAGRAM_SIZE,
            });
        }

        // 2. Check terminal support
        if !Self::terminal_supports_images() {
            tracing::debug!("Terminal does not support image rendering");
            return Err(MermaidRenderError::NoImageSupport);
        }

        // 3. Render the diagram to a temporary PNG file
        let output_path = self.render_to_temp_png()?;

        // 4. Display with viuer
        let config = viuer::Config {
            absolute_offset: false,
            ..Default::default()
        };

        tracing::info!(path = ?output_path, "Displaying diagram in terminal");

        viuer::print_from_file(&output_path, &config)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?;

        tracing::debug!("Displayed diagram in terminal");

        // 5. Cleanup output file
        let _ = std::fs::remove_file(&output_path);
        tracing::trace!("Cleaned up temporary output file");

        Ok(())
    }

    /// Renders the diagram to the terminal (stub when viuer is disabled).
    #[cfg(not(feature = "viuer"))]
    pub fn render_for_terminal(&self) -> Result<(), MermaidRenderError> {
        Err(MermaidRenderError::NoImageSupport)
    }

    /// Renders the diagram to a temporary PNG file.
    ///
    /// Returns the path to the generated PNG file. The caller is responsible
    /// for cleaning up the file after use.
    ///
    /// ## Errors
    ///
    /// Returns error if mmdc is not available or execution fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_temp_png(&self) -> Result<std::path::PathBuf, MermaidRenderError> {
        use std::io::Write as _;
        use tempfile::Builder;

        // Validate size
        if self.instructions.len() > MAX_DIAGRAM_SIZE {
            return Err(MermaidRenderError::ContentTooLarge {
                size: self.instructions.len(),
                max: MAX_DIAGRAM_SIZE,
            });
        }

        // Create temp files with tempfile crate (RAII cleanup for input)
        let input_file = Builder::new().suffix(".mmd").tempfile()?;

        tracing::debug!(path = ?input_file.path(), "Created temporary input file");

        // Write instructions to input file
        std::fs::write(input_file.path(), &self.instructions)?;

        // Output path (alongside input, will be returned to caller for cleanup)
        let output_path = input_file.path().with_extension("png");

        tracing::debug!(
            input = ?input_file.path(),
            output = ?output_path,
            "Prepared file paths for mmdc"
        );

        // Determine how to run mmdc (direct or via npx)
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

        // Build and execute mmdc command
        tracing::info!(use_npx, "Executing mmdc CLI");
        let mut cmd = if use_npx {
            let mut c = Command::new("npx");
            c.args(["-p", "@mermaid-js/mermaid-cli", "mmdc"]);
            c
        } else {
            Command::new("mmdc")
        };

        // Add common arguments
        cmd.args(["-i", input_file.path().to_str().unwrap()])
            .args(["-o", output_path.to_str().unwrap()])
            .args(["--theme", self.theme.as_str()])
            .args(["--scale", &self.scale.to_string()]);

        // Add transparent background if requested
        if self.transparent_background {
            cmd.args(["--backgroundColor", "transparent"]);
        }

        // Add icon packs
        cmd.arg("--iconPacks").args(ICON_PACKS);

        let output = cmd.output();

        // Handle errors
        let output = match output {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
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

        Ok(output_path)
    }

    /// Renders the diagram to a PNG file at the specified path.
    ///
    /// ## Arguments
    ///
    /// * `output_path` - The path where the PNG file should be written
    ///
    /// ## Errors
    ///
    /// Returns error if mmdc is not available or execution fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_file(&self, output_path: &Path) -> Result<(), MermaidRenderError> {
        use tempfile::Builder;

        // Validate size
        if self.instructions.len() > MAX_DIAGRAM_SIZE {
            return Err(MermaidRenderError::ContentTooLarge {
                size: self.instructions.len(),
                max: MAX_DIAGRAM_SIZE,
            });
        }

        // Create temp input file
        let input_file = Builder::new().suffix(".mmd").tempfile()?;
        std::fs::write(input_file.path(), &self.instructions)?;

        // Determine how to run mmdc
        let use_npx = if command_exists("mmdc") {
            false
        } else if command_exists("npx") {
            let _ = writeln!(
                std::io::stderr(),
                "- Using npx to run mmdc temporarily\n\
                 - To install permanently: npm install -g @mermaid-js/mermaid-cli"
            );
            true
        } else {
            return Err(MermaidRenderError::NpmNotFound);
        };

        // Build command
        let mut cmd = if use_npx {
            let mut c = Command::new("npx");
            c.args(["-p", "@mermaid-js/mermaid-cli", "mmdc"]);
            c
        } else {
            Command::new("mmdc")
        };

        // Add common arguments
        cmd.args(["-i", input_file.path().to_str().unwrap()])
            .args(["-o", output_path.to_str().unwrap()])
            .args(["--theme", self.theme.as_str()])
            .args(["--scale", &self.scale.to_string()]);

        // Add transparent background if requested
        if self.transparent_background {
            cmd.args(["--backgroundColor", "transparent"]);
        }

        // Add icon packs
        cmd.arg("--iconPacks").args(ICON_PACKS);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(MermaidRenderError::MmdcExecutionFailed { exit_code, stderr });
        }

        Ok(())
    }
}

impl From<String> for MermaidRenderer {
    fn from(instructions: String) -> Self {
        Self::new(instructions)
    }
}

impl From<&str> for MermaidRenderer {
    fn from(instructions: &str) -> Self {
        Self::new(instructions)
    }
}

/// Detects the diagram type from the first line of instructions.
///
/// Returns a human-readable string like "Flowchart diagram" or "Sequence diagram".
fn detect_diagram_type(instructions: &str) -> String {
    let first_line = instructions.lines().next().unwrap_or("").to_lowercase();

    if first_line.starts_with("flowchart") || first_line.starts_with("graph") {
        "Flowchart diagram".to_string()
    } else if first_line.starts_with("sequencediagram") {
        "Sequence diagram".to_string()
    } else if first_line.starts_with("classdiagram") {
        "Class diagram".to_string()
    } else if first_line.starts_with("statediagram") {
        "State diagram".to_string()
    } else if first_line.starts_with("erdiagram") {
        "Entity-Relationship diagram".to_string()
    } else if first_line.starts_with("gantt") {
        "Gantt chart".to_string()
    } else if first_line.starts_with("pie") {
        "Pie chart".to_string()
    } else if first_line.starts_with("journey") {
        "User journey diagram".to_string()
    } else if first_line.starts_with("gitgraph") {
        "Git graph".to_string()
    } else if first_line.starts_with("mindmap") {
        "Mind map".to_string()
    } else if first_line.starts_with("timeline") {
        "Timeline".to_string()
    } else if first_line.starts_with("quadrantchart") {
        "Quadrant chart".to_string()
    } else if first_line.starts_with("sankey") {
        "Sankey diagram".to_string()
    } else if first_line.starts_with("xychart") {
        "XY chart".to_string()
    } else {
        "Mermaid diagram".to_string()
    }
}

/// Returns a fallback code block string for the given instructions.
///
/// This is a standalone function for use when you don't need the full
/// `MermaidRenderer` struct.
///
/// ## Examples
///
/// ```rust
/// use biscuit_terminal::components::mermaid::fallback_code_block;
///
/// let output = fallback_code_block("flowchart LR\n    A --> B");
/// assert!(output.starts_with("```mermaid\n"));
/// assert!(output.ends_with("\n```"));
/// ```
pub fn fallback_code_block(instructions: &str) -> String {
    format!("```mermaid\n{}\n```", instructions)
}

/// Prints a fallback code block for the given instructions to stdout.
///
/// This is a standalone function for use when you don't need the full
/// `MermaidRenderer` struct.
pub fn print_fallback_code_block(instructions: &str) {
    println!("{}", fallback_code_block(instructions));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mermaid_renderer_new() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
        assert!(renderer.title().is_none());
    }

    #[test]
    fn test_mermaid_renderer_with_title() {
        let renderer =
            MermaidRenderer::new("flowchart LR\n    A --> B").with_title("My Flowchart");
        assert_eq!(renderer.title(), Some("My Flowchart"));
    }

    #[test]
    fn test_mermaid_renderer_from_string() {
        let instructions = String::from("flowchart LR\n    A --> B");
        let renderer = MermaidRenderer::from(instructions.clone());
        assert_eq!(renderer.instructions(), instructions);
    }

    #[test]
    fn test_mermaid_renderer_from_str() {
        let renderer = MermaidRenderer::from("flowchart LR\n    A --> B");
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
    }

    #[test]
    fn test_mermaid_renderer_clone() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("Test");
        let cloned = renderer.clone();
        assert_eq!(renderer.instructions(), cloned.instructions());
        assert_eq!(renderer.title(), cloned.title());
    }

    #[test]
    fn test_fallback_code_block() {
        let output = fallback_code_block("flowchart LR\n    A --> B");
        assert!(output.starts_with("```mermaid\n"));
        assert!(output.ends_with("\n```"));
        assert!(output.contains("A --> B"));
    }

    #[test]
    fn test_mermaid_renderer_fallback_code_block() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
        let output = renderer.fallback_code_block();
        assert!(output.starts_with("```mermaid\n"));
        assert!(output.ends_with("\n```"));
    }

    #[test]
    fn test_detect_diagram_type_flowchart() {
        assert_eq!(
            detect_diagram_type("flowchart LR\n    A --> B"),
            "Flowchart diagram"
        );
        assert_eq!(
            detect_diagram_type("graph TD\n    A --> B"),
            "Flowchart diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_sequence() {
        assert_eq!(
            detect_diagram_type("sequenceDiagram\n    A->>B: Hello"),
            "Sequence diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_class() {
        assert_eq!(
            detect_diagram_type("classDiagram\n    class Animal"),
            "Class diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_state() {
        assert_eq!(
            detect_diagram_type("stateDiagram-v2\n    [*] --> State1"),
            "State diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_er() {
        assert_eq!(
            detect_diagram_type("erDiagram\n    CUSTOMER ||--o{ ORDER"),
            "Entity-Relationship diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_gantt() {
        assert_eq!(
            detect_diagram_type("gantt\n    title A Gantt Diagram"),
            "Gantt chart"
        );
    }

    #[test]
    fn test_detect_diagram_type_pie() {
        assert_eq!(detect_diagram_type("pie\n    \"Dogs\" : 386"), "Pie chart");
    }

    #[test]
    fn test_detect_diagram_type_journey() {
        assert_eq!(
            detect_diagram_type("journey\n    title My working day"),
            "User journey diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_gitgraph() {
        assert_eq!(
            detect_diagram_type("gitGraph\n    commit"),
            "Git graph"
        );
    }

    #[test]
    fn test_detect_diagram_type_mindmap() {
        assert_eq!(
            detect_diagram_type("mindmap\n    root((mindmap))"),
            "Mind map"
        );
    }

    #[test]
    fn test_detect_diagram_type_timeline() {
        assert_eq!(
            detect_diagram_type("timeline\n    title History"),
            "Timeline"
        );
    }

    #[test]
    fn test_detect_diagram_type_unknown() {
        assert_eq!(
            detect_diagram_type("unknown\n    foo bar"),
            "Mermaid diagram"
        );
    }

    #[test]
    fn test_alt_text_with_title() {
        let renderer =
            MermaidRenderer::new("flowchart LR\n    A --> B").with_title("Custom Title");
        assert_eq!(renderer.alt_text(), "Custom Title");
    }

    #[test]
    fn test_alt_text_auto_detect() {
        let renderer = MermaidRenderer::new("sequenceDiagram\n    A->>B: Hello");
        assert_eq!(renderer.alt_text(), "Sequence diagram");
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
    fn test_error_display_npm_not_found() {
        let error = MermaidRenderError::NpmNotFound;
        assert_eq!(
            error.to_string(),
            "npm not found. Install Node.js and npm to render Mermaid diagrams in the terminal"
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

    #[test]
    fn test_error_display_display_error() {
        let error = MermaidRenderError::DisplayError("failed to render".to_string());
        assert_eq!(error.to_string(), "Failed to display image: failed to render");
    }

    #[test]
    fn test_error_display_no_image_support() {
        let error = MermaidRenderError::NoImageSupport;
        assert!(error
            .to_string()
            .contains("does not support image rendering"));
    }

    #[test]
    fn test_max_diagram_size_constant() {
        assert_eq!(MAX_DIAGRAM_SIZE, 10_000);
    }

    #[test]
    fn test_small_diagram_under_limit() {
        let instructions = "flowchart LR\n    A --> B";
        assert!(instructions.len() < MAX_DIAGRAM_SIZE);
    }

    #[test]
    fn test_command_exists_with_common_command() {
        // 'which' should exist on all Unix systems
        assert!(command_exists("which"));
    }

    #[test]
    fn test_command_exists_with_nonexistent_command() {
        assert!(!command_exists(
            "this_command_definitely_does_not_exist_xyz123"
        ));
    }

    #[test]
    fn test_icon_packs_constant() {
        assert_eq!(ICON_PACKS.len(), 4);
        assert!(ICON_PACKS.contains(&"@iconify-json/fa7-brands"));
        assert!(ICON_PACKS.contains(&"@iconify-json/lucide"));
        assert!(ICON_PACKS.contains(&"@iconify-json/carbon"));
        assert!(ICON_PACKS.contains(&"@iconify-json/system-uicons"));
    }

    #[test]
    fn test_render_to_temp_png_rejects_large_content() {
        // Create a diagram that exceeds the size limit
        let large_instructions = "A".repeat(MAX_DIAGRAM_SIZE + 1);
        let renderer = MermaidRenderer::new(large_instructions);

        let result = renderer.render_to_temp_png();
        assert!(matches!(
            result,
            Err(MermaidRenderError::ContentTooLarge { .. })
        ));
    }

    #[test]
    fn test_render_to_file_rejects_large_content() {
        let large_instructions = "B".repeat(MAX_DIAGRAM_SIZE + 1);
        let renderer = MermaidRenderer::new(large_instructions);

        let result = renderer.render_to_file(std::path::Path::new("/tmp/test.png"));
        assert!(matches!(
            result,
            Err(MermaidRenderError::ContentTooLarge { .. })
        ));
    }

    #[test]
    fn test_detect_diagram_type_quadrant() {
        assert_eq!(
            detect_diagram_type("quadrantChart\n    title Test"),
            "Quadrant chart"
        );
    }

    #[test]
    fn test_detect_diagram_type_sankey() {
        assert_eq!(
            detect_diagram_type("sankey-beta\n    A[Source]"),
            "Sankey diagram"
        );
    }

    #[test]
    fn test_detect_diagram_type_xychart() {
        assert_eq!(
            detect_diagram_type("xychart-beta\n    title Test"),
            "XY chart"
        );
    }

    #[test]
    fn test_render_for_terminal_rejects_large_content() {
        let large_instructions = "C".repeat(MAX_DIAGRAM_SIZE + 1);
        let renderer = MermaidRenderer::new(large_instructions);

        let result = renderer.render_for_terminal();
        assert!(matches!(
            result,
            Err(MermaidRenderError::ContentTooLarge { .. })
        ));
    }

    #[test]
    fn test_mermaid_renderer_debug() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("Test");
        let debug_str = format!("{:?}", renderer);
        assert!(debug_str.contains("MermaidRenderer"));
        assert!(debug_str.contains("flowchart"));
    }
}
