use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent};
use biscuit_terminal::terminal::Terminal;
use claudine::diagnostics::{CODES, Category, CodeSpec, Severity};
use clap::Args;
use color_eyre::eyre::Result;

use crate::commands::context_render::{
    configure_shared_table, function_first_column_width, inline_code_text, render_table_resilient,
    report_column, TableLayout,
};
use crate::log;

/// Arguments for the `claudine errors` subcommand.
#[derive(Debug, Args)]
pub struct ErrorsArgs {
    /// Emit the locked code catalog as a JSON array.
    #[arg(long)]
    pub json: bool,
}

/// Groups the locked catalog by [`Category`], preserving the registry's
/// declaration order both across categories and within each.
///
/// `CODES` is already laid out grouped by category in declaration order, so a
/// single forward pass that breaks on category change reproduces the catalog's
/// intended grouping without re-sorting. Every code appears exactly once.
fn grouped_codes() -> Vec<(Category, Vec<&'static CodeSpec>)> {
    let mut groups: Vec<(Category, Vec<&'static CodeSpec>)> = Vec::new();
    for spec in CODES {
        match groups.last_mut() {
            Some((cat, specs)) if *cat == spec.category => specs.push(spec),
            _ => groups.push((spec.category, vec![spec])),
        }
    }
    groups
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

/// Colors a severity label to match its operator weight.
fn format_severity(severity: Severity, term: &Terminal) -> String {
    let label = severity_label(severity);
    let markup = match severity {
        Severity::Info => format!("<dim>{label}</dim>"),
        Severity::Warning => format!("<orange>{label}</orange>"),
        Severity::Error => format!("<red>{label}</red>"),
    };
    Prose::new(markup).render(term)
}

fn render_report() {
    let term = log::terminal();
    let groups = grouped_codes();

    let title = Prose::new("<blue><b>Claudine Diagnostic Error Codes</b></blue>");
    log::data(&title.render(&term));
    log::data("");

    let intro = "Every code is a stable `err.code` an author can match in a `when:` clause; \
        the `Detail` column lists the `err.detail.*` fields each code carries.";
    log::data(&inline_code_text(intro, &term));
    log::data("");

    // The widest code drives the first-column pin so every category section
    // aligns, mirroring the context report's shared-width contract. Cells render
    // backticked (and the backticks stay visible under NO_COLOR), so the pin must
    // measure the backticked form or the longest code wraps out of the copyable
    // surface.
    let backticked: Vec<String> = CODES.iter().map(|spec| format!("`{}`", spec.code)).collect();
    let code_width = function_first_column_width("Code", backticked.iter().map(String::as_str));

    for (category, specs) in &groups {
        let heading = Prose::new(format!("<blue><b>{}</b></blue>", category.as_str()));
        log::data(&heading.render(&term));

        let rendered = render_table_resilient(&term, |layout| {
            let mut code = report_column("Code");
            if layout == TableLayout::Pinned {
                code = code.with_min_width(code_width).with_max_width(code_width);
            }
            let mut table = Table::new().with_columns(vec![
                code,
                report_column("Disposition"),
                report_column("Origin"),
                report_column("Severity"),
                report_column("Detail"),
            ]);
            configure_shared_table(&mut table);
            for spec in specs {
                let row: Vec<TableCellContent> = vec![
                    inline_code_text(&format!("`{}`", spec.code), &term).into(),
                    spec.disposition.as_str().into(),
                    spec.origin.as_str().into(),
                    format_severity(spec.severity(), &term).into(),
                    inline_code_text(&detail_cell(spec.detail), &term).into(),
                ];
                table.add_row(row);
            }
            table
        });
        log::data(&rendered);
        log::data("");
    }
}

/// Renders the `detail` field names as a backticked, comma-joined list (empty
/// payloads render an em dash so the column never reads as a stray blank).
fn detail_cell(detail: &[&str]) -> String {
    if detail.is_empty() {
        return "—".to_string();
    }
    detail
        .iter()
        .map(|field| format!("`{field}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A serializable projection of one [`CodeSpec`] for `--json`.
///
/// Built CLI-side rather than deriving `Serialize` on the lib's `CodeSpec` so
/// the machine surface (`category`/`disposition`/`origin`/`severity` as their
/// stable snake_case slugs, plus the flat `detail` field list) is owned here
/// and the lib's public derives stay minimal.
fn json_view(spec: &CodeSpec) -> serde_json::Value {
    serde_json::json!({
        "code": spec.code,
        "category": spec.category.as_str(),
        "disposition": spec.disposition.as_str(),
        "origin": spec.origin.as_str(),
        "severity": severity_label(spec.severity()),
        "detail": spec.detail,
    })
}

/// Show the diagnostic error-code contract every code can be matched against.
pub fn run(args: ErrorsArgs) -> Result<()> {
    if args.json {
        let entries: Vec<serde_json::Value> = CODES.iter().map(json_view).collect();
        log::data(&serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    render_report();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Every code in the locked registry must appear exactly once in the
    /// grouping — so a future `CodeSpec` addition cannot silently drop from the
    /// introspection surface.
    #[test]
    fn grouped_codes_cover_every_code_exactly_once() {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, specs) in grouped_codes() {
            for spec in specs {
                *counts.entry(spec.code).or_default() += 1;
            }
        }

        assert_eq!(
            counts.len(),
            CODES.len(),
            "grouping must surface every distinct code"
        );
        for (code, count) in &counts {
            assert_eq!(*count, 1, "code `{code}` rendered {count} times, expected 1");
        }
    }

    /// Each category appears at most once as a heading — the registry is laid
    /// out grouped, so a single forward pass must not split a category into
    /// multiple non-adjacent runs.
    #[test]
    fn grouped_codes_emit_each_category_once() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for (category, _) in grouped_codes() {
            assert!(
                seen.insert(category),
                "category `{}` emitted more than once",
                category.as_str()
            );
        }
    }

    /// The detail cell joins field names and renders an em dash for the (rare)
    /// empty payload rather than a blank that reads as a missing cell.
    #[test]
    fn detail_cell_joins_fields_and_marks_empty() {
        assert_eq!(detail_cell(&["provider", "scope"]), "`provider`, `scope`");
        assert_eq!(detail_cell(&[]), "—");
    }

    /// The JSON view projects the stable snake_case slugs and the flat detail
    /// list — the machine contract `claudine errors --json` promises.
    #[test]
    fn json_view_uses_stable_slugs() {
        let spec = claudine::diagnostics::code_spec("cap.plan_limit").unwrap();
        let view = json_view(spec);
        assert_eq!(view["code"], "cap.plan_limit");
        assert_eq!(view["category"], "cap");
        assert_eq!(view["disposition"], "throttled");
        assert_eq!(view["origin"], "provider");
        assert!(
            view["detail"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "reset_at"),
            "detail must carry reset_at"
        );
    }
}
