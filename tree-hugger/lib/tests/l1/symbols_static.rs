//! Test 4 of `2026-10-03-visibility-and-kind`: the static matrix (member
//! `static` on the shared flag, field copies and `Modifier::Static`) and
//! storage `static` (`Modifier::StaticStorage` / `is_static_storage`).

use tree_hugger::{Modifier, ProgrammingLanguage, SymbolKind, SymbolKindData, SymbolRecord};

use crate::symbols_fixtures::{Extracted, extract_inline, extract_primary};
use crate::symbols_kinds::rows_by_language;

use ProgrammingLanguage as L;

fn has_static(record: &SymbolRecord) -> bool {
    record.modifiers.iter().any(|modifier| matches!(modifier, Modifier::Static))
}

fn has_static_storage(record: &SymbolRecord) -> bool {
    record
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier, Modifier::StaticStorage))
}

/// Asserts the shared flags, the v2 modifiers, and every convenience copy.
pub fn assert_static(
    extracted: &Extracted,
    name: &str,
    container: Option<&str>,
    is_static: bool,
    is_static_storage: bool,
) {
    let label = &extracted.label;
    let symbol = extracted.symbol(name, container);
    assert_eq!(symbol.is_static, is_static, "{label}: `{name}` is_static");
    assert_eq!(
        symbol.is_static_storage, is_static_storage,
        "{label}: `{name}` is_static_storage"
    );
    if let Some(signature) = &symbol.signature {
        assert_eq!(signature.is_static, is_static, "{label}: `{name}` signature copy");
    }

    let record = extracted.record(name, container);
    assert_eq!(has_static(record), is_static, "{label}: `{name}` Modifier::Static");
    assert_eq!(
        has_static_storage(record),
        is_static_storage,
        "{label}: `{name}` Modifier::StaticStorage"
    );
    if let SymbolKindData::Field(field) = &record.kind_data {
        assert_eq!(field.is_static, is_static, "{label}: `{name}` FieldData copy");
    }
}

/// One committed cell of the static matrix: `(language, name, container,
/// is_static, is_static_storage)` in the language's primary fixture.
pub type StaticRow = (
    ProgrammingLanguage,
    &'static str,
    Option<&'static str>,
    bool,
    bool,
);

/// Languages with static members (spec section 4), plus C for storage
/// `static`; the coverage guard requires rows for each.
pub const STATIC_LANGUAGES: &[ProgrammingLanguage] = &[
    L::TypeScript,
    L::JavaScript,
    L::Java,
    L::CSharp,
    L::Php,
    L::Cpp,
    L::Swift,
    L::Scala,
    L::Python,
    L::Rust,
    L::C,
];

/// The static matrix: one static and one instance method, and one static and
/// one instance field where fields are extracted, per language; then storage
/// `static`, which is never member `static`.
pub const STATIC_MATRIX: &[StaticRow] = &[
    (L::TypeScript, "staticMethod", Some("ExportedWidget"), true, false),
    (L::TypeScript, "privateStaticMethod", Some("ExportedWidget"), true, false),
    (L::TypeScript, "publicMethod", Some("ExportedWidget"), false, false),
    (L::TypeScript, "staticProperty", Some("ExportedWidget"), true, false),
    (L::TypeScript, "privateStaticProperty", Some("ExportedWidget"), true, false),
    (L::TypeScript, "publicProperty", Some("ExportedWidget"), false, false),
    (L::TypeScript, "exportedFunction", None, false, false),
    (L::JavaScript, "staticMethod", Some("ExportedWidget"), true, false),
    (L::JavaScript, "unmarkedMethod", Some("ExportedWidget"), false, false),
    (L::JavaScript, "staticProperty", Some("ExportedWidget"), true, false),
    (L::JavaScript, "uninitializedStaticProperty", Some("ExportedWidget"), true, false),
    (L::JavaScript, "unmarkedProperty", Some("ExportedWidget"), false, false),
    (L::Java, "staticFactory", Some("PublicWidget"), true, false),
    (L::Java, "publicMethod", Some("PublicWidget"), false, false),
    (L::Java, "publicStaticField", Some("PublicWidget"), true, false),
    (L::Java, "publicField", Some("PublicWidget"), false, false),
    // interface constants are implicitly `public static final`
    (L::Java, "SIDES", Some("PublicShape"), true, false),
    (L::Java, "unit", Some("PublicShape"), true, false),
    (L::Java, "area", Some("PublicShape"), false, false),
    (L::CSharp, "StaticFactory", Some("PublicWidget"), true, false),
    (L::CSharp, "PublicMethod", Some("PublicWidget"), false, false),
    (L::CSharp, "PublicStaticField", Some("PublicWidget"), true, false),
    (L::CSharp, "PublicField", Some("PublicWidget"), false, false),
    (L::CSharp, "Unit", Some("IShape"), true, false),
    (L::Php, "staticFactory", Some("PublicWidget"), true, false),
    (L::Php, "unmarkedStaticMethod", Some("PublicWidget"), true, false),
    (L::Php, "area", Some("PublicWidget"), false, false),
    (L::Php, "publicStaticField", Some("PublicWidget"), true, false),
    (L::Php, "publicField", Some("PublicWidget"), false, false),
    (L::Php, "traitStaticMethod", Some("Greets"), true, false),
    (L::Cpp, "staticFactory", Some("AccessWidget"), true, false),
    (L::Cpp, "firstPublicMethod", Some("AccessWidget"), false, false),
    (L::Cpp, "staticMemberField", Some("AccessWidget"), true, false),
    (L::Cpp, "firstPublicField", Some("AccessWidget"), false, false),
    (L::Swift, "staticMethod", Some("OpenWidget"), true, false),
    (L::Swift, "classMethod", Some("OpenWidget"), true, false),
    (L::Swift, "instanceMethod", Some("OpenWidget"), false, false),
    (L::Swift, "staticProperty", Some("OpenWidget"), true, false),
    (L::Swift, "classProperty", Some("OpenWidget"), true, false),
    (L::Swift, "publicProperty", Some("OpenWidget"), false, false),
    // `object` members are static; class members are not
    (L::Scala, "objectMethod", Some("StandaloneObject"), true, false),
    (L::Scala, "companionStaticMethod", Some("PlainWidget"), true, false),
    (L::Scala, "companionStaticField", Some("PlainWidget"), true, false),
    (L::Scala, "publicMethod", Some("PlainWidget"), false, false),
    (L::Scala, "publicField", Some("PlainWidget"), false, false),
    (L::Python, "static_method", Some("PlainWidget"), true, false),
    (L::Python, "class_method", Some("PlainWidget"), true, false),
    (L::Python, "instance_method", Some("PlainWidget"), false, false),
    // the annotation decides: `x: T` and `x: T = v` are instance fields,
    // `x: ClassVar[T]` and unannotated `x = v` class attributes
    (L::Python, "annotated_instance_field", Some("PlainWidget"), false, false),
    (L::Python, "annotated_instance_field_with_value", Some("PlainWidget"), false, false),
    (L::Python, "class_var_static_field", Some("PlainWidget"), true, false),
    (L::Python, "class_var_static_field_with_value", Some("PlainWidget"), true, false),
    (L::Python, "unannotated_static_field", Some("PlainWidget"), true, false),
    (L::Python, "MODULE_CONSTANT", None, false, false),
    (L::Rust, "new_associated_function", Some("PublicStruct"), true, false),
    (L::Rust, "crate_associated_function", Some("PublicStruct"), true, false),
    (L::Rust, "instance_method", Some("PublicStruct"), false, false),
    (L::Rust, "public_function", None, false, false),
    // storage `static`
    (L::Rust, "PUBLIC_STATIC", None, false, true),
    (L::Rust, "PRIVATE_STATIC_MUT", None, false, true),
    (L::Rust, "LOCAL_STATIC", None, false, true),
    (L::Rust, "PUBLIC_CONSTANT", None, false, false),
    // implicitly static: associated constants, inherent and trait
    (L::Rust, "INHERENT_ASSOCIATED_CONSTANT", Some("PublicStruct"), true, false),
    (L::Rust, "TRAIT_ASSOCIATED_CONSTANT", Some("PublicTrait"), true, false),
    (L::Rust, "TRAIT_ASSOCIATED_CONSTANT", Some("PublicStruct"), true, false),
    // implicitly static: C# `const` fields in a class, struct and interface
    (L::CSharp, "ClassConstant", Some("ConstantForms"), true, false),
    (L::CSharp, "StructConstant", Some("ConstantStruct"), true, false),
    (L::CSharp, "InterfaceConstant", Some("IConstantForms"), true, false),
    // implicitly static: PHP class, interface, trait and enum constants; a
    // top-level `const` is not a member
    (L::Php, "CLASS_CONSTANT", Some("ConstantForms"), true, false),
    (L::Php, "INTERFACE_CONSTANT", Some("ConstantContract"), true, false),
    (L::Php, "TRAIT_CONSTANT", Some("ConstantTrait"), true, false),
    (L::Php, "ENUM_CONSTANT", Some("ConstantEnum"), true, false),
    (L::Php, "TOP_CONSTANT", None, false, false),
    (L::Cpp, "memberLocalStatic", Some("AccessWidget"), false, true),
    (L::Cpp, "fileStaticVariable", None, false, true),
    (L::Cpp, "fileStaticFunction", None, false, true),
    (L::Cpp, "localStaticCounter", None, false, true),
    (L::Cpp, "publicGlobal", None, false, false),
    (L::C, "file_static_variable", None, false, true),
    (L::C, "file_static_function", None, false, true),
    (L::C, "local_static_counter", None, false, true),
    (L::C, "public_global_variable", None, false, false),
    (L::C, "public_function", None, false, false),
];

crate::language_cases::language_tests!(static_matrix_holds_through_both_paths => check_static_matrix_holds_through_both_paths);

/// Test 4: every static-matrix row holds on the shared flags, the v2
/// modifiers, and every convenience copy.
fn check_static_matrix_holds_through_both_paths(language: ProgrammingLanguage) {
    for (language, rows) in rows_by_language(STATIC_MATRIX, |row| row.0).into_iter().filter(|(row_language, _)| *row_language == language) {
        let extracted = extract_primary(language);
        for (_, name, container, is_static, is_static_storage) in rows {
            assert_static(&extracted, name, *container, *is_static, *is_static_storage);
        }
    }
}

/// `hug symbols` (standalone fields) and `hug classes` (type metadata) list
/// the same Python class-body fields with the same static flag.
#[test]
fn python_class_fields_agree_with_the_standalone_fields() {
    let python = extract_primary(ProgrammingLanguage::Python);
    let mut from_metadata: Vec<(String, bool)> = python
        .symbol("PlainWidget", None)
        .type_metadata
        .as_ref()
        .expect("PlainWidget metadata")
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.is_static))
        .collect();
    let mut standalone: Vec<(String, bool)> = python
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.kind == SymbolKind::Field && symbol.container_name.as_deref() == Some("PlainWidget")
        })
        .map(|symbol| (symbol.name.clone(), symbol.is_static))
        .collect();
    from_metadata.sort();
    standalone.sort();
    assert_eq!(from_metadata, standalone);
    assert_eq!(standalone.len(), 5, "{standalone:?}");
}

/// Review finding 4: Python member `static` comes from the decorator
/// expression itself, `staticmethod` or `classmethod`, bare or as
/// `builtins.…`, and only on a method declared in a class body. A decorator
/// whose text merely contains those names applies some other callable.
#[test]
fn python_static_reads_the_decorator_name_not_its_text() {
    let python = extract_inline(
        "import builtins\n\
         @staticmethod\n\
         def top_level_probe(): pass\n\
         class Sweep:\n    \
             @notstaticmethod\n    \
             def decoratorProbe(self): pass\n    \
             @notclassmethod\n    \
             def decoratorClassProbe(self): pass\n    \
             @staticmethod_helper\n    \
             def helperProbe(self): pass\n    \
             @wrap(staticmethod)\n    \
             def argumentProbe(self): pass\n    \
             @other.staticmethod\n    \
             def attributeProbe(self): pass\n    \
             @staticmethod()\n    \
             def callProbe(): pass\n    \
             @builtins.staticmethod\n    \
             def qualifiedStatic(): pass\n    \
             @builtins.classmethod\n    \
             def qualifiedClass(cls): pass\n    \
             @staticmethod\n    \
             def realStatic(): pass\n    \
             @classmethod\n    \
             def realClass(cls): pass\n    \
             @abc.abstractmethod\n    \
             @staticmethod\n    \
             def stackedStatic(): pass\n    \
             def instanceProbe(self): pass\n",
        "py",
    );
    let class = Some("Sweep");

    for name in [
        "decoratorProbe",
        "decoratorClassProbe",
        "helperProbe",
        "argumentProbe",
        "attributeProbe",
        "callProbe",
        "instanceProbe",
    ] {
        assert_static(&python, name, class, false, false);
    }
    for name in [
        "qualifiedStatic",
        "qualifiedClass",
        "realStatic",
        "realClass",
        "stackedStatic",
    ] {
        assert_static(&python, name, class, true, false);
    }
    // Not a member, so never member-static.
    assert_static(&python, "top_level_probe", None, false, false);
}

/// Review finding 4: a Scala member is static only when its nearest owning
/// type is an `object`; a class, trait, case class, enum, anonymous class or
/// nested object between it and an outer object decides instead.
#[test]
fn scala_static_stops_at_the_nearest_owning_type() {
    let scala = extract_inline(
        "object SweepObject {\n  \
           trait InnerTrait { def traitProbe(): Unit }\n  \
           class InnerClass { def instanceProbe(): Unit = () }\n  \
           case class InnerCase(x: Int) { def caseProbe(): Unit = () }\n  \
           enum InnerEnum { case A; def enumProbe(): Unit = () }\n  \
           val anon = new Runnable { def run(): Unit = (); val anonField = 1 }\n  \
           object Nested { def nestedProbe(): Unit = () }\n  \
           def objectProbe(): Unit = ()\n  \
           val objectField = 1\n  \
           extension (x: Int) def twice: Int = x * 2\n\
         }\n\
         class Outer { object Inner { def innerObjectProbe(): Unit = () } }\n",
        "scala",
    );

    assert_static(&scala, "traitProbe", Some("InnerTrait"), false, false);
    assert_static(&scala, "instanceProbe", Some("InnerClass"), false, false);
    assert_static(&scala, "caseProbe", Some("InnerCase"), false, false);
    assert_static(&scala, "enumProbe", Some("InnerEnum"), false, false);
    assert_static(&scala, "run", None, false, false);
    assert_static(&scala, "anonField", None, false, false);

    assert_static(&scala, "nestedProbe", Some("Nested"), true, false);
    assert_static(&scala, "objectProbe", Some("SweepObject"), true, false);
    assert_static(&scala, "objectField", Some("SweepObject"), true, false);
    assert_static(&scala, "anon", Some("SweepObject"), true, false);
    assert_static(&scala, "twice", Some("SweepObject"), true, false);
    assert_static(&scala, "innerObjectProbe", Some("Inner"), true, false);
}

/// Every name of a grouped Scala `val`/`var` takes its owner's member
/// `static`: static in an object, instance in a class, trait or enum.
#[test]
fn scala_grouped_fields_take_their_owners_static() {
    let scala = extract_inline(
        "object O {\n  val a, b: Int = 1\n  var c, d: Int = 2\n}\n\
         class K {\n  val a, b: Int = 1\n  var c, d: Int = 2\n}\n\
         trait T {\n  val a, b: Int\n  var c, d: Int\n}\n\
         enum E {\n  case V\n  val a, b: Int = 1\n  var c, d: Int = 2\n}\n",
        "scala",
    );
    for (owner, is_static) in [("O", true), ("K", false), ("T", false), ("E", false)] {
        for name in ["a", "b", "c", "d"] {
            assert_static(&scala, name, Some(owner), is_static, false);
        }
    }
}

/// Review finding 4 sweep: a typed `self` receiver (`self: Box<Self>`) makes
/// a Rust method an instance method; a parameter merely named like `self`
/// does not.
#[test]
fn rust_typed_self_receiver_is_an_instance_method() {
    let rust = extract_inline(
        "struct Sweep;\n\
         impl Sweep {\n    \
             fn boxed_probe(self: Box<Self>) {}\n    \
             fn mut_boxed_probe(mut self: Box<Self>) {}\n    \
             fn by_value(self) {}\n    \
             fn named_like_self(self_: i32) {}\n    \
             fn associated() {}\n\
         }\n",
        "rs",
    );
    let owner = Some("Sweep");

    assert_static(&rust, "boxed_probe", owner, false, false);
    assert_static(&rust, "mut_boxed_probe", owner, false, false);
    assert_static(&rust, "by_value", owner, false, false);
    assert_static(&rust, "named_like_self", owner, true, false);
    assert_static(&rust, "associated", owner, true, false);
}

/// Review finding 4 sweep: the keyword readers match the `static` token or
/// modifier itself, never a name, an annotation or attribute, or an
/// enclosing declaration.
#[test]
fn static_keyword_readers_match_the_modifier_token() {
    for extension in ["ts", "js"] {
        let ecma = extract_inline(
            "class Sweep {\n  static() {}\n  staticProbe() {}\n  static real() {}\n}\n",
            extension,
        );
        assert_static(&ecma, "static", Some("Sweep"), false, false);
        assert_static(&ecma, "staticProbe", Some("Sweep"), false, false);
        assert_static(&ecma, "real", Some("Sweep"), true, false);
    }

    let java = extract_inline(
        "class Sweep {\n  \
           static class Nested { void nestedProbe() {} }\n  \
           static Runnable field = new Runnable() { public void anonymousProbe() {} };\n  \
           @Static void annotatedProbe() {}\n\
         }\n",
        "java",
    );
    assert_static(&java, "nestedProbe", Some("Nested"), false, false);
    assert_static(&java, "anonymousProbe", None, false, false);
    assert_static(&java, "annotatedProbe", Some("Sweep"), false, false);
    assert_static(&java, "field", Some("Sweep"), true, false);

    let csharp = extract_inline(
        "class Sweep {\n  \
           static class Nested { void NestedProbe() {} }\n  \
           [Static] void AttributedProbe() {}\n\
         }\n",
        "cs",
    );
    assert_static(&csharp, "NestedProbe", Some("Nested"), false, false);
    assert_static(&csharp, "AttributedProbe", Some("Sweep"), false, false);

    let swift = extract_inline(
        "class Sweep {\n  \
           class func classProbe() {}\n  \
           final class Inner { func innerProbe() {} }\n  \
           var staticName = 1\n\
         }\n",
        "swift",
    );
    assert_static(&swift, "classProbe", Some("Sweep"), true, false);
    assert_static(&swift, "innerProbe", Some("Inner"), false, false);
    assert_static(&swift, "staticName", Some("Sweep"), false, false);

    let php = extract_inline(
        "<?php\nclass Sweep {\n  \
           public static function real() {}\n  \
           public function staticProbe() {}\n  \
           public $staticField;\n\
         }\n",
        "php",
    );
    assert_static(&php, "real", Some("Sweep"), true, false);
    assert_static(&php, "staticProbe", Some("Sweep"), false, false);
    assert_static(&php, "staticField", Some("Sweep"), false, false);
}

/// Review finding 4 sibling: a Python class-body annotation is a class
/// attribute when its expression is `ClassVar`, bare or subscripted, under
/// any qualifier; a name that merely starts with or contains `ClassVar` is an
/// ordinary annotation, so the field is an instance field.
#[test]
fn python_class_var_reads_the_annotation_expression() {
    let python = extract_inline(
        "import typing\n\
         import typing as t\n\
         class Sweep:\n    \
             bareProbe: ClassVar = 1\n    \
             subscriptProbe: ClassVar[int] = 1\n    \
             nestedProbe: ClassVar[list[int]]\n    \
             typingProbe: typing.ClassVar[int]\n    \
             aliasProbe: t.ClassVar[int] = 2\n    \
             bareTypingProbe: typing.ClassVar = 3\n    \
             likeProbe: ClassVarLike[int]\n    \
             prefixProbe: MyClassVar\n    \
             attributeLikeProbe: t.ClassVarLike[int]\n    \
             stringProbe: 'ClassVar[int]'\n    \
             instanceProbe: int = 0\n    \
             unannotatedProbe = 0\n",
        "py",
    );
    let class = Some("Sweep");

    for name in [
        "bareProbe",
        "subscriptProbe",
        "nestedProbe",
        "typingProbe",
        "aliasProbe",
        "bareTypingProbe",
        "unannotatedProbe",
    ] {
        assert_static(&python, name, class, true, false);
    }
    for name in [
        "likeProbe",
        "prefixProbe",
        "attributeLikeProbe",
        "stringProbe",
        "instanceProbe",
    ] {
        assert_static(&python, name, class, false, false);
    }
}
