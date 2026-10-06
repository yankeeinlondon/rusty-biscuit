//! Glob references: one or more glob patterns that use the file-reference
//! prefix grammar.
//!
//! [`FileReference`](crate::FileReference) names one file and never reads
//! glob syntax, so `pages/[id].md` is a literal name. A caller that wants a
//! set of files builds a [`GlobReference`] instead: each pattern is
//! `[!][reference-prefix]glob`, and its prefix maps to the same roots
//! single-file resolution searches. The user-facing contract is the
//! "Glob References" section of `biscuit-file/docs/topics/file-references.md`.

mod error;
mod list;
mod matches;
mod parse;
mod roots;

use std::path::{Path, PathBuf};

pub use error::GlobReferenceError;

use crate::file_reference::FileResolutionContext;
use crate::file_reference::context::ResolutionContext;
use crate::file_reference::{MagicPathList, ParsedReference, ReferenceKind};
use list::PreparedSet;
use parse::{GlobPattern, Piece, parse_pattern};
use roots::{PatternInput, authored_pieces, prepare};

/// One or more glob patterns sharing the file-reference prefix grammar.
///
/// ## Examples
///
/// ```rust,no_run
/// use std::collections::HashMap;
/// use biscuit_file::{FileResolutionContext, GlobReference};
///
/// let ctx = FileResolutionContext::from_snapshot("/repo/pkg", None, HashMap::new())
///     .with_repository_root("/repo");
/// let specs = GlobReference::new(["^**/*spec*.md", "!&**/_completed/**"])?;
/// for path in specs.list_files(&ctx)?.matches {
///     println!("{}", path.display());
/// }
/// # Ok::<(), biscuit_file::GlobReferenceError>(())
/// ```
///
/// ## Notes
///
/// Results come in **native order**: the roots of the prefix in precedence
/// order (most local first), then fewest path components below the root,
/// then component-wise. A file belongs to the first root that contains it
/// and is judged only relative to that root, so a later root never
/// re-includes a file an earlier root excluded.
///
/// Matching is case-sensitive on every OS, `*` and `?` do not cross `/`
/// (`**` does), and `\` never escapes: it is a literal character on Unix and
/// a separator on Windows. Escape literal text with
/// [`GlobReference::escape`]. No hidden, ignored, or underscore filter
/// applies.
#[derive(Debug, Clone)]
pub struct GlobReference {
    patterns: Vec<GlobPattern>,
    file_name_view: bool,
}

/// The result of [`GlobReference::list_files`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobListing {
    /// Every matched file, in native order, each once.
    pub matches: Vec<PathBuf>,
    /// File symlinks a bare, `./`, or `../` pattern matched whose target lies
    /// outside the context's file tree. They are not in `matches`.
    pub skipped: Vec<SkippedEntry>,
}

/// A matched file symlink left out of a listing because its target lies
/// outside the file tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedEntry {
    /// The symlink, spelled under its root.
    pub link: PathBuf,
    /// Where the symlink really leads.
    pub target: PathBuf,
}

impl GlobReference {
    /// Parse one or more patterns.
    ///
    /// ## Errors
    ///
    /// - [`GlobReferenceError::RejectedPrefix`] for a remote URL or `%`
    /// - [`GlobReferenceError::MalformedPrefix`] for text the reference
    ///   grammar rejects, an empty pattern, or a prefix with no glob after it
    /// - [`GlobReferenceError::InvalidGlob`] for invalid glob syntax
    /// - [`GlobReferenceError::NoPositivePattern`] when no pattern is
    ///   positive (an empty list, or only `!` exclusions)
    pub fn new<I, S>(patterns: I) -> Result<Self, GlobReferenceError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let patterns = patterns
            .into_iter()
            .map(|pattern| parse_pattern(pattern.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        if patterns.iter().all(|pattern| pattern.negated) {
            return Err(GlobReferenceError::NoPositivePattern);
        }
        Ok(Self {
            patterns,
            file_name_view: false,
        })
    }

    /// Also match a file's bare name, at any depth, for every pattern whose
    /// glob text after the prefix has no `/` (`*.md`, `^*.md`, `!_*.md`).
    #[must_use]
    pub fn with_file_name_view(mut self) -> Self {
        self.file_name_view = true;
        self
    }

    /// The patterns as authored.
    pub fn patterns(&self) -> impl Iterator<Item = &str> {
        self.patterns.iter().map(|pattern| pattern.authored.as_str())
    }

    /// Escape `text` so a pattern reads it as a literal name: `[id].md`
    /// becomes `[[]id[]].md`. The result means the same on every OS.
    #[must_use]
    pub fn escape(text: &str) -> String {
        parse::escape(text)
    }

    /// Every matching file across all roots, in native order.
    ///
    /// ## Errors
    ///
    /// Any pattern's root failure (see [`GlobReferenceError`]), an invalid
    /// context, or [`GlobReferenceError::Io`] naming a directory the search
    /// must enter (a search directory or any directory below it) that exists
    /// but cannot be read. Such a failure is never an empty or partial
    /// listing. A search directory that does not exist is not an error: it
    /// holds no matches.
    pub fn list_files(
        &self,
        ctx: &FileResolutionContext,
    ) -> Result<GlobListing, GlobReferenceError> {
        self.prepare_all(ctx)?.list(false)
    }

    /// The first file of the native order: the shallowest match under the
    /// first root that has any. Later roots are never walked once a root has
    /// a match.
    ///
    /// ## Errors
    ///
    /// As [`list_files`](Self::list_files), for every root it walks: an
    /// unreadable directory under the first root with a match is an error,
    /// not `Ok(None)` or a later match.
    pub fn take_first(
        &self,
        ctx: &FileResolutionContext,
    ) -> Result<Option<PathBuf>, GlobReferenceError> {
        Ok(self.prepare_all(ctx)?.list(true)?.matches.into_iter().next())
    }

    /// Whether `path` is a member, judged without walking. The path need not
    /// exist; a relative path is read from the context's `cwd`.
    ///
    /// A pattern whose roots this context cannot supply admits or rejects
    /// nothing, and an invalid context admits nothing; construct-time errors
    /// were already reported by [`new`](Self::new).
    pub fn matches(&self, path: &Path, ctx: &FileResolutionContext) -> bool {
        self.prepare_available(ctx)
            .is_some_and(|set| set.matches(path, ctx.cwd()))
    }

    /// Whether [`list_files`](Self::list_files) would list the existing file
    /// `path` if its walk reached it: [`matches`](Self::matches), except that
    /// a file symlink whose target leaves the tree, which a listing reports
    /// as a [`SkippedEntry`], is not listed.
    ///
    /// For a caller that walks [`roots`](Self::roots) itself (to apply its
    /// own filters) and must offer exactly what a listing would. Reads the
    /// link's target, so it is not lexical.
    pub fn lists_file(&self, path: &Path, ctx: &FileResolutionContext) -> bool {
        self.prepare_available(ctx)
            .is_some_and(|set| set.lists(path, ctx.cwd()))
    }

    /// Whether the absolute `path` is a member, judged with no context: a
    /// bare pattern is read from the path's filesystem root (so
    /// `**/fixes/**/spec.md` judges the full path) and an absolute pattern as
    /// written. A pattern whose roots need a context (`./`, `../`, `&`, `^`,
    /// `@`, `~`, a vault, or a `{{VAR}}`) neither admits nor rejects, and a
    /// relative `path` is never a member.
    ///
    /// For a caller that has no request, such as a validator that judges
    /// values by syntax alone. Nothing is read from the process.
    pub fn matches_without_context(&self, path: &Path) -> bool {
        let Some(root) = path.ancestors().last().filter(|_| path.is_absolute()) else {
            return false;
        };
        let detached = Self {
            patterns: self
                .patterns
                .iter()
                .filter(|pattern| {
                    matches!(
                        pattern.parsed.kind,
                        ReferenceKind::ImplicitRelative(_) | ReferenceKind::Absolute(_)
                    )
                })
                .cloned()
                .collect(),
            file_name_view: self.file_name_view,
        };
        let ctx = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new());
        detached.matches(path, &ctx)
    }

    /// The positive patterns' roots in precedence order, for a caller that
    /// walks them itself and judges each file with
    /// [`lists_file`](Self::lists_file).
    /// A pattern whose roots this context cannot supply contributes none.
    pub fn roots(&self, ctx: &FileResolutionContext) -> Vec<PathBuf> {
        self.prepare_available(ctx)
            .map(|set| set.roots())
            .unwrap_or_default()
    }

    fn prepare_all(&self, ctx: &FileResolutionContext) -> Result<PreparedSet, GlobReferenceError> {
        ctx.validate().map_err(GlobReferenceError::InvalidContext)?;
        let internal = ResolutionContext::from_context(ctx);
        let patterns = self
            .patterns
            .iter()
            .map(|pattern| self.prepare_one(pattern, ctx.magic_paths(), ctx.vault_roots(), &internal))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PreparedSet { patterns })
    }

    fn prepare_available(&self, ctx: &FileResolutionContext) -> Option<PreparedSet> {
        ctx.validate().ok()?;
        let internal = ResolutionContext::from_context(ctx);
        let patterns = self
            .patterns
            .iter()
            .filter_map(|pattern| {
                self.prepare_one(pattern, ctx.magic_paths(), ctx.vault_roots(), &internal)
                    .ok()
            })
            .collect();
        Some(PreparedSet { patterns })
    }

    fn prepare_one(
        &self,
        pattern: &GlobPattern,
        magic_paths: &MagicPathList,
        vault_roots: &[PathBuf],
        ctx: &ResolutionContext,
    ) -> Result<roots::PreparedPattern, GlobReferenceError> {
        let pieces = authored_pieces(&pattern.parsed, ctx)
            .map_err(|source| GlobReferenceError::from_reference(&pattern.authored, source))?;
        prepare(
            PatternInput {
                authored: &pattern.authored,
                negated: pattern.negated,
                parsed: &pattern.parsed,
                pieces,
                name_view: self.file_name_view && pattern.name_only,
            },
            magic_paths,
            vault_roots,
            ctx,
        )
    }
}

/// The recursive (`%`) search: the shallowest match of `**/<payload>` under
/// the reference's roots, most local root first.
///
/// `interpolated` is the payload after interpolation; all of it is literal,
/// so `%pages/[id].md` finds a file named `[id].md`. Leading `./` and `../`
/// stay ahead of the `**/` (`%./x.md` searches `./**/x.md`). An absolute
/// payload searches below its own directory (`%/a/b/x.md` is `/a/b/**/x.md`).
pub(crate) fn take_first_recursive(
    parsed: &ParsedReference,
    interpolated: &str,
    absolute: bool,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, GlobReferenceError> {
    let pieces = if absolute {
        let split = interpolated
            .rfind(std::path::is_separator)
            .map_or(0, |separator| separator + 1);
        vec![
            Piece::literal(&interpolated[..split]),
            Piece::glob("**/"),
            Piece::literal(&interpolated[split..]),
        ]
    } else {
        let lead = leading_dot_hops(interpolated);
        vec![
            Piece::literal(&interpolated[..lead]),
            Piece::glob("**/"),
            Piece::literal(&interpolated[lead..]),
        ]
    };
    let pattern = prepare(
        PatternInput {
            authored: &parsed.authored,
            negated: false,
            parsed,
            pieces,
            name_view: false,
        },
        magic_paths,
        vault_roots,
        ctx,
    )?;
    Ok(PreparedSet {
        patterns: vec![pattern],
    }
    .list(true)?
    .matches
    .into_iter()
    .next())
}

/// The byte length of the leading `./` and `../` hops of `text`.
fn leading_dot_hops(text: &str) -> usize {
    let mut rest = text;
    loop {
        let hop = ["./", "../", ".\\", "..\\"]
            .into_iter()
            .find(|hop| rest.starts_with(hop) && (cfg!(windows) || !hop.contains('\\')));
        match hop {
            Some(hop) => rest = &rest[hop.len()..],
            None => return text.len() - rest.len(),
        }
    }
}
