//! The declaring keyword (`identity.keyword`) and the display header
//! (`source.signature_text`) of fixture declarations, in every supported
//! language, read through `TreeFile::symbol_index_v2()`.
//!
//! The signature matrix accounts for every declaration shape in every
//! language: a fixture record with its expected header, or an explicit
//! reason the shape does not apply (`NotApplicable`) or the grammar cannot
//! parse it (`Unsupported`).

use tree_hugger::{ProgrammingLanguage, SymbolRecord};

use crate::symbols_fixtures::{Extracted, extract_inline, extract_path, extract_primary};

type L = ProgrammingLanguage;

/// `(language, name, container, keyword)`: the record `name` in `container`
/// reports `keyword`; `None` where the construct has no declaring keyword.
pub type KeywordRow = (L, &'static str, Option<&'static str>, Option<&'static str>);

pub const KEYWORD_MATRIX: &[KeywordRow] = &[
    (L::Rust, "public_function", None, Some("fn")),
    (L::Rust, "required_trait_method", Some("PublicTrait"), Some("fn")),
    (L::Rust, "PUBLIC_CONSTANT", None, Some("const")),
    (L::Rust, "PUBLIC_STATIC", None, Some("static")),
    (L::Rust, "PublicAlias", None, Some("type")),
    (L::Rust, "TraitAssociatedType", Some("PublicTrait"), Some("type")),
    (L::Rust, "outer_module", None, Some("mod")),
    (L::Rust, "PublicStruct", None, Some("struct")),
    (L::Rust, "PublicUnion", None, Some("union")),
    (L::Rust, "PublicEnum", None, Some("enum")),
    (L::Rust, "PublicTrait", None, Some("trait")),
    (L::Rust, "local_value", None, Some("let")),
    (L::Rust, "public_field", Some("PublicStruct"), None),
    (L::Rust, "UnitVariant", Some("PublicEnum"), None),
    // preceded by `// … fn struct enum trait const static`
    (L::Rust, "BOUNDARY_TABLE", None, Some("const")),
    (L::Go, "ExportedFunction", None, Some("func")),
    (L::Go, "ExportedMethod", Some("ExportedStruct"), Some("func")),
    (L::Go, "ExportedStruct", None, Some("type")),
    (L::Go, "ExportedConstant", None, Some("const")),
    (L::Go, "ExportedVariable", None, Some("var")),
    (L::Go, "symbols", None, Some("package")),
    (L::Go, "localValue", None, None),
    (L::Go, "parameterValue", None, None),
    // preceded by `// … func type var const struct`
    (L::Go, "boundaryTable", None, Some("var")),
    (L::Swift, "OpenWidget", None, Some("class")),
    (L::Swift, "PublicApi", None, Some("struct")),
    (L::Swift, "PublicDirection", None, Some("enum")),
    (L::Swift, "PublicDrawable", None, Some("protocol")),
    (L::Swift, "openMethod", Some("OpenWidget"), Some("func")),
    (L::Swift, "classMethod", Some("OpenWidget"), Some("func")),
    (L::Swift, "publicProperty", Some("OpenWidget"), Some("var")),
    (L::Swift, "drawableName", Some("PublicDrawable"), Some("var")),
    (L::Swift, "localValue", None, Some("let")),
    (L::Swift, "north", Some("PublicDirection"), Some("case")),
    (L::Swift, "parameterValue", None, None),
    (L::Scala, "topLevelFunction", None, Some("def")),
    (L::Scala, "abstractTraitMethod", Some("PlainTrait"), Some("def")),
    (L::Scala, "publicField", Some("PlainWidget"), Some("val")),
    (L::Scala, "PlainTrait", None, Some("trait")),
    (L::Scala, "StandaloneObject", None, Some("object")),
    (L::Scala, "PlainColor", None, Some("enum")),
    // `case class`: the token that names the construct
    (L::Scala, "CasePoint", None, Some("class")),
    (L::Scala, "constructorField", Some("PlainWidget"), Some("val")),
    (L::Scala, "xCoordinate", Some("CasePoint"), None),
    (L::TypeScript, "exportedFunction", None, Some("function")),
    (L::TypeScript, "EXPORTED_CONSTANT", None, Some("const")),
    (L::TypeScript, "localValue", None, Some("const")),
    (L::TypeScript, "ExportedAlias", None, Some("type")),
    (L::TypeScript, "ExportedInterface", None, Some("interface")),
    (L::TypeScript, "ExportedEnum", None, Some("enum")),
    (L::TypeScript, "ExportedAbstractBase", None, Some("class")),
    (L::TypeScript, "publicMethod", Some("ExportedWidget"), None),
    (L::TypeScript, "publicProperty", Some("ExportedWidget"), None),
    (L::TypeScript, "boundaryArrow", None, None),
    (L::JavaScript, "exportedFunction", None, Some("function")),
    (L::JavaScript, "EXPORTED_CONSTANT", None, Some("const")),
    (L::JavaScript, "ExportedWidget", None, Some("class")),
    (L::JavaScript, "unmarkedMethod", Some("ExportedWidget"), None),
    (L::JavaScript, "staticProperty", Some("ExportedWidget"), None),
    (L::Java, "PublicWidget", None, Some("class")),
    (L::Java, "PublicShape", None, Some("interface")),
    (L::Java, "PublicColor", None, Some("enum")),
    (L::Java, "PublicPoint", None, Some("record")),
    (L::Java, "com.example.symbols", None, Some("package")),
    (L::Java, "publicMethod", Some("PublicWidget"), None),
    (L::Java, "publicField", Some("PublicWidget"), None),
    // preceded by `// … class interface enum record`
    (L::Java, "BOUNDARY_TABLE", Some("DeclarationBoundaries"), None),
    (L::CSharp, "PublicWidget", Some("Example.Symbols"), Some("class")),
    (L::CSharp, "PublicStruct", Some("Example.Symbols"), Some("struct")),
    (L::CSharp, "IShape", Some("Example.Symbols"), Some("interface")),
    (L::CSharp, "PublicColor", Some("Example.Symbols"), Some("enum")),
    (L::CSharp, "RecordFieldForms", Some("Example.Symbols"), Some("record")),
    (L::CSharp, "Example.Symbols", None, Some("namespace")),
    (L::CSharp, "PublicMethod", Some("PublicWidget"), None),
    (L::CSharp, "Count", Some("PublicWidget"), None),
    (L::CSharp, "ClassConstant", Some("ConstantForms"), None),
    (L::C, "NamedTag", None, Some("struct")),
    (L::C, "NamedUnion", None, Some("union")),
    (L::C, "NamedColor", None, Some("enum")),
    (L::C, "CountAlias", None, Some("typedef")),
    (L::C, "FIXTURE_MACRO", None, Some("#define")),
    (L::C, "public_function", None, None),
    (L::C, "file_static_variable", None, None),
    (L::C, "tag_field", Some("NamedTag"), None),
    (L::Cpp, "AccessWidget", Some("fixture_space"), Some("class")),
    (L::Cpp, "PlainStruct", Some("fixture_space"), Some("struct")),
    (L::Cpp, "ScopedColor", Some("fixture_space"), Some("enum")),
    (L::Cpp, "SizeAlias", Some("fixture_space"), Some("using")),
    (L::Cpp, "IntTypedef", Some("fixture_space"), Some("typedef")),
    (L::Cpp, "fixture_space", None, Some("namespace")),
    (L::Cpp, "namespaceFunction", Some("fixture_space"), None),
    (L::Cpp, "instanceMethod", Some("AccessWidget"), None),
    (L::Php, "topLevelFunction", None, Some("function")),
    (L::Php, "greet", Some("Greets"), Some("function")),
    (L::Php, "PublicWidget", None, Some("class")),
    (L::Php, "Shape", None, Some("interface")),
    (L::Php, "Greets", None, Some("trait")),
    (L::Php, "Suit", None, Some("enum")),
    (L::Php, "CLASS_CONSTANT", Some("ConstantForms"), Some("const")),
    (L::Php, "legacyUnmarkedField", Some("PublicWidget"), Some("var")),
    (L::Php, "publicField", Some("PublicWidget"), None),
    (L::Python, "module_function", None, Some("def")),
    (L::Python, "instance_method", Some("PlainWidget"), Some("def")),
    (L::Python, "PlainWidget", None, Some("class")),
    (L::Python, "MODULE_CONSTANT", None, None),
    (L::Python, "local_value", None, None),
    // preceded by `# … def class async lambda`
    (L::Python, "BOUNDARY_TABLE", None, None),
    (L::Perl, "greet", Some("Fixture::Greeter"), Some("sub")),
    (L::Perl, "Fixture::Greeter", None, Some("package")),
    (L::Perl, "$package_level_our", Some("Fixture::Greeter"), Some("our")),
    (L::Perl, "$joined_name", Some("greet"), Some("my")),
    (L::Perl, "$call_count", Some("greet"), Some("state")),
    (L::Lua, "globalFunction", None, Some("function")),
    (L::Lua, "chunkLocalFunction", None, Some("function")),
    (L::Lua, "chunkLocalVariable", None, Some("local")),
    (L::Lua, "globalVariable", None, None),
    (L::Bash, "function_keyword_function", None, Some("function")),
    (L::Bash, "keyword_function", None, None),
    (L::Bash, "local_value", None, Some("local")),
    (L::Bash, "GLOBAL_VARIABLE", None, None),
    // preceded by `# … function local declare`
    (L::Bash, "BOUNDARY_TABLE", None, None),
    (L::Zsh, "keyword_function", None, Some("function")),
    (L::Zsh, "paren_function", None, None),
    (L::Zsh, "local_value", None, Some("local")),
    (L::Zsh, "GLOBAL_VARIABLE", None, None),
];

/// The declaration shapes the `signature_text` contract distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A function or method header keeps its modifiers and stops at the body.
    FunctionHeader,
    /// A large or multi-line initializer is excluded.
    Initializer,
    /// A multi-line header keeps its internal line breaks and indentation.
    MultiLineHeader,
    /// An annotation, attribute or decorator before the first modifier or
    /// keyword is excluded, whether or not the grammar puts it in the node.
    LeadingAnnotation,
    /// An annotation written between modifiers stays.
    AnnotationBetweenModifiers,
    /// The `=`, `=>` or `:` that introduces a body is excluded.
    BodyIntroducer,
    /// The `;` (Lua: `end`) that ends a bodyless declaration is excluded.
    Terminator,
}

pub const SHAPES: &[Shape] = &[
    Shape::FunctionHeader,
    Shape::Initializer,
    Shape::MultiLineHeader,
    Shape::LeadingAnnotation,
    Shape::AnnotationBetweenModifiers,
    Shape::BodyIntroducer,
    Shape::Terminator,
];

pub enum Case {
    /// The fixture record `name` in `container` has this `signature_text`.
    Is {
        name: &'static str,
        container: Option<&'static str>,
        signature: &'static str,
    },
    /// The language cannot express the shape.
    NotApplicable(&'static str),
    /// The language has the shape, but its grammar cannot parse it.
    Unsupported(&'static str),
}

const fn is(name: &'static str, container: Option<&'static str>, signature: &'static str) -> Case {
    Case::Is { name, container, signature }
}

pub type SignatureRow = (L, Shape, Case);

const NO_ANNOTATIONS: &str = "the language has no annotations, attributes or decorators";
const ANNOTATIONS_LEAD: &str = "annotations, attributes or decorators precede every modifier";
const NO_INTRODUCER: &str = "no token separates the header from the body";
const NO_TERMINATOR: &str = "bodyless declarations end at a newline, not a token";

pub const SIGNATURE_MATRIX: &[SignatureRow] = &[
    (L::Rust, Shape::FunctionHeader, is("in_path_function", Some("crate_inner_module"), "pub(in crate::outer_module) fn in_path_function()")),
    (L::Rust, Shape::Initializer, is("BOUNDARY_TABLE", None, "pub const BOUNDARY_TABLE: [u32; 4]")),
    (L::Rust, Shape::MultiLineHeader, is("boundary_multi_line_header", None, "pub fn boundary_multi_line_header(\n    first_value: u32,\n    second_value: u32,\n) -> u32")),
    // `#[derive]` is a sibling of the item: in neither text.
    (L::Rust, Shape::LeadingAnnotation, is("BoundaryAttributed", None, "pub struct BoundaryAttributed")),
    (L::Rust, Shape::AnnotationBetweenModifiers, Case::NotApplicable("attributes are items before the declaration, never between modifiers")),
    (L::Rust, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Rust, Shape::Terminator, is("required_trait_method", Some("PublicTrait"), "fn required_trait_method(&self) -> u32")),

    (L::Go, Shape::FunctionHeader, is("ExportedMethod", Some("ExportedStruct"), "func (receiver *ExportedStruct) ExportedMethod() string")),
    (L::Go, Shape::Initializer, is("boundaryTable", None, "boundaryTable")),
    (L::Go, Shape::MultiLineHeader, is("boundaryMultiLineHeader", None, "func boundaryMultiLineHeader(\n\tfirstValue int,\n\tsecondValue int,\n) int")),
    (L::Go, Shape::LeadingAnnotation, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Go, Shape::AnnotationBetweenModifiers, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Go, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Go, Shape::Terminator, Case::NotApplicable(NO_TERMINATOR)),

    (L::Swift, Shape::FunctionHeader, is("staticMethod", Some("OpenWidget"), "static func staticMethod() -> OpenWidget")),
    (L::Swift, Shape::Initializer, is("count", Some("PublicApi"), "public private(set) var count")),
    (L::Swift, Shape::Initializer, is("boundaryTable", None, "let boundaryTable")),
    (L::Swift, Shape::MultiLineHeader, is("boundaryMultiLineHeader", None, "func boundaryMultiLineHeader(\n    firstValue: Int,\n    secondValue: Int\n) -> Int")),
    (L::Swift, Shape::LeadingAnnotation, is("boundaryAnnotated", None, "func boundaryAnnotated()")),
    (L::Swift, Shape::AnnotationBetweenModifiers, Case::NotApplicable(ANNOTATIONS_LEAD)),
    (L::Swift, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Swift, Shape::Terminator, Case::NotApplicable(NO_TERMINATOR)),

    (L::Scala, Shape::FunctionHeader, is("packageQualifiedMethod", Some("PlainWidget"), "private[symbols] def packageQualifiedMethod(): Unit")),
    (L::Scala, Shape::Initializer, is("boundaryTable", None, "val boundaryTable: List[Int]")),
    (L::Scala, Shape::MultiLineHeader, is("boundaryMultiLineHeader", None, "def boundaryMultiLineHeader(\n    firstValue: Int,\n    secondValue: Int\n): Int")),
    (L::Scala, Shape::LeadingAnnotation, is("boundaryAnnotated", None, "def boundaryAnnotated(): Unit")),
    (L::Scala, Shape::AnnotationBetweenModifiers, Case::NotApplicable(ANNOTATIONS_LEAD)),
    (L::Scala, Shape::BodyIntroducer, is("secret", Some("Shape"), "private[Shape] def secret()")),
    (L::Scala, Shape::Terminator, Case::NotApplicable(NO_TERMINATOR)),

    (L::TypeScript, Shape::FunctionHeader, is("privateStaticMethod", Some("ExportedWidget"), "private static privateStaticMethod(): void")),
    (L::TypeScript, Shape::Initializer, is("BOUNDARY_TABLE", None, "const BOUNDARY_TABLE")),
    (L::TypeScript, Shape::Initializer, is("publicProperty", Some("ExportedWidget"), "public publicProperty: number")),
    (L::TypeScript, Shape::MultiLineHeader, is("boundaryMultiLineHeader", None, "function boundaryMultiLineHeader(\n  firstValue: number,\n  secondValue: number,\n): number")),
    // TypeScript decorators are siblings of the member: in neither text.
    (L::TypeScript, Shape::LeadingAnnotation, is("decoratedMethod", Some("BoundaryDecorated"), "decoratedMethod(): void")),
    (L::TypeScript, Shape::AnnotationBetweenModifiers, Case::NotApplicable(ANNOTATIONS_LEAD)),
    (L::TypeScript, Shape::BodyIntroducer, is("boundaryArrow", None, "(value: number): number")),
    (L::TypeScript, Shape::Terminator, Case::NotApplicable("interface members and bodyless signatures carry no terminator in the node")),

    (L::JavaScript, Shape::FunctionHeader, is("staticMethod", Some("ExportedWidget"), "static staticMethod()")),
    (L::JavaScript, Shape::Initializer, is("BOUNDARY_TABLE", None, "const BOUNDARY_TABLE")),
    (L::JavaScript, Shape::MultiLineHeader, is("boundaryMultiLineHeader", None, "function boundaryMultiLineHeader(\n  firstValue,\n  secondValue,\n)")),
    // JavaScript decorators are inside the method node.
    (L::JavaScript, Shape::LeadingAnnotation, is("decoratedMethod", Some("BoundaryDecorated"), "decoratedMethod()")),
    (L::JavaScript, Shape::AnnotationBetweenModifiers, Case::NotApplicable(ANNOTATIONS_LEAD)),
    (L::JavaScript, Shape::BodyIntroducer, is("boundaryArrow", None, "(value)")),
    (L::JavaScript, Shape::Terminator, Case::NotApplicable("the language has no bodyless function declarations")),

    (L::Java, Shape::FunctionHeader, is("staticFactory", Some("PublicWidget"), "public static PublicWidget staticFactory()")),
    (L::Java, Shape::Initializer, is("BOUNDARY_TABLE", Some("DeclarationBoundaries"), "static final int[] BOUNDARY_TABLE")),
    (L::Java, Shape::MultiLineHeader, is("boundaryMultiLineHeader", Some("DeclarationBoundaries"), "int boundaryMultiLineHeader(\n            int firstValue,\n            int secondValue)")),
    (L::Java, Shape::LeadingAnnotation, is("boundaryAnnotated", Some("DeclarationBoundaries"), "void boundaryAnnotated()")),
    (L::Java, Shape::LeadingAnnotation, is("AnnotatedPackageWidget", None, "class AnnotatedPackageWidget")),
    (L::Java, Shape::AnnotationBetweenModifiers, is("boundaryAnnotationBetweenModifiers", Some("DeclarationBoundaries"), "public @Deprecated static void boundaryAnnotationBetweenModifiers()")),
    (L::Java, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Java, Shape::Terminator, is("area", Some("PublicShape"), "double area()")),

    (L::CSharp, Shape::FunctionHeader, is("ProtectedInternalMethod", Some("PublicWidget"), "protected internal void ProtectedInternalMethod()")),
    (L::CSharp, Shape::Initializer, is("BoundaryTable", Some("DeclarationBoundaries"), "static readonly int[] BoundaryTable")),
    (L::CSharp, Shape::Initializer, is("Count", Some("PublicWidget"), "public int Count")),
    (L::CSharp, Shape::MultiLineHeader, is("BoundaryMultiLineHeader", Some("DeclarationBoundaries"), "int BoundaryMultiLineHeader(\n        int firstValue,\n        int secondValue)")),
    (L::CSharp, Shape::LeadingAnnotation, is("BoundaryAnnotated", Some("DeclarationBoundaries"), "void BoundaryAnnotated()")),
    (L::CSharp, Shape::AnnotationBetweenModifiers, Case::NotApplicable("attribute sections must precede the modifiers (`public [Obsolete] static` does not parse)")),
    // `=> PrivateField`: the expression body starts at `=>`.
    (L::CSharp, Shape::BodyIntroducer, is("PublicMethod", Some("PublicWidget"), "public int PublicMethod()")),
    (L::CSharp, Shape::Terminator, is("Area", Some("IShape"), "double Area()")),

    (L::C, Shape::FunctionHeader, is("file_static_function", None, "static int file_static_function(void)")),
    (L::C, Shape::Initializer, is("boundary_table", None, "const int boundary_table[]")),
    (L::C, Shape::MultiLineHeader, is("boundary_multi_line_header", None, "int boundary_multi_line_header(\n    int first_value,\n    int second_value)")),
    (L::C, Shape::LeadingAnnotation, is("boundary_annotated", None, "int boundary_annotated(void)")),
    (L::C, Shape::AnnotationBetweenModifiers, Case::NotApplicable(ANNOTATIONS_LEAD)),
    (L::C, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::C, Shape::Terminator, is("extern_declared_variable", None, "extern int extern_declared_variable")),

    (L::Cpp, Shape::FunctionHeader, is("instanceMethod", Some("AccessWidget"), "int instanceMethod() const")),
    (L::Cpp, Shape::Initializer, is("boundaryTable", None, "const int boundaryTable[]")),
    (L::Cpp, Shape::MultiLineHeader, is("boundaryMultiLineHeader", None, "int boundaryMultiLineHeader(\n    int firstValue,\n    int secondValue)")),
    (L::Cpp, Shape::LeadingAnnotation, is("boundaryAnnotated", None, "int boundaryAnnotated()")),
    (L::Cpp, Shape::AnnotationBetweenModifiers, Case::NotApplicable(ANNOTATIONS_LEAD)),
    (L::Cpp, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Cpp, Shape::Terminator, is("staticFactory", Some("AccessWidget"), "static AccessWidget staticFactory()")),

    (L::Php, Shape::FunctionHeader, is("staticFactory", Some("PublicWidget"), "public static function staticFactory(): self")),
    (L::Php, Shape::Initializer, is("BOUNDARY_TABLE", None, "const BOUNDARY_TABLE")),
    (L::Php, Shape::Initializer, is("privateStaticField", Some("PublicWidget"), "private static int $privateStaticField")),
    (L::Php, Shape::MultiLineHeader, is("boundary_multi_line_header", None, "function boundary_multi_line_header(\n    int $firstValue,\n    int $secondValue\n): int")),
    (L::Php, Shape::LeadingAnnotation, is("boundary_annotated", None, "function boundary_annotated(): void")),
    (L::Php, Shape::AnnotationBetweenModifiers, Case::NotApplicable(ANNOTATIONS_LEAD)),
    // The `:` before a return type is part of the header.
    (L::Php, Shape::BodyIntroducer, is("topLevelFunction", None, "function topLevelFunction(int $parameterValue): int")),
    (L::Php, Shape::Terminator, is("area", Some("Shape"), "public function area(): float")),

    (L::Python, Shape::FunctionHeader, is("instance_method", Some("PlainWidget"), "def instance_method(self) -> int")),
    (L::Python, Shape::Initializer, is("BOUNDARY_TABLE", None, "BOUNDARY_TABLE")),
    (L::Python, Shape::Initializer, is("annotated_instance_field_with_value", Some("PlainWidget"), "annotated_instance_field_with_value: str")),
    (L::Python, Shape::MultiLineHeader, is("boundary_multi_line_header", None, "def boundary_multi_line_header(\n    first_value: int,\n    second_value: int,\n) -> int")),
    // The decorator belongs to the `decorated_definition` wrapper.
    (L::Python, Shape::LeadingAnnotation, is("boundary_decorated", None, "def boundary_decorated() -> None")),
    (L::Python, Shape::AnnotationBetweenModifiers, Case::NotApplicable("the language has no modifiers")),
    (L::Python, Shape::BodyIntroducer, is("module_function", None, "def module_function(parameter_value: int) -> int")),
    (L::Python, Shape::Terminator, Case::NotApplicable(NO_TERMINATOR)),

    (L::Perl, Shape::FunctionHeader, is("greet", Some("Fixture::Greeter"), "sub greet")),
    (L::Perl, Shape::Initializer, is("%boundary_table", Some("Fixture::Other"), "our %boundary_table")),
    (L::Perl, Shape::MultiLineHeader, is("boundary_multi_line_header", Some("Fixture::Other"), "sub boundary_multi_line_header")),
    (L::Perl, Shape::LeadingAnnotation, Case::NotApplicable("subroutine attributes follow the name; none lead")),
    (L::Perl, Shape::AnnotationBetweenModifiers, Case::NotApplicable("the language has no modifiers")),
    (L::Perl, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Perl, Shape::Terminator, is("Fixture::Greeter", None, "package Fixture::Greeter")),

    (L::Lua, Shape::FunctionHeader, is("chunkLocalFunction", None, "local function chunkLocalFunction()")),
    (L::Lua, Shape::Initializer, is("boundaryTable", None, "boundaryTable")),
    (L::Lua, Shape::MultiLineHeader, is("boundaryMultiLineHeader", Some("Widget"), "function Widget.boundaryMultiLineHeader(\n  firstValue,\n  secondValue\n)")),
    (L::Lua, Shape::LeadingAnnotation, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Lua, Shape::AnnotationBetweenModifiers, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Lua, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    // An empty function has no body node; its `end` is the terminator.
    (L::Lua, Shape::Terminator, is("nestedBlockLocalFunction", None, "local function nestedBlockLocalFunction()")),

    (L::Bash, Shape::FunctionHeader, is("function_keyword_with_parens", None, "function function_keyword_with_parens()")),
    (L::Bash, Shape::Initializer, is("BOUNDARY_TABLE", None, "BOUNDARY_TABLE")),
    (L::Bash, Shape::MultiLineHeader, is("boundary_multi_line_header", None, "boundary_multi_line_header ()")),
    (L::Bash, Shape::LeadingAnnotation, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Bash, Shape::AnnotationBetweenModifiers, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Bash, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Bash, Shape::Terminator, Case::NotApplicable(NO_TERMINATOR)),

    (L::Zsh, Shape::FunctionHeader, is("keyword_with_parens", None, "function keyword_with_parens()")),
    (L::Zsh, Shape::Initializer, is("BOUNDARY_TABLE", None, "BOUNDARY_TABLE")),
    (L::Zsh, Shape::MultiLineHeader, Case::Unsupported("tree-sitter-zsh rejects a body brace on the line after the header")),
    (L::Zsh, Shape::LeadingAnnotation, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Zsh, Shape::AnnotationBetweenModifiers, Case::NotApplicable(NO_ANNOTATIONS)),
    (L::Zsh, Shape::BodyIntroducer, Case::NotApplicable(NO_INTRODUCER)),
    (L::Zsh, Shape::Terminator, Case::NotApplicable(NO_TERMINATOR)),
];

fn rows_for<T>(matrix: &'static [T], language: L, row_language: fn(&T) -> L) -> Vec<&'static T> {
    matrix.iter().filter(|row| row_language(row) == language).collect()
}

/// The record `name` in `container`, or a failure naming the containers that
/// do hold a record of that name.
fn find<'a>(
    extracted: &'a Extracted,
    language: L,
    name: &str,
    container: Option<&str>,
) -> Result<&'a SymbolRecord, String> {
    let named: Vec<&SymbolRecord> = extracted
        .records
        .iter()
        .filter(|record| record.identity.name == name)
        .collect();
    let mut matching = named
        .iter()
        .filter(|record| record.identity.container_name.as_deref() == container);
    match (matching.next(), matching.next()) {
        (Some(record), None) => Ok(record),
        _ => Err(format!(
            "{}: expected one `{name}` in {container:?}; `{name}` records are in {:?}",
            language.name(),
            named.iter().map(|record| record.identity.container_name.as_deref()).collect::<Vec<_>>()
        )),
    }
}

crate::language_cases::language_tests!(keyword_matrix_holds_for_every_language => check_keyword_matrix_holds_for_every_language);

fn check_keyword_matrix_holds_for_every_language(language: ProgrammingLanguage) {
    let mut failures = Vec::new();
    for language in [language] {
        let rows = rows_for(KEYWORD_MATRIX, language, |row| row.0);
        if rows.is_empty() {
            continue;
        }
        let extracted = extract_primary(language);
        for (_, name, container, keyword) in rows {
            match find(&extracted, language, name, *container) {
                Ok(record) if record.identity.keyword.as_deref() != *keyword => failures.push(format!(
                    "{}: keyword of `{name}` is {:?}, expected {keyword:?}",
                    language.name(),
                    record.identity.keyword
                )),
                Ok(_) => {}
                Err(failure) => failures.push(failure),
            }
        }
    }
    assert!(failures.is_empty(), "keyword matrix:\n{}", failures.join("\n"));
}

crate::language_cases::language_tests!(signature_matrix_holds_for_every_language => check_signature_matrix_holds_for_every_language);

fn check_signature_matrix_holds_for_every_language(language: ProgrammingLanguage) {
    let mut failures = Vec::new();
    for language in [language] {
        let rows = rows_for(SIGNATURE_MATRIX, language, |row| row.0);
        if rows.is_empty() {
            continue;
        }
        let extracted = extract_primary(language);
        for (_, shape, case) in rows {
            let (name, container, signature) = match case {
                Case::Is { name, container, signature } => (name, container, signature),
                Case::NotApplicable(reason) | Case::Unsupported(reason) => {
                    assert!(!reason.is_empty(), "{} {shape:?}: state why", language.name());
                    continue;
                }
            };
            match find(&extracted, language, name, *container) {
                Ok(record) if record.source.signature_text.as_deref() != Some(*signature) => {
                    failures.push(format!(
                        "{} {shape:?}: signature_text of `{name}` is {:?}, expected {signature:?}",
                        language.name(),
                        record.source.signature_text
                    ))
                }
                Ok(_) => {}
                Err(failure) => failures.push(failure),
            }
        }
    }
    assert!(failures.is_empty(), "signature matrix:\n{}", failures.join("\n"));
}

crate::language_cases::language_tests!(every_fixture_signature_is_a_contiguous_part_of_its_declaration_text => check_every_fixture_signature_is_a_contiguous_part_of_its_declaration_text);

/// Every header in every fixture is a trimmed, contiguous part of its
/// declaration text that stops before the body.
fn check_every_fixture_signature_is_a_contiguous_part_of_its_declaration_text(language: ProgrammingLanguage) {
    let mut checked = 0;
    for path in crate::symbols_fixtures::fixtures_for(language, false) {
        let extracted = extract_path(&path);
        for record in &extracted.records {
            let Some(signature) = record.source.signature_text.as_deref() else {
                continue;
            };
            checked += 1;
            let label = format!("{}: `{}`", extracted.label, record.identity.name);
            let declaration = record
                .source
                .declaration_text
                .as_deref()
                .unwrap_or_else(|| panic!("{label}: a signature without declaration text"));
            assert!(!signature.is_empty(), "{label}: empty signature_text");
            assert_eq!(signature, signature.trim(), "{label}: signature_text keeps outer whitespace");
            assert!(
                declaration.contains(signature),
                "{label}: signature_text {signature:?} is not part of declaration_text {declaration:?}"
            );
            assert!(
                !signature.ends_with(';') && !signature.ends_with('='),
                "{label}: signature_text {signature:?} keeps its terminator or `=`"
            );
        }
    }
    assert!(checked >= crate::language_cases::corpus_minimums(language).signatures, "{language}: corpus coverage shrank ({checked})");
}

/// One inline source and the record whose `signature_text` it pins.
#[derive(Debug)]
struct Inline {
    extension: &'static str,
    source: &'static str,
    name: &'static str,
    container: Option<&'static str>,
    expected: Option<&'static str>,
}

/// One row of the documented comparison table, as a complete source.
#[derive(Debug)]
struct Documented {
    extension: &'static str,
    source: &'static str,
    name: &'static str,
    container: Option<&'static str>,
    declaration_text: &'static str,
    signature_text: &'static str,
    keyword: Option<&'static str>,
}

/// A header that cannot be isolated, or that an earlier name's initializer
/// would end before this name, is `None` rather than a misleading header.
#[test]
fn headers_that_cannot_be_isolated_are_none() {
    let cases = [
        Inline { extension: "c", source: "typedef struct { int x; } Point;\n", name: "Point", container: None, expected: None },
        Inline { extension: "c", source: "struct Pair { int a; } pair;\n", name: "pair", container: None, expected: None },
        Inline { extension: "c", source: "int first = 1, second = 2;\n", name: "first", container: None, expected: Some("int first") },
        Inline { extension: "c", source: "int first = 1, second = 2;\n", name: "second", container: None, expected: None },
        Inline { extension: "java", source: "class Holder {\n    int first = 1, second = 2;\n}\n", name: "second", container: Some("Holder"), expected: None },
        Inline { extension: "ts", source: "const first = 1, second = 2;\n", name: "second", container: None, expected: None },
        Inline { extension: "py", source: "first = second = 0\n", name: "second", container: None, expected: None },
        Inline { extension: "swift", source: "var first = 1, second = 2\n", name: "second", container: None, expected: None },
    ];
    for case in &cases {
        let extracted = extract_inline(case.source, case.extension);
        assert_eq!(
            extracted.record(case.name, case.container).source.signature_text.as_deref(),
            case.expected,
            "{case:?}"
        );
    }
}

/// Each type of a grouped Go `type ( … )` block is declared by its own spec:
/// its own header, declaration text and span, with the group's `type`
/// keyword. An ungrouped type keeps the `type` keyword in its texts.
#[test]
fn grouped_go_types_have_their_own_headers_and_spans() {
    let source = "package p\n\ntype (\n\tFirst struct{ a int }\n\tSecond int\n)\n\ntype Single struct{ b int }\n";
    let extracted = extract_inline(source, "go");
    let cases = [
        ("First", "First struct{ a int }", "First struct"),
        ("Second", "Second int", "Second int"),
        ("Single", "type Single struct{ b int }", "type Single struct"),
    ];
    for (name, declaration_text, signature_text) in cases {
        let record = extracted.record(name, None);
        assert_eq!(record.source.declaration_text.as_deref(), Some(declaration_text), "{name}");
        assert_eq!(record.source.signature_text.as_deref(), Some(signature_text), "{name}");
        assert_eq!(record.identity.keyword.as_deref(), Some("type"), "{name}");
        let span = &record.source.declaration_span;
        assert_eq!(
            &source[span.start_byte as usize..span.end_byte as usize],
            declaration_text,
            "{name}: the span is the type's own"
        );
    }
}

/// A comment inside the declaration node that names other keywords never
/// supplies the keyword, and a construct with no keyword stays `None`.
#[test]
fn a_keyword_inside_a_comment_never_becomes_the_keyword() {
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        // (extension, source, name, keyword)
        ("rs", "pub /* struct enum */ fn commented() {}\n", "commented", Some("fn")),
        ("go", "package p\n\nfunc /* type var */ commented() {}\n", "commented", Some("func")),
        ("c", "static /* struct union */ int commented(void) { return 0; }\n", "commented", None),
        ("ts", "export /* class */ function commented(): void {}\n", "commented", Some("function")),
        ("php", "<?php\n/* class */ function commented(): void {}\n", "commented", Some("function")),
        ("sh", "commented() { # function local\n  :\n}\n", "commented", None),
    ];
    for (extension, source, name, keyword) in cases {
        let extracted = extract_inline(source, extension);
        assert_eq!(
            extracted.record(name, None).identity.keyword.as_deref(),
            *keyword,
            ".{extension} `{name}` in {source:?}"
        );
    }
}

/// The declarations of the `declaration_text` / `signature_text` / `keyword`
/// comparison table in `docs/symbol-records/fields.md`, as complete sources
/// (the table elides initializers and bodies with `…`).
#[test]
fn the_documented_comparison_rows_hold() {
    let rows = [
        Documented {
            extension: "rs",
            source: "mod a { pub(in crate::a) fn f() {} }\n",
            name: "f",
            container: Some("a"),
            declaration_text: "pub(in crate::a) fn f()",
            signature_text: "pub(in crate::a) fn f()",
            keyword: Some("fn"),
        },
        Documented {
            extension: "rs",
            source: "pub const TABLE: [u8; 4] = [0, 1, 2, 3];\n",
            name: "TABLE",
            container: None,
            declaration_text: "pub const TABLE: [u8; 4] = [0, 1, 2, 3];",
            signature_text: "pub const TABLE: [u8; 4]",
            keyword: Some("const"),
        },
        Documented {
            extension: "swift",
            source: "struct S {\n    public private(set) var count = 0\n}\n",
            name: "count",
            container: Some("S"),
            declaration_text: "public private(set) var count = 0",
            signature_text: "public private(set) var count",
            keyword: Some("var"),
        },
        Documented {
            extension: "java",
            source: "class C {\n    @Override public void run() {}\n}\n",
            name: "run",
            container: Some("C"),
            declaration_text: "@Override public void run()",
            signature_text: "public void run()",
            keyword: None,
        },
        Documented {
            extension: "java",
            source: "class C {\n    @Override public @Nullable String name() { return null; }\n}\n",
            name: "name",
            container: Some("C"),
            declaration_text: "@Override public @Nullable String name()",
            signature_text: "public @Nullable String name()",
            keyword: None,
        },
        Documented {
            extension: "scala",
            source: "class Shape {\n  private[Shape] def secret() = 2\n}\n",
            name: "secret",
            container: Some("Shape"),
            declaration_text: "private[Shape] def secret() =",
            signature_text: "private[Shape] def secret()",
            keyword: Some("def"),
        },
        Documented {
            extension: "cs",
            source: "class W {\n    public int Count { get; private set; }\n}\n",
            name: "Count",
            container: Some("W"),
            declaration_text: "public int Count { get; private set; }",
            signature_text: "public int Count",
            keyword: None,
        },
    ];
    for row in &rows {
        let extracted = extract_inline(row.source, row.extension);
        let record = extracted.record(row.name, row.container);
        assert_eq!(record.source.declaration_text.as_deref(), Some(row.declaration_text), "{row:?}: declaration_text");
        assert_eq!(record.source.signature_text.as_deref(), Some(row.signature_text), "{row:?}: signature_text");
        assert_eq!(record.identity.keyword.as_deref(), row.keyword, "{row:?}: keyword");
    }
}

/// Declaring keywords of constructs the per-language fixtures do not hold,
/// one inline source each.
/// Each name of a grouped Scala `val`/`var` is its own record with its own
/// name span, and shares the declaration's keyword, header and doc comment:
/// the header holds every name, so it contains each record's own.
#[test]
fn scala_grouped_names_share_their_declarations_header() {
    let extracted = extract_inline(
        "class C {\n  /** Pair. */\n  val a, b: Int = 1\n  var u, w = 2\n  val e, f: Long\n}\n",
        "scala",
    );
    let cases = [
        ("a", "val", "val a, b: Int", (3, 7)),
        ("b", "val", "val a, b: Int", (3, 10)),
        ("u", "var", "var u, w", (4, 7)),
        ("w", "var", "var u, w", (4, 10)),
        ("e", "val", "val e, f: Long", (5, 7)),
        ("f", "val", "val e, f: Long", (5, 10)),
    ];
    for (name, keyword, header, (line, column)) in cases {
        let record = extracted.record(name, Some("C"));
        assert_eq!(record.identity.keyword.as_deref(), Some(keyword), "`{name}` keyword");
        assert_eq!(record.source.signature_text.as_deref(), Some(header), "`{name}` header");
        let start = record.source.name_span.as_ref().expect("name span").start;
        assert_eq!((start.line, start.column), (line, column), "`{name}` name span");
    }
    for name in ["a", "b"] {
        assert_eq!(
            extracted.symbol(name, Some("C")).doc_comment.as_deref(),
            Some("Pair."),
            "`{name}` doc comment"
        );
    }
}

#[test]
fn less_common_declarations_report_their_keyword() {
    let cases: &[(&str, &str, &str, Option<&str>, &str)] = &[
        // (extension, source, name, container, keyword)
        ("rs", "macro_rules! square { ($x:expr) => { $x * $x }; }\n", "square", None, "macro_rules!"),
        ("swift", "actor Counter {}\n", "Counter", None, "actor"),
        ("ts", "let counter = 0;\n", "counter", None, "let"),
        ("ts", "function over(a: number): void;\nfunction over(a: any): void {}\n", "over", None, "function"),
        ("cs", "class E {\n    public event System.Action Changed;\n}\n", "Changed", Some("E"), "event"),
        ("cpp", "template <typename T> T identity(T value) { return value; }\n", "T", None, "typename"),
        ("c", "#define SQUARE(x) ((x) * (x))\n", "SQUARE", None, "#define"),
        ("go", "package p\n\nvar (\n\ta = 1\n\tb = 2\n)\n", "b", None, "var"),
        ("go", "package p\n\ntype (\n\tA int\n\tB string\n)\n", "B", None, "type"),
        ("lua", "local a, b = 1, 2\n", "b", None, "local"),
        ("sh", "f() {\n  declare counter=1\n}\n", "counter", None, "declare"),
        ("sh", "export PATH_EXTRA=/opt\n", "PATH_EXTRA", None, "export"),
        ("zsh", "f() {\n  typeset counter=1\n}\n", "counter", None, "typeset"),
    ];
    let mut failures = Vec::new();
    for (extension, source, name, container, keyword) in cases {
        let extracted = extract_inline(source, extension);
        let found: Vec<&SymbolRecord> = extracted
            .records
            .iter()
            .filter(|record| record.identity.name == *name && record.identity.container_name.as_deref() == *container)
            .collect();
        match found.as_slice() {
            [signature, implementation] if *name == "over"
                && signature.identity.keyword.as_deref() == Some(*keyword)
                && implementation.identity.keyword.as_deref() == Some(*keyword)
                && signature.source.body_span.is_none()
                && implementation.source.body_span.is_some() => {}
            [record] if record.identity.keyword.as_deref() == Some(*keyword) => {}
            [record] => failures.push(format!(".{extension} `{name}`: keyword {:?}, expected {keyword:?}", record.identity.keyword)),
            _ => failures.push(format!(
                ".{extension} `{name}` in {container:?}: {} records; all: {:?}",
                found.len(),
                extracted.records.iter().map(|record| (&record.identity.name, &record.identity.container_name, &record.identity.keyword)).collect::<Vec<_>>()
            )),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
