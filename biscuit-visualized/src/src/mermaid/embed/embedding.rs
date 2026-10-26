//! The page's embedded font: one `@font-face` rule per embedded family and
//! weight the page's diagrams draw, the comments that say which face and
//! why, and Inter's OFL attribution.
//!
//! Nothing here is cached: the rules cover the union of every diagram on a
//! page, which no single diagram's cache entry knows, so they are rebuilt
//! from each diagram's [`UsedGlyphs`] per page.

use std::collections::BTreeSet;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use resvg::usvg::fontdb::ID;

use super::comment::{ofl_attribution, page_comment};
use super::faces::{ChoiceReason, EmbeddedFace, EmbeddedFaces, FamilyChoice};
use super::glyphs::UsedGlyphs;
use super::subset::{SubsetError, subset, whole_face};
use crate::fonts::FontChain;
use crate::mermaid::postprocess::css_quoted;

/// Makes the file embedded for one face: from its data and index, and the
/// code points to keep, or `None` for the whole face.
type MakeFile<'a> = dyn Fn(&[u8], u32, Option<&BTreeSet<char>>) -> Result<Vec<u8>, SubsetError> + 'a;

/// The size above which an embedded face's file is logged: the TrueType
/// bytes of one face and weight, before base64. A larger file is still
/// embedded.
pub(crate) const BUDGET_BYTES: usize = 60_000;

/// One embedded family and weight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddedRule {
    /// The family the rule declares, which the labels lead with.
    pub(crate) family: String,
    pub(crate) weight: u16,
    pub(crate) id: ID,
    /// The TrueType file's size, before base64.
    pub(crate) bytes: usize,
    /// The whole face, because its `fsType` forbids subsetting.
    pub(crate) unsubset: bool,
}

/// What a page carries for its embedded font.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FontEmbedding {
    /// One HTML comment per embedded family, which go immediately before the
    /// `<style>` element.
    pub(crate) comments: Vec<String>,
    /// The `<style>` element's content: the `@font-face` rules, Inter's
    /// preceded by its OFL attribution.
    pub(crate) css: String,
    pub(crate) rules: Vec<EmbeddedRule>,
}

impl FontEmbedding {
    /// The comments followed by the `<style>` element; empty when the page
    /// embeds nothing (its diagrams draw no text).
    pub(crate) fn markup(&self) -> String {
        if self.rules.is_empty() {
            return String::new();
        }
        format!("{}\n<style>\n{}</style>\n", self.comments.join("\n"), self.css)
    }
}

/// The page cannot carry an embedded font; its diagrams are emitted without
/// one.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FontEmbeddingError {
    #[error("the font data of \"{family}\" weight {weight} could not be read")]
    FaceData { family: String, weight: u16 },
    #[error("\"{family}\" weight {weight} could not be subset: {source}")]
    Subset {
        family: String,
        weight: u16,
        source: SubsetError,
    },
}

/// Builds the embedded font for a page whose diagrams chose `faces` and draw
/// `used` (one entry per diagram).
///
/// Each family gets one rule per weight that some diagram draws in it (a
/// weight no label uses is not embedded, and a face that may not be embedded
/// never is), whichever requested families chose it, as a subset of that weight's face holding the union of those
/// diagrams' code points at that weight, or the whole face when its `fsType`
/// forbids subsetting. A face serving both weights is embedded under each, as
/// a separate file. A file over [`BUDGET_BYTES`] is still embedded, and
/// logged.
///
/// ## Errors
///
/// Any face that cannot be read or subset fails the whole page, with a
/// warning naming the cause: the diagrams are then emitted as static SVG
/// without an embedded font, rather than with a font that covers only some
/// of them.
pub(crate) fn build_font_embedding(
    faces: &EmbeddedFaces,
    used: &[UsedGlyphs],
    chain: &FontChain,
) -> Result<FontEmbedding, FontEmbeddingError> {
    build_font_embedding_with(faces, used, chain, |data, index, code_points| match code_points {
        Some(code_points) => subset(data, index, code_points),
        None => whole_face(data, index),
    })
}

/// [`build_font_embedding`] with the file maker injected: it receives a
/// face's data and index, and the code points to keep, or `None` for the
/// whole face.
pub(crate) fn build_font_embedding_with(
    faces: &EmbeddedFaces,
    used: &[UsedGlyphs],
    chain: &FontChain,
    make_file: impl Fn(&[u8], u32, Option<&BTreeSet<char>>) -> Result<Vec<u8>, SubsetError>,
) -> Result<FontEmbedding, FontEmbeddingError> {
    let result = build(faces, used, chain, &make_file);
    if let Err(error) = &result {
        tracing::warn!(%error, "the Mermaid font cannot be embedded; the page's diagrams carry no embedded font");
    }
    result
}

fn build(
    faces: &EmbeddedFaces,
    used: &[UsedGlyphs],
    chain: &FontChain,
    make_file: &MakeFile<'_>,
) -> Result<FontEmbedding, FontEmbeddingError> {
    let mut embedding = FontEmbedding::default();
    // A rule is the family and weight it declares, and the face its file is
    // cut from: one face can serve both weights of a family, with a
    // different subset under each.
    let mut embedded: Vec<(&str, u16, ID)> = Vec::new();
    let mut commented: Vec<Commented<'_>> = Vec::new();
    for choice in &faces.families {
        let (mut regular, mut bold) = (BTreeSet::new(), BTreeSet::new());
        for glyphs in used.iter().flat_map(|diagram| diagram.for_family(&choice.family)) {
            regular.extend(&glyphs.regular);
            bold.extend(&glyphs.bold);
        }
        let mut css = String::new();
        let mut unsubset = None;
        for (face, weight, code_points) in [(&choice.regular, 400, regular), (&choice.bold, 700, bold)] {
            let seen = embedded.iter().any(|&(family, seen_weight, id)| {
                family.eq_ignore_ascii_case(&choice.family) && (seen_weight, id) == (weight, face.id)
            });
            if code_points.is_empty() || seen {
                continue;
            }
            // A render that draws a refused weight chooses the family as Inter
            // instead (`faces::forced_inter`), so no glyphs reach this branch.
            if let Err(refusal) = face.permission {
                debug_assert!(false, "a refused face ({refusal:?}) has glyphs to embed");
                tracing::warn!(family = choice.family, weight, ?refusal, "not embedding a face that may not be embedded");
                continue;
            }
            embedded.push((&choice.family, weight, face.id));
            let file = face_file(choice, face, weight, &code_points, chain, make_file)?;
            css.push_str(&font_face_rule(&choice.family, weight, &file));
            if face.unsubset {
                unsubset.get_or_insert(face);
            }
            embedding.rules.push(EmbeddedRule {
                family: choice.family.clone(),
                weight,
                id: face.id,
                bytes: file.len(),
                unsubset: face.unsubset,
            });
        }
        if !css.is_empty() {
            if is_inter(choice) {
                embedding.css.push_str(&ofl_attribution());
                embedding.css.push('\n');
            }
            embedding.css.push_str(&css);
        }
        // One comment per embedded family. Several requested families can
        // share one (a generic that resolves nothing and a refused face both
        // become Inter); the comment gives the reason that most explains the
        // substitution, whichever choice embedded the rules.
        if let Some(slot) = commented.iter_mut().find(|slot| slot.choice.family.eq_ignore_ascii_case(&choice.family)) {
            if reason_rank(&choice.reason) > reason_rank(&slot.choice.reason) {
                slot.choice = choice;
            }
            slot.unsubset = slot.unsubset.or(unsubset.copied());
            embedding.comments[slot.index] = page_comment(slot.choice, slot.unsubset.as_ref());
        } else if !css.is_empty() {
            embedding.comments.push(page_comment(choice, unsubset));
            commented.push(Commented {
                choice,
                index: embedding.comments.len() - 1,
                unsubset: unsubset.copied(),
            });
        }
    }
    Ok(embedding)
}

/// The comment of one embedded family: the choice whose reason it gives, its
/// index among the page's comments, and the family's face embedded unsubset.
struct Commented<'a> {
    choice: &'a FamilyChoice,
    index: usize,
    unsubset: Option<EmbeddedFace>,
}

/// The TrueType file embedded for `face` at `weight`, logged when it is over
/// budget or whole.
fn face_file(
    choice: &FamilyChoice,
    face: &EmbeddedFace,
    weight: u16,
    code_points: &BTreeSet<char>,
    chain: &FontChain,
    make_file: &MakeFile<'_>,
) -> Result<Vec<u8>, FontEmbeddingError> {
    let family = &choice.family;
    let file = chain
        .database()
        .with_face_data(face.id, |data, index| {
            make_file(data, index, (!face.unsubset).then_some(code_points))
        })
        .ok_or_else(|| FontEmbeddingError::FaceData {
            family: family.clone(),
            weight,
        })?
        .map_err(|source| FontEmbeddingError::Subset {
            family: family.clone(),
            weight,
            source,
        })?;
    let bytes = file.len();
    if face.unsubset {
        tracing::info!(
            family,
            weight,
            bytes,
            "embedding \"{family}\" weight {weight} unsubset ({bytes} bytes): its fsType forbids subsetting"
        );
    }
    if bytes > BUDGET_BYTES {
        tracing::info!(
            family,
            weight,
            bytes,
            "the embedded font \"{family}\" weight {weight} is {bytes} bytes, over the {BUDGET_BYTES}-byte budget; embedding it anyway"
        );
    }
    Ok(file)
}

fn font_face_rule(family: &str, weight: u16, file: &[u8]) -> String {
    format!(
        "@font-face{{font-family:{};font-style:normal;font-weight:{weight};src:url(data:font/ttf;base64,{}) format(\"truetype\")}}\n",
        css_quoted(family),
        BASE64.encode(file)
    )
}

/// How much `reason` explains which face a family is embedded as: a refusal
/// most, the chain reaching no face least.
fn reason_rank(reason: &ChoiceReason) -> u8 {
    match reason {
        ChoiceReason::NoInstalledFace => 0,
        ChoiceReason::Resolved => 1,
        ChoiceReason::MermaidFontInter => 2,
        ChoiceReason::Refused { .. } => 3,
    }
}

/// Whether `choice` embeds Inter, whose license notice the page repeats.
fn is_inter(choice: &FamilyChoice) -> bool {
    choice.bundled_inter || choice.family.eq_ignore_ascii_case("Inter")
}
