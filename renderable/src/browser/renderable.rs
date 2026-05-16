use std::any::Any;

use crate::browser::PageOptions;
use crate::browser::fragment::{BrowserFragment, Ready};
use crate::html::HtmlPage;

/// A component capable of rendering itself for browser display.
///
/// The trait surface is structural: a component produces a
/// [`BrowserFragment<Ready>`] and can optionally be promoted to a
/// standalone [`HtmlPage`]. There is no legacy string-producing surface.
///
/// ## Notes
///
/// - `render_html_fragment` is the single required method. It returns the
///   typestate [`BrowserFragment<Ready>`] — the universal "done" currency
///   for composition (see decisions.md item 1).
/// - `render_html_page` ships with a default that builds an `HtmlPage`
///   from `render_html_fragment`; it returns an [`HtmlPage`], not a
///   `String`, so the caller then calls [`HtmlPage::render`] for the
///   final string (see decisions.md item 7).
/// - `as_any` enables downcasting to the concrete component type.
pub trait BrowserRenderable: std::fmt::Debug + Any {
    /// Produces a fully-composed [`BrowserFragment<Ready>`] for this
    /// component.
    ///
    /// Components that emit prebuilt markup (SVG, third-party HTML) wrap
    /// it as a
    /// [`ComposableNode::RawHtml`](crate::browser::fragment::ComposableNode::RawHtml)
    /// island; components built from typed nodes assemble a node tree.
    fn render_html_fragment(&self) -> BrowserFragment<Ready>;

    /// Promotes this single component to a standalone [`HtmlPage`].
    ///
    /// The default builds an `HtmlPage` from this component's fragment
    /// and applies `page` when supplied. Infallible: external asset paths
    /// in [`PageOptions`] are validated when their
    /// [`RelativeAssetPath`](crate::browser::RelativeAssetPath) is
    /// constructed, so applying the options here cannot fail.
    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn Any;
}
