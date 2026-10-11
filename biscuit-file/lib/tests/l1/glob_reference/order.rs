//! Native order (root precedence, then shallowest, then component-wise),
//! ownership by the first containing root, and one file per canonical
//! location.

use std::collections::HashSet;

use biscuit_file::GlobReference;

use super::{Fixture, glob, link_dir, listed, snapshot};

/// Criterion 15: local-first order across the `^` roots, each file once,
/// although the repository pass also reaches the package's and area's files.
#[test]
fn list_files_orders_by_root_precedence_then_depth() {
    let fx = Fixture::new();
    for file in [
        "repo/z/intro.md",
        "repo/intro.md",
        "repo/area/intro.md",
        "repo/area/pkg/a/b/intro.md",
        "repo/area/pkg/intro.md",
    ] {
        fx.file(file);
    }
    let ctx = fx.ctx(&fx.pkg);

    let expected = [
        fx.pkg.join("intro.md"),
        fx.pkg.join("a/b/intro.md"),
        fx.area.join("intro.md"),
        fx.repo.join("intro.md"),
        fx.repo.join("z/intro.md"),
    ];
    assert_eq!(listed(&["^**/intro.md"], &ctx), expected);
    assert_eq!(
        glob(&["^**/intro.md"]).take_first(&ctx).unwrap(),
        Some(fx.pkg.join("intro.md"))
    );
    assert_eq!(
        glob(&["^**/intro.md"]).roots(&ctx),
        [fx.pkg.clone(), fx.area.clone(), fx.repo.clone()]
    );
}

/// Criterion 15, last clause: `take_first` stops at the first root with a
/// match. A later root whose search directory cannot be read fails
/// `list_files`, so a successful `take_first` proves it was never walked.
/// (`@` is used because, unlike `&` and `^`, it runs no repository
/// containment check over its roots before searching.)
#[cfg(unix)]
#[test]
fn take_first_never_walks_a_later_root() {
    use std::os::unix::fs::PermissionsExt;

    let fx = Fixture::new();
    fx.file("repo/area/pkg/x/y/hit.md");
    fx.file("repo/x/y/later.md");
    let locked = fx.repo.join("x");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let unreadable = std::fs::read_dir(locked.join("y")).is_err();
    let ctx = fx.ctx(&fx.pkg);

    let first = glob(&["@x/y/*.md"]).take_first(&ctx);
    let listing = glob(&["@x/y/*.md"]).list_files(&ctx);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(first.unwrap(), Some(fx.pkg.join("x/y/hit.md")));
    // A privileged user reads through any mode, so only an unreadable root
    // can show the difference.
    if unreadable {
        assert!(listing.is_err(), "listing walks the locked repository root: {listing:?}");
    }
}

/// Criterion 16: under one root, shallowest first, then component by
/// component (`a` < `a-b`, although `/` > `-` as text).
#[test]
fn ties_break_component_wise() {
    let fx = Fixture::new();
    for file in ["repo/b/intro.md", "repo/a/intro.md", "repo/intro.md", "repo/a-b/x.md", "repo/a/x.md"]
    {
        fx.file(file);
    }
    let ctx = fx.ctx(&fx.repo);

    assert_eq!(
        listed(&["&**/intro.md"], &ctx),
        [fx.repo.join("intro.md"), fx.repo.join("a/intro.md"), fx.repo.join("b/intro.md")]
    );
    assert_eq!(listed(&["&**/x.md"], &ctx), [fx.repo.join("a/x.md"), fx.repo.join("a-b/x.md")]);
}

/// Criteria 18 and 5 (biscuit-file part): a file belongs to the first root
/// that contains it, so an exclusion judged there is not undone by a later
/// root, and a negation is judged against its own nearest root.
#[test]
fn an_excluded_file_is_not_reincluded_by_a_later_root() {
    let fx = Fixture::new();
    for file in [
        "repo/area/pkg/x/spec.md",
        "repo/area/pkg/y/spec.md",
        "repo/x/spec.md",
        "repo/other/spec.md",
    ] {
        fx.file(file);
    }
    let ctx = fx.ctx(&fx.pkg);
    let globs = glob(&["^**/*spec*.md", "!x/**"]);

    assert_eq!(
        globs.list_files(&ctx).unwrap().matches,
        [fx.pkg.join("y/spec.md"), fx.repo.join("other/spec.md")]
    );
    // `area/pkg/x/spec.md` does not match `!x/**` from the repository root,
    // yet it is rejected: its nearest root for both patterns is the package.
    assert!(!globs.matches(&fx.pkg.join("x/spec.md"), &ctx));
    assert!(!globs.matches(&fx.repo.join("x/spec.md"), &ctx));
    assert!(globs.matches(&fx.pkg.join("y/spec.md"), &ctx));
    assert!(globs.matches(&fx.repo.join("other/spec.md"), &ctx));
}

/// Criterion 4: a `&` exclusion removes a file a `^` pattern admits.
#[test]
fn a_repository_exclusion_composes_with_a_scoped_pattern() {
    let fx = Fixture::new();
    fx.file("repo/area/pkg/_completed/old/spec.md");
    fx.file("repo/area/pkg/new/spec.md");
    let ctx = fx.ctx(&fx.pkg);
    let globs = glob(&["^**/spec.md", "!&**/_completed/**"]);

    assert_eq!(globs.list_files(&ctx).unwrap().matches, [fx.pkg.join("new/spec.md")]);
    assert!(!globs.matches(&fx.pkg.join("_completed/old/spec.md"), &ctx));
}

/// Two spellings of one directory (a link, as `/var` and `/private/var` on
/// macOS) are one location: each file is listed once, in the spelling of the
/// first root that reaches it, and `matches` agrees whichever spelling a
/// caller types.
#[test]
fn one_file_reached_through_two_spellings_is_listed_once() {
    let fx = Fixture::new();
    for file in ["repo/area/pkg/intro.md", "repo/area/intro.md", "repo/intro.md"] {
        fx.file(file);
    }
    let alias = fx.root.join("alias");
    link_dir(&fx.repo, &alias);
    let aliased = alias.to_str().unwrap();
    let canonical = fx.repo.to_str().unwrap();
    let ctx = fx.ctx_with_env(&fx.pkg, &[("ALIASED", aliased), ("CANONICAL", canonical)]);
    let globs = glob(&["{{ALIASED}}/**/intro.md", "{{CANONICAL}}/**/intro.md"]);

    let matches = globs.list_files(&ctx).unwrap().matches;
    assert_eq!(
        matches,
        [alias.join("intro.md"), alias.join("area/intro.md"), alias.join("area/pkg/intro.md")]
    );
    assert_eq!(globs.roots(&ctx), std::slice::from_ref(&alias));
    for spelled in [fx.pkg.join("intro.md"), alias.join("area/pkg/intro.md")] {
        assert!(globs.matches(&spelled, &ctx), "{}", spelled.display());
    }

    // A context spelled through the link lists in that spelling, once.
    let linked = snapshot(&alias.join("area/pkg"), Some(&fx.home), &[])
        .with_repository_root(&alias)
        .with_package_area(alias.join("area"))
        .with_package_root(alias.join("area/pkg"));
    let scoped = glob(&["^**/intro.md"]);
    let matches = scoped.list_files(&linked).unwrap().matches;
    assert_eq!(
        matches,
        [alias.join("area/pkg/intro.md"), alias.join("area/intro.md"), alias.join("intro.md")]
    );
    let distinct: HashSet<_> =
        matches.iter().map(|path| dunce::canonicalize(path).unwrap()).collect();
    assert_eq!(distinct.len(), matches.len());
    assert!(scoped.matches(&fx.pkg.join("intro.md"), &linked));
}

/// Bare patterns search the value's `cwd`, then the repository root.
#[test]
fn bare_patterns_search_cwd_then_the_repository_root() {
    let fx = Fixture::new();
    fx.file("repo/area/pkg/top.md");
    fx.file("repo/area/pkg/deep/no.md");
    fx.file("repo/top.md");
    fx.file("repo/other/no.md");
    let ctx = fx.ctx(&fx.pkg);

    assert_eq!(listed(&["*.md"], &ctx), [fx.pkg.join("top.md"), fx.repo.join("top.md")]);
    assert_eq!(listed(&["./*.md"], &ctx), [fx.pkg.join("top.md")]);
    assert_eq!(glob(&["*.md"]).roots(&ctx), [fx.pkg.clone(), fx.repo.clone()]);
}

/// The file-name view (R11): a pattern with no `/` after its prefix also
/// matches a bare file name at any depth, negations included; without the
/// view it does not widen (criterion 11, biscuit-file part).
#[test]
fn the_file_name_view_widens_only_slashless_patterns() {
    let fx = Fixture::new();
    fx.file("repo/docs/spec.md");
    fx.file("repo/docs/_draft.md");
    let ctx = fx.ctx(&fx.repo);

    let plain = glob(&["spec.md"]);
    let widened = GlobReference::new(["spec.md"]).unwrap().with_file_name_view();
    assert!(plain.list_files(&ctx).unwrap().matches.is_empty());
    assert!(!plain.matches(&fx.repo.join("docs/spec.md"), &ctx));
    assert_eq!(widened.list_files(&ctx).unwrap().matches, [fx.repo.join("docs/spec.md")]);
    assert!(widened.matches(&fx.repo.join("docs/spec.md"), &ctx));

    let scoped = GlobReference::new(["^spec.md"]).unwrap().with_file_name_view();
    assert!(scoped.matches(&fx.repo.join("docs/spec.md"), &ctx));

    let negated = GlobReference::new(["*.md", "!_*.md"]).unwrap().with_file_name_view();
    assert!(!negated.matches(&fx.repo.join("docs/_draft.md"), &ctx));
    assert!(negated.matches(&fx.repo.join("docs/spec.md"), &ctx));

    // A `/` after the prefix keeps the pattern path-shaped.
    let pathed = GlobReference::new(["docs/*.md"]).unwrap().with_file_name_view();
    assert!(!pathed.matches(&fx.repo.join("x/docs/spec.md"), &ctx));
}
