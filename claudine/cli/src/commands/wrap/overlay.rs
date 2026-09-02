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
    runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
    invocation: &claudine::invocation_context::InvocationContext,
    source_context: &claudine::invocation_context::SourceContext,
) -> Result<super::harness_orch::MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(source_path).map_err(|e| {
        claudine::composition::CompositionError::MarkdownLoad {
            path: source_path.to_path_buf(),
            source: claudine::composition::MarkdownLoadCause::Read(e),
        }
    })?;
    let source_markdown: darkmatter::markdown::Markdown = source_text.into();
    let source_markdown = source_markdown.with_source(
        darkmatter::markdown::compose::ComposeSource::File(source_path.to_path_buf()),
    );
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_document(
        &source_markdown,
    );
    // Launch-anchored, never source-anchored: moving the wrapped memory file
    // must not change its launch-facing `ctx.*` expansion (AC9).
    let document_epoch = invocation.begin_document_epoch();
    let context = document_epoch.capture_launch_context(&requirements);
    let options = claudine::composition::bind_agent_workspace(
        darkmatter::markdown::compose::ComposeOptions::new_with_context(context.clone())
            .with_file_resolution_context(source_context.file_resolution_context().clone()),
        source_path,
        shell_cwd,
    );
    invocation.record_compose_operation();
    document_epoch.record_prepared_context_consumer(
        claudine::invocation_context::PreparedContextConsumer::EffectiveFrontmatter,
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
        file_resolution_context: Some(source_context.file_resolution_context().clone()),
        compose_context: Some(context),
        document_epoch: Some(document_epoch),
        live_frontmatter,
        runtime_state,
        lifecycle: None,
        // A direct-wrapper passthrough records `mcp_body_tags: Vec::new()` as its
        // whole invocation facet set (`wrapper_stages::passthrough_launch_intent`)
        // — its MCP set comes from `--mcp-use`, not from a document body. Lexing
        // the provider memory file it wraps would fabricate a moved facet and
        // send this path into a replay it recorded no inputs for.
        mcp_body_tags: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_git_repo(path: &Path) {
        let status = std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn relocated_overlay_seed_uses_launch_context_and_source_resolution() {
        let temp = tempfile::tempdir().unwrap();
        let launch_repo = temp.path().join("launch");
        let launch_dir = launch_repo.join("alpha/lib");
        let source_repo = temp.path().join("source");
        let source_dir = source_repo.join("nested");
        fs::create_dir_all(&launch_dir).unwrap();
        fs::create_dir_all(&source_dir).unwrap();
        init_git_repo(&launch_repo);
        init_git_repo(&source_repo);
        fs::write(
            launch_repo.join("Cargo.toml"),
            "[workspace]\nmembers = [\"alpha/lib\", \"sibling\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        fs::write(
            launch_dir.join("Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::create_dir_all(launch_repo.join("sibling")).unwrap();
        fs::write(
            launch_repo.join("sibling/Cargo.toml"),
            "[package]\nname = \"sibling\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();

        fs::write(
            launch_dir.join("schema.yaml"),
            "launch_only: string(required)\n",
        )
        .unwrap();
        fs::write(launch_dir.join("spec.md"), "LAUNCH-SPEC\n").unwrap();
        fs::write(
            source_dir.join("schema.yaml"),
            concat!(
                "source_file: 'file(eager; required)'\n",
                "prepared_area: string(required)\n",
                "prepared_repo: string(required)\n",
                "prepared_cwd: string(required)\n",
            ),
        )
        .unwrap();
        fs::write(source_dir.join("spec.md"), "SOURCE-SPEC\n").unwrap();
        let source = source_dir.join("memory.md");
        fs::write(
            &source,
            concat!(
                "---\n",
                "$schema: ./schema.yaml\n",
                "source_file: spec.md\n",
                "prepared_area: '{{ ctx.area }}'\n",
                "prepared_repo: '{{ ctx.repo_root }}'\n",
                "prepared_cwd: '{{ ctx.cwd }}'\n",
                "---\n",
                "Memory body.\n",
            ),
        )
        .unwrap();

        let invocation = claudine::invocation_context::InvocationContext::capture_at(&launch_dir);
        let source_context = invocation.derive_source(&source).unwrap();
        let materialized = materialize_passthrough_harness_seed(
            &source,
            "provider-owned prompt".to_string(),
            None,
            std::sync::Arc::new(claudine::composition::RuntimeState::new()),
            &invocation,
            &source_context,
        )
        .unwrap();

        assert_eq!(materialized.frontmatter["prepared_area"], serde_json::json!("alpha"));
        assert_eq!(
            materialized.frontmatter["prepared_cwd"],
            serde_json::json!(biscuit_file::to_portable_string(&launch_dir))
        );
        assert_eq!(
            materialized.frontmatter["prepared_repo"],
            serde_json::json!(biscuit_file::to_portable_string(&launch_repo))
        );
        assert_eq!(materialized.prompt, "provider-owned prompt");
        let resolution = materialized
            .file_resolution_context
            .expect("overlay seed keeps the source resolution snapshot");
        assert_eq!(resolution.repository_root(), Some(source_repo.as_path()));
        assert_eq!(resolution.source_path(), Some(source.as_path()));
    }
}
