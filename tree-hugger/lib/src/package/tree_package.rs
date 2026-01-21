use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

use crate::error::TreeHuggerError;
use crate::shared::ProgrammingLanguage;

/// Configuration options for building a `TreePackage`.
#[derive(Debug, Clone)]
pub struct TreePackageConfig {
    /// Optional language override for the package.
    pub language: Option<ProgrammingLanguage>,
    /// Glob patterns to exclude when scanning files.
    pub ignores: Vec<String>,
}

impl Default for TreePackageConfig {
    fn default() -> Self {
        Self {
            language: None,
            ignores: Vec::new(),
        }
    }
}

/// A `TreePackage` represents a collection of related source files.
#[derive(Debug, Clone)]
pub struct TreePackage {
    /// The root directory for the package.
    pub root_dir: PathBuf,
    /// The primary programming language detected for the package.
    pub language: ProgrammingLanguage,
    /// Cached module list for the package.
    modules: Option<Vec<String>>,
    /// Source files matching the package language.
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

        let language = match config.language {
            Some(language) => language,
            None => detect_primary_language(&root_dir)?,
        };

        let source_files = collect_files(&root_dir, language.extensions(), &config.ignores)?;
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

fn find_git_root(start: &Path) -> Result<PathBuf, TreeHuggerError> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }

    Err(TreeHuggerError::GitRootNotFound {
        path: start.to_path_buf(),
    })
}

fn find_package_root(start: &Path, git_root: &Path) -> PathBuf {
    for ancestor in start.ancestors() {
        if ancestor == git_root {
            break;
        }

        if has_manifest(ancestor) {
            return ancestor.to_path_buf();
        }
    }

    if has_manifest(git_root) {
        return git_root.to_path_buf();
    }

    git_root.to_path_buf()
}

fn has_manifest(path: &Path) -> bool {
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

    MANIFESTS
        .iter()
        .any(|manifest| path.join(manifest).is_file())
}

fn detect_primary_language(root: &Path) -> Result<ProgrammingLanguage, TreeHuggerError> {
    let mut counts: HashMap<ProgrammingLanguage, usize> = HashMap::new();

    for entry in WalkBuilder::new(root).standard_filters(true).build() {
        let entry = entry.map_err(TreeHuggerError::Ignore)?;
        if !entry
            .file_type()
            .map(|file| file.is_file())
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(language) = ProgrammingLanguage::from_path(&entry.path()) {
            *counts.entry(language).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(language, _)| language)
        .ok_or_else(|| TreeHuggerError::NoSourceFiles {
            path: root.to_path_buf(),
        })
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
