use crate::common;

use common::{CliProcessFixture, baseline};

#[test]
fn validate_refs_text_output() {
    let fixture = CliProcessFixture::named("validate_refs_text_output");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let md_path = dir.join("test.md");
    std::fs::write(&md_path, "# Heading\n\n[link](https://example.com)\n").unwrap();

    fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .success();
}

#[test]
fn validate_refs_json_output() {
    let fixture = CliProcessFixture::named("validate_refs_json_output");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let md_path = dir.join("test.md");
    std::fs::write(&md_path, "[link](https://example.com)\n").unwrap();

    let output = fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .args(["--output", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON output should be parseable
    let _: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("Expected valid JSON, got error: {e}\nOutput: {stdout}");
    });
}

#[test]
fn validate_refs_nonzero_exit_on_errors() {
    let fixture = CliProcessFixture::named("validate_refs_nonzero_exit_on_errors");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let md_path = dir.join("test.md");
    std::fs::write(&md_path, "[broken](./nonexistent.md)\n").unwrap();

    fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .failure();
}

#[test]
fn validate_refs_with_fragments() {
    let fixture = CliProcessFixture::named("validate_refs_with_fragments");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let md_path = dir.join("test.md");
    std::fs::write(&md_path, "# Hello\n\n[link](#hello)\n").unwrap();

    fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--fragments")
        .assert()
        .success();
}

#[test]
fn validate_refs_graph_mermaid() {
    let fixture = CliProcessFixture::named("validate_refs_graph_mermaid");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let md_path = dir.join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("mermaid")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("flowchart TD"),
        "Expected mermaid flowchart output, got: {stdout}"
    );
}

#[test]
fn validate_refs_graph_dot() {
    let fixture = CliProcessFixture::named("validate_refs_graph_dot");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let md_path = dir.join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("dot")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("digraph"),
        "Expected dot graph output, got: {stdout}"
    );
}

// ── JSON baseline fixtures (byte-for-byte compatibility) ────────────────
//
// `md validate refs --output json` serializes the library
// `ReferenceValidationReport` directly via `Serialize` (the same shape
// `md graph --validate --output json` uses for its `validation` block). These
// tests pin that public shape against the baseline fixtures under
// `darkmatter/features/2026-06-17-cli-atheist/baseline/json/`. Each
// test reproduces the source content used to capture the baseline,
// runs the serde-backed JSON path, normalizes temp paths and any
// remaining FNV-1a reference-id hash prefix, then compares the full
// JSON value.

/// Asserts that `md validate refs --output json <input>` produces output that,
/// after environment normalization, equals the named baseline fixture.
///
/// Note: `md validate refs` exits with a non-zero status when validation
/// finds errors, even in `--output json` mode (the JSON payload is still
/// printed to stdout before the process exits). The baseline fixtures
/// capture both the success and the error shape, so we must not require
/// `status.success()` here.
fn assert_json_matches_baseline(
    fixture: &CliProcessFixture,
    input: &std::path::Path,
    baseline_name: &str,
) {
    let output = fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(input)
        .args(["--output", "json"])
        .output()
        .expect("md command failed to spawn");

    // The CLI always prints the JSON report to stdout even when the
    // subsequent validation summary errors out and sets a non-zero exit.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut actual: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "md validate refs --output json did not produce valid JSON: {e}\n\
                 status: {:?}\nstderr: {}\nstdout: {stdout}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        )
    });

    assert!(output.stderr.is_empty(), "a data output writes nothing to stderr: {output:?}");

    let expected = baseline::load_json(baseline_name);
    // The baseline pins the report's own shape; a failed run adds these two.
    if expected["valid"] == false {
        assert_eq!(output.status.code(), Some(1), "{stdout}");
        assert_eq!(actual["outcome"], "failure", "{stdout}");
        assert!(!actual["diagnostics"].as_array().expect("diagnostics").is_empty(), "{stdout}");
        let report = actual.as_object_mut().expect("a report object");
        report.remove("outcome");
        report.remove("diagnostics");
    } else {
        assert_eq!(output.status.code(), Some(0), "{stdout}");
    }

    let parent = input.parent().expect("input has a parent dir");
    let redact = baseline::paths_to_redact(parent);
    let redact_refs: Vec<&str> = redact.iter().map(|s| s.as_str()).collect();
    let actual_norm = baseline::normalize(actual, &redact_refs);

    let expected_norm = baseline::normalize(expected, &redact_refs);

    assert_eq!(
        actual_norm,
        expected_norm,
        "md validate refs --output json did not match baseline {baseline_name}\n\
         status: {:?}\nraw output:\n{stdout}",
        output.status.code(),
    );
}

#[test]
fn validate_refs_json_local_baseline() {
    let fixture = CliProcessFixture::named("validate_refs_json_local_baseline");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let root = dir;
    std::fs::write(
        root.join("local.md"),
        "# Local Test\n\n[local link](./other.md)\n![local image](./img.png)\n::file other.md\n",
    )
    .unwrap();
    // `other.md` exists so the hyperlink and the `::file` transclusion are
    // both valid; only the image reference at `./img.png` is broken.
    std::fs::write(root.join("other.md"), "# Other\n").unwrap();
    assert_json_matches_baseline(&fixture, &root.join("local.md"), "validate_refs_local.json");
}

#[test]
fn validate_refs_json_remote_baseline() {
    let fixture = CliProcessFixture::named("validate_refs_json_remote_baseline");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let root = dir;
    std::fs::write(
        root.join("remote.md"),
        "# Remote\n\n[remote](https://example.com)\n",
    )
    .unwrap();
    assert_json_matches_baseline(
        &fixture,
        &root.join("remote.md"),
        "validate_refs_remote.json",
    );
}

#[test]
fn validate_refs_json_fragment_baseline() {
    let fixture = CliProcessFixture::named("validate_refs_json_fragment_baseline");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let root = dir;
    std::fs::write(
        root.join("fragment.md"),
        "# Fragment\n\n[fragment](#fragment)\n",
    )
    .unwrap();

    // Fragment validation requires `--fragments`, which the baseline
    // capture also used; run the CLI directly to mirror that flag.
    let output = fixture
        .command()
        .arg("validate")
        .arg("refs")
        .arg(root.join("fragment.md"))
        .arg("--fragments")
        .args(["--output", "json"])
        .output()
        .expect("md command failed to spawn");

    // The CLI always prints the JSON report to stdout even when the
    // subsequent validation summary errors out and sets a non-zero exit.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "md validate refs --json --fragments did not produce valid JSON: {e}\n\
                 status: {:?}\nstderr: {}\nstdout: {stdout}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        )
    });
    let redact = baseline::paths_to_redact(root);
    let redact_refs: Vec<&str> = redact.iter().map(|s| s.as_str()).collect();
    let actual_norm = baseline::normalize(actual, &redact_refs);
    let expected_norm = baseline::normalize(
        baseline::load_json("validate_refs_fragment.json"),
        &redact_refs,
    );
    assert_eq!(
        actual_norm, expected_norm,
        "md validate refs --json --fragments output did not match baseline\n\
         raw output:\n{stdout}",
    );
}

#[test]
fn validate_refs_json_datauri_baseline() {
    let fixture = CliProcessFixture::named("validate_refs_json_datauri_baseline");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let root = dir;
    std::fs::write(
        root.join("datauri.md"),
        "# Data\n\n![data](data:image/png;base64,abc)\n",
    )
    .unwrap();
    assert_json_matches_baseline(
        &fixture,
        &root.join("datauri.md"),
        "validate_refs_datauri.json",
    );
}

#[test]
fn validate_refs_json_inline_baseline() {
    let fixture = CliProcessFixture::named("validate_refs_json_inline_baseline");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let root = dir;
    std::fs::write(
        root.join("inline.md"),
        "# Inline\n\n<style>.x{color:red}</style>\n\n<script src=\"app.js\"></script>\n",
    )
    .unwrap();
    // The baseline capture treats the `<script src>` as a valid local
    // reference because `app.js` exists next to the document.
    std::fs::write(root.join("app.js"), "// inline script target\n").unwrap();
    assert_json_matches_baseline(
        &fixture,
        &root.join("inline.md"),
        "validate_refs_inline.json",
    );
}

#[test]
fn validate_refs_json_errors_baseline() {
    let fixture = CliProcessFixture::named("validate_refs_json_errors_baseline");
    assert!(fixture.initialize_repository());
    let dir = fixture.cwd();
    let root = dir;
    std::fs::write(
        root.join("errors.md"),
        "# Errors\n\n[missing](./missing.md)\n",
    )
    .unwrap();
    // Note: no `missing.md` is written — the test asserts the missing
    // target produces exactly the baseline error shape.
    assert_json_matches_baseline(
        &fixture,
        &root.join("errors.md"),
        "validate_refs_errors.json",
    );
}

/// R10-F2.5: both validating commands fail on a malformed inclusion that
/// `md compose` rejects, naming the document and the directive's line, and
/// pass the valid spelling.
#[test]
fn reference_commands_refuse_a_malformed_inclusion() {
    let fixture = CliProcessFixture::named("reference_commands_malformed_inclusion");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    for (body, code) in [("::file ./side.md", 0), ("::file", 1)] {
        let input = fixture.write_file("cwd/doc.md", &format!("# Doc\n\n{body}\n"));
        for args in [&["validate", "refs"][..], &["graph", "--validate"][..]] {
            let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert_eq!(output.status.code(), Some(code), "{body} {args:?}: {stdout}{}", String::from_utf8_lossy(&output.stderr));
            if code == 1 {
                let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
                let diagnostic = json["diagnostics"]
                    .as_array()
                    .and_then(|all| all.iter().find(|d| d["code"] == "dm.directive.parse_failure"))
                    .unwrap_or_else(|| panic!("{args:?}: {stdout}"));
                assert!(
                    diagnostic["source"]["file"].as_str().is_some_and(|file| file.ends_with("doc.md")),
                    "{args:?}: {diagnostic}"
                );
                assert!(diagnostic["message"].as_str().is_some_and(|m| m.contains("line 3")), "{args:?}: {diagnostic}");
            }
        }
    }
}

/// R10-F2.16–.18: a failure keeps its specific code at every command
/// boundary. A `::toc-linking` or `::file-links` failure `md compose`
/// tolerates is a warning under its own code (and `--deny` takes that code),
/// a `when=` that does not parse is `dm.expression.parse_failure` at its
/// authored line, and an invalid file reference is `dm.reference.failure` from
/// `md graph` as from `md validate refs`.
#[test]
fn commands_keep_specific_codes_for_tolerated_failures_conditions_and_references() {
    let fixture = CliProcessFixture::named("commands_keep_specific_codes");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/files/a.md", "# A\n\n## Part\n");
    fixture.write_file("cwd/notdir.md", "# Not a directory\n");
    let run = |args: &[&str], content: &str| {
        let input = fixture.write_file("cwd/doc.md", content);
        let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{args:?} {content:?}: {e}: {stdout}"));
        (output.status.code(), json)
    };
    let only = |json: &serde_json::Value, context: &str| -> serde_json::Value {
        let [record] = json["diagnostics"].as_array().unwrap().as_slice() else {
            panic!("{context}: one record: {json:#}");
        };
        record.clone()
    };

    for (directive, code) in [
        ("::toc-linking ./files/a.md filter=\"[\"", "dm.toc_linking.invalid_filter"),
        ("::file-links ./files/[.md", "dm.file_links.invalid_glob"),
        ("::file-links --dir ./absent", "dm.reference.failure"),
        ("::file-links --dir ./notdir.md", "dm.file_links.not_a_directory"),
    ] {
        let content = format!("# Doc\n\n{directive}\n");
        let (status, json) = run(&["compose"], &content);
        assert_eq!(status, Some(0), "{directive}: tolerated: {json:#}");
        let record = only(&json, directive);
        assert_eq!((record["code"].as_str(), record["severity"].as_str()), (Some(code), Some("warning")), "{directive}: {record:#}");
        let (status, json) = run(&["compose", "--deny", code], &content);
        assert_eq!(status, Some(1), "{directive}: `--deny {code}` blocks it: {json:#}");
    }
    let stdin = fixture
        .command()
        .args(["compose", "-", "--output", "json"])
        .write_stdin("# Doc\n\n::file-links ./files/*.md\n")
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&stdin.stdout).unwrap();
    assert_eq!(stdin.status.code(), Some(0), "{json:#}");
    assert_eq!(only(&json, "stdin")["code"], "dm.file_links.missing_source", "{json:#}");

    let commands = [&["compose"][..], &["validate", "refs"], &["graph"], &["graph", "--validate"]];
    for args in commands {
        let (status, json) = run(args, "# Doc\n\n::file ./files/a.md when=\"1 +\"\n");
        assert_eq!(status, Some(1), "{args:?}: {json:#}");
        let record = only(&json, "when");
        assert_eq!(record["code"], "dm.expression.parse_failure", "{args:?}: {record:#}");
        assert_eq!(record["source"]["line"], 3, "{args:?}: {record:#}");
    }
    for content in ["# Doc\n\n::file @\n", "---\nprologue: \"@\"\n---\n# Doc\n"] {
        for args in &commands[1..] {
            let (status, json) = run(args, content);
            assert_eq!(status, Some(1), "{content:?} {args:?}: {json:#}");
            let record = only(&json, content);
            assert_eq!(record["code"], "dm.reference.failure", "{content:?} {args:?}: {record:#}");
            assert_eq!(record["failure"], "invalid-reference", "{content:?} {args:?}: {record:#}");
        }
    }
}

/// R10-F2: `md compose` and both reference commands report what they could
/// not inspect alike. A `::toc-linking` or `::file-links` failure carries its
/// own code, an invalid `prologue:` its key's authored line, and a failing
/// `when=` fails all three, as does a child whose frontmatter does not parse
/// (`md compose` refuses it before composing, in its pre-flight walk).
#[test]
fn compose_and_reference_commands_report_inclusion_failures_alike() {
    let fixture = CliProcessFixture::named("compose_and_reference_inclusion_failures");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    fixture.write_file("cwd/bad.md", "---\ntitle: [unclosed\n---\n# Bad\n");
    let compose = &["compose"][..];
    let without_baseline = &["compose", "--disable", "baseline-schema"][..];
    let references = [&["validate", "refs"][..], &["graph", "--validate"][..]];
    let toc = "# Doc\n\n::toc-linking ./side.md level=h9\n";
    let depth = "# Doc\n\n::file-links --dir . --depth two\n";
    let prologue = "---\ntitle: T\nprologue: { path: ./side.md }\n---\n# Doc\n";
    let when = "# Doc\n\n::file ./side.md when=\"1 / 0\"\n";
    let child = "# Doc\n\n::file ./bad.md\n";
    // (document, command, exit, the expected code, its file, its line)
    type Case<'a> = (&'a str, &'a [&'a str], i32, &'a str, &'a str, Option<u64>);
    let mut cases: Vec<Case> = vec![
        // `md compose`'s baseline schema rejects the shape first, at the same
        // line; without it, the parser's own error.
        (prologue, compose, 1, "dm.schema.constraint", "doc.md", Some(3)),
        (prologue, without_baseline, 1, "dm.directive.parse_failure", "doc.md", Some(3)),
    ];
    for command in references {
        cases.push((prologue, command, 1, "dm.directive.parse_failure", "doc.md", Some(3)));
    }
    for command in [compose].into_iter().chain(references) {
        cases.extend([
            (toc, command, 1, "dm.toc_linking.invalid_level", "doc.md", None),
            (depth, command, 1, "dm.file_links.invalid_depth", "doc.md", None),
            (when, command, 1, "dm.expression.evaluation_failure", "doc.md", Some(3)),
        ]);
    }
    for command in [compose].into_iter().chain(references) {
        cases.push((child, command, 1, "dm.frontmatter.yaml_parse", "bad.md", None));
    }
    for (content, args, exit, code, file, line) in cases {
        let input = fixture.write_file("cwd/doc.md", content);
        let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(exit), "{content:?} {args:?}: {stdout}");
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let diagnostic = json["diagnostics"]
            .as_array()
            .and_then(|all| all.iter().find(|d| d["code"] == code))
            .unwrap_or_else(|| panic!("{content:?} {args:?}: no {code}: {stdout}"));
        assert!(
            diagnostic["source"]["file"].as_str().is_some_and(|path| path.ends_with(file)),
            "{content:?} {args:?}: {diagnostic}"
        );
        if let Some(line) = line {
            assert_eq!(diagnostic["source"]["line"].as_u64(), Some(line), "{content:?} {args:?}: {diagnostic}");
        }
        assert!(!diagnostic["message"].as_str().unwrap_or_default().contains("line 0"), "{diagnostic}");
    }
}

/// R11-F1: an inline `prologue`/`epilogue` and a `::toc-linking` chain
/// suppressed by a trailing `false` pass `md compose`'s reference check,
/// `md validate refs`, and `md graph --validate`; a missing link inside the
/// inline text and an exhausted chain without `false` fail all three.
#[test]
fn reference_commands_accept_inline_frontmatter_and_suppressed_toc_chains() {
    let fixture = CliProcessFixture::named("reference_commands_inline_and_suppressed");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    let commands = [&["compose"][..], &["validate", "refs"], &["graph", "--validate"]];
    // (document, exit, the construct of the missing-target record)
    for (content, exit, construct) in [
        ("---\nprologue: \"[Setup](./side.md)\"\n---\n# Doc\n", 0, ""),
        ("---\nepilogue: |\n  Intro\n  [Setup](./side.md)\n---\n# Doc\n", 0, ""),
        ("---\nprologue: \"\"\n---\n# Doc\n", 0, ""),
        ("# Doc\n\n::toc-linking \"./missing.md | ./gone.md | false\"\n", 0, ""),
        ("---\nprologue: \"[Setup](./missing.md)\"\n---\n# Doc\n", 1, "hyperlink"),
        ("# Doc\n\n::toc-linking \"./missing.md | ./gone.md\"\n", 1, "::toc-linking"),
    ] {
        let input = fixture.write_file("cwd/doc.md", content);
        for args in commands {
            let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert_eq!(output.status.code(), Some(exit), "{content:?} {args:?}: {stdout}");
            if exit == 1 {
                let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
                let missing = json["diagnostics"]
                    .as_array()
                    .and_then(|all| all.iter().find(|d| d["failure"] == "no-match"))
                    .unwrap_or_else(|| panic!("{content:?} {args:?}: no missing-target record: {stdout}"));
                assert_eq!(missing["reference"], "./missing.md", "{content:?} {args:?}: {missing:#}");
                assert_eq!(missing["construct"], construct, "{content:?} {args:?}: {missing:#}");
            }
        }
    }
}

/// R11-F2: a malformed body directive after a frontmatter header and a false
/// page block is reported at its line in the file by `md compose` (its
/// command scan and composition), `md graph`, and `md validate refs`, in the
/// message and in the record's `source.line`; the valid spelling passes.
#[test]
fn directive_parse_failures_report_their_authored_line() {
    let fixture = CliProcessFixture::named("directive_parse_failures_authored_line");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    fixture.write_file("cwd/docs/a.md", "# A\n");
    let header = "---\na: 1\nb: 2\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n";
    let commands = [&["compose"][..], &["graph"], &["validate", "refs"]];
    for (directive, code) in [
        ("::file", "dm.directive.parse_failure"),
        ("::toc-linking ./side.md level=h9", "dm.toc_linking.invalid_level"),
        ("::file-links --dir ./docs --depth two", "dm.file_links.invalid_depth"),
        ("::file ./side.md", ""),
    ] {
        let input = fixture.write_file("cwd/doc.md", &format!("{header}{directive}\n"));
        for args in commands {
            let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            if code.is_empty() {
                assert_eq!(output.status.code(), Some(0), "{directive} {args:?}: {stdout}");
                continue;
            }
            assert_eq!(output.status.code(), Some(1), "{directive} {args:?}: {stdout}");
            let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let [record] = json["diagnostics"].as_array().unwrap().as_slice() else {
                panic!("{directive} {args:?}: one record: {stdout}");
            };
            assert_eq!(record["code"], code, "{directive} {args:?}: {record:#}");
            assert_eq!(record["source"]["line"], 10, "{directive} {args:?}: {record:#}");
            assert!(record["message"].as_str().unwrap().contains("line 10"), "{directive} {args:?}: {record:#}");
        }
    }
}

/// R11-F2: `md compose` places a `::shell` line's parse failure at its file
/// line in the JSON record's `source`, past a header and a false page block,
/// pre-flight's approval refusal names the file line of a command that
/// interpolation moved down in the prepared body, and pre-flight refuses a
/// malformed `::shell-block` with its typed code at its file line.
#[test]
fn shell_failures_report_their_authored_line() {
    let fixture = CliProcessFixture::named("shell_failures_authored_line");
    assert!(fixture.initialize_repository());
    let record = |content: &str| {
        let input = fixture.write_file("cwd/doc.md", content);
        let output = fixture.command().args(["compose"]).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert_eq!(output.status.code(), Some(1), "{stdout}");
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let [record] = json["diagnostics"].as_array().unwrap().as_slice() else { panic!("one record: {stdout}") };
        record.clone()
    };

    let parse = record("---\na: 1\nb: 2\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n::shell echo \"open\n");
    assert_eq!(parse["code"], "dm.directive.parse_failure", "{parse:#}");
    assert_eq!((&parse["source"]["line"], &parse["source"]["column"]), (&10.into(), &1.into()), "{parse:#}");
    assert!(parse["message"].as_str().unwrap().contains("line 10"), "{parse:#}");

    let approval = record("---\nv: \"a\\nb\\nc\\nd\"\n---\n\n{{ v }}\n\n::shell darkmatter-r11f2-unapproved\n");
    assert_eq!(approval["code"], "dm.shell.failure", "{approval:#}");
    let message = approval["message"].as_str().unwrap();
    // Line 7 as written; the prepared body, three lines longer, counts 10.
    assert!(message.contains("Approval required") && message.contains("line 7"), "{approval:#}");
    assert!(!message.contains("line 10"), "{approval:#}");

    // A block command that does not tokenize, refused by pre-flight before
    // anything runs: the stage's own code, file line 10 (prepared body line
    // 2), and no tokenizer `line 0`.
    let block = record("---\na: 1\nb: 2\n---\n\n::block when=\"false\"\nhidden\n::end-block\n::shell-block\necho \"open\n::end-block\n");
    assert_eq!(block["code"], "dm.shell.failure", "{block:#}");
    assert_eq!((&block["source"]["line"], &block["source"]["column"]), (&10.into(), &1.into()), "{block:#}");
    let message = block["message"].as_str().unwrap();
    assert!(message.contains("line 10") && !message.contains("line 0") && !message.contains("line 2"), "{block:#}");
}

/// A loopback HTTP server answering every request with `200 OK`, its URL, and
/// the number of requests it has answered.
fn counting_loopback() -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::io::{BufRead, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("address").port();
    let requests = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = std::sync::Arc::clone(&requests);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = std::io::BufReader::new(&stream);
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                line.clear();
            }
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let _ = (&stream).write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    });
    (format!("http://127.0.0.1:{port}/guide.md"), requests)
}

/// The `reference` of every missing-target (`no-match`) record of a JSON run.
fn missing_references(stdout: &str) -> Vec<String> {
    let json: serde_json::Value = serde_json::from_str(stdout).unwrap_or_else(|e| panic!("{e}: {stdout}"));
    json["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|diagnostic| diagnostic["failure"] == "no-match")
        .map(|diagnostic| diagnostic["reference"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// R12-F1.1: `md validate refs --fragments` judges `::file ./side.md#side`
/// as the whole filename composition opens, so the existing `side.md` and
/// its `# Side` heading do not pass it.
#[test]
fn validate_refs_fragments_judges_a_file_inclusion_by_its_whole_filename() {
    let fixture = CliProcessFixture::named("validate_refs_fragments_whole_filename");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    let input = fixture.write_file("cwd/doc.md", "# Doc\n\n::file ./side.md#side\n");
    let output = fixture
        .command()
        .args(["validate", "refs", "--fragments"])
        .arg(&input)
        .args(["--output", "json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert_eq!(missing_references(&stdout), vec!["./side.md#side"], "{stdout}");
}

/// R12-F1.3, .4: under `md validate refs --remote`, a required
/// `::toc-linking` URL is a required failure and a suppressed one is
/// nothing; neither is requested, though the server would answer `200 OK`.
#[test]
fn validate_refs_remote_never_requests_a_toc_target() {
    let fixture = CliProcessFixture::named("validate_refs_remote_toc_target");
    assert!(fixture.initialize_repository());
    let (url, requests) = counting_loopback();
    let input = fixture.write_file(
        "cwd/doc.md",
        &format!("# Doc\n\n::toc-linking \"{url}\"\n\n::toc-linking \"{url}?optional | false\"\n"),
    );
    let output = fixture
        .command()
        .args(["validate", "refs", "--remote", "--timeout", "2"])
        .arg(&input)
        .args(["--output", "json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert_eq!(missing_references(&stdout), vec![url], "{stdout}");
    assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 0, "{stdout}");
}

/// R12-F1.3: `md graph --validate` reports a required `::toc-linking
/// "#here"` as a missing inclusion, though the root has `# Here`.
#[test]
fn graph_validate_reports_a_fragment_shaped_toc_target_as_missing() {
    let fixture = CliProcessFixture::named("graph_validate_fragment_shaped_toc");
    assert!(fixture.initialize_repository());
    let input = fixture.write_file("cwd/doc.md", "# Here\n\n::toc-linking \"#here\"\n");
    let output = fixture.command().args(["graph", "--validate"]).arg(&input).args(["--output", "json"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert_eq!(missing_references(&stdout), vec!["#here"], "{stdout}");
}

/// Runs `md compose`, `md validate refs`, and `md graph --validate` on
/// `input` and asserts each refuses it with one `dm.reference.failure`
/// record whose `source` names `file` at `line`.
fn assert_target_failure_site(fixture: &CliProcessFixture, input: &std::path::Path, file: &str, line: u64) {
    for args in [&["compose"][..], &["validate", "refs"], &["graph", "--validate"]] {
        let output = fixture.command().args(args).arg(input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stdout}");
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let [record] = json["diagnostics"].as_array().unwrap().as_slice() else {
            panic!("{args:?}: one record: {stdout}");
        };
        assert_eq!(record["code"], "dm.reference.failure", "{args:?}: {record:#}");
        let named = record["source"]["file"].as_str().unwrap_or_default();
        assert!(std::path::Path::new(named).ends_with(file), "{args:?}: {record:#}");
        assert_eq!(record["source"]["line"], line, "{args:?}: {record:#}");
    }
}

/// R12-F2.1: an incomplete `@` body target past a header and a false page
/// block is reported at its directive's file line.
#[test]
fn an_invalid_body_target_reports_its_directive_line() {
    let fixture = CliProcessFixture::named("invalid_body_target_site");
    assert!(fixture.initialize_repository());
    let input = fixture.write_file(
        "cwd/doc.md",
        "---\ntitle: x\nother: y\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n\n::file '@'\n",
    );
    assert_target_failure_site(&fixture, &input, "doc.md", 11);
}

/// R12-F2.2: an incomplete `@` `prologue` is reported at its key's line.
#[test]
fn an_invalid_frontmatter_target_reports_its_key_line() {
    let fixture = CliProcessFixture::named("invalid_frontmatter_target_site");
    assert!(fixture.initialize_repository());
    let input = fixture.write_file("cwd/doc.md", "---\ntitle: x\nprologue: \"@\"\n---\n\n# Doc\n");
    assert_target_failure_site(&fixture, &input, "doc.md", 3);
}

/// R12-F2.4: a nested child's incomplete target is reported in the child at
/// its line, never in the input document.
#[test]
fn a_nested_invalid_target_reports_the_child() {
    let fixture = CliProcessFixture::named("nested_invalid_target_site");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/nested.md", "---\na: b\n---\n\n# Nested\n\n::file @\n");
    let input = fixture.write_file("cwd/doc.md", "# Root\n\n::file ./nested.md\n");
    assert_target_failure_site(&fixture, &input, "nested.md", 7);
}

/// A link or image target that is no file reference (`[x](@)`) fails
/// `md validate refs`, `md graph --validate`, and `md compose`'s reference
/// check as one located `dm.reference.failure` record per reference, with
/// reason `invalid file reference` and class `invalid-reference`; the
/// missing link beside them keeps its own code.
#[test]
fn an_invalid_link_target_is_reported_at_its_position() {
    let fixture = CliProcessFixture::named("invalid_link_target_site");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    let input = fixture.write_file("cwd/doc.md", "# Doc\n\n[x](@) [ok](./side.md)\n\n![y](@)\n\n[gone](./gone.md)\n");
    for args in [&["validate", "refs"][..], &["graph", "--validate"], &["compose"]] {
        let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stdout}");
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let records: Vec<_> = json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| {
                let named = record["source"]["file"].as_str().unwrap_or_default();
                assert!(std::path::Path::new(named).ends_with("doc.md"), "{args:?}: {record:#}");
                (
                    record["code"].as_str().unwrap_or_default().to_string(),
                    record["source"]["line"].as_u64(),
                    record["source"]["column"].as_u64(),
                    record["reason"].as_str().unwrap_or_default().to_string(),
                    record["failure"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let invalid = |line, column| {
            ("dm.reference.failure".to_string(), Some(line), Some(column), "invalid file reference".to_string(), "invalid-reference".to_string())
        };
        assert_eq!(
            records,
            vec![
                invalid(3, 1),
                invalid(5, 1),
                ("dm.reference.missing_hyperlink".to_string(), Some(7), Some(1), "file not found".to_string(), "no-match".to_string()),
            ],
            "{args:?}: {stdout}"
        );
    }
}

/// Runs `md compose`, `md validate refs` (with `extra` flags), and `md graph
/// --validate` on `input` and asserts each refuses it with one `code` record
/// at `doc.md` line `line`.
fn assert_admission_refusal(fixture: &CliProcessFixture, input: &std::path::Path, extra: &[&str], code: &str, line: u64) {
    for args in [&["compose"][..], &["validate", "refs"], &["graph", "--validate"]] {
        let extra = if args[0] == "validate" { extra } else { &[][..] };
        let output = fixture.command().args(args).args(extra).arg(input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stdout}");
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let [record] = json["diagnostics"].as_array().unwrap().as_slice() else {
            panic!("{args:?}: one record: {stdout}");
        };
        assert_eq!(record["code"], code, "{args:?}: {record:#}");
        let named = record["source"]["file"].as_str().unwrap_or_default();
        assert!(std::path::Path::new(named).ends_with("doc.md"), "{args:?}: {record:#}");
        assert_eq!(record["source"]["line"], line, "{args:?}: {record:#}");
    }
}

/// R13-F1.1: `::file` of an existing plain-text file is the unsupported
/// file type composition refuses, at its directive, in both validating
/// commands and in the human report; the Markdown file passes.
#[test]
fn validating_commands_refuse_a_text_file_inclusion_at_its_directive() {
    let fixture = CliProcessFixture::named("validate_refs_text_file_inclusion");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    fixture.write_file("cwd/side.txt", "plain\n");
    let input = fixture.write_file("cwd/doc.md", "---\ntitle: x\n---\n\n# Doc\n\n::file \"./side.txt\"\n");
    assert_admission_refusal(&fixture, &input, &[], "dm.transclusion.failure", 7);

    let output = fixture.command().args(["validate", "refs"]).arg(&input).output().unwrap();
    let human = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "{human}");
    assert!(human.contains("unsupported file type") && human.contains("Line: 7"), "{human}");

    let control = fixture.write_file("cwd/doc.md", "---\ntitle: x\n---\n\n# Doc\n\n::file ./side.md\n");
    for args in [&["validate", "refs"][..], &["graph", "--validate"]] {
        let output = fixture.command().args(args).arg(&control).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

/// R13-F1.3: `::url` of a relative path or a scheme-relative address is the
/// URL parse failure composition refuses, at its directive, and `md validate
/// refs --remote` sends no request for it; an absolute URL is requested once.
#[test]
fn validating_commands_refuse_a_relative_url_inclusion_without_a_request() {
    let fixture = CliProcessFixture::named("validate_refs_relative_url_inclusion");
    assert!(fixture.initialize_repository());
    fixture.write_file("cwd/side.md", "# Side\n");
    let (url, requests) = counting_loopback();
    let scheme_relative = url.trim_start_matches("http:").to_string();
    for target in ["./side.md", scheme_relative.as_str()] {
        let input = fixture.write_file("cwd/doc.md", &format!("---\ntitle: x\n---\n\n# Doc\n\n::url {target}\n"));
        assert_admission_refusal(&fixture, &input, &["--remote", "--timeout", "2"], "dm.transclusion.failure", 7);
    }
    assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 0);

    let input = fixture.write_file("cwd/doc.md", &format!("# Doc\n\n::url {url}\n"));
    let output = fixture.command().args(["validate", "refs", "--remote", "--timeout", "2"]).arg(&input).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(requests.load(std::sync::atomic::Ordering::SeqCst), 1);
}

/// R13-F2: a `::toc-linking` target saved as UTF-16 cannot be read. Every
/// command that reads its headings reports the read failure at the
/// directive in the nested file that wrote it, with its line, never at the
/// input document with no line: `md validate refs`, `md graph` with and
/// without `--validate` fail, and `md compose` tolerates it with a warning.
/// A UTF-8 target passes everywhere.
#[test]
fn commands_report_an_unreadable_toc_target_at_its_directive() {
    let fixture = CliProcessFixture::named("validate_refs_unreadable_toc_target");
    assert!(fixture.initialize_repository());
    let utf16 = fixture.write_file("cwd/utf16.md", "");
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend("# Utf16\n".encode_utf16().flat_map(u16::to_le_bytes));
    std::fs::write(&utf16, bytes).unwrap();
    fixture.write_file("cwd/side.md", "# Side\n");
    let nested = "---\ntitle: x\nother: y\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Nested\n\n::toc-linking ./utf16.md\n";
    fixture.write_file("cwd/nested.md", nested);
    let input = fixture.write_file("cwd/guide.md", "# Guide\n\n::file ./nested.md\n");
    for args in [&["compose"][..], &["validate", "refs"], &["graph", "--validate"], &["graph"]] {
        let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected = if args[0] == "compose" { 0 } else { 1 };
        assert_eq!(output.status.code(), Some(expected), "{args:?}: {stdout}");
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let [record] = json["diagnostics"].as_array().unwrap().as_slice() else {
            panic!("{args:?}: one record: {stdout}");
        };
        assert_eq!(record["code"], "dm.input.read_failure", "{args:?}: {record:#}");
        assert!(record["message"].as_str().unwrap_or_default().contains("utf16.md"), "{args:?}: {record:#}");
        let named = record["source"]["file"].as_str().unwrap_or_default();
        assert!(std::path::Path::new(named).ends_with("nested.md"), "{args:?}: {record:#}");
        assert_eq!(record["source"]["line"], 11, "{args:?}: {record:#}");
    }

    let output = fixture.command().args(["validate", "refs"]).arg(&input).output().unwrap();
    let human = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "{human}");
    assert!(human.contains("nested.md") && human.contains("utf16.md"), "{human}");

    fixture.write_file("cwd/nested.md", &nested.replace("./utf16.md", "./side.md"));
    for args in [&["compose"][..], &["validate", "refs"], &["graph", "--validate"], &["graph"]] {
        let output = fixture.command().args(args).arg(&input).args(["--output", "json"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {stdout}");
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(json["diagnostics"].as_array().map_or(0, Vec::len), 0, "{args:?}: {stdout}");
    }
}
