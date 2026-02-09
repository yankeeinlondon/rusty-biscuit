use axum_test::TestServer;
use homelab_server::{build_router, config::HomeyConfig, state::AppState};
use std::time::Duration;

/// Create a test AppState with no devices configured
fn empty_state() -> AppState {
    AppState::from_config(HomeyConfig::new(), None)
}

// ============================================================================
// Health Endpoints (2 tests)
// ============================================================================

#[tokio::test]
async fn test_health_basic() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;

    response.assert_status_ok();
    response.assert_json(&serde_json::json!({
        "status": "healthy"
    }));
}

#[tokio::test]
async fn test_health_devices_none_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health/devices").await;

    response.assert_status_ok();
    response.assert_json(&serde_json::json!({
        "sony": {
            "configured": false,
            "host": null
        },
        "arcam": {
            "configured": false,
            "host": null
        }
    }));
}

// ============================================================================
// Sony Endpoints - Device Not Configured (9 tests)
// ============================================================================

#[tokio::test]
async fn test_sony_power_get_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony/power").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
    assert!(json["error"].as_str().unwrap().contains("Sony Receiver"));
}

#[tokio::test]
async fn test_sony_power_post_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony/power")
        .json(&serde_json::json!({ "active": true }))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_volume_get_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony/volume").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_volume_post_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony/volume")
        .json(&serde_json::json!({ "level": 50 }))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_mute_get_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony/mute").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_mute_post_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony/mute")
        .json(&serde_json::json!({ "mute": true }))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_inputs_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony/inputs").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_current_input_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony/input/current").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_set_input_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony/input")
        .json(&serde_json::json!({ "uri": "extInput:hdmi?port=1" }))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_sony_system_info_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony/system/info").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

// ============================================================================
// Arcam Endpoints - Device Not Configured (7 tests)
// ============================================================================

#[tokio::test]
async fn test_arcam_power_get_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/arcam/power").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
    assert!(json["error"].as_str().unwrap().contains("Arcam Amplifier"));
}

#[tokio::test]
async fn test_arcam_power_on_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.post("/arcam/power/on").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_arcam_power_off_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.post("/arcam/power/off").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_arcam_mute_get_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/arcam/mute").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_arcam_mute_on_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.post("/arcam/mute/on").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_arcam_mute_off_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.post("/arcam/mute/off").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

#[tokio::test]
async fn test_arcam_mode_not_configured() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/arcam/mode").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

// ============================================================================
// Validation Tests (2 tests)
// ============================================================================

#[tokio::test]
async fn test_sony_volume_invalid_too_high() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony/volume")
        .json(&serde_json::json!({ "level": 150 }))
        .await;

    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "INVALID_VOLUME");
    assert!(json["error"].as_str().unwrap().contains("0-100"));
    assert!(json["error"].as_str().unwrap().contains("150"));
}

#[tokio::test]
async fn test_sony_volume_valid_range_boundary() {
    // Even without a device, the volume validation should still work
    // This test verifies the validation happens before the device check
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    // Test volume at max boundary (100) - should pass validation
    let response = server
        .post("/sony/volume")
        .json(&serde_json::json!({ "level": 100 }))
        .await;

    // Should get NOT_FOUND (device not configured), not BAD_REQUEST (invalid volume)
    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

// ============================================================================
// Additional Edge Cases (2 tests)
// ============================================================================

#[tokio::test]
async fn test_nonexistent_route() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/nonexistent").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_sony_volume_at_zero_boundary() {
    // Test volume at minimum boundary (0)
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony/volume")
        .json(&serde_json::json!({ "level": 0 }))
        .await;

    // Should get NOT_FOUND (device not configured), not BAD_REQUEST
    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_CONFIGURED");
}

// ============================================================================
// Sony Receiver CRUD Tests (8 tests)
// ============================================================================

#[tokio::test]
async fn test_sony_receiver_list_empty() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony_receiver").await;

    response.assert_status_ok();
    response.assert_json(&serde_json::json!([]));
}

#[tokio::test]
async fn test_sony_receiver_create() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony_receiver")
        .json(&serde_json::json!({
            "name": "living-room",
            "host": "192.168.1.100",
            "port": 10000
        }))
        .await;

    response.assert_status(axum::http::StatusCode::CREATED);
    let json: serde_json::Value = response.json();
    assert_eq!(json["name"], "living-room");
    assert_eq!(json["host"], "192.168.1.100");
    assert_eq!(json["port"], 10000);
}

#[tokio::test]
async fn test_sony_receiver_create_invalid_name() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/sony_receiver")
        .json(&serde_json::json!({
            "name": "Invalid Name!",
            "host": "192.168.1.100"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "INVALID_DEVICE_NAME");
}

#[tokio::test]
async fn test_sony_receiver_create_duplicate() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    // Create first device
    server
        .post("/sony_receiver")
        .json(&serde_json::json!({
            "name": "test-device",
            "host": "192.168.1.100"
        }))
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // Try to create duplicate
    let response = server
        .post("/sony_receiver")
        .json(&serde_json::json!({
            "name": "test-device",
            "host": "192.168.1.101"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::CONFLICT);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_EXISTS");
}

#[tokio::test]
async fn test_sony_receiver_update() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    // Create device
    server
        .post("/sony_receiver")
        .json(&serde_json::json!({
            "name": "test-device",
            "host": "192.168.1.100"
        }))
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // Update device
    let response = server
        .put("/sony_receiver/test-device")
        .json(&serde_json::json!({
            "host": "192.168.1.200",
            "port": 12000
        }))
        .await;

    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    assert_eq!(json["host"], "192.168.1.200");
    assert_eq!(json["port"], 12000);
}

#[tokio::test]
async fn test_sony_receiver_update_not_found() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .put("/sony_receiver/nonexistent")
        .json(&serde_json::json!({
            "host": "192.168.1.100"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_FOUND");
}

#[tokio::test]
async fn test_sony_receiver_delete() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    // Create device
    server
        .post("/sony_receiver")
        .json(&serde_json::json!({
            "name": "test-device",
            "host": "192.168.1.100"
        }))
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // Delete device
    let response = server.delete("/sony_receiver/test-device").await;
    response.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify it's gone
    let list_response = server.get("/sony_receiver").await;
    list_response.assert_json(&serde_json::json!([]));
}

#[tokio::test]
async fn test_sony_receiver_delete_not_found() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.delete("/sony_receiver/nonexistent").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_FOUND");
}

// ============================================================================
// Arcam Amplifier CRUD Tests (6 tests)
// ============================================================================

#[tokio::test]
async fn test_arcam_amp_list_empty() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/arcam_amp").await;

    response.assert_status_ok();
    response.assert_json(&serde_json::json!([]));
}

#[tokio::test]
async fn test_arcam_amp_create() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/arcam_amp")
        .json(&serde_json::json!({
            "name": "office",
            "host": "192.168.1.101"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::CREATED);
    let json: serde_json::Value = response.json();
    assert_eq!(json["name"], "office");
    assert_eq!(json["host"], "192.168.1.101");
    assert_eq!(json["port"], 50000); // default port
}

#[tokio::test]
async fn test_arcam_amp_create_empty_host() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/arcam_amp")
        .json(&serde_json::json!({
            "name": "test-device",
            "host": ""
        }))
        .await;

    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "INVALID_HOST");
}

#[tokio::test]
async fn test_arcam_amp_delete() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    // Create device
    server
        .post("/arcam_amp")
        .json(&serde_json::json!({
            "name": "test-amp",
            "host": "192.168.1.101"
        }))
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // Delete device
    let response = server.delete("/arcam_amp/test-amp").await;
    response.assert_status(axum::http::StatusCode::NO_CONTENT);
}

// ============================================================================
// Named Device Control Tests (Device Not Found) (4 tests)
// ============================================================================

#[tokio::test]
async fn test_sony_receiver_named_power_not_found() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony_receiver/nonexistent/power").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_FOUND");
}

#[tokio::test]
async fn test_sony_receiver_named_volume_not_found() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/sony_receiver/nonexistent/volume").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_FOUND");
}

#[tokio::test]
async fn test_arcam_amp_named_power_not_found() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/arcam_amp/nonexistent/power").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_FOUND");
}

#[tokio::test]
async fn test_arcam_amp_named_mute_not_found() {
    let state = empty_state();
    let app = build_router(state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/arcam_amp/nonexistent/mute").await;

    response.assert_status(axum::http::StatusCode::NOT_FOUND);
    let json: serde_json::Value = response.json();
    assert_eq!(json["code"], "DEVICE_NOT_FOUND");
}
