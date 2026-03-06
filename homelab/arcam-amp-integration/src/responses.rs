//! UC WebSocket response and event JSON builders.

use serde_json::{Value, json};
use std::collections::HashMap;

use crate::types::{ArcamEntity, ArcamEntityState, DRIVER_ID, DRIVER_VERSION, MIN_CORE_API};

pub fn driver_version_response(req_id: u64) -> Value {
    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": "driver_version",
        "msg_data": {
            "name": DRIVER_ID,
            "version": { "driver": DRIVER_VERSION },
            "min_core_api": MIN_CORE_API,
        }
    })
}

pub fn device_state_event(state: &str) -> Value {
    json!({
        "kind": "event",
        "msg": "device_state",
        "msg_data": { "state": state },
        "cat": "DEVICE",
    })
}

pub fn available_entities_response(req_id: u64, entities: &[ArcamEntity]) -> Value {
    let entity_list: Vec<Value> = entities
        .iter()
        .map(|e| json!({
            "entity_id": e.entity_id,
            "entity_type": e.entity_type,
            "name": e.name,
            "features": e.features,
        }))
        .collect();

    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": "available_entities",
        "msg_data": { "available_entities": entity_list }
    })
}

pub fn entity_states_response(req_id: u64, states: &[ArcamEntityState]) -> Value {
    let state_list: Vec<Value> = states
        .iter()
        .map(|s| json!({
            "entity_id": s.entity_id,
            "entity_type": s.entity_type,
            "attributes": s.attributes,
        }))
        .collect();

    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": "entity_states",
        "msg_data": state_list,
    })
}

pub fn result_response(req_id: u64, code: u16) -> Value {
    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": "result",
        "msg_data": { "code": code }
    })
}

pub fn entity_change_event(entity_id: &str, attributes: &HashMap<String, Value>) -> Value {
    json!({
        "kind": "event",
        "msg": "entity_change",
        "msg_data": {
            "entity_id": entity_id,
            "entity_type": "switch",
            "attributes": attributes,
        },
        "cat": "ENTITY",
    })
}

#[allow(dead_code)]
pub fn authentication_response(req_id: u64) -> Value {
    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": "authentication",
        "msg_data": { "code": 200 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types;

    #[test]
    fn test_driver_version_response() {
        let resp = driver_version_response(1);
        assert_eq!(resp["kind"], "resp");
        assert_eq!(resp["req_id"], 1);
        assert_eq!(resp["msg"], "driver_version");
        assert_eq!(resp["msg_data"]["name"], DRIVER_ID);
    }

    #[test]
    fn test_device_state_event() {
        let evt = device_state_event("CONNECTED");
        assert_eq!(evt["kind"], "event");
        assert_eq!(evt["msg_data"]["state"], "CONNECTED");
    }

    #[test]
    fn test_available_entities_response() {
        let entities = types::build_entities("office");
        let resp = available_entities_response(2, &entities);
        let avail = &resp["msg_data"]["available_entities"];
        assert_eq!(avail.as_array().unwrap().len(), 2);
        assert_eq!(avail[0]["entity_id"], "arcam.office.power");
    }

    #[test]
    fn test_result_response() {
        let resp = result_response(4, 200);
        assert_eq!(resp["msg_data"]["code"], 200);
    }

    #[test]
    fn test_entity_change_event() {
        let mut attrs = HashMap::new();
        attrs.insert("state".to_string(), json!("ON"));
        let evt = entity_change_event("arcam.office.power", &attrs);
        assert_eq!(evt["msg_data"]["entity_id"], "arcam.office.power");
        assert_eq!(evt["msg_data"]["attributes"]["state"], "ON");
    }

    #[test]
    fn test_authentication_response() {
        let resp = authentication_response(0);
        assert_eq!(resp["msg"], "authentication");
        assert_eq!(resp["msg_data"]["code"], 200);
    }
}
