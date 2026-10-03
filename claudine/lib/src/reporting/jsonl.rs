use std::io::Write;
use std::path::Path;

use serde::Serialize;

/// Append `record` as one `\n`-terminated JSON line, creating the file and its
/// parent directories when needed. Shared by dispatch event logs and steering
/// audit records.
pub fn append_record<T: Serialize>(record: &T, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut line = serde_json::to_string(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())
}
