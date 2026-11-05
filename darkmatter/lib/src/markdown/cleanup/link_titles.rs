//! Link titles located by source span: the findings composition reports, and
//! the migration of legacy `key=value` titles that `md clean` runs.
//!
//! Both read one parse of the same view the body rules use, so a directive
//! line, a code fence, an inline code span, or an image title is never
//! mistaken for a link title, and every title reported as legacy is one the
//! migration rewrites. The exception is a title holding an expression: it is
//! not final until composition evaluates it, so it is neither reported from
//! authored text nor migrated.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Tag, TagEnd};
use renderable::markdown::code_span;
use renderable::output::link_metadata::{LinkTitleNotice, parse_link_title};

use super::body::Fallback;
use super::diagnostic::FallbackReason;
use super::edits::{self, SourceEdit};
use super::protect::{ParseView, Protection};
use super::structure::{self, Definition};
use crate::render::link::{is_encoded_metadata_title, legacy_structured_title};

const BYTE_ORDER_MARK: &str = "\u{FEFF}";

/// A link whose title raised a notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkTitleFinding {
    /// The whole link in the scanned text.
    pub span: Range<usize>,
    /// The title in the scanned text, delimiters included; for a
    /// reference-style link, in its definition.
    pub title: Option<Range<usize>>,
    /// The link as a code span, for a message: its text and destination.
    pub name: String,
    pub notice: LinkTitleNotice,
}

/// The links of `content` whose titles raise a notice, in document order. A
/// reference-style link is reported where it is used.
pub(crate) fn scan(content: &str) -> Vec<LinkTitleFinding> {
    let (bom, text) = split_bom(content);
    if !may_hold_structured_title(text) {
        return Vec::new();
    }
    // A malformed region fails composition elsewhere; its links are still links.
    let protection = Protection::scan(text).unwrap_or_default();
    let view = ParseView::new(text, &protection);
    let parsed = structure::parse(&view);
    let definitions: HashMap<String, &Definition> =
        parsed.definitions.iter().map(|definition| (folded_label(&definition.label), definition)).collect();

    let mut findings = Vec::new();
    let mut open: Vec<OpenLink<'_>> = Vec::new();
    for (event, range) in &parsed.events {
        match event {
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                let title_range = match reference_label(*link_type, id) {
                    Some(label) => definitions.get(&label).and_then(|definition| {
                        title_span(text, trimmed_end(text, definition.at..definition.end)?)
                    }),
                    None => inline_title_span(text, range),
                };
                let expression = title_range.as_ref().is_some_and(|span| protection.touches_inline(span));
                let notice = (!expression).then(|| title_notice(title)).flatten();
                open.push(OpenLink {
                    start: range.start,
                    end: range.end,
                    dest: dest_url,
                    title: title_range,
                    notice,
                    display: String::new(),
                });
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(link) = open.last_mut() {
                    link.display.push_str(text);
                }
            }
            Event::End(TagEnd::Link) => {
                let Some(link) = open.pop() else { continue };
                if let Some(notice) = link.notice {
                    findings.push(LinkTitleFinding {
                        span: link.start + bom..link.end + bom,
                        title: link.title.map(|title| title.start + bom..title.end + bom),
                        name: code_span(&format!("[{}]({})", link.display, link.dest)),
                        notice,
                    });
                }
            }
            _ => {}
        }
    }
    findings.sort_by_key(|finding| finding.span.start);
    findings
}

struct OpenLink<'e> {
    start: usize,
    end: usize,
    dest: &'e str,
    title: Option<Range<usize>>,
    notice: Option<LinkTitleNotice>,
    display: String,
}

/// A body with its legacy titles rewritten.
#[derive(Debug)]
pub(super) struct TitleMigration {
    pub text: String,
    /// Sorted by position: each rewritten range of the input and the length
    /// that replaced it.
    rewrites: Vec<(Range<usize>, usize)>,
}

impl TitleMigration {
    /// The offset in the input of `offset` in [`Self::text`]: the start of the
    /// rewrite when it falls inside one.
    pub(super) fn source_offset(&self, offset: usize) -> usize {
        let mut growth = 0isize;
        for (range, written) in &self.rewrites {
            let output_start = range.start.saturating_add_signed(growth);
            if offset < output_start {
                break;
            }
            if offset < output_start + written {
                return range.start;
            }
            growth += *written as isize - range.len() as isize;
        }
        offset.saturating_add_signed(-growth)
    }
}

/// Rewrites every legacy `key=value` title of a link in `body` as the JSON5
/// title that means what the old parser found, using the title's source span,
/// and proves nothing else changed. An image title keeps its own syntax, so a
/// reference definition an image also uses is left as authored.
///
/// ## Returns
///
/// `None` when there is nothing to rewrite.
///
/// ## Errors
///
/// A [`Fallback`] when the edit cannot be applied or its result does not parse
/// to the same document with those titles replaced.
pub(super) fn migrate(body: &str) -> Result<Option<TitleMigration>, Fallback> {
    let (bom, text) = split_bom(body);
    if !text.contains('=') || !may_hold_structured_title(text) {
        return Ok(None);
    }
    let Ok(protection) = Protection::scan(text) else {
        // `clean_body` reports the malformed region.
        return Ok(None);
    };
    let view = ParseView::new(text, &protection);
    let parsed = structure::parse(&view);

    let mut edits = Vec::new();
    let mut linked: HashSet<String> = HashSet::new();
    let mut imaged: HashSet<String> = HashSet::new();
    let mut table_depth = 0usize;
    for (event, range) in &parsed.events {
        match event {
            Event::Start(Tag::Table(_)) => table_depth += 1,
            Event::End(TagEnd::Table) => table_depth -= 1,
            Event::Start(Tag::Image { link_type, id, .. }) => {
                imaged.extend(reference_label(*link_type, id));
            }
            Event::Start(Tag::Link { link_type, id, title, .. }) => match reference_label(*link_type, id) {
                Some(label) => {
                    linked.insert(label);
                }
                None if is_legacy(title) => {
                    if let Some(span) = inline_title_span(text, range)
                        && !protection.touches_inline(&span)
                    {
                        let title = markdown_title(title);
                        let title = if table_depth > 0 { title.replace('|', "\\|") } else { title };
                        edits.push(SourceEdit::replace(span, title));
                    }
                }
                None => {}
            },
            _ => {}
        }
    }
    for definition in &parsed.definitions {
        let label = folded_label(&definition.label);
        if linked.contains(&label)
            && !imaged.contains(&label)
            && let Some(title) = definition.title.as_deref().filter(|title| is_legacy(title))
            && let Some(span) = trimmed_end(text, definition.at..definition.end).and_then(|end| title_span(text, end))
            && !protection.touches_inline(&span)
        {
            edits.push(SourceEdit::replace(span, markdown_title(title)));
        }
    }
    if edits.is_empty() {
        return Ok(None);
    }

    let mut ordered: Vec<(Range<usize>, usize)> =
        edits.iter().map(|edit| (edit.range.clone(), edit.replacement.len())).collect();
    ordered.sort_by_key(|(range, _)| range.start);
    let candidate = edits::apply(text, edits, &protection).map_err(|conflict| Fallback {
        reason: FallbackReason::SerializationFailure,
        offset: conflict.offset + bom,
        detail: conflict.message,
    })?;
    prove_only_titles_changed(&parsed, &candidate).map_err(|(offset, detail)| Fallback {
        reason: FallbackReason::SemanticMismatch,
        offset: offset + bom,
        detail,
    })?;
    Ok(Some(TitleMigration {
        text: format!("{}{candidate}", &body[..bom]),
        rewrites: ordered.into_iter().map(|(range, written)| (range.start + bom..range.end + bom, written)).collect(),
    }))
}

/// The migration's own guard: the candidate parses to the input's events and
/// definitions, except that a migrated title reads as its JSON5 form.
fn prove_only_titles_changed(before: &structure::Parsed<'_>, candidate: &str) -> Result<(), (usize, String)> {
    let protection = Protection::scan(candidate).map_err(|malformed| (malformed.offset, malformed.message))?;
    let view = ParseView::new(candidate, &protection);
    let after = structure::parse(&view);
    let same_title = |old: &str, new: &str| old == new || (is_legacy(old) && new == legacy_structured_title(old.trim()).to_json5());

    if before.events.len() != after.events.len() {
        return Err((0, "migrating a link title changed the document's structure".to_string()));
    }
    for ((old, range), (new, _)) in before.events.iter().zip(&after.events) {
        let same = match (old, new) {
            (
                Event::Start(Tag::Link { link_type: old_type, dest_url: old_dest, title: old_title, id: old_id }),
                Event::Start(Tag::Link { link_type: new_type, dest_url: new_dest, title: new_title, id: new_id }),
            ) => old_type == new_type && old_dest == new_dest && old_id == new_id && same_title(old_title, new_title),
            _ => old == new,
        };
        if !same {
            return Err((range.start, "migrating a link title changed more than the title".to_string()));
        }
    }
    let definitions_match = before.definitions.len() == after.definitions.len()
        && before.definitions.iter().zip(&after.definitions).all(|(old, new)| {
            old.label == new.label
                && old.destination == new.destination
                && match (&old.title, &new.title) {
                    (Some(old), Some(new)) => same_title(old, new),
                    (old, new) => old == new,
                }
        });
    if !definitions_match {
        return Err((0, "migrating a link title changed a reference definition".to_string()));
    }
    Ok(())
}

fn split_bom(text: &str) -> (usize, &str) {
    match text.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => (BYTE_ORDER_MARK.len(), rest),
        None => (0, text),
    }
}

/// Spares a parse for the common text no structured title can be in.
fn may_hold_structured_title(text: &str) -> bool {
    (text.contains('=') || text.contains('{')) && (text.contains("](") || text.contains("]:") || text.contains("]["))
}

fn title_notice(title: &str) -> Option<LinkTitleNotice> {
    let title = title.trim();
    if title.is_empty() || is_encoded_metadata_title(title) {
        return None;
    }
    parse_link_title(title).notice
}

fn is_legacy(title: &str) -> bool {
    title_notice(title) == Some(LinkTitleNotice::Legacy)
}

fn markdown_title(title: &str) -> String {
    legacy_structured_title(title.trim()).to_markdown_title()
}

/// The folded definition label of a reference-style link; `None` for an
/// inline link.
fn reference_label(link_type: LinkType, id: &str) -> Option<String> {
    match link_type {
        LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut => Some(folded_label(id)),
        _ => None,
    }
}

/// A label as definitions match it: case-insensitive, whitespace collapsed.
fn folded_label(label: &str) -> String {
    label.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// The title of the inline link occupying `link`, delimiters included.
fn inline_title_span(source: &str, link: &Range<usize>) -> Option<Range<usize>> {
    let inner = source.get(link.clone())?;
    if !inner.ends_with(')') {
        return None;
    }
    title_span(source, trimmed_end(source, link.start..link.end - 1)?)
}

/// `range.end` past any trailing whitespace of `range`.
fn trimmed_end(source: &str, range: Range<usize>) -> Option<Range<usize>> {
    let trimmed = source.get(range.clone())?.trim_end();
    Some(range.start..range.start + trimmed.len())
}

/// The title ending at the end of `range`, delimiters included. A title's own
/// delimiter is escaped inside it, so its opener is the nearest unescaped one.
fn title_span(source: &str, range: Range<usize>) -> Option<Range<usize>> {
    let bytes = source.as_bytes();
    let last = range.end.checked_sub(1).filter(|last| *last > range.start)?;
    let opener = match bytes[last] {
        quote @ (b'"' | b'\'') => quote,
        b')' => b'(',
        _ => return None,
    };
    (range.start..last)
        .rev()
        .find(|&at| bytes[at] == opener && bytes[..at].iter().rev().take_while(|byte| **byte == b'\\').count() % 2 == 0)
        .map(|at| at..range.end)
}
