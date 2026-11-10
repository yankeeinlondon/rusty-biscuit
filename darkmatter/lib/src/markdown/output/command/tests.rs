use biscuit_terminal::output::{OutputTarget, render_output, write_output};
use biscuit_terminal::terminal::Terminal;
use renderable::output::{CommandOutput, OutputOptions, TerminalProfile, Width};
use serde_json::Value;

use super::*;
use crate::layout::DarkmatterPage;
use crate::markdown::Markdown;
use crate::markdown::reference::validate::ReferenceValidationReport;

const SOURCE: &str = "---\ntitle: A title\n---\nA soft-broken paragraph\nthat continues past twenty columns here.\n";

fn page() -> DarkmatterPage {
    DarkmatterPage::new(&Terminal::new_optimistic(80))
}

fn plain(width: Width) -> OutputOptions {
    OutputOptions::new(width, TerminalProfile::plain())
}

fn prepared(target: OutputTarget, options: &OutputOptions) -> DocumentOutput {
    DocumentOutput::prepare(&Markdown::from(SOURCE), &page(), target, options).expect("prepare")
}

#[test]
fn document_markdown_source_keeps_the_body() {
    let document = Markdown::from(SOURCE);
    assert_eq!(document_markdown(&document, true, Width::Source), document.as_string());
    assert_eq!(document_markdown(&document, false, Width::Source), document.content());
}

#[test]
fn document_markdown_columns_reflows_only_the_body() {
    let document = Markdown::from(SOURCE);
    let output = document_markdown(&document, true, Width::Columns(20));
    let (frontmatter, body) = output.rsplit_once("---\n").expect("frontmatter");
    assert_eq!(frontmatter, "---\ntitle: A title\n");
    assert!(body.lines().count() > 2, "{body}");
    assert!(body.lines().all(|line| line.chars().count() <= 20), "{body}");
}

#[test]
fn document_markdown_unlimited_unwraps_only_the_body() {
    let document = Markdown::from(SOURCE);
    assert_eq!(
        document_markdown(&document, false, Width::Unlimited).trim_end(),
        "A soft-broken paragraph that continues past twenty columns here."
    );
    assert!(document_markdown(&document, true, Width::Unlimited).starts_with("---\ntitle: A title\n---\n"));
}

#[test]
fn document_markdown_removes_embedded_metadata() {
    let source = "Before.\n\n<!--bt:render-tree:start:e30=-->\n\nInner.\n\n<!--bt:render-tree:end-->\n";
    let document = Markdown::from(source);
    assert_eq!(
        document_markdown(&document, false, Width::Source),
        crate::markdown::render_tree::embedded::without_embedded_metadata(source)
    );
}

#[test]
fn prepare_plain_stores_the_page_text_and_writes_it_through_dispatch() {
    let options = plain(Width::Columns(30));
    let output = prepared(OutputTarget::Plain, &options);
    let expected = page()
        .with_output_options(&options)
        .render(&Markdown::from(SOURCE))
        .expect("render");

    assert_eq!(output.render_terminal(&options).as_deref(), Some(expected.as_str()));
    let mut written = Vec::new();
    write_output(&output, OutputTarget::Plain, &options, &mut written).expect("write");
    assert_eq!(String::from_utf8(written).expect("utf-8"), expected);
    assert!(!expected.contains('\u{1b}'));
    assert_eq!(output.render_markdown(&options), None);
    assert_eq!(output.render_html(&options), None);
}

#[test]
fn prepare_terminal_on_a_pipe_equals_plain() {
    let mut piped = Terminal::new_optimistic(80).output_profile();
    piped.is_tty = false;
    piped.graphics = false;
    let options = OutputOptions::new(Width::Columns(30), piped);
    let terminal = prepared(OutputTarget::Terminal, &options);
    let plain_output = prepared(OutputTarget::Plain, &plain(Width::Columns(30)));
    assert_eq!(
        terminal.render_terminal(&options),
        plain_output.render_terminal(&options)
    );
}

#[test]
fn prepare_html_comes_from_the_page_builder() {
    let options = plain(Width::Columns(40));
    let framed = page().with_margin_x(2);
    let output = DocumentOutput::prepare(&Markdown::from(SOURCE), &framed, OutputTarget::Html, &options).expect("prepare");
    let html = output.render_html(&options).expect("html");

    assert!(html.starts_with("<!DOCTYPE html>"), "{html}");
    assert!(html.contains("class=\"darkmatter-page\""), "the page frame: {html}");
    assert!(html.contains(":where(p, li, blockquote"), "the prose constraint: {html}");
    assert!(html.contains("40ch"), "{html}");
    assert_eq!(output.render_terminal(&options), None);
    assert_eq!(
        render_output(&output, OutputTarget::Html, &options).expect("html").text,
        html
    );
}

#[test]
fn prepare_html_without_columns_adds_no_prose_constraint() {
    for width in [Width::Unlimited, Width::Source] {
        let options = plain(width);
        let html = prepared(OutputTarget::Html, &options).render_html(&options).expect("html");
        assert!(!html.contains(":where(p, li, blockquote"), "{width:?}: {html}");
    }
}

#[test]
fn prepare_markdown_is_the_source_text_under_the_width() {
    let source_options = plain(Width::Source);
    let output = prepared(OutputTarget::Markdown, &source_options);
    assert_eq!(output.render_markdown(&source_options).as_deref(), Some(Markdown::from(SOURCE).as_string().as_str()));
    assert_eq!(output.render_terminal(&source_options), None);

    let columns = plain(Width::Columns(20));
    let reflowed = prepared(OutputTarget::Markdown, &columns);
    assert_eq!(
        reflowed.render_markdown(&columns).as_deref(),
        Some(document_markdown(&Markdown::from(SOURCE), true, Width::Columns(20)).as_str())
    );
    assert_eq!(reflowed.markdown(), reflowed.render_markdown(&columns).expect("markdown"));

    let unlimited = plain(Width::Unlimited);
    let unwrapped = prepared(OutputTarget::Markdown, &unlimited);
    assert!(unwrapped.markdown().contains("paragraph that continues"), "{}", unwrapped.markdown());
}

#[test]
fn prepare_markdown_plus_uses_the_page_tree_and_the_generic_fold() {
    let source = "::disclosure\nSummary\n::details\nBody.\n::end-disclosure\n";
    let options = plain(Width::Source);
    let document = Markdown::from(source);
    let output = DocumentOutput::prepare(&document, &page(), OutputTarget::MarkdownPlus, &options).expect("prepare");

    assert_eq!(output.render_markdown_plus(&options), None);
    let folded = render_output(&output, OutputTarget::MarkdownPlus, &options).expect("fold").text;
    assert_eq!(folded, page().render_to_markdown_plus(&document).expect("page"));
    assert!(folded.contains("<details"), "{folded}");
}

#[test]
fn prepare_data_renders_nothing_and_defaults_the_envelope() {
    let options = plain(Width::Source);
    let output = prepared(OutputTarget::Json, &options);
    assert_eq!(output.render_terminal(&options), None);
    assert_eq!(output.render_markdown(&options), None);
    assert_eq!(output.render_html(&options), None);

    let envelope = output.as_data();
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.document.as_deref(), Some(output.markdown()));
    assert!(envelope.diagnostics.is_empty());
}

#[test]
fn with_envelope_replaces_the_data() {
    let options = plain(Width::Source);
    let mut envelope = prepared(OutputTarget::Json, &options).as_data();
    envelope.outcome = Outcome::Failure;
    envelope.document = None;
    let output = prepared(OutputTarget::Json, &options).with_envelope(envelope.clone());
    assert_eq!(output.as_data(), envelope);
}

#[test]
fn a_frame_too_wide_for_the_width_fails_at_prepare() {
    let options = plain(Width::Columns(4));
    let result = DocumentOutput::prepare(&Markdown::from(SOURCE), &page().with_margin_x(5), OutputTarget::Plain, &options);
    assert!(result.is_err());
}

fn envelope() -> DocumentEnvelope {
    DocumentEnvelope {
        file: None,
        outcome: Outcome::Success,
        document: None,
        diagnostics: Vec::new(),
        summary: None,
        perf: None,
        lifecycle_output: Vec::new(),
        read_set: Vec::new(),
    }
}

#[test]
fn the_envelope_serializes_in_order_with_nulls_and_without_perf() {
    let json = serde_json::to_string(&envelope()).expect("json");
    assert_eq!(
        json,
        r#"{"file":null,"outcome":"success","document":null,"diagnostics":[],"summary":null,"lifecycle_output":[],"read_set":[]}"#
    );
}

#[test]
fn the_envelope_serializes_perf_and_lifecycle_when_present() {
    let value = DocumentEnvelope {
        file: Some("doc.md".into()),
        outcome: Outcome::Failure,
        document: Some("text".into()),
        summary: Some(ReferenceSummary {
            references: 3,
            documents: 2,
            ok: 2,
            broken: 1,
        }),
        perf: Some(PerfData {
            stages: vec![PerfStage {
                name: "compose".into(),
                milliseconds: 1.5,
            }],
        }),
        lifecycle_output: vec![LifecycleOutputEntry {
            stage: "pre".into(),
            step: "lint".into(),
            text: "ok".into(),
        }],
        ..envelope()
    };
    let json: Value = serde_json::to_value(&value).expect("json");
    let keys: Vec<&str> = json.as_object().expect("object").keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["file", "outcome", "document", "diagnostics", "summary", "perf", "lifecycle_output", "read_set"]
    );
    assert_eq!(json["outcome"], "failure");
    assert_eq!(json["summary"]["broken"], 1);
    assert_eq!(json["perf"]["stages"][0]["milliseconds"], 1.5);
    assert_eq!(json["lifecycle_output"][0]["step"], "lint");
}

#[test]
fn the_envelope_prose_is_its_document_or_empty() {
    let empty = envelope().as_prose();
    assert!(empty.root.children().is_empty());

    let with_text = DocumentEnvelope {
        document: Some("Hello.".into()),
        ..envelope()
    };
    assert_eq!(with_text.as_prose(), Markdown::from("Hello.").as_document().expect("doc"));
    assert_eq!(with_text.as_data(), with_text);
}

#[test]
fn reference_summary_uses_the_report_counts() {
    let report = ReferenceValidationReport {
        references_scanned: 5,
        ..ReferenceValidationReport::default()
    };
    assert_eq!(
        ReferenceSummary::from_report(&report),
        ReferenceSummary {
            references: 5,
            documents: 0,
            ok: 5,
            broken: 0,
        }
    );
}
