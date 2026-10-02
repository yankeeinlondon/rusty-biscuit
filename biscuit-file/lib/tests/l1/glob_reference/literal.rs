//! Literal names versus character classes, `%` on `take_first`, lexical
//! `matches`, and case sensitivity.

use biscuit_file::{FileReference, GlobReference};

use super::{Fixture, glob, listed};

/// Criterion 19: `[id]` is a literal name in a file reference and a class in
/// a glob reference; `%` and `GlobReference::escape` keep it literal.
#[test]
fn brackets_are_literal_in_a_file_reference_and_a_class_in_a_glob() {
    let fx = Fixture::new();
    let literal = fx.file("repo/pages/[id].md");
    let i = fx.file("repo/pages/i.md");
    let d = fx.file("repo/pages/d.md");
    let ctx = fx.ctx(&fx.repo);

    assert_eq!(
        FileReference::new("pages/[id].md").unwrap().resolve_in_context(&ctx).unwrap(),
        Some(literal.clone())
    );
    assert_eq!(listed(&["pages/[id].md"], &ctx), [d, i]);
    assert_eq!(
        FileReference::new("%pages/[id].md").unwrap().resolve_in_context(&ctx).unwrap(),
        Some(literal.clone())
    );

    let escaped = format!("pages/{}", GlobReference::escape("[id].md"));
    assert_eq!(escaped, "pages/[[]id[]].md");
    assert_eq!(listed(&[escaped.as_str()], &ctx), [literal]);
}

/// `escape` covers every glob metacharacter and leaves `\` alone, because a
/// glob reference never reads `\` as an escape.
#[test]
fn escape_makes_every_metacharacter_literal() {
    for (text, escaped) in [
        ("[id].md", "[[]id[]].md"),
        ("*.md", "[*].md"),
        ("?.md", "[?].md"),
        ("{a,b}.md", "[{]a,b[}].md"),
        (r"a\b.md", r"a\b.md"),
    ] {
        assert_eq!(GlobReference::escape(text), escaped, "{text}");
    }
}

/// On Unix, where `*`, `?`, and `\` are legal in names, an escaped name finds
/// exactly the file, and `\` never escapes the next character.
#[cfg(unix)]
#[test]
fn escaped_names_find_exactly_the_literal_file() {
    let fx = Fixture::new();
    let star = fx.file("repo/n/*.md");
    let question = fx.file("repo/n/?.md");
    let brace = fx.file("repo/n/{a,b}.md");
    let backslash = fx.file(r"repo/n/a\b.md");
    fx.file("repo/n/a.md");
    let ctx = fx.ctx(&fx.repo);

    for (name, file) in [("*.md", star), ("?.md", question), ("{a,b}.md", brace), (r"a\b.md", backslash)]
    {
        let pattern = format!("n/{}", GlobReference::escape(name));
        assert_eq!(listed(&[pattern.as_str()], &ctx), [file], "{name}");
    }
    // `\[` is a literal backslash and then a class, not an escaped bracket.
    let brackets = fx.file("repo/n/[x].md");
    assert!(!listed(&[r"n/\[x\].md"], &ctx).contains(&brackets));
}

/// Criterion 23: `%` returns the most local match, not the lexical winner
/// across roots, and searches vault roots.
#[test]
fn recursive_references_are_local_first() {
    let fx = Fixture::new();
    let local = fx.file("repo/area/pkg/x/y/README.md");
    fx.file("repo/README.md");
    let notes = fx.file("vault/sub/notes.md");
    let ctx = fx.ctx(&fx.pkg).add_vault(fx.root.join("vault"));

    // Lexically `repo/README.md` sorts first (`R` < `a`); the package wins.
    assert_eq!(
        FileReference::new("%^README.md").unwrap().resolve_in_context(&ctx).unwrap(),
        Some(local)
    );
    assert_eq!(
        FileReference::new("%vault:notes.md").unwrap().resolve_in_context(&ctx).unwrap(),
        Some(notes)
    );
}

/// `%` keeps its other shapes: a parent-directory suffix, leading `./`, an
/// absolute payload, and a `{{VAR}}` payload whose value holds glob syntax.
#[test]
fn recursive_references_keep_their_payload_literal() {
    let fx = Fixture::new();
    let nested = fx.file("repo/a/docs/spec.md");
    fx.file("repo/b/other/spec.md");
    let config = fx.file("repo/area/pkg/deep/config.toml");
    let odd = fx.file("outside/odd[dir]/deep/x.md");
    let odd_dir = fx.root.join("outside/odd[dir]");
    let ctx = fx.ctx_with_env(&fx.pkg, &[("ODD", odd_dir.to_str().unwrap())]);

    let resolve = |raw: &str| FileReference::new(raw).unwrap().resolve_in_context(&ctx).unwrap();
    assert_eq!(resolve("%docs/spec.md"), Some(nested));
    assert_eq!(resolve("%./config.toml"), Some(config));
    assert_eq!(resolve("%{{ODD}}/x.md"), Some(odd.clone()));
    let absolute = format!("%{}", biscuit_file::to_portable_string(&odd_dir.join("x.md")));
    assert_eq!(resolve(&absolute), Some(odd));
    assert_eq!(resolve("%missing.md"), None);
}

/// Criterion 26: `matches` judges a path that does not exist the same as an
/// existing one, under an existing directory and under a missing one.
#[test]
fn matches_is_lexical_for_paths_that_do_not_exist() {
    let fx = Fixture::new();
    let existing = fx.file("repo/docs/x.md");
    let ctx = fx.ctx(&fx.repo);
    let docs = glob(&["docs/*.md"]);
    let anywhere = glob(&["**/x.md"]);

    for path in [
        existing.clone(),
        fx.repo.join("docs/new.md"),
        fx.repo.join("missing/deep/x.md"),
    ] {
        let in_docs = path.parent() == existing.parent();
        assert_eq!(docs.matches(&path, &ctx), in_docs, "{}", path.display());
    }
    assert!(anywhere.matches(&fx.repo.join("missing/deep/x.md"), &ctx));
    assert!(!anywhere.matches(&fx.repo.join("missing/deep/y.md"), &ctx));
    // A relative path is read from the context's `cwd`.
    assert!(docs.matches(std::path::Path::new("docs/new.md"), &ctx));
}

/// With no context, a bare pattern judges an absolute path's full path, an
/// absolute pattern is read as written, and a pattern whose roots need a
/// context neither admits nor rejects.
#[test]
fn matches_without_context_judges_the_full_path() {
    let fx = Fixture::new();
    let spec = fx.repo.join("fixes/x/spec.md");
    let portable_repo = biscuit_file::to_portable_string(&fx.repo);

    // Control: a bare `**/` pattern admits the full path, which need not exist.
    assert!(glob(&["**/fixes/**/spec.md"]).matches_without_context(&spec));
    assert!(!glob(&["**/features/**/spec.md"]).matches_without_context(&spec));
    // A bare pattern is read from the filesystem root, not some document.
    assert!(!glob(&["fixes/**/spec.md"]).matches_without_context(&spec));
    // An absolute pattern is read as written.
    let absolute = format!("{portable_repo}/fixes/*/spec.md");
    assert!(glob(&[absolute.as_str()]).matches_without_context(&spec));
    // Bare negations still reject; the file-name view still applies.
    assert!(!glob(&["**/*.md", "!**/fixes/**"]).matches_without_context(&spec));
    assert!(glob(&["spec.md"]).with_file_name_view().matches_without_context(&spec));
    // Patterns that need a context admit nothing and reject nothing.
    for pattern in ["&**/spec.md", "^**/spec.md", "@**/spec.md", "~/**/spec.md", "./**/spec.md", "{{X}}/**/spec.md"] {
        assert!(!glob(&[pattern]).matches_without_context(&spec), "{pattern}");
        assert!(glob(&["**/spec.md", &format!("!{pattern}")]).matches_without_context(&spec), "!{pattern}");
    }
    // A relative path is never a member.
    assert!(!glob(&["**/spec.md"]).matches_without_context(std::path::Path::new("fixes/x/spec.md")));
}

/// Criterion 26: matching is case-sensitive on every OS.
#[test]
fn matching_is_case_sensitive() {
    let fx = Fixture::new();
    let lower = fx.file("repo/x.md");
    let ctx = fx.ctx(&fx.repo);

    assert!(listed(&["*.MD"], &ctx).is_empty());
    assert!(!glob(&["*.MD"]).matches(&lower, &ctx));
    assert_eq!(listed(&["*.md"], &ctx), [lower]);
}

/// A literal miss whose text looks like a glob carries a hint toward a glob
/// form; a match, another failure, or plain text carries none.
#[test]
fn a_literal_miss_that_looks_like_a_glob_hints_at_glob_references() {
    let fx = Fixture::new();
    fx.file("repo/docs/a.md");
    let ctx = fx.ctx(&fx.repo);
    let hint = |raw: &str| FileReference::new(raw).unwrap().resolve_detailed(&ctx).glob_hint();

    for raw in ["docs/*.md", "docs/?.md", "pages/[id].md"] {
        let text = hint(raw).unwrap_or_else(|| panic!("`{raw}` misses with a hint"));
        assert!(text.contains("glob reference") && text.contains("::file-links"), "{text}");
    }
    assert_eq!(hint("docs/missing.md"), None, "plain text");
    assert_eq!(hint("docs/a.md"), None, "a match");
    assert_eq!(hint("&../*.md"), None, "a failure other than a miss");
}
