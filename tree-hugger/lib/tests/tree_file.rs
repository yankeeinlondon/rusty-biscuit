use std::path::PathBuf;

use tree_hugger_lib::{ProgrammingLanguage, TreeFile, TreeHuggerError};

#[test]
fn parses_all_fixtures() -> Result<(), TreeHuggerError> {
    let fixtures = vec![
        ("sample.rs", ProgrammingLanguage::Rust, true),
        ("sample.js", ProgrammingLanguage::JavaScript, true),
        ("sample.ts", ProgrammingLanguage::TypeScript, true),
        ("sample.go", ProgrammingLanguage::Go, true),
        ("sample.py", ProgrammingLanguage::Python, true),
        ("Sample.java", ProgrammingLanguage::Java, true),
        ("sample.php", ProgrammingLanguage::Php, true),
        ("sample.pl", ProgrammingLanguage::Perl, false),
        ("sample.sh", ProgrammingLanguage::Bash, true),
        ("sample.zsh", ProgrammingLanguage::Zsh, true),
        ("sample.c", ProgrammingLanguage::C, true),
        ("sample.cpp", ProgrammingLanguage::Cpp, true),
        ("sample.cs", ProgrammingLanguage::CSharp, true),
        ("sample.swift", ProgrammingLanguage::Swift, true),
        ("sample.scala", ProgrammingLanguage::Scala, true),
        ("sample.lua", ProgrammingLanguage::Lua, true),
    ];

    for (file, language, expect_symbols) in fixtures {
        let path = fixture_path(file);
        let tree_file = TreeFile::new(&path)?;
        assert_eq!(tree_file.language, language, "language mismatch for {file}");
        assert!(tree_file.syntax_diagnostics().is_empty());

        let symbols = tree_file.symbols()?;
        if expect_symbols {
            assert!(!symbols.is_empty(), "expected symbols for {file}");
        }
    }

    Ok(())
}

#[test]
fn captures_imports_for_javascript() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("sample.js"))?;
    let imports = tree_file.imported_symbols()?;
    assert!(!imports.is_empty());
    Ok(())
}

#[test]
fn captures_exports_for_javascript() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("sample.js"))?;
    let exports = tree_file.exported_symbols()?;
    assert!(!exports.is_empty(), "should find exported 'greet' function");
    assert_eq!(exports[0].name, "greet");
    Ok(())
}

fn fixture_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(file)
}

// ============================================================================
// Regression tests for function signature and doc comment extraction
// These tests ensure return types, parameters, and doc comments are captured.
// ============================================================================

#[test]
fn extracts_rust_function_return_type() -> Result<(), TreeHuggerError> {
    // Regression test: Rust return types were not being extracted because the
    // code looked for a "return_type" node, but Rust's AST has the type as a
    // direct child after the "->" token.
    let tree_file = TreeFile::new(fixture_path("sample.rs"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    // Check return type
    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("String"),
        "Rust function return type should be extracted"
    );

    // Check parameters
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("&str"),
        "parameter type should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_rust_doc_comments() -> Result<(), TreeHuggerError> {
    // Regression test: Doc comments should be extracted from preceding /// comments
    let tree_file = TreeFile::new(fixture_path("sample.rs"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let doc = greet.doc_comment.as_ref().expect("should have doc comment");
    assert!(
        doc.contains("Greets a person by name"),
        "doc comment should be extracted: {doc}"
    );

    Ok(())
}

#[test]
fn extracts_typescript_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: TypeScript return types should be extracted from type_annotation
    let tree_file = TreeFile::new(fixture_path("sample.ts"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("string"),
        "TypeScript function return type should be extracted"
    );

    // Check that variadic parameters are captured
    let greet_many = symbols
        .iter()
        .find(|s| s.name == "greetMany")
        .expect("should find greetMany function");

    let sig = greet_many.signature.as_ref().expect("should have signature");
    assert!(!sig.parameters.is_empty(), "variadic parameter should be captured");
    assert!(
        sig.parameters[0].is_variadic,
        "parameter should be marked as variadic"
    );
    assert_eq!(sig.parameters[0].name, "names");

    Ok(())
}

#[test]
fn extracts_typescript_doc_comments_through_export() -> Result<(), TreeHuggerError> {
    // Regression test: Doc comments for exported functions should be found by
    // looking at the parent export_statement's siblings
    let tree_file = TreeFile::new(fixture_path("sample.ts"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let doc = greet.doc_comment.as_ref().expect("should have doc comment for exported function");
    assert!(
        doc.contains("Greets a person by name"),
        "doc comment should be extracted through export: {doc}"
    );

    Ok(())
}

#[test]
fn extracts_python_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: Python return types should be extracted from type nodes
    let tree_file = TreeFile::new(fixture_path("sample.py"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("str"),
        "Python function return type should be extracted"
    );

    // Check default value
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert!(
        sig.parameters[0].default_value.is_some(),
        "default value should be captured"
    );

    // Check variadic parameter
    let greet_many = symbols
        .iter()
        .find(|s| s.name == "greet_many")
        .expect("should find greet_many function");

    let sig = greet_many.signature.as_ref().expect("should have signature");
    assert!(!sig.parameters.is_empty(), "variadic parameter should be captured");
    assert!(
        sig.parameters[0].is_variadic,
        "parameter should be marked as variadic"
    );

    Ok(())
}

#[test]
fn extracts_go_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: Go return types and method parameters should be correct
    let tree_file = TreeFile::new(fixture_path("sample.go"))?;
    let symbols = tree_file.symbols()?;

    // Check function return type
    let greet = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find Greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("string"),
        "Go function return type should be extracted"
    );

    // Check method parameters (should NOT include receiver)
    let method_greet = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Method)
        .expect("should find Greet method");

    let sig = method_greet.signature.as_ref().expect("should have signature");
    assert!(!sig.parameters.is_empty(), "method should have parameters");
    // The first parameter should be "name", not the receiver "g"
    assert_eq!(
        sig.parameters[0].name, "name",
        "Go method parameters should not include receiver"
    );

    Ok(())
}

#[test]
fn extracts_go_doc_comments() -> Result<(), TreeHuggerError> {
    // Regression test: Go doc comments should be extracted from // comments
    let tree_file = TreeFile::new(fixture_path("sample.go"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find Greet function");

    let doc = greet.doc_comment.as_ref().expect("should have doc comment");
    assert!(
        doc.contains("greets a person by name"),
        "Go doc comment should be extracted: {doc}"
    );

    Ok(())
}

// ============================================================================
// Regression tests for generics support
// ============================================================================

#[test]
fn extracts_rust_generic_types() -> Result<(), TreeHuggerError> {
    // Regression test: Generic types in parameters and return types should be captured
    let tree_file = TreeFile::new(fixture_path("generics.rs"))?;
    let symbols = tree_file.symbols()?;

    // Check generic return type
    let identity = symbols
        .iter()
        .find(|s| s.name == "identity")
        .expect("should find identity function");
    let sig = identity.signature.as_ref().expect("should have signature");
    assert_eq!(sig.return_type.as_deref(), Some("T"), "generic return type");
    assert_eq!(sig.parameters[0].type_annotation.as_deref(), Some("T"), "generic param type");

    // Check complex generic return type
    let try_parse = symbols
        .iter()
        .find(|s| s.name == "try_parse")
        .expect("should find try_parse function");
    let sig = try_parse.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("Result<T, E>"),
        "complex generic return type"
    );

    // Check impl Trait return type
    let make_iter = symbols
        .iter()
        .find(|s| s.name == "make_iter")
        .expect("should find make_iter function");
    let sig = make_iter.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("impl Iterator<Item = usize>"),
        "impl Trait return type"
    );

    // Check primitive type in parameters (previously broken)
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("usize"),
        "primitive param type should be captured"
    );

    Ok(())
}

#[test]
fn extracts_typescript_generic_types() -> Result<(), TreeHuggerError> {
    // Regression test: TypeScript generics should be captured
    let tree_file = TreeFile::new(fixture_path("generics.ts"))?;
    let symbols = tree_file.symbols()?;

    // Check generic identity function
    let identity = symbols
        .iter()
        .find(|s| s.name == "identity")
        .expect("should find identity function");
    let sig = identity.signature.as_ref().expect("should have signature");
    assert_eq!(sig.return_type.as_deref(), Some("T"), "TS generic return type");

    // Check complex generic types
    let map_array = symbols
        .iter()
        .find(|s| s.name == "mapArray")
        .expect("should find mapArray function");
    let sig = map_array.signature.as_ref().expect("should have signature");
    assert_eq!(sig.return_type.as_deref(), Some("U[]"), "array generic return type");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("T[]"),
        "array generic param type"
    );
    assert_eq!(
        sig.parameters[1].type_annotation.as_deref(),
        Some("(item: T) => U"),
        "function type param"
    );

    // Check Promise generic
    let fetch_data = symbols
        .iter()
        .find(|s| s.name == "fetchData")
        .expect("should find fetchData function");
    let sig = fetch_data.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("Promise<T>"),
        "Promise generic return type"
    );

    Ok(())
}

#[test]
fn extracts_python_generic_types() -> Result<(), TreeHuggerError> {
    // Regression test: Python typing generics should be captured
    let tree_file = TreeFile::new(fixture_path("generics.py"))?;
    let symbols = tree_file.symbols()?;

    // Check List generic
    let map_list = symbols
        .iter()
        .find(|s| s.name == "map_list")
        .expect("should find map_list function");
    let sig = map_list.signature.as_ref().expect("should have signature");
    assert_eq!(sig.return_type.as_deref(), Some("List[U]"), "List generic return");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("List[T]"),
        "List generic param"
    );

    // Check Optional generic
    let first_or_none = symbols
        .iter()
        .find(|s| s.name == "first_or_none")
        .expect("should find first_or_none function");
    let sig = first_or_none.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("Optional[T]"),
        "Optional generic return"
    );

    Ok(())
}

#[test]
fn extracts_go_generic_types() -> Result<(), TreeHuggerError> {
    // Regression test: Go 1.18+ generics should be captured
    let tree_file = TreeFile::new(fixture_path("generics.go"))?;
    let symbols = tree_file.symbols()?;

    // Check generic function
    let identity = symbols
        .iter()
        .find(|s| s.name == "Identity")
        .expect("should find Identity function");
    let sig = identity.signature.as_ref().expect("should have signature");
    assert_eq!(sig.return_type.as_deref(), Some("T"), "Go generic return type");

    // Check generic pointer return type
    let new_container = symbols
        .iter()
        .find(|s| s.name == "NewContainer")
        .expect("should find NewContainer function");
    let sig = new_container.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("*Container[T]"),
        "Go generic pointer return type"
    );

    // Check multi-parameter declaration (a, b T should capture both)
    let max = symbols
        .iter()
        .find(|s| s.name == "Max")
        .expect("should find Max function");
    let sig = max.signature.as_ref().expect("should have signature");
    assert_eq!(sig.parameters.len(), 2, "should have 2 parameters");
    assert_eq!(sig.parameters[0].name, "a");
    assert_eq!(sig.parameters[1].name, "b");
    assert_eq!(sig.parameters[0].type_annotation.as_deref(), Some("T"));
    assert_eq!(sig.parameters[1].type_annotation.as_deref(), Some("T"));

    Ok(())
}

// ============================================================================
// Regression tests for type metadata extraction
// These tests ensure struct fields, enum variants, and type parameters are captured.
// ============================================================================

#[test]
fn extracts_rust_struct_fields() -> Result<(), TreeHuggerError> {
    // Regression test: Struct fields should be captured with their types and doc comments
    let tree_file = TreeFile::new(fixture_path("sample.rs"))?;
    let symbols = tree_file.symbols()?;

    let greeter = symbols
        .iter()
        .find(|s| s.name == "Greeter" && s.kind == tree_hugger_lib::SymbolKind::Type)
        .expect("should find Greeter struct");

    let meta = greeter
        .type_metadata
        .as_ref()
        .expect("should have type metadata");

    // Check fields
    assert_eq!(meta.fields.len(), 1, "Greeter should have 1 field");
    assert_eq!(meta.fields[0].name, "prefix");
    assert_eq!(
        meta.fields[0].type_annotation.as_deref(),
        Some("String"),
        "field type should be captured"
    );
    assert!(
        meta.fields[0].doc_comment.is_some(),
        "field doc comment should be captured"
    );

    Ok(())
}

#[test]
fn extracts_rust_enum_variants() -> Result<(), TreeHuggerError> {
    // Regression test: Enum variants should be captured with tuple fields and struct fields
    let tree_file = TreeFile::new(fixture_path("types.rs"))?;
    let symbols = tree_file.symbols()?;

    // Check simple enum (now correctly identified as Enum, not Type)
    let message = symbols
        .iter()
        .find(|s| s.name == "Message" && s.kind == tree_hugger_lib::SymbolKind::Enum)
        .expect("should find Message enum");

    let meta = message
        .type_metadata
        .as_ref()
        .expect("should have type metadata");

    // Should have multiple variants
    assert!(meta.variants.len() >= 3, "Message should have at least 3 variants");

    // Find specific variants
    let quit = meta.variants.iter().find(|v| v.name == "Quit");
    assert!(quit.is_some(), "should find Quit variant");
    assert!(
        quit.unwrap().tuple_fields.is_empty() && quit.unwrap().struct_fields.is_empty(),
        "Quit should be a unit variant"
    );

    let write = meta.variants.iter().find(|v| v.name == "Write");
    assert!(write.is_some(), "should find Write variant");
    assert!(
        !write.unwrap().tuple_fields.is_empty(),
        "Write should be a tuple variant"
    );

    let move_variant = meta.variants.iter().find(|v| v.name == "Move");
    assert!(move_variant.is_some(), "should find Move variant");
    assert!(
        !move_variant.unwrap().struct_fields.is_empty(),
        "Move should be a struct variant"
    );

    Ok(())
}

#[test]
fn extracts_rust_generic_type_parameters() -> Result<(), TreeHuggerError> {
    // Regression test: Generic type parameters should be captured
    let tree_file = TreeFile::new(fixture_path("types.rs"))?;
    let symbols = tree_file.symbols()?;

    let container = symbols
        .iter()
        .find(|s| s.name == "Container" && s.kind == tree_hugger_lib::SymbolKind::Type)
        .expect("should find Container struct");

    let meta = container
        .type_metadata
        .as_ref()
        .expect("should have type metadata");

    assert!(
        !meta.type_parameters.is_empty(),
        "Container should have type parameters"
    );
    assert!(
        meta.type_parameters.contains(&"T".to_string()),
        "should capture T type parameter"
    );

    Ok(())
}

#[test]
fn extracts_rust_tuple_struct_fields() -> Result<(), TreeHuggerError> {
    // Regression test: Tuple struct fields should be captured with indexed names
    let tree_file = TreeFile::new(fixture_path("types.rs"))?;
    let symbols = tree_file.symbols()?;

    let point = symbols
        .iter()
        .find(|s| s.name == "Point" && s.kind == tree_hugger_lib::SymbolKind::Type)
        .expect("should find Point tuple struct");

    let meta = point
        .type_metadata
        .as_ref()
        .expect("should have type metadata");

    assert_eq!(meta.fields.len(), 2, "Point should have 2 tuple fields");
    assert_eq!(meta.fields[0].name, "0", "first field should be indexed as 0");
    assert_eq!(meta.fields[1].name, "1", "second field should be indexed as 1");
    assert!(
        meta.fields[0].type_annotation.is_some(),
        "tuple field types should be captured"
    );

    Ok(())
}

// ============================================================================
// Regression tests for struct vs enum type distinction
// Bug: Both structs and enums were reported as "type" without distinguishing them.
// ============================================================================

#[test]
fn distinguishes_rust_struct_from_enum() -> Result<(), TreeHuggerError> {
    // Regression test: Structs should be SymbolKind::Type, enums should be SymbolKind::Enum
    let tree_file = TreeFile::new(fixture_path("types.rs"))?;
    let symbols = tree_file.symbols()?;

    // Check that Point struct is captured as Type
    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("should find Point");
    assert_eq!(
        point.kind,
        tree_hugger_lib::SymbolKind::Type,
        "Point struct should be SymbolKind::Type"
    );

    // Check that Container struct is captured as Type
    let container = symbols
        .iter()
        .find(|s| s.name == "Container")
        .expect("should find Container");
    assert_eq!(
        container.kind,
        tree_hugger_lib::SymbolKind::Type,
        "Container struct should be SymbolKind::Type"
    );

    // Check that Message enum is captured as Enum, not Type
    let message = symbols
        .iter()
        .find(|s| s.name == "Message")
        .expect("should find Message");
    assert_eq!(
        message.kind,
        tree_hugger_lib::SymbolKind::Enum,
        "Message enum should be SymbolKind::Enum, not Type"
    );

    // Check that Result enum is captured as Enum
    let result = symbols
        .iter()
        .find(|s| s.name == "Result")
        .expect("should find Result");
    assert_eq!(
        result.kind,
        tree_hugger_lib::SymbolKind::Enum,
        "Result enum should be SymbolKind::Enum, not Type"
    );

    Ok(())
}

#[test]
fn rust_enum_is_included_in_types_filter() -> Result<(), TreeHuggerError> {
    // Regression test: Enums should still be included when filtering for "types"
    // via SymbolKind::is_type(), even though they have a distinct kind.
    let tree_file = TreeFile::new(fixture_path("types.rs"))?;
    let symbols = tree_file.symbols()?;

    // Filter for type-like symbols (what the `hug types` command does)
    let type_symbols: Vec<_> = symbols
        .into_iter()
        .filter(|s| s.kind.is_type())
        .collect();

    // Should find both structs and enums
    assert!(
        type_symbols.iter().any(|s| s.name == "Point"),
        "Point struct should be in types"
    );
    assert!(
        type_symbols.iter().any(|s| s.name == "Message"),
        "Message enum should be in types"
    );
    assert!(
        type_symbols.iter().any(|s| s.name == "Result"),
        "Result enum should be in types"
    );

    Ok(())
}

#[test]
fn distinguishes_typescript_interface_from_enum() -> Result<(), TreeHuggerError> {
    // Regression test: TypeScript enums should be SymbolKind::Enum, not Type
    let tree_file = TreeFile::new(fixture_path("sample.ts"))?;
    let symbols = tree_file.symbols()?;

    // Check that GreetingService interface is captured as Interface
    let greeting_service = symbols
        .iter()
        .find(|s| s.name == "GreetingService")
        .expect("should find GreetingService");
    assert_eq!(
        greeting_service.kind,
        tree_hugger_lib::SymbolKind::Interface,
        "GreetingService should be SymbolKind::Interface"
    );

    // Check that GreetFn type alias is captured as Type
    let greet_fn = symbols
        .iter()
        .find(|s| s.name == "GreetFn")
        .expect("should find GreetFn");
    assert_eq!(
        greet_fn.kind,
        tree_hugger_lib::SymbolKind::Type,
        "GreetFn type alias should be SymbolKind::Type"
    );

    // Check that Status enum is captured as Enum, not Type
    let status = symbols
        .iter()
        .find(|s| s.name == "Status")
        .expect("should find Status");
    assert_eq!(
        status.kind,
        tree_hugger_lib::SymbolKind::Enum,
        "Status enum should be SymbolKind::Enum, not Type"
    );

    Ok(())
}

// ============================================================================
// Comprehensive type distinction tests for all typed languages
// ============================================================================

#[test]
fn distinguishes_java_types() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.java"))?;
    let symbols = tree_file.symbols()?;

    // Class should be Type (Java classes are the primary type mechanism)
    let point = symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "should find Point class");
    assert_eq!(
        point.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "Java class should be SymbolKind::Type"
    );

    // Enum should be Enum
    let status = symbols.iter().find(|s| s.name == "Status");
    assert!(status.is_some(), "should find Status enum");
    assert_eq!(
        status.unwrap().kind,
        tree_hugger_lib::SymbolKind::Enum,
        "Java enum should be SymbolKind::Enum"
    );

    // Record should be Type (Java records are a type mechanism)
    let person = symbols.iter().find(|s| s.name == "Person");
    assert!(person.is_some(), "should find Person record");
    assert_eq!(
        person.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "Java record should be SymbolKind::Type"
    );

    Ok(())
}

#[test]
fn distinguishes_c_types() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.c"))?;
    let symbols = tree_file.symbols()?;

    // Struct should be Type
    let point = symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "should find Point struct");
    assert_eq!(
        point.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "C struct should be SymbolKind::Type"
    );

    // Enum should be Enum
    let status = symbols.iter().find(|s| s.name == "Status");
    assert!(status.is_some(), "should find Status enum");
    assert_eq!(
        status.unwrap().kind,
        tree_hugger_lib::SymbolKind::Enum,
        "C enum should be SymbolKind::Enum"
    );

    // Typedef should be Type
    let alias = symbols.iter().find(|s| s.name == "PointAlias");
    assert!(alias.is_some(), "should find PointAlias typedef");
    assert_eq!(
        alias.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "C typedef should be SymbolKind::Type"
    );

    Ok(())
}

#[test]
fn distinguishes_cpp_types() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.cpp"))?;
    let symbols = tree_file.symbols()?;

    // Struct should be Type
    let point = symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "should find Point struct");
    assert_eq!(
        point.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "C++ struct should be SymbolKind::Type"
    );

    // Class should be Type
    let greeter = symbols.iter().find(|s| s.name == "Greeter");
    assert!(greeter.is_some(), "should find Greeter class");
    assert_eq!(
        greeter.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "C++ class should be SymbolKind::Type"
    );

    // Enum should be Enum
    let status = symbols.iter().find(|s| s.name == "Status");
    assert!(status.is_some(), "should find Status enum");
    assert_eq!(
        status.unwrap().kind,
        tree_hugger_lib::SymbolKind::Enum,
        "C++ enum should be SymbolKind::Enum"
    );

    // Scoped enum (enum class) should also be Enum
    let color = symbols.iter().find(|s| s.name == "Color");
    assert!(color.is_some(), "should find Color enum class");
    assert_eq!(
        color.unwrap().kind,
        tree_hugger_lib::SymbolKind::Enum,
        "C++ enum class should be SymbolKind::Enum"
    );

    Ok(())
}

#[test]
fn distinguishes_csharp_types() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.cs"))?;
    let symbols = tree_file.symbols()?;

    // Struct should be Type
    let point = symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "should find Point struct");
    assert_eq!(
        point.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "C# struct should be SymbolKind::Type"
    );

    // Class should be Class
    let greeter = symbols.iter().find(|s| s.name == "Greeter");
    assert!(greeter.is_some(), "should find Greeter class");
    assert_eq!(
        greeter.unwrap().kind,
        tree_hugger_lib::SymbolKind::Class,
        "C# class should be SymbolKind::Class"
    );

    // Interface should be Interface
    let service = symbols.iter().find(|s| s.name == "IGreetingService");
    assert!(service.is_some(), "should find IGreetingService interface");
    assert_eq!(
        service.unwrap().kind,
        tree_hugger_lib::SymbolKind::Interface,
        "C# interface should be SymbolKind::Interface"
    );

    // Enum should be Enum
    let status = symbols.iter().find(|s| s.name == "Status");
    assert!(status.is_some(), "should find Status enum");
    assert_eq!(
        status.unwrap().kind,
        tree_hugger_lib::SymbolKind::Enum,
        "C# enum should be SymbolKind::Enum"
    );

    // Record should be Type
    let person = symbols.iter().find(|s| s.name == "Person");
    assert!(person.is_some(), "should find Person record");
    assert_eq!(
        person.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "C# record should be SymbolKind::Type"
    );

    Ok(())
}

#[test]
fn distinguishes_swift_types() -> Result<(), TreeHuggerError> {
    // Note: Swift's tree-sitter grammar uses a single `class_declaration` node for
    // struct, class, and enum, with a `declaration_kind` field to differentiate.
    // Fine-grained distinction would require Rust-level handling of this field.
    // For now, all are captured as Type, with protocols captured as Interface.
    let tree_file = TreeFile::new(fixture_path("types.swift"))?;
    let symbols = tree_file.symbols()?;

    // Struct should be Type (Swift grammar limitation - all class_declaration are Type)
    let point = symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "should find Point struct");
    assert_eq!(
        point.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "Swift struct should be SymbolKind::Type"
    );

    // Class should be Type (Swift grammar limitation)
    let greeter = symbols.iter().find(|s| s.name == "Greeter");
    assert!(greeter.is_some(), "should find Greeter class");
    assert_eq!(
        greeter.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "Swift class should be SymbolKind::Type (grammar limitation)"
    );

    // Enum should be Type (Swift grammar limitation)
    let status = symbols.iter().find(|s| s.name == "Status");
    assert!(status.is_some(), "should find Status enum");
    assert_eq!(
        status.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "Swift enum should be SymbolKind::Type (grammar limitation)"
    );

    // Protocol should be Interface (this works correctly)
    let service = symbols.iter().find(|s| s.name == "GreetingService");
    assert!(service.is_some(), "should find GreetingService protocol");
    assert_eq!(
        service.unwrap().kind,
        tree_hugger_lib::SymbolKind::Interface,
        "Swift protocol should be SymbolKind::Interface"
    );

    Ok(())
}

#[test]
fn distinguishes_scala_types() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.scala"))?;
    let symbols = tree_file.symbols()?;

    // Class should be Class
    let point = symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "should find Point class");
    assert_eq!(
        point.unwrap().kind,
        tree_hugger_lib::SymbolKind::Class,
        "Scala class should be SymbolKind::Class"
    );

    // Trait should be Trait
    let greeter = symbols.iter().find(|s| s.name == "Greeter");
    assert!(greeter.is_some(), "should find Greeter trait");
    assert_eq!(
        greeter.unwrap().kind,
        tree_hugger_lib::SymbolKind::Trait,
        "Scala trait should be SymbolKind::Trait"
    );

    // Object should be Module
    let default_greeter = symbols.iter().find(|s| s.name == "DefaultGreeter");
    assert!(default_greeter.is_some(), "should find DefaultGreeter object");
    assert_eq!(
        default_greeter.unwrap().kind,
        tree_hugger_lib::SymbolKind::Module,
        "Scala object should be SymbolKind::Module"
    );

    // Enum should be Enum (Scala 3)
    let status = symbols.iter().find(|s| s.name == "Status");
    assert!(status.is_some(), "should find Status enum");
    assert_eq!(
        status.unwrap().kind,
        tree_hugger_lib::SymbolKind::Enum,
        "Scala 3 enum should be SymbolKind::Enum"
    );

    Ok(())
}

#[test]
fn distinguishes_php_types() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.php"))?;
    let symbols = tree_file.symbols()?;

    // Class should be Class
    let greeter = symbols.iter().find(|s| s.name == "Greeter");
    assert!(greeter.is_some(), "should find Greeter class");
    assert_eq!(
        greeter.unwrap().kind,
        tree_hugger_lib::SymbolKind::Class,
        "PHP class should be SymbolKind::Class"
    );

    // Interface should be Interface
    let service = symbols.iter().find(|s| s.name == "GreetingService");
    assert!(service.is_some(), "should find GreetingService interface");
    assert_eq!(
        service.unwrap().kind,
        tree_hugger_lib::SymbolKind::Interface,
        "PHP interface should be SymbolKind::Interface"
    );

    // Trait should be Trait
    let greeting_trait = symbols.iter().find(|s| s.name == "GreetingTrait");
    assert!(greeting_trait.is_some(), "should find GreetingTrait trait");
    assert_eq!(
        greeting_trait.unwrap().kind,
        tree_hugger_lib::SymbolKind::Trait,
        "PHP trait should be SymbolKind::Trait"
    );

    // Enum should be Enum (PHP 8.1+)
    let status = symbols.iter().find(|s| s.name == "Status");
    assert!(status.is_some(), "should find Status enum");
    assert_eq!(
        status.unwrap().kind,
        tree_hugger_lib::SymbolKind::Enum,
        "PHP enum should be SymbolKind::Enum"
    );

    Ok(())
}

#[test]
fn distinguishes_go_types() -> Result<(), TreeHuggerError> {
    let tree_file = TreeFile::new(fixture_path("types.go"))?;
    let symbols = tree_file.symbols()?;

    // Struct should be Type (Go uses type for all type definitions)
    let point = symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "should find Point struct");
    assert_eq!(
        point.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "Go struct should be SymbolKind::Type"
    );

    // Interface should also be Type (Go doesn't distinguish at AST level)
    // This is expected behavior for Go - the type keyword is used for both
    let service = symbols.iter().find(|s| s.name == "GreetingService");
    assert!(service.is_some(), "should find GreetingService interface");
    assert_eq!(
        service.unwrap().kind,
        tree_hugger_lib::SymbolKind::Type,
        "Go interface should be SymbolKind::Type (Go uses 'type' for all)"
    );

    Ok(())
}

// ============================================================================
// Type metadata extraction regression tests
// Bug: Types were showing without field/variant information (e.g., "type Point"
// instead of "type Point { x: int, y: int }").
// ============================================================================

#[test]
fn extracts_java_type_metadata() -> Result<(), TreeHuggerError> {
    // Regression test: Java classes should have field metadata
    let tree_file = TreeFile::new(fixture_path("types.java"))?;
    let symbols = tree_file.symbols()?;

    // Class should have fields
    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("should find Point class");
    let meta = point
        .type_metadata
        .as_ref()
        .expect("Point should have type_metadata");
    assert!(meta.fields.len() >= 2, "Point should have at least 2 fields");
    assert!(
        meta.fields.iter().any(|f| f.name == "x"),
        "Point should have field x"
    );
    assert!(
        meta.fields.iter().any(|f| f.name == "y"),
        "Point should have field y"
    );

    // Enum should have variants
    let status = symbols
        .iter()
        .find(|s| s.name == "Status")
        .expect("should find Status enum");
    let meta = status
        .type_metadata
        .as_ref()
        .expect("Status should have type_metadata");
    assert!(
        meta.variants.len() >= 3,
        "Status should have at least 3 variants"
    );
    assert!(
        meta.variants.iter().any(|v| v.name == "SUCCESS"),
        "Status should have SUCCESS variant"
    );

    // Record should have fields (components)
    let person = symbols
        .iter()
        .find(|s| s.name == "Person")
        .expect("should find Person record");
    let meta = person
        .type_metadata
        .as_ref()
        .expect("Person should have type_metadata");
    assert!(meta.fields.len() >= 2, "Person should have at least 2 fields");
    assert!(
        meta.fields.iter().any(|f| f.name == "name"),
        "Person should have field name"
    );
    assert!(
        meta.fields.iter().any(|f| f.name == "age"),
        "Person should have field age"
    );

    Ok(())
}

#[test]
fn extracts_c_type_metadata() -> Result<(), TreeHuggerError> {
    // Regression test: C structs should have field metadata
    let tree_file = TreeFile::new(fixture_path("types.c"))?;
    let symbols = tree_file.symbols()?;

    // Struct should have fields
    let point = symbols
        .iter()
        .find(|s| s.name == "Point" && s.kind == tree_hugger_lib::SymbolKind::Type)
        .expect("should find Point struct");
    let meta = point
        .type_metadata
        .as_ref()
        .expect("Point should have type_metadata");
    assert!(meta.fields.len() >= 2, "Point should have at least 2 fields");
    assert!(
        meta.fields.iter().any(|f| f.name == "x"),
        "Point should have field x"
    );

    // Enum should have variants
    let status = symbols
        .iter()
        .find(|s| s.name == "Status")
        .expect("should find Status enum");
    let meta = status
        .type_metadata
        .as_ref()
        .expect("Status should have type_metadata");
    assert!(
        meta.variants.len() >= 3,
        "Status should have at least 3 variants"
    );
    assert!(
        meta.variants.iter().any(|v| v.name == "SUCCESS"),
        "Status should have SUCCESS variant"
    );

    Ok(())
}

#[test]
fn extracts_cpp_type_metadata() -> Result<(), TreeHuggerError> {
    // Regression test: C++ classes/structs should have field metadata
    let tree_file = TreeFile::new(fixture_path("types.cpp"))?;
    let symbols = tree_file.symbols()?;

    // Struct should have fields
    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("should find Point struct");
    let meta = point
        .type_metadata
        .as_ref()
        .expect("Point should have type_metadata");
    assert!(meta.fields.len() >= 2, "Point should have at least 2 fields");
    assert!(
        meta.fields.iter().any(|f| f.name == "x"),
        "Point should have field x"
    );

    // Class should have fields
    let greeter = symbols
        .iter()
        .find(|s| s.name == "Greeter")
        .expect("should find Greeter class");
    let meta = greeter
        .type_metadata
        .as_ref()
        .expect("Greeter should have type_metadata");
    assert!(
        meta.fields.iter().any(|f| f.name == "prefix"),
        "Greeter should have field prefix"
    );

    // Enum should have variants
    let status = symbols
        .iter()
        .find(|s| s.name == "Status")
        .expect("should find Status enum");
    let meta = status
        .type_metadata
        .as_ref()
        .expect("Status should have type_metadata");
    assert!(
        meta.variants.len() >= 3,
        "Status should have at least 3 variants"
    );

    Ok(())
}

#[test]
fn extracts_csharp_type_metadata() -> Result<(), TreeHuggerError> {
    // Regression test: C# types should have field/variant metadata
    let tree_file = TreeFile::new(fixture_path("types.cs"))?;
    let symbols = tree_file.symbols()?;

    // Struct should have fields
    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("should find Point struct");
    let meta = point
        .type_metadata
        .as_ref()
        .expect("Point should have type_metadata");
    assert!(meta.fields.len() >= 2, "Point should have at least 2 fields");
    assert!(
        meta.fields.iter().any(|f| f.name == "X"),
        "Point should have field X"
    );

    // Enum should have variants
    let status = symbols
        .iter()
        .find(|s| s.name == "Status")
        .expect("should find Status enum");
    let meta = status
        .type_metadata
        .as_ref()
        .expect("Status should have type_metadata");
    assert!(
        meta.variants.len() >= 3,
        "Status should have at least 3 variants"
    );
    assert!(
        meta.variants.iter().any(|v| v.name == "Success"),
        "Status should have Success variant"
    );

    // Interface should have method signatures
    let service = symbols
        .iter()
        .find(|s| s.name == "IGreetingService")
        .expect("should find IGreetingService interface");
    let meta = service
        .type_metadata
        .as_ref()
        .expect("IGreetingService should have type_metadata");
    assert!(
        meta.fields.iter().any(|f| f.name == "Greet"),
        "IGreetingService should have Greet method"
    );

    // Record should have fields
    let person = symbols
        .iter()
        .find(|s| s.name == "Person")
        .expect("should find Person record");
    let meta = person
        .type_metadata
        .as_ref()
        .expect("Person should have type_metadata");
    assert!(meta.fields.len() >= 2, "Person should have at least 2 fields");
    assert!(
        meta.fields.iter().any(|f| f.name == "Name"),
        "Person should have field Name"
    );

    Ok(())
}

#[test]
fn extracts_swift_type_metadata() -> Result<(), TreeHuggerError> {
    // Regression test: Swift types should have field metadata
    let tree_file = TreeFile::new(fixture_path("types.swift"))?;
    let symbols = tree_file.symbols()?;

    // Struct should have fields
    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("should find Point struct");
    let meta = point
        .type_metadata
        .as_ref()
        .expect("Point should have type_metadata");
    assert!(meta.fields.len() >= 2, "Point should have at least 2 fields");
    assert!(
        meta.fields.iter().any(|f| f.name == "x"),
        "Point should have field x"
    );

    // Class should have fields
    let greeter = symbols
        .iter()
        .find(|s| s.name == "Greeter")
        .expect("should find Greeter class");
    let meta = greeter
        .type_metadata
        .as_ref()
        .expect("Greeter should have type_metadata");
    assert!(
        meta.fields.iter().any(|f| f.name == "prefix"),
        "Greeter should have field prefix"
    );

    // Protocol should have method signatures
    let service = symbols
        .iter()
        .find(|s| s.name == "GreetingService")
        .expect("should find GreetingService protocol");
    let meta = service
        .type_metadata
        .as_ref()
        .expect("GreetingService should have type_metadata");
    assert!(
        meta.fields.iter().any(|f| f.name == "greet"),
        "GreetingService should have greet method"
    );

    Ok(())
}

#[test]
fn extracts_scala_type_metadata() -> Result<(), TreeHuggerError> {
    // Regression test: Scala types should have field/method metadata
    let tree_file = TreeFile::new(fixture_path("types.scala"))?;
    let symbols = tree_file.symbols()?;

    // Class should have parameters as fields
    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("should find Point class");
    let meta = point
        .type_metadata
        .as_ref()
        .expect("Point should have type_metadata");
    assert!(
        meta.fields.len() >= 2,
        "Point should have at least 2 class parameters"
    );
    assert!(
        meta.fields.iter().any(|f| f.name == "x"),
        "Point should have parameter x"
    );

    // Trait should have method signatures
    let greeter = symbols
        .iter()
        .find(|s| s.name == "Greeter")
        .expect("should find Greeter trait");
    let meta = greeter
        .type_metadata
        .as_ref()
        .expect("Greeter should have type_metadata");
    assert!(
        meta.fields.iter().any(|f| f.name == "greet"),
        "Greeter should have greet method"
    );

    Ok(())
}

#[test]
fn extracts_php_type_metadata() -> Result<(), TreeHuggerError> {
    // Regression test: PHP types should have field/variant metadata
    let tree_file = TreeFile::new(fixture_path("types.php"))?;
    let symbols = tree_file.symbols()?;

    // Class should have fields
    let greeter = symbols
        .iter()
        .find(|s| s.name == "Greeter")
        .expect("should find Greeter class");
    let meta = greeter
        .type_metadata
        .as_ref()
        .expect("Greeter should have type_metadata");
    assert!(
        meta.fields.iter().any(|f| f.name == "prefix"),
        "Greeter should have field prefix"
    );

    // Interface should have method signatures
    let service = symbols
        .iter()
        .find(|s| s.name == "GreetingService")
        .expect("should find GreetingService interface");
    let meta = service
        .type_metadata
        .as_ref()
        .expect("GreetingService should have type_metadata");
    assert!(
        meta.fields.iter().any(|f| f.name == "greet"),
        "GreetingService should have greet method"
    );

    // Enum should have variants
    let status = symbols
        .iter()
        .find(|s| s.name == "Status")
        .expect("should find Status enum");
    let meta = status
        .type_metadata
        .as_ref()
        .expect("Status should have type_metadata");
    assert!(
        meta.variants.len() >= 3,
        "Status should have at least 3 variants"
    );
    assert!(
        meta.variants.iter().any(|v| v.name == "Success"),
        "Status should have Success variant"
    );

    Ok(())
}

#[test]
fn c_enum_not_duplicated() -> Result<(), TreeHuggerError> {
    // Regression test: C enums should only appear once, not duplicated per enumerator
    let tree_file = TreeFile::new(fixture_path("types.c"))?;
    let symbols = tree_file.symbols()?;

    let status_count = symbols.iter().filter(|s| s.name == "Status").count();
    assert_eq!(
        status_count, 1,
        "C enum should only appear once, not duplicated (found {status_count})"
    );

    Ok(())
}

#[test]
fn cpp_enum_not_duplicated() -> Result<(), TreeHuggerError> {
    // Regression test: C++ enums should only appear once, not duplicated per enumerator
    let tree_file = TreeFile::new(fixture_path("types.cpp"))?;
    let symbols = tree_file.symbols()?;

    let status_count = symbols.iter().filter(|s| s.name == "Status").count();
    assert_eq!(
        status_count, 1,
        "C++ enum should only appear once, not duplicated (found {status_count})"
    );

    let color_count = symbols.iter().filter(|s| s.name == "Color").count();
    assert_eq!(
        color_count, 1,
        "C++ enum class should only appear once, not duplicated (found {color_count})"
    );

    Ok(())
}

// ============================================================================
// Regression tests for consistent function signature extraction across languages
// Bug: Function signatures were only extracted for Rust, TypeScript, Go, Python.
// Other languages (PHP, Java, C, C++, C#, Swift, Scala) showed only names.
// ============================================================================

#[test]
fn extracts_php_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: PHP functions should have parameters and return types extracted
    let tree_file = TreeFile::new(fixture_path("signatures.php"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("string"),
        "PHP function return type should be extracted"
    );

    // Check parameters
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("string"),
        "PHP parameter type should be extracted"
    );

    // Check default value
    assert!(sig.parameters.len() >= 2, "should have multiple parameters");
    assert_eq!(
        sig.parameters[1].default_value.as_deref(),
        Some("25"),
        "PHP default value should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_java_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: Java methods should have parameters and return types extracted
    let tree_file = TreeFile::new(fixture_path("signatures.java"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Method)
        .expect("should find greet method");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("String"),
        "Java method return type should be extracted"
    );

    // Check parameters
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("String"),
        "Java parameter type should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_c_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: C functions should have parameters and return types extracted
    let tree_file = TreeFile::new(fixture_path("signatures.c"))?;
    let symbols = tree_file.symbols()?;

    let add = symbols
        .iter()
        .find(|s| s.name == "add" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find add function");

    let sig = add.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("int"),
        "C function return type should be extracted"
    );

    // Check parameters
    assert_eq!(sig.parameters.len(), 2, "should have 2 parameters");
    assert_eq!(sig.parameters[0].name, "a");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("int"),
        "C parameter type should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_cpp_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: C++ functions should have parameters and return types extracted
    let tree_file = TreeFile::new(fixture_path("signatures.cpp"))?;
    let symbols = tree_file.symbols()?;

    let add = symbols
        .iter()
        .find(|s| s.name == "add" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find add function");

    let sig = add.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("int"),
        "C++ function return type should be extracted"
    );

    // Check parameters
    assert_eq!(sig.parameters.len(), 2, "should have 2 parameters");
    assert_eq!(sig.parameters[0].name, "a");
    assert_eq!(sig.parameters[1].name, "b");

    Ok(())
}

#[test]
fn extracts_csharp_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: C# methods should have parameters and return types extracted
    let tree_file = TreeFile::new(fixture_path("signatures.cs"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Method)
        .expect("should find Greet method");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("string"),
        "C# method return type should be extracted"
    );

    // Check parameters
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("string"),
        "C# parameter type should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_swift_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: Swift functions should have parameters and return types extracted
    let tree_file = TreeFile::new(fixture_path("signatures.swift"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("String"),
        "Swift function return type should be extracted"
    );

    // Check parameters
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("String"),
        "Swift parameter type should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_scala_function_signature() -> Result<(), TreeHuggerError> {
    // Regression test: Scala functions should have parameters and return types extracted
    let tree_file = TreeFile::new(fixture_path("signatures.scala"))?;
    let symbols = tree_file.symbols()?;

    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find greet function");

    let sig = greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("String"),
        "Scala function return type should be extracted"
    );

    // Check parameters
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(
        sig.parameters[0].type_annotation.as_deref(),
        Some("String"),
        "Scala parameter type should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_scala_default_parameters() -> Result<(), TreeHuggerError> {
    // Regression test: Scala default parameter values should be extracted
    let tree_file = TreeFile::new(fixture_path("signatures.scala"))?;
    let symbols = tree_file.symbols()?;

    let greet_with_prefix = symbols
        .iter()
        .find(|s| s.name == "greetWithPrefix")
        .expect("should find greetWithPrefix function");

    let sig = greet_with_prefix
        .signature
        .as_ref()
        .expect("should have signature");

    assert!(sig.parameters.len() >= 2, "should have 2 parameters");
    assert_eq!(
        sig.parameters[1].default_value.as_deref(),
        Some("\"Hello\""),
        "Scala default value should be extracted"
    );

    Ok(())
}

#[test]
fn extracts_swift_multiple_parameters() -> Result<(), TreeHuggerError> {
    // Regression test: Swift functions with multiple parameters should extract all
    let tree_file = TreeFile::new(fixture_path("signatures.swift"))?;
    let symbols = tree_file.symbols()?;

    let greet_with_age = symbols
        .iter()
        .find(|s| s.name == "greetWithAge")
        .expect("should find greetWithAge function");

    let sig = greet_with_age
        .signature
        .as_ref()
        .expect("should have signature");

    assert_eq!(sig.parameters.len(), 2, "should have 2 parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(sig.parameters[0].type_annotation.as_deref(), Some("String"));
    assert_eq!(sig.parameters[1].name, "age");
    assert_eq!(sig.parameters[1].type_annotation.as_deref(), Some("Int"));

    Ok(())
}

// ============================================================================
// Visibility modifier extraction tests
// ============================================================================

#[test]
fn extracts_typescript_visibility_modifiers() -> Result<(), TreeHuggerError> {
    // Regression test: TypeScript methods should have visibility modifiers extracted
    let tree_file = TreeFile::new(fixture_path("arrows.ts"))?;
    let symbols = tree_file.symbols()?;

    // Find the public greet method
    let public_greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function && s.range.start_line == 32)
        .expect("should find public greet method");

    let sig = public_greet.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.visibility,
        Some(tree_hugger_lib::Visibility::Public),
        "TypeScript public method should have Public visibility"
    );

    // Find the protected formatName method
    let protected_method = symbols
        .iter()
        .find(|s| s.name == "formatName")
        .expect("should find protected formatName method");

    let sig = protected_method.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.visibility,
        Some(tree_hugger_lib::Visibility::Protected),
        "TypeScript protected method should have Protected visibility"
    );

    // Find the private log method
    let private_method = symbols
        .iter()
        .find(|s| s.name == "log")
        .expect("should find private log method");

    let sig = private_method.signature.as_ref().expect("should have signature");
    assert_eq!(
        sig.visibility,
        Some(tree_hugger_lib::Visibility::Private),
        "TypeScript private method should have Private visibility"
    );

    Ok(())
}

#[test]
fn infers_csharp_interface_method_visibility() -> Result<(), TreeHuggerError> {
    // C# interface members are implicitly public
    let tree_file = TreeFile::new(fixture_path("types.cs"))?;
    let symbols = tree_file.symbols()?;

    // Find the interface method (line 19)
    let interface_method = symbols
        .iter()
        .find(|s| s.name == "Greet" && s.kind == tree_hugger_lib::SymbolKind::Method && s.range.start_line == 19)
        .expect("should find Greet method in interface");

    let sig = interface_method
        .signature
        .as_ref()
        .expect("interface method should have signature");
    assert_eq!(
        sig.visibility,
        Some(tree_hugger_lib::Visibility::Public),
        "C# interface method should have inferred Public visibility"
    );

    Ok(())
}

#[test]
fn infers_java_interface_method_visibility() -> Result<(), TreeHuggerError> {
    // Java interface members are implicitly public
    let tree_file = TreeFile::new(fixture_path("types.java"))?;
    let symbols = tree_file.symbols()?;

    // Find the interface method (line 18)
    let interface_method = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Method && s.range.start_line == 18)
        .expect("should find greet method in interface");

    let sig = interface_method
        .signature
        .as_ref()
        .expect("interface method should have signature");
    assert_eq!(
        sig.visibility,
        Some(tree_hugger_lib::Visibility::Public),
        "Java interface method should have inferred Public visibility"
    );

    Ok(())
}

// ============================================================================
// Arrow function extraction tests
// ============================================================================

#[test]
fn extracts_typescript_arrow_function_signatures() -> Result<(), TreeHuggerError> {
    // Regression test: TypeScript arrow functions should have signatures extracted
    let tree_file = TreeFile::new(fixture_path("arrows.ts"))?;
    let symbols = tree_file.symbols()?;

    // Find the greet arrow function (line 2)
    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function && s.range.start_line == 2)
        .expect("should find greet arrow function");

    let sig = greet.signature.as_ref().expect("arrow function should have signature");
    assert_eq!(
        sig.return_type.as_deref(),
        Some("string"),
        "Arrow function return type should be extracted"
    );
    assert!(!sig.parameters.is_empty(), "Arrow function should have parameters");
    assert_eq!(sig.parameters[0].name, "name");
    assert_eq!(sig.parameters[0].type_annotation.as_deref(), Some("string"));

    Ok(())
}

#[test]
fn extracts_typescript_arrow_function_with_multiple_params() -> Result<(), TreeHuggerError> {
    // Regression test: Arrow functions with multiple parameters
    let tree_file = TreeFile::new(fixture_path("arrows.ts"))?;
    let symbols = tree_file.symbols()?;

    let add = symbols
        .iter()
        .find(|s| s.name == "add" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find add arrow function");

    let sig = add.signature.as_ref().expect("should have signature");
    assert_eq!(sig.parameters.len(), 2, "should have 2 parameters");
    assert_eq!(sig.parameters[0].name, "a");
    assert_eq!(sig.parameters[0].type_annotation.as_deref(), Some("number"));
    assert_eq!(sig.parameters[1].name, "b");
    assert_eq!(sig.parameters[1].type_annotation.as_deref(), Some("number"));

    Ok(())
}

#[test]
fn extracts_typescript_arrow_function_with_rest_params() -> Result<(), TreeHuggerError> {
    // Regression test: Arrow functions with rest parameters
    let tree_file = TreeFile::new(fixture_path("arrows.ts"))?;
    let symbols = tree_file.symbols()?;

    let sum = symbols
        .iter()
        .find(|s| s.name == "sum" && s.kind == tree_hugger_lib::SymbolKind::Function)
        .expect("should find sum arrow function");

    let sig = sum.signature.as_ref().expect("should have signature");
    assert!(!sig.parameters.is_empty(), "should have parameters");
    assert_eq!(sig.parameters[0].name, "numbers");
    assert!(
        sig.parameters[0].is_variadic,
        "rest parameter should be marked as variadic"
    );

    Ok(())
}

#[test]
fn extracts_javascript_arrow_function_signatures() -> Result<(), TreeHuggerError> {
    // Regression test: JavaScript arrow functions should have signatures extracted
    let tree_file = TreeFile::new(fixture_path("arrows.js"))?;
    let symbols = tree_file.symbols()?;

    // Find the greet arrow function
    let greet = symbols
        .iter()
        .find(|s| s.name == "greet" && s.kind == tree_hugger_lib::SymbolKind::Function && s.range.start_line == 2)
        .expect("should find greet arrow function");

    let sig = greet.signature.as_ref().expect("arrow function should have signature");
    assert!(!sig.parameters.is_empty(), "Arrow function should have parameters");
    assert_eq!(sig.parameters[0].name, "name");

    Ok(())
}
