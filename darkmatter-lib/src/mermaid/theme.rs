//! Mermaid theme color schemes.
//!
//! Provides the `MermaidTheme` struct for representing Mermaid diagram themes
//! with support for JSON parsing and validation.

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::markdown::highlighting::{ColorMode, ThemePair};

/// Error type for Mermaid theme operations.
#[derive(Error, Debug)]
pub enum MermaidThemeError {
    /// Invalid JSON syntax.
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// Invalid color value for a field.
    #[error("Invalid color value for '{field}': {value}")]
    InvalidColor {
        /// The field name with the invalid color.
        field: String,
        /// The invalid color value.
        value: String,
    },
}

/// Mermaid theme color scheme.
///
/// Represents all color variables that can be customized in a Mermaid theme.
/// Supports JSON serialization/deserialization with camelCase field names.
///
/// ## Examples
///
/// ```
/// use darkmatter_lib::mermaid::MermaidTheme;
/// use std::convert::TryFrom;
///
/// let json = r##"{
///     "background": "#1e1e1e",
///     "primaryColor": "#569cd6"
/// }"##;
///
/// let theme = MermaidTheme::try_from(json).unwrap();
/// assert_eq!(theme.background, "#1e1e1e");
/// assert_eq!(theme.primary_color, "#569cd6");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MermaidTheme {
    // Core theme colors
    /// Background color for the diagram.
    #[serde(default = "default_background")]
    pub background: String,

    /// Primary theme color.
    #[serde(default = "default_primary_color")]
    pub primary_color: String,

    /// Text color for primary elements.
    #[serde(default)]
    pub primary_text_color: Option<String>,

    /// Border color for primary elements.
    #[serde(default)]
    pub primary_border_color: Option<String>,

    /// Secondary theme color.
    #[serde(default)]
    pub secondary_color: Option<String>,

    /// Text color for secondary elements.
    #[serde(default)]
    pub secondary_text_color: Option<String>,

    /// Border color for secondary elements.
    #[serde(default)]
    pub secondary_border_color: Option<String>,

    /// Tertiary theme color.
    #[serde(default)]
    pub tertiary_color: Option<String>,

    /// Text color for tertiary elements.
    #[serde(default)]
    pub tertiary_text_color: Option<String>,

    /// Border color for tertiary elements.
    #[serde(default)]
    pub tertiary_border_color: Option<String>,

    // Note styling
    /// Background color for notes.
    #[serde(default)]
    pub note_bkg_color: Option<String>,

    /// Text color for notes.
    #[serde(default)]
    pub note_text_color: Option<String>,

    /// Border color for notes.
    #[serde(default)]
    pub note_border_color: Option<String>,

    // General styling
    /// Default line color.
    #[serde(default)]
    pub line_color: Option<String>,

    /// Default text color.
    #[serde(default)]
    pub text_color: Option<String>,

    /// Main background color.
    #[serde(default)]
    pub main_bkg: Option<String>,

    /// Font family for text.
    #[serde(default)]
    pub font_family: Option<String>,

    /// Font size for text.
    #[serde(default)]
    pub font_size: Option<String>,

    // Flowchart-specific
    /// Node border color.
    #[serde(default)]
    pub node_border: Option<String>,

    /// Cluster background color.
    #[serde(default)]
    pub cluster_bkg: Option<String>,

    /// Cluster border color.
    #[serde(default)]
    pub cluster_border: Option<String>,

    /// Default link/edge color.
    #[serde(default)]
    pub default_link_color: Option<String>,

    /// Title text color.
    #[serde(default)]
    pub title_color: Option<String>,

    /// Edge label background color.
    #[serde(default)]
    pub edge_label_background: Option<String>,

    /// Node text color.
    #[serde(default)]
    pub node_text_color: Option<String>,

    // Sequence diagram
    /// Actor background color.
    #[serde(default)]
    pub actor_bkg: Option<String>,

    /// Actor border color.
    #[serde(default)]
    pub actor_border: Option<String>,

    /// Actor text color.
    #[serde(default)]
    pub actor_text_color: Option<String>,

    /// Actor line color.
    #[serde(default)]
    pub actor_line_color: Option<String>,

    /// Signal/message color.
    #[serde(default)]
    pub signal_color: Option<String>,

    /// Signal text color.
    #[serde(default)]
    pub signal_text_color: Option<String>,

    /// Label box background color.
    #[serde(default)]
    pub label_box_bkg_color: Option<String>,

    /// Label box border color.
    #[serde(default)]
    pub label_box_border_color: Option<String>,

    /// Label text color.
    #[serde(default)]
    pub label_text_color: Option<String>,

    /// Loop text color.
    #[serde(default)]
    pub loop_text_color: Option<String>,

    /// Activation border color.
    #[serde(default)]
    pub activation_border_color: Option<String>,

    /// Activation background color.
    #[serde(default)]
    pub activation_bkg_color: Option<String>,

    /// Sequence number color.
    #[serde(default)]
    pub sequence_number_color: Option<String>,
}

fn default_background() -> String {
    "transparent".to_string()
}

fn default_primary_color() -> String {
    "#eee".to_string()
}

impl TryFrom<String> for MermaidTheme {
    type Error = MermaidThemeError;

    fn try_from(json: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&json).map_err(MermaidThemeError::from)
    }
}

impl TryFrom<&str> for MermaidTheme {
    type Error = MermaidThemeError;

    fn try_from(json: &str) -> Result<Self, Self::Error> {
        serde_json::from_str(json).map_err(MermaidThemeError::from)
    }
}

impl TryFrom<serde_json::Value> for MermaidTheme {
    type Error = MermaidThemeError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).map_err(MermaidThemeError::from)
    }
}

lazy_static! {
    /// Default light theme matching common documentation sites.
    ///
    /// Optimized for light backgrounds with soft, readable colors. The palette
    /// uses pastel shades for primary, secondary, and tertiary elements while
    /// maintaining sufficient contrast for text legibility.
    pub static ref DEFAULT_LIGHT_THEME: MermaidTheme = MermaidTheme {
        background: "#ffffff".into(),
        primary_color: "#fff4dd".into(),
        primary_text_color: Some("#333333".into()),
        primary_border_color: Some("#666666".into()),
        secondary_color: Some("#e8f5e9".into()),
        tertiary_color: Some("#e3f2fd".into()),
        line_color: Some("#333333".into()),
        text_color: Some("#333333".into()),
        secondary_text_color: None,
        secondary_border_color: None,
        tertiary_text_color: None,
        tertiary_border_color: None,
        note_bkg_color: None,
        note_text_color: None,
        note_border_color: None,
        main_bkg: None,
        font_family: None,
        font_size: None,
        node_border: None,
        cluster_bkg: None,
        cluster_border: None,
        default_link_color: None,
        title_color: None,
        edge_label_background: None,
        node_text_color: None,
        actor_bkg: None,
        actor_border: None,
        actor_text_color: None,
        actor_line_color: None,
        signal_color: None,
        signal_text_color: None,
        label_box_bkg_color: None,
        label_box_border_color: None,
        label_text_color: None,
        loop_text_color: None,
        activation_border_color: None,
        activation_bkg_color: None,
        sequence_number_color: None,
    };

    /// Default dark theme for dark mode interfaces.
    ///
    /// Optimized for dark backgrounds with muted colors that reduce eye strain.
    /// Uses blue-gray tones for neutral elements and soft light text colors.
    pub static ref DEFAULT_DARK_THEME: MermaidTheme = MermaidTheme {
        background: "#1a1a2e".into(),
        primary_color: "#4a5568".into(),
        primary_text_color: Some("#e2e8f0".into()),
        primary_border_color: Some("#718096".into()),
        line_color: Some("#a0aec0".into()),
        text_color: Some("#e2e8f0".into()),
        secondary_color: None,
        secondary_text_color: None,
        secondary_border_color: None,
        tertiary_color: None,
        tertiary_text_color: None,
        tertiary_border_color: None,
        note_bkg_color: None,
        note_text_color: None,
        note_border_color: None,
        main_bkg: None,
        font_family: None,
        font_size: None,
        node_border: None,
        cluster_bkg: None,
        cluster_border: None,
        default_link_color: None,
        title_color: None,
        edge_label_background: None,
        node_text_color: None,
        actor_bkg: None,
        actor_border: None,
        actor_text_color: None,
        actor_line_color: None,
        signal_color: None,
        signal_text_color: None,
        label_box_bkg_color: None,
        label_box_border_color: None,
        label_text_color: None,
        loop_text_color: None,
        activation_border_color: None,
        activation_bkg_color: None,
        sequence_number_color: None,
    };

    /// High contrast neutral theme for accessibility (WCAG 2.1 AA).
    ///
    /// Provides maximum contrast with pure black text on white backgrounds,
    /// ensuring 21:1 contrast ratio which exceeds WCAG 2.1 Level AAA standards.
    /// Recommended for users requiring enhanced legibility.
    pub static ref NEUTRAL_THEME: MermaidTheme = MermaidTheme {
        background: "#ffffff".into(),
        primary_color: "#ffffff".into(),
        primary_text_color: Some("#000000".into()),
        primary_border_color: Some("#000000".into()),
        line_color: Some("#000000".into()),
        text_color: Some("#000000".into()),
        secondary_color: None,
        secondary_text_color: None,
        secondary_border_color: None,
        tertiary_color: None,
        tertiary_text_color: None,
        tertiary_border_color: None,
        note_bkg_color: None,
        note_text_color: None,
        note_border_color: None,
        main_bkg: None,
        font_family: None,
        font_size: None,
        node_border: None,
        cluster_bkg: None,
        cluster_border: None,
        default_link_color: None,
        title_color: None,
        edge_label_background: None,
        node_text_color: None,
        actor_bkg: None,
        actor_border: None,
        actor_text_color: None,
        actor_line_color: None,
        signal_color: None,
        signal_text_color: None,
        label_box_bkg_color: None,
        label_box_border_color: None,
        label_text_color: None,
        loop_text_color: None,
        activation_border_color: None,
        activation_bkg_color: None,
        sequence_number_color: None,
    };
}

/// Returns the appropriate MermaidTheme for a syntect ThemePair.
///
/// Maps syntect color modes (light/dark) to corresponding Mermaid themes.
/// Currently ignores the specific theme pair and only uses the color mode,
/// but the signature supports future theme-specific customization.
///
/// ## Examples
///
/// ```
/// use darkmatter_lib::mermaid::mermaid_theme_for_syntect;
/// use darkmatter_lib::markdown::highlighting::{ThemePair, ColorMode};
///
/// let theme = mermaid_theme_for_syntect(ThemePair::Github, ColorMode::Dark);
/// assert_eq!(theme.background, "#1a1a2e");
/// ```
pub fn mermaid_theme_for_syntect(_theme_pair: ThemePair, mode: ColorMode) -> &'static MermaidTheme {
    match mode {
        ColorMode::Light => &DEFAULT_LIGHT_THEME,
        ColorMode::Dark => &DEFAULT_DARK_THEME,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn test_theme_from_valid_json() {
        let json = r##"{
            "background": "#1e1e1e",
            "primaryColor": "#569cd6",
            "primaryTextColor": "#ffffff",
            "secondaryColor": "#ce9178",
            "lineColor": "#858585",
            "textColor": "#d4d4d4",
            "mainBkg": "#252526",
            "nodeBorder": "#569cd6",
            "clusterBkg": "#2d2d30",
            "clusterBorder": "#454545",
            "defaultLinkColor": "#858585",
            "titleColor": "#ffffff",
            "edgeLabelBackground": "#1e1e1e",
            "actorBkg": "#2d2d30",
            "actorBorder": "#569cd6",
            "actorTextColor": "#d4d4d4",
            "actorLineColor": "#858585",
            "signalColor": "#858585",
            "signalTextColor": "#d4d4d4",
            "labelBoxBkgColor": "#2d2d30",
            "labelBoxBorderColor": "#569cd6",
            "labelTextColor": "#d4d4d4",
            "loopTextColor": "#d4d4d4",
            "activationBorderColor": "#569cd6",
            "activationBkgColor": "#2d2d30",
            "sequenceNumberColor": "#d4d4d4"
        }"##;

        let theme = MermaidTheme::try_from(json).unwrap();
        assert_eq!(theme.background, "#1e1e1e");
        assert_eq!(theme.primary_color, "#569cd6");
        assert_eq!(theme.primary_text_color, Some("#ffffff".to_string()));
        assert_eq!(theme.secondary_color, Some("#ce9178".to_string()));
        assert_eq!(theme.line_color, Some("#858585".to_string()));
        assert_eq!(theme.text_color, Some("#d4d4d4".to_string()));
        assert_eq!(theme.main_bkg, Some("#252526".to_string()));
        assert_eq!(theme.node_border, Some("#569cd6".to_string()));
        assert_eq!(theme.cluster_bkg, Some("#2d2d30".to_string()));
        assert_eq!(theme.cluster_border, Some("#454545".to_string()));
        assert_eq!(theme.default_link_color, Some("#858585".to_string()));
        assert_eq!(theme.title_color, Some("#ffffff".to_string()));
        assert_eq!(theme.edge_label_background, Some("#1e1e1e".to_string()));
        assert_eq!(theme.actor_bkg, Some("#2d2d30".to_string()));
        assert_eq!(theme.actor_border, Some("#569cd6".to_string()));
        assert_eq!(theme.actor_text_color, Some("#d4d4d4".to_string()));
        assert_eq!(theme.actor_line_color, Some("#858585".to_string()));
        assert_eq!(theme.signal_color, Some("#858585".to_string()));
        assert_eq!(theme.signal_text_color, Some("#d4d4d4".to_string()));
        assert_eq!(theme.label_box_bkg_color, Some("#2d2d30".to_string()));
        assert_eq!(theme.label_box_border_color, Some("#569cd6".to_string()));
        assert_eq!(theme.label_text_color, Some("#d4d4d4".to_string()));
        assert_eq!(theme.loop_text_color, Some("#d4d4d4".to_string()));
        assert_eq!(theme.activation_border_color, Some("#569cd6".to_string()));
        assert_eq!(theme.activation_bkg_color, Some("#2d2d30".to_string()));
        assert_eq!(theme.sequence_number_color, Some("#d4d4d4".to_string()));
    }

    #[test]
    fn test_theme_from_partial_json() {
        let json = r##"{
            "background": "#ffffff",
            "primaryColor": "#000000",
            "lineColor": "#333333"
        }"##;

        let theme = MermaidTheme::try_from(json).unwrap();
        assert_eq!(theme.background, "#ffffff");
        assert_eq!(theme.primary_color, "#000000");
        assert_eq!(theme.line_color, Some("#333333".to_string()));
        // Optional fields should be None
        assert_eq!(theme.primary_text_color, None);
        assert_eq!(theme.secondary_color, None);
        assert_eq!(theme.actor_bkg, None);
    }

    #[test]
    fn test_theme_from_empty_json() {
        let json = r#"{}"#;

        let theme = MermaidTheme::try_from(json).unwrap();
        // Should use defaults
        assert_eq!(theme.background, "transparent");
        assert_eq!(theme.primary_color, "#eee");
        // All optional fields should be None
        assert_eq!(theme.primary_text_color, None);
        assert_eq!(theme.line_color, None);
        assert_eq!(theme.actor_bkg, None);
    }

    #[test]
    fn test_theme_invalid_json_syntax() {
        let json = r#"{ invalid json }"#;

        let result = MermaidTheme::try_from(json);
        assert!(result.is_err());
        match result {
            Err(MermaidThemeError::InvalidJson(_)) => {
                // Expected error type
            }
            _ => panic!("Expected InvalidJson error"),
        }
    }

    #[test]
    fn test_theme_try_from_string() {
        let json = String::from(
            r##"{
            "background": "#1e1e1e",
            "primaryColor": "#569cd6"
        }"##,
        );

        let theme = MermaidTheme::try_from(json).unwrap();
        assert_eq!(theme.background, "#1e1e1e");
        assert_eq!(theme.primary_color, "#569cd6");
    }

    #[test]
    fn test_theme_try_from_str() {
        let json = r##"{
            "background": "#1e1e1e",
            "primaryColor": "#569cd6"
        }"##;

        let theme = MermaidTheme::try_from(json).unwrap();
        assert_eq!(theme.background, "#1e1e1e");
        assert_eq!(theme.primary_color, "#569cd6");
    }

    #[test]
    fn test_theme_try_from_value() {
        let value = serde_json::json!({
            "background": "#1e1e1e",
            "primaryColor": "#569cd6",
            "lineColor": "#858585"
        });

        let theme = MermaidTheme::try_from(value).unwrap();
        assert_eq!(theme.background, "#1e1e1e");
        assert_eq!(theme.primary_color, "#569cd6");
        assert_eq!(theme.line_color, Some("#858585".to_string()));
    }

    // Phase 2: Static theme definition tests

    #[test]
    fn test_default_light_theme_has_required_fields() {
        let theme = &*DEFAULT_LIGHT_THEME;
        assert_eq!(theme.background, "#ffffff");
        assert_eq!(theme.primary_color, "#fff4dd");
        assert!(theme.primary_text_color.is_some());
        assert!(theme.primary_border_color.is_some());
        assert!(theme.secondary_color.is_some());
        assert!(theme.tertiary_color.is_some());
        assert!(theme.line_color.is_some());
        assert!(theme.text_color.is_some());
    }

    #[test]
    fn test_default_dark_theme_has_required_fields() {
        let theme = &*DEFAULT_DARK_THEME;
        assert_eq!(theme.background, "#1a1a2e");
        assert_eq!(theme.primary_color, "#4a5568");
        assert!(theme.primary_text_color.is_some());
        assert!(theme.primary_border_color.is_some());
        assert!(theme.line_color.is_some());
        assert!(theme.text_color.is_some());
    }

    #[test]
    fn test_neutral_theme_high_contrast() {
        let theme = &*NEUTRAL_THEME;
        assert_eq!(theme.background, "#ffffff");
        assert_eq!(theme.primary_color, "#ffffff");
        assert_eq!(theme.primary_text_color, Some("#000000".to_string()));
        assert_eq!(theme.primary_border_color, Some("#000000".to_string()));
        assert_eq!(theme.line_color, Some("#000000".to_string()));
        assert_eq!(theme.text_color, Some("#000000".to_string()));
    }

    #[test]
    fn test_theme_for_syntect_light_mode() {
        use crate::markdown::highlighting::{ColorMode, ThemePair};

        let theme = mermaid_theme_for_syntect(ThemePair::Github, ColorMode::Light);
        assert_eq!(theme.background, "#ffffff");
        assert_eq!(theme.primary_color, "#fff4dd");
    }

    #[test]
    fn test_theme_for_syntect_dark_mode() {
        use crate::markdown::highlighting::{ColorMode, ThemePair};

        let theme = mermaid_theme_for_syntect(ThemePair::Github, ColorMode::Dark);
        assert_eq!(theme.background, "#1a1a2e");
        assert_eq!(theme.primary_color, "#4a5568");
    }

    #[test]
    fn test_static_themes_serialize_to_json() {
        // Test that the static themes can be serialized to JSON
        let light_json = serde_json::to_string(&*DEFAULT_LIGHT_THEME).unwrap();
        assert!(light_json.contains("ffffff"));
        assert!(light_json.contains("fff4dd"));

        let dark_json = serde_json::to_string(&*DEFAULT_DARK_THEME).unwrap();
        assert!(dark_json.contains("1a1a2e"));
        assert!(dark_json.contains("4a5568"));

        let neutral_json = serde_json::to_string(&*NEUTRAL_THEME).unwrap();
        assert!(neutral_json.contains("ffffff"));
        assert!(neutral_json.contains("000000"));
    }
}
