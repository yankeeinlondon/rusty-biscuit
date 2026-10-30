//! Source guard for the schema-trigger renames
//! (`2026-10-27-darkmatter-lifecycle-ownership`, criterion 16): no active
//! trigger file, example, test, completion, or document spells
//! `kind: trigger-schema`, `--no-trigger-schemas`, or the `any:` combinator.
//!
//! The exceptions are explicit: the code and diagnostics that reject a
//! renamed spelling, the negative fixtures that prove the rejection, and the
//! documents that describe the rename. Completed and historical material
//! (`_completed/`, `features/`, `fixes/`, `research/`) is never scanned.
//! `trigger-schemas` (the plural concept, as in the `#repository-trigger-schemas`
//! anchor) is not a renamed spelling.

use std::path::{Path, PathBuf};

use biscuit_test_harness::manifest_dir;

/// Files allowed to spell a renamed form, relative to the repository root.
const ALLOWED: &[&str] = &[
    // The rejecting code and its diagnostics.
    "darkmatter/lib/src/markdown/schemas/triggers/envelope.rs",
    "darkmatter/lib/src/markdown/schemas/errors.rs",
    "darkmatter/cli/src/main.rs",
    // Negative fixtures and the tests that prove the rejection.
    "darkmatter/lib/tests/l1/trigger_envelope_claim.rs",
    "darkmatter/lib/tests/l1/trigger_input_matrix.rs",
    "darkmatter/lib/tests/l1/trigger_match_grammar.rs",
    "darkmatter/lib/tests/l1/trigger_selection_parity.rs",
    "darkmatter/lib/tests/l1/trigger_spelling_guard.rs",
    "darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs",
    "darkmatter/cli/tests/l1/schema_trigger_rename.rs",
    "darkmatter/dmls/tests/l1/workspace_schema_scopes.rs",
    "claudine/lib/tests/l1/claudine_trigger_selection_parity.rs",
    // Documents that describe the rename.
    "darkmatter/docs/topics/schemas/definition.md",
    ".claude/skills/darkmatter/schema.md",
];

/// Folder names never scanned: completed and historical material, and
/// build output.
const SKIPPED_FOLDERS: &[&str] = &["_completed", "features", "fixes", "research", "target", "node_modules"];

/// The extensions that can hold a trigger, an example, or a completion.
const EXTENSIONS: &[&str] = &["rs", "md", "yaml", "yml", "toml", "sh", "fish", "zsh"];

fn collect(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        panic!("{} must be readable", folder.display());
    };
    for entry in entries {
        let path = entry.expect("folder entry").path();
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
        let file_type = std::fs::symlink_metadata(&path).expect("entry metadata").file_type();
        if file_type.is_dir() {
            if !SKIPPED_FOLDERS.contains(&name.as_str()) {
                collect(&path, out);
            }
        } else if file_type.is_file()
            && path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| EXTENSIONS.contains(&ext))
            && !name.ends_with("_research.md")
        {
            out.push(path);
        }
    }
}

/// Each renamed spelling found on `line`.
fn renamed_spellings(line: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("trigger-schema") {
        let after = &rest[at + "trigger-schema".len()..];
        if !after.starts_with('s') {
            found.push("trigger-schema");
            break;
        }
        rest = after;
    }
    if line.contains("--no-trigger-schemas") {
        found.push("--no-trigger-schemas");
    }
    // `any:` as a YAML key: a block line (`any:` or `- any:`) or inside an
    // escaped string (`"match:\n  any:\n"`).
    let trimmed = line.trim_start().trim_start_matches("- ");
    if trimmed.starts_with("any:") && trimmed["any:".len()..].trim().is_empty_or_comment()
        || line.contains("any:\\n")
        || line.contains("{any:")
        || line.contains("\"any\":")
    {
        found.push("any:");
    }
    found
}

trait EmptyOrComment {
    fn is_empty_or_comment(&self) -> bool;
}

impl EmptyOrComment for str {
    fn is_empty_or_comment(&self) -> bool {
        self.is_empty() || self.starts_with('#')
    }
}

#[test]
fn no_active_file_spells_a_renamed_trigger_form() {
    let root = manifest_dir!().join("../..");
    let folders = [
        manifest_dir!().join("src"),
        manifest_dir!().join("tests"),
        manifest_dir!().join("../cli/src"),
        manifest_dir!().join("../cli/tests"),
        manifest_dir!().join("../dmls/src"),
        manifest_dir!().join("../dmls/tests"),
        manifest_dir!().join("../docs"),
        manifest_dir!().join("../tests/fixtures"),
        manifest_dir!().join("../example-docs"),
        manifest_dir!().join("../examples"),
        manifest_dir!().join("../schemas"),
        manifest_dir!().join("../../claudine/schemas"),
        manifest_dir!().join("../../claudine/docs/topics"),
        manifest_dir!().join("../../claudine/lib/src"),
        manifest_dir!().join("../../claudine/lib/tests"),
        manifest_dir!().join("../../claudine/cli/src"),
        manifest_dir!().join("../../claudine/cli/tests"),
        manifest_dir!().join("../../content-policy/docs"),
        manifest_dir!().join("../../.claude/skills/darkmatter"),
        manifest_dir!().join("../../.claude/skills/claudine"),
        manifest_dir!().join("../../schemas"),
    ];
    let mut files = vec![
        manifest_dir!().join("../README.md"),
        manifest_dir!().join("../cli/README.md"),
        manifest_dir!().join("../../claudine/schemas/README.md"),
    ];
    for folder in &folders {
        collect(folder, &mut files);
    }
    assert!(files.len() > 100, "the scan found {} files", files.len());

    let mut offenders = Vec::new();
    let mut allowed_hits = std::collections::BTreeSet::new();
    for file in &files {
        let relative = std::fs::canonicalize(file)
            .expect("canonical file")
            .strip_prefix(std::fs::canonicalize(&root).expect("canonical root"))
            .expect("inside the repository")
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            for spelling in renamed_spellings(line) {
                if ALLOWED.contains(&relative.as_str()) {
                    allowed_hits.insert(relative.clone());
                } else {
                    offenders.push(format!("{relative}:{}: `{spelling}`: {}", index + 1, line.trim()));
                }
            }
        }
    }
    assert!(offenders.is_empty(), "renamed trigger spellings:\n{}", offenders.join("\n"));
    // An allowlist entry that no longer names a renamed form is stale.
    let stale: Vec<&&str> = ALLOWED.iter().filter(|allowed| !allowed_hits.contains(**allowed)).collect();
    assert!(stale.is_empty(), "allowlist entries with no renamed spelling: {stale:?}");
}

#[test]
fn the_guard_recognizes_each_renamed_spelling_and_nothing_else() {
    for line in [
        "kind: trigger-schema",
        "    \"kind: trigger-schema\\nmatch:\\n\"",
        "md compose --no-trigger-schemas doc.md",
        "    any:",
        "  - any:   # alternatives",
        "\"match:\\n  any:\\n    - a: string(required)\"",
        "match: {any: [{a: string(required)}]}",
        "{\"any\": []}",
    ] {
        assert!(!renamed_spellings(line).is_empty(), "{line}");
    }
    for line in [
        "kind: schema-trigger",
        "[Repository Trigger Schemas](#repository-trigger-schemas)",
        "launch-trigger-schemas-frozen",
        "--no-schema-triggers",
        "    group:",
        "steps: any(required)",
        "anything: string",
        "use std::any::Any;",
    ] {
        assert!(renamed_spellings(line).is_empty(), "{line}");
    }
}
