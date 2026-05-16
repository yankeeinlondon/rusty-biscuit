use std::collections::HashMap;

use crate::{
    browser::{PageOptions, fragment::BrowserFragment, fragment::Ready, feature::PageFeature},
    html::tag::{link::LinkTag, meta::MetaTag},
    microdata::MicrodataKey,
    stylesheet::Stylesheet,
};

pub mod attribute;
pub mod cors;
pub mod script;
pub mod tag;

/// A fully-assembled HTML page: a tree of fragments plus page-level
/// `<head>` state.
///
/// Per decisions.md item 8, `HtmlPage` does not store a `title` field —
/// [`set_title`](HtmlPage::set_title) writes the `Title` microdata key so
/// the title fans out into HTML / OpenGraph / Twitter / Schema.org tags.
/// `HtmlPage` owns all metadata; component metadata bubbles up the
/// fragment tree and page-level metadata wins on conflict.
pub struct HtmlPage {
    /// Page-level stylesheet. Wins over component defaults at equal
    /// specificity.
    stylesheet: Stylesheet,
    /// Page-level `<link>` tags. Deduped against fragment dependency
    /// links at render time.
    links: Vec<LinkTag>,
    /// Page-level inline `<script>` blocks.
    script_blocks: Vec<String>,
    /// Page-level `<meta>` tags not derived from microdata.
    #[allow(dead_code)]
    meta: Vec<MetaTag>,
    /// Page-level features rolled up from fragments and the caller.
    #[allow(dead_code)]
    features: Vec<PageFeature>,
    /// Page-level microdata. Page entries win over component entries on
    /// key conflict (decisions.md item 8).
    metadata: HashMap<MicrodataKey, String>,
    /// Owned fragments. A page is the natural lifetime root for the
    /// fragments it composes.
    fragments: Vec<BrowserFragment<Ready>>,
    /// `(variable_name, value)` overrides for the `:root` block. `None`
    /// means the page emits only semantic-token defaults.
    css_variables: Option<Vec<(String, String)>>,
    /// Inline-vs-external CSS choice. `Some(path)` → external `<link>`.
    external_stylesheet: Option<std::path::PathBuf>,
    /// Inline-vs-external JS choice. `Some(path)` → external `<script src>`.
    external_code: Option<std::path::PathBuf>,
}

impl Default for HtmlPage {
    fn default() -> HtmlPage {
        HtmlPage {
            stylesheet: Stylesheet::new(),
            links: Vec::new(),
            script_blocks: Vec::new(),
            meta: Vec::new(),
            features: Vec::new(),
            metadata: HashMap::new(),
            fragments: Vec::new(),
            css_variables: None,
            external_stylesheet: None,
            external_code: None,
        }
    }
}

impl From<BrowserFragment<Ready>> for HtmlPage {
    fn from(fragment: BrowserFragment<Ready>) -> HtmlPage {
        HtmlPage {
            fragments: vec![fragment],
            ..HtmlPage::default()
        }
    }
}

impl HtmlPage {
    /// Construct a page from an ordered list of fragments.
    pub fn from_fragments(fragments: Vec<BrowserFragment<Ready>>) -> HtmlPage {
        HtmlPage {
            fragments,
            ..HtmlPage::default()
        }
    }

    /// Append a fragment to the page body.
    pub fn add_fragment(&mut self, fragment: BrowserFragment<Ready>) -> &mut HtmlPage {
        self.fragments.push(fragment);
        self
    }

    /// Set the page title.
    ///
    /// Per decisions.md item 8 this writes the `Title` microdata key so
    /// the title fans out into HTML / OpenGraph / Twitter / Schema.org
    /// tags — one code path, no dedicated `title` field.
    pub fn set_title(&mut self, title: impl Into<String>) -> &mut HtmlPage {
        self.metadata.insert(MicrodataKey::Title, title.into());
        self
    }

    /// Add a page-level microdata key/value pair. Page-level entries win
    /// over component entries on key conflict.
    pub fn add_metadata(&mut self, key: MicrodataKey, value: impl Into<String>) -> &mut HtmlPage {
        self.metadata.insert(key, value.into());
        self
    }

    /// Add a `<link>` tag to `<head>`.
    pub fn add_link(&mut self, link: LinkTag) -> &mut HtmlPage {
        self.links.push(link);
        self
    }

    /// Add a page-level inline `<script>` block.
    pub fn add_script_block(&mut self, code_block: impl Into<String>) -> &mut HtmlPage {
        self.script_blocks.push(code_block.into());
        self
    }

    /// Apply a [`PageOptions`] to this page in place.
    ///
    /// Replaces the stylesheet when one is supplied, merges CSS-variable
    /// overrides, and records the inline-vs-external asset choices.
    pub fn apply_page_options(&mut self, options: PageOptions) -> &mut HtmlPage {
        if let Some(stylesheet) = options.stylesheet {
            self.stylesheet = stylesheet;
        }
        if let Some(variables) = options.css_variables {
            self.css_variables = Some(variables);
        }
        self.external_stylesheet = options.external_stylesheet;
        self.external_code = options.external_code;
        self
    }

    /// Dedups the page's own `<link>` tags **and** the dependency links
    /// pulled from every composed fragment, returning the unified list in
    /// first-seen order.
    ///
    /// Identity is [`LinkTag::dedup_key`] — `(rel, href)`. Page-level
    /// links are seen first and win ordering ties.
    pub fn collect_dedup_links(&self) -> Vec<&LinkTag> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for link in &self.links {
            if seen.insert(link.dedup_key()) {
                out.push(link);
            }
        }
        for fragment in &self.fragments {
            for link in fragment.dependency_links() {
                if seen.insert(link.dedup_key()) {
                    out.push(link);
                }
            }
        }
        out
    }

    /// Renders the page to a complete HTML string.
    ///
    /// Pure: never performs I/O. When [`PageOptions`] selected external
    /// assets, this emits `<link>` / `<script src>` references; the
    /// caller pulls content from [`stylesheet`](HtmlPage::stylesheet) /
    /// [`inline_code`](HtmlPage::inline_code) and writes it.
    pub fn render(&self) -> String {
        todo!("Phase E")
    }

    /// Returns the page's rolled-up CSS text (`:root` block + page
    /// stylesheet + component default stylesheets).
    pub fn stylesheet(&self) -> String {
        todo!("Phase E")
    }

    /// Returns the page's rolled-up JS text (page script blocks +
    /// per-fragment feature code).
    pub fn inline_code(&self) -> String {
        todo!("Phase E")
    }
}
