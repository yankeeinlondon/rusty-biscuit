//! The embedded face's TrueType file: an `allsorts` subset of the face, or
//! the whole face when its `fsType` forbids subsetting.

use std::collections::BTreeSet;

use allsorts::binary::read::ReadScope;
use allsorts::font_data::{DynamicFontTableProvider, FontData};
use allsorts::subset::{CmapTarget, SubsetProfile, whole_font};
use allsorts::tables::FontTableProvider;
use allsorts::tables::cmap::{Cmap, CmapSubtable};
use allsorts::tag;

/// Why a face could not be turned into an embeddable file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SubsetError {
    #[error("the face does not parse: {0}")]
    Parse(String),
    #[error("the face has no Unicode cmap: {0}")]
    NoUnicodeCmap(String),
    #[error("allsorts could not subset the face: {0}")]
    Subset(String),
}

/// A TrueType subset of face `index` of `data` holding the glyphs that the
/// face's Unicode `cmap` maps `code_points` to, plus `.notdef`.
///
/// Code points the face does not map are skipped: they are drawn by the
/// reader's fallback font, never taken from another face. The subset keeps
/// `cmap`, `OS/2`, and `name` (with its copyright and license records) along
/// with the outline and metric tables; `allsorts` writes no `GPOS`, `GSUB`,
/// or `kern`, so the subset draws plain glyph advances.
pub(crate) fn subset(data: &[u8], index: u32, code_points: &BTreeSet<char>) -> Result<Vec<u8>, SubsetError> {
    with_provider(data, index, |provider| {
        let glyph_ids = glyph_ids(provider, code_points)?;
        allsorts::subset::subset(provider, &glyph_ids, &SubsetProfile::Minimal, CmapTarget::Unicode)
            .map_err(|error| SubsetError::Subset(error.to_string()))
    })
}

/// Face `index` of `data` whole, as a single-face TrueType file: the bytes
/// themselves for a single-face file, the one face rebuilt with every table
/// for a collection (a `.ttc` is never embedded).
pub(crate) fn whole_face(data: &[u8], index: u32) -> Result<Vec<u8>, SubsetError> {
    if !data.starts_with(b"ttcf") {
        check_single_face_index(index)?;
        ttf_parser::Face::parse(data, 0).map_err(|error| SubsetError::Parse(error.to_string()))?;
        return Ok(data.to_vec());
    }
    with_provider(data, index, |provider| {
        let tags = provider
            .table_tags()
            .ok_or_else(|| SubsetError::Parse("the table directory does not read".to_string()))?;
        whole_font(provider, &tags).map_err(|error| SubsetError::Subset(error.to_string()))
    })
}

fn with_provider<T>(
    data: &[u8],
    index: u32,
    run: impl FnOnce(&DynamicFontTableProvider<'_>) -> Result<T, SubsetError>,
) -> Result<T, SubsetError> {
    if !data.starts_with(b"ttcf") {
        check_single_face_index(index)?;
    }
    let font = ReadScope::new(data)
        .read::<FontData<'_>>()
        .map_err(|error| SubsetError::Parse(error.to_string()))?;
    let provider = font
        .table_provider(index as usize)
        .map_err(|error| SubsetError::Parse(error.to_string()))?;
    run(&provider)
}

/// A single-face file has face 0 only; `allsorts` would read face 0 for any
/// index.
fn check_single_face_index(index: u32) -> Result<(), SubsetError> {
    if index == 0 {
        Ok(())
    } else {
        Err(SubsetError::Parse(format!("face index {index} of a single-face file")))
    }
}

/// `.notdef` first, then each distinct glyph `code_points` map to.
fn glyph_ids(provider: &DynamicFontTableProvider<'_>, code_points: &BTreeSet<char>) -> Result<Vec<u16>, SubsetError> {
    let no_cmap = |error: allsorts::error::ParseError| SubsetError::NoUnicodeCmap(error.to_string());
    let cmap_data = provider.read_table_data(tag::CMAP).map_err(no_cmap)?;
    let cmap = ReadScope::new(&cmap_data).read::<Cmap<'_>>().map_err(no_cmap)?;
    let (_, record) = allsorts::font::find_good_cmap_subtable(&cmap)
        .ok_or_else(|| SubsetError::NoUnicodeCmap("no Unicode subtable".to_string()))?;
    let subtable = cmap
        .scope
        .offset(record.offset as usize)
        .read::<CmapSubtable<'_>>()
        .map_err(no_cmap)?;
    let mut ids = vec![0];
    for &code_point in code_points {
        let mapped = subtable.map_glyph(u32::from(code_point)).map_err(no_cmap)?;
        if let Some(id) = mapped
            && id != 0
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    Ok(ids)
}
