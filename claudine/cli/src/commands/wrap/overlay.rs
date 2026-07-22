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
    shell_cwd: Option<&Path>,
    runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
) -> Result<super::harness_orch::MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(source_path).map_err(|e| {
        claudine::composition::CompositionError::MarkdownLoad {
            path: source_path.to_path_buf(),
            source: claudine::composition::MarkdownLoadCause::Read(e),
        }
    })?;
    let source_markdown: darkmatter::markdown::Markdown = source_text.into();
    let options = claudine::composition::bind_agent_workspace(
        darkmatter::markdown::compose::ComposeOptions::new(),
        source_path,
        shell_cwd,
    );
    let (composed, _report) = source_markdown.compose_with(options)?;

    let frontmatter = frontmatter_map_to_value(composed.frontmatter());
    let live_frontmatter =
        super::harness_orch::MaterializedHarnessPrompt::live_cell_from(&frontmatter);
    Ok(super::harness_orch::MaterializedHarnessPrompt {
        frontmatter,
        prompt,
        env_overrides: Vec::new(),
        inline_closure_plan: None,
        file_resolution_context: None,
        live_frontmatter,
        runtime_state,
    })
}
