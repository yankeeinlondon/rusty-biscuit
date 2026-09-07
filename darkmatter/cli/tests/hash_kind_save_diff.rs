mod common;

use common::md_cmd;
use predicates::prelude::*;


// =============================================================================
//                     HASH KIND / SAVE / DIFF TESTS
// =============================================================================

#[test]
fn test_hash_kind_structured_outputs_four_parts() {
    md_cmd()
        .args(["hash", "--kind", "structured", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}\n$")
                .unwrap(),
        );
}

#[test]
fn test_hash_kind_structured_strict_outputs_four_parts() {
    md_cmd()
        .args(["hash", "--kind", "structured", "--strict", "-"])
        .write_stdin("---\nbeta: 1\nalpha: 2\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}\n$")
                .unwrap(),
        );
}

#[test]
fn test_hash_kind_structured_strict_respects_key_order() {
    let reordered = |args: &[&str]| {
        let beta_first = md_cmd()
            .args(args)
            .write_stdin("---\nbeta: 1\nalpha: 2\n---\n# H\n\nBody.")
            .output()
            .unwrap()
            .stdout;
        let alpha_first = md_cmd()
            .args(args)
            .write_stdin("---\nalpha: 2\nbeta: 1\n---\n# H\n\nBody.")
            .output()
            .unwrap()
            .stdout;
        (beta_first, alpha_first)
    };

    // Strict preserves key order, so reordering keys changes the hash.
    let (strict_beta, strict_alpha) = reordered(&["hash", "--kind", "structured", "--strict", "-"]);
    assert_ne!(strict_beta, strict_alpha, "strict must not reorder keys");

    // Non-strict sorts keys, so reordering is a no-op.
    let (ns_beta, ns_alpha) = reordered(&["hash", "--kind", "structured", "-"]);
    assert_eq!(ns_beta, ns_alpha, "non-strict sorts keys");
}

#[test]
fn test_hash_diff_malformed_stored_hash_exits_one() {
    // A corrupt stored hash is an operational error (exit 1), never a content
    // difference (exit 2).
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(
        &file,
        "---\ntitle: T\nhash: not-a-real-hash-but-two-parts\n---\n# H\n\nBody.\n",
    )
    .unwrap();

    md_cmd().arg("hash").arg("--diff").arg(&file).assert().code(1);
}

#[test]
fn test_hash_diff_detailed_bad_section_level_exits_one() {
    // A stored detailed hash whose section level is outside 1-6 is a malformed
    // baseline (operational error, exit 1), never a content difference (exit 2).
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(
        &file,
        concat!(
            "---\n",
            "title: T\n",
            "hash:\n",
            "  kind: detailed\n",
            "  value:\n",
            "    frontmatter:\n",
            "      fm: \"0000000000000001\"\n",
            "      keys: \"0000000000000002\"\n",
            "    preamble: null\n",
            "    sections:\n",
            "      - [9, \"Bad\", \"0000000000000004\"]\n",
            "---\n",
            "# H\n\nBody.\n",
        ),
    )
    .unwrap();

    md_cmd().arg("hash").arg("--diff").arg(&file).assert().code(1);
}

#[test]
fn test_hash_kind_detailed_outputs_nested_yaml() {
    md_cmd()
        .args(["hash", "--kind", "detailed", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("frontmatter:"))
        .stdout(predicate::str::contains("sections:"));
}

#[test]
fn test_hash_kind_fm_matches_frontmatter_flag() {
    let input = "---\ntitle: Hello\n---\n# Content";
    let by_kind = md_cmd()
        .args(["hash", "--kind", "fm", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    let by_flag = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert_eq!(by_kind.stdout, by_flag.stdout);
}

#[test]
fn test_hash_kind_conflicts_with_body() {
    md_cmd()
        .args(["hash", "--kind", "fm", "--body", "-"])
        .write_stdin("# H")
        .assert()
        .failure();
}

#[test]
fn test_hash_save_and_diff_conflict() {
    md_cmd()
        .args(["hash", "--save", "--diff", "-"])
        .write_stdin("# H")
        .assert()
        .failure();
}

#[test]
fn test_hash_env_property_override() {
    // HASH_PROPERTY changes which frontmatter key the hash is written to.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    md_cmd()
        .env("HASH_PROPERTY", "fingerprint")
        .arg("hash")
        .arg("--save")
        .arg(&file)
        .assert()
        .success();

    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("fingerprint:"), "got:\n{written}");
    assert!(!written.contains("\nhash:"), "got:\n{written}");
}

#[test]
fn test_hash_ignore_properties_excludes_key() {
    // A document differing only in an ignored property hashes identically.
    let with_draft = md_cmd()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: T\ndraft: true\n---\n# H")
        .output()
        .unwrap();
    let without_draft = md_cmd()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: T\n---\n# H")
        .output()
        .unwrap();
    assert_eq!(with_draft.stdout, without_draft.stdout);
}

#[test]
fn test_hash_save_writes_baseline_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    md_cmd()
        .arg("hash")
        .arg("--save")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("baseline"));

    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("hash:"), "got:\n{written}");
}

#[test]
fn test_hash_save_preserves_raw_frontmatter_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    let source = concat!(
        "---\r\n",
        "title: T # keep\r\n",
        "prompt: |-\r\n",
        "    Keep trailing space  \r\n",
        "\r\n",
        "    Keep indentation.\r\n",
        "---\r\n",
        "# H\r\n\r\nBody.\r\n"
    );
    std::fs::write(&file, source).unwrap();

    md_cmd().arg("hash").arg("--save").arg(&file).assert().success();
    let first = std::fs::read_to_string(&file).unwrap();
    assert!(first.contains("title: T # keep\r\nprompt: |-\r\n    Keep trailing space  \r\n"));
    assert!(first.ends_with("---\r\n# H\r\n\r\nBody.\r\n"));

    md_cmd().arg("hash").arg("--save").arg(&file).assert().success();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), first);
    md_cmd().arg("hash").arg("--diff").arg(&file).assert().success();
}

#[test]
fn test_hash_save_failure_does_not_modify_flow_mapping() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    let source = "---\n{title: T}\n---\nBody.\n";
    std::fs::write(&file, source).unwrap();

    md_cmd().arg("hash").arg("--save").arg(&file).assert().failure();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
}

#[test]
fn test_hash_save_honors_quoted_custom_property() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    let source = concat!(
        "---\n",
        "title: T # keep\n",
        "'fingerprint': aaaa111111111111-bbbb222222222222\n",
        "---\n",
        "Changed body.\n"
    );
    std::fs::write(&file, source).unwrap();

    md_cmd()
        .env("HASH_PROPERTY", "fingerprint")
        .arg("hash")
        .arg("--save")
        .arg(&file)
        .assert()
        .success();
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("title: T # keep\n'fingerprint': "));
    assert!(!written.contains("\nhash:"));
    md_cmd()
        .env("HASH_PROPERTY", "fingerprint")
        .arg("hash")
        .arg("--diff")
        .arg(&file)
        .assert()
        .success();
}

#[test]
fn test_hash_save_detailed_value_preserves_authored_neighbors() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    let source = "---\ntitle: T # keep\n---\n# Heading\n\nBody.\n";
    std::fs::write(&file, source).unwrap();

    md_cmd()
        .args(["hash", "--kind", "detailed", "--save"])
        .arg(&file)
        .assert()
        .success();
    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("title: T # keep\nhash:\n  kind: detailed\n"));
    assert!(written.ends_with("---\n# Heading\n\nBody.\n"));
    md_cmd().arg("hash").arg("--diff").arg(&file).assert().success();
}

#[test]
fn test_hash_save_requires_file_not_stdin() {
    md_cmd()
        .args(["hash", "--save", "-"])
        .write_stdin("# H")
        .assert()
        .failure();
}

#[test]
fn test_hash_diff_no_stored_hash_exits_two() {
    md_cmd()
        .args(["hash", "--diff", "-"])
        .write_stdin("---\ntitle: T\n---\n# H\n\nBody.")
        .assert()
        .code(2)
        .stdout(predicate::str::contains("No stored hash to compare against"));
}

#[test]
fn test_hash_diff_unchanged_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    // Establish a baseline, then diff against it without any edit.
    md_cmd().arg("hash").arg("--save").arg(&file).assert().success();

    md_cmd()
        .arg("hash")
        .arg("--diff")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("No semantic changes detected"));
}

#[test]
fn test_hash_diff_changed_exits_two() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    md_cmd().arg("hash").arg("--save").arg(&file).assert().success();

    // Edit the body, leaving the stored hash in place.
    let stored = std::fs::read_to_string(&file).unwrap();
    let edited = stored.replace("Body.", "Different body.");
    std::fs::write(&file, edited).unwrap();

    md_cmd().arg("hash").arg("--diff").arg(&file).assert().code(2);
}

#[test]
fn test_hash_save_preservation_matrix_covers_representations_and_newlines() {
    struct Case {
        name: &'static str,
        kind: Option<&'static str>,
        property: &'static str,
        authored_key: &'static str,
        old_node: &'static str,
    }

    let cases = [
        Case {
            name: "simple",
            kind: None,
            property: "hash",
            authored_key: "hash",
            old_node: "hash: 0000000000000000-0000000000000000",
        },
        Case {
            name: "structured",
            kind: Some("structured"),
            property: "hash",
            authored_key: "hash",
            old_node: concat!(
                "hash:\n",
                "  kind: structured\n",
                "  value: 0000000000000000-0000000000000000-0000000000000000-0000000000000000"
            ),
        },
        Case {
            name: "detailed",
            kind: Some("detailed"),
            property: "hash",
            authored_key: "hash",
            old_node: concat!(
                "hash:\n",
                "  kind: detailed\n",
                "  value:\n",
                "    frontmatter:\n",
                "      fm: '0000000000000000'\n",
                "      keys: '0000000000000000'\n",
                "    preamble: null\n",
                "    sections:\n",
                "      - [1, Heading, '0000000000000000']"
            ),
        },
        Case {
            name: "custom-property",
            kind: None,
            property: "fingerprint",
            authored_key: "fingerprint",
            old_node: "fingerprint: 0000000000000000-0000000000000000",
        },
        Case {
            name: "quoted-key",
            kind: None,
            property: "hash",
            authored_key: "'hash'",
            old_node: "'hash': 0000000000000000-0000000000000000",
        },
    ];

    for newline in ["\n", "\r\n"] {
        for case in &cases {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join(format!("{}-doc.md", case.name));
            let prefix = [
                "---",
                "title: Kept # authored",
                "prompt: |-",
                "    First line  ",
                "",
                "    Second line.",
            ]
            .join(newline)
                + newline;
            let old_node = case.old_node.replace('\n', newline);
            let suffix = [
                "# boundary comment",
                "author: A",
                "---",
                "# Heading",
                "",
                "Body with trailing spaces.  ",
                "",
            ]
            .join(newline);
            let source = format!(
                "{prefix}{old_node}{newline}last_updated: '2026-01-01' # managed{newline}{suffix}"
            );
            std::fs::write(&file, source).unwrap();

            let mut save = md_cmd();
            save.arg("hash");
            if let Some(kind) = case.kind {
                save.args(["--kind", kind]);
            }
            if case.property != "hash" {
                save.env("HASH_PROPERTY", case.property);
            }
            save.arg("--save").arg(&file).assert().success();

            let written = std::fs::read_to_string(&file).unwrap();
            assert!(
                written.starts_with(&prefix),
                "{} {newline:?} changed the authored prefix:\n{written}",
                case.name
            );
            assert!(
                written.ends_with(&suffix),
                "{} {newline:?} changed the authored suffix:\n{written}",
                case.name
            );
            assert!(
                written.contains(&format!("{}:", case.authored_key)),
                "{} {newline:?} did not preserve the managed key spelling:\n{written}",
                case.name
            );
            if let Some(kind) = case.kind {
                assert!(
                    written.contains(&format!("kind: {kind}")),
                    "{} {newline:?} did not use longhand output:\n{written}",
                    case.name
                );
            }
            if newline == "\r\n" {
                assert!(
                    !written.replace("\r\n", "").contains('\n'),
                    "{} introduced a bare LF into CRLF output:\n{written}",
                    case.name
                );
            }

            let mut diff = md_cmd();
            diff.arg("hash");
            if case.property != "hash" {
                diff.env("HASH_PROPERTY", case.property);
            }
            diff.arg("--diff").arg(&file).assert().success();
        }
    }
}

#[test]
fn test_hash_save_flow_root_no_write_matrix_covers_newlines() {
    for newline in ["\n", "\r\n"] {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("flow.md");
        let source = [
            "---",
            "{title: Kept, prompt: \"First line  ",
            "  Second line\", hash: 0000000000000000-0000000000000000}",
            "---",
            "First body line.  ",
            "Second body line.",
            "",
        ]
        .join(newline);
        std::fs::write(&file, &source).unwrap();

        md_cmd().arg("hash").arg("--save").arg(&file).assert().failure();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
    }
}
