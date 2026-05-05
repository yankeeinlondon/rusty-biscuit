use std::fs;
use std::path::PathBuf;

use biscuit_file::serde_yaml_ng;

use crate::error::Result;
use crate::linking::compatibility::{has_claude_specific_properties, parse_markdown_document};

/// Partition skills into shareable (no Claude-specific properties) and count of skipped.
///
/// Skills are directories; the check targets `SKILL.md` inside each directory.
pub(super) fn filter_unshareable_skills(
    skills: Vec<(String, PathBuf)>,
) -> (Vec<(String, PathBuf)>, usize) {
    let mut shareable = Vec::new();
    let mut skipped = 0;
    for entry in skills {
        let skill_md = entry.1.join("SKILL.md");
        if skill_md.exists() && has_claude_specific_properties(&skill_md) {
            skipped += 1;
        } else {
            shareable.push(entry);
        }
    }
    (shareable, skipped)
}

/// Fix a SKILL.md that is missing the `name` frontmatter property.
///
/// - If frontmatter exists but has no `name`, inserts it after the opening `---`.
/// - If no frontmatter exists, prepends a `---\nname: {topic}\n---\n` block.
///
/// Returns `true` if a fix was applied.
pub(super) fn fix_missing_name(topic: &str, skill_md: &PathBuf) -> Result<bool> {
    let content = match fs::read_to_string(skill_md) {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };
    let parsed = match parse_markdown_document(&content) {
        Ok(p) => p,
        Err(_) => return Ok(false),
    };

    let has_name = parsed
        .frontmatter
        .get(serde_yaml_ng::Value::String("name".to_string()))
        .map(|v| match v {
            serde_yaml_ng::Value::String(s) => !s.trim().is_empty(),
            _ => false,
        })
        .unwrap_or(false);

    if has_name {
        return Ok(false);
    }

    let new_content = if parsed.had_frontmatter {
        // Insert `name: {topic}` right after the opening `---`
        if let Some(rest) = content.strip_prefix("---\r\n") {
            format!("---\r\nname: {topic}\r\n{rest}")
        } else if let Some(rest) = content.strip_prefix("---\n") {
            format!("---\nname: {topic}\n{rest}")
        } else {
            return Ok(false);
        }
    } else {
        // No frontmatter — prepend a new block
        format!("---\nname: {topic}\n---\n\n{content}")
    };

    fs::write(skill_md, new_content)?;
    Ok(true)
}
