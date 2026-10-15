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

#[cfg(test)]
use schematic_define::{ApiResponse, AuthStrategy};
use crate::registry::{SchemaRegistry, ModuleGroup, assemble_module_group};
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{
    CrossOrigin, Endpoint, EndpointParams, EnvMapping, QueryParamType,
    RestApi, RestMethod,
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
    module_group().expect("valid shipped API").registry
}

/// Assemble the APIs and ordered schemas in one destination context.
pub fn module_group() -> Result<ModuleGroup, DefinitionError> {
    assemble_module_group("::schematic_definitions::eversolo", |ctx| {
        ctx.register::<StatusResponse>()?;
        ctx.register::<GetModelResponse>()?;
        ctx.register::<GetStateResponse>()?;
        ctx.register::<InputOutputListResponse>()?;
        ctx.register::<PowerOptionsResponse>()?;
        ctx.register::<BrightnessResponse>()?;
        ctx.register::<DisplayModeListResponse>()?;
        Ok(vec![build_eversolo_api(ctx)?])
    })
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
    module_group().expect("valid shipped API").apis.remove(0)
}

fn build_eversolo_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    {
        let endpoints = build_endpoints(ctx)?;
        RestApi::builder(ctx, "Eversolo", "http://192.168.1.1:9529")
            .describes("Eversolo DMP-A8 local-network HTTP control API (Zidoo lineage)")
            .no_auth()
            .env_mapping(EnvMapping::default())
            .docs_url("https://eversolo.com/Support/developer/".to_string())
            .module_path("eversolo".to_string())
            .cross_origin(CrossOrigin::same_origin())
            .endpoints(endpoints)
            .build()
    }
}

fn build_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    let mut eps = Vec::with_capacity(24);
    eps.extend(device_endpoints(ctx)?);
    eps.extend(remote_endpoints(ctx)?);
    eps.extend(music_endpoints(ctx)?);
    eps.extend(power_endpoints(ctx)?);
    eps.extend(system_endpoints(ctx)?);
    Ok(eps)
}

// =============================================================================
// Device Identity
// =============================================================================

fn device_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![Endpoint::new("DeviceGetModel", RestMethod::Get, "/ZidooControlCenter/getModel")
            .describes("Get device model, firmware, and network information")
            .returns_json::<GetModelResponse>(ctx)
            .build()?])

}

// =============================================================================
// Remote Control
// =============================================================================

fn remote_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("RemoteSendKey", RestMethod::Get, "/ZidooControlCenter/RemoteControl/sendkey")
            .describes("Send a remote control key command to the device")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "key",
                QueryParamType::String,
                true,
                Some("Remote key constant (e.g., Key.VolumeUp, Key.MediaPlay, Key.PowerOff)"),
            ))
            .build()?,
        Endpoint::new("RemoteInputText", RestMethod::Get, "/ZidooControlCenter/RemoteControl/inputtext")
            .describes("Send text input to the device")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "text",
                QueryParamType::String,
                true,
                Some("Text string to input"),
            ))
            .build()?,
    ])

}

// =============================================================================
// Music Control
// =============================================================================

fn music_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("MusicGetState", RestMethod::Get, "/ZidooMusicControl/v2/getState")
            .describes("Get current playback state, track info, and volume")
            .returns_json::<GetStateResponse>(ctx)
            .build()?,
        Endpoint::new("MusicPlayOrPause", RestMethod::Get, "/ZidooMusicControl/v2/playOrPause")
            .describes("Toggle play/pause on the current track")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("MusicPlayNext", RestMethod::Get, "/ZidooMusicControl/v2/playNext")
            .describes("Skip to the next track")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("MusicPlayLast", RestMethod::Get, "/ZidooMusicControl/v2/playLast")
            .describes("Go back to the previous track")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("MusicSeekTo", RestMethod::Get, "/ZidooMusicControl/v2/seekTo")
            .describes("Seek to a position in the current track")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "time",
                QueryParamType::Integer,
                true,
                Some("Target position in milliseconds"),
            ))
            .build()?,
        Endpoint::new("MusicGetInputOutputList", RestMethod::Get, "/ZidooMusicControl/v2/getInputAndOutputList")
            .describes("List available audio inputs and outputs")
            .returns_json::<InputOutputListResponse>(ctx)
            .build()?,
        Endpoint::new("MusicSetInput", RestMethod::Get, "/ZidooMusicControl/v2/setInputList")
            .describes("Set the active audio input")
            .returns_json::<InputOutputListResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "tag",
                QueryParamType::String,
                true,
                Some("Input tag from getInputAndOutputList"),
            ))
            .build()?,
        Endpoint::new("MusicSetOutput", RestMethod::Get, "/ZidooMusicControl/v2/setOutInputList")
            .describes("Set the active audio output")
            .returns_json::<InputOutputListResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "tag",
                QueryParamType::String,
                true,
                Some("Output tag from getInputAndOutputList"),
            ))
            .build()?,
        Endpoint::new("MusicSetVolume", RestMethod::Get, "/ZidooMusicControl/v2/setDevicesVolume")
            .describes("Set the absolute volume level")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "volume",
                QueryParamType::Integer,
                true,
                Some("Volume level (0 to maxVolume)"),
            ))
            .build()?,
        Endpoint::new("MusicSetMute", RestMethod::Get, "/ZidooMusicControl/v2/setMuteVolume")
            .describes("Set or clear mute state")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "isMute",
                QueryParamType::Integer,
                true,
                Some("Mute state: 0 = unmuted, 1 = muted"),
            ))
            .build()?,
    ])

}

// =============================================================================
// Power Control
// =============================================================================

fn power_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("PowerGetOptions", RestMethod::Get, "/ZidooMusicControl/v2/getPowerOption")
            .describes("Get available power options (shutdown, reboot, etc.)")
            .returns_json::<PowerOptionsResponse>(ctx)
            .build()?,
        Endpoint::new("PowerSetOption", RestMethod::Get, "/ZidooMusicControl/v2/setPowerOption")
            .describes("Execute a power action (poweroff, reboot, screen, timeshutdown)"
                )
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "tag",
                QueryParamType::String,
                true,
                Some("Power option tag (poweroff, reboot, screen, timeshutdown)"),
            ))
            .build()?,
    ])

}

// =============================================================================
// System Settings (community-discovered, undocumented)
// =============================================================================

fn system_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("SystemGetScreenBrightness", RestMethod::Get, "/SystemSettings/displaySettings/getScreenBrightness")
            .describes("Get current screen brightness level (community-discovered)")
            .returns_json::<BrightnessResponse>(ctx)
            .build()?,
        Endpoint::new("SystemSetScreenBrightness", RestMethod::Get, "/SystemSettings/displaySettings/setScreenBrightness")
            .describes("Set screen brightness level (community-discovered)")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("Brightness level index"),
            ))
            .build()?,
        Endpoint::new("SystemGetKnobBrightness", RestMethod::Get, "/SystemSettings/displaySettings/getKnobBrightness")
            .describes("Get current knob LED brightness level (community-discovered)")
            .returns_json::<BrightnessResponse>(ctx)
            .build()?,
        Endpoint::new("SystemSetKnobBrightness", RestMethod::Get, "/SystemSettings/displaySettings/setKnobBrightness")
            .describes("Set knob LED brightness level (community-discovered)")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("Brightness level index"),
            ))
            .build()?,
        Endpoint::new("SystemGetVuModeList", RestMethod::Get, "/SystemSettings/displaySettings/getVUModeList")
            .describes("List available VU meter display modes (community-discovered)")
            .returns_json::<DisplayModeListResponse>(ctx)
            .build()?,
        Endpoint::new("SystemSetVuMode", RestMethod::Get, "/SystemSettings/displaySettings/setVUMode")
            .describes("Set the VU meter display mode (community-discovered)")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("VU mode index from getVUModeList"),
            ))
            .build()?,
        Endpoint::new("SystemGetSpectrumModeList", RestMethod::Get, "/SystemSettings/displaySettings/getSpPlayModeList")
            .describes("List available spectrum display modes (community-discovered)")
            .returns_json::<DisplayModeListResponse>(ctx)
            .build()?,
        Endpoint::new("SystemSetSpectrumMode", RestMethod::Get, "/SystemSettings/displaySettings/setSpPlayModeList")
            .describes("Set the spectrum display mode (community-discovered)")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "index",
                QueryParamType::Integer,
                true,
                Some("Spectrum mode index from getSpPlayModeList"),
            ))
            .build()?,
        Endpoint::new("SystemChangeVuDisplay", RestMethod::Get, "/ZidooMusicControl/v2/changVUDisplay")
            .describes("Toggle VU display open/close (community-discovered)")
            .returns_json::<StatusResponse>(ctx)
            .params(EndpointParams::default().with_query_param(
                "openType",
                QueryParamType::Integer,
                true,
                Some("Display open type"),
            ))
            .build()?,
    ])

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
        assert_eq!(device_endpoints(&mut DefinitionContext::types_in("::schematic_definitions::eversolo")).unwrap().len(), 1);
    }

    #[test]
    fn remote_group_has_2_endpoints() {
        assert_eq!(remote_endpoints(&mut DefinitionContext::types_in("::schematic_definitions::eversolo")).unwrap().len(), 2);
    }

    #[test]
    fn music_group_has_10_endpoints() {
        assert_eq!(music_endpoints(&mut DefinitionContext::types_in("::schematic_definitions::eversolo")).unwrap().len(), 10);
    }

    #[test]
    fn power_group_has_2_endpoints() {
        assert_eq!(power_endpoints(&mut DefinitionContext::types_in("::schematic_definitions::eversolo")).unwrap().len(), 2);
    }

    #[test]
    fn system_group_has_9_endpoints() {
        assert_eq!(system_endpoints(&mut DefinitionContext::types_in("::schematic_definitions::eversolo")).unwrap().len(), 9);
    }
    #[test]
    fn typed_module_group_preserves_endpoint_order_and_inline_components() {
        let group = module_group().unwrap();
        let ids = |api: &RestApi| api.endpoints.iter().map(|ep| ep.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&group.apis[0]), ["DeviceGetModel", "RemoteSendKey", "RemoteInputText", "MusicGetState", "MusicPlayOrPause", "MusicPlayNext", "MusicPlayLast", "MusicSeekTo", "MusicGetInputOutputList", "MusicSetInput", "MusicSetOutput", "MusicSetVolume", "MusicSetMute", "PowerGetOptions", "PowerSetOption", "SystemGetScreenBrightness", "SystemSetScreenBrightness", "SystemGetKnobBrightness", "SystemSetKnobBrightness", "SystemGetVuModeList", "SystemSetVuMode", "SystemGetSpectrumModeList", "SystemSetSpectrumMode", "SystemChangeVuDisplay"]);
        for api in &group.apis {
            assert!(group.registry.validate_completeness(api).is_ok());
        }
    }

}
