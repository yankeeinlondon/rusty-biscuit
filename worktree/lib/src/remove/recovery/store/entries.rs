//! `entries.jsonl.gz`: every journal entry of an operation, one JSON line
//! each ([`JournalEntry`]), gzip-compressed. The report names it
//! ([`OperationReport::entries_file`](crate::remove::outcome::OperationReport::entries_file))
//! instead of listing the entries inline.
//!
//! `wt` writes this file but never reads it back: recovery reads the
//! journals, which are deleted only after this file is published.

use std::io::{self, Write};
use std::path::Path;

use flate2::{Compression, GzBuilder};

use super::super::journal::JournalEntry;
use super::super::storage;
use super::{SaveStep, fault};

/// Replaces `path` with `entries`, as [`storage::write_durable`] replaces a
/// file: written and flushed under a hidden temporary name, then renamed
/// over `path`, so `path` is always absent, the old file, or the complete
/// new one.
///
/// The gzip header carries no name and a zero modification time, so equal
/// entries give equal bytes.
pub(super) fn write(path: &Path, entries: &[JournalEntry]) -> Result<(), super::StoreError> {
    let temp = storage::temp_beside(path)?;
    let result = (|| -> Result<(), super::StoreError> {
        let file = storage::create_private_file(&temp)?;
        let encoder = GzBuilder::new().mtime(0).write(file, Compression::default());
        // `serde_json` writes each token separately, and the encoder runs a
        // compression step per write: unbuffered, compressing 100,000
        // entries took seconds in a debug build instead of a tenth of one.
        let mut lines = io::BufWriter::with_capacity(64 * 1024, encoder);
        for entry in entries {
            serde_json::to_writer(&mut lines, entry).map_err(io::Error::from)?;
            lines.write_all(b"\n")?;
        }
        let file = lines.into_inner().map_err(io::IntoInnerError::into_error)?.finish()?;
        fault::save_at(SaveStep::EntriesWritten, &temp)?;
        storage::flush(&file)?;
        fault::save_at(SaveStep::EntriesFlushed, &temp)?;
        // Closed before the rename: Windows refuses to rename an open file.
        drop(file);
        Ok(storage::publish(&temp, path)?)
    })();
    if let Err(error) = &result
        && !fault::crashed(error)
    {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// Reads an entries file back, for tests: the gzip trailer must check out,
/// every line must end in a newline, and each must be one [`JournalEntry`].
#[cfg(test)]
pub(crate) fn read(path: &Path) -> Vec<JournalEntry> {
    use std::io::Read;
    let mut text = String::new();
    flate2::read::GzDecoder::new(std::fs::File::open(path).unwrap()).read_to_string(&mut text).unwrap();
    if text.is_empty() {
        return Vec::new();
    }
    let lines = text.strip_suffix('\n').unwrap_or_else(|| panic!("the last line of {} is unterminated", path.display()));
    lines.split('\n').map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("{line}: {error}"))).collect()
}
