use std::path::PathBuf;

/// Directory names that never contain authored Markdown worth hashing: dependency
/// and build-output trees. Pruned by [`collect_markdown_files`] alongside hidden
/// (dot-prefixed) directories so `md hash <dir>` does not descend into vendored
/// subtrees. Kept deliberately conservative — a vendored dependency's own docs
/// are not part of the caller's aggregate fingerprint.
const SKIPPED_VENDOR_DIRS: &[&str] = &["node_modules", "target", "vendor"];

/// Recursively collect all markdown files (`.md`, `.dm`) under a directory.
///
/// Hidden directories (dot-prefixed) and the well-known vendored/build-output
/// directories in [`SKIPPED_VENDOR_DIRS`] are pruned.
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
            // Skip hidden directories and well-known vendored/build-output trees.
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.') || SKIPPED_VENDOR_DIRS.contains(&n))
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

    #[test]
    fn skips_vendored_and_hidden_directories() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("top.md"), "# top").unwrap();

        for skipped in ["node_modules", "target", "vendor", ".hidden"] {
            let sub = root.join(skipped);
            fs::create_dir(&sub).unwrap();
            fs::write(sub.join("nested.md"), "# nested").unwrap();
        }

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

        assert_eq!(found, vec!["guide.md".to_string(), "top.md".to_string()]);
    }
}
