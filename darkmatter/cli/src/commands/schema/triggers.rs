//! `md schema triggers` implementation.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::capture_file_resolution_context;
use darkmatter::markdown::schemas::{DarkmatterSchemas, normalize_path, trace_registry};
use std::path::Path;

/// Prints repository roots, shadowing, and arm-by-arm trigger results.
pub fn run_triggers(file: &Path) -> Result<()> {
    let document_path = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let markdown = Markdown::try_from(document_path.as_path())?;
    let boundary = capture_file_resolution_context(document_path.parent().unwrap_or(&document_path))
        .repository_root()
        .map(Path::to_path_buf)
        .ok_or_else(|| eyre!("no repository boundary found for `{}`", file.display()))?;
    let api = DarkmatterSchemas::new().with_trigger_discovery(&document_path, &boundary)?;
    let registry = api
        .trigger_registry()
        .expect("trigger discovery always installs a registry");
    let normalized = normalize_path(&document_path, &boundary)
        .ok_or_else(|| eyre!("document is outside discovery boundary"))?;
    let frontmatter = serde_json::Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    let trace = trace_registry(registry, &frontmatter, &normalized);
    let terminal = Terminal::default();

    emit(&terminal, format!("<bold>Boundary:</bold> {}", escaped(&trace.boundary)));
    emit(&terminal, "<bold>Schema roots:</bold>".to_string());
    if trace.roots.is_empty() {
        emit(&terminal, "- <dim>none</dim>".to_string());
    }
    for root in &trace.roots {
        emit(&terminal, format!("- {}", escaped(root)));
    }

    emit(&terminal, "<bold>Shadowed envelopes:</bold>".to_string());
    if trace.shadowed.is_empty() {
        emit(&terminal, "- <dim>none</dim>".to_string());
    }
    for (path, winner) in &trace.shadowed {
        emit(
            &terminal,
            format!("- {} <dim>(shadowed by {})</dim>", escaped(path), escaped(winner)),
        );
    }

    emit(&terminal, "<bold>Triggers:</bold>".to_string());
    if trace.triggers.is_empty() {
        emit(&terminal, "- <dim>none</dim>".to_string());
    }
    for trigger in &trace.triggers {
        let status = if trigger.matched { "<green>matched</green>" } else { "<red>not matched</red>" };
        emit(&terminal, format!("- {} — {status}", escaped(&trigger.source)));
        for arm in &trigger.arms {
            let result = if arm.matched {
                "<green>matched</green>".to_string()
            } else {
                format!(
                    "<red>defeated</red>: {}",
                    Prose::escape_text(arm.defeat.as_deref().unwrap_or("condition did not match"))
                )
            };
            emit(&terminal, format!("  - arm {} — {result}", arm.index + 1));
        }
    }
    Ok(())
}

fn escaped(path: &Path) -> String {
    Prose::escape_text(&path.display().to_string())
}

fn emit(terminal: &Terminal, content: String) {
    println!("{}", Prose::new(content).render(terminal));
}
