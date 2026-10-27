//! Allowlist-based SVG sanitizer for safe raw-HTML emission.
//!
//! The browser render-tree path promotes a `mermaid` fence to a static `<svg>`
//! produced by Merman (via `biscuit-visualized`) and emits it
//! through [`define_as_raw_html`], which never escapes its input. That makes the
//! SVG an injection surface: a future renderer path — or an unescaped graph
//! label — could carry `<script>`, an `on*` event handler, an external
//! `xlink:href`, a `<foreignObject>` wrapping arbitrary HTML, or CSS
//! (`<style>`, `style="…"`, `filter="…"`) that fetches an external resource.
//! [`sanitize_svg`]
//! is the safety net at that boundary: it re-parses the SVG and re-emits only an
//! allowlisted element/attribute subset, so the emitted markup stays safe
//! independently of the upstream renderer or its string overrides.
//!
//! [`define_as_raw_html`]: renderable::browser::fragment::BrowserFragment::define_as_raw_html

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use cssparser::{Delimiter, ParseError, Parser, ParserInput, Token};
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

/// SVG local element names that are safe to re-emit.
///
/// Compared case-insensitively against the lowercased local name, so any
/// element whose lowercased name is absent — `script`, `foreignObject`,
/// `image`, `iframe`, the `animate*`/`set`/`fe*` families — is dropped
/// along with its entire subtree. The set is presentation-only: no scripting,
/// no external resource loading. It covers everything Merman emits plus the
/// standard SVG shape vocabulary. `<filter>` is absent on purpose: it is
/// admitted only through [`sanitize_filter_element`], which keeps bounded
/// drop shadows and nothing else.
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
/// references — a sanitized static SVG carries no live links). Any attribute
/// value whose CSS carries `@import` or a non-local `url(...)` is dropped too,
/// and so is any `<style>` block that fails [`style_sheet_is_safe`], blocking
/// external stylesheet/filter/image fetches. XML declarations, doctypes,
/// processing instructions, and comments are dropped.
///
/// Of the filter family only a `<filter>` holding `<feDropShadow>` primitives
/// survives, under the limits of [`sanitize_filter_element`]; every other `fe*`
/// primitive is dropped.
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
/// `<script>` becomes its own element and is dropped. `<style>` CSS is kept
/// for diagram fidelity but validated as a unit: a block carrying `@import` or
/// a non-local `url(...)` is dropped wholesale; the one exception is a
/// `data:font/ttf` URL in an `@font-face` `src` (see [`style_sheet_is_safe`]).
/// Every attribute value is checked without that exception, so `style` /
/// `filter` / `fill` cannot smuggle a `data:` or external `url(...)` — only
/// local `url(#fragment)` references survive.
pub fn sanitize_svg(svg: &str) -> Option<String> {
    let mut reader = Reader::from_str(svg);
    let mut writer = Writer::new(Vec::new());
    let mut buf = Vec::new();
    let mut skip_buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(e) => {
                let local = local_name_lower(e.name().as_ref());
                if local == b"style" {
                    // `<style>` is allowlisted for diagram fidelity, but its CSS
                    // is an external-reference surface (`@import`, `url(...)`).
                    // Validate the whole block and drop it wholesale when unsafe.
                    sanitize_style_element(&mut reader, &mut writer, &e)?;
                } else if local == b"filter" {
                    sanitize_filter_element(&mut reader, &mut writer, &e)?;
                } else if is_allowed_element(e.name().as_ref()) {
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
                // A disallowed empty element has no subtree; just drop it. This
                // includes `<filter/>` (no primitive, see
                // `sanitize_filter_element`) and any `fe*` outside a filter.
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
/// for `href` / `xlink:href` only a local `#…` fragment, and — for any other
/// attribute — no CSS that fetches an external resource.
fn attribute_is_safe(attr: &Attribute) -> bool {
    let local = local_name_lower(attr.key.as_ref());
    if local.starts_with(b"on") {
        return false;
    }
    if local == b"href" {
        return href_is_local_fragment(&attr.value);
    }
    // `style`, `filter`, `fill`, `clip-path`, `mask`, … can carry a CSS
    // `url(...)` or `@import` that fetches off-document. Allow only local
    // `#…` fragment targets; reject the attribute otherwise.
    match attr.unescape_value() {
        Ok(value) => css_is_safe(&value),
        Err(_) => false,
    }
}

/// Returns `true` when an attribute's CSS text loads no external resource: it
/// has no `@import`, and every `url(...)` target is a local `#…` fragment.
///
/// External `url(https://…)`, `url(//…)`, `url(data:…)`, and `@import` are the
/// SVG/CSS surfaces that fetch off-document; a presentation-only sanitized SVG
/// must carry none. Local `url(#gradient)` references stay — they point inside
/// the same document, so gradient/clip/marker fills survive. A `<style>`
/// element's CSS is checked by [`style_sheet_is_safe`] instead.
fn css_is_safe(css: &str) -> bool {
    let lower = css.to_ascii_lowercase();
    if lower.contains("@import") {
        return false;
    }
    for (idx, _) in lower.match_indices("url(") {
        let rest = lower[idx + "url(".len()..].trim_start();
        let rest = rest
            .strip_prefix('"')
            .or_else(|| rest.strip_prefix('\''))
            .unwrap_or(rest)
            .trim_start();
        if !rest.starts_with('#') {
            return false;
        }
    }
    true
}

/// Returns `true` when a style sheet (a `<style>` element's CSS) loads
/// nothing from outside the page.
///
/// The rule is [`css_is_safe`]'s — no `@import`, and every `url(...)` target a
/// local `#…` fragment — with one exception, the embedded Mermaid font: a
/// top-level `@font-face` rule whose single `src` descriptor is one or more
/// `url(data:font/ttf;base64,…) format("truetype")` entries with well-formed
/// base64. The scheme, MIME type, and `base64` are matched case-insensitively.
/// A `data:` URL anywhere else (another descriptor or property, another
/// at-rule, a nested rule) fails, as does a `data:` URL of another type or
/// form.
///
/// Fails closed: a sheet the tokenizer finds malformed (an unterminated
/// string, a `url(` with a space inside), an unterminated `@font-face` block
/// or font `url(`, a duplicate `src`, or an `@font-face` body that is not a
/// plain declaration list is rejected whole. Darkmatter runs the page-level
/// font style it generates through this same check before emitting it.
pub(crate) fn style_sheet_is_safe(css: &str) -> bool {
    // Kept from the substring check: `@import` anywhere, even in a comment or
    // string, fails the sheet.
    if css.to_ascii_lowercase().contains("@import") {
        return false;
    }
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    rules_are_safe(&mut parser, true).is_ok()
}

type CssCheck<'i> = Result<(), ParseError<'i, ()>>;

fn unsafe_css<'i>(parser: &Parser<'i, '_>) -> CssCheck<'i> {
    Err(parser.new_custom_error(()))
}

/// Walks every token of `parser`'s input, recursing into blocks and
/// functions. Only at the top level of the sheet does `@font-face` get its
/// font `src` exception.
fn rules_are_safe<'i>(parser: &mut Parser<'i, '_>, top_level: bool) -> CssCheck<'i> {
    loop {
        let Ok(token) = parser.next_including_whitespace_and_comments() else {
            return Ok(());
        };
        match token.clone() {
            Token::AtKeyword(name) if top_level && name.eq_ignore_ascii_case("font-face") => {
                font_face_rule(parser)?;
            }
            Token::AtKeyword(name) if name.eq_ignore_ascii_case("import") => return unsafe_css(parser),
            Token::UnquotedUrl(url) if url.trim_start().starts_with('#') => {}
            Token::Function(name) if name.eq_ignore_ascii_case("url") => {
                parser.parse_nested_block(|args| match quoted_url(args)? {
                    url if url.trim_start().starts_with('#') => Ok(()),
                    _ => unsafe_css(args),
                })?;
            }
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                parser.parse_nested_block(|block| rules_are_safe(block, false))?;
            }
            Token::UnquotedUrl(_) | Token::BadUrl(_) | Token::BadString(_) => return unsafe_css(parser),
            _ => {}
        }
    }
}

/// An `@font-face` rule after its at-keyword: an empty prelude, then a
/// terminated `{…}` block holding a declaration list with at most one `src`.
fn font_face_rule<'i>(parser: &mut Parser<'i, '_>) -> CssCheck<'i> {
    loop {
        let start = parser.position();
        match parser.next_including_whitespace_and_comments()? {
            Token::WhiteSpace(_) | Token::Comment(_) => {}
            Token::CurlyBracketBlock => {
                parser.parse_nested_block(font_face_body)?;
                if !parser.slice_from(start).ends_with('}') {
                    return unsafe_css(parser);
                }
                return Ok(());
            }
            _ => return unsafe_css(parser),
        }
    }
}

fn font_face_body<'i>(parser: &mut Parser<'i, '_>) -> CssCheck<'i> {
    let mut seen_src = false;
    loop {
        skip_whitespace(parser);
        let name = match parser.next() {
            Err(_) => return Ok(()),
            Ok(Token::Semicolon) => continue,
            Ok(Token::Ident(name)) => name.clone(),
            Ok(_) => return unsafe_css(parser),
        };
        skip_whitespace(parser);
        parser.expect_colon()?;
        let is_src = name.eq_ignore_ascii_case("src");
        if is_src && std::mem::replace(&mut seen_src, true) {
            // Two `src` descriptors: the browser keeps the last, so a check
            // that read the first would vouch for the wrong one.
            return unsafe_css(parser);
        }
        parser.parse_until_after(Delimiter::Semicolon, |value| {
            if is_src { font_src(value) } else { rules_are_safe(value, false) }
        })?;
    }
}

/// An `@font-face` `src` value: one or more comma-separated
/// `url(data:font/ttf;base64,…) format("truetype")` entries, the URL quoted or
/// not.
fn font_src<'i>(parser: &mut Parser<'i, '_>) -> CssCheck<'i> {
    loop {
        skip_whitespace(parser);
        let start = parser.position();
        let url = match parser.next()?.clone() {
            Token::UnquotedUrl(url) => url.to_string(),
            Token::Function(name) if name.eq_ignore_ascii_case("url") => {
                parser.parse_nested_block(quoted_url)?
            }
            _ => return unsafe_css(parser),
        };
        if !parser.slice_from(start).ends_with(')') || !is_font_data_url(&url) {
            return unsafe_css(parser);
        }
        skip_whitespace(parser);
        let start = parser.position();
        parser.expect_function_matching("format")?;
        parser.parse_nested_block(|args| {
            let format = args.expect_string()?.clone();
            args.expect_exhausted()?;
            if format.eq_ignore_ascii_case("truetype") { Ok(()) } else { unsafe_css(args) }
        })?;
        if !parser.slice_from(start).ends_with(')') {
            return unsafe_css(parser);
        }
        skip_whitespace(parser);
        match parser.next() {
            Err(_) => return Ok(()),
            Ok(Token::Comma) => {}
            Ok(_) => return unsafe_css(parser),
        }
    }
}

/// The string argument of a `url("…")` function; anything else in the
/// parentheses is an error.
fn quoted_url<'i>(parser: &mut Parser<'i, '_>) -> Result<String, ParseError<'i, ()>> {
    let url = parser.expect_string()?.to_string();
    parser.expect_exhausted()?;
    Ok(url)
}

/// Whether `url` is `data:font/ttf;base64,` followed by well-formed,
/// padded base64 with no whitespace.
fn is_font_data_url(url: &str) -> bool {
    const PREFIX: &str = "data:font/ttf;base64,";
    let Some(body) = url
        .get(..PREFIX.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(PREFIX))
        .map(|_| &url[PREFIX.len()..])
    else {
        return false;
    };
    !body.is_empty() && BASE64.decode(body).is_ok()
}

/// Skips whitespace and comments (`Parser::next` already skips both, but a
/// position read before a token must not include them).
fn skip_whitespace(parser: &mut Parser<'_, '_>) {
    loop {
        let state = parser.state();
        match parser.next_including_whitespace_and_comments() {
            Ok(Token::WhiteSpace(_) | Token::Comment(_)) => {}
            _ => {
                parser.reset(&state);
                return;
            }
        }
    }
}

/// Re-emits a `<style>` element only when its CSS passes
/// [`style_sheet_is_safe`]; otherwise drops the whole block.
///
/// The inner text is written with its original escaping preserved — CSS may
/// contain `>` child combinators that must not be entity-mangled — so the
/// captured `Text`/`CData` events are re-emitted verbatim rather than
/// round-tripped through unescape/escape.
///
/// ## Returns
///
/// `None` on malformed input (an unterminated `<style>`), so the caller fails
/// closed exactly as for any other parse error.
fn sanitize_style_element(
    reader: &mut Reader<&[u8]>,
    writer: &mut Writer<Vec<u8>>,
    start: &BytesStart,
) -> Option<()> {
    let mut css = String::new();
    let mut inner: Vec<Event<'static>> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Text(t) => {
                // Scan the raw (still-escaped) bytes: entity-encoding a `url(`
                // target only makes the `#…` locality check fail, which fails
                // closed. The original event is re-emitted verbatim below.
                css.push_str(&String::from_utf8_lossy(&t));
                inner.push(Event::Text(t).into_owned());
            }
            Event::CData(c) => {
                css.push_str(&String::from_utf8_lossy(&c));
                inner.push(Event::CData(c).into_owned());
            }
            Event::End(e) if local_name_lower(e.name().as_ref()) == b"style" => break,
            // A `<style>` should not wrap child elements; drop anything else.
            Event::Eof => return None,
            _ => {}
        }
        buf.clear();
    }
    if style_sheet_is_safe(&css) {
        writer.write_event(Event::Start(sanitize_start(start))).ok()?;
        for ev in inner {
            writer.write_event(ev).ok()?;
        }
        writer.write_event(Event::End(start.to_end())).ok()?;
    }
    Some(())
}

/// Largest `|dx|`, `|dy|`, and `stdDeviation` a kept `<feDropShadow>` may carry.
///
/// Bounds the blur radius and offset so a crafted diagram cannot make the
/// reader's browser render slowly.
const DROP_SHADOW_CAP: f64 = 20.0;

/// Re-emits a `<filter>` only as a bounded drop shadow; otherwise drops it with
/// its whole subtree.
///
/// Kept: the `<filter>` and its `<feDropShadow>` children, each only when every
/// attribute other than `id` is a number (a percentage counts, as in Merman's
/// `width="130%"` filter region) or a color, `|dx|` and `|dy|` are at most
/// [`DROP_SHADOW_CAP`], `stdDeviation` is one or two numbers each in
/// `0..=DROP_SHADOW_CAP`, and `flood-opacity` is a number in `0..=1`. Every
/// other child — `feImage`, `feTurbulence`, `feGaussianBlur`, any non-`fe`
/// element, and anything nested inside a `<feDropShadow>` — is dropped.
///
/// A `<filter>` left with no `<feDropShadow>` is dropped too: an empty filter
/// renders the element that references it as transparent, whereas a
/// `filter="url(#…)"` naming a missing filter is ignored and the element paints
/// unfiltered.
///
/// ## Returns
///
/// `None` on malformed input, so the caller fails closed.
fn sanitize_filter_element(
    reader: &mut Reader<&[u8]>,
    writer: &mut Writer<Vec<u8>>,
    start: &BytesStart,
) -> Option<()> {
    let filter_is_safe = filter_attributes_are_safe(start);
    let mut shadows: Vec<BytesStart<'static>> = Vec::new();
    let mut buf = Vec::new();
    let mut skip_buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(e) => {
                if is_safe_drop_shadow(&e) {
                    shadows.push(sanitize_start(&e));
                }
                reader.read_to_end_into(e.name(), &mut skip_buf).ok()?;
                skip_buf.clear();
            }
            Event::Empty(e) => {
                if is_safe_drop_shadow(&e) {
                    shadows.push(sanitize_start(&e));
                }
            }
            // Nested starts are consumed whole above, so this is the filter's own end.
            Event::End(_) => break,
            Event::Eof => return None,
            _ => {}
        }
        buf.clear();
    }
    if filter_is_safe && !shadows.is_empty() {
        writer.write_event(Event::Start(sanitize_start(start))).ok()?;
        for shadow in shadows {
            writer.write_event(Event::Empty(shadow)).ok()?;
        }
        writer.write_event(Event::End(start.to_end())).ok()?;
    }
    Some(())
}

/// Returns `true` when `element` is a `<feDropShadow>` within the limits of
/// [`sanitize_filter_element`].
fn is_safe_drop_shadow(element: &BytesStart) -> bool {
    local_name_lower(element.name().as_ref()) == b"fedropshadow"
        && filter_attributes_are_safe(element)
}

/// Returns `true` when every attribute of a `<filter>` or `<feDropShadow>` is
/// within the limits of [`sanitize_filter_element`].
fn filter_attributes_are_safe(element: &BytesStart) -> bool {
    element.attributes().with_checks(false).all(|attr| {
        let Ok(attr) = attr else {
            return false;
        };
        let Ok(value) = attr.unescape_value() else {
            return false;
        };
        // The full lowercased key: a prefixed `foo:dx` is not `dx`, so it must
        // pass as a plain number or color like any other attribute.
        match attr.key.as_ref().to_ascii_lowercase().as_slice() {
            b"id" => attribute_is_safe(&attr),
            b"dx" | b"dy" => parse_svg_number(&value).is_some_and(|n| n.abs() <= DROP_SHADOW_CAP),
            b"stddeviation" => std_deviation_is_safe(&value),
            b"flood-opacity" => parse_svg_number(&value).is_some_and(|n| (0.0..=1.0).contains(&n)),
            _ => is_number_or_percentage(&value) || is_color(&value),
        }
    })
}

/// Returns `true` when `value` is one number, or two separated by whitespace or
/// one comma, each in `0..=DROP_SHADOW_CAP`. A negative deviation is an SVG
/// error, so it is rejected rather than capped by magnitude.
fn std_deviation_is_safe(value: &str) -> bool {
    let parts: Vec<&str> = match value.split_once(',') {
        Some((first, second)) => vec![first.trim(), second.trim()],
        None => value.split_ascii_whitespace().collect(),
    };
    (1..=2).contains(&parts.len())
        && parts.iter().all(|part| {
            parse_svg_number(part).is_some_and(|n| (0.0..=DROP_SHADOW_CAP).contains(&n))
        })
}

/// Parses an SVG `<number>`: an optional sign, digits with an optional
/// fraction, and an optional exponent. No surrounding whitespace.
///
/// The grammar is checked before `f64::from_str`, which would also accept
/// `inf`, `NaN`, and `infinity`.
fn parse_svg_number(value: &str) -> Option<f64> {
    let bytes = value.as_bytes();
    let mut idx = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        idx += 1;
    }
    let int_start = idx;
    while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
        idx += 1;
    }
    let mut digits = idx - int_start;
    if bytes.get(idx) == Some(&b'.') {
        idx += 1;
        let frac_start = idx;
        while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
            idx += 1;
        }
        digits += idx - frac_start;
    }
    if digits == 0 {
        return None;
    }
    if matches!(bytes.get(idx), Some(b'e' | b'E')) {
        idx += 1;
        if matches!(bytes.get(idx), Some(b'+' | b'-')) {
            idx += 1;
        }
        let exp_start = idx;
        while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
            idx += 1;
        }
        if idx == exp_start {
            return None;
        }
    }
    if idx != bytes.len() {
        return None;
    }
    value.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Returns `true` for an SVG number or a number followed by `%`.
fn is_number_or_percentage(value: &str) -> bool {
    parse_svg_number(value.strip_suffix('%').unwrap_or(value)).is_some()
}

/// Returns `true` for a hex color (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`) or
/// an `rgb()`/`rgba()`/`hsl()`/`hsla()` whose three or four arguments are all
/// numbers or percentages. Named colors are not admitted.
fn is_color(value: &str) -> bool {
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    let lower = value.to_ascii_lowercase();
    ["rgb(", "rgba(", "hsl(", "hsla("].iter().any(|function| {
        lower
            .strip_prefix(function)
            .and_then(|rest| rest.strip_suffix(')'))
            .is_some_and(|args| {
                let parts: Vec<&str> = args
                    .split(|c: char| c == ',' || c == '/' || c.is_ascii_whitespace())
                    .filter(|part| !part.is_empty())
                    .collect();
                (3..=4).contains(&parts.len())
                    && parts.iter().all(|part| is_number_or_percentage(part))
            })
    })
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
    fn drops_style_block_with_import() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>@import url(https://attacker.example/x.css);</style><rect/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("@import"), "@import kept: {out}");
        assert!(!out.contains("attacker.example"), "external host kept: {out}");
        assert!(!out.contains("<style"), "unsafe style block kept: {out}");
        assert!(out.contains("<rect"), "safe sibling dropped: {out}");
    }

    #[test]
    fn drops_style_block_with_external_url() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>rect { fill: url(https://attacker.example/p.svg#x) }</style><rect/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("attacker.example"), "external url kept: {out}");
        assert!(!out.contains("<style"), "unsafe style block kept: {out}");
    }

    #[test]
    fn keeps_style_block_with_local_fragment_url() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>rect { fill: url(#grad) }</style><rect/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(out.contains("<style"), "benign style block dropped: {out}");
        assert!(out.contains("url(#grad)"), "local fragment url dropped: {out}");
    }

    #[test]
    fn drops_style_attribute_with_external_url() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect style="fill:url(https://attacker.example/p.svg#x)" width="1"/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("attacker.example"), "external url in style attr kept: {out}");
        assert!(!out.contains("style="), "unsafe style attr kept: {out}");
        // The element itself and its safe attribute survive.
        assert!(out.contains("<rect"), "rect element dropped: {out}");
        assert!(out.contains(r#"width="1""#), "safe attr dropped: {out}");
    }

    #[test]
    fn drops_filter_attribute_with_external_url() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect filter="url(https://attacker.example/filter.svg#f)" width="1"/></svg>"#;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("attacker.example"), "external filter url kept: {out}");
        assert!(!out.contains("filter="), "unsafe filter attr kept: {out}");
        assert!(out.contains("<rect"), "rect element dropped: {out}");
    }

    #[test]
    fn keeps_filter_attribute_with_local_fragment() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect filter="url(#f)" width="1"/></svg>"##;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(out.contains(r##"filter="url(#f)""##), "local fragment filter dropped: {out}");
    }


    // ── `@font-face` admission of the embedded Mermaid font ─────────────

    /// A well-formed base64 body (12 bytes).
    const FONT_BODY: &str = "AAEAAAALAIAAAwAw";

    /// The rule the embedded-font page style carries, exactly as generated.
    fn font_face(src: &str) -> String {
        format!("@font-face{{font-family:'Inter';font-style:normal;font-weight:400;src:{src}}}")
    }

    fn font_src(url: &str) -> String {
        format!("url({url}) format(\"truetype\")")
    }

    fn data_url() -> String {
        format!("data:font/ttf;base64,{FONT_BODY}")
    }

    /// The sanitized form of an SVG whose `<style>` holds `css`.
    fn sanitized_style(css: &str) -> String {
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg"><style>{css}</style><rect/></svg>"#);
        sanitize_svg(&svg).expect("well-formed")
    }

    fn assert_kept(css: &str) {
        let out = sanitized_style(css);
        assert!(out.contains(&format!("<style>{css}</style>")), "style dropped: {css}\n{out}");
        assert!(style_sheet_is_safe(css), "{css}");
    }

    fn assert_dropped(css: &str) {
        let out = sanitized_style(css);
        assert!(!out.contains("<style"), "style kept: {css}\n{out}");
        assert!(out.contains("<rect"), "safe sibling dropped: {out}");
        assert!(!style_sheet_is_safe(css), "{css}");
    }

    #[test]
    fn keeps_font_face_with_truetype_data_url_in_src() {
        // Control: the generated shape, with the OFL comment before it.
        let rule = font_face(&font_src(&data_url()));
        assert_kept(&format!("/* Inter: Copyright (c) 2016 The Inter Project Authors; SIL Open Font License 1.1 */\n{rule}\n"));
        // Two weights, and several `src` entries in one rule.
        let bold = rule.replace("font-weight:400", "font-weight:700");
        assert_kept(&format!("{rule}\n{bold}\n"));
        assert_kept(&font_face(&format!("{}, {}", font_src(&data_url()), font_src(&data_url()))));
        // The URL quoted, and the spec's example (double-quoted family).
        assert_kept(&font_face(&font_src(&format!("\"{}\"", data_url()))));
        assert_kept(&format!(r#"@font-face{{font-family:"Inter";src:url({}) format("truetype")}}"#, data_url()));
    }

    #[test]
    fn font_data_url_admission_is_case_insensitive_for_scheme_and_mime() {
        assert_kept(&font_face(&format!("URL(DATA:FONT/TTF;BASE64,{FONT_BODY}) FORMAT(\"TRUETYPE\")")));
        assert_kept(&format!("@FONT-FACE{{SRC:url(Data:Font/Ttf;Base64,{FONT_BODY}) format(\"truetype\")}}"));
    }

    #[test]
    fn drops_font_face_whose_src_is_not_an_embedded_truetype_data_url() {
        let cases = [
            ("https URL", "https://attacker.example/f.ttf".to_string()),
            ("protocol-relative URL", "//attacker.example/f.ttf".to_string()),
            ("PNG data", format!("data:image/png;base64,{FONT_BODY}")),
            ("WOFF2 data", format!("data:font/woff2;base64,{FONT_BODY}")),
            ("OTF data", format!("data:font/otf;base64,{FONT_BODY}")),
            ("HTML data", format!("data:text/html;base64,{FONT_BODY}")),
            ("not base64", "data:font/ttf,%00%01".to_string()),
            ("extra parameter", format!("data:font/ttf;charset=utf-8;base64,{FONT_BODY}")),
            ("invalid base64 character", "data:font/ttf;base64,AAEA*AALAIAAAwAw".to_string()),
            ("unpadded base64", "data:font/ttf;base64,AAEAAAALAIAAAwA".to_string()),
            ("empty body", "data:font/ttf;base64,".to_string()),
            ("local fragment", "#font".to_string()),
        ];
        for (case, url) in cases {
            let css = font_face(&font_src(&url));
            assert!(!style_sheet_is_safe(&css), "{case}: {css}");
            assert_dropped(&css);
        }
        // Whitespace inside the body is a malformed `url(` token.
        assert_dropped(&font_face(&font_src("data:font/ttf;base64,AAEA AALAIAAAwAw")));
        // A valid entry followed by an external one.
        assert_dropped(&font_face(&format!(
            "{}, {}",
            font_src(&data_url()),
            font_src("https://attacker.example/f.ttf")
        )));
        // `format()` must be "truetype" and alone; the URL must carry one.
        assert_dropped(&font_face(&format!("url({}) format(\"woff2\")", data_url())));
        assert_dropped(&font_face(&format!(
            "url({}) format(\"truetype\", url(https://attacker.example/f))",
            data_url()
        )));
        assert_dropped(&font_face(&format!("url({})", data_url())));
        assert_dropped(&font_face(&format!("local(Inter), {}", font_src(&data_url()))));
    }

    #[test]
    fn drops_truetype_data_url_outside_font_face_src() {
        let url = data_url();
        // Another `@font-face` descriptor.
        assert_dropped(&format!(
            "@font-face{{font-family:'Inter';font-display:url({url});src:{}}}",
            font_src(&url)
        ));
        // An ordinary rule's properties.
        assert_dropped(&format!("rect{{background:url({url})}}"));
        assert_dropped(&format!("rect{{mask:url({url})}}"));
        assert_dropped(&format!("rect{{fill:url({url})}}"));
        // Other at-rules, and an `@font-face` nested in one or in a rule.
        assert_dropped(&format!("@media screen{{{}}}", font_face(&font_src(&url))));
        assert_dropped(&format!("@supports (display:grid){{{}}}", font_face(&font_src(&url))));
        assert_dropped(&format!("@keyframes k{{from{{background:url({url})}}}}"));
        assert_dropped(&format!("g{{{}}}", font_face(&font_src(&url))));
    }

    #[test]
    fn drops_truetype_data_url_in_attributes() {
        let url = data_url();
        for attr in ["fill", "filter", "mask", "style"] {
            let value = if attr == "style" { format!("fill:url({url})") } else { format!("url({url})") };
            let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg"><rect {attr}="{value}" width="1"/></svg>"#);
            let out = sanitize_svg(&svg).expect("well-formed");
            assert!(!out.contains("data:"), "{attr}: {out}");
            assert!(out.contains(r#"width="1""#), "{attr}: safe attr dropped: {out}");
        }
        // An `@font-face` rule spelled into a `style` attribute is not a style sheet.
        let style = font_face(&font_src(&url)).replace('\'', "");
        let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg"><rect style="{style}"/></svg>"#);
        let out = sanitize_svg(&svg).expect("well-formed");
        assert!(!out.contains("data:"), "{out}");
    }

    #[test]
    fn drops_font_face_with_import_duplicate_src_or_unterminated_parts() {
        let rule = font_face(&font_src(&data_url()));
        // `@import` beside the rule, inside it, and escaped.
        assert_dropped(&format!("@import url(#x);{rule}"));
        assert_dropped(&format!(
            "@font-face{{@import url(#x);src:{}}}",
            font_src(&data_url())
        ));
        assert_dropped(&format!("@\\69mport url(#x);{rule}"));
        // Two `src` descriptors, even when both are well formed.
        assert_dropped(&format!(
            "@font-face{{src:{};src:{}}}",
            font_src(&data_url()),
            font_src(&data_url())
        ));
        // Unterminated block, `url(`, and `format(`.
        assert_dropped(&rule[..rule.len() - 1]);
        assert_dropped(&format!("@font-face{{src:url({}", data_url()));
        assert_dropped(&format!("@font-face{{src:url({}) format(\"truetype\"", data_url()));
        // A prelude, or a body that is not a declaration list.
        assert_dropped(&format!("@font-face x{{src:{}}}", font_src(&data_url())));
        assert_dropped(&format!("@font-face{{g{{}} src:{}}}", font_src(&data_url())));
    }

    #[test]
    fn font_face_beside_other_rules_is_kept_only_when_every_rule_passes() {
        let rule = font_face(&font_src(&data_url()));
        assert_kept(&format!("{rule}\nrect{{fill:url(#grad);stroke:red}}"));
        assert_dropped(&format!("{rule}\nrect{{fill:url(https://attacker.example/p.svg#x)}}"));
        assert_dropped(&format!("rect{{fill:url(//attacker.example/p.svg)}}\n{rule}"));
    }


    /// Merman's drop-shadow defs exactly as its ER, state, class, mindmap,
    /// requirement, block, and flowchart renderers emit them (light theme).
    const MERMAN_SHADOW_DEFS: &str = r##"<defs><filter id="bv-2f30a68e5cebaffa-drop-shadow" height="130%" width="130%"><feDropShadow dx="4" dy="4" stdDeviation="0" flood-opacity="0.06" flood-color="#000000"/></filter></defs><defs><filter id="bv-2f30a68e5cebaffa-drop-shadow-small" height="150%" width="150%"><feDropShadow dx="2" dy="2" stdDeviation="0" flood-opacity="0.06" flood-color="#000000"/></filter></defs>"##;

    /// Wraps one `<feDropShadow>` attribute list in a referenced filter.
    fn shadow_svg(shadow_attrs: &str) -> String {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg"><defs><filter id="f" width="130%"><feDropShadow {shadow_attrs}/></filter></defs><rect filter="url(#f)" width="1"/></svg>"##
        )
    }

    fn keeps_shadow(shadow_attrs: &str) -> bool {
        let out = sanitize_svg(&shadow_svg(shadow_attrs)).expect("well-formed");
        out.contains("<feDropShadow")
    }

    #[test]
    fn keeps_merman_drop_shadow_verbatim() {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg">{MERMAN_SHADOW_DEFS}<rect filter="url(#bv-2f30a68e5cebaffa-drop-shadow)" width="1"/></svg>"##
        );
        let out = sanitize_svg(&svg).expect("well-formed");
        assert!(out.contains(MERMAN_SHADOW_DEFS), "Merman shadow altered or dropped: {out}");
        assert!(
            out.contains(r##"filter="url(#bv-2f30a68e5cebaffa-drop-shadow)""##),
            "local filter reference dropped: {out}"
        );
    }

    #[test]
    fn drop_shadow_offsets_are_capped_by_magnitude() {
        for attr in ["dx", "dy"] {
            assert!(keeps_shadow(&format!(r#"{attr}="20""#)), "{attr} at cap dropped");
            assert!(keeps_shadow(&format!(r#"{attr}="-20""#)), "{attr} at negative cap dropped");
            assert!(keeps_shadow(&format!(r#"{attr}="0""#)), "{attr} zero dropped");
            assert!(!keeps_shadow(&format!(r#"{attr}="20.001""#)), "{attr} above cap kept");
            assert!(!keeps_shadow(&format!(r#"{attr}="-21""#)), "{attr} below negative cap kept");
            assert!(!keeps_shadow(&format!(r#"{attr}="2e2""#)), "{attr} exponent above cap kept");
        }
    }

    #[test]
    fn drop_shadow_std_deviation_is_capped() {
        assert!(keeps_shadow(r#"stdDeviation="20""#), "stdDeviation at cap dropped");
        assert!(keeps_shadow(r#"stdDeviation="0""#), "stdDeviation zero dropped");
        assert!(keeps_shadow(r#"stdDeviation="20 20""#), "two values at cap dropped");
        assert!(keeps_shadow(r#"stdDeviation="3,4""#), "comma pair dropped");
        assert!(!keeps_shadow(r#"stdDeviation="21""#), "stdDeviation above cap kept");
        assert!(!keeps_shadow(r#"stdDeviation="20 21""#), "second value above cap kept");
        assert!(!keeps_shadow(r#"stdDeviation="-1""#), "negative stdDeviation kept");
        assert!(!keeps_shadow(r#"stdDeviation="1 2 3""#), "three values kept");
        assert!(!keeps_shadow(r#"stdDeviation="""#), "empty stdDeviation kept");
    }

    #[test]
    fn drop_shadow_flood_opacity_is_limited_to_unit_range() {
        assert!(keeps_shadow(r#"flood-opacity="0""#), "flood-opacity 0 dropped");
        assert!(keeps_shadow(r#"flood-opacity="1""#), "flood-opacity 1 dropped");
        assert!(!keeps_shadow(r#"flood-opacity="1.01""#), "flood-opacity above 1 kept");
        assert!(!keeps_shadow(r#"flood-opacity="-0.01""#), "flood-opacity below 0 kept");
        assert!(!keeps_shadow(r#"flood-opacity="50%""#), "percentage flood-opacity kept");
    }

    #[test]
    fn drop_shadow_numbers_reject_empty_and_non_numeric_values() {
        for value in ["", " 4", "4px", "inf", "NaN", "infinity", ".", "1e", "--1", "1.2.3"] {
            assert!(!keeps_shadow(&format!(r#"dx="{value}""#)), "dx={value:?} kept");
        }
        assert!(keeps_shadow(r#"dx="+1.5e1""#), "signed exponent number dropped");
        assert!(keeps_shadow(r#"dx=".5""#), "leading-dot number dropped");
    }

    #[test]
    fn drop_shadow_colors_accept_hex_and_numeric_functions_only() {
        let kept = [
            "#000",
            "#000F",
            "#FFFFFF",
            "#00000080",
            "rgb(0, 0, 0)",
            "rgba(185,185,185,0.5)",
            "hsl(120 50% 50% / 0.4)",
        ];
        for color in kept {
            assert!(keeps_shadow(&format!(r#"flood-color="{color}""#)), "color {color:?} dropped");
        }
        let rejected = [
            "",
            "#",
            "#12345",
            "#GGGGGG",
            "red",
            "url(#x)",
            "rgb(0, 0)",
            "rgb(a, b, c)",
            "rgb(0 0 0",
            "var(--c)",
        ];
        for color in rejected {
            assert!(!keeps_shadow(&format!(r#"flood-color="{color}""#)), "color {color:?} kept");
        }
    }

    #[test]
    fn drop_shadow_with_keyword_attribute_is_dropped() {
        assert!(!keeps_shadow(r#"in="SourceGraphic" dx="1""#), "keyword `in` kept");
        assert!(!keeps_shadow(r#"result="shadow" dx="1""#), "keyword `result` kept");
        assert!(!keeps_shadow(r#"dx="1" onload="alert(1)""#), "event handler kept");
    }

    #[test]
    fn filter_with_non_numeric_attribute_is_dropped_with_its_children() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><filter id="f" filterUnits="userSpaceOnUse"><feDropShadow dx="1"/></filter><rect/></svg>"##;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("<filter"), "filter with keyword attribute kept: {out}");
        assert!(!out.contains("feDropShadow"), "child of dropped filter kept: {out}");
        assert!(out.contains("<rect"), "safe sibling dropped: {out}");
    }

    #[test]
    fn filter_region_accepts_numbers_and_percentages_only() {
        let keeps = |region: &str| {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg"><filter id="f" {region}><feDropShadow dx="1"/></filter></svg>"#
            );
            sanitize_svg(&svg).expect("well-formed").contains("<filter")
        };
        assert!(keeps(r#"x="-20%" y="-20%" width="140%" height="140%""#), "percent region dropped");
        assert!(keeps(r#"x="0" width="10.5""#), "number region dropped");
        assert!(!keeps(r#"width="""#), "empty region kept");
        assert!(!keeps(r#"width="%""#), "bare percent kept");
        assert!(!keeps(r#"width="130 %""#), "spaced percent kept");
    }

    #[test]
    fn other_filter_primitives_are_dropped() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><filter id="f"><feImage href="https://attacker.example/x.png"/><feTurbulence baseFrequency="0.9" numOctaves="10"/><feGaussianBlur stdDeviation="1"/><feDropShadow dx="1"/></filter></svg>"##;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("feImage"), "feImage kept: {out}");
        assert!(!out.contains("attacker.example"), "feImage target kept: {out}");
        assert!(!out.contains("feTurbulence"), "feTurbulence kept: {out}");
        assert!(!out.contains("feGaussianBlur"), "feGaussianBlur kept: {out}");
        assert!(out.contains("<feDropShadow"), "sibling drop shadow dropped: {out}");
    }

    #[test]
    fn filter_left_without_a_drop_shadow_is_dropped() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><filter id="a"><feImage href="#x"/></filter><filter id="b"><feDropShadow dx="99"/></filter><filter id="c"/><filter id="d"></filter><rect/></svg>"##;
        let out = sanitize_svg(svg).expect("well-formed");
        assert!(!out.contains("<filter"), "empty filter kept: {out}");
        assert!(out.contains("<rect"), "safe sibling dropped: {out}");
    }

    #[test]
    fn drop_shadow_outside_a_filter_and_its_children_are_dropped() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><feDropShadow dx="1"/><filter id="f"><feDropShadow dx="1"><animate attributeName="dx" to="9999"/></feDropShadow></filter></svg>"##;
        let out = sanitize_svg(svg).expect("well-formed");
        assert_eq!(out.matches("<feDropShadow").count(), 1, "stray drop shadow kept: {out}");
        assert!(!out.contains("<animate"), "drop shadow child kept: {out}");
    }

    #[test]
    fn drops_external_url_on_filter_attribute_of_shadowed_element() {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg">{MERMAN_SHADOW_DEFS}<g filter="url(https://attacker.example/f.svg#bv-2f30a68e5cebaffa-drop-shadow)"><rect/></g></svg>"##
        );
        let out = sanitize_svg(&svg).expect("well-formed");
        assert!(!out.contains("attacker.example"), "external filter url kept: {out}");
        assert!(out.contains("<feDropShadow"), "local shadow dropped: {out}");
    }

    #[test]
    fn unterminated_filter_returns_none() {
        assert!(sanitize_svg(r#"<svg><filter id="f"><feDropShadow dx="1"/></svg>"#).is_none());
    }

    /// Text content of every `<text>` element, in document order.
    fn label_texts(svg: &str) -> Vec<String> {
        let mut reader = Reader::from_str(svg);
        let mut labels = Vec::new();
        let mut depth = 0usize;
        loop {
            match reader.read_event().expect("well-formed") {
                Event::Start(e) if e.name().as_ref() == b"text" => {
                    depth += 1;
                    labels.push(String::new());
                }
                Event::End(e) if e.name().as_ref() == b"text" => depth -= 1,
                Event::Text(t) if depth > 0 => {
                    let text = t.decode().expect("utf-8");
                    labels.last_mut().expect("open label").push_str(&text);
                }
                Event::GeneralRef(r) if depth > 0 => {
                    let name = r.decode().expect("utf-8");
                    labels.last_mut().expect("open label").push_str(&format!("&{name};"));
                }
                Event::Eof => break,
                _ => {}
            }
        }
        labels
    }

    #[test]
    fn merman_flowchart_survives_with_labels_and_shadow() {
        let source = "flowchart LR\n  A[Start here] --> B{Is it?}\n  \
                      B -->|Yes| C(Tom & Jerry)\n  B -->|No| D[(Store)]";
        let svg = biscuit_visualized::components::mermaid::MermaidDiagram::new(source)
            .render_to_svg()
            .expect("Merman renders the flowchart");
        let out = sanitize_svg(&svg).expect("Merman SVG is well-formed");

        let before = label_texts(&svg);
        let after = label_texts(&out);
        assert_eq!(after, before, "label text changed by sanitization");
        for label in ["Start here", "Is it?", "Yes", "No", "Tom &amp; Jerry", "Store"] {
            assert!(
                after.iter().any(|text| text.contains(label)),
                "label {label:?} missing: {after:?}"
            );
        }
        assert!(svg.contains("<feDropShadow"), "fixture no longer exercises a shadow: {svg}");
        assert_eq!(
            out.matches("<feDropShadow").count(),
            svg.matches("<feDropShadow").count(),
            "Merman drop shadow dropped: {out}"
        );
        assert!(out.contains("<style"), "Merman's scoped stylesheet dropped: {out}");
    }

    #[test]
    fn malformed_xml_returns_none() {
        assert!(sanitize_svg("<svg><rect></svg>").is_none(), "unbalanced tags must fail closed");
        assert!(sanitize_svg("not xml < at all").is_none());
    }
}
