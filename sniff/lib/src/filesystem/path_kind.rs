//! Path classification utilities for changed files.
//!
//! [`classify_path`] is the canonical, path-only classifier: it assigns every
//! path exactly one [`ChangeCategory`]. The source-code and documentation
//! predicates are wrappers over it, so the blast-radius analyzer, git commit
//! reporting, and downstream consumers cannot drift apart. Keeping this in a
//! dedicated leaf module avoids a layering inversion where `git` depends on
//! `blast_radius`.

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::filesystem::FileAssociation;
use crate::filesystem::file_types::{
    lookup_basename_pattern, lookup_exact_filename, lookup_extension,
};

/// The single category a changed path belongs to.
///
/// Categories are mutually exclusive. See [`classify_path`] for the precedence
/// that decides between overlapping signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeCategory {
    SourceCode,
    /// HTML, stylesheets, and fonts.
    WebAssets,
    /// Raster and vector images, including SVG.
    Images,
    Documentation,
    Configuration,
    /// CI/CD pipeline definitions.
    Cicd,
    Other,
}

impl ChangeCategory {
    /// Every category, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::SourceCode,
        Self::WebAssets,
        Self::Images,
        Self::Documentation,
        Self::Configuration,
        Self::Cicd,
        Self::Other,
    ];

    /// The serialized `snake_case` name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceCode => "source_code",
            Self::WebAssets => "web_assets",
            Self::Images => "images",
            Self::Documentation => "documentation",
            Self::Configuration => "configuration",
            Self::Cicd => "cicd",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for ChangeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Directory sequences whose descendants are CI/CD definitions. Matched as
/// consecutive path components anywhere in the path, so absolute worktree paths
/// classify the same as repository-relative ones.
const CICD_DIRECTORIES: &[&[&str]] = &[
    &[".github", "workflows"],
    &[".gitea", "workflows"],
    &[".forgejo", "workflows"],
    &[".circleci"],
    &[".buildkite"],
    &[".woodpecker"],
];

/// File names that are CI/CD definitions wherever they appear.
const CICD_FILE_NAMES: &[&str] = &[
    ".gitlab-ci.yml",
    ".gitlab-ci.yaml",
    "jenkinsfile",
    "azure-pipelines.yml",
    "azure-pipelines.yaml",
    ".drone.yml",
    ".drone.yaml",
    "buildspec.yml",
    "buildspec.yaml",
    ".travis.yml",
    "bitbucket-pipelines.yml",
    ".woodpecker.yml",
    ".woodpecker.yaml",
    "appveyor.yml",
    ".appveyor.yml",
    "cloudbuild.yml",
    "cloudbuild.yaml",
];

/// Classify a path into exactly one [`ChangeCategory`].
///
/// Only the path is inspected; the file is never opened. Precedence, first
/// match wins:
///
/// 1. CI/CD path rules (for example `.github/workflows/**`, `.gitlab-ci.yml`,
///    `Jenkinsfile`, `.circleci/**`).
/// 2. File-type registry: exact file name, then framework basename pattern,
///    then extension. `ProgrammingLanguage`/`FrameworkFile` are source code;
///    `Styling`, `Font`, and `.html`/`.htm` are web assets; `Image` (including
///    SVG) is images; `Documentation` and `Configuration` map to themselves.
///    An Angular `.component.html` template is a framework file and therefore
///    source code.
/// 3. Anything else, including paths without a UTF-8 file name, is
///    [`ChangeCategory::Other`].
///
/// ## Examples
///
/// ```
/// use std::path::Path;
/// use sniff::filesystem::path_kind::{ChangeCategory, classify_path};
///
/// assert_eq!(classify_path(Path::new(".github/workflows/ci.yml")), ChangeCategory::Cicd);
/// assert_eq!(classify_path(Path::new("site/index.html")), ChangeCategory::WebAssets);
/// assert_eq!(classify_path(Path::new("app/app.component.html")), ChangeCategory::SourceCode);
/// ```
pub fn classify_path(path: &Path) -> ChangeCategory {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return ChangeCategory::Other;
    };

    if is_cicd_path(path, file_name) {
        return ChangeCategory::Cicd;
    }

    if let Some(descriptor) = lookup_exact_filename(file_name) {
        return category_for_association(descriptor.association);
    }
    if let Some(descriptor) = lookup_basename_pattern(file_name) {
        return category_for_association(descriptor.association);
    }

    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return ChangeCategory::Other;
    };
    // The registry files HTML under `Documentation` for inventory purposes;
    // change reporting deliberately treats it as a web asset.
    if extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm") {
        return ChangeCategory::WebAssets;
    }
    lookup_extension(extension)
        .map(|descriptor| category_for_association(descriptor.association))
        .unwrap_or(ChangeCategory::Other)
}

fn is_cicd_path(path: &Path, file_name: &str) -> bool {
    if CICD_FILE_NAMES
        .iter()
        .any(|name| file_name.eq_ignore_ascii_case(name))
    {
        return true;
    }

    let directories: Vec<&str> = path
        .parent()
        .map(|parent| {
            parent
                .components()
                .filter_map(|component| match component {
                    Component::Normal(name) => name.to_str(),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    CICD_DIRECTORIES.iter().any(|rule| {
        directories.windows(rule.len()).any(|window| {
            window
                .iter()
                .zip(rule.iter())
                .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
        })
    })
}

fn category_for_association(association: FileAssociation) -> ChangeCategory {
    match association {
        FileAssociation::ProgrammingLanguage | FileAssociation::FrameworkFile => {
            ChangeCategory::SourceCode
        }
        FileAssociation::Styling | FileAssociation::Font => ChangeCategory::WebAssets,
        FileAssociation::Image => ChangeCategory::Images,
        FileAssociation::Documentation => ChangeCategory::Documentation,
        FileAssociation::Configuration => ChangeCategory::Configuration,
        FileAssociation::Data
        | FileAssociation::Binary
        | FileAssociation::BinaryExecutable
        | FileAssociation::Archive
        | FileAssociation::Audio
        | FileAssociation::Video
        | FileAssociation::Unknown => ChangeCategory::Other,
    }
}

/// Returns `true` if [`classify_path`] places the path in
/// [`ChangeCategory::SourceCode`].
///
/// Stylesheets and `.html`/`.htm` files are web assets, not source code;
/// Angular `.component.html` templates remain source code.
pub fn is_source_code_path(path: &Path) -> bool {
    classify_path(path) == ChangeCategory::SourceCode
}

/// Returns `true` if [`classify_path`] places the path in
/// [`ChangeCategory::Documentation`].
///
/// Bare `README`, `CHANGELOG`, and `CONTRIBUTING` count as documentation.
/// `.html`/`.htm` files are web assets, and a registry file name such as
/// `requirements.txt` keeps its configuration category despite its extension.
pub fn is_documentation_path(path: &Path) -> bool {
    classify_path(path) == ChangeCategory::Documentation
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
        fn css_file_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("styles/main.css")));
        }

        #[test]
        fn html_file_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("public/index.html")));
        }

        #[test]
        fn htm_file_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("public/page.htm")));
        }

        #[test]
        fn angular_component_template_is_source_code() {
            assert!(is_source_code_path(&PathBuf::from(
                "src/app/app.component.html"
            )));
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
        fn scss_is_not_source_code() {
            assert!(!is_source_code_path(&PathBuf::from("styles/theme.scss")));
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
        fn html_file_is_not_documentation() {
            assert!(!is_documentation_path(&PathBuf::from("docs/index.html")));
        }

        #[test]
        fn requirements_txt_keeps_its_configuration_file_name_category() {
            assert!(!is_documentation_path(&PathBuf::from("requirements.txt")));
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

    mod classification {
        use super::*;

        fn category(path: &str) -> ChangeCategory {
            classify_path(Path::new(path))
        }

        #[test]
        fn cicd_directory_rules_win_over_the_registry() {
            for path in [
                ".github/workflows/ci.yml",
                ".github/workflows/release.yaml",
                ".github/workflows/nested/scripts/check.sh",
                ".gitea/workflows/build.yml",
                ".forgejo/workflows/build.yml",
                ".circleci/config.yml",
                ".buildkite/pipeline.yml",
                ".woodpecker/test.yml",
                "services/api/.github/workflows/ci.yml",
            ] {
                assert_eq!(category(path), ChangeCategory::Cicd, "{path}");
            }
        }

        #[test]
        fn cicd_file_name_rules_match_at_any_depth_and_case() {
            for path in [
                ".gitlab-ci.yml",
                "Jenkinsfile",
                "ci/JENKINSFILE",
                "azure-pipelines.yml",
                ".drone.yml",
                "buildspec.yml",
                ".travis.yml",
                "bitbucket-pipelines.yml",
                "appveyor.yml",
                "deploy/cloudbuild.yaml",
            ] {
                assert_eq!(category(path), ChangeCategory::Cicd, "{path}");
            }
        }

        #[test]
        fn github_files_outside_workflows_are_not_cicd() {
            assert_eq!(category(".github/CODEOWNERS"), ChangeCategory::Other);
            assert_eq!(
                category(".github/dependabot.yml"),
                ChangeCategory::Configuration
            );
            assert_eq!(
                category(".github/actions/setup/index.js"),
                ChangeCategory::SourceCode
            );
            assert_eq!(category("workflows/ci.yml"), ChangeCategory::Configuration);
        }

        #[cfg(windows)]
        #[test]
        fn cicd_directory_rule_matches_native_windows_separators() {
            assert_eq!(
                category(r"C:\repo\.github\workflows\ci.yml"),
                ChangeCategory::Cicd
            );
        }

        #[test]
        fn cicd_directory_rule_matches_absolute_paths() {
            let absolute = std::env::temp_dir()
                .join("repo")
                .join(".github")
                .join("workflows")
                .join("ci.yml");
            assert_eq!(classify_path(&absolute), ChangeCategory::Cicd);
        }

        #[test]
        fn web_assets_cover_html_stylesheets_and_fonts() {
            for path in [
                "index.html",
                "public/page.HTM",
                "styles/main.css",
                "styles/theme.scss",
                "styles/theme.sass",
                "styles/theme.less",
                "styles/theme.pcss",
                "fonts/Inter.woff2",
                "fonts/Inter.woff",
                "fonts/Inter.ttf",
                "fonts/Inter.otf",
            ] {
                assert_eq!(category(path), ChangeCategory::WebAssets, "{path}");
            }
        }

        #[test]
        fn images_include_svg() {
            for path in ["logo.svg", "logo.png", "photo.JPG", "icon.ico", "hero.avif"] {
                assert_eq!(category(path), ChangeCategory::Images, "{path}");
            }
        }

        #[test]
        fn angular_component_template_is_source_code_not_web_asset() {
            assert_eq!(
                category("src/app/app.component.html"),
                ChangeCategory::SourceCode
            );
            assert_eq!(
                category("src/app/app.component.htm"),
                ChangeCategory::SourceCode
            );
        }

        #[test]
        fn exact_file_names_take_precedence_over_extensions() {
            assert_eq!(
                category("requirements.txt"),
                ChangeCategory::Configuration
            );
            assert_eq!(category("Cargo.lock"), ChangeCategory::Configuration);
            assert_eq!(category("CHANGELOG"), ChangeCategory::Documentation);
            assert_eq!(category("Dockerfile"), ChangeCategory::Configuration);
        }

        #[test]
        fn unmatched_and_data_paths_are_other() {
            for path in [
                "data/export.csv",
                "schema.sql",
                "bundle.zip",
                "docs/manual.pdf",
                "clip.mp4",
                "LICENSE",
                "file.unknownext",
                "",
                "..",
            ] {
                assert_eq!(category(path), ChangeCategory::Other, "{path:?}");
            }
        }

        #[cfg(unix)]
        #[test]
        fn non_utf8_file_name_is_other() {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;

            let path = Path::new(OsStr::from_bytes(b"src/\xFFbad.rs"));
            assert_eq!(classify_path(path), ChangeCategory::Other);
        }

        #[test]
        fn serialized_names_match_as_str() {
            for category in ChangeCategory::ALL {
                assert_eq!(
                    serde_json::to_value(category).unwrap(),
                    serde_json::Value::String(category.as_str().to_string())
                );
                let round_trip: ChangeCategory =
                    serde_json::from_value(serde_json::to_value(category).unwrap()).unwrap();
                assert_eq!(round_trip, category);
            }
        }

        /// Every extension and exact file name the registry knows, so a new
        /// registry association that lacks a category decision fails here.
        const REGISTRY_EXTENSIONS: &[&str] = &[
            "rs", "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "py", "pyw", "go", "sh",
            "bash", "zsh", "fish", "ps1", "psm1", "psd1", "bat", "cmd", "rb", "php", "lua",
            "swift", "zig", "c", "h", "cc", "cpp", "cxx", "hpp", "hh", "hxx", "java", "kt",
            "kts", "scala", "sc", "clj", "cljs", "cljc", "edn", "cs", "fs", "fsi", "fsx",
            "jsonnet", "libsonnet", "wat", "wast", "vue", "svelte", "astro", "html", "htm",
            "css", "scss", "sass", "less", "pcss", "json", "json5", "jsonl", "toml", "yaml",
            "yml", "ini", "cfg", "conf", "lock", "md", "mdx", "txt", "rst", "adoc", "org", "tex",
            "csv", "tsv", "xml", "graphql", "gql", "sql", "png", "jpg", "jpeg", "gif", "svg",
            "webp", "avif", "ico", "bmp", "tif", "tiff", "zip", "tar", "gz", "bz2", "xz", "7z",
            "rar", "tgz", "ttf", "otf", "woff", "woff2", "mp3", "wav", "flac", "ogg", "m4a",
            "aac", "opus", "mp4", "mov", "mkv", "webm", "avi", "pdf", "so", "dylib", "dll", "a",
            "o", "wasm", "exe",
        ];

        const REGISTRY_FILE_NAMES: &[&str] = &[
            ".editorconfig", ".gitignore", ".gitattributes", ".gitmodules", ".env", ".env.local",
            "Cargo.toml", "Cargo.lock", "package.json", "package-lock.json", "pnpm-lock.yaml",
            "pnpm-workspace.yaml", "yarn.lock", "bun.lock", "bun.lockb", "go.mod", "go.sum",
            "pyproject.toml", "requirements.txt", "Pipfile", "Pipfile.lock", "Dockerfile",
            "Containerfile", "README", "README.md", "CHANGELOG", "CHANGELOG.md", "CONTRIBUTING",
            "CONTRIBUTING.md", "justfile", "Makefile", "Rakefile", "Brewfile", "Procfile",
        ];

        fn expected_from_registry(file_name: &str) -> ChangeCategory {
            let association = lookup_exact_filename(file_name)
                .or_else(|| {
                    Path::new(file_name)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .and_then(lookup_extension)
                })
                .map(|descriptor| descriptor.association)
                .unwrap_or_else(|| panic!("{file_name} is not in the registry corpus"));
            let lower = file_name.to_ascii_lowercase();
            if lookup_exact_filename(file_name).is_none()
                && (lower.ends_with(".html") || lower.ends_with(".htm"))
            {
                return ChangeCategory::WebAssets;
            }
            category_for_association(association)
        }

        #[test]
        fn registry_corpus_classifies_consistently_with_registry_and_wrappers() {
            let corpus = REGISTRY_EXTENSIONS
                .iter()
                .map(|ext| format!("dir/file.{ext}"))
                .chain(REGISTRY_FILE_NAMES.iter().map(|name| format!("dir/{name}")));

            for path in corpus {
                let path = Path::new(&path);
                let file_name = path.file_name().unwrap().to_str().unwrap();
                let actual = classify_path(path);
                assert_eq!(actual, expected_from_registry(file_name), "{path:?}");
                assert_eq!(
                    is_source_code_path(path),
                    actual == ChangeCategory::SourceCode,
                    "{path:?}"
                );
                assert_eq!(
                    is_documentation_path(path),
                    actual == ChangeCategory::Documentation,
                    "{path:?}"
                );
            }
        }

        #[test]
        fn registry_corpus_covers_every_file_association() {
            let associations: std::collections::BTreeSet<&str> = REGISTRY_EXTENSIONS
                .iter()
                .filter_map(|ext| lookup_extension(ext))
                .chain(
                    REGISTRY_FILE_NAMES
                        .iter()
                        .filter_map(|name| lookup_exact_filename(name)),
                )
                .map(|descriptor| descriptor.association.as_str())
                .collect();
            for ext in REGISTRY_EXTENSIONS {
                assert!(lookup_extension(ext).is_some(), "stale corpus extension {ext}");
            }
            for name in REGISTRY_FILE_NAMES {
                assert!(
                    lookup_exact_filename(name).is_some(),
                    "stale corpus file name {name}"
                );
            }
            // Every association except Unknown (never produced by a lookup).
            assert_eq!(associations.len(), 13, "{associations:?}");
        }
    }
}
