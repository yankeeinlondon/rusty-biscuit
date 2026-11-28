//! The retired sequence keys and their replacement advice, read from the
//! shipped schemas rather than written here.
//!
//! `claudine/registration/sequence.yaml` declares the task keys
//! (`task-fields`) and the group key as `removed(<replacement>)`, and
//! `sequence-document.yaml` / `formal-sequence-file.yaml` declare the root's.
//! Simplified Schema lowers each to a `false` property plus the owning
//! object's removal annotation, so the editor, the schema verdict, and the
//! task parser all report the same replacement. The parser keeps its own
//! typed refusal ([`CompositionError::RemovedGrammar`]) at the boundary where
//! it always refused the key, and takes only the advice from here.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::Value;
use simplified_schema::REMOVED_KEYS_ANNOTATION;

use crate::composition::CompositionError;

/// The object a removed key is written on, as the parser reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RemovedOwner {
    /// A sequence document's root (Markdown frontmatter or a formal YAML
    /// sequence file).
    Root,
    /// A task: a sequence step, a group leg, or a task file.
    Task,
    /// A group object: inline, a group file, or a catalog entry.
    Group,
}

/// The shipped schemas' replacement advice for `key` on `owner`, or `None`
/// when the key is not removed there.
pub fn replacement(owner: RemovedOwner, key: &str) -> Option<&'static str> {
    removed_keys().get(&owner)?.get(key).map(String::as_str)
}

/// The removed keys of `owner`, in the order the schema declares them.
pub fn removed_keys_of(owner: RemovedOwner) -> impl Iterator<Item = &'static str> {
    removed_keys().get(&owner).into_iter().flat_map(|keys| keys.keys().map(String::as_str))
}

/// The first removed key of `owner` present in `map`, as the typed refusal
/// the parser has always raised for it, located at `property` (the owner's
/// authored path; empty for a file root) in `origin`.
pub fn refuse_removed(
    owner: RemovedOwner,
    map: &serde_json::Map<String, Value>,
    origin: &std::path::Path,
) -> Result<(), CompositionError> {
    match removed_keys_of(owner).find(|key| map.contains_key(*key)) {
        Some(key) => Err(removed_key(owner, key, &map[key], origin)),
        None => Ok(()),
    }
}

/// The typed refusal of the removed `key` (holding `value`) on `owner`.
///
/// The replacement is the schema's. `execution: serial` also gets the
/// value-specific sentence the parser has always given it, since ordered work
/// has a concrete spelling.
pub fn removed_key(owner: RemovedOwner, key: &str, value: &Value, origin: &std::path::Path) -> CompositionError {
    let advice = replacement(owner, key)
        .unwrap_or_else(|| panic!("`{key}` is declared `removed(…)` on {owner:?} in Claudine's shipped schemas"));
    let replacement = if key == "execution" && value.as_str() == Some("serial") {
        format!("{advice}; for example `sequence: [{{name: first, shell: command}}]`, nested as a group leg when needed")
    } else {
        advice.to_string()
    };
    CompositionError::RemovedGrammar {
        source_path: origin.to_path_buf(),
        path: key.into(),
        removed: key.into(),
        replacement,
    }
}

type RemovedTable = BTreeMap<RemovedOwner, indexmap::IndexMap<String, String>>;

fn removed_keys() -> &'static RemovedTable {
    static TABLE: OnceLock<RemovedTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let payload = sequence_payload();
        let mut table = RemovedTable::new();
        collect(&payload, true, &mut table);
        table
    })
}

/// The prepared JSON Schema of the sequence-document trigger, which reaches
/// the root, every task, and every group declaration.
fn sequence_payload() -> Value {
    const PATH: &str = "schemas/sequence-document.yaml";
    if let Some(payload) = super::super::schema::shipped::registered_payload(PATH) {
        return payload.json_schema.as_ref().clone();
    }
    // A refused artifact: prepare the trigger from its compiled text, as
    // registration does.
    super::super::schema::shipped::prepare_registered()
        .ok()
        .and_then(|records| records.into_iter().find(|record| record.path == PATH))
        .and_then(|record| record.payload)
        .map(|payload| payload.json_schema)
        .unwrap_or_else(|| panic!("Claudine's shipped `{PATH}` prepares"))
}

/// Records every removal annotation in `node`, classifying its owner by the
/// declarations beside it: the root, a group (it declares `tasks`), or a task
/// (it declares `side_effect`).
fn collect(node: &Value, is_root: bool, table: &mut RemovedTable) {
    match node {
        Value::Object(map) => {
            if let Some(Value::Object(removed)) = map.get(REMOVED_KEYS_ANNOTATION) {
                let declares = |key: &str| map.get("properties").and_then(|props| props.get(key)).is_some();
                let owner = if is_root {
                    Some(RemovedOwner::Root)
                } else if declares("tasks") {
                    Some(RemovedOwner::Group)
                } else if declares("side_effect") {
                    Some(RemovedOwner::Task)
                } else {
                    None
                };
                if let Some(owner) = owner {
                    let keys = table.entry(owner).or_default();
                    for (key, advice) in removed {
                        if let Some(advice) = advice.as_str() {
                            keys.entry(key.clone()).or_insert_with(|| advice.to_string());
                        }
                    }
                }
            }
            for child in map.values() {
                collect(child, false, table);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect(item, false, table)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_owner_reads_the_spec_replacements_from_the_shipped_schemas() {
        assert_eq!(replacement(RemovedOwner::Root, "fail_fast"), Some("set fail_fast on a group object"));
        assert_eq!(
            replacement(RemovedOwner::Task, "setup"),
            Some("use a preceding `side_effect:` task for an action or `shell:` task for a command")
        );
        assert_eq!(
            replacement(RemovedOwner::Task, "teardown"),
            Some("use a group's `finalize:`; wrap a single task in a group of one")
        );
        assert_eq!(
            replacement(RemovedOwner::Task, "execution"),
            Some("omit execution; use a nested sequence for ordered work")
        );
        assert_eq!(replacement(RemovedOwner::Task, "fail_fast"), Some("`fail_fast` belongs on a `group:` object only"));
        assert_eq!(
            replacement(RemovedOwner::Group, "execution"),
            Some("omit execution; use a nested sequence for ordered work")
        );
        // Current keys are not removed: `fail_fast` on a group, `kind`.
        assert_eq!(replacement(RemovedOwner::Group, "fail_fast"), None);
        assert_eq!(replacement(RemovedOwner::Root, "kind"), None);
        assert_eq!(removed_keys_of(RemovedOwner::Task).collect::<Vec<_>>(), ["setup", "teardown", "execution", "fail_fast"]);
    }

    #[test]
    fn serial_execution_keeps_its_value_specific_sentence() {
        let origin = std::path::Path::new("seq.md");
        let CompositionError::RemovedGrammar { replacement, path, .. } =
            removed_key(RemovedOwner::Task, "execution", &Value::String("serial".into()), origin)
        else {
            panic!("a removed key is RemovedGrammar");
        };
        assert_eq!(path, "execution");
        assert!(replacement.starts_with("omit execution; use a nested sequence for ordered work; for example"), "{replacement}");
        let CompositionError::RemovedGrammar { replacement, .. } =
            removed_key(RemovedOwner::Group, "execution", &Value::String("parallel".into()), origin)
        else {
            panic!("a removed key is RemovedGrammar");
        };
        assert_eq!(replacement, "omit execution; use a nested sequence for ordered work");
    }
}
