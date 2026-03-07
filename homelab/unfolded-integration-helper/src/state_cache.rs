use std::collections::HashMap;

use crate::Attributes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Shared entity state payload used across integrations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: String,
    pub entity_type: String,
    pub attributes: Attributes,
}

impl EntityState {
    #[must_use]
    pub fn new(
        entity_id: impl Into<String>,
        entity_type: impl Into<String>,
        attributes: Attributes,
    ) -> Self {
        Self {
            entity_id: entity_id.into(),
            entity_type: entity_type.into(),
            attributes,
        }
    }
}

pub type CachedEntityState = EntityState;

/// Keyed cache for UC entity state snapshots.
#[derive(Debug, Clone, Default)]
pub struct StateCache {
    states: HashMap<String, EntityState>,
}

impl StateCache {
    #[must_use]
    pub fn new(initial_states: impl IntoIterator<Item = EntityState>) -> Self {
        let mut states = HashMap::new();
        for state in initial_states {
            states.insert(state.entity_id.clone(), state);
        }
        Self { states }
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<EntityState> {
        let mut states: Vec<_> = self.states.values().cloned().collect();
        states.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));
        states
    }

    pub fn replace(
        &mut self,
        entity_id: impl Into<String>,
        entity_type: impl Into<String>,
        attributes: Attributes,
    ) -> Option<EntityState> {
        let state = EntityState::new(entity_id, entity_type, attributes);

        let changed = self
            .states
            .get(&state.entity_id)
            .map(|current| current != &state)
            .unwrap_or(true);

        self.states.insert(state.entity_id.clone(), state.clone());
        changed.then_some(state)
    }

    pub fn merge(
        &mut self,
        entity_id: &str,
        entity_type: &str,
        attributes: &HashMap<String, Value>,
    ) -> Option<EntityState> {
        let mut merged = self
            .states
            .get(entity_id)
            .cloned()
            .unwrap_or_else(|| EntityState::new(entity_id, entity_type, HashMap::new()));

        for (key, value) in attributes {
            merged.attributes.insert(key.clone(), value.clone());
        }

        self.replace(
            merged.entity_id.clone(),
            merged.entity_type.clone(),
            merged.attributes,
        )
    }

    pub fn replace_attributes(
        &mut self,
        entity_id: &str,
        entity_type: &str,
        attributes: Attributes,
    ) -> Option<EntityState> {
        self.replace(entity_id.to_string(), entity_type.to_string(), attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replace_detects_changes() {
        let mut cache = StateCache::new([EntityState::new(
            "demo.power",
            "switch",
            HashMap::from([("state".to_string(), json!("OFF"))]),
        )]);

        assert!(
            cache
                .replace(
                    "demo.power",
                    "switch",
                    HashMap::from([("state".to_string(), json!("OFF"))]),
                )
                .is_none()
        );
        assert!(
            cache
                .replace(
                    "demo.power",
                    "switch",
                    HashMap::from([("state".to_string(), json!("ON"))]),
                )
                .is_some()
        );
    }
}
