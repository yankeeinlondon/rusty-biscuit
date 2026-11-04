//! The Markdown of the `$schema` and `file` value hovers.
//!
//! Every function here formats facts a caller already gathered: none reads
//! the filesystem, resolves a reference, or composes. Links are Markdown
//! only (clients render inline HTML unreliably), labels and authored error
//! text are escaped, and a URI comes from
//! [`file_path_to_uri`](crate::workspace::file_path_to_uri).

use std::path::{Component, Path, PathBuf};

use biscuit_file::ResolutionFailure;
use simplified_schema::resolve::SchemaTargetFormat;

use crate::workspace::file_path_to_uri;

/// The first sentence both missing-file hovers share.
const MISSING_FILE_LEAD: &str =
    "This file reference is a valid file reference but the file does not exist yet in the filesystem.";

/// `text` with every character Markdown could read as syntax
/// backslash-escaped, and line breaks turned into spaces, so it reads as the
/// literal text inside a sentence or a link label.
pub(crate) fn markdown_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\r' => {}
            '\n' => escaped.push(' '),
            '\\' | '`' | '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '&' | '~' | '|' | '!' => {
                escaped.push('\\');
                escaped.push(character);
            }
            other => escaped.push(other),
        }
    }
    escaped
}

/// `text` as an inline code span, its fence one backtick longer than the
/// longest backtick run inside it.
pub(crate) fn code_span(text: &str) -> String {
    let text = text.replace(['\r', '\n'], " ");
    let fence = "`".repeat(longest_backtick_run(&text) + 1);
    // CommonMark strips one space from each side of a padded span, so a
    // backtick at either end cannot merge with the fence.
    if fence.len() > 1 {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

/// The backtick fence for a code block holding `lines`: at least three, and
/// longer than any backtick run in them.
pub(crate) fn fence_for(lines: &[String]) -> String {
    let longest = lines.iter().map(|line| longest_backtick_run(line)).max().unwrap_or(0);
    "`".repeat((longest + 1).max(3))
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|character| character != '`').map(str::len).max().unwrap_or(0)
}

/// How `path` is named to the reader: relative to `repository_root` when it
/// is inside it, otherwise absolute; `/`-separated either way.
pub(crate) fn display_path(path: &Path, repository_root: Option<&Path>) -> String {
    if let Some(relative) = repository_root.and_then(|root| path.strip_prefix(root).ok())
        && relative.components().next().is_some()
    {
        return relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/");
    }
    path.to_string_lossy().replace('\\', "/")
}

/// A Markdown link from `label` (already Markdown) to the file `path`, with
/// `(` and `)` percent-encoded so they cannot end the destination. Only the
/// label when the path has no file URI.
pub(crate) fn markdown_link(label: &str, path: &Path) -> String {
    match file_path_to_uri(path) {
        Some(uri) => format!("[{label}]({})", uri.as_str().replace('(', "%28").replace(')', "%29")),
        None => label.to_string(),
    }
}

/// `path`'s display label in a code span, linked to the file.
fn code_link(path: &Path, repository_root: Option<&Path>) -> String {
    markdown_link(&code_span(&display_path(path, repository_root)), path)
}

/// A selected schema file that is an accepted, resolvable target.
pub(crate) struct ValidTarget<'a> {
    pub(crate) path: &'a Path,
    pub(crate) format: SchemaTargetFormat,
    pub(crate) description: Option<&'a str>,
    /// At most the first ten lines of the definition.
    pub(crate) preview: &'a [String],
    pub(crate) total_lines: usize,
}

/// The hover of a `$schema` value whose file is a valid schema: its
/// description, the definition's first lines, and where it resolves.
pub(crate) fn valid_schema_target(target: &ValidTarget<'_>, repository_root: Option<&Path>) -> String {
    let mut markdown = String::new();
    if let Some(description) = target.description.filter(|description| !description.trim().is_empty()) {
        markdown.push_str(description);
        markdown.push_str("\n\n");
    }
    let language = match target.format {
        SchemaTargetFormat::Yaml => "yaml",
        SchemaTargetFormat::Json => "json",
    };
    let fence = fence_for(target.preview);
    markdown.push_str("Schema Definition:\n\n");
    markdown.push_str(&format!("{fence}{language}\n"));
    for line in target.preview {
        markdown.push_str(line);
        markdown.push('\n');
    }
    markdown.push_str(&fence);
    if target.total_lines > target.preview.len() {
        markdown.push_str(&format!(
            "\n\n*Showing **{}** of **{}** lines*",
            target.preview.len(),
            target.total_lines
        ));
    }
    let label = markdown_escape(&display_path(target.path, repository_root));
    markdown.push_str(&format!("\n\nResolves to {}", markdown_link(&label, target.path)));
    markdown
}

/// The hover of a `$schema` value whose file exists but is not an accepted
/// schema format.
pub(crate) fn not_a_schema_target(path: &Path, repository_root: Option<&Path>) -> String {
    format!(
        "The file {} exists but it is **not** a valid schema definition!",
        code_link(path, repository_root)
    )
}

/// The hover of a `$schema` value whose JSON file declares a dialect other
/// than Draft 2020-12.
pub(crate) fn unsupported_dialect_target(path: &Path, repository_root: Option<&Path>, declared: &str) -> String {
    format!(
        "The file {} declares the unsupported JSON Schema dialect {}. Only {} is supported.",
        code_link(path, repository_root),
        code_span(declared),
        code_span(simplified_schema::DRAFT_2020_12_DIALECT),
    )
}

/// The hover of a `$schema` value whose file could not be used for any other
/// reason: malformed, unreadable, an invalid grammar or dependency, a cycle.
pub(crate) fn unusable_schema_target(path: &Path, repository_root: Option<&Path>, error: &str) -> String {
    format!(
        "The file {} could not be used as a schema: {}",
        code_link(path, repository_root),
        markdown_escape(error)
    )
}

/// A `$schema` reference no probed location holds.
pub(crate) struct MissingReference<'a> {
    pub(crate) reference: &'a str,
    /// The schema-root lookup found a file of this name beside the document,
    /// which the bare name does not name; this is how to name it.
    pub(crate) sibling_suggestion: Option<&'a str>,
    /// A bare name looked up like any other file reference because no schema
    /// root folder exists.
    pub(crate) without_roots: bool,
    pub(crate) attempted: &'a [PathBuf],
    pub(crate) creation: Option<&'a Path>,
}

/// The hover of a `$schema` reference that resolves nowhere: what was
/// searched, in order, and the path following the link opens.
pub(crate) fn missing_schema_reference(missing: &MissingReference<'_>, repository_root: Option<&Path>) -> String {
    let reference = code_span(missing.reference);
    let mut markdown = match (missing.sibling_suggestion, missing.without_roots) {
        (Some(suggestion), _) => format!(
            "{reference} is not in any schema root, but a file with that name is beside this document. \
             Write {} to reference it.",
            code_span(suggestion)
        ),
        (None, true) => {
            format!("{reference} was looked up like any other file reference (no schema root folder exists).")
        }
        (None, false) => format!("{reference} does not match a schema file."),
    };
    if !missing.attempted.is_empty() {
        markdown.push_str("\n\nSearched, in order:\n");
        for path in missing.attempted {
            markdown.push_str(&format!("\n- {}", code_span(&display_path(path, repository_root))));
        }
    }
    if let Some(creation) = missing.creation {
        markdown.push_str(&format!(
            "\n\nMissing: {} does not exist yet; following the link opens this path.",
            code_span(&display_path(creation, repository_root))
        ));
    }
    markdown
}

/// The hover of a `$schema` reference that cannot be resolved: `reason` is
/// the file-reference error.
pub(crate) fn unresolvable_schema_reference(reference: &str, reason: &str) -> String {
    format!("{} could not be resolved: {}", code_span(reference), markdown_escape(reason))
}

/// The hover of a remote `$schema` reference.
pub(crate) fn remote_schema_reference(reference: &str) -> String {
    format!("{} is a remote reference. Remote schemas are not fetched.", code_span(reference))
}

/// The hover of a `$schema` reference whose schema roots could not be read.
pub(crate) fn unreadable_schema_roots(reference: &str, error: &str) -> String {
    format!(
        "{} could not be resolved because the schema roots could not be read: {}",
        code_span(reference),
        markdown_escape(error)
    )
}

/// The hover of a reference in a document without a file-resolution
/// context.
pub(crate) fn no_resolution_context(value: &str) -> String {
    format!("This document has no file-resolution context, so {} is not resolved.", code_span(value))
}

/// The hover of a `file` value that `FileReference::new` rejects.
pub(crate) fn invalid_file_reference(value: &str, reason: &str) -> String {
    format!("{} is not a valid file reference: {}", code_span(value), markdown_escape(reason))
}

/// The hover of a `file` value whose file exists.
pub(crate) fn found_file(path: &Path, repository_root: Option<&Path>) -> String {
    format!("Resolves to {}", code_link(path, repository_root))
}

/// How the property a missing `file` value belongs to treats existence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Eagerness {
    /// The file need not exist, even at run time.
    Lazy,
    /// Every `file` arm is eager: the file must exist at run time.
    Eager,
    /// Several types remain possible and a `file` arm is eager.
    EagerFileArm,
}

/// The hover of a well-formed `file` value whose file does not exist;
/// `property` is the dotted path of the property it belongs to.
pub(crate) fn missing_file(property: &str, eagerness: Eagerness) -> String {
    let property = code_span(property);
    match eagerness {
        Eagerness::Lazy => format!(
            "{MISSING_FILE_LEAD} Because {property} is not defined as being **eager** then the file reference \
             is not required to exist even at run time."
        ),
        Eagerness::Eager => format!(
            "{MISSING_FILE_LEAD} The {property} is defined as being an **eager** file reference so that means \
             the file reference must refer to a real file at runtime."
        ),
        Eagerness::EagerFileArm => format!(
            "{MISSING_FILE_LEAD} The {property} has an **eager** `file` arm, so when this value is read as \
             that file reference it must refer to a real file at runtime."
        ),
    }
}

/// The hover of a `file` value that could not be checked: a missing anchor,
/// an I/O failure, or a remote URL. `None` for the other failure classes,
/// which have their own hovers.
pub(crate) fn unchecked_file(failure: ResolutionFailure) -> Option<&'static str> {
    Some(match failure {
        ResolutionFailure::MissingContext => {
            "This file reference needs context this document does not have (a home folder, repository, or \
             package), so it cannot be resolved."
        }
        ResolutionFailure::Io => "The filesystem could not be checked for this file reference (a permission or I/O failure).",
        ResolutionFailure::UnsupportedRemote => {
            "This is a remote URL. It is a well-formed file reference, but the editor never fetches remote content \
             and has not checked whether it exists."
        }
        ResolutionFailure::InvalidReference | ResolutionFailure::NoMatch => return None,
    })
}

#[cfg(test)]
mod tests;
