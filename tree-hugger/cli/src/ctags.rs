//! Extended-subset ctags output for declaration navigation.

use std::io::{self, Write};
use std::path::Path;

use tree_hugger::{ProgrammingLanguage, SymbolKindV2, SymbolRecord};

use crate::report::portable_relative;

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

/// Writes one record. Unsafe values are skipped and returned as a diagnostic.
pub(crate) fn write_record(out: &mut dyn Write, record: &SymbolRecord, root: &Path) -> io::Result<Option<(String, usize)>> {
    if record.visibility.is_synthetic || record.source.file_path.as_os_str().is_empty() {
        return Ok(None);
    }
    let path = portable_relative(&record.source.file_path, root);
    let line = record.source.declaration_span.start.line as usize;
    let container = record.relations.container.as_ref().and_then(|owner| owner.qualified_name.as_deref());
    if has_controls(&record.identity.name) || container.is_some_and(has_controls) || has_controls(&path) {
        return Ok(Some((path, line)));
    }
    let name = record.identity.name.replace('\\', "\\\\");
    let path = path.replace('\\', "/");
    write!(out, "{name}\t{path}\t{line};\"\tkind:{}", kind(record.language, record.kind))?;
    if let Some(container) = container {
        write!(out, "\tclass:{}", container.replace('\\', "\\\\"))?;
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
                assert!(skipped.is_some(), "{field} containing U+{:04X} was accepted", control as u32);
            }
        }
    }
}
