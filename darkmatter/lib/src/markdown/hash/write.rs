//! Write-back paths for turning a [`SaveDecision`] into Markdown text.
//!
//! The decision layer ([`Markdown::plan_hash_save`]) chooses *what* to persist;
//! map-owning callers can use [`Markdown::apply_hash_save`], while callers that
//! own the source text use [`apply_hash_save_text`] to preserve every byte
//! outside the managed hash and `last_updated` nodes.

use super::options::{LAST_UPDATED_KEY, MdHashOptions};
use super::save::SaveDecision;
use crate::markdown::{Markdown, MarkdownError, MarkdownResult, extract_frontmatter_block};
use biscuit_file::serde_yaml_ng;
use std::ops::Range;

/// Applies a hash-save decision directly to authored Markdown source.
///
/// Only the complete top-level node named by [`MdHashOptions::property`] and,
/// when requested by the decision, the `last_updated` scalar are changed. The
/// document newline style and all other frontmatter and body bytes are retained.
/// A document without frontmatter gains a minimal frontmatter block.
///
/// ## Errors
///
/// Returns [`MarkdownError::FrontmatterTextEdit`] when the YAML root is not a
/// supported block mapping, a managed semantic key occurs more than once, or a
/// managed node cannot be replaced without ambiguity.
pub fn apply_hash_save_text(
    document_text: &str,
    decision: &SaveDecision,
    options: &MdHashOptions,
    today: &str,
) -> MarkdownResult<Option<String>> {
    let Some(new_stored) = decision.new_stored.as_ref() else {
        return Ok(None);
    };

    let newline = detect_newline(document_text);
    let Some(extraction) = extract_frontmatter_block(document_text)? else {
        let hash_entry = serialize_entry(
            &options.property,
            &new_stored.to_frontmatter_value(),
            newline,
        )?;
        let mut block = format!("---{newline}{hash_entry}");
        if decision.bump_last_updated {
            block.push_str(&format!("{LAST_UPDATED_KEY}: {today}{newline}"));
        }
        block.push_str("---");
        block.push_str(newline);
        block.push_str(document_text);
        return Ok(Some(block));
    };

    validate_block_mapping(extraction.yaml)?;
    let mut updated = document_text.to_string();
    let hash_node = locate_node(extraction.yaml, &options.property)?;
    let replacement = match hash_node.as_ref() {
        Some(node) => serialize_existing_entry(
            &extraction.yaml[node.range.clone()],
            node,
            &new_stored.to_frontmatter_value(),
            newline,
        )?,
        None => serialize_entry(
            &options.property,
            &new_stored.to_frontmatter_value(),
            newline,
        )?,
    };

    match hash_node {
        Some(node) => {
            let range = absolute_range(&extraction.yaml_span, node.range);
            updated.replace_range(range, &replacement);
        }
        None => updated.insert_str(extraction.yaml_span.end, &replacement),
    }

    if decision.bump_last_updated {
        let refreshed = extract_frontmatter_block(&updated)?.ok_or_else(|| text_edit_error(
            "frontmatter disappeared while applying the managed hash",
        ))?;
        let last_updated = locate_node(refreshed.yaml, LAST_UPDATED_KEY)?;
        match last_updated {
            Some(node) => {
                let node_text = &refreshed.yaml[node.range.clone()];
                let replacement = rewrite_date_scalar(node_text, &node, today, newline)?;
                let range = absolute_range(&refreshed.yaml_span, node.range);
                if updated[range.clone()] != replacement {
                    updated.replace_range(range, &replacement);
                }
            }
            None => updated.insert_str(
                refreshed.yaml_span.end,
                &format!("{LAST_UPDATED_KEY}: {today}{newline}"),
            ),
        }
    }

    Ok(Some(updated))
}

#[derive(Debug)]
struct TextNode {
    range: Range<usize>,
    key_end: usize,
    first_line_end: usize,
}

#[derive(Clone, Copy)]
struct LineSpan {
    start: usize,
    content_end: usize,
    end: usize,
}

fn text_edit_error(reason: impl Into<String>) -> MarkdownError {
    MarkdownError::FrontmatterTextEdit {
        reason: reason.into(),
    }
}

fn detect_newline(source: &str) -> &'static str {
    if source.as_bytes().windows(2).any(|pair| pair == b"\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn line_spans(source: &str) -> Vec<LineSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\n' || bytes[cursor] == b'\r' {
            let terminator = if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            spans.push(LineSpan {
                start,
                content_end: cursor,
                end: cursor + terminator,
            });
            cursor += terminator;
            start = cursor;
        } else {
            cursor += 1;
        }
    }
    if start < source.len() {
        spans.push(LineSpan {
            start,
            content_end: source.len(),
            end: source.len(),
        });
    }
    spans
}

fn validate_block_mapping(yaml: &str) -> MarkdownResult<()> {
    let first_content = yaml
        .lines()
        .map(str::trim_start)
        .find(|line| !line.is_empty() && !line.starts_with('#'));
    if first_content.is_some_and(|line| line.starts_with('{') || line.starts_with('[')) {
        return Err(text_edit_error(
            "flow-style frontmatter roots are not supported",
        ));
    }
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml) {
        Ok(serde_yaml_ng::Value::Mapping(_)) | Ok(serde_yaml_ng::Value::Null) => Ok(()),
        Ok(_) => Err(text_edit_error(
            "frontmatter must be a top-level block mapping",
        )),
        Err(error) => Err(text_edit_error(format!(
            "frontmatter YAML could not be parsed: {error}"
        ))),
    }
}

fn locate_node(yaml: &str, target: &str) -> MarkdownResult<Option<TextNode>> {
    let lines = line_spans(yaml);
    let mut nodes = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let content = &yaml[line.start..line.content_end];
        if content.is_empty()
            || content.chars().next().is_some_and(char::is_whitespace)
            || content.starts_with('#')
        {
            index += 1;
            continue;
        }

        let colon = mapping_colon(content).ok_or_else(|| {
            text_edit_error(format!("unsupported top-level YAML at byte {}", line.start))
        })?;
        let key_text = content[..colon].trim_end();
        let semantic = parse_semantic_key(key_text)?;
        let mut last_included_end = line.end;
        let mut cursor = index + 1;
        while cursor < lines.len() {
            let following = lines[cursor];
            let following_text = &yaml[following.start..following.content_end];
            let indentless_sequence = following_text == "-" || following_text.starts_with("- ");
            if !following_text.is_empty()
                && !following_text
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
                && !indentless_sequence
            {
                break;
            }
            if !following_text.is_empty() {
                last_included_end = following.end;
            }
            cursor += 1;
        }
        if semantic == target {
            nodes.push(TextNode {
                range: line.start..last_included_end,
                key_end: line.start + colon,
                first_line_end: line.end,
            });
        }
        index = cursor;
    }

    match nodes.len() {
        0 => Ok(None),
        1 => Ok(nodes.pop()),
        count => Err(text_edit_error(format!(
            "frontmatter contains {count} occurrences of semantic key `{target}`"
        ))),
    }
}

fn mapping_colon(line: &str) -> Option<usize> {
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if double && escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if double => escaped = true,
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            ':' if !single && !double => return Some(index),
            _ => {}
        }
    }
    None
}

fn parse_semantic_key(raw: &str) -> MarkdownResult<String> {
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw) {
        Ok(serde_yaml_ng::Value::String(key)) => Ok(key),
        Ok(_) => Err(text_edit_error("top-level frontmatter keys must be strings")),
        Err(error) => Err(text_edit_error(format!(
            "frontmatter key could not be parsed: {error}"
        ))),
    }
}

fn serialize_entry(
    key: &str,
    value: &serde_json::Value,
    newline: &str,
) -> MarkdownResult<String> {
    let mut map = indexmap::IndexMap::new();
    map.insert(key.to_string(), value);
    let serialized = serde_yaml_ng::to_string(&map)
        .map_err(|error| text_edit_error(format!("managed value could not be serialized: {error}")))?;
    Ok(serialized.replace('\n', newline))
}

fn serialize_existing_entry(
    node_text: &str,
    node: &TextNode,
    value: &serde_json::Value,
    newline: &str,
) -> MarkdownResult<String> {
    let key_end = node.key_end - node.range.start;
    let key_source = &node_text[..key_end];
    let canonical = serialize_entry("managed", value, newline)?;
    let colon = mapping_colon(canonical.lines().next().unwrap_or_default())
        .ok_or_else(|| text_edit_error("serialized managed value did not contain a mapping key"))?;
    Ok(format!("{key_source}{}", &canonical[colon..]))
}

fn rewrite_date_scalar(
    node_text: &str,
    node: &TextNode,
    today: &str,
    newline: &str,
) -> MarkdownResult<String> {
    if node.range.end > node.first_line_end {
        return Err(text_edit_error("`last_updated` must be a scalar value"));
    }
    let content_end = node_text
        .strip_suffix(newline)
        .map_or(node_text.len(), str::len);
    let line = &node_text[..content_end];
    let colon = node.key_end - node.range.start;
    let after_colon = &line[colon + 1..];
    let leading_len = after_colon.len() - after_colon.trim_start().len();
    let leading = &after_colon[..leading_len];
    let value_and_comment = &after_colon[leading_len..];
    let comment_start = yaml_comment_start(value_and_comment).unwrap_or(value_and_comment.len());
    let old_value = value_and_comment[..comment_start].trim_end();
    let comment_prefix = &value_and_comment[old_value.len()..];
    let rendered = match old_value.as_bytes().first() {
        Some(b'\'') => format!("'{today}'"),
        Some(b'"') => format!("\"{today}\""),
        _ => today.to_string(),
    };
    Ok(format!(
        "{}{leading}{rendered}{comment_prefix}{newline}",
        &line[..=colon]
    ))
}

fn yaml_comment_start(value: &str) -> Option<usize> {
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if double && escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if double => escaped = true,
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '#' if !single
                && !double
                && (index == 0
                    || value[..index]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_whitespace)) =>
            {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn absolute_range(parent: &Range<usize>, child: Range<usize>) -> Range<usize> {
    parent.start + child.start..parent.start + child.end
}

impl Markdown {
    /// Applies a [`SaveDecision`] and returns the serialized Markdown to write,
    /// or `None` when the decision requires no change.
    ///
    /// The active hash property is set from `options.property`, preserving its
    /// existing position in the frontmatter when present and appending it when
    /// new (`IndexMap` insertion semantics). When the decision bumps
    /// `last_updated`, that key is set to `today`, which the caller supplies as a
    /// `YYYY-MM-DD` string so the library stays deterministic and clock-free.
    ///
    /// The source document is never mutated; mutation happens on a clone.
    /// This API reserializes the complete frontmatter map. Callers for which
    /// the authored YAML text is authoritative must use
    /// [`apply_hash_save_text`] instead.
    ///
    /// ## Returns
    ///
    /// `Some(markdown)` to write, or `None` when [`SaveDecision::new_stored`] is
    /// `None` (nothing changed, so the file is left untouched).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::{Markdown, MdHashOptions};
    ///
    /// let doc: Markdown = "---\ntitle: T\n---\n# H\n\nBody.".into();
    /// let opts = MdHashOptions::default();
    /// let decision = doc.plan_hash_save(None, &opts).unwrap();
    /// let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();
    /// assert!(written.ends_with("# H\n\nBody."));
    /// ```
    pub fn apply_hash_save(
        &self,
        decision: &SaveDecision,
        options: &MdHashOptions,
        today: &str,
    ) -> Option<String> {
        let new_stored = decision.new_stored.as_ref()?;

        let mut updated = self.clone();
        let map = updated.frontmatter_mut().as_map_mut();
        // Direct map insertion keeps the serialized value identical to the
        // parsed `StoredHash` shape and preserves key position for an existing
        // hash property; a new property appends at the end.
        map.insert(options.property.clone(), new_stored.to_frontmatter_value());
        if decision.bump_last_updated {
            map.insert(
                LAST_UPDATED_KEY.to_string(),
                serde_json::Value::String(today.to_string()),
            );
        }
        Some(updated.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::hash::MdHashKind;

    fn md(content: &str) -> Markdown {
        content.into()
    }

    /// The serialized body must equal the original `content()` byte-for-byte,
    /// so the written document always ends with the untouched body.
    fn assert_body_preserved(original: &Markdown, serialized: &str) {
        assert!(
            serialized.ends_with(original.content()),
            "body not preserved verbatim.\noriginal body: {:?}\nserialized: {:?}",
            original.content(),
            serialized
        );
    }

    #[test]
    fn no_change_returns_none() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = StoredFromDoc::stored(&doc, MdHashKind::Simple, &opts);

        let decision = doc.plan_hash_save(Some(&stored), &opts).unwrap();
        assert!(decision.new_stored.is_none());
        assert!(doc.apply_hash_save(&decision, &opts, "2026-05-28").is_none());
    }

    #[test]
    fn first_baseline_adds_hash_and_preserves_body() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("hash:"));
        assert!(written.contains("title: T"));
        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn first_baseline_does_not_bump_last_updated() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(!written.contains("last_updated"));
    }

    #[test]
    fn document_without_frontmatter_gains_a_valid_block() {
        let doc = md("# H\n\nBody only, no frontmatter.\n");
        assert!(doc.frontmatter().is_empty());
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        // A fresh frontmatter block wraps the body, which is left untouched
        // after the closing delimiter.
        assert!(written.starts_with("---\n"));
        assert!(written.contains("\n---\n"));
        assert!(written.contains("hash:"));
        assert_body_preserved(&doc, &written);

        // The written document re-parses with a populated frontmatter block.
        let reparsed: Markdown = written.into();
        assert!(!reparsed.frontmatter().is_empty());
        assert!(reparsed.content().contains("Body only, no frontmatter."));
    }

    #[test]
    fn content_change_bumps_last_updated() {
        let original = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions::default();
        let stored = StoredFromDoc::stored(&original, MdHashKind::Simple, &opts);

        let edited = md("---\ntitle: T\n---\n# H\n\nNew body.");
        let decision = edited.plan_hash_save(Some(&stored), &opts).unwrap();
        let written = edited.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("last_updated: 2026-05-28"));
        assert_body_preserved(&edited, &written);
    }

    #[test]
    fn updating_existing_hash_preserves_key_order() {
        // `hash` sits in the middle: its position must be preserved on update.
        let opts = MdHashOptions::default();
        let stored = crate::markdown::hash::StoredHash::parse(
            &serde_json::json!("aaaa111111111111-bbbb222222222222"),
            "hash",
        )
        .unwrap();

        // Force a body change so a rewrite is required.
        let edited = md(
            "---\ntitle: T\nhash: aaaa111111111111-bbbb222222222222\nauthor: A\n---\n# H\n\nChanged body.",
        );
        let decision = edited.plan_hash_save(Some(&stored), &opts).unwrap();
        let written = edited.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        let reparsed: Markdown = written.into();
        let keys: Vec<&str> = reparsed
            .frontmatter()
            .as_map()
            .keys()
            .map(String::as_str)
            .collect();
        // Original three keys keep their relative order; `last_updated` is the
        // only newcomer and lands at the end.
        assert_eq!(keys, vec!["title", "hash", "author", "last_updated"]);
    }

    #[test]
    fn body_with_irregular_spacing_and_code_fences_is_byte_exact() {
        let body = "#   Heading\n\n\n   indented prose   \n\n```rust\nfn main() {\n    let x = 1;\n}\n```\n\n\ntrailing text\n\n\n";
        let source = format!("---\ntitle: T\n---\n{body}");
        let doc = md(&source);
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn body_without_trailing_newline_is_preserved() {
        let doc = md("---\ntitle: T\n---\n# H\n\nNo trailing newline.");
        assert!(!doc.content().ends_with('\n'));
        let opts = MdHashOptions::default();

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert_body_preserved(&doc, &written);
    }

    #[test]
    fn custom_property_name_is_used_for_write() {
        let doc = md("---\ntitle: T\n---\n# H\n\nBody.");
        let opts = MdHashOptions {
            property: "fingerprint".to_string(),
            ..MdHashOptions::default()
        };

        let decision = doc.plan_hash_save(None, &opts).unwrap();
        let written = doc.apply_hash_save(&decision, &opts, "2026-05-28").unwrap();

        assert!(written.contains("fingerprint:"));
        assert!(!written.contains("\nhash:"));
        assert_body_preserved(&doc, &written);
    }

    fn textual_decision(stored: crate::markdown::hash::StoredHash, bump: bool) -> SaveDecision {
        SaveDecision {
            kind: stored.kind,
            new_stored: Some(stored),
            bump_last_updated: bump,
            comparison: None,
        }
    }

    fn simple_stored(value: &str) -> crate::markdown::hash::StoredHash {
        crate::markdown::hash::StoredHash {
            kind: MdHashKind::Simple,
            value: crate::markdown::hash::StoredHashValue::Flat(value.to_string()),
            ignored: Vec::new(),
        }
    }

    #[test]
    fn textual_save_preserves_authored_lf_frontmatter_and_quote_style() {
        let source = concat!(
            "---\n",
            "title: Kept # authored\n",
            "prompt: |-\n",
            "    First line  \n",
            "\n",
            "    Literal \\\"quote\\\".\n",
            "'hash': old\n",
            "last_updated: '2026-01-01' # keep\n",
            "---\n",
            "# Body\n"
        );
        let decision = textual_decision(simple_stored("1111111111111111-2222222222222222"), true);
        let written = apply_hash_save_text(
            source,
            &decision,
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();

        assert!(written.contains("title: Kept # authored\n"));
        assert!(written.contains("prompt: |-\n    First line  \n\n    Literal \\\"quote\\\".\n"));
        assert!(written.contains("'hash': 1111111111111111-2222222222222222\n"));
        assert!(written.contains("last_updated: '2026-09-01' # keep\n"));
        assert!(written.ends_with("---\n# Body\n"));
    }

    #[test]
    fn textual_save_replaces_block_node_and_preserves_crlf() {
        let source = concat!(
            "---\r\n",
            "title: Kept\r\n",
            "hash:\r\n",
            "  kind: structured\r\n",
            "  value: old\r\n",
            "  # managed comment\r\n",
            "# boundary comment\r\n",
            "author: A\r\n",
            "---\r\n",
            "Body  \r\n"
        );
        let stored = crate::markdown::hash::StoredHash {
            kind: MdHashKind::Body,
            value: crate::markdown::hash::StoredHashValue::Flat("1111111111111111".to_string()),
            ignored: Vec::new(),
        };
        let written = apply_hash_save_text(
            source,
            &textual_decision(stored, false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();

        assert!(written.contains("hash:\r\n  kind: body\r\n  value: '1111111111111111'\r\n"));
        assert!(written.contains("# boundary comment\r\nauthor: A\r\n"));
        assert!(written.ends_with("---\r\nBody  \r\n"));
        assert!(!written.replace("\r\n", "").contains('\n'));
    }

    #[test]
    fn textual_save_adds_minimal_frontmatter_without_changing_body() {
        let source = "# Body\r\n\r\nKept.\r\n";
        let written = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();
        assert!(written.starts_with("---\r\nhash: 1111111111111111-2222222222222222\r\n---\r\n"));
        assert!(written.ends_with(source));
    }

    #[test]
    fn textual_save_rejects_duplicate_semantic_managed_keys() {
        let source = "---\nhash: old\n\"hash\": newer\n---\nBody\n";
        let err = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap_err();
        assert!(matches!(err, MarkdownError::FrontmatterTextEdit { .. }));
    }

    #[test]
    fn textual_save_rejects_flow_style_root() {
        let source = "---\n{title: Kept, hash: old}\n---\nBody\n";
        let err = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap_err();
        assert!(matches!(err, MarkdownError::FrontmatterTextEdit { .. }));
    }

    #[test]
    fn textual_no_change_does_not_parse_unsupported_source() {
        let decision = SaveDecision {
            kind: MdHashKind::Simple,
            new_stored: None,
            bump_last_updated: false,
            comparison: None,
        };
        assert!(
            apply_hash_save_text(
                "---\n{hash: duplicate, hash: ambiguous}\n---\nBody\n",
                &decision,
                &MdHashOptions::default(),
                "2026-09-01",
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn textual_save_treats_indentless_sequence_as_one_node() {
        let source = concat!(
            "---\n",
            "response_frontmatter:\n",
            "- routers\n",
            "- generated_by\n",
            "hash: old\n",
            "next: kept\n",
            "---\n",
            "Body\n"
        );
        let written = apply_hash_save_text(
            source,
            &textual_decision(simple_stored("1111111111111111-2222222222222222"), false),
            &MdHashOptions::default(),
            "2026-09-01",
        )
        .unwrap()
        .unwrap();

        assert!(written.contains(
            "response_frontmatter:\n- routers\n- generated_by\nhash: 1111111111111111-2222222222222222\nnext: kept\n"
        ));
    }

    #[test]
    fn textual_save_preservation_matrix_covers_representations_and_newlines() {
        struct Case {
            name: &'static str,
            kind: MdHashKind,
            property: &'static str,
            authored_key: &'static str,
            old_node: &'static str,
        }

        let cases = [
            Case {
                name: "simple",
                kind: MdHashKind::Simple,
                property: "hash",
                authored_key: "hash",
                old_node: "hash: 0000000000000000-0000000000000000",
            },
            Case {
                name: "structured",
                kind: MdHashKind::Structured,
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
                kind: MdHashKind::Detailed,
                property: "hash",
                authored_key: "hash",
                old_node: "hash: old",
            },
            Case {
                name: "custom-property",
                kind: MdHashKind::Simple,
                property: "fingerprint",
                authored_key: "fingerprint",
                old_node: "fingerprint: 0000000000000000-0000000000000000",
            },
            Case {
                name: "quoted-key",
                kind: MdHashKind::Simple,
                property: "hash",
                authored_key: "\"hash\"",
                old_node: "\"hash\": 0000000000000000-0000000000000000",
            },
        ];

        for newline in ["\n", "\r\n"] {
            for case in &cases {
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
                let options = MdHashOptions {
                    property: case.property.to_string(),
                    ..MdHashOptions::default()
                };
                let doc = md(&source);
                let stored = StoredFromDoc::stored(&doc, case.kind, &options);
                let written = apply_hash_save_text(
                    &source,
                    &textual_decision(stored, true),
                    &options,
                    "2026-09-01",
                )
                .unwrap()
                .unwrap();

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
                if matches!(case.kind, MdHashKind::Structured | MdHashKind::Detailed) {
                    assert!(
                        written.contains(&format!("kind: {}", case.kind)),
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
            }
        }
    }

    #[test]
    fn textual_save_flow_root_no_write_matrix_covers_newlines() {
        for newline in ["\n", "\r\n"] {
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
            let error = apply_hash_save_text(
                &source,
                &textual_decision(
                    simple_stored("1111111111111111-2222222222222222"),
                    false,
                ),
                &MdHashOptions::default(),
                "2026-09-01",
            )
            .unwrap_err();

            assert!(matches!(error, MarkdownError::FrontmatterTextEdit { .. }));
            assert_eq!(
                source,
                [
                    "---",
                    "{title: Kept, prompt: \"First line  ",
                    "  Second line\", hash: 0000000000000000-0000000000000000}",
                    "---",
                    "First body line.  ",
                    "Second body line.",
                    "",
                ]
                .join(newline)
            );
        }
    }

    /// Test helper: a stored hash computed from a document at a kind.
    struct StoredFromDoc;
    impl StoredFromDoc {
        fn stored(
            doc: &Markdown,
            kind: MdHashKind,
            opts: &MdHashOptions,
        ) -> crate::markdown::hash::StoredHash {
            crate::markdown::hash::StoredHash {
                kind,
                value: doc.compute_hash(kind, opts).to_stored_value(),
                ignored: Vec::new(),
            }
        }
    }
}
