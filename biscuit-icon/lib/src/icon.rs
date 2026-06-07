//! The central `Icon` handle.

use std::str::FromStr;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;

use crate::body::IconBody;
use crate::error::{IconError, Result};
use crate::glyph::Glyph;
use crate::style::{Flip, Rotate, Style};

/// Where an icon's body came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A compiled-in curated domain icon.
    Embedded,
    /// A network-fetched / cache-resident Iconify icon.
    Network,
}

/// A renderable icon: a body plus accumulated style, optional glyph, and the
/// identifier used for text fallback.
#[derive(Debug, Clone)]
pub struct Icon {
    pub(crate) id: String,
    pub(crate) body: IconBody,
    pub(crate) glyph: Option<Glyph>,
    pub(crate) source: Source,
    pub(crate) style: Style,
    pub(crate) layout: Layout,
    pub(crate) nerd_font: bool,
}

impl Icon {
    /// Builds an icon from an embedded domain body.
    #[must_use]
    pub(crate) fn from_domain(id: &str, body: IconBody, glyph: Option<Glyph>) -> Self {
        Self {
            id: id.to_string(),
            body,
            glyph,
            source: Source::Embedded,
            style: Style::default(),
            layout: Layout::default(),
            nerd_font: false,
        }
    }

    /// Builds an icon from a network/cache body.
    #[must_use]
    pub(crate) fn from_network(id: &str, body: IconBody) -> Self {
        Self {
            id: id.to_string(),
            body,
            glyph: None,
            source: Source::Network,
            style: Style::default(),
            layout: Layout::default(),
            nerd_font: false,
        }
    }

    /// The icon identifier (`prefix:name` for network icons, Iconify id for
    /// embedded domain icons).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Fetches an Iconify icon by `prefix:name`, consulting the local cache
    /// first and falling back to the network (then caching the result).
    ///
    /// ## Errors
    /// - [`IconError::InvalidIdentifier`] for malformed ids.
    /// - [`IconError::NotFound`] / [`IconError::Fetch`] on lookup failure.
    /// - [`IconError::Cache`] on cache failure.
    pub async fn iconify(id: &str) -> Result<Icon> {
        let cache = crate::cache::IconCache::open_default()?;
        let client = crate::iconify::IconifyClient::new();
        Self::iconify_with(id, &cache, &client).await
    }

    /// Cache-first lookup against an explicit cache and client (used in tests).
    ///
    /// ## Errors
    /// See [`Icon::iconify`].
    pub async fn iconify_with(
        id: &str,
        cache: &crate::cache::IconCache,
        client: &crate::iconify::IconifyClient,
    ) -> Result<Icon> {
        let (prefix, name) = crate::iconify::parse_id(id)?;

        // Cache reads and writes are synchronous SQLite I/O; move them off
        // the async runtime thread with spawn_blocking.
        let id_owned = id.to_string();
        let path = cache.path().to_path_buf();
        let cached = tokio::task::spawn_blocking(move || {
            let c = crate::cache::IconCache::open_at(&path)?;
            c.get(&prefix, &name)
        })
        .await
        .map_err(|e| IconError::Cache(e.to_string()))?
        .map_err(IconError::Cache)?;

        if let Some(body) = cached {
            return Ok(Icon::from_network(&id_owned, body));
        }

        let body = client.fetch_body(&id_owned).await?;
        let path = cache.path().to_path_buf();
        let body_for_cache = body.clone();
        tokio::task::spawn_blocking(move || {
            let c = crate::cache::IconCache::open_at(&path)?;
            c.put(&prefix, &name, &body_for_cache)
        })
        .await
        .map_err(|e| IconError::Cache(e.to_string()))?
        .map_err(IconError::Cache)?;

        Ok(Icon::from_network(&id_owned, body))
    }

    /// The body's provenance.
    #[must_use]
    pub fn source(&self) -> Source {
        self.source
    }

    /// The raw body.
    #[must_use]
    pub fn body(&self) -> &IconBody {
        &self.body
    }

    /// The Unicode character, if this icon has one.
    #[must_use]
    pub fn unicode_char(&self) -> Option<char> {
        self.glyph.and_then(|g| g.unicode)
    }

    /// The Nerd Font character, if this icon has one.
    #[must_use]
    pub fn nerd_font_char(&self) -> Option<char> {
        self.glyph.and_then(|g| g.nerd_font)
    }

    /// Assembles the styled `<svg>` markup.
    #[must_use]
    pub fn svg(&self) -> String {
        self.style.assemble(&self.body)
    }

    // --- styling builders ---

    /// Sets the CSS color (drives `currentColor` for monochrome icons).
    #[must_use]
    pub fn color(mut self, c: impl Into<String>) -> Self {
        self.style.color = Some(c.into());
        self
    }

    /// Sets the SVG width.
    #[must_use]
    pub fn width(mut self, w: impl Into<String>) -> Self {
        self.style.width = Some(w.into());
        self
    }

    /// Sets the SVG height.
    #[must_use]
    pub fn height(mut self, h: impl Into<String>) -> Self {
        self.style.height = Some(h.into());
        self
    }

    /// Flips the icon.
    #[must_use]
    pub fn flip(mut self, f: Flip) -> Self {
        self.style.flip = Some(f);
        self
    }

    /// Rotates the icon.
    #[must_use]
    pub fn rotate(mut self, r: Rotate) -> Self {
        self.style.rotate = Some(r);
        self
    }

    /// Enables or disables Nerd Font glyph eligibility during terminal
    /// rendering.
    #[must_use]
    pub fn nerd_font(mut self, on: bool) -> Self {
        self.nerd_font = on;
        self
    }

    /// Toggles the transparent bounding-box rect.
    #[must_use]
    pub fn view_box(mut self, on: bool) -> Self {
        self.style.view_box = on;
        self
    }
}

impl TerminalRenderable for Icon {
    fn render(&self, term: &Terminal) -> String {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::discovery::detection::ImageSupport;

        if self.nerd_font && let Some(c) = self.nerd_font_char() {
            return Prose::new(c.to_string()).render(term);
        }
        if let Some(c) = self.unicode_char() {
            return Prose::new(c.to_string()).render(term);
        }
        #[cfg(feature = "image")]
        if term.image_support != ImageSupport::None {
            if let Ok(s) = self.render_image(term) {
                return s;
            }
        }
        Prose::new(self.id.clone()).render(term)
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "image")]
impl Icon {
    /// Rasterizes the assembled SVG to a temp file and renders it via the
    /// terminal's image protocol.
    fn render_image(&self, term: &Terminal) -> std::io::Result<String> {
        use std::io::Write;
        use biscuit_terminal::components::terminal_image::TerminalImage;

        let mut file = tempfile::Builder::new().suffix(".svg").tempfile()?;
        file.write_all(self.svg().as_bytes())?;
        let img = TerminalImage::new(file.path())
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(img.render(term))
    }
}

/// Generates a string-convenience constructor for a domain set.
macro_rules! domain_ctor {
    ($fn_name:ident, $enum:ty, $set:literal) => {
        impl Icon {
            #[doc = concat!("Looks up a `", $set, "` icon by its snake_case name.")]
            ///
            /// ## Errors
            /// Returns [`IconError::UnknownDomainIcon`] when the name is unknown.
            pub fn $fn_name(name: &str) -> Result<Icon> {
                use crate::domain::DomainIcon;
                <$enum>::from_str(name)
                    .map(DomainIcon::icon)
                    .map_err(|_| IconError::UnknownDomainIcon { set: $set, name: name.to_string() })
            }
        }
    };
}

domain_ctor!(os, crate::domain::Os, "os");
domain_ctor!(emoji, crate::domain::Emoji, "emoji");
domain_ctor!(arrow, crate::domain::Arrow, "arrow");
domain_ctor!(data, crate::domain::Data, "data");
domain_ctor!(file, crate::domain::File, "file");
domain_ctor!(hardware, crate::domain::Hardware, "hardware");
domain_ctor!(timing, crate::domain::Timing, "timing");
domain_ctor!(button, crate::domain::Button, "button");
domain_ctor!(control, crate::domain::Control, "control");
domain_ctor!(network, crate::domain::Network, "network");
domain_ctor!(dev_ops, crate::domain::DevOps, "dev_ops");
domain_ctor!(actors, crate::domain::Actors, "actors");
domain_ctor!(nav, crate::domain::Nav, "nav");
domain_ctor!(sport, crate::domain::Sport, "sport");
domain_ctor!(brand, crate::domain::Brand, "brand");
domain_ctor!(social, crate::domain::Social, "social");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainIcon, Os};
    use crate::style::{Flip, Rotate};

    #[test]
    fn builder_threads_style_into_svg() {
        let svg = Os::Apple.icon().color("red").width("48").svg();
        assert!(svg.contains("width=\"48\""));
        assert!(svg.contains("style=\"color: red\""));
    }

    #[test]
    fn flip_and_rotate_use_typed_enums() {
        let svg = Os::Apple.icon().flip(Flip::Horizontal).rotate(Rotate::R90).svg();
        assert!(svg.contains("scale(-1 1)"));
        assert!(svg.contains("rotate(90)"));
    }

    #[test]
    fn string_ctor_unknown_name_errors() {
        let err = Icon::os("nope").unwrap_err();
        assert!(matches!(err, IconError::UnknownDomainIcon { set: "os", .. }));
    }

    #[test]
    fn string_ctor_known_name_succeeds() {
        assert!(Icon::os("finder").is_ok());
    }

    #[test]
    fn icon_remembers_its_id() {
        let icon = Os::Apple.icon();
        assert_eq!(icon.id(), "ic:baseline-apple");
    }
}
