pub mod feature;
pub mod fragment;
pub mod renderable;
pub mod utils;

pub use feature::{
    DefaultFeatureResolver, FeatureAssets, FeatureContext, FeatureResolveError, FeatureResolver,
    FeatureScript, PageFeature,
};
pub use renderable::BrowserRenderable;

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::stylesheet::{CssRule, CssStyle, Stylesheet};

/// A scoped collection of CSS rulesets owned by a component.
///
/// The `name` is the component's wrapper class (e.g. `simple-table`).
/// Internal selectors registered on the component target elements
/// **within** that wrapper; the rendered output is a descendant
/// selector (`.<name> .<child>`).
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

    /// Register an internal class-scoped rule.
    ///
    /// `internal_class` is the bare class name (no leading `.`) of an
    /// element **within** the component wrapper. [`as_stylesheet`](Self::as_stylesheet)
    /// lowers it to the descendant selector `.<name> .<internal_class>`.
    pub fn add(
        mut self,
        internal_class: impl Into<String>,
        style: CssStyle,
    ) -> ComponentStylesheet {
        self.style.push(CssRule::new(internal_class, style));
        self
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
        let mut out = Stylesheet::new();
        for rule in self.style.entries() {
            let scoped = format!(".{} .{}", self.name, rule.selector);
            out.push(CssRule::new(scoped, rule.style.clone()));
        }
        out
    }
}

/// Caller-supplied options that shape page assembly and rendering.
///
/// Per decisions.md item 6, `PageOptions` carries no `Layout`: page
/// background, margins, and padding are expressed through the page
/// [`Stylesheet`] (rulesets on `html` / `body`). Per item 7, the
/// inline-vs-external asset choice lives here — `None` means inline.
#[derive(Debug, Clone, Default)]
pub struct PageOptions {
    /// Page-level stylesheet. Wins over component defaults at equal
    /// specificity. `None` leaves the page with only component styles.
    pub stylesheet: Option<Stylesheet>,
    /// Ordered `(variable_name, value)` overrides for the page-level
    /// `:root` block. A name present here replaces the semantic-token
    /// default; names not listed keep their default.
    pub css_variables: Option<Vec<(String, String)>>,
    /// When `Some`, the page's rolled-up CSS is emitted as an external
    /// `<link href="…">` instead of an inline `<style>`. The path type
    /// guarantees it is relative so the `href` stays portable.
    pub external_stylesheet: Option<RelativeAssetPath>,
    /// When `Some`, the page's rolled-up JS is emitted as an external
    /// `<script src="…">` instead of an inline `<script>`. The path type
    /// guarantees it is relative so the `src` stays portable.
    pub external_code: Option<RelativeAssetPath>,
}

/// A filesystem path guaranteed to be relative.
///
/// External page assets (`<link href>`, `<script src>`) must be relative
/// so the emitted HTML stays portable across hosting locations
/// (decisions.md item 7). [`RelativeAssetPath::new`] is the single
/// validation point — once a caller holds a `RelativeAssetPath`, page
/// assembly and rendering are infallible, so [`PageOptions`] can be
/// applied without a `Result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeAssetPath(PathBuf);

impl RelativeAssetPath {
    /// Wraps a relative path.
    ///
    /// ## Errors
    ///
    /// Returns [`PageOptionsError::AbsoluteAssetPath`] when `path` is
    /// absolute — external asset paths must be relative.
    pub fn new(path: impl Into<PathBuf>) -> Result<RelativeAssetPath, PageOptionsError> {
        let path = path.into();
        if path.has_root() {
            return Err(PageOptionsError::AbsoluteAssetPath(path));
        }
        Ok(RelativeAssetPath(path))
    }

    /// The wrapped relative path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Error raised when constructing a [`RelativeAssetPath`] from an invalid
/// path.
#[derive(Debug, Error)]
pub enum PageOptionsError {
    /// An external asset path was absolute. External `<link href>` /
    /// `<script src>` paths must be relative so the emitted HTML stays
    /// portable across hosting locations (decisions.md item 7).
    #[error("external asset path must be relative, got absolute path: {0}")]
    AbsoluteAssetPath(PathBuf),
}
