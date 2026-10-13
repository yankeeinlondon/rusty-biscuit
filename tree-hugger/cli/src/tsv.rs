//! Tab-separated declaration export.

use std::io::{self, Write};

use tree_hugger::{SymbolKindV2, SymbolRecord, VisibilityLevel};

use crate::output::SignatureMode;

pub(crate) const HEADER: &str = "file\tline\tcolumn\tkind\tname\tqualified_name\tvisibility\tsignature\n";

/// Writes one row per record. Each `file` cell is the record's own
/// `source.file_path`, which callers have already made scan-root-relative
/// (`report::trim_record`): a prelude alias is analyzed with the prelude
/// file but declared at its target.
pub(crate) fn write_rows(
    out: &mut dyn Write,
    records: &[&SymbolRecord],
    signatures: SignatureMode,
) -> io::Result<()> {
    for record in records {
        let span = &record.source.declaration_span.start;
        let cells = [
            record.source.file_path.to_string_lossy().into_owned(),
            span.line.to_string(),
            span.column.to_string(),
            kind_name(record.kind).to_owned(),
            record.identity.name.clone(),
            record.identity.qualified_name.clone(),
            visibility_name(record.visibility.level).unwrap_or_default().to_owned(),
            crate::render::signature_cell(record, signatures),
        ];
        for (index, cell) in cells.iter().enumerate() {
            if index != 0 {
                out.write_all(b"\t")?;
            }
            out.write_all(crate::export::tsv_cell(cell).as_bytes())?;
        }
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn kind_name(kind: SymbolKindV2) -> &'static str {
    match kind {
        SymbolKindV2::Function => "function", SymbolKindV2::Method => "method",
        SymbolKindV2::Constructor => "constructor", SymbolKindV2::Type => "type",
        SymbolKindV2::Class => "class", SymbolKindV2::Interface => "interface",
        SymbolKindV2::Enum => "enum", SymbolKindV2::Trait => "trait",
        SymbolKindV2::Module => "module", SymbolKindV2::Namespace => "namespace",
        SymbolKindV2::Field => "field", SymbolKindV2::Property => "property",
        SymbolKindV2::Variable => "variable", SymbolKindV2::Constant => "constant",
        SymbolKindV2::Parameter => "parameter", SymbolKindV2::TypeAlias => "type_alias",
        SymbolKindV2::EnumVariant => "enum_variant", SymbolKindV2::Macro => "macro",
        SymbolKindV2::Unknown => "unknown",
    }
}

fn visibility_name(level: VisibilityLevel) -> Option<&'static str> {
    Some(match level {
        VisibilityLevel::Public => "public", VisibilityLevel::Protected => "protected",
        VisibilityLevel::Private => "private", VisibilityLevel::Internal => "internal",
        VisibilityLevel::Package => "package", VisibilityLevel::Crate => "crate",
        VisibilityLevel::ModulePrivate => "module_private",
        VisibilityLevel::ProtectedInternal => "protected_internal",
        VisibilityLevel::PrivateProtected => "private_protected",
        VisibilityLevel::Unknown => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_json_snake_case_spellings() {
        let kind = serde_json::to_value(SymbolKindV2::EnumVariant).unwrap();
        assert_eq!(kind_name(SymbolKindV2::EnumVariant), "enum_variant");
        assert_eq!(kind.as_str(), Some("EnumVariant"));
        assert_eq!(visibility_name(VisibilityLevel::Unknown), None);
    }
}
