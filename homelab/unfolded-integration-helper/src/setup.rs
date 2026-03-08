//! Setup flow orchestration for UC Remote-driven device configuration.
//!
//! Implements the multi-step setup flow where a Remote initiates device
//! discovery, presents candidates to the user, validates the selection,
//! and persists the configuration.

use serde_json::{Value, json};

use crate::registry::{ConfiguredDevice, KnownDevice, RemoteAssignment};

/// Setup flow state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum SetupState {
    /// Waiting for Remote to initiate setup.
    WaitingForSetup,
    /// Discovery running, will present candidates.
    Discovering,
    /// Showing device selection to user.
    DeviceSelection { candidates: Vec<KnownDevice> },
    /// Validating user's selection.
    Validating { host: String, port: u16 },
    /// Setup complete.
    Complete,
    /// Setup failed.
    Error(String),
}

/// Build `setup_data_schema` for device selection/entry.
///
/// Presents a dropdown of discovered devices plus a manual entry field.
#[must_use]
pub fn device_selection_schema(
    discovered_devices: &[KnownDevice],
    configured_devices: &[ConfiguredDevice],
    remote_id: &str,
    assignments: &[RemoteAssignment],
) -> Value {
    // Find device IDs already assigned to this Remote
    let assigned_ids: Vec<&str> = assignments
        .iter()
        .filter(|a| a.remote_id == remote_id)
        .flat_map(|a| a.device_ids.iter().map(String::as_str))
        .collect();

    // Build dropdown options from discovered + configured, excluding already-assigned
    let mut options = Vec::new();

    for device in discovered_devices {
        if !assigned_ids.contains(&device.device_id.as_str()) {
            let label = device
                .metadata
                .friendly_name
                .as_deref()
                .or(device.metadata.model.as_deref())
                .unwrap_or(&device.host);
            options.push(json!({
                "id": device.device_id,
                "label": { "en": format!("{label} ({host})", host = device.host) },
            }));
        }
    }

    for device in configured_devices {
        if !assigned_ids.contains(&device.device_id.as_str())
            && !options.iter().any(|o| o["id"] == device.device_id)
        {
            options.push(json!({
                "id": device.device_id,
                "label": { "en": format!("{} ({})", device.device_name, device.host) },
            }));
        }
    }

    json!({
        "title": { "en": "Device Configuration" },
        "settings": [
            {
                "id": "device_select",
                "label": { "en": "Select device" },
                "field": {
                    "dropdown": {
                        "value": "",
                        "items": options,
                    }
                }
            },
            {
                "id": "device_host",
                "label": { "en": "Or enter host address" },
                "field": {
                    "text": {
                        "value": "",
                    }
                }
            },
            {
                "id": "device_name",
                "label": { "en": "Device name" },
                "field": {
                    "text": {
                        "value": "",
                    }
                }
            }
        ]
    })
}

/// Build a setup progress event to send to the Remote.
#[must_use]
pub fn setup_progress_event(state: &SetupState) -> Value {
    match state {
        SetupState::Discovering => json!({
            "kind": "event",
            "msg": "driver_setup_change",
            "cat": "DEVICE",
            "msg_data": {
                "event_type": "SETUP",
                "state": "SETUP",
            }
        }),
        SetupState::DeviceSelection { .. } => json!({
            "kind": "event",
            "msg": "driver_setup_change",
            "cat": "DEVICE",
            "msg_data": {
                "event_type": "SETUP",
                "state": "WAIT_USER_ACTION",
            }
        }),
        SetupState::Complete => json!({
            "kind": "event",
            "msg": "driver_setup_change",
            "cat": "DEVICE",
            "msg_data": {
                "event_type": "STOP",
                "state": "OK",
            }
        }),
        SetupState::Error(message) => json!({
            "kind": "event",
            "msg": "driver_setup_change",
            "cat": "DEVICE",
            "msg_data": {
                "event_type": "STOP",
                "state": "ERROR",
                "error": "OTHER",
                "msg": message,
            }
        }),
        _ => json!(null),
    }
}

/// Build a `setup_driver` response acknowledging the setup request.
#[must_use]
pub fn setup_driver_response(req_id: u64, code: u16) -> Value {
    json!({
        "kind": "resp",
        "req_id": req_id,
        "msg": "result",
        "code": code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{DeviceMetadata, DiscoverySource};

    fn sample_known(id: &str, host: &str, model: &str) -> KnownDevice {
        KnownDevice {
            device_id: id.to_string(),
            source: DiscoverySource::NetworkScan,
            host: host.to_string(),
            port: 9529,
            metadata: DeviceMetadata {
                model: Some(model.to_string()),
                ..Default::default()
            },
            last_validated: None,
        }
    }

    fn sample_configured(id: &str, name: &str) -> ConfiguredDevice {
        ConfiguredDevice {
            device_id: id.to_string(),
            device_name: name.to_string(),
            host: "192.168.1.100".to_string(),
            port: 9529,
            metadata: Default::default(),
            driver_config: Default::default(),
        }
    }

    #[test]
    fn schema_excludes_already_assigned() {
        let discovered = vec![
            sample_known("dev1", "192.168.1.10", "DMP-A8"),
            sample_known("dev2", "192.168.1.20", "DMP-A6"),
        ];
        let configured = vec![];
        let assignments = vec![RemoteAssignment {
            remote_id: "remote-1".to_string(),
            device_ids: vec!["dev1".to_string()],
        }];

        let schema = device_selection_schema(&discovered, &configured, "remote-1", &assignments);
        let items = &schema["settings"][0]["field"]["dropdown"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["id"], "dev2");
    }

    #[test]
    fn schema_includes_configured_not_in_discovered() {
        let discovered = vec![sample_known("dev1", "192.168.1.10", "DMP-A8")];
        let configured = vec![sample_configured("dev2", "streamer")];
        let assignments = vec![];

        let schema = device_selection_schema(&discovered, &configured, "remote-1", &assignments);
        let items = &schema["settings"][0]["field"]["dropdown"]["items"];
        assert_eq!(items.as_array().unwrap().len(), 2);
    }

    #[test]
    fn setup_progress_events() {
        let discovering = setup_progress_event(&SetupState::Discovering);
        assert_eq!(discovering["msg_data"]["state"], "SETUP");

        let selection = setup_progress_event(&SetupState::DeviceSelection {
            candidates: vec![],
        });
        assert_eq!(selection["msg_data"]["state"], "WAIT_USER_ACTION");

        let complete = setup_progress_event(&SetupState::Complete);
        assert_eq!(complete["msg_data"]["state"], "OK");

        let error = setup_progress_event(&SetupState::Error("oops".into()));
        assert_eq!(error["msg_data"]["state"], "ERROR");
        assert_eq!(error["msg_data"]["msg"], "oops");
    }
}
