use std::fs;
use std::path::Path;

use crate::harness::model::{FileFingerprint, PreRunSnapshot};

use super::{CheckResult, PostRunMarkdownState};

pub(crate) fn fingerprint_file(path: &Path) -> FileFingerprint {
    let exists = path.exists();
    let is_dir = path.is_dir();
    let blake3 = if exists && !is_dir {
        fs::read(path).ok().map(|bytes| {
            let hash_bytes = biscuit_hash::blake3_hash_bytes(&bytes);
            hash_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        })
    } else {
        None
    };
    FileFingerprint {
        exists,
        is_dir,
        blake3,
    }
}

pub(crate) fn check_file_changed(
    file: &Path,
    snapshot: Option<&PreRunSnapshot>,
    expect_changed: bool,
) -> CheckResult {
    let snapshot = snapshot.ok_or("internal error: no pre-run snapshot for file comparison")?;
    let pre = snapshot.tracked_files.get(file).ok_or_else(|| {
        format!(
            "internal error: file {} not tracked in snapshot",
            file.display()
        )
    })?;
    let post = fingerprint_file(file);

    let changed = pre.blake3 != post.blake3;
    if expect_changed {
        if changed {
            Ok(())
        } else {
            Err(format!("file {} was not modified", file.display()))
        }
    } else if changed {
        Err(format!("file {} was unexpectedly modified", file.display()))
    } else {
        Ok(())
    }
}

pub(crate) fn check_frontmatter_prop_changed(
    prop: &str,
    snapshot: Option<&PreRunSnapshot>,
    post_run_markdown: Option<&PostRunMarkdownState>,
    expect_changed: bool,
) -> CheckResult {
    let snapshot =
        snapshot.ok_or("internal error: no pre-run snapshot for frontmatter comparison")?;
    let pre_value = snapshot.tracked_frontmatter.get(prop);

    // Read the current on-disk post-state from the post-run markdown.
    let post_md = get_post_run_markdown(post_run_markdown, "frontmatter comparison")?;

    let post_value = post_md.fm_get::<serde_json::Value>(prop).ok().flatten();

    let changed = pre_value != post_value.as_ref();
    if expect_changed {
        if changed {
            Ok(())
        } else {
            Err(format!("frontmatter property \"{prop}\" was not modified"))
        }
    } else if changed {
        Err(format!(
            "frontmatter property \"{prop}\" was unexpectedly modified"
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn check_frontmatter_prop_equals(
    expected: &indexmap::IndexMap<String, serde_json::Value>,
    post_run_markdown: Option<&PostRunMarkdownState>,
) -> CheckResult {
    let post_md = get_post_run_markdown(post_run_markdown, "frontmatter equals check")?;

    let mut mismatches = Vec::new();
    for (key, expected_val) in expected {
        let actual = post_md.fm_get::<serde_json::Value>(key).ok().flatten();
        match actual {
            Some(ref actual_val) if actual_val == expected_val => {}
            Some(actual_val) => {
                mismatches.push(format!("{key}: expected {expected_val}, got {actual_val}"));
            }
            None => {
                mismatches.push(format!(
                    "{key}: expected {expected_val}, but property is missing"
                ));
            }
        }
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!("frontmatter mismatch: {}", mismatches.join("; ")))
    }
}

fn get_post_run_markdown<'a>(
    post_run_markdown: Option<&'a PostRunMarkdownState>,
    context: &str,
) -> Result<&'a darkmatter::markdown::Markdown, String> {
    match post_run_markdown {
        Some(PostRunMarkdownState::Loaded(markdown)) => Ok(markdown),
        Some(PostRunMarkdownState::ReadFailed { path, error }) => Err(format!(
            "failed to read {} for {context}: {error}",
            path.display()
        )),
        None => Err(format!(
            "internal error: post-run markdown not available for {context}"
        )),
    }
}
