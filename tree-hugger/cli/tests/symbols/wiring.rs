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
            ("Rust", "crate_function", "visibility (crate)", "crate function crate_function() -> ()"),
            ("Rust", "PublicAlias", "kind (type alias)", "public type alias PublicAlias"),
            ("Rust", "UnitVariant", "kind and container (enum variant)", "public enum variant UnitVariant [in enum PublicEnum]"),
            ("Rust", "crate_associated_function", "container (inherent impl member)", "crate method crate_associated_function() -> () [in type PublicStruct]"),
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

    let typescript = scratch.stdout(&["--plain", "symbols", "typescript.ts"]);
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
        let pretty = stdout_of(
            scratch.hug_with_env(&pretty_args, &[("CLICOLOR_FORCE", "1")]),
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

fn files_symbols(json: &Value) -> &Vec<Value> {
    json["files"][0]["symbols"].as_array().expect("files[0].symbols")
}

fn index_records(json: &Value) -> &Vec<Value> {
    json["symbol_indexes"][0]["symbols"].as_array().expect("symbol_indexes[0].symbols")
}

#[test]
fn json_symbols_carry_visibility_and_schema_3_0() {
    let scratch = Scratch::new(&[("typescript.ts", TYPESCRIPT_FIXTURE)]);
    let json = json_of(&scratch.stdout(&["--json", "symbols", "typescript.ts"]));

    let schema = serde_json::json!({ "major": 3, "minor": 0 });
    assert_eq!(json["schema_version"], schema, "TypeScript: envelope schema_version");
    assert_eq!(
        json["symbol_indexes"][0]["schema_version"], schema,
        "TypeScript: symbol index schema_version"
    );

    let symbol = |name: &str| {
        files_symbols(&json)
            .iter()
            .find(|symbol| symbol["name"] == name)
            .unwrap_or_else(|| panic!("TypeScript `{name}`: missing from files[].symbols"))
            .clone()
    };

    let make = symbol("privateStaticMethod");
    assert_eq!(make["visibility"], "Private", "TypeScript `privateStaticMethod`: visibility");
    assert_eq!(make["is_static"], true, "TypeScript `privateStaticMethod`: is_static");
    assert_eq!(make["container_kind"], "Class", "TypeScript `privateStaticMethod`: container_kind");
    assert!(
        make["signature"].get("visibility").is_none(),
        "TypeScript `privateStaticMethod`: visibility moved off the signature"
    );
    assert_eq!(symbol("ModuleAlias")["visibility"], "ModulePrivate", "TypeScript `ModuleAlias`: visibility");
    assert_eq!(symbol("ModuleAlias")["kind"], "TypeAlias", "TypeScript `ModuleAlias`: kind");
    assert!(
        symbol("localValue").get("visibility").is_none(),
        "TypeScript `localValue`: Unknown access must omit `visibility`"
    );
}

/// The legacy `files[]` symbols and the full symbol indexes agree on kind,
/// access, `static` and container kind for every symbol.
#[test]
fn json_legacy_symbols_match_the_symbol_indexes() {
    let files = [
        ("TypeScript", "typescript.ts", TYPESCRIPT_FIXTURE),
        ("Rust", "rust.rs", RUST_FIXTURE),
        ("C++", "cpp.cpp", CPP_FIXTURE),
        ("Java", "java.java", JAVA_FIXTURE),
        ("Bash", "bash.sh", BASH_FIXTURE),
    ];
    let scratch = Scratch::new(&files.map(|(_, name, contents)| (name, contents)));

    for (language, file, _) in files {
        let json = json_of(&scratch.stdout(&["--json", "symbols", file]));
        let records = index_records(&json);
        let symbols = files_symbols(&json);
        assert!(!symbols.is_empty(), "{language}: no symbols rendered");

        for symbol in symbols {
            let name = symbol["name"].as_str().unwrap_or_default();
            let line = &symbol["range"]["start_line"];
            let record = records
                .iter()
                .find(|record| {
                    record["identity"]["name"] == name
                        && record["source"]["declaration_span"]["start"]["line"] == *line
                })
                .unwrap_or_else(|| panic!("{language} `{name}` (line {line}): no matching index record"));

            let record_level = &record["visibility"]["level"];
            let expected_level = match symbol.get("visibility") {
                Some(level) => level.clone(),
                None => Value::from("Unknown"),
            };
            assert_eq!(*record_level, expected_level, "{language} `{name}`: visibility");
            assert_eq!(record["kind"], symbol["kind"], "{language} `{name}`: kind");
            assert_eq!(
                record["identity"].get("container_kind"),
                symbol.get("container_kind"),
                "{language} `{name}`: container_kind"
            );
            let record_static = record["modifiers"]
                .as_array()
                .is_some_and(|modifiers| modifiers.iter().any(|m| m["type"] == "Static"));
            let symbol_static = symbol.get("is_static").and_then(Value::as_bool).unwrap_or(false);
            assert_eq!(record_static, symbol_static, "{language} `{name}`: is_static");
        }
    }
}
