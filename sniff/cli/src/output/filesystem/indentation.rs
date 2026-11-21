//! Human rendering of `sniff repo indent`: one resolved file type, or the
//! repository summary.
//!
//! This only projects the library's result (`sniff::filesystem::indentation`);
//! nothing here reads configuration or re-ranks opinions. `--json` bypasses
//! this module and serializes the library value directly.

use std::path::Path;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::indentation::{
    AdjustmentReason, IndentAuthority, IndentOpinion, IndentStyle, IndentationDiagnostic,
    IndentationSummary, ResolvedIndentation,
};

/// The resolved value with its sources, each alternative authority with any
/// value that disagrees highlighted, adjustment and skipped notes, and the
/// consensus line.
pub(crate) fn render_resolved(resolved: &ResolvedIndentation, term: &Terminal) -> String {
    let mut lines = vec![format!(
        "<b>{}</b>  <dim>({})</dim>",
        value_markup(resolved.indent_using, resolved.indent.get(), None),
        sources(resolved),
    )];
    lines.extend(adjustment_note(resolved));
    let ranked = (ranked_style(resolved), resolved.indent.get());
    for opinion in &resolved.alt_authorities {
        lines.push(format!(
            "also: {}  {}",
            Prose::escape_text(&label(opinion)),
            opinion_markup(opinion, ranked),
        ));
    }
    lines.push(if resolved.consensus {
        "consensus: <green>yes</green>".to_string()
    } else {
        "consensus: <yellow>no</yellow>".to_string()
    });
    let mut out = render_lines(&lines, term);
    out.push_str(&render_skipped(&resolved.skipped, term));
    out
}

/// `default` first, then each extension whose indentation differs, with the
/// columns aligned, followed by the summary's deduplicated skipped notes.
pub(crate) fn render_summary(summary: &IndentationSummary, term: &Terminal) -> String {
    let rows: Vec<(&str, &ResolvedIndentation)> = std::iter::once(("default", &summary.default))
        .chain(summary.overrides.iter().map(|(ext, resolved)| (ext.as_str(), resolved)))
        .collect();
    let name_width = rows.iter().map(|(name, _)| name.chars().count()).max().unwrap_or(0);
    let values: Vec<String> = rows
        .iter()
        .map(|(_, resolved)| plain_value(resolved.indent_using, resolved.indent.get()))
        .collect();
    let value_width = values.iter().map(|value| value.chars().count()).max().unwrap_or(0);
    let lines: Vec<String> = rows
        .iter()
        .zip(&values)
        .map(|((name, resolved), value)| {
            let mut line = format!(
                "<b>{:<name_width$}</b>  {:<value_width$}  <dim>({})</dim>",
                Prose::escape_text(name),
                Prose::escape_text(value),
                sources(resolved),
            );
            if !resolved.consensus {
                line.push_str("  <yellow>sources disagree</yellow>");
            }
            line
        })
        .collect();
    let mut out = render_lines(&lines, term);
    out.push_str(&render_skipped(&summary.skipped, term));
    out
}

fn render_lines(lines: &[String], term: &Terminal) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(&InlineProse::new(line.as_str()).render(term));
        out.push('\n');
    }
    out
}

fn render_skipped(skipped: &[IndentationDiagnostic], term: &Terminal) -> String {
    if skipped.is_empty() {
        return String::new();
    }
    let items: Vec<String> = skipped
        .iter()
        .map(|diagnostic| {
            let key = diagnostic
                .key
                .as_deref()
                .map(|key| format!(" <b>{}</b>", Prose::escape_text(key)))
                .unwrap_or_default();
            InlineProse::new(format!(
                "{}{key}: {}",
                Prose::escape_text(&portable(&diagnostic.path)),
                Prose::escape_text(&diagnostic.reason),
            ))
            .render(term)
        })
        .collect();
    format!(
        "{}\n{}\n",
        InlineProse::new("<dim>skipped configuration:</dim>").render(term),
        UnorderedList::new(items).render(term)
    )
}

/// The source of the resolved value. When the style and the width come from
/// different opinions (or one from the fallback), each is named.
fn sources(resolved: &ResolvedIndentation) -> String {
    let style = supplier(resolved, |opinion| opinion.indent_using.is_some());
    let width = supplier(resolved, |opinion| opinion.indent.is_some());
    if style == width {
        Prose::escape_text(&style)
    } else {
        format!(
            "style: {}; width: {}",
            Prose::escape_text(&style),
            Prose::escape_text(&width)
        )
    }
}

/// The label of the highest-ranked opinion that states a value, or the
/// fallback's.
fn supplier(resolved: &ResolvedIndentation, states: impl Fn(&IndentOpinion) -> bool) -> String {
    std::iter::once(&resolved.authority)
        .chain(&resolved.alt_authorities)
        .find(|opinion| states(opinion))
        .map_or_else(|| "default".to_string(), label)
}

fn adjustment_note(resolved: &ResolvedIndentation) -> Option<String> {
    let adjustment = resolved.adjustment.as_ref()?;
    let reason = match adjustment.reason {
        AdjustmentReason::YamlForbidsTabs => "YAML does not allow tabs for indentation",
    };
    Some(format!(
        "note: {} requested; using {} because {reason}",
        style_word(adjustment.requested),
        style_word(resolved.indent_using),
    ))
}

/// The style the ranking selected, before any format adjustment: the value
/// alternative opinions are compared against.
fn ranked_style(resolved: &ResolvedIndentation) -> IndentStyle {
    resolved
        .adjustment
        .as_ref()
        .map_or(resolved.indent_using, |adjustment| adjustment.requested)
}

/// An opinion's stated values, each highlighted where it differs from the
/// ranked result.
fn opinion_markup(opinion: &IndentOpinion, (style, width): (IndentStyle, u8)) -> String {
    match (opinion.indent_using, opinion.indent.map(|width| width.get())) {
        (Some(own_style), Some(own_width)) => {
            value_markup(own_style, own_width, Some((style, width)))
        }
        (Some(own_style), None) => highlight(style_word(own_style), own_style != style),
        (None, Some(own_width)) => {
            format!("width {}", highlight(&own_width.to_string(), own_width != width))
        }
        (None, None) => "<dim>no value</dim>".to_string(),
    }
}

/// `4 spaces` or `tabs (width 2)`; with `compare`, each part that differs is
/// highlighted.
fn value_markup(style: IndentStyle, width: u8, compare: Option<(IndentStyle, u8)>) -> String {
    let (style_differs, width_differs) =
        compare.map_or((false, false), |(s, w)| (style != s, width != w));
    match style {
        IndentStyle::Spaces => format!(
            "{} {}",
            highlight(&width.to_string(), width_differs),
            highlight(if width == 1 { "space" } else { "spaces" }, style_differs),
        ),
        IndentStyle::Tabs => format!(
            "{} (width {})",
            highlight("tabs", style_differs),
            highlight(&width.to_string(), width_differs),
        ),
    }
}

fn plain_value(style: IndentStyle, width: u8) -> String {
    match style {
        IndentStyle::Spaces if width == 1 => "1 space".to_string(),
        IndentStyle::Spaces => format!("{width} spaces"),
        IndentStyle::Tabs => format!("tabs (width {width})"),
    }
}

fn highlight(text: &str, differs: bool) -> String {
    if differs {
        format!("<yellow><b>{text}</b></yellow>")
    } else {
        text.to_string()
    }
}

fn style_word(style: IndentStyle) -> &'static str {
    match style {
        IndentStyle::Spaces => "spaces",
        IndentStyle::Tabs => "tabs",
    }
}

/// How an opinion is named in plain output: `.editorconfig [*], [*.rs]`,
/// `rustfmt.toml tab_spaces`, `biome.json defaults`, `gofmt convention`.
fn label(opinion: &IndentOpinion) -> String {
    match opinion.source {
        IndentAuthority::Default => "default".to_string(),
        IndentAuthority::Convention => {
            format!("{} convention", opinion.via.first().map_or("language", String::as_str))
        }
        IndentAuthority::EditorConfig => {
            let path = opinion.path.as_deref().map_or_else(|| ".editorconfig".to_string(), portable);
            let sections: Vec<String> =
                opinion.via.iter().map(|section| format!("[{section}]")).collect();
            if sections.is_empty() {
                path
            } else {
                format!("{path} {}", sections.join(", "))
            }
        }
        _ => {
            let path = opinion.path.as_deref().map(portable).unwrap_or_default();
            if opinion.via.is_empty() {
                format!("{path} defaults")
            } else {
                format!("{path} {}", opinion.via.join(", "))
            }
        }
    }
}

fn portable(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;
    use std::path::PathBuf;

    fn opinion(
        source: IndentAuthority,
        path: Option<&str>,
        via: &[&str],
        style: Option<IndentStyle>,
        width: Option<u8>,
    ) -> IndentOpinion {
        IndentOpinion {
            source,
            path: path.map(PathBuf::from),
            via: via.iter().map(|v| v.to_string()).collect(),
            indent_using: style,
            indent: width.and_then(NonZeroU8::new),
        }
    }

    fn plain(text: String) -> String {
        biscuit_terminal::prelude::strip_escape_codes(text)
    }

    #[test]
    fn a_partial_authority_names_the_source_of_each_value() {
        let resolved = ResolvedIndentation {
            indent_using: IndentStyle::Spaces,
            indent: NonZeroU8::new(2).unwrap(),
            authority: opinion(
                IndentAuthority::EditorConfig,
                Some(".editorconfig"),
                &["*"],
                Some(IndentStyle::Spaces),
                None,
            ),
            alt_authorities: vec![opinion(
                IndentAuthority::Rustfmt,
                Some("rustfmt.toml"),
                &["tab_spaces"],
                None,
                Some(2),
            )],
            consensus: true,
            adjustment: None,
            skipped: vec![],
        };
        let text = plain(render_resolved(&resolved, &Terminal::default()));
        assert!(
            text.starts_with("2 spaces  (style: .editorconfig [*]; width: rustfmt.toml tab_spaces)\n"),
            "{text}"
        );
        assert!(text.contains("also: rustfmt.toml tab_spaces  width 2\n"), "{text}");
        assert!(text.ends_with("consensus: yes\n"), "{text}");
    }

    #[test]
    fn only_the_disagreeing_part_of_an_alternative_is_highlighted() {
        let ranked = (IndentStyle::Spaces, 4);
        let biome = opinion(IndentAuthority::Biome, Some("biome.json"), &[], Some(IndentStyle::Tabs), Some(4));
        assert_eq!(opinion_markup(&biome, ranked), "<yellow><b>tabs</b></yellow> (width 4)");
        let narrow = opinion(IndentAuthority::Prettier, Some(".prettierrc"), &[], Some(IndentStyle::Spaces), Some(2));
        assert_eq!(opinion_markup(&narrow, ranked), "<yellow><b>2</b></yellow> spaces");
        let agrees = opinion(IndentAuthority::Rustfmt, Some("rustfmt.toml"), &[], Some(IndentStyle::Spaces), Some(4));
        assert_eq!(opinion_markup(&agrees, ranked), "4 spaces");
    }

    #[test]
    fn a_value_stated_by_nobody_is_named_as_the_default() {
        let resolved = ResolvedIndentation {
            indent_using: IndentStyle::Tabs,
            indent: NonZeroU8::new(4).unwrap(),
            authority: opinion(
                IndentAuthority::EditorConfig,
                Some(".editorconfig"),
                &["*"],
                Some(IndentStyle::Tabs),
                None,
            ),
            alt_authorities: vec![],
            consensus: true,
            adjustment: None,
            skipped: vec![],
        };
        let text = plain(render_resolved(&resolved, &Terminal::default()));
        assert!(
            text.starts_with("tabs (width 4)  (style: .editorconfig [*]; width: default)\n"),
            "{text}"
        );
    }
}
