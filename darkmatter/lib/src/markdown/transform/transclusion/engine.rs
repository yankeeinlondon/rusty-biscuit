//! Engine helpers for transclusion insertion and heading handling.

use crate::markdown::normalize::HeadingLevel;
use crate::markdown::transform::TransformWarning;
use pulldown_cmark::{Event, HeadingLevel as PulldownHeadingLevel, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
struct HeadingInfo {
    level: HeadingLevel,
    title: String,
    line: usize,
    start: usize,
    end: usize,
}

/// Finds the nearest preceding heading level before a byte offset.
pub fn find_preceding_heading_level(content: &str, offset: usize) -> Option<HeadingLevel> {
    let mut current = None;

    for (event, range) in Parser::new(content).into_offset_iter() {
        if range.start >= offset {
            break;
        }

        if let Event::Start(Tag::Heading { level, .. }) = event {
            current = Some(pulldown_to_heading_level(level));
        }
    }

    current
}

/// Re-levels markdown content and gracefully degrades H6 overflow to bold text.
pub fn relevel_with_overflow(
    content: &str,
    target: HeadingLevel,
) -> (String, Vec<TransformWarning>) {
    let headings = extract_headings(content);
    if headings.is_empty() {
        return (content.to_string(), Vec::new());
    }

    let root = headings[0].level;
    let adjustment = target.as_u8() as i8 - root.as_u8() as i8;
    if adjustment == 0 {
        return (content.to_string(), Vec::new());
    }

    #[derive(Debug, Clone)]
    enum Replacement {
        Prefix {
            start: usize,
            old_level: HeadingLevel,
            new_level: HeadingLevel,
        },
        Overflow {
            start: usize,
            end: usize,
            title: String,
            line: usize,
            new_level_raw: u8,
        },
    }

    let mut replacements = Vec::new();
    let mut warnings = Vec::new();

    for heading in &headings {
        let new_level_raw = heading.level.as_u8() as i8 + adjustment;
        if (1..=6).contains(&new_level_raw) {
            if let Some(level) = HeadingLevel::new(new_level_raw as u8) {
                replacements.push(Replacement::Prefix {
                    start: heading.start,
                    old_level: heading.level,
                    new_level: level,
                });
            }
        } else {
            replacements.push(Replacement::Overflow {
                start: heading.start,
                end: heading.end,
                title: heading.title.clone(),
                line: heading.line,
                new_level_raw: new_level_raw.max(7) as u8,
            });
        }
    }

    replacements.sort_by(|left, right| {
        let left_start = match left {
            Replacement::Prefix { start, .. } | Replacement::Overflow { start, .. } => *start,
        };
        let right_start = match right {
            Replacement::Prefix { start, .. } | Replacement::Overflow { start, .. } => *start,
        };
        right_start.cmp(&left_start)
    });

    let mut result = content.to_string();

    for replacement in replacements {
        match replacement {
            Replacement::Prefix {
                start,
                old_level,
                new_level,
            } => {
                let prefix_end = start + old_level.hash_count();
                let replacement = "#".repeat(new_level.hash_count());
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    replacement,
                    &result[prefix_end..]
                );
            }
            Replacement::Overflow {
                start,
                end,
                title,
                line,
                new_level_raw,
            } => {
                let bold_block = format!("\n\n**{}**\n\n", title.trim());
                result = format!("{}{}{}", &result[..start], bold_block, &result[end..]);
                warnings.push(
                    TransformWarning::new(
                        "transclusion",
                        format!(
                            "Heading overflow at line {line}: converted to bold text (would become H{new_level_raw})"
                        ),
                    )
                    .at_line(line),
                );
            }
        }
    }

    (result, warnings)
}

fn extract_headings(content: &str) -> Vec<HeadingInfo> {
    let mut headings = Vec::new();
    let mut current: Option<(HeadingLevel, String, usize)> = None;

    for (event, range) in Parser::new(content).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((pulldown_to_heading_level(level), String::new(), range.start));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, title, _)) = current.as_mut() {
                    title.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, title, start)) = current.take() {
                    let line = content[..start].lines().count() + 1;
                    headings.push(HeadingInfo {
                        level,
                        title,
                        line,
                        start,
                        end: range.end,
                    });
                }
            }
            _ => {}
        }
    }

    headings
}

fn pulldown_to_heading_level(level: PulldownHeadingLevel) -> HeadingLevel {
    match level {
        PulldownHeadingLevel::H1 => HeadingLevel::H1,
        PulldownHeadingLevel::H2 => HeadingLevel::H2,
        PulldownHeadingLevel::H3 => HeadingLevel::H3,
        PulldownHeadingLevel::H4 => HeadingLevel::H4,
        PulldownHeadingLevel::H5 => HeadingLevel::H5,
        PulldownHeadingLevel::H6 => HeadingLevel::H6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_preceding_heading_level() {
        let content = "# Root\n\nText\n\n## Child\n\n::file ./x.md\n";
        let offset = content.find("::file").unwrap();
        assert_eq!(
            find_preceding_heading_level(content, offset),
            Some(HeadingLevel::H2)
        );
    }

    #[test]
    fn overflow_headings_become_bold() {
        let content = "## Section\n\n### Deep\n";
        let (new_content, warnings) = relevel_with_overflow(content, HeadingLevel::H6);

        assert!(new_content.contains("###### Section"));
        assert!(new_content.contains("**Deep**"));
        assert_eq!(warnings.len(), 1);
    }
}
