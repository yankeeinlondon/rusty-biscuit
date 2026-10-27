//! Claudine's embedded schema bundle and the launch verdict that uses it
//! (`2026-10-21-claudine-schemas`, criteria 6, 10, and 13).
//!
//! - The embed: `CLAUDINE_SCHEMA_FILES` is the eight repository files byte for
//!   byte, each parses as its kind, and every export and every payload resolves
//!   through the embedded seam with no disk schema anywhere.
//! - The input robustness matrix of `claudine_schemas_matrix.rs`, walked with
//!   each edited bundle supplied as an embedded bundle.
//! - Schema locations and the last-resort rule, through Claudine's public
//!   launch verdict (`pre_validate_schema`) and the effective schema it judges
//!   by.
//! - Document metadata from `document.yaml`, judged by passive validation and
//!   the launch verdict through each metadata trigger alone, their merge, a
//!   disk copy of `inline-compose.yaml`, and one `hash` definition shared by
//!   two properties; the longhand `kind` default is declared, never inserted.
//! - The strict policy for Claudine controls at the library boundary.
//! - Passive parity: for the AC5 fixtures, the launch verdict and the DMLS
//!   path agree.
//! - Each failing use of a lifecycle helper reached from several places keeps
//!   its problem code, instance path, position, and declaring trigger.
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
use darkmatter::markdown::schemas::simplified::{DefKey, DefNamespace};
use darkmatter::markdown::schemas::triggers::parse_trigger_envelope_from_str;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, EmbeddedSchemaBundle, SchemaOriginKind, SchemaRootKind, SchemaRoots,
    StandaloneSchemaEnvelope, TriggerSource, parse_standalone_schema_document,
};

use crate::claudine_schemas::{AC5_GROUPS, BundleFile, SchemaRepo, problems, shipped_bundle};
use crate::claudine_schemas_matrix::{Origin, Shape, Syntax, assert_cells_are_the_shapes, assert_non_string_trigger_key_errors, column_names};

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
    let repository: [(&str, &str); 8] = [
        ("action.yaml", include_str!("../../../schemas/action.yaml")),
        ("claudine.yaml", include_str!("../../../schemas/claudine.yaml")),
        ("document.yaml", include_str!("../../../schemas/document.yaml")),
        ("inline-compose.yaml", include_str!("../../../schemas/inline-compose.yaml")),
        ("lifecycle.yaml", include_str!("../../../schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../schemas/linking.yaml")),
        ("sequence-types.yaml", include_str!("../../../schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../schemas/sequence.yaml")),
    ];
    assert_eq!(CLAUDINE_SCHEMA_FILES, &repository[..], "the embed is the eight repository files, unchanged");
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
    assert_eq!((CLAUDINE_SCHEMA_FILES.len(), triggers.len()), (8, 3), "eight trigger and library files, three of them triggers");
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
    ("stack", "[stop, { info: hi }]", "[{ inf0: hi }]"),
    ("item", "stop", "{ inf0: hi }"),
    ("conditional", "{ when: \"ctx.x\", then: [stop] }", "{ when: \"ctx.x\" }"),
    ("initialize-stack", "[{ info: hi }]", "[{ shell: ls }]"),
    ("initialize-item", "{ info: hi }", "{ shell: ls }"),
    ("initialize-conditional", "{ when: \"ctx.x\", then: [stop] }", "{ when: \"ctx.x\", then: [{ shell: ls }] }"),
    ("loop-action", "{ op: increment, prop: n }", "{ op: jump, prop: n }"),
    ("loop-action-entry", "increment(n)", "{ op: increment }"),
    ("loop", "{ while: \"ctx.x\", max: 3 }", "{ max: 0 }"),
    ("cancelled-stack", "[stop, { info: hi }]", "[retry]"),
    ("cancelled-item", "{ shell: ls }", "{ retry: 2 }"),
    ("cancelled-conditional", "{ when: \"ctx.x\", then: [stop] }", "{ when: \"ctx.x\", else: [stop] }"),
];

/// Every export of the recursive container family. Each container type is a
/// definition referenced at every depth, so a nested probe costs no more to
/// resolve than a flat one.
const SEQUENCE_TYPES_PROBES: &[ExportProbe] = &[
    ("steps", "[a, { name: b, shell: x }]", "{ a: 1 }"),
    ("step", "a", "[a]"),
    ("task", "{ name: a, prompt: a.md, with: { x: 1 }, topic: data }", "{ name: a, with: [1] }"),
    ("group", "{ tasks: [{ name: a, shell: x }], max_parallel: 2 }", "{ tasks: [{ name: a }], execution: parallel }"),
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

/// The longhand hash's `value` is `unknown`, so any data is accepted there.
const DOCUMENT_PROBES: &[ExportProbe] = &[
    ("prompt", "hi", "{ a: 1 }"),
    ("hash", "{ kind: detailed, value: [1, { a: b }], ignored: [x] }", "{ kind: bogus }"),
    ("last_updated", "2026-10-26", "[2026-10-26]"),
];

/// A test trigger, added to a copy of the embedded bundle, whose payload
/// imports every export of `library` as `t_{export}`. Its imports resolve
/// inside the bundle exactly as the shipped payloads' do.
fn exports_trigger(library: &str, probes: &[ExportProbe]) -> String {
    let marker = library.trim_end_matches(".yaml");
    let mut text = format!("kind: schema-trigger\nmatch:\n    probe_{marker}: string(required)\n$schema:\n");
    for (export, _, _) in probes {
        text.push_str(&format!("    t_{export}: \"{export}@./{library}\"\n"));
    }
    text
}

/// Resolves and judges the supplied exports through the embedded seam with no
/// disk schemas.
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
fn embedded_document_exports_resolve_without_disk_schemas() {
    assert_exports_resolve_embedded("document.yaml", DOCUMENT_PROBES);
}

macro_rules! sequence_export_tests {
    ($($test:ident => $export:literal),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                let probe = SEQUENCE_TYPES_PROBES
                    .iter()
                    .find(|probe| probe.0 == $export)
                    .expect("the export has positive and negative probes");
                assert_exports_resolve_embedded(
                    "sequence-types.yaml",
                    std::slice::from_ref(probe),
                );
            }
        )*

        #[test]
        fn every_sequence_export_probe_has_one_scheduled_test() {
            let scheduled = [$($export),*];
            let probes: Vec<_> = SEQUENCE_TYPES_PROBES.iter().map(|probe| probe.0).collect();
            assert_eq!(scheduled.as_slice(), probes.as_slice());
        }
    };
}

sequence_export_tests! {
    embedded_sequence_types_exports_resolve_without_disk_schemas => "steps",
    embedded_sequence_step_export_resolves_without_disk_schemas => "step",
    embedded_sequence_task_export_resolves_without_disk_schemas => "task",
    embedded_sequence_group_export_resolves_without_disk_schemas => "group",
}

/// The resolved size limit for Claudine's embedded trigger payloads: the
/// compact `serde_json` text of the resolved JSON Schema, `$defs` included,
/// as `recursive_schema_frozen_fixture.rs` measures it in Darkmatter.
const PAYLOAD_LIMIT_BYTES: usize = 2_000_000;

/// The recursive event families, each stored once as a definition.
const EVENT_FAMILIES: &[(&str, &str)] = &[
    ("lifecycle.yaml", "stack"),
    ("lifecycle.yaml", "item"),
    ("lifecycle.yaml", "conditional"),
    ("lifecycle.yaml", "initialize-stack"),
    ("lifecycle.yaml", "initialize-item"),
    ("lifecycle.yaml", "initialize-conditional"),
    ("lifecycle.yaml", "cancelled-stack"),
    ("lifecycle.yaml", "cancelled-item"),
    ("lifecycle.yaml", "cancelled-conditional"),
];

/// The container families and every event family their events reach.
const CONTAINER_FAMILIES: &[(&str, &str)] = &[
    ("sequence-types.yaml", "steps"),
    ("sequence-types.yaml", "step"),
    ("sequence-types.yaml", "task"),
    ("sequence-types.yaml", "group"),
    ("lifecycle.yaml", "stack"),
    ("lifecycle.yaml", "initialize-stack"),
    ("lifecycle.yaml", "cancelled-stack"),
];

/// Names of the retired unrolled and loose types, by their authored file.
const UNROLLED_TYPE_NAMES: &[(&str, &str)] = &[
    ("lifecycle.yaml", "loose-body"),
    ("lifecycle.yaml", "item-1"),
    ("lifecycle.yaml", "stack-2"),
    ("lifecycle.yaml", "conditional-3"),
    ("lifecycle.yaml", "initialize-item-1"),
    ("lifecycle.yaml", "initialize-stack-3"),
    ("lifecycle.yaml", "cancelled-item-1"),
    ("lifecycle.yaml", "cancelled-stack-6"),
    ("sequence-types.yaml", "steps-2"),
    ("sequence-types.yaml", "step-2"),
    ("sequence-types.yaml", "task-2"),
    ("sequence-types.yaml", "group-3"),
];

/// Whether `name`, declared by the bundle file `file`, is a definition of
/// `effective`, looked up by its `DefKey` (never by a `$defs` key).
fn embedded_definition(effective: &EffectiveSchema, file: &str, name: &str) -> bool {
    let namespace = DefNamespace::Embedded(format!("{CLAUDINE_SCHEMA_BUNDLE_NAME}/{file}"));
    effective.definition(&DefKey::new(namespace, name)).is_some()
}

/// The recursive families are stored once as definitions and the container
/// types reach them at every depth, so each trigger's resolved schema stays
/// under the limit. The measured schema is the effective one, which also
/// holds the built-in baseline, so the payload alone is smaller still.
#[test]
fn embedded_trigger_payloads_resolve_within_two_megabytes() {
    let bare = Bare::new();
    let schemas = bare.schemas(claudine_schema_bundle());
    let mut report = Vec::new();
    for (label, frontmatter, families) in [
        ("claudine.yaml", "start: []", EVENT_FAMILIES),
        ("sequence.yaml", "sequence: []", CONTAINER_FAMILIES),
        ("both", "start: []\nsequence: []", CONTAINER_FAMILIES),
    ] {
        let markdown = memory_document(frontmatter);
        let effective = schemas.effective_for(&markdown).expect("resolves").expect("a trigger applies");
        let bytes = serde_json::to_string(&*effective.json_schema).expect("the schema serializes").len();
        report.push(format!("{label}: {bytes} B"));
        assert!(bytes <= PAYLOAD_LIMIT_BYTES, "{label} resolves to {bytes} bytes, over {PAYLOAD_LIMIT_BYTES}");
        assert!(
            effective.json_schema.get("$defs").and_then(|defs| defs.as_object()).is_some_and(|defs| !defs.is_empty()),
            "{label}: the recursive families are local `$defs`"
        );
        for (file, name) in families {
            assert!(embedded_definition(&effective, file, name), "{label}: `{name}@./{file}` is a definition");
        }
        for (file, name) in UNROLLED_TYPE_NAMES {
            assert!(!embedded_definition(&effective, file, name), "{label}: `{name}@./{file}` must not remain");
        }
    }
    eprintln!("{}", report.join("\n"));
    bare.assert_untouched();
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
                $crate::claudine_schemas_matrix::matrix_cells!($column, $syntax, Embedded);
            }
        )*

        /// Every generated module, as `(column, syntax)`.
        const EMBEDDED_GENERATED: &[(&str, Syntax)] = &[$(($column, Syntax::$syntax)),*];

        /// Every generated module's shape cells.
        const EMBEDDED_GENERATED_CELLS: &[&[Shape]] = &[$($test::CELLS),*];
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
fn embedded_matrix_tests_cover_the_table() {
    for syntax in [Syntax::Yaml, Syntax::Json] {
        let generated: Vec<&str> = EMBEDDED_GENERATED
            .iter()
            .filter(|(_, generated)| *generated == syntax)
            .map(|(column, _)| *column)
            .collect();
        assert_eq!(generated, column_names(), "one embedded {syntax:?} module per column");
    }
    assert_cells_are_the_shapes(EMBEDDED_GENERATED_CELLS);
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
        self.verdict_recording(name, frontmatter, &claudine::composition::TriggerFindings::default())
    }

    /// [`Self::verdict`], recording the skipped triggers in `findings`.
    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    fn verdict_recording(
        &self,
        name: &str,
        frontmatter: &str,
        findings: &claudine::composition::TriggerFindings,
    ) -> Result<PreValidatedSchema, CompositionError> {
        let path = self.document(name, frontmatter);
        let context = self.context();
        let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context)
            .expect("the document resolves");
        pre_validate_schema(&source, None, None, &context, findings)
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
    format!("kind: schema-trigger\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    tier: enum({tier})\n")
}

/// A same-name `claudine.yaml` that types `model` as a number.
const NUMBER_MODEL_TRIGGER: &str =
    "kind: schema-trigger\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    model: number\n";

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

/// The disk `claudine.yaml` discovery skipped, with no Claudine payload
/// applied: the skipped file still shadows the embedded copy.
fn assert_skipped_without_embedded_fallback(locations: &Locations, name: &str) {
    let schemas = locations.schemas();
    let skipped: Vec<_> = schemas
        .discovery_findings()
        .iter()
        .filter(|finding| finding.source().path().is_some_and(|path| path.ends_with("claudine.yaml")))
        .collect();
    assert_eq!(skipped.len(), 1, "{:?}", schemas.discovery_findings());
    let findings = claudine::composition::TriggerFindings::default();
    assert_accepts(locations.verdict_recording(name, "start: [{ inf0: hi }]", &findings), "no Claudine payload applies");
    let recorded: Vec<_> = findings.warnings().into_iter().map(|warning| warning.source).collect();
    assert_eq!(recorded, [skipped[0].identity().clone()], "the launch verdict carries the skipped trigger");
    assert_accepts(locations.verdict(name, "title: x"), "an unrelated document launches");
}

/// A malformed disk `claudine.yaml` is skipped for every launch verdict; it
/// never falls back to the embedded copy.
#[test]
fn malformed_disk_trigger_is_skipped_without_embedded_fallback() {
    let locations = Locations::new();
    locations.write_schema(Root::Tree, "claudine.yaml", "kind: schema-trigger\nmatch: 42\n$schema: {}\n");
    assert_skipped_without_embedded_fallback(&locations, "malformed");
}

/// A disk trigger's import never resolves to the embedded file of the same
/// basename.
#[test]
fn disk_import_never_falls_back_to_an_embedded_file() {
    let locations = Locations::new();
    locations.write_schema(
        Root::Tree,
        "claudine.yaml",
        "kind: schema-trigger\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    start: stack@./lifecycle.yaml\n",
    );
    // The missing disk import skips the trigger.
    assert_skipped_without_embedded_fallback(&locations, "import");
}

/// A disk `lifecycle.yaml` whose recursive family names a type it does not
/// declare skips the trigger importing it: the embedded file of the same
/// basename, which declares `item`, never supplies it.
#[test]
fn broken_disk_recursive_family_never_falls_back_to_an_embedded_file() {
    let locations = Locations::new();
    locations.write_schema(
        Root::Tree,
        "claudine.yaml",
        "kind: schema-trigger\nmatch:\n    start: \"unknown[](required)\"\n$schema:\n    start: stack@./lifecycle.yaml\n",
    );
    locations.write_schema(
        Root::Tree,
        "lifecycle.yaml",
        "kind: schema\ntypes:\n    stack:\n        - item[]\n    conditional:\n        when: expression(required)\n        then: stack(required)\n",
    );
    // The broken disk family skips the trigger.
    assert_skipped_without_embedded_fallback(&locations, "broken");
}

/// A disk `claudine.yaml` that matches nothing still shadows the embedded
/// one, while the embedded inline-compose.yaml still applies.
#[test]
fn non_matching_disk_trigger_still_shadows_the_embedded_one() {
    let locations = Locations::new();
    locations.write_schema(
        Root::Tree,
        "claudine.yaml",
        "kind: schema-trigger\nmatch:\n    never_set: string(required)\n$schema:\n    never_set: string\n",
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

/// Document metadata cases: the frontmatter and the one property a problem
/// is reported under, or `None` when the document is valid. Absent and null
/// metadata stay valid, both stored hash forms are accepted, and the
/// longhand `value` takes any data. A scalar `prompt` or shorthand `hash` is
/// convertible text, and `last_updated` must be a calendar date.
const DOCUMENT_METADATA_CASES: &[(&str, Option<&str>)] = &[
    ("prompt: hi", None),
    ("prompt: hi\nhash: null\nlast_updated: null", None),
    ("start: [stop]\nprompt: null", None),
    ("prompt: 5", None),
    ("prompt: true", None),
    ("prompt: hi\nhash: 0123456789abcdef", None),
    ("prompt: hi\nhash: 42", None),
    ("prompt: hi\nhash: { kind: detailed, value: [1, { a: b }], ignored: [title] }", None),
    ("prompt: hi\nhash: { value: 42 }", None),
    ("prompt: hi\nhash: { value: { nested: [1, 2.5, true, null, text] } }", None),
    ("prompt: hi\nhash: { value: null }", None),
    ("prompt: hi\nhash: { kind: simple }", None),
    ("prompt: hi\nhash: {}", None),
    ("prompt: hi\nlast_updated: 2026-10-26", None),
    ("prompt: hi\nhash: [1]", Some("hash")),
    ("prompt: hi\nhash: { kind: bogus }", Some("hash")),
    ("prompt: hi\nhash: { ignored: title }", Some("hash")),
    ("prompt: hi\nhash: { kind: fm, extra: 1 }", Some("hash")),
    ("prompt: hi\nlast_updated: [2026-10-26]", Some("last_updated")),
    ("prompt: hi\nlast_updated: \"2026-02-30\"", Some("last_updated")),
    ("prompt: hi\nlast_updated: yesterday", Some("last_updated")),
    ("prompt: hi\nlast_updated: 2026-10-26T10:00:00Z", Some("last_updated")),
    ("start: [stop]\nprompt: [hi]", Some("prompt")),
    ("prompt: { a: 1 }", Some("prompt")),
];

/// The metadata properties `document.yaml` declares.
const DOCUMENT_METADATA: [&str; 3] = ["prompt", "hash", "last_updated"];

/// A disk trigger that never matches. It shadows the embedded trigger of its
/// name, so the other metadata trigger applies alone.
const NEVER_MATCHING_TRIGGER: &str =
    "kind: schema-trigger\nmatch:\n    never_set: string(required)\n$schema:\n    never_set: string\n";

/// Asserts the passive verdict and the launch verdict of every metadata
/// case at `locations`.
fn assert_document_metadata_cases(locations: &Locations, label: &str) {
    let schemas = locations.schemas();
    for (frontmatter, rejected_at) in DOCUMENT_METADATA_CASES {
        let what = format!("{label}: `{frontmatter}`");
        let expected: BTreeSet<String> = rejected_at.iter().map(|property| property.to_string()).collect();
        assert_eq!(locations.passive(&schemas, "metadata", frontmatter), expected, "{what}: passive");
        match rejected_at {
            None => assert_accepts(locations.verdict("metadata", frontmatter), &what),
            Some(property) => assert_rejects_at(locations.verdict("metadata", frontmatter), property, &what),
        }
    }
}

/// Passive validation and the launch verdict agree on every metadata case,
/// with the embedded bundle alone and with the shipped bundle on disk, where
/// the disk triggers import `document.yaml` from disk.
#[test]
fn document_metadata_verdicts_agree_between_passive_validation_and_launch() {
    let embedded = Locations::new();
    let on_disk = Locations::new();
    on_disk.write_bundle(Root::Tree, &shipped_bundle!());
    for (label, locations) in [("embedded", &embedded), ("disk", &on_disk)] {
        assert_document_metadata_cases(locations, label);
        let effective = locations.effective(&locations.schemas(), "origin", "prompt: hi");
        for property in DOCUMENT_METADATA {
            assert!(is_claudine_owned(&effective, property), "{label}: `{property}` is a Claudine control");
        }
    }
}

/// Each metadata trigger alone gives every metadata case the verdict the
/// merged pair gives it. A never-matching disk copy shadows the other
/// embedded trigger, so the metadata is declared by the one that applies.
#[test]
fn document_metadata_verdicts_agree_through_each_trigger_alone() {
    for (shadowed, applies) in [("inline-compose.yaml", "claudine.yaml"), ("claudine.yaml", "inline-compose.yaml")] {
        let locations = Locations::new();
        locations.write_schema(Root::Tree, shadowed, NEVER_MATCHING_TRIGGER);
        let label = format!("{applies} alone");
        assert_document_metadata_cases(&locations, &label);
        let effective = locations.effective(&locations.schemas(), "origin", "prompt: hi");
        for property in DOCUMENT_METADATA {
            assert_eq!(
                effective.origins[property].trigger_source(),
                Some(TriggerSource::Embedded { bundle: CLAUDINE_SCHEMA_BUNDLE_NAME.to_string(), name: applies.to_string() }),
                "{label}: `{property}`"
            );
            assert!(is_claudine_owned(&effective, property), "{label}: `{property}` is a Claudine control");
        }
    }
}

/// A disk `inline-compose.yaml` shadows the embedded one independently of
/// `claudine.yaml`, and the nearest copy wins. Where the copy alone applies,
/// its own type for `hash` judges the document. Where the embedded
/// `claudine.yaml` also applies, it is later in registry order, so its
/// declaration of `hash` replaces the copy's.
#[test]
fn a_disk_inline_compose_trigger_shadows_the_embedded_one_and_types_the_metadata() {
    let locations = Locations::new();
    let shadow = |hash: &str| {
        format!("kind: schema-trigger\nmatch:\n    hash: number(required)\n$schema:\n    hash: \"{hash}\"\n")
    };
    let origin = |frontmatter: &str| locations.effective(&locations.schemas(), "origin", frontmatter).origins["hash"].trigger_source();
    let embedded_claudine =
        Some(TriggerSource::Embedded { bundle: CLAUDINE_SCHEMA_BUNDLE_NAME.to_string(), name: "claudine.yaml".to_string() });
    assert_accepts(locations.verdict("none", "hash: 0"), "no trigger applies without a copy");

    let user = locations.write_schema(Root::Home, "inline-compose.yaml", &shadow("number(min(1))"));
    assert_eq!(origin("hash: 5"), Some(TriggerSource::File(user)));
    assert_accepts(locations.verdict("user", "hash: 5"), "user copy alone");
    assert_rejects_at(locations.verdict("user", "hash: 0"), "hash", "user copy alone: below its minimum");

    let project = locations.write_schema(Root::Package, "inline-compose.yaml", &shadow("number(max(0))"));
    assert_eq!(origin("hash: 0"), Some(TriggerSource::File(project)));
    assert_accepts(locations.verdict("project", "hash: 0"), "project copy alone");
    assert_rejects_at(locations.verdict("project", "hash: 5"), "hash", "the project copy shadows the user copy");

    // Beside a prompt the embedded claudine.yaml applies too, after the copy.
    // The embedded inline-compose.yaml would follow it and own `hash`, so
    // this origin also shows that the copy shadows it.
    assert_eq!(origin("prompt: hi\nhash: 5"), embedded_claudine);
    assert_accepts(locations.verdict("merged", "prompt: hi\nhash: 5"), "claudine.yaml's shorthand converts the number");
    assert_rejects_at(locations.verdict("merged", "prompt: hi\nhash: [1]"), "hash", "claudine.yaml's `hash`");
}

/// The `default` of `kind` in `hash`'s longhand arm within `fragment`,
/// following local references into `root`'s `$defs`.
fn longhand_kind_default(root: &serde_json::Value, fragment: &serde_json::Value) -> Option<serde_json::Value> {
    if let Some(reference) = fragment.get("$ref").and_then(serde_json::Value::as_str) {
        return longhand_kind_default(root, root.pointer(reference.strip_prefix('#')?)?);
    }
    if let Some(default) = fragment.pointer("/properties/kind/default") {
        return Some(default.clone());
    }
    fragment.get("anyOf")?.as_array()?.iter().find_map(|arm| longhand_kind_default(root, arm))
}

/// `hash`'s longhand `kind` declares `default(simple)`. The default survives
/// as an annotation through each trigger alone and through their merge, and
/// the launch never inserts it: a longhand without `kind` reaches the
/// prepared frontmatter as written, and so does a shorthand string.
#[test]
fn longhand_hash_kind_default_is_declared_and_never_inserted() {
    let both = Locations::new();
    let claudine_alone = Locations::new();
    claudine_alone.write_schema(Root::Tree, "inline-compose.yaml", NEVER_MATCHING_TRIGGER);
    let inline_alone = Locations::new();
    inline_alone.write_schema(Root::Tree, "claudine.yaml", NEVER_MATCHING_TRIGGER);
    for (label, locations) in [("both", &both), ("claudine.yaml alone", &claudine_alone), ("inline-compose.yaml alone", &inline_alone)] {
        let effective = locations.effective(&locations.schemas(), "default", "prompt: hi");
        let schema = &effective.json_schema;
        assert_eq!(
            longhand_kind_default(schema, &schema["properties"]["hash"]),
            Some(serde_json::json!("simple")),
            "{label}: `kind` declares its default"
        );
    }

    let context = both.context();
    let prepared_hash = |name: &str, frontmatter: &str, inline: bool| {
        let path = both.document(name, frontmatter);
        let source = resolve_composition_source_in_context(path.to_str().expect("UTF-8 path"), &context).expect("resolves");
        let options = claudine::composition::PrepareOptions::new(context.clone());
        let prepared = if inline {
            claudine::composition::prepare_inline_with_schema(&source, options)
        } else {
            claudine::composition::prepare_direct_with_schema(&source, options)
        };
        prepared.unwrap_or_else(|error| panic!("`{frontmatter}` prepares: {error:?}")).effective_frontmatter["hash"].clone()
    };
    for (frontmatter, inline, expected) in [
        ("prompt: hi\nhash: { value: 42 }", true, serde_json::json!({ "value": 42 })),
        ("start: [stop]\nhash: { value: 42 }", false, serde_json::json!({ "value": 42 })),
        ("prompt: hi\nhash: { kind: fm, value: x }", true, serde_json::json!({ "kind": "fm", "value": "x" })),
        ("prompt: hi\nhash: 0123456789abcdef", true, serde_json::json!("0123456789abcdef")),
    ] {
        assert_eq!(prepared_hash("prepared", frontmatter, inline), expected, "`{frontmatter}`");
    }
}

/// One `hash` imported into two properties of an authored schema is one
/// stored definition, referenced at both properties, that keeps its `kind`
/// default. Through the launch verdict both stored forms pass at each use,
/// and an invalid value is cleaned at its own property alone.
#[test]
fn a_shared_hash_definition_keeps_both_stored_forms_through_the_launch_verdict() {
    let locations = Locations::new();
    let library = locations.root.join("repo/area/pkg/docs/lib/document.yaml");
    std::fs::create_dir_all(library.parent().expect("parent")).expect("library dir");
    std::fs::write(&library, include_str!("../../../schemas/document.yaml")).expect("write the library");
    let schema = "$schema:\n    first: hash@./lib/document.yaml\n    second: hash@./lib/document.yaml\ntitle: x";

    let effective = locations.effective(&locations.schemas(), "shared", schema);
    let json = &effective.json_schema;
    let references = |property: &str| {
        let mut found = Vec::new();
        crate::claudine_schemas::collect_references(&json["properties"][property], &mut found);
        found
    };
    let first_refs = references("first");
    assert_eq!(first_refs.len(), 1, "`first` refers to one definition: {first_refs:?}");
    assert_eq!(first_refs, references("second"), "both uses refer to the same definition");
    let defs = json["$defs"].as_object().expect("the shared body is a definition");
    assert_eq!(defs.len(), 1, "only `hash` is stored: {:?}", defs.keys().collect::<Vec<_>>());
    for property in ["first", "second"] {
        assert_eq!(longhand_kind_default(json, &json["properties"][property]), Some(serde_json::json!("simple")), "{property}");
    }

    let shorthand = "0123456789abcdef";
    let longhand = "{ kind: structured, value: { mixed: [1, true] }, ignored: [title] }";
    for (first, second) in [(shorthand, longhand), (longhand, shorthand), (shorthand, shorthand), (longhand, longhand)] {
        let frontmatter = format!("{schema}\nfirst: {first}\nsecond: {second}");
        assert_accepts(locations.verdict("shared", &frontmatter), &frontmatter);
    }
    for (property, other) in [("first", "second"), ("second", "first")] {
        for invalid in ["[1]", "{ kind: bogus }", "{ ignored: title }"] {
            let frontmatter = format!("{schema}\n{property}: {invalid}\n{other}: {shorthand}");
            let validated = locations.verdict("shared", &frontmatter).unwrap_or_else(|error| panic!("{frontmatter}: {error:?}"));
            let dropped: Vec<&str> = validated.dropped_optionals.iter().map(|dropped| dropped.property.as_str()).collect();
            assert_eq!(dropped, [property], "{frontmatter}");
        }
    }
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
        let (cleaned, _, dropped) = drop_invalid_optionals(source, None, None, &context, &claudine::composition::TriggerFindings::default());
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
    let (cleaned, _, dropped) = drop_invalid_optionals(source, None, None, &context, &claudine::composition::TriggerFindings::default());
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

/// A stored literal token is data: the launch verdict judges the text it
/// holds as a generated verb's argument and never defers it as a template,
/// while the same text authored directly is a template and is deferred.
#[test]
fn a_stored_literal_token_is_type_checked_at_launch() {
    let locations = Locations::new();
    let token = darkmatter::markdown::literal_token::encode("{{ fm.count }}");

    let found = rejected(locations.verdict("token", &format!("start:\n    - abs: \"{token}\"")), "the stored token");
    assert!(found.iter().any(|pointer| pointer == "/start/0/abs"), "a problem at the operand, got {found:?}");

    assert_accepts(locations.verdict("authored", "start:\n    - abs: \"{{ fm.count }}\""), "the authored template");
    let numeral = darkmatter::markdown::literal_token::encode("3");
    assert_accepts(locations.verdict("numeral", &format!("start:\n    - abs: \"{numeral}\"")), "a token holding `3`");
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
    match (pre_validate_schema(&source, None, None, &context, &claudine::composition::TriggerFindings::default()), editor.valid) {
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
    passive_parity_depth6_wrong_verb =>
        ("depth6-wrong-verb", include_str!("../fixtures/claudine-schemas/depth6-wrong-verb.md"));
    passive_parity_depth5_guarded_bundle =>
        ("depth5-guarded-bundle", include_str!("../fixtures/claudine-schemas/depth5-guarded-bundle.md"));
    passive_parity_depth4_completion =>
        ("depth4-completion", include_str!("../fixtures/claudine-schemas/depth4-completion.md"));
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
    passive_parity_shell_in_initialize_depth3_then =>
        ("shell-in-initialize-depth3-then", include_str!("../fixtures/claudine-schemas/shell-in-initialize-depth3-then.md"));
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
    passive_parity_start_number => ("start-number", include_str!("../fixtures/claudine-schemas/start-number.md"));
    passive_parity_agent_empty_list => ("agent-empty-list", include_str!("../fixtures/claudine-schemas/agent-empty-list.md"));
    passive_parity_loop_number => ("loop-number", include_str!("../fixtures/claudine-schemas/loop-number.md"));
    passive_parity_generated_verb_typed_value =>
        ("generated-verb-typed-value", include_str!("../fixtures/claudine-schemas/generated-verb-typed-value.md"));
    passive_parity_generated_verb_wrong_type =>
        ("generated-verb-wrong-type", include_str!("../fixtures/claudine-schemas/generated-verb-wrong-type.md"));
    passive_parity_generated_verb_convertible_text =>
        ("generated-verb-convertible-text", include_str!("../fixtures/claudine-schemas/generated-verb-convertible-text.md"));
    passive_parity_generated_verb_deferred =>
        ("generated-verb-deferred", include_str!("../fixtures/claudine-schemas/generated-verb-deferred.md"));
    passive_parity_generated_verb_deferred_beside_unknown_parameter =>
        ("generated-verb-deferred-beside-unknown-parameter", include_str!("../fixtures/claudine-schemas/generated-verb-deferred-beside-unknown-parameter.md"));
    passive_parity_generated_verb_deferred_beside_extra_argument =>
        ("generated-verb-deferred-beside-extra-argument", include_str!("../fixtures/claudine-schemas/generated-verb-deferred-beside-extra-argument.md"));
    passive_parity_generated_verb_deferred_beside_bad_sibling =>
        ("generated-verb-deferred-beside-bad-sibling", include_str!("../fixtures/claudine-schemas/generated-verb-deferred-beside-bad-sibling.md"));
    passive_parity_generated_verb_malformed_template =>
        ("generated-verb-malformed-template", include_str!("../fixtures/claudine-schemas/generated-verb-malformed-template.md"));
    passive_parity_generated_verb_malformed_string_template =>
        ("generated-verb-malformed-string-template", include_str!("../fixtures/claudine-schemas/generated-verb-malformed-string-template.md"));
    passive_parity_handwritten_action_malformed_template =>
        ("handwritten-action-malformed-template", include_str!("../fixtures/claudine-schemas/handwritten-action-malformed-template.md"));
    passive_parity_optional_nonnullable_null =>
        ("optional-nonnullable-null", include_str!("../fixtures/claudine-schemas/optional-nonnullable-null.md"));
    passive_parity_ensure_file_overloads =>
        ("ensure-file-overloads", include_str!("../fixtures/claudine-schemas/ensure-file-overloads.md"));
    passive_parity_variadic_and_or =>
        ("variadic-and-or", include_str!("../fixtures/claudine-schemas/variadic-and-or.md"));
    passive_parity_generated_verb_numeric_recovery =>
        ("generated-verb-numeric-recovery", include_str!("../fixtures/claudine-schemas/generated-verb-numeric-recovery.md"));
    passive_parity_generated_verb_excluded_source =>
        ("generated-verb-excluded-source", include_str!("../fixtures/claudine-schemas/generated-verb-excluded-source.md"));
    passive_parity_single_action_root_dictionary =>
        ("single-action-root-dictionary", include_str!("../fixtures/claudine-schemas/single-action-root-dictionary.md"));
    passive_parity_single_action_branch_dictionaries =>
        ("single-action-branch-dictionaries", include_str!("../fixtures/claudine-schemas/single-action-branch-dictionaries.md"));
    passive_parity_single_action_loop_loose =>
        ("single-action-loop-loose", include_str!("../fixtures/claudine-schemas/single-action-loop-loose.md"));
    passive_parity_long_forms_in_loop_bundle =>
        ("long-forms-in-loop-bundle", include_str!("../fixtures/claudine-schemas/long-forms-in-loop-bundle.md"));
    passive_parity_conditional_root_dictionary =>
        ("conditional-root-dictionary", include_str!("../fixtures/claudine-schemas/conditional-root-dictionary.md"));
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

/// The lifecycle helpers (`entry`, the action categories, the short-form
/// bundle) are reached from several events, list items, and conditional
/// bodies. Each failing use keeps the problem code, full instance path,
/// line and column, the union explanation it reports, and its declaring
/// trigger, whatever way the resolver stores those helpers. Recorded while
/// every acyclic helper was inlined at each use.
#[test]
fn a_shared_lifecycle_helper_reports_each_failing_use_where_it_is_written() {
    use darkmatter::markdown::schemas::ValidationProblemCode::ConstraintViolation;
    use darkmatter::markdown::schemas::validate::build_position_map;

    const BARE_VERB: &str = "a zero-argument verb written as a bare list item";
    let bare = Bare::new();
    let schemas = bare.schemas(claudine_schema_bundle());
    /// `(instance path, line, column, explanation)` of each expected problem.
    type Expected = &'static [(&'static str, u32, u32, &'static str)];
    let cases: [(&str, Expected); 5] = [
        (
            "start:\n  - inf0: hi\nfinalize:\n  - say: hi\n  - inf0: hi\n",
            &[("/start/0", 2, 5, BARE_VERB), ("/finalize/1", 5, 5, BARE_VERB)],
        ),
        (
            "success:\n  - when: \"{{ ready }}\"\n    then:\n      - say: hi\n      - inf0: hi\n",
            &[("/success/0/then/1", 5, 9, BARE_VERB)],
        ),
        ("start:\n  - say: hi\n    inf0: hi\n", &[("/start/0", 2, 5, BARE_VERB)]),
        // `cancelled` admits a shell; `initialize` never does.
        ("initialize:\n  - shell: ls\ncancelled:\n  - shell: ls\n", &[("/initialize/0", 2, 5, BARE_VERB)]),
        // A loop action missing its required `prop`.
        ("loop:\n  action:\n    op: set\n", &[("/loop/action", 3, 5, "an iteration action such as increment(iteration)")]),
    ];
    for (frontmatter, expected) in cases {
        let markdown = memory_document(frontmatter);
        let effective = schemas.effective_for(&markdown).expect("resolves").expect("a trigger applies");
        let instance = crate::claudine_schemas::frontmatter_json(&markdown);
        let report = effective.validate_with_positions(&instance, &build_position_map(frontmatter));
        let found: Vec<_> = report
            .problems
            .iter()
            .map(|problem| (problem.code, problem.path.as_str(), problem.line, problem.column, problem.description.as_deref()))
            .collect();
        let expected: Vec<_> = expected
            .iter()
            .map(|(path, line, column, description)| (ConstraintViolation, *path, Some(*line), Some(*column), Some(*description)))
            .collect();
        assert_eq!(found, expected, "`{frontmatter}`");
        for property in instance.as_object().expect("a mapping").keys() {
            let origin = effective.origins.get(property).unwrap_or_else(|| panic!("`{property}` has an origin"));
            assert_eq!(
                origin.trigger_source(),
                Some(TriggerSource::Embedded {
                    bundle: CLAUDINE_SCHEMA_BUNDLE_NAME.to_string(),
                    name: "claudine.yaml".to_string()
                }),
                "`{property}` in `{frontmatter}`"
            );
        }
    }
    bare.assert_untouched();
}
