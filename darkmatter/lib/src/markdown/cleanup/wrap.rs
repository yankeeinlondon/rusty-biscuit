//! `fixed-width`: re-wrapping paragraphs to a display width.
//!
//! Runs on the first stage's output, where `unwrap` has already joined every
//! joinable soft break, so each paragraph segment (the text between hard
//! breaks) sits on one line. Breaks go only at single ASCII spaces outside
//! code, HTML, math, link destinations, and expressions, never in front of a
//! word that would start a new block, and never where the space separates
//! spaceless-script text (the guard reads such a break as no separator).
//! Container prefixes count toward the width.

use std::ops::Range;

use biscuit_terminal::utils::UnicodeWidthStr;
use pulldown_cmark::{Event, LinkType, Tag, TagEnd};

use super::edits::SourceEdit;
use super::protect::{Lines, Protection, content_start};
use super::structure::{InlineRun, Model};
use super::transform::join_separator;

/// Plans the re-wrap edits for every paragraph.
pub(super) fn plan(
    source: &str,
    lines: &Lines,
    protection: &Protection,
    events: &[(Event<'_>, Range<usize>)],
    model: &Model,
    width: usize,
) -> Vec<SourceEdit> {
    model
        .runs
        .iter()
        .flat_map(|run| plan_run(source, lines, protection, events, run, width))
        .collect()
}

fn plan_run(
    source: &str,
    lines: &Lines,
    protection: &Protection,
    events: &[(Event<'_>, Range<usize>)],
    run: &InlineRun,
    width: usize,
) -> Vec<SourceEdit> {
    let run_events = &events[run.events.clone()];
    if run.range.is_empty() || run_events.iter().any(|(event, _)| matches!(event, Event::SoftBreak)) {
        return Vec::new();
    }
    let unbreakable = unbreakable_spans(run_events, protection, &run.range);
    if unbreakable.iter().any(|span| source[span.clone()].contains(['\n', '\r'])) {
        return Vec::new();
    }

    // Segments end at each hard break; the hard-break bytes stay as written.
    let mut segments: Vec<Range<usize>> = Vec::new();
    let mut start = run.range.start;
    for (index, (event, range)) in run_events.iter().enumerate() {
        if matches!(event, Event::HardBreak) {
            segments.push(start..range.start);
            start = run_events.get(index + 1).map_or(range.end, |(_, next)| {
                content_start(source, next.start, lines.get(lines.index_of(next.start)).start)
            });
        }
    }
    segments.push(start..run.range.end);

    let first_line = lines.index_of(run.range.start);
    let terminator = match lines.terminator(source, first_line) {
        "" if first_line > 0 => lines.terminator(source, first_line - 1),
        terminator => terminator,
    };
    let terminator = if terminator.is_empty() { "\n" } else { terminator };

    let mut edits = Vec::new();
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        let line = lines.get(lines.index_of(segment.start));
        if segment.end > line.end {
            return Vec::new();
        }
        let prefix = &source[line.start..segment.start];
        if !prefix.is_ascii() || prefix.contains('\t') {
            continue;
        }
        let continuation: String = prefix.chars().map(|c| if c == '>' { '>' } else { ' ' }).collect();
        if let Some(wrapped) = wrap_segment(source, &segment, &unbreakable, prefix, &continuation, terminator, width)
            && wrapped != source[segment.clone()]
        {
            edits.push(SourceEdit::replace(segment, wrapped));
        }
    }
    edits
}

/// Spans inside which a line break would change meaning.
fn unbreakable_spans(
    run_events: &[(Event<'_>, Range<usize>)],
    protection: &Protection,
    run_range: &Range<usize>,
) -> Vec<Range<usize>> {
    let mut spans: Vec<Range<usize>> = protection
        .inline
        .iter()
        .filter(|span| span.start < run_range.end && run_range.start < span.end)
        .cloned()
        .collect();
    let mut link_starts: Vec<(usize, bool)> = Vec::new();
    let mut last_inner_end: Vec<usize> = Vec::new();
    for (event, range) in run_events {
        match event {
            Event::Code(_)
            | Event::InlineHtml(_)
            | Event::Html(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_) => spans.push(range.clone()),
            Event::Start(Tag::Link { link_type, .. } | Tag::Image { link_type, .. }) => {
                let autolink = matches!(link_type, LinkType::Autolink | LinkType::Email);
                link_starts.push((range.start, autolink));
                last_inner_end.push(range.start);
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                let (start, autolink) = link_starts.pop().unwrap_or((range.start, true));
                let inner_end = last_inner_end.pop().unwrap_or(start);
                // The link text may wrap; its destination and title may not.
                spans.push(if autolink { start..range.end } else { inner_end..range.end });
            }
            _ => {}
        }
        if !matches!(event, Event::Start(Tag::Link { .. } | Tag::Image { .. }))
            && let Some(inner_end) = last_inner_end.last_mut()
        {
            *inner_end = (*inner_end).max(range.end);
        }
    }
    spans
}

/// Greedily re-wraps one single-line segment. `None` when nothing can move.
fn wrap_segment(
    source: &str,
    segment: &Range<usize>,
    unbreakable: &[Range<usize>],
    prefix: &str,
    continuation: &str,
    terminator: &str,
    width: usize,
) -> Option<String> {
    let text = &source[segment.clone()];
    // Words are separated by single spaces that may become line breaks.
    let mut words: Vec<&str> = Vec::new();
    let mut word_start = 0usize;
    let bytes = text.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b' ' {
            continue;
        }
        let absolute = segment.start + index;
        let single = (index == 0 || bytes[index - 1] != b' ') && bytes.get(index + 1).is_some_and(|next| *next != b' ');
        let inside = unbreakable.iter().any(|span| span.start <= absolute && absolute < span.end);
        if single && !inside && index > word_start {
            words.push(&text[word_start..index]);
            word_start = index + 1;
        }
    }
    words.push(&text[word_start..]);
    if words.len() < 2 && UnicodeWidthStr::width(text) + UnicodeWidthStr::width(prefix) <= width {
        return None;
    }

    let mut output = String::with_capacity(text.len() + 16);
    let mut line_width = UnicodeWidthStr::width(prefix);
    for (index, word) in words.iter().enumerate() {
        let word_width = UnicodeWidthStr::width(*word);
        if index == 0 {
            output.push_str(word);
            line_width += word_width;
            continue;
        }
        let breakable = !starts_block(word)
            && join_separator(words[index - 1].chars().next_back(), word.chars().next()) == " ";
        if line_width + 1 + word_width > width && breakable {
            output.push_str(terminator);
            output.push_str(continuation);
            output.push_str(word);
            line_width = UnicodeWidthStr::width(continuation) + word_width;
        } else {
            output.push(' ');
            output.push_str(word);
            line_width += 1 + word_width;
        }
    }
    Some(output)
}

/// Whether a line beginning with `word` would open a block instead of
/// continuing the paragraph.
fn starts_block(word: &str) -> bool {
    let first = word.chars().next().unwrap_or(' ');
    if matches!(word, "-" | "+" | "*") || word.starts_with(['>', '<', '#', '|']) || word.starts_with("::") {
        return true;
    }
    if word.starts_with("```") || word.starts_with("~~~") || word.starts_with("$$") || word.starts_with("[^") {
        return true;
    }
    if (first == '-' || first == '=' || first == '*' || first == '_') && word.chars().all(|c| c == first) {
        return true;
    }
    let digits = word.bytes().take_while(u8::is_ascii_digit).count();
    (1..=9).contains(&digits) && matches!(&word[digits..], "." | ")")
}
