//! Tests for the schema-free `non-deterministic-find` lints (acceptance
//! B-4): ambiguous scalars, suspicious empty values, block-scalar smells,
//! comment truncation and indicator smells, style/indentation
//! inconsistency, and similar/misplaced keys.
//!
//! Every lint is a suggestion, never an error: each carries the
//! `NonDeterministicFind` classification and no repair, and applying the
//! analysis always leaves the source byte-identical. Suppression and
//! confidence boundaries keep common intentional YAML quiet; the reason for
//! every heuristic threshold is recorded next to the test that pins it.

use super::super::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, analyze_yaml};

fn lints(source: &str, code: YamlDiagnosticCode) -> Vec<YamlDiagnostic> {
    analyze_yaml(source)
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .cloned()
        .collect()
}

fn ambiguous(source: &str) -> Vec<YamlDiagnostic> {
    lints(source, YamlDiagnosticCode::AmbiguousScalar)
}

/// Every lint diagnostic must be report-only with no repair, and the source
/// must survive application byte-identical.
fn assert_lint_invariants(source: &str) {
    let analysis = analyze_yaml(source);
    for diagnostic in analysis.diagnostics() {
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "lints are never auto-applied: {diagnostic:?}"
        );
        assert!(diagnostic.repairs.is_empty(), "{diagnostic:?}");
    }
    let outcome = analysis.apply();
    assert_eq!(outcome.source, source, "source must stay byte-identical");
}

/// One lint of `code` covering `covered` in `source`, with the message
/// containing `expected`.
fn assert_single_lint(
    code: YamlDiagnosticCode,
    source: &str,
    covered: &str,
    expected: &str,
) -> YamlDiagnostic {
    let diagnostics = lints(source, code);
    assert_eq!(diagnostics.len(), 1, "for {source:?}: got {diagnostics:?}");
    let diagnostic = diagnostics.into_iter().next().unwrap();
    assert_eq!(
        &source[diagnostic.span.clone()],
        covered,
        "span must cover the offending text in {source:?}"
    );
    assert!(
        diagnostic.message.contains(expected),
        "message {:?} must contain {expected:?}",
        diagnostic.message
    );
    assert_eq!(diagnostic.classification, YamlCertainty::NonDeterministicFind);
    assert_lint_invariants(source);
    diagnostic
}

// ===== Ambiguous scalars =====

#[test]
fn test_ambiguous_version_like_float() {
    // The research example: a trailing-zero float loses digits on parse.
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "release: 1.20\n",
        "1.20",
        "parses as the number 1.2",
    );
}

#[test]
fn test_ambiguous_trailing_dot_float() {
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "ratio: 1.\n",
        "1.",
        "loses trailing digits",
    );
}

#[test]
fn test_canonical_float_is_quiet() {
    // Threshold reason: `1.5` and `3.14` parse losslessly and are
    // overwhelmingly intentional numbers.
    assert!(ambiguous("ratio: 1.5\n").is_empty());
    assert!(ambiguous("pi: 3.14\n").is_empty());
}

#[test]
fn test_plain_integer_is_quiet() {
    // Threshold reason: plain decimal integers (ports, counts, replicas)
    // are the most common intentional scalars in configuration.
    assert!(ambiguous("port: 8080\n").is_empty());
    assert!(ambiguous("replicas: 3\n").is_empty());
}

#[test]
fn test_non_canonical_bool_spellings() {
    for (source, covered) in [
        ("enabled: TRUE\n", "TRUE"),
        ("enabled: True\n", "True"),
        ("enabled: FALSE\n", "FALSE"),
    ] {
        assert_single_lint(
            YamlDiagnosticCode::AmbiguousScalar,
            source,
            covered,
            "parses as the boolean",
        );
    }
}

#[test]
fn test_canonical_bool_spellings_are_quiet() {
    // Threshold reason: lowercase `true`/`false` are the canonical YAML 1.2
    // spellings and unambiguous in every supported dialect.
    assert!(ambiguous("enabled: true\n").is_empty());
    assert!(ambiguous("enabled: false\n").is_empty());
}

#[test]
fn test_yaml_1_1_bool_spellings_are_dialect_traps() {
    // serde_yaml_ng implements YAML 1.2 (yes/no/on/off are strings), but
    // YAML 1.1 tools read booleans — the research's portability trap.
    for spelling in ["yes", "Yes", "YES", "no", "on", "ON", "off", "Off"] {
        let source = format!("enabled: {spelling}\n");
        let diagnostics = ambiguous(&source);
        assert_eq!(diagnostics.len(), 1, "for {source:?}");
        assert!(diagnostics[0].message.contains("YAML 1.1"));
    }
    assert_lint_invariants("enabled: yes\n");
}

#[test]
fn test_non_canonical_null_spellings() {
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "answer: ~\n",
        "~",
        "parses as null",
    );
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "answer: Null\n",
        "Null",
        "parses as null",
    );
}

#[test]
fn test_canonical_null_is_quiet() {
    // Threshold reason: explicit `null` is the canonical spelling and
    // common intentional YAML.
    assert!(ambiguous("answer: null\n").is_empty());
}

#[test]
fn test_infinity_and_nan_forms() {
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "limit: .inf\n",
        ".inf",
        "parses as infinity",
    );
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "limit: .nan\n",
        ".nan",
        "parses as NaN",
    );
}

#[test]
fn test_non_decimal_integers() {
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "mask: 0x1F\n",
        "0x1F",
        "parses as the number 31",
    );
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "mode: 0o17\n",
        "0o17",
        "parses as the number 15",
    );
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "bits: 0b101\n",
        "0b101",
        "parses as the number 5",
    );
}

#[test]
fn test_scientific_notation() {
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "rate: 1e6\n",
        "1e6",
        "scientific notation",
    );
}

#[test]
fn test_leading_zero_digit_strings() {
    // serde_yaml_ng keeps `02139` a string; YAML 1.1 tools read a number —
    // the ZIP-code trap from the research.
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "zip: 02139\n",
        "02139",
        "leading zero",
    );
    // A single zero is an ordinary integer.
    assert!(ambiguous("count: 0\n").is_empty());
}

#[test]
fn test_timestamp_shaped_scalars() {
    // serde_yaml_ng keeps timestamps as strings; many YAML tools produce
    // date objects — portability note only.
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "date: 2026-07-14\n",
        "2026-07-14",
        "timestamp",
    );
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "stamp: 2026-07-14T10:00:00Z\n",
        "2026-07-14T10:00:00Z",
        "timestamp",
    );
}

#[test]
fn test_semver_and_prefixed_versions_are_quiet() {
    // Threshold reason: `1.2.3` and `v1.2` are strings in every YAML
    // dialect — there is nothing ambiguous about them.
    assert!(ambiguous("version: 1.2.3\n").is_empty());
    assert!(ambiguous("version: v1.2\n").is_empty());
}

#[test]
fn test_non_string_mapping_keys() {
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "80: port\n",
        "80",
        "mapping key `80` parses as the number 80",
    );
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "true: enabled\n",
        "true",
        "mapping key `true` parses as the boolean",
    );
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "~: nothing\n",
        "~",
        "mapping key `~` parses as null",
    );
}

#[test]
fn test_dialect_trap_mapping_key() {
    let diagnostics = ambiguous("yes: affirmative\n");
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert!(diagnostics[0].message.contains("mapping key"));
    assert!(diagnostics[0].message.contains("YAML 1.1"));
}

#[test]
fn test_quoted_variants_are_quiet() {
    // Threshold reason: quotes make the string intent explicit — nothing
    // is ambiguous once the author has quoted.
    assert!(ambiguous("release: \"1.20\"\n").is_empty());
    assert!(ambiguous("release: '1.20'\n").is_empty());
    assert!(ambiguous("\"80\": port\n").is_empty());
    assert!(ambiguous("enabled: \"yes\"\n").is_empty());
}

#[test]
fn test_nested_ambiguous_scalar() {
    let source = "packages:\n  release: 1.20\n";
    let diagnostic = assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        source,
        "1.20",
        "parses as the number 1.2",
    );
    assert!(diagnostic.span.start > source.find("packages:").unwrap());
}

#[test]
fn test_sequence_entry_ambiguous_scalar() {
    assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        "versions:\n  - 1.20\n",
        "1.20",
        "parses as the number 1.2",
    );
}

#[test]
fn test_flow_scalars_are_out_of_scope() {
    // Documented boundary: flow-collection scalars are not enumerated by
    // the v1 lint surface, so `[1.20, yes]` produces nothing even though
    // the same lexemes would be flagged in block position.
    assert!(ambiguous("[1.20, yes]\n").is_empty());
    assert!(ambiguous("key: [TRUE, 0x1F]\n").is_empty());
}

#[test]
fn test_multiline_plain_scalar_is_not_misflagged() {
    // `1.20\n  continued` parses as the *string* "1.20 continued", so
    // flagging the first line as a float would be a false positive.
    assert!(ambiguous("key: 1.20\n  continued\n").is_empty());
}

#[test]
fn test_anchored_value_is_not_misflagged() {
    // The value region of `a` includes the anchor; the scalar is not
    // cleanly isolatable, so the lint stays out.
    assert!(ambiguous("a: &x 1.20\nb: *x\n").is_empty());
}

#[test]
fn test_comment_after_ambiguous_value_is_preserved() {
    let source = "release: 1.20 # keep this note\n";
    let diagnostic = assert_single_lint(
        YamlDiagnosticCode::AmbiguousScalar,
        source,
        "1.20",
        "parses as the number 1.2",
    );
    assert!(diagnostic.span.end <= source.find('#').unwrap());
    let outcome = analyze_yaml(source).apply();
    assert!(outcome.source.contains("# keep this note"));
}

#[test]
fn test_windows_paths_and_urls_are_quiet() {
    // Threshold reason: URLs and Windows paths are ordinary strings in
    // every YAML dialect; only indicator-adjacent surprises are reported.
    assert!(ambiguous("url: http://example.com\n").is_empty());
    assert!(ambiguous("path: C:\\Users\\Ken\n").is_empty());
}

// ===== Suspicious empty values =====

#[test]
fn test_empty_mapping_value() {
    assert_single_lint(
        YamlDiagnosticCode::SuspiciousEmptyValue,
        "timeout:\n",
        "timeout",
        "resolves to null",
    );
}

#[test]
fn test_empty_sequence_entry() {
    let source = "features:\n  -\n";
    let diagnostics = lints(source, YamlDiagnosticCode::SuspiciousEmptyValue);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "-");
    // The parent key is a container and must not be flagged.
    assert!(&source[diagnostics[0].span.clone()] != "features");
    assert_lint_invariants(source);
}

#[test]
fn test_container_keys_are_quiet() {
    // Threshold reason: an empty inline value that parents a nested block
    // is intentional structure, not an accidental null.
    assert!(lints("features:\n  - a\n", YamlDiagnosticCode::SuspiciousEmptyValue).is_empty());
    assert!(lints("a:\n  b: 1\n", YamlDiagnosticCode::SuspiciousEmptyValue).is_empty());
    assert!(lints("a:\n  # a note\n  b: 1\n", YamlDiagnosticCode::SuspiciousEmptyValue).is_empty());
}

#[test]
fn test_explicit_null_is_not_an_empty_value() {
    // Boundary: `key: null` is an explicit spelling — at most the
    // ambiguous-scalar lint applies, never the empty-value lint.
    assert!(lints("key: null\n", YamlDiagnosticCode::SuspiciousEmptyValue).is_empty());
}

#[test]
fn test_empty_value_with_prose_comment() {
    // A null placeholder with a note is still a possible accidental null;
    // only *tight* comments defer to comment-truncation.
    assert_single_lint(
        YamlDiagnosticCode::SuspiciousEmptyValue,
        "timeout: # fill me in\n",
        "timeout",
        "resolves to null",
    );
}

#[test]
fn test_nested_empty_value() {
    let source = "a:\n  b:\n";
    let diagnostics = lints(source, YamlDiagnosticCode::SuspiciousEmptyValue);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "b");
    assert_lint_invariants(source);
}

// ===== Block scalar smells =====

#[test]
fn test_folded_scalar_with_shell_commands() {
    // The research example: a folded script collapses onto one line.
    assert_single_lint(
        YamlDiagnosticCode::BlockScalarSmell,
        "script: >\n  echo first\n  echo second\n",
        ">",
        "use `|` if they are meant to stay separate",
    );
}

#[test]
fn test_literal_scalar_is_never_flagged() {
    assert!(lints("script: |\n  echo first\n  echo second\n", YamlDiagnosticCode::BlockScalarSmell)
        .is_empty());
}

#[test]
fn test_folded_prose_is_quiet() {
    // Threshold reason: sentence-final punctuation signals intentional
    // prose, and folding prose is the whole point of `>`.
    let source = "notes: >\n  this is a sentence.\n  another sentence.\n";
    assert!(lints(source, YamlDiagnosticCode::BlockScalarSmell).is_empty());
}

#[test]
fn test_single_line_fold_is_quiet() {
    // Threshold reason: a one-line folded scalar is prose-shaped; there is
    // no join to be surprised by.
    let source = "desc: >\n  just one line\n";
    assert!(lints(source, YamlDiagnosticCode::BlockScalarSmell).is_empty());
}

#[test]
fn test_fold_without_structural_signal_is_quiet() {
    // Threshold reason: without a structural signal the detector would be
    // guessing vocabulary; unpunctuated prose fragments stay quiet.
    let source = "notes: >\n  alpha beta\n  gamma delta\n";
    assert!(lints(source, YamlDiagnosticCode::BlockScalarSmell).is_empty());
}

#[test]
fn test_fold_with_shell_operators() {
    let source = "pipe: >\n  cargo build &&\n  cargo test\n";
    assert_single_lint(YamlDiagnosticCode::BlockScalarSmell, source, ">", "folded scalar");
}

#[test]
fn test_fold_with_template_markers() {
    let source = "tmpl: >\n  hello {{name}}\n  bye {{other}}\n";
    assert_single_lint(YamlDiagnosticCode::BlockScalarSmell, source, ">", "folded scalar");
}

#[test]
fn test_fold_with_pem_material() {
    let source = "key: >\n  -----BEGIN KEY-----\n  abcdef\n";
    assert_single_lint(YamlDiagnosticCode::BlockScalarSmell, source, ">", "folded scalar");
}

#[test]
fn test_fold_with_shell_prompts() {
    let source = "run: >\n  $ cargo build\n  $ cargo test\n";
    assert_single_lint(YamlDiagnosticCode::BlockScalarSmell, source, ">", "folded scalar");
}

#[test]
fn test_fold_with_chomping_modifier() {
    let source = "script: >-\n  echo a\n  echo b\n";
    assert_single_lint(YamlDiagnosticCode::BlockScalarSmell, source, ">", "folded scalar");
}

// ===== Comment truncation and indicator smells =====

#[test]
fn test_comment_in_value_position() {
    // The research example: `color` resolves to null and `#fff` reads as
    // the intended value.
    let source = "color: #fff\n";
    let diagnostic = assert_single_lint(
        YamlDiagnosticCode::CommentTruncation,
        source,
        "#fff",
        "the value of `color` is null",
    );
    assert_eq!(diagnostic.span.start, source.find('#').unwrap());
}

#[test]
fn test_tight_trailing_comment_truncates_value() {
    // The research example: the value is only `abc`, not `abc #123`.
    assert_single_lint(
        YamlDiagnosticCode::CommentTruncation,
        "token: abc #123\n",
        "#123",
        "the value of `token` is only `abc`",
    );
}

#[test]
fn test_spaced_comments_are_quiet() {
    // Threshold reason: `# ` reads as intentional prose (the yamllint
    // comment-spacing convention); only tight comments look like content.
    assert!(lints("color: # a hex color\n", YamlDiagnosticCode::CommentTruncation).is_empty());
    assert!(lints("token: abc # 123\n", YamlDiagnosticCode::CommentTruncation).is_empty());
}

#[test]
fn test_double_hash_comments_are_quiet() {
    // Threshold reason: `##` reads as a heading-style comment, not a value.
    assert!(lints("key: ##section\n", YamlDiagnosticCode::CommentTruncation).is_empty());
}

#[test]
fn test_quoted_value_with_comment_is_quiet() {
    // Threshold reason: quotes already mark the value boundary, so a
    // trailing tight comment cannot be mistaken for content.
    assert!(lints("token: \"abc\" #123\n", YamlDiagnosticCode::CommentTruncation).is_empty());
}

#[test]
fn test_sequence_entry_comment_truncation() {
    assert_single_lint(
        YamlDiagnosticCode::CommentTruncation,
        "- #123\n",
        "#123",
        "the sequence entry is null",
    );
    assert_single_lint(
        YamlDiagnosticCode::CommentTruncation,
        "- abc #123\n",
        "#123",
        "the entry is only `abc`",
    );
}

#[test]
fn test_block_scalar_content_is_not_comment_truncation() {
    // `#` inside a block scalar is literal text, never a comment.
    assert!(lints("script: |\n  echo # not a comment\n", YamlDiagnosticCode::CommentTruncation)
        .is_empty());
}

#[test]
fn test_double_quoted_windows_path_escape_smell() {
    // `"C:\new\tmp"` is valid YAML whose escapes silently change the path
    // (`\n` is a newline, `\t` a tab) — the research's Windows-path trap.
    let source = "path: \"C:\\new\\tmp\"\n";
    let diagnostics = lints(source, YamlDiagnosticCode::CommentTruncation);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert!(diagnostics[0].message.contains("escape sequences"));
    assert!(diagnostics[0].message.contains("single quotes"));
    assert_lint_invariants(source);
}

#[test]
fn test_single_quoted_and_plain_windows_paths_are_quiet() {
    // Single quotes interpret no escapes; plain scalars have no escapes to
    // misread.
    assert!(lints("path: 'C:\\new\\tmp'\n", YamlDiagnosticCode::CommentTruncation).is_empty());
    assert!(lints("path: C:\\new\\tmp\n", YamlDiagnosticCode::CommentTruncation).is_empty());
}

#[test]
fn test_escaped_backslash_windows_path_is_quiet() {
    // `"C:\\Users\\Ken"` correctly escapes every backslash — nothing is
    // surprising.
    assert!(lints("path: \"C:\\\\Users\\\\Ken\"\n", YamlDiagnosticCode::CommentTruncation)
        .is_empty());
}

// ===== Style and indentation inconsistency =====

#[test]
fn test_mixed_indentation_widths() {
    let source = "a:\n  x: 1\nb:\n    y: 2\n";
    let diagnostics = lints(source, YamlDiagnosticCode::StyleInconsistency);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert!(diagnostics[0].message.contains("2 and 4"));
    assert!(diagnostics[0].message.contains("most of the document uses 2"));
    assert_eq!(&source[diagnostics[0].span.clone()], "    y: 2");
    assert_lint_invariants(source);
}

#[test]
fn test_consistent_indentation_is_quiet() {
    assert!(lints("a:\n  x: 1\nb:\n  y: 2\n", YamlDiagnosticCode::StyleInconsistency).is_empty());
}

#[test]
fn test_mixed_boolean_spellings() {
    let source = "a: true\nb: True\nc: true\n";
    let diagnostics = lints(source, YamlDiagnosticCode::StyleInconsistency);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "True");
    assert!(diagnostics[0].message.contains("differs from `true`"));
    assert_lint_invariants(source);
}

#[test]
fn test_boolean_spelling_tie_resolves_to_canonical() {
    // Threshold reason: with no majority, the canonical YAML 1.2 lowercase
    // spelling is the reference.
    let source = "a: TRUE\nb: true\n";
    let diagnostics = lints(source, YamlDiagnosticCode::StyleInconsistency);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "TRUE");
}

#[test]
fn test_uniform_boolean_spellings_are_quiet() {
    assert!(lints("a: true\nb: true\n", YamlDiagnosticCode::StyleInconsistency).is_empty());
    assert!(lints("a: True\n", YamlDiagnosticCode::StyleInconsistency).is_empty());
}

#[test]
fn test_mixed_quote_styles_are_deliberately_quiet() {
    // Threshold reason: escapes legitimately force double quotes, so mixed
    // quote styles are idiomatic and must not be reported.
    assert!(lints("a: 'x'\nb: \"y\"\n", YamlDiagnosticCode::StyleInconsistency).is_empty());
}

// ===== Similar and misplaced keys =====

#[test]
fn test_similar_keys_across_sibling_scopes() {
    // The research example: the repeated structure makes `timeuot`
    // suspicious.
    let source = "development:\n  timeout: 10\nproduction:\n  timeuot: 30\n";
    let diagnostics = lints(source, YamlDiagnosticCode::SimilarKey);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "timeuot");
    assert!(diagnostics[0].message.contains("similar to `timeout`"));
    assert!(diagnostics[0].message.contains("line 2"));
    assert_eq!(diagnostics[0].classification, YamlCertainty::NonDeterministicFind);
    assert_lint_invariants(source);
}

#[test]
fn test_similar_keys_within_one_scope() {
    let source = "a:\n  timeout: 1\n  timeuot: 2\n";
    let diagnostics = lints(source, YamlDiagnosticCode::SimilarKey);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "timeuot");
    assert_lint_invariants(source);
}

#[test]
fn test_identical_keys_across_sibling_scopes_are_quiet() {
    // Threshold reason: repeated key names across parallel scopes are the
    // normal configuration shape, not a typo.
    assert!(lints("dev:\n  host: a\nprod:\n  host: b\n", YamlDiagnosticCode::SimilarKey).is_empty());
}

#[test]
fn test_distant_keys_are_quiet() {
    assert!(lints("a:\n  timeout: 1\nb:\n  retries: 2\n", YamlDiagnosticCode::SimilarKey).is_empty());
}

#[test]
fn test_short_keys_are_quiet() {
    // Threshold reason: under three characters every pair is close; the
    // noise floor swallows the signal.
    assert!(lints("a:\n  ab: 1\nb:\n  ac: 2\n", YamlDiagnosticCode::SimilarKey).is_empty());
}

#[test]
fn test_separator_variants_are_similar() {
    // `max_count` versus `max-count`: the classic kebab/snake confusion.
    let source = "a:\n  max_count: 1\nb:\n  max-count: 2\n";
    let diagnostics = lints(source, YamlDiagnosticCode::SimilarKey);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "max-count");
}

#[test]
fn test_case_variants_are_similar() {
    let source = "a:\n  Timeout: 1\nb:\n  timeout: 2\n";
    let diagnostics = lints(source, YamlDiagnosticCode::SimilarKey);
    assert_eq!(diagnostics.len(), 1, "got {diagnostics:?}");
    assert_eq!(&source[diagnostics[0].span.clone()], "timeout");
}

// ===== Cross-cutting lint behavior =====

#[test]
fn test_resolver_cross_checked_against_parser() {
    // The lint resolver must mirror serde_yaml_ng's YAML 1.2 core-schema
    // resolution exactly; this pins the two together lexeme by lexeme.
    let cases: &[(&str, &str)] = &[
        ("null", "null"), ("Null", "null"), ("NULL", "null"), ("~", "null"),
        ("true", "bool"), ("True", "bool"), ("FALSE", "bool"),
        ("0", "int"), ("-0", "int"), ("+12", "int"), ("80", "int"),
        ("0x1F", "int"), ("0o17", "int"), ("0b101", "int"),
        ("007", "string"), ("1e6", "float"), ("1.20", "float"), ("1.5", "float"),
        ("0.50", "float"), ("1.", "float"), (".5", "float"), ("-.5", "float"),
        (".inf", "float"), ("-.inf", "float"), (".nan", "float"), (".NaN", "float"),
        ("yes", "string"), ("on", "string"), ("off", "string"),
        ("2026-07-14", "string"), ("1.2.3", "string"), ("v1.2", "string"),
        ("1_000", "string"), ("12:34:56", "string"), ("hello world", "string"),
    ];
    for (lexeme, expected) in cases {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(lexeme).unwrap();
        let actual = match &value {
            serde_yaml_ng::Value::Null => "null",
            serde_yaml_ng::Value::Bool(_) => "bool",
            serde_yaml_ng::Value::Number(number) if number.is_i64() || number.is_u64() => "int",
            serde_yaml_ng::Value::Number(_) => "float",
            serde_yaml_ng::Value::String(_) => "string",
            other => panic!("unexpected value kind for {lexeme:?}: {other:?}"),
        };
        assert_eq!(actual, *expected, "parser resolution of {lexeme:?}");
        // And the analyzer's own finding (when present) must agree with the
        // parser's resolved kind: typed lexemes are described with a
        // "parses as …" claim matching the parser's kind; string lexemes
        // may only carry a portability note ("parses as a string here
        // but…"), never a typed-value claim.
        let source = format!("key: {lexeme}\n");
        for diagnostic in ambiguous(&source) {
            match actual {
                "null" => assert!(diagnostic.message.contains("parses as null"), "{lexeme:?}"),
                "bool" => assert!(diagnostic.message.contains("parses as the boolean"), "{lexeme:?}"),
                "int" => assert!(diagnostic.message.contains("parses as the number"), "{lexeme:?}"),
                "float" => assert!(
                    diagnostic.message.contains("parses as the number")
                        || diagnostic.message.contains("parses as infinity")
                        || diagnostic.message.contains("parses as NaN"),
                    "{lexeme:?}"
                ),
                _ => assert!(
                    diagnostic.message.contains("parses as a string here but"),
                    "a string lexeme must never carry a typed-value claim: {lexeme:?} — {:?}",
                    diagnostic.message
                ),
            }
        }
    }
}

#[test]
fn test_lint_findings_sorted_in_source_order() {
    let source = "release: 1.20\ntimeout:\nscript: >\n  echo a\n  echo b\n";
    let analysis = analyze_yaml(source);
    assert!(analysis.diagnostics().len() >= 3);
    let spans: Vec<_> = analysis
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.span.start, diagnostic.span.end))
        .collect();
    let mut sorted = spans.clone();
    sorted.sort();
    assert_eq!(spans, sorted, "diagnostics must be in stable source order");
}

#[test]
fn test_mixed_deterministic_and_report_only_findings() {
    // A deterministic S1 repair (trailing whitespace) coexists with a
    // report-only lint (the ambiguous `1.20`): the repair applies, the
    // lint's span is untouched.
    let source = "release: 1.20\nkey: value  \n";
    let analysis = analyze_yaml(source);
    assert!(analysis
        .diagnostics()
        .iter()
        .any(|d| d.classification == YamlCertainty::Deterministic));
    assert_eq!(ambiguous(source).len(), 1);
    let outcome = analysis.apply();
    assert_eq!(outcome.source, "release: 1.20\nkey: value\n");
    assert!(outcome.source.contains("1.20"), "the lint must never be applied");
}

#[test]
fn test_lints_deterministic_across_runs() {
    let source = "release: 1.20\ntimeout:\ncolor: #fff\ndev:\n  timeout: 1\nprod:\n  timeuot: 2\n";
    let first: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    let second: Vec<_> = analyze_yaml(source).diagnostics().to_vec();
    assert_eq!(first, second);
}
