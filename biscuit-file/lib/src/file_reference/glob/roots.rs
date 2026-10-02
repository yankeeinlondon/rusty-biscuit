//! Resolving a pattern's roots in a context, and the one rule for which root
//! owns a file.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

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
/// absolute pattern's root is its literal directory prefix.
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
            (vec![root], Vec::new(), split.glob)
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
    })
}

impl PreparedPattern {
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
