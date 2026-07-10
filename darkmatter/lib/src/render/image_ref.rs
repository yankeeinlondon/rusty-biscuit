//! Type-safe image references with Markdown, HTML, and terminal rendering.
//!
//! `ImageRef` models the state of an HTML `<img>` tag while providing
//! ergonomic parsing and rendering for Markdown and terminal output.

use std::collections::BTreeMap;
#[cfg(test)]
use std::env;
use std::fmt;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::errors::SourceContext;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::render::stylesheet::{
    CssSizing, CssSizingProp, CssStyle, StylesheetBlockError, StylesheetError,
};
use crate::render::metadata_codec::{self, MetadataPolicy};
use crate::render::reference_parse;

/// BEL character used by OSC 8 hyperlinks.
const BEL: &str = "\x07";
/// OSC 8 hyperlink start sequence.
const OSC8_START: &str = "\x1b]8;;";
/// OSC 8 hyperlink end sequence.
const OSC8_END: &str = "\x1b]8;;\x07";
/// Marker used for markdown metadata payload titles.
const MARKDOWN_METADATA_MARKER: &str = "darkmatter:image-ref:v1";

/// Errors returned by [`ImageRef`] construction, parsing, and validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImageRefError {
    /// The image source URL was empty.
    #[error("image source URL cannot be empty")]
    EmptySource,

    /// Neither `src` nor `srcset` was provided.
    #[error("image reference must define either `src` or `srcset`")]
    MissingSource,

    /// The input could not be recognized as HTML or Markdown image syntax.
    #[error("input is not a recognized image reference format")]
    UnrecognizedFormat,

    /// HTML input failed to parse.
    #[error("malformed HTML image reference: {0}")]
    MalformedHtml(String),

    /// Markdown input failed to parse.
    #[error("malformed markdown image reference: {message}")]
    MalformedMarkdown {
        ctx: Box<SourceContext>,
        message: String,
        /// Byte offset in `ctx.content` where the error occurred.
        caret: Option<usize>,
    },

    /// A CSS style declaration failed to parse.
    #[error("invalid CSS style: {0}")]
    InvalidStyle(StylesheetBlockError),

    /// Invalid `decoding` value.
    #[error("invalid image decoding value `{value}`")]
    InvalidDecoding {
        /// Received value.
        value: String,
    },

    /// Invalid `fetchpriority` value.
    #[error("invalid image fetchpriority value `{value}`")]
    InvalidFetchPriority {
        /// Received value.
        value: String,
    },

    /// Invalid `loading` value.
    #[error("invalid image loading value `{value}`")]
    InvalidLoading {
        /// Received value.
        value: String,
    },

    /// Invalid `referrerpolicy` value.
    #[error("invalid image referrerpolicy value `{value}`")]
    InvalidReferrerPolicy {
        /// Received value.
        value: String,
    },
}

/// Wraps a stylesheet parse failure so `?` propagates a [`StylesheetError`]
/// straight into [`ImageRefError::InvalidStyle`].
impl From<StylesheetError> for ImageRefError {
    fn from(error: StylesheetError) -> Self {
        Self::InvalidStyle(StylesheetBlockError(error))
    }
}

impl biscuit_terminal::errors::BlockError for ImageRefError {
    fn status_block(
        &self,
        term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            Self::EmptySource => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ImageRefError", "empty source"))
                .body("Image source URL cannot be empty or whitespace-only.")
                .hint("Pass a non-empty URL or path to <cyan>ImageRef::new</cyan>."),

            Self::MissingSource => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ImageRefError", "missing source"))
                .body("Image reference must define either `src` or `srcset`.")
                .hint("Set <cyan>src=\"...\"</cyan> or <cyan>srcset=\"...\"</cyan> on the image."),

            Self::UnrecognizedFormat => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "ImageRefError",
                    "unrecognized image format",
                ))
                .body(
                    "Input did not look like an HTML `<img ... />` tag or a Markdown `![alt](src)` reference.",
                )
                .hint(
                    "Use <cyan>![alt](src \"title\")</cyan> for Markdown or <cyan>&lt;img src=\"...\" alt=\"...\" /&gt;</cyan> for HTML.",
                ),

            Self::MalformedHtml(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ImageRefError", "malformed HTML image"))
                .body(format!("<dim>Message:</dim> {message}"))
                .hint(
                    "Ensure the tag starts with <cyan>&lt;img</cyan> and ends with <cyan>&gt;</cyan> (self-closing is allowed).",
                ),

            Self::MalformedMarkdown {
                ctx,
                message,
                caret,
            } => {
                let mut body = vec![Prose::new(format!("<dim>Message:</dim> {message}"))];
                body.push(Prose::new("Image parsing failed here:"));

                // Image fragments usually start at line 1 of their own string.
                body.push(ctx.excerpt_prose(1, 0, "md"));

                if let Some(pos) = caret {
                    body.push(Prose::new(format!(
                        "{} <red><b>^</b></red> (offset {})",
                        " ".repeat(*pos),
                        pos
                    )));
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ImageRefError", "malformed markdown image"))
                    .body(body)
                    .hint("Use the pattern <cyan>![alt](src \"optional title\")</cyan> with balanced brackets.")
            }

            Self::InvalidStyle(source) => {
                biscuit_terminal::errors::BlockError::status_block(source, term)
            }

            Self::InvalidDecoding { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ImageRefError", "invalid decoding"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Accepted values: <cyan>sync</cyan>, <cyan>async</cyan>, <cyan>auto</cyan>.",
                ),

            Self::InvalidFetchPriority { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "ImageRefError",
                    "invalid fetch priority",
                ))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint(
                    "Accepted values: <cyan>high</cyan>, <cyan>low</cyan>, <cyan>auto</cyan>.",
                ),

            Self::InvalidLoading { value } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("ImageRefError", "invalid loading"))
                .body(format!("<dim>Value:</dim> <cyan>{value}</cyan>"))
                .hint("Accepted values: <cyan>eager</cyan>, <cyan>lazy</cyan>."),

            Self::InvalidReferrerPolicy { value } => {
                let mut body = format!("<dim>Value:</dim> <cyan>{value}</cyan>");
                if let Some(suggestion) = suggest_referrer_policy(value) {
                    body.push_str(&format!(
                        "\n<dim>Did you mean:</dim> <cyan>{suggestion}</cyan>?"
                    ));
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "ImageRefError",
                        "invalid referrer policy",
                    ))
                    .body(body)
                    .hint(
                        "Accepted values: <cyan>no-referrer</cyan>, <cyan>no-referrer-when-downgrade</cyan>, <cyan>origin</cyan>, <cyan>origin-when-cross-origin</cyan>, <cyan>same-origin</cyan>, <cyan>strict-origin</cyan>, <cyan>strict-origin-when-cross-origin</cyan>, <cyan>unsafe-url</cyan>.",
                    )
            }
        }
    }

    fn block_source(&self) -> Option<&(dyn biscuit_terminal::errors::BlockError + 'static)> {
        match self {
            Self::InvalidStyle(inner) => Some(inner),
            _ => None,
        }
    }
}

impl ImageRefError {
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

impl From<&str> for ImageRefError {
    fn from(value: &str) -> Self {
        Self::malformed_markdown(value)
    }
}

fn suggest_referrer_policy(value: &str) -> Option<&'static str> {
    const CANDIDATES: &[&str] = &[
        "no-referrer",
        "no-referrer-when-downgrade",
        "origin",
        "origin-when-cross-origin",
        "same-origin",
        "strict-origin",
        "strict-origin-when-cross-origin",
        "unsafe-url",
    ];

    let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
    if normalized.is_empty() {
        return None;
    }

    let mut best: Option<(&'static str, usize)> = None;
    for candidate in CANDIDATES {
        let distance = levenshtein_distance(&normalized, candidate);
        match best {
            Some((_, best_distance)) if distance >= best_distance => {}
            _ => best = Some((candidate, distance)),
        }
    }

    let (candidate, distance) = best?;
    let threshold = candidate.len().max(3) / 2;
    if distance <= threshold {
        Some(candidate)
    } else {
        None
    }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    if a.is_empty() {
        return b.chars().count();
    }
    if b.is_empty() {
        return a.chars().count();
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr = vec![0usize; b_chars.len() + 1];

    for (i, ac) in a_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, bc) in b_chars.iter().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            let deletion = prev[j + 1] + 1;
            let insertion = curr[j] + 1;
            let substitution = prev[j] + cost;
            curr[j + 1] = deletion.min(insertion).min(substitution);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_chars.len()]
}

/// Browser image decoding hint values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageDecoding {
    /// Synchronously decode.
    Sync,
    /// Asynchronously decode.
    Async,
    /// Let the browser decide.
    #[default]
    Auto,
}

impl fmt::Display for ImageDecoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sync => write!(f, "sync"),
            Self::Async => write!(f, "async"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl TryFrom<&str> for ImageDecoding {
    type Error = ImageRefError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "sync" => Ok(Self::Sync),
            "async" => Ok(Self::Async),
            "auto" => Ok(Self::Auto),
            _ => Err(ImageRefError::InvalidDecoding {
                value: value.to_string(),
            }),
        }
    }
}

impl TryFrom<String> for ImageDecoding {
    type Error = ImageRefError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Browser image fetch priority hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FetchPriority {
    /// High fetch priority.
    High,
    /// Low fetch priority.
    Low,
    /// Automatic fetch priority.
    #[default]
    Auto,
}

impl fmt::Display for FetchPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Low => write!(f, "low"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl TryFrom<&str> for FetchPriority {
    type Error = ImageRefError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "high" => Ok(Self::High),
            "low" => Ok(Self::Low),
            "auto" => Ok(Self::Auto),
            _ => Err(ImageRefError::InvalidFetchPriority {
                value: value.to_string(),
            }),
        }
    }
}

impl TryFrom<String> for FetchPriority {
    type Error = ImageRefError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Browser image loading strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageLoading {
    /// Eager loading.
    #[default]
    Eager,
    /// Lazy loading.
    Lazy,
}

impl fmt::Display for ImageLoading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eager => write!(f, "eager"),
            Self::Lazy => write!(f, "lazy"),
        }
    }
}

impl TryFrom<&str> for ImageLoading {
    type Error = ImageRefError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "eager" => Ok(Self::Eager),
            "lazy" => Ok(Self::Lazy),
            _ => Err(ImageRefError::InvalidLoading {
                value: value.to_string(),
            }),
        }
    }
}

impl TryFrom<String> for ImageLoading {
    type Error = ImageRefError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Browser referrer policy values for image requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReferrerPolicy {
    /// `no-referrer`
    NoReferrer,
    /// `no-referrer-when-downgrade`
    NoReferrerWhenDowngrade,
    /// `origin`
    Origin,
    /// `origin-when-cross-origin`
    OriginWhenCrossOrigin,
    /// `same-origin`
    SameOrigin,
    /// `strict-origin`
    StrictOrigin,
    /// `strict-origin-when-cross-origin`
    StrictOriginWhenCrossOrigin,
    /// `unsafe-url`
    UnsafeUrl,
}

impl fmt::Display for ReferrerPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoReferrer => write!(f, "no-referrer"),
            Self::NoReferrerWhenDowngrade => write!(f, "no-referrer-when-downgrade"),
            Self::Origin => write!(f, "origin"),
            Self::OriginWhenCrossOrigin => write!(f, "origin-when-cross-origin"),
            Self::SameOrigin => write!(f, "same-origin"),
            Self::StrictOrigin => write!(f, "strict-origin"),
            Self::StrictOriginWhenCrossOrigin => write!(f, "strict-origin-when-cross-origin"),
            Self::UnsafeUrl => write!(f, "unsafe-url"),
        }
    }
}

impl TryFrom<&str> for ReferrerPolicy {
    type Error = ImageRefError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        match normalized.as_str() {
            "no-referrer" => Ok(Self::NoReferrer),
            "no-referrer-when-downgrade" => Ok(Self::NoReferrerWhenDowngrade),
            "origin" => Ok(Self::Origin),
            "origin-when-cross-origin" => Ok(Self::OriginWhenCrossOrigin),
            "same-origin" => Ok(Self::SameOrigin),
            "strict-origin" => Ok(Self::StrictOrigin),
            "strict-origin-when-cross-origin" => Ok(Self::StrictOriginWhenCrossOrigin),
            "unsafe-url" => Ok(Self::UnsafeUrl),
            _ => Err(ImageRefError::InvalidReferrerPolicy {
                value: value.to_string(),
            }),
        }
    }
}

impl TryFrom<String> for ReferrerPolicy {
    type Error = ImageRefError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// A rich image reference with multi-target rendering support.
///
/// The struct mirrors HTML `<img>` state while supporting:
/// - Markdown parsing and rendering
/// - HTML parsing and rendering
/// - Terminal OSC 8 hyperlink rendering
#[derive(Debug, Clone, PartialEq)]
pub struct ImageRef {
    alt: String,
    src: Option<String>,
    srcset: Option<String>,
    style: Option<CssStyle>,
    class: Option<String>,
    title: Option<String>,
    decoding: ImageDecoding,
    fetch_priority: FetchPriority,
    height: Option<u32>,
    width: Option<u32>,
    loading: ImageLoading,
    referrer_policy: Option<ReferrerPolicy>,
    sizes: Option<String>,
    data: BTreeMap<String, String>,
}

impl ImageRef {
    /// Creates a new image reference with `src` and `alt` text.
    ///
    /// `url` must be non-empty after trimming.
    pub fn new<T: Into<String>, U: Into<String>>(
        url: T,
        alt_text: U,
    ) -> Result<Self, ImageRefError> {
        let src = normalize_optional(url.into()).ok_or(ImageRefError::EmptySource)?;
        Self::from_sources(Some(src), None, alt_text.into())
    }

    fn from_sources(
        src: Option<String>,
        srcset: Option<String>,
        alt_text: String,
    ) -> Result<Self, ImageRefError> {
        let src = src.and_then(normalize_optional);
        let srcset = srcset.and_then(normalize_optional);
        if src.is_none() && srcset.is_none() {
            return Err(ImageRefError::MissingSource);
        }

        Ok(Self {
            alt: alt_text,
            src,
            srcset,
            style: None,
            class: None,
            title: None,
            decoding: ImageDecoding::Auto,
            fetch_priority: FetchPriority::Auto,
            height: None,
            width: None,
            loading: ImageLoading::Eager,
            referrer_policy: None,
            sizes: None,
            data: BTreeMap::new(),
        })
    }

    /// Returns the alt text as stored (terminal representation may contain escapes).
    pub fn alt(&self) -> &str {
        &self.alt
    }

    /// Returns alt text stripped of ANSI escape sequences.
    pub fn alt_plain(&self) -> String {
        strip_ansi_sequences(&self.alt)
    }

    /// Returns `src` if set.
    pub fn src(&self) -> Option<&str> {
        self.src.as_deref()
    }

    /// Returns `srcset` if set.
    pub fn srcset(&self) -> Option<&str> {
        self.srcset.as_deref()
    }

    /// Returns `style` if set.
    pub fn style(&self) -> Option<&CssStyle> {
        self.style.as_ref()
    }

    /// Returns CSS class if set.
    pub fn class(&self) -> Option<&str> {
        self.class.as_deref()
    }

    /// Returns title if set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the image decoding hint.
    pub fn decoding(&self) -> ImageDecoding {
        self.decoding
    }

    /// Returns fetch priority.
    pub fn fetch_priority(&self) -> FetchPriority {
        self.fetch_priority
    }

    /// Returns intrinsic height if set.
    pub fn height(&self) -> Option<u32> {
        self.height
    }

    /// Returns intrinsic width if set.
    pub fn width(&self) -> Option<u32> {
        self.width
    }

    /// Returns loading behavior.
    pub fn loading(&self) -> ImageLoading {
        self.loading
    }

    /// Returns referrer policy if set.
    pub fn referrer_policy(&self) -> Option<ReferrerPolicy> {
        self.referrer_policy
    }

    /// Returns `sizes` if set.
    pub fn sizes(&self) -> Option<&str> {
        self.sizes.as_deref()
    }

    /// Returns data attributes (`data-*`) without the `data-` prefix.
    pub fn data(&self) -> &BTreeMap<String, String> {
        &self.data
    }

    /// Returns true if this image has either `src` or `srcset`.
    pub fn has_source(&self) -> bool {
        self.src.is_some() || self.srcset.is_some()
    }

    /// Sets the terminal alt text.
    pub fn with_alt(mut self, alt_text: impl Into<String>) -> Self {
        self.alt = alt_text.into();
        self
    }

    /// Sets `src`.
    pub fn with_src(mut self, src: impl Into<String>) -> Result<Self, ImageRefError> {
        self.src = normalize_optional(src.into());
        self.ensure_source()?;
        Ok(self)
    }

    /// Removes `src`.
    pub fn without_src(mut self) -> Result<Self, ImageRefError> {
        self.src = None;
        self.ensure_source()?;
        Ok(self)
    }

    /// Sets `srcset`.
    pub fn with_srcset(mut self, srcset: impl Into<String>) -> Result<Self, ImageRefError> {
        self.srcset = normalize_optional(srcset.into());
        self.ensure_source()?;
        Ok(self)
    }

    /// Removes `srcset`.
    pub fn without_srcset(mut self) -> Result<Self, ImageRefError> {
        self.srcset = None;
        self.ensure_source()?;
        Ok(self)
    }

    /// Sets CSS class.
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = normalize_optional(class.into());
        self
    }

    /// Sets inline style from a typed stylesheet.
    pub fn with_style(mut self, style: CssStyle) -> Self {
        if style.is_empty() {
            self.style = None;
        } else {
            self.style = Some(style);
        }
        self
    }

    /// Sets inline style from CSS text.
    pub fn with_style_css(mut self, style: impl Into<String>) -> Result<Self, ImageRefError> {
        let style = style.into();
        let parsed = CssStyle::try_from(style.as_str())?;
        self.style = if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        };
        Ok(self)
    }

    /// Sets title text.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = normalize_optional(title.into());
        self
    }

    /// Creates an image and parses the title segment as a title or structured
    /// directive.
    ///
    /// Parallel to
    /// [`Link::with_title_parsed`](crate::render::Link::with_title_parsed): a
    /// `key='value'` title (e.g. `style='color: blue'`) populates typed
    /// `style` / `class` / `title` / `data-*` attrs rather than being stored as
    /// literal title text; an encoded metadata package is decoded; any other
    /// title is kept verbatim.
    ///
    /// ## Errors
    ///
    /// Returns [`ImageRefError`] when the source URL is empty.
    pub fn with_title_parsed(
        src: impl Into<String>,
        alt: impl Into<String>,
        title: &str,
    ) -> Result<Self, ImageRefError> {
        let mut image = Self::new(src, alt)?;
        image.apply_parsed_title(title.trim());
        Ok(image)
    }

    /// Dispatches an (already unquoted) title to metadata decoding, structured
    /// directive parsing, or plain title storage. Shared by the markdown parser
    /// and [`Self::with_title_parsed`].
    fn apply_parsed_title(&mut self, parsed_title: &str) {
        if parsed_title.is_empty() {
            return;
        }
        if let Some(metadata) = decode_markdown_metadata(parsed_title) {
            self.apply_markdown_metadata(metadata);
        } else if is_structured_image_title(parsed_title) {
            // A `key='value'` title is a structured directive (parallel to
            // `Link::with_title_parsed`): parse `style` / `class` / `title` /
            // `data-*` into typed attrs instead of leaking the raw syntax as a
            // literal `title`.
            parse_structured_image_props(self, parsed_title);
        } else {
            self.title = normalize_optional(parsed_title.to_string());
        }
    }

    /// Sets image decoding behavior.
    pub fn with_decoding(mut self, decoding: ImageDecoding) -> Self {
        self.decoding = decoding;
        self
    }

    /// Sets fetch priority.
    pub fn with_fetch_priority(mut self, fetch_priority: FetchPriority) -> Self {
        self.fetch_priority = fetch_priority;
        self
    }

    /// Sets intrinsic height in pixels.
    pub fn with_height(mut self, height: u32) -> Self {
        if height == 0 {
            self.height = None;
        } else {
            self.height = Some(height);
        }
        self
    }

    /// Sets intrinsic width in pixels.
    pub fn with_width(mut self, width: u32) -> Self {
        if width == 0 {
            self.width = None;
        } else {
            self.width = Some(width);
        }
        self
    }

    /// Sets image loading behavior.
    pub fn with_loading(mut self, loading: ImageLoading) -> Self {
        self.loading = loading;
        self
    }

    /// Sets referrer policy.
    pub fn with_referrer_policy(mut self, policy: ReferrerPolicy) -> Self {
        self.referrer_policy = Some(policy);
        self
    }

    /// Clears referrer policy.
    pub fn without_referrer_policy(mut self) -> Self {
        self.referrer_policy = None;
        self
    }

    /// Sets `sizes`.
    pub fn with_sizes(mut self, sizes: impl Into<String>) -> Self {
        self.sizes = normalize_optional(sizes.into());
        self
    }

    /// Sets a `data-*` attribute.
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let Some(key) = normalize_data_key(key.into()) {
            self.data.insert(key, value.into());
        }
        self
    }

    /// Sets all data attributes from a map.
    pub fn with_data_map(mut self, data: BTreeMap<String, String>) -> Self {
        let normalized = data
            .into_iter()
            .filter_map(|(key, value)| normalize_data_key(key).map(|key| (key, value)))
            .collect::<BTreeMap<_, _>>();
        self.data = normalized;
        self
    }

    /// Renders a terminal OSC 8 hyperlink with fallback.
    ///
    /// The raw alt text is used for terminal output, so ANSI styling is preserved.
    pub fn to_terminal(&self) -> String {
        let Some(target) = self.best_source_for_link() else {
            return self.alt.clone();
        };

        if biscuit_terminal::discovery::detection::osc8_link_support() {
            format!("{}{}{}{}{}", OSC8_START, target, BEL, self.alt, OSC8_END)
        } else {
            format!("{} [{}]", self.alt, target)
        }
    }

    /// Renders terminal OSC 8 output without capability detection.
    pub fn to_terminal_unchecked(&self) -> String {
        let Some(target) = self.best_source_for_link() else {
            return self.alt.clone();
        };
        format!("{}{}{}{}{}", OSC8_START, target, BEL, self.alt, OSC8_END)
    }

    /// Renders as HTML `<img ... />`.
    ///
    /// ANSI escape sequences are stripped from `title` and `alt`.
    pub fn to_html(&self) -> String {
        let mut attrs = Vec::new();

        if let Some(src) = &self.src {
            attrs.push(format!(r#"src="{}""#, html_escape(src)));
        }
        if let Some(srcset) = &self.srcset {
            attrs.push(format!(r#"srcset="{}""#, html_escape(srcset)));
        }

        attrs.push(format!(r#"alt="{}""#, html_escape(&self.alt_plain())));

        if let Some(class) = &self.class {
            attrs.push(format!(r#"class="{}""#, html_escape(class)));
        }

        if let Some(style) = self.style_inline_css() {
            attrs.push(format!(r#"style="{}""#, html_escape(&style)));
        }

        if let Some(title) = self.sanitized_title() {
            attrs.push(format!(r#"title="{}""#, html_escape(&title)));
        }

        if self.decoding != ImageDecoding::Auto {
            attrs.push(format!(r#"decoding="{}""#, self.decoding));
        }

        if self.fetch_priority != FetchPriority::Auto {
            attrs.push(format!(r#"fetchpriority="{}""#, self.fetch_priority));
        }

        if let Some(height) = self.height {
            attrs.push(format!(r#"height="{}""#, height));
        }

        if let Some(width) = self.width {
            attrs.push(format!(r#"width="{}""#, width));
        }

        if self.loading != ImageLoading::Eager {
            attrs.push(format!(r#"loading="{}""#, self.loading));
        }

        if let Some(policy) = self.referrer_policy {
            attrs.push(format!(r#"referrerpolicy="{}""#, policy));
        }

        if let Some(sizes) = &self.sizes {
            attrs.push(format!(r#"sizes="{}""#, html_escape(sizes)));
        }

        for (key, value) in &self.data {
            attrs.push(format!(
                r#"data-{}="{}""#,
                html_escape(key),
                html_escape(value)
            ));
        }

        format!("<img {} />", attrs.join(" "))
    }

    /// Renders as Markdown image syntax or inline HTML.
    ///
    /// If metadata beyond `(alt, src, title)` exists:
    /// - `with_inline = true` always emits inline HTML.
    /// - otherwise, `IMAGE_REF_METADATA` drives behavior:
    ///   - `inline` => inline HTML
    ///   - `strip` => idiomatic markdown with metadata dropped
    ///   - default => idiomatic markdown with metadata encoded in title
    pub fn to_markdown(&self, with_inline: bool) -> String {
        let Some(src) = &self.src else {
            return self.to_html();
        };

        let has_extended = self.has_extended_metadata();
        if !has_extended {
            return self.render_markdown_basic(src, self.sanitized_title().as_deref());
        }

        match metadata_policy(with_inline) {
            MetadataPolicy::Inline => self.to_html(),
            MetadataPolicy::Strip => {
                self.render_markdown_basic(src, self.sanitized_title().as_deref())
            }
            MetadataPolicy::Lossless => {
                let encoded = self
                    .to_markdown_metadata_package()
                    .and_then(|pkg| encode_markdown_metadata(&pkg));
                self.render_markdown_basic(src, encoded.as_deref())
            }
        }
    }

    fn ensure_source(&self) -> Result<(), ImageRefError> {
        if self.src.is_none() && self.srcset.is_none() {
            return Err(ImageRefError::MissingSource);
        }
        Ok(())
    }

    fn best_source_for_link(&self) -> Option<&str> {
        if let Some(src) = self.src.as_deref() {
            return Some(src);
        }

        self.srcset
            .as_deref()
            .and_then(extract_primary_src_from_srcset)
    }

    fn style_inline_css(&self) -> Option<String> {
        let style = self.style.as_ref()?;
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

    fn sanitized_title(&self) -> Option<String> {
        self.title.as_ref().and_then(|title| {
            let stripped = strip_ansi_sequences(title);
            normalize_optional(stripped)
        })
    }

    fn has_extended_metadata(&self) -> bool {
        self.class.is_some()
            || self.style.as_ref().is_some_and(|style| !style.is_empty())
            || self.decoding != ImageDecoding::Auto
            || self.fetch_priority != FetchPriority::Auto
            || self.height.is_some()
            || self.width.is_some()
            || self.loading != ImageLoading::Eager
            || self.referrer_policy.is_some()
            || self.sizes.is_some()
            || self.srcset.is_some()
            || !self.data.is_empty()
    }

    fn render_markdown_basic(&self, src: &str, title: Option<&str>) -> String {
        let alt = escape_markdown_alt(&self.alt_plain());
        let src = escape_markdown_url(src);

        if let Some(title) = title {
            let escaped_title = title.replace('"', "\\\"");
            format!("![{alt}]({src} \"{escaped_title}\")")
        } else {
            format!("![{alt}]({src})")
        }
    }

    fn to_markdown_metadata_package(&self) -> Option<MarkdownMetadataPackage> {
        if !self.has_extended_metadata() {
            return None;
        }

        let mut package = MarkdownMetadataPackage::new();
        package.class = self.class.clone();
        package.style = self.style_inline_css();
        package.title = self.sanitized_title();

        if self.decoding != ImageDecoding::Auto {
            package.decoding = Some(self.decoding);
        }
        if self.fetch_priority != FetchPriority::Auto {
            package.fetch_priority = Some(self.fetch_priority);
        }
        package.height = self.height;
        package.width = self.width;
        if self.loading != ImageLoading::Eager {
            package.loading = Some(self.loading);
        }
        package.referrer_policy = self.referrer_policy;
        package.sizes = self.sizes.clone();
        package.srcset = self.srcset.clone();
        package.data = self.data.clone();

        if package.has_payload() {
            Some(package)
        } else {
            None
        }
    }

    fn apply_markdown_metadata(&mut self, metadata: MarkdownMetadataPackage) {
        if metadata.marker != MARKDOWN_METADATA_MARKER {
            return;
        }

        self.class = metadata.class.and_then(normalize_optional);
        self.title = metadata.title.and_then(normalize_optional);

        if let Some(style) = metadata.style.and_then(normalize_optional)
            && let Ok(parsed) = CssStyle::try_from(style.as_str())
            && !parsed.is_empty()
        {
            self.style = Some(parsed);
        }

        if let Some(decoding) = metadata.decoding {
            self.decoding = decoding;
        }

        if let Some(fetch_priority) = metadata.fetch_priority {
            self.fetch_priority = fetch_priority;
        }

        if let Some(height) = metadata.height
            && height > 0
        {
            self.height = Some(height);
        }

        if let Some(width) = metadata.width
            && width > 0
        {
            self.width = Some(width);
        }

        if let Some(loading) = metadata.loading {
            self.loading = loading;
        }

        if let Some(policy) = metadata.referrer_policy {
            self.referrer_policy = Some(policy);
        }

        self.sizes = metadata.sizes.and_then(normalize_optional);
        self.srcset = metadata.srcset.and_then(normalize_optional);

        if !metadata.data.is_empty() {
            self.data.extend(metadata.data);
        }
    }
}

impl fmt::Display for ImageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_terminal())
    }
}

impl TryFrom<String> for ImageRef {
    type Error = ImageRefError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let trimmed = value.trim();

        if trimmed.starts_with('<') {
            parse_html_image(trimmed)
        } else if trimmed.starts_with("![") {
            parse_markdown_image(trimmed)
        } else {
            Err(ImageRefError::UnrecognizedFormat)
        }
    }
}

impl TryFrom<&str> for ImageRef {
    type Error = ImageRefError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<&String> for ImageRef {
    type Error = ImageRefError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<(&str, &str)> for ImageRef {
    fn from((url, alt_text): (&str, &str)) -> Self {
        ImageRef::new(url, alt_text)
            .expect("ImageRef tuple conversion requires a non-empty source URL")
    }
}

impl From<(&String, &String)> for ImageRef {
    fn from((url, alt_text): (&String, &String)) -> Self {
        ImageRef::new(url.as_str(), alt_text.as_str())
            .expect("ImageRef tuple conversion requires a non-empty source URL")
    }
}

impl From<(&str, &Prose)> for ImageRef {
    fn from((url, alt_text): (&str, &Prose)) -> Self {
        let rendered_alt = alt_text.render_optimistic(None);
        ImageRef::new(url, rendered_alt)
            .expect("ImageRef tuple conversion requires a non-empty source URL")
    }
}

impl From<(&String, &Prose)> for ImageRef {
    fn from((url, alt_text): (&String, &Prose)) -> Self {
        let rendered_alt = alt_text.render_optimistic(None);
        ImageRef::new(url.as_str(), rendered_alt)
            .expect("ImageRef tuple conversion requires a non-empty source URL")
    }
}

fn metadata_policy(with_inline: bool) -> MetadataPolicy {
    metadata_codec::metadata_policy(with_inline, "IMAGE_REF_METADATA")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MarkdownMetadataPackage {
    #[serde(rename = "__image_ref")]
    marker: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decoding: Option<ImageDecoding>,
    #[serde(
        default,
        rename = "fetchpriority",
        skip_serializing_if = "Option::is_none"
    )]
    fetch_priority: Option<FetchPriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    loading: Option<ImageLoading>,
    #[serde(
        default,
        rename = "referrerpolicy",
        skip_serializing_if = "Option::is_none"
    )]
    referrer_policy: Option<ReferrerPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sizes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    srcset: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    data: BTreeMap<String, String>,
}

impl MarkdownMetadataPackage {
    fn new() -> Self {
        Self {
            marker: MARKDOWN_METADATA_MARKER.to_string(),
            class: None,
            style: None,
            title: None,
            decoding: None,
            fetch_priority: None,
            height: None,
            width: None,
            loading: None,
            referrer_policy: None,
            sizes: None,
            srcset: None,
            data: BTreeMap::new(),
        }
    }

    fn has_payload(&self) -> bool {
        self.class.is_some()
            || self.style.is_some()
            || self.title.is_some()
            || self.decoding.is_some()
            || self.fetch_priority.is_some()
            || self.height.is_some()
            || self.width.is_some()
            || self.loading.is_some()
            || self.referrer_policy.is_some()
            || self.sizes.is_some()
            || self.srcset.is_some()
            || !self.data.is_empty()
    }
}

fn parse_html_image(input: &str) -> Result<ImageRef, ImageRefError> {
    let trimmed = input.trim();
    if !trimmed.to_ascii_lowercase().starts_with("<img") {
        return Err(ImageRefError::MalformedHtml(
            "image tag must start with `<img`".to_string(),
        ));
    }
    if !trimmed.ends_with('>') {
        return Err(ImageRefError::MalformedHtml(
            "image tag must end with `>`".to_string(),
        ));
    }

    let Some(tag_end) = trimmed.find('>') else {
        return Err(ImageRefError::MalformedHtml(
            "missing end of opening tag".to_string(),
        ));
    };

    let attrs_text = trimmed[4..tag_end]
        .trim()
        .strip_suffix('/')
        .unwrap_or(trimmed[4..tag_end].trim())
        .trim();

    let attrs = parse_html_attributes(attrs_text);
    let src = attrs.get("src").cloned().and_then(normalize_optional);
    let srcset = attrs.get("srcset").cloned().and_then(normalize_optional);
    let alt = attrs.get("alt").cloned().unwrap_or_default();

    let mut image = ImageRef::from_sources(src, srcset, alt)?;

    if let Some(class) = attrs
        .get("class")
        .and_then(|v| normalize_optional(v.clone()))
    {
        image.class = Some(class);
    }

    if let Some(style) = attrs.get("style")
        && let Ok(parsed) = CssStyle::try_from(style.as_str())
        && !parsed.is_empty()
    {
        image.style = Some(parsed);
    }

    if let Some(title) = attrs
        .get("title")
        .and_then(|v| normalize_optional(v.clone()))
    {
        image.title = Some(title);
    }

    if let Some(decoding) = attrs
        .get("decoding")
        .and_then(|value| ImageDecoding::try_from(value.as_str()).ok())
    {
        image.decoding = decoding;
    }

    if let Some(fetch_priority) = attrs
        .get("fetchpriority")
        .and_then(|value| FetchPriority::try_from(value.as_str()).ok())
    {
        image.fetch_priority = fetch_priority;
    }

    if let Some(height) = attrs
        .get("height")
        .and_then(|value| parse_positive_u32(value))
    {
        image.height = Some(height);
    }

    if let Some(width) = attrs
        .get("width")
        .and_then(|value| parse_positive_u32(value))
    {
        image.width = Some(width);
    }

    if let Some(loading) = attrs
        .get("loading")
        .and_then(|value| ImageLoading::try_from(value.as_str()).ok())
    {
        image.loading = loading;
    }

    if let Some(policy) = attrs
        .get("referrerpolicy")
        .and_then(|value| ReferrerPolicy::try_from(value.as_str()).ok())
    {
        image.referrer_policy = Some(policy);
    }

    if let Some(sizes) = attrs
        .get("sizes")
        .and_then(|value| normalize_optional(value.clone()))
    {
        image.sizes = Some(sizes);
    }

    for (key, value) in &attrs {
        if let Some(data_key) = key.strip_prefix("data-")
            && !data_key.is_empty()
        {
            image.data.insert(data_key.to_string(), value.clone());
        }
    }

    Ok(image)
}

fn parse_markdown_image(input: &str) -> Result<ImageRef, ImageRefError> {
    let input = input.trim();
    if !input.starts_with("![") {
        return Err(ImageRefError::malformed_markdown_with_context(
            "markdown image must start with `![`",
            input,
            0,
        ));
    }

    let alt_end = find_closing_bracket(input, 1)?;
    let raw_alt = unescape_markdown_alt(&input[2..alt_end]);

    let rest = &input[alt_end + 1..];
    if !rest.starts_with('(') {
        return Err(ImageRefError::malformed_markdown_with_context(
            "expected `(` after alt text",
            input,
            alt_end + 1,
        ));
    }

    let paren_end = find_closing_paren(rest, 0)?;
    let paren_content = &rest[1..paren_end];
    if !rest[paren_end + 1..].trim().is_empty() {
        return Err(ImageRefError::malformed_markdown_with_context(
            "unexpected trailing content after markdown image",
            input,
            alt_end + 1 + paren_end + 1,
        ));
    }

    let (source, trailing) = extract_markdown_url(paren_content);
    let source = normalize_optional(source).ok_or(ImageRefError::MissingSource)?;
    let source = decode_markdown_url(&source);

    let (alt, width_hint) = parse_alt_width_hint(&raw_alt);
    let mut image = ImageRef::new(source, alt)?;

    let trailing = trailing.trim();
    if !trailing.is_empty() {
        let parsed_title = parse_markdown_title_value(trailing);
        image.apply_parsed_title(&parsed_title);
    }

    if let Some(width) = width_hint {
        let stylesheet = image.style.take().unwrap_or_default();
        image.style = Some(stylesheet.add(CssSizingProp::Width, width));
    }

    Ok(image)
}

fn parse_html_attributes(input: &str) -> BTreeMap<String, String> {
    reference_parse::parse_html_attributes(input)
}

fn find_closing_bracket(input: &str, start: usize) -> Result<usize, ImageRefError> {
    reference_parse::find_closing_bracket(input, start).ok_or_else(|| {
        ImageRefError::malformed_markdown_with_context(
            "unmatched `[` in markdown image", input, start,
        )
    })
}

fn find_closing_paren(input: &str, start: usize) -> Result<usize, ImageRefError> {
    reference_parse::find_closing_paren(input, start).ok_or_else(|| {
        ImageRefError::malformed_markdown_with_context(
            "unmatched `(` in markdown image", input, start,
        )
    })
}

fn extract_markdown_url(content: &str) -> (String, &str) {
    reference_parse::extract_markdown_url(content)
}

/// Whether a markdown image title is a structured `key='value'` directive
/// rather than plain title text.
///
/// Mirrors the hyperlink directive heuristic
/// ([`Link::with_title_parsed`](crate::render::Link::with_title_parsed)): a bare
/// `key=` token (the key being alphanumeric plus `-`/`_`) outside any quoted
/// run marks structured mode.
fn is_structured_image_title(content: &str) -> bool {
    reference_parse::is_structured(content)
}

/// Parses a structured `key='value'` image title into typed [`ImageRef`] attrs.
///
/// Recognizes `style`, `class`, `title`, and `data-*` (the image-relevant subset
/// of the hyperlink directive vocabulary). Unknown keys are ignored. The shared
/// tokenizer matches `parse_structured_props` in [`Link`](crate::render::Link).
fn parse_structured_image_props(image: &mut ImageRef, content: &str) {
    reference_parse::parse_structured(content, |key, value| {
        apply_structured_image_prop(image, key, value);
    });
}

/// Applies a single structured image directive property.
fn apply_structured_image_prop(image: &mut ImageRef, key: &str, value: String) {
    let key = key.to_ascii_lowercase();
    match key.as_str() {
        "title" => image.title = normalize_optional(value),
        "class" => image.class = normalize_optional(value),
        "style" => {
            if let Some(style) = normalize_optional(value)
                && let Ok(parsed) = CssStyle::try_from(style.as_str())
                && !parsed.is_empty()
            {
                image.style = Some(parsed);
            }
        }
        k if k.starts_with("data-") => {
            if let Some(normalized) = normalize_data_key(k.to_string()) {
                image.data.insert(normalized, value);
            }
        }
        _ => {}
    }
}

fn parse_markdown_title_value(value: &str) -> String {
    reference_parse::parse_title(value)
}

fn parse_alt_width_hint(alt_text: &str) -> (String, Option<CssSizing>) {
    let Some(pipe_pos) = alt_text.find('|') else {
        return (alt_text.to_string(), None);
    };

    let after_pipe = &alt_text[pipe_pos + 1..];
    let trimmed = after_pipe.trim_start();
    if !trimmed.starts_with(|ch: char| ch.is_ascii_digit()) {
        return (alt_text.to_string(), None);
    }

    let parsed = parse_css_width_value(trimmed);
    if let Some(width) = parsed {
        let alt = alt_text[..pipe_pos].trim_end().to_string();
        return (alt, Some(width));
    }

    (alt_text.to_string(), None)
}

fn parse_css_width_value(value: &str) -> Option<CssSizing> {
    if let Ok(parsed) = CssSizing::try_from(value) {
        return Some(parsed);
    }

    if value.chars().all(|ch| ch.is_ascii_digit())
        && let Ok(width) = value.parse::<f32>()
    {
        return Some(CssSizing::px(width));
    }

    None
}

fn decode_markdown_url(value: &str) -> String {
    reference_parse::decode_markdown_url(value)
}

fn escape_markdown_url(value: &str) -> String {
    reference_parse::escape_markdown_url(value)
}

fn unescape_markdown_alt(value: &str) -> String {
    value
        .replace("\\[", "[")
        .replace("\\]", "]")
        .replace("\\!", "!")
}

fn escape_markdown_alt(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn encode_markdown_metadata(value: &MarkdownMetadataPackage) -> Option<String> {
    metadata_codec::encode(value)
}

fn decode_markdown_metadata(value: &str) -> Option<MarkdownMetadataPackage> {
    let metadata: MarkdownMetadataPackage = metadata_codec::decode(value)?;
    if metadata.marker == MARKDOWN_METADATA_MARKER {
        Some(metadata)
    } else {
        None
    }
}

fn parse_positive_u32(value: &str) -> Option<u32> {
    let parsed = value.trim().parse::<u32>().ok()?;
    if parsed == 0 { None } else { Some(parsed) }
}

fn normalize_optional(value: String) -> Option<String> {
    reference_parse::normalize_optional(value)
}

fn normalize_data_key(key: String) -> Option<String> {
    reference_parse::normalize_data_key(key)
}

fn extract_primary_src_from_srcset(srcset: &str) -> Option<&str> {
    for candidate in srcset.split(',') {
        let candidate = candidate.trim();
        if candidate.is_empty() {
            continue;
        }

        if let Some(source) = candidate.split_whitespace().next()
            && !source.is_empty()
        {
            return Some(source);
        }
    }

    None
}

fn html_escape(value: &str) -> String {
    reference_parse::html_escape(value)
}

fn strip_ansi_sequences(value: &str) -> String {
    reference_parse::strip_ansi_sequences(value)
}


#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn markdown_image_parses_structured_style_directive() {
        // Review-1 finding 2: a structured `style='...'` title is parsed into a
        // typed inline stylesheet (mirroring `Link::with_title_parsed`), not left
        // as a literal `title` that leaks the raw directive into the HTML.
        let image = ImageRef::try_from("![A](./local.png \"style='color: blue;'\")")
            .expect("parse markdown image");
        let css = image.style().expect("inline style parsed from title").to_css();
        assert!(css.contains("color"), "style not parsed: {css}");
        assert_eq!(image.title(), None, "raw directive must not remain as title");
    }

    #[test]
    fn markdown_image_parses_structured_class_directive() {
        let image = ImageRef::try_from("![A](./local.png \"class='hero' style='width: 20%'\")")
            .expect("parse markdown image");
        assert_eq!(image.class(), Some("hero"));
        assert!(image.style().is_some(), "style not parsed alongside class");
        assert_eq!(image.title(), None);
    }

    #[test]
    fn markdown_image_plain_title_is_not_treated_as_directive() {
        // A normal (non `key=value`) title is preserved verbatim.
        let image = ImageRef::try_from("![A](./local.png \"A lovely photo\")")
            .expect("parse markdown image");
        assert_eq!(image.title(), Some("A lovely photo"));
        assert!(image.style().is_none());
    }

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
    fn new_sets_required_fields_and_defaults() {
        let image =
            ImageRef::new("https://example.com/cat.png", "Cat").expect("image should build");

        assert_eq!(image.src(), Some("https://example.com/cat.png"));
        assert_eq!(image.srcset(), None);
        assert_eq!(image.alt(), "Cat");
        assert_eq!(image.decoding(), ImageDecoding::Auto);
        assert_eq!(image.fetch_priority(), FetchPriority::Auto);
        assert_eq!(image.loading(), ImageLoading::Eager);
    }

    #[test]
    fn new_rejects_empty_source() {
        let err = ImageRef::new("   ", "Cat").expect_err("empty source should fail");
        assert_eq!(err, ImageRefError::EmptySource);
    }

    #[test]
    fn from_tuple_with_prose_preserves_terminal_styling() {
        let prose = Prose::new("<red>Hi</red>");
        let image = ImageRef::from(("https://example.com/hi.png", &prose));

        assert!(image.alt().contains("\u{1b}["));
        assert_eq!(image.alt_plain(), "Hi");
    }

    #[test]
    fn html_output_uses_plain_alt_and_title() {
        let image = ImageRef::new("https://example.com/img.png", "\u{1b}[31mRed\u{1b}[0m")
            .expect("image should build")
            .with_title("\u{1b}[1mBold\u{1b}[0m");

        let html = image.to_html();
        assert!(html.contains(r#"alt="Red""#));
        assert!(html.contains(r#"title="Bold""#));
        assert!(!html.contains("\u{1b}["));
    }

    #[test]
    fn markdown_core_state_uses_idiomatic_syntax() {
        let image = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_title("A title");

        let markdown = image.to_markdown(false);
        assert_eq!(markdown, r#"![Pic](https://example.com/pic.png "A title")"#);
    }

    #[test]
    fn markdown_with_inline_true_uses_html_for_extended_metadata() {
        let image = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_class("hero")
            .with_width(640);

        let markdown = image.to_markdown(true);
        assert!(markdown.starts_with("<img "));
        assert!(markdown.contains(r#"class="hero""#));
        assert!(markdown.contains(r#"width="640""#));
    }

    #[test]
    #[serial]
    fn markdown_policy_strip_drops_extended_metadata() {
        let _env = ScopedEnv::set("IMAGE_REF_METADATA", "strip");

        let style = CssStyle::new().add(CssSizingProp::Width, CssSizing::percent(50.0));
        let image = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_style(style)
            .with_title("Title");

        let markdown = image.to_markdown(false);
        assert_eq!(markdown, r#"![Pic](https://example.com/pic.png "Title")"#);
    }

    #[test]
    #[serial]
    fn markdown_policy_inline_from_env_uses_html() {
        let _env = ScopedEnv::set("IMAGE_REF_METADATA", "inline");
        let image = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_class("rounded");

        let markdown = image.to_markdown(false);
        assert!(markdown.starts_with("<img "));
        assert!(markdown.contains(r#"class="rounded""#));
    }

    #[test]
    #[serial]
    fn markdown_default_policy_is_lossless_and_roundtrips() {
        let _env = ScopedEnv::remove("IMAGE_REF_METADATA");

        let style = CssStyle::new().add(CssSizingProp::Width, CssSizing::percent(40.0));
        let original = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_class("hero")
            .with_title("Tooltip")
            .with_style(style)
            .with_data("id", "123")
            .with_fetch_priority(FetchPriority::High)
            .with_loading(ImageLoading::Lazy);

        let markdown = original.to_markdown(false);
        assert!(markdown.starts_with("![Pic](https://example.com/pic.png \""));
        assert!(!markdown.contains("Tooltip"));

        let reparsed =
            ImageRef::try_from(markdown.as_str()).expect("lossless markdown should parse");
        assert_eq!(reparsed.class(), Some("hero"));
        assert_eq!(reparsed.title(), Some("Tooltip"));
        assert_eq!(reparsed.fetch_priority(), FetchPriority::High);
        assert_eq!(reparsed.loading(), ImageLoading::Lazy);
        assert_eq!(reparsed.data().get("id"), Some(&"123".to_string()));
        assert!(reparsed.style().is_some());
    }

    #[test]
    fn inline_style_round_trips_through_renderable_css_style() {
        // Proves the migration to `renderable::stylesheet`: an inline `style`
        // string parses into the shared `CssStyle` type, survives a markdown
        // round-trip, and a malformed value surfaces a `StylesheetError`
        // wrapped into `ImageRefError::InvalidStyle`.
        let image = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_style_css("width: 40%; height: 240px;")
            .expect("valid inline style should parse");

        let style = image.style().expect("style should be set");
        assert_eq!(style.len(), 2);
        assert_eq!(style.to_css(), "width: 40%;\nheight: 240px;");

        // The typed `CssStyle` builder produces the same declaration block.
        let typed = CssStyle::new()
            .add(CssSizingProp::Width, CssSizing::percent(40.0))
            .add(CssSizingProp::Height, CssSizing::px(240.0));
        assert_eq!(style.to_css(), typed.to_css());

        // An invalid value bubbles a `StylesheetError` via `?` into the
        // wrapped `ImageRefError::InvalidStyle` variant.
        let err = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_style_css("z-index: not-a-number;")
            .expect_err("invalid integer value should fail");
        match err {
            ImageRefError::InvalidStyle(StylesheetBlockError(inner)) => {
                assert!(matches!(inner, StylesheetError::InvalidInteger { .. }));
            }
            other => panic!("expected InvalidStyle, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn markdown_default_policy_roundtrips_all_defined_metadata_fields() {
        let _env = ScopedEnv::remove("IMAGE_REF_METADATA");

        let style = CssStyle::new()
            .add(CssSizingProp::Width, CssSizing::percent(40.0))
            .add(CssSizingProp::Height, CssSizing::px(240.0));
        let original = ImageRef::new("https://example.com/pic.png", "Diagram")
            .expect("image should build")
            .with_class("hero")
            .with_style(style)
            .with_title("Tooltip")
            .with_decoding(ImageDecoding::Async)
            .with_fetch_priority(FetchPriority::High)
            .with_height(480)
            .with_width(640)
            .with_loading(ImageLoading::Lazy)
            .with_referrer_policy(ReferrerPolicy::StrictOriginWhenCrossOrigin)
            .with_sizes("(max-width: 600px) 100vw, 600px")
            .with_srcset("small.png 1x, large.png 2x")
            .expect("srcset should be valid")
            .with_data("id", "123")
            .with_data("role", "diagram");

        let markdown = original.to_markdown(false);
        let reparsed =
            ImageRef::try_from(markdown.as_str()).expect("lossless markdown should parse");

        assert_eq!(reparsed.class(), Some("hero"));
        assert_eq!(reparsed.title(), Some("Tooltip"));
        assert_eq!(reparsed.decoding(), ImageDecoding::Async);
        assert_eq!(reparsed.fetch_priority(), FetchPriority::High);
        assert_eq!(reparsed.height(), Some(480));
        assert_eq!(reparsed.width(), Some(640));
        assert_eq!(reparsed.loading(), ImageLoading::Lazy);
        assert_eq!(
            reparsed.referrer_policy(),
            Some(ReferrerPolicy::StrictOriginWhenCrossOrigin)
        );
        assert_eq!(reparsed.sizes(), Some("(max-width: 600px) 100vw, 600px"));
        assert_eq!(reparsed.srcset(), Some("small.png 1x, large.png 2x"));
        assert_eq!(reparsed.data().get("id"), Some(&"123".to_string()));
        assert_eq!(reparsed.data().get("role"), Some(&"diagram".to_string()));
        assert_eq!(
            reparsed.style().map(CssStyle::to_css),
            original.style().map(CssStyle::to_css)
        );
    }

    #[test]
    fn markdown_metadata_payload_omits_undefined_fields() {
        let markdown = ImageRef::new("https://example.com/pic.png", "Pic")
            .expect("image should build")
            .with_class("hero")
            .to_markdown(false);

        let alt_end = find_closing_bracket(&markdown, 1).expect("alt closing bracket");
        let rest = &markdown[alt_end + 1..];
        let paren_end = find_closing_paren(rest, 0).expect("closing paren");
        let (_, trailing) = extract_markdown_url(&rest[1..paren_end]);
        let encoded = parse_markdown_title_value(trailing.trim());
        let decoded = metadata_codec::base64_decode(&encoded).expect("metadata should be base64");
        let json = String::from_utf8(decoded).expect("metadata should be utf8");

        assert!(json.contains("\"class\":\"hero\""));
        for key in [
            "\"style\":",
            "\"decoding\":",
            "\"fetchpriority\":",
            "\"height\":",
            "\"width\":",
            "\"loading\":",
            "\"referrerpolicy\":",
            "\"sizes\":",
            "\"srcset\":",
            "\"data\":",
        ] {
            assert!(!json.contains(key), "unexpected serialized key: {key}");
        }
    }

    #[test]
    fn markdown_parse_width_hint_applies_style_width() {
        let image =
            ImageRef::try_from("![hi|15%](./my-image.png)").expect("markdown image should parse");
        assert_eq!(image.alt(), "hi");

        let html = image.to_html();
        assert!(html.contains(r#"style="width: 15%;""#));
    }

    #[test]
    fn markdown_parse_with_plain_title() {
        let image = ImageRef::try_from(r#"![Alt](https://example.com/pic.png "Title")"#)
            .expect("markdown image should parse");
        assert_eq!(image.src(), Some("https://example.com/pic.png"));
        assert_eq!(image.title(), Some("Title"));
    }

    #[test]
    fn html_parse_drops_invalid_enum_values() {
        let image = ImageRef::try_from(
            r#"<img src="https://example.com/pic.png" alt="Pic" loading="wrong" decoding="async" fetchpriority="invalid" />"#,
        )
        .expect("html image should parse");

        assert_eq!(image.loading(), ImageLoading::Eager);
        assert_eq!(image.decoding(), ImageDecoding::Async);
        assert_eq!(image.fetch_priority(), FetchPriority::Auto);
    }

    #[test]
    fn html_parse_supports_srcset_only() {
        let image = ImageRef::try_from(r#"<img srcset="a.png 1x, b.png 2x" alt="Alt" />"#)
            .expect("srcset-only html image should parse");

        assert_eq!(image.src(), None);
        assert_eq!(image.srcset(), Some("a.png 1x, b.png 2x"));
        assert!(image.to_markdown(false).starts_with("<img "));
    }

    #[test]
    fn terminal_unchecked_uses_osc8_format() {
        let image =
            ImageRef::new("https://example.com/pic.png", "Pic").expect("image should build");
        let terminal = image.to_terminal_unchecked();

        assert!(terminal.starts_with("\x1b]8;;https://example.com/pic.png\x07"));
        assert!(terminal.ends_with("\x1b]8;;\x07"));
    }
}
