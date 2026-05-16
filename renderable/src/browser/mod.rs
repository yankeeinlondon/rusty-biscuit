pub mod feature;
pub mod fragment;
pub mod utils;
pub mod renderable;

pub use renderable::BrowserRenderable;

use crate::stylesheet::Stylesheet;

/// A scoped collection of CSS rulesets owned by a component.
///
/// The `name` is the component's wrapper class (e.g. `simple-table`).
/// Internal selectors registered on the component target elements
/// **within** that wrapper; the rendered output is a descendant
/// selector (`.<name> .<child>`).
#[allow(dead_code)]
pub struct ComponentStylesheet {
    name: String,
    style: Stylesheet,
}

impl ComponentStylesheet {
    pub fn new<T: Into<String>>(name: T) -> ComponentStylesheet {
        ComponentStylesheet {
            name: name.into(),
            style: Stylesheet::new(),
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
    ///   declaration block of `{ text-align: left; }`,
    ///
    /// this returns a [`Stylesheet`] containing:
    ///
    /// ```css
    /// .simple-table .col-string { text-align: left; }
    /// ```
    ///
    /// The original (unscoped) entries are not retained in the output; only
    /// the scoped selectors. Order is preserved.
    pub fn as_stylesheet(&self) -> Stylesheet {
        todo!()
    }
}

/// Caller-supplied options that shape page assembly and rendering.
#[allow(dead_code)]
pub struct PageOptions {
    /// Page-level stylesheet that wins over component defaults at equal
    /// specificity.
    stylesheet: Option<Stylesheet>,
    /// Ordered `(variable_name, value)` pairs emitted as
    /// `:root { --name: value; … }`. Order is preserved because CSS cascade
    /// resolves ties in source order.
    css_variables: Option<Vec<(String, String)>>,
}
