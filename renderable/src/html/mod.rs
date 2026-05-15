use crate::{
    browser::{BrowserFragment, CodeFeature, HtmlStyleSheet},
    html::{link::LinkTag, meta::MetaTag},
    microdata::MicrodataKey,
};

pub mod cors;
pub mod link;
pub mod meta;
pub mod script;

pub struct HtmlPage {
    stylesheet: HtmlStyleSheet,
    links: Vec<LinkTag>,
    script_blocks: Vec<String>,
    meta: Vec<MetaTag>,
    title: Option<String>,
    features: Vec<CodeFeature>,
    fragments: Vec<&BrowserFragment>,
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
        }
    }
}

impl From<&BrowserFragment> for HtmlPage {
    fn from(fragment: &BrowserFragment) -> HtmlPage {
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

    pub fn from_fragments(fragments: Vec<&BrowserFragment>) -> HtmlPage {
        HtmlPage {
            fragments: fragments,
            ..HtmlPage::default()
        }
    }

    // builder

    /// adds microdata key/value pairs
    pub fn add_metadata(&self, key: MicrodataKey, value: String) -> self {
        todo!()
    }

    /// adds a `<link>` tag to the head section
    pub fn add_link(&mut self, link: LinkTag) -> &mut HtmlPage {
        self.links.push(link);
        self
    }

    pub fn add_script_block(&self, code_block: String) -> &mut HtmlPage {
        todo!()
    }
}
