use crate::{
    browser::{fragment::BrowserFragment, feature::CodeFeature},
    html::tag::{link::LinkTag, meta::MetaTag},
    microdata::MicrodataKey,
    stylesheet::Stylesheet,
};

pub mod attribute;
pub mod cors;
pub mod script;
pub mod tag;

#[derive(Default)]
#[allow(dead_code)]
pub struct HtmlPage {
    stylesheet: Stylesheet,
    links: Vec<LinkTag>,
    script_blocks: Vec<String>,
    meta: Vec<MetaTag>,
    title: Option<String>,
    features: Vec<CodeFeature>,
    /// Owned fragments. A page is the natural lifetime root for the fragments
    /// it composes; borrowing here forces every caller into lifetime management
    /// for no benefit.
    fragments: Vec<BrowserFragment>,
    /// Ordered `(variable_name, value)` pairs emitted as `:root { --name: value; … }`.
    /// Order is preserved because CSS cascade resolves ties in source order.
    /// `value` is a raw CSS expression string; tightens to a typed `CssValue`
    /// once the stylesheet-move lands.
    css_variables: Option<Vec<(String, String)>>,
}

impl From<BrowserFragment> for HtmlPage {
    fn from(fragment: BrowserFragment) -> HtmlPage {
        HtmlPage {
            fragments: vec![fragment],
            ..HtmlPage::default()
        }
    }
}

impl HtmlPage {
    // input
    pub fn new(style: Option<Stylesheet>) -> HtmlPage {
        HtmlPage {
            stylesheet: style.unwrap_or_default(),
            ..HtmlPage::default()
        }
    }

    pub fn from_fragments(fragments: Vec<BrowserFragment>) -> HtmlPage {
        HtmlPage {
            fragments,
            ..HtmlPage::default()
        }
    }

    // builder

    /// Adds microdata key/value pairs that fan out into HTML / OpenGraph /
    /// Twitter / Schema.org meta tags at render time (see [`crate::microdata::microdata`]).
    pub fn add_metadata(&mut self, _key: MicrodataKey, _value: String) -> &mut HtmlPage {
        todo!()
    }

    /// Adds a `<link>` tag to the head section.
    pub fn add_link(&mut self, link: LinkTag) -> &mut HtmlPage {
        self.links.push(link);
        self
    }

    pub fn add_script_block(&mut self, code_block: String) -> &mut HtmlPage {
        self.script_blocks.push(code_block);
        self
    }

    /// Dedups the page's own `<link>` tags **and** the dependency_links pulled
    /// in from every composed fragment, returning the unified, deduplicated
    /// list in first-seen order.
    ///
    /// Identity is determined by `LinkTag::dedup_key` (typically `(rel, href)`),
    /// not by structural equality — two link tags with the same `href` but
    /// different `media` queries are treated as duplicates and the first one
    /// wins.
    ///
    /// Called by the (future) `render()` step; not part of the builder surface.
    pub fn collect_dedup_links(&self) -> Vec<&LinkTag> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        // Page-level links first (they win on ordering ties).
        for link in &self.links {
            let key = link.dedup_key();
            if seen.insert(key) {
                out.push(link);
            }
        }
        // Then dependencies from each fragment in registration order.
        for fragment in &self.fragments {
            for link in &fragment.dependency_links {
                let key = link.dedup_key();
                if seen.insert(key) {
                    out.push(link);
                }
            }
        }
        out
    }
}
