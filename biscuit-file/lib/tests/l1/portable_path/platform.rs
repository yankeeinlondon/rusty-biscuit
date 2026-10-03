//! Spellings the text seam must refuse, directory links in and out of the
//! tree, and Windows path forms.

use biscuit_file::{
    AttemptOutcome, NotApplicable, PortabilityPreference as P, PortablePath, PortablePathError,
    ResolutionProblem, SpellingProblem,
};

use super::*;

/// A directory link that needs no privilege: a symlink on Unix, a junction on
/// Windows (ruling R7).
fn link_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed");
    }
}

#[test]
fn a_leading_sigil_in_a_file_name_is_protected_by_dot_slash() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let ctx = fx.ctx(&cwd, &[]);
    for name in ["@notes.md", "&notes.md", "^notes.md", "~notes.md"] {
        let target = fx.file(&format!("repo/docs/{name}"));
        let found = evaluate(PortablePath::from_path(&target).with_ctx(&ctx));
        assert_eq!(found.reference().raw(), format!("./{name}"));
        let resolved = found.reference().resolve_in_context(&ctx).unwrap().unwrap();
        assert!(same_file(&resolved, &target), "{name}");
    }
}

#[test]
fn interpolation_syntax_in_a_file_name_has_no_reference_spelling() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/{{HOME}}.md");
    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&fx.ctx(&cwd, &[])));
    assert!(matches!(error, PortablePathError::NoStrategyMatched { .. }), "{error}");
    for strategy in [P::SameDirRelative, P::RepoRoot(None), P::AbsolutePath] {
        assert_eq!(
            not_applicable(attempt_for(error.attempts(), &strategy)),
            &NotApplicable::UnsafeSpelling(SpellingProblem::GrammarMismatch),
            "{strategy}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_unix_backslash_in_a_name_is_never_split() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file(r"repo/docs/my\report.md");
    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&fx.ctx(&cwd, &[])));
    assert!(matches!(error, PortablePathError::NoStrategyMatched { .. }), "{error}");
    for strategy in [P::SameDirRelative, P::RepoRoot(None), P::AbsolutePath] {
        assert_eq!(
            not_applicable(attempt_for(error.attempts(), &strategy)),
            &NotApplicable::UnsafeSpelling(SpellingProblem::ChangesComponents),
            "{strategy}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_non_unicode_target_is_unrenderable_even_with_absolute_path() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = cwd.join(OsStr::from_bytes(b"caf\xe9.md"));
    let error = evaluate_err(PortablePath::from_path(&target).with_ctx(&fx.ctx(&cwd, &[])));
    let PortablePathError::UnrenderableTarget { target: actual, attempts } = &error else {
        panic!("expected UnrenderableTarget, got {error}");
    };
    assert_eq!(actual, &target);
    assert_eq!(attempts.len(), 1, "only AuthoredIntent ran");
    assert_eq!(not_applicable(&attempts[0]), &NotApplicable::NotAReference);
}

#[test]
fn a_relative_link_must_land_inside_the_tree() {
    let fx = Fixture::new();
    let cwd = fx.dir("tree/docs");
    fx.file("tree/v2/x.md");
    fx.file("team/x.md");
    link_dir(&fx.path("tree/v2"), &fx.path("tree/current"));
    link_dir(&fx.path("team"), &fx.path("tree/shared"));
    let ctx = fx.bare_ctx(&cwd, &[]).with_base_dir(fx.path("tree"));

    let in_tree = fx.path("tree/current/x.md");
    let found = evaluate(PortablePath::from_path(&in_tree).with_ctx(&ctx));
    assert_eq!(found.reference().raw(), "../current/x.md", "an in-tree link keeps working");

    let escaping = fx.path("tree/shared/x.md");
    let found = evaluate(PortablePath::from_path(&escaping).with_ctx(&ctx));
    assert_eq!(found.strategy(), &P::AbsolutePath, "the relative spelling falls through");
    assert_eq!(found.reference().raw(), absolute_text(&escaping));
    match &attempt_for(found.attempts(), &P::PeerDir).outcome {
        AttemptOutcome::NotApplicable(NotApplicable::Rejected(ResolutionProblem::TreeEscape {
            base_dir, ..
        })) => assert_eq!(base_dir, &fx.path("tree")),
        other => panic!("expected a boundary rejection, got {other}"),
    }
}

#[cfg(windows)]
#[test]
fn windows_spellings_of_one_directory_route_alike() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let target = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&cwd, &[]);
    let text = target.to_str().unwrap();
    let verbatim = PathBuf::from(format!(r"\\?\{text}"));
    let lower_drive = PathBuf::from(format!("{}{}", text[..1].to_ascii_lowercase(), &text[1..]));
    for spelling in [target.clone(), verbatim, lower_drive] {
        let found = evaluate(PortablePath::from_path(&spelling).with_ctx(&ctx));
        assert_eq!(found.reference().raw(), "./x.md", "{}", spelling.display());
    }
}

#[cfg(windows)]
#[test]
fn a_target_on_another_drive_has_no_relative_route() {
    let fx = Fixture::new();
    let cwd = fx.dir("repo/docs");
    let ctx = fx.ctx(&cwd, &[]).allow_external_relative();
    let drive = fx.root.to_str().unwrap().as_bytes()[0].to_ascii_uppercase();
    let other = if drive == b'Z' { 'Y' } else { 'Z' };
    // Never probed: the target check is the only filesystem access, and a
    // missing drive reports absence or a typed probe failure, never a route.
    let target = PathBuf::from(format!(r"{other}:\elsewhere\x.md"));
    let result = PortablePath::from_path(&target)
        .with_ctx(&ctx)
        .with_strategy([P::ExternalRelativePath])
        .file_reference();
    match result {
        Err(PortablePathError::NoStrategyMatched { attempts, .. }) => {
            assert_eq!(not_applicable(&attempts[0]), &NotApplicable::NoSharedRoot);
        }
        Err(PortablePathError::ProbeFailed { .. }) => {}
        other => panic!("expected no relative route, got {other:?}"),
    }
}
