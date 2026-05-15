use std::collections::HashMap;

use crate::{html::link::LinkTag, microdata::MicrodataKey};

/// Keys represent the **class** definitions and the values are CSS key/values
pub struct HtmlStyleSheet(HashMap<String, Stylesheet>);

impl HtmlStyleSheet {
    pub fn new() -> HtmlStyleSheet {
        HtmlStyleSheet(HashMap::new())
    }
}

/// A component stylesheet must provide a name that
/// matches the **class** name used on the wrapper/parent
/// tag of the component's
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

    /// injects the component style sheet's name into the keys
    /// of the stylesheet and returns as a plain `HtmlStyleSheet`.
    ///
    /// Example:
    /// - if the component style sheet has a name of `simple-table`
    /// - and the style sheet has a configuration of: `{ "color": "red" }`
    /// - running this command will return an HtmlStyleSheet with a
    ///   configuration of `{ "simple-table color": "red" }
    ///
    /// This provides namespacing support for the default styles of
    ///
    pub fn as_stylesheet(&self) -> HtmlStyleSheet {
        todo!()
    }
}

/// The key output of a `BrowserRenderable` component which
/// represents an HTML fragment along with page-level attributes
/// that this fragment expects to be included in the page when it's
/// rendered.
pub struct BrowserFragment {
    /// The HTML fragment
    pub body: Option<String>,
    /// A stylesheet defining the classes which this component uses for it's default values
    pub stylesheet: Option<ComponentStylesheet>,
    /// FUTURE: will allow reusable code blocks to be included on a page
    pub code_features: Vec<CodeFeature>,

    /// stores key/value pairs of metadata that will be converted to HTML
    /// microdata at runtime.
    ///
    /// Note: this is more typically set at the page level but if a component
    /// has a strong view on what the page's metadata should be then it can
    /// express it and it will be honored unless the page overrides the value
    /// for the properties set by the component
    metadata: HashMap<MicrodataKey, String>,

    pub dependency_links: Vec<LinkTag>,
}

impl Default for BrowserFragment {
    fn default() -> BrowserFragment {
        BrowserFragment {
            body: None,
            stylesheet: None,
            code_features: vec![],
            metadata: HashMap::new(),
            dependency_links: vec![],
        }
    }
}

impl BrowserFragment {
    pub fn new(body: Option<String>) -> BrowserFragment {
        BrowserFragment {
            body,
            ..BrowserFragment::default()
        }
    }

    pub fn set_body(&mut self, content: String) -> &mut BrowserFragment {
        todo!()
    }

    pub fn set_default_styles(style: ComponentStylesheet) -> &mut BrowserFragment {
        todo!()
    }

    pub fn add_metadata_keypair<T: Into<String>>(
        &mut self,
        key: MicrodataKey,
        value: T,
    ) -> &mut BrowserFragment {
        self.metadata.insert(key, value.into());
        self
    }

    pub fn add_linked_dependency(&mut self, link: LinkTag) -> &mut BrowserFragment {
        self.dependency_links.push(link);
        self
    }
}

/// enumerates all of the reusable code blocks which can be added to a
/// HTML page.
pub enum CodeFeature {}
