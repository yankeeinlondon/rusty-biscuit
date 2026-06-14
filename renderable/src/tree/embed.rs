//! Loss-free embedding of a fully-styled render subtree inside Markdown text.
//!
//! A component projects its render tree once, [`encode_embedded_subtree`]
//! serializes that subtree into a Markdown-safe block, and a Markdown fold that
//! recognizes the markers (via [`decode_embedded_open`] / [`is_embedded_close`])
//! splices the exact subtree back in. Styling that plain Markdown cannot express
//! — color, dim, icon spans — therefore survives a `render → text → fold →
//! render` round-trip with no fidelity loss and no re-computation.
//!
//! ## Wire format
//!
//! ```text
//! <!--bt:render-tree:v1:HEX-->
//! <portable Markdown fallback>
//! <!--bt:render-tree:end-->
//! ```
//!
//! The opening marker is a single-line HTML comment carrying the hex-encoded
//! JSON of the subtree. Hex (not raw JSON) keeps the byte sequence `--` out of
//! the comment, which an HTML comment may not contain. The lines between the
//! markers are a best-effort portable rendering shown to consumers that do not
//! recognize the markers; a recognizing fold drops them in favor of the decoded
//! subtree.

use super::error::{RenderError, RenderStrictness};
use super::node::RenderNode;
use super::render::{MarkdownRenderOptions, render_markdown_node};

/// Opening-marker prefix: the hex payload and `-->` suffix follow on one line.
pub const EMBED_OPEN_PREFIX: &str = "<!--bt:render-tree:v1:";

/// Suffix that closes the opening-marker HTML comment.
pub const EMBED_MARKER_SUFFIX: &str = "-->";

/// The closing marker that ends an embedded-subtree region.
pub const EMBED_CLOSE: &str = "<!--bt:render-tree:end-->";

/// An error encoding a subtree into its embedded Markdown form.
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    /// The subtree could not be serialized to JSON.
    #[error("failed to serialize embedded subtree: {0}")]
    Serialize(#[from] serde_json::Error),
    /// The portable Markdown fallback could not be rendered.
    #[error("failed to render embedded fallback: {0}")]
    Render(#[from] RenderError),
}

/// Encodes `node` as an embedded-subtree block: a marker comment carrying the
/// hex-encoded subtree, the portable Markdown fallback, and a closing marker.
///
/// ## Errors
///
/// Returns [`EmbedError`] when the subtree cannot be serialized or its portable
/// fallback cannot be rendered.
pub fn encode_embedded_subtree(node: &RenderNode) -> Result<String, EmbedError> {
    let json = serde_json::to_vec(node)?;
    let hex = to_hex(&json);
    let opts = MarkdownRenderOptions {
        strictness: RenderStrictness::Lossy,
        ..MarkdownRenderOptions::default()
    };
    let fallback = render_markdown_node(node, &opts)?.output;
    // Blank lines isolate the marker comments and the fallback as distinct
    // Markdown blocks, so a parser cannot fold them into one another.
    Ok(format!(
        "{EMBED_OPEN_PREFIX}{hex}{EMBED_MARKER_SUFFIX}\n\n{fallback}\n\n{EMBED_CLOSE}"
    ))
}

/// Decodes the subtree carried by an opening-marker HTML comment.
///
/// Returns `None` when `html` is not a well-formed opening marker or its payload
/// fails to decode, so a fold can fall back to treating the comment as ordinary
/// HTML.
pub fn decode_embedded_open(html: &str) -> Option<RenderNode> {
    let trimmed = html.trim();
    let hex = trimmed
        .strip_prefix(EMBED_OPEN_PREFIX)?
        .strip_suffix(EMBED_MARKER_SUFFIX)?;
    let bytes = from_hex(hex)?;
    serde_json::from_slice(&bytes).ok()
}

/// Whether `html` is the closing marker of an embedded-subtree region.
pub fn is_embedded_close(html: &str) -> bool {
    html.trim() == EMBED_CLOSE
}

/// Lowercase hex encoding of `bytes`.
fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit((byte >> 4) as u32, 16).unwrap());
        out.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap());
    }
    out
}

/// Decodes a lowercase/uppercase hex string, returning `None` on malformed input.
fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let bytes = hex.as_bytes();
    let mut out = Vec::with_capacity(hex.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
        i += 2;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Style, TextEmphasis};

    /// Builds a subtree whose styling (dim + classes) has no plain-Markdown
    /// form, so a successful round-trip proves the embed path is lossless.
    fn styled_subtree() -> RenderNode {
        let mut span =
            RenderNode::span(vec!["fs-dir".to_string()], vec![RenderNode::text("topics")]);
        span.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                dim: true,
                ..TextEmphasis::default()
            },
            ..Style::default()
        });
        RenderNode::root(vec![span])
    }

    #[test]
    fn encode_then_decode_round_trips_styled_subtree() {
        let node = styled_subtree();
        let encoded = encode_embedded_subtree(&node).unwrap();
        let open_line = encoded.lines().next().unwrap();
        let decoded = decode_embedded_open(open_line).expect("opening marker decodes");
        assert_eq!(decoded, node);
    }

    #[test]
    fn encoded_block_has_open_and_close_markers() {
        let encoded = encode_embedded_subtree(&styled_subtree()).unwrap();
        assert!(encoded.starts_with(EMBED_OPEN_PREFIX));
        assert!(encoded.trim_end().ends_with(EMBED_CLOSE));
        assert!(is_embedded_close(encoded.lines().last().unwrap()));
    }

    #[test]
    fn decode_rejects_non_marker_html() {
        assert!(decode_embedded_open("<!-- ordinary comment -->").is_none());
        assert!(decode_embedded_open("<div>not a marker</div>").is_none());
    }

    #[test]
    fn decode_rejects_corrupt_payload() {
        // Well-formed wrapper, non-hex payload.
        let junk = format!("{EMBED_OPEN_PREFIX}zzzz{EMBED_MARKER_SUFFIX}");
        assert!(decode_embedded_open(&junk).is_none());
    }

    #[test]
    fn decode_tolerates_leading_whitespace() {
        let encoded = encode_embedded_subtree(&styled_subtree()).unwrap();
        let open_line = encoded.lines().next().unwrap();
        let indented = format!("    {open_line}");
        assert!(decode_embedded_open(&indented).is_some());
    }
}
