use std::path::Path;

use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};

// -- Post-processing: Darkmatter cleanup ----------------------------------

/// Run Darkmatter's cleanup pass over a written inline composition file.
///
/// Reads the file, applies `cleanup_content` to the body (preserving
/// frontmatter), and writes back only if the content changed.
///
/// Returns `Ok(true)` when the file was updated, `Ok(false)` when no
/// changes were needed.
pub(crate) fn cleanup_inline_output(path: &Path) -> Result<bool> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| eyre!("failed to read {}: {e}", path.display()))?;

    // Split frontmatter from body so cleanup operates only on the body,
    // preserving frontmatter (including YAML block scalars) byte-for-byte.
    let (frontmatter_prefix, body) = split_frontmatter_and_body(&text);

    let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(body);

    if cleaned_body == body {
        return Ok(false);
    }

    let mut output = String::with_capacity(frontmatter_prefix.len() + cleaned_body.len());
    output.push_str(frontmatter_prefix);
    output.push_str(&cleaned_body);

    std::fs::write(path, output.as_bytes())
        .map_err(|e| eyre!("failed to write cleaned output to {}: {e}", path.display()))?;

    Ok(true)
}

/// Split text into a frontmatter prefix (including closing delimiter) and the body.
///
/// If the text starts with `---\n`, scans for the closing `---\n` and returns
/// everything up to and including that line as the prefix. Otherwise returns
/// an empty prefix and the full text as body.
pub(crate) fn split_frontmatter_and_body(text: &str) -> (&str, &str) {
    let mut lines = text.split_inclusive('\n');
    let first = match lines.next() {
        Some(l) => l,
        None => return ("", text),
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return ("", text);
    }

    let mut offset = first.len();
    for line in lines {
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return (&text[..offset], &text[offset..]);
        }
    }

    // No closing delimiter — treat entire text as body
    ("", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_basic() {
        let text = "---\ntitle: Test\n---\n# Body\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "---\ntitle: Test\n---\n");
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn split_frontmatter_block_scalar() {
        let text = concat!(
            "---\n",
            "prompt: |-\n",
            "    First line\n",
            "\n",
            "    - bullet\n",
            "last_updated: 2026-03-18\n",
            "---\n",
            "# Body\n",
        );
        let (prefix, body) = split_frontmatter_and_body(text);
        assert!(prefix.ends_with("---\n"));
        assert!(prefix.contains("prompt: |-"));
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn split_frontmatter_no_frontmatter() {
        let text = "# Just a heading\n\nContent\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "");
        assert_eq!(body, text);
    }

    #[test]
    fn split_frontmatter_unclosed() {
        let text = "---\ntitle: Test\nNo closing\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "");
        assert_eq!(body, text);
    }

    #[test]
    fn cleanup_preserves_frontmatter_block_scalar() {
        // Reproduces the bug: cleanup_content on full text corrupts YAML
        // block scalar indentation. The fix splits frontmatter from body
        // so cleanup only operates on the body.
        let frontmatter = concat!(
            "---\n",
            "prompt: |-\n",
            "    First line of prompt\n",
            "\n",
            "    - bullet one\n",
            "    - bullet two\n",
            "\n",
            "    Final paragraph\n",
            "last_updated: 2026-03-18\n",
            "---\n",
        );
        let body = "# Body\n\nSome content\n";
        let text = format!("{frontmatter}{body}");

        let (prefix, body_part) = split_frontmatter_and_body(&text);

        // Frontmatter must be preserved byte-for-byte
        assert_eq!(prefix, frontmatter);

        // Cleaning only the body should not corrupt frontmatter
        let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(body_part);
        let result = format!("{prefix}{cleaned_body}");

        // The frontmatter portion must remain unchanged
        assert!(result.starts_with(frontmatter));
    }

    #[test]
    fn cleanup_inline_output_rewrites_dirty_body_on_disk() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let file = dir.path().join("doc.md");

        let frontmatter = "---\nprompt: test\nlast_updated: 2026-03-18\n---\n";
        // "Dirty" body: no blank line between header and paragraph
        let dirty_body = "# Title\nParagraph text\n";
        let text = format!("{frontmatter}{dirty_body}");
        std::fs::write(&file, &text).unwrap();

        let changed = cleanup_inline_output(&file).unwrap();
        assert!(changed, "cleanup should report changes for dirty body");

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(
            on_disk.contains("# Title\n\nParagraph"),
            "cleanup must insert blank line between header and paragraph; got:\n{on_disk}"
        );
        assert!(
            on_disk.starts_with(frontmatter),
            "frontmatter must be preserved byte-for-byte"
        );
    }

    #[test]
    fn cleanup_inline_output_returns_false_for_clean_body() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let file = dir.path().join("doc.md");

        let frontmatter = "---\nprompt: test\nlast_updated: 2026-03-18\n---\n";
        // Already-clean body
        let clean_body = "# Title\n\nParagraph text\n";
        let text = format!("{frontmatter}{clean_body}");
        std::fs::write(&file, &text).unwrap();

        let changed = cleanup_inline_output(&file).unwrap();
        assert!(
            !changed,
            "cleanup should report no changes for already-clean body"
        );

        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert_eq!(on_disk, text, "file must be unchanged");
    }
}
