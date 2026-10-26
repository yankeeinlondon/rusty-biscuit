//! The embedded face's file: an `allsorts` subset that keeps `cmap`, `OS/2`
//! and `name` (AC-E8 and AC-E12's table check), or the whole face, never a
//! collection.
//!
//! Subsets are read back with `ttf-parser`, independently of `allsorts`.

use std::collections::BTreeSet;

use ttf_parser::{Face, Tag};

use super::embed_permission_tests::{Tables, collection};
use crate::fonts::bundled::{INTER_BOLD, INTER_REGULAR};
use crate::mermaid::embed::subset::{SubsetError, subset, whole_face};

/// Characters of typical labels, ASCII and beyond.
const LABELS: &str = "Start → Stop, naïve café “quoted” — 42%";
/// Not mapped by Inter: an emoji and a supplementary private-use code point.
const UNMAPPED: [char; 2] = ['\u{1F600}', '\u{10FFFD}'];

fn code_points(text: &str) -> BTreeSet<char> {
    text.chars().collect()
}

/// The tags of `font`'s table directory.
fn table_tags(font: &[u8]) -> Vec<String> {
    let face = Face::parse(font, 0).expect("the file parses");
    let raw = face.raw_face();
    let count = u16::from_be_bytes([raw.data[4], raw.data[5]]) as usize;
    (0..count)
        .map(|i| String::from_utf8_lossy(&raw.data[12 + 16 * i..16 + 16 * i]).into_owned())
        .collect()
}

/// `font`'s tables, for building a collection around them.
pub(super) fn tables_of(font: &[u8]) -> Tables {
    table_tags(font)
        .into_iter()
        .map(|tag| {
            let tag: &'static [u8; 4] = Box::leak(Box::new(tag.as_bytes().try_into().expect("four bytes")));
            let data = Face::parse(font, 0)
                .expect("the file parses")
                .raw_face()
                .table(Tag::from_bytes(tag))
                .expect("the table reads")
                .to_vec();
            (tag, data)
        })
        .collect()
}

fn name(face: &Face<'_>, id: u16) -> Option<String> {
    face.names()
        .into_iter()
        .filter(|record| record.name_id == id)
        .find_map(|record| record.to_string())
}

fn fs_type(face: &Face<'_>) -> u16 {
    let os2 = face.raw_face().table(Tag::from_bytes(b"OS/2")).expect("OS/2");
    u16::from_be_bytes([os2[8], os2[9]])
}

#[test]
fn the_subset_keeps_cmap_os2_and_name() {
    for source in [INTER_REGULAR, INTER_BOLD] {
        let file = subset(source, 0, &code_points(LABELS)).expect("Inter subsets");
        let tags = table_tags(&file);
        for kept in ["cmap", "OS/2", "name", "glyf", "loca", "hmtx", "head", "hhea", "maxp", "post"] {
            assert!(tags.iter().any(|tag| tag == kept), "{kept} in {tags:?}");
        }
        // The subset draws plain advances: no layout tables travel with it.
        for dropped in ["GPOS", "GSUB", "kern", "fvar"] {
            assert!(!tags.iter().any(|tag| tag == dropped), "{dropped} in {tags:?}");
        }
        assert_eq!(&file[..4], &0x0001_0000u32.to_be_bytes(), "a TrueType file, not a collection");
        assert!(file.len() < source.len() / 10, "{} of {}", file.len(), source.len());
    }
}

#[test]
fn the_subset_carries_inters_copyright_and_license() {
    let file = subset(INTER_REGULAR, 0, &code_points("Ab")).expect("Inter subsets");
    let face = Face::parse(&file, 0).expect("the subset parses");
    let original = Face::parse(INTER_REGULAR, 0).expect("Inter parses");
    for id in [0, 13, 14] {
        let kept = name(&face, id);
        assert!(kept.is_some(), "name ID {id}");
        assert_eq!(kept, name(&original, id), "name ID {id}");
    }
    assert!(name(&face, 0).expect("copyright").contains("The Inter Project Authors"));
    assert!(name(&face, 13).expect("license").contains("Open Font License"));
    assert_eq!(fs_type(&face), fs_type(&original));
}

#[test]
fn every_mapped_code_point_is_in_the_subset_and_nothing_else() {
    let mut requested = code_points(LABELS);
    requested.extend(UNMAPPED);
    let file = subset(INTER_BOLD, 0, &requested).expect("Inter subsets");
    let face = Face::parse(&file, 0).expect("the subset parses");
    let original = Face::parse(INTER_BOLD, 0).expect("Inter parses");
    for c in LABELS.chars() {
        let glyph = face.glyph_index(c).unwrap_or_else(|| panic!("{c:?} is mapped"));
        // The same glyph: its advance is the original's.
        assert_eq!(
            face.glyph_hor_advance(glyph),
            original.glyph_hor_advance(original.glyph_index(c).expect("Inter maps it")),
            "{c:?}"
        );
    }
    for c in UNMAPPED {
        assert!(original.glyph_index(c).is_none(), "fixture: Inter does not map {c:?}");
        assert!(face.glyph_index(c).is_none(), "{c:?} is skipped");
    }
    for c in ['Z', 'j', '€'] {
        assert!(original.glyph_index(c).is_some(), "fixture: Inter maps {c:?}");
        assert!(face.glyph_index(c).is_none(), "{c:?} was not requested");
    }
}

#[test]
fn an_empty_or_wholly_unmapped_set_gives_a_notdef_only_subset() {
    for requested in [BTreeSet::new(), UNMAPPED.into_iter().collect()] {
        let file = subset(INTER_REGULAR, 0, &requested).expect("Inter subsets");
        let face = Face::parse(&file, 0).expect("the subset parses");
        assert_eq!(face.number_of_glyphs(), 1, "{requested:?}");
    }
}

#[test]
fn a_collection_face_is_subset_to_a_single_face_file() {
    let ttc = collection(&[tables_of(INTER_REGULAR), tables_of(INTER_BOLD)]);
    for (index, weight) in [(0, 400), (1, 700)] {
        let file = subset(&ttc, index, &code_points("Ab")).expect("the collection face subsets");
        assert_eq!(&file[..4], &0x0001_0000u32.to_be_bytes());
        let face = Face::parse(&file, 0).expect("the subset parses");
        assert_eq!(face.weight().to_number(), weight, "face {index}");
        assert!(face.glyph_index('A').is_some());
    }
}

#[test]
fn the_whole_face_is_the_file_itself_or_the_one_face_of_a_collection() {
    assert_eq!(whole_face(INTER_REGULAR, 0).expect("whole"), INTER_REGULAR);
    let ttc = collection(&[tables_of(INTER_REGULAR), tables_of(INTER_BOLD)]);
    let bold = whole_face(&ttc, 1).expect("the collection face rebuilds");
    assert_eq!(&bold[..4], &0x0001_0000u32.to_be_bytes(), "never a collection");
    let face = Face::parse(&bold, 0).expect("the rebuilt face parses");
    let original = Face::parse(INTER_BOLD, 0).expect("Inter parses");
    assert_eq!(face.number_of_glyphs(), original.number_of_glyphs());
    assert_eq!(face.weight().to_number(), 700);
    let mut tags = table_tags(&bold);
    let mut expected = table_tags(INTER_BOLD);
    tags.sort();
    expected.sort();
    assert_eq!(tags, expected, "every table is kept");
}

#[test]
fn unreadable_faces_are_errors() {
    let some = code_points("A");
    assert!(matches!(subset(b"not a font", 0, &some), Err(SubsetError::Parse(_))));
    assert!(matches!(subset(INTER_REGULAR, 1, &some), Err(SubsetError::Parse(_))));
    assert!(matches!(whole_face(b"not a font", 0), Err(SubsetError::Parse(_))));
    assert!(matches!(whole_face(INTER_REGULAR, 1), Err(SubsetError::Parse(_))));
    let ttc = collection(&[tables_of(INTER_REGULAR)]);
    assert!(matches!(whole_face(&ttc, 1), Err(SubsetError::Parse(_))));
    // A face whose cmap is missing cannot map any code point.
    let without_cmap: Tables = tables_of(INTER_REGULAR).into_iter().filter(|(tag, _)| *tag != b"cmap").collect();
    let file = collection(&[without_cmap]);
    assert!(matches!(subset(&file, 0, &some), Err(SubsetError::NoUnicodeCmap(_))), "{:?}", subset(&file, 0, &some));
}
