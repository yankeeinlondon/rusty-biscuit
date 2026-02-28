use std::path::PathBuf;

/// Recursively collect all markdown files (`.md`, `.dm`) under a directory.
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
            // Skip hidden directories
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
