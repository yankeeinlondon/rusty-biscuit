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
    target_env_overrides: &[(String, String)],
) -> Result<super::harness_orch::MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(source_path).map_err(|e| {
        claudine::composition::CompositionError::MarkdownLoad {
            path: source_path.to_path_buf(),
            source: claudine::composition::MarkdownLoadCause::Read(e),
        }
    })?;
    let source_markdown: darkmatter::markdown::Markdown = source_text.into();
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_document(
        &source_markdown,
    );
    let mut context = invocation.capture_launch_context(&requirements);
    for (key, value) in target_env_overrides {
        context.env_mut().insert(key.clone(), value.clone());
    }
    let options = claudine::composition::bind_agent_workspace(
        darkmatter::markdown::compose::ComposeOptions::new_with_context(context.clone())
            .with_file_resolution_context(source_context.file_resolution_context().clone()),
        source_path,
        shell_cwd,
    );
    invocation.record_compose_operation();
    let (composed, _report) = source_markdown.compose_with(options)?;

    let frontmatter = frontmatter_map_to_value(composed.frontmatter());
    let live_frontmatter =
        super::harness_orch::MaterializedHarnessPrompt::live_cell_from(&frontmatter);
    Ok(super::harness_orch::MaterializedHarnessPrompt {
        frontmatter,
        prompt,
        env_overrides: target_env_overrides.to_vec(),
        selection_hints: claudine::composition::EffectiveSelectionHints::default(),
        inline_closure_plan: None,
        file_resolution_context: Some(source_context.file_resolution_context().clone()),
        compose_context: Some(context),
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

    fn init_repo(root: &Path) {
        std::fs::create_dir_all(root).unwrap();
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn write_two_area_workspace(root: &Path) {
        for (area, package) in [("alpha", "alpha-lib"), ("beta", "beta-lib")] {
            let package_root = root.join(area).join("lib");
            std::fs::create_dir_all(package_root.join("src")).unwrap();
            std::fs::write(
                package_root.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
                ),
            )
            .unwrap();
            std::fs::write(package_root.join("src/lib.rs"), "").unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"alpha/lib\", \"beta/lib\"]\nresolver = \"2\"\n",
        )
        .unwrap();
    }

    #[test]
    fn passthrough_seed_relocation_keeps_launch_ctx_and_source_file_resolution() {
        let fixture = tempfile::tempdir().unwrap();
        let launch_repo = fixture.path().join("launch-repository");
        let external_repo = fixture.path().join("external-repository");
        init_repo(&launch_repo);
        init_repo(&external_repo);
        write_two_area_workspace(&launch_repo);

        let launch_dir = launch_repo.join("alpha/lib");
        std::fs::write(launch_dir.join("spec.md"), "launch copy").unwrap();
        let sources = [
            launch_repo.join("memory-root.md"),
            launch_repo.join("beta/lib/memory-opposing.md"),
            external_repo.join("memory-external.md"),
        ];
        let invocation =
            claudine::invocation_context::InvocationContext::capture_at(&launch_dir);

        for source in &sources {
            let parent = source.parent().unwrap();
            std::fs::create_dir_all(parent).unwrap();
            let source_spec = parent.join("spec.md");
            std::fs::write(&source_spec, "source copy").unwrap();
            std::fs::write(
                source,
                "---\nlaunch_area: \"{{ ctx.area }}\"\nlaunch_repo: \"{{ ctx.repo_root }}\"\n---\nMemory body.\n",
            )
            .unwrap();
            let source_context = invocation.derive_source(source).unwrap();
            let materialized = materialize_passthrough_harness_seed(
                source,
                "provider prompt".to_string(),
                Some(&launch_dir),
                std::sync::Arc::new(claudine::composition::RuntimeState::new()),
                &invocation,
                &source_context,
                &[],
            )
            .unwrap();

            assert_eq!(
                materialized.frontmatter.get("launch_area").and_then(Value::as_str),
                Some("alpha-lib"),
                "source relocation changed ctx.area for {}",
                source.display()
            );
            assert_eq!(
                materialized.frontmatter.get("launch_repo").and_then(Value::as_str),
                Some(launch_repo.to_string_lossy().as_ref()),
                "source relocation changed ctx.repo_root for {}",
                source.display()
            );
            let resolved_spec = biscuit_file::FileReference::new("./spec.md")
                .unwrap()
                .resolve_in_context(
                    materialized
                        .file_resolution_context
                        .as_ref()
                        .expect("passthrough source keeps its file-resolution context"),
                )
                .unwrap();
            assert_eq!(
                resolved_spec.as_deref(),
                Some(source_spec.as_path()),
                "file resolution did not remain source-relative for {}",
                source.display()
            );
            assert_eq!(materialized.prompt, "provider prompt");
            assert_eq!(
                materialized
                    .compose_context
                    .as_ref()
                    .and_then(|context| context.get("area"))
                    .and_then(Value::as_str),
                Some("alpha-lib")
            );
        }

        let work = invocation.work_snapshot();
        assert_eq!(work.launch_context_constructions, sources.len());
        assert_eq!(work.ambient_fallbacks, 0);
    }
}
