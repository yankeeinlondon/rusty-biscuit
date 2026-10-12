//! `md clean` and `md FILE --save` per transaction outcome: a declined body
//! emits the original and saves nothing, invalid YAML fails without saving,
//! and source bytes no rule touches (terminators, BOM, empty frontmatter,
//! final newline) come through unchanged on every input route.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use std::fs;

/// `::block` with no `::end-block`: a malformed directive region, which the
/// body cleanup refuses even though list padding would otherwise apply.
const UNPAIRED: &str = "Intro:\n- a\n\n::block\n\ntext\n";

/// A deterministic `yaml.reserved-indicator` repair `md clean` normally
/// applies, in front of the unpaired directive.
const REPAIRABLE_AND_UNPAIRED: &str = "---\ntitle: @daily-report\n---\nIntro:\n- a\n\n::block\n\ntext\n";

fn json_report(stdout: &[u8]) -> Value {
    serde_json::from_slice(stdout)
        .unwrap_or_else(|error| panic!("not a JSON envelope ({error}): {}", String::from_utf8_lossy(stdout)))
}

/// The diagnostic with `code`, which must exist.
fn finding<'a>(report: &'a Value, code: &str) -> &'a Value {
    report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["code"] == code)
        .unwrap_or_else(|| panic!("no {code} finding: {report:#}"))
}

#[test]
fn declined_clean_prints_the_original_document() {
    let fixture = CliProcessFixture::named("clean-fidelity-declined-clean-prints-the-original-document");
    let path = fixture.write_file("cwd/doc.md", UNPAIRED);

    let assert = fixture.command().arg("clean").arg(&path).assert().success();
    let output = assert.get_output();

    assert_eq!(output.stdout, UNPAIRED.as_bytes());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("markdown cleanup"), "{stderr}");
    assert!(stderr.contains("clean.fidelity-fallback"), "{stderr}");
    assert!(stderr.contains("malformed directive region"), "{stderr}");
}

#[test]
fn declined_clean_saves_nothing() {
    let fixture = CliProcessFixture::named("clean-fidelity-declined-clean-saves-nothing");
    let path = fixture.write_file("cwd/doc.md", UNPAIRED);

    let assert = fixture
        .command()
        .arg("clean")
        .arg(&path)
        .arg("--save")
        .assert()
        .success();

    assert_eq!(fs::read(&path).unwrap(), UNPAIRED.as_bytes());
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("clean.fidelity-fallback"), "{stderr}");
}

#[test]
fn declined_clean_json_reports_the_fallback() {
    let fixture = CliProcessFixture::named("clean-fidelity-declined-clean-json-reports-the-fallback");
    let path = fixture.write_file("cwd/doc.md", UNPAIRED);

    for extra in [&[][..], &["--save"][..]] {
        let assert = fixture
            .command()
            .arg("clean")
            .arg(&path)
            .arg("--json")
            .args(extra)
            .assert()
            .success();
        let output = assert.get_output();
        assert!(output.stderr.is_empty(), "JSON mode keeps stderr empty");

        let report = json_report(&output.stdout);
        assert_eq!(report["changed"], json!(false));
        assert_eq!(report["applied"], json!([]));
        let fallback = finding(&report, "clean.fidelity-fallback");
        assert_eq!(fallback["classification"], json!("non_deterministic_find"));
        assert_eq!(fallback["repairs"], json!([]));
        assert_eq!(fallback["span"]["start"], json!(0));
        assert_eq!(fallback["span"]["end"], json!(UNPAIRED.len()));
        assert_eq!(fs::read(&path).unwrap(), UNPAIRED.as_bytes(), "extra: {extra:?}");
    }
}

/// A body refusal discards the accepted candidate frontmatter repair too.
#[test]
fn declined_clean_discards_frontmatter_repairs() {
    let fixture =
        CliProcessFixture::named("clean-fidelity-declined-clean-discards-frontmatter-repairs");
    let path = fixture.write_file("cwd/doc.md", REPAIRABLE_AND_UNPAIRED);

    let json = fixture
        .command()
        .args(["clean", "--json"])
        .arg(&path)
        .assert()
        .success();
    let report = json_report(&json.get_output().stdout);
    assert_eq!(report["applied"], json!([]));
    assert_eq!(report["changed"], json!(false));
    assert_eq!(
        finding(&report, "yaml.reserved-indicator")["repairs"]
            .as_array()
            .map(Vec::len),
        Some(1),
        "the repair was found, then discarded"
    );
    finding(&report, "clean.fidelity-fallback");

    let printed = fixture.command().arg("clean").arg(&path).assert().success();
    assert_eq!(printed.get_output().stdout, REPAIRABLE_AND_UNPAIRED.as_bytes());

    fixture.command().arg("clean").arg(&path).arg("--save").assert().success();
    assert_eq!(fs::read(&path).unwrap(), REPAIRABLE_AND_UNPAIRED.as_bytes());
}

#[test]
fn top_level_save_writes_a_changed_document() {
    let fixture = CliProcessFixture::named("clean-fidelity-top-level-save-writes-a-changed-document");
    let path = fixture.write_file("cwd/doc.md", "# Hello\n\nIntro:\n- a\n");

    let assert = fixture.command().arg(&path).arg("--save").assert().success();

    assert_eq!(fs::read_to_string(&path).unwrap(), "# Hello\n\nIntro:\n\n- a\n");
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("Whitespace changes only"), "{stdout}");
}

#[test]
fn top_level_save_leaves_a_declined_document_untouched() {
    let fixture =
        CliProcessFixture::named("clean-fidelity-top-level-save-leaves-a-declined-document-untouched");
    let path = fixture.write_file("cwd/doc.md", REPAIRABLE_AND_UNPAIRED);

    let assert = fixture.command().arg(&path).arg("--save").assert().success();

    assert_eq!(fs::read(&path).unwrap(), REPAIRABLE_AND_UNPAIRED.as_bytes());
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("clean.fidelity-fallback"), "{stderr}");
}

/// The library's test-only broken pass corrupts the formatted candidate
/// before the guard. It is process-global and never reaches a spawned `md`,
/// so this drives `run_clean` in-process.
#[test]
fn broken_pass_refusal_is_never_saved() {
    use darkmatter::markdown::cleanup::test_hooks::install_broken_pass;
    use darkmatter::markdown::compose::RequestSnapshot;
    use darkmatter_cli::commands::{CleanOptions, run_clean};
    use darkmatter_cli::request::MdRequest;

    fn retarget_link(candidate: &str) -> String {
        candidate.replace("https://example.com", "https://example.org")
    }

    let fixture = CliProcessFixture::named("clean-fidelity-broken-pass-refusal-is-never-saved");
    let source = "---\ntitle: @daily-report\n---\nIntro:\n- [a](https://example.com)\n";
    let path = fixture.write_file("cwd/doc.md", source);
    let request = MdRequest::new(
        RequestSnapshot::new(fixture.cwd()).with_home(Some(fixture.home().to_path_buf())),
    );
    let options = CleanOptions {
        save: true,
        ..CleanOptions::default()
    };

    let _guard = install_broken_pass(retarget_link);
    run_clean(Some(&path), &options, &request).expect("a refusal is not an error");

    assert_eq!(fs::read(&path).unwrap(), source.as_bytes());
}

#[test]
fn unrepairable_frontmatter_fails_without_saving() {
    let fixture =
        CliProcessFixture::named("clean-fidelity-unrepairable-frontmatter-fails-without-saving");
    let source = "---\ntitle: [unclosed\n---\n\n# Body\n";
    let path = fixture.write_file("cwd/doc.md", source);

    let human = fixture.command().arg("clean").arg(&path).arg("--save").assert().failure();
    assert_eq!(human.get_output().status.code(), Some(1));
    assert!(human.get_output().stdout.is_empty());
    let stderr = String::from_utf8_lossy(&human.get_output().stderr);
    assert!(stderr.contains("frontmatter parse failed"), "{stderr}");

    let json = fixture
        .command()
        .arg("clean")
        .arg(&path)
        .args(["--save", "--json"])
        .assert()
        .failure();
    let output = json.get_output();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "JSON mode keeps stderr empty");
    let report = json_report(&output.stdout);
    assert_eq!(report["applied"], json!([]));
    assert_eq!(report["changed"], json!(false));
    finding(&report, "yaml.parse");

    assert_eq!(fs::read(&path).unwrap(), source.as_bytes());
}

/// `frontmatter: false` is read from the repaired candidate, so the repair
/// is never emitted and the authored YAML stays invalid.
#[test]
fn disabled_frontmatter_repair_leaves_invalid_yaml_invalid() {
    let fixture = CliProcessFixture::named(
        "clean-fidelity-disabled-frontmatter-repair-leaves-invalid-yaml-invalid",
    );
    let source =
        "---\nstyle:\n  markdown:\n    frontmatter: false\ntitle: @daily-report\n---\nIntro:\n- a\n";
    let path = fixture.write_file("cwd/doc.md", source);

    let human = fixture.command().arg("clean").arg(&path).arg("--save").assert().failure();
    assert_eq!(human.get_output().status.code(), Some(1));

    let json = fixture
        .command()
        .args(["clean", "--json"])
        .arg(&path)
        .assert()
        .failure();
    assert_eq!(json.get_output().status.code(), Some(1));
    assert!(json.get_output().stderr.is_empty());
    let report = json_report(&json.get_output().stdout);
    assert_eq!(report["applied"], json!([]));
    assert_eq!(report["changed"], json!(false));

    assert_eq!(fs::read(&path).unwrap(), source.as_bytes());
}

#[test]
fn disabled_frontmatter_repair_keeps_the_block_and_formats_the_body() {
    let fixture = CliProcessFixture::named(
        "clean-fidelity-disabled-frontmatter-repair-keeps-the-block-and-formats-the-body",
    );
    let frontmatter = "---\nstyle:\n  markdown:\n    frontmatter: false\ntitle: Fine   \n---\n";
    let path = fixture.write_file("cwd/doc.md", &format!("{frontmatter}Intro:\n- a\n"));

    fixture.command().arg("clean").arg(&path).arg("--save").assert().success();

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        format!("{frontmatter}Intro:\n\n- a\n")
    );
}

/// Cleans `source` from stdin (stdout) and from a file (`--save`), asserting
/// both produce exactly `expected`.
fn assert_cleans_to(fixture: &CliProcessFixture, name: &str, source: &str, expected: &str) {
    let stdin = fixture
        .command()
        .args(["clean", "-"])
        .write_stdin(source)
        .assert()
        .success();
    assert_eq!(
        String::from_utf8_lossy(&stdin.get_output().stdout),
        expected,
        "{name} via stdin"
    );

    let path = fixture.write_file(format!("cwd/{name}.md"), source);
    fixture.command().arg("clean").arg(&path).arg("--save").assert().success();
    assert_eq!(fs::read_to_string(&path).unwrap(), expected, "{name} via file");
}

#[test]
fn crlf_terminators_are_kept() {
    let fixture = CliProcessFixture::named("clean-fidelity-crlf-terminators-are-kept");
    assert_cleans_to(
        &fixture,
        "crlf",
        "# T\r\n\r\nIntro:\r\n- a\r\n",
        "# T\r\n\r\nIntro:\r\n\r\n- a\r\n",
    );
}

#[test]
fn byte_order_mark_without_frontmatter_is_kept() {
    let fixture =
        CliProcessFixture::named("clean-fidelity-byte-order-mark-without-frontmatter-is-kept");
    assert_cleans_to(
        &fixture,
        "bom",
        "\u{FEFF}# T\n\nIntro:\n- a\n",
        "\u{FEFF}# T\n\nIntro:\n\n- a\n",
    );
}

#[test]
fn empty_frontmatter_block_is_kept() {
    let fixture = CliProcessFixture::named("clean-fidelity-empty-frontmatter-block-is-kept");
    assert_cleans_to(&fixture, "empty", "---\n---\nIntro:\n- a\n", "---\n---\nIntro:\n\n- a\n");
}

#[test]
fn missing_final_newline_is_not_added() {
    let fixture = CliProcessFixture::named("clean-fidelity-missing-final-newline-is-not-added");
    assert_cleans_to(&fixture, "unterminated", "Intro:\n- a", "Intro:\n\n- a");
}

#[test]
fn every_body_rule_off_returns_the_document_unchanged() {
    let fixture =
        CliProcessFixture::named("clean-fidelity-every-body-rule-off-returns-the-document-unchanged");
    let body = "Intro:\n- a\n  - b\n\n1. x\n3. y\n\n| a | bb |\n|-|-|\n| ccc | d |\n\n```\ncode\n```\none\ntwo\n";
    let off = concat!(
        "---\nstyle:\n  markdown:\n",
        "    table-layout: false\n",
        "    fixed-width: false\n",
        "    unwrap: false\n",
        "    indentation: preserve\n",
        "    ordered-lists: false\n",
        "    list-padding: false\n",
        "    list-item-padding: preserve\n",
        "    list-interior-padding: preserve\n",
        "    default-language: \"\"\n",
        "---\n",
    );

    let defaults = fixture
        .command()
        .args(["clean", "-"])
        .write_stdin(body)
        .assert()
        .success();
    assert_ne!(
        String::from_utf8_lossy(&defaults.get_output().stdout),
        body,
        "the defaults have work to do on this body"
    );

    let document = format!("{off}{body}");
    assert_cleans_to(&fixture, "all-off", &document, &document);
}

#[test]
fn stdin_routes_clean_and_report() {
    let fixture = CliProcessFixture::named("clean-fidelity-stdin-routes-clean-and-report");
    let source = "---\ntitle: @daily-report\n---\nIntro:\n- a\n";

    let dash = fixture
        .command()
        .args(["clean", "-"])
        .write_stdin(source)
        .assert()
        .success();
    assert_eq!(
        String::from_utf8_lossy(&dash.get_output().stdout),
        "---\ntitle: \"@daily-report\"\n---\nIntro:\n\n- a\n"
    );

    let json = fixture
        .command()
        .args(["clean", "--json"])
        .write_stdin(source)
        .assert()
        .success();
    assert!(json.get_output().stderr.is_empty());
    let report = json_report(&json.get_output().stdout);
    assert_eq!(report["source"], json!({ "kind": "stdin", "path": null }));
    assert_eq!(report["changed"], json!(true));
    assert_eq!(report["applied"].as_array().map(Vec::len), Some(1));
}
