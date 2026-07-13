//! The side-effect capability report.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::Table;
use darkmatter::effects::{effect_descriptors, EffectDescriptor};

use crate::commands::context_render::{
    configure_shared_table, example_column_floor, function_first_column_width, inline_code_text,
    render_table_resilient, render_unordered_list, report_column, TableLayout,
};
use crate::log;

use super::expressions::show_examples;
use super::format::{format_effect_example, format_safety};

pub(super) fn group_effect_descriptors() -> Vec<(&'static str, Vec<&'static EffectDescriptor>)> {
    let mut groups: Vec<(&'static str, Vec<&'static EffectDescriptor>)> = Vec::new();
    let mut current_cat: Option<&'static str> = None;

    for desc in effect_descriptors() {
        if current_cat != Some(desc.category) {
            current_cat = Some(desc.category);
            groups.push((desc.category, Vec::new()));
        }
        groups.last_mut().unwrap().1.push(desc);
    }
    groups
}

pub(super) fn render_side_effects_report() {
    let term = log::terminal();

    let title = Prose::new("<blue><b>Darkmatter Side-Effect Capabilities</b></blue>");
    log::data(&title.render(&term));
    log::data("");

    let intro = Prose::new(
        "This report documents Darkmatter's side-effect capabilities. It is documentation-only and does not invoke any side effects.",
    );
    log::data(&intro.render(&term));

    let constraint_items = [
        "this report is documentation only and does not invoke side effects",
        "only an external orchestrator invokes side effects",
        "filesystem writes are restricted to the configured mutation root",
        "network operations are restricted by the host allowlist, which is deny-all by default",
        "Markdown mutations honor Darkmatter's auto-rehash behavior",
        "capability catalog membership does not imply authorization or availability",
    ];
    log::data(&render_unordered_list(
        &constraint_items
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        &term,
    ));

    let groups = group_effect_descriptors();

    let all_signatures: Vec<String> = groups
        .iter()
        .flat_map(|(_, effects)| effects.iter().map(|e| e.signature.to_string()))
        .collect();
    let sig_refs: Vec<&str> = all_signatures.iter().map(|s| s.as_str()).collect();
    let cap_width = function_first_column_width("Capability", sig_refs.into_iter());

    // The Example column is useful on wider terminals, but at the minimum
    // supported width the four-column report cannot satisfy the table planner.
    // Drop the column below the shared threshold rather than emitting a
    // width-error diagnostic.
    let with_examples = show_examples(&term);

    for (category, effects) in &groups {
        let cat_heading = Prose::new(format!("<blue><b>{category}</b></blue>"));
        log::data(&cat_heading.render(&term));

        // Floor the `Example` column so a long `Description` line cannot starve
        // it under the planner's greedy surplus distribution. See
        // `example_column_floor`.
        let example_strings: Vec<String> = effects
            .iter()
            .map(|effect| format_effect_example(effect.example.as_ref(), &term))
            .collect();
        let example_width = example_column_floor(example_strings.iter().map(String::as_str));

        let rendered = render_table_resilient(&term, |layout| {
            let mut capability = report_column("Capability");
            if layout == TableLayout::Pinned {
                capability = capability.with_min_width(cap_width).with_max_width(cap_width);
            }
            let mut columns = vec![
                capability,
                report_column("Description"),
                report_column("Safety"),
            ];
            if with_examples {
                let mut example = report_column("Example");
                if layout == TableLayout::Pinned {
                    example = example.with_min_width(example_width);
                }
                columns.insert(2, example);
            }
            let mut table = Table::new().with_columns(columns);
            configure_shared_table(&mut table);
            for effect in effects {
                let mut row: Vec<biscuit_terminal::components::table::table::TableCellContent> = vec![
                    inline_code_text(effect.signature, &term).into(),
                    inline_code_text(effect.description, &term).into(),
                    format_safety(effect.safety, &term).into(),
                ];
                if with_examples {
                    row.insert(
                        2,
                        format_effect_example(effect.example.as_ref(), &term).into(),
                    );
                }
                table.add_row(row);
            }
            table
        });
        log::data(&rendered);
        log::data("");
    }
}
