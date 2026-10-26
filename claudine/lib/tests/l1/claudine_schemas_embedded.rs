//! Claudine's embedded schema bundle and the launch verdict that uses it
//! (`2026-10-21-claudine-schemas`, criteria 6, 10, and 13).
//!
//! - The embed: `CLAUDINE_SCHEMA_FILES` is the seven repository files byte for
//!   byte, each parses as its kind, and every export and every payload resolves
//!   through the embedded seam with no disk schema anywhere.
//! - The input robustness matrix of `claudine_schemas_matrix.rs`, walked with
//!   each edited bundle supplied as an embedded bundle.
//! - Schema locations and the last-resort rule, through Claudine's public
//!   launch verdict (`pre_validate_schema`) and the effective schema it judges
//!   by.
//! - The strict policy for Claudine controls at the library boundary.
//! - Passive parity: for the AC5 fixtures, the launch verdict and the DMLS
//!   path agree.
//!
//! Every context is built from a `RequestSnapshot` with a fixture home and an
//! explicit environment inside a temporary directory; neither the checkout,
//! the process environment, nor the process CWD is consulted.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::composition::schema::bundle::{
    CLAUDINE_SCHEMA_BUNDLE_NAME, CLAUDINE_SCHEMA_FILES, CLAUDINE_TRIGGER_FILES, claudine_schema_bundle,
    is_claudine_owned,
};
use claudine::composition::{
    CompositionError, PreValidatedSchema, drop_invalid_optionals, pre_validate_schema,
    resolve_composition_source_in_context,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::triggers::parse_trigger_envelope_from_str;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, EmbeddedSchemaBundle, SchemaOriginKind, SchemaRootKind, SchemaRoots,
    StandaloneSchemaEnvelope, TriggerSource, parse_standalone_schema_document,
};

use crate::claudine_schemas::{AC5_GROUPS, BundleFile, SchemaRepo, problems, shipped_bundle};
use crate::claudine_schemas_matrix::{
    Half, Origin, Syntax, assert_halves_cover_the_shapes, assert_non_string_trigger_key_errors, column_names, controls_from,
    walk_from,
};

// ── Shared helpers ─────────────────────────────────────────────────────────

/// The top-level property each JSON pointer names, deduplicated.
fn top_level(pointers: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    pointers
        .into_iter()
        .filter_map(|pointer| pointer.split('/').nth(1).map(str::to_string))
        .collect()
}

/// The problem pointers of a launch-verdict rejection; panics on any other
/// outcome.
fn rejected(outcome: Result<PreValidatedSchema, CompositionError>, what: &str) -> Vec<String> {
    match outcome {
        Err(CompositionError::SchemaValidation { problems, .. }) => problems,
        Err(other) => panic!("{what}: expected SchemaValidation, got {other:?}"),
        Ok(validated) => panic!("{what}: expected a rejection, it passed (dropped {:?})", validated.dropped_optionals),
    }
}

/// Asserts the launch verdict rejects `what` with a problem under `/{property}`.
fn assert_rejects_at(outcome: Result<PreValidatedSchema, CompositionError>, property: &str, what: &str) {
    let found = rejected(outcome, what);
    assert!(
        found.iter().any(|pointer| pointer == &format!("/{property}") || pointer.starts_with(&format!("/{property}/"))),
        "{what}: a problem under `/{property}`, got {found:?}"
    );
}

/// Asserts the launch verdict passes `what` and drops nothing.
fn assert_accepts(outcome: Result<PreValidatedSchema, CompositionError>, what: &str) {
    match outcome {
        Ok(validated) => assert!(validated.dropped_optionals.is_empty(), "{what}: dropped {:?}", validated.dropped_optionals),
        Err(error) => panic!("{what}: expected the verdict to pass, got {error:?}"),
    }
}

/// Every file below `dir`, recursively.
fn files_below(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            found.extend(files_below(&path));
        } else {
            found.push(path);
        }
    }
    found
}

// ── AC6: the embed ─────────────────────────────────────────────────────────

/// A temporary directory outside any repository: `work/` is the launch and
/// document folder and `home/` an empty fixture home. No `schemas/` folder
/// exists anywhere, and the environment is empty, so no disk schema root is
/// searched.
struct Bare {
    dir: tempfile::TempDir,
    context: FileResolutionContext,
}

impl Bare {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("work")).expect("work dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        let snapshot =
            RequestSnapshot::new(root.join("work")).with_home(Some(root.join("home"))).with_env(HashMap::new());
        let context = build_resolution_context(&snapshot).expect("fixture context");
        Self { dir, context }
    }

    /// Discovery with `bundle` as the only trigger source.
    fn schemas(&self, bundle: EmbeddedSchemaBundle) -> DarkmatterSchemas {
        let schemas = DarkmatterSchemas::new(self.context.clone())
            .with_embedded_trigger_bundle(bundle)
            .and_then(DarkmatterSchemas::with_trigger_discovery)
            .expect("the embedded bundle loads");
        let roots = SchemaRoots::for_document(&self.context).expect("schema roots");
        assert!(roots.searched().is_empty(), "no disk schema root exists: {:?}", roots.searched());
        schemas
    }

    /// No file was written: every resolution happened in memory.
    fn assert_untouched(&self) {
        let written = files_below(self.dir.path());
        assert!(written.is_empty(), "no schema was extracted to disk: {written:?}");
    }
}

/// An in-memory document holding `frontmatter`.
fn memory_document(frontmatter: &str) -> Markdown {
    Markdown::from(format!("---\n{frontmatter}\n---\nbody\n").as_str())
}

/// The effective schema of an in-memory document, and its verdict.
fn judge_memory(schemas: &DarkmatterSchemas, frontmatter: &str) -> (EffectiveSchema, Vec<(String, String)>, bool) {
    let markdown = memory_document(frontmatter);
    let effective = schemas
        .effective_for(&markdown)
        .unwrap_or_else(|error| panic!("`{frontmatter}` resolves: {error}"))
        .unwrap_or_else(|| panic!("`{frontmatter}` selects a trigger"));
    let report = schemas.validate(&markdown).expect("validation");
    (effective, problems(&report), report.valid)
}

#[test]
fn embedded_files_are_the_repository_files_byte_for_byte() {
    let repository: [(&str, &str); 7] = [
        ("action.yaml", include_str!("../../../schemas/action.yaml")),
        ("claudine.yaml", include_str!("../../../schemas/claudine.yaml")),
        ("inline-compose.yaml", include_str!("../../../schemas/inline-compose.yaml")),
        ("lifecycle.yaml", include_str!("../../../schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../schemas/linking.yaml")),
        ("sequence-types.yaml", include_str!("../../../schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../schemas/sequence.yaml")),
    ];
    assert_eq!(CLAUDINE_SCHEMA_FILES, &repository[..], "the embed is the seven repository files, unchanged");
    assert_eq!(claudine_schema_bundle().name(), CLAUDINE_SCHEMA_BUNDLE_NAME);
    assert_eq!(claudine_schema_bundle().files(), CLAUDINE_SCHEMA_FILES);

    let mut triggers = Vec::new();
    for (name, text) in CLAUDINE_SCHEMA_FILES {
        let envelope = parse_trigger_envelope_from_str(text).unwrap_or_else(|error| panic!("{name}: {error}"));
        if envelope.is_some() {
            triggers.push(*name);
            continue;
        }
        let library = parse_standalone_schema_document(text, name)
            .unwrap_or_else(|error| panic!("{name} parses: {error}"))
            .unwrap_or_else(|| panic!("{name} is a schema library"));
        assert_eq!(library.envelope, StandaloneSchemaEnvelope::Tagged, "{name} is a `kind: schema` library");
    }
    assert_eq!(triggers, CLAUDINE_TRIGGER_FILES, "exactly the three trigger files are trigger envelopes");
}

/// One export's probes: its library, its name, and a value it accepts and one
/// it rejects (as YAML flow text).
type ExportProbe = (&'static str, &'static str, &'static str);

const ACTION_PROBES: &[ExportProbe] = &[
    ("interpolation", "\"{{ ctx.x }}\"", "plain"),
    ("count", "3", "-1"),
    ("communicate", "{ say: hi }", "{ inf0: hi }"),
    ("flow", "{ retry: 2 }", "{ retry: { max: 2 } }"),
    ("shell", "{ shell: ls }", "{ shell: { cmd: ls } }"),
    ("expression", "{ set: { a: 1 } }", "{ set: [1] }"),
    ("bundle", "{ say: hi, warn: careful }", "{ say: { text: hi }, warn: careful }"),
    ("initialize-bundle", "{ say: hi, warn: careful }", "{ say: hi, shell: ls }"),
];

const LIFECYCLE_PROBES: &[ExportProbe] = &[
    ("bare-verb", "stop", "jump"),
    ("loose-body", "[anything, 1]", "42"),
    ("stack", "[stop, { info: hi }]", "[{ inf0: hi }]"),
    ("item-1", "stop", "{ inf0: hi }"),
    ("conditional-1", "{ when: \"ctx.x\", then: [stop] }", "{ when: \"ctx.x\" }"),
    ("stack-2", "[{ info: hi }]", "[{ inf0: hi }]"),
    ("item-2", "{ info: hi }", "{ inf0: hi }"),
    ("conditional-2", "{ when: \"ctx.x\", then: [stop] }", "{ when: \"ctx.x\" }"),
    ("stack-3", "[{ info: hi }]", "[{ inf0: hi }]"),
    ("item-3", "{ info: hi }", "{ inf0: hi }"),
    ("conditional-3", "{ when: \"ctx.x\", then: [anything] }", "{ when: \"ctx.x\" }"),
    ("initialize-stack", "[{ info: hi }]", "[{ shell: ls }]"),
    ("initialize-item-1", "{ info: hi }", "{ shell: ls }"),
    ("initialize-conditional-1", "{ when: \"ctx.x\", then: [stop] }", "{ when: \"ctx.x\" }"),
    ("initialize-stack-2", "[stop]", "[{ shell: ls }]"),
    ("initialize-item-2", "{ info: hi }", "{ shell: ls }"),
    ("initialize-conditional-2", "{ when: \"ctx.x\", then: [stop] }", "{ when: \"ctx.x\" }"),
    ("initialize-stack-3", "[stop]", "[{ shell: ls }]"),
    ("initialize-item-3", "{ info: hi }", "{ shell: ls }"),
    ("initialize-conditional-3", "{ when: \"ctx.x\", then: [anything] }", "{ when: \"ctx.x\" }"),
    ("loop-action", "{ op: increment, prop: n }", "{ op: jump, prop: n }"),
    ("loop-action-entry", "increment(n)", "{ op: increment }"),
    ("loop", "{ while: \"ctx.x\", max: 3 }", "{ max: 0 }"),
];

/// The first level and the untyped boundary; the levels between are judged
/// through the disk bundle in `sequence_schema.rs`. Every probe inlines the
/// container types, so a longer list makes this test slow.
const SEQUENCE_TYPES_PROBES: &[ExportProbe] = &[
    ("steps", "[a, { name: b, shell: x }]", "{ a: 1 }"),
    ("task", "{ name: a, prompt: a.md, with: { x: 1 }, topic: data }", "{ name: a, with: [1] }"),
    ("group", "{ tasks: [{ name: a, shell: x }], max_parallel: 2 }", "{ tasks: [{ name: a }], execution: parallel }"),
    ("task-3", "{ name: c, sequence: [{ with: [1] }] }", "{ name: c, with: [1] }"),
    ("group-3", "{ tasks: [{ anything: 1 }] }", "{ tasks: [] }"),
];

/// Negative probes are lists or mappings where a scalar is declared, since
/// validation coerces scalars to text (implementation-log ruling R5).
const LINKING_PROBES: &[ExportProbe] = &[
    ("name", "x", "{ a: 1 }"),
    ("allowed-tools", "Read", "{ a: 1 }"),
    ("tools", "[Read]", "[{ a: 1 }]"),
    ("skills", "review", "{ a: 1 }"),
    ("license", "MIT", "[1]"),
    ("compatibility", "{ claude: \">=2\" }", "[1]"),
    ("metadata", "{ a: [1] }", "[1]"),
    ("user-invocable", "true", "[1]"),
    ("disable-model-invocation", "false", "[1]"),
    ("argument-hint", "\"<file>\"", "{ a: 1 }"),
    ("max_turns", "4", "[4]"),
];

/// A test trigger, added to a copy of the embedded bundle, whose payload
/// imports every export of `library` as `t_{export}`. Its imports resolve
/// inside the bundle exactly as the shipped payloads' do.
fn exports_trigger(library: &str, probes: &[ExportProbe]) -> String {
    let marker = library.trim_end_matches(".yaml");
    let mut text = format!("kind: trigger-schema\nmatch:\n    probe_{marker}: string(required)\n$schema:\n");
    for (export, _, _) in probes {
        text.push_str(&format!("    t_{export}: \"{export}@./{library}\"\n"));
    }
    text
}

/// Resolves every export of `library` through the embedded seam with no disk
/// schemas, and judges each export's probes.
fn assert_exports_resolve_embedded(library: &'static str, probes: &[ExportProbe]) {
    let bare = Bare::new();
    let exports_name: &'static str = Box::leak(format!("exports-{library}").into_boxed_str());
    let mut files: Vec<(&'static str, &'static str)> = CLAUDINE_SCHEMA_FILES.to_vec();
    files.push((exports_name, Box::leak(exports_trigger(library, probes).into_boxed_str())));
    let bundle = EmbeddedSchemaBundle::new(CLAUDINE_SCHEMA_BUNDLE_NAME, Box::leak(files.into_boxed_slice()));
    let schemas = bare.schemas(bundle);
    let marker = library.trim_end_matches(".yaml");

    let mut failures = Vec::new();
    for (export, good, bad) in probes {
        let property = format!("t_{export}");
        let (effective, found, valid) = judge_memory(&schemas, &format!("probe_{marker}: on\n{property}: {good}"));
        if !valid {
            failures.push(format!("{export}: `{good}` is rejected: {found:?}"));
        }
        let origin = effective.origins.get(&property).unwrap_or_else(|| panic!("{property} has an origin"));
        assert_eq!(origin.kind, SchemaOriginKind::Trigger);
        assert_eq!(
            origin.trigger_source(),
            Some(TriggerSource::Embedded { bundle: CLAUDINE_SCHEMA_BUNDLE_NAME.to_string(), name: exports_name.to_string() }),
            "{export} is declared by the embedded test trigger"
        );
        assert!(effective.dependencies().is_empty(), "an embedded schema has no file dependencies");

        let (_, found, valid) = judge_memory(&schemas, &format!("probe_{marker}: on\n{property}: {bad}"));
        let under = top_level(found.iter().map(|(pointer, _)| pointer.clone()));
        if valid || under != BTreeSet::from([property.clone()]) {
            failures.push(format!("{export}: `{bad}` must be rejected under /{property}: {found:?}"));
        }
    }
    assert!(failures.is_empty(), "{library}:\n{}", failures.join("\n"));
    bare.assert_untouched();
}

#[test]
fn embedded_action_exports_resolve_without_disk_schemas() {
    assert_exports_resolve_embedded("action.yaml", ACTION_PROBES);
}

#[test]
fn embedded_lifecycle_exports_resolve_without_disk_schemas() {
    assert_exports_resolve_embedded("lifecycle.yaml", LIFECYCLE_PROBES);
}

#[test]
fn embedded_linking_exports_resolve_without_disk_schemas() {
    assert_exports_resolve_embedded("linking.yaml", LINKING_PROBES);
}

#[test]
fn embedded_sequence_types_exports_resolve_without_disk_schemas() {
    assert_exports_resolve_embedded("sequence-types.yaml", SEQUENCE_TYPES_PROBES);
}

#[test]
fn embedded_payloads_apply_without_disk_schemas() {
    let bare = Bare::new();
    let schemas = bare.schemas(claudine_schema_bundle());
    let embedded = |name: &str| {
        Some(TriggerSource::Embedded { bundle: CLAUDINE_SCHEMA_BUNDLE_NAME.to_string(), name: name.to_string() })
    };
    let sources: Vec<String> = schemas
        .trigger_registry()
        .expect("registry")
        .triggers
        .iter()
        .map(|trigger| trigger.source.to_string())
        .collect();
    assert_eq!(
        sources,
        ["embedded:claudine/claudine.yaml", "embedded:claudine/inline-compose.yaml", "embedded:claudine/sequence.yaml"]
    );

    // claudine.yaml's payload, with its lifecycle, action, and linking imports.
    let (effective, found, valid) =
        judge_memory(&schemas, "start: [stop, { say: hi }]\ninitialize: [{ info: hi }]\nloop: { gate: [stop] }\nname: x");
    assert!(valid, "{found:?}");
    for property in ["start", "initialize", "loop", "name", "agent", "timeout"] {
        let origin = effective.origins.get(property).unwrap_or_else(|| panic!("{property} has an origin"));
        assert_eq!(origin.trigger_source(), embedded("claudine.yaml"), "{property}");
        assert!(is_claudine_owned(&effective, property), "{property} is a Claudine control");
    }
    assert!(effective.dependencies().is_empty(), "{:?}", effective.dependencies());
    for (frontmatter, property) in [
        ("start: [{ inf0: hi }]", "start"),
        ("initialize: [{ shell: ls }]", "initialize"),
        ("loop: { gate: [{ inf0: hi }] }", "loop"),
        ("start: [stop]\nname: { a: 1 }", "name"),
    ] {
        let (_, found, valid) = judge_memory(&schemas, frontmatter);
        assert!(!valid, "`{frontmatter}` is rejected");
        assert_eq!(top_level(found.into_iter().map(|(pointer, _)| pointer)), BTreeSet::from([property.to_string()]));
    }

    // inline-compose.yaml's payload, applied after claudine.yaml.
    let (effective, found, valid) = judge_memory(&schemas, "prompt: hi\nhash: abc");
    assert!(valid, "{found:?}");
    for property in ["prompt", "hash", "last_updated"] {
        assert_eq!(effective.origins[property].trigger_source(), embedded("inline-compose.yaml"), "{property}");
    }
    assert!(effective.dependencies().is_empty());
    let (_, found, valid) = judge_memory(&schemas, "prompt: hi\nhash: [1]");
    assert!(!valid);
    assert_eq!(top_level(found.into_iter().map(|(pointer, _)| pointer)), BTreeSet::from(["hash".to_string()]));

    // sequence.yaml's payload, with its sequence-types and lifecycle imports:
    // a sequence document with no other control is a Claudine control.
    let (effective, found, valid) = judge_memory(
        &schemas,
        "sequence: [{ name: a, sequence: [{ name: b, group: { tasks: [{ name: c, shell: x }], loop: { until: done } } }] }]",
    );
    assert!(valid, "{found:?}");
    for property in ["sequence", "template", "fail_fast"] {
        assert_eq!(effective.origins[property].trigger_source(), embedded("sequence.yaml"), "{property}");
        assert!(is_claudine_owned(&effective, property), "{property} is a Claudine control");
    }
    assert!(effective.dependencies().is_empty());
    for (frontmatter, property) in [
        ("sequence: [{ name: a, sequence: [{ name: b, sequence: [{ name: c, with: [1] }] }] }]", "sequence"),
        ("sequence: [{ group: { tasks: [{ name: l }], execution: parallel } }]", "sequence"),
        ("sequence: [a]\nfail_fast: true", "fail_fast"),
    ] {
        let (_, found, valid) = judge_memory(&schemas, frontmatter);
        assert!(!valid, "`{frontmatter}` is rejected");
        assert_eq!(top_level(found.into_iter().map(|(pointer, _)| pointer)), BTreeSet::from([property.to_string()]));
    }

    bare.assert_untouched();
}

// ── The input robustness matrix, embedded origin ───────────────────────────

macro_rules! embedded_matrix_tests {
    ($($test:ident => ($column:literal, $syntax:ident);)*) => {
        $(
            mod $test {
                use super::*;

                #[test]
                fn shapes_1_to_4() {
                    walk_from($column, Syntax::$syntax, Half::First, &shipped_bundle!(), Origin::Embedded);
                }

                #[test]
                fn shapes_5_to_8() {
                    walk_from($column, Syntax::$syntax, Half::Second, &shipped_bundle!(), Origin::Embedded);
                }
            }
        )*

        /// Every generated column test, as `(column, syntax)`; each covers
        /// both halves.
        const EMBEDDED_GENERATED: &[(&str, Syntax)] = &[$(($column, Syntax::$syntax)),*];
    };
}

embedded_matrix_tests! {
    embedded_yaml_library_kind => ("library_kind", Yaml);
    embedded_yaml_library_types => ("library_types", Yaml);
    embedded_yaml_action_communicate => ("action_communicate", Yaml);
    embedded_yaml_action_flow => ("action_flow", Yaml);
    embedded_yaml_action_shell => ("action_shell", Yaml);
    embedded_yaml_action_expression => ("action_expression", Yaml);
    embedded_yaml_action_bundle => ("action_bundle", Yaml);
    embedded_yaml_action_initialize_bundle => ("action_initialize_bundle", Yaml);
    embedded_yaml_lifecycle_stack => ("lifecycle_stack", Yaml);
    embedded_yaml_lifecycle_initialize_stack => ("lifecycle_initialize_stack", Yaml);
    embedded_yaml_lifecycle_loop => ("lifecycle_loop", Yaml);
    embedded_yaml_linking_user_invocable => ("linking_user_invocable", Yaml);
    embedded_yaml_trigger_kind => ("trigger_kind", Yaml);
    embedded_yaml_trigger_match => ("trigger_match", Yaml);
    embedded_yaml_trigger_payload => ("trigger_payload", Yaml);
    embedded_yaml_trigger_condition_operand => ("trigger_condition_operand", Yaml);
    embedded_yaml_trigger_combinator => ("trigger_combinator", Yaml);
    embedded_yaml_payload_property => ("payload_property", Yaml);
    embedded_json_library_kind => ("library_kind", Json);
    embedded_json_library_types => ("library_types", Json);
    embedded_json_action_communicate => ("action_communicate", Json);
    embedded_json_action_flow => ("action_flow", Json);
    embedded_json_action_shell => ("action_shell", Json);
    embedded_json_action_expression => ("action_expression", Json);
    embedded_json_action_bundle => ("action_bundle", Json);
    embedded_json_action_initialize_bundle => ("action_initialize_bundle", Json);
    embedded_json_lifecycle_stack => ("lifecycle_stack", Json);
    embedded_json_lifecycle_initialize_stack => ("lifecycle_initialize_stack", Json);
    embedded_json_lifecycle_loop => ("lifecycle_loop", Json);
    embedded_json_linking_user_invocable => ("linking_user_invocable", Json);
    embedded_json_trigger_kind => ("trigger_kind", Json);
    embedded_json_trigger_match => ("trigger_match", Json);
    embedded_json_trigger_payload => ("trigger_payload", Json);
    embedded_json_trigger_condition_operand => ("trigger_condition_operand", Json);
    embedded_json_trigger_combinator => ("trigger_combinator", Json);
    embedded_json_payload_property => ("payload_property", Json);
}

#[test]
fn embedded_yaml_controls() {
    controls_from(Syntax::Yaml, &shipped_bundle!(), Origin::Embedded);
}

#[test]
fn embedded_json_controls() {
    controls_from(Syntax::Json, &shipped_bundle!(), Origin::Embedded);
}

#[test]
fn embedded_matrix_tests_cover_the_table() {
    assert_halves_cover_the_shapes();
    for syntax in [Syntax::Yaml, Syntax::Json] {
        let generated: Vec<&str> = EMBEDDED_GENERATED
            .iter()
            .filter(|(_, generated)| *generated == syntax)
            .map(|(column, _)| *column)
            .collect();
        assert_eq!(generated, column_names(), "one embedded {syntax:?} test per column");
    }
}

/// The R3 cell, embedded: block YAML only, since every JSON key is a string.
#[test]
fn embedded_non_string_top_level_trigger_key_is_an_error() {
    assert_non_string_trigger_key_errors(&shipped_bundle!(), Origin::Embedded);
}

// ── AC13: schema locations and the last-resort rule ────────────────────────

/// The schema roots a fixture can write to, most local first.
#[derive(Clone, Copy, Debug)]
enum Root {
    Package,
    Area,
    Tree,
    SchemasDir,
    Home,
}

/// `{dir}/repo` is a repository (a `.git` skeleton, so no Git process runs)
/// whose Cargo workspace has the package `area/pkg`; documents live in
/// `area/pkg/docs/`. `{dir}/defs` is the folder `SCHEMAS_DIR` names, and
/// `{dir}/home` the fixture home. A root's folder exists only once a test
/// writes a schema into it.
struct Locations {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Locations {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let repo = root.join("repo");
        let write = |path: PathBuf, text: &str| {
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, text).expect("write fixture");
        };
        std::fs::create_dir_all(repo.join(".git/objects")).expect("git objects");
        std::fs::create_dir_all(repo.join(".git/refs/heads")).expect("git refs");
        write(repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        write(repo.join(".git/config"), "[core]\nrepositoryformatversion = 0\nbare = false\n");
        write(repo.join("Cargo.toml"), "[workspace]\nmembers = [\"area/pkg\"]\n");
        write(
            repo.join("area/pkg/Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(repo.join("area/pkg/src/lib.rs"), "");
        std::fs::create_dir_all(repo.join("area/pkg/docs")).expect("docs dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        Self { _dir: dir, root }
    }

    fn folder(&self, root: Root) -> PathBuf {
        match root {
            Root::Package => self.root.join("repo/area/pkg/schemas"),
            Root::Area => self.root.join("repo/area/schemas"),
            Root::Tree => self.root.join("repo/schemas"),
            Root::SchemasDir => self.root.join("defs"),
            Root::Home => self.root.join("home/schemas"),
        }
    }

    fn write_schema(&self, root: Root, name: &str, text: &str) -> PathBuf {
        let folder = self.folder(root);
        std::fs::create_dir_all(&folder).expect("schema root");
        let path = folder.join(name);
        std::fs::write(&path, text).expect("write schema");
        path
    }

    fn write_bundle(&self, root: Root, files: &[BundleFile]) {
        for (name, text) in files {
            self.write_schema(root, name, text);
        }
    }

    fn remove_schema(&self, root: Root, name: &str) {
        std::fs::remove_file(self.folder(root).join(name)).expect("remove schema");
    }

    fn document(&self, name: &str, frontmatter: &str) -> PathBuf {
        let path = self.root.join("repo/area/pkg/docs").join(format!("{name}.md"));
        std::fs::write(&path, format!("---\n{frontmatter}\n---\nbody\n")).expect("write document");
        path
    }

    /// The launch context: the documents' folder, the fixture home, and
    /// `SCHEMAS_DIR` naming `{dir}/defs`.
    fn context(&self) -> FileResolutionContext {
        let env = HashMap::from([("SCHEMAS_DIR".to_string(), self.folder(Root::SchemasDir).display().to_string())]);
        let snapshot = RequestSnapshot::new(self.root.join("repo/area/pkg/docs"))
            .with_home(Some(self.root.join("home")))
            .with_env(env);
        build_resolution_context(&snapshot).expect("fixture context")
    }

    /// Claudine's launch verdict for a document holding `frontmatter`.
    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    fn verdict(&self, name: &str, frontmatter: &str) -> Result<PreValidatedSchema, CompositionError> {
        let path = self.document(name, frontmatter);
        let context = self.context();
        let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context)
            .expect("the document resolves");
        pre_validate_schema(&source, None, None, &context)
    }

    /// The trigger discovery the launch verdict uses, from the document
    /// folder's own context.
    fn schemas(&self) -> DarkmatterSchemas {
        let document = self.document("discovery", "title: x");
        DarkmatterSchemas::new(self.context().for_source(&document))
            .with_embedded_trigger_bundle(claudine_schema_bundle())
            .and_then(DarkmatterSchemas::with_trigger_discovery)
            .expect("discovery succeeds")
    }

    /// The effective schema of a document holding `frontmatter`.
    fn effective(&self, schemas: &DarkmatterSchemas, name: &str, frontmatter: &str) -> EffectiveSchema {
        let path = self.document(name, frontmatter);
        let markdown =
            Markdown::try_from(path.as_path()).expect("document loads").with_source(ComposeSource::File(path.clone()));
        schemas.effective_for(&markdown).expect("resolves").expect("a schema applies")
    }

    /// The verdict of the same discovery the launch verdict uses (no
    /// composition), for many probes over one discovery.
    fn passive(&self, schemas: &DarkmatterSchemas, name: &str, frontmatter: &str) -> BTreeSet<String> {
        let path = self.document(name, frontmatter);
        let markdown =
            Markdown::try_from(path.as_path()).expect("document loads").with_source(ComposeSource::File(path.clone()));
        let report = schemas.validate(&markdown).expect("validation");
        top_level(report.problems.into_iter().map(|problem| problem.path))
    }
}

/// A Claudine trigger that types `tier` as the one value `tier`, selected by
/// any `start` list.
fn tier_trigger(tier: &str) -> String {
    format!("kind: trigger-schema\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    tier: enum({tier})\n")
}

/// A same-name `claudine.yaml` that types `model` as a number.
const NUMBER_MODEL_TRIGGER: &str =
    "kind: trigger-schema\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    model: number\n";

fn is_load_failure(error: &CompositionError) -> bool {
    matches!(error, CompositionError::SchemaLoad { .. } | CompositionError::SchemaParse { .. })
}

/// The nearest root holding `claudine.yaml` wins over every farther one, and
/// with no disk copy left the embedded one applies.
#[test]
fn nearest_claudine_trigger_shadows_farther_ones() {
    let locations = Locations::new();
    let order = [
        (Root::Package, "package"),
        (Root::Area, "area"),
        (Root::Tree, "tree"),
        (Root::SchemasDir, "defs"),
        (Root::Home, "home"),
    ];
    for (root, tier) in order {
        locations.write_schema(root, "claudine.yaml", &tier_trigger(tier));
    }
    let document = locations.document("roots", "title: x");
    let roots = SchemaRoots::for_document(&locations.context().for_source(&document)).expect("schema roots");
    let kinds: Vec<SchemaRootKind> = roots.searched().iter().map(|root| root.kind).collect();
    assert_eq!(
        kinds,
        [SchemaRootKind::Package, SchemaRootKind::PackageArea, SchemaRootKind::Tree, SchemaRootKind::SchemasDir, SchemaRootKind::Home],
        "the fixture has all five roots"
    );

    for (index, (root, tier)) in order.iter().enumerate() {
        assert_accepts(locations.verdict("tier", &format!("start: [stop]\ntier: {tier}")), &format!("{root:?} wins"));
        if let Some((_, farther)) = order.get(index + 1) {
            assert_rejects_at(
                locations.verdict("tier", &format!("start: [stop]\ntier: {farther}")),
                "tier",
                &format!("{root:?} shadows the farther `{farther}`"),
            );
        }
        locations.remove_schema(*root, "claudine.yaml");
    }
    // No disk copy is left: the embedded claudine.yaml applies.
    assert_accepts(locations.verdict("tier", "start: [stop]\ntier: anything"), "the embedded payload leaves `tier` open");
    assert_rejects_at(locations.verdict("tier", "start: [{ inf0: hi }]"), "start", "the embedded payload types `start`");
}

/// A symlinked `claudine.yaml` in a root is not a root file: it claims no
/// name, so the embedded copy applies.
#[cfg(unix)]
#[test]
fn symlinked_trigger_file_in_a_root_is_ignored() {
    let locations = Locations::new();
    let real = locations.write_schema(Root::Tree, "claudine.yaml", &tier_trigger("linked"));
    assert_rejects_at(locations.verdict("link", "start: [stop]\ntier: other"), "tier", "control: the real file applies");

    let elsewhere = locations.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("elsewhere");
    let target = elsewhere.join("claudine.yaml");
    std::fs::rename(&real, &target).expect("move the trigger out of the root");
    std::os::unix::fs::symlink(&target, &real).expect("symlink");

    assert_accepts(locations.verdict("link", "start: [stop]\ntier: other"), "the symlinked trigger is not read");
    assert_rejects_at(
        locations.verdict("link", "start: [{ inf0: hi }]"),
        "start",
        "the symlink claims no name, so the embedded claudine.yaml applies",
    );
}

/// A disk bundle resolves every relative import from disk: a disk-only
/// spelling in `lifecycle.yaml` reaches `start` through `claudine.yaml`, and
/// the action and linking imports type their properties.
#[test]
fn disk_bundle_resolves_its_relative_imports_from_disk() {
    let locations = Locations::new();
    // Without a disk bundle the spelling is unknown.
    assert_rejects_at(locations.verdict("disk", "start: [disk-only]"), "start", "embedded: `disk-only` is no verb");

    let mut bundle: Vec<BundleFile> = shipped_bundle!().to_vec();
    let lifecycle = &mut bundle.iter_mut().find(|(name, _)| *name == "lifecycle.yaml").expect("lifecycle").1;
    let edited = lifecycle.replacen("\"enum(stop, skip,", "\"enum(stop, disk-only, skip,", 1);
    assert_ne!(&edited, lifecycle, "the bare-verb enum was edited");
    *lifecycle = edited;
    locations.write_bundle(Root::Tree, &bundle);

    assert_accepts(locations.verdict("disk", "start: [disk-only]"), "disk: the edited lifecycle.yaml is read");
    let schemas = locations.schemas();
    for (frontmatter, property) in [
        ("start: [{ inf0: hi }]", "start"),
        ("initialize: [{ shell: ls }]", "initialize"),
        ("loop: { gate: [{ inf0: hi }] }", "loop"),
        ("start: [stop]\nname: { a: 1 }", "name"),
        ("prompt: hi\nhash: [1]", "hash"),
    ] {
        assert_eq!(locations.passive(&schemas, "probe", frontmatter), BTreeSet::from([property.to_string()]), "{frontmatter}");
    }
    let effective = locations.effective(&schemas, "origin", "start: [disk-only]");
    assert_eq!(
        effective.origins["start"].trigger_source(),
        Some(TriggerSource::File(locations.folder(Root::Tree).join("claudine.yaml")))
    );
    assert!(is_claudine_owned(&effective, "start"));
    assert!(!effective.dependencies().is_empty(), "disk payload files are dependencies");
}

/// A trigger envelope is not a schema: naming one in `$schema` fails, and an
/// embedded trigger is no bare name at all.
#[test]
fn schema_reference_to_a_trigger_envelope_fails() {
    let locations = Locations::new();
    locations.write_schema(Root::Tree, "claudine.yaml", &tier_trigger("x"));
    for frontmatter in ["$schema: claudine.yaml\ntitle: x", "$schema: inline-compose.yaml\ntitle: x"] {
        let error = locations.verdict("reference", frontmatter).expect_err(frontmatter);
        assert!(is_load_failure(&error), "{frontmatter}: {error:?}");
    }
}

/// A same-name disk `claudine.yaml` replaces the embedded one: its `model`
/// type applies, and the embedded `start` type no longer does.
#[test]
fn same_name_disk_trigger_wins_over_the_embedded_copy() {
    let locations = Locations::new();
    assert_accepts(locations.verdict("model", "start: [stop]\nmodel: claude"), "embedded: `model` is a string");

    locations.write_schema(Root::Tree, "claudine.yaml", NUMBER_MODEL_TRIGGER);
    assert_rejects_at(locations.verdict("model", "start: [stop]\nmodel: claude"), "model", "disk: `model` is a number");
    assert_accepts(locations.verdict("model", "start: [{ inf0: hi }]\nmodel: 5"), "disk: `start` is untyped");
}

/// With no disk bundle, the embedded trigger applies and owns its controls.
#[test]
fn embedded_trigger_applies_with_no_disk_bundle() {
    let locations = Locations::new();
    assert_rejects_at(locations.verdict("embedded", "start: [{ inf0: hi }]"), "start", "embedded payload");
    let effective = locations.effective(&locations.schemas(), "embedded", "start: [stop]");
    assert_eq!(
        effective.origins["start"].trigger_source(),
        Some(TriggerSource::Embedded { bundle: CLAUDINE_SCHEMA_BUNDLE_NAME.to_string(), name: "claudine.yaml".to_string() })
    );
    assert!(is_claudine_owned(&effective, "start"));
}

/// An authored `$schema` applies last: its `model` replaces Claudine's, is
/// no longer a Claudine control, and an invalid value is cleaned by the
/// authored rule (dropped with a warning) rather than failing the launch.
#[test]
fn authored_schema_override_applies_last_and_takes_ownership() {
    let locations = Locations::new();
    let frontmatter = "$schema:\n    model: number\nstart: [stop]\nmodel: claude";
    let validated = locations.verdict("override", frontmatter).expect("the invalid authored optional is dropped");
    let dropped: Vec<&str> = validated.dropped_optionals.iter().map(|dropped| dropped.property.as_str()).collect();
    assert_eq!(dropped, ["model"]);
    assert!(!validated.source.markdown.frontmatter().as_map().contains_key("model"));

    let effective = locations.effective(&locations.schemas(), "override", frontmatter);
    assert_eq!(effective.origins["model"].kind, SchemaOriginKind::Document);
    assert!(!is_claudine_owned(&effective, "model"), "the author owns `model`");
    assert!(is_claudine_owned(&effective, "start"), "Claudine still owns `start`");
}

/// A malformed disk `claudine.yaml` fails every launch verdict; it never
/// falls back to the embedded copy.
#[test]
fn malformed_disk_trigger_fails_without_embedded_fallback() {
    let locations = Locations::new();
    locations.write_schema(Root::Tree, "claudine.yaml", "kind: trigger-schema\nmatch: 42\n$schema: {}\n");
    for frontmatter in ["start: [stop]", "title: x"] {
        match locations.verdict("malformed", frontmatter) {
            Err(CompositionError::SchemaLoad { .. }) => {}
            other => panic!("`{frontmatter}`: expected SchemaLoad, got {other:?}"),
        }
    }
}

/// A disk trigger's import never resolves to the embedded file of the same
/// basename.
#[test]
fn disk_import_never_falls_back_to_an_embedded_file() {
    let locations = Locations::new();
    locations.write_schema(
        Root::Tree,
        "claudine.yaml",
        "kind: trigger-schema\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    start: stack@./lifecycle.yaml\n",
    );
    let error = locations.verdict("import", "start: [stop]").expect_err("the disk import is missing");
    assert!(is_load_failure(&error), "{error:?}");
}

/// A disk `claudine.yaml` that matches nothing still shadows the embedded
/// one, while the embedded inline-compose.yaml still applies.
#[test]
fn non_matching_disk_trigger_still_shadows_the_embedded_one() {
    let locations = Locations::new();
    locations.write_schema(
        Root::Tree,
        "claudine.yaml",
        "kind: trigger-schema\nmatch:\n    never_set: string(required)\n$schema:\n    never_set: string\n",
    );
    assert_accepts(locations.verdict("shadow", "start: [{ inf0: hi }]"), "no Claudine payload applies");
    assert_accepts(locations.verdict("shadow", "prompt: hi\nstart: [{ inf0: hi }]"), "inline-compose does not type `start`");
    assert_rejects_at(locations.verdict("shadow", "prompt: hi\nhash: [1]"), "hash", "embedded inline-compose applies");
}

/// A winning disk `claudine.yaml` and the embedded inline-compose.yaml apply
/// together, and both own their properties.
#[test]
fn disk_claudine_and_embedded_inline_compose_apply_together() {
    let locations = Locations::new();
    locations.write_schema(Root::Tree, "claudine.yaml", NUMBER_MODEL_TRIGGER);
    let effective = locations.effective(&locations.schemas(), "both", "prompt: hi\nstart: [stop]\nmodel: 5\nhash: abc");
    assert_eq!(
        effective.origins["model"].trigger_source(),
        Some(TriggerSource::File(locations.folder(Root::Tree).join("claudine.yaml")))
    );
    assert_eq!(
        effective.origins["hash"].trigger_source(),
        Some(TriggerSource::Embedded { bundle: CLAUDINE_SCHEMA_BUNDLE_NAME.to_string(), name: "inline-compose.yaml".to_string() })
    );
    for property in ["model", "prompt", "hash", "last_updated"] {
        assert!(is_claudine_owned(&effective, property), "{property} is a Claudine control");
    }
    assert_rejects_at(locations.verdict("both", "prompt: hi\nstart: [stop]\nmodel: claude"), "model", "disk payload");
    assert_rejects_at(locations.verdict("both", "prompt: hi\nstart: [stop]\nhash: [1]"), "hash", "embedded payload");
}

/// The same bundle on disk and embedded classifies the same properties as
/// Claudine controls; only the origin's source differs.
#[test]
fn disk_and_embedded_bundles_own_the_same_properties() {
    let frontmatter = "$schema:\n    model: number\nprompt: hi\nstart: [stop]";
    let owned = |effective: &EffectiveSchema| -> BTreeSet<String> {
        effective.origins.keys().filter(|property| is_claudine_owned(effective, property)).cloned().collect()
    };

    let embedded_only = Locations::new();
    let embedded = embedded_only.effective(&embedded_only.schemas(), "owned", frontmatter);
    let on_disk = Locations::new();
    on_disk.write_bundle(Root::Tree, &shipped_bundle!());
    let disk = on_disk.effective(&on_disk.schemas(), "owned", frontmatter);

    let embedded_owned = owned(&embedded);
    assert!(embedded_owned.contains("start") && embedded_owned.contains("hash"), "{embedded_owned:?}");
    assert!(!embedded_owned.contains("model"), "the authored `model` is not Claudine's");
    assert_eq!(embedded_owned, owned(&disk));
    assert!(matches!(embedded.origins["start"].trigger_source(), Some(TriggerSource::Embedded { .. })));
    assert!(matches!(disk.origins["start"].trigger_source(), Some(TriggerSource::File(_))));
}

// ── The strict policy at the library boundary ──────────────────────────────

/// An invalid optional Claudine control in a document with no `$schema`
/// fails the launch verdict at its pointer and is never dropped.
#[test]
fn invalid_claudine_controls_fail_the_launch_and_are_not_dropped() {
    let locations = Locations::new();
    let context = locations.context();
    for (frontmatter, property) in [
        ("agent: []", "agent"),
        ("start: [stop]\ntimeout: [1]", "timeout"),
        ("start: [{ say: { txet: x } }]", "start"),
    ] {
        assert_rejects_at(locations.verdict("strict", frontmatter), property, frontmatter);

        let path = locations.document("strict", frontmatter);
        let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context).expect("resolves");
        let (cleaned, _, dropped) = drop_invalid_optionals(source, None, None, &context);
        assert!(dropped.is_empty(), "{frontmatter}: dropped {dropped:?}");
        assert!(cleaned.markdown.frontmatter().as_map().contains_key(property), "{frontmatter}: `{property}` is kept");
    }
}

/// An unrelated authored optional keeps its drop-and-warn cleanup beside a
/// Claudine control, whose own invalid value still fails the launch.
#[test]
fn authored_optional_is_dropped_while_a_claudine_control_fails() {
    let locations = Locations::new();
    let context = locations.context();
    let authored = "$schema:\n    count: number\nstart: [stop]\ncount: [1]";
    let validated = locations.verdict("cleanup", authored).expect("the authored optional is dropped");
    let dropped: Vec<&str> = validated.dropped_optionals.iter().map(|dropped| dropped.property.as_str()).collect();
    assert_eq!(dropped, ["count"]);

    let both = format!("{authored}\ntimeout: [1]");
    let found = rejected(locations.verdict("cleanup", &both), "the invalid `timeout`");
    assert_eq!(top_level(found), BTreeSet::from(["timeout".to_string()]), "only the Claudine control is reported");

    let path = locations.document("cleanup", &both);
    let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context).expect("resolves");
    let (cleaned, _, dropped) = drop_invalid_optionals(source, None, None, &context);
    let dropped: Vec<&str> = dropped.iter().map(|dropped| dropped.property.as_str()).collect();
    assert_eq!(dropped, ["count"]);
    let kept = cleaned.markdown.frontmatter().as_map();
    assert!(!kept.contains_key("count") && kept.contains_key("timeout"));
}

/// A retired lifecycle spelling is the parser's `RemovedGrammar`, which
/// names the replacement, not the schema mismatch it also is.
#[test]
fn retired_spelling_reports_removed_grammar() {
    let locations = Locations::new();
    match locations.verdict("retired", "start:\n    stack:\n        - info: hi") {
        Err(CompositionError::RemovedGrammar { path, removed, .. }) => {
            assert_eq!((path.as_str(), removed.as_str()), ("start.stack", "stack"));
        }
        other => panic!("expected RemovedGrammar, got {other:?}"),
    }
}

// ── AC10: passive parity across the launch verdict and DMLS ────────────────

/// For one fixture, Claudine's launch verdict over a disk bundle and
/// Darkmatter's DMLS path (`with_trigger_discovery` → `effective_for` →
/// `validate`, unphased) see the same sources and the same frontmatter: the
/// launch verdict passes exactly when DMLS reports the document valid, and a
/// rejection names the same top-level properties. Both judge the passive
/// snapshot; composition coercion is not part of either.
fn assert_passive_parity(name: &str, text: &str) {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let path = repo.write_document(&format!("{name}.md"), text);
    let editor = discovered
        .judge(&path, include_str!("../../../schemas/claudine.yaml"), include_str!("../../../schemas/inline-compose.yaml"))
        .expect("judged")
        .report;
    let editor_properties = top_level(editor.problems.iter().map(|problem| problem.path.clone()));

    let context = repo.context();
    let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context).expect("resolves");
    match (pre_validate_schema(&source, None, None, &context), editor.valid) {
        (Ok(validated), true) => assert!(validated.dropped_optionals.is_empty(), "{name}: {:?}", validated.dropped_optionals),
        (Err(CompositionError::SchemaValidation { problems, .. }), false) => {
            assert_eq!(top_level(problems), editor_properties, "{name}: the launch verdict and DMLS name the same properties");
        }
        (launch, editor_valid) => panic!("{name}: DMLS valid = {editor_valid}, launch verdict = {launch:?}"),
    }
}

macro_rules! passive_parity_tests {
    ($($test:ident => ($name:literal, $text:expr);)*) => {
        $(
            #[test]
            fn $test() {
                assert_passive_parity($name, $text);
            }
        )*

        /// Every generated test's fixture.
        const PARITY_FIXTURES: &[&str] = &[$($name),*];
    };
}

passive_parity_tests! {
    passive_parity_depth1 => ("depth1", include_str!("../fixtures/claudine-schemas/depth1.md"));
    passive_parity_depth3 => ("depth3", include_str!("../fixtures/claudine-schemas/depth3.md"));
    passive_parity_depth5 => ("depth5", include_str!("../fixtures/claudine-schemas/depth5.md"));
    passive_parity_depth6 => ("depth6", include_str!("../fixtures/claudine-schemas/depth6.md"));
    passive_parity_root_dictionary => ("root-dictionary", include_str!("../fixtures/claudine-schemas/root-dictionary.md"));
    passive_parity_bundle_in_then => ("bundle-in-then", include_str!("../fixtures/claudine-schemas/bundle-in-then.md"));
    passive_parity_loop_gate => ("loop-gate", include_str!("../fixtures/claudine-schemas/loop-gate.md"));
    passive_parity_long_forms => ("long-forms", include_str!("../fixtures/claudine-schemas/long-forms.md"));
    passive_parity_bare_stop => ("bare-stop", include_str!("../fixtures/claudine-schemas/bare-stop.md"));
    passive_parity_shell_in_initialize_list =>
        ("shell-in-initialize-list", include_str!("../fixtures/claudine-schemas/shell-in-initialize-list.md"));
    passive_parity_shell_in_initialize_then =>
        ("shell-in-initialize-then", include_str!("../fixtures/claudine-schemas/shell-in-initialize-then.md"));
    passive_parity_shell_in_initialize_root_dictionary => (
        "shell-in-initialize-root-dictionary",
        include_str!("../fixtures/claudine-schemas/shell-in-initialize-root-dictionary.md")
    );
    passive_parity_shell_in_initialize_below_typed_levels => (
        "shell-in-initialize-below-typed-levels",
        include_str!("../fixtures/claudine-schemas/shell-in-initialize-below-typed-levels.md")
    );
    passive_parity_shell_expansion_in_initialize_set => (
        "shell-expansion-in-initialize-set",
        include_str!("../fixtures/claudine-schemas/shell-expansion-in-initialize-set.md")
    );
    passive_parity_shell_in_start => ("shell-in-start", include_str!("../fixtures/claudine-schemas/shell-in-start.md"));
    passive_parity_message_shell_bundle_with_when =>
        ("message-shell-bundle-with-when", include_str!("../fixtures/claudine-schemas/message-shell-bundle-with-when.md"));
    passive_parity_when_with_then_and_sibling =>
        ("when-with-then-and-sibling", include_str!("../fixtures/claudine-schemas/when-with-then-and-sibling.md"));
    passive_parity_misspelled_parameter =>
        ("misspelled-parameter", include_str!("../fixtures/claudine-schemas/misspelled-parameter.md"));
    passive_parity_misspelled_verb => ("misspelled-verb", include_str!("../fixtures/claudine-schemas/misspelled-verb.md"));
    passive_parity_misspelled_verb_in_then =>
        ("misspelled-verb-in-then", include_str!("../fixtures/claudine-schemas/misspelled-verb-in-then.md"));
    passive_parity_long_form_in_bundle =>
        ("long-form-in-bundle", include_str!("../fixtures/claudine-schemas/long-form-in-bundle.md"));
    passive_parity_long_form_in_root_dictionary =>
        ("long-form-in-root-dictionary", include_str!("../fixtures/claudine-schemas/long-form-in-root-dictionary.md"));
    passive_parity_generated_verb_arbitrary_value =>
        ("generated-verb-arbitrary-value", include_str!("../fixtures/claudine-schemas/generated-verb-arbitrary-value.md"));
    passive_parity_start_number => ("start-number", include_str!("../fixtures/claudine-schemas/start-number.md"));
    passive_parity_agent_empty_list => ("agent-empty-list", include_str!("../fixtures/claudine-schemas/agent-empty-list.md"));
    passive_parity_loop_number => ("loop-number", include_str!("../fixtures/claudine-schemas/loop-number.md"));
    passive_parity_empty_frontmatter =>
        ("empty-frontmatter", include_str!("../fixtures/claudine-schemas/empty-frontmatter.md"));
    passive_parity_unrelated_ticket => ("unrelated-ticket", include_str!("../fixtures/claudine-schemas/unrelated-ticket.md"));
    passive_parity_null_identifiers => ("null-identifiers", include_str!("../fixtures/claudine-schemas/null-identifiers.md"));
    passive_parity_controls_beside_ticket =>
        ("controls-beside-ticket", include_str!("../fixtures/claudine-schemas/controls-beside-ticket.md"));
    passive_parity_prompt_with_malformed_sequence =>
        ("prompt-with-malformed-sequence", include_str!("../fixtures/claudine-schemas/prompt-with-malformed-sequence.md"));
    passive_parity_linked_prompt => ("linked-prompt", include_str!("../fixtures/claudine-schemas/linked-prompt.md"));
}

#[test]
fn passive_parity_tests_cover_every_ac5_fixture() {
    let ac5: Vec<&str> = AC5_GROUPS.iter().flat_map(|group| group.iter().copied()).collect();
    assert_eq!(PARITY_FIXTURES, &ac5[..], "one parity test per AC5 fixture, in table order");
}
