//! The seam where a generated reference becomes text.
//!
//! [`try_portable_string`] is a presentation helper: it converts lossily and
//! turns a Unix backslash into a slash. A generated reference must instead
//! read back as exactly the names it was built from, in exactly the reference
//! form intended, so every spelling produced here is re-parsed and compared
//! before it is returned.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::path_identity::PathIdentity;
use crate::file_reference::parse;
use crate::file_reference::{FileReferenceKind, TemplateSegment};
use crate::try_portable_string;

/// Why a target cannot be written as the requested reference text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextRejection {
    /// A name is not valid Unicode, so no reference text can spell it; this
    /// holds for an absolute spelling too.
    Unrenderable,
    /// A Windows path with no portable absolute spelling: a device or verbatim
    /// path that cannot be reduced and is not a legacy UNC path.
    NoPortableSpelling,
    /// The text would parse back to different names: a Unix name containing
    /// `\`, a literal verbatim `.` or `..`, or a Windows name that changes
    /// meaning without its `\\?\` prefix.
    ChangesComponents,
    /// The text does not parse back as the intended reference form: `{{…}}`
    /// in a name read as interpolation, a leading sigil in a bare name, or an
    /// anchor that requires a payload given none.
    GrammarMismatch,
}

/// What precedes the rendered names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lead<'a> {
    /// `./names`, or `../`-hops followed by names.
    Relative { parent_hops: usize },
    /// The names alone, read relative to the current directory.
    Bare,
    /// `~/names`.
    Home,
    /// `&names`.
    RepositoryRoot,
    /// `^names`.
    RepositoryScoped,
    /// `@names`.
    Magic,
    /// `{{NAME}}/names`.
    Env(&'a str),
}

impl Lead<'_> {
    fn expected_kind(self) -> FileReferenceKind {
        match self {
            Self::Relative { .. } => FileReferenceKind::ExplicitRelative,
            Self::Bare | Self::Env(_) => FileReferenceKind::ImplicitRelative,
            Self::Home => FileReferenceKind::Home,
            Self::RepositoryRoot => FileReferenceKind::RepositoryRoot,
            Self::RepositoryScoped => FileReferenceKind::RepositoryScoped,
            Self::Magic => FileReferenceKind::Magic,
        }
    }

    fn spell(self, tail: &str) -> String {
        let joined = |lead: &str| {
            if tail.is_empty() {
                lead.trim_end_matches('/').to_string()
            } else {
                format!("{lead}{tail}")
            }
        };
        match self {
            Self::Relative { parent_hops: 0 } => joined("./"),
            Self::Relative { parent_hops } => joined(&"../".repeat(parent_hops)),
            Self::Bare => tail.to_string(),
            Self::Home => joined("~/"),
            Self::RepositoryRoot => format!("&{tail}"),
            Self::RepositoryScoped => format!("^{tail}"),
            Self::Magic => format!("@{tail}"),
            Self::Env(name) => joined(&format!("{{{{{name}}}}}/")),
        }
    }
}

/// Render `names` after `lead` as reference text that parses back to the same
/// names in the intended form.
///
/// ## Errors
///
/// Returns the first [`TextRejection`] that applies, checked in the order the
/// variants are declared.
pub(crate) fn render_reference(lead: Lead<'_>, names: &[OsString]) -> Result<String, TextRejection> {
    if names.iter().any(|name| name.to_str().is_none()) {
        return Err(TextRejection::Unrenderable);
    }
    if cfg!(windows)
        && !names
            .iter()
            .filter_map(|name| name.to_str())
            .all(survives_without_verbatim_prefix)
    {
        return Err(TextRejection::ChangesComponents);
    }

    let joined: PathBuf = names.iter().collect();
    let tail = try_portable_string(&joined).ok_or(TextRejection::ChangesComponents)?;
    // Compared with the names themselves, not with an identity of `joined`:
    // that identity would collapse a literal verbatim `..` just as the
    // rendered text does, and so prove nothing.
    let reparsed = PathIdentity::new(Path::new(&tail));
    if !reparsed.is_unanchored() || reparsed.components() != names {
        return Err(TextRejection::ChangesComponents);
    }

    let text = lead.spell(&tail);
    check_grammar(&text, lead.expected_kind(), lead)?;
    Ok(text)
}

/// Render an absolute `path` as reference text.
///
/// A path with a portable spelling is written with `/` separators; a Windows
/// legacy UNC path keeps its native spelling, which the parser accepts.
///
/// ## Errors
///
/// [`TextRejection::Unrenderable`] for a non-Unicode path, before any other
/// check; [`TextRejection::NoPortableSpelling`] for a device or unreducible
/// verbatim path; [`TextRejection::ChangesComponents`] when the text would
/// split or merge names; [`TextRejection::GrammarMismatch`] when it would not
/// parse back as a literal absolute path.
pub(crate) fn render_absolute(path: &Path) -> Result<String, TextRejection> {
    let native = path.to_str().ok_or(TextRejection::Unrenderable)?;
    let text = match try_portable_string(path) {
        Some(text) => text,
        None if is_legacy_unc(path) => native.to_string(),
        None => return Err(TextRejection::NoPortableSpelling),
    };
    if PathIdentity::new(Path::new(&text)) != PathIdentity::new(path) {
        return Err(TextRejection::ChangesComponents);
    }
    check_grammar(&text, FileReferenceKind::Absolute, Lead::Bare)?;
    Ok(text)
}

/// Parse `text` and require the expected kind, no recursive modifier, and no
/// interpolation beyond the lead's own variable.
fn check_grammar(text: &str, expected: FileReferenceKind, lead: Lead<'_>) -> Result<(), TextRejection> {
    let parsed = parse::parse(text).map_err(|_| TextRejection::GrammarMismatch)?;
    if parsed.recursive || parsed.kind.public_kind() != expected {
        return Err(TextRejection::GrammarMismatch);
    }
    let mut variables = parsed
        .kind
        .template()
        .segments
        .iter()
        .filter_map(|segment| match segment {
            TemplateSegment::EnvVar(name) => Some(name.as_str()),
            TemplateSegment::Literal(_) => None,
        });
    let expected_variable = match lead {
        Lead::Env(name) => Some(name),
        _ => None,
    };
    if variables.next() != expected_variable || variables.next().is_some() {
        return Err(TextRejection::GrammarMismatch);
    }
    Ok(())
}

#[cfg(windows)]
fn is_legacy_unc(path: &Path) -> bool {
    use std::path::{Component, Prefix};
    matches!(
        path.components().next(),
        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::UNC(..))
    )
}

/// Off Windows [`try_portable_string`] never declines, so this is unreachable.
#[cfg(not(windows))]
fn is_legacy_unc(_path: &Path) -> bool {
    false
}

/// Whether a Windows name means the same thing written without `\\?\`.
///
/// A literal `.`/`..`, a reserved DOS device name, a trailing dot or space, a
/// character Win32 forbids, or a name over 255 UTF-16 units is re-read as
/// something else once the prefix is gone. The rules mirror `dunce`'s own
/// component checks, minus its whole-path length test, so a long descendant
/// below an anchor stays renderable. Compiled on every host so the rules are
/// tested everywhere; applied only on Windows.
pub(crate) fn survives_without_verbatim_prefix(name: &str) -> bool {
    if name == "." || name == ".." {
        return false;
    }
    // UTF-16 units, not `char`s: one astral character is two units.
    if name.encode_utf16().count() > 255 {
        return false;
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return false;
    }
    if name
        .bytes()
        .any(|byte| matches!(byte, 0..=31 | b'<' | b'>' | b':' | b'"' | b'/' | b'\\' | b'|' | b'?' | b'*'))
    {
        return false;
    }
    !is_reserved_dos_name(name)
}

/// `CON`, `con.txt`, and `con.. .txt` are all the DOS console device.
fn is_reserved_dos_name(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "AUX", "NUL", "PRN", "CON", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or(name).trim_end_matches(' ');
    RESERVED.iter().any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

#[cfg(test)]
mod tests;
