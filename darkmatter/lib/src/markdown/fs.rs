use biscuit_file::{FileReference, FileReferenceError, FileResolutionContext, ProbeDisposition};
use std::path::PathBuf;

/// What a reference names when a directory is as acceptable as a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceEntry {
    File(PathBuf),
    Directory(PathBuf),
}

/// Resolves `reference` to the first candidate in its plan that exists as a
/// regular file or a directory.
///
/// Precedence is file resolution's own: candidates are taken in the shared
/// resolver's order ([`FileReference::resolve_detailed`]), with its
/// containment checks, so an earlier file outranks a later directory and an
/// earlier directory outranks a later file. A candidate whose metadata cannot
/// be read (permission denied, a symlink loop) stops the search exactly as it
/// stops file resolution; a later candidate never turns that failure into a
/// result. An existing entry that is neither (a socket, a FIFO) is skipped. A
/// recursive (`%`) reference resolves to files only.
///
/// ## Returns
///
/// `Ok(None)` when no candidate exists as a file or a directory. A
/// [`ReferenceEntry::File`] is the path [`FileReference::resolve_in_context`]
/// returns.
///
/// ## Errors
///
/// The resolver's typed error, including [`FileReferenceError::Io`] for the
/// first candidate whose metadata cannot be read.
pub fn resolve_entry_in_context(
    reference: &FileReference,
    context: &FileResolutionContext,
) -> Result<Option<ReferenceEntry>, FileReferenceError> {
    let resolution = reference.resolve_detailed(context);
    if !reference.class().recursive {
        // The resolver probed every candidate before its outcome and advanced
        // past each `NonFile` one; the first of those that is a directory
        // precedes the file match, the I/O failure, or the miss.
        for probed in resolution.candidates() {
            if probed.disposition() != ProbeDisposition::NonFile {
                continue;
            }
            let path = probed.candidate().path();
            match std::fs::metadata(path) {
                Ok(metadata) if metadata.is_dir() => {
                    return Ok(Some(ReferenceEntry::Directory(path.to_path_buf())));
                }
                Ok(_) => {}
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(FileReferenceError::Io { path: path.to_path_buf(), source });
                }
            }
        }
    }
    Ok(resolution.into_convenience()?.map(ReferenceEntry::File))
}

/// Recursively collect all markdown files (`.md`, `.dm`) under a directory.
///
/// Only hidden directories (dot-prefixed) are pruned. Vendored/build-output
/// trees such as `node_modules`, `target`, and `vendor` are traversed and their
/// Markdown contributes to the aggregate — matching the pre-optimization
/// membership. A future opt-in ignore policy that changes this membership would
/// require a separately approved compatibility ruling and hash-migration
/// semantics.
pub fn collect_markdown_files(dir: &std::path::Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    collect_markdown_files_recursive(dir, &mut files)?;
    Ok(files)
}

fn collect_markdown_files_recursive(
    dir: &std::path::Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip only hidden directories (dot-prefixed). Vendored/build-output
            // trees are part of the aggregate again (Finding 22 revert).
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
            {
                continue;
            }
            collect_markdown_files_recursive(&path, files)?;
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("dm"))
        {
            files.push(path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Finding 22 revert: vendored/build-output directory names
    /// (`node_modules`, `target`, `vendor`) are traversed again and their
    /// Markdown is collected; only hidden (dot-prefixed) directories are pruned.
    #[test]
    fn includes_vendored_but_skips_hidden_directories() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("top.md"), "# top").unwrap();

        // Vendored/build-output trees are now part of the aggregate.
        for vendored in ["node_modules", "target", "vendor"] {
            let sub = root.join(vendored);
            fs::create_dir(&sub).unwrap();
            fs::write(sub.join("nested.md"), "# nested").unwrap();
        }

        // Hidden directories remain pruned.
        let hidden = root.join(".hidden");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("secret.md"), "# secret").unwrap();

        // A real content directory must still be descended into.
        let docs = root.join("docs");
        fs::create_dir(&docs).unwrap();
        fs::write(docs.join("guide.md"), "# guide").unwrap();

        let mut found: Vec<String> = collect_markdown_files(root)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        found.sort();

        // One `nested.md` per vendored dir + guide.md + top.md; the hidden
        // `secret.md` is excluded.
        assert_eq!(
            found,
            vec![
                "guide.md".to_string(),
                "nested.md".to_string(),
                "nested.md".to_string(),
                "nested.md".to_string(),
                "top.md".to_string(),
            ],
        );
    }
}
