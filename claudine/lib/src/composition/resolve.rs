//! File reference resolution for composition sources.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{
    DetailedOutcome, FileReference, FileResolutionContext, MagicPathTier, PathPosition,
    ResolutionFailure,
};
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot};
use darkmatter::markdown::{Markdown, MarkdownError};

use super::error::{CompositionError, MarkdownLoadCause};
use super::types::ResolvedCompositionSource;

/// Map a Markdown load failure to the most actionable `CompositionError`.
///
/// A malformed-frontmatter failure routes to [`CompositionError::FrontmatterParse`]
/// (which carries the typed error for rich rendering); any other failure
/// (e.g. a parse error from `try_from`) routes to
/// [`CompositionError::MarkdownLoad`] carrying the typed
/// [`MarkdownLoadCause`].
fn map_load_error(path: &Path, err: MarkdownError) -> CompositionError {
    match err {
        MarkdownError::FrontmatterParse { .. }
        | MarkdownError::FrontmatterFenceMismatch { .. } => CompositionError::FrontmatterParse(err),
        other => CompositionError::MarkdownLoad {
            path: path.to_path_buf(),
            source: MarkdownLoadCause::Parse(Box::new(other)),
        },
    }
}

/// Resolve a file reference string to a loaded Markdown document.
///
/// Uses `biscuit-file::FileReference` for all path resolution. Validates
/// that the resolved file has a `.md` or `.markdown` extension.
///
/// The reference resolves against the launch context
/// [`capture_file_resolution_context`] builds from `snapshot`. Canonical
/// Claudine command paths resolve with an invocation-owned
/// [`FileResolutionContext`] via [`resolve_composition_source_in_context`].
pub fn resolve_composition_source(
    file_ref: &str,
    snapshot: &RequestSnapshot,
) -> Result<ResolvedCompositionSource, CompositionError> {
    let context = capture_file_resolution_context(snapshot)?;
    resolve_composition_source_in_context(file_ref, &context)
}

/// Builds the launch file-resolution context for `snapshot`, with Claudine's
/// prompt conventions registered as extra `@` roots.
///
/// ## Notes
///
/// This helper discovers the repository at the snapshot's request directory
/// on every call. Canonical command paths instead create one
/// [`InvocationContext`](crate::invocation_context::InvocationContext), use
/// its launch projection for top-level resolution, and retain the definitive
/// source bundle returned after resolution.
///
/// ## Errors
///
/// [`CompositionError::ResolutionContext`] when the builder rejects the
/// context.
pub fn capture_file_resolution_context(
    snapshot: &RequestSnapshot,
) -> Result<FileResolutionContext, CompositionError> {
    let cwd = snapshot.request_dir();
    let git_root = sniff::filesystem::git::GitRepo::discover(cwd)
        .ok()
        .flatten()
        .map(|repo| repo.repo_root().to_path_buf());
    let repo_info = git_root
        .as_deref()
        .and_then(|root| sniff::filesystem::repo::detect_repo_structure(root).ok().flatten());
    Ok(build_prompt_resolution_context(snapshot, git_root.as_deref(), repo_info.as_ref())?)
}

/// Builds the launch file-resolution context for `snapshot` from a repository
/// the caller already observed, with Claudine's prompt conventions registered
/// as extra `@` roots.
///
/// `repository_root` is the repository containing the snapshot's request
/// directory and `repo_info` its topology, when known. The one builder shared
/// by composition, the invocation context, and shell completion, so a value
/// completion offers is one runtime resolves.
///
/// ## Errors
///
/// The builder's [`ContextBuildError`](darkmatter::markdown::compose::ContextBuildError)
/// when the context fails validation.
pub fn build_prompt_resolution_context(
    snapshot: &RequestSnapshot,
    repository_root: Option<&Path>,
    repo_info: Option<&sniff::filesystem::repo::RepoInfo>,
) -> Result<FileResolutionContext, darkmatter::markdown::compose::ContextBuildError> {
    crate::invocation_context::build_file_resolution_context(
        snapshot,
        None,
        repository_root,
        repo_info,
        None,
    )
}

/// Build the context for a top-level source in another repository.
///
/// The source's directory gets its own built context (its repository is
/// discovered there), keeping `provisional_context`'s home and environment,
/// and the launch `@` scope captured by `provisional_context` is carried into
/// it.
///
/// ## Notes
///
/// - Anchors repository-root discovery at `source_path.parent()`, never at
///   the launch directory: a top-level document selected from a different
///   repository keeps that repository's nested references rather than being
///   hijacked by wherever the binary was launched from (D2/D10, AC12).
/// - The prompt conventions are registered against the launch local root
///   rather than the source's repository (ruling 2 of
///   2026-09-23-local-before-home): a nested `@x.md` searches the launch
///   tree first, while `./`, bare, `&`, and `^` references keep the
///   source-specific anchors.
///
/// ## Errors
///
/// [`CompositionError::InvalidReference`] when `source_path` has no parent
/// directory (unreachable in practice, since the source arrives here already
/// resolved to an absolute path), and [`CompositionError::ResolutionContext`]
/// when the builder rejects the source's context.
pub fn derive_request_context_for_source(
    provisional_context: &FileResolutionContext,
    source_path: &Path,
) -> Result<FileResolutionContext, CompositionError> {
    let base_dir = source_path.parent().ok_or_else(|| CompositionError::InvalidReference {
        reference: biscuit_file::to_portable_string(source_path),
        source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
            "resolved source path has no parent directory: {}",
            biscuit_file::to_portable_string(source_path)
        )),
    })?;

    let git_root = sniff::filesystem::git::GitRepo::discover(base_dir)
        .ok()
        .flatten()
        .map(|repo| repo.repo_root().to_path_buf());
    let repo_info = git_root
        .as_deref()
        .and_then(|root| sniff::filesystem::repo::detect_repo_structure(root).ok().flatten());
    let snapshot = RequestSnapshot::new(base_dir)
        .with_home(provisional_context.home_dir().map(Path::to_path_buf))
        .with_env(provisional_context.env().clone());
    Ok(crate::invocation_context::build_file_resolution_context(
        &snapshot,
        Some(source_path),
        git_root.as_deref(),
        repo_info.as_ref(),
        Some(provisional_context.launch_magic_scope()),
    )?)
}

/// Resolves and loads a top-level composition source using a previously
/// captured request snapshot.
pub fn resolve_composition_source_in_context(
    file_ref: &str,
    context: &FileResolutionContext,
) -> Result<ResolvedCompositionSource, CompositionError> {
    // Phase 3 (2026-05-09-slow-prep): instrument the file-reference
    // resolution phase so trace inspection / `--perf` reporting can see
    // when the `biscuit-file` resolver dominates compose prep cost.
    let _span = tracing::info_span!("compose_prep.file_reference", file = %file_ref).entered();
    let reference = FileReference::new(file_ref).map_err(|source| {
        CompositionError::InvalidReference {
            reference: file_ref.to_string(),
            source,
        }
    })?;

    let detailed = reference.resolve_detailed(context);
    let resolved_path = match detailed.outcome() {
        DetailedOutcome::Matched(path) => path.clone(),
        DetailedOutcome::Failed(ResolutionFailure::NoMatch) => {
            return Err(CompositionError::from_detailed_no_match(&detailed, context));
        }
        DetailedOutcome::Failed(_) => {
            let source = match detailed.into_convenience() {
                Err(source) => source,
                Ok(_) => biscuit_file::FileReferenceError::InvalidSyntax(format!(
                    "resolver returned a failed outcome without a typed error for `{file_ref}`"
                )),
            };
            return Err(CompositionError::InvalidReference {
                reference: file_ref.to_string(),
                source,
            });
        }
    };

    // Validate markdown extension
    let ext = resolved_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown") {
        return Err(CompositionError::NotMarkdown(
            biscuit_file::to_portable_string(&resolved_path),
        ));
    }

    let original_text = fs::read_to_string(&resolved_path).map_err(|e| {
        CompositionError::MarkdownLoad {
            path: resolved_path.clone(),
            source: MarkdownLoadCause::Read(e),
        }
    })?;

    // Parse fallibly so malformed frontmatter surfaces as a real error. The
    // infallible `From<String>` drops a `FrontmatterParse` error and returns an
    // empty-frontmatter document, which downstream looks like a *missing*
    // `prompt` property — hiding the actual YAML syntax error from the user.
    let markdown = Markdown::try_from(resolved_path.as_path())
        .map_err(|e| map_load_error(&resolved_path, e))?
        .with_source(ComposeSource::File(resolved_path.clone()));

    Ok(ResolvedCompositionSource {
        original_ref: file_ref.to_string(),
        resolved_path,
        original_text,
        markdown,
    })
}

/// Whether `path` is a YAML file, which composition loads as frontmatter with
/// an empty body rather than as Markdown.
pub fn is_yaml_source(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "yaml" | "yml"))
}

/// Load a YAML file as a frontmatter-only document with an empty body.
///
/// This is the second composition entry mode: a `kind: sequence` YAML file
/// invoked directly is one document whose *root mapping is its frontmatter*, so
/// `sequence`, `prompt`, `$schema`, and every other key behave exactly as they
/// would inside a Markdown frontmatter block.
///
/// It lives here rather than beside the CLI's sequence command because the
/// just-in-time re-read ([`reload_composition_source`]) must reach the identical
/// document the initial resolution produced. Loading a YAML source as plain
/// Markdown yields an empty frontmatter, which downstream is indistinguishable
/// from a document that simply declared nothing.
///
/// ## Errors
///
/// Returns [`CompositionError::MarkdownLoad`] when the file cannot be read or
/// parsed as YAML, and [`CompositionError::SequenceExternalWrongType`] when its
/// root is not a mapping.
pub fn load_yaml_document(path: &Path) -> Result<Markdown, CompositionError> {
    let yaml = biscuit_file::Yaml::new(path).map_err(|e| CompositionError::MarkdownLoad {
        path: path.to_path_buf(),
        source: MarkdownLoadCause::Yaml(e),
    })?;
    let json_value = yaml.as_json().map_err(|e| CompositionError::MarkdownLoad {
        path: path.to_path_buf(),
        source: MarkdownLoadCause::Yaml(e),
    })?;
    let root = json_value.as_object().ok_or_else(|| {
        CompositionError::SequenceExternalWrongType(
            "YAML sequence file root must be an object".to_string(),
        )
    })?;

    let mut frontmatter = darkmatter::markdown::Frontmatter::new();
    for (key, value) in root {
        frontmatter
            .insert(key, value.clone())
            .map_err(|e| CompositionError::MarkdownLoad {
                path: path.to_path_buf(),
                source: MarkdownLoadCause::Parse(Box::new(e)),
            })?;
    }

    // A whole-file YAML source has no `---` block for Darkmatter to collect
    // order from, so the order index comes from the document this frontmatter
    // was projected out of.
    let frontmatter = frontmatter
        .with_mapping_orders(darkmatter::markdown::MappingOrders::collect(yaml.value()));

    Ok(darkmatter::markdown::Markdown::with_frontmatter(
        frontmatter,
        "",
    ))
}

/// Frontmatter keys that belong to a *formal sequence document* rather than to
/// the composition frontmatter of the document carrying them.
///
/// Referenced through `sequence: steps.yaml` these keys never reach the
/// invoking document's frontmatter at all; a directly invoked YAML document has
/// only one mapping to put them in, so they must be lifted back out before
/// composition — otherwise Darkmatter's always-on `$schema` stage would judge
/// each step's *state* schema against the document root, which cannot satisfy
/// it. The sequence plan has already consumed both by this point.
const FORMAL_SEQUENCE_KEYS: [&str; 2] = ["$schema", "template"];

/// Return `source` with [`FORMAL_SEQUENCE_KEYS`] removed when it is a directly
/// invoked formal sequence document, and unchanged otherwise.
#[must_use]
pub fn without_formal_sequence_keys(
    source: &ResolvedCompositionSource,
) -> ResolvedCompositionSource {
    if !super::sequence::formal::is_direct_formal_document(source) {
        return source.clone();
    }

    let mut frontmatter = darkmatter::markdown::Frontmatter::new();
    for (key, value) in source.markdown.frontmatter().as_map() {
        if FORMAL_SEQUENCE_KEYS.contains(&key.as_str()) {
            continue;
        }
        // The values round-trip from a frontmatter that already accepted them,
        // so a rejection here is not reachable; keeping the key is strictly
        // better than panicking on an unreachable branch.
        let _ = frontmatter.insert(key, value.clone());
    }
    // Dropping two root keys does not move anything the order index points at,
    // so it travels with the rebuilt frontmatter.
    let frontmatter =
        frontmatter.with_mapping_orders(source.markdown.frontmatter().mapping_orders().clone());

    ResolvedCompositionSource {
        original_ref: source.original_ref.clone(),
        resolved_path: source.resolved_path.clone(),
        original_text: source.original_text.clone(),
        markdown: Markdown::with_frontmatter(frontmatter, source.markdown.content()),
    }
}

/// Re-read an already-resolved source from disk.
///
/// Sequence steps compose just in time, so each step reads the file as it
/// stands at its turn — an earlier step's inline-compose write-back or an
/// agent's mid-run frontmatter edit is visible to every later step. The
/// reference is *not* re-resolved: the path was decided once, and re-running
/// magic-root discovery mid-sequence could silently retarget the run.
///
/// A YAML source reloads through [`load_yaml_document`], the same conversion
/// its initial resolution used, and then sheds its formal sequence keys
/// ([`without_formal_sequence_keys`]) so every step composes the same document
/// shape the first one did.
///
/// ## Errors
///
/// Returns [`CompositionError::MarkdownLoad`] when the file has become
/// unreadable or its frontmatter no longer parses.
pub fn reload_composition_source(
    source: &ResolvedCompositionSource,
) -> Result<ResolvedCompositionSource, CompositionError> {
    let original_text =
        fs::read_to_string(&source.resolved_path).map_err(|e| CompositionError::MarkdownLoad {
            path: source.resolved_path.clone(),
            source: MarkdownLoadCause::Read(e),
        })?;
    let markdown = if is_yaml_source(&source.resolved_path) {
        load_yaml_document(&source.resolved_path)?
    } else {
        Markdown::try_from(source.resolved_path.as_path())
            .map_err(|e| map_load_error(&source.resolved_path, e))?
    };

    Ok(without_formal_sequence_keys(&ResolvedCompositionSource {
        original_ref: source.original_ref.clone(),
        resolved_path: source.resolved_path.clone(),
        original_text,
        markdown,
    }))
}

/// Parse a top-level Claudine prompt argument with the shared file-reference
/// grammar.
///
/// Request capture registers Claudine's convention roots on the associated
/// [`FileResolutionContext`]. Keeping parsing independent of ambient discovery
/// prevents this builder from creating a second, request-unstable root order.
///
/// ## Errors
///
/// Returns [`CompositionError::InvalidReference`] when the reference string is
/// syntactically invalid.
pub fn build_prompt_reference(file_ref: &str) -> Result<FileReference, CompositionError> {
    FileReference::new(file_ref).map_err(|e| CompositionError::InvalidReference {
        reference: file_ref.to_string(),
        source: e,
    })
}

/// The ordered magic roots shared by composition completion and execution.
///
/// Roots are **closest-first**: the discrete package's prompt directory, the
/// package-area prompt directory, the local tree's prompt/document/peer-skill
/// conventions, then the user prompt directory. The local rows anchor on
/// `local_root` — the launch repository root when one exists, otherwise the
/// launch directory itself, so a plain directory registers the same
/// convention rows a repository does. Bare package, package-area,
/// local-root, and home roots are intrinsic `@` scopes supplied by
/// [`FileResolutionContext`] and are intentionally not registered again.
///
/// This function is pure; callers are responsible for discovering the anchors
/// once for their request.
#[must_use]
pub fn prompt_magic_roots(
    local_root: &Path,
    package_area: Option<&Path>,
    package: Option<&Path>,
    home: Option<&Path>,
) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(package) = package {
        push_unique_root(&mut roots, package.join("prompts"));
    }
    if let Some(area) = package_area {
        push_unique_root(&mut roots, area.join("prompts"));
    }
    push_unique_root(&mut roots, local_root.join("prompts"));
    push_unique_root(&mut roots, local_root.join(".claudine").join("prompts"));
    push_unique_root(&mut roots, local_root.join("docs"));
    for peer in [
        ".claude", ".codex", ".gemini", ".opencode", ".goose", ".qwen", ".kimi",
    ] {
        push_unique_root(&mut roots, local_root.join(peer).join("skills"));
    }
    if let Some(home) = home {
        push_unique_root(&mut roots, home.join(".claudine").join("prompts"));
    }
    roots
}

/// The bare `.claudine` roots, searched after every intrinsic `@` scope of
/// their tier.
///
/// These let the path-shaped `@prompts/<x>` form reach the local and user
/// Claudine prompt tiers, which the concise `@<x>` form reaches through
/// [`prompt_magic_roots`]. Without them `@prompts/<x>` resolves only where a
/// package, area, or local root happens to contain `prompts/`, so it fails
/// in a local tree with no `prompts/` directory and outside any repository.
/// They are appended, not prepended, so a closer `prompts/<x>` still wins.
#[must_use]
pub fn prompt_magic_fallback_roots(local_root: &Path, home: Option<&Path>) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    push_unique_root(&mut roots, local_root.join(".claudine"));
    if let Some(home) = home {
        push_unique_root(&mut roots, home.join(".claudine"));
    }
    roots
}

/// Register Claudine's prompt conventions as a request snapshot's extra `@`
/// roots, in the order the builder registers them.
///
/// The single registration point shared by composition resolution, the
/// invocation context, and shell completion, so a value completion offers
/// is one runtime resolves. Convention rows anchor on `local_root` — the
/// launch repository root when one exists, otherwise the launch directory —
/// so the local tree registers the same prompt conventions whether or not it
/// is a repository.
///
/// The two `~/.claudine` rows are registered with an explicit user-tier
/// override: they are user conventions wherever the launch tree lies,
/// including when the local root is or contains `$HOME`, where containment
/// inference alone could not tell them from local rows. When the local root
/// is `$HOME` itself, its `.claudine` convention rows are the user rows and
/// are dropped instead of registered inferred — the chain deduplicates by
/// path with the local tier first, so an inferred twin would reclassify the
/// user convention as local. That overlap is matched by directory identity
/// as well as spelling: the launch directory arrives in its physical
/// spelling while `$HOME` keeps the environment's, so under a symlinked home
/// (macOS `/var` → `/private/var`, a Windows 8.3 alias) the same directory
/// is spelled two ways. Any other local root keeps every row, including a
/// `<root>/.claudine` that is a symlink to `~/.claudine`: tier follows the
/// lexical path (spec R2), so such a row stays local.
#[must_use]
pub fn with_prompt_magic_roots(
    mut snapshot: RequestSnapshot,
    local_root: &Path,
    package_area: Option<&Path>,
    package: Option<&Path>,
    home: Option<&Path>,
) -> RequestSnapshot {
    let user_prompt_root = home.map(|home| home.join(".claudine").join("prompts"));
    let user_fallback_root = home.map(|home| home.join(".claudine"));
    let local_root_is_home = home.is_some_and(|home| {
        home == local_root
            || fs::canonicalize(home)
                .is_ok_and(|physical| fs::canonicalize(local_root).is_ok_and(|local| local == physical))
    });
    // Physical identity decides only whether the local root is `$HOME`; a
    // row is never classified by where a symlink points (spec R2).
    let is_user_row = |root: &Path| {
        local_root_is_home
            && (root == local_root.join(".claudine")
                || root == local_root.join(".claudine").join("prompts"))
    };

    // Local-tier rows, registered inferred: their tier follows from
    // containment in the local root, and every row here lies inside it.
    for root in prompt_magic_roots(local_root, package_area, package, None) {
        if is_user_row(&root) {
            continue;
        }
        snapshot = snapshot.with_magic_root(root, PathPosition::Start);
    }
    for root in prompt_magic_fallback_roots(local_root, None) {
        if is_user_row(&root) {
            continue;
        }
        snapshot = snapshot.with_magic_root(root, PathPosition::End);
    }
    // User-tier rows, registered with the explicit override so they stay
    // behind every local-tier candidate in every layout.
    if let Some(root) = user_prompt_root {
        snapshot = snapshot.with_magic_root_tier(root, PathPosition::Start, MagicPathTier::User);
    }
    if let Some(root) = user_fallback_root {
        snapshot = snapshot.with_magic_root_tier(root, PathPosition::End, MagicPathTier::User);
    }
    snapshot
}

fn push_unique_root(roots: &mut Vec<PathBuf>, root: PathBuf) {
    if !roots.iter().any(|candidate| candidate == &root) {
        roots.push(root);
    }
}

/// Enrich a source-load error with the authored frontmatter block.
///
/// Source-load failures can happen after the file has resolved and been read
/// but before a [`ResolvedCompositionSource`] exists. This helper reconstructs
/// the resolved source text through `context` for the CLI render boundary and
/// leaves the error unchanged when the file cannot be resolved/read again or
/// the error is not frontmatter-rooted.
pub fn enrich_composition_source_load_error_in_context(
    file_ref: &str,
    error: CompositionError,
    stderr_is_tty: bool,
    context: &FileResolutionContext,
) -> CompositionError {
    let Some(source_text) = read_source_text_for_enrichment_in_context(file_ref, context) else {
        return error;
    };
    error.enrich_frontmatter_text(&source_text, stderr_is_tty)
}

fn read_source_text_for_enrichment_in_context(
    file_ref: &str,
    context: &FileResolutionContext,
) -> Option<String> {
    let reference = FileReference::new(file_ref).ok()?;
    let resolved_path = reference.resolve_in_context(context).ok()??;
    let ext = resolved_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown") {
        return None;
    }
    fs::read_to_string(resolved_path).ok()
}

/// Validate that the resolved file is readable and writable.
///
/// This is a cross-provider pre-flight check: regardless of which agent
/// is used, the inline composition workflow requires the agent to read
/// the file (to understand context) and write back (to update the body).
pub fn validate_file_permissions(path: &Path) -> Result<(), CompositionError> {
    // Try opening for write — the most reliable cross-platform method,
    // delegating the actual permission decision to the OS.
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| CompositionError::InsufficientFilePermissions {
            path: path.to_path_buf(),
            source: e,
        })?;

    Ok(())
}

/// Resolve a fixture document without the ambient repository walk.
///
/// [`resolve_composition_source`] first captures the *process* working
/// directory's repository topology. Under `cargo nextest` that directory is the
/// monorepo checkout, so every call performs a full 35-member package scan —
/// measured at 0.23 s per call against 0.01 s from outside a repository, paid
/// once per test process.
///
/// That topology decides nothing for a fixture: `path` is an absolute path to a
/// file the test just wrote, so it resolves identically from any anchor, and
/// [`ResolvedCompositionSource`] carries no context forward — every downstream
/// stage re-anchors on the document's own parent. Tests whose subject *is*
/// discovery build their own repository and keep the ambient entry point.
///
/// ## Panics
///
/// Panics when `path` is relative. A relative fixture reference would resolve
/// against the synthetic anchor rather than the process CWD, which is a
/// different question from the one the ambient entry point answers — failing
/// loudly is the only way that difference cannot pass silently.
#[cfg(test)]
pub(crate) fn resolve_fixture_source(
    path: &str,
) -> Result<ResolvedCompositionSource, CompositionError> {
    let anchor = Path::new(path);
    assert!(
        anchor.is_absolute(),
        "resolve_fixture_source needs an absolute fixture path; got `{path}`. A relative \
         reference resolves against the ambient CWD, so use resolve_composition_source."
    );
    let context = FileResolutionContext::new(
        anchor
            .parent()
            .expect("an absolute path with a file name has a parent"),
    );
    resolve_composition_source_in_context(path, &context)
}

/// Validate that a path has a markdown extension.
#[allow(dead_code)]
pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
