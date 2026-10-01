//! Properties every result must have (idempotence, candidate equality,
//! stable failures, captured-state isolation) and the documented examples.

use std::collections::HashMap;

use biscuit_file::{PORTABLE_ENV_VARIABLES, PortabilityPreference as P, PortablePath, PortablePathError};

use super::*;

/// A context and a corpus of targets spread over every kind of root.
fn corpus(fx: &Fixture) -> (FileResolutionContext, Vec<PathBuf>) {
    let cwd = fx.dir("repo/apps/web/docs");
    let notes = fx.dir("notes");
    let ctx = fx.ctx(
        &cwd,
        &[(PORTABLE_ENV_VARIABLES, "NOTES"), ("NOTES", notes.to_str().unwrap())],
    );
    let mut targets: Vec<PathBuf> = [
        "repo/apps/web/docs/a.md",
        "repo/apps/web/docs/sub/b.md",
        "repo/apps/web/c.md",
        "repo/apps/web/src/d.md",
        "repo/apps/api/e.md",
        "repo/f.md",
        "home/g.md",
        "home/.cfg/h.json",
        "notes/i.md",
        "notes/deep/j.md",
        "outside/k.md",
        "repo/apps/web/docs/@at.md",
    ]
    .iter()
    .map(|rel| fx.file(rel))
    .collect();
    targets.push(fx.path("repo/new/n.md"));
    targets.push(fx.path("home/missing.json"));
    targets.push(fx.dir("repo/apps/web/docs/assets"));
    (ctx, targets)
}

fn single_location(strategy: &P) -> bool {
    !matches!(
        strategy,
        P::AuthoredIntent(_) | P::RepoMultiPath(_) | P::MagicPath(_)
    )
}

#[test]
fn every_result_is_a_fixed_point_and_names_its_target() {
    let fx = Fixture::new();
    let (ctx, targets) = corpus(&fx);
    for target in &targets {
        let first = evaluate(PortablePath::from_path(target).with_ctx(&ctx));
        let raw = first.reference().raw().to_string();

        // Candidate equality: a single-location result plans exactly the target.
        if single_location(first.strategy()) {
            let plan = first.reference().candidate_plan(&ctx).unwrap();
            assert_eq!(plan.len(), 1, "{raw}");
            assert!(same_file(plan[0].path(), target), "{raw} plans {:?}", plan[0].path());
        }

        // Idempotence: feeding the result back returns it unchanged, twice.
        let second = evaluate(PortablePath::from_reference(reference(&raw)).with_ctx(&ctx));
        assert_eq!(second.reference().raw(), raw, "{}", target.display());
        let third = evaluate(PortablePath::from_reference(second.into_reference()).with_ctx(&ctx));
        assert_eq!(third.reference().raw(), raw);
    }
}

#[test]
fn cleaning_authored_links_twice_changes_nothing_the_second_time() {
    let fx = Fixture::new();
    let (ctx, _) = corpus(&fx);
    for (authored, cleaned) in [
        ("../../../f.md", "&f.md"),
        ("./a.md", "./a.md"),
        ("a.md", "a.md"),
        ("sub/b.md", "sub/b.md"),
        ("../c.md", "../c.md"),
        ("../src/d.md", "../src/d.md"),
        ("../../api/e.md", "&apps/api/e.md"),
        ("./../docs/a.md", "./a.md"),
        ("./@at.md", "./@at.md"),
        ("^/f.md", "^/f.md"),
    ] {
        let first = evaluate(PortablePath::from_reference(reference(authored)).with_ctx(&ctx));
        assert_eq!(first.reference().raw(), cleaned, "{authored}");
        let second = evaluate(PortablePath::from_reference(first.into_reference()).with_ctx(&ctx));
        assert_eq!(second.reference().raw(), cleaned, "{authored}: second run");
    }
}

#[test]
fn an_unresolvable_input_fails_the_same_way_on_retry() {
    let fx = Fixture::new();
    let (ctx, _) = corpus(&fx);
    for authored in ["missing.md", "../../../../../outside.md", "{{UNSET}}/x.md"] {
        let portable = PortablePath::from_reference(reference(authored)).with_ctx(&ctx);
        let first = evaluate_err(portable.clone());
        let second = evaluate_err(portable);
        assert!(matches!(first, PortablePathError::UnresolvableInput { .. }), "{first}");
        assert!(matches!(second, PortablePathError::UnresolvableInput { .. }), "{second}");
        assert_eq!(first.findings(), second.findings(), "{authored}");
        assert_eq!(first.to_string(), second.to_string(), "{authored}");
    }
}

/// The builder holds its own copy of the context, and nothing is read from
/// the live process once a context is supplied.
#[test]
fn a_captured_context_isolates_evaluation_from_later_changes() {
    const NAME: &str = "PORTABLE_PATH_ISOLATION_ANCHOR";
    assert!(std::env::var_os(NAME).is_none(), "precondition: the process does not set {NAME}");
    let fx = Fixture::new();
    let cwd = fx.dir("work");
    let anchor = fx.dir("anchor");
    let in_anchor = fx.file("anchor/x.md");
    let in_home = fx.file("home/y.md");
    let mut ctx = fx.bare_ctx(&cwd, &[(PORTABLE_ENV_VARIABLES, NAME), (NAME, anchor.to_str().unwrap())]);

    // Batch reuse: one prepared context, several builders.
    let anchored = PortablePath::from_path(&in_anchor).with_ctx(&ctx);
    let homed = PortablePath::from_path(&in_home).with_ctx(&ctx);
    ctx = ctx.with_env(HashMap::new()).without_home_dir();
    assert!(ctx.env().is_empty());

    assert_eq!(evaluate(anchored).reference().raw(), format!("{{{{{NAME}}}}}/x.md"));
    assert_eq!(evaluate(homed).reference().raw(), "~/y.md", "the fixture home, not the live one");
}

// ---- documented examples ------------------------------------------------------

#[test]
fn the_documented_examples_hold() {
    let fx = Fixture::new();
    let topics = fx.dir("repo/biscuit-file/docs/topics");
    let page = fx.file("repo/biscuit-file/docs/topics/file-references.md");

    // From inside the tree the nearby relative link wins; from elsewhere in
    // the repository, the `&` form. (From the repository root itself
    // `ChildDir` precedes `RepoRoot`, so the default writes `./biscuit-file/…`.)
    let found = evaluate(PortablePath::from_path(&page).with_ctx(&fx.ctx(&topics, &[])));
    assert_eq!(found.reference().raw(), "./file-references.md");
    let elsewhere = fx.dir("repo/apps/web");
    let found = evaluate(PortablePath::from_path(&page).with_ctx(&fx.ctx(&elsewhere, &[])));
    assert_eq!(found.reference().raw(), "&biscuit-file/docs/topics/file-references.md");
    let found = evaluate(PortablePath::from_path(&page).with_ctx(&fx.ctx(&fx.repo, &[])));
    assert_eq!(found.reference().raw(), "./biscuit-file/docs/topics/file-references.md");

    // `with_cwd` without a context (the fixture is not a git repository).
    let found = evaluate(PortablePath::from_path(&page).with_cwd(&topics));
    assert_eq!(found.reference().raw(), "./file-references.md");

    // A non-repository tree supplied with `with_base_dir`.
    let docs = fx.dir("docs");
    let my_doc = fx.file("docs/my-doc.md");
    let found = evaluate(PortablePath::from_path(&my_doc).with_cwd(&docs).with_base_dir(&docs));
    assert_eq!(found.reference().raw(), "./my-doc.md");

    // `^/foo.md` is kept; `../../../foo.md` becomes `&foo.md`.
    let guide_dir = fx.dir("repo/apps/web/docs");
    fx.file("repo/foo.md");
    let ctx = fx.ctx(&guide_dir, &[]);
    let kept = evaluate(PortablePath::from_reference(reference("^/foo.md")).with_ctx(&ctx));
    assert_eq!(kept.reference().raw(), "^/foo.md");
    let cleaned = evaluate(PortablePath::from_reference(reference("../../../foo.md")).with_ctx(&ctx));
    assert_eq!(cleaned.reference().raw(), "&foo.md");

    // Suffixes stay with the caller: split before, reattach after.
    let link = "../../../foo.md#install";
    let (path, suffix) = link.split_at(link.find('#').unwrap());
    let cleaned = evaluate(PortablePath::from_reference(reference(path)).with_ctx(&ctx));
    assert_eq!(format!("{}{suffix}", cleaned.reference().raw()), "&foo.md#install");
}
