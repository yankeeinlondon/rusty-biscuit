use std::collections::HashSet;
use std::path::PathBuf;

use biscuit_file::{FileReference, FileResolutionContext};

use crate::system_prompt::context::LaunchContext;
use crate::system_prompt::types::*;

const STANDARD_FILENAME: &str = "system-prompt.md";
const NON_INTERACTIVE_FILENAME: &str = "non-interactive.md";

/// Resolve the effective system prompt source.
///
/// If explicit args are provided, resolves only the named file and
/// skips standard discovery. Otherwise searches the launch context
/// hierarchy for `system-prompt.md`.
///
/// ## Returns
///
/// Returns `Ok(Some((source, text)))` if a prompt file is found, `Ok(None)` if
/// no file exists in the search path, or `Err` if file I/O fails.
///
/// ## Errors
///
/// Returns `ClaudineError::SystemPromptFileNotFound` if an explicit file path
/// does not exist. Returns `ClaudineError::Io` if reading any file fails.
///
/// ## Examples
///
/// ```no_run
/// use claudine::system_prompt::context::LaunchContext;
/// use claudine::system_prompt::types::SystemPromptArgs;
/// use claudine::system_prompt::resolve::resolve_system_prompt_source;
/// use std::path::PathBuf;
///
/// let args = SystemPromptArgs::default();
/// let context = LaunchContext {
///     agent: None,
///     cwd: PathBuf::from("/workspace"),
///     repo_root: Some(PathBuf::from("/workspace")),
///     package_area_root: None,
///     package_root: None,
/// };
///
/// let result = resolve_system_prompt_source(&args, &context)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn resolve_system_prompt_source(
    args: &SystemPromptArgs,
    context: &LaunchContext,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    resolve_system_prompt_source_with_home(args, context, dirs::home_dir().as_deref())
}

/// Resolve a system-prompt source from one invocation's frozen launch facts.
pub(crate) fn resolve_system_prompt_source_with_invocation(
    args: &SystemPromptArgs,
    context: &LaunchContext,
    invocation: &crate::invocation_context::InvocationContext,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    if let Some(ref file) = args.append_file {
        return resolve_explicit_file_with_context(
            file,
            SystemPromptMode::Append,
            invocation.launch_file_resolution_context(),
        );
    }
    if let Some(ref file) = args.replace_file {
        return resolve_explicit_file_with_context(
            file,
            SystemPromptMode::Replace,
            invocation.launch_file_resolution_context(),
        );
    }
    discover_standard_file(context, invocation.home_dir())
}

fn resolve_explicit_file_with_context(
    file_ref: &str,
    mode: SystemPromptMode,
    resolution_context: &FileResolutionContext,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    let not_found =
        || crate::error::ClaudineError::SystemPromptFileNotFound(file_ref.to_string());
    let reference = FileReference::new(file_ref).map_err(|_| not_found())?;
    let path = reference
        .resolve_in_context(resolution_context)
        .map_err(|_| not_found())?
        .ok_or_else(not_found)?;
    let text = std::fs::read_to_string(&path)?;
    Ok(Some((SystemPromptSource::ExplicitFile { path, mode }, text)))
}

fn resolve_system_prompt_source_with_home(
    args: &SystemPromptArgs,
    context: &LaunchContext,
    home_dir: Option<&std::path::Path>,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    // 1. Explicit --append-system-prompt
    if let Some(ref file) = args.append_file {
        let path = resolve_file_ref(file, context, home_dir)?;
        let text = std::fs::read_to_string(&path)?;
        return Ok(Some((
            SystemPromptSource::ExplicitFile {
                path,
                mode: SystemPromptMode::Append,
            },
            text,
        )));
    }

    // 2. Explicit --replace-system-prompt
    if let Some(ref file) = args.replace_file {
        let path = resolve_file_ref(file, context, home_dir)?;
        let text = std::fs::read_to_string(&path)?;
        return Ok(Some((
            SystemPromptSource::ExplicitFile {
                path,
                mode: SystemPromptMode::Replace,
            },
            text,
        )));
    }

    // 3. Standard discovery
    discover_standard_file(context, home_dir)
}

/// Discover a `system-prompt.md` by filename over the launch context's known
/// anchors (package → package-area → repo, then user home).
///
/// This is filename **discovery** over a fixed set of directories, not
/// file-reference *resolution* (D5): it never parses `@`/`~`/`vault:` grammar
/// and never consults [`FileReference`]. It is deliberately distinct from
/// [`resolve_file_ref`], which resolves an author-supplied reference string.
fn discover_standard_file(
    context: &LaunchContext,
    home_dir: Option<&std::path::Path>,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    // Search local scopes in precedence order
    let scope_dirs = build_scope_list(context);
    for (dir, scope) in &scope_dirs {
        let candidate = dir.join(STANDARD_FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)?;
            return Ok(Some((
                SystemPromptSource::StandardDiscovered {
                    path: candidate,
                    scope: *scope,
                },
                text,
            )));
        }
    }

    // User-home fallback
    let user_home = home_dir.map(|h| h.join(".claudine").join(STANDARD_FILENAME));
    if let Some(ref home_path) = user_home
        && home_path.is_file()
    {
        let text = std::fs::read_to_string(home_path)?;
        return Ok(Some((
            SystemPromptSource::StandardDiscovered {
                path: home_path.clone(),
                scope: StandardPromptScope::User,
            },
            text,
        )));
    }

    Ok(None)
}

/// Resolve candidate prompt texts for non-interactive safety instructions.
///
/// The returned list is ordered by precedence and always ends with the
/// built-in fallback instructions.
pub fn resolve_non_interactive_candidates(
    context: &LaunchContext,
) -> Result<Vec<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    resolve_non_interactive_candidates_with_home(context, dirs::home_dir().as_deref())
}

pub(crate) fn resolve_non_interactive_candidates_with_invocation(
    context: &LaunchContext,
    invocation: &crate::invocation_context::InvocationContext,
) -> Result<Vec<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    resolve_non_interactive_candidates_with_home(context, invocation.home_dir())
}

fn resolve_non_interactive_candidates_with_home(
    context: &LaunchContext,
    home_dir: Option<&std::path::Path>,
) -> Result<Vec<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    let mut candidates = Vec::with_capacity(3);

    if let Some(repo_root) = &context.repo_root {
        let repo_path = repo_root.join(".claudine").join(NON_INTERACTIVE_FILENAME);
        if repo_path.is_file() {
            candidates.push((
                SystemPromptSource::NonInteractiveFile {
                    path: repo_path.clone(),
                    scope: StandardPromptScope::Repo,
                },
                std::fs::read_to_string(&repo_path)?,
            ));
        }
    }

    if let Some(home_path) =
        home_dir.map(|h| h.join(".claudine").join(NON_INTERACTIVE_FILENAME))
        && home_path.is_file()
    {
        candidates.push((
            SystemPromptSource::NonInteractiveFile {
                path: home_path.clone(),
                scope: StandardPromptScope::User,
            },
            std::fs::read_to_string(&home_path)?,
        ));
    }

    candidates.push((
        SystemPromptSource::BuiltInNonInteractive,
        DEFAULT_NON_INTERACTIVE_SYSTEM_PROMPT.to_string(),
    ));

    Ok(candidates)
}

/// Build the local scope search list from the launch context.
///
/// When inside a repo/monorepo, returns package -> package-area -> repo.
/// When outside a repo, returns CWD only.
fn build_scope_list(context: &LaunchContext) -> Vec<(PathBuf, StandardPromptScope)> {
    if context.repo_root.is_some() {
        let mut list = Vec::with_capacity(3);
        let mut seen = HashSet::new();
        if let Some(ref p) = context.package_root
            && seen.insert(p.clone())
        {
            list.push((p.clone(), StandardPromptScope::Package));
        }
        if let Some(ref p) = context.package_area_root
            && seen.insert(p.clone())
        {
            list.push((p.clone(), StandardPromptScope::PackageArea));
        }
        if let Some(ref p) = context.repo_root
            && seen.insert(p.clone())
        {
            list.push((p.clone(), StandardPromptScope::Repo));
        }
        list
    } else {
        vec![(context.cwd.clone(), StandardPromptScope::CurrentDirectory)]
    }
}

/// Resolve a `--append`/`--replace-system-prompt` file reference against the
/// launch context.
///
/// Delegates all grammar and candidate ordering to [`FileReference`] and the
/// shared [`FileResolutionContext`] rather than the former absolute-or-`cwd.join`
/// grammar: implicit references are launch-relative then repository-relative,
/// `@foo` is a magic-root search, `&foo` pins to the repository root, `^foo`
/// searches package, package-area, then repository roots, `~`/`~/foo` is
/// home-pinned (`~user` unsupported), explicit `./`/`../` stay pinned to the launch directory, and an
/// absolute path resolves to itself. The launch context has no source document,
/// so its directory is the base and the caller-supplied repository root anchors
/// implicit references after the launch directory. Resolution probes the
/// filesystem, so only an existing regular file is a match.
///
/// ## Errors
///
/// Returns [`ClaudineError::SystemPromptFileNotFound`](crate::error::ClaudineError::SystemPromptFileNotFound)
/// when the reference is malformed or resolves to no existing file.
fn resolve_file_ref(
    file_ref: &str,
    context: &LaunchContext,
    home_dir: Option<&std::path::Path>,
) -> Result<PathBuf, crate::error::ClaudineError> {
    let not_found =
        || crate::error::ClaudineError::SystemPromptFileNotFound(file_ref.to_string());

    let reference = FileReference::new(file_ref).map_err(|_| not_found())?;

    let mut resolution_ctx = match home_dir {
        Some(home_dir) => FileResolutionContext::new(context.cwd.clone()).with_home_dir(home_dir),
        None => FileResolutionContext::new(context.cwd.clone()).without_home_dir(),
    };
    if let Some(root) = context
        .repo_root
        .as_deref()
        .filter(|root| context.cwd.starts_with(root))
    {
        resolution_ctx = resolution_ctx.with_repository_root(root);
    }

    reference
        .resolve_in_context(&resolution_ctx)
        .map_err(|_| not_found())?
        .ok_or_else(not_found)
}

#[cfg(test)]
mod tests;
