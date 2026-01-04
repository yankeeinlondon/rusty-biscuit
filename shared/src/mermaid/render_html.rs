//! HTML rendering for Mermaid diagrams.
//!
//! This module provides HTML output generation with accessibility features,
//! including ARIA attributes, alt text detection, and theme-aware CSS variables.

/// HTML output from mermaid rendering.
///
/// Contains separate head and body sections for flexible HTML integration.
///
/// ## Examples
///
/// ```rust
/// use shared::mermaid::Mermaid;
///
/// let diagram = Mermaid::new("flowchart LR\n    A --> B");
/// let html = diagram.render_for_html();
///
/// // Use in HTML document
/// println!("<html><head>{}</head><body>{}</body></html>", html.head, html.body);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MermaidHtml {
    /// Content for the <head> section (scripts, styles)
    pub head: String,
    /// Content for the <body> section (diagram container)
    pub body: String,
}

impl MermaidHtml {
    /// Creates a new MermaidHtml with head and body content.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::mermaid::MermaidHtml;
    ///
    /// let html = MermaidHtml::new("<script>...</script>", "<pre>...</pre>");
    /// ```
    pub fn new<S1: Into<String>, S2: Into<String>>(head: S1, body: S2) -> Self {
        Self {
            head: head.into(),
            body: body.into(),
        }
    }
}

/// Detects the diagram type from the first line of instructions.
///
/// ## Returns
///
/// Returns a human-readable description of the diagram type.
///
/// ## Examples
///
/// ```rust
/// # use shared::mermaid::render_html::detect_diagram_type;
/// assert_eq!(detect_diagram_type("flowchart LR"), "Flowchart diagram");
/// assert_eq!(detect_diagram_type("sequenceDiagram"), "Sequence diagram");
/// assert_eq!(detect_diagram_type("unknown"), "Mermaid diagram");
/// ```
pub fn detect_diagram_type(instructions: &str) -> &'static str {
    let first_line = instructions.lines().next().unwrap_or("").trim();

    if first_line.starts_with("flowchart") || first_line.starts_with("graph") {
        "Flowchart diagram"
    } else if first_line.starts_with("sequenceDiagram") {
        "Sequence diagram"
    } else if first_line.starts_with("classDiagram") {
        "Class diagram"
    } else if first_line.starts_with("stateDiagram") {
        "State diagram"
    } else if first_line.starts_with("erDiagram") {
        "Entity relationship diagram"
    } else if first_line.starts_with("pie") {
        "Pie chart"
    } else if first_line.starts_with("gantt") {
        "Gantt chart"
    } else if first_line.starts_with("journey") {
        "User journey diagram"
    } else if first_line.starts_with("gitGraph") || first_line.starts_with("gitgraph") {
        "Git graph diagram"
    } else if first_line.starts_with("mindmap") {
        "Mind map diagram"
    } else if first_line.starts_with("timeline") {
        "Timeline diagram"
    } else {
        "Mermaid diagram"
    }
}

/// Generates CSS variable definitions for theme colors.
///
/// ## Examples
///
/// ```rust
/// # use shared::mermaid::{DEFAULT_LIGHT_THEME, DEFAULT_DARK_THEME, render_html::generate_css_variables};
/// let css = generate_css_variables(&DEFAULT_LIGHT_THEME, &DEFAULT_DARK_THEME);
/// assert!(css.contains("--mermaid-primary-color"));
/// ```
pub fn generate_css_variables(
    light_theme: &crate::mermaid::MermaidTheme,
    dark_theme: &crate::mermaid::MermaidTheme,
) -> String {
    // Helper to unwrap Option<String> or use fallback
    let opt = |val: &Option<String>, fallback: &'static str| -> String {
        val.as_ref().map(|s| s.to_string()).unwrap_or_else(|| fallback.to_string())
    };

    format!(
        r#"<style>
  :root {{
    --mermaid-background: {};
    --mermaid-primary-color: {};
    --mermaid-secondary-color: {};
    --mermaid-tertiary-color: {};
    --mermaid-primary-border-color: {};
    --mermaid-secondary-border-color: {};
    --mermaid-tertiary-border-color: {};
    --mermaid-primary-text-color: {};
    --mermaid-secondary-text-color: {};
    --mermaid-tertiary-text-color: {};
    --mermaid-line-color: {};
    --mermaid-text-color: {};
    --mermaid-main-bkg: {};
    --mermaid-node-border: {};
  }}

  @media (prefers-color-scheme: dark) {{
    :root {{
      --mermaid-background: {};
      --mermaid-primary-color: {};
      --mermaid-secondary-color: {};
      --mermaid-tertiary-color: {};
      --mermaid-primary-border-color: {};
      --mermaid-secondary-border-color: {};
      --mermaid-tertiary-border-color: {};
      --mermaid-primary-text-color: {};
      --mermaid-secondary-text-color: {};
      --mermaid-tertiary-text-color: {};
      --mermaid-line-color: {};
      --mermaid-text-color: {};
      --mermaid-main-bkg: {};
      --mermaid-node-border: {};
    }}
  }}
</style>"#,
        // Light theme
        &light_theme.background,
        &light_theme.primary_color,
        &opt(&light_theme.secondary_color, "#6699cc"),
        &opt(&light_theme.tertiary_color, "#99ccff"),
        &opt(&light_theme.primary_border_color, "#9370db"),
        &opt(&light_theme.secondary_border_color, "#6699cc"),
        &opt(&light_theme.tertiary_border_color, "#99ccff"),
        &opt(&light_theme.primary_text_color, "#000000"),
        &opt(&light_theme.secondary_text_color, "#000000"),
        &opt(&light_theme.tertiary_text_color, "#000000"),
        &opt(&light_theme.line_color, "#333333"),
        &opt(&light_theme.text_color, "#333333"),
        &opt(&light_theme.main_bkg, "#ececff"),
        &opt(&light_theme.node_border, "#9370db"),
        // Dark theme
        &dark_theme.background,
        &dark_theme.primary_color,
        &opt(&dark_theme.secondary_color, "#6699cc"),
        &opt(&dark_theme.tertiary_color, "#99ccff"),
        &opt(&dark_theme.primary_border_color, "#9370db"),
        &opt(&dark_theme.secondary_border_color, "#6699cc"),
        &opt(&dark_theme.tertiary_border_color, "#99ccff"),
        &opt(&dark_theme.primary_text_color, "#ffffff"),
        &opt(&dark_theme.secondary_text_color, "#ffffff"),
        &opt(&dark_theme.tertiary_text_color, "#ffffff"),
        &opt(&dark_theme.line_color, "#cccccc"),
        &opt(&dark_theme.text_color, "#cccccc"),
        &opt(&dark_theme.main_bkg, "#1e1e1e"),
        &opt(&dark_theme.node_border, "#9370db"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::{DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME};

    #[test]
    fn test_mermaid_html_new() {
        let html = MermaidHtml::new("<script>test</script>", "<pre>diagram</pre>");
        assert_eq!(html.head, "<script>test</script>");
        assert_eq!(html.body, "<pre>diagram</pre>");
    }

    #[test]
    fn test_detect_diagram_type_flowchart() {
        assert_eq!(detect_diagram_type("flowchart LR"), "Flowchart diagram");
        assert_eq!(detect_diagram_type("flowchart TD\n    A --> B"), "Flowchart diagram");
    }

    #[test]
    fn test_detect_diagram_type_graph() {
        assert_eq!(detect_diagram_type("graph LR"), "Flowchart diagram");
    }

    #[test]
    fn test_detect_diagram_type_sequence() {
        assert_eq!(detect_diagram_type("sequenceDiagram"), "Sequence diagram");
    }

    #[test]
    fn test_detect_diagram_type_class() {
        assert_eq!(detect_diagram_type("classDiagram"), "Class diagram");
    }

    #[test]
    fn test_detect_diagram_type_state() {
        assert_eq!(detect_diagram_type("stateDiagram"), "State diagram");
    }

    #[test]
    fn test_detect_diagram_type_er() {
        assert_eq!(detect_diagram_type("erDiagram"), "Entity relationship diagram");
    }

    #[test]
    fn test_detect_diagram_type_pie() {
        assert_eq!(detect_diagram_type("pie"), "Pie chart");
    }

    #[test]
    fn test_detect_diagram_type_gantt() {
        assert_eq!(detect_diagram_type("gantt"), "Gantt chart");
    }

    #[test]
    fn test_detect_diagram_type_journey() {
        assert_eq!(detect_diagram_type("journey"), "User journey diagram");
    }

    #[test]
    fn test_detect_diagram_type_git_graph() {
        assert_eq!(detect_diagram_type("gitGraph"), "Git graph diagram");
        assert_eq!(detect_diagram_type("gitgraph"), "Git graph diagram");
    }

    #[test]
    fn test_detect_diagram_type_mindmap() {
        assert_eq!(detect_diagram_type("mindmap"), "Mind map diagram");
    }

    #[test]
    fn test_detect_diagram_type_timeline() {
        assert_eq!(detect_diagram_type("timeline"), "Timeline diagram");
    }

    #[test]
    fn test_detect_diagram_type_unknown() {
        assert_eq!(detect_diagram_type("unknown"), "Mermaid diagram");
        assert_eq!(detect_diagram_type(""), "Mermaid diagram");
    }

    #[test]
    fn test_generate_css_variables_contains_light_theme() {
        let css = generate_css_variables(&DEFAULT_LIGHT_THEME, &DEFAULT_DARK_THEME);
        assert!(css.contains("--mermaid-primary-color"));
        assert!(css.contains(&DEFAULT_LIGHT_THEME.primary_color));
    }

    #[test]
    fn test_generate_css_variables_contains_dark_theme() {
        let css = generate_css_variables(&DEFAULT_LIGHT_THEME, &DEFAULT_DARK_THEME);
        assert!(css.contains("prefers-color-scheme: dark"));
        assert!(css.contains(&DEFAULT_DARK_THEME.primary_color));
    }

    #[test]
    fn test_generate_css_variables_all_properties() {
        let css = generate_css_variables(&DEFAULT_LIGHT_THEME, &DEFAULT_DARK_THEME);

        // Verify all CSS variables are present
        assert!(css.contains("--mermaid-background"));
        assert!(css.contains("--mermaid-primary-color"));
        assert!(css.contains("--mermaid-secondary-color"));
        assert!(css.contains("--mermaid-tertiary-color"));
        assert!(css.contains("--mermaid-primary-border-color"));
        assert!(css.contains("--mermaid-secondary-border-color"));
        assert!(css.contains("--mermaid-tertiary-border-color"));
        assert!(css.contains("--mermaid-primary-text-color"));
        assert!(css.contains("--mermaid-secondary-text-color"));
        assert!(css.contains("--mermaid-tertiary-text-color"));
        assert!(css.contains("--mermaid-line-color"));
        assert!(css.contains("--mermaid-text-color"));
        assert!(css.contains("--mermaid-main-bkg"));
        assert!(css.contains("--mermaid-node-border"));
    }
}
