//! The displayed prefix of a qualified name, applied after analysis and cache
//! lookup: a naming-context module path, or the file's portable path plus
//! `#`. Neither enters a stored record, a cache key or a symbol ID.
//!
//! `docs/symbol-kinds-and-visibility.md#qualified-names` is the user-facing
//! description.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use biscuit_file::{
    PathIdentity, RelativeRoute, canonicalize_simplified, to_portable_string, try_portable_string,
};

use crate::shared::ProgrammingLanguage;

/// A known module or package root and the name prefix it maps to.
///
/// A file below `root` is named by its route from `root`, one segment per
/// directory plus the file stem, joined with the language's separator after
/// `prefix`: with root `src/` and prefix `crate`, Rust's
/// `src/shared/symbol.rs` is `crate::shared::symbol::`. A Rust `mod.rs`, a
/// Rust `lib.rs` or `main.rs` directly in `root`, and a Python `__init__.py`
/// add no segment of their own. A file outside `root` falls back to the
/// file-path prefix.
///
/// ## Examples
///
/// ```
/// use tree_hugger::NamingContext;
///
/// let context = NamingContext::new("src", "crate");
/// assert_eq!(context.prefix(), "crate");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingContext {
    root: PathBuf,
    prefix: String,
}

impl NamingContext {
    pub fn new(root: impl Into<PathBuf>, prefix: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            prefix: prefix.into(),
        }
    }

    #[allow(missing_docs)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[allow(missing_docs)]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// The module path of `file`, ending in the separator, or `None` when
    /// `file` is not below the root.
    fn module_prefix(&self, file: &Path, language: ProgrammingLanguage) -> Option<String> {
        let root = crate::file_identity(&self.root);
        let route = PathIdentity::new(file).relative_from(&PathIdentity::new(&root))?;
        if route.parent_hops() > 0 || route.forward().is_empty() {
            return None;
        }
        let names: Vec<String> = route
            .forward()
            .iter()
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        let (file_name, directories) = names.split_last()?;
        let stem = Path::new(file_name)
            .file_stem()
            .map_or_else(|| file_name.clone(), |stem| stem.to_string_lossy().into_owned());
        let stem_adds_no_segment = match language {
            ProgrammingLanguage::Rust => {
                stem == "mod" || (directories.is_empty() && (stem == "lib" || stem == "main"))
            }
            ProgrammingLanguage::Python => stem == "__init__",
            _ => false,
        };
        let separator = module_separator(language);
        let segments = std::iter::once(self.prefix.as_str())
            .chain(directories.iter().map(String::as_str))
            .chain((!stem_adds_no_segment).then_some(stem.as_str()))
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        if segments.is_empty() {
            return Some(String::new());
        }
        Some(format!("{}{separator}", segments.join(separator)))
    }
}

/// Separator between module segments and the in-file name.
fn module_separator(language: ProgrammingLanguage) -> &'static str {
    match language {
        ProgrammingLanguage::Rust | ProgrammingLanguage::Cpp | ProgrammingLanguage::Perl => "::",
        ProgrammingLanguage::Php => "\\",
        _ => ".",
    }
}

/// The inputs of the displayed prefix, carried by a query.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Naming {
    pub(crate) scan_root: Option<PathBuf>,
    pub(crate) context: Option<NamingContext>,
}

impl Naming {
    /// The prefix a not namespace-rooted qualified name of `file` gets: the
    /// naming context's module path when `file` is below its root, else the
    /// portable path of `file` relative to the scan root (default: the file's
    /// own directory) plus `#`.
    pub(crate) fn prefix_for(&self, file: &Path, language: ProgrammingLanguage) -> String {
        if let Some(prefix) = self
            .context
            .as_ref()
            .and_then(|context| context.module_prefix(file, language))
        {
            return prefix;
        }
        let root = match &self.scan_root {
            Some(root) => Cow::Borrowed(root.as_path()),
            None => Cow::Owned(file.parent().map(Path::to_path_buf).unwrap_or_default()),
        };
        format!("{}#", portable_relative(file, &root))
    }
}

/// `path` relative to `root`, `/`-separated, for display and portable
/// reports: `src/lib.rs` beneath the root, `../vendor/x.rs` beside it.
///
/// Both are resolved through symlinks before comparing (so a temp directory
/// reached as `/var/…` and `/private/var/…` agrees), a relative `path` is
/// taken from the working directory, and a path with no relative route from
/// `root` (another Windows drive or share) keeps its absolute spelling,
/// `/`-separated.
pub fn portable_relative(path: &Path, root: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    // A `..` is only collapsed after resolution: lexically it would step out
    // of a symlinked directory the wrong way.
    let has_parent_hop = absolute.components().any(|part| part == Component::ParentDir);
    if !has_parent_hop
        && let Some(route) = PathIdentity::new(&absolute).relative_from(&PathIdentity::new(root))
        && route.parent_hops() == 0
    {
        return route_text(&route);
    }
    let resolved_root = canonicalize_simplified(root).unwrap_or_else(|_| root.to_path_buf());
    let resolved = resolve_directory(&absolute);
    match PathIdentity::new(&resolved).relative_from(&PathIdentity::new(&resolved_root)) {
        Some(route) => route_text(&route),
        // Resolution turns a share into a verbatim `\\?\UNC\...` path; keep
        // the spelling the file was named by instead.
        None => try_portable_string(&resolved).unwrap_or_else(|| to_portable_string(&absolute)),
    }
}

/// `path` with its directory resolved through symlinks; the final name is
/// kept as spelled. Unchanged when the directory cannot be resolved.
fn resolve_directory(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => canonicalize_simplified(parent)
            .map(|parent| parent.join(name))
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

/// `route` `/`-separated. A literal `..` name, possible only under a Windows
/// `\\?\` prefix, would read as a hop; canonical paths never contain one.
fn route_text(route: &RelativeRoute) -> String {
    let hops = std::iter::repeat_n(Cow::Borrowed(".."), route.parent_hops());
    let names = route.forward().iter().map(|name| name.to_string_lossy());
    hops.chain(names).collect::<Vec<_>>().join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let base = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(base.join("repo/src")).expect("repo dir");
        std::fs::create_dir_all(base.join("vendor")).expect("vendor dir");
        (dir, base)
    }

    #[test]
    fn a_path_beneath_the_root_is_relative_to_it() {
        let (_dir, base) = layout();
        let root = base.join("repo");
        assert_eq!(portable_relative(&root.join("src").join("lib.rs"), &root), "src/lib.rs");
        assert_eq!(
            portable_relative(&root.join("src").join("..").join("src").join("lib.rs"), &root),
            "src/lib.rs",
            "a `..` is resolved, not kept"
        );
    }

    #[test]
    fn a_path_beside_the_root_climbs_out_of_it() {
        let (_dir, base) = layout();
        let root = base.join("repo");
        assert_eq!(portable_relative(&base.join("vendor").join("x.rs"), &root), "../vendor/x.rs");
        assert_eq!(portable_relative(&base.join("x.rs"), &root), "../x.rs");
        assert_eq!(
            portable_relative(&root.join("..").join("vendor").join("x.rs"), &root),
            "../vendor/x.rs"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_spellings_of_one_location_agree() {
        let (_dir, base) = layout();
        let root = base.join("repo");
        std::os::unix::fs::symlink(&base, base.join("alias")).expect("symlink");
        let alias = base.join("alias");
        assert_eq!(portable_relative(&alias.join("repo/src/lib.rs"), &root), "src/lib.rs");
        assert_eq!(portable_relative(&alias.join("vendor/x.rs"), &root), "../vendor/x.rs");
        assert_eq!(
            portable_relative(&base.join("vendor/x.rs"), &alias.join("repo")),
            "../vendor/x.rs",
            "a symlinked root resolves too"
        );
    }

    /// With no relative route between them the file keeps its absolute path,
    /// `/`-separated.
    #[cfg(windows)]
    #[test]
    fn another_drive_keeps_its_absolute_path() {
        let root = Path::new(r"C:\no-such-repo");
        assert_eq!(portable_relative(Path::new(r"D:\no-such-vendor\x.rs"), root), "D:/no-such-vendor/x.rs");
        assert_eq!(
            portable_relative(Path::new(r"c:\no-such-repo\src\lib.rs"), root),
            "src/lib.rs",
            "drive letters compare without case"
        );
    }

    #[test]
    fn the_file_prefix_is_relative_to_the_scan_root_or_the_files_directory() {
        let (_dir, base) = layout();
        let file = base.join("repo/src/symbol.rs");
        let rust = ProgrammingLanguage::Rust;
        assert_eq!(Naming::default().prefix_for(&file, rust), "symbol.rs#");
        let rooted = Naming { scan_root: Some(base.join("repo")), context: None };
        assert_eq!(rooted.prefix_for(&file, rust), "src/symbol.rs#");
        let beside = Naming { scan_root: Some(base.join("vendor")), context: None };
        assert_eq!(beside.prefix_for(&file, rust), "../repo/src/symbol.rs#");
    }

    #[test]
    fn a_naming_context_maps_the_route_to_a_module_path() {
        let (_dir, base) = layout();
        let src = base.join("repo/src");
        let crate_root = NamingContext::new(&src, "crate");
        let with = |context: &NamingContext| Naming { scan_root: Some(base.clone()), context: Some(context.clone()) };
        let rust = ProgrammingLanguage::Rust;
        let python = ProgrammingLanguage::Python;
        let cases = [
            (&crate_root, src.join("shared/symbol.rs"), rust, "crate::shared::symbol::"),
            (&crate_root, src.join("shared/mod.rs"), rust, "crate::shared::"),
            (&crate_root, src.join("lib.rs"), rust, "crate::"),
            (&crate_root, src.join("main.rs"), rust, "crate::"),
            // `lib` below the root is an ordinary module.
            (&crate_root, src.join("nested/lib.rs"), rust, "crate::nested::lib::"),
        ];
        for (context, file, language, expected) in cases {
            assert_eq!(with(context).prefix_for(&file, language), expected, "{}", file.display());
        }
        let package_root = NamingContext::new(&src, "");
        assert_eq!(with(&package_root).prefix_for(&src.join("package/module.py"), python), "package.module.");
        assert_eq!(with(&package_root).prefix_for(&src.join("package/__init__.py"), python), "package.");
        assert_eq!(with(&package_root).prefix_for(&src.join("__init__.py"), python), "");
        // Outside the context root the file prefix applies.
        let vendor = base.join("vendor/x.rs");
        assert_eq!(with(&crate_root).prefix_for(&vendor, rust), "vendor/x.rs#");
    }
}
