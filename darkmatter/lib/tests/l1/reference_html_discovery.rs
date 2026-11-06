//! Which HTML and CSS references the graph discovers, and where it says each
//! one is: every shape of a tag reaches validation, wherever its attributes or
//! body fall across lines.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::reference::validate::{
    ReferenceIssue, ReferenceIssueCode, ReferenceSeverity, ReferenceValidationOptions,
    ReferenceValidationReport,
};
use darkmatter::markdown::reference::{
    ReferenceGraphOptions, ReferenceKind, ReferenceSyntax, ReferenceTarget, extract_document_references,
};

fn graph_options(dir: &Path) -> ReferenceGraphOptions {
    ReferenceGraphOptions::with_compose(&crate::request_support::request_at(dir, ComposeOptions::new()))
}

/// A document in a fresh directory, validated. The directory is returned to
/// keep it alive.
fn validated(text: &str) -> (tempfile::TempDir, ReferenceValidationReport) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "doc.md", text);
    let report = validate(&path);
    (dir, report)
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    path
}

fn validate(path: &Path) -> ReferenceValidationReport {
    let options = ReferenceValidationOptions::with_graph(graph_options(path.parent().unwrap()));
    Markdown::try_from(path).unwrap().validate_references(options).unwrap()
}

fn errors(report: &ReferenceValidationReport) -> Vec<&ReferenceIssue> {
    report.issues.iter().filter(|issue| issue.severity == ReferenceSeverity::Error).collect()
}

/// `text` reports one missing target: `target`, written as `syntax`, at
/// `(line, column)`.
#[track_caller]
fn assert_one_missing(text: &str, target: &str, syntax: ReferenceSyntax, position: (usize, usize)) {
    let (_dir, report) = validated(text);
    let errors = errors(&report);
    assert_eq!(errors.len(), 1, "{text:?}: {errors:#?}");
    let issue = errors[0];
    assert_eq!(issue.code, ReferenceIssueCode::MissingLocalTarget, "{text:?}");
    assert_eq!(issue.reference_display, target, "{text:?}");
    assert_eq!(issue.origin.syntax, syntax, "{text:?}");
    let authored = issue.origin.authored.as_ref().expect("an authored position");
    assert_eq!((authored.line, authored.column), position, "{text:?}");
}

/// One `#[test]` per shape, so a failing shape names itself.
macro_rules! missing_target_rows {
    ($($name:ident: $text:expr, $target:expr, $syntax:ident, $position:expr;)+) => {
        $(
            #[test]
            fn $name() {
                assert_one_missing($text, $target, ReferenceSyntax::$syntax, $position);
            }
        )+
    };
}

missing_target_rows! {
    anchor_on_one_line: "<a href=\"./gone.md\">text</a>\n", "./gone.md", HtmlAnchor, (1, 1);
    anchor_attribute_on_next_line: "Intro\n\n<a\n  href=\"./gone.md\">text</a>\n", "./gone.md", HtmlAnchor, (3, 1);
    image_on_one_line: "<img src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 1);
    image_attribute_on_next_line: "<img\n src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 1);
    image_after_text_on_its_line: "Look: <img\n src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 7);
    script_on_one_line: "<script src=\"./gone.js\"></script>\n", "./gone.js", HtmlScriptTag, (1, 1);
    script_attribute_on_next_line: "<script\n  src=\"./gone.js\"></script>\n", "./gone.js", HtmlScriptTag, (1, 1);
    stylesheet_on_one_line: "<link rel=\"stylesheet\" href=\"./gone.css\">\n", "./gone.css", HtmlLinkTag, (1, 1);
    stylesheet_attribute_on_next_line: "<link\n  rel=\"stylesheet\"\n  href=\"./gone.css\">\n", "./gone.css", HtmlLinkTag, (1, 1);
    css_import_inline: "<style>@import \"./gone.css\";</style>\n", "./gone.css", CssAtImport, (1, 8);
    css_import_multiline: "<style>\nbody { color: red; }\n@import \"./gone.css\";\n</style>\n", "./gone.css", CssAtImport, (3, 1);
    css_import_after_multiline_open_tag: "<style\n  media=\"print\">@import \"./gone.css\";\n</style>\n", "./gone.css", CssAtImport, (2, 17);
    css_import_after_closing_tag_lookalike: "<style>\n/* </b> */\n@import \"./gone.css\";\n</style>\n", "./gone.css", CssAtImport, (3, 1);
    css_import_in_unclosed_style: "<style>\n@import \"./gone.css\";\n", "./gone.css", CssAtImport, (2, 1);
    css_import_in_running_text: "Text <style>@import \"./gone.css\";</style>\n", "./gone.css", CssAtImport, (1, 13);
    font_face_inline: "<style>@font-face { src: url(\"./gone.woff2\"); }</style>\n", "./gone.woff2", CssFontFaceSrc, (1, 26);
    font_face_multiline: "<style>\n@font-face {\n  font-family: \"F\";\n  src: url(\"./gone.woff2\") format(\"woff2\");\n}\n</style>\n", "./gone.woff2", CssFontFaceSrc, (4, 8);
    font_face_unquoted_url: "<style>\n@font-face {\n  src: url(./gone.woff2);\n}\n</style>\n", "./gone.woff2", CssFontFaceSrc, (3, 8);
    font_face_inside_media_query: "<style>\n@media print {\n  @font-face { src: url(\"./gone.woff2\"); }\n}\n</style>\n", "./gone.woff2", CssFontFaceSrc, (3, 21);
    video_on_one_line: "<video src=\"./gone.mp4\"></video>\n", "./gone.mp4", HtmlVideoTag, (1, 1);
    video_attribute_on_next_line: "<video\n  src=\"./gone.mp4\"></video>\n", "./gone.mp4", HtmlVideoTag, (1, 1);
    audio_on_one_line: "<audio src=\"./gone.mp3\"></audio>\n", "./gone.mp3", HtmlAudioTag, (1, 1);
    audio_attribute_on_next_line: "<audio\n  src=\"./gone.mp3\"></audio>\n", "./gone.mp3", HtmlAudioTag, (1, 1);
    source_on_one_line: "<source src=\"./gone.ogg\">\n", "./gone.ogg", HtmlSourceTag, (1, 1);
    source_attribute_on_next_line: "<source\n  src=\"./gone.ogg\">\n", "./gone.ogg", HtmlSourceTag, (1, 1);
    source_nested_in_video: "<video controls>\n  <source\n    src=\"./gone.mp4\">\n</video>\n", "./gone.mp4", HtmlSourceTag, (2, 3);
    iframe_on_one_line: "<iframe src=\"./gone.html\"></iframe>\n", "./gone.html", HtmlIframeTag, (1, 1);
    iframe_attribute_on_next_line: "<iframe\n  src=\"./gone.html\"></iframe>\n", "./gone.html", HtmlIframeTag, (1, 1);

    image_inside_another_tag_on_its_line: "<p><img src=\"./gone.png\"></p>\n", "./gone.png", HtmlImage, (1, 4);
    uppercase_tag_and_attribute: "<IMG SRC=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 1);
    unquoted_attribute: "<img src=gone.png>\n", "gone.png", HtmlImage, (1, 1);
    whitespace_around_a_target: "<img src=\" ./gone.png \">\n", "./gone.png", HtmlImage, (1, 1);
    tab_after_tag_name: "<img\tsrc=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 1);
    value_on_the_line_after_its_equals: "<img src =\n  \"./gone.png\">\n", "./gone.png", HtmlImage, (1, 1);
    greater_than_in_earlier_attribute: "<img alt=\"a > b\"\n  src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 1);

    image_in_block_quote: "> <img\n>  src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 3);
    second_tag_line_in_block_quote: "> <div>\n> <img src=\"./gone.png\">\n> </div>\n", "./gone.png", HtmlImage, (2, 3);
    css_in_block_quote: "> <style>\n> @import \"./gone.css\";\n> </style>\n", "./gone.css", CssAtImport, (2, 3);
    font_face_in_list_item: "- <style>\n  @font-face {\n    src: url(\"./gone.woff\");\n  }\n  </style>\n", "./gone.woff", CssFontFaceSrc, (3, 10);
    crlf_line_endings: "<div>\r\n<img\r\n src=\"./gone.png\">\r\n</div>\r\n", "./gone.png", HtmlImage, (2, 1);
    crlf_css_body: "<style>\r\n\r\n@import \"./gone.css\";\r\n</style>\r\n", "./gone.css", CssAtImport, (3, 1);
    column_counts_characters_not_bytes: "<div>é <img src=\"./gone.png\"></div>\n", "./gone.png", HtmlImage, (1, 8);
    after_frontmatter: "---\ntitle: t\n---\n\n<img\n src=\"./gone.png\">\n", "./gone.png", HtmlImage, (5, 1);
}

#[test]
fn each_tag_on_a_line_is_its_own_reference_with_its_own_position() {
    let (_dir, report) = validated("<div><img src=\"./one.png\"> <img\n  src=\"./two.png\"></div>\n");
    let found: Vec<_> = errors(&report)
        .iter()
        .map(|issue| {
            let authored = issue.origin.authored.as_ref().unwrap();
            (issue.reference_display.clone(), authored.line, authored.column)
        })
        .collect();
    assert_eq!(found, [("./one.png".to_string(), 1, 6), ("./two.png".to_string(), 1, 28)]);
}

#[test]
fn every_css_target_of_a_block_keeps_its_own_position() {
    let (_dir, report) = validated(
        "<style>\n@import \"./one.css\";\n@font-face {\n  src: url(\"./two.woff\"),\n       url(\"./three.woff\");\n}\n</style>\n",
    );
    let mut found: Vec<_> = errors(&report)
        .iter()
        .map(|issue| {
            let authored = issue.origin.authored.as_ref().unwrap();
            (issue.reference_display.clone(), authored.line, authored.column)
        })
        .collect();
    found.sort_by_key(|(_, line, _)| *line);
    assert_eq!(
        found,
        [
            ("./one.css".to_string(), 2, 1),
            ("./two.woff".to_string(), 4, 8),
            ("./three.woff".to_string(), 5, 8),
        ]
    );
}

#[test]
fn a_meta_tag_is_one_inline_record_with_no_issue_in_either_shape() {
    for text in [
        "<meta name=\"author\" content=\"./not-a-file.md\">\n",
        "<meta\n  name=\"author\"\n  content=\"./not-a-file.md\">\n",
    ] {
        let (_dir, report) = validated(text);
        assert_eq!(report.issues.len(), 0, "{text:?}: {:#?}", report.issues);

        let records = extract_document_references(text).records;
        assert_eq!(records.len(), 1, "{text:?}");
        assert_eq!(records[0].kind, ReferenceKind::MetaTag);
        assert!(matches!(records[0].target, ReferenceTarget::Inline));
        assert_eq!(records[0].attributes["content"], "./not-a-file.md");
    }
}

#[test]
fn markdown_links_and_images_are_found_by_their_own_extraction() {
    assert_one_missing("[text](./gone.md)\n", "./gone.md", ReferenceSyntax::MarkdownLink, (1, 1));
    assert_one_missing("![text](./gone.png)\n", "./gone.png", ReferenceSyntax::MarkdownImage, (1, 1));
}

#[test]
fn existing_targets_pass_in_every_multiline_shape() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.md", "b.png", "c.js", "d.css", "e.woff", "f.mp4", "g.mp3", "h.ogg", "i.html"] {
        write(dir.path(), name, "x");
    }
    let path = write(
        dir.path(),
        "doc.md",
        "<a\n href=\"./a.md\">a</a> <img\n src=\"./b.png\">\n\n<script\n src=\"./c.js\"></script>\n\n<link\n rel=\"stylesheet\" href=\"./d.css\">\n\n<style>\n@import \"./d.css\";\n@font-face {\n  src: url(\"./e.woff\");\n}\n</style>\n\n<video\n src=\"./f.mp4\"></video> <audio\n src=\"./g.mp3\"></audio>\n\n<source\n src=\"./h.ogg\">\n\n<iframe\n src=\"./i.html\"></iframe>\n",
    );

    let report = validate(&path);

    assert_eq!(report.issues.len(), 0, "{:#?}", report.issues);
    assert_eq!(report.references_valid, report.references_scanned);
    assert!(report.references_scanned >= 9, "{}", report.references_scanned);
}

/// Markup that looks like a reference but is not one: its tag is inert.
macro_rules! not_a_reference_rows {
    ($($name:ident: $text:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let (_dir, report) = validated($text);
                assert_eq!(errors(&report).len(), 0, "{:#?}", report.issues);
            }
        )+
    };
}

not_a_reference_rows! {
    tag_in_one_line_comment: "<!-- <img src=\"./gone.png\"> -->\n";
    tag_on_its_own_line_of_a_multiline_comment: "<!--\n<img src=\"./gone.png\">\n-->\n";
    tag_in_a_fence: "```html\n<img\n src=\"./gone.png\">\n```\n";
    tag_in_inline_code: "`<img src=\"./gone.png\">`\n";
    tag_in_a_line_start_script_body: "<script>\nconst tag = '<img src=\"./gone.png\">';\n</script>\n";
    tag_in_a_script_body_after_prose: "Note: <script>const tag = '<img src=\"./gone.png\">';</script>\n";
    tag_in_a_style_comment: "<style>\n/* <img src=\"./gone.png\"> */\n</style>\n";
    import_in_a_css_comment: "<style>\n/* @import \"./gone.css\"; */\n</style>\n";
    tag_in_a_textarea: "<textarea>\n<img src=\"./gone.png\">\n</textarea>\n";
    attribute_named_like_the_target_with_a_prefix: "<img data-src=\"./gone.png\">\n";
    target_text_inside_another_attribute: "<img alt='src=\"./gone.png\"'>\n";
    closing_tag: "</img>\n";
    empty_media_target: "<iframe src=\"\"></iframe>\n\n<source\n  src=\" \">\n\n<video src=\"\"></video>\n";
    empty_anchor_target: "<a href=\"\">text</a>\n";
}

/// Targets that are not local files: a media record of these must not read as
/// a missing file, and nothing here may touch the network.
macro_rules! non_file_target_rows {
    ($($name:ident: $text:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let (_dir, report) = validated($text);
                assert_eq!(report.references_scanned, 1, "{:#?}", report);
                assert_eq!(errors(&report).len(), 0, "{:#?}", report.issues);
                assert!(
                    report.issues.iter().all(|issue| issue.code != ReferenceIssueCode::MissingLocalTarget),
                    "{:#?}",
                    report.issues
                );
            }
        )+
    };
}

non_file_target_rows! {
    video_remote_url: "<video\n src=\"https://example.invalid/a.mp4\"></video>\n";
    audio_protocol_relative_url: "<audio\n src=\"//example.invalid/a.mp3\"></audio>\n";
    source_data_uri: "<source\n src=\"data:audio/ogg;base64,AAAA\">\n";
    iframe_about_blank: "<iframe\n src=\"about:blank\"></iframe>\n";
    video_blob_url: "<video src=\"blob:https://example.invalid/1\"></video>\n";
    anchor_javascript_url: "<a href=\"javascript:void(0)\">x</a>\n";
}

#[test]
fn a_media_fragment_does_not_hide_a_present_target() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.mp4", "x");
    let path = write(dir.path(), "doc.md", "<video\n src=\"./a.mp4#t=10\"></video>\n");

    let report = validate(&path);

    assert_eq!(report.issues.len(), 0, "{:#?}", report.issues);
    assert_eq!(report.references_valid, 1);
}

#[test]
fn a_fragment_only_media_target_is_a_fragment_not_a_file() {
    let records = extract_document_references("<iframe\n src=\"#top\"></iframe>\n").records;
    assert_eq!(records.len(), 1);
    assert!(matches!(records[0].target, ReferenceTarget::Fragment { .. }), "{:#?}", records[0]);
}

#[test]
fn a_child_documents_recovered_records_reach_the_graph_and_the_report() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "sub/child.md",
        "<img\n src=\"./gone.png\">\n\n<style>\n@import \"./gone.css\";\n</style>\n\n<video src=\"./gone.mp4\"></video>\n",
    );
    let root = write(dir.path(), "root.md", "::file sub/child.md\n");

    let graph = Markdown::try_from(root.as_path()).unwrap().reference_graph(graph_options(dir.path())).unwrap();
    let report = validate(&root);

    let kinds: Vec<_> = graph.iter().flat_map(|node| node.local_references.records.iter().map(|r| r.kind)).collect();
    for kind in [ReferenceKind::Image, ReferenceKind::CssImport, ReferenceKind::HtmlVideo] {
        assert!(kinds.contains(&kind), "{kind:?} missing from {kinds:?}");
    }
    let found: Vec<_> = errors(&report)
        .iter()
        .map(|issue| {
            assert_eq!(issue.included_via.len(), 1, "{issue:#?}");
            let authored = issue.origin.authored.as_ref().unwrap();
            (issue.reference_display.clone(), authored.line, authored.column)
        })
        .collect();
    assert_eq!(
        found,
        [
            ("./gone.png".to_string(), 1, 1),
            ("./gone.css".to_string(), 5, 1),
            ("./gone.mp4".to_string(), 8, 1),
        ]
    );
}

#[test]
fn document_extraction_spans_index_the_text_it_was_given() {
    let body = "> <img\n>  src=\"./a.png\">\n\n<style>\n@import \"./b.css\";\n</style>\n\n<video\n src=\"./c.mp4\"></video>\n";

    let records = extract_document_references(body).records;

    let text_of = |target: &str| {
        let record = records.iter().find(|record| record.target.raw() == Some(target)).unwrap();
        (&body[record.origin.span.clone()], record.origin.line)
    };
    assert_eq!(text_of("./a.png"), ("<img\n>  src=\"./a.png\">", 1));
    assert_eq!(text_of("./b.css"), ("@import \"./b.css\"", 5));
    assert_eq!(text_of("./c.mp4"), ("<video\n src=\"./c.mp4\">", 8));
}

#[test]
fn single_document_accessors_see_multiline_blocks_and_tags() {
    let markdown = Markdown::new(
        "<style>\n@import \"./a.css\";\n@font-face {\n  src: url(\"./b.woff\");\n}\n</style>\n\n<script>\nconsole.log(1);\n</script>\n\n<script\n src=\"./c.js\"></script>\n\n<meta\n name=\"author\"\n content=\"Ken\">\n",
    );

    let inline_css = markdown.inline_css().unwrap();
    assert_eq!(inline_css.len(), 1);
    assert!(inline_css[0].css_content.contains("@font-face"), "{:?}", inline_css[0].css_content);
    assert_eq!(markdown.css_imports().unwrap()[0].record.target.raw(), Some("./a.css"));
    assert_eq!(markdown.font_imports().unwrap()[0].record.target.raw(), Some("./b.woff"));
    assert_eq!(markdown.inline_scripts().unwrap()[0].script_content, "\nconsole.log(1);\n");
    assert_eq!(markdown.script_imports().unwrap()[0].record.target.raw(), Some("./c.js"));
    assert_eq!(markdown.meta_tags().unwrap().get("author").unwrap().as_str(), "Ken");
}

#[test]
fn setting_a_meta_tag_replaces_a_multiline_tag_with_a_quoted_greater_than() {
    let mut markdown = Markdown::new("<meta content=\"a > b\"\n  name=\"author\">\n");

    let replaced = markdown.set_meta_tag("author", "Ken");

    assert_eq!(replaced, 1);
    assert_eq!(markdown.content(), "<meta name=\"author\" content=\"Ken\">\n");
}

#[test]
fn composing_keeps_a_transcluded_multiline_tags_target_correct() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "sub/a.png", "x");
    write(dir.path(), "sub/b.mp4", "x");
    write(dir.path(), "sub/child.md", "<img\n src=\"./a.png\">\n\n<video\n src=\"./b.mp4\"></video>\n");
    let root = write(dir.path(), "root.md", "::file sub/child.md\n");
    let request = crate::request_support::request_at(dir.path(), ComposeOptions::new().with_source_file(&root));

    let (composed, _report) = Markdown::try_from(root.as_path()).unwrap().compose_with(&request).unwrap();

    let content = composed.content();
    assert!(content.contains("src=\"sub/a.png\"") || content.contains("src=\"./sub/a.png\""), "{content}");
    assert!(content.contains("src=\"sub/b.mp4\"") || content.contains("src=\"./sub/b.mp4\""), "{content}");
}

#[test]
fn composing_rewrites_the_target_attribute_not_one_sharing_its_value() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "sub/a.png", "x");
    write(dir.path(), "sub/b.png", "x");
    write(
        dir.path(),
        "sub/child.md",
        "<img data-src=\"./a.png\" src=\"./a.png\">\n\n<img data-src=\"./b.png\" SRC = \"./b.png\">\n",
    );
    let root = write(dir.path(), "root.md", "::file sub/child.md\n");
    let request = crate::request_support::request_at(dir.path(), ComposeOptions::new().with_source_file(&root));

    let (composed, _report) = Markdown::try_from(root.as_path()).unwrap().compose_with(&request).unwrap();

    let content = composed.content();
    assert!(content.contains("data-src=\"./a.png\""), "{content}");
    assert!(content.contains("data-src=\"./b.png\""), "{content}");
    assert!(content.contains(" src=\"sub/a.png\"") || content.contains(" src=\"./sub/a.png\""), "{content}");
    assert!(content.contains("SRC = \"sub/b.png\"") || content.contains("SRC = \"./sub/b.png\""), "{content}");
}

/// `text` holds exactly the records `kinds` names, in order, and none of them
/// reports an issue.
#[track_caller]
fn assert_only_records(text: &str, kinds: &[ReferenceKind]) {
    let found: Vec<_> = extract_document_references(text).records.iter().map(|record| record.kind).collect();
    assert_eq!(found, kinds, "{text:?}");
    let (_dir, report) = validated(text);
    assert_eq!(report.issues.len(), 0, "{text:?}: {:#?}", report.issues);
    assert_eq!(report.references_scanned, kinds.len(), "{text:?}");
}

macro_rules! inert_text_rows {
    ($($name:ident: $text:expr => [$($kind:ident),*];)+) => {
        $(
            #[test]
            fn $name() {
                assert_only_records($text, &[$(ReferenceKind::$kind),*]);
            }
        )+
    };
}

inert_text_rows! {
    script_body_as_a_block: "<script><img src=\"./not-a-resource.png\"></script>\n" => [InlineScript];
    script_body_after_prose: "Note: <script><img src=\"./not-a-resource.png\"></script>\n" => [InlineScript];
    style_body_as_a_block: "<style><img src=\"./not-a-resource.png\"></style>\n" => [InlineCss];
    style_body_after_prose: "Note: <style><img src=\"./not-a-resource.png\"></style>\n" => [InlineCss];
    style_comment_as_a_block: "<style>/* <img src=\"./not-a-resource.png\"> */</style>\n" => [InlineCss];
    style_comment_after_prose: "Note: <style>/* <img src=\"./not-a-resource.png\"> */</style>\n" => [InlineCss];
    textarea_body_as_a_block: "<textarea><img src=\"./not-a-resource.png\"></textarea>\n" => [];
    textarea_body_after_prose: "Note: <textarea><img src=\"./not-a-resource.png\"></textarea>\n" => [];
    title_body_as_a_block: "<title><img src=\"./not-a-resource.png\"></title>\n" => [];
    title_body_after_prose: "Note: <title><img src=\"./not-a-resource.png\"></title>\n" => [];
    xmp_body_at_line_start: "<xmp><img src=\"./not-a-resource.png\"></xmp>\n" => [];
    xmp_body_after_prose: "Note: <xmp><img src=\"./not-a-resource.png\"></xmp>\n" => [];
    iframe_body_as_a_block: "<iframe><img src=\"./not-a-resource.png\"></iframe>\n" => [];
    iframe_body_after_prose: "Note: <iframe><img src=\"./not-a-resource.png\"></iframe>\n" => [];
    noembed_body_at_line_start: "<noembed><img src=\"./not-a-resource.png\"></noembed>\n" => [];
    noembed_body_after_prose: "Note: <noembed><img src=\"./not-a-resource.png\"></noembed>\n" => [];
    noframes_body_as_a_block: "<noframes><img src=\"./not-a-resource.png\"></noframes>\n" => [];
    noframes_body_after_prose: "Note: <noframes><img src=\"./not-a-resource.png\"></noframes>\n" => [];

    textarea_holding_an_anchor: "Note: <textarea><a href=\"./missing.md\">Example</a></textarea>\n" => [];
    textarea_holding_an_image: "Note: <textarea><img src=\"./missing.png\"></textarea>\n" => [];
    textarea_holding_a_script: "Note: <textarea><script src=\"./missing.js\"></script></textarea>\n" => [];
    textarea_holding_a_stylesheet: "Note: <textarea><link rel=\"stylesheet\" href=\"./missing.css\"></textarea>\n" => [];
    textarea_holding_a_video: "Note: <textarea><video src=\"./missing.mp4\"></video></textarea>\n" => [];
    textarea_holding_an_audio: "Note: <textarea><audio src=\"./missing.mp3\"></audio></textarea>\n" => [];
    textarea_holding_a_source: "Note: <textarea><source src=\"./missing.ogg\"></textarea>\n" => [];
    textarea_holding_an_iframe: "Note: <textarea><iframe src=\"./missing.html\"></iframe></textarea>\n" => [];
    textarea_holding_a_meta: "Note: <textarea><meta name=\"description\" content=\"example\"></textarea>\n" => [];
    textarea_holding_a_style_with_css_targets: "Note: <textarea><style>@import \"./missing.css\"; @font-face { src: url(\"./missing.woff\"); }</style></textarea>\n" => [];
    textarea_holding_a_markdown_link: "Note: <textarea>[example](./missing.md)</textarea>\n" => [];
    textarea_holding_a_markdown_image: "Note: <textarea>![example](./missing.png)</textarea>\n" => [];

    script_at_line_start_stays_clean: "<script>const label = '<a href=\"./draft.md\">Draft</a>';</script>\n" => [InlineScript];
    script_in_inline_code_stays_clean: "`<script>const label = '<a href=\"./draft.md\">Draft</a>';</script>`\n" => [];
    script_in_a_fence_stays_clean: "```html\n<script>const label = '<a href=\"./draft.md\">Draft</a>';</script>\n```\n" => [];

    opener_in_a_heading: "# Note <textarea>\n\n<img src=\"./gone.png\">\n\n</textarea>\n" => [];
    opener_in_a_table_cell: "| a |\n|---|\n| x <title><img src=\"./gone.png\"></title> |\n" => [];
    mixed_case_tags: "Note: <TEXTAREA><img src=\"./gone.png\"></TextArea >\n" => [];
    closer_in_inline_code_does_not_close: "Note: <textarea>`</textarea>`<img src=\"./gone.png\"></textarea>\n" => [];
    closer_in_a_fence_does_not_close: "Note: <textarea>\n\n```\n</textarea>\n```\n\n<img src=\"./gone.png\">\n\n</textarea>\n" => [];
    body_across_paragraphs: "Note: <textarea>\n\n[a](./a.md)\n\n<img src=\"./b.png\">\n\n![c](./c.png)\n\n</textarea>\n" => [];
    body_on_block_quote_lines: "> Note: <textarea>[a](./a.md)\n> <img src=\"./b.png\">\n> </textarea>\n" => [];
    crlf_body: "Note: <textarea>[a](./a.md)\r\n<img src=\"./b.png\">\r\n</textarea>\r\n" => [];
    tag_of_an_unclosed_block_element_stays_in_its_node: "<title><img src=\"./gone.png\">\n" => [];
}

missing_target_rows! {
    css_import_beside_a_comment_after_prose: "Note: <style>/* <img src=\"./gone.png\"> */ @import \"./gone.css\";</style>\n", "./gone.css", CssAtImport, (1, 43);
    css_import_on_a_block_quote_line_after_prose: "> Note: <style>\n> @import \"./gone.css\";\n> </style>\n", "./gone.css", CssAtImport, (2, 3);
    frame_target_after_prose_with_a_body: "Note: <iframe src=\"./gone.html\">[a](./a.md)</iframe>\n", "./gone.html", HtmlIframeTag, (1, 7);

    escaped_opener_leaves_a_later_tag_active: "Note: \\<textarea><img src=\"./gone.png\"></textarea>\n", "./gone.png", HtmlImage, (1, 18);
    opener_in_inline_code_leaves_a_later_tag_active: "`<textarea>` <img src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 14);
    closer_only_in_a_fence_leaves_later_text_active: "Note: <title>\n\n```\n</title>\n```\n\n[x](./gone.md)\n", "./gone.md", MarkdownLink, (7, 1);
    unclosed_opener_after_prose_leaves_a_later_link_active: "Use <title> here.\n\n[active](./gone.md)\n", "./gone.md", MarkdownLink, (3, 1);
    unclosed_opener_after_prose_leaves_a_later_tag_active: "Use <title> here <img src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 18);
    unclosed_block_opener_leaves_the_next_block_active: "<title><img src=\"./inert.png\">\n\n[active](./gone.md)\n", "./gone.md", MarkdownLink, (3, 1);
    link_that_starts_before_the_opener_stays: "[a <textarea> b](./gone.md) </textarea>\n", "./gone.md", MarkdownLink, (1, 1);

    tag_after_the_closer_in_its_block: "Note: <title>a\n\n<div></title><img src=\"./gone.png\"></div>\n", "./gone.png", HtmlImage, (3, 14);
    tag_before_the_closer_in_its_block_is_inert: "Note: <title>a\n\n<div><img src=\"./inert.png\"></title><img src=\"./gone.png\"></div>\n", "./gone.png", HtmlImage, (3, 37);
    tag_after_the_closer_on_a_later_line: "Note: <textarea>\n\n<div>\n</textarea><img\n src=\"./gone.png\">\n</div>\n", "./gone.png", HtmlImage, (4, 12);
    link_after_a_multi_paragraph_body: "Note: <textarea>\n\n[a](./a.md)\n\n</textarea>\n\n[d](./gone.md)\n", "./gone.md", MarkdownLink, (7, 1);
    tag_after_a_block_quote_body: "> Note: <textarea>[a](./a.md)\n> <img src=\"./b.png\">\n> </textarea>\n\n<img src=\"./gone.png\">\n", "./gone.png", HtmlImage, (5, 1);
    link_after_a_crlf_body: "Note: <textarea><img src=\"./a.png\">\r\n</textarea>\r\n[x](./gone.md)\r\n", "./gone.md", MarkdownLink, (3, 1);
    tag_after_two_raw_elements: "Note: <textarea>[a](./a.md)</textarea> and <title>[b](./b.md)</title> <img src=\"./gone.png\">\n", "./gone.png", HtmlImage, (1, 71);
}

#[test]
fn a_script_opened_after_prose_is_one_record_holding_its_whole_body() {
    let text = "Note: <script>const label = '<a href=\"./draft.md\">Draft</a>';</script>\n";

    let (_dir, report) = validated(text);

    assert_eq!(report.references_scanned, 1, "{report:#?}");
    assert_eq!(report.issues.len(), 0, "{:#?}", report.issues);
    let markdown = Markdown::new(text);
    let scripts = markdown.inline_scripts().unwrap();
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].script_content, "const label = '<a href=\"./draft.md\">Draft</a>';");
}

#[test]
fn a_closer_in_a_code_span_does_not_cut_a_raw_body_short() {
    let style = "Note: <style>`</style>` @import \"./gone.css\"; </style>\n";
    assert_one_missing(style, "./gone.css", ReferenceSyntax::CssAtImport, (1, 25));

    let scripts = Markdown::new("Note: <script>`</script>` tail();</script>\n").inline_scripts().unwrap();
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].script_content, "`</script>` tail();");
}

#[test]
fn a_tag_after_a_closer_on_a_long_body_line_stays_active_in_every_accessor() {
    let markdown = Markdown::new("Note: <textarea>\n\nlonger body text </textarea> <img src=\"./real.png\">\n");

    assert_eq!(markdown.inline_html_image_references().len(), 1);
}

#[test]
fn a_css_comments_tag_is_inert_while_its_import_stays_in_every_accessor() {
    let markdown = Markdown::new("Note: <style>/* <img src=\"./gone.png\"> */ @import \"./gone.css\";</style>\n");

    assert_eq!(markdown.css_imports().unwrap()[0].record.target.raw(), Some("./gone.css"));
    assert!(markdown.inline_html_image_references().is_empty());
}

#[test]
fn active_tags_after_prose_each_keep_their_own_position() {
    let (_dir, report) = validated("Text <a href=\"./gone.md\"><img src=\"./gone.png\"></a>\n");

    let found: Vec<_> = errors(&report)
        .iter()
        .map(|issue| {
            let authored = issue.origin.authored.as_ref().unwrap();
            (issue.reference_display.clone(), authored.line, authored.column)
        })
        .collect();
    assert_eq!(found, [("./gone.md".to_string(), 1, 6), ("./gone.png".to_string(), 1, 26)]);
}

#[test]
fn public_accessors_do_not_read_raw_text_as_references() {
    for text in [
        "Note: <textarea><a href=\"./a.md\">a</a><img src=\"./b.png\">[c](./c.md)![d](./d.png)<meta name=\"author\" content=\"Ken\"></textarea>\n",
        "<textarea><a href=\"./a.md\">a</a><img src=\"./b.png\">[c](./c.md)![d](./d.png)<meta name=\"author\" content=\"Ken\"></textarea>\n",
    ] {
        let markdown = Markdown::new(text);

        assert!(markdown.links().is_empty(), "{text:?}");
        assert!(markdown.image_references().is_empty(), "{text:?}");
        assert!(markdown.inline_html_links().is_empty(), "{text:?}");
        assert!(markdown.inline_html_image_references().is_empty(), "{text:?}");
        assert!(markdown.meta_tags().unwrap().is_empty(), "{text:?}");
    }
}

#[test]
fn public_accessors_keep_what_is_outside_raw_text() {
    let markdown = Markdown::new(
        "Note: <textarea>[c](./c.md)</textarea> [e](./e.md) ![f](./f.png) <a href=\"./g.md\">g</a> <img src=\"./h.png\">\n",
    );

    let hrefs: Vec<_> = markdown.links().iter().map(|link| link.href().to_string()).collect();
    let html_hrefs: Vec<_> = markdown.inline_html_links().iter().map(|link| link.href().to_string()).collect();
    assert_eq!(hrefs, ["./e.md"]);
    assert_eq!(markdown.image_references().len(), 1);
    assert_eq!(html_hrefs, ["./g.md"]);
    assert_eq!(markdown.inline_html_image_references().len(), 1);
}

#[test]
fn setting_a_meta_tag_leaves_one_in_raw_text_or_code_alone() {
    for text in [
        "Note: <textarea><meta name=\"author\" content=\"old\"></textarea>\n",
        "<textarea><meta name=\"author\" content=\"old\"></textarea>\n",
        "```html\n<meta name=\"author\" content=\"old\">\n```\n",
    ] {
        let mut markdown = Markdown::new(text);

        let replaced = markdown.set_meta_tag("author", "Ken");

        assert_eq!(replaced, 0, "{text:?}");
        assert!(markdown.content().starts_with(text), "{}", markdown.content());
        assert!(markdown.content().ends_with("<meta name=\"author\" content=\"Ken\">\n"), "{}", markdown.content());
    }
}

#[test]
fn a_file_directive_in_a_raw_body_is_still_a_composition_directive() {
    let (_dir, report) = validated("Note: <textarea>\n::file ./gone.md\n</textarea>\n");

    assert_eq!(errors(&report).len(), 1, "{:#?}", report.issues);
    assert_eq!(errors(&report)[0].reference_display, "./gone.md");
}

#[test]
fn composing_rewrites_active_targets_and_leaves_raw_text_bytes_alone() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "sub/a.png", "x");
    write(dir.path(), "sub/b.md", "x");
    write(dir.path(), "sub/c.js", "x");
    let inert = "<textarea><img src=\"./a.png\">[b](./b.md)</textarea>";
    write(
        dir.path(),
        "sub/child.md",
        &format!("Note: {inert} <img src=\"./a.png\"> [b](./b.md) <script src=\"./c.js\"></script>\n"),
    );
    let root = write(dir.path(), "root.md", "::file sub/child.md\n");
    let request = crate::request_support::request_at(dir.path(), ComposeOptions::new().with_source_file(&root));

    let (composed, _report) = Markdown::try_from(root.as_path()).unwrap().compose_with(&request).unwrap();

    let content = composed.content();
    assert!(content.contains(inert), "{content}");
    let active = content.split("</textarea>").nth(1).unwrap();
    assert!(!active.contains("\"./a.png\""), "{content}");
    assert!(active.contains("sub/a.png"), "{content}");
    assert!(active.contains("sub/b.md"), "{content}");
    assert!(active.contains("sub/c.js"), "{content}");
}
