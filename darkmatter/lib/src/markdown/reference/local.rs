//! Local extraction of markdown-native links and images with provenance.

use super::types::{
    ReferenceKind, ReferenceOrigin, ReferenceRecord, ReferenceSyntax, classify_target,
    make_reference_id,
};
use crate::markdown::compose::ComposeSource;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Extract Markdown-native links as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_markdown_links(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(content, options);

    let line_index = LineIndex::new(content);
    let mut records = Vec::new();
    let mut in_link = false;
    let mut current_href = String::new();
    let mut current_title = String::new();
    let mut current_display = String::new();
    let mut link_span_start: usize = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                in_link = true;
                current_href = dest_url.to_string();
                current_title = title.to_string();
                current_display.clear();
                link_span_start = range.start;
            }
            Event::End(TagEnd::Link) if in_link => {
                let display = std::mem::take(&mut current_display);
                let href = std::mem::take(&mut current_href);
                let title = std::mem::take(&mut current_title);
                let span = link_span_start..range.end;
                let line = line_index.line_at(link_span_start);

                let mut attributes = serde_json::Map::new();
                attributes.insert("display".into(), serde_json::Value::String(display));
                if !title.is_empty() {
                    attributes.insert("title".into(), serde_json::Value::String(title));
                }

                records.push(ReferenceRecord {
                    id: make_reference_id(source, line, span.start),
                    kind: ReferenceKind::Hyperlink,
                    target: classify_target(&href),
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line,
                        span,
                        syntax: ReferenceSyntax::MarkdownLink,
                    },
                    attributes,
                });

                in_link = false;
            }
            Event::Text(text) if in_link => {
                current_display.push_str(&text);
            }
            Event::Code(code) if in_link => {
                current_display.push('`');
                current_display.push_str(&code);
                current_display.push('`');
            }
            Event::SoftBreak if in_link => {
                current_display.push(' ');
            }
            Event::HardBreak if in_link => {
                current_display.push('\n');
            }
            _ => {}
        }
    }

    records
}

/// Extract Markdown-native images as [`ReferenceRecord`]s with provenance.
pub(crate) fn extract_markdown_images(
    content: &str,
    source: &ComposeSource,
) -> Vec<ReferenceRecord> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(content, options);

    let line_index = LineIndex::new(content);
    let mut records = Vec::new();
    let mut in_image = false;
    let mut current_src = String::new();
    let mut current_title = String::new();
    let mut current_alt = String::new();
    let mut image_span_start: usize = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                in_image = true;
                current_src = dest_url.to_string();
                current_title = title.to_string();
                current_alt.clear();
                image_span_start = range.start;
            }
            Event::End(TagEnd::Image) if in_image => {
                let alt = std::mem::take(&mut current_alt);
                let src = std::mem::take(&mut current_src);
                let title = std::mem::take(&mut current_title);
                let span = image_span_start..range.end;
                let line = line_index.line_at(image_span_start);

                let mut attributes = serde_json::Map::new();
                attributes.insert("alt".into(), serde_json::Value::String(alt));
                if !title.is_empty() {
                    attributes.insert("title".into(), serde_json::Value::String(title));
                }

                records.push(ReferenceRecord {
                    id: make_reference_id(source, line, span.start),
                    kind: ReferenceKind::Image,
                    target: classify_target(&src),
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line,
                        span,
                        syntax: ReferenceSyntax::MarkdownImage,
                    },
                    attributes,
                });

                in_image = false;
            }
            Event::Text(text) if in_image => {
                current_alt.push_str(&text);
            }
            Event::Code(code) if in_image => {
                current_alt.push('`');
                current_alt.push_str(&code);
                current_alt.push('`');
            }
            Event::SoftBreak if in_image => {
                current_alt.push(' ');
            }
            Event::HardBreak if in_image => {
                current_alt.push('\n');
            }
            _ => {}
        }
    }

    records
}

/// Line index for converting byte offsets to 1-based line numbers.
pub(crate) struct LineIndex {
    /// Byte offset of each line start.
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub(crate) fn new(content: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Returns the 1-based line number for a byte offset.
    pub(crate) fn line_at(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_markdown_link_local_path() {
        let content = "[my link](./file.md)";
        let records = extract_markdown_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::Hyperlink);
        assert!(
            matches!(&records[0].target, super::super::types::ReferenceTarget::LocalPath { raw } if raw == "./file.md")
        );
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::MarkdownLink);
        assert_eq!(records[0].origin.line, 1);
    }

    #[test]
    fn extract_markdown_link_remote_url() {
        let content = "[example](https://example.com)";
        let records = extract_markdown_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::RemoteUrl { .. }
        ));
    }

    #[test]
    fn extract_markdown_link_fragment() {
        let content = "[section](#section)";
        let records = extract_markdown_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::Fragment { .. }
        ));
    }

    #[test]
    fn extract_markdown_link_mailto() {
        let content = "[email](mailto:user@example.com)";
        let records = extract_markdown_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::OtherScheme { .. }
        ));
    }

    #[test]
    fn extract_markdown_image_local() {
        let content = "![photo](./img.png)";
        let records = extract_markdown_images(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, ReferenceKind::Image);
        assert!(
            matches!(&records[0].target, super::super::types::ReferenceTarget::LocalPath { raw } if raw == "./img.png")
        );
        assert_eq!(records[0].origin.syntax, ReferenceSyntax::MarkdownImage);
    }

    #[test]
    fn extract_markdown_image_data_uri() {
        let content = "![alt](data:image/png;base64,abc123)";
        let records = extract_markdown_images(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert!(matches!(
            &records[0].target,
            super::super::types::ReferenceTarget::DataUri { .. }
        ));
    }

    #[test]
    fn extract_display_text_preserved() {
        let content = "[click `here`](./doc.md)";
        let records = extract_markdown_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        let display = records[0]
            .attributes
            .get("display")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(display, "click `here`");
    }

    #[test]
    fn line_numbers_are_correct() {
        let content = "line 1\nline 2\n[link](./a.md)\nline 4";
        let records = extract_markdown_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].origin.line, 3);
    }

    #[test]
    fn multiple_links_extracted() {
        let content = "[a](./a.md) and [b](./b.md)";
        let records = extract_markdown_links(content, &ComposeSource::Unknown);
        assert_eq!(records.len(), 2);
    }
}
