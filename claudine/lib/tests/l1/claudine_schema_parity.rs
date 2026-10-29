//! Claudine's registry and its schema bundle agree: the bundle types exactly
//! the registry's authored event names (the iteration gate's stack under
//! `loop.gate`), each event admits exactly the action types the registry
//! declares for it, and a Darkmatter event name Claudine spells differently
//! is an error that names Claudine's spelling. Claudine's action categories
//! and bundles are Darkmatter's neutral definitions, reached through the
//! reserved `darkmatter:` namespace and closed to unknown verbs.
//!
//! Documents are judged as Claudine's launch verdict judges them: through the
//! bundle embedded in the binary. The expected names and action types come
//! from the registries, never from a list kept here.

use std::collections::{BTreeMap, BTreeSet};

use claudine::composition::lifecycle::claudine_registry;
use darkmatter::lifecycle::md_host;
use darkmatter::lifecycle::registry::{ActionType, ActionTypes, CoreEvent, Role};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};
use serde_json::Value;

use crate::claudine_schemas::{Discovered, SchemaRepo, embedded_bundle, problems, shipped_bundle};

/// Every action type, each with one well-formed action of that type that
/// both hosts provide.
const SAMPLES: [(ActionType, &str); 4] = [
    (ActionType::Communicate, "{ info: hi }"),
    (ActionType::Flow, "{ stop: null }"),
    (ActionType::Shell, "{ shell: echo hi }"),
    (ActionType::Expression, "{ set: { answer: 42 } }"),
];

/// A stack whose one action is well formed.
const WELL_FORMED: &str = "[{ info: hi }]";
/// A stack whose one action is a known verb in a malformed long form.
const MALFORMED: &str = "[{ info: { text: hi, bogus: 1 } }]";

/// Selects Claudine's trigger whatever else a probe holds.
const SELECTOR: &str = "agent: claude";

struct Bundle {
    repo: SchemaRepo,
    discovered: Discovered,
}

impl Bundle {
    fn embedded() -> Self {
        let shipped = shipped_bundle!();
        let repo = SchemaRepo::new(&[]);
        let discovered = repo.discover_embedded(embedded_bundle(&shipped)).expect("the embedded bundle loads");
        Self { repo, discovered }
    }

    fn judge(&self, frontmatter: &str) -> ValidationReport {
        let path = self.repo.write_document("probe.md", &format!("---\n{SELECTOR}\n{frontmatter}\n---\n"));
        let judged = self
            .discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged");
        assert!(judged.claudine, "the probe selects Claudine's trigger: {frontmatter}");
        judged.report
    }

    /// The frontmatter placing `stack` under `event`: the iteration gate's
    /// stack is its block's `gate`.
    fn placed(event: &str, stack: &str) -> String {
        let registry = claudine_registry();
        let id = registry.id(event).expect("a Claudine event");
        if registry.role(id) == Some(Role::IterationGate) {
            format!("{event}: {{ gate: {stack} }}")
        } else {
            format!("{event}: {stack}")
        }
    }

    fn types_as_stack(&self, key: &str) -> bool {
        self.judge(&format!("{key}: {WELL_FORMED}")).valid && !self.judge(&format!("{key}: {MALFORMED}")).valid
    }
}

#[test]
fn the_samples_cover_every_action_type_once() {
    let types: Vec<ActionType> = SAMPLES.iter().map(|(action, _)| *action).collect();
    assert_eq!(ActionTypes::of(types.iter().copied()), ActionTypes::all());
    assert_eq!(types.iter().collect::<BTreeSet<_>>().len(), types.len(), "each type once");
}

#[test]
fn the_schema_types_exactly_the_registry_event_names() {
    let bundle = Bundle::embedded();
    let registry = claudine_registry();
    let md = md_host::registry();
    let authored: BTreeSet<&str> = registry.authored_event_keys().into_iter().collect();
    let gates: BTreeSet<&str> = authored
        .iter()
        .copied()
        .filter(|event| registry.role(registry.id(event).expect("event")) == Some(Role::IterationGate))
        .collect();
    let candidates: BTreeSet<&str> = authored.iter().copied().chain(md.authored_event_keys()).collect();
    let typed: BTreeSet<&str> = candidates.iter().copied().filter(|key| bundle.types_as_stack(key)).collect();
    let expected: BTreeSet<&str> = authored.difference(&gates).copied().collect();
    assert_eq!(typed, expected, "the list-typed stacks are Claudine's event names");
    for gate in gates {
        assert!(bundle.judge(&Bundle::placed(gate, WELL_FORMED)).valid, "`{gate}.gate` accepts a stack");
        assert!(!bundle.judge(&Bundle::placed(gate, MALFORMED)).valid, "`{gate}.gate` is typed");
    }
}

#[test]
fn darkmatter_event_names_claudine_spells_differently_name_the_claudine_event() {
    let bundle = Bundle::embedded();
    let registry = claudine_registry();
    let renamed: Vec<(&str, &str)> = CoreEvent::ALL
        .into_iter()
        .filter_map(|event| {
            let claudine = registry.name_of(event.signal()).expect("Claudine names every Darkmatter event");
            (claudine != event.darkmatter_name()).then_some((event.darkmatter_name(), claudine))
        })
        .collect();
    assert!(!renamed.is_empty(), "Claudine renames at least one Darkmatter event");
    for (darkmatter, claudine) in renamed {
        assert!(registry.id(darkmatter).is_none(), "`{darkmatter}` is not a Claudine event");
        for stack in [WELL_FORMED, "[]", "{ info: hi }"] {
            let report = bundle.judge(&format!("{darkmatter}: {stack}"));
            assert!(!report.valid, "`{darkmatter}: {stack}` is rejected");
            let named = report.problems.iter().any(|problem| {
                problem.path.starts_with(&format!("/{darkmatter}"))
                    && problem.description.as_deref().is_some_and(|description| description.contains(&format!("`{claudine}`")))
            });
            assert!(named, "`{darkmatter}` names `{claudine}`: {:?}", report.problems);
        }
        assert!(bundle.judge(&format!("{darkmatter}: null")).valid, "an explicit null passes");
    }
}

/// With Darkmatter's baseline too (`md schema validate` on a Claudine
/// document), Claudine's declarations override the baseline's md stacks.
#[test]
fn claudine_declarations_override_the_md_baseline() {
    let bundle = Bundle::embedded();
    let schemas: DarkmatterSchemas =
        bundle.discovered.schemas.clone().with_darkmatter_baseline_json_schema().expect("the baseline attaches");
    let validate = |frontmatter: &str| {
        let path = bundle.repo.write_document("baseline.md", &format!("---\n{SELECTOR}\n{frontmatter}\n---\n"));
        let markdown =
            Markdown::try_from(path.as_path()).expect("loads").with_source(ComposeSource::File(path.clone()));
        schemas.validate(&markdown).expect("the schema prepares")
    };
    for (frontmatter, valid) in [
        ("composed: [{ info: hi }]", false),
        ("failed: [{ info: hi }]", false),
        ("start: [{ say: hi, retry: 2 }]", true),
        ("initialize: [{ say: hi }]", true),
        ("initialize: [{ shell: echo hi }]", false),
        ("cancelled: [{ error: oops }]", false),
        ("finalize: [{ bogus_verb: 1 }]", false),
        ("finalize: [{ retry: 2 }]", true),
    ] {
        let report = validate(frontmatter);
        assert_eq!(report.valid, valid, "`{frontmatter}`: {:?}", problems(&report));
    }
}

#[test]
fn each_event_admits_exactly_the_registry_action_types() {
    let bundle = Bundle::embedded();
    let registry = claudine_registry();
    for event in registry.authored_event_keys() {
        let id = registry.id(event).expect("an authored key names an event");
        for (action, sample) in SAMPLES {
            let report = bundle.judge(&Bundle::placed(event, &format!("[{sample}]")));
            assert_eq!(
                report.valid,
                registry.actions(id).contains(action),
                "`{event}` with a {action:?} action ({sample}): {:?}",
                problems(&report)
            );
        }
    }
}

/// The JSON Schema each `{type}@{reference}` lowers to, from an inline
/// `$schema` in the repository's `docs/`.
fn lowered(repo: &SchemaRepo, references: &[(&str, String)]) -> BTreeMap<String, Value> {
    let mut frontmatter = String::from("---\n$schema:\n");
    for (name, reference) in references {
        frontmatter.push_str(&format!("    {name}: \"{reference}\"\n"));
    }
    frontmatter.push_str("---\n");
    let path = repo.write_document("lowered.md", &frontmatter);
    let markdown = Markdown::try_from(path.as_path()).expect("loads").with_source(ComposeSource::File(path.clone()));
    let effective = DarkmatterSchemas::new(repo.context())
        .effective_for(&markdown)
        .expect("resolves")
        .expect("an inline schema is effective");
    references
        .iter()
        .map(|(name, _)| (name.to_string(), effective.json_schema["properties"][*name].clone()))
        .collect()
}

#[test]
fn claudine_action_types_are_darkmatters_neutral_definitions() {
    let repo = SchemaRepo::new(&[("action.yaml", include_str!("../../../schemas/action.yaml").to_string())]);
    let exports = ["interpolation", "count", "communicate", "flow", "shell", "expression", "bundle", "initialize-bundle"];
    let claudine: Vec<(&str, String)> =
        exports.iter().map(|export| (*export, format!("{export}@../schemas/action.yaml"))).collect();
    let darkmatter: Vec<(&str, String)> =
        exports.iter().map(|export| (*export, format!("{export}@darkmatter:lifecycle"))).collect();
    let claudine = lowered(&repo, &claudine);
    let darkmatter = lowered(&repo, &darkmatter);
    for export in exports {
        assert!(!claudine[export].is_null(), "action.yaml exports `{export}`");
        assert_eq!(claudine[export], darkmatter[export], "`{export}` is Darkmatter's neutral definition");
    }
    // Closed to unknown verbs: no category or bundle has an open key.
    for export in ["communicate", "flow", "shell", "expression", "bundle", "initialize-bundle"] {
        let object = claudine[export]["anyOf"]
            .as_array()
            .and_then(|arms| arms.iter().find(|arm| arm["type"] == "object"))
            .unwrap_or(&claudine[export]);
        assert_eq!(object["additionalProperties"], Value::Bool(false), "`{export}` is closed");
        assert!(object.get("patternProperties").is_none(), "`{export}` has no open key");
    }
}
