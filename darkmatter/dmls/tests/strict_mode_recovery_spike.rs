//! R12 spike: characterize the current server through LSP, then test a small
//! proposed publication model separately. Passing does not mean R12 is built.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const DOCUMENT: &str = "---\nprompt: hello\npolicy: ok\n\n---\n\n[broken](missing.md)\n";
const DIRECT_DOCUMENT: &str = "---\nprompt: hello\npolicy: ok\n$schema: ./schemas/payload.yaml\n---\n\n[broken](missing.md)\n";
const TRIGGER: &str = "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: payload.yaml\n";
const PAYLOAD: &str = "$schema:\n  policy: policy@../../shared.yaml\n  required_probe: string(required) -> Spike completion marker\n";

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify("textDocument/didOpen", json!({
        "textDocument": {"uri": uri, "languageId": "markdown", "version": 1, "text": text}
    }));
}

fn changed(fixture: &mut LspFixture<'_>, paths: &[(&Path, u8)]) {
    fixture.notify("workspace/didChangeWatchedFiles", json!({
        "changes": paths.iter().map(|(path, kind)| json!({"uri": uri(path), "type": kind})).collect::<Vec<_>>()
    }));
    fixture.flush_server();
}

fn has_required(diagnostics: &[Value]) -> bool {
    diagnostics.iter().any(|d| d["code"] == "dm.schema.missing_required"
        && d["message"].as_str().is_some_and(|m| m.contains("required_probe")))
}

fn hover(fixture: &mut LspFixture<'_>, uri: &str) -> String {
    let response = fixture.request("textDocument/hover", json!({
        "textDocument": {"uri": uri}, "position": {"line": 2, "character": 2}
    }));
    assert!(response.error.is_none(), "{response:?}");
    response.result.unwrap_or(Value::Null).to_string()
}

fn completion(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    let response = fixture.request("textDocument/completion", json!({
        "textDocument": {"uri": uri}, "position": {"line": 2, "character": 0}
    }));
    assert!(response.error.is_none(), "{response:?}");
    response.result.unwrap().as_array().unwrap().clone()
}

fn offers_probe(items: &[Value]) -> bool {
    items.iter().any(|item| item["label"] == "required_probe")
}

#[test]
fn spike_recovery_protocol_shared_import_failure_repair_and_directory_recreation() {
    #[cfg(feature = "effects-instrumentation")]
    let effects_before = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    let schemas = root.join("payments/schemas");
    std::fs::create_dir_all(&schemas).unwrap();
    std::fs::create_dir(root.join("shipping")).unwrap();
    std::fs::write(root.join(".dmls.toml"), "[schema]\nstrict = true\n").unwrap();
    let shared = root.join("shared.yaml");
    let write_shared = |description: &str| {
        std::fs::write(&shared, format!("kind: schema\ntypes:\n  policy: string -> {description}\n")).unwrap();
    };
    write_shared("SPIKE_OLD_TYPE");
    let trigger = schemas.join("prompt.trigger.yaml");
    let payload = schemas.join("payload.yaml");
    std::fs::write(&trigger, TRIGGER).unwrap();
    std::fs::write(&payload, PAYLOAD).unwrap();
    let paths = [root.join("payments/one.md"), root.join("payments/two.md"), root.join("shipping/one.md")];
    let documents = [DOCUMENT, DIRECT_DOCUMENT, DOCUMENT];
    for (path, document) in paths.iter().zip(documents) { std::fs::write(path, document).unwrap(); }
    let uris = paths.iter().map(|path| uri(path)).collect::<Vec<_>>();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(json!({
        "processId": null,
        "capabilities": {"workspace": {"didChangeWatchedFiles": {"dynamicRegistration": true}},
                         "window": {"workDoneProgress": true}},
        "workspaceFolders": [{"uri": url::Url::from_directory_path(root).unwrap().as_str(), "name": "spike"}]
    }));
    fixture.wait_for_startup_complete();
    let mut independent_codes = BTreeSet::new();
    for (index, uri) in uris.iter().enumerate() {
        open(&fixture, uri, documents[index]);
        let diagnostics = fixture.wait_for_diagnostics(uri);
        assert_eq!(has_required(&diagnostics), index < 2, "initial {uri}: {diagnostics:?}");
        let items = completion(&mut fixture, uri);
        assert_eq!(offers_probe(&items), index == 1, "current trigger-only completion gap: {items:?}");
        assert_eq!(hover(&mut fixture, uri).contains("SPIKE_OLD_TYPE"), index == 1);
        if index == 2 {
            independent_codes = diagnostics.iter().filter_map(|d| d["code"].as_str().map(str::to_owned)).collect();
            assert!(!independent_codes.is_empty(), "independent diagnostic positive control");
        }
    }
    fixture.flush_server();
    for uri in &uris { while fixture.take_buffered_diagnostics(uri).is_some() {} }

    std::fs::remove_file(&shared).unwrap();
    changed(&mut fixture, &[(&shared, 3)]);
    let failed = fixture.wait_for_diagnostics(&uri(&trigger));
    assert!(failed.iter().any(|d| d["code"] == "dm.schema.prepare"), "{failed:?}");
    for (index, uri) in uris[..2].iter().enumerate() {
        let diagnostics = fixture.wait_for_diagnostics(uri);
        assert_eq!(has_required(&diagnostics), index == 0, "failure route {index}: {diagnostics:?}");
        let codes = diagnostics.iter().filter_map(|d| d["code"].as_str().map(str::to_owned)).collect::<BTreeSet<_>>();
        assert!(independent_codes.is_subset(&codes), "independent checks must survive in affected documents");
        let items = completion(&mut fixture, uri);
        assert!(!offers_probe(&items));
        assert!(items.iter().any(|item| item["label"] == "title"), "baseline assistance must remain available");
        assert!(!hover(&mut fixture, uri).contains("SPIKE_OLD_TYPE"));
    }
    let independent = fixture.wait_for_diagnostics(&uris[2]);
    let codes = independent.iter().filter_map(|d| d["code"].as_str().map(str::to_owned)).collect::<BTreeSet<_>>();
    assert_eq!(codes, independent_codes);
    assert!(!offers_probe(&completion(&mut fixture, &uris[2])));

    write_shared("SPIKE_REPAIRED_TYPE");
    changed(&mut fixture, &[(&shared, 1)]);
    assert!(fixture.wait_for_diagnostics(&uri(&trigger)).is_empty());
    for (index, uri) in uris[..2].iter().enumerate() {
        assert!(has_required(&fixture.wait_for_diagnostics(uri)));
        let text = hover(&mut fixture, uri);
        assert_eq!(text.contains("SPIKE_REPAIRED_TYPE"), index == 1, "{text}");
        assert!(!text.contains("SPIKE_OLD_TYPE"));
        assert_eq!(offers_probe(&completion(&mut fixture, uri)), index == 1);
    }
    fixture.flush_server();
    for uri in &uris { while fixture.take_buffered_diagnostics(uri).is_some() {} }

    std::fs::remove_dir_all(&schemas).unwrap();
    changed(&mut fixture, &[(&trigger, 3), (&payload, 3)]);
    for uri in &uris[..2] {
        assert!(!has_required(&fixture.wait_for_diagnostics(uri)));
        assert!(!offers_probe(&completion(&mut fixture, uri)));
        assert!(!hover(&mut fixture, uri).contains("SPIKE_REPAIRED_TYPE"));
    }
    std::fs::create_dir(&schemas).unwrap();
    std::fs::write(&trigger, TRIGGER).unwrap();
    std::fs::write(&payload, PAYLOAD).unwrap();
    changed(&mut fixture, &[(&trigger, 1), (&payload, 1)]);
    for (index, uri) in uris[..2].iter().enumerate() {
        assert!(has_required(&fixture.wait_for_diagnostics(uri)));
        assert_eq!(offers_probe(&completion(&mut fixture, uri)), index == 1);
        assert_eq!(hover(&mut fixture, uri).contains("SPIKE_REPAIRED_TYPE"), index == 1);
    }
    assert!(!offers_probe(&completion(&mut fixture, &uris[2])));
    fixture.shutdown();
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(effects_before, (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()));
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Assistance {
    diagnostics: Vec<&'static str>,
    hover: Option<&'static str>,
    completions: Vec<&'static str>,
}

#[derive(Clone)]
struct Contribution {
    dependencies: BTreeSet<&'static str>,
    assistance: Assistance,
}

#[derive(Default)]
struct PublicationModel {
    generations: BTreeMap<&'static str, u64>,
    pending: BTreeSet<&'static str>,
    failures: BTreeMap<&'static str, &'static str>,
    contributions: Vec<Contribution>,
}

impl PublicationModel {
    fn invalidate(&mut self, source: &'static str) -> u64 {
        let generation = self.generations.entry(source).or_default();
        *generation += 1;
        self.pending.insert(source);
        *generation
    }

    fn finish(&mut self, generation: u64, source: &'static str, result: Result<Vec<Contribution>, &'static str>) -> bool {
        if self.generations.get(source) != Some(&generation) { return false; }
        self.pending.remove(source);
        // Invalid definitions are removed, not retained as a last-good fallback.
        self.contributions.retain(|item| !item.dependencies.contains(source));
        match result {
            Ok(items) => { self.failures.remove(source); self.contributions.extend(items); }
            Err(cause) => { self.failures.insert(source, cause); }
        }
        true
    }

    fn assistance(&self) -> Assistance {
        let mut result = Assistance { diagnostics: vec![], hover: None, completions: vec![] };
        for item in &self.contributions {
            if item.dependencies.iter().any(|source| self.pending.contains(source) || self.failures.contains_key(source)) { continue; }
            result.diagnostics.extend(&item.assistance.diagnostics);
            result.hover = item.assistance.hover.or(result.hover);
            result.completions.extend(&item.assistance.completions);
        }
        if !self.pending.is_empty() || !self.failures.is_empty() { result.diagnostics.push("validation incomplete"); }
        result
    }
}

fn contribution(source: &'static str, label: &'static str) -> Contribution {
    Contribution {
        dependencies: BTreeSet::from([source]),
        assistance: Assistance { diagnostics: vec![label], hover: Some(label), completions: vec![label] },
    }
}

#[test]
fn spike_recovery_model_rejects_late_results_and_suppresses_all_dependent_surfaces() {
    let mut model = PublicationModel::default();
    let independent = Contribution {
        dependencies: BTreeSet::new(),
        assistance: Assistance { diagnostics: vec!["broken link"], hover: None, completions: vec!["baseline key"] },
    };
    model.contributions = vec![independent, contribution("shared import", "old")];
    let delayed = model.invalidate("shared import");
    assert_eq!(model.assistance(), Assistance {
        diagnostics: vec!["broken link", "validation incomplete"], hover: None, completions: vec!["baseline key"]
    });
    let newer = model.invalidate("shared import");
    assert!(model.finish(newer, "shared import", Err("missing required import")));
    assert!(!model.finish(delayed, "shared import", Ok(vec![contribution("shared import", "old")])));
    assert_eq!(model.failures["shared import"], "missing required import");
    assert_eq!(model.assistance().hover, None);
    let repaired = model.invalidate("shared import");
    assert!(model.finish(repaired, "shared import", Ok(vec![contribution("shared import", "new")])));
    assert_eq!(model.assistance(), Assistance {
        diagnostics: vec!["broken link", "new"], hover: Some("new"), completions: vec!["baseline key", "new"]
    });
    assert!(!model.finish(newer, "shared import", Err("late failure")));
    assert!(model.failures.is_empty());
}

#[test]
fn spike_recovery_model_independent_refreshes_do_not_strand_each_other() {
    let mut model = PublicationModel::default();
    let first = model.invalidate("first import");
    let second = model.invalidate("second import");
    assert!(model.finish(second, "second import", Ok(vec![contribution("second import", "second")])));
    assert!(model.finish(first, "first import", Ok(vec![contribution("first import", "first")])),
        "an independent source refresh must not be discarded by a global epoch");
    assert!(model.pending.is_empty());
}
