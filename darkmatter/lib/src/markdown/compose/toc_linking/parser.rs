//! Parser for `::toc-linking` directives.

use super::types::{
    CleanupService, HeadingGlob, TocLinkingDirective, TocLinkingError, TocLinkingOptions,
};
use crate::markdown::compose::parse_utils::{Cursor, find_code_regions, is_in_code_region};
use crate::markdown::normalize::HeadingLevel;

/// Parses all `::toc-linking` directives from markdown content.
pub fn parse_toc_linking_directives(
    content: &str,
) -> Result<Vec<TocLinkingDirective>, TocLinkingError> {
    let code_regions = find_code_regions(content);
    let mut directives = Vec::new();

    let bytes = content.as_bytes();
    let mut line_start = 0usize;
    let mut line_number = 1usize;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let line_end = i;
        let span_end = if i < bytes.len() { i + 1 } else { i };
        let line = &content[line_start..line_end];
        let trimmed = line.trim();

        if trimmed.starts_with("::toc-linking") {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            if !is_in_code_region(first_non_ws, &code_regions) {
                let indent = &content[line_start..first_non_ws];
                let inferred_indent = if indent.is_empty() {
                    infer_indent_from_previous_line(content, line_start)
                } else {
                    None
                };
                let directive = parse_directive_line(
                    trimmed,
                    line_start..span_end,
                    line_number,
                    indent.to_string(),
                    inferred_indent,
                )?;
                directives.push(directive);
            }
        }

        line_start = i.saturating_add(1);
        line_number += 1;
    }

    Ok(directives)
}

/// Scan backward from `pos` to find the first non-empty line and return its leading whitespace.
fn infer_indent_from_previous_line(content: &str, pos: usize) -> Option<String> {
    let before = &content[..pos];
    for line in before.lines().rev() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let leading_ws_len = line.len() - line.trim_start().len();
            if leading_ws_len > 0 {
                return Some(line[..leading_ws_len].to_string());
            } else {
                return None;
            }
        }
    }
    None
}

fn parse_directive_line(
    input: &str,
    span: std::ops::Range<usize>,
    line: usize,
    indent: String,
    inferred_indent: Option<String>,
) -> Result<TocLinkingDirective, TocLinkingError> {
    let mut cursor = Cursor::new(input);

    cursor.expect_literal("::toc-linking", line)?;
    cursor.skip_ws();

    // Read the target value — may be quoted (for pipe syntax) or bare
    let raw_target = cursor.read_value(line)?;
    if raw_target.is_empty() {
        return Err(TocLinkingError::ParseDirective {
            line,
            message: "Directive target cannot be empty".to_string(),
        });
    }

    // Parse target chain: split on `|`, trim, check for trailing `false`
    let (targets, suppress_not_found) = parse_target_chain(&raw_target, line)?;

    let mut options = TocLinkingOptions::default();

    // Parse key=value options
    while !cursor.is_eof() {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        let key = cursor.read_identifier(line)?;
        cursor.skip_ws();
        cursor.expect_char('=', line)?;
        cursor.skip_ws();
        let value = cursor.read_value(line)?;

        apply_toc_linking_option(&mut options, &key, &value, line)?;
    }

    Ok(TocLinkingDirective {
        targets,
        suppress_not_found,
        options,
        span,
        line,
        indent,
        inferred_indent,
    })
}

fn parse_target_chain(raw: &str, line: usize) -> Result<(Vec<String>, bool), TocLinkingError> {
    let parts: Vec<&str> = raw.split('|').map(|s| s.trim()).collect();
    let mut targets = Vec::new();
    let mut suppress = false;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if part.eq_ignore_ascii_case("false") {
            if i == parts.len() - 1 {
                suppress = true;
            } else {
                return Err(TocLinkingError::ParseDirective {
                    line,
                    message: "'false' can only appear as the last item in a fallback chain"
                        .to_string(),
                });
            }
        } else {
            targets.push(part.to_string());
        }
    }

    if targets.is_empty() && !suppress {
        return Err(TocLinkingError::ParseDirective {
            line,
            message: "No file targets specified".to_string(),
        });
    }

    Ok((targets, suppress))
}

fn apply_toc_linking_option(
    options: &mut TocLinkingOptions,
    key: &str,
    value: &str,
    line: usize,
) -> Result<(), TocLinkingError> {
    match key {
        "level" => {
            // Comma-separated h{N} values; also supports repeated keys
            for part in value.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let level = parse_heading_level(part, line)?;
                options.levels.levels.insert(level);
            }
        }
        "cleanup" => {
            // true = all, false = none, or comma-separated service names
            if value.eq_ignore_ascii_case("true") {
                options.cleanup_services = CleanupService::all();
            } else if value.eq_ignore_ascii_case("false") {
                options.cleanup_services.clear();
            } else {
                for part in value.split(',') {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }
                    let service = CleanupService::parse(part, line)?;
                    options.cleanup_services.push(service);
                }
            }
        }
        "filter" => {
            // NOT comma-separated; each filter= is a single glob
            let (pattern, case_sensitive) = parse_glob_prefix(value);
            options.filter_patterns.push(HeadingGlob {
                pattern: pattern.to_string(),
                case_sensitive,
            });
        }
        "keep" => {
            let (pattern, case_sensitive) = parse_glob_prefix(value);
            options.keep_patterns.push(HeadingGlob {
                pattern: pattern.to_string(),
                case_sensitive,
            });
        }
        "empty" => {
            options.empty_text = Some(value.to_string());
        }
        _ => {
            return Err(TocLinkingError::ParseDirective {
                line,
                message: format!("Unknown option '{}'", key),
            });
        }
    }

    Ok(())
}

fn parse_heading_level(s: &str, line: usize) -> Result<HeadingLevel, TocLinkingError> {
    let s = s.trim().to_ascii_lowercase();
    let num_str = s.strip_prefix('h').unwrap_or(&s);
    let level: u8 = num_str.parse().map_err(|_| TocLinkingError::InvalidLevel {
        level: s.clone(),
        line,
    })?;
    HeadingLevel::new(level).ok_or(TocLinkingError::InvalidLevel { level: s, line })
}

fn parse_glob_prefix(value: &str) -> (&str, bool) {
    if let Some(rest) = value.strip_prefix('^') {
        (rest, true)
    } else {
        (value, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_directive() {
        let content = "::toc-linking ./doc.md\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].targets, vec!["./doc.md"]);
        assert!(!directives[0].suppress_not_found);
    }

    #[test]
    fn parses_level_filter() {
        let content = "::toc-linking ./doc.md level=h2,h3\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        let levels = &directives[0].options.levels.levels;
        assert!(levels.contains(&HeadingLevel::H2));
        assert!(levels.contains(&HeadingLevel::H3));
        assert!(!levels.contains(&HeadingLevel::H4));
    }

    #[test]
    fn parses_repeated_levels() {
        let content = "::toc-linking ./doc.md level=h2 level=h4\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        let levels = &directives[0].options.levels.levels;
        assert!(levels.contains(&HeadingLevel::H2));
        assert!(levels.contains(&HeadingLevel::H4));
    }

    #[test]
    fn parses_cleanup_services() {
        let content = "::toc-linking ./doc.md cleanup=emoji,number\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        let services = &directives[0].options.cleanup_services;
        assert_eq!(services.len(), 2);
        assert_eq!(services[0], CleanupService::Emoji);
        assert_eq!(services[1], CleanupService::Number);
    }

    #[test]
    fn parses_cleanup_true() {
        let content = "::toc-linking ./doc.md cleanup=true\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(
            directives[0].options.cleanup_services.len(),
            CleanupService::all().len()
        );
    }

    #[test]
    fn parses_cleanup_false() {
        let content = "::toc-linking ./doc.md cleanup=false\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert!(directives[0].options.cleanup_services.is_empty());
    }

    #[test]
    fn parses_filter_and_keep() {
        let content = "::toc-linking ./doc.md filter=3.* keep=\"*important*\"\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives[0].options.filter_patterns.len(), 1);
        assert_eq!(directives[0].options.filter_patterns[0].pattern, "3.*");
        assert!(!directives[0].options.filter_patterns[0].case_sensitive);
        assert_eq!(directives[0].options.keep_patterns.len(), 1);
        assert_eq!(
            directives[0].options.keep_patterns[0].pattern,
            "*important*"
        );
    }

    #[test]
    fn parses_case_sensitive_glob() {
        let content = "::toc-linking ./doc.md filter=^Important*\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(
            directives[0].options.filter_patterns[0].pattern,
            "Important*"
        );
        assert!(directives[0].options.filter_patterns[0].case_sensitive);
    }

    #[test]
    fn parses_fallback_chain() {
        let content = "::toc-linking \"./a.md | ./b.md | ./c.md\"\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives[0].targets, vec!["./a.md", "./b.md", "./c.md"]);
        assert!(!directives[0].suppress_not_found);
    }

    #[test]
    fn parses_suppress_not_found() {
        let content = "::toc-linking \"./a.md | false\"\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives[0].targets, vec!["./a.md"]);
        assert!(directives[0].suppress_not_found);
    }

    #[test]
    fn parses_empty_text() {
        let content = "::toc-linking ./doc.md empty=\"no results\"\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(
            directives[0].options.empty_text,
            Some("no results".to_string())
        );
    }

    #[test]
    fn invalid_cleanup_service_errors() {
        let content = "::toc-linking ./doc.md cleanup=bogus\n";
        let err = parse_toc_linking_directives(content).unwrap_err();
        assert!(matches!(err, TocLinkingError::InvalidCleanupService { .. }));
    }

    #[test]
    fn invalid_level_errors() {
        let content = "::toc-linking ./doc.md level=h9\n";
        let err = parse_toc_linking_directives(content).unwrap_err();
        assert!(matches!(err, TocLinkingError::InvalidLevel { .. }));
    }

    #[test]
    fn skips_directives_inside_code_blocks() {
        let content = "```md\n::toc-linking ./ignored.md\n```\n::toc-linking ./included.md\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].targets, vec!["./included.md"]);
    }

    #[test]
    fn quoted_filename() {
        let content = "::toc-linking \"./my file.md\"\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives[0].targets, vec!["./my file.md"]);
    }

    #[test]
    fn unknown_option_errors() {
        let content = "::toc-linking ./doc.md bogus=value\n";
        let err = parse_toc_linking_directives(content).unwrap_err();
        assert!(matches!(err, TocLinkingError::ParseDirective { .. }));
    }

    #[test]
    fn captures_indent_from_leading_whitespace() {
        let content = "  ::toc-linking ./doc.md\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].indent, "  ");
    }

    #[test]
    fn captures_indent_from_tabs() {
        let content = "- Item\n\t::toc-linking ./doc.md\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].indent, "\t");
    }

    #[test]
    fn inferred_indent_from_previous_line() {
        let content = "  some text\n::toc-linking ./doc.md\n";
        let directives = parse_toc_linking_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].indent, "");
        assert_eq!(directives[0].inferred_indent, Some("  ".to_string()));
    }
}
