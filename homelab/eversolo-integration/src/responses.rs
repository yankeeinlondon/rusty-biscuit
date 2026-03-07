//! UC WebSocket response and event JSON builders.

use serde_json::{Value, json};
use std::collections::HashMap;

use crate::types::{DRIVER_ID, DRIVER_VERSION, EversoloEntity, EversoloEntityState, MIN_CORE_API};

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

pub fn available_entities_response(req_id: u64, entities: &[EversoloEntity]) -> Value {
    let entity_list: Vec<Value> = entities
        .iter()
        .map(|entity| {
            let mut value = json!({
                "entity_id": entity.entity_id,
                "entity_type": entity.entity_type,
                "name": entity.name,
                "features": entity.features,
            });
            if let Some(options) = &entity.options {
                value["options"] = json!(options);
            }
            value
        })
        .collect();

    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": "available_entities",
        "msg_data": { "available_entities": entity_list }
    })
}

pub fn entity_states_response(req_id: u64, states: &[EversoloEntityState]) -> Value {
    let state_list: Vec<Value> = states
        .iter()
        .map(|state| {
            json!({
                "entity_id": state.entity_id,
                "entity_type": state.entity_type,
                "attributes": state.attributes,
            })
        })
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

pub fn entity_change_event(
    entity_id: &str,
    entity_type: &str,
    attributes: &HashMap<String, Value>,
) -> Value {
    json!({
        "kind": "event",
        "msg": "entity_change",
        "msg_data": {
            "entity_id": entity_id,
            "entity_type": entity_type,
            "attributes": attributes,
        },
        "cat": "ENTITY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types;

    #[test]
    fn test_driver_version_response() {
        let response = driver_version_response(1);
        assert_eq!(response["kind"], "resp");
        assert_eq!(response["req_id"], 1);
        assert_eq!(response["msg"], "driver_version");
        assert_eq!(response["msg_data"]["name"], DRIVER_ID);
    }

    #[test]
    fn test_device_state_event() {
        let event = device_state_event("CONNECTED");
        assert_eq!(event["kind"], "event");
        assert_eq!(event["msg_data"]["state"], "CONNECTED");
    }

    #[test]
    fn test_available_entities_response() {
        let entities = types::build_entities("living", &["USB".to_string()], 160);
        let response = available_entities_response(2, &entities);
        let available = &response["msg_data"]["available_entities"];
        assert_eq!(available.as_array().unwrap().len(), 2);
        assert_eq!(available[0]["entity_id"], "eversolo.living.power");
        assert_eq!(available[1]["options"]["source_list"][0], "USB");
    }

    #[test]
    fn test_result_response() {
        let response = result_response(4, 200);
        assert_eq!(response["msg_data"]["code"], 200);
    }

    #[test]
    fn test_entity_change_event() {
        let mut attrs = HashMap::new();
        attrs.insert("state".to_string(), json!("PLAYING"));
        let event = entity_change_event("eversolo.living.player", "media_player", &attrs);
        assert_eq!(event["msg_data"]["entity_id"], "eversolo.living.player");
        assert_eq!(event["msg_data"]["entity_type"], "media_player");
        assert_eq!(event["msg_data"]["attributes"]["state"], "PLAYING");
    }
}
