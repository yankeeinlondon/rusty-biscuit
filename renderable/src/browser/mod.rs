pub mod feature;
pub mod fragment;
pub mod utils;
pub mod renderable;

pub use renderable::BrowserRenderable;

/// Placeholder type until the real Stylesheet is moved from darkmatter in Phase C.
pub struct Stylesheet;

/// Placeholder type until Layout is moved from biscuit-terminal in Phase D.
pub struct Layout;

/// A collection of CSS rulesets keyed by **selector** in declaration order.
///
/// Each entry is a `(selector, Stylesheet)` pair. The order is preserved
/// because CSS cascade resolves ties in source order, and silently dropping
/// duplicate selectors (as `HashMap` would) breaks the override model that
/// lets later rules win.
///
/// ## Notes
///
/// - Selectors are stored as strings rather than a typed selector model;
///   the rendering layer is responsible for emitting them verbatim.
/// - For class-scoped collections produced by [`ComponentStylesheet::as_stylesheet`],
///   selectors are descendant selectors like `.simple-table .col-string`.
/// - Page assembly may dedup or merge entries with identical selectors,
///   but this type does not enforce it.
pub struct HtmlStyleSheet(Vec<(String, Stylesheet)>);

impl Default for HtmlStyleSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlStyleSheet {
    pub fn new() -> HtmlStyleSheet {
        HtmlStyleSheet(Vec::new())
    }

    /// Append a `(selector, Stylesheet)` entry, preserving order.
    pub fn push(&mut self, selector: impl Into<String>, sheet: Stylesheet) -> &mut Self {
        self.0.push((selector.into(), sheet));
        self
    }

    pub fn entries(&self) -> &[(String, Stylesheet)] {
        &self.0
    }
}

/// A scoped collection of CSS rulesets owned by a component.
///
/// The `name` is the component's wrapper class (e.g. `simple-table`).
/// Internal selectors registered via [`ComponentStylesheet::add`] target
/// elements **within** that wrapper; the rendered output is a descendant
/// selector (`.<name> .<child>`).
#[allow(dead_code)]
pub struct ComponentStylesheet {
    name: String,
    style: HtmlStyleSheet,
}

impl ComponentStylesheet {
    pub fn new<T: Into<String>>(name: T) -> ComponentStylesheet {
        ComponentStylesheet {
            name: name.into(),
            style: HtmlStyleSheet::new(),
        }
    }

    /// Returns the component's wrapper class name (without the leading `.`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Lowers internal class selectors into fully-qualified **descendant**
    /// selectors scoped under the component's wrapper class.
    ///
    /// ## Example
    ///
    /// Given:
    /// - component name `"simple-table"`,
    /// - an entry mapping the internal class `"col-string"` to a
    ///   `Stylesheet` of `{ text-align: left; }`,
    ///
    /// this returns an [`HtmlStyleSheet`] containing:
    ///
    /// ```css
    /// .simple-table .col-string { text-align: left; }
    /// ```
    ///
    /// The original (unscoped) entries are not retained in the output; only
    /// the scoped selectors. Order is preserved.
    pub fn as_stylesheet(&self) -> HtmlStyleSheet {
        todo!()
    }
}

#[allow(dead_code)]
pub struct PageOptions {
    /// Cross-target layout settings (margins, alignment, page bg color).
    /// `Layout` will live in `renderable::layout` once the layout-move spec lands.
    layout: Option<Layout>,
    /// Page-level stylesheet that wins over component defaults at equal specificity.
    stylesheet: Option<HtmlStyleSheet>,
    /// Ordered `(variable_name, value)` pairs emitted as `:root { --name: value; … }`.
    /// `value` is a raw CSS expression string; once the stylesheet-move lands this
    /// can tighten to a typed `CssValue`.
    css_variables: Option<Vec<(String, String)>>,
}
