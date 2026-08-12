use std::path::PathBuf;

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
