use std::sync::Arc;

use crate::terminal::Terminal;
use crate::utils::layout::Layout;

/// A struct or enum which implements the **Renderable** trait
/// can be reduced down to a string designed to be displayed
/// in a terminal.
pub trait Renderable: std::fmt::Debug {
    /// **Opportunistic Render**
    ///
    /// renders without knowledge of the underlying terminal's
    /// capabilities with an "opportunistic" view that the
    /// terminal supports all capabilities.
    fn render(&self, layout: Option<&Layout>) -> String;

    /// **Fallback Render**
    ///
    /// Renders the component based on the capabilities of the
    /// passed in `Terminal`. Will provide graceful fallbacks
    /// when possible.
    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String;
}

/// A `RenderableWrapper`` is a utility which operates at at
/// a lower level than a `Renderable` **component** and takes in
/// string content and outputs that string _wrapped_ in some
/// sort of formatting.
pub trait RenderableWrapper {
    fn render<T: Into<String>>(&self, content: T) -> String;

    fn fallback_render<T: Into<String>>(&self, content: T, term: &Terminal) -> String;
}


#[derive(Debug)]
pub enum RenderableContent {
    String(String),
    Component(Arc<dyn Renderable>),
}

impl Clone for RenderableContent {
    fn clone(&self) -> Self {
        match self {
            RenderableContent::String(s) => RenderableContent::String(s.clone()),
            RenderableContent::Component(c) => RenderableContent::Component(Arc::clone(c)),
        }
    }
}

impl<T: Renderable + 'static> From<T> for RenderableContent {
    fn from(value: T) -> Self {
        RenderableContent::Component(Arc::new(value))
    }
}

impl From<String> for RenderableContent {
    fn from(value: String) -> Self {
        RenderableContent::String(value)
    }
}

impl From<&str> for RenderableContent {
    fn from(value: &str) -> Self {
        RenderableContent::String(value.to_string())
    }
}

impl<'a> From<&'a String> for RenderableContent {
    fn from(value: &'a String) -> Self {
        RenderableContent::String(value.clone())
    }
}
