//! Go module (`go.mod`) parsing and `go.work` workspace detection helpers.

use std::path::Path;

use crate::Result;
use crate::package::{DependencyEntry, DependencyKind};
use crate::performance;
use crate::performance::counters;

use super::detection::{DetectorOutcome, probe_exists};
use super::seed::{PackageSeed, merge_seeds};
use super::standard::{MonorepoStandard, PackageProvenance};

pub(super) fn detect_go_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    let go_work = root.join("go.work");
    if !probe_exists(&go_work) {
        return Ok(None);
    }

    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(&go_work)?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
    let uses = parse_go_work_uses(&content);
    if uses.is_empty() {
        return Ok(None);
    }

    let mut seeds = Vec::new();
    for member in uses {
        let member_path = root.join(&member);
        if !probe_exists(&member_path) {
            continue;
        }
        seeds.push(PackageSeed::new(
            &member_path,
            root,
            MonorepoStandard::GoWorkspace,
            PackageProvenance::Explicit,
        ));
    }

    if seeds.is_empty() {
        return Ok(None);
    }

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::GoWorkspace,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
        incomplete: false,
    }))
}

/// Parse the relative module paths from a `go.work` file's `use` directives.
///
/// Handles both the single-line form (`use ./svc-a`) and the parenthesized
/// block form (`use (\n\t./svc-a\n\t./svc-b\n)`). Leading `./` and surrounding
/// quotes are stripped so each entry is a clean repo-relative path.
pub(super) fn parse_go_work_uses(content: &str) -> Vec<String> {
    let mut uses = Vec::new();
    let mut in_block = false;

    for line in content.lines() {
        let trimmed = line.split("//").next().unwrap_or("").trim();
        if trimmed.is_empty() {
            continue;
        }

        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(path) = clean_use_path(trimmed) {
                uses.push(path);
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("use") {
            let rest = rest.trim();
            if let Some(after_paren) = rest.strip_prefix('(') {
                in_block = true;
                // Tolerate `use ( ./a` on the opening line, though gofmt splits it.
                if let Some(path) = clean_use_path(after_paren.trim()) {
                    uses.push(path);
                }
                continue;
            }
            if let Some(path) = clean_use_path(rest) {
                uses.push(path);
            }
        }
    }

    uses
}

/// Normalize a single `use` entry to a repo-relative path, or `None` when blank.
fn clean_use_path(raw: &str) -> Option<String> {
    let unquoted = raw.trim().trim_matches('"');
    let relative = unquoted.trim_start_matches("./").trim_end_matches('/');
    if relative.is_empty() {
        None
    } else {
        Some(relative.to_string())
    }
}

/// Parses go.mod `require` entries from cached file contents.
pub(super) fn go_mod_dependencies_from_content(content: &str) -> Option<Vec<DependencyEntry>> {
    let mut deps = Vec::new();
    let mut in_require_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if in_require_block {
            if trimmed.starts_with(')') {
                in_require_block = false;
                continue;
            }

            let line_without_comment = trimmed.split("//").next().unwrap_or(trimmed).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });

            continue;
        }

        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("require ") {
            let line_without_comment = rest.split("//").next().unwrap_or(rest).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });
        }
    }

    if deps.is_empty() { None } else { Some(deps) }
}

/// Read a unique module directive from a lexically well-formed go.mod.
///
/// This bounded reader recognizes directives and balanced blocks, but does not
/// validate dependency versions or resolve modules. Quoted tokens are decoded
/// before returning the declared identity; malformed documents have no name.
pub(super) fn go_module_name_from_content(content: &str) -> Option<String> {
    let mut module = None;
    let mut block = false;
    for line in content.lines() {
        let tokens = go_manifest_tokens(line)?;
        if tokens.is_empty() {
            continue;
        }
        if block {
            if tokens == [")"] {
                block = false;
            } else if tokens.iter().any(|token| token == "(" || token == ")") {
                return None;
            }
            continue;
        }
        let directive = tokens[0].as_str();
        if !matches!(directive, "module" | "go" | "toolchain" | "require" | "exclude" | "replace" | "retract" | "tool" | "godebug" | "ignore") {
            return None;
        }
        if tokens.len() == 2 && tokens[1] == "(" {
            if matches!(directive, "module" | "go" | "toolchain") {
                return None;
            }
            block = true;
            continue;
        }
        if tokens.len() < 2 || tokens.iter().any(|token| token == "(" || token == ")") {
            return None;
        }
        if matches!(directive, "module" | "go" | "toolchain") && tokens.len() != 2 {
            return None;
        }
        if directive == "module" {
            let name = &tokens[1];
            if module.is_some() || name.is_empty() || name.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '[' | ']' | '{' | '}' | '"' | '`' | '\\')) {
                return None;
            }
            module = Some(name.clone());
        }
    }
    if block { None } else { module }
}

fn go_manifest_tokens(line: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            break;
        }
        if ch == '"' || ch == '`' {
            let mut end = None;
            while let Some((index, next)) = chars.next() {
                if next == ch {
                    end = Some(index);
                    break;
                }
                if ch == '"' && next == '\\' {
                    chars.next()?;
                }
            }
            let end = end?;
            let value = if ch == '`' {
                line[start + 1..end].to_string()
            } else {
                go_quoted_token(&line[start + 1..end])?
            };
            tokens.push(value);
            if chars.peek().is_some_and(|(_, next)| !next.is_whitespace() && *next != '/') {
                return None;
            }
        } else {
            let mut end = line.len();
            while let Some((index, next)) = chars.peek() {
                if next.is_whitespace() || *next == '/' && line[*index..].starts_with("//") {
                    end = *index;
                    break;
                }
                if *next == '"' || *next == '`' {
                    return None;
                }
                chars.next();
            }
            tokens.push(line[start..end].to_string());
        }
    }
    Some(tokens)
}

fn go_quoted_token(value: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            let mut encoded = [0; 4];
            bytes.extend_from_slice(ch.encode_utf8(&mut encoded).as_bytes());
            continue;
        }
        let escape = chars.next()?;
        match escape {
            'a' => bytes.push(7), 'b' => bytes.push(8), 'f' => bytes.push(12),
            'n' => bytes.push(10), 'r' => bytes.push(13), 't' => bytes.push(9),
            'v' => bytes.push(11), '\\' => bytes.push(b'\\'), '"' => bytes.push(b'"'),
            'x' | 'u' | 'U' => {
                let digits = match escape { 'x' => 2, 'u' => 4, _ => 8 };
                let mut code = 0_u32;
                for _ in 0..digits {
                    code = code.checked_mul(16)?.checked_add(chars.next()?.to_digit(16)?)?;
                }
                if escape == 'x' {
                    bytes.push(u8::try_from(code).ok()?);
                } else {
                    let mut encoded = [0; 4];
                    bytes.extend_from_slice(char::from_u32(code)?.encode_utf8(&mut encoded).as_bytes());
                }
            }
            '0'..='7' => {
                let mut code = escape.to_digit(8)?;
                for _ in 0..2 { code = code * 8 + chars.next()?.to_digit(8)?; }
                bytes.push(u8::try_from(code).ok()?);
            }
            _ => return None,
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uses_reads_block_form() {
        let content = "go 1.21\n\nuse (\n\t./svc-a\n\t./svc-b\n)\n";
        assert_eq!(parse_go_work_uses(content), vec!["svc-a", "svc-b"]);
    }

    #[test]
    fn parse_uses_reads_single_line_form() {
        let content = "go 1.21\n\nuse ./only\n";
        assert_eq!(parse_go_work_uses(content), vec!["only"]);
    }

    #[test]
    fn parse_uses_empty_block_yields_nothing() {
        let content = "go 1.21\n\nuse (\n)\n";
        assert!(parse_go_work_uses(content).is_empty());
    }

    #[test]
    fn parse_uses_strips_comments_and_nested_paths() {
        let content = "use (\n\t./svc-a // primary\n\t./group/svc-b\n)\n";
        assert_eq!(parse_go_work_uses(content), vec!["svc-a", "group/svc-b"]);
    }
}
