//! Test 5 of `2026-10-03-visibility-and-kind`: the v1/v2 adapter preserves
//! kind, visibility, `is_static`, `is_static_storage`, container kind and
//! signature presence for every fixture symbol, in both directions; and,
//! since `2026-10-03-symbol-filters-and-reports`, scope, the in-file
//! qualified name, its namespace root and `async`.

use std::path::PathBuf;

use tree_hugger::{
    CodeRange, FunctionSignature, Modifier, ParameterInfo, ProgrammingLanguage, SchemaVersion,
    SymbolInfo, SymbolKind, SymbolKindV2, SymbolRecord, SymbolScope, TreeFile, TypeMetadata,
    Visibility, VisibilityLevel,
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
        scope: SymbolScope::Item,
        qualified_name: name.to_string(),
        is_namespace_rooted: false,
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
        is_async: false,
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

fn has_async(record: &SymbolRecord) -> bool {
    record.modifiers.iter().any(|modifier| matches!(modifier, Modifier::Async))
}

#[test]
fn scope_qualified_name_and_async_survive_both_conversions() {
    for scope in [SymbolScope::Item, SymbolScope::Member, SymbolScope::Local] {
        for (is_async, is_namespace_rooted) in [(false, false), (true, false), (false, true)] {
            let mut input = symbol("run", SymbolKind::Method);
            input.scope = scope;
            input.qualified_name = "com.example.Widget.run".to_string();
            input.is_namespace_rooted = is_namespace_rooted;
            input.is_async = is_async;
            let (record, back) = round_trip(input);

            let context = format!("{scope:?} async={is_async} rooted={is_namespace_rooted}");
            assert_eq!(record.scope, scope, "{context}: record scope");
            assert_eq!(record.identity.qualified_name, "com.example.Widget.run", "{context}");
            assert_eq!(record.identity.is_namespace_rooted, is_namespace_rooted, "{context}");
            assert_eq!(has_async(&record), is_async, "{context}: Modifier::Async");
            assert_eq!(back.scope, scope, "{context}: restored scope");
            assert_eq!(back.qualified_name, "com.example.Widget.run", "{context}");
            assert_eq!(back.is_namespace_rooted, is_namespace_rooted, "{context}");
            assert_eq!(back.is_async, is_async, "{context}: restored is_async");
        }
    }
}

/// The record ID hashes the in-file qualified name, not the container: two
/// same-named methods of different types get different IDs, and the same
/// qualified name gets the same ID whatever the container field says.
#[test]
fn the_record_id_derives_from_the_qualified_name() {
    let id_of = |qualified: &str, container: Option<&str>| {
        let mut input = symbol("area", SymbolKind::Method);
        input.qualified_name = qualified.to_string();
        input.container_name = container.map(str::to_string);
        SymbolRecord::from(input).id
    };
    assert_ne!(id_of("Circle.area", Some("Circle")), id_of("Square.area", Some("Square")));
    assert_eq!(id_of("Circle.area", Some("Circle")), id_of("Circle.area", None));

    // A v1 symbol built by hand with no qualified name is qualified by its
    // name alone.
    let mut bare = symbol("area", SymbolKind::Method);
    bare.qualified_name = String::new();
    assert_eq!(SymbolRecord::from(bare).identity.qualified_name, "area");
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
    input.scope = SymbolScope::Member;
    input.qualified_name = "Example.Widget.Widget".to_string();
    input.is_namespace_rooted = true;
    input.is_async = true;
    let record = SymbolRecord::from(input);
    assert_eq!(record.schema_version, SchemaVersion::CURRENT);

    let first = serde_json::to_string(&record).expect("serialize");
    let parsed: SymbolRecord = serde_json::from_str(&first).expect("deserialize");
    let back = SymbolInfo::try_from(parsed.clone()).expect("convert");
    let second = serde_json::to_string(&SymbolRecord::from(back.clone())).expect("re-serialize");
    let reparsed: SymbolRecord = serde_json::from_str(&second).expect("re-deserialize");

    for record in [&parsed, &reparsed] {
        assert_eq!(record.schema_version, SchemaVersion::CURRENT);
        assert_eq!(record.kind, SymbolKindV2::Constructor);
        assert_eq!(record.identity.container_kind, Some(SymbolKindV2::Class));
        assert_eq!(record.visibility.level, VisibilityLevel::ProtectedInternal);
        assert!(has_static_storage(record) && !has_static(record));
        assert_eq!(record.scope, SymbolScope::Member);
        assert_eq!(record.identity.qualified_name, "Example.Widget.Widget");
        assert!(record.identity.is_namespace_rooted && has_async(record));
        assert_eq!(record.id, parsed.id, "the ID survives the round trip");
    }
    assert_eq!(back.visibility, Some(Visibility::ProtectedInternal));
    assert!(back.is_static_storage && !back.is_static);
    assert!(back.is_async && back.is_namespace_rooted && back.scope == SymbolScope::Member);

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
    for key in ["visibility", "is_static", "is_static_storage", "is_async", "is_namespace_rooted"] {
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

crate::language_cases::language_tests!(adapter_agrees_with_extraction_for_every_fixture_symbol => check_adapter_agrees_with_extraction_for_every_fixture_symbol);

/// Both directions agree on every symbol extracted from every fixture; the
/// matrix tests below pin the expected values.
fn check_adapter_agrees_with_extraction_for_every_fixture_symbol(language: ProgrammingLanguage) {
    let mut checked = 0usize;
    for path in crate::symbols_fixtures::fixtures_for(language, false) {
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
            assert_eq!(info.scope, record.scope, "{label}: scope");
            assert_eq!(info.qualified_name, record.identity.qualified_name, "{label}: qualified name");
            assert_eq!(info.is_namespace_rooted, record.identity.is_namespace_rooted, "{label}: rooted");
            assert_eq!(info.is_async, has_async(&record), "{label}: async");
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
            assert_eq!(record.scope, symbol.scope, "{label}: scope");
            assert_eq!(record.identity.qualified_name, symbol.qualified_name, "{label}: qualified name");
            assert_eq!(record.identity.is_namespace_rooted, symbol.is_namespace_rooted, "{label}: rooted");
            assert_eq!(has_async(&record), symbol.is_async, "{label}: async");
        }
    }

    assert!(checked >= crate::language_cases::corpus_minimums(language).symbols, "only {checked} fixture records checked");
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

crate::language_cases::language_tests!(kind_matrix_survives_both_conversions => check_kind_matrix_survives_both_conversions);

/// Test 5 with explicit expected values: every kind-matrix row survives
/// `SymbolRecord -> SymbolInfo` and `SymbolInfo -> SymbolRecord`.
fn check_kind_matrix_survives_both_conversions(language: ProgrammingLanguage) {
    for (language, rows) in rows_by_language(KIND_MATRIX, |row| row.0).into_iter().filter(|(row_language, _)| *row_language == language) {
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

crate::language_cases::language_tests!(visibility_matrix_survives_both_conversions => check_visibility_matrix_survives_both_conversions);

/// Test 5: every visibility-matrix row survives both conversions.
fn check_visibility_matrix_survives_both_conversions(language: ProgrammingLanguage) {
    for (language, rows) in rows_by_language(VISIBILITY_MATRIX, |row| row.0).into_iter().filter(|(row_language, _)| *row_language == language) {
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

crate::language_cases::language_tests!(static_matrix_survives_both_conversions => check_static_matrix_survives_both_conversions);

/// Test 5: every static-matrix row survives both conversions.
fn check_static_matrix_survives_both_conversions(language: ProgrammingLanguage) {
    for (language, rows) in rows_by_language(STATIC_MATRIX, |row| row.0).into_iter().filter(|(row_language, _)| *row_language == language) {
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

crate::language_cases::language_tests!(every_fixture_symbol_round_trips_through_the_record => check_every_fixture_symbol_round_trips_through_the_record);

/// Every fixture symbol survives `SymbolInfo -> SymbolRecord -> SymbolInfo`
/// unchanged in the fields this fix promises, its signature and its type
/// metadata (compared in their serialized form).
fn check_every_fixture_symbol_round_trips_through_the_record(language: ProgrammingLanguage) {
    let mut with_metadata = 0usize;
    let mut lossy_aliases = Vec::new();
    for path in crate::symbols_fixtures::fixtures_for(language, false) {
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

    assert!(with_metadata >= crate::language_cases::corpus_minimums(language).metadata, "{language}: type metadata corpus shrank");
    assert_eq!(
        lossy_aliases,
        if language == ProgrammingLanguage::C {
            vec!["c.c: TagAlias", "c.c: AnonymousStructAlias", "c.c: SameName"]
        } else { Vec::new() },
        "aliases whose v1 field metadata the record cannot carry"
    );
}

/// One edit to a serialized record.
type Edit = dyn Fn(&mut serde_json::Value);

/// Reader shapes of the record fields this feature adds, from a real record
/// with one edit per cell. `metrics` is optional as a facet (absent = not
/// requested) but each metric inside it is required and nullable (`null` =
/// unsupported); the other fields are required.
#[test]
fn new_record_fields_read_per_shape() {
    let fixture = crate::symbols_fixtures::primary_fixture(ProgrammingLanguage::Python).expect("fixture");
    let record = TreeFile::new(&fixture)
        .expect("parse")
        .symbol_records()
        .expect("records")
        .into_iter()
        .find(|record| record.identity.name == "instance_method")
        .expect("instance_method record");
    let control = serde_json::to_value(&record).expect("serialize");
    assert!(control.get("metrics").is_none(), "an unrequested facet is not written");
    let read = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = control.clone();
        edit(&mut value);
        serde_json::from_value::<SymbolRecord>(value)
    };

    // Control: the unedited record reads back with no metrics.
    let unedited = read(&|_| {}).expect("control");
    assert!(unedited.metrics.is_none());
    assert_eq!(unedited.scope, SymbolScope::Member);
    assert_eq!(unedited.identity.qualified_name, "PlainWidget.instance_method");

    // `metrics`: present values, and `null` metrics inside a present facet.
    let both = read(&|value| value["metrics"] = serde_json::json!({ "parameters": 0, "has_docs": false, "lines": 3, "cyclomatic": 1 }))
        .expect("measured facet");
    let metrics = both.metrics.expect("facet");
    assert_eq!(
        (metrics.parameters, metrics.has_docs, metrics.lines, metrics.cyclomatic),
        (Some(0), Some(false), Some(3), Some(1))
    );
    let unsupported =
        read(&|value| value["metrics"] = serde_json::json!({ "parameters": null, "has_docs": null, "lines": null, "cyclomatic": null }))
            .expect("unsupported metrics")
            .metrics
            .expect("facet");
    assert_eq!(
        (unsupported.parameters, unsupported.has_docs, unsupported.lines, unsupported.cyclomatic),
        (None, None, None, None)
    );
    // An empty body span is zero lines, distinct from `null`.
    let empty_body = read(&|value| value["metrics"] = serde_json::json!({ "parameters": 0, "has_docs": false, "lines": 0, "cyclomatic": 1 }))
        .expect("empty body")
        .metrics
        .expect("facet");
    assert_eq!(empty_body.lines, Some(0));

    // `cyclomatic`: the largest `u32` reads; it is never capped or wrapped.
    let largest = read(&|value| {
        value["metrics"] = serde_json::json!({ "parameters": 0, "has_docs": false, "lines": 3, "cyclomatic": 4_294_967_295_u64 })
    })
    .expect("largest cyclomatic")
    .metrics
    .expect("facet");
    assert_eq!(largest.cyclomatic, Some(u32::MAX));

    let rejected: [(&str, Box<Edit>); 27] = [
        ("metrics null", Box::new(|value| value["metrics"] = serde_json::Value::Null)),
        ("metrics wrong type", Box::new(|value| value["metrics"] = serde_json::json!(123))),
        ("metrics empty", Box::new(|value| value["metrics"] = serde_json::json!({}))),
        ("metric absent", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "lines": 1, "cyclomatic": 1 }))),
        ("metric wrong type", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": "one", "has_docs": true, "lines": 1, "cyclomatic": 1 }))),
        ("metric unknown", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 3, "bogus": 3, "cyclomatic": 1 }))),
        // A schema 4.0 facet (no `lines`) is never read as a computed `null`.
        ("lines absent", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "cyclomatic": 1 }))),
        ("lines wrong type", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": "3", "cyclomatic": 1 }))),
        ("lines fraction", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 3.5, "cyclomatic": 1 }))),
        ("lines negative", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": -1, "cyclomatic": 1 }))),
        ("lines one wrong element", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": [3, "x"], "cyclomatic": 1 }))),
        // A schema 4.2 facet (no `cyclomatic`) is never read as a computed `null`.
        ("cyclomatic absent", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1 }))),
        ("cyclomatic string", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": "3" }))),
        ("cyclomatic list", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": [3] }))),
        ("cyclomatic bool", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": true }))),
        ("cyclomatic negative", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": -1 }))),
        ("cyclomatic fraction", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": 1.5 }))),
        ("cyclomatic beyond u32", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": 4_294_967_296_u64 }))),
        ("cyclomatic empty object", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": {} }))),
        ("cyclomatic empty list", Box::new(|value| value["metrics"] = serde_json::json!({ "parameters": 1, "has_docs": true, "lines": 1, "cyclomatic": [] }))),
        ("scope absent", Box::new(|value| { value.as_object_mut().expect("record").remove("scope"); })),
        ("scope null", Box::new(|value| value["scope"] = serde_json::Value::Null)),
        ("scope unknown", Box::new(|value| value["scope"] = serde_json::json!(""))),
        ("qualified name absent", Box::new(|value| { value["identity"].as_object_mut().expect("identity").remove("qualified_name"); })),
        ("qualified name empty", Box::new(|value| value["identity"]["qualified_name"] = serde_json::json!(""))),
        ("rooted absent", Box::new(|value| { value["identity"].as_object_mut().expect("identity").remove("is_namespace_rooted"); })),
        ("is_test absent", Box::new(|value| { value["visibility"].as_object_mut().expect("visibility").remove("is_test"); })),
    ];
    for (label, edit) in &rejected {
        assert!(read(edit.as_ref()).is_err(), "{label} must not read");
    }

    // A repeated metric key is rejected, not last-wins.
    let text = serde_json::to_string(&control).expect("serialize");
    let duplicated =
        text.replacen("{", "{\"metrics\":{\"parameters\":1,\"parameters\":2,\"has_docs\":true,\"lines\":1,\"cyclomatic\":1},", 1);
    assert!(serde_json::from_str::<SymbolRecord>(&duplicated).is_err(), "duplicate metric");
    let duplicated_lines =
        text.replacen("{", "{\"metrics\":{\"parameters\":1,\"has_docs\":true,\"lines\":1,\"lines\":2,\"cyclomatic\":1},", 1);
    assert!(serde_json::from_str::<SymbolRecord>(&duplicated_lines).is_err(), "duplicate lines");
    let duplicated_cyclomatic = text.replacen(
        "{",
        "{\"metrics\":{\"parameters\":1,\"has_docs\":true,\"lines\":1,\"cyclomatic\":1,\"cyclomatic\":2},",
        1,
    );
    assert!(serde_json::from_str::<SymbolRecord>(&duplicated_cyclomatic).is_err(), "duplicate cyclomatic");
    // Trailing content after a valid record is rejected.
    let measured = text.replacen("{", "{\"metrics\":{\"parameters\":1,\"has_docs\":true,\"lines\":1,\"cyclomatic\":1},", 1);
    assert!(serde_json::from_str::<SymbolRecord>(&measured).is_ok(), "control for the trailing cell");
    assert!(serde_json::from_str::<SymbolRecord>(&format!("{measured} trailing")).is_err(), "trailing content");
}

crate::symbols_relations::declaration_fact_group_tests!(
    record_only_declaration_facts_are_lost_through_the_older_model => record_only_facts_are_lost
);

/// The older `SymbolInfo` model has no slot for the declaration facts that
/// records now carry, so `SymbolRecord -> SymbolInfo -> SymbolRecord` is
/// lossy for them, by design: structured generic parameters and
/// constraints, `extends`/`implements`, raw attributes, the `metrics` facet
/// (and so `lines` and `cyclomatic`: never invented), and every modifier other than `Static`,
/// `StaticStorage` and `Async` come back empty. Pinned over every
/// declaration-fact source, one test per corpus group; each loss must
/// actually occur in the Rust group, so a fixed adapter flips this
/// test.
fn record_only_facts_are_lost(group: usize) {
    let dir = tempfile::TempDir::new().expect("temp");
    let mut lost = [0usize; 7];
    for (label, path) in crate::symbols_relations::write_declaration_fact_group(dir.path(), group) {
        let index = TreeFile::new(&path).expect("parse").symbol_index_v2().expect("complete index");
        for record in index.symbols {
            let cell = format!("{label}: `{}`", record.identity.qualified_name);
            let back = SymbolRecord::from(SymbolInfo::try_from(record.clone()).expect("named record"));
            let kept: Vec<Modifier> = record
                .modifiers
                .iter()
                .filter(|modifier| matches!(modifier, Modifier::Static | Modifier::StaticStorage | Modifier::Async))
                .cloned()
                .collect();
            assert_eq!(back.modifiers, kept, "{cell}: modifiers");
            assert!(back.type_info.generic_params.is_empty(), "{cell}: generic parameters");
            assert!(back.type_info.constraints.is_empty(), "{cell}: constraints");
            assert!(back.relations.extends.is_empty(), "{cell}: extends");
            assert!(back.relations.implements.is_empty(), "{cell}: implements");
            assert!(back.relations.unclassified_bases.is_empty(), "{cell}: unclassified bases");
            assert!(back.attributes.is_empty(), "{cell}: attributes");
            assert!(back.metrics.is_none(), "{cell}: metrics");

            lost[0] += usize::from(!record.type_info.generic_params.is_empty());
            lost[1] += usize::from(!record.type_info.constraints.is_empty());
            lost[2] += usize::from(!record.relations.extends.is_empty() || !record.relations.implements.is_empty());
            lost[3] += usize::from(!record.attributes.is_empty());
            lost[4] += usize::from(record.metrics.as_ref().is_some_and(|metrics| metrics.lines.is_some()));
            lost[5] += usize::from(back.modifiers.len() < record.modifiers.len());
            lost[6] += usize::from(record.metrics.as_ref().is_some_and(|metrics| metrics.cyclomatic.is_some()));
        }
    }
    if group != crate::symbols_relations::RUST_FACT_GROUP {
        return;
    }
    for (fact, count) in ["generic parameters", "constraints", "relations", "attributes", "lines", "modifiers", "cyclomatic"]
        .iter()
        .zip(lost)
    {
        assert!(count > 0, "no record lost its {fact}: the adapter now carries it; update the docs and this test");
    }
}
