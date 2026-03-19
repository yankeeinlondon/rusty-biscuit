//! Type-safe image references with Markdown, HTML, and terminal rendering.
//!
//! `ImageRef` models the state of an HTML `<img>` tag while providing
//! ergonomic parsing and rendering for Markdown and terminal output.

use std::collections::BTreeMap;
use std::env;
use std::fmt;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::render::stylesheet::{CssSizing, CssSizingProp, Stylesheet, StylesheetError};

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
    #[error("malformed markdown image reference: {0}")]
    MalformedMarkdown(String),

    /// A CSS style declaration failed to parse.
    #[error("invalid CSS style: {0}")]
    InvalidStyle(#[from] StylesheetError),

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
    style: Option<Stylesheet>,
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
    pub fn style(&self) -> Option<&Stylesheet> {
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
    pub fn with_style(mut self, style: Stylesheet) -> Self {
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
        let parsed = Stylesheet::try_from(style.as_str())?;
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
            && let Ok(parsed) = Stylesheet::try_from(style.as_str())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataPolicy {
    Inline,
    Strip,
    Lossless,
}

fn metadata_policy(with_inline: bool) -> MetadataPolicy {
    if with_inline {
        return MetadataPolicy::Inline;
    }

    let value = env::var("IMAGE_REF_METADATA")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase());

    match value.as_deref() {
        Some("inline") => MetadataPolicy::Inline,
        Some("strip") => MetadataPolicy::Strip,
        _ => MetadataPolicy::Lossless,
    }
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
        && let Ok(parsed) = Stylesheet::try_from(style.as_str())
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
        return Err(ImageRefError::MalformedMarkdown(
            "markdown image must start with `![`".to_string(),
        ));
    }

    let alt_end = find_closing_bracket(input, 1)?;
    let raw_alt = unescape_markdown_alt(&input[2..alt_end]);

    let rest = &input[alt_end + 1..];
    if !rest.starts_with('(') {
        return Err(ImageRefError::MalformedMarkdown(
            "expected `(` after alt text".to_string(),
        ));
    }

    let paren_end = find_closing_paren(rest, 0)?;
    let paren_content = &rest[1..paren_end];
    if !rest[paren_end + 1..].trim().is_empty() {
        return Err(ImageRefError::MalformedMarkdown(
            "unexpected trailing content after markdown image".to_string(),
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
        if !parsed_title.is_empty() {
            if let Some(metadata) = decode_markdown_metadata(&parsed_title) {
                image.apply_markdown_metadata(metadata);
            } else {
                image.title = Some(parsed_title);
            }
        }
    }

    if let Some(width) = width_hint {
        let stylesheet = image.style.take().unwrap_or_default();
        image.style = Some(stylesheet.add(CssSizingProp::Width, width));
    }

    Ok(image)
}

fn parse_html_attributes(input: &str) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let mut chars = input.chars().peekable();

    while chars.peek().is_some() {
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
            chars.next();
        }

        let mut key = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() || ch == '=' {
                break;
            }
            key.push(ch);
            chars.next();
        }

        if key.is_empty() {
            break;
        }

        while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
            chars.next();
        }

        if chars.peek() != Some(&'=') {
            attrs.insert(key.to_ascii_lowercase(), String::new());
            continue;
        }
        chars.next();

        while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
            chars.next();
        }

        let value = match chars.peek().copied() {
            Some('"') | Some('\'') => {
                let quote = chars.next().unwrap_or('"');
                let mut value = String::new();
                for ch in chars.by_ref() {
                    if ch == quote {
                        break;
                    }
                    value.push(ch);
                }
                html_unescape(&value)
            }
            _ => {
                let mut value = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_whitespace() {
                        break;
                    }
                    value.push(ch);
                    chars.next();
                }
                html_unescape(&value)
            }
        };

        attrs.insert(key.to_ascii_lowercase(), value);
    }

    attrs
}

fn find_closing_bracket(input: &str, start: usize) -> Result<usize, ImageRefError> {
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut idx = start;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\\' if idx + 1 < bytes.len() => {
                idx += 2;
            }
            b'[' => {
                depth += 1;
                idx += 1;
            }
            b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(idx);
                }
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    Err(ImageRefError::MalformedMarkdown(
        "unmatched `[` in markdown image".to_string(),
    ))
}

fn find_closing_paren(input: &str, start: usize) -> Result<usize, ImageRefError> {
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut idx = start;
    let mut in_quotes = false;
    let mut quote_char = b'"';

    while idx < bytes.len() {
        let byte = bytes[idx];

        if in_quotes {
            if byte == b'\\' && idx + 1 < bytes.len() {
                idx += 2;
                continue;
            }
            if byte == quote_char {
                in_quotes = false;
            }
            idx += 1;
            continue;
        }

        match byte {
            b'"' | b'\'' => {
                in_quotes = true;
                quote_char = byte;
                idx += 1;
            }
            b'(' => {
                depth += 1;
                idx += 1;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(idx);
                }
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    Err(ImageRefError::MalformedMarkdown(
        "unmatched `(` in markdown image".to_string(),
    ))
}

fn extract_markdown_url(content: &str) -> (String, &str) {
    let content = content.trim();
    let bytes = content.as_bytes();
    let mut idx = 0usize;

    if bytes.first() == Some(&b'<') {
        idx = 1;
        while idx < bytes.len() && bytes[idx] != b'>' {
            idx += 1;
        }
        if idx < bytes.len() {
            return (content[1..idx].to_string(), &content[idx + 1..]);
        }
    }

    while idx < bytes.len() {
        if bytes[idx].is_ascii_whitespace() {
            break;
        }
        idx += 1;
    }

    (content[..idx].to_string(), &content[idx..])
}

fn parse_markdown_title_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() > 1 && (bytes[0] == b'"' || bytes[0] == b'\'') {
        let quote = bytes[0];
        let mut idx = 1usize;
        let mut out = String::new();

        while idx < bytes.len() {
            if bytes[idx] == b'\\' && idx + 1 < bytes.len() {
                out.push(bytes[idx + 1] as char);
                idx += 2;
                continue;
            }
            if bytes[idx] == quote {
                break;
            }
            out.push(bytes[idx] as char);
            idx += 1;
        }

        return out;
    }

    trimmed.to_string()
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
    value.replace("%28", "(").replace("%29", ")")
}

fn escape_markdown_url(value: &str) -> String {
    value.replace('(', "%28").replace(')', "%29")
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
    let json = serde_json::to_string(value).ok()?;
    Some(base64_encode(json.as_bytes()))
}

fn decode_markdown_metadata(value: &str) -> Option<MarkdownMetadataPackage> {
    let trimmed = value.trim();
    let decoded = base64_decode(trimmed)?;
    let json = String::from_utf8(decoded).ok()?;
    let metadata = serde_json::from_str::<MarkdownMetadataPackage>(&json).ok()?;
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
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_data_key(key: String) -> Option<String> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    let normalized = key.strip_prefix("data-").unwrap_or(key);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
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
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
}

fn strip_ansi_sequences(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }

        match chars.peek().copied() {
            Some('[') => {
                chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                let mut prev_escape = false;
                for c in chars.by_ref() {
                    if c == '\u{7}' {
                        break;
                    }
                    if prev_escape && c == '\\' {
                        break;
                    }
                    prev_escape = c == '\u{1b}';
                }
            }
            _ => {}
        }
    }

    out
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut idx = 0usize;

    while idx < input.len() {
        let b0 = input[idx];
        let b1 = if idx + 1 < input.len() {
            input[idx + 1]
        } else {
            0
        };
        let b2 = if idx + 2 < input.len() {
            input[idx + 2]
        } else {
            0
        };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        if idx + 1 < input.len() {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if idx + 2 < input.len() {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }

        idx += 3;
    }

    out
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let input = input.trim();
    if input.is_empty() || !input.len().is_multiple_of(4) {
        return None;
    }

    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let bytes = input.as_bytes();
    let mut idx = 0usize;

    while idx < bytes.len() {
        let c0 = bytes[idx] as char;
        let c1 = bytes[idx + 1] as char;
        let c2 = bytes[idx + 2] as char;
        let c3 = bytes[idx + 3] as char;

        let v0 = base64_value(c0)?;
        let v1 = base64_value(c1)?;
        let v2 = if c2 == '=' {
            None
        } else {
            Some(base64_value(c2)?)
        };
        let v3 = if c3 == '=' {
            None
        } else {
            Some(base64_value(c3)?)
        };

        let n = ((v0 as u32) << 18)
            | ((v1 as u32) << 12)
            | ((v2.unwrap_or(0) as u32) << 6)
            | (v3.unwrap_or(0) as u32);

        out.push(((n >> 16) & 0xff) as u8);
        if v2.is_some() {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if v3.is_some() {
            out.push((n & 0xff) as u8);
        }

        idx += 4;
    }

    Some(out)
}

fn base64_value(ch: char) -> Option<u8> {
    match ch {
        'A'..='Z' => Some((ch as u8) - b'A'),
        'a'..='z' => Some((ch as u8) - b'a' + 26),
        '0'..='9' => Some((ch as u8) - b'0' + 52),
        '+' => Some(62),
        '/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let style = Stylesheet::new().add(CssSizingProp::Width, CssSizing::percent(50.0));
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

        let style = Stylesheet::new().add(CssSizingProp::Width, CssSizing::percent(40.0));
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
    #[serial]
    fn markdown_default_policy_roundtrips_all_defined_metadata_fields() {
        let _env = ScopedEnv::remove("IMAGE_REF_METADATA");

        let style = Stylesheet::new()
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
            reparsed.style().map(Stylesheet::to_css),
            original.style().map(Stylesheet::to_css)
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
        let decoded = base64_decode(&encoded).expect("metadata should be base64");
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
