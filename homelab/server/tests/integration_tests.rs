use axum_test::TestServer;
use homelab_server::{build_router, state::AppState};
use std::time::Duration;

/// Create a test AppState with no devices configured
fn empty_state() -> AppState {
    AppState {
        sony: None,
        arcam_host: None,
        request_timeout: Duration::from_millis(5000),
    }
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
