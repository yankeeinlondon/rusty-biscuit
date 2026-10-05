//! The pattern grammar `[!][reference-prefix]glob`, one edit per shape.

use biscuit_file::{FileReference, FileReferenceError, GlobReference, GlobReferenceError, ResolutionFailure};

use super::{Fixture, listed};

/// What constructing (and, when it succeeds, listing) a pattern list gives.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The listing, as paths relative to the fixture root.
    Listed(Vec<String>),
    NoPositivePattern,
    MalformedPrefix,
    RejectedPrefix(ResolutionFailure),
    InvalidGlob,
}

fn outcome(patterns: &[&str], fx: &Fixture) -> Outcome {
    match GlobReference::new(patterns) {
        Ok(globs) => Outcome::Listed(
            globs
                .list_files(&fx.ctx(&fx.pkg))
                .unwrap()
                .matches
                .iter()
                .map(|path| {
                    biscuit_file::to_portable_string(path.strip_prefix(&fx.root).unwrap())
                })
                .collect(),
        ),
        Err(error) => {
            // Every pattern-level error names a pattern as authored.
            if !matches!(error, GlobReferenceError::NoPositivePattern) {
                let message = error.to_string();
                assert!(
                    patterns.iter().any(|pattern| message.contains(&format!("`{pattern}`"))),
                    "`{message}` must name one of {patterns:?}"
                );
            }
            match error {
                GlobReferenceError::NoPositivePattern => Outcome::NoPositivePattern,
                GlobReferenceError::MalformedPrefix { .. } => Outcome::MalformedPrefix,
                ref rejected @ GlobReferenceError::RejectedPrefix { .. } => {
                    Outcome::RejectedPrefix(rejected.resolution_failure())
                }
                GlobReferenceError::InvalidGlob { .. } => Outcome::InvalidGlob,
                other => panic!("unexpected construction error for {patterns:?}: {other:?}"),
            }
        }
    }
}

#[test]
fn every_pattern_shape_has_a_defined_outcome() {
    let fx = Fixture::new();
    fx.file("repo/area/pkg/a-spec.md");
    fx.file("repo/area/pkg/x/spec.md");
    fx.file("repo/fixes/one/spec.md");
    fx.file("repo/area/pkg/$x/a.md");

    let control = Outcome::Listed(vec![
        "repo/area/pkg/a-spec.md".into(),
        "repo/area/pkg/x/spec.md".into(),
        "repo/fixes/one/spec.md".into(),
    ]);
    let rows: Vec<(&str, Vec<&str>, Outcome)> = vec![
        ("control", vec!["^**/*spec*.md"], control),
        ("empty list", vec![], Outcome::NoPositivePattern),
        ("only negations", vec!["!&x/**"], Outcome::NoPositivePattern),
        ("empty string", vec![""], Outcome::MalformedPrefix),
        ("bare `!`", vec!["!"], Outcome::MalformedPrefix),
        ("doubled `!`", vec!["!!x.md"], Outcome::MalformedPrefix),
        ("sigil with no payload", vec!["^"], Outcome::MalformedPrefix),
        ("directory with no file", vec!["^docs/"], Outcome::MalformedPrefix),
        ("unclosed interpolation", vec!["{{X/*.md"], Outcome::MalformedPrefix),
        ("`~user`", vec!["~user/*.md"], Outcome::MalformedPrefix),
        ("unsupported scheme", vec!["ftp:x/*.md"], Outcome::MalformedPrefix),
        (
            "remote",
            vec!["https://x/*.md"],
            Outcome::RejectedPrefix(ResolutionFailure::UnsupportedRemote),
        ),
        (
            "`%`",
            vec!["%**/*.md"],
            Outcome::RejectedPrefix(ResolutionFailure::InvalidReference),
        ),
        (
            "`%` after `!`",
            vec!["^**/*.md", "!%x.md"],
            Outcome::RejectedPrefix(ResolutionFailure::InvalidReference),
        ),
        ("invalid glob", vec!["^**/[x"], Outcome::InvalidGlob),
        (
            "duplicate patterns",
            vec!["&**/spec.md", "&**/spec.md"],
            Outcome::Listed(vec![
                "repo/fixes/one/spec.md".into(),
                "repo/area/pkg/x/spec.md".into(),
            ]),
        ),
        // `$` is not a sigil in the reference grammar: `$x/a.md` is a bare
        // single-file reference too, so the glob reads it the same way.
        (
            "`$` is a bare name",
            vec!["$x/*.md"],
            Outcome::Listed(vec!["repo/area/pkg/$x/a.md".into()]),
        ),
    ];
    for (shape, patterns, expected) in rows {
        assert_eq!(outcome(&patterns, &fx), expected, "shape: {shape} ({patterns:?})");
    }

    // `!` stays the removed sigil in a single-file reference.
    assert!(matches!(
        FileReference::new("!x"),
        Err(FileReferenceError::InvalidSyntax(message)) if message.contains("`!` file-reference sigil was removed")
    ));
    // The same `!x` is an exclusion in a glob reference.
    assert_eq!(
        listed(&["&**/spec.md", "!&fixes/**"], &fx.ctx(&fx.pkg)),
        [fx.pkg.join("x/spec.md")]
    );
}

#[test]
fn an_invalid_glob_names_the_pattern_and_the_cause() {
    let error = GlobReference::new(["*.md", "^**/[x"]).unwrap_err();
    assert!(matches!(
        &error,
        GlobReferenceError::InvalidGlob { pattern, .. } if pattern == "^**/[x"
    ));
    assert_eq!(error.resolution_failure(), ResolutionFailure::InvalidReference);
}

#[test]
fn patterns_are_kept_as_authored() {
    let globs = GlobReference::new(["^**/*.md", "!_*.md"]).unwrap();
    assert_eq!(globs.patterns().collect::<Vec<_>>(), ["^**/*.md", "!_*.md"]);
}
