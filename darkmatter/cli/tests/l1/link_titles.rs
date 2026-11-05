//! Link-title warnings and migration through the real `md` binary: the report
//! JSON, `--deny` on `md compose` and `md render`, and `md clean` as a fixed
//! point that keeps code intact.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;

const LEGACY: &str = "dm.link.legacy_structured_title";
const MALFORMED: &str = "dm.link.malformed_structured_title";

fn report_of(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("not JSON ({error}): {}", String::from_utf8_lossy(&output.stdout)))
}

fn diagnostic<'a>(report: &'a Value, code: &str) -> &'a Value {
    report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == code)
        .unwrap_or_else(|| panic!("no {code}: {report:#}"))
}

#[test]
fn report_json_locates_both_warnings_and_succeeds() {
    let fixture = CliProcessFixture::named("link-title-report");
    let path = fixture.write_file(
        "cwd/doc.md",
        "---\nkey: value\n---\n\n[Docs](https://example.com/a \"class='x'\")\n\nThen [Bad](https://example.com/b \"{ class: \").\n",
    );
    let output = fixture.command().args(["compose", "--output", "report-json"]).arg(&path).output().unwrap();
    let report = report_of(&output);
    assert_eq!(report["outcome"], "success", "{report:#}");

    let legacy = diagnostic(&report, LEGACY);
    assert_eq!(legacy["severity"], "warning");
    assert!(legacy["message"].as_str().unwrap().contains("[Docs](https://example.com/a)"), "{legacy:#}");
    assert!(legacy["message"].as_str().unwrap().contains("md clean"), "{legacy:#}");
    assert_eq!((&legacy["source"]["line"], &legacy["source"]["column"]), (&Value::from(5), &Value::from(1)), "{legacy:#}");

    let malformed = diagnostic(&report, MALFORMED);
    assert!(malformed["message"].as_str().unwrap().contains("[Bad](https://example.com/b)"), "{malformed:#}");
    assert_eq!((&malformed["source"]["line"], &malformed["source"]["column"]), (&Value::from(7), &Value::from(6)), "{malformed:#}");
}

#[test]
fn a_failing_composition_reports_the_warning_too() {
    let fixture = CliProcessFixture::named("link-title-failure");
    let path = fixture.write_file("cwd/doc.md", "[Docs](https://example.com/a \"class='x'\")\n\n{{ 1 / 0 }}\n");
    let output = fixture.command().args(["compose", "--output", "report-json"]).arg(&path).output().unwrap();
    assert!(!output.status.success());
    let report = report_of(&output);
    assert_eq!(report["document"], Value::Null);
    diagnostic(&report, LEGACY);
}

#[test]
fn compose_deny_names_each_code() {
    for (code, title) in [(LEGACY, "class='x'"), (MALFORMED, "{ class: ")] {
        let fixture = CliProcessFixture::named("link-title-deny");
        let path = fixture.write_file("cwd/doc.md", &format!("[Docs](https://example.com/a \"{title}\")\n"));
        let allowed = fixture.command().arg("compose").arg(&path).output().unwrap();
        assert!(allowed.status.success(), "{allowed:?}");

        let denied = fixture.command().args(["compose", "--deny", code]).arg(&path).output().unwrap();
        assert!(!denied.status.success(), "{code}: {denied:?}");
        assert!(denied.stdout.is_empty(), "{code}: no document");
        assert!(String::from_utf8_lossy(&denied.stderr).contains("[Docs](https://example.com/a)"), "{code}: {denied:?}");
    }
}

#[test]
fn render_warns_on_stderr_and_deny_blocks_it() {
    let fixture = CliProcessFixture::named("link-title-render");
    let path = fixture.write_file("cwd/doc.md", "[Docs](https://example.com/a \"class='x'\")\n");

    let rendered = fixture.command().args(["render", "--output", "markdown"]).arg(&path).output().unwrap();
    assert!(rendered.status.success(), "{rendered:?}");
    let stderr = String::from_utf8_lossy(&rendered.stderr);
    assert!(stderr.contains("md clean") && stderr.contains(":1:"), "{stderr}");
    assert!(String::from_utf8_lossy(&rendered.stdout).contains("[Docs]"), "{rendered:?}");

    let denied = fixture.command().args(["render", "--output", "markdown", "--deny", LEGACY]).arg(&path).output().unwrap();
    assert!(!denied.status.success(), "{denied:?}");
    assert!(denied.stdout.is_empty(), "{denied:?}");
}

#[test]
fn clean_migrates_once_and_leaves_code_alone() {
    let fixture = CliProcessFixture::named("link-title-clean");
    let source = "[Docs](https://example.com/docs \"class='docs-link' target='_blank' data-section='help'\")\n\n```text\n[a](./a.md \"class='x'\")\nkey=value\n```\n\n`[b](./b.md \"class='y'\")`\n";
    let path = fixture.write_file("cwd/doc.md", source);

    let saved = fixture.command().args(["clean", "--save"]).arg(&path).output().unwrap();
    assert!(saved.status.success(), "{saved:?}");
    let first = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        first,
        source.replace(
            "\"class='docs-link' target='_blank' data-section='help'\"",
            "'{ class: \"docs-link\", target: \"_blank\", \"data-section\": \"help\" }'",
        )
    );

    let again = fixture.command().args(["clean", "--save"]).arg(&path).output().unwrap();
    assert!(again.status.success(), "{again:?}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), first, "a second clean is byte-identical");

    let output = fixture.command().args(["compose", "--output", "report-json"]).arg(&path).output().unwrap();
    let report = report_of(&output);
    assert!(
        report["diagnostics"].as_array().unwrap().iter().all(|diagnostic| diagnostic["code"] != LEGACY),
        "{report:#}"
    );
}
