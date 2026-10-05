//! Resolving a pattern's roots in a context, and the one rule for which root
//! owns a file.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};

use globset::GlobMatcher;

use super::error::GlobReferenceError;
use super::parse::{Piece, SplitPattern, compile, escape};
use crate::canonicalize_simplified;
use crate::file_reference::context::ResolutionContext;
use crate::file_reference::resolve::{
    Boundary, interpolate_pieces, normalize_components, reference_roots, validate_containment,
};
use crate::file_reference::{MagicPathList, ParsedReference, PathIdentity};

/// What [`prepare`] needs to know about one pattern.
pub(crate) struct PatternInput<'a> {
    /// The pattern as authored, for error messages.
    pub authored: &'a str,
    pub negated: bool,
    pub parsed: &'a ParsedReference,
    /// The pattern text after its prefix, interpolated.
    pub pieces: Vec<Piece>,
    /// Whether a match on the file name alone admits a file.
    pub name_view: bool,
}

/// One pattern resolved against one context.
pub(crate) struct PreparedPattern {
    pub negated: bool,
    /// The roots in precedence order.
    pub roots: Vec<PatternRoot>,
    /// Matches a file's `/`-spelled path relative to its owning root.
    matcher: GlobMatcher,
    name_view: bool,
    /// The canonical tree root a relative pattern is bound to, when the
    /// context enforces one.
    pub tree_boundary: Option<PathIdentity>,
    /// The most components below its root a matched file can have, or
    /// `None` when unbounded (`**`, or the file-name view). An over-estimate
    /// is safe; an under-estimate would hide a failure.
    max_depth: Option<usize>,
}

/// One root of a pattern.
pub(crate) struct PatternRoot {
    /// The root as the context spells it, normalized. Listed paths keep this
    /// spelling.
    pub root: PathBuf,
    /// The root's canonical identity, which every containment judgment uses,
    /// so `/var` and `/private/var` are one directory.
    pub identity: PathIdentity,
    /// The directory a walk starts in: the root plus the pattern's literal
    /// directory prefix.
    pub walk: PathBuf,
}

/// Interpolate a pattern's payload, keeping `{{VAR}}` values literal.
pub(crate) fn authored_pieces(
    parsed: &ParsedReference,
    ctx: &ResolutionContext,
) -> Result<Vec<Piece>, crate::FileReferenceError> {
    Ok(interpolate_pieces(parsed.kind.template(), ctx)?
        .into_iter()
        .map(|piece| Piece {
            text: piece.text,
            glob: !piece.from_env,
        })
        .collect())
}

/// Resolve one pattern's roots and compile its glob for this context.
///
/// The roots are exactly the ones single-file resolution uses for the
/// pattern's prefix. A pattern's literal leading directories narrow each root
/// to the directory a walk starts in; leading `..` hops move the root itself,
/// so a file is always judged relative to a directory that contains it. An
/// absolute pattern's root is its literal directory prefix; when that prefix
/// reaches an existing directory stored under another spelling, or one whose
/// spelling cannot be verified, the pattern has no root and admits nothing
/// (see [`respelled`]).
///
/// ## Errors
///
/// A bare, `./`, or `../` walk directory outside the context's tree is a
/// [`GlobReferenceError::RelativeTreeEscape`], a `&`/`^` one outside the
/// repository a resolution error, and a root the context cannot supply
/// (repository, home, vault, environment variable) the matching
/// single-file error.
pub(crate) fn prepare(
    input: PatternInput<'_>,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<PreparedPattern, GlobReferenceError> {
    let authored = input.authored;
    let reference_error = |source| GlobReferenceError::from_reference(authored, source);
    let raw: String = input.pieces.iter().map(|piece| piece.text.as_str()).collect();
    let reference = reference_roots(input.parsed, &raw, magic_paths, vault_roots, ctx)
        .map_err(reference_error)?;
    let split = SplitPattern::new(&input.pieces);

    let (spelled_roots, walk_suffix, glob): (Vec<PathBuf>, Vec<OsString>, String) =
        if reference.absolute {
            let root = normalize_components(Path::new(&split.literal_dir));
            let roots = if respelled(&root, &supplied_prefix(&input.pieces)) {
                Vec::new()
            } else {
                vec![root]
            };
            (roots, Vec::new(), split.glob)
        } else {
            let route = PathIdentity::new(Path::new(&split.literal_dir))
                .relative_from(&PathIdentity::new(Path::new("")))
                .ok_or_else(|| {
                    reference_error(crate::FileReferenceError::InvalidSyntax(format!(
                        "the directory `{}` of glob reference `{authored}` is not relative",
                        split.literal_dir
                    )))
                })?;
            let hops: PathBuf = std::iter::repeat_n("..", route.parent_hops()).collect();
            let forward = route.forward().to_vec();
            let mut glob: Vec<String> = forward
                .iter()
                .map(|name| escape(&name.to_string_lossy()))
                .collect();
            glob.push(split.glob);
            let roots = reference
                .roots
                .iter()
                .map(|root| normalize_components(&root.join(&hops)))
                .collect();
            (roots, forward, glob.join("/"))
        };

    // `*` never crosses `/`, so without `**` every separator in the text
    // (a brace alternative's included) bounds the depth from above.
    let max_depth = (!input.name_view && !glob.contains("**")).then(|| glob.matches('/').count() + 1);
    let matcher = compile(&glob).map_err(|message| GlobReferenceError::InvalidGlob {
        pattern: authored.to_string(),
        message,
    })?;

    let mut roots: Vec<PatternRoot> = Vec::with_capacity(spelled_roots.len());
    for root in spelled_roots {
        let walk = walk_suffix.iter().fold(root.clone(), |dir, name| dir.join(name));
        if let Some(base_dir) = &reference.tree_boundary {
            validate_containment(Boundary::Tree { base_dir }, authored, &walk)
                .map_err(reference_error)?;
        }
        if let Some((sigil, repository_root)) = &reference.repository {
            validate_containment(
                Boundary::Repository {
                    sigil: *sigil,
                    root: repository_root,
                },
                authored,
                &walk,
            )
            .map_err(reference_error)?;
        }
        let identity = canonical_identity(&root);
        if roots.iter().all(|seen| seen.identity != identity) {
            roots.push(PatternRoot {
                root,
                identity,
                walk,
            });
        }
    }

    Ok(PreparedPattern {
        negated: input.negated,
        roots,
        matcher,
        name_view: input.name_view,
        tree_boundary: reference.tree_boundary.as_deref().map(canonical_identity),
        max_depth,
    })
}

impl PreparedPattern {
    /// Whether a file inside the directory `dir` could be a match: `dir`
    /// lies under one of the roots, no deeper than a match can reach.
    pub(crate) fn could_match_below(&self, dir: &PathIdentity) -> bool {
        owning_root(&self.roots, dir).is_some_and(|(_, relative)| {
            self.max_depth.is_none_or(|max| relative.len() < max)
        })
    }

    /// Judge a file by its canonical identity and file name.
    ///
    /// `None` when no root of the pattern contains the file. Otherwise the
    /// file is judged once, relative to the first root that contains it
    /// (nearest-root judgment), never relative to a later one.
    pub(crate) fn judge(&self, file: &PathIdentity, name: &OsStr) -> Option<bool> {
        let (_, relative) = owning_root(&self.roots, file)?;
        if relative.is_empty() {
            return Some(false);
        }
        let relative: Vec<_> = relative.iter().map(|part| part.to_string_lossy()).collect();
        Some(
            self.matcher.is_match(relative.join("/"))
                || (self.name_view && self.matcher.is_match(name.to_string_lossy().as_ref())),
        )
    }
}

/// The first root, in precedence order, that contains `file`, with the names
/// of `file` below it.
///
/// This is the one ownership rule: listing skips a file in every pass but
/// its owner's, and matching judges a file only relative to its owner.
pub(crate) fn owning_root<'a>(
    roots: &[PatternRoot],
    file: &'a PathIdentity,
) -> Option<(usize, &'a [OsString])> {
    roots
        .iter()
        .enumerate()
        .find_map(|(index, root)| file.strip_prefix(&root.identity).map(|rest| (index, rest)))
}

/// Whether an authored directory name of the absolute `root` reaches a
/// directory stored under another spelling.
///
/// An absolute pattern's literal directories become its root, which every
/// judgment compares by canonical identity, so on a filesystem that folds
/// names (`DOCS` for `docs`, `ς` for `Σ`, `ß` for `SS`) the root would own the
/// stored directory. A relative pattern keeps the same names in its
/// case-sensitive matcher; this restores that rule for the names the author
/// wrote, whatever equivalence the filesystem applies. Names inside
/// `supplied` ([`supplied_prefix`]) were not written as glob text and are
/// judged by identity, like any other context root.
///
/// Each authored name the filesystem resolves must be spelled exactly as an
/// entry of its directory ([`spelled_as_stored`]); that keeps a symlinked name
/// such as macOS `/var` valid. A name that cannot be examined at all is not
/// resolved by the filesystem either, so every judgment below it stays
/// lexical and the walk reports the failure.
fn respelled(root: &Path, supplied: &Path) -> bool {
    let (mut dir, authored) = match root.strip_prefix(supplied) {
        Ok(rest) => (supplied.to_path_buf(), rest),
        Err(_) => (PathBuf::new(), root),
    };
    for component in authored.components() {
        let Component::Normal(name) = component else {
            dir.push(component);
            continue;
        };
        let path = dir.join(name);
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            return false;
        };
        if !spelled_as_stored(&dir, name, &path, &meta) {
            return true;
        }
        dir = path;
    }
    false
}

/// Whether the existing `path`, reached as `name` in `dir`, is stored under
/// exactly that spelling.
///
/// The decisive evidence is an entry of `dir` spelled `name`. When `dir`
/// cannot be listed (a traversal-only directory), the stored spelling is
/// taken from the canonical path on macOS and Windows, which report stored
/// names; a symlinked name, or any name on another OS, is accepted only when
/// the directory provably does not fold that name's case. Anything else is
/// unverified and rejected. On Windows an 8.3 short name (`RUNNER~1`) is the
/// filesystem's own alternate name for a directory; it is never listed, and
/// it is accepted.
fn spelled_as_stored(dir: &Path, name: &OsStr, path: &Path, meta: &std::fs::Metadata) -> bool {
    if cfg!(windows) && is_short_name(name) {
        return true;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut complete = true;
        for entry in entries {
            match entry {
                Ok(entry) if entry.file_name() == name => return true,
                Ok(_) => {}
                Err(_) => complete = false,
            }
        }
        if complete {
            return false;
        }
    }
    if cfg!(any(target_os = "macos", windows)) && !meta.file_type().is_symlink() {
        return canonicalize_simplified(path)
            .is_ok_and(|canonical| canonical.file_name() == Some(name));
    }
    !folds_case(dir, name, meta)
}

/// Whether `dir` may resolve another case of `name` to the same entry: a
/// differently cased spelling exists and is not provably a different entry.
/// A name without a differently cased spelling cannot be a case alias.
fn folds_case(dir: &Path, name: &OsStr, meta: &std::fs::Metadata) -> bool {
    let Some(text) = name.to_str() else {
        return true;
    };
    [text.to_uppercase(), text.to_lowercase()]
        .into_iter()
        .filter(|variant| variant != text)
        .any(|variant| match std::fs::symlink_metadata(dir.join(variant)) {
            Ok(other) => same_entry(meta, &other),
            Err(error) => error.kind() != std::io::ErrorKind::NotFound,
        })
}

#[cfg(unix)]
fn same_entry(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

/// Stable Rust exposes no file identity on Windows, so a second spelling that
/// exists is assumed to be the same entry.
#[cfg(not(unix))]
fn same_entry(_: &std::fs::Metadata, _: &std::fs::Metadata) -> bool {
    true
}

/// Whether `name` has the form of a Windows 8.3 short name: a base of at most
/// eight characters ending in `~` and digits, and an extension of at most
/// three.
fn is_short_name(name: &OsStr) -> bool {
    let Some(text) = name.to_str() else {
        return false;
    };
    let (base, extension) = text.rsplit_once('.').unwrap_or((text, ""));
    let Some((stem, digits)) = base.rsplit_once('~') else {
        return false;
    };
    !stem.is_empty()
        && !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && base.chars().count() <= 8
        && extension.chars().count() <= 3
}

/// The leading pattern text that is not glob text, cut back to whole
/// directory names: a `{{VAR}}` value, or the directory of a `%` payload,
/// which keeps the case rules of single-file resolution.
fn supplied_prefix(pieces: &[Piece]) -> PathBuf {
    let supplied: String = pieces
        .iter()
        .take_while(|piece| !piece.glob)
        .map(|piece| piece.text.as_str())
        .collect();
    let next = pieces
        .iter()
        .skip_while(|piece| !piece.glob)
        .find_map(|piece| piece.text.chars().next());
    let whole = if next.is_none_or(std::path::is_separator) {
        supplied.len()
    } else {
        supplied
            .rfind(std::path::is_separator)
            .map_or(0, |separator| separator + 1)
    };
    normalize_components(Path::new(&supplied[..whole]))
}

/// The identity of `path` with its longest existing prefix canonicalized.
pub(crate) fn canonical_identity(path: &Path) -> PathIdentity {
    PathIdentity::new(&canonical_prefix(path))
}

/// `path` with its longest existing prefix canonicalized and the rest
/// appended unchanged, so `/var/x/new.md` and `/private/var/x/new.md` are
/// spelled alike whether or not `new.md` exists. Reads no more than that
/// prefix's metadata.
pub(crate) fn canonical_prefix(path: &Path) -> PathBuf {
    let normalized = normalize_components(path);
    let mut existing = normalized.as_path();
    let mut missing: Vec<&OsStr> = Vec::new();
    loop {
        if let Ok(canonical) = canonicalize_simplified(existing) {
            return missing.iter().rev().fold(canonical, |dir, name| dir.join(name));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name);
                existing = parent;
            }
            _ => return normalized,
        }
    }
}
