use crate::{
    browser::{BrowserFragment, CodeFeature, HtmlStyleSheet},
    html::tag::{link::LinkTag, meta::MetaTag},
    microdata::MicrodataKey,
};

pub mod attribute;
pub mod cors;
pub mod script;
pub mod tag;

pub struct HtmlPage {
    stylesheet: HtmlStyleSheet,
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

impl Default for HtmlPage {
    fn default() -> HtmlPage {
        HtmlPage {
            stylesheet: HtmlStyleSheet::new(),
            links: vec![],
            script_blocks: vec![],
            meta: vec![],
            title: None,
            features: vec![],
            fragments: vec![],
            css_variables: None,
        }
    }
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
    pub fn new(style: Option<HtmlStyleSheet>) -> HtmlPage {
        HtmlPage {
            stylesheet: style.unwrap_or(HtmlStyleSheet::new()),
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
    pub fn add_metadata(&mut self, key: MicrodataKey, value: String) -> &mut HtmlPage {
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
        let push_if_new = |link: &'_ LinkTag,
                           seen: &mut std::collections::HashSet<String>,
                           out: &mut Vec<&'_ LinkTag>| {
            let key = link.dedup_key();
            if seen.insert(key) {
                out.push(link);
            }
        };
        // Page-level links first (they win on ordering ties).
        for link in &self.links {
            push_if_new(link, &mut seen, &mut out);
        }
        // Then dependencies from each fragment in registration order.
        for fragment in &self.fragments {
            for link in &fragment.dependency_links {
                push_if_new(link, &mut seen, &mut out);
            }
        }
        out
    }
}
