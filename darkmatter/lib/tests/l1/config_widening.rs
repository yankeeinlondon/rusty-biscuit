//! `darkmatter::config::widening`: the trusted CI comparison of two commits'
//! permissions, through its public API. Each test commits fixture revisions
//! with git2 (no subprocess, no checkout) and compares them in process.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use darkmatter::config::schema::CONFIG_SCHEMA_SOURCE;
use darkmatter::config::widening::{
    DefaultSchema, PolicyFile, PolicyFormat, ReasonKind, ResolvedPolicy, SchemaFormat, WideningInputs,
    WideningReport, compare_policies, compare_revisions,
};

const FILE: i32 = 0o100_644;
const EXECUTABLE: i32 = 0o100_755;
const LINK: i32 = 0o120_000;

const DM: &str = ".darkmatter.yaml";
const CLAUDINE: &str = ".claudine/config.yaml";
const DM_SCHEMA: &str = "schemas/darkmatter-config.yaml";
const HOST_SCHEMA: &str = "schemas/claudine-config.yaml";

/// A repository with one commit per revision written as a tree, never
/// checked out.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    repo: git2::Repository,
    parent: Option<git2::Oid>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("work");
        std::fs::create_dir_all(&root).unwrap();
        let repo = git2::Repository::init(&root).unwrap();
        Self { _dir: dir, root, repo, parent: None }
    }

    /// Commits exactly `entries` (path, mode, content) and returns the id.
    fn commit(&mut self, entries: &[(&str, i32, &str)]) -> String {
        let tree = self.repo.find_tree(write_tree(&self.repo, entries)).unwrap();
        let signature = git2::Signature::new("Fixture", "fixture@example.com", &git2::Time::new(0, 0)).unwrap();
        let parents: Vec<git2::Commit<'_>> = self.parent.iter().map(|id| self.repo.find_commit(*id).unwrap()).collect();
        let parents: Vec<&git2::Commit<'_>> = parents.iter().collect();
        let id = self.repo.commit(Some("HEAD"), &signature, &signature, "revision", &tree, &parents).unwrap();
        self.parent = Some(id);
        id.to_string()
    }

    /// Commits `base`, then `head`, and compares them.
    fn compare(&mut self, base: &[(&str, i32, &str)], head: &[(&str, i32, &str)]) -> WideningReport {
        self.try_compare(base, head).unwrap_or_else(|error| panic!("the comparison runs: {error}"))
    }

    fn try_compare(&mut self, base: &[(&str, i32, &str)], head: &[(&str, i32, &str)]) -> Result<WideningReport, String> {
        let base = self.commit(base);
        let head = self.commit(head);
        compare_revisions(&self.root, &base, &head, &inputs()).map_err(|error| error.to_string())
    }
}

fn write_tree(repo: &git2::Repository, entries: &[(&str, i32, &str)]) -> git2::Oid {
    let mut builder = repo.treebuilder(None).unwrap();
    let mut folders: BTreeMap<&str, Vec<(&str, i32, &str)>> = BTreeMap::new();
    for &(path, mode, content) in entries {
        match path.split_once('/') {
            Some((folder, rest)) => folders.entry(folder).or_default().push((rest, mode, content)),
            None => {
                let blob = repo.blob(content.as_bytes()).unwrap();
                builder.insert(path, blob, mode).unwrap();
            }
        }
    }
    for (folder, nested) in folders {
        let tree = write_tree(repo, &nested);
        builder.insert(folder, tree, 0o040_000).unwrap();
    }
    builder.write().unwrap()
}

fn inputs() -> WideningInputs {
    WideningInputs {
        policy_files: vec![
            PolicyFile { path: DM.into(), format: PolicyFormat::DarkmatterConfig },
            PolicyFile { path: CLAUDINE.into(), format: PolicyFormat::PermissionsKey },
        ],
        default_schemas: vec![
            DefaultSchema { path: DM_SCHEMA.into(), format: SchemaFormat::DarkmatterConfig },
            DefaultSchema { path: HOST_SCHEMA.into(), format: SchemaFormat::HostConfig },
        ],
        checker_inputs: vec!["checker/".into(), "ci.yml".into()],
    }
}

fn kinds(report: &WideningReport) -> Vec<(String, ReasonKind)> {
    report.reasons.iter().map(|reason| (reason.input.clone(), reason.kind)).collect()
}

fn assert_no_widening(report: &WideningReport) {
    assert!(!report.potential_widening && report.reasons.is_empty(), "{:#?}", report.reasons);
}

fn after<'a>(report: &'a WideningReport, input: &str) -> &'a ResolvedPolicy {
    &report.policies.iter().find(|policy| policy.input == input).expect("the policy is reported").after
}

fn block(body: &str) -> String {
    format!("permissions:\n{body}")
}

/// The block every absent file defaults to, written out.
const REPOSITORY_DEFAULT: &str = "permissions:\n  file-read: [\"&**/*.txt\", \"&**/*.text\", \"&**/*.md\", \"&**/*.csv\", \"&**/*.tsv\", \"&**/*.json\", \"&**/*.json5\", \"&**/*.yaml\", \"&**/*.yml\", \"&**/*.toml\"]\n  shell-commands: []\n  network: deny\n";

#[test]
fn unrelated_changes_compare_equal_and_name_no_input() {
    let mut fixture = Fixture::new();
    let config = block("  shell-commands: [prefix git]\n");
    let report = fixture.compare(&[(DM, FILE, &config), ("a.txt", FILE, "1")], &[(DM, FILE, &config), ("a.txt", FILE, "2")]);
    assert_no_widening(&report);
    assert!(report.changed_inputs.is_empty(), "{:?}", report.changed_inputs);
    assert_eq!(after(&report, DM).shell_commands, ["prefix git"]);
    assert_eq!(after(&report, DM).block_file.as_deref(), Some(DM));
}

#[test]
fn an_inline_block_moved_to_a_reference_compares_its_resolved_block() {
    let mut fixture = Fixture::new();
    let body = "  shell-commands: [prefix git, \"!exact git push\"]\n  network: allow\n";
    let report = fixture.compare(
        &[(DM, FILE, &block(body))],
        &[(DM, FILE, "permissions: ./policies/shared.yaml\n"), ("policies/shared.yaml", FILE, &block(body))],
    );
    assert_no_widening(&report);
    assert_eq!(report.changed_inputs, [DM, "policies/shared.yaml"]);
    assert_eq!(after(&report, DM).block_file.as_deref(), Some("policies/shared.yaml"));

    // The reverse move, with an equivalent spelling of the same entries.
    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[(DM, FILE, "permissions: ./policies/shared.yaml\n"), ("policies/shared.yaml", FILE, &block(body))],
        &[(DM, FILE, &block("  network: allow\n  shell-commands: [\"!exact  git push\", prefix git, prefix git]\n"))],
    );
    assert_no_widening(&report);
}

#[test]
fn a_deleted_file_or_property_restores_the_default_and_is_compared_against_it() {
    // An explicit default, then no file: equal.
    let mut fixture = Fixture::new();
    assert_no_widening(&fixture.compare(&[(DM, FILE, REPOSITORY_DEFAULT)], &[]));

    // A narrower block, then no file: the restored default widens.
    let mut fixture = Fixture::new();
    let report = fixture.compare(&[(DM, FILE, &block("  file-read: []\n"))], &[]);
    assert!(report.potential_widening);
    assert!(report.reasons.iter().all(|reason| reason.kind == ReasonKind::AddedAllow && reason.input == DM));
    assert_eq!(report.reasons.len(), 10, "one reason per restored default entry: {:#?}", report.reasons);
    assert_eq!(report.changed_inputs, [DM]);

    // The property alone deleted, with the rest of the file kept.
    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[(DM, FILE, &format!("register: []\n{}", block("  file-read: [\"&docs/**/*.md\"]\n")))],
        &[(DM, FILE, "register: []\n")],
    );
    assert!(kinds(&report).iter().all(|(_, kind)| *kind == ReasonKind::AddedAllow) && report.potential_widening);
}

#[test]
fn a_same_length_replacement_is_a_widening_however_it_is_spelled() {
    for (before, after_list) in [
        ("[prefix git]", "[prefix gh]"),
        ("[\"exact git status\"]", "[\"template git status {{ string }}\"]"),
        ("[\"template git add {{ string }}\"]", "[\"template git rm {{ string }}\"]"),
    ] {
        let mut fixture = Fixture::new();
        let report = fixture.compare(
            &[(DM, FILE, &block(&format!("  shell-commands: {before}\n")))],
            &[(DM, FILE, &block(&format!("  shell-commands: {after_list}\n")))],
        );
        assert_eq!(kinds(&report), [(DM.to_string(), ReasonKind::AddedAllow)], "{before} → {after_list}");
    }
    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[(DM, FILE, &block("  file-read: [\"&docs/*.md\"]\n"))],
        &[(DM, FILE, &block("  file-read: [\"&docs/**/*.md\"]\n"))],
    );
    assert_eq!(kinds(&report), [(DM.to_string(), ReasonKind::AddedAllow)], "a broader glob is never proven contained");
}

#[test]
fn exclusions_narrow_when_added_and_widen_when_removed() {
    let with = block("  shell-commands: [prefix git, \"!exact git push\"]\n  file-read: [\"&**/*.md\", \"!&secrets/**\"]\n");
    let without = block("  shell-commands: [prefix git]\n  file-read: [\"&**/*.md\"]\n");
    let mut fixture = Fixture::new();
    assert_no_widening(&fixture.compare(&[(DM, FILE, &without)], &[(DM, FILE, &with)]));
    let mut fixture = Fixture::new();
    let report = fixture.compare(&[(DM, FILE, &with)], &[(DM, FILE, &without)]);
    assert_eq!(
        kinds(&report),
        [(DM.to_string(), ReasonKind::RemovedExclusion), (DM.to_string(), ReasonKind::RemovedExclusion)]
    );
    assert!(report.reasons[0].detail.contains("!&secrets/**"), "{}", report.reasons[0].detail);
}

#[test]
fn a_mixed_change_widens_through_its_addition_and_a_removal_alone_narrows() {
    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[(DM, FILE, &block("  shell-commands: [prefix ls, prefix cat, prefix git]\n"))],
        &[(DM, FILE, &block("  shell-commands: [prefix ls, prefix rm]\n"))],
    );
    assert_eq!(kinds(&report), [(DM.to_string(), ReasonKind::AddedAllow)]);
    assert!(report.reasons[0].detail.contains("prefix rm"));

    let mut fixture = Fixture::new();
    assert_no_widening(&fixture.compare(
        &[(DM, FILE, &block("  shell-commands: [prefix ls, prefix cat]\n"))],
        &[(DM, FILE, &block("  shell-commands: [prefix ls]\n"))],
    ));
}

#[test]
fn network_widens_only_from_deny_to_allow() {
    let deny = block("  network: deny\n");
    let allow = block("  network: allow\n");
    let mut fixture = Fixture::new();
    assert_eq!(kinds(&fixture.compare(&[(DM, FILE, &deny)], &[(DM, FILE, &allow)])), [(DM.to_string(), ReasonKind::NetworkAllowed)]);
    let mut fixture = Fixture::new();
    assert_no_widening(&fixture.compare(&[(DM, FILE, &allow)], &[(DM, FILE, &deny)]));
    // Absent is the default, `deny`.
    let mut fixture = Fixture::new();
    assert_eq!(kinds(&fixture.compare(&[], &[(DM, FILE, &allow)])), [(DM.to_string(), ReasonKind::NetworkAllowed)]);
}

#[test]
fn a_changed_reference_target_or_link_is_read_in_each_commit() {
    let narrow = block("  shell-commands: [prefix ls]\n");
    let broad = block("  shell-commands: [prefix ls, prefix curl]\n");
    let reference = "permissions: ./policy.yaml\n";

    // The reference is unchanged; only its target is edited.
    let mut fixture = Fixture::new();
    let report = fixture.compare(&[(DM, FILE, reference), ("policy.yaml", FILE, &narrow)], &[(DM, FILE, reference), ("policy.yaml", FILE, &broad)]);
    assert_eq!(kinds(&report), [(DM.to_string(), ReasonKind::AddedAllow)]);
    assert_eq!(report.changed_inputs, ["policy.yaml"]);

    // A committed link is redirected to a broader block.
    let mut fixture = Fixture::new();
    let files = |link: &'static str| {
        vec![(DM, FILE, reference), ("policy.yaml", LINK, link), ("a.yaml", FILE, narrow.as_str()), ("b.yaml", FILE, broad.as_str())]
    };
    let report = fixture.compare(&files("a.yaml"), &files("b.yaml"));
    assert_eq!(kinds(&report), [(DM.to_string(), ReasonKind::AddedAllow)]);
    assert_eq!(after(&report, DM).block_file.as_deref(), Some("b.yaml"));
    assert_eq!(report.changed_inputs, ["policy.yaml"], "the redirected link; neither target was edited");

    // A link out of the repository is an error, never a default.
    let mut fixture = Fixture::new();
    let error = fixture
        .try_compare(&files("a.yaml"), &[(DM, FILE, reference), ("policy.yaml", LINK, "../outside.yaml")])
        .unwrap_err();
    assert!(error.contains("head commit") && error.contains("outside the repository"), "{error}");
}

#[test]
fn claudines_configuration_is_judged_by_its_permissions_alone() {
    let claudine = |permissions: &str| format!("default-provider: claude\nloop:\n  max: 3\n{permissions}");
    // Claudine's own settings change; its block does not.
    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[(CLAUDINE, FILE, &claudine(&block("  shell-commands: [prefix ls]\n")))],
        &[(CLAUDINE, FILE, &claudine(&block("  shell-commands: [prefix ls]\n")).replace("claude", "codex"))],
    );
    assert_no_widening(&report);
    assert_eq!(report.changed_inputs, [CLAUDINE]);

    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[(CLAUDINE, FILE, &claudine(&block("  shell-commands: [prefix ls]\n")))],
        &[(CLAUDINE, FILE, &claudine(&block("  shell-commands: [prefix ls, prefix make]\n")))],
    );
    assert_eq!(kinds(&report), [(CLAUDINE.to_string(), ReasonKind::AddedAllow)]);

    // Its block may be a reference too, followed in the commit.
    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[(CLAUDINE, FILE, &claudine("permissions: ./shared.yaml\n")), (".claudine/shared.yaml", FILE, &block("  network: deny\n"))],
        &[(CLAUDINE, FILE, &claudine("permissions: ./shared.yaml\n")), (".claudine/shared.yaml", FILE, &block("  network: allow\n"))],
    );
    assert_eq!(kinds(&report), [(CLAUDINE.to_string(), ReasonKind::NetworkAllowed)]);
}

#[test]
fn a_file_or_reference_that_cannot_be_read_fails_the_check() {
    for (head, needle) in [
        (vec![(DM, FILE, "permissions: [unclosed\n")], DM),
        (vec![(DM, FILE, "permissions: ./missing.yaml\n")], "does not exist"),
        (vec![(DM, FILE, "not-a-setting: 1\n")], "not-a-setting"),
        (vec![(DM, FILE, &block("  shell-commands: 3\n"))], "shell-commands"),
        (vec![(CLAUDINE, FILE, "- a list\n")], "not a mapping"),
        (vec![(DM, LINK, "elsewhere.yaml")], "symbolic link"),
    ] {
        let mut fixture = Fixture::new();
        let error = fixture.try_compare(&[], &head).unwrap_err();
        assert!(error.contains("head commit") && error.contains(needle), "{needle}: {error}");
    }

    let fixture = Fixture::new();
    for (base, head) in [("HEAD", "HEAD"), (&*"0".repeat(40), &*"1".repeat(40))] {
        let error = compare_revisions(&fixture.root, base, head, &inputs()).unwrap_err();
        assert!(error.to_string().contains("base commit"), "{error}");
    }
    let error = compare_revisions(Path::new("/nonexistent/widening/repository"), "a", "b", &inputs());
    assert!(error.is_err());
}

#[test]
fn hostile_candidate_files_are_read_as_data_and_never_run() {
    let mut fixture = Fixture::new();
    let sentinel = fixture.root.join("ran");
    let script = format!("#!/bin/sh\ntouch '{}'\n", sentinel.display());
    let head = [
        ("build.rs", EXECUTABLE, script.as_str()),
        (".cargo/config.toml", FILE, "[build]\nrustc-wrapper = \"./build.rs\"\n"),
        ("checker/hook.sh", EXECUTABLE, script.as_str()),
        (DM, FILE, "permissions: ./build.rs\n"),
    ];
    let error = fixture.try_compare(&[], &head).unwrap_err();
    assert!(error.contains("build.rs") && error.contains("not a YAML mapping"), "{error}");

    let report = fixture.compare(&[], &head[..3]);
    assert_eq!(kinds(&report), [("checker/hook.sh".to_string(), ReasonKind::CheckerInputChanged)]);
    assert!(!sentinel.exists(), "no candidate file ran");
    assert!(!fixture.root.join("build.rs").exists(), "nothing was checked out");
}

#[test]
fn a_change_to_a_checker_input_is_a_reason() {
    let mut fixture = Fixture::new();
    let report = fixture.compare(
        &[("ci.yml", FILE, "jobs: {}\n"), ("checker/a.rs", FILE, "1"), ("checker/gone.rs", FILE, "1")],
        &[("ci.yml", FILE, "jobs: {skip: true}\n"), ("checker/a.rs", FILE, "1"), ("checker/new.rs", FILE, "1")],
    );
    let mut changed = kinds(&report);
    changed.sort();
    let expected: Vec<(String, ReasonKind)> = ["checker/gone.rs", "checker/new.rs", "ci.yml"]
        .into_iter()
        .map(|path| (path.to_string(), ReasonKind::CheckerInputChanged))
        .collect();
    assert_eq!(changed, expected);
    assert_eq!(report.changed_inputs, ["checker/gone.rs", "checker/new.rs", "ci.yml"]);
}

/// The shipped schema with one of its default lists edited.
fn edited_schema(from: &str, to: &str) -> String {
    assert!(CONFIG_SCHEMA_SOURCE.contains(from), "the shipped schema still spells `{from}`");
    CONFIG_SCHEMA_SOURCE.replace(from, to)
}

#[test]
fn a_changed_shipped_default_is_compared_with_the_trusted_parser() {
    // The repository's block default and its entry default both gain `.rs`.
    let wider = edited_schema("\"&**/*.toml\"]", "\"&**/*.toml\", \"&**/*.rs\"]");
    let mut fixture = Fixture::new();
    let report = fixture.compare(&[(DM_SCHEMA, FILE, CONFIG_SCHEMA_SOURCE)], &[(DM_SCHEMA, FILE, &wider)]);
    let mut scopes: Vec<&str> = report.reasons.iter().map(|reason| reason.detail.as_str()).collect();
    scopes.sort_unstable();
    assert_eq!(kinds(&report), vec![(DM_SCHEMA.to_string(), ReasonKind::AddedAllow); 2], "{scopes:?}");
    let reported: Vec<&str> = report.policies.iter().filter(|policy| policy.input == DM_SCHEMA).map(|policy| policy.scope).collect();
    assert_eq!(reported, ["user-block-default", "user-entry-defaults", "repository-block-default", "repository-entry-defaults"]);

    // Only the entry default (what a block omitting `file-read` takes).
    let entry_only = CONFIG_SCHEMA_SOURCE.replacen("\"&**/*.toml\"]", "\"&**/*.toml\", \"&**/*.rs\"]", 1);
    assert_ne!(entry_only, wider);
    let mut fixture = Fixture::new();
    let report = fixture.compare(&[(DM_SCHEMA, FILE, CONFIG_SCHEMA_SOURCE)], &[(DM_SCHEMA, FILE, &entry_only)]);
    assert_eq!(kinds(&report), [(DM_SCHEMA.to_string(), ReasonKind::AddedAllow)]);
    let widened = report.policies.iter().find(|policy| policy.after.file_read.contains(&"&**/*.rs".to_string())).unwrap();
    assert_eq!(widened.scope, "repository-entry-defaults");

    // A narrower default and a reworded description are not widenings.
    let narrower = edited_schema(", \"&**/*.toml\"]", "]");
    let reworded = edited_schema("Default: none, so every command prompts.", "Default: none.");
    for edited in [narrower, reworded] {
        let mut fixture = Fixture::new();
        assert_no_widening(&fixture.compare(&[(DM_SCHEMA, FILE, CONFIG_SCHEMA_SOURCE)], &[(DM_SCHEMA, FILE, &edited)]));
    }

    // `network` defaulting to `allow`.
    let allow = edited_schema("default(deny))", "default(allow))");
    let mut fixture = Fixture::new();
    let report = fixture.compare(&[(DM_SCHEMA, FILE, CONFIG_SCHEMA_SOURCE)], &[(DM_SCHEMA, FILE, &allow)]);
    assert!(report.reasons.iter().any(|reason| reason.kind == ReasonKind::NetworkAllowed), "{:#?}", report.reasons);
}

#[test]
fn a_default_schema_the_trusted_code_cannot_read_is_potential_widening() {
    for head in [
        Some("types: [this is not a schema\n".to_string()),
        Some(edited_schema("repository-config:", "renamed-config:")),
        None,
    ] {
        let mut fixture = Fixture::new();
        let head_files: Vec<(&str, i32, &str)> = head.iter().map(|text| (DM_SCHEMA, FILE, text.as_str())).collect();
        let report = fixture.compare(&[(DM_SCHEMA, FILE, CONFIG_SCHEMA_SOURCE)], &head_files);
        assert!(report.potential_widening);
        assert!(
            report.reasons.iter().all(|reason| reason.kind == ReasonKind::DefaultsNotUnderstood),
            "{:#?}",
            report.reasons
        );
        assert!(report.reasons.iter().any(|reason| reason.detail.contains("at the head commit")));
    }
}

#[test]
fn claudines_shipped_schema_default_is_compared_through_its_import() {
    let shipped = include_str!("../../../../claudine/registration/claudine-config.yaml");
    let from = "\"&**/*.toml\"], \"shell-commands\": []";
    assert!(shipped.contains(from), "Claudine's schema still spells its repository default");
    let wider = shipped.replace(from, "\"&**/*.toml\"], \"shell-commands\": [\"prefix git\"]");
    let mut fixture = Fixture::new();
    let report = fixture.compare(&[(HOST_SCHEMA, FILE, shipped)], &[(HOST_SCHEMA, FILE, &wider)]);
    assert_eq!(kinds(&report), [(HOST_SCHEMA.to_string(), ReasonKind::AddedAllow)], "{:#?}", report.policies);
    let mut fixture = Fixture::new();
    assert_no_widening(&fixture.compare(&[(HOST_SCHEMA, FILE, &wider)], &[(HOST_SCHEMA, FILE, shipped)]));
}

#[test]
fn the_comparator_is_pure_over_resolved_blocks() {
    let policy = |file_read: &[&str], shell: &[&str], network: &'static str| ResolvedPolicy {
        block_file: None,
        file_read: file_read.iter().map(|entry| (*entry).to_string()).collect(),
        shell_commands: shell.iter().map(|entry| (*entry).to_string()).collect(),
        network,
    };
    let base = policy(&["&a/**", "!&a/secret"], &["prefix ls"], "allow");
    assert!(compare_policies("x", &base, &base).is_empty());
    assert!(compare_policies("x", &base, &policy(&["!&a/secret", "!&b"], &[], "deny")).is_empty());
    let widened = compare_policies("x", &base, &policy(&["&a/**", "&b/**"], &["prefix ls"], "allow"));
    let kinds: Vec<ReasonKind> = widened.iter().map(|reason| reason.kind).collect();
    assert_eq!(kinds, [ReasonKind::AddedAllow, ReasonKind::RemovedExclusion]);
}

/// The real inputs file: every list non-empty and every path committed in
/// this repository's tree at the time it was written.
#[test]
fn the_repository_inputs_file_parses_and_names_the_shipped_inputs() {
    let text = include_str!("../../../../.github/ci/permission-inputs.json");
    let inputs = WideningInputs::from_json(text).unwrap();
    let policy_paths: Vec<&str> = inputs.policy_files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(policy_paths, [".darkmatter.yaml", ".claudine/config.yaml"]);
    let schema_paths: Vec<&str> = inputs.default_schemas.iter().map(|schema| schema.path.as_str()).collect();
    assert_eq!(schema_paths, ["darkmatter/schemas/darkmatter-config.yaml", "claudine/registration/claudine-config.yaml"]);
    for required in [
        ".github/ci/permission-inputs.json",
        ".github/workflows/ci.yml",
        "darkmatter/lib/examples/permission_widening.rs",
        "darkmatter/lib/src/config/",
        "scripts/ci/permission_scope.py",
    ] {
        assert!(inputs.checker_inputs.iter().any(|input| input == required), "{required}");
    }
}

/// The Input Robustness Matrix for the inputs file's one loader: each field
/// shape once, plus the document shapes.
#[test]
fn the_inputs_file_rejects_every_malformed_shape() {
    let control = r#"{"policy-files": [{"path": ".darkmatter.yaml", "format": "darkmatter-config"}],
        "default-schemas": [{"path": "s.yaml", "format": "host-config"}], "checker-inputs": ["c/"]}"#;
    assert!(WideningInputs::from_json(control).is_ok(), "control");
    let field = |key: &str, value: &str| {
        let mut document: serde_json::Value = serde_json::from_str(control).unwrap();
        if value == "<absent>" {
            document.as_object_mut().unwrap().remove(key);
        } else {
            document[key] = serde_json::from_str(value).unwrap();
        }
        document.to_string()
    };
    for key in ["policy-files", "default-schemas", "checker-inputs"] {
        for shape in ["<absent>", "null", "123", "[]", "[123]"] {
            assert!(WideningInputs::from_json(&field(key, shape)).is_err(), "{key}: {shape}");
        }
    }
    for (key, element) in [
        ("checker-inputs", r#"["c/", 123]"#),
        ("checker-inputs", r#"[""]"#),
        ("checker-inputs", r#"["../outside"]"#),
        ("checker-inputs", r#"["a\\b"]"#),
        ("policy-files", r#"[{"path": ".darkmatter.yaml"}]"#),
        ("policy-files", r#"[{"path": ".darkmatter.yaml", "format": "toml"}]"#),
        ("policy-files", r#"[{"path": null, "format": "darkmatter-config"}]"#),
        ("default-schemas", r#"[{"path": "s.yaml", "format": "host-config", "extra": 1}]"#),
    ] {
        assert!(WideningInputs::from_json(&field(key, element)).is_err(), "{key}: {element}");
    }
    for document in [
        control.replacen('{', r#"{"checker-inputs": ["d/"], "#, 1),
        format!("{control} {{}}"),
        control.replacen('{', r#"{"unknown": 1, "#, 1),
        "[]".to_string(),
        String::new(),
    ] {
        assert!(WideningInputs::from_json(&document).is_err(), "{document}");
    }
}

/// The CI permission report: `.github/ci/permission-report.md`, composed with
/// data shaped as `scripts/ci/permission_scope.py decide` writes it (the
/// `permission_widening --render` example's path), states each change in
/// words and shows every repository value verbatim, never as template syntax.
#[test]
fn the_permission_report_template_states_each_change_verbatim() {
    use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};
    use darkmatter::markdown::literal_token::encode;
    use darkmatter::markdown::{Markdown, MergeStrategy};
    use serde_json::json;

    let template = include_str!("../../../../.github/ci/permission-report.md");
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("permission-report.md");
    std::fs::write(&path, template).expect("the template copy is written");
    let code = |text: &str| encode(&format!("`{text}`"));
    let rule = |form: &str, command: &str| json!({ "form": form, "code": code(command) });
    let data = json!({
        "verdict": "widens",
        "sources": [code(".darkmatter.yaml")],
        "policies": [{
            "title": code(".darkmatter.yaml"),
            "scope_note": "",
            "shell": {
                "changed": true, "before_count": 12, "before_default": false,
                "added": [rule("prefix", "git status"), rule("exact", "make tidy"), rule("template", "git add {{ string }}")],
                "removed": [rule("prefix", "ls")], "excluded": [], "unexcluded": [],
            },
            "file_read": {
                "changed": true, "before_count": 3, "before_default": true,
                "added": [code("&docs/specs/**")], "removed": [], "excluded": [], "unexcluded": [],
            },
            "network": { "changed": true, "before": "deny", "after": "allow" },
        }],
        "unreadable": [],
    });

    let mut document: Markdown = template.to_string().into();
    document
        .fm_merge_with(json!({ "report": data }), MergeStrategy::PreferExternal)
        .expect("the data merges into the template's frontmatter");
    let snapshot = RequestSnapshot::new(dir.path()).with_home(None);
    let request = ComposeRequest::prepare(ComposeOptions::new().with_source_file(&path), &snapshot)
        .expect("a request for the template");
    let (composed, report) = document.compose_with(&request).expect("the template composes");
    assert!(report.warnings.is_empty(), "a warning fails the CI render: {:?}", report.warnings);
    let text = composed.content();
    for expected in [
        "_This pull request **widens** what Darkmatter and Claudine may do without asking",
        "### `.darkmatter.yaml`",
        "Before this change there were **12** shell rules.",
        "- Commands **starting with** `git status` will run without asking",
        "- The `make tidy` command (exact command, no variation) will run without asking",
        "- Commands **shaped like** `git add {{ string }}` will run without asking",
        "Removed, so these will now ask first:\n\n- Commands **starting with** `ls`",
        "Before this change these were the defaults",
        "- Files matching `&docs/specs/**`",
        "- Changed from **no URLs** to **every URL** fetched without asking",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
    for absent in ["narrows**", "Could not be compared", "::block", "::map", "::end-"] {
        assert!(!text.contains(absent), "unexpected {absent:?} in:\n{text}");
    }
}
