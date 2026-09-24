//! Bounded D5 experiment, not a production provenance implementation.
//! Real Darkmatter interpolation and Claudine batch validation are used below;
//! sparse metadata, stage scheduling, and joint publication are test-only.

use std::collections::{BTreeMap, HashMap};

use claudine::composition::runtime_state::{RuntimeMutationError, RuntimeSnapshot, RuntimeState};
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};
use darkmatter::markdown::compose::subtree::SubtreeCompose;
use indexmap::IndexMap;
use serde_json::{Map, Value, json};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Segment {
    Key(String),
    Index(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Complete,
    Interpolate,
    Shell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Metadata {
    no_shell: bool,
    stage: Stage,
}

#[derive(Clone, Debug, PartialEq)]
struct Envelope {
    value: Value,
    records: BTreeMap<Vec<Segment>, Metadata>,
}

impl Envelope {
    fn new(value: Value, no_shell: bool, stage: Stage) -> Self {
        Self { value, records: BTreeMap::from([(vec![], Metadata { no_shell, stage })]) }
    }

    fn effective(&self, path: &[Segment]) -> Metadata {
        let mut result = self.records[&vec![]];
        for depth in 1..=path.len() {
            if let Some(record) = self.records.get(&path[..depth]) {
                result.no_shell |= record.no_shell;
                result.stage = record.stage;
            }
        }
        result
    }

    fn select(&self, path: &[Segment]) -> Self {
        let mut value = &self.value;
        for part in path {
            value = match part {
                Segment::Key(key) => value.get(key).unwrap(),
                Segment::Index(index) => value.get(index).unwrap(),
            };
        }
        let mut records = BTreeMap::from([(vec![], self.effective(path))]);
        for (location, metadata) in &self.records {
            if location.len() > path.len() && location.starts_with(path) {
                records.insert(location[path.len()..].to_vec(), *metadata);
            }
        }
        Self { value: value.clone(), records }
    }

    fn array(items: &[Self], destination_no_shell: bool) -> Self {
        let mut result = Self::new(json!([]), destination_no_shell, Stage::Complete);
        for (index, item) in items.iter().enumerate() {
            result.value.as_array_mut().unwrap().push(item.value.clone());
            for (path, metadata) in &item.records {
                let mut rebased = vec![Segment::Index(index)];
                rebased.extend(path.clone());
                result.records.insert(rebased, *metadata);
            }
        }
        result
    }

    // A shell-stage counter is a sentinel, never an actual shell invocation.
    fn resume(&self, destination_no_shell: bool, calls: &mut usize) -> Result<Self, &'static str> {
        let metadata = self.effective(&[]);
        let no_shell = metadata.no_shell || destination_no_shell;
        let value = match metadata.stage {
            Stage::Complete => self.value.clone(),
            Stage::Interpolate => interpolate(&self.value)?,
            Stage::Shell if no_shell => return Err("shell prohibited before dispatch"),
            Stage::Shell => {
                *calls += 1;
                json!("sentinel output")
            }
        };
        Ok(Self::new(value, no_shell, Stage::Complete))
    }
}

fn interpolate(value: &Value) -> Result<Value, &'static str> {
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(HashMap::from([("secret".to_string(), json!("evaluated again"))]))
        .with_context(ComposeContext::capture_minimal())
        .build().unwrap();
    SubtreeCompose::new(value, &state).strict().compose().map_err(|_| "interpolation failed")
}

#[derive(Clone, Default)]
struct Candidate {
    runtime: RuntimeSnapshot,
    metadata: BTreeMap<String, Envelope>,
}

impl Candidate {
    // Production would put both fields behind RuntimeState's one mutex. This
    // single-threaded prototype proves the candidate/commit boundary only.
    fn commit(&mut self, updates: &IndexMap<String, Envelope>) -> Result<(), RuntimeMutationError> {
        let candidate = RuntimeState::from_snapshot(self.runtime.clone());
        let values = updates.iter().map(|(key, value)| (key.clone(), value.value.clone())).collect();
        candidate.set_batch(&EffectEngine::builder().build(), &values, &Map::new())?;
        let mut metadata = self.metadata.clone();
        metadata.extend(updates.iter().map(|(key, value)| (key.clone(), value.clone())));
        *self = Self { runtime: candidate.snapshot(), metadata };
        Ok(())
    }

    fn read(&self, key: &str) -> Envelope {
        let mut envelope = self.metadata[key].clone();
        envelope.value = self.runtime.mutations[key].clone();
        envelope
    }
}

#[test]
fn spike_provenance_batch_failure_and_retry_keep_value_and_policy_together() {
    let source = Envelope::new(json!({"nested": ["$(sentinel)"]}), true, Stage::Shell);
    let selected = source.select(&[Segment::Key("nested".into()), Segment::Index(0)]);
    assert!(selected.effective(&[]).no_shell, "selection must materialize the ancestor ban");
    let mut state = Candidate::default();
    state.commit(&IndexMap::from([("command".into(), selected.clone())])).unwrap();
    let before = state.clone();
    let bad = IndexMap::from([
        ("command".into(), Envelope::new(json!("replacement"), false, Stage::Complete)),
        ("outputs".into(), Envelope::new(json!(1), false, Stage::Complete)),
    ]);
    assert!(matches!(state.commit(&bad), Err(RuntimeMutationError::ReservedKey { .. })));
    assert_eq!(state.runtime.mutations, before.runtime.mutations);
    assert_eq!(state.metadata, before.metadata);

    let mut calls = 0;
    for _ in 0..2 {
        let retry = state.clone();
        assert_eq!(retry.read("command").resume(false, &mut calls), Err("shell prohibited before dispatch"));
    }
    assert_eq!(calls, 0);
    assert_eq!(state.read("command"), selected, "failed preparation cannot consume the retained stage");
    let unrestricted = Envelope::new(json!("$(sentinel)"), false, Stage::Shell);
    assert!(unrestricted.resume(true, &mut calls).is_err(), "destination restriction also applies");
    assert!(unrestricted.resume(false, &mut calls).is_ok());
    assert_eq!(calls, 1, "positive control must reach the sentinel");
}

#[test]
fn spike_provenance_array_moves_preserve_sibling_and_typed_path_identity() {
    let mut source = Envelope::new(json!({"a.b": {"0": ["restricted", "free"]}}), false, Stage::Complete);
    let prefix = vec![Segment::Key("a.b".into()), Segment::Key("0".into())];
    let mut first = prefix.clone();
    first.push(Segment::Index(0));
    source.records.insert(first.clone(), Metadata { no_shell: true, stage: Stage::Shell });
    let selected = source.select(&prefix);
    let restricted = selected.select(&[Segment::Index(0)]);
    let free = selected.select(&[Segment::Index(1)]);
    let moved = Envelope::array(&[free.clone(), restricted.clone()], false);
    assert!(!moved.effective(&[Segment::Index(0)]).no_shell);
    assert!(moved.effective(&[Segment::Index(1)]).no_shell);
    assert_eq!(moved.select(&[Segment::Index(1)]), restricted);
    let removed = Envelope::array(&[moved.select(&[Segment::Index(1)])], false);
    assert_eq!(removed.select(&[Segment::Index(0)]), source.select(&first));
    let inserted = Envelope::array(&[free.clone(), removed.select(&[Segment::Index(0)]), free], true);
    assert!(inserted.effective(&[Segment::Index(0)]).no_shell, "descendant cannot relax destination policy");
    assert!(inserted.effective(&[Segment::Index(1)]).no_shell);
}

#[test]
fn spike_provenance_completed_literals_survive_real_batch_and_retry_without_rescan() {
    for (authored, expected) in [
        ("{{{ secret }}}", "{{ secret }}"),
        (r"\{{ secret }}", r"\{{ secret }}"),
        (r"\{\{ secret }}", r"\{\{ secret }}"),
    ] {
        let output = interpolate(&json!(authored)).unwrap();
        assert_eq!(output, json!(expected), "pin this boundary's ordinary escape behavior");
        if authored.starts_with("{{{") {
            assert_eq!(interpolate(&output).unwrap(), json!("evaluated again"), "negative control: plain JSON re-entry loses completion");
        }
        let complete = Envelope::new(output.clone(), true, Stage::Complete);
        let mut state = Candidate::default();
        state.commit(&IndexMap::from([("message".into(), complete)])).unwrap();
        for _ in 0..2 {
            assert_eq!(state.clone().read("message").resume(false, &mut 0).unwrap().value, output);
        }
    }
}

#[test]
fn spike_provenance_replacement_drops_origin_but_keeps_destination_policy() {
    let mut state = Candidate::default();
    state.commit(&IndexMap::from([("message".into(), Envelope::new(json!("old"), true, Stage::Shell))])).unwrap();
    let new = Envelope::new(json!("{{ secret }}"), false, Stage::Interpolate);
    state.commit(&IndexMap::from([("message".into(), new.clone())])).unwrap();
    assert_eq!(state.read("message"), new, "discarded content must not taint replacement");
    let output = state.read("message").resume(true, &mut 0).unwrap();
    assert_eq!(output.value, json!("evaluated again"));
    assert!(output.effective(&[]).no_shell);
    assert_eq!(output.effective(&[]).stage, Stage::Complete);
}
