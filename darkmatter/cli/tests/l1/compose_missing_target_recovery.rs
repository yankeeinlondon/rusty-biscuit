//! `md compose`'s missing-target allowances (`missing-hyperlinks`,
//! `missing-image-refs`, `missing-transclusions`) cover a well-formed target
//! that names no file, and nothing else. An invalid file reference, an
//! unreadable candidate, or a malformed URL refuses the run under its kind's
//! allowance, in the direct route and the lifecycle route alike: no document,
//! status 1, the located issue with its failure class, and no `composed`
//! event. The controls pin what the allowances still recover.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use std::path::Path;

/// The two ways a run reaches the shared reference check, and the
/// frontmatter each adds.
const LIFECYCLE: &str = "composed:\n  - stdout: done\n";

/// Writes `doc.md` as `frontmatter` (plus the `composed` event for a
/// lifecycle run) over `body`.
fn document(fixture: &CliProcessFixture, frontmatter: &str, lifecycle: bool, body: &str) -> std::path::PathBuf {
    let event = if lifecycle { LIFECYCLE } else { "" };
    fixture.write_file("cwd/doc.md", &format!("---\ntitle: doc\n{frontmatter}{event}---\n{body}"))
}

/// `md compose --output json` under each `--allow` in `allow`.
fn report(fixture: &CliProcessFixture, document: &Path, allow: &[&str]) -> (Option<i32>, Value) {
    let mut command = fixture.command();
    command.args(["compose", "--output", "json"]);
    for allowance in allow {
        command.args(["--allow", allowance]);
    }
    let output = command.arg(document).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "the report holds everything: {stderr}");
    (output.status.code(), serde_json::from_slice(&output.stdout).expect("one JSON report"))
}

/// The Markdown run under the same allowances: its status and stdout.
fn markdown_run(fixture: &CliProcessFixture, document: &Path, allow: &[&str]) -> (Option<i32>, String) {
    let mut command = fixture.command();
    command.arg("compose");
    for allowance in allow {
        command.args(["--allow", allowance]);
    }
    let output = command.arg(document).output().unwrap();
    (output.status.code(), String::from_utf8_lossy(&output.stdout).into_owned())
}

fn codes(report: &Value) -> Vec<&str> {
    report["diagnostics"].as_array().unwrap().iter().map(|diagnostic| diagnostic["code"].as_str().unwrap()).collect()
}

/// The frontmatter lines ahead of the body: `---`, `title`, the case's own
/// lines, the event's, and the closing `---`.
fn body_offset(frontmatter: &str, lifecycle: bool) -> u64 {
    let event = if lifecycle { LIFECYCLE.lines().count() } else { 0 };
    (3 + frontmatter.lines().count() + event) as u64
}

/// Asserts `body` under `frontmatter` refuses the run in either route, with
/// and without `allow`, reporting one `dm.reference.failure` of class
/// `failure` at `line` (`None`: the first body line).
fn assert_refused(name: &str, frontmatter: &str, body: &str, allow: &[&str], failure: Value, line: Option<u64>) {
    for lifecycle in [false, true] {
        let route = if lifecycle { "lifecycle" } else { "direct" };
        let fixture = CliProcessFixture::named(name);
        let document = document(&fixture, frontmatter, lifecycle, body);
        let line = line.unwrap_or_else(|| body_offset(frontmatter, lifecycle) + 1);
        for allowance in [&[][..], allow, &["all"]] {
            let (status, report) = report(&fixture, &document, allowance);
            assert_eq!(status, Some(1), "{route} {allowance:?}: {report:#}");
            assert_eq!(report["outcome"], "failure", "{route} {allowance:?}: {report:#}");
            assert_eq!(report["document"], Value::Null, "{route} {allowance:?}: no document");
            assert_eq!(report["lifecycle_output"], json!([]), "{route} {allowance:?}: `composed` ran: {report:#}");
            assert_eq!(codes(&report), ["dm.reference.failure"], "{route} {allowance:?}: {report:#}");
            let diagnostic = &report["diagnostics"][0];
            assert_eq!(diagnostic["severity"], "error", "{route} {allowance:?}");
            assert_eq!(diagnostic["failure"], failure, "{route} {allowance:?}: {diagnostic:#}");
            assert_eq!(diagnostic["source"]["line"], line, "{route} {allowance:?}: {diagnostic:#}");
            assert!(diagnostic["source"]["file"].as_str().unwrap().ends_with("doc.md"), "{diagnostic:#}");

            let (status, stdout) = markdown_run(&fixture, &document, allowance);
            assert_eq!(status, Some(1), "{route} {allowance:?}: Markdown run");
            assert!(stdout.is_empty(), "{route} {allowance:?}: no document: {stdout}");
        }
    }
}

/// Asserts `body` composes under `allow` in either route, reporting `code`,
/// and that the lifecycle run dispatched `composed`.
fn assert_recovered(name: &str, frontmatter: &str, body: &str, allow: &[&str], code: &str, kept: &str) {
    for lifecycle in [false, true] {
        let route = if lifecycle { "lifecycle" } else { "direct" };
        let fixture = CliProcessFixture::named(name);
        let document = document(&fixture, frontmatter, lifecycle, body);
        let (status, report) = report(&fixture, &document, allow);
        assert_eq!(status, Some(0), "{route}: {report:#}");
        assert!(report["document"].as_str().is_some_and(|text| text.contains(kept)), "{route}: {report:#}");
        assert!(codes(&report).contains(&code), "{route}: {report:#}");
        if lifecycle {
            assert_eq!(report["lifecycle_output"][0]["text"].as_str().map(str::trim), Some("done"), "{report:#}");
        }
    }
}

macro_rules! refused {
    ($($name:ident: $frontmatter:expr, $body:expr, $allow:expr, $failure:expr, $line:expr;)+) => {$(
        #[test]
        fn $name() {
            assert_refused(stringify!($name), $frontmatter, $body, $allow, $failure, $line);
        }
    )+};
}

refused! {
    an_invalid_link_target_refuses_under_its_allowance:
        "", "[Guide](@)\n", &["missing-hyperlinks"], json!("invalid-reference"), None;
    an_angle_bracketed_invalid_link_target_refuses_under_its_allowance:
        "", "[Guide](<@>)\n", &["missing-hyperlinks"], json!("invalid-reference"), None;
    an_invalid_image_target_refuses_under_its_allowance:
        "", "![Guide](@)\n", &["missing-image-refs"], json!("invalid-reference"), None;
    an_invalid_html_anchor_target_refuses_under_its_allowance:
        "", "<a href=\"@\">Guide</a>\n", &["missing-hyperlinks"], json!("invalid-reference"), None;
    an_invalid_prologue_link_refuses_under_its_allowance:
        "prologue: \"[Guide](@)\"\n", "Body\n", &["missing-hyperlinks"], json!("invalid-reference"), Some(3);
    an_invalid_epilogue_image_refuses_under_its_allowance:
        "epilogue: \"![Guide](@)\"\n", "Body\n", &["missing-image-refs"], json!("invalid-reference"), Some(3);
    a_malformed_link_url_refuses_under_its_allowance:
        "", "[Guide](http://[bad)\n", &["missing-hyperlinks"], Value::Null, None;
    a_malformed_inclusion_target_refuses_under_its_allowance:
        "", "::file @\n", &["missing-transclusions"], json!("invalid-reference"), None;
    a_malformed_code_target_refuses_under_its_allowance:
        "", "::code @\n", &["missing-transclusions"], json!("invalid-reference"), None;
    a_malformed_url_target_refuses_under_its_allowance:
        "", "::url @\n", &["missing-transclusions"], json!("invalid-reference"), None;
}

/// A link or image whose candidate sits under a directory the process cannot
/// traverse is an I/O failure, which no missing-target allowance covers.
#[cfg(unix)]
fn assert_unreadable_refused(name: &str, body: &str, allow: &[&str]) {
    use std::os::unix::fs::PermissionsExt;
    let probe = CliProcessFixture::named(name);
    let locked = probe.cwd().join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::write(locked.join("guide.md"), "# Guide\n").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    // A process that reads through it anyway (root) cannot produce the case.
    let enforced = std::fs::metadata(locked.join("guide.md")).is_err();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if !enforced {
        eprintln!("skipped: directory permissions are not enforced for this process");
        return;
    }
    for lifecycle in [false, true] {
        let route = if lifecycle { "lifecycle" } else { "direct" };
        let fixture = CliProcessFixture::named(name);
        let locked = fixture.cwd().join("locked");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::write(locked.join("guide.md"), "# Guide\n").unwrap();
        let document = document(&fixture, "", lifecycle, body);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let (status, report) = report(&fixture, &document, allow);
        let (markdown_status, stdout) = markdown_run(&fixture, &document, allow);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(status, Some(1), "{route}: {report:#}");
        assert_eq!(report["document"], Value::Null, "{route}: {report:#}");
        assert_eq!(report["lifecycle_output"], json!([]), "{route}: {report:#}");
        assert_eq!(codes(&report), ["dm.reference.failure"], "{route}: {report:#}");
        assert_eq!(report["diagnostics"][0]["failure"], "io", "{route}: {report:#}");
        assert_eq!(report["diagnostics"][0]["source"]["line"], body_offset("", lifecycle) + 1, "{route}: {report:#}");
        assert_eq!(markdown_status, Some(1), "{route}");
        assert!(stdout.is_empty(), "{route}: {stdout}");
    }
}

#[cfg(unix)]
#[test]
fn an_unreadable_link_target_refuses_under_its_allowance() {
    assert_unreadable_refused("recovery_unreadable_link", "[Guide](./locked/guide.md)\n", &["missing-hyperlinks"]);
}

#[cfg(unix)]
#[test]
fn an_unreadable_image_target_refuses_under_its_allowance() {
    assert_unreadable_refused("recovery_unreadable_image", "![Guide](./locked/guide.md)\n", &["missing-image-refs"]);
}

// ── Controls ───────────────────────────────────────────────────────────

#[test]
fn an_absent_link_target_recovers_under_its_allowance() {
    assert_recovered(
        "recovery_absent_link",
        "",
        "[Guide](./gone.md)\n",
        &["missing-hyperlinks"],
        "dm.reference.missing_hyperlink",
        "[Guide](",
    );
}

#[test]
fn an_absent_image_target_recovers_under_its_allowance() {
    assert_recovered(
        "recovery_absent_image",
        "",
        "![Guide](./gone.png)\n",
        &["missing-image-refs"],
        "dm.reference.missing_image",
        "![Guide](",
    );
}

#[test]
fn an_absent_inclusion_target_recovers_under_its_allowance() {
    assert_recovered(
        "recovery_absent_inclusion",
        "",
        "Before\n\n::file ./gone.md\n",
        &["missing-transclusions"],
        "dm.reference.missing_transclusion",
        "Before",
    );
}

/// An empty link target only warns, with no allowance.
#[test]
fn an_empty_link_target_warns() {
    assert_recovered("recovery_empty_link", "", "[Guide]()\n", &[], "dm.reference.missing_hyperlink", "Guide");
}

/// A TOC chain that ends in `false` and finds no file reports nothing.
#[test]
fn a_suppressed_toc_chain_stays_clean() {
    for lifecycle in [false, true] {
        let fixture = CliProcessFixture::named("recovery_suppressed_toc");
        let document = document(&fixture, "", lifecycle, "# Doc\n\n::toc-linking \"./gone.md | false\"\n");
        let (status, report) = report(&fixture, &document, &[]);
        assert_eq!(status, Some(0), "{report:#}");
        assert_eq!(report["diagnostics"], json!([]), "{report:#}");
    }
}
