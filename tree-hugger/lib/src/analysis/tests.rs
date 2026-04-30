use std::fs;

use tempfile::TempDir;

use crate::TreeFile;
use crate::analysis::{OwnerSymbolIndex, analyze_file};
use crate::shared::{
    AnalysisPass, DocsFacet, IdentityFacet, ProgrammingLanguage, ProvenanceFacet, SchemaVersion,
    SemanticFacet, SourceFacet, SymbolId, SymbolKindData, SymbolKindV2, SymbolRecord, TextPoint,
    TextSpan, TypeFacet, VisibilityFacet,
};

#[test]
fn pipeline_marks_completed_passes() {
    let dir = TempDir::new().expect("tempdir");
    let file = dir.path().join("sample.rs");
    fs::write(
        &file,
        "/// says hello\nfn hello(name: &str) -> String { format!(\"hi {name}\") }",
    )
    .expect("write fixture");

    let tree_file = TreeFile::new(&file).expect("parse");
    let index = analyze_file(
        &tree_file,
        &[
            AnalysisPass::Parse,
            AnalysisPass::Bind,
            AnalysisPass::Semantic,
            AnalysisPass::Docs,
        ],
    )
    .expect("analyze");

    assert!(index.completed_passes.contains(&AnalysisPass::Parse));
    assert!(index.completed_passes.contains(&AnalysisPass::Bind));
    assert!(index.completed_passes.contains(&AnalysisPass::Semantic));
    assert!(index.completed_passes.contains(&AnalysisPass::Docs));
    assert!(!index.symbols.is_empty());
}

#[test]
fn owner_symbol_index_picks_smallest_containing_span_with_equal_starts() {
    let symbols = vec![
        test_symbol("module", 0, 200),
        test_symbol("function", 0, 120),
        test_symbol("block", 0, 80),
        test_symbol("inner", 20, 40),
        test_symbol("sibling", 100, 140),
    ];
    let index = OwnerSymbolIndex::new(&symbols);

    assert_eq!(index.find(24, 30), Some(3));
    assert_eq!(index.find(70, 75), Some(2));
    assert_eq!(index.find(90, 95), Some(1));
    assert_eq!(index.find(180, 190), Some(0));
    assert_eq!(index.find(205, 210), None);
}

fn test_symbol(name: &str, start_byte: u32, end_byte: u32) -> SymbolRecord {
    SymbolRecord {
        schema_version: SchemaVersion::V2_0,
        id: SymbolId(name.to_string()),
        language: ProgrammingLanguage::Rust,
        kind: SymbolKindV2::Function,
        identity: IdentityFacet {
            name: name.to_string(),
            display_name: name.to_string(),
            qualified_name: None,
            module_path: None,
            stable_key: name.to_string(),
        },
        source: SourceFacet {
            declaration_span: TextSpan {
                start: TextPoint::default(),
                end: TextPoint::default(),
                start_byte,
                end_byte,
            },
            ..SourceFacet::default()
        },
        visibility: VisibilityFacet::default(),
        docs: DocsFacet::default(),
        attributes: Vec::new(),
        modifiers: Vec::new(),
        type_info: TypeFacet::default(),
        relations: Default::default(),
        semantics: SemanticFacet::default(),
        provenance: ProvenanceFacet {
            extractor: "test".to_string(),
            extractor_version: "test".to_string(),
            parse_pass: AnalysisPass::Parse.as_str().to_string(),
            created_at_epoch_ms: 0,
            updated_at_epoch_ms: 0,
        },
        kind_data: SymbolKindData::Unknown,
        extensions: Default::default(),
    }
}
