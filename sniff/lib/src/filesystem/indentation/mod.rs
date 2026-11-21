//! Which indentation a repository wants for a file, and where that answer
//! came from.
//!
//! Sniff reads the worktree root's `.editorconfig` and the supported
//! formatter configurations as data. Each applicable source is an
//! **opinion**. They rank EditorConfig first, then formatters in table order,
//! then a built-in language convention. Each resolved value comes from the
//! highest-ranked opinion that states it, and four spaces fill whatever no
//! opinion states. The result is the value to hand biscuit-file's YAML
//! writer ([`ResolvedIndentation::indentation`]).
//!
//! The free functions discover the worktree and read only what one target
//! needs. [`RepoIndentation`] reads every supported file once and answers
//! many targets from that snapshot.
//!
//! See `sniff/docs/topics/indentation.md`.

mod catalog;
mod editorconfig;
mod formatters;
mod inputs;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::num::NonZeroU8;
use std::path::{Component, Path, PathBuf};

use biscuit_file::{PathIdentity, canonicalize_simplified};
use serde::{Deserialize, Serialize};

pub use biscuit_file::{IndentStyle, Indentation};

use crate::{Result, SniffError};
use catalog::{CONVENTIONS, ROWS, YAML_EXTENSIONS};
use inputs::Inputs;

/// The kind of source an opinion came from.
///
/// Serialized in kebab-case: `editorconfig`, `clang-format`, …
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndentAuthority {
    #[serde(rename = "editorconfig")]
    EditorConfig,
    Rustfmt,
    Prettier,
    Biome,
    Deno,
    Dprint,
    Oxfmt,
    Ruff,
    Tombi,
    Taplo,
    Yamlfmt,
    Stylua,
    ClangFormat,
    SwiftFormat,
    Rubocop,
    Csharpier,
    /// A built-in language convention; `via` names it (`gofmt`).
    Convention,
    /// Sniff's fallback; used as the authority only when no opinion exists.
    Default,
}

/// One source's view of the indentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndentOpinion {
    pub source: IndentAuthority,
    /// Worktree-relative, serialized with `/`. Absent for `Convention` and
    /// `Default`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "portable_path_option")]
    pub path: Option<PathBuf>,
    /// EditorConfig sections, formatter keys read, or the convention name.
    /// Empty for a formatter whose defaults supplied every value.
    pub via: Vec<String>,
    /// Absent only on an EditorConfig opinion that does not state it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent_using: Option<IndentStyle>,
    /// Columns per level. Absent only on an EditorConfig opinion that does
    /// not state it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<NonZeroU8>,
}

/// The indentation to use for one target, with its provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedIndentation {
    pub indent_using: IndentStyle,
    /// Columns per indentation level.
    pub indent: NonZeroU8,
    /// The highest-ranked opinion, or the `default` fallback when there is
    /// none.
    pub authority: IndentOpinion,
    /// Remaining opinions in rank order. The fallback never appears here.
    pub alt_authorities: Vec<IndentOpinion>,
    /// Every opinion agrees on each value it states. Computed before
    /// [`adjustment`](Self::adjustment).
    pub consensus: bool,
    /// Present when the target's format overrode the ranked style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment: Option<IndentAdjustment>,
    /// Configuration Sniff could not use, or used only in part.
    pub skipped: Vec<IndentationDiagnostic>,
}

impl ResolvedIndentation {
    /// The value to pass to biscuit-file's writer.
    #[must_use]
    pub fn indentation(&self) -> Indentation {
        Indentation::new(self.indent_using, self.indent)
    }
}

/// A format constraint applied after ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndentAdjustment {
    /// The style ranking selected before the format constraint applied.
    pub requested: IndentStyle,
    pub reason: AdjustmentReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentReason {
    /// YAML forbids tab indentation, so tabs become spaces at the same width.
    YamlForbidsTabs,
}

/// A configuration file, or one setting in it, that Sniff skipped.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndentationDiagnostic {
    /// Worktree-relative, serialized with `/`.
    #[serde(with = "portable_path")]
    pub path: PathBuf,
    /// The setting, as a dotted key path; absent when the whole file is
    /// skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub reason: String,
}

/// Every file type's indentation in one worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndentationSummary {
    /// The generic EditorConfig rules (`[*]`) and the fallback. Can differ
    /// from four spaces.
    pub default: ResolvedIndentation,
    /// Extensions whose style or width differs from `default`, by extension.
    pub overrides: BTreeMap<String, ResolvedIndentation>,
    /// Diagnostics from every query, including extensions that match
    /// `default`, deduplicated by path, key, and reason in first-seen order.
    pub skipped: Vec<IndentationDiagnostic>,
}

/// Validates a file extension and strips one leading dot.
///
/// After the optional dot, the extension starts with an ASCII letter or
/// digit; the rest are ASCII letters, digits, `_`, `+`, or `-`.
///
/// ## Errors
///
/// [`SniffError::InvalidExtension`] for any other input.
///
/// ## Examples
///
/// ```
/// use sniff::filesystem::indentation::normalize_extension;
///
/// assert_eq!(normalize_extension(".yaml").unwrap(), "yaml");
/// assert!(normalize_extension("..yaml").is_err());
/// ```
pub fn normalize_extension(input: &str) -> Result<&str> {
    let extension = input.strip_prefix('.').unwrap_or(input);
    let invalid = |reason| SniffError::InvalidExtension { input: input.to_string(), reason };
    let mut chars = extension.chars();
    match chars.next() {
        None => return Err(invalid("it is empty")),
        Some(first) if !first.is_ascii_alphanumeric() => {
            return Err(invalid("it must start with an ASCII letter or digit"));
        }
        Some(_) => {}
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-')) {
        return Err(invalid("only ASCII letters, digits, `_`, `+`, and `-` may follow the first character"));
    }
    Ok(extension)
}

/// Resolves indentation for the file at `path`, which need not exist.
///
/// Finds the worktree from the nearest existing ancestor and matches the
/// full destination, relative to the worktree root. Reads only the inputs
/// this file's type needs. A path outside a worktree resolves to its
/// language convention, if any, then the fallback.
///
/// ## Errors
///
/// [`SniffError::IndentationTarget`] when the destination cannot be
/// identified (it names no file, or an existing ancestor is not a
/// directory); Git discovery and canonicalization failures. Unreadable or
/// malformed configuration is never an error: it is reported in
/// [`ResolvedIndentation::skipped`].
pub fn resolve_for_path(path: &Path) -> Result<ResolvedIndentation> {
    let destination = Destination::new(path)?;
    let Some(root) = discover_root(&destination.directory)? else {
        return Ok(resolve(None, &Target::outside(&destination.basename)));
    };
    let relative = destination.relative_to(&root, path)?;
    let target = Target::path(&relative);
    let inputs = Inputs::load(&root, inputs::Selection::Target(target.file_type.as_deref()));
    Ok(resolve(Some(&inputs), &target))
}

/// Resolves indentation for a file extension (with or without the leading
/// dot) as if for a file at the root of the worktree containing `base`.
///
/// Only rules that apply to every file of that type take part: EditorConfig
/// `[*]` and suffix sections such as `[*.{yaml,yml}]`, and Prettier or
/// oxfmt overrides whose patterns depend on nothing but the extension.
/// Outside a worktree, the language convention or fallback applies.
///
/// ## Errors
///
/// [`SniffError::InvalidExtension`] (see [`normalize_extension`]); Git
/// discovery and canonicalization failures.
pub fn resolve_for_extension(base: &Path, ext: &str) -> Result<ResolvedIndentation> {
    let extension = normalize_extension(ext)?;
    let target = Target::extension(extension);
    let Some(root) = discover_root(base)? else {
        return Ok(resolve(None, &target));
    };
    let inputs = Inputs::load(&root, inputs::Selection::Target(target.file_type.as_deref()));
    Ok(resolve(Some(&inputs), &target))
}

/// Resolves the summary for the worktree containing `base`.
///
/// Outside a worktree, `default` is the fallback and `overrides` is empty.
///
/// ## Errors
///
/// Git discovery and canonicalization failures.
pub fn resolve_summary(base: &Path) -> Result<IndentationSummary> {
    match discover_root(base)? {
        Some(root) => Ok(RepoIndentation::load(&root)?.summary()),
        None => Ok(IndentationSummary {
            default: resolve(None, &Target::generic()),
            overrides: BTreeMap::new(),
            skipped: Vec::new(),
        }),
    }
}

/// Every supported configuration in one worktree, read once.
///
/// A snapshot answers any number of targets without reading again. It does
/// not detect nested repositories: load a separate snapshot for one, or use
/// the free functions, which discover the worktree per call.
#[derive(Debug)]
pub struct RepoIndentation {
    root: PathBuf,
    inputs: Inputs,
}

impl RepoIndentation {
    /// Reads every supported configuration at `repo_root`, which is taken as
    /// the worktree root without discovery.
    ///
    /// ## Errors
    ///
    /// [`SniffError::IndentationTarget`] when `repo_root` is not a directory;
    /// canonicalization failures.
    pub fn load(repo_root: &Path) -> Result<Self> {
        if !repo_root.is_dir() {
            return Err(SniffError::IndentationTarget {
                path: repo_root.to_path_buf(),
                reason: "the worktree root is not a directory".to_string(),
            });
        }
        let root = canonicalize_simplified(repo_root)?;
        let inputs = Inputs::load(&root, inputs::Selection::All);
        Ok(Self { root, inputs })
    }

    /// The canonical worktree root this snapshot read.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves an absolute path inside this snapshot's worktree, which need
    /// not exist.
    ///
    /// ## Errors
    ///
    /// [`SniffError::IndentationTarget`] for a relative path, one that names
    /// no file, or one outside the worktree.
    pub fn resolve_for_path(&self, path: &Path) -> Result<ResolvedIndentation> {
        if !path.is_absolute() {
            return Err(SniffError::IndentationTarget {
                path: path.to_path_buf(),
                reason: "a snapshot resolves absolute paths only".to_string(),
            });
        }
        let destination = Destination::new(path)?;
        let relative = destination.relative_to(&self.root, path)?;
        Ok(resolve(Some(&self.inputs), &Target::path(&relative)))
    }

    /// Resolves an extension already accepted by [`normalize_extension`].
    ///
    /// An input that fails that grammar is not rejected here: it matches no
    /// extension-specific rule.
    #[must_use]
    pub fn resolve_for_extension(&self, ext: &str) -> ResolvedIndentation {
        let extension = ext.strip_prefix('.').unwrap_or(ext);
        resolve(Some(&self.inputs), &Target::extension(extension))
    }

    /// The generic default plus every extension whose result differs.
    ///
    /// Extensions come from the formatter and convention tables and from
    /// EditorConfig sections that name literal suffixes (`[*.rs]`,
    /// `[*.{yaml,yml}]`), sorted and deduplicated.
    #[must_use]
    pub fn summary(&self) -> IndentationSummary {
        let default = resolve(Some(&self.inputs), &Target::generic());
        let mut skipped = Vec::new();
        push_unique(&mut skipped, &default.skipped);
        let mut overrides = BTreeMap::new();
        for extension in self.summary_extensions() {
            let resolved = self.resolve_for_extension(&extension);
            push_unique(&mut skipped, &resolved.skipped);
            if resolved.indent_using != default.indent_using || resolved.indent != default.indent {
                overrides.insert(extension, resolved);
            }
        }
        IndentationSummary { default, overrides, skipped }
    }

    fn summary_extensions(&self) -> Vec<String> {
        let mut extensions: Vec<String> = ROWS
            .iter()
            .flat_map(|row| row.extensions.iter())
            .chain(CONVENTIONS.iter().flat_map(|convention| convention.extensions.iter()))
            .map(|extension| (*extension).to_string())
            .collect();
        extensions.extend(editorconfig::literal_suffixes(self.inputs.editorconfig()));
        extensions.sort();
        extensions.dedup();
        extensions
    }
}

fn push_unique(into: &mut Vec<IndentationDiagnostic>, from: &[IndentationDiagnostic]) {
    for diagnostic in from {
        if !into.contains(diagnostic) {
            into.push(diagnostic.clone());
        }
    }
}

fn discover_root(base: &Path) -> Result<Option<PathBuf>> {
    let start = nearest_existing_directory(base);
    match crate::filesystem::git::repo_root(&start)? {
        Some(root) => Ok(Some(canonicalize_simplified(&root)?)),
        None => Ok(None),
    }
}

/// `path` itself when it is a directory, else its nearest existing
/// directory ancestor, else the path as given (discovery then finds no
/// repository).
fn nearest_existing_directory(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    absolute
        .ancestors()
        .find(|ancestor| ancestor.is_dir())
        .map_or(absolute.clone(), Path::to_path_buf)
}

/// A destination file split into its canonical existing directory and the
/// components below it that do not exist yet.
#[derive(Debug)]
struct Destination {
    /// The nearest existing directory ancestor, canonicalized.
    directory: PathBuf,
    /// Missing directory components between `directory` and the file.
    missing: Vec<OsString>,
    basename: String,
}

impl Destination {
    fn new(path: &Path) -> Result<Self> {
        let target_error = |reason: &str| SniffError::IndentationTarget {
            path: path.to_path_buf(),
            reason: reason.to_string(),
        };
        let absolute = std::path::absolute(path)?;
        let Some(name) = absolute.file_name() else {
            return Err(target_error("the path names no file"));
        };
        let basename = name.to_string_lossy().into_owned();
        let parent = absolute.parent().ok_or_else(|| target_error("the path names no file"))?;

        // Walk up to the nearest existing ancestor, remembering what was
        // missing below it. The file itself is never canonicalized, so a
        // symlink at the destination is matched by its own name: a caller
        // that writes through a symlink passes the path it will write.
        let mut existing = parent;
        let mut pending: Vec<Component<'_>> = Vec::new();
        loop {
            match std::fs::metadata(existing) {
                Ok(metadata) if metadata.is_dir() => break,
                Ok(_) => return Err(target_error("an existing ancestor is not a directory")),
                Err(_) => {}
            }
            let (Some(up), Some(last)) = (existing.parent(), existing.components().next_back()) else {
                return Err(target_error("no ancestor of the path exists"));
            };
            pending.push(last);
            existing = up;
        }
        let mut directory = canonicalize_simplified(existing)?;
        let mut missing: Vec<OsString> = Vec::new();
        for component in pending.into_iter().rev() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if missing.pop().is_none() {
                        directory = directory.parent().map_or(directory.clone(), Path::to_path_buf);
                    }
                }
                Component::Normal(name) => missing.push(name.to_os_string()),
                Component::RootDir | Component::Prefix(_) => {}
            }
        }
        Ok(Self { directory, missing, basename })
    }

    /// The destination relative to `root`, joined with `/`.
    fn relative_to(&self, root: &Path, original: &Path) -> Result<String> {
        let directory = PathIdentity::new(&self.directory);
        let root_identity = PathIdentity::new(root);
        let Some(inside) = directory.strip_prefix(&root_identity) else {
            return Err(SniffError::IndentationTarget {
                path: original.to_path_buf(),
                reason: format!("the path is outside the worktree at {}", root.display()),
            });
        };
        let mut parts: Vec<String> = inside.iter().map(|part| part.to_string_lossy().into_owned()).collect();
        parts.extend(self.missing.iter().map(|part| part.to_string_lossy().into_owned()));
        parts.push(self.basename.clone());
        Ok(parts.join("/"))
    }
}

/// What a lookup matches against.
#[derive(Debug, Clone)]
pub(crate) enum TargetKind {
    /// A file, by its worktree-relative path with `/` separators.
    Path(String),
    /// Every file with this extension, as written by the caller.
    Extension(String),
    /// Every file: only the generic EditorConfig rules apply.
    Generic,
}

#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub(crate) kind: TargetKind,
    /// The lowercase extension that selects formatter rows and conventions.
    pub(crate) file_type: Option<String>,
    /// The real basename, for path lookups only.
    pub(crate) basename: Option<String>,
}

impl Target {
    fn path(relative: &str) -> Self {
        let basename = relative.rsplit('/').next().unwrap_or(relative).to_string();
        Self {
            file_type: file_type_of(&basename),
            basename: Some(basename),
            kind: TargetKind::Path(relative.to_string()),
        }
    }

    /// A file outside every worktree: only its language convention applies.
    fn outside(basename: &str) -> Self {
        Self {
            file_type: file_type_of(basename),
            basename: Some(basename.to_string()),
            kind: TargetKind::Path(basename.to_string()),
        }
    }

    fn extension(extension: &str) -> Self {
        Self {
            file_type: Some(extension.to_ascii_lowercase()),
            basename: None,
            kind: TargetKind::Extension(extension.to_string()),
        }
    }

    fn generic() -> Self {
        Self { kind: TargetKind::Generic, file_type: None, basename: None }
    }

    /// The path matched against path-dependent rules, or the synthetic root
    /// basename (`file.<ext>`) of an extension lookup.
    pub(crate) fn match_path(&self) -> String {
        match &self.kind {
            TargetKind::Path(relative) => relative.clone(),
            TargetKind::Extension(extension) => format!("file.{extension}"),
            TargetKind::Generic => "file".to_string(),
        }
    }
}

fn file_type_of(basename: &str) -> Option<String> {
    let (stem, extension) = basename.rsplit_once('.')?;
    (!stem.is_empty() && !extension.is_empty()).then(|| extension.to_ascii_lowercase())
}

/// Ordered diagnostics for one resolution.
#[derive(Debug, Default)]
pub(crate) struct Diagnostics(Vec<IndentationDiagnostic>);

impl Diagnostics {
    pub(crate) fn push(&mut self, path: &str, key: Option<&str>, reason: impl Into<String>) {
        let diagnostic = IndentationDiagnostic {
            path: PathBuf::from(path),
            key: key.map(str::to_string),
            reason: reason.into(),
        };
        if !self.0.contains(&diagnostic) {
            self.0.push(diagnostic);
        }
    }
}

/// The one resolution every entry point shares. `inputs` is `None` outside
/// a worktree.
fn resolve(inputs: Option<&Inputs>, target: &Target) -> ResolvedIndentation {
    let mut diagnostics = Diagnostics::default();
    let mut opinions: Vec<IndentOpinion> = Vec::new();
    let mut formatter_spoke = false;

    if let Some(inputs) = inputs {
        if let Some(opinion) = editorconfig::opinion(inputs, target, &mut diagnostics) {
            opinions.push(opinion);
        }
        if let Some(file_type) = target.file_type.as_deref() {
            for row in ROWS.iter().filter(|row| row.applies_to(file_type)) {
                if let Some(opinion) = formatters::row_opinion(row, inputs, target, &mut diagnostics) {
                    opinions.push(opinion);
                    formatter_spoke = true;
                }
            }
        }
    }
    if !formatter_spoke && let Some(opinion) = convention_opinion(target) {
        opinions.push(opinion);
    }

    let stated_style = opinions.iter().find_map(|opinion| opinion.indent_using);
    let stated_width = opinions.iter().find_map(|opinion| opinion.indent);
    let consensus = opinions.iter().all(|opinion| {
        opinion.indent_using.is_none_or(|style| Some(style) == stated_style)
            && opinion.indent.is_none_or(|width| Some(width) == stated_width)
    });
    let fallback = Indentation::default();
    let mut indent_using = stated_style.unwrap_or(fallback.style);
    let indent = stated_width.unwrap_or(fallback.width);

    let mut adjustment = None;
    let is_yaml = target.file_type.as_deref().is_some_and(|file_type| YAML_EXTENSIONS.contains(&file_type));
    if is_yaml && indent_using == IndentStyle::Tabs {
        adjustment = Some(IndentAdjustment { requested: IndentStyle::Tabs, reason: AdjustmentReason::YamlForbidsTabs });
        indent_using = IndentStyle::Spaces;
    }

    let mut ranked = opinions.into_iter();
    let authority = ranked.next().unwrap_or(IndentOpinion {
        source: IndentAuthority::Default,
        path: None,
        via: Vec::new(),
        indent_using: Some(fallback.style),
        indent: Some(fallback.width),
    });
    ResolvedIndentation {
        indent_using,
        indent,
        authority,
        alt_authorities: ranked.collect(),
        consensus,
        adjustment,
        skipped: diagnostics.0,
    }
}

fn convention_opinion(target: &Target) -> Option<IndentOpinion> {
    let file_type = target.file_type.as_deref();
    let basename = target.basename.as_deref();
    let lower_basename = basename.map(str::to_ascii_lowercase);
    let convention = CONVENTIONS.iter().find(|convention| {
        file_type.is_some_and(|file_type| convention.extensions.contains(&file_type))
            || basename.is_some_and(|basename| convention.basenames.contains(&basename))
            || lower_basename
                .as_deref()
                .is_some_and(|name| convention.suffixes.iter().any(|suffix| name.ends_with(suffix)))
    })?;
    Some(IndentOpinion {
        source: IndentAuthority::Convention,
        path: None,
        via: vec![convention.name.to_string()],
        indent_using: Some(convention.style),
        indent: NonZeroU8::new(convention.width),
    })
}

/// Serializes a relative path with `/` separators on every platform.
mod portable_path {
    use std::path::{Path, PathBuf};

    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn text(path: &Path) -> String {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    }

    pub(super) fn serialize<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&text(path))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
        String::deserialize(deserializer).map(PathBuf::from)
    }
}

mod portable_path_option {
    use std::path::PathBuf;

    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(path: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error> {
        match path {
            Some(path) => serializer.serialize_str(&super::portable_path::text(path)),
            None => serializer.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<PathBuf>, D::Error> {
        Option::<String>::deserialize(deserializer).map(|path| path.map(PathBuf::from))
    }
}

#[cfg(test)]
mod tests;
