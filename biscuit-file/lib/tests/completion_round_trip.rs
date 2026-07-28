//! D9 completion-to-execution round trip.
//!
//! The specification requires that a value dynamic completion emits "must
//! resolve through the same candidate builder in the same context" — completion
//! must not teach one precedence while execution uses another. These tests close
//! the review gap that "no test takes an emitted completion value and executes
//! it unchanged."
//!
//! Each test emulates a completion consumer: it calls
//! [`FileReference::complete_partial_in_context`], enumerates the emitted roots
//! in order to pick the file the user would *see*, reconstructs the token that
//! would be inserted, then resolves that token through
//! [`FileReference::resolve_in_context`] against the **same**
//! [`FileResolutionContext`]. The file completion displayed and the file
//! execution resolves must be identical — proving the two share one root builder
//! and one context rather than agreeing by coincidence.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceError, FileResolutionContext, PathPosition};
use tempfile::TempDir;

/// Initialize a fresh git repository at `path` using gix (no system git binary).
fn git_init(path: &Path) {
    gix::init(path).expect("git init failed");
}

/// Canonicalize into the spelling the resolver reports.
///
/// `std::fs::canonicalize` returns a `\\?\` verbatim path on Windows; the
/// resolver reduces every anchor to the legacy form before joining, so
/// expectations must be built in that same form.
fn canonical(path: &Path) -> PathBuf {
    dunce::canonicalize(path).expect("canonicalize")
}

/// The file a completion consumer would display, plus the token it would insert.
struct Emitted {
    /// The absolute path of the file the consumer enumerated and displayed.
    displayed: PathBuf,
    /// The reference string the shell would insert for that file.
    token: String,
}

/// Emulate a completion consumer over an emitted [`PartialCompletion`].
///
/// Walks the emitted roots in order, returning the first file whose name starts
/// with the active segment, together with the token a consumer would insert
/// (`rendered_prefix` + basename). Roots are enumerated in the exact order the
/// builder produced them, so "first displayed" mirrors the ranking a real
/// consumer sees before it emits a value.
fn pick_first_candidate(token: &str, ctx: &FileResolutionContext) -> Emitted {
    let completion = FileReference::complete_partial_in_context(token, ctx)
        .expect("completion expansion")
        .expect("supported entry form");

    for root in completion.roots() {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if entry.file_type().ok()?.is_file() {
                    entry.file_name().to_str().map(str::to_string)
                } else {
                    None
                }
            })
            .filter(|name| name.starts_with(completion.active_segment()))
            .collect();
        names.sort();
        if let Some(name) = names.into_iter().next() {
            return Emitted {
                displayed: root.join(&name),
                token: format!("{}{name}", completion.rendered_prefix()),
            };
        }
    }
    panic!("no completion candidate matched token `{token}`");
}

/// An implicit-relative value emitted by completion resolves to the same file
/// completion displayed — repository-first, even when a same-named decoy exists
/// under the base directory.
#[test]
fn implicit_relative_completion_value_resolves_to_the_displayed_file() {
    let tmp = TempDir::new().unwrap();
    let repo_root = canonical(tmp.path());
    git_init(&repo_root);

    // The repository-root target and a same-named decoy under the base dir, so
    // completion and execution must agree on *which* one wins.
    fs::create_dir_all(repo_root.join("prompts")).unwrap();
    fs::write(repo_root.join("prompts/plan.md"), b"repo").unwrap();
    let base = repo_root.join("pkg");
    fs::create_dir_all(base.join("prompts")).unwrap();
    fs::write(base.join("prompts/plan.md"), b"base decoy").unwrap();

    let ctx = FileResolutionContext::new(base.clone())
        .with_source_path(base.join("router.md"))
        .with_repository_root(repo_root.clone());

    let emitted = pick_first_candidate("prompts/pl", &ctx);
    assert_eq!(
        emitted.token, "prompts/plan.md",
        "the consumer inserts the scope + basename verbatim",
    );
    // Completion enumerates the repository root first, so the displayed file is
    // the repository-root twin, not the base decoy.
    assert_eq!(emitted.displayed, repo_root.join("prompts/plan.md"));

    let resolved = FileReference::new(&emitted.token)
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();
    assert_eq!(
        resolved.as_deref(),
        Some(emitted.displayed.as_path()),
        "the emitted implicit value must execute to the file completion displayed",
    );
}

/// The published completion order for an implicit-relative token is the
/// repository root first, then the base directory (Phase 4 precedence). This
/// pins the corrected `CompletionEntryForm::ImplicitRelative` and
/// `complete_partial` rustdoc, which previously advertised the reverse
/// (`{base, git_root}`).
#[test]
fn implicit_relative_completion_roots_are_repository_first() {
    let tmp = TempDir::new().unwrap();
    let repo_root = canonical(tmp.path());
    let base = repo_root.join("prompts");
    fs::create_dir_all(&base).unwrap();

    let ctx = FileResolutionContext::new(base.clone()).with_repository_root(repo_root.clone());

    let completion = FileReference::complete_partial_in_context("sub/p", &ctx)
        .expect("completion expansion")
        .expect("supported entry form");

    assert_eq!(
        completion.roots(),
        &[repo_root.join("sub"), base.join("sub")],
        "implicit-relative completion enumerates the repository root before the base",
    );
}

/// A magic value emitted by completion resolves to the same file completion
/// displayed, using a magic root configured only on the context — the root the
/// ambient completer cannot see. This is the review's named divergence:
/// configured magic roots must be shared between completion and execution.
#[test]
fn magic_completion_value_resolves_through_a_shared_configured_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = canonical(tmp.path());
    git_init(&repo_root);

    // `plan.md` lives ONLY under the configured magic root, never directly under
    // the repository root or home. The ambient completer (roots = repo, home)
    // would find nothing; only a context that shares the configured magic root
    // can both display and resolve it.
    let magic_root = repo_root.join("prompts");
    fs::create_dir_all(&magic_root).unwrap();
    fs::write(magic_root.join("plan.md"), b"magic").unwrap();

    let empty_home = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(repo_root.clone())
        .with_repository_root(repo_root.clone())
        .with_home_dir(canonical(empty_home.path()))
        .add_magic_path(&magic_root, PathPosition::Start);

    let emitted = pick_first_candidate("@pl", &ctx);
    assert_eq!(emitted.token, "@plan.md");
    assert_eq!(emitted.displayed, magic_root.join("plan.md"));

    let resolved = FileReference::new(&emitted.token)
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();
    assert_eq!(
        resolved.as_deref(),
        Some(emitted.displayed.as_path()),
        "the emitted magic value must execute through the same configured magic root",
    );
}

#[test]
fn completion_rejects_rooted_magic_payloads_before_enumerating_roots() {
    let tmp = TempDir::new().unwrap();
    let base = canonical(tmp.path());
    let ctx = FileResolutionContext::new(&base)
        .with_repository_root(&base)
        .with_home_dir(&base);

    for token in [
        "@//etc/ho",
        "@///etc/ho",
        "%@//etc/ho",
        r"@C:\Windows\wi",
        r"%@/C:\Windows\wi",
        r"@\Windows\wi",
        r"@/\\server\share\fi",
        r"%@\\server\share\fi",
    ] {
        assert!(
            matches!(
                FileReference::complete_partial_in_context(token, &ctx),
                Err(FileReferenceError::InvalidSyntax(_))
            ),
            "completion must reject rooted magic token: {token:?}",
        );
    }
}

/// The context-aware completer performs no live discovery: with no repository
/// root supplied, an implicit token enumerates only the base directory, and the
/// emitted value resolves base-only through the same context.
#[test]
fn context_without_repository_root_stays_base_only() {
    let tmp = TempDir::new().unwrap();
    let base = canonical(tmp.path());
    // Deliberately NOT a git repo and no repository root on the context, so a
    // live-discovery completer would still find nothing, but the point is the
    // context path never probes for one.
    fs::create_dir_all(base.join("prompts")).unwrap();
    fs::write(base.join("prompts/plan.md"), b"base").unwrap();

    let ctx = FileResolutionContext::new(base.clone());

    let emitted = pick_first_candidate("prompts/pl", &ctx);
    assert_eq!(emitted.displayed, base.join("prompts/plan.md"));

    let resolved = FileReference::new(&emitted.token)
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();
    assert_eq!(resolved.as_deref(), Some(emitted.displayed.as_path()));
}
