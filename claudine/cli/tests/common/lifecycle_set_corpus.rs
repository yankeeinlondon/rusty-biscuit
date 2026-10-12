//! The corpus check that authored lifecycle `set` payloads are mappings.
//!
//! Shared by the Level 1 check over claudine's own artifacts and the opt-in
//! check over the repository's internal `prompts/`, which CI never runs.

use darkmatter::markdown::Markdown;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// How many `set` payloads the corpus under `roots` holds, and every artifact
/// that uses a non-mapping payload, the removed `action: set` form, or fails
/// to parse.
pub fn mapping_only_set_findings(roots: &[PathBuf]) -> (usize, Vec<String>) {
    use claudine::composition::lifecycle::parse_lifecycle_config;

    let mut files = Vec::new();
    for artifact_root in roots {
        collect_artifact_files(artifact_root, &mut files);
    }
    files.sort();
    assert!(!files.is_empty(), "the shipped artifact corpus must not be empty");

    let mut mapping_witnesses = 0;
    let mut defects = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            for line in text.lines() {
                let authored = line.trim_start().trim_start_matches("- ");
                for removed in ["set: [", "\"set\": [", "action: set", "action: \"set\""] {
                    if authored.starts_with(removed) {
                        defects.push(format!(
                            "{} advertises removed lifecycle syntax `{removed}`",
                            path.display()
                        ));
                    }
                }
            }
        }

        match path.extension().and_then(|extension| extension.to_str()) {
            Some("md") => match Markdown::try_from(path.as_path()) {
                Ok(markdown) => {
                    let frontmatter = frontmatter_value(&markdown);
                    let mut shape_defects = Vec::new();
                    let mut document_witnesses = 0;
                    inspect_mapping_only_set(
                        &frontmatter,
                        "$",
                        &mut document_witnesses,
                        &mut shape_defects,
                    );
                    mapping_witnesses += document_witnesses;
                    defects.extend(
                        shape_defects
                            .into_iter()
                            .map(|defect| format!("{}: {defect}", path.display())),
                    );
                    if document_witnesses > 0
                        && let Err(error) = parse_lifecycle_config(&frontmatter, path)
                    {
                        defects.push(format!("{}: {error}", path.display()));
                    }
                }
                Err(error) => defects.push(format!("{}: {error}", path.display())),
            },
            Some("json") => match serde_json::from_str::<Value>(&text) {
                Ok(value) => {
                    inspect_mapping_only_set(
                        &value,
                        &path.display().to_string(),
                        &mut mapping_witnesses,
                        &mut defects,
                    );
                }
                Err(error) => defects.push(format!("{}: {error}", path.display())),
            },
            _ => {}
        }
    }
    (mapping_witnesses, defects)
}

pub fn frontmatter_value(markdown: &Markdown) -> Value {
    Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

fn inspect_mapping_only_set(
    value: &Value,
    path: &str,
    witnesses: &mut usize,
    defects: &mut Vec<String>,
) {
    match value {
        Value::Object(map) => {
            if let Some(payload) = map.get("set") {
                *witnesses += 1;
                if !payload.is_object() {
                    defects.push(format!(
                        "{path}.set uses {}, not the required mapping payload",
                        match payload {
                            Value::Null => "null",
                            Value::Bool(_) => "a boolean",
                            Value::Number(_) => "a number",
                            Value::String(_) => "a string",
                            Value::Array(_) => "an array",
                            Value::Object(_) => unreachable!(),
                        }
                    ));
                }
            }
            if map.get("action") == Some(&Value::String("set".to_string())) {
                defects.push(format!(
                    "{path}.action uses the removed explicit `action: set` form"
                ));
            }
            for (key, child) in map {
                inspect_mapping_only_set(
                    child,
                    &format!("{path}.{key}"),
                    witnesses,
                    defects,
                );
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                inspect_mapping_only_set(
                    child,
                    &format!("{path}[{index}]"),
                    witnesses,
                    defects,
                );
            }
        }
        _ => {}
    }
}

fn collect_artifact_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
    {
        let path = entry.expect("artifact directory entry").path();
        if path.is_dir() {
            collect_artifact_files(&path, files);
        } else if path.extension().is_some_and(|extension| {
            matches!(extension.to_str(), Some("md" | "yaml" | "yml" | "json"))
        }) {
            files.push(path);
        }
    }
}
