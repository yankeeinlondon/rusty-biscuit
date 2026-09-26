use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncludeRules {
    Present(PathBuf),
    Empty,
    Missing,
    Indeterminate(String),
}

impl IncludeRules {
    pub fn locate(root: &Path) -> Self {
        let path = root.join(".worktreeinclude");
        match fs::symlink_metadata(&path) {
            Ok(_) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::Missing,
            Err(error) => return Self::Indeterminate(error.to_string()),
        }
        let resolved = match fs::canonicalize(&path) {
            Ok(resolved) => resolved,
            Err(error) => return Self::Indeterminate(error.to_string()),
        };
        let root = match fs::canonicalize(root) {
            Ok(root) => root,
            Err(error) => return Self::Indeterminate(error.to_string()),
        };
        if !resolved.starts_with(&root) {
            return Self::Indeterminate("rules link points outside the checkout".into());
        }
        match fs::metadata(&resolved) {
            Ok(meta) if meta.is_file() => {
                match fs::read(&resolved) {
                    Ok(bytes) if bytes.is_empty() => Self::Empty,
                    Ok(_) => Self::Present(path),
                    Err(error) => Self::Indeterminate(error.to_string()),
                }
            }
            Ok(_) => Self::Indeterminate("rules path is not a file".into()),
            Err(error) => Self::Indeterminate(error.to_string()),
        }
    }
}
