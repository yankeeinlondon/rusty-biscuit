use biscuit_terminal::components::{prose::Prose, renderable::Renderable};
use sniff::filesystem::JustfileInfo;

/// Render justfile detection results as styled text.
pub fn render_just_text(justfiles: &[JustfileInfo], verbose: u8) -> String {
    if justfiles.is_empty() {
        return Prose::new("<dim>No justfiles found.</dim>").render_optimistic(None);
    }

    let mut out = String::new();

    out.push_str(
        &Prose::new(format!(
            "<b>Justfiles</b> <dim>({})</dim>\n",
            justfiles.len()
        ))
        .render_optimistic(None),
    );

    for jf in justfiles {
        let public_count = jf.recipes.iter().filter(|r| !r.private).count();
        let private_count = jf.recipes.iter().filter(|r| r.private).count();

        let summary = if private_count > 0 {
            format!(
                "  <b>{}</b> <dim>({} recipes, {} private)</dim>\n",
                jf.relative, public_count, private_count
            )
        } else {
            format!(
                "  <b>{}</b> <dim>({} recipes)</dim>\n",
                jf.relative, public_count
            )
        };
        out.push_str(&Prose::new(&summary).render_optimistic(None));

        for recipe in &jf.recipes {
            if recipe.private && verbose == 0 {
                continue;
            }

            let name_style = if recipe.private { "<dim>" } else { "<cyan>" };
            let name_end = if recipe.private { "</dim>" } else { "</cyan>" };
            let params_str = recipe
                .params
                .as_deref()
                .map(|p| format!(" <dim>{}</dim>", p))
                .unwrap_or_default();

            let line = if verbose > 0 {
                format!(
                    "    {}{}{}{} <dim>#{:016x}</dim>\n",
                    name_style, recipe.name, name_end, params_str, recipe.hash
                )
            } else {
                format!(
                    "    {}{}{}{}\n",
                    name_style, recipe.name, name_end, params_str
                )
            };
            out.push_str(&Prose::new(&line).render_optimistic(None));
        }

        out.push('\n');
    }

    out
}
