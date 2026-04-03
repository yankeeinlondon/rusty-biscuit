use super::framework;
use super::model::{
    ClassificationConfidence, ClassificationSource, FileAssociation, FileClassification,
    FileInventory, FileScanScope, ProgrammingLanguage,
};
use super::registry;
use crate::Result;
use ignore::{DirEntry, WalkBuilder};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 10_000;
const READ_LIMIT: usize = 8 * 1024;

pub fn scan_file_inventory(root: &Path) -> Result<FileInventory> {
    scan_file_inventory_with_exclusions(root, &[])
}

pub fn scan_file_inventory_with_exclusions(
    root: &Path,
    exclude_roots: &[PathBuf],
) -> Result<FileInventory> {
    let scope = FileScanScope {
        root: root.to_path_buf(),
        exclude_roots: exclude_roots
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    root.join(path)
                }
            })
            .collect(),
    };

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry({
            let exclude_roots = scope.exclude_roots.clone();
            move |entry| !is_excluded_entry(entry, &exclude_roots)
        })
        .build();

    let mut classifications = Vec::new();
    let mut total_files_scanned = 0;

    for entry in walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .take(MAX_FILES)
    {
        total_files_scanned += 1;
        classifications.push(classify_file(root, entry.path()));
    }

    Ok(FileInventory {
        scope,
        total_files_scanned,
        classifications,
    })
}

fn is_excluded_entry(entry: &DirEntry, exclude_roots: &[PathBuf]) -> bool {
    if exclude_roots
        .iter()
        .any(|root| entry.path() == root || entry.path().starts_with(root))
    {
        return true;
    }

    if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
        return false;
    }

    matches!(
        entry.file_name().to_str(),
        Some(
            ".git"
                | ".turbo"
                | "node_modules"
                | "target"
                | "vendor"
                | "dist"
                | "build"
                | "__pycache__"
        )
    )
}

fn classify_file(root: &Path, path: &Path) -> FileClassification {
    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let extension = path.extension().and_then(|value| value.to_str());

    if let Some(descriptor) = registry::lookup_exact_filename(file_name) {
        return FileClassification {
            path: relative_path,
            association: descriptor.association,
            language: descriptor.language,
            language_type: descriptor.language_type,
            framework: descriptor.framework,
            related_languages: Vec::new(),
            confidence: ClassificationConfidence::Exact,
            source: ClassificationSource::ExactFilename,
        };
    }

    if let Some(descriptor) = registry::lookup_basename_pattern(file_name) {
        return FileClassification {
            path: relative_path,
            association: descriptor.association,
            language: descriptor.language,
            language_type: descriptor.language_type,
            framework: descriptor.framework,
            related_languages: vec![ProgrammingLanguage::TypeScript],
            confidence: ClassificationConfidence::High,
            source: ClassificationSource::ExactFilename,
        };
    }

    if let Some(ext) = extension
        && let Some(descriptor) = registry::lookup_extension(ext)
    {
        let mut classification = FileClassification {
            path: relative_path,
            association: descriptor.association,
            language: descriptor.language,
            language_type: descriptor.language_type,
            framework: descriptor.framework,
            related_languages: Vec::new(),
            confidence: ClassificationConfidence::Exact,
            source: ClassificationSource::Extension,
        };

        if descriptor.association == FileAssociation::FrameworkFile
            && let Some(framework) = descriptor.framework
        {
            let (related_languages, confidence, source) =
                framework::related_languages(framework, path);
            classification.related_languages = related_languages;
            classification.confidence = confidence;
            classification.source = source;
        }

        return classification;
    }

    let header = read_prefix(path, READ_LIMIT);
    if let Some(ref bytes) = header {
        if let Some(classification) = classify_by_shebang(bytes, &relative_path) {
            return classification;
        }
        if let Some(classification) = classify_by_binary_signature(path, bytes, &relative_path) {
            return classification;
        }
        if is_probably_text(bytes)
            && let Ok(Some(detection)) = hyperpolyglot::detect(path)
            && let Some(language) = ProgrammingLanguage::from_hyperpolyglot(detection.language())
        {
            return FileClassification {
                path: relative_path,
                association: FileAssociation::ProgrammingLanguage,
                language: Some(language),
                language_type: Some(language.language_type()),
                framework: None,
                related_languages: Vec::new(),
                confidence: ClassificationConfidence::Low,
                source: ClassificationSource::Fallback,
            };
        }
    }

    FileClassification {
        path: relative_path,
        association: FileAssociation::Unknown,
        language: None,
        language_type: None,
        framework: None,
        related_languages: Vec::new(),
        confidence: ClassificationConfidence::Low,
        source: ClassificationSource::Fallback,
    }
}

fn read_prefix(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut buffer = vec![0_u8; limit];
    let bytes_read = file.read(&mut buffer).ok()?;
    buffer.truncate(bytes_read);
    Some(buffer)
}

fn classify_by_shebang(bytes: &[u8], relative_path: &Path) -> Option<FileClassification> {
    let content = std::str::from_utf8(bytes).ok()?;
    let first_line = content.lines().next()?;
    let shebang = first_line.strip_prefix("#!")?.trim();

    let interpreter = shebang
        .strip_prefix("/usr/bin/env ")
        .unwrap_or(shebang)
        .split_whitespace()
        .next()
        .unwrap_or(shebang);

    let descriptor = registry::shebang_descriptor(interpreter)?;

    Some(FileClassification {
        path: relative_path.to_path_buf(),
        association: descriptor.association,
        language: descriptor.language,
        language_type: descriptor.language_type,
        framework: descriptor.framework,
        related_languages: Vec::new(),
        confidence: ClassificationConfidence::High,
        source: ClassificationSource::Shebang,
    })
}

fn classify_by_binary_signature(
    path: &Path,
    bytes: &[u8],
    relative_path: &Path,
) -> Option<FileClassification> {
    let association = if bytes.starts_with(b"%PDF-") {
        FileAssociation::Binary
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xff\xd8\xff")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
    {
        FileAssociation::Image
    } else if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"\x1f\x8b")
        || bytes.starts_with(b"7z\xbc\xaf\x27\x1c")
        || bytes.starts_with(b"Rar!\x1a\x07")
        || bytes.starts_with(b"\xfd7zXZ\x00")
    {
        FileAssociation::Archive
    } else if bytes.starts_with(b"OggS")
        || bytes.starts_with(b"fLaC")
        || bytes.starts_with(b"ID3")
        || bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE")
    {
        FileAssociation::Audio
    } else if bytes.starts_with(b"\x7fELF")
        || bytes.starts_with(b"MZ")
        || bytes.starts_with(&[0xfe, 0xed, 0xfa, 0xce])
        || bytes.starts_with(&[0xfe, 0xed, 0xfa, 0xcf])
        || bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
        || bytes.starts_with(&[0xca, 0xfe, 0xba, 0xbe])
    {
        if is_shared_library(path) {
            FileAssociation::Binary
        } else {
            FileAssociation::BinaryExecutable
        }
    } else {
        return None;
    };

    Some(FileClassification {
        path: relative_path.to_path_buf(),
        association,
        language: None,
        language_type: None,
        framework: None,
        related_languages: Vec::new(),
        confidence: ClassificationConfidence::High,
        source: ClassificationSource::BinarySignature,
    })
}

fn is_shared_library(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("so" | "dylib" | "dll" | "a" | "o")
    )
}

fn is_probably_text(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return false;
    }
    std::str::from_utf8(bytes).is_ok()
}

/// Project a package-level inventory from a shared repo inventory.
///
/// Filters classifications by path prefix to extract the subset
/// belonging to a specific package, avoiding a separate filesystem scan.
///
/// ## Returns
///
/// A new `FileInventory` containing only files under `package_path`,
/// excluding any paths under `exclude_roots`.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::file_types::{scan_file_inventory, project_package_inventory};
/// use std::path::Path;
///
/// let repo_root = Path::new("/repo");
/// let repo_inventory = scan_file_inventory(repo_root).unwrap();
///
/// // Extract package inventory without rescanning
/// let package_path = repo_root.join("packages/foo");
/// let package_inventory = project_package_inventory(
///     &repo_inventory,
///     &package_path,
///     &[]
/// );
/// ```
pub fn project_package_inventory(
    repo_inventory: &FileInventory,
    package_path: &Path,
    exclude_roots: &[PathBuf],
) -> FileInventory {
    let classifications: Vec<_> = repo_inventory
        .classifications
        .iter()
        .filter(|c| {
            let full_path = repo_inventory.scope.root.join(&c.path);
            full_path.starts_with(package_path)
                && !exclude_roots.iter().any(|ex| full_path.starts_with(ex))
        })
        .cloned()
        .collect();

    FileInventory {
        scope: FileScanScope {
            root: package_path.to_path_buf(),
            exclude_roots: exclude_roots.to_vec(),
        },
        total_files_scanned: classifications.len(),
        classifications,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::FrameworkKind;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn shebang_is_classified_as_script_language() {
        let dir = TempDir::new().unwrap();
        let script = dir.path().join("tool");
        fs::write(&script, "#!/usr/bin/env python\nprint('hi')\n").unwrap();

        let inventory = scan_file_inventory(dir.path()).unwrap();
        assert_eq!(inventory.classifications.len(), 1);
        let file = &inventory.classifications[0];
        assert_eq!(file.language, Some(ProgrammingLanguage::Python));
        assert_eq!(file.source, ClassificationSource::Shebang);
    }

    #[test]
    fn framework_file_uses_embedded_language_hint() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("App.vue"),
            "<script setup lang=\"ts\">const answer = 42;</script>",
        )
        .unwrap();

        let inventory = scan_file_inventory(dir.path()).unwrap();
        let file = &inventory.classifications[0];
        assert_eq!(file.association, FileAssociation::FrameworkFile);
        assert_eq!(file.framework, Some(FrameworkKind::Vue));
        assert_eq!(
            file.related_languages,
            vec![ProgrammingLanguage::TypeScript]
        );
        assert_eq!(file.source, ClassificationSource::EmbeddedLanguageHint);
    }

    #[test]
    fn generic_html_is_not_treated_as_angular_template() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            "<!doctype html><html></html>",
        )
        .unwrap();

        let inventory = scan_file_inventory(dir.path()).unwrap();
        let file = &inventory.classifications[0];
        assert_eq!(file.association, FileAssociation::Documentation);
        assert_eq!(file.framework, None);
        assert!(file.related_languages.is_empty());
    }

    #[test]
    fn binary_signature_detects_png() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("image.bin"), b"\x89PNG\r\n\x1a\nrest").unwrap();

        let inventory = scan_file_inventory(dir.path()).unwrap();
        let file = &inventory.classifications[0];
        assert_eq!(file.association, FileAssociation::Image);
        assert_eq!(file.source, ClassificationSource::BinarySignature);
    }

    #[test]
    fn project_package_inventory_filters_by_path() {
        let dir = TempDir::new().unwrap();

        // Create repo structure
        fs::create_dir_all(dir.path().join("packages/foo")).unwrap();
        fs::create_dir_all(dir.path().join("packages/bar")).unwrap();

        // Files in package foo
        fs::write(dir.path().join("packages/foo/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("packages/foo/lib.rs"), "pub fn foo() {}").unwrap();

        // Files in package bar
        fs::write(
            dir.path().join("packages/bar/index.js"),
            "console.log('bar')",
        )
        .unwrap();

        // Root level file
        fs::write(dir.path().join("README.md"), "# Repo").unwrap();

        // Scan entire repo
        let repo_inventory = scan_file_inventory(dir.path()).unwrap();
        assert_eq!(repo_inventory.total_files_scanned, 4);

        // Project to package foo
        let foo_path = dir.path().join("packages/foo");
        let foo_inventory = project_package_inventory(&repo_inventory, &foo_path, &[]);

        assert_eq!(foo_inventory.total_files_scanned, 2);
        assert!(
            foo_inventory
                .classifications
                .iter()
                .all(|c| c.language == Some(ProgrammingLanguage::Rust))
        );

        // Project to package bar
        let bar_path = dir.path().join("packages/bar");
        let bar_inventory = project_package_inventory(&repo_inventory, &bar_path, &[]);

        assert_eq!(bar_inventory.total_files_scanned, 1);
        assert_eq!(
            bar_inventory.classifications[0].language,
            Some(ProgrammingLanguage::JavaScript)
        );
    }

    #[test]
    fn project_package_inventory_respects_exclusions() {
        let dir = TempDir::new().unwrap();

        fs::create_dir_all(dir.path().join("pkg/src")).unwrap();
        fs::create_dir_all(dir.path().join("pkg/target")).unwrap();

        fs::write(dir.path().join("pkg/src/lib.rs"), "pub fn lib() {}").unwrap();
        fs::write(dir.path().join("pkg/target/debug.log"), "logs").unwrap();

        let repo_inventory = scan_file_inventory(dir.path()).unwrap();

        let pkg_path = dir.path().join("pkg");
        let exclude_roots = vec![pkg_path.join("target")];
        let pkg_inventory = project_package_inventory(&repo_inventory, &pkg_path, &exclude_roots);

        // Should only have the src file, not the target file
        assert_eq!(pkg_inventory.total_files_scanned, 1);
        assert_eq!(
            pkg_inventory.classifications[0].language,
            Some(ProgrammingLanguage::Rust)
        );
    }
}
