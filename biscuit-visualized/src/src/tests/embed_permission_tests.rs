//! The embedding classifier over font files built in the test, one `fsType`
//! or table shape per case, plus the bundled Inter.

use crate::fonts::bundled::{INTER_BOLD, INTER_REGULAR};
use crate::mermaid::embed::permission::{EmbedPermission, Permission, Refusal, classify};

use super::font_chain_tests::NOTO_EMOJI_FILE;

/// A face's tables, by tag.
pub(super) type Tables = Vec<(&'static [u8; 4], Vec<u8>)>;

/// An `OS/2` table (version 4 length, all zero) whose `fsType` is `fs_type`.
fn os2(fs_type: u16) -> Vec<u8> {
    let mut table = vec![0u8; 96];
    table[8..10].copy_from_slice(&fs_type.to_be_bytes());
    table
}

/// An sfnt holding `tables` (sorted by tag, as the format requires), placed
/// at byte `base` of the file it will sit in (table offsets are from the
/// file's start, also inside a collection).
fn sfnt_at(tables: &[(&[u8; 4], Vec<u8>)], base: usize) -> Vec<u8> {
    let mut tables: Vec<_> = tables.to_vec();
    tables.sort_by_key(|(tag, _)| **tag);
    let mut out = Vec::new();
    out.extend(0x0001_0000u32.to_be_bytes());
    out.extend((tables.len() as u16).to_be_bytes());
    out.extend([0u8; 6]);
    let mut offset = base + 12 + 16 * tables.len();
    let mut data = Vec::new();
    for (tag, bytes) in &tables {
        out.extend(*tag);
        out.extend(0u32.to_be_bytes());
        out.extend((offset as u32).to_be_bytes());
        out.extend((bytes.len() as u32).to_be_bytes());
        let padded = bytes.len().div_ceil(4) * 4;
        data.extend(bytes);
        data.resize(data.len() + padded - bytes.len(), 0);
        offset += padded;
    }
    out.extend(data);
    out
}

fn sfnt(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    sfnt_at(tables, 0)
}

/// A static TrueType face whose `fsType` is `fs_type`.
fn truetype(fs_type: u16) -> Vec<u8> {
    sfnt(&[(b"OS/2", os2(fs_type)), (b"glyf", vec![0; 4])])
}

/// A font collection of `faces` (each a table list).
pub(super) fn collection(faces: &[Tables]) -> Vec<u8> {
    let header_len = 12 + 4 * faces.len();
    let mut bodies = Vec::new();
    let mut offsets = Vec::new();
    for tables in faces {
        let base = header_len + bodies.len();
        offsets.push(base as u32);
        bodies.extend(sfnt_at(tables, base));
    }
    let mut out = b"ttcf".to_vec();
    out.extend(0x0001_0000u32.to_be_bytes());
    out.extend((faces.len() as u32).to_be_bytes());
    for offset in offsets {
        out.extend(offset.to_be_bytes());
    }
    out.extend(bodies);
    out
}

fn embed(permission: Permission, fs_type: u16) -> EmbedPermission {
    EmbedPermission::Embed { permission, fs_type }
}

// ------------------------------------------------------------- fsType rows

#[test]
fn installable_editable_and_preview_print_embed() {
    assert_eq!(classify(&truetype(0x0000), 0), embed(Permission::Installable, 0x0000));
    assert_eq!(classify(&truetype(0x0008), 0), embed(Permission::Editable, 0x0008));
    assert_eq!(classify(&truetype(0x0004), 0), embed(Permission::PreviewAndPrint, 0x0004));
}

#[test]
fn restricted_as_the_only_permission_bit_is_refused() {
    assert_eq!(
        classify(&truetype(0x0002), 0),
        EmbedPermission::Refuse(Refusal::Restricted { fs_type: 0x0002 })
    );
}

#[test]
fn several_permission_bits_take_the_least_restrictive() {
    assert_eq!(classify(&truetype(0x0006), 0), embed(Permission::PreviewAndPrint, 0x0006));
    assert_eq!(classify(&truetype(0x000A), 0), embed(Permission::Editable, 0x000A));
    assert_eq!(classify(&truetype(0x000E), 0), embed(Permission::Editable, 0x000E));
    assert_eq!(classify(&truetype(0x000C), 0), embed(Permission::Editable, 0x000C));
}

#[test]
fn bitmap_only_is_refused_whatever_the_permission_bits() {
    for fs_type in [0x0200, 0x0208, 0x0204, 0x0300] {
        assert_eq!(
            classify(&truetype(fs_type), 0),
            EmbedPermission::Refuse(Refusal::BitmapOnly { fs_type }),
            "fsType {fs_type:#06x}"
        );
    }
    // Restricted and bitmap-only together: refused either way.
    assert_eq!(
        classify(&truetype(0x0202), 0),
        EmbedPermission::Refuse(Refusal::Restricted { fs_type: 0x0202 })
    );
}

#[test]
fn no_subsetting_embeds_unsubset_unless_restricted() {
    assert_eq!(
        classify(&truetype(0x0100), 0),
        EmbedPermission::EmbedUnsubset { permission: Permission::Installable, fs_type: 0x0100 }
    );
    assert_eq!(
        classify(&truetype(0x0108), 0),
        EmbedPermission::EmbedUnsubset { permission: Permission::Editable, fs_type: 0x0108 }
    );
    assert_eq!(
        classify(&truetype(0x0102), 0),
        EmbedPermission::Refuse(Refusal::Restricted { fs_type: 0x0102 })
    );
}

#[test]
fn reserved_bits_do_not_change_the_decision() {
    for reserved in [0x0001, 0x00F0, 0xFC00, 0xFCF1] {
        assert_eq!(
            classify(&truetype(reserved), 0),
            embed(Permission::Installable, reserved),
            "reserved {reserved:#06x}"
        );
        assert_eq!(
            classify(&truetype(reserved | 0x0002), 0),
            EmbedPermission::Refuse(Refusal::Restricted { fs_type: reserved | 0x0002 }),
            "reserved {reserved:#06x} with Restricted"
        );
    }
}

// --------------------------------------------------------------- OS/2 rows

#[test]
fn a_face_without_os2_is_refused() {
    let font = sfnt(&[(b"glyf", vec![0; 4])]);
    assert_eq!(classify(&font, 0), EmbedPermission::Refuse(Refusal::NoReadableOs2));
}

#[test]
fn an_os2_table_too_short_for_fs_type_is_refused_and_one_just_long_enough_is_read() {
    let mut short = os2(0x0000);
    short.truncate(9);
    let font = sfnt(&[(b"OS/2", short), (b"glyf", vec![0; 4])]);
    assert_eq!(classify(&font, 0), EmbedPermission::Refuse(Refusal::NoReadableOs2));

    let mut exact = os2(0x0008);
    exact.truncate(10);
    let font = sfnt(&[(b"OS/2", exact), (b"glyf", vec![0; 4])]);
    assert_eq!(classify(&font, 0), embed(Permission::Editable, 0x0008));
}

#[test]
fn an_os2_record_pointing_past_the_end_of_the_file_is_refused() {
    let mut font = truetype(0x0000);
    // The first record is `OS/2`; its offset field is bytes 8..12 of it.
    assert_eq!(&font[12..16], b"OS/2");
    let past_the_end = font.len() as u32;
    font[20..24].copy_from_slice(&past_the_end.to_be_bytes());
    assert_eq!(classify(&font, 0), EmbedPermission::Refuse(Refusal::NoReadableOs2));
}

// ------------------------------------------------------------ outline rows

#[test]
fn cff_cff2_variable_and_glyf_less_faces_are_refused_as_outline_format() {
    let cases: [(&str, Tables); 4] = [
        ("CFF", vec![(b"OS/2", os2(0)), (b"CFF ", vec![0; 4])]),
        ("CFF2", vec![(b"OS/2", os2(0)), (b"CFF2", vec![0; 4])]),
        ("variable", vec![(b"OS/2", os2(0)), (b"glyf", vec![0; 4]), (b"fvar", vec![0; 4])]),
        ("no glyf", vec![(b"OS/2", os2(0))]),
    ];
    for (name, tables) in cases {
        assert_eq!(
            classify(&sfnt(&tables), 0),
            EmbedPermission::Refuse(Refusal::OutlineFormat),
            "{name}"
        );
    }
}

#[test]
fn the_real_variable_noto_emoji_is_refused_as_outline_format() {
    assert_eq!(classify(NOTO_EMOJI_FILE, 0), EmbedPermission::Refuse(Refusal::OutlineFormat));
}

// ---------------------------------------------------------- document shapes

#[test]
fn a_collection_face_is_classified_by_its_own_index() {
    let font = collection(&[
        vec![(b"OS/2", os2(0x0002)), (b"glyf", vec![0; 4])],
        vec![(b"OS/2", os2(0x0008)), (b"glyf", vec![0; 4])],
    ]);
    assert_eq!(
        classify(&font, 0),
        EmbedPermission::Refuse(Refusal::Restricted { fs_type: 0x0002 })
    );
    assert_eq!(classify(&font, 1), embed(Permission::Editable, 0x0008));
}

#[test]
fn a_face_index_out_of_range_is_refused() {
    let font = collection(&[vec![(b"OS/2", os2(0)), (b"glyf", vec![0; 4])]]);
    assert_eq!(classify(&font, 1), EmbedPermission::Refuse(Refusal::NoReadableOs2));
    assert_eq!(classify(&truetype(0), 1), EmbedPermission::Refuse(Refusal::NoReadableOs2));
}

#[test]
fn trailing_bytes_after_a_valid_face_do_not_change_the_decision() {
    let mut font = truetype(0x0004);
    font.extend(b"trailing garbage that is not part of any table");
    assert_eq!(classify(&font, 0), embed(Permission::PreviewAndPrint, 0x0004));
}

#[test]
fn data_that_is_not_a_font_is_refused() {
    for data in [&b""[..], b"not a font at all", &[0u8; 3]] {
        assert_eq!(classify(data, 0), EmbedPermission::Refuse(Refusal::NoReadableOs2));
    }
}

#[test]
fn the_bundled_inter_is_installable() {
    assert_eq!(classify(INTER_REGULAR, 0), embed(Permission::Installable, 0x0000));
    assert_eq!(classify(INTER_BOLD, 0), embed(Permission::Installable, 0x0000));
}
