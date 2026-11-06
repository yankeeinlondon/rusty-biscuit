//! The reference report: grouping, wording, summaries, and every output's
//! location link, from real validation runs in fixture directories wherever a
//! run can produce the case.

use std::path::{Path, PathBuf};
use std::time::Duration;

use biscuit_terminal::output::{OutputTarget, render_output};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeSource};
use darkmatter::markdown::reference::validate::{
    ReferenceIssue, ReferenceIssueCode, ReferenceReportVisibility, ReferenceSeverity, ReferenceValidationOptions,
    ReferenceValidationReport, ValidationReportView,
};
use darkmatter::markdown::reference::{
    ReferenceGraphOptions, ReferenceKind, ReferenceOrigin, ReferenceReportDocument,
    ReferenceSyntax,
};
use renderable::output::{CommandOutput, OutputOptions, TerminalProfile, Width};
use renderable::color::{ColorDepth, ColorMode};

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        Self { _dir: dir, root }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn report(&self, path: &Path) -> ReferenceValidationReport {
        let mut options = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
            &crate::request_support::request_at(&self.root, ComposeOptions::new()),
        ));
        options.validate_fragments = true;
        Markdown::try_from(path).unwrap().validate_references(options).unwrap()
    }

    fn view(&self, path: &Path) -> ValidationReportView {
        ValidationReportView::new(self.report(path)).with_launch_dir(&self.root)
    }

    fn uri(&self, name: &str) -> String {
        url::Url::from_file_path(self.root.join(name)).unwrap().to_string()
    }

    fn path_text(&self, name: &str) -> String {
        biscuit_file::to_portable_string(&self.root.join(name))
    }
}

fn plain(view: &ValidationReportView) -> String {
    fold(view, OutputTarget::Plain, 0)
}

fn fold(view: &ValidationReportView, target: OutputTarget, verbosity: u8) -> String {
    let options = OutputOptions::new(Width::Unlimited, TerminalProfile::plain()).with_verbosity(verbosity);
    render_output(view, target, &options).unwrap().text
}

fn osc8(view: &ValidationReportView) -> String {
    let profile = TerminalProfile {
        color_depth: ColorDepth::TrueColor,
        color_mode: ColorMode::Dark,
        is_tty: true,
        hyperlinks: true,
        italics: true,
        ..TerminalProfile::plain()
    };
    render_output(view, OutputTarget::Terminal, &OutputOptions::new(Width::Unlimited, profile)).unwrap().text
}

/// An issue no local validation run produces.
fn issue(code: ReferenceIssueCode, severity: ReferenceSeverity, source: ComposeSource, reference: &str) -> ReferenceIssue {
    ReferenceIssue {
        code,
        message: reference.to_string(),
        severity,
        kind: ReferenceKind::Hyperlink,
        reference_display: reference.to_string(),
        reference_id: reference.to_string(),
        origin: ReferenceOrigin {
            source,
            line: 1,
            span: 0..1,
            syntax: ReferenceSyntax::MarkdownLink,
            authored: None,
            section: None,
        },
        included_via: Vec::new(),
        detail: None,
    }
}

fn report_of(issues: Vec<ReferenceIssue>, scanned: usize) -> ReferenceValidationReport {
    ReferenceValidationReport { references_scanned: scanned, issues, ..Default::default() }
}

fn file_source(name: &str) -> ComposeSource {
    ComposeSource::File(PathBuf::from("/fixture").join(name))
}

fn rows_with<'a>(text: &'a str, needle: &str) -> Vec<&'a str> {
    text.lines().filter(|line| line.contains(needle)).collect()
}

#[test]
fn a_report_reads_as_title_group_rows_and_summary() {
    let fixture = Fixture::new();
    let path = fixture.write(
        "docs/types.md",
        "# Types\n\n[one](./one.md)\n\n## Setup\n\n![two](./two.png) [frag](#nope)\n",
    );
    let view = ValidationReportView::new(fixture.report(&path)).with_launch_dir(&fixture.root);

    assert_eq!(
        plain(&view),
        "\
Broken references in types.md

docs/types.md
  file not found in hyperlink       `./one.md`   types.md:3:1
  file not found in image           `./two.png`  types.md:7:1
  no matching heading in hyperlink  `#nope`      types.md:7:19

Checked 3 references across 1 document: 0 ok, 3 broken
  2 × file not found · 1 × no matching heading"
    );
}

#[test]
fn a_report_has_no_failure_row_verdict_or_kind_heading() {
    let fixture = Fixture::new();
    let path = fixture.write("doc.md", "[a](./a*.md)\n");
    let text = plain(&fixture.view(&path));
    for banned in ["failure:", "is not valid", "Invalid Hyperlink", "references scanned"] {
        assert!(!text.contains(banned), "{banned}: {text}");
    }
    assert_eq!(serde_json::to_value(fixture.view(&path).as_data()).unwrap()["issues"][0]["code"], "MissingLocalTarget");
}

#[test]
fn documents_with_equal_file_names_differ_by_relative_path() {
    let fixture = Fixture::new();
    fixture.write("a/doc.md", "[x](./x.md)\n");
    fixture.write("b/doc.md", "[y](./y.md)\n");
    let path = fixture.write("root.md", "::file a/doc.md\n\n::file b/doc.md\n");
    let text = plain(&fixture.view(&path));
    assert!(text.lines().any(|line| line == "a/doc.md (transcluded at root.md:1)"), "{text}");
    assert!(text.lines().any(|line| line == "b/doc.md (transcluded at root.md:3)"), "{text}");
    assert!(text.contains("Checked 4 references across 3 documents: 2 ok, 2 broken"), "{text}");
}

#[test]
fn a_deep_issue_shows_its_whole_trail_outermost_first() {
    let fixture = Fixture::new();
    fixture.write("leaf.md", "\n[x](./x.md)\n");
    fixture.write("mid.md", "---\ntitle: t\n---\n\n\n::file leaf.md\n");
    let path = fixture.write("root.md", "# Root\n\n::file mid.md\n");
    let view = fixture.view(&path);

    let text = plain(&view);
    assert!(text.contains("leaf.md (transcluded at root.md:3 → mid.md:6)"), "{text}");
    assert!(text.contains("leaf.md:2:1"), "{text}");

    let data = serde_json::to_value(view.as_data()).unwrap();
    let issue = &data["issues"][0];
    let via = issue["included_via"].as_array().unwrap();
    assert_eq!(via.len(), 2);
    assert_eq!(via[0]["line"], 3);
    assert_eq!(via[1]["line"], 6);
    assert_eq!(std::path::Path::new(via[1]["file"].as_str().unwrap()), fixture.root.join("mid.md"));
}

#[test]
fn a_child_included_twice_is_two_groups_and_one_document() {
    let fixture = Fixture::new();
    fixture.write("child.md", "[x](./x.md)\n");
    let path = fixture.write("twice.md", "::file child.md\n\ntext\n\n::file child.md\n");
    let report = fixture.report(&path);
    assert_eq!(report.document_count(), 2);
    assert_eq!(report.issues.len(), 2);
    assert_eq!(report.references_scanned, 4, "references count per occurrence");

    let text = plain(&ValidationReportView::new(report).with_launch_dir(&fixture.root));
    assert!(text.contains("child.md (transcluded at twice.md:1)"), "{text}");
    assert!(text.contains("child.md (transcluded at twice.md:5)"), "{text}");
    assert!(text.contains("Checked 4 references across 2 documents: 2 ok, 2 broken"), "{text}");
}

#[test]
fn copies_of_a_nested_child_keep_their_own_whole_trails() {
    let fixture = Fixture::new();
    fixture.write("leaf.md", "[x](./x.md)\n");
    fixture.write("mid.md", "text\n\n::file leaf.md\n");
    let path = fixture.write("root.md", "::file mid.md\n\ntext\n\n::file mid.md\n");
    let text = plain(&fixture.view(&path));
    assert!(text.contains("leaf.md (transcluded at root.md:1 → mid.md:3)"), "{text}");
    assert!(text.contains("leaf.md (transcluded at root.md:5 → mid.md:3)"), "{text}");
}

#[test]
fn the_root_group_comes_first_whatever_the_walk_reaches_first() {
    let fixture = Fixture::new();
    fixture.write("child.md", "[in child](./child-missing.md)\n");
    let path = fixture.write("root.md", "::file child.md\n\n[in root](./root-missing.md)\n");
    let text = plain(&fixture.view(&path));
    let root = text.find("./root-missing.md").unwrap();
    let child = text.find("./child-missing.md").unwrap();
    assert!(root < child, "{text}");
}

#[test]
fn clean_documents_still_count() {
    let fixture = Fixture::new();
    fixture.write("clean.md", "no references here\n");
    let path = fixture.write("root.md", "::file clean.md\n");
    let report = fixture.report(&path);
    assert_eq!(
        report.documents,
        vec![
            ReferenceReportDocument { source: ComposeSource::File(path.clone()), included_via: Vec::new() },
            ReferenceReportDocument {
                source: ComposeSource::File(fixture.root.join("clean.md")),
                included_via: vec![darkmatter::markdown::reference::ReferenceTrailStep {
                    file: ComposeSource::File(path.clone()),
                    line: Some(1),
                }],
            },
        ]
    );
    assert_eq!(plain(&fixture.view(&path)), "Checked 1 reference across 2 documents: all ok");
}

#[test]
fn a_remote_group_shows_its_url_unlinked() {
    let url: url::Url = "https://example.com/page.md".parse().unwrap();
    let source = ComposeSource::Url(url);
    let report = report_of(
        vec![issue(ReferenceIssueCode::MissingFragmentTarget, ReferenceSeverity::Error, source.clone(), "#x")],
        1,
    );
    let view = ValidationReportView::new(report).with_root_source(source);

    let text = plain(&view);
    assert_eq!(
        text,
        "\
Broken references in https://example.com/page.md

https://example.com/page.md
  no matching heading in hyperlink  `#x`  https://example.com/page.md

Checked 1 reference across 0 documents: 0 ok, 1 broken"
    );
    assert!(!osc8(&view).contains("\x1b]8;;https"), "{:?}", osc8(&view));
    assert!(!fold(&view, OutputTarget::Markdown, 0).contains("](https"));
}

#[test]
fn an_in_memory_row_shows_its_source_name_without_a_position() {
    let md = Markdown::new("[a](https://[bad)\n");
    let options = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
        &crate::request_support::request(ComposeOptions::new()),
    ));
    let report = md.validate_references(options).unwrap();
    assert_eq!(report.issues[0].code, ReferenceIssueCode::InvalidUrl);
    let text = plain(&ValidationReportView::new(report));
    let rows = rows_with(&text, "malformed URL");
    assert_eq!(rows.len(), 1, "{text}");
    assert!(rows[0].trim_end().ends_with("<stdin>"), "{text}");
    assert!(!text.contains("<stdin>:"), "{text}");
    assert!(text.starts_with("Broken references in <stdin>\n\n<stdin>\n"), "{text}");
}

#[test]
fn every_issue_code_has_its_reason() {
    let reasons = [
        (ReferenceIssueCode::MissingLocalTarget, "file not found"),
        (ReferenceIssueCode::MissingFragmentTarget, "no matching heading"),
        (ReferenceIssueCode::InvalidUrl, "malformed URL"),
        (ReferenceIssueCode::RemoteUnreachable, "unreachable"),
        (ReferenceIssueCode::UnsupportedScheme, "unsupported scheme"),
        (ReferenceIssueCode::MissingSourceContext, "no document context"),
        (ReferenceIssueCode::RemoteDisallowed, "remote check disabled"),
        (ReferenceIssueCode::MalformedHtmlTag, "malformed HTML tag"),
        (ReferenceIssueCode::MalformedCssImport, "malformed CSS import"),
        (ReferenceIssueCode::MalformedMetaTag, "malformed meta tag"),
    ];
    for (code, reason) in reasons {
        assert_eq!(code.reason_phrase(), reason);
        let report = report_of(vec![issue(code, ReferenceSeverity::Error, file_source("a.md"), "x")], 1);
        assert!(plain(&ValidationReportView::new(report)).contains(reason), "{reason}");
    }
}

#[test]
fn every_syntax_has_its_construct() {
    let constructs = [
        (ReferenceSyntax::MarkdownLink, "in hyperlink"),
        (ReferenceSyntax::HtmlAnchor, "in hyperlink"),
        (ReferenceSyntax::MarkdownImage, "in image"),
        (ReferenceSyntax::HtmlImage, "in image"),
        (ReferenceSyntax::HtmlVideoTag, "in <video>"),
        (ReferenceSyntax::HtmlAudioTag, "in <audio>"),
        (ReferenceSyntax::HtmlSourceTag, "in <source>"),
        (ReferenceSyntax::HtmlIframeTag, "in <iframe>"),
        (ReferenceSyntax::DirectiveFile, "in ::file"),
        (ReferenceSyntax::DirectiveCode, "in ::code"),
        (ReferenceSyntax::DirectiveUrl, "in ::url"),
        (ReferenceSyntax::DirectiveTocLinking, "in ::toc-linking"),
        (ReferenceSyntax::DirectiveFileLinks, "in ::file-links"),
        (ReferenceSyntax::FrontmatterPrologue, "in prologue:"),
        (ReferenceSyntax::FrontmatterEpilogue, "in epilogue:"),
        (ReferenceSyntax::HtmlLinkTag, "in <link>"),
        (ReferenceSyntax::HtmlScriptTag, "in <script>"),
        (ReferenceSyntax::HtmlStyleTag, "in <style>"),
        (ReferenceSyntax::CssAtImport, "in CSS @import"),
        (ReferenceSyntax::CssFontFaceSrc, "in CSS @font-face"),
        (ReferenceSyntax::HtmlMetaTag, "in <meta>"),
    ];
    for (syntax, construct) in constructs {
        assert_eq!(syntax.construct_phrase(), construct);
        let mut only = issue(ReferenceIssueCode::MissingLocalTarget, ReferenceSeverity::Error, file_source("a.md"), "x");
        only.origin.syntax = syntax;
        let text = plain(&ValidationReportView::new(report_of(vec![only], 1)));
        assert!(text.replace('`', "").contains(construct), "{construct}: {text}");
    }
}

/// A broken link, a link in inserted data (a warning), and a `mailto:` (info).
fn mixed_severity_report() -> ReferenceValidationReport {
    report_of(
        vec![
            issue(ReferenceIssueCode::MissingLocalTarget, ReferenceSeverity::Error, file_source("a.md"), "./gone.md"),
            issue(ReferenceIssueCode::MissingLocalTarget, ReferenceSeverity::Warning, file_source("a.md"), "./data.md"),
            issue(ReferenceIssueCode::UnsupportedScheme, ReferenceSeverity::Info, file_source("a.md"), "mailto:x"),
        ],
        3,
    )
}

#[test]
fn errors_visibility_hides_warnings_and_info_entirely() {
    let text = plain(&ValidationReportView::new(mixed_severity_report()));
    assert!(text.contains("./gone.md"), "{text}");
    assert!(!text.contains("./data.md"), "{text}");
    assert!(!text.contains("mailto:x"), "{text}");
    assert!(!text.contains("warning"), "{text}");
    assert!(text.ends_with("2 ok, 1 broken"), "{text}");
}

#[test]
fn all_visibility_lists_every_severity_and_counts_them_apart() {
    let view = ValidationReportView::new(mixed_severity_report()).with_visibility(ReferenceReportVisibility::All);
    let text = plain(&view);
    for reference in ["./gone.md", "./data.md", "mailto:x"] {
        assert!(text.contains(reference), "{reference}: {text}");
    }
    assert!(text.ends_with("2 ok, 1 broken\n  1 warning · 1 info"), "{text}");

    let colored = osc8(&view);
    let red = rows_with(&colored, "./gone.md")[0];
    let yellow = rows_with(&colored, "./data.md")[0];
    let dimmed = rows_with(&colored, "mailto:x")[0];
    assert!(red.starts_with("  \x1b[38;2;"), "{red:?}");
    assert_ne!(red[..20], yellow[..20], "{red:?} {yellow:?}");
    assert!(dimmed.contains("\x1b[2m"), "{dimmed:?}");
}

#[test]
fn warnings_do_not_make_a_report_broken() {
    let warning = issue(ReferenceIssueCode::MissingSourceContext, ReferenceSeverity::Warning, file_source("a.md"), "x");
    let info = issue(ReferenceIssueCode::UnsupportedScheme, ReferenceSeverity::Info, file_source("a.md"), "y");
    let other = issue(ReferenceIssueCode::MissingSourceContext, ReferenceSeverity::Warning, file_source("a.md"), "z");
    let report = report_of(vec![warning, info, other], 3);
    assert!(report.is_valid());

    let all = ValidationReportView::new(report.clone()).with_visibility(ReferenceReportVisibility::All);
    let text = plain(&all);
    assert!(!text.contains("Broken references"), "{text}");
    assert!(text.contains("no document context"), "{text}");
    assert!(text.ends_with("Checked 3 references across 0 documents: all ok\n  2 warnings · 1 info"), "{text}");

    assert_eq!(
        plain(&ValidationReportView::new(report)),
        "Checked 3 references across 0 documents: all ok"
    );
}

fn broken(code: ReferenceIssueCode) -> ReferenceIssue {
    issue(code, ReferenceSeverity::Error, file_source("a.md"), "x")
}

#[test]
fn a_tally_runs_from_most_common_to_least() {
    use ReferenceIssueCode::{InvalidUrl, MissingFragmentTarget, MissingLocalTarget};
    let report = report_of(vec![broken(InvalidUrl), broken(MissingLocalTarget), broken(MissingLocalTarget), broken(MissingFragmentTarget), broken(MissingLocalTarget), broken(MissingFragmentTarget)], 6);
    let text = plain(&ValidationReportView::new(report));
    assert!(text.ends_with("  3 × file not found · 2 × no matching heading · 1 × malformed URL"), "{text}");
}

#[test]
fn a_tally_tie_keeps_first_occurrence() {
    use ReferenceIssueCode::{InvalidUrl, MissingLocalTarget};
    let report = report_of(vec![broken(InvalidUrl), broken(MissingLocalTarget)], 2);
    let text = plain(&ValidationReportView::new(report));
    assert!(text.ends_with("  1 × malformed URL · 1 × file not found"), "{text}");
}

#[test]
fn one_reason_adds_no_tally() {
    let report = report_of(vec![broken(ReferenceIssueCode::InvalidUrl), broken(ReferenceIssueCode::InvalidUrl)], 2);
    let text = plain(&ValidationReportView::new(report));
    assert!(text.ends_with("0 ok, 2 broken"), "{text}");
    assert!(!text.contains('×'), "{text}");
}

#[test]
fn singular_forms_and_empty_documents() {
    let fixture = Fixture::new();
    let empty = fixture.write("empty.md", "nothing here\n");
    assert_eq!(plain(&fixture.view(&empty)), "Checked 0 references across 1 document: all ok");

    let one_ok = fixture.write("ok.md", "[a](https://example.com)\n");
    assert_eq!(plain(&fixture.view(&one_ok)), "Checked 1 reference across 1 document: all ok");

    let one_broken = fixture.write("broken.md", "[a](./gone.md)\n");
    assert!(
        plain(&fixture.view(&one_broken)).ends_with("Checked 1 reference across 1 document: 0 ok, 1 broken"),
    );
}

#[test]
fn detail_lines_sit_under_their_rows_without_widening_the_columns() {
    let fixture = Fixture::new();
    let path = fixture.write("doc.md", "[glob](./a*.md) [url](https://[bad)\n");
    let text = plain(&fixture.view(&path));
    assert_eq!(
        text,
        "\
Broken references in doc.md

doc.md
  file not found in hyperlink  `./a*.md`       doc.md:1:1
  `*`, `?`, and `[` are literal in a file reference; to match a set of files, use a form that accepts a glob reference (for example `::file-links`)
  malformed URL in hyperlink   `https://[bad`  doc.md:1:17
  invalid IPv6 address

Checked 2 references across 1 document: 0 ok, 2 broken
  1 × file not found · 1 × malformed URL"
    );
}

#[test]
fn a_detail_line_wraps_at_a_bounded_width() {
    let fixture = Fixture::new();
    let path = fixture.write("doc.md", "[glob](./a*.md)\n");
    let options = OutputOptions::new(Width::Columns(60), TerminalProfile::plain());
    let text = render_output(&fixture.view(&path), OutputTarget::Plain, &options).unwrap().text;
    assert!(text.lines().all(|line| line.chars().count() <= 60), "{text}");
    assert!(text.contains("are literal in"), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_remote_failure_says_its_status_and_a_timeout_its_limit() {
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(any()).respond_with(ResponseTemplate::new(404)).mount(&server).await;
    let slow = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(4)))
        .mount(&slow)
        .await;

    let fixture = Fixture::new();
    let path = fixture.write("doc.md", &format!("[a]({}/a)\n\n[b]({}/b)\n", server.uri(), slow.uri()));
    let mut options = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
        &crate::request_support::request_at(&fixture.root, ComposeOptions::new()),
    ));
    options.validate_remote = true;
    options.remote_timeout = Duration::from_secs(1);
    let report = Markdown::try_from(path.as_path()).unwrap().validate_references(options).unwrap();

    let text = plain(&ValidationReportView::new(report).with_launch_dir(&fixture.root));
    assert_eq!(rows_with(&text, "unreachable").len(), 2, "{text}");
    assert_eq!(rows_with(&text, "server returned 404").len(), 1, "{text}");
    assert_eq!(rows_with(&text, "timed out after 1s").len(), 1, "{text}");
}

#[test]
fn markdown_writes_the_exact_location_link() {
    let fixture = Fixture::new();
    let path = fixture.write("docs/types.md", "# Simplified Schema Types\n\n[one](./one.md)\n");
    let view = fixture.view(&path);
    let expected = format!(
        "[types.md:3:1]({}#simplified-schema-types '{{ qp: {{ line: 3, column: 1 }} }}')",
        fixture.path_text("docs/types.md"),
    );
    for target in [OutputTarget::Markdown, OutputTarget::MarkdownPlus] {
        let text = fold(&view, target, 0);
        assert!(text.contains(&expected), "{target:?}: {text}");
        assert!(text.contains("`./one.md`"), "{target:?}: {text}");
    }
    let text = fold(&view, OutputTarget::Markdown, 0);
    assert!(text.contains(&format!("[docs/types.md]({})", fixture.path_text("docs/types.md"))), "{text}");
}

#[test]
fn markdown_location_above_the_first_heading_has_no_fragment() {
    let fixture = Fixture::new();
    let path = fixture.write("doc.md", "[one](./one.md)\n");
    let text = fold(&fixture.view(&path), OutputTarget::Markdown, 0);
    let expected = format!("[doc.md:1:1]({} '{{ qp: {{ line: 1, column: 1 }} }}')", fixture.path_text("doc.md"));
    assert!(text.contains(&expected), "{text}");
}

#[test]
fn html_moves_the_section_and_position_into_the_query() {
    let fixture = Fixture::new();
    let path = fixture.write("docs/types.md", "# Simplified Schema Types\n\n[one](./one.md)\n");
    let text = fold(&fixture.view(&path), OutputTarget::Html, 0);
    let expected = format!(
        "href=\"{}?heading=simplified-schema-types&amp;line=3&amp;column=1\"",
        fixture.path_text("docs/types.md"),
    );
    assert!(text.contains(&expected), "{text}");
    assert!(!text.contains("types.md#"), "{text}");
    assert!(!text.contains("data-"), "{text}");
}

#[test]
fn osc8_links_carry_a_bare_uri_and_their_position_as_parameters() {
    let fixture = Fixture::new();
    let path = fixture.write("docs/types.md", "# Simplified Schema Types\n\n[one](./one.md)\n");
    let text = osc8(&fixture.view(&path));
    let expected = format!(
        "\x1b]8;line=3:column=1:heading=simplified-schema-types;{}\x1b\\",
        fixture.uri("docs/types.md"),
    );
    assert!(text.contains(&expected), "{text:?}");
    let title = format!("\x1b]8;;{}\x1b\\", fixture.uri("docs/types.md"));
    assert!(text.contains(&title), "the headings link the file: {text:?}");
}

#[test]
fn verbose_plain_prints_the_bare_file_uri() {
    let fixture = Fixture::new();
    let path = fixture.write("docs/types.md", "# Types\n\n[one](./one.md)\n");
    let text = fold(&fixture.view(&path), OutputTarget::Plain, 1);
    let expected = format!("types.md:3:1 <{}>", fixture.uri("docs/types.md"));
    assert!(text.contains(&expected), "{text}");
}

#[test]
fn a_path_with_url_delimiters_stays_one_destination() {
    let fixture = Fixture::new();
    let path = fixture.write("c#/doc.md", "[one](./one.md)\n");
    let text = fold(&fixture.view(&path), OutputTarget::Html, 0);
    assert!(text.contains("c%23/doc.md?line=1&amp;column=1"), "{text}");
}

#[test]
fn group_paths_come_from_the_launch_directory_given() {
    let fixture = Fixture::new();
    let path = fixture.write("a/b/doc.md", "[one](./one.md)\n");
    let elsewhere = fixture.root.join("a/c");
    let text = plain(&ValidationReportView::new(fixture.report(&path)).with_launch_dir(&elsewhere));
    assert!(text.lines().any(|line| line == "../b/doc.md"), "{text}");

    let unanchored = plain(&ValidationReportView::new(fixture.report(&path)));
    assert!(unanchored.lines().any(|line| line == fixture.path_text("a/b/doc.md")), "{unanchored}");
}

#[test]
fn a_narrow_terminal_wraps_without_losing_the_location() {
    let fixture = Fixture::new();
    let path = fixture.write("doc.md", "[one](./a-rather-long-reference-name.md)\n");
    let options = OutputOptions::new(Width::Columns(50), TerminalProfile::plain());
    let text = render_output(&fixture.view(&path), OutputTarget::Plain, &options).unwrap().text;
    assert!(text.lines().all(|line| line.chars().count() <= 50), "{text}");
    assert!(text.contains("doc.md:1:1"), "{text}");
}

#[test]
fn the_data_form_is_the_report_and_the_listing_adds_references() {
    let fixture = Fixture::new();
    let path = fixture.write("doc.md", "# Head\n\n[one](./one.md) [ok](https://example.com)\n");
    let view = fixture.view(&path);
    let data = serde_json::to_value(view.as_data()).unwrap();
    assert_eq!(data, serde_json::to_value(fixture.report(&path)).unwrap());
    assert!(data.get("references").is_none());
    let issue = &data["issues"][0];
    assert_eq!(issue["line"], 3);
    assert_eq!(issue["column"], 1);
    assert_eq!(issue["section"], "head");
    assert_eq!(issue["inserted"], false);

    let listed = view.clone().with_reference_listing();
    let data = serde_json::to_value(listed.as_data()).unwrap();
    assert_eq!(data["references"].as_array().unwrap().len(), 2);
    assert_eq!(data["valid"], false);

    let text = plain(&listed);
    assert!(text.contains("All references"), "{text}");
    assert!(text.contains("https://example.com"), "{text}");
    assert!(text.contains("Remote URL not verified: https://example.com"), "{text}");
    assert!(text.ends_with("1 ok, 1 broken"), "{text}");
}

#[test]
fn an_inserted_reference_points_at_its_construct_in_the_report() {
    let fixture = Fixture::new();
    let path = fixture.write(
        "doc.md",
        "---\nlink: \"[l](./nope.md)\"\n---\nsee {{ link }}\n",
    );
    let view = fixture.view(&path).with_visibility(ReferenceReportVisibility::All);
    let data = serde_json::to_value(view.as_data()).unwrap();
    let issue = &data["issues"][0];
    assert_eq!(issue["severity"], "warning");
    assert_eq!(issue["inserted"], true);
    assert_eq!(issue["line"], 4);
    assert_eq!(issue["column"], 5);
}
