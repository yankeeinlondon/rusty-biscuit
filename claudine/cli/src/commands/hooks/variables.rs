use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::utils::layout::{Edges, Length};
use claudine::dispatch::template::{TemplateVariable, VariableCategory};
use claudine::events::{EventMeta, detect_environment};

use crate::log;

use super::bold;

/// Show available template variables for speak/report actions.
///
/// Uses `TemplateVariable::all()` as the single source of truth - adding new
/// variables to the enum automatically updates this display.
pub(super) fn run_variables() -> Result<()> {
    let term = crate::log::terminal();

    let header = Prose::new("<bold>Template Variables</bold>");
    log::data(&format!("\n {}", header.render(&term)));
    log::data("");
    let intro = Prose::new(
        "<dim>Use these in speak messages and report templates: </dim><cyan>\"Tool {{tool_name}} failed: {{error}}\"</cyan>",
    );
    log::data(&format!(" {}", intro.render(&term)));
    log::data("");

    let columns = vec![
        TableColumn::new(bold("Variable")),
        TableColumn::new(bold("Description")),
        TableColumn::new(bold("Available For")),
    ];
    let mut table = Table::new().with_columns(columns);
    table.layout_mut().margin = Edges::x(Length::ch(1));

    for var in TemplateVariable::event_variables() {
        table.add_row(vec![
            Prose::new(format!("<cyan>{}</cyan>", var.placeholder()))
                .render(&term)
                .into(),
            var.description().into(),
            Prose::new(format!("<dim>{}</dim>", var.availability()))
                .render(&term)
                .into(),
        ]);
    }

    let rendered = table.render(&term);
    log::data(&rendered);

    log::data("");
    let ctx_header =
        Prose::new("<bold>Context Variables</bold> <dim>(auto-detected at runtime)</dim>");
    log::data(&format!(" {}", ctx_header.render(&term)));

    let cwd = std::env::current_dir().unwrap_or_default();
    let env_context = detect_environment(&cwd);
    let dummy_meta = EventMeta::dummy_with_env(env_context);

    let ctx_columns = vec![
        TableColumn::new(bold("Variable")),
        TableColumn::new(bold("Description")),
        TableColumn::new(bold("Current Value")),
    ];
    let mut ctx_table = Table::new().with_columns(ctx_columns);
    ctx_table.layout_mut().margin = Edges::x(Length::ch(1));

    let mut current_category: Option<VariableCategory> = None;

    for var in TemplateVariable::context_variables() {
        let cat = var.category();

        if current_category != Some(cat) {
            current_category = Some(cat);
            ctx_table.add_row(vec![
                Prose::new(format!("<dim># {}</dim>", cat.label()))
                    .render(&term)
                    .into(),
                "".into(),
                "".into(),
            ]);
        }

        let current_value = var.resolve(&dummy_meta);
        let value_display = if current_value.is_empty() {
            Prose::new("<dim>-</dim>").render(&term)
        } else {
            let truncated = if current_value.len() > 40 {
                format!("{}…", &current_value[..39])
            } else {
                current_value.to_string()
            };
            Prose::new(format!("<green>{}</green>", Prose::escape_text(&truncated))).render(&term)
        };

        ctx_table.add_row(vec![
            Prose::new(format!("<cyan>{}</cyan>", var.placeholder()))
                .render(&term)
                .into(),
            var.description().into(),
            value_display.into(),
        ]);
    }

    let ctx_rendered = ctx_table.render(&term);
    log::data(&ctx_rendered);

    log::data("");
    let example_header = Prose::new("<bold>Example</bold>");
    log::data(&format!(" {}", example_header.render(&term)));
    log::data("");
    let example = Prose::new(
        r#"<dim>  {
    "type": "speak",
    "message": "Tool {{tool_name}} failed on {{git.branch}}: {{error}}"
  }</dim>"#,
    );
    log::data(&example.render(&term));
    log::data("");

    Ok(())
}
