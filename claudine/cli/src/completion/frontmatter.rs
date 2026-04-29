//! Mode-specific file validation for composition completion candidates.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. This is
//! the successor to the `2026-04-17-file-completion` feature's `validate.rs`
//! with a slightly narrower contract:
//!
//! - `compose` — markdown files (`.md` / `.markdown`) whose frontmatter does
//!   **not** carry a `prompt` key. A frontmatter-less file passes.
//! - `inline-compose` — markdown files whose frontmatter exposes a
//!   non-empty string `prompt` key.
//! - `sequence` — markdown files whose frontmatter has a `sequence` key,
//!   **plus** raw YAML files (`.yaml` / `.yml`) whose top-level document has
//!   a `sequence` key. Unlike `validate.rs` we do **not** resolve external
//!   references or require at least one step: during completion the file is
//!   a candidate so long as the key is present — the runtime composition
//!   pipeline is the authority on whether it actually runs.
//!
//! All validators are fail-closed: any I/O, UTF-8, parse, or size failure
//! becomes "not a candidate", never a shell-visible diagnostic. The shared
//! size gate defers to [`MAX_FRONTMATTER_BYTES`] so every size-sensitive
//! check shares one tunable constant.
//!
//! Key matching is case-sensitive — only lowercase `prompt` / `sequence`
//! keys are recognized, per spec §5.4.

use std::fs;
use std::path::Path;

use biscuit_file::serde_yaml_ng;
use darkmatter::markdown::Markdown;

use super::scopes::ComposeMode;

/// Maximum file size, in bytes, the frontmatter validators are willing to
/// read. Larger files are skipped without opening so completion latency is
/// bounded on generated-fixture files.
pub(crate) const MAX_FRONTMATTER_BYTES: u64 = 1024 * 1024;

/// Validate a candidate file against the rules for `mode`.
///
/// Returns `true` when the file should be emitted as a completion
/// candidate, `false` when it must be silently omitted.
pub(crate) fn valid_for_mode(path: &Path, mode: ComposeMode) -> bool {
    match mode {
        ComposeMode::Compose => is_valid_compose(path),
        ComposeMode::InlineCompose => is_valid_inline_compose(path),
        ComposeMode::Sequence => is_valid_sequence(path),
    }
}

/// `compose` accepts markdown files whose frontmatter does not carry a
/// `prompt` key. A frontmatter-less file passes — the composition runtime
/// treats a missing `prompt` the same as an absent one.
///
/// Every size/read/UTF-8 failure is a rejection so expensive or noisy
/// candidates never surface in compose output. This mirrors the behavior
/// of [`is_valid_inline_compose`] and [`is_valid_sequence`] for a single
/// uniform size/read contract (review-1 finding 6).
fn is_valid_compose(path: &Path) -> bool {
    if !has_markdown_extension(path) {
        return false;
    }
    let Some(text) = read_text_within_size_cap(path) else {
        // Oversized, unreadable, or non-UTF-8 files are rejected so
        // expensive or noisy candidates never surface in compose output.
        return false;
    };
    let markdown: Markdown = text.into();
    // Reject only when `prompt` is definitively present. A missing key is
    // the happy path for compose.
    !markdown.frontmatter().as_map().contains_key("prompt")
}

/// `inline-compose` requires a non-empty string `prompt` key in
/// frontmatter. Empty strings, whitespace-only strings, non-string types,
/// and missing keys are all rejected — completion should not surface files
/// that the runtime would refuse.
fn is_valid_inline_compose(path: &Path) -> bool {
    if !has_markdown_extension(path) {
        return false;
    }
    let Some(text) = read_text_within_size_cap(path) else {
        return false;
    };
    let markdown: Markdown = text.into();
    let Some(prompt_value) = markdown.frontmatter().as_map().get("prompt") else {
        return false;
    };
    match prompt_value {
        serde_json::Value::String(s) => !s.trim().is_empty(),
        _ => false,
    }
}

/// `sequence` accepts two extension families:
///
/// - `.md` / `.markdown` — the frontmatter must expose a `sequence` key.
/// - `.yaml` / `.yml` — the top-level YAML document must expose a
///   `sequence` key.
///
/// The value itself is not inspected (scalar list, object list, external
/// reference, scalar filename). Walking every shape would duplicate the
/// runtime resolver; completion is content to accept presence and defer the
/// full validation to runtime.
fn is_valid_sequence(path: &Path) -> bool {
    let Some(ext) = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
    else {
        return false;
    };
    let Some(text) = read_text_within_size_cap(path) else {
        return false;
    };
    match ext.as_str() {
        "md" | "markdown" => {
            let markdown: Markdown = text.into();
            markdown.frontmatter().as_map().contains_key("sequence")
        }
        "yaml" | "yml" => yaml_has_sequence_key(&text),
        _ => false,
    }
}

/// Return `true` when the YAML document's root mapping contains a
/// `sequence` key.
///
/// Parse errors are treated as "no sequence". The caller's contract —
/// completion must never raise to the shell — demands silent rejection.
fn yaml_has_sequence_key(text: &str) -> bool {
    let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
        return false;
    };
    match value {
        serde_yaml_ng::Value::Mapping(map) => {
            map.contains_key(serde_yaml_ng::Value::String("sequence".to_string()))
        }
        _ => false,
    }
}

/// Accept `.md` and `.markdown` extensions case-insensitively.
fn has_markdown_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

/// Read the file iff it is at most [`MAX_FRONTMATTER_BYTES`] bytes long and
/// valid UTF-8. Returns `None` for every failure mode so callers can treat
/// every error class (missing, too big, non-UTF-8, unreadable) identically.
fn read_text_within_size_cap(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    if metadata.len() > MAX_FRONTMATTER_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    // -- compose -----------------------------------------------------------

    #[test]
    fn compose_accepts_frontmatter_less_markdown() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("doc.md");
        write(&path, "# Hello\n\nBody.\n");
        assert!(valid_for_mode(&path, ComposeMode::Compose));
    }

    #[test]
    fn compose_accepts_markdown_without_prompt_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("doc.md");
        write(&path, "---\ntitle: Hi\n---\nBody\n");
        assert!(valid_for_mode(&path, ComposeMode::Compose));
    }

    #[test]
    fn compose_rejects_markdown_with_prompt_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("doc.md");
        write(&path, "---\nprompt: Say hi\n---\nBody\n");
        assert!(!valid_for_mode(&path, ComposeMode::Compose));
    }

    #[test]
    fn compose_rejects_non_markdown_extension() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("doc.txt");
        write(&path, "# Hello");
        assert!(!valid_for_mode(&path, ComposeMode::Compose));
    }

    #[test]
    fn compose_accepts_markdown_extension_case_insensitive() {
        assert!(has_markdown_extension(&PathBuf::from("a.MD")));
        assert!(has_markdown_extension(&PathBuf::from("a.Markdown")));
    }

    #[test]
    fn compose_rejects_oversized_markdown() {
        // Finding #6: oversized Markdown files must be rejected uniformly
        // across compose/inline-compose/sequence (size-guard contract).
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("huge.md");
        let padding = "a".repeat((MAX_FRONTMATTER_BYTES + 1) as usize);
        write(&path, &padding);
        assert!(!valid_for_mode(&path, ComposeMode::Compose));
    }

    #[test]
    fn compose_rejects_non_utf8_markdown() {
        // Finding #6: non-UTF-8 markdown fails `read_to_string` inside
        // `read_text_within_size_cap`, which must now reject rather than
        // accept. Write raw bytes so the test is portable.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.md");
        // Invalid UTF-8 byte sequence.
        fs::write(&path, [0xFF, 0xFE, 0x00, 0x00, 0xC0, 0xAF]).unwrap();
        assert!(!valid_for_mode(&path, ComposeMode::Compose));
    }

    #[test]
    fn compose_rejects_prompt_key_case_sensitively() {
        // Uppercase `Prompt:` is NOT the spec-defined key; the file passes
        // compose mode because the lowercase `prompt` is missing.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("doc.md");
        write(&path, "---\nPrompt: Say hi\n---\nBody\n");
        assert!(valid_for_mode(&path, ComposeMode::Compose));
    }

    // -- inline-compose ----------------------------------------------------

    #[test]
    fn inline_compose_accepts_non_empty_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inline.md");
        write(&path, "---\nprompt: Write a poem\n---\nbody\n");
        assert!(valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    #[test]
    fn inline_compose_rejects_empty_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.md");
        write(&path, "---\nprompt: \"\"\n---\nbody\n");
        assert!(!valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    #[test]
    fn inline_compose_rejects_whitespace_only_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ws.md");
        write(&path, "---\nprompt: \"   \\t  \"\n---\nbody\n");
        assert!(!valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    #[test]
    fn inline_compose_rejects_non_string_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("int.md");
        write(&path, "---\nprompt: 42\n---\nbody\n");
        assert!(!valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    #[test]
    fn inline_compose_rejects_missing_prompt() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nope.md");
        write(&path, "---\ntitle: Hi\n---\nbody\n");
        assert!(!valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    #[test]
    fn inline_compose_rejects_non_markdown_extension() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inline.txt");
        write(&path, "---\nprompt: hi\n---\nbody\n");
        assert!(!valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    #[test]
    fn inline_compose_rejects_oversized_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("big.md");
        let padding = "a".repeat((MAX_FRONTMATTER_BYTES + 1) as usize);
        let content = format!("---\nprompt: hi\n---\n{padding}");
        write(&path, &content);
        assert!(!valid_for_mode(&path, ComposeMode::InlineCompose));
    }

    // -- sequence ----------------------------------------------------------

    #[test]
    fn sequence_accepts_markdown_with_sequence_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.md");
        write(&path, "---\nsequence:\n  - one\n  - two\n---\nBody\n");
        assert!(valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_accepts_markdown_with_external_ref_string() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.md");
        // Points to a (nonexistent) file — completion accepts presence of
        // the key and defers real validation to runtime.
        write(&path, "---\nsequence: steps.yaml\n---\nBody\n");
        assert!(valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_accepts_yaml_with_top_level_sequence_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("steps.yaml");
        write(&path, "sequence:\n  - one\n  - two\n");
        assert!(valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_accepts_yml_with_top_level_sequence_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("steps.yml");
        write(&path, "sequence:\n  - one\n  - two\n");
        assert!(valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_rejects_yaml_without_sequence_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("steps.yaml");
        write(&path, "other:\n  - x\n");
        assert!(!valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_rejects_malformed_yaml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.yaml");
        write(&path, ": not valid :\n::\n");
        assert!(!valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_rejects_markdown_without_sequence_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("prose.md");
        write(&path, "---\ntitle: Hi\n---\nBody\n");
        assert!(!valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_rejects_case_variant_sequence_key() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.md");
        // Capital S — not the spec-defined key.
        write(&path, "---\nSequence:\n  - one\n---\nBody\n");
        assert!(!valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_rejects_non_md_non_yaml_extension() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("seq.txt");
        write(&path, "sequence:\n  - one\n");
        assert!(!valid_for_mode(&path, ComposeMode::Sequence));
    }

    #[test]
    fn sequence_rejects_oversized_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("big.md");
        let padding = "a".repeat((MAX_FRONTMATTER_BYTES + 1) as usize);
        let content = format!("---\nsequence:\n  - one\n---\n{padding}");
        write(&path, &content);
        assert!(!valid_for_mode(&path, ComposeMode::Sequence));
    }

    // -- shared gates ------------------------------------------------------

    #[test]
    fn extension_gate_accepts_md_and_markdown_case_insensitive() {
        assert!(has_markdown_extension(&PathBuf::from("a.md")));
        assert!(has_markdown_extension(&PathBuf::from("a.MD")));
        assert!(has_markdown_extension(&PathBuf::from("a.markdown")));
        assert!(has_markdown_extension(&PathBuf::from("a.Markdown")));
        assert!(!has_markdown_extension(&PathBuf::from("a.txt")));
        assert!(!has_markdown_extension(&PathBuf::from("a")));
        assert!(!has_markdown_extension(&PathBuf::from("a.yaml")));
    }

    #[test]
    fn size_gate_rejects_files_larger_than_cap() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("big.md");
        let content = "a".repeat((MAX_FRONTMATTER_BYTES + 1) as usize);
        write(&path, &content);
        assert!(read_text_within_size_cap(&path).is_none());
    }

    #[test]
    fn size_gate_accepts_files_at_or_below_cap() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ok.md");
        write(&path, "hello");
        assert!(read_text_within_size_cap(&path).is_some());
    }

    #[test]
    fn size_gate_rejects_missing_files() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.md");
        assert!(read_text_within_size_cap(&path).is_none());
    }
}
