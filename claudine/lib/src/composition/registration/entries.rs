//! What a path holds, read without following links, and whether two
//! spellings reach one directory entry.

use std::fs;
use std::io;
use std::path::Path;

use sniff::filesystem::query::FileIdentity;

use super::{FILE_ATTRIBUTE_REPARSE_POINT, Obstruction, PublishError, access_log, io_error};

/// What a path holds, read without following links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Entry {
    Absent,
    File,
    Directory,
    Obstruction(Obstruction),
    /// A socket, device, or anything else.
    Other,
}

/// Classifies an entry from its no-follow file type and its Windows file
/// attributes (`0` elsewhere). A symbolic link wins over the reparse bit it
/// also carries on Windows; a junction carries only the bit.
pub(super) fn obstruction_of(is_symlink: bool, attributes: u32) -> Option<Obstruction> {
    if is_symlink {
        Some(Obstruction::SymbolicLink)
    } else if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        Some(Obstruction::ReparsePoint)
    } else {
        None
    }
}

#[cfg(windows)]
fn attributes(metadata: &fs::Metadata) -> u32 {
    std::os::windows::fs::MetadataExt::file_attributes(metadata)
}

#[cfg(not(windows))]
const fn attributes(_: &fs::Metadata) -> u32 {
    0
}

pub(super) fn entry_at(path: &Path) -> Result<Entry, PublishError> {
    access_log::note("inspect", path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Entry::Absent),
        Err(error) => return Err(io_error("inspect", path)(error)),
    };
    Ok(match obstruction_of(metadata.file_type().is_symlink(), attributes(&metadata)) {
        Some(obstruction) => Entry::Obstruction(obstruction),
        None if metadata.is_dir() => Entry::Directory,
        None if metadata.is_file() => Entry::File,
        None => Entry::Other,
    })
}

/// Whether the spellings `a` and `b` reach one directory entry: each is a
/// regular file or a folder (examined without following a link), both have
/// one file identity, their folders are one folder, and that folder does not
/// list both names. On a case-insensitive volume `schemas/Action.yaml` and
/// `schemas/action.yaml` are one entry; on a case-sensitive one they are
/// two, as are two hard links to one file.
pub(super) fn one_entry(a: &Path, b: &Path) -> Result<bool, PublishError> {
    fn identity(path: &Path) -> Result<Option<FileIdentity>, PublishError> {
        if !matches!(entry_at(path)?, Entry::File | Entry::Directory) {
            return Ok(None);
        }
        match FileIdentity::of_path(path, false) {
            Ok(identity) => Ok(Some(identity)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error("inspect", path)(error)),
        }
    }
    let Some(found) = identity(a)? else { return Ok(false) };
    if identity(b)? != Some(found) {
        return Ok(false);
    }
    let (Some(folder), Some(other), Some(name_a), Some(name_b)) = (a.parent(), b.parent(), a.file_name(), b.file_name())
    else {
        return Ok(false);
    };
    if folder != other {
        let Some(folder_identity) = identity(folder)? else { return Ok(false) };
        if identity(other)? != Some(folder_identity) {
            return Ok(false);
        }
    }
    if name_a == name_b {
        return Ok(true);
    }
    access_log::note("list", folder);
    let (mut listed_a, mut listed_b) = (false, false);
    for entry in fs::read_dir(folder).map_err(io_error("list", folder))? {
        let name = entry.map_err(io_error("list", folder))?.file_name();
        listed_a |= name == name_a;
        listed_b |= name == name_b;
    }
    Ok(!(listed_a && listed_b))
}
