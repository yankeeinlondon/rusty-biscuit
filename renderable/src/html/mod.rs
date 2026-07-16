use std::collections::HashMap;
use std::rc::Rc;

use crate::{
    browser::{
        PageOptions, RelativeAssetPath,
        feature::{DefaultFeatureResolver, FeatureContext, FeatureResolver, PageFeature},
        fragment::{BrowserFragment, ComposableNode, Ready},
    },
    html::tag::{BlockTag, link::LinkTag, meta::MetaTag},
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
    external_stylesheet: Option<RelativeAssetPath>,
    /// Inline-vs-external JS choice. `Some(path)` → external `<script src>`.
    external_code: Option<RelativeAssetPath>,
    /// Resolves requested [`PageFeature`]s to their assets. Defaults to
    /// [`DefaultFeatureResolver`]; Darkmatter installs its own. Consumed by
    /// [`resolved_feature_head`](HtmlPage::resolved_feature_head) when
    /// [`render`](HtmlPage::render) assembles the page `<head>`.
    feature_resolver: Rc<dyn FeatureResolver>,
    /// Renderable-owned context threaded into feature resolution.
    feature_context: FeatureContext,
    /// Memoized [`resolved_feature_head`](HtmlPage::resolved_feature_head)
    /// output. `render_browser_document` resolves eagerly to surface unresolved
    /// features at its fallible entry point; caching the result lets the final
    /// [`render`](HtmlPage::render) reuse it so a possibly-impure
    /// [`FeatureResolver`] runs exactly once per feature per page. Cleared by
    /// every `&mut self` method that changes the resolver, context, or fragment
    /// set (the resolution inputs).
    resolved_head_cache: std::cell::RefCell<Option<String>>,
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
            feature_resolver: Rc::new(DefaultFeatureResolver),
            feature_context: FeatureContext::default(),
            resolved_head_cache: std::cell::RefCell::new(None),
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
        self.invalidate_resolved_head_cache();
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

    /// Install the feature resolver used to turn requested
    /// [`PageFeature`]s into assets. Defaults to [`DefaultFeatureResolver`].
    pub fn set_feature_resolver(&mut self, resolver: Rc<dyn FeatureResolver>) -> &mut HtmlPage {
        self.feature_resolver = resolver;
        self.invalidate_resolved_head_cache();
        self
    }

    /// Install the renderable-owned [`FeatureContext`] threaded into feature
    /// resolution.
    pub fn set_feature_context(&mut self, context: FeatureContext) -> &mut HtmlPage {
        self.feature_context = context;
        self.invalidate_resolved_head_cache();
        self
    }

    /// Drops any memoized [`resolved_feature_head`](HtmlPage::resolved_feature_head)
    /// output so the next resolution reflects changed inputs. Called by every
    /// `&mut self` method that touches a resolution input (resolver, context, or
    /// fragment set).
    fn invalidate_resolved_head_cache(&mut self) {
        *self.resolved_head_cache.borrow_mut() = None;
    }

    /// Resolves this page's rolled-up [`features`](HtmlPage::features) through
    /// the installed [`FeatureResolver`] for the Browser target and serializes
    /// the resulting `<head>` assets for [`render`](HtmlPage::render) to append.
    ///
    /// Assets are emitted in first-seen feature order, after the page's authored
    /// links / styles / scripts, so a no-feature page yields an empty string and
    /// is byte-for-byte unchanged. This is the single feature-injection point for
    /// the fragment/`HtmlPage` browser path; the streaming full-document
    /// assembler resolves its own accumulated features directly.
    ///
    /// ## Errors
    ///
    /// - [`FeatureResolveError::UnresolvedFeature`] when a requested browser
    ///   feature has no assets under the installed resolver. Dropping a browser
    ///   dependency silently is forbidden.
    ///
    /// [`FeatureResolveError::UnresolvedFeature`]: crate::browser::feature::FeatureResolveError::UnresolvedFeature
    pub(crate) fn resolved_feature_head(
        &self,
    ) -> Result<String, crate::browser::feature::FeatureResolveError> {
        // Memoized so an eager caller (`render_browser_document`) and the final
        // `render` share one resolution — a `FeatureResolver` is not required to
        // be pure, so double invocation could do redundant work or diverge. The
        // cache holds the serialized head (including the empty-features result)
        // and is cleared whenever a resolution input changes.
        if let Some(cached) = self.resolved_head_cache.borrow().as_ref() {
            return Ok(cached.clone());
        }
        let features = self.features();
        let head = if features.is_empty() {
            String::new()
        } else {
            let resolved = crate::browser::feature::resolve_features(
                &features,
                self.feature_resolver.as_ref(),
                crate::target::RenderTarget::Browser,
                &self.feature_context,
            )?;
            crate::browser::feature::serialize_features_head(&resolved)
        };
        *self.resolved_head_cache.borrow_mut() = Some(head.clone());
        Ok(head)
    }

    /// Apply a [`PageOptions`] to this page in place.
    ///
    /// Replaces the stylesheet when one is supplied, merges CSS-variable
    /// overrides, and records the inline-vs-external asset choices. This
    /// is infallible: external asset paths are validated when the
    /// [`RelativeAssetPath`] is constructed, so by the time `PageOptions`
    /// reaches this method nothing can be rejected.
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
    /// The fragment scan is recursive: dependency links contributed by
    /// components nested inside other components are collected too.
    /// Identity is [`LinkTag::dedup_key`] — `(rel, href)`. Page-level
    /// links are seen first and win ordering ties.
    pub fn collect_dedup_links(&self) -> Vec<&LinkTag> {
        self.collect_dedup_links_from(&self.all_fragments())
    }

    /// [`collect_dedup_links`](HtmlPage::collect_dedup_links) over an
    /// already-collected fragment list, so [`render_head`](HtmlPage::render_head)
    /// can share one [`all_fragments`](HtmlPage::all_fragments) traversal across
    /// the link, metadata, and stylesheet rollups instead of walking the
    /// fragment tree once per rollup.
    fn collect_dedup_links_from<'a>(
        &'a self,
        all_fragments: &[&'a BrowserFragment<Ready>],
    ) -> Vec<&'a LinkTag> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for link in &self.links {
            if seen.insert(link.dedup_key()) {
                out.push(link);
            }
        }
        for fragment in all_fragments {
            for link in fragment.dependency_links() {
                if seen.insert(link.dedup_key()) {
                    out.push(link);
                }
            }
        }
        out
    }

    /// Returns every page fragment plus every component fragment nested
    /// anywhere within their node trees, in document order (a parent
    /// precedes its descendants).
    ///
    /// This is the recursion point that prevents nested-component
    /// auxiliary state (stylesheets, dependency links, metadata,
    /// features) from being silently lost during page assembly.
    fn all_fragments(&self) -> Vec<&BrowserFragment<Ready>> {
        let mut out = Vec::new();
        for fragment in &self.fragments {
            collect_fragments(fragment, &mut out);
        }
        out
    }

    /// Builds the effective metadata map over an already-collected fragment
    /// list — see [`collect_dedup_links_from`](HtmlPage::collect_dedup_links_from).
    ///
    /// Component metadata is collected in document order with **first-write
    /// wins** semantics — when two components set the same key, the
    /// earlier-positioned component's value is kept. Page-level metadata is
    /// applied last and **overwrites** any component value, so the page always
    /// wins on key conflict (decisions.md item 8).
    fn merged_metadata_from(
        &self,
        all_fragments: &[&BrowserFragment<Ready>],
    ) -> HashMap<MicrodataKey, String> {
        let mut merged = HashMap::new();
        for fragment in all_fragments {
            for (key, value) in fragment.metadata() {
                merged.entry(*key).or_insert_with(|| value.clone());
            }
        }
        for (key, value) in &self.metadata {
            merged.insert(*key, value.clone());
        }
        merged
    }

    /// Returns the page's features rolled up from the page itself and
    /// every fragment (recursively, through nested components), in
    /// first-seen order with duplicates removed.
    pub fn features(&self) -> Vec<PageFeature> {
        let mut out: Vec<PageFeature> = Vec::new();
        for &feature in &self.features {
            if !out.contains(&feature) {
                out.push(feature);
            }
        }
        for fragment in self.all_fragments() {
            for &feature in fragment.features() {
                if !out.contains(&feature) {
                    out.push(feature);
                }
            }
        }
        out
    }

    /// Returns the page's rolled-up CSS text.
    ///
    /// Order: `:root` semantic-token block, then the page stylesheet,
    /// then each fragment's component default stylesheet in
    /// fragment-registration order.
    pub fn stylesheet(&self) -> String {
        self.stylesheet_from(&self.all_fragments())
    }

    /// [`stylesheet`](HtmlPage::stylesheet) over an already-collected fragment
    /// list — see [`collect_dedup_links_from`](HtmlPage::collect_dedup_links_from).
    fn stylesheet_from(&self, all_fragments: &[&BrowserFragment<Ready>]) -> String {
        let mut out = String::new();
        out.push_str(&self.render_root_block());
        out.push_str(&render_stylesheet(&self.stylesheet));
        for fragment in all_fragments {
            if let Some(component_sheet) = fragment.stylesheet() {
                out.push_str(&render_stylesheet(&component_sheet.as_stylesheet()));
            }
        }
        out
    }

    /// Returns the page's rolled-up JS text (page-level script blocks,
    /// joined by blank lines).
    pub fn inline_code(&self) -> String {
        self.script_blocks.join("\n\n")
    }

    /// Renders the page to a complete HTML string, resolving and injecting the
    /// page's requested browser [`features`](HtmlPage::features) into `<head>`.
    /// Pure — no I/O.
    ///
    /// Feature assets are resolved through the installed [`FeatureResolver`] and
    /// appended after the page's authored links / styles / scripts, so a
    /// no-feature page is byte-for-byte unchanged and cannot fail.
    ///
    /// ## Errors
    ///
    /// - [`FeatureResolveError::UnresolvedFeature`] when a requested browser
    ///   feature has no assets under the installed [`FeatureResolver`]. Dropping a
    ///   declared browser dependency silently is forbidden.
    ///
    /// [`FeatureResolveError::UnresolvedFeature`]: crate::browser::feature::FeatureResolveError::UnresolvedFeature
    pub fn render(&self) -> Result<String, crate::browser::feature::FeatureResolveError> {
        let head = self.render_head(self.first_h1_text().as_deref());
        let feature_head = self.resolved_feature_head()?;
        // One allocator threaded across every fragment so prompted-link popover
        // ids stay document-unique even when the fragments were rendered
        // independently and composed here (spec criterion 7).
        let mut popover_ids = crate::browser::fragment::PopoverIdAllocator::default();
        let body: String = self
            .fragments
            .iter()
            .map(|f| f.render_with(&mut popover_ids))
            .collect();
        Ok(format!(
            "<!DOCTYPE html><html><head>{head}{feature_head}</head><body>{body}</body></html>"
        ))
    }

    /// Builds the page `<head>` contents (everything between `<head>` and
    /// `</head>`), in the fixed order: charset, viewport, title, microdata
    /// meta tags, deduped links, stylesheet, then script blocks. Resolved
    /// feature assets are appended by [`render`](HtmlPage::render) after this
    /// authored head, so both the fragment path and the streaming full-document
    /// path ([`render_browser_document_html`]) place them identically.
    ///
    /// `fallback_title` supplies the `<title>` when no `Title` microdata key is
    /// present; [`render`](HtmlPage::render) passes [`first_h1_text`] so the
    /// title falls back to the first `<h1>`. The direct render-tree document
    /// renderer ([`render_browser_document_html`]) computes the equivalent
    /// first-`<h1>` text from the node tree and passes it here so the two paths
    /// emit byte-identical heads.
    ///
    /// [`first_h1_text`]: HtmlPage::first_h1_text
    /// [`render_browser_document_html`]: crate::tree::render::render_browser_document_html
    pub(crate) fn render_head(&self, fallback_title: Option<&str>) -> String {
        // Collect the fragment tree once and share it across the metadata,
        // link, and stylesheet rollups below — each of those used to walk the
        // whole fragment tree independently.
        let all_fragments = self.all_fragments();
        let mut head = String::new();
        // 1. charset — always first.
        head.push_str(r#"<meta charset="utf-8">"#);
        // 2. viewport.
        head.push_str(r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#);
        // 3. title — from the Title microdata key, else the fallback, else empty.
        // Metadata is the page/component merge so a nested component's
        // title or description is not lost.
        let metadata = self.merged_metadata_from(&all_fragments);
        let title = metadata
            .get(&MicrodataKey::Title)
            .map(String::as_str)
            .or(fallback_title)
            .unwrap_or_default();
        head.push_str(&format!(
            "<title>{}</title>",
            crate::browser::utils::escape_text(title)
        ));
        // 4. other microdata-driven meta tags.
        for (key, value) in &metadata {
            if *key == MicrodataKey::Title {
                continue;
            }
            for (_source, html) in crate::microdata::microdata(*key, value) {
                for tag in html {
                    head.push_str(&tag);
                }
            }
        }
        // 5. link tags — deduped, page-level first.
        for link in self.collect_dedup_links_from(&all_fragments) {
            head.push_str(&link.render());
        }
        // 6. stylesheet — external <link> or inline <style>.
        match &self.external_stylesheet {
            Some(path) => head.push_str(&format!(
                r#"<link rel="stylesheet" href="{}">"#,
                crate::browser::utils::escape_attribute(&path.as_path().display().to_string())
            )),
            None => {
                let css = self.stylesheet_from(&all_fragments);
                if !css.is_empty() {
                    head.push_str(&format!("<style>{css}</style>"));
                }
            }
        }
        // 7. script blocks — external <script src> or inline <script>.
        match &self.external_code {
            Some(path) => head.push_str(&format!(
                r#"<script src="{}" defer></script>"#,
                crate::browser::utils::escape_attribute(&path.as_path().display().to_string())
            )),
            None => {
                let code = self.inline_code();
                if !code.is_empty() {
                    head.push_str(&format!("<script>{code}</script>"));
                }
            }
        }
        head
    }

    /// Renders the page-level `:root { … }` block. Starts from the
    /// semantic-token defaults and applies any `css_variables` overrides.
    fn render_root_block(&self) -> String {
        let mut variables: Vec<(String, String)> = crate::tokens::root_defaults();
        if let Some(overrides) = &self.css_variables {
            for (name, value) in overrides {
                match variables.iter_mut().find(|(n, _)| n == name) {
                    Some(entry) => entry.1 = value.clone(),
                    None => variables.push((name.clone(), value.clone())),
                }
            }
        }
        let body: String = variables
            .iter()
            .map(|(name, value)| format!("--{name}: {value};"))
            .collect();
        format!(":root{{{body}}}")
    }

    /// Walks the fragment tree for the first `<h1>` and returns its
    /// concatenated text content. `RawHtml` islands are invisible to
    /// this scan (decisions.md item 8B).
    fn first_h1_text(&self) -> Option<String> {
        for fragment in &self.fragments {
            if let Some(node) = fragment.node()
                && let Some(text) = find_first_h1_text(node)
            {
                return Some(text);
            }
        }
        None
    }
}

/// Pushes `fragment` and every component fragment nested anywhere within
/// its node tree onto `out`, in document order (parent before children).
fn collect_fragments<'a>(
    fragment: &'a BrowserFragment<Ready>,
    out: &mut Vec<&'a BrowserFragment<Ready>>,
) {
    out.push(fragment);
    if let Some(node) = fragment.node() {
        collect_component_fragments(node, out);
    }
}

/// Walks a composable node tree, descending into `BlockTag` children and
/// recursing into every `Component` fragment found.
fn collect_component_fragments<'a>(
    node: &'a ComposableNode,
    out: &mut Vec<&'a BrowserFragment<Ready>>,
) {
    match node {
        ComposableNode::Component(child) => collect_fragments(child, out),
        ComposableNode::BlockTag(block) => {
            for child in &block.content.children {
                collect_component_fragments(child, out);
            }
        }
        ComposableNode::Popover(popover) => {
            for child in &popover.anchor_children {
                collect_component_fragments(child, out);
            }
        }
        _ => {}
    }
}

/// Serializes a `Stylesheet` collection into CSS rule text.
fn render_stylesheet(sheet: &Stylesheet) -> String {
    sheet
        .entries()
        .iter()
        .map(|rule| format!("{}{{{}}}", rule.selector, rule.style.to_css()))
        .collect()
}

/// Recursively searches a composable node tree for the first `<h1>` and
/// returns its concatenated text content. Returns `None` if no `<h1>` is
/// found in the typed tree (`RawHtml` islands are not scanned).
pub(crate) fn find_first_h1_text(node: &ComposableNode) -> Option<String> {
    match node {
        ComposableNode::BlockTag(block) => {
            if matches!(block.tag, BlockTag::H1) {
                return Some(collect_text(&block.content.children));
            }
            for child in &block.content.children {
                if let Some(text) = find_first_h1_text(child) {
                    return Some(text);
                }
            }
            None
        }
        ComposableNode::Component(fragment) => fragment.node().and_then(find_first_h1_text),
        _ => None,
    }
}

/// Concatenates the text content of a node slice (text fragments only).
fn collect_text(children: &[ComposableNode]) -> String {
    let mut out = String::new();
    for child in children {
        match child {
            ComposableNode::TextFragment(text) => out.push_str(text),
            ComposableNode::BlockTag(block) => {
                out.push_str(&collect_text(&block.content.children));
            }
            ComposableNode::Component(fragment) => {
                if let Some(node) = fragment.node() {
                    if let ComposableNode::BlockTag(block) = node {
                        out.push_str(&collect_text(&block.content.children));
                    } else if let ComposableNode::TextFragment(text) = node {
                        out.push_str(text);
                    }
                }
            }
            _ => {}
        }
    }
    out
}
