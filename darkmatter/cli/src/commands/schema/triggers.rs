//! `md schema triggers` implementation.

use biscuit_terminal::components::list::OrderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use renderable::markdown::code_span;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use crate::io::{DocumentOutsideRepository, open_argument};
use crate::request::MdRequest;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, InvalidSchemasDir, SchemaRoot, SchemaRootKind, SchemaRootState,
    trace_registry,
};
use std::path::Path;

/// Prints the five schema roots, shadowing, and arm-by-arm trigger results.
pub fn run_triggers(file: &Path, request: &MdRequest) -> Result<()> {
    // The argument resolves first, so a tree escape or malformed reference is
    // `InvalidReference` rather than a later missing-repository error.
    let opened = open_argument(file, request)?;
    // Legacy-spelling canonicalization: a verbatim `\\?\` result would gain a
    // path segment the gix-derived repository root lacks.
    let document_path = biscuit_file::canonicalize_simplified(opened.path())
        .unwrap_or_else(|_| opened.path().to_path_buf());
    let markdown = Markdown::try_from(document_path.as_path())?;
    let context = request.document_context(Some(opened.reference()), &document_path)?;
    if context.repository_root().is_none() {
        return Err(DocumentOutsideRepository { document: opened.path().to_path_buf() }.into());
    }
    let api = DarkmatterSchemas::new(context).with_trigger_discovery()?;
    let registry = api
        .trigger_registry()
        .expect("trigger discovery always installs a registry");
    let frontmatter = serde_json::Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    let trace = trace_registry(registry, &frontmatter, Some(&document_path));
    let terminal = Terminal::default();

    emit(&terminal, format!("<bold>Document:</bold> {}", escaped(opened.path())));
    emit(&terminal, "<bold>Schema roots (search order):</bold>".to_string());
    let mut roots = OrderedList::empty();
    for root in trace.roots.entries() {
        roots.add(Prose::new(describe_root(root)));
    }
    println!("{}", roots.render(&terminal));

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

/// One root's line: its label, its folder, and why it is not searched when it
/// is not.
fn describe_root(root: &SchemaRoot) -> String {
    let label = root.kind.label();
    match &root.state {
        SchemaRootState::Searched(path) => format!("{label}: {}", escaped(path)),
        SchemaRootState::Absent(path) => {
            format!("{label}: {} <dim>(absent)</dim>", escaped(path))
        }
        SchemaRootState::Duplicate { path, of } => format!(
            "{label}: {} <dim>(same folder as the {}; searched there)</dim>",
            escaped(path),
            of.label()
        ),
        SchemaRootState::NotApplicable => {
            let reason = match root.kind {
                SchemaRootKind::Package => "none (the document is not in a package)",
                SchemaRootKind::PackageArea => "none (the document is not in a package area)",
                SchemaRootKind::Tree => "none",
                SchemaRootKind::SchemasDir => "unset",
                SchemaRootKind::Home => "none (no home directory)",
            };
            format!("{label}: <dim>{reason}</dim>")
        }
        SchemaRootState::Invalid { value, reason } => {
            let reason = match reason {
                InvalidSchemasDir::Empty => "invalid (empty)",
                InvalidSchemasDir::Relative => "invalid (not an absolute path)",
            };
            format!("{label}: {} <red>{reason}</red>", code_span(value))
        }
    }
}

fn escaped(path: &Path) -> String {
    Prose::escape_text(&path.display().to_string())
}

fn emit(terminal: &Terminal, content: String) {
    println!("{}", Prose::new(content).render(terminal));
}
