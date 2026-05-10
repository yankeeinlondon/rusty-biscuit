//! Path classification utilities for source code and documentation files.
//!
//! This module provides low-level path classification functions that are used
//! by both the blast-radius analyzer and git commit rendering. Keeping them in
//! a dedicated leaf module avoids a layering inversion where `git` depends on
//! `blast_radius`.

use std::path::Path;

use crate::filesystem::FileAssociation;
use crate::filesystem::file_types::{lookup_exact_filename, lookup_extension};

/// Returns `true` if the path refers to a source code file.
///
/// Classification uses the file-type registry (exact filename match, then
/// extension match). Paths whose association is `ProgrammingLanguage`,
/// `FrameworkFile`, or `Styling` are considered source code. HTML/HTM files
/// are also accepted as an explicit fallback (they are classified as
/// `Documentation` in the registry but were historically treated as source
/// code in the CLI).
pub fn is_source_code_path(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // Try exact filename match first
    if let Some(desc) = lookup_exact_filename(file_name) {
        return matches!(
            desc.association,
            FileAssociation::ProgrammingLanguage
                | FileAssociation::FrameworkFile
                | FileAssociation::Styling
        );
    }

    // Try extension match
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        // Explicit fallback: accept HTML/HTM (classified as Documentation in
        // the registry, but historically treated as source code)
        if ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm") {
            return true;
        }

        if let Some(desc) = lookup_extension(ext) {
            return matches!(
                desc.association,
                FileAssociation::ProgrammingLanguage
                    | FileAssociation::FrameworkFile
                    | FileAssociation::Styling
            );
        }
    }

    false
}

/// Returns `true` if the path refers to a documentation file.
///
/// Classification uses the file-type registry (exact filename match, then
/// extension match). Paths whose association is `Documentation` are considered
/// documentation. This includes bare filenames like `README`, `CHANGELOG`,
/// and `CONTRIBUTING` (without extension) as well as extension-based matches.
pub fn is_documentation_path(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    if let Some(desc) = lookup_exact_filename(file_name)
        && matches!(desc.association, FileAssociation::Documentation)
    {
        return true;
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lower = ext.to_ascii_lowercase();
        if matches!(lower.as_str(), "md" | "mdx" | "rst" | "txt" | "adoc") {
            return true;
        }

        if let Some(desc) = lookup_extension(ext) {
            return matches!(desc.association, FileAssociation::Documentation);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    mod source_code_detection {
        use super::*;

        #[test]
        fn rust_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("src/main.rs")));
        }

        #[test]
        fn typescript_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("src/index.ts")));
        }

        #[test]
        fn vue_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("components/App.vue")));
        }

        #[test]
        fn css_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("styles/main.css")));
        }

        #[test]
        fn html_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("public/index.html")));
        }

        #[test]
        fn htm_file_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("public/page.htm")));
        }

        #[test]
        fn markdown_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("docs/README.md")));
        }

        #[test]
        fn json_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("config.json")));
        }

        #[test]
        fn png_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("images/logo.png")));
        }

        #[test]
        fn no_extension_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("Makefile")));
        }

        #[test]
        fn scss_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("styles/theme.scss")));
        }

        #[test]
        fn python_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from("script.py")));
        }

        #[test]
        fn toml_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("Cargo.toml")));
        }

        #[test]
        fn yaml_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("config.yaml")));
        }
    }

    mod documentation_detection {
        use super::*;

        #[test]
        fn markdown_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/README.md")));
        }

        #[test]
        fn mdx_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/guide.mdx")));
        }

        #[test]
        fn rst_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/index.rst")));
        }

        #[test]
        fn txt_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("LICENSE.txt")));
        }

        #[test]
        fn adoc_file_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("docs/guide.adoc")));
        }

        #[test]
        fn bare_readme_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("README")));
        }

        #[test]
        fn bare_changelog_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("CHANGELOG")));
        }

        #[test]
        fn bare_contributing_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("CONTRIBUTING")));
        }

        #[test]
        fn readme_md_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("readme.md")));
        }

        #[test]
        fn changelog_md_is_documentation() {
            assert!(is_documentation_path(&PathBuf::from("changelog.md")));
        }

        #[test]
        fn rust_file_is_not_documentation() {
            assert!(!is_documentation_path(&PathBuf::from("src/lib.rs")));
        }

        #[test]
        fn json_is_not_documentation() {
            assert!(!is_documentation_path(&PathBuf::from("package.json")));
        }

        #[test]
        fn png_is_not_documentation() {
            assert!(!is_documentation_path(&PathBuf::from("images/logo.png")));
        }
    }
}
