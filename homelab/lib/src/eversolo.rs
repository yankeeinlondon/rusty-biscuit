//! Eversolo DMP-A8 music streamer control.
//!
//! Provides a homelab-native interface over the generated schematic client.

use schematic_schema::eversolo::{
    DeviceGetModelRequest, Eversolo as EversoloApi, EversoloClient, MusicGetInputOutputListRequest,
    MusicGetStateRequest, MusicPlayLastRequest, MusicPlayNextRequest, MusicPlayOrPauseRequest,
    MusicSeekToRequest, MusicSetInputRequest, MusicSetMuteRequest, MusicSetOutputRequest,
    MusicSetVolumeRequest, PowerGetOptionsRequest, PowerSetOptionRequest, RemoteInputTextRequest,
    RemoteSendKeyRequest, SystemChangeVuDisplayRequest, SystemGetKnobBrightnessRequest,
    SystemGetScreenBrightnessRequest, SystemGetSpectrumModeListRequest, SystemGetVuModeListRequest,
    SystemSetKnobBrightnessRequest, SystemSetScreenBrightnessRequest, SystemSetSpectrumModeRequest,
    SystemSetVuModeRequest,
};
use schematic_schema::schematic_runtime::{EndpointSpec, NoDeclaredError};

use crate::rest::{self, RestError};

// Re-export response types for consumers
pub use schematic_schema::eversolo::{
    BrightnessResponse, DisplayModeListResponse, GetModelResponse, GetStateResponse,
    InputOutputListResponse, PowerOptionsResponse, StatusResponse,
};

/// Default Eversolo API port.
pub const DEFAULT_PORT: u16 = 9529;

/// Errors from the Eversolo API.
#[derive(Debug, thiserror::Error)]
pub enum EversoloError {
    /// The client could not be built, or a request failed or was refused.
    #[error("Eversolo API error: {0}")]
    Api(#[from] RestError),
}

/// Eversolo DMP-A8 music streamer client.
pub struct Eversolo {
    client: EversoloClient,
    host: String,
    port: u16,
}

/// Effective network-visible power state for an Eversolo streamer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectivePowerState {
    /// The device is on and not in its display-off idle mode.
    Active,
    /// The device is reachable but appears to be idling with the display off.
    Standby,
}

impl Eversolo {
    /// Creates a new Eversolo client for `http://{host}:{port}`.
    ///
    /// Uses a 5-second connect timeout and 10-second request timeout.
    ///
    /// ## Errors
    ///
    /// Returns [`EversoloError::Api`] when `host` does not form a valid base
    /// URL or the HTTP client cannot be initialized.
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, EversoloError> {
        let host = host.into();
        let base_url = format!("http://{host}:{port}");
        let client = rest::device_builder(EversoloApi::builder(), base_url)
            .build()
            .map_err(RestError::from)?;
        Ok(Self { client, host, port })
    }

    /// Returns the host address.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the port number.
    pub fn port(&self) -> u16 {
        self.port
    }

    async fn send<R>(&self, request: R) -> Result<R::Success, EversoloError>
    where
        R: EndpointSpec<Api = EversoloApi, Failure = NoDeclaredError>,
    {
        Ok(rest::send(&self.client, request).await?)
    }

    // ── Device ──────────────────────────────────────────────────────

    /// Get device model, firmware, and network information.
    pub async fn get_model(&self) -> Result<GetModelResponse, EversoloError> {
        self.send(DeviceGetModelRequest {}).await
    }

    // ── Music Control ───────────────────────────────────────────────

    /// Get current playback state, track info, and volume.
    pub async fn get_state(&self) -> Result<GetStateResponse, EversoloError> {
        self.send(MusicGetStateRequest {}).await
    }

    /// Toggle play/pause on the current track.
    pub async fn play_or_pause(&self) -> Result<StatusResponse, EversoloError> {
        self.send(MusicPlayOrPauseRequest {}).await
    }

    /// Skip to the next track.
    pub async fn play_next(&self) -> Result<StatusResponse, EversoloError> {
        self.send(MusicPlayNextRequest {}).await
    }

    /// Go back to the previous track.
    pub async fn play_previous(&self) -> Result<StatusResponse, EversoloError> {
        self.send(MusicPlayLastRequest {}).await
    }

    /// Seek to a position in the current track.
    pub async fn seek_to(&self, time_ms: i64) -> Result<StatusResponse, EversoloError> {
        self.send(MusicSeekToRequest::new().with_time(time_ms))
            .await
    }

    // ── Audio ───────────────────────────────────────────────────────

    /// List available audio inputs and outputs.
    pub async fn get_inputs_outputs(&self) -> Result<InputOutputListResponse, EversoloError> {
        self.send(MusicGetInputOutputListRequest {}).await
    }

    /// Set the active audio input.
    pub async fn set_input(&self, tag: &str) -> Result<InputOutputListResponse, EversoloError> {
        self.send(MusicSetInputRequest::new().with_tag(tag.to_string()))
            .await
    }

    /// Set the active audio output.
    pub async fn set_output(&self, tag: &str) -> Result<InputOutputListResponse, EversoloError> {
        self.send(MusicSetOutputRequest::new().with_tag(tag.to_string()))
            .await
    }

    /// Set the absolute volume level.
    pub async fn set_volume(&self, level: i64) -> Result<StatusResponse, EversoloError> {
        self.send(MusicSetVolumeRequest::new().with_volume(level))
            .await
    }

    /// Set or clear mute state.
    #[allow(non_snake_case)]
    pub async fn set_mute(&self, muted: bool) -> Result<StatusResponse, EversoloError> {
        let value = if muted { 1 } else { 0 };
        self.send(MusicSetMuteRequest::new().with_is_mute(value))
            .await
    }

    // ── Power ───────────────────────────────────────────────────────

    /// Get available power options (shutdown, reboot, etc.).
    pub async fn get_power_options(&self) -> Result<PowerOptionsResponse, EversoloError> {
        self.send(PowerGetOptionsRequest {}).await
    }

    /// Execute a power action by tag.
    pub async fn set_power_option(&self, tag: &str) -> Result<StatusResponse, EversoloError> {
        self.send(PowerSetOptionRequest::new().with_tag(tag.to_string()))
            .await
    }

    // ── Remote ──────────────────────────────────────────────────────

    /// Send a remote control key command.
    pub async fn send_key(&self, key: &str) -> Result<StatusResponse, EversoloError> {
        self.send(RemoteSendKeyRequest::new().with_key(key.to_string()))
            .await
    }

    /// Send text input to the device.
    pub async fn input_text(&self, text: &str) -> Result<StatusResponse, EversoloError> {
        self.send(RemoteInputTextRequest::new().with_text(text.to_string()))
            .await
    }

    // ── Display ─────────────────────────────────────────────────────

    /// Get current screen brightness level.
    pub async fn get_screen_brightness(&self) -> Result<BrightnessResponse, EversoloError> {
        self.send(SystemGetScreenBrightnessRequest {}).await
    }

    /// Set screen brightness level.
    pub async fn set_screen_brightness(&self, index: i64) -> Result<StatusResponse, EversoloError> {
        self.send(SystemSetScreenBrightnessRequest::new().with_index(index))
            .await
    }

    /// Get current knob LED brightness level.
    pub async fn get_knob_brightness(&self) -> Result<BrightnessResponse, EversoloError> {
        self.send(SystemGetKnobBrightnessRequest {}).await
    }

    /// Set knob LED brightness level.
    pub async fn set_knob_brightness(&self, index: i64) -> Result<StatusResponse, EversoloError> {
        self.send(SystemSetKnobBrightnessRequest::new().with_index(index))
            .await
    }

    /// List available VU meter display modes.
    pub async fn get_vu_modes(&self) -> Result<DisplayModeListResponse, EversoloError> {
        self.send(SystemGetVuModeListRequest {}).await
    }

    /// Set the VU meter display mode.
    pub async fn set_vu_mode(&self, index: i64) -> Result<StatusResponse, EversoloError> {
        self.send(SystemSetVuModeRequest::new().with_index(index))
            .await
    }

    /// List available spectrum display modes.
    pub async fn get_spectrum_modes(&self) -> Result<DisplayModeListResponse, EversoloError> {
        self.send(SystemGetSpectrumModeListRequest {}).await
    }

    /// Set the spectrum display mode.
    #[allow(non_snake_case)]
    pub async fn set_spectrum_mode(&self, index: i64) -> Result<StatusResponse, EversoloError> {
        self.send(SystemSetSpectrumModeRequest::new().with_index(index))
            .await
    }

    /// Toggle VU display open/close.
    #[allow(non_snake_case)]
    pub async fn change_vu_display(&self, open_type: i64) -> Result<StatusResponse, EversoloError> {
        self.send(SystemChangeVuDisplayRequest::new().with_open_type(open_type))
            .await
    }
}

/// Returns whether the streamer is actively playing media.
pub fn is_actively_playing(playback_state: i32) -> bool {
    playback_state == 1
}

/// Infers the effective power state from playback plus screen brightness.
///
/// The Eversolo only has true `on` and `off` power states. The front panel can
/// still enter an idle display-off mode while remaining reachable over HTTP.
/// We treat that as an effective standby state when playback is not active and
/// the screen brightness is reported as off.
pub fn infer_effective_power_state(
    playback_state: i32,
    screen_brightness: Option<i32>,
) -> EffectivePowerState {
    if !is_actively_playing(playback_state) && screen_brightness.is_some_and(|value| value <= 0) {
        EffectivePowerState::Standby
    } else {
        EffectivePowerState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectivePowerState, infer_effective_power_state, is_actively_playing};

    #[test]
    fn playing_state_is_active_playback() {
        assert!(is_actively_playing(1));
        assert!(!is_actively_playing(0));
        assert!(!is_actively_playing(2));
    }

    #[test]
    fn infers_standby_only_when_not_playing_and_screen_is_off() {
        assert_eq!(
            infer_effective_power_state(0, Some(0)),
            EffectivePowerState::Standby
        );
        assert_eq!(
            infer_effective_power_state(2, Some(0)),
            EffectivePowerState::Standby
        );
        assert_eq!(
            infer_effective_power_state(1, Some(0)),
            EffectivePowerState::Active
        );
        assert_eq!(
            infer_effective_power_state(0, Some(1)),
            EffectivePowerState::Active
        );
        assert_eq!(
            infer_effective_power_state(0, None),
            EffectivePowerState::Active
        );
    }
}

#[cfg(test)]
mod client_tests {
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::{Eversolo, EversoloError};
    use crate::rest::RestError;
    use crate::rest::test_support::closing_listener;

    fn client_for(server: &MockServer) -> Eversolo {
        Eversolo::new("127.0.0.1", server.address().port()).expect("client builds")
    }

    #[test]
    fn new_targets_plain_http_with_the_device_timeouts() {
        let eversolo = Eversolo::new("192.0.2.10", 9529).expect("client builds");

        let described = format!("{:?}", eversolo.client);
        assert!(described.contains("http://192.0.2.10:9529"), "{described}");
        assert!(described.contains("timeout: 10s"), "{described}");
    }

    #[test]
    fn new_reports_a_host_that_forms_no_base_url_as_an_error() {
        let result = Eversolo::new("bad host", 9529);

        assert!(matches!(
            result,
            Err(EversoloError::Api(RestError::Build(_)))
        ));
    }

    #[tokio::test]
    async fn set_volume_returns_the_decoded_json_reply() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ZidooMusicControl/v2/setDevicesVolume"))
            .and(query_param("volume", "42"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status":200}"#))
            .expect(1)
            .mount(&server)
            .await;

        let reply = client_for(&server).set_volume(42).await.expect("success");

        assert_eq!(reply.status, 200);
    }

    #[tokio::test]
    async fn a_non_success_status_is_an_error_carrying_status_and_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503).set_body_string("device busy"))
            .mount(&server)
            .await;

        let error = client_for(&server)
            .play_next()
            .await
            .expect_err("503 fails");

        match &error {
            EversoloError::Api(RestError::Status { status, body }) => {
                assert_eq!(*status, 503);
                assert_eq!(body, "device busy");
            }
            other => panic!("expected a status error, got {other}"),
        }
        assert_eq!(
            error.to_string(),
            "Eversolo API error: status 503: device busy"
        );
    }

    #[tokio::test]
    async fn a_malformed_success_body_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let error = client_for(&server)
            .play_next()
            .await
            .expect_err("bad body fails");

        match error {
            EversoloError::Api(RestError::Response { status, error }) => {
                assert_eq!(status, 200);
                assert!(error.is_decode_failure());
            }
            other => panic!("expected a decode failure, got {other}"),
        }
    }

    #[tokio::test]
    async fn a_connection_closed_before_any_reply_is_an_error() {
        let address = closing_listener().await;
        let eversolo = Eversolo::new("127.0.0.1", address.port()).expect("client builds");

        let error = eversolo.get_state().await.expect_err("no reply fails");

        assert!(
            matches!(error, EversoloError::Api(RestError::Transport(_))),
            "{error}"
        );
    }
}
