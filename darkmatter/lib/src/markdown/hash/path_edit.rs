//! Text-preserving edits of nested frontmatter nodes.
//!
//! [`restore_properties_text`](super::restore_properties_text) rewrites whole
//! top-level nodes, so using it for `author.name` would lose the comments on
//! `author`'s other children. This editor changes only the named node, and
//! keeps every other byte apart from the separators a container needs to stay
//! valid.
//!
//! An edit is never trusted because it was computed: the result is reloaded
//! and compared with the values the caller expects, before anything is
//! returned. See [`edit_frontmatter_paths`]. The node-range engine itself is
//! shared with the plain-YAML editor ([`crate::yaml_edit`]); this module is its
//! Markdown adapter, supplying the frontmatter envelope.

use super::node_edit::{self, Envelope, Layout, differences, unsupported, would_change};
use super::write::{
    apply_terminators, new_block_terminator, new_frontmatter_block, preceding_terminator,
};
use crate::markdown::{FrontmatterMap, Markdown, MarkdownError, extract_frontmatter_block};
use biscuit_file::Indentation;
use simplified_schema::completion::PathStep;
use std::cell::OnceCell;
use std::ops::Range;

/// One change to make to a document's frontmatter text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathEdit<'a> {
    /// Writes the value `expected` holds at this path, creating any missing
    /// mapping parents. The steps describe `expected`, as
    /// `PropertyPath::resolve_map` reports them for the edited map.
    Set(Vec<PathStep<'a>>),
    /// Removes the node at this path. The steps describe the unedited source,
    /// as `RemovalPlan::targets` reports them; array indexes are the original
    /// ones, whatever order the removals are listed in.
    Remove(Vec<PathStep<'a>>),
}

/// Why a frontmatter edit produced no text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathEditRefusal {
    /// The source is written in a form this editor cannot change safely. Whole
    /// frontmatter serialization may still be able to express the edit.
    #[error("the frontmatter cannot be edited in place: {reason}")]
    UnsupportedForm {
        /// What the editor met.
        reason: String,
    },
    /// The edit would change a value it does not name, normally because an
    /// alias reads an anchor inside the edited node. Serializing instead would
    /// hide this, so it has no fallback.
    #[error("{reason}")]
    WouldChangeUnnamed {
        /// Which value would change.
        reason: String,
    },
}

/// Applies `edits` to `source`'s frontmatter, keeping every byte outside the
/// edited nodes.
///
/// The result has been reloaded by the document loader and every value compared
/// with `expected`, the map the same edits produced on the parsed document, so
/// a returned text is safe to write. Only a path's own node is replaced, a new
/// key goes at the end of its parent, and a removed node takes its own lines. A
/// mapping or sequence left empty stays, written `{}` or `[]`.
///
/// ## Errors
///
/// [`PathEditRefusal::UnsupportedForm`] for a source this editor cannot change
/// safely: frontmatter that is indented, flow-style, or not strict YAML; a path
/// that walks into a flow collection, an alias, or a tagged node; or a node the
/// locator cannot bound. It is also the outcome when the named value itself
/// does not reload as expected.
///
/// [`PathEditRefusal::WouldChangeUnnamed`] when any other value would differ
/// from `expected`, or an alias would lose its anchor.
pub fn edit_frontmatter_paths(
    source: &str,
    expected: &FrontmatterMap,
    edits: &[PathEdit<'_>],
) -> Result<String, PathEditRefusal> {
    edit_frontmatter_paths_with_indentation(source, expected, edits, Indentation::default)
}

/// [`edit_frontmatter_paths`], laying out a new collection that has no
/// authored example in the frontmatter with the preference `fallback`
/// returns for the file the caller saves.
///
/// `fallback` is asked at most once, and only when a changed value needs a
/// layout: when `source` already holds `expected`, `source` is returned as it
/// is and nothing is resolved.
///
/// ## Errors
///
/// As [`edit_frontmatter_paths`].
pub fn edit_frontmatter_paths_with_indentation(
    source: &str,
    expected: &FrontmatterMap,
    edits: &[PathEdit<'_>],
    fallback: impl Fn() -> Indentation,
) -> Result<String, PathEditRefusal> {
    // Edits that already hold change nothing, so a repeated save keeps the
    // authored bytes and resolves no layout, as the plain-YAML editor does.
    if reload_values(source).is_ok_and(|original| original == *expected) {
        return Ok(source.to_string());
    }
    let layout = Layout::new(&fallback, true);
    let text = node_edit::apply_edits(&Frontmatter, source, expected, edits, &layout)?;
    node_edit::verify_edited(&Frontmatter, &text, expected, edits)?;
    Ok(text)
}

/// The text to write for `edited`, whose frontmatter already holds `edits`.
///
/// Edits the text `edited` was loaded from. Where the editor refuses the form,
/// or there is no source text, the whole frontmatter is serialized instead,
/// which loses comments and spelling in the frontmatter but is verified the same
/// way. Either way, the fences and body keep the source's bytes.
///
/// ## Errors
///
/// [`PathEditRefusal::WouldChangeUnnamed`] from the editor, or when the
/// serialized text would not reload to the same values.
/// [`PathEditRefusal::UnsupportedForm`] when the serialized text does not
/// reload at all.
pub fn edited_document_text(
    edited: &Markdown,
    edits: &[PathEdit<'_>],
) -> Result<String, PathEditRefusal> {
    edited_document_text_with_indentation(edited, edits, Indentation::default)
}

/// [`edited_document_text`] for a document saved to a destination whose
/// preference `fallback` returns. The editor uses it for a new collection with
/// no authored example, and the whole-frontmatter fallback writes with it.
///
/// `fallback` is asked at most once, and never when `edited` still holds the
/// values of the text it was loaded from: that text is returned unchanged.
///
/// ## Errors
///
/// As [`edited_document_text`].
pub fn edited_document_text_with_indentation(
    edited: &Markdown,
    edits: &[PathEdit<'_>],
    fallback: impl Fn() -> Indentation,
) -> Result<String, PathEditRefusal> {
    let resolved = OnceCell::new();
    let fallback = || *resolved.get_or_init(&fallback);
    let expected = edited.frontmatter().as_map();
    let Some(source) = edited.source_text() else {
        return serialized_document_text(edited, fallback());
    };
    match edit_frontmatter_paths_with_indentation(source, expected, edits, fallback) {
        Err(PathEditRefusal::UnsupportedForm { .. }) => serialized_document_text(edited, fallback()),
        result => result,
    }
}

fn serialized_document_text(edited: &Markdown, indentation: Indentation) -> Result<String, PathEditRefusal> {
    let text = serialized_into_source(edited, indentation)?;
    let reloaded = reload_values(&text)?;
    match differences(edited.frontmatter().as_map(), &reloaded).first() {
        Some(path) => Err(would_change(path)),
        None => Ok(text),
    }
}

/// The frontmatter serialized between the source's own fences.
///
/// `Markdown::as_string` is not enough when there is a source: the loaded body
/// has lost its final line break and its line endings.
fn serialized_into_source(edited: &Markdown, indentation: Indentation) -> Result<String, PathEditRefusal> {
    let Some(source) = edited.source_text() else {
        return Ok(edited.as_string());
    };
    let map = edited.frontmatter().as_map();
    let yaml = if map.is_empty() {
        String::new()
    } else {
        biscuit_file::to_yaml_string(map, indentation)
            .map_err(|error| unsupported(format!("the frontmatter could not be serialized: {error}")))?
    };
    let frontmatter =
        extract_frontmatter_block(source).map_err(|error| unsupported(error.to_string()))?;
    let Some(frontmatter) = frontmatter else {
        return Ok(new_frontmatter_block(
            source,
            &apply_terminators(&yaml, &[new_block_terminator(source)]),
        ));
    };
    let span = frontmatter.yaml_span;
    let newline = preceding_terminator(source, span.end);
    Ok(format!(
        "{}{}{}",
        &source[..span.start],
        apply_terminators(&yaml, &[newline]),
        &source[span.end..]
    ))
}
/// The Markdown frontmatter block, between its `---` fences.
struct Frontmatter;

impl Envelope for Frontmatter {
    fn yaml_span(&self, document: &str) -> Result<Option<Range<usize>>, PathEditRefusal> {
        let frontmatter =
            extract_frontmatter_block(document).map_err(|error| unsupported(error.to_string()))?;
        Ok(frontmatter.map(|frontmatter| frontmatter.yaml_span))
    }

    fn with_new_block(&self, document: &str, yaml: &str) -> String {
        new_frontmatter_block(document, yaml)
    }

    fn reload(&self, text: &str) -> Result<FrontmatterMap, PathEditRefusal> {
        reload_values(text)
    }
}

/// The frontmatter values of `text`, read as a document load reads them.
fn reload_values(text: &str) -> Result<FrontmatterMap, PathEditRefusal> {
    match Markdown::try_from_content(text) {
        Ok(document) => Ok(document.frontmatter().as_map().clone()),
        // The parser's wording is the only signal that this is an orphaned
        // alias rather than some other parse failure.
        Err(MarkdownError::FrontmatterParse { source, .. })
            if source.to_string().contains("unknown anchor") =>
        {
            Err(PathEditRefusal::WouldChangeUnnamed {
                reason: format!(
                    "the edit would remove an anchor that an alias outside the edited paths \
                     still reads: {source}"
                ),
            })
        }
        Err(error) => Err(unsupported(format!("the edited text does not load: {error}"))),
    }
}
