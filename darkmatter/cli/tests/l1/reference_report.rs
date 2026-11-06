//! The reference report through the real binary: which command shows what, on
//! which stream, and the identical body `md graph --validate` and
//! `md validate refs` share.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository());
    fixture
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn graph_and_validate_refs_print_the_same_report_body() {
    let fixture = fixture("reference_report_same_body");
    fixture.write_file("cwd/docs/child.md", "# Child\n\n[gone](./child-gone.md)\n");
    let input = fixture.write_file(
        "cwd/docs/root.md",
        "# Root\n\n[gone](./gone.md)\n\n::file child.md\n\n![image](./image.png)\n",
    );

    let refs = fixture.command().args(["validate", "refs"]).arg(&input).output().unwrap();
    let graph = fixture.command().arg("graph").arg(&input).arg("--validate").output().unwrap();
    let body = stdout_of(&refs);

    assert_eq!(refs.status.code(), Some(1));
    assert_eq!(graph.status.code(), Some(1));
    assert_eq!(
        body,
        "\
Broken references in root.md

docs/root.md
  file not found in hyperlink  `./gone.md`    root.md:3:1
  file not found in image      `./image.png`  root.md:7:1

docs/child.md (transcluded at root.md:5)
  file not found in hyperlink  `./child-gone.md`  child.md:3:1

Checked 4 references across 2 documents: 1 ok, 3 broken
"
    );
    let graph_out = stdout_of(&graph);
    assert!(
        graph_out.ends_with(&format!("\n\n{body}")),
        "graph must append the same body after its tree:\n{graph_out}"
    );
}

#[test]
fn a_clean_document_prints_the_same_all_ok_summary_on_both() {
    let fixture = fixture("reference_report_all_ok");
    let input = fixture.write_file("cwd/doc.md", "[ok](https://example.com)\n");

    fixture
        .command()
        .args(["validate", "refs"])
        .arg(&input)
        .assert()
        .success()
        .stdout("Checked 1 reference across 1 document: all ok\n");
    let graph = fixture.command().arg("graph").arg(&input).arg("--validate").output().unwrap();
    assert!(graph.status.success());
    let text = stdout_of(&graph);
    assert!(text.ends_with("\n\nChecked 1 reference across 1 document: all ok\n"), "{text}");
    assert!(!text.contains("0 issues"), "{text}");
}

#[test]
fn validate_refs_shows_every_severity_and_graph_shows_errors() {
    let fixture = fixture("reference_report_visibility");
    let input = fixture.write_file("cwd/doc.md", "[mail](mailto:a@example.com) [gone](./gone.md)\n");

    let refs = stdout_of(&fixture.command().args(["validate", "refs"]).arg(&input).output().unwrap());
    assert!(refs.contains("unsupported scheme"), "{refs}");
    assert!(refs.ends_with("1 ok, 1 broken\n  1 info\n"), "{refs}");

    let refs_only_info = fixture.write_file("cwd/info.md", "[mail](mailto:a@example.com)\n");
    fixture
        .command()
        .args(["validate", "refs"])
        .arg(&refs_only_info)
        .assert()
        .success()
        .stdout(predicate::str::contains("unsupported scheme"))
        .stdout(predicate::str::contains("Broken references").not())
        .stdout(predicate::str::ends_with("all ok\n  1 info\n"));

    let graph = stdout_of(&fixture.command().arg("graph").arg(&input).arg("--validate").output().unwrap());
    assert!(!graph.contains("unsupported scheme"), "{graph}");
}

#[test]
fn show_all_adds_the_listing_to_the_text_and_the_data() {
    let fixture = fixture("reference_report_show_all");
    let input = fixture.write_file("cwd/doc.md", "[ok](https://example.com) [gone](./gone.md)\n");

    let without = stdout_of(&fixture.command().args(["validate", "refs"]).arg(&input).output().unwrap());
    assert!(!without.contains("All references"), "{without}");

    fixture
        .command()
        .args(["validate", "refs", "--show-all"])
        .arg(&input)
        .assert()
        .failure()
        .stdout(predicate::str::contains("All references"))
        .stdout(predicate::str::contains("`https://example.com`"))
        .stdout(predicate::str::contains("Remote URL not verified: https://example.com"));

    let data = |args: &[&str]| -> serde_json::Value {
        let output = fixture.command().args(["validate", "refs"]).args(args).arg(&input).output().unwrap();
        serde_json::from_slice(&output.stdout).unwrap()
    };
    assert!(data(&["--output", "json"]).get("references").is_none());
    assert_eq!(data(&["--output", "json", "--show-all"])["references"].as_array().unwrap().len(), 2);
}

#[test]
fn the_graph_notation_mode_prints_only_its_diagram() {
    let fixture = fixture("reference_report_graph_notation");
    let input = fixture.write_file("cwd/doc.md", "[gone](./gone.md)\n");

    fixture
        .command()
        .args(["validate", "refs", "--graph", "mermaid"])
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("flowchart TD"))
        .stdout(predicate::str::contains("Checked").not());
}

#[test]
fn verbose_prints_each_destination_beside_its_text() {
    let fixture = fixture("reference_report_verbose");
    let input = fixture.write_file("cwd/doc.md", "# Head\n\n[gone](./gone.md)\n");

    let output = fixture.command().args(["-v", "validate", "refs"]).arg(&input).output().unwrap();
    let text = stdout_of(&output);
    let row = text.lines().find(|line| line.contains("doc.md:3:1")).unwrap_or_else(|| panic!("{text}"));
    assert!(row.contains("doc.md:3:1 <file:///"), "{text}");
    assert!(row.ends_with("/doc.md>"), "a bare file URI, no fragment: {text}");
}

#[test]
fn data_reports_authored_positions_and_the_trail() {
    let fixture = fixture("reference_report_data");
    fixture.write_file("cwd/child.md", "---\ntitle: c\n---\n\n[gone](./gone.md)\n");
    let input = fixture.write_file("cwd/root.md", "---\ntitle: r\n---\n\n# Root\n\n::file child.md\n");

    let output = fixture
        .command()
        .args(["validate", "refs", "--output", "json"])
        .arg(&input)
        .output()
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let issue = &data["issues"][0];
    assert_eq!(issue["line"], 5, "the file's line, not the prepared body's: {issue}");
    assert_eq!(issue["column"], 1);
    assert!(issue["section"].is_null(), "{issue}");
    assert_eq!(issue["inserted"], false);
    assert_eq!(issue["included_via"][0]["line"], 7, "{issue}");
    assert_eq!(std::path::Path::new(issue["included_via"][0]["file"].as_str().unwrap()), input);
}

#[test]
fn a_flow_list_reports_each_value_in_data_and_text() {
    let fixture = fixture("reference_report_flow_list");
    fixture.write_file("cwd/child.md", "[gone](./child-gone.md)\n");
    let input = fixture.write_file("cwd/root.md", "---\nprologue: [\n  ./missing.md,\n  ./child.md\n]\n---\n# Root\n");

    let json = fixture.command().args(["validate", "refs", "--output", "json"]).arg(&input).output().unwrap();
    let data: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let issue = |target: &str| {
        data["issues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|issue| issue["raw"] == target || issue["reference"] == target)
            .unwrap_or_else(|| panic!("no issue of {target}: {data}"))
            .clone()
    };
    let missing = issue("./missing.md");
    assert_eq!((&missing["line"], &missing["column"]), (&serde_json::json!(3), &serde_json::json!(3)), "{missing}");
    assert_eq!(issue("./child-gone.md")["included_via"][0]["line"], 4);

    let text = fixture.command().args(["validate", "refs"]).arg(&input).output().unwrap();
    let body = stdout_of(&text);
    assert!(body.contains("root.md:3:3"), "{body}");
    assert!(body.contains("(transcluded at root.md:4)"), "{body}");

    let composed = fixture.command().args(["compose", "--output", "json"]).arg(&input).output().unwrap();
    let envelope: serde_json::Value = serde_json::from_slice(&composed.stdout).unwrap();
    let located = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["reference_details"]["reference"] == "./missing.md" || diagnostic["reference"] == "./missing.md")
        .unwrap_or_else(|| panic!("no diagnostic of the target: {envelope}"));
    assert_eq!((&located["source"]["line"], &located["source"]["column"]), (&serde_json::json!(3), &serde_json::json!(3)), "{located}");
}

#[test]
fn compose_reports_broken_references_on_stderr_without_the_old_wording() {
    let fixture = fixture("reference_report_compose");
    let input = fixture.write_file("cwd/doc.md", "# Head\n\n[gone](./gone*.md)\n");

    fixture
        .command()
        .arg("compose")
        .arg(&input)
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("Broken references in doc.md"))
        .stderr(predicate::str::contains("file not found in hyperlink"))
        .stderr(predicate::str::contains("doc.md:3:1"))
        .stderr(predicate::str::contains("are literal in a file reference"))
        .stderr(predicate::str::contains("Checked 1 reference across 1 document: 0 ok, 1 broken"))
        .stderr(predicate::str::contains("failure:").not())
        .stderr(predicate::str::contains("is not valid").not())
        .stderr(predicate::str::contains("Invalid Hyperlink").not());
}

#[test]
fn compose_warning_row_names_the_inserting_construct() {
    let fixture = fixture("reference_report_inserted");
    let input = fixture.write_file(
        "cwd/doc.md",
        "---\nlink: \"[l](./nope.md)\"\n---\nsee {{ link }}\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&input)
        .assert()
        .success()
        .stderr(predicate::str::contains("Checked").not())
        .stderr(predicate::str::contains("(line 4, column 5, inside inserted text)"));
}

#[test]
fn multiline_html_and_css_targets_report_invalid_and_exit_one() {
    let fixture = fixture("reference_report_multiline_html");
    let input = fixture.write_file(
        "cwd/doc.md",
        "<img\n src=\"./missing.png\">\n\n<style>\n@import \"./missing.css\";\n@font-face {\n  src: url(\"./missing.woff\");\n}\n</style>\n\n<video\n src=\"./missing.mp4\"></video>\n",
    );

    let refs = fixture.command().args(["validate", "refs"]).arg(&input).args(["--output", "json"]).output().unwrap();
    let compose = fixture.command().arg("compose").arg(&input).output().unwrap();
    let graph = fixture.command().arg("graph").arg(&input).arg("--validate").output().unwrap();

    assert_eq!(refs.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&refs.stdout).unwrap();
    let found: Vec<_> = report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| {
            (
                issue["reference"].as_str().unwrap().to_string(),
                issue["line"].as_u64().unwrap(),
                issue["column"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("./missing.png".to_string(), 1, 1),
            ("./missing.css".to_string(), 5, 1),
            ("./missing.woff".to_string(), 7, 8),
            ("./missing.mp4".to_string(), 11, 1),
        ]
    );

    assert_eq!(compose.status.code(), Some(1), "{}", String::from_utf8_lossy(&compose.stderr));
    assert_eq!(graph.status.code(), Some(1));
    let graph_out = stdout_of(&graph);
    assert!(graph_out.contains("./missing.mp4"), "{graph_out}");
    assert!(graph_out.contains("./missing.css"), "{graph_out}");
}

const SCRIPT_TEMPLATE_DOCUMENT: &str = "Note: <script>const label = '<a href=\"./draft.md\">Draft</a>';</script>\n";

#[test]
fn validate_refs_accepts_a_script_template_after_prose() {
    let fixture = fixture("reference_report_script_template_refs");
    let input = fixture.write_file("cwd/doc.md", SCRIPT_TEMPLATE_DOCUMENT);

    let output = fixture.command().args(["validate", "refs"]).arg(&input).args(["--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["issues"].as_array().unwrap().len(), 0, "{report:#}");
}

#[test]
fn graph_validate_accepts_a_script_template_after_prose() {
    let fixture = fixture("reference_report_script_template_graph");
    let input = fixture.write_file("cwd/doc.md", SCRIPT_TEMPLATE_DOCUMENT);

    let output = fixture.command().arg("graph").arg(&input).args(["--validate", "--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn compose_accepts_a_script_template_after_prose() {
    let fixture = fixture("reference_report_script_template_compose");
    let input = fixture.write_file("cwd/doc.md", SCRIPT_TEMPLATE_DOCUMENT);

    let output = fixture.command().arg("compose").arg(&input).args(["--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

const STYLE_COMMENT_DOCUMENT: &str = "Note: <style>/* <img src=\"./gone.png\"> */</style>\n";

#[test]
fn graph_validate_accepts_a_style_comment_after_prose() {
    let fixture = fixture("reference_report_style_comment_graph");
    let input = fixture.write_file("cwd/doc.md", STYLE_COMMENT_DOCUMENT);

    let output = fixture.command().arg("graph").arg(&input).args(["--validate", "--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn compose_accepts_a_style_comment_after_prose() {
    let fixture = fixture("reference_report_style_comment_compose");
    let input = fixture.write_file("cwd/doc.md", STYLE_COMMENT_DOCUMENT);

    let output = fixture.command().arg("compose").arg(&input).args(["--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
}

#[test]
fn a_missing_link_after_a_script_template_still_fails() {
    let fixture = fixture("reference_report_script_template_control");
    let input = fixture.write_file("cwd/doc.md", &format!("{SCRIPT_TEMPLATE_DOCUMENT}\n[x](./gone.md)\n"));

    let output = fixture.command().args(["validate", "refs"]).arg(&input).args(["--output", "json"]).output().unwrap();

    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let references: Vec<_> = report["issues"].as_array().unwrap().iter().map(|issue| issue["reference"].as_str().unwrap()).collect();
    assert_eq!(references, ["./gone.md"]);
}
