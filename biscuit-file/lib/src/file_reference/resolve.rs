use std::path::{Path, PathBuf};

use tracing::{debug, trace};
use walkdir::WalkDir;

use crate::file_reference::context::{
    ResolutionContext, find_git_root, find_package_area, home_dir,
};
use crate::file_reference::error::FileReferenceError;
use crate::file_reference::{
    CompletionEntryForm, MagicPathList, ParsedReference, PartialCompletion, PathTemplate,
    ReferenceKind, TemplateSegment, make_partial_completion,
};

#[cfg(feature = "url")]
use crate::file_reference::Resolved;

/// Resolve a parsed reference against runtime context.
pub(crate) fn resolve(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    #[cfg(feature = "url")]
    if matches!(parsed.kind, ReferenceKind::Url(_)) {
        let raw = interpolated_url_string(&parsed.kind, ctx)?;
        return Err(FileReferenceError::RemoteNotLocal(raw));
    }

    let interpolated = interpolate(parsed.kind.template(), ctx)?;

    if parsed.recursive {
        resolve_recursive(parsed, &interpolated, magic_paths, vault_roots, ctx)
    } else {
        resolve_direct(parsed, &interpolated, magic_paths, vault_roots, ctx)
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

/// Resolve without recursion -- check exact file paths.
fn resolve_direct(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    let candidates = build_candidates(parsed, interpolated, magic_paths, vault_roots, ctx)?;

    for candidate in &candidates {
        let exists = candidate.is_file();
        trace!(?candidate, exists, "checking candidate");
        if exists {
            debug!(?candidate, "resolved file reference");
            return Ok(Some(normalize_absolute(candidate, &ctx.cwd)));
        }
    }

    debug!("no candidate matched");
    Ok(None)
}

/// Resolve with recursive directory traversal.
fn resolve_recursive(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    let roots = build_search_roots(parsed, magic_paths, vault_roots, ctx)?;
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

    for root in &roots {
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

    // Sort lexicographically and return the first match
    matches.sort();
    debug!(match_count = matches.len(), "recursive search complete");
    Ok(matches.into_iter().next())
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

/// Collect search root directories for a given reference kind.
fn collect_roots(
    kind: &ReferenceKind,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    match kind {
        ReferenceKind::Relative(_) => Ok(vec![ctx.cwd.clone()]),
        ReferenceKind::ImplicitRelative(_) => {
            let mut roots = vec![ctx.cwd.clone()];
            if let Some(git_root) = resolve_repository_root(ctx)?
                && git_root != ctx.cwd
            {
                roots.push(git_root);
            }
            Ok(roots)
        }
        ReferenceKind::Absolute(_) => Ok(vec![PathBuf::from("/")]),
        ReferenceKind::Magic(_) => {
            let mut roots = Vec::new();
            roots.extend(magic_paths.prepend.iter().cloned());
            if let Some(git_root) = resolve_repository_root(ctx)? {
                roots.push(git_root);
            }
            if let Some(ref home) = ctx.home_dir {
                roots.push(home.clone());
            }
            roots.extend(magic_paths.append.iter().cloned());
            Ok(roots)
        }
        ReferenceKind::Package(_) => {
            let git_root = match resolve_repository_root(ctx)? {
                Some(root) => root,
                None => return Ok(vec![]),
            };
            let area = find_package_area(&git_root, &ctx.cwd)?;
            Ok(vec![area.unwrap_or(git_root)])
        }
        ReferenceKind::Home(_) => match &ctx.home_dir {
            Some(home) => Ok(vec![home.clone()]),
            None => Err(FileReferenceError::MissingHomeContext),
        },
        ReferenceKind::Vault(_) => {
            let mut roots: Vec<PathBuf> = vault_roots.to_vec();
            if let Some(vault_env) = ctx.env.get("VAULT") {
                roots.extend(std::env::split_paths(vault_env));
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

/// Build candidate file paths for non-recursive resolution.
fn build_candidates(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    if let ReferenceKind::Absolute(_) = &parsed.kind {
        let path = PathBuf::from(interpolated);
        if !path.is_absolute() {
            return Err(FileReferenceError::InvalidSyntax(format!(
                "absolute reference resolved to non-absolute path: {interpolated}"
            )));
        }
        return Ok(vec![path]);
    }

    #[cfg(feature = "url")]
    if matches!(parsed.kind, ReferenceKind::Url(_)) {
        return Ok(vec![]);
    }

    let roots = collect_roots(&parsed.kind, magic_paths, vault_roots, ctx)?;
    Ok(roots.into_iter().map(|r| r.join(interpolated)).collect())
}

/// Build search roots for recursive resolution.
fn build_search_roots(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    collect_roots(&parsed.kind, magic_paths, vault_roots, ctx)
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

    #[test]
    fn implicit_relative_uses_cwd_then_git_root() {
        use crate::file_reference::{MagicPathList, ParsedReference, ReferenceKind};

        // We can't call collect_roots for ImplicitRelative without a real git
        // context, so exercise the *direct* path by constructing a
        // ParsedReference with no git root. In that case the only root
        // should be CWD, matching the "git lookup returned None" branch.
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
        let roots = collect_roots(&parsed.kind, &MagicPathList::default(), &[], &ctx).unwrap();
        // /tmp has no git repo, so only CWD is returned.
        assert_eq!(roots, vec![PathBuf::from("/tmp")]);
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
        let roots = collect_roots(&parsed, &MagicPathList::default(), &[], &ctx).unwrap();
        assert_eq!(roots, vec![PathBuf::from("/home/test")]);
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
        let err = collect_roots(&parsed, &MagicPathList::default(), &[], &ctx).unwrap_err();
        assert!(
            matches!(err, FileReferenceError::MissingHomeContext),
            "expected MissingHomeContext, got {err}"
        );
    }

    #[test]
    fn caller_supplied_repository_root_overrides_discovery() {
        // With a supplied root, collect_roots must not perform live git
        // discovery from cwd -- it uses the supplied root directly.
        let parsed = ReferenceKind::ImplicitRelative(PathTemplate {
            segments: vec![TemplateSegment::Literal("x.md".to_string())],
        });
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp/base"),
            home_dir: None,
            env: std::collections::HashMap::new(),
            repository_root: Some(PathBuf::from("/tmp/repo")),
        };
        let roots = collect_roots(&parsed, &MagicPathList::default(), &[], &ctx).unwrap();
        assert_eq!(
            roots,
            vec![PathBuf::from("/tmp/base"), PathBuf::from("/tmp/repo")],
            "base stays first; supplied repo root is the second candidate",
        );
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
