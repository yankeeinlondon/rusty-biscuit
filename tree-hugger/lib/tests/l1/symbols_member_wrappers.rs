//! Review finding 2 of `2026-10-03-visibility-and-kind` (both review
//! cycles): a transparent wrapper between a declaration and its type body (a
//! C++ `template<…>` header, a C, C++ or C# preprocessor conditional) is not a
//! scope, so a wrapped member keeps its kind, member `static`, access and
//! field-list entry, and a wrapped declaration outside a type stays outside.

use tree_hugger::{SymbolKind, Visibility};

use crate::symbols_fixtures::{Extracted, extract_inline};
use crate::symbols_static::assert_static;
use crate::symbols_visibility::assert_level;

use SymbolKind as K;

const PUBLIC: Option<Visibility> = Some(Visibility::Public);
const PRIVATE: Option<Visibility> = Some(Visibility::Private);
const INTERNAL: Option<Visibility> = Some(Visibility::Internal);
const MODULE_PRIVATE: Option<Visibility> = Some(Visibility::ModulePrivate);

/// One expectation on both public paths: `(name, container, container kind,
/// kind, is_static, is_static_storage, visibility)`.
type Row = (
    &'static str,
    Option<&'static str>,
    Option<SymbolKind>,
    SymbolKind,
    bool,
    bool,
    Option<Visibility>,
);

fn assert_rows(extracted: &Extracted, rows: &[Row]) {
    for &(name, container, container_kind, kind, is_static, is_static_storage, visibility) in rows {
        extracted.assert_kind(name, container, kind, container_kind);
        assert_static(extracted, name, container, is_static, is_static_storage);
        assert_level(extracted, name, container, visibility);
    }
}

/// The names and member-`static` flags of `type_name`'s type-metadata fields.
fn metadata_fields(extracted: &Extracted, type_name: &str) -> Vec<(String, bool)> {
    let mut fields: Vec<(String, bool)> = extracted
        .symbol(type_name, None)
        .type_metadata
        .as_ref()
        .unwrap_or_else(|| panic!("{}: `{type_name}` has no type metadata", extracted.label))
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.is_static))
        .collect();
    fields.sort();
    fields
}

/// `type_name`'s type-metadata fields as `(name, is_static, visibility)`.
fn metadata_field_flags(
    extracted: &Extracted,
    type_name: &str,
    container: Option<&str>,
) -> Vec<(String, bool, Option<Visibility>)> {
    extracted
        .symbol(type_name, container)
        .type_metadata
        .as_ref()
        .unwrap_or_else(|| panic!("{}: `{type_name}` has no type metadata", extracted.label))
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.is_static, field.visibility))
        .collect()
}

/// Asserts that every `(name, is_static, visibility)` is one of `type_name`'s
/// metadata fields. Other entries are allowed, so a reader that lists more
/// member forms does not break these rows.
fn assert_fields_listed(
    extracted: &Extracted,
    type_name: &str,
    container: Option<&str>,
    expected: &[(&str, bool, Option<Visibility>)],
) {
    let fields = metadata_field_flags(extracted, type_name, container);
    for &(name, is_static, visibility) in expected {
        assert!(
            fields.contains(&(name.to_string(), is_static, visibility)),
            "{}: `{type_name}` metadata lacks `{name}` (static {is_static}, {visibility:?}): {fields:?}",
            extracted.label
        );
    }
}

#[test]
fn cpp_template_members_keep_member_kind_and_static() {
    let cpp = extract_inline(
        "class TemplateSweep {\n\
         public:\n    \
             template<class T> static void staticProbe(T x) {}\n    \
             template<class T> void instanceProbe(T x) {}\n    \
             template<class T> static T fieldProbe;\n    \
             template<class T> static void staticDeclProbe(T x);\n    \
             static void plainStatic() {}\n    \
             int countCalls() { static int localCounter = 0; return ++localCounter; }\n\
         };\n\
         template<class T> void freeTemplate(T x) {}\n\
         template<class T> static void freeStaticTemplate(T x) {}\n",
        "cpp",
    );
    let class = Some("TemplateSweep");
    let owner = Some(K::Class);
    assert_rows(
        &cpp,
        &[
            ("staticProbe", class, owner, K::Method, true, false, PUBLIC),
            ("instanceProbe", class, owner, K::Method, false, false, PUBLIC),
            ("fieldProbe", class, owner, K::Field, true, false, PUBLIC),
            ("staticDeclProbe", class, owner, K::Method, true, false, PUBLIC),
            // Controls: an unwrapped static member, a function-local static,
            // and template functions outside any type.
            ("plainStatic", class, owner, K::Method, true, false, PUBLIC),
            ("freeTemplate", None, None, K::Function, false, false, PUBLIC),
            ("freeStaticTemplate", None, None, K::Function, false, true, MODULE_PRIVATE),
        ],
    );
    let local = cpp
        .symbols
        .iter()
        .find(|symbol| symbol.name == "localCounter")
        .expect("localCounter");
    assert_eq!(local.kind, K::Variable);
    assert!(!local.is_static && local.is_static_storage, "{local:?}");

    assert_eq!(
        metadata_fields(&cpp, "TemplateSweep"),
        [("fieldProbe".to_string(), true)]
    );
}

#[test]
fn cpp_preprocessor_conditional_members_stay_members() {
    let cpp = extract_inline(
        "class Guarded {\n\
         public:\n\
         #ifdef FEATURE\n    \
             static void guardedStatic() {}\n    \
             void guardedInstance() {}\n    \
             static int guardedField;\n\
         private:\n    \
             int hiddenField;\n\
         #else\n    \
             int fallbackField;\n\
         #endif\n    \
             void afterBlock() {}\n\
         };\n\
         #ifdef FEATURE\n\
         static void guardedFree() {}\n\
         #endif\n",
        "cpp",
    );
    let class = Some("Guarded");
    let owner = Some(K::Class);
    assert_rows(
        &cpp,
        &[
            ("guardedStatic", class, owner, K::Method, true, false, PUBLIC),
            ("guardedInstance", class, owner, K::Method, false, false, PUBLIC),
            ("guardedField", class, owner, K::Field, true, false, PUBLIC),
            // A section inside the branch governs the rest of that branch;
            // the `#else` branch does not inherit it.
            ("hiddenField", class, owner, K::Field, false, false, PRIVATE),
            ("fallbackField", class, owner, K::Field, false, false, PUBLIC),
            ("afterBlock", class, owner, K::Method, false, false, PUBLIC),
            ("guardedFree", None, None, K::Function, false, true, MODULE_PRIVATE),
        ],
    );
    assert_eq!(
        metadata_fields(&cpp, "Guarded"),
        [
            ("fallbackField".to_string(), false),
            ("guardedField".to_string(), true),
            ("hiddenField".to_string(), false),
        ]
    );
}

#[test]
fn c_preprocessor_conditional_fields_and_enumerators_are_listed() {
    let c = extract_inline(
        "struct Guarded {\n    \
             int always;\n\
         #ifdef FEATURE\n    \
             int guarded;\n\
         #endif\n\
         };\n\
         enum Mode { MODE_ON,\n\
         #if FEATURE\n    \
             MODE_GUARDED,\n\
         #endif\n    \
             MODE_OFF };\n",
        "c",
    );
    c.assert_kind("guarded", Some("Guarded"), K::Field, Some(K::Type));
    assert_eq!(
        metadata_fields(&c, "Guarded"),
        [("always".to_string(), false), ("guarded".to_string(), false)]
    );

    let variants: Vec<&str> = c
        .symbol("Mode", None)
        .type_metadata
        .as_ref()
        .expect("Mode metadata")
        .variants
        .iter()
        .map(|variant| variant.name.as_str())
        .collect();
    assert_eq!(variants, ["MODE_ON", "MODE_GUARDED", "MODE_OFF"]);
}

/// A `friend` function is declared in a class body but is not a member.
#[test]
fn cpp_friend_function_stays_a_function() {
    let cpp = extract_inline(
        "class Befriends {\n    friend void befriended(Befriends& value) {}\n};\n",
        "cpp",
    );
    let befriended = cpp
        .symbols
        .iter()
        .find(|symbol| symbol.name == "befriended")
        .expect("befriended");
    assert_eq!(befriended.kind, K::Function);
    assert!(!befriended.is_static, "{befriended:?}");
}

/// C# conditional members, in every branch form and nested, under a
/// file-scoped namespace.
const CSHARP_CONDITIONALS: &str = "namespace Sweep;\n\
#if FLAG\n\
class TopIf {}\n\
#else\n\
class TopElse {}\n\
#endif\n\
class Conditional {\n\
#if FLAG\n    \
    int ifField;\n    \
    static int ifStatic;\n    \
    int IfProp { get; set; }\n    \
    void ifMethod() {}\n    \
    static void ifStaticMethod() {}\n    \
    class IfNested {}\n\
#elif OTHER\n    \
    int elifField;\n\
#else\n    \
    int elseField;\n\
#if INNER\n    \
    int nestedField;\n\
#endif\n\
#endif\n    \
    int plainField;\n\
#if FLAG\n    \
    public static int PublicStatic;\n    \
    public static int PublicProp { get; set; }\n    \
    protected int protectedField;\n\
#endif\n\
}\n\
struct ConditionalStruct {\n\
#if FLAG\n    \
    int structField;\n    \
    int StructProp { get; set; }\n    \
    void structMethod() {}\n\
#endif\n\
}\n\
record ConditionalRecord {\n\
#if FLAG\n    \
    int recordField;\n    \
    int RecordProp { get; set; }\n    \
    void recordMethod() {}\n\
#endif\n\
}\n\
interface IConditional {\n\
#if FLAG\n    \
    int Count { get; }\n    \
    static int Shared { get; }\n    \
    void Tick();\n\
#if INNER\n    \
    int Nested { get; }\n\
#endif\n\
#endif\n    \
    int Plain { get; }\n\
}\n\
enum Mode { On,\n\
#if FLAG\n    \
    Guarded\n\
#endif\n\
}\n";

#[test]
fn csharp_conditional_members_take_their_owner_default_access() {
    let cs = extract_inline(CSHARP_CONDITIONALS, "cs");
    let namespace = Some("Sweep");
    let class = Some("Conditional");
    let class_kind = Some(K::Class);
    let protected = Some(Visibility::Protected);
    assert_rows(
        &cs,
        &[
            // Top-level types under `#if` / `#else` after a file-scoped
            // namespace.
            ("TopIf", namespace, Some(K::Namespace), K::Class, false, false, INTERNAL),
            ("TopElse", namespace, Some(K::Namespace), K::Class, false, false, INTERNAL),
            // Class members in each branch form and a nested conditional.
            ("ifField", class, class_kind, K::Field, false, false, PRIVATE),
            ("ifStatic", class, class_kind, K::Field, true, false, PRIVATE),
            ("IfProp", class, class_kind, K::Property, false, false, PRIVATE),
            ("ifMethod", class, class_kind, K::Method, false, false, PRIVATE),
            ("ifStaticMethod", class, class_kind, K::Method, true, false, PRIVATE),
            ("IfNested", class, class_kind, K::Class, false, false, PRIVATE),
            ("elifField", class, class_kind, K::Field, false, false, PRIVATE),
            ("elseField", class, class_kind, K::Field, false, false, PRIVATE),
            ("nestedField", class, class_kind, K::Field, false, false, PRIVATE),
            // Explicit modifiers under a conditional, and an unwrapped member.
            ("PublicStatic", class, class_kind, K::Field, true, false, PUBLIC),
            ("PublicProp", class, class_kind, K::Property, true, false, PUBLIC),
            ("protectedField", class, class_kind, K::Field, false, false, protected),
            ("plainField", class, class_kind, K::Field, false, false, PRIVATE),
            // Struct and record members.
            ("structField", Some("ConditionalStruct"), Some(K::Type), K::Field, false, false, PRIVATE),
            ("StructProp", Some("ConditionalStruct"), Some(K::Type), K::Property, false, false, PRIVATE),
            ("structMethod", Some("ConditionalStruct"), Some(K::Type), K::Method, false, false, PRIVATE),
            ("recordField", Some("ConditionalRecord"), Some(K::Type), K::Field, false, false, PRIVATE),
            ("RecordProp", Some("ConditionalRecord"), Some(K::Type), K::Property, false, false, PRIVATE),
            ("recordMethod", Some("ConditionalRecord"), Some(K::Type), K::Method, false, false, PRIVATE),
            // Interface members, wrapped once, twice and not at all.
            ("Count", Some("IConditional"), Some(K::Interface), K::Property, false, false, PUBLIC),
            ("Shared", Some("IConditional"), Some(K::Interface), K::Property, true, false, PUBLIC),
            ("Tick", Some("IConditional"), Some(K::Interface), K::Method, false, false, PUBLIC),
            ("Nested", Some("IConditional"), Some(K::Interface), K::Property, false, false, PUBLIC),
            ("Plain", Some("IConditional"), Some(K::Interface), K::Property, false, false, PUBLIC),
        ],
    );
}

#[test]
fn csharp_conditional_types_in_a_block_namespace_are_internal() {
    let cs = extract_inline(
        "namespace Block {\n\
         #if FLAG\n    \
             class InBlock {}\n\
         #elif OTHER\n    \
             class InBlockElif {}\n\
         #else\n    \
             class InBlockElse {}\n\
         #endif\n    \
             class PlainInBlock {}\n\
         }\n",
        "cs",
    );
    let block = Some("Block");
    let namespace = Some(K::Namespace);
    assert_rows(
        &cs,
        &[
            ("InBlock", block, namespace, K::Class, false, false, INTERNAL),
            ("InBlockElif", block, namespace, K::Class, false, false, INTERNAL),
            ("InBlockElse", block, namespace, K::Class, false, false, INTERNAL),
            ("PlainInBlock", block, namespace, K::Class, false, false, INTERNAL),
        ],
    );
}

#[test]
fn csharp_conditional_fields_properties_and_enum_members_are_listed() {
    let cs = extract_inline(CSHARP_CONDITIONALS, "cs");
    let namespace = Some("Sweep");
    let protected = Some(Visibility::Protected);
    assert_fields_listed(
        &cs,
        "Conditional",
        namespace,
        &[
            ("ifField", false, PRIVATE),
            ("ifStatic", true, PRIVATE),
            ("elifField", false, PRIVATE),
            ("elseField", false, PRIVATE),
            ("nestedField", false, PRIVATE),
            ("PublicStatic", true, PUBLIC),
            ("protectedField", false, protected),
            ("plainField", false, PRIVATE),
        ],
    );
    assert_fields_listed(&cs, "ConditionalStruct", namespace, &[("structField", false, PRIVATE)]);
    assert_fields_listed(
        &cs,
        "IConditional",
        namespace,
        &[
            ("Count", false, PUBLIC),
            ("Shared", true, PUBLIC),
            ("Nested", false, PUBLIC),
            ("Plain", false, PUBLIC),
        ],
    );
    // Interface methods are standalone Methods, never field-list entries.
    assert!(
        !metadata_field_flags(&cs, "IConditional", namespace)
            .iter()
            .any(|(name, ..)| name == "Tick"),
        "{}: `Tick` is a method, not an interface field",
        cs.label
    );

    let variants: Vec<&str> = cs
        .symbol("Mode", namespace)
        .type_metadata
        .as_ref()
        .expect("Mode metadata")
        .variants
        .iter()
        .map(|variant| variant.name.as_str())
        .collect();
    assert_eq!(variants, ["On", "Guarded"]);
}

/// Control: C++ conditionals nested in one another and around a template
/// header keep the class's default access, static flags and field list.
#[test]
fn cpp_nested_conditionals_keep_default_access_and_fields() {
    let cpp = extract_inline(
        "class Nested {\n\
         #ifdef OUTER\n    \
             int outerField;\n\
         #if INNER\n    \
             static int innerStatic;\n    \
             template<class T> static T innerTemplate;\n    \
             void innerMethod() {}\n\
         #endif\n\
         #endif\n\
         };\n",
        "cpp",
    );
    let class = Some("Nested");
    let owner = Some(K::Class);
    assert_rows(
        &cpp,
        &[
            ("outerField", class, owner, K::Field, false, false, PRIVATE),
            ("innerStatic", class, owner, K::Field, true, false, PRIVATE),
            ("innerTemplate", class, owner, K::Field, true, false, PRIVATE),
            ("innerMethod", class, owner, K::Method, false, false, PRIVATE),
        ],
    );
    assert_fields_listed(
        &cpp,
        "Nested",
        None,
        &[
            ("outerField", false, PRIVATE),
            ("innerStatic", true, PRIVATE),
            ("innerTemplate", true, PRIVATE),
        ],
    );
}

/// Control: Swift `#if` directives are siblings of the members they guard,
/// not wrappers, so the guarded members read like unguarded ones.
#[test]
fn swift_conditional_members_keep_default_access_and_fields() {
    let swift = extract_inline(
        "private struct Guarded {\n\
         #if FLAG\n    \
             var guardedField = 1\n    \
             static var guardedStatic = 2\n\
         #else\n    \
             func guardedMethod() {}\n\
         #endif\n\
         }\n",
        "swift",
    );
    let owner = Some("Guarded");
    let kind = Some(K::Type);
    assert_rows(
        &swift,
        &[
            ("guardedField", owner, kind, K::Field, false, false, MODULE_PRIVATE),
            ("guardedStatic", owner, kind, K::Field, true, false, MODULE_PRIVATE),
            ("guardedMethod", owner, kind, K::Method, false, false, MODULE_PRIVATE),
        ],
    );
    assert_fields_listed(
        &swift,
        "Guarded",
        None,
        &[("guardedField", false, MODULE_PRIVATE), ("guardedStatic", true, MODULE_PRIVATE)],
    );
}
