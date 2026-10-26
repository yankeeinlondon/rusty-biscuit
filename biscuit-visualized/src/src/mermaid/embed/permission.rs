//! Whether a font face may be embedded in a page, read from the face's own
//! `OS/2` `fsType` and outline tables.

use std::sync::atomic::{AtomicBool, Ordering};

use ttf_parser::{RawFace, Tag};

/// `fsType` bit 1: Restricted License embedding.
const RESTRICTED: u16 = 0x0002;
/// `fsType` bit 2: Preview & Print embedding.
const PREVIEW_PRINT: u16 = 0x0004;
/// `fsType` bit 3: Editable embedding.
const EDITABLE: u16 = 0x0008;
/// `fsType` bit 8: No subsetting.
const NO_SUBSETTING: u16 = 0x0100;
/// `fsType` bit 9: Bitmap embedding only.
const BITMAP_ONLY: u16 = 0x0200;
/// Every bit the OpenType specification defines; the rest are reserved.
const DEFINED_BITS: u16 = RESTRICTED | PREVIEW_PRINT | EDITABLE | NO_SUBSETTING | BITMAP_ONLY;

/// Byte offset of `fsType` in the `OS/2` table (every table version).
const FS_TYPE_OFFSET: usize = 8;

/// Process-wide guard: reserved `fsType` bits are logged at most once.
static RESERVED_BITS_LOGGED: AtomicBool = AtomicBool::new(false);

/// The embedding permission a face grants, the least restrictive of the
/// permission bits it sets (the OpenType rule for fonts that set several).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Permission {
    /// No permission bit set (`0x0000`).
    Installable,
    /// `0x0008`.
    Editable,
    /// `0x0004`.
    PreviewAndPrint,
}

/// Why a face may not be embedded; its family is then measured and embedded
/// as the bundled Inter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// `0x0002` is the only permission bit.
    Restricted { fs_type: u16 },
    /// Bitmap embedding only (`0x0200`): this opt-in embeds outlines.
    BitmapOnly { fs_type: u16 },
    /// The face does not parse, or has no `OS/2` table long enough to hold
    /// `fsType`.
    NoReadableOs2,
    /// The face is not a static TrueType face: it has `CFF `/`CFF2`
    /// outlines, no `glyf` table, or an `fvar` table (a variable font). The
    /// page embeds `font/ttf` only, and a subset of a variable face keeps only
    /// its default instance.
    OutlineFormat,
}

/// What may be done with one face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmbedPermission {
    /// Embed a subset of the face.
    Embed { permission: Permission, fs_type: u16 },
    /// Embed the whole face: `fsType` sets "no subsetting" (`0x0100`).
    EmbedUnsubset { permission: Permission, fs_type: u16 },
    Refuse(Refusal),
}

/// Classifies face `index` of the font file `data` (a single font or a
/// collection).
///
/// Anything that cannot be established reads as a refusal, never as a
/// permission: an unparseable face or a missing or short `OS/2` table is
/// [`Refusal::NoReadableOs2`]. Reserved `fsType` bits are ignored (and logged
/// once per process at debug level); the defined bits decide.
pub(crate) fn classify(data: &[u8], index: u32) -> EmbedPermission {
    let Ok(face) = RawFace::parse(data, index) else {
        return EmbedPermission::Refuse(Refusal::NoReadableOs2);
    };
    let has = |tag: &[u8; 4]| face.table(Tag::from_bytes(tag)).is_some();
    if has(b"CFF ") || has(b"CFF2") || has(b"fvar") || !has(b"glyf") {
        return EmbedPermission::Refuse(Refusal::OutlineFormat);
    }
    let Some(fs_type) = face
        .table(Tag::from_bytes(b"OS/2"))
        .and_then(|os2| os2.get(FS_TYPE_OFFSET..FS_TYPE_OFFSET + 2))
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
    else {
        return EmbedPermission::Refuse(Refusal::NoReadableOs2);
    };
    classify_fs_type(fs_type)
}

/// The decision for a raw `fsType` value.
fn classify_fs_type(fs_type: u16) -> EmbedPermission {
    if fs_type & !DEFINED_BITS != 0 && !RESERVED_BITS_LOGGED.swap(true, Ordering::Relaxed) {
        tracing::debug!(fs_type = format!("{fs_type:#06x}"), "font fsType sets reserved bits; ignoring them");
    }
    let permission = if fs_type & (RESTRICTED | PREVIEW_PRINT | EDITABLE) == 0 {
        Permission::Installable
    } else if fs_type & EDITABLE != 0 {
        Permission::Editable
    } else if fs_type & PREVIEW_PRINT != 0 {
        Permission::PreviewAndPrint
    } else {
        return EmbedPermission::Refuse(Refusal::Restricted { fs_type });
    };
    if fs_type & BITMAP_ONLY != 0 {
        EmbedPermission::Refuse(Refusal::BitmapOnly { fs_type })
    } else if fs_type & NO_SUBSETTING != 0 {
        EmbedPermission::EmbedUnsubset { permission, fs_type }
    } else {
        EmbedPermission::Embed { permission, fs_type }
    }
}
