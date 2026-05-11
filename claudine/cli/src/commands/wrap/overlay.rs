use color_eyre::eyre::{Result, eyre};
use indexmap::IndexMap;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) fn merge_frontmatter_overlay(
    overlay: &mut IndexMap<String, Value>,
    update: &IndexMap<String, Value>,
) {
    for (key, value) in update {
        if value.is_null() {
            overlay.shift_remove(key);
        } else {
            overlay.insert(key.clone(), value.clone());
        }
    }
}

pub(crate) fn frontmatter_map_to_value(frontmatter: &darkmatter::markdown::Frontmatter) -> Value {
    Value::Object(
        frontmatter
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

pub(crate) fn materialize_passthrough_harness_seed(
    source_path: &Path,
    prompt: String,
) -> Result<super::harness_orch::MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(source_path)
        .map_err(|e| eyre!("failed to read '{}': {e}", source_path.display()))?;
    let source_markdown: darkmatter::markdown::Markdown = source_text.into();
    let options =
        darkmatter::markdown::compose::ComposeOptions::new().with_source_file(source_path);
    let (composed, _report) = source_markdown.compose_with(options)?;

    Ok(super::harness_orch::MaterializedHarnessPrompt {
        frontmatter: frontmatter_map_to_value(composed.frontmatter()),
        prompt,
        env_overrides: Vec::new(),
        inline_closure_plan: None,
    })
}
