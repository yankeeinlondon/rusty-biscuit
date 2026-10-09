//! Extended-subset ctags output for declaration navigation.

use std::io::{self, Write};
use std::path::Path;
use tree_hugger::{ProgrammingLanguage, SymbolKindV2, SymbolRecord, lossless_portable_relative, portable_relative};

pub(crate) const HEADER: &str = "!_TAG_FILE_FORMAT\t2\t/extended format/\n!_TAG_FILE_SORTED\t0\t/0=unsorted/\n";

/// Hug kind names are the published tags `kind:` values. Every language is
/// listed so adding a supported language requires reviewing this contract.
fn kind(language: ProgrammingLanguage, kind: SymbolKindV2) -> &'static str {
    let _language = match language {
        ProgrammingLanguage::Rust | ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript |
        ProgrammingLanguage::Go | ProgrammingLanguage::Python | ProgrammingLanguage::Java |
        ProgrammingLanguage::Php | ProgrammingLanguage::Perl | ProgrammingLanguage::Bash |
        ProgrammingLanguage::Zsh | ProgrammingLanguage::C | ProgrammingLanguage::Cpp |
        ProgrammingLanguage::CSharp | ProgrammingLanguage::Swift | ProgrammingLanguage::Scala |
        ProgrammingLanguage::Lua => language,
    };
    match kind {
        SymbolKindV2::Function => "function",
        SymbolKindV2::Method => "method",
        SymbolKindV2::Constructor => "constructor",
        SymbolKindV2::Type => "type",
        SymbolKindV2::Class => "class",
        SymbolKindV2::Interface => "interface",
        SymbolKindV2::Enum => "enum",
        SymbolKindV2::Trait => "trait",
        SymbolKindV2::Module => "module",
        SymbolKindV2::Namespace => "namespace",
        SymbolKindV2::Field => "field",
        SymbolKindV2::Property => "property",
        SymbolKindV2::Variable => "variable",
        SymbolKindV2::Constant => "constant",
        SymbolKindV2::Parameter => "parameter",
        SymbolKindV2::TypeAlias => "type_alias",
        SymbolKindV2::EnumVariant => "enum_variant",
        SymbolKindV2::Macro => "macro",
        SymbolKindV2::Unknown => "unknown",
    }
}

fn has_controls(value: &str) -> bool {
    value.chars().any(char::is_control)
}

/// Why a record has no tag line: a value the reader would not read back as
/// written. Every other byte of the name, path and `class:` value is written
/// raw: Neovim decodes no escape in any field, so `\\` would stay two
/// backslashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unsupported {
    /// A control character in the name, path or container, which would end
    /// the field or the line.
    Control,
    /// A name starting with `!_`: readers take the line for a pseudo-tag and
    /// drop it (or, as `!_TAG_FILE_SORTED`, reconfigure the whole file).
    PseudoTagName,
    /// A path Neovim expands before opening it, so it could open another
    /// file or run a command.
    ExpandedPath,
    /// A path whose route from the scan root holds a name that is not valid
    /// Unicode (Linux bytes, a Windows unpaired surrogate). The UTF-8 tag
    /// file could only hold a replacement spelling, which names a missing or
    /// different file.
    UnrepresentablePath,
}

impl Unsupported {
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::Control => "unsupported control character",
            Self::PseudoTagName => "pseudo-tag name starting with !_",
            Self::ExpandedPath => "file name Neovim would expand",
            Self::UnrepresentablePath => "file name that is not valid Unicode",
        }
    }
}

/// Whether Neovim rewrites `path` when it opens a tag. A tag file name
/// holding a wildcard goes through environment, home and shell expansion:
/// `$` before a name or shell parameter (`$x`, `$$`, `${x}`) is replaced, a
/// backtick runs a command, a leading `~` is a home directory, and a `[…]`
/// class can match a different existing file instead of itself. `*`, `?`,
/// `{…}`, `%`, `#`, an inner `~` and a `$` before `.`, space, `/` or the end
/// reach the file as written (Neovim 0.12, headless reader test).
fn reader_expands(path: &str) -> bool {
    let mut chars = path.chars().peekable();
    if path.starts_with('~') {
        return true;
    }
    while let Some(character) = chars.next() {
        match character {
            '`' | '[' => return true,
            '$' if chars.peek().is_some_and(|next| !matches!(next, '.' | ' ' | '/')) => return true,
            _ => {}
        }
    }
    false
}

/// Writes one record, or returns why it was skipped (with its displayed
/// path and line) without writing anything. `record` is the native record,
/// never one `report::trim_record` projected: the tag path is the record's
/// route from `scan_root`, decided here from the native `source.file_path`
/// so that a name the UTF-8 file cannot hold is refused instead of replaced.
/// The route is written as is: on Unix it keeps a literal `\` in a file
/// name, and on Windows `\` cannot occur in a name.
pub(crate) fn write_record(
    out: &mut dyn Write,
    record: &SymbolRecord,
    scan_root: &Path,
) -> io::Result<Option<(String, usize, Unsupported)>> {
    if record.visibility.is_synthetic || record.source.file_path.as_os_str().is_empty() {
        return Ok(None);
    }
    match lossless_portable_relative(&record.source.file_path, scan_root) {
        Some(path) => write_tag(out, record, path),
        None => Ok(Some((
            portable_relative(&record.source.file_path, scan_root),
            record.source.declaration_span.start.line as usize,
            Unsupported::UnrepresentablePath,
        ))),
    }
}

/// Writes `record`'s tag at `path`, or returns why it was skipped.
fn write_tag(out: &mut dyn Write, record: &SymbolRecord, path: String) -> io::Result<Option<(String, usize, Unsupported)>> {
    let line = record.source.declaration_span.start.line as usize;
    let name = &record.identity.name;
    let container = record.relations.container.as_ref().and_then(|owner| owner.qualified_name.as_deref());
    let unsupported = if has_controls(name) || container.is_some_and(has_controls) || has_controls(&path) {
        Some(Unsupported::Control)
    } else if name.starts_with("!_") {
        Some(Unsupported::PseudoTagName)
    } else if reader_expands(&path) {
        Some(Unsupported::ExpandedPath)
    } else {
        None
    };
    if let Some(reason) = unsupported {
        return Ok(Some((path, line, reason)));
    }
    write!(out, "{name}\t{path}\t{line};\"\tkind:{}", kind(record.language, record.kind))?;
    if let Some(container) = container {
        write!(out, "\tclass:{container}")?;
    }
    out.write_all(b"\n")?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_hugger::{Capability, SourceFile, SymbolKindV2, SymbolQuery};

    fn method_record() -> (tempfile::TempDir, SymbolRecord) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.rs");
        std::fs::write(&path, "struct Owner;\nimpl Owner { fn run() {} }\n").unwrap();
        let query = SymbolQuery::new().capabilities([Capability::Containment]);
        let records = SourceFile::read(&path).unwrap().query(&query).unwrap().index.symbols;
        let record = records.into_iter().find(|record| record.kind == SymbolKindV2::Method).unwrap();
        assert!(record.relations.container.is_some());
        (dir, record)
    }

    #[test]
    fn kind_mapping_is_exhaustive_and_type_is_never_promoted() {
        assert_eq!(kind(ProgrammingLanguage::Rust, SymbolKindV2::Type), "type");
        assert_eq!(kind(ProgrammingLanguage::Java, SymbolKindV2::Unknown), "unknown");
    }

    #[test]
    fn name_container_and_path_controls_are_rejected_without_emitting_partial_tags() {
        let (dir, base) = method_record();
        for control in ['\t', '\r', '\n', '\u{001b}', '\u{0085}', '\u{009b}'] {
            for field in ["name", "container", "path"] {
                let mut record = base.clone();
                match field {
                    "name" => record.identity.name = format!("bad{control}name"),
                    "container" => record.relations.container.as_mut().unwrap().qualified_name = Some(format!("bad{control}owner")),
                    "path" => record.source.file_path = dir.path().join(format!("bad{control}path.rs")),
                    _ => unreachable!(),
                }
                let mut bytes = Vec::new();
                let skipped = write_record(&mut bytes, &record, dir.path()).unwrap();
                assert!(bytes.is_empty(), "{field} containing U+{:04X} emitted a tag", control as u32);
                assert!(
                    skipped.is_some_and(|(_, _, reason)| reason == Unsupported::Control),
                    "{field} containing U+{:04X} was accepted",
                    control as u32
                );
            }
        }
    }

    /// The tag written for `record` at the already-projected `path`.
    fn written_at(record: &SymbolRecord, path: &str) -> (String, Option<Unsupported>) {
        let mut bytes = Vec::new();
        let skipped = write_tag(&mut bytes, record, path.to_string()).unwrap();
        (String::from_utf8(bytes).unwrap(), skipped.map(|(_, _, reason)| reason))
    }

    fn written(record: &SymbolRecord) -> (String, Option<Unsupported>) {
        written_at(record, "sample.rs")
    }

    /// Neovim reads every field byte for byte (`taglist()` gives a PHP
    /// `Outer\Inner\Account` owner 19 bytes), so a backslash is written once,
    /// never doubled or turned into a separator.
    #[test]
    fn backslashes_in_name_container_and_path_are_written_raw() {
        let (_dir, mut record) = method_record();
        record.identity.name = r"Outer\Inner".to_string();
        record.relations.container.as_mut().unwrap().qualified_name = Some(r"Outer\Inner\Account".to_string());
        let (line, skipped) = written_at(&record, r"back\slash.php");
        assert_eq!(skipped, None);
        assert_eq!(line, "Outer\\Inner\tback\\slash.php\t2;\"\tkind:method\tclass:Outer\\Inner\\Account\n");
        let fields: Vec<&str> = line.trim_end().split('\t').collect();
        assert_eq!(fields[0].len(), 11);
        assert_eq!(fields[4].strip_prefix("class:").unwrap().len(), 19);
    }

    #[test]
    fn pseudo_tag_names_are_skipped() {
        let (_dir, mut record) = method_record();
        for name in ["!_TAG_FILE_SORTED", "!_TAG_X", "!_"] {
            record.identity.name = name.to_string();
            assert_eq!(written(&record), (String::new(), Some(Unsupported::PseudoTagName)), "{name}");
        }
        record.identity.name = "!not_pseudo".to_string();
        assert_eq!(written(&record).1, None);
    }

    /// A route through a name that is not UTF-8 is refused before any
    /// lossy conversion; a synthetic record is omitted before any path check.
    #[cfg(target_os = "linux")]
    #[test]
    fn routes_through_names_that_are_not_utf8_are_skipped() {
        use std::os::unix::ffi::OsStrExt;
        let (dir, mut record) = method_record();
        record.source.file_path = dir.path().join(std::ffi::OsStr::from_bytes(b"bad\xff.rs"));
        let mut bytes = Vec::new();
        let skipped = write_record(&mut bytes, &record, dir.path()).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(skipped, Some(("bad\u{FFFD}.rs".to_string(), 2, Unsupported::UnrepresentablePath)));
        record.visibility.is_synthetic = true;
        assert_eq!(write_record(&mut bytes, &record, dir.path()).unwrap(), None);
        assert!(bytes.is_empty());
    }

    /// Each rejected path made headless Neovim 0.12 open another file, run a
    /// command or fail; each accepted one opened the file as written.
    #[test]
    fn paths_neovim_would_expand_are_skipped_and_the_rest_are_written() {
        let (_dir, record) = method_record();
        for path in [
            "a$x.rs", "$HOME.rs", "a$$.rs", "a$-.rs", "a${x}.rs", "a$1.rs", "a$\u{fc}.rs", "a`echo b`c.rs",
            "a[b].rs", "~/f.rs", "~root/g.rs", "~home.rs",
        ] {
            assert_eq!(written_at(&record, path), (String::new(), Some(Unsupported::ExpandedPath)), "{path}");
        }
        for path in [
            "a$.rs", "end$", "a$ b.rs", "d$/f.rs", "pct%.rs", "hash#.rs", " lead.rs", "star*.rs", "q?.rs",
            "c{d,e}.rs", "d/~/h.rs", "a ~/f.rs", "../outside.rs", r"\lead.rs", r"end\",
        ] {
            let (line, skipped) = written_at(&record, path);
            assert_eq!(skipped, None, "{path}");
            assert_eq!(line.split('\t').nth(1), Some(path), "{path}");
        }
    }
}
