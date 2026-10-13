//! Review finding 3 of `2026-10-03-visibility-and-kind`: a function declared
//! in another function's body stays a Function, even when a query captures
//! it as a method (C# local functions). A member of a type declared inside a
//! function stays a member, and so does a declaration that names its owner
//! (Lua `function M:m()`).
//!
//! Its siblings: a JavaScript/TypeScript `constructor` is a Constructor only
//! in a class body, a member of an unnamed type (anonymous class, class
//! expression, object literal, tagless C/C++ struct) has no container rather
//! than the named type around it, and a member of a type declared inside a
//! function keeps its access.

use tree_hugger::{SymbolKind, SymbolKindV2, Visibility};

use crate::symbols_fixtures::{Extracted, extract_inline};
use crate::symbols_static::assert_static;
use crate::symbols_visibility::assert_level;

use SymbolKind as K;

const PUBLIC: Option<Visibility> = Some(Visibility::Public);

/// `(name, kind)` of a symbol whose name is unique in its source.
type KindRow = (&'static str, SymbolKind);

/// Asserts the kind of the single symbol named `name` on both public paths,
/// whatever its container.
pub fn assert_named_kind(extracted: &Extracted, name: &str, kind: SymbolKind) {
    let symbols: Vec<_> = extracted.symbols.iter().filter(|symbol| symbol.name == name).collect();
    assert_eq!(symbols.len(), 1, "{}: expected one symbol `{name}`", extracted.label);
    assert_eq!(symbols[0].kind, kind, "{}: `{name}` kind in symbols()", extracted.label);

    let records: Vec<_> = extracted
        .records
        .iter()
        .filter(|record| record.identity.name == name)
        .collect();
    assert_eq!(records.len(), 1, "{}: expected one record `{name}`", extracted.label);
    assert_eq!(
        records[0].kind,
        SymbolKindV2::from(kind),
        "{}: `{name}` kind in symbol_index_v2()",
        extracted.label
    );
}

#[test]
fn csharp_local_functions_are_functions_not_methods() {
    let csharp = extract_inline(
        "class Sweep {\n    \
             public void Outer() {\n        \
                 void nestedProbe() {}\n        \
                 static void nestedStaticProbe() {}\n        \
                 int arrowProbe() => 1;\n        \
                 if (true) { void blockProbe() {} }\n        \
                 System.Action act = () => { void lambdaProbe() {} };\n    \
             }\n    \
             public static void StaticOuter() {}\n    \
             public int Gauge { get { int accessorProbe() => 1; return accessorProbe(); } }\n    \
             public Sweep() { void ctorProbe() {} }\n\
         }\n",
        "cs",
    );
    let class = Some("Sweep");
    let owner = Some(K::Class);

    for name in [
        "nestedProbe",
        "nestedStaticProbe",
        "arrowProbe",
        "blockProbe",
        "lambdaProbe",
        "accessorProbe",
        "ctorProbe",
    ] {
        csharp.assert_kind(name, class, K::Function, owner);
        assert_static(&csharp, name, class, false, false);
        assert_level(&csharp, name, class, None);
    }

    // Controls: the enclosing methods and property stay members.
    csharp.assert_kind("Outer", class, K::Method, owner);
    assert_static(&csharp, "Outer", class, false, false);
    assert_level(&csharp, "Outer", class, PUBLIC);
    csharp.assert_kind("StaticOuter", class, K::Method, owner);
    assert_static(&csharp, "StaticOuter", class, true, false);
    csharp.assert_kind("Gauge", class, K::Property, owner);
}

#[test]
fn nested_functions_stay_functions_in_every_language() {
    // (extension, source, nested function, ordinary method or None)
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        (
            "rs",
            "struct Sweep;\nimpl Sweep {\n    fn outer(&self) {\n        fn nested_probe() {}\n    }\n}\n",
            "nested_probe",
            Some("outer"),
        ),
        (
            "ts",
            "class Sweep {\n  outer() {\n    function nestedProbe() {}\n  }\n}\n",
            "nestedProbe",
            Some("outer"),
        ),
        (
            "js",
            "class Sweep {\n  outer() {\n    function nestedProbe() {}\n  }\n}\n",
            "nestedProbe",
            Some("outer"),
        ),
        (
            "py",
            "class Sweep:\n    def outer(self):\n        def nested_probe(): ...\n",
            "nested_probe",
            Some("outer"),
        ),
        (
            "swift",
            "class Sweep {\n    func outer() {\n        func nestedProbe() {}\n    }\n}\n",
            "nestedProbe",
            Some("outer"),
        ),
        (
            "scala",
            "class Sweep {\n  def outer(): Unit = {\n    def nestedProbe(): Unit = ()\n  }\n}\n",
            "nestedProbe",
            Some("outer"),
        ),
        (
            "php",
            "<?php\nclass Sweep {\n    public function outer() {\n        function nestedProbe() {}\n    }\n}\n",
            "nestedProbe",
            Some("outer"),
        ),
        (
            "lua",
            "local M = {}\nfunction M:outer()\n  local function nestedProbe() end\nend\n",
            "nestedProbe",
            Some("outer"),
        ),
        (
            "pl",
            "package Sweep;\nsub outer {\n    sub nested_probe { }\n}\n",
            "nested_probe",
            None,
        ),
        ("sh", "outer() {\n  nested_probe() { :; }\n}\n", "nested_probe", None),
        ("zsh", "outer() {\n  nested_probe() { :; }\n}\n", "nested_probe", None),
    ];

    for &(extension, source, nested, method) in cases {
        let extracted = extract_inline(source, extension);
        assert_named_kind(&extracted, nested, K::Function);
        if let Some(method) = method {
            assert_named_kind(&extracted, method, K::Method);
        }
    }
}

#[test]
fn members_of_types_declared_in_functions_stay_members() {
    let cases: &[(&str, &str, &[KindRow])] = &[
        (
            "rs",
            "struct Sweep;\nimpl Sweep {\n    fn outer(&self) {\n        \
             struct Local { field_probe: i32 }\n        \
             impl Local { fn local_method(&self) {} }\n    }\n}\n\
             trait Shape { fn area(&self) -> f64; }\n",
            &[("field_probe", K::Field), ("local_method", K::Method), ("area", K::Method)],
        ),
        (
            "ts",
            "class Sweep {\n  outer() {\n    const kept = class { keptMethod() {} };\n    \
             const literal = { literalMethod() {} };\n  }\n}\n\
             interface Shape { area(): number; }\n",
            &[("keptMethod", K::Method), ("literalMethod", K::Method), ("area", K::Method)],
        ),
        (
            "py",
            "class Sweep:\n    def outer(self):\n        class Local:\n            \
             def local_method(self): ...\n",
            &[("local_method", K::Method)],
        ),
        (
            "java",
            "class Sweep {\n    void outer() {\n        class Local { void localMethod() {} }\n        \
             Runnable task = new Runnable() { public void run() {} };\n    }\n}\n",
            &[("localMethod", K::Method), ("run", K::Method)],
        ),
        (
            "cs",
            "class Sweep {\n    class Inner { void innerMethod() {} }\n}\n",
            &[("innerMethod", K::Method)],
        ),
        (
            "cpp",
            "class Sweep {\npublic:\n    void outer() { struct Local { int fieldProbe; void localMethod() {} }; }\n};\n",
            &[("fieldProbe", K::Field), ("localMethod", K::Method)],
        ),
        (
            "c",
            "void outer(void) { struct Local { int fieldProbe; }; }\n",
            &[("fieldProbe", K::Field)],
        ),
        (
            "go",
            "package p\ntype Sweep struct{}\nfunc (s Sweep) Outer() {}\n",
            &[("Outer", K::Method)],
        ),
        (
            "swift",
            "class Sweep {}\nextension Sweep { func extra() {} }\n",
            &[("extra", K::Method)],
        ),
        (
            "scala",
            "object Holder { def member(): Unit = () }\n",
            &[("member", K::Method)],
        ),
        // A Lua method names its table, so it is a method wherever it is written.
        (
            "lua",
            "local M = {}\nfunction M:outer()\n  function M:innerMethod() end\nend\n",
            &[("outer", K::Method), ("innerMethod", K::Method)],
        ),
    ];

    for &(extension, source, rows) in cases {
        let extracted = extract_inline(source, extension);
        for &(name, kind) in rows {
            assert_named_kind(&extracted, name, kind);
        }
    }
}

#[test]
fn object_literal_constructor_is_a_method() {
    for extension in ["js", "ts"] {
        let top = extract_inline("const top = { constructor() {}, topMethod() {} };\n", extension);
        top.assert_kind("constructor", None, K::Method, None);
        top.assert_kind("topMethod", None, K::Method, None);

        let nested = extract_inline(
            "class Sweep {\n  outer() {\n    const literal = { constructor() {} };\n  }\n}\n",
            extension,
        );
        nested.assert_kind("constructor", None, K::Method, None);

        // Controls: a class body's `constructor`, also a class expression's.
        let class = extract_inline(
            "class Sweep {\n  constructor() {}\n}\nconst Kept = class {\n  constructor() {}\n};\n",
            extension,
        );
        class.assert_kind("constructor", Some("Sweep"), K::Constructor, Some(K::Class));
        class.assert_kind("constructor", None, K::Constructor, None);
    }
}

#[test]
fn members_of_unnamed_types_have_no_container() {
    let java = extract_inline(
        "class Sweep {\n    \
             Runnable field = new Runnable() { public void fieldRun() {} };\n    \
             void outer() {\n        \
                 Runnable r = new Runnable() { public void run() {} };\n        \
                 class Local { public void localMethod() {} }\n    \
             }\n    \
             enum Mode { FAST { void constantMethod() {} } }\n\
         }\n",
        "java",
    );
    java.assert_kind("fieldRun", None, K::Method, None);
    java.assert_kind("run", None, K::Method, None);
    java.assert_kind("constantMethod", None, K::Method, None);
    // Controls: a named local class and the enclosing method.
    java.assert_kind("localMethod", Some("Local"), K::Method, Some(K::Class));
    java.assert_kind("outer", Some("Sweep"), K::Method, Some(K::Class));

    for extension in ["js", "ts"] {
        let ecma = extract_inline(
            "class Sweep {\n  outer() {\n    \
                 const kept = class { keptMethod() {} };\n    \
                 const named = class NamedExpr { namedMethod() {} };\n    \
                 const literal = { literalMethod() {} };\n  \
             }\n}\n",
            extension,
        );
        ecma.assert_kind("keptMethod", None, K::Method, None);
        ecma.assert_kind("namedMethod", None, K::Method, None);
        ecma.assert_kind("literalMethod", None, K::Method, None);
        ecma.assert_kind("outer", Some("Sweep"), K::Method, Some(K::Class));
    }

    let php = extract_inline(
        "<?php\nclass Sweep {\n    public function outer() {\n        \
             return new class { public function anonMethod() {} };\n    }\n}\n",
        "php",
    );
    php.assert_kind("anonMethod", None, K::Method, None);
    php.assert_kind("outer", Some("Sweep"), K::Method, Some(K::Class));

    // Scala: a `def` in an anonymous class body is a Method too.
    let scala = extract_inline(
        "class Sweep {\n  def outer(): Any = new Runnable { def run(): Unit = () }\n}\n",
        "scala",
    );
    scala.assert_kind("run", None, K::Method, None);
    scala.assert_kind("outer", Some("Sweep"), K::Method, Some(K::Class));

    let cpp = extract_inline(
        "class Holder {\npublic:\n    \
             struct { int anonField; void anonMethod() {} } member;\n    \
             union { int unionA; float unionB; };\n\
         };\n",
        "cpp",
    );
    cpp.assert_kind("anonField", None, K::Field, None);
    cpp.assert_kind("anonMethod", None, K::Method, None);
    // Controls: the named member, and an anonymous union's members, which
    // are members of the enclosing class.
    cpp.assert_kind("member", Some("Holder"), K::Field, Some(K::Class));
    cpp.assert_kind("unionA", Some("Holder"), K::Field, Some(K::Class));

    let c = extract_inline(
        "struct Outer {\n    struct { int inner; } nested;\n    union { int unionA; float unionB; };\n};\n",
        "c",
    );
    c.assert_kind("inner", None, K::Field, None);
    c.assert_kind("nested", Some("Outer"), K::Field, Some(K::Type));
    c.assert_kind("unionA", Some("Outer"), K::Field, Some(K::Type));
}

#[test]
fn members_of_types_declared_in_functions_keep_their_access() {
    let java = extract_inline(
        "class Sweep {\n    void outer() {\n        \
             Runnable r = new Runnable() { public void run() { int counter = 0; } };\n        \
             class Local { public void localMethod() {} }\n    \
         }\n}\n",
        "java",
    );
    assert_level(&java, "run", None, PUBLIC);
    assert_level(&java, "localMethod", Some("Local"), PUBLIC);
    // Control: a local of that method is still a local.
    assert_level(&java, "counter", None, None);

    let php = extract_inline(
        "<?php\nclass Sweep {\n    public function outer() {\n        \
             return new class { public function anonMethod() {} };\n    }\n}\n",
        "php",
    );
    assert_level(&php, "anonMethod", None, PUBLIC);

    let python = extract_inline(
        "class Sweep:\n    def outer(self):\n        class Local:\n            \
             def local_method(self): ...\n",
        "py",
    );
    python.assert_kind("local_method", Some("Local"), K::Method, Some(K::Class));
    assert_static(&python, "local_method", Some("Local"), false, false);
}

/// A Scala `def` in an `enum` body is a Method of that enum; a `def` in a
/// `given … with` instance is a Method of an unnamed owner; a `def` in an
/// `extension` block belongs to the type around the block, and is a Function
/// at top level. A `package object` is not a container, so its `def` stays a
/// top-level Function.
#[test]
fn scala_enum_given_and_extension_members_belong_to_their_owner() {
    let scala = extract_inline(
        "package object pkg { def packageProbe(): Int = 1 }\n\
         object Host:\n  \
           enum Color { case Red, Green; def enumProbe(): Int = 1 }\n  \
           enum Planet(mass: Double) {\n    \
             case Earth extends Planet(1.0)\n    \
             def planetProbe(): Double = mass\n  \
           }\n  \
           given ordering: Ordering[Int] with\n    \
             def givenProbe(a: Int, b: Int): Int = 0\n  \
           extension (x: Int) def objectExtension: Int = x * 2\n  \
           case class Point(x: Int) { def pointProbe(): Int = x; extension (y: Int) def classExtension: Int = y }\n  \
           def objectProbe(): Int = 1\n\
         enum Braceless:\n  \
           case Only\n  \
           def bracelessProbe(): Int = 1\n\
         extension (z: Int) def topExtension: Int = z\n",
        "scala",
    );

    scala.assert_kind("enumProbe", Some("Color"), K::Method, Some(K::Enum));
    assert_static(&scala, "enumProbe", Some("Color"), false, false);
    scala.assert_kind("planetProbe", Some("Planet"), K::Method, Some(K::Enum));
    scala.assert_kind("bracelessProbe", Some("Braceless"), K::Method, Some(K::Enum));
    scala.assert_kind("Color", Some("Host"), K::Enum, Some(K::Module));
    // Enum cases are variants of their own enum, never members.
    for (case, owner) in [("Red", "Color"), ("Green", "Color"), ("Earth", "Planet"), ("Only", "Braceless")] {
        scala.assert_kind(case, Some(owner), K::EnumVariant, Some(K::Enum));
    }

    scala.assert_kind("givenProbe", None, K::Method, None);
    assert_static(&scala, "givenProbe", None, false, false);

    scala.assert_kind("objectExtension", Some("Host"), K::Method, Some(K::Module));
    assert_static(&scala, "objectExtension", Some("Host"), true, false);
    scala.assert_kind("classExtension", Some("Point"), K::Method, Some(K::Class));
    assert_static(&scala, "classExtension", Some("Point"), false, false);
    assert_named_kind(&scala, "topExtension", K::Function);

    assert_named_kind(&scala, "packageProbe", K::Function);
    assert_static(&scala, "packageProbe", None, false, false);

    // Controls.
    scala.assert_kind("pointProbe", Some("Point"), K::Method, Some(K::Class));
    scala.assert_kind("objectProbe", Some("Host"), K::Method, Some(K::Module));
    assert_static(&scala, "objectProbe", Some("Host"), true, false);
}

/// A declaration in a block-valued initializer, accessor or initializer block
/// is local: a Function (or Variable, or local type) with Unknown access and
/// never static, whose container stays the enclosing type like any other
/// function-local declaration. The member that holds the block keeps its own
/// access and `static`, and a member of a type declared in the block keeps
/// its access.
#[test]
fn declarations_in_block_valued_initializers_are_locals() {
    // (extension, source, container, local functions, local non-functions,
    // control members with their access)
    type Case = (
        &'static str,
        &'static str,
        Option<&'static str>,
        &'static [&'static str],
        &'static [&'static str],
        &'static [(&'static str, Option<Visibility>)],
    );
    let cases: &[Case] = &[
        (
            "scala",
            "object Host:\n  \
               given alias: Int = { def givenHelper(): Int = 1; givenHelper() }\n  \
               val held: Int = { def valHelper(): Int = 1; val valLocal = 2; valHelper() }\n  \
               lazy val lazyHeld: Int = { def lazyHelper(): Int = 1; lazyHelper() }\n  \
               var varHeld: Int = { def varHelper(): Int = 1; varHelper() }\n  \
               val indented: Int =\n    \
                 def indentedHelper(): Int = 1\n    \
                 indentedHelper()\n  \
               val matched = 1 match { case _ => { def caseHelper() = 1; caseHelper() } }\n  \
               val withClass = { class BlockClass { def blockMember(): Int = 1 }; 1 }\n",
            Some("Host"),
            &[
                "givenHelper",
                "valHelper",
                "lazyHelper",
                "varHelper",
                "indentedHelper",
                "caseHelper",
            ],
            &["valLocal", "BlockClass"],
            &[("held", PUBLIC), ("lazyHeld", PUBLIC), ("varHeld", PUBLIC)],
        ),
        (
            "swift",
            "class Sweep {\n  \
               var computed: Int {\n    \
                 func computedHelper() -> Int { 1 }\n    \
                 return computedHelper()\n  \
               }\n  \
               var accessors: Int {\n    \
                 get { func getHelper() -> Int { 1 }; return getHelper() }\n    \
                 set { func setHelper() {} }\n  \
               }\n  \
               var observed: Int = 0 { didSet { func didSetHelper() {} } }\n\
             }\n",
            Some("Sweep"),
            &["computedHelper", "getHelper", "setHelper", "didSetHelper"],
            &[],
            &[
                ("computed", Some(Visibility::Internal)),
                ("observed", Some(Visibility::Internal)),
            ],
        ),
        (
            "java",
            "class Sweep {\n  \
               static { int staticInitLocal = 1; class InitClass { public void initMember() {} } }\n  \
               { int instanceInitLocal = 1; }\n  \
               public int kept = 1;\n\
             }\n",
            Some("Sweep"),
            &[],
            &["staticInitLocal", "instanceInitLocal", "InitClass"],
            &[("kept", PUBLIC)],
        ),
        (
            "rs",
            "pub const HELD: i32 = { fn const_helper() -> i32 { 1 } const_helper() };\n\
             pub static STORED: i32 = { fn static_helper() -> i32 { 1 } static_helper() };\n",
            None,
            &["const_helper", "static_helper"],
            &[],
            &[("HELD", PUBLIC), ("STORED", PUBLIC)],
        ),
    ];

    for (extension, source, container, functions, others, controls) in cases {
        let extracted = extract_inline(source, extension);
        let container = *container;
        for name in *functions {
            assert_named_kind(&extracted, name, K::Function);
            assert_static(&extracted, name, container, false, false);
            assert_level(&extracted, name, container, None);
        }
        for name in *others {
            assert_static(&extracted, name, container, false, false);
            assert_level(&extracted, name, container, None);
        }
        for (name, level) in *controls {
            assert_level(&extracted, name, container, *level);
        }
    }

    // Members of a type declared in the block keep their access.
    let scala = extract_inline(
        "object Host:\n  val withClass = { class BlockClass { def blockMember(): Int = 1 }; 1 }\n",
        "scala",
    );
    scala.assert_kind("blockMember", Some("BlockClass"), K::Method, Some(K::Class));
    assert_level(&scala, "blockMember", Some("BlockClass"), PUBLIC);
    // The object member that holds a block stays a static member.
    let held = extract_inline("object Host:\n  val held: Int = { def helper(): Int = 1; helper() }\n", "scala");
    assert_static(&held, "held", Some("Host"), true, false);
}
