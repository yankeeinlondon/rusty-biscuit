//! Test 7: `hug --plain` / `--json` output renders kinds, visibility labels,
//! containers and the `classes` static/instance partition from the library's
//! records. Exhaustive language/value combinations stay in the library tests.

use serde_json::Value;

use crate::{
    BASH_FIXTURE, CPP_FIXTURE, CSHARP_FIXTURE, JAVA_FIXTURE, RUST_FIXTURE, Scratch,
    TYPESCRIPT_FIXTURE, json_of, stdout_of,
};

/// `(language, declaration, field under test, expected line without its location)`.
type LineRow = (&'static str, &'static str, &'static str, &'static str);

/// The text of each `  - …` symbol line with the trailing location removed.
fn symbol_lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("  - "))
        .map(|line| match line.rsplit_once(' ') {
            Some((text, _location)) => text.to_string(),
            None => line.to_string(),
        })
        .collect()
}

fn assert_lines(stdout: &str, rows: &[LineRow]) {
    let lines = symbol_lines(stdout);
    for (language, declaration, field, expected) in rows {
        if lines.iter().any(|line| line == expected) {
            continue;
        }
        let candidates: Vec<_> = lines
            .iter()
            .filter(|line| line.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '#')
                .any(|word| word == *declaration))
            .collect();
        panic!(
            "{language} `{declaration}`: {field} differs\n  expected: {expected}\n  rendered: {candidates:#?}"
        );
    }
}

#[test]
fn plain_symbols_print_known_visibility_for_every_kind() {
    let scratch = Scratch::new(&[
        ("typescript.ts", TYPESCRIPT_FIXTURE),
        ("rust.rs", RUST_FIXTURE),
        ("java.java", JAVA_FIXTURE),
        ("c_sharp.cs", CSHARP_FIXTURE),
    ]);

    let typescript = scratch.stdout(&["--plain", "symbols", "typescript.ts"]);
    assert_lines(
        &typescript,
        &[
            ("TypeScript", "exportedFunction", "visibility (function)", "public function exportedFunction(): void"),
            ("TypeScript", "moduleFunction", "visibility (module-private)", "module-private function moduleFunction(parameterValue: number): number"),
            ("TypeScript", "ExportedAbstractBase", "visibility (class)", "public class ExportedAbstractBase"),
            ("TypeScript", "ModuleInterface", "visibility (interface)", "module-private interface ModuleInterface"),
            ("TypeScript", "ExportedAlias", "kind (type alias)", "public type alias ExportedAlias"),
            ("TypeScript", "MODULE_CONSTANT", "visibility (const)", "module-private const MODULE_CONSTANT"),
            ("TypeScript", "constructor", "kind and container (constructor)", "public constructor constructor(initialValue: number) [in class ExportedWidget]"),
            ("TypeScript", "publicMethod", "visibility and container (method)", "public method publicMethod(): number [in class ExportedWidget]"),
            ("TypeScript", "protectedMethod", "visibility (protected method)", "protected method protectedMethod(): void [in class ExportedWidget]"),
            ("TypeScript", "privateStaticMethod", "visibility (private static method)", "private method privateStaticMethod(): void [in class ExportedWidget]"),
            ("TypeScript", "unmarkedMethod", "parameter list (parameterless method)", "public method unmarkedMethod() [in class ExportedWidget]"),
            ("TypeScript", "#hashPrivateProperty", "visibility and container (property)", "private property #hashPrivateProperty [in class ExportedWidget]"),
            ("TypeScript", "interfaceMethod", "container (interface member)", "public method interfaceMethod(): void [in interface ExportedInterface]"),
        ],
    );

    let rust = scratch.stdout(&["--plain", "symbols", "rust.rs"]);
    assert_lines(
        &rust,
        &[
            ("Rust", "crate_function", "visibility (crate)", "crate function crate_function()"),
            ("Rust", "PublicAlias", "kind (type alias)", "public type alias PublicAlias"),
            ("Rust", "UnitVariant", "kind and container (enum variant)", "public enum variant UnitVariant [in enum PublicEnum]"),
            ("Rust", "crate_associated_function", "container (inherent impl member)", "crate method crate_associated_function() [in type PublicStruct]"),
            ("Rust", "INHERENT_ASSOCIATED_CONSTANT", "container (associated const)", "public const INHERENT_ASSOCIATED_CONSTANT [in type PublicStruct]"),
        ],
    );
    assert!(
        !rust.contains("[in impl"),
        "Rust impl members must name the implemented type, not the impl header:\n{rust}"
    );

    let java = scratch.stdout(&["--plain", "symbols", "java.java"]);
    assert_lines(
        &java,
        &[(
            "Java",
            "packageMethod",
            "visibility (package)",
            "package method packageMethod() -> void [in class PublicWidget]",
        )],
    );

    let csharp = scratch.stdout(&["--plain", "symbols", "c_sharp.cs"]);
    assert_lines(
        &csharp,
        &[
            ("C#", "ProtectedInternalMethod", "visibility (protected internal)", "protected internal method ProtectedInternalMethod() -> void [in class PublicWidget]"),
            ("C#", "PrivateProtectedField", "visibility (private protected)", "private protected field PrivateProtectedField [in class PublicWidget]"),
        ],
    );
}

#[test]
fn plain_symbols_omit_the_visibility_prefix_for_unknown_access() {
    let scratch = Scratch::new(&[("bash.sh", BASH_FIXTURE), ("typescript.ts", TYPESCRIPT_FIXTURE)]);

    let bash = scratch.stdout(&["--plain", "symbols", "bash.sh"]);
    assert_lines(
        &bash,
        &[
            ("Bash", "keyword_function", "visibility (Unknown)", "function keyword_function()"),
            ("Bash", "GLOBAL_VARIABLE", "visibility (Unknown)", "variable GLOBAL_VARIABLE"),
        ],
    );
    let labels = ["public ", "private ", "protected ", "internal ", "package ", "crate ", "module-private "];
    for line in symbol_lines(&bash) {
        assert!(
            !labels.iter().any(|label| line.starts_with(label)),
            "Bash: every access is Unknown, but `{line}` carries a visibility prefix"
        );
    }

    // `localValue` is a local, which `symbols` lists only with `--all-scopes`.
    let typescript = scratch.stdout(&["--plain", "symbols", "--all-scopes", "typescript.ts"]);
    assert_lines(
        &typescript,
        &[("TypeScript", "localValue", "visibility (Unknown local)", "const localValue")],
    );
}

#[test]
fn plain_functions_include_constructors_and_types_include_aliases() {
    let scratch = Scratch::new(&[("typescript.ts", TYPESCRIPT_FIXTURE)]);

    let functions = scratch.stdout(&["--plain", "functions", "typescript.ts"]);
    assert_lines(
        &functions,
        &[
            ("TypeScript", "constructor", "`functions` membership", "public constructor constructor(initialValue: number) [in class ExportedWidget]"),
            ("TypeScript", "constructor", "`functions` membership", "public constructor constructor() [in class ModuleWidget]"),
        ],
    );

    let types = scratch.stdout(&["--plain", "types", "typescript.ts"]);
    assert_lines(
        &types,
        &[
            ("TypeScript", "ExportedAlias", "`types` membership", "public type alias ExportedAlias"),
            ("TypeScript", "ModuleAlias", "`types` membership", "module-private type alias ModuleAlias"),
        ],
    );
}

/// Removes SGR color sequences and OSC 8 hyperlink wrappers.
fn strip_styling(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.next() {
            // CSI: parameters up to a final byte in `@`..=`~`.
            Some('[') => {
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            // OSC: up to ST (`ESC \`) or BEL.
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[test]
fn pretty_symbols_use_the_same_labels_as_plain() {
    let scratch = Scratch::new(&[("typescript.ts", TYPESCRIPT_FIXTURE), ("rust.rs", RUST_FIXTURE)]);

    for file in ["typescript.ts", "rust.rs"] {
        let plain = scratch.stdout(&["--plain", "symbols", file]);
        let pretty_args = ["symbols", file];
        // Forced color counts as interactive output, which wraps; a wide
        // `COLUMNS` keeps every line whole so the text compares with plain.
        let pretty = stdout_of(
            scratch.hug_with_env(&pretty_args, &[("CLICOLOR_FORCE", "1"), ("COLUMNS", "10000")]),
            &pretty_args,
        );
        assert!(
            pretty.contains('\u{1b}'),
            "{file}: CLICOLOR_FORCE must produce styled output"
        );
        assert_eq!(
            strip_styling(&pretty),
            plain,
            "{file}: pretty output must carry the same visibility, kind and container text as plain"
        );
    }
}

/// `(class, section, member line)` for every member line of `hug --plain classes`.
fn class_members(stdout: &str) -> Vec<(String, String, String)> {
    let mut class = String::new();
    let mut section = String::new();
    let mut members = Vec::new();
    for line in stdout.lines() {
        if let Some(member) = line.strip_prefix("      - ") {
            members.push((class.clone(), section.clone(), member.to_string()));
        } else if let Some(title) = line.strip_prefix("    ") {
            section = title.rsplit_once(" (").map_or(title, |(name, _)| name).to_string();
        } else if let Some(header) = line.strip_prefix("  ") {
            // `  {kind} {Name} [line:col]`
            class = header.split_whitespace().nth(1).unwrap_or_default().to_string();
            section.clear();
        }
    }
    members
}

/// The member's name: the text after any visibility label, up to `(` or `:`.
fn member_name(line: &str) -> &str {
    let end = line.find(['(', ':']).unwrap_or(line.len());
    line[..end].rsplit(' ').next().unwrap_or_default()
}

fn sections_of(members: &[(String, String, String)], class: &str, name: &str) -> Vec<String> {
    members
        .iter()
        .filter(|(owner, _, line)| owner == class && member_name(line) == name)
        .map(|(_, section, _)| section.clone())
        .collect()
}

fn assert_section(
    members: &[(String, String, String)],
    language: &str,
    class: &str,
    name: &str,
    section: &str,
) {
    let found = sections_of(members, class, name);
    assert_eq!(
        found,
        vec![section.to_string()],
        "{language} `{class}.{name}`: `hug classes` section differs (members: {members:#?})"
    );
}

#[test]
fn plain_classes_partition_typescript_static_members() {
    let scratch = Scratch::new(&[("typescript.ts", TYPESCRIPT_FIXTURE)]);
    let members = class_members(&scratch.stdout(&["--plain", "classes", "typescript.ts"]));

    assert_section(&members, "TypeScript", "ExportedWidget", "staticMethod", "Static Methods");
    assert_section(&members, "TypeScript", "ExportedWidget", "privateStaticMethod", "Static Methods");
    assert_section(&members, "TypeScript", "ExportedWidget", "publicMethod", "Instance Methods");
    assert_section(&members, "TypeScript", "ExportedWidget", "constructor", "Instance Methods");
    assert_section(&members, "TypeScript", "ExportedWidget", "staticProperty", "Static Fields");
    assert_section(&members, "TypeScript", "ExportedWidget", "privateStaticProperty", "Static Fields");
    assert_section(&members, "TypeScript", "ExportedWidget", "publicProperty", "Instance Fields");
}

#[test]
fn plain_classes_list_cpp_static_members_but_not_function_local_statics() {
    let scratch = Scratch::new(&[("cpp.cpp", CPP_FIXTURE)]);
    let members = class_members(&scratch.stdout(&["--plain", "classes", "cpp.cpp"]));

    assert_section(&members, "C++", "AccessWidget", "staticMemberField", "Static Fields");
    assert_section(&members, "C++", "AccessWidget", "staticFactory", "Static Methods");
    assert_section(&members, "C++", "AccessWidget", "countCalls", "Instance Methods");
    assert!(
        !members.iter().any(|(_, _, line)| member_name(line) == "memberLocalStatic"),
        "C++ `AccessWidget.countCalls`: the function-local `static memberLocalStatic` must not be a class member: {members:#?}"
    );
}

/// Members behind a `template<…>` header or a preprocessor conditional are
/// listed in their class like unwrapped ones; a template function outside the
/// class is not a member.
#[test]
fn plain_classes_list_cpp_wrapped_members_in_their_sections() {
    let source = "class TemplateSweep {\n\
                  public:\n    \
                      template<class T> static void staticProbe(T x) {}\n    \
                      template<class T> void instanceProbe(T x) {}\n    \
                      template<class T> static T fieldProbe;\n\
                  #ifdef FEATURE\n    \
                      static void guardedStatic() {}\n\
                  #endif\n\
                  };\n\
                  template<class T> void freeTemplate(T x) {}\n";
    let scratch = Scratch::new(&[("sweep.cpp", source)]);
    let members = class_members(&scratch.stdout(&["--plain", "classes", "sweep.cpp"]));

    assert_section(&members, "C++", "TemplateSweep", "staticProbe", "Static Methods");
    assert_section(&members, "C++", "TemplateSweep", "instanceProbe", "Instance Methods");
    assert_section(&members, "C++", "TemplateSweep", "fieldProbe", "Static Fields");
    assert_section(&members, "C++", "TemplateSweep", "guardedStatic", "Static Methods");
    assert!(
        !members.iter().any(|(_, _, line)| member_name(line) == "freeTemplate"),
        "C++ `freeTemplate` is not a class member: {members:#?}"
    );
}

/// C# local functions are Functions, not members, so `hug classes` lists
/// only the methods that contain them.
#[test]
fn plain_classes_omit_csharp_local_functions() {
    let source = "class Sweep {\n    \
                      public void Outer() {\n        \
                          void nestedProbe() {}\n        \
                          static void nestedStaticProbe() {}\n    \
                      }\n    \
                      public static void StaticOuter() {}\n\
                  }\n";
    let scratch = Scratch::new(&[("sweep.cs", source)]);
    let members = class_members(&scratch.stdout(&["--plain", "classes", "sweep.cs"]));

    assert_section(&members, "C#", "Sweep", "Outer", "Instance Methods");
    assert_section(&members, "C#", "Sweep", "StaticOuter", "Static Methods");
    for local in ["nestedProbe", "nestedStaticProbe"] {
        assert!(
            !members.iter().any(|(_, _, line)| member_name(line) == local),
            "C# local function `{local}` is not a class member: {members:#?}"
        );
    }
}

/// C# fields and interface properties behind `#if`/`#else` are listed in
/// their type like unwrapped ones, with the type's default access.
#[test]
fn plain_classes_list_csharp_conditional_fields() {
    let source = "class Guarded {\n\
                  #if FLAG\n    \
                      static int guardedStatic;\n    \
                      void guardedMethod() {}\n\
                  #else\n    \
                      int guardedField;\n\
                  #endif\n\
                  }\n\
                  interface IGuarded {\n\
                  #if FLAG\n    \
                      int Count { get; }\n\
                  #endif\n\
                  }\n";
    let scratch = Scratch::new(&[("sweep.cs", source)]);
    let members = class_members(&scratch.stdout(&["--plain", "classes", "sweep.cs"]));

    assert_section(&members, "C#", "Guarded", "guardedStatic", "Static Fields");
    assert_section(&members, "C#", "Guarded", "guardedField", "Instance Fields");
    assert_section(&members, "C#", "Guarded", "guardedMethod", "Instance Methods");
    assert_section(&members, "C#", "IGuarded", "Count", "Instance Fields");
    let field_line = members
        .iter()
        .find(|(owner, _, line)| owner == "Guarded" && member_name(line) == "guardedField")
        .map(|(_, _, line)| line.as_str())
        .unwrap_or_default();
    assert!(
        field_line.starts_with("private "),
        "C# `Guarded.guardedField` should be listed as private: {field_line:?}"
    );
}

/// Members of an anonymous class, a class expression or an object literal
/// have no container, so `hug classes` does not list them under the named
/// class that encloses them; an object literal's `constructor` is not the
/// class's constructor.
#[test]
fn plain_classes_omit_members_of_unnamed_types() {
    let java = "class Sweep {\n    \
                    Runnable field = new Runnable() { public void fieldRun() {} };\n    \
                    void outer() { Runnable r = new Runnable() { public void run() {} }; }\n\
                }\n";
    let typescript = "class Sweep {\n  \
                          outer() {\n    \
                              const kept = class { keptMethod() {} };\n    \
                              const literal = { literalMethod() {}, constructor() {} };\n  \
                          }\n\
                      }\n";
    let scratch = Scratch::new(&[("sweep.java", java), ("sweep.ts", typescript)]);

    let java_members = class_members(&scratch.stdout(&["--plain", "classes", "sweep.java"]));
    assert_section(&java_members, "Java", "Sweep", "outer", "Instance Methods");
    for name in ["fieldRun", "run"] {
        assert!(
            sections_of(&java_members, "Sweep", name).is_empty(),
            "Java anonymous-class method `{name}` is not a member of `Sweep`: {java_members:#?}"
        );
    }

    let ts_members = class_members(&scratch.stdout(&["--plain", "classes", "sweep.ts"]));
    assert_section(&ts_members, "TypeScript", "Sweep", "outer", "Instance Methods");
    for name in ["keptMethod", "literalMethod", "constructor"] {
        assert!(
            sections_of(&ts_members, "Sweep", name).is_empty(),
            "TypeScript `{name}` is not a member of `Sweep`: {ts_members:#?}"
        );
    }
}

/// An earlier C++ `static` prototype reaches a later definition that names
/// its callback's parameters or spells the namespace as `a::b`, and `hug`
/// prints both declarations as module-private.
#[test]
fn plain_symbols_print_cpp_linkage_across_parameter_and_namespace_spelling() {
    let source = "static int callback(void (*)(int));\n\
                  int callback(void (*cb)(int value)) { return 0; }\n\
                  namespace a { namespace b { static int helper(int); } }\n\
                  namespace a::b { int helper(int value) { return value; } }\n";
    let scratch = Scratch::new(&[("linkage.cpp", source)]);
    let lines = symbol_lines(&scratch.stdout(&["--plain", "symbols", "linkage.cpp"]));

    for name in ["callback", "helper"] {
        let declarations: Vec<_> = lines
            .iter()
            .filter(|line| line.contains(&format!(" {name}(")))
            .collect();
        assert_eq!(declarations.len(), 2, "C++ `{name}`: two declarations expected: {lines:#?}");
        assert!(
            declarations.iter().all(|line| line.starts_with("module-private function ")),
            "C++ `{name}`: both declarations must be module-private: {declarations:#?}"
        );
    }
}

/// The earlier-`static` scan attaches only to the same function: a function
/// in an unrelated namespace stays public (no false private), and a
/// definition returning a function pointer still finds its `static`
/// prototype when its own parameter gains a name (no false public).
#[test]
fn plain_symbols_print_cpp_linkage_for_namespaces_and_return_declarators() {
    let source = "static int scope_leak(int);\n\
                  namespace unrelated { int scope_leak(int x) { return x; } }\n\
                  static int (*pointer_result(int))(int);\n\
                  int (*pointer_result(int value))(int) { return 0; }\n";
    let scratch = Scratch::new(&[("linkage.cpp", source)]);
    let lines = symbol_lines(&scratch.stdout(&["--plain", "symbols", "linkage.cpp"]));

    for (name, expected) in [
        ("scope_leak", ["module-private function ", "public function "]),
        ("pointer_result", ["module-private function ", "module-private function "]),
    ] {
        let declarations: Vec<_> = lines
            .iter()
            .filter(|line| line.contains(&format!(" {name}(")) || line.contains(&format!("*{name}(")))
            .collect();
        assert_eq!(declarations.len(), 2, "C++ `{name}`: two declarations expected: {lines:#?}");
        for (line, prefix) in declarations.iter().zip(expected) {
            assert!(
                line.starts_with(prefix),
                "C++ `{name}`: expected `{prefix}…`, rendered {line:?}"
            );
        }
    }
}

/// A function template's trailing return type and constraints are part of
/// its identity: a later template that differs in either stays public (no
/// false private), and an identical one is module-private (no false public).
#[test]
fn plain_symbols_print_cpp_linkage_for_template_return_and_constraints() {
    let source = "template<class T> concept Small = sizeof(T) < 4;\n\
                  template<class T> concept Big = sizeof(T) >= 4;\n\
                  template<class T> static auto trailing_differs(T) -> int;\n\
                  template<class T> auto trailing_differs(T value) -> long { return 0; }\n\
                  template<class T> requires Small<T> static int constraint_differs(T);\n\
                  template<class T> requires Big<T> int constraint_differs(T value) { return 0; }\n\
                  template<class T> static auto trailing_same(T) -> int requires Small<T>;\n\
                  template<class T> auto trailing_same(T value) -> int requires Small<T> { return 0; }\n";
    let scratch = Scratch::new(&[("templates.cpp", source)]);
    let lines = symbol_lines(&scratch.stdout(&["--plain", "symbols", "templates.cpp"]));

    for (name, expected) in [
        ("trailing_differs", ["module-private function ", "public function "]),
        ("constraint_differs", ["module-private function ", "public function "]),
        ("trailing_same", ["module-private function ", "module-private function "]),
    ] {
        let declarations: Vec<_> = lines
            .iter()
            .filter(|line| line.contains(&format!(" {name}(")))
            .collect();
        assert_eq!(declarations.len(), 2, "C++ `{name}`: two declarations expected: {lines:#?}");
        for (line, prefix) in declarations.iter().zip(expected) {
            assert!(
                line.starts_with(prefix),
                "C++ `{name}`: expected `{prefix}…`, rendered {line:?}"
            );
        }
    }
}

/// A C++ namespace-scope object is module-private only when the object
/// itself is `const`: a mutable pointer to a const pointer stays public (no
/// false private), and a const pointer to a pointer is module-private (no
/// false public).
#[test]
fn plain_symbols_print_cpp_const_object_linkage_from_the_outermost_declarator() {
    let source = "char *const *mutable_to_const;\n\
                  char **const const_double = 0;\n\
                  int (*const const_function_pointer)(int) = 0;\n";
    let scratch = Scratch::new(&[("objects.cpp", source)]);
    let lines = symbol_lines(&scratch.stdout(&["--plain", "symbols", "objects.cpp"]));

    for (name, prefix) in [
        ("mutable_to_const", "public variable "),
        ("const_double", "module-private variable "),
        ("const_function_pointer", "module-private variable "),
    ] {
        let declarations: Vec<_> = lines
            .iter()
            .filter(|line| line.split(|c: char| !c.is_alphanumeric() && c != '_').any(|word| word == name))
            .collect();
        assert_eq!(declarations.len(), 1, "C++ `{name}`: one declaration expected: {lines:#?}");
        assert!(
            declarations[0].starts_with(prefix),
            "C++ `{name}`: expected `{prefix}…`, rendered {:?}",
            declarations[0]
        );
    }
}

/// A Python decorator whose name merely contains `staticmethod` or
/// `classmethod` leaves the method an instance method, and a Scala trait
/// nested in an object owns its methods as instance methods.
#[test]
fn plain_classes_partition_python_decoys_and_scala_nested_traits() {
    let python = "class Sweep:\n    \
                      @notstaticmethod\n    \
                      def decoratorProbe(self): pass\n    \
                      @notclassmethod\n    \
                      def decoratorClassProbe(self): pass\n    \
                      @staticmethod\n    \
                      def realStatic(): pass\n    \
                      @classmethod\n    \
                      def realClass(cls): pass\n    \
                      def instanceProbe(self): pass\n";
    let scala = "object SweepObject {\n  \
                     trait InnerTrait { def traitProbe(): Unit }\n  \
                     class InnerClass { def instanceProbe(): Unit = () }\n\
                 }\n";
    let scratch = Scratch::new(&[("sweep.py", python), ("sweep.scala", scala)]);

    let python_members = class_members(&scratch.stdout(&["--plain", "classes", "sweep.py"]));
    for name in ["decoratorProbe", "decoratorClassProbe", "instanceProbe"] {
        assert_section(&python_members, "Python", "Sweep", name, "Instance Methods");
    }
    for name in ["realStatic", "realClass"] {
        assert_section(&python_members, "Python", "Sweep", name, "Static Methods");
    }

    let scala_members = class_members(&scratch.stdout(&["--plain", "classes", "sweep.scala"]));
    assert_section(&scala_members, "Scala", "InnerTrait", "traitProbe", "Instance Methods");
    assert_section(&scala_members, "Scala", "InnerClass", "instanceProbe", "Instance Methods");
}

/// Every member form a type declares reaches `hug classes`, in the section
/// its member `static` selects: a TypeScript `static #private` property, a C#
/// static property (public despite its `private set`), each target of a
/// Python chained assignment, and C++ array, pointer and function-pointer
/// fields beside a static array variable template.
#[test]
fn plain_classes_list_every_declared_field_form() {
    let python = "class AssignmentFieldForms:\n    \
                      first = second = 0\n    \
                      a, b = (1, 2)\n";
    let scratch = Scratch::new(&[
        ("typescript.ts", TYPESCRIPT_FIXTURE),
        ("c_sharp.cs", CSHARP_FIXTURE),
        ("cpp.cpp", CPP_FIXTURE),
        ("fields.py", python),
    ]);

    let ts = class_members(&scratch.stdout(&["--plain", "classes", "typescript.ts"]));
    assert_section(&ts, "TypeScript", "PrivateFieldForms", "#count", "Static Fields");
    assert_section(&ts, "TypeScript", "AbstractFieldForms", "count", "Static Fields");

    let csharp = class_members(&scratch.stdout(&["--plain", "classes", "c_sharp.cs"]));
    assert_section(&csharp, "C#", "PropertyFieldForms", "Count", "Static Fields");
    assert_section(&csharp, "C#", "RecordFieldForms", "value", "Instance Fields");
    let count_line = csharp
        .iter()
        .find(|(owner, _, line)| owner == "PropertyFieldForms" && member_name(line) == "Count")
        .map(|(_, _, line)| line.as_str())
        .unwrap_or_default();
    assert!(
        count_line.starts_with("public "),
        "C# `PropertyFieldForms.Count` should be listed as public: {count_line:?}"
    );

    let python_members = class_members(&scratch.stdout(&["--plain", "classes", "fields.py"]));
    for name in ["first", "second", "a", "b"] {
        assert_section(&python_members, "Python", "AssignmentFieldForms", name, "Static Fields");
    }

    let cpp = class_members(&scratch.stdout(&["--plain", "classes", "cpp.cpp"]));
    for name in ["values", "pointer", "callback"] {
        assert_section(&cpp, "C++", "DeclaratorFieldForms", name, "Instance Fields");
    }
    assert_section(&cpp, "C++", "DeclaratorFieldForms", "cache", "Static Fields");
}

/// Names behind an attribute or a member function's return declarators keep
/// their members: `hug classes` lists all four attributed fields in C and
/// C++, and `hug symbols` prints a callback-returning member function as a
/// method of its type, next to a function-pointer field.
#[test]
fn plain_classes_and_symbols_list_names_behind_declarator_wrappers() {
    let owner = "struct AttributeOwner {\n    \
                     int scalar [[maybe_unused]];\n    \
                     int *pointer [[maybe_unused]];\n    \
                     int values [[maybe_unused]] [2];\n    \
                     int (*callback [[maybe_unused]])(int);\n\
                 };\n";
    let methods = "struct MethodOwner {\n    \
                       int (*returns_callback(int value))(long);\n    \
                       int (*callback_field)(int);\n\
                   };\n";
    let scratch = Scratch::new(&[("owner.c", owner), ("owner.cpp", owner), ("methods.cpp", methods)]);

    for (language, file) in [("C", "owner.c"), ("C++", "owner.cpp")] {
        let members = class_members(&scratch.stdout(&["--plain", "classes", file]));
        for name in ["scalar", "pointer", "values", "callback"] {
            assert_section(&members, language, "AttributeOwner", name, "Instance Fields");
        }
    }

    assert_lines(
        &scratch.stdout(&["--plain", "symbols", "methods.cpp"]),
        &[
            (
                "C++",
                "returns_callback",
                "kind and owner",
                "public method returns_callback(value: int) -> int [in type MethodOwner]",
            ),
            ("C++", "callback_field", "kind and owner", "public field callback_field [in type MethodOwner]"),
        ],
    );
}

/// Constants the language makes static without a `static` keyword (a C#
/// `const`, a Rust trait's associated `const`, a PHP class constant) are
/// listed under Static Fields.
#[test]
fn plain_classes_list_implicitly_static_constants_as_static_fields() {
    let php = "<?php\nclass Limits { const MAX = 1; public $count; }\n";
    let scratch = Scratch::new(&[
        ("c_sharp.cs", CSHARP_FIXTURE),
        ("rust.rs", RUST_FIXTURE),
        ("limits.php", php),
    ]);

    let csharp = class_members(&scratch.stdout(&["--plain", "classes", "c_sharp.cs"]));
    assert_section(&csharp, "C#", "ConstantForms", "ClassConstant", "Static Fields");
    assert_section(&csharp, "C#", "IConstantForms", "InterfaceConstant", "Static Fields");

    let rust = class_members(&scratch.stdout(&["--plain", "classes", "rust.rs"]));
    assert_section(&rust, "Rust", "PublicTrait", "TRAIT_ASSOCIATED_CONSTANT", "Static Fields");

    let php_members = class_members(&scratch.stdout(&["--plain", "classes", "limits.php"]));
    assert_section(&php_members, "PHP", "Limits", "MAX", "Static Fields");
    assert_section(&php_members, "PHP", "Limits", "count", "Instance Fields");
}

#[test]
fn plain_classes_list_a_java_interface_constant_as_a_static_field() {
    let scratch = Scratch::new(&[("java.java", JAVA_FIXTURE)]);
    let members = class_members(&scratch.stdout(&["--plain", "classes", "java.java"]));

    assert_section(&members, "Java", "PublicShape", "SIDES", "Static Fields");
    assert_section(&members, "Java", "PublicShape", "area", "Instance Methods");
}

#[test]
fn plain_classes_group_rust_impl_members_under_their_struct() {
    let scratch = Scratch::new(&[("rust.rs", RUST_FIXTURE)]);
    let members = class_members(&scratch.stdout(&["--plain", "classes", "rust.rs"]));

    // Inherent impl.
    assert_section(&members, "Rust", "PublicStruct", "instance_method", "Instance Methods");
    assert_section(&members, "Rust", "PublicStruct", "new_associated_function", "Static Methods");
    // Trait impls (same-file trait and `fmt::Display`).
    assert_section(&members, "Rust", "PublicStruct", "required_trait_method", "Instance Methods");
    assert_section(&members, "Rust", "PublicStruct", "fmt", "Instance Methods");
}

/// Every record of a `--json` report's file entries.
fn report_records(json: &Value) -> Vec<&Value> {
    json["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("files[].symbols"))
        .collect()
}

/// Every record of a `--json-full` report's indexes.
fn index_records(json: &Value) -> Vec<&Value> {
    json["indexes"]
        .as_array()
        .expect("indexes")
        .iter()
        .flat_map(|index| index["symbols"].as_array().expect("indexes[].symbols"))
        .collect()
}

fn named<'a>(records: &[&'a Value], name: &str, context: &str) -> &'a Value {
    records
        .iter()
        .copied()
        .find(|record| record["identity"]["name"] == name)
        .unwrap_or_else(|| panic!("{context} `{name}`: missing"))
}

fn is_static(record: &Value) -> bool {
    record["modifiers"]
        .as_array()
        .is_some_and(|modifiers| modifiers.iter().any(|modifier| modifier["type"] == "Static"))
}

#[test]
fn json_symbols_carry_visibility_and_current_schema() {
    let scratch = Scratch::new(&[("typescript.ts", TYPESCRIPT_FIXTURE)]);
    // `--all-scopes`: the local `localValue` is asserted below.
    let json = json_of(&scratch.stdout(&["--json", "symbols", "--all-scopes", "typescript.ts"]));

    let schema = serde_json::json!({ "major": 5, "minor": 2 });
    assert_eq!(json["report_schema_version"], 1, "TypeScript: report_schema_version");
    assert_eq!(json["schema_version"], schema, "TypeScript: envelope schema_version");
    let records = report_records(&json);
    assert!(
        records.iter().all(|record| record["schema_version"] == schema),
        "TypeScript: every record is schema 5.0"
    );

    let make = named(&records, "privateStaticMethod", "TypeScript");
    assert_eq!(make["visibility"]["level"], "Private", "TypeScript `privateStaticMethod`: visibility");
    assert!(is_static(make), "TypeScript `privateStaticMethod`: static");
    assert_eq!(
        make["identity"]["container_kind"], "Class",
        "TypeScript `privateStaticMethod`: container_kind"
    );
    let alias = named(&records, "ModuleAlias", "TypeScript");
    assert_eq!(alias["visibility"]["level"], "ModulePrivate", "TypeScript `ModuleAlias`: visibility");
    assert_eq!(alias["kind"], "TypeAlias", "TypeScript `ModuleAlias`: kind");
    assert_eq!(
        named(&records, "localValue", "TypeScript")["visibility"]["level"], "Unknown",
        "TypeScript `localValue`: Unknown access"
    );
}

/// `--json-full` is the complete index, so it carries the re-export report
/// and the import marker; a plain `--json` listing computes neither.
#[test]
fn json_full_carries_the_reexport_report_and_json_does_not() {
    let scratch = Scratch::new(&[("typescript.ts", TYPESCRIPT_FIXTURE), ("lib.rs", "pub use std::io::Read;\nuse std::fmt::Debug;\n")]);
    let full = json_of(&scratch.stdout(&["--json-full", "symbols", "--no-cache", "typescript.ts", "lib.rs"]));
    let mut seen_rust = false;
    for index in full["indexes"].as_array().expect("indexes") {
        let file = index["file"].as_str().expect("file");
        assert_eq!(index["reexports"]["coverage"]["state"], "complete", "{file}");
        let imports = index["imports"].as_array().expect("imports");
        if file.ends_with("lib.rs") {
            seen_rust = true;
            let entries = index["reexports"]["entries"].as_array().expect("entries");
            assert_eq!(entries.len(), 1, "{file}: one forwarding site");
            assert_eq!(entries[0]["exported_name"], "Read", "{file}");
            // The marker lands on the forwarded import only.
            let marked: Vec<_> = imports.iter().filter(|import| import["reexported"] == true).map(|import| import["name"].clone()).collect();
            assert_eq!(marked, [serde_json::json!("Read")], "{file}: only the `pub use` import is marked");
        } else {
            assert!(index["reexports"]["entries"].as_array().is_some_and(Vec::is_empty), "{file}");
            assert!(imports.iter().all(|import| import["reexported"] == false), "{file}");
        }
    }
    assert!(seen_rust, "lib.rs was indexed");

    let plain = json_of(&scratch.stdout(&["--json", "symbols", "--no-cache", "typescript.ts"]));
    assert!(!plain.to_string().contains("\"reexports\""), "an unrequested report is not written");
}

/// Schema 4.x records carry the declaring keyword, the display header and
/// `identity.container_name`, and no wall-clock timestamps or `module_path`.
/// `declaration_text` is evidence, not display: only `--json-full` carries it.
#[test]
fn json_records_carry_keyword_signature_text_and_container_name() {
    let scratch = Scratch::new(&[("typescript.ts", TYPESCRIPT_FIXTURE)]);
    let stdout = scratch.stdout(&["--json", "symbols", "typescript.ts"]);
    let json = json_of(&stdout);
    let records = report_records(&json);

    let method = named(&records, "privateStaticMethod", "TypeScript");
    assert_eq!(method["identity"]["container_name"], "ExportedWidget", "`privateStaticMethod` container_name");
    assert!(method["identity"].get("keyword").is_none(), "a TypeScript method has no keyword");
    assert_eq!(
        method["source"]["signature_text"], "private static privateStaticMethod(): void",
        "`privateStaticMethod` signature_text"
    );

    let table = named(&records, "BOUNDARY_TABLE", "TypeScript");
    assert_eq!(table["identity"]["keyword"], "const", "`BOUNDARY_TABLE` keyword");
    assert_eq!(table["source"]["signature_text"], "const BOUNDARY_TABLE", "`BOUNDARY_TABLE` signature_text");
    assert!(
        records.iter().all(|record| record["source"].get("declaration_text").is_none()),
        "a default report omits declaration_text"
    );

    let full_stdout = scratch.stdout(&["--json-full", "symbols", "typescript.ts"]);
    let full = json_of(&full_stdout);
    let full_table = named(&index_records(&full), "BOUNDARY_TABLE", "TypeScript --json-full");
    assert!(
        full_table["source"]["declaration_text"].as_str().is_some_and(|text| text.contains("first: 1")),
        "`BOUNDARY_TABLE` declaration_text keeps the initializer in --json-full"
    );

    for removed in ["\"module_path\"", "created_at_epoch_ms", "updated_at_epoch_ms"] {
        assert!(!stdout.contains(removed), "schema 4.x JSON must not contain {removed}");
        assert!(!full_stdout.contains(removed), "schema 4.x --json-full must not contain {removed}");
    }
}

/// C and C++ JSON signatures carry the named function's own parameters: a
/// pointer return keeps them, a returned callback's parameters never replace
/// them, and a name under an array declarator is kept; the complete index
/// carries the same list.
#[test]
fn json_c_and_cpp_signatures_carry_the_functions_own_parameters() {
    let source = "int *pointer_return(int value) { return 0; }\n\
                  int (*callback_return(int actual))(long returned) { return 0; }\n\
                  int array_param(int values[2]) { return 0; }\n";
    let scratch = Scratch::new(&[("signatures.c", source), ("signatures.cpp", source)]);
    let parameter_names = |record: &Value| -> Vec<String> {
        record["kind_data"]["data"]["parameters"]
            .as_array()
            .map(|parameters| {
                parameters
                    .iter()
                    .map(|parameter| parameter["name"].as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };

    for file in ["signatures.c", "signatures.cpp"] {
        let json = json_of(&scratch.stdout(&["--json", "functions", file]));
        let full = json_of(&scratch.stdout(&["--json-full", "functions", file]));
        for (name, expected) in [
            ("pointer_return", ["value"]),
            ("callback_return", ["actual"]),
            ("array_param", ["values"]),
        ] {
            let record = named(&report_records(&json), name, file);
            assert_eq!(parameter_names(record), expected, "{file} `{name}`: --json parameters");
            let indexed = named(&index_records(&full), name, file);
            assert_eq!(parameter_names(indexed), expected, "{file} `{name}`: --json-full parameters");
        }
    }
}

/// A default `--json` record is the complete index's record of the same ID,
/// minus only what the listing did not request: kind, access, `static`,
/// container, keyword, signature text and kind data agree for every symbol.
#[test]
fn json_records_match_the_complete_index() {
    let files = [
        ("TypeScript", "typescript.ts", TYPESCRIPT_FIXTURE),
        ("Rust", "rust.rs", RUST_FIXTURE),
        ("C++", "cpp.cpp", CPP_FIXTURE),
        ("Java", "java.java", JAVA_FIXTURE),
        ("Bash", "bash.sh", BASH_FIXTURE),
    ];
    let scratch = Scratch::new(&files.map(|(_, name, contents)| (name, contents)));

    for (language, file, _) in files {
        let json = json_of(&scratch.stdout(&["--json", "symbols", "--all-scopes", file]));
        let full = json_of(&scratch.stdout(&["--json-full", "symbols", file]));
        let records = report_records(&json);
        let indexed = index_records(&full);
        assert!(!records.is_empty(), "{language}: no symbols reported");
        assert_eq!(records.len(), indexed.len(), "{language}: `symbols` lists every record");

        for record in records {
            let name = record["identity"]["name"].as_str().unwrap_or_default();
            let full_record = indexed
                .iter()
                .find(|candidate| candidate["id"] == record["id"])
                .unwrap_or_else(|| panic!("{language} `{name}`: no index record with its ID"));
            for field in [
                &["kind"][..],
                &["visibility", "level"],
                &["identity", "container_kind"],
                &["identity", "container_name"],
                &["identity", "keyword"],
                &["identity", "stable_key"],
                &["source", "file_path"],
                &["source", "declaration_span"],
                &["source", "signature_text"],
                &["kind_data"],
                &["docs", "raw_doc"],
            ] {
                let pick = |value: &Value| field.iter().fold(value.clone(), |value, key| value[*key].clone());
                assert_eq!(pick(record), pick(full_record), "{language} `{name}`: {field:?}");
            }
            assert_eq!(is_static(record), is_static(full_record), "{language} `{name}`: static");
        }
    }
}
