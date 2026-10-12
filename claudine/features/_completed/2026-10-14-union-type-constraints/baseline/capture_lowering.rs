//! Scratch capture of lowered JSON Schema bytes for the byte-identity
//! baseline (AC 6). Not a shipped test.
//!
//! To rerun: copy this file to `darkmatter/lib/tests/l1/zz_lowering_capture.rs`,
//! add `mod zz_lowering_capture;` to `darkmatter/lib/tests/l1/main.rs`, run
//!
//! ```sh
//! LOWERING_CAPTURE_OUT=/tmp/lowered.txt cargo nextest run -p darkmatter \
//!     --test l1 -E 'test(zz_lowering_capture)'
//! ```
//!
//! then remove both edits. The run writes the full capture to the named file
//! and a per-entry BLAKE3 manifest beside it (`<file>.blake3`); compare the
//! manifest with `lowered-corpus.blake3`. For any differing entry, rerun the
//! capture in a worktree at the baseline revision to diff the full bytes.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::resolve::{resolve_schema_with_roots, resolve_yaml_schema};
use serde_yaml_ng::Value as YamlValue;

fn files_below(dir: &Path, ext: &str, skip_fixtures: bool) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = fs::read_dir(&current) else { continue };
        for entry in entries {
            let path = entry.expect("entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            if path.is_dir() {
                let skipped = matches!(name.as_str(), "target" | "node_modules" | ".git")
                    || (skip_fixtures && name == "fixtures");
                if !skipped {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|e| e == ext) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn yaml_key<'a>(map: &'a YamlValue, key: &str) -> Option<&'a YamlValue> {
    map.as_mapping()?.get(YamlValue::String(key.into()))
}

fn record(
    out: &mut String,
    manifest: &mut String,
    root: &str,
    label: &str,
    result: Result<serde_json::Value, String>,
) {
    let body = match result {
        Ok(schema) => serde_json::to_string_pretty(&schema).expect("serialize"),
        Err(message) => format!("ERROR {}", message.replace(root, "<root>")),
    };
    out.push_str(&format!("### {label}\n{body}\n"));
    manifest.push_str(&format!("{}  {label}\n", biscuit_hash::blake3_hash(&body)));
}

#[test]
fn zz_lowering_capture() {
    let Some(out_path) = std::env::var_os("LOWERING_CAPTURE_OUT") else {
        return;
    };
    let root = biscuit_test_harness::manifest_dir!().join("../..");
    let root = biscuit_file::canonicalize_simplified(&root).expect("root");
    let root_text = root.to_string_lossy().to_string();
    let scratch = tempfile::tempdir().expect("scratch");
    let context = FileResolutionContext::new(scratch.path());
    let mut out = String::new();
    let mut manifest = String::new();

    // Standalone schema files: shipped artifacts and test fixtures.
    let yaml_groups: [(&str, bool); 12] = [
        ("darkmatter/docs/schemas", true),
        ("darkmatter/schemas", true),
        ("content-policy/schemas", true),
        ("schemas", true),
        ("claudine/schemas", true),
        (".claudine/schemas", true),
        ("darkmatter/example-docs/schemas", true),
        ("claudine/docs/schemas", true),
        ("darkmatter/lib/tests/fixtures", false),
        ("darkmatter/cli/tests/fixtures", false),
        ("darkmatter/dmls/tests/fixtures", false),
        ("content-policy/tests/fixtures", false),
    ];
    for (group, skip_fixtures) in yaml_groups {
        for path in files_below(&root.join(group), "yaml", skip_fixtures) {
            let label = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let reference = serde_json::json!(format!("./{name}"));
            let result = resolve_schema_with_roots(&reference, path.parent().unwrap(), &[], &context)
                .map(|resolved| resolved.json_schema)
                .map_err(|error| error.to_string());
            record(&mut out, &mut manifest, &root_text, &label, result);
        }
    }

    // Inline `$schema` declarations in shipped Markdown and Markdown fixtures.
    for group in [
        "darkmatter/docs",
        "darkmatter/example-docs",
        "claudine/docs",
        "content-policy/docs",
        "docs/topics",
        ".claude/skills",
        "darkmatter/lib/tests/fixtures",
        "darkmatter/cli/tests/fixtures",
        "darkmatter/dmls/tests/fixtures",
    ] {
        for path in files_below(&root.join(group), "md", false) {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            if !text.contains("$schema") {
                continue;
            }
            let text = text.replace("\r\n", "\n");
            let Some(frontmatter) = text
                .strip_prefix("---\n")
                .and_then(|rest| rest.find("\n---").map(|end| rest[..end].to_string()))
            else {
                continue;
            };
            let Ok(frontmatter) = serde_yaml_ng::from_str::<YamlValue>(&frontmatter) else {
                continue;
            };
            let Some(declaration) = yaml_key(&frontmatter, "$schema") else { continue };
            let label = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
            let result = resolve_yaml_schema(declaration, path.parent().unwrap(), &context)
                .map(|resolved| resolved.json_schema)
                .map_err(|error| error.to_string());
            record(&mut out, &mut manifest, &root_text, &label, result);
        }
    }

    let mut manifest_path = out_path.clone();
    manifest_path.push(".blake3");
    fs::write(&out_path, out).expect("write capture");
    fs::write(manifest_path, manifest).expect("write manifest");
}
