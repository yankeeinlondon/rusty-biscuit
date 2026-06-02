use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

use crate::error::TreeHuggerError;
use crate::shared::ProgrammingLanguage;

/// Configuration options for building a `TreePackage`.
#[derive(Debug, Clone, Default)]
pub struct TreePackageConfig {
    /// Optional language override for the package.
    pub language: Option<ProgrammingLanguage>,
    /// Glob patterns to exclude when scanning files.
    pub ignores: Vec<String>,
}

/// A `TreePackage` represents a collection of related source files.
///
/// Without a language override the package is polyglot: every supported source
/// file is collected regardless of language, and [`language`] reports the
/// dominant (most common) language as the package's primary language. With an
/// override only that language's files are collected. Per-file languages are
/// available via [`language_of`] and [`languages`].
///
/// [`language`]: Self::language
/// [`language_of`]: Self::language_of
/// [`languages`]: Self::languages
#[derive(Debug, Clone)]
pub struct TreePackage {
    /// The root directory for the package.
    pub root_dir: PathBuf,
    /// The primary (dominant) programming language detected for the package.
    pub language: ProgrammingLanguage,
    /// Cached module list for the package.
    modules: Option<Vec<String>>,
    /// All supported source files discovered in the package.
    pub source_files: Vec<PathBuf>,
    /// Markdown documentation files in the package.
    pub doc_files: Vec<PathBuf>,
}

impl TreePackage {
    /// Creates a new `TreePackage` from the provided directory.
    ///
    /// ## Returns
    /// Returns the discovered package with source and doc files populated.
    ///
    /// ## Errors
    /// Returns an error if the directory is not in a git repo or has no sources.
    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self, TreeHuggerError> {
        Self::with_config(dir, TreePackageConfig::default())
    }

    /// Creates a new `TreePackage` with explicit configuration.
    ///
    /// ## Returns
    /// Returns the discovered package with source and doc files populated.
    ///
    /// ## Errors
    /// Returns an error if the directory is not in a git repo or has no sources.
    pub fn with_config<P: AsRef<Path>>(
        dir: P,
        config: TreePackageConfig,
    ) -> Result<Self, TreeHuggerError> {
        let start_dir = dir.as_ref().to_path_buf();
        let git_root = find_git_root(&start_dir)?;
        let root_dir = find_package_root(&start_dir, &git_root);

        let (language, source_files) = match config.language {
            // A forced language restricts collection to that language's files.
            Some(language) => {
                let files = collect_files(&root_dir, language.extensions(), &config.ignores)?;
                (language, files)
            }
            // No override: collect every supported source file and report the
            // dominant language as the package's primary language.
            None => {
                let files = collect_supported_files(&root_dir, &config.ignores)?;
                let language = dominant_language(&files).ok_or_else(|| {
                    TreeHuggerError::NoSourceFiles {
                        path: root_dir.clone(),
                    }
                })?;
                (language, files)
            }
        };

        if source_files.is_empty() {
            return Err(TreeHuggerError::NoSourceFiles { path: root_dir });
        }

        let doc_files = collect_files(&root_dir, &["md"], &config.ignores)?;

        Ok(Self {
            root_dir,
            language,
            modules: None,
            source_files,
            doc_files,
        })
    }

    /// Returns the language detected for a specific source file.
    ///
    /// ## Returns
    /// Returns the file's detected language, falling back to the package's
    /// primary language when the extension is unrecognized.
    pub fn language_of(&self, file: &Path) -> ProgrammingLanguage {
        ProgrammingLanguage::from_path(file).unwrap_or(self.language)
    }

    /// Returns the distinct languages present among the package's source files.
    ///
    /// ## Returns
    /// Returns the languages found, ordered by name.
    pub fn languages(&self) -> Vec<ProgrammingLanguage> {
        let mut languages: Vec<ProgrammingLanguage> = self
            .source_files
            .iter()
            .filter_map(|file| ProgrammingLanguage::from_path(file))
            .collect();
        languages.sort_by_key(ProgrammingLanguage::name);
        languages.dedup();
        languages
    }

    /// Returns the cached module list for the package.
    ///
    /// ## Returns
    /// Returns the discovered module names, computing them on first access.
    pub fn modules(&mut self) -> Vec<String> {
        if let Some(modules) = self.modules.clone() {
            return modules;
        }

        let modules = match self.language {
            ProgrammingLanguage::Rust => rust_modules(&self.root_dir, &self.source_files),
            _ => Vec::new(),
        };

        self.modules = Some(modules.clone());
        modules
    }
}

/// Finds the git repository root by walking up from `start`.
///
/// ## Errors
///
/// Returns `TreeHuggerError::GitRootNotFound` if no `.git` directory is found.
pub fn find_git_root(start: &Path) -> Result<PathBuf, TreeHuggerError> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }

    Err(TreeHuggerError::GitRootNotFound {
        path: start.to_path_buf(),
    })
}

/// Finds the nearest package root between `start` and `git_root`.
///
/// Walks up from `start` looking for package manifest files (Cargo.toml with
/// `[package]`, package.json without `"workspaces"`, etc.). Workspace-only
/// manifests are skipped so that monorepo roots don't become the scan scope.
///
/// Falls back to `start` if no package manifest is found.
pub fn find_package_root(start: &Path, git_root: &Path) -> PathBuf {
    for ancestor in start.ancestors() {
        if ancestor == git_root {
            break;
        }

        if has_package_manifest(ancestor) {
            return ancestor.to_path_buf();
        }
    }

    if has_package_manifest(git_root) {
        return git_root.to_path_buf();
    }

    start.to_path_buf()
}

/// Package manifest file names to check for.
const MANIFESTS: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "go.mod",
    "pyproject.toml",
    "setup.py",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "composer.json",
];

/// Returns `true` if `path` contains a manifest that represents a real package
/// (not a workspace-only root).
///
/// Cargo and Node manifests are parsed structurally so that comments, nested
/// metadata, or unrelated strings cannot change package-root selection.
pub fn has_package_manifest(path: &Path) -> bool {
    for manifest in MANIFESTS {
        let file = path.join(manifest);
        if !file.is_file() {
            continue;
        }

        // For manifests that can be workspace-only, inspect contents.
        match *manifest {
            "Cargo.toml" => return is_cargo_package(&file),
            "package.json" => return is_node_package(&file),
            _ => return true,
        }
    }
    false
}

/// A `Cargo.toml` is a package if it contains a `[package]` table.
///
/// A workspace-only `Cargo.toml` (only `[workspace]`, no `[package]`) is not
/// a package root.
fn is_cargo_package(path: &Path) -> bool {
    biscuit_file::Toml::new(path)
        .map(|manifest| manifest.value().get("package").is_some())
        .unwrap_or(false)
}

/// A `package.json` is a package if it does NOT contain a top-level
/// `"workspaces"` field (which signals a monorepo root).
fn is_node_package(path: &Path) -> bool {
    biscuit_file::Json5::new(path)
        .map(|manifest| manifest.as_json_value().get("workspaces").is_none())
        .unwrap_or(false)
}

/// Returns the most common language among the given files.
///
/// Ties are broken by language name so the result is deterministic.
fn dominant_language(files: &[PathBuf]) -> Option<ProgrammingLanguage> {
    let mut counts: HashMap<ProgrammingLanguage, usize> = HashMap::new();
    for file in files {
        if let Some(language) = ProgrammingLanguage::from_path(file) {
            *counts.entry(language).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .max_by_key(|(language, count)| (*count, std::cmp::Reverse(language.name())))
        .map(|(language, _)| language)
}

/// Collects every supported source file under `root`, regardless of language.
fn collect_supported_files(
    root: &Path,
    ignores: &[String],
) -> Result<Vec<PathBuf>, TreeHuggerError> {
    let mut overrides = OverrideBuilder::new(root);
    for ignore in ignores {
        overrides.add(&format!("!{}", ignore))?;
    }
    let overrides = overrides.build()?;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .overrides(overrides)
        .build();

    for entry in walker {
        let entry = entry.map_err(TreeHuggerError::Ignore)?;
        if !entry
            .file_type()
            .map(|file| file.is_file())
            .unwrap_or(false)
        {
            continue;
        }

        if ProgrammingLanguage::from_path(entry.path()).is_some() {
            files.push(entry.into_path());
        }
    }

    files.sort();
    Ok(files)
}

fn collect_files(
    root: &Path,
    extensions: &[&str],
    ignores: &[String],
) -> Result<Vec<PathBuf>, TreeHuggerError> {
    let mut overrides = OverrideBuilder::new(root);
    for extension in extensions {
        overrides.add(&format!("**/*.{}", extension))?;
    }
    for ignore in ignores {
        overrides.add(&format!("!{}", ignore))?;
    }
    let overrides = overrides.build()?;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .overrides(overrides)
        .build();

    for entry in walker {
        let entry = entry.map_err(TreeHuggerError::Ignore)?;
        if entry
            .file_type()
            .map(|file| file.is_file())
            .unwrap_or(false)
        {
            files.push(entry.into_path());
        }
    }

    files.sort();
    Ok(files)
}

fn rust_modules(root: &Path, files: &[PathBuf]) -> Vec<String> {
    let mut modules = Vec::new();

    for file in files {
        // The package may be polyglot; only Rust files contribute modules.
        if file.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let relative = match file.strip_prefix(root) {
            Ok(path) => path,
            Err(_) => file.as_path(),
        };

        let mut components: Vec<String> = relative
            .components()
            .filter_map(|component| component.as_os_str().to_str().map(String::from))
            .collect();

        if components.first().map(String::as_str) == Some("src") {
            components.remove(0);
        }

        if let Some(file_name) = components.pop() {
            let module_name = file_name.trim_end_matches(".rs");
            if module_name != "mod" && module_name != "lib" && module_name != "main" {
                components.push(module_name.to_string());
            }
        }

        let module_path = components.join("::");
        if !module_path.is_empty() {
            modules.push(module_path);
        }
    }

    modules.sort();
    modules.dedup();
    modules
}
