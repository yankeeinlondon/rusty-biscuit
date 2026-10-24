//! Which face the embedded-font opt-in measures with and embeds, per
//! requested family.

use resvg::usvg::fontdb::ID;

use super::permission::{EmbedPermission, Permission, Refusal, classify};
use crate::fonts::chain::ResolutionStep;
use crate::fonts::{FontChain, FontQuery};

/// The two weights the renderer can request. Both decide a family's
/// embeddability before layout, whichever the diagram ends up using, because
/// the weights a diagram uses are known only after it is laid out.
const WEIGHTS: [u16; 2] = [400, 700];

/// One face to measure with and embed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EmbeddedFace {
    pub(crate) id: ID,
    pub(crate) permission: Permission,
    /// The face's raw `OS/2` `fsType`, for the page comment.
    pub(crate) fs_type: u16,
    /// `fsType` forbids subsetting: the whole face is embedded.
    pub(crate) unsubset: bool,
}

/// Why a family is embedded as the face it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChoiceReason {
    /// The face the chain resolved for the family, which permits embedding
    /// (including the bundled Inter when the family's stack names `Inter`).
    Resolved,
    /// `MERMAID_FONT=Inter` selected the bundled Inter.
    MermaidFontInter,
    /// The bundled Inter, because the resolved face `face` (its family name)
    /// may not be embedded.
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

/// The face choice for every requested family of one render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddedFaces {
    pub(crate) families: Vec<FamilyChoice>,
}

/// Chooses, for each distinct CSS `font-family` list in `requested`, the
/// faces the opt-in measures with and embeds; they are the same faces.
///
/// A family keeps the faces `chain` resolves for it at regular and bold when
/// both permit embedding (see [`classify`]). Otherwise the whole family is
/// the bundled Inter, so one family never mixes two faces' metrics: when
/// either face is refused (logged as a warning), when the chain resolves no
/// face, and when `MERMAID_FONT=Inter` selects the bundled Inter.
pub(crate) fn choose_embedded_faces(requested: &[&str], chain: &FontChain) -> EmbeddedFaces {
    let mut families: Vec<FamilyChoice> = Vec::new();
    for css in requested {
        if !families.iter().any(|choice| choice.requested == *css) {
            families.push(choose_family(css, chain));
        }
    }
    EmbeddedFaces { families }
}

fn choose_family(css: &str, chain: &FontChain) -> FamilyChoice {
    let [regular, bold] = WEIGHTS.map(|weight| chain.resolve_with_step(&FontQuery::from_css(css, weight, false)));
    let (Some((regular, step)), Some((bold, _))) = (regular, bold) else {
        return inter_family(css, chain, ChoiceReason::NoInstalledFace);
    };
    if step == ResolutionStep::MermaidFont && chain.is_bundled_inter(regular) {
        return inter_family(css, chain, ChoiceReason::MermaidFontInter);
    }
    let faces = [regular, bold].map(|id| (id, classify_face(chain, id)));
    let refused = faces.iter().find_map(|(id, permission)| match permission {
        EmbedPermission::Refuse(refusal) => Some((*id, *refusal)),
        _ => None,
    });
    if let Some((id, refusal)) = refused {
        let face = family_name(chain, id);
        tracing::warn!(
            requested = css,
            face,
            ?refusal,
            "the resolved font may not be embedded; measuring and embedding the bundled Inter for this family"
        );
        return inter_family(css, chain, ChoiceReason::Refused { face, refusal });
    }
    let [(regular, regular_permission), (bold, bold_permission)] = faces;
    FamilyChoice {
        requested: css.to_string(),
        family: family_name(chain, regular),
        bundled_inter: chain.is_bundled_inter(regular),
        regular: embedded_face(regular, regular_permission),
        bold: embedded_face(bold, bold_permission),
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

/// `permission` of a face already known to be embeddable.
fn embedded_face(id: ID, permission: EmbedPermission) -> EmbeddedFace {
    match permission {
        EmbedPermission::Embed { permission, fs_type } => EmbeddedFace {
            id,
            permission,
            fs_type,
            unsubset: false,
        },
        EmbedPermission::EmbedUnsubset { permission, fs_type } => EmbeddedFace {
            id,
            permission,
            fs_type,
            unsubset: true,
        },
        EmbedPermission::Refuse(refusal) => {
            unreachable!("a refused face ({refusal:?}) is never embedded")
        }
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
