use std::any::Any;
use std::collections::HashMap;

/// **Browser-Aware Render**
///
/// Renders the component as HTML/SVG suitable for browser display.
/// This trait is used by libraries like darkmatter to generate
/// web-compatible output from terminal components.
pub trait BrowserRenderable: std::fmt::Debug + Any {
    /// Renders the component to browser-compatible HTML/SVG.
    fn render_to_browser(&self) -> String;

    /// Renders the component to browser-compatible HTML/SVG with inline CSS variables.
    /// The `variables` parameter provides CSS variable definitions that can be used
    /// in the rendered output for dynamic styling.
    fn render_to_browser_with_inline_variables(
        &self,
        _variables: &HashMap<String, String>,
    ) -> String {
        // Default implementation ignores variables and calls the basic render method
        self.render_to_browser()
    }

    fn as_any(&self) -> &dyn Any;
}
