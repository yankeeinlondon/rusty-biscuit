use super::framework;
use super::model::{
    ClassificationConfidence, ClassificationSource, FileAssociation, FileClassification,
    FileInventory, FileScanScope, ProgrammingLanguage,
};
use super::registry;
use crate::Result;
use crate::performance;
use ignore::{DirEntry, WalkBuilder};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tracing::trace;

pub(crate) const MAX_FILES: usize = 10_000;
const READ_LIMIT: usize = 8 * 1024;

pub fn scan_file_inventory(root: &Path) -> Result<FileInventory> {
    scan_file_inventory_with_exclusions(root, &[])
}

pub fn scan_file_inventory_with_exclusions(
    root: &Path,
    exclude_roots: &[PathBuf],
) -> Result<FileInventory> {
    let started = Instant::now();
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

    // Use parallel walker for production, sequential for tests to avoid ordering issues
    #[cfg(not(test))]
    let (classifications, total_files_scanned) =
        scan_inventory_parallel(root, &scope.exclude_roots);
    #[cfg(test)]
    let (classifications, total_files_scanned) =
        scan_inventory_sequential(root, &scope.exclude_roots);

    performance::record_logged_stage(
        "filesystem.file_inventory.scan",
        started.elapsed(),
        tracing::Level::DEBUG,
    );

    Ok(FileInventory {
        scope,
        total_files_scanned,
        classifications: Arc::new(classifications),
    })
}

#[cfg(not(test))]
fn scan_inventory_parallel(root: &Path, exclude_roots: &[PathBuf]) -> (Vec<FileClassification>, usize) {
    use ignore::WalkState;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    let shared: Arc<Mutex<Vec<FileClassification>>> = Arc::new(Mutex::new(Vec::new()));
    let scanned = Arc::new(AtomicUsize::new(0));

    WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry({
            let exclude_roots = exclude_roots.to_vec();
            move |entry| !is_excluded_entry(entry, &exclude_roots)
        })
        .build_parallel()
        .run(|| {
            let _shared = Arc::clone(&shared);
            let scanned = Arc::clone(&scanned);
            let scan_root = root.to_path_buf();
            let mut local = Vec::new();
            Box::new(move |result| {
                let Ok(entry) = result else {
                    return WalkState::Continue;
                };
                if !entry
                    .file_type()
                    .is_some_and(|ft| ft.is_file())
                {
                    return WalkState::Continue;
                }
                let idx = scanned.fetch_add(1, Ordering::Relaxed);
                if idx >= MAX_FILES {
                    return WalkState::Continue;
                }
                // Only record per-file counters when metrics feature is enabled
                // to reduce atomic overhead in the hot path.
                #[cfg(feature = "metrics")]
                performance::increment_counter("filesystem.file_inventory.files_scanned", 1);
                local.push(classify_file(
                    &scan_root, entry.path()));
                WalkState::Continue
            })
        });

    let total = scanned.load(Ordering::Relaxed).min(MAX_FILES);
    let mut classifications = match Arc::try_unwrap(shared) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => arc.lock().unwrap_or_else(|e| e.into_inner()).clone(),
    };
    classifications.sort_by(|a, b| a.path.cmp(&b.path));
    (classifications, total)
}

#[cfg(test)]
fn scan_inventory_sequential(root: &Path, exclude_roots: &[PathBuf]) -> (Vec<FileClassification>, usize) {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry({
            let exclude_roots = exclude_roots.to_vec();
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
        // Only record per-file counters when metrics feature is enabled
        // to reduce overhead in the hot path.
        #[cfg(feature = "metrics")]
        performance::increment_counter("filesystem.file_inventory.files_scanned", 1);
        classifications.push(classify_file(root, entry.path()));
    }

    (classifications, total_files_scanned)
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

    entry
        .file_name()
        .to_str()
        .is_some_and(should_skip_directory_name)
}

pub(crate) fn should_skip_directory_name(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".turbo" | "node_modules" | "target" | "vendor" | "dist" | "build"
            | "__pycache__" | ".venv" | "venv" | ".env" | ".pytest_cache" | ".mypy_cache"
            | ".tox" | ".ruff_cache" | ".next" | ".nuxt" | ".output" | ".vercel"
            | ".parcel-cache" | ".cache" | "out" | "bin" | "obj" | ".idea" | ".vscode"
            | "coverage" | "htmlcov" | ".svelte-kit" | ".astro"
    )
}

pub(crate) fn classify_file(root: &Path, path: &Path) -> FileClassification {
    // Start timing unconditionally; the overhead of Instant::now() is ~10-30ns
    // and is only used when metrics feature is enabled.
    let started = Instant::now();
    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let extension = path.extension().and_then(|value| value.to_str());

    if let Some(descriptor) = registry::lookup_exact_filename(file_name) {
        return finish_classification(
            relative_path,
            FileClassification {
                path: PathBuf::new(),
                association: descriptor.association,
                language: descriptor.language,
                language_type: descriptor.language_type,
                framework: descriptor.framework,
                related_languages: Vec::new(),
                confidence: ClassificationConfidence::Exact,
                source: ClassificationSource::ExactFilename,
            },
            started,
        );
    }

    if let Some(descriptor) = registry::lookup_basename_pattern(file_name) {
        return finish_classification(
            relative_path,
            FileClassification {
                path: PathBuf::new(),
                association: descriptor.association,
                language: descriptor.language,
                language_type: descriptor.language_type,
                framework: descriptor.framework,
                related_languages: vec![ProgrammingLanguage::TypeScript],
                confidence: ClassificationConfidence::High,
                source: ClassificationSource::ExactFilename,
            },
            started,
        );
    }

    if let Some(ext) = extension
        && let Some(descriptor) = registry::lookup_extension(ext)
    {
        let mut classification = FileClassification {
            path: PathBuf::new(),
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
            #[cfg(feature = "metrics")]
            let framework_started = Instant::now();
            let (related_languages, confidence, source) =
                framework::related_languages(framework, path);
            #[cfg(feature = "metrics")]
            performance::record_stage(
                "filesystem.file_inventory.classify.framework",
                framework_started.elapsed(),
            );
            classification.related_languages = related_languages;
            classification.confidence = confidence;
            classification.source = source;
        }

        return finish_classification(relative_path, classification, started);
    }

    let header = read_prefix(path, READ_LIMIT);
    if let Some(ref bytes) = header {
        if let Some(classification) = classify_by_shebang(bytes) {
            return finish_classification(relative_path, classification, started);
        }
        if let Some(classification) = classify_by_binary_signature(path, bytes) {
            return finish_classification(relative_path, classification, started);
        }
        if is_probably_text(bytes) && extension.is_some_and(is_hyperpolyglot_worthwhile) {
            #[cfg(feature = "metrics")]
            performance::increment_counter(
                "filesystem.file_inventory.files_classified_by_content",
                1,
            );
            #[cfg(feature = "metrics")]
            let hyperpolyglot_started = Instant::now();
            let detection = hyperpolyglot::detect(path);
            #[cfg(feature = "metrics")]
            {
                let hyperpolyglot_elapsed = hyperpolyglot_started.elapsed();
                performance::record_stage(
                    "filesystem.file_inventory.classify.hyperpolyglot",
                    hyperpolyglot_elapsed,
                );
                trace!(
                    path = %relative_path.display(),
                    duration_ms = performance::duration_ms(hyperpolyglot_elapsed),
                    "hyperpolyglot probe complete"
                );
            }
            #[cfg(not(feature = "metrics"))]
            {
                trace!(
                    path = %relative_path.display(),
                    "hyperpolyglot probe complete"
                );
            }
            if let Ok(Some(detection)) = detection
                && let Some(language) =
                    ProgrammingLanguage::from_hyperpolyglot(detection.language())
            {
                #[cfg(feature = "metrics")]
                performance::increment_counter(
                    "filesystem.file_inventory.files_classified_by_hyperpolyglot",
                    1,
                );
                return finish_classification(
                    relative_path,
                    FileClassification {
                        path: PathBuf::new(),
                        association: FileAssociation::ProgrammingLanguage,
                        language: Some(language),
                        language_type: Some(language.language_type()),
                        framework: None,
                        related_languages: Vec::new(),
                        confidence: ClassificationConfidence::Low,
                        source: ClassificationSource::Fallback,
                    },
                    started,
                );
            }
        }
    }

    finish_classification(
        relative_path,
        FileClassification {
            path: PathBuf::new(),
            association: FileAssociation::Unknown,
            language: None,
            language_type: None,
            framework: None,
            related_languages: Vec::new(),
            confidence: ClassificationConfidence::Low,
            source: ClassificationSource::Fallback,
        },
        started,
    )
}

fn finish_classification(
    relative_path: PathBuf,
    mut classification: FileClassification,
    started: Instant,
) -> FileClassification {
    classification.path = relative_path;
    // Only record detailed per-file metrics when the metrics feature is enabled.
    // This avoids mutex contention and counter overhead in the hot path for
    // the default build.
    #[cfg(feature = "metrics")]
    {
        let duration = started.elapsed();
        performance::increment_counter("filesystem.file_inventory.files_classified", 1);
        performance::increment_counter_dynamic(classification_counter_name(&classification), 1);
        performance::record_stage(classification_stage_name(&classification), duration);
        trace!(
            path = %classification.path.display(),
            source = ?classification.source,
            association = ?classification.association,
            duration_ms = performance::duration_ms(duration),
            "file classified"
        );
    }
    #[cfg(not(feature = "metrics"))]
    {
        let _ = started; // suppress unused warning when metrics is off
        trace!(
            path = %classification.path.display(),
            source = ?classification.source,
            association = ?classification.association,
            "file classified"
        );
    }
    classification
}

fn classification_stage_name(classification: &FileClassification) -> &'static str {
    match classification.source {
        ClassificationSource::ExactFilename => "filesystem.file_inventory.classify.exact_filename",
        ClassificationSource::Extension => "filesystem.file_inventory.classify.extension",
        ClassificationSource::Shebang => "filesystem.file_inventory.classify.shebang",
        ClassificationSource::EmbeddedLanguageHint => {
            "filesystem.file_inventory.classify.embedded_language_hint"
        }
        ClassificationSource::BinarySignature => {
            "filesystem.file_inventory.classify.binary_signature"
        }
        ClassificationSource::Fallback => "filesystem.file_inventory.classify.fallback",
    }
}

fn classification_counter_name(classification: &FileClassification) -> &'static str {
    match classification.source {
        ClassificationSource::ExactFilename => {
            "filesystem.file_inventory.classified_exact_filename"
        }
        ClassificationSource::Extension => "filesystem.file_inventory.classified_extension",
        ClassificationSource::Shebang => "filesystem.file_inventory.classified_shebang",
        ClassificationSource::EmbeddedLanguageHint => {
            "filesystem.file_inventory.classified_embedded_language_hint"
        }
        ClassificationSource::BinarySignature => {
            "filesystem.file_inventory.classified_binary_signature"
        }
        ClassificationSource::Fallback => "filesystem.file_inventory.classified_fallback",
    }
}

fn read_prefix(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut buffer = vec![0_u8; limit];
    let bytes_read = file.read(&mut buffer).ok()?;
    buffer.truncate(bytes_read);
    Some(buffer)
}

fn classify_by_shebang(bytes: &[u8]) -> Option<FileClassification> {
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

    performance::increment_counter("filesystem.file_inventory.files_classified_by_content", 1);

    Some(FileClassification {
        path: PathBuf::new(),
        association: descriptor.association,
        language: descriptor.language,
        language_type: descriptor.language_type,
        framework: descriptor.framework,
        related_languages: Vec::new(),
        confidence: ClassificationConfidence::High,
        source: ClassificationSource::Shebang,
    })
}

fn classify_by_binary_signature(path: &Path, bytes: &[u8]) -> Option<FileClassification> {
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

    performance::increment_counter("filesystem.file_inventory.files_classified_by_content", 1);

    Some(FileClassification {
        path: PathBuf::new(),
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

/// Determines whether a file extension is worth analyzing with hyperpolyglot.
///
/// Hyperpolyglot is a heavyweight content analysis that may spawn subprocesses.
/// Skip generic extensions that are unlikely to contain meaningful source code.
fn is_hyperpolyglot_worthwhile(ext: &str) -> bool {
    !matches!(
        ext.to_ascii_lowercase().as_str(),
        "txt" | "log" | "tmp" | "bak" | "out" | "cache" | "md" | "rst"
            | "csv" | "tsv" | "json" | "yaml" | "yml" | "xml" | "ini"
            | "cfg" | "conf" | "properties" | "env" | "gitignore"
            | "dockerignore" | "eslintignore" | "prettierignore"
    )
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
        classifications: Arc::new(classifications),
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
