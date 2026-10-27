//! Which face the embedded-font opt-in measures with and embeds, per
//! requested family.

use std::sync::{Arc, Mutex};

use resvg::usvg::fontdb::ID;

use super::glyphs::UsedGlyphs;
use super::permission::{EmbedPermission, Permission, Refusal, classify};
use crate::fonts::chain::ResolutionStep;
use crate::fonts::{FontChain, FontQuery};

/// The two weights the renderer can request. Both are resolved and
/// classified before layout; whether a refused one matters is known only
/// after layout, from the weights the diagram draws ([`forced_inter`]).
const WEIGHTS: [u16; 2] = [400, 700];

/// One face to measure with and, unless it is refused, embed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EmbeddedFace {
    pub(crate) id: ID,
    /// What the face's `fsType` grants, or why it may not be embedded. A
    /// refused face is measured in a first layout, which is kept only when
    /// the diagram does not draw its weight, and is never embedded.
    pub(crate) permission: Result<Permission, Refusal>,
    /// The face's raw `OS/2` `fsType`, for the page comment (0 when refused
    /// for a reason that carries none).
    pub(crate) fs_type: u16,
    /// `fsType` forbids subsetting: the whole face is embedded.
    pub(crate) unsubset: bool,
}

/// Why a family is embedded as the face it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChoiceReason {
    /// The faces the chain resolved for the family; every weight the
    /// diagram draws permits embedding. A family the chain resolves to a face
    /// named `Inter` is the bundled Inter, so one page never declares two
    /// different faces as `Inter`.
    Resolved,
    /// `MERMAID_FONT=Inter` selected the bundled Inter.
    MermaidFontInter,
    /// The bundled Inter, because the resolved face `face` (its family name)
    /// may not be embedded and the diagram draws its weight.
    Refused { face: String, refusal: Refusal },
    /// The bundled Inter, because the chain resolves no face for the family.
    NoInstalledFace,
}

/// The face choice for one requested family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FamilyChoice {
    /// The CSS `font-family` list the diagram asks for.
    pub(crate) requested: String,
    /// The embedded face's family name, which labels lead with and the
    /// `@font-face` rule declares (`Inter` for the bundled Inter).
    pub(crate) family: String,
    pub(crate) bundled_inter: bool,
    pub(crate) regular: EmbeddedFace,
    pub(crate) bold: EmbeddedFace,
    pub(crate) reason: ChoiceReason,
}

impl FamilyChoice {
    /// The face text of CSS weight `weight` is measured and drawn in: bold
    /// from 600 up, as a browser matches a 600 request against the page's
    /// 400 and 700 rules, and as the code-point collector splits weights.
    pub(crate) fn face_for_weight(&self, weight: u16) -> EmbeddedFace {
        if weight >= 600 { self.bold } else { self.regular }
    }
}

/// The face choice for every requested family of one render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddedFaces {
    pub(crate) families: Vec<FamilyChoice>,
}

/// Chooses, for each distinct CSS `font-family` list in `requested`, the
/// faces the opt-in measures with and embeds; they are the same faces.
///
/// A family keeps the faces `chain` resolves for it at regular and bold,
/// recording a face that may not be embedded (see [`classify`]) as refused.
/// The whole family is the bundled Inter, so one family never mixes two
/// faces' metrics, when it is in `forced` (a refused weight is drawn, see
/// [`forced_inter`]), when the chain resolves no face, when
/// `MERMAID_FONT=Inter` selects the bundled Inter, and when the resolved
/// face is named `Inter` (an installed copy is never embedded under the name
/// the bundled Inter is declared as).
pub(crate) fn choose_embedded_faces(requested: &[&str], forced: &[String], chain: &FontChain) -> EmbeddedFaces {
    let mut families: Vec<FamilyChoice> = Vec::new();
    for css in requested {
        if !families.iter().any(|choice| choice.requested == *css) {
            families.push(choose_family(css, forced, chain));
        }
    }
    EmbeddedFaces { families }
}

/// The requested families of `faces` that must be measured and embedded as
/// the bundled Inter because the diagram draws, per `glyphs`, a weight whose
/// resolved face is refused; each is logged as a warning.
///
/// A render whose list is not empty is laid out again with that list forced,
/// which needs no third pass: the bundled Inter is always embeddable.
pub(crate) fn forced_inter(faces: &EmbeddedFaces, glyphs: &UsedGlyphs) -> Vec<String> {
    let mut forced = Vec::new();
    for choice in &faces.families {
        let (mut regular, mut bold) = (false, false);
        for drawn in glyphs.for_family(&choice.family) {
            regular |= !drawn.regular.is_empty();
            bold |= !drawn.bold.is_empty();
        }
        let refused = [(regular, &choice.regular), (bold, &choice.bold)]
            .into_iter()
            .find_map(|(drawn, face)| face.permission.err().filter(|_| drawn));
        if let Some(refusal) = refused {
            tracing::warn!(
                requested = choice.requested,
                family = choice.family,
                ?refusal,
                "the resolved font may not be embedded; measuring and embedding the bundled Inter for this family"
            );
            forced.push(choice.requested.clone());
        }
    }
    forced
}

/// The face choices of one opt-in render, made the first time the render asks
/// for each requested family, before any of that family's labels is
/// measured, and kept, so every label of the family gets the same faces.
pub(crate) struct FaceChooser {
    chain: Arc<FontChain>,
    /// The requested families to choose as the bundled Inter.
    forced: Vec<String>,
    families: Mutex<Vec<FamilyChoice>>,
}

impl FaceChooser {
    pub(crate) fn new(chain: &Arc<FontChain>, forced: &[String]) -> Self {
        FaceChooser {
            chain: Arc::clone(chain),
            forced: forced.to_vec(),
            families: Mutex::default(),
        }
    }

    /// The choice for the CSS `font-family` list `css`, made now if this is
    /// the first time it is asked for.
    pub(crate) fn choice(&self, css: &str) -> FamilyChoice {
        let mut families = self.families.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(choice) = families.iter().find(|choice| choice.requested == css) {
            return choice.clone();
        }
        let choice = choose_family(css, &self.forced, &self.chain);
        families.push(choice.clone());
        choice
    }

    /// Every choice made so far, in the order the families were first asked
    /// for.
    pub(crate) fn faces(&self) -> EmbeddedFaces {
        EmbeddedFaces {
            families: self.families.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone(),
        }
    }
}

fn choose_family(css: &str, forced: &[String], chain: &FontChain) -> FamilyChoice {
    let [regular, bold] = WEIGHTS.map(|weight| chain.resolve_with_step(&FontQuery::from_css(css, weight, false)));
    let (Some((regular, step)), Some((bold, _))) = (regular, bold) else {
        return inter_family(css, chain, ChoiceReason::NoInstalledFace);
    };
    let family = family_name(chain, regular);
    // An installed face named Inter is replaced by the bundled one: both
    // would be declared as `Inter`, and another family of the page may fall
    // back to the bundled Inter.
    let named_inter = chain.is_bundled_inter(regular) || family.eq_ignore_ascii_case("Inter");
    if named_inter && step == ResolutionStep::MermaidFont {
        return inter_family(css, chain, ChoiceReason::MermaidFontInter);
    }
    if named_inter {
        return inter_family(css, chain, ChoiceReason::Resolved);
    }
    let [regular, bold] = [regular, bold].map(|id| embedded_face(id, classify_face(chain, id)));
    if forced.iter().any(|name| name == css) {
        // Forced because a drawn weight is refused; regular is named first,
        // as either refusal is true of the family.
        if let Some((id, refusal)) = [regular, bold].iter().find_map(|face| face.permission.err().map(|refusal| (face.id, refusal))) {
            let face = family_name(chain, id);
            return inter_family(css, chain, ChoiceReason::Refused { face, refusal });
        }
    }
    FamilyChoice {
        requested: css.to_string(),
        family,
        bundled_inter: chain.is_bundled_inter(regular.id),
        regular,
        bold,
        reason: ChoiceReason::Resolved,
    }
}

/// The bundled Inter for the whole family `css`.
fn inter_family(css: &str, chain: &FontChain, reason: ChoiceReason) -> FamilyChoice {
    let [regular, bold] = WEIGHTS.map(|weight| {
        let id = chain
            .bundled_inter(&FontQuery::from_css(css, weight, false))
            .expect("every chain registers the bundled Inter");
        embedded_face(id, classify_face(chain, id))
    });
    FamilyChoice {
        requested: css.to_string(),
        family: family_name(chain, regular.id),
        bundled_inter: true,
        regular,
        bold,
        reason,
    }
}

fn classify_face(chain: &FontChain, id: ID) -> EmbedPermission {
    chain
        .database()
        .with_face_data(id, classify)
        .unwrap_or(EmbedPermission::Refuse(Refusal::NoReadableOs2))
}

fn embedded_face(id: ID, permission: EmbedPermission) -> EmbeddedFace {
    let (permission, fs_type, unsubset) = match permission {
        EmbedPermission::Embed { permission, fs_type } => (Ok(permission), fs_type, false),
        EmbedPermission::EmbedUnsubset { permission, fs_type } => (Ok(permission), fs_type, true),
        EmbedPermission::Refuse(refusal) => {
            let fs_type = match refusal {
                Refusal::Restricted { fs_type } | Refusal::BitmapOnly { fs_type } => fs_type,
                Refusal::NoReadableOs2 | Refusal::OutlineFormat => 0,
            };
            (Err(refusal), fs_type, false)
        }
    };
    EmbeddedFace {
        id,
        permission,
        fs_type,
        unsubset,
    }
}

/// The face's first family name, as the font database records it.
fn family_name(chain: &FontChain, id: ID) -> String {
    chain
        .database()
        .face(id)
        .and_then(|face| face.families.first())
        .map_or_else(String::new, |(name, _)| name.clone())
}
