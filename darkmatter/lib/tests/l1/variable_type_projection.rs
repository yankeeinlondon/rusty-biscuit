//! The complete variable-type projection at the library's public API: root
//! unions, union normalization, every effective-schema layer, raw JSON
//! Schema, nested paths, reserved roots and host globals, call and operator
//! typing, and passivity. Every type is read through [`DeclaredTypes`] and
//! [`TypeScope`], the one projection the call check, composition, and DMLS
//! share.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, ArgumentType, Availability, BindingView, CallCheck, DeclaredType, DeclaredTypes, RawDeclarations,
    ScopeId, Shape, TypeScope, UnavailabilityReason, check_expression_calls, expression_type, parse, parse_spanned,
};
use darkmatter::markdown::schemas::catalog::{GlobalCatalog, GlobalRequirement};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, SimplifiedSchema, parse_yaml_schema};
use serde_json::json;
use tempfile::TempDir;

use ArgumentCategory::{Compatible, NullableMatch, Unknown};

fn schema(yaml: &str) -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).expect("YAML parses")).expect("schema parses")
}

/// The text a call-check message shows for `shape`.
fn text(shape: &Shape) -> String {
    ArgumentType::Typed(shape.clone()).to_string()
}

/// The projected top-level types as `name: type` text; `unknown` names are
/// absent.
fn projected(declared: &DeclaredTypes<'_>) -> Vec<(String, String)> {
    declared.top_level_types().iter().map(|(name, shape)| (name.clone(), text(shape))).collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected.iter().map(|(name, ty)| (name.to_string(), ty.to_string())).collect()
}

/// The type `path` reads at an unguarded site of `declared`.
fn type_of(declared: &DeclaredTypes<'_>, path: &str) -> Shape {
    let scope = TypeScope::unguarded(declared);
    scope.type_of_expr(&parse(path).expect("path parses")).expect("a stable subject").shape().clone()
}

fn declared_of(declared: &DeclaredTypes<'_>, path: &str) -> DeclaredType {
    let scope = TypeScope::unguarded(declared);
    scope.type_of_expr(&parse(path).expect("path parses")).expect("a stable subject").declared().clone()
}

/// The type a one-property schema `x: <definition>` projects for `x`;
/// `definition` is YAML.
fn ty(definition: &str) -> Shape {
    let schema = schema(&format!("x: {definition}"));
    type_of(&DeclaredTypes::new(Some(&schema)), "x")
}

fn checks(declared: &DeclaredTypes<'_>, expression: &str) -> Vec<CallCheck> {
    let expr = parse_spanned(expression).expect("the expression parses");
    check_expression_calls(&expr, &TypeScope::unguarded(declared))
}

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(&path, content).expect("write fixture");
    path
}

fn context_at(dir: &Path) -> biscuit_file::FileResolutionContext {
    biscuit_file::FileResolutionContext::new(dir).with_repository_root(dir)
}

fn markdown_at(path: &Path) -> Markdown {
    Markdown::try_from(path).expect("document loads").with_source(ComposeSource::File(path.to_path_buf()))
}

/// The motivating root union of the declarations design, arms and keys in
/// the given order.
fn motivating(reversed: bool) -> SimplifiedSchema {
    let specification = "- spec: file(required)\n  design: file\n  plan: file(required)\n  area: string\n";
    let review = "- area: string\n  plan: file(required)\n  review: file(required)\n";
    schema(&if reversed { format!("{review}{specification}") } else { format!("{specification}{review}") })
}

mod root_unions {
    use super::*;

    /// Criterion 1: an arm that does not declare a property contributes
    /// `unknown`, which absorbs the fold.
    #[test]
    fn the_motivating_table_folds_every_arm() {
        let schema = motivating(false);
        let declared = DeclaredTypes::new(Some(&schema));
        assert_eq!(projected(&declared), pairs(&[("area", "string | null"), ("plan", "file")]));
        for unknown in ["spec", "design", "review"] {
            assert_eq!(declared_of(&declared, unknown), DeclaredType::NoProjection, "{unknown}");
            assert!(matches!(type_of(&declared, unknown), Shape::Unknown { non_null: false }), "{unknown}");
        }
        // `plan` is required in both arms, so it is present.
        assert!(!type_of(&declared, "plan").admits_null());
    }

    #[test]
    fn the_fold_is_independent_of_arm_and_key_order() {
        let (forward, reversed) = (motivating(false), motivating(true));
        assert_eq!(
            projected(&DeclaredTypes::new(Some(&forward))),
            projected(&DeclaredTypes::new(Some(&reversed)))
        );
        let left = schema("- x: string(required)\n- x: number(required)\n");
        let right = schema("- x: number(required)\n- x: string(required)\n");
        assert_eq!(ty_in(&left), ty_in(&right));
        assert_eq!(text(&ty_in(&left)), "number | string");
    }

    #[test]
    fn arms_that_differ_fold_to_a_union_with_each_arms_optionality() {
        let schema = schema("- x: string(required)\n- x: number\n");
        assert_eq!(text(&ty_in(&schema)), "number | string | null");
    }

    fn ty_in(schema: &SimplifiedSchema) -> Shape {
        type_of(&DeclaredTypes::new(Some(schema)), "x")
    }
}

mod normalization {
    use super::*;

    fn null() -> Shape {
        ty("'null'")
    }

    fn unknown() -> Shape {
        ty("unknown")
    }

    /// Criterion 2: every row of the normalization table, in both orders.
    #[test]
    fn every_row_of_the_table() {
        let rows: Vec<(Vec<Shape>, &str)> = vec![
            (vec![ty("file(required)"), ty("file(required)")], "file"),
            (vec![ty("file(required)"), unknown()], "unknown"),
            (vec![ty("string(required)"), ty("number(required)")], "number | string"),
            (vec![ty("file(required)"), ty("string(required)")], "file | string"),
            (vec![ty("string(required)"), null()], "string | null"),
            (vec![ty("file(required)"), null(), null()], "file | null"),
            (vec![ty("string[](required)"), ty("string[](required)")], "string[]"),
            (vec![ty("string[](required)"), ty("number[](required)")], "number[] | string[]"),
            (vec![ty("string(required)"), ty("string[](required)")], "string | string[]"),
            (vec![unknown(), ty("[string(required), number(required)]")], "unknown"),
            (vec![unknown(), null()], "unknown"),
            (vec![null(), null()], "null"),
        ];
        for (inputs, expected) in rows {
            let forward = inputs.iter().skip(1).fold(inputs[0].clone(), |acc, next| acc.union(next));
            let backward = inputs.iter().rev().skip(1).fold(inputs[inputs.len() - 1].clone(), |acc, next| acc.union(next));
            assert_eq!(text(&forward), expected, "{inputs:?}");
            assert_eq!(forward, backward, "order independence: {inputs:?}");
        }
        // `unknown` absorbs `null` too: the union still admits it.
        assert!(unknown().union(&null()).admits_null());
    }

    /// Presence markers and descriptions are not part of a type: two arms that
    /// differ only there are one member.
    #[test]
    fn presence_markers_do_not_split_a_member() {
        let schema = schema("x:\n    - 'string(required) -> one'\n    - 'string(default(a)) -> two'\n    - number\n");
        // `required` on any arm hoists to the union, so it is present.
        assert_eq!(text(&type_of(&DeclaredTypes::new(Some(&schema)), "x")), "number | string");
    }

    #[test]
    fn a_property_level_union_normalizes_like_a_fold() {
        assert_eq!(text(&ty("[file(required), file(required)]")), "file");
        assert_eq!(text(&ty("[file(required), unknown]")), "unknown");
        assert_eq!(text(&ty("[string, number]")), "number | string | null");
        assert_eq!(
            ty("[string(required), number(required)]"),
            ty("[number(required), string(required)]")
        );
    }
}

mod layers {
    use super::*;

    /// Criterion 3: an imported named type, the baseline, a trigger, and the
    /// document each contribute their top-level properties, the document
    /// winning a shared one.
    #[test]
    fn every_layer_of_the_effective_schema_projects() {
        let repo = TempDir::new().unwrap();
        std::fs::create_dir_all(repo.path().join(".git")).unwrap();
        write(repo.path(), "types.yaml", "kind: schema\ntypes:\n    stamp: number(integer; required)\n");
        write(
            repo.path(),
            "schemas/marker.trigger.yaml",
            "kind: schema-trigger\nmatch:\n  gate: literal(on; required)\n$schema: payload.yaml\n",
        );
        write(repo.path(), "schemas/payload.yaml", "$schema:\n  from_trigger: boolean(required)\n  shared: boolean\n");
        let path = write(
            repo.path(),
            "doc.md",
            "---\n$schema:\n  imported: stamp@./types.yaml\n  own: string\n  shared: number(required)\n  \"<starting::x_>\": number\ngate: on\n---\nBody\n",
        );
        let baseline = schema("from_baseline: number(required)\nshared: string(required)\n");
        let effective = DarkmatterSchemas::new(context_at(repo.path()))
            .with_baseline(baseline)
            .unwrap()
            .with_trigger_discovery()
            .unwrap()
            .effective_for(&markdown_at(&path))
            .unwrap()
            .expect("an effective schema");
        let declared = DeclaredTypes::for_schema(Some(&effective));
        let types = projected(&declared);
        for (name, expected) in [
            ("imported", "number(integer)"),
            ("own", "string | null"),
            ("from_trigger", "boolean"),
            ("from_baseline", "number"),
            ("shared", "number"),
        ] {
            assert!(
                types.iter().any(|(projected, ty)| projected == name && ty == expected),
                "{name}: expected {expected} in {types:?}"
            );
        }
        // A pattern key declares no literal name.
        assert!(!types.iter().any(|(name, _)| name.starts_with('<') || name.starts_with("x_")), "{types:?}");
        assert_eq!(declared_of(&declared, "x_anything"), DeclaredType::Undeclared);
        // The document-only layer, as the call check read it before, sees
        // only the authored schema.
        let authored = DeclaredTypes::new(effective.simplified.as_ref());
        assert!(!projected(&authored).iter().any(|(name, _)| name == "from_trigger" || name == "from_baseline"));
    }

    /// Criterion 3, raw half: enumerable literal properties are visible with
    /// conservative types; an indeterminate one is `unknown`.
    #[test]
    fn a_raw_json_schema_exposes_its_enumerable_properties() {
        let raw = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "count": { "type": "integer" },
                "nullable": { "type": ["string", "null"] },
                "either": { "anyOf": [{ "type": "string" }, { "type": "number" }] },
                "doc": { "type": "string", "format": "darkmatter-file-reference" },
                "items": { "type": "array", "items": { "type": "string" } },
                "mode": { "enum": ["a", "b"] },
                "linked": { "$ref": "#/$defs/thing" },
                "open": {}
            },
            "required": ["name", "count", "nullable", "either", "doc", "items", "mode"],
            "allOf": [
                { "properties": { "joined": { "type": "boolean" } }, "required": ["joined"] },
                { "properties": { "name": { "type": "number" } } }
            ],
            "patternProperties": { "^x_": { "type": "number" } }
        });
        let declarations = RawDeclarations::from_json_schema(&raw);
        let names: Vec<&str> = declarations.names().collect();
        assert_eq!(
            names,
            ["name", "count", "nullable", "either", "doc", "items", "mode", "linked", "open", "joined"]
        );
        let typed = |name: &str| declarations.get(name).expect("declared").map(|def| text(&shape_of(def)));
        // `allOf` types `name` differently in two parts: their intersection is
        // not represented, so it is `unknown`, but still declared.
        assert_eq!(typed("name"), None);
        assert_eq!(typed("count").as_deref(), Some("number"));
        assert_eq!(typed("nullable").as_deref(), Some("string | null"));
        assert_eq!(typed("either").as_deref(), Some("number | string"));
        assert_eq!(typed("doc").as_deref(), Some("file"));
        assert_eq!(typed("items").as_deref(), Some("string[]"));
        assert_eq!(typed("mode").as_deref(), Some("literal(a) | literal(b)"));
        assert_eq!(typed("linked"), None, "a raw `$ref` is unknown");
        assert_eq!(typed("open"), None);
        assert_eq!(typed("joined").as_deref(), Some("boolean"));
        assert_eq!(declarations.get("x_any"), None, "a pattern declares no literal name");
    }

    /// A raw root union folds like a SimplifiedSchema one.
    #[test]
    fn a_raw_root_union_folds_its_arms() {
        let raw = json!({ "anyOf": [
            { "properties": { "plan": { "type": "string" }, "spec": { "type": "string" } }, "required": ["plan", "spec"] },
            { "properties": { "plan": { "type": "string" } }, "required": ["plan"] }
        ] });
        let declarations = RawDeclarations::from_json_schema(&raw);
        assert_eq!(declarations.get("plan").flatten().map(|def| text(&shape_of(def))).as_deref(), Some("string"));
        assert_eq!(declarations.get("spec"), Some(None));
    }

    /// The raw layer reaches the call check through a referenced JSON Schema
    /// document.
    #[test]
    fn a_referenced_raw_schema_types_its_properties() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "schema.json",
            r#"{"type":"object","properties":{"flag":{"type":"boolean"},"size":{"type":"number"}},"required":["flag","size"]}"#,
        );
        let path = write(dir.path(), "doc.md", "---\n$schema: ./schema.json\nflag: true\nsize: 3\n---\nBody\n");
        let effective = DarkmatterSchemas::new(context_at(dir.path()))
            .effective_for(&markdown_at(&path))
            .unwrap()
            .expect("an effective schema");
        assert!(effective.simplified.is_none(), "a raw document schema has no SimplifiedSchema");
        let declared = DeclaredTypes::for_schema(Some(&effective));
        assert_eq!(checks(&declared, "min(size, 5)")[0].category(), Compatible);
        assert!(checks(&declared, "min(flag, 5)")[0].is_rejected());
    }

    fn shape_of(def: &darkmatter::markdown::schemas::PropertyDef) -> Shape {
        let schema = SimplifiedSchema::Single({
            let mut shape = darkmatter::markdown::schemas::SchemaShape::new();
            shape.properties.insert("x".to_string(), def.clone());
            shape
        });
        type_of(&DeclaredTypes::new(Some(&schema)), "x")
    }
}

mod nested_paths {
    use super::*;

    /// Criterion 4: an optional object on the path adds `null`.
    #[test]
    fn a_member_of_an_optional_object_may_be_null() {
        let optional = schema("meta: \"{ count: number(required) }\"\n");
        assert_eq!(text(&type_of(&DeclaredTypes::new(Some(&optional)), "meta.count")), "number | null");
        let required = schema("meta: \"{ count: number(required) }(required)\"\n");
        let declared = DeclaredTypes::new(Some(&required));
        assert_eq!(text(&type_of(&declared, "meta.count")), "number");
        assert_eq!(text(&type_of(&declared, "meta['count']")), "number");
        // An undeclared member is `unknown`, never a guess.
        assert!(matches!(type_of(&declared, "meta.missing"), Shape::Unknown { .. }));
        assert_eq!(checks(&declared, "min(meta.count, 5)")[0].category(), Compatible);
        let declared = DeclaredTypes::new(Some(&optional));
        assert_eq!(checks(&declared, "min(meta.count, 5)")[0].category(), NullableMatch);
    }

    /// Criterion 32: a constant index projects the item type with `null`; a
    /// dynamic one is not a subject.
    #[test]
    fn constant_indices_project_the_item_type() {
        let schema = schema(
            "list: string[](required)\npair: \"tuple([number, string?])\"\nnested: \"{ rows: number[](required) }(required)\"\n",
        );
        let declared = DeclaredTypes::new(Some(&schema));
        assert_eq!(text(&type_of(&declared, "list[0]")), "string | null");
        assert_eq!(text(&type_of(&declared, "list[-1]")), "string | null");
        // `pair` itself is optional, so even its required position may be null.
        assert_eq!(text(&type_of(&declared, "pair[0]")), "number | null");
        assert_eq!(text(&type_of(&declared, "pair[1]")), "string | null");
        assert_eq!(text(&type_of(&declared, "pair[2]")), "null");
        assert_eq!(text(&type_of(&declared, "nested.rows[3]")), "number | null");
        let scope = TypeScope::unguarded(&declared);
        assert!(scope.type_of_expr(&parse("list[i]").unwrap()).is_none(), "a dynamic index has no subject");
        assert_eq!(checks(&declared, "min(list[i], 5)")[0].arguments[0].category, Unknown);
    }

    /// Criterion 32: a path through a recursive type opens one definition per
    /// segment and keeps a back-reference named.
    #[test]
    fn a_path_through_a_recursive_type_terminates() {
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "types.yaml",
            "kind: schema\ntypes:\n    node:\n        label: string(required)\n        children: node[](required)\n",
        );
        let path = write(dir.path(), "doc.md", "---\n$schema:\n  tree: node(required)@./types.yaml\n---\nBody\n");
        let effective = DarkmatterSchemas::new(context_at(dir.path()))
            .effective_for(&markdown_at(&path))
            .unwrap()
            .expect("an effective schema");
        let declared = DeclaredTypes::for_schema(Some(&effective));
        assert_eq!(text(&type_of(&declared, "tree")), "node");
        assert_eq!(text(&type_of(&declared, "tree.label")), "string");
        assert_eq!(text(&type_of(&declared, "tree.children")), "node[]");
        assert_eq!(text(&type_of(&declared, "tree.children[0]")), "node | null");
        let deep = format!("tree{}.label", ".children[0]".repeat(64));
        assert_eq!(text(&type_of(&declared, &deep)), "string | null");
    }
}

mod globals {
    use super::*;

    fn view(err: Availability) -> BindingView {
        BindingView::builder(ScopeId::new("test.event"))
            .declare("err", "The failure.", err)
            .declare("timing", "Durations.", Availability::Available)
            .build()
            .expect("valid view")
    }

    /// Criterion 5: reserved roots and Darkmatter's globals take their
    /// catalog shapes.
    #[test]
    fn reserved_roots_take_their_catalog_shapes() {
        let available = view(Availability::Available);
        let declared = DeclaredTypes::new(None).with_binding_view(&available);
        assert_eq!(text(&type_of(&declared, "err.msg")), "string");
        assert_eq!(text(&type_of(&declared, "err.code")), "string | null");
        assert_eq!(text(&type_of(&declared, "timing.document_ms")), "number(integer;min(0)) | null");
        assert_eq!(text(&type_of(&declared, "ctx.today")), "date");
        assert!(matches!(type_of(&declared, "env.HOME"), Shape::Unknown { .. }));
        assert!(matches!(declared_of(&declared, "err.msg"), DeclaredType::Host(Availability::Available)));
        assert!(checks(&declared, "min(err.msg, 5)")[0].category() != Compatible);
    }

    /// Criterion 5: a global the event declares unavailable gets no type.
    #[test]
    fn an_unavailable_global_has_no_type() {
        let reason = UnavailabilityReason::new("test.no_failure");
        let unavailable = view(Availability::Unavailable(reason));
        let declared = DeclaredTypes::new(None).with_binding_view(&unavailable);
        assert!(matches!(type_of(&declared, "err.msg"), Shape::Unknown { non_null: false }));
        assert!(matches!(declared_of(&declared, "err"), DeclaredType::Host(Availability::Unavailable(_))));
    }

    /// Criterion 5: a host global is typed by the catalog its host registers;
    /// the catalog alone declares nothing.
    #[test]
    fn a_host_global_takes_its_host_catalog_type() {
        const HOST: &[GlobalRequirement] = &[GlobalRequirement::shape("seq")];
        let catalog = GlobalCatalog::from_documents(
            "host.yaml",
            &[("host.yaml", "kind: schema\n$schema:\n    seq:\n        index: \"number(integer; min(1))\"\n")],
            HOST,
        )
        .expect("the host catalog loads");
        let roots = ["seq"];
        let typed = DeclaredTypes::new(None).with_host_globals(&roots).with_global_catalog(&catalog);
        assert_eq!(text(&type_of(&typed, "seq.index")), "number(integer;min(1)) | null");
        let untyped = DeclaredTypes::new(None).with_host_globals(&roots);
        assert!(matches!(type_of(&untyped, "seq.index"), Shape::Unknown { .. }));
        let undeclared = DeclaredTypes::new(None).with_global_catalog(&catalog);
        assert!(matches!(declared_of(&undeclared, "seq.index"), DeclaredType::NoProjection));
    }
}

mod precedence {
    use super::*;

    /// Criterion 31: bare and `fm` paths read one document binding; a host
    /// global shadows only the bare spelling.
    #[test]
    fn bare_and_fm_paths_agree_unless_a_host_global_shadows_the_bare_one() {
        let schema = schema("meta: \"{ count: number(required) }\"\nseq: string(required)\n");
        let roots = ["seq"];
        let declared = DeclaredTypes::new(Some(&schema)).with_host_globals(&roots);
        assert_eq!(type_of(&declared, "meta.count"), type_of(&declared, "fm.meta.count"));
        assert_eq!(text(&type_of(&declared, "fm.seq")), "string");
        assert!(matches!(declared_of(&declared, "seq"), DeclaredType::Host(_)));
        assert!(matches!(type_of(&declared, "seq"), Shape::Unknown { .. }), "an untyped host global");
        // A guard on the unavailable global never makes the property reachable.
        let reason = UnavailabilityReason::new("test.unavailable");
        let view = BindingView::builder(ScopeId::new("test.event"))
            .declare("seq", "The step.", Availability::Unavailable(reason))
            .build()
            .unwrap();
        let declared = DeclaredTypes::new(Some(&schema)).with_binding_view(&view);
        let scope = TypeScope::unguarded(&declared).assume(&parse("seq").unwrap(), darkmatter::markdown::compose::expression::Polarity::True);
        let seq = scope.type_of_expr(&parse("seq").unwrap()).unwrap();
        assert!(matches!(seq.declared(), DeclaredType::Host(Availability::Unavailable(_))));
        assert!(matches!(seq.shape(), Shape::Unknown { .. }));
        assert_eq!(text(&type_of(&declared, "fm.seq")), "string");
    }
}

mod call_typing {
    use super::*;

    const WORKED: &str = "count: string\nmaybe: number\nsize: number(required)\nflag: boolean(required)\n";

    /// Criterion 6: rows f and i are exact; the inner call keeps its own
    /// category.
    #[test]
    fn worked_example_rows_f_and_i_are_exact() {
        let schema = schema(WORKED);
        let declared = DeclaredTypes::new(Some(&schema));
        let row_f = checks(&declared, "min(length(count), 5)");
        assert_eq!(row_f[0].category(), Compatible);
        assert_eq!(row_f[1].function, "length");
        assert_eq!(row_f[1].category(), NullableMatch);
        assert_eq!(checks(&declared, "min(maybe || 0, 5)")[0].category(), Compatible);
    }

    #[test]
    fn operators_and_calls_are_typed_from_their_operands() {
        let schema = schema(WORKED);
        let declared = DeclaredTypes::new(Some(&schema));
        let scope = TypeScope::unguarded(&declared);
        let typed = |expression: &str| text(&expression_type(&parse_spanned(expression).unwrap(), &scope));
        assert_eq!(typed("length(count)"), "number");
        assert_eq!(typed("maybe || 0"), "literal(0.0) | number");
        assert_eq!(typed("maybe || count"), "number | string | null");
        assert_eq!(typed("flag ? size : count"), "number | string | null");
        // Each ternary branch is typed under the guard's narrowing.
        assert_eq!(typed("is_number(maybe) ? maybe : 0"), "literal(0.0) | number");
        assert_eq!(typed("size + 1"), "unknown");
        assert_eq!(typed("no_such_function(size)"), "unknown");
        assert_eq!(checks(&declared, "min(flag ? size : 1, 5)")[0].category(), Compatible);
        assert!(checks(&declared, "min(length(count) > 1, 5)")[0].category() != Compatible);
    }

    /// An overloaded callee's return is the union over every overload its
    /// argument count can select, never chosen by argument type.
    #[test]
    fn an_overloaded_call_unions_its_candidate_returns() {
        let _overloaded = hooks::register(
            FunctionRegistration::builder(
                "pick",
                "function(parameters(x: number); returns(number)) | function(parameters(x: string); returns(string)) \
                 | function(parameters(x: string, y: string); returns(boolean))",
            )
            .build()
                .expect("the overloads compile"),
            Arc::new(|_| Ok(json!(1))),
        );
        let declared = DeclaredTypes::new(None);
        let scope = TypeScope::unguarded(&declared);
        assert_eq!(text(&expression_type(&parse_spanned("pick(1)").unwrap(), &scope)), "number | string");
        assert_eq!(text(&expression_type(&parse_spanned("pick('a', 'b')").unwrap(), &scope)), "boolean");
    }
}

mod passivity {
    use super::*;

    fn probe() -> TestFunctionGuard {
        hooks::register(
            FunctionRegistration::builder("probe", "function(parameters(x: unknown | null); returns(number))")
                .build()
                .expect("the probe compiles"),
            Arc::new(|_| Ok(json!(1))),
        )
    }

    /// Criterion 7: projecting every layer and typing a call result reads no
    /// file (the fixture is gone before projection), enters no handler, and
    /// leaves the schema and frontmatter unchanged.
    #[test]
    fn projection_reads_no_file_and_runs_no_handler() {
        let _probe = probe();
        let dir = TempDir::new().unwrap();
        write(dir.path(), "types.yaml", "kind: schema\ntypes:\n    stamp: number(required)\n");
        let path = write(dir.path(), "doc.md", "---\n$schema:\n  stamp: stamp@./types.yaml\n  meta: \"{ count: number(required) }\"\n---\nBody\n");
        let markdown = markdown_at(&path);
        let effective: EffectiveSchema = DarkmatterSchemas::new(context_at(dir.path()))
            .effective_for(&markdown)
            .unwrap()
            .expect("an effective schema");
        let frontmatter_before = markdown.frontmatter().clone();
        let json_before = effective.json_schema.as_ref().clone();
        // A failing access stub: every file the schema came from is gone.
        drop(dir);
        assert!(!path.exists());

        hooks::reset_handler_entries();
        let declared = DeclaredTypes::for_schema(Some(&effective));
        let first = projected(&declared);
        assert_eq!(text(&type_of(&declared, "meta.count")), "number | null");
        assert_eq!(checks(&declared, "min(probe(stamp), 5)")[0].category(), Compatible);
        assert_eq!(hooks::handler_entries("probe"), 0, "typing a call never runs its handler");
        assert_eq!(hooks::handler_entries("min"), 0);
        assert_eq!(projected(&DeclaredTypes::for_schema(Some(&effective))), first, "deterministic");
        assert_eq!(markdown.frontmatter(), &frontmatter_before);
        assert_eq!(effective.json_schema.as_ref(), &json_before);
    }

    /// Criterion 7: schema-stage `null` materialization is unchanged. Only the
    /// document's own optional, no-default properties become `null`; a
    /// property only the baseline declares (now projected) is never inserted,
    /// a present value is preserved, and recomposing is idempotent.
    #[test]
    fn null_materialization_is_unchanged() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "doc.md",
            "---\n$schema:\n  optional: string\n  defaulted: string(default(x))\n  present: string\npresent: kept\n---\n{{ is_null(optional) }} {{ present }}\n",
        );
        let compose = || {
            let options = darkmatter::markdown::compose::ComposeOptions::new_with_context(
                darkmatter::markdown::compose::ComposeContext::capture_for_content(dir.path(), ""),
            )
            .with_source_file(path.clone())
            .with_baseline_schema(schema("from_baseline: string\n"));
            let (composed, _report) = markdown_at(&path)
                .compose_with(&crate::request_support::request(options))
                .expect("composes");
            composed
        };
        let first = compose();
        let data = first.frontmatter().as_map();
        assert_eq!(data.get("optional"), Some(&json!(null)));
        assert_eq!(data.get("present"), Some(&json!("kept")));
        assert!(!data.contains_key("from_baseline"), "a baseline-only property is never inserted");
        assert_eq!(first.content().trim_end(), "true kept");
        let second = compose();
        assert_eq!(second.frontmatter().as_map(), data, "idempotent");
    }
}

mod lifecycle_calls {
    use super::*;
    use darkmatter::lifecycle::md_host;

    /// The lifecycle rejection under `md`'s registry, against `schema`.
    fn rejection(frontmatter: &str, schema: Option<&EffectiveSchema>) -> Option<darkmatter::lifecycle::error::LifecycleError> {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML parses");
        let value: serde_json::Value = serde_json::to_value(yaml).expect("JSON");
        let source = Path::new("doc.md");
        let config = md_host::parse_lifecycle(&value, source, None).expect("the lifecycle parses");
        darkmatter::lifecycle::reject_incompatible_calls(&value, &config, source, None, md_host::registry(), schema).err()
    }

    fn effective_for(dir: &Path, frontmatter: &str) -> EffectiveSchema {
        let path = write(dir, "doc.md", &format!("---\n{frontmatter}---\nBody\n"));
        DarkmatterSchemas::new(context_at(dir)).effective_for(&markdown_at(&path)).unwrap().expect("a schema")
    }

    /// Criterion 8: `min(flag, 5)` in a lifecycle action and in a `when`
    /// predicate is rejected with the verdict the body call gets.
    #[test]
    fn a_lifecycle_call_is_rejected_exactly_like_the_body_call() {
        let dir = TempDir::new().unwrap();
        let schema_yaml = "$schema:\n  flag: boolean(required)\nflag: true\n";
        let body_schema = schema("flag: boolean(required)\n");
        let body = checks(&DeclaredTypes::new(Some(&body_schema)), "min(flag, 5)").remove(0);
        assert!(body.is_rejected());
        for (stack, property) in [
            ("composed:\n  - stdout: \"{{ min(flag, 5) }}\"\n", "composed[0].stdout.text"),
            ("composed:\n  - when: min(flag, 5) > 1\n    then:\n      - stdout: hi\n", "composed[0].when"),
            (
                "finalize:\n  - when: \"false\"\n    then:\n      - stdout: ok\n    else:\n      - stdout: \"a {{ min(fm.flag, 5) }} b\"\n",
                "finalize[0].else[0].stdout.text",
            ),
        ] {
            let frontmatter = format!("{schema_yaml}{stack}");
            let effective = effective_for(dir.path(), &frontmatter);
            let error = rejection(&frontmatter, Some(&effective)).unwrap_or_else(|| panic!("{stack}: not rejected"));
            let check = error.rejected_call().expect("a call verdict");
            assert_eq!(check.rejection_message(), body.rejection_message(), "{stack}");
            assert_eq!(check.category(), body.category());
            let darkmatter::lifecycle::error::LifecycleError::LifecycleCallRejected { property: found, .. } = &error
            else {
                panic!("{error:?}");
            };
            assert_eq!(found, property);
        }
    }

    #[test]
    fn a_compatible_or_untyped_lifecycle_call_is_not_rejected() {
        let dir = TempDir::new().unwrap();
        let typed = "$schema:\n  size: number(required)\nsize: 3\ncomposed:\n  - when: min(size, 5) > 1\n    then:\n      - stdout: \"{{ min(size, 5) }}\"\n";
        assert!(rejection(typed, Some(&effective_for(dir.path(), typed))).is_none());
        // No schema: `flag` is unknown, which is never proven wrong.
        assert!(rejection("composed:\n  - stdout: \"{{ min(flag, 5) }}\"\n", None).is_none());
        // A literal is judged by its value even without a schema.
        assert!(rejection("composed:\n  - stdout: \"{{ min(true, 5) }}\"\n", None).is_some());
    }
}
