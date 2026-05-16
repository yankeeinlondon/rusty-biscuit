use std::collections::HashMap;

use crate::{browser::feature::PageFeature, microdata::MicrodataKey, stylesheet::Stylesheet};

pub struct MarkdownStyle {
    // pub table: PageBlock,
    // pub hr: PageBlock,
}

pub enum MarkdownStyleOptions {
    /// although a stylesheet is more tightly aligned with an HTML
    /// target, Markdown content can take advantage of _downsampling_
    /// the stylesheet to a `MarkdownStyle` struct.
    Stylesheet(Stylesheet),
    /// if you're really only designing for Markdown and/or Terminal
    /// target then you can just pass in `MarkdownStyle` who's shape
    /// is designed to slot into a Markdown page's `style` property.
    MarkdownStyle(MarkdownStyle),
}

/// Both render functions associated with the `MarkdownRenderable` trait allow
/// the caller to pass in a set of options to effect rendering output.
pub struct MarkdownOptions {
    pub style: Option<MarkdownStyleOptions>,
    pub features: Option<Vec<PageFeature>>,
    pub metadata: Option<HashMap<MicrodataKey, String>>,
}

/// A component capable of rendering itself as Markdown output.
///
/// Markdown is a superset of HTML which means strictly speaking, it can
/// contain as much inline HTML as you like but there are good reasons to
/// keep this to a minimum:
///
/// 1. Every HTML tag makes the authoring experience
pub trait MarkdownRenderable {
    /// Renders the component as a Markdown string.
    fn render_markdown(&self, style: Option<MarkdownOptions>) -> String;

    /// Renders the component as a Markdown content with an emphasis on functionality
    /// over ergonomics.
    fn render_markdown_plus(&self, style: Option<MarkdownOptions>) -> String;
}
