//! Eversolo DMP-A8 local-network HTTP control API definition.
//!
//! The Eversolo DMP-A8 exposes an unauthenticated HTTP API on port 9529 derived
//! from the Zidoo platform. This module defines all 24 known endpoints across
//! five functional groups: device identity, remote control, music playback,
//! power management, and system display settings.
//!
//! ## Endpoint Groups
//!
//! | Group | Count | Path Root |
//! |-------|-------|-----------|
//! | Device | 1 | `/ZidooControlCenter/` |
//! | Remote | 2 | `/ZidooControlCenter/RemoteControl/` |
//! | Music | 10 | `/ZidooMusicControl/v2/` |
//! | Power | 2 | `/ZidooMusicControl/v2/` |
//! | System | 9 | `/SystemSettings/displaySettings/` |

mod types;

pub use types::*;

use crate::registry::SchemaRegistry;
use schematic_define::{
    ApiResponse, AuthStrategy, Endpoint, EndpointParams, EnvMapping, QueryParamType, RestApi,
    RestMethod,
};

/// Creates a schema registry containing all Eversolo response types.
///
/// This registry can be used to generate OpenAPI schemas for the Eversolo API.
/// All response types used by the API endpoints are registered.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::eversolo::{openapi_registry, define_eversolo_api};
///
/// let registry = openapi_registry();
/// let api = define_eversolo_api();
///
/// // Registry contains all response types
/// assert!(registry.get("StatusResponse").is_some());
/// assert!(registry.get("GetModelResponse").is_some());
/// assert!(registry.get("GetStateResponse").is_some());
///
/// // Registry is complete for the API
/// assert!(registry.validate_completeness(&api).is_ok());
/// ```
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        .register::<StatusResponse>("StatusResponse")
        .register::<GetModelResponse>("GetModelResponse")
        .register::<GetStateResponse>("GetStateResponse")
        .register::<InputOutputListResponse>("InputOutputListResponse")
        .register::<PowerOptionsResponse>("PowerOptionsResponse")
        .register::<BrightnessResponse>("BrightnessResponse")
        .register::<DisplayModeListResponse>("DisplayModeListResponse")
}

/// Creates the Eversolo DMP-A8 API definition.
///
/// The API runs on the local network at port 9529 with no authentication.
/// All endpoints use HTTP GET, even for state-mutating operations.
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::eversolo::define_eversolo_api;
///
/// let api = define_eversolo_api();
/// assert_eq!(api.name, "Eversolo");
/// assert_eq!(api.endpoints.len(), 24);
/// ```
pub fn define_eversolo_api() -> RestApi {
    RestApi {
        name: "Eversolo".to_string(),
        description: "Eversolo DMP-A8 local-network HTTP control API (Zidoo lineage)".to_string(),
        base_url: "http://192.168.1.1:9529".to_string(),
        docs_url: Some("https://eversolo.com/Support/developer/".to_string()),
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: build_endpoints(),
        module_path: Some("eversolo".to_string()),
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping::default()),
    }
}

fn build_endpoints() -> Vec<Endpoint> {
    let mut eps = Vec::with_capacity(24);
    eps.extend(device_endpoints());
    eps.extend(remote_endpoints());
    eps.extend(music_endpoints());
    eps.extend(power_endpoints());
    eps.extend(system_endpoints());
    eps
}

// =============================================================================
// Device Identity
// =============================================================================

fn device_endpoints() -> Vec<Endpoint> {
    vec![Endpoint {
        id: "DeviceGetModel".to_string(),
        method: RestMethod::Get,
        path: "/ZidooControlCenter/getModel".to_string(),
        description: "Get device model, firmware, and network information".to_string(),
        request: None,
        response: ApiResponse::json_type("GetModelResponse"),
        headers: vec![],
        params: None,
        oauth_scopes: None,
    }]
}

// =============================================================================
// Remote Control
// =============================================================================

fn remote_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "RemoteSendKey".to_string(),
            method: RestMethod::Get,
            path: "/ZidooControlCenter/RemoteControl/sendkey".to_string(),
            description: "Send a remote control key command to the device".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "key",
                QueryParamType::String,
                true,
                Some("Remote key constant (e.g., Key.VolumeUp, Key.MediaPlay, Key.PowerOff)"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "RemoteInputText".to_string(),
            method: RestMethod::Get,
            path: "/ZidooControlCenter/RemoteControl/inputtext".to_string(),
            description: "Send text input to the device".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "text",
                QueryParamType::String,
                true,
                Some("Text string to input"),
            )),
            oauth_scopes: None,
        },
    ]
}

// =============================================================================
// Music Control
// =============================================================================

fn music_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "MusicGetState".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/getState".to_string(),
            description: "Get current playback state, track info, and volume".to_string(),
            request: None,
            response: ApiResponse::json_type("GetStateResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicPlayOrPause".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/playOrPause".to_string(),
            description: "Toggle play/pause on the current track".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicPlayNext".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/playNext".to_string(),
            description: "Skip to the next track".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicPlayLast".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/playLast".to_string(),
            description: "Go back to the previous track".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicSeekTo".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/seekTo".to_string(),
            description: "Seek to a position in the current track".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "time",
                QueryParamType::Integer,
                true,
                Some("Target position in milliseconds"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicGetInputOutputList".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/getInputAndOutputList".to_string(),
            description: "List available audio inputs and outputs".to_string(),
            request: None,
            response: ApiResponse::json_type("InputOutputListResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicSetInput".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/setInputList".to_string(),
            description: "Set the active audio input".to_string(),
            request: None,
            response: ApiResponse::json_type("InputOutputListResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "tag",
                QueryParamType::String,
                true,
                Some("Input tag from getInputAndOutputList"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicSetOutput".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/setOutInputList".to_string(),
            description: "Set the active audio output".to_string(),
            request: None,
            response: ApiResponse::json_type("InputOutputListResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "tag",
                QueryParamType::String,
                true,
                Some("Output tag from getInputAndOutputList"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicSetVolume".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/setDevicesVolume".to_string(),
            description: "Set the absolute volume level".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "volume",
                QueryParamType::Integer,
                true,
                Some("Volume level (0 to maxVolume)"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "MusicSetMute".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/setMuteVolume".to_string(),
            description: "Set or clear mute state".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            // Keep integer here: the wire contract is `0` / `1` rather than `true` / `false`.
            params: Some(EndpointParams::default().with_query_param(
                "isMute",
                QueryParamType::Integer,
                true,
                Some("Mute state: 0 = unmuted, 1 = muted"),
            )),
            oauth_scopes: None,
        },
    ]
}

// =============================================================================
// Power Control
// =============================================================================

fn power_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "PowerGetOptions".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/getPowerOption".to_string(),
            description: "Get available power options (shutdown, reboot, etc.)".to_string(),
            request: None,
            response: ApiResponse::json_type("PowerOptionsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "PowerSetOption".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/setPowerOption".to_string(),
            description: "Execute a power action (poweroff, reboot, screen, timeshutdown)"
                .to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "tag",
                QueryParamType::String,
                true,
                Some("Power option tag (poweroff, reboot, screen, timeshutdown)"),
            )),
            oauth_scopes: None,
        },
    ]
}

// =============================================================================
// System Settings (community-discovered, undocumented)
// =============================================================================

fn system_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "SystemGetScreenBrightness".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/getScreenBrightness".to_string(),
            description: "Get current screen brightness level (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("BrightnessResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemSetScreenBrightness".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/setScreenBrightness".to_string(),
            description: "Set screen brightness level (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("Brightness level index"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemGetKnobBrightness".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/getKnobBrightness".to_string(),
            description: "Get current knob LED brightness level (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("BrightnessResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemSetKnobBrightness".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/setKnobBrightness".to_string(),
            description: "Set knob LED brightness level (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("Brightness level index"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemGetVuModeList".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/getVUModeList".to_string(),
            description: "List available VU meter display modes (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("DisplayModeListResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemSetVuMode".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/setVUMode".to_string(),
            description: "Set the VU meter display mode (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("VU mode index from getVUModeList"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemGetSpectrumModeList".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/getSpPlayModeList".to_string(),
            description: "List available spectrum display modes (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("DisplayModeListResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemSetSpectrumMode".to_string(),
            method: RestMethod::Get,
            path: "/SystemSettings/displaySettings/setSpPlayModeList".to_string(),
            description: "Set the spectrum display mode (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("Spectrum mode index from getSpPlayModeList"),
            )),
            oauth_scopes: None,
        },
        Endpoint {
            id: "SystemChangeVuDisplay".to_string(),
            method: RestMethod::Get,
            path: "/ZidooMusicControl/v2/changVUDisplay".to_string(),
            description: "Toggle VU display open/close (community-discovered)".to_string(),
            request: None,
            response: ApiResponse::json_type("StatusResponse"),
            headers: vec![],
            params: Some(EndpointParams::default().with_query_param(
                "openType",
                QueryParamType::Integer,
                true,
                Some("Display open type"),
            )),
            oauth_scopes: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_has_correct_metadata() {
        let api = define_eversolo_api();
        assert_eq!(api.name, "Eversolo");
        assert_eq!(api.base_url, "http://192.168.1.1:9529");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn api_uses_no_auth() {
        let api = define_eversolo_api();
        assert!(matches!(api.auth, AuthStrategy::None));
        assert!(api.env_auth.is_empty());
        assert!(api.env_username.is_none());
    }

    #[test]
    fn api_has_24_endpoints() {
        let api = define_eversolo_api();
        assert_eq!(api.endpoints.len(), 24);
    }

    #[test]
    fn api_has_module_path() {
        let api = define_eversolo_api();
        assert_eq!(api.module_path, Some("eversolo".to_string()));
    }

    #[test]
    fn api_has_no_request_suffix() {
        let api = define_eversolo_api();
        assert!(api.request_suffix.is_none());
    }

    // =========================================================================
    // Device Endpoints
    // =========================================================================

    #[test]
    fn device_get_model_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "DeviceGetModel")
            .unwrap();
        assert_eq!(ep.method, RestMethod::Get);
        assert_eq!(ep.path, "/ZidooControlCenter/getModel");
        assert!(ep.request.is_none());
        assert!(matches!(ep.response, ApiResponse::Json { .. }));
        assert!(ep.params.is_none());
    }

    // =========================================================================
    // Remote Endpoints
    // =========================================================================

    #[test]
    fn remote_send_key_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "RemoteSendKey")
            .unwrap();
        assert_eq!(ep.method, RestMethod::Get);
        assert_eq!(ep.path, "/ZidooControlCenter/RemoteControl/sendkey");
        assert!(ep.params.is_some());
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query.len(), 1);
        assert_eq!(params.query[0].name, "key");
        assert!(params.query[0].required);
    }

    #[test]
    fn remote_input_text_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "RemoteInputText")
            .unwrap();
        assert_eq!(ep.method, RestMethod::Get);
        assert!(ep.params.is_some());
    }

    // =========================================================================
    // Music Endpoints
    // =========================================================================

    #[test]
    fn music_get_state_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicGetState")
            .unwrap();
        assert_eq!(ep.method, RestMethod::Get);
        assert_eq!(ep.path, "/ZidooMusicControl/v2/getState");
        assert!(ep.params.is_none());
    }

    #[test]
    fn music_play_or_pause_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicPlayOrPause")
            .unwrap();
        assert_eq!(ep.method, RestMethod::Get);
        assert!(ep.params.is_none());
    }

    #[test]
    fn music_seek_to_has_time_param() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicSeekTo")
            .unwrap();
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "time");
    }

    #[test]
    fn music_set_volume_has_volume_param() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicSetVolume")
            .unwrap();
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "volume");
    }

    #[test]
    fn music_set_mute_has_is_mute_param() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicSetMute")
            .unwrap();
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "isMute");
    }

    #[test]
    fn music_get_input_output_list_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicGetInputOutputList")
            .unwrap();
        assert_eq!(ep.path, "/ZidooMusicControl/v2/getInputAndOutputList");
    }

    #[test]
    fn music_set_input_has_tag_param() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicSetInput")
            .unwrap();
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "tag");
    }

    #[test]
    fn music_set_output_has_tag_param() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "MusicSetOutput")
            .unwrap();
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "tag");
    }

    // =========================================================================
    // Power Endpoints
    // =========================================================================

    #[test]
    fn power_get_options_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "PowerGetOptions")
            .unwrap();
        assert_eq!(ep.method, RestMethod::Get);
        assert!(ep.params.is_none());
    }

    #[test]
    fn power_set_option_has_tag_param() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "PowerSetOption")
            .unwrap();
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "tag");
    }

    // =========================================================================
    // System Endpoints
    // =========================================================================

    #[test]
    fn system_get_screen_brightness_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "SystemGetScreenBrightness")
            .unwrap();
        assert_eq!(
            ep.path,
            "/SystemSettings/displaySettings/getScreenBrightness"
        );
        assert!(ep.params.is_none());
    }

    #[test]
    fn system_set_screen_brightness_has_index_param() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "SystemSetScreenBrightness")
            .unwrap();
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "index");
    }

    #[test]
    fn system_change_vu_display_endpoint() {
        let api = define_eversolo_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "SystemChangeVuDisplay")
            .unwrap();
        assert_eq!(ep.path, "/ZidooMusicControl/v2/changVUDisplay");
        let params = ep.params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "openType");
    }

    // =========================================================================
    // All endpoints are GET with no request body
    // =========================================================================

    #[test]
    fn all_endpoints_are_get() {
        let api = define_eversolo_api();
        for ep in &api.endpoints {
            assert_eq!(ep.method, RestMethod::Get, "Endpoint {} is not GET", ep.id);
        }
    }

    #[test]
    fn no_endpoints_have_request_bodies() {
        let api = define_eversolo_api();
        for ep in &api.endpoints {
            assert!(
                ep.request.is_none(),
                "Endpoint {} has a request body",
                ep.id
            );
        }
    }

    // =========================================================================
    // Endpoint group counts
    // =========================================================================

    #[test]
    fn device_group_has_1_endpoint() {
        assert_eq!(device_endpoints().len(), 1);
    }

    #[test]
    fn remote_group_has_2_endpoints() {
        assert_eq!(remote_endpoints().len(), 2);
    }

    #[test]
    fn music_group_has_10_endpoints() {
        assert_eq!(music_endpoints().len(), 10);
    }

    #[test]
    fn power_group_has_2_endpoints() {
        assert_eq!(power_endpoints().len(), 2);
    }

    #[test]
    fn system_group_has_9_endpoints() {
        assert_eq!(system_endpoints().len(), 9);
    }
}
