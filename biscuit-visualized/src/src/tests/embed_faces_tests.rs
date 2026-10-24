//! The opt-in's face choice per requested family over injected font sets.
//!
//! Installed families are copies of the bundled Inter renamed to `Arial`
//! (an in-place, equal-length rename) with their `OS/2` `fsType` patched, so
//! results do not depend on the host's fonts.
//!
//! Each case checks that the measured face is the embedded face. A permitted
//! face must be the face the chain itself resolves (what the measurer uses
//! with no substitution); a substituted family must be the face a chain built
//! with `MERMAID_FONT=Inter` over the same installed fonts resolves, which is
//! how the substitution is defined.

use resvg::usvg::fontdb::ID;

use crate::fonts::bundled::{INTER_BOLD, INTER_REGULAR};
use crate::fonts::{FontChain, FontQuery};
use crate::mermaid::embed::faces::{ChoiceReason, EmbeddedFaces, FamilyChoice, choose_embedded_faces, forced_inter};
use crate::mermaid::embed::glyphs::{UsedGlyphs, WeightGlyphs};
use crate::mermaid::embed::permission::{Permission, Refusal};
use crate::tests::capture_warnings;

use super::font_chain_tests::{MERMAID_STACK, NOTO_EMOJI_FILE, renamed, table};

/// A copy of `font` whose `OS/2` `fsType` is `fs_type`.
fn with_fs_type(font: &[u8], fs_type: u16) -> Vec<u8> {
    let mut bytes = font.to_vec();
    let os2 = table(&bytes, b"OS/2");
    bytes[os2.start + 8..os2.start + 10].copy_from_slice(&fs_type.to_be_bytes());
    bytes
}

/// Installed `Arial` Regular and Bold with the given `fsType`s.
pub(super) fn arial(regular_fs_type: u16, bold_fs_type: u16) -> [Vec<u8>; 2] {
    [
        with_fs_type(&renamed(INTER_REGULAR, "Inter", "Arial"), regular_fs_type),
        with_fs_type(&renamed(INTER_BOLD, "Inter", "Arial"), bold_fs_type),
    ]
}

/// An installed Inter Regular and Bold (Editable), distinct files from the
/// bundled ones.
pub(super) fn installed_inter() -> [Vec<u8>; 2] {
    [with_fs_type(INTER_REGULAR, 0x0008), with_fs_type(INTER_BOLD, 0x0008)]
}

pub(super) fn chain(installed: &[Vec<u8>], mermaid_font: Option<&str>) -> FontChain {
    let installed: Vec<&[u8]> = installed.iter().map(Vec::as_slice).collect();
    FontChain::injected(&installed, &[], mermaid_font)
}

/// Which weights a diagram draws, in every family it uses.
#[derive(Clone, Copy)]
enum Drawn {
    Regular,
    Bold,
    Both,
}

/// The choices a render drawing `drawn` ends with for `requested`: chosen,
/// then chosen again with the families [`forced_inter`] names (the render's
/// second pass); and the warnings logged.
fn decide(requested: &[&str], chain: &FontChain, drawn: Drawn) -> (EmbeddedFaces, Vec<String>) {
    capture_warnings(|| {
        let first = choose_embedded_faces(requested, &[], chain);
        let (regular, bold) = match drawn {
            Drawn::Regular => ("a", ""),
            Drawn::Bold => ("", "a"),
            Drawn::Both => ("a", "a"),
        };
        let glyphs = UsedGlyphs {
            families: first
                .families
                .iter()
                .map(|choice| {
                    let weights = WeightGlyphs {
                        regular: regular.chars().collect(),
                        bold: bold.chars().collect(),
                    };
                    (choice.family.clone(), weights)
                })
                .collect(),
        };
        let forced = forced_inter(&first, &glyphs);
        choose_embedded_faces(requested, &forced, chain)
    })
}

/// The single choice for the Mermaid stack when the diagram draws both
/// weights.
fn decide_both(chain: &FontChain) -> FamilyChoice {
    only(decide(&[MERMAID_STACK], chain, Drawn::Both).0)
}

fn only(faces: EmbeddedFaces) -> FamilyChoice {
    assert_eq!(faces.families.len(), 1, "{faces:?}");
    faces.families.into_iter().next().expect("one family")
}

/// `(family, weight, bundled)` of face `id`, comparable across chains.
fn describe(chain: &FontChain, id: ID) -> (String, u16, bool) {
    let face = chain.database().face(id).expect("face");
    (face.families[0].0.clone(), face.weight.0, chain.is_bundled_inter(id))
}

fn query(weight: u16) -> FontQuery {
    FontQuery::from_css(MERMAID_STACK, weight, false)
}

/// The embedded faces are the faces `chain` resolves for the stack.
fn assert_measured_as_resolved(chain: &FontChain, choice: &FamilyChoice) {
    assert_eq!(Some(choice.regular.id), chain.resolve(&query(400)), "regular");
    assert_eq!(Some(choice.bold.id), chain.resolve(&query(700)), "bold");
}

/// The embedded faces are the bundled Inter, as a chain with
/// `MERMAID_FONT=Inter` over the same installed fonts resolves it.
fn assert_measured_as_inter(chain: &FontChain, installed: &[Vec<u8>], choice: &FamilyChoice) {
    let inter_chain = self::chain(installed, Some("Inter"));
    for (id, weight) in [(choice.regular.id, 400), (choice.bold.id, 700)] {
        let measured = inter_chain.resolve(&query(weight)).expect("Inter resolves");
        assert_eq!(describe(chain, id), describe(&inter_chain, measured), "weight {weight}");
        assert_eq!(describe(chain, id), ("Inter".to_string(), weight, true));
    }
    assert_eq!(choice.family, "Inter");
    assert!(choice.bundled_inter);
}

#[test]
fn permitted_resolved_faces_are_measured_and_embedded() {
    for (fs_type, permission) in [
        (0x0000, Permission::Installable),
        (0x0008, Permission::Editable),
        (0x0004, Permission::PreviewAndPrint),
        (0x000E, Permission::Editable),
    ] {
        let installed = arial(fs_type, fs_type);
        let chain = chain(&installed, None);
        let choice = only(choose_embedded_faces(&[MERMAID_STACK], &[], &chain));
        assert_eq!(choice.reason, ChoiceReason::Resolved, "fsType {fs_type:#06x}");
        assert_eq!(choice.family, "Arial");
        assert!(!choice.bundled_inter);
        assert_eq!((choice.regular.permission, choice.regular.fs_type), (Ok(permission), fs_type));
        assert_eq!((choice.bold.permission, choice.bold.fs_type), (Ok(permission), fs_type));
        assert!(!choice.regular.unsubset && !choice.bold.unsubset);
        assert_eq!(describe(&chain, choice.bold.id), ("Arial".to_string(), 700, false));
        assert_measured_as_resolved(&chain, &choice);
    }
}

#[test]
fn a_drawn_restricted_resolved_face_gives_inter_and_warns() {
    let installed = arial(0x0002, 0x0002);
    let chain = chain(&installed, None);
    let (faces, warnings) = decide(&[MERMAID_STACK], &chain, Drawn::Both);
    let choice = only(faces);
    assert_eq!(
        choice.reason,
        ChoiceReason::Refused {
            face: "Arial".to_string(),
            refusal: Refusal::Restricted { fs_type: 0x0002 }
        }
    );
    assert_measured_as_inter(&chain, &installed, &choice);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("may not be embedded"), "{warnings:?}");
}

/// Choosing alone keeps a refused face, recorded as refused: whether it
/// matters depends on the weights the diagram draws.
#[test]
fn before_layout_a_refused_face_is_kept_and_recorded() {
    let installed = arial(0x0000, 0x0002);
    let chain = chain(&installed, None);
    let (faces, warnings) = capture_warnings(|| choose_embedded_faces(&[MERMAID_STACK], &[], &chain));
    let choice = only(faces);
    assert_eq!(choice.reason, ChoiceReason::Resolved);
    assert_eq!(choice.regular.permission, Ok(Permission::Installable));
    assert_eq!(choice.bold.permission, Err(Refusal::Restricted { fs_type: 0x0002 }));
    assert_measured_as_resolved(&chain, &choice);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn a_drawn_restricted_weight_makes_the_whole_family_inter() {
    for (regular_fs_type, bold_fs_type, drawn) in [
        (0x0000, 0x0002, Drawn::Bold),
        (0x0000, 0x0002, Drawn::Both),
        (0x0002, 0x0000, Drawn::Regular),
        (0x0002, 0x0000, Drawn::Both),
    ] {
        let installed = arial(regular_fs_type, bold_fs_type);
        let chain = chain(&installed, None);
        let (faces, warnings) = decide(&[MERMAID_STACK], &chain, drawn);
        let choice = only(faces);
        assert_eq!(
            choice.reason,
            ChoiceReason::Refused {
                face: "Arial".to_string(),
                refusal: Refusal::Restricted { fs_type: 0x0002 }
            },
            "{regular_fs_type:#06x}/{bold_fs_type:#06x}"
        );
        assert_measured_as_inter(&chain, &installed, &choice);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }
}

#[test]
fn an_undrawn_restricted_weight_keeps_the_resolved_family() {
    for (regular_fs_type, bold_fs_type, drawn) in [(0x0000, 0x0002, Drawn::Regular), (0x0002, 0x0000, Drawn::Bold)] {
        let installed = arial(regular_fs_type, bold_fs_type);
        let chain = chain(&installed, None);
        let (faces, warnings) = decide(&[MERMAID_STACK], &chain, drawn);
        let choice = only(faces);
        assert_eq!(choice.reason, ChoiceReason::Resolved, "{regular_fs_type:#06x}/{bold_fs_type:#06x}");
        assert_eq!(choice.family, "Arial");
        assert_measured_as_resolved(&chain, &choice);
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}

/// An installed face named Inter is never chosen: it and the bundled Inter
/// would both be declared as `Inter` on one page.
#[test]
fn an_installed_inter_is_replaced_by_the_bundled_inter() {
    let installed = installed_inter();
    let chain = chain(&installed, None);
    let stack = "Inter, sans-serif";
    let resolved = chain.resolve(&FontQuery::from_css(stack, 400, false)).expect("resolves");
    assert!(!chain.is_bundled_inter(resolved), "precondition: the chain resolves the installed Inter");
    let choice = only(choose_embedded_faces(&[stack], &[], &chain));
    assert_eq!(choice.reason, ChoiceReason::Resolved);
    assert!(choice.bundled_inter);
    assert!(chain.is_bundled_inter(choice.regular.id) && chain.is_bundled_inter(choice.bold.id));
    assert_eq!((choice.regular.permission, choice.regular.fs_type), (Ok(Permission::Installable), 0));
}

#[test]
fn bitmap_only_gives_inter() {
    let installed = arial(0x0200, 0x0200);
    let chain = chain(&installed, None);
    let choice = decide_both(&chain);
    assert!(matches!(
        choice.reason,
        ChoiceReason::Refused { refusal: Refusal::BitmapOnly { fs_type: 0x0200 }, .. }
    ));
    assert_measured_as_inter(&chain, &installed, &choice);
}

#[test]
fn a_face_without_os2_gives_inter() {
    // Renaming the `OS/2` record to `OS/1` keeps the directory sorted.
    let without_os2 = |font: &[u8]| {
        let mut bytes = renamed(font, "Inter", "Arial");
        let at = bytes.windows(4).position(|w| w == b"OS/2").expect("an OS/2 record");
        bytes[at..at + 4].copy_from_slice(b"OS/1");
        bytes
    };
    let installed = [without_os2(INTER_REGULAR), without_os2(INTER_BOLD)];
    let chain = chain(&installed, None);
    let choice = decide_both(&chain);
    assert_eq!(
        choice.reason,
        ChoiceReason::Refused { face: "Arial".to_string(), refusal: Refusal::NoReadableOs2 }
    );
    assert_measured_as_inter(&chain, &installed, &choice);
}

#[test]
fn a_variable_resolved_face_gives_inter_as_outline_format() {
    let installed = [NOTO_EMOJI_FILE.to_vec()];
    let chain = chain(&installed, None);
    let choice = only(decide(&["'Noto Emoji'"], &chain, Drawn::Both).0);
    assert_eq!(
        choice.reason,
        ChoiceReason::Refused { face: "Noto Emoji".to_string(), refusal: Refusal::OutlineFormat }
    );
    assert!(choice.bundled_inter);
}

#[test]
fn no_subsetting_embeds_the_resolved_face_unsubset() {
    let installed = arial(0x0100, 0x0000);
    let chain = chain(&installed, None);
    let choice = only(choose_embedded_faces(&[MERMAID_STACK], &[], &chain));
    assert_eq!(choice.reason, ChoiceReason::Resolved);
    assert_eq!(choice.family, "Arial");
    assert!(choice.regular.unsubset);
    assert!(!choice.bold.unsubset);
    assert_measured_as_resolved(&chain, &choice);
}

#[test]
fn mermaid_font_inter_selects_inter() {
    let installed = arial(0x0000, 0x0000);
    let chain = chain(&installed, Some("Inter"));
    let choice = only(choose_embedded_faces(&[MERMAID_STACK], &[], &chain));
    assert_eq!(choice.reason, ChoiceReason::MermaidFontInter);
    assert_measured_as_inter(&chain, &installed, &choice);
    assert_measured_as_resolved(&chain, &choice);
}

#[test]
fn mermaid_font_naming_a_permitted_family_embeds_it() {
    let installed = arial(0x0008, 0x0008);
    let chain = chain(&installed, Some("Arial"));
    // The stack does not name Arial: only `MERMAID_FONT` reaches it.
    let choice = only(choose_embedded_faces(&["'Trebuchet MS', serif"], &[], &chain));
    assert_eq!(choice.reason, ChoiceReason::Resolved);
    assert_eq!(choice.family, "Arial");
    let regular = FontQuery::from_css("'Trebuchet MS', serif", 400, false);
    assert_eq!(Some(choice.regular.id), chain.resolve(&regular));
}

#[test]
fn mermaid_font_naming_a_restricted_family_gives_inter() {
    let installed = arial(0x0002, 0x0002);
    let chain = chain(&installed, Some("Arial"));
    let choice = decide_both(&chain);
    assert!(matches!(
        choice.reason,
        ChoiceReason::Refused { refusal: Refusal::Restricted { .. }, .. }
    ));
    assert_measured_as_inter(&chain, &installed, &choice);
}

#[test]
fn a_chain_with_no_installed_face_gives_inter() {
    let chain = chain(&[], None);
    let choice = only(choose_embedded_faces(&[MERMAID_STACK], &[], &chain));
    assert_eq!(choice.reason, ChoiceReason::NoInstalledFace);
    assert_eq!(chain.resolve(&query(400)), None, "nothing to measure with but Inter");
    assert_measured_as_inter(&chain, &[], &choice);
}

#[test]
fn a_stack_naming_inter_embeds_the_bundled_inter_as_resolved() {
    let chain = chain(&[], None);
    let choice = only(choose_embedded_faces(&["Inter, sans-serif"], &[], &chain));
    assert_eq!(choice.reason, ChoiceReason::Resolved);
    assert!(choice.bundled_inter);
    assert_eq!((choice.regular.permission, choice.regular.fs_type), (Ok(Permission::Installable), 0));
    let regular = FontQuery::from_css("Inter, sans-serif", 400, false);
    assert_eq!(Some(choice.regular.id), chain.resolve(&regular));
}

#[test]
fn each_requested_family_is_decided_on_its_own_and_duplicates_once() {
    let mut installed = arial(0x0002, 0x0002).to_vec();
    installed.extend([
        with_fs_type(&renamed(INTER_REGULAR, "Inter", "Verda"), 0x0000),
        with_fs_type(&renamed(INTER_BOLD, "Inter", "Verda"), 0x0000),
    ]);
    let chain = chain(&installed, None);
    let faces = decide(&["Arial", "Verda", "Arial"], &chain, Drawn::Both).0;
    let decided: Vec<(&str, &str)> = faces
        .families
        .iter()
        .map(|choice| (choice.requested.as_str(), choice.family.as_str()))
        .collect();
    assert_eq!(decided, [("Arial", "Inter"), ("Verda", "Verda")]);
}
