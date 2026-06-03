//! Allowlist-based SVG sanitizer for safe raw-HTML emission.
//!
//! The browser render-tree path promotes a `mermaid` fence to a static `<svg>`
//! produced by `mermaid-rs-renderer` (via `biscuit-visualized`) and emits it
//! through [`define_as_raw_html`], which never escapes its input. That makes the
//! SVG an injection surface: a future renderer path — or an unescaped graph
//! label — could carry `<script>`, an `on*` event handler, an external
//! `xlink:href`, or a `<foreignObject>` wrapping arbitrary HTML. [`sanitize_svg`]
//! is the safety net at that boundary: it re-parses the SVG and re-emits only an
//! allowlisted element/attribute subset, so the emitted markup stays safe
//! independently of the upstream renderer or its string overrides.
//!
//! [`define_as_raw_html`]: renderable::browser::fragment::BrowserFragment::define_as_raw_html

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

/// SVG local element names that are safe to re-emit.
///
/// Compared case-insensitively against the lowercased local name, so any
/// element whose lowercased name is absent — `script`, `foreignObject`,
/// `image`, `iframe`, the `animate*`/`set`/`filter`/`fe*` families — is dropped
/// along with its entire subtree. The set is presentation-only: no scripting,
/// no external resource loading. It covers everything `mermaid-rs-renderer`
/// emits plus the standard SVG shape vocabulary.
const ALLOWED_ELEMENTS: &[&str] = &[
    "svg",
    "g",
    "defs",
    "symbol",
    "use",
    "title",
    "desc",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "text",
    "tspan",
    "textpath",
    "marker",
    "clippath",
    "mask",
    "pattern",
    "lineargradient",
    "radialgradient",
    "stop",
    "style",
    "a",
];

/// Re-emits `svg` keeping only [`ALLOWED_ELEMENTS`] and safe attributes.
///
/// Dropped: every non-allowlisted element (with its subtree), every `on*`
/// event-handler attribute, and any `href` / `xlink:href` that is not a local
/// `#…` fragment (which blocks `javascript:`, `data:`, and external/SSRF
/// references — a sanitized static SVG carries no live links). XML
/// declarations, doctypes, processing instructions, and comments are dropped.
///
/// ## Returns
///
/// The sanitized SVG, or `None` when the input is not well-formed XML — in
/// which case the caller must treat promotion as failed and **not** emit the
/// string as raw HTML.
///
/// ## Notes
///
/// Because sanitization is parse-based, a payload that tries to break framing
/// (`</text><script>…`, `]]><script>…`) is re-tokenized: the injected
/// `<script>` becomes its own element and is dropped. `<style>` CSS text is
/// preserved for diagram fidelity; the residual surface is CSS `url()` /
/// `@import` fetches, which cannot execute script and which the controlled
/// renderer does not emit.
pub fn sanitize_svg(svg: &str) -> Option<String> {
    let mut reader = Reader::from_str(svg);
    let mut writer = Writer::new(Vec::new());
    let mut buf = Vec::new();
    let mut skip_buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(e) => {
                if is_allowed_element(e.name().as_ref()) {
                    writer.write_event(Event::Start(sanitize_start(&e))).ok()?;
                } else {
                    // Drop the disallowed element and its entire subtree. This
                    // consumes the matching end tag, so no stray `End` reaches
                    // the loop below.
                    reader.read_to_end_into(e.name(), &mut skip_buf).ok()?;
                    skip_buf.clear();
                }
            }
            Event::Empty(e) => {
                if is_allowed_element(e.name().as_ref()) {
                    writer.write_event(Event::Empty(sanitize_start(&e))).ok()?;
                }
                // A disallowed empty element has no subtree; just drop it.
            }
            // Only allowed-element ends reach here (disallowed subtrees are
            // consumed above), so write them straight through.
            Event::End(e) => writer.write_event(Event::End(e)).ok()?,
            Event::Text(e) => writer.write_event(Event::Text(e)).ok()?,
            // quick-xml delivers `&amp;` / `&#x27;` as their own event; re-emit
            // them so escaped text/attribute-adjacent entities are not lost.
            Event::GeneralRef(e) => writer.write_event(Event::GeneralRef(e)).ok()?,
            // CDATA content is character data; re-tokenization already split off
            // any trailing markup, so the inner text is inert. Re-emit verbatim
            // to preserve `<style>` CSS.
            Event::CData(e) => writer.write_event(Event::CData(e)).ok()?,
            Event::Eof => break,
            // Decl / PI / DocType / Comment are intentionally dropped.
            _ => {}
        }
        buf.clear();
    }

    String::from_utf8(writer.into_inner()).ok()
}

/// Rebuilds `start` (a `Start` or `Empty` tag) preserving its name but keeping
/// only safe attributes.
///
/// Each value is fully unescaped and then re-escaped for a double-quoted
/// attribute context (`quick-xml`'s [`push_attribute`] writes the value
/// verbatim — it does not escape), so a single-quoted source value carrying a
/// literal `"` cannot break out, and entities round-trip without doubling.
///
/// [`push_attribute`]: quick_xml::events::BytesStart::push_attribute
fn sanitize_start(start: &BytesStart) -> BytesStart<'static> {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut out = BytesStart::new(name);
    for attr in start.attributes().with_checks(false).flatten() {
        if attribute_is_safe(&attr)
            && let Ok(value) = attr.unescape_value()
        {
            let escaped = quick_xml::escape::escape(value.as_ref());
            out.push_attribute((attr.key.as_ref(), escaped.as_bytes()));
        }
    }
    out
}

/// Returns `true` when `name`'s lowercased local part is in [`ALLOWED_ELEMENTS`].
fn is_allowed_element(name: &[u8]) -> bool {
    let local = local_name_lower(name);
    ALLOWED_ELEMENTS
        .iter()
        .any(|allowed| allowed.as_bytes() == local.as_slice())
}

/// Returns `true` when `attr` is safe to re-emit: not an `on*` event handler,
/// and — for `href` / `xlink:href` — only a local `#…` fragment reference.
fn attribute_is_safe(attr: &Attribute) -> bool {
    let local = local_name_lower(attr.key.as_ref());
    if local.starts_with(b"on") {
        return false;
    }
    if local == b"href" {
        return href_is_local_fragment(&attr.value);
    }
    true
}

/// Returns `true` only when `value` (ignoring leading ASCII whitespace) begins
/// with `#`, i.e. an in-document fragment reference. Everything else — absolute
/// URLs, `javascript:`, `data:`, scheme-relative — is rejected.
fn href_is_local_fragment(value: &[u8]) -> bool {
    let trimmed = value
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .map(|i| &value[i..])
        .unwrap_or(&[]);
    trimmed.first() == Some(&b'#')
}

/// Lowercases an attribute/element name and strips any namespace prefix
/// (`xlink:href` → `href`, `clipPath` → `clippath`).
fn local_name_lower(name: &[u8]) -> Vec<u8> {
    let local = match name.iter().position(|&b| b == b':') {
        Some(i) => &name[i + 1..],
        None => name,
    };
    local.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_element_and_subtree() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script><rect x="0"/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("<script"), "script element kept: {out}");
        assert!(!out.contains("alert(1)"), "script body kept: {out}");
        assert!(out.contains("<rect"), "safe sibling dropped: {out}");
    }

    #[test]
    fn strips_event_handler_attributes() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect onload="alert(1)" onclick="x()" fill="red"/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("onload"), "onload kept: {out}");
        assert!(!out.contains("onclick"), "onclick kept: {out}");
        assert!(out.contains(r#"fill="red""#), "safe attr dropped: {out}");
    }

    #[test]
    fn strips_foreign_object_subtree() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><foreignObject><body xmlns="http://www.w3.org/1999/xhtml"><img src=x onerror="alert(1)"/></body></foreignObject><g/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("foreignObject"), "foreignObject kept: {out}");
        assert!(!out.contains("<img"), "embedded html kept: {out}");
        assert!(!out.contains("onerror"), "handler kept: {out}");
        assert!(out.contains("<g"), "safe sibling dropped: {out}");
    }

    #[test]
    fn strips_external_and_javascript_hrefs() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><a xlink:href="javascript:alert(1)"><text>x</text></a><use xlink:href="https://evil.example/x.svg#a"/><use xlink:href="#local"/></svg>"##;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("javascript:"), "js href kept: {out}");
        assert!(!out.contains("evil.example"), "external href kept: {out}");
        assert!(out.contains("#local"), "local fragment ref dropped: {out}");
        // The `<a>`/`<text>` structure survives, just without the live link.
        assert!(out.contains("<text") || out.contains("&lt;text"), "text dropped: {out}");
    }

    #[test]
    fn preserves_benign_diagram_markup() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50"><g class="node"><rect x="1" y="2" width="10" height="5" fill="#ff00ff"/><text x="3" y="4">A &amp; B</text></g></svg>"##;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(out.contains("<svg"), "svg root dropped: {out}");
        assert!(out.contains("viewBox"), "viewBox dropped: {out}");
        assert!(out.contains(r##"fill="#ff00ff""##), "fill dropped: {out}");
        // The escaped `&` must round-trip as a single entity, not double-escaped.
        assert!(out.contains("A &amp; B"), "text not preserved cleanly: {out}");
        assert!(!out.contains("&amp;amp;"), "attribute/text double-escaped: {out}");
    }

    #[test]
    fn case_obfuscated_script_is_dropped() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><ScRiPt>alert(1)</ScRiPt><rect/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.to_ascii_lowercase().contains("script"), "cased script kept: {out}");
        assert!(!out.contains("alert(1)"), "script body kept: {out}");
    }

    #[test]
    fn malformed_xml_returns_none() {
        assert!(sanitize_svg("<svg><rect></svg>").is_none(), "unbalanced tags must fail closed");
        assert!(sanitize_svg("not xml < at all").is_none());
    }
}
