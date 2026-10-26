//! The embedded-font opt-in's public surface: what one rendered diagram
//! hands its page ([`DiagramFont`]), and the page's `@font-face` markup built
//! from every diagram on it ([`page_font`]).
//!
//! A diagram's cache entry keeps, beside its SVG, a [`FontRecord`]: the
//! families the render requested, those it chose as Inter because it draws a
//! refused weight, and the code points it draws. The subset is
//! never cached (it covers a whole page); a cache hit rebuilds the face
//! choice from the record and re-scans nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::embedding::{FontEmbedding, FontEmbeddingError, build_font_embedding};
use super::faces::{EmbeddedFaces, choose_embedded_faces};
use super::glyphs::{UsedGlyphs, WeightGlyphs};
use crate::fonts::FontChain;

/// What one diagram rendered with the embedded-font opt-in
/// ([`MermaidDiagram::with_embedded_font`](crate::mermaid::MermaidDiagram::with_embedded_font))
/// contributes to its page's embedded font: the faces its labels were
/// measured in and the characters it draws in each. Pass every diagram's
/// value on a page to [`page_font`] once.
#[derive(Clone)]
pub struct DiagramFont {
    pub(crate) faces: EmbeddedFaces,
    pub(crate) glyphs: UsedGlyphs,
    pub(crate) chain: Arc<FontChain>,
}

impl std::fmt::Debug for DiagramFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiagramFont")
            .field("faces", &self.faces)
            .field("glyphs", &self.glyphs)
            .finish_non_exhaustive()
    }
}

impl DiagramFont {
    /// The family names this diagram's labels lead their `font-family` with,
    /// which the page's `@font-face` rules declare; empty when it draws no
    /// text.
    pub fn families(&self) -> Vec<&str> {
        self.glyphs.families.keys().map(String::as_str).collect()
    }
}

/// The embedded font of one page: one `@font-face` rule per family and weight
/// its diagrams draw, Inter's license line before Inter's rules, and one HTML
/// comment per embedded family saying which face was embedded and why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageFont(FontEmbedding);

impl PageFont {
    /// The HTML comments, one per embedded family, which go immediately
    /// before the `<style>` element.
    pub fn comments(&self) -> &[String] {
        &self.0.comments
    }

    /// The `<style>` element's content.
    pub fn css(&self) -> &str {
        &self.0.css
    }

    /// The comments followed by the `<style>` element; empty when
    /// [`is_empty`](Self::is_empty).
    pub fn markup(&self) -> String {
        self.0.markup()
    }

    /// Whether the page embeds nothing (its diagrams draw no text).
    pub fn is_empty(&self) -> bool {
        self.0.rules.is_empty()
    }
}

/// The embedded font for a page holding `diagrams`.
///
/// Each family is embedded once per weight some diagram draws, as a subset
/// of that weight's face holding the union of those diagrams' characters at
/// that weight (the whole face when its `fsType` forbids subsetting). Every
/// diagram of one process measures over the same font chain; the first
/// diagram's chain reads the faces.
///
/// ## Errors
///
/// When any face cannot be read or subset, after logging a warning naming
/// the cause. The page's diagrams are then shown without an embedded font.
pub fn page_font<'a>(diagrams: impl IntoIterator<Item = &'a DiagramFont>) -> Result<PageFont, FontEmbeddingError> {
    let mut faces = EmbeddedFaces { families: Vec::new() };
    let mut used = Vec::new();
    let mut chain = None;
    for diagram in diagrams {
        chain.get_or_insert(&diagram.chain);
        // Two diagrams can choose differently for one requested family (one
        // draws a refused weight, one does not), so whole choices are kept.
        for choice in &diagram.faces.families {
            if !faces.families.contains(choice) {
                faces.families.push(choice.clone());
            }
        }
        used.push(diagram.glyphs.clone());
    }
    let Some(chain) = chain else {
        return Ok(PageFont::default());
    };
    build_font_embedding(&faces, &used, chain).map(PageFont)
}

/// What a diagram's cache entry keeps beside its SVG for the opt-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FontRecord {
    /// The CSS `font-family` lists the render chose faces for, in order.
    requested: Vec<String>,
    /// Those of `requested` the render chose as the bundled Inter because it
    /// draws a weight whose resolved face is refused.
    forced_inter: Vec<String>,
    /// Per leading family, the code points drawn at each weight, as sorted
    /// strings.
    families: BTreeMap<String, RecordedGlyphs>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RecordedGlyphs {
    regular: String,
    bold: String,
}

impl FontRecord {
    pub(crate) fn new(faces: &EmbeddedFaces, forced_inter: &[String], glyphs: &UsedGlyphs) -> Self {
        let text = |set: &BTreeSet<char>| set.iter().collect::<String>();
        FontRecord {
            requested: faces.families.iter().map(|choice| choice.requested.clone()).collect(),
            forced_inter: forced_inter.to_vec(),
            families: glyphs
                .families
                .iter()
                .map(|(family, glyphs)| {
                    let recorded = RecordedGlyphs {
                        regular: text(&glyphs.regular),
                        bold: text(&glyphs.bold),
                    };
                    (family.clone(), recorded)
                })
                .collect(),
        }
    }

    /// The diagram's font over `chain`: the recorded families' faces chosen
    /// again with the recorded families forced to Inter (the choice depends
    /// only on the chain, which the cache key covers, and on that list), and
    /// the recorded code points.
    pub(crate) fn into_diagram_font(self, chain: &Arc<FontChain>) -> DiagramFont {
        let requested: Vec<&str> = self.requested.iter().map(String::as_str).collect();
        let glyphs = UsedGlyphs {
            families: self
                .families
                .into_iter()
                .map(|(family, glyphs)| {
                    let weights = WeightGlyphs {
                        regular: glyphs.regular.chars().collect(),
                        bold: glyphs.bold.chars().collect(),
                    };
                    (family, weights)
                })
                .collect(),
        };
        DiagramFont {
            faces: choose_embedded_faces(&requested, &self.forced_inter, chain),
            glyphs,
            chain: Arc::clone(chain),
        }
    }
}
