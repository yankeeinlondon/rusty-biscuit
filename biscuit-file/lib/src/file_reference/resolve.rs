use std::path::{Path, PathBuf};

use tracing::{debug, trace};
use walkdir::WalkDir;

use crate::file_reference::context::{
    ResolutionContext, find_git_root, find_package_area, home_dir,
};
use crate::file_reference::error::FileReferenceError;
use crate::file_reference::{
    CompletionEntryForm, DetailedOutcome, MagicPathList, ParsedReference, PartialCompletion,
    PathTemplate, ProbeDisposition, ProbedCandidate, ReferenceKind, ResolutionCandidate,
    ResolutionFailure, RootProvenance, TemplateSegment, make_candidate, make_partial_completion,
    make_probed,
};

#[cfg(feature = "url")]
use crate::file_reference::Resolved;

/// The engine-level result of a single detailed resolution.
///
/// [`resolve_core`] never returns an `Err`: every failure is a typed
/// [`CoreOutcome::Failed`] carrying its underlying [`FileReferenceError`]. The
/// legacy convenience projection ([`resolve`]) collapses this onto
/// `Result<Option<PathBuf>, _>`.
pub(crate) struct CoreResolution {
    pub candidates: Vec<ProbedCandidate>,
    pub repository_root: Option<PathBuf>,
    pub outcome: CoreOutcome,
}

pub(crate) enum CoreOutcome {
    Matched(PathBuf),
    NoMatch,
    Failed(ResolutionFailure, FileReferenceError),
}

impl CoreOutcome {
    /// Split the engine outcome into the public [`DetailedOutcome`] plus the
    /// underlying error (present for every failure except `NoMatch`).
    pub(crate) fn into_parts(self) -> (DetailedOutcome, Option<FileReferenceError>) {
        match self {
            CoreOutcome::Matched(p) => (DetailedOutcome::Matched(p), None),
            CoreOutcome::NoMatch => (DetailedOutcome::Failed(ResolutionFailure::NoMatch), None),
            CoreOutcome::Failed(kind, e) => (DetailedOutcome::Failed(kind), Some(e)),
        }
    }
}

/// Resolve a parsed reference against runtime context, projecting the detailed
/// outcome onto the legacy `Result<Option<PathBuf>, _>` convenience shape
/// without reordering candidates.
pub(crate) fn resolve(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    match resolve_core(parsed, magic_paths, vault_roots, ctx).outcome {
        CoreOutcome::Matched(p) => Ok(Some(p)),
        CoreOutcome::NoMatch => Ok(None),
        CoreOutcome::Failed(_, e) => Err(e),
    }
}

/// Resolve a parsed reference into the full engine-level detailed outcome.
///
/// Discovers the repository root at most once (D10) and only for kinds that
/// anchor on it, so an explicit/absolute/home/vault reference performs no git
/// lookup. The kind-specific candidate plan is then probed with a fallible
/// metadata operation.
pub(crate) fn resolve_core(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> CoreResolution {
    #[cfg(feature = "url")]
    if matches!(parsed.kind, ReferenceKind::Url(_)) {
        return match interpolated_url_string(&parsed.kind, ctx) {
            Ok(raw) => failed_core(
                ResolutionFailure::UnsupportedRemote,
                FileReferenceError::RemoteNotLocal(raw),
            ),
            Err(e) => failed_core(classify_error(&e), e),
        };
    }

    let interpolated = match interpolate(parsed.kind.template(), ctx) {
        Ok(value) => value,
        Err(e) => return failed_core(classify_error(&e), e),
    };

    let repository_root = if kind_uses_repository_root(&parsed.kind) {
        match resolve_repository_root(ctx) {
            Ok(root) => root,
            Err(e) => return failed_core(classify_error(&e), e),
        }
    } else {
        ctx.repository_root.clone()
    };

    if parsed.recursive {
        resolve_recursive_core(parsed, &interpolated, magic_paths, vault_roots, ctx, repository_root)
    } else {
        resolve_direct_core(parsed, &interpolated, magic_paths, vault_roots, ctx, repository_root)
    }
}

/// Build a failed [`CoreResolution`] with no attempted candidates.
fn failed_core(failure: ResolutionFailure, error: FileReferenceError) -> CoreResolution {
    CoreResolution {
        candidates: Vec::new(),
        repository_root: None,
        outcome: CoreOutcome::Failed(failure, error),
    }
}

/// Whether a reference kind anchors any of its roots on the repository root.
///
/// Only these kinds trigger git discovery; the rest never touch it, so an
/// explicit-relative, absolute, home, or vault reference cannot fail on a git
/// error it never needed.
fn kind_uses_repository_root(kind: &ReferenceKind) -> bool {
    matches!(
        kind,
        ReferenceKind::ImplicitRelative(_) | ReferenceKind::Magic(_) | ReferenceKind::Package(_)
    )
}

/// Map a typed [`FileReferenceError`] onto its resolution-failure class.
///
/// The class must not be re-derived from the reference kind downstream (D8);
/// permission/I/O failures in particular must stay distinct from `no_match`.
fn classify_error(error: &FileReferenceError) -> ResolutionFailure {
    use FileReferenceError as E;
    match error {
        E::RemoteNotLocal(_) => ResolutionFailure::UnsupportedRemote,
        #[cfg(feature = "url")]
        E::InvalidUrl(_) => ResolutionFailure::UnsupportedRemote,
        E::InvalidSyntax(_) | E::RelativePath { .. } => ResolutionFailure::InvalidReference,
        E::MissingEnvironmentVariable { .. }
        | E::VaultNotConfigured
        | E::MissingHomeContext
        | E::UnsupportedUserHome(_)
        | E::RepositoryRootNotContainingSource { .. }
        | E::BareRepository
        | E::Git(_)
        | E::Workspace(_) => ResolutionFailure::MissingContext,
        E::Io { .. } | E::CurrentDirectory(_) => ResolutionFailure::Io,
    }
}

/// The outcome of probing a single candidate path with a fallible metadata
/// operation.
enum ProbeStep {
    /// The path does not exist (`NotFound`) -- advance the search.
    Missing,
    /// The path exists but is not a regular file -- advance the search.
    NonFile,
    /// The path is a regular file (following a symlink) -- the winner.
    Matched,
    /// A permission/invalid-path/other I/O failure -- stop the search.
    Io(std::io::Error),
}

/// Probe a candidate with `fs::metadata`, distinguishing absence from I/O
/// failure. `Path::is_file()` is insufficient because it collapses permission
/// and other I/O errors into `false`. Metadata follows symlinks, so a symlink
/// to a regular file still matches.
fn probe_candidate(path: &Path) -> ProbeStep {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => ProbeStep::Matched,
        Ok(_) => ProbeStep::NonFile,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ProbeStep::Missing,
        Err(e) => ProbeStep::Io(e),
    }
}

#[cfg(feature = "url")]
fn interpolated_url_string(
    kind: &ReferenceKind,
    ctx: &ResolutionContext,
) -> Result<String, FileReferenceError> {
    let template = kind.template();
    let mut raw = String::new();
    for seg in &template.segments {
        match seg {
            TemplateSegment::Literal(l) => raw.push_str(l),
            TemplateSegment::EnvVar(name) => {
                // Source from the captured env snapshot, not live process env,
                // so `resolve_from`/`resolve_in_context` stay honest. A missing
                // var re-emits `{{NAME}}` verbatim (unchanged URL behavior).
                let val = ctx
                    .env
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| format!("{{{{{name}}}}}"));
                raw.push_str(&val);
            }
        }
    }
    Ok(raw)
}

/// Resolve a parsed reference to a typed [`Resolved`] target.
#[cfg(feature = "url")]
pub(crate) fn resolve_target(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
) -> Result<Option<Resolved>, FileReferenceError> {
    let ctx = ResolutionContext::from_ambient()?;

    if let ReferenceKind::Url(_) = &parsed.kind {
        let raw = interpolated_url_string(&parsed.kind, &ctx)?;
        let url = ::url::Url::parse(&raw)
            .map_err(|e| FileReferenceError::InvalidUrl(e.to_string()))?;
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(FileReferenceError::InvalidUrl(format!(
                "unsupported scheme: {scheme}"
            )));
        }
        return Ok(Some(Resolved::Remote(url)));
    }

    let local = resolve(parsed, magic_paths, vault_roots, &ctx)?;
    Ok(local.map(Resolved::Local))
}

/// Resolve without recursion -- probe the ordered candidate plan.
fn resolve_direct_core(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
    repository_root: Option<PathBuf>,
) -> CoreResolution {
    let candidates = match build_candidates(
        parsed,
        interpolated,
        magic_paths,
        vault_roots,
        ctx,
        repository_root.as_deref(),
    ) {
        Ok(candidates) => candidates,
        Err(e) => {
            let failure = classify_error(&e);
            return CoreResolution {
                candidates: Vec::new(),
                repository_root,
                outcome: CoreOutcome::Failed(failure, e),
            };
        }
    };

    let mut probed: Vec<ProbedCandidate> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        match probe_candidate(candidate.path()) {
            ProbeStep::Matched => {
                let resolved = normalize_absolute(candidate.path(), &ctx.cwd);
                debug!(?resolved, "resolved file reference");
                probed.push(make_probed(candidate, ProbeDisposition::Matched));
                return CoreResolution {
                    candidates: probed,
                    repository_root,
                    outcome: CoreOutcome::Matched(resolved),
                };
            }
            ProbeStep::Missing => {
                trace!(candidate = ?candidate.path(), "candidate missing");
                probed.push(make_probed(candidate, ProbeDisposition::Missing));
            }
            ProbeStep::NonFile => {
                trace!(candidate = ?candidate.path(), "candidate is not a regular file");
                probed.push(make_probed(candidate, ProbeDisposition::NonFile));
            }
            ProbeStep::Io(err) => {
                // A permission/invalid-path/other I/O failure stops the search
                // with the candidate path and typed source attached; it must not
                // be misreported as absence.
                let kind = err.kind();
                let path = candidate.path().to_path_buf();
                probed.push(make_probed(candidate, ProbeDisposition::Io(kind)));
                let typed = FileReferenceError::Io { path, source: err };
                return CoreResolution {
                    candidates: probed,
                    repository_root,
                    outcome: CoreOutcome::Failed(ResolutionFailure::Io, typed),
                };
            }
        }
    }

    debug!("no candidate matched");
    CoreResolution {
        candidates: probed,
        repository_root,
        outcome: CoreOutcome::NoMatch,
    }
}

/// Resolve with recursive directory traversal, retaining the global lexical
/// winner while sourcing its search roots from the shared candidate builder.
fn resolve_recursive_core(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
    repository_root: Option<PathBuf>,
) -> CoreResolution {
    let roots = match build_search_roots(
        parsed,
        magic_paths,
        vault_roots,
        ctx,
        repository_root.as_deref(),
    ) {
        Ok(roots) => roots,
        Err(e) => {
            let failure = classify_error(&e);
            return CoreResolution {
                candidates: Vec::new(),
                repository_root,
                outcome: CoreOutcome::Failed(failure, e),
            };
        }
    };

    let path = Path::new(interpolated);

    // Extract the filename to search for and optional subdirectory filter
    let needle = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| interpolated.to_string());

    let subdir_filter = if path.components().count() > 1 {
        path.parent().map(|p| p.to_path_buf())
    } else {
        None
    };

    debug!(
        root_count = roots.len(),
        ?needle,
        "starting recursive search"
    );

    let mut matches: Vec<PathBuf> = Vec::new();

    for root_candidate in &roots {
        let root = root_candidate.path();
        if !root.is_dir() {
            continue;
        }

        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walker {
            if !entry.file_type().is_file() {
                continue;
            }

            let entry_name = entry.file_name().to_string_lossy();
            if entry_name != needle {
                continue;
            }

            // If there's a subdirectory filter, check that the entry's parent ends with it
            if let Some(ref subdir) = subdir_filter {
                let entry_path = entry.path();
                if let Ok(rel) = entry_path.strip_prefix(root) {
                    if let Some(parent) = rel.parent() {
                        let subdir_str = subdir.to_string_lossy();
                        let parent_str = parent.to_string_lossy();
                        if !parent_str.ends_with(subdir_str.as_ref()) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            }

            matches.push(normalize_absolute(entry.path(), &ctx.cwd));
        }
    }

    // Sort lexicographically and select the first match (global lexical winner).
    matches.sort();
    debug!(match_count = matches.len(), "recursive search complete");

    // The search roots are the diagnostics-visible "candidates" for a recursive
    // reference; they carry provenance but are not direct file probes.
    let probed: Vec<ProbedCandidate> = roots
        .into_iter()
        .map(|candidate| make_probed(candidate, ProbeDisposition::SearchRoot))
        .collect();

    let outcome = match matches.into_iter().next() {
        Some(path) => CoreOutcome::Matched(path),
        None => CoreOutcome::NoMatch,
    };

    CoreResolution {
        candidates: probed,
        repository_root,
        outcome,
    }
}

/// Resolve the repository root for a resolution context.
///
/// A caller-supplied `repository_root` wins; otherwise the root is discovered
/// live from `cwd` via `gix`. This is the single seam that makes the root
/// caller-suppliable without `biscuit-file` depending on `sniff`.
fn resolve_repository_root(
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    match &ctx.repository_root {
        Some(root) => Ok(Some(root.clone())),
        None => find_git_root(&ctx.cwd),
    }
}

/// A search root paired with the provenance of where it came from.
///
/// Provenance is data, never inferred from a string prefix downstream. The
/// repository root is passed in pre-resolved so discovery happens at most once
/// per resolution (D10).
#[derive(Debug)]
struct RootEntry {
    path: PathBuf,
    provenance: RootProvenance,
}

/// Collect the ordered search roots for a reference kind, each tagged with its
/// provenance. `repository_root` is the pre-resolved root (supplied or
/// discovered once); kinds that do not anchor on it ignore it.
fn collect_roots(
    kind: &ReferenceKind,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
    repository_root: Option<&Path>,
) -> Result<Vec<RootEntry>, FileReferenceError> {
    let source = |path: PathBuf| RootEntry {
        path,
        provenance: RootProvenance::Source,
    };
    match kind {
        ReferenceKind::Relative(_) => Ok(vec![source(ctx.cwd.clone())]),
        ReferenceKind::ImplicitRelative(_) => {
            // Base/CWD first, then repository root -- Phase 3 preserves the
            // current precedence; the flip to repository-first is Phase 4.
            let mut roots = vec![source(ctx.cwd.clone())];
            if let Some(git_root) = repository_root
                && git_root != ctx.cwd
            {
                roots.push(RootEntry {
                    path: git_root.to_path_buf(),
                    provenance: RootProvenance::Repository,
                });
            }
            Ok(roots)
        }
        ReferenceKind::Absolute(_) => Ok(vec![RootEntry {
            path: PathBuf::from("/"),
            provenance: RootProvenance::Absolute,
        }]),
        ReferenceKind::Magic(_) => {
            let mut roots = Vec::new();
            roots.extend(magic_paths.prepend.iter().cloned().map(|path| RootEntry {
                path,
                provenance: RootProvenance::Magic,
            }));
            if let Some(git_root) = repository_root {
                roots.push(RootEntry {
                    path: git_root.to_path_buf(),
                    provenance: RootProvenance::Repository,
                });
            }
            if let Some(ref home) = ctx.home_dir {
                roots.push(RootEntry {
                    path: home.clone(),
                    provenance: RootProvenance::Home,
                });
            }
            roots.extend(magic_paths.append.iter().cloned().map(|path| RootEntry {
                path,
                provenance: RootProvenance::Magic,
            }));
            Ok(roots)
        }
        ReferenceKind::Package(_) => {
            let git_root = match repository_root {
                Some(root) => root,
                None => return Ok(vec![]),
            };
            let area = find_package_area(git_root, &ctx.cwd)?;
            Ok(vec![RootEntry {
                path: area.unwrap_or_else(|| git_root.to_path_buf()),
                provenance: RootProvenance::Package,
            }])
        }
        ReferenceKind::Home(_) => match &ctx.home_dir {
            Some(home) => Ok(vec![RootEntry {
                path: home.clone(),
                provenance: RootProvenance::Home,
            }]),
            None => Err(FileReferenceError::MissingHomeContext),
        },
        ReferenceKind::Vault(_) => {
            let mut roots: Vec<RootEntry> = vault_roots
                .iter()
                .cloned()
                .map(|path| RootEntry {
                    path,
                    provenance: RootProvenance::Vault,
                })
                .collect();
            if let Some(vault_env) = ctx.env.get("VAULT") {
                roots.extend(std::env::split_paths(vault_env).map(|path| RootEntry {
                    path,
                    provenance: RootProvenance::Vault,
                }));
            }
            if roots.is_empty() {
                return Err(FileReferenceError::VaultNotConfigured);
            }
            Ok(roots)
        }
        #[cfg(feature = "url")]
        ReferenceKind::Url(_) => Ok(vec![]),
    }
}

/// Build the ordered, deduplicated candidate paths for non-recursive
/// resolution, each carrying the provenance of its root.
fn build_candidates(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
    repository_root: Option<&Path>,
) -> Result<Vec<ResolutionCandidate>, FileReferenceError> {
    if let ReferenceKind::Absolute(_) = &parsed.kind {
        let path = PathBuf::from(interpolated);
        if !path.is_absolute() {
            return Err(FileReferenceError::InvalidSyntax(format!(
                "absolute reference resolved to non-absolute path: {interpolated}"
            )));
        }
        return Ok(vec![make_candidate(path, RootProvenance::Absolute)]);
    }

    #[cfg(feature = "url")]
    if matches!(parsed.kind, ReferenceKind::Url(_)) {
        return Ok(vec![]);
    }

    let roots = collect_roots(&parsed.kind, magic_paths, vault_roots, ctx, repository_root)?;
    let candidates = roots
        .into_iter()
        .map(|root| make_candidate(root.path.join(interpolated), root.provenance))
        .collect();
    Ok(dedupe_candidates(candidates))
}

/// Build the ordered, deduplicated search roots for recursive resolution.
fn build_search_roots(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
    repository_root: Option<&Path>,
) -> Result<Vec<ResolutionCandidate>, FileReferenceError> {
    let roots = collect_roots(&parsed.kind, magic_paths, vault_roots, ctx, repository_root)?;
    let candidates = roots
        .into_iter()
        .map(|root| make_candidate(root.path, root.provenance))
        .collect();
    Ok(dedupe_candidates(candidates))
}

/// Remove lexically duplicate candidates while preserving first-seen order.
///
/// The dedupe key is the `.`/`..`-normalized path, so `<root>/x` reached via two
/// equal roots collapses to one entry; the earlier provenance wins.
fn dedupe_candidates(candidates: Vec<ResolutionCandidate>) -> Vec<ResolutionCandidate> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if seen.insert(normalize_components(candidate.path())) {
            out.push(candidate);
        }
    }
    out
}

/// Build the ordered candidate plan without probing the filesystem.
///
/// Shares the exact builder that [`resolve_core`] probes, so a plan a consumer
/// inspects cannot drift from what execution attempts.
pub(crate) fn candidate_plan(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<ResolutionCandidate>, FileReferenceError> {
    #[cfg(feature = "url")]
    if matches!(parsed.kind, ReferenceKind::Url(_)) {
        return Ok(Vec::new());
    }

    let interpolated = interpolate(parsed.kind.template(), ctx)?;
    let repository_root = if kind_uses_repository_root(&parsed.kind) {
        resolve_repository_root(ctx)?
    } else {
        ctx.repository_root.clone()
    };

    if parsed.recursive {
        build_search_roots(parsed, magic_paths, vault_roots, ctx, repository_root.as_deref())
    } else {
        build_candidates(
            parsed,
            &interpolated,
            magic_paths,
            vault_roots,
            ctx,
            repository_root.as_deref(),
        )
    }
}

/// Interpolate template segments using the resolution context's env vars.
fn interpolate(
    template: &PathTemplate,
    ctx: &ResolutionContext,
) -> Result<String, FileReferenceError> {
    let mut result = String::new();

    for segment in &template.segments {
        match segment {
            TemplateSegment::Literal(s) => result.push_str(s),
            TemplateSegment::EnvVar(name) => {
                let value = ctx.env.get(name).ok_or_else(|| {
                    FileReferenceError::MissingEnvironmentVariable { name: name.clone() }
                })?;
                result.push_str(value);
            }
        }
    }

    Ok(result)
}

/// Normalize a path to absolute without following symlinks.
fn normalize_absolute(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_components(path)
    } else {
        normalize_components(&cwd.join(path))
    }
}

/// Resolve `.` and `..` components without touching the filesystem.
pub(crate) fn normalize_components(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

/// Compute a relative path from `base` to `target`.
pub(crate) fn diff_paths(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = normalize_components(target);
    let base = normalize_components(base);

    // Both must be absolute
    if !target.is_absolute() || !base.is_absolute() {
        return None;
    }

    let mut target_components = target.components().peekable();
    let mut base_components = base.components().peekable();

    // Skip common prefix
    while let (Some(t), Some(b)) = (target_components.peek(), base_components.peek()) {
        if t == b {
            target_components.next();
            base_components.next();
        } else {
            break;
        }
    }

    // Add `..` for each remaining base component
    let mut result = PathBuf::new();
    for _ in base_components {
        result.push("..");
    }

    // Add remaining target components
    for component in target_components {
        result.push(component);
    }

    if result.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(result)
    }
}

/// Expand a partial completion token into its implied roots and segments.
///
/// See [`FileReference::complete_partial`] for the public contract.
pub(crate) fn complete_partial(
    token: &str,
    base: &Path,
) -> Result<Option<PartialCompletion>, FileReferenceError> {
    let Some((form, path_part)) = classify_token(token) else {
        return Ok(None);
    };

    let (scope, active) = split_scope_and_active(path_part);

    let base_abs = if base.is_absolute() {
        base.to_path_buf()
    } else {
        let ambient = std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?;
        ambient.join(base)
    };

    // Capture home once (via the cross-platform provider) rather than reading
    // it deep inside the magic-root helper.
    let home = home_dir();

    let roots = match form {
        CompletionEntryForm::Magic => magic_completion_roots(scope, &base_abs, home.as_deref())?,
        CompletionEntryForm::ImplicitRelative => {
            implicit_relative_completion_roots(scope, &base_abs)?
        }
    };

    let rendered_prefix = match form {
        CompletionEntryForm::Magic => format!("@{scope}"),
        CompletionEntryForm::ImplicitRelative => scope.to_string(),
    };

    Ok(Some(make_partial_completion(
        form,
        roots,
        active.to_string(),
        rendered_prefix,
    )))
}

/// Classify a raw token into an entry form plus the path portion following
/// the sigil (if any).
///
/// Returns `None` for any form the supplement does not support.
fn classify_token(token: &str) -> Option<(CompletionEntryForm, &str)> {
    // Recursive (`%`) wraps another form; completion support would need to
    // deal with it explicitly, so opt out.
    if token.starts_with('%') {
        return None;
    }
    // Vault, package, absolute, explicit-relative, and interpolation forms
    // are all deliberately out of scope.
    if token.starts_with("vault:") {
        return None;
    }
    if token.starts_with('!') {
        return None;
    }
    if token.starts_with('/') {
        return None;
    }
    if token.starts_with("./") || token.starts_with("../") || token == "." || token == ".." {
        return None;
    }
    if token.starts_with("{{") {
        return None;
    }

    if let Some(rest) = token.strip_prefix('@') {
        Some((CompletionEntryForm::Magic, rest))
    } else {
        Some((CompletionEntryForm::ImplicitRelative, token))
    }
}

/// Split a path portion into a (scope, active_segment) pair at the last
/// `/`.
///
/// The scope includes the trailing `/` so callers can concatenate it onto
/// other strings without special-casing an empty path.
fn split_scope_and_active(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(idx) => (&path[..=idx], &path[idx + 1..]),
        None => ("", path),
    }
}

/// Compute the absolute roots implied by a `@`-prefixed token at the given
/// scope. `home` is captured once by the caller so this helper performs no
/// ambient home read of its own.
fn magic_completion_roots(
    scope: &str,
    base: &Path,
    home: Option<&Path>,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    let mut roots: Vec<PathBuf> = Vec::new();

    if let Some(git_root) = find_git_root(base)? {
        roots.push(append_scope(&git_root, scope));
    }
    if let Some(home) = home {
        let rooted = append_scope(home, scope);
        if !roots.iter().any(|r| r == &rooted) {
            roots.push(rooted);
        }
    }

    Ok(roots)
}

/// Compute the absolute roots implied by an implicit-relative token at the
/// given scope.
fn implicit_relative_completion_roots(
    scope: &str,
    base: &Path,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    let mut roots: Vec<PathBuf> = vec![append_scope(base, scope)];

    if let Some(git_root) = find_git_root(base)?
        && git_root.as_path() != base
    {
        let rooted = append_scope(&git_root, scope);
        if !roots.iter().any(|r| r == &rooted) {
            roots.push(rooted);
        }
    }

    Ok(roots)
}

/// Append a scope string to a root directory.
///
/// Strips the trailing `/` before joining so the result is a normal
/// directory path rather than one with an empty final component.
fn append_scope(root: &Path, scope: &str) -> PathBuf {
    let trimmed = scope.trim_end_matches('/');
    if trimmed.is_empty() {
        root.to_path_buf()
    } else {
        root.join(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_paths_same_dir() {
        let result = diff_paths(Path::new("/a/b/file.txt"), Path::new("/a/b")).unwrap();
        assert_eq!(result, PathBuf::from("file.txt"));
    }

    #[test]
    fn diff_paths_sibling_dir() {
        let result = diff_paths(Path::new("/a/b/file.txt"), Path::new("/a/c")).unwrap();
        assert_eq!(result, PathBuf::from("../b/file.txt"));
    }

    #[test]
    fn diff_paths_parent() {
        let result = diff_paths(Path::new("/a/file.txt"), Path::new("/a/b/c")).unwrap();
        assert_eq!(result, PathBuf::from("../../file.txt"));
    }

    #[test]
    fn diff_paths_same_path() {
        let result = diff_paths(Path::new("/a/b"), Path::new("/a/b")).unwrap();
        assert_eq!(result, PathBuf::from("."));
    }

    #[test]
    fn normalize_dotdot() {
        let result = normalize_components(Path::new("/a/b/../c/./d"));
        assert_eq!(result, PathBuf::from("/a/c/d"));
    }

    #[test]
    fn interpolate_literal_only() {
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: Some(PathBuf::from("/home/test")),
            env: std::collections::HashMap::new(),
            repository_root: None,
        };
        let template = PathTemplate {
            segments: vec![TemplateSegment::Literal("foo/bar.md".to_string())],
        };
        let result = interpolate(&template, &ctx).unwrap();
        assert_eq!(result, "foo/bar.md");
    }

    #[test]
    fn interpolate_with_env_var() {
        let mut env = std::collections::HashMap::new();
        env.insert("PROJECT".to_string(), "myproject".to_string());
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env,
            repository_root: None,
        };
        let template = PathTemplate {
            segments: vec![
                TemplateSegment::EnvVar("PROJECT".to_string()),
                TemplateSegment::Literal("/README.md".to_string()),
            ],
        };
        let result = interpolate(&template, &ctx).unwrap();
        assert_eq!(result, "myproject/README.md");
    }

    #[test]
    fn interpolate_missing_env_var() {
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env: std::collections::HashMap::new(),
            repository_root: None,
        };
        let template = PathTemplate {
            segments: vec![TemplateSegment::EnvVar("MISSING".to_string())],
        };
        let result = interpolate(&template, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn normalize_absolute_relative_path() {
        let result = normalize_absolute(Path::new("foo/bar.txt"), Path::new("/home/user"));
        assert_eq!(result, PathBuf::from("/home/user/foo/bar.txt"));
    }

    #[test]
    fn normalize_absolute_already_absolute() {
        let result = normalize_absolute(Path::new("/etc/config.toml"), Path::new("/home/user"));
        assert_eq!(result, PathBuf::from("/etc/config.toml"));
    }

    /// Extract just the paths from a collected root list for comparison.
    fn root_paths(roots: &[RootEntry]) -> Vec<PathBuf> {
        roots.iter().map(|r| r.path.clone()).collect()
    }

    #[test]
    fn implicit_relative_uses_cwd_then_git_root() {
        use crate::file_reference::{MagicPathList, ParsedReference, ReferenceKind};

        // With no repository root supplied, the only root is CWD, matching the
        // "git lookup returned None" branch.
        let parsed = ParsedReference {
            recursive: false,
            kind: ReferenceKind::ImplicitRelative(PathTemplate {
                segments: vec![TemplateSegment::Literal("nope.md".to_string())],
            }),
        };
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env: std::collections::HashMap::new(),
            repository_root: None,
        };
        let roots =
            collect_roots(&parsed.kind, &MagicPathList::default(), &[], &ctx, None).unwrap();
        assert_eq!(root_paths(&roots), vec![PathBuf::from("/tmp")]);
        assert_eq!(roots[0].provenance, RootProvenance::Source);
    }

    #[test]
    fn home_kind_uses_context_home_dir() {
        let parsed = ReferenceKind::Home(PathTemplate {
            segments: vec![TemplateSegment::Literal("cfg.toml".to_string())],
        });
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: Some(PathBuf::from("/home/test")),
            env: std::collections::HashMap::new(),
            repository_root: None,
        };
        let roots = collect_roots(&parsed, &MagicPathList::default(), &[], &ctx, None).unwrap();
        assert_eq!(root_paths(&roots), vec![PathBuf::from("/home/test")]);
        assert_eq!(roots[0].provenance, RootProvenance::Home);
    }

    #[test]
    fn home_kind_without_home_is_typed_missing_context() {
        let parsed = ReferenceKind::Home(PathTemplate {
            segments: vec![TemplateSegment::Literal("cfg.toml".to_string())],
        });
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env: std::collections::HashMap::new(),
            repository_root: None,
        };
        let err = collect_roots(&parsed, &MagicPathList::default(), &[], &ctx, None).unwrap_err();
        assert!(
            matches!(err, FileReferenceError::MissingHomeContext),
            "expected MissingHomeContext, got {err}"
        );
    }

    #[test]
    fn caller_supplied_repository_root_overrides_discovery() {
        // With a supplied root, collect_roots uses it directly and never
        // performs git discovery from cwd.
        let parsed = ReferenceKind::ImplicitRelative(PathTemplate {
            segments: vec![TemplateSegment::Literal("x.md".to_string())],
        });
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp/base"),
            home_dir: None,
            env: std::collections::HashMap::new(),
            repository_root: Some(PathBuf::from("/tmp/repo")),
        };
        let roots = collect_roots(
            &parsed,
            &MagicPathList::default(),
            &[],
            &ctx,
            ctx.repository_root.as_deref(),
        )
        .unwrap();
        assert_eq!(
            root_paths(&roots),
            vec![PathBuf::from("/tmp/base"), PathBuf::from("/tmp/repo")],
            "base stays first; supplied repo root is the second candidate",
        );
        assert_eq!(roots[0].provenance, RootProvenance::Source);
        assert_eq!(roots[1].provenance, RootProvenance::Repository);
    }

    #[test]
    fn classify_token_magic_empty_tail() {
        let (form, tail) = classify_token("@").unwrap();
        assert_eq!(form, CompletionEntryForm::Magic);
        assert_eq!(tail, "");
    }

    #[test]
    fn classify_token_magic_with_partial_name() {
        let (form, tail) = classify_token("@pr").unwrap();
        assert_eq!(form, CompletionEntryForm::Magic);
        assert_eq!(tail, "pr");
    }

    #[test]
    fn classify_token_magic_with_scope_and_partial() {
        let (form, tail) = classify_token("@prompts/p").unwrap();
        assert_eq!(form, CompletionEntryForm::Magic);
        assert_eq!(tail, "prompts/p");
    }

    #[test]
    fn classify_token_empty_string_is_implicit_relative() {
        let (form, tail) = classify_token("").unwrap();
        assert_eq!(form, CompletionEntryForm::ImplicitRelative);
        assert_eq!(tail, "");
    }

    #[test]
    fn classify_token_bare_subdir_is_implicit_relative() {
        let (form, tail) = classify_token("prompts/p").unwrap();
        assert_eq!(form, CompletionEntryForm::ImplicitRelative);
        assert_eq!(tail, "prompts/p");
    }

    #[test]
    fn classify_token_rejects_recursive() {
        assert!(classify_token("%foo").is_none());
        assert!(classify_token("%@foo").is_none());
    }

    #[test]
    fn classify_token_rejects_unsupported_forms() {
        assert!(classify_token("!foo").is_none());
        assert!(classify_token("/abs/path").is_none());
        assert!(classify_token("./rel").is_none());
        assert!(classify_token("../rel").is_none());
        assert!(classify_token(".").is_none());
        assert!(classify_token("..").is_none());
        assert!(classify_token("vault:notes").is_none());
        assert!(classify_token("{{DIR}}/x").is_none());
    }

    #[test]
    fn split_scope_and_active_no_separator() {
        assert_eq!(split_scope_and_active(""), ("", ""));
        assert_eq!(split_scope_and_active("pro"), ("", "pro"));
    }

    #[test]
    fn split_scope_and_active_with_separator() {
        assert_eq!(split_scope_and_active("prompts/"), ("prompts/", ""));
        assert_eq!(split_scope_and_active("prompts/ab"), ("prompts/", "ab"));
        assert_eq!(
            split_scope_and_active("a/b/c"),
            ("a/b/", "c"),
            "last slash is the split point"
        );
    }

    #[test]
    fn append_scope_trims_trailing_slash() {
        assert_eq!(
            append_scope(Path::new("/root"), "prompts/"),
            PathBuf::from("/root/prompts")
        );
        assert_eq!(append_scope(Path::new("/root"), ""), PathBuf::from("/root"));
        assert_eq!(
            append_scope(Path::new("/root"), "/"),
            PathBuf::from("/root"),
            "a lone slash reduces to the root itself"
        );
    }

    #[test]
    fn complete_partial_rejects_unsupported_forms() {
        let result = complete_partial("!foo", Path::new("/tmp")).unwrap();
        assert!(result.is_none());
        let result = complete_partial("vault:x", Path::new("/tmp")).unwrap();
        assert!(result.is_none());
        let result = complete_partial("./x", Path::new("/tmp")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn complete_partial_magic_bare_sigil_outside_repo() {
        // /tmp is not inside a git repo on most systems.
        let base = Path::new("/tmp");
        let result = complete_partial("@", base).unwrap().expect("supported");
        assert_eq!(result.entry_form(), CompletionEntryForm::Magic);
        assert_eq!(result.active_segment(), "");
        assert_eq!(result.rendered_prefix(), "@");
        // Roots include at most HOME; no git root.
        assert!(
            result.roots().len() <= 1,
            "no git root expected under /tmp, got {:?}",
            result.roots()
        );
    }

    #[test]
    fn complete_partial_implicit_relative_outside_repo() {
        let base = Path::new("/tmp");
        let result = complete_partial("prompts/p", base)
            .unwrap()
            .expect("supported");
        assert_eq!(result.entry_form(), CompletionEntryForm::ImplicitRelative);
        assert_eq!(result.active_segment(), "p");
        assert_eq!(result.rendered_prefix(), "prompts/");
        // No git root under /tmp, so only the base-derived root is present.
        assert_eq!(result.roots(), &[PathBuf::from("/tmp/prompts")]);
    }
}
