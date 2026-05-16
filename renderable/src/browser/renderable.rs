use std::any::Any;
use std::collections::HashMap;

use crate::browser::PageOptions;
use crate::browser::fragment::{BrowserFragment, Ready};
use crate::html::HtmlPage;

/// A component capable of rendering itself for browser display.
///
/// During the Project 2 coexistence window this trait carries four
/// methods. The first two are the legacy string-producing surface,
/// deprecated and removed in Project 3. The last two are the new
/// structural surface and ship with default implementations so existing
/// implementors are not burdened until they choose to migrate.
///
/// ## Notes
///
/// - `render_html_fragment` returns the typestate
///   [`BrowserFragment<Ready>`] — the universal "done" currency for
///   composition (see decisions.md item 1).
/// - `render_html_page` returns an [`HtmlPage`], not a `String`; the
///   caller then calls [`HtmlPage::render`] for the final string (see
///   decisions.md item 7).
/// - The default `render_html_fragment` wraps the legacy
///   `render_to_browser()` output in a
///   [`ComposableNode::RawHtml`](crate::browser::fragment::ComposableNode::RawHtml)
///   fragment, matching the one-line migration shim in decisions.md
///   item 12B.
pub trait BrowserRenderable: std::fmt::Debug + Any {
    /// Renders the component to browser-compatible HTML/SVG.
    ///
    /// Deprecated — removed in Project 3. New code implements
    /// [`render_html_fragment`](BrowserRenderable::render_html_fragment).
    fn render_to_browser(&self) -> String;

    /// Renders the component with inline CSS-variable substitution.
    ///
    /// Deprecated — removed in Project 3. The default ignores
    /// `variables` and calls [`render_to_browser`](BrowserRenderable::render_to_browser).
    fn render_to_browser_with_inline_variables(
        &self,
        _variables: &HashMap<String, String>,
    ) -> String {
        self.render_to_browser()
    }

    /// Produces a fully-composed [`BrowserFragment<Ready>`] for this
    /// component.
    ///
    /// The default implementation wraps `render_to_browser()` output as
    /// caller-owned raw HTML. Components migrate by overriding this with
    /// a typed-node build.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserFragment::new()
            .define_as_raw_html(self.render_to_browser())
            .finalize()
    }

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
