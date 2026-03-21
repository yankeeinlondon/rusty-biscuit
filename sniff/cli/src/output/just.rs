use std::collections::BTreeMap;
use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::pad::PadRight;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::RenderableContent;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::JustfileInfo;

/// Filter justfiles for JSON output when `--with` is active.
pub fn filter_justfiles_for_json<'a>(
    justfiles: &'a [JustfileInfo],
    with_recipe: Option<&str>,
) -> Vec<&'a JustfileInfo> {
    match with_recipe {
        Some(recipe) => justfiles
            .iter()
            .filter(|jf| jf.recipes.iter().any(|r| r.name == recipe))
            .collect(),
        None => justfiles.iter().collect(),
    }
}

/// Render justfile detection results as styled text.
pub fn render_just_text(
    justfiles: &[JustfileInfo],
    verbose: u8,
    with_recipe: Option<&str>,
    grouped: bool,
) -> String {
    let term = Terminal::default();

    if justfiles.is_empty() {
        return Prose::new("<dim>No justfiles found.</dim>").render(&term);
    }

    let output = if let Some(recipe) = with_recipe {
        if grouped {
            render_grouped_by_hash(justfiles, verbose, recipe, &term)
        } else {
            render_with_recipe_filter(justfiles, verbose, recipe, &term)
        }
    } else {
        render_justfile_list(justfiles, verbose, &term)
    };

    // Emit stderr warnings for justfiles missing a default recipe
    emit_default_warnings(justfiles);

    output
}

/// Render the standard justfile list (no `--with` filter).
fn render_justfile_list(justfiles: &[JustfileInfo], verbose: u8, term: &Terminal) -> String {
    let mut list = UnorderedList::empty();

    for jf in justfiles {
        list.add(render_justfile_item(jf, verbose, term));
    }

    let header = Prose::new(format!(
        "<b>Justfiles</b> <dim>({})</dim>",
        justfiles.len()
    ))
    .render(term);

    format!("{}\n{}", header, list.render(term))
}

/// Render justfiles with `--with <recipe>` filtering.
fn render_with_recipe_filter(
    justfiles: &[JustfileInfo],
    verbose: u8,
    recipe: &str,
    term: &Terminal,
) -> String {
    let (with, without): (Vec<_>, Vec<_>) = justfiles
        .iter()
        .partition(|jf| jf.recipes.iter().any(|r| r.name == recipe));

    if verbose == 0 {
        // Non-verbose: only show justfiles WITH the recipe
        if with.is_empty() {
            return Prose::new(format!(
                "<dim>No justfiles contain the </dim><purple-500>{}</purple-500><dim> recipe.</dim>",
                recipe
            ))
            .render(term);
        }

        let header = Prose::new(format!(
            "<b>Justfiles</b> with <purple-500>{}</purple-500> <dim>({})</dim>",
            recipe,
            with.len()
        ))
        .render(term);

        let mut list = UnorderedList::empty();
        for jf in &with {
            list.add(render_justfile_item_inner(jf, 0, false, term));
        }

        format!("{}\n{}", header, list.render(term))
    } else {
        // Verbose: show both With and Without headings
        let mut out = String::new();

        let with_header = Prose::new(format!(
            "<b>With</b> <purple-500>{}</purple-500> <dim>({})</dim>",
            recipe,
            with.len()
        ))
        .render(term);
        out.push_str(&with_header);
        out.push('\n');

        if with.is_empty() {
            out.push_str(&Prose::new("<dim>  (none)</dim>").render(term));
            out.push('\n');
        } else {
            let mut with_list = UnorderedList::empty();
            for jf in &with {
                with_list.add(render_justfile_item_inner(jf, verbose, false, term));
            }
            out.push_str(&with_list.render(term));
        }

        out.push('\n');

        let without_header = Prose::new(format!(
            "<b>Without</b> <purple-500>{}</purple-500> <dim>({})</dim>",
            recipe,
            without.len()
        ))
        .render(term);
        out.push_str(&without_header);
        out.push('\n');

        if without.is_empty() {
            out.push_str(&Prose::new("<dim>  (none)</dim>").render(term));
            out.push('\n');
        } else {
            let mut without_list = UnorderedList::empty();
            for jf in &without {
                without_list.add(render_justfile_item(jf, verbose, term));
            }
            out.push_str(&without_list.render(term));
        }

        out
    }
}

/// Render justfiles grouped by recipe hash when `--with <recipe> --grouped` is active.
///
/// Justfiles containing the recipe are grouped by matching hash values, so
/// justfiles whose recipe bodies are identical appear together. Each group
/// is a top-level list item with a nested list of justfile paths underneath.
/// Justfiles without the recipe are listed separately at the end (verbose only).
fn render_grouped_by_hash(
    justfiles: &[JustfileInfo],
    verbose: u8,
    recipe: &str,
    term: &Terminal,
) -> String {
    let (with, without): (Vec<_>, Vec<_>) = justfiles
        .iter()
        .partition(|jf| jf.recipes.iter().any(|r| r.name == recipe));

    if with.is_empty() {
        return Prose::new(format!(
            "<dim>No justfiles contain the </dim><purple-500>{}</purple-500><dim> recipe.</dim>",
            recipe
        ))
        .render(term);
    }

    // Group justfiles by the hash of the matching recipe
    let mut groups: BTreeMap<u64, Vec<&JustfileInfo>> = BTreeMap::new();
    for jf in &with {
        if let Some(r) = jf.recipes.iter().find(|r| r.name == recipe) {
            groups.entry(r.hash).or_default().push(jf);
        }
    }

    let mut out = String::new();

    let header = Prose::new(format!(
        "<b>Justfiles</b> with <purple-500>{}</purple-500> <dim>({} justfiles, {} unique)</dim>",
        recipe,
        with.len(),
        groups.len()
    ))
    .render(term);
    out.push_str(&header);
    out.push('\n');

    let mut outer_list = UnorderedList::empty();

    for (hash, members) in &groups {
        // Get description from the first member's matching recipe
        let description = members
            .first()
            .and_then(|jf| jf.recipes.iter().find(|r| r.name == recipe))
            .and_then(|r| r.description.as_deref())
            .unwrap_or("(no description)");

        let count_suffix = if members.len() > 1 {
            format!(" <dim>({}×)</dim>", members.len())
        } else {
            String::new()
        };

        let group_label = format!(
            "<b>#{:016x}</b>{}\n\n<dim><i>{}</i></dim>",
            hash, count_suffix, description
        );

        // Add the group label as a Prose component so markup is rendered
        outer_list.add(RenderableContent::Component(Rc::new(Prose::new(group_label))));

        // Add nested list of justfile paths
        let mut inner_list = UnorderedList::empty();
        for jf in members {
            inner_list.add(render_justfile_item_inner(jf, verbose, false, term));
        }
        outer_list.add(RenderableContent::Component(Rc::new(inner_list)));
    }

    out.push_str(&outer_list.render(term));

    // In verbose mode, also show justfiles without the recipe
    if verbose > 0 && !without.is_empty() {
        out.push('\n');
        let without_header = Prose::new(format!(
            "<b>Without</b> <purple-500>{}</purple-500> <dim>({})</dim>",
            recipe,
            without.len()
        ))
        .render(term);
        out.push_str(&without_header);
        out.push('\n');

        let mut without_list = UnorderedList::empty();
        for jf in &without {
            without_list.add(render_justfile_item(jf, verbose, term));
        }
        out.push_str(&without_list.render(term));
    }

    out
}

/// Render a single justfile as an UnorderedList item.
///
/// Returns a Prose-rendered string. In non-verbose mode, just the file path
/// and recipe count. In verbose mode, appends the recipe list underneath.
/// When `show_counts` is false, the recipe count suffix is omitted (used
/// with `--with` filtering where counts aren't relevant).
fn render_justfile_item(jf: &JustfileInfo, verbose: u8, term: &Terminal) -> String {
    render_justfile_item_inner(jf, verbose, true, term)
}

fn render_justfile_item_inner(
    jf: &JustfileInfo,
    verbose: u8,
    show_counts: bool,
    term: &Terminal,
) -> String {
    let abs_path = jf.path.display();

    let summary = if show_counts {
        let public_count = jf.recipes.iter().filter(|r| !r.private).count();
        let private_count = jf.recipes.iter().filter(|r| r.private).count();

        let private_suffix = if private_count > 0 {
            format!(", <i>{} private</i>", private_count)
        } else {
            String::new()
        };

        Prose::new(format!(
            "<a href=\"file://{}\">{}</a> <dim>({} recipes{})</dim>",
            abs_path, jf.relative, public_count, private_suffix
        ))
        .render(term)
    } else {
        Prose::new(format!(
            "<a href=\"file://{}\">{}</a>",
            abs_path, jf.relative
        ))
        .render(term)
    };

    if verbose == 0 {
        return summary;
    }

    // Verbose: add recipe list underneath
    let mut out = summary;
    out.push('\n');

    // Calculate max visible width of name + params for description alignment
    let max_name_params_width = jf
        .recipes
        .iter()
        .map(|r| {
            let params_display = format_params_plain(r);
            let name_width = r.name.len();
            if params_display.is_empty() {
                name_width
            } else {
                name_width + 1 + params_display.len() // +1 for space
            }
        })
        .max()
        .unwrap_or(0);

    let pad_width = (max_name_params_width + 2) as u32; // +2 breathing room

    let mut recipe_list = UnorderedList::empty().with_bullet("  ".to_string());

    for recipe in &jf.recipes {
        let line = render_recipe_line(recipe, verbose, pad_width, term);
        recipe_list.add(line);
    }

    out.push_str(&recipe_list.render(term));
    out
}

/// Format parameters as plain text (no ANSI) for width calculation.
fn format_params_plain(recipe: &sniff::filesystem::JustRecipe) -> String {
    if recipe.params.is_empty() {
        return String::new();
    }

    recipe
        .params
        .iter()
        .map(|p| {
            let prefix = if p.variadic {
                if p.optional && p.default.is_none() {
                    "*"
                } else if !p.optional {
                    "+"
                } else {
                    "*"
                }
            } else {
                ""
            };

            match &p.default {
                Some(val) if val.is_empty() => format!("{}{}", prefix, p.name),
                Some(val) => format!("{}{}=\"{}\"", prefix, p.name, val),
                None => format!("{}{}", prefix, p.name),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a single parameter with Prose markup.
fn format_param_styled(p: &sniff::filesystem::JustRecipeParam) -> String {
    let prefix = if p.variadic {
        if p.optional && p.default.is_none() {
            "*"
        } else if !p.optional {
            "+"
        } else {
            "*"
        }
    } else {
        ""
    };

    let display = match &p.default {
        Some(val) if val.is_empty() => format!("{}{}", prefix, p.name),
        Some(val) => format!("{}{}=\"{}\"", prefix, p.name, val),
        None => format!("{}{}", prefix, p.name),
    };

    if p.optional {
        format!("<dim>{}</dim>", display)
    } else {
        display
    }
}

/// Render a single recipe line with styled name, params, and optional description.
fn render_recipe_line(
    recipe: &sniff::filesystem::JustRecipe,
    verbose: u8,
    pad_width: u32,
    term: &Terminal,
) -> String {
    let name_styled = if recipe.private {
        format!("<dim>{}</dim>", recipe.name)
    } else {
        format!("<purple-500>{}</purple-500>", recipe.name)
    };

    let params_styled: String = if recipe.params.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = recipe.params.iter().map(format_param_styled).collect();
        format!(" {}", parts.join(" "))
    };

    let name_and_params = format!("{}{}", name_styled, params_styled);

    let line = if let Some(ref desc) = recipe.description {
        // Use PadRight to align descriptions
        let padded = PadRight::new(
            Prose::new(&name_and_params),
            pad_width,
        );
        let padded_str = padded.render(term);
        let desc_str = Prose::new(format!("<dim>{}</dim>", desc)).render(term);
        format!("{}{}", padded_str, desc_str)
    } else {
        Prose::new(&name_and_params).render(term)
    };

    if verbose >= 2 {
        let hash_str = Prose::new(format!("  <dim>#{:016x}</dim>", recipe.hash)).render(term);
        format!("{}{}", line, hash_str)
    } else {
        line
    }
}

/// Emit stderr warnings for justfiles that lack a `default` recipe.
fn emit_default_warnings(justfiles: &[JustfileInfo]) {
    for jf in justfiles {
        if !jf.has_default {
            let term = Terminal::default();
            let warning = Prose::new(format!(
                "<i><dim>no </dim><blue>default</blue><dim> recipe was defined in </dim>{}</i>",
                jf.relative
            ))
            .render(&term);
            eprintln!("{}", warning);
        }
    }
}
