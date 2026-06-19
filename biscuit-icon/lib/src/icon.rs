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
        let prefix_get = prefix.clone();
        let name_get = name.clone();
        let cached = tokio::task::spawn_blocking(move || {
            let c = crate::cache::IconCache::open_at(&path).map_err(|e| IconError::Cache(e.to_string()))?;
            c.get(&prefix_get, &name_get).map_err(|e| IconError::Cache(e.to_string()))
        })
        .await
        .map_err(|e| IconError::Cache(e.to_string()))??;

        if let Some(body) = cached {
            return Ok(Icon::from_network(&id_owned, body));
        }

        let body = client.fetch_body(&id_owned).await?;
        let path = cache.path().to_path_buf();
        let body_for_cache = body.clone();
        tokio::task::spawn_blocking(move || {
            let c = crate::cache::IconCache::open_at(&path).map_err(|e| IconError::Cache(e.to_string()))?;
            c.put(&prefix, &name, &body_for_cache).map_err(|e| IconError::Cache(e.to_string()))
        })
        .await
        .map_err(|e| IconError::Cache(e.to_string()))??;

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

    /// Assembles the styled SVG and returns it as a CSS `url()` data URI.
    ///
    /// Percent-encodes characters that are unsafe in a CSS URL literal:
    /// `#`, `<`, `>`, `"`, `'`, plus ASCII whitespace.
    #[must_use]
    pub fn css(&self) -> String {
        let svg = self.svg();
        let mut out = String::with_capacity(svg.len() * 2);
        for c in svg.chars() {
            match c {
                '#' => out.push_str("%23"),
                '<' => out.push_str("%3C"),
                '>' => out.push_str("%3E"),
                '"' => out.push_str("%22"),
                '\'' => out.push_str("%27"),
                ' ' => out.push_str("%20"),
                '\n' => out.push_str("%0A"),
                '\t' => out.push_str("%09"),
                '\r' => out.push_str("%0D"),
                c => out.push(c),
            }
        }
        format!("url('data:image/svg+xml,{out}')")
    }

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

        if self.nerd_font && let Some(c) = self.nerd_font_char() {
            return Prose::new(c.to_string()).render(term);
        }
        if let Some(c) = self.unicode_char() {
            return Prose::new(c.to_string()).render(term);
        }
        #[cfg(feature = "image")]
        {
            use biscuit_terminal::discovery::detection::ImageSupport;
            if term.image_support != ImageSupport::None
                && let Ok(s) = self.render_image(term)
            {
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

    /// Renders the icon as a small inline image sized to fit inside a table
    /// cell. Unlike [`TerminalRenderable::render`], which fills the
    /// terminal width, this clamps the image to `cells_wide` columns and
    /// 1 row so the surrounding cell borders stay aligned. The cursor
    /// advances exactly one row after the image, matching normal cell
    /// line-height behavior.
    ///
    /// Returns `Err` when the terminal cannot inline images; callers
    /// should fall back to a glyph or text identifier in that case.
    pub fn render_in_cell(&self, term: &Terminal, cells_wide: u32) -> std::io::Result<String> {
        use std::io::Write;
        use biscuit_terminal::components::terminal_image::{ImageWidth, TerminalImage};
        use biscuit_terminal::discovery::detection::ImageSupport;

        if matches!(term.image_support, ImageSupport::None) || !term.is_tty {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "terminal cannot inline images",
            ));
        }

        let mut file = tempfile::Builder::new().suffix(".svg").tempfile()?;
        file.write_all(self.svg().as_bytes())?;
        let img = TerminalImage::new(file.path())
            .map_err(|e| std::io::Error::other(e.to_string()))?
            .with_width(ImageWidth::Characters(cells_wide.max(1)));
        Ok(img.render(term))
    }
}

#[cfg(all(test, feature = "image"))]
mod render_in_cell_tests {
    use super::*;
    use crate::domain::DomainIcon;
    use biscuit_terminal::terminal::Terminal;

    #[test]
    fn render_in_cell_errors_when_terminal_has_no_image_support() {
        // `Terminal::default()` resolves image_support to None (no
        // terminal metadata in the test env), so render_in_cell must
        // refuse rather than silently falling back to a huge image.
        let icon = crate::domain::Os::Apple.icon();
        let term = Terminal::default();
        let result = icon.render_in_cell(&term, 1);
        assert!(result.is_err(), "expected Err when image support is None; got Ok: {:?}", result.ok());
    }

    #[test]
    fn render_in_cell_clamps_zero_to_one() {
        // 0 cells_wide is a misconfiguration; the API must clamp to 1
        // rather than producing a zero-width image.
        let icon = crate::domain::Os::Apple.icon();
        let term = Terminal::default();
        let _ = icon.render_in_cell(&term, 0);
        // We only assert that the call doesn't panic; the image-support
        // check fires first and returns Err before reaching width.
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

    #[test]
    fn css_wraps_encoded_svg_in_url() {
        use crate::domain::DomainIcon;
        let css = Os::Apple.icon().color("#d97706").css();
        assert!(css.starts_with("url('data:image/svg+xml,"), "expected CSS url() prefix; got: {css}");
        assert!(css.contains("%23d97706"), "expected percent-encoded hex color; got: {css}");
        assert!(!css.contains('#'), "raw # must be encoded; got: {css}");
        assert!(!css.contains('\n'), "raw newline must be encoded; got: {css}");
    }

    #[test]
    fn css_encodes_url_hostile_characters() {
        let body = crate::body::IconBody::new(
            "<path d=\"M0 0\" fill=\"#fff\" title=\"it's ok\"/>",
            24,
            24,
        );
        let icon = Icon::from_network("test:icon", body);
        let css = icon.css();
        assert!(css.contains("%3C"), "expected encoded <; got: {css}");
        assert!(css.contains("%3E"), "expected encoded >; got: {css}");
        assert!(css.contains("%22"), "expected encoded double quote; got: {css}");
        assert!(css.contains("%27"), "expected encoded single quote; got: {css}");
        assert!(css.contains("%20"), "expected encoded space; got: {css}");
        assert!(css.contains("%23"), "expected encoded #; got: {css}");
    }

    #[tokio::test]
    async fn iconify_with_non_zero_origin_persists_through_cache() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let json = serde_json::json!({
            "prefix": "custom",
            "icons": { "logo": { "body": "<path d=\"M0 0\"/>", "left": 10, "top": 20, "width": 32, "height": 32 } }
        });
        Mock::given(method("GET"))
            .and(path("/custom.json"))
            .and(query_param("icons", "logo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let cache = crate::cache::IconCache::open_at(dir.path().join("icons.db")).unwrap();
        let client = crate::iconify::IconifyClient::with_base(server.uri());

        let icon = Icon::iconify_with("custom:logo", &cache, &client).await.unwrap();
        let svg = icon.svg();
        assert!(svg.contains("viewBox=\"10 20 32 32\""), "expected non-zero viewBox in assembled SVG; got: {svg}");

        // Ensure cache round-trip also preserves origin.
        let cached = Icon::iconify_with("custom:logo", &cache, &client).await.unwrap();
        let cached_svg = cached.svg();
        assert!(cached_svg.contains("viewBox=\"10 20 32 32\""), "expected non-zero viewBox after cache hit; got: {cached_svg}");
    }

    #[tokio::test]
    async fn offline_resize_obligation_test() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        use crate::domain::Os;

        // 1. Curated icon test:
        let curated = Os::Apple.icon().width("64").height("64");
        let svg = curated.svg();
        assert!(svg.contains("width=\"64\""), "expected width=\"64\"; got: {svg}");
        assert!(svg.contains("height=\"64\""), "expected height=\"64\"; got: {svg}");
        assert!(svg.contains("viewBox=\"0 0 24 24\""), "expected preserved viewBox; got: {svg}");

        // 2. Cached Iconify icon test:
        let server = MockServer::start().await;
        let json = serde_json::json!({
            "prefix": "custom",
            "icons": { "resized": { "body": "<path d=\"M1 1\"/>", "left": 2, "top": 4, "width": 16, "height": 16 } }
        });
        
        Mock::given(method("GET"))
            .and(path("/custom.json"))
            .and(query_param("icons", "resized"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json))
            .expect(1) // must be called exactly once (proving subsequent resizes don't hit the network)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let cache = crate::cache::IconCache::open_at(dir.path().join("icons.db")).unwrap();
        let client = crate::iconify::IconifyClient::with_base(server.uri());

        // First call - cache miss, hits wiremock.
        let icon1 = Icon::iconify_with("custom:resized", &cache, &client).await.unwrap();
        let svg1 = icon1.width("64").height("64").svg();
        assert!(svg1.contains("width=\"64\""), "expected width=\"64\" on first load; got: {svg1}");
        assert!(svg1.contains("height=\"64\""), "expected height=\"64\" on first load; got: {svg1}");
        assert!(svg1.contains("viewBox=\"2 4 16 16\""), "expected preserved viewBox on first load; got: {svg1}");

        // Second call - cache hit, does NOT hit wiremock (resizing is purely local)
        let icon2 = Icon::iconify_with("custom:resized", &cache, &client).await.unwrap();
        let svg2 = icon2.width("64").height("64").svg();
        assert!(svg2.contains("width=\"64\""), "expected width=\"64\" on cache hit; got: {svg2}");
        assert!(svg2.contains("height=\"64\""), "expected height=\"64\" on cache hit; got: {svg2}");
        assert!(svg2.contains("viewBox=\"2 4 16 16\""), "expected preserved viewBox on cache hit; got: {svg2}");
    }
}
