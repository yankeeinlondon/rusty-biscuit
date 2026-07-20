use color_eyre::eyre::{Result, eyre};
use indexmap::IndexMap;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Layer `update` onto `overlay` with shallow top-level semantics.
///
/// Every value — scalar, array, **and object** — replaces the key outright: no
/// deep merge. `null` removes the key instead of storing a null, so a router can
/// take a property away from its target rather than only overwrite it.
///
/// The shallowness is deliberate. A deep merge would make the result of
/// `with: { loop: {...} }` depend on what the target happened to author under
/// `loop:`, so the same router would mean different things against different
/// targets.
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
) -> Result<super::harness_orch::MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(source_path)
        .map_err(|e| eyre!("failed to read '{}': {e}", source_path.display()))?;
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
        selection_hints: claudine::composition::EffectiveSelectionHints::default(),
        inline_closure_plan: None,
        lifecycle: None,
        live_frontmatter,
        // A direct-wrapper passthrough records `mcp_body_tags: Vec::new()` as its
        // whole invocation facet set (`wrapper_stages::passthrough_launch_intent`)
        // — its MCP set comes from `--mcp-use`, not from a document body. Lexing
        // the provider memory file it wraps would fabricate a moved facet and
        // send this path into a replay it recorded no inputs for.
        mcp_body_tags: Vec::new(),
    })
}
