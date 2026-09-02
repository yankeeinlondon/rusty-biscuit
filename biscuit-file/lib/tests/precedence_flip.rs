//! Phase 4 integration tests: the implicit-relative precedence flip and the
//! OQ1 interpolation-reclassification rule.
//!
//! Implicit relative references resolve **the source/base directory first,
//! then the repository root**. Explicit `./`/`../` references keep their single
//! base-relative candidate with no fallback. Environment interpolation
//! reclassifies filesystem anchoring for the local family (OQ1 option 2) and
//! rejects any injected grammar sigil.
//!
//! These use an explicit [`FileResolutionContext`] with a supplied repository
//! root, so they never read ambient state and are deterministic on every host.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use biscuit_file::{
    DetailedOutcome, FileReference, FileReferenceError, FileReferenceKind, FileResolutionContext,
    ResolutionFailure, RootProvenance,
};
use tempfile::TempDir;

/// Build a context anchored at `base`, supplying `repo` as the repository root.
fn ctx_with_repo(base: &Path, repo: &Path) -> FileResolutionContext {
    FileResolutionContext::new(base.to_path_buf()).with_repository_root(repo.to_path_buf())
}

/// Canonicalize into the spelling the resolver reports.
///
/// `std::fs::canonicalize` returns a `\\?\` verbatim path on Windows. The
/// resolver reduces every anchor to the legacy form, and these fixtures also
/// join `/`-separated sub-paths, which Win32 refuses to normalize under a
/// verbatim prefix.
fn canonical(path: &Path) -> std::path::PathBuf {
    dunce::canonicalize(path).unwrap()
}

/// A base sub-directory inside `repo`, both canonicalized.
fn repo_and_base(tmp: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    let repo = canonical(tmp.path());
    let base = repo.join("pkg");
    fs::create_dir_all(&base).unwrap();
    let base = canonical(&base);
    (repo, base)
}

#[test]
fn implicit_both_exist_source_candidate_wins() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);

    // The same filename exists at both the repository root and the base.
    fs::write(repo.join("shared.md"), b"repo").unwrap();
    fs::write(base.join("shared.md"), b"source").unwrap();

    let ctx = ctx_with_repo(&base, &repo);
    let resolved = FileReference::new("shared.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(base.join("shared.md").as_path()),
        "source candidate wins on a name collision",
    );
}

#[test]
fn implicit_only_source_exists_falls_back_to_base() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);

    // The file exists only in the base; the repository root lacks it.
    fs::write(base.join("only.md"), b"source").unwrap();

    let ctx = ctx_with_repo(&base, &repo);
    let resolved = FileReference::new("only.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(base.join("only.md").as_path()),
        "implicit resolution falls back to the source candidate",
    );
}

#[test]
fn implicit_without_git_root_uses_base_only() {
    let tmp = TempDir::new().unwrap();
    let base = canonical(tmp.path());
    fs::write(base.join("loose.md"), b"loose").unwrap();

    // No repository root supplied: the base is the only candidate.
    let ctx = FileResolutionContext::new(base.clone());
    let file_ref = FileReference::new("loose.md").unwrap();
    let detailed = file_ref.resolve_detailed(&ctx);

    assert_eq!(detailed.matched_path(), Some(base.join("loose.md").as_path()));
    let candidates = detailed.candidates();
    assert_eq!(candidates.len(), 1, "no git root means a single base candidate");
    assert_eq!(candidates[0].candidate().provenance(), RootProvenance::Source);
}

#[test]
fn explicit_relative_yields_one_base_candidate_and_never_falls_back() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);

    // A repo-root twin exists, but an explicit `./` reference must ignore it.
    fs::write(repo.join("x.md"), b"repo").unwrap();

    let ctx = ctx_with_repo(&base, &repo);
    let file_ref = FileReference::new("./x.md").unwrap();
    let detailed = file_ref.resolve_detailed(&ctx);

    let candidates = detailed.candidates();
    assert_eq!(candidates.len(), 1, "explicit relative has exactly one candidate");
    assert_eq!(candidates[0].candidate().provenance(), RootProvenance::Source);
    assert_eq!(candidates[0].candidate().path(), base.join("x.md"));

    // The source-local file is absent, so it fails rather than adopting the
    // repository twin.
    assert!(matches!(
        detailed.outcome(),
        DetailedOutcome::Failed(ResolutionFailure::NoMatch)
    ));
    assert_eq!(detailed.class().kind, FileReferenceKind::ExplicitRelative);
    assert_eq!(detailed.effective_kind(), FileReferenceKind::ExplicitRelative);
}

#[test]
fn implicit_when_base_is_repository_root_dedupes_to_one_candidate() {
    let tmp = TempDir::new().unwrap();
    let repo = canonical(tmp.path());
    fs::write(repo.join("top.md"), b"top").unwrap();

    // Base and repository root are the same directory: a single candidate, not
    // a duplicated pair.
    let ctx = ctx_with_repo(&repo, &repo);
    let detailed = FileReference::new("top.md").unwrap().resolve_detailed(&ctx);

    assert_eq!(detailed.matched_path(), Some(repo.join("top.md").as_path()));
    let candidates = detailed.candidates();
    assert_eq!(candidates.len(), 1, "equal roots collapse to one candidate");
    assert_eq!(
        candidates[0].candidate().provenance(),
        RootProvenance::Source,
        "the surviving candidate carries source provenance",
    );
}

#[test]
fn interpolation_to_absolute_reclassifies_anchoring() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);

    // `{{ROOT}}` expands to an absolute directory outside the base/repo, so the
    // implicit-authored reference resolves as an absolute one.
    let abs_dir = repo.join("elsewhere");
    fs::create_dir_all(&abs_dir).unwrap();
    fs::write(abs_dir.join("spec.md"), b"abs").unwrap();

    let mut env = HashMap::new();
    env.insert("ROOT".to_string(), abs_dir.to_string_lossy().to_string());
    let ctx = ctx_with_repo(&base, &repo).with_env(env);

    let file_ref = FileReference::new("{{ROOT}}/spec.md").unwrap();
    let detailed = file_ref.resolve_detailed(&ctx);

    assert_eq!(
        detailed.matched_path(),
        Some(abs_dir.join("spec.md").as_path()),
        "the interpolated absolute path resolves verbatim",
    );

    // Diagnostics record both the authored kind and the effective anchoring.
    assert_eq!(
        detailed.class().kind,
        FileReferenceKind::ImplicitRelative,
        "the authored kind stays implicit relative",
    );
    assert_eq!(
        detailed.effective_kind(),
        FileReferenceKind::Absolute,
        "the effective anchoring is absolute after interpolation",
    );

    // Exactly one candidate -- the absolute path -- with Absolute provenance;
    // the root was not joined onto it.
    let candidates = detailed.candidates();
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].candidate().provenance(),
        RootProvenance::Absolute
    );
    assert_eq!(candidates[0].candidate().path(), abs_dir.join("spec.md"));
}

#[test]
fn interpolation_staying_implicit_keeps_both_kinds_implicit() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);
    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(repo.join("docs/readme.md"), b"doc").unwrap();

    let mut env = HashMap::new();
    env.insert("DIR".to_string(), "docs".to_string());
    let ctx = ctx_with_repo(&base, &repo).with_env(env);

    let file_ref = FileReference::new("{{DIR}}/readme.md").unwrap();
    let detailed = file_ref.resolve_detailed(&ctx);

    assert_eq!(
        detailed.matched_path(),
        Some(repo.join("docs/readme.md").as_path()),
        "a relative interpolation stays implicit and resolves repository-first",
    );
    assert_eq!(detailed.class().kind, FileReferenceKind::ImplicitRelative);
    assert_eq!(detailed.effective_kind(), FileReferenceKind::ImplicitRelative);
}

#[test]
fn interpolation_injecting_a_sigil_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);

    // An environment value that begins with a grammar sigil must not be honored
    // as that kind; grammar sigils stay author-controlled.
    for (var, injected) in [
        ("MAGIC", "@docs/spec.md"),
        ("REPOSITORY", "&docs/spec.md"),
        ("SCOPED", "^docs/spec.md"),
        ("PKG", "!README.md"),
        ("VAULT_REF", "vault:notes.md"),
        ("REMOTE", "https://example.com/a.md"),
        ("RECUR", "%deep.md"),
    ] {
        let mut env = HashMap::new();
        env.insert(var.to_string(), injected.to_string());
        let ctx = ctx_with_repo(&base, &repo).with_env(env);

        let file_ref = FileReference::new(&format!("{{{{{var}}}}}")).unwrap();
        let detailed = file_ref.resolve_detailed(&ctx);

        assert!(
            matches!(
                detailed.outcome(),
                DetailedOutcome::Failed(ResolutionFailure::InvalidReference)
            ),
            "injected `{injected}` must be an invalid reference, got {:?}",
            detailed.outcome(),
        );
        assert!(
            matches!(detailed.error(), Some(FileReferenceError::InvalidSyntax(_))),
            "injected sigil yields a typed InvalidSyntax error",
        );
    }
}

#[test]
fn recursive_interpolation_reclassifies_and_resolves_local_paths() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);
    let absolute_dir = repo.join("absolute");
    fs::create_dir_all(&absolute_dir).unwrap();
    fs::create_dir_all(base.join("nested")).unwrap();
    fs::create_dir_all(repo.join("nested")).unwrap();
    fs::write(absolute_dir.join("absolute.md"), b"absolute").unwrap();
    fs::write(base.join("nested/explicit.md"), b"explicit").unwrap();
    fs::write(repo.join("nested/implicit.md"), b"implicit").unwrap();

    let cases = [
        (
            "ABSOLUTE",
            absolute_dir.to_string_lossy().into_owned(),
            "absolute.md",
            absolute_dir.join("absolute.md"),
            FileReferenceKind::Absolute,
            vec![RootProvenance::Absolute],
        ),
        (
            "EXPLICIT",
            "./nested".to_string(),
            "explicit.md",
            base.join("nested/explicit.md"),
            FileReferenceKind::ExplicitRelative,
            vec![RootProvenance::Source],
        ),
        (
            "IMPLICIT",
            "nested".to_string(),
            "implicit.md",
            repo.join("nested/implicit.md"),
            FileReferenceKind::ImplicitRelative,
            vec![RootProvenance::Source, RootProvenance::Repository],
        ),
    ];

    for (var, expansion, filename, expected_path, expected_kind, expected_provenance) in cases {
        let ctx = ctx_with_repo(&base, &repo)
            .with_env(HashMap::from([(var.to_string(), expansion)]));
        let file_ref = FileReference::new(&format!("%{{{{{var}}}}}/{filename}")).unwrap();
        let plan = file_ref.candidate_plan(&ctx).unwrap();
        let provenance: Vec<_> = plan.iter().map(|root| root.provenance()).collect();

        assert_eq!(
            provenance, expected_provenance,
            "recursive `{var}` expansion must use its effective local anchoring",
        );

        let detailed = file_ref.resolve_detailed(&ctx);
        assert_eq!(detailed.effective_kind(), expected_kind);
        assert_eq!(
            detailed.matched_path(),
            Some(expected_path.as_path()),
            "recursive `{var}` expansion must resolve through its effective roots",
        );
    }
}

#[test]
fn recursive_interpolation_injecting_a_sigil_is_rejected_before_root_planning() {
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);

    for (var, injected) in [
        ("MAGIC", "@docs/spec.md"),
        ("REPOSITORY", "&docs/spec.md"),
        ("SCOPED", "^docs/spec.md"),
        ("PACKAGE", "!README.md"),
        ("RECURSIVE", "%deep.md"),
        ("VAULT", "vault:notes.md"),
        ("HTTP_LOWER", "http://example.com/a.md"),
        ("HTTPS_LOWER", "https://example.com/a.md"),
        ("HTTP_UPPER", "HTTP://example.com/a.md"),
        ("HTTPS_MIXED", "HtTpS://example.com/a.md"),
    ] {
        let ctx = ctx_with_repo(&base, &repo).with_env(HashMap::from([(
            var.to_string(),
            injected.to_string(),
        )]));
        let file_ref = FileReference::new(&format!("%{{{{{var}}}}}")).unwrap();

        assert!(
            matches!(
                file_ref.candidate_plan(&ctx),
                Err(FileReferenceError::InvalidSyntax(_))
            ),
            "recursive interpolation must reject injected `{injected}` before building roots",
        );

        let detailed = file_ref.resolve_detailed(&ctx);
        assert!(detailed.candidates().is_empty());
        assert!(matches!(
            detailed.outcome(),
            DetailedOutcome::Failed(ResolutionFailure::InvalidReference)
        ));
        assert!(matches!(
            detailed.error(),
            Some(FileReferenceError::InvalidSyntax(_))
        ));
    }
}

/// Windows callers routinely anchor a context with `std::fs::canonicalize`,
/// which yields a `\\?\` verbatim path. Win32 skips path normalization under
/// that prefix, so the reference's own `/` would stay a literal filename
/// character; the resolver must reduce the anchor before joining.
#[cfg(windows)]
#[test]
fn verbatim_anchor_resolves_a_slash_separated_reference() {
    let tmp = TempDir::new().unwrap();
    let verbatim = std::fs::canonicalize(tmp.path()).unwrap();
    assert!(
        verbatim.as_os_str().to_string_lossy().starts_with(r"\\?\"),
        "fixture is only meaningful with a verbatim anchor, got {verbatim:?}",
    );
    let plain = dunce::simplified(&verbatim).to_path_buf();

    let base = plain.join("pkg");
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(plain.join("docs")).unwrap();
    fs::write(plain.join("docs/readme.md"), b"doc").unwrap();

    let ctx = ctx_with_repo(&verbatim.join("pkg"), &verbatim);
    let resolved = FileReference::new("docs/readme.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(plain.join("docs/readme.md").as_path()),
        "a verbatim anchor must resolve and report the legacy spelling",
    );
}

/// A caller-supplied repository root and base commonly reach the resolver from
/// different producers (`gix` discovery yields legacy, `canonicalize` yields
/// verbatim). Two spellings of one directory must not become two candidates,
/// and the survivor carries the same source provenance as the same-spelling
/// collapse in [`implicit_when_base_is_repository_root_dedupes_to_one_candidate`].
#[cfg(windows)]
#[test]
fn verbatim_and_legacy_spellings_of_one_root_dedupe_to_one_candidate() {
    let tmp = TempDir::new().unwrap();
    let verbatim = std::fs::canonicalize(tmp.path()).unwrap();
    let plain = dunce::simplified(&verbatim).to_path_buf();
    assert_ne!(verbatim, plain, "fixture needs two distinct spellings");
    fs::write(plain.join("top.md"), b"top").unwrap();

    let ctx = ctx_with_repo(&verbatim, &plain);
    let detailed = FileReference::new("top.md").unwrap().resolve_detailed(&ctx);

    let candidates = detailed.candidates();
    assert_eq!(candidates.len(), 1, "one directory yields one candidate");
    assert_eq!(
        candidates[0].candidate().provenance(),
        RootProvenance::Source,
    );
    assert_eq!(detailed.matched_path(), Some(plain.join("top.md").as_path()));
}

#[test]
fn special_kinds_are_unchanged_by_the_flip() {
    // Magic, repository-scoped, and vault references keep their own root order; the
    // implicit flip does not touch them.
    let tmp = TempDir::new().unwrap();
    let (repo, base) = repo_and_base(&tmp);
    fs::write(repo.join("cfg.toml"), b"root").unwrap();

    let ctx = ctx_with_repo(&base, &repo);

    // `@cfg.toml` searches the repository root (magic), unaffected by the flip.
    let magic = FileReference::new("@cfg.toml")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();
    assert_eq!(magic.as_deref(), Some(repo.join("cfg.toml").as_path()));

    // A vault reference with no configured roots still fails as before.
    let vault = FileReference::new("vault:notes.md")
        .unwrap()
        .resolve_detailed(&ctx);
    assert!(matches!(
        vault.error(),
        Some(FileReferenceError::VaultNotConfigured)
    ));
}
