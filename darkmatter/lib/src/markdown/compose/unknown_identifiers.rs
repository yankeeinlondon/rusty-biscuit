//! Turns one document's unknown-root candidates into
//! `dm.expression.unknown_identifier` warnings (spec Requirement 4).
//!
//! Every runtime surface records a candidate when an evaluated read finds no
//! value, no absence construct handles it, and the surface's lookup does not
//! know the root. Frontmatter interpolation pass 1 runs before schema
//! validation and shell expansion, so a candidate is not yet a verdict. Each
//! document's pipeline calls [`reconcile`] once, after every stage has run and
//! before its report merges into a parent's: a root known to the final
//! effective state or declared by the effective schema is dropped, and each
//! remaining root warns once for the document.

use super::context::report::{CandidateLocus, UnknownRootCandidate};
use super::expression::EvaluationLookup;
use super::schema_validation::PreparedSchemas;
use super::shell_expansion::types::frontmatter_key_line;
use super::util::abbreviate_path;
use super::{ComposeOptions, ComposeReport, ComposeSource, ComposeWarning, EffectiveState};
use crate::markdown::Markdown;

/// Reconciles `report`'s candidates for `markdown` into warnings.
///
/// Known roots are those of the final effective state (frontmatter, `--set`,
/// caller files, inherited parent state) and any caller input record, even
/// one whose value never reached the state.
///
/// A non-terminal discovery pass (preflight command collection) composes
/// without page blocks, so it reads roots that the real run may never reach;
/// its candidates are dropped, never reported.
pub(crate) fn reconcile(
    report: &mut ComposeReport,
    markdown: &Markdown,
    options: &ComposeOptions,
    state: &EffectiveState,
    schemas: &PreparedSchemas,
) {
    let candidates = report.unknown_root_candidates.take();
    if options.defer_expression_failures {
        return;
    }
    let unknown: Vec<UnknownRootCandidate> = candidates
        .into_iter()
        .filter(|candidate| {
            !state.is_known_variable_root(&candidate.root)
                && !options.caller_input_records().contains_key(&candidate.root)
        })
        .collect();
    if unknown.is_empty() {
        return;
    }

    // `required` is not consulted: a required property left unset already
    // failed schema validation before this point, and an optional one is
    // declared, so either way the schema owns it.
    let schema = schemas.effective_for(markdown);
    let document = match markdown.source().as_ref().unwrap_or(&options.source) {
        ComposeSource::File(path) => Some(path.clone()),
        ComposeSource::Url(url) => Some(std::path::PathBuf::from(url.to_string())),
        ComposeSource::Unknown => None,
    };
    let display = document.as_deref().map(abbreviate_path);
    let source = markdown.full_source_context_for_errors();

    for candidate in unknown {
        if schema
            .as_ref()
            .is_some_and(|schema| schema.declares_top_level_property(&candidate.root))
        {
            continue;
        }
        let (line, key) = match &candidate.locus {
            CandidateLocus::Line(line) => (Some(*line), None),
            CandidateLocus::FrontmatterKey(key) => {
                (frontmatter_key_line(&source, key), Some(key.as_str()))
            }
            CandidateLocus::BodyLine(_) | CandidateLocus::Document => (None, None),
        };
        let message = message(&candidate.root, display.as_deref(), line, key);
        report.add_warning(ComposeWarning::unknown_identifier(
            candidate.stage,
            message,
            &candidate.root,
            document.clone(),
            line,
        ));
    }
}

/// Locates a directive whose `when=` read an unknown root, given its
/// one-based `body_line` in the document's current (possibly rewritten) body.
///
/// The line is claimed only when provable: the authored file has the same
/// text at that line, or has that directive text on exactly one line.
pub(crate) fn directive_locus(markdown: &Markdown, body_line: usize) -> CandidateLocus {
    let Some(current) = markdown.content().lines().nth(body_line.saturating_sub(1)) else {
        return CandidateLocus::Document;
    };
    let Some(loaded) = markdown.loaded_source_context_for_errors() else {
        return CandidateLocus::Document;
    };
    let authored = body_line + markdown.frontmatter_line_count();
    if loaded.content.lines().nth(authored - 1) == Some(current) {
        return CandidateLocus::Line(authored);
    }
    let mut matches = loaded
        .content
        .lines()
        .enumerate()
        .filter(|(_, line)| *line == current);
    match (matches.next(), matches.next()) {
        (Some((index, _)), None) => CandidateLocus::Line(index + 1),
        _ => CandidateLocus::Document,
    }
}

/// The one-based line of `text` in `loaded` when it occurs there exactly once,
/// which proves where an expression rescanned from rewritten text was authored.
pub(crate) fn unique_authored_line(loaded: &str, text: &str) -> Option<usize> {
    let mut found = loaded.match_indices(text);
    match (found.next(), found.next()) {
        (Some((start, _)), None) => Some(loaded[..start].matches('\n').count() + 1),
        _ => None,
    }
}

/// The warning text. The CLI renders only a warning's message, so the location
/// is part of it. Messages are Prose markup (both `md` and Claudine render them
/// through `Prose`), so every author-controlled part is escaped: a path such as
/// `prompts/_add/_workflow.md` would otherwise lose its underscores to emphasis.
fn message(root: &str, document: Option<&str>, line: Option<usize>, key: Option<&str>) -> String {
    use biscuit_terminal::components::prose::Prose;
    let root = Prose::escape_text(root);
    let document = document.map(Prose::escape_text);
    let document = document.as_deref();
    let key = key.map(Prose::escape_text);
    let key = key.as_deref();
    let mut location = match (document, line) {
        (Some(document), Some(line)) => format!(" at {document}:{line}"),
        (Some(document), None) => format!(" in {document}"),
        (None, Some(line)) => format!(" at line {line}"),
        (None, None) => String::new(),
    };
    if let Some(key) = key {
        location.push_str(&format!(" (frontmatter key '{key}')"));
    }
    format!(
        "unknown identifier '{root}'{location}: no frontmatter key, caller input, or schema \
         property defines it, so it resolves to null"
    )
}

#[cfg(test)]
mod tests {
    use super::{message, unique_authored_line};

    #[test]
    fn unique_authored_line_claims_only_a_single_occurrence() {
        let loaded = "---\na: 1\n---\none {{ x }}\ntwo {{ y }} {{ y }}\n";
        assert_eq!(unique_authored_line(loaded, "{{ x }}"), Some(4));
        assert_eq!(unique_authored_line(loaded, "{{ y }}"), None);
        assert_eq!(unique_authored_line(loaded, "{{ z }}"), None);
    }

    #[test]
    fn message_names_the_root_and_the_most_precise_location() {
        assert_eq!(
            message("spec-name", Some("prompts/a.md"), Some(12), None),
            "unknown identifier 'spec-name' at prompts/a.md:12: no frontmatter key, caller \
             input, or schema property defines it, so it resolves to null"
        );
        assert!(message("x", Some("a.md"), None, None).contains("'x' in a.md:"));
        assert!(
            message("x", Some("a.md"), Some(3), Some("label"))
                .contains("'x' at a.md:3 (frontmatter key 'label'):")
        );
        assert!(message("x", None, Some(3), None).contains("'x' at line 3:"));
        assert!(message("x", None, None, None).starts_with("unknown identifier 'x': "));
    }

    #[test]
    fn message_escapes_author_controlled_markup() {
        let text = message("a_b", Some("prompts/_add/_workflow.md"), Some(2), Some("k_y"));
        assert!(text.starts_with(r"unknown identifier 'a\_b' at prompts/\_add/\_workflow.md:2"), "{text}");
        assert!(text.contains(r"(frontmatter key 'k\_y')"), "{text}");
    }
}
