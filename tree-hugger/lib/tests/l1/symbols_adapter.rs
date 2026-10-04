//! Test 5 of `2026-10-03-visibility-and-kind`: the v1/v2 adapter preserves
//! kind, visibility, `is_static`, `is_static_storage`, container kind and
//! signature presence for every fixture symbol, in both directions.

use std::path::PathBuf;

use tree_hugger::{
    CodeRange, FunctionSignature, Modifier, ParameterInfo, ProgrammingLanguage, SchemaVersion,
    SymbolInfo, SymbolKind, SymbolKindV2, SymbolRecord, TreeFile, TypeMetadata, Visibility,
    VisibilityLevel,
};

use crate::symbols_fixtures::{Extracted, all_fixtures, display_name, extract_inline, extract_primary};
use crate::symbols_kinds::{KIND_MATRIX, rows_by_language};
use crate::symbols_static::STATIC_MATRIX;
use crate::symbols_visibility::VISIBILITY_MATRIX;

/// Every v1 kind and the v2 kind it must map to, both ways.
const KIND_PAIRS: [(SymbolKind, SymbolKindV2); 19] = [
    (SymbolKind::Function, SymbolKindV2::Function),
    (SymbolKind::Method, SymbolKindV2::Method),
    (SymbolKind::Constructor, SymbolKindV2::Constructor),
    (SymbolKind::Type, SymbolKindV2::Type),
    (SymbolKind::TypeAlias, SymbolKindV2::TypeAlias),
    (SymbolKind::Class, SymbolKindV2::Class),
    (SymbolKind::Interface, SymbolKindV2::Interface),
    (SymbolKind::Enum, SymbolKindV2::Enum),
    (SymbolKind::EnumVariant, SymbolKindV2::EnumVariant),
    (SymbolKind::Trait, SymbolKindV2::Trait),
    (SymbolKind::Module, SymbolKindV2::Module),
    (SymbolKind::Namespace, SymbolKindV2::Namespace),
    (SymbolKind::Variable, SymbolKindV2::Variable),
    (SymbolKind::Parameter, SymbolKindV2::Parameter),
    (SymbolKind::Field, SymbolKindV2::Field),
    (SymbolKind::Property, SymbolKindV2::Property),
    (SymbolKind::Macro, SymbolKindV2::Macro),
    (SymbolKind::Constant, SymbolKindV2::Constant),
    (SymbolKind::Unknown, SymbolKindV2::Unknown),
];

/// Every v1 visibility (with `None` for Unknown) and its v2 level.
const VISIBILITY_PAIRS: [(Option<Visibility>, VisibilityLevel); 10] = [
    (Some(Visibility::Public), VisibilityLevel::Public),
    (Some(Visibility::Protected), VisibilityLevel::Protected),
    (Some(Visibility::Private), VisibilityLevel::Private),
    (Some(Visibility::Internal), VisibilityLevel::Internal),
    (Some(Visibility::Package), VisibilityLevel::Package),
    (Some(Visibility::Crate), VisibilityLevel::Crate),
    (Some(Visibility::ModulePrivate), VisibilityLevel::ModulePrivate),
    (Some(Visibility::ProtectedInternal), VisibilityLevel::ProtectedInternal),
    (Some(Visibility::PrivateProtected), VisibilityLevel::PrivateProtected),
    (None, VisibilityLevel::Unknown),
];

fn symbol(name: &str, kind: SymbolKind) -> SymbolInfo {
    SymbolInfo {
        name: name.to_string(),
        kind,
        range: CodeRange {
            start_line: 3,
            start_column: 5,
            end_line: 3,
            end_column: 9,
            start_byte: 40,
            end_byte: 44,
        },
        language: ProgrammingLanguage::TypeScript,
        file: PathBuf::from("src/widget.ts"),
        container_name: None,
        container_kind: None,
        doc_comment: None,
        visibility: None,
        is_static: false,
        is_static_storage: false,
        signature: None,
        type_metadata: None,
    }
}

fn round_trip(symbol: SymbolInfo) -> (SymbolRecord, SymbolInfo) {
    let record = SymbolRecord::from(symbol);
    let back = SymbolInfo::try_from(record.clone()).expect("adapter accepts a named record");
    (record, back)
}

fn has_static(record: &SymbolRecord) -> bool {
    record.modifiers.iter().any(|modifier| matches!(modifier, Modifier::Static))
}

fn has_static_storage(record: &SymbolRecord) -> bool {
    record.modifiers.iter().any(|modifier| matches!(modifier, Modifier::StaticStorage))
}

#[test]
fn every_kind_survives_both_conversions() {
    for (v1, v2) in KIND_PAIRS {
        let (record, back) = round_trip(symbol("item", v1));
        assert_eq!(record.kind, v2, "v1 {v1:?} -> record kind");
        assert_eq!(back.kind, v1, "record {v2:?} -> v1 kind");
        assert_eq!(SymbolKind::from(v2), v1, "direct v2 {v2:?} -> v1");
    }
}

#[test]
fn constructor_is_function_like_and_type_alias_is_type_like() {
    assert!(SymbolKind::Constructor.is_function());
    assert!(SymbolKind::TypeAlias.is_type());
    assert!(!SymbolKind::Property.is_function() && !SymbolKind::Property.is_type());
    assert!(!SymbolKind::EnumVariant.is_function() && !SymbolKind::EnumVariant.is_type());
}

#[test]
fn every_visibility_level_survives_both_conversions() {
    for (visibility, level) in VISIBILITY_PAIRS {
        for kind in [SymbolKind::Method, SymbolKind::Class, SymbolKind::Field, SymbolKind::Constant] {
            let mut input = symbol("item", kind);
            input.visibility = visibility;
            let (record, back) = round_trip(input);
            assert_eq!(record.visibility.level, level, "{kind:?} {visibility:?} -> level");
            assert_eq!(back.visibility, visibility, "{kind:?} {level:?} -> visibility");
        }
    }
}

#[test]
fn compound_csharp_levels_have_their_own_labels() {
    let labels: Vec<String> = VISIBILITY_PAIRS
        .iter()
        .filter_map(|(visibility, _)| visibility.map(|value| value.to_string()))
        .collect();
    assert_eq!(
        labels,
        [
            "public",
            "protected",
            "private",
            "internal",
            "package",
            "crate",
            "module-private",
            "protected internal",
            "private protected",
        ]
    );
}

#[test]
fn public_level_never_marks_a_record_exported() {
    for kind in [SymbolKind::Function, SymbolKind::Method, SymbolKind::Class] {
        let mut input = symbol("item", kind);
        input.visibility = Some(Visibility::Public);
        let (record, _) = round_trip(input);
        assert_eq!(record.visibility.level, VisibilityLevel::Public);
        assert!(!record.visibility.is_exported, "{kind:?}: access level must not imply export");
    }
}

#[test]
fn static_and_static_storage_are_separate_markers() {
    // (is_static, is_static_storage) for a method, a field, a property and a
    // variable, so both flags are checked on function-like and data kinds.
    for kind in [SymbolKind::Method, SymbolKind::Field, SymbolKind::Property, SymbolKind::Variable] {
        for (is_static, is_static_storage) in [(false, false), (true, false), (false, true)] {
            let mut input = symbol("item", kind);
            input.is_static = is_static;
            input.is_static_storage = is_static_storage;
            let (record, back) = round_trip(input);

            let context = format!("{kind:?} static={is_static} storage={is_static_storage}");
            assert_eq!(has_static(&record), is_static, "{context}: Modifier::Static");
            assert_eq!(
                has_static_storage(&record),
                is_static_storage,
                "{context}: Modifier::StaticStorage"
            );
            assert_eq!(back.is_static, is_static, "{context}: restored is_static");
            assert_eq!(
                back.is_static_storage, is_static_storage,
                "{context}: restored is_static_storage"
            );
            if kind.is_function() {
                let signature = back.signature.expect("method keeps a signature");
                assert_eq!(signature.is_static, is_static, "{context}: signature copy");
            }
        }
    }
}

#[test]
fn static_field_reaches_field_data() {
    let mut input = symbol("count", SymbolKind::Field);
    input.is_static = true;
    let (record, _) = round_trip(input);
    match record.kind_data {
        tree_hugger::SymbolKindData::Field(data) => assert!(data.is_static),
        other => panic!("field record carries {other:?}"),
    }
}

#[test]
fn container_kind_survives_both_conversions() {
    for (v1, v2) in KIND_PAIRS {
        let mut input = symbol("member", SymbolKind::Method);
        input.container_name = Some("Box".to_string());
        input.container_kind = Some(v1);
        let (record, back) = round_trip(input);
        assert_eq!(record.identity.container_kind, Some(v2), "container {v1:?} -> record");
        assert_eq!(record.identity.container_name.as_deref(), Some("Box"));
        assert_eq!(back.container_kind, Some(v1), "container {v2:?} -> v1");
        assert_eq!(back.container_name.as_deref(), Some("Box"));
    }

    let (record, back) = round_trip(symbol("free", SymbolKind::Function));
    assert_eq!(record.identity.container_kind, None);
    assert_eq!(back.container_kind, None);
}

#[test]
fn function_like_kinds_always_carry_a_signature() {
    for kind in [SymbolKind::Function, SymbolKind::Method, SymbolKind::Constructor] {
        // No signature at all on the way in: a parameterless declaration.
        let (_, back) = round_trip(symbol("tick", kind));
        let signature = back.signature.unwrap_or_else(|| panic!("{kind:?} lost its signature"));
        assert!(signature.parameters.is_empty(), "{kind:?}: no invented parameters");
        assert_eq!(signature.return_type, None, "{kind:?}: no invented return type");

        // An empty signature on the way in.
        let mut input = symbol("tick", kind);
        input.signature = Some(FunctionSignature::new());
        let (_, back) = round_trip(input);
        assert!(back.signature.is_some(), "{kind:?}: empty signature dropped");
    }

    for kind in [
        SymbolKind::Field,
        SymbolKind::Property,
        SymbolKind::Variable,
        SymbolKind::Type,
        SymbolKind::TypeAlias,
        SymbolKind::EnumVariant,
        SymbolKind::Constant,
    ] {
        let (_, back) = round_trip(symbol("value", kind));
        assert!(back.signature.is_none(), "{kind:?} must not gain a signature");
    }
}

#[test]
fn signature_content_is_preserved() {
    let mut input = symbol("resize", SymbolKind::Method);
    input.signature = Some(FunctionSignature {
        parameters: vec![
            ParameterInfo::with_type("width", "number"),
            ParameterInfo::new("height"),
        ],
        return_type: Some("void".to_string()),
        is_static: false,
    });
    let (_, back) = round_trip(input);
    let signature = back.signature.expect("signature");
    let names: Vec<&str> = signature.parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["width", "height"]);
    assert_eq!(signature.parameters[0].type_annotation.as_deref(), Some("number"));
    assert_eq!(signature.return_type.as_deref(), Some("void"));
}

/// Write, read, convert, write again: the persisted record keeps every field
/// this fix adds, and an absent `container_kind` stays absent.
#[test]
fn serialized_record_round_trips_the_new_fields() {
    let mut input = symbol("Widget", SymbolKind::Constructor);
    input.container_name = Some("Widget".to_string());
    input.container_kind = Some(SymbolKind::Class);
    input.visibility = Some(Visibility::ProtectedInternal);
    input.is_static_storage = true;
    let record = SymbolRecord::from(input);
    assert_eq!(record.schema_version, SchemaVersion::V4_0);

    let first = serde_json::to_string(&record).expect("serialize");
    let parsed: SymbolRecord = serde_json::from_str(&first).expect("deserialize");
    let back = SymbolInfo::try_from(parsed.clone()).expect("convert");
    let second = serde_json::to_string(&SymbolRecord::from(back.clone())).expect("re-serialize");
    let reparsed: SymbolRecord = serde_json::from_str(&second).expect("re-deserialize");

    for record in [&parsed, &reparsed] {
        assert_eq!(record.schema_version, SchemaVersion::V4_0);
        assert_eq!(record.kind, SymbolKindV2::Constructor);
        assert_eq!(record.identity.container_kind, Some(SymbolKindV2::Class));
        assert_eq!(record.visibility.level, VisibilityLevel::ProtectedInternal);
        assert!(has_static_storage(record) && !has_static(record));
    }
    assert_eq!(back.visibility, Some(Visibility::ProtectedInternal));
    assert!(back.is_static_storage && !back.is_static);

    let mut free = SymbolRecord::from(symbol("free", SymbolKind::Function));
    free.identity.container_kind = None;
    let json = serde_json::to_value(&free).expect("serialize");
    assert!(
        json["identity"].get("container_kind").is_none(),
        "absent container kind is omitted: {}",
        json["identity"]
    );
}

/// v1 JSON written before this fix has no `visibility`, `is_static` or
/// `is_static_storage`; reading it gives Unknown and `false`.
#[test]
fn symbol_info_json_defaults_and_omissions() {
    let mut input = symbol("item", SymbolKind::Field);
    let json = serde_json::to_value(&input).expect("serialize");
    for key in ["visibility", "is_static", "is_static_storage"] {
        assert!(json.get(key).is_none(), "{key} omitted when Unknown/false: {json}");
    }
    let parsed: SymbolInfo = serde_json::from_value(json).expect("deserialize");
    assert_eq!(parsed.visibility, None);
    assert!(!parsed.is_static && !parsed.is_static_storage);

    input.visibility = Some(Visibility::Crate);
    input.is_static = true;
    let json = serde_json::to_value(&input).expect("serialize");
    assert_eq!(json["visibility"], "Crate");
    assert_eq!(json["is_static"], true);
    let parsed: SymbolInfo = serde_json::from_value(json).expect("deserialize");
    assert_eq!(parsed.visibility, Some(Visibility::Crate));
    assert!(parsed.is_static);
}

/// Both directions agree on every symbol extracted from every fixture; the
/// matrix tests below pin the expected values.
#[test]
fn adapter_agrees_with_extraction_for_every_fixture_symbol() {
    let mut checked = 0usize;
    for path in all_fixtures() {
        let name = display_name(&path);
        let tree_file = TreeFile::new(&path).expect("load fixture");

        for record in tree_file.symbol_records().expect("records") {
            let label = format!("{name}: {} ({:?})", record.identity.name, record.kind);
            let info = SymbolInfo::try_from(record.clone()).expect("convert record");
            assert_eq!(SymbolKindV2::from(info.kind), record.kind, "{label}: kind");
            assert_eq!(
                VisibilityLevel::from(info.visibility),
                record.visibility.level,
                "{label}: visibility"
            );
            assert_eq!(info.is_static, has_static(&record), "{label}: is_static");
            assert_eq!(
                info.is_static_storage,
                has_static_storage(&record),
                "{label}: is_static_storage"
            );
            assert_eq!(
                info.container_kind.map(SymbolKindV2::from),
                record.identity.container_kind,
                "{label}: container kind"
            );
            assert_eq!(info.signature.is_some(), info.kind.is_function(), "{label}: signature");
            checked += 1;
        }

        for symbol in tree_file.symbols().expect("symbols") {
            let label = format!("{name}: {} ({:?})", symbol.name, symbol.kind);
            let record = SymbolRecord::from(symbol.clone());
            assert_eq!(record.kind, SymbolKindV2::from(symbol.kind), "{label}: kind");
            assert_eq!(
                record.visibility.level,
                VisibilityLevel::from(symbol.visibility),
                "{label}: visibility"
            );
            assert_eq!(has_static(&record), symbol.is_static, "{label}: is_static");
            assert_eq!(
                has_static_storage(&record),
                symbol.is_static_storage,
                "{label}: is_static_storage"
            );
            assert_eq!(
                record.identity.container_kind,
                symbol.container_kind.map(SymbolKindV2::from),
                "{label}: container kind"
            );
        }
    }
    assert!(checked > 100, "only {checked} fixture records checked");
}

/// Records name the extraction rules that produced them, so a cached record
/// can be traced to its revision.
#[test]
fn records_stamp_the_extractor_revision() {
    let expected = format!(
        "{}+rev.{}",
        env!("CARGO_PKG_VERSION"),
        tree_hugger::EXTRACTOR_REVISION
    );
    assert!(!tree_hugger::EXTRACTOR_REVISION.is_empty());
    for path in all_fixtures().into_iter().take(3) {
        let records = tree_hugger::TreeFile::new(&path)
            .expect("parse")
            .symbol_index_v2()
            .expect("index")
            .symbols;
        assert!(!records.is_empty());
        for record in records {
            assert_eq!(record.provenance.extractor_version, expected, "{}", display_name(&path));
        }
    }
}

/// Converts the extracted record through the adapter and the extracted symbol
/// back into a record, so a matrix row is checked on both conversions.
fn converted(extracted: &Extracted, name: &str, container: Option<&str>) -> (SymbolInfo, SymbolRecord) {
    let info = SymbolInfo::try_from(extracted.record(name, container).clone())
        .unwrap_or_else(|error| panic!("{}: `{name}` record -> SymbolInfo: {error:?}", extracted.label));
    let record = SymbolRecord::from(extracted.symbol(name, container).clone());
    (info, record)
}

/// Test 5 with explicit expected values: every kind-matrix row survives
/// `SymbolRecord -> SymbolInfo` and `SymbolInfo -> SymbolRecord`.
#[test]
fn kind_matrix_survives_both_conversions() {
    for (language, rows) in rows_by_language(KIND_MATRIX, |row| row.0) {
        let extracted = extract_primary(language);
        for (_, name, container, kind, container_kind) in rows {
            let label = format!("{}: `{name}`", extracted.label);
            let (info, record) = converted(&extracted, name, *container);
            assert_eq!(info.kind, *kind, "{label}: kind after record -> SymbolInfo");
            assert_eq!(info.container_kind, *container_kind, "{label}: container kind after record -> SymbolInfo");
            assert_eq!(info.container_name.as_deref(), *container, "{label}: container after record -> SymbolInfo");
            assert_eq!(record.kind, SymbolKindV2::from(*kind), "{label}: kind after SymbolInfo -> record");
            assert_eq!(
                record.identity.container_kind,
                container_kind.map(SymbolKindV2::from),
                "{label}: container kind after SymbolInfo -> record"
            );
            assert_eq!(
                info.signature.is_some(),
                kind.is_function(),
                "{label}: signature presence after record -> SymbolInfo"
            );
        }
    }
}

/// Test 5: every visibility-matrix row survives both conversions.
#[test]
fn visibility_matrix_survives_both_conversions() {
    for (language, rows) in rows_by_language(VISIBILITY_MATRIX, |row| row.0) {
        let extracted = extract_primary(language);
        for (_, name, container, visibility) in rows {
            let label = format!("{}: `{name}`", extracted.label);
            let (info, record) = converted(&extracted, name, *container);
            assert_eq!(info.visibility, *visibility, "{label}: visibility after record -> SymbolInfo");
            assert_eq!(
                record.visibility.level,
                VisibilityLevel::from(*visibility),
                "{label}: visibility after SymbolInfo -> record"
            );
        }
    }
}

/// Test 5: every static-matrix row survives both conversions.
#[test]
fn static_matrix_survives_both_conversions() {
    for (language, rows) in rows_by_language(STATIC_MATRIX, |row| row.0) {
        let extracted = extract_primary(language);
        for (_, name, container, is_static, is_static_storage) in rows {
            let label = format!("{}: `{name}`", extracted.label);
            let (info, record) = converted(&extracted, name, *container);
            assert_eq!(info.is_static, *is_static, "{label}: is_static after record -> SymbolInfo");
            assert_eq!(
                info.is_static_storage, *is_static_storage,
                "{label}: is_static_storage after record -> SymbolInfo"
            );
            assert_eq!(has_static(&record), *is_static, "{label}: Modifier::Static after SymbolInfo -> record");
            assert_eq!(
                has_static_storage(&record),
                *is_static_storage,
                "{label}: Modifier::StaticStorage after SymbolInfo -> record"
            );
        }
    }
}

/// `(language, name, container, kind, parameter names)` in a primary fixture.
type SignatureCase = (ProgrammingLanguage, &'static str, Option<&'static str>, SymbolKind, &'static [&'static str]);

/// Test 5: parameterless constructors, methods and functions keep a present,
/// empty signature through both conversions; parameters are not dropped.
#[test]
fn fixture_signatures_survive_both_conversions() {
    let cases: &[SignatureCase] = &[
        (ProgrammingLanguage::TypeScript, "constructor", Some("ModuleWidget"), SymbolKind::Constructor, &[]),
        (ProgrammingLanguage::JavaScript, "constructor", Some("ModuleWidget"), SymbolKind::Constructor, &[]),
        (ProgrammingLanguage::JavaScript, "instanceMethod", Some("ModuleWidget"), SymbolKind::Method, &[]),
        (ProgrammingLanguage::JavaScript, "constructor", Some("ExportedWidget"), SymbolKind::Constructor, &["initialValue"]),
        (ProgrammingLanguage::TypeScript, "exportedFunction", None, SymbolKind::Function, &[]),
        (ProgrammingLanguage::Rust, "public_function", None, SymbolKind::Function, &[]),
    ];
    for (language, name, container, kind, parameters) in cases {
        let extracted = extract_primary(*language);
        let label = format!("{}: `{name}`", extracted.label);
        let (info, record) = converted(&extracted, name, *container);
        assert_eq!(info.kind, *kind, "{label}: kind");
        let signature = info
            .signature
            .unwrap_or_else(|| panic!("{label}: record -> SymbolInfo lost the signature"));
        let names: Vec<&str> = signature.parameters.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, *parameters, "{label}: parameters after record -> SymbolInfo");

        let back = SymbolInfo::try_from(record).expect("convert back");
        let signature = back
            .signature
            .unwrap_or_else(|| panic!("{label}: SymbolInfo -> record -> SymbolInfo lost the signature"));
        let names: Vec<&str> = signature.parameters.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, *parameters, "{label}: parameters after SymbolInfo -> record");
    }
}

/// A C++ function returning a function pointer keeps its own parameters,
/// with their types, default and variadic tail, through both conversions.
#[test]
fn returned_callback_signature_survives_both_conversions() {
    let extracted = extract_inline(
        "int (*callback_result(char **argv, int count = 2, ...))(long returned) { return 0; }\n",
        "cpp",
    );
    let label = "inline .cpp: `callback_result`";
    let expected = extracted
        .symbol("callback_result", None)
        .signature
        .clone()
        .unwrap_or_else(|| panic!("{label}: no signature"));
    let names: Vec<&str> = expected.parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["argv", "count", "..."], "{label}: extracted parameters");

    let (info, record) = converted(&extracted, "callback_result", None);
    let back = SymbolInfo::try_from(record).expect("convert back");
    for (path, symbol) in [("record -> SymbolInfo", info), ("SymbolInfo -> record -> SymbolInfo", back)] {
        assert_eq!(
            serde_json::to_value(symbol.signature.as_ref().map(|s| &s.parameters)).expect("serialize"),
            serde_json::to_value(Some(&expected.parameters)).expect("serialize"),
            "{label}: parameters after {path}"
        );
    }
}

/// The type metadata a `SymbolInfo -> SymbolRecord -> SymbolInfo` round trip
/// can return. Two losses are known and pinned here so a fix flips the test:
/// `TypeData` has no slot for type parameters, and `TypeAliasData` holds only
/// the aliased type, so a C `typedef struct {…} Alias` loses its fields.
fn metadata_after_round_trip(symbol: &SymbolInfo) -> Option<TypeMetadata> {
    if symbol.kind == SymbolKind::TypeAlias {
        return None;
    }
    symbol.type_metadata.clone().map(|mut metadata| {
        metadata.type_parameters.clear();
        metadata
    })
}

/// Every fixture symbol survives `SymbolInfo -> SymbolRecord -> SymbolInfo`
/// unchanged in the fields this fix promises, its signature and its type
/// metadata (compared in their serialized form).
#[test]
fn every_fixture_symbol_round_trips_through_the_record() {
    let mut with_metadata = 0usize;
    let mut lossy_aliases = Vec::new();
    for path in all_fixtures() {
        let name = display_name(&path);
        for symbol in TreeFile::new(&path).expect("load fixture").symbols().expect("symbols") {
            let label = format!("{name}: {} ({:?})", symbol.name, symbol.kind);
            let back = SymbolInfo::try_from(SymbolRecord::from(symbol.clone()))
                .unwrap_or_else(|error| panic!("{label}: {error:?}"));
            assert_eq!(back.kind, symbol.kind, "{label}: kind");
            assert_eq!(back.visibility, symbol.visibility, "{label}: visibility");
            assert_eq!(back.is_static, symbol.is_static, "{label}: is_static");
            assert_eq!(back.is_static_storage, symbol.is_static_storage, "{label}: is_static_storage");
            assert_eq!(back.container_name, symbol.container_name, "{label}: container");
            assert_eq!(back.container_kind, symbol.container_kind, "{label}: container kind");
            assert_eq!(
                serde_json::to_value(&back.signature).expect("serialize"),
                serde_json::to_value(&symbol.signature).expect("serialize"),
                "{label}: signature"
            );
            assert_eq!(
                serde_json::to_value(&back.type_metadata).expect("serialize"),
                serde_json::to_value(metadata_after_round_trip(&symbol)).expect("serialize"),
                "{label}: type metadata"
            );
            with_metadata += usize::from(symbol.type_metadata.is_some());
            if symbol.kind == SymbolKind::TypeAlias && symbol.type_metadata.is_some() {
                lossy_aliases.push(format!("{name}: {}", symbol.name));
            }
        }
    }
    assert!(with_metadata > 20, "only {with_metadata} symbols carried type metadata");
    assert_eq!(
        lossy_aliases,
        ["c.c: TagAlias", "c.c: AnonymousStructAlias", "c.c: SameName"],
        "aliases whose v1 field metadata the record cannot carry"
    );
}
