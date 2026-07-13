//! Type-safe hyperlinks with Markdown, HTML, and terminal rendering.
//!
//! `Link` provides strongly-typed link metadata while preserving ergonomic
//! parsing and rendering for terminal, HTML, and Markdown outputs.

use std::borrow::Cow;
use std::collections::BTreeMap;
#[cfg(test)]
use std::env;
use std::fmt;
use std::hash::{Hash, Hasher};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::errors::SourceContext;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::render::stylesheet::{CssStyle, StylesheetBlockError, StylesheetError};
use crate::render::metadata_codec::{self, MetadataPolicy};
use crate::render::reference_parse;

/// BEL character used by OSC 8 hyperlinks.
const BEL: &str = "\x07";
/// OSC 8 hyperlink start sequence.
const LINK_START: &str = "\x1b]8;;";
/// OSC 8 hyperlink end sequence.
const LINK_END: &str = "\x1b]8;;\x07";
/// Marker used for markdown metadata payload titles.
const MARKDOWN_METADATA_MARKER: &str = "darkmatter:link:v1";
/// Env var used to control markdown metadata behavior.
const LINK_METADATA_ENV: &str = "LINK_METADATA";

/// Errors returned by [`Link`] construction, parsing, and validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LinkError {
    /// The URL/href was empty.
    #[error("link href cannot be empty")]
    EmptyHref,

    /// Input does not match HTML or Markdown link syntax.
    #[error("input is not a recognized link format")]
    UnrecognizedFormat,

    /// HTML input failed to parse.
    #[error("malformed HTML link: {0}")]
    MalformedHtml(String),

    /// Markdown input failed to parse.
    #[error("malformed markdown link: {message}")]
    MalformedMarkdown {
        ctx: Box<SourceContext>,
        message: String,
        /// Byte offset in `ctx.content` where the error occurred.
        caret: Option<usize>,
    },

    /// Missing required href/url field.
    #[error("link is missing href/url")]
    MissingHref,

    /// Invalid typed style declaration.
    #[error("invalid CSS style: {0}")]
    InvalidStyle(StylesheetBlockError),

    /// Invalid target value.
    #[error("invalid link target `{value}`")]
    InvalidTarget {
        /// Received target value.
        value: String,
    },
}

/// Wraps a stylesheet parse failure so `?` propagates a [`StylesheetError`]
/// straight into [`LinkError::InvalidStyle`].
impl From<StylesheetError> for LinkError {
    fn from(error: StylesheetError) -> Self {
        Self::InvalidStyle(StylesheetBlockError(error))
    }
}

impl biscuit_terminal::errors::BlockError for LinkError {
    fn status_block(
        &self,
        term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            Self::EmptyHref => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("LinkError", "empty href"))
                .body("Link href cannot be empty or whitespace-only.")
                .hint("Pass a non-empty URL, relative path, or anchor to <cyan>Link::new</cyan>."),

            Self::UnrecognizedFormat => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("LinkError", "unrecognized link format"))
                .body("Input did not look like an HTML `<a>` tag or a Markdown `[text](href)` link.")
                .hint(
                    "Use <cyan>[display](url)</cyan> for Markdown or <cyan>&lt;a href=\"url\"&gt;text&lt;/a&gt;</cyan> for HTML.",
                ),

            Self::MalformedHtml(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("LinkError", "malformed HTML link"))
                .body(format!("<dim>Message:</dim> {message}"))
                .hint(
                    "Ensure the link opens with <cyan>&lt;a ...&gt;</cyan> and closes with <cyan>&lt;/a&gt;</cyan> and has an <cyan>href</cyan> attribute.",
                ),

            Self::MalformedMarkdown {
                ctx,
                message,
                caret,
            } => {
                let mut body = vec![Prose::new(format!("<dim>Message:</dim> {message}"))];
                body.push(Prose::new("Link parsing failed here:"));

                // Link fragments usually start at line 1 of their own string.
                body.push(ctx.excerpt_prose(1, 0, "md"));

                if let Some(pos) = caret {
                    body.push(Prose::new(format!(
                        "{} <red><b>^</b></red> (offset {})",
                        " ".repeat(*pos),
                        pos
                    )));
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("LinkError", "malformed markdown link"))
                    .body(body)
                    .hint("Use the pattern <cyan>[display](url \"optional title\")</cyan> with balanced brackets.")
            }

            Self::MissingHref => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("LinkError", "missing href"))
                .body("Link has no `href`/`url` attribute.")
                .hint("Add an <cyan>href=\"...\"</cyan> attribute (HTML) or a URL inside <cyan>( )</cyan> (Markdown)."),

            Self::InvalidStyle(source) => {
                biscuit_terminal::errors::BlockError::status_block(source, term)
            }

            Self::InvalidTarget { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("LinkError", "invalid target"))
                .body(format!(
                    "<dim>Target:</dim> <cyan>{}</cyan>",
                    value.replace('_', "\\_"),
                ))
                .hint(
                    "Valid targets: <cyan>\\_self</cyan>, <cyan>\\_blank</cyan>, <cyan>\\_parent</cyan>, <cyan>\\_top</cyan>, or a named context.",
                ),
        }
    }

    fn block_source(&self) -> Option<&(dyn biscuit_terminal::errors::BlockError + 'static)> {
        match self {
            Self::InvalidStyle(inner) => Some(inner),
            _ => None,
        }
    }
}

impl LinkError {
    /// Creates a markdown-parse error without source-context details.
    pub fn malformed_markdown(message: impl Into<String>) -> Self {
        Self::MalformedMarkdown {
            ctx: Box::new(SourceContext::new(
                std::path::PathBuf::from("unknown"),
                std::path::PathBuf::from("unknown"),
                "",
            )),
            message: message.into(),
            caret: None,
        }
    }

    fn malformed_markdown_with_context(
        message: impl Into<String>,
        input: impl Into<String>,
        caret: usize,
    ) -> Self {
        let input_str = input.into();
        Self::MalformedMarkdown {
            ctx: Box::new(SourceContext::new(
                std::path::PathBuf::from("unknown"),
                std::path::PathBuf::from("unknown"),
                input_str,
            )),
            message: message.into(),
            caret: Some(caret),
        }
    }
}

impl From<&str> for LinkError {
    fn from(value: &str) -> Self {
        Self::malformed_markdown(value)
    }
}

/// Backward-compatible parse error alias.
pub type LinkParseError = LinkError;

/// The type of resource a link points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LinkType {
    /// HTTP or HTTPS URL.
    Url,
    /// Non-HTTP resource such as relative or absolute file path.
    File,
}

fn type_from_string(input: &str) -> LinkType {
    if input.starts_with("http://") || input.starts_with("https://") {
        LinkType::Url
    } else {
        LinkType::File
    }
}

/// Typed HTML target values for links.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LinkTarget {
    /// `_self`
    Self_,
    /// `_blank`
    Blank,
    /// `_parent`
    Parent,
    /// `_top`
    Top,
    /// Named browsing context.
    Named(String),
}

impl LinkTarget {
    /// Returns the target string value to use in HTML attributes.
    pub fn as_attr_value(&self) -> Cow<'_, str> {
        match self {
            Self::Self_ => Cow::Borrowed("_self"),
            Self::Blank => Cow::Borrowed("_blank"),
            Self::Parent => Cow::Borrowed("_parent"),
            Self::Top => Cow::Borrowed("_top"),
            Self::Named(value) => Cow::Borrowed(value.as_str()),
        }
    }
}

impl fmt::Display for LinkTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_attr_value())
    }
}

impl TryFrom<&str> for LinkTarget {
    type Error = LinkError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(LinkError::InvalidTarget {
                value: value.to_string(),
            });
        }

        match trimmed.to_ascii_lowercase().as_str() {
            "_self" => Ok(Self::Self_),
            "_blank" => Ok(Self::Blank),
            "_parent" => Ok(Self::Parent),
            "_top" => Ok(Self::Top),
            _ => Ok(Self::Named(trimmed.to_string())),
        }
    }
}

impl TryFrom<String> for LinkTarget {
    type Error = LinkError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// A rich hyperlink supporting terminal, HTML, and Markdown rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    kind: LinkType,
    display: String,
    link_to: String,
    class: Option<String>,
    style: Option<CssStyle>,
    target: Option<LinkTarget>,
    title: Option<String>,
    prompt: Option<String>,
    data: BTreeMap<String, String>,
}

impl Eq for Link {}

impl Link {
    /// Creates a new link with validated href.
    pub fn new(display: impl Into<String>, link: impl Into<String>) -> Result<Self, LinkError> {
        let link_to = normalize_optional(link.into()).ok_or(LinkError::EmptyHref)?;
        let kind = type_from_string(&link_to);

        Ok(Self {
            kind,
            display: display.into(),
            link_to,
            class: None,
            style: None,
            target: None,
            title: None,
            prompt: None,
            data: BTreeMap::new(),
        })
    }

    /// Creates a link and parses the markdown title segment as title or structured metadata.
    pub fn with_title_parsed(
        display: impl Into<String>,
        link: impl Into<String>,
        title: &str,
    ) -> Result<Self, LinkError> {
        let mut result = Self::new(display, link)?;
        let title = title.trim();
        if title.is_empty() {
            return Ok(result);
        }

        if let Some(metadata) = decode_markdown_metadata(title) {
            result.apply_markdown_metadata(metadata);
            return Ok(result);
        }

        if is_structured_mode(title) {
            parse_structured_props(&mut result, title);
        } else {
            result.title = normalize_optional(parse_title_value(title));
        }

        Ok(result)
    }

    /// Sets display text.
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = display.into();
        self
    }

    /// Sets href and updates the computed link type.
    pub fn with_href(mut self, href: impl Into<String>) -> Result<Self, LinkError> {
        let href = normalize_optional(href.into()).ok_or(LinkError::EmptyHref)?;
        self.kind = type_from_string(&href);
        self.link_to = href;
        Ok(self)
    }

    /// Sets CSS class.
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = normalize_optional(class.into());
        self
    }

    /// Sets typed inline stylesheet.
    pub fn with_style(mut self, style: CssStyle) -> Self {
        self.style = if style.is_empty() { None } else { Some(style) };
        self
    }

    /// Sets inline stylesheet from CSS text.
    pub fn with_style_css(mut self, style: impl Into<String>) -> Result<Self, LinkError> {
        let parsed = CssStyle::try_from(style.into().as_str())?;
        self.style = if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        };
        Ok(self)
    }

    /// Sets typed target.
    pub fn with_target(mut self, target: LinkTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Sets target from raw string.
    pub fn with_target_str(mut self, target: impl Into<String>) -> Result<Self, LinkError> {
        self.target = Some(LinkTarget::try_from(target.into())?);
        Ok(self)
    }

    /// Sets title text.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = normalize_optional(title.into());
        self
    }

    /// Sets optional prompt text.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = normalize_optional(prompt.into());
        self
    }

    /// Adds a `data-*` attribute.
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let Some(key) = normalize_data_key(key.into()) {
            self.data.insert(key, value.into());
        }
        self
    }

    /// Replaces all `data-*` attributes.
    pub fn with_data_map(mut self, data: BTreeMap<String, String>) -> Self {
        self.data = data
            .into_iter()
            .filter_map(|(k, v)| normalize_data_key(k).map(|nk| (nk, v)))
            .collect();
        self
    }

    /// Returns href.
    pub fn href(&self) -> &str {
        &self.link_to
    }

    /// Returns display text (terminal representation may include ANSI escapes).
    pub fn display(&self) -> &str {
        &self.display
    }

    /// Returns display text with ANSI removed.
    pub fn display_plain(&self) -> String {
        strip_ansi_sequences(&self.display)
    }

    /// Returns computed link type.
    pub fn kind(&self) -> LinkType {
        self.kind
    }

    /// Returns class if set.
    pub fn class(&self) -> Option<&str> {
        self.class.as_deref()
    }

    /// Returns typed style if set.
    pub fn style(&self) -> Option<&CssStyle> {
        self.style.as_ref()
    }

    /// Returns style as inline CSS string if set.
    pub fn style_css(&self) -> Option<String> {
        style_inline_css(self.style.as_ref())
    }

    /// Returns target if set.
    pub fn target(&self) -> Option<&LinkTarget> {
        self.target.as_ref()
    }

    /// Returns target as HTML attribute value if set.
    pub fn target_attr(&self) -> Option<String> {
        self.target.as_ref().map(ToString::to_string)
    }

    /// Returns title if set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns title with ANSI removed if set.
    pub fn title_plain(&self) -> Option<String> {
        self.title
            .as_ref()
            .and_then(|t| normalize_optional(strip_ansi_sequences(t)))
    }

    /// Returns prompt if set.
    pub fn prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
    }

    /// Returns data map (`data-*` keys without prefix).
    pub fn data(&self) -> &BTreeMap<String, String> {
        &self.data
    }

    /// Parses style into property-value map for diagnostics.
    pub fn parsed_style(&self) -> Option<BTreeMap<String, String>> {
        self.style_css().map(|style| parse_css_style(&style))
    }

    /// Returns `true` for HTTP/HTTPS links.
    pub fn is_url(&self) -> bool {
        self.kind == LinkType::Url
    }

    /// Returns `true` for non-HTTP links.
    pub fn is_file(&self) -> bool {
        self.kind == LinkType::File
    }

    /// Renders an OSC 8 hyperlink with capability fallback.
    pub fn to_terminal(&self) -> String {
        if biscuit_terminal::discovery::detection::osc8_link_support() {
            format!(
                "{}{}{}{}{}",
                LINK_START, self.link_to, BEL, self.display, LINK_END
            )
        } else {
            format!("{} [{}]", self.display, self.link_to)
        }
    }

    /// Renders an OSC 8 hyperlink without capability checks.
    pub fn to_terminal_unchecked(&self) -> String {
        format!(
            "{}{}{}{}{}",
            LINK_START, self.link_to, BEL, self.display, LINK_END
        )
    }

    /// Renders as HTML `<a ...>...</a>`.
    pub fn to_html(&self) -> String {
        let mut attrs = Vec::new();
        attrs.push(format!(r#"href="{}""#, html_escape(&self.link_to)));

        if let Some(class) = &self.class {
            attrs.push(format!(r#"class="{}""#, html_escape(class)));
        }

        if let Some(style) = self.style_css() {
            attrs.push(format!(r#"style="{}""#, html_escape(&style)));
        }

        if let Some(target) = &self.target {
            attrs.push(format!(r#"target="{}""#, html_escape(&target.to_string())));
        }

        if let Some(title) = self.title_plain() {
            attrs.push(format!(r#"title="{}""#, html_escape(&title)));
        }

        if let Some(prompt) = self
            .prompt
            .as_ref()
            .and_then(|p| normalize_optional(strip_ansi_sequences(p)))
        {
            attrs.push(format!(r#"data-prompt="{}""#, html_escape(&prompt)));
        }

        for (key, value) in &self.data {
            attrs.push(format!(
                r#"data-{}="{}""#,
                html_escape(key),
                html_escape(value)
            ));
        }

        format!(
            "<a {}>{}</a>",
            attrs.join(" "),
            html_escape(&self.display_plain())
        )
    }

    /// Backward-compatible alias for [`Link::to_html`].
    pub fn to_browser(&self) -> String {
        self.to_html()
    }

    /// Renders Markdown with policy-driven metadata handling.
    ///
    /// When extended metadata exists (class/style/target/prompt/data):
    /// - `with_inline=true` => inline HTML
    /// - env `LINK_METADATA=inline` => inline HTML
    /// - env `LINK_METADATA=strip` => strip metadata
    /// - otherwise => lossless metadata payload in markdown title
    pub fn to_markdown(&self, with_inline: bool) -> String {
        if !self.has_extended_metadata() {
            return self.render_markdown_basic(self.title_plain().as_deref());
        }

        match metadata_policy(with_inline) {
            MetadataPolicy::Inline => self.to_html(),
            MetadataPolicy::Strip => self.render_markdown_basic(self.title_plain().as_deref()),
            MetadataPolicy::Lossless => {
                let encoded = self
                    .to_markdown_metadata_package()
                    .and_then(|pkg| encode_markdown_metadata(&pkg));
                self.render_markdown_basic(encoded.as_deref())
            }
        }
    }

    /// Backward-compatible alias for legacy callers.
    pub fn to_markdown_legacy(&self) -> String {
        self.to_markdown(false)
    }

    /// Renders the accessible prompted-link markup when a prompt is present.
    ///
    /// The returned string is the canonical wrapper/anchor/prompt structure the
    /// production render-tree browser path emits (see
    /// `renderable::tree::render`): a `dm-popover-wrapper` span holding the
    /// navigable anchor (real `href`, `interestfor` + `aria-describedby`
    /// association) followed by a `dm-popover-prompt` `popover="hint"` element
    /// with escaped content. The shared Popover CSS is supplied separately by the
    /// feature resolver. This standalone helper is for callers rendering a single
    /// `Link` outside the document pipeline; it deliberately mirrors production so
    /// the two do not diverge.
    ///
    /// ## Returns
    ///
    /// `None` when the link carries no prompt.
    ///
    /// ## Notes
    ///
    /// The popover `id` is derived deterministically from this link's target and
    /// display alone, with no document-scoped occurrence state, so this helper
    /// guarantees a document-unique id only for a **single** prompted link.
    /// Emitting several links this way and concatenating the fragments can repeat
    /// an id and make `aria-describedby` ambiguous. For a document with more than
    /// one prompted link, render through the render-tree browser path
    /// (`renderable::tree::render`), which defers id allocation to final-document
    /// assembly and keeps every occurrence unique.
    pub fn to_html_with_popover(&self) -> Option<String> {
        let prompt = self
            .prompt
            .as_ref()
            .and_then(|p| normalize_optional(strip_ansi_sequences(p)))?;

        let id = generate_popover_id(&self.link_to, &self.display);
        let mut attrs = vec![format!(r#"href="{}""#, html_escape(&self.link_to))];

        if let Some(class) = &self.class {
            attrs.push(format!(r#"class="{}""#, html_escape(class)));
        }

        if let Some(style) = self.style_css() {
            attrs.push(format!(r#"style="{}""#, html_escape(&style)));
        }

        if let Some(target) = &self.target {
            attrs.push(format!(r#"target="{}""#, html_escape(&target.to_string())));
        }

        if let Some(title) = self.title_plain() {
            attrs.push(format!(r#"title="{}""#, html_escape(&title)));
        }

        for (key, value) in &self.data {
            attrs.push(format!(
                r#"data-{}="{}""#,
                html_escape(key),
                html_escape(value)
            ));
        }

        attrs.push(format!(r#"interestfor="{id}""#));
        attrs.push(format!(r#"aria-describedby="{id}""#));

        let anchor = format!(
            "<a {}>{}</a>",
            attrs.join(" "),
            html_escape(&self.display_plain())
        );
        let prompt = format!(
            r#"<span id="{id}" class="dm-popover-prompt" popover="hint" role="note">{}</span>"#,
            html_escape(&prompt)
        );

        Some(format!(
            r#"<span class="dm-popover-wrapper">{anchor}{prompt}</span>"#
        ))
    }

    /// Backward-compatible alias for legacy callers.
    pub fn to_browser_with_popover(&self) -> Option<String> {
        self.to_html_with_popover()
    }

    fn has_extended_metadata(&self) -> bool {
        self.class.is_some()
            || self.style.as_ref().is_some_and(|style| !style.is_empty())
            || self.target.is_some()
            || self.prompt.is_some()
            || !self.data.is_empty()
    }

    fn render_markdown_basic(&self, title: Option<&str>) -> String {
        let display = escape_markdown_display(&self.display_plain());
        let href = escape_markdown_url(&self.link_to);

        if let Some(title) = title {
            let escaped_title = title.replace('"', "\\\"");
            format!("[{display}]({href} \"{escaped_title}\")")
        } else {
            format!("[{display}]({href})")
        }
    }

    fn to_markdown_metadata_package(&self) -> Option<LinkMarkdownMetadataPackage> {
        if !self.has_extended_metadata() {
            return None;
        }

        let package = LinkMarkdownMetadataPackage {
            marker: MARKDOWN_METADATA_MARKER.to_string(),
            class: self.class.clone(),
            style: self.style_css(),
            target: self.target.as_ref().map(ToString::to_string),
            title: self.title_plain(),
            prompt: self
                .prompt
                .as_ref()
                .and_then(|p| normalize_optional(strip_ansi_sequences(p))),
            data: self.data.clone(),
        };

        if package.has_payload() {
            Some(package)
        } else {
            None
        }
    }

    fn apply_markdown_metadata(&mut self, metadata: LinkMarkdownMetadataPackage) {
        if metadata.marker != MARKDOWN_METADATA_MARKER {
            return;
        }

        self.class = metadata.class.and_then(normalize_optional);
        self.title = metadata.title.and_then(normalize_optional);
        self.prompt = metadata.prompt.and_then(normalize_optional);

        if let Some(style) = metadata.style.and_then(normalize_optional)
            && let Ok(parsed) = CssStyle::try_from(style.as_str())
            && !parsed.is_empty()
        {
            self.style = Some(parsed);
        }

        if let Some(target) = metadata
            .target
            .and_then(normalize_optional)
            .and_then(|t| LinkTarget::try_from(t.as_str()).ok())
        {
            self.target = Some(target);
        }

        if !metadata.data.is_empty() {
            self.data.extend(metadata.data);
        }
    }
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_terminal())
    }
}

impl Hash for Link {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.display.hash(state);
        self.link_to.hash(state);
    }
}

impl PartialOrd for Link {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Link {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.link_to
            .cmp(&other.link_to)
            .then_with(|| self.display.cmp(&other.display))
    }
}

impl TryFrom<String> for Link {
    type Error = LinkError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();

        if trimmed.starts_with('<') {
            parse_html_link(trimmed)
        } else if trimmed.starts_with('[') {
            parse_markdown_link(trimmed)
        } else {
            Err(LinkError::UnrecognizedFormat)
        }
    }
}

impl TryFrom<&str> for Link {
    type Error = LinkError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<&String> for Link {
    type Error = LinkError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<(&str, &str)> for Link {
    fn from((display, link): (&str, &str)) -> Self {
        Link::new(display, link).expect("Link tuple conversion requires a non-empty href")
    }
}

impl From<(&String, &String)> for Link {
    fn from((display, link): (&String, &String)) -> Self {
        Link::new(display.as_str(), link.as_str())
            .expect("Link tuple conversion requires a non-empty href")
    }
}

impl From<(&str, &Prose)> for Link {
    fn from((href, display): (&str, &Prose)) -> Self {
        let rendered_display = display.render_optimistic(None);
        Link::new(rendered_display, href).expect("Link tuple conversion requires a non-empty href")
    }
}

impl From<(&String, &Prose)> for Link {
    fn from((href, display): (&String, &Prose)) -> Self {
        let rendered_display = display.render_optimistic(None);
        Link::new(rendered_display, href.as_str())
            .expect("Link tuple conversion requires a non-empty href")
    }
}

fn metadata_policy(with_inline: bool) -> MetadataPolicy {
    metadata_codec::metadata_policy(with_inline, LINK_METADATA_ENV)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkMarkdownMetadataPackage {
    #[serde(rename = "__link_ref")]
    marker: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    data: BTreeMap<String, String>,
}

impl LinkMarkdownMetadataPackage {
    fn has_payload(&self) -> bool {
        self.class.is_some()
            || self.style.is_some()
            || self.target.is_some()
            || self.title.is_some()
            || self.prompt.is_some()
            || !self.data.is_empty()
    }
}

fn parse_html_link(input: &str) -> Result<Link, LinkError> {
    let input = input.trim();

    if !is_anchor_opening_tag(input) {
        return Err(LinkError::MalformedHtml(
            "link must start with an opening '<a>' tag".to_string(),
        ));
    }

    let Some(closing_tag_start) = input.rfind('<') else {
        return Err(LinkError::MalformedHtml(
            "link must end with a closing '</a>' tag".to_string(),
        ));
    };

    if !is_anchor_closing_tag(&input[closing_tag_start..]) {
        return Err(LinkError::MalformedHtml(
            "link must end with a closing '</a>' tag".to_string(),
        ));
    }

    let Some(tag_end) = input.find('>') else {
        return Err(LinkError::MalformedHtml(
            "could not find end of opening tag".to_string(),
        ));
    };

    let opening_tag = &input[..tag_end];
    let display_start = tag_end + 1;
    let display_end = closing_tag_start;

    if display_start > display_end {
        return Err(LinkError::MalformedHtml("empty display text".to_string()));
    }

    let display = html_unescape(&input[display_start..display_end]);
    let attrs_str = opening_tag[2..].trim();
    let attrs = parse_html_attributes(attrs_str);

    let href = attrs
        .get("href")
        .cloned()
        .and_then(normalize_optional)
        .ok_or(LinkError::MissingHref)?;

    let mut link = Link::new(display, href)?;

    if let Some(class) = attrs
        .get("class")
        .and_then(|v| normalize_optional(v.clone()))
    {
        link.class = Some(class);
    }

    if let Some(style) = attrs.get("style")
        && let Ok(parsed) = CssStyle::try_from(style.as_str())
        && !parsed.is_empty()
    {
        link.style = Some(parsed);
    }

    if let Some(target) = attrs
        .get("target")
        .and_then(|v| normalize_optional(v.clone()))
        .and_then(|v| LinkTarget::try_from(v.as_str()).ok())
    {
        link.target = Some(target);
    }

    if let Some(title) = attrs
        .get("title")
        .and_then(|v| normalize_optional(v.clone()))
    {
        link.title = Some(title);
    }

    if let Some(prompt) = attrs
        .get("data-prompt")
        .and_then(|v| normalize_optional(v.clone()))
    {
        link.prompt = Some(prompt);
    }

    for (key, value) in attrs {
        if let Some(data_key) = key.strip_prefix("data-") {
            if data_key == "prompt" || data_key.is_empty() {
                continue;
            }
            link.data.insert(data_key.to_string(), value);
        }
    }

    Ok(link)
}

fn is_anchor_opening_tag(input: &str) -> bool {
    let Some(rest) = input.strip_prefix('<') else {
        return false;
    };

    let mut chars = rest.chars();
    let Some(tag_name) = chars.next() else {
        return false;
    };

    if !tag_name.eq_ignore_ascii_case(&'a') {
        return false;
    }

    match chars.next() {
        Some('>') => true,
        Some(ch) if ch.is_ascii_whitespace() => true,
        _ => false,
    }
}

fn is_anchor_closing_tag(input: &str) -> bool {
    let trimmed = input.trim();
    let Some(rest) = trimmed.strip_prefix("</") else {
        return false;
    };
    let Some(rest) = rest.strip_suffix('>') else {
        return false;
    };

    rest.trim().eq_ignore_ascii_case("a")
}

fn parse_markdown_link(input: &str) -> Result<Link, LinkError> {
    let input = input.trim();
    if !input.starts_with('[') {
        return Err(LinkError::malformed_markdown_with_context(
            "link must start with '['",
            input,
            0,
        ));
    }

    let display_end = find_closing_bracket(input, 0)?;
    let display = unescape_markdown_display(&input[1..display_end]);

    let rest = &input[display_end + 1..];
    if !rest.starts_with('(') {
        return Err(LinkError::malformed_markdown_with_context(
            "expected '(' after display text",
            input,
            display_end + 1,
        ));
    }

    let paren_end = find_closing_paren(rest, 0)?;
    let paren_content = &rest[1..paren_end];

    let (url, trailing) = extract_url(paren_content);
    let url = normalize_optional(url).ok_or(LinkError::MissingHref)?;
    let url = decode_markdown_url(&url);

    let mut link = Link::new(display, url)?;

    let trailing = trailing.trim();
    if trailing.is_empty() {
        return Ok(link);
    }

    if let Some(metadata) = decode_markdown_metadata(&parse_title_value(trailing)) {
        link.apply_markdown_metadata(metadata);
        return Ok(link);
    }

    if is_structured_mode(trailing) {
        parse_structured_props(&mut link, trailing);
    } else {
        link.title = normalize_optional(parse_title_value(trailing));
    }

    Ok(link)
}

fn parse_html_attributes(input: &str) -> BTreeMap<String, String> {
    reference_parse::parse_html_attributes(input)
}

fn find_closing_bracket(input: &str, start: usize) -> Result<usize, LinkError> {
    reference_parse::find_closing_bracket(input, start).ok_or_else(|| {
        LinkError::malformed_markdown_with_context("unmatched '[' in link", input, start)
    })
}

fn find_closing_paren(input: &str, start: usize) -> Result<usize, LinkError> {
    reference_parse::find_closing_paren(input, start).ok_or_else(|| {
        LinkError::malformed_markdown_with_context("unmatched '(' in link", input, start)
    })
}

fn extract_url(content: &str) -> (String, &str) {
    reference_parse::extract_markdown_url(content)
}

fn is_structured_mode(content: &str) -> bool {
    reference_parse::is_structured(content)
}

fn parse_structured_props(link: &mut Link, content: &str) {
    reference_parse::parse_structured(content, |key, value| {
        apply_structured_prop(link, key, value);
    });
}

fn apply_structured_prop(link: &mut Link, key: &str, value: String) {
    let key = key.to_ascii_lowercase();
    match key.as_str() {
        "title" => link.title = normalize_optional(value),
        "prompt" => link.prompt = normalize_optional(value),
        "class" => link.class = normalize_optional(value),
        "style" => {
            if let Some(style) = normalize_optional(value)
                && let Ok(parsed) = CssStyle::try_from(style.as_str())
                && !parsed.is_empty()
            {
                link.style = Some(parsed);
            }
        }
        "target" => {
            if let Some(target) = normalize_optional(value)
                && let Ok(parsed) = LinkTarget::try_from(target.as_str())
            {
                link.target = Some(parsed);
            }
        }
        k if k.starts_with("data-") => {
            if let Some(normalized) = normalize_data_key(k.to_string()) {
                link.data.insert(normalized, value);
            }
        }
        _ => {}
    }
}

fn parse_title_value(content: &str) -> String {
    reference_parse::parse_title(content)
}

fn unescape_markdown_display(value: &str) -> String {
    value.replace("\\[", "[").replace("\\]", "]")
}

fn escape_markdown_display(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn decode_markdown_url(value: &str) -> String {
    reference_parse::decode_markdown_url(value)
}

fn escape_markdown_url(value: &str) -> String {
    reference_parse::escape_markdown_url(value)
}

fn encode_markdown_metadata(value: &LinkMarkdownMetadataPackage) -> Option<String> {
    metadata_codec::encode(value)
}

fn decode_markdown_metadata(value: &str) -> Option<LinkMarkdownMetadataPackage> {
    let metadata: LinkMarkdownMetadataPackage = metadata_codec::decode(value)?;
    if metadata.marker == MARKDOWN_METADATA_MARKER {
        Some(metadata)
    } else {
        None
    }
}

fn generate_popover_id(url: &str, display: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    display.hash(&mut hasher);
    format!("popover-{:x}", hasher.finish())
}

fn parse_css_style(style: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for declaration in style.split(';') {
        let trimmed = declaration.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim().to_lowercase();
            let value = trimmed[colon_pos + 1..].trim();
            if !key.is_empty() && !value.is_empty() {
                result.insert(key, value.to_string());
            }
        }
    }
    result
}

fn style_inline_css(style: Option<&CssStyle>) -> Option<String> {
    let style = style?;
    if style.is_empty() {
        return None;
    }

    let rendered = style
        .to_css()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    normalize_optional(rendered)
}

fn normalize_optional(value: String) -> Option<String> {
    reference_parse::normalize_optional(value)
}

fn normalize_data_key(key: String) -> Option<String> {
    reference_parse::normalize_data_key(key)
}

fn html_escape(value: &str) -> String {
    reference_parse::html_escape(value)
}

fn html_unescape(value: &str) -> String {
    reference_parse::html_unescape(value)
}

fn strip_ansi_sequences(value: &str) -> String {
    reference_parse::strip_ansi_sequences(value)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::stylesheet::{CssSizing, CssSizingProp};
    use serial_test::serial;

    struct ScopedEnv {
        key: String,
        original: Option<String>,
    }

    impl ScopedEnv {
        fn set(key: &str, value: &str) -> Self {
            let original = env::var(key).ok();
            // SAFETY: Used only in serial tests in this module.
            unsafe {
                env::set_var(key, value);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }

        fn remove(key: &str) -> Self {
            let original = env::var(key).ok();
            // SAFETY: Used only in serial tests in this module.
            unsafe {
                env::remove_var(key);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            // SAFETY: Used only in serial tests in this module.
            unsafe {
                match &self.original {
                    Some(value) => env::set_var(&self.key, value),
                    None => env::remove_var(&self.key),
                }
            }
        }
    }

    #[test]
    fn new_requires_non_empty_href() {
        assert!(matches!(Link::new("x", "  "), Err(LinkError::EmptyHref)));
    }

    #[test]
    fn new_sets_kind() {
        let url = Link::new("x", "https://example.com").expect("link should build");
        let file = Link::new("x", "./file.md").expect("link should build");

        assert!(url.is_url());
        assert!(file.is_file());
    }

    #[test]
    fn typed_style_is_rendered_to_html() {
        let style = CssStyle::new().add(CssSizingProp::TopMargin, CssSizing::px(8.0));
        let link = Link::new("Click", "https://example.com")
            .expect("link should build")
            .with_style(style);

        let html = link.to_html();
        assert!(html.contains(r#"style="margin-top: 8px;""#));
    }

    #[test]
    fn html_output_strips_ansi_from_display_and_title() {
        let link = Link::new("\u{1b}[31mClick\u{1b}[0m", "https://example.com")
            .expect("link should build")
            .with_title("\u{1b}[1mTitle\u{1b}[0m");

        let html = link.to_html();
        assert!(html.contains("Click"));
        assert!(html.contains("Title"));
        assert!(!html.contains("\u{1b}["));
    }

    #[test]
    fn markdown_core_state_stays_idiomatic() {
        let link = Link::new("Example", "https://example.com")
            .expect("link should build")
            .with_title("Tooltip");

        assert_eq!(
            link.to_markdown(false),
            r#"[Example](https://example.com "Tooltip")"#
        );
    }

    #[test]
    fn markdown_inline_mode_uses_html_for_extended_metadata() {
        let link = Link::new("Example", "https://example.com")
            .expect("link should build")
            .with_class("chip")
            .with_target(LinkTarget::Blank);

        let markdown = link.to_markdown(true);
        assert!(markdown.starts_with("<a "));
        assert!(markdown.contains(r#"class="chip""#));
        assert!(markdown.contains(r#"target="_blank""#));
    }

    #[test]
    #[serial]
    fn markdown_env_strip_drops_extended_metadata() {
        let _env = ScopedEnv::set(LINK_METADATA_ENV, "strip");

        let link = Link::new("Example", "https://example.com")
            .expect("link should build")
            .with_class("chip")
            .with_title("Title");

        assert_eq!(
            link.to_markdown(false),
            r#"[Example](https://example.com "Title")"#
        );
    }

    #[test]
    #[serial]
    fn markdown_env_inline_uses_html() {
        let _env = ScopedEnv::set(LINK_METADATA_ENV, "inline");

        let link = Link::new("Example", "https://example.com")
            .expect("link should build")
            .with_class("chip");

        assert!(link.to_markdown(false).starts_with("<a "));
    }

    #[test]
    #[serial]
    fn markdown_default_is_lossless_and_roundtrips_extended_metadata() {
        let _env = ScopedEnv::remove(LINK_METADATA_ENV);

        let style = CssStyle::new().add(CssSizingProp::TopMargin, CssSizing::px(10.0));
        let original = Link::new("Example", "https://example.com")
            .expect("link should build")
            .with_class("chip")
            .with_style(style)
            .with_target(LinkTarget::Blank)
            .with_title("Tooltip")
            .with_prompt("Prompt")
            .with_data("id", "abc");

        let markdown = original.to_markdown(false);
        assert!(markdown.starts_with("[Example](https://example.com \""));
        assert!(!markdown.contains("Tooltip"));

        let parsed = Link::try_from(markdown.as_str()).expect("roundtrip markdown should parse");
        assert_eq!(parsed.class(), Some("chip"));
        assert_eq!(parsed.target(), Some(&LinkTarget::Blank));
        assert_eq!(parsed.title(), Some("Tooltip"));
        assert_eq!(parsed.prompt(), Some("Prompt"));
        assert_eq!(parsed.data().get("id"), Some(&"abc".to_string()));
        assert!(parsed.style().is_some());
    }

    #[test]
    fn parse_html_link_with_attributes() {
        let link = Link::try_from(
            r#"<a href="https://example.com" class="btn" style="margin-top: 8px;" target="_blank" title="Go" data-id="123">Click</a>"#,
        )
        .expect("html link should parse");

        assert_eq!(link.display(), "Click");
        assert_eq!(link.href(), "https://example.com");
        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.target(), Some(&LinkTarget::Blank));
        assert_eq!(link.title(), Some("Go"));
        assert_eq!(link.data().get("id"), Some(&"123".to_string()));
        assert!(link.style().is_some());
    }

    #[test]
    fn parse_html_link_accepts_uppercase_tags() {
        let link = Link::try_from(r#"<A HREF="https://example.com">Text</A>"#)
            .expect("uppercase html link should parse");

        assert_eq!(link.display(), "Text");
        assert_eq!(link.href(), "https://example.com");
    }

    #[test]
    fn parse_html_link_accepts_double_space_after_tag_name() {
        let link = Link::try_from(r#"<a  href="https://example.com">Text</a>"#)
            .expect("double-space html link should parse");

        assert_eq!(link.display(), "Text");
        assert_eq!(link.href(), "https://example.com");
    }

    #[test]
    fn parse_html_link_accepts_tab_after_tag_name() {
        let link = Link::try_from("<a\thref=\"https://example.com\">Text</a>")
            .expect("tab-separated html link should parse");

        assert_eq!(link.display(), "Text");
        assert_eq!(link.href(), "https://example.com");
    }

    #[test]
    fn parse_html_link_accepts_mixed_case_closing_tag() {
        let link = Link::try_from(r#"<A href="https://example.com">text</a>"#)
            .expect("mixed-case html link should parse");

        assert_eq!(link.display(), "text");
        assert_eq!(link.href(), "https://example.com");
    }

    #[test]
    fn parse_markdown_title_mode() {
        let link = Link::try_from(r#"[Click](https://example.com "Title")"#)
            .expect("markdown link should parse");
        assert_eq!(link.display(), "Click");
        assert_eq!(link.href(), "https://example.com");
        assert_eq!(link.title(), Some("Title"));
    }

    #[test]
    fn parse_markdown_structured_mode_legacy() {
        let link = Link::try_from(
            r#"[Click](https://example.com class="btn" target=_blank prompt="go" data-id=42)"#,
        )
        .expect("structured markdown should parse");

        assert_eq!(link.class(), Some("btn"));
        assert_eq!(link.target(), Some(&LinkTarget::Blank));
        assert_eq!(link.prompt(), Some("go"));
        assert_eq!(link.data().get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn with_title_parsed_supports_structured_and_title_modes() {
        let title_mode = Link::with_title_parsed("Click", "https://example.com", "Simple")
            .expect("link should build");
        assert_eq!(title_mode.title(), Some("Simple"));

        let structured = Link::with_title_parsed(
            "Click",
            "https://example.com",
            "class='btn' target='_blank'",
        )
        .expect("link should build");
        assert_eq!(structured.class(), Some("btn"));
        assert_eq!(structured.target(), Some(&LinkTarget::Blank));
    }

    #[test]
    fn terminal_unchecked_uses_osc8_sequence() {
        let link = Link::new("Click", "https://example.com").expect("link should build");
        let output = link.to_terminal_unchecked();

        assert!(output.starts_with("\x1b]8;;https://example.com\x07"));
        assert!(output.ends_with("\x1b]8;;\x07"));
    }

    #[test]
    fn html_with_popover_uses_prompt() {
        let link = Link::new("Click", "https://example.com")
            .expect("link should build")
            .with_prompt("Hover me");

        let html = link
            .to_html_with_popover()
            .expect("popover should be generated");
        assert!(html.contains(r#"<span class="dm-popover-wrapper">"#));
        assert!(html.contains("interestfor"));
        assert!(html.contains("aria-describedby"));
        assert!(html.contains(r#"popover="hint""#));
        assert!(html.contains(r#"class="dm-popover-prompt""#));
    }
}
