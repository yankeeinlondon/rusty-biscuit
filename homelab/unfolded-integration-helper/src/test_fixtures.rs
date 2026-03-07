use serde_json::{Value, json};

#[must_use]
pub fn request(id: u64, msg: &str, msg_data: Value) -> Value {
    json!({
        "kind": "req",
        "id": id,
        "msg": msg,
        "msg_data": msg_data,
    })
}

#[must_use]
pub fn request_with_data(msg: &str, id: u64, msg_data: Value) -> Value {
    json!({
        "kind": "req",
        "id": id,
        "msg": msg,
        "msg_data": msg_data,
    })
}

#[must_use]
pub fn entity_command_request(id: u64, entity_id: &str, cmd_id: &str, params: Value) -> Value {
    let mut msg_data = json!({
        "entity_id": entity_id,
        "cmd_id": cmd_id,
    });

    if !params.is_null() {
        msg_data["params"] = params;
    }

    request_with_data("entity_command", id, msg_data)
}

#[must_use]
pub fn subscribe_events_request(id: u64) -> Value {
    request(id, "subscribe_events", json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_use_top_level_id_field() {
        let request = request(3, "get_driver_version", json!({}));
        assert_eq!(request["id"], 3);
        assert!(request.get("req_id").is_none());
    }
}
