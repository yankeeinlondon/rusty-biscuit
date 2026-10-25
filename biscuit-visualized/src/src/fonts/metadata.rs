//! Face metadata read from font tables, and the installed sans-serif rule
//! (the chain's last step).

use std::cmp::Ordering;
use std::ops::RangeInclusive;

/// The Unicode blocks whose names contain "Latin", for the Latin-coverage
/// preference of [`choose_sans_serif_family`]:
///
/// | Block | Range |
/// | ----- | ----- |
/// | Basic Latin | U+0000–U+007F |
/// | Latin-1 Supplement | U+0080–U+00FF |
/// | Latin Extended-A | U+0100–U+017F |
/// | Latin Extended-B | U+0180–U+024F |
/// | Latin Extended Additional | U+1E00–U+1EFF |
/// | Latin Extended-C | U+2C60–U+2C7F |
/// | Latin Extended-D | U+A720–U+A7FF |
/// | Latin Extended-E | U+AB30–U+AB6F |
/// | Latin Extended-F | U+10780–U+107BF |
/// | Latin Extended-G | U+1DF00–U+1DFFF |
pub(crate) const LATIN_BLOCKS: [RangeInclusive<u32>; 10] = [
    0x0000..=0x007F,
    0x0080..=0x00FF,
    0x0100..=0x017F,
    0x0180..=0x024F,
    0x1E00..=0x1EFF,
    0x2C60..=0x2C7F,
    0xA720..=0xA7FF,
    0xAB30..=0xAB6F,
    0x10780..=0x107BF,
    0x1DF00..=0x1DFFF,
];

/// The `sFamilyClass` class (high byte) the OS/2 table uses for sans-serif.
pub(crate) const SANS_SERIF_CLASS: u8 = 8;

/// The class byte (high byte) of the face's OS/2 `sFamilyClass`; `None`
/// without an OS/2 table.
pub(crate) fn family_class(data: &[u8], index: u32) -> Option<u8> {
    ttf_parser::RawFace::parse(data, index)
        .ok()?
        .table(ttf_parser::Tag::from_bytes(b"OS/2"))?
        .get(30)
        .copied()
}

/// What the installed sans-serif rule needs to know about one face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FaceMetadata {
    /// The face's first family name, as fontdb reports it.
    pub(crate) family: String,
    /// The class byte of OS/2 `sFamilyClass`; `None` without an OS/2 table.
    pub(crate) family_class: Option<u8>,
    pub(crate) weight: u16,
    pub(crate) italic: bool,
    /// Code points in [`LATIN_BLOCKS`] that the `cmap` maps to a glyph.
    pub(crate) latin_coverage: u32,
    /// The `post` table's `isFixedPitch`.
    pub(crate) fixed_pitch: bool,
}

impl FaceMetadata {
    /// Reads the metadata of face `index` in `data`; `family` is the name
    /// fontdb registered it under. `None` if the face does not parse.
    pub(crate) fn read(data: &[u8], index: u32, family: &str) -> Option<Self> {
        let face = ttf_parser::Face::parse(data, index).ok()?;
        let latin_coverage = LATIN_BLOCKS
            .iter()
            .flat_map(|block| block.clone())
            .filter_map(char::from_u32)
            .filter(|&c| face.glyph_index(c).is_some())
            .count() as u32;
        Some(FaceMetadata {
            family: family.to_string(),
            family_class: family_class(data, index),
            weight: face.weight().to_number(),
            italic: face.is_italic(),
            latin_coverage,
            fixed_pitch: face.is_monospaced(),
        })
    }

    fn is_sans_serif(&self) -> bool {
        self.family_class == Some(SANS_SERIF_CLASS)
    }

    fn is_upright(&self, weight: u16) -> bool {
        !self.italic && self.weight == weight
    }
}

/// One family's standing under the installed sans-serif rule.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FamilyStanding {
    folded: String,
    regular_and_bold: bool,
    latin_coverage: u32,
    proportional: bool,
}

/// Chooses the family for the chain's last step among `faces` (installed
/// faces only; the caller leaves the bundled ones out), returning its
/// case-folded name.
///
/// Only faces whose own OS/2 class is sans-serif count. Their families are
/// grouped by case-folded name and ranked by, in order:
///
/// 1. having both a regular (400, upright) and a bold (700, upright) face;
/// 2. the widest Latin coverage of any of its faces;
/// 3. being proportional (some face not `isFixedPitch`) over monospace;
/// 4. the alphabetically first case-folded family name.
///
/// No font names are preferred by name; the order is total, so the answer
/// does not depend on the order of `faces`.
pub(crate) fn choose_sans_serif_family(faces: &[FaceMetadata]) -> Option<String> {
    let mut standings: Vec<FamilyStanding> = Vec::new();
    for face in faces.iter().filter(|face| face.is_sans_serif()) {
        let folded = fold(&face.family);
        let standing = match standings.iter_mut().find(|s| s.folded == folded) {
            Some(standing) => standing,
            None => {
                standings.push(FamilyStanding {
                    folded,
                    regular_and_bold: false,
                    latin_coverage: 0,
                    proportional: false,
                });
                standings.last_mut().expect("just pushed")
            }
        };
        standing.latin_coverage = standing.latin_coverage.max(face.latin_coverage);
        standing.proportional |= !face.fixed_pitch;
    }
    for standing in &mut standings {
        let family: Vec<&FaceMetadata> = faces
            .iter()
            .filter(|face| face.is_sans_serif() && fold(&face.family) == standing.folded)
            .collect();
        standing.regular_and_bold = family.iter().any(|face| face.is_upright(400))
            && family.iter().any(|face| face.is_upright(700));
    }
    standings
        .into_iter()
        .min_by(rank)
        .map(|standing| standing.folded)
}

/// `Less` when `a` is preferred.
fn rank(a: &FamilyStanding, b: &FamilyStanding) -> Ordering {
    b.regular_and_bold
        .cmp(&a.regular_and_bold)
        .then(b.latin_coverage.cmp(&a.latin_coverage))
        .then(b.proportional.cmp(&a.proportional))
        .then_with(|| a.folded.cmp(&b.folded))
}

/// The case-folded form the chain compares family names in.
pub(crate) fn fold(name: &str) -> String {
    name.to_lowercase()
}
