//! Claudine's registration inventory (`claudine/registration/`), installed
//! as a format 2 `darkmatter-host` bundle in a temporary home and judged
//! through Darkmatter's registered-host validation: which trigger each
//! document selects, the inline composition's `prompt`/`sections` union, the
//! sequence triggers, the formal `kind: sequence` YAML kind, and the
//! configuration schema setup consumes.
//!
//! Each test spells the repository files it installs with `include_str!`.
//! Nothing is read from the process or the checkout.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::load::{ConfigRevision, Registration};
use darkmatter::config::registration::namespaces::member_digest;
use darkmatter::config::registration::{
    HostClaim, HostSelection, RegisteredHostProfile, RegisteredHosts, load_entries, md_owned_names,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, Route, YamlOutcome, frontmatter_as_json};

/// The manifest's file name inside the installed bundle folder.
const MANIFEST: &str = "manifest.yaml";
/// The lifecycle artifact's bundle path.
const LIFECYCLE: &str = "globals.lifecycle.json";

/// The registration files, by file name under `schemas/`, with their text.
type Files<'a> = [(&'a str, &'a str)];

/// A temporary home and working directory with `files` installed as
/// Claudine's bundle under `home/.claudine/registrations/darkmatter/`.
struct Installed {
    _dir: tempfile::TempDir,
    root: PathBuf,
    hosts: Arc<RegisteredHosts>,
}

impl Installed {
    fn new(files: &Files<'_>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        let bundle = root.join("home/.claudine/registrations/darkmatter");
        let mut members: BTreeMap<String, String> =
            files.iter().map(|(name, text)| (format!("schemas/{name}"), (*text).to_string())).collect();
        members.insert(LIFECYCLE.to_string(), claudine::composition::lifecycle::lifecycle_artifact().expect("the lifecycle artifact generates"));
        for (path, text) in &members {
            let path = bundle.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let mut manifest = String::from("kind: darkmatter-host\nformat_version: 2\nhost: claudine\nversion: \"0.1.0\"\nfiles:\n");
        for name in members.keys().filter(|path| path.starts_with("schemas/")) {
            manifest.push_str(&format!("    - ./{name}\n"));
        }
        manifest.push_str(&format!("lifecycle: ./{LIFECYCLE}\nfunctions: []\ndigests:\n"));
        for (path, text) in &members {
            manifest.push_str(&format!("    ./{path}: \"{}\"\n", member_digest(text.as_bytes())));
        }
        std::fs::write(bundle.join(MANIFEST), manifest).unwrap();

        let context = Self::context_at(&root);
        let entry = Registration {
            reference: format!("~/.claudine/registrations/darkmatter/{MANIFEST}"),
            index: 0,
            declared_in: root.join("home/.darkmatter.yaml"),
            revision: ConfigRevision::User,
            at: None,
        };
        let hosts = Arc::new(RegisteredHosts::new(Arc::new(load_entries(&[entry], &context, md_owned_names()))));
        assert!(hosts.diagnostics().is_empty(), "Claudine's inventory loads: {:#?}", hosts.diagnostics());
        Self { _dir: dir, root, hosts }
    }

    /// Claudine's one agreed profile.
    fn profile(&self) -> Arc<RegisteredHostProfile> {
        match self.hosts.claims() {
            [HostClaim::Agreed { profile, .. }] => Arc::clone(profile),
            other => panic!("one agreed Claudine claim, not {other:?}"),
        }
    }

    fn context_at(root: &Path) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(root.join("work"), Some(root.join("home")), HashMap::new())
    }

    fn context(&self) -> FileResolutionContext {
        Self::context_at(&self.root)
    }

    /// Writes `work/<name>` and returns it with its context.
    fn write(&self, name: &str, text: &str) -> (PathBuf, FileResolutionContext) {
        let path = self.root.join("work").join(name);
        std::fs::write(&path, text).unwrap();
        let context = self.context().for_source(&path);
        (path, context)
    }

    fn schemas(&self, context: FileResolutionContext) -> DarkmatterSchemas {
        DarkmatterSchemas::new(context).with_registered_hosts(Arc::clone(&self.hosts))
    }

    /// The file names of the Claudine triggers `frontmatter` (YAML) selects;
    /// empty when no Claudine trigger matches.
    fn selected(&self, frontmatter: &str) -> Vec<String> {
        let (path, context) = self.write("doc.md", &format!("---\n{frontmatter}---\nBody.\n"));
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let selected = self.hosts.identify(&frontmatter_as_json(&markdown), Some(&path), &context);
        match &selected.selection {
            HostSelection::Registered(profile) => assert_eq!(profile.host(), "claudine"),
            HostSelection::Md => assert!(selected.matched.is_empty()),
            HostSelection::Ambiguous(hosts) => panic!("only Claudine is registered, not {hosts:?}"),
        }
        selected
            .matched_triggers()
            .map(|trigger| trigger.path().file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    /// The problem paths of a Markdown document with `frontmatter`.
    fn problems(&self, frontmatter: &str) -> Vec<String> {
        let (path, context) = self.write("doc.md", &format!("---\n{frontmatter}---\nBody.\n"));
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let effective = self
            .schemas(context)
            .effective_for(&markdown)
            .unwrap_or_else(|error| panic!("the schema assembles for {frontmatter:?}: {error}"))
            .expect("a schema applies");
        let mut paths: Vec<String> =
            effective.validate(&frontmatter_as_json(&markdown)).problems.iter().map(|problem| problem.path.clone()).collect();
        paths.sort();
        paths.dedup();
        paths
    }

    /// The route and problem paths of the YAML document `text`.
    fn yaml_problems(&self, text: &str) -> (Route, Vec<String>) {
        let (path, context) = self.write("plan.yaml", text);
        match self.schemas(context).validate_yaml_document(text, &path) {
            Ok(YamlOutcome::Validated(validation)) => {
                let mut paths: Vec<String> = validation.report.problems.iter().map(|problem| problem.path.clone()).collect();
                paths.sort();
                paths.dedup();
                (validation.route, paths)
            }
            other => panic!("a validated YAML document for {text:?}: {other:?}"),
        }
    }
}

#[test]
fn the_inventory_installs_whole_and_its_content_policy_copy_is_exact() {
    let files: &Files<'_> = &[
        ("action.yaml", include_str!("../../../registration/action.yaml")),
        ("claudine-config.yaml", include_str!("../../../registration/claudine-config.yaml")),
        ("composition.yaml", include_str!("../../../registration/composition.yaml")),
        ("content-policy.yaml", include_str!("../../../registration/content-policy.yaml")),
        ("document.yaml", include_str!("../../../registration/document.yaml")),
        ("formal-sequence-file.yaml", include_str!("../../../registration/formal-sequence-file.yaml")),
        ("group.yaml", include_str!("../../../registration/group.yaml")),
        ("inline-composition.yaml", include_str!("../../../registration/inline-composition.yaml")),
        ("lifecycle-events.yaml", include_str!("../../../registration/lifecycle-events.yaml")),
        ("linking.yaml", include_str!("../../../registration/linking.yaml")),
        ("loop.yaml", include_str!("../../../registration/loop.yaml")),
        ("seq.yaml", include_str!("../../../registration/seq.yaml")),
        ("sequence-document.yaml", include_str!("../../../registration/sequence-document.yaml")),
        ("sequence.yaml", include_str!("../../../registration/sequence.yaml")),
        ("shared.yaml", include_str!("../../../registration/shared.yaml")),
    ];
    // Loading refuses an import outside the listed members or the built-in
    // `darkmatter:` sources, so a whole install proves every reference.
    let installed = Installed::new(files);
    let profile = installed.profile();
    let mut triggers: Vec<String> =
        profile.triggers().iter().map(|trigger| trigger.path().file_name().unwrap().to_string_lossy().into_owned()).collect();
    triggers.sort();
    assert_eq!(triggers, ["composition.yaml", "inline-composition.yaml", "sequence-document.yaml"]);

    assert_eq!(
        include_str!("../../../registration/content-policy.yaml"),
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        "the bundled content-policy types are a byte-for-byte copy"
    );
}

#[test]
fn shared_events_alone_stay_darkmatters_and_a_misspelled_kind_selects_nothing() {
    let files: &Files<'_> = &[
        ("action.yaml", include_str!("../../../registration/action.yaml")),
        ("claudine-config.yaml", include_str!("../../../registration/claudine-config.yaml")),
        ("composition.yaml", include_str!("../../../registration/composition.yaml")),
        ("content-policy.yaml", include_str!("../../../registration/content-policy.yaml")),
        ("document.yaml", include_str!("../../../registration/document.yaml")),
        ("formal-sequence-file.yaml", include_str!("../../../registration/formal-sequence-file.yaml")),
        ("group.yaml", include_str!("../../../registration/group.yaml")),
        ("inline-composition.yaml", include_str!("../../../registration/inline-composition.yaml")),
        ("lifecycle-events.yaml", include_str!("../../../registration/lifecycle-events.yaml")),
        ("linking.yaml", include_str!("../../../registration/linking.yaml")),
        ("loop.yaml", include_str!("../../../registration/loop.yaml")),
        ("seq.yaml", include_str!("../../../registration/seq.yaml")),
        ("sequence-document.yaml", include_str!("../../../registration/sequence-document.yaml")),
        ("sequence.yaml", include_str!("../../../registration/sequence.yaml")),
        ("shared.yaml", include_str!("../../../registration/shared.yaml")),
    ];
    let installed = Installed::new(files);
    let none: [&str; 0] = [];
    assert_eq!(installed.selected("initialize: [{ say: hi }]\nfinalize: [{ say: bye }]\ncancelled: [stop]\n"), none);
    assert_eq!(installed.selected("title: plain\n"), none);
    assert_eq!(installed.selected("kind: inline-composee\n"), none, "a literal mismatch is no inline composition");
    assert_eq!(installed.selected("kind: inline-composee\nagent: claude\n"), ["composition.yaml"]);
    assert!(installed.problems("kind: inline-composee\nagent: claude\n").is_empty());
    assert_eq!(installed.selected("agent: claude\nprompt: null\nsections: null\n"), ["composition.yaml"], "null is absence");
}

#[test]
fn prompt_or_sections_selects_the_inline_union_and_each_arm_admits_only_null_for_the_other() {
    let files: &Files<'_> = &[
        ("action.yaml", include_str!("../../../registration/action.yaml")),
        ("claudine-config.yaml", include_str!("../../../registration/claudine-config.yaml")),
        ("composition.yaml", include_str!("../../../registration/composition.yaml")),
        ("content-policy.yaml", include_str!("../../../registration/content-policy.yaml")),
        ("document.yaml", include_str!("../../../registration/document.yaml")),
        ("formal-sequence-file.yaml", include_str!("../../../registration/formal-sequence-file.yaml")),
        ("group.yaml", include_str!("../../../registration/group.yaml")),
        ("inline-composition.yaml", include_str!("../../../registration/inline-composition.yaml")),
        ("lifecycle-events.yaml", include_str!("../../../registration/lifecycle-events.yaml")),
        ("linking.yaml", include_str!("../../../registration/linking.yaml")),
        ("loop.yaml", include_str!("../../../registration/loop.yaml")),
        ("seq.yaml", include_str!("../../../registration/seq.yaml")),
        ("sequence-document.yaml", include_str!("../../../registration/sequence-document.yaml")),
        ("sequence.yaml", include_str!("../../../registration/sequence.yaml")),
        ("shared.yaml", include_str!("../../../registration/shared.yaml")),
    ];
    let installed = Installed::new(files);
    for valid in [
        "prompt: Fix the bug.\n",
        "sections:\n    Goal: Fix the bug.\n",
        "prompt: Fix the bug.\nsections: null\n",
        "sections: { Goal: Fix the bug. }\nprompt: null\n",
        "kind: inline-compose\nprompt: Fix the bug.\nagent: claude\nstart: [{ say: go }]\n",
    ] {
        assert_eq!(installed.selected(valid), ["inline-composition.yaml"], "{valid}");
        assert!(installed.problems(valid).is_empty(), "{valid}: {:?}", installed.problems(valid));
    }
    for invalid in [
        "prompt: Fix the bug.\nsections: { Goal: Fix the bug. }\n",
        "kind: inline-compose\n",
        "kind: inline-compose\nagent: claude\n",
        "prompt: { text: Fix the bug. }\n",
        "sections: [Goal]\n",
    ] {
        assert_eq!(installed.selected(invalid), ["inline-composition.yaml"], "{invalid}");
        assert!(!installed.problems(invalid).is_empty(), "{invalid} is refused");
    }
    // The common attributes are typed in each arm.
    assert_eq!(installed.problems("prompt: Fix the bug.\nagent: { a: 1 }\n"), ["/agent"]);
    assert_eq!(installed.problems("sections: { Goal: x }\ncontent_policy: []\n"), ["/content_policy"]);
}

#[test]
fn a_sequence_document_selects_composition_and_its_sequence_trigger_and_types_its_steps() {
    let files: &Files<'_> = &[
        ("action.yaml", include_str!("../../../registration/action.yaml")),
        ("claudine-config.yaml", include_str!("../../../registration/claudine-config.yaml")),
        ("composition.yaml", include_str!("../../../registration/composition.yaml")),
        ("content-policy.yaml", include_str!("../../../registration/content-policy.yaml")),
        ("document.yaml", include_str!("../../../registration/document.yaml")),
        ("formal-sequence-file.yaml", include_str!("../../../registration/formal-sequence-file.yaml")),
        ("group.yaml", include_str!("../../../registration/group.yaml")),
        ("inline-composition.yaml", include_str!("../../../registration/inline-composition.yaml")),
        ("lifecycle-events.yaml", include_str!("../../../registration/lifecycle-events.yaml")),
        ("linking.yaml", include_str!("../../../registration/linking.yaml")),
        ("loop.yaml", include_str!("../../../registration/loop.yaml")),
        ("seq.yaml", include_str!("../../../registration/seq.yaml")),
        ("sequence-document.yaml", include_str!("../../../registration/sequence-document.yaml")),
        ("sequence.yaml", include_str!("../../../registration/sequence.yaml")),
        ("shared.yaml", include_str!("../../../registration/shared.yaml")),
    ];
    let installed = Installed::new(files);
    let both = ["composition.yaml", "sequence-document.yaml"];
    for valid in [
        "sequence: [plan, implement]\n",
        "sequence: ./steps.yaml\n",
        "sequence:\n    - name: plan\n      prompt: ./plan.md\n      when: \"{{ true }}\"\n",
        "kind: sequence\nsequence: []\n",
        // A sequence document is never an inline composition, so it may carry
        // both keys the inline union tells apart.
        "sequence: [plan]\nprompt: Plan it.\nsections: { Goal: x }\n",
    ] {
        assert_eq!(installed.selected(valid), both, "{valid}");
        assert!(installed.problems(valid).is_empty(), "{valid}: {:?}", installed.problems(valid));
    }
    assert_eq!(installed.selected("kind: sequence\n"), both);
    // A missing required key is reported on the root.
    assert_eq!(installed.problems("kind: sequence\n"), [""], "an explicit sequence kind requires `sequence`");
    assert_eq!(installed.problems("sequence:\n    - name: { a: 1 }\n"), ["/sequence/0/name"]);
    assert_eq!(installed.problems("sequence:\n    - name: a\n      group: { tasks: [] }\n"), ["/sequence/0/group/tasks"]);
    assert_eq!(installed.problems("sequence: [plan]\nfail_fast: true\n"), ["/fail_fast"]);
    assert_eq!(installed.problems("sequence: [plan]\nagent: { a: 1 }\n"), ["/agent"], "composition types its controls");
}

#[test]
fn a_matching_sequence_layer_cannot_weaken_the_inline_union() {
    let files: &Files<'_> = &[
        ("action.yaml", include_str!("../../../registration/action.yaml")),
        ("claudine-config.yaml", include_str!("../../../registration/claudine-config.yaml")),
        ("composition.yaml", include_str!("../../../registration/composition.yaml")),
        ("content-policy.yaml", include_str!("../../../registration/content-policy.yaml")),
        ("document.yaml", include_str!("../../../registration/document.yaml")),
        ("formal-sequence-file.yaml", include_str!("../../../registration/formal-sequence-file.yaml")),
        ("group.yaml", include_str!("../../../registration/group.yaml")),
        ("inline-composition.yaml", include_str!("../../../registration/inline-composition.yaml")),
        ("lifecycle-events.yaml", include_str!("../../../registration/lifecycle-events.yaml")),
        ("linking.yaml", include_str!("../../../registration/linking.yaml")),
        ("loop.yaml", include_str!("../../../registration/loop.yaml")),
        ("seq.yaml", include_str!("../../../registration/seq.yaml")),
        ("sequence-document.yaml", include_str!("../../../registration/sequence-document.yaml")),
        ("sequence.yaml", include_str!("../../../registration/sequence.yaml")),
        ("shared.yaml", include_str!("../../../registration/shared.yaml")),
    ];
    let installed = Installed::new(files);
    let profile = installed.profile();
    let index = |name: &str| {
        profile.triggers().iter().position(|trigger| trigger.path().file_name().unwrap() == name).expect("a trigger")
    };
    let (inline, sequence) = (index("inline-composition.yaml"), index("sequence-document.yaml"));

    // The two never match one document; checked together anyway, the
    // sequence payload declares nothing the union's arms tell apart, so the
    // union comes through unchanged and still owns the root.
    let alone = profile.checked_layer(&[inline]).expect("the union checks alone");
    let together = profile.checked_layer(&[inline, sequence]).expect("a sequence layer is compatible with the union");
    let union = |layer: &darkmatter::config::registration::merge::CheckedLayer| {
        let part = layer.payloads.iter().find(|part| part.union).expect("the root union").clone();
        assert_eq!(part.trigger, inline);
        part.json_schema
    };
    assert_eq!(union(&alone), union(&together));
    let sequence_part = together.payloads.iter().find(|part| part.trigger == sequence).expect("the sequence payload");
    let declared: Vec<&String> =
        sequence_part.json_schema["properties"].as_object().expect("an object payload").keys().collect();
    assert!(!declared.iter().any(|key| *key == "prompt" || *key == "sections"), "{declared:?}");
    assert_eq!(installed.selected("prompt: x\nsections: { a: b }\n"), ["inline-composition.yaml"]);
    assert!(!installed.problems("prompt: x\nsections: { a: b }\n").is_empty());
}

#[test]
fn a_formal_sequence_yaml_requires_an_inline_step_list() {
    let files: &Files<'_> = &[
        ("action.yaml", include_str!("../../../registration/action.yaml")),
        ("claudine-config.yaml", include_str!("../../../registration/claudine-config.yaml")),
        ("composition.yaml", include_str!("../../../registration/composition.yaml")),
        ("content-policy.yaml", include_str!("../../../registration/content-policy.yaml")),
        ("document.yaml", include_str!("../../../registration/document.yaml")),
        ("formal-sequence-file.yaml", include_str!("../../../registration/formal-sequence-file.yaml")),
        ("group.yaml", include_str!("../../../registration/group.yaml")),
        ("inline-composition.yaml", include_str!("../../../registration/inline-composition.yaml")),
        ("lifecycle-events.yaml", include_str!("../../../registration/lifecycle-events.yaml")),
        ("linking.yaml", include_str!("../../../registration/linking.yaml")),
        ("loop.yaml", include_str!("../../../registration/loop.yaml")),
        ("seq.yaml", include_str!("../../../registration/seq.yaml")),
        ("sequence-document.yaml", include_str!("../../../registration/sequence-document.yaml")),
        ("sequence.yaml", include_str!("../../../registration/sequence.yaml")),
        ("shared.yaml", include_str!("../../../registration/shared.yaml")),
    ];
    let installed = Installed::new(files);
    let kind = Route::Kind("sequence".into());
    for valid in [
        "kind: sequence\nsequence: []\n",
        "kind: sequence\nsequence: [plan, 2, true]\n",
        "kind: sequence\n$schema:\n    spec: string\nplan: \"{{ spec }}\"\nsequence:\n    - name: plan\n      prompt: ./plan.md\n",
    ] {
        assert_eq!(installed.yaml_problems(valid), (kind.clone(), Vec::new()), "{valid}");
    }
    assert_eq!(installed.yaml_problems("kind: sequence\n"), (kind.clone(), vec![String::new()]), "`sequence` is required");
    assert_eq!(installed.yaml_problems("kind: sequence\nsequence: ./steps.yaml\n"), (kind.clone(), vec!["/sequence".to_string()]));
    assert_eq!(installed.yaml_problems("kind: sequence\nsequence:\n    - name: { a: 1 }\n"), (kind, vec!["/sequence/0/name".to_string()]));
}

#[test]
fn setup_and_config_consume_the_canonical_config_definition() {
    let canonical = include_str!("../../../registration/claudine-config.yaml");
    assert_eq!(claudine::config::schema::CONFIG_SCHEMA_SOURCE, canonical);
    assert_eq!(claudine::config::schema::CONFIG_SCHEMA_PATH, "claudine/registration/claudine-config.yaml");

    // Setup writes the scope's projection of `{}`: the canonical definition's
    // defaults, without `permissions`, the Darkmatter block setup transfers on
    // its own.
    let user = claudine::config::schema::ConfigScope::User;
    let prepared = darkmatter::config::schema::prepare_host_config_type(
        canonical,
        claudine::config::schema::CONFIG_SCHEMA_PATH,
        user.config_type(),
    )
    .expect("the canonical user type prepares");
    let mut defaults = prepared.validate_soft(&serde_json::json!({})).expect("an empty file is valid").value.into_owned();
    defaults.as_object_mut().expect("a mapping").remove("permissions");
    let expected = biscuit_file::to_yaml_string(&defaults, biscuit_file::Indentation::default()).expect("a mapping");
    assert_eq!(claudine::config::persist::initial_text(user), expected);
}

#[test]
fn the_embedded_inventory_is_a_fresh_walk_of_the_registration_folder() {
    use claudine::composition::inventory::{self, walk};
    let root = biscuit_test_harness::manifest_dir!().join("../registration");
    let (walked, _) = walk::walk(&root).unwrap_or_else(|errors| panic!("the folder is a valid inventory: {errors:#?}"));
    let fresh: Vec<(&str, &str, inventory::MemberKind, Option<&str>)> = walked
        .iter()
        .map(|member| (member.path.as_str(), member.text.as_str(), member.kind, member.name.as_deref()))
        .collect();
    let embedded: Vec<(&str, &str, inventory::MemberKind, Option<&str>)> =
        inventory::members().iter().map(|member| (member.path, member.text, member.kind, member.name)).collect();
    assert_eq!(embedded, fresh, "the build embedded the folder as it is now, in path order");
    let paths: Vec<&str> = embedded.iter().map(|(path, ..)| *path).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted);
    for member in inventory::members() {
        assert_eq!(inventory::bundle_bytes()[&member.bundle_path()].as_ref(), member.bytes(), "{}", member.path);
    }
}
