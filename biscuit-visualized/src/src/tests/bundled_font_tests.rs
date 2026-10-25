use ttf_parser::{Face, name_id};

use crate::fonts::bundled::{INTER_BOLD, INTER_REGULAR};

fn family_name(face: &Face<'_>) -> Option<String> {
    face.names()
        .into_iter()
        .filter(|name| name.name_id == name_id::FAMILY && name.is_unicode())
        .find_map(|name| name.to_string())
}

#[test]
fn inter_regular_is_upright_400_in_family_inter() {
    let face = Face::parse(INTER_REGULAR, 0).expect("Inter Regular parses");

    assert_eq!(family_name(&face).as_deref(), Some("Inter"));
    assert_eq!(face.weight().to_number(), 400);
    assert!(!face.is_italic());
    assert!(!face.is_variable());
}

#[test]
fn inter_bold_is_upright_700_in_family_inter() {
    let face = Face::parse(INTER_BOLD, 0).expect("Inter Bold parses");

    assert_eq!(family_name(&face).as_deref(), Some("Inter"));
    assert_eq!(face.weight().to_number(), 700);
    assert!(!face.is_italic());
    assert!(!face.is_variable());
}

#[cfg(target_os = "linux")]
#[test]
fn noto_emoji_maps_grinning_face_at_default_weight_400() {
    let face = Face::parse(crate::fonts::bundled::NOTO_EMOJI, 0).expect("Noto Emoji parses");

    let glyph = face.glyph_index('\u{1F600}').expect("U+1F600 is mapped");
    assert_ne!(glyph.0, 0);
    let weight_axis = face
        .variation_axes()
        .into_iter()
        .find(|axis| axis.tag == ttf_parser::Tag::from_bytes(b"wght"))
        .expect("wght axis");
    assert_eq!(weight_axis.def_value, 400.0);
}
