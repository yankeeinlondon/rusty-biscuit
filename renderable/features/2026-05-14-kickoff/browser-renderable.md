
```rust
// the update API surface
pub trait BrowserRenderable: std::fmt::Debug + Any {
    // pre-existing
    fn render_to_browser(&self) -> String;
    fn render_to_browser_with_inline_variables(&self,_variables: &HashMap<String, String>) -> String;
    // adding for future API surface
    fn render_html_fragment(&self) -> BrowserFragment;
    fn render_html_page(&self) -> String;
}

// important to the new structured view on browser rendering
pub struct BrowserFragment {
    pub body: String,
    pub javascript: Vec<JavascriptDependency>, 
    pub meta: Vec<MetaProperties>,
    pub style: Vec<CssDependency>
}

// must implement `Default` trait and adds
// builder pattern to add in metadata, links, or
// inline CSS/JS.
impl BrowserFragment {
    pub fn new() -> BrowserFragment;
    
    // builders
    
    /// allows setting meta data once as key/value pairs that are translated into
    /// into all common micro-data formats for discoverability
    pub fn with_meta_kv<T: into String, U: into Value>(&self, name: T, value: U) -> self;
    /// allows a javascript code block to be added to the "stack" of Javascript needed on the page
    pub fn with_inline_js<T: into Javascript>(code: T) -> self;
    pub fn with_inline_css<T: into Stylesheet>(&self, code: T) -> self;
    pub fn with_css_link(link) -> self;
    pub fn with_js_link(link) -> self;

    /// FUTURE: we don't yet have a library of "available" JS features but we will
    pub fn with_js_feature(feature: JsFeature) -> self;
}

/// Composes one or more BrowserFragment's into a correctly structured HTML page
pub struct HtmlPage {
    fragments: Vec<BrowserFragment>
}

impl From<Vec<String>> for HtmlPage {
    from() -> HtmlPage
}

impl HtmlPage {
    pub fn new<T: into BrowserFragment>(fragment: T) -> self;
}
```
